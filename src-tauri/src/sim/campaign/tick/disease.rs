//! disease — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

impl CampaignSim {

    /// Latest tick a plague quarantine at `hub` runs to (0 = none active). The hot
    /// paths (dispatch/fulfill) inline this into a per-tick lookup table; this
    /// single-hub form is kept for queries (UI / network routing).
    #[allow(dead_code)]
    pub(crate) fn quarantine_until(&self, hub: usize) -> u32 {
        self.active_events.iter()
            .filter(|e| e.kind == "plague_lockup" && e.hub == hub as i32 && e.until_tick > self.tick)
            .map(|e| e.until_tick).max().unwrap_or(0)
    }

    /// True while `hub` is locked up by plague (no trade in or out).
    #[allow(dead_code)]
    pub(crate) fn is_quarantined(&self, hub: usize) -> bool { self.quarantine_until(hub) > self.tick }


    /// Phase 5 (flavour) · strike a hub with plague: cull its population, quarantine
    /// it (a `plague_lockup` that suspends contracts + reroutes trade), and chronicle
    /// it. `carried_from` names the origin city for a contagion beat (route-borne
    /// spread) rather than a spontaneous outbreak.
    /// Strike a hub with a plague of the given `category` (1..3). `carried` is
    /// `Some((source_hub, outbreak, origin_hub))` for a contagion that travelled the
    /// lanes, or `None` for a spontaneous outbreak (which opens a fresh outbreak id and
    /// is its own origin). A city that is currently IMMUNE (survived a recent visitation)
    /// simply shrugs the strike off. Surviving a strike THEN confers lasting immunity,
    /// deeper the longer the lockdown lasts.
    pub(crate) fn strike_plague(&mut self, hub: usize, mag: f32, category: u8, disease: u8, carried: Option<(usize, u32, u32, u8)>) {
        if hub >= self.hubs.len() { return; }
        let tick = self.tick;
        // Immunity: a city that weathered a recent outbreak resists a new one entirely.
        if self.hubs[hub].plague_immune_until > tick { return; }
        // Small cities are less crowded/dense → 3× less likely to be struck at all
        // (user rule): shrug off 2 of every 3 strikes when under 10k people.
        if self.hubs[hub].population < SMALL_CITY_POP
            && hash01(self.seed, tick as u64 ^ 0x5A1717, hub as u64) > 1.0 / SMALL_CITY_PLAGUE_RESIST {
            return;
        }
        // Public health (hospices/quarantine) buys down the DEATH toll — the same people
        // still fall ill, but a well-provisioned city nurses more of them through.
        let ph = self.hubs[hub].public_health.clamp(0.0, HOSPICE_MAX_LEVEL);
        let base_mag = mag.clamp(0.0, 0.6);
        let mag_eff = (base_mag * (1.0 - ph)).clamp(0.0, 0.6);
        let pre = self.hubs[hub].population.max(0.0);
        self.hubs[hub].population *= 1.0 - mag_eff;
        let post = self.hubs[hub].population.max(0.0);
        // Phase 2c · the plague also ravages the surrounding COUNTRYSIDE: the struck
        // hub's province loses a slice of its rural reservoir (gated on a seeded
        // province layer, so the base economy / dynamics test is untouched). The rural
        // toll is lighter than the packed city's — fields are less crowded.
        if !self.prov_rural.is_empty() {
            if let Some(&pid) = self.hub_province.get(hub) {
                if pid >= 0 {
                    if let Some(r) = self.prov_rural.get_mut(pid as usize) {
                        *r = (*r * (1.0 - mag_eff * 0.5)).max(0.0);
                    }
                }
            }
        }
        // Lockdown (trade restriction) length scales with severity: a local outbreak is
        // a brief quarantine; a great plague shuts the gates for months.
        let jitter = hash01(self.seed, tick as u64 ^ 0x10CC, hub as u64);
        let lock = match category {
            1 => 90 + (jitter * 90.0) as u32,   // Great Plague: ~90-180 ticks
            2 => 45 + (jitter * 45.0) as u32,   // Regional: ~45-90
            _ => 18 + (jitter * 27.0) as u32,   // Local: ~18-45 (some trade restrictions)
        };
        self.active_events.push(ActiveEvent {
            kind: "plague_lockup".into(), hub: hub as i32, good: -1,
            magnitude: 1.0, until_tick: tick + lock,
        });
        // Surviving the visitation confers immunity for years afterward (longer for a
        // harsher, longer lockdown) — the city cannot be re-struck or re-seeded until then.
        // Immunity window scaled by the DISEASE (some, like flu/cholera, confer little
        // lasting immunity so they recur; smallpox/measles/plague confer strong immunity).
        let dimm = DISEASES.get(disease as usize).map(|s| s.immunity).unwrap_or(1.0);
        // Public health also lengthens the immunity earned (better convalescence + lasting
        // quarantine discipline) — a well-provisioned city resists the next visitation longer.
        let immune_span = ((PLAGUE_IMMUNITY_BASE_YEARS * TICKS_PER_YEAR as f32
            + lock as f32 * PLAGUE_IMMUNITY_LOCK_MULT) * dimm * (1.0 + ph)) as u32;
        self.hubs[hub].plague_immune_until = tick + lock + immune_span;
        // Observability record (Plagues panel + map). Spontaneous → a new outbreak;
        // contagion → inherits the source hub's outbreak id + origin + disease.
        let (source, outbreak, origin) = match carried {
            Some((src, ob, org, _dz)) => (src as i32, ob, org),
            None => { let ob = self.next_outbreak; self.next_outbreak += 1; (-1, ob, hub as u32) }
        };
        // SIR split (observability): infer how many fell ILL from the deaths and the
        // disease's case-fatality rate (its `dead_hi` reads as CFR-per-case). A mild
        // disease (low CFR) infects many and kills few; a lethal one infects nearer the
        // death toll. `recovered = infected − deaths` is derived by readers.
        let deaths = (pre - post).max(0.0);
        let cfr = DISEASES.get(disease as usize).map(|s| s.dead_hi).unwrap_or(0.5).clamp(0.03, 0.7);
        // `infected` reflects the UNMITIGATED attack (hospices cut deaths, not infections),
        // so a city with public health shows the same ill count but more recovered, fewer
        // dead — the visible payoff of its spending.
        let base_deaths = pre * base_mag;
        let infected = (base_deaths / cfr).clamp(deaths, pre * 0.9);
        self.epidemics.push(PlagueStrike {
            hub: hub as u32, source, outbreak, deaths,
            pop_at: post, start_tick: tick, until_tick: tick + lock,
            category, origin_hub: origin, disease, infected,
        });
        if self.epidemics.len() > 400 { let d = self.epidemics.len() - 400; self.epidemics.drain(0..d); }
        let city = self.hubs[hub].name.clone();
        let dname = DISEASES.get(disease as usize).map(|s| s.name).unwrap_or("sickness");
        let (kind, text) = match carried {
            Some((src, _, _, _)) => {
                let from = self.hubs.get(src).map(|h| h.name.clone()).unwrap_or_default();
                ("contagion".to_string(),
                    format!("{} reaches {}, carried by traders from {}.", dname, city, from))
            }
            None => ("disaster".to_string(),
                format!("{} breaks out in {}; the city is locked down under quarantine", dname, city)),
        };
        self.journal.push(JournalEntry {
            tick, kind, hub: hub as i32, good: -1, value: lock as f32, text });
    }


    /// The (category, outbreak, origin_hub, disease) of the outbreak currently live at
    /// a hub (its most recent unexpired strike), if any.
    pub(crate) fn active_strike_at(&self, hub: usize) -> Option<(u8, u32, u32, u8)> {
        let tick = self.tick;
        self.epidemics.iter().rev()
            .find(|s| s.hub as usize == hub && s.until_tick > tick)
            .map(|s| (if s.category == 0 { 3 } else { s.category }, s.outbreak, s.origin_hub, s.disease))
    }


    /// Phase 5 (flavour) · CONTAGION: a plague travels the TRADE NETWORK only — never
    /// jumps geographically. Each infected (quarantined) focus may, on a low per-tick
    /// chance (a plague is hard to carry), pass the pestilence to a trade partner its
    /// merchants actually reach:
    ///   · category 3 (LOCAL) never spreads;
    ///   · category 2 (REGIONAL) reaches at most ONE further city (a nearer partner);
    ///   · category 1 (GREAT PLAGUE) spreads city-to-city up to ~4000 km from origin.
    /// Immune cities (survivors) block the wave. Bounded: ≤2 new foci per tick, and a
    /// hard cap so the plague burns out before it locks a quarter of the world.
    pub(crate) fn spread_epidemics(&mut self) {
        use std::collections::HashSet;
        let tick = self.tick;
        let n = self.hubs.len();
        if n == 0 { return; }
        let locked: Vec<usize> = self.active_events.iter()
            .filter(|e| e.kind == "plague_lockup" && e.until_tick > tick && e.hub >= 0)
            .map(|e| e.hub as usize).filter(|&h| h < n).collect();
        if locked.is_empty() { return; }
        if locked.len() >= (n / 4).max(1) { return; } // hard cap → burns out
        let locked_set: HashSet<usize> = locked.iter().copied().collect();
        let km_per_cell = EARTH_EQUATOR_KM / self.world_w.max(1.0);
        let mut new_infections = 0usize;
        for &src in &locked {
            if new_infections >= EPIDEMIC_MAX_SPREAD_PER_TICK { break; }
            let (cat, outbreak, origin, disease) = match self.active_strike_at(src) { Some(x) => x, None => continue };
            let spec = &DISEASES[disease.min(8) as usize];
            // VECTOR diseases (malaria) never pass city-to-city — they re-emerge from
            // the land itself, so they don't travel here.
            if spec.mode == 3 || spec.spread <= 0.0 { continue; }
            if cat >= 3 && spec.mode != 2 { continue; } // local outbreaks stay put (airborne still drifts)
            if hash01(self.seed, tick as u64 ^ 0xC0FFEE, src as u64) >= spec.spread { continue; }
            // A regional outbreak only ever reaches ONE further city — once a contagion
            // child of this outbreak exists, it stops travelling.
            if cat == 2 && self.epidemics.iter().any(|s| s.outbreak == outbreak && s.source >= 0) {
                continue;
            }
            let reach_cells = spec.reach_km / km_per_cell.max(1e-3);
            let org = origin as usize;
            let comp = self.hubs[org.min(n - 1)].component;
            // OUTBREAK MEMORY: the set of cities THIS outbreak has ALREADY struck. A
            // contagion must never bounce back into a city it already passed through
            // (that inflated one 14-city plague into "186 cities" of re-records). When
            // the outbreak later dies out its records age away, so a FRESH outbreak (new
            // id) can strike those cities again — plague can recur, just not loop.
            let hit: HashSet<usize> = self.epidemics.iter()
                .filter(|s| s.outbreak == outbreak)
                .map(|s| s.hub as usize).collect();
            // Candidate destinations by TRANSMISSION MODE:
            //   trade (0): a trade-route neighbour (merchants carry it)
            //   water (1): a COASTAL/river-mouth trade neighbour (foul water)
            //   airborne (2): ANY nearby city (not only trade partners — it drifts)
            let mut best: (usize, f32) = (usize::MAX, f32::MAX);
            let mut consider = |me: &Self, h: usize, best: &mut (usize, f32)| {
                if h >= n || h == src || me.hubs[h].is_estate || me.hubs[h].abandoned { return; }
                if hit.contains(&h) { return; } // already visited by this outbreak — no loop-back
                if locked_set.contains(&h) || me.hubs[h].plague_immune_until > tick { return; }
                if me.hubs[h].component != comp { return; }
                if spec.mode == 1 && !me.hubs[h].coastal { return; } // water needs a wet city
                let dsrc = me.hub_cell_dist(src, h) * km_per_cell;
                if dsrc > PLAGUE_HOP_MAX_KM { return; }
                if org < n && me.hub_cell_dist(org, h) * km_per_cell > spec.reach_km { return; }
                if (dsrc / km_per_cell) < best.1 { *best = (h, dsrc / km_per_cell); }
            };
            if spec.mode == 2 {
                for h in 0..n { if self.hub_cell_dist(src, h) <= reach_cells { consider(self, h, &mut best); } }
            } else if let Some(v) = self.neighbors.get(src) {
                for &x in v { consider(self, x as usize, &mut best); }
            }
            // OVERLAND vector (Phase 2c): a plague also creeps across the COUNTRYSIDE from
            // one province into the next — so it reaches an adjacent province's city even
            // when the two aren't trade partners. Gated on a seeded province layer, and it
            // reuses every guard in `consider` (distance, immunity, outbreak memory, mode).
            if !self.prov_neighbors.is_empty() {
                let sp = self.hub_province.get(src).copied().unwrap_or(-1);
                if sp >= 0 {
                    if let Some(neigh) = self.prov_neighbors.get(sp as usize) {
                        if !neigh.is_empty() {
                            for h in 0..n {
                                let hp = self.hub_province.get(h).copied().unwrap_or(-1);
                                if hp >= 0 && neigh.contains(&(hp as u16)) {
                                    consider(self, h, &mut best);
                                }
                            }
                        }
                    }
                }
            }
            if best.0 != usize::MAX {
                self.strike_plague(best.0, EPIDEMIC_CONTAGION_MAG, cat, disease, Some((src, outbreak, origin, disease)));
                new_infections += 1;
            }
        }
    }


    /// Food balance per hub → estates & starvation.
    pub(crate) fn update_food_and_starvation(&mut self, needs: &[Vec<f32>]) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        // Busiest hub's realized trade this year — the reference for RELATIVE trade
        // development below, so the top trade cities always earn the full growth
        // headroom (and reach metropolis scale) regardless of the world's trade scale.
        let world_max_trade = self.hubs.iter()
            .filter(|h| !h.is_estate && !h.abandoned)
            .map(|h| h.trade_last_year)
            .fold(0.0f32, f32::max);
        // MEGACITY primacy: the top-treasury hub of each trade component is its political
        // CAPITAL. A capital that is also water-connected (coastal) and a real trade hub can
        // command tribute-grain (annona) by secured water lanes and grow toward a million.
        // Precompute each component's capital once (cheap; one per region → stays rare).
        let mut comp_capital: std::collections::HashMap<u32, (f32, usize)> =
            std::collections::HashMap::new();
        for i in 0..n {
            if self.hubs[i].is_estate || self.hubs[i].abandoned { continue; }
            let e = comp_capital.entry(self.hubs[i].component).or_insert((f32::MIN, usize::MAX));
            if self.hubs[i].treasury > e.0 { *e = (self.hubs[i].treasury, i); }
        }
        for h in 0..n {
            // A dead settlement stays dead — the founding-pop floor below must
            // never resurrect an abandoned ruin (or a collapsed colony).
            if self.hubs[h].abandoned { continue; }
            let mut food_need = 0.0;
            let mut food_have = 0.0;
            let mut food_prod = 0.0;
            for g in 0..ng {
                if self.goods[g].food {
                    food_need += needs[h][g];
                    food_prod += self.hubs[h].production[g];
                    food_have += self.hubs[h].stock[g] + self.hubs[h].production[g];
                }
            }
            // ── Settlement-colony food LIFELINE (dedicated supply ships) ─────────
            // A young colony can't feed itself: its metropolis runs a fleet of dedicated
            // SUPPLY SHIPS carrying real grain from a sufficient food SOURCE (a nearby
            // surplus city — often the metropolis). The ships' capacity, the source's
            // spare grain, and the backers' freight budget bound the delivery; a reserve
            // buffers brief breaks. Monthly, the metropolis re-picks the source and
            // INVESTS in more ships when the colony runs short (steady supply). Grain
            // physically leaves the source, so the source must genuinely be sufficient.
            // A metropolis-BOUND satellite (built or absorbed, kind 3, done building) is
            // fed by the SAME lifeline: its mother city keeps it supplied. This is what
            // lets a big city genuinely REVIVE a tiny town it has adopted — and stops a
            // finished satellite whose own farms fall short from quietly starving.
            let bound_satellite = self.hubs[h].colony_kind == 3
                && self.hubs[h].build_stage == 0 && self.hubs[h].founder_hub >= 0;
            if (self.hubs[h].colony_kind == 1 && !self.hubs[h].autonomous) || bound_satellite {
                self.hubs[h].reserve_cap = 365.0;
                let deficit = (food_need - food_prod).max(0.0); // daily grain shortfall
                // Monthly: re-designate the food source + top up the dedicated fleet.
                if self.tick % 30 == 0 {
                    self.designate_colony_supply(h, deficit * 30.0);
                }
                if deficit <= EPS {
                    // Self-sufficient: top the reserve, supply unbroken.
                    self.hubs[h].reserve_food = (self.hubs[h].reserve_food + 1.0).min(self.hubs[h].reserve_cap);
                    self.hubs[h].supply_years += 1.0 / TICKS_PER_YEAR as f32;
                    self.hubs[h].supply_delivered = 0.0;
                } else {
                    let fleet_daily = self.hubs[h].supply_ships as f32 * SUPPLY_SHIP_CAPACITY / 30.0;
                    let mut delivered = deficit.min(fleet_daily);
                    let src = self.hubs[h].supply_source;
                    // Real grain moves: pull from the source's spare food STOCK (kept above
                    // a buffer for its own residents), deducting it — no free food.
                    if delivered > EPS && src >= 0 && (src as usize) < n {
                        let s = src as usize;
                        let mut remaining = delivered;
                        for g in 0..ng {
                            if !self.goods[g].food || remaining <= EPS { continue; }
                            let buffer = self.hubs[s].production[g] * SOURCE_BUFFER_DAYS;
                            let spare = ((self.hubs[s].stock[g] - buffer) * SUPPLY_SOURCE_SPARE_FRAC).max(0.0);
                            let take = spare.min(remaining);
                            self.hubs[s].stock[g] -= take;
                            self.hubs[s].export_earn += take * self.goods[g].base_value;
                            remaining -= take;
                        }
                        delivered -= remaining; // only what the source could actually spare
                    } else {
                        delivered = 0.0;
                    }
                    // Backers pay the FREIGHT for the run (best-effort — the metropolis is
                    // committed to its colony, so a thin treasury doesn't cut the food off,
                    // it just runs the treasury down). This removes the old hard "can't
                    // pay → 0 food" gate that silently starved every colony.
                    let price = self.goods.iter().filter(|g| g.food).map(|g| g.base_value).fold(0.0, f32::max).max(1.0);
                    let freight = delivered * price * COLONY_FREIGHT_RATE;
                    let m = self.hubs[h].founder_hub;
                    if m >= 0 && (m as usize) < n {
                        let mi = m as usize;
                        let pay = self.hubs[mi].treasury.max(0.0).min(freight);
                        self.hubs[mi].treasury -= pay;
                        self.hubs[mi].finance.spent_works += pay;
                    }
                    // Deliver the grain into the colony's first food good.
                    if let Some(fg) = (0..ng).find(|&g| self.goods[g].food) {
                        self.hubs[h].stock[fg] += delivered;
                        food_have += delivered;
                    }
                    self.hubs[h].supply_delivered = delivered * 30.0; // monthly, for the readout
                    let short = deficit - delivered;
                    if short <= EPS {
                        self.hubs[h].reserve_food = (self.hubs[h].reserve_food + 0.5).min(self.hubs[h].reserve_cap);
                        self.hubs[h].supply_years += 1.0 / TICKS_PER_YEAR as f32;
                    } else if self.hubs[h].reserve_food > 0.0 {
                        // Eat into the reserve to stay fed; the supply record continues.
                        self.hubs[h].reserve_food -= 1.0;
                        if let Some(fg) = (0..ng).find(|&g| self.goods[g].food) {
                            self.hubs[h].stock[fg] += short;
                            food_have += short;
                        }
                        self.hubs[h].supply_years += 1.0 / TICKS_PER_YEAR as f32;
                    } else {
                        // Lifeline snapped: reserve empty AND supply short → the record
                        // breaks and the colony starves (handled below by food balance).
                        self.hubs[h].supply_years = 0.0;
                    }
                }
            }
            let bal = if food_need > EPS {
                (food_have - food_need) / food_need
            } else {
                1.0
            };
            // Smooth.
            self.hubs[h].food_balance = 0.85 * self.hubs[h].food_balance + 0.15 * bal;
            let fb = self.hubs[h].food_balance;
            // Starvation pressure builds when food balance is negative.
            if fb < 0.0 {
                self.hubs[h].starving = (self.hubs[h].starving + 0.02 * (-fb).min(1.0)).min(1.0);
            } else {
                self.hubs[h].starving = (self.hubs[h].starving - 0.02).max(0.0);
            }
            // Population: logistic growth toward a CARRYING CAPACITY set by both
            // FOOD security and TRADE prosperity. Well-fed, well-connected trade
            // hubs grow well above their founding size; food-poor or commercially
            // isolated settlements stagnate or shrink back. Uses the eased
            // sentiment drivers (already normalized 0..1) as the food/trade signal,
            // so the capacity tracks the same numbers the settlement window shows.
            let pop = self.hubs[h].population;
            let food_sec = self.hubs[h].sent_food.clamp(0.0, 1.0); // 1 = well fed
            let prosperity = self.hubs[h].sent_prosperity.clamp(0.0, 1.0); // trade+grain wealth
            // Capacity in multiples of the founding population. FOOD security and
            // TRADE prosperity gate the base; realized trade throughput adds a large,
            // slowly-EARNED headroom on top (`trade_dev`) so a hub that becomes a busy
            // entrepôt keeps growing into a metropolis instead of freezing at a fixed
            // ~9× of its humble founding size (the old ceiling that stalled the world
            // at ~3.2M). An isolated/low-trade hub earns little headroom and stays
            // small — exactly the historical pattern (arid inland town vs. great port).
            // RELATIVE trade eminence: a hub's share of the busiest hub's throughput,
            // scaled to the full headroom cap. The world's top entrepôt earns the whole
            // `TRADE_DEV_CAP`; lesser hubs earn proportionally less. This guarantees the
            // leading trade cities reach metropolis scale (≥150k for a large-founding hub)
            // no matter the absolute trade volume, instead of stalling when trade is thin.
            let trade_dev = if world_max_trade > EPS {
                TRADE_DEV_CAP * (self.hubs[h].trade_last_year / world_max_trade).clamp(0.0, 1.0)
            } else { 0.0 };
            // MEGACITY primacy headroom: only for the regional CAPITAL (top treasury in its
            // component) that is water-connected AND a real trade hub — it commands grain and
            // can approach a million. Everyone else gets 0. The food_sec factor above still
            // gates it, so an unfed capital can't balloon — it must actually be provisioned.
            let primacy_dev = if self.hubs[h].coastal && self.hubs[h].hub_class >= 1
                && comp_capital.get(&self.hubs[h].component).map(|&(_, i)| i == h).unwrap_or(false) {
                PRIMACY_DEV
            } else { 0.0 };
            let cap_mult = (0.35 + 2.0 * food_sec)
                * (0.60 + 5.0 * prosperity * prosperity + trade_dev + primacy_dev);
            let capacity = (self.hubs[h].founding_pop * cap_mult)
                .max(self.hubs[h].founding_pop * 0.15);
            // Logistic step: approach capacity from below, decline when above it.
            // Slower organic growth (~5%/yr peak at low pop, was ~24%). Young
            // SETTLEMENT colonies grow faster (frontier boom + sponsored migration on
            // top) so they still mature into cities within a campaign.
            let colony_boom = if self.hubs[h].colony_kind == 1 && !self.hubs[h].autonomous { POP_GROWTH_COLONY_MULT } else { 1.0 };
            // Small-city growth bonus (user rule): a well-fed town under 10k grows up
            // to 5× faster so humble settlements rise into cities — scaled by food
            // security (needs food on-site or traded in), no benefit when starving.
            let small_boost = if pop < SMALL_CITY_POP {
                1.0 + (SMALL_CITY_GROWTH_MULT - 1.0) * food_sec
            } else { 1.0 };
            // Trade-rich + well-fed cities of ANY size grow faster (user rule: up to
            // ~10%/yr) — a thriving, food-secure entrepôt booms; a poor/hungry one crawls.
            let trade_food_boost = 1.0 + TRADE_FOOD_GROWTH_BONUS * prosperity * food_sec;
            let rate = if pop < capacity { POP_GROWTH_RATE * colony_boom * small_boost * trade_food_boost } else { POP_DECLINE_RATE };
            let mut new_pop = pop + rate * pop * (1.0 - pop / capacity);
            // Net demographic drift (births − deaths): a well-fed populace has a small
            // birth surplus so the TOTAL world can grow (migration alone only reshuffles
            // a fixed pie). Applied only BELOW capacity and damped by remaining headroom
            // so it can't overshoot — total population stays bounded and finite.
            if pop < capacity {
                let net = BIRTH_RATE * food_sec - DEATH_RATE_BASE;
                new_pop += net * pop * (1.0 - pop / capacity);
            }
            // Famine empties a city faster than trade decline alone.
            if self.hubs[h].starving > 0.5 {
                new_pop *= 1.0 - 0.0016 * (self.hubs[h].starving - 0.5);
                if self.tick % 90 == 0 {
                    self.journal.push(JournalEntry {
                        tick: self.tick,
                        kind: "starvation".into(),
                        hub: h as i32,
                        good: -1,
                        value: self.hubs[h].starving,
                        text: format!("{} suffers famine; people leave", self.hubs[h].name),
                    });
                }
            }
            let mut np = new_pop.max(self.hubs[h].founding_pop * 0.10);
            // A house trade outpost stays a small trade post — hard population cap.
            if self.hubs[h].colony_kind == 2 { np = np.min(OUTPOST_MAX_POP); }
            self.hubs[h].population = np;
        }
        // Resilience: after a tick panic the crash-recovery layer freezes territorial
        // expansion for a while (`expansion_frozen_until`) so re-advancing can't re-hit
        // the same founding fault. Everything else keeps simulating.
        let expansion_ok = self.tick >= self.expansion_frozen_until;
        // Estate founding: a big, rich, food-secure hub with a hungry neighbour
        // founds a food estate. At most one per advance batch (cheap, rare).
        if expansion_ok && self.tick % 120 == 0 {
            self.maybe_found_estate();
        }
        // Colonization of new land: rarer (yearly) — the settled map fills in.
        // Two distinct drives: a great house plants a remote trade OUTPOST (strategic
        // reach / new goods), and an overcrowded city founds a full SETTLEMENT colony
        // (population expansion) that can grow into a city of its own. The age of
        // colonisation only opens once the world has matured — from YEAR 50 onward,
        // and only when the wealth/population/site conditions are actually met.
        // Expeditions crawl toward distant lands EVERY tick (cheap when none active),
        // rolling hazards and, once a route is proven, establishing a corridor.
        self.expedition_travel_pass();
        // Cultures 2.0 · sample every people's population twice a year for the line chart.
        if self.tick % (TICKS_PER_YEAR / 2) == 0 {
            self.sample_culture_history();
        }
        if self.tick % 365 == 0 {
            // Phase 2b · watershed demography: the province countryside grows and feeds
            // its cities (migration carries culture); big cities pay a graveyard mortality.
            // No-op unless a province layer was seeded into the campaign.
            self.province_demography_pass();
            // Yearly social mobility: strata shift with prosperity / hardship.
            self.update_society();
            // Then the people may stir: unrest builds, riots flare, revolts topple
            // councils (reads the freshly-updated inequality / welfare above).
            self.update_unrest();
            // Estate/manufactory disasters strike + funded repairs progress (yearly).
            self.estate_condition_pass();
            self.manufactory_solvency_pass(); // shut works idle 4+ years
            // Atlas 2.0 — the LIVING MAP: organic settlement death & birth.
            self.lifecycle_pass(expansion_ok);
            // …and REBIRTH: long-dead ruins in recovered regions are resettled.
            self.resettle_pass(expansion_ok);
            // #23 · Peoples: seed/settle cultures, drift people toward opportunity
            // (draws migration arrows), and slowly assimilate minority quarters.
            self.ensure_hub_cultures();
            self.economic_migration_pass();
            self.diaspora_pass();
            // Which trade tongue dominates each region (drives the assimilation bridge).
            self.compute_lingua();
            self.assimilation_pass();
            // Whoever now holds the plurality becomes the city's majority people.
            self.rebalance_hub_majorities();
            // Cultures 2.0 · sustained blending in a city can birth a new creole people.
            self.ethnogenesis_pass(self.tick / TICKS_PER_YEAR);
            // Cultures 3.0 · a far-flung, isolated community can splinter into a new
            // daughter people of the same stock.
            self.splinter_pass(self.tick / TICKS_PER_YEAR);
            // Connect sub-cap villages to the trade network via their nearest market town.
            self.hinterland_pass();
            // Rare merchant expeditions reach the far, isolated outposts trade misses.
            self.expedition_pass(self.tick / TICKS_PER_YEAR);
            // Financed EXPEDITIONS toward distant unconnected cities — the way a
            // permanent trade corridor is earned (hazards, failed attempts, then
            // port/caravanserai villages). See docs/EXPEDITIONS_CORRIDORS_PLAN.md.
            self.expedition_launch_pass(expansion_ok);
            // House trade outposts from year 30 (rich house, heavy cost); full
            // settlement colonies from year 50 (joint-stock, food lifeline).
            if expansion_ok && self.tick >= BASE_START_TICK {
                self.maybe_establish_trade_base();
                self.trade_base_pass();
            }
            if expansion_ok && self.tick >= OUTPOST_START_TICK {
                self.maybe_found_house_outpost();
                self.maybe_graduate_outpost(); // a thriving old outpost matures into a colony
            }
            if expansion_ok && self.tick >= COLONY_START_TICK {
                self.maybe_found_settlement_colony();
                self.maybe_found_food_colony(); // Greek-Crimea grain colony (food stress)
                self.colony_pass(); // graduation · dividends · autonomy
                self.maybe_found_satellite(expansion_ok); // port/granary/workshop suburbs
                self.maybe_absorb_dying_city();            // big city adopts a failing neighbour
                self.maybe_found_caravanserai(expansion_ok); // waystations on long land ties
                self.satellite_independence_pass();        // mature satellites → free cities
            }
        }
    }


    // ── Expeditions & Corridors ──────────────────────────────────────────────
    // (docs/EXPEDITIONS_CORRIDORS_PLAN.md) A corridor is EARNED: a house finances
    // a venture toward a distant unconnected city; hazards cull it; several proven
    // round-trips establish the route + found port/caravanserai villages.

    /// Climate hostility 0..1 of a Köppen zone — the deadly ground a venture crosses.
    pub(crate) fn koppen_peril(k: u8) -> f32 {
        use crate::sim::koppen::*;
        match k {
            BWH | BWK => 0.90,           // desert — heat, thirst
            ET | EF => 0.85,             // polar — cold
            AF | AM | AW => 0.60,        // tropical — disease
            BSH | BSK => 0.45,           // steppe — raiders
            _ => 0.20,
        }
    }


    /// Which peril struck (0 illness·1 climate·2 raid·3 storm·4 wreck·5 starvation·6 bandits).
    pub(crate) fn pick_hazard_kind(sea: bool, ko: u8, kd: u8, r: f32) -> u8 {
        use crate::sim::koppen::*;
        if sea { return if r < 0.5 { 3 } else { 4 }; }
        let desert = matches!(ko, BWH | BWK) || matches!(kd, BWH | BWK);
        let tropical = matches!(ko, AF | AM | AW) || matches!(kd, AF | AM | AW);
        if desert { if r < 0.5 { 1 } else { 5 } }
        else if tropical { if r < 0.6 { 0 } else { 2 } }
        else if r < 0.4 { 2 } else if r < 0.7 { 6 } else { 1 }
    }
}
