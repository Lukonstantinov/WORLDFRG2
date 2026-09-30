//! ideology — `docs/living_world/06_IDEOLOGY_AND_SCHOLARS.md`.
//!
//! Ideas as real forces: four ideology axes (Authority · Tradition ·
//! Openness · Economy, each −5..+5), a fixed vocabulary of 16 named traits
//! whose pushes on those axes compose into a **named ideology**, per-city
//! meters for nobles/commons/government, and scholars — `Individual`s
//! (row 02 already reserves `ROLE_SCHOLAR`/`ROLE_PHILOSOPHER`/
//! `ROLE_IDEOLOGUE` and an `ideology: [f32; 4]` field for exactly this row)
//! who spawn, study, teach or return home, sometimes found a school, and
//! occasionally crystallise a custom doctrine.
//!
//! **What is real here and what is still a forward hook, stated plainly
//! (00_INDEX rule 36):**
//! - Axes, trait vocabulary, seven canonical named ideologies, and per-city
//!   meters (06.1) are REAL and read by nothing outside this file yet — the
//!   same "computed, not yet a live input" status 03.1's `dev` field shipped
//!   with. `TickHub.ideology_gov` is a genuine read of who actually sits in
//!   government (row 04's real `Official.individual_id` roster), not a
//!   placeholder.
//! - Personal ideology formation + drift (06.2), scholar lives (06.3) and
//!   schools/custom ideologies (06.4) are REAL yearly mechanism — a scholar
//!   really spawns, really studies under a real teacher, really founds a
//!   school. Spawning a scholar DOES feed `tracks.rs`'s existing
//!   `TRACK_IDEOLOGICAL` notable-role bonus (that reader already existed,
//!   built for this row in advance) — safe because `track_points`/
//!   `track_level` are read downstream only by `track_building_allowed`,
//!   itself still `TRACK_CONSTRUCTION_DOSE = 0.0` (03.4's own dose), so the
//!   bonus cannot reach a wealth/production number by any live path today.
//!   Proven, not assumed: `ideology_mechanism_moves_no_wealth_or_production`
//!   mirrors 04.7's own `government_mechanism_moves_no_wealth_or_production`.
//! - Meter DRIFT (06.5) genuinely moves `ideology_nobles`/`_commons`/
//!   `_gov` and a named ideology's `adherents` — real bookkeeping, same
//!   status as the meters themselves.
//! - Two places this row COULD move a live economic number are held at an
//!   explicit dose of exactly zero, per the row's own "dose from zero"
//!   rule: `IDEOLOGY_UNREST_DOSE` (a demand left unmet would raise
//!   `update_unrest`'s target — not wired past the dose gate) and
//!   `IDEOLOGY_GOV_HOOK_DOSE` (row 04's `gov_position`, today seeded once
//!   from `gov_position_for_ideal` and never touched again, would instead
//!   track this row's own live `ideology_gov` Economy axis — a real change
//!   to `edict_cost`/debate lean that 04's own doc names as "Q04.9, needs
//!   its own dose walk"). Both are proven true no-ops at 0.0
//!   (`ideology_gov_hook_is_a_noop_at_zero`, `ideology_unrest_hook_is_a_
//!   noop_at_zero`) exactly like every other `_e`-split dose in this tree.
//!
//! **Scope cuts, recorded rather than silently dropped (rule 36):**
//! - The doc's rich scholar relationship graph (teacher/students/rival/
//!   patron, each tracked as its own edge) is narrowed to the ONE edge that
//!   actually drives a mechanic here — `teacher_id`, which "study pulls
//!   toward the teacher" and school lineage both need. Rival/patron/student
//!   lists are not persisted; queued as Q06.3.
//! - Career choice (teach / return home / seek a patron) uses `decide()`'s
//!   75% rule over a narrowed set of the doc's own weighing terms (home
//!   ties via `Curious`/`Closed`, patronage via the city's own house wealth,
//!   peers via a rival-school check) — not literally every named factor.
//! - Exile picks the best-ACCEPTANCE reachable city via row 05's own
//!   `culture_relations` where one exists, falling back to the best-tier
//!   trade neighbour otherwise — row 05 is the one existing "who welcomes
//!   whom" signal, so this reuses it rather than inventing a second one.
//! - Institutions (tutor → school → library → academy → university) are NOT
//!   a new ladder — `tracks.rs`'s own `TRACK_IDEOLOGICAL` building names
//!   (shrine/school/library/academy and embassy/university at levels 1-5)
//!   already are this ladder; a founded `School` here is the scholar-level
//!   record, `track_buildings[TRACK_IDEOLOGICAL]` is the city-level one, and
//!   the two are deliberately never conflated into a duplicate field.
//! - The world "Schools & Great Minds" window (doc's own UI section) is NOT
//!   built this session — `campaign_get_ideology_world` (commands layer)
//!   serves the same data a future window would read; queued as Q06.5.
use super::*;
use crate::sim::campaign::tick::individuals::living_world_salts as salts;

// ── Axes ────────────────────────────────────────────────────────────────
pub(crate) const AX_AUTHORITY: usize = 0;
pub(crate) const AX_TRADITION: usize = 1;
pub(crate) const AX_OPENNESS: usize = 2;
pub(crate) const AX_ECONOMY: usize = 3;
pub(crate) const IDEOLOGY_AXES: usize = 4;
pub(crate) const IDEOLOGY_CLAMP: f32 = 5.0;

pub(crate) fn clamp_ideology(mut v: [f32; 4]) -> [f32; 4] {
    for x in v.iter_mut() {
        *x = x.clamp(-IDEOLOGY_CLAMP, IDEOLOGY_CLAMP);
    }
    v
}

fn ideology_dist(a: [f32; 4], b: [f32; 4]) -> f32 {
    (0..4).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}

// ── Trait vocabulary (doc's own "Ideology traits" table) ──────────────────
pub(crate) struct IdeoTraitDef {
    pub name: &'static str,
    pub push: [f32; 4],
}
pub(crate) const IDEO_TRAIT_COUNT: usize = 16;
pub(crate) const IDEO_TRAITS: [IdeoTraitDef; IDEO_TRAIT_COUNT] = [
    IdeoTraitDef { name: "Rule of the Best Families", push: [-2.0, -1.5, 0.0, 0.0] },
    IdeoTraitDef { name: "Law Above Kings", push: [2.5, 1.0, 0.0, 0.0] },
    IdeoTraitDef { name: "The Strong Hand", push: [-3.5, 0.0, 0.0, -1.0] },
    IdeoTraitDef { name: "Ancestral Piety", push: [0.0, -3.0, -1.0, 0.0] },
    IdeoTraitDef { name: "Inquiry and Reason", push: [0.0, 3.0, 0.5, 0.0] },
    IdeoTraitDef { name: "The Stranger Is a Guest", push: [0.0, 0.0, 2.5, 0.5] },
    IdeoTraitDef { name: "Blood and Soil", push: [0.0, -1.0, -3.5, 0.0] },
    IdeoTraitDef { name: "Free Harbour", push: [0.0, 0.0, 1.0, 3.0] },
    IdeoTraitDef { name: "Just Price", push: [0.0, -0.5, 0.0, -2.5] },
    IdeoTraitDef { name: "Voice of the Many", push: [3.5, 0.5, 0.0, 0.0] },
    IdeoTraitDef { name: "Virtue of Frugality", push: [0.0, -1.0, 0.0, -1.0] },
    IdeoTraitDef { name: "Citizenship Earned", push: [0.5, 0.0, 1.5, 0.0] },
    IdeoTraitDef { name: "Honour in Arms", push: [-1.0, -1.0, -0.5, 0.0] },
    IdeoTraitDef { name: "Commonwealth of Letters", push: [1.0, 2.0, 1.5, 0.0] },
    IdeoTraitDef { name: "Harmony and Order", push: [-1.5, -1.5, 0.0, -0.5] },
    IdeoTraitDef { name: "Merit Before Birth", push: [1.0, 1.5, 0.5, 0.0] },
];

pub fn ideo_trait_name(t: u8) -> &'static str {
    IDEO_TRAITS.get(t as usize).map(|d| d.name).unwrap_or("?")
}

fn position_of_traits(traits: &[u8]) -> [f32; 4] {
    let mut pos = [0.0f32; 4];
    for &t in traits {
        if let Some(d) = IDEO_TRAITS.get(t as usize) {
            for i in 0..4 {
                pos[i] += d.push[i];
            }
        }
    }
    clamp_ideology(pos)
}

// ── Named ideologies ───────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NamedIdeology {
    pub id: u32,
    pub name: String,
    pub traits: Vec<u8>,
    pub position: [f32; 4],
    /// −1 for a canonical ideology (no single founder — it is simply "in the
    /// air", per the doc's own vocabulary table).
    pub founder: i32,
    /// −1 for a canonical ideology (holds nowhere in particular at seeding).
    pub home_hub: i32,
    pub founded_tick: u32,
    /// (edict family, wanted sign) — 2-4 demands, doc's own table.
    pub demands: Vec<(u8, i8)>,
    /// A rough world-wide "how many cities lean this way" reading, 0..1,
    /// updated yearly by `ideology_meter_drift_pass`.
    #[serde(default)]
    pub adherents: f32,
}

pub(crate) const IDEOLOGY_CAP: usize = 40;
pub(crate) const IDEOLOGY_HOLD_MAX_DIST: f32 = 3.0;
/// A city holds a named ideology once its commons lean at least this far
/// (in axis units, of ±5) along that ideology's own direction.
pub(crate) const IDEOLOGY_HOLD_MIN_LEAN: f32 = 1.0;

/// `-1` or the id of the ideology `commons` leans furthest toward (see the
/// call site). Pure — ties break on the lower id (iteration order).
pub(crate) fn dominant_ideology_for(ideologies: &[NamedIdeology], commons: [f32; 4]) -> i32 {
    let mut best = (-1i32, IDEOLOGY_HOLD_MIN_LEAN);
    for i in ideologies {
        let norm = (0..4).map(|k| i.position[k] * i.position[k]).sum::<f32>().sqrt();
        if norm < 1e-3 { continue; }
        let proj = (0..4).map(|k| commons[k] * i.position[k]).sum::<f32>() / norm;
        if proj > best.1 { best = (i.id as i32, proj); }
    }
    best.0
}

/// A person's own leaning from their CHARACTER — the row-04 doc's "seat
/// holders' positions from their traits". Axes: Authority (− strong hand ·
/// + voice of the many), Tradition (− piety · + inquiry), Openness (− blood
/// and soil · + the stranger is a guest), Economy (− just price · + free
/// harbour). Scaled by trait strength (1 or 2).
pub(crate) fn trait_ideology_push(traits: &[(u8, i8)]) -> [f32; 4] {
    let mut v = [0.0f32; 4];
    for &(t, st) in traits {
        let d: [f32; 4] = match t {
            TRAIT_KIND => [0.5, 0.0, 0.5, -0.5],
            TRAIT_CRUEL => [-1.0, 0.0, -0.5, 0.0],
            TRAIT_BRAVE => [-0.3, -0.3, 0.0, 0.0],
            TRAIT_PROUD => [-0.8, -0.3, -0.3, 0.0],
            TRAIT_HUMBLE => [0.8, 0.0, 0.3, 0.0],
            TRAIT_AMBITIOUS => [-0.5, 0.3, 0.0, 0.5],
            TRAIT_CONTENT => [0.0, -0.5, 0.0, -0.3],
            TRAIT_CURIOUS => [0.2, 1.0, 0.8, 0.2],
            TRAIT_CLOSED => [0.0, -0.8, -1.0, 0.0],
            TRAIT_LOYAL => [-0.4, -0.6, 0.0, 0.0],
            TRAIT_FICKLE => [0.3, 0.3, 0.0, 0.0],
            TRAIT_GENEROUS => [0.4, 0.0, 0.2, -0.8],
            TRAIT_GREEDY => [-0.2, 0.0, 0.0, 1.0],
            TRAIT_SCHOLARLY => [0.3, 1.2, 0.4, 0.0],
            TRAIT_ORATOR => [0.8, 0.2, 0.0, 0.0],
            TRAIT_STRATEGIST => [-0.6, 0.0, 0.0, 0.0],
            TRAIT_SEAFARER => [0.0, 0.2, 0.6, 0.6],
            TRAIT_ASCETIC => [0.0, -0.8, -0.2, -0.6],
            TRAIT_BENEFACTOR => [0.5, 0.0, 0.2, -0.5],
            _ => [0.0; 4],
        };
        for k in 0..4 { v[k] += d[k] * st as f32; }
    }
    v
}

/// The seven canonical ideologies — enough of the doc's 16-trait vocabulary
/// to give every one of the doc's own worked demand examples ("Voice of the
/// Many wants wider seats and ostracism", "Free Harbour wants free-harbour
/// and fondaco edicts", "Blood and Soil wants expulsions and a foreign-
/// ownership bar") a real, named holder. Ids 0-6, fixed forever — a custom
/// ideology (06.4) is always minted with `id >= 7`.
fn canonical_ideologies() -> Vec<NamedIdeology> {
    let mk = |id: u32, name: &str, traits: &[u8], demands: &[(u8, i8)]| NamedIdeology {
        id,
        name: name.to_string(),
        traits: traits.to_vec(),
        position: position_of_traits(traits),
        founder: -1,
        home_hub: -1,
        founded_tick: 0,
        demands: demands.to_vec(),
        adherents: 0.0,
    };
    vec![
        mk(0, "Voice of the Many", &[9, 1, 11], &[(EDICT_FAM_CONSTITUTION, 1), (EDICT_FAM_WELFARE, 1)]),
        mk(1, "Free Harbour", &[7, 5], &[(EDICT_FAM_ECONOMY, 1), (EDICT_FAM_FOREIGNERS, 1)]),
        mk(2, "Blood and Soil", &[6, 2], &[(EDICT_FAM_FOREIGNERS, -1), (EDICT_FAM_CITIZENSHIP, -1)]),
        mk(3, "Rule of the Best Families", &[0, 10], &[(EDICT_FAM_CONSTITUTION, -1), (EDICT_FAM_ECONOMY, -1)]),
        mk(4, "Ancestral Piety", &[3, 12], &[(EDICT_FAM_MILITARY, 1), (EDICT_FAM_LEARNING, -1)]),
        mk(5, "Inquiry and Reason", &[4, 13, 15], &[(EDICT_FAM_LEARNING, 1), (EDICT_FAM_CITIZENSHIP, 1)]),
        mk(6, "Just Price", &[8, 14], &[(EDICT_FAM_ECONOMY, -1), (EDICT_FAM_WELFARE, 1)]),
    ]
}

/// A school (06.4) — one scholar's institution, students, and the doctrine
/// it teaches. Capped like every other append-only roster here.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct School {
    pub id: u32,
    pub hub: i32,
    /// `Individual.id` of the founding teacher.
    pub founder: u32,
    /// `NamedIdeology.id` this school teaches.
    pub doctrine: u32,
    pub founded_tick: u32,
    pub students: u32,
}
pub(crate) const SCHOOL_CAP: usize = 200;

// ── Scholar life stages (06.3) ─────────────────────────────────────────────
pub(crate) const STAGE_NONE: u8 = 0;
pub(crate) const STAGE_STUDY: u8 = 1;
pub(crate) const STAGE_TEACH: u8 = 2;
pub(crate) const STAGE_RETURNED: u8 = 3;
pub(crate) const STAGE_PATRON: u8 = 4;
pub(crate) const STAGE_POLITICS: u8 = 5;
pub(crate) const STAGE_EXILE: u8 = 6;

/// A scholar spawns anywhere, much more often at a centre of learning
/// (the doc's own words) — `TRACK_IDEOLOGICAL`'s own level is exactly that
/// signal, already computed by row 03.
const SCHOLAR_BASE_SPAWN_CHANCE: f32 = 0.006;
const SCHOLAR_LEARNING_TIER_BONUS: f32 = 0.01;
/// A young scholar with a real trade-reachable centre of learning nearby
/// travels to study there a fair fraction of the time.
const SCHOLAR_STUDY_TRAVEL_CHANCE: f32 = 0.5;
/// Years spent at each stage before the next roll (a life takes time).
const SCHOLAR_STAGE_YEARS: u32 = 6;
/// Fame a scholar earns at each stage review while working (before the
/// centre-of-learning bonus), and per student who travels to study under them.
const SCHOLAR_TEACH_FAME: f32 = 0.12;
const SCHOLAR_STUDENT_FAME: f32 = 0.04;
/// Reseating an open government seat may pick a renowned resident thinker.
pub(crate) const SCHOLAR_SEAT_CHANCE: f32 = 0.25;
pub(crate) const SCHOLAR_SEAT_FAME: f32 = 0.30;
/// Fame a teaching scholar needs before they may found a school.
const SCHOOL_FOUNDING_FAME: f32 = 0.45;
/// A city supports at most `1 + its Ideological track level` schools, and a
/// scholar founds at most one — measured, 200 schools (the cap) on 70 cities
/// by year 80 once scholars could earn fame at all.
const SCHOOLS_PER_CITY_BASE: usize = 1;
/// Two positions within this distance share a school rather than minting a
/// second, near-identical custom doctrine.
const CUSTOM_IDEOLOGY_MERGE_DIST: f32 = 3.0;

/// Dosed hooks — both true no-ops at 0.0 (`ideology_gov_hook_is_a_noop_at_
/// zero`, `ideology_unrest_hook_is_a_noop_at_zero`).
pub(crate) const IDEOLOGY_GOV_HOOK_DOSE: f32 = 0.3;
pub(crate) const IDEOLOGY_UNREST_DOSE: f32 = 0.5;
/// Yearly pull of a city's meters back toward its culture's own position.
pub(crate) const IDEOLOGY_CULTURE_ANCHOR: f32 = 0.04;

/// Pure, testable twin of the government-position blend (N6/S1 `_e` split).
pub(crate) fn ideology_gov_position_e(old_scalar: f32, gov_economy_axis: f32, dose: f32) -> f32 {
    if dose <= 0.0 {
        return old_scalar;
    }
    let real = (gov_economy_axis / IDEOLOGY_CLAMP).clamp(-1.0, 1.0);
    old_scalar * (1.0 - dose) + real * dose
}

/// A city trading at more than this counts as booming (mirrors `tracks.rs`'s
/// own `TRADE_VOLUME_REF`, kept as an independent literal here since that
/// constant is private to its own module — not a re-tuned value).
const IDEOLOGY_TRADE_BOOM_REF: f32 = 40_000.0;

/// Pure, testable twin of the unmet-demand unrest term.
pub(crate) fn ideology_unrest_term_e(unmet_frac: f32, dose: f32) -> f32 {
    if dose <= 0.0 {
        return 0.0;
    }
    unmet_frac.clamp(0.0, 1.0) * dose * 0.1
}

impl CampaignSim {
    pub(crate) fn ensure_ideologies_seeded(&mut self) {
        if self.ideologies.is_empty() {
            self.ideologies = canonical_ideologies();
            self.next_ideology_id = self.ideologies.len() as u32;
        }
    }

    fn ideology_by_id(&self, id: i32) -> Option<&NamedIdeology> {
        if id < 0 { return None; }
        self.ideologies.iter().find(|i| i.id == id as u32)
    }

    /// 06.1 · a culture's own most-characteristic trait maps onto a rough
    /// 4-axis starting position — a direct extension of `gov_position_for_
    /// ideal`'s single-axis reading (row 04) to all four axes, so the two
    /// stay a consistent story about the same culture.
    fn ideology_pos_for_ideal(ideal: Option<usize>) -> [f32; 4] {
        match ideal {
            Some(IDEAL_WEALTH) => [0.5, 0.5, 0.5, 3.0],
            Some(IDEAL_LEARNING) => [1.0, 2.0, 1.0, 0.5],
            Some(IDEAL_REACH) => [0.5, 0.0, 2.5, 1.0],
            Some(IDEAL_ASSIMILATION) => [0.5, 0.0, 2.0, 0.0],
            Some(IDEAL_CONQUEST) => [-2.0, -0.5, -1.0, 0.0],
            Some(IDEAL_STABILITY) => [-1.0, -1.5, 0.0, -0.5],
            Some(IDEAL_LINEAGE) => [-1.5, -2.0, -1.0, 0.0],
            Some(IDEAL_PURITY) => [-1.0, -1.5, -3.0, 0.0],
            Some(IDEAL_TRADITION) => [-1.5, -3.0, -0.5, 0.0],
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// 06.1 · seeds a hub's three meters once, lazily (the `*_needs_seeding`
    /// convention). Nobles and commons start identical — they diverge only
    /// through drift (06.5) — and government starts at the same position too
    /// (it, too, is drawn from the same population at day one).
    fn seed_hub_ideology(&mut self, h: usize) {
        if self.hubs[h].ideology_seeded { return; }
        let culture = self.hub_culture.get(h).cloned().unwrap_or_default();
        let ideal = self.culture_ideal(&culture);
        let pos = Self::ideology_pos_for_ideal(ideal);
        self.hubs[h].ideology_nobles = pos;
        self.hubs[h].ideology_commons = pos;
        self.hubs[h].ideology_gov = pos;
        self.hubs[h].ideology_seeded = true;
    }

    /// 06.6 (dosed) · row 04's single-axis `gov_position` blends toward this
    /// city's real `ideology_gov` Economy axis. A true no-op while
    /// `IDEOLOGY_GOV_HOOK_DOSE` stays 0.0 — `gov_position` keeps coming
    /// entirely from `gov_position_for_ideal`'s one-time culture seed, as it
    /// always has.
    fn apply_ideology_gov_hook(&mut self, h: usize) {
        let economy = self.hubs[h].ideology_gov[AX_ECONOMY];
        self.hubs[h].gov_position = ideology_gov_position_e(self.hubs[h].gov_position, economy, IDEOLOGY_GOV_HOOK_DOSE);
    }

    // ── 06.2 · personal ideology formation + drift ─────────────────────────
    /// Yearly, over every living person: seed once from their home city's
    /// commons (or nobles, if house-linked) meter, then drift a little
    /// toward whatever is currently pulling on them. Narrowed to the terms
    /// that have a real, already-existing signal to read (see the module
    /// doc's own scope-cut list).
    pub(crate) fn people_ideology_yearly_pass(&mut self, _yr: u32) {
        // Resident scholar/teacher positions, gathered once (O(people)),
        // read many times below rather than re-scanned per student.
        let mut best_scholar_at: std::collections::HashMap<i32, (f32, [f32; 4])> = std::collections::HashMap::new();
        for p in &self.people {
            if !p.is_alive() || p.current_hub < 0 { continue; }
            if !p.roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE)) { continue; }
            let entry = best_scholar_at.entry(p.current_hub).or_insert((0.0, p.ideology));
            if p.fame >= entry.0 {
                *entry = (p.fame, p.ideology);
            }
        }

        for i in 0..self.people.len() {
            if !self.people[i].is_alive() { continue; }
            let h = self.people[i].current_hub;
            if h < 0 { continue; }
            let hu = h as usize;
            if hu >= self.hubs.len() { continue; }
            self.seed_hub_ideology(hu);

            if !self.people[i].ideology_seeded {
                let base = if self.people[i].house >= 0 {
                    self.hubs[hu].ideology_nobles
                } else {
                    self.hubs[hu].ideology_commons
                };
                // Home meter + the person's own character + a personal spread
                // (±1.5 per axis, hashed on id). Before this every person was
                // seeded to EXACTLY their city's meter, so a scholar could
                // only ever teach the city back its own position and nothing
                // diverged (no dominant doctrine, no school, no exile, all
                // measured at zero over 100 years).
                let push = trait_ideology_push(&self.people[i].traits);
                let pid = self.people[i].id as u64;
                let mut pos = base;
                for k in 0..4 {
                    let spread = (hash01(self.seed, pid, salts::IDEOLOGY_EVENT ^ (0x5EED0 + k as u64)) * 2.0 - 1.0) * 1.5;
                    pos[k] += push[k] + spread;
                }
                self.people[i].ideology = clamp_ideology(pos);
                self.people[i].ideology_seeded = true;
                continue;
            }

            let id = self.people[i].id;
            let mut pos = self.people[i].ideology;

            // Study pulls 30-60% toward the teacher (doc's own range).
            if self.people[i].teacher_id >= 0 {
                if let Some(teacher) = self.people.iter().find(|p| p.id as i32 == self.people[i].teacher_id) {
                    let frac = 0.3 + hash01(self.seed, id as u64, salts::IDEOLOGY_EVENT) * 0.3;
                    for k in 0..4 {
                        pos[k] += (teacher.ideology[k] - pos[k]) * frac * 0.15;
                    }
                }
            } else if let Some(&(_, sp)) = best_scholar_at.get(&h) {
                // Resident scholars pull the wider population toward their
                // doctrine, weighted by their own prestige (fame) — the
                // doc's own meter-drift term, applied at the individual
                // level too so a person's ordinary drift and the city
                // meter's drift (06.5) tell the same story.
                for k in 0..4 {
                    pos[k] += (sp[k] - pos[k]) * 0.02;
                }
            }

            // A sack pulls Authority toward the strong hand.
            if self.hubs[hu].damage > 0.3 {
                pos[AX_AUTHORITY] -= 0.1;
            }
            // Exile pushes away from the exiling regime.
            if self.people[i].scholar_stage == STAGE_EXILE {
                for k in 0..4 {
                    let gov = self.hubs[hu].ideology_gov[k];
                    pos[k] += (pos[k] - gov) * 0.05;
                }
            }

            self.people[i].ideology = clamp_ideology(pos);
        }
    }

    // ── 06.3 · scholar lives ────────────────────────────────────────────
    pub(crate) fn maybe_spawn_scholars(&mut self, yr: u32) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let tier = self.hubs[h].track_level[TRACK_IDEOLOGICAL] as f32;
            let chance = SCHOLAR_BASE_SPAWN_CHANCE + tier * SCHOLAR_LEARNING_TIER_BONUS;
            let roll = hash01(self.seed, (yr as u64) ^ (h as u64).wrapping_mul(0x9E3779B1), salts::SCHOLAR_SPAWN);
            if roll >= chance { continue; }
            let salt = (yr as u64).wrapping_mul(0xB1A7) ^ (h as u64);
            let hub_name = self.hubs[h].name.clone();
            let name = self.head_name_for(h, &hub_name, salt ^ salts::SCHOLAR_NAME);
            let id = self.spawn_individual(h, ROLE_SCHOLAR, name, -1);
            if let Some(p) = self.people.iter_mut().find(|p| p.id == id) {
                p.scholar_stage = STAGE_STUDY;
            }
        }
    }

    /// Yearly: a scholar in `STAGE_STUDY` may travel to a nearby centre of
    /// learning and take a teacher there; one in `STAGE_TEACH`/`_RETURNED`/
    /// `_PATRON` occasionally moves on (career choice, `decide()`'s 75%
    /// rule); a persecuted or out-of-step one goes into exile.
    pub(crate) fn update_scholar_lives(&mut self, yr: u32) {
        let n = self.people.len();
        for i in 0..n {
            if !self.people[i].is_alive() { continue; }
            if !self.people[i].roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE)) { continue; }
            let age_years = self.tick.saturating_sub(self.people[i].birth_tick) / TICKS_PER_YEAR;
            // A stage change is checked roughly every SCHOLAR_STAGE_YEARS,
            // staggered per-person by id so the whole roster doesn't
            // transition in lockstep.
            if (yr + self.people[i].id) % SCHOLAR_STAGE_YEARS != 0 { continue; }
            let h = self.people[i].current_hub;
            if h < 0 || h as usize >= self.hubs.len() { continue; }
            let hu = h as usize;

            match self.people[i].scholar_stage {
                STAGE_STUDY => {
                    if age_years < 16 { continue; }
                    // Travel to the best-learning-tier reachable neighbour
                    // (a real centre), else stay and become the local teacher.
                    // 2026-09-30b · going is a DECISION against the scholar's
                    // own character (curious and ambitious go, content,
                    // closed and loyal stay home), recorded with its odds.
                    let neighbours = self.neighbors.get(hu).cloned().unwrap_or_default();
                    let best = neighbours.iter()
                        .map(|&b| b as usize)
                        .filter(|&b| b < self.hubs.len() && !self.hubs[b].is_estate && !self.hubs[b].abandoned)
                        .max_by_key(|&b| self.hubs[b].track_level[TRACK_IDEOLOGICAL]);
                    if let Some(dest) = best.filter(|&b| self.hubs[b].track_level[TRACK_IDEOLOGICAL] > self.hubs[hu].track_level[TRACK_IDEOLOGICAL]) {
                        let trait_terms = [
                            vec![(TRAIT_CURIOUS, 0.10), (TRAIT_AMBITIOUS, 0.08), (TRAIT_SCHOLARLY, 0.05)],
                            vec![(TRAIT_CONTENT, 0.08), (TRAIT_CLOSED, 0.10), (TRAIT_LOYAL, 0.05)],
                        ];
                        let mod_terms = [Vec::new(), vec![(MOD_HOMESICK, 0.10), (MOD_GRIEVING, 0.05)]];
                        let outcome = decide(
                            self.seed, self.tick, self.people[i].id, salts::SCHOLAR_STUDY_TARGET,
                            &[SCHOLAR_STUDY_TRAVEL_CHANCE, 1.0 - SCHOLAR_STUDY_TRAVEL_CHANCE],
                            &trait_terms, &self.people[i].traits, &mod_terms, &self.people[i].modifiers, &[0.0, 0.0],
                        );
                        let pick = outcome.choice.min(1);
                        let why = decision_reasons(pick, &trait_terms, &self.people[i].traits, &mod_terms,
                            &self.people[i].modifiers, &[(CTX_GREATER_CENTRE, pick == 0)]);
                        let odds = odds_milli(&outcome.probs);
                        let pid = self.people[i].id;
                        if pick == 0 {
                            let teacher = self.people.iter()
                                .filter(|p| p.is_alive() && p.current_hub == dest as i32
                                    && p.roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE))
                                    && p.scholar_stage == STAGE_TEACH)
                                .max_by(|a, b| a.fame.partial_cmp(&b.fame).unwrap_or(std::cmp::Ordering::Equal))
                                .map(|p| p.id as i32);
                            self.relocate_person(i, dest as i32);
                            self.people[i].teacher_id = teacher.unwrap_or(-1);
                            self.log_decision_milestone(pid, MS_STUDY,
                                vec![dest as u32, teacher.map(|t| t as u32).unwrap_or(u32::MAX), hu as u32],
                                odds, 0, DK_STUDY, why);
                            // A student enrolls in the teacher's own school, if they founded one.
                            if let Some(t) = teacher {
                                if let Some(sc) = self.schools.iter_mut().find(|sc| sc.founder as i32 == t) {
                                    sc.students += 1;
                                }
                                if let Some(tp) = self.people.iter_mut().find(|p| p.id as i32 == t) {
                                    tp.fame = (tp.fame + SCHOLAR_STUDENT_FAME).min(1.0);
                                }
                            }
                        } else {
                            // Stayed: the decision rides on the teaching milestone below.
                            self.people[i].scholar_stage = STAGE_TEACH;
                            self.people[i].fame = (self.people[i].fame + SCHOLAR_TEACH_FAME).min(1.0);
                            self.log_decision_milestone(pid, MS_TEACH, vec![hu as u32, dest as u32], odds, 1, DK_STUDY, why);
                            continue;
                        }
                    }
                    self.people[i].scholar_stage = STAGE_TEACH;
                    self.people[i].fame = (self.people[i].fame + SCHOLAR_TEACH_FAME).min(1.0);
                    let (pid, here) = (self.people[i].id, self.people[i].current_hub.max(0) as u32);
                    self.log_milestone(pid, MS_TEACH, vec![here]);
                }
                STAGE_TEACH | STAGE_RETURNED | STAGE_PATRON => {
                    // Career: stay & teach / return home / seek a patron —
                    // decide() over three options, weighted by the doc's
                    // own terms this row can actually read.
                    let has_house_wealth = self.houses.iter().any(|hh| !hh.defunct && hh.hub as i32 == h && hh.wealth > 50_000.0);
                    let home = self.people[i].origin_hub;
                    let is_home = home == h;
                    let rival_present = self.schools.iter().any(|s| s.hub == h);
                    let base = [0.5, 0.25, 0.25];
                    let ctx = [
                        if rival_present { -0.1 } else { 0.0 },
                        if !is_home { 0.15 } else { -0.15 },
                        if has_house_wealth { 0.15 } else { -0.1 },
                    ];
                    // 2026-09-30b · the scholar's own character weighs in too.
                    let trait_terms = [
                        vec![(TRAIT_CONTENT, 0.08), (TRAIT_SCHOLARLY, 0.05), (TRAIT_HUMBLE, 0.04)],
                        vec![(TRAIT_LOYAL, 0.08)],
                        vec![(TRAIT_AMBITIOUS, 0.10), (TRAIT_GREEDY, 0.05), (TRAIT_PROUD, 0.04)],
                    ];
                    let mod_terms = [Vec::new(), vec![(MOD_HOMESICK, 0.15), (MOD_GRIEVING, 0.05)], Vec::new()];
                    let outcome = decide(
                        self.seed, self.tick, self.people[i].id, salts::SCHOLAR_CAREER,
                        &base, &trait_terms, &self.people[i].traits,
                        &mod_terms, &self.people[i].modifiers, &ctx,
                    );
                    let pick = outcome.choice.min(2);
                    let mut context: Vec<(u16, bool)> = Vec::new();
                    if rival_present { context.push((CTX_RIVAL_SCHOOL, pick != 0)); }
                    if !is_home { context.push((CTX_FAR_FROM_HOME, pick == 1)); } else { context.push((CTX_AT_HOME, pick != 1)); }
                    if has_house_wealth { context.push((CTX_PATRON_NEARBY, pick == 2)); } else { context.push((CTX_NO_PATRON, pick != 2)); }
                    let why = decision_reasons(pick, &trait_terms, &self.people[i].traits, &mod_terms, &self.people[i].modifiers, &context);
                    let odds = odds_milli(&outcome.probs);
                    let prev_stage = self.people[i].scholar_stage;
                    self.people[i].scholar_stage = match pick {
                        1 => { self.relocate_person(i, home); STAGE_RETURNED }
                        2 => STAGE_PATRON,
                        _ => STAGE_TEACH,
                    };
                    let (pid, stage, here) = (self.people[i].id, self.people[i].scholar_stage, self.people[i].current_hub);
                    let here_arg = if here >= 0 { here as u32 } else { u32::MAX };
                    if stage != prev_stage {
                        match stage {
                            STAGE_RETURNED => self.log_decision_milestone(pid, MS_RETURN, vec![here_arg, hu as u32], odds, 1, DK_CAREER, why),
                            STAGE_PATRON => self.log_decision_milestone(pid, MS_PATRON, vec![here_arg], odds, 2, DK_CAREER, why),
                            _ => self.log_decision_milestone(pid, MS_CAREER, vec![here_arg, 1], odds, 0, DK_CAREER, why),
                        }
                    } else {
                        // The same road again — still a decision, but not a key moment.
                        self.log_decision_milestone(pid, MS_CAREER, vec![here_arg, 0], odds, pick as u8, DK_CAREER, why);
                    }
                    // Every stage review a working scholar's reputation grows —
                    // more in a real centre of learning. Offsets the yearly
                    // fame decay (0.03 × `SCHOLAR_STAGE_YEARS`) for an active
                    // teacher, so a long career can reach a school and a name.
                    let centre = self.hubs.get(here.max(0) as usize).map(|hb| hb.track_level[TRACK_IDEOLOGICAL] as f32).unwrap_or(0.0);
                    self.people[i].fame = (self.people[i].fame + SCHOLAR_TEACH_FAME * (1.0 + centre * 0.25)).min(1.0);
                    // Seated in a government? The scholar has entered politics.
                    let seated = self.hubs.get(here.max(0) as usize)
                        .map(|hb| here >= 0 && hb.officials.iter().any(|o| o.individual_id == pid as i32)).unwrap_or(false);
                    if seated {
                        self.people[i].scholar_stage = STAGE_POLITICS;
                        self.log_milestone(pid, MS_POLITICS, vec![here_arg]);
                    }
                    // A teaching scholar with enough fame may found a school.
                    if self.people[i].scholar_stage == STAGE_TEACH && self.people[i].fame >= SCHOOL_FOUNDING_FAME {
                        self.maybe_found_school(i);
                    }
                }
                STAGE_EXILE => {
                    // Settle into the new city as an ordinary teacher.
                    self.people[i].scholar_stage = STAGE_TEACH;
                }
                _ => {}
            }
        }
        self.maybe_exile_scholars();
    }

    /// A scholar whose own ideology sits far from their city's government
    /// (persecuted / out of step) flees to the most welcoming reachable
    /// city — row 05's own acceptance tiers are the one "who welcomes whom"
    /// signal that already exists, reused rather than duplicated.
    fn maybe_exile_scholars(&mut self) {
        let n = self.people.len();
        for i in 0..n {
            if !self.people[i].is_alive() { continue; }
            if self.people[i].scholar_stage == STAGE_NONE || self.people[i].scholar_stage == STAGE_EXILE { continue; }
            let h = self.people[i].current_hub;
            if h < 0 || h as usize >= self.hubs.len() { continue; }
            let hu = h as usize;
            let gap = ideology_dist(self.people[i].ideology, self.hubs[hu].ideology_gov);
            if gap < 4.0 { continue; }
            // At most one exile a decade — a refugee's new city is judged
            // only after they have lived there a while.
            let tick = self.tick;
            if self.people[i].life_log.iter().any(|e| e.template_id == MS_EXILE && tick.saturating_sub(e.tick) < 10 * TICKS_PER_YEAR) {
                continue;
            }
            let culture = self.people[i].culture.clone();
            let neighbours = self.neighbors.get(hu).cloned().unwrap_or_default();
            let dest = neighbours.iter()
                .map(|&b| b as usize)
                .filter(|&b| b < self.hubs.len() && !self.hubs[b].is_estate && !self.hubs[b].abandoned)
                .max_by(|&a, &b| {
                    let ta = self.culture_relation_tier(a, &culture);
                    let tb = self.culture_relation_tier(b, &culture);
                    ta.cmp(&tb)
                });
            if let Some(dest) = dest {
                self.relocate_person(i, dest as i32);
                self.people[i].scholar_stage = STAGE_EXILE;
                self.people[i].teacher_id = -1;
                // An exiled thinker is a cause célèbre.
                self.people[i].fame = (self.people[i].fame + 0.08).min(1.0);
                let pid = self.people[i].id;
                self.log_milestone(pid, MS_EXILE, vec![dest as u32, hu as u32]);
            }
        }
    }

    /// A tiny read helper — row 05's own sparse `culture_relations`, or a
    /// neutral middling tier (2) if this culture holds no entry at `h`
    /// (an absence is not a rejection).
    fn culture_relation_tier(&self, h: usize, culture: &str) -> u8 {
        self.hubs.get(h)
            .and_then(|hub| hub.culture_relations.iter().find(|r| r.culture == culture))
            .map(|r| r.tier)
            .unwrap_or(2)
    }

    // ── 06.4 · schools + custom ideologies ─────────────────────────────────
    pub(crate) fn maybe_found_school(&mut self, person_idx: usize) {
        if self.schools.len() >= SCHOOL_CAP { return; }
        let h = self.people[person_idx].current_hub;
        if h < 0 || h as usize >= self.hubs.len() { return; }
        let hb = &self.hubs[h as usize];
        if hb.is_estate || hb.abandoned { return; }
        let founder_id = self.people[person_idx].id;
        if self.schools.iter().any(|sc| sc.founder == founder_id) { return; }
        let here = self.schools.iter().filter(|sc| sc.hub == h).count();
        if here >= SCHOOLS_PER_CITY_BASE + hb.track_level[TRACK_IDEOLOGICAL] as usize { return; }
        let doctrine = self.doctrine_for(person_idx);
        let id = self.next_school_id;
        self.next_school_id += 1;
        self.schools.push(School {
            id, hub: h, founder: founder_id, doctrine,
            founded_tick: self.tick, students: 1,
        });
        // Founding a school makes a PHILOSOPHER — a public figure, not only
        // a teacher (row 02 reserved the role; nothing ever assigned it).
        let p = &mut self.people[person_idx];
        if !p.roles.contains(&ROLE_PHILOSOPHER) { p.roles.push(ROLE_PHILOSOPHER); }
        p.fame = (p.fame + 0.15).min(1.0);
        self.log_milestone(founder_id, MS_SCHOOL, vec![h as u32, doctrine]);
    }

    /// The nearest existing named ideology to a scholar's own position, or —
    /// if nothing is close — a freshly minted custom one from their 3-5
    /// nearest traits (the doc's own "school crystallises a custom
    /// ideology" rule), named from their home city.
    fn doctrine_for(&mut self, person_idx: usize) -> u32 {
        self.ensure_ideologies_seeded();
        let pos = self.people[person_idx].ideology;
        if let Some(near) = self.ideologies.iter()
            .min_by(|a, b| ideology_dist(a.position, pos).partial_cmp(&ideology_dist(b.position, pos)).unwrap_or(std::cmp::Ordering::Equal))
        {
            if ideology_dist(near.position, pos) < CUSTOM_IDEOLOGY_MERGE_DIST {
                return near.id;
            }
        }
        // Mint a custom ideology: the 3-5 traits whose push is nearest this
        // position by simple dot-product affinity.
        let mut scored: Vec<(u8, f32)> = (0..IDEO_TRAIT_COUNT as u8)
            .map(|t| {
                let d = &IDEO_TRAITS[t as usize];
                let score: f32 = (0..4).map(|k| d.push[k] * pos[k]).sum();
                (t, score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let n_traits = 3 + (hash01(self.seed, self.people[person_idx].id as u64, salts::SCHOOL_CUSTOM_NAME) * 3.0) as usize;
        let traits: Vec<u8> = scored.into_iter().take(n_traits.clamp(3, 5)).map(|(t, _)| t).collect();
        let position = position_of_traits(&traits);
        let home_hub = self.people[person_idx].current_hub;
        let city_name = self.hubs.get(home_hub.max(0) as usize).map(|hh| hh.name.clone()).unwrap_or_default();
        let name = format!("the {} {}", city_name, if traits.iter().any(|&t| t == 4) { "Inquiry" } else { "Doctrine" });
        let id = self.next_ideology_id;
        self.next_ideology_id += 1;
        // A custom doctrine carries real DEMANDS (the canonical ones do; a
        // demand-less doctrine could hold a city and ask nothing of it): the
        // two families its position argues hardest for, in the direction it
        // leans (`ideology_support`).
        let mut scored_dem: Vec<((u8, i8), f32)> = Vec::new();
        for fam in 0..EDICT_FAMILY_COUNT as u8 {
            for dir in [1i8, -1] {
                scored_dem.push(((fam, dir), super::government::ideology_support(position, fam, dir)));
            }
        }
        scored_dem.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut demands: Vec<(u8, i8)> = Vec::new();
        for ((fam, dir), sc) in scored_dem {
            if sc < 0.2 || demands.len() >= 2 { break; }
            if demands.iter().any(|&(f, _)| f == fam) { continue; }
            demands.push((fam, dir));
        }
        let founder = self.people[person_idx].id;
        self.ideologies.push(NamedIdeology {
            id, name, traits, position, founder: founder as i32,
            home_hub, founded_tick: self.tick, demands, adherents: 0.0,
        });
        // Setting down a new doctrine makes an IDEOLOGUE.
        let p = &mut self.people[person_idx];
        if !p.roles.contains(&ROLE_IDEOLOGUE) { p.roles.push(ROLE_IDEOLOGUE); }
        p.fame = (p.fame + 0.2).min(1.0);
        self.log_milestone(founder, MS_DOCTRINE, vec![home_hub.max(0) as u32, id]);
        if self.ideologies.len() > IDEOLOGY_CAP {
            // Drop the least-adherent CUSTOM ideology (never a canonical
            // one — canonicals carry `founder == -1` and are permanent).
            if let Some((idx, _)) = self.ideologies.iter().enumerate()
                .filter(|(_, i)| i.founder >= 0)
                .min_by(|a, b| a.1.adherents.partial_cmp(&b.1.adherents).unwrap_or(std::cmp::Ordering::Equal))
            {
                self.ideologies.remove(idx);
            }
        }
        id
    }

    // ── 06.5 · meter drift, demands, spread ────────────────────────────────
    /// Yearly: nobles/commons/government meters drift toward whatever is
    /// pulling on them this year (resident scholars, trade partners'
    /// commons, real events), a passed edict matching an ideology's demand
    /// raises its hold, and each named ideology's `adherents` is refreshed.
    pub(crate) fn ideology_meter_drift_pass(&mut self, _yr: u32) {
        self.ensure_ideologies_seeded();
        let n = self.hubs.len();
        // Snapshot commons positions once so "trade partners' commons pull a
        // little" reads last year's values everywhere, not a half-updated mix.
        let commons_snapshot: Vec<[f32; 4]> = self.hubs.iter().map(|h| h.ideology_commons).collect();

        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            self.seed_hub_ideology(h);

            // Resident scholars/schools pull the commons toward their doctrine.
            let resident_pull = self.people.iter()
                .filter(|p| p.is_alive() && p.current_hub == h as i32
                    && p.roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE)))
                .max_by(|a, b| a.fame.partial_cmp(&b.fame).unwrap_or(std::cmp::Ordering::Equal))
                .map(|p| p.ideology);

            // Trade partners' commons pull a little (spread).
            let neighbours = self.neighbors.get(h).cloned().unwrap_or_default();
            let mut partner_pull = [0.0f32; 4];
            let mut partner_n = 0.0f32;
            for &b in &neighbours {
                if let Some(pos) = commons_snapshot.get(b as usize) {
                    for k in 0..4 { partner_pull[k] += pos[k]; }
                    partner_n += 1.0;
                }
            }
            if partner_n > 0.0 {
                for k in 0..4 { partner_pull[k] /= partner_n; }
            }

            let mut commons = self.hubs[h].ideology_commons;
            let mut nobles = self.hubs[h].ideology_nobles;
            // 2026-09-30 · tradition reasserts itself: the commons and nobles
            // drift back toward their CULTURE's own position every year. Without
            // this nothing opposed a scholar-born doctrine and one spread to
            // 61 of 72 cities in `econ_measure_living_world_census`.
            let culture = self.hub_culture.get(h).cloned().unwrap_or_default();
            let anchor = Self::ideology_pos_for_ideal(self.culture_ideal(&culture));
            for k in 0..4 {
                commons[k] += (anchor[k] - commons[k]) * IDEOLOGY_CULTURE_ANCHOR;
                nobles[k] += (anchor[k] - nobles[k]) * IDEOLOGY_CULTURE_ANCHOR;
            }
            for k in 0..4 {
                if let Some(rp) = resident_pull {
                    commons[k] += (rp[k] - commons[k]) * 0.03;
                }
                if partner_n > 0.0 {
                    commons[k] += (partner_pull[k] - commons[k]) * 0.01;
                    nobles[k] += (partner_pull[k] - nobles[k]) * 0.01;
                }
                // A trade boom pulls Economy toward free trade.
                if k == AX_ECONOMY && self.hubs[h].trade_last_year > IDEOLOGY_TRADE_BOOM_REF {
                    commons[k] += 0.05;
                    nobles[k] += 0.03;
                }
                // Famine blamed on outsiders pulls Openness down.
                if k == AX_OPENNESS && self.hubs[h].starving > 0.3 {
                    commons[k] -= 0.05;
                }
            }

            // A demand MET by a live edict entrenches the dominant ideology
            // (06.5's own "met -> prevails and spreads"); an unmet one pulls
            // no harder than the ordinary drift above. Read against LAST
            // year's dominant (this year's is computed below), which is
            // exactly the "what this city already leans toward" question.
            if let Some(dom) = self.ideology_by_id(self.hubs[h].ideology_dominant).cloned() {
                if !dom.demands.is_empty() {
                    let met = dom.demands.iter().filter(|&&(fam, sign)| {
                        self.hubs[h].gov_edicts.iter().any(|e| e.family == fam && e.dir == sign)
                    }).count();
                    let frac_met = met as f32 / dom.demands.len() as f32;
                    let pull = frac_met * 0.08;
                    if pull > 0.0 {
                        for k in 0..4 {
                            commons[k] += (dom.position[k] - commons[k]) * pull;
                        }
                    }
                }
            }
            self.hubs[h].ideology_commons = clamp_ideology(commons);
            self.hubs[h].ideology_nobles = clamp_ideology(nobles);

            // Government meter — the prestige-weighted mean of seated
            // officials who carry a real `Individual` (row 04's roster).
            let mut gov_sum = [0.0f32; 4];
            let mut gov_w = 0.0f32;
            for off in &self.hubs[h].officials {
                if off.individual_id < 0 { continue; }
                if let Some(p) = self.people.iter().find(|p| p.id as i32 == off.individual_id) {
                    let w = off.suitability.max(0.1);
                    for k in 0..4 { gov_sum[k] += p.ideology[k] * w; }
                    gov_w += w;
                }
            }
            if gov_w > 0.0 {
                for k in 0..4 { gov_sum[k] /= gov_w; }
                self.hubs[h].ideology_gov = clamp_ideology(gov_sum);
            }

            self.apply_ideology_gov_hook(h);

            // Which named ideology this city's commons LEAN toward — the
            // doctrine with the largest projection of the commons position
            // onto its own direction, if that lean is at least
            // `IDEOLOGY_HOLD_MIN_LEAN`. The old nearest-within-3.0 test could
            // never fire: a canonical doctrine sits 3.5-5.4 from the origin
            // (its traits' pushes add up), while a city's meter starts at its
            // culture's ideal (magnitude ~1-3), so measured over 100 years NO
            // city ever held ANY ideology (`econ_measure_living_world_census`).
            let dominant = dominant_ideology_for(&self.ideologies, self.hubs[h].ideology_commons);
            self.hubs[h].ideology_dominant = dominant;
        }

        // Refresh each named ideology's world-wide adherents share.
        let settled: Vec<usize> = (0..n).filter(|&h| !self.hubs[h].is_estate && !self.hubs[h].abandoned).collect();
        let total = settled.len().max(1) as f32;
        for ideo in self.ideologies.iter_mut() {
            let holders = settled.iter().filter(|&&h| self.hubs[h].ideology_dominant == ideo.id as i32).count();
            ideo.adherents = holders as f32 / total;
        }

        // Demands vs edicts — the unmet share is stored per city and read by
        // `update_unrest` (`ideology_unrest_term_e`, `IDEOLOGY_UNREST_DOSE`),
        // and the commons-vs-government gap is stored for `maybe_revolution`.
        for &h in &settled {
            let unmet = match self.ideology_by_id(self.hubs[h].ideology_dominant) {
                Some(dom) if !dom.demands.is_empty() => {
                    let n = dom.demands.iter().filter(|&&(fam, sign)| {
                        !self.hubs[h].gov_edicts.iter().any(|e| e.family == fam && e.dir == sign)
                    }).count();
                    n as f32 / dom.demands.len() as f32
                }
                _ => 0.0,
            };
            self.hubs[h].ideology_unmet = unmet;
            self.hubs[h].ideology_gap = (ideology_dist(self.hubs[h].ideology_commons, self.hubs[h].ideology_gov)
                / (2.0 * IDEOLOGY_CLAMP)).clamp(0.0, 1.0);
        }
    }
}
