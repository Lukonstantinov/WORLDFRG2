//! CULTURAL VALUATIONS (goods batch G7, `docs/GENERATOR_AND_GOODS_REDESIGN.md` §9.2).
//!
//! Prices already differ by region through supply and transport. On top of that,
//! each people values a few goods its own way — not what they cost to make, but how
//! much that people WANTS them:
//!
//! | kind      | effect on the city's demand (scaled by the people's share)    |
//! |-----------|---------------------------------------------------------------|
//! | Sacred    | steady extra demand (temples buy in famine years too)          |
//! | Prestige  | elites pay more and display it (the old "cultural taste")      |
//! | Staple    | the people's own staple, preferred over its substitutes        |
//! | Taboo     | no demand at all (the good does not sell there)                |
//! | Currency  | which treasure metal it mints and hoards                       |
//!
//! A profile is derived by default from the people's kit tastes and traits
//! (`auto_values`); the author can replace any people's profile by hand in the
//! Goods Builder (stored in world metadata `culture_values`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    Sacred,
    Prestige,
    Staple,
    Taboo,
    Currency,
}

/// One people's valuation of one good.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CultureValue {
    pub culture: String,
    pub good: String,
    pub kind: ValueKind,
}

/// Demand lift per kind at a 100% share (Prestige equals the old cultural-taste
/// boost, so a people with only kit tastes behaves exactly as before). Taboo and
/// Currency are not lifts: Taboo scales demand by (1 - share); Currency picks a mint.
pub fn lift(kind: ValueKind) -> f32 {
    match kind {
        ValueKind::Prestige => 0.4,
        ValueKind::Sacred => 0.3,
        ValueKind::Staple => 0.25,
        ValueKind::Taboo | ValueKind::Currency => 0.0,
    }
}

// Trait indices into `cultures::TRAITS`.
const MERCANTILE: usize = 0;
const MARTIAL: usize = 3;
const DEVOUT: usize = 4;
const NOMADIC: usize = 5;
const PASTORAL: usize = 11;

/// The default profile: the people's kit tastes as Prestige (the shipped behaviour),
/// plus what its traits imply — Devout keeps something sacred, Martial and Nomadic
/// prize horses, Pastoral eats from its herds, Mercantile mints silver. Only goods
/// present in `goods` (this world's ids) are named.
pub fn auto_values(culture: &str, tastes: &[&str], traits: &[usize], goods: &[&str]) -> Vec<CultureValue> {
    let has = |g: &str| goods.contains(&g);
    let mut out: Vec<CultureValue> = Vec::new();
    let mut push = |good: &str, kind: ValueKind| {
        if has(good) && !out.iter().any(|v| v.good == good) {
            out.push(CultureValue { culture: culture.to_string(), good: good.to_string(), kind });
        }
    };
    for t in tastes { push(&favourite(culture, t, goods), ValueKind::Prestige); }
    if traits.contains(&DEVOUT) {
        if let Some(g) = ["incense", "frankincense", "candles"].into_iter().find(|g| has(g)) { push(g, ValueKind::Sacred); }
    }
    if traits.contains(&MARTIAL) || traits.contains(&NOMADIC) { push("horses", ValueKind::Prestige); }
    if traits.contains(&PASTORAL) { push("hides", ValueKind::Staple); }
    if traits.contains(&MERCANTILE) { push("silver", ValueKind::Currency); }
    out
}

/// Generic goods that stand for a whole family. A people never craves "spices" in
/// general: it craves its own favourite spice.
const GENERIC: [&str; 3] = ["spices", "gemstones", "dyes"];

/// A people's favourite member of a taste's family (owner 2026-10-17: "spices" →
/// nutmeg for one people, pepper for another). A specific good stays itself; a
/// generic one resolves to one specific member present on this world, chosen by a
/// stable hash of the people and the family, so a people keeps its favourite.
pub fn favourite(culture: &str, taste: &str, goods: &[&str]) -> String {
    if !GENERIC.contains(&taste) { return taste.to_string(); }
    let Some(fam) = crate::sim::goods_family::family_of(taste) else { return taste.to_string() };
    let mut members: Vec<&str> = goods.iter().copied()
        .filter(|g| !GENERIC.contains(g) && crate::sim::goods_family::family_of(g) == Some(fam)).collect();
    if members.is_empty() { return taste.to_string(); }
    members.sort_unstable();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in culture.bytes().chain(fam.bytes()) { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    members[(h % members.len() as u64) as usize].to_string()
}

/// The profile in force for `culture`: the author's entries if any were set for it,
/// else the derived default.
pub fn profile<'a>(culture: &str, custom: &'a [CultureValue], auto: impl FnOnce() -> Vec<CultureValue>) -> Vec<CultureValue> {
    let mine: Vec<CultureValue> = custom.iter().filter(|v| v.culture == culture).cloned().collect();
    if mine.is_empty() { auto() } else { mine }
}

#[cfg(test)]
mod culture_values_tests {
    use super::*;

    #[test]
    fn goods_culture_values_follow_traits_and_hand_entries_replace_them() {
        let goods = ["wine", "incense", "horses", "silver", "hides"];
        let auto = auto_values("Vexillia", &["wine", "pearls"], &[DEVOUT, MARTIAL, MERCANTILE], &goods);
        let kind = |g: &str| auto.iter().find(|v| v.good == g).map(|v| v.kind);
        assert_eq!(kind("wine"), Some(ValueKind::Prestige));
        assert_eq!(kind("pearls"), None, "a good this world does not have is never named");
        assert_eq!(kind("incense"), Some(ValueKind::Sacred));
        assert_eq!(kind("horses"), Some(ValueKind::Prestige));
        assert_eq!(kind("silver"), Some(ValueKind::Currency));
        let custom = vec![CultureValue { culture: "Vexillia".into(), good: "wine".into(), kind: ValueKind::Taboo }];
        let p = profile("Vexillia", &custom, || auto.clone());
        assert_eq!(p, custom, "an author's entries replace the derived profile");
        assert_eq!(profile("Other", &custom, Vec::new).len(), 0);
        // A generic taste becomes this people's own favourite member, stably.
        let world = ["spices", "pepper", "nutmeg", "cloves"];
        let fav = favourite("Vexillia", "spices", &world);
        assert!(["pepper", "nutmeg", "cloves"].contains(&fav.as_str()));
        assert_eq!(fav, favourite("Vexillia", "spices", &world));
        assert_eq!(favourite("Vexillia", "wine", &world), "wine");
        let peoples = ["Aiora", "Belgar", "Corran", "Dunmar", "Esk", "Fal"];
        let picks: std::collections::HashSet<String> = peoples.iter().map(|p| favourite(p, "spices", &world)).collect();
        assert!(picks.len() > 1, "different peoples favour different spices");
    }
}
