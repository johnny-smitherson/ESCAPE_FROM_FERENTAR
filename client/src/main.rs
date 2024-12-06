//! This example shows how to use the hash segment to store state in the url.
//!
//! You can set up two way data binding between the url hash and signals.
//!
//! Run this example on desktop with  
//! ```sh
//! dx serve --example hash_fragment_state --features=ciborium,base64
//! ```
//! Or on web with
//! ```sh
//! dx serve --platform web --features web --example hash_fragment_state --features=ciborium,base64 -- --no-default-features
//! ```
use client::url_state::{MapState, INIT_STATE};
#[allow(non_snake_case)]
use client::{comp::MapsDisplay, input::MapsController};
use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info, warn};

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");
    info!("dioxus launch...");
    dioxus::launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, Clone, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/#:url_hash")]
    Home {
        url_hash: MapState,
    },
    #[route("/hello")]
    Hello{},
    #[route("/storage")]
    Storage{}
}
#[component]
fn Storage() -> Element {
    let mut count_session = dioxus_sdk::storage::use_singleton_persistent(|| 0);
    let mut count_local = dioxus_sdk::storage::use_synced_storage:: <dioxus_sdk::storage::LocalStorage,i32>("synced".to_string(), || 0);

    rsx!(
        div {
            button {
                onclick: move |_| {
                    *count_session.write() += 1;
                },
                "Click me!"
            },
            "I persist for the current session. Clicked {count_session} times"
        }
        div {
            button {
                onclick: move |_| {
                    *count_local.write() += 1;
                },
                "Click me!"
            },
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
                    cancel_future = match futures::future::select(stream.next(), cancel_future).await {
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
    }});

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
                info!("cancel ack received.");
                fut.cancel();
                fut.restart();
                info!("future restarted.");
            },
            "Cancel & Restart stream"
        }
        pre {
            "{response}"
        }
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
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Err(_) = tx.unbounded_send(Ok("Hello, world!\n".to_string())) {
                warn!(" test_stream() : stream close");
                tx.close_channel();
                return;
            }
        }
    });

    Ok(TextStream::new(rx))
}



#[component]
fn Home(url_hash: ReadOnlySignal<MapState>) -> Element {
    // The initial state of the state comes from the url hash
    let mut _init_state = (&*url_hash)();
    // if url is invalid, they will not match
    if !_init_state.is_init {
        warn!("redirecting from invalid url_hash into default...");
        _init_state = INIT_STATE;
        navigator().replace(Route::Home {
            url_hash: INIT_STATE,
        });
    }

    let mut state = use_signal(|| _init_state.clone());
    let mut zoom = use_signal(|| state.peek().zoom);
    let mut pos = use_signal(|| state.peek().pos);
    let dimensions: Signal<(f64, f64)> = use_signal(|| (0.0, 0.0));

    // Change the state signal when the url hash changes
    use_effect(move || {
        if *state.peek() != *url_hash.read() {
            state.set(url_hash());
        }
    });

    // Change the url hash when the state changes -- on a debounce
    let mut debounce_write_url =
        dioxus_sdk::utils::timing::use_debounce(std::time::Duration::from_millis(100), move |_| {
            navigator().replace(Route::Home { url_hash: state() });
        });
        use_effect(move || {
        if *state.read() != *url_hash.peek() {
            debounce_write_url.action(());
        }
    });

    // change zoom when state changes
    use_effect(move || {
        if *zoom.peek() != (*state.read()).zoom {
            zoom.set(state.read().zoom);
        }
    });
    // change state when zoom changes
    use_effect(move || {
        if *zoom.read() != (*state.peek()).zoom {
            state.write().zoom = *zoom.read();
        }
    });

    // change pos when state changes
    use_effect(move || {
        if *pos.peek() != (*state.read()).pos {
            pos.set(state.read().pos);
        }
    });
    // change state when pos changes
    use_effect(move || {
        if *pos.read() != (*state.peek()).pos {
            state.write().pos = *pos.read();
        }
    });

    rsx! {
        MapsController {  zoom, pos, dimensions }
        MapsDisplay {  zoom, pos, dimensions  }
    }
}
