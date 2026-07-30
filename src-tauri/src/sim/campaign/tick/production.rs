//! production — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

impl CampaignSim {

    // ───────────────────────── DLC 3.5 · Coin, Credit & Crashes ──────────────

    /// DLC 3.5 · accrue shipped volume on a hub-pair for the yearly Dynamic Trade
    /// Flow snapshot (keyed by stable hub IDs, ordered low→high; direction-agnostic).
    /// `good` also feeds the per-hub PER-GOOD ledger (Batch 1: per-good Trade Heat,
    /// basin top-goods); pass `usize::MAX` for goods-less flavour flow (fairs,
    /// pilgrimages) to skip that ledger.
    #[inline]
    pub(crate) fn accrue_flow(&mut self, from: usize, to: usize, good: usize, amount: f32) {
        if amount <= 0.0 || from >= self.hubs.len() || to >= self.hubs.len() || from == to { return; }
        let (ia, ib) = (self.hubs[from].id, self.hubs[to].id);
        let key = if ia <= ib { (ia, ib) } else { (ib, ia) };
        *self.flow_accum.entry(key).or_insert(0.0) += amount;
        if good != usize::MAX && good < self.goods.len() {
            let ng = self.goods.len();
            let need = self.hubs.len() * ng;
            if self.good_flow_accum.len() < need { self.good_flow_accum.resize(need, 0.0); }
            self.good_flow_accum[from * ng + good] += amount;
            self.good_flow_accum[to * ng + good] += amount;
        }
    }


    /// Recent trade volume touching a hub (the decaying per-class tallies) — used
    /// as the coinage "throughput" weight and the seigniorage base.
    pub(crate) fn hub_throughput(&self, h: usize) -> f32 {
        let hb = &self.hubs[h];
        hb.tw_house + hb.tw_local + hb.tw_guild
    }


    /// Whether a hub is currently inside a regional financial panic.
    pub(crate) fn hub_in_panic(&self, h: usize) -> bool {
        let tick = self.tick;
        self.active_events.iter()
            .any(|e| e.kind == "panic" && e.hub == h as i32 && e.until_tick > tick)
    }


    /// DLC 4 · learning-by-doing: each month every MANUFACTORY/city drifts the
    /// quality of the manufactured goods it makes toward a skill cap set by its size
    /// and its craft buildings (guildhall/workshop). "Manufacturers start producing
    /// higher quality goods" — and big skilled cities reach finer grades.
    pub(crate) fn update_good_quality(&mut self) {
        let ng = self.goods.len();
        for h in 0..self.hubs.len() {
            if self.hubs[h].quality.len() != ng { continue; }
            let pop = self.hubs[h].population.max(1.0);
            let size_bonus = (pop / 60_000.0).min(0.20);
            let mut struct_bonus = 0.0;
            if self.hub_has_struct(h, STRUCT_WORKSHOP) { struct_bonus += 0.08; }
            if self.hub_has_struct(h, STRUCT_GUILDHALL) { struct_bonus += 0.06; }
            let manu_estate = self.hubs[h].is_estate && self.hubs[h].estate_kind == 6;
            for g in 0..ng {
                let manufactured = !self.goods[g].inputs.is_empty();
                if !(manufactured || manu_estate) { continue; }
                if self.hubs[h].production.get(g).copied().unwrap_or(0.0) <= 0.0
                    && self.hubs[h].quality[g] <= 0.0 { continue; }
                let cap = (0.62 + size_bonus + struct_bonus).clamp(0.0, 0.97);
                let q = self.hubs[h].quality[g];
                if q < cap {
                    self.hubs[h].quality[g] = (q + (cap - q) * QUALITY_LEARN_RATE).min(cap);
                }
            }
        }
    }


    /// DLC 4 · industrial espionage: once a year a house running a manufactory may
    /// STEAL the technique of the world's finest maker of a good it produces,
    /// jumping its own quality toward the leader's. Recorded on the thief's hub
    /// (`stolen_good`/`stolen_from`) and journaled — visible in the estate view.
    pub(crate) fn maybe_steal_quality(&mut self, yr: u32) {
        let ng = self.goods.len();
        // World-leading quality + its hub, per good (producers only).
        let mut best_q = vec![0.0f32; ng];
        let mut best_hub = vec![usize::MAX; ng];
        for h in 0..self.hubs.len() {
            if self.hubs[h].quality.len() != ng { continue; }
            for g in 0..ng {
                if self.hubs[h].production.get(g).copied().unwrap_or(0.0) <= 0.0 { continue; }
                if self.hubs[h].quality[g] > best_q[g] { best_q[g] = self.hubs[h].quality[g]; best_hub[g] = h; }
            }
        }
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            // Spies are the cunning sorts: specialist / political / banking houses.
            if self.houses[hi].archetype == ARCH_FLEET { continue; }
            if hash01(self.seed, (hi as u64) ^ 0x57EA1, yr as u64) > QUALITY_STEAL_CHANCE { continue; }
            // A manufactory this house owns, making a good a rival makes far better.
            let mine: Option<(usize, usize)> = self.hubs.iter().enumerate()
                .filter(|(_, e)| e.is_estate && e.owner_house == hi as i32 && e.estate_kind == 6
                    && e.quality.len() == ng)
                .filter_map(|(ei, e)| {
                    let g = (0..ng).find(|&g| e.production.get(g).copied().unwrap_or(0.0) > 0.0)?;
                    Some((ei, g))
                })
                .find(|&(ei, g)| best_hub[g] != usize::MAX && best_hub[g] != ei
                    && best_q[g] - self.hubs[ei].quality.get(g).copied().unwrap_or(0.0) > 0.12);
            let Some((ei, g)) = mine else { continue };
            let leader = best_hub[g];
            let gain = (best_q[g] - self.hubs[ei].quality[g]) * QUALITY_STEAL_FRAC;
            self.hubs[ei].quality[g] += gain;
            self.hubs[ei].stolen_good = g as i32;
            self.hubs[ei].stolen_from = self.hubs[leader].id as i32;
            let (hn, gn, victim) = (self.houses[hi].name.clone(),
                self.goods[g].name.clone(), self.hubs[leader].name.clone());
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "espionage".into(), hub: ei as i32, good: g as i32, value: gain,
                text: format!("{} steals the secret of {} from {}", hn, gn, victim),
            });
            self.houses[hi].events.push(HouseEvent {
                tick: self.tick, kind: "espionage".into(),
                text: format!("stole the {} craft from {}", gn, victim),
            });
        }
    }


    pub fn rebuild_routes(&mut self) {
        let n = self.hubs.len();
        let bn = self.base_n;
        // The precomputed PATHFOUND matrix (real lanes over the trade-route cost grid) is
        // preferred for the founding hubs; hubs added later (colonies, index ≥ bn) and any
        // pair the pathfinder couldn't connect fall back to the straight-line estimate.
        let have_base = bn > 0 && self.base_days.len() == bn * bn;
        let mut days = vec![f32::INFINITY; n * n];
        for a in 0..n {
            days[a * n + a] = 0.0;
            for b in (a + 1)..n {
                let d = if have_base && a < bn && b < bn && self.base_days[a * bn + b].is_finite() {
                    // Pathfinder found a real route (possibly across continents via sea).
                    self.base_days[a * bn + b]
                } else if self.hubs[a].component == self.hubs[b].component {
                    // Same geographic component but no pathfound route: straight-line fallback.
                    let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
                    if self.world_w > 1.0 {
                        dx = dx.min(self.world_w - dx);
                    }
                    let dy = self.hubs[a].y - self.hubs[b].y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    (dist * self.days_per_cell).max(1.0)
                } else {
                    // Different components AND no pathfound sea route: unreachable.
                    continue;
                };
                days[a * n + b] = d;
                days[b * n + a] = d;
            }
        }
        self.days = days;
        self.rebuild_neighbors();
        self.routes_dirty = false;
    }


    /// Build each hub's nearest reachable trade partners (sorted nearest first,
    /// capped to `NEIGHBOR_K`). Estates are kept as candidates (they have a
    /// population that must still import food); the cap simply means dispatch
    /// never scans far-flung hubs, which is where the late-campaign cost went.
    /// Trade GRAVITY of a hub (≥ 1): big / high-class markets pull trade from farther
    /// afield. A hub's EFFECTIVE distance to others is real distance ÷ this, so a great
    /// entrepôt appears "nearer" and enters the partner lists of cities much farther away,
    /// and wins more merchant dispatch — the "large trade hubs attract trade from afar and
    /// are more attractive to merchants" rule. Ordinary towns sit at ~1 (no distortion).
    #[inline]
    pub(crate) fn hub_pull(&self, b: usize) -> f32 {
        let h = &self.hubs[b];
        if h.is_estate || h.abandoned { return 1.0; }
        let by_class = HUB_PULL_CLASS * h.hub_class as f32;
        let by_pop = (h.population / HUB_PULL_POP_REF).clamp(0.0, 1.0);
        (1.0 + by_class + by_pop).clamp(1.0, HUB_PULL_MAX)
    }


    #[inline]
    pub(crate) fn live_price(&self, stock: f32, need: f32, base: f32) -> f32 {
        (base * ((need + EPS) / (stock + EPS)).powf(self.k))
            .clamp(base * PRICE_FLOOR_MULT, base * PRICE_CEIL_MULT)
    }


    /// Aggregate inventory of good `g` available AT hub `h` for pricing & needs:
    /// the local-merchant pool (stored inline on the hub) PLUS every house/guild
    /// warehouse sited here. While `warehouses` is empty this equals the old
    /// `hubs[h].stock[g]`, so behaviour is unchanged until house depots exist.
    #[inline]
    pub fn hub_stock(&self, h: usize, g: usize) -> f32 {
        let mut s = self.hubs[h].stock.get(g).copied().unwrap_or(0.0);
        for w in &self.warehouses {
            if w.hub as usize == h {
                s += w.stock.get(g).copied().unwrap_or(0.0);
            }
        }
        s
    }


    /// Freight to haul one unit of good `g` over `days` at an already-discounted
    /// per-day `rate`: bulky goods cost more, perishable goods accrue spoilage.
    /// A 0 bulk (old saves) is treated as 1.0, so freight is unchanged for them.
    #[inline]
    pub(crate) fn good_freight(&self, g: usize, rate: f32, days: f32) -> f32 {
        let bulk = { let b = self.goods[g].bulk; if b <= 0.0 { 1.0 } else { b } };
        rate * days * bulk + self.goods[g].perishable.max(0.0) * days
    }


    /// Turn each hub's input STOCK into finished `Manufactured` goods, scaled by
    /// labor capacity (∝ population). Mirrors the worldgen `apply_manufacturing`
    /// pass so the living economy and the static trade map agree. Manufactured
    /// goods are ordered raws-first so multi-stage chains resolve; cycles are
    /// skipped (a good that never reaches depth is left unmade).
    pub(crate) fn manufacture_pass(&mut self) {
        let ng = self.goods.len();
        // Manufactured goods = those carrying a recipe.
        let recipe_goods: Vec<usize> = (0..ng).filter(|&g| !self.goods[g].inputs.is_empty()).collect();
        if recipe_goods.is_empty() {
            return;
        }
        // Depth = longest chain of manufactured inputs feeding this good; raws-first
        // order. Iterative relaxation (ng is small); leftover at -1 = a cycle, skip.
        let is_recipe = |g: usize| !self.goods[g].inputs.is_empty();
        let mut depth: Vec<i32> = vec![-1; ng];
        for _pass in 0..recipe_goods.len() + 1 {
            let mut changed = false;
            for &g in &recipe_goods {
                let mut d = 0;
                let mut ready = true;
                for &(idx, _) in &self.goods[g].inputs {
                    if idx < ng && is_recipe(idx) {
                        if depth[idx] < 0 { ready = false; break; }
                        d = d.max(depth[idx] + 1);
                    }
                }
                if ready && depth[g] != d {
                    depth[g] = d;
                    changed = true;
                }
            }
            if !changed { break; }
        }
        let mut order: Vec<usize> = recipe_goods.iter().copied().filter(|&g| depth[g] >= 0).collect();
        order.sort_by_key(|&g| (depth[g], g));

        // Median population → labor scale (big cities out-make villages).
        let mut pops: Vec<f32> = self.hubs.iter().map(|h| h.population.max(0.0)).collect();
        pops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_pop = if pops.is_empty() { 1.0 } else { pops[pops.len() / 2].max(1.0) };

        // Fungible input substitutes (bay salt ↔ rock salt as a preservative cure).
        // Mirrors worldgen `manufacture::apply_manufacturing`; narrow by design so
        // metals/fibres never swap as structural inputs.
        let subs: std::collections::HashMap<usize, Vec<usize>> = {
            let mut m: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for g in 0..ng {
                if !self.goods[g].fungible_input || self.goods[g].category == i32::MAX { continue; }
                let cat = self.goods[g].category;
                let sibs: Vec<usize> = (0..ng)
                    .filter(|&j| j != g && self.goods[j].fungible_input && self.goods[j].category == cat)
                    .collect();
                if !sibs.is_empty() { m.insert(g, sibs); }
            }
            m
        };

        for h in 0..self.hubs.len() {
            let pop = self.hubs[h].population.max(0.0);
            for &g in &order {
                let labor = { let l = self.goods[g].labor; if l <= 0.0 { 1.0 } else { l } };
                let labor_cap = (pop / median_pop) * labor;
                if labor_cap <= 0.0 { continue; }
                let mut by_inputs = f32::INFINITY;
                for &(idx, qty) in &self.goods[g].inputs {
                    if qty <= 0.0 || idx >= ng { continue; }
                    let mut avail = self.hubs[h].stock[idx];
                    if let Some(sl) = subs.get(&idx) { for &s in sl { avail += self.hubs[h].stock[s]; } }
                    by_inputs = by_inputs.min(avail / qty);
                }
                if !by_inputs.is_finite() || by_inputs <= 0.0 { continue; }
                let made = by_inputs.min(labor_cap);
                if made <= 0.0 { continue; }
                // Clone inputs to avoid borrow conflict while mutating stock.
                let inputs = self.goods[g].inputs.clone();
                for (idx, qty) in inputs {
                    if idx >= ng { continue; }
                    let mut need = made * qty;
                    let take = self.hubs[h].stock[idx].min(need);
                    self.hubs[h].stock[idx] -= take;
                    need -= take;
                    if need > 0.0 {
                        if let Some(sl) = subs.get(&idx) {
                            for &s in sl {
                                if need <= 0.0 { break; }
                                let t = self.hubs[h].stock[s].min(need);
                                self.hubs[h].stock[s] -= t;
                                need -= t;
                            }
                        }
                    }
                }
                self.hubs[h].stock[g] += made;
                self.hubs[h].production[g] += made;
            }
        }
    }


    /// Add manufacturing (derived) demand for recipe INPUTS onto the needs table,
    /// so dispatch carries raw wool/iron/sugar into the cities able to work them.
    /// Demand scales with each city's labour capacity (∝ population) — big cities
    /// pull more inputs and so become the manufacturing centres.
    pub(crate) fn add_manufacturing_demand(&mut self, needs: &mut [Vec<f32>]) {
        let ng = self.goods.len();
        let recipe_goods: Vec<usize> = (0..ng).filter(|&g| !self.goods[g].inputs.is_empty()).collect();
        if recipe_goods.is_empty() { return; }
        let mut pops: Vec<f32> = self.hubs.iter().map(|h| h.population.max(0.0)).collect();
        pops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = pops.get(pops.len() / 2).copied().unwrap_or(1.0).max(1.0);
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; } // manufacturing happens in cities
            let cap = (self.hubs[h].population.max(0.0) / median).min(8.0);
            if cap <= 0.0 { continue; }
            for &g in &recipe_goods {
                let labor = { let l = self.goods[g].labor; if l <= 0.0 { 1.0 } else { l } };
                for &(idx, qty) in &self.goods[g].inputs {
                    if idx < ng && qty > 0.0 { needs[h][idx] += cap * labor * qty * MANUFACTURE_PULL; }
                }
            }
        }
    }


    pub(crate) fn base_need(&self, h: usize, g: usize) -> f32 {
        let tg = &self.goods[g];
        // Demand cadence: a good consumed every N days exerts ~30/N of the daily
        // pull of a monthly good. Clamped so it modulates (not dominates) — long
        // cadence goods (furs, luxuries) sit cheaper locally and skew to wholesale.
        let interval = if tg.consumption_interval > 0.0 { tg.consumption_interval } else { 30.0 };
        let cadence = (30.0 / interval).clamp(0.30, 1.8);
        // Craving for FOREIGN luxuries (#2): a luxury-tier good a city can't make at
        // home is coveted the more — and the finer/dearer it is, the more so — so
        // distant high-value luxuries pull strong import demand and feed inter-city
        // trade, instead of every city resting on its own produce.
        let mut foreign_lux = 1.0;
        if tg.need_tier >= 2 {
            let local = self.hubs[h].base_per_capita.get(g).copied().unwrap_or(0.0);
            if local < 1e-4 {
                let prestige = (tg.base_value / 15.0).clamp(0.4, 1.6);
                foreign_lux = 1.0 + LUX_IMPORT_DESIRE * prestige;
            }
        }
        self.hubs[h].population
            * TIER_WEIGHT[tg.need_tier.min(2) as usize]
            * tg.desire.max(0.0)
            * cadence
            * foreign_lux
            * self.society_demand_mult(h, tg.need_tier)
            * self.need_scale
            * DEMAND_PRESSURE
    }


    /// Hub latitude as a signed fraction: +1 = north pole, 0 = equator, −1 =
    /// south pole. In the equirectangular world `y=0` is the north edge and
    /// `y=world_h` the south edge. Returns 0 when world height is unknown
    /// (old saves) so seasonality degrades to a single global hemisphere.
    #[inline]
    pub(crate) fn hub_lat_frac(&self, h: usize) -> f32 {
        if self.world_h > 1.0 {
            (1.0 - 2.0 * self.hubs[h].y / self.world_h).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }


    /// Phase G: a house barred from a market PAYS the city to regain its trading
    /// rights (one market a month, when it can afford the fee). The fee scales with
    /// the city's size, flows into the city's civic_pool (reaching the people), and
    /// is recorded on the Accountant's misfortune line.
    pub(crate) fn pay_to_regain_markets(&mut self) {
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct {
                continue;
            }
            let city = match self.house_barred.get(hi).and_then(|v| v.first().copied()) {
                Some(c) => c,
                None => continue,
            };
            let fee = self
                .hubs
                .get(city as usize)
                .map(|h| (h.population / 5000.0).clamp(2.0, 40.0))
                .unwrap_or(5.0);
            if self.houses[hi].wealth > fee * 2.0 {
                self.houses[hi].wealth -= fee;
                if let Some(hb) = self.hubs.get_mut(city as usize) {
                    hb.civic_pool += fee;
                }
                if let Some(v) = self.house_barred.get_mut(hi) {
                    v.retain(|&c| c != city);
                }
                if hi < self.house_ledger.len() {
                    self.house_ledger[hi].events += fee;
                }
            }
        }
    }


    /// Push one weekly history sample per hub (capped to the last ~5 years) for the
    /// settlement-window charts.
    pub(crate) fn sample_hub_history(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        for hb in &mut self.hubs {
            let mut idx = 0.0f32;
            if ng > 0 {
                let mut s = 0.0f32;
                for g in 0..ng {
                    s += hb.price[g] / self.goods[g].base_value.max(EPS);
                }
                idx = s / ng as f32;
            }
            let (pop_house, pop_local, pop_guild) = merchant_pops(hb);
            hb.history.push(HubSample {
                tick,
                population: hb.population,
                wealth: hb.grain_wealth + hb.trade_wealth,
                mood: hb.mood,
                price_index: idx,
                lack_basic: hb.lack_basic,
                lack_comfort: hb.lack_comfort,
                lack_luxury: hb.lack_luxury,
                pop_house,
                pop_local,
                pop_guild,
            });
            // Monthly samples → keep ~30 years of history.
            if hb.history.len() > 360 {
                let drop = hb.history.len() - 360;
                hb.history.drain(0..drop);
            }
        }
    }


    /// Per-hub per-good production multiplier from active events (drought/blight…).
    /// Fill `m` (reused across ticks) with per-hub/good production multipliers from
    /// active events — default 1.0, dented by drought/blight/etc. Resizing + resetting
    /// in place avoids reallocating an n×ng matrix every single tick.
    pub(crate) fn fill_event_production_mult(&self, m: &mut Vec<Vec<f32>>) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        m.resize(n, Vec::new());
        for row in m.iter_mut() {
            row.clear();
            row.resize(ng, 1.0);
        }
        for e in &self.active_events {
            match e.kind.as_str() {
                "drought" | "blight" | "fishery_collapse" => {
                    // Regional: affect hubs within a radius of the event hub.
                    let center = if e.hub >= 0 { e.hub as usize } else { continue };
                    let (cx, cy) = (self.hubs[center].x, self.hubs[center].y);
                    for h in 0..n {
                        let mut dx = (self.hubs[h].x - cx).abs();
                        if self.world_w > 1.0 {
                            dx = dx.min(self.world_w - dx);
                        }
                        let dy = self.hubs[h].y - cy;
                        if (dx * dx + dy * dy).sqrt() < self.world_w * 0.12 {
                            for g in 0..ng {
                                let hit = match e.kind.as_str() {
                                    "drought" | "blight" => self.goods[g].food,
                                    "fishery_collapse" => {
                                        self.goods[g].name.contains("fish")
                                            || self.goods[g].name.contains("herring")
                                            || self.goods[g].name.contains("whal")
                                    }
                                    _ => false,
                                };
                                if hit {
                                    m[h][g] *= 1.0 - e.magnitude;
                                }
                            }
                        }
                    }
                }
                "embargo" | "guild_strike" => {
                    // A trade embargo, or a craft guild downing tools — the good
                    // stops being made at that hub for the duration.
                    if e.hub >= 0 && e.good >= 0 {
                        m[e.hub as usize][e.good as usize] *= 1.0 - e.magnitude;
                    }
                }
                "riot" | "revolt" => {
                    // Civil disorder halts the workshops & wharves city-wide.
                    if e.hub >= 0 {
                        let c = e.hub as usize;
                        for g in 0..ng { m[c][g] *= 1.0 - e.magnitude; }
                    }
                }
                "bumper" => {
                    // Exceptional harvest: production surges at the hub (+mag), so
                    // its goods grow plentiful and cheap.
                    if let Some(center) = (e.hub >= 0).then_some(e.hub as usize) {
                        for g in 0..ng {
                            if self.goods[g].food || self.goods[g].name.contains("wine")
                                || self.goods[g].name.contains("oil") {
                                m[center][g] *= 1.0 + e.magnitude;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        // Floor the cumulative penalty: overlapping shocks (e.g. several droughts
        // covering one clustered region) used to stack multiplicatively to near
        // zero, draining even a full granary and triggering a famine death-spiral.
        // A bad season is a bad season — never a total production wipeout.
        for row in m.iter_mut() {
            for v in row.iter_mut() {
                if *v < EVENT_PROD_FLOOR { *v = EVENT_PROD_FLOOR; }
            }
        }
    }


    /// Arbitrage one round: each surplus hub ships toward the best reachable
    /// deficit hubs, creating in-transit cargo with an ETA. Bounded per hub.
    pub(crate) fn dispatch(&mut self, needs: &[Vec<f32>]) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let tick = self.tick;
        // Quarantine lookup, built ONCE per dispatch (O(events)) instead of scanning
        // every active event inside the hot seller×target×good loop. A locked-down
        // city neither ships nor receives.
        let mut quarantined = vec![false; n];
        for e in &self.active_events {
            if e.kind == "plague_lockup" && e.until_tick > tick && e.hub >= 0 && (e.hub as usize) < n {
                quarantined[e.hub as usize] = true;
            }
        }
        // DLC 3.5 · per-destination reserve-coin freight discount, precomputed once
        // (it's constant across this dispatch round and read in the hot inner loop).
        let coin_disc: Vec<f32> = (0..n).map(|d| self.coin_discount(d)).collect();
        // ── Merchant fleet capacity (concurrent shipment slots) for this round ──
        // Each house has fleet_sea sea-slots and (fleet_river + fleet_caravan)
        // land-slots. Slots already busy with in-flight cargo are subtracted, so a
        // house can only finance as many NEW shipments as it has free vessels. A
        // trade it can't carry falls to the independent local merchants/guilds.
        let nh = self.houses.len();
        let mut cap_sea: Vec<i32> = vec![0; nh];
        let mut cap_land: Vec<i32> = vec![0; nh];
        for (i, h) in self.houses.iter().enumerate() {
            if h.defunct { continue; }
            cap_sea[i] = h.fleet_sea as i32;
            cap_land[i] = (h.fleet_river + h.fleet_caravan) as i32;
        }
        for c in &self.in_transit {
            // Contract deliveries are covered by the standing reservation below — don't
            // subtract them here too, or the same ship is counted busy twice.
            if c.owner >= 0 && !c.contract {
                let oi = c.owner as usize;
                if oi < nh { if c.sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; } }
            }
        }
        // Futures contracts reserve dedicated vessels up front: hold back the ships /
        // caravans each ACTIVE contract needs for its monthly delivery, so opportunistic
        // spot arbitrage can never strand a signed contract without transport (this is
        // the reservation that makes the contract carry first call on the fleet).
        for c in &self.contracts {
            if self.tick >= c.end_tick || c.suspended_until > self.tick { continue; }
            let oi = c.seller_house as usize;
            let (src, buyer) = (c.source_hub as usize, c.buyer_hub as usize);
            if oi >= nh || src >= n || buyer >= n { continue; }
            let (sc, bc) = (self.hubs[src].coastal, self.hubs[buyer].coastal);
            if sc || bc { // sea leg
                cap_sea[oi] -= (c.monthly_qty / SHIP_CAPACITY).ceil() as i32;
            }
            if !(sc && bc) { // land leg
                let rv = self.houses[oi].fleet_river as f32;
                let cv = self.houses[oi].fleet_caravan as f32;
                let land_per = if rv + cv > 0.0 {
                    (rv * BOAT_CAPACITY + cv * CARAVAN_CAPACITY) / (rv + cv)
                } else { CARAVAN_CAPACITY };
                cap_land[oi] -= (c.monthly_qty / land_per).ceil() as i32;
            }
        }
        // Snapshot stocks so a single round's decisions use consistent prices.
        for g in 0..ng {
            let base = self.goods[g].base_value;
            // Build (hub, surplus) and (hub, price) lists.
            let reserve_mult = if self.goods[g].food { FOOD_RESERVE_DAYS } else { TRADE_RESERVE_MULT };
            let mut sellers: Vec<(usize, f32)> = Vec::new();
            for a in 0..n {
                // Keep a reserve (a granary for food) before exporting the rest.
                let surplus = self.hubs[a].stock[g] - needs[a][g] * reserve_mult;
                if surplus > EPS {
                    sellers.push((a, surplus));
                }
            }
            if sellers.is_empty() {
                continue;
            }
            for a_i in 0..sellers.len() {
                let (a, mut surplus) = sellers[a_i];
                if surplus <= EPS {
                    continue;
                }
                // A city under plague quarantine ships nothing out.
                if quarantined[a] { continue; }
                let pa = self.live_price(self.hubs[a].stock[g], needs[a][g], base);
                // A Guildhall at the SELLER's hub lowers freight on its exports.
                let freight_rate = self.freight_per_day
                    * if self.hub_has_struct(a, STRUCT_GUILDHALL) { GUILDHALL_FREIGHT } else { 1.0 };
                // Find the best deficit hubs among a's NEAREST reachable markets.
                // (Capping to the K nearest keeps this O(K) rather than O(n); the
                // 3 hungriest are kept below, so far-flung hubs never mattered.)
                let mut targets: Vec<(usize, f32, f32)> = Vec::new(); // (b, gap, days)
                for ti in 0..self.neighbors[a].len() {
                    let b = self.neighbors[a][ti] as usize;
                    if b == a {
                        continue;
                    }
                    // A quarantined city takes no imports either.
                    if quarantined[b] { continue; }
                    let days = self.days[a * n + b];
                    if !days.is_finite() {
                        continue;
                    }
                    let pb = self.live_price(self.hubs[b].stock[g], needs[b][g], base);
                    // A trusted reserve coin at the buyer `b` shaves freight (DLC 3.5).
                    let freight = self.good_freight(g, freight_rate * coin_disc[b], days);
                    let gap = pb - (pa + freight) - self.margin * base;
                    if gap > 0.0 {
                        // GRAVITY: weight the profit by the destination's trade pull so
                        // merchants prefer to supply the great markets (big entrepôts) —
                        // they clear more volume and pay reliably. The real `gap`/`days`
                        // still govern the actual sale; pull only orders the shortlist.
                        targets.push((b, gap * self.hub_pull(b), days));
                    }
                }
                if targets.is_empty() {
                    continue;
                }
                targets.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
                targets.truncate(3); // ship to the 3 most attractive reachable markets
                for (b, _gap, days) in targets {
                    if surplus <= EPS {
                        break;
                    }
                    // Don't overfill b past delivered-cost parity.
                    let delivered = pa + self.good_freight(g, freight_rate, days);
                    let max_stock =
                        needs[b][g] * (base / delivered.max(EPS)).powf(1.0 / self.k);
                    let room = (max_stock - self.hubs[b].stock[g]).max(0.0);
                    let mut amount = surplus.min(room * 0.5);
                    if amount <= EPS {
                        continue;
                    }
                    // Route mode: a sea voyage when both ends are coastal, else overland.
                    let sea = self.hubs[a].coastal && self.hubs[b].coastal;
                    // ── Who carries it ──────────────────────────────────────────
                    // Prefer the SELLER's house (the exporter organizes the sale);
                    // if it has no free vessel / no capital, fall back to the
                    // BUYER's house (the importing city sends its own ships to fetch
                    // the goods). This lets houses in big IMPORTER cities earn — the
                    // old code only ever credited the exporter, so houses clustered
                    // in importing capitals never grew. Only if NEITHER can carry it
                    // does it fall to independent local merchants & guilds.
                    let mut owner = -1i32;
                    for cand in [self.house_for(a, g), self.house_for(b, g)] {
                        if cand < 0 { continue; }
                        let oi = cand as usize;
                        // Trade war: a house barred from either market cannot run this
                        // leg — the trade falls to a rival or independent merchants.
                        if self.house_barred.get(oi).is_some_and(|v| v.contains(&(a as u32)) || v.contains(&(b as u32))) {
                            continue;
                        }
                        let slots = if sea { cap_sea[oi] } else { cap_land[oi] };
                        // Merchant-banker houses can finance cargo beyond their cash.
                        let credit = if self.houses[oi].archetype == ARCH_BANKING { BANK_CREDIT_MULT } else { 1.0 };
                        let afford = if pa > EPS { self.houses[oi].wealth * credit / pa } else { f32::MAX };
                        if slots >= 1 && afford > EPS {
                            amount = amount.min(afford);
                            if sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; }
                            owner = cand;
                            break;
                        }
                    }
                    surplus -= amount;
                    self.hubs[a].stock[g] -= amount;
                    let sale = amount * pa;
                    self.hubs[a].export_earn += sale;
                    // An ESTATE's sales pay rent to its OWNER: a share to the owning
                    // house's wealth (the engine of house growth), or to the parent
                    // city's prosperity if the estate is city-owned.
                    if self.hubs[a].is_estate {
                        let mut cut = sale * ESTATE_OWNER_CUT;
                        // A bank holding an equity STAKE in this manufactory collects its
                        // share of the owner-cut as a dividend (income to its reserves).
                        let sb = self.hubs[a].stake_bank;
                        if sb >= 0 && (sb as usize) < self.banks.len() && !self.banks[sb as usize].defunct {
                            let div = cut * self.hubs[a].stake_share.clamp(0.0, 0.9);
                            if div > 0.0 {
                                cut -= div;
                                self.banks[sb as usize].reserves += div;
                                self.banks[sb as usize].dividends_earned += div;
                            }
                        }
                        let owner = self.hubs[a].owner_house;
                        if owner >= 0 && (owner as usize) < self.houses.len()
                            && !self.houses[owner as usize].defunct {
                            self.houses[owner as usize].wealth += cut;
                            // Phase G: estate income, taxed by the parent city.
                            let etax = cut * ESTATE_TAX_RATE;
                            self.houses[owner as usize].wealth -= etax;
                            let parent = self.hubs[a].parent;
                            let is_manu = self.hubs[a].estate_kind == 6;
                            if parent >= 0 && (parent as usize) < self.hubs.len() {
                                let p = parent as usize;
                                let treas = etax * TREASURY_TAX_SHARE;
                                self.hubs[p].treasury += treas;
                                self.hubs[p].civic_pool += etax - treas;
                                // Manufactories (kind 6) book under manufacturing tax;
                                // raw estates under estate tax — for the City Finances panel.
                                if is_manu { self.hubs[p].finance.tax_manufacture += etax; }
                                else { self.hubs[p].finance.tax_estate += etax; }
                                self.hubs[p].finance.spent_civic += etax - treas;
                            }
                            if (owner as usize) < self.house_ledger.len() {
                                self.house_ledger[owner as usize].estate_income += cut;
                                self.house_ledger[owner as usize].estate_tax += etax;
                            }
                        } else {
                            let p = self.hubs[a].parent;
                            if p >= 0 && (p as usize) < self.hubs.len() {
                                self.hubs[p as usize].export_earn += cut;
                            }
                        }
                    }
                    // ── Voyage loss: storms at sea, ambush/wreck overland ──
                    let lost = if owner >= 0 {
                        let oi = owner as usize;
                        let mut p = if sea {
                            SEA_LOSS
                        } else {
                            // River boats are safer than caravans — blend by fleet mix.
                            let cv = self.houses[oi].fleet_caravan as f32;
                            let rv = self.houses[oi].fleet_river as f32;
                            let tot = (cv + rv).max(1.0);
                            CARAVAN_LOSS * (cv / tot) + RIVER_LOSS * (rv / tot)
                        };
                        // A shipping dynasty loses fewer cargoes (skilled crews).
                        if self.houses[oi].archetype == ARCH_FLEET { p *= FLEET_LOSS_MULT; }
                        hash01(self.seed,
                            (tick as u64) ^ 0x5EA10 ^ ((a as u64) << 8) ^ (b as u64),
                            g as u64) < p
                    } else { false };
                    if lost {
                        let oi = owner as usize;
                        let invested = amount * pa;
                        self.houses[oi].wealth = (self.houses[oi].wealth - invested).max(0.0);
                        if oi < self.house_ledger.len() {
                            self.house_ledger[oi].lost_cargo += invested;
                        }
                        self.damage_fleet(oi, sea);
                        let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                        let hn = self.houses[oi].name.clone();
                        let (etext, jtext) = if sea {
                            (format!("A storm sank a ship carrying {}", gn),
                             format!("A storm sinks a ship of {} ({})", hn, gn))
                        } else {
                            (format!("A caravan carrying {} was ambushed", gn),
                             format!("A caravan of {} is ambushed ({})", hn, gn))
                        };
                        self.houses[oi].events.push(HouseEvent { tick, kind: "voyage_loss".into(), text: etext });
                        // Tagged "voyage_loss" (not "event") so the settlement chronicle
                        // can hide the shipwreck/ambush spam — it's noise, not history.
                        self.journal.push(JournalEntry {
                            tick, kind: "voyage_loss".into(), hub: a as i32, good: g as i32,
                            value: invested, text: jtext,
                        });
                        self.diag_lost += 1;
                        // Cargo is gone — never delivered (source already debited).
                        continue;
                    }
                    self.diag_shipments += 1;
                    self.diag_volume += amount;
                    if owner >= 0 { self.diag_by_house += 1; } else { self.diag_by_guild += 1; }
                    // Attribute throughput to a merchant CLASS at both endpoints for
                    // the population estimate: a house-owned voyage → houses; an
                    // independent short haul → local merchants; a long haul → guilds.
                    let cls = if owner >= 0 { 0u8 } else if days <= LOCAL_HAUL_DAYS { 1 } else { 2 };
                    for &hh in &[a, b] {
                        match cls {
                            0 => self.hubs[hh].tw_house += amount,
                            1 => self.hubs[hh].tw_local += amount,
                            _ => self.hubs[hh].tw_guild += amount,
                        }
                    }
                    let value = amount * delivered;
                    self.hubs[b].import_spend += value;
                    if owner >= 0 {
                        let oi = owner as usize;
                        let margin = amount * (delivered - pa).max(0.0);
                        // A house that holds a monopoly on this good extracts extra
                        // rent (pricing power) on top of the plain margin.
                        let mono = self.houses[oi].monopoly.iter()
                            .find(|(mg, _)| *mg == g).map(|(_, s)| *s).unwrap_or(0.0);
                        let mut mult = 1.0 + 0.6 * mono;
                        // Specialist houses earn fatter margins on their trade; a city
                        // charter (political houses) adds further rent on that good.
                        if self.houses[oi].archetype == ARCH_SPECIALTY
                            && self.houses[oi].spec.contains(&g) { mult *= SPECIALTY_MARGIN; }
                        if self.houses[oi].charters.contains(&g) { mult *= CHARTER_RENT; }
                        // MONOPOLY EXPORT (#2): a house with ≥80% trade control of the
                        // SOURCE city `a` commands its surplus and exports on its own
                        // terms, taking a further rent — but only from a sustainable
                        // (not starving) city that has goods to spare.
                        if self.hubs[a].food_balance >= -0.1 {
                            let ctrl = self.houses[oi].influence.iter()
                                .find(|(c, _)| *c == a as u32).map(|(_, v)| *v).unwrap_or(0.0);
                            if ctrl >= MONOPOLY_CONTROL { mult *= 1.0 + MONOPOLY_EXPORT_RENT; }
                        }
                        let profit = margin * mult;
                        self.houses[oi].wealth += profit;
                        self.houses[oi].volume += amount;
                        // Phase G: civic taxes on this trade (export at origin a,
                        // import at destination b) — paid by the house, funding the
                        // cities (civic_pool → people). Guilds pay heavier taxes.
                        // Guilds pay heavier, PROGRESSIVE taxes — the more a guild
                        // trades, the higher the rate on each shipment.
                        let tax_mult = if self.houses[oi].is_guild {
                            let vol = self.houses[oi].volume.max(0.0);
                            GUILD_TAX_MULT
                                + GUILD_TAX_PROGRESSIVE * (vol / GUILD_TAX_VOLUME_REF).clamp(0.0, 1.0)
                        } else { 1.0 };
                        // DLC 3 · the origin/destination poleis levy their COUNCIL-set
                        // tariff (0 = no policy yet → the global default rate); a
                        // per-city prosperity bracket then scales it — rich cities tax
                        // harder, poor ones stay cheap to trade through.
                        let exp_rate = if self.hubs[a].tariff_export > 0.0 { self.hubs[a].tariff_export } else { EXPORT_TAX_RATE };
                        let imp_rate = if self.hubs[b].tariff_import > 0.0 { self.hubs[b].tariff_import } else { IMPORT_TAX_RATE };
                        // Bailo concession / dominance edge for the carrying house at each end.
                        let export_tax = value * exp_rate * tax_mult * self.city_tax_factor(a) * self.house_city_tax_mult(oi, a);
                        let import_tax = value * imp_rate * tax_mult * self.city_tax_factor(b) * self.house_city_tax_mult(oi, b);
                        self.houses[oi].wealth -= export_tax + import_tax;
                        // DLC 3.5 · split tariffs: a share is retained in the city
                        // treasury (capital for war/works), the rest reaches the
                        // people via the civic pool. Gross is booked as city income.
                        let exp_treas = export_tax * TREASURY_TAX_SHARE;
                        let imp_treas = import_tax * TREASURY_TAX_SHARE;
                        self.hubs[a].treasury += exp_treas;
                        self.hubs[a].civic_pool += export_tax - exp_treas;
                        self.hubs[a].finance.tax_trade += export_tax;
                        self.hubs[a].finance.spent_civic += export_tax - exp_treas;
                        self.hubs[b].treasury += imp_treas;
                        self.hubs[b].civic_pool += import_tax - imp_treas;
                        self.hubs[b].finance.tax_trade += import_tax;
                        self.hubs[b].finance.spent_civic += import_tax - imp_treas;
                        if oi < self.house_ledger.len() {
                            LedgerAcc::add_city(&mut self.house_ledger[oi].trade_profit_by_city, b as u32, profit);
                            LedgerAcc::add_city(&mut self.house_ledger[oi].export_tax_by_city, a as u32, export_tax);
                            LedgerAcc::add_city(&mut self.house_ledger[oi].import_tax_by_city, b as u32, import_tax);
                        }
                        // Track cumulative profit + volume per good (the trade ledger:
                        // "most profitable resources" + "goods moved the most").
                        let gp = &mut self.houses[oi].good_profit;
                        if gp.len() <= g { gp.resize(g + 1, 0.0); }
                        gp[g] += profit;
                        let gv = &mut self.houses[oi].good_volume;
                        if gv.len() <= g { gv.resize(g + 1, 0.0); }
                        gv[g] += amount;
                        // Build the holder's trade ties at both ends (for offices).
                        self.bump_trade_at(oi, a, amount);
                        self.bump_trade_at(oi, b, amount);
                    }
                    self.accrue_flow(a, b, g, amount);
                    self.in_transit.push(InTransit {
                        from: a as u32,
                        to: b as u32,
                        good: g,
                        amount,
                        eta_tick: tick + (days.ceil() as u32).max(1),
                        owner,
                        sea,
                        // A house voyage is a ROUND TRIP: on arrival at b it tries to
                        // buy b's surplus and carry it home to a (sold there for a
                        // second profit). Guild/local one-way trips spawn no return.
                        phase: 0,
                        home: if owner >= 0 { a as i32 } else { -1 },
                        contract: false,
                    });
                    self.log_trade(a as u32, b as u32, g, amount, owner, sea, pa);
                }
            }
        }
    }


    /// The RETURN leg of a house round trip. A house vessel that just sold its
    /// outbound cargo at `b` buys `b`'s most profitable surplus good and carries it
    /// home to `a`, where it sells for a SECOND profit. The buy is usually at `b`'s
    /// market price, but an over-supplied (glutted) source yields an occasional
    /// ~25% bargain (a windfall that voyage). Profit here is true arbitrage
    /// (sell − buy − freight), so source discounts actually raise the take — the
    /// hook the office −5% discount (C3) plugs into. Respects the same food granary
    /// reserve and the buyer-side import cap, so it never strips `b`'s supply.
    pub(crate) fn deploy_return_leg(&mut self, owner: usize, b: usize, a: usize, needs: &[Vec<f32>]) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        if b >= n || a >= n || owner >= self.houses.len() || self.houses[owner].defunct {
            return;
        }
        let days = self.days[b * n + a];
        if !days.is_finite() {
            return;
        }
        // Freight home (a Guildhall at b lowers it); per-good weight/spoilage is
        // folded in per candidate good below via `good_freight`. A trusted reserve
        // coin at the destination `a` shaves transaction cost (DLC 3.5 coinage).
        let freight_rate = self.freight_per_day
            * if self.hub_has_struct(b, STRUCT_GUILDHALL) { GUILDHALL_FREIGHT } else { 1.0 }
            * self.coin_discount(a);
        // An office at b gives the holder a standing −5% on what it buys there.
        let office_disc = if self.houses[owner].offices.contains(&(b as u32)) { OFFICE_BUY_DISCOUNT } else { 0.0 };
        // Pick b's surplus good that earns the most carried home to a.
        let mut best: Option<(usize, f32, f32, f32)> = None; // (good, amount, buy_price, sell_price)
        let mut best_score = 0.0f32;
        for g in 0..ng {
            let base = self.goods[g].base_value;
            let reserve_mult = if self.goods[g].food { FOOD_RESERVE_DAYS } else { TRADE_RESERVE_MULT };
            let surplus = self.hubs[b].stock[g] - needs[b][g] * reserve_mult;
            if surplus <= EPS { continue; }
            let pb = self.live_price(self.hubs[b].stock[g], needs[b][g], base);
            // Occasional bargain when b is heavily oversupplied in this good.
            let glut = self.hubs[b].stock[g] > (needs[b][g] * reserve_mult * 2.0).max(20.0);
            let bargain = glut && hash01(self.seed,
                (self.tick as u64) ^ 0x0BA46A1 ^ ((b as u64) << 8) ^ a as u64, g as u64) < 0.25;
            // Source-buy discount: an occasional glut bargain + any office −5%, capped.
            let discount = (if bargain { 0.25 } else { 0.0 } + office_disc).min(MAX_BUY_DISCOUNT);
            let pb_buy = pb * (1.0 - discount);
            let pa_sell = self.live_price(self.hubs[a].stock[g], needs[a][g], base);
            let freight = self.good_freight(g, freight_rate, days);
            let gap = pa_sell - pb_buy - freight - self.margin * base;
            if gap <= 0.0 { continue; }
            // Don't overfill a past delivered-cost parity.
            let delivered = pb_buy + freight;
            let max_stock = needs[a][g] * (base / delivered.max(EPS)).powf(1.0 / self.k);
            let room = (max_stock - self.hubs[a].stock[g]).max(0.0);
            let amount = surplus.min(room * 0.5);
            if amount <= EPS { continue; }
            let score = gap * amount;
            if score > best_score {
                best_score = score;
                best = Some((g, amount, pb_buy, pa_sell));
            }
        }
        let Some((g, amount, pb_buy, pa_sell)) = best else { return };
        let freight = self.good_freight(g, freight_rate, days);
        let sea = self.hubs[b].coastal && self.hubs[a].coastal;
        // Buy at b (goods leave b's stock), sell on arrival at a.
        self.hubs[b].stock[g] -= amount;
        self.hubs[b].export_earn += amount * pb_buy;
        self.hubs[a].import_spend += amount * (pb_buy + freight);
        // True-arbitrage profit (so the source discount actually pays).
        let mono = self.houses[owner].monopoly.iter()
            .find(|(mg, _)| *mg == g).map(|(_, s)| *s).unwrap_or(0.0);
        let mut mult = 1.0 + 0.6 * mono;
        if self.houses[owner].archetype == ARCH_SPECIALTY && self.houses[owner].spec.contains(&g) {
            mult *= SPECIALTY_MARGIN;
        }
        if self.houses[owner].charters.contains(&g) { mult *= CHARTER_RENT; }
        let profit = amount * (pa_sell - pb_buy - freight).max(0.0) * mult;
        self.houses[owner].wealth += profit;
        if owner < self.house_ledger.len() {
            // Round-trip arbitrage profit, realised selling at the home hub `a`.
            LedgerAcc::add_city(&mut self.house_ledger[owner].trade_profit_by_city, a as u32, profit);
        }
        self.houses[owner].volume += amount;
        let gp = &mut self.houses[owner].good_profit;
        if gp.len() <= g { gp.resize(g + 1, 0.0); }
        gp[g] += profit;
        let gv = &mut self.houses[owner].good_volume;
        if gv.len() <= g { gv.resize(g + 1, 0.0); }
        gv[g] += amount;
        // Diagnostics + throughput at both ends (house class).
        self.diag_shipments += 1;
        self.diag_by_house += 1;
        self.diag_volume += amount;
        self.hubs[b].tw_house += amount;
        self.hubs[a].tw_house += amount;
        self.bump_trade_at(owner, a, amount);
        self.bump_trade_at(owner, b, amount);
        // The same vessel carries it home (occupies the owner's slot until it lands).
        // No fresh voyage-loss roll here — the return is the trip's bonus leg.
        self.accrue_flow(b, a, g, amount);
        self.in_transit.push(InTransit {
            from: b as u32,
            to: a as u32,
            good: g,
            amount,
            eta_tick: self.tick + (days.ceil() as u32).max(1),
            owner: owner as i32,
            sea,
            phase: 1,
            home: -1,
            contract: false,
        });
        self.log_trade(b as u32, a as u32, g, amount, owner as i32, sea, pb_buy);
    }


    /// Straight-line distance (in cells, cylindrical-X) between two hubs.
    pub(crate) fn hub_cell_dist(&self, a: usize, b: usize) -> f32 {
        let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
        if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
        let dy = self.hubs[a].y - self.hubs[b].y;
        (dx * dx + dy * dy).sqrt()
    }


    /// Roll low-probability events for this tick.
    pub(crate) fn roll_events(&mut self) {
        let n = self.hubs.len();
        if n == 0 {
            return;
        }
        let tick = self.tick;
        let r = hash01(self.seed, tick as u64, 0xE7E7);
        // ~ one event every ~10 ticks on average (raised for MORE epidemics; the
        // disease origin gates + immunity keep them from overrunning the world).
        if r > 0.10 {
            return;
        }
        let pick = hash01(self.seed, tick as u64, 0x1234);
        let hub = (hash01(self.seed, tick as u64, 0x5678) * n as f32) as usize % n;
        let (kind, mag, dur, good): (&str, f32, u32, i32) = if pick < 0.22 {
            // A drought trims food for a season. Kept moderate so a hub's granary
            // reserve + the baseline food surplus can ride it out — a bad year, not
            // an automatic famine (deep deficits used to spiral into collapse).
            ("drought", 0.20 + 0.15 * pick, 30 + (pick * 40.0) as u32, -1)
        } else if pick < 0.46 {
            // The disease + severity are chosen inside the match below (24% of events).
            ("plague", 0.0, 0, -1)
        } else if pick < 0.54 {
            ("fire", 0.5, 1, -1)
        } else if pick < 0.66 {
            ("fishery_collapse", 0.5, 120, -1)
        } else if pick < 0.82 {
            // EXCEPTIONAL YEAR — a bumper harvest: production surges for a season,
            // stocks build and prices fall, so this settlement's goods turn cheap
            // and flood out to its trade partners.
            ("bumper", 0.55 + 0.35 * pick, 120 + (pick * 60.0) as u32, -1)
        } else if pick < 0.90 {
            ("festival", 0.0, 1, -1)
        } else {
            // House feud → embargo on a random good at this hub.
            let g = (hash01(self.seed, tick as u64, 0x9999) * self.goods.len() as f32) as i32
                % self.goods.len().max(1) as i32;
            ("embargo", 0.8, 60, g)
        };
        let text = match kind {
            "drought" => format!("Drought grips the lands around {}", self.hubs[hub].name),
            "plague" => format!("Plague strikes {}", self.hubs[hub].name),
            "fire" => format!("Fire ravages the warehouses of {}", self.hubs[hub].name),
            "fishery_collapse" => format!("The fisheries off {} collapse", self.hubs[hub].name),
            "bumper" => format!("An exceptional harvest at {} — goods turn cheap", self.hubs[hub].name),
            "festival" => format!("{} holds a great festival", self.hubs[hub].name),
            _ => format!("A trade feud erupts at {}", self.hubs[hub].name),
        };
        // Immediate one-shot effects.
        match kind {
            "fire" => {
                for g in 0..self.goods.len() {
                    self.hubs[hub].stock[g] *= 1.0 - mag;
                }
                // The warehouses that burn belong to the city's merchant houses:
                // every resident house loses a slice of its wealth (stored stock
                // value), the heavier the richer it is — a stabilizing loss that
                // scales with prosperity. Recorded in the Accountant's misfortune line.
                for hi in 0..self.houses.len() {
                    if self.houses[hi].defunct || self.houses[hi].hub as usize != hub {
                        continue;
                    }
                    let loss = self.houses[hi].wealth * mag * 0.5;
                    self.houses[hi].wealth -= loss;
                    if hi < self.house_ledger.len() {
                        self.house_ledger[hi].events += loss;
                    }
                }
                // Phase 2: the fire also strikes ONE house depot in the city —
                // BURNING it out (all stock lost, gutted to a Tier-1 building) or
                // DAMAGING it (up to 80% of stock AND capacity, which may demote a
                // tier). A burned depot that can't meet a futures contract will later
                // trigger a seller default (Phase 3). Tagged "disaster" so it is kept
                // in the chronicle (unlike routine voyage losses).
                let wis: Vec<usize> = (0..self.warehouses.len())
                    .filter(|&i| self.warehouses[i].hub as usize == hub
                        && self.warehouses[i].owner >= 0
                        && !self.houses.get(self.warehouses[i].owner as usize)
                            .map(|h| h.defunct).unwrap_or(true))
                    .collect();
                if !wis.is_empty() {
                    let wi = wis[(hash01(self.seed, tick as u64 ^ 0xB175, 0) * wis.len() as f32)
                        as usize % wis.len()];
                    let oi = self.warehouses[wi].owner as usize;
                    let hname = self.houses[oi].name.clone();
                    let cname = self.hubs[hub].name.clone();
                    let old_t = self.warehouses[wi].tier;
                    let sev_roll = hash01(self.seed, tick as u64 ^ 0xF13E, wi as u64);
                    let txt = if sev_roll < 0.4 {
                        // BURN: total stock loss, building gutted to a Tier-1 depot.
                        for s in self.warehouses[wi].stock.iter_mut() { *s = 0.0; }
                        self.warehouses[wi].capacity = WH_TIER1_CAP;
                        self.warehouses[wi].tier = 1;
                        self.warehouses[wi].damage = 1.0;
                        format!("Fire guts the {} warehouse of {} — all stock lost", cname, hname)
                    } else {
                        // DAMAGE: up to 80% of stock AND capacity; capacity loss may demote.
                        let sev = 0.2 + 0.6 * sev_roll; // 0.2 .. 0.8
                        for s in self.warehouses[wi].stock.iter_mut() { *s *= 1.0 - sev; }
                        let newcap = (self.warehouses[wi].capacity * (1.0 - sev)).max(WH_TIER1_CAP * 0.5);
                        self.warehouses[wi].capacity = newcap;
                        self.warehouses[wi].tier = Self::capacity_tier(newcap);
                        self.warehouses[wi].damage = (self.warehouses[wi].damage + sev).min(1.0);
                        let note = if self.warehouses[wi].tier < old_t {
                            format!(", dropped to tier {}", self.warehouses[wi].tier)
                        } else { String::new() };
                        format!("Fire damages the {} warehouse of {} (−{:.0}% stock{})",
                            cname, hname, sev * 100.0, note)
                    };
                    self.houses[oi].events.push(HouseEvent {
                        tick, kind: "disaster".into(), text: txt.clone() });
                    self.journal.push(JournalEntry {
                        tick, kind: "disaster".into(), hub: hub as i32, good: -1, value: 0.0, text: txt });
                }
                // Estates around the city are struck too — a fire/blight cripples a
                // farm, mine or manufactory; a SEVERE one ABANDONS it (its people
                // scatter, production stops). The estate hub is kept (no index churn)
                // but goes dormant — its population falls toward zero.
                let ests: Vec<usize> = (0..self.hubs.len())
                    .filter(|&i| self.hubs[i].is_estate && self.hubs[i].parent == hub as i32).collect();
                if !ests.is_empty() {
                    let ei = ests[(hash01(self.seed, tick as u64 ^ 0xE57A, hub as u64) * ests.len() as f32) as usize % ests.len()];
                    let sev = hash01(self.seed, tick as u64 ^ 0xDEAD, ei as u64);
                    let ename = self.hubs[ei].name.clone();
                    let parent = self.hubs[ei].parent;
                    let txt = if sev < 0.18 {
                        self.hubs[ei].population = (self.hubs[ei].population * 0.02).max(1.0);
                        for v in self.hubs[ei].base_per_capita.iter_mut() { *v = 0.0; }
                        self.hubs[ei].estate_tier = 0;
                        format!("{} is abandoned after disaster — its lands fall silent", ename)
                    } else {
                        let s = 0.3 + 0.4 * sev;
                        self.hubs[ei].population *= 1.0 - s * 0.5;
                        for v in self.hubs[ei].base_per_capita.iter_mut() { *v *= 1.0 - s; }
                        self.hubs[ei].estate_tier = self.hubs[ei].estate_tier.saturating_sub(1).max(1);
                        format!("Disaster cripples {} (−{:.0}% output)", ename, s * 100.0)
                    };
                    self.journal.push(JournalEntry {
                        tick, kind: "disaster".into(), hub: parent, good: -1, value: 0.0, text: txt });
                }
            }
            "plague" => {
                // A spontaneous outbreak. Roll its CATEGORY (rarity ↑ with severity):
                // most are local cat-3 nuisances; a few become regional cat-2; a great
                // cat-1 plague is rare. Severity (the cull) scales with the category.
                // The market routes around the lockup; futures touching it are force-
                // majeure suspended. Phase 5 `spread_epidemics` then carries a cat-1/2
                // along the trade lanes (never geographically) from this focus.
                // Pick a DISEASE (weighted). Some need a specific ORIGIN: water-borne
                // (cholera/dysentery) and vector (malaria) emerge in a wet/coastal
                // locale; trade/airborne can start anywhere. The cull is the disease's
                // own deadliness range; the severity category follows from that.
                let disease = pick_disease(self.seed, tick, hub);
                let spec = &DISEASES[disease as usize];
                let origin_ok = match spec.mode { 1 | 3 => self.hubs[hub].coastal, _ => true };
                if origin_ok {
                    let sev = hash01(self.seed, tick as u64 ^ 0x5EE7, hub as u64);
                    let cull = spec.dead_lo + (spec.dead_hi - spec.dead_lo) * sev;
                    let category = disease_category(disease);
                    self.strike_plague(hub, cull, category, disease, None);
                }
            }
            "festival" => { /* demand spike handled implicitly by low stock */ }
            _ => {}
        }
        // Adverse events also dent the GLOBAL production index: an ordinary shock
        // trims ~1%, a rare catastrophic fire ~4.5% (on top of the local hit). The
        // slow +0.5%/yr drift recovers it over the following years.
        match kind {
            "fire" => self.tech_factor *= 1.0 - PROD_FIRE_SETBACK,
            "drought" | "plague" | "fishery_collapse" | "embargo" => {
                self.tech_factor *= 1.0 - PROD_EVENT_SETBACK;
            }
            _ => {}
        }
        self.tech_factor = self.tech_factor.max(TECH_FACTOR_FLOOR);
        if dur > 1 {
            self.active_events.push(ActiveEvent {
                kind: kind.to_string(),
                hub: hub as i32,
                good,
                magnitude: mag,
                until_tick: tick + dur,
            });
        }
        self.journal.push(JournalEntry {
            tick,
            kind: "event".into(),
            hub: hub as i32,
            good,
            value: mag,
            text,
        });
    }


    pub(crate) fn hub_has_struct(&self, h: usize, id: u8) -> bool {
        self.hubs[h].structures.contains(&id)
    }


    /// Standing production multipliers from a hub's structures: `(all_goods, food_only)`.
    pub(crate) fn hub_struct_prod(&self, h: usize) -> (f32, f32) {
        let (mut all, mut food) = (1.0f32, 1.0f32);
        for &s in &self.hubs[h].structures {
            match s {
                STRUCT_WORKSHOP => all *= WORKSHOP_PROD,
                STRUCT_WAREHOUSE => all *= WAREHOUSE_PROD,
                STRUCT_GRANARY => food *= GRANARY_FOOD_PROD,
                _ => {}
            }
        }
        (all, food)
    }


    /// Monthly: a prosperous settlement erects the most useful building it lacks.
    /// A Shipyard grants the resident house an extra sea ship on completion.
    pub(crate) fn update_structures(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].trade_wealth < STRUCT_BUILD_WEALTH { continue; }
            // Gradual: ~8%/month for an eligible hub → about one building a year.
            if hash01(self.seed, tick as u64 ^ 0x57D0C7, h as u64) > 0.08 { continue; }
            let has = |id: u8| self.hubs[h].structures.contains(&id);
            let coastal = self.hubs[h].coastal;
            let resident = self.strongest_house_at(h);
            let pick = if !has(STRUCT_WORKSHOP) { STRUCT_WORKSHOP }
                else if !has(STRUCT_GRANARY) { STRUCT_GRANARY }
                else if coastal && resident.is_some() && !has(STRUCT_SHIPYARD) { STRUCT_SHIPYARD }
                else if !has(STRUCT_GUILDHALL) { STRUCT_GUILDHALL }
                else if !has(STRUCT_WAREHOUSE) { STRUCT_WAREHOUSE }
                else { continue; };
            self.hubs[h].structures.push(pick);
            if pick == STRUCT_SHIPYARD {
                if let Some(hi) = resident { self.houses[hi].fleet_sea += 1; }
            }
            let hn = self.hubs[h].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "structure".into(), hub: h as i32, good: -1, value: 0.0,
                text: format!("{} builds a {}", hn, structure_label(pick)),
            });
        }
    }


    /// Monthly: a city spends its civic treasury (`civic_pool` — fed by trade
    /// taxes, guild dues and endowments) on PUBLIC WORKS. While it still lacks a
    /// useful civic building it erects one outright; once well-built it instead
    /// throws an occasional festival that lifts the people's prosperity and
    /// stability. This is the visible return of the guild-endowment sink to the
    /// settlement, distinct from the slower trade-wealth-funded `update_structures`.
    pub(crate) fn fund_public_works(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let pop = self.hubs[h].population.max(1.0);
            let civic_pc = self.hubs[h].civic_pool / pop * 100.0;
            if civic_pc < PUBLIC_WORKS_PC { continue; }
            let size = self.city_size_factor(h);
            let has = |id: u8| self.hubs[h].structures.contains(&id);
            // 1) Erect the next civic building it lacks (workshop → granary →
            //    guildhall → warehouse), if the treasury covers the cost.
            let pick = if !has(STRUCT_WORKSHOP) { Some(STRUCT_WORKSHOP) }
                else if !has(STRUCT_GRANARY) { Some(STRUCT_GRANARY) }
                else if !has(STRUCT_GUILDHALL) { Some(STRUCT_GUILDHALL) }
                else if !has(STRUCT_WAREHOUSE) { Some(STRUCT_WAREHOUSE) }
                else { None };
            let build_cost = PUBLIC_WORKS_BUILD_COST * size;
            if let Some(pick) = pick {
                if self.hubs[h].civic_pool >= build_cost {
                    self.hubs[h].civic_pool -= build_cost;
                    self.hubs[h].structures.push(pick);
                    let hn = self.hubs[h].name.clone();
                    self.journal.push(JournalEntry {
                        tick, kind: "structure".into(), hub: h as i32, good: -1, value: 0.0,
                        text: format!("{} funds public works — a {}", hn, structure_label(pick)),
                    });
                    continue;
                }
            }
            // 2) Well-built already → an occasional festival (a one-off lift to the
            //    populace), if the treasury can spare it.
            let fest_cost = FESTIVAL_COST * size;
            if self.hubs[h].civic_pool >= fest_cost
                && hash01(self.seed, tick as u64 ^ 0xFE57, h as u64) < 0.35
            {
                self.hubs[h].civic_pool -= fest_cost;
                self.hubs[h].sent_prosperity = (self.hubs[h].sent_prosperity + FESTIVAL_PROSPERITY).min(1.0);
                self.hubs[h].sent_stability = (self.hubs[h].sent_stability + FESTIVAL_STABILITY).min(1.0);
                let hn = self.hubs[h].name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "festival".into(), hub: h as i32, good: -1, value: 0.0,
                    text: format!("{} holds a public festival", hn),
                });
            }
        }
    }


    /// Ensure every live hub has a majority culture (seeded from the worldgen
    /// culture map, inherited by its founder for colonies) and a minorities slot.
    /// Only fills empties, so it self-heals old saves and newly-founded hubs.
    pub(crate) fn ensure_hub_cultures(&mut self) {
        let n = self.hubs.len();
        if self.hub_culture.len() < n { self.hub_culture.resize(n, String::new()); }
        if self.hub_minorities.len() < n { self.hub_minorities.resize(n, Vec::new()); }
        let map = crate::sim::cultures::active();
        let real = |c: &str| !c.is_empty() && c != "—";
        for i in 0..n {
            // Reassign empty AND stuck "—" hubs (a hub seeded "—" before the culture
            // map existed used to keep it forever → small cities with no people).
            if real(&self.hub_culture[i]) { continue; }
            // 1) inherit the founding settlement's people
            let founder = self.hubs[i].founder_hub;
            if founder >= 0 && (founder as usize) < n {
                let fc = self.hub_culture[founder as usize].clone();
                if real(&fc) { self.hub_culture[i] = fc; continue; }
            }
            // 2) the culture hearth whose region this cell falls in
            if let Some(m) = &map {
                if let Some(h) = m.hearth_at(self.hubs[i].x as u32, self.hubs[i].y as u32) {
                    if real(&h.people) { self.hub_culture[i] = h.people.clone(); continue; }
                }
            }
            // 3) fallback: the nearest already-cultured hub, so NO settlement is left
            //    without a people (fixes small towns showing no culture at all).
            let (hx, hy) = (self.hubs[i].x, self.hubs[i].y);
            let mut best: Option<(f32, String)> = None;
            for j in 0..n {
                if j == i { continue; }
                let c = &self.hub_culture[j];
                if !real(c) { continue; }
                let dx = self.hubs[j].x - hx; let dy = self.hubs[j].y - hy;
                let d = dx * dx + dy * dy;
                if best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) { best = Some((d, c.clone())); }
            }
            self.hub_culture[i] = best.map(|(_, c)| c).unwrap_or_else(|| "—".into());
        }
    }


    /// The city's people composition for DISPLAY: the plurality people as the
    /// majority + the rest as minority quarters (shares ~sum to 1). Read-only, so
    /// the panel shows the true dominant people even before the next yearly
    /// `rebalance_hub_majorities` has run on an in-progress save.
    pub(crate) fn hub_people_display(&self, h: usize) -> (String, Vec<(String, f32)>) {
        let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
        let mins = self.hub_minorities.get(h).cloned().unwrap_or_default();
        if maj.is_empty() || maj == "—" { return (maj, mins); }
        let minsum: f32 = mins.iter().map(|(_, s)| *s).sum();
        let mut all: Vec<(String, f32)> = Vec::with_capacity(mins.len() + 1);
        all.push((maj, (1.0 - minsum).clamp(0.0, 1.0)));
        for (c, s) in mins { all.push((c, s)); }
        all.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top = all.remove(0);
        (top.0, all)
    }


    /// Occasional construction event (both kinds, per the user): masons/patron speed the
    /// works; a collapse/flood sets them back. Rolled monthly per active site.
    pub(crate) fn maybe_construction_event(&mut self, h: usize) {
        let tick = self.tick;
        let r = hash01(self.seed, tick as u64 ^ 0xC0_11_5A, h as u64);
        if r < 0.03 {
            self.hubs[h].build_progress = (self.hubs[h].build_progress + 0.25).min(0.999);
            let nm = self.hubs[h].name.clone();
            self.journal.push(JournalEntry { tick, kind: "construction".into(), hub: h as i32, good: -1,
                value: 0.0, text: format!("Skilled masons hasten the works at {}", nm) });
        } else if r > 0.975 {
            self.hubs[h].build_progress = (self.hubs[h].build_progress - 0.2).max(0.0);
            let nm = self.hubs[h].name.clone();
            self.journal.push(JournalEntry { tick, kind: "construction".into(), hub: h as i32, good: -1,
                value: 0.0, text: format!("A scaffold collapse sets back the works at {}", nm) });
        }
    }


    /// The living merchant families: ageing heads, monopolies, feuds, founding,
    /// extinction and political power.
    /// Log a dispatched trade for the Market "recent deals" rows (rolling, capped).
    pub(crate) fn log_trade(&mut self, from: u32, to: u32, good: usize, amount: f32, owner: i32, sea: bool, price: f32) {
        self.recent_trades.push(RecentTrade { from, to, good, amount, owner, sea, price, tick: self.tick });
        let n = self.recent_trades.len();
        if n > 400 { self.recent_trades.drain(0..n - 400); }
        // Accumulate the year's trade flows for the Flows subtab: this shipment is an
        // INBOUND flow at `to` (from `from`) and an OUTBOUND flow at `from` (to `to`).
        if amount > 0.0 {
            let g = good as u32;
            *self.trade_cur.entry((to, g, from, 0)).or_insert(0.0) += amount;
            *self.trade_cur.entry((from, g, to, 1)).or_insert(0.0) += amount;
        }
    }


    /// At each New Year: snapshot the year's trade flows as `trade_last` (the Flows
    /// subtab's per-partner detail), append the per-(hub,good) yearly volume to the
    /// trend history (goods that DIDN'T trade get a 0, so a fallen trade shows its
    /// decline), then clear the accumulator for the new year.
    pub(crate) fn fold_trade_year(&mut self) {
        let mut last: Vec<TradeFlowAgg> = self.trade_cur.iter()
            .map(|(&(hub, good, partner, dir), &amount)| TradeFlowAgg { hub, good, partner, dir, amount })
            .collect();
        // Deterministic order (the panel re-sorts by volume anyway).
        last.sort_by(|a, b| (a.hub, a.good, a.dir, a.partner).cmp(&(b.hub, b.good, b.dir, b.partner)));
        // Per-(hub,good) total volume this year.
        let mut vol: std::collections::HashMap<(u32, u32), f32> = std::collections::HashMap::new();
        for f in &last { *vol.entry((f.hub, f.good)).or_insert(0.0) += f.amount; }
        // Extend existing series (0 for goods not traded this year → visible decline).
        let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for h in self.trade_hist.iter_mut() {
            let v = vol.get(&(h.hub, h.good)).copied().unwrap_or(0.0);
            h.vols.push(v);
            if h.vols.len() > TRADE_HIST_CAP { let d = h.vols.len() - TRADE_HIST_CAP; h.vols.drain(0..d); }
            seen.insert((h.hub, h.good));
        }
        // Brand-new (hub,good) trades start a fresh series. DETERMINISM: pushing in
        // HashMap order makes `trade_hist`'s order vary run to run, and the peak sort
        // below is stable — so equal peaks keep insertion order and a different set
        // survives the truncation. Iterate in key order.
        let mut fresh: Vec<((u32, u32), f32)> = vol.iter().map(|(&k, &v)| (k, v)).collect();
        fresh.sort_by_key(|&(k, _)| k);
        for ((hub, good), v) in fresh {
            if !seen.contains(&(hub, good)) { self.trade_hist.push(TradeHist { hub, good, vols: vec![v] }); }
        }
        // Bound memory: if over the row cap, drop the deadest trades (lowest peak).
        if self.trade_hist.len() > TRADE_HIST_ROWS {
            self.trade_hist.sort_by(|a, b| {
                let pa = a.vols.iter().cloned().fold(0.0f32, f32::max);
                let pb = b.vols.iter().cloned().fold(0.0f32, f32::max);
                // Tie-break on (hub, good) — without it two equally dead trades order
                // by whatever the Vec happened to hold.
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.hub.cmp(&b.hub)).then(a.good.cmp(&b.good))
            });
            self.trade_hist.truncate(TRADE_HIST_ROWS);
        }
        self.trade_last = last;
        self.trade_cur.clear();
    }


    /// Record trade VOLUME a holder moved through a hub (for office ties).
    pub(crate) fn bump_trade_at(&mut self, holder: usize, hub: usize, amount: f32) {
        if holder >= self.houses.len() { return; }
        let t = &mut self.houses[holder].trade_at;
        if let Some(e) = t.iter_mut().find(|(hb, _)| *hb == hub as u32) {
            e.1 += amount;
        } else {
            t.push((hub as u32, amount));
        }
    }
}
