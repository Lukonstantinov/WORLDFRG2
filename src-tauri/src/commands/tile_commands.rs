use serde::Serialize;
use tauri::State;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
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
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut results = Vec::new();

    for &(tx, ty) in &tiles {
        let tile = tile_store::load_tile(&conn, tx, ty, lod)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(TileData::new_sea);

        let version = tile_store::get_tile_version(&conn, tx, ty, lod)
            .map_err(|e| e.to_string())?;

        for layer in &layers {
            let rgba_bytes = tile_image::render_tile(&tile, layer);
            results.push(TileResponse {
                tx,
                ty,
                layer: layer.clone(),
                version,
                rgba: BASE64.encode(&rgba_bytes),
            });
        }
    }

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
