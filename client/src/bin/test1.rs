use client::index_db::init_db_globals;
use client::url_state::{MapState, INIT_STATE};
#[allow(non_snake_case)]
use client::{comp::MapsDisplay, input::MapsController};
use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info, warn};

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    info!("dioxus launch...");
    dioxus::launch(|| {
        info!("init db globals...");
        init_db_globals();
        info!("init db globals: done.");
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Hello{},
    #[route("/storage")]
    Storage{}
}

#[component]
fn Storage() -> Element {
    let mut count_session = dioxus_sdk::storage::use_singleton_persistent(|| 0);
    let mut count_local = dioxus_sdk::storage::use_synced_storage::<
        dioxus_sdk::storage::LocalStorage,
        i32,
    >("synced".to_string(), || 0);

    rsx!(
        div {
            button {
                onclick: move |_| {
                    *count_session.write() += 1;
                },
                "Click me!"
            }
            "I persist for the current session. Clicked {count_session} times"
        }
        div {
            button {
                onclick: move |_| {
                    *count_local.write() += 1;
                },
                "Click me!"
            }
            "I persist across all sessions. Clicked {count_local} times"
        }
    )
}

#[component]
fn Hello() -> Element {
    let mut response = use_signal(String::new);
    use futures_util::StreamExt;
    let cancel = use_signal(|| tokio::sync::broadcast::channel::<()>(1));
    let cancel_ack = use_signal(|| async_channel::bounded::<()>(1));
    let mut fut = use_future(move || {
        async move {
            info!("future started.");
            response.write().clear();
            if let Ok(stream) = test_stream().await {
                response.write().push_str("Stream started\n");
                let mut stream = stream.into_inner();
                let mut cancel_rx = cancel.peek().0.subscribe();
                let cancel_ack_tx = cancel_ack.peek().0.clone();
                let cancel_future = cancel_rx.recv();
                futures::pin_mut!(cancel_future);
                loop {
                    cancel_future =
                        match futures::future::select(stream.next(), cancel_future).await {
                            futures::future::Either::Left((stream_value, _next_fut)) => {
                                if let Some(Ok(text)) = stream_value {
                                    response.write().push_str(&text);
                                    _next_fut
                                } else {
                                    info!("stream finished.");
                                    return;
                                }
                            }
                            futures::future::Either::Right((_stop_value, _)) => {
                                if let Err(e) = cancel_ack_tx.send(()).await {
                                    error!("failed to send cancel ack message: {:?}", e);
                                }
                                info!("cancel channel got message: cancelling stream.");
                                // async_std::task::sleep(std::time::Duration::from_millis(1000)).await;
                                return;
                            }
                        }
                }
            }
        }
    });

    rsx! {
        button {
            onclick: move |_| async move {
                info!("button clicked.");
                if let Err(e) = cancel.peek().0.clone().send(()) {
                    warn!("cancel failed!!! {:?}", e);
                }
                info!("cancel sent, waiting for resp...");
                if let Err(e) = cancel_ack.peek().1.recv().await {
                    warn!("cancel_ack read failed: {:?}!", e);
                }
                fut.cancel();
                fut.restart();
                info!("future restarted.");
            },
            "Cancel & Restart stream"
        }
        pre { "{response}" }
    }
}
use crate::server_fn::codec::StreamingText;
use crate::server_fn::codec::TextStream;
#[server(output = StreamingText)]
pub async fn test_stream() -> Result<TextStream, ServerFnError> {
    // let (tx, rx) = async_channel::bounded(1);
    let (mut tx, rx) = futures::channel::mpsc::unbounded();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            if let Err(_) = tx.unbounded_send(Ok("Hello, world!\n".to_string())) {
                warn!(" test_stream() : stream close");
                tx.close_channel();
                return;
            }
        }
    });

    Ok(TextStream::new(rx))
}
