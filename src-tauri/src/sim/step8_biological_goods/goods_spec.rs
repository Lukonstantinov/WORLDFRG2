//! Declarative, editable trade-good definitions.
//!
//! A `GoodSpec` is the data-driven description of one trade good: its display
//! metadata (name/icon/color), where it may sit (`domain`), how it is distributed
//! (`distribution`), and either a reference to a built-in hardcoded scorer
//! (`builtin = true`, `scoring = None`) or a fully declarative `Envelope` for
//! custom goods.
//!
//! The 30 built-ins are produced by `default_list()`, seeded from the const
//! tables in `biological.rs`, so spec-driven generation reproduces the original
//! behavior exactly. Worlds snapshot their active list into DB metadata
//! (`goods_spec`); a global JSON library is the editing template for new worlds.

use serde::{Deserialize, Serialize};

use super::biological::{
    deposit_params, GOOD_BASE_VALUE, GOOD_BULK, GOOD_CATEGORY, GOOD_COLOR, GOOD_DESIRE, GOOD_ICON,
    GOOD_LABEL, GOOD_MARINE, GOOD_NAMES, GOOD_NEED_TIER, GOOD_NETWORK_LUXURY, GOOD_PERISH,
    GOOD_RARITY, GOOD_UNLIMITED, GOOD_DYES, GOOD_FRANKINCENSE, GOOD_GEMSTONES, GOOD_INDIGO,
    GOOD_TOBACCO,
};
use crate::tile::cell::GOODS_COUNT;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    Marine,
    Coastal,
    Continental,
    Island,
}

/// CLAUDE.md §8.19 (goods localities, shipped) Slice 2 — which part of the shelf a marine good may
/// occupy. `Either` (the default, and what every pre-existing save carries via
/// serde) reproduces the old undifferentiated `sea_coastal` gate exactly: F5 found
/// that gate served pearls and stockfish alike, so a fishing bank sat on the beach
/// and a pearl bed sat on the open shelf. `Inshore`/`Bank` narrow that gate for the
/// specific goods that need it (§2.1's default table); every other marine good is
/// unaffected.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum MarineBand {
    #[default]
    Either,
    /// A shelf cell adjacent to land — the beach/lagoon fringe.
    Inshore,
    /// A shelf cell away from land — the open bank.
    Bank,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Distribution {
    /// Every suitable cell produces (the old UNLIMITED model).
    Global,
    /// One suitability-weighted homeland, flood-filled (the old SEEDED model).
    Local,
    /// Discrete highland-locked blobs scattered worldwide (gems / metals).
    Deposits,
    /// Confined to ONE small landmass and unable to leave it — the Banda nutmeg
    /// case. Distinct from `Local` in two ways that matter: the homeland is chosen
    /// among cells on landmasses under `ISLAND_MAX_CELLS`, and the flood-fill's
    /// island-jump is DISABLED, so the belt physically cannot hop to a neighbouring
    /// island or the mainland.
    ///
    /// This is why nutmeg was worth its weight in gold for two centuries: not
    /// because the climate was rare (nutmeg grows happily across the wet tropics)
    /// but because the *tree* only grew on ten islands totalling 60 km². A rarity
    /// knob cannot express that — it makes a good scarce EVERYWHERE rather than
    /// abundant in exactly one place and absent from the rest of the world, which
    /// is the shape that actually produced the spice trade.
    Endemic,
    /// Made in cities from a recipe (`inputs`), not extracted from any cell — has
    /// NO per-cell belt. Placed by `apply_manufacturing`, not the worldgen placer.
    Manufactured,
}

/// One input line of a `Manufactured` good's recipe: `qty` units of `good`
/// (referenced by its spec `id`) are consumed per 1 unit of output.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecipeInput {
    pub good: String,
    pub qty: f32,
}

fn default_province_scale() -> f32 {
    0.06
}

/// One homeland — the historical behaviour of every good before `origins` existed.
fn default_origins() -> u8 {
    1
}

/// Parameters for a `Deposits`-distributed good.
///
/// `model` / `placer_frac` / `parent` are the GEOLOGICAL half (see
/// `sim::deposits`); `min_elev` and `province_scale` are the older purely
/// elevation-and-noise knobs, kept because saved worlds carry them and because a
/// user who wants the blunt instrument should still have it.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DepositSpec {
    pub min_elev: f32,
    pub count_num: u32,
    pub count_den: u32,
    /// Low-frequency noise frequency for this mineral's ore-province field, so each
    /// deposit good lights up its OWN mountain ranges instead of all clustering on
    /// the single tallest one. Larger = smaller, more scattered provinces.
    #[serde(default = "default_province_scale")]
    pub province_scale: f32,
    /// The ore-forming environment this mineral belongs to. `None` = fall back to
    /// `deposits::default_model_for(id)`, which is the geologically correct default
    /// for every shipped mineral — so nobody has to author geology, and anybody can
    /// override one mineral without touching the rest.
    #[serde(default)]
    pub model: Option<crate::sim::deposits::DepositModel>,
    /// Share of this mineral's districts placed as river gravel BELOW a lode
    /// district instead of in situ. `None` = the default for this mineral. High for
    /// exactly the minerals history won from gravel first: gold, stream tin,
    /// Golconda's diamonds, Ratnapura's sapphires.
    #[serde(default)]
    pub placer_frac: Option<f32>,
    /// Parent good id for a DERIVED mineral (`Weathering`) — turquoise forms where
    /// a copper orebody weathers in a desert. `None` = the shipped default.
    #[serde(default)]
    pub parent: Option<String>,
}

/// Declarative scoring envelope for custom (and overridden) goods. Every term is
/// optional; an absent term contributes a neutral 1.0. Mirrors the house scoring
/// style used by the built-in scorer.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Envelope {
    /// Sparse per-Köppen-zone suitability (zone code → weight 0..1). Empty = any.
    #[serde(default)]
    pub climate: Vec<(u8, f32)>,
    /// Temperature bell `{center, width}` in °C.
    #[serde(default)]
    pub temp: Option<[f32; 2]>,
    /// Precipitation band `{lo, hi, edge}` in mm/yr.
    #[serde(default)]
    pub precip: Option<[f32; 3]>,
    /// Normalized-elevation band `{lo, hi, edge}` (0..1).
    #[serde(default)]
    pub elevation: Option<[f32; 3]>,
    /// |latitude| band `{lo, hi, edge}` in degrees.
    #[serde(default)]
    pub abs_lat: Option<[f32; 3]>,
    /// Fertility weight: score *= (1 - w) + w*fertility.
    #[serde(default)]
    pub fertility: f32,
    /// Bonus for being near a coast (land) / on the shelf (marine).
    #[serde(default)]
    pub coast_bonus: f32,
    // ── CLAUDE.md §8.19 (goods localities, shipped) Slice 1 (F6) — river placement factors. Each is a
    // 0..1 weight; 0 (the serde default, so every pre-existing custom good is
    // untouched) means the factor contributes nothing. Every factor is a
    // MULTIPLIER on the score above, never a replacement (§5.4) — a floodplain
    // bonus can make a marginal cell better but never pulls a good outside its
    // climate gate, since the gate already zeroed cells the climate rejects.
    /// Delta / low flat ground near a MAJOR river (rice, cotton, indigo, sugar,
    /// wheat paddies and floodplain fields).
    #[serde(default)]
    pub floodplain: f32,
    /// River water reaching an arid (Köppen B) cell — the oasis/canal signature
    /// (cotton, dates, sugar, rice grown by irrigation rather than rainfall).
    #[serde(default)]
    pub irrigation: f32,
    /// Within a few cells of ANY river (papyrus/bamboo paper, honey, hides — a
    /// watering place, not necessarily a floodplain).
    #[serde(default)]
    pub riverbank: f32,
    /// Near a NAVIGABLE river — the good can reach market by water (timber,
    /// hardwoods, furs: the historical Baltic/Canadian river timber trades).
    #[serde(default)]
    pub float_out: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GoodSpec {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub enabled: bool,
    pub domain: Domain,
    pub distribution: Distribution,
    pub rarity: f32,
    pub desire: f32,
    pub network_luxury: bool,
    /// True for the 30 shipped goods (scored by the hardcoded built-in scorer
    /// when `scoring` is None). Custom goods are false and must carry `scoring`.
    pub builtin: bool,
    #[serde(default)]
    pub deposit: Option<DepositSpec>,
    #[serde(default)]
    pub scoring: Option<Envelope>,
    /// CLAUDE.md §8.19 (goods localities, shipped) Slice 2 (F5) — which part of the shelf this marine
    /// good may occupy. `Either` (serde default) reproduces the old undifferentiated
    /// gate exactly, so every save predating this field is unaffected.
    #[serde(default)]
    pub marine_band: MarineBand,
    /// How many INDEPENDENT homelands a `Local` good seeds. 1 (the serde default,
    /// so every pre-existing good and save is byte-identical) is the old
    /// one-seed-one-flood-fill behaviour.
    ///
    /// Multi-origin is the historical NORM, not an exception: pepper came from
    /// Malabar *and* Sumatra, cinnamon from Ceylon *and* China (cassia), cotton was
    /// domesticated independently in India, Peru and the Levant, frankincense grew
    /// in Hadhramaut *and* the Horn. A single homeland per good made every world's
    /// trade map a set of one-source monopolies, which is the rarest case in the
    /// record, not the usual one.
    ///
    /// Each origin repels the ones already placed through the SAME dispersion
    /// penalty `localize_good` already applies between different goods, so origins
    /// land on genuinely different parts of the world.
    #[serde(default = "default_origins")]
    pub origins: u8,
    // ── FINE-GRAIN terrain factors (the patchiness pass) ──────────────────────
    // These live on the SPEC, not on `Envelope`, deliberately: they must apply to
    // built-in goods (scored by the hardcoded matcher, which has no Envelope) and
    // to custom goods (scored by `envelope_score`) alike, and they are applied
    // once by the placer after whichever scorer ran. Putting them in `Envelope`
    // would have silently REPLACED a built-in's whole climate scorer with an
    // envelope holding only these two terms.
    //
    // Every other scoring term varies over hundreds of km — Köppen zone,
    // temperature, precipitation, |latitude|, normalized elevation, and fertility
    // (itself a smoothed blend). That is why a belt rendered as one smooth
    // continent-sized wash: nothing in the model varied at the 2-10 km scale at
    // which real crop distributions are actually mottled. `soil_type` was computed
    // by phase 6 and read by NOTHING in the goods layer; slope was never computed.
    /// Sparse per-soil-class suitability (soil code → weight 0..1). Empty (the
    /// serde default) = any soil, i.e. exactly the old behaviour.
    ///
    /// This is what separates goods that SHARE a climate: vines want stony,
    /// sharply drained ground and fail on heavy clay; rice wants that clay
    /// precisely because it holds water; olives want thin calcareous soil that
    /// would starve a cereal. All three are Mediterranean.
    #[serde(default)]
    pub soil: Vec<(u8, f32)>,
    /// Local relief band `{lo, hi, edge}` — the drop in normalized elevation
    /// across a 2-cell neighbourhood. A SLOPE gate, not an altitude gate, and the
    /// distinction is the point: terraced vines and hill tea want a slope at any
    /// altitude, paddy rice and cereal want flat ground at any altitude, and
    /// `Envelope::elevation` alone cannot say either.
    #[serde(default)]
    pub relief: Option<[f32; 3]>,
    // ── Market fields (serde defaults keep pre-market spec JSON loading) ──
    /// Need category; alternatives within a category substitute for each other
    /// in the market's needs ladder ("" = no substitution group).
    #[serde(default)]
    pub category: String,
    /// Needs ladder tier: 0 basic, 1 comfort, 2 luxury.
    #[serde(default)]
    pub need_tier: u8,
    /// World-standard value per unit in the grain-equivalent numeraire
    /// (wheat = 1.0). Local prices are quoted as multiples of grain.
    #[serde(default = "default_base_value")]
    pub base_value: f32,
    // ── Transport + production fields (serde defaults keep older spec JSON loading) ──
    /// Freight weight/volume multiplier on per-day haulage cost. 1.0 = a compact
    /// luxury (silk); 3-4 = a bulky low-value staple (timber, grain, ore) that
    /// stays regional because hauling it eats its value.
    #[serde(default = "default_bulk")]
    pub bulk: f32,
    /// Extra freight cost per travel-day from spoilage (additive). 0 = durable
    /// (metals, salt-cod); high for fresh fish / fruit so they can't travel far.
    #[serde(default)]
    pub perishable: f32,
    /// Recipe inputs for a `Manufactured` good (empty = raw/extracted). Each line
    /// consumes `qty` units of the referenced good per 1 unit produced.
    #[serde(default)]
    pub inputs: Vec<RecipeInput>,
    /// Output-rate factor for manufacture: a hub's labor capacity (∝ population) is
    /// multiplied by this (a city makes more cloth than a village).
    #[serde(default = "default_labor")]
    pub labor: f32,
    /// Demand CADENCE in days — how often a person consumes a unit. Food ≈ 1
    /// (constant), cloth ≈ 30, durables like furs/furniture ≈ 90, luxuries ≈ 365.
    /// A long interval means weak daily local pull → the good sits cheaper and is
    /// mostly bought by merchants for resale rather than citizens for use.
    #[serde(default = "default_consumption_interval")]
    pub consumption_interval: f32,
}

fn default_base_value() -> f32 {
    1.0
}

fn default_bulk() -> f32 {
    1.0
}

fn default_consumption_interval() -> f32 {
    30.0 // a month — a sensible neutral cadence
}

fn default_labor() -> f32 {
    1.0
}

/// Fill the market fields on specs saved before they existed (old world
/// snapshots / library files): builtins backfill from the const tables, customs
/// get a neutral category with their luxury flag mapped to a tier.
pub fn backfill_market_fields(specs: &mut [GoodSpec]) {
    for spec in specs.iter_mut() {
        if !spec.category.is_empty() {
            continue;
        }
        if let Some(g) = builtin_index_of(&spec.id) {
            spec.category = GOOD_CATEGORY[g].to_string();
            spec.need_tier = GOOD_NEED_TIER[g];
            spec.base_value = GOOD_BASE_VALUE[g];
            // Transport facts share the same "filled before?" guard: a pre-market
            // save predates bulk/perish too, so seed them from the const tables.
            spec.bulk = GOOD_BULK[g];
            spec.perishable = GOOD_PERISH[g];
        } else {
            spec.category = custom_category(&spec.id).to_string();
            spec.need_tier = if spec.network_luxury { 2 } else { 1 };
            if spec.base_value <= 0.0 {
                spec.base_value = 1.0;
            }
        }
    }
}

/// Categories for the shipped declarative (non-builtin) goods; unknown custom
/// ids fall back to "misc" (no substitution group).
fn custom_category(id: &str) -> &'static str {
    match id {
        "bay_salt" => "preservative",
        "citrus" => "sweetener",
        "flax" => "fiber",
        "coral" | "ambergris" | "jade" => "prestige",
        // Split gem types share the fungible "gem" category (see manufacture.rs).
        "ruby" | "sapphire" | "emerald" | "diamond" | "amethyst" | "topaz" => "gem",
        "cinnamon" | "saffron" => "aromatic",
        "tyrian_purple" => "dye",
        "silver" | "lead" => "metal",
        "marble" => "construction",
        // Manufactured chain goods satisfy the same NEED as their finished form, so
        // they substitute against the matching raws in the market's needs ladder.
        "cloth" => "fiber",
        "metalware" => "metal",
        "refined_sugar" => "sweetener",
        "citrus_liqueur" => "drink",
        // ── Ported richer manufactured catalog (+ Clay raw) ──
        "linen" | "cotton_cloth" | "silk_brocade" | "carpets" => "fiber",
        "leather_goods" => "leather",
        "bronzeware" => "metal",
        "jewelry" | "ivory_carvings" => "prestige",
        "brandy" | "mead" => "drink",
        "perfume" => "aromatic",
        "soap" | "candles" | "books" | "furniture" => "craft",
        "statuary" | "clay" => "construction",
        _ => "misc",
    }
}

/// Index of a built-in good by id (its column / hardcoded-scorer index), or None
/// for custom goods.
pub fn builtin_index_of(id: &str) -> Option<usize> {
    GOOD_NAMES.iter().position(|&n| n == id)
}

/// The shipped 30-good library, seeded from the `biological.rs` const tables so
/// that spec-driven generation is byte-identical to the pre-spec behavior.
pub fn default_list() -> Vec<GoodSpec> {
    let mut list: Vec<GoodSpec> = (0..GOODS_COUNT)
        .map(|g| {
            let dp = deposit_params(g);
            let distribution = if dp.is_some() {
                Distribution::Deposits
            } else if GOOD_UNLIMITED[g] {
                Distribution::Global
            } else {
                Distribution::Local
            };
            let domain = if GOOD_MARINE[g] {
                Domain::Marine
            } else {
                Domain::Continental
            };
            GoodSpec {
                id: GOOD_NAMES[g].to_string(),
                name: GOOD_LABEL[g].to_string(),
                icon: GOOD_ICON[g].to_string(),
                color: GOOD_COLOR[g].to_string(),
                // ~1400 curation: tobacco is post-period flavor and frankincense/
                // indigo fold into the incense/dyes categories — shipped disabled
                // (still selectable in the editor, and old worlds keep them).
                //
                // RETIRED as duplicates (goods cull). Disabled, never deleted: these
                // are fixed indices in `TileData.goods`, so removing a slot would
                // break every saved `.worldforge` (rule 7). An old world keeps
                // whatever its snapshot says; only NEW worlds ship without them.
                //   • `gemstones` — a generic "gem" alongside ELEVEN specific stones
                //     (jade, ruby, sapphire, emerald, diamond, amethyst, topaz,
                //     garnet, carnelian, lapis_lazuli, turquoise). §8.16 already
                //     repurposed `gem_deposits` to mean ore DISTRICTS, so the
                //     generic's original job no longer exists.
                //   • `dyes` — marine murex purple, i.e. the same product as the
                //     custom `tyrian_purple`. It is also the one good
                //     `goods_validation` carries as a standing coverage-floor
                //     EXCEPTION, so retiring it removes that exception too.
                enabled: !matches!(
                    g,
                    GOOD_TOBACCO | GOOD_FRANKINCENSE | GOOD_INDIGO | GOOD_GEMSTONES | GOOD_DYES
                ),
                domain,
                distribution,
                rarity: GOOD_RARITY[g],
                desire: GOOD_DESIRE[g],
                network_luxury: GOOD_NETWORK_LUXURY[g],
                builtin: true,
                deposit: dp.map(|d| DepositSpec {
                    min_elev: d.min_elev,
                    count_num: d.count_num,
                    count_den: d.count_den,
                    province_scale: default_province_scale(),
                    // Left None so the geological default table
                    // (`deposits::default_model_for`) drives placement. Setting it
                    // here would freeze today's table into every saved world.
                    model: None,
                    placer_frac: None,
                    parent: None,
                }),
                scoring: None,
                marine_band: default_marine_band_for(GOOD_NAMES[g]),
                origins: default_origins_for(GOOD_NAMES[g]),
                soil: Vec::new(),
                relief: None,
                category: GOOD_CATEGORY[g].to_string(),
                need_tier: GOOD_NEED_TIER[g],
                base_value: GOOD_BASE_VALUE[g],
                bulk: GOOD_BULK[g],
                perishable: GOOD_PERISH[g],
                inputs: Vec::new(),
                labor: 1.0,
                // Cadence by need: staples eaten ~weekly, comfort monthly-ish,
                // luxuries/durables seasonally → weak daily pull (wholesale skew).
                consumption_interval: match GOOD_NEED_TIER[g] { 0 => 7.0, 1 => 45.0, _ => 180.0 },
            }
        })
        .collect();
    list.extend(default_custom_goods());
    // Customs are built with empty market fields; fill them from the id map.
    backfill_market_fields(&mut list);
    // Convert the two unambiguous CITY crafts from raw belts to true manufactures
    // (made from imported clay/soda + kiln fuel, not dug from a cell). Paper & beer
    // stay raw belts (paper's per-cell subtypes + the brewing homeland need a belt).
    for spec in list.iter_mut() {
        match spec.id.as_str() {
            "ceramics" => {
                spec.distribution = Distribution::Manufactured;
                spec.inputs = vec![
                    RecipeInput { good: "clay".into(), qty: 1.0 },
                    RecipeInput { good: "timber".into(), qty: 0.2 },
                ];
            }
            "glassware" => {
                spec.distribution = Distribution::Manufactured;
                spec.inputs = vec![
                    RecipeInput { good: "bay_salt".into(), qty: 0.1 },
                    RecipeInput { good: "timber".into(), qty: 0.3 },
                ];
            }
            _ => {}
        }
    }
    apply_terroir_defaults(&mut list);
    list
}

/// Attach the FINE-GRAIN terrain terms (`Envelope::soil` / `Envelope::relief`) to
/// the goods whose real distribution is decided by them.
///
/// Why this exists: every other scoring term varies over hundreds of km — Köppen
/// zone, temperature, precipitation, |latitude|, normalized elevation, and
/// fertility (itself a smoothed blend). That is precisely why a belt rendered as
/// one smooth continent-sized wash: the model had NO channel varying at the 2-10
/// km scale at which real crop distributions are actually mottled. `soil_type`
/// was computed by phase 6 and read by nothing in the goods layer; slope was
/// never computed at all.
///
/// These entries are the classic soil preferences, and note that they *separate
/// goods that share a climate*, which is the whole point — vines and cereal want
/// opposite ground in the same Mediterranean zone:
///   • vines/olives  thin, stony, sharply-drained ground on a SLOPE; heavy
///                   alluvial clay makes vigorous vines and poor wine.
///   • rice          the clay and the flat that vines reject — it must hold water.
///   • cereals       deep prairie/forest loams on gentle ground.
///   • tea/coffee    volcanic and leached uplands, on a slope for drainage.
///   • dates/millet  desert and young soils nothing else will take.
///
/// Applied here rather than in each good's own constructor so the whole table is
/// readable in one place, exactly as the ceramics/glassware conversion above is.
fn apply_terroir_defaults(list: &mut [GoodSpec]) {
    use crate::sim::soil::*;
    // (id, soil weights, optional relief band {lo, hi, edge})
    let table: &[(&str, &[(u8, f32)], Option<[f32; 3]>)] = &[
        ("wine",      &[(SOIL_ENTISOL,1.0),(SOIL_ALFISOL,0.85),(SOIL_MOLLISOL,0.6),(SOIL_ANDISOL,0.9),(SOIL_ULTISOL,0.5),(SOIL_ARIDISOL,0.45),(SOIL_ALLUVIAL,0.3)], Some([0.010,0.090,0.030])),
        ("oliveoil",  &[(SOIL_ENTISOL,1.0),(SOIL_ARIDISOL,0.7),(SOIL_ALFISOL,0.8),(SOIL_MOLLISOL,0.55),(SOIL_ALLUVIAL,0.35)], Some([0.008,0.080,0.030])),
        ("rice",      &[(SOIL_ALLUVIAL,1.0),(SOIL_ULTISOL,0.75),(SOIL_OXISOL,0.6),(SOIL_HISTOSOL,0.55),(SOIL_ANDISOL,0.7),(SOIL_ENTISOL,0.3)], Some([0.0,0.022,0.012])),
        ("wheat",     &[(SOIL_MOLLISOL,1.0),(SOIL_ALFISOL,0.85),(SOIL_ALLUVIAL,0.8),(SOIL_ULTISOL,0.5),(SOIL_ENTISOL,0.4),(SOIL_ARIDISOL,0.3)], Some([0.0,0.045,0.020])),
        ("barley",    &[(SOIL_MOLLISOL,0.9),(SOIL_ALFISOL,0.9),(SOIL_SPODOSOL,0.7),(SOIL_ENTISOL,0.6),(SOIL_ALLUVIAL,0.7)], Some([0.0,0.060,0.025])),
        ("millet",    &[(SOIL_ARIDISOL,1.0),(SOIL_ENTISOL,0.85),(SOIL_MOLLISOL,0.7),(SOIL_ALFISOL,0.4)], None),
        ("tea",       &[(SOIL_ULTISOL,1.0),(SOIL_ANDISOL,0.95),(SOIL_OXISOL,0.7),(SOIL_ALFISOL,0.5)], Some([0.020,0.120,0.040])),
        ("coffee",    &[(SOIL_ANDISOL,1.0),(SOIL_VOLCANIC_ASH,0.95),(SOIL_ULTISOL,0.8),(SOIL_OXISOL,0.6)], Some([0.020,0.120,0.040])),
        ("cotton",    &[(SOIL_ALLUVIAL,1.0),(SOIL_MOLLISOL,0.7),(SOIL_ULTISOL,0.6),(SOIL_ARIDISOL,0.5),(SOIL_ENTISOL,0.4)], Some([0.0,0.030,0.015])),
        ("dates",     &[(SOIL_ARIDISOL,1.0),(SOIL_ENTISOL,0.8),(SOIL_ALLUVIAL,0.9)], None),
        ("timber",    &[(SOIL_SPODOSOL,1.0),(SOIL_ALFISOL,0.9),(SOIL_HISTOSOL,0.5),(SOIL_MOLLISOL,0.4),(SOIL_ANDISOL,0.7)], None),
        ("horses",    &[(SOIL_MOLLISOL,1.0),(SOIL_ARIDISOL,0.6),(SOIL_ALFISOL,0.6),(SOIL_ENTISOL,0.5)], Some([0.0,0.040,0.020])),
        ("flax",      &[(SOIL_ALLUVIAL,1.0),(SOIL_ALFISOL,0.8),(SOIL_MOLLISOL,0.7),(SOIL_HISTOSOL,0.5)], Some([0.0,0.030,0.015])),
        // saffron is deliberately NOT here. Its belt is already tightly bounded by
        // climate, elevation AND latitude, and adding a soil preference on top
        // over-constrained it: it moved off every settlement's catchment and
        // tripped the Slice 0 coverage floor. A good this narrow has no room for
        // another gate.
    ];
    for spec in list.iter_mut() {
        let Some((_, soil, relief)) = table.iter().find(|(id, _, _)| *id == spec.id) else { continue };
        spec.soil = soil.to_vec();
        spec.relief = *relief;
    }
}

/// Curated extra goods shipped on top of the 30+ built-ins: grain & paper & salt
/// *types* (per the user's "types of wheat/paper", "bay vs rock salt") plus a set
/// of regionally-distinct commodities and a few extreme-rare luxuries. All are
/// declarative (`Envelope`-scored) custom goods, so they need no column/const
/// changes and can be freely edited in the Goods Editor.
fn default_custom_goods() -> Vec<GoodSpec> {
    // Köppen codes used below: AF1 AM2 AW3 BWH4 BWK5 BSH6 BSK7 CSA8 CSB9 CFA11
    // CFB12 DFA14 DFB15 DFC16 DSB19 AS23 CWA24.
    #[allow(clippy::too_many_arguments)]
    fn cg(
        id: &str, name: &str, icon: &str, color: &str, domain: Domain, dist: Distribution,
        rarity: f32, desire: f32, luxury: bool, deposit: Option<DepositSpec>, env: Envelope,
    ) -> GoodSpec {
        GoodSpec {
            id: id.into(), name: name.into(), icon: icon.into(), color: color.into(),
            enabled: true, domain, distribution: dist, rarity, desire, network_luxury: luxury,
            builtin: false, deposit, scoring: Some(env),
            marine_band: default_marine_band_for(id),
            origins: default_origins_for(id),
            soil: Vec::new(),
            relief: None,
            category: String::new(), need_tier: 0, base_value: 1.0,
            bulk: 1.0, perishable: 0.0, inputs: Vec::new(), labor: 1.0,
            consumption_interval: 45.0,
        }
    }
    // A Manufactured chain good: made in cities from `inputs`, no per-cell belt.
    #[allow(clippy::too_many_arguments)]
    fn mg(
        id: &str, name: &str, icon: &str, color: &str, base_value: f32, bulk: f32, perish: f32,
        luxury: bool, labor: f32, inputs: Vec<(&str, f32)>,
    ) -> GoodSpec {
        GoodSpec {
            id: id.into(), name: name.into(), icon: icon.into(), color: color.into(),
            enabled: true, domain: Domain::Continental, distribution: Distribution::Manufactured,
            rarity: 0.5, desire: 0.55, network_luxury: luxury, builtin: false, deposit: None,
            marine_band: MarineBand::Either,
            origins: 1,
            soil: Vec::new(),
            relief: None,
            scoring: None, category: String::new(), need_tier: 0, base_value,
            bulk, perishable: perish, labor,
            inputs: inputs.into_iter().map(|(g, q)| RecipeInput { good: g.into(), qty: q }).collect(),
            consumption_interval: if luxury { 180.0 } else { 60.0 },
        }
    }
    let dep = |min_elev: f32, num: u32, den: u32| Some(DepositSpec {
        min_elev, count_num: num, count_den: den, province_scale: default_province_scale(),
        model: None, placer_frac: None, parent: None,
    });
    let env = |climate: Vec<(u8, f32)>, temp: Option<[f32; 2]>, precip: Option<[f32; 3]>,
               elevation: Option<[f32; 3]>, abs_lat: Option<[f32; 3]>, fertility: f32, coast_bonus: f32| Envelope {
        climate, temp, precip, elevation, abs_lat, fertility, coast_bonus,
        ..Default::default()
    };
    // A DEPOSIT good with its OWN base_value/category/bulk (DEPOSITS_AND_MINING_PLAN
    // slice 3). `cg()` hardcodes base_value to 1.0 for every good that goes through
    // it — fine for the existing customs, which never needed to sit at a different
    // price band, but wrong for these eight, which are meant to carry real,
    // differentiated values (lapis at 40, coal near nothing). `model`/`parent` are
    // left to `deposits::default_model_for`/`default_parent_for`, which already
    // carry the correct entry for every id below — that table was built with this
    // slice in mind.
    #[allow(clippy::too_many_arguments)]
    fn dg(
        id: &str, name: &str, icon: &str, color: &str, domain: Domain,
        rarity: f32, desire: f32, luxury: bool, num: u32, den: u32,
        category: &str, need_tier: u8, base_value: f32, bulk: f32, env: Envelope,
    ) -> GoodSpec {
        GoodSpec {
            id: id.into(), name: name.into(), icon: icon.into(), color: color.into(),
            enabled: true, domain, distribution: Distribution::Deposits, rarity, desire,
            network_luxury: luxury, builtin: false,
            marine_band: MarineBand::Either,
            origins: 1,
            soil: Vec::new(),
            relief: None,
            deposit: Some(DepositSpec {
                min_elev: 0.0, count_num: num, count_den: den,
                province_scale: default_province_scale(),
                model: None, placer_frac: None, parent: None,
            }),
            scoring: Some(env),
            category: category.into(), need_tier, base_value, bulk, perishable: 0.0,
            inputs: Vec::new(), labor: 1.0,
            consumption_interval: if luxury { 180.0 } else { 45.0 },
        }
    }

    vec![
        // Note: grain & paper "types" already exist as per-cell SUBTYPES of the
        // built-in "Grain"/"Paper" goods (GRAIN_SUBTYPES / PAPER_SUBTYPES in the
        // frontend goods.ts), so they are NOT duplicated as separate goods here.
        // ── Salt types — built-in "salt" is now Rock Salt; this is coastal bay salt ──
        cg("bay_salt", "Bay Salt", "\u{1F9C2}", "#e8e0d0", Domain::Coastal, Distribution::Deposits, 0.45, 0.55, false,
            dep(0.0, 2, 1), env(vec![(4,1.0),(5,0.9),(6,0.9),(7,0.8),(8,0.6),(9,0.6)], Some([24.0,14.0]), Some([0.0,500.0,300.0]), None, Some([8.0,45.0,12.0]), 0.0, 1.0)),
        // ── Regionally-distinct commodities ──
        cg("citrus", "Citrus", "\u{1F34A}", "#f4a33a", Domain::Coastal, Distribution::Local, 0.55, 0.45, false,
            None, env(vec![(8,1.0),(9,1.0),(11,0.7)], Some([19.0,7.0]), Some([400.0,1100.0,300.0]), None, Some([25.0,40.0,8.0]), 0.4, 0.5)),
        cg("flax", "Flax / Linen", "\u{1F331}", "#cfe0e8", Domain::Continental, Distribution::Local, 0.50, 0.40, false,
            None, env(vec![(12,1.0),(11,0.7),(15,0.8),(14,0.7)], Some([14.0,7.0]), Some([500.0,1200.0,300.0]), None, None, 0.5, 0.0)),
        cg("coral", "Red Coral", "\u{1F41A}", "#ff6f61", Domain::Marine, Distribution::Local, 0.60, 0.40, true,
            None, env(vec![], Some([26.0,5.0]), None, None, Some([0.0,30.0,8.0]), 0.0, 0.6)),
        // ── ISLAND ENDEMICS (`Distribution::Endemic`) ─────────────────────────
        // NOTE the domain: these are `Coastal`, not `Island`, and the split is
        // deliberate. DOMAIN says where the plant can grow at all (a wet tropical
        // coast); DISTRIBUTION says how it is confined (one landmass, chosen by
        // `localize_good` as the smallest that scores, preferring a true island).
        // Using `Domain::Island` here instead zeroed the SCORE on every
        // continental cell before the distribution could pick a home, so all six
        // measured zero cells on the coverage diagnostic — the domain gate and the
        // distribution gate were fighting each other.
        // A good that grows in exactly ONE place on the whole world and cannot
        // spread off it. Every entry below is a real endemic whose value came
        // from its geography rather than from its climate being rare — nutmeg
        // grows happily across the wet tropics, but the TREE only grew on ten
        // islands totalling 60 km², and that is why it was worth its weight in
        // gold. `rarity` cannot express this: it makes a good scarce everywhere,
        // rather than abundant in one place and absent from the rest of the
        // world, which is the shape that actually produced the spice trade.
        //
        // NUTMEG and MACE deliberately share an envelope and a domain: they are
        // two products of ONE tree (the seed and its aril), so on any world they
        // land on the same archipelago and trade as a pair at different values —
        // the Banda relationship exactly.
        cg("nutmeg", "Nutmeg", "\u{1F330}", "#8b5a2b", Domain::Coastal, Distribution::Endemic, 0.93, 0.60, true,
            None, env(vec![(1,1.0),(2,0.9)], Some([26.0,4.0]), Some([1800.0,3500.0,500.0]), Some([0.0,0.30,0.12]), Some([0.0,12.0,5.0]), 0.0, 0.8)),
        cg("mace", "Mace", "\u{1F53A}", "#d2691e", Domain::Coastal, Distribution::Endemic, 0.94, 0.55, true,
            None, env(vec![(1,1.0),(2,0.9)], Some([26.0,4.0]), Some([1800.0,3500.0,500.0]), Some([0.0,0.30,0.12]), Some([0.0,12.0,5.0]), 0.0, 0.8)),
        // Socotra's dragon's blood — one island, an ARID one, which is why this
        // envelope looks nothing like the other endemics'.
        cg("dragons_blood", "Dragon's Blood Resin", "\u{2764}", "#a41d1d", Domain::Coastal, Distribution::Endemic, 0.95, 0.50, true,
            None, env(vec![(6,1.0),(4,0.8),(7,0.5)], Some([25.0,7.0]), Some([100.0,500.0,200.0]), Some([0.05,0.45,0.18]), Some([8.0,22.0,7.0]), 0.0, 0.5)),
        // Barus camphor (Sumatra / Borneo) — a wet-tropical island forest resin.
        cg("camphor", "Camphor", "\u{2744}", "#e8f4f8", Domain::Coastal, Distribution::Endemic, 0.90, 0.50, true,
            None, env(vec![(1,1.0),(2,0.8)], Some([25.0,5.0]), Some([2000.0,4000.0,600.0]), Some([0.05,0.40,0.15]), Some([0.0,10.0,5.0]), 0.4, 0.3)),
        // Sumatran / Javanese benzoin — the same islands, drier margin.
        cg("benzoin", "Benzoin", "\u{1F36F}", "#c98a3c", Domain::Coastal, Distribution::Endemic, 0.88, 0.45, true,
            None, env(vec![(2,1.0),(1,0.7),(3,0.5)], Some([24.0,5.0]), Some([1400.0,3000.0,500.0]), Some([0.05,0.45,0.18]), Some([0.0,14.0,6.0]), 0.3, 0.3)),
        // Timor / Sumba sandalwood — the dry-monsoon island heartwood.
        cg("sandalwood", "Sandalwood", "\u{1F334}", "#c8a165", Domain::Coastal, Distribution::Endemic, 0.87, 0.55, true,
            None, env(vec![(3,1.0),(23,0.8),(2,0.5)], Some([26.0,5.0]), Some([700.0,1600.0,400.0]), Some([0.05,0.45,0.18]), Some([5.0,20.0,7.0]), 0.0, 0.4)),
        // ── NAVAL STORES ────────────────────────────────────────────────────
        // The two shipbuilding materials the world did not have. Both are
        // `Global` (every suitable cell produces), so their scarcity is
        // GEOGRAPHIC rather than a seeded homeland: a boreal coast is thick with
        // pitch and a tropical one has none, which is a trade dependency instead
        // of an accident of where one seed happened to land.
        //
        // THEY ARE MATERIALS, NOT REQUIREMENTS. The shipyard they are meant for
        // does not hold a fixed recipe. A hull is built from whatever suitable
        // construction material reaches the city — grown in its own hinterland or
        // landed on its quay — and the MIX sets what kind of ship comes out, not
        // whether one can be built at all. A fixed recipe would have locked the
        // tropics and the desert out of seafaring permanently, which is the exact
        // inverse of the history: Arabia is desert and was a great maritime
        // culture precisely BECAUSE it imported Malabar teak, and a teak hull
        // needs no paying at all because the wood's own oils do the job.
        //
        // So these two are worth having for what they add, not what they gate:
        // a boreal city with pitch and hemp builds a better, longer-lived vessel
        // than one paying with whatever it has — and a city with neither can
        // still build, or buy them in. `timber` and `iron` are both
        // `GOOD_UNLIMITED` and `hardwoods` now is too, so hull material is
        // reachable everywhere and these are the goods that make it GOOD.
        cg("pitch", "Pitch & Tar", "\u{1F6E2}", "#3b2f2a", Domain::Continental, Distribution::Global, 0.35, 0.40, false,
            None, env(vec![(16,1.0),(15,0.9),(14,0.6),(12,0.4)], Some([2.0,12.0]), Some([350.0,1200.0,350.0]), Some([0.0,0.45,0.20]), Some([45.0,70.0,12.0]), 0.2, 0.2)),
        cg("hemp", "Hemp", "\u{1F33F}", "#7f8f5a", Domain::Continental, Distribution::Global, 0.35, 0.40, false,
            None, env(vec![(12,1.0),(15,0.9),(11,0.8),(14,0.7),(9,0.5),(8,0.4)], Some([13.0,9.0]), Some([500.0,1300.0,350.0]), Some([0.0,0.35,0.18]), Some([30.0,60.0,14.0]), 0.6, 0.0)),
        // ── Extreme-rare luxuries (harsh placement: tiny homelands / few deposits) ──
        cg("cinnamon", "Cinnamon", "\u{1F33F}", "#a9603a", Domain::Coastal, Distribution::Local, 0.72, 0.50, true,
            None, env(vec![(2,1.0),(1,0.8),(3,0.6)], Some([27.0,6.0]), Some([1200.0,3000.0,400.0]), None, Some([0.0,15.0,6.0]), 0.0, 0.5)),
        cg("saffron", "Saffron", "\u{1F33C}", "#f4c430", Domain::Continental, Distribution::Local, 0.88, 0.55, true,
            None, env(vec![(8,1.0),(9,0.9),(7,0.7),(19,0.6)], Some([18.0,7.0]), Some([300.0,700.0,200.0]), Some([0.12,0.45,0.15]), Some([30.0,42.0,7.0]), 0.0, 0.0)),
        cg("tyrian_purple", "Tyrian Purple", "\u{1F40C}", "#6a0dad", Domain::Coastal, Distribution::Deposits, 0.90, 0.55, true,
            dep(0.0, 1, 3), env(vec![(8,1.0),(9,1.0),(11,0.7),(4,0.6)], Some([22.0,8.0]), None, None, Some([28.0,42.0,6.0]), 0.0, 1.0)),
        cg("ambergris", "Ambergris", "\u{1F40B}", "#cfc0a0", Domain::Marine, Distribution::Deposits, 0.92, 0.60, true,
            dep(0.0, 1, 4), env(vec![], Some([12.0,10.0]), None, None, Some([30.0,65.0,12.0]), 0.0, 0.0)),
        cg("jade", "Jade", "\u{2618}", "#00a86b", Domain::Continental, Distribution::Deposits, 0.90, 0.55, true,
            dep(0.40, 1, 3), env(vec![], None, None, Some([0.40,1.0,0.12]), None, 0.0, 0.0)),
        // ── Gem types — the generic "gemstones" belt SPLIT into distinct gems, each
        //    a deposit good with its OWN ore-province noise (`province_scale`) +
        //    elevation floor, so ruby ranges ≠ sapphire ranges (real gem geology).
        //    Category "gem" is fungible → a jeweller's "gemstones" input takes any of
        //    them. Each is terroir-graded (clarity) by the quality system. Disable the
        //    generic "gemstones" in the editor to show only these. ──
        cg("ruby", "Ruby", "\u{1F534}", "#e0294a", Domain::Continental, Distribution::Deposits, 0.80, 0.62, true,
            dep(0.42, 1, 2), env(vec![], Some([26.0,12.0]), None, Some([0.42,1.0,0.14]), None, 0.0, 0.0)),
        cg("sapphire", "Sapphire", "\u{1F537}", "#2a5fd0", Domain::Continental, Distribution::Deposits, 0.80, 0.60, true,
            dep(0.46, 1, 2), env(vec![], None, None, Some([0.46,1.0,0.13]), None, 0.0, 0.0)),
        cg("emerald", "Emerald", "\u{1F49A}", "#1ea866", Domain::Continental, Distribution::Deposits, 0.82, 0.60, true,
            dep(0.38, 1, 2), env(vec![], Some([22.0,14.0]), None, Some([0.38,0.9,0.14]), None, 0.0, 0.0)),
        cg("diamond", "Diamond", "\u{1F48E}", "#dfe6ee", Domain::Continental, Distribution::Deposits, 0.90, 0.70, true,
            dep(0.55, 1, 3), env(vec![], None, None, Some([0.55,1.0,0.12]), None, 0.0, 0.0)),
        cg("amethyst", "Amethyst", "\u{1F49C}", "#9b59d0", Domain::Continental, Distribution::Deposits, 0.66, 0.48, false,
            dep(0.34, 2, 1), env(vec![], None, None, Some([0.34,0.85,0.16]), None, 0.0, 0.0)),
        cg("topaz", "Topaz", "\u{1F538}", "#e0a92a", Domain::Continental, Distribution::Deposits, 0.62, 0.46, false,
            dep(0.30, 2, 1), env(vec![], None, None, Some([0.30,0.8,0.16]), None, 0.0, 0.0)),
        // ── Precious metals & quarried stone (high-desire deposit goods) ──
        // Silver: a prized monetary metal, a touch commoner than gold. Hill/mountain
        // deposits. High desire — every wealthy market wants coin metal.
        cg("silver", "Silver", "\u{1F948}", "#c8ccd6", Domain::Continental, Distribution::Deposits, 0.74, 0.65, true,
            dep(0.30, 2, 1), env(vec![], None, None, Some([0.30,1.0,0.16]), None, 0.0, 0.0)),
        // Marble: quarried building/sculpture stone of the uplands.
        cg("marble", "Marble", "\u{1F3DB}\u{FE0F}", "#e8e6e0", Domain::Continental, Distribution::Deposits, 0.62, 0.45, false,
            dep(0.28, 1, 1), env(vec![], None, None, Some([0.28,0.9,0.14]), None, 0.0, 0.0)),
        // Lead / tin-grey base metal (pewter, pipes, shot): low hills.
        cg("lead", "Lead", "\u{1F529}", "#8a8e96", Domain::Continental, Distribution::Deposits, 0.55, 0.40, false,
            dep(0.26, 2, 1), env(vec![], None, None, Some([0.26,0.85,0.16]), None, 0.0, 0.0)),

        // ── DEPOSITS_AND_MINING_PLAN slice 3: eight minerals, each demonstrating a
        //    deposit model or mechanic the shipped six/gem-split never exercised. ──
        // Mercury: near-monopoly volcanic-arc cinnabar (Almadán, Idrija). High value,
        // industrial rather than a luxury — amalgamation is a slice-4 mechanic, not
        // built here; see docs/DEPOSITS_AND_MINING_PLAN.md slice 3's own caveat.
        dg("mercury", "Mercury", "\u{1F4A7}", "#c0c4ca", Domain::Continental,
            0.85, 0.40, false, 1, 2, "metal", 1, 20.0, 1.5,
            env(vec![], None, None, Some([0.30,0.85,0.15]), None, 0.0, 0.0)),
        // Alum: the cloth mordant of Tolfa (1462). The dyeing-chain link is
        // historical flavour, not wired into the shipped `cloth` recipe this slice —
        // adding a hard input dependency to an existing manufactured good needs its
        // own econ_ measurement, which is slice 4 territory, not an add-only slice.
        dg("alum", "Alum", "\u{2697}\u{FE0F}", "#e8e0c0", Domain::Continental,
            0.50, 0.35, false, 1, 1, "craft", 1, 4.0, 2.0,
            env(vec![], None, None, Some([0.25,0.80,0.15]), None, 0.0, 0.0)),
        // Lapis lazuli: contact-metamorphic, and famously ONE source for four
        // thousand years (Sar-i-Sang) — districts=1 at the default deposit slider.
        dg("lapis_lazuli", "Lapis Lazuli", "\u{1F30C}", "#26619c", Domain::Continental,
            0.93, 0.60, true, 1, 6, "gem", 2, 40.0, 1.0,
            env(vec![], None, None, Some([0.40,1.0,0.12]), None, 0.0, 0.0)),
        // Turquoise: a DERIVED weathering mineral — supergene alteration of a
        // copper body under an arid climate (Nishapur, Sinai). No independent
        // search: `place_mineral` walks `default_parent_for("turquoise")` == copper.
        dg("turquoise", "Turquoise", "\u{1F4A0}", "#30d5c8", Domain::Continental,
            0.75, 0.50, true, 1, 3, "gem", 2, 18.0, 1.0,
            env(vec![(4,1.0),(5,0.9),(6,0.8),(7,0.7)], None, None, Some([0.15,0.55,0.15]), None, 0.0, 0.0)),
        // Bog iron: wetland precipitate iron, structurally impossible under an
        // elevation-floor placer — the iron source of early medieval N. Europe.
        dg("bog_iron", "Bog Iron", "\u{26CF}", "#5b4636", Domain::Continental,
            0.35, 0.30, false, 2, 1, "metal", 1, 2.0, 3.5,
            env(vec![(11,0.8),(12,1.0),(14,0.8),(15,0.9),(16,0.7)], None, None, Some([0.0,0.15,0.10]), None, 0.0, 0.0)),
        // Coal: a sedimentary-basin measure alongside limestone, already referenced
        // by `estate_kind_for_good`'s "coal" substring match but never a real good.
        dg("coal", "Coal", "\u{2B1B}", "#1c1c1c", Domain::Continental,
            0.45, 0.35, false, 2, 1, "fuel", 0, 1.2, 3.0,
            env(vec![], None, None, Some([0.0,0.30,0.15]), None, 0.0, 0.0)),
        // Garnet: the bottom rung of the gem ladder — common orogenic gem gravel.
        dg("garnet", "Garnet", "\u{2666}\u{FE0F}", "#8b0000", Domain::Continental,
            0.45, 0.42, false, 2, 1, "gem", 1, 6.0, 1.0,
            env(vec![], None, None, Some([0.30,0.80,0.16]), None, 0.0, 0.0)),
        // Carnelian: agate's warm cousin, filling the same basalt amygdules — the
        // great Khambhat bead trade.
        dg("carnelian", "Carnelian", "\u{1F53B}", "#b33009", Domain::Continental,
            0.50, 0.40, false, 2, 1, "gem", 1, 5.0, 1.0,
            env(vec![], None, None, Some([0.20,0.70,0.18]), None, 0.0, 0.0)),

        // ── Manufactured chain goods (the shipped recipe LIBRARY) — made in cities
        // from imported raws, no per-cell belt. The placement engine skips them; the
        // shared `apply_manufacturing` pass produces them at populous hubs that hold
        // the inputs. Edit these (or add your own) in the Goods Editor recipe rows. ──
        // Cloth ← fleece wool + a touch of dye. The classic "import raw wool, export
        // finished cloth" trade that made wool-poor weaving towns rich.
        mg("cloth", "Woolen Cloth", "\u{1F45A}", "#d8c8b0", 8.0, 1.4, 0.0, true, 1.2,
            vec![("wool_fleece", 1.0), ("dyes", 0.2)]),
        // Metalware & Arms ← iron + a little copper (tools, fittings, weapons).
        mg("metalware", "Metalware & Arms", "\u{2694}\u{FE0F}", "#9099a8", 9.0, 2.0, 0.0, false, 1.0,
            vec![("iron", 1.0), ("copper", 0.3)]),
        // Refined Sugar ← raw sugar cane (boiled & refined in port cities).
        mg("refined_sugar", "Refined Sugar", "\u{1F367}", "#f0e8d8", 7.0, 1.6, 0.0, true, 1.0,
            vec![("sugar", 1.2)]),
        // Citrus Liqueur ← citrus + sugar (a multi-stage chain: both raws, no map
        // belt of its own; distilled in cities).
        mg("citrus_liqueur", "Citrus Liqueur", "\u{1F378}", "#e8b24a", 14.0, 2.0, 0.02, true, 1.1,
            vec![("citrus", 1.0), ("sugar", 0.5)]),

        // ── Clay: a common low-elevation raw, the literal input to ceramics. Cheap
        //    and heavy/wet (high bulk → stays near its kiln cities). Category set
        //    explicitly so backfill leaves its value/bulk alone. ──
        GoodSpec {
            id: "clay".into(), name: "Clay".into(), icon: "\u{1F9F1}".into(), color: "#b07a52".into(),
            enabled: true, domain: Domain::Continental, distribution: Distribution::Global,
            rarity: 0.40, desire: 0.35, network_luxury: false, builtin: false, deposit: None,
            marine_band: MarineBand::Either,
            origins: 1,
            soil: Vec::new(),
            relief: None,
            scoring: Some(env(vec![], None, None, Some([0.0, 0.30, 0.20]), None, 0.5, 0.0)),
            category: "construction".into(), need_tier: 1, base_value: 0.5,
            bulk: 2.0, perishable: 0.0, inputs: Vec::new(), labor: 1.0,
            consumption_interval: 90.0,
        },

        // ── Ported richer manufactured library (on top of the PR's cloth / metalware
        //    / refined_sugar / citrus_liqueur). All hub-made, no map belt. ──
        // Textiles
        mg("linen", "Linen", "\u{1F454}", "#cfe0e8", 5.0, 0.7, 0.0, false, 1.1,
            vec![("flax", 1.0)]),
        // Each cloth is woven from ONE fibre (cotton→cotton cloth, flax→linen,
        // wool→woolen cloth, silk→brocade); they are a "Cloth family" of distinct
        // goods, not a blend.
        mg("cotton_cloth", "Cotton Cloth", "\u{1F455}", "#eef0e8", 5.0, 0.7, 0.0, false, 1.1,
            vec![("cotton", 1.0)]),
        mg("silk_brocade", "Fine Silk Brocade", "\u{1F458}", "#d96fb0", 28.0, 0.25, 0.0, true, 1.0,
            vec![("silk", 1.0), ("dyes", 0.2), ("gold", 0.02)]),
        mg("carpets", "Carpets & Tapestry", "\u{1F3F5}", "#9b4f2f", 11.0, 1.0, 0.0, true, 1.0,
            vec![("wool_fleece", 1.2), ("dyes", 0.3)]),
        mg("leather_goods", "Leather Goods", "\u{1F45E}", "#7a4a2a", 4.0, 0.9, 0.0, false, 1.0,
            vec![("hides", 1.0), ("salt", 0.1)]),
        // Salted Herring ← fresh herring + salt (the preservative). Fresh herring
        // (built-in "herring") is highly perishable and eaten at its origin port;
        // ONLY once salted in a curing town does it keep long enough to be shipped
        // across the network. Salt (rock salt) is the cure — bay salt is an
        // interchangeable preservative once category-substitution covers inputs.
        // Cheap bulk protein, no luxury, low labor (the great Hanseatic staple).
        mg("salted_herring", "Salted Herring", "\u{1F9C2}", "#88b8c0", 3.0, 1.5, 0.0, false, 0.9,
            vec![("herring", 1.0), ("salt", 0.3)]),
        // Metalwork
        mg("bronzeware", "Bronzeware", "\u{1F514}", "#b06a3a", 6.0, 1.4, 0.0, false, 1.0,
            vec![("copper", 0.9), ("tin", 0.1)]),
        mg("jewelry", "Fine Jewelry", "\u{1F48D}", "#d4af37", 45.0, 0.1, 0.0, true, 1.0,
            vec![("gold", 0.3), ("silver", 0.3), ("gemstones", 0.1)]),
        // Refined drink
        mg("brandy", "Brandy & Spirits", "\u{1F943}", "#a9603a", 10.0, 0.5, 0.0, true, 1.0,
            vec![("wine", 1.0)]),
        mg("mead", "Mead", "\u{1F36F}", "#e0a020", 4.0, 1.2, 0.05, false, 1.0,
            vec![("honey", 1.0)]),
        // Aromatics, cosmetics & craft luxuries
        mg("perfume", "Perfume & Attar", "\u{1F338}", "#c79a4b", 30.0, 0.15, 0.0, true, 1.0,
            vec![("frankincense", 0.3), ("saffron", 0.05), ("oliveoil", 0.2)]),
        mg("soap", "Soap", "\u{1F6C1}", "#cfe0d8", 3.0, 0.9, 0.0, false, 1.0,
            vec![("oliveoil", 0.6), ("salt", 0.2)]),
        mg("candles", "Candles & Wax Goods", "\u{1F56F}\u{FE0F}", "#e8d8a0", 3.0, 0.8, 0.0, false, 1.0,
            vec![("honey", 0.8)]),
        mg("books", "Books & Manuscripts", "\u{1F4DA}", "#8a6a3a", 20.0, 0.5, 0.0, true, 1.0,
            vec![("paper", 0.8), ("dyes", 0.1)]),
        mg("furniture", "Fine Furniture", "\u{1F6CB}", "#6b4226", 5.0, 2.0, 0.0, false, 1.0,
            vec![("hardwoods", 1.0), ("iron", 0.1)]),
        mg("ivory_carvings", "Ivory Carvings", "\u{265F}\u{FE0F}", "#efe6d0", 25.0, 0.5, 0.0, true, 1.0,
            vec![("ivory", 1.0)]),
        mg("statuary", "Statuary", "\u{1F5FF}", "#e8e6e0", 8.0, 3.0, 0.0, false, 1.0,
            vec![("marble", 1.0)]),
    ]
}

/// CLAUDE.md §8.19 (goods localities, shipped) §2.1's default table. Every other marine/coastal good
/// stays `Either` — the pre-existing, undifferentiated `sea_coastal` gate.
pub fn default_marine_band_for(id: &str) -> MarineBand {
    match id {
        "pearls" | "coral" | "bay_salt" | "tyrian_purple" | "amber" => MarineBand::Inshore,
        "stockfish" | "herring" | "whaling" => MarineBand::Bank,
        _ => MarineBand::Either,
    }
}

/// How many independent homelands a good seeds, by id — the historically-grounded
/// defaults for `GoodSpec.origins`. Anything not named here gets 1, which is the
/// old behaviour, so this table is purely additive.
///
/// Every entry is a real case of independent or separately-diffused origin, not a
/// variety knob:
///   • `pepper`     Malabar and Sumatra — two distinct trades that reached Europe
///                  by two different routes.
///   • `cinnamon`   Ceylon (true cinnamon) and Chinese cassia — different species
///                  sold as one commodity, which is exactly what a second origin
///                  of the same good models.
///   • `cotton`     independently domesticated in India, Peru AND the Levant.
///   • `spices`     the generic aromatic basket was never one place: three.
///   • `incense`    Hadhramaut and the Horn.
///   • `silk`       China, then Byzantium (552) and Italy — diffusion the model
///                  can't animate over time, but CAN express as several homelands.
///   • `sugar`      New Guinea → India → the Mediterranean.
///   • `coffee`     Kaffa and Yemen.
///   • `wine`/`oliveoil`/`horses`/`hides`/`honey`  broad domesticates that had
///                  many separate centres; two apiece keeps them from reading as
///                  a single improbable world monopoly.
pub fn default_origins_for(id: &str) -> u8 {
    match id {
        "cotton" | "spices" => 3,
        "pepper" | "cinnamon" | "incense" | "silk" | "sugar" | "coffee" | "wine" | "oliveoil"
        | "horses" | "hides" | "honey" | "flax" | "tea" => 2,
        _ => 1,
    }
}

/// A deterministic per-good salt for placement (seeded RNG), derived from the id
/// so custom goods scatter independently of the built-ins.
pub fn id_salt(id: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
