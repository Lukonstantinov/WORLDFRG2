use serde::Serialize;
use tauri::State;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rayon::prelude::*;
use crate::db::{WorldDb, tile_store};
use crate::render::tile_image;
use crate::tile::cell::TileData;

#[derive(Serialize)]
pub struct TileResponse {
    pub tx: i32,
    pub ty: i32,
    pub layer: String,
    pub version: i64,
    pub rgba: String, // base64-encoded RGBA pixels
}

#[tauri::command]
pub fn get_tiles(
    tiles: Vec<(i32, i32)>,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    // Fetch the compressed blobs + versions under the lock (cheap memcpy), then
    // release it so the CPU-bound decompress → render → base64 runs in parallel
    // off-lock instead of serializing every tile behind the DB mutex.
    let raw: Vec<(i32, i32, i64, Option<Vec<u8>>)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        tiles
            .iter()
            .map(|&(tx, ty)| {
                let bv = tile_store::load_blob_with_version(&conn, tx, ty, lod)
                    .map_err(|e| e.to_string())?;
                Ok(match bv {
                    Some((version, blob)) => (tx, ty, version, Some(blob)),
                    None => (tx, ty, 0, None),
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    let results = raw
        .par_iter()
        .flat_map_iter(|(tx, ty, version, blob)| {
            let tile = match blob {
                Some(b) => TileData::decompress(b),
                None => TileData::new_sea(),
            };
            layers.iter().map(move |layer| {
                let rgba_bytes = tile_image::render_tile(&tile, layer);
                TileResponse {
                    tx: *tx,
                    ty: *ty,
                    layer: layer.clone(),
                    version: *version,
                    rgba: BASE64.encode(&rgba_bytes),
                }
            }).collect::<Vec<_>>()
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn get_tile_range(
    tx_min: i32,
    tx_max: i32,
    ty_min: i32,
    ty_max: i32,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    let mut tile_coords = Vec::new();
    for ty in ty_min..=ty_max {
        for tx in tx_min..=tx_max {
            tile_coords.push((tx, ty));
        }
    }
    get_tiles(tile_coords, layers, lod, db)
}
