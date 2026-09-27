//! government — `docs/living_world/04_GOVERNMENT_AND_EDICTS.md`, slices
//! 04.3-04.6.
//!
//! Builds the weekly point-accrual → propose → debate → resolve → expire
//! loop, the tyrant's no-vote path, and the five-yearly Lustrum. Everything
//! here is deliberately **descriptive-only this session**: an edict's
//! `family` records WHY it was proposed but changes no other economic field
//! once it passes (no tariff actually moves, no wall actually rises) — that
//! wiring is `Q04.9`, queued rather than built, exactly as this row's own
//! `04.8` names ("edict effects dosed from zero"). Because nothing here
//! reads `legitimacy`/`gov_position`/a passed edict's `family` to change a
//! wealth, population, price or production number, none of it needs a dose
//! gate of its own — the same "unconditional bookkeeping" precedent L4's age
//! pyramid and 04.2's paths/suitability already set. The one place this row
//! COULD move a live number — `stability_at` reading a real `legitimacy`
//! instead of `development.rs`'s `NEUTRAL_LEGITIMACY` constant — is
//! deliberately NOT wired this session (Q04.9): row 03's track-point math is
//! live (not itself dosed), so swapping its legitimacy input for a moving
//! target is a real behaviour change that needs its own dose walk, not a
//! side effect of this row landing.
//!
//! The one piece that DOES change an existing field — a coup replacing the
//! captor house / regime — sits behind `GOV_POWER_DOSE` exactly as 04.1's own
//! doc comment already promised for "the NEW capture/coup rules (04.2,
//! 04.5)"; at the shipped `GOV_POWER_DOSE = 0.0` it is computed and would-log
//! only, per that same convention.
use super::*;

/// Mirrors `development.rs::NEUTRAL_LEGITIMACY` (0.75) — kept as its own
/// named constant here because `development.rs`'s is private to that module
/// and this is a genuinely separate seed value (a real per-city field, not a
/// forward-hook placeholder), even though the two are equal by design so a
/// freshly seeded government reads exactly as "no different from before this
/// row existed".
pub(crate) const NEUTRAL_LEGITIMACY_SEED: f32 = 0.75;

/// One point per year at full stability (`stability_at` == 1.0), spent
/// weekly so a government accrues gradually rather than in one lump sum.
const GOV_POINTS_PER_WEEK: f32 = 1.0 / 52.0;
/// A minor edict's base cost, in the same "years of accrual" units.
const EDICT_COST_MINOR: f32 = 1.0;
/// A major edict's base cost — the doc's own 25-50 year midpoint.
const EDICT_COST_MAJOR: f32 = 35.0;
/// A council/senate-form government only ever proposes a major edict once it
/// already holds at least this fraction of a major edict's cost — otherwise
/// every week would-propose a major it can't possibly afford yet.
const MAJOR_EDICT_READY_FRAC: f32 = 0.5;
/// Chance, per week, that an ELIGIBLE government proposes a major edict
/// instead of a minor one (both gated on affording it — see above).
const MAJOR_EDICT_CHANCE: f32 = 0.20;
/// A closed debate's |tally| must clear this to read as a clear pass/fail
/// rather than running out the round cap into a deadlock.
const DEBATE_DECISIVE: f32 = 0.35;
/// A failed edict still spends this fraction of its cost (wasted campaigning).
const EDICT_FAIL_COST_FRAC: f32 = 0.4;
/// A deadlocked edict spends less still (no one committed to a side).
const EDICT_DEADLOCK_COST_FRAC: f32 = 0.15;
/// Deadlock's hit to legitimacy — a government that cannot decide looks weak.
const DEADLOCK_LEGITIMACY_HIT: f32 = 0.04;
/// A passed edict that matches the government's own lean is a small
/// legitimacy WIN; one that fights it (rare, since cost already discourages
/// it) costs a little.
const EDICT_LEGITIMACY_SWING: f32 = 0.02;
/// Every edict expires eventually (the doc's own rule) — minor terms run a
/// human generation, major ones roughly two.
const EDICT_MINOR_DURATION_YEARS: u32 = 25;
const EDICT_MAJOR_DURATION_YEARS: u32 = 50;
pub(crate) const GOV_EDICTS_CAP: usize = 12;
pub(crate) const GOV_HISTORY_CAP: usize = 20;
/// 04.6 — every five years, a direction edict (the Lustrum).
pub(crate) const LUSTRUM_YEARS: u32 = 5;
/// Q04.13 · points credited to the Lustrum's chosen track, at full dose.
/// `TRACK_THRESHOLDS[0]` (level 1) is 8.0, so 2.0 is a quarter of a level —
/// a real nudge every five years, not a level-up in itself (the doc's own
/// "benefits to its backers" reads as an edge, not a guarantee).
pub(crate) const LUSTRUM_TRACK_BONUS: f32 = 2.0;
/// Raised 0.0 → 1.0 (full bonus) in the same session it shipped, after
/// `tick::tests`/`econ_` both confirmed it safe (Q04.13's own dose walk —
/// see this row's doc/SCOREBOARD entry for the before/after numbers).
/// 0.0 remains a true no-op, still guarded by `lustrum_bonus_is_a_noop_at_
/// zero_dose`'s own pure-function test.
pub(crate) const LUSTRUM_TRACK_BONUS_DOSE: f32 = 1.0;

/// Pure `_e` split (the N6/S1-series pattern) so the zero-dose claim is
/// testable independent of whichever value is currently shipped.
pub(crate) fn lustrum_bonus_e(dose: f32) -> f32 { LUSTRUM_TRACK_BONUS * dose }

/// Q04.9's first slice: two edict families, once PASSED, may now enact the
/// matching ALREADY-LIVE standing `Law` — reusing a real effect rather than
/// inventing a new one. Welfare → `LAW_GRAIN` (eases `decide_crisis_relief`'s
/// dearth triggers, today only ever earned by living through a famine —
/// this is what lets a government act BEFORE one). Foreigners → `LAW_
/// FOREIGN_BAR` (blocks a foreign house's envoy without existing presence,
/// today only a rare "fresh capture" roll — this gives a genuinely
/// PROTECTIONIST government its own path to it). Both idempotent (a second
/// enactment while the law already stands is a no-op, the same guard
/// `enact_standing_laws`/the capture payoff already use) and both a real
/// behaviour change (a city that would never have crossed either law's own
/// trigger can now get it), so both sit behind ONE dose,
/// `EDICT_EFFECT_DOSE` — shipped `0.0` first
/// (`edict_effects_are_a_noop_at_zero_dose`), walked to `1.0` in the same
/// session once `tick::tests`/`econ_` confirmed it safe.
pub(crate) const EDICT_EFFECT_DOSE: f32 = 1.0;

fn edict_law_for_family(family: u8) -> Option<u8> {
    match family {
        EDICT_FAM_WELFARE => Some(LAW_GRAIN),
        EDICT_FAM_FOREIGNERS => Some(LAW_FOREIGN_BAR),
        _ => None,
    }
}

// ── Q04.9's remainder — one dose per family, each shipped at `0.0` (a true
// no-op), each a small, BOUNDED, one-time nudge applied at the moment an
// edict of that family PASSES (the same call site Welfare/Foreigners already
// use). None of the six moves population, price or a house's wealth —
// Economy nudges a hub's own `mint_fineness` (already 0..1-clamped and
// itself eased back toward `decide_polis_policy`'s own target every year, so
// a repeated edict cannot accumulate an unbounded swing); Military tops up
// `war_manpower` no further than the SAME soldier-pool ceiling `raise_
// manpower_levy` (war.rs) already enforces; Learning/Buildings credit
// `track_points` exactly like the already-dosed-to-1.0 Lustrum bonus (Q04.13)
// does; Citizenship nudges one existing `CultureRelation.score` (row 05,
// -100..100 by construction); Constitution adds or removes one generic
// role-4 seat, bounded by the existing `GOVT_SEAT_CAP`. Raising any of these
// needs its own `econ_` walk per 00_INDEX's own per-dose-step testing rule —
// left queued (Q04.9) for session budget, not attempted this pass.
pub(crate) const EDICT_ECONOMY_DOSE: f32 = 0.0;
pub(crate) const EDICT_MILITARY_DOSE: f32 = 0.0;
pub(crate) const EDICT_LEARNING_DOSE: f32 = 0.0;
pub(crate) const EDICT_CITIZENSHIP_DOSE: f32 = 0.0;
pub(crate) const EDICT_CONSTITUTION_DOSE: f32 = 0.0;
pub(crate) const EDICT_BUILDINGS_DOSE: f32 = 0.0;

/// A "mint reform" nudges fineness by this much (0..1 scale) — bounded, and
/// eased back toward `decide_polis_policy`'s own `mint_target` every year
/// regardless, so it can never accumulate across repeated edicts.
const ECONOMY_MINT_BUMP: f32 = 0.05;
/// "Raise walls / a levy / hire mercenaries" folded into one readiness bump —
/// a fraction of the SAME soldier-pool ceiling `war.rs::raise_manpower_levy`
/// already caps `war_manpower` at, so this can never exceed what an ordinary
/// wartime levy could reach on its own.
const MILITARY_MANPOWER_BUMP_FRAC: f32 = 0.20;
/// "Fund a school / found a university" — the same magnitude Q04.13's own
/// Lustrum bonus credits, since both are "a government decision nudges one
/// development track" and neither has ever needed a bigger one.
pub(crate) const LEARNING_TRACK_BONUS: f32 = 2.0;
pub(crate) const BUILDINGS_TRACK_BONUS: f32 = 2.0;
/// "Grant a tier" to the city's most-established resident minority — well
/// under one hysteresis band (`ACCEPT_T*` in `culture_acceptance.rs` sit
/// ~15-30 apart), so a single edict nudges, never instantly promotes.
const CITIZENSHIP_SCORE_BUMP: f32 = 6.0;

fn neg_one_i8_regime_kind() -> i8 { -1 }

/// Q04.5b · human name for a `GovHistoryEntry.regime_kind` — `""` for an
/// ordinary edict/Lustrum entry (`regime_kind < 0`).
pub fn regime_kind_name(k: i8) -> &'static str {
    match k {
        REGIME_COUP => "coup",
        REGIME_REVOLUTION => "revolution",
        REGIME_OLIGARCHIC_CLOSING => "oligarchic closing",
        REGIME_EMERGENCY_RULER => "emergency rule",
        REGIME_SUCCESSION_CRISIS => "succession crisis",
        REGIME_REFORM => "reform",
        REGIME_OSTRACISM => "ostracism",
        _ => "",
    }
}

/// Q04.5b · the doc's own six-kind "changes of government" table, minus
/// "Imposed" (already exists — a war's Enthrone goal / realm formation, row
/// 09) — all behind `GOV_POWER_DOSE` (0.0, unchanged from 04.1's own
/// promise), plus ostracism.
pub const REGIME_COUP: i8 = 0;
pub const REGIME_REVOLUTION: i8 = 1;
pub const REGIME_OLIGARCHIC_CLOSING: i8 = 2;
pub const REGIME_EMERGENCY_RULER: i8 = 3;
pub const REGIME_SUCCESSION_CRISIS: i8 = 4;
pub const REGIME_REFORM: i8 = 5;
pub const REGIME_OSTRACISM: i8 = 6;

/// No two LARGE regime changes (everything but ostracism) may fire in the
/// same city within this many years — bounds churn (CLAUDE.md rule 22's
/// discipline extended from one mechanism's own round cap to a floor on how
/// often the whole government can flip form).
const GOV_CHANGE_COOLDOWN_YEARS: u32 = 15;

const COUP_LEGITIMACY_CEILING: f32 = 0.35;
const COUP_WEALTH_REF: f32 = 200_000.0;
const COUP_FLEET_WEIGHT: f32 = 0.03;
const COUP_STRENGTH_FLOOR: f32 = 0.5;
const COUP_CHANCE_SCALE: f32 = 0.30;
const COUP_CHANCE_CAP: f32 = 0.20;
const COUP_LEGITIMACY_RESET: f32 = 0.55;

pub(crate) const EMERGENCY_RULER_YEARS: u32 = 5;
const EMERGENCY_RULER_CHANCE: f32 = 0.25;
const EMERGENCY_KEEP_CHANCE: f32 = 0.20;

const REVOLUTION_UNREST_FLOOR: f32 = 0.55;
const REVOLUTION_CHANCE_SCALE: f32 = 0.40;
const REVOLUTION_CHANCE_CAP: f32 = 0.25;
const REVOLUTION_LEGITIMACY_RESET: f32 = 0.55;

const CLOSING_WEALTH_FLOOR: f32 = 50_000.0;
const CLOSING_MIN_RICH_HOUSES: usize = 3;
const CLOSING_LEGITIMACY_FLOOR: f32 = 0.55;
const CLOSING_BASE_CHANCE: f32 = 0.15;
const CLOSING_LEGITIMACY_HIT: f32 = 0.05;

const REFORM_IDEO_LEVEL_FLOOR: u8 = 3;
const REFORM_BASE_CHANCE: f32 = 0.12;
const REFORM_LEGITIMACY_RESET: f32 = 0.65;

/// Q04.5b · "once a year the assembly may vote to exile one person for 10
/// years."
pub(crate) const OSTRACISM_YEARS: u32 = 10;
const OSTRACISM_CHANCE: f32 = 0.10;
const OSTRACISM_CAP: usize = 8;

/// Round cap by government form (04.4's own table). `govt_type` 0 = Council
/// (also standing in for "Senate" — the code has no distinct fourth form
/// yet, Q04.10), 1 = Principality (no debate at all — 04.5's tyrant path),
/// 2 = Free Commune/Assembly.
fn round_cap_for(govt_type: u8, major: bool) -> u8 {
    match (govt_type, major) {
        (0, false) => 1,
        (0, true) => 4,
        (2, false) => 1,
        (2, true) => 2,
        _ => 1,
    }
}

/// 04.3's edict families and their fixed ideology tag (−1 conservative · 0
/// neutral · 1 libertarian). The Lustrum is deliberately NOT one of these —
/// it is its own 5-yearly mechanism (04.6), never proposed through this list.
pub(crate) const EDICT_FAM_CITIZENSHIP: u8 = 0;
pub(crate) const EDICT_FAM_FOREIGNERS: u8 = 1;
pub(crate) const EDICT_FAM_LEARNING: u8 = 2;
pub(crate) const EDICT_FAM_WELFARE: u8 = 3;
pub(crate) const EDICT_FAM_ECONOMY: u8 = 4;
pub(crate) const EDICT_FAM_MILITARY: u8 = 5;
pub(crate) const EDICT_FAM_CONSTITUTION: u8 = 6;
pub(crate) const EDICT_FAM_BUILDINGS: u8 = 7;
pub(crate) const EDICT_FAMILY_COUNT: usize = 8;

fn edict_family_tag(family: u8) -> i8 {
    match family {
        EDICT_FAM_CITIZENSHIP => 1,
        EDICT_FAM_FOREIGNERS => -1,
        EDICT_FAM_LEARNING => 1,
        EDICT_FAM_WELFARE => -1,
        EDICT_FAM_ECONOMY => 0,
        EDICT_FAM_MILITARY => -1,
        EDICT_FAM_CONSTITUTION => 0,
        _ => 0, // Buildings — a track choice (row 03), not an ideological one
    }
}

/// 04.3's own cost formula, verbatim: `base × (1 + distance between the
/// edict's ideology position and the government's position)` — a libertarian
/// edict is cheap in a libertarian government and dear in a conservative one,
/// and the reverse. A pure function so the gate (`mismatched_edicts_cost_
/// more`) can test the formula directly rather than through the whole
/// propose→afford path, which also rolls randomness.
pub(crate) fn edict_cost(base: f32, tag: i8, gov_position: f32) -> f32 {
    base * (1.0 + (tag as f32 - gov_position).abs())
}

pub fn edict_family_name(family: u8) -> &'static str {
    match family {
        EDICT_FAM_CITIZENSHIP => "citizenship and culture",
        EDICT_FAM_FOREIGNERS => "foreigners",
        EDICT_FAM_LEARNING => "learning",
        EDICT_FAM_WELFARE => "welfare",
        EDICT_FAM_ECONOMY => "economy",
        EDICT_FAM_MILITARY => "military",
        EDICT_FAM_CONSTITUTION => "constitution",
        _ => "buildings",
    }
}

/// One edict currently in force, held until `expires_tick`. `family`/`tag`
/// record what it was and how it leant; nothing downstream reads either yet
/// (Q04.9 — see module doc comment).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GovEdict {
    pub family: u8,
    pub tag: i8,
    pub major: bool,
    pub enacted_tick: u32,
    pub expires_tick: u32,
}

/// A debate in progress on one proposed edict — never held under a tyranny
/// (04.5's ruler decides directly, no vote).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GovDebate {
    pub family: u8,
    pub tag: i8,
    pub major: bool,
    pub cost: f32,
    pub round: u8,
    pub round_cap: u8,
    /// Running lean, −1 (solid fail) .. +1 (solid pass), smoothed round to
    /// round rather than reset, so persuasion/bribery in one round carries
    /// weight into the next.
    pub tally: f32,
}

/// One closed debate's outcome, or a coup — the Government window's "recent
/// history" list.
pub const GOV_OUTCOME_PASSED: u8 = 0;
pub const GOV_OUTCOME_FAILED: u8 = 1;
pub const GOV_OUTCOME_DEADLOCKED: u8 = 2;
pub const GOV_OUTCOME_COUP: u8 = 3;
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GovHistoryEntry {
    pub tick: u32,
    pub family: u8,
    pub outcome: u8,
    /// Q04.5b · which `REGIME_*` kind this entry records, when `outcome ==
    /// GOV_OUTCOME_COUP` (reused as "a change of government", per the doc's
    /// own six-kind table) — `-1` for an ordinary edict/Lustrum entry. Every
    /// construction site sets this explicitly; the serde default only ever
    /// fires for an entry recorded before this field existed, which is
    /// correctly read as "not a regime change".
    #[serde(default = "neg_one_i8_regime_kind")]
    pub regime_kind: i8,
}

/// A culture's ideology lean from its own most-characteristic real trait
/// (`ideal_for_trait`) — the row's own forward-hook convention, reused
/// rather than re-invented (04's intro names the exact trait examples this
/// mirrors: Mercantile → libertarian/economic, Insular/Xenophobic →
/// conservative, Clannish → conservative-authority).
pub(crate) fn gov_position_for_ideal(ideal: Option<usize>) -> f32 {
    match ideal {
        Some(IDEAL_WEALTH) => 0.4,
        Some(IDEAL_LEARNING) => 0.3,
        Some(IDEAL_REACH) => 0.2,
        Some(IDEAL_ASSIMILATION) => 0.1,
        Some(IDEAL_CONQUEST) => 0.0,
        Some(IDEAL_STABILITY) => -0.4,
        Some(IDEAL_LINEAGE) => -0.5,
        Some(IDEAL_PURITY) => -0.5,
        Some(IDEAL_TRADITION) => -0.6,
        _ => 0.0,
    }
}

fn push_gov_history(hub: &mut TickHub, entry: GovHistoryEntry) {
    hub.gov_history.push(entry);
    if hub.gov_history.len() > GOV_HISTORY_CAP {
        let drop = hub.gov_history.len() - GOV_HISTORY_CAP;
        hub.gov_history.drain(0..drop);
    }
}

impl CampaignSim {
    /// 04.3-04.6 · the weekly cadence hook (00_INDEX "Cadence hooks",
    /// `tick % 7`) — accrue points, run one debate round (or a tyrant's own
    /// no-vote decision), expire spent edicts. `O(hubs × officials)`, well
    /// inside this layer's <100 ms/year budget (a few thousand hubs × ~4
    /// officials, once a week).
    pub(crate) fn government_weekly_pass(&mut self) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            // A city whose government hasn't been seeded yet (the yearly
            // `update_government` pass seeds it once) has nothing to debate —
            // this only matters in a campaign's very first partial year.
            if self.hubs[h].officials.is_empty() { continue; }
            self.government_weekly_step(h);
        }
    }

    fn government_weekly_step(&mut self, h: usize) {
        let stab = self.stability_at(h);
        self.hubs[h].gov_points += GOV_POINTS_PER_WEEK * stab;

        if self.hubs[h].gov_debate.is_some() {
            self.run_debate_round(h);
        } else if self.hubs[h].govt_type == 1 {
            self.maybe_tyrant_decide(h);
        } else {
            self.maybe_propose_edict(h);
        }
        self.expire_edicts(h);
    }

    /// Pick a family weighted by open issues (war → Military, famine →
    /// Welfare, else a plain hashed pick), open a debate once the government
    /// can afford at least the minor cost. A council/senate/assembly debates;
    /// a tyranny never reaches this arm (04.5's own path handles it).
    fn maybe_propose_edict(&mut self, h: usize) {
        let hub = &self.hubs[h];
        if hub.gov_points < EDICT_COST_MINOR { return; }
        let family = self.pick_edict_family(h);
        let tag = edict_family_tag(family);
        let dist = (tag as f32 - self.hubs[h].gov_position).abs();
        let major = self.hubs[h].gov_points >= EDICT_COST_MAJOR * (1.0 + dist) * MAJOR_EDICT_READY_FRAC
            && hash01(self.seed, self.tick as u64, h as u64 ^ 0xE01C) < MAJOR_EDICT_CHANCE;
        let base = if major { EDICT_COST_MAJOR } else { EDICT_COST_MINOR };
        let cost = edict_cost(base, tag, self.hubs[h].gov_position);
        if self.hubs[h].gov_points < cost { return; }
        let round_cap = round_cap_for(self.hubs[h].govt_type, major);
        self.hubs[h].gov_debate = Some(GovDebate { family, tag, major, cost, round: 0, tally: 0.0, round_cap });
    }

    fn pick_edict_family(&self, h: usize) -> u8 {
        let hub = &self.hubs[h];
        if hub.war_with >= 0 { return EDICT_FAM_MILITARY; }
        if hub.starving > 0.3 { return EDICT_FAM_WELFARE; }
        // Otherwise a plain weighted pick favouring a family closer to this
        // government's own lean (cheaper to pass), via rejection against a
        // hashed roll rather than a full weighted-sample table.
        for attempt in 0..EDICT_FAMILY_COUNT as u64 {
            let f = ((hash01(self.seed, self.tick as u64 ^ attempt, h as u64 ^ 0xFA30) * EDICT_FAMILY_COUNT as f32) as u8)
                .min(EDICT_FAMILY_COUNT as u8 - 1);
            let dist = (edict_family_tag(f) as f32 - hub.gov_position).abs();
            if hash01(self.seed, self.tick as u64 ^ attempt, h as u64 ^ 0x0FA3) < 1.0 - dist * 0.4 {
                return f;
            }
        }
        EDICT_FAM_ECONOMY
    }

    /// One weekly round: each seated official leans toward the edict by how
    /// close its tag sits to their OWN allegiance-derived position (a house
    /// seat mirrors its patron's tilt more than the government's average;
    /// everyone else reads the government's own `gov_position`), noised by
    /// `1 - suitability` (a less capable member is more swayable either way —
    /// persuasion, bribery and simple foolishness folded into one term rather
    /// than three separate mechanics, a documented scope cut — Q04.11).
    pub(crate) fn run_debate_round(&mut self, h: usize) {
        let Some(mut deb) = self.hubs[h].gov_debate.take() else { return };
        deb.round += 1;
        let gov_lean = self.hubs[h].gov_position;
        let mut lean_sum = 0.0f32;
        let mut n = 0.0f32;
        for (oi, o) in self.hubs[h].officials.iter().enumerate() {
            let base = if official_allegiance(o) == ALLEGIANCE_HOUSE { gov_lean * 0.5 } else { gov_lean };
            let align = 1.0 - (deb.tag as f32 - base).abs() * 0.5;
            let noise = (hash01(self.seed, self.tick as u64 ^ (oi as u64), h as u64 ^ deb.round as u64) * 2.0 - 1.0)
                * (1.0 - o.suitability);
            lean_sum += (align - 0.5) * 2.0 + noise;
            n += 1.0;
        }
        let round_lean = if n > EPS { (lean_sum / n).clamp(-1.0, 1.0) } else { 0.0 };
        deb.tally = (deb.tally * 0.5 + round_lean * 0.5).clamp(-1.0, 1.0);

        if deb.tally >= DEBATE_DECISIVE || deb.tally <= -DEBATE_DECISIVE || deb.round >= deb.round_cap {
            self.resolve_debate(h, deb);
        } else {
            self.hubs[h].gov_debate = Some(deb);
        }
    }

    pub(crate) fn resolve_debate(&mut self, h: usize, deb: GovDebate) {
        let tick = self.tick;
        let passed = deb.tally >= DEBATE_DECISIVE;
        let failed = deb.tally <= -DEBATE_DECISIVE;
        let outcome = if passed { GOV_OUTCOME_PASSED } else if failed { GOV_OUTCOME_FAILED } else { GOV_OUTCOME_DEADLOCKED };
        let spend = if passed { deb.cost }
            else if failed { deb.cost * EDICT_FAIL_COST_FRAC }
            else { deb.cost * EDICT_DEADLOCK_COST_FRAC };
        self.hubs[h].gov_points = (self.hubs[h].gov_points - spend).max(0.0);

        if passed {
            let years = if deb.major { EDICT_MAJOR_DURATION_YEARS } else { EDICT_MINOR_DURATION_YEARS };
            let edict = GovEdict { family: deb.family, tag: deb.tag, major: deb.major, enacted_tick: tick, expires_tick: tick + years * TICKS_PER_YEAR };
            self.hubs[h].gov_edicts.push(edict);
            if self.hubs[h].gov_edicts.len() > GOV_EDICTS_CAP {
                let drop = self.hubs[h].gov_edicts.len() - GOV_EDICTS_CAP;
                self.hubs[h].gov_edicts.drain(0..drop);
            }
            let swing = if (deb.tag as f32 - self.hubs[h].gov_position).abs() < 0.5 { EDICT_LEGITIMACY_SWING } else { -EDICT_LEGITIMACY_SWING };
            self.hubs[h].legitimacy = (self.hubs[h].legitimacy + swing).clamp(0.0, 1.0);
        } else if outcome == GOV_OUTCOME_DEADLOCKED {
            self.hubs[h].legitimacy = (self.hubs[h].legitimacy - DEADLOCK_LEGITIMACY_HIT).clamp(0.0, 1.0);
        }

        if passed { self.apply_edict_effect(h, deb.family); }
        push_gov_history(&mut self.hubs[h], GovHistoryEntry { tick, family: deb.family, outcome, regime_kind: -1 });
        self.chronicle_edict_outcome(h, deb.family, outcome);
    }

    /// Q04.9 · a PASSED edict's real effect, called from both the debate
    /// path (on PASSED only) and the tyrant path (which always "passes").
    /// Welfare/Foreigners enact the matching standing `Law` (idempotent,
    /// gated on `EDICT_EFFECT_DOSE`); the other six families each carry
    /// their own dose (see the six `EDICT_*_DOSE` constants' own doc
    /// comment) and are dispatched here too.
    pub(crate) fn apply_edict_effect(&mut self, h: usize, family: u8) {
        if EDICT_EFFECT_DOSE > 0.0 {
            if let Some(kind) = edict_law_for_family(family) {
                if !self.hubs[h].laws.iter().any(|l| l.kind == kind) {
                    let year = self.tick / TICKS_PER_YEAR;
                    self.push_law(h, kind, -1, -1, year);
                }
            }
        }
        match family {
            EDICT_FAM_ECONOMY => self.apply_economy_edict(h, EDICT_ECONOMY_DOSE),
            EDICT_FAM_MILITARY => self.apply_military_edict(h, EDICT_MILITARY_DOSE),
            EDICT_FAM_LEARNING => self.apply_learning_edict(h, EDICT_LEARNING_DOSE),
            EDICT_FAM_CITIZENSHIP => self.apply_citizenship_edict(h, EDICT_CITIZENSHIP_DOSE),
            EDICT_FAM_CONSTITUTION => self.apply_constitution_edict(h, EDICT_CONSTITUTION_DOSE),
            EDICT_FAM_BUILDINGS => self.apply_buildings_edict(h, EDICT_BUILDINGS_DOSE),
            _ => {}
        }
    }

    /// "A mint reform" (Economy) — a one-time, bounded nudge to
    /// `mint_fineness`, in either direction (a stable per-edict hash, since
    /// the family's own ideology tag is neutral and cannot say which way).
    /// Reformed toward honest coin or debased toward cheap credit are both
    /// real historical "mint reforms"; `decide_polis_policy` eases the field
    /// back toward its own yearly target regardless, so this can never
    /// accumulate across repeated edicts. `dose` is passed explicitly (never
    /// read from `EDICT_ECONOMY_DOSE` internally) so a test can exercise the
    /// real mechanism while the shipped call site always passes the
    /// dose-zero constant (the `track_build_progress_e`/`update_track_
    /// buildings(dose)` pattern).
    pub(crate) fn apply_economy_edict(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        if self.hubs[h].mint_fineness <= 0.0 { self.hubs[h].mint_fineness = 1.0; }
        let dir = if hash01(self.seed, self.tick as u64, h as u64 ^ 0xEC0E) < 0.5 { 1.0 } else { -1.0 };
        self.hubs[h].mint_fineness = (self.hubs[h].mint_fineness + dir * ECONOMY_MINT_BUMP * dose).clamp(0.5, 1.0);
    }

    /// "Raise walls / a levy / hire mercenaries" (Military) — tops up
    /// `war_manpower` no further than the same soldier-pool ceiling an
    /// ordinary wartime levy (`war.rs::raise_manpower_levy`) already caps it
    /// at, so a Military edict can never out-arm what the city could raise
    /// on its own in a real war.
    pub(crate) fn apply_military_edict(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        let soldiers = self.hubs[h].pops.iter().find(|p| p.profession == POP_SOLDIER).map(|p| p.size).unwrap_or(0.0);
        let ceiling = soldiers * LEVY_MAX_FRAC_OF_SOLDIERS;
        if ceiling <= EPS { return; }
        let bump = ceiling * MILITARY_MANPOWER_BUMP_FRAC * dose;
        self.hubs[h].war_manpower = (self.hubs[h].war_manpower + bump).min(ceiling);
    }

    /// "Fund a school / found a university" (Learning) — credits the
    /// Ideological track exactly like Q04.13's own Lustrum bonus.
    pub(crate) fn apply_learning_edict(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        self.hubs[h].track_points[TRACK_IDEOLOGICAL] += LEARNING_TRACK_BONUS * dose;
    }

    /// "Build a track building" (Buildings) — credits whichever track
    /// currently trails the others, the same choice Q04.6's own Lustrum
    /// makes for its 5-yearly bonus.
    pub(crate) fn apply_buildings_edict(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        let track = self.hubs[h].track_points.iter().enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
        self.hubs[h].track_points[track] += BUILDINGS_TRACK_BONUS * dose;
    }

    /// "Grant a tier" (Citizenship, row 05) — nudges the most-established
    /// resident minority's own acceptance score upward. A city with no
    /// recorded minority relation yet (`culture_relations` not seeded, or no
    /// minority present) has nothing to grant to — a true no-op regardless
    /// of dose, not a fabricated target.
    pub(crate) fn apply_citizenship_edict(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        let majority = self.hub_culture.get(h).cloned().unwrap_or_default();
        let Some(rel) = self.hubs[h].culture_relations.iter_mut()
            .filter(|r| r.culture != majority)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
        else { return };
        rel.score = (rel.score + CITIZENSHIP_SCORE_BUMP * dose).clamp(-100.0, 100.0);
        rel.tier = tier_for_score(rel.score, rel.tier);
    }

    /// "Create or abolish an office" (Constitution) — a stable per-edict hash
    /// (the family's own tag is neutral) picks the direction; CREATE adds one
    /// generic role-4 seat, bounded by `GOVT_SEAT_CAP`; ABOLISH removes the
    /// most recently created one, if any — never one of the four named
    /// offices.
    pub(crate) fn apply_constitution_edict(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        let create = hash01(self.seed, self.tick as u64, h as u64 ^ 0xC057) < 0.5;
        if create {
            if self.hubs[h].officials.len() >= GOVT_SEAT_CAP { return; }
            let city = self.hubs[h].name.clone();
            let salt = (h as u64).wrapping_mul(0x9E37).wrapping_add((self.tick as u64) ^ 0x4F);
            let name = self.head_name_for(h, &city, salt);
            let suitability = official_suitability_roll(self.seed, h, salt);
            let iid = self.individual_id_for_official(h, &name);
            let govt = (self.hubs[h].govt_type as usize).min(2);
            let te = self.tick + GOVT_TERM_YEARS[govt] * TICKS_PER_YEAR;
            self.hubs[h].officials.push(Official {
                role: 4, name, house: -1, control: 0.0, kin: false, term_end: te,
                path: PATH_APPOINTED, suitability, individual_id: iid,
            });
        } else if let Some(pos) = self.hubs[h].officials.iter().rposition(|o| o.role == 4) {
            self.hubs[h].officials.remove(pos);
        }
    }

    /// A tyrant skips the vote entirely (04.5). Once points allow, the ruler
    /// simply decides — biased toward their own government's lean, since
    /// nothing else yet models the ruler as their own `Individual` with a
    /// character axis of their own (Q04.3's own note: `path`/`suitability`
    /// exist per seat, but nothing here reads the HEAD seat's specifically —
    /// a documented scope cut, Q04.12). No vote tally, no debate rounds.
    fn maybe_tyrant_decide(&mut self, h: usize) {
        if self.hubs[h].gov_points < EDICT_COST_MINOR { return; }
        let family = self.pick_edict_family(h);
        let tag = edict_family_tag(family);
        let dist = (tag as f32 - self.hubs[h].gov_position).abs();
        let major = self.hubs[h].gov_points >= EDICT_COST_MAJOR * (1.0 + dist) * MAJOR_EDICT_READY_FRAC
            && hash01(self.seed, self.tick as u64, h as u64 ^ 0x7A24) < MAJOR_EDICT_CHANCE;
        let base = if major { EDICT_COST_MAJOR } else { EDICT_COST_MINOR };
        let cost = edict_cost(base, tag, self.hubs[h].gov_position);
        if self.hubs[h].gov_points < cost { return; }
        let tick = self.tick;
        self.hubs[h].gov_points -= cost;
        let years = if major { EDICT_MAJOR_DURATION_YEARS } else { EDICT_MINOR_DURATION_YEARS };
        self.hubs[h].gov_edicts.push(GovEdict { family, tag, major, enacted_tick: tick, expires_tick: tick + years * TICKS_PER_YEAR });
        if self.hubs[h].gov_edicts.len() > GOV_EDICTS_CAP {
            let drop = self.hubs[h].gov_edicts.len() - GOV_EDICTS_CAP;
            self.hubs[h].gov_edicts.drain(0..drop);
        }
        // An edict against the commons' comfort (read from the existing
        // `mood` sentiment — the closest proxy to "the commons oppose this"
        // this session builds; a real commons meter is row 06's job) raises
        // overthrow risk, which at `GOV_POWER_DOSE == 0.0` is recorded as a
        // legitimacy cost only — no regime change fires from this arm while
        // the dose is zero (the 04.1-established convention).
        let opposition = (1.0 - self.hubs[h].mood).clamp(0.0, 1.0);
        let risk = opposition * (1.0 - self.hubs[h].legitimacy) * if major { 1.5 } else { 0.6 };
        self.hubs[h].legitimacy = (self.hubs[h].legitimacy - risk * 0.05).clamp(0.0, 1.0);
        if GOV_POWER_DOSE > 0.0 {
            // Q04.5 (not built this session): the actual overthrow roll and
            // regime-change mutation would live here, gated on this exact
            // dose, per 04.1's own doc comment.
        }
        self.apply_edict_effect(h, family);
        push_gov_history(&mut self.hubs[h], GovHistoryEntry { tick, family, outcome: GOV_OUTCOME_PASSED, regime_kind: -1 });
        self.chronicle_edict_outcome(h, family, GOV_OUTCOME_PASSED);
    }

    pub(crate) fn expire_edicts(&mut self, h: usize) {
        let tick = self.tick;
        self.hubs[h].gov_edicts.retain(|e| e.expires_tick > tick);
    }

    /// Chronicle salience (00_INDEX's own rule, already applied to crises and
    /// the L12 townspeople): only a tier 1-2 city's edict reaches the WORLD
    /// journal; every city's own `gov_history` still records it in full.
    fn chronicle_edict_outcome(&mut self, h: usize, family: u8, outcome: u8) {
        if !(1..=2).contains(&self.hubs[h].tier) { return; }
        let city = self.hubs[h].name.clone();
        let fam = edict_family_name(family);
        let text = match outcome {
            GOV_OUTCOME_PASSED => format!("{} enacts a {} edict", city, fam),
            GOV_OUTCOME_FAILED => format!("{}'s {} edict fails", city, fam),
            _ => format!("{}'s government deadlocks over a {} edict", city, fam),
        };
        self.journal.push(JournalEntry { tick: self.tick, kind: "government".into(), hub: h as i32, good: -1, value: 0.0, text });
    }

    /// 04.6 — every `LUSTRUM_YEARS`, a direction edict feeding one of row
    /// 03's four development tracks (military/trade/civil/ideological, by
    /// whichever currently trails the others — "benefits to its backers" per
    /// the doc's own words, read here as simply the track most worth
    /// investing in). Called yearly (`update_government`'s own cadence),
    /// never weekly — the Lustrum is explicitly its own 5-yearly allowance,
    /// separate from the weekly edict points above.
    ///
    /// Q04.13 — now credits the chosen track with `LUSTRUM_TRACK_BONUS ×
    /// LUSTRUM_TRACK_BONUS_DOSE` real points, BEFORE `update_tracks` recomputes
    /// `track_level` from points later the same year (`update_government` runs
    /// ahead of `update_food_and_starvation`'s own `update_tracks` call in the
    /// yearly sequence, so a Lustrum-earned level shows the same year it
    /// fires, never a year late). Shipped at dose 0.0 first and proven inert
    /// (`lustrum_bonus_is_a_noop_at_zero_dose`), then raised to 1.0 in the
    /// same session once `tick::tests`/`econ_` confirmed it safe — see
    /// CLAUDE.md §2.4's own discipline: a dose is walked, not assumed.
    pub(crate) fn maybe_run_lustrum(&mut self, h: usize) {
        if self.hubs[h].is_estate || self.hubs[h].abandoned { return; }
        if self.hubs[h].officials.is_empty() { return; }
        if self.tick < self.hubs[h].gov_lustrum_tick { return; }
        self.hubs[h].gov_lustrum_tick = self.tick + LUSTRUM_YEARS * TICKS_PER_YEAR;
        let track = self.hubs[h].track_points.iter().enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
        self.hubs[h].track_points[track] += lustrum_bonus_e(LUSTRUM_TRACK_BONUS_DOSE);
        push_gov_history(&mut self.hubs[h], GovHistoryEntry { tick: self.tick, family: EDICT_FAM_BUILDINGS, outcome: GOV_OUTCOME_PASSED, regime_kind: -1 });
        if (1..=2).contains(&self.hubs[h].tier) {
            let city = self.hubs[h].name.clone();
            let track_name = track_name_for(track);
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "government".into(), hub: h as i32, good: -1, value: 0.0,
                text: format!("{}'s Lustrum favours the {} track", city, track_name),
            });
        }
    }
}

// ── Q04.5b — 5 of the doc's 6 "changes of government" kinds (Imposed
// already exists — a war's Enthrone goal / realm formation) + ostracism.
// All behind `GOV_POWER_DOSE`, shipped `0.0` exactly as 04.1's own doc
// comment promised for "the NEW capture/coup rules" — every mechanism below
// is real, tested at a nonzero TEST dose, and a true no-op at the shipped
// dose (`government_change_kinds_are_noops_at_zero_dose`). Never walked up
// this session (CLAUDE.md §2.4 — a dose is walked only once its own gate has
// judged it, and six regime-change kinds moving population/political state
// at once is exactly the "three doses in one session is the ceiling" risk
// this codebase's own plans keep naming; left queued, Q04.5b).
impl CampaignSim {
    /// The yearly regime-change check, called once per settled hub from
    /// `update_government`'s own per-hub loop, right after its existing
    /// capture/bribery bookkeeping. `dose` is passed explicitly (never read
    /// from `GOV_POWER_DOSE` internally) so a test can exercise the real
    /// mechanism while the shipped call site always passes the dose-zero
    /// constant. A realm capital's CROWN never changes hands here (rule 27)
    /// — every mutation below touches only `hubs[h].govt_type`/`officials`,
    /// never `Realm`/`prov_realm`, so a coup in a realm capital replaces the
    /// CITY's government only, by construction rather than by a special
    /// case. At most one LARGE regime change fires per city per year (tried
    /// in the doc's own rough Polybius order, first match wins); ostracism
    /// is small, independent, and may fire in the same year as one of them.
    pub(crate) fn government_change_pass(&mut self, h: usize, dose: f32) {
        if dose <= 0.0 { return; }
        if self.hubs[h].is_estate || self.hubs[h].abandoned { return; }
        if self.hubs[h].officials.is_empty() { return; }
        self.maybe_end_emergency_rule(h, dose);
        if self.tick >= self.hubs[h].gov_change_cooldown {
            let fired = self.maybe_succession_crisis(h, dose)
                || self.maybe_coup(h, dose)
                || self.maybe_emergency_ruler(h, dose)
                || self.maybe_revolution(h, dose)
                || self.maybe_oligarchic_closing(h, dose)
                || self.maybe_reform(h, dose);
            if fired {
                self.hubs[h].gov_change_cooldown = self.tick + GOV_CHANGE_COOLDOWN_YEARS * TICKS_PER_YEAR;
            }
        }
        self.maybe_ostracism(h, dose);
    }

    fn chronicle_regime_change(&mut self, h: usize, kind: i8, text: String) {
        push_gov_history(&mut self.hubs[h], GovHistoryEntry {
            tick: self.tick, family: 0, outcome: GOV_OUTCOME_COUP, regime_kind: kind,
        });
        // Chronicle salience (00_INDEX's own rule) — only a tier 1-2 city's
        // regime change reaches the WORLD journal; every city's own
        // `gov_history` still records it in full via `push_gov_history` above.
        if !(1..=2).contains(&self.hubs[h].tier) { return; }
        self.journal.push(JournalEntry { tick: self.tick, kind: "government".into(), hub: h as i32, good: -1, value: 0.0, text });
    }

    /// "A tyrant dies without an accepted heir" (reuse `crisis.rs` in
    /// spirit — the same `heir_is_female`/culture-`LineRule` filter a forced
    /// house-head installation already applies, CLAUDE.md rule 23, since the
    /// successor here is drawn from a real house's own kin roster). A dead
    /// ruler is read directly off `self.people` (an `Individual` no longer
    /// present there is dead — `remove_dead_individual`'s own convention),
    /// so no new death hook is needed.
    pub(crate) fn maybe_succession_crisis(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type != 1 { return false; }
        let Some(head_idx) = self.hubs[h].officials.iter().position(|o| o.role == 0) else { return false };
        let iid = self.hubs[h].officials[head_idx].individual_id;
        if iid < 0 || self.people.iter().any(|p| p.id == iid as u32) { return false; } // still alive, or never linked
        let house = if self.hubs[h].officials[head_idx].kin { self.hubs[h].officials[head_idx].house }
            else { self.hubs[h].captor_house };
        let has_heir = house >= 0 && (house as usize) < self.houses.len()
            && !self.houses[house as usize].defunct && !self.houses[house as usize].kin.is_empty();
        let tick = self.tick;
        let cn = self.hubs[h].name.clone();
        if has_heir {
            let hi = house as usize;
            let (line_rule, _inh) = self.rules_for_hub(h);
            let female = crate::sim::inheritance::heir_is_female(line_rule, (h as u64) << 8 ^ tick as u64 ^ 0x5CC5, self.seed);
            let hname = self.houses[hi].name.clone();
            let name = self.head_name_sexed_for(h, &hname, tick as u64 ^ 0x5CC5, female);
            let suitability = official_suitability_roll(self.seed, h, tick as u64 ^ 0x5CC5);
            let iid2 = self.individual_id_for_official(h, &name);
            let term = GOVT_TERM_YEARS[1] * TICKS_PER_YEAR;
            {
                let o = &mut self.hubs[h].officials[head_idx];
                o.name = name; o.term_end = tick + term; o.path = PATH_KIN;
                o.suitability = suitability; o.individual_id = iid2;
                o.house = hi as i32; o.control = 1.0; o.kin = true;
            }
            self.hubs[h].legitimacy = (self.hubs[h].legitimacy + 0.10 * dose).clamp(0.0, 1.0);
            self.chronicle_regime_change(h, REGIME_SUCCESSION_CRISIS, format!("{cn}'s ruler dies; the succession passes to an heir"));
        } else {
            self.hubs[h].legitimacy = (self.hubs[h].legitimacy - 0.35 * dose).clamp(0.0, 1.0);
            if self.hubs[h].legitimacy < 0.15 {
                self.hubs[h].govt_type = 0;
                self.hubs[h].captor_house = -1;
            }
            self.chronicle_regime_change(h, REGIME_SUCCESSION_CRISIS, format!("{cn}'s ruler dies with no accepted heir — a succession crisis"));
        }
        true
    }

    /// "A house or commander with wealth/fleet/army vs a low-legitimacy
    /// government" — a Tyranny only; a council/assembly's own equivalent is
    /// the EXISTING control-weighted capture mechanism (`update_government`),
    /// dose-independent since 04.1.
    pub(crate) fn maybe_coup(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type != 1 { return false; }
        if self.hubs[h].legitimacy > COUP_LEGITIMACY_CEILING { return false; }
        let cur_house = self.hubs[h].officials.iter().find(|o| o.role == 0).map(|o| o.house).unwrap_or(-1);
        let mut best = (-1i32, 0.0f32);
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct || hh.is_guild || hi as i32 == cur_house || hh.hub as usize != h { continue; }
            let strength = (hh.wealth.max(0.0) / COUP_WEALTH_REF)
                + (hh.fleet_sea + hh.fleet_river + hh.fleet_caravan) as f32 * COUP_FLEET_WEIGHT;
            if strength > best.1 { best = (hi as i32, strength); }
        }
        if best.0 < 0 || best.1 < COUP_STRENGTH_FLOOR { return false; }
        let chance = (best.1 * (1.0 - self.hubs[h].legitimacy) * COUP_CHANCE_SCALE * dose).min(COUP_CHANCE_CAP);
        if hash01(self.seed, self.tick as u64, h as u64 ^ 0xC0DE) >= chance { return false; }
        let hi = best.0 as usize;
        let tick = self.tick;
        let (line_rule, _inh) = self.rules_for_hub(h);
        let female = crate::sim::inheritance::heir_is_female(line_rule, (h as u64) ^ (tick as u64) << 3 ^ 0xC0DE, self.seed);
        let hname = self.houses[hi].name.clone();
        let name = self.head_name_sexed_for(h, &hname, tick as u64 ^ 0xC0DE, female);
        let suitability = official_suitability_roll(self.seed, h, tick as u64 ^ 0xC0DE);
        let iid = self.individual_id_for_official(h, &name);
        let term = GOVT_TERM_YEARS[1] * TICKS_PER_YEAR;
        if let Some(o) = self.hubs[h].officials.iter_mut().find(|o| o.role == 0) {
            o.name = name; o.term_end = tick + term; o.path = PATH_MILITARY;
            o.suitability = suitability; o.individual_id = iid;
            o.house = hi as i32; o.control = 1.0; o.kin = true;
        }
        self.hubs[h].captor_house = hi as i32;
        self.hubs[h].legitimacy = COUP_LEGITIMACY_RESET;
        let (hn2, cn) = (self.houses[hi].name.clone(), self.hubs[h].name.clone());
        self.chronicle_regime_change(h, REGIME_COUP, format!("{hn2} seizes the throne of {cn} in a palace coup"));
        true
    }

    /// "War or plague; the body grants one person power for a term; they may
    /// keep it." Applies to a council/assembly (a Tyranny is already a
    /// single ruler). `maybe_end_emergency_rule` (called first, every year)
    /// reverts — or, rarely, makes permanent — an expired emergency.
    fn maybe_emergency_ruler(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type == 1 { return false; }
        if self.hubs[h].gov_emergency_prev_type >= 0 { return false; } // already in one
        let crisis = self.hubs[h].war_with >= 0 || self.hubs[h].starving > 0.5;
        if !crisis { return false; }
        if hash01(self.seed, self.tick as u64, h as u64 ^ 0xE43E) >= EMERGENCY_RULER_CHANCE * dose { return false; }
        self.hubs[h].gov_emergency_prev_type = self.hubs[h].govt_type as i8;
        self.hubs[h].govt_type = 1;
        self.hubs[h].gov_emergency_until = self.tick + EMERGENCY_RULER_YEARS * TICKS_PER_YEAR;
        if let Some(head) = self.hubs[h].officials.iter_mut().find(|o| o.role == 0) { head.path = PATH_MILITARY; }
        self.hubs[h].legitimacy = (self.hubs[h].legitimacy + 0.05 * dose).clamp(0.0, 1.0);
        let cn = self.hubs[h].name.clone();
        self.chronicle_regime_change(h, REGIME_EMERGENCY_RULER, format!("{cn} grants one ruler emergency power"));
        true
    }

    /// Every year, before anything else: end an emergency whose term has
    /// passed — revert to the prior form, or (rarely) let it stand ("they
    /// may keep it", the historical route from an emergency dictatorship to
    /// a standing one).
    fn maybe_end_emergency_rule(&mut self, h: usize, dose: f32) {
        if self.hubs[h].gov_emergency_prev_type < 0 { return; }
        if self.tick < self.hubs[h].gov_emergency_until { return; }
        let keep = dose > 0.0 && hash01(self.seed, self.tick as u64, h as u64 ^ 0x5EED) < EMERGENCY_KEEP_CHANCE * dose;
        let cn = self.hubs[h].name.clone();
        if keep {
            self.chronicle_regime_change(h, REGIME_EMERGENCY_RULER, format!("{cn}'s emergency ruler never steps down"));
        } else {
            self.hubs[h].govt_type = self.hubs[h].gov_emergency_prev_type as u8;
            self.chronicle_regime_change(h, REGIME_EMERGENCY_RULER, format!("{cn}'s emergency ruler steps down"));
        }
        self.hubs[h].gov_emergency_prev_type = -1;
        self.hubs[h].gov_emergency_until = 0;
    }

    /// "Commons' meter far from the government + unrest + a demagogue" — the
    /// commons-meter forward hook (row 06) reads `1 - mood` until then, per
    /// this row's own intro.
    fn maybe_revolution(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type == 2 { return false; } // already an assembly
        let unrest = self.hubs[h].society.unrest;
        if unrest < REVOLUTION_UNREST_FLOOR { return false; }
        let gap = (1.0 - self.hubs[h].mood).clamp(0.0, 1.0);
        let demagogue = self.figures.iter().any(|f| !f.dead && f.hub as usize == h && f.kind == 1);
        if !demagogue { return false; }
        let chance = (unrest * gap * REVOLUTION_CHANCE_SCALE * dose).min(REVOLUTION_CHANCE_CAP);
        if hash01(self.seed, self.tick as u64, h as u64 ^ 0x2E10) >= chance { return false; }
        self.hubs[h].govt_type = 2;
        self.hubs[h].captor_house = -1;
        let n = self.hubs[h].officials.len();
        for oi in 0..n { self.reseat_official(h, oi); }
        self.hubs[h].legitimacy = REVOLUTION_LEGITIMACY_RESET;
        let cn = self.hubs[h].name.clone();
        self.chronicle_regime_change(h, REGIME_REVOLUTION, format!("the commons of {cn} rise and found an assembly"));
        true
    }

    /// "A long-ruling council with rising rich houses" — Venice's own
    /// *Serrata*, 1297: entry closes to new families, permanently.
    fn maybe_oligarchic_closing(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type != 0 || self.hubs[h].gov_closed { return false; }
        if self.hubs[h].legitimacy < CLOSING_LEGITIMACY_FLOOR { return false; }
        let mut richest: Vec<(usize, f32)> = self.houses.iter().enumerate()
            .filter(|(_, hh)| !hh.defunct && !hh.is_guild && hh.hub as usize == h && hh.wealth > CLOSING_WEALTH_FLOOR)
            .map(|(i, hh)| (i, hh.wealth)).collect();
        if richest.len() < CLOSING_MIN_RICH_HOUSES { return false; }
        if hash01(self.seed, self.tick as u64, h as u64 ^ 0x5E44) >= CLOSING_BASE_CHANCE * dose { return false; }
        richest.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let mut ri = 0usize;
        for oi in 0..self.hubs[h].officials.len() {
            if self.hubs[h].officials[oi].kin { continue; }
            if ri >= richest.len() { break; }
            let hi = richest[ri].0; ri += 1;
            let o = &mut self.hubs[h].officials[oi];
            o.house = hi as i32; o.control = 1.0; o.kin = true; o.path = PATH_WEALTH;
        }
        self.hubs[h].gov_closed = true;
        self.hubs[h].legitimacy = (self.hubs[h].legitimacy - CLOSING_LEGITIMACY_HIT * dose).clamp(0.0, 1.0);
        let cn = self.hubs[h].name.clone();
        self.chronicle_regime_change(h, REGIME_OLIGARCHIC_CLOSING, format!("{cn}'s council closes itself to new families"));
        true
    }

    /// "A body votes to change the constitution, pushed by scholars" —
    /// Solon, Cleisthenes: a Tyranny reforms into a Council once the
    /// Ideological track has matured and a resident scholar pushes for it.
    fn maybe_reform(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type != 1 { return false; }
        if self.hubs[h].track_level[TRACK_IDEOLOGICAL] < REFORM_IDEO_LEVEL_FLOOR { return false; }
        let scholar = self.people.iter().any(|p| p.current_hub as usize == h
            && (p.roles.contains(&ROLE_SCHOLAR) || p.roles.contains(&ROLE_PHILOSOPHER) || p.roles.contains(&ROLE_IDEOLOGUE)));
        if !scholar { return false; }
        if hash01(self.seed, self.tick as u64, h as u64 ^ 0x501070) >= REFORM_BASE_CHANCE * dose { return false; }
        self.hubs[h].govt_type = 0;
        self.hubs[h].captor_house = -1;
        let n = self.hubs[h].officials.len();
        for oi in 0..n { self.reseat_official(h, oi); }
        self.hubs[h].legitimacy = REFORM_LEGITIMACY_RESET;
        let cn = self.hubs[h].name.clone();
        self.chronicle_regime_change(h, REGIME_REFORM, format!("{cn} reforms its constitution into a council"));
        true
    }

    /// "Once a year the assembly may vote to exile one person for 10 years."
    /// Exiles the seated official with the LOWEST suitability (never the
    /// Head) — the seat most likely to have stirred the trouble the
    /// assembly wants gone — and immediately reseats it fresh.
    pub(crate) fn maybe_ostracism(&mut self, h: usize, dose: f32) -> bool {
        if dose <= 0.0 || self.hubs[h].govt_type != 2 || self.hubs[h].officials.len() < 2 { return false; }
        if hash01(self.seed, self.tick as u64, h as u64 ^ 0x0577) >= OSTRACISM_CHANCE * dose { return false; }
        let Some((oi, iid)) = self.hubs[h].officials.iter().enumerate()
            .filter(|(_, o)| o.role != 0)
            .min_by(|a, b| a.1.suitability.partial_cmp(&b.1.suitability).unwrap())
            .map(|(oi, o)| (oi, o.individual_id))
        else { return false };
        let until = self.tick + OSTRACISM_YEARS * TICKS_PER_YEAR;
        self.hubs[h].ostracized.push((iid, until));
        if self.hubs[h].ostracized.len() > OSTRACISM_CAP {
            let d = self.hubs[h].ostracized.len() - OSTRACISM_CAP;
            self.hubs[h].ostracized.drain(0..d);
        }
        let name = self.hubs[h].officials[oi].name.clone();
        self.reseat_official(h, oi);
        let cn = self.hubs[h].name.clone();
        self.chronicle_regime_change(h, REGIME_OSTRACISM, format!("{name} is ostracized from {cn} for a decade"));
        true
    }
}

fn track_name_for(track: usize) -> &'static str {
    match track {
        TRACK_MILITARY => "military",
        TRACK_TRADE => "trade",
        TRACK_CIVIL => "civil",
        _ => "ideological",
    }
}
