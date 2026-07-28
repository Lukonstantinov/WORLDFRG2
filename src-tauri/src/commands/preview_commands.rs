//! Planetary-settings preview commands (read-only).
//!
//! Both commands take the planetary knobs as ARGUMENTS rather than reading the
//! world's stored state, so the UI can preview values while the user is still
//! dragging a slider — nothing is committed until they let go and
//! `set_planet_config` / `set_latitude_config` persist it.
//!
//! Neither command writes a tile, a metadata row, or an undo entry.

use crate::db::WorldDb;
use crate::sim::preview::{self, CoarsePreview, PreviewParams, ZonalProfile};
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use tauri::State;

/// Assemble `PreviewParams` from the loose arguments the bridge sends.
#[allow(clippy::too_many_arguments)]
fn params(
    obliquity: f32, rotation_rate: f32, solar_lum: f32, greenhouse: f32,
    eccentricity: f32, dryness: f32,
    equator_offset: f32, lat_scale: f32, lat_ratio: f32,
) -> PreviewParams {
    PreviewParams {
        // Same clamps `set_planet_config` applies, so the preview can never show
        // a world the user would not actually be allowed to generate.
        obliquity: obliquity.clamp(0.0, 80.0),
        rotation_rate: if rotation_rate < 0.0 {
            rotation_rate.clamp(-4.0, -0.25)
        } else {
            rotation_rate.clamp(0.25, 4.0)
        },
        solar_lum: solar_lum.clamp(0.5, 1.6),
        greenhouse: greenhouse.clamp(0.0, 3.0),
        eccentricity: eccentricity.clamp(0.0, 0.4),
        dryness: dryness.clamp(0.3, 3.0),
        equator_offset: equator_offset.clamp(0.0, 1.0),
        lat_scale: lat_scale.clamp(0.25, 4.0),
        lat_ratio: lat_ratio.clamp(0.5, 5.0),
    }
}

/// TIER 1 — the 1-D zonal profile. Two EBM solves plus closed-form circulation;
/// microseconds, so the UI may call it on every slider movement.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn preview_zonal_profile(
    obliquity: f32, rotation_rate: f32, solar_lum: f32, greenhouse: f32,
    eccentricity: f32, dryness: f32,
    equator_offset: f32, lat_scale: f32, lat_ratio: f32,
) -> Result<ZonalProfile, String> {
    Ok(preview::zonal_profile(params(
        obliquity, rotation_rate, solar_lum, greenhouse,
        eccentricity, dryness, equator_offset, lat_scale, lat_ratio,
    )))
}

/// TIER 2 — the real Ocean & Atmosphere → Köppen chain on a downsampled copy of
/// the world's own landmass, returning a thumbnail plus the climate-class mix.
///
/// Costs a few hundred milliseconds (it loads terrain + elevation at full
/// resolution once, then works on a ~600-cell-wide grid), so it belongs behind a
/// button rather than on a slider.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn preview_coarse_climate(
    obliquity: f32, rotation_rate: f32, solar_lum: f32, greenhouse: f32,
    eccentricity: f32, dryness: f32,
    equator_offset: f32, lat_scale: f32, lat_ratio: f32,
    db: State<'_, WorldDb>,
) -> Result<CoarsePreview, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Only the landmass is needed — everything else the preview derives itself.
    let src = WorldBuffer::load_with(&conn, ColumnSet::TERRAIN | ColumnSet::ELEVATION)?;
    if src.terrain.iter().all(|&t| t != 1) {
        return Err("No land yet — create a landmass first (step 1), then preview.".into());
    }
    preview::coarse_climate_preview(
        &src,
        params(
            obliquity, rotation_rate, solar_lum, greenhouse,
            eccentricity, dryness, equator_offset, lat_scale, lat_ratio,
        ),
    )
}
