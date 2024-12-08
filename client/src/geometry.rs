
use crate::_const::{MAX_Z, MIN_Z, REF_Z};

fn get_tile_positions_one_level(
    pos: (f64, f64),
    zoom: f64,
    dimensions: (f64, f64),
    max_pic_pixels: f64,
) -> Vec<(i32, i32, i32)> {
    let vmin_px = f64::min(dimensions.0, dimensions.1);
    let min_dim_tiles = vmin_px / max_pic_pixels;
    let ideal_tile_level = (f64::trunc(f64::log2(min_dim_tiles) + zoom) as i32).clamp(MIN_Z, MAX_Z);

    let x0 = (pos.0 / f64::exp2(REF_Z - ideal_tile_level as f64)).floor() as i32;
    let y0 = (pos.1 / f64::exp2(REF_Z - ideal_tile_level as f64)).floor() as i32;
    let remainder0 = f64::exp2(ideal_tile_level as f64) as i32;
    let x0 = x0.rem_euclid(remainder0);
    let y0 = y0.rem_euclid(remainder0);

    let mut ze_squarez = vec![];
    let tile_diff_exp = f64::exp(f64::fract(f64::log2(min_dim_tiles) + zoom));
    let tile_count_x = (dimensions.0 / vmin_px / tile_diff_exp * min_dim_tiles + 1.1).ceil() as i32 + 1;
    let tile_count_y = (dimensions.1 / vmin_px / tile_diff_exp * min_dim_tiles + 1.1).ceil() as i32 + 1;
    for i in (x0 - tile_count_x)..=(x0 + tile_count_x) {
        for j in (y0 - tile_count_y)..=(y0 + tile_count_y) {
            ze_squarez.push((
                ideal_tile_level,
                i.rem_euclid(remainder0),
                j.rem_euclid(remainder0),
            ));
        }
    }

    ze_squarez
}

/// Computes (squares to load in memory, squares to put on screen)
pub(crate) fn get_tile_positions(
    pos: (f64, f64),
    zoom: f64,
    dimensions: (f64, f64),
) -> Vec<(i32, i32, i32)> {
    if zoom < MIN_Z as f64 - 0.0001 {
        return vec![];
    }
    const IMG_MAX_PX: f64 = 256.0+128.0;
    let mut current_px = IMG_MAX_PX;
    let mut all_sq = vec![];
    for _ in 0..5 {
        let mut new_sq = get_tile_positions_one_level(pos, zoom, dimensions, current_px);
        if new_sq.is_empty() {
            break;
        }
        let current_z = new_sq[0].0;
        all_sq.append(&mut new_sq);
        if current_z == MIN_Z {
            break;
        }
        current_px *= 2.0;
    }
    all_sq.sort();
    all_sq.dedup();
    all_sq
}
