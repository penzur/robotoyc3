#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_hal::clock::CpuClock;
use esp_hal::ledc::channel::{ChannelHW, ChannelIFace};
use esp_hal::ledc::timer::TimerIFace;
use esp_hal::ledc::timer::config::Duty;
use esp_hal::ledc::{self, LowSpeed};
use esp_hal::rng::Rng;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use esp_radio::Controller;
use picoserve::{AppBuilder, AppRouter};
use robotoyc3::wifi::{self, WEB_POOL_SIZE, check_connection, init_stack};
use robotoyc3::ws::{App, CTL_STATE, Control, ctl_state_task, serve};
use static_cell::StaticCell;

static RADIO: StaticCell<Controller> = StaticCell::new();
static APP: StaticCell<AppRouter<App>> = StaticCell::new();
static CONFIG: StaticCell<picoserve::Config<Duration>> = StaticCell::new();

const ESC_PWM_HZ: u32 = 50;
const ESC_PERIOD_US: u32 = 20_000;
const ESC_NEUTRAL_US: u32 = 1_500;

fn pulse_us_to_duty(pulse_us: u32, r_max_duty: u16) -> u16 {
    ((pulse_us * u32::from(r_max_duty)) / ESC_PERIOD_US) as u16
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let radio_init = RADIO.init(esp_radio::init().unwrap());
    let (wctl, wface) =
        esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default()).unwrap();
    let device = wface.ap;

    let rng = Rng::new();
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    let ip_cfg = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(core::net::Ipv4Addr::new(192, 168, 1, 1), 24),
        gateway: Some(core::net::Ipv4Addr::new(192, 168, 1, 1)),
        dns_servers: Default::default(),
    });

    let resources = init_stack();
    let (stack, runner) = embassy_net::new(device, ip_cfg, resources, net_seed);

    spawner.spawn(wifi::wifi_ap_setup(wctl)).ok();
    spawner.spawn(wifi::network_stack(runner)).ok();
    check_connection(stack).await;

    let mut ledc = ledc::Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(ledc::LSGlobalClkSource::APBClk);

    let mut t0 = ledc.timer::<LowSpeed>(ledc::timer::Number::Timer0);
    t0.configure(ledc::timer::config::Config {
        duty: Duty::Duty14Bit,
        clock_source: ledc::timer::LSClockSource::APBClk,
        frequency: Rate::from_hz(ESC_PWM_HZ),
    })
    .expect("Could not configure time");
    let r_max_duty = 1u16 << (t0.duty().unwrap() as u16);

    let mut right_channel =
        ledc.channel::<LowSpeed>(ledc::channel::Number::Channel0, peripherals.GPIO21);
    right_channel
        .configure(ledc::channel::config::Config {
            duty_pct: 0,
            timer: &mut t0,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        })
        .unwrap();

    let mut t1 = ledc.timer::<LowSpeed>(ledc::timer::Number::Timer1);
    t1.configure(ledc::timer::config::Config {
        duty: Duty::Duty14Bit,
        clock_source: ledc::timer::LSClockSource::APBClk,
        frequency: Rate::from_hz(ESC_PWM_HZ),
    })
    .expect("Could not configure time");
    let l_max_duty = 1u16 << (t1.duty().unwrap() as u16);

    let mut left_channel =
        ledc.channel::<LowSpeed>(ledc::channel::Number::Channel1, peripherals.GPIO20);
    left_channel
        .configure(ledc::channel::config::Config {
            duty_pct: 0,
            timer: &mut t1,
            drive_mode: esp_hal::gpio::DriveMode::PushPull,
        })
        .unwrap();

    let app = APP.init(App.build_app());
    let config = CONFIG.init(
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            persistent_start_read_request: Some(Duration::from_secs(5)),
            read_request: Some(Duration::from_secs(5)),
            write: Some(Duration::from_secs(5)),
        })
        .keep_connection_alive(),
    );

    for tid in 0..WEB_POOL_SIZE {
        spawner.must_spawn(serve(tid, stack, app, config));
    }

    spawner.spawn(ctl_state_task()).ok();

    loop {
        let control: Control = CTL_STATE.wait().await;

        if (control.forward || control.left) && !control.right && !control.back {
            let duty = ESC_NEUTRAL_US + (control.speed as u32 * 500 / 100);
            right_channel.set_duty_hw(u32::from(pulse_us_to_duty(duty, r_max_duty)));
        } else if control.back && !control.right {
            let duty = ESC_NEUTRAL_US - (control.speed as u32 * 500 / 100);
            right_channel.set_duty_hw(u32::from(pulse_us_to_duty(duty, r_max_duty)));
        } else {
            right_channel.set_duty_hw(u32::from(pulse_us_to_duty(ESC_NEUTRAL_US, r_max_duty)));
        }

        // TODO: reverse motor wiring for left channel
        if (control.forward || control.right) && !control.left && !control.back {
            let duty = ESC_NEUTRAL_US + (control.speed as u32 * 500 / 100);
            left_channel.set_duty_hw(u32::from(pulse_us_to_duty(duty, l_max_duty)));
        } else if control.back && !control.left {
            let duty = ESC_NEUTRAL_US - (control.speed as u32 * 500 / 100);
            left_channel.set_duty_hw(u32::from(pulse_us_to_duty(duty, l_max_duty)));
        } else {
            left_channel.set_duty_hw(u32::from(pulse_us_to_duty(ESC_NEUTRAL_US, l_max_duty)));
        }
    }
}
