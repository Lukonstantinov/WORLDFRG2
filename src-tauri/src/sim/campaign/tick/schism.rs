//! schism — Phase 4.1 of `docs/proposals/HOUSE_MASTER_PLAN.md`: a house under
//! enough internal tension either QUARRELS (common, chronicled, no split) or, more
//! rarely, sees a disloyal POSTED kin DEPART with the holding they run to found a
//! new rival house. `HOUSE_PEOPLE_AND_TIERS.md` §5 names a third outcome, Rupture (a
//! Tier 1-2 house splitting by line of descent) — deliberately NOT built here,
//! consistent with `HOUSE_MASTER_PLAN.md` Part 3's own earlier call to defer
//! "Rupture... behind Departure" (Departure gives most of the effect for a fraction
//! of the risk to the wealth distribution).
use super::*;

impl CampaignSim {
    /// A stand-in for `HOUSE_PEOPLE_AND_TIERS.md` §5's own `tension` formula, built
    /// from state this codebase already carries rather than the full `cohesion`
    /// gauge (that one is computed read-only in `campaign_house_stability` and isn't
    /// cheaply reachable from inside the tick). Combines the design's `cohesion` and
    /// `mean kin loyalty` terms into one loyalty-weighted term (they overlap heavily
    /// — both are spread/loyalty signals), and keeps `stretch`/feuds/passed-over-heir
    /// as their own terms.
    fn house_tension(&self, hi: usize) -> f32 {
        let h = &self.houses[hi];
        if h.kin.is_empty() { return 0.0; }
        let live: Vec<&Kin> = h.kin.iter().filter(|k| k.role != 4 && k.role != 5).collect();
        if live.is_empty() { return 0.0; }
        let mean_loyalty = live.iter().map(|k| k.loyalty).sum::<f32>() / live.len() as f32;
        let nodes = 1 + h.offices.len() + h.bailos.len();
        let stretch = ((nodes as f32 - 3.0).max(0.0) / 10.0).clamp(0.0, 1.0);
        let feuds = (h.rivals.len() as f32 / 3.0).clamp(0.0, 1.0);
        let passed_over = live.iter().any(|k| k.role == 1 && k.loyalty < 0.35) as u8 as f32;
        (0.45 * (1.0 - mean_loyalty) + 0.25 * stretch + 0.20 * feuds + 0.10 * passed_over).clamp(0.0, 1.0)
    }

    /// Monthly: a house whose tension crosses the threshold (and is past its own
    /// cooldown) quarrels or, rarely, loses a posted kin to Departure.
    pub(crate) fn update_house_schisms(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            if tick < self.houses[hi].schism_cooldown_until { continue; }
            if self.house_tension(hi) < SCHISM_TENSION_THRESHOLD { continue; }
            let seat = self.houses[hi].hub as i32;
            let disloyal = self.houses[hi].kin.iter().enumerate()
                .filter(|(_, k)| k.role != 0 && k.role != 4 && k.role != 5)
                .min_by(|a, b| a.1.loyalty.partial_cmp(&b.1.loyalty).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i);
            let Some(ki) = disloyal else { continue; };
            let posted = self.houses[hi].kin[ki].posted;
            let roll = hash01(self.seed, hi as u64, tick as u64 ^ 0x5C41);
            if posted >= 0 && posted != seat && roll < DEPARTURE_CHANCE {
                self.departure_schism(hi, ki);
                self.houses[hi].schism_cooldown_until = tick + SCHISM_COOLDOWN_DEPARTURE_YEARS * TICKS_PER_YEAR;
            } else {
                self.quarrel_schism(hi, ki);
                self.houses[hi].schism_cooldown_until = tick + SCHISM_COOLDOWN_QUARREL_YEARS * TICKS_PER_YEAR;
            }
        }
    }

    /// Common outcome: cohesion drops, the disloyal kin's own loyalty craters
    /// further, chronicled as chatter (not a milestone — a quarrel alone changes
    /// nothing structural about the house).
    fn quarrel_schism(&mut self, hi: usize, ki: usize) {
        let tick = self.tick;
        let name = self.houses[hi].name.clone();
        let kname = self.houses[hi].kin[ki].name.clone();
        self.houses[hi].kin[ki].loyalty = (self.houses[hi].kin[ki].loyalty * 0.5).max(0.0);
        self.houses[hi].prestige = (self.houses[hi].prestige - 0.03).max(0.0);
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "quarrel".into(),
            text: format!("a quarrel breaks out in {} — {} grows openly hostile", name, kname),
        });
    }

    /// Rarer outcome: the disloyal kin leaves WITH the holding they were posted to
    /// and founds a new house there — `found_branch`'s own machinery, reused with a
    /// forced identity (the departing kinsman becomes the new house's founder)
    /// rather than a fresh culture-rule pick, the same pattern `depose_and_succeed`
    /// already established in `crisis.rs`. The new house starts as a RIVAL, per the
    /// design.
    fn departure_schism(&mut self, hi: usize, ki: usize) {
        let tick = self.tick;
        let posted = self.houses[hi].kin[ki].posted;
        if posted < 0 || posted as usize >= self.hubs.len() { self.quarrel_schism(hi, ki); return; }
        let dest = posted as usize;
        let kin = self.houses[hi].kin[ki].clone();
        let parent = self.houses[hi].name.clone();
        let bname = self.branch_name_for(&parent, dest);
        if self.houses.iter().any(|h| !h.defunct && h.hub as usize == dest && h.name == bname) {
            // Blocked (a branch already sits there) — still reads as family strain.
            self.quarrel_schism(hi, ki);
            return;
        }
        let split = (self.houses[hi].wealth * DEPARTURE_WEALTH_FRAC).max(0.0);
        self.houses[hi].wealth -= split;
        let spec = self.houses[hi].spec.clone();
        let (_, inh) = self.rules_for_hub(dest);
        let (age, tenure) = self.roll_tenure(inh, hi as u64 ^ tick as u64 ^ 0x5C4E);
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("{} departs {} and founds {} in {}", kin.name, parent, bname, self.hubs[dest].name),
        };
        self.houses.push(House {
            name: bname.clone(), hub: dest as u32, wealth: split, prestige: 0.05,
            spec, monopoly: vec![], rivals: vec![hi], generation: 1,
            events: vec![founded], good_profit: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: split, worst_loss: 0.0,
            fleet_sea: 0, fleet_river: 0, fleet_caravan: 0,
            head_name: kin.name.clone(), head_since: tick, head_lifespan: tenure,
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: self.houses[hi].archetype,
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
            head_female: kin.female, head_age: age, line: Vec::new(), tier: 0, standing: 0.0,
            peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0,
            golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(),
            goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0,
            crisis_history: Vec::new(), schism_cooldown_until: 0,
            origin_house: hi as i32, origin_kind: ORIGIN_DEPARTURE,
        });
        let ni = self.houses.len() - 1;
        self.found_head_record(ni, "departure");
        self.houses[hi].rivals.push(ni);
        self.houses[hi].kin.remove(ki);
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "schism".into(),
            text: format!("{} departs {} to found {} in {}", kin.name, parent, bname, self.hubs[dest].name),
        });
        self.journal.push(JournalEntry {
            tick, kind: "schism".into(), hub: dest as i32, good: -1, value: split,
            text: format!("{} founds {} in {}, departing {}", kin.name, bname, self.hubs[dest].name, parent),
        });
    }
}
