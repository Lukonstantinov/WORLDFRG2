use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::{WorldDb, metadata, tile_store};
use crate::sim::world_buffer::{DEFAULT_EQUATOR_OFFSET, DEFAULT_LAT_SCALE, DEFAULT_LAT_RATIO};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    pub name: String,
    pub grid_width: u32,
    pub grid_height: u32,
    pub tile_size: u32,
    /// Equator position as a fraction of height from the top (0.5 = centred).
    pub equator_offset: f32,
    /// Latitude expansion factor (1.0 = default; >1 stretches the bands so the
    /// poles fall off-canvas and are cropped).
    pub lat_scale: f32,
    /// Latitude-line spacing ratio (gap 30→60 ÷ gap 0→30); the simulation uses
    /// the SAME ratio as the drawn lines so every latitude-dependent layer lands
    /// on the lines. 1.0 = even.
    pub lat_ratio: f32,
    /// True once `finalize_world` froze the geography (campaign steps unlocked).
    #[serde(default)]
    pub frozen: bool,
}

/// Read the latitude framing from metadata, falling back to the defaults for
/// worlds saved before the feature existed.
pub fn read_lat_config(conn: &rusqlite::Connection) -> (f32, f32, f32) {
    let equator_offset = metadata::get_meta(conn, "equator_offset")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_EQUATOR_OFFSET);
    let lat_scale = metadata::get_meta(conn, "lat_scale")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LAT_SCALE);
    let lat_ratio = metadata::get_meta(conn, "lat_ratio")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LAT_RATIO);
    (equator_offset, lat_scale, lat_ratio)
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
         DELETE FROM sim_state; DELETE FROM undo_journal; DELETE FROM campaign;"
    ).map_err(|e| e.to_string())?;

    // Set metadata
    metadata::set_meta(&conn, "name", &name).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "grid_width", &grid_width.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "grid_height", &grid_height.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "tile_size", "128").map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "format_version", "1").map_err(|e| e.to_string())?;
    // Default latitude framing: equator centred, no expansion.
    metadata::set_meta(&conn, "equator_offset", &DEFAULT_EQUATOR_OFFSET.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lat_scale", &DEFAULT_LAT_SCALE.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lat_ratio", &DEFAULT_LAT_RATIO.to_string())
        .map_err(|e| e.to_string())?;

    // Initialize tiles
    tile_store::init_tiles(&conn, grid_width, grid_height).map_err(|e| e.to_string())?;

    Ok(WorldMeta {
        name,
        grid_width,
        grid_height,
        tile_size: 128,
        equator_offset: DEFAULT_EQUATOR_OFFSET,
        lat_scale: DEFAULT_LAT_SCALE,
        lat_ratio: DEFAULT_LAT_RATIO,
        frozen: false,
    })
}

/// Update the latitude framing (equator position + expansion). Persisted to
/// metadata so the next run of any simulation phase generates against the new
/// latitudes. Values are clamped to sane ranges.
#[tauri::command]
pub fn set_latitude_config(
    equator_offset: f32,
    lat_scale: f32,
    lat_ratio: f32,
    db: State<'_, WorldDb>,
) -> Result<WorldMeta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let equator_offset = equator_offset.clamp(0.0, 1.0);
    let lat_scale = lat_scale.clamp(0.25, 4.0);
    let lat_ratio = lat_ratio.clamp(0.5, 5.0);

    metadata::set_meta(&conn, "equator_offset", &equator_offset.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lat_scale", &lat_scale.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lat_ratio", &lat_ratio.to_string())
        .map_err(|e| e.to_string())?;

    let name = metadata::get_meta(&conn, "name").map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Untitled".to_string());
    let grid_width: u32 = metadata::get_meta_required(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    let grid_height: u32 = metadata::get_meta_required(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;

    Ok(WorldMeta {
        name,
        grid_width,
        grid_height,
        tile_size: 128,
        equator_offset,
        lat_scale,
        lat_ratio,
        frozen: crate::commands::campaign_commands::is_frozen(&conn),
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

    let (equator_offset, lat_scale, lat_ratio) = read_lat_config(&conn);

    Ok(Some(WorldMeta {
        name,
        grid_width,
        grid_height,
        tile_size: 128,
        equator_offset,
        lat_scale,
        lat_ratio,
        frozen: crate::commands::campaign_commands::is_frozen(&conn),
    }))
}
