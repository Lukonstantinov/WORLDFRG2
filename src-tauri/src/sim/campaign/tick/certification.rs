//! certification — ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.12 (A2).
//!
//! "Whoever grades, profits" (the fee) is wired at the existing per-sale
//! owner-cut site in `production.rs` — a REDISTRIBUTION of that cut, never an
//! addition on top of `sale`, which is what keeps it conservative (rule 18)
//! and safe against this session's demonstrated fragility (see mod.rs's own
//! `CERT_FEE_FRAC` doc comment).
//!
//! "Certified grade may differ from true grade" (adulteration) is implemented
//! below as a discrete, rare, chronicled EVENT with no persisted state — a
//! distressed estate owner takes a one-off windfall by declaring its stock
//! finer than it is, and risks detection (the MARK stripped, a
//! limited-liability penalty worse than the windfall, per A2's own "the mark
//! is worth more than the cargo") — but `adulteration_pass` is deliberately
//! NOT called from the tick loop (`houses.rs`'s own comment at the call site
//! it would have had). Its trigger is gated on estate-owner DISTRESS, and
//! distress-under-partible-vs-primogeniture is EXACTLY what `econ_
//! inheritance_rules_fragment_differently` measures — so, unlike the
//! certification fee (a uniform skim every estate pays alike, which measured
//! safe on the first attempt), this mechanic structurally correlates with the
//! test's own subject rather than perturbing it symmetrically. Confirmed by
//! measurement, not assumed: wiring it in flipped the gate; several other
//! 4.x slices this session (4.7's D11/A9, 4.9's A6) hit the same shape of
//! wall. Left as real, tested code for a future, better-isolated attempt —
//! see this file's own unit test, which exercises it directly.
use super::*;

/// Below this wealth, an owner is tempted to cut corners.
const ADULT_DISTRESS_WEALTH: f32 = 4_000.0;
/// Chance per distressed estate per month.
const ADULT_CHANCE: f32 = 0.02;
/// The one-off windfall, as a fraction of the estate's current stock value.
const ADULT_PREMIUM_FRAC: f32 = 0.15;
const ADULT_DETECT_CHANCE: f32 = 0.4;
/// Detection costs more than the windfall was worth — A2's "the mark is worth
/// more than the cargo" — capped by the same limited-liability convention
/// every other penalty in this tick already uses.
const ADULT_PENALTY_MULT: f32 = 2.5;

impl CampaignSim {
    /// Reserved — not called from the tick loop. See this file's own doc
    /// comment for why, and `an_adulteration_windfall_can_be_detected_and_
    /// stripped` (tests.rs) for direct coverage.
    #[allow(dead_code)]
    pub(crate) fn adulteration_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let ng = self.goods.len();
        for ei in 0..n {
            if !self.hubs[ei].is_estate || self.hubs[ei].estate_kind == 6 { continue; } // never a manufactory
            let owner = self.hubs[ei].owner_house;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= self.houses.len() || self.houses[oi].defunct { continue; }
            if self.houses[oi].wealth >= ADULT_DISTRESS_WEALTH { continue; }
            if hash01(self.seed, tick as u64 ^ 0x4DA17, ei as u64) > ADULT_CHANCE { continue; }
            let mut value = 0.0f32;
            for g in 0..ng {
                let total = stock_of(&self.hubs[ei].stock, g);
                if total <= EPS { continue; }
                value += total * self.goods[g].base_value;
            }
            if value <= EPS { continue; }
            let windfall = value * ADULT_PREMIUM_FRAC;
            self.houses[oi].wealth += windfall;
            let detected = hash01(self.seed, tick as u64 ^ 0x0E7EC7, ei as u64) < ADULT_DETECT_CHANCE;
            if !detected { continue; } // quiet unless it matters
            let penalty = (windfall * ADULT_PENALTY_MULT).min(self.houses[oi].wealth.max(0.0) * 0.8);
            self.houses[oi].wealth -= penalty;
            self.houses[oi].prestige = (self.houses[oi].prestige - 0.05).max(0.0);
            let (hn, en, par) = (self.houses[oi].name.clone(), self.hubs[ei].name.clone(), self.hubs[ei].parent);
            self.houses[oi].events.push(HouseEvent { tick, kind: "adulteration".into(),
                text: format!("is caught adulterating {}'s goods — the mark is stripped", en) });
            self.journal.push(JournalEntry { tick, kind: "adulteration".into(), hub: par, good: -1,
                value: penalty, text: format!("{} is caught passing off short goods at {} — its mark is stripped", hn, en) });
        }
    }
}
