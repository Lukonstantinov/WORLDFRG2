use tauri::State;
use crate::db::{WorldDb, tile_store, metadata};
use crate::tile::coords::TILE_SIZE;
use crate::tile::cell::TileData;
use crate::history::undo;

/// Load an image as a land/sea template.
/// Receives a file path, reads the image, auto-detects land/sea,
/// and applies to the world grid.
#[tauri::command]
pub fn load_image_template(
    path: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get world dimensions
    let grid_width: u32 = metadata::get_meta_required(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    let grid_height: u32 = metadata::get_meta_required(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;

    // Read image from file path
    eprintln!("[template] load_image_template called with path={:?}, grid={}x{}", path, grid_width, grid_height);
    let img = image::open(&path).map_err(|e| {
        eprintln!("[template] image::open failed: {}", e);
        format!("Failed to load image: {}", e)
    })?;
    eprintln!("[template] image decoded OK, dimensions={}x{}", img.width(), img.height());
    let img = img.resize_exact(grid_width, grid_height, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();

    // Auto-detect land/sea using dominant color analysis
    let terrain = detect_land_sea(&rgba, grid_width, grid_height);

    // Apply terrain to tiles
    let tiles_x = ((grid_width + TILE_SIZE - 1) / TILE_SIZE) as i32;
    let tiles_y = ((grid_height + TILE_SIZE - 1) / TILE_SIZE) as i32;

    // Save old states for undo
    let mut old_states: Vec<(i32, i32, Vec<u8>)> = Vec::new();
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let old = tile_store::load_tile(&conn, tx, ty, 0)
                .map_err(|e| e.to_string())?
                .unwrap_or_else(TileData::new_sea);
            old_states.push((tx, ty, old.compress()));
        }
    }
    undo::push_undo(&conn, "Load image template", &old_states)?;

    // Apply terrain to each tile
    let mut modified = Vec::new();
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let mut tile = tile_store::load_tile(&conn, tx, ty, 0)
                .map_err(|e| e.to_string())?
                .unwrap_or_else(TileData::new_sea);

            let tile_w = TILE_SIZE.min(grid_width - (tx as u32 * TILE_SIZE));
            let tile_h = TILE_SIZE.min(grid_height - (ty as u32 * TILE_SIZE));

            for ly in 0..tile_h {
                for lx in 0..tile_w {
                    let wx = tx as u32 * TILE_SIZE + lx;
                    let wy = ty as u32 * TILE_SIZE + ly;
                    let world_idx = (wy * grid_width + wx) as usize;
                    let tile_idx = (ly * TILE_SIZE + lx) as usize;

                    tile.terrain[tile_idx] = terrain[world_idx];
                    if terrain[world_idx] == 0 {
                        tile.elevation[tile_idx] = 0.0;
                    }
                }
            }

            tile_store::save_tile(&conn, tx, ty, 0, &tile).map_err(|e| e.to_string())?;
            modified.push((tx, ty));
        }
    }

    eprintln!("[template] done, {} tiles modified", modified.len());
    Ok(modified)
}

/// Auto-detect land/sea from image pixels.
/// Strategy:
/// 1. If image has significant transparency (>10% transparent pixels),
///    use alpha directly: opaque = land, transparent = ocean
/// 2. Otherwise, find dominant opaque color via 4-bit quantization:
///    a. If dominant is bright (avg > 200), white = ocean, colored = land
///    b. Otherwise, similar to dominant = ocean, different = land
fn detect_land_sea(
    rgba: &image::RgbaImage,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let total = (width * height) as usize;
    let mut terrain = vec![0u8; total];

    // Count transparent vs opaque pixels
    let mut opaque_count = 0u32;
    let mut transparent_count = 0u32;

    for y in 0..height {
        for x in 0..width {
            let p = rgba.get_pixel(x, y);
            if p[3] < 128 {
                transparent_count += 1;
            } else {
                opaque_count += 1;
            }
        }
    }

    if opaque_count == 0 {
        return terrain; // all transparent = all sea
    }

    // If image has significant transparency, use alpha channel directly
    let transparency_ratio = transparent_count as f32 / total as f32;
    if transparency_ratio > 0.10 {
        eprintln!("[template] using alpha-based detection ({:.0}% transparent)", transparency_ratio * 100.0);
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let p = rgba.get_pixel(x, y);
                terrain[idx] = if p[3] >= 128 { 1 } else { 0 };
            }
        }
        return terrain;
    }

    // Fully opaque image: use color-based detection
    // Build 4-bit color histogram (4096 bins)
    let mut hist = vec![0u32; 4096];
    for y in 0..height {
        for x in 0..width {
            let p = rgba.get_pixel(x, y);
            let bin = ((p[0] as usize >> 4) << 8) | ((p[1] as usize >> 4) << 4) | (p[2] as usize >> 4);
            hist[bin] += 1;
        }
    }

    // Find dominant color bin
    let dominant_bin = hist.iter().enumerate().max_by_key(|(_, &c)| c).map(|(i, _)| i).unwrap_or(0);
    let dom_r = ((dominant_bin >> 8) & 0xF) as u8 * 17;
    let dom_g = ((dominant_bin >> 4) & 0xF) as u8 * 17;
    let dom_b = (dominant_bin & 0xF) as u8 * 17;
    let dom_avg = (dom_r as u16 + dom_g as u16 + dom_b as u16) / 3;

    eprintln!("[template] using color-based detection, dominant=({},{},{}) avg={}", dom_r, dom_g, dom_b, dom_avg);

    // Classify pixels
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let p = rgba.get_pixel(x, y);

            if dom_avg > 200 {
                // Dominant is bright/white: white pixels = ocean, colored = land
                let is_white = p[0] > 200 && p[1] > 200 && p[2] > 200;
                terrain[idx] = if is_white { 0 } else { 1 };
            } else {
                // Dominant is dark/colored: similar to dominant = ocean, different = land
                let dist = (p[0] as i32 - dom_r as i32).abs()
                    + (p[1] as i32 - dom_g as i32).abs()
                    + (p[2] as i32 - dom_b as i32).abs();
                terrain[idx] = if dist > 80 { 1 } else { 0 };
            }
        }
    }

    terrain
}
