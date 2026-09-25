//! individuals — `docs/living_world/02_PEOPLE.md`: ONE system for every named
//! human the world tracks (ruler, senator, commander, scholar, artisan,
//! gladiator, horde leader): a face, a character, a life that responds to
//! what happens around them, decisions that are characteristic but not
//! predictable, and a story the maintainer can read.
//!
//! `Individual` is the one record for public people (NOT `Person` — that name
//! is the realm genealogy struct, `mod.rs` — and NOT `Notable`, the L12
//! per-city local-role entry, which is not a persisted person: it is rebuilt
//! yearly and now points at a stable `Individual` id instead of minting one).
//!
//! **Scope cut, recorded per 00_INDEX.md rule 36**: geography tags here cover
//! only what a `TickHub` already carries at campaign granularity — `coastal`,
//! `river`, and climate band from `koppen` (tropical/desert/steppe/cold).
//! `lake`/`mountain`/`forest`/`festival`/`besieged` have no per-hub signal in
//! `tick/` today and are left out of `context_tags`, not faked; the geography
//! lint (`life_event_templates_respect_geography`) only ever checks tags this
//! function can actually raise, so no template claims a tag that can never
//! fire. Queued: wiring those tags in needs a producing signal, not a lint
//! change (Q02.4).
use super::*;

// ─────────────────────────────────────────────────────────────────────────
// 00_INDEX.md "Hash salts" — the ONE registry for every `hash01` roll rows
// 02–10 add, so dozens of new calls cannot silently collide on the same
// (a,b,c) triple. Add a new named constant here, never reuse an existing one.
// ─────────────────────────────────────────────────────────────────────────
pub(crate) mod living_world_salts {
    pub const WEEKLY_TICK: u64 = 0xD0A1;
    pub const DEBUT_FEMALE: u64 = 0x9E32;
    pub const DEBUT_AGE: u64 = 0x9E33;
    pub const DEBUT_TRAIT: u64 = 0x9E34;
    pub const DEBUT_TRAIT_STRENGTH: u64 = 0x9E35;
    pub const DEBUT_FACE: u64 = 0x9E36;
    pub const MORTALITY_ROLL: u64 = 0x9E37;
    pub const DEATH_CAUSE: u64 = 0x9E38;
    pub const FIGURE_MIGRATION_FEMALE: u64 = 0x9E39;
    pub const FIGURE_MIGRATION_AGE: u64 = 0x9E3A;
    pub const NOTABLE_SPAWN: u64 = 0x9E3B;
    pub const DEMOTE_MARGIN: u64 = 0x9E3C;
    pub const EVENT_COUNT: u64 = 0x9E3D;
    pub const EVENT_PICK: u64 = 0x9E3E;
    pub const EVENT_SPREAD: u64 = 0x9E3F;
    pub const EVENT_ARG: u64 = 0x9E40;
    pub const DECISION_ROLL: u64 = 0x9E41;
}
use living_world_salts as salts;

// ── Roles (00_INDEX "the 75% rule" table + `## Roles`) ─────────────────────
pub const ROLE_RULER: u8 = 0;
pub const ROLE_COMMANDER: u8 = 1;
pub const ROLE_ADMIRAL: u8 = 2;
pub const ROLE_DIPLOMAT: u8 = 3;
pub const ROLE_MERCHANT_PRINCE: u8 = 4;
pub const ROLE_BANKER: u8 = 5;
pub const ROLE_GUILDMASTER: u8 = 6;
pub const ROLE_PHILOSOPHER: u8 = 7;
pub const ROLE_SCHOLAR: u8 = 8;
pub const ROLE_IDEOLOGUE: u8 = 9;
pub const ROLE_PHYSICIAN: u8 = 10;
pub const ROLE_ARTISAN: u8 = 11;
pub const ROLE_PERFORMER: u8 = 12;
pub const ROLE_EXPLORER: u8 = 13;
pub const ROLE_DEMAGOGUE: u8 = 14;
pub const ROLE_OFFICIAL: u8 = 15;
pub const ROLE_HORDE_LEADER: u8 = 16;
pub const ROLE_ALDERMAN: u8 = 17;
pub const ROLE_COUNT: usize = 18;

pub fn role_name(r: u8) -> &'static str {
    match r {
        ROLE_RULER => "Ruler",
        ROLE_COMMANDER => "Commander",
        ROLE_ADMIRAL => "Admiral",
        ROLE_DIPLOMAT => "Diplomat",
        ROLE_MERCHANT_PRINCE => "Merchant Prince",
        ROLE_BANKER => "Banker",
        ROLE_GUILDMASTER => "Guildmaster",
        ROLE_PHILOSOPHER => "Philosopher",
        ROLE_SCHOLAR => "Scholar",
        ROLE_IDEOLOGUE => "Ideologue",
        ROLE_PHYSICIAN => "Physician",
        ROLE_ARTISAN => "Artisan",
        ROLE_PERFORMER => "Performer",
        ROLE_EXPLORER => "Explorer",
        ROLE_DEMAGOGUE => "Demagogue",
        ROLE_OFFICIAL => "Official",
        ROLE_HORDE_LEADER => "Horde Leader",
        ROLE_ALDERMAN => "Alderman",
        _ => "Notable",
    }
}

/// The existing five `FIGURE_KINDS` mapped onto the unified role list, per
/// 00_INDEX's unification rule.
pub fn role_for_figure_kind(kind: u8) -> u8 {
    match kind {
        0 => ROLE_ADMIRAL,
        1 => ROLE_DEMAGOGUE,
        2 => ROLE_ARTISAN, // Master Craftsman
        3 => ROLE_BANKER,
        _ => ROLE_EXPLORER,
    }
}

fn notable_role_to_individual_role(role: u8) -> u8 {
    match role {
        NOTABLE_GUILDMASTER => ROLE_GUILDMASTER,
        NOTABLE_ALDERMAN => ROLE_ALDERMAN,
        _ => ROLE_DEMAGOGUE, // NOTABLE_AGITATOR
    }
}

// ── Death causes (reuses the L4 `CAUSE_*` table's own order, extended) ─────
pub const ICAUSE_OLD_AGE: u8 = 0;
pub const ICAUSE_PLAGUE: u8 = 1;
pub const ICAUSE_FEVER: u8 = 2;
pub const ICAUSE_WAR: u8 = 3;
pub const ICAUSE_FIRE: u8 = 4;
pub const ICAUSE_FLOOD: u8 = 5;
pub const ICAUSE_FAMINE: u8 = 6;
pub const ICAUSE_SACK: u8 = 7;
pub const ICAUSE_ACCIDENT: u8 = 8;
pub const ICAUSE_DUEL: u8 = 9;
pub const ICAUSE_EXECUTION: u8 = 10;

pub fn death_cause_name(c: u8) -> &'static str {
    match c {
        ICAUSE_OLD_AGE => "old age",
        ICAUSE_PLAGUE => "plague",
        ICAUSE_FEVER => "fever",
        ICAUSE_WAR => "war",
        ICAUSE_FIRE => "fire",
        ICAUSE_FLOOD => "flood",
        ICAUSE_FAMINE => "famine",
        ICAUSE_SACK => "the sack of the city",
        ICAUSE_ACCIDENT => "an accident",
        ICAUSE_DUEL => "a duel",
        _ => "execution",
    }
}

// ── Traits (00_INDEX "about 40, in five groups") ────────────────────────────
pub const TRAIT_KIND: u8 = 0;
pub const TRAIT_CRUEL: u8 = 1;
pub const TRAIT_BRAVE: u8 = 2;
pub const TRAIT_CRAVEN: u8 = 3;
pub const TRAIT_HONEST: u8 = 4;
pub const TRAIT_DECEITFUL: u8 = 5;
pub const TRAIT_PROUD: u8 = 6;
pub const TRAIT_HUMBLE: u8 = 7;
pub const TRAIT_AMBITIOUS: u8 = 8;
pub const TRAIT_CONTENT: u8 = 9;
pub const TRAIT_CURIOUS: u8 = 10;
pub const TRAIT_CLOSED: u8 = 11;
pub const TRAIT_LOYAL: u8 = 12;
pub const TRAIT_FICKLE: u8 = 13;
pub const TRAIT_TEMPERATE: u8 = 14;
pub const TRAIT_IMPULSIVE: u8 = 15;
pub const TRAIT_GENEROUS: u8 = 16;
pub const TRAIT_GREEDY: u8 = 17;
// Education
pub const TRAIT_ORATOR: u8 = 18;
pub const TRAIT_STRATEGIST: u8 = 19;
pub const TRAIT_SCHOLARLY: u8 = 20;
pub const TRAIT_ADMINISTRATOR: u8 = 21;
pub const TRAIT_SEAFARER: u8 = 22;
pub const TRAIT_PHYSICIAN_TRAINED: u8 = 23;
// Lifestyle
pub const TRAIT_DRUNKARD: u8 = 24;
pub const TRAIT_GAMBLER: u8 = 25;
pub const TRAIT_ASCETIC: u8 = 26;
pub const TRAIT_GLUTTON: u8 = 27;
pub const TRAIT_HUNTER: u8 = 28;
pub const TRAIT_PATRON_OF_ARTS: u8 = 29;
// Health (also sets a face feature — see `feature_for_trait`)
pub const TRAIT_ONE_EYED: u8 = 30;
pub const TRAIT_LAME: u8 = 31;
pub const TRAIT_SCARRED: u8 = 32;
pub const TRAIT_SICKLY: u8 = 33;
pub const TRAIT_ROBUST: u8 = 34;
pub const TRAIT_MAIMED_HAND: u8 = 35;
// Reputation
pub const TRAIT_HERO: u8 = 36;
pub const TRAIT_COWARD: u8 = 37;
pub const TRAIT_OATH_BREAKER: u8 = 38;
pub const TRAIT_BENEFACTOR: u8 = 39;
pub const TRAIT_BOUGHT: u8 = 40;
pub const TRAIT_KIN_SLAYER: u8 = 41;
pub const TRAIT_COUNT: usize = 42;

/// Personality traits only — what a debut roll draws from (education/
/// lifestyle/health/reputation are all gained through life, never at debut).
const DEBUT_TRAIT_POOL: [u8; 18] = [
    TRAIT_KIND, TRAIT_CRUEL, TRAIT_BRAVE, TRAIT_CRAVEN, TRAIT_HONEST, TRAIT_DECEITFUL,
    TRAIT_PROUD, TRAIT_HUMBLE, TRAIT_AMBITIOUS, TRAIT_CONTENT, TRAIT_CURIOUS, TRAIT_CLOSED,
    TRAIT_LOYAL, TRAIT_FICKLE, TRAIT_TEMPERATE, TRAIT_IMPULSIVE, TRAIT_GENEROUS, TRAIT_GREEDY,
];

pub fn trait_name(t: u8) -> &'static str {
    match t {
        TRAIT_KIND => "Kind", TRAIT_CRUEL => "Cruel", TRAIT_BRAVE => "Brave",
        TRAIT_CRAVEN => "Craven", TRAIT_HONEST => "Honest", TRAIT_DECEITFUL => "Deceitful",
        TRAIT_PROUD => "Proud", TRAIT_HUMBLE => "Humble", TRAIT_AMBITIOUS => "Ambitious",
        TRAIT_CONTENT => "Content", TRAIT_CURIOUS => "Curious", TRAIT_CLOSED => "Closed",
        TRAIT_LOYAL => "Loyal", TRAIT_FICKLE => "Fickle", TRAIT_TEMPERATE => "Temperate",
        TRAIT_IMPULSIVE => "Impulsive", TRAIT_GENEROUS => "Generous", TRAIT_GREEDY => "Greedy",
        TRAIT_ORATOR => "Orator", TRAIT_STRATEGIST => "Strategist", TRAIT_SCHOLARLY => "Scholar",
        TRAIT_ADMINISTRATOR => "Administrator", TRAIT_SEAFARER => "Seafarer",
        TRAIT_PHYSICIAN_TRAINED => "Physician-trained",
        TRAIT_DRUNKARD => "Drunkard", TRAIT_GAMBLER => "Gambler", TRAIT_ASCETIC => "Ascetic",
        TRAIT_GLUTTON => "Glutton", TRAIT_HUNTER => "Hunter", TRAIT_PATRON_OF_ARTS => "Patron of the Arts",
        TRAIT_ONE_EYED => "One-eyed", TRAIT_LAME => "Lame", TRAIT_SCARRED => "Scarred",
        TRAIT_SICKLY => "Sickly", TRAIT_ROBUST => "Robust", TRAIT_MAIMED_HAND => "Maimed hand",
        TRAIT_HERO => "Hero", TRAIT_COWARD => "Coward", TRAIT_OATH_BREAKER => "Oath-breaker",
        TRAIT_BENEFACTOR => "Benefactor", TRAIT_BOUGHT => "Bought", TRAIT_KIN_SLAYER => "Kin-slayer",
        _ => "?",
    }
}

/// A health trait doubles as a face feature — see `## Faces and life story`.
pub(crate) fn feature_for_trait(t: u8) -> u32 {
    match t {
        TRAIT_ONE_EYED => FEATURE_ONE_EYED,
        TRAIT_LAME => FEATURE_LAME,
        TRAIT_SCARRED => FEATURE_SCARRED,
        TRAIT_MAIMED_HAND => FEATURE_MAIMED_HAND,
        _ => 0,
    }
}

pub const TRAIT_CAP_YOUNG: usize = 3;
pub const TRAIT_CAP_MAX: usize = 8;
/// Young = debut before this age (00_INDEX "Traits").
pub const YOUNG_DEBUT_MAX_AGE: u32 = 25;

// ── Modifiers (`## Decisions — the 75% rule`) ───────────────────────────────
pub const MOD_RUSHED: u8 = 0;
pub const MOD_DRUNK: u8 = 1;
pub const MOD_THREATENED: u8 = 2;
pub const MOD_UNDER_INFLUENCE: u8 = 3;
pub const MOD_REMEMBERED_HELP: u8 = 4;
pub const MOD_GRIEVING: u8 = 5;
pub const MOD_IN_LOVE: u8 = 6;
pub const MOD_FEVERISH: u8 = 7;
pub const MOD_FLATTERED: u8 = 8;
pub const MOD_HUMILIATED: u8 = 9;
pub const MOD_SLEEPLESS: u8 = 10;
pub const MOD_EMBOLDENED: u8 = 11;
pub const MOD_OWES_DEBT: u8 = 12;
pub const MOD_BLACKMAILED: u8 = 13;
pub const MOD_HOMESICK: u8 = 14;
pub const MOD_NEWLY_WEALTHY: u8 = 15;
pub const MOD_COUNT: usize = 16;

/// "≈ ±5%" per 00_INDEX — one magnitude for every modifier; sign is chosen by
/// the decision site (a modifier tips whichever option it names).
pub const MODIFIER_MAGNITUDE: f32 = 0.05;
pub const MODIFIER_CAP: usize = 6;

pub fn modifier_name(m: u8) -> &'static str {
    match m {
        MOD_RUSHED => "a rushed decision",
        MOD_DRUNK => "drunk",
        MOD_THREATENED => "threatened",
        MOD_UNDER_INFLUENCE => "under the influence",
        MOD_REMEMBERED_HELP => "remembered how a friend helped",
        MOD_GRIEVING => "grieving",
        MOD_IN_LOVE => "in love",
        MOD_FEVERISH => "feverish",
        MOD_FLATTERED => "flattered",
        MOD_HUMILIATED => "recently humiliated",
        MOD_SLEEPLESS => "sleepless",
        MOD_EMBOLDENED => "emboldened by victory",
        MOD_OWES_DEBT => "owes a debt",
        MOD_BLACKMAILED => "blackmailed",
        MOD_HOMESICK => "homesick",
        _ => "newly wealthy",
    }
}

// ── Relations ────────────────────────────────────────────────────────────
pub const REL_MENTOR: u8 = 0;
pub const REL_RIVAL: u8 = 1;
pub const REL_FRIEND: u8 = 2;
pub const REL_PATRON: u8 = 3;
pub const REL_SPOUSE: u8 = 4;
pub const REL_STUDENT: u8 = 5;
pub const RELATIONS_CAP: usize = 8;

// ── Face features (bitflags on `Individual.features`) ───────────────────────
pub const FEATURE_ONE_EYED: u32 = 1 << 0;
pub const FEATURE_SCARRED: u32 = 1 << 1;
pub const FEATURE_LAME: u32 = 1 << 2;
pub const FEATURE_BALD: u32 = 1 << 3;
pub const FEATURE_GREY: u32 = 1 << 4;
pub const FEATURE_TATTOOED: u32 = 1 << 5;
pub const FEATURE_MAIMED_HAND: u32 = 1 << 6;

// ── Life-cycle constants ────────────────────────────────────────────────────
pub const DEBUT_MIN_AGE: u32 = 16;
pub const NOTABLE_CAP: usize = 40;
pub const NOTABLE_FAME_THRESHOLD: f32 = 0.65;
pub const NOTABLE_DEMOTE_MARGIN: f32 = 0.10;
pub const FAME_DECAY_PER_YEAR: f32 = 0.03;
pub const ORDINARY_LIFE_LOG_CAP: usize = 12;
pub const TOMBSTONE_CAP: usize = 4000;
pub const DECISION_CERTAIN_AT: f32 = 0.75;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Modifier {
    pub kind: u8,
    /// Signed — applied directly to the option it names (a positive value
    /// favors that option, negative disfavors it); magnitude is always
    /// `MODIFIER_MAGNITUDE` per 00_INDEX, so only the sign varies here.
    pub value: f32,
    pub expires_tick: u32,
    /// Who/what the modifier names ("Threatened by House Cassii"), empty if none.
    #[serde(default)]
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IndividualLifeEntry {
    pub tick: u32,
    pub template_id: u16,
    #[serde(default)]
    pub args: Vec<u32>,
}

/// A tombstone — kept while anything still points at a forgotten ordinary
/// person's id (00_INDEX "Ids and tombstones"). Capped like every other
/// append-only log in this file; ids are never reused so a tombstone dropped
/// by the cap is simply a reference that goes unresolved, not a dangling one.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tombstone {
    pub id: u32,
    pub name: String,
    pub culture: String,
    pub debut_year: u32,
    pub death_year: u32,
    /// The LAST role they held (`role_name`).
    pub role: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Individual {
    pub id: u32,
    pub name: String,
    pub female: bool,
    /// A culture name (`hub_culture`'s own convention) — deviates from the
    /// design doc's "index into the culture table" because no stable
    /// culture-index table exists anywhere in `tick/`; every other culture
    /// field in this module (`CultureRule.culture`, `hub_culture: Vec<String>`)
    /// is already a `String`, so `Individual` matches the codebase's own
    /// convention rather than inventing new infrastructure for this row alone.
    pub culture: String,
    pub birth_tick: u32,
    pub debut_tick: u32,
    /// 0 = alive.
    pub death_tick: u32,
    pub death_cause: u8,
    /// −1 = abroad/unknown.
    pub origin_hub: i32,
    pub formed_hub: i32,
    pub current_hub: i32,
    /// −1 = none.
    pub house: i32,
    /// Index into `House.kin`, or −1.
    pub kin_ref: i32,
    pub roles: Vec<u8>,
    pub famous: bool,
    pub fame: f32,
    pub traits: Vec<(u8, i8)>,
    pub modifiers: Vec<Modifier>,
    /// Row 06 fills; zero until then (forward hook).
    pub ideology: [f32; 4],
    pub face_seed: u32,
    pub features: u32,
    pub relations: Vec<(u8, u32)>,
    pub backstory: Vec<(u16, Vec<u32>)>,
    pub life_log: Vec<IndividualLifeEntry>,
    /// The calendar year `events_this_year` was last rolled for (life_events.rs).
    #[serde(default)]
    pub events_year: u32,
    /// Remaining event quota for `events_year`, drawn once per year, spent
    /// gradually across that year's weekly passes.
    #[serde(default)]
    pub events_this_year: u8,
}

impl Individual {
    pub fn is_alive(&self) -> bool {
        self.death_tick == 0
    }
}

/// `decide()`'s result — the chosen option, the normalised probabilities
/// (for the UI), whether the 75% rule made it certain, and the dominant
/// reasons behind the CHOSEN option (00_INDEX "Every decision is logged
/// with its dominant reasons").
pub struct DecisionOutcome {
    pub choice: usize,
    pub probs: Vec<f32>,
    pub certain: bool,
    pub reasons: Vec<String>,
}

/// The 75% rule (`## Decisions — the 75% rule`). `base` is one probability
/// per option; `trait_terms`/`modifier_terms` are the SAME shape (one list of
/// (trait/modifier kind, signed weight) per option) so a caller need not
/// filter which traits apply — an absent trait/modifier simply contributes
/// nothing. `context` is one more additive term per option for a decision
/// kind's own situational nudge (never its own threshold — 00_INDEX).
///
/// Deterministic: the only randomness is `hash01(seed, tick, individual_id, kind)`,
/// so the SAME inputs always produce the SAME outcome (`decisions_are_deterministic`).
#[allow(clippy::too_many_arguments)]
pub fn decide(
    seed: u64,
    tick: u32,
    individual_id: u32,
    kind: u64,
    base: &[f32],
    trait_terms: &[Vec<(u8, f32)>],
    traits: &[(u8, i8)],
    modifier_terms: &[Vec<(u8, f32)>],
    modifiers: &[Modifier],
    context: &[f32],
) -> DecisionOutcome {
    let n = base.len().max(1);
    let mut p = vec![0.0f32; n];
    let mut reason_terms: Vec<Vec<(String, f32)>> = vec![Vec::new(); n];
    for o in 0..n {
        let mut v = base[o];
        reason_terms[o].push((format!("{:.0}% base", base[o] * 100.0), base[o]));
        if let Some(terms) = trait_terms.get(o) {
            for &(tk, w) in terms {
                if let Some(&(_, strength)) = traits.iter().find(|&&(t, _)| t == tk) {
                    let contrib = w * strength as f32;
                    v += contrib;
                    if contrib.abs() > 0.001 {
                        reason_terms[o].push((trait_name(tk).to_string(), contrib));
                    }
                }
            }
        }
        if let Some(terms) = modifier_terms.get(o) {
            for &(mk, w) in terms {
                if modifiers.iter().any(|m| m.kind == mk) {
                    v += w;
                    if w.abs() > 0.001 {
                        reason_terms[o].push((modifier_name(mk).to_string(), w));
                    }
                }
            }
        }
        if let Some(&c) = context.get(o) {
            v += c;
        }
        p[o] = v.clamp(0.0, 1.0);
    }
    let sum: f32 = p.iter().sum();
    if sum > 0.0 {
        for v in p.iter_mut() {
            *v /= sum;
        }
    } else {
        let u = 1.0 / n as f32;
        for v in p.iter_mut() {
            *v = u;
        }
    }
    let (best, &best_p) = p
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));
    let (choice, certain) = if best_p >= DECISION_CERTAIN_AT {
        (best, true)
    } else {
        let r = hash01(seed, tick as u64 ^ (individual_id as u64).wrapping_mul(0x9E3779B1), kind ^ salts::DECISION_ROLL);
        let mut acc = 0.0f32;
        let mut pick = n - 1;
        for (o, &pv) in p.iter().enumerate() {
            acc += pv;
            if r < acc {
                pick = o;
                break;
            }
        }
        (pick, false)
    };
    let mut reasons: Vec<(String, f32)> = reason_terms[choice].clone();
    reasons.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
    reasons.truncate(3);
    DecisionOutcome {
        choice,
        probs: p,
        certain,
        reasons: reasons.into_iter().map(|(s, _)| s).collect(),
    }
}

// ── Geography/context tags for events + the geography lint ─────────────────
pub const TAG_COAST: u32 = 1 << 0;
pub const TAG_RIVER: u32 = 1 << 1;
pub const TAG_DESERT: u32 = 1 << 2;
pub const TAG_STEPPE: u32 = 1 << 3;
pub const TAG_COLD: u32 = 1 << 4;
pub const TAG_TROPICAL: u32 = 1 << 5;
pub const TAG_LARGE: u32 = 1 << 6;
pub const TAG_AT_WAR: u32 = 1 << 7;
pub const TAG_FAMINE: u32 = 1 << 8;
pub const TAG_TRAVELLING: u32 = 1 << 9;
pub const TAG_ROLE_COMMANDER: u32 = 1 << 10;
pub const TAG_ROLE_ADMIRAL: u32 = 1 << 11;
pub const TAG_ROLE_SCHOLAR: u32 = 1 << 12;
pub const TAG_ROLE_ARTISAN: u32 = 1 << 13;
pub const TAG_ROLE_PERFORMER: u32 = 1 << 14;
pub const TAG_ROLE_SENATOR: u32 = 1 << 15;
pub const TAG_GENERIC: u32 = 0;

/// `LARGE` mirrors the settlement-life plan's own metropolis order of magnitude.
const LARGE_POP: f32 = 60_000.0;

impl CampaignSim {
    /// The context tags a person's CURRENT circumstances satisfy — a plain
    /// function of the hub they are in + their own roles, so the same tag
    /// vocabulary drives both event template matching and decision context.
    pub(crate) fn individual_context_tags(&self, p: &Individual) -> u32 {
        let mut t = TAG_GENERIC;
        if let Some(h) = self.hubs.get(p.current_hub.max(0) as usize) {
            if p.current_hub >= 0 {
                if h.coastal { t |= TAG_COAST; }
                if h.river { t |= TAG_RIVER; }
                if (4..=5).contains(&h.koppen) { t |= TAG_DESERT; }
                if (6..=7).contains(&h.koppen) { t |= TAG_STEPPE; }
                if h.koppen >= 14 { t |= TAG_COLD; }
                if (1..=3).contains(&h.koppen) { t |= TAG_TROPICAL; }
                if h.population >= LARGE_POP { t |= TAG_LARGE; }
                if h.war_with >= 0 { t |= TAG_AT_WAR; }
                if h.starving > 0.3 { t |= TAG_FAMINE; }
            }
        }
        if p.origin_hub != p.current_hub && p.formed_hub != p.current_hub {
            t |= TAG_TRAVELLING;
        }
        for &r in &p.roles {
            t |= match r {
                ROLE_COMMANDER | ROLE_HORDE_LEADER => TAG_ROLE_COMMANDER,
                ROLE_ADMIRAL => TAG_ROLE_ADMIRAL,
                ROLE_SCHOLAR | ROLE_PHILOSOPHER => TAG_ROLE_SCHOLAR,
                ROLE_ARTISAN => TAG_ROLE_ARTISAN,
                ROLE_PERFORMER => TAG_ROLE_PERFORMER,
                ROLE_OFFICIAL | ROLE_RULER | ROLE_ALDERMAN => TAG_ROLE_SENATOR,
                _ => TAG_GENERIC,
            };
        }
        t
    }

    // ── 02.1 migration ──────────────────────────────────────────────────
    /// One-time: every still-referenced `Figure` becomes an `Individual`, so
    /// the roster is unified without losing an existing figure's chronicle
    /// identity (name/hub/house/kind survive; nothing about `figures` itself
    /// is removed — other code still reads it — this only ADDS the mirror).
    pub(crate) fn migrate_figures_to_individuals(&mut self) {
        for i in 0..self.figures.len() {
            let f = self.figures[i].clone();
            let role = role_for_figure_kind(f.kind);
            // Already migrated? (idempotent — a second call, e.g. from a
            // fixture that never sets the flag, must not double-mint.)
            if self.people.iter().any(|p| p.name == f.name && p.roles.contains(&role) && p.current_hub == f.hub as i32)
                || self.hall_of_dead.iter().any(|p| p.name == f.name && p.roles.contains(&role))
            {
                continue;
            }
            let female = hash01(self.seed, f.hub as u64, salts::FIGURE_MIGRATION_FEMALE) < 0.5;
            let age = 25 + (hash01(self.seed, f.hub as u64 ^ i as u64, salts::FIGURE_MIGRATION_AGE) * 35.0) as u32;
            let culture = self.hub_culture.get(f.hub as usize).cloned().unwrap_or_default();
            let id = self.next_individual_id;
            self.next_individual_id += 1;
            let indiv = Individual {
                id,
                name: f.name.clone(),
                female,
                culture,
                birth_tick: f.born_tick.saturating_sub(age * TICKS_PER_YEAR),
                debut_tick: f.born_tick,
                death_tick: if f.dead { f.dies_tick } else { 0 },
                death_cause: ICAUSE_OLD_AGE,
                origin_hub: f.hub as i32,
                formed_hub: f.hub as i32,
                current_hub: f.hub as i32,
                house: f.house,
                kin_ref: -1,
                roles: vec![role],
                famous: !f.dead,
                fame: if f.dead { 0.0 } else { 0.5 },
                traits: Vec::new(),
                modifiers: Vec::new(),
                ideology: [0.0; 4],
                face_seed: hash01(self.seed, f.hub as u64 ^ (i as u64).wrapping_mul(7), salts::DEBUT_FACE).to_bits(),
                features: 0,
                relations: Vec::new(),
                backstory: Vec::new(),
                life_log: vec![IndividualLifeEntry { tick: f.born_tick, template_id: 0, args: vec![] }],
                events_year: 0,
                events_this_year: 0,
            };
            if f.dead {
                self.hall_of_dead.push(indiv);
            } else {
                self.people.push(indiv);
            }
        }
    }

    // ── L12 Notable linkage ─────────────────────────────────────────────
    /// Resolve (or mint) the stable `Individual` id a local role (guildmaster/
    /// alderman/agitator) at `h` should carry, so `update_notables`'s yearly
    /// rebuild never mints a fresh person for the same seat while the same
    /// name still holds it (`local_roles_do_not_mint_new_people_yearly`).
    pub(crate) fn individual_id_for_notable(&mut self, h: usize, role: u8, name: &str, good: i32) -> i32 {
        if let Some(existing) = self.hubs[h].notables.iter().find(|n| n.role == role && n.name == name) {
            if existing.individual_id >= 0 {
                return existing.individual_id;
            }
        }
        let iv_role = notable_role_to_individual_role(role);
        if let Some(p) = self.people.iter().find(|p| p.current_hub == h as i32 && p.name == name && p.roles.contains(&iv_role)) {
            return p.id as i32;
        }
        self.spawn_individual(h, iv_role, name.to_string(), good) as i32
    }

    /// Mint a fresh ordinary `Individual` taking up local role `role` at hub `h`.
    pub(crate) fn spawn_individual(&mut self, h: usize, role: u8, name: String, _related_good: i32) -> u32 {
        let salt = (h as u64).wrapping_mul(0x9E3779B1) ^ (self.next_individual_id as u64);
        let female = hash01(self.seed, salt, salts::DEBUT_FEMALE) < 0.5;
        let age = DEBUT_MIN_AGE + (hash01(self.seed, salt, salts::DEBUT_AGE) * 45.0) as u32;
        let culture = self.hub_culture.get(h).cloned().unwrap_or_default();
        let mut traits = Vec::new();
        let n_traits = 1 + (hash01(self.seed, salt, salts::DEBUT_TRAIT) * TRAIT_CAP_YOUNG as f32) as usize;
        for k in 0..n_traits.min(TRAIT_CAP_YOUNG) {
            let ti = (hash01(self.seed, salt ^ k as u64, salts::DEBUT_TRAIT) * DEBUT_TRAIT_POOL.len() as f32) as usize;
            let t = DEBUT_TRAIT_POOL[ti.min(DEBUT_TRAIT_POOL.len() - 1)];
            let strength = if hash01(self.seed, salt ^ k as u64, salts::DEBUT_TRAIT_STRENGTH) < 0.5 { 1i8 } else { 2i8 };
            if !traits.iter().any(|&(tt, _): &(u8, i8)| tt == t) {
                traits.push((t, strength));
            }
        }
        let id = self.next_individual_id;
        self.next_individual_id += 1;
        let indiv = Individual {
            id,
            name,
            female,
            culture,
            birth_tick: self.tick.saturating_sub(age * TICKS_PER_YEAR),
            debut_tick: self.tick,
            death_tick: 0,
            death_cause: 0,
            origin_hub: h as i32,
            formed_hub: h as i32,
            current_hub: h as i32,
            house: -1,
            kin_ref: -1,
            roles: vec![role],
            famous: false,
            fame: 0.0,
            traits,
            modifiers: Vec::new(),
            ideology: [0.0; 4],
            face_seed: hash01(self.seed, salt, salts::DEBUT_FACE).to_bits(),
            features: 0,
            relations: Vec::new(),
            backstory: Vec::new(),
            life_log: Vec::new(),
            events_year: 0,
            events_this_year: 0,
        };
        self.people.push(indiv);
        id
    }

    // ── 02.1 weekly cadence hook ─────────────────────────────────────────
    /// The new weekly hook (00_INDEX "Cadence hooks") — expires modifiers
    /// and fires due life events for every LIVING person, O(people) with
    /// small constants.
    pub(crate) fn people_weekly_pass(&mut self) {
        let tick = self.tick;
        for i in 0..self.people.len() {
            self.people[i].modifiers.retain(|m| m.expires_tick > tick);
        }
        self.fire_due_life_events();
    }

    // ── 02.3 yearly life cycle ───────────────────────────────────────────
    /// Aging, mortality, fame decay/promotion/demotion, and death handling —
    /// called once a year, after the figure/notable passes above so a
    /// notable-linked individual's fame this year is already current.
    pub(crate) fn people_yearly_pass(&mut self, yr: u32) {
        let tick = self.tick;
        let mut newly_dead: Vec<usize> = Vec::new();
        for i in 0..self.people.len() {
            // Fame decays slowly toward 0 every year (00_INDEX "`fame` rises
            // with deeds ... decays slowly").
            self.people[i].fame = (self.people[i].fame - FAME_DECAY_PER_YEAR).max(0.0);
            let age_years = tick.saturating_sub(self.people[i].birth_tick) / TICKS_PER_YEAR;
            let hazard = person_mortality_hazard(age_years);
            let roll = hash01(self.seed, tick as u64 ^ (self.people[i].id as u64).wrapping_mul(0x9E3779B1), salts::MORTALITY_ROLL);
            if roll < hazard {
                let at_war = self.hubs.get(self.people[i].current_hub.max(0) as usize)
                    .map(|h| self.people[i].current_hub >= 0 && h.war_with >= 0).unwrap_or(false);
                let in_role = self.people[i].roles.iter().any(|&r| matches!(r, ROLE_COMMANDER | ROLE_ADMIRAL | ROLE_HORDE_LEADER));
                let cause_roll = hash01(self.seed, tick as u64, (self.people[i].id as u64) ^ salts::DEATH_CAUSE);
                let cause = if at_war && in_role && cause_roll < 0.4 {
                    ICAUSE_WAR
                } else if age_years >= 65 {
                    ICAUSE_OLD_AGE
                } else if cause_roll < 0.15 {
                    ICAUSE_ACCIDENT
                } else {
                    ICAUSE_OLD_AGE
                };
                self.people[i].death_tick = tick;
                self.people[i].death_cause = cause;
                newly_dead.push(i);
                continue;
            }
            // Promotion: an ordinary person whose fame crosses the threshold
            // becomes notable IF a slot is free, or bumps the least-famous
            // living notable if the newcomer clears it by a margin.
            if !self.people[i].famous && self.people[i].fame >= NOTABLE_FAME_THRESHOLD {
                let living_notables = self.people.iter().filter(|p| p.famous && p.is_alive()).count();
                if living_notables < NOTABLE_CAP {
                    self.people[i].famous = true;
                } else {
                    let newcomer_fame = self.people[i].fame;
                    if let Some((lo_idx, lo_fame)) = self.people.iter().enumerate()
                        .filter(|(_, p)| p.famous && p.is_alive())
                        .min_by(|a, b| a.1.fame.partial_cmp(&b.1.fame).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(idx, p)| (idx, p.fame))
                    {
                        let margin_roll = hash01(self.seed, tick as u64, self.people[i].id as u64 ^ salts::DEMOTE_MARGIN);
                        if newcomer_fame > lo_fame + NOTABLE_DEMOTE_MARGIN && margin_roll < 0.5 {
                            self.people[lo_idx].famous = false;
                            self.people[i].famous = true;
                        }
                    }
                }
            }
        }
        // Remove the dead — famous go to the Hall (kept forever, modifiers
        // cleared); ordinary are forgotten but leave a tombstone.
        for &i in newly_dead.iter().rev() {
            self.remove_dead_individual(i, yr);
        }
    }

    /// Removes `people[i]` (already marked dead) — famous go to the Hall of
    /// the Dead (kept forever, modifiers cleared), ordinary are forgotten
    /// but leave a capped tombstone. Split out from `people_yearly_pass` so
    /// it is directly testable without relying on a probabilistic mortality
    /// roll (`dead_notables_keep_their_story`, `dead_ordinary_people_are_removed`).
    pub(crate) fn remove_dead_individual(&mut self, i: usize, yr: u32) {
        let mut p = self.people.remove(i);
        if p.famous {
            p.modifiers.clear();
            self.hall_of_dead.push(p);
        } else {
            let debut_year = p.debut_tick / TICKS_PER_YEAR;
            let role = p.roles.last().copied().unwrap_or(ROLE_OFFICIAL);
            self.people_tombstones.push(Tombstone {
                id: p.id, name: p.name.clone(), culture: p.culture.clone(),
                debut_year, death_year: yr, role,
            });
            if self.people_tombstones.len() > TOMBSTONE_CAP {
                let drop = self.people_tombstones.len() - TOMBSTONE_CAP;
                self.people_tombstones.drain(0..drop);
            }
        }
    }
}
