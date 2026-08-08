//! Planetary-settings PREVIEW — "what will this world do?" before generating it.
//!
//! Two tiers, because the two questions have wildly different costs:
//!
//! 1. **Zonal profile** (`zonal_profile`) — everything the planetary knobs do
//!    that is purely a function of LATITUDE: the energy-balance temperature
//!    curve, the seasonal envelope, and the circulation belt edges. This is a
//!    1-D problem (two EBM solves plus closed-form circulation), so it costs
//!    microseconds and can be recomputed on every slider drag. It sees no land,
//!    so it cannot tell you where a desert lands — only how wide the desert BELT
//!    will be and how warm the world is at each latitude.
//!
//! 2. **Coarse map** (`coarse_climate_preview`) — the real phase-3 → Köppen
//!    chain run on the user's actual landmass at reduced resolution. This is the
//!    only way to answer "where do the deserts actually go", because that
//!    depends on continentality, ocean currents and rain shadows. At 1/6-ish
//!    scale it is ~1/36 the cell count of the full run, so a few hundred
//!    milliseconds instead of ~16 s.
//!
//! Both are READ-ONLY: they build their own throwaway `WorldBuffer` and never
//! touch a tile, the database, or the world's stored planetary state.

use crate::db::schema;
use crate::sim::koppen;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{elevation, jets, ocean, precipitation, temperature};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rusqlite::Connection;
use serde::Serialize;

use super::circulation::Circulation;
use super::ebm::EbmAnomaly;
use super::insolation;

/// The planetary state a preview is run against. Mirrors the world's stored
/// knobs so the caller can preview values the user has not committed yet.
#[derive(Clone, Copy, Debug)]
pub struct PreviewParams {
    pub obliquity: f32,
    pub rotation_rate: f32,
    pub solar_lum: f32,
    pub greenhouse: f32,
    pub eccentricity: f32,
    pub dryness: f32,
    pub equator_offset: f32,
    pub lat_scale: f32,
    pub lat_ratio: f32,
}

impl PreviewParams {
    pub fn earth() -> Self {
        Self {
            obliquity: 23.44,
            rotation_rate: 1.0,
            solar_lum: 1.0,
            greenhouse: 1.0,
            eccentricity: 0.0167,
            dryness: 1.0,
            equator_offset: 0.5,
            lat_scale: 1.0,
            lat_ratio: 1.0,
        }
    }
}

// ─────────────────────────── Tier 1: zonal profile ───────────────────────────

/// One latitude's row in the zonal preview.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct ZonalSample {
    /// Latitude in degrees (+N).
    pub lat: f32,
    /// Annual-mean temperature for THIS world (°C).
    pub temp: f32,
    /// Annual-mean temperature Earth would have at the same latitude (°C) — the
    /// ghost trace, so the user sees the anomaly rather than an absolute curve.
    pub temp_earth: f32,
    /// Warmest-month mean on a MARITIME cell (°C) — the mild extreme.
    pub summer_maritime: f32,
    /// Coldest-month mean on a maritime cell (°C).
    pub winter_maritime: f32,
    /// Warmest-month mean deep in a CONTINENTAL interior (°C) — the harsh extreme.
    pub summer_continental: f32,
    /// Coldest-month mean in a continental interior (°C).
    pub winter_continental: f32,
    /// Predicted Köppen MAIN class for a notional lowland cell at this latitude
    /// ('A'..'E'), from the zonal temperature and the belt geometry. Indicative
    /// only — the real classification needs land, currents and rain shadows.
    pub zone: char,
}

/// The full zonal preview: per-latitude traces plus the derived belt geometry
/// and the warnings that geometry implies.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZonalProfile {
    pub samples: Vec<ZonalSample>,
    /// Descending branch of the Hadley cell — the subtropical high / desert belt.
    pub hadley_edge: f32,
    /// Polar front — the mid-latitude storm track.
    pub polar_front: f32,
    /// Meridional circulation cells per hemisphere (3 on Earth).
    pub cells: u32,
    /// True when the planet spins backwards (trade winds and boundary currents
    /// mirror; belt LATITUDES are unaffected).
    pub retrograde: bool,
    /// Area-weighted global mean temperature (°C).
    pub global_mean: f32,
    /// Earth's global mean under the same calculation, for comparison.
    pub global_mean_earth: f32,
    /// Lowest |latitude| whose annual mean is below the −8 °C ice-albedo
    /// threshold, i.e. where permanent ice begins. 90 = no ice at all.
    pub ice_line: f32,
    /// The ice line has advanced far enough that the albedo feedback is close to
    /// running away — the world is heading for a snowball.
    pub snowball_risk: bool,
    /// The Hadley edge and polar front have collided (the three-cell structure
    /// has collapsed into one wide overturning cell per hemisphere).
    pub belts_collapsed: bool,
    /// Top and bottom latitude actually visible on the map under the current
    /// latitude frame — bands beyond the poles are cropped.
    pub visible_top: f32,
    pub visible_bottom: f32,
}

/// Latitude samples in the zonal profile (every 2° from 90 N to 90 S).
const ZONAL_STEPS: usize = 91;

/// Ice line (in degrees of latitude) at which the albedo feedback is close to
/// running away. Earth's sits at ~72°.
///
/// Measured, not guessed — `sweep_cooling_knobs` walks the cooling knobs and the
/// collapse is ABRUPT: at greenhouse 0.55 the world is still alive (−3.7 °C,
/// 29 % polar, ice at 42°), and one step further at 0.50 it is a total snowball
/// (−42 °C, 100 % polar). The same cliff appears between solar 0.96 and 0.95.
/// 52° puts the warning one to two steps ahead of that edge, which is the point:
/// the user should see it coming, not discover it after generating.
const SNOWBALL_ICE_LINE: f32 = 52.0;

/// Mirror of `temperature.rs`'s slab-response seasonality so the preview's
/// envelope matches what the real pipeline will produce.
fn seasonal_span(dq: f32, cont: f32) -> f32 {
    const K_SEASONAL: f32 = 0.20;
    const TAU_MARITIME: f32 = 0.57;
    const TAU_CONTINENTAL: f32 = 0.03;
    let omega = 2.0 * std::f32::consts::PI;
    let tau = TAU_MARITIME + (TAU_CONTINENTAL - TAU_MARITIME) * cont;
    let wt = omega * tau;
    let gain = 1.0 / (1.0 + wt * wt).sqrt();
    K_SEASONAL * dq * gain
}

/// Indicative Köppen main class from the zonal temperature and belt geometry.
///
/// This is deliberately crude — it exists to colour the preview strip, not to
/// classify anything. The real `classify_koppen` needs precipitation, which
/// needs land, currents and orography.
fn zonal_class(lat: f32, t_warm: f32, t_cold: f32, circ: &Circulation, dryness: f32) -> char {
    // Polar first: nothing else matters if the warmest month can't clear 10 °C.
    if t_warm < 10.0 {
        return 'E';
    }
    // The subtropical dry belt sits just equatorward of the Hadley edge. Scale
    // its width with `dryness`, which is the global precipitation multiplier.
    let abs_lat = lat.abs();
    let belt_centre = circ.hadley_edge * 0.78;
    let belt_half = circ.hadley_edge * (0.30 + 0.22 * (dryness - 1.0).clamp(-0.7, 2.0));
    if (abs_lat - belt_centre).abs() < belt_half {
        return 'B';
    }
    // Tropical: no cold season, well inside the Hadley cell.
    if t_cold >= 18.0 && abs_lat < circ.hadley_edge {
        return 'A';
    }
    // Continental vs temperate on the classic Köppen −3 °C coldest-month line.
    if t_cold < -3.0 {
        'D'
    } else {
        'C'
    }
}

/// Compute the 1-D zonal preview. Microseconds — safe to call on every drag.
pub fn zonal_profile(p: PreviewParams) -> ZonalProfile {
    let circ = Circulation::from_params(p.rotation_rate, p.solar_lum, p.greenhouse);
    let ebm = EbmAnomaly::for_world(p.obliquity, p.solar_lum, p.greenhouse, p.rotation_rate);

    let mut samples = Vec::with_capacity(ZONAL_STEPS);
    let (mut sum_w, mut sum_e, mut weight) = (0.0f64, 0.0f64, 0.0f64);
    let mut ice_line = 90.0f32;

    for i in 0..ZONAL_STEPS {
        let lat = 90.0 - (i as f32) * (180.0 / (ZONAL_STEPS - 1) as f32);
        let abs_lat = lat.abs();
        let base = temperature::earth_base_curve(abs_lat);
        let temp = base + ebm.sample(lat);

        // Seasonal envelope, using the same asymmetric split Köppen reads
        // (`seasonal_temps`: winter dips 0.55 of the span, summer rises 0.45).
        let dq = insolation::insolation_stats(lat, p.obliquity).1;
        let span_mar = seasonal_span(dq, 0.0);
        let span_con = seasonal_span(dq, 1.0);

        let s = ZonalSample {
            lat,
            temp,
            temp_earth: base,
            summer_maritime: temp + span_mar * 0.45,
            winter_maritime: temp - span_mar * 0.55,
            summer_continental: temp + span_con * 0.45,
            winter_continental: temp - span_con * 0.55,
            zone: zonal_class(
                lat,
                temp + span_con * 0.45,
                temp - span_con * 0.55,
                &circ,
                p.dryness,
            ),
        };

        // Area weight ∝ cos(lat).
        let wgt = lat.to_radians().cos().max(0.0) as f64;
        sum_w += temp as f64 * wgt;
        sum_e += base as f64 * wgt;
        weight += wgt;

        // Ice line: the lowest |lat| that never thaws in the annual mean.
        if temp < -8.0 && abs_lat < ice_line {
            ice_line = abs_lat;
        }
        samples.push(s);
    }

    let global_mean = if weight > 0.0 { (sum_w / weight) as f32 } else { 0.0 };
    let global_mean_earth = if weight > 0.0 { (sum_e / weight) as f32 } else { 0.0 };

    // `Circulation` clamps polar_front to at least hadley_edge + 8; when that
    // clamp is what's holding them apart, the three-cell structure is gone.
    let belts_collapsed = (circ.polar_front - (circ.hadley_edge + 8.0)).abs() < 0.05;

    // Visible band under the current latitude frame (mirrors `lat_from_y`).
    let scale = if p.lat_scale <= 1e-4 { 1.0 } else { p.lat_scale };
    let lat_at = |y: f32| (((p.equator_offset - y) * 180.0) / scale).clamp(-90.0, 90.0);

    ZonalProfile {
        samples,
        hadley_edge: circ.hadley_edge,
        polar_front: circ.polar_front,
        cells: circ.cells,
        retrograde: p.rotation_rate < 0.0,
        global_mean,
        global_mean_earth,
        ice_line,
        snowball_risk: ice_line < SNOWBALL_ICE_LINE,
        belts_collapsed,
        visible_top: lat_at(0.0),
        visible_bottom: lat_at(1.0),
    }
}

// ─────────────────────────── Tier 2: coarse map ───────────────────────────

/// One traced ocean-current streamline in COARSE-grid (thumbnail pixel) coords,
/// so the frontend can draw it straight over the preview canvas. `ctype`:
/// 0 = neutral, 1 = warm, 2 = cold — the same convention as the map overlay's
/// `Streamline`. These are the ROUGH major currents this settings preview
/// implies; the real generation stage refines them under the full rules.
#[derive(Serialize)]
pub struct PreviewStreamline {
    pub points: Vec<[f32; 2]>,
    pub ctype: u8,
}

/// The coarse climate preview: a thumbnail plus the class mix it implies.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoarsePreview {
    pub width: u32,
    pub height: u32,
    /// Base64-encoded RGBA pixels of the Köppen thumbnail (`width × height × 4`).
    /// Base64 rather than a raw array because a 600×300 preview is 720 KB of
    /// bytes, which serialises to roughly 3 MB as a JSON number array — the
    /// encode/parse would cost more than the physics does. Same convention as
    /// `get_tiles`.
    pub rgba: String,
    /// Share of LAND in each Köppen main class, as percentages summing to ~100.
    pub tropical_pct: f32,
    pub arid_pct: f32,
    pub temperate_pct: f32,
    pub continental_pct: f32,
    pub polar_pct: f32,
    /// Land cells classified, for a sanity readout.
    pub land_cells: u32,
    /// Mean land temperature (°C) and total mean annual precipitation (mm).
    pub mean_temp: f32,
    pub mean_precip: f32,
    /// The major ocean-current streamlines the previewed field implies, in
    /// thumbnail pixel coords. Drawn over the Köppen thumbnail so the user can
    /// see the rough currents (and, by the class mix, how they shape the
    /// climate) BEFORE the real generation stage runs. Traced from the same
    /// `trace_current_streamlines` the map overlay uses, so they stop at the
    /// shelf break identically.
    pub streamlines: Vec<PreviewStreamline>,
}

/// Longest edge of the coarse preview grid. 1/6-ish of a default 3600×1800 world;
/// small enough to be interactive, large enough that continents keep their shape
/// and the km-based moisture e-folding still spans several cells.
const COARSE_MAX: u32 = 600;
const COARSE_MIN: u32 = 180;

/// Run the real phase-3 → Köppen chain on a downsampled copy of `src`'s landmass
/// under the given planetary parameters.
///
/// `src` must carry TERRAIN + ELEVATION. Nothing is written back — the caller's
/// world is untouched.
pub fn coarse_climate_preview(src: &WorldBuffer, p: PreviewParams) -> Result<CoarsePreview, String> {
    // ── Pick a coarse grid that preserves the world's aspect ratio ──
    let (sw, sh) = (src.width, src.height);
    if sw == 0 || sh == 0 {
        return Err("world has no grid".into());
    }
    let scale = (COARSE_MAX as f32 / sw.max(sh) as f32).min(1.0);
    let cw = ((sw as f32 * scale).round() as u32).max(COARSE_MIN.min(sw)).max(2);
    let ch = ((sh as f32 * scale).round() as u32).max(2);

    // ── A throwaway in-memory world at the coarse size ──
    let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    schema::create_tables(&conn).map_err(|e| e.to_string())?;
    for (k, v) in [("grid_width", cw.to_string()), ("grid_height", ch.to_string())] {
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        ).map_err(|e| e.to_string())?;
    }
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).map_err(|e| e.to_string())?;

    // Apply the previewed planetary state (this is the whole point).
    buf.obliquity = p.obliquity;
    buf.rotation_rate = p.rotation_rate;
    buf.solar_lum = p.solar_lum;
    buf.greenhouse = p.greenhouse;
    buf.eccentricity = p.eccentricity;
    buf.dryness = p.dryness;
    buf.equator_offset = p.equator_offset;
    buf.lat_scale = p.lat_scale;
    buf.lat_ratio = p.lat_ratio;

    // ── Downsample the landmass ──
    // Box-average elevation, and take a MAJORITY vote on land/sea rather than a
    // point sample: point-sampling a coarse grid drops islands and shreds narrow
    // isthmuses, which would change the ocean circulation the preview exists to
    // show. Cheap because the source is already in memory.
    let fx = sw as f32 / cw as f32;
    let fy = sh as f32 / ch as f32;
    for cy in 0..ch {
        let y0 = (cy as f32 * fy) as u32;
        let y1 = (((cy + 1) as f32 * fy) as u32).min(sh).max(y0 + 1);
        for cx in 0..cw {
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
            let ci = (cy * cw + cx) as usize;
            let is_land = total > 0 && land * 2 >= total;
            buf.terrain[ci] = if is_land { 1 } else { 0 };
            buf.elevation[ci] = if is_land && land > 0 { esum / land as f32 } else { 0.0 };
        }
    }

    // ── The exact Ocean & Atmosphere → Climate sequence (see sim_commands.rs and
    //    earth_validation.rs — all three must stay in step) ──
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, 42, 12.0, 0.4, 0.3, 8.0);
    ocean::compute_wind_belts(&mut buf);
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    ocean::compute_sst(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    let sea_freeze = ocean::compute_shelf_freeze(&buf);
    ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
    temperature::compute_seasonal_amplitude(&mut buf);
    temperature::apply_ice_albedo_feedback(&mut buf);
    jets::compute_low_level_jets(&mut buf);
    precipitation::compute_precipitation(&mut buf);
    koppen::classify_koppen(&mut buf);

    // ── Class mix over LAND, area-weighted by cos(lat) ──
    let mut class_w = [0.0f64; 5]; // A B C D E
    let mut land_w = 0.0f64;
    let (mut tsum, mut psum, mut land_cells) = (0.0f64, 0.0f64, 0u32);
    let mut rgba = vec![0u8; (cw * ch * 4) as usize];

    for y in 0..ch {
        let wgt = buf.latitude(y).to_radians().cos().max(0.0) as f64;
        for x in 0..cw {
            let i = (y * cw + x) as usize;
            let o = i * 4;
            if buf.terrain[i] != 1 {
                // Sea: a flat slate so the land classes carry the eye.
                rgba[o] = 16;
                rgba[o + 1] = 30;
                rgba[o + 2] = 54;
                rgba[o + 3] = 255;
                continue;
            }
            let k = buf.koppen[i];
            let (r, g, b) = koppen_preview_color(k);
            rgba[o] = r;
            rgba[o + 1] = g;
            rgba[o + 2] = b;
            rgba[o + 3] = 255;

            land_cells += 1;
            tsum += buf.temperature[i] as f64;
            psum += buf.precipitation[i] as f64;
            if let Some(ci) = main_class_index(k) {
                class_w[ci] += wgt;
                land_w += wgt;
            }
        }
    }

    // The rough major currents this world implies — traced from the field the
    // chain just computed (and threw away until now). Coords are already in the
    // coarse grid, i.e. thumbnail pixels, so the frontend draws them directly.
    let streamlines = ocean::trace_current_streamlines(
        &buf.current_vx, &buf.current_vy, &buf.current_type,
        &buf.terrain, &buf.is_shelf, cw, ch,
    )
    .into_iter()
    .map(|(points, ctype)| PreviewStreamline { points, ctype })
    .collect();

    let pct = |v: f64| if land_w > 0.0 { (v / land_w * 100.0) as f32 } else { 0.0 };
    let n = land_cells.max(1) as f64;

    Ok(CoarsePreview {
        width: cw,
        height: ch,
        rgba: BASE64.encode(&rgba),
        tropical_pct: pct(class_w[0]),
        arid_pct: pct(class_w[1]),
        temperate_pct: pct(class_w[2]),
        continental_pct: pct(class_w[3]),
        polar_pct: pct(class_w[4]),
        land_cells,
        mean_temp: (tsum / n) as f32,
        mean_precip: (psum / n) as f32,
        streamlines,
    })
}

/// Köppen zone code → main-class index (0=A .. 4=E), or None for unclassified.
fn main_class_index(code: u8) -> Option<usize> {
    use crate::sim::koppen::*;
    Some(match code {
        AF | AM | AW | AS => 0,
        BWH | BWK | BSH | BSK => 1,
        CSA | CSB | CSC | CFA | CFB | CFC | CWA | CWB | CWC => 2,
        DFA | DFB | DFC | DFD | DSA | DSB | DSC | DSD | DWA | DWB | DWC | DWD => 3,
        ET | EF => 4,
        H => 4, // highland reads as cold for the mix
        _ => return None,
    })
}

/// Main-class colours for the thumbnail. Deliberately the five broad families
/// rather than all 31 zones — at thumbnail scale the detail is noise, and the
/// question the preview answers is "how much of my world is desert".
fn koppen_preview_color(code: u8) -> (u8, u8, u8) {
    match main_class_index(code) {
        Some(0) => (24, 122, 60),   // A tropical — green
        Some(1) => (214, 168, 74),  // B arid — sand
        Some(2) => (126, 190, 92),  // C temperate — light green
        Some(3) => (92, 148, 184),  // D continental — blue
        Some(4) => (226, 233, 240), // E polar — white
        _ => (90, 96, 104),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At Earth parameters the preview must reproduce Earth: zero anomaly
    /// everywhere, belts at exactly 30/60, three cells. This is the same no-op
    /// invariant the rest of the planetary system is built on (CLAUDE.md §3.5),
    /// and a preview that disagreed with it would mislead on every slider.
    #[test]
    fn earth_params_preview_is_earth() {
        let z = zonal_profile(PreviewParams::earth());
        assert!((z.hadley_edge - 30.0).abs() < 0.01, "hadley {}", z.hadley_edge);
        assert!((z.polar_front - 60.0).abs() < 0.01, "front {}", z.polar_front);
        assert_eq!(z.cells, 3);
        assert!(!z.retrograde);
        for s in &z.samples {
            assert!((s.temp - s.temp_earth).abs() < 0.01,
                "anomaly must be zero at Earth params, got {} at lat {}",
                s.temp - s.temp_earth, s.lat);
        }
        assert!((z.global_mean - z.global_mean_earth).abs() < 0.01);
        assert!(!z.snowball_risk, "Earth is not a snowball");
        assert!(!z.belts_collapsed);
    }

    /// Rotation moves the belts the way Held–Hou says: slower spin → one wide
    /// Hadley cell reaching far poleward; faster spin → many tight bands.
    #[test]
    fn rotation_moves_the_belts() {
        let slow = zonal_profile(PreviewParams { rotation_rate: 0.4, ..PreviewParams::earth() });
        let fast = zonal_profile(PreviewParams { rotation_rate: 3.0, ..PreviewParams::earth() });
        assert!(slow.hadley_edge > 50.0, "slow spinner: {}", slow.hadley_edge);
        assert!(fast.hadley_edge < 20.0, "fast spinner: {}", fast.hadley_edge);
        assert!(fast.cells > slow.cells, "faster rotation packs in more cells");
    }

    /// The snowball guardrail must actually fire before the world is dead, and
    /// must not fire on a merely chilly world.
    #[test]
    fn snowball_warning_fires_only_when_ice_advances() {
        let chilly = zonal_profile(PreviewParams { greenhouse: 0.85, ..PreviewParams::earth() });
        assert!(!chilly.snowball_risk, "a slightly cool world is not a snowball");
        let frozen = zonal_profile(PreviewParams { greenhouse: 0.1, solar_lum: 0.6, ..PreviewParams::earth() });
        assert!(frozen.snowball_risk, "ice line {} should have tripped the warning", frozen.ice_line);
        assert!(frozen.ice_line < chilly.ice_line);
    }

    /// Greenhouse and luminosity both warm the world, and the ice line retreats.
    #[test]
    fn greenhouse_warms_and_retreats_the_ice() {
        let earth = zonal_profile(PreviewParams::earth());
        let hot = zonal_profile(PreviewParams { greenhouse: 2.0, ..PreviewParams::earth() });
        assert!(hot.global_mean > earth.global_mean + 2.0,
            "hothouse {} vs earth {}", hot.global_mean, earth.global_mean);
        assert!(hot.ice_line >= earth.ice_line, "ice must not advance when warming");
    }

    /// Retrograde rotation flips the flag but must NOT move a belt — latitude
    /// depends on rotation magnitude only (CLAUDE.md §3.5).
    #[test]
    fn retrograde_flips_direction_without_moving_belts() {
        let fwd = zonal_profile(PreviewParams { rotation_rate: 1.4, ..PreviewParams::earth() });
        let rev = zonal_profile(PreviewParams { rotation_rate: -1.4, ..PreviewParams::earth() });
        assert!(rev.retrograde && !fwd.retrograde);
        assert!((fwd.hadley_edge - rev.hadley_edge).abs() < 1e-4);
        assert!((fwd.polar_front - rev.polar_front).abs() < 1e-4);
    }

    /// Obliquity drives seasonality: zero tilt means no seasonal swing anywhere,
    /// and a high tilt makes continental interiors violent.
    #[test]
    fn obliquity_drives_the_seasonal_envelope() {
        let none = zonal_profile(PreviewParams { obliquity: 0.0, ..PreviewParams::earth() });
        let steep = zonal_profile(PreviewParams { obliquity: 60.0, ..PreviewParams::earth() });
        let mid = |z: &ZonalProfile| {
            let s = z.samples.iter().min_by(|a, b| {
                (a.lat - 50.0).abs().partial_cmp(&(b.lat - 50.0).abs()).unwrap()
            }).unwrap();
            s.summer_continental - s.winter_continental
        };
        assert!(mid(&none) < 1.0, "no tilt → no seasons, got {}", mid(&none));
        assert!(mid(&steep) > mid(&none) + 10.0, "high tilt → violent seasons");
    }

    /// A synthetic world: a broad continent spanning the tropics to the
    /// mid-latitudes, inside a sea frame.
    fn test_world(w: u32, h: u32) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                // A continent covering the middle half of the map in both axes.
                let land = x > w / 5 && x < w * 4 / 5 && y > h / 6 && y < h * 5 / 6;
                buf.terrain[i] = if land { 1 } else { 0 };
                buf.elevation[i] = if land { 0.05 } else { 0.0 };
            }
        }
        buf
    }

    /// The coarse preview must actually run the whole chain at reduced
    /// resolution and come back with a classified, plausible world — this is the
    /// piece with real risk, since phase 3 normally runs on a grid 36× larger.
    #[test]
    fn coarse_preview_classifies_a_world() {
        let src = test_world(240, 120);
        let out = coarse_climate_preview(&src, PreviewParams::earth()).unwrap();
        assert!(out.width >= 2 && out.height >= 2);
        assert_eq!(out.rgba.is_empty(), false);
        assert!(out.land_cells > 100, "expected a populated continent, got {}", out.land_cells);
        let total = out.tropical_pct + out.arid_pct + out.temperate_pct
            + out.continental_pct + out.polar_pct;
        assert!((total - 100.0).abs() < 1.0, "class mix must sum to 100, got {total}");
        assert!(out.mean_precip > 0.0, "a world with rain");
    }

    /// The whole point of the preview: turning a knob has to change the answer,
    /// in the direction the physics says. A hothouse must come out warmer and
    /// less polar than an icehouse on the SAME landmass.
    #[test]
    fn coarse_preview_responds_to_the_knobs() {
        let src = test_world(240, 120);
        let hot = coarse_climate_preview(&src, PreviewParams {
            greenhouse: 2.2, ..PreviewParams::earth()
        }).unwrap();
        let cold = coarse_climate_preview(&src, PreviewParams {
            greenhouse: 0.5, solar_lum: 0.9, ..PreviewParams::earth()
        }).unwrap();
        assert!(hot.mean_temp > cold.mean_temp + 5.0,
            "hothouse {} vs icehouse {}", hot.mean_temp, cold.mean_temp);
        assert!(hot.polar_pct < cold.polar_pct,
            "hothouse must have less polar land ({}% vs {}%)", hot.polar_pct, cold.polar_pct);
    }

    /// A dry world must actually read as drier: more arid land and less rain.
    #[test]
    fn coarse_preview_shows_a_dry_world_as_dry() {
        let src = test_world(240, 120);
        let wet = coarse_climate_preview(&src, PreviewParams {
            dryness: 0.6, ..PreviewParams::earth()
        }).unwrap();
        let dry = coarse_climate_preview(&src, PreviewParams {
            dryness: 2.4, ..PreviewParams::earth()
        }).unwrap();
        assert!(dry.mean_precip < wet.mean_precip,
            "dry {} vs wet {}", dry.mean_precip, wet.mean_precip);
        assert!(dry.arid_pct > wet.arid_pct,
            "dry world must show more arid land ({}% vs {}%)", dry.arid_pct, wet.arid_pct);
    }

    /// The UI's archetype table makes PROMISES ("expect arid to pass 40% of
    /// land", "ice caps gone or vestigial"). This checks the headline claim of
    /// each one actually holds at its default dial position, so the blurbs stay
    /// honest as the physics is tuned.
    ///
    /// The parameter values mirror `src/ui/workflow/planetArchetypes.ts` at each
    /// archetype's `defaultIntensity`. If you retune an archetype there, retune
    /// it here — a drifted copy is the whole reason this test exists.
    #[test]
    fn archetypes_deliver_what_their_blurbs_promise() {
        let src = test_world(240, 120);
        let run = |p: PreviewParams| coarse_climate_preview(&src, p).unwrap();
        let earth = run(PreviewParams::earth());

        // Desert World @ 0.45: dryness 1.35→2.4, rotation 0.85→0.6.
        let desert = run(PreviewParams {
            dryness: 1.35 + (2.4 - 1.35) * 0.45,
            rotation_rate: 0.85 + (0.6 - 0.85) * 0.45,
            ..PreviewParams::earth()
        });
        assert!(desert.arid_pct > earth.arid_pct + 5.0,
            "Desert World must be markedly more arid: {:.1}% vs Earth {:.1}%",
            desert.arid_pct, earth.arid_pct);

        // Hothouse Jungle @ 0.5: greenhouse 1.45→2.3, dryness 0.85→0.6, tilt 20→12.
        let hot = run(PreviewParams {
            greenhouse: 1.875, dryness: 0.725, obliquity: 16.0, ..PreviewParams::earth()
        });
        assert!(hot.polar_pct < earth.polar_pct,
            "Hothouse must shrink the ice: {:.1}% vs Earth {:.1}%", hot.polar_pct, earth.polar_pct);
        assert!(hot.mean_temp > earth.mean_temp + 3.0, "Hothouse must be much warmer");

        // Ice House @ 0.4: greenhouse 0.82→0.58.
        let ice = run(PreviewParams {
            greenhouse: 0.82 + (0.58 - 0.82) * 0.4, ..PreviewParams::earth()
        });
        assert!(ice.polar_pct > earth.polar_pct,
            "Ice House must grow the ice: {:.1}% vs Earth {:.1}%", ice.polar_pct, earth.polar_pct);
        assert!(ice.mean_temp < earth.mean_temp - 3.0, "Ice House must be much colder");
        // …and it must still be a WORLD, not a snowball. The blurb promises a
        // pinched temperate ribbon; a dead planet would make that a lie.
        assert!(ice.polar_pct < 60.0,
            "Ice House default must stop short of the runaway, got {:.1}% polar", ice.polar_pct);
        assert!(ice.temperate_pct + ice.arid_pct > 20.0,
            "Ice House must keep a habitable band");
        // Even the STRONG end must stay off the cliff — the dial should never be
        // able to kill the world on its own.
        let ice_max = run(PreviewParams { greenhouse: 0.58, ..PreviewParams::earth() });
        assert!(ice_max.polar_pct < 70.0,
            "Ice House at full dial must not be a snowball, got {:.1}% polar", ice_max.polar_pct);

        // Superhabitable @ 0.5 must be warm AND wetter than Earth.
        let sup = run(PreviewParams {
            greenhouse: 1.3, dryness: 0.8, ..PreviewParams::earth()
        });
        assert!(sup.mean_precip > earth.mean_precip, "Superhabitable must be wetter");
        assert!(sup.mean_temp > earth.mean_temp, "Superhabitable must be warmer");

        // NOTE the absolute shares below are artefacts of this test's synthetic
        // flat rectangular continent — no mountains means no orographic rain, so
        // its interior is far more arid than a real world's. Only the DELTAS
        // between rows are meaningful here; the real fidelity measure is the
        // Earth scorecard in `earth_validation.rs`.
        println!("\n  archetype        temp    precip   A     B     C     D     E");
        for (name, o) in [
            ("earth         ", &earth), ("desert        ", &desert), ("hothouse      ", &hot),
            ("icehouse      ", &ice), ("superhabitable", &sup),
        ] {
            println!("  {name} {:6.1} {:7.0}  {:4.1}  {:4.1}  {:4.1}  {:4.1}  {:4.1}",
                o.mean_temp, o.mean_precip, o.tropical_pct, o.arid_pct,
                o.temperate_pct, o.continental_pct, o.polar_pct);
        }
    }

    /// Diagnostic sweep for TUNING the archetype endpoints — not a guard.
    /// `cargo test --lib preview::tests::sweep -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn sweep_cooling_knobs() {
        let src = test_world(240, 120);
        println!("\n  greenhouse  sun    temp     A     B     C     D     E");
        for (g, l) in [
            (1.0, 1.0), (0.9, 1.0), (0.8, 1.0), (0.75, 1.0), (0.7, 1.0),
            (0.65, 1.0), (0.6, 1.0), (0.55, 1.0), (0.5, 1.0),
            (0.9, 0.98), (0.9, 0.97), (0.9, 0.96), (0.9, 0.95),
        ] {
            let o = coarse_climate_preview(&src, PreviewParams {
                greenhouse: g, solar_lum: l, ..PreviewParams::earth()
            }).unwrap();
            let z = zonal_profile(PreviewParams { greenhouse: g, solar_lum: l, ..PreviewParams::earth() });
            println!("  {g:9.2} {l:5.2} {:7.1}  {:4.1}  {:4.1}  {:4.1}  {:4.1}  {:4.1}   ice@{:4.0}{}",
                o.mean_temp, o.tropical_pct, o.arid_pct, o.temperate_pct,
                o.continental_pct, o.polar_pct, z.ice_line,
                if z.snowball_risk { "  WARN" } else { "" });
        }
    }

    /// The latitude frame is reported honestly: cropping the map must show up as
    /// a narrower visible band, not silently.
    #[test]
    fn latitude_frame_reports_the_visible_band() {
        let full = zonal_profile(PreviewParams::earth());
        assert!((full.visible_top - 90.0).abs() < 0.01);
        assert!((full.visible_bottom + 90.0).abs() < 0.01);
        let zoomed = zonal_profile(PreviewParams { lat_scale: 2.0, ..PreviewParams::earth() });
        assert!(zoomed.visible_top < 50.0, "expansion crops the poles: {}", zoomed.visible_top);
    }
}
