use core::fmt::Write;

use crate::wifi::WEB_POOL_SIZE;
use defmt::{error, info};
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use heapless::String;
use picoserve::{
    AppBuilder, AppRouter,
    response::ws,
    routing::{get, get_service},
};
use serde::Deserialize;

#[derive(Copy, Clone, Deserialize, Debug, Default)]
pub struct Control {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub speed: u8,
}
pub enum WsMessage {
    RangeChange(u8),
}
pub static CTL_STATE: Signal<CriticalSectionRawMutex, Control> = Signal::new();
pub static WS_CTL: Signal<CriticalSectionRawMutex, Control> = Signal::new();
pub static TEMP: Signal<CriticalSectionRawMutex, f32> = Signal::new();

pub struct App;

impl AppBuilder for App {
    type PathRouter = impl picoserve::routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        picoserve::Router::new()
            .route(
                "/",
                get_service(picoserve::response::File::html(include_str!("index.html"))),
            )
            .route(
                "/ws",
                get(async |upgrade: picoserve::response::WebSocketUpgrade| {
                    upgrade.on_upgrade(WebSocket).with_protocol("robotoy")
                }),
            )
    }
}

struct WebSocket;

impl ws::WebSocketCallback for WebSocket {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let mut buffer = [0; 1024];

        let close_reason = loop {
            match select(
                rx.next_message(&mut buffer, core::future::pending()),
                TEMP.wait(),
            )
            .await
            {
                Either::First(v) => {
                    match v?.ignore_never_b() {
                        Ok(ws::Message::Text(data)) => {
                            match serde_json_core::from_str::<Control>(data) {
                                Ok((ctl, _)) => {
                                    WS_CTL.signal(ctl);
                                }
                                Err(e) => error!("error: {}", e),
                            }
                            Ok(())
                        }
                        Ok(ws::Message::Binary(data)) => tx.send_binary(data).await,
                        Ok(ws::Message::Close(reason)) => {
                            info!("Websocket close reason: {:?}", reason);
                            break None;
                        }
                        Ok(ws::Message::Ping(data)) => tx.send_pong(data).await,
                        Ok(ws::Message::Pong(_)) => continue,
                        Err(e) => {
                            error!("Websocket Error: {:?}", e);
                            break Some((e.code(), "Websocket Error"));
                        }
                    }?;
                }
                Either::Second(temp) => {
                    let mut buf: String<16> = String::new();
                    write!(&mut buf, "{:.2}", temp).unwrap();
                    tx.send_text(buf.as_str()).await.unwrap();
                }
            }
        };

        tx.close(close_reason).await
    }
}

#[embassy_executor::task]
pub async fn ctl_state_task() {
    let mut state: Control = Default::default();
    loop {
        match select(Timer::after_micros(100), WS_CTL.wait()).await {
            Either::First(_) => {
                CTL_STATE.signal(state);
            }
            Either::Second(ctl) => {
                state = ctl;
                CTL_STATE.signal(state);
            }
        }
    }
}

#[embassy_executor::task(pool_size = WEB_POOL_SIZE)]
pub async fn serve(
    task_id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<App>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::Server::new(app, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}
