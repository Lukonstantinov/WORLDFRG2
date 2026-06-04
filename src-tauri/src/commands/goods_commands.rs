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
    match metadata::get_meta(conn, "goods_spec") {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_else(|_| default_list()),
        _ => default_list(),
    }
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
}

/// Score where a (possibly unsaved) good spec would place across the current
/// world — a live preview for the Goods Editor, no regeneration needed.
#[tauri::command]
pub fn preview_good_score(spec: GoodSpec, db: State<'_, WorldDb>) -> Result<PreviewGrid, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let buf = crate::sim::world_buffer::WorldBuffer::load(&conn)?;
    let (pw, ph) = (140u32, 70u32);
    let data = crate::sim::biological::preview_score_grid(&buf, &spec, pw, ph);
    Ok(PreviewGrid { width: pw, height: ph, data })
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
