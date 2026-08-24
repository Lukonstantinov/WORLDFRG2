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
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
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

/// A read-only downsampled thumbnail of the CURRENT world's land/sea + elevation
/// (`ITCZ_AND_LAND_TOOLS_PLAN.md` Commit 1 — the variant compare). Samples the
/// `WorldBuffer` directly by majority-vote land/sea and mean elevation per block
/// (`preview::coarse_climate_preview`'s own downsampling discipline), rather
/// than going through `get_tiles`/the LOD cache, whose invalidation timing after
/// a generate would make a thumbnail silently stale.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldThumbnail {
    pub width: u32,
    pub height: u32,
    /// Base64-encoded RGBA pixels (`width × height × 4`), same convention as
    /// `get_tiles`/`CoarsePreview`.
    pub rgba: String,
}

#[tauri::command]
pub fn render_world_thumbnail(max_px: u32, db: State<'_, WorldDb>) -> Result<WorldThumbnail, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let src = WorldBuffer::load_with(&conn, ColumnSet::TERRAIN | ColumnSet::ELEVATION)?;
    let (sw, sh) = (src.width, src.height);
    if sw == 0 || sh == 0 {
        return Err("world has no grid".into());
    }
    let max_px = max_px.clamp(64, 2000);
    let scale = (max_px as f32 / sw.max(sh) as f32).min(1.0);
    let tw = ((sw as f32 * scale).round() as u32).max(2);
    let th = ((sh as f32 * scale).round() as u32).max(2);
    let fx = sw as f32 / tw as f32;
    let fy = sh as f32 / th as f32;
    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for cy in 0..th {
        let y0 = (cy as f32 * fy) as u32;
        let y1 = (((cy + 1) as f32 * fy) as u32).min(sh).max(y0 + 1);
        for cx in 0..tw {
            let x0 = (cx as f32 * fx) as u32;
            let x1 = (((cx + 1) as f32 * fx) as u32).min(sw).max(x0 + 1);
            let (mut land, mut total, mut esum) = (0u32, 0u32, 0.0f32);
            for y in y0..y1 {
                for x in x0..x1 {
                    let si = (y * sw + x) as usize;
                    total += 1;
                    if src.terrain[si] == 1 {
                        land += 1;
                        esum += src.elevation[si];
                    }
                }
            }
            let is_land = total > 0 && land * 2 >= total;
            let out_i = ((cy * tw + cx) * 4) as usize;
            if is_land {
                let mean_e = if land > 0 { (esum / land as f32).clamp(0.0, 1.0) } else { 0.0 };
                rgba[out_i] = (60.0 + mean_e * 140.0) as u8;
                rgba[out_i + 1] = (110.0 + mean_e * 100.0) as u8;
                rgba[out_i + 2] = (60.0 + mean_e * 40.0) as u8;
            } else {
                rgba[out_i] = 20;
                rgba[out_i + 1] = 60;
                rgba[out_i + 2] = 120;
            }
            rgba[out_i + 3] = 255;
        }
    }
    Ok(WorldThumbnail { width: tw, height: th, rgba: BASE64.encode(&rgba) })
}
