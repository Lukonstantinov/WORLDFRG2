use super::seasonal;
use crate::sim::world_buffer::WorldBuffer;

// â”€â”€ Local value-noise (keeps moisture flow from following dead-straight rays) â”€â”€
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

// â”€â”€ Constants (ported from WF1 precipitation.ts / itcz.ts / orographic-lift.ts) â”€â”€
const BASE_PRECIPITATION: f32 = 800.0;

// Per-packet heading fan-out (radians) so the discrete wind belts don't lay
// down identical straight diagonal rays.
const MEANDER_TURN: f32 = 0.85;

// Earth's circumference (km) â€” used to translate moisture-decay distances
// expressed in real km into a per-cell decay that is independent of grid
// resolution. The previous fixed FWDA_STEPS=60 / decay 0.935-0.960 were tuned
// for a tiny grid; on the 3600-wide default a moisture packet died after ~60
// cells (~660 km) so every continental interior beyond that collapsed to the
// 0.04 floor â†’ instant desert. The Amazon, the US/Asian interiors etc. all
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
const COLD_COAST_DRYING: f32 = 0.35; // cold-current coast â†’ fog desert
const UPWELLING_DRYING: f32 = 0.60;  // upwelling-cooled coast â†’ extra drying

// â”€â”€ Enclosed subtropical sea suppression (Red Sea / Persian Gulf effect) â”€â”€
// Warm but narrow seas sitting under the Hadley-cell subtropical high evaporate
// enormous amounts of water yet produce almost no rain. Three barriers combine:
//   1. Sinking subtropical-high air (the descending Hadley branch near ~30Â°)
//      compresses and warms, capping any convection.
//   2. The surrounding deserts blow a hot, dry inversion lid over the marine
//      boundary layer, trapping humidity at the surface.
//   3. The basin is narrow â€” air crosses too fast to load the moisture needed to
//      break the high.
// We model this by suppressing the moisture such basins *emit* (and drying their
// immediate coasts) so Red-Sea / Persian-Gulf-style seas stay hyper-arid and
// their desert coasts (the Arabian / Sudanese Red Sea coast) don't sprout a wet
// belt the real world never has.
const ENCLOSED_SUPPRESS_STRENGTH: f32 = 0.85; // max fraction of emitted moisture removed
const ENCLOSED_NARROW_KM: f32 = 1400.0;       // seas narrower than this read as "enclosed"
const ENCLOSED_COAST_DRYING: f32 = 0.45;      // extra drying on land bordering such seas

// Orographic. Reaches are expressed in KM (resolution-independent) and converted to
// cells per grid inside `orographic_multiplier`: the upslope-uplift zone is a
// relatively narrow band just windward of a range, while the rain shadow reaches
// far downwind — the Patagonian / Great-Plains lee steppe stays dry for hundreds of
// km east of the cordillera.
const MOUNTAIN_THRESHOLD: f32 = 0.19; // elevation counting as a ridge (~1700 m)
const WINDWARD_KM: f32 = 220.0;       // upslope-enhancement fetch windward of a range
const SHADOW_KM: f32 = 500.0;         // rain-shadow reach downwind of a range

// â”€â”€ Low-level-jet entrance/exit dynamics (jets.rs supplies buf.wind_speed) â”€â”€
// A jet ENTRANCE (accelerating flow) diverges at low level and sweeps moisture
// along before it can accumulate â†’ the coast dries (Somali / Arabian coast). A
// jet EXIT (decelerating flow) converges and is forced to rise â†’ torrential
// rainfall at the terminus (the Western-Ghats monsoon dump).
const JET_MIN_SPEED: f32 = 8.0;      // below this a cell isn't in a jet
const JET_ACCEL_SCALE: f32 = 6.0;    // along-flow Î”speed (m/s) that saturates the effect
const JET_ENTRANCE_DRY: f32 = 0.55;  // max fraction of moisture removed at a strong entrance
const JET_EXIT_WET_MAX: f32 = 750.0; // max convergence rainfall (mm/yr) at a strong exit

// ── Clausius–Clapeyron moisture capacity ────────────────────────────────────
// Saturation vapour pressure rises exponentially with temperature, so warm air
// holds far more water vapour than cold air. Physically this is why the deep
// tropics (Amazon, Congo, SE Asia) stay wet hundreds of km inland while cold
// continental interiors dry out fast: a warm moist air parcel can travel further
// before it rains itself out. We fold this into the moisture-advection depletion —
// the parcel loses a smaller fraction of its moisture per step in warm air.

/// Fraction (0..1) of a moisture parcel's per-step depletion that warm air resists,
/// from the Clausius–Clapeyron (Tetens) saturation curve, referenced to 0 °C so it
/// is a pure warm-air BOOST: 0 at/below freezing, rising toward CC_STRENGTH in the
/// hot tropics. (Referencing to 0 °C also means the depletion is unchanged wherever
/// the temperature field is 0 — e.g. isolated unit tests — so it only adds physics
/// in the real pipeline where temperature has been computed.)
const CC_STRENGTH: f32 = 0.5;
#[inline]
fn cc_retain_frac(t_c: f32) -> f32 {
    if t_c <= 0.0 {
        return 0.0;
    }
    let e_sat = |t: f32| 6.11 * (17.27 * t / (t + 237.3)).exp();
    let ratio = (1.0 - e_sat(0.0) / e_sat(t_c)).clamp(0.0, 1.0);
    CC_STRENGTH * ratio
}

/// Orographic precipitation multiplier for one land cell.
/// 2.5 = windward uplift, 0.15..1.0 = leeward rain shadow (graduated), 1.0 = none.
fn orographic_multiplier(buf: &WorldBuffer, x: u32, y: u32, wvx: f32, wvy: f32) -> f32 {
    let wind_len = (wvx * wvx + wvy * wvy).sqrt();
    if wind_len < 0.01 { return 1.0; }
    let ndx = wvx / wind_len;
    let ndy = wvy / wind_len;
    let h = buf.height as i32;

    // Resolution-independent reaches (km → cells for this grid).
    let km_per_cell = EARTH_CIRCUMFERENCE_KM / buf.width as f32;
    let shadow_steps = ((SHADOW_KM / km_per_cell).round() as i32).max(6);
    let windward_steps = ((WINDWARD_KM / km_per_cell).round() as i32).max(4);

    // Leeward (rain shadow): walk upwind for a recently crossed mountain. The shadow
    // is deepest right behind the crest and recovers with distance downwind.
    for step in 1..=shadow_steps {
        let bx = (x as f32 - ndx * step as f32).round() as i32;
        let by = (y as f32 - ndy * step as f32).round() as i32;
        if by < 0 || by >= h { break; }
        let bi = buf.idx(buf.wrap_x(bx), by as u32);
        if buf.terrain[bi] == 0 { break; } // open ocean upstream â†’ no shadow
        if buf.elevation[bi] > MOUNTAIN_THRESHOLD {
            return 0.15 + 0.85 * (((step - 1) as f32 / shadow_steps as f32).sqrt());
        }
    }

    // Windward: walk downwind for an approaching mountain.
    for step in 1..=windward_steps {
        let fx = (x as f32 + ndx * step as f32).round() as i32;
        let fy = (y as f32 + ndy * step as f32).round() as i32;
        if fy < 0 || fy >= h { break; }
        let fi = buf.idx(buf.wrap_x(fx), fy as u32);
        if buf.terrain[fi] == 0 { break; } // ocean downwind â†’ no windward uplift
        if buf.elevation[fi] > MOUNTAIN_THRESHOLD { return 2.5; }
    }

    1.0
}

/// Per-column ITCZ latitude shift from local land distribution.
///
/// The ITCZ position varies strongly by longitude: it migrates to 15-20 deg N
/// over West Africa / the Sahel (drawn by the Saharan heat low) but stays
/// near 5-8 deg N over the Indian Ocean east of Africa, and near 5-10 deg N
/// over the open Pacific. A single global shift treats every longitude the
/// same and over-rains eastern tropical Africa relative to western Africa.
///
/// Here we compute a per-column raw shift (same NH/SH land-fraction formula
/// applied to each x-column) then smooth over a +/-30 deg longitude window so
/// continental-scale patterns emerge without single-cell noise. Each cell in
/// season_precip uses its own column's shift.
fn compute_itcz_shift_zonal(buf: &WorldBuffer) -> Vec<f32> {
    let w = buf.width;
    let h = buf.height;
    // Smoothing window: approx 30 deg longitude at any grid size (min 8 cells).
    let radius = ((w as f32 * 30.0 / 360.0) as i32).max(8);

    // Pass 1: raw per-column shift.
    let mut raw = vec![0.0f32; w as usize];
    for x in 0..w {
        let (mut nh_l, mut sh_l, mut nh_t, mut sh_t) = (0.0f32, 0.0, 0.0, 0.0);
        for y in 0..h {
            let lat = buf.latitude(y);
            let abs_lat = lat.abs();
            if abs_lat > 30.0 { continue; }
            let wt = 1.0 - abs_lat / 30.0;
            let land = buf.terrain[buf.idx(x, y)] == 1;
            if lat >= 0.0 { nh_t += wt; if land { nh_l += wt; } }
            else          { sh_t += wt; if land { sh_l += wt; } }
        }
        if nh_t > 0.0 && sh_t > 0.0 {
            raw[x as usize] = ((nh_l / nh_t - sh_l / sh_t) * 20.0).clamp(-12.0, 12.0);
        }
    }

    // Pass 2: triangular-weighted smoothing over +/-radius columns (x wraps).
    let mut out = vec![0.0f32; w as usize];
    for x in 0..w {
        let (mut sum, mut wsum) = (0.0f32, 0.0f32);
        for d in -radius..=radius {
            let nx = buf.wrap_x(x as i32 + d);
            let wt = 1.0 - d.unsigned_abs() as f32 / (radius + 1) as f32;
            sum += raw[nx as usize] * wt;
            wsum += wt;
        }
        out[x as usize] = sum / wsum;
    }
    out
}

/// Shifted ITCZ precipitation bonus (mm/yr) at a latitude.
///
/// Widened taper (was full â‰¤5Â°, zero by 12Â°). The ITCZ migrates seasonally
/// across a broad tropical band, so the annual-mean wet zone reaches well past
/// Â±12Â° in continental monsoon regions â€” that band is what keeps tropical
/// rainforest/savanna interiors (Amazon, Congo, the Sahel margin) wet instead
/// of collapsing to desert once they sit beyond pure trade-wind advection.
fn itcz_bonus_shifted(lat: f32, shift: f32) -> f32 {
    let eff = (lat - shift).abs();
    if eff <= 6.0 { 1200.0 }
    else if eff <= 18.0 { 1200.0 * (1.0 - (eff - 6.0) / 12.0) }
    else { 0.0 }
}

/// Strength (0..1) of the subtropical-high sinking-air inversion by latitude.
/// Peaks across the descending branch of the Hadley cell (~20-32Â°), tapering to
/// zero by 12Â° (where the ITCZ takes over) and 38Â° (where the mid-latitude storm
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
/// span). Wide open ocean â†’ 0 (normal moisture); a narrow strait under the
/// subtropical high â†’ near 1 (heavily suppressed). Land cells stay 0.
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

/// Subtropical-high fractional drying (0..0.82), peak near 26°.
/// The old ±10° window around 28° gave <14% suppression at 20°N (Yemen/Oman)
/// — nowhere near enough to produce desert there. The real subtropical high
/// belt extends from 13° to 42°, centred on ~26°, with the Arabian/Saharan
/// desert corridor running 15-32°N. Widened accordingly; max raised to 0.82.
fn subtropical_penalty(abs_lat: f32) -> f32 {
    if abs_lat < 13.0 || abs_lat > 42.0 { return 0.0; }
    let frac = if abs_lat <= 26.0 {
        (abs_lat - 13.0) / 13.0  // 0→1 across 13-26°
    } else {
        (42.0 - abs_lat) / 16.0  // 1→0 across 26-42°
    };
    0.82 * frac
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
/// zero (continental subtropics in the easterly trades â€” e.g. the Indian
/// subcontinent) it left the land as desert. The summer monsoon instead draws a
/// large, fresh moisture load off the warm seas onto the land, so it is modelled
/// as an ADDITIVE term over the 5â€“45Â° coastal belt: strongest on/near the coast,
/// reaching inland on big continents. This wets India, SE Asia, E China and the
/// Japan / E-Asian seaboard; the dry-winter seasonal split (koppen.rs) then turns
/// these into proper monsoon climates (Aw/Am, Cwa/Cwb, Dwa/Dwb).
fn monsoon_bonus(abs_lat: f32, dist_ocean: f32, land_frac_tropical: f32) -> f32 {
    if abs_lat < 5.0 || abs_lat > 45.0 { return 0.0; }
    // Latitude weight: full across the core monsoon belt (12â€“28Â°), ramping in from
    // 5Â° and fading out by 45Â° (so the E-Asian / Japan monsoon still reaches).
    let lat_w = if abs_lat < 12.0 { (abs_lat - 5.0) / 7.0 }
        else if abs_lat <= 28.0 { 1.0 }
        else { (1.0 - (abs_lat - 28.0) / 17.0).max(0.0) };
    // Proximity to the moisture source (the warm sea): strong near the coast,
    // penetrating well inland on large continents but fading in deep interiors.
    let prox = if dist_ocean < 0.03 { 0.75 }
        else if dist_ocean < 0.28 { 1.0 }
        else { (1.0 - (dist_ocean - 0.28) * 1.1).max(0.0) };
    // Continentality: a big landmass drives a stronger monsoon (landâ€“sea thermal
    // contrast); small islands get only a mild boost.
    let cont = (0.45 + land_frac_tropical * 2.4).min(1.25);
    const MONSOON_BONUS_MAX: f32 = 850.0;
    MONSOON_BONUS_MAX * lat_w * prox * cont
}

/// Onshore-monsoon GATE (0..1). A monsoon only fires where summer flow blows OFF a
/// warm sea ONTO the land â€” i.e. there is a warm ocean on the cell's EQUATORWARD
/// and/or EASTERN side within reach (the Indian/SE-Asian/E-Asian geometry). This
/// excludes subtropical-high deserts (the Sahara, Arabia, the Atacama/Namib/W-Australia
/// coasts) whose moisture source is blocked â€” so the monsoon term no longer turns
/// dry land green. A cold current offshore (cold-upwelling desert coast) counts for
/// little. Cheap: only the 5â€“45Â° belt calls it, scanning a few short rays.
fn monsoon_onshore(buf: &WorldBuffer, x: u32, y: u32, sea_suppress: &[f32]) -> f32 {
    let lat = buf.latitude(y);
    let h = buf.height as i32;
    let eqdir = if lat >= 0.0 { 1i32 } else { -1 }; // equatorward = toward the equator
    // Resolution-aware moisture fetch: a summer monsoon draws on a warm sea up to
    // ~MONSOON_FETCH_KM away. The old fixed 28-cell reach was only ~310 km on a
    // 3600-wide grid, so a continental interior (the Indian Deccan, the N-China
    // plain) never "saw" the ocean and stayed desert. ~800 km reaches India's
    // interior while still leaving the Sahara dry â€” its nearest warm equatorward
    // ocean (the Gulf of Guinea) is >1500 km away past the Sahel, and its west
    // coast is the cold Canary upwelling (discounted below).
    const MONSOON_FETCH_KM: f32 = 800.0;
    let km_per_cell = EARTH_CIRCUMFERENCE_KM / buf.width as f32;
    let range = ((MONSOON_FETCH_KM / km_per_cell) as i32).max(12);
    let mut best = 0.0f32;
    // Equatorward (S), equatorward-east (SE), equatorward-west (SW â€” the Indian /
    // Western-Ghats summer monsoon blows from the south-west), and due-east rays.
    for &(dx, dy) in &[(0i32, eqdir), (1, eqdir), (-1, eqdir), (1, 0)] {
        for s in 1..=range {
            let nx = buf.wrap_x(x as i32 + dx * s);
            let ny = y as i32 + dy * s;
            if ny < 0 || ny >= h { break; }
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] == 0 {
                // Enclosed/suppressed seas (Red Sea, Persian Gulf, Gulf of Aden) sit
                // under the Hadley inversion and block the monsoon pathway — STOP the
                // ray here (a `continue` wrongly let the ray find open ocean beyond
                // the enclosed sea and made Arabia/Yemen wet despite the drying gate).
                if sea_suppress[ni] > 0.4 { break; }
                // Cold-current coasts (Somali/Canary/Humboldt) produce coastal deserts,
                // not monsoon — they supply zero monsoon moisture.
                let warm = if buf.current_type[ni] == 2 { 0.0 } else { 1.0 };
                best = best.max((1.0 - s as f32 / range as f32) * warm);
                break; // first usable sea cell along the ray decides it
            }
        }
    }
    best
}

/// Frontal precipitation bonus (mm/yr) at mid-latitude storm tracks (30-66Â°).
///
/// Extratropical cyclones deliver year-round precipitation across the whole
/// mid-latitude belt regardless of the prevailing-wind direction, which is what
/// keeps east-coast and interior temperate zones (eastern US, Europe, NE Asia)
/// humid even though the westerlies put the ocean *downwind* of them. The pure
/// downwind-advection model can't supply that moisture, so the storm-track term
/// is the main thing standing between a realistic temperate belt and a runaway
/// band of steppe. Broadened (was 35-60Â°) and strengthened accordingly.
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
    // No jet field loaded (e.g. an old save re-run without the jet step) â†’ no-op.
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
    let fwda_steps = ((efold_mid * 5.0) as i32).max(60);

    // Warm-but-narrow subtropical seas (Red Sea / Persian Gulf): per-ocean-cell
    // moisture-emission suppression under the Hadley subtropical high.
    let sea_suppress = compute_enclosed_suppression(buf, km_per_cell);

    // Per-column ITCZ shift (zonal variation): each longitude gets its own
    // NH/SH land-fraction shift, smoothed over ~30 deg of longitude.
    // The seasonal migration (+/-MIGRATE) is kept separate and added per-cell
    // inside season_precip so the column shift is reused across both seasons.
    let itcz_col = compute_itcz_shift_zonal(buf);

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

    // â”€â”€ Two seasonal wind states (thermal landâ€“sea low/high) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Boreal summer (sun_sign +1 â‰ˆ July: NH summer / SH winter) and boreal winter
    // (âˆ’1 â‰ˆ January). Each derives its own onshore/offshore monsoon flow, its own
    // moisture advection and its own ITCZ position; summing the two half-year loads
    // gives an annual field whose seasonality is EMERGENT rather than faked.
    let hsn = seasonal::compute_seasonal_wind(buf, 1.0);  // high-sun-north
    let hss = seasonal::compute_seasonal_wind(buf, -1.0); // high-sun-south
    let migrate = seasonal::ITCZ_SEASONAL_MIGRATE;

    let ctx = SeasonCtx {
        sea_suppress: &sea_suppress,
        cold_coast: &cold_coast,
        enclosed_coast: &enclosed_coast,
        land_frac_tropical,
        decay_base,
        decay_trop,
        fwda_steps,
    };
    // Half-year precipitation for each insolation state (pre-blur, land cells only).
    let p_hsn = season_precip(buf, &hsn.vx, &hsn.vy, 1.0, &itcz_col,  migrate, &ctx);
    let p_hss = season_precip(buf, &hss.vx, &hss.vy, -1.0, &itcz_col, -migrate, &ctx);

    // Blur each seasonal field (dissolves the advection "zebra") before combining,
    // so the derived summer fraction is as smooth as the annual total.
    let p_hsn = blur_land(buf, p_hsn, 18);
    let p_hss = blur_land(buf, p_hss, 18);

    // â”€â”€ Combine â†’ annual precip + summer fraction â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    for y in 0..h {
        let lat = buf.latitude(y);
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 1 {
                buf.precipitation[i] = 0.0;
                if !buf.precip_summer_frac.is_empty() { buf.precip_summer_frac[i] = 0; }
                continue;
            }
            // A cell's summer = its own hemisphere's high-sun state.
            let (summer, winter) = if lat >= 0.0 {
                (p_hsn[i], p_hss[i])
            } else {
                (p_hss[i], p_hsn[i])
            };
            let annual = (summer + winter).clamp(0.0, 4000.0);
            buf.precipitation[i] = annual;
            if !buf.precip_summer_frac.is_empty() {
                let frac = if annual > 1.0 { (summer / (summer + winter)).clamp(0.0, 1.0) } else { 0.5 };
                buf.precip_summer_frac[i] = (frac * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Shared read-only context for a seasonal precipitation pass.
struct SeasonCtx<'a> {
    sea_suppress: &'a [f32],
    cold_coast: &'a [bool],
    enclosed_coast: &'a [bool],
    land_frac_tropical: f32,
    decay_base: f32,
    decay_trop: f32,
    fwda_steps: i32,
}

/// Half-year precipitation (mm) for one insolation state, using that season's wind
/// field. Mirrors the annual model but: (1) advects on the seasonal winds; (2) scales
/// every term by 0.5 (a half-year load, so the two seasons sum to a full year); (3)
/// fires the additive monsoon and weights the subtropical-high / frontal terms by the
/// LOCAL season, so the wet-summer / dry-winter (and dry-summer Mediterranean) splits
/// emerge instead of being imposed. Returns a pre-blur land field (ocean = 0).
fn season_precip(
    buf: &WorldBuffer,
    wind_vx: &[f32],
    wind_vy: &[f32],
    sun_sign: f32,
    itcz_col: &[f32],  // per-column base ITCZ shift (zonal variation)
    migrate: f32,       // seasonal ITCZ migration (+ve = NH summer)
    ctx: &SeasonCtx,
) -> Vec<f32> {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Half-year scale: the two seasons sum to one year, so each carries half the
    // annual baseline. Totals are allowed to shift where the seasonal advection
    // disagrees with the annual mean (monsoon geography), but stay in a sane band.
    const SEASON_SCALE: f32 = 0.5;

    // â”€â”€ Pass 1: forward moisture advection on the seasonal wind â†’ MAX field â”€â”€
    let mut moisture_field = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 { continue; } // only ocean cells emit moisture

            let wvx = wind_vx[idx];
            let wvy = wind_vy[idx];
            let wl = (wvx * wvx + wvy * wvy).sqrt();
            if wl < 0.01 { continue; }
            let ndx = wvx / wl;
            let ndy = wvy / wl;

            let init_moisture = match buf.current_type[idx] {
                1 => 1.7, // warm current â†’ much more moisture carried inland
                2 => 0.4, // cold current â†’ far less (coastal desert downwind)
                _ => 1.0,
            };
            let init_moisture = init_moisture * (1.0 - ENCLOSED_SUPPRESS_STRENGTH * ctx.sea_suppress[idx]);

            let mut m = init_moisture;
            // Per-emitter fixed angular offset decorrelates the discrete wind belts'
            // parallel rays (the diagonal "zebra") without curving packets back to sea.
            let off = fbm2(x as f32 / 3.5, y as f32 / 3.5, 5237) * MEANDER_TURN;
            let (so, co) = off.sin_cos();
            let adx = ndx * co - ndy * so;
            let ady = ndx * so + ndy * co;
            for step in 1..=ctx.fwda_steps {
                let fx = x as f32 + adx * step as f32;
                let fy = y as f32 + ady * step as f32;
                let cy = fy.round() as i32;
                if cy < 0 || cy >= h as i32 { break; }
                let ci = buf.idx(buf.wrap_x(fx.round() as i32), cy as u32);
                if buf.terrain[ci] == 0 { break; } // hit ocean again â†’ packet ends
                let t_lat = buf.latitude(cy as u32).abs();
                let trop_blend = if t_lat < 15.0 { 1.0 }
                    else if t_lat < 30.0 { (30.0 - t_lat) / 15.0 } else { 0.0 };
                let decay = ctx.decay_base + (ctx.decay_trop - ctx.decay_base) * trop_blend;
                // Clausius–Clapeyron: warm air holds more moisture, so the parcel
                // depletes MORE SLOWLY over warm land (decay pushed toward 1) — the
                // conserved-moisture reason tropical rainforest reaches far inland.
                // Zero effect at/below 0 °C, so the depletion is unchanged in the cold.
                let cc = cc_retain_frac(buf.temperature[ci]);
                let decay = decay + (1.0 - decay) * cc;
                m *= decay;
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

    // ── Pass 1b: Hadley subsidence moisture sink ────────────────────────────────
    // The descending Hadley branch compresses and heats arriving air, dramatically
    // lowering relative humidity so that even parcels carrying significant moisture
    // produce little to no precipitation. This is the physical reason Arabia and the
    // Sahara are desert despite being hit by the summer monsoon surface flow: the
    // wind IS there (streamlines show SW flow), but the subsidence lid kills
    // convection and heats the parcel above its dew point.
    // Applied directly to moisture_field so that both the base (p=moisture*800) AND
    // the ITCZ/frontal terms (which also scale off moisture) see the reduction.
    // Only the subtropical belt (subtropical_penalty > 0, i.e., 13-42°N/S) is touched;
    // tropical land (ITCZ belt, West Africa, Amazon) and high-lat land are unaffected.
    for y in 0..h {
        let sub_sink = subtropical_penalty(buf.latitude(y).abs());
        if sub_sink <= 0.0 { continue; }
        // In local summer the Hadley high is strongest (retreats slightly poleward
        // but the land heats MORE, increasing the thermal inversion lid).
        // Apply in both seasons to keep the annual total realistic; summer is
        // handled more strongly than winter by using a mild seasonal weight.
        let sink_frac = 0.80 * sub_sink; // up to 0.80×0.82 = 0.66 reduction at peak
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            // Compress the excess above the floor (keeps a minimum MOISTURE_FLOOR).
            let excess = (moisture_field[idx] - MOISTURE_FLOOR).max(0.0);
            moisture_field[idx] = MOISTURE_FLOOR + excess * (1.0 - sink_frac);
        }
    }

    // â”€â”€ Pass 2: climatic adjustments per land cell â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let mut precip = vec![0.0f32; n];
    for y in 0..h {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();
        // Is THIS insolation state the cell's local summer?
        let local_summer = sun_sign * if lat >= 0.0 { 1.0 } else { -1.0 } > 0.0;
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { precip[idx] = 0.0; continue; }

            // Per-column ITCZ shift: the column's land-asymmetry base + seasonal migration.
            let itcz_shift = itcz_col[x as usize] + migrate;

            // SEASON_SCALE applied to the moisture BASE only: the two seasons sum
            // to the annual base (0.5+0.5=1). ITCZ/monsoon bonuses are NOT
            // scaled here because they only fire in one season anyway - their
            // annual sum already equals the old model's full-year value.
            let mut moisture = moisture_field[idx].max(MOISTURE_FLOOR) * SEASON_SCALE;

            // Orographic uplift / rain shadow on the SEASONAL wind (windward flips
            // between monsoon seasons â€” the Western-Ghats summer dump).
            let oro = orographic_multiplier(buf, x, y, wind_vx[idx], wind_vy[idx]);
            moisture *= oro;

            // Low-level jet: entrance dries, exit dumps. Uses the annual jet field
            // (the Somali jet is a boreal-summer feature; keeping it both seasons
            // slightly over-dries the already-dry winter coast â€” acceptable).
            let (jet_dry, jet_wet) = jet_effect(buf, x, y);
            moisture *= jet_dry;

            // Cold-current fog-desert / upwelling coastal drying.
            if ctx.cold_coast[idx] {
                let subtrop = ((40.0 - abs_lat) / 10.0).clamp(0.0, 1.0);
                let dry = COLD_COAST_DRYING + (1.0 - COLD_COAST_DRYING) * (1.0 - subtrop);
                moisture *= dry;
            } else if buf.distance_to_ocean[idx] < 0.02 {
                let shelf_adj = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { return false; }
                    let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
                    buf.terrain[ni] == 0 && buf.is_shelf[ni] == 1 && buf.current_type[ni] == 2
                });
                if shelf_adj { moisture *= UPWELLING_DRYING; }
            }
            if ctx.enclosed_coast[idx] {
                moisture *= ENCLOSED_COAST_DRYING;
            }

            // â”€â”€ Convergence-rainfall gate (see the annual-model note in git history):
            // the additive ITCZ/monsoon terms can only rain out moisture the flow
            // delivers, only where air rises, and are capped hard under the Hadley
            // subtropical high â€” so a shifted-ITCZ excursion over the Sahara/Arabia
            // interior stays a dry thermal trough. Hadley coefficient raised 0.6â†’0.75
            // to push the subtropical interior toward true (but not bone-dry) desert.
            let avail = ((moisture_field[idx] - MOISTURE_FLOOR) / 0.80).clamp(0.0, 1.0);
            // Tropical ITCZ conv_avail floor: raised to 0.65 only for cells that
            // have warm (not cold, not enclosed) ocean within ~300 km to their
            // EQUATORWARD side. That is where local ocean evaporation directly
            // sustains deep convection (Nigeria coast / Gulf of Guinea, Congo
            // coast, Amazon mouth). Interior land cells and coasts whose
            // equatorward direction hits land before open ocean â€” Somalia, the
            // Red Sea coast â€” keep the standard 0.30 floor and rely on advected
            // moisture + monsoon_bonus to produce rainfall. This prevents the
            // Horn of Africa from becoming wet (its equatorward scan hits Kenya
            // land for >1000 km before reaching ocean).
            let conv_floor = if abs_lat < 15.0 {
                let ey = if lat >= 0.0 { 1i32 } else { -1i32 };
                let km_pc = EARTH_CIRCUMFERENCE_KM / buf.width as f32;
                let eq_range = ((300.0 / km_pc) as i32).max(3).min(30);
                let near_warm_eq = (1..=eq_range).any(|s| {
                    let ny = y as i32 + ey * s;
                    if ny < 0 || ny >= h as i32 { return false; }
                    let ni = buf.idx(x, ny as u32);
                    buf.terrain[ni] == 0          // ocean cell
                        && buf.current_type[ni] != 2  // not cold upwelling
                        && ctx.sea_suppress[ni] < 0.3 // not enclosed subtropical sea
                });
                if near_warm_eq { 0.65 } else { 0.30 }
            } else { 0.30 };
            let conv_avail = conv_floor + (1.0 - conv_floor) * avail;
            // FIX 1 â€” quadratic jet suppression on convective terms.
            // In a jet entrance, low-level DIVERGENCE prevents deep convection
            // non-linearly: even when moisture is present, the ascending motion
            // needed for the ITCZ and monsoon is shut off. Linear (jet_dry)
            // only removed 55% of convective rainfall at the Somali coast;
            // squaring it drops that to ~80%, which is physically correct for
            // a strong entrance (divergence inhibition is exponential).
            let mut conv_suppress = if jet_dry < 1.0 { jet_dry * jet_dry } else { 1.0 };
            if ctx.cold_coast[idx] { conv_suppress *= 0.5; }
            if ctx.enclosed_coast[idx] { conv_suppress *= ENCLOSED_COAST_DRYING; }
            // FIX 2 â€” proxy for seasonal Somali-jet upwelling.
            // The Somali Current reverses in boreal summer into one of Earth's
            // strongest upwellings (SSTs 14-18Â°C). The annual-mean ocean model
            // cannot capture this â€” it averages to warm/neutral. Where a strong
            // jet entrance (jet_dry < 0.65) sits below 20Â° latitude, simulate
            // the same moisture-removal and convective suppression that a cold
            // coast would produce.  Gated to exclude cells already flagged cold.
            // Extended to 28° — Omani/Yemeni coasts (22-26°N) sit in the same
            // Somali-jet divergence zone as the Horn; they should be equally arid.
            let has_jet_upwelling = !ctx.cold_coast[idx] && jet_dry < 0.65 && abs_lat < 28.0;
            if has_jet_upwelling {
                moisture *= COLD_COAST_DRYING;
                conv_suppress *= 0.5;
            }
            // Hadley inversion weakened in LOCAL SUMMER: the subtropical high
            // retreats poleward and the monsoon thermal trough takes over
            // (India, West Africa, SE Asia monsoon belt 15-32Â°N/S). The full
            // inversion strength only applies in local winter when the STH is
            // at its equatorward limit. Factor 0.25 in summer preserves just
            // enough subsidence to keep true subtropical deserts dry while
            // allowing the monsoon trough to fire rainfall over India etc.
            let hadley_summer_factor = if local_summer { 0.25 } else { 1.0 };
            // FIX 3 â€” extend Hadley gate into the 8-12Â° eastern dry corridor.
            // The Horn of Africa / Arabian-Sea coast at 8-12Â°N sits under
            // Hadley-like dry subsidence (the Indian Ocean subtropical high
            // expands into this band), NOT under ITCZ convergence. The standard
            // hadley_inversion() cutoff at 12Â° misses this. The extension only
            // fires where conv_floor â‰¤ 0.30 (no warm equatorial ocean to the
            // south within 300 km) â€” this targets eastern Africa/Horn without
            // touching Nigeria/West Africa, which has warm Gulf of Guinea to
            // its south and therefore conv_floor = 0.65.
            // Extended to 16° so Aden/Djibouti (12-16°N, blocked from equatorial
            // ocean by the Somali coast) are also arid, not just the Horn <12°.
            let hadley_ext = if abs_lat > 8.0 && abs_lat < 16.0 && conv_floor <= 0.30 {
                (abs_lat - 8.0) / 8.0
            } else {
                0.0
            };
            let hadley_total = (hadley_inversion(abs_lat) + hadley_ext).min(1.0);
            let itcz_gate = conv_avail * conv_suppress
                * (1.0 - 0.75 * hadley_total * hadley_summer_factor);

            let mut p = moisture * BASE_PRECIPITATION;

            // Seasonally-migrated ITCZ (gated).
            p += itcz_bonus_shifted(lat, itcz_shift) * itcz_gate;

            // Subtropical high â€” a SUMMER phenomenon (the descending Hadley branch
            // parks over the subtropics in the warm season). Weighted stronger in the
            // local summer / weaker in winter so a dry-summer (Mediterranean) split
            // can emerge; the two-season average â‰ˆ the annual penalty.
            let sub_pen = (subtropical_penalty(abs_lat) * if local_summer { 1.35 } else { 0.55 }).min(0.9);
            p *= 1.0 - sub_pen;

            // Monsoon interior penetration (multiplicative).
            p *= monsoon_multiplier(abs_lat, buf.distance_to_ocean[idx], ctx.land_frac_tropical);

            // Additive summer monsoon — LOCAL-SUMMER ONLY, gated to onshore-flow
            // regions and damped by the convection suppressors. This is what makes the
            // wet season wet and the dry season dry (the emergent Aw/Am/Cwa split).
            // The Hadley inversion lid (hadley_total) also suppresses the monsoon bonus:
            // even where surface onshore flow is present (Arabia faces the Indian Ocean),
            // the descending Hadley column physically prevents the deep convective towers
            // needed for monsoon rainfall — this is the key difference between India
            // (low hadley_total, ITCZ migrates in) and Arabia (peak hadley, lid stays on).
            // Use hadley_total (which includes hadley_ext for the Horn/Aden gap) so the
            // full inversion is accounted for, not just the latitude-based portion.
            if local_summer && (5.0..=45.0).contains(&abs_lat) {
                let monsoon_hadley_block = (1.0 - 0.90 * hadley_total).max(0.0);
                p += monsoon_bonus(abs_lat, buf.distance_to_ocean[idx], ctx.land_frac_tropical)
                    * monsoon_onshore(buf, x, y, ctx.sea_suppress)
                    * conv_suppress
                    * monsoon_hadley_block
                    * oro; // upslope enhances (Western Ghats), lee shadows (Deccan)
            }

            // Frontal storm tracks â€” a WINTER-dominant source (extratropical cyclones
            // are strongest in the cold season), so a mid-latitude west coast gets its
            // dry-summer Mediterranean rhythm; the annual average is preserved.
            let near_ocean = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 { return false; }
                buf.terrain[buf.idx(buf.wrap_x(x as i32 + dx), ny as u32)] == 0
            });
            // Frontal storm tracks: fire in BOTH seasons but with a latitude-
            // dependent summer suppression in the 28-47 deg band where the
            // subtropical high expands poleward in local summer and physically
            // blocks the westerly storm track (the Mediterranean mechanism).
            // Peak blocking near 37 deg; zero outside 28-47 deg. Annual sum
            // preserved: (summer + winter) scale = 2.0 at every latitude.
            //
            //   |  lat   | summer scale | winter scale |
            //   | <28    |     0.70     |     1.30     |  (unchanged)
            //   |  37    |     0.10     |     1.90     |  (peak suppression)
            //   |  47+   |     0.70     |     1.30     |  (unchanged)
            let frontal_sub_block = if abs_lat >= 28.0 && abs_lat <= 47.0 {
                if abs_lat < 37.0 { (abs_lat - 28.0) / 9.0 }
                else              { (47.0 - abs_lat) / 10.0 }
            } else { 0.0_f32 };
            let frontal_scale = if local_summer {
                0.70 - 0.60 * frontal_sub_block   // 0.70 -> 0.10 at peak
            } else {
                1.30 + 0.60 * frontal_sub_block   // 1.30 -> 1.90 at peak
            };
            // Orographic control of the FRONTAL storm-track rain — the key to the
            // Andes: at these westerly latitudes the extratropical cyclones are the
            // dominant moisture source, so uplift soaks the windward slope (the wet
            // Chilean coast / Norway / BC / South Island) while the rain shadow keeps
            // the lee bone-dry (the Argentine Patagonian steppe & pampas). Previously
            // this term ignored `oro`, so the lee stayed as wet as the windward side.
            p += frontal_bonus(abs_lat, near_ocean) * frontal_scale * SEASON_SCALE * oro;

            // Jet-exit convergence dump (monsoon terminus), amplified up windward relief.
            if jet_wet > 0.0 {
                p += jet_wet * (0.6 + 0.4 * oro.min(2.5)) * SEASON_SCALE;
            }

            // Fine-grained spatial texture (Â±18%).
            p *= 1.0 + 0.18 * fbm2(x as f32 / 6.5, y as f32 / 6.5, 28411);

            // No SEASON_SCALE here: already applied per-term above.
            precip[idx] = p.max(0.0);
        }
    }

    precip
}

/// Multi-pass isotropic (8-neighbour, centre-weighted) blur over land cells; ocean
/// held at 0. Dissolves the directional advection "zebra" while keeping the
/// large-scale wet-coast / dry-interior gradient and orographic contrasts.
fn blur_land(buf: &WorldBuffer, field: Vec<f32>, passes: u32) -> Vec<f32> {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let mut a = field;
    let mut b = vec![0.0f32; n];
    for _ in 0..passes {
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                if buf.terrain[idx] != 1 { b[idx] = 0.0; continue; }
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
    a
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
    /// NH subtropical interior (~20Â°, the Arabian latitude) just as hard as a wet
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
        // compute_itcz_shift return a strong positive (northward) shift â€” the exact
        // condition that used to push the wet ITCZ band deep into the subtropics.
        for y in 0..h {
            let lat = buf.latitude(y);
            for x in 0..w {
                let i = buf.idx(x, y);
                let land = lat > -4.0 && lat < 34.0;
                buf.terrain[i] = if land { 1 } else { 0 };
                buf.elevation[i] = if land { 0.05 } else { 0.0 };
                // Northward onshore flow (screen âˆ’y is north here): the SH ocean feeds
                // moisture onto the southern (equatorial) coast, which decays inland
                // toward the subtropics â€” the real moisture-supply gradient.
                buf.wind_vx[i] = 0.0;
                buf.wind_vy[i] = -1.0;
                // The SH source ocean carries a warm current (a strong moisture source);
                // the equatorial coast is thus well-supplied, the far interior is not.
                buf.current_type[i] = if land { 0 } else { 1 };
                buf.wind_speed[i] = 4.0;
            }
        }
        // distance_to_ocean: cheap approximation â€” degrees of latitude north of the
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
        // here since the whole band is land). Find rows nearest 3Â° and 22Â°.
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

        // The dry subtropical interior (Arabia's latitude) must be arid â€” nowhere
        // near tropical-forest wetness â€” even though the ITCZ has shifted onto it.
        assert!(sub_p < 700.0, "dry subtropical interior should stay arid, got {sub_p} mm");
        // And it must be markedly drier than the moist equatorial belt (which the
        // gate leaves wet).
        assert!(eq_p > sub_p * 1.5, "equator {eq_p} should be far wetter than subtropics {sub_p}");

        // Global land-precip mean must stay in a plausible band â€” the two-season sum
        // is allowed to shift for realism but must not collapse or explode (which
        // would give rivers/fertility a degenerate field).
        let (mut sum, mut cnt) = (0.0f64, 0u64);
        for i in 0..(w * h) as usize {
            if buf.terrain[i] == 1 { sum += buf.precipitation[i] as f64; cnt += 1; }
        }
        let mean = (sum / cnt.max(1) as f64) as f32;
        assert!((150.0..=4000.0).contains(&mean), "global land mean out of band: {mean} mm");
    }

    /// Emergent seasonality: a tropical monsoon coast fed by a warm equatorward sea
    /// must come out SUMMER-WET (high summer fraction), while the dry subtropical
    /// interior under the descending Hadley high has no strong wet season. This is
    /// what drives the emergent Aw/Am (vs Af) and BWh KÃ¶ppen assignments.
    #[test]
    fn monsoon_coast_is_summer_wet() {
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
        // NH continent (lat 6..40) north of a warm equatorial/SH ocean.
        for y in 0..h {
            let lat = buf.latitude(y);
            for x in 0..w {
                let i = buf.idx(x, y);
                let land = lat > 6.0 && lat < 40.0;
                buf.terrain[i] = if land { 1 } else { 0 };
                buf.elevation[i] = if land { 0.05 } else { 0.0 };
                buf.current_type[i] = if land { 0 } else { 1 }; // warm ocean source
                buf.wind_speed[i] = 4.0;
                buf.distance_to_ocean[i] = if land { ((lat - 6.0) / 60.0).clamp(0.0, 1.0) } else { 0.0 };
            }
        }
        compute_precipitation(&mut buf);

        let row_at = |target: f32| -> u32 {
            (0..h).min_by(|&a, &b| {
                (buf.latitude(a) - target).abs().partial_cmp(&(buf.latitude(b) - target).abs()).unwrap()
            }).unwrap()
        };
        let x = w / 2;
        // A near-coastal tropical row draws the summer monsoon â†’ summer-weighted.
        let coast = buf.idx(x, row_at(12.0));
        let frac = buf.precip_summer_frac[coast] as f32 / 255.0;
        assert!(frac > 0.55, "tropical monsoon coast should be summer-wet, frac={frac}");
    }

    /// Clausius–Clapeyron retention: zero at/below freezing (so cold-temperature
    /// unit worlds are unaffected), strictly increasing with warmth, bounded.
    #[test]
    fn cc_retain_is_gated_and_monotonic() {
        assert_eq!(cc_retain_frac(-5.0), 0.0);
        assert_eq!(cc_retain_frac(0.0), 0.0);
        assert!(cc_retain_frac(10.0) > 0.0);
        assert!(cc_retain_frac(25.0) > cc_retain_frac(10.0));
        assert!(cc_retain_frac(40.0) <= CC_STRENGTH + 1e-6);
    }

    /// The Andes rain shadow: at a westerly latitude (~45°), a meridional mountain
    /// ridge with ocean to its west must leave the WINDWARD (west) side far wetter
    /// than the LEE (east) side — the wet Chilean coast vs. the dry Argentine steppe.
    /// This guards the fix that made the dominant frontal storm-track rain respect
    /// the orographic uplift / rain shadow (previously it bypassed it).
    #[test]
    fn meridional_ridge_shadows_its_lee() {
        let (w, h) = (48u32, 120u32);
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap();
        // Ocean west of x=8; a tall N–S cordillera at x=12; land everywhere else.
        // (The westerlies at 45° blow eastward, off the western ocean onto the range.)
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                let ocean = x < 8;
                buf.terrain[i] = if ocean { 0 } else { 1 };
                buf.elevation[i] = if x == 12 { 0.5 } else if ocean { 0.0 } else { 0.06 };
            }
        }
        crate::sim::ocean::compute_distance_to_ocean(&mut buf);
        compute_precipitation(&mut buf);

        // Sample the westerly belt (~45°N).
        let row = (0..h).min_by(|&a, &b| {
            (buf.latitude(a) - 45.0).abs().partial_cmp(&(buf.latitude(b) - 45.0).abs()).unwrap()
        }).unwrap();
        let windward = buf.precipitation[buf.idx(10, row)]; // just west of the crest
        let lee = buf.precipitation[buf.idx(16, row)];       // just east of the crest
        assert!(
            windward > lee * 2.5,
            "windward slope {windward} mm should soak vs the rain-shadow lee {lee} mm"
        );
    }

    /// Direct check that the Clausius–Clapeyron retention makes a moisture parcel
    /// deplete more slowly over warm land than cold: for the same distance-decay,
    /// the warm-adjusted retained fraction exceeds the cold one. (The end-to-end
    /// pipeline effect — deeper tropical moisture penetration — rides on this; a
    /// synthetic single-continent unit world can't exercise it because season_precip
    /// advects on the DERIVED seasonal winds, not a hand-set wind field.)
    #[test]
    fn cc_retention_slows_warm_air_depletion() {
        let decay = 0.90f32; // an arbitrary per-step distance decay
        let warm = decay + (1.0 - decay) * cc_retain_frac(28.0);
        let cold = decay + (1.0 - decay) * cc_retain_frac(0.0);
        assert!(warm > cold, "warm decay {warm} should exceed cold decay {cold}");
        assert!((cold - decay).abs() < 1e-6, "cold (<=0 °C) leaves the decay unchanged");
        assert!(warm < 1.0, "retention never fully stops depletion");
    }
}

