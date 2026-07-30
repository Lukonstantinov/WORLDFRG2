//! foreign_hand — Phase 4.4 of `docs/proposals/HOUSE_MASTER_PLAN.md`
//! (`HOUSE_POWER_STRUGGLE_VIEW.md` §2). Built only after
//! `economy_validation.rs::econ_measure_foreign_hand_conjunction` measured the
//! trigger — a rival's commercial footprint in a posted kin's city — firing far
//! more than "a handful a century" (1229/century measured), the bar §2.5 itself
//! set for whether this was worth building at all.
//!
//! Two channels, both concrete, both derived from state that already exists —
//! neither invents a conspiracy, they model commercial DEPENDENCY:
//!   A — a rival house holds an office or bailo in the city our kin is posted to.
//!   B — our own house leases an office in a city that rival CONTROLS
//!       (`captor_house`) — the stronger channel, real dependency rather than mere
//!       proximity.
//!
//! Leverage only DEEPENS an existing grievance (a small, bounded monthly loyalty
//! decay); it never creates one — a kin who starts fully loyal needs YEARS of
//! sustained exposure before it matters, never a single month's coincidence.
use super::*;

impl CampaignSim {
    /// Monthly: nudge down the loyalty of any posted kin whose city shows a rival's
    /// commercial footprint, and occasionally disclose it by name.
    pub(crate) fn apply_foreign_hand(&mut self) {
        let tick = self.tick;
        let seed = self.seed;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild || self.houses[hi].kin.is_empty() { continue; }
            let leases = self.houses[hi].office_leases.clone();
            let rivals = self.houses[hi].rivals.clone();
            let n = self.houses[hi].kin.len();
            for ki in 0..n {
                let (posted, role) = { let k = &self.houses[hi].kin[ki]; (k.posted, k.role) };
                if posted < 0 || role == 4 || role == 5 { continue; }
                let hub = posted as usize;
                if hub >= self.hubs.len() { continue; }

                let rival_a = self.houses.iter().enumerate()
                    .find(|&(oj, oh)| oj != hi && !oh.defunct
                        && (oh.offices.contains(&(hub as u32)) || oh.bailos.contains(&(hub as u32))))
                    .map(|(oj, _)| oj);
                let captor = self.hubs[hub].captor_house;
                let rival_b = if captor >= 0 && captor as usize != hi
                    && leases.iter().any(|&(h, _)| h as usize == hub) {
                    Some(captor as usize)
                } else { None };

                let a = rival_a.is_some();
                let b = rival_b.is_some();
                if !a && !b { continue; }
                // Name the CONTROLLING rival (channel B, the real-dependency "strong"
                // channel per the design) when both are present and differ.
                let rival = rival_b.or(rival_a).unwrap();

                let rival_weight = self.houses.get(rival).map(|r| r.political_power.clamp(0.0, 1.0)).unwrap_or(0.0);
                if rival_weight <= 0.0 { continue; }
                let feud = if rivals.contains(&rival) { 1.0 } else { 0.0 };
                let leverage = ((FOREIGN_HAND_CHANNEL_A_WEIGHT * a as u8 as f32
                    + FOREIGN_HAND_CHANNEL_B_WEIGHT * b as u8 as f32).min(1.0))
                    * rival_weight * (1.0 + 0.5 * feud);
                if leverage <= 0.0 { continue; }

                let decay = leverage * FOREIGN_HAND_DECAY_RATE;
                self.houses[hi].kin[ki].loyalty = (self.houses[hi].kin[ki].loyalty - decay).max(0.0);

                let roll = hash01(seed, hi as u64 ^ ki as u64, tick as u64 ^ 0xF04E);
                if roll < FOREIGN_HAND_DISCLOSE_CHANCE * leverage.min(1.0) {
                    let kname = self.houses[hi].kin[ki].name.clone();
                    let rname = self.houses.get(rival).map(|r| r.name.clone()).unwrap_or_default();
                    let hub_name = self.hubs[hub].name.clone();
                    let channel_text = if b { format!("leases their bailo at {}", hub_name) }
                        else { format!("holds an office at {}", hub_name) };
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "foreign_hand".into(),
                        text: format!("{}'s hand is seen — {} {}", rname, kname, channel_text),
                    });
                }
            }
        }
    }
}
