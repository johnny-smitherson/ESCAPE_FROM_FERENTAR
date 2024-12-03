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
#[allow(non_snake_case)]
use std::{fmt::Display, str::FromStr};

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;

use dioxus::prelude::*;
use dioxus_elements::geometry::{euclid::Size2D, WheelDelta};
use dioxus_logger::tracing::{error, info, warn};
use serde::{Deserialize, Serialize};

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
}

// You can use a custom type with the hash segment as long as it implements Display, FromStr and Default
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
struct MapState {
    /// if false, overwrite with the default value with "true" set.
    is_init: bool,
    zoom: f64,
    pos: (f64, f64),
}

const INIT_STATE: MapState = MapState {
    is_init: true,
    zoom: 18.0,
    pos: (0.0, 0.0),
};

// Display the state in a way that can be parsed by FromStr
impl Display for MapState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut serialized = Vec::new();
        if ciborium::into_writer(self, &mut serialized).is_ok() {
            write!(f, "{}", URL_SAFE.encode(serialized))?;
        }
        Ok(())
    }
}

enum StateParseError {
    DecodeError(base64::DecodeError),
    CiboriumError(ciborium::de::Error<std::io::Error>),
}

impl std::fmt::Display for StateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeError(err) => write!(f, "Failed to decode base64: {}", err),
            Self::CiboriumError(err) => write!(f, "Failed to deserialize: {}", err),
        }
    }
}

// Parse the state from a string that was created by Display
impl FromStr for MapState {
    type Err = StateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decompressed = URL_SAFE
            .decode(s.as_bytes())
            .map_err(StateParseError::DecodeError)?;
        let parsed = ciborium::from_reader(std::io::Cursor::new(decompressed))
            .map_err(StateParseError::CiboriumError)?;
        Ok(parsed)
    }
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


#[component]
fn MapsDisplay (
    zoom: ReadOnlySignal<f64>,
    pos: ReadOnlySignal<(f64, f64)>,
    dimensions: ReadOnlySignal<(f64, f64)>,
) -> Element {

    let ze_squarez = vec![
        ("red", (0, 0, 18)),
        ("green", (1, 1, 18)),
        ("blue", (-1, -1, 18)),
        ("purple", (1, -1, 18)),
        ("purple", (1, -1, 18)),
    ];

    rsx! {
        h3 { "zoom = {zoom:?}" }
        h3 { "pos = {pos:?}" }
        for (sq_color, (sq_x, sq_y, sq_z)) in ze_squarez {

            MapsTile {
                zoom, pos, dimensions,
                sq_x, sq_y, sq_z, sq_color: sq_color.to_string()
            }

        }
    }
}

#[component]
fn MapsTile (
    zoom: ReadOnlySignal<f64>,
    pos: ReadOnlySignal<(f64, f64)>,
    dimensions: ReadOnlySignal<(f64, f64)>,
    sq_x: i32, sq_y: i32, sq_z: i32, sq_color: String,
) -> Element {
    let zoom = *zoom.read()  - sq_z as f64;
    let zoom = f64::exp2(zoom);
    let tile_pos_abs = (sq_x as f64 * zoom + pos.read().0, sq_y as f64 * zoom + pos.read().1);
    rsx! {
        div { style: "
        width: {zoom*100.0}vmin;
        height: {zoom*100.0}vmin; 
        position: absolute; 
        top: calc({tile_pos_abs.1*100.0}vmin - 50vh); 
        left: calc({tile_pos_abs.0*100.0}vmin - 50vw);
        color: {sq_color};
        background-color: {sq_color};
    " }
    }
}


#[component]
fn MapsController(
    mut zoom: Signal<f64>,
    mut pos: Signal<(f64, f64)>,
    mut dimensions: Signal<(f64, f64)>,
) -> Element {
    #[derive(Copy, Clone, Debug)]
    struct PointerMoveEvent {
        coord_x: f64,
        coord_y: f64,
        is_pressed: bool,
    }

    #[derive(Copy, Clone, Debug)]
    struct MouseZoomEvent {
        lines_diff: f64,
        pixels_diff: f64,
        pinch_dist: f64,
        is_pinch: bool,
    }

    let mut last_pointer_pos: Signal<Option<(f64, f64)>> = use_signal(|| None);
    let mut on_movement = move |event: PointerMoveEvent| {
        let last = *last_pointer_pos.peek();
        let current = if event.is_pressed {
            Some((event.coord_x, event.coord_y))
        } else {
            None
        };
        let quad_edge = *dimensions.peek();
        let quad_edge = f64::min(quad_edge.0, quad_edge.1);

        if let (Some(current), Some(last)) = (current, last) {
            let diff = (
                (current.0 - last.0) / quad_edge,
                (current.1 - last.1) / quad_edge,
            );
            if diff.0.abs() + diff.1.abs() > 0.00001 {
                // warn!("MOVEMENT DIFF = {diff:?}");
                let old_pos = *pos.peek();
                *pos.write() = (old_pos.0 + diff.0, old_pos.1 + diff.1);
            }
        }

        if last != current {
            *last_pointer_pos.write() = current;
        }
    };

    let mut last_pinch_dist: Signal<Option<f64>> = use_signal(|| None);
    let mut on_zoom = move |event: MouseZoomEvent| {
        let last = *last_pinch_dist.peek();
        let current = if event.is_pinch {
            Some(event.pinch_dist)
        } else {
            None
        };
        // let quad_edge = *dimensions.peek();
        // let quad_edge = f64::min(quad_edge.0, quad_edge.1);

        let diff_wheel = if event.lines_diff.abs() > 0.1 {
            (event.lines_diff.signum()) / 5.0 
        } else {
            0.0
        } + if event.pixels_diff.abs() > 0.1 {
            event.pixels_diff.signum() / 5.0
        } else {
            0.0
        };

        let diff_pinch = if let (Some(current), Some(last)) = (current, last) {
            (current / last).log2()
        } else {
            0.0
        };

        let diff = -diff_wheel + diff_pinch;
        if diff.abs() > 0.00001 {
            // warn!("ZOOM = {diff}");
            let _old_zoom_sig = *zoom.peek();
            *zoom.write() = _old_zoom_sig + diff;
            info!("{:#?}", zoom.peek());
        }

        if last != current {
            *last_pinch_dist.write() = current;
        }
    };

    let on_mouse = move |event: Event<MouseData>| {
        event.prevent_default();
        let data = event.data();

        let ev = PointerMoveEvent {
            coord_x: data.page_coordinates().x,
            coord_y: data.page_coordinates().y,
            is_pressed: data
                .held_buttons()
                .contains(dioxus_elements::input_data::MouseButton::Primary),
        };

        on_movement(ev);
    };

    let on_touch = move |event: Event<TouchData>| {
        event.prevent_default();
        let data = event.data();
        let _changed = data.touches_changed();
        let _current = data.touches();
        let _target = data.target_touches();

        let new_touch = if let Some(n) = _target.get(0) {
            n
        } else {
            if let Some(n) = _changed.get(0) {
                n
            } else {
                return;
            }
        };

        let ev = PointerMoveEvent {
            coord_x: new_touch.page_coordinates().x,
            coord_y: new_touch.page_coordinates().y,
            is_pressed: data.touches().len() == 1,
        };

        on_movement(ev);

        let ev2 = if _current.len() >= 2 {
            let p1 = _current[0].page_coordinates();
            let p2 = _current[1].page_coordinates();
            let touch_diff = (p1 - p2).length();
            MouseZoomEvent {
                pixels_diff: 0.,
                lines_diff: 0.,
                pinch_dist: touch_diff,
                is_pinch: true,
            }
        } else {
            MouseZoomEvent {
                pixels_diff: 0.,
                lines_diff: 0.,
                pinch_dist: 0.,
                is_pinch: false,
            }
        };
        on_zoom(ev2);
    };

    let on_wheel = move |event: Event<WheelData>| {
        event.prevent_default();
        let data = event.data();
        let diff = data.delta();

        let lines_diff = if let WheelDelta::Lines(x) = diff {
            x.y
        } else {
            0.0
        } + if let WheelDelta::Pages(x) = diff {
            x.y / 30.0
        } else {
            0.0
        };
        let pixels_diff = if let WheelDelta::Pixels(x) = diff {
            x.y
        } else {
            0.0
        };

        let ev = MouseZoomEvent {
            lines_diff,
            pixels_diff,
            pinch_dist: 0.0,
            is_pinch: false,
        };
        info!("{ev:#?}");
        on_zoom(ev);
    };

    rsx! {
        div {
            id: "receiver",
            tabindex: 0,
            onmousemove: on_mouse,
            onmousedown: on_mouse,
            onmouseup: on_mouse,
            onwheel: on_wheel,

            ontouchcancel: on_touch,
            ontouchend: on_touch,
            ontouchmove: on_touch,
            ontouchstart: on_touch,

            // get initial mounted component size
            onmounted: move |event| async move {
                if let Ok(client_rect) = event.get_client_rect().await {
                    let size = client_rect.size;
                    dimensions.set((size.width, size.height));
                }
            },
            // update component size
            onresize: move |event| {
                let size = event.data().get_content_box_size().unwrap();
                dimensions.set((size.width, size.height))
            },
        }
    }
    // }
}
