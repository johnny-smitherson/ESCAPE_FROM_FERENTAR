use std::collections::HashMap;
use std::time::Duration;
#[allow(non_snake_case)]
use std::{fmt::Display, str::FromStr};

use async_std::prelude::FutureExt;
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;

use dioxus::prelude::*;
use dioxus_elements::geometry::{euclid::Size2D, WheelDelta};
use dioxus_logger::tracing::{error, info, warn};
use serde::{Deserialize, Serialize};
use server_fn::error::{NoCustomError, ServerFnErrorErr};

use crate::_const::{MAX_Z, MIN_Z, REF_Z};
use crate::index_db::{read_image, write_image};


fn get_tile_positions_one_level(pos: (f64, f64), zoom: f64, dimensions: (f64, f64), max_pic_pixels: f64) -> Vec<(i32, i32, i32)> {
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

fn get_tile_positions(pos: (f64, f64), zoom: f64, dimensions: (f64, f64)) -> Vec<(i32, i32, i32)> {
    const IMG_AVG_PX : f64 = 512.0;
    let mut ze_squarez = get_tile_positions_one_level(pos, zoom, dimensions, IMG_AVG_PX * 4.0);
    let mut ze_squarez_big = get_tile_positions_one_level(pos, zoom, dimensions, IMG_AVG_PX * 2.0);
    // info!("first grp: z={} num={}", ze_squarez[0].2, ze_squarez.len());
    // info!("second grp: z={} num={}", ze_squarez_big[0].2, ze_squarez_big.len());
    ze_squarez.append(&mut ze_squarez_big);
    ze_squarez_big = get_tile_positions_one_level(pos, zoom, dimensions, IMG_AVG_PX);
    ze_squarez.append(&mut ze_squarez_big);
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

    let squares_in_view = use_memo(move || {
        let sq =         get_tile_positions(*pos.read(), *zoom.read(), *dimensions.read());
        // info!("computed {:?} squares", sq.len());
        sq
    });
    let mut loaded_tiles = use_signal(HashMap::<(i32,i32,i32),String>::new);
    // info!("SQUARE STORAGE: {}", square_storage.peek().len());

    // vvvvvvvvvvvv
    let cancel = use_signal(|| tokio::sync::broadcast::channel::<()>(1));
    let cancel_ack = use_signal(|| async_channel::bounded::<()>(1));
    // ^^^^^^^^^^^^

    let mut fut = use_future(move || async move {
        // vvvvvvvvvvvv
        let mut cancel_rx = cancel.peek().0.subscribe();
        let cancel_ack_tx = cancel_ack.peek().0.clone();
        let cancel_future = cancel_rx.recv();
        futures::pin_mut!(cancel_future);
        // ^^^^^^^^^^^^
        let do_stuff = async move {
            info!("init future...");
            let _total_count = squares_in_view.len();
            let request_list = squares_in_view.read().iter().filter(|k| !loaded_tiles.peek().contains_key(k)).cloned().collect::<Vec<_>>();
            let _new_conut = request_list.len();
            if _new_conut == 0 {
                return;
            }
            
            // try read from index storage
            info!("reading {} images from local storage...", _new_conut);
            let mut _read_from_local = 0;
            for k in request_list.iter().cloned() {
                let cache_line = match read_image(k).await {
                    Ok(cache_line) => cache_line,
                    Err(e) => {
                        error!("failed to read cached img from indexed db: {:#?}", e);
                        return;
                    }
                };

                if let Some(img) = cache_line {
                    loaded_tiles.write().insert(k, img.img_b64);
                    _read_from_local += 1;
                }
            }
            let request_list = request_list.iter().filter(|k| !loaded_tiles.peek().contains_key(k)).cloned().collect::<Vec<_>>();
            info!("found {} images in local storage.", _read_from_local);
            let _new_conut = request_list.len();
            if _new_conut == 0 {
                return;
            }
            
            info!("reading tile list started for {_new_conut} imgs new / {_total_count} total");
            match  get_tile_list(request_list.clone()).await {
                Ok(x) => {
                    let mut stream = x.into_inner();
                    use futures_util::stream::StreamExt;
                    let mut i = 0;
                    let mut buf = "".to_string();
                    while let Some(Ok(chunk)) = stream.next().await {
                        buf.push_str(&chunk);
                        let mut lines = vec![];
                        while let Some(newline_pos) = buf.find("\n") {
                            lines.push(buf[..newline_pos].to_string());
                            buf = buf[(newline_pos+1)..].to_string();
                        }
                        for line in lines {
                            let fields :Vec<_> = line.splitn(5, "|").collect();
                            // info!("FIELDS: {:?}", fields);
                            let is_ok = fields[0] == "ok";
                            let sq_z = fields[1].parse().unwrap_or(0);
                            let sq_x = fields[2].parse().unwrap_or(0);
                            let sq_y = fields[3].parse().unwrap_or(0);
                            let body = fields[4];
                            if is_ok {
                                loaded_tiles.write().insert((sq_z, sq_x, sq_y), body.to_string());
                                // async_std::task::sleep(std::time::Duration::from_millis(1)).await;
                                if let Err(e) = write_image((sq_z, sq_x, sq_y), &body).await {
                                    error!("failed to write downloaded image to local storage: {:#?}", e);
                                }
                            } else {
                                warn!("stream err:  img z={sq_z}/x={sq_x}/y={sq_y}: {body}");
                            }
                            i += 1;
                            if i == request_list.len() {
                                info!("img stream finished all {} tiles.", i);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("err fetching img list from  server: {:#?}", e);
                    return;
                }
            }
        };
        
        let (abort_handle, abortable_future) = {
            use futures::future::{Abortable, AbortHandle, Aborted};
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let future = Abortable::new(do_stuff, abort_registration);
            (abort_handle, future)
        };
        futures::pin_mut!(abortable_future);
        match futures::future::select(abortable_future, cancel_future).await {
            futures::future::Either::Left((_r, _f)) => {
                info!("stream finished.");
            },
            futures::future::Either::Right((_r, _f)) => {
                abort_handle.abort();
                info!("abort() called; waiting for err");
                assert!(_f.is_aborted());
                let _abort_w = _f.await;
                info!("abort() finished waiting.");
                assert!(_abort_w.is_err());

                if let Err(e) = cancel_ack_tx.send(()).await {
                    error!("failed to send cancel ack message: {:?}", e);
                }
                info!("ZZZZ: cancelling stream.");
                return;
            },
        }
    });


    let _r = use_resource(move || async move {
        let z2 = squares_in_view.read().len();
        if z2 > 0 {
            if !fut.finished() {
                if let Err(e) = cancel.peek().0.clone().send(()) {
                    warn!("cancel failed!!! {:?}", e);
                }
                info!("cancel sent, waiting for resp...");
                if let Err(e) = cancel_ack.peek().1.recv().await {
                    warn!("cancel_ack read failed: {:?}!", e);
                }
                info!("cancel ack received.");
            }
            fut.cancel();

            // clear unused keys
            let zl = std::collections::HashSet::<(i32,i32,i32)>::from_iter(squares_in_view.peek().iter().cloned());
            let to_delete = loaded_tiles.peek().keys().filter(|k| !zl.contains(k)).cloned().collect::<Vec<_>>();

            if !to_delete.is_empty() {
                for d in to_delete {
                    loaded_tiles.write().remove(&d);
                }
            }

            fut.restart();
            info!("future restarted.");
        }
    });

    rsx! {
        h3 { "zoom = {zoom:?} pos = {pos:?}" }
        ul {
            id:"main_display_list",
            style:"list-style-type:none;margin:0;padding:0;",

            for (sq_z, sq_x, sq_y) in squares_in_view.read().iter().cloned() {
                li {
                    key: "tile_li_{sq_z}_{sq_x}_{sq_y}",
                    MapsTile {
                        zoom, pos, dimensions,
                        sq_x, sq_y, sq_z,
                        loaded_tiles,
                        // src: //use_memo(move || {
                            
                        //}.into())
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
    // src: ReadOnlySignal<String>,
    loaded_tiles: ReadOnlySignal<HashMap::<(i32,i32,i32),String>>,
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

    let src = use_memo(move || loaded_tiles.read().get(&(sq_z, sq_x, sq_y)).cloned().unwrap_or("".to_string()));

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

// #[server(GetServerTile)]
use server_fn::codec::StreamingText;
use server_fn::codec::TextStream;

#[server(output = StreamingText)]
async fn get_tile_list(list: Vec<(i32,i32,i32)>) -> Result<TextStream, ServerFnError> {
    let (mut tx, rx) = async_channel::bounded(1);
    tokio::spawn(async move {
        use futures::stream::FuturesUnordered;
        let list_len = list.len();
        info!("server: feteching {} img", list_len);
        let mut fut_unordered = FuturesUnordered::from_iter(list.into_iter().map(get_server_tile_img));
        use futures_util::StreamExt;

        let mut success_count = 0;
        let mut err_count = 0;
        while let Some((coord, result)) = fut_unordered.next().await {
            let msg = match result {
                Ok(result) => {
                    success_count += 1;
                    format!("ok|{}|{}|{}|{}\n", coord.0, coord.1, coord.2, result)

                },
                Err(err) => {
                    err_count += 1;
                    let err = format!("{:?}", err).as_str()[0..10].to_string();
                    format!("err|{}|{}|{}|{}\n", coord.0, coord.1, coord.2, err)
                }
            };
            
            let _x = tx.send(Ok(msg)).await;
            if let Err(err) = _x {
                warn!("cut streaming {} img / {} planned: {:?}",  success_count+err_count, list_len, err);
                tx.close();
                return;
            }
            
        }
        info!("done streaming {} img: {} success  / {} fail.", list_len, success_count, err_count);
    });

    Ok(TextStream::new(rx))
}

async fn get_server_tile_img(coord: (i32,i32,i32)) -> ((i32,i32,i32), Result<String, ServerFnError>) {
    const RETRIES: u32 = 5; // ~ 64s
    for x in 1..=RETRIES {
        match get_server_tile_img_once(coord).await {
            Ok(r) => {return (coord, Ok(r));},
            Err(r) => {
                if x == RETRIES {
                    return (coord, Err(r));
                }
                let sleep_ms = x as u64 * 250 * 2_u64.pow(x);
                // info!("failed to get tile img; chance {x}/{RETRIES}; sleep {sleep_ms}ms");
                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                continue;
            }
        }
    }
    unreachable!();
}

async fn get_server_tile_img_once(coord: (i32,i32,i32)) -> Result<String, ServerFnError> {
    let (sq_z, sq_x, sq_y) = coord;
    let url = format!("http://localhost:8000/api/tile/google_hybrid/{sq_z}/{sq_x}/{sq_y}/jpg");

    let response = reqwest::get(&url).await?;
    let status_code = response.status().clone();
    let content_type = response.headers()["Content-Type"].to_str()?.to_string();
    if !status_code.is_success() ||  content_type.len() < 4 {
        return Err(ServerFnError::new(format!("bad response from tile server: {:?}", status_code)));
    }
    
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

