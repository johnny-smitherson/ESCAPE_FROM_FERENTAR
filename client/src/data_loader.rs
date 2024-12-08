use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info, warn};

use std::collections::HashMap;

use crate::index_db::{read_image, write_image};
use base64::Engine;

pub(crate) fn _use_handle_data_loading(
    squares_to_load: ReadOnlySignal<Vec<(i32, i32, i32)>>,
    mut map_tile_is_loaded: Signal<HashMap<(i32, i32, i32), bool>>,
    mut map_tile_data: Signal<HashMap<(i32, i32, i32), String>>,
) {
    // debounce the squares changing, so we debounce the whole load process
    let mut squares_in_view = use_signal(Vec::<(i32, i32, i32)>::new);
    let mut debounce_update_squares =
        dioxus_sdk::utils::timing::use_debounce(std::time::Duration::from_millis(100), move |_| {
            squares_in_view.set(squares_to_load.peek().clone());
        });
    use_effect(move || {
        let _ = squares_to_load.read();
        debounce_update_squares.action(());
    });
    let squares_in_view = use_memo(move || squares_in_view.read().clone());

    // vvvvvvvvvvvv
    let cancel = use_signal(|| tokio::sync::broadcast::channel::<()>(1));
    let cancel_ack = use_signal(|| async_channel::bounded::<()>(1));
    // vvvvvvvvvvvv
    macro_rules! await_cancel2 {
        ($expression:expr) => {{
            let _expr = $expression;
            let mut cancel_rx = cancel.peek().0.subscribe();
            let cancel_future = cancel_rx.recv();
            use futures::future::{AbortHandle, Abortable};

            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let _expr = Abortable::new(_expr, abort_registration);

            futures::pin_mut!(cancel_future);
            futures::pin_mut!(_expr);
            match futures::future::select(_expr, cancel_future).await {
                futures::future::Either::Left((_r, _f)) => _r.expect("aborted??!?"),
                futures::future::Either::Right((_r, _f)) => {
                    abort_handle.abort();
                    assert!(_f.is_aborted());
                    assert!(_f.await.is_err());
                    let cancel_ack_tx = cancel_ack.peek().0.clone();
                    if let Err(e) = cancel_ack_tx.send(()).await {
                        error!("failed to send cancel ack message: {:?}", e);
                    }
                    // info!(
                    //     "await_cancel(): >> {} <<: cancelling stream.",
                    //     stringify!($expression)
                    // );
                    return;
                }
            }
        }};
    }
    // ^^^^^^^^^^^^
    // ^^^^^^^^^^^^

    let mut clear_unused_keys = move || {
        let total_item_count = squares_in_view.peek().len();
        if total_item_count < 500 {
            return;
        }
        let mut _count = 0;
        let zl = std::collections::HashSet::<(i32, i32, i32)>::from_iter(
            squares_in_view.peek().iter().cloned(),
        );
        let to_delete = map_tile_is_loaded
            .peek()
            .keys()
            .filter(|k| !zl.contains(k))
            .cloned()
            .collect::<Vec<_>>();

        if !to_delete.is_empty() {
            for d in to_delete {
                map_tile_is_loaded.write().remove(&d);
                map_tile_data.write().remove(&d);
                _count += 1;
            }
        }
        if _count > 0 {
            info!("cleared {_count} unused keys / {total_item_count} total");
        }
    };

    let filter_loaded_keys = move |list: &Vec<_>| {
        list.iter()
            .filter(|k| !map_tile_is_loaded.peek().contains_key(k))
            .cloned()
            .collect::<Vec<_>>()
    };

    let mut fut = use_future(move || async move {
        let do_stuff = async move {
            let _total_count = squares_in_view.len();
            let request_list = filter_loaded_keys(squares_in_view.read().as_ref());
            let _new_conut = request_list.len();
            if _new_conut == 0 {
                return;
            }

            // try read from index storage
            // info!("reading {} images from local storage...", _new_conut);
            let mut _read_from_local = 0;
            // use futures::stream::FuturesUnordered;
            // pop is slow
            for k in request_list.iter().cloned() {
                let cache_line = match read_image(k).await {
                    Ok(cache_line) => cache_line,
                    Err(e) => {
                        error!("failed to read cached img from indexed db: {:#?}", e);
                        return;
                    }
                };

                if let Some(img) = cache_line {
                    map_tile_data.write().insert(k, img.img_b64);
                    map_tile_is_loaded.write().insert(k, true);
                    _read_from_local += 1;
                }
            }
            // info!("found {} images in local storage.", _read_from_local);

            let request_list = filter_loaded_keys(&request_list);
            let _new_conut = request_list.len();
            if _new_conut == 0 {
                return;
            }

            // info!("reading tile list started for {_new_conut} imgs new / {_total_count} total");
            let request_list_len = request_list.len();
            match get_tile_list(request_list).await {
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
                            buf = buf[(newline_pos + 1)..].to_string();
                        }
                        for line in lines {
                            let fields: Vec<_> = line.splitn(5, "|").collect();
                            // info!("FIELDS: {:?}", fields);
                            let is_ok = fields[0] == "ok";
                            let is_ping = fields[0] == "ping";
                            if is_ping {
                                info!("pong");
                                continue;
                            }
                            let sq_z = fields[1].parse().unwrap_or(0);
                            let sq_x = fields[2].parse().unwrap_or(0);
                            let sq_y = fields[3].parse().unwrap_or(0);
                            let body = fields[4];
                            if is_ok {
                                map_tile_data
                                    .write()
                                    .insert((sq_z, sq_x, sq_y), body.to_string());
                                map_tile_is_loaded.write().insert((sq_z, sq_x, sq_y), true);
                                // async_std::task::sleep(std::time::Duration::from_millis(1)).await;
                                if let Err(e) = write_image((sq_z, sq_x, sq_y), &body).await {
                                    error!(
                                        "failed to write downloaded image to local storage: {:#?}",
                                        e
                                    );
                                }
                            } else {
                                warn!("stream err:  img z={sq_z}/x={sq_x}/y={sq_y}: \n{body}");
                            }
                            i += 1;
                            if i == request_list_len {
                                // info!("img stream finished all {} tiles.", i);
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

        await_cancel2!(do_stuff);
        clear_unused_keys();
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
                // info!("cancel ack received.");
            }
            fut.cancel();
            fut.restart();
            // info!("future restarted.");
        }
    });
    _r.read();
}

// #[server(GetServerTile)]
use server_fn::codec::StreamingText;
use server_fn::codec::TextStream;

#[server(output = StreamingText)]
async fn get_tile_list(list: Vec<(i32, i32, i32)>) -> Result<TextStream, ServerFnError> {
    let (tx, rx) = async_channel::bounded(1);
    const PINGPONG_INTERVAL: f32 = 1.0;
    const SEND_TIMEOUT: f32 = 5.0;

    // let (mut tx, rx) = futures::channel::mpsc::unbounded();
    tokio::spawn(async move {
        let send_msg = move |msg| {
            let mut tx2 = tx.clone();
            async move {
                if tx2.is_closed() {
                    anyhow::bail!("already closed.");
                }
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs_f32(SEND_TIMEOUT),
                    tx2.send(Ok(msg)),
                )
                .await
                {
                    Err(e) => {
                        tx2.close();
                        anyhow::bail!("timeout: {e}");
                    }
                    Ok(Err(e)) => {
                        tx2.close();
                        anyhow::bail!("send err: {e}");
                    }
                    Ok(Ok(r)) => {
                        return Ok(());
                    }
                }
            }
        };

        let ping_str = "ping|{}|{}|{}|pong\n";

        if let Err(e) = send_msg(ping_str.to_string()).await {
            warn!("fail to send first ping: {e}");
            return;
        }

        use futures::stream::FuturesUnordered;
        let list_len = list.len();
        info!("server: feteching {} img", list_len);
        let mut fut_unordered =
            FuturesUnordered::from_iter(list.into_iter().map(get_server_tile_img));
        use futures_util::StreamExt;

        let mut success_count = 0;
        let mut err_count = 0;
        loop {
            let msg = match tokio::time::timeout(
                tokio::time::Duration::from_secs_f32(PINGPONG_INTERVAL),
                fut_unordered.next(),
            )
            .await
            {
                Err(_timeout) => {
                    // no new traffic - send ping
                    info!("sending ping...");
                    ping_str.to_string()
                }
                Ok(None) => {
                    // no new futures - stop streaming
                    info!(
                        "done streaming {} img: {} success  / {} fail.",
                        list_len, success_count, err_count
                    );
                    return;
                }
                Ok(Some((coord, result))) => match result {
                    Ok(result) => {
                        success_count += 1;
                        format!("ok|{}|{}|{}|{}\n", coord.0, coord.1, coord.2, result)
                    }
                    Err(err) => {
                        err_count += 1;
                        let err = format!("{:?}", err).as_str()[0..10].to_string();
                        format!("err|{}|{}|{}|{}\n", coord.0, coord.1, coord.2, err)
                    }
                },
            };
            if let Err(e) = send_msg(msg).await {
                warn!(
                    "cut streaming {} img / {} planned: {}",
                    success_count + err_count,
                    list_len,
                    e
                );
                return;
            }
        }
        info!("stream done.");
    });

    Ok(TextStream::new(rx))
}

#[cfg(feature = "server")]
async fn get_server_tile_img(
    coord: (i32, i32, i32),
) -> ((i32, i32, i32), Result<String, ServerFnError>) {
    const RETRIES: u32 = 5; // ~ 64s
    for x in 1..=RETRIES {
        match get_server_tile_img_once(coord).await {
            Ok(r) => {
                return (coord, Ok(r));
            }
            Err(r) => {
                info!("ERR {x}/{RETRIES}: {:#?}", r);
                if x == RETRIES {
                    return (coord, Err(ServerFnError::new(format!("{r}"))));
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

#[cfg(feature = "server")]
async fn get_server_tile_img_once(coord: (i32, i32, i32)) -> anyhow::Result<String> {
    let (sq_z, sq_x, sq_y) = coord;
    // let url = format!("http://localhost:8000/api/tile/google_hybrid/{sq_z}/{sq_x}/{sq_y}/jpg");
    // let url = format!("https://tile.openstreetmap.org/{sq_z}/{sq_x}/{sq_y}.png");
    let url = format!("https://mt1.google.com/vt/lyrs=y&x={sq_x}&y={sq_y}&z={sq_z}");

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Ubuntu; Linux i686; rv:133.0) Gecko/20100101 Firefox/133.0")
        .build()?;

    let response = anyhow::Context::context(client.get(&url).send().await, "reqwest send error:")?;
    let status_code = response.status().clone();
    let content_type = response
        .headers()
        .get("Content-Type")
        .map(|x| x.to_str().unwrap_or("image/png"))
        .unwrap_or("image/png")
        .to_string();
    if !status_code.is_success() || content_type.len() < 4 {
        anyhow::bail!(
            "bad response from tile server: {:?}, url:{:?} err: {:?}",
            status_code,
            url,
            response.text().await
        );
    }

    let resp_bytes = anyhow::Context::context(response.bytes().await, "reqwest read bytes: ")?;
    let resp_base64 = base64::prelude::BASE64_STANDARD.encode(resp_bytes);

    let img_src = format!("data:{content_type};base64,{resp_base64}");
    Ok(img_src)
}
