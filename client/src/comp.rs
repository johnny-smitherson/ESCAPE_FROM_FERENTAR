use crate::{_const::REF_Z, url_state::MapState};
#[allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_logger::tracing::info;
use std::collections::HashMap;

#[component]
pub fn MapsDisplay(
    map_state: ReadOnlySignal<MapState>,
    dimensions: ReadOnlySignal<(f64, f64)>,
) -> Element {
    let squares_in_view = use_memo(move || {
        crate::geometry::get_tile_positions(map_state.read().pos, map_state.read().zoom, *dimensions.read())
    });
    let map_tile_is_loaded = use_signal(HashMap::<(i32, i32, i32), bool>::new);
    let map_tile_data = use_signal(HashMap::<(i32, i32, i32), String>::new);

    crate::data_loader::use_handle_data_loading(
        squares_in_view.into(),
        map_tile_is_loaded,
        map_tile_data,
    );

    rsx! {
        MapsCrosshair {}
        MapsInterface {map_state},

        ul {
            id: "main_display_list",
            style: "list-style-type:none;margin:0;padding:0;",

            for (sq_z , sq_x , sq_y) in squares_in_view.read().iter().cloned() {
                li { key: "tile_li_{sq_z}_{sq_x}_{sq_y}",
                    MapsTile {
                        map_state,
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
fn MapsInterface(map_state: ReadOnlySignal<MapState>) -> Element {
    rsx! {
        div {
            style: "
                position: absolute; 
                left: 2vmin; 
                top: 2vh;
                width: 20vmin; 
                height: 92vh;
                background-color: white;
                z-index: 0;
                padding: 1vmin;
                margin: 1vmin;
            ",
            
            h3 { "zoom = {map_state.read().zoom:?} pos = {map_state.read().pos:?}" }
        }
    }
}

#[component]
fn MapsTile(
    map_state: ReadOnlySignal<MapState>,
    dimensions: ReadOnlySignal<(f64, f64)>,
    sq_x: i32,
    sq_y: i32,
    sq_z: i32,
    // src: ReadOnlySignal<String>,
    map_tile_is_loaded: ReadOnlySignal<HashMap<(i32, i32, i32), bool>>,
    map_tile_data: ReadOnlySignal<HashMap<(i32, i32, i32), String>>,
) -> Element {
    let pos = map_state.read().pos;
    let zoom = map_state.read().zoom;
    let tile_size_abs = f64::exp2(REF_Z - sq_z as f64);
    let tile_pos_abs = (sq_x as f64 * tile_size_abs, sq_y as f64 * tile_size_abs);
    let tile_relative = (tile_pos_abs.0 - pos.0, tile_pos_abs.1 - pos.1);
    let camera_zoom = f64::exp2(REF_Z - zoom);
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
            id: "maps_crosshairs",
            div { id: "maps_crosshair_1", style: "
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
            " }
            div { id: "maps_crosshair_2", style: "
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
