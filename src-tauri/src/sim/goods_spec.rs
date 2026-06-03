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
    deposit_params, GOOD_COLOR, GOOD_DESIRE, GOOD_ICON, GOOD_LABEL, GOOD_MARINE, GOOD_NAMES,
    GOOD_NETWORK_LUXURY, GOOD_RARITY, GOOD_UNLIMITED,
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
}

/// Parameters for a `Deposits`-distributed good.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DepositSpec {
    pub min_elev: f32,
    pub count_num: u32,
    pub count_den: u32,
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
}

/// Index of a built-in good by id (its column / hardcoded-scorer index), or None
/// for custom goods.
pub fn builtin_index_of(id: &str) -> Option<usize> {
    GOOD_NAMES.iter().position(|&n| n == id)
}

/// The shipped 30-good library, seeded from the `biological.rs` const tables so
/// that spec-driven generation is byte-identical to the pre-spec behavior.
pub fn default_list() -> Vec<GoodSpec> {
    (0..GOODS_COUNT)
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
                enabled: true,
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
                }),
                scoring: None,
            }
        })
        .collect()
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
