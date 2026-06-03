use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::{WorldDb, metadata, tile_store};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    pub name: String,
    pub grid_width: u32,
    pub grid_height: u32,
    pub tile_size: u32,
}

#[tauri::command]
pub fn new_world(
    name: String,
    grid_width: u32,
    grid_height: u32,
    db: State<'_, WorldDb>,
) -> Result<WorldMeta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Clear existing data
    conn.execute_batch(
        "DELETE FROM tiles; DELETE FROM metadata; DELETE FROM objects;
         DELETE FROM sim_state; DELETE FROM undo_journal;"
    ).map_err(|e| e.to_string())?;

    // Set metadata
    metadata::set_meta(&conn, "name", &name).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "grid_width", &grid_width.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "grid_height", &grid_height.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "tile_size", "128").map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "format_version", "1").map_err(|e| e.to_string())?;

    // Initialize tiles
    tile_store::init_tiles(&conn, grid_width, grid_height).map_err(|e| e.to_string())?;

    Ok(WorldMeta {
        name,
        grid_width,
        grid_height,
        tile_size: 128,
    })
}

#[tauri::command]
pub fn get_world_meta(db: State<'_, WorldDb>) -> Result<Option<WorldMeta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let name = metadata::get_meta(&conn, "name").map_err(|e| e.to_string())?;
    let name = match name {
        Some(n) => n,
        None => return Ok(None),
    };

    let grid_width: u32 = metadata::get_meta_required(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let grid_height: u32 = metadata::get_meta_required(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;

    Ok(Some(WorldMeta {
        name,
        grid_width,
        grid_height,
        tile_size: 128,
    }))
}
