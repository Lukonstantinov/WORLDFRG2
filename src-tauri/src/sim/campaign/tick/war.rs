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


    // ═══════════════════════════════════════════════════════════════════════════
    //  FEUDS
    //
    //  The old model was one symmetric `rivals` entry plus a 15%-per-half-year roll
    //  in which the weaker house lost 8% of its wealth. That is a tax on being poor:
    //  it had no cause, no memory, no escalation, and no ending short of one side
    //  dying. What follows keeps `rivals` in sync — every existing consumer (war
    //  causes in `pick_war_pair`, marriage eligibility, the Houses panel) reads the
    //  same field it always did — and adds the four things a quarrel between merchant
    //  families actually has:
    //
    //    · a CAUSE      (`FEUD_TRADE` … `FEUD_SUCCESSION`) — feuds are ABOUT something
    //    · an INTENSITY that heats with live overlap and cools without it
    //    · STAGES whose weapons differ: a cold rivalry is words, a vendetta burns ships
    //    · a SETTLEMENT — arbitration by a council, a marriage, ruin, or plain neglect
    //
    //  Historically this is the Italian/Hanseatic pattern: quarrels between trading
    //  families were adjudicated by the commune they both traded in, because an open
    //  feud between two big houses was a threat to the city's own commerce. A feud
    //  that can only end when someone dies produces a world where every old house has
    //  a dozen rivals; one that can be settled produces a world with a HISTORY.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Index of the live feud between `a` and `b`, if any (order-insensitive).
    pub(crate) fn feud_between(&self, a: usize, b: usize) -> Option<usize> {
        let (lo, hi) = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        self.feuds.iter().position(|f| f.a == lo && f.b == hi && f.outcome == FEUD_RUNNING)
    }

    /// How much two houses actually get in each other's way right now, plus the good
    /// and the city the quarrel is most about. Overlap drives heating: two houses that
    /// no longer trade the same goods in the same places stop feuding, which is what
    /// lets a feud cool on its own.
    pub(crate) fn feud_overlap(&self, a: usize, b: usize) -> (f32, i32, i32) {
        if a >= self.houses.len() || b >= self.houses.len() { return (0.0, -1, -1); }
        let (ha, hb) = (&self.houses[a], &self.houses[b]);
        if ha.defunct || hb.defunct { return (0.0, -1, -1); }
        // Shared trade: the goods both live off. The FIRST shared good names the feud.
        let mut good = -1i32;
        let mut shared_goods = 0u32;
        for g in &ha.spec {
            if hb.spec.contains(g) {
                shared_goods += 1;
                if good < 0 { good = *g as i32; }
            }
        }
        // Shared ground: cities both stand in (seat, offices, bailos). The one where
        // their combined influence is highest is the city the feud is about.
        let nodes = |h: &House| -> Vec<u32> {
            let mut v = Vec::with_capacity(1 + h.offices.len() + h.bailos.len());
            v.push(h.hub);
            v.extend(h.offices.iter().copied());
            v.extend(h.bailos.iter().copied());
            v
        };
        let (na, nb) = (nodes(ha), nodes(hb));
        let inf_at = |h: &House, c: u32| h.influence.iter().find(|(x, _)| *x == c)
            .map(|(_, v)| *v).unwrap_or(0.0);
        let mut shared_cities = 0u32;
        let mut hub = -1i32;
        let mut best_inf = -1.0f32;
        for &c in &na {
            if !nb.contains(&c) { continue; }
            shared_cities += 1;
            let combined = inf_at(ha, c) + inf_at(hb, c);
            // Ties break on the LOWER hub index so the choice is order-independent.
            if combined > best_inf { best_inf = combined; hub = c as i32; }
        }
        // Same trading component is a weak tie on its own — it is what made the old
        // model pair up every house on a continent. It counts, but only a little.
        let same_component = self.hubs.get(ha.hub as usize).map(|h| h.component)
            == self.hubs.get(hb.hub as usize).map(|h| h.component);
        let overlap = (0.22 * shared_goods.min(3) as f32
            + 0.30 * shared_cities.min(3) as f32
            + if same_component { 0.10 } else { 0.0 }).clamp(0.0, 1.0);
        (overlap, good, hub)
    }

    /// Open (or re-heat) a feud between two houses for a named reason. Every path that
    /// creates bad blood — a soured match, a closed market, a contested council — comes
    /// through here, so a feud always knows why it exists.
    pub(crate) fn open_feud(&mut self, a: usize, b: usize, cause: u8, good: i32, hub: i32,
                            heat: f32) {
        if a == b || a >= self.houses.len() || b >= self.houses.len() { return; }
        if self.houses[a].defunct || self.houses[b].defunct { return; }
        let (lo, hi) = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        if let Some(fi) = self.feud_between(a, b) {
            // Already quarrelling — a fresh grievance pours heat on the existing feud
            // rather than starting a second one.
            self.feuds[fi].intensity = (self.feuds[fi].intensity + heat).min(1.0);
            let cur = self.feuds[fi].stage;
            self.feuds[fi].stage = feud_stage(self.feuds[fi].intensity, cur);
            return;
        }
        if self.feuds.iter().filter(|f| f.outcome == FEUD_RUNNING).count() >= FEUDS_CAP { return; }
        self.feuds.push(Feud {
            a: lo, b: hi, cause, good, hub,
            intensity: heat.clamp(0.0, 1.0),
            stage: feud_stage(heat, FEUD_COLD),
            started_tick: self.tick, last_flare_tick: 0, flares: 0,
            damage_a: 0.0, damage_b: 0.0,
            outcome: FEUD_RUNNING, ended_tick: 0, log: Vec::new(),
        });
        if !self.houses[a].rivals.contains(&b) { self.houses[a].rivals.push(b); }
        if !self.houses[b].rivals.contains(&a) { self.houses[b].rivals.push(a); }
        let (na, nb) = (self.houses[a].name.clone(), self.houses[b].name.clone());
        let why = FEUD_CAUSES.get(cause as usize).copied().unwrap_or("an old grievance");
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "feud".into(), hub: self.houses[a].hub as i32,
            good, value: 0.0,
            text: format!("{} and {} fall out over {}", na, nb, why),
        });
        for (h, other) in [(a, nb.clone()), (b, na.clone())] {
            self.houses[h].events.push(HouseEvent {
                tick: self.tick, kind: "feud".into(),
                text: format!("Fell out with {} over {}", other, why),
            });
        }
    }

    /// Close a feud with a stated outcome, clearing the rival entries on both sides so
    /// the rest of the sim sees the quarrel as genuinely over.
    pub(crate) fn close_feud(&mut self, fi: usize, outcome: u8) {
        if fi >= self.feuds.len() || self.feuds[fi].outcome != FEUD_RUNNING { return; }
        let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
        self.feuds[fi].outcome = outcome;
        self.feuds[fi].ended_tick = self.tick;
        if a < self.houses.len() { self.houses[a].rivals.retain(|&r| r != b); }
        if b < self.houses.len() { self.houses[b].rivals.retain(|&r| r != a); }
    }

    /// Every feud a (now defunct) house was part of ends in its ruin. Called from
    /// `dissolve_house` so a dead family does not leave live quarrels behind.
    pub(crate) fn end_feuds_of(&mut self, hi: usize) {
        let idx: Vec<usize> = self.feuds.iter().enumerate()
            .filter(|(_, f)| f.outcome == FEUD_RUNNING
                && (f.a as usize == hi || f.b as usize == hi))
            .map(|(i, _)| i).collect();
        for fi in idx { self.close_feud(fi, FEUD_RUINED); }
    }

    /// Half-yearly · FORMATION. Scan house pairs for a reason to fall out. This is the
    /// only O(n²) part of the feud system and it keeps the old cadence; the per-feud
    /// work below runs monthly over the bounded `feuds` list instead.
    pub(crate) fn update_rivalries(&mut self) {
        let n = self.houses.len();
        // Old saves (and any state written before feuds existed) carry rival pairs with
        // no feud object. Adopt them once, as trade feuds already part-way heated, so a
        // loaded campaign does not silently lose its quarrels.
        if self.feuds.is_empty() {
            let mut adopt: Vec<(usize, usize)> = Vec::new();
            for a in 0..n {
                if self.houses[a].defunct { continue; }
                for &b in &self.houses[a].rivals {
                    if b > a && b < n && !self.houses[b].defunct { adopt.push((a, b)); }
                }
            }
            for (a, b) in adopt {
                let (_, good, hub) = self.feud_overlap(a, b);
                self.open_feud(a, b, FEUD_TRADE, good, hub, 0.35);
            }
        }
        for a in 0..n {
            if self.houses[a].defunct { continue; }
            for b in (a + 1)..n {
                if self.houses[b].defunct { continue; }
                if self.feud_between(a, b).is_some() { continue; }
                // Allied houses do not start quarrels with each other.
                let (lo, hi) = (a.min(b) as u32, a.max(b) as u32);
                if self.alliances.contains(&(lo, hi)) { continue; }
                let (overlap, good, hub) = self.feud_overlap(a, b);
                if overlap < 0.30 { continue; } // brushing past each other is not a feud
                // A contested COUNCIL is a sharper cause than mere competition: both
                // families are courting the same city's seats.
                let contested = hub >= 0 && {
                    let inf = |h: usize| self.houses[h].influence.iter()
                        .find(|(c, _)| *c as i32 == hub).map(|(_, v)| *v).unwrap_or(0.0);
                    inf(a) >= 0.12 && inf(b) >= 0.12
                };
                let cause = if contested { FEUD_SEAT } else { FEUD_TRADE };
                // Not every overlap becomes a quarrel — some houses simply coexist.
                let roll = hash01(self.seed, self.tick as u64 ^ (a as u64) << 12, b as u64);
                if roll > overlap * 0.55 { continue; }
                self.open_feud(a, b, cause, good, hub, 0.10 + 0.20 * overlap);
            }
        }
    }

    /// Monthly · TEMPERATURE + FLARES. Runs over the bounded feud list: heat each live
    /// feud by how much the two houses still get in each other's way, cool it when they
    /// no longer do, re-derive the stage, and occasionally let it flare.
    pub(crate) fn update_feuds(&mut self) {
        if self.feuds.is_empty() { return; }
        let tick = self.tick;
        let mut forget: Vec<usize> = Vec::new();
        for fi in 0..self.feuds.len() {
            if self.feuds[fi].outcome != FEUD_RUNNING { continue; }
            let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
            if a >= self.houses.len() || b >= self.houses.len()
                || self.houses[a].defunct || self.houses[b].defunct {
                self.close_feud(fi, FEUD_RUINED);
                continue;
            }
            let (overlap, good, hub) = self.feud_overlap(a, b);
            // Phase 2.4 · the greedier of the two heads (axis 1) heats the quarrel
            // faster; the more honourable cools it — averaged since a feud has two
            // sides, ±15% capped.
            let heat_mod = (self.head_character_factor(a, 1) + self.head_character_factor(b, 1)) / 2.0;
            {
                let f = &mut self.feuds[fi];
                if overlap > 0.0 {
                    f.intensity = (f.intensity + FEUD_HEAT * heat_mod * overlap).min(1.0);
                    // Keep the feud pointed at what it is currently about — two houses
                    // whose quarrel has moved to a new market should say so.
                    if good >= 0 { f.good = good; }
                    if hub >= 0 { f.hub = hub; }
                } else {
                    f.intensity = (f.intensity - FEUD_COOL).max(0.0);
                }
                f.stage = feud_stage(f.intensity, f.stage);
            }
            if self.feuds[fi].intensity < FEUD_FORGET { forget.push(fi); continue; }
            let stage = self.feuds[fi].stage as usize;
            let roll = hash01(self.seed, tick as u64 ^ ((a as u64) << 20), b as u64);
            if roll < FEUD_FLARE_CHANCE[stage] { self.feud_flare(fi); }
        }
        for fi in forget { self.close_feud(fi, FEUD_COOLED); }
        // A settled feud is kept for the record but must not grow without bound.
        if self.feuds.len() > FEUDS_CAP * 2 {
            let drop = self.feuds.len() - FEUDS_CAP * 2;
            let mut removed = 0usize;
            self.feuds.retain(|f| {
                if removed < drop && f.outcome != FEUD_RUNNING { removed += 1; false } else { true }
            });
        }
    }

    /// One flare. The stage decides the weapon — this is where the elaboration earns
    /// its keep, because "what a feud DOES" now depends on how bad it has become.
    fn feud_flare(&mut self, fi: usize) {
        let tick = self.tick;
        let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
        let stage = self.feuds[fi].stage;
        // The stronger house prevails — by wealth, with prestige and political weight
        // counting for something (a poorer but better-connected family can win).
        let power = |h: &House| h.wealth.max(0.0) + h.prestige * 400.0 + h.political_power * 600.0;
        let (winner, loser) = if power(&self.houses[a]) >= power(&self.houses[b]) { (a, b) } else { (b, a) };
        let (wn, ln) = (self.houses[winner].name.clone(), self.houses[loser].name.clone());
        // Limited liability: a bite is a share of what the loser actually has, so a
        // feud can impoverish a house but never drive it arbitrarily negative.
        let bite = self.houses[loser].wealth.max(0.0) * FEUD_BITE[stage as usize];
        self.houses[loser].wealth -= bite;
        // Prestige is UNBOUNDED and feeds political power → charters → monopolies →
        // wealth, so a per-flare award is a compounding loop, not flavour. The old
        // model paid +0.03 at ~0.3 flares/yr ≈ 0.009/yr; paying +0.008..0.032 at up to
        // 3.4 flares/yr came to ~0.11/yr, and the dynamics run's sustained-richest house
        // went from 298k to 1.9M — a monopoly runaway driven entirely by feud prestige.
        // Kept deliberately close to the old annual rate, and capped, so winning feuds
        // makes a family respected without making it unassailable.
        let gain = (0.002 + 0.002 * stage as f32).min(FEUD_PRESTIGE_CAP - self.houses[winner].prestige);
        self.houses[winner].prestige += gain.max(0.0);
        self.houses[loser].prestige = (self.houses[loser].prestige - 0.004 * stage as f32).max(0.0);
        let good_name = {
            let g = self.feuds[fi].good;
            if g >= 0 { self.goods.get(g as usize).map(|x| x.name.clone()) } else { None }
        };
        let city = self.feuds[fi].hub;
        let city_name = if city >= 0 {
            self.hubs.get(city as usize).map(|h| h.name.clone()).unwrap_or_default()
        } else { String::new() };

        let text = match stage {
            FEUD_COLD => match &good_name {
                Some(g) => format!("{} snubs {} over the {} trade", wn, ln, g),
                None => format!("{} snubs {} at the exchange", wn, ln),
            },
            FEUD_OPEN => {
                // Undercutting: the loser's recent volume — the basis of its market
                // share and monopolies — is cut back.
                self.houses[loser].volume *= 0.94;
                match &good_name {
                    Some(g) => format!("{} undercuts {} in {}, taking the trade", wn, ln, g),
                    None => format!("{} undercuts {} and takes its custom", wn, ln),
                }
            }
            FEUD_TRADEWAR => {
                self.houses[loser].volume *= 0.90;
                // Strip the loser's standing in the contested city, and — where the
                // winner governs it — close the market outright (the pre-existing
                // trade-war path, now driven by a feud that EARNED it).
                let mut barred_here = false;
                if city >= 0 {
                    let c = city as u32;
                    if let Some(e) = self.houses[loser].influence.iter_mut().find(|(x, _)| *x == c) {
                        e.1 = (e.1 - FEUD_INFLUENCE_STRIP).max(0.0);
                    }
                    let governs = self.hubs.get(c as usize)
                        .map(|h| h.captor_house == winner as i32).unwrap_or(false)
                        || (self.houses[winner].dominant_seat && self.houses[winner].hub == c);
                    if governs && !self.houses[loser].is_guild {
                        let already = self.house_barred.get(loser).is_some_and(|v| v.contains(&c));
                        if !already {
                            if let Some(v) = self.house_barred.get_mut(loser) { v.push(c); }
                            barred_here = true;
                        }
                    }
                }
                if barred_here {
                    self.journal.push(JournalEntry {
                        tick, kind: "trade_war".into(), hub: city, good: -1, value: 0.0,
                        text: format!("{} bars {} from the market of {}", wn, ln, city_name),
                    });
                    format!("{} bars {} from the market of {}", wn, ln, city_name)
                } else if !city_name.is_empty() {
                    format!("{} drives {} out of the {} trade", wn, ln, city_name)
                } else {
                    format!("{} shuts {} out of its markets", wn, ln)
                }
            }
            _ => {
                // Vendetta: property. A ship taken, or a foreign office forced shut —
                // the two things that actually cost a merchant house its reach.
                self.houses[loser].volume *= 0.88;
                let pick = hash01(self.seed, tick as u64, (loser as u64) << 8);
                let fleet = self.houses[loser].fleet_sea + self.houses[loser].fleet_river
                    + self.houses[loser].fleet_caravan;
                if fleet > 0 && pick < 0.6 {
                    let sea = self.houses[loser].fleet_sea > 0;
                    self.damage_fleet(loser, sea);
                    format!("{} has a {} of {} taken at sea", wn,
                        if sea { "ship" } else { "caravan" }, ln)
                } else {
                    // Force a foreign counting-house shut. Only a lease-free office can
                    // be taken; a leased one is held open by its term, and one backing a
                    // live contract stays open — the same rules the office system uses.
                    let victim = self.houses[loser].offices.iter().copied().find(|&o| {
                        (city < 0 || o as i32 == city)
                            && !self.office_leased(loser, o)
                            && !self.backs_active_contract(loser, o)
                    });
                    match victim {
                        Some(off) => {
                            let on = self.hubs.get(off as usize).map(|h| h.name.clone())
                                .unwrap_or_default();
                            self.houses[loser].offices.retain(|&o| o != off);
                            self.houses[loser].bailos.retain(|&o| o != off);
                            self.houses[loser].influence.retain(|&(c, _)| c != off);
                            format!("{} forces {}'s counting-house in {} to close", wn, ln, on)
                        }
                        None => format!("{} sets its bravos on {}'s factors", wn, ln),
                    }
                }
            }
        };

        {
            let f = &mut self.feuds[fi];
            f.flares += 1;
            f.last_flare_tick = tick;
            if loser as u32 == f.a { f.damage_a += bite; } else { f.damage_b += bite; }
            f.log.push(FeudFlare { tick, stage, loser: loser as u32, cost: bite, text: text.clone() });
            if f.log.len() > FEUD_LOG_CAP { let d = f.log.len() - FEUD_LOG_CAP; f.log.drain(0..d); }
        }
        // Only the loud stages reach the world chronicle; a cold snub belongs to the
        // two families' own records, not to the history of the world.
        if stage >= FEUD_TRADEWAR {
            self.journal.push(JournalEntry {
                tick, kind: "feud".into(), hub: self.houses[winner].hub as i32,
                good: self.feuds[fi].good, value: bite, text: text.clone(),
            });
        }
        self.houses[winner].events.push(HouseEvent { tick, kind: "feud".into(), text: text.clone() });
        self.houses[loser].events.push(HouseEvent { tick, kind: "feud".into(), text });
    }

    /// Yearly · SETTLEMENT. A council both houses trade in has every reason to end a
    /// long feud: two great families at open war in its market is the city's problem,
    /// not just theirs. This is the mechanism the old model lacked entirely, and it is
    /// what stops the world converging on "every old house feuds with every other".
    pub(crate) fn arbitrate_feuds(&mut self, yr: u32) {
        if self.feuds.is_empty() { return; }
        let tick = self.tick;
        let mut settle: Vec<(usize, usize, f32)> = Vec::new(); // feud, city, damages
        for fi in 0..self.feuds.len() {
            let f = &self.feuds[fi];
            if f.outcome != FEUD_RUNNING { continue; }
            if tick.saturating_sub(f.started_tick) < FEUD_ARBITRATE_YEARS * TICKS_PER_YEAR { continue; }
            let city = f.hub;
            if city < 0 { continue; }
            let ci = city as usize;
            // A city at war has other concerns; a captured one favours its captor
            // instead of arbitrating.
            match self.hubs.get(ci) {
                Some(h) if h.war_with < 0 && h.captor_house < 0 && !h.abandoned => {}
                _ => continue,
            }
            let roll = hash01(self.seed, yr as u64 ^ 0xA2B1, fi as u64);
            if roll >= FEUD_ARBITRATE_CHANCE { continue; }
            // The council fines the AGGRESSOR — the house that has taken less damage,
            // i.e. the one that has been winning — and pays part to the injured party.
            let (a, b) = (f.a as usize, f.b as usize);
            let aggressor = if f.damage_a <= f.damage_b { a } else { b };
            let injured = if aggressor == a { b } else { a };
            let owed = (f.damage_a.max(f.damage_b) * 0.25)
                .min(self.houses.get(aggressor).map(|h| h.wealth.max(0.0) * 0.10).unwrap_or(0.0));
            settle.push((fi, ci, owed));
            let _ = (aggressor, injured);
        }
        for (fi, ci, owed) in settle {
            let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
            let aggressor = if self.feuds[fi].damage_a <= self.feuds[fi].damage_b { a } else { b };
            let injured = if aggressor == a { b } else { a };
            if aggressor >= self.houses.len() || injured >= self.houses.len() { continue; }
            self.houses[aggressor].wealth -= owed;
            self.houses[injured].wealth += owed * 0.7;
            if let Some(h) = self.hubs.get_mut(ci) { h.civic_pool += owed * 0.3; }
            // Both lose a little standing: needing the council to settle your quarrel
            // is not a good look for a great house.
            for h in [a, b] {
                self.houses[h].prestige = (self.houses[h].prestige - FEUD_ARBITRATE_PRESTIGE).max(0.0);
            }
            // The peace also lifts any market closure between them at that city.
            let c = ci as u32;
            for h in [a, b] {
                if let Some(v) = self.house_barred.get_mut(h) { v.retain(|&x| x != c); }
            }
            self.close_feud(fi, FEUD_ARBITRATED);
            let (an, bn) = (self.houses[a].name.clone(), self.houses[b].name.clone());
            let cn = self.hubs.get(ci).map(|h| h.name.clone()).unwrap_or_default();
            self.journal.push(JournalEntry {
                tick, kind: "feud".into(), hub: ci as i32, good: -1, value: owed,
                text: format!("The council of {} imposes a settlement on {} and {}", cn, an, bn),
            });
            for h in [a, b] {
                self.houses[h].events.push(HouseEvent {
                    tick, kind: "feud".into(),
                    text: format!("The council of {} settled our quarrel", cn),
                });
            }
        }
    }
}
