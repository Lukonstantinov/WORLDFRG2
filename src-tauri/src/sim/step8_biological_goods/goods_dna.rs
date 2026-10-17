//! GOODS DNA — the six properties every good is described by
//! (`docs/GENERATOR_AND_GOODS_REDESIGN.md` §4.1, owner decisions §7/§12).
//!
//! Role (why people want it) · Origin (how it is obtained) · Geography (the existing
//! `Distribution`, not repeated here) · Value tier (per weight) · Difficulty 1-5 ·
//! Keeping. Rarity stops being one free slider and becomes an outcome of
//! Geography × Difficulty × Tier.
//!
//! A spec carries `dna: Option<GoodDna>`. `None` means "derive": shipped goods read
//! the hand-authored table below, custom goods a heuristic over their other fields.
//! `backfill_dna` fills it once (the same way `backfill_market_fields` fills the
//! market fields), so the Goods Builder edits a concrete value.

use serde::{Deserialize, Serialize};

use super::goods_spec::{Distribution, Domain, GoodSpec};

/// Why people want a good. Drives who buys it and what substitutes for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoodRole {
    /// Eaten every day; a city without it starves.
    Staple,
    /// Needed to live and work: salt, fibre, fuel, building stuff.
    Essential,
    /// Needed for tools, war and ships.
    Strategic,
    /// Makes life better; bought when there is money.
    Comfort,
    /// Status and pleasure; bought by the rich, carried far.
    Luxury,
    /// Store of value: hoarded, minted, given. Never eaten, never a tool.
    Treasure,
}

/// How a good is obtained. Each origin fails in its own way (plan §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoodOrigin {
    Grown,
    Raised,
    Gathered,
    Caught,
    Mined,
    Crafted,
}

/// How long a good keeps. Eternal goods can serve as money.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoodKeeping {
    Perishable,
    Seasonal,
    Storable,
    Eternal,
}

/// Value per weight, as a tier: 0 Bulk ×1 · 1 Common ×3-10 · 2 Valuable ×30-100 ·
/// 3 Precious ×300-1,000 · 4 Treasure ×2,000+ (multiples of grain by weight).
pub const TIER_BULK: u8 = 0;
pub const TIER_COMMON: u8 = 1;
pub const TIER_VALUABLE: u8 = 2;
pub const TIER_PRECIOUS: u8 = 3;
pub const TIER_TREASURE: u8 = 4;

/// Treasure-role goods are produced at this share of the volume the world economy
/// would otherwise give them (goods batch G2, `compute_economy`): a rich gold
/// district yields a fraction of a grain belt and earns by value instead.
pub const TREASURE_VOLUME: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoodDna {
    pub role: GoodRole,
    pub origin: GoodOrigin,
    /// 0..=4, see `TIER_*`.
    pub tier: u8,
    /// 1 (anyone can) ..= 5 (specialists, capital, years).
    pub difficulty: u8,
    pub keeping: GoodKeeping,
}

/// The hand-authored classification of every shipped good. `None` for an id the
/// table does not know (a custom good), which then falls to `derive_dna`'s heuristic.
fn shipped_dna(id: &str) -> Option<GoodDna> {
    use GoodKeeping::*;
    use GoodOrigin::*;
    use GoodRole::*;
    let (role, origin, tier, difficulty, keeping) = match id {
        // ── Staples ──
        "wheat" | "barley" | "millet" => (Staple, Grown, TIER_BULK, 2, Storable),
        "rice" => (Staple, Grown, TIER_BULK, 3, Storable),
        "dates" => (Staple, Grown, TIER_BULK, 2, Seasonal),
        "stockfish" => (Staple, Caught, TIER_COMMON, 2, Storable),
        "salted_herring" => (Staple, Crafted, TIER_COMMON, 2, Storable),
        "herring" => (Staple, Caught, TIER_BULK, 2, Perishable),
        // ── Essentials ──
        "salt" | "bay_salt" => (Essential, Mined, TIER_BULK, 2, Eternal),
        "timber" => (Essential, Gathered, TIER_BULK, 1, Storable),
        "clay" => (Essential, Mined, TIER_BULK, 1, Eternal),
        "coal" => (Essential, Mined, TIER_BULK, 2, Eternal),
        "marble" => (Essential, Mined, TIER_COMMON, 3, Eternal),
        "wool_fleece" | "wool_llama" => (Essential, Raised, TIER_COMMON, 2, Storable),
        "cotton" | "flax" => (Essential, Grown, TIER_COMMON, 2, Storable),
        "hides" => (Essential, Raised, TIER_COMMON, 1, Storable),
        "cloth" | "linen" | "cotton_cloth" => (Essential, Crafted, TIER_COMMON, 3, Storable),
        "ceramics" => (Essential, Crafted, TIER_COMMON, 3, Eternal),
        // ── Strategic ──
        "iron" => (Strategic, Mined, TIER_COMMON, 3, Storable),
        "bog_iron" => (Strategic, Mined, TIER_BULK, 2, Storable),
        "copper" | "lead" => (Strategic, Mined, TIER_COMMON, 3, Eternal),
        "tin" => (Strategic, Mined, TIER_COMMON, 3, Eternal),
        "mercury" => (Strategic, Mined, TIER_VALUABLE, 4, Eternal),
        "alum" => (Strategic, Mined, TIER_COMMON, 3, Eternal),
        "horses" => (Strategic, Raised, TIER_VALUABLE, 3, Seasonal),
        "hardwoods" => (Strategic, Gathered, TIER_COMMON, 2, Storable),
        "pitch" | "hemp" => (Strategic, Gathered, TIER_BULK, 2, Storable),
        "metalware" | "bronzeware" => (Strategic, Crafted, TIER_VALUABLE, 3, Eternal),
        "armour" => (Strategic, Crafted, TIER_VALUABLE, 4, Eternal),
        "sailcloth" | "cordage" => (Strategic, Crafted, TIER_COMMON, 3, Storable),
        // ── Comforts ──
        "wine" => (Comfort, Grown, TIER_COMMON, 3, Storable),
        "oliveoil" => (Comfort, Grown, TIER_COMMON, 3, Storable),
        "sugar" => (Comfort, Grown, TIER_VALUABLE, 3, Storable),
        "honey" => (Comfort, Raised, TIER_COMMON, 1, Eternal),
        "beer" => (Comfort, Crafted, TIER_BULK, 2, Perishable),
        "mead" => (Comfort, Crafted, TIER_COMMON, 2, Storable),
        "tea" => (Comfort, Grown, TIER_VALUABLE, 3, Seasonal),
        "coffee" | "cacao" => (Comfort, Grown, TIER_VALUABLE, 3, Storable),
        "citrus" => (Comfort, Grown, TIER_COMMON, 3, Perishable),
        "tobacco" => (Comfort, Grown, TIER_VALUABLE, 3, Storable),
        "whaling" => (Comfort, Caught, TIER_COMMON, 4, Storable),
        "garum" => (Comfort, Crafted, TIER_COMMON, 2, Storable),
        "glassware" => (Comfort, Crafted, TIER_VALUABLE, 4, Eternal),
        "paper" | "parchment" => (Comfort, Crafted, TIER_VALUABLE, 4, Storable),
        "leather_goods" | "soap" | "candles" | "furniture" => (Comfort, Crafted, TIER_COMMON, 3, Storable),
        "refined_sugar" => (Comfort, Crafted, TIER_VALUABLE, 3, Storable),
        // ── Luxuries ──
        "silk" => (Luxury, Raised, TIER_PRECIOUS, 5, Storable),
        "spices" | "pepper" => (Luxury, Grown, TIER_VALUABLE, 3, Storable),
        "cloves" | "cinnamon" => (Luxury, Grown, TIER_PRECIOUS, 3, Storable),
        "nutmeg" | "mace" => (Luxury, Grown, TIER_PRECIOUS, 3, Storable),
        "saffron" => (Luxury, Grown, TIER_PRECIOUS, 4, Storable),
        "frankincense" | "incense" | "benzoin" => (Luxury, Gathered, TIER_VALUABLE, 3, Storable),
        "camphor" | "sandalwood" | "dragons_blood" => (Luxury, Gathered, TIER_PRECIOUS, 3, Storable),
        "dyes" | "tyrian_purple" => (Luxury, Caught, TIER_PRECIOUS, 4, Eternal),
        "indigo" => (Luxury, Grown, TIER_VALUABLE, 3, Storable),
        "fixed_dye" => (Luxury, Crafted, TIER_VALUABLE, 4, Storable),
        "furs" => (Luxury, Gathered, TIER_VALUABLE, 3, Storable),
        "ivory" => (Luxury, Gathered, TIER_PRECIOUS, 4, Eternal),
        "amber" => (Luxury, Gathered, TIER_VALUABLE, 2, Eternal),
        "coral" => (Luxury, Caught, TIER_VALUABLE, 4, Eternal),
        "ambergris" => (Luxury, Gathered, TIER_PRECIOUS, 4, Eternal),
        "jade" | "lapis_lazuli" | "turquoise" => (Luxury, Mined, TIER_PRECIOUS, 4, Eternal),
        "amethyst" | "topaz" | "garnet" | "carnelian" => (Luxury, Mined, TIER_VALUABLE, 3, Eternal),
        "perfume" => (Luxury, Crafted, TIER_PRECIOUS, 4, Storable),
        "silk_brocade" | "carpets" => (Luxury, Crafted, TIER_PRECIOUS, 5, Storable),
        "books" => (Luxury, Crafted, TIER_VALUABLE, 4, Storable),
        "brandy" | "citrus_liqueur" => (Luxury, Crafted, TIER_VALUABLE, 3, Storable),
        "statuary" | "ivory_carvings" => (Luxury, Crafted, TIER_PRECIOUS, 4, Eternal),
        // ── Treasure ──
        "gold" => (Treasure, Mined, TIER_TREASURE, 4, Eternal),
        "silver" => (Treasure, Mined, TIER_TREASURE, 4, Eternal),
        "gemstones" | "ruby" | "sapphire" | "emerald" => (Treasure, Mined, TIER_TREASURE, 4, Eternal),
        "diamond" => (Treasure, Mined, TIER_TREASURE, 5, Eternal),
        "pearls" => (Treasure, Caught, TIER_TREASURE, 4, Eternal),
        "jewelry" => (Treasure, Crafted, TIER_TREASURE, 4, Eternal),
        _ => return None,
    };
    Some(GoodDna { role, origin, tier, difficulty, keeping })
}

/// Best guess for a good the shipped table does not know, from the fields every
/// spec already has. Only a starting point: the author edits it in the builder.
fn heuristic_dna(spec: &GoodSpec) -> GoodDna {
    let origin = match spec.distribution {
        Distribution::Manufactured => GoodOrigin::Crafted,
        Distribution::Deposits => GoodOrigin::Mined,
        _ if spec.domain == Domain::Marine => GoodOrigin::Caught,
        _ => GoodOrigin::Grown,
    };
    let v = spec.base_value;
    let tier = if v < 1.5 { TIER_BULK } else if v < 5.0 { TIER_COMMON } else if v < 15.0 { TIER_VALUABLE } else if v < 40.0 { TIER_PRECIOUS } else { TIER_TREASURE };
    let role = match (spec.need_tier, spec.category.as_str()) {
        (_, "cereal" | "protein") => GoodRole::Staple,
        _ if tier >= TIER_TREASURE => GoodRole::Treasure,
        (_, "metal") => GoodRole::Strategic,
        (_, "gem") if tier >= TIER_PRECIOUS => GoodRole::Treasure,
        (0, _) => GoodRole::Essential,
        (1, _) => GoodRole::Comfort,
        _ => GoodRole::Luxury,
    };
    let keeping = if spec.perishable > 0.1 {
        GoodKeeping::Perishable
    } else if spec.perishable > 0.015 {
        GoodKeeping::Seasonal
    } else if origin == GoodOrigin::Mined && tier >= TIER_VALUABLE {
        GoodKeeping::Eternal
    } else {
        GoodKeeping::Storable
    };
    let difficulty = (1.0 + spec.rarity.clamp(0.0, 1.0) * 4.0).round() as u8;
    GoodDna { role, origin, tier, difficulty: difficulty.clamp(1, 5), keeping }
}

/// The DNA a spec has: its own if set, else the shipped table, else the heuristic.
pub fn dna_of(spec: &GoodSpec) -> GoodDna {
    spec.dna.or_else(|| shipped_dna(&spec.id)).unwrap_or_else(|| heuristic_dna(spec))
}

/// Fill `dna` on every spec that has none (old saves, imported libraries).
pub fn backfill_dna(specs: &mut [GoodSpec]) {
    for spec in specs.iter_mut() {
        if spec.dna.is_none() {
            spec.dna = Some(dna_of(spec));
        }
    }
}

#[cfg(test)]
mod goods_dna_tests {
    use super::*;
    use crate::sim::goods_spec::default_list;

    #[test]
    fn goods_dna_every_shipped_good_is_classified_by_hand() {
        let list = default_list();
        let missing: Vec<&str> = list.iter().filter(|s| shipped_dna(&s.id).is_none()).map(|s| s.id.as_str()).collect();
        assert!(missing.is_empty(), "shipped goods with no hand classification: {missing:?}");
    }

    #[test]
    fn goods_dna_treasure_is_not_a_tool_metal() {
        let list = default_list();
        let get = |id: &str| dna_of(list.iter().find(|s| s.id == id).unwrap());
        for id in ["gold", "silver", "diamond", "pearls"] {
            let d = get(id);
            assert_eq!(d.role, GoodRole::Treasure, "{id}");
            assert_eq!(d.keeping, GoodKeeping::Eternal, "{id}");
            assert_eq!(d.tier, TIER_TREASURE, "{id}");
        }
        assert_eq!(get("iron").role, GoodRole::Strategic);
        assert_eq!(get("wheat").role, GoodRole::Staple);
        // Crafted goods are made, never dug or grown.
        for s in list.iter().filter(|s| s.distribution == Distribution::Manufactured) {
            assert_eq!(dna_of(s).origin, GoodOrigin::Crafted, "{}", s.id);
        }
    }

    /// G2: treasure is worth treasure and is bullion, not a tool metal; the repair
    /// also applies to a world saved with the old values.
    #[test]
    fn goods_treasure_has_real_value_and_its_own_category() {
        let list = default_list();
        let get = |id: &str| list.iter().find(|s| s.id == id).unwrap();
        let (gold, silver) = (get("gold"), get("silver"));
        assert_eq!((gold.category.as_str(), silver.category.as_str()), ("bullion", "bullion"));
        assert!((gold.base_value / silver.base_value - 10.0).abs() < 0.01, "gold:silver 1:10");
        for id in ["silver", "ruby", "diamond", "saffron", "nutmeg", "tyrian_purple"] {
            assert!(get(id).base_value > 1.0, "{id} must not be worth exactly a unit of grain");
        }
        let mut old = vec![silver.clone()];
        old[0].base_value = 1.0;
        old[0].category = "metal".into();
        crate::sim::goods_spec::backfill_market_fields(&mut old);
        assert_eq!((old[0].base_value, old[0].category.as_str()), (5.0, "bullion"));
    }

    #[test]
    fn goods_dna_heuristic_reads_the_spec() {
        let mut s = default_list().into_iter().find(|s| s.id == "gold").unwrap();
        s.id = "moon_gold".into();
        let d = dna_of(&s);
        assert_eq!(d.origin, GoodOrigin::Mined);
        assert_eq!(d.tier, TIER_TREASURE);
        assert_eq!(d.role, GoodRole::Treasure);
        assert!((1..=5).contains(&d.difficulty));
    }
}
