//! Trade-good library commands.
//!
//! Two layers of storage (see the design doc):
//!   • **Per-world** — the active spec list is snapshotted into the world DB
//!     `metadata` key `goods_spec`, so a saved `.worldforge` reproduces its exact
//!     goods regardless of later library edits. `load_world_goods` reads it
//!     (falling back to the shipped defaults) and is used by generation + the
//!     trade matrix.
//!   • **Global library** — a `goods_library.json` in the app config dir; the
//!     editing template for new worlds, carried across worlds.

use tauri::{Manager, State};

use crate::db::{metadata, WorldDb};
use crate::sim::goods_spec::{default_list, GoodSpec};

/// Active good specs for the current world: the per-world snapshot if present,
/// otherwise the shipped defaults. Used by generation and the trade matrix so a
/// world always generates/reports the goods it was authored with.
pub fn load_world_goods(conn: &rusqlite::Connection) -> Vec<GoodSpec> {
    let mut specs: Vec<GoodSpec> = match metadata::get_meta(conn, "goods_spec") {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_else(|_| default_list()),
        _ => default_list(),
    };
    // Worlds snapshotted before the market fields existed load with empty
    // categories — fill them so the economy solver always has real values.
    crate::sim::goods_spec::backfill_market_fields(&mut specs);
    specs
}

/// The shipped 30-good defaults (used for "reset to default" in the editor).
#[tauri::command]
pub fn default_goods() -> Vec<GoodSpec> {
    default_list()
}

/// The current world's active good specs (per-world snapshot or defaults).
#[tauri::command]
pub fn get_goods_spec(db: State<'_, WorldDb>) -> Result<Vec<GoodSpec>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(load_world_goods(&conn))
}

/// Snapshot a good-spec list into the current world (used before generation).
#[tauri::command]
pub fn set_goods_spec(db: State<'_, WorldDb>, specs: Vec<GoodSpec>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&specs).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "goods_spec", &json).map_err(|e| e.to_string())
}

/// A downsampled suitability heatmap for one good spec (Goods Editor preview).
#[derive(serde::Serialize)]
pub struct PreviewGrid {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // row-major 0..255 scores
    /// Row-major land mask (1 = land, 0 = sea) at the same resolution, so the
    /// preview can draw the world's coastline outline behind the heatmap.
    pub land: Vec<u8>,
}

/// Score where a (possibly unsaved) good spec would place across the current
/// world — a live preview for the Goods Editor, no regeneration needed.
#[tauri::command]
pub fn preview_good_score(spec: GoodSpec, db: State<'_, WorldDb>) -> Result<PreviewGrid, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Load ONLY the columns the suitability scorer reads (good_score / envelope_score).
    // Loading ColumnSet::ALL here pulled in the 45-wide `goods` array plus the
    // salinity/shark/shipworm/storm/reef/disease columns — none of which the preview
    // touches — allocating well over a gigabyte on a large world just to draw a
    // 220×110 thumbnail. A failed allocation aborts the process, so clicking a good
    // to preview its minimap could crash the app. This minimal set avoids that.
    use crate::sim::world_buffer::ColumnSet;
    let cols = ColumnSet::TERRAIN | ColumnSet::ELEVATION | ColumnSet::SEA_DEPTH
        | ColumnSet::SHELF | ColumnSet::CURRENTS | ColumnSet::DIST_OCEAN
        | ColumnSet::TEMPERATURE | ColumnSet::PRECIPITATION | ColumnSet::KOPPEN
        | ColumnSet::VOLCANIC | ColumnSet::FERTILITY | ColumnSet::FISHERY
        | ColumnSet::HABITABILITY;
    let buf = crate::sim::world_buffer::WorldBuffer::load_with(&conn, cols)?;
    let (pw, ph) = (220u32, 110u32);
    let data = crate::sim::biological::preview_score_grid(&buf, &spec, pw, ph);
    // Coarse land mask at the same resolution (same nearest-sample mapping as
    // preview_score_grid) so the UI can outline the landmass behind the heatmap.
    let mut land = vec![0u8; (pw * ph) as usize];
    if buf.width > 0 && buf.height > 0 {
        for py in 0..ph {
            for px in 0..pw {
                let x = (px * buf.width / pw).min(buf.width - 1);
                let y = (py * buf.height / ph).min(buf.height - 1);
                land[(py * pw + px) as usize] = if buf.terrain[buf.idx(x, y)] == 1 { 1 } else { 0 };
            }
        }
    }
    Ok(PreviewGrid { width: pw, height: ph, data, land })
}

/// Lightweight land/sea mask at 220×110 for minimaps — loads only TERRAIN.
/// Returns `{width, height, data: [], land}` where `land[i]` is 1 for land, 0 for sea.
#[tauri::command]
pub fn preview_land_grid(db: State<'_, WorldDb>) -> Result<PreviewGrid, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    use crate::sim::world_buffer::ColumnSet;
    let buf = crate::sim::world_buffer::WorldBuffer::load_with(&conn, ColumnSet::TERRAIN)?;
    let (pw, ph) = (220u32, 110u32);
    let mut land = vec![0u8; (pw * ph) as usize];
    if buf.width > 0 && buf.height > 0 {
        for py in 0..ph {
            for px in 0..pw {
                let x = (px * buf.width / pw).min(buf.width - 1);
                let y = (py * buf.height / ph).min(buf.height - 1);
                land[(py * pw + px) as usize] = if buf.terrain[buf.idx(x, y)] == 1 { 1 } else { 0 };
            }
        }
    }
    Ok(PreviewGrid { width: pw, height: ph, data: vec![], land })
}

fn library_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("goods_library.json"))
}

/// The global good library (editing template for new worlds). Returns the shipped
/// defaults if no library file exists yet.
#[tauri::command]
pub fn get_goods_library(app: tauri::AppHandle) -> Result<Vec<GoodSpec>, String> {
    let path = library_path(&app)?;
    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).map_err(|e| e.to_string()),
        Err(_) => Ok(default_list()),
    }
}

/// Persist the global good library.
#[tauri::command]
pub fn save_goods_library(app: tauri::AppHandle, specs: Vec<GoodSpec>) -> Result<(), String> {
    let path = library_path(&app)?;
    let json = serde_json::to_string_pretty(&specs).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// The post-generation goods placement report (see
/// `biological::GoodsPlacementReport`). Read-only; returns an empty report on a
/// world generated before the report existed, rather than erroring — an old world
/// simply has nothing to say here.
#[tauri::command]
pub fn get_goods_report(
    db: State<'_, WorldDb>,
) -> Result<crate::sim::biological::GoodsPlacementReport, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let raw = crate::db::metadata::get_meta(&conn, "goods_report")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if raw.is_empty() {
        return Ok(Default::default());
    }
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}
