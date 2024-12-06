#[allow(non_snake_case)]
use std::{fmt::Display, str::FromStr};

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;

use dioxus::prelude::*;
use dioxus_elements::geometry::{euclid::Size2D, WheelDelta};
use dioxus_logger::tracing::{error, info, warn};
use serde::{Deserialize, Serialize};

use crate::_const::{MAX_Z, MIN_Z, REF_Z};


fn get_tile_positions(pos: (f64, f64), zoom: f64, dimensions: (f64, f64), max_pic_pixels: f64) -> Vec<(i32, i32, i32)> {
    let vmin_px = f64::min(dimensions.0, dimensions.1);
    let min_dim_tiles = vmin_px / max_pic_pixels;
    let ideal_tile_level = (f64::trunc(f64::log2(min_dim_tiles) + zoom) as i32).clamp(MIN_Z, MAX_Z);

    let x0 = (pos.0 / f64::exp2(REF_Z - ideal_tile_level as f64)).floor() as i32;
    let y0 = (pos.1 / f64::exp2(REF_Z - ideal_tile_level as f64)).floor() as i32;
    let remainder0 = f64::exp2(ideal_tile_level as f64) as i32;
    let x0 = x0 .rem_euclid(remainder0);
    let y0 = y0 .rem_euclid(remainder0);

    let mut ze_squarez = vec![];
    let tile_diff_exp = f64::exp(f64::fract(f64::log2(min_dim_tiles) + zoom));
    let tile_count_x = (dimensions.0/vmin_px / tile_diff_exp * min_dim_tiles+1.1).ceil() as i32;
    let tile_count_y = ( dimensions.1/vmin_px / tile_diff_exp * min_dim_tiles+1.1).ceil() as i32;
    for i in (x0-tile_count_x)..=(x0+tile_count_x) {
        for j in (y0-tile_count_y)..=(y0+tile_count_y) {
            ze_squarez.push((ideal_tile_level, i .rem_euclid(remainder0), j .rem_euclid(remainder0)));
        }
    }

    ze_squarez.sort();
    ze_squarez.dedup();
    ze_squarez
}
#[component]
pub fn MapsDisplay (
    zoom: ReadOnlySignal<f64>,
    pos: ReadOnlySignal<(f64, f64)>,
    dimensions: ReadOnlySignal<(f64, f64)>,
) -> Element {

    // screen must contain at least this many images

    let mut ze_squarez = get_tile_positions(*pos.read(), *zoom.read(), *dimensions.read(), 512.0);
    let mut ze_squarez_big = get_tile_positions(*pos.read(), *zoom.read(), *dimensions.read(), 256.0);
    // info!("first grp: z={} num={}", ze_squarez[0].2, ze_squarez.len());
    // info!("second grp: z={} num={}", ze_squarez_big[0].2, ze_squarez_big.len());
    ze_squarez.append(&mut ze_squarez_big);
    ze_squarez.sort();
    ze_squarez.dedup();

    rsx! {
        h3 { "zoom = {zoom:?} pos = {pos:?}" }
        ul {
            id:"main_display_list",
            style:"list-style-type:none;margin:0;padding:0;",

            for (sq_z, sq_x, sq_y) in ze_squarez {
                li {
                    key: "tile_li_{sq_z}_{sq_x}_{sq_y}",
                    MapsTile {
                        zoom, pos, dimensions,
                        sq_x, sq_y, sq_z,
                    }
                }
            }
        }

        MapsCrosshair{}
    }
}


#[component]
fn MapsTile (
    zoom: ReadOnlySignal<f64>,
    pos: ReadOnlySignal<(f64, f64)>,
    dimensions: ReadOnlySignal<(f64, f64)>,
    sq_x: i32, sq_y: i32, sq_z: i32,
) -> Element {
    let tile_size_abs = f64::exp2(REF_Z - sq_z as f64);
    let tile_pos_abs = (sq_x as f64 * tile_size_abs , sq_y as f64 * tile_size_abs );
    let tile_relative = (
        tile_pos_abs.0 - pos.read().0,
        tile_pos_abs.1 - pos.read().1,
    );
    let camera_zoom = f64::exp2(REF_Z - *zoom.read());
    let tile_camera = (
        tile_relative.0 /  camera_zoom,
        tile_relative.1 /  camera_zoom,
    );
    let tile_size = tile_size_abs / camera_zoom;
    let mut src = use_signal(|| "".to_string());
    use_future(move || async move {
        match  get_server_tile_img(sq_z, sq_x, sq_y).await {
            Ok(x) => {
                *src.write()=x;
            }
            Err(x) => {
                warn!("err fetching img z={sq_z}/x={sq_x}/y={sq_y}:  {:?}", x)
            }
        }
    });
    rsx! {
        img { 
            id: "tile_img_{sq_z}_{sq_x}_{sq_y}",
            style: "
                width: {tile_size*50.0}vmin;
                height: {tile_size*50.0}vmin; 
                position: absolute; 
                left: calc({tile_camera.0*50.0}vmin + 50vw);
                top: calc({tile_camera.1*50.0}vmin + 50vh); 
                background-color: transparent;
            ", 
            src: src,
            // src: "https://mt1.google.com/vt/lyrs=y&x={sq_x}&y={sq_y}&z={sq_z}"
        }
    }
}

#[server(GetServerTile)]
async fn get_server_tile_img(sq_z: i32, sq_x: i32, sq_y: i32) -> Result<String, ServerFnError> {
    let url = format!("http://localhost:8000/api/tile/google_hybrid/{sq_z}/{sq_x}/{sq_y}/jpg");

    let response = reqwest::get(&url).await?;
    let status_code = response.status().clone();
    let content_type = response.headers()["Content-Type"].to_str()?.to_string();
    let content_length = response.content_length().clone();
    let resp_bytes = response.bytes().await?;
    let resp_base64 = base64::prelude::BASE64_STANDARD.encode(resp_bytes);

    let img_src = format!("data:{content_type};base64,{resp_base64}");
    Ok(img_src)
}


#[component]
pub fn MapsCrosshair() -> Element {
    rsx! {
        div {
            style: "
                background-color: #FFF;
                mix-blend-mode: difference;
                width: 10vmin;
                height: 0.5vmin;
                margin-left: -5vmin;
                margin-top: -0.25vmin;
                position: absolute;
                left: 50vw;
                top: 50vh;
            "
        }
        div {
            style: "
                background-color: #FFF;
                mix-blend-mode: difference;
                width: 0.5vmin;
                height: 10vmin;
                margin-left: -0.25vmin;
                margin-top: -5vmin;
                position: absolute;
                left: 50vw;
                top: 50vh;
            "
        }
    }
}

