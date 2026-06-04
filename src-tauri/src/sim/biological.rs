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
use super::goods_spec::{builtin_index_of, id_salt, Distribution, Domain, Envelope, GoodSpec};
use crate::tile::cell::GOODS_COUNT;

// Salt mixed into the per-good seeded-homeland RNG (preserves the original
// built-in seed selection: built-ins pass `index * this`).
const SEED_SALT_K: u64 = 0x9E3779B97F4A7C15;

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
// ── New goods (30..32) ──
pub const GOOD_CLOVES: usize = 30;      // tropical spice-island clove
pub const GOOD_PEPPER: usize = 31;      // tropical wet-coast peppercorn
pub const GOOD_PAPER: usize = 32;       // multi-source (papyrus/bamboo/manufactured)
// ── Curated additions (33..37): two manufactured goods, two tropical cash
// crops, one desert good. All land goods, appended LAST for save back-compat. ──
pub const GOOD_CERAMICS: usize = 33;    // porcelain/pottery — clay + skilled cities
pub const GOOD_GLASSWARE: usize = 34;   // glass — silica (coast/arid) + skilled cities
pub const GOOD_TOBACCO: usize = 35;     // warm humid-subtropical cash crop
pub const GOOD_INDIGO: usize = 36;      // hot wet tropical/subtropical dye crop (land)
pub const GOOD_DATES: usize = 37;       // hot-desert oasis fruit

/// Ordered good identifiers (sent to the frontend for labels/emoji/matrix).
pub const GOOD_NAMES: [&str; GOODS_COUNT] = [
    "silk", "wine", "oliveoil", "sugar", "frankincense", "stockfish",
    "spices", "tea", "coffee", "furs", "timber", "amber", "salt", "dyes", "incense",
    "pearls", "whaling", "wheat", "iron", "cotton", "gemstones",
    "hardwoods", "horses", "wool_fleece", "wool_llama", "ivory", "cacao",
    "copper", "tin", "gold",
    "cloves", "pepper", "paper",
    "ceramics", "glassware", "tobacco", "indigo", "dates",
];

/// Default UI metadata for each built-in good (label, icon glyph, region tint).
/// These mirror the frontend `GOOD_DEFS` and seed the editable `GoodSpec` library
/// (see `sim/goods_spec.rs`). Order matches `GOOD_NAMES`.
pub const GOOD_LABEL: [&str; GOODS_COUNT] = [
    "Silk", "Wine", "Olive Oil", "Sugar", "Frankincense", "Stockfish & Salt-cod",
    "Spices", "Tea", "Coffee", "Furs", "Timber", "Amber", "Salt", "Dyes", "Incense",
    "Pearls", "Whaling Grounds", "Wheat", "Iron / Ore", "Cotton", "Gemstones",
    "Tropical Hardwoods", "Horses", "Fleece Wool", "Highland Wool", "Ivory", "Cacao",
    "Copper", "Tin", "Gold",
    "Cloves", "Pepper", "Paper",
    "Ceramics", "Glassware", "Tobacco", "Indigo", "Dates",
];
pub const GOOD_ICON: [&str; GOODS_COUNT] = [
    "\u{1F41B}", "\u{1F377}", "\u{1FAD2}", "\u{1F36C}", "\u{1FA94}", "\u{1F41F}",
    "\u{1F336}\u{FE0F}", "\u{1F375}", "\u{2615}", "\u{1F98A}", "\u{1FAB5}", "\u{1F7E0}",
    "\u{1F9C2}", "\u{1F41A}", "\u{1F4A8}", "\u{1F9AA}", "\u{1F40B}", "\u{1F33E}",
    "\u{26CF}\u{FE0F}", "\u{1F9F6}", "\u{1F48E}", "\u{1F333}", "\u{1F40E}", "\u{1F411}",
    "\u{1F999}", "\u{1F418}", "\u{1F36B}", "\u{1F7E4}", "\u{26AA}", "\u{1F7E1}",
    "\u{1F33F}", "\u{26AB}", "\u{1F4DC}",
    "\u{1F3FA}", "\u{1FA9F}", "\u{1F6AC}", "\u{1F7E6}", "\u{1F33D}",
];
pub const GOOD_COLOR: [&str; GOODS_COUNT] = [
    "#d97fb0", "#9b2d4f", "#8ea33a", "#e8d8a0", "#c79a4b", "#6fb0c8",
    "#d2622a", "#5fae6f", "#7a4a2a", "#a9763d", "#6b8f4e", "#e0962a",
    "#cfd6dc", "#8a52c0", "#b0a0c0", "#d8e4ec", "#5878a0", "#d9b94a",
    "#9aa0a6", "#eef0e8", "#56c8d8", "#5b3a1e", "#b5793a", "#e8e3d8",
    "#c8a06a", "#efe6d0", "#6b4226", "#b06a3a", "#b8bcc0", "#d4af37",
    "#7a3b1e", "#2f2f33", "#e8e0c8",
    "#5a86c8", "#9fd8d0", "#8a6a3a", "#3a4fb0", "#c08a3a",
];
/// Default base demand weight per good (matrix desire). Single source of truth,
/// also used by the editable spec.
pub const GOOD_DESIRE: [f32; GOODS_COUNT] = [
    0.35, 0.45, 0.45, 0.50, 0.30, 0.70, // silk,wine,oliveoil,sugar,frankincense,stockfish
    0.40, 0.40, 0.45, 0.35, 0.60, 0.25, // spices,tea,coffee,furs,timber,amber
    0.75, 0.30, 0.30, 0.30, 0.55,        // salt,dyes,incense,pearls,whaling
    0.85, 0.65, 0.45, 0.40,              // wheat,iron,cotton,gemstones
    0.55, 0.70, 0.50, 0.40, 0.35, 0.40,  // hardwoods,horses,wool_fleece,wool_llama,ivory,cacao
    0.55, 0.55, 0.60,                    // copper,tin,gold
    0.55, 0.60, 0.40,                    // cloves,pepper,paper
    0.60, 0.55, 0.55, 0.45, 0.40,        // ceramics,glassware,tobacco,indigo,dates
];
/// Default scarcity per good (0..1). 0.5 is neutral (no change to belt size).
/// Higher = rarer (tighter seed/spread thresholds → a smaller belt). Most goods
/// ship neutral; cloves and paper are deliberately rarer (user request), and the
/// two manufactured goods (ceramics/glassware) are uncommon city crafts.
pub const GOOD_RARITY: [f32; GOODS_COUNT] = {
    let mut r = [0.5f32; GOODS_COUNT];
    r[GOOD_CLOVES] = 0.80;   // a single fabled spice island
    r[GOOD_PAPER] = 0.72;    // scarce; one or two papermaking homelands
    r[GOOD_CERAMICS] = 0.62; // famed porcelain centres are few
    r[GOOD_GLASSWARE] = 0.60;
    r
};

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
    false, false, false,                          // cloves, pepper, paper (land)
    false, false, false, false, false,            // ceramics, glassware, tobacco, indigo, dates (land)
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
    false, false, false,                          // cloves(seeded), pepper(seeded), paper(seeded → rarer)
    false, false, false, false, false,            // ceramics, glassware, tobacco, indigo, dates (all seeded homelands)
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
    true,  true,  true,                         // cloves, pepper, paper (paper now rare → a prized export)
    true,  true,  true,  true,  false,          // ceramics, glassware, tobacco, indigo (luxuries); dates (staple)
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
#[derive(Clone, Copy)]
pub struct DepositParams {
    pub min_elev: f32,
    pub salt: u64,
    pub count_num: u32, // count = gem_deposits * count_num / count_den (min 1)
    pub count_den: u32,
    /// `false` = highland deposit (candidates ranked purely by elevation, e.g.
    /// gems/metals). `true` = suitability deposit: candidates come from the good's
    /// own climate/relief score (e.g. salt's arid coast, iron's hill country), so
    /// the scatter follows where the good actually belongs rather than the peaks.
    pub suitability: bool,
}

/// Deposit parameters for goods placed as scattered sporadic deposits (mostly
/// single-cell, occasionally a small rich cluster), else None (the good uses a
/// continuous climate-scored belt instead). Highland deposits lock to elevation;
/// suitability deposits (salt/iron) scatter through their climate/relief score.
pub fn deposit_params(g: usize) -> Option<DepositParams> {
    match g {
        GOOD_GEMSTONES => Some(DepositParams { min_elev: GEM_MIN_ELEV, salt: 0xA1B2C3D4E5F60718, count_num: 1, count_den: 1, suitability: false }),
        GOOD_COPPER    => Some(DepositParams { min_elev: 0.30, salt: 0xC0FFEE_1234_5678, count_num: 1, count_den: 1, suitability: false }),
        GOOD_TIN       => Some(DepositParams { min_elev: 0.35, salt: 0x7117_BEEF_D00D_F00D, count_num: 2, count_den: 3, suitability: false }),
        GOOD_GOLD      => Some(DepositParams { min_elev: 0.45, salt: 0x901D_901D_901D_901D, count_num: 3, count_den: 2, suitability: false }),
        // Salt (arid-coast pans) and iron (hill country) are now scattered sporadic
        // deposits driven by their suitability score, not continuous belts.
        GOOD_SALT      => Some(DepositParams { min_elev: 0.0, salt: 0x5A17_5A17_5A17_5A17, count_num: 6, count_den: 1, suitability: true }),
        GOOD_IRON      => Some(DepositParams { min_elev: 0.0, salt: 0x1804_1804_1804_1804, count_num: 5, count_den: 1, suitability: true }),
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
/// Generate every good's belt from the active (editable) spec list. Built-in
/// goods (`builtin`, `scoring = None`) use the hardcoded scorer keyed by their id
/// and reproduce the original behavior exactly; custom goods use their declarative
/// `Envelope`. Disabled goods leave a zeroed column. One tile column is written
/// per spec (the count is no longer capped at `GOODS_COUNT`).
pub fn compute_trade_goods(
    buf: &mut WorldBuffer, _rivers: &[River], seed: u64, gem_deposits: u32, specs: &[GoodSpec],
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Size the buffer's good columns to the active spec list (grow for custom
    // goods, drop trailing columns that no longer exist).
    buf.goods.resize(specs.len().max(1), vec![0u8; n]);

    for slot in 0..specs.len() {
        let spec = match specs.get(slot) {
            Some(s) if s.enabled => s,
            _ => { buf.goods[slot] = vec![0u8; n]; continue; }
        };
        let builtin_idx = builtin_index_of(&spec.id);

        match spec.distribution {
            Distribution::Deposits => {
                // Min-elevation / count are editable; the placement salt is derived
                // (built-ins keep their original salt so output is unchanged).
                let (min_elev, num, den) = spec.deposit
                    .map(|d| (d.min_elev, d.count_num.max(1), d.count_den.max(1)))
                    .unwrap_or((0.40, 1, 1));
                let dp = builtin_idx.and_then(deposit_params);
                let salt = dp.map(|d| d.salt).unwrap_or_else(|| id_salt(&spec.id));
                // Suitability deposits (salt/iron, or any custom deposit good with a
                // scoring envelope) scatter through the good's own climate/relief
                // score; highland deposits (gems/metals) rank candidates by elevation.
                let suitability = dp.map(|d| d.suitability).unwrap_or(spec.scoring.is_some());
                let count = (gem_deposits * num / den).max(1);

                let mut cand = vec![0.0f32; n];
                if suitability {
                    for y in 0..h {
                        for x in 0..w {
                            let i = buf.idx(x, y);
                            if buf.terrain[i] != 1 { continue; }
                            cand[i] = if let Some(env) = &spec.scoring {
                                envelope_score(buf, env, spec.domain, x, y)
                            } else if let Some(idx) = builtin_idx {
                                good_score(buf, idx, x, y)
                            } else { 0.0 };
                        }
                    }
                } else {
                    for i in 0..n {
                        if buf.terrain[i] == 1 && buf.elevation[i] >= min_elev {
                            cand[i] = (buf.elevation[i] - min_elev + 0.15).min(1.0);
                        }
                    }
                }
                let thresh = if suitability { 0.30 } else { 1e-4 };
                buf.goods[slot] = place_deposits(buf, seed, count, salt, &cand, thresh);
            }
            _ => {
                let marine = matches!(spec.domain, Domain::Marine);
                let unlimited = matches!(spec.distribution, Distribution::Global);
                let mut score = vec![0.0f32; n];
                for y in 0..h {
                    for x in 0..w {
                        let s = if let Some(env) = &spec.scoring {
                            envelope_score(buf, env, spec.domain, x, y)
                        } else if let Some(idx) = builtin_idx {
                            good_score(buf, idx, x, y)
                        } else {
                            0.0
                        };
                        score[buf.idx(x, y)] = s;
                    }
                }
                // Built-ins reproduce their original seed by salting with index*K;
                // custom goods get an id-derived salt.
                let salt = match builtin_idx {
                    Some(idx) if spec.scoring.is_none() => (idx as u64).wrapping_mul(SEED_SALT_K),
                    _ => id_salt(&spec.id),
                };
                buf.goods[slot] = localize_good(buf, &score, marine, unlimited, spec.rarity, salt, seed);
            }
        }
    }
}

/// Declarative envelope scorer for custom (and overridden) goods. Reproduces the
/// house scoring style: domain gate × climate × temp/precip/elevation/lat bands ×
/// fertility × coast bonus. Absent terms contribute a neutral 1.0.
fn envelope_score(buf: &WorldBuffer, env: &Envelope, domain: Domain, x: u32, y: u32) -> f32 {
    let i = buf.idx(x, y);
    let land = buf.terrain[i] == 1;
    match domain {
        Domain::Marine => {
            if land { return 0.0; }
            if !(buf.is_shelf[i] == 1 || has_land_within(buf, x, y, 3)) { return 0.0; }
        }
        Domain::Coastal => {
            if !land || buf.distance_to_ocean[i] >= 0.12 { return 0.0; }
        }
        Domain::Continental => {
            if !land { return 0.0; }
        }
        Domain::Island => {
            // Approximation (no connected-component pass): near-coast land stands
            // in for island / small-landmass placement.
            if !land || buf.distance_to_ocean[i] >= 0.20 { return 0.0; }
        }
    }

    let k = buf.koppen[i];
    let t = buf.temperature[i];
    let p = buf.precipitation[i];
    let elev = buf.elevation[i];
    let fert = buf.fertility[i].clamp(0.0, 1.0);
    let abs_lat = buf.abs_latitude(y);

    let mut s = 1.0f32;
    if !env.climate.is_empty() {
        s *= env.climate.iter().find(|(z, _)| *z == k).map(|(_, w)| *w).unwrap_or(0.0);
    }
    if let Some([c, wd]) = env.temp { s *= bell(t, c, wd); }
    if let Some([lo, hi, e]) = env.precip { s *= band(p, lo, hi, e); }
    if let Some([lo, hi, e]) = env.elevation { s *= band(elev, lo, hi, e); }
    if let Some([lo, hi, e]) = env.abs_lat { s *= band(abs_lat, lo, hi, e); }
    if env.fertility > 0.0 { s *= (1.0 - env.fertility) + env.fertility * fert; }
    if env.coast_bonus > 0.0 {
        let near = if land { buf.distance_to_ocean[i] < 0.08 } else { true };
        if near { s *= 1.0 + env.coast_bonus; }
    }
    s.clamp(0.0, 1.0)
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
            // Sericulture is warm-temperate / humid-subtropical mulberry country
            // (China's Yangtze, Bengal, the Levant) — NOT the cold northern
            // continental interior. Dropped DFA/DFB, warmed the temperature peak,
            // and added a |lat| cap so silk stays out of the high north.
            let clim = match k { CFA | CWA => 1.0, CSA | CWB => 0.5, CFB => 0.25, _ => 0.0 };
            clim * bell(t, 21.0, 5.0) * band(p, 600.0, 1600.0, 500.0)
                * band(abs_lat, 0.0, 38.0, 8.0)
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
        GOOD_CLOVES => {
            // Clove (Moluccas / Zanzibar): hot wet tropical, strongly coastal /
            // island. Seeded → one fabled spice-island homeland.
            let clim = match k { AF | AM => 1.0, AW => 0.45, _ => 0.0 };
            let cst = if coast_near { 1.0 } else if coastland { 0.6 } else { 0.25 };
            clim * smoothstep(22.0, 27.0, t) * band(p, 1500.0, 3200.0, 700.0) * cst
        }
        GOOD_PEPPER => {
            // Black pepper (Malabar): hot, very wet monsoon coast. Seeded.
            let clim = match k { AM | AF => 1.0, AW | CWA => 0.5, _ => 0.0 };
            let cst = if coastland { 1.0 } else { 0.45 };
            clim * smoothstep(21.0, 27.0, t) * band(p, 1500.0, 3500.0, 800.0) * cst
        }
        GOOD_PAPER => {
            // Paper has THREE sources; the score is the strongest of them and the
            // overlay classifies which one wins per cell (papyrus / bamboo /
            // manufactured) for distinct icons. Unlimited (every viable cell).
            let papyrus = {
                // Warm freshwater delta / marsh reed (Nile papyrus): warm, very
                // low-lying, well-watered (fertility proxies alluvial wetland).
                let warm = smoothstep(16.0, 24.0, t);
                let low = 1.0 - smoothstep(0.12, 0.32, elev);
                warm * low * (0.25 + 0.75 * fert) * band(abs_lat, 0.0, 33.0, 8.0)
                    * if coastland { 1.0 } else { 0.7 }
            };
            let bamboo = {
                // Humid-subtropical bamboo / mulberry pulp (East-Asian paper).
                let clim = match k { CFA | CWA => 1.0, CWB | CFB => 0.5, AM => 0.4, _ => 0.0 };
                clim * smoothstep(12.0, 20.0, t) * band(p, 800.0, 2200.0, 500.0)
            };
            let manufactured = {
                // Rag/wood paper milled where there is dense settlement (a
                // "civilisation" good): keyed on habitability, climate-independent.
                smoothstep(0.50, 0.80, buf.habitability[i]) * 0.95
            };
            papyrus.max(bamboo).max(manufactured)
        }
        GOOD_CERAMICS => {
            // Porcelain / fine pottery — a *manufactured* good of skilled cities
            // sitting on good potter's clay (alluvial, well-watered lowland).
            // Climate-independent; driven by habitability (settlement skill) and
            // a clay proxy (fertility on low ground).
            let skill = smoothstep(0.45, 0.78, buf.habitability[i]);
            let clay = (0.30 + 0.70 * fert) * (1.0 - smoothstep(0.35, 0.60, elev));
            skill * clay
        }
        GOOD_GLASSWARE => {
            // Glass — skilled cities working silica sand (coastal dunes / arid
            // quartz sand) with ample fuel. Settlement-driven + a sand proxy
            // (warm coast or desert margin).
            let skill = smoothstep(0.45, 0.78, buf.habitability[i]);
            let sand = if coast_near { 1.0 }
                else { match k { BWH | BSH | BWK | BSK => 0.7, _ => 0.3 } };
            skill * sand * (1.0 - smoothstep(0.45, 0.7, elev))
        }
        GOOD_TOBACCO => {
            // Warm, humid-subtropical / tropical-savanna cash crop on fertile
            // low ground. Seeded → one New-World-style plantation homeland.
            let clim = match k { CFA | CWA => 1.0, AW | CSA => 0.5, BSH => 0.3, _ => 0.0 };
            clim * smoothstep(16.0, 22.0, t) * band(p, 700.0, 1600.0, 500.0)
                * (1.0 - smoothstep(0.35, 0.6, elev)) * (0.4 + 0.6 * fert)
        }
        GOOD_INDIGO => {
            // Indigo dye plant — hot, wet tropical/subtropical lowland. A LAND dye
            // distinct from the marine murex "dyes". Seeded.
            let clim = match k { AW | CWA => 1.0, AM | AF | CFA => 0.5, BSH => 0.3, _ => 0.0 };
            clim * smoothstep(19.0, 26.0, t) * band(p, 800.0, 2000.0, 600.0)
                * (1.0 - smoothstep(0.35, 0.6, elev)) * (0.4 + 0.6 * fert)
        }
        GOOD_DATES => {
            // Date palms — hot desert OASIS fruit: hot arid climate but locally
            // watered (fertility = oasis/wadi). Seeded.
            let clim = match k { BWH => 1.0, BSH => 0.7, BWK | BSK => 0.3, _ => 0.0 };
            let oasis = 0.25 + 0.75 * fert;
            clim * smoothstep(18.0, 26.0, t) * band(abs_lat, 12.0, 34.0, 8.0)
                * oasis * (1.0 - smoothstep(0.4, 0.65, elev))
        }
        // ── Marine goods (no walls; the score envelope itself bounds the belt) ──
        GOOD_STOCKFISH => {
            // Stockfish (dried cod) comes off the rich NORTHERN fishing banks —
            // it must track the actual fishery field, not blanket every cold
            // shelf. The hard fishery gate (no 0.3 floor) confines it to genuine
            // grounds (Lofoten / Grand Banks / North Sea style).
            if !sea_coastal { return 0.0; }
            let shelf = if buf.is_shelf[i] == 1 { 1.0 } else { 0.35 };
            let cold = 1.0 - smoothstep(2.0, 12.0, t);
            let fish = smoothstep(0.20, 0.55, buf.fishery[i].clamp(0.0, 1.0));
            shelf * cold * fish * band(abs_lat, 45.0, 72.0, 12.0)
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
/// (land goods) or simply by the score envelope (marine goods).
///
/// Turn a per-cell suitability score into a placed belt. `marine` selects the
/// passability rule, `unlimited` chooses the Global (every-cell) vs Local (one
/// seeded homeland) model, `rarity` (0..1, 0.5 = neutral) tightens the seed/spread
/// thresholds, and `salt` seeds the deterministic homeland pick.
fn localize_good(
    buf: &WorldBuffer, score: &[f32], marine: bool, unlimited: bool, rarity: f32, salt: u64, seed: u64,
) -> Vec<u8> {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let mut out = vec![0u8; n];

    // Rarity gently shifts the thresholds around the neutral 0.5 (rarer → harder
    // to seed and spread, so a smaller belt). At 0.5 these are the legacy values.
    let r = (rarity - 0.5).clamp(-0.5, 0.5);
    let seed_thresh = (0.45 + r * 0.30).clamp(0.20, 0.75);
    let spread_thresh = (0.22 + r * 0.20).clamp(0.10, 0.50);

    let passable = |i: usize| -> bool {
        if marine {
            buf.terrain[i] == 0
        } else {
            buf.terrain[i] == 1 && buf.elevation[i] < MOUNTAIN_NORM
        }
    };

    // ── UNLIMITED goods: every suitable cell produces (many producers) ──
    if unlimited {
        for i in 0..n {
            if passable(i) && score[i] >= spread_thresh {
                out[i] = q(score[i]);
            }
        }
        return out;
    }

    // ── SEEDED goods: one contiguous homeland ──
    // Deterministic weighted-random seed selection.
    let gs = seed ^ salt;
    let mut best_seed = usize::MAX;
    let mut best_key = -1.0f32;
    let mut fallback = usize::MAX;
    let mut fallback_score = spread_thresh;
    for i in 0..n {
        if !passable(i) { continue; }
        let s = score[i];
        if s > fallback_score { fallback_score = s; fallback = i; }
        if s >= seed_thresh {
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
                if !visited[ni] && score[ni] >= spread_thresh {
                    visited[ni] = true;
                    queue.push_back(ni);
                }
                break; // first passable cell along this ray decides the ray
            }
        }
    }
    out
}

/// Place a good as scattered **sporadic deposits**: `count` points seeded on the
/// candidate field `cand` (where `cand[i] >= thresh`), spaced apart and ranked by
/// a deterministic weighted-random key. Each deposit is **mostly a single cell**
/// (the abundance = its candidate weight); only occasionally (~25%) does a point
/// grow into a small 1–2-cell rich cluster. Used for gems/metals (highland `cand`)
/// and the suitability deposits salt/iron (climate/relief `cand`).
fn place_deposits(buf: &WorldBuffer, seed: u64, count: u32, salt: u64, cand: &[f32], thresh: f32) -> Vec<u8> {
    let w = buf.width as i32;
    let h = buf.height as i32;
    let n = buf.total();
    let mut out = vec![0u8; n];
    if count == 0 { return out; }

    // Candidate cells (above threshold), ranked by a deterministic weighted-random
    // key so the same seed always picks the same deposits.
    let gs = seed ^ salt;
    let mut cands: Vec<(usize, f32)> = Vec::new();
    for i in 0..n {
        if cand[i] >= thresh {
            let key = (cand[i] + 0.05) * hash01(gs ^ (i as u64).wrapping_mul(0x100000001B3));
            cands.push((i, key));
        }
    }
    if cands.is_empty() { return out; }
    cands.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Sporadic but spaced, so deposits dot the map rather than clumping.
    let min_sep = ((w as f32) * 0.025).max(3.0) as i32;
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

        // Single-cell deposit; richer candidates read brighter (abundance).
        let base_v = (0.55 + 0.45 * cand[i].clamp(0.0, 1.0)).clamp(0.0, 1.0);
        if q(base_v) > out[i] { out[i] = q(base_v); }

        // Rarely, a sporadic point is a richer multi-cell deposit (a few cells).
        let roll = hash01(gs ^ 0x0D15EA5E ^ (i as u64).wrapping_mul(0x2545F4914F6CDD1D));
        if roll < 0.25 {
            let radius = if roll < 0.08 { 2 } else { 1 };
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx == 0 && dy == 0 { continue; }
                    let d2 = (dx * dx + dy * dy) as f32;
                    if d2 > (radius * radius) as f32 + 0.1 { continue; }
                    let ny = cy + dy;
                    if ny < 0 || ny >= h { continue; }
                    let nx = buf.wrap_x(cx + dx);
                    let ni = buf.idx(nx, ny as u32);
                    if cand[ni] < thresh * 0.6 { continue; }
                    let v = (0.45 + 0.4 * cand[ni].clamp(0.0, 1.0)).clamp(0.0, 1.0);
                    if q(v) > out[ni] { out[ni] = q(v); }
                }
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
