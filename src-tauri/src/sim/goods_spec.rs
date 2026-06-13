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
    GOOD_RARITY, GOOD_UNLIMITED, GOOD_FRANKINCENSE, GOOD_INDIGO, GOOD_TOBACCO,
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Distribution {
    /// Every suitable cell produces (the old UNLIMITED model).
    Global,
    /// One suitability-weighted homeland, flood-filled (the old SEEDED model).
    Local,
    /// Discrete highland-locked blobs scattered worldwide (gems / metals).
    Deposits,
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

/// Parameters for a `Deposits`-distributed good.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DepositSpec {
    pub min_elev: f32,
    pub count_num: u32,
    pub count_den: u32,
    /// Low-frequency noise frequency for this mineral's ore-province field, so each
    /// deposit good lights up its OWN mountain ranges instead of all clustering on
    /// the single tallest one. Larger = smaller, more scattered provinces.
    #[serde(default = "default_province_scale")]
    pub province_scale: f32,
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
}

fn default_base_value() -> f32 {
    1.0
}

fn default_bulk() -> f32 {
    1.0
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
                enabled: !matches!(g, GOOD_TOBACCO | GOOD_FRANKINCENSE | GOOD_INDIGO),
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
                }),
                scoring: None,
                category: GOOD_CATEGORY[g].to_string(),
                need_tier: GOOD_NEED_TIER[g],
                base_value: GOOD_BASE_VALUE[g],
                bulk: GOOD_BULK[g],
                perishable: GOOD_PERISH[g],
                inputs: Vec::new(),
                labor: 1.0,
            }
        })
        .collect();
    list.extend(default_custom_goods());
    // Customs are built with empty market fields; fill them from the id map.
    backfill_market_fields(&mut list);
    list
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
            category: String::new(), need_tier: 0, base_value: 1.0,
            bulk: 1.0, perishable: 0.0, inputs: Vec::new(), labor: 1.0,
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
            scoring: None, category: String::new(), need_tier: 0, base_value,
            bulk, perishable: perish, labor,
            inputs: inputs.into_iter().map(|(g, q)| RecipeInput { good: g.into(), qty: q }).collect(),
        }
    }
    let dep = |min_elev: f32, num: u32, den: u32| Some(DepositSpec { min_elev, count_num: num, count_den: den, province_scale: default_province_scale() });
    let env = |climate: Vec<(u8, f32)>, temp: Option<[f32; 2]>, precip: Option<[f32; 3]>,
               elevation: Option<[f32; 3]>, abs_lat: Option<[f32; 3]>, fertility: f32, coast_bonus: f32| Envelope {
        climate, temp, precip, elevation, abs_lat, fertility, coast_bonus,
    };

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
        cg("flax", "Flax / Linen", "\u{1F9F5}", "#cfe0e8", Domain::Continental, Distribution::Local, 0.50, 0.40, false,
            None, env(vec![(12,1.0),(11,0.7),(15,0.8),(14,0.7)], Some([14.0,7.0]), Some([500.0,1200.0,300.0]), None, None, 0.5, 0.0)),
        cg("coral", "Red Coral", "\u{1FAB8}", "#ff6f61", Domain::Marine, Distribution::Local, 0.60, 0.40, true,
            None, env(vec![], Some([26.0,5.0]), None, None, Some([0.0,30.0,8.0]), 0.0, 0.6)),
        // ── Extreme-rare luxuries (harsh placement: tiny homelands / few deposits) ──
        cg("cinnamon", "Cinnamon", "\u{1F33F}", "#a9603a", Domain::Coastal, Distribution::Local, 0.72, 0.50, true,
            None, env(vec![(2,1.0),(1,0.8),(3,0.6)], Some([27.0,6.0]), Some([1200.0,3000.0,400.0]), None, Some([0.0,15.0,6.0]), 0.0, 0.5)),
        cg("saffron", "Saffron", "\u{1F33C}", "#f4c430", Domain::Continental, Distribution::Local, 0.88, 0.55, true,
            None, env(vec![(8,1.0),(9,0.9),(7,0.7),(19,0.6)], Some([18.0,7.0]), Some([300.0,700.0,200.0]), Some([0.12,0.45,0.15]), Some([30.0,42.0,7.0]), 0.0, 0.0)),
        cg("tyrian_purple", "Tyrian Purple", "\u{1F7E3}", "#6a0dad", Domain::Coastal, Distribution::Deposits, 0.90, 0.55, true,
            dep(0.0, 1, 3), env(vec![(8,1.0),(9,1.0),(11,0.7),(4,0.6)], Some([22.0,8.0]), None, None, Some([28.0,42.0,6.0]), 0.0, 1.0)),
        cg("ambergris", "Ambergris", "\u{1F40B}", "#cfc0a0", Domain::Marine, Distribution::Deposits, 0.92, 0.60, true,
            dep(0.0, 1, 4), env(vec![], Some([12.0,10.0]), None, None, Some([30.0,65.0,12.0]), 0.0, 0.0)),
        cg("jade", "Jade", "\u{1F7E2}", "#00a86b", Domain::Continental, Distribution::Deposits, 0.90, 0.55, true,
            dep(0.40, 1, 3), env(vec![], None, None, Some([0.40,1.0,0.12]), None, 0.0, 0.0)),
        // ── Precious metals & quarried stone (high-desire deposit goods) ──
        // Silver: a prized monetary metal, a touch commoner than gold. Hill/mountain
        // deposits. High desire — every wealthy market wants coin metal.
        cg("silver", "Silver", "\u{1FA99}", "#c8ccd6", Domain::Continental, Distribution::Deposits, 0.74, 0.65, true,
            dep(0.30, 2, 1), env(vec![], None, None, Some([0.30,1.0,0.16]), None, 0.0, 0.0)),
        // Marble: quarried building/sculpture stone of the uplands.
        cg("marble", "Marble", "\u{1F3DB}\u{FE0F}", "#e8e6e0", Domain::Continental, Distribution::Deposits, 0.62, 0.45, false,
            dep(0.28, 1, 1), env(vec![], None, None, Some([0.28,0.9,0.14]), None, 0.0, 0.0)),
        // Lead / tin-grey base metal (pewter, pipes, shot): low hills.
        cg("lead", "Lead", "\u{1F529}", "#8a8e96", Domain::Continental, Distribution::Deposits, 0.55, 0.40, false,
            dep(0.26, 2, 1), env(vec![], None, None, Some([0.26,0.85,0.16]), None, 0.0, 0.0)),
        // ── Manufactured chain goods (the shipped recipe LIBRARY) — made in cities
        // from imported raws, no per-cell belt. The placement engine skips them; the
        // shared `apply_manufacturing` pass produces them at populous hubs that hold
        // the inputs. Edit these (or add your own) in the Goods Editor recipe rows. ──
        // Cloth ← fleece wool + a touch of dye. The classic "import raw wool, export
        // finished cloth" trade that made wool-poor weaving towns rich.
        mg("cloth", "Cloth", "\u{1F9F6}", "#d8c8b0", 8.0, 1.4, 0.0, true, 1.2,
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
    ]
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
