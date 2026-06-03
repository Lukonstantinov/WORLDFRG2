//! Phase 8 — Biological layer.
//!
//! Two persisted products:
//!   • `shark_risk`  — habitat danger for "shark-infested" coastal water
//!     (bull/tiger-shark style: warm, shallow, frequented coasts; brackish
//!     river-mouth bonus). People-independent.
//!   • `goods[GOOD_*]` — trade-good belt intensities derived from climate,
//!     terrain, soil/fertility, coast and ocean productivity. Each good is a
//!     separate sublayer (land and/or marine).
//!
//! All outputs are u8 (0..255). Tuning is expected to need a visual pass.

use std::collections::VecDeque;
use super::world_buffer::WorldBuffer;
use super::rivers::River;
use super::koppen::*;
use crate::tile::cell::GOODS_COUNT;

// ── Good indices (must match TileData.goods ordering + GOOD_NAMES) ──
pub const GOOD_SILK: usize = 0;
pub const GOOD_WINE: usize = 1;
pub const GOOD_OLIVEOIL: usize = 2;
pub const GOOD_SUGAR: usize = 3;
pub const GOOD_FRANKINCENSE: usize = 4;
pub const GOOD_STOCKFISH: usize = 5;
pub const GOOD_SPICES: usize = 6;
pub const GOOD_TEA: usize = 7;
pub const GOOD_COFFEE: usize = 8;
pub const GOOD_FURS: usize = 9;
pub const GOOD_TIMBER: usize = 10;
pub const GOOD_AMBER: usize = 11;
pub const GOOD_SALT: usize = 12;
pub const GOOD_DYES: usize = 13;
pub const GOOD_INCENSE: usize = 14;
pub const GOOD_PEARLS: usize = 15;
pub const GOOD_WHALING: usize = 16;
pub const GOOD_WHEAT: usize = 17;
pub const GOOD_IRON: usize = 18;
pub const GOOD_COTTON: usize = 19;
pub const GOOD_GEMSTONES: usize = 20;
// ── Round-1 additions (21..29) ──
pub const GOOD_HARDWOODS: usize = 21;   // tropical rainforest export wood
pub const GOOD_HORSES: usize = 22;      // steppe / grassland horse country
pub const GOOD_WOOL_FLEECE: usize = 23; // cool-wet oceanic sheep pasture
pub const GOOD_WOOL_LLAMA: usize = 24;  // dry-winter highland camelid wool
pub const GOOD_IVORY: usize = 25;       // tropical-savanna megafauna
pub const GOOD_CACAO: usize = 26;       // wet tropical lowland
pub const GOOD_COPPER: usize = 27;      // hill-country ore deposits
pub const GOOD_TIN: usize = 28;         // montane ore deposits (bronze pair)
pub const GOOD_GOLD: usize = 29;        // rare highland precious-metal deposits

/// Ordered good identifiers (sent to the frontend for labels/emoji/matrix).
pub const GOOD_NAMES: [&str; GOODS_COUNT] = [
    "silk", "wine", "oliveoil", "sugar", "frankincense", "stockfish",
    "spices", "tea", "coffee", "furs", "timber", "amber", "salt", "dyes", "incense",
    "pearls", "whaling", "wheat", "iron", "cotton", "gemstones",
    "hardwoods", "horses", "wool_fleece", "wool_llama", "ivory", "cacao",
    "copper", "tin", "gold",
];

/// Domain of each good: true = its belt may sit on sea cells (marine/coastal),
/// false = land-only. (Marine goods are still scored only near the shelf/coast,
/// except whaling which spans open cold productive water.)
pub const GOOD_MARINE: [bool; GOODS_COUNT] = [
    false, false, false, false, false, true,   // silk..stockfish
    false, false, false, false, false, true,    // spices..amber (amber=coastal)
    false, true, false,                          // salt(land arid-coast), dyes(marine), incense(land)
    true, true,                                  // pearls, whaling (marine)
    false, false, false, false,                  // wheat, iron, cotton, gemstones (all land)
    false, false, false, false, false, false,    // hardwoods, horses, wool_fleece, wool_llama, ivory, cacao (land)
    false, false, false,                          // copper, tin, gold (land deposits)
];

/// Distribution model. true = UNLIMITED: the good fills *every* suitable area in
/// the world (many producers). false = SEEDED: localized to one contiguous
/// homeland (one main producer → clean trade monopolies). Gemstones use a
/// separate deposit-placement path and ignore this flag.
pub const GOOD_UNLIMITED: [bool; GOODS_COUNT] = [
    false, false, false, false, false, true,    // ..stockfish (unlimited fisheries)
    false, false, false, true, true, false,      // furs, timber unlimited
    true, false, false,                           // salt unlimited
    false, true,                                  // whaling unlimited
    true, true, false, false,                     // wheat+iron unlimited; cotton seeded; gemstones special
    false, false, false, false, false, false,     // hardwoods/horses/wools/ivory/cacao seeded
    false, false, false,                          // copper/tin/gold = deposit goods (flag unused)
];

/// Goods whose demand is only realized in a large/open trade network: distant
/// luxuries (incl. the two wool subtypes, which sit on different continents). In
/// small or closed networks — and in the good's own producing homeland — desire
/// for these is discounted (you can't trade for, or don't prize, what's far or
/// local). Staples (wheat, salt, timber, iron…) keep flat, universal demand.
pub const GOOD_NETWORK_LUXURY: [bool; GOODS_COUNT] = [
    true,  false, false, false, true,  false, // silk, _, _, _, frankincense, _
    true,  true,  true,  false, false, true,  // spices, tea, coffee, _, _, amber
    false, true,  true,  true,  false,         // _, dyes, incense, pearls, _
    false, false, false, true,                 // _, _, _, gemstones
    true,  false, true,  true,  true,  true,   // hardwoods, _, wool_fleece, wool_llama, ivory, cacao
    false, false, true,                         // _, _, gold
];

// Mountains ≥3000 m wall off a good's spread across a continent.
const MOUNTAIN_NORM: f32 = 3000.0 / 8848.0; // ≈ 0.339

// Gemstone deposits form in old highland/mountainous terrain (≥ montane).
const GEM_MIN_ELEV: f32 = 0.40;
/// Stone names cycled across gemstone deposits (for InfoPanel / region labels).
pub const GEM_STONES: [&str; 5] = ["Ruby", "Sapphire", "Emerald", "Diamond", "Topaz"];

/// Parameters for a discrete deposit-distributed good (gemstones + metals): the
/// minimum normalized elevation it locks to, a deterministic seed-salt so each
/// metal scatters independently, and a base count multiplier relative to the
/// gemstone-deposit count chosen in the UI.
struct DepositParams {
    min_elev: f32,
    salt: u64,
    count_num: u32, // count = gem_deposits * count_num / count_den (min 1)
    count_den: u32,
}

/// Deposit parameters for goods placed as scattered highland blobs, else None
/// (the good uses climate scoring + flood-fill localization instead).
fn deposit_params(g: usize) -> Option<DepositParams> {
    match g {
        GOOD_GEMSTONES => Some(DepositParams { min_elev: GEM_MIN_ELEV, salt: 0xA1B2C3D4E5F60718, count_num: 1, count_den: 1 }),
        GOOD_COPPER    => Some(DepositParams { min_elev: 0.30, salt: 0xC0FFEE_1234_5678, count_num: 1, count_den: 1 }),
        GOOD_TIN       => Some(DepositParams { min_elev: 0.35, salt: 0x7117_BEEF_D00D_F00D, count_num: 2, count_den: 3 }),
        GOOD_GOLD      => Some(DepositParams { min_elev: 0.45, salt: 0x901D_901D_901D_901D, count_num: 1, count_den: 2 }),
        _ => None,
    }
}

// ── Small scoring helpers ──────────────────────────────────────────────────

#[inline]
fn bell(x: f32, center: f32, width: f32) -> f32 {
    (-((x - center) / width).powi(2)).exp()
}

/// 0 below `lo`, ramp to 1 between lo..hi.
#[inline]
fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    if hi <= lo { return if x >= hi { 1.0 } else { 0.0 }; }
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// Plateau band: ~1 between lo..hi, falling off over `edge` on each side.
#[inline]
fn band(x: f32, lo: f32, hi: f32, edge: f32) -> f32 {
    if x < lo { (1.0 - (lo - x) / edge).max(0.0) }
    else if x > hi { (1.0 - (x - hi) / edge).max(0.0) }
    else { 1.0 }
}

#[inline]
fn q(v: f32) -> u8 { (v.clamp(0.0, 1.0) * 255.0) as u8 }

// ── Shark waters ───────────────────────────────────────────────────────────

/// Compute shark-habitat danger for sea cells. Warm, shallow, coastal water,
/// strongest where prey (fisheries) and brackish river outflow concentrate
/// large coastal sharks. Independent of settlements.
pub fn compute_shark_risk(buf: &mut WorldBuffer, rivers: &[River]) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Coast-proximity for sea cells: BFS distance from land (bounded).
    let max_coast = 8u32;
    let mut coast_d = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    for i in 0..n {
        if buf.terrain[i] == 1 {
            // seed sea neighbours of land at distance 1
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = buf.clamp_y(y as i32 + dy);
                let ni = buf.idx(nx, ny);
                if buf.terrain[ni] == 0 && coast_d[ni] > 1 {
                    coast_d[ni] = 1;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let i = buf.idx(x, y);
        let d = coast_d[i];
        if d >= max_coast { continue; }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if buf.terrain[ni] == 0 && coast_d[ni] > d + 1 {
                coast_d[ni] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    // River-mouth bonus (bull sharks frequent brackish estuaries).
    let mut river_mouth = vec![false; n];
    for river in rivers {
        if let Some(&(mx, my)) = river.points.last() {
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let nx = buf.wrap_x(mx as i32 + dx);
                    let ny = buf.clamp_y(my as i32 + dy);
                    river_mouth[buf.idx(nx, ny)] = true;
                }
            }
        }
    }

    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { buf.shark_risk[i] = 0; continue; }

            // Warmth: bull/tiger sharks favour warm water; taper out of the cold.
            let t = buf.temperature[i];
            let warmth = smoothstep(10.0, 23.0, t); // 0 ≤10°C, full ≥23°C

            // Shallow, sunlit shelf water (prey + where people swim/fish).
            let shallow = if buf.is_shelf[i] == 1 {
                1.0
            } else {
                (1.0 - (buf.sea_depth[i] - 0.10) / 0.10).clamp(0.0, 1.0)
            };

            // Frequented coast: close to shore.
            let coast = if coast_d[i] == u32::MAX {
                0.0
            } else {
                (1.0 - coast_d[i] as f32 / max_coast as f32).clamp(0.0, 1.0)
            };

            // Prey richness from the fishery field.
            let prey = 0.6 + 0.4 * buf.fishery[i].clamp(0.0, 1.0);

            // Brackish/estuary bonus: river mouths + locally fresher water.
            let brackish = if river_mouth[i] { 0.25 } else { 0.0 }
                + (1.0 - buf.salinity[i] as f32 / 255.0) * 0.10;

            let risk = (warmth * shallow * coast * prey + brackish * coast).clamp(0.0, 1.0);
            buf.shark_risk[i] = q(risk);
        }
    }
}

// ── Shipworms (Teredo) ───────────────────────────────────────────────────────

/// Compute shipworm hull-hazard for sea cells. *Teredo navalis* riddles wooden
/// ships in WARM, SHALLOW, COASTAL water and is worst where salinity is reduced
/// (estuaries / brackish harbours) and where there is shore timber. Unlike
/// sharks this is largely prey-independent and leans on warmth + low salinity.
pub fn compute_shipworm_risk(buf: &mut WorldBuffer, rivers: &[River]) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Coast distance for sea cells (BFS from land, bounded).
    let max_coast = 7u32;
    let mut coast_d = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    for i in 0..n {
        if buf.terrain[i] == 1 {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = buf.clamp_y(y as i32 + dy);
                let ni = buf.idx(nx, ny);
                if buf.terrain[ni] == 0 && coast_d[ni] > 1 {
                    coast_d[ni] = 1;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let i = buf.idx(x, y);
        let d = coast_d[i];
        if d >= max_coast { continue; }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if buf.terrain[ni] == 0 && coast_d[ni] > d + 1 {
                coast_d[ni] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    // Estuary/river-mouth bonus (brackish water shipworms thrive in).
    let mut river_mouth = vec![false; n];
    for river in rivers {
        if let Some(&(mx, my)) = river.points.last() {
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let nx = buf.wrap_x(mx as i32 + dx);
                    let ny = buf.clamp_y(my as i32 + dy);
                    river_mouth[buf.idx(nx, ny)] = true;
                }
            }
        }
    }

    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { buf.shipworm_risk[i] = 0; continue; }

            // Warm water: shipworm activity rises sharply with temperature.
            let warmth = smoothstep(13.0, 24.0, buf.temperature[i]);

            // Shallow, coastal water where wooden ships actually sail/moor.
            let shallow = if buf.is_shelf[i] == 1 {
                1.0
            } else {
                (1.0 - (buf.sea_depth[i] - 0.10) / 0.12).clamp(0.0, 1.0)
            };
            let coast = if coast_d[i] == u32::MAX {
                0.0
            } else {
                (1.0 - coast_d[i] as f32 / max_coast as f32).clamp(0.0, 1.0)
            };

            // Reduced salinity (estuary/brackish) is the shipworm's preferred
            // habitat. salinity u8 0..255 ↔ 28..42 PSU, so low u8 = fresher.
            let fresher = (1.0 - buf.salinity[i] as f32 / 255.0).clamp(0.0, 1.0);
            let brackish = 0.45 + 0.55 * fresher + if river_mouth[i] { 0.35 } else { 0.0 };

            let risk = (warmth * shallow * coast * brackish).clamp(0.0, 1.0);
            buf.shipworm_risk[i] = q(risk);
        }
    }
}

// ── Storms / cyclones (open ocean) ───────────────────────────────────────────

/// Compute the **annual** storm/cyclone potential of every sea cell. Unlike the
/// coastal shark/shipworm hazards, cyclones roam open water: the field is warm
/// tropical SST × a cyclogenesis latitude band (≈8–30°, ~0 on the equator).
/// Seasonality is derived analytically at query time from this base + latitude
/// (see `query_commands::compute_storm_zones`), so nothing per-month is stored.
/// Seasonal multiplier (0..1) applied to `storm_base` at moon `month`
/// (1..=`months`) for a cell at signed `lat` (north positive). Cyclone seasons
/// are hemisphere-offset — the northern season peaks in late summer/autumn, the
/// southern roughly half a year opposite — so there is always a calm hemisphere.
/// Near the equator the season smears toward year-round. Derived analytically so
/// nothing per-month is stored. `month <= 0` (or `months == 0`) → 1.0 (the
/// annual/combined peak).
pub fn storm_season_phase(month: i32, months: u32, lat: f32) -> f32 {
    if months == 0 || month <= 0 { return 1.0; }
    let m = ((month as u32).min(months) - 1) as f32 / months as f32; // 0..1 round the year
    let peak = if lat >= 0.0 { 0.70 } else { 0.20 };                 // fraction of year
    let theta = 2.0 * std::f32::consts::PI * (m - peak);
    let season = theta.cos().max(0.0).powf(1.5);     // concentrate into ~half the year
    let blend = smoothstep(0.0, 15.0, lat.abs());    // 0 at equator → 1 by 15°
    (season * blend + 0.5 * (1.0 - blend)).clamp(0.0, 1.0)
}

pub fn compute_storm_base(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    for y in 0..h {
        let abs_lat = buf.abs_latitude(y);
        // Cyclogenesis belt: nothing right on the equator (weak Coriolis), peak
        // through the subtropics, fading by ~30°.
        let lat_band = band(abs_lat, 8.0, 30.0, 8.0);
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { buf.storm_base[i] = 0; continue; }
            let warm = smoothstep(24.0, 27.0, buf.temperature[i]); // warm SST fuels cyclones
            buf.storm_base[i] = q(warm * lat_band);
        }
    }
}

// ── Reefs / shoals (warm shallow coast) ──────────────────────────────────────

/// Compute static reef/shoal wreck hazard for sea cells: warm, very shallow,
/// coastal water (coral-reef / atoll navigation danger). Mirrors the shark
/// coast-BFS but keys on very shallow shelf water rather than prey.
pub fn compute_reef_risk(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    let max_coast = 6u32;
    let mut coast_d = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    for i in 0..n {
        if buf.terrain[i] == 1 {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = buf.clamp_y(y as i32 + dy);
                let ni = buf.idx(nx, ny);
                if buf.terrain[ni] == 0 && coast_d[ni] > 1 {
                    coast_d[ni] = 1;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let i = buf.idx(x, y);
        let d = coast_d[i];
        if d >= max_coast { continue; }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if buf.terrain[ni] == 0 && coast_d[ni] > d + 1 {
                coast_d[ni] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { buf.reef_risk[i] = 0; continue; }

            let warm = smoothstep(20.0, 25.0, buf.temperature[i]); // coral builds in warm seas
            let very_shallow = if buf.is_shelf[i] == 1 {
                1.0
            } else {
                (1.0 - (buf.sea_depth[i] - 0.06) / 0.06).clamp(0.0, 1.0)
            };
            let coast = if coast_d[i] == u32::MAX {
                0.0
            } else {
                (1.0 - coast_d[i] as f32 / max_coast as f32).clamp(0.0, 1.0)
            };
            buf.reef_risk[i] = q(warm * very_shallow * coast);
        }
    }
}

// ── Trade goods ────────────────────────────────────────────────────────────

/// Compute every trade-good belt. Each good's raw climate/terrain suitability is
/// scored, then **localized to a single contiguous region**: a suitability-
/// weighted random seed is chosen (deterministic from the world seed) and the
/// good spreads by flood-fill through suitable cells until it hits a boundary.
/// For LAND goods the boundaries are sea/ocean and mountain ranges (≥3000 m); for
/// MARINE goods there are no physical walls — the good spreads through coast/sea
/// and stops where its environmental envelope (temperature / salinity / etc.,
/// encoded in the score) makes it unviable. This gives each good one homeland
/// (silk = one land, frankincense = one coast, pearls = one warm sea…) and a
/// clear single producer for the trade matrix.
pub fn compute_trade_goods(buf: &mut WorldBuffer, _rivers: &[River], seed: u64, gem_deposits: u32) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    for g in 0..GOODS_COUNT {
        // Deposit goods (gemstones + metals) are placed as discrete highland
        // blobs rather than climate-scored belts.
        if let Some(dp) = deposit_params(g) {
            let count = (gem_deposits * dp.count_num / dp.count_den).max(1);
            buf.goods[g] = place_deposits(buf, seed, count, dp.min_elev, dp.salt);
            continue;
        }
        let mut score = vec![0.0f32; n];
        for y in 0..h {
            for x in 0..w {
                score[buf.idx(x, y)] = good_score(buf, g, x, y);
            }
        }
        buf.goods[g] = localize_good(buf, &score, g, seed);
    }
}

/// Raw 0..1 suitability of good `g` at one cell (before localization).
fn good_score(buf: &WorldBuffer, g: usize, x: u32, y: u32) -> f32 {
    let i = buf.idx(x, y);
    let land = buf.terrain[i] == 1;
    let marine = GOOD_MARINE[g];
    if marine && land { return 0.0; }
    if !marine && !land { return 0.0; }

    let k = buf.koppen[i];
    let t = buf.temperature[i];
    let p = buf.precipitation[i];
    let elev = buf.elevation[i];
    let fert = buf.fertility[i].clamp(0.0, 1.0);
    let abs_lat = buf.abs_latitude(y);
    let coastland = land && buf.distance_to_ocean[i] < 0.12;
    let coast_near = land && buf.distance_to_ocean[i] < 0.06;
    let sea_coastal = !land && (buf.is_shelf[i] == 1 || has_land_within(buf, x, y, 3));

    match g {
        GOOD_SILK => {
            let clim = match k { CFA | CWA => 1.0, CFB | CSA => 0.6, DFA | DFB => 0.4, _ => 0.0 };
            clim * bell(t, 18.0, 7.0) * band(p, 600.0, 1600.0, 500.0)
                * (0.4 + 0.6 * fert) * (1.0 - smoothstep(0.4, 0.7, elev))
        }
        GOOD_WINE => {
            let clim = match k { CSA | CSB => 1.0, CFA | CFB | DSA | DSB => 0.45, _ => 0.0 };
            let hill = 0.7 + 0.3 * band(elev, 0.05, 0.35, 0.2);
            clim * bell(t, 16.0, 6.0) * band(p, 300.0, 950.0, 400.0) * hill
        }
        GOOD_OLIVEOIL => {
            let clim = match k { CSA => 1.0, CSB => 0.8, CFA => 0.3, _ => 0.0 };
            let warm = smoothstep(13.0, 18.0, t);
            let low = 1.0 - smoothstep(0.35, 0.6, elev);
            clim * warm * low * (0.7 + 0.3 * if coastland { 1.0 } else { 0.0 })
        }
        GOOD_SUGAR => {
            let clim = match k { AF | AM => 1.0, AW | AS => 0.6, CWA => 0.4, _ => 0.0 };
            clim * smoothstep(20.0, 25.0, t) * smoothstep(900.0, 1400.0, p)
                * (1.0 - smoothstep(0.18, 0.4, elev)) * (0.5 + 0.5 * fert)
        }
        GOOD_FRANKINCENSE => {
            let clim = match k { BWH => 1.0, BSH => 0.8, BWK | BSK => 0.3, _ => 0.0 };
            clim * band(abs_lat, 12.0, 30.0, 8.0) * (0.6 + 0.4 * band(elev, 0.08, 0.4, 0.25))
        }
        GOOD_SPICES => {
            let clim = match k { AM => 1.0, AF | AW => 0.6, _ => 0.0 };
            let cst = if coastland { 1.0 } else { 0.35 };
            clim * smoothstep(21.0, 26.0, t) * band(p, 1200.0, 3000.0, 700.0) * cst
        }
        GOOD_TEA => {
            let clim = match k { CWB | CWA => 1.0, CFA | CWC => 0.6, AW => 0.3, _ => 0.0 };
            clim * band(elev, 0.15, 0.5, 0.18) * smoothstep(900.0, 1400.0, p) * bell(t, 17.0, 7.0)
        }
        GOOD_COFFEE => {
            let clim = match k { AW | CWB => 1.0, AM | CWA => 0.6, AF => 0.3, _ => 0.0 };
            clim * band(abs_lat, 0.0, 25.0, 8.0) * band(elev, 0.12, 0.45, 0.16)
                * band(p, 1000.0, 2500.0, 700.0) * bell(t, 20.0, 6.0)
        }
        GOOD_FURS => {
            let clim = match k { DFC | DFD | DWC | DWD => 1.0, ET | DFB | DWB => 0.5, _ => 0.0 };
            clim * (1.0 - smoothstep(2.0, 10.0, t))
        }
        GOOD_TIMBER => {
            let clim = match k {
                DFB | DFC | CFB => 1.0,
                DFA | DWB | DWC | CFC | DWA => 0.6,
                _ => 0.0,
            };
            clim * smoothstep(350.0, 800.0, p) * (0.4 + 0.6 * fert) * band(t, -5.0, 18.0, 8.0)
        }
        GOOD_SALT => {
            // Arid coast solar salt pans (sebkhas). Land good.
            let clim = match k { BWH | BSH | BWK | BSK => 1.0, _ => 0.0 };
            clim * if coast_near { 1.0 } else { 0.2 }
        }
        GOOD_INCENSE => {
            let clim = match k { BWK | BSK => 1.0, BWH | BSH => 0.5, _ => 0.0 };
            let interior = 0.5 + 0.5 * smoothstep(0.06, 0.3, buf.distance_to_ocean[i]);
            clim * band(abs_lat, 18.0, 40.0, 10.0) * interior
        }
        GOOD_WHEAT => {
            // Bread-basket grain belts. Mainly Mediterranean dry-summer climates,
            // then the broader temperate grasslands / humid-continental grain
            // belts (steppe margins, oceanic, warm continental). Unlimited.
            let clim = match k {
                CSA | CSB => 1.0,                 // Mediterranean — prime wheat
                BSK | CFA | CFB => 0.7,           // steppe margin / humid-subtropical / oceanic
                DFA | DFB | DSA | DSB => 0.6,     // continental grain belt
                BSH | CWA => 0.4,                 // hot steppe / dry-winter subtropical
                _ => 0.0,
            };
            let warm = bell(t, 15.0, 9.0);
            let dryish = band(p, 300.0, 900.0, 450.0); // grain likes semi-arid to subhumid
            let low = 1.0 - smoothstep(0.45, 0.7, elev);
            clim * warm * dryish * low * (0.5 + 0.5 * fert)
        }
        GOOD_IRON => {
            // Ore in hill country and mountain margins (not the highest peaks, not
            // the flats). Any non-frozen climate. Unlimited — many producers.
            let relief = band(elev, 0.30, 0.68, 0.16);
            let not_frozen = 1.0 - smoothstep(-6.0, 2.0, -t); // dampen tundra/ice
            let volcanic = if buf.is_volcanic[i] != 0 { 1.0 } else { 0.85 };
            relief * not_frozen.clamp(0.2, 1.0) * volcanic
        }
        GOOD_COTTON => {
            // Warm subtropical river valleys / well-watered warm lowland. Seeded
            // (one growing region).
            let clim = match k { CFA | CWA => 1.0, BSH | AW | AS => 0.55, CSA => 0.4, _ => 0.0 };
            let warm = smoothstep(17.0, 23.0, t);
            let low = 1.0 - smoothstep(0.30, 0.55, elev);
            let watered = 0.35 + 0.65 * fert; // fertility carries river/alluvial moisture
            clim * warm * low * watered
        }
        GOOD_HARDWOODS => {
            // Tropical rainforest export wood (ebony / mahogany / teak). Fills the
            // gap left by `timber`, which is boreal/temperate only.
            let clim = match k { AF | AM => 1.0, AW => 0.5, CWA => 0.3, _ => 0.0 };
            clim * smoothstep(800.0, 1800.0, p) * (0.4 + 0.6 * fert)
                * (1.0 - smoothstep(0.40, 0.65, elev))
        }
        GOOD_HORSES => {
            // Open semi-arid grassland / steppe horse country.
            let clim = match k { BSK | BSH => 1.0, CFB | DFB | DSB | CWB => 0.5, BWK => 0.3, _ => 0.0 };
            clim * (1.0 - smoothstep(0.45, 0.7, elev)) * band(p, 250.0, 700.0, 350.0)
                * bell(t, 12.0, 12.0)
        }
        GOOD_WOOL_FLEECE => {
            // Cool, wet oceanic uplands — sheep fleece.
            let clim = match k { CFB | CFC => 1.0, CSB | DFB | ET => 0.5, CWB => 0.4, _ => 0.0 };
            clim * band(elev, 0.10, 0.50, 0.25) * band(p, 600.0, 1600.0, 500.0)
                * band(t, 4.0, 14.0, 7.0)
        }
        GOOD_WOOL_LLAMA => {
            // Dry-winter highland camelid wool — a distinct homeland (different
            // continent) from fleece wool.
            let clim = match k { CWB | CWC => 1.0, BSK => 0.5, ET => 0.4, _ => 0.0 };
            clim * band(elev, 0.35, 0.70, 0.18) * band(abs_lat, 0.0, 40.0, 15.0)
                * band(t, 2.0, 12.0, 8.0)
        }
        GOOD_IVORY => {
            // Tropical-savanna megafauna.
            let clim = match k { AW | AS => 1.0, BSH => 0.5, AM => 0.4, _ => 0.0 };
            clim * band(abs_lat, 0.0, 20.0, 8.0) * (1.0 - smoothstep(0.40, 0.7, elev))
                * band(p, 400.0, 1200.0, 500.0)
        }
        GOOD_CACAO => {
            // Wet tropical lowland.
            let clim = match k { AF | AM => 1.0, AW => 0.5, _ => 0.0 };
            clim * smoothstep(22.0, 27.0, t) * band(p, 1500.0, 3000.0, 600.0)
                * (1.0 - smoothstep(0.20, 0.45, elev)) * (0.5 + 0.5 * fert)
        }
        // ── Marine goods (no walls; the score envelope itself bounds the belt) ──
        GOOD_STOCKFISH => {
            if !sea_coastal { return 0.0; }
            let shelf = if buf.is_shelf[i] == 1 { 1.0 } else { 0.4 };
            shelf * (1.0 - smoothstep(2.0, 12.0, t)) * (0.3 + 0.7 * buf.fishery[i].clamp(0.0, 1.0))
                * band(abs_lat, 45.0, 70.0, 12.0)
        }
        GOOD_AMBER => {
            if !sea_coastal { return 0.0; }
            band(abs_lat, 45.0, 63.0, 8.0) * (1.0 - smoothstep(12.0, 20.0, t)) * 0.9
        }
        GOOD_DYES => {
            if !sea_coastal { return 0.0; }
            band(abs_lat, 15.0, 40.0, 10.0) * smoothstep(15.0, 22.0, t) * 0.9
        }
        GOOD_PEARLS => {
            // Warm, very shallow tropical reef/lagoon shelf.
            if !sea_coastal { return 0.0; }
            let shallow = if buf.sea_depth[i] < 0.08 {
                1.0
            } else {
                (1.0 - (buf.sea_depth[i] - 0.08) / 0.06).clamp(0.0, 1.0)
            };
            let shelfb = if buf.is_shelf[i] == 1 { 1.0 } else { 0.6 };
            smoothstep(20.0, 26.0, t) * shallow * band(abs_lat, 0.0, 24.0, 8.0) * shelfb
        }
        GOOD_WHALING => {
            // Cold, productive high-latitude water (open sea allowed). Stops where
            // the water warms below productivity — a temperature envelope.
            let cold = 1.0 - smoothstep(3.0, 13.0, t);
            let prod = (0.4 + 0.8 * buf.fishery[i].clamp(0.0, 1.0)
                + if buf.current_type[i] == 2 { 0.3 } else { 0.0 }).min(1.0);
            cold * band(abs_lat, 40.0, 78.0, 12.0) * prod
        }
        _ => 0.0,
    }
}

/// Pick one suitability-weighted random seed (deterministic from `seed`) and
/// flood-fill the good outward through suitable cells, bounded by sea/mountains
/// (land goods) or simply by the score envelope (marine goods). Returns the u8
/// belt field (score inside the region, 0 elsewhere).
fn localize_good(buf: &WorldBuffer, score: &[f32], g: usize, seed: u64) -> Vec<u8> {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let marine = GOOD_MARINE[g];
    let mut out = vec![0u8; n];

    const SEED_THRESH: f32 = 0.45;  // a cell strong enough to be a homeland seed
    const SPREAD_THRESH: f32 = 0.22; // a cell the good can still spread into

    let passable = |i: usize| -> bool {
        if marine {
            buf.terrain[i] == 0
        } else {
            buf.terrain[i] == 1 && buf.elevation[i] < MOUNTAIN_NORM
        }
    };

    // ── UNLIMITED goods: every suitable cell produces (many producers) ──
    if GOOD_UNLIMITED[g] {
        for i in 0..n {
            if passable(i) && score[i] >= SPREAD_THRESH {
                out[i] = q(score[i]);
            }
        }
        return out;
    }

    // ── SEEDED goods: one contiguous homeland ──
    // Deterministic weighted-random seed selection.
    let gs = seed ^ (g as u64).wrapping_mul(0x9E3779B97F4A7C15);
    let mut best_seed = usize::MAX;
    let mut best_key = -1.0f32;
    let mut fallback = usize::MAX;
    let mut fallback_score = SPREAD_THRESH;
    for i in 0..n {
        if !passable(i) { continue; }
        let s = score[i];
        if s > fallback_score { fallback_score = s; fallback = i; }
        if s >= SEED_THRESH {
            let key = s * hash01(gs ^ (i as u64).wrapping_mul(0x100000001B3));
            if key > best_key { best_key = key; best_seed = i; }
        }
    }
    let seed_cell = if best_seed != usize::MAX {
        best_seed
    } else if fallback != usize::MAX {
        fallback
    } else {
        return out; // good's climate doesn't exist in this world
    };

    // Island-jump: a seeded belt may hop a narrow sea (or, for marine goods, a
    // narrow land bridge) up to ~4% of the map width, so thin straits / island
    // chains don't chop one homeland into several disconnected patches. We probe
    // the 4 cardinal directions, skipping up to `jump` impassable cells to land
    // on the next passable, in-envelope cell.
    let jump = ((w as f32) * 0.04).round().clamp(2.0, 80.0) as i32;

    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();
    visited[seed_cell] = true;
    queue.push_back(seed_cell);
    while let Some(ci) = queue.pop_front() {
        out[ci] = q(score[ci]);
        let cx = (ci as u32 % w) as i32;
        let cy = (ci as u32 / w) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            // Step 1 = ordinary neighbour; steps 2..=jump = water/land gap hops.
            for k in 1..=jump {
                let nx = buf.wrap_x(cx + dx * k);
                let ny = cy + dy * k;
                if ny < 0 || ny >= h as i32 { break; }
                let ni = buf.idx(nx, ny as u32);
                if !passable(ni) { continue; } // still in the gap — keep probing
                if !visited[ni] && score[ni] >= SPREAD_THRESH {
                    visited[ni] = true;
                    queue.push_back(ni);
                }
                break; // first passable cell along this ray decides the ray
            }
        }
    }
    out
}

/// Place a good as a handful of discrete, highland-locked deposits scattered
/// worldwide (global — not climate-bound). Deterministic from the world seed.
/// Used for gemstones and the metals (copper/tin/gold), each with its own
/// `min_elev` and `salt`. `count` deposits are seeded on terrain ≥ `min_elev`,
/// spaced apart, and each grown into a small blob of nearby highland cells.
fn place_deposits(buf: &WorldBuffer, seed: u64, count: u32, min_elev: f32, salt: u64) -> Vec<u8> {
    let w = buf.width as i32;
    let h = buf.height as i32;
    let n = buf.total();
    let mut out = vec![0u8; n];
    if count == 0 { return out; }

    // Candidate highland cells, ranked by a deterministic weighted-random key so
    // the same seed always picks the same deposits.
    let gs = seed ^ salt;
    let mut cands: Vec<(usize, f32)> = Vec::new();
    for i in 0..n {
        if buf.terrain[i] == 1 && buf.elevation[i] >= min_elev {
            // Higher, more rugged ground is a touch more likely.
            let weight = (buf.elevation[i] - min_elev) + 0.15;
            let key = weight * hash01(gs ^ (i as u64).wrapping_mul(0x100000001B3));
            cands.push((i, key));
        }
    }
    if cands.is_empty() { return out; }
    cands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let min_sep = ((w as f32) * 0.06).max(6.0) as i32; // spread deposits out
    let radius = ((w as f32) * 0.006).round().clamp(2.0, 14.0) as i32; // small blob
    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > w / 2 { d = w - d; }
        d
    };

    let mut centers: Vec<(i32, i32)> = Vec::new();
    for (i, _) in cands {
        if centers.len() as u32 >= count { break; }
        let cx = (i as i32) % w;
        let cy = (i as i32) / w;
        let far = centers.iter().all(|&(qx, qy)| {
            let dx = wrap_dx(cx, qx);
            let dy = cy - qy;
            ((dx * dx + dy * dy) as f32).sqrt() >= min_sep as f32
        });
        if !far { continue; }
        centers.push((cx, cy));

        // Grow a small deposit blob of highland cells around the centre.
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 > (radius * radius) as f32 { continue; }
                let ny = cy + dy;
                if ny < 0 || ny >= h { continue; }
                let nx = buf.wrap_x(cx + dx);
                let ni = buf.idx(nx, ny as u32);
                if buf.terrain[ni] != 1 || buf.elevation[ni] < min_elev * 0.85 { continue; }
                let falloff = 1.0 - (d2.sqrt() / (radius as f32 + 1.0));
                let v = (0.55 + 0.45 * falloff).clamp(0.0, 1.0);
                if q(v) > out[ni] { out[ni] = q(v); }
            }
        }
    }
    out
}

/// Deterministic hash → [0,1) (splitmix64-style finalizer).
fn hash01(mut x: u64) -> f32 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CEB9FE1A85EC53);
    x ^= x >> 33;
    ((x >> 40) as f32) / 16_777_216.0
}

#[inline]
fn has_land_within(buf: &WorldBuffer, x: u32, y: u32, r: i32) -> bool {
    for dy in -r..=r {
        for dx in -r..=r {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            if buf.terrain[buf.idx(nx, ny)] == 1 { return true; }
        }
    }
    false
}
