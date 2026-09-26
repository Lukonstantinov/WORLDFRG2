//! culture_acceptance — `docs/living_world/05_CULTURE_ACCEPTANCE.md`.
//!
//! Five acceptance TIERS per (city, culture) — Citizens .. Hated — with a
//! `-100..100` relation SCORE that drifts yearly and decides the tier
//! (hysteresis-banded, same discipline `assign_house_tiers`/`assign_city_
//! tiers` already use so a relation sitting on a boundary doesn't relabel
//! every year). State is SPARSE (`TickHub.culture_relations`, one entry per
//! culture actually resident or trading here, never a 1,200×every-culture
//! matrix, per 00_INDEX's own storage rule) and lazily seeded
//! (`ensure_culture_relations`).
//!
//! **Where this deliberately does NOT reuse row 04's `GovDebate` slot.** The
//! design doc asks for "an edict to change the tier enters the government's
//! agenda". `TickHub.gov_debate` is a SINGLE slot — a city debates one
//! ordinary edict at a time (04.4) — and a culture holding several resident
//! minorities could propose several tier changes in the same year. Routing
//! every one of them through that one scarce slot would either starve
//! ordinary government edicts (military, welfare, economy…) of debate time
//! or starve culture policy of it, a real conflict rather than a shortcut
//! avoided. Instead each `CultureRelation` carries its own small
//! "shadow debate" (`proposed_tier`/`debate_round`/`debate_tally`), resolved
//! with the SAME shape of math row 04 uses (`gov_position`, `stability_at`,
//! a hashed noise term) but on its own per-relation state, so a culture
//! policy question and an ordinary edict can be live at once. `family`
//! still borrows `EDICT_FAM_CITIZENSHIP`/`EDICT_FAM_FOREIGNERS` from
//! `government.rs` for the CHRONICLE text, so a "citizenship" and a
//! "culture-tier" event read as the same kind of thing to a player. A real
//! merge into one shared agenda list (so the Government window shows both
//! kinds of proposal side by side) is `Q05.3`.
//!
//! **Forward hooks, read neutral exactly as 00_INDEX requires:**
//! - "resident scholars/artisans" (relation UP term) — rows 06/07 (scholars,
//!   artisans) do not exist yet, so this term is not computed at all rather
//!   than approximated from an unrelated signal.
//! - realm cohesion (the effects table's last row) — `acceptance_cohesion_
//!   term_e` always returns `0.0`; row 09's `Realm` does not exist yet to
//!   read it.
//! - admiration / barbarian judgement — NOT a stub: `culture_ideals.rs`'s
//!   `admires_more_developed`/`is_barbarian_to` (row 03, already built and
//!   documented there as "called by nothing") are the real, live hook this
//!   row calls.
//!
//! **Dose discipline.** Population movement (persecution/diaspora, 05.3) is
//! computed but not applied at `PERSECUTION_DOSE = 0.0`; the five effects
//! (05.5 — office, scholar/artisan chance, tax, settlement, development
//! share) are each a pure `_e` function proven a no-op at its own dose
//! constant, all shipped `0.0`. Per the `is_barbarian_to`/`admires_more_
//! developed` precedent already in this file tree, wiring an effect
//! function into a real economic pass (tax collection, migration weighting)
//! needs a per-culture population/wealth attribution `hub_minorities`'s
//! plain share does not yet carry — recorded as `Q05.4`, not silently
//! skipped.
use super::*;

// ---------------------------------------------------------------------
// Tiers
// ---------------------------------------------------------------------

pub const ACCEPT_TIER_CITIZENS: u8 = 1;
pub const ACCEPT_TIER_ENFRANCHISED: u8 = 2;
pub const ACCEPT_TIER_RESIDENT: u8 = 3;
pub const ACCEPT_TIER_UNWELCOME: u8 = 4;
pub const ACCEPT_TIER_HATED: u8 = 5;

pub fn acceptance_tier_name(tier: u8) -> &'static str {
    match tier {
        ACCEPT_TIER_CITIZENS => "Citizens",
        ACCEPT_TIER_ENFRANCHISED => "Enfranchised",
        ACCEPT_TIER_RESIDENT => "Resident foreigners",
        ACCEPT_TIER_UNWELCOME => "Unwelcome",
        _ => "Hated",
    }
}

/// Below this share of a hub's population a culture is not "resident" enough
/// to earn its own sparse entry — it is simply not present in any way that
/// matters to this city yet.
const ACCEPT_MIN_MINORITY_SHARE: f32 = 0.02;

// Tier boundaries in score space, with a hysteresis band (mirrors
// `TIER_PCT_DEAD_BAND` in `houses.rs`) so a relation sitting on a boundary
// does not relabel every year.
const ACCEPT_T1_ENTER: f32 = 62.0;
const ACCEPT_T1_EXIT: f32 = 52.0;
const ACCEPT_T2_ENTER: f32 = 22.0;
const ACCEPT_T2_EXIT: f32 = 12.0;
const ACCEPT_T4_ENTER: f32 = -22.0; // crossing DOWNWARD enters tier 4
const ACCEPT_T4_EXIT: f32 = -12.0;
const ACCEPT_T5_ENTER: f32 = -62.0;
const ACCEPT_T5_EXIT: f32 = -52.0;

fn tier_for_score_plain(score: f32) -> u8 {
    if score >= ACCEPT_T1_ENTER { ACCEPT_TIER_CITIZENS }
    else if score >= ACCEPT_T2_ENTER { ACCEPT_TIER_ENFRANCHISED }
    else if score >= ACCEPT_T4_ENTER { ACCEPT_TIER_RESIDENT }
    else if score >= ACCEPT_T5_ENTER { ACCEPT_TIER_UNWELCOME }
    else { ACCEPT_TIER_HATED }
}

/// Tier from a score, hysteresis-banded against the PREVIOUS tier so a
/// relation oscillating around one boundary does not flip every year.
pub(crate) fn tier_for_score(score: f32, prev_tier: u8) -> u8 {
    match prev_tier {
        ACCEPT_TIER_CITIZENS => if score < ACCEPT_T1_EXIT { tier_for_score_plain(score) } else { ACCEPT_TIER_CITIZENS },
        ACCEPT_TIER_ENFRANCHISED => {
            if score >= ACCEPT_T1_ENTER { ACCEPT_TIER_CITIZENS }
            else if score < ACCEPT_T2_EXIT { tier_for_score_plain(score) }
            else { ACCEPT_TIER_ENFRANCHISED }
        }
        ACCEPT_TIER_RESIDENT => {
            if score >= ACCEPT_T2_ENTER { ACCEPT_TIER_ENFRANCHISED }
            else if score < ACCEPT_T4_EXIT { tier_for_score_plain(score) }
            else { ACCEPT_TIER_RESIDENT }
        }
        ACCEPT_TIER_UNWELCOME => {
            if score >= ACCEPT_T4_ENTER { ACCEPT_TIER_RESIDENT }
            else if score < ACCEPT_T5_EXIT { tier_for_score_plain(score) }
            else { ACCEPT_TIER_UNWELCOME }
        }
        ACCEPT_TIER_HATED => if score >= ACCEPT_T5_ENTER { ACCEPT_TIER_UNWELCOME } else { ACCEPT_TIER_HATED },
        _ => tier_for_score_plain(score),
    }
}

// ---------------------------------------------------------------------
// The relation record
// ---------------------------------------------------------------------

fn neg_one_i8_field() -> i8 { -1 }

/// One (city, culture) relation — sparse, lazily seeded. `-100..100` score,
/// the doc's own range; `reason` is rebuilt each yearly drift as a short
/// human phrase ("trade +12, the war of 214 -20") for the UI.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CultureRelation {
    pub culture: String,
    pub tier: u8,
    pub score: f32,
    /// Net drift applied at the last yearly pass — the UI's trend arrow.
    pub trend: f32,
    #[serde(default)]
    pub reason: String,
    /// A tier change under debate (05.2) — `-1` while no proposal is open,
    /// else the TARGET tier the proposal would move to.
    #[serde(default = "neg_one_i8_field")]
    pub proposed_tier: i8,
    #[serde(default)]
    pub debate_round: u8,
    /// `-1.0` (solid fail) .. `+1.0` (solid pass), same shape as
    /// `GovDebate.tally` (government.rs).
    #[serde(default)]
    pub debate_tally: f32,
}

// ---------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------

/// Shadow-debate round cap — a culture-tier question is a MINOR edict in
/// row 04's own vocabulary (a generation-scale civic question, not a
/// constitutional one), so it gets the same one-round-decisive shape
/// `round_cap_for(0, false)` gives an ordinary council's minor edict.
const ACCEPT_DEBATE_ROUND_CAP: u8 = 2;
const ACCEPT_DEBATE_DECISIVE: f32 = 0.30;

/// 05.3 — population movement behind expulsion/diaspora. Ships at `0.0`: a
/// persecution event still fires, is still chronicled, and the diaspora
/// DESTINATION is still computed (so the mechanism is exercised by its own
/// test at a nonzero dose), but at the shipped dose no resident is actually
/// moved and no craft tradition actually transfers — the same shape as
/// every other dosed-from-zero constant in this tree.
pub(crate) const PERSECUTION_DOSE: f32 = 0.0;

/// 05.5 effects — each a true no-op at `0.0`.
pub(crate) const ACCEPT_TAX_DOSE: f32 = 0.0;
pub(crate) const ACCEPT_SETTLE_DOSE: f32 = 0.0;
pub(crate) const ACCEPT_SCHOLAR_DOSE: f32 = 0.0;
pub(crate) const ACCEPT_OFFICE_DOSE: f32 = 0.0;
pub(crate) const ACCEPT_DEV_DOSE: f32 = 0.0;
/// Row 09 forward hook — `acceptance_cohesion_term_e` returns `0.0`
/// regardless of this dose until a real `Realm` exists to read it; the
/// constant exists so a future row 09 session has a named place to raise.
pub(crate) const ACCEPT_COHESION_DOSE: f32 = 0.0;

/// 05.4 — the culture-fondaco founding pass (a house whose culture holds
/// tier 2-3 here is chartered a fondaco). Kept separate from the pre-existing
/// `yards.rs::maybe_found_fondaco` (`YARDS_VESSELS_AND_DEPOTS_PLAN.md` W5,
/// its own zero-dose stub) — that one is a generic vessel-capacity/depot
/// mechanism with no culture-tier trigger; this one is row 05's own,
/// narrower path onto the SAME `Fondaco` struct and `CampaignSim.fondacos`
/// list. Both are dosed at zero, so a world with no fondaco founded is
/// bit-identical regardless of which plan's session ran last.
pub(crate) const FONDACO_CHARTER_DOSE: f32 = 0.0;

// ---------------------------------------------------------------------
// Bondage attitude (05.4)
// ---------------------------------------------------------------------

/// A culture's own default bondage attitude, purely from its traits:
/// `-1.0` (rejects) .. `+1.0` (fully accepts). Martial/Nomadic cultures
/// (captive-taking is part of how they make war) lean permissive; Devout/
/// Scholarly cultures lean against — a documented simplification (not every
/// devout culture historically rejected slavery; the doc's own brief names
/// only "some accept it, some don't" without a full historical table, and a
/// finer table is real future work, not built here).
pub(crate) fn bondage_attitude_for_traits(traits: &[usize]) -> f32 {
    let mut v = 0.0f32;
    if traits.contains(&3) { v += 0.30; } // Martial
    if traits.contains(&5) { v += 0.30; } // Nomadic
    if traits.contains(&4) { v -= 0.25; } // Devout
    if traits.contains(&9) { v -= 0.20; } // Scholarly
    v.clamp(-1.0, 1.0)
}

// ---------------------------------------------------------------------
// 05.5 — effects, each a pure `_e` function, dosed from zero
// ---------------------------------------------------------------------

/// Tax surcharge multiplier by tier (1.0 = no surcharge). Tier 3 small,
/// tier 4 large, per the doc's own table.
pub(crate) fn acceptance_tax_mult_e(tier: u8, dose: f32) -> f32 {
    if dose <= 0.0 { return 1.0; }
    let raw = match tier {
        ACCEPT_TIER_RESIDENT => 1.15,
        ACCEPT_TIER_UNWELCOME => 1.60,
        ACCEPT_TIER_HATED => 2.0,
        _ => 1.0,
    };
    1.0 + (raw - 1.0) * dose
}

/// Settlement (migration-in) weight multiplier by tier — 1-3 normal, 4 rare,
/// 5 none.
pub(crate) fn acceptance_settle_mult_e(tier: u8, dose: f32) -> f32 {
    if dose <= 0.0 { return 1.0; }
    let raw = match tier {
        ACCEPT_TIER_UNWELCOME => 0.20,
        ACCEPT_TIER_HATED => 0.0,
        _ => 1.0,
    };
    1.0 + (raw - 1.0) * dose
}

/// Scholar/artisan appearance-chance multiplier by tier — tier 1 full,
/// 2-3 reduced (Aristotle-the-metic, per the doc), 4-5 none. Reads neutral
/// (1.0) regardless of dose because rows 06/07 (the mechanisms that would
/// actually roll a scholar or artisan into being) do not exist yet — this
/// is the forward hook, named rather than approximated.
pub(crate) fn acceptance_scholar_mult_e(tier: u8, dose: f32) -> f32 {
    let _ = dose; // forward hook — see doc comment; not yet a real multiplier
    let _ = tier;
    1.0
}

/// Office/seat eligibility by tier — tier 1 full, tier 2 lesser offices
/// only (folded here into a single boolean; a finer full/lesser split is
/// real future work once row 04's office KINDS are read from here).
pub(crate) fn acceptance_office_allowed_e(tier: u8, dose: f32) -> bool {
    if dose <= 0.0 { return true; }
    tier <= ACCEPT_TIER_ENFRANCHISED
}

/// Development-contribution share by tier (row 03 hook) — full share in
/// tiers 1-3, none in 4-5.
pub(crate) fn acceptance_dev_share_e(tier: u8, dose: f32) -> f32 {
    if dose <= 0.0 { return 1.0; }
    let raw = if tier <= ACCEPT_TIER_RESIDENT { 1.0 } else { 0.0 };
    1.0 + (raw - 1.0) * dose
}

/// Realm-cohesion contribution (row 09 hook) — see module doc comment.
/// Always `0.0`.
pub(crate) fn acceptance_cohesion_term_e(_tier: u8, _dose: f32) -> f32 { 0.0 }

/// Diaspora population-movement fraction (05.3) — the share of a minority's
/// resident population that leaves in a persecution year, at full dose.
pub(crate) fn persecution_migration_frac_e(minority_share: f32, dose: f32) -> f32 {
    if dose <= 0.0 { return 0.0; }
    (minority_share * 0.6 * dose).clamp(0.0, minority_share)
}

// ---------------------------------------------------------------------
// Hash salts (this module's own registry — no central `living_world_salts`
// module exists yet in this codebase; every theme file names its own inline
// salts, e.g. `government.rs`'s `0xE01C`/`0xFA30`, and this follows that
// same, already-established convention).
// ---------------------------------------------------------------------
const SALT_ACCEPT_SEED: u64 = 0xACCE_5EED;
const SALT_ACCEPT_DRIFT: u64 = 0xACCE_D41F;
const SALT_ACCEPT_DEBATE: u64 = 0xACCE_DEBA;
const SALT_ACCEPT_PERSECUTE: u64 = 0xACCE_9E45;

impl CampaignSim {
    /// 05.4 — whether bondage is permitted at this city: the city's own
    /// `bondage_override` (set by a chronicled edict-like toggle) if any,
    /// else the resident majority culture's own trait-derived default.
    /// Pure, derived, reads nothing that isn't already stored.
    pub(crate) fn bondage_permitted(&self, h: usize) -> bool {
        let Some(hub) = self.hubs.get(h) else { return false };
        match hub.bondage_override {
            0 => false,
            1 => true,
            _ => {
                let culture = self.hub_culture.get(h).cloned().unwrap_or_default();
                if culture.is_empty() { return false; }
                bondage_attitude_for_traits(&self.culture_trait_ids(&culture)) >= 0.0
            }
        }
    }

    /// A city's own stance bias, in score-space (-40..40): remoteness and
    /// Insular/Xenophobic traits pull closed, Mercantile/Assimilative
    /// traits and trade dependence pull open. `comp_size` is the OTHER live
    /// hub count in this hub's own trade component, precomputed once per
    /// pass (never re-scanned per relation — see the yearly pass below).
    fn culture_stance_bias(&self, h: usize, culture: &str, comp_mates: u32) -> f32 {
        let traits = self.culture_trait_ids(culture);
        let mut bias = 0.0f32;
        if traits.contains(&2) { bias -= 10.0; }  // Insular
        if traits.contains(&13) { bias -= 16.0; } // Xenophobic
        if traits.contains(&0) { bias += 8.0; }   // Mercantile
        if traits.contains(&7) { bias += 12.0; }  // Assimilative
        // Remoteness: a city with few live trade-component mates is
        // conservative by circumstance, not just by trait.
        let remote = (8u32.saturating_sub(comp_mates.min(8))) as f32;
        bias -= remote * 1.2;
        // Trade dependence: a hub that trades heavily leans open (foreigners
        // bring income), the doc's own stated reason.
        let trade_dep = (self.hubs[h].trade_last_year / 5000.0).min(2.0);
        bias += trade_dep * 6.0;
        bias.clamp(-40.0, 40.0)
    }

    /// Lazy seed (00_INDEX "Lazy seeding") — called at the top of the yearly
    /// pass for every settled hub. A hub whose `culture_relations` is
    /// already non-empty is left untouched (an old save re-seeds ONCE on the
    /// first year the pass runs after this row lands; a hub that later loses
    /// every minority keeps its last known relations rather than forgetting
    /// them, which is the honest reading of a "relation", not a live
    /// residency flag).
    pub(crate) fn ensure_culture_relations(&mut self, h: usize, comp_mates: u32) {
        if !self.hubs[h].culture_relations.is_empty() { return; }
        let majority = self.hub_culture.get(h).cloned().unwrap_or_default();
        if majority.is_empty() { return; }
        let mut seeded = Vec::new();
        // The majority culture always starts as Citizens in its own city.
        let maj_bias = self.culture_stance_bias(h, &majority, comp_mates);
        seeded.push(CultureRelation {
            culture: majority.clone(),
            tier: ACCEPT_TIER_CITIZENS,
            score: (80.0 + maj_bias * 0.25).clamp(-100.0, 100.0),
            trend: 0.0, reason: "native population".into(),
            proposed_tier: -1, debate_round: 0, debate_tally: 0.0,
        });
        let mins: Vec<(String, f32)> = self.hub_minorities.get(h).cloned().unwrap_or_default();
        for (culture, share) in mins {
            if share < ACCEPT_MIN_MINORITY_SHARE || culture == majority || culture.is_empty() { continue; }
            let bias = self.culture_stance_bias(h, &culture, comp_mates);
            let roll = (hash01(self.seed, h as u64, fnv_mix(SALT_ACCEPT_SEED, &culture)) - 0.5) * 10.0;
            let base = 15.0 + bias * 1.1 + roll; // an unfamiliar minority starts near ENFRANCHISED/RESIDENT, stance-shifted
            seeded.push(CultureRelation {
                culture: culture.clone(),
                tier: tier_for_score_plain(base),
                score: base.clamp(-100.0, 100.0),
                trend: 0.0, reason: "newly present".into(),
                proposed_tier: -1, debate_round: 0, debate_tally: 0.0,
            });
        }
        self.hubs[h].culture_relations = seeded;
    }

    /// 05.2 — one yearly drift for one relation. Returns `(delta, reason)`;
    /// `feud_pressure`/`comp_mates` are precomputed by the caller so this
    /// stays O(1) per relation rather than re-scanning the world.
    pub(crate) fn culture_relation_drift(
        &self, h: usize, culture: &str, share: f32, comp_mates: u32, feud_pressure: f32,
    ) -> (f32, String) {
        let majority = self.hub_culture.get(h).cloned().unwrap_or_default();
        let hub = &self.hubs[h];
        let is_majority = culture == majority;
        let mut terms: Vec<(&'static str, f32)> = Vec::new();

        // Stance mean-reversion: nudge toward the city's own openness bias.
        let bias = self.culture_stance_bias(h, culture, comp_mates);
        terms.push(("stance", bias * 0.08));

        if !is_majority {
            // Resident merchants / trade tie — the minority's own share is
            // the cheapest real proxy this pass has for "how present is
            // this culture's trade here" without a per-culture trade ledger.
            let trade_term = share.min(0.6) * 14.0 * (1.0 + (hub.trade_last_year / 8000.0).min(1.5));
            terms.push(("trade", trade_term));

            // Related culture family / same language kit.
            if let (Some(mk), Some(ck)) = (crate::sim::cultures::kit_of_people(&majority), crate::sim::cultures::kit_of_people(culture)) {
                if mk == ck { terms.push(("kinship", 6.0)); }
            }

            // Admiration of a more-developed resident culture (row 03 hook,
            // real — not a stub).
            if self.admires_more_developed(&majority, culture) { terms.push(("admiration", 5.0)); }
            // Barbarian judgement (row 03 hook, real).
            if self.is_barbarian_to(&majority, culture) { terms.push(("judged barbarous", -10.0)); }

            // Dominant-foreigner resentment.
            if share > 0.35 { terms.push(("dominance", -(share - 0.35) * 22.0)); }

            // War: this hub is at war with the culture's own homeland (the
            // city whose majority culture is `culture` and that this hub's
            // `war_with` names).
            if hub.war_with >= 0 {
                if let Some(wc) = self.hub_culture.get(hub.war_with as usize) {
                    if wc == culture { terms.push(("war", -22.0)); }
                }
            }
            // Feuds — houses of this culture feuding in this city.
            if feud_pressure > 0.0 { terms.push(("feuds", -feud_pressure)); }

            // Famine/plague blamed on outsiders — only when the CITY's own
            // majority leans closed (Insular/Xenophobic); an open city does
            // not scapegoat.
            if hub.starving > 0.3 {
                let maj_traits = self.culture_trait_ids(&majority);
                if maj_traits.contains(&2) || maj_traits.contains(&13) {
                    terms.push(("famine blamed on outsiders", -6.0));
                }
            }
        }

        let delta: f32 = terms.iter().map(|(_, v)| *v).sum::<f32>().clamp(-20.0, 20.0);
        // Reason: the two largest-magnitude non-zero terms.
        let mut sorted = terms;
        sorted.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        let reason = sorted.iter().take(2).filter(|(_, v)| v.abs() > 0.05)
            .map(|(k, v)| format!("{k} {v:+.0}"))
            .collect::<Vec<_>>().join(", ");
        (delta, if reason.is_empty() { "quiet".into() } else { reason })
    }

    /// 05.2 — one shadow-debate round for a proposed tier change (see module
    /// doc comment for why this is self-contained rather than sharing row
    /// 04's `GovDebate` slot). Mirrors `run_debate_round`'s own math:
    /// `gov_position` sets the government's lean, `stability_at` scales how
    /// decisively it can act, noised by a hashed roll.
    fn run_acceptance_debate_round(&mut self, h: usize, ri: usize) {
        let gov_lean = self.hubs[h].gov_position;
        let stab = self.stability_at(h);
        let round = self.hubs[h].culture_relations[ri].debate_round;
        let noise = (hash01(self.seed, self.tick as u64 ^ (ri as u64), (h as u64) ^ SALT_ACCEPT_DEBATE ^ round as u64) * 2.0 - 1.0) * 0.4;
        let lean = (gov_lean * 0.3 + stab * 0.4 + noise).clamp(-1.0, 1.0);
        let rel = &mut self.hubs[h].culture_relations[ri];
        rel.debate_round += 1;
        rel.debate_tally = (rel.debate_tally * 0.5 + lean * 0.5).clamp(-1.0, 1.0);
        if rel.debate_tally.abs() >= ACCEPT_DEBATE_DECISIVE || rel.debate_round >= ACCEPT_DEBATE_ROUND_CAP {
            let passed = rel.debate_tally >= ACCEPT_DEBATE_DECISIVE;
            let target = rel.proposed_tier as u8;
            let (culture, old_tier) = (rel.culture.clone(), rel.tier);
            rel.proposed_tier = -1;
            rel.debate_round = 0;
            rel.debate_tally = 0.0;
            if passed {
                self.hubs[h].culture_relations[ri].tier = target;
                let raising = target < old_tier;
                let family = if raising { EDICT_FAM_CITIZENSHIP } else { EDICT_FAM_FOREIGNERS };
                let text = format!(
                    "{} vote to {} the {culture} of {} from {} to {}",
                    self.hubs[h].name,
                    if raising { "raise" } else { "lower" },
                    self.hubs[h].name,
                    acceptance_tier_name(old_tier),
                    acceptance_tier_name(target),
                );
                self.chronicle_acceptance(h, family, &text);
            }
        }
    }

    fn chronicle_acceptance(&mut self, h: usize, family: u8, text: &str) {
        let tick = self.tick;
        let kind = if family == EDICT_FAM_FOREIGNERS { "acceptance_restricted" } else { "acceptance_changed" };
        self.journal.push(JournalEntry { tick, kind: kind.into(), hub: h as i32, good: -1, value: 0.0, text: text.into() });
    }

    /// 05.3 — a persecution event at tier 5 (or a severe single-year drop).
    /// Always chronicled; the diaspora DESTINATION is always computed (so
    /// the mechanism is real and testable), but at `PERSECUTION_DOSE = 0.0`
    /// no population or tradition actually moves.
    fn maybe_persecute(&mut self, h: usize, ri: usize, comp_mates_of: &std::collections::HashMap<u32, u32>) {
        let (culture, share) = {
            let rel = &self.hubs[h].culture_relations[ri];
            (rel.culture.clone(), 0.0f32) // share filled below from hub_minorities
        };
        let share = self.hub_minorities.get(h)
            .and_then(|m| m.iter().find(|(c, _)| *c == culture).map(|(_, s)| *s))
            .unwrap_or(share);
        if share <= 0.0 { return; }
        let roll = hash01(self.seed, self.tick as u64, (h as u64) ^ SALT_ACCEPT_PERSECUTE ^ fnv_mix(0, &culture));
        // A modest yearly chance while Hated — not a certainty every year,
        // consistent with "massacres are rare" from the doc's own decisions.
        const PERSECUTE_CHANCE: f32 = 0.12;
        if roll >= PERSECUTE_CHANCE { return; }
        let massacre = roll < PERSECUTE_CHANCE * 0.08; // a small slice of persecution years turn violent
        let kind = if massacre { "the {culture} of {city} are massacred" } else { "the {culture} of {city} are expelled" };
        let text = kind.replace("{culture}", &culture).replace("{city}", &self.hubs[h].name);
        self.chronicle_acceptance(h, EDICT_FAM_FOREIGNERS, &text);

        // Diaspora destination: the highest-scoring OTHER hub for this
        // culture within the same trade component (the "most welcoming
        // reachable city" the doc names), else none.
        let comp = self.hubs[h].component;
        let mut best: Option<(usize, f32)> = None;
        for (oh, ohub) in self.hubs.iter().enumerate() {
            if oh == h || ohub.is_estate || ohub.abandoned { continue; }
            if ohub.component != comp { continue; }
            if let Some(rel) = ohub.culture_relations.iter().find(|r| r.culture == culture) {
                if best.map(|(_, s)| rel.score > s).unwrap_or(true) { best = Some((oh, rel.score)); }
            }
        }
        let _ = comp_mates_of; // reserved for a future reach-bounded search (Q05.5)
        let moved_frac = persecution_migration_frac_e(share, PERSECUTION_DOSE);
        if moved_frac <= 0.0 { return; }
        let Some((dest, _)) = best else { return };
        let moved_pop = self.hubs[h].population * share * moved_frac;
        self.hubs[h].population = (self.hubs[h].population - moved_pop).max(0.0);
        self.hubs[dest].population += moved_pop;
        // Carry a slice of craft tradition with them (the Huguenot effect).
        let ng = self.goods.len();
        if self.hubs[h].tradition.len() == ng && self.hubs[dest].tradition.len() == ng {
            for g in 0..ng {
                let carry = self.hubs[h].tradition[g] * moved_frac * 0.5;
                if carry <= 0.0 { continue; }
                self.hubs[h].tradition[g] -= carry;
                self.hubs[dest].tradition[g] += carry;
            }
        }
    }

    /// 05.4 — a house whose culture holds tier 2-3 in a city it does not
    /// already occupy a fondaco in may be chartered one. See the module's
    /// own doc comment for why this is dosed at zero and kept separate from
    /// `yards.rs::maybe_found_fondaco`.
    pub(crate) fn maybe_charter_culture_fondacos(&mut self) {
        if FONDACO_CHARTER_DOSE <= 0.0 { return; }
        let occupied: std::collections::HashSet<u32> = self.fondacos.iter().map(|f| f.occupant).collect();
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            for ri in 0..self.hubs[h].culture_relations.len() {
                let (culture, tier) = {
                    let r = &self.hubs[h].culture_relations[ri];
                    (r.culture.clone(), r.tier)
                };
                if tier != ACCEPT_TIER_ENFRANCHISED && tier != ACCEPT_TIER_RESIDENT { continue; }
                // Houses carry no `culture` field of their own — a house's
                // culture is its home hub's majority culture, the same
                // reading `culture_trait_ids`'s own callers use elsewhere.
                let Some(hi) = self.houses.iter().position(|ho| {
                    ho.is_merchant() && ho.hub as usize != h && !occupied.contains(&(ho.hub))
                        && self.hub_culture.get(ho.hub as usize).map(|c| c.as_str()) == Some(culture.as_str())
                }) else { continue };
                let cut = 0.10;
                self.fondacos.push(Fondaco { hub: h as u32, occupant: hi as u32, cut, founded_tick: self.tick, closed: false });
                self.chronicle_acceptance(h, EDICT_FAM_CITIZENSHIP,
                    &format!("{} charters a fondaco for the {culture} house {}", self.hubs[h].name, self.houses[hi].name));
                break; // one new fondaco per city per call, per doc's "a real, if rare, political act"
            }
        }
    }

    /// 05.2/05.3 — the yearly cadence hook. `O(hubs × resident cultures)`,
    /// bounded (a hub carries a handful of relations at most), well inside
    /// this layer's <100 ms/year budget.
    pub(crate) fn culture_acceptance_yearly_pass(&mut self, _yr: u32) {
        // Precompute trade-component mate counts ONCE (never per relation —
        // see `culture_stance_bias`'s own doc note).
        let mut comp_mates_of: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for hub in self.hubs.iter() {
            if hub.is_estate || hub.abandoned { continue; }
            *comp_mates_of.entry(hub.component).or_insert(0) += 1;
        }
        // Precompute per-(hub, culture) feud pressure ONCE from the bounded
        // world feud list, rather than rescanning it per relation.
        let mut feud_pressure: std::collections::HashMap<(usize, String), f32> = std::collections::HashMap::new();
        for feud in self.feuds.iter() {
            if feud.outcome != FEUD_RUNNING || feud.hub < 0 { continue; }
            let h = feud.hub as usize;
            for hi in [feud.a as usize, feud.b as usize] {
                if let Some(house) = self.houses.get(hi) {
                    if house.hub as usize == h { continue; } // a local house feuding at home isn't a foreign-culture signal
                    // Houses carry no `culture` field of their own — read it
                    // off the house's home hub's majority culture.
                    let Some(hc) = self.hub_culture.get(house.hub as usize) else { continue };
                    let e = feud_pressure.entry((h, hc.clone())).or_insert(0.0);
                    *e = (*e + 8.0).min(24.0);
                }
            }
        }

        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let mates = comp_mates_of.get(&self.hubs[h].component).copied().unwrap_or(0).saturating_sub(1);
            self.ensure_culture_relations(h, mates);

            let n = self.hubs[h].culture_relations.len();
            for ri in 0..n {
                // Continue an already-open shadow debate first.
                if self.hubs[h].culture_relations[ri].proposed_tier >= 0 {
                    self.run_acceptance_debate_round(h, ri);
                    continue;
                }
                let culture = self.hubs[h].culture_relations[ri].culture.clone();
                let share = self.hub_minorities.get(h)
                    .and_then(|m| m.iter().find(|(c, _)| *c == culture).map(|(_, s)| *s))
                    .unwrap_or(1.0); // the majority entry has no minorities row — treat as fully present
                let fp = feud_pressure.get(&(h, culture.clone())).copied().unwrap_or(0.0);
                let (delta, reason) = self.culture_relation_drift(h, &culture, share, mates, fp);

                let rel = &mut self.hubs[h].culture_relations[ri];
                rel.score = (rel.score + delta).clamp(-100.0, 100.0);
                rel.trend = delta;
                rel.reason = reason;
                let new_tier = tier_for_score(rel.score, rel.tier);
                if new_tier != rel.tier {
                    // A crossing opens a shadow debate rather than applying
                    // immediately (05.2's "score proposes, edict disposes").
                    rel.proposed_tier = new_tier as i8;
                    rel.debate_round = 0;
                    rel.debate_tally = 0.0;
                }
            }
            for ri in 0..n {
                if self.hubs[h].culture_relations[ri].tier == ACCEPT_TIER_HATED {
                    self.maybe_persecute(h, ri, &comp_mates_of);
                }
            }
        }
    }
}

/// A small, stable per-string mix folded into a `hash01` salt so two
/// different culture names never collide on the same numeric salt.
fn fnv_mix(salt: u64, s: &str) -> u64 {
    let mut x = salt ^ 0xcbf29ce484222325u64;
    for b in s.bytes() { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); }
    x
}
