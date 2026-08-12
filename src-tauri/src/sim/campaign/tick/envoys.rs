//! envoys — ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.9 (D7/D8), amended by A4-A6.
//!
//! `estate_resale_pass` (houses.rs) already resolves LOCAL acquisition — a
//! resident house or bank buying a distressed holding in its own city —
//! instantly, one sale a month. That stays untouched (it is the path the
//! dynamics gate is calibrated against). This file adds the CROSS-CITY case,
//! where distance itself is the mechanic (§8.4 of the plan): an envoy takes
//! real travel time, during which a nearer, already-resident rival can close
//! first (`status` 5, pre-empted) — the argument for maintaining a physical
//! presence (an office or bailo) rather than just being rich.
//!
//! Two passes, run monthly right after `estate_resale_pass` so both engines
//! see the same "who's already for sale" picture:
//!   `envoy_dispatch_pass` — at most ONE new envoy a month (the same "find
//!   ONE" discipline `estate_resale_pass` already uses), so this stays a rare,
//!   watched event rather than a second acquisition engine running beside the
//!   local one.
//!   `envoy_travel_pass` — advances the clock and resolves any arrival.
//!
//! Amendments wired in at resolution:
//!   A4 — a target city's `LAW_FOREIGN_BAR` (enacted only occasionally, at a
//!        fresh council capture — mod.rs's "5) Payoff" step) blocks outright
//!        unless the house already holds an office or bailo there.
//!   A5 — payment clears at full price through a bank whose branch network
//!        spans BOTH the house's home city and the target city (the same
//!        `branches` list B4's bills-of-exchange income already reads);
//!        otherwise it falls back to slower, costlier specie
//!        (`ENVOY_SPECIE_PENALTY`).
//!   A6 (partial) — a REFUSED outcome is not modelled here as a bank
//!        lend-into-arrears conversion; that needs a loan tagged to a specific
//!        estate, which `Loan` does not yet carry, and retrofitting it touches
//!        `bank_pass`'s own default handling — the single most gate-sensitive
//!        loop in the tick (see `estate_condition_pass`'s own postmortem on
//!        this file's sibling, 4.7). Left unbuilt and flagged rather than
//!        risked; the Fugger case belongs with a real `Loan.estate_hub` field.
use super::*;

/// At most one active envoy per house at a time, and a hard cap on how many
/// can be in flight world-wide — this is a rare event, not a second market.
const MAX_ACTIVE_ENVOYS: usize = 2;
/// A house needs real surplus wealth before it looks abroad — well above the
/// local resale market's own `RESALE_DISTRESS_WEALTH` floor, since it is about
/// to pay full price plus possibly the specie penalty. Deliberately high: this
/// is meant to gate only the established elite, not an ordinary prospering
/// house (§4.9's own gate says "expected small, since it gates transactions
/// rather than flows" — a mechanic most houses never trigger is what keeps it
/// small). Measured against `econ_inheritance_rules_fragment_differently`
/// (economy_validation.rs): the mechanism's footprint moves that gate's own
/// partible-vs-primogeniture margin roughly IN PROPORTION to its trigger
/// rate — cutting dispatch chance/candidates/min-wealth by ~3x each cut the
/// margin's overshoot by a similar factor (from partible reading 16.6% ABOVE
/// primogeniture, the wrong side, down to 2.1% — see the reverted intermediate
/// values in git history). This is genuine dose-dependence, not the discrete
/// branching-order sensitivity `estate_condition_pass` hit in 4.7 (there,
/// changing magnitude left the dynamics test's OWN richest-house figure
/// bit-for-bit identical across two different tunings — the signature of a
/// hash01 branch flipping, not a proportional response). Tuned to the point
/// this gate passes with a real, if thin, margin.
const ENVOY_MIN_WEALTH: f32 = 40_000.0;
/// Only the single wealthiest candidate is even considered each month — this
/// is a rare event, not a scan of the whole elite.
const ENVOY_CANDIDATES_PER_MONTH: usize = 1;
/// Chance an eligible, idle candidate house dispatches this month.
const ENVOY_DISPATCH_CHANCE: f32 = 0.006;
/// No envoy before this many years — an unsettled young house doesn't reach
/// abroad (mirrors the existing "guilds cluster from year 5, houses from year
/// 10" emergence convention).
const ENVOY_MIN_YEAR: u32 = 25;
/// A house won't set out unless it can plausibly afford the holding once it
/// arrives (price can drift with condition/tier in the meantime).
const ENVOY_WEALTH_MULT: f32 = 1.8;
/// World-distance units (cell-equivalents) covered per tick — tuned so a
/// journey across a modest map takes on the order of a season, not a day.
const ENVOY_SPEED_CELLS_PER_TICK: f32 = 0.6;
const ENVOY_MIN_TRAVEL_TICKS: u32 = 20;
/// Base chance the negotiation succeeds at all (agreed OR partial) before
/// presence/feud/war adjustments.
const ENVOY_BASE_SUCCESS: f32 = 0.55;
const ENVOY_PRESENCE_BONUS: f32 = 0.20;
const ENVOY_FEUD_PENALTY: f32 = 0.20;
const ENVOY_WAR_PENALTY: f32 = 0.25;
/// A5 · the specie fallback's cost multiplier when no bank branch network
/// spans both ends.
const ENVOY_SPECIE_PENALTY: f32 = 1.35;
/// A partial outcome buys this fraction rather than full ownership.
const ENVOY_PARTIAL_FRAC: f32 = 0.35;

impl CampaignSim {
    /// Wrap-aware straight-line distance in cells — the same calculation
    /// `hub_distance_cells` (realms.rs) makes, kept as its own small copy here
    /// since that one is private to its module (realms.rs's own comment notes
    /// this is already a duplicated pattern, not a new one).
    fn envoy_distance_cells(&self, a: usize, b: usize) -> f32 {
        let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
        if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
        let dy = self.hubs[a].y - self.hubs[b].y;
        (dx * dx + dy * dy).sqrt()
    }

    pub(crate) fn envoy_dispatch_pass(&mut self) {
        let tick = self.tick;
        if tick < ENVOY_MIN_YEAR * 365 { return; }
        let n = self.hubs.len();
        if self.envoys.iter().filter(|e| e.status == 0).count() >= MAX_ACTIVE_ENVOYS { return; }
        let busy: std::collections::HashSet<u32> = self.envoys.iter()
            .filter(|e| e.status == 0).map(|e| e.house).collect();
        let mut candidates: Vec<usize> = (0..self.houses.len())
            .filter(|&hi| self.houses[hi].is_merchant() && !self.houses[hi].is_guild
                && self.houses[hi].wealth > ENVOY_MIN_WEALTH
                && !busy.contains(&(hi as u32)))
            .collect();
        if candidates.is_empty() { return; }
        candidates.sort_by(|&a, &b|
            self.houses[b].wealth.partial_cmp(&self.houses[a].wealth).unwrap_or(std::cmp::Ordering::Equal));
        for &hi in candidates.iter().take(ENVOY_CANDIDATES_PER_MONTH) {
            if hash01(self.seed, tick as u64 ^ 0x3E4A0, hi as u64) > ENVOY_DISPATCH_CHANCE { continue; }
            let home = self.houses[hi].hub as usize;
            if home >= n { continue; }
            // Target: the NEAREST reachable for-sale estate outside the house's
            // own city — mirrors `estate_resale_pass`'s own availability test
            // (a distressed private owner, or a civic holding below the
            // treasury floor) rather than a second definition of "for sale".
            let mut best: Option<(usize, f32)> = None;
            for ei in 0..n {
                let e = &self.hubs[ei];
                if !e.is_estate || e.estate_tier == 0 || e.owner_house == hi as i32 { continue; }
                let parent = e.parent;
                if parent < 0 { continue; }
                let parent = parent as usize;
                if parent == home || parent >= n || !self.hubs_within_war_reach(home, parent) { continue; }
                let for_sale = if e.owner_house >= 0 {
                    let oh = e.owner_house as usize;
                    oh < self.houses.len() && !self.houses[oh].defunct
                        && self.houses[oh].wealth < RESALE_DISTRESS_WEALTH
                } else {
                    self.hubs[parent].treasury < CIVIC_SALE_TREASURY_FLOOR
                };
                if !for_sale { continue; }
                let price = self.estate_market_value(ei);
                if price < 0.5 || self.houses[hi].wealth < price * ENVOY_WEALTH_MULT { continue; }
                let d = self.envoy_distance_cells(home, parent);
                if best.map_or(true, |(_, bd)| d < bd) { best = Some((ei, d)); }
            }
            let Some((ei, dist)) = best else { continue };
            let parent = self.hubs[ei].parent;
            let travel = ((dist / ENVOY_SPEED_CELLS_PER_TICK).ceil() as u32).max(ENVOY_MIN_TRAVEL_TICKS);
            self.envoys.push(Envoy {
                house: hi as u32, origin_hub: home as u32, target_estate: ei as u32,
                dispatched_tick: tick, arrive_tick: tick + travel, rounds_done: 0, status: 0,
            });
            let (hn, en) = (self.houses[hi].name.clone(), self.hubs[ei].name.clone());
            self.houses[hi].events.push(HouseEvent { tick, kind: "envoy".into(),
                text: format!("dispatches an envoy abroad to treat for {}", en) });
            self.journal.push(JournalEntry { tick, kind: "envoy".into(), hub: parent, good: -1,
                value: 0.0, text: format!("An envoy from {} arrives to treat for a holding here", hn) });
            return; // one dispatch per month
        }
    }

    pub(crate) fn envoy_travel_pass(&mut self) {
        let tick = self.tick;
        let arrived: Vec<usize> = self.envoys.iter().enumerate()
            .filter(|(_, e)| e.status == 0 && tick >= e.arrive_tick)
            .map(|(i, _)| i).collect();
        for idx in arrived { self.resolve_envoy(idx); }
        // Keep a resolved envoy around for ~a month (the House Dossier's own
        // reading window) so its outcome is visible, then drop it.
        self.envoys.retain(|e| e.status == 0 || tick.saturating_sub(e.arrive_tick) < 30);
    }

    fn resolve_envoy(&mut self, idx: usize) {
        let tick = self.tick;
        let (hi, ei) = (self.envoys[idx].house as usize, self.envoys[idx].target_estate as usize);
        if hi >= self.houses.len() || ei >= self.hubs.len()
            || !self.houses[hi].is_merchant() || !self.hubs[ei].is_estate {
            self.envoys[idx].status = 5;
            return;
        }
        let parent = self.hubs[ei].parent;
        // PRE-EMPTION: sold (to anyone) or no longer distressed while travelling.
        let still_available = self.hubs[ei].owner_house != hi as i32 && if self.hubs[ei].owner_house >= 0 {
            let oh = self.hubs[ei].owner_house as usize;
            oh < self.houses.len() && !self.houses[oh].defunct
                && self.houses[oh].wealth < RESALE_DISTRESS_WEALTH
        } else {
            parent >= 0 && (parent as usize) < self.hubs.len()
                && self.hubs[parent as usize].treasury < CIVIC_SALE_TREASURY_FLOOR
        };
        if !still_available {
            self.envoys[idx].status = 5;
            let en = self.hubs[ei].name.clone();
            self.houses[hi].events.push(HouseEvent { tick, kind: "envoy".into(),
                text: format!("'s envoy to {} arrives too late — the holding is already spoken for", en) });
            return;
        }
        let target_city = parent.max(0) as usize;
        // A4 · a foreign-ownership bar blocks outright unless the house already
        // has presence in the target city.
        let barred = target_city < self.hubs.len()
            && self.hubs[target_city].laws.iter().any(|l| l.kind == LAW_FOREIGN_BAR)
            && !self.houses[hi].offices.contains(&(target_city as u32))
            && !self.houses[hi].bailos.contains(&(target_city as u32));
        if barred {
            self.envoys[idx].status = 4;
            let cn = self.hubs[target_city].name.clone();
            self.houses[hi].events.push(HouseEvent { tick, kind: "envoy".into(),
                text: format!("'s envoy is turned away at {} — the city bars foreign ownership", cn) });
            return;
        }
        let mut standing = ENVOY_BASE_SUCCESS;
        if self.houses[hi].offices.contains(&(target_city as u32))
            || self.houses[hi].bailos.contains(&(target_city as u32)) { standing += ENVOY_PRESENCE_BONUS; }
        let rival_resident = self.hubs.iter().enumerate().any(|(oe, oh)|
            oe != ei && oh.is_estate && oh.parent >= 0 && oh.parent as usize == target_city
            && oh.owner_house >= 0 && self.houses[hi].rivals.contains(&(oh.owner_house as usize)));
        if rival_resident { standing -= ENVOY_FEUD_PENALTY; }
        if self.hubs[target_city].war_with >= 0 { standing -= ENVOY_WAR_PENALTY; }
        standing = standing.clamp(0.05, 0.95);

        let roll = hash01(self.seed, tick as u64 ^ 0x7E407, (ei as u64) ^ ((hi as u64) << 20));
        let price = self.estate_market_value(ei);
        // A5 · a shared bank branch network clears the deal at full price;
        // otherwise the specie fallback costs more.
        let home = self.envoys[idx].origin_hub as usize;
        let has_bill_route = self.banks.iter().any(|b| !b.defunct
            && b.branches.contains(&(home as u32)) && b.branches.contains(&(target_city as u32)));
        let cost = price * if has_bill_route { 1.0 } else { ENVOY_SPECIE_PENALTY };
        self.envoys[idx].rounds_done = 1;

        if roll < standing * 0.5 {
            // AGREED — full purchase, same money shape as estate_resale_pass'
            // BUYER 1 (a single owner-to-owner transfer, no manufactured value).
            if self.houses[hi].wealth < cost { self.envoys[idx].status = 4; return; }
            self.houses[hi].wealth -= cost;
            let seller = self.hubs[ei].owner_house;
            if seller >= 0 && (seller as usize) < self.houses.len() { self.houses[seller as usize].wealth += cost; }
            else if parent >= 0 { self.hubs[parent as usize].treasury += cost; }
            self.hubs[ei].owner_house = hi as i32;
            self.hubs[ei].shares.clear();
            self.envoys[idx].status = 2;
            let (hn, en) = (self.houses[hi].name.clone(), self.hubs[ei].name.clone());
            self.houses[hi].events.push(HouseEvent { tick, kind: "acquire".into(),
                text: format!("'s envoy secures {} for {:.0}", en, cost) });
            self.journal.push(JournalEntry { tick, kind: "estate".into(), hub: parent, good: -1,
                value: cost, text: format!("{} treats abroad and secures {}", hn, en) });
        } else if roll < standing {
            // PARTIAL — a minority share; ownership stays where it was. Dilutes
            // every existing row proportionally, the same conservation the
            // share table's other writers already use (D1's own invariant:
            // fractions always sum to 1).
            let frac = ENVOY_PARTIAL_FRAC;
            let pcost = cost * frac;
            if self.houses[hi].wealth < pcost { self.envoys[idx].status = 4; return; }
            self.houses[hi].wealth -= pcost;
            let seller = self.hubs[ei].owner_house;
            if seller >= 0 && (seller as usize) < self.houses.len() { self.houses[seller as usize].wealth += pcost; }
            else if parent >= 0 { self.hubs[parent as usize].treasury += pcost; }
            if self.hubs[ei].shares.is_empty() && seller >= 0 {
                self.hubs[ei].shares.push(Share {
                    holder_kind: 1, holder: seller as u32, frac: 1.0, payout: 1,
                    acquired_tick: tick, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
                });
            }
            for sh in self.hubs[ei].shares.iter_mut() { sh.frac *= 1.0 - frac; }
            self.hubs[ei].shares.push(Share {
                holder_kind: 1, holder: hi as u32, frac, payout: 1,
                acquired_tick: tick, paid: pcost, instrument: 0, term_years: 0, neglect_years: 0,
            });
            self.envoys[idx].status = 3;
            let (hn, en) = (self.houses[hi].name.clone(), self.hubs[ei].name.clone());
            self.houses[hi].events.push(HouseEvent { tick, kind: "acquire".into(),
                text: format!("'s envoy wins a {:.0}% share of {} for {:.0}", frac * 100.0, en, pcost) });
            self.journal.push(JournalEntry { tick, kind: "estate".into(), hub: parent, good: -1,
                value: pcost, text: format!("{} buys into {} from afar", hn, en) });
        } else {
            // REFUSED.
            self.envoys[idx].status = 4;
            let en = self.hubs[ei].name.clone();
            self.houses[hi].events.push(HouseEvent { tick, kind: "envoy".into(),
                text: format!("'s envoy is refused at {}", en) });
        }
    }
}
