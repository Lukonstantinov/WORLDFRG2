//! Layered world import: copy chosen layer GROUPS (terrain, climate, soil,
//! hazards, goods, hydrology) from another `.worldforge` file into the current
//! world, leaving everything else untouched. Reuses the column-mask machinery
//! (`ColumnSet` + `TileData::merge_columns`).

use rayon::prelude::*;
use rusqlite::Connection;
use tauri::State;
use crate::db::{metadata, tile_store, WorldDb};
use crate::history::undo;
use crate::sim::world_buffer::ColumnSet;
use crate::tile::cell::TileData;
use crate::tile::coords::TILE_SIZE;

/// Tile columns + extra metadata copied for each importable group.
fn group_columns(group: &str) -> Option<ColumnSet> {
    Some(match group {
        "terrain" => ColumnSet::TERRAIN
            | ColumnSet::ELEVATION
            | ColumnSet::SEA_DEPTH
            | ColumnSet::SHELF
            | ColumnSet::PLATES
            | ColumnSet::VOLCANIC
            | ColumnSet::LOCKED,
        "climate" => ColumnSet::WIND
            | ColumnSet::CURRENTS
            | ColumnSet::TEMPERATURE
            | ColumnSet::PRECIPITATION
            | ColumnSet::KOPPEN
            | ColumnSet::SALINITY
            | ColumnSet::DIST_OCEAN,
        "soil" => ColumnSet::SOIL | ColumnSet::FERTILITY | ColumnSet::FISHERY,
        "hazards" => ColumnSet::SHARK
            | ColumnSet::SHIPWORM
            | ColumnSet::STORM
            | ColumnSet::REEF
            | ColumnSet::DISEASE,
        "goods" => ColumnSet::GOODS,
        // Rivers/lakes are overlays (metadata JSON), not tile columns.
        "hydrology" => ColumnSet::NONE,
        _ => return None,
    })
}

fn src_meta(src: &Connection, key: &str) -> Option<String> {
    src.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .ok()
}

/// Import the selected layer groups from `path`. Grid dimensions must match.
/// Returns the modified tile coords (all of them when tile columns were
/// imported) so the frontend can invalidate its cache.
#[tauri::command]
pub fn import_world_layers(
    path: String,
    groups: Vec<String>,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;

    let mut cols = ColumnSet::NONE;
    for g in &groups {
        cols = cols | group_columns(g).ok_or_else(|| format!("Unknown layer group: {g}"))?;
    }
    let want_hydrology = groups.iter().any(|g| g == "hydrology");
    let want_goods = groups.iter().any(|g| g == "goods");
    let want_terrain = groups.iter().any(|g| g == "terrain");

    // Climate-and-later layers only make sense over a matching landmass: either
    // import terrain together with them, or have terrain already generated here.
    if !want_terrain && cols != ColumnSet::NONE {
        let world_done = metadata::get_meta(&conn, "world_progress")
            .ok()
            .flatten()
            .map(|j| j.contains("\"2\""))
            .unwrap_or(true); // pre-progress saves: assume the user knows
        if !world_done {
            return Err(
                "Generate or import terrain first — climate/soil/goods layers need a landmass."
                    .into(),
            );
        }
    }

    let src = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;

    // Dimensions must match cell-for-cell.
    let (gw, gh): (u32, u32) = (
        metadata::get_meta_required(&conn, "grid_width").map_err(|e| e.to_string())?
            .parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
        metadata::get_meta_required(&conn, "grid_height").map_err(|e| e.to_string())?
            .parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
    );
    let (sw, sh) = (
        src_meta(&src, "grid_width").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
        src_meta(&src, "grid_height").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
    );
    if (sw, sh) != (gw, gh) {
        return Err(format!(
            "Grid size mismatch: this world is {gw}\u{d7}{gh}, the source is {sw}\u{d7}{sh}."
        ));
    }

    // Overlay/metadata groups.
    if want_hydrology {
        for key in ["rivers", "lakes"] {
            if let Some(v) = src_meta(&src, key) {
                metadata::set_meta(&conn, key, &v).map_err(|e| e.to_string())?;
            }
        }
    }
    if want_goods {
        if let Some(v) = src_meta(&src, "goods_spec") {
            metadata::set_meta(&conn, "goods_spec", &v).map_err(|e| e.to_string())?;
        }
    }

    if cols == ColumnSet::NONE {
        return Ok(Vec::new()); // hydrology-only import: no tiles touched
    }

    let tiles_x = ((gw + TILE_SIZE - 1) / TILE_SIZE) as i32;
    let tiles_y = ((gh + TILE_SIZE - 1) / TILE_SIZE) as i32;
    let coords: Vec<(i32, i32)> = (0..tiles_y)
        .flat_map(|ty| (0..tiles_x).map(move |tx| (tx, ty)))
        .collect();

    // Snapshot current blobs for undo first (compressed, cheap).
    let mut old_states: Vec<(i32, i32, Vec<u8>)> = Vec::with_capacity(coords.len());
    for &(tx, ty) in &coords {
        let blob = tile_store::load_blob_with_version(&conn, tx, ty, 0)
            .map_err(|e| e.to_string())?
            .map(|(_, b)| b)
            .unwrap_or_else(|| TileData::new_sea().compress());
        old_states.push((tx, ty, blob));
    }
    undo::push_undo(&conn, "Import world layers", &old_states)?;

    // Merge in batches: fetch src+dst blobs, decompress+merge+compress on
    // rayon workers, write inside one transaction (bounded peak memory).
    const BATCH: usize = 256;
    let txn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (chunk_idx, batch) in coords.chunks(BATCH).enumerate() {
        let start = chunk_idx * BATCH;
        let src_blobs: Vec<Option<Vec<u8>>> = batch
            .iter()
            .map(|&(tx, ty)| {
                src.query_row(
                    "SELECT data FROM tiles WHERE tx = ?1 AND ty = ?2 AND lod = 0",
                    rusqlite::params![tx, ty],
                    |r| r.get(0),
                )
                .ok()
            })
            .collect();

        let merged: Vec<(i32, i32, Vec<u8>)> = batch
            .par_iter()
            .enumerate()
            .map(|(i, &(tx, ty))| {
                let mut dst = TileData::decompress(&old_states[start + i].2);
                let src_tile = match &src_blobs[i] {
                    Some(b) => TileData::decompress(b),
                    None => TileData::new_sea(),
                };
                dst.merge_columns(&src_tile, cols);
                (tx, ty, dst.compress())
            })
            .collect();

        for (tx, ty, blob) in &merged {
            tile_store::save_tile_blob(&txn, *tx, *ty, 0, blob).map_err(|e| e.to_string())?;
        }
    }
    txn.commit().map_err(|e| e.to_string())?;

    Ok(coords)
}
