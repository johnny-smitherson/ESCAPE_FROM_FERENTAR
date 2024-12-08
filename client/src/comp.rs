use std::collections::HashMap;
#[allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use crate::_const::REF_Z;

#[component]
pub fn MapsDisplay(
    zoom: ReadOnlySignal<f64>,
    pos: ReadOnlySignal<(f64, f64)>,
    dimensions: ReadOnlySignal<(f64, f64)>,
) -> Element {
    let squares_in_view = use_memo(move || crate::geometry::get_tile_positions(*pos.read(), *zoom.read(), *dimensions.read()));
    let map_tile_is_loaded = use_signal(HashMap::<(i32, i32, i32), bool>::new);
    let map_tile_data = use_signal(HashMap::<(i32, i32, i32), String>::new);


    crate::data_loader::_use_handle_data_loading(squares_in_view.into(), map_tile_is_loaded, map_tile_data);

    rsx! {
        MapsCrosshair {}
        h3 { "zoom = {zoom:?} pos = {pos:?}" },

        ul {
            id: "main_display_list",
            style: "list-style-type:none;margin:0;padding:0;",

            for (sq_z , sq_x , sq_y) in squares_in_view.read().iter().cloned() {
                li { key: "tile_li_{sq_z}_{sq_x}_{sq_y}",
                    MapsTile {
                        zoom,
                        pos,
                        dimensions,
                        sq_x,
                        sq_y,
                        sq_z,
                        map_tile_is_loaded,
                        map_tile_data,
                    }
                }
            }
        }
    }
}


#[component]
fn MapsTile(
    zoom: ReadOnlySignal<f64>,
    pos: ReadOnlySignal<(f64, f64)>,
    dimensions: ReadOnlySignal<(f64, f64)>,
    sq_x: i32,
    sq_y: i32,
    sq_z: i32,
    // src: ReadOnlySignal<String>,
    map_tile_is_loaded: ReadOnlySignal<HashMap<(i32, i32, i32), bool>>,
    map_tile_data: ReadOnlySignal<HashMap<(i32, i32, i32), String>>,
) -> Element {
    let tile_size_abs = f64::exp2(REF_Z - sq_z as f64);
    let tile_pos_abs = (sq_x as f64 * tile_size_abs, sq_y as f64 * tile_size_abs);
    let tile_relative = (tile_pos_abs.0 - pos.read().0, tile_pos_abs.1 - pos.read().1);
    let camera_zoom = f64::exp2(REF_Z - *zoom.read());
    let tile_camera = (tile_relative.0 / camera_zoom, tile_relative.1 / camera_zoom);
    let tile_size = tile_size_abs / camera_zoom;
    let z_index = sq_z - 32;

    let is_loaded = use_memo(move || {
        if let Some(x) = map_tile_is_loaded.read().get(&(sq_z, sq_x, sq_y)) {
            *x
        } else {
            false
        }
    });
    let src = use_memo(move || {
        if *is_loaded.read() {
            map_tile_data
                .peek()
                .get(&(sq_z, sq_x, sq_y))
                .cloned()
                .unwrap_or("".to_string())
        } else {
            "".to_string()
        }
    });

    rsx! {
        if *is_loaded.read() {
            img {
                id: "tile_img_{sq_z}_{sq_x}_{sq_y}",
                style: "
                    width: {tile_size*50.0}vmin;
                    height: {tile_size*50.0}vmin; 
                    position: absolute; 
                    left: calc({tile_camera.0*50.0}vmin + 50vw);
                    top: calc({tile_camera.1*50.0}vmin + 50vh); 
                    background-color: transparent;
                    z-index: {z_index};
                ",
                src,
                        // src: "https://mt1.google.com/vt/lyrs=y&x={sq_x}&y={sq_y}&z={sq_z}"
            }
        }
    }
}


#[component]
pub fn MapsCrosshair() -> Element {
    rsx! {
        div {
            div { style: "
                z-index: 0;
                background-color: #FFF;
                mix-blend-mode: difference;
                width: 10vmin;
                height: 0.5vmin;
                margin-left: -5vmin;
                margin-top: -0.25vmin;
                position: absolute;
                left: 50vw;
                top: 50vh;
            " },
        div { style: "
                z-index: 0;
                background-color: #FFF;
                mix-blend-mode: difference;
                width: 0.5vmin;
                height: 10vmin;
                margin-left: -0.25vmin;
                margin-top: -5vmin;
                position: absolute;
                left: 50vw;
                top: 50vh;
            " }
        }
    }
}
