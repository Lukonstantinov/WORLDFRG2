use crate::sim::world_buffer::{WorldBuffer, SEASON_AMP_SCALE};
use super::{insolation, ebm};
use std::f32::consts::PI;

/// Earth-calibrated base annual-mean temperature (°C) at sea level for a latitude.
/// Tuned to real Earth anchors so the mid-latitudes are warm enough to host
/// temperate climates:
///   0° → 30,  30° → 20,  45° → 12.5,  60° → 5,  75° → -12,  90° → -29.
/// The planetary energy budget (`ebm.rs`) is layered on as an anomaly on top of
/// this, so at Earth parameters the curve is used unchanged.
#[inline]
pub(crate) fn earth_base_curve(abs_lat: f32) -> f32 {
    if abs_lat < 30.0 {
        30.0 - 0.333 * abs_lat
    } else if abs_lat < 60.0 {
        20.0 - 0.5 * (abs_lat - 30.0)
    } else {
        5.0 - 1.15 * (abs_lat - 60.0)
    }
}

/// Compute temperature for all cells.
/// Base from latitude bands + planetary energy-budget anomaly + altitude lapse +
/// current influence + coastal damping. Matches WF1 temperature.ts algorithm plus
/// the anomaly-coupled energy balance (`ebm.rs`).
pub fn compute_temperature(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;

    // Planetary energy-budget anomaly: how much this world's tilt / stellar
    // luminosity / greenhouse / rotation shift the zonal-mean temperature away
    // from Earth's. Exactly zero at Earth parameters (Earth calibration intact).
    let ebm_anom = ebm::EbmAnomaly::for_world(
        buf.obliquity, buf.solar_lum, buf.greenhouse, buf.rotation_rate,
    );

    for y in 0..h {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();

        // Earth base curve + the planetary anomaly for this latitude.
        let base_temp = earth_base_curve(abs_lat) + ebm_anom.sample(lat);

        for x in 0..w {
            let idx = buf.idx(x, y);
            let mut temp = base_temp;

            if buf.terrain[idx] == 1 {
                // Altitude lapse rate: -5Â°C per 1000m (elevation is 0-1, max 8848m)
                let altitude_m = buf.elevation[idx] * 8848.0;
                temp -= 5.0 * altitude_m / 1000.0;

                // Coastal damping: only a coast exposed to OPEN ocean on the
                // prevailing-wind upwind side is moderated toward 15Â°C. A lee /
                // east coast â€” OR a coast fronted by a wide continental shelf
                // (shelf water doesn't count as open ocean) â€” keeps its continental
                // mean so it reads cold-winter Df/Dw instead of mild oceanic Cfb.
                let ocean_dist = buf.distance_to_ocean[idx];
                if ocean_dist < 0.1 && crate::sim::koppen::upwind_is_open_ocean(buf, x, y, 6) {
                    let coastal_factor = 1.0 - ocean_dist / 0.1;
                    temp = temp + (15.0 - temp) * 0.45 * coastal_factor;
                }
            }

            buf.temperature[idx] = temp;
        }
    }

    // Current temperature influence: warm/cold currents affect nearby land.
    // Accumulate into a separate delta buffer so a coastal cell that sits
    // downwind of many ocean source cells doesn't stack a runaway anomaly,
    // then clamp the total per cell to a realistic maritime range before
    // applying it. (Earlier this added directly to temperature, letting
    // dozens of ocean sources in the same row sum into +20Â°C-plus coasts.)
    //
    // The thermal push now scales with the current's SPEED â€” a "volume" proxy.
    // A strong warm boundary current (Gulf Stream / Kuroshio) carries a large,
    // warm body of water and pushes its signal far inland; a slow drift carries
    // less and reaches a shorter distance. Cold currents are modelled with a
    // somewhat smaller thermal magnitude ("less water"); their characteristic
    // DRYNESS is handled separately in precipitation. This keeps the
    // Gulf-Stream-to-Europe conveyor while stopping weak high-latitude drift
    // from over-warming the far north (which was letting warm-climate crops like
    // wine creep too far poleward).
    let current_types = buf.current_type.clone();
    let current_vx = buf.current_vx.clone();
    let current_vy = buf.current_vy.clone();
    let mut temp_delta = vec![0.0f32; buf.total()];

    // Reference speed: a vigorous boundary current. `vol` 0.35..1.3 scales both
    // the anomaly strength and how far it carries.
    const REF_SPEED: f32 = 1.4;

    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if current_types[idx] == 0 { continue; }
            if buf.terrain[idx] != 0 { continue; } // only from ocean cells

            let speed = (current_vx[idx] * current_vx[idx]
                + current_vy[idx] * current_vy[idx]).sqrt();
            let vol = (speed / REF_SPEED).clamp(0.35, 1.3);

            let base_delta = match current_types[idx] {
                1 => 3.4,  // warm â€” a warm current carries a larger heat anomaly
                2 => -2.4, // cold â€” less thermal mass; aridity dominates its effect
                _ => continue,
            };
            let delta = base_delta * vol;

            // Spread influence into land cells upwind. A strong current reaches
            // ~78 cells; a weak drift only ~30.
            let wind_dir_x = if current_vx[idx].abs() > 0.1 {
                if current_vx[idx] > 0.0 { 1i32 } else { -1 }
            } else {
                // Default: spread from ocean toward land
                let east_land = (1..=3).any(|d| {
                    let ni = buf.idx(buf.wrap_x(x as i32 + d), y);
                    buf.terrain[ni] == 1
                });
                if east_land { 1 } else { -1 }
            };

            let reach = (24.0 + 42.0 * vol) as i32; // ~30 (weak) .. ~78 (strong)
            let efold = 18.0 + 12.0 * vol;           // strong currents decay slower
            for step in 1..=reach {
                let nx = buf.wrap_x(x as i32 + wind_dir_x * step);
                let ni = buf.idx(nx, y);
                if buf.terrain[ni] != 1 { continue; }

                let decay = (-step as f32 / efold).exp();
                temp_delta[ni] += delta * decay;
            }
        }
    }

    // Apply clamped anomaly. Warm moderation can run a little higher than cold
    // cooling: a strong warm current keeps a coast mild (NW-Europe style), while
    // the cold-current floor is tighter so upwelling can't freeze a subtropical
    // coast into a polar/subarctic zone.
    for i in 0..buf.total() {
        if buf.terrain[i] == 1 {
            buf.temperature[i] += temp_delta[i].clamp(-5.0, 6.5);
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Energy-balance seasonality
// ────────────────────────────────────────────────────────────────────────────
//
// The annual-mean `temperature` field above is kept as the Earth-calibrated
// baseline. What was fabricated before is the SEASONAL SWING — the coldest/warmest
// month split that drives Köppen's C/D/E boundaries. Here we derive that swing
// physically: the seasonal amplitude of the incoming sunlight (`insolation.rs`)
// attenuated by the surface's heat capacity (a slab responding to a sinusoidal
// forcing). Land responds fast → big swing near the solstices; a maritime coast is
// a deep, slow mixed layer → the swing is damped and lagged. Continentality (how
// far a cell is from open ocean upwind) sets the effective heat capacity, so the
// cold-winter interiors / lee coasts emerge instead of being imposed.

/// Calibration: °C of seasonal temperature span per (W/m²) of seasonal insolation
/// amplitude, at full (land) response. Tuned so a deep continental interior at ~55°
/// reaches a ~44 °C warmest−coldest span (Earth-like) — keeping downstream Köppen in
/// the same regime the old parametric range produced.
const K_SEASONAL: f32 = 0.20;
/// Thermal response time (years) of a maritime column (deep mixed layer): a long τ
/// heavily damps and lags the seasonal cycle (mild oceanic coasts).
const TAU_MARITIME: f32 = 0.57;
/// Thermal response time (years) of a continental column (shallow land slab): a
/// short τ lets the surface nearly follow the insolation (large continental swing).
const TAU_CONTINENTAL: f32 = 0.03;

/// Fraction of the year a cell spends below `thresh` °C, given its coldest/warmest
/// month under a sinusoidal seasonal cycle. Used for snow-cover / ice-albedo.
fn year_frac_below(coldest: f32, warmest: f32, thresh: f32) -> f32 {
    if warmest <= thresh { return 1.0; }
    if coldest >= thresh { return 0.0; }
    let mean = (coldest + warmest) * 0.5;
    let amp = (warmest - coldest) * 0.5;
    if amp < 1e-3 { return if mean < thresh { 1.0 } else { 0.0 }; }
    // T(t) = mean + amp·sin(2πt); fraction of the cycle with T < thresh.
    let r = ((thresh - mean) / amp).clamp(-1.0, 1.0);
    (0.5 + r.asin() / PI).clamp(0.0, 1.0)
}

/// Compute the physically-derived seasonal temperature span (warmest−coldest month,
/// °C) for every land cell and write it to `buf.seasonal_amp` (encoded ×
/// `SEASON_AMP_SCALE`). Call right after `compute_temperature`; consumed by
/// `koppen::seasonal_temps`. Requires `distance_to_ocean` + `is_shelf` + the world's
/// `obliquity` to be available (all in the ocean-atmosphere column set).
pub fn compute_seasonal_amplitude(buf: &mut WorldBuffer) {
    if buf.seasonal_amp.is_empty() {
        return;
    }
    let w = buf.width;
    let h = buf.height;
    let omega = 2.0 * PI; // annual angular frequency (yr⁻¹), τ measured in years

    // Per-row seasonal insolation amplitude ΔQ (W/m²) for this world's tilt.
    let dq: Vec<f32> = (0..h)
        .map(|y| insolation::insolation_stats(buf.latitude(y), buf.obliquity).1)
        .collect();

    for y in 0..h {
        let dqy = dq[y as usize];
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 {
                buf.seasonal_amp[idx] = 0;
                continue;
            }
            let dist = buf.distance_to_ocean[idx].clamp(0.0, 1.0);
            // Continentality 0 (maritime, ocean upwind) .. 1 (deep interior / lee).
            // A shelf sea upwind does NOT count as open ocean, so wide-shelf coasts
            // stay continental — same signal koppen::continentality uses.
            let maritime = crate::sim::koppen::upwind_is_open_ocean(buf, x, y, 6);
            let cont = if maritime {
                (dist / 0.20).clamp(0.0, 1.0)
            } else {
                (0.55 + 0.45 * dist).clamp(0.0, 1.0)
            };
            // Effective heat-capacity → thermal response time → slab attenuation of
            // the seasonal cycle. gain = 1/√(1+(ωτ)²) (standard forced-slab response).
            let tau = TAU_MARITIME + (TAU_CONTINENTAL - TAU_MARITIME) * cont;
            let wt = omega * tau;
            let gain = 1.0 / (1.0 + wt * wt).sqrt();
            let span = K_SEASONAL * dqy * gain;
            buf.seasonal_amp[idx] = (span * SEASON_AMP_SCALE).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Bounded ice/snow-albedo feedback. From each land cell's coldest/warmest month
/// (annual mean ± its seasonal span) derive the annual snow-cover fraction, store it
/// in `buf.snow_frac`, and cool the cell in proportion — a snow surface reflects far
/// more sunlight than bare ground, so a cold cell absorbs less and gets colder. A
/// single bounded pass (no runaway); it sharpens the tundra / ice-cap / cold-
/// continental margins. Call after `compute_seasonal_amplitude`.
pub fn apply_ice_albedo_feedback(buf: &mut WorldBuffer) {
    if buf.snow_frac.is_empty() {
        return;
    }
    // Max extra cooling (°C) at perennial snow cover.
    const ALB_MAX_COOL: f32 = 4.0;
    let w = buf.width;
    let h = buf.height;
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 {
                buf.snow_frac[idx] = 0;
                continue;
            }
            let span = if buf.seasonal_amp.is_empty() {
                0.0
            } else {
                buf.seasonal_amp[idx] as f32 / SEASON_AMP_SCALE
            };
            let t = buf.temperature[idx];
            let coldest = t - span * 0.55;
            let warmest = t + span * 0.45;
            let snow = year_frac_below(coldest, warmest, 0.0);
            buf.snow_frac[idx] = (snow * 255.0).clamp(0.0, 255.0) as u8;
            buf.temperature[idx] = t - ALB_MAX_COOL * snow;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
    use crate::db::schema;
    use rusqlite::Connection;

    fn buf_of(w: u32, h: u32) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            )
            .unwrap();
        }
        WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap()
    }

    /// A deep continental interior must swing far more than a maritime coast at the
    /// SAME latitude — the emergent continentality the fabricated range only faked.
    #[test]
    fn interior_swings_more_than_coast() {
        let (w, h) = (40u32, 120u32);
        let mut buf = buf_of(w, h);
        // A bounded NH continent: lat ~10..60 AND the middle half of the longitudes,
        // so open ocean lies to the WEST (x < w/4). At 45° the westerlies blow off
        // that ocean, so the west coast (x = w/4) is maritime while the interior
        // (x = w/2) is > reach cells from any ocean and stays continental.
        for y in 0..h {
            let lat = buf.latitude(y);
            for x in 0..w {
                let i = buf.idx(x, y);
                let land = lat > 10.0 && lat < 60.0 && x >= w / 4 && x < 3 * w / 4;
                buf.terrain[i] = if land { 1 } else { 0 };
                buf.elevation[i] = if land { 0.05 } else { 0.0 };
            }
        }
        crate::sim::ocean::compute_distance_to_ocean(&mut buf);
        compute_seasonal_amplitude(&mut buf);

        let row = (0..h)
            .min_by(|&a, &b| {
                (buf.latitude(a) - 45.0).abs().partial_cmp(&(buf.latitude(b) - 45.0).abs()).unwrap()
            })
            .unwrap();
        let coast = buf.seasonal_amp[buf.idx(w / 4, row)] as f32 / SEASON_AMP_SCALE;
        let interior = buf.seasonal_amp[buf.idx(w / 2, row)] as f32 / SEASON_AMP_SCALE;
        assert!(
            interior > coast * 1.6,
            "interior span {interior} should dwarf maritime-coast span {coast} at 45°"
        );
        assert!(interior > 15.0, "45° continental span looks too small: {interior} °C");
    }

    /// The ice-albedo feedback is bounded and only cools cold, snowy cells.
    #[test]
    fn ice_albedo_is_bounded_and_gated() {
        let (w, h) = (8u32, 8u32);
        let mut buf = buf_of(w, h);
        for i in 0..(w * h) as usize {
            buf.terrain[i] = 1;
            buf.seasonal_amp[i] = (10.0 * SEASON_AMP_SCALE) as u8; // ±~5 °C swing
        }
        // A warm cell (no snow) and a frozen cell (perennial snow).
        let warm = buf.idx(1, 1);
        let cold = buf.idx(5, 5);
        buf.temperature[warm] = 25.0;
        buf.temperature[cold] = -20.0;
        apply_ice_albedo_feedback(&mut buf);
        assert_eq!(buf.snow_frac[warm], 0, "warm cell carries no snow");
        assert_eq!(buf.temperature[warm], 25.0, "warm cell not cooled");
        assert!(buf.snow_frac[cold] > 250, "frozen cell is snow-covered");
        assert!(
            (buf.temperature[cold] - (-24.0)).abs() < 0.01,
            "frozen cell cooled by the full bounded amount: {}",
            buf.temperature[cold]
        );
    }
}


