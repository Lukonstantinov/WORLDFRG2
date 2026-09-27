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
        mk(4, "Ancestral Piety", &[3, 12], &[(EDICT_FAM_MILITARY, -1), (EDICT_FAM_LEARNING, -1)]),
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
/// Fame a teaching scholar needs before they may found a school.
const SCHOOL_FOUNDING_FAME: f32 = 0.35;
/// Two positions within this distance share a school rather than minting a
/// second, near-identical custom doctrine.
const CUSTOM_IDEOLOGY_MERGE_DIST: f32 = 1.5;

/// Dosed hooks — both true no-ops at 0.0 (`ideology_gov_hook_is_a_noop_at_
/// zero`, `ideology_unrest_hook_is_a_noop_at_zero`).
pub(crate) const IDEOLOGY_GOV_HOOK_DOSE: f32 = 0.0;
pub(crate) const IDEOLOGY_UNREST_DOSE: f32 = 0.0;

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
                self.people[i].ideology = base;
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
                    let roll = hash01(self.seed, self.people[i].id as u64, salts::SCHOLAR_STUDY_TARGET);
                    // Travel to the best-learning-tier reachable neighbour
                    // (a real centre), else stay and become the local teacher.
                    let neighbours = self.neighbors.get(hu).cloned().unwrap_or_default();
                    let best = neighbours.iter()
                        .map(|&b| b as usize)
                        .filter(|&b| b < self.hubs.len())
                        .max_by_key(|&b| self.hubs[b].track_level[TRACK_IDEOLOGICAL]);
                    if roll < SCHOLAR_STUDY_TRAVEL_CHANCE {
                        if let Some(dest) = best.filter(|&b| self.hubs[b].track_level[TRACK_IDEOLOGICAL] > self.hubs[hu].track_level[TRACK_IDEOLOGICAL]) {
                            let teacher = self.people.iter()
                                .filter(|p| p.is_alive() && p.current_hub == dest as i32
                                    && p.roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE))
                                    && p.scholar_stage == STAGE_TEACH)
                                .max_by(|a, b| a.fame.partial_cmp(&b.fame).unwrap_or(std::cmp::Ordering::Equal))
                                .map(|p| p.id as i32);
                            self.people[i].current_hub = dest as i32;
                            self.people[i].teacher_id = teacher.unwrap_or(-1);
                        }
                    }
                    self.people[i].scholar_stage = STAGE_TEACH;
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
                    let outcome = decide(
                        self.seed, self.tick, self.people[i].id, salts::SCHOLAR_CAREER,
                        &base, &[Vec::new(), Vec::new(), Vec::new()], &self.people[i].traits,
                        &[Vec::new(), Vec::new(), Vec::new()], &self.people[i].modifiers, &ctx,
                    );
                    self.people[i].scholar_stage = match outcome.choice {
                        1 => { self.people[i].current_hub = home; STAGE_RETURNED }
                        2 => STAGE_PATRON,
                        _ => STAGE_TEACH,
                    };
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
            let culture = self.people[i].culture.clone();
            let neighbours = self.neighbors.get(hu).cloned().unwrap_or_default();
            let dest = neighbours.iter()
                .map(|&b| b as usize)
                .filter(|&b| b < self.hubs.len())
                .max_by(|&a, &b| {
                    let ta = self.culture_relation_tier(a, &culture);
                    let tb = self.culture_relation_tier(b, &culture);
                    ta.cmp(&tb)
                });
            if let Some(dest) = dest {
                self.people[i].current_hub = dest as i32;
                self.people[i].scholar_stage = STAGE_EXILE;
                self.people[i].teacher_id = -1;
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
        if h < 0 { return; }
        let founder_id = self.people[person_idx].id;
        let doctrine = self.doctrine_for(person_idx);
        let id = self.next_school_id;
        self.next_school_id += 1;
        self.schools.push(School {
            id, hub: h, founder: founder_id, doctrine,
            founded_tick: self.tick, students: 1,
        });
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
        self.ideologies.push(NamedIdeology {
            id, name, traits, position, founder: self.people[person_idx].id as i32,
            home_hub, founded_tick: self.tick, demands: Vec::new(), adherents: 0.0,
        });
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
                    let met = dom.demands.iter().filter(|&&(fam, tag)| {
                        self.hubs[h].gov_edicts.iter().any(|e| e.family == fam && e.tag == tag)
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

            // Which named ideology (if any) is closest to this city's commons.
            let dominant = self.ideologies.iter()
                .min_by(|a, b| ideology_dist(a.position, self.hubs[h].ideology_commons)
                    .partial_cmp(&ideology_dist(b.position, self.hubs[h].ideology_commons))
                    .unwrap_or(std::cmp::Ordering::Equal))
                .filter(|i| ideology_dist(i.position, self.hubs[h].ideology_commons) < IDEOLOGY_HOLD_MAX_DIST)
                .map(|i| i.id as i32)
                .unwrap_or(-1);
            self.hubs[h].ideology_dominant = dominant;
        }

        // Refresh each named ideology's world-wide adherents share.
        let settled: Vec<usize> = (0..n).filter(|&h| !self.hubs[h].is_estate && !self.hubs[h].abandoned).collect();
        let total = settled.len().max(1) as f32;
        for ideo in self.ideologies.iter_mut() {
            let holders = settled.iter().filter(|&&h| self.hubs[h].ideology_dominant == ideo.id as i32).count();
            ideo.adherents = holders as f32 / total;
        }

        // Demands vs edicts (a real read, effect dosed at zero — see module doc).
        if IDEOLOGY_UNREST_DOSE > 0.0 {
            for &h in &settled {
                let Some(dom) = self.ideology_by_id(self.hubs[h].ideology_dominant) else { continue };
                let unmet = dom.demands.iter().filter(|&&(fam, tag)| {
                    !self.hubs[h].gov_edicts.iter().any(|e| e.family == fam && (e.tag as i8) == tag)
                }).count();
                let frac = unmet as f32 / dom.demands.len().max(1) as f32;
                let _term = ideology_unrest_term_e(frac, IDEOLOGY_UNREST_DOSE);
                // Wiring `_term` into `update_unrest`'s target is the rest of
                // this dose walk — not reached while the dose is 0.0.
            }
        }
    }
}
