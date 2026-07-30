//! Inheritance — who may inherit, and how the estate divides.
//!
//! `HOUSE_MASTER_PLAN` Phase 0.4. Two small enums that live on the CULTURE and are
//! read once, at a house's succession. They are the cheapest historical lever in the
//! whole house series and the one that explains the most: whether a merchant family
//! *accumulates* across generations or is reconstituted (and often destroyed) at every
//! death is, first of all, a question of what the local law of inheritance says.
//!
//! Two axes, because that is how the historical variation actually decomposes:
//!
//! * [`LineRule`] — WHO is eligible (agnatic · agnatic-cognatic · absolute · enatic).
//! * [`InheritanceRule`] — HOW the estate divides (partible · primogeniture ·
//!   ultimogeniture · seniority · matrilineal).
//!
//! The pairing per language kit follows the record where the record is clear
//! (Roman *sui heredes* took equal shares; Celtic tanistry elected the eldest capable
//! of the *derbfine*; the Mongol *otchigin* was the hearth-keeping youngest son), and
//! is seeded where it is not.
//!
//! **Matriliny is not assigned to a named kit.** These cultures are inspired-by rather
//! than literal, so pinning matriliny to (say) the Mande kit would be a factual claim
//! about a real people that this model cannot support. Instead a seeded minority of
//! peoples — biased toward the `Clannish` trait, since kin-bound descent groups are the
//! actual precondition — comes out matrilineal, deterministically from the culture's
//! own name. Akan, Minangkabau, Nair and Haudenosaunee practice is the warrant that the
//! case is real and worth modelling; no single kit is asked to stand for it.
//!
//! Everything here is a pure function of `(name, kit, seed)`, so the campaign tick can
//! resolve a rule without touching the world.

use super::cultures::hash64;

// ── LineRule — who is eligible ───────────────────────────────────────────────
/// Who may inherit. Stored as a `u8` on the campaign side so it serialises with no
/// schema churn; `from_u8` is total (an unknown code reads as `Agnatic`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum LineRule {
    /// Males only — Salic practice; most patrician republics.
    Agnatic = 0,
    /// Males preferred, females when there is no eligible male — English common law;
    /// Qur'anic shares (a daughter takes half a son's).
    AgnaticCognatic = 1,
    /// The eldest regardless of sex — rarer; some Iberian and Italian practice.
    Absolute = 2,
    /// Females only — the matrilineal case (see the module note).
    Enatic = 3,
}

impl LineRule {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => LineRule::AgnaticCognatic,
            2 => LineRule::Absolute,
            3 => LineRule::Enatic,
            _ => LineRule::Agnatic,
        }
    }
    pub fn as_u8(self) -> u8 { self as u8 }
    pub fn label(self) -> &'static str {
        match self {
            LineRule::Agnatic => "agnatic",
            LineRule::AgnaticCognatic => "agnatic-cognatic",
            LineRule::Absolute => "absolute",
            LineRule::Enatic => "enatic",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            LineRule::Agnatic => "the line runs through sons; daughters marry out",
            LineRule::AgnaticCognatic => "sons first, but a daughter inherits where there is no son",
            LineRule::Absolute => "the eldest child inherits, son or daughter alike",
            LineRule::Enatic => "the line runs through daughters and sisters",
        }
    }
}

// ── InheritanceRule — how the estate divides ─────────────────────────────────
/// How the estate divides at a head's death.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum InheritanceRule {
    /// Equal shares among eligible children — Italian *fraterna*, Qur'anic shares,
    /// Sinitic equal division among sons. Capital splits every generation.
    Partible = 0,
    /// The eldest takes nearly all — English gentry, Yamato. Concentrates capital and
    /// leaves a surplus of able, portionless younger kin.
    Primogeniture = 1,
    /// The youngest keeps the hearth — Mongol *otchigin*, Borough English. Concentrates
    /// too, but onto a YOUNG heir: long tenures that open weak.
    Ultimogeniture = 2,
    /// The eldest *capable* of the lineage, chosen by the house — Celtic tanistry, the
    /// Rurikid *rota*, elected patrician heads. Many claimants, short tenures, and
    /// disputes as the norm rather than the exception.
    Seniority = 3,
    /// Descent through the female line; a house cannot be split by sons.
    Matrilineal = 4,
}

impl InheritanceRule {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => InheritanceRule::Primogeniture,
            2 => InheritanceRule::Ultimogeniture,
            3 => InheritanceRule::Seniority,
            4 => InheritanceRule::Matrilineal,
            _ => InheritanceRule::Partible,
        }
    }
    pub fn as_u8(self) -> u8 { self as u8 }
    pub fn label(self) -> &'static str {
        match self {
            InheritanceRule::Partible => "partible",
            InheritanceRule::Primogeniture => "primogeniture",
            InheritanceRule::Ultimogeniture => "ultimogeniture",
            InheritanceRule::Seniority => "seniority",
            InheritanceRule::Matrilineal => "matrilineal",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            InheritanceRule::Partible =>
                "the estate is divided in equal shares among the heirs",
            InheritanceRule::Primogeniture =>
                "the eldest takes the house entire; the rest are portioned off",
            InheritanceRule::Ultimogeniture =>
                "the youngest keeps the hearth and the counting-house",
            InheritanceRule::Seniority =>
                "the house elects the eldest capable of the kindred",
            InheritanceRule::Matrilineal =>
                "the house descends through its daughters and sisters",
        }
    }
    /// Does this rule DIVIDE the estate at succession? Only partible does — the payoff
    /// of every other rule is concentration, and concentration is the absence of a split.
    pub fn divides(self) -> bool { matches!(self, InheritanceRule::Partible) }
}

// ── Assignment ───────────────────────────────────────────────────────────────
/// `(LineRule, InheritanceRule)` per language kit, indexed by kit. Grounded where the
/// record is clear, seeded where it is not — see the table in
/// `docs/proposals/HOUSE_INHERITANCE_AND_TERRITORY.md` B.3.
const KIT_RULES: [(LineRule, InheritanceRule); 18] = [
    (LineRule::Agnatic, InheritanceRule::Partible),          //  0 Roman    — sui heredes, equal shares
    (LineRule::Agnatic, InheritanceRule::Partible),          //  1 Hellene
    (LineRule::Agnatic, InheritanceRule::Partible),          //  2 Punic
    (LineRule::AgnaticCognatic, InheritanceRule::Partible),  //  3 Persian
    (LineRule::Agnatic, InheritanceRule::Partible),          //  4 Norse    — óðal land divided
    (LineRule::Agnatic, InheritanceRule::Seniority),         //  5 Celtic   — tanistry
    (LineRule::AgnaticCognatic, InheritanceRule::Partible),  //  6 Arab     — Qur'anic fixed shares
    (LineRule::Agnatic, InheritanceRule::Partible),          //  7 Indic    — Mitakshara coparcenary
    (LineRule::Agnatic, InheritanceRule::Partible),          //  8 Sinitic  — equal division among sons
    (LineRule::Agnatic, InheritanceRule::Partible),          //  9 Slavic
    (LineRule::Agnatic, InheritanceRule::Primogeniture),     // 10 Nahua
    (LineRule::Agnatic, InheritanceRule::Ultimogeniture),    // 11 Turkic   — the hearth-keeping youngest
    (LineRule::AgnaticCognatic, InheritanceRule::Partible),  // 12 Nilotic
    (LineRule::AgnaticCognatic, InheritanceRule::Partible),  // 13 Amazigh
    (LineRule::Agnatic, InheritanceRule::Primogeniture),     // 14 Yamato
    (LineRule::Agnatic, InheritanceRule::Ultimogeniture),    // 15 Mongol   — otchigin
    (LineRule::Agnatic, InheritanceRule::Primogeniture),     // 16 Quechua
    (LineRule::AgnaticCognatic, InheritanceRule::Partible),  // 17 Mande
];

/// Share of generated peoples that come out MATRILINEAL, before the `Clannish` bias.
/// Held deliberately low: matriliny is a real and well-documented arrangement but a
/// minority one, and a world where a third of peoples reckoned descent through the
/// female line would be a claim about the world, not about this model.
const MATRILINEAL_BASE: f32 = 0.10;
/// A `Clannish` people — kin-bound descent groups — is likelier to reckon descent
/// through the female line, that being the actual documented precondition.
const MATRILINEAL_CLANNISH: f32 = 0.22;

/// The rules for a people. `kit` is its language kit where known (a creole takes its
/// first parent's), `clannish` whether the people carries the Clannish trait, and
/// `seed` the world seed. Pure and total: an unknown people falls back to the kit
/// distribution rolled from its own name, so a synthetic or legacy culture still gets a
/// stable, historically-shaped rule rather than a hard-coded one.
pub fn rules_for(name: &str, kit: Option<usize>, clannish: bool, seed: u64) -> (LineRule, InheritanceRule) {
    let h = name_hash(name, seed);
    // The matrilineal minority is rolled FIRST and overrides the kit pairing, so it can
    // fall on any people rather than being the property of one named culture.
    let p = if clannish { MATRILINEAL_CLANNISH } else { MATRILINEAL_BASE };
    if unit(h ^ 0x4D41_5452) < p {
        return (LineRule::Enatic, InheritanceRule::Matrilineal);
    }
    match kit {
        Some(k) if k < KIT_RULES.len() => KIT_RULES[k],
        // Unknown people: draw from the kit table itself, so the fallback carries the
        // same historical distribution the named kits do (mostly partible) instead of
        // inventing a uniform one.
        _ => KIT_RULES[(hash64(h ^ 0x4B49_5421) % KIT_RULES.len() as u64) as usize],
    }
}

/// Under `Matrilineal`, is this house's line the AVUNCULATE one — the head's heir is
/// his **sister's son**, as in Akan practice — rather than straightforward
/// eldest-daughter succession? Rolled per house, stable for its life.
///
/// Worth distinguishing because it behaves differently: a head's own sons are not his
/// heirs, which is a genuinely different source of family quarrel than any other rule
/// here produces.
pub fn is_avunculate(house_salt: u64, seed: u64) -> bool {
    unit(hash64(house_salt ^ seed ^ 0x4156_554E)) < 0.5
}

/// Is the heir under `line` a woman? Enatic lines are always female, absolute lines
/// are whoever was born first (a coin), agnatic-cognatic lines occasionally have no
/// eligible son, and agnatic lines never pass to a daughter.
pub fn heir_is_female(line: LineRule, salt: u64, seed: u64) -> bool {
    let r = unit(hash64(salt ^ seed ^ 0x5345_5821));
    match line {
        LineRule::Agnatic => false,
        // "No eligible son" is not rare in a pre-modern family — with the child
        // mortality of the period, roughly a fifth of lines had no surviving son.
        LineRule::AgnaticCognatic => r < 0.20,
        LineRule::Absolute => r < 0.50,
        LineRule::Enatic => true,
    }
}

/// How many heirs take a share under a partible division. Two to four, weighted toward
/// two — an estate split eight ways is a modelling artefact, not a family.
pub fn partible_heirs(salt: u64, seed: u64) -> u32 {
    let r = unit(hash64(salt ^ seed ^ 0x4845_4952));
    if r < 0.55 { 2 } else if r < 0.85 { 3 } else { 4 }
}

// ── helpers ──────────────────────────────────────────────────────────────────
fn name_hash(name: &str, seed: u64) -> u64 {
    // FNV-1a over the bytes, folded with the world seed. A culture keeps its rule for
    // the life of the world, and two worlds with different seeds differ.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    hash64(h ^ seed)
}

#[inline]
fn unit(x: u64) -> f32 { (hash64(x) >> 40) as f32 / (1u64 << 24) as f32 }

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kit has a rule, and the codes round-trip through the u8 the campaign
    /// stores. A silently-defaulting `from_u8` would make a save load as the wrong
    /// culture's law without any error.
    #[test]
    fn rule_codes_round_trip() {
        for (l, i) in KIT_RULES {
            assert_eq!(LineRule::from_u8(l.as_u8()), l);
            assert_eq!(InheritanceRule::from_u8(i.as_u8()), i);
        }
        for v in 0u8..=8 {
            let _ = LineRule::from_u8(v);
            let _ = InheritanceRule::from_u8(v);
        }
    }

    /// A people's rule is fixed for the life of the world — read it twice, get the
    /// same answer. (The campaign stores the resolved rule, but the resolver must be
    /// stable anyway or a reload could re-roll it.)
    #[test]
    fn rules_are_stable_per_people() {
        for n in ["Aiora", "Belgar", "Vexillia", "Culture3"] {
            let a = rules_for(n, None, false, 42);
            let b = rules_for(n, None, false, 42);
            assert_eq!(a.0, b.0);
            assert_eq!(a.1, b.1);
        }
    }

    /// The matrilineal minority is a MINORITY, and Clannish peoples carry more of it.
    /// Both halves matter: a world with none of it loses a real arrangement, and a
    /// world with a third of it is making a claim the model cannot support.
    #[test]
    fn matriliny_is_a_seeded_minority_biased_to_clannish() {
        let count = |clannish: bool| {
            (0..600)
                .filter(|i| {
                    let n = format!("People{i}");
                    rules_for(&n, Some(i % 18), clannish, 7).1 == InheritanceRule::Matrilineal
                })
                .count() as f32 / 600.0
        };
        let plain = count(false);
        let clan = count(true);
        assert!((0.05..0.18).contains(&plain), "matrilineal share off the base rate: {plain:.3}");
        assert!((0.14..0.32).contains(&clan), "clannish matrilineal share out of range: {clan:.3}");
        assert!(clan > plain + 0.04, "Clannish must raise the rate: {plain:.3} → {clan:.3}");
    }

    /// An unknown culture must not collapse onto one rule — the fallback draws from the
    /// kit table, so a world of synthetic culture names still has a mix of laws.
    #[test]
    fn unknown_cultures_draw_a_spread_of_rules() {
        let mut seen = [0usize; 5];
        for i in 0..400 {
            let n = format!("Unknown{i}");
            seen[rules_for(&n, None, false, 3).1.as_u8() as usize] += 1;
        }
        assert!(seen[InheritanceRule::Partible as usize] > 100, "partible should dominate: {seen:?}");
        assert!(seen.iter().filter(|&&c| c > 0).count() >= 4, "too few rules represented: {seen:?}");
    }

    /// The line rule decides the heir's sex, and the two extremes must be absolute:
    /// an agnatic house never passes to a daughter, an enatic one always does.
    #[test]
    fn line_rule_decides_the_heirs_sex() {
        let share = |line: LineRule| {
            (0..400).filter(|i| heir_is_female(line, *i as u64, 11)).count() as f32 / 400.0
        };
        assert_eq!(share(LineRule::Agnatic), 0.0);
        assert_eq!(share(LineRule::Enatic), 1.0);
        let ac = share(LineRule::AgnaticCognatic);
        assert!((0.12..0.30).contains(&ac), "agnatic-cognatic female share off: {ac:.3}");
        let ab = share(LineRule::Absolute);
        assert!((0.40..0.60).contains(&ab), "absolute female share should be ~half: {ab:.3}");
    }

    /// A partible division is a family, not a fan-out: two to four shares, most often two.
    #[test]
    fn partible_divisions_stay_small() {
        let mut hist = [0usize; 5];
        for i in 0..500 { hist[partible_heirs(i as u64, 5) as usize] += 1; }
        assert_eq!(hist[0] + hist[1], 0, "a division must have at least two heirs");
        assert!(hist[2] > hist[3] && hist[3] > hist[4], "should taper: {hist:?}");
    }
}
