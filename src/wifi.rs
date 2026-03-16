use defmt::info;
use embassy_net::StackResources;
use embassy_net::{Runner, Stack};
use embassy_time::Timer;
use esp_radio::Controller;
use esp_radio::wifi::{
    AccessPointConfig, AuthMethod, ModeConfig, WifiApState, WifiController, WifiDevice, WifiEvent,
};

use static_cell::StaticCell;

pub static WEB_POOL_SIZE: usize = 4;

static STATIC_RADIO: StaticCell<Controller> = StaticCell::new();
static STACK_RESOURCE: StaticCell<StackResources<WEB_POOL_SIZE>> = StaticCell::new();

pub fn init_radio() -> &'static Controller<'static> {
    STATIC_RADIO.init(esp_radio::init().expect("failed to init radio"))
}

pub fn init_stack() -> &'static mut StackResources<WEB_POOL_SIZE> {
    STACK_RESOURCE.init(StackResources::<{ WEB_POOL_SIZE }>::new())
}

pub async fn check_connection(stack: Stack<'static>) {
    info!("waiting for link...");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after_millis(500).await;
    }
    info!("waiting for ip address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("ip address is: {}", config.address);
            break;
        }
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
pub async fn network_stack(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
pub async fn wifi_ap_setup(mut wctl: WifiController<'static>) {
    info!("setting up wifi AP and shit...");
    loop {
        match esp_radio::wifi::ap_state() {
            WifiApState::Started => {
                // wait until we're no longer connected
                wctl.wait_for_event(WifiEvent::ApStop).await;
                embassy_time::Timer::after_millis(5000).await
            }
            _ => {}
        }
        if !matches!(wctl.is_started(), Ok(true)) {
            let apcfg = AccessPointConfig::default()
                .with_ssid(env!("SSID").into())
                .with_password(env!("PASSWORD").into())
                .with_auth_method(AuthMethod::Wpa2Personal);
            let config = ModeConfig::AccessPoint(apcfg);
            wctl.set_config(&config)
                .expect("could not set wifi AP config");
            info!("starting wifi access point");
            wctl.start_async().await.expect("could not start wifi AP");
            info!("wifi AP started!");
        }
    }
}
