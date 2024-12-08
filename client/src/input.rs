#[allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_elements::geometry::{euclid::Size2D, WheelDelta};
use dioxus_logger::tracing::{error, info, warn};
use serde::{Deserialize, Serialize};

use crate::_const::{MAX_Z, MIN_Z};

#[component]
pub fn MapsController(
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
        let quad_edge = f64::min(quad_edge.0, quad_edge.1) / 2.0;

        if let (Some(current), Some(last)) = (current, last) {
            let diff = (
                (current.0 - last.0) / quad_edge,
                (current.1 - last.1) / quad_edge,
            );
            if diff.0.abs() + diff.1.abs() > 0.00001 {
                // warn!("MOVEMENT DIFF = {diff:?}");
                let old_pos = *pos.peek();
                let exp = f64::exp2(crate::_const::REF_Z - *zoom.peek());
                *pos.write() = (old_pos.0 - diff.0 * exp, old_pos.1 - diff.1 * exp)
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
            *zoom.write() = (_old_zoom_sig + diff).clamp(MIN_Z as f64, MAX_Z as f64);
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
