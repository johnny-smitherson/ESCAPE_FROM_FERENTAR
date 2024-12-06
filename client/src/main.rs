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
    Hello{}
}



#[component]
fn Hello() -> Element {
    let mut response = use_signal(String::new);
    use futures_util::StreamExt;
    rsx! {
        button {
            onclick: move |_| async move {
                response.write().clear();
                if let Ok(stream) = test_stream().await {
                    response.write().push_str("Stream started\n");
                    let mut stream = stream.into_inner();
                    while let Some(Ok(text)) = stream.next().await {
                        response.write().push_str(&text);
                    }
                }
            },
            "Start stream"
        }
        "{response}"
    }
}
use crate::server_fn::codec::StreamingText;
use crate::server_fn::codec::TextStream;
#[server(output = StreamingText)]
pub async fn test_stream() -> Result<TextStream, ServerFnError> {
    let (tx, rx) = futures::channel::mpsc::unbounded();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let _ = tx.unbounded_send(Ok("Hello, world!".to_string()));
        }
    });

    Ok(TextStream::new(rx))
}



#[component]
fn Home(url_hash: ReadOnlySignal<MapState>) -> Element {
    // The initial state of the state comes from the url hash
    let _init_state = (&*url_hash)();
    // if url is invalid, they will not match
    if !_init_state.is_init {
        warn!("redirecting from invalid url_hash into default...");
        navigator().replace(Route::Home {
            url_hash: INIT_STATE,
        });
    }

    let mut state = use_signal(&*url_hash);
    let mut zoom = use_signal(|| state.peek().zoom);
    let mut pos = use_signal(|| state.peek().pos);
    let dimensions: Signal<(f64, f64)> = use_signal(|| (0.0, 0.0));

    // Change the state signal when the url hash changes
    use_memo(move || {
        if *state.peek() != *url_hash.read() {
            state.set(url_hash());
        }
    });

    // Change the url hash when the state changes -- on a debounce
    let mut debounce_write_url =
        dioxus_sdk::utils::timing::use_debounce(std::time::Duration::from_millis(100), move |_| {
            navigator().replace(Route::Home { url_hash: state() });
        });
    use_memo(move || {
        if *state.read() != *url_hash.peek() {
            debounce_write_url.action(());
        }
    });

    // change zoom when state changes
    use_memo(move || {
        if *zoom.peek() != (*state.read()).zoom {
            zoom.set(state.read().zoom);
        }
    });
    // change state when zoom changes
    use_memo(move || {
        if *zoom.read() != (*state.peek()).zoom {
            state.write().zoom = *zoom.read();
        }
    });

    // change pos when state changes
    use_memo(move || {
        if *pos.peek() != (*state.read()).pos {
            pos.set(state.read().pos);
        }
    });
    // change state when pos changes
    use_memo(move || {
        if *pos.read() != (*state.peek()).pos {
            state.write().pos = *pos.read();
        }
    });

    rsx! {
        MapsController {  zoom, pos, dimensions }
        MapsDisplay {  zoom, pos, dimensions  }
    }
}
