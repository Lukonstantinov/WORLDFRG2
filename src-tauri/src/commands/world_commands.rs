use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::{WorldDb, metadata, tile_store};
use crate::sim::world_buffer::{DEFAULT_EQUATOR_OFFSET, DEFAULT_LAT_SCALE, DEFAULT_LAT_RATIO, DEFAULT_OBLIQUITY,
    DEFAULT_ROTATION_RATE, DEFAULT_SOLAR_LUM, DEFAULT_GREENHOUSE, DEFAULT_ECCENTRICITY, DEFAULT_DRYNESS};

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
    /// Axial tilt (obliquity, degrees) driving the seasonal insolation cycle.
    /// 23.44 = Earth-like; higher = more extreme seasons; 0 = no seasons.
    #[serde(default = "default_obliquity")]
    pub obliquity: f32,
    /// True once `finalize_world` froze the geography (campaign steps unlocked).
    #[serde(default)]
    pub frozen: bool,
}

fn default_obliquity() -> f32 { DEFAULT_OBLIQUITY }

/// Read the latitude framing + axial tilt from metadata, falling back to the
/// defaults for worlds saved before each feature existed.
pub fn read_lat_config(conn: &rusqlite::Connection) -> (f32, f32, f32, f32) {
    let equator_offset = metadata::get_meta(conn, "equator_offset")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_EQUATOR_OFFSET);
    let lat_scale = metadata::get_meta(conn, "lat_scale")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LAT_SCALE);
    let lat_ratio = metadata::get_meta(conn, "lat_ratio")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_LAT_RATIO);
    let obliquity = metadata::get_meta(conn, "obliquity_deg")
        .ok().flatten().and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_OBLIQUITY);
    (equator_offset, lat_scale, lat_ratio, obliquity)
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
    // Fresh world → no campaign; drop any resident sim from the previous one.
    db.invalidate_campaign();

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
    // Default axial tilt: Earth-like 23.44°.
    metadata::set_meta(&conn, "obliquity_deg", &DEFAULT_OBLIQUITY.to_string())
        .map_err(|e| e.to_string())?;
    // Default planetary state: Earth (so the anomaly-coupled climate physics is
    // zero and the generator reproduces the Earth calibration out of the box).
    metadata::set_meta(&conn, "rotation_rate", &DEFAULT_ROTATION_RATE.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "solar_lum", &DEFAULT_SOLAR_LUM.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "greenhouse", &DEFAULT_GREENHOUSE.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "eccentricity", &DEFAULT_ECCENTRICITY.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "dryness", &DEFAULT_DRYNESS.to_string())
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
        obliquity: DEFAULT_OBLIQUITY,
        frozen: false,
    })
}

/// Set how many CULTURES (hearths) the world starts with. Persisted to metadata so
/// the next culture-map computation (settlement/run-all phase) uses it. `0` = auto
/// (scale with land area, the default). Clamped to a sane upper bound at generation.
#[tauri::command]
pub fn set_culture_count(count: u32, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "culture_count", &count.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Read the world's chosen culture count (0 = auto). For the worldgen UI to show the
/// current setting.
#[tauri::command]
pub fn get_culture_count(db: State<'_, WorldDb>) -> Result<u32, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(metadata::get_meta(&conn, "culture_count").map_err(|e| e.to_string())?
        .and_then(|s| s.parse::<u32>().ok()).unwrap_or(0))
}

/// Update the latitude framing (equator position + expansion). Persisted to
/// metadata so the next run of any simulation phase generates against the new
/// latitudes. Values are clamped to sane ranges.
#[tauri::command]
pub fn set_latitude_config(
    equator_offset: f32,
    lat_scale: f32,
    lat_ratio: f32,
    obliquity: f32,
    db: State<'_, WorldDb>,
) -> Result<WorldMeta, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let equator_offset = equator_offset.clamp(0.0, 1.0);
    let lat_scale = lat_scale.clamp(0.25, 4.0);
    let lat_ratio = lat_ratio.clamp(0.5, 5.0);
    // Obliquity 0..80°: 0 removes seasons entirely; >45° gives extreme (tropics-
    // reach-the-pole) seasonality. The insolation model stays well-defined across
    // the whole range.
    let obliquity = obliquity.clamp(0.0, 80.0);

    metadata::set_meta(&conn, "equator_offset", &equator_offset.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lat_scale", &lat_scale.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lat_ratio", &lat_ratio.to_string())
        .map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "obliquity_deg", &obliquity.to_string())
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
        obliquity,
        frozen: crate::commands::campaign_commands::is_frozen(&conn),
    })
}

// ── Planetary state (energy budget + general circulation) ───────────────────
// Kept separate from the latitude framing so the Planet panel reads/writes it on
// its own. All four fields default to Earth, so a world that never touched them
// generates exactly as before (the anomaly-coupled physics is zero at Earth).

/// The planetary knobs that drive the emergent climate (see `world_buffer.rs`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlanetConfig {
    /// Rotation rate (× Earth). Width of the Hadley cell / wind-belt latitudes.
    /// Sign is the rotation DIRECTION: positive = prograde (Earth-like),
    /// negative = retrograde (mirrors the Coriolis-deflection direction in the
    /// wind belts, ocean gyres and boundary currents; belt LATITUDE only
    /// depends on the magnitude, so it is unaffected by the sign).
    pub rotation_rate: f32,
    /// Stellar irradiance (× Earth solar constant). Global-mean temperature.
    pub solar_lum: f32,
    /// Greenhouse factor (× Earth). Global warming + equator-pole gradient.
    pub greenhouse: f32,
    /// Orbital eccentricity (0 = circular). Hemispheric season asymmetry.
    pub eccentricity: f32,
    /// Global aridity multiplier (1.0 = Earth/no-op). >1 = drier, <1 = wetter.
    #[serde(default = "default_dryness")]
    pub dryness: f32,
}

fn default_dryness() -> f32 { DEFAULT_DRYNESS }

/// Read the planetary state from metadata, defaulting each field to Earth.
pub fn read_planet_config(conn: &rusqlite::Connection) -> PlanetConfig {
    let get = |k: &str, d: f32| metadata::get_meta(conn, k)
        .ok().flatten().and_then(|s| s.parse().ok()).unwrap_or(d);
    PlanetConfig {
        rotation_rate: get("rotation_rate", DEFAULT_ROTATION_RATE),
        solar_lum: get("solar_lum", DEFAULT_SOLAR_LUM),
        greenhouse: get("greenhouse", DEFAULT_GREENHOUSE),
        eccentricity: get("eccentricity", DEFAULT_ECCENTRICITY),
        dryness: get("dryness", DEFAULT_DRYNESS),
    }
}

/// Current planetary state (for the Planet panel to show).
#[tauri::command]
pub fn get_planet_config(db: State<'_, WorldDb>) -> Result<PlanetConfig, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(read_planet_config(&conn))
}

/// Update the planetary state. Persisted to metadata so the next run of Ocean &
/// Atmosphere → Climate generates against it. Values clamped to sane ranges.
#[tauri::command]
pub fn set_planet_config(
    rotation_rate: f32,
    solar_lum: f32,
    greenhouse: f32,
    eccentricity: f32,
    dryness: f32,
    db: State<'_, WorldDb>,
) -> Result<PlanetConfig, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Rotation magnitude 0.25×..4×: slower than ¼ Earth degenerates to a single
    // Hadley cell we don't model; faster than 4× bands finer than the grid
    // resolves. The SIGN is preserved (negative = retrograde) — belt latitude
    // only depends on magnitude, so clamping it separately doesn't touch the
    // Earth-calibration invariant at rotation_rate == 1.0.
    let rot_mag = rotation_rate.abs().clamp(0.25, 4.0);
    let rot_signed = if rotation_rate < 0.0 { -rot_mag } else { rot_mag };
    let cfg = PlanetConfig {
        rotation_rate: rot_signed,
        // 0.5×..1.6× the solar constant brackets the circumstellar habitable zone.
        solar_lum: solar_lum.clamp(0.5, 1.6),
        // 0×..3× Earth's greenhouse: 0 ≈ airless/snowball, 3 ≈ deep hothouse.
        greenhouse: greenhouse.clamp(0.0, 3.0),
        eccentricity: eccentricity.clamp(0.0, 0.4),
        // 0.3×..3×: below halves precip toward a wetter world, above triples
        // aridity toward a desert world. 1.0 = Earth (no-op).
        dryness: dryness.clamp(0.3, 3.0),
    };
    metadata::set_meta(&conn, "rotation_rate", &cfg.rotation_rate.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "solar_lum", &cfg.solar_lum.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "greenhouse", &cfg.greenhouse.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "eccentricity", &cfg.eccentricity.to_string()).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "dryness", &cfg.dryness.to_string()).map_err(|e| e.to_string())?;
    Ok(cfg)
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

    let (equator_offset, lat_scale, lat_ratio, obliquity) = read_lat_config(&conn);

    Ok(Some(WorldMeta {
        name,
        grid_width,
        grid_height,
        tile_size: 128,
        equator_offset,
        lat_scale,
        lat_ratio,
        obliquity,
        frozen: crate::commands::campaign_commands::is_frozen(&conn),
    }))
}
