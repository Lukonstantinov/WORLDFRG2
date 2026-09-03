//! Phase 8 â€” Biological layer.
//!
//! Two persisted products:
//!   â€¢ `shark_risk`  â€” habitat danger for "shark-infested" coastal water
//!     (bull/tiger-shark style: warm, shallow, frequented coasts; brackish
//!     river-mouth bonus). People-independent.
//!   â€¢ `goods[GOOD_*]` â€” trade-good belt intensities derived from climate,
//!     terrain, soil/fertility, coast and ocean productivity. Each good is a
//!     separate sublayer (land and/or marine).
//!
//! All outputs are u8 (0..255). Tuning is expected to need a visual pass.

use std::collections::VecDeque;
use crate::sim::world_buffer::WorldBuffer;
use crate::sim::rivers::River;
use crate::sim::koppen::*;
use super::goods_spec::{builtin_index_of, id_salt, Distribution, Domain, Envelope, GoodSpec, MarineBand};
use super::deposits::{bfs_dist, near};
use crate::tile::cell::GOODS_COUNT;

// Salt mixed into the per-good seeded-homeland RNG (preserves the original
// built-in seed selection: built-ins pass `index * this`).
const SEED_SALT_K: u64 = 0x9E3779B97F4A7C15;

// â”€â”€ Good indices (must match TileData.goods ordering + GOOD_NAMES) â”€â”€
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
// â”€â”€ Round-1 additions (21..29) â”€â”€
pub const GOOD_HARDWOODS: usize = 21;   // tropical rainforest export wood
pub const GOOD_HORSES: usize = 22;      // steppe / grassland horse country
pub const GOOD_WOOL_FLEECE: usize = 23; // cool-wet oceanic sheep pasture
pub const GOOD_WOOL_LLAMA: usize = 24;  // dry-winter highland camelid wool
pub const GOOD_IVORY: usize = 25;       // tropical-savanna megafauna
pub const GOOD_CACAO: usize = 26;       // wet tropical lowland
pub const GOOD_COPPER: usize = 27;      // hill-country ore deposits
pub const GOOD_TIN: usize = 28;         // montane ore deposits (bronze pair)
pub const GOOD_GOLD: usize = 29;        // rare highland precious-metal deposits
// â”€â”€ New goods (30..32) â”€â”€
pub const GOOD_CLOVES: usize = 30;      // tropical spice-island clove
pub const GOOD_PEPPER: usize = 31;      // tropical wet-coast peppercorn
pub const GOOD_PAPER: usize = 32;       // multi-source (papyrus/bamboo/manufactured)
// â”€â”€ Curated additions (33..37): two manufactured goods, two tropical cash
// crops, one desert good. All land goods, appended LAST for save back-compat. â”€â”€
pub const GOOD_CERAMICS: usize = 33;    // porcelain/pottery â€” clay + skilled cities
pub const GOOD_GLASSWARE: usize = 34;   // glass â€” silica (coast/arid) + skilled cities
pub const GOOD_TOBACCO: usize = 35;     // warm humid-subtropical cash crop
pub const GOOD_INDIGO: usize = 36;      // hot wet tropical/subtropical dye crop (land)
pub const GOOD_DATES: usize = 37;       // hot-desert oasis fruit
// â”€â”€ ~1400 curation additions (38..44): everyday staples so every climate has
// an answer in each need category (cereal/protein/sweetener/livestock/drink).
// Appended LAST for save back-compat. â”€â”€
pub const GOOD_RICE: usize = 38;        // warm-wet paddy staple
pub const GOOD_BARLEY: usize = 39;      // cool-belt grain (barley & rye)
pub const GOOD_MILLET: usize = 40;      // steppe / arid-margin grain
pub const GOOD_HERRING: usize = 41;     // everyday cold-temperate fishery
pub const GOOD_HONEY: usize = 42;       // temperate forest honey & wax
pub const GOOD_HIDES: usize = 43;       // pastoral hides & leather
pub const GOOD_BEER: usize = 44;        // famed brewing towns in grain country

/// Ordered good identifiers (sent to the frontend for labels/emoji/matrix).
pub const GOOD_NAMES: [&str; GOODS_COUNT] = [
    "silk", "wine", "oliveoil", "sugar", "frankincense", "stockfish",
    "spices", "tea", "coffee", "furs", "timber", "amber", "salt", "dyes", "incense",
    "pearls", "whaling", "wheat", "iron", "cotton", "gemstones",
    "hardwoods", "horses", "wool_fleece", "wool_llama", "ivory", "cacao",
    "copper", "tin", "gold",
    "cloves", "pepper", "paper",
    "ceramics", "glassware", "tobacco", "indigo", "dates",
    "rice", "barley", "millet", "herring", "honey", "hides", "beer",
];

/// Default UI metadata for each built-in good (label, icon glyph, region tint).
/// These mirror the frontend `GOOD_DEFS` and seed the editable `GoodSpec` library
/// (see `sim/goods_spec.rs`). Order matches `GOOD_NAMES`.
pub const GOOD_LABEL: [&str; GOODS_COUNT] = [
    "Silk", "Wine", "Olive Oil", "Sugar Cane", "Frankincense", "Stockfish & Salt-cod",
    "Spices", "Tea", "Coffee", "Furs", "Timber", "Amber", "Rock Salt", "Dyes", "Incense",
    "Pearls", "Whaling Grounds", "Grain", "Iron / Ore", "Cotton", "Gemstones",
    "Tropical Hardwoods", "Horses", "Fleece Wool", "Highland Wool", "Ivory", "Cacao",
    "Copper", "Tin", "Gold",
    "Cloves", "Pepper", "Paper",
    "Ceramics", "Glassware", "Tobacco", "Indigo", "Dates",
    "Rice", "Barley & Rye", "Millet", "Herring", "Honey & Wax", "Hides & Leather", "Beer & Ale",
];
pub const GOOD_ICON: [&str; GOODS_COUNT] = [
    "\u{1F41B}", "\u{1F377}", "\u{1FAD2}", "\u{1F36C}", "\u{1FA94}", "\u{1F41F}",
    "\u{1F336}\u{FE0F}", "\u{1F375}", "\u{2615}", "\u{1F98A}", "\u{1FAB5}", "\u{1F7E0}",
    "\u{1F9C2}", "\u{1F41A}", "\u{1F4A8}", "\u{1F9AA}", "\u{1F40B}", "\u{1F33E}",
    "\u{26CF}\u{FE0F}", "\u{1F9F6}", "\u{1F48E}", "\u{1F333}", "\u{1F40E}", "\u{1F411}",
    "\u{1F999}", "\u{1F418}", "\u{1F36B}", "\u{1F7E4}", "\u{26AA}", "\u{1F7E1}",
    "\u{1F33F}", "\u{26AB}", "\u{1F4DC}",
    "\u{1F3FA}", "\u{1FA9F}", "\u{1F6AC}", "\u{1F7E6}", "\u{1F33D}",
    "\u{1F35A}", "\u{1F35E}", "\u{1F963}", "\u{1F420}", "\u{1F36F}", "\u{1F404}", "\u{1F37A}",
];
pub const GOOD_COLOR: [&str; GOODS_COUNT] = [
    "#d97fb0", "#9b2d4f", "#8ea33a", "#e8d8a0", "#c79a4b", "#6fb0c8",
    "#d2622a", "#5fae6f", "#7a4a2a", "#a9763d", "#6b8f4e", "#e0962a",
    "#cfd6dc", "#8a52c0", "#b0a0c0", "#d8e4ec", "#5878a0", "#d9b94a",
    "#9aa0a6", "#eef0e8", "#56c8d8", "#5b3a1e", "#b5793a", "#e8e3d8",
    "#c8a06a", "#efe6d0", "#6b4226", "#b06a3a", "#b8bcc0", "#d4af37",
    "#7a3b1e", "#2f2f33", "#e8e0c8",
    "#5a86c8", "#9fd8d0", "#8a6a3a", "#3a4fb0", "#c08a3a",
    "#e6e2c8", "#c8a85a", "#d8c070", "#7ab8d0", "#e0a020", "#9a7a50", "#d09030",
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
    0.80, 0.70, 0.50, 0.65, 0.45, 0.55, 0.50, // rice,barley,millet,herring,honey,hides,beer
];
/// Default scarcity per good (0..1). 0.5 is neutral (no change to belt size).
/// Higher = rarer (tighter seed/spread thresholds â†’ a smaller belt). Most goods
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
    false, false, false, true, false, false, false, // rice..beer (only herring is marine)
];

/// Distribution model. true = UNLIMITED: the good fills *every* suitable area in
/// the world (many producers). false = SEEDED: localized to one contiguous
/// homeland (one main producer â†’ clean trade monopolies). Gemstones use a
/// separate deposit-placement path and ignore this flag.
pub const GOOD_UNLIMITED: [bool; GOODS_COUNT] = [
    false, false, false, false, false, true,    // ..stockfish (unlimited fisheries)
    false, false, false, true, true, false,      // furs, timber unlimited
    true, false, false,                           // salt unlimited
    false, true,                                  // whaling unlimited
    true, true, false, false,                     // wheat+iron unlimited; cotton seeded; gemstones special
    false, false, false, false, false, false,     // hardwoods/horses/wools/ivory/cacao seeded
    false, false, false,                          // copper/tin/gold = deposit goods (flag unused)
    false, false, false,                          // cloves(seeded), pepper(seeded), paper(seeded â†’ rarer)
    false, false, false, false, false,            // ceramics, glassware, tobacco, indigo, dates (all seeded homelands)
    true, true, true, true, true, true, false,     // staples unlimited; beer = famed brewing homeland
];

/// Goods whose demand is only realized in a large/open trade network: distant
/// luxuries (incl. the two wool subtypes, which sit on different continents). In
/// small or closed networks â€” and in the good's own producing homeland â€” desire
/// for these is discounted (you can't trade for, or don't prize, what's far or
/// local). Staples (wheat, salt, timber, ironâ€¦) keep flat, universal demand.
pub const GOOD_NETWORK_LUXURY: [bool; GOODS_COUNT] = [
    true,  false, false, false, true,  false, // silk, _, _, _, frankincense, _
    true,  true,  true,  false, false, true,  // spices, tea, coffee, _, _, amber
    false, true,  true,  true,  false,         // _, dyes, incense, pearls, _
    false, false, false, true,                 // _, _, _, gemstones
    true,  false, true,  true,  true,  true,   // hardwoods, _, wool_fleece, wool_llama, ivory, cacao
    false, false, true,                         // _, _, gold
    true,  true,  true,                         // cloves, pepper, paper (paper now rare â†’ a prized export)
    true,  true,  true,  true,  false,          // ceramics, glassware, tobacco, indigo (luxuries); dates (staple)
    false, false, false, false, false, false, false, // everyday staples, never network luxuries
];

/// Need category per good. Within a category, alternatives substitute for each
/// other in the market's needs ladder (a city short of wheat buys rice or
/// barley at a small penalty). 15 categories â€” see the redesign plan III.5.
pub const GOOD_CATEGORY: [&str; GOODS_COUNT] = [
    "fiber", "drink", "oil", "sweetener", "aromatic", "protein",      // silk..stockfish
    "aromatic", "drink", "drink", "prestige", "construction", "prestige", // spices..amber
    "preservative", "dye", "aromatic",                                 // salt, dyes, incense
    "prestige", "oil",                                                 // pearls, whaling(oil)
    "cereal", "metal", "fiber", "gem",                                 // wheat, iron, cotton, gemstones (split into gem types; "gem" is fungible)
    "construction", "livestock", "fiber", "fiber", "prestige", "drink", // hardwoods..cacao
    "metal", "metal", "metal",                                         // copper, tin, gold
    "aromatic", "aromatic", "craft",                                   // cloves, pepper, paper
    "craft", "craft", "prestige", "dye", "sweetener",                  // ceramics..dates
    "cereal", "cereal", "cereal", "protein", "sweetener", "livestock", "drink", // rice..beer
];
/// Needs ladder tier: 0 = basic need (food, fuel, salt, clothâ€¦) filled first,
/// 1 = comfort, 2 = luxury (filled last; price-elastic).
pub const GOOD_NEED_TIER: [u8; GOODS_COUNT] = [
    2, 1, 1, 2, 2, 0,    // silk, wine, oliveoil, sugar, frankincense, stockfish
    2, 2, 2, 2, 0, 2,    // spices, tea, coffee, furs, timber, amber
    0, 2, 2,             // salt, dyes, incense
    2, 1,                // pearls, whaling
    0, 1, 1, 2,          // wheat, iron, cotton, gemstones
    1, 1, 1, 1, 2, 2,    // hardwoods, horses, wool_fleece, wool_llama, ivory, cacao
    1, 1, 2,             // copper, tin, gold
    2, 2, 2,             // cloves, pepper, paper
    1, 2, 2, 2, 0,       // ceramics, glassware, tobacco, indigo, dates
    0, 0, 0, 0, 1, 0, 1, // rice, barley, millet, herring, honey, hides, beer
];
/// World-standard value per unit in the GRAIN-EQUIVALENT numeraire (wheat = 1.0
/// by definition). The market quotes every price in this standard.
pub const GOOD_BASE_VALUE: [f32; GOODS_COUNT] = [
    20.0, 3.0, 3.0, 5.0, 12.0, 2.5,  // silk, wine, oliveoil, sugar, frankincense, stockfish
    15.0, 6.0, 6.0, 8.0, 0.8, 15.0,  // spices, tea, coffee, furs, timber, amber
    2.5, 10.0, 9.0,                  // salt, dyes, incense
    30.0, 4.0,                       // pearls, whaling
    1.0, 3.0, 3.5, 60.0,             // wheat, iron, cotton, gemstones
    2.5, 12.0, 3.0, 3.0, 20.0, 8.0,  // hardwoods, horses, wool_fleece, wool_llama, ivory, cacao
    4.0, 6.0, 50.0,                  // copper, tin, gold
    25.0, 12.0, 8.0,                 // cloves, pepper, paper
    7.0, 9.0, 6.0, 12.0, 1.5,        // ceramics, glassware, tobacco, indigo, dates
    1.1, 0.9, 0.8, 1.8, 4.0, 2.0, 1.5, // rice, barley, millet, herring, honey, hides, beer
];

/// Freight weight/volume multiplier per good (1.0 = a compact luxury like silk;
/// 3-4 = a bulky low-value staple â€” timber, grain, ore â€” whose haulage eats its
/// value so it stays regional). Multiplies the per-day freight cost.
pub const GOOD_BULK: [f32; GOODS_COUNT] = [
    1.0, 2.5, 2.2, 2.0, 1.0, 1.8,    // silk, wine, oliveoil, sugar, frankincense, stockfish
    1.0, 1.2, 1.3, 1.2, 4.0, 1.0,    // spices, tea, coffee, furs, timber, amber
    3.0, 1.2, 1.0,                   // salt, dyes, incense
    1.0, 2.0,                        // pearls, whaling(oil)
    3.0, 3.5, 1.8, 1.0,              // wheat, iron, cotton, gemstones
    3.5, 2.0, 1.6, 1.6, 1.2, 1.4,    // hardwoods, horses, wool_fleece, wool_llama, ivory, cacao
    3.0, 3.0, 1.0,                   // copper, tin, gold
    1.0, 1.0, 1.5,                   // cloves, pepper, paper
    2.5, 2.2, 1.4, 1.2, 1.8,         // ceramics, glassware, tobacco, indigo, dates
    3.0, 3.0, 3.0, 2.0, 1.6, 2.0, 3.0, // rice, barley, millet, herring, honey, hides, beer
];
/// Extra freight cost per travel-day from spoilage (additive). 0 = durable
/// (metals, salt-cod, dried/preserved); high for fresh fish & fruit so they
/// can't sail across the world.
pub const GOOD_PERISH: [f32; GOODS_COUNT] = [
    0.0, 0.02, 0.01, 0.0, 0.0, 0.0,  // silk, wine, oliveoil, sugar, frankincense, stockfish(salted)
    0.0, 0.0, 0.0, 0.01, 0.0, 0.0,   // spices, tea, coffee, furs, timber, amber
    0.0, 0.0, 0.0,                   // salt, dyes, incense
    0.0, 0.02,                       // pearls, whaling
    0.02, 0.0, 0.0, 0.0,             // wheat, iron, cotton, gemstones
    0.0, 0.05, 0.0, 0.0, 0.0, 0.01,  // hardwoods, horses(livestock), wool_fleece, wool_llama, ivory, cacao
    0.0, 0.0, 0.0,                   // copper, tin, gold
    0.0, 0.0, 0.01,                  // cloves, pepper, paper
    0.0, 0.0, 0.01, 0.0, 0.04,       // ceramics, glassware, tobacco, indigo, dates(fruit)
    0.02, 0.02, 0.02, 0.55, 0.0, 0.02, 0.04, // rice, barley, millet, herring(fresh-VERY perishable, eaten at origin; salt it to ship), honey, hides, beer
];

// Mountains â‰¥3000 m wall off a good's spread across a continent.
const MOUNTAIN_NORM: f32 = 3000.0 / 8848.0; // â‰ˆ 0.339

// Gemstone deposits form in old highland/mountainous terrain (â‰¥ montane).
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
}

/// Deposit parameters for the six BUILT-IN deposit goods. Since the geological
/// placer landed (`sim::deposits`) the only field placement still reads is `salt`
/// — kept so a built-in mineral's district centres stay reproducible against the
/// seed — plus the counts, which now mean ORE DISTRICTS rather than single cells.
/// WHERE a mineral goes is decided by its `DepositModel`, not by `min_elev`.
pub fn deposit_params(g: usize) -> Option<DepositParams> {
    match g {
        GOOD_GEMSTONES => Some(DepositParams { min_elev: GEM_MIN_ELEV, salt: 0xA1B2C3D4E5F60718, count_num: 1, count_den: 1 }),
        GOOD_COPPER    => Some(DepositParams { min_elev: 0.30, salt: 0xC0FFEE_1234_5678, count_num: 1, count_den: 1 }),
        GOOD_TIN       => Some(DepositParams { min_elev: 0.35, salt: 0x7117_BEEF_D00D_F00D, count_num: 2, count_den: 3 }),
        GOOD_GOLD      => Some(DepositParams { min_elev: 0.45, salt: 0x901D_901D_901D_901D, count_num: 3, count_den: 2 }),
        // Salt (arid-coast pans) and iron (hill country) are now scattered sporadic
        // deposits driven by their suitability score, not continuous belts.
        GOOD_SALT      => Some(DepositParams { min_elev: 0.0, salt: 0x5A17_5A17_5A17_5A17, count_num: 6, count_den: 1 }),
        GOOD_IRON      => Some(DepositParams { min_elev: 0.0, salt: 0x1804_1804_1804_1804, count_num: 5, count_den: 1 }),
        _ => None,
    }
}

// â”€â”€ Small scoring helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Shark waters â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
            let warmth = smoothstep(10.0, 23.0, t); // 0 â‰¤10Â°C, full â‰¥23Â°C

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

// â”€â”€ Shipworms (Teredo) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
            // habitat. salinity u8 0..255 â†” 28..42 PSU, so low u8 = fresher.
            let fresher = (1.0 - buf.salinity[i] as f32 / 255.0).clamp(0.0, 1.0);
            let brackish = 0.45 + 0.55 * fresher + if river_mouth[i] { 0.35 } else { 0.0 };

            let risk = (warmth * shallow * coast * brackish).clamp(0.0, 1.0);
            buf.shipworm_risk[i] = q(risk);
        }
    }
}

// â”€â”€ Disease (malaria / fever) â€” land hazard â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Per-LAND-cell malaria/fever risk: warm, wet, low-lying ground near standing
/// water (river floodplains, coastal lagoons, marshy lowlands) breeds the
/// mosquito vector. Highest in wet tropics/subtropics, ~0 in deserts, cool
/// highlands and cold latitudes. Stored u8 0..255 â€” suppresses settlement and is
/// rendered as the Disease overlay. Depends only on climate/relief/water, so it
/// is computed in the settlement phase (before habitability needs it).
pub fn compute_disease_risk(buf: &mut WorldBuffer, rivers: &[River]) {
    let w = buf.width;
    let n = buf.total();

    // Bounded BFS distance from standing/fresh water: river cells + low coastal
    // ground (lagoon / mangrove). Closer water = stronger breeding habitat.
    let max_d = 6u32;
    let mut wd = vec![u32::MAX; n];
    let mut queue = VecDeque::new();
    for river in rivers {
        for &(rx, ry) in &river.points {
            let i = buf.idx(rx, ry);
            if wd[i] != 0 { wd[i] = 0; queue.push_back((rx, ry)); }
        }
    }
    for i in 0..n {
        if buf.terrain[i] == 1 && buf.distance_to_ocean[i] < 0.02 && buf.elevation[i] < 0.10 && wd[i] != 0 {
            wd[i] = 0;
            queue.push_back(((i as u32) % w, (i as u32) / w));
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let d = wd[buf.idx(x, y)];
        if d >= max_d { continue; }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if wd[ni] > d + 1 { wd[ni] = d + 1; queue.push_back((nx, ny)); }
        }
    }

    for i in 0..n {
        if buf.terrain[i] != 1 { buf.disease_risk[i] = 0; continue; }
        let t = buf.temperature[i];
        // Warm vector window: from ~15Â°C, fading out in extreme heat/aridity.
        let warmth = smoothstep(15.0, 22.0, t) * (1.0 - smoothstep(33.0, 40.0, t));
        // Standing water needs moisture; dry land breeds little.
        let wet = band(buf.precipitation[i], 700.0, 3000.0, 500.0);
        // Malaria fades with altitude (cooler, fewer pools).
        let lowland = 1.0 - smoothstep(0.12, 0.40, buf.elevation[i]);
        // Proximity to standing / fresh water.
        let water = if wd[i] == u32::MAX { 0.0 } else { 1.0 - wd[i] as f32 / max_d as f32 };
        // KÃ¶ppen reinforcement: wet tropics & humid subtropics are worst.
        let clim = match buf.koppen[i] {
            1 | 2 => 1.0,        // Af / Am rainforest & monsoon
            3 | 23 => 0.85,      // Aw / As savanna (seasonal pools)
            11 | 24 => 0.75,     // Cfa / Cwa humid subtropical
            8 | 9 | 12 => 0.35,  // Mediterranean / oceanic (marsh only)
            _ => 0.15,
        };
        let risk = (warmth * wet * lowland * (0.35 + 0.65 * water) * clim).clamp(0.0, 1.0);
        buf.disease_risk[i] = q(risk);
    }
}

// â”€â”€ Storms / cyclones (open ocean) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compute the **annual** storm/cyclone potential of every sea cell. Unlike the
/// coastal shark/shipworm hazards, cyclones roam open water: the field is warm
/// tropical SST Ã— a cyclogenesis latitude band (â‰ˆ8â€“30Â°, ~0 on the equator).
/// Seasonality is derived analytically at query time from this base + latitude
/// (see `query_commands::compute_storm_zones`), so nothing per-month is stored.
/// Seasonal multiplier (0..1) applied to `storm_base` at moon `month`
/// (1..=`months`) for a cell at signed `lat` (north positive). Cyclone seasons
/// are hemisphere-offset â€” the northern season peaks in late summer/autumn, the
/// southern roughly half a year opposite â€” so there is always a calm hemisphere.
/// Near the equator the season smears toward year-round. Derived analytically so
/// nothing per-month is stored. `month <= 0` (or `months == 0`) â†’ 1.0 (the
/// annual/combined peak).
pub fn storm_season_phase(month: i32, months: u32, lat: f32) -> f32 {
    if months == 0 || month <= 0 { return 1.0; }
    let m = ((month as u32).min(months) - 1) as f32 / months as f32; // 0..1 round the year
    let peak = if lat >= 0.0 { 0.70 } else { 0.20 };                 // fraction of year
    let theta = 2.0 * std::f32::consts::PI * (m - peak);
    let season = theta.cos().max(0.0).powf(1.5);     // concentrate into ~half the year
    let blend = smoothstep(0.0, 15.0, lat.abs());    // 0 at equator â†’ 1 by 15Â°
    (season * blend + 0.5 * (1.0 - blend)).clamp(0.0, 1.0)
}

pub fn compute_storm_base(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    for y in 0..h {
        let abs_lat = buf.abs_latitude(y);
        // Cyclogenesis belt: nothing right on the equator (weak Coriolis), peak
        // through the subtropics, fading by ~30Â°.
        let lat_band = band(abs_lat, 8.0, 30.0, 8.0);
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { buf.storm_base[i] = 0; continue; }
            let warm = smoothstep(24.0, 27.0, buf.temperature[i]); // warm SST fuels cyclones
            buf.storm_base[i] = q(warm * lat_band);
        }
    }
}

// â”€â”€ Reefs / shoals (warm shallow coast) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Trade goods â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compute every trade-good belt. Each good's raw climate/terrain suitability is
/// scored, then **localized to a single contiguous region**: a suitability-
/// weighted random seed is chosen (deterministic from the world seed) and the
/// good spreads by flood-fill through suitable cells until it hits a boundary.
/// For LAND goods the boundaries are sea/ocean and mountain ranges (â‰¥3000 m); for
/// MARINE goods there are no physical walls â€” the good spreads through coast/sea
/// and stops where its environmental envelope (temperature / salinity / etc.,
/// encoded in the score) makes it unviable. This gives each good one homeland
/// (silk = one land, frankincense = one coast, pearls = one warm seaâ€¦) and a
/// clear single producer for the trade matrix.
/// Generate every good's belt from the active (editable) spec list. Built-in
/// goods (`builtin`, `scoring = None`) use the hardcoded scorer keyed by their id
/// and reproduce the original behavior exactly; custom goods use their declarative
/// `Envelope`. Disabled goods leave a zeroed column. One tile column is written
/// per spec (the count is no longer capped at `GOODS_COUNT`).
/// Terminal SALT lakes are evaporite salt factories (Bonneville, Uyuni, Assal).
/// For each endorheic lake whose brine reaches salt-production strength, write the
/// brine into the persisted salinity column (so the salinity layer surfaces the
/// inland playa) and force strong SALT production on the pan cells and their
/// immediate evaporite shore. Coastal solar-salt belts are still placed by the
/// normal good pass; this adds the inland lake source, wiring salt lakes into the
/// economy. No-op if the world has no salt good or no salt lakes.
pub fn apply_salt_pans(buf: &mut WorldBuffer, lakes: &[crate::sim::rivers::Lake], specs: &[GoodSpec]) {
    let salt_slot = match specs.iter().position(|s| builtin_index_of(&s.id) == Some(GOOD_SALT)) {
        Some(s) => s,
        None => return,
    };
    if buf.goods.len() <= salt_slot { return; }
    let h = buf.height as i32;
    let have_sal = !buf.salinity.is_empty();
    let shore_u8 = crate::sim::rivers::salinity_to_u8(crate::sim::rivers::SALT_PRODUCTION_PPT);
    for lk in lakes {
        if !lk.endorheic || lk.salinity_ppt < crate::sim::rivers::SALT_PRODUCTION_PPT { continue; }
        let pan_u8 = crate::sim::rivers::salinity_to_u8(lk.salinity_ppt);
        for &(x, y) in &lk.cells {
            let ci = buf.idx(x, y);
            if have_sal { buf.salinity[ci] = buf.salinity[ci].max(pan_u8); }
            buf.goods[salt_slot][ci] = buf.goods[salt_slot][ci].max(230);
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h { continue; }
                let ni = buf.idx(nx, ny as u32);
                if buf.terrain[ni] == 1 {
                    buf.goods[salt_slot][ni] = buf.goods[salt_slot][ni].max(180);
                    if have_sal { buf.salinity[ni] = buf.salinity[ni].max(shore_u8); }
                }
            }
        }
    }
}

// ── River placement factors (CLAUDE.md §8.19 (goods localities, shipped) Slice 1, F6) ─────────────
//
// `good_score`/`envelope_score` never read rivers at all before this — the only
// river influence on goods placement was fertility's single 0.20 proximity
// scalar. Built ONCE per world (a per-good BFS across 45 goods would be a real
// cost, exactly the discipline `deposits::GeoContext` already applies) and reused
// by every good's score arm.
pub struct RiverContext {
    /// Cells to the nearest river cell of ANY kind.
    pub dist_any: Vec<u16>,
    /// Cells to the nearest NAVIGABLE river cell — the good can reach market by water.
    pub dist_navigable: Vec<u16>,
    /// 1 = delta/floodplain membership (a major river's own delta cells, plus low
    /// flat ground a short reach from a major river's course).
    pub floodplain: Vec<u8>,
}

impl RiverContext {
    pub fn build(buf: &WorldBuffer, rivers: &[River]) -> RiverContext {
        let w = buf.width;
        let h = buf.height;
        let n = buf.total();
        let mut any_seeds = Vec::new();
        let mut nav_seeds = Vec::new();
        let mut major_seeds = Vec::new();
        let mut floodplain = vec![0u8; n];
        for r in rivers {
            for &(x, y) in &r.points {
                if x >= w || y >= h { continue; }
                let i = (y as usize) * (w as usize) + x as usize;
                any_seeds.push(i);
                if r.navigable { nav_seeds.push(i); }
                if r.major { major_seeds.push(i); }
            }
            for &(x, y) in &r.delta {
                if x >= w || y >= h { continue; }
                floodplain[(y as usize) * (w as usize) + x as usize] = 1;
            }
        }
        let dist_any = bfs_dist(&any_seeds, w, h);
        let dist_navigable = bfs_dist(&nav_seeds, w, h);
        let dist_major = bfs_dist(&major_seeds, w, h);
        // Low flat ground within a short reach of a MAJOR river's course is a
        // floodplain even outside a mapped delta (the mid-course alluvial plain,
        // not just the river mouth).
        for i in 0..n {
            if floodplain[i] == 1 { continue; }
            if buf.terrain.get(i).copied().unwrap_or(0) != 1 { continue; }
            let dm = dist_major.get(i).copied().unwrap_or(u16::MAX);
            let elev = buf.elevation.get(i).copied().unwrap_or(1.0);
            if dm <= 3 && elev < 0.14 { floodplain[i] = 1; }
        }
        RiverContext { dist_any, dist_navigable, floodplain }
    }
}

/// A MULTIPLIER on an existing score, never a replacement (§5.4) — every weight
/// defaults to 0 (no effect) so a good that ignores rivers is untouched, and even
/// at full weight this only ever scales a score the climate gate already allowed
/// through zero. `w_irrig` only fires on arid (Köppen B) cells — river water
/// reaching a desert is irrigation; the same proximity in a humid climate is not.
#[inline]
fn river_multiplier(
    rc: Option<&RiverContext>, buf: &WorldBuffer, i: usize,
    w_flood: f32, w_irrig: f32, w_bank: f32, w_float: f32,
) -> f32 {
    let Some(rc) = rc else { return 1.0 };
    let mut m = 1.0f32;
    if w_flood > 0.0 {
        m += w_flood * (rc.floodplain.get(i).copied().unwrap_or(0) as f32);
    }
    if w_irrig > 0.0 {
        let k = buf.koppen.get(i).copied().unwrap_or(0);
        if matches!(k, BWH | BWK | BSH | BSK) {
            m += w_irrig * near(rc.dist_any.get(i).copied().unwrap_or(u16::MAX), 6.0);
        }
    }
    if w_bank > 0.0 {
        m += w_bank * near(rc.dist_any.get(i).copied().unwrap_or(u16::MAX), 4.0);
    }
    if w_float > 0.0 {
        m += w_float * near(rc.dist_navigable.get(i).copied().unwrap_or(u16::MAX), 10.0);
    }
    m.max(0.0)
}

/// CLAUDE.md §8.19 (goods localities, shipped) Slice 2 (F5) — the marine-band gate. `Either`
/// reproduces the caller's own gate unchanged (the historical undifferentiated
/// `sea_coastal` test); `Inshore`/`Bank` narrow it to a strict SUBSET of that same
/// footprint, so an `Either` good's placement is byte-identical to before this
/// slice and only the specifically-tagged goods (§2.1's default table) shrink.
#[inline]
fn marine_band_ok(buf: &WorldBuffer, x: u32, y: u32, marine_band: MarineBand) -> bool {
    match marine_band {
        MarineBand::Either => true,
        MarineBand::Inshore => has_land_within(buf, x, y, 1),
        MarineBand::Bank => {
            let i = buf.idx(x, y);
            buf.is_shelf.get(i).copied().unwrap_or(0) == 1 && !has_land_within(buf, x, y, 1)
        }
    }
}

/// The FINE-GRAIN terrain multiplier for one good at one cell — soil class and
/// local slope, the two channels that vary at 2-10 km rather than at hundreds of
/// km (see `GoodSpec::soil`/`::relief` for why that matters). Returns exactly 1.0
/// for a good declaring neither, so this is a true no-op on every good and every
/// saved world that predates it, not an approximation of one.
fn terroir_multiplier(buf: &WorldBuffer, spec: &GoodSpec, x: u32, y: u32) -> f32 {
    if spec.soil.is_empty() && spec.relief.is_none() { return 1.0; }
    let i = buf.idx(x, y);
    let mut m = 1.0f32;
    if !spec.soil.is_empty() && buf.terrain[i] == 1 {
        let sc = buf.soil_type.get(i).copied().unwrap_or(0);
        // Soil is a PREFERENCE, never a veto — two deliberate rules:
        //  • An UNCLASSIFIED cell (class 0, or a world generated before phase 6
        //    ran) scores 1.0: no information is not the same as bad ground, the
        //    same discipline the campaign applies to an empty kin roster.
        //  • A classified-but-unlisted soil keeps `SOIL_UNLISTED`, not zero.
        //    Vetoing would let one term silently delete a good's entire belt —
        //    which is what a first cut of this did, and it emptied `saffron`
        //    outright. The floor still produces the intended patchiness (a 4x
        //    contrast between preferred and indifferent ground) while keeping the
        //    "a good must never silently vanish" rule §8.16 already holds
        //    minerals to.
        m *= if sc == 0 {
            1.0
        } else {
            spec.soil.iter().find(|(c, _)| *c == sc).map(|(_, w)| *w).unwrap_or(SOIL_UNLISTED)
        };
    }
    if let Some([lo, hi, e]) = spec.relief {
        m *= band(local_relief(buf, x, y), lo, hi, e);
    }
    // Remap into [TERROIR_FLOOR, 1.0] — see the constant for why this floor is
    // the whole safety mechanism.
    TERROIR_FLOOR + (1.0 - TERROIR_FLOOR) * m.clamp(0.0, 1.0)
}

/// Local RELIEF at a cell: the greatest normalized-elevation drop between the
/// cell and any neighbour within 2 cells. A slope measure, deliberately NOT an
/// altitude measure — terraced vines and hill tea want a slope at any height,
/// paddy rice and cereal want flat ground at any height, and `elevation` alone
/// cannot distinguish those two cases.
///
/// Cheap by construction: a bounded 5x5 max over a field already in memory, only
/// evaluated for goods that actually declare an `Envelope::relief` band, so the
/// default path costs nothing (§8.9 rule 1 — no outward scan per cell in the
/// general case).
fn local_relief(buf: &WorldBuffer, x: u32, y: u32) -> f32 {
    let e0 = buf.elevation[buf.idx(x, y)];
    let mut lo = e0;
    let mut hi = e0;
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            let ni = buf.widx(x as i32 + dx, y as i32 + dy);
            if buf.terrain[ni] != 1 { continue; }
            let e = buf.elevation[ni];
            if e < lo { lo = e; }
            if e > hi { hi = e; }
        }
    }
    hi - lo
}

/// Weight kept by a classified soil class a good does not list. A preference, not
/// a veto — see `terroir_multiplier`.
const SOIL_UNLISTED: f32 = 0.25;

/// The least a good's score may be reduced to by the fine-grain terroir terms.
///
/// Terroir shapes a belt's TEXTURE; it must not decide whether the belt exists.
/// A first cut applied soil x relief as a raw multiplier and pushed `tea` and
/// `saffron` — whose climates were already marginal on the diagnostic world —
/// under `localize_good`'s seed threshold, so both placed literally nothing. That
/// is the same failure mode the locality pass already guards with its own FRINGE
/// and FLOOR (CLAUDE.md §8.19 D5), and it gets the same answer here: the
/// multiplier is remapped into `[TERROIR_FLOOR, 1.0]`, which still gives roughly a
/// 2x contrast between preferred and indifferent ground — plenty for visible
/// patchiness — while never being able to delete a good from a world.
const TERROIR_FLOOR: f32 = 0.45;

/// How many of the world's smallest landmasses an endemic good may fall back to
/// when no true island carries it (see `localize_good`'s `island_relax`).
const SMALLEST_LANDMASS_FALLBACK: usize = 6;

/// The largest a landmass may be and still count as an ISLAND, in km². Stated in
/// km² and converted per world, never in cells, for the same reason the locality
/// size ladder is (CLAUDE.md §8.19, the size ladder): a cell is ~11 km across
/// at 3600x1800 but ~133 km at the sizes the test worlds use, so a fixed CELL
/// count would mean "Great Britain" on one world and "most of Eurasia" on
/// another. ~250,000 km² is roughly Great Britain plus Ireland — generous on
/// purpose, because the failure it guards against (no landmass anywhere
/// qualifies, so every endemic good silently vanishes) is far worse than an
/// endemic occasionally landing on a large island.
pub const ISLAND_MAX_KM2: f32 = 250_000.0;

/// Connected-component labelling of the world's LAND — the pass this codebase
/// never had.
///
/// Until now `Domain::Island` was approximated as `distance_to_ocean < 0.20`,
/// which is *near-coast land*, not an island: it matched the entire coastal
/// fringe of every continent. So an "island" good was really a coastal good, and
/// a true island endemic — the single most valuable structure in the pre-modern
/// spice trade — could not be expressed at all.
///
/// One BFS over the whole grid, wrap-aware in X and clamped at the poles (rule
/// 6), built ONCE per world and shared by every good — the same discipline
/// `deposits::GeoContext` and `RiverContext` already follow. Never a per-good
/// scan (§8.9 rule 1).
pub struct LandmassContext {
    /// Component id per cell; `u32::MAX` for sea.
    pub id: Vec<u32>,
    /// Cell count per component, indexed by component id.
    pub area: Vec<u32>,
    /// `ISLAND_MAX_KM2` converted to this world's cells (see the const).
    pub island_max_cells: u32,
}

impl LandmassContext {
    pub fn build(buf: &WorldBuffer) -> LandmassContext {
        let w = buf.width;
        let h = buf.height;
        let n = buf.total();
        let mut id = vec![u32::MAX; n];
        let mut area: Vec<u32> = Vec::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        for start in 0..n {
            if buf.terrain[start] != 1 || id[start] != u32::MAX { continue; }
            let comp = area.len() as u32;
            let mut count = 0u32;
            id[start] = comp;
            queue.clear();
            queue.push_back(start);
            while let Some(ci) = queue.pop_front() {
                count += 1;
                let cx = (ci as u32 % w) as i32;
                let cy = (ci as u32 / w) as i32;
                // 8-connected: a diagonal land step keeps an isthmus or a chain of
                // skerries as ONE landmass, which is what a walker (or a nutmeg
                // tree) experiences. 4-connectivity would split Denmark from itself.
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let ny = cy + dy;
                        if ny < 0 || ny >= h as i32 { continue; }
                        let ni = buf.idx(buf.wrap_x(cx + dx), ny as u32);
                        if buf.terrain[ni] == 1 && id[ni] == u32::MAX {
                            id[ni] = comp;
                            queue.push_back(ni);
                        }
                    }
                }
            }
            area.push(count);
        }
        // A cell's area in km²: the equatorial circumference divided by the grid
        // width gives its width; cells are square in this projection's own units.
        let km_per_cell = 40075.0f32 / w.max(1) as f32;
        let cell_km2 = (km_per_cell * km_per_cell).max(1.0);
        let island_max_cells = (ISLAND_MAX_KM2 / cell_km2).round().max(1.0) as u32;
        LandmassContext { id, area, island_max_cells }
    }

    /// Cells in the landmass containing `i`, or 0 for a sea cell.
    pub fn area_at(&self, i: usize) -> u32 {
        match self.id.get(i).copied() {
            Some(c) if c != u32::MAX => self.area.get(c as usize).copied().unwrap_or(0),
            _ => 0,
        }
    }

    /// Is this cell on a landmass small enough to count as an island?
    pub fn is_island(&self, i: usize) -> bool {
        let a = self.area_at(i);
        a > 0 && a <= self.island_max_cells
    }

    /// Is this cell on one of the world's `k` SMALLEST landmasses? The relaxation
    /// an endemic good falls back to when no landmass clears `is_island` — a world
    /// whose islands are all slightly too big must still be able to grow nutmeg
    /// somewhere, and "the smallest land there is" is the honest answer. Ranking
    /// is by area then component id, so it is deterministic.
    pub fn is_among_smallest(&self, i: usize, k: usize) -> bool {
        let Some(c) = self.id.get(i).copied().filter(|&c| c != u32::MAX) else { return false };
        let mut order: Vec<(u32, u32)> =
            self.area.iter().copied().enumerate().map(|(idx, a)| (a, idx as u32)).collect();
        order.sort();
        order.iter().take(k).any(|&(_, idx)| idx == c)
    }
}


/// Place every good's belt. Returns the discrete ORE WORKINGS (see `sim::deposits`)
/// — the per-deposit grade / extent / depth that the u8 belt column cannot carry.
/// The belt column is still written exactly as before, so every existing reader,
/// the overlay and the v2 blob format are untouched (rule 7).
/// One good's line in the post-generation placement report.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct GoodPlacementRow {
    pub id: String,
    pub name: String,
    pub icon: String,
    /// "global" | "local" | "endemic" | "deposits" | "manufactured".
    pub distribution: String,
    pub category: String,
    /// Cells carrying any production, and that as a share of the world's LAND
    /// (or, for a marine good, of its sea).
    pub cells: u32,
    pub land_share: f32,
    /// Independent homelands actually seeded (`GoodSpec::origins` is the request;
    /// this is what the world could deliver).
    pub origins: u8,
    pub localities: u32,
    /// Named (notable-grade) localities, for the "subcategories" reading.
    pub notable: Vec<String>,
    /// Mean belt value where present, 0..1 — the good's typical QUALITY here.
    pub mean_grade: f32,
    /// Empty when placement is healthy; otherwise why it is not.
    pub flags: Vec<String>,
}

/// The whole report. `absent` and `flagged` are the point of it: a good that
/// silently failed to place used to be invisible until someone went looking for
/// it on the map.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct GoodsPlacementReport {
    pub rows: Vec<GoodPlacementRow>,
    pub enabled: u32,
    pub placed: u32,
    pub absent: u32,
    pub flagged: u32,
}

/// Flag kinds, as stable strings the frontend can style. Kept as consts so the
/// report and any future test agree on the vocabulary.
pub const FLAG_ABSENT: &str = "absent";
pub const FLAG_FALLBACK: &str = "fallback_seed";
pub const FLAG_UBIQUITOUS: &str = "ubiquitous";
pub const FLAG_SINGLE_CELL: &str = "single_cell";

pub fn compute_trade_goods(
    buf: &mut WorldBuffer, rivers: &[River], seed: u64, gem_deposits: u32,
    climate_strictness: f32, specs: &[GoodSpec],
) -> (
    Vec<crate::sim::deposits::Deposit>,
    Vec<super::localities::GoodLocality>,
    GoodsPlacementReport,
) {
    use crate::sim::deposits::{self, DepositModel, GeoContext, MineralPlan};
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // CLAUDE.md §8.19 (goods localities, shipped) Slice 1 (F6) — built ONCE for the whole world, the
    // same discipline `GeoContext` already applies for deposit goods. `None` for
    // every OTHER caller of `good_score`/`envelope_score` (the Goods Editor's live
    // preview, which has no rivers to hand) — see `river_multiplier`'s neutral
    // fallback.
    let river_ctx = Some(RiverContext::build(buf, rivers));
    // Connected-component labelling of the land, built ONCE for the whole world
    // (same discipline as `RiverContext`/`GeoContext`). Needed by `Domain::Island`
    // and by every `Distribution::Endemic` good.
    let land_ctx = LandmassContext::build(buf);
    // CLAUDE.md §8.19 (goods localities, shipped) Slice 3 — every locality placed this run, across
    // every good, flattened for persistence exactly as the ore workings are.
    let mut all_localities: Vec<super::localities::GoodLocality> = Vec::new();

    // Climate strictness sharpens (or softens) every good's suitability score
    // before placement: gamma > 1 tightens belts toward their ideal climate
    // (more clustered, rarer), gamma < 1 lets them sprawl. 0.5 = neutral.
    let gamma = (0.55 + climate_strictness.clamp(0.0, 1.0) * 0.90).clamp(0.30, 2.0);
    let shape = |s: f32| -> f32 { if (gamma - 1.0).abs() < 1e-3 { s } else { s.clamp(0.0, 1.0).powf(gamma) } };

    // Size the buffer's good columns to the active spec list (grow for custom
    // goods, drop trailing columns that no longer exist).
    buf.goods.resize(specs.len().max(1), vec![0u8; n]);


    // Seed cells of the homelands placed so far, so each new Local good is pushed
    // to a DIFFERENT part of the continent (plausible spread, not all clustered).
    let mut placed_seeds: Vec<usize> = Vec::new();
    // Which landmass each ENDEMIC good took, with the fingerprint of the
    // suitability field that chose it. Two goods sharing a fingerprint are two
    // products of one tree (nutmeg/mace) and share a home; everything else is
    // pushed to an island of its own. See `score_signature`.
    let mut endemic_claims: Vec<(u32, u64)> = Vec::new();
    // Goods whose FIRST homeland could not clear the seed threshold anywhere and
    // had to fall back to the least-bad cell on the map. That is the honest
    // signal "this world may have no suitable climate for this good" — before the
    // report it was completely silent, and the good simply appeared somewhere
    // implausible.
    let mut fallback_seeded: std::collections::BTreeSet<String> = Default::default();

    // ── GEOLOGICAL CONTEXT for deposit goods ────────────────────────────────
    // Built ONCE for the whole world (a per-good BFS across 45 goods would be a
    // real cost) and only when some deposit good actually exists. Holds the shared
    // distance fields every ore model reads; dropped before `save`.
    let any_deposit = specs
        .iter()
        .any(|s| s.enabled && matches!(s.distribution, Distribution::Deposits));
    let geo: Option<GeoContext> = any_deposit.then(|| GeoContext::build(buf, rivers));

    // Every working placed so far, so a DERIVED mineral (turquoise weathering out
    // of a copper body) can find its parent. Keyed by good id.
    let mut placed_deposits: std::collections::BTreeMap<String, Vec<deposits::Deposit>> =
        std::collections::BTreeMap::new();
    // Deposit goods whose model is derived, deferred to a second pass below.
    let mut derived_slots: Vec<usize> = Vec::new();

    for slot in 0..specs.len() {
        let spec = match specs.get(slot) {
            Some(s) if s.enabled => s,
            _ => { buf.goods[slot] = vec![0u8; n]; continue; }
        };
        let builtin_idx = builtin_index_of(&spec.id);

        match spec.distribution {
            Distribution::Deposits => {
                // ── GEOLOGICAL PLACEMENT (see `sim::deposits`) ──────────────────
                // A mineral's DEPOSIT MODEL — arc, orogen, craton, rift, platform,
                // contact-metamorphic, evaporite, bog, coastal, placer, weathering —
                // decides where it goes, from the plate boundaries and volcanism
                // phase 1 already computed and this placer used to ignore entirely.
                //
                // Districts, not dots: the count below is now the number of ORE
                // DISTRICTS, each of which scatters several workings across a real
                // camp-sized radius.
                let (num, den) = spec.deposit.as_ref()
                    .map(|d| (d.count_num.max(1), d.count_den.max(1)))
                    .unwrap_or((1, 1));
                let dp = builtin_idx.and_then(deposit_params);
                let salt = dp.map(|d| d.salt).unwrap_or_else(|| id_salt(&spec.id));
                let (def_model, def_placer) = deposits::default_model_for(&spec.id);
                let model = spec.deposit.as_ref().and_then(|d| d.model).unwrap_or(def_model);
                let placer_frac = spec.deposit.as_ref()
                    .and_then(|d| d.placer_frac)
                    .unwrap_or(def_placer);
                let parent = spec.deposit.as_ref()
                    .and_then(|d| d.parent.clone())
                    .or_else(|| deposits::default_parent_for(&spec.id).map(|s| s.to_string()));

                // A derived mineral needs its parent placed first — defer it.
                if model.is_derived() && model != DepositModel::Placer {
                    derived_slots.push(slot);
                    buf.goods[slot] = vec![0u8; n];
                    continue;
                }

                let Some(ctx) = geo.as_ref() else {
                    buf.goods[slot] = vec![0u8; n];
                    continue;
                };
                let plan = MineralPlan {
                    id: &spec.id,
                    model,
                    placer_frac,
                    parent: parent.as_deref(),
                    districts: (gem_deposits * num / den).max(1),
                    salt,
                    rarity: spec.rarity,
                };
                let placement = deposits::place_mineral(buf, ctx, rivers, &plan, &[], seed);
                buf.goods[slot] = placement.belt;
                placed_deposits.insert(spec.id.clone(), placement.deposits);
                continue;
            }
            Distribution::Manufactured => {
                // Made in cities from a recipe, not extracted from the land: no belt.
                // `apply_manufacturing` produces it at hubs holding the inputs; the
                // overlay (compute_good_regions) shows the producing cities.
                buf.goods[slot] = vec![0u8; n];
            }
            _ => {
                let marine = matches!(spec.domain, Domain::Marine);
                let unlimited = matches!(spec.distribution, Distribution::Global);
                let mut score = vec![0.0f32; n];
                for y in 0..h {
                    for x in 0..w {
                        let s = if let Some(env) = &spec.scoring {
                            envelope_score(buf, env, spec.domain, x, y, river_ctx.as_ref(), spec.marine_band, Some(&land_ctx))
                        } else if let Some(idx) = builtin_idx {
                            good_score(buf, idx, x, y, river_ctx.as_ref(), spec.marine_band)
                        } else {
                            0.0
                        };
                        // FINE-GRAIN terroir, applied AFTER whichever scorer ran
                        // so it covers built-in and custom goods identically (see
                        // `GoodSpec::soil` / `::relief`). Absent by default, so a
                        // good that declares neither is scored exactly as before.
                        let s = s * terroir_multiplier(buf, spec, x, y);
                        score[buf.idx(x, y)] = shape(s);
                    }
                }
                // Built-ins reproduce their original seed by salting with index*K;
                // custom goods get an id-derived salt.
                let salt = match builtin_idx {
                    Some(idx) if spec.scoring.is_none() => (idx as u64).wrapping_mul(SEED_SALT_K),
                    _ => id_salt(&spec.id),
                };
                let endemic = matches!(spec.distribution, Distribution::Endemic);
                let best_score = score.iter().copied().fold(0.0f32, f32::max);
                let (mut belt, seed_cells, endemic_comp) = localize_good(
                    buf, &score, marine, unlimited, spec.rarity, salt, seed, &placed_seeds,
                    spec.origins, endemic, Some(&land_ctx), &endemic_claims);
                // Remember every homeland so the next good is seeded elsewhere.
                placed_seeds.extend(seed_cells.iter().copied());
                if let Some(c) = endemic_comp {
                    endemic_claims.push((c, score_signature(&score)));
                }
                // `localize_good`'s seed threshold, recomputed here so the report
                // can say WHY a good landed where it did rather than guessing.
                let seed_thresh = (0.45 + (spec.rarity - 0.5).clamp(-0.5, 0.5) * 0.30).clamp(0.20, 0.75);
                if !unlimited && !seed_cells.is_empty() && best_score < seed_thresh {
                    fallback_seeded.insert(spec.id.clone());
                }
                // CLAUDE.md §8.19 (goods localities, shipped) Slice 3 (D1/D5/D6) — cluster the belt into
                // real terroir patches and thin it between them (full modulation, with
                // a floor so a producing cell never reaches literal zero). Runs BEFORE
                // dilation so the trade-reach rings spread from the already-modulated
                // belt, not the raw pre-locality one.
                let localities = super::localities::place_localities(
                    buf, &mut belt, spec, river_ctx.as_ref(), seed);
                all_localities.extend(localities);
                // Extra reach: trade carries a good a bit past its core homeland.
                dilate_belt(buf, &mut belt, marine, 2, 0.72);
                buf.goods[slot] = belt;
            }
        }
    }

    // ── Second pass: DERIVED minerals ───────────────────────────────────────
    // A weathering deposit is a DAUGHTER of another orebody — turquoise forms
    // where copper-bearing rock alters in a desert — so it can only be placed once
    // its parent exists. Deferred here rather than reordering the main loop, which
    // would change every other good's placement seed order.
    if let Some(ctx) = geo.as_ref() {
        for slot in derived_slots {
            let Some(spec) = specs.get(slot) else { continue };
            let (def_model, def_placer) = deposits::default_model_for(&spec.id);
            let model = spec.deposit.as_ref().and_then(|d| d.model).unwrap_or(def_model);
            let placer_frac = spec.deposit.as_ref()
                .and_then(|d| d.placer_frac).unwrap_or(def_placer);
            let parent_id = spec.deposit.as_ref()
                .and_then(|d| d.parent.clone())
                .or_else(|| deposits::default_parent_for(&spec.id).map(|s| s.to_string()));
            // A derived mineral with no parent on this map yields nothing — which is
            // correct (no copper, no turquoise) and is reported, not silent.
            let parent_deposits = parent_id
                .as_deref()
                .and_then(|p| placed_deposits.get(p))
                .cloned()
                .unwrap_or_default();
            let (num, den) = spec.deposit.as_ref()
                .map(|d| (d.count_num.max(1), d.count_den.max(1)))
                .unwrap_or((1, 1));
            let plan = MineralPlan {
                id: &spec.id,
                model,
                placer_frac,
                parent: parent_id.as_deref(),
                districts: (gem_deposits * num / den).max(1),
                salt: id_salt(&spec.id),
                rarity: spec.rarity,
            };
            let placement =
                deposits::place_mineral(buf, ctx, rivers, &plan, &parent_deposits, seed);
            buf.goods[slot] = placement.belt;
            placed_deposits.insert(spec.id.clone(), placement.deposits);
        }
    }

    // Flatten into one world-wide working list, ordered by good id then position so
    // the output is stable across runs (a BTreeMap iterates in key order).
    let mut all: Vec<deposits::Deposit> = Vec::new();
    for (_, v) in placed_deposits {
        all.extend(v);
    }

    // CLAUDE.md §8.19 (goods localities, shipped) Slice 4 (D8) — name the notable localities, now that
    // Phase 7's culture map is active (see `sim_run_all`'s ordering).
    super::localities::name_notable_localities(buf, &mut all_localities);

    let report = build_placement_report(buf, specs, &all_localities, &fallback_seeded);
    (all, all_localities, report)
}


/// Build the post-generation placement report from the finished belts.
///
/// Pure read-back over `buf.goods` — it changes nothing and can never move a
/// belt. Its whole value is the two lists nobody could see before: goods that
/// placed NOTHING, and goods that placed only because the seeder fell back to the
/// least-bad cell on a world with no suitable climate for them.
fn build_placement_report(
    buf: &WorldBuffer,
    specs: &[GoodSpec],
    localities: &[super::localities::GoodLocality],
    fallback_seeded: &std::collections::BTreeSet<String>,
) -> GoodsPlacementReport {
    let n = buf.total();
    let land_cells = buf.terrain.iter().filter(|&&t| t == 1).count().max(1) as f32;
    let sea_cells = (n as f32 - land_cells).max(1.0);

    let mut report = GoodsPlacementReport::default();
    for (slot, spec) in specs.iter().enumerate() {
        if !spec.enabled { continue; }
        report.enabled += 1;
        let marine = matches!(spec.domain, Domain::Marine);
        let col = buf.goods.get(slot);
        let (cells, sum) = match col {
            Some(c) => {
                let mut cells = 0u32;
                let mut sum = 0.0f32;
                for &v in c.iter() {
                    if v > 0 { cells += 1; sum += v as f32 / 255.0; }
                }
                (cells, sum)
            }
            None => (0, 0.0),
        };
        let denom = if marine { sea_cells } else { land_cells };
        let mine: Vec<&super::localities::GoodLocality> =
            localities.iter().filter(|l| l.good == spec.id).collect();
        let mut notable: Vec<String> =
            mine.iter().filter(|l| !l.name.is_empty()).map(|l| l.name.clone()).collect();
        notable.sort();
        notable.dedup();

        let mut flags: Vec<String> = Vec::new();
        let manufactured = matches!(spec.distribution, Distribution::Manufactured);
        if cells == 0 && !manufactured {
            flags.push(FLAG_ABSENT.to_string());
            report.absent += 1;
        } else {
            if !manufactured { report.placed += 1; }
            if fallback_seeded.contains(&spec.id) { flags.push(FLAG_FALLBACK.to_string()); }
            let share = cells as f32 / denom;
            // A non-staple covering a quarter of the world is almost always a
            // scoring mistake, not a rich world.
            if share > 0.25 && spec.need_tier > 0 { flags.push(FLAG_UBIQUITOUS.to_string()); }
            if cells <= 2 && !matches!(spec.distribution, Distribution::Deposits) {
                flags.push(FLAG_SINGLE_CELL.to_string());
            }
        }
        if !flags.is_empty() && !flags.iter().any(|f| f == FLAG_ABSENT) { report.flagged += 1; }

        report.rows.push(GoodPlacementRow {
            id: spec.id.clone(),
            name: spec.name.clone(),
            icon: spec.icon.clone(),
            distribution: match spec.distribution {
                Distribution::Global => "global",
                Distribution::Local => "local",
                Distribution::Endemic => "endemic",
                Distribution::Deposits => "deposits",
                Distribution::Manufactured => "manufactured",
            }
            .to_string(),
            category: spec.category.clone(),
            cells,
            land_share: cells as f32 / denom,
            origins: mine.len().min(u8::MAX as usize) as u8,
            localities: mine.len() as u32,
            notable,
            mean_grade: if cells > 0 { sum / cells as f32 } else { 0.0 },
            flags,
        });
    }
    // Absent and flagged goods first — the report exists to surface them.
    report.rows.sort_by(|a, b| {
        let rank = |r: &GoodPlacementRow| if r.flags.iter().any(|f| f == FLAG_ABSENT) { 0 }
            else if !r.flags.is_empty() { 1 } else { 2 };
        rank(a).cmp(&rank(b))
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.id.cmp(&b.id))
    });
    report
}

/// Declarative envelope scorer for custom (and overridden) goods. Reproduces the
/// house scoring style: domain gate Ã— climate Ã— temp/precip/elevation/lat bands Ã—
/// fertility Ã— coast bonus. Absent terms contribute a neutral 1.0.
fn envelope_score(
    buf: &WorldBuffer, env: &Envelope, domain: Domain, x: u32, y: u32,
    rc: Option<&RiverContext>, marine_band: MarineBand, lm: Option<&LandmassContext>,
) -> f32 {
    let i = buf.idx(x, y);
    let land = buf.terrain[i] == 1;
    match domain {
        Domain::Marine => {
            if land { return 0.0; }
            if !(buf.is_shelf[i] == 1 || has_land_within(buf, x, y, 3)) { return 0.0; }
            if !marine_band_ok(buf, x, y, marine_band) { return 0.0; }
        }
        Domain::Coastal => {
            if !land || buf.distance_to_ocean[i] >= 0.12 { return 0.0; }
        }
        Domain::Continental => {
            if !land { return 0.0; }
        }
        Domain::Island => {
            if !land { return 0.0; }
            match lm {
                // A REAL island now (`LandmassContext`), not the old near-coast
                // approximation — which matched the entire coastal fringe of every
                // continent, so an "island" good was really just a coastal good.
                Some(l) => if !l.is_island(i) { return 0.0; },
                // No landmass context: the Goods Editor's live preview, which has
                // no world to label. Fall back to the old approximation rather
                // than scoring nothing, so the preview still shows something.
                None => if buf.distance_to_ocean[i] >= 0.20 { return 0.0; },
            }
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
        // Same raw-then-folded rule `good_score` uses: an author who listed the
        // humid zone (`CFB`) and not its dry-winter twin (`CWB`) still scores in
        // the twin, but an author who listed the twin EXPLICITLY is never
        // overridden by the fold. Before this, custom goods saw raw Köppen while
        // built-ins saw folded Köppen — the same cell scored under two different
        // climate labels depending on which scorer ran.
        let w_raw = env.climate.iter().find(|(z, _)| *z == k).map(|(_, w)| *w);
        let w = w_raw.or_else(|| {
            let kf = clim_base(k);
            (kf != k).then(|| env.climate.iter().find(|(z, _)| *z == kf).map(|(_, w)| *w)).flatten()
        });
        s *= w.unwrap_or(0.0);
    }
    if let Some([c, wd]) = env.temp { s *= bell(t, c, wd); }
    if let Some([lo, hi, e]) = env.precip { s *= band(p, lo, hi, e); }
    if let Some([lo, hi, e]) = env.elevation { s *= band(elev, lo, hi, e); }
    if let Some([lo, hi, e]) = env.abs_lat { s *= band(abs_lat, lo, hi, e); }
    if env.fertility > 0.0 { s *= (1.0 - env.fertility) + env.fertility * fert; }
    if env.coast_bonus > 0.0 {
        let nearcoast = if land { buf.distance_to_ocean[i] < 0.08 } else { true };
        if nearcoast { s *= 1.0 + env.coast_bonus; }
    }
    s *= river_multiplier(rc, buf, i, env.floodplain, env.irrigation, env.riverbank, env.float_out);
    s.clamp(0.0, 1.0)
}

/// Fold the dry-winter / dry-summer-continental KÃ¶ppen variants onto their humid
/// (f) equivalents for GOODS scoring. The crops/animals don't care about winter
/// dryness â€” a dry-winter continental zone (Dwb) grows the same goods as humid
/// continental (Dfb) â€” but most `good_score` arms only list the f variants, so
/// the newly-introduced Cw*/Dw* zones came up empty. Mediterranean (Cs*) and
/// tropical savanna (As) are climatically meaningful for their goods (olives,
/// wine, etc.) and are deliberately NOT folded.
///
/// **Used as a FALLBACK only, never as a pre-pass.** `good_score` applied this
/// before its match, which made every arm naming a Cw/Dw/Ds zone unreachable and
/// scored tea and coffee at exactly 0.0 in Cwb — their home climate. See
/// `good_score`'s own doc and the `dry_winter_zones_are_reachable` gate.
fn clim_base(k: u8) -> u8 {
    match k {
        CWA => CFA, CWB => CFB, CWC => CFC,
        DWA => DFA, DWB => DFB, DWC => DFC, DWD => DFD,
        DSA => DFA, DSB => DFB, DSC => DFC, DSD => DFD,
        other => other,
    }
}

/// Raw 0..1 suitability of good `g` at one cell (before localization).
///
/// The dry-winter / dry-summer Köppen variants are scored in their **RAW** zone
/// first and folded onto the humid equivalent (`clim_base`) only as a FALLBACK.
/// Folding unconditionally — which is what this did — made every match arm below
/// that names `CWA`/`CWB`/`CWC`/`DW*`/`DS*` **unreachable**, because `k` could
/// never hold those codes by the time the match ran. That was not cosmetic: tea
/// and coffee both name `CWB` (subtropical highland, dry winter — Darjeeling,
/// Yunnan, the Ethiopian and Kenyan highlands, i.e. THE tea and coffee climate)
/// and both scored exactly **0.0** there, since `CWB` folds to `CFB` and neither
/// good lists it. They were placed by their weak fallback arms instead, which put
/// them in the wrong climates entirely. Wine's `DSA|DSB` and silk's `CWB` arms
/// were dead the same way.
///
/// Raw-first preserves `clim_base`'s original purpose (a good that genuinely
/// doesn't care about winter dryness still scores in a Cw/Dw/Ds zone via its
/// humid arm) while never zeroing a good that named the dry-winter zone
/// explicitly. Gate: `dry_winter_zones_are_reachable`.
fn good_score(
    buf: &WorldBuffer, g: usize, x: u32, y: u32,
    rc: Option<&RiverContext>, marine_band: MarineBand,
) -> f32 {
    let k_raw = buf.koppen[buf.idx(x, y)];
    let s = good_score_in_zone(buf, g, x, y, rc, marine_band, k_raw);
    if s > 0.0 { return s; }
    let k_folded = clim_base(k_raw);
    if k_folded != k_raw {
        return good_score_in_zone(buf, g, x, y, rc, marine_band, k_folded);
    }
    0.0
}

/// `good_score` evaluated against ONE specific Köppen zone code (see the two-pass
/// raw-then-folded scheme documented on `good_score`). Never call this directly.
fn good_score_in_zone(
    buf: &WorldBuffer, g: usize, x: u32, y: u32,
    rc: Option<&RiverContext>, marine_band: MarineBand, k: u8,
) -> f32 {
    let i = buf.idx(x, y);
    let land = buf.terrain[i] == 1;
    let marine = GOOD_MARINE[g];
    if marine && land { return 0.0; }
    if !marine && !land { return 0.0; }
    if marine && !marine_band_ok(buf, x, y, marine_band) { return 0.0; }

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
            // (China's Yangtze, Bengal, the Levant) â€” NOT the cold northern
            // continental interior. Dropped DFA/DFB, warmed the temperature peak,
            // and added a |lat| cap so silk stays out of the high north.
            let clim = match k { CFA | CWA => 1.0, CSA | CWB => 0.5, CFB => 0.25, _ => 0.0 };
            clim * bell(t, 21.0, 5.0) * band(p, 600.0, 1600.0, 500.0)
                * band(abs_lat, 0.0, 38.0, 8.0)
                * (0.4 + 0.6 * fert) * (1.0 - smoothstep(0.4, 0.7, elev))
        }
        GOOD_WINE => {
            // Viticulture sits in a warm-temperate band (â‰ˆ30â€“50Â°): Mediterranean
            // cores, humid-subtropical and the warm oceanic margins. We score from
            // KÃ¶ppen WHERE it tags those zones, but also from the underlying
            // CONDITIONS (warm-temperate, mid-latitude, sub-humid) so grapes still
            // appear on a world whose KÃ¶ppen classifier produced little explicit Cs.
            let clim: f32 = match k { CSA | CSB => 1.0, CFA => 0.55, CFB | DSA | DSB => 0.40, _ => 0.0 };
            let med_like = bell(t, 15.0, 6.5) * band(p, 300.0, 1000.0, 500.0) * band(abs_lat, 30.0, 50.0, 9.0);
            let suit = clim.max(0.75 * med_like);
            let hill = 0.7 + 0.3 * band(elev, 0.05, 0.35, 0.2);
            suit * (1.0 - smoothstep(0.55, 0.8, elev)) * hill
        }
        GOOD_OLIVEOIL => {
            // Olives = hot dry summers, mild winters, â‰ˆ30â€“45Â°. Peaks in KÃ¶ppen
            // Mediterranean; elsewhere fall back to the CONDITIONS (warm, dry-summer,
            // mid-latitude, low elevation, coastal preference) so olive country
            // still shows where the classifier didn't stamp an explicit Cs zone.
            let clim: f32 = match k { CSA => 1.0, CSB => 0.85, CFA => 0.2, BSH | BSK => 0.3, _ => 0.0 };
            // dry-summer warm-temperate signature
            let med_like = smoothstep(15.0, 19.0, t) * (1.0 - smoothstep(30.0, 38.0, t))
                * (1.0 - smoothstep(800.0, 1200.0, p)) * band(abs_lat, 30.0, 45.0, 8.0);
            let suit = clim.max(0.8 * med_like);
            let low = 1.0 - smoothstep(0.40, 0.65, elev);
            suit * low * (0.75 + 0.25 * if coastland { 1.0 } else { 0.0 })
        }
        GOOD_SUGAR => {
            let clim = match k { AF | AM => 1.0, AW | AS => 0.6, CWA => 0.4, _ => 0.0 };
            clim * smoothstep(20.0, 25.0, t) * smoothstep(900.0, 1400.0, p)
                * (1.0 - smoothstep(0.18, 0.4, elev)) * (0.5 + 0.5 * fert)
                * river_multiplier(rc, buf, i, 0.30, 0.35, 0.0, 0.0)
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
                * river_multiplier(rc, buf, i, 0.0, 0.0, 0.0, 0.25)
        }
        GOOD_TIMBER => {
            let clim = match k {
                DFB | DFC | CFB => 1.0,
                DFA | DWB | DWC | CFC | DWA => 0.6,
                _ => 0.0,
            };
            clim * smoothstep(350.0, 800.0, p) * (0.4 + 0.6 * fert) * band(t, -5.0, 18.0, 8.0)
                * river_multiplier(rc, buf, i, 0.0, 0.0, 0.0, 0.35)
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
                CSA | CSB => 1.0,                 // Mediterranean â€” prime wheat
                BSK | CFA | CFB => 0.7,           // steppe margin / humid-subtropical / oceanic
                DFA | DFB | DSA | DSB => 0.6,     // continental grain belt
                BSH | CWA => 0.4,                 // hot steppe / dry-winter subtropical
                _ => 0.0,
            };
            let warm = bell(t, 15.0, 9.0);
            let dryish = band(p, 300.0, 900.0, 450.0); // grain likes semi-arid to subhumid
            let low = 1.0 - smoothstep(0.45, 0.7, elev);
            clim * warm * dryish * low * (0.5 + 0.5 * fert)
                * river_multiplier(rc, buf, i, 0.20, 0.0, 0.0, 0.0)
        }
        GOOD_IRON => {
            // Ore in hill country and mountain margins (not the highest peaks, not
            // the flats). Any non-frozen climate. Unlimited â€” many producers.
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
                * river_multiplier(rc, buf, i, 0.30, 0.40, 0.0, 0.0)
        }
        GOOD_HARDWOODS => {
            // Tropical rainforest export wood (ebony / mahogany / teak). Fills the
            // gap left by `timber`, which is boreal/temperate only.
            let clim = match k { AF | AM => 1.0, AW => 0.5, CWA => 0.3, _ => 0.0 };
            clim * smoothstep(800.0, 1800.0, p) * (0.4 + 0.6 * fert)
                * (1.0 - smoothstep(0.40, 0.65, elev))
                * river_multiplier(rc, buf, i, 0.0, 0.0, 0.0, 0.30)
        }
        GOOD_HORSES => {
            // Open semi-arid grassland / steppe horse country.
            let clim = match k { BSK | BSH => 1.0, CFB | DFB | DSB | CWB => 0.5, BWK => 0.3, _ => 0.0 };
            clim * (1.0 - smoothstep(0.45, 0.7, elev)) * band(p, 250.0, 700.0, 350.0)
                * bell(t, 12.0, 12.0)
        }
        GOOD_WOOL_FLEECE => {
            // Cool, wet oceanic uplands â€” sheep fleece.
            let clim = match k { CFB | CFC => 1.0, CSB | DFB | ET => 0.5, CWB => 0.4, _ => 0.0 };
            clim * band(elev, 0.10, 0.50, 0.25) * band(p, 600.0, 1600.0, 500.0)
                * band(t, 4.0, 14.0, 7.0)
        }
        GOOD_WOOL_LLAMA => {
            // Dry-winter highland camelid wool â€” a distinct homeland (different
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
            // island. Seeded â†’ one fabled spice-island homeland.
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
            (papyrus * river_multiplier(rc, buf, i, 0.0, 0.0, 0.30, 0.0))
                .max(bamboo).max(manufactured)
        }
        GOOD_CERAMICS => {
            // Porcelain / fine pottery â€” a *manufactured* good of skilled cities
            // sitting on good potter's clay (alluvial, well-watered lowland).
            // Climate-independent; driven by habitability (settlement skill) and
            // a clay proxy (fertility on low ground).
            let skill = smoothstep(0.45, 0.78, buf.habitability[i]);
            let clay = (0.30 + 0.70 * fert) * (1.0 - smoothstep(0.35, 0.60, elev));
            skill * clay
        }
        GOOD_GLASSWARE => {
            // Glass â€” skilled cities working silica sand (coastal dunes / arid
            // quartz sand) with ample fuel. Settlement-driven + a sand proxy
            // (warm coast or desert margin).
            let skill = smoothstep(0.45, 0.78, buf.habitability[i]);
            let sand = if coast_near { 1.0 }
                else { match k { BWH | BSH | BWK | BSK => 0.7, _ => 0.3 } };
            skill * sand * (1.0 - smoothstep(0.45, 0.7, elev))
        }
        GOOD_TOBACCO => {
            // Warm, humid-subtropical / tropical-savanna cash crop on fertile
            // low ground. Seeded â†’ one New-World-style plantation homeland.
            let clim = match k { CFA | CWA => 1.0, AW | CSA => 0.5, BSH => 0.3, _ => 0.0 };
            clim * smoothstep(16.0, 22.0, t) * band(p, 700.0, 1600.0, 500.0)
                * (1.0 - smoothstep(0.35, 0.6, elev)) * (0.4 + 0.6 * fert)
        }
        GOOD_INDIGO => {
            // Indigo dye plant â€” hot, wet tropical/subtropical lowland. A LAND dye
            // distinct from the marine murex "dyes". Seeded.
            let clim = match k { AW | CWA => 1.0, AM | AF | CFA => 0.5, BSH => 0.3, _ => 0.0 };
            clim * smoothstep(19.0, 26.0, t) * band(p, 800.0, 2000.0, 600.0)
                * (1.0 - smoothstep(0.35, 0.6, elev)) * (0.4 + 0.6 * fert)
                * river_multiplier(rc, buf, i, 0.30, 0.0, 0.0, 0.0)
        }
        GOOD_DATES => {
            // Date palms â€” hot desert OASIS fruit: hot arid climate but locally
            // watered (fertility = oasis/wadi). Seeded.
            let clim = match k { BWH => 1.0, BSH => 0.7, BWK | BSK => 0.3, _ => 0.0 };
            let oasis = 0.25 + 0.75 * fert;
            clim * smoothstep(18.0, 26.0, t) * band(abs_lat, 12.0, 34.0, 8.0)
                * oasis * (1.0 - smoothstep(0.4, 0.65, elev))
                * river_multiplier(rc, buf, i, 0.0, 0.45, 0.0, 0.0)
        }
        GOOD_RICE => {
            // Paddy rice: warm, wet, low alluvial land (monsoon river plains and
            // wet tropics). The everyday cereal of the warm-wet world. Unlimited.
            let clim = match k { CWA | CFA => 1.0, AF | AM => 0.8, AW => 0.6, _ => 0.0 };
            let warm = smoothstep(18.0, 24.0, t);
            let wet = smoothstep(900.0, 1500.0, p);
            let low = 1.0 - smoothstep(0.18, 0.4, elev);
            clim * warm * wet * low * (0.3 + 0.7 * fert)
                * river_multiplier(rc, buf, i, 0.40, 0.0, 0.20, 0.0)
        }
        GOOD_BARLEY => {
            // Barley & rye: the cool-belt bread grains â€” they ripen where wheat
            // struggles (short summers, oceanic damp, continental cold). Unlimited.
            let clim = match k {
                DFB | DFC => 1.0, CFB | DFA => 0.8, CSB | DSB | CFC => 0.5, ET => 0.15,
                _ => 0.0,
            };
            let cool = bell(t, 7.0, 7.0);
            clim * cool * band(p, 250.0, 900.0, 400.0)
                * (1.0 - smoothstep(0.5, 0.75, elev)) * (0.4 + 0.6 * fert)
        }
        GOOD_MILLET => {
            // Millet & sorghum: the drought grains of the steppe and savanna
            // margins where neither wheat nor rice will carry a town. Unlimited.
            let clim = match k { BSH | BSK => 1.0, AW | AS => 0.6, CWA => 0.5, BWH => 0.2, _ => 0.0 };
            let warm = bell(t, 19.0, 8.0);
            clim * warm * band(p, 200.0, 650.0, 300.0)
                * (1.0 - smoothstep(0.45, 0.7, elev)) * (0.4 + 0.6 * fert)
        }
        GOOD_HONEY => {
            // Forest honey & beeswax: temperate woodland and meadow with a real
            // flowering season. Unlimited â€” every wooded province keeps bees.
            let clim = match k {
                CFB | DFA | DFB => 1.0, CFA | CWB => 0.7, CSA | CSB | DWB => 0.5,
                DFC => 0.3, _ => 0.0,
            };
            clim * band(t, 6.0, 20.0, 7.0) * smoothstep(400.0, 750.0, p) * (0.4 + 0.6 * fert)
                * river_multiplier(rc, buf, i, 0.0, 0.0, 0.20, 0.0)
        }
        GOOD_HIDES => {
            // Hides & leather: pastoral grassland and savanna herds (and the
            // continental margins where ranching beats farming). Unlimited.
            let clim = match k {
                BSK | BSH => 1.0, AW | AS => 0.7, DFA | DFB | CFB => 0.4, CSA | CSB => 0.35,
                _ => 0.0,
            };
            clim * band(p, 200.0, 750.0, 300.0) * (1.0 - smoothstep(0.5, 0.72, elev))
                * band(t, 2.0, 24.0, 8.0)
                * river_multiplier(rc, buf, i, 0.0, 0.0, 0.18, 0.0)
        }
        GOOD_BEER => {
            // Beer & ale: famed brewing towns in cool grain-and-water country
            // (one renowned homeland â€” every village brews, only one exports).
            let clim = match k { CFB | DFA | DFB => 1.0, CFA => 0.6, DSA | DSB => 0.4, _ => 0.0 };
            clim * bell(t, 9.0, 8.0) * smoothstep(450.0, 800.0, p)
                * (0.4 + 0.6 * fert) * (1.0 - smoothstep(0.45, 0.7, elev))
        }
        // â”€â”€ Marine goods (no walls; the score envelope itself bounds the belt) â”€â”€
        GOOD_HERRING => {
            // Everyday herring/sardine shoals: temperate shelf seas a step warmer
            // and broader than the stockfish banks â€” the cheap fish of the
            // common table. Unlimited.
            if !sea_coastal { return 0.0; }
            let shelf = if buf.is_shelf[i] == 1 { 1.0 } else { 0.3 };
            let cool = bell(t, 10.0, 6.0);
            let fish = 0.25 + 0.75 * smoothstep(0.10, 0.45, buf.fishery[i].clamp(0.0, 1.0));
            shelf * cool * fish * band(abs_lat, 35.0, 62.0, 10.0)
        }
        GOOD_STOCKFISH => {
            // Stockfish (dried cod) comes off the rich NORTHERN fishing banks â€”
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
            // the water warms below productivity â€” a temperature envelope.
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
/// Place a good's belt. Returns the belt column and EVERY homeland seed cell it
/// used (one per origin; empty for a `Global` good, which has no homeland).
///
/// `origins` is how many independent homelands to seed (1 = the historical
/// behaviour). `endemic` confines the good to a single small landmass and
/// disables the island-jump, which is the difference between "rare" and "grows
/// in exactly one place on Earth" — see `Distribution::Endemic`.
/// A deterministic fingerprint of a good's suitability field, used ONLY to tell
/// "the same plant, sold as two products" from "a different plant that happens to
/// like the same weather".
///
/// Nutmeg and mace are the aril and the seed of ONE tree and are shipped sharing
/// an envelope deliberately, so they must land on the same island. Every other
/// pair of endemics must not. Comparing the quantized score fields answers that
/// exactly, with no new spec field and no hard-coded pair table: identical
/// envelopes produce identical fields, and anything else does not.
fn score_signature(score: &[f32]) -> u64 {
    let mut hsh: u64 = 0xcbf29ce484222325;
    for (i, &v) in score.iter().enumerate() {
        let qv = (v.clamp(0.0, 1.0) * 255.0) as u8;
        if qv == 0 { continue; }
        hsh ^= (i as u64) ^ ((qv as u64) << 56);
        hsh = hsh.wrapping_mul(0x100000001b3);
    }
    hsh
}

#[allow(clippy::too_many_arguments)]
fn localize_good(
    buf: &WorldBuffer, score: &[f32], marine: bool, unlimited: bool, rarity: f32, salt: u64, seed: u64,
    existing: &[usize], origins: u8, endemic: bool, lm: Option<&LandmassContext>,
    endemic_claims: &[(u32, u64)],
) -> (Vec<u8>, Vec<usize>, Option<u32>) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let mut out = vec![0u8; n];

    // Rarity gently shifts the thresholds around the neutral 0.5 (rarer â†’ harder
    // to seed and spread, so a smaller belt). At 0.5 these are the legacy values.
    let r = (rarity - 0.5).clamp(-0.5, 0.5);
    let seed_thresh = (0.45 + r * 0.30).clamp(0.20, 0.75);
    let spread_thresh = (0.22 + r * 0.20).clamp(0.10, 0.50);
    // Harsh rule for extreme-rare goods: hard-cap the homeland to a small patch, so
    // a prized luxury (saffron, Tyrian purple, jadeâ€¦) stays genuinely scarce no
    // matter how much suitable land exists.
    let cap: usize = if rarity > 0.78 {
        (((1.0 - rarity).max(0.05)) * 1800.0 + 30.0) as usize
    } else { usize::MAX };

    // ── ENDEMIC: pick the ONE landmass this good lives on ─────────────────────
    // The smallest landmass that carries any positive score for it — preferring a
    // true island (`is_island`) when one qualifies, and otherwise taking the
    // smallest scoring landmass there is.
    //
    // Choosing the target up front, rather than filtering per cell against a size
    // threshold, is what makes the guarantee unconditional: if this good's climate
    // exists ANYWHERE on this world, it gets exactly one home, and if it exists
    // nowhere it is honestly absent (and the report says so). A first cut filtered
    // on a size threshold alone and all six shipped endemics measured ZERO cells —
    // a silent total failure of the feature, which is the exact outcome §8.16's
    // "a mineral must never silently vanish" rule exists to prevent.
    let endemic_comp: Option<u32> = if endemic {
        let sig = score_signature(score);
        // TWO PRODUCTS OF ONE TREE share a home. Checked first, so the dispersion
        // rule below can never separate nutmeg from mace.
        if let Some(&(c, _)) = endemic_claims.iter().find(|&&(_, s)| s == sig) {
            Some(c)
        } else {
        lm.and_then(|l| {
            let mut best: Option<(u8, u8, u32, u32)> = None; // (claimed, rank, area, comp)
            let mut seen: std::collections::BTreeSet<u32> = Default::default();
            for i in 0..n {
                if buf.terrain[i] != 1 || score[i] <= 0.0 { continue; }
                let Some(c) = l.id.get(i).copied().filter(|&c| c != u32::MAX) else { continue };
                if !seen.insert(c) { continue; }
                // An island ANOTHER endemic already lives on is the last resort.
                //
                // Without this the six shipped endemics all pile onto one island:
                // they share a wet-tropical coastal envelope, so they score on the
                // same landmasses, and "smallest scoring island" is a deterministic
                // function of the world alone — every one of them picks the SAME
                // answer. The reported "benzoin and other rare goods are placed on
                // the same island" is exactly that, and it is also historically
                // backwards: Banda nutmeg, Sumatran benzoin, Bornean camphor,
                // Timorese sandalwood and Socotran dragon's blood are five islands,
                // and their separateness is the whole reason each was worth a
                // voyage.
                //
                // It stays a PREFERENCE, never a filter: if every scoring landmass
                // is taken, the good still gets one rather than vanishing — the
                // unconditional guarantee this block's own comment is built on, and
                // the rule §8.16 states as "a mineral must never silently vanish".
                let claimed = u8::from(endemic_claims.iter().any(|&(cc, _)| cc == c));
                // rank 0 = a true island, 1 = anything else: a real island always
                // beats a smaller-but-continental landmass.
                let rank = if l.is_island(i) { 0u8 } else { 1u8 };
                let area = l.area.get(c as usize).copied().unwrap_or(u32::MAX);
                let cand = (claimed, rank, area, c);
                if best.map(|b| cand < b).unwrap_or(true) { best = Some(cand); }
            }
            best.map(|(_, _, _, c)| c)
        })
        }
    } else {
        None
    };

    let passable = |i: usize| -> bool {
        if marine {
            buf.terrain[i] == 0
        } else if !(buf.terrain[i] == 1 && buf.elevation[i] < MOUNTAIN_NORM) {
            false
        } else if endemic {
            // Confined to its ONE chosen landmass. Combined with the disabled
            // island-jump below, the belt physically cannot reach the mainland or
            // a neighbouring island — which is the whole mechanism.
            match (endemic_comp, lm) {
                (Some(c), Some(l)) => l.id.get(i).copied() == Some(c),
                _ => false,
            }
        } else {
            true
        }
    };

    // â”€â”€ UNLIMITED goods: every suitable cell produces (many producers) â”€â”€
    if unlimited {
        for i in 0..n {
            if passable(i) && score[i] >= spread_thresh {
                out[i] = q(score[i]);
            }
        }
        return (out, Vec::new(), endemic_comp);
    }

    // Dispersion: push each homeland AWAY from those already placed — both the
    // homelands of OTHER goods (`existing`) and the earlier origins of THIS good —
    // so the world's goods spread plausibly instead of all piling into the single
    // most-suitable region, and a multi-origin good's origins land in genuinely
    // different parts of the world rather than adjacent to each other.
    let disp_r = (w as f32 * 0.16).max(8.0);
    let wrapdx = |a: i32, b: i32| -> i32 { let mut d = (a - b).abs(); if d > w as i32 / 2 { d = w as i32 - d; } d };
    let dispersion = |i: usize, others: &[usize]| -> f32 {
        if others.is_empty() { return 1.0; }
        let cx = (i as u32 % w) as i32;
        let cy = (i as u32 / w) as i32;
        let mut nd = f32::INFINITY;
        for &e in others {
            let ex = (e as u32 % w) as i32;
            let ey = (e as u32 / w) as i32;
            let dx = wrapdx(cx, ex) as f32;
            let dy = (cy - ey) as f32;
            nd = nd.min((dx * dx + dy * dy).sqrt());
        }
        (nd / disp_r).clamp(0.15, 1.0) // ~0.15 right on a rival homeland -> 1.0 far away
    };

    // ── SEEDED goods: one contiguous homeland PER ORIGIN ──────────────────────
    // `origins` is 1 for every good that predates the field, in which case this is
    // exactly the old single-homeland behaviour.
    let gs = seed ^ salt;
    let n_origins = origins.max(1) as usize;
    // The extreme-rarity homeland cap is a budget for the good AS A WHOLE, split
    // between its origins — two origins of a rare good are two small patches, not
    // two full-size ones (which would double the world's supply of it).
    let cap_per_origin = if cap == usize::MAX { usize::MAX } else { (cap / n_origins).max(30) };

    // Island-jump: a seeded belt may hop a narrow sea (or, for marine goods, a
    // narrow land bridge) up to ~4% of the map width, so thin straits / island
    // chains don't chop one homeland into several disconnected patches. An
    // ENDEMIC good may never hop: that is what keeps nutmeg on the Bandas rather
    // than spreading down the whole archipelago.
    let jump = if endemic { 1 } else { ((w as f32) * 0.04).round().clamp(2.0, 80.0) as i32 };

    let mut visited = vec![false; n];
    let mut seeds: Vec<usize> = Vec::new();
    let mut queue = VecDeque::new();

    for origin in 0..n_origins {
        // Every origin repels both the other goods' homelands and this good's own
        // earlier origins.
        let mut repel: Vec<usize> = existing.to_vec();
        repel.extend(seeds.iter().copied());

        let mut best_seed = usize::MAX;
        let mut best_key = -1.0f32;
        let mut fallback = usize::MAX;
        let mut fallback_score = spread_thresh;
        for i in 0..n {
            if !passable(i) { continue; }
            if visited[i] { continue; } // already inside an earlier origin's belt
            let s = score[i];
            if s > fallback_score { fallback_score = s; fallback = i; }
            if s >= seed_thresh {
                // The origin index enters the hash, so origin 2 is not simply the
                // runner-up of origin 1's ranking — it is its own draw.
                let key = s * dispersion(i, &repel)
                    * hash01(gs ^ (origin as u64).wrapping_mul(0x9E3779B97F4A7C15)
                        ^ (i as u64).wrapping_mul(0x100000001B3));
                if key > best_key { best_key = key; best_seed = i; }
            }
        }
        let seed_cell = if best_seed != usize::MAX {
            best_seed
        } else if origin == 0 && fallback != usize::MAX {
            // Only the FIRST origin may fall back to the least-bad cell (the
            // "a mineral must never silently vanish" rule, applied to belts). A
            // second origin that cannot clear the threshold simply does not exist
            // in this world, which is the honest answer — not a duplicate homeland
            // shoved onto marginal ground.
            fallback
        } else {
            break;
        };

        seeds.push(seed_cell);
        visited[seed_cell] = true;
        queue.clear();
        queue.push_back(seed_cell);
        let mut placed = 0usize;
        while let Some(ci) = queue.pop_front() {
            out[ci] = q(score[ci]);
            placed += 1;
            if placed >= cap_per_origin { break; }
            let cx = (ci as u32 % w) as i32;
            let cy = (ci as u32 / w) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                // Step 1 = ordinary neighbour; steps 2..=jump = water/land gap hops.
                for k in 1..=jump {
                    let nx = buf.wrap_x(cx + dx * k);
                    let ny = cy + dy * k;
                    if ny < 0 || ny >= h as i32 { break; }
                    let ni = buf.idx(nx, ny as u32);
                    if !passable(ni) { continue; } // still in the gap - keep probing
                    if !visited[ni] && score[ni] >= spread_thresh {
                        visited[ni] = true;
                        queue.push_back(ni);
                    }
                    break; // first passable cell along this ray decides the ray
                }
            }
        }
    }
    (out, seeds, endemic_comp)
}



/// Grow a placed belt outward by `rings` cells at decaying intensity, so the
/// good's production reaches **a bit further** than its core homeland (trade lets
/// a region's staple spread to its near hinterland / along the coast). Modest by
/// design â€” a couple of rings â€” and bounded by the same passability rule as the
/// belt (sea/mountains for land goods). Deposit goods are not dilated.
fn dilate_belt(buf: &WorldBuffer, out: &mut [u8], marine: bool, rings: u32, decay: f32) {
    let w = buf.width;
    let h = buf.height;
    let passable = |i: usize| -> bool {
        if marine { buf.terrain[i] == 0 }
        else { buf.terrain[i] == 1 && buf.elevation[i] < MOUNTAIN_NORM }
    };
    for _ in 0..rings {
        let src = out.to_vec();
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                if src[i] == 0 { continue; }
                let spread = (src[i] as f32 * decay) as u8;
                if spread == 0 { continue; }
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny = buf.clamp_y(y as i32 + dy);
                    let ni = buf.idx(nx, ny);
                    if passable(ni) && out[ni] < spread { out[ni] = spread; }
                }
            }
        }
    }
}

/// Deterministic hash â†’ [0,1) (splitmix64-style finalizer).
fn hash01(mut x: u64) -> f32 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CEB9FE1A85EC53);
    x ^= x >> 33;
    ((x >> 40) as f32) / 16_777_216.0
}

/// Downsampled suitability heatmap for one (possibly unsaved) good spec, used by
/// the Goods Editor to preview where a good would place before a full regen.
/// Returns a row-major `pwÃ—ph` grid of u8 scores (0..255).
pub fn preview_score_grid(buf: &WorldBuffer, spec: &GoodSpec, pw: u32, ph: u32) -> Vec<u8> {
    let builtin_idx = builtin_index_of(&spec.id);
    let mut out = vec![0u8; (pw * ph) as usize];
    if buf.width == 0 || buf.height == 0 { return out; }
    for py in 0..ph {
        for px in 0..pw {
            let x = (px * buf.width / pw).min(buf.width - 1);
            let y = (py * buf.height / ph).min(buf.height - 1);
            let s = if let Some(env) = &spec.scoring {
                envelope_score(buf, env, spec.domain, x, y, None, spec.marine_band, None)
            } else if let Some(idx) = builtin_idx {
                good_score(buf, idx, x, y, None, spec.marine_band)
            } else { 0.0 };
            out[(py * pw + px) as usize] = q(s);
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use crate::sim::elevation::fbm_noise;
    use std::collections::HashSet;
    use super::*;

    /// THE ENDEMIC DISPERSION GATE — asserted at the chooser, where it is
    /// decidable, rather than inferred from a generated world (which, at any test
    /// resolution this suite can afford, offers exactly one qualifying landmass
    /// and so cannot fail either way — see
    /// `goods_validation::endemic_homelands_diagnostic`). Same discipline as
    /// `a_division_moves_capital_and_creates_none`: assert the invariant at the
    /// mechanism, not sixty years downstream.
    ///
    /// The world is four separate islands, all equally and identically suitable.
    /// Before the claim list was threaded through `localize_good`, the chooser was
    /// a pure function of the world — "smallest scoring landmass, preferring a
    /// true island" — so every endemic returned the SAME island however many were
    /// on offer, which is the reported "benzoin and the other rare goods keep
    /// landing on one island".
    ///
    /// Verified to fail on the unfixed chooser: with `endemic_claims` ignored,
    /// all four goods report the same component.
    #[test]
    fn endemic_goods_take_different_islands() {
        // 40×10 of ocean with four 4×4 islands, well apart. Every island is the
        // same size, so nothing but the claim list can separate them — which is
        // exactly the property under test.
        let (w, h) = (40u32, 10u32);
        let mut buf = scoring_buf(w, h);
        buf.terrain = vec![0u8; (w * h) as usize];
        let islands = [2usize, 12, 22, 32];
        for &ox in &islands {
            for dy in 3..7usize { for dx in 0..4usize {
                buf.terrain[dy * w as usize + ox + dx] = 1;
            } }
        }
        let lm = LandmassContext::build(&buf);
        // Every land cell scores identically and well: the score field cannot
        // prefer one island over another.
        let score: Vec<f32> = (0..buf.total())
            .map(|i| if buf.terrain[i] == 1 { 0.9 } else { 0.0 }).collect();

        let mut claims: Vec<(u32, u64)> = Vec::new();
        let mut homes: Vec<u32> = Vec::new();
        for k in 0..4u64 {
            // Four DIFFERENT goods: same suitability shape, different seeds — so
            // they must disperse. (A good whose score field is identical to an
            // earlier one is the nutmeg/mace case and is asserted separately.)
            let mut sc = score.clone();
            // Perturb one cell per good so the signatures differ, exactly as two
            // genuinely different plants would.
            let probe = 3 * w as usize + islands[k as usize];
            sc[probe] = 0.89;
            let (_belt, _seeds, comp) = localize_good(
                &buf, &sc, false, false, 0.5, k.wrapping_mul(0x9E37), 42, &[], 1, true, Some(&lm), &claims);
            let c = comp.expect("an endemic good must always be given a home");
            homes.push(c);
            claims.push((c, score_signature(&sc)));
        }
        let distinct: HashSet<u32> = homes.iter().copied().collect();
        assert_eq!(distinct.len(), 4,
            "four endemic goods on a world of four equal islands landed on {} of them ({homes:?}) \
             — the chooser is ignoring what earlier endemics claimed", distinct.len());

        // TWO PRODUCTS OF ONE TREE: an identical score field must SHARE a home,
        // never be dispersed. This is the nutmeg/mace exception, and it is why
        // the mechanism keys on the score signature rather than simply banning
        // every repeat.
        let mut claims2: Vec<(u32, u64)> = Vec::new();
        let (_b1, _s1, c1) = localize_good(
            &buf, &score, false, false, 0.5, 7, 42, &[], 1, true, Some(&lm), &claims2);
        claims2.push((c1.unwrap(), score_signature(&score)));
        let (_b2, _s2, c2) = localize_good(
            &buf, &score, false, false, 0.5, 99, 42, &[], 1, true, Some(&lm), &claims2);
        assert_eq!(c1, c2,
            "two goods with an IDENTICAL suitability field are one plant sold as two \
             products (nutmeg and mace) and must share an island — got {c1:?} and {c2:?}");
    }

    /// A minimal all-land world buffer for scoring tests. Mirrors the literal in
    /// `salt_pans_make_salt_and_brine` rather than sharing it, so a change to that
    /// test's fixture cannot silently alter this one's premise.
    fn scoring_buf(w: u32, h: u32) -> crate::sim::world_buffer::WorldBuffer {
        use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
        let n = (w * h) as usize;
        WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: vec![1u8; n], elevation: vec![0.2f32; n], sea_depth: vec![0.0; n],
            is_shelf: vec![0; n], is_shelf_edge: vec![0; n], locked_bits: Vec::new(),
            plate_index: Vec::new(), boundary_type: Vec::new(), is_volcanic: Vec::new(),
            temperature: vec![18.0; n], precipitation: vec![1200.0; n],
            koppen: vec![0u8; n],
            soil_type: vec![9u8; n], fertility: vec![0.7f32; n], fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(),
            distance_to_ocean: vec![0.5f32; n],
            habitability: Vec::new(), salinity: vec![0u8; n], shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(),
            seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        }
    }


    /// The ore-province fix: each deposit good's candidate field is its OWN
    /// salt-seeded low-frequency noise, so two minerals rank DIFFERENT highland
    /// cells highest â€” their deposits no longer all coincide on the tallest range
    /// (the old pure-elevation candidate bug).
    #[test]
    fn ore_provinces_differ_by_salt() {
        let (w, h) = (64u32, 48u32);
        let scale = 0.06f32;
        let field = |salt: u64| -> Vec<f32> {
            (0..(w * h))
                .map(|i| {
                    let (x, y) = ((i % w) as f32, (i / w) as f32);
                    fbm_noise(x * scale, y * scale, salt, 4, 2.0, 0.5)
                })
                .collect()
        };
        let a = field(0xC0FFEE_1234_5678); // copper-like salt
        let b = field(0x901D_901D_901D_901D); // gold-like salt
        // Richest 10% of cells in each mineral's province field.
        let topk = |f: &[f32]| -> HashSet<usize> {
            let mut idx: Vec<usize> = (0..f.len()).collect();
            idx.sort_by(|&i, &j| f[j].partial_cmp(&f[i]).unwrap());
            idx.into_iter().take(f.len() / 10).collect()
        };
        let (ta, tb) = (topk(&a), topk(&b));
        let overlap = ta.intersection(&tb).count();
        // Largely disjoint: the two minerals' richest provinces mostly don't coincide.
        assert!(overlap < ta.len() / 2, "provinces overlap too much: {overlap}/{}", ta.len());
    }

    /// A terminal salt lake must (a) produce SALT on its pan + shore and (b) write
    /// its brine into the salinity column; a freshwater lake does neither.
    #[test]
    fn salt_pans_make_salt_and_brine() {
        use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
        use crate::sim::rivers::Lake;
        use crate::sim::goods_spec::{default_list, builtin_index_of};

        let (w, h) = (20u32, 12u32);
        let n = (w * h) as usize;
        let mut buf = WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: vec![1u8; n], elevation: vec![0.1f32; n], sea_depth: vec![0.0; n],
            is_shelf: vec![0; n], is_shelf_edge: vec![0; n], locked_bits: Vec::new(),
            plate_index: Vec::new(), boundary_type: Vec::new(), is_volcanic: Vec::new(),
            temperature: Vec::new(), precipitation: vec![120.0; n],
            koppen: vec![crate::sim::koppen::BWK; n],
            soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: vec![0u8; n], shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(), seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        };
        let specs = default_list();
        buf.goods = vec![vec![0u8; n]; specs.len()];
        let salt_slot = specs.iter().position(|s| builtin_index_of(&s.id) == Some(super::GOOD_SALT)).unwrap();

        let salt_lake = Lake { cells: vec![(10, 6), (11, 6)], elevation: 0.1, kind: 0, endorheic: true, salinity_ppt: 120.0 };
        super::apply_salt_pans(&mut buf, &[salt_lake], &specs);
        let ci = buf.idx(10, 6);
        assert_eq!(buf.goods[salt_slot][ci], 230, "salt produced on the pan");
        assert_eq!(buf.salinity[ci], 255, "hypersaline brine written to the column");
        // A shore cell got the weaker evaporite-flat production.
        let shore = buf.idx(9, 6);
        assert!(buf.goods[salt_slot][shore] >= 180, "shore evaporite flats make salt");

        // A freshwater lake produces neither (its cell was untouched above).
        let fi = buf.idx(2, 2);
        let fresh = Lake { cells: vec![(2, 2)], elevation: 0.1, kind: 0, endorheic: false, salinity_ppt: 0.2 };
        super::apply_salt_pans(&mut buf, &[fresh], &specs);
        assert_eq!(buf.goods[salt_slot][fi], 0, "no salt at a freshwater lake");
        assert_eq!(buf.salinity[fi], 0, "no brine at a freshwater lake");
    }

    /// CLAUDE.md §8.19 (goods localities, shipped) Slice 2 (F5) — a Bank good may never place a cell
    /// adjacent to land, and an Inshore good may never place a cell that isn't
    /// adjacent to land. `Either` is untouched (reproduces the old undifferentiated
    /// `sea_coastal` gate) on both a shore cell and a bank cell.
    /// THE regression gate for the `clim_base` fold.
    ///
    /// `good_score` used to fold the dry-winter Köppen variants onto their humid
    /// equivalents BEFORE its match ran, which made every arm naming `CWA`/`CWB`/
    /// `CWC`/`DW*`/`DS*` unreachable. Tea and coffee both name `CWB` — subtropical
    /// highland, dry winter: Darjeeling, Yunnan, the Ethiopian and Kenyan
    /// highlands, i.e. exactly where tea and coffee come from — and both scored
    /// 0.0 there, because `CWB` folds to `CFB` and neither good lists `CFB`. They
    /// were placed by weak fallback arms in the wrong climates instead.
    ///
    /// The test asserts the CLAIM, not the implementation: a good that names a
    /// dry-winter zone must score in that zone. Any future refactor of the fold
    /// that reintroduces the bug fails here.
    #[test]
    fn dry_winter_zones_are_reachable() {
        let w = 8u32;
        let h = 4u32;
        let mut buf = scoring_buf(w, h);
        for i in 0..buf.total() {
            buf.koppen[i] = crate::sim::koppen::CWB;
            buf.elevation[i] = 0.30;
        }
        for (label, g) in [("tea", GOOD_TEA), ("coffee", GOOD_COFFEE)] {
            let s = good_score(&buf, g, 3, 2, None, MarineBand::Either);
            assert!(
                s > 0.0,
                "{label} scored {s} in Cwb — its own home climate. The clim_base \
                 fold has been reapplied before the match, which makes every arm \
                 naming a Cw/Dw/Ds zone unreachable."
            );
        }
    }

    /// The fold must still WORK as a fallback: a good that lists only the humid
    /// zone keeps scoring in the dry-winter twin, which is the behaviour
    /// `clim_base` was introduced for and which the fix must not cost.
    #[test]
    fn the_humid_fold_still_applies_as_a_fallback() {
        let w = 8u32;
        let h = 4u32;
        let mut buf = scoring_buf(w, h);
        for i in 0..buf.total() {
            // Dwb folds to Dfb, which timber lists at full weight.
            buf.koppen[i] = crate::sim::koppen::DWB;
            buf.temperature[i] = 8.0;
            buf.precipitation[i] = 700.0;
        }
        let s = good_score(&buf, GOOD_TIMBER, 3, 2, None, MarineBand::Either);
        assert!(s > 0.0, "timber lost its humid-equivalent fallback in Dwb (scored {s})");
    }

    /// A landmass component must be a real connected region, and an island must be
    /// distinguishable from a continent — the capability `Domain::Island` and
    /// `Distribution::Endemic` both rest on, and which did not exist before.
    #[test]
    fn landmass_labelling_separates_an_island_from_a_continent() {
        // A realistic grid width matters here: `ISLAND_MAX_KM2` converts to cells
        // per world, so on a very coarse grid one cell is already several hundred
        // km across and almost nothing qualifies as an island. 720 wide puts a
        // cell at ~56 km, close to the app's "Large" preset.
        let w = 720u32;
        let h = 360u32;
        let mut buf = scoring_buf(w, h);
        for i in 0..buf.total() { buf.terrain[i] = 0; }
        // A continent comfortably OVER `ISLAND_MAX_CELLS` (60x50 = 3000 cells)...
        for y in 40..100u32 {
            for x in 20..120u32 { let i = buf.idx(x, y); buf.terrain[i] = 1; }
        }
        // ...and a small island, separated by open water.
        for y in 200..203u32 {
            for x in 400..404u32 { let i = buf.idx(x, y); buf.terrain[i] = 1; }
        }
        let lm = LandmassContext::build(&buf);
        let cont = buf.idx(60, 60);
        let isle = buf.idx(401, 201);
        assert_ne!(lm.id[cont], lm.id[isle], "the island was merged into the continent");
        assert!(!lm.is_island(cont), "a 6000-cell continent must not read as an island");
        assert!(lm.is_island(isle), "a 4x3 island must read as an island");
        assert_eq!(lm.area_at(isle), 12, "island area");
    }

    #[test]
    fn marine_band_splits_inshore_from_bank() {
        use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
        use crate::sim::goods_spec::MarineBand;

        // A tiny world: a solid land column at x=5..7, sea either side. is_shelf=1
        // everywhere at sea so the shelf test alone can't already do the sorting.
        let (w, h) = (20u32, 10u32);
        let n = (w * h) as usize;
        let mut buf = WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: vec![0u8; n], elevation: vec![0.0f32; n], sea_depth: vec![0.3; n],
            is_shelf: vec![1u8; n], is_shelf_edge: vec![0; n], locked_bits: Vec::new(),
            plate_index: Vec::new(), boundary_type: Vec::new(), is_volcanic: Vec::new(),
            temperature: Vec::new(), precipitation: Vec::new(),
            koppen: vec![0u8; n],
            soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(),
            seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        };
        for y in 0..h {
            for x in 5..8u32 {
                buf.terrain[(y * w + x) as usize] = 1;
            }
        }
        // (x=8, y=5) is the sea cell touching the land column at x=7.
        let shore = (8u32, 5u32);
        // (x=15, y=5) is far offshore (wrapped distance to the land column ~8 cells).
        let bank = (15u32, 5u32);

        assert!(super::has_land_within(&buf, shore.0, shore.1, 1), "x=8 must be adjacent to the land column");
        assert!(!super::has_land_within(&buf, bank.0, bank.1, 1), "x=15 must NOT be adjacent to the land column");

        assert!(super::marine_band_ok(&buf, shore.0, shore.1, MarineBand::Either));
        assert!(super::marine_band_ok(&buf, shore.0, shore.1, MarineBand::Inshore));
        assert!(!super::marine_band_ok(&buf, shore.0, shore.1, MarineBand::Bank),
            "a Bank good must never place a cell adjacent to land");

        assert!(super::marine_band_ok(&buf, bank.0, bank.1, MarineBand::Either));
        assert!(super::marine_band_ok(&buf, bank.0, bank.1, MarineBand::Bank));
        assert!(!super::marine_band_ok(&buf, bank.0, bank.1, MarineBand::Inshore),
            "an Inshore good must never place a cell off the shore fringe");
    }
}






