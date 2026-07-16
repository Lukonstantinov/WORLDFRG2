use super::world_buffer::WorldBuffer;

// ── Local value-noise (keeps moisture flow from following dead-straight rays) ──
fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = seed as i32;
    h = h.wrapping_add(x.wrapping_mul(374761393));
    h ^= y.wrapping_mul(668265263);
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    h = h.wrapping_mul(1911520717);
    h ^= h >> 16;
    (h as u32) as f32 / 4294967296.0
}
fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let v00 = hash2(ix, iy, seed);
    let v10 = hash2(ix + 1, iy, seed);
    let v01 = hash2(ix, iy + 1, seed);
    let v11 = hash2(ix + 1, iy + 1, seed);
    let top = v00 * (1.0 - sx) + v10 * sx;
    let bot = v01 * (1.0 - sx) + v11 * sx;
    top * (1.0 - sy) + bot * sy
}
/// Two-octave fbm in [-1,1], used to perturb advection headings and add texture.
fn fbm2(x: f32, y: f32, seed: u32) -> f32 {
    let a = vnoise(x, y, seed);
    let b = vnoise(x * 2.13 + 5.1, y * 2.13 - 3.7, seed.wrapping_add(9173));
    ((a * 0.65 + b * 0.35) - 0.5) * 2.0
}

// ── Constants (ported from WF1 precipitation.ts / itcz.ts / orographic-lift.ts) ──
const BASE_PRECIPITATION: f32 = 800.0;

// Per-packet heading fan-out (radians) so the discrete wind belts don't lay
// down identical straight diagonal rays.
const MEANDER_TURN: f32 = 0.85;

// Earth's circumference (km) — used to translate moisture-decay distances
// expressed in real km into a per-cell decay that is independent of grid
// resolution. The previous fixed FWDA_STEPS=60 / decay 0.935-0.960 were tuned
// for a tiny grid; on the 3600-wide default a moisture packet died after ~60
// cells (~660 km) so every continental interior beyond that collapsed to the
// 0.04 floor → instant desert. The Amazon, the US/Asian interiors etc. all
// dried out. We now scale the travel distance and per-cell decay to the grid.
const EARTH_CIRCUMFERENCE_KM: f32 = 40075.0;
// Moisture e-folding distances (km): how far inland precip falls to 1/e of its
// coastal value. Mid/high latitudes carry moisture much deeper than the hot
// tropics, whose convective interiors dry out faster.
const EFOLD_MID_KM: f32 = 1700.0;
const EFOLD_TROP_KM: f32 = 1300.0;
// Minimum moisture for any reachable land cell (floor before climatic terms).
// Bumped from 0.04 so deep interiors that no packet reaches still get a little
// baseline rather than reading as absolute desert; subtropical-high / cold-coast
// drying still pushes genuine deserts back down.
const MOISTURE_FLOOR: f32 = 0.09;

// Coastal drying
const COLD_COAST_DRYING: f32 = 0.35; // cold-current coast → fog desert
const UPWELLING_DRYING: f32 = 0.60;  // upwelling-cooled coast → extra drying

// ── Enclosed subtropical sea suppression (Red Sea / Persian Gulf effect) ──
// Warm but narrow seas sitting under the Hadley-cell subtropical high evaporate
// enormous amounts of water yet produce almost no rain. Three barriers combine:
//   1. Sinking subtropical-high air (the descending Hadley branch near ~30°)
//      compresses and warms, capping any convection.
//   2. The surrounding deserts blow a hot, dry inversion lid over the marine
//      boundary layer, trapping humidity at the surface.
//   3. The basin is narrow — air crosses too fast to load the moisture needed to
//      break the high.
// We model this by suppressing the moisture such basins *emit* (and drying their
// immediate coasts) so Red-Sea / Persian-Gulf-style seas stay hyper-arid and
// their desert coasts (the Arabian / Sudanese Red Sea coast) don't sprout a wet
// belt the real world never has.
const ENCLOSED_SUPPRESS_STRENGTH: f32 = 0.85; // max fraction of emitted moisture removed
const ENCLOSED_NARROW_KM: f32 = 1400.0;       // seas narrower than this read as "enclosed"
const ENCLOSED_COAST_DRYING: f32 = 0.45;      // extra drying on land bordering such seas

// Orographic
const MOUNTAIN_THRESHOLD: f32 = 0.19; // elevation counting as a ridge (~1700 m)
const WINDWARD_STEPS: i32 = 12;
const SHADOW_STEPS: i32 = 20;

// ── Low-level-jet entrance/exit dynamics (jets.rs supplies buf.wind_speed) ──
// A jet ENTRANCE (accelerating flow) diverges at low level and sweeps moisture
// along before it can accumulate → the coast dries (Somali / Arabian coast). A
// jet EXIT (decelerating flow) converges and is forced to rise → torrential
// rainfall at the terminus (the Western-Ghats monsoon dump).
const JET_MIN_SPEED: f32 = 8.0;      // below this a cell isn't in a jet
const JET_ACCEL_SCALE: f32 = 6.0;    // along-flow Δspeed (m/s) that saturates the effect
const JET_ENTRANCE_DRY: f32 = 0.55;  // max fraction of moisture removed at a strong entrance
const JET_EXIT_WET_MAX: f32 = 750.0; // max convergence rainfall (mm/yr) at a strong exit

/// Orographic precipitation multiplier for one land cell.
/// 2.5 = windward uplift, 0.15..1.0 = leeward rain shadow (graduated), 1.0 = none.
fn orographic_multiplier(buf: &WorldBuffer, x: u32, y: u32, wvx: f32, wvy: f32) -> f32 {
    let wind_len = (wvx * wvx + wvy * wvy).sqrt();
    if wind_len < 0.01 { return 1.0; }
    let ndx = wvx / wind_len;
    let ndy = wvy / wind_len;
    let h = buf.height as i32;

    // Leeward (rain shadow): walk upwind for a recently crossed mountain.
    for step in 1..=SHADOW_STEPS {
        let bx = (x as f32 - ndx * step as f32).round() as i32;
        let by = (y as f32 - ndy * step as f32).round() as i32;
        if by < 0 || by >= h { break; }
        let bi = buf.idx(buf.wrap_x(bx), by as u32);
        if buf.terrain[bi] == 0 { break; } // open ocean upstream → no shadow
        if buf.elevation[bi] > MOUNTAIN_THRESHOLD {
            return 0.15 + 0.85 * (((step - 1) as f32 / SHADOW_STEPS as f32).sqrt());
        }
    }

    // Windward: walk downwind for an approaching mountain.
    for step in 1..=WINDWARD_STEPS {
        let fx = (x as f32 + ndx * step as f32).round() as i32;
        let fy = (y as f32 + ndy * step as f32).round() as i32;
        if fy < 0 || fy >= h { break; }
        let fi = buf.idx(buf.wrap_x(fx), fy as u32);
        if buf.terrain[fi] == 0 { break; } // ocean downwind → no windward uplift
        if buf.elevation[fi] > MOUNTAIN_THRESHOLD { return 2.5; }
    }

    1.0
}

/// ITCZ latitude shift from land distribution (monsoon asymmetry). Range ±12°.
fn compute_itcz_shift(buf: &WorldBuffer) -> f32 {
    let (mut nh_land, mut sh_land, mut nh_total, mut sh_total) = (0.0f32, 0.0, 0.0, 0.0);
    for y in 0..buf.height {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();
        if abs_lat > 30.0 { continue; }
        let weight = 1.0 - abs_lat / 30.0;
        for x in 0..buf.width {
            let is_land = buf.terrain[buf.idx(x, y)] == 1;
            if lat >= 0.0 {
                nh_total += weight;
                if is_land { nh_land += weight; }
            } else {
                sh_total += weight;
                if is_land { sh_land += weight; }
            }
        }
    }
    if nh_total == 0.0 || sh_total == 0.0 { return 0.0; }
    let diff = nh_land / nh_total - sh_land / sh_total;
    (diff * 20.0).clamp(-12.0, 12.0)
}

/// Shifted ITCZ precipitation bonus (mm/yr) at a latitude.
///
/// Widened taper (was full ≤5°, zero by 12°). The ITCZ migrates seasonally
/// across a broad tropical band, so the annual-mean wet zone reaches well past
/// ±12° in continental monsoon regions — that band is what keeps tropical
/// rainforest/savanna interiors (Amazon, Congo, the Sahel margin) wet instead
/// of collapsing to desert once they sit beyond pure trade-wind advection.
fn itcz_bonus_shifted(lat: f32, shift: f32) -> f32 {
    let eff = (lat - shift).abs();
    if eff <= 6.0 { 1200.0 }
    else if eff <= 18.0 { 1200.0 * (1.0 - (eff - 6.0) / 12.0) }
    else { 0.0 }
}

/// Strength (0..1) of the subtropical-high sinking-air inversion by latitude.
/// Peaks across the descending branch of the Hadley cell (~20-32°), tapering to
/// zero by 12° (where the ITCZ takes over) and 38° (where the mid-latitude storm
/// track takes over). This is what lets a warm enclosed sea here suppress rain.
fn hadley_inversion(abs_lat: f32) -> f32 {
    if abs_lat <= 12.0 || abs_lat >= 38.0 { 0.0 }
    else if abs_lat < 20.0 { (abs_lat - 12.0) / 8.0 }
    else if abs_lat <= 32.0 { 1.0 }
    else { (38.0 - abs_lat) / 6.0 }
}

/// Per-ocean-cell suppression factor (0..1) for warm enclosed subtropical seas.
/// Combines the Hadley-inversion strength at the cell's latitude with how
/// *narrow* the basin is (measured as the shorter of its N-S / E-W open-water
/// span). Wide open ocean → 0 (normal moisture); a narrow strait under the
/// subtropical high → near 1 (heavily suppressed). Land cells stay 0.
fn compute_enclosed_suppression(buf: &WorldBuffer, km_per_cell: f32) -> Vec<f32> {
    let w = buf.width;
    let h = buf.height;
    let cap = ((ENCLOSED_NARROW_KM / km_per_cell).round() as i32).max(8);
    let mut sup = vec![0.0f32; buf.total()];

    for y in 0..h {
        let inv = hadley_inversion(buf.latitude(y).abs());
        if inv <= 0.0 { continue; } // outside the subtropical-high band
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 { continue; } // ocean only

            // East-West open-water span through this cell (break at first land).
            let mut ew = 1i32;
            for d in 1..=cap {
                if buf.terrain[buf.idx(buf.wrap_x(x as i32 - d), y)] != 0 { break; }
                ew += 1;
            }
            for d in 1..=cap {
                if buf.terrain[buf.idx(buf.wrap_x(x as i32 + d), y)] != 0 { break; }
                ew += 1;
            }
            // North-South open-water span.
            let mut ns = 1i32;
            for d in 1..=cap {
                let ny = y as i32 - d;
                if ny < 0 || buf.terrain[buf.idx(x, ny as u32)] != 0 { break; }
                ns += 1;
            }
            for d in 1..=cap {
                let ny = y as i32 + d;
                if ny >= h as i32 || buf.terrain[buf.idx(x, ny as u32)] != 0 { break; }
                ns += 1;
            }

            // Narrowness in the tighter axis: a narrow channel (Red Sea, Gulf) is
            // what defeats convection regardless of its long axis.
            let span = ew.min(ns);
            let narrow = ((cap - span) as f32 / cap as f32).clamp(0.0, 1.0);
            sup[idx] = inv * narrow;
        }
    }
    sup
}

/// Subtropical-high fractional drying (0..0.70), peak near 28°.
fn subtropical_penalty(abs_lat: f32) -> f32 {
    let dist = (abs_lat - 28.0).abs();
    if dist >= 10.0 { 0.0 } else { 0.70 * (1.0 - dist / 10.0) }
}

/// Monsoon multiplier (1.0..2.0) for continental tropical interiors.
fn monsoon_multiplier(abs_lat: f32, dist_ocean: f32, land_frac_tropical: f32) -> f32 {
    if abs_lat < 8.0 || abs_lat > 35.0 { return 1.0; }
    let lat_weight = if abs_lat < 15.0 { (abs_lat - 8.0) / 7.0 }
        else if abs_lat <= 25.0 { 1.0 }
        else { 1.0 - (abs_lat - 25.0) / 10.0 };
    // Monsoons are continental-scale: they pull moisture deep into the
    // interior, so the inland falloff must not bottom out near zero (the old
    // 0.1 floor beyond ~33 cells left tropical interiors arid on large grids).
    let dist_weight = if dist_ocean < 0.02 { 0.6 }
        else if dist_ocean < 0.25 { 1.0 }
        else { (1.0 - (dist_ocean - 0.25) * 0.55).max(0.5) };
    let cont_weight = (land_frac_tropical * 3.0).min(1.0);
    1.0 + (lat_weight * dist_weight * cont_weight)
}

/// Dedicated monsoon ADDITIVE bonus (mm/yr). The multiplicative `monsoon_multiplier`
/// can only scale an existing base, so where the downwind-advection base is near
/// zero (continental subtropics in the easterly trades — e.g. the Indian
/// subcontinent) it left the land as desert. The summer monsoon instead draws a
/// large, fresh moisture load off the warm seas onto the land, so it is modelled
/// as an ADDITIVE term over the 5–45° coastal belt: strongest on/near the coast,
/// reaching inland on big continents. This wets India, SE Asia, E China and the
/// Japan / E-Asian seaboard; the dry-winter seasonal split (koppen.rs) then turns
/// these into proper monsoon climates (Aw/Am, Cwa/Cwb, Dwa/Dwb).
fn monsoon_bonus(abs_lat: f32, dist_ocean: f32, land_frac_tropical: f32) -> f32 {
    if abs_lat < 5.0 || abs_lat > 45.0 { return 0.0; }
    // Latitude weight: full across the core monsoon belt (12–28°), ramping in from
    // 5° and fading out by 45° (so the E-Asian / Japan monsoon still reaches).
    let lat_w = if abs_lat < 12.0 { (abs_lat - 5.0) / 7.0 }
        else if abs_lat <= 28.0 { 1.0 }
        else { (1.0 - (abs_lat - 28.0) / 17.0).max(0.0) };
    // Proximity to the moisture source (the warm sea): strong near the coast,
    // penetrating well inland on large continents but fading in deep interiors.
    let prox = if dist_ocean < 0.03 { 0.75 }
        else if dist_ocean < 0.28 { 1.0 }
        else { (1.0 - (dist_ocean - 0.28) * 1.1).max(0.0) };
    // Continentality: a big landmass drives a stronger monsoon (land–sea thermal
    // contrast); small islands get only a mild boost.
    let cont = (0.45 + land_frac_tropical * 2.4).min(1.25);
    const MONSOON_BONUS_MAX: f32 = 850.0;
    MONSOON_BONUS_MAX * lat_w * prox * cont
}

/// Onshore-monsoon GATE (0..1). A monsoon only fires where summer flow blows OFF a
/// warm sea ONTO the land — i.e. there is a warm ocean on the cell's EQUATORWARD
/// and/or EASTERN side within reach (the Indian/SE-Asian/E-Asian geometry). This
/// excludes subtropical-high deserts (the Sahara, Arabia, the Atacama/Namib/W-Australia
/// coasts) whose moisture source is blocked — so the monsoon term no longer turns
/// dry land green. A cold current offshore (cold-upwelling desert coast) counts for
/// little. Cheap: only the 5–45° belt calls it, scanning a few short rays.
fn monsoon_onshore(buf: &WorldBuffer, x: u32, y: u32, sea_suppress: &[f32]) -> f32 {
    let lat = buf.latitude(y);
    let h = buf.height as i32;
    let eqdir = if lat >= 0.0 { 1i32 } else { -1 }; // equatorward = toward the equator
    // Resolution-aware moisture fetch: a summer monsoon draws on a warm sea up to
    // ~MONSOON_FETCH_KM away. The old fixed 28-cell reach was only ~310 km on a
    // 3600-wide grid, so a continental interior (the Indian Deccan, the N-China
    // plain) never "saw" the ocean and stayed desert. ~800 km reaches India's
    // interior while still leaving the Sahara dry — its nearest warm equatorward
    // ocean (the Gulf of Guinea) is >1500 km away past the Sahel, and its west
    // coast is the cold Canary upwelling (discounted below).
    const MONSOON_FETCH_KM: f32 = 800.0;
    let km_per_cell = EARTH_CIRCUMFERENCE_KM / buf.width as f32;
    let range = ((MONSOON_FETCH_KM / km_per_cell) as i32).max(12);
    let mut best = 0.0f32;
    // Equatorward (S), equatorward-east (SE), equatorward-west (SW — the Indian /
    // Western-Ghats summer monsoon blows from the south-west), and due-east rays.
    for &(dx, dy) in &[(0i32, eqdir), (1, eqdir), (-1, eqdir), (1, 0)] {
        for s in 1..=range {
            let nx = buf.wrap_x(x as i32 + dx * s);
            let ny = y as i32 + dy * s;
            if ny < 0 || ny >= h { break; }
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] == 0 {
                // Enclosed/suppressed warm seas (Red Sea, Persian Gulf) sit under
                // the Hadley subtropical-high inversion and supply NO monsoon — skip
                // them as a source so East Sahara / Arabia stay dry, while India's
                // open Arabian Sea / Bay of Bengal still fires the monsoon.
                if sea_suppress[ni] > 0.4 { continue; }
                // Warm/neutral sea is a good moisture source; a cold current is not.
                let warm = if buf.current_type[ni] == 2 { 0.30 } else { 1.0 };
                best = best.max((1.0 - s as f32 / range as f32) * warm);
                break; // first usable sea cell along the ray decides it
            }
        }
    }
    best
}

/// Frontal precipitation bonus (mm/yr) at mid-latitude storm tracks (30-66°).
///
/// Extratropical cyclones deliver year-round precipitation across the whole
/// mid-latitude belt regardless of the prevailing-wind direction, which is what
/// keeps east-coast and interior temperate zones (eastern US, Europe, NE Asia)
/// humid even though the westerlies put the ocean *downwind* of them. The pure
/// downwind-advection model can't supply that moisture, so the storm-track term
/// is the main thing standing between a realistic temperate belt and a runaway
/// band of steppe. Broadened (was 35-60°) and strengthened accordingly.
fn frontal_bonus(abs_lat: f32, near_ocean: bool) -> f32 {
    if abs_lat < 30.0 || abs_lat > 66.0 { return 0.0; }
    let weight = if abs_lat < 45.0 { (abs_lat - 30.0) / 15.0 }
        else if abs_lat <= 52.0 { 1.0 }
        else { (1.0 - (abs_lat - 52.0) / 14.0).max(0.0) };
    let base = if near_ocean { 550.0 } else { 360.0 };
    base * weight
}

/// Sample the low-level wind speed at a float position (nearest cell, x-wrapped,
/// y-clamped).
#[inline]
fn sample_speed(buf: &WorldBuffer, fx: f32, fy: f32) -> f32 {
    let x = buf.wrap_x(fx.round() as i32);
    let y = buf.clamp_y(fy.round() as i32);
    buf.wind_speed[buf.idx(x, y)]
}

/// Jet entrance/exit effect for one land cell: `(dry_mult, wet_bonus_mm)`.
///
/// Reads the along-flow gradient of `buf.wind_speed`. Where the flow accelerates
/// downwind (jet entrance) it returns a drying multiplier < 1; where it
/// decelerates (jet exit / terminus) it returns an additive convergence-rainfall
/// bonus (mm/yr). Cells not in a jet (`speed < JET_MIN_SPEED`) are unaffected.
fn jet_effect(buf: &WorldBuffer, x: u32, y: u32) -> (f32, f32) {
    // No jet field loaded (e.g. an old save re-run without the jet step) → no-op.
    if buf.wind_speed.is_empty() {
        return (1.0, 0.0);
    }
    let i = buf.idx(x, y);
    let s = buf.wind_speed[i];
    if s < JET_MIN_SPEED {
        return (1.0, 0.0);
    }
    let wvx = buf.wind_vx[i];
    let wvy = buf.wind_vy[i];
    let wl = (wvx * wvx + wvy * wvy).sqrt();
    if wl < 0.01 {
        return (1.0, 0.0);
    }
    let dx = wvx / wl;
    let dy = wvy / wl;
    let step = 2.0;
    let s_down = sample_speed(buf, x as f32 + dx * step, y as f32 + dy * step);
    let s_up = sample_speed(buf, x as f32 - dx * step, y as f32 - dy * step);
    let accel = s_down - s_up; // >0 accelerating (entrance), <0 decelerating (exit)
    let norm = (accel / JET_ACCEL_SCALE).clamp(-1.0, 1.0);
    if norm > 0.0 {
        (1.0 - JET_ENTRANCE_DRY * norm, 0.0)
    } else {
        // Exit dump scales with how deep into the jet the cell sits.
        let strength = ((s - JET_MIN_SPEED) / 12.0).clamp(0.0, 1.0);
        (1.0, JET_EXIT_WET_MAX * (-norm) * strength)
    }
}

/// Compute annual precipitation (mm/yr) for every land cell.
///
/// Faithful port of WF1 `computePrecipitation`. The crucial difference from the
/// previous WF2 version: moisture is accumulated as a **maximum** field
/// (`moisture[ti] = max(moisture[ti], m)`) and read per land cell, instead of
/// *summing* deposits along each ocean cell's discretized downwind ray. Summing
/// overlapping 8-direction rays is what produced the diagonal "zebra" streaks.
pub fn compute_precipitation(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Resolution-aware moisture-advection parameters. Translate the km-based
    // e-folding distances into a per-cell decay and a travel cap so interiors
    // are penetrated correctly at any grid size (the old fixed 60-step / 0.96
    // decay starved every continent wider than ~660 km).
    let km_per_cell = EARTH_CIRCUMFERENCE_KM / w as f32;
    let efold_mid = (EFOLD_MID_KM / km_per_cell).max(8.0);
    let efold_trop = (EFOLD_TROP_KM / km_per_cell).max(6.0);
    let decay_base = (-1.0 / efold_mid).exp();   // per-cell decay, mid/high lat
    let decay_trop = (-1.0 / efold_trop).exp();  // per-cell decay, hot tropics
    // Let packets cross a wide continent (~4-5 e-foldings). Only coastal ocean
    // cells actually emit a long ray (inland-bound packets break the instant
    // they re-enter ocean), so a large cap stays cheap.
    let fwda_steps = ((efold_mid * 5.0) as i32).max(60);

    // Warm-but-narrow subtropical seas (Red Sea / Persian Gulf): per-ocean-cell
    // moisture-emission suppression under the Hadley subtropical high.
    let sea_suppress = compute_enclosed_suppression(buf, km_per_cell);

    let itcz_shift = compute_itcz_shift(buf);

    // Tropical land fraction (for monsoon strength).
    let (mut trop_land, mut trop_total) = (0.0f32, 0.0f32);
    for y in 0..h {
        if buf.latitude(y).abs() > 35.0 { continue; }
        for x in 0..w {
            trop_total += 1.0;
            if buf.terrain[buf.idx(x, y)] == 1 { trop_land += 1.0; }
        }
    }
    let land_frac_tropical = if trop_total > 0.0 { trop_land / trop_total } else { 0.0 };

    // Cold-current coast flag (any adjacent ocean cell carries a cold current)
    // and enclosed-sea coast flag (borders a strongly suppressed subtropical sea).
    let mut cold_coast = vec![false; n];
    let mut enclosed_coast = vec![false; n];
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
                if buf.terrain[ni] == 0 && buf.current_type[ni] == 2 {
                    cold_coast[idx] = true;
                }
                if buf.terrain[ni] == 0 && sea_suppress[ni] > 0.4 {
                    enclosed_coast[idx] = true;
                }
            }
        }
    }

    // ── Pass 1: forward moisture advection → MAX moisture field ──────────────
    let mut moisture_field = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 { continue; } // only ocean cells emit moisture

            let wvx = buf.wind_vx[idx];
            let wvy = buf.wind_vy[idx];
            let wl = (wvx * wvx + wvy * wvy).sqrt();
            if wl < 0.01 { continue; }
            let ndx = wvx / wl;
            let ndy = wvy / wl;

            let init_moisture = match buf.current_type[idx] {
                1 => 1.7, // warm current → much more moisture carried inland
                2 => 0.4, // cold current → far less (coastal desert downwind)
                _ => 1.0,
            };
            // A warm enclosed subtropical sea evaporates freely but the inversion
            // lid stops that moisture ever reaching the air aloft, so it emits
            // far less usable moisture downwind.
            let init_moisture = init_moisture * (1.0 - ENCLOSED_SUPPRESS_STRENGTH * sea_suppress[idx]);

            let mut m = init_moisture;
            // Fan each packet out by a small, origin-seeded *fixed* angular
            // offset. The three discrete wind belts otherwise send every packet
            // in a band along the exact same heading, laying down identical
            // diagonal rays (the "zebra"). A constant per-packet offset
            // decorrelates the rays while the straight path still carries
            // moisture deep inland (an accumulating per-step turn curved packets
            // back to sea and starved interiors → runaway deserts).
            // High-frequency per-emitter offset: at a low frequency neighbouring
            // ocean cells share an offset and lay down PARALLEL rays (the visible
            // diagonal banding). Decorrelating it cell-to-cell makes adjacent rays
            // fan apart so the multi-pass blur below evens them into a smooth field.
            let off = fbm2(x as f32 / 3.5, y as f32 / 3.5, 5237) * MEANDER_TURN;
            let (so, co) = off.sin_cos();
            let adx = ndx * co - ndy * so;
            let ady = ndx * so + ndy * co;
            for step in 1..=fwda_steps {
                let fx = x as f32 + adx * step as f32;
                let fy = y as f32 + ady * step as f32;
                let cy = fy.round() as i32;
                if cy < 0 || cy >= h as i32 { break; }
                let ci = buf.idx(buf.wrap_x(fx.round() as i32), cy as u32);
                if buf.terrain[ci] == 0 { break; } // hit ocean again → packet ends
                // Latitude-dependent decay (hot tropics dry out faster inland).
                let t_lat = buf.latitude(cy as u32).abs();
                let trop_blend = if t_lat < 15.0 { 1.0 }
                    else if t_lat < 30.0 { (30.0 - t_lat) / 15.0 } else { 0.0 };
                let decay = decay_base + (decay_trop - decay_base) * trop_blend;
                m *= decay;
                // Splat the packet onto the 2×2 block of cells surrounding the
                // float position instead of a single rounded cell. A rounded
                // single-cell deposit skips cells along a diagonal heading,
                // leaving the gaps that read as the diagonal "zebra"; splatting to
                // all four neighbours fills them so adjacent rays merge smoothly.
                let x0 = fx.floor() as i32;
                let y0 = fy.floor() as i32;
                for &(ox, oy) in &[(0i32, 0i32), (1, 0), (0, 1), (1, 1)] {
                    let ny = y0 + oy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let ti = buf.idx(buf.wrap_x(x0 + ox), ny as u32);
                    if buf.terrain[ti] == 1 && m > moisture_field[ti] {
                        moisture_field[ti] = m;
                    }
                }
            }
        }
    }

    // ── Pass 2: apply moisture + climatic adjustments per land cell ──────────
    let mut precip = vec![0.0f32; n];
    for y in 0..h {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { precip[idx] = 0.0; continue; }

            let mut moisture = moisture_field[idx].max(MOISTURE_FLOOR);

            // Orographic uplift / rain shadow (captured so the jet-exit dump can
            // be amplified where the decelerating jet slams into windward relief).
            let oro = orographic_multiplier(buf, x, y, buf.wind_vx[idx], buf.wind_vy[idx]);
            moisture *= oro;

            // Low-level-jet entrance dries the coast (accelerating flow sweeps
            // moisture away); the exit dumps it (evaluated below as an additive
            // convergence bonus). Applied here to the moisture before drying terms.
            let (jet_dry, jet_wet) = jet_effect(buf, x, y);
            moisture *= jet_dry;

            // Coastal drying. Cold-current "fog deserts" (Atacama / Namib / coastal
            // Peru) are a SUBTROPICAL phenomenon: a cold current under the sinking
            // subtropical high. Above ~38° a cold current instead gives a cool,
            // WET maritime coast (Pacific NW, Patagonia), so fade the drying out
            // between 30° and 40° rather than parching high-latitude coasts.
            if cold_coast[idx] {
                let subtrop = ((40.0 - abs_lat) / 10.0).clamp(0.0, 1.0); // 1 ≤30°, 0 ≥40°
                let dry = COLD_COAST_DRYING + (1.0 - COLD_COAST_DRYING) * (1.0 - subtrop);
                moisture *= dry;
            } else if buf.distance_to_ocean[idx] < 0.02 {
                // Upwelling-adjacent coasts (cool shelf water) dry a little.
                let shelf_adj = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { return false; }
                    let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
                    buf.terrain[ni] == 0 && buf.is_shelf[ni] == 1 && buf.current_type[ni] == 2
                });
                if shelf_adj { moisture *= UPWELLING_DRYING; }
            }

            // Hyper-arid coast of an enclosed subtropical sea (Arabian / Sudanese
            // Red Sea coast): trapped surface humidity never becomes rain.
            if enclosed_coast[idx] {
                moisture *= ENCLOSED_COAST_DRYING;
            }

            // ── Convergence-rainfall gate ────────────────────────────────────
            // The ITCZ and summer-monsoon terms below are ADDITIVE convergence
            // rainfall. Convergence can only wring out moisture the low-level flow
            // actually delivers, and only where the air is RISING — not where it is
            // subsiding or being swept away. Previously these bonuses bypassed every
            // local drying term, so the flat ITCZ (up to 1200 mm/yr) fell on
            // hyper-arid coasts and interiors exactly as hard as on the wet Congo —
            // turning the Somali/Arabian coast and the Saharan/Arabian interior into
            // rainforest. Gate them by three physical brakes:
            //   • advected-moisture availability — no imported vapour → no rain, so a
            //     shifted-ITCZ / summer-heat-low excursion over the Sahara/Arabia
            //     interior stays a DRY thermal trough, not the wet oceanic ITCZ;
            //   • the descending branch of the Hadley cell (subsidence caps deep
            //     convection across ~20–32°, Arabia's latitude);
            //   • the same coastal suppressors already applied to the advected base —
            //     jet-entrance divergence (the Somali jet sweeping moisture past the
            //     coast), cold-current/upwelling coasts, and enclosed-sea coasts.
            // The wet convergent monsoon regions (India, the Sahel, SE Asia, the
            // Congo/Amazon) keep high availability, rising flow and no cold coast, so
            // their gate stays ≈1 and they are unaffected.
            let avail = ((moisture_field[idx] - MOISTURE_FLOOR) / 0.80).clamp(0.0, 1.0);
            let conv_avail = 0.30 + 0.70 * avail;
            let mut conv_suppress = jet_dry;
            if cold_coast[idx] { conv_suppress *= 0.5; }
            if enclosed_coast[idx] { conv_suppress *= ENCLOSED_COAST_DRYING; }
            let itcz_gate = conv_avail * conv_suppress * (1.0 - 0.6 * hadley_inversion(abs_lat));

            let mut p = moisture * BASE_PRECIPITATION;

            // Continental-shifted ITCZ (gated — see above).
            p += itcz_bonus_shifted(lat, itcz_shift) * itcz_gate;
            // Subtropical high.
            p *= 1.0 - subtropical_penalty(abs_lat);
            // Monsoon: multiplicative interior penetration + a large ADDITIVE
            // coastal-belt load (the additive term is what actually wets the
            // continental subtropics — India etc. — that the advection base misses).
            p *= monsoon_multiplier(abs_lat, buf.distance_to_ocean[idx], land_frac_tropical);
            // Additive monsoon — GATED to genuine onshore-flow regions AND damped by
            // the local convection suppressors, so a jet-entrance / upwelling coast
            // (the Somali/Arabian side of the monsoon) gets no dump even where the
            // onshore geometry alone would fire it. India's jet-EXIT coast keeps
            // conv_suppress≈1 and stays wet.
            if (5.0..=45.0).contains(&abs_lat) {
                p += monsoon_bonus(abs_lat, buf.distance_to_ocean[idx], land_frac_tropical)
                    * monsoon_onshore(buf, x, y, &sea_suppress)
                    * conv_suppress;
            }
            // Frontal storm tracks.
            let near_ocean = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 { return false; }
                buf.terrain[buf.idx(buf.wrap_x(x as i32 + dx), ny as u32)] == 0
            });
            p += frontal_bonus(abs_lat, near_ocean);

            // Jet-exit convergence dump (the monsoon terminus), amplified where the
            // decelerating jet is forced up windward relief (the Western Ghats).
            if jet_wet > 0.0 {
                p += jet_wet * (0.6 + 0.4 * oro.min(2.5));
            }

            // Fine-grained spatial texture (±18%) so isobands aren't flat slabs.
            p *= 1.0 + 0.18 * fbm2(x as f32 / 6.5, y as f32 / 6.5, 28411);

            precip[idx] = p.max(0.0);
        }
    }

    // Multi-pass isotropic (8-neighbour) blur over land. The moisture packets
    // are advected along the wind heading and rounded to integer cells, which
    // aliases into regular diagonal rays — the "zebra". A single smoothing pass
    // left those stripes; several passes of an isotropic average dissolve the
    // directional banding while keeping the large-scale wet-coast / dry-interior
    // gradient and orographic contrasts.
    // Raised 12→18 to further dissolve the residual diagonal precip banding that
    // was reading through as stepped climate stripes.
    const BLUR_PASSES: u32 = 18;
    let mut a = precip;
    let mut b = vec![0.0f32; n];
    for _ in 0..BLUR_PASSES {
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                if buf.terrain[idx] != 1 { b[idx] = 0.0; continue; }
                // Centre weighted 2×, neighbours 1× (a light Gaussian).
                let mut sum = a[idx] * 2.0;
                let mut cnt = 2.0f32;
                for &(dx, dy) in &[
                    (-1i32, 0), (1, 0), (0, -1i32), (0, 1),
                    (-1, -1), (1, -1), (-1, 1), (1, 1),
                ] {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
                    if buf.terrain[ni] != 1 { continue; }
                    sum += a[ni];
                    cnt += 1.0;
                }
                b[idx] = sum / cnt;
            }
        }
        std::mem::swap(&mut a, &mut b);
    }

    for i in 0..n {
        buf.precipitation[i] = if buf.terrain[i] == 1 {
            a[i].clamp(0.0, 4000.0)
        } else {
            0.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
    use crate::db::schema;
    use rusqlite::Connection;

    /// The core jet contract: an ACCELERATING cell (jet entrance) dries; a
    /// DECELERATING cell (jet exit / terminus) gets a convergence rain bonus.
    #[test]
    fn jet_entrance_dries_exit_wets() {
        let (w, h) = (40u32, 12u32);
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap();
        // Uniform eastward flow. Speed ramps UP over x=0..20 (accelerating
        // entrance) then DOWN over x=20..40 (decelerating exit).
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                buf.wind_vx[i] = 1.0;
                buf.wind_vy[i] = 0.0;
                let s = if x <= 20 { 4.0 + x as f32 } else { 24.0 - (x as f32 - 20.0) };
                buf.wind_speed[i] = s.max(0.0);
            }
        }
        // Entrance (x=10, accelerating): drying multiplier < 1, no wet bonus.
        let (dry_e, wet_e) = jet_effect(&buf, 10, 6);
        assert!(dry_e < 1.0, "entrance should dry the coast: {dry_e}");
        assert_eq!(wet_e, 0.0);
        // Exit (x=30, decelerating): full moisture kept, positive convergence dump.
        let (dry_x, wet_x) = jet_effect(&buf, 30, 6);
        assert_eq!(dry_x, 1.0);
        assert!(wet_x > 0.0, "exit terminus should add rainfall: {wet_x}");
    }

    /// Regression for the "Arabia / Somalia reads as rainforest" bug. A strongly
    /// NH-heavy world drives a large poleward ITCZ shift; before the convergence
    /// gate, the flat additive ITCZ (up to 1200 mm/yr) then flooded the DRY-advection
    /// NH subtropical interior (~20°, the Arabian latitude) just as hard as a wet
    /// equatorial coast, pushing it to tropical-forest wetness. The gate ties that
    /// additive ITCZ to advected-moisture availability and the descending Hadley
    /// high, so the arid subtropical interior stays dry while a moist equatorial
    /// coast stays wet.
    #[test]
    fn shifted_itcz_does_not_flood_dry_subtropics() {
        let (w, h) = (24u32, 180u32);
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap();

        // NH-heavy geography: everything from a little south of the equator up to the
        // subtropics is a single wide continent; the SH is open ocean. This makes
        // compute_itcz_shift return a strong positive (northward) shift — the exact
        // condition that used to push the wet ITCZ band deep into the subtropics.
        for y in 0..h {
            let lat = buf.latitude(y);
            for x in 0..w {
                let i = buf.idx(x, y);
                let land = lat > -4.0 && lat < 34.0;
                buf.terrain[i] = if land { 1 } else { 0 };
                buf.elevation[i] = if land { 0.05 } else { 0.0 };
                // Northward onshore flow (screen −y is north here): the SH ocean feeds
                // moisture onto the southern (equatorial) coast, which decays inland
                // toward the subtropics — the real moisture-supply gradient.
                buf.wind_vx[i] = 0.0;
                buf.wind_vy[i] = -1.0;
                // The SH source ocean carries a warm current (a strong moisture source);
                // the equatorial coast is thus well-supplied, the far interior is not.
                buf.current_type[i] = if land { 0 } else { 1 };
                buf.wind_speed[i] = 4.0;
            }
        }
        // distance_to_ocean: cheap approximation — degrees of latitude north of the
        // southern coast, normalised. Enough for the monsoon/interior terms.
        for y in 0..h {
            let lat = buf.latitude(y);
            for x in 0..w {
                let i = buf.idx(x, y);
                buf.distance_to_ocean[i] = if buf.terrain[i] == 1 {
                    ((lat + 4.0) / 90.0).clamp(0.0, 1.0)
                } else { 0.0 };
            }
        }

        compute_precipitation(&mut buf);

        // Sample a mid-continent column (far from the E/W wrap edges is irrelevant
        // here since the whole band is land). Find rows nearest 3° and 22°.
        let row_at = |target: f32| -> u32 {
            (0..h).min_by(|&a, &b| {
                (buf.latitude(a) - target).abs()
                    .partial_cmp(&(buf.latitude(b) - target).abs()).unwrap()
            }).unwrap()
        };
        let eq_row = row_at(3.0);
        let sub_row = row_at(22.0);
        let x = w / 2;
        let eq_p = buf.precipitation[buf.idx(x, eq_row)];
        let sub_p = buf.precipitation[buf.idx(x, sub_row)];

        // The dry subtropical interior (Arabia's latitude) must be arid — nowhere
        // near tropical-forest wetness — even though the ITCZ has shifted onto it.
        assert!(sub_p < 700.0, "dry subtropical interior should stay arid, got {sub_p} mm");
        // And it must be markedly drier than the moist equatorial belt (which the
        // gate leaves wet).
        assert!(eq_p > sub_p * 1.5, "equator {eq_p} should be far wetter than subtropics {sub_p}");
    }
}
