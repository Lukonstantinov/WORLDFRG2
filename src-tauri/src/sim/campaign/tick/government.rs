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

        if passed { self.maybe_enact_edict_law(h, deb.family); }
        push_gov_history(&mut self.hubs[h], GovHistoryEntry { tick, family: deb.family, outcome });
        self.chronicle_edict_outcome(h, deb.family, outcome);
    }

    /// Q04.9 · at `EDICT_EFFECT_DOSE > 0.0`, a PASSED Welfare/Foreigners
    /// edict enacts the matching standing `Law` if this city doesn't already
    /// carry it — idempotent, dose-gated, called from both the debate path
    /// (on PASSED only) and the tyrant path (which always "passes").
    pub(crate) fn maybe_enact_edict_law(&mut self, h: usize, family: u8) {
        if EDICT_EFFECT_DOSE <= 0.0 { return; }
        let Some(kind) = edict_law_for_family(family) else { return };
        if self.hubs[h].laws.iter().any(|l| l.kind == kind) { return; }
        let year = self.tick / TICKS_PER_YEAR;
        self.push_law(h, kind, -1, -1, year);
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
        self.maybe_enact_edict_law(h, family);
        push_gov_history(&mut self.hubs[h], GovHistoryEntry { tick, family, outcome: GOV_OUTCOME_PASSED });
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
        push_gov_history(&mut self.hubs[h], GovHistoryEntry { tick: self.tick, family: EDICT_FAM_BUILDINGS, outcome: GOV_OUTCOME_PASSED });
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

fn track_name_for(track: usize) -> &'static str {
    match track {
        TRACK_MILITARY => "military",
        TRACK_TRADE => "trade",
        TRACK_CIVIL => "civil",
        _ => "ideological",
    }
}
