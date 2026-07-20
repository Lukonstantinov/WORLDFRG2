//! war — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

impl CampaignSim {

    /// Raise a forced WAR LEVY from every house homed at `hub`: a slice of each
    /// fortune into the city's war chest (treasury). The core wealth sink of war.
    /// Returns the total raised.
    pub(crate) fn raise_war_levy(&mut self, hub: usize) -> f32 {
        let mut total = 0.0f32;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            if self.houses[hi].hub as usize != hub { continue; }
            let levy = self.houses[hi].wealth.max(0.0) * WAR_LEVY_RATE;
            if levy <= EPS { continue; }
            self.houses[hi].wealth -= levy;
            total += levy;
            if hi < self.house_ledger.len() { self.house_ledger[hi].civic_tax += levy; }
        }
        if hub < self.hubs.len() {
            self.hubs[hub].treasury += total;
            self.hubs[hub].finance.war_levy += total;
        }
        total
    }


    /// Spend a slice of `hub`'s treasury on the war effort (consumed — the cost of
    /// armies & blockade). Returns the amount spent (its contribution to victory).
    pub(crate) fn spend_war(&mut self, hub: usize) -> f32 {
        let spend = (self.hubs[hub].treasury.max(0.0) * WAR_SPEND_RATE).min(self.hubs[hub].treasury.max(0.0));
        self.hubs[hub].treasury -= spend;
        self.hubs[hub].finance.spent_war += spend;
        self.hubs[hub].war_effort += spend;
        spend
    }


    /// Apply a resolved war's GOAL — the victor's lasting spoils beyond the one-off
    /// plunder. Returns a short clause appended to the journal / Wars-log text.
    /// Trade rights & annexation both work through the BAILO primitive (a foreign
    /// governing foothold), so the winner's house is seated on the loser's council
    /// by the ordinary yearly recompute and the control transfer sticks.
    pub(crate) fn apply_war_goal(&mut self, win: usize, lose: usize, goal: u8, tick: u32, _yr: u32) -> String {
        let (wn, ln) = (self.hubs[win].name.clone(), self.hubs[lose].name.clone());
        // The victor's ruling family: its council head, else its richest resident.
        let ruler = {
            let c = self.hubs[win].council_house;
            if c >= 0 && (c as usize) < self.houses.len() { Some(c as usize) }
            else { self.strongest_house_at(win) }
        };
        let grant_bailo = |me: &mut Self, hi: usize| {
            if !me.houses[hi].bailos.contains(&(lose as u32)) {
                me.houses[hi].bailos.push(lose as u32);
            }
        };
        match goal {
            WAR_GOAL_TRIBUTE => {
                self.hubs[lose].tribute_to = win as i32;
                self.hubs[lose].tribute_until = tick + TRIBUTE_YEARS * TICKS_PER_YEAR;
                format!("; {} is made a tributary of {} for {} years", ln, wn, TRIBUTE_YEARS)
            }
            WAR_GOAL_TRADE_RIGHTS => {
                if let Some(hi) = ruler {
                    grant_bailo(self, hi);
                    let hn = self.houses[hi].name.clone();
                    format!("; {} wins trade rights in {} — a bailo for {}", wn, ln, hn)
                } else { String::new() }
            }
            WAR_GOAL_ANNEX => {
                if let Some(hi) = ruler {
                    grant_bailo(self, hi);
                    self.hubs[lose].council_house = hi as i32;
                    self.hubs[lose].coin_trust = (self.hubs[lose].coin_trust - 0.15).max(0.0);
                    let hn = self.houses[hi].name.clone();
                    format!("; {} is annexed by {} — {} installed on its council", ln, wn, hn)
                } else {
                    format!("; {} is annexed by {}", ln, wn)
                }
            }
            _ => String::new(),
        }
    }


    /// DLC 3.5 · the economic-war engine. Once a year: wage & resolve active wars
    /// (levies, war-chest spending, trade blockade, reparations) and occasionally
    /// declare a new one between rival poleis. Deterministic.
    pub(crate) fn update_wars(&mut self, yr: u32) {
        let tick = self.tick;
        // Tributaries pay their overlords first — a bounded treasury→treasury
        // transfer (never minted), lapsing when the term ends.
        for h in 0..self.hubs.len() {
            let to = self.hubs[h].tribute_to;
            if to < 0 { continue; }
            if tick >= self.hubs[h].tribute_until || to as usize >= self.hubs.len() {
                self.hubs[h].tribute_to = -1;
                continue;
            }
            let to = to as usize;
            let cap = TRIBUTE_CAP * self.city_size_factor(h);
            let pay = (self.hubs[h].treasury.max(0.0) * TRIBUTE_RATE).min(cap);
            if pay <= EPS { continue; }
            self.hubs[h].treasury -= pay;
            self.hubs[to].treasury += pay;
            self.hubs[h].finance.reparations_out += pay;
            self.hubs[to].finance.reparations_in += pay;
        }
        let mut ended: Vec<usize> = Vec::new();
        for wi in 0..self.wars.len() {
            let (a, b) = (self.wars[wi].a as usize, self.wars[wi].b as usize);
            if a >= self.hubs.len() || b >= self.hubs.len() {
                ended.push(wi); continue;
            }
            // Wage: levies on each side's houses, war-chest spending, trade blockade.
            let lev = self.raise_war_levy(a) + self.raise_war_levy(b);
            self.wars[wi].levies += lev;
            self.wars[wi].chest_a += self.spend_war(a);
            self.wars[wi].chest_b += self.spend_war(b);
            for &h in &[a, b] {
                self.hubs[h].trade_wealth *= 0.8; // blockade bites commerce
                self.active_events.push(ActiveEvent {
                    kind: "war".into(), hub: h as i32, good: -1,
                    magnitude: 0.4, until_tick: tick + TICKS_PER_YEAR,
                });
            }
            // Resolve after >= 2 years, weighted by mustered effort + treasury.
            let years = tick.saturating_sub(self.wars[wi].start_tick) / TICKS_PER_YEAR;
            if years >= 2 {
                let pa = self.wars[wi].chest_a + self.hubs[a].treasury + 1.0;
                let pb = self.wars[wi].chest_b + self.hubs[b].treasury + 1.0;
                let a_wins = hash01(self.seed, tick as u64 ^ 0xBA771E, wi as u64) < pa / (pa + pb);
                let (win, lose) = if a_wins { (a, b) } else { (b, a) };
                let rep = (self.hubs[lose].treasury.max(0.0) * 0.4).max(0.0);
                self.hubs[lose].treasury -= rep;
                self.hubs[win].treasury += rep;
                self.hubs[win].finance.reparations_in += rep;
                self.hubs[lose].finance.reparations_out += rep;
                self.hubs[lose].coin_trust = (self.hubs[lose].coin_trust - 0.15).max(0.0);
                self.hubs[a].war_with = -1;
                self.hubs[b].war_with = -1;
                // The victor claims the war's GOAL — its lasting spoils (tribute /
                // trade rights / annexation), beyond the one-off plunder above.
                let spoils = self.apply_war_goal(win, lose, self.wars[wi].goal, tick, yr);
                let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
                let (wn, ln) = (self.hubs[win].name.clone(), self.hubs[lose].name.clone());
                let text = format!(
                    "The war of {} and {} ends in year {}: {} prevails, {} pays {:.0} in reparations{}.",
                    an, bn, yr, wn, ln, rep, spoils);
                self.journal.push(JournalEntry { tick, kind: "war".into(), hub: win as i32, good: -1,
                    value: rep, text: text.clone() });
                self.war_log.push(WarRecord {
                    start_year: self.wars[wi].start_tick / TICKS_PER_YEAR, end_year: yr,
                    a_name: an, b_name: bn, winner: wn, loser: ln,
                    reparations: rep, levies_total: self.wars[wi].levies,
                    cause: self.wars[wi].cause.clone(), text,
                });
                if self.war_log.len() > WAR_LOG_CAP {
                    let drop = self.war_log.len() - WAR_LOG_CAP;
                    self.war_log.drain(0..drop);
                }
                // War of independence: the colony either wins free, or is brought to
                // heel for 15 years before it may rebel again.
                if self.wars[wi].cause == "independence" {
                    let colony = if self.hubs[a].colony_kind == 1 { a }
                        else if self.hubs[b].colony_kind == 1 { b } else { usize::MAX };
                    if colony != usize::MAX {
                        if colony == win {
                            self.make_colony_independent(colony, true);
                        } else {
                            self.hubs[colony].indep_cooldown_until = tick + 15 * TICKS_PER_YEAR;
                            let cn = self.hubs[colony].name.clone();
                            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: colony as i32,
                                good: -1, value: 0.0, text: format!("{}'s bid for independence is crushed; it remains a colony", cn) });
                        }
                    }
                }
                ended.push(wi);
            }
        }
        for &wi in ended.iter().rev() { self.wars.remove(wi); }
        self.maybe_declare_war(yr);
    }


    /// Occasionally ignite a new economic war between two rival poleis in the same
    /// region, both at peace. Rival councils are the spark; prosperity is the prize.
    pub(crate) fn maybe_declare_war(&mut self, yr: u32) {
        if self.wars.len() >= MAX_ACTIVE_WARS { return; }
        if hash01(self.seed, self.tick as u64 ^ 0xDEC1A6E, yr as u64) > WAR_DECLARE_CHANCE { return; }
        let n = self.hubs.len();
        // Candidate seats: real cities with a council, at peace.
        let seats: Vec<usize> = (0..n).filter(|&h|
            !self.hubs[h].is_estate && self.hubs[h].war_with < 0
            && self.hubs[h].council_house >= 0 && self.hubs[h].population > 1.0
        ).collect();
        if seats.len() < 2 { return; }
        // Prefer a pair in the same region whose councils are rivals; else any pair.
        let mut best: Option<(usize, usize, &'static str)> = None;
        for (ii, &a) in seats.iter().enumerate() {
            for &b in seats.iter().skip(ii + 1) {
                if self.hubs[a].component != self.hubs[b].component { continue; }
                let ca = self.hubs[a].council_house as usize;
                let cb = self.hubs[b].council_house as usize;
                let rivals = self.houses.get(ca).map(|h| h.rivals.contains(&cb)).unwrap_or(false)
                    || self.houses.get(cb).map(|h| h.rivals.contains(&ca)).unwrap_or(false);
                if rivals { best = Some((a, b, "rival councils")); break; }
                if best.is_none() { best = Some((a, b, "trade dispute")); }
            }
            if matches!(best, Some((_, _, "rival councils"))) { break; }
        }
        let Some((a, b, cause)) = best else { return };
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        // A WAR GOAL — what the aggressor is after. Rival councils fight for political
        // supremacy (annexation / trade rights); a plain trade dispute is fought for
        // tribute or plunder. A lopsided match leans toward annexation (the strong
        // swallow the weak); an even one toward tribute. Deterministic.
        let pow = |me: &Self, h: usize| me.hubs[h].population.max(1.0) * (me.hubs[h].treasury.max(0.0) + 5.0);
        let (pa, pb) = (pow(self, a), pow(self, b));
        let ratio = pa.max(pb) / pa.min(pb).max(1.0);
        let g = hash01(self.seed, self.tick as u64 ^ 0x60A15, (a * 131 + b) as u64);
        let goal = if cause == "rival councils" {
            if ratio >= 2.0 && g < 0.6 { WAR_GOAL_ANNEX } else { WAR_GOAL_TRADE_RIGHTS }
        } else if ratio >= 2.5 && g < 0.4 {
            WAR_GOAL_ANNEX
        } else if g < 0.6 {
            WAR_GOAL_TRIBUTE
        } else {
            WAR_GOAL_PLUNDER
        };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "war".into(), hub: a as i32, good: -1, value: 0.0,
            text: format!("{} declares war on {} ({} · {})", an, bn, cause, war_goal_label(goal)),
        });
        self.wars.push(War {
            a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, cargo_lost: 0, cause: cause.into(), goal,
        });
    }


    /// Monthly warehouse pass (Phase 2): (1) ensure every live house has a home depot
    /// and one in each office city; (2) STOCK — each house draws a slice of its
    /// specialty goods' LOCAL SURPLUS (never food, never below the city's trade
    /// reserve) into its depot, paying the market (the cost circulates into the
    /// city's civic pool) — this is the inventory a futures contract later ships out
    /// of; (3) EXPAND — a profitable house enlarges a depot that stays nearly full.
    /// Stocking only MOVES goods within a hub (pool → house depot), so the aggregate
    /// `hub_stock` — and thus prices, needs and the famine balance — is unchanged.
    pub(crate) fn sync_and_stock_warehouses(&mut self, needs: &[Vec<f32>]) {
        let ng = self.goods.len();
        let nh = self.houses.len();
        // Slowly heal cosmetic damage on standing depots.
        for w in &mut self.warehouses { w.damage *= 0.98; }
        // (1) Ensure home + office depots exist. A single membership set of existing
        //     (owner, hub) pairs replaces the old per-call linear scan, so this is
        //     O(houses + offices) rather than O(houses · warehouses).
        let nhub = self.hubs.len();
        let mut have: std::collections::HashSet<(i32, u32)> =
            self.warehouses.iter().map(|w| (w.owner, w.hub)).collect();
        let mut new_depots: Vec<(i32, u32)> = Vec::new();
        for hi in 0..nh {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub;
            if (home as usize) < nhub && have.insert((hi as i32, home)) {
                new_depots.push((hi as i32, home));
            }
            for off in self.houses[hi].offices.clone() {
                if (off as usize) < nhub && have.insert((hi as i32, off)) {
                    new_depots.push((hi as i32, off));
                }
            }
        }
        for (owner, hub) in new_depots {
            self.warehouses.push(Warehouse {
                owner, hub, capacity: WH_START_CAP,
                stock: vec![0.0; ng], tier: Self::capacity_tier(WH_START_CAP), damage: 0.0,
            });
        }
        // (2) Stocking.
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= nh || self.houses[oi].defunct { continue; }
            let hub = self.warehouses[wi].hub as usize;
            if hub >= self.hubs.len() { continue; }
            // `needs` is sized to the hub count at the consumption pass; a hub added
            // later this tick (a fresh estate/colony) has no needs row yet — skip it
            // this tick rather than index out of bounds.
            if hub >= needs.len() { continue; }
            let used: f32 = self.warehouses[wi].stock.iter().sum();
            let mut room = (self.warehouses[wi].capacity - used).max(0.0);
            if room <= EPS { continue; }
            for g in self.houses[oi].spec.clone() {
                if room <= EPS { break; }
                if g >= ng || self.goods[g].food { continue; }
                let reserve = needs[hub][g] * TRADE_RESERVE_MULT;
                let surplus = (self.hubs[hub].stock[g] - reserve).max(0.0);
                if surplus <= EPS { continue; }
                let price = self.live_price(self.hub_stock(hub, g), needs[hub][g], self.goods[g].base_value);
                let afford = if price > EPS { (self.houses[oi].wealth * 0.25).max(0.0) / price } else { 0.0 };
                let take = (surplus * WH_STOCK_FRAC).min(room).min(afford);
                if take <= EPS { continue; }
                self.hubs[hub].stock[g] -= take;
                self.warehouses[wi].stock[g] += take;
                let cost = take * price;
                self.houses[oi].wealth -= cost;
                self.hubs[hub].civic_pool += cost;
                room -= take;
            }
        }
        // (3) Expansion.
        let tick = self.tick;
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= nh || self.houses[oi].defunct { continue; }
            let cap = self.warehouses[wi].capacity;
            if cap >= WH_MAX_CAP { continue; }
            let used: f32 = self.warehouses[wi].stock.iter().sum();
            if used < cap * WH_FULL_FRAC { continue; }
            let cost = WH_EXPAND_COST * self.warehouses[wi].tier.max(1) as f32;
            if self.houses[oi].wealth < cost * 1.5 { continue; }
            if hash01(self.seed, tick as u64 ^ 0x3A5E, wi as u64) > 0.25 { continue; }
            self.houses[oi].wealth -= cost;
            let newcap = (cap * WH_EXPAND_MULT).min(WH_MAX_CAP);
            self.warehouses[wi].capacity = newcap;
            self.warehouses[wi].tier = Self::capacity_tier(newcap);
        }
    }


    /// A thriving, crowded city sends SWARM_POP_FRAC of its people to break new
    /// ground: an INDEPENDENT town (no charter, no lifeline) on the best farmable
    /// free site within short range. At most one founding a year worldwide so each
    /// stays a legible chronicle event.
    pub(crate) fn maybe_swarm_town(&mut self) {
        if self.colonizable.is_empty() { return; }
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || hub.colony_kind != 0 { continue; }
            if hub.population < SWARM_MIN_POP
                || hub.population < hub.founding_pop * SWARM_PRESSURE { continue; }
            if hub.starving > 0.05 || hub.mood < 0.5 { continue; }
            let score = hub.population / hub.founding_pop.max(1.0) * (0.5 + hub.mood);
            if score > best.1 { best = (h, score); }
        }
        let Some(mother) = (best.0 != usize::MAX).then_some(best.0) else { return };
        let cap = self.world_w * SWARM_REACH_FRAC;
        let (mx, my) = (self.hubs[mother].x, self.hubs[mother].y);
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            if s.fertility < SWARM_MIN_FERTILE { continue; }
            let mut dx = (s.x - mx).abs();
            if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
            let d = (dx * dx + (s.y - my).powi(2)).sqrt();
            // Not beyond a settler's walk, and not on the mother's doorstep either.
            if d > cap || d < cap * 0.15 { continue; }
            let score = (0.3 + s.fertility + 0.5 * s.trade_value) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        let site = self.colonizable.remove(si);
        let seed_pop = (self.hubs[mother].population * SWARM_POP_FRAC).max(600.0);
        self.hubs[mother].population -= seed_pop;
        let mother_name = self.hubs[mother].name.clone();
        let idx = self.create_organic_town(mother, &site, seed_pop);
        let new_name = self.hubs[idx].name.clone();
        self.total_foundings += 1;
        // Naturally-spawned free towns are a MERCHANT venture (related to a house/
        // guild), unlike the city-founded DEPENDENT satellites: name the sponsoring
        // house/guild in the beat so the tie reads in the chronicle.
        let sponsor = self.strongest_house_at(mother).map(|hi| self.houses[hi].name.clone());
        let text = match &sponsor {
            Some(hn) => format!("Merchants of {} lead settlers from {} to found {}", hn, mother_name, new_name),
            None => format!("Settlers from {} found {}", mother_name, new_name),
        };
        self.journal.push(JournalEntry { tick: self.tick, kind: "founding".into(),
            hub: idx as i32, good: -1, value: seed_pop, text });
    }


    /// Open a war of independence between a colony and its metropolis (reuses the
    /// economic-war machinery; resolved in `update_wars`, where chest+treasury+luck
    /// decide it — a colony can still win against the odds).
    pub(crate) fn declare_independence_war(&mut self, colony: usize, metro: usize) {
        let (a, b) = if colony < metro { (colony, metro) } else { (metro, colony) };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        self.wars.push(War { a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, cargo_lost: 0, cause: "independence".into(),
            goal: WAR_GOAL_PLUNDER });
        let (cn, mn) = (self.hubs[colony].name.clone(), self.hubs[metro].name.clone());
        self.journal.push(JournalEntry { tick: self.tick, kind: "war".into(), hub: colony as i32,
            good: -1, value: 0.0, text: format!("{} rises in a war of independence against {}", cn, mn) });
    }


    /// Houses that specialize in the same good and sit in the same component become
    /// rivals (competing for the same trade). A feud occasionally flares into a
    /// Chronicle event with a mutual prestige/wealth cost.
    pub(crate) fn update_rivalries(&mut self) {
        let n = self.houses.len();
        for a in 0..n {
            if self.houses[a].defunct { continue; }
            for b in (a + 1)..n {
                if self.houses[b].defunct { continue; }
                let shared = self.houses[a].spec.iter().any(|g| self.houses[b].spec.contains(g));
                let same_region = self.houses[a].hub == self.houses[b].hub
                    || self.hubs.get(self.houses[a].hub as usize).map(|h| h.component)
                        == self.hubs.get(self.houses[b].hub as usize).map(|h| h.component);
                if shared && same_region {
                    if !self.houses[a].rivals.contains(&b) { self.houses[a].rivals.push(b); }
                    if !self.houses[b].rivals.contains(&a) { self.houses[b].rivals.push(a); }
                    // Feud flare: the weaker pays, occasionally logged.
                    let roll = hash01(self.seed, self.tick as u64 ^ a as u64, b as u64);
                    if roll < 0.15 {
                        let (loser, winner) = if self.houses[a].wealth < self.houses[b].wealth {
                            (a, b)
                        } else { (b, a) };
                        self.houses[loser].wealth *= 0.92;
                        self.houses[winner].prestige += 0.03;
                        if roll < 0.05 {
                            let (ln, wn) = (self.houses[loser].name.clone(),
                                self.houses[winner].name.clone());
                            self.journal.push(JournalEntry {
                                tick: self.tick, kind: "feud".into(),
                                hub: self.houses[winner].hub as i32, good: -1, value: 0.0,
                                text: format!("{} outmaneuvers {} in a bitter trade feud", wn, ln),
                            });
                            // Trade war: if the winner dominates its seat city and the
                            // loser is not an embargo-immune guild, CLOSE that market to
                            // the loser until it pays to regain its rights.
                            if self.houses[winner].dominant_seat && !self.houses[loser].is_guild {
                                let city = self.houses[winner].hub;
                                let already = self.house_barred.get(loser).is_some_and(|v| v.contains(&city));
                                if !already {
                                    let cn = self.hubs.get(city as usize).map(|h| h.name.clone()).unwrap_or_default();
                                    if let Some(v) = self.house_barred.get_mut(loser) { v.push(city); }
                                    self.journal.push(JournalEntry {
                                        tick: self.tick, kind: "trade_war".into(),
                                        hub: city as i32, good: -1, value: 0.0,
                                        text: format!("{} bars {} from the market of {}", wn, ln, cn),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
