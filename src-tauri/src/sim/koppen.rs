use super::world_buffer::WorldBuffer;

/// Köppen climate classification codes (1-based, 0=none)
pub const AF: u8 = 1;   // Tropical rainforest
pub const AM: u8 = 2;   // Tropical monsoon
pub const AW: u8 = 3;   // Tropical savanna
pub const BWH: u8 = 4;  // Hot desert
pub const BWK: u8 = 5;  // Cold desert
pub const BSH: u8 = 6;  // Hot steppe
pub const BSK: u8 = 7;  // Cold steppe
pub const CSA: u8 = 8;  // Mediterranean hot summer
pub const CSB: u8 = 9;  // Mediterranean warm summer
pub const CSC: u8 = 10; // Mediterranean cold summer
pub const CFA: u8 = 11; // Humid subtropical
pub const CFB: u8 = 12; // Oceanic
pub const CFC: u8 = 13; // Subpolar oceanic
pub const DFA: u8 = 14; // Hot-summer continental
pub const DFB: u8 = 15; // Warm-summer continental
pub const DFC: u8 = 16; // Subarctic
pub const DFD: u8 = 17; // Extremely cold subarctic
pub const DSA: u8 = 18; // Mediterranean-influenced hot continental
pub const DSB: u8 = 19; // Mediterranean-influenced warm continental
pub const DSC: u8 = 20; // Mediterranean-influenced subarctic
pub const ET: u8 = 21;  // Tundra
pub const EF: u8 = 22;  // Ice cap
// Dry-season variants added so the full Köppen set is representable.
pub const AS: u8 = 23;  // Tropical savanna, dry summer
pub const CWA: u8 = 24; // Monsoon-influenced humid subtropical (dry winter)
pub const CWB: u8 = 25; // Subtropical highland (dry winter)
pub const CWC: u8 = 26; // Cold subtropical highland (dry winter)
pub const DWA: u8 = 27; // Hot-summer continental, dry winter
pub const DWB: u8 = 28; // Warm-summer continental, dry winter
pub const DWC: u8 = 29; // Subarctic, dry winter
pub const DWD: u8 = 30; // Extremely cold subarctic, dry winter
pub const DSD: u8 = 31; // Extremely cold subarctic, dry summer
pub const H:   u8 = 32; // Highland / Alpine

// Mediterranean latitude bounds
const MED_LAT_MIN: f32 = 25.0;
const MED_LAT_MAX: f32 = 45.0;

// Earth-scale reference width for current influence radius
const EARTH_W: f32 = 3600.0;

/// Smooth seasonal range — continuous interpolation instead of discrete bands.
/// Prevents the "zebra pattern" caused by sharp latitude thresholds.
fn seasonal_range(abs_lat: f32, distance_to_ocean: f32) -> f32 {
    // Smooth piecewise-linear ramp matching WF1 anchor points:
    //   lat  0 → range  2
    //   lat 10 → range  5
    //   lat 25 → range 12
    //   lat 40 → range 22
    //   lat 55 → range 30
    //   lat 75 → range 38
    let base = if abs_lat < 10.0 {
        2.0 + abs_lat * 0.3
    } else if abs_lat < 25.0 {
        5.0 + (abs_lat - 10.0) * 0.467
    } else if abs_lat < 40.0 {
        12.0 + (abs_lat - 25.0) * 0.667
    } else if abs_lat < 55.0 {
        22.0 + (abs_lat - 40.0) * 0.533
    } else {
        30.0 + (abs_lat - 55.0) * 0.4
    };

    // Ocean damping: smooth transition based on distance to ocean
    // Very coastal (<0.05) → 45% of range, inland → full range
    let ocean_damp = if distance_to_ocean < 0.15 {
        0.45 + 0.55 * (distance_to_ocean / 0.15)
    } else {
        1.0
    };

    base * ocean_damp
}

/// Estimate months with temperature above 10°C from warmest/coldest month.
fn months_above_10(t_coldest: f32, t_warmest: f32) -> f32 {
    if t_warmest < 10.0 { return 0.0; }
    if t_coldest >= 10.0 { return 12.0; }
    let t_mean = (t_coldest + t_warmest) / 2.0;
    let amp = (t_warmest - t_coldest) / 2.0;
    if amp < 0.001 { return if t_mean >= 10.0 { 12.0 } else { 0.0 }; }
    let ratio = (10.0 - t_mean) / amp;
    if ratio >= 1.0 { return 0.0; }
    if ratio <= -1.0 { return 12.0; }
    (12.0 / std::f32::consts::PI) * ratio.acos()
}

/// Approximate number of months above 10 °C at a cell — a continuous
/// growing-season proxy (0..12) reused by the settlement carrying-capacity model.
/// Mirrors the t_coldest/t_warmest derivation in `classify_cell`.
pub(crate) fn growing_season_months(buf: &WorldBuffer, x: u32, y: u32) -> f32 {
    let idx = buf.idx(x, y);
    let abs_lat = buf.latitude(y).abs();
    let range = seasonal_range(abs_lat, buf.distance_to_ocean[idx]);
    let t = buf.temperature[idx];
    months_above_10(t - range * 0.55, t + range * 0.45)
}

/// Check if there is ocean in the upwind direction within 2 cells.
fn is_windward_ocean(buf: &WorldBuffer, x: u32, y: u32) -> bool {
    let lat = buf.latitude(y);
    let abs_lat = lat.abs();
    let sign = if lat >= 0.0 { 1.0f32 } else { -1.0 };

    let (upwx, upwy) = if abs_lat < 30.0 || abs_lat >= 60.0 {
        (0.707f32, -sign * 0.707) // against trades/polar
    } else {
        (-0.707f32, sign * 0.707)  // against westerlies
    };

    let rwx = upwx.round() as i32;
    let rwy = upwy.round() as i32;

    // Reach a few cells upwind: a windward coast just needs ocean to weather from,
    // and on a coarse grid a 2-cell reach missed most genuine west coasts (this is
    // a big reason Mediterranean climates were so rare). 4 cells is still local.
    for &(dx, dy) in &[(rwx, rwy), (rwx, 0i32)] {
        if dx == 0 && dy == 0 { continue; }
        for s in 1..=4i32 {
            let nx = buf.wrap_x(x as i32 + dx * s);
            let ny = y as i32 + dy * s;
            if ny < 0 || ny >= buf.height as i32 {
                return true;
            }
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] == 0 {
                return true;
            }
        }
    }
    false
}

/// Check if a warm ocean current is near enough to suppress the Mediterranean
/// (dry-summer) pattern. Real Mediterranean climates sit beside *cold* eastern-
/// boundary currents; a warm current upwind brings year-round moisture and must
/// veto Cs. We look both along the rounded upwind direction *and* across the
/// immediate cardinal coast (within 3 cells) so a coastal warm current isn't
/// missed when the wind vector rounds away from it.
fn is_upwind_warm_current(buf: &WorldBuffer, x: u32, y: u32) -> bool {
    let lat = buf.latitude(y);
    let abs_lat = lat.abs();
    let sign = if lat >= 0.0 { 1.0f32 } else { -1.0 };

    let (upwx, upwy) = if abs_lat < 30.0 || abs_lat >= 60.0 {
        (0.707f32, -sign * 0.707)
    } else {
        (-0.707f32, sign * 0.707)
    };

    let rwx = upwx.round() as i32;
    let rwy = upwy.round() as i32;

    // Directions to probe: rounded upwind, its horizontal component, and the
    // four cardinals (catch coastal warm currents regardless of wind rounding).
    let dirs: [(i32, i32); 6] = [
        (rwx, rwy), (rwx, 0),
        (-1, 0), (1, 0), (0, -1), (0, 1),
    ];

    for &(dx, dy) in &dirs {
        if dx == 0 && dy == 0 { continue; }
        for s in 1..=3i32 {
            let nx = buf.wrap_x(x as i32 + dx * s);
            let ny = y as i32 + dy * s;
            if ny < 0 || ny >= buf.height as i32 { break; }
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] == 0 {
                if buf.current_type[ni] == 1 { return true; }
            } else if s > 1 {
                // Hit land beyond the immediate neighbour → stop probing this ray.
                break;
            }
        }
    }
    false
}

/// Smooth seasonal precipitation split factor.
/// Returns (summer_fraction, winter_fraction) as multipliers on monthly precip.
/// Uses smooth transitions instead of hard latitude thresholds.
fn seasonal_split(
    abs_lat: f32,
    windward_ocean: bool,
    upwind_warm: bool,
    near_ocean: bool,
) -> (f32, f32) {
    // Equatorial: nearly even
    if abs_lat < 10.0 {
        let t = abs_lat / 10.0; // 0 at equator, 1 at 10°
        let summer = 1.05 + t * 0.55; // ramps from 1.05 to 1.60
        let winter = 0.95 - t * 0.55;
        return (summer, winter);
    }

    // Tropical (10-25): summer-dominant monsoon
    if abs_lat < 25.0 {
        let t = (abs_lat - 10.0) / 15.0; // 0-1
        let summer = 1.60 + t * 0.20; // 1.60 to 1.80
        let winter = 0.40 - t * 0.20; // 0.40 to 0.20
        return (summer, winter);
    }

    // Subtropical to mid-latitude (25-45): depends on wind/ocean
    if abs_lat < 45.0 {
        // Smooth transition from trades to westerlies influence
        let t = (abs_lat - 25.0) / 20.0; // 0 at 25°, 1 at 45°
        // Trade influence fades, westerly influence grows
        let trade_influence = 1.0 - t;
        let westerly_influence = t;

        if windward_ocean && !upwind_warm && abs_lat >= 30.0 {
            // Mediterranean pattern: the subtropical high parks over west-facing
            // coasts in summer (dry) while the winter westerlies bring the rain.
            // The previous split only became dry enough to satisfy Köppen's
            // summer<winter/3 test within a sliver near 45°, so Cs almost never
            // appeared. Ramp the dry-summer strength in quickly from 30° so Cs
            // covers its real ~30–45° band (Iberia, California, Chile, Cape, SW
            // Australia).
            // Ramp the dry summer in from ~30° (was effectively ~32°) and dry it a
            // touch harder, so the whole 30–45° windward band clears Köppen's
            // summer<winter/3 test and reads Cs.
            let med = ((abs_lat - 29.0) / 3.0).clamp(0.0, 1.0);
            let summer = (0.52 - 0.36 * med).max(0.15);
            let winter = (1.50 + 0.55 * med).max(0.18);
            return (summer, winter);
        }

        if windward_ocean && upwind_warm {
            // Warm current → year-round moisture
            return (0.80, 1.20);
        }

        if near_ocean && westerly_influence > 0.3 {
            // Near coast but NOT windward — i.e. *east* coasts in the westerly
            // belt (ocean lies downwind to the east). These are humid
            // subtropical / oceanic (Cfa/Cfb), with NO dry summer — frequently
            // summer-wet (monsoonal east coasts: SE USA, E China, SE Brazil, E
            // Australia). The old split made winter strongly dominant near 45°
            // (summer 0.40 / winter 1.60), which tripped Köppen's dry-summer
            // test and put *Mediterranean* on east coasts. Keep summer ≳ winter
            // so these never read as Cs; the Mediterranean (dry-summer) pattern
            // is reserved for the windward west-coast branch above.
            let summer = 1.30 * trade_influence + 1.05 * westerly_influence;
            let winter = 0.70 * trade_influence + 0.95 * westerly_influence;
            return (summer.max(0.5), winter.max(0.5));
        }

        // Interior: mild contrast
        let summer = 1.40 * trade_influence + 0.50 * westerly_influence;
        let winter = 0.60 * trade_influence + 1.05 * westerly_influence;
        return (summer.max(0.40), winter.max(0.40));
    }

    // High latitude (45+): mild seasonal contrast
    (1.10, 0.85)
}

/// Classify a single land cell's Köppen zone.
fn classify_cell(buf: &WorldBuffer, x: u32, y: u32) -> u8 {
    let idx = buf.idx(x, y);
    let temp = buf.temperature[idx];
    let precip = buf.precipitation[idx];
    let lat = buf.latitude(y);
    let abs_lat = lat.abs();
    let elevation = buf.elevation[idx];
    let dist_ocean = buf.distance_to_ocean[idx];

    // Check if near ocean (within 2 cells in cardinal directions)
    let near_ocean = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2)]
        .iter()
        .any(|&(dx, dy)| {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = (y as i32 + dy).clamp(0, buf.height as i32 - 1) as u32;
            buf.terrain[buf.idx(nx, ny)] == 0
        });

    let windward_ocean = is_windward_ocean(buf, x, y);
    let upwind_warm = is_upwind_warm_current(buf, x, y);

    // Smooth seasonal range
    let range = seasonal_range(abs_lat, dist_ocean);
    let t_coldest = temp - range * 0.55;
    let t_warmest = temp + range * 0.45;
    let p12 = precip / 12.0;

    // Smooth seasonal precipitation split
    let (summer_mult, winter_mult) = seasonal_split(abs_lat, windward_ocean, upwind_warm, near_ocean);
    let summer_wet = p12 * summer_mult;
    let winter_wet = p12 * winter_mult;

    // Aridity threshold
    let b_threshold = if summer_wet > winter_wet * 2.33 {
        20.0 * temp + 280.0
    } else if winter_wet > summer_wet * 2.33 {
        20.0 * temp
    } else {
        20.0 * temp + 140.0
    };

    let n_months_10 = months_above_10(t_coldest, t_warmest);

    // Precipitation seasonality (third Köppen letter f / s / w).
    //   s = dry summer, w = dry winter, f = no marked dry season.
    // A simplified but symmetric ratio test (a season < 1/3 of the other) so the
    // full s/w/f variety can actually appear.
    let summer_dry = summer_wet < winter_wet / 3.0;
    let winter_dry = winter_wet < summer_wet / 3.0;

    // Summer-warmth subclass (a/b/c) and extreme-cold flag (d).
    let hot_summer = t_warmest >= 22.0;          // a
    let warm_summer = !hot_summer && n_months_10 >= 4.0; // b
    let extreme_cold = t_coldest < -38.0;        // d

    // Highland (H): genuine high-elevation alpine climate, independent of the
    // temperature/precip class. The treeline falls with latitude — tropical peaks
    // stay forested/temperate much higher than polar ones — so use a
    // latitude-adjusted threshold instead of a blanket 0.55. This frees temperate
    // and Mediterranean UPLANDS (≈0.42–0.55) to read Cfb / CSb (highland-
    // Mediterranean) instead of being swallowed by H, while tropical summits still
    // need to be genuinely high before turning alpine.
    let treeline = (0.62 - abs_lat * 0.0030).clamp(0.42, 0.62);
    if elevation > treeline { return H; }

    // Polar (E)
    if t_warmest < 10.0 {
        return if t_warmest < 0.0 { EF } else { ET };
    }

    // Arid (B)
    if precip < b_threshold {
        let is_desert = precip < b_threshold * 0.5;
        let lat_force_hot = abs_lat < 20.0 && elevation < 0.25;
        let is_hot = temp >= 18.0 || lat_force_hot;
        if is_desert {
            return if is_hot { BWH } else { BWK };
        }
        return if is_hot { BSH } else { BSK };
    }

    // Tropical (A)
    if t_coldest >= 18.0 {
        let p_driest = summer_wet.min(winter_wet);
        if p_driest >= 60.0 { return AF; }
        if p_driest >= 100.0 - precip / 25.0 { return AM; }
        return if summer_dry { AS } else { AW };
    }

    // Temperate (C): -3 < t_coldest < 18
    if t_coldest > -3.0 {
        if summer_dry && precip < 900.0 {
            if hot_summer { return CSA; }
            if warm_summer { return CSB; }
            return CSC;
        }
        if winter_dry {
            if hot_summer { return CWA; }
            if warm_summer { return CWB; }
            return CWC;
        }
        if hot_summer { return CFA; }
        if warm_summer { return CFB; }
        return CFC;
    }

    // Continental (D): t_coldest <= -3, t_warmest >= 10
    if summer_dry && precip < 900.0 {
        if hot_summer { return DSA; }
        if warm_summer { return DSB; }
        return if extreme_cold { DSD } else { DSC };
    }
    if winter_dry {
        if hot_summer { return DWA; }
        if warm_summer { return DWB; }
        return if extreme_cold { DWD } else { DWC };
    }
    if hot_summer { return DFA; }
    if warm_summer { return DFB; }
    if extreme_cold { return DFD; }
    DFC
}

/// Soft latitude plausibility guardrails.
///
/// Real Köppen is set by temperature/precipitation, but threshold artefacts let
/// implausible zones slip through (the classic "temperate broadleaf forest on
/// the equator"). These clamps forbid the clearly-impossible cases while leaving
/// the borderline ones to the current-override pass, which can still extend a
/// zone poleward (Gulf-Stream style). Mountains (high elevation) are exempt —
/// they legitimately host cold climates at any latitude.
fn latitude_guardrail(code: u8, abs_lat: f32, elevation: f32, precip: f32, temp: f32) -> u8 {
    // Above the treeline we don't second-guess the classifier.
    if elevation > 0.40 { return code; }

    let is_tropical = matches!(code, AF | AM | AW | AS);
    let is_temperate = matches!(code,
        CSA | CSB | CSC | CFA | CFB | CFC | CWA | CWB | CWC);
    let is_continental = matches!(code,
        DFA | DFB | DFC | DFD | DSA | DSB | DSC | DSD | DWA | DWB | DWC | DWD);
    // The coldest continental classes (subarctic / extreme). These plus polar are
    // what a cold-current + upwelling coast wrongly produces in the subtropics.
    let is_subarctic = matches!(code, DFC | DFD | DWC | DWD | DSC | DSD);
    let is_polar = code == ET || code == EF;

    // Deep tropics (<12°): no temperate/continental forest. Reclassify by moisture.
    if abs_lat < 12.0 && (is_temperate || is_continental) {
        let b_thresh = 20.0 * temp + 140.0;
        if precip < b_thresh * 0.5 { return BWH; }
        if precip < b_thresh { return BSH; }
        if precip >= 1800.0 { return AF; }
        if precip >= 900.0 { return AM; }
        return AW;
    }

    // Tropical zones can't sit in the mid-latitudes. Allow a little overshoot
    // (monsoon/savanna reach ~30°); beyond that, demote toward subtropical.
    if abs_lat > 32.0 && is_tropical {
        let b_thresh = 20.0 * temp + 140.0;
        if precip < b_thresh { return if temp >= 18.0 { BSH } else { BSK }; }
        return CFA;
    }

    // Cold-computed cells in the tropics / subtropics are an over-cooling
    // artefact (upwelling + cold-current coasts pull the annual mean down). Real
    // such coasts are cool deserts, steppe or mild temperate zones — *never*
    // subarctic or polar. The old guardrail bumped polar→DFC all the way down to
    // 38°, so an over-cooled tropical coast (ET) became subarctic (Dfc) — the
    // "subarctic climate on a tropical coastline" bug. Below ~25° demote any
    // polar/continental class by moisture instead (cool desert / steppe / mild
    // temperate, like coastal Peru or Namibia).
    if abs_lat < 25.0 && (is_polar || is_continental) {
        let b_thresh = 20.0 * temp + 140.0;
        if precip < b_thresh * 0.5 { return if temp >= 18.0 { BWH } else { BWK }; }
        if precip < b_thresh { return if temp >= 18.0 { BSH } else { BSK }; }
        return CFA;
    }

    // Between 25-38° a cold-current / upwelling coast can still over-cool into
    // polar or *subarctic*. Those are impossible at sea level in the subtropics
    // (the teal "subarctic next to a hot desert" coastal strip). Demote by
    // moisture to a cool desert / steppe / oceanic class instead — the real
    // Atacama / Namib / Benguela-coast outcome.
    if abs_lat < 38.0 && (is_polar || is_subarctic) {
        let b_thresh = 20.0 * temp + 140.0;
        if precip < b_thresh * 0.5 { return BWK; }
        if precip < b_thresh { return BSK; }
        return CFB;
    }

    code
}

/// Classify Köppen climate zones for all land cells, then apply current overrides.
pub fn classify_koppen(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;

    // Pass 1: classify all land cells (with latitude guardrails)
    for y in 0..h {
        let abs_lat = buf.latitude(y).abs();
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 {
                buf.koppen[idx] = 0;
                continue;
            }
            let raw = classify_cell(buf, x, y);
            buf.koppen[idx] = latitude_guardrail(
                raw, abs_lat, buf.elevation[idx], buf.precipitation[idx], buf.temperature[idx],
            );
        }
    }

    // Pass 2: apply current-driven overrides (can extend a zone past its band)
    apply_current_overrides(buf);

    // Pass 3: majority filter to dissolve thin "zebra" stripes that arise when
    // the aridity threshold flips between adjacent latitude rows.
    smooth_koppen(buf, 3);
}

/// Replace each land cell whose class is a small minority among its 3×3
/// neighbourhood with the locally dominant class. Removes single-cell stripes
/// without blurring genuine zone boundaries.
fn smooth_koppen(buf: &mut WorldBuffer, passes: u32) {
    let w = buf.width;
    let h = buf.height;
    for _ in 0..passes {
        let src = buf.koppen.clone();
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                if buf.terrain[idx] != 1 { continue; }
                let self_code = src[idx];
                let mut counts: [u8; 33] = [0; 33];
                let mut best = self_code;
                let mut best_n = 0u8;
                let mut self_n = 0u8;
                for dy in -1i32..=1 {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    for dx in -1i32..=1 {
                        let ni = buf.widx(x as i32 + dx, ny);
                        if buf.terrain[ni] != 1 { continue; }
                        let c = src[ni];
                        if c == 0 || c > 32 { continue; }
                        counts[c as usize] += 1;
                        if counts[c as usize] > best_n { best_n = counts[c as usize]; best = c; }
                        if c == self_code { self_n = counts[c as usize]; }
                    }
                }
                // Only override clear minorities so real boundaries are preserved.
                if best != self_code && self_n <= 2 && best_n >= 5 {
                    buf.koppen[idx] = best;
                }
            }
        }
    }
}

/// Apply warm/cold current influence on coastal Köppen zones.
///
/// Influence is accumulated from each current source into a continuous float
/// field (walking downwind onto land), then **blurred** before being thresholded
/// and applied. The previous version overwrote Köppen directly along each
/// source's discretized downwind ray; adjacent sources produced parallel rays
/// that read as a diagonal "zebra" of (e.g.) Mediterranean cells. Smearing the
/// influence into a smooth coastal band first removes the rays while keeping the
/// "warm current extends temperate climate inland" effect.
fn apply_current_overrides(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    let warm_radius = ((w as f32 / EARTH_W * 70.0).round() as i32).max(5);
    let cold_radius = ((w as f32 / EARTH_W * 14.0).round() as i32).max(2);

    let mut warm = vec![0.0f32; n];
    let mut cold = vec![0.0f32; n];

    for sy in 0..h {
        for sx in 0..w {
            let sidx = buf.idx(sx, sy);
            if buf.terrain[sidx] != 0 { continue; }
            let ct = buf.current_type[sidx];
            if ct != 1 && ct != 2 { continue; }

            let wvx = buf.wind_vx[sidx];
            let wvy = buf.wind_vy[sidx];
            let wind_len = (wvx * wvx + wvy * wvy).sqrt();
            if wind_len < 0.01 { continue; }
            let ndx = wvx / wind_len;
            let ndy = wvy / wind_len;

            let max_radius = if ct == 1 { warm_radius } else { cold_radius };
            let scale = (max_radius as f32 * 0.45).max(2.0);

            for step in 1..=max_radius {
                let tx = buf.wrap_x(sx as i32 + (ndx * step as f32).round() as i32);
                let ty = (sy as i32 + (ndy * step as f32).round() as i32).clamp(0, h as i32 - 1) as u32;
                let tidx = buf.idx(tx, ty);
                if buf.terrain[tidx] != 1 { break; } // stop at ocean/edge
                let contrib = (-(step as f32) / scale).exp();
                if ct == 1 { warm[tidx] += contrib; } else { cold[tidx] += contrib; }
            }
        }
    }

    // Smear the ray accumulation into a coherent coastal band (kills the rays).
    blur_field(&mut warm, &buf.terrain, w, h, 4);
    blur_field(&mut cold, &buf.terrain, w, h, 4);

    // Apply the stronger influence per cell, above a small threshold.
    const THRESH: f32 = 0.18;
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            let k = buf.koppen[idx];
            if k == 0 { continue; }
            let (wv, cv) = (warm[idx], cold[idx]);
            if wv < THRESH && cv < THRESH { continue; }
            let replacement = if wv >= cv {
                warm_override(k)
            } else {
                // Mediterranean (Cs) only forms on windward (west-facing) coasts
                // beside a *cold* offshore current. Gate the Cs conversions on
                // windward geometry AND the absence of warm-current influence so
                // an east coast warmed by a western-boundary current (Gulf
                // Stream / Kuroshio) reads humid-subtropical (Cfa), never Med.
                let windward = is_windward_ocean(buf, x, y) && wv < THRESH;
                cold_override(k, buf.latitude(y).abs(), windward)
            };
            if let Some(r) = replacement { buf.koppen[idx] = r; }
        }
    }

    // ── Hard rule: kill Mediterranean (Cs) on warm-current / lee coasts ───────
    // A real Mediterranean climate sits on a WINDWARD (west-facing) coast beside a
    // COLD offshore current. Any Cs cell that carries ANY warm-current influence
    // (even weak, below the override threshold) OR that is not a windward coast
    // (i.e. an east / lee coast warmed by a western-boundary current) is demoted to
    // humid-subtropical / oceanic. This catches the residual east-coast Med that
    // slipped through both the seasonal split and the thresholded override above.
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            let k = buf.koppen[idx];
            if k != CSA && k != CSB { continue; }
            let warm_here = warm[idx] > 0.05;
            let windward = is_windward_ocean(buf, x, y) && !warm_here;
            if warm_here || !windward {
                buf.koppen[idx] = if k == CSA { CFA } else { CFB };
            }
        }
    }
}

/// Box-blur a land-masked f32 field in place (ocean cells held at 0).
fn blur_field(field: &mut [f32], terrain: &[u8], w: u32, h: u32, passes: u32) {
    let n = (w * h) as usize;
    let mut tmp = vec![0.0f32; n];
    for _ in 0..passes {
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                if terrain[idx] != 1 { tmp[idx] = 0.0; continue; }
                let mut sum = field[idx];
                let mut cnt = 1.0f32;
                for &(dx, dy) in &[
                    (-1i32, 0i32), (1, 0), (0, -1), (0, 1),
                    (-1, -1), (1, -1), (-1, 1), (1, 1),
                ] {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let nx = ((x as i32 + dx) % w as i32 + w as i32) % w as i32;
                    let ni = (ny as u32 * w + nx as u32) as usize;
                    if terrain[ni] != 1 { continue; }
                    sum += field[ni];
                    cnt += 1.0;
                }
                tmp[idx] = sum / cnt;
            }
        }
        field.copy_from_slice(&tmp);
    }
}

fn warm_override(k: u8) -> Option<u8> {
    match k {
        BWH => Some(BSH),
        BWK => Some(BSK),
        BSH => Some(CFA),
        BSK => Some(CFB),
        CSA => Some(CFA),
        CSB => Some(CFB),
        DFA => Some(CFA),
        DFB => Some(CFB),
        DFC => Some(CFB),
        DFD => Some(DFC),
        ET  => Some(DFC),
        _ => None,
    }
}

fn cold_override(k: u8, target_lat: f32, windward: bool) -> Option<u8> {
    match k {
        AF | AM | AW => Some(BWH),
        BSH => Some(BWH),
        BSK => Some(BWK),
        // Mediterranean only on windward west coasts within the Med latitude band.
        CFA => {
            if windward && target_lat >= MED_LAT_MIN && target_lat <= MED_LAT_MAX { Some(CSA) } else { None }
        }
        CFB => {
            if windward && target_lat >= MED_LAT_MIN && target_lat <= MED_LAT_MAX { Some(CSB) } else { None }
        }
        DFA => Some(DFB),
        DFB => Some(DFC),
        _ => None,
    }
}
