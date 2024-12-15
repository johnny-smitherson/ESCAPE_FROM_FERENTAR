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
    #[route("/#:url_hash")]
    Home {
        url_hash: MapState,
    },
}

#[component]
fn Home(url_hash: ReadOnlySignal<MapState>) -> Element {
    // The initial state of the state comes from the url hash
    let mut _init_state = url_hash.read().clone();
    let mut map_state = use_signal(|| _init_state.clone());
    let dimensions: Signal<(f64, f64)> = use_signal(|| (0.0, 0.0));
    // if url is invalid, they will not match
    if !_init_state.is_init {
        warn!("redirecting from invalid url_hash into default...");
        _init_state = INIT_STATE;
        navigator().replace(Route::Home {
            url_hash: INIT_STATE,
        });
    }

    // Change the state signal when the url hash changes -- on a debounce
    use_effect(move || {
        if *map_state.peek() != *url_hash.read() {
            map_state.set(url_hash());
        }
    });

    // Change the url hash when the state changes -- on a debounce
    let mut debounce_write_url =
        dioxus_sdk::utils::timing::use_debounce(std::time::Duration::from_millis(100), move |_| {
            navigator().replace(Route::Home { url_hash: map_state() });
        });
    use_effect(move || {
        if *map_state.read() != *url_hash.peek() {
            debounce_write_url.action(());
        }
    });


    rsx! {
        MapsController { map_state, dimensions }
        MapsDisplay { map_state, dimensions }
    }
}
