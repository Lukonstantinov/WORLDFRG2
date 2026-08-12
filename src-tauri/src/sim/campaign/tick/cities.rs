//! cities — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

impl CampaignSim {

    /// DLC 3.5 · at New Year, snapshot each city's treasury books (for the City
    /// Finances panel) and the year's bundled trade flows (for the Dynamic Trade
    /// Flow overlay), then open a fresh year.
    pub(crate) fn roll_city_finances(&mut self, yr: u32) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let mut done = std::mem::take(&mut self.hubs[h].finance);
            done.year = yr.saturating_sub(1);
            done.prev = None; // don't nest histories
            self.hubs[h].finance = CityFinance { year: yr, prev: Some(Box::new(done)), ..Default::default() };
        }
        // Snapshot the year's per-pair trade volume, then reset the accumulator.
        if !self.flow_accum.is_empty() {
            // Sorted for the same reason as `classify_hubs` below: this vector is
            // serialized and read by the Dynamic Trade Flow overlay, so HashMap
            // iteration order would make a saved campaign differ run-to-run.
            self.flow_year = self.flow_accum.iter().map(|(&(a, b), &v)| (a, b, v)).collect();
            self.flow_year.sort_by_key(|&(a, b, _)| (a, b));
            self.flow_accum.clear();
        }
        // ── Atlas 2.0 · per-hub yearly throughput (Trade Heat / census) + the
        //    world sample for the Atlas graphs. ──
        let by_id: std::collections::HashMap<u32, usize> =
            self.hubs.iter().enumerate().map(|(i, h)| (h.id, i)).collect();
        for h in self.hubs.iter_mut() { h.trade_last_year = 0.0; }
        let mut trade_total = 0.0f32;
        for &(a, b, v) in &self.flow_year {
            trade_total += v;
            if let Some(&i) = by_id.get(&a) { self.hubs[i].trade_last_year += v; }
            if let Some(&i) = by_id.get(&b) { self.hubs[i].trade_last_year += v; }
        }
        let population: f32 = self.hubs.iter()
            .filter(|h| !h.is_estate).map(|h| h.population.max(0.0)).sum();
        let alive = self.hubs.iter()
            .filter(|h| !h.is_estate && !h.abandoned && h.population >= 1.0).count();
        self.world_series.push([
            yr.saturating_sub(1) as f32, population, trade_total, alive as f32,
            self.total_foundings as f32, self.total_abandonments as f32,
        ]);
        if self.world_series.len() > 400 {
            let excess = self.world_series.len() - 400;
            self.world_series.drain(0..excess);
        }
        // ── Batch 1 · per-GOOD ledger snapshot + era frame + Hall of Records ──
        if !self.good_flow_accum.is_empty() {
            self.hub_good_trade = std::mem::take(&mut self.good_flow_accum);
        }
        self.year_frames.push(YearFrame {
            year: yr.saturating_sub(1),
            pop: self.hubs.iter()
                .map(|h| if h.is_estate { -1.0 } else { h.population.max(0.0) }).collect(),
            trade: self.hubs.iter().map(|h| h.trade_last_year).collect(),
        });
        if self.year_frames.len() > 400 {
            let excess = self.year_frames.len() - 400;
            self.year_frames.drain(0..excess);
        }
        self.update_records(yr.saturating_sub(1), trade_total);
    }

    /// CITY_PROVINCE_WAR_PLAN.md §3.2 · city tiers, monthly, mirroring
    /// `assign_house_tiers` one for one (percentile rank + hysteresis + an absolute
    /// Tier-1 floor, so a young world has an EMPTY Tier 1 — a tier that's always
    /// occupied carries no information). Four axes (§1.3): population, trade wealth,
    /// treasury, territory administered (rural population under provinces THIS city
    /// holds — `prov_holder`, the city case; a house-held province, §5.9, counts
    /// toward the HOUSE not the city), and the ruling house's own standing (already
    /// 0..1 from `assign_house_tiers`, so it needs no rank-norm of its own — this is
    /// why 3.2 must run AFTER the house-tier refresh). **Query-side only at this
    /// step**: nothing downstream reads `hub.tier`/`hub.standing` yet, so this is
    /// provably bit-identical to the dynamics test, exactly as house tiers shipped
    /// (§5.6 of the plan: 3.3 is where that guarantee ends, not before).
    pub(crate) fn assign_city_tiers(&mut self) {
        let tick = self.tick;
        let live: Vec<usize> = (0..self.hubs.len())
            .filter(|&h| !self.hubs[h].is_estate && !self.hubs[h].abandoned)
            .collect();
        let n = live.len();
        if n == 0 { return; }

        // Territory, precomputed once (O(provinces), not O(cities·provinces)).
        let mut territory = vec![0.0f32; self.hubs.len()];
        for p in 0..self.prov_holder.len() {
            let h = self.prov_holder[p];
            if h >= 0 {
                territory[h as usize] += self.prov_rural.get(p).copied().unwrap_or(0.0);
            }
        }

        let pops: Vec<f32> = live.iter().map(|&h| self.hubs[h].population.max(0.0)).collect();
        let trades: Vec<f32> = live.iter()
            .map(|&h| (self.hubs[h].grain_wealth + self.hubs[h].trade_wealth).max(0.0)).collect();
        let treasuries: Vec<f32> = live.iter().map(|&h| self.hubs[h].treasury.max(0.0)).collect();
        let territories: Vec<f32> = live.iter().map(|&h| territory[h]).collect();
        let pr = rank_norm(&pops);
        let tr = rank_norm(&trades);
        let xr = rank_norm(&treasuries);
        let gr = rank_norm(&territories);

        let mut standings = vec![0.0f32; n];
        for (k, &h) in live.iter().enumerate() {
            // The office as a person (§3.1): captor outranks a merely-dominant council.
            let leader_house = if self.hubs[h].captor_house >= 0 { self.hubs[h].captor_house }
                else { self.hubs[h].council_house };
            let leader_standing = leader_house.try_into().ok()
                .and_then(|hi: usize| self.houses.get(hi))
                .filter(|hh| !hh.defunct)
                .map(|hh| hh.standing)
                .unwrap_or(0.0);
            let s = 0.20 * pr[k] + 0.15 * tr[k] + 0.25 * xr[k] + 0.20 * gr[k] + 0.20 * leader_standing;
            standings[k] = s.clamp(0.0, 1.0);
        }

        // Percentile position: 0 = the most prominent live city, 1 = the least.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| standings[b].partial_cmp(&standings[a]).unwrap_or(std::cmp::Ordering::Equal));
        let mut pct = vec![0.0f32; n];
        for (rank, &k) in order.iter().enumerate() {
            pct[k] = if n > 1 { rank as f32 / (n - 1) as f32 } else { 0.0 };
        }

        for k in 0..n {
            let h = live[k];
            self.hubs[h].standing = standings[k];
            let prev = self.hubs[h].tier;
            let new_tier = if prev == 0 {
                Self::city_tier_band(pct[k], standings[k])
            } else {
                Self::city_tier_with_hysteresis(prev, pct[k], standings[k])
            };
            // A tier RISE is chronicled (a milestone); a fall is not — the same
            // asymmetry `assign_house_tiers` uses.
            if new_tier != prev && prev != 0 && new_tier < prev {
                let name = self.hubs[h].name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "city_tier".into(), hub: h as i32, good: -1, value: new_tier as f32,
                    text: format!("{} is now counted among the {} cities", name, CITY_TIER_NAMES[new_tier as usize]),
                });
            }
            self.hubs[h].tier = new_tier;
        }
    }

    /// The RAW tier a (percentile, standing) pair bands into, with no memory of the
    /// city's previous tier. Used only for a city's first-ever assignment.
    fn city_tier_band(pct: f32, standing: f32) -> u8 {
        if pct < CITY_TIER_PCT_CUTS[0] && standing >= CITY_TIER1_STANDING_ENTER { 1 }
        else if pct < CITY_TIER_PCT_CUTS[1] { 2 }
        else if pct < CITY_TIER_PCT_CUTS[2] { 3 }
        else { 4 }
    }

    /// The tier a city holds THIS month, given the tier it held last month —
    /// identical shape to `tier_with_hysteresis`, a separate function only because
    /// the constants are city-scoped, not house-scoped.
    fn city_tier_with_hysteresis(prev: u8, pct: f32, standing: f32) -> u8 {
        let mut cuts = CITY_TIER_PCT_CUTS;
        if (2..=4).contains(&prev) { cuts[(prev - 2) as usize] -= CITY_TIER_PCT_DEAD_BAND; }
        if (1..=3).contains(&prev) { cuts[(prev - 1) as usize] += CITY_TIER_PCT_DEAD_BAND; }
        let by_rank = if pct < cuts[0] { 1 } else if pct < cuts[1] { 2 }
            else if pct < cuts[2] { 3 } else { 4 };
        if by_rank == 1 {
            let floor = if prev == 1 { CITY_TIER1_STANDING_EXIT } else { CITY_TIER1_STANDING_ENTER };
            if standing >= floor { 1 } else { 2 }
        } else {
            by_rank
        }
    }

    /// Capacity tier (1..5) for a warehouse `capacity`; 0 = the uncapped −1 pool.
    #[inline]
    pub fn capacity_tier(capacity: f32) -> u8 {
        if capacity <= 0.0 { 0 }
        else if capacity <= WH_TIER1_CAP { 1 }
        else if capacity <= WH_TIER2_CAP { 2 }
        else if capacity <= WH_TIER3_CAP { 3 }
        else if capacity <= WH_TIER4_CAP { 4 }
        else { 5 }
    }


    /// Phase 2b · WATERSHED DEMOGRAPHY (yearly). The province countryside is a living
    /// population reservoir: it grows toward the land's carrying capacity, then sheds its
    /// surplus into the cities that stand in (and administer) it — carrying its people's
    /// CULTURE with them. The largest cities also pay an "urban graveyard" mortality
    /// (crowding + endemic disease), so a metropolis genuinely DEPENDS on a fed hinterland
    /// to hold its numbers. Entirely gated on a seeded province layer, so a world without
    /// provinces (and the dynamics test) is unaffected.
    pub(crate) fn province_demography_pass(&mut self) {
        let np = self.prov_rural.len();
        if np == 0 { return; }
        // Self-heal membership for any hubs founded since the last pass (colonies,
        // swarm towns, resettled ruins) so they join their province's reservoir.
        if self.hub_province.len() < self.hubs.len() {
            let start = self.hub_province.len();
            let assigns: Vec<i32> = (start..self.hubs.len())
                .map(|h| self.province_at(self.hubs[h].x, self.hubs[h].y)).collect();
            self.hub_province.resize(self.hubs.len(), -1);
            for (i, h) in (start..self.hubs.len()).enumerate() { self.hub_province[h] = assigns[i]; }
        }
        self.prov_net_mig = vec![0.0; np];
        // Civilization advances over centuries: better FARMING raises the land's carrying
        // capacity (food production) and MEDICINE eases the urban graveyard (fighting
        // disease). A saturating function of campaign time, so the countryside keeps
        // filling and cities keep surviving larger for centuries, but always bounded.
        // Province-gated, so the province-FREE dynamics gate is unaffected by this.
        let adv = (1.0 - (-(self.tick as f32 / TICKS_PER_YEAR as f32) / 250.0).exp()).clamp(0.0, 1.0);
        let food_cap_mult = 1.0 + 0.9 * adv;         // up to ~+90% farm carrying capacity
        // 1. Rural natural increase toward (advancing) capacity — Malthusian check above it.
        for p in 0..np {
            let cap = self.prov_cap.get(p).copied().unwrap_or(0.0).max(1.0) * food_cap_mult;
            let r = self.prov_rural[p];
            if r < cap {
                self.prov_rural[p] = (r + RURAL_GROWTH * r * (1.0 - r / cap)).max(0.0);
            } else {
                self.prov_rural[p] = (r * (1.0 - 0.004)).max(cap * 0.5);
            }
        }
        // 2. Urban graveyard: the biggest cities lose a little population naturally each
        //    year (crowding + disease), softened by public health. Migration (below) is
        //    what lets them hold and grow.
        // Medicine pushes the "urban graveyard" floor UP and its mortality DOWN over the
        // campaign, so a big city no longer decays back toward ~25k as readily.
        let crowd_floor = URBAN_CROWD_FLOOR * (1.0 + 0.8 * adv);
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let pop = self.hubs[h].population;
            if pop <= crowd_floor { continue; }
            let crowd = ((pop - crowd_floor) / 120_000.0).clamp(0.0, 1.0);
            let health = self.hubs[h].public_health.clamp(0.0, 1.0);
            let mort = URBAN_CROWDING_MORTALITY * crowd * (1.0 - 0.6 * health) * (1.0 - 0.5 * adv);
            self.hubs[h].population = (pop * (1.0 - mort)).max(1.0);
        }
        // 3. Rural → urban migration, per province, weighted by each city's OPPORTUNITY
        //    (prosperity · fed · commercial standing). People flow to where life is better.
        // Group live member hubs per province.
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); np];
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < 1.0 { continue; }
            if let Some(&pid) = self.hub_province.get(h) {
                if pid >= 0 && (pid as usize) < np { members[pid as usize].push(h); }
            }
        }
        for p in 0..np {
            if members[p].is_empty() { continue; }
            let cap = self.prov_cap[p].max(1.0);
            let r = self.prov_rural[p];
            if r < 20.0 { continue; }
            // Opportunity pull per member city.
            let mut pulls: Vec<(usize, f32)> = Vec::with_capacity(members[p].len());
            let mut total = 0.0f32;
            for &h in &members[p] {
                let hub = &self.hubs[h];
                let prosp = hub.sent_prosperity.clamp(0.0, 1.0);
                let fed = (1.0 - hub.starving).clamp(0.0, 1.0);
                let pull = (0.15 + prosp) * fed * (1.0 + 0.4 * hub.hub_class as f32);
                if pull > 0.0 { pulls.push((h, pull)); total += pull; }
            }
            if total <= 0.0 { continue; }
            // Out-migration: a fraction of the pool, larger the fuller the countryside is.
            let pressure = (r / cap).clamp(0.0, 1.0);
            let out = (r * RURAL_MIGRATION_RATE * (0.3 + 0.7 * pressure)).min(r * 0.5);
            if out < 1.0 { continue; }
            let culture = self.prov_culture.get(p).cloned().unwrap_or_default();
            for (h, pull) in pulls {
                let share = out * pull / total;
                if share < 0.5 { continue; }
                self.hubs[h].population += share;
                // Migrants carry the province's people into the city (culture mixing).
                if !culture.is_empty() { self.add_minority(h, &culture, share); }
            }
            self.prov_rural[p] -= out;
            self.prov_net_mig[p] = -out; // the countryside is a net source this year
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  PROVINCE LAND STATE  (FIX_PLAN B1 — the feedback edge)
    //
    //  `province_demography_pass` above moves PEOPLE from the countryside into the
    //  cities. It does not move GRAIN, and nothing in the campaign ever reads the land
    //  back. That is the gap B1 names: five centuries pass and the world is identical,
    //  because the only thing the campaign knows about a province is how many people
    //  stand on it.
    //
    //  This pass gives the land state that CHANGES and then feeds it back:
    //
    //    woodland ⇄ arable   population pressure clears forest; abandoned land regrows
    //    soil                depletes under intensive cropping, recovers on fallow
    //    surplus  ────────►  the seat city's FOOD STOCK (the feedback edge)
    //    dues     ────────►  the holder's TREASURY (rural fiscality — the base every
    //                        pre-modern polity actually ran on, and which this model
    //                        previously lacked entirely: city treasuries came from
    //                        tariffs and seigniorage alone)
    //    unrest              from crowding, taxation and tenure concentration; every
    //                        major pre-modern revolt was rural, and unrest here was
    //                        a city-only property
    //
    //  Entirely gated on a seeded province layer — `prov_rural.is_empty()` returns at
    //  once — so a campaign without provinces (including the dynamics test) is
    //  bit-identical. Cost is O(provinces) once a year.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Yearly · the land is worked, worn, improved and taxed, and what it yields above
    /// subsistence is delivered to the city that administers it.
    pub(crate) fn province_land_pass(&mut self, yr: u32) {
        let np = self.prov_rural.len();
        if np == 0 { return; }
        self.ensure_province_land(np);
        // R3 · this year's tithe-to-crown accumulator starts fresh; the per-
        // province loop below fills it in as dues are delivered.
        for r in self.realms.iter_mut() { r.tithe_last_year = 0.0; }
        let ng = self.goods.len();
        // The staple the countryside actually grows and the city actually eats.
        let food_good = (0..ng).find(|&g| self.goods[g].food);
        // Urban population per province, for the tenure/holder logic and the sample.
        let mut urban = vec![0.0f32; np];
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            if let Some(&pid) = self.hub_province.get(h) {
                if pid >= 0 && (pid as usize) < np { urban[pid as usize] += self.hubs[h].population; }
            }
        }
        for p in 0..np {
            let cap = self.prov_cap[p].max(1.0);
            let rural = self.prov_rural[p].max(0.0);
            let pressure = (rural / cap).clamp(0.0, 2.0);

            // ── 1. Land use. People with mouths to feed clear woodland; land nobody
            //    works grows back. Clearance is bounded by what is left and by a floor
            //    (the steep, the wet and the sacred are never fully cleared).
            let forest = self.prov_forest[p];
            let arable = self.prov_arable[p];
            let want = (pressure - 0.55).max(0.0); // only a crowded countryside clears
            let cleared = (PROV_CLEAR_RATE * want).min((forest - 0.05).max(0.0));
            let abandoned = if pressure < 0.45 { PROV_REGROW_RATE * (0.45 - pressure) * 2.0 } else { 0.0 };
            let regrow = abandoned.min((arable - 0.02).max(0.0));
            self.prov_forest[p] = (forest - cleared + regrow).clamp(0.0, 1.0);
            self.prov_arable[p] = (arable + cleared - regrow).clamp(0.0, 1.0);
            // Pasture takes the slack between crop and wood.
            self.prov_pasture[p] =
                (1.0 - self.prov_forest[p] - self.prov_arable[p]).clamp(0.0, 1.0) * 0.55;

            // ── 2. Soil. Cropping intensity is people per unit of arable, not raw
            //    population — the same crowd on twice the land wears it half as fast,
            //    which is why clearance buys time and irrigation buys more.
            let arable_now = self.prov_arable[p].max(0.02);
            let intensity = (pressure / (arable_now / 0.35).max(0.35)).clamp(0.0, 2.0);
            let irr = self.prov_irrigated[p].clamp(0.0, 1.0);
            let wear = PROV_DEPLETE * intensity * (1.0 - 0.45 * irr);
            let heal = PROV_RECOVER * (1.0 - (intensity / 1.4).min(1.0));
            self.prov_soil[p] = (self.prov_soil[p] - wear + heal).clamp(PROV_SOIL_FLOOR, 1.0);

            // ── 3. Works under way (clearance / drainage / irrigation / road).
            //    Handled per-province below so its progress lands before the harvest.
            self.advance_province_works(p, yr);

            // ── 4. The harvest. Land quality × soil condition × irrigation, against
            //    what the countryside eats itself. A province whose soil is exhausted
            //    and whose people are many produces NOTHING to spare — which is the
            //    Malthusian squeeze the demography pass was already half-modelling.
            // The multiplier is CENTRED ON 1.0 for ordinary land — soil in good heart and
            // the province's typical share under the plough. Getting this wrong is not a
            // flavour error: the first cut averaged ~0.7, which put gross output below
            // rural subsistence on decent land, so NO province ever had a surplus and the
            // feedback edge silently delivered nothing. The surplus share it now yields
            // (~20-25% of gross on average land) is the band that supports the 10-20%
            // urbanisation rates the economy oracle scores against.
            let soil_term = 0.50 + 0.50 * self.prov_soil[p];
            let arable_term = (arable_now / PROV_ARABLE_REFERENCE).clamp(0.30, 1.60);
            let land = soil_term * arable_term * (1.0 + PROV_IRRIGATION_GAIN * irr);
            let gross = rural * PROV_YIELD_PER_HEAD * land.clamp(0.15, 2.0);
            let eaten = rural * PROV_SUBSISTENCE;
            let surplus = (gross - eaten).max(0.0);
            self.prov_surplus[p] = surplus;

            // ── 5. Dues. The holder taxes the surplus; unrest and distance turn part
            //    of it into arrears rather than revenue.
            let rate = self.prov_tax[p].clamp(0.0, PROV_TAX_MAX);
            let unrest = self.prov_unrest[p].clamp(0.0, 1.0);
            let assessed = surplus * rate;
            let evaded = assessed * (0.15 + 0.55 * unrest)
                * if unrest > PROV_REVOLT_AT { PROV_REVOLT_LOSS } else { 0.35 };
            let collected = (assessed - evaded).max(0.0);
            self.prov_arrears[p] = (self.prov_arrears[p] + evaded) * 0.85; // old arrears fade
            self.prov_revenue[p] = collected;

            // ── 6. Delivery — THE FEEDBACK EDGE. What the land grew above subsistence
            //    and above the tax reaches the seat city's granary; the dues reach its
            //    treasury. This is the contado feeding its city, and it is the first
            //    thing in the campaign that makes a hinterland matter as land rather
            //    than as a population reservoir.
            let holder = self.province_seat_hub(p);
            // Phase 5 (`HOUSE_INHERITANCE_AND_TERRITORY.md` Part D) · a house may hold
            // this province's writ instead of the seat city (the Stato da Mar case).
            // Released back to the city automatically the moment the holder is gone —
            // `prov_holder_house` never points at a defunct house.
            let house_holder = self.prov_holder_house.get(p).copied().unwrap_or(-1);
            let house_holds = house_holder >= 0 && (house_holder as usize) < self.houses.len()
                && !self.houses[house_holder as usize].defunct;
            if !house_holds && house_holder >= 0 { self.prov_holder_house[p] = -1; }
            if let Some(seat) = holder {
                // The land still physically feeds the seat city either way — only the
                // MONETARY dues redirect to a house holder.
                let to_market = (surplus - collected).max(0.0);
                if let Some(fg) = food_good {
                    if fg < self.goods.len() {
                        stock_add_ungraded(&mut self.hubs[seat].stock, fg, to_market);
                    }
                }
                if house_holds {
                    self.houses[house_holder as usize].wealth += collected;
                } else {
                    // R3 · a sovereign province's dues go to the CROWN, scaled by
                    // collection efficiency (§3.3 — pre-modern states were limited
                    // by what they could COLLECT, not what they charged). What
                    // efficiency doesn't collect reaches neither crown nor city —
                    // a real administrative loss, the same shape `evaded` already
                    // has a few lines above, not money quietly retained locally.
                    let realm_i = self.prov_realm.get(p).copied().unwrap_or(-1);
                    let realm_i = if realm_i >= 0 { Some(realm_i as usize) } else { None }
                        .filter(|&ri| ri < self.realms.len() && self.realms[ri].fallen_tick == 0);
                    match realm_i {
                        Some(ri) => {
                            let efficiency = self.realm_collection_efficiency(ri, seat);
                            // R5 · the autonomy axis's own "Revenue" column — a
                            // centralized crown squeezes harder, an autonomous one
                            // leaves more with its provinces.
                            let to_crown = collected * efficiency * autonomy_revenue_mult(self.realms[ri].autonomy);
                            // A tax farm on the tithe redirects the crown's share to
                            // the farming house instead — it already paid up front.
                            let farmer = self.realms[ri].tax_farm.as_ref().map(|f| f.house);
                            match farmer {
                                Some(hi) if (hi as usize) < self.houses.len()
                                    && self.houses[hi as usize].is_merchant() => {
                                    self.houses[hi as usize].wealth += to_crown;
                                }
                                _ => { self.realms[ri].treasury += to_crown; }
                            }
                            self.realms[ri].tithe_last_year += to_crown;
                        }
                        None => { self.hubs[seat].treasury += collected; }
                    }
                }
                self.prov_holder[p] = seat as i32;
            } else {
                self.prov_holder[p] = -1;
            }

            // ── 7. Tenure drifts toward who actually holds the land. Every estate a
            //    house founds here converts common land into a private holding — the
            //    engrossment that drove enclosure and the second serfdom alike. Slow,
            //    and it eases back when houses withdraw.
            {
                let mut house_land = 0.0f32;
                for h in 0..self.hubs.len() {
                    if !self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
                    if self.hub_province.get(h).copied().unwrap_or(-1) != p as i32 { continue; }
                    if self.hubs[h].owner_house >= 0 { house_land += 0.04; }
                }
                let target = house_land.min(0.60);
                let t = &mut self.prov_tenure[p];
                let shift = (target - t[1]) * 0.10;
                t[1] = (t[1] + shift).clamp(0.0, 0.75);
                // The commons absorb the change; civic and temple shares hold.
                t[3] = (1.0 - t[0] - t[1] - t[2]).clamp(0.0, 1.0);
            }

            // ── 8. Unrest. Crowding, taxation beyond what is tolerated, a failed
            //    harvest, and land concentrated in few hands. Calms when none apply.
            let tenure = self.prov_tenure[p];
            let concentration = (tenure[1] - 0.35).max(0.0); // house/noble share above a third
            let dearth = if gross < eaten * 1.02 { 1.0 } else { 0.0 };
            let crowd = (pressure - 1.0).max(0.0);
            let up = PROV_UNREST_CROWD * crowd
                + PROV_UNREST_TAX * (rate - PROV_TAX_TOLERATED).max(0.0)
                + PROV_UNREST_DEARTH * dearth
                + 0.25 * concentration;
            let next = if up > 0.0 { unrest + up } else { unrest - PROV_UNREST_CALM };
            self.prov_unrest[p] = next.clamp(0.0, 1.0);

            // ── 9. Revolt. The countryside rises: dues stop, the seat's mood suffers,
            //    and the province remembers it.
            if self.prov_unrest[p] >= PROV_REVOLT_AT {
                let roll = hash01(self.seed, yr as u64 ^ 0x5EED, p as u64);
                if roll < 0.35 {
                    self.prov_unrest[p] = (self.prov_unrest[p] - 0.30).max(0.0);
                    self.prov_revenue[p] = 0.0;
                    // Phase 5 · a house-held province's revolt costs the HOUSE, not the
                    // seat's civic mood — prestige falls and a slice of wealth is lost,
                    // the same "unrest is directed at the holder" the design asks for.
                    if house_holds {
                        let hi = house_holder as usize;
                        self.houses[hi].prestige = (self.houses[hi].prestige - 0.10).max(0.0);
                        let loss = (self.houses[hi].wealth.max(0.0) * 0.05).max(0.0);
                        self.houses[hi].wealth -= loss;
                        let (pn, hname) = (self.province_name(p), self.houses[hi].name.clone());
                        self.houses[hi].events.push(HouseEvent {
                            tick: self.tick, kind: "loss".into(),
                            text: format!("the province of {} rises against {}'s rule", pn, hname),
                        });
                        self.push_prov_event(p, yr, "revolt",
                            format!("The countryside rose against {}'s rule — dues went uncollected", hname));
                        self.journal.push(JournalEntry {
                            tick: self.tick, kind: "revolt".into(), hub: -1, good: -1,
                            value: 0.0,
                            text: format!("The countryside of {} rises against {}'s rule", pn, hname),
                        });
                    } else if let Some(seat) = holder {
                        self.hubs[seat].sent_stability = (self.hubs[seat].sent_stability - 0.18).max(0.0);
                        let (pn, sn) = (self.province_name(p), self.hubs[seat].name.clone());
                        self.push_prov_event(p, yr, "revolt",
                            format!("The countryside rose against {} — dues went uncollected", sn));
                        self.journal.push(JournalEntry {
                            tick: self.tick, kind: "revolt".into(), hub: seat as i32, good: -1,
                            value: 0.0,
                            text: format!("The countryside of {} rises against the dues of {}", pn, sn),
                        });
                    }
                }
            } else if dearth > 0.0 && rural > 40.0 {
                let roll = hash01(self.seed, yr as u64 ^ 0xDEA2, p as u64);
                if roll < 0.20 {
                    let pn = self.province_name(p);
                    self.push_prov_event(p, yr, "dearth",
                        format!("A failed harvest in {} — the land fed no-one but itself", pn));
                }
            }

            // ── 10. Sample the year for the panel's time slider.
            let s = ProvSample {
                year: yr, rural, urban: urban[p],
                forest: self.prov_forest[p], arable: self.prov_arable[p],
                pasture: self.prov_pasture[p], irrigated: self.prov_irrigated[p],
                soil: self.prov_soil[p], unrest: self.prov_unrest[p],
                surplus: self.prov_surplus[p],
            };
            let hist = &mut self.prov_history[p];
            hist.push(s);
            if hist.len() > PROV_HISTORY_CAP { let d = hist.len() - PROV_HISTORY_CAP; hist.drain(0..d); }
        }
        self.maybe_grant_provinces(yr);
    }

    /// CITY_PROVINCE_WAR_PLAN.md §2.5 · a good's land-use category, from a small
    /// name table over the shipped goods (`GOOD_NAMES`) — `TickGood` doesn't carry
    /// the world half's `Domain`/`Distribution` classification, so this is a
    /// pragmatic stand-in rather than a full schema addition. Unrecognized/custom
    /// goods default to UNCONSTRAINED (share 1.0): never wrongly shrinking a good
    /// we don't understand is safer than guessing wrong. 0 unconstrained (marine,
    /// mineral, unknown) · 1 forest · 2 arable · 3 pasture.
    fn good_land_kind(name: &str) -> u8 {
        match name {
            "timber" | "furs" | "hardwoods" | "honey" => 1,
            "wheat" | "rice" | "barley" | "millet" | "cotton" | "sugar" | "tobacco"
                | "indigo" | "dates" | "cacao" | "silk" | "wine" | "oliveoil" => 2,
            "wool_fleece" | "wool_llama" | "horses" | "hides" => 3,
            _ => 0,
        }
    }

    /// The share of the province's land this good's potential scales by. Manufactured
    /// goods (non-empty `inputs`) are excluded entirely by the CALLER — a manufactory
    /// extracts nothing from the land (§1.2), so they never reach this at all.
    fn province_good_land_share(&self, p: usize, g: usize) -> f32 {
        let Some(name) = self.goods.get(g).map(|tg| tg.name.as_str()) else { return 1.0 };
        match Self::good_land_kind(name) {
            1 => self.prov_forest.get(p).copied().unwrap_or(0.3),
            2 => self.prov_arable.get(p).copied().unwrap_or(0.2),
            3 => self.prov_pasture.get(p).copied().unwrap_or(0.2),
            _ => 1.0,
        }
        .clamp(0.02, 1.0)
    }

    /// Σ hub + estate production of every good, by province — a plain re-attribution
    /// of production the day loop already computed (no new production, see §2.5's
    /// own formula: "actual = production of hubs + estates here"). Flat `np * ng`.
    pub(crate) fn province_good_actual(&self) -> Vec<f32> {
        let ng = self.goods.len();
        let np = if ng > 0 { self.prov_good_belt.len() / ng } else { 0 };
        let mut out = vec![0.0f32; np * ng];
        if np == 0 { return out; }
        for (h, hub) in self.hubs.iter().enumerate() {
            if hub.abandoned { continue; }
            let p = self.hub_province.get(h).copied().unwrap_or(-1);
            if p < 0 || p as usize >= np { continue; }
            let base = p as usize * ng;
            for g in 0..ng.min(hub.production.len()) {
                out[base + g] += hub.production[g].max(0.0);
            }
        }
        out
    }

    /// Raw capacity BEFORE the world calibration scalar and BEFORE depletion —
    /// `belt_score · land carrying capacity (prov_cap, the best area proxy the
    /// campaign holds) · live land-use share`. §5.2: scales the FROZEN belt by LIVE
    /// land use every call, which is what lets clearing forest actually shrink
    /// timber capacity instead of the exploitation layer being scenery.
    fn province_good_potential_base(&self, p: usize, g: usize) -> f32 {
        let ng = self.goods.len();
        if ng == 0 { return 0.0; }
        let belt = self.prov_good_belt.get(p * ng + g).copied().unwrap_or(0.0);
        if belt <= 0.0 { return 0.0; }
        let cap = self.prov_cap.get(p).copied().unwrap_or(0.0).max(0.0);
        belt * cap * self.province_good_land_share(p, g)
    }

    /// The calibrated, depleted potential — what `exploitation = actual/potential`
    /// (§2.5's query-time formula) divides by.
    pub(crate) fn province_good_potential(&self, p: usize, g: usize) -> f32 {
        let ng = self.goods.len();
        if ng == 0 { return 0.0; }
        let base = self.province_good_potential_base(p, g);
        if base <= 0.0 { return 0.0; }
        let depletion = self.prov_good_depletion.get(p * ng + g).copied().unwrap_or(0.0);
        base * self.prov_good_yield_scale.max(0.01) * (1.0 - depletion.clamp(0.0, PROV_GOOD_DEPLETION_CAP))
    }

    /// Once, at campaign start (`lifecycle.rs`): scale the world-wide yield constant
    /// so mean exploitation reads ≈1.0 on day one, whatever this particular world's
    /// belt intensities and province sizes happen to be — the same self-calibration
    /// `need_scale` already uses, so this doesn't need its own hand-picked constant
    /// that would silently read wrong on a differently-shaped world.
    pub(crate) fn calibrate_province_good_yield(&mut self) {
        let ng = self.goods.len();
        let np = if ng > 0 { self.prov_good_belt.len() / ng } else { 0 };
        if np == 0 { self.prov_good_yield_scale = 1.0; return; }
        let actual = self.province_good_actual();
        let mut sum_actual = 0.0f64;
        let mut sum_base = 0.0f64;
        for p in 0..np {
            for g in 0..ng {
                let a = actual[p * ng + g];
                if a <= 0.0 { continue; }
                sum_actual += a as f64;
                sum_base += self.province_good_potential_base(p, g) as f64;
            }
        }
        self.prov_good_yield_scale = if sum_base > 1e-6 {
            ((sum_actual / sum_base) as f32).clamp(0.02, 50.0)
        } else { 1.0 };
    }

    /// Which estate KIND, if any, is this province's main producer of good `g` —
    /// drives the kind-specific depletion behaviour below (§1.2: mine exhausts,
    /// fishery collapses and recovers, plantation wears soil, vineyard doesn't lose
    /// tonnage). 0 = no estate found (ordinary rural production; the default rate).
    fn dominant_estate_kind(&self, p: usize, g: usize) -> u8 {
        for (h, hub) in self.hubs.iter().enumerate() {
            if !hub.is_estate || hub.abandoned { continue; }
            if self.hub_province.get(h).copied().unwrap_or(-1) != p as i32 { continue; }
            let main = hub.production.iter().enumerate()
                .filter(|&(_, &v)| v > 0.0)
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((mg, _)) = main {
                if mg == g { return hub.estate_kind; }
            }
        }
        0
    }

    /// §2.5 · the MARKET ↔ LOCAL split — what share of a province's production of
    /// good `g` never enters trade at all, because the very population that grew it
    /// consumes it. Reuses `base_need`, the real per-hub local-demand formula the
    /// day loop itself prices against (not a reinvented estimate), summed over
    /// every hub in the province. "Most output never enters trade" is the true
    /// pre-modern picture (§2.5) — this is what lets the Goods tab show it honestly.
    pub(crate) fn province_good_market_share(&self, p: usize, g: usize, actual: f32) -> f32 {
        if actual <= EPS || g >= self.goods.len() { return 0.0; }
        let mut local_demand = 0.0f32;
        for (h, hub) in self.hubs.iter().enumerate() {
            if hub.abandoned { continue; }
            if self.hub_province.get(h).copied().unwrap_or(-1) != p as i32 { continue; }
            local_demand += self.base_need(h, g);
        }
        ((actual - local_demand) / actual).clamp(0.0, 1.0)
    }

    /// §2.5 · once a year, right after the land pass so this year's forest/arable/
    /// pasture are fresh: update the ONE piece of exploitation state that actually
    /// needs memory — a per-(province, good) depletion multiplier — from how hard
    /// each good is being worked. Reuses `prov_soil`'s own wear/heal SHAPE (erode
    /// above 1.0 exploitation, recover below it); `potential`/`actual` themselves
    /// stay pure derived reads (`province_good_potential`/`_actual` above), so
    /// nothing about actual hub production, prices or the `econ_` bands changes —
    /// this pass writes ONLY `prov_good_depletion` (and, for a plantation under
    /// pressure, a small extra nudge to the ordinary `prov_soil`).
    pub(crate) fn update_province_goods_pressure(&mut self, _yr: u32) {
        let ng = self.goods.len();
        let np = if ng > 0 { self.prov_good_belt.len() / ng } else { 0 };
        if np == 0 { return; }
        let actual = self.province_good_actual();
        for p in 0..np {
            for g in 0..ng {
                let idx = p * ng + g;
                if self.prov_good_belt.get(idx).copied().unwrap_or(0.0) <= 0.001 { continue; }
                let potential = self.province_good_potential(p, g);
                if potential <= EPS { continue; }
                let exploitation = actual[idx] / potential;
                let pressure = (exploitation - 1.0).max(0.0);
                let ease = (1.0 - exploitation).max(0.0);
                let kind = self.dominant_estate_kind(p, g);
                let (wear_k, recover_k): (f32, f32) = match kind {
                    2 => (1.3, 0.15), // mine — "exhausts": wears fast, almost never heals
                    4 => (0.8, 2.2),  // fishery — "collapses and recovers": hard down, fast back
                    5 => (0.0, 1.0),  // vineyard — doesn't lose tonnage under pressure
                    _ => (1.0, 1.0),
                };
                let wear = PROV_GOOD_DEPLETE * pressure * wear_k;
                let heal = PROV_GOOD_RECOVER * ease * recover_k;
                let d = self.prov_good_depletion[idx];
                self.prov_good_depletion[idx] = (d + wear - heal).clamp(0.0, PROV_GOOD_DEPLETION_CAP);
                // "plantation wears soil" — a real cross-link into the ordinary soil
                // condition, on top of the good's own depletion.
                if kind == 3 && pressure > 0.0 {
                    if let Some(s) = self.prov_soil.get_mut(p) {
                        *s = (*s - PROV_DEPLETE * pressure * 0.5).max(PROV_SOIL_FLOOR);
                    }
                }
            }
        }
    }

    /// Phase 5 · the Stato da Mar case: a province with no house holder may be
    /// GRANTED to whichever house already dominates its seat city — holds its
    /// council/captor seat OR its bailo (the exact "seats" a house's `standing`
    /// already counts in `assign_house_tiers`), is Tier 1-2, and the province is not
    /// currently in open revolt. Deliberately narrow (see the constants' own doc):
    /// this is a house's reach over its OWN seat's hinterland extending outward, not
    /// a land-grab anywhere. Contesting a HELD province (war, a rival house) is
    /// explicitly NOT built — `HOUSE_INHERITANCE_AND_TERRITORY.md` Part D's own "war
    /// goals gain a territorial option" is a materially bigger item (new war-goal
    /// machinery) than this pass's scope; a granted province is sticky until its
    /// holder is gone.
    pub(crate) fn maybe_grant_provinces(&mut self, yr: u32) {
        let np = self.prov_holder_house.len();
        for p in 0..np {
            if self.prov_holder_house[p] >= 0 { continue; }
            if self.prov_unrest.get(p).copied().unwrap_or(1.0) > PROV_GRANT_UNREST_MAX { continue; }
            let Some(seat) = self.province_seat_hub(p) else { continue; };
            let dominates = self.hubs.get(seat)
                .is_some_and(|h| h.council_house >= 0 || h.captor_house >= 0);
            if !dominates { continue; }
            let (council, captor) = (self.hubs[seat].council_house, self.hubs[seat].captor_house);
            let candidate = self.houses.iter().enumerate().find(|(hi, h)| {
                !h.defunct && !h.is_guild && h.tier >= 1 && h.tier <= PROV_GRANT_TIER_MAX
                    && (council == *hi as i32 || captor == *hi as i32 || h.bailos.contains(&(seat as u32)))
            }).map(|(hi, _)| hi);
            let Some(hi) = candidate else { continue; };
            if hash01(self.seed, yr as u64 ^ 0x9A0D, p as u64) >= PROV_GRANT_CHANCE { continue; }
            self.prov_holder_house[p] = hi as i32;
            let (pn, hname) = (self.province_name(p), self.houses[hi].name.clone());
            self.houses[hi].events.push(HouseEvent {
                tick: self.tick, kind: "province_granted".into(),
                text: format!("{} is granted the province of {} — its writ now runs beyond the city walls", hname, pn),
            });
            self.push_prov_event(p, yr, "granted",
                format!("The province passes to {}'s administration", hname));
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "province_granted".into(), hub: seat as i32, good: -1, value: 0.0,
                text: format!("{} is granted the province of {}", hname, pn),
            });
        }
    }

    /// Grow the land-state vectors to `np` and seed any province that has none yet.
    /// Called from the land pass rather than only at campaign start, so a province
    /// layer joined mid-campaign (or a save written before this existed) fills in.
    pub(crate) fn ensure_province_land(&mut self, np: usize) {
        if self.prov_forest.len() >= np && self.prov_history.len() >= np { return; }
        // Initial land use from what the world half already knows: fertile, wet, low
        // country starts largely wooded with a modest clearing around the seat; arid or
        // upland country starts open. `prov_cap` is the land's food potential, which is
        // the best proxy the campaign holds for all of it.
        let cap_max = self.prov_cap.iter().copied().fold(1.0f32, f32::max);
        let seed_one = |p: usize, cap: f32| -> (f32, f32, f32) {
            let quality = (cap / cap_max).clamp(0.0, 1.0);
            let forest = (0.20 + 0.55 * quality).clamp(0.05, 0.80);
            // Arable must fit in what the woodland leaves — the two are shares of the
            // SAME province. Without this cap the fallback seeding could hand out
            // forest 0.75 + arable 0.38 = 1.13 of a province, which
            // `province_land_pass_feeds_the_seat_and_stays_bounded` catches.
            let arable = (0.08 + 0.30 * quality).clamp(0.03, 0.45).min((1.0 - forest).max(0.02));
            let soil = (0.55 + 0.35 * quality).clamp(0.35, 0.95);
            (forest, arable, soil)
        };
        for p in self.prov_forest.len()..np {
            let cap = self.prov_cap.get(p).copied().unwrap_or(1.0);
            let (f, a, s) = seed_one(p, cap);
            self.prov_forest.push(f);
            self.prov_arable.push(a);
            self.prov_pasture.push(((1.0 - f - a).max(0.0) * 0.55).clamp(0.0, 1.0));
            self.prov_irrigated.push(0.0);
            self.prov_soil.push(s);
            // Tenure: mostly common land with a civic and a temple share, the
            // house/noble share growing later as estates are actually founded here.
            self.prov_tenure.push([0.18, 0.10, 0.09, 0.63]);
            self.prov_tax.push(PROV_TAX_DEFAULT);
            self.prov_arrears.push(0.0);
            self.prov_unrest.push(0.0);
            self.prov_surplus.push(0.0);
            self.prov_revenue.push(0.0);
            self.prov_holder.push(-1);
            self.prov_holder_house.push(-1);
        }
        // The remaining vectors are plain per-province containers. `prov_holder_house`
        // is included here too — an old save whose OTHER province vectors already
        // reached `np` (so the loop above never ran) still needs it backfilled, since
        // it's a field newer than the rest of the land layer.
        if self.prov_history.len() < np { self.prov_history.resize(np, Vec::new()); }
        if self.prov_events.len() < np { self.prov_events.resize(np, Vec::new()); }
        if self.prov_holder_house.len() < np { self.prov_holder_house.resize(np, -1); }
        // Sovereignty (rule 25). −1 = free land, which is the default every province
        // starts and most provinces end in — a realm only ever claims what it takes.
        if self.prov_realm.len() < np { self.prov_realm.resize(np, -1); }
        // §2.5 · flat `np * ng` arrays. A province that arrives via this fallback
        // path (no `Province.good_belt` in hand — only `campaign_start_sim`'s own
        // seeding has that) simply gets zero belt score everywhere, which the
        // exploitation query already treats as "nothing known to grow here".
        let ng = self.goods.len();
        if self.prov_good_belt.len() < np * ng { self.prov_good_belt.resize(np * ng, 0.0); }
        if self.prov_good_depletion.len() < np * ng { self.prov_good_depletion.resize(np * ng, 0.0); }
    }

    /// The hub that administers province `p`: its largest live member city. Returns
    /// `None` for a province with no standing town (a frontier — nobody collects there).
    pub(crate) fn province_seat_hub(&self, p: usize) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            if self.hub_province.get(h).copied().unwrap_or(-1) != p as i32 { continue; }
            let pop = self.hubs[h].population;
            // Ties break on the LOWER hub index so the seat is order-independent.
            if best.map(|(_, bp)| pop > bp).unwrap_or(true) { best = Some((h, pop)); }
        }
        best.map(|(h, _)| h)
    }

    /// A province's display name — its seat city's, falling back to its culture. The
    /// campaign does not carry the worldgen province names (they live in `metadata`),
    /// so the chronicle names the place by the town that speaks for it.
    pub(crate) fn province_name(&self, p: usize) -> String {
        if let Some(h) = self.province_seat_hub(p) {
            return format!("the {} country", self.hubs[h].name);
        }
        match self.prov_culture.get(p) {
            Some(c) if !c.is_empty() => format!("the {} country", c),
            _ => format!("province {}", p),
        }
    }

    /// Append to a province's own chronicle (bounded).
    pub(crate) fn push_prov_event(&mut self, p: usize, yr: u32, kind: &str, text: String) {
        if self.prov_events.len() <= p { self.prov_events.resize(p + 1, Vec::new()); }
        let v = &mut self.prov_events[p];
        v.push(ProvEvent { year: yr, kind: kind.to_string(), text });
        if v.len() > PROV_EVENTS_CAP { let d = v.len() - PROV_EVENTS_CAP; v.drain(0..d); }
    }

    /// Advance every land improvement under way in province `p`. Funded out of the
    /// funder's treasury (a polis) or wealth (a house); unpaid work STALLS and slowly
    /// decays rather than failing outright — the same forgiving shape the satellite
    /// construction system uses, and for the same reason: a project the player cannot
    /// see failing is a project they cannot learn from.
    fn advance_province_works(&mut self, p: usize, yr: u32) {
        let idx: Vec<usize> = self.prov_works.iter().enumerate()
            .filter(|(_, w)| w.province as usize == p && w.progress < 1.0)
            .map(|(i, _)| i).collect();
        for wi in idx {
            let kind = self.prov_works[wi].kind as usize;
            if kind >= WORK_KINDS.len() { continue; }
            let cost = WORK_COST[kind];
            // Draw the year's cost. A polis pays from its treasury, a house from wealth.
            let paid = {
                let fh = self.prov_works[wi].funder_hub;
                let fs = self.prov_works[wi].funder_house;
                if fh >= 0 && (fh as usize) < self.hubs.len() && self.hubs[fh as usize].treasury >= cost {
                    self.hubs[fh as usize].treasury -= cost; true
                } else if fs >= 0 && (fs as usize) < self.houses.len()
                    && self.houses[fs as usize].wealth >= cost * 1.5 {
                    self.houses[fs as usize].wealth -= cost; true
                } else { false }
            };
            if !paid {
                let w = &mut self.prov_works[wi];
                w.idle_years += 1;
                if w.idle_years > 2 { w.progress = (w.progress - 0.05).max(0.0); }
                continue;
            }
            let step = 1.0 / WORK_YEARS[kind].max(1.0);
            let w = &mut self.prov_works[wi];
            w.idle_years = 0;
            w.progress = (w.progress + step).min(1.0);
            if w.progress < 1.0 { continue; }
            // Completed — apply the improvement to the land.
            let k = w.kind;
            match k {
                WORK_CLEAR => {
                    let take = self.prov_forest[p].min(0.10);
                    self.prov_forest[p] -= take;
                    self.prov_arable[p] = (self.prov_arable[p] + take).min(1.0);
                }
                WORK_DRAIN => {
                    self.prov_arable[p] = (self.prov_arable[p] + 0.08).min(1.0);
                    self.prov_soil[p] = (self.prov_soil[p] + 0.06).min(1.0);
                }
                WORK_IRRIGATE => {
                    self.prov_irrigated[p] = (self.prov_irrigated[p] + 0.25).min(1.0);
                }
                _ => {
                    // A made road: dues arrive instead of vanishing into arrears, and
                    // the countryside is less sullen about paying them.
                    self.prov_arrears[p] *= 0.5;
                    self.prov_unrest[p] = (self.prov_unrest[p] - 0.08).max(0.0);
                }
            }
            let pn = self.province_name(p);
            let label = WORK_KINDS[k as usize];
            self.push_prov_event(p, yr, label, format!("{} completed in {}", label, pn));
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "public_works".into(),
                hub: self.prov_works[wi].funder_hub, good: -1, value: 0.0,
                text: format!("{} completed in {}", label, pn),
            });
        }
        self.prov_works.retain(|w| w.progress < 1.0);
    }

    /// The province id a world cell (x,y) falls in, via the nearest seat (used to place
    /// campaign-founded hubs). -1 when no province layer is seeded.
    pub(crate) fn province_at(&self, x: f32, y: f32) -> i32 {
        if self.prov_seat.is_empty() { return -1; }
        let mut best = (-1i32, f32::INFINITY);
        for (i, s) in self.prov_seat.iter().enumerate() {
            let mut dx = (s[0] - x).abs();
            if self.world_w > 1.0 && dx > self.world_w / 2.0 { dx = self.world_w - dx; }
            let d = dx * dx + (s[1] - y) * (s[1] - y);
            if d < best.1 { best = (i as i32, d); }
        }
        best.0
    }


    /// Update each hub's mood and its three drivers (food / prosperity /
    /// stability), easing toward a target so the mood drifts rather than jumps.
    pub(crate) fn update_sentiment(&mut self) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        const EASE: f32 = 0.12;
        for h in 0..n {
            // Food security — the inverse of accumulated starvation pressure.
            let target_food = (1.0 - self.hubs[h].starving).clamp(0.0, 1.0);
            // Prosperity — saturating curve over grain + trade wealth + the civic
            // money the resident merchant houses spend locally (Phase G: trade
            // wealth reaching the populace). Per-capita so a feast lifts a town more
            // than a metropolis. The pool then decays (the money is spent through).
            let civic_pc = self.hubs[h].civic_pool / self.hubs[h].population.max(1.0) * 100.0;
            let w = (self.hubs[h].grain_wealth * 0.4 + self.hubs[h].trade_wealth * 0.8
                + civic_pc * 0.6).max(0.0);
            self.hubs[h].civic_pool *= CIVIC_DECAY;
            let target_prosp = (w / (w + 1.2)).clamp(0.0, 1.0);
            // Stability — lowered by active shocks on this hub (or world-wide) and
            // by widespread dearth (goods priced far above their world value).
            let mut hostility = 0.0f32;
            for e in &self.active_events {
                if e.hub == h as i32 { hostility += e.magnitude.max(0.25) + 0.1; }
                else if e.hub < 0 { hostility += 0.15; }
            }
            let mut dear = 0.0f32;
            for g in 0..ng {
                if self.hubs[h].price[g] > self.goods[g].base_value * 2.2 { dear += 1.0; }
            }
            let dear_frac = if ng > 0 { dear / ng as f32 } else { 0.0 };
            let target_stab = (1.0 - hostility - 0.5 * dear_frac).clamp(0.1, 1.0);

            let hb = &mut self.hubs[h];
            hb.sent_food += (target_food - hb.sent_food) * EASE;
            hb.sent_prosperity += (target_prosp - hb.sent_prosperity) * EASE;
            hb.sent_stability += (target_stab - hb.sent_stability) * EASE;
            let target_mood = 0.45 * hb.sent_food + 0.30 * hb.sent_prosperity + 0.25 * hb.sent_stability;
            hb.mood += (target_mood - hb.mood) * EASE;
        }
    }


    /// Seed a settlement's social strata to its structural target. Called once per hub
    /// (first advance / a freshly founded colony), then evolved by `update_society`.
    pub(crate) fn seed_society(&mut self, h: usize) {
        let (p, b, c, u) = self.target_shares(h);
        {
            let so = &mut self.hubs[h].society;
            so.patrician = p; so.burgher = b; so.commoner = c; so.underclass = u;
        }
        let (cw, ineq) = self.society_metrics(h);
        let so = &mut self.hubs[h].society;
        so.commoner_wealth = cw;
        so.inequality = ineq;
    }


    /// Derived strata read-outs from the live economy: per-capita money reaching the
    /// commons, and an inequality index (elite wealth per head vs the commoner's).
    pub(crate) fn society_metrics(&self, h: usize) -> (f32, f32) {
        let hub = &self.hubs[h];
        let pop = hub.population.max(1.0);
        let civic_pc = hub.civic_pool / pop * 100.0; // same scale as update_sentiment
        let food_aff = (1.0 - hub.starving).clamp(0.0, 1.0);
        let tax = hub.tariff_export + hub.tariff_import;
        let commoner_wealth = (civic_pc * 0.5 + hub.grain_wealth * 0.3 + food_aff * 0.4 - tax).max(0.0);
        let elite_w: f32 = self.houses.iter()
            .filter(|hh| !hh.defunct && hh.hub as usize == h)
            .map(|hh| hh.wealth.max(0.0)).sum::<f32>()
            + hub.treasury.max(0.0);
        let elite_heads = ((hub.society.patrician + hub.society.burgher) * pop).max(1.0);
        let elite_pc = elite_w / elite_heads;
        let ratio = elite_pc / (commoner_wealth + 0.5);
        let inequality = (ratio / (ratio + 8.0)).clamp(0.0, 1.0);
        (commoner_wealth, inequality)
    }


    /// Yearly social mobility: prosperity lifts families up the strata; hardship and
    /// shocks push them down (swelling the underclass). Shares stay bounded + Σ=1.
    /// Then refresh the derived `commoner_wealth` / `inequality` read-outs (eased).
    pub(crate) fn update_society(&mut self) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            // A colony founded after the one-time seed starts blank — seed it lazily.
            let s0 = &self.hubs[h].society;
            if s0.patrician + s0.burgher + s0.commoner + s0.underclass < 1e-3 {
                self.seed_society(h);
            }
            let (lackb, starv) = {
                let hub = &self.hubs[h];
                (hub.lack_basic.clamp(0.0, 1.0), hub.starving.clamp(0.0, 1.0))
            };
            let shock: f32 = self.active_events.iter()
                .filter(|e| e.hub == h as i32).map(|e| e.magnitude.max(0.2)).sum::<f32>().min(1.0);
            // Hardship (famine / dearth / disaster) skews the target DOWN: it drains the
            // upper tiers toward the underclass for as long as the hard times last.
            let hard = (starv * 0.6 + lackb * 0.4 + shock * 0.3).clamp(0.0, 0.7);
            let (tp, tb, tc, _tu) = self.target_shares(h);
            let drain = hard * 0.5;
            let (mut p, mut b, mut c) = (tp * (1.0 - drain), tb * (1.0 - drain), tc * (1.0 - drain * 0.5));
            let mut u = (1.0 - p - b - c).max(0.0);
            let tot = (p + b + c + u).max(1e-6);
            p /= tot; b /= tot; c /= tot; u /= tot;
            // Ease the LIVE shares toward this (skewed) target — gradual mobility, so
            // shocks register over a few years and recovery takes a few more.
            let ease = STRATA_MOBILITY_RATE * 2.5; // ~0.10/yr
            {
                let so = &mut self.hubs[h].society;
                so.patrician += (p - so.patrician) * ease;
                so.burgher += (b - so.burgher) * ease;
                so.commoner += (c - so.commoner) * ease;
                so.underclass += (u - so.underclass) * ease;
                let t = (so.patrician + so.burgher + so.commoner + so.underclass).max(1e-6);
                so.patrician /= t; so.burgher /= t; so.commoner /= t; so.underclass /= t;
            }
            let (cw, ineq) = self.society_metrics(h);
            let so = &mut self.hubs[h].society;
            so.commoner_wealth += (cw - so.commoner_wealth) * 0.3;
            so.inequality += (ineq - so.inequality) * 0.3;
        }
        // DLC 4 · derive typed Pops from the freshly-updated shares (read-only foundation).
        for h in 0..self.hubs.len() { self.derive_pops(h); }
    }


    /// DLC 4 · derive typed `Pop` units for hub `h` from its `society` shares ×
    /// population. Refreshed yearly so the Population panel can read them, and
    /// (FIX_PLAN B3) now a genuine INPUT to `update_unrest` rather than a pure
    /// rendering of it: `militancy` is derived from THIS year's own need deficits
    /// (not last year's `so.unrest`), so the profession-mix bias below carries
    /// information `update_unrest`'s aggregate stats don't — an underclass-heavy
    /// city reads more militant than a burgher-heavy one at the same inequality.
    pub(crate) fn derive_pops(&mut self, h: usize) {
        let hub = &self.hubs[h];
        if hub.is_estate || hub.population < 1.0 {
            self.hubs[h].pops.clear();
            return;
        }
        let pop = hub.population.max(0.0);
        let so = &hub.society;
        let split: [(u8, f32); 9] = [
            (0, so.commoner * 0.60),                        // farmers
            (1, so.commoner * 0.40 + so.underclass * 0.55), // labourers (+ urban poor)
            (2, so.burgher * 0.40),                         // craftsmen
            (3, so.burgher * 0.28),                         // clerks
            (4, so.burgher * 0.32),                         // merchants
            (5, so.patrician * 0.30),                       // clergy
            (6, so.patrician * 0.28),                       // capitalists
            (7, so.patrician * 0.42),                       // aristocrats
            (8, so.underclass * 0.45),                      // soldiers
        ];
        let cw = so.commoner_wealth.max(0.0);
        let life = (1.0 - hub.lack_basic).clamp(0.0, 1.0);
        let every = (1.0 - hub.lack_comfort).clamp(0.0, 1.0);
        let lux = (1.0 - hub.lack_luxury).clamp(0.0, 1.0);
        // Hardship this pop actually lives (not so.unrest — that would make
        // militancy a pure rescaling of last year's unrest, telling
        // update_unrest nothing it didn't already know).
        let hardship = (hub.lack_basic.clamp(0.0, 1.0) * 0.5
            + hub.starving.clamp(0.0, 1.0) * 0.35
            + (1.0 - every) * 0.15).clamp(0.0, 1.0);
        let base_mil = (hardship * 10.0).clamp(0.0, 10.0);
        let base_con = (so.inequality * 6.0).clamp(0.0, 10.0);
        let mut pops = Vec::with_capacity(9);
        for (prof, frac) in split {
            let size = frac * pop;
            if size < 1.0 { continue; }
            let elite = matches!(prof, 6 | 7);
            let mid = matches!(prof, 2 | 3 | 4 | 5);
            let money = if elite { cw * 6.0 + 5.0 } else if mid { cw * 1.6 + 1.0 } else { cw };
            let mil = (base_mil
                + if elite { -2.0 } else if prof == 1 || prof == 8 { 1.5 } else { 0.0 }).clamp(0.0, 10.0);
            let con = (base_con + if mid || elite { 1.5 } else { 0.0 }).clamp(0.0, 10.0);
            pops.push(Pop {
                profession: prof, size, money,
                needs_life: life, needs_everyday: every, needs_luxury: lux,
                consciousness: con, militancy: mil,
            });
        }
        self.hubs[h].pops = pops;
    }


    /// It. 3 · Civil unrest. Each year every settled hub's `unrest` eases toward a
    /// target set by HOW THE PEOPLE LIVE — low mood, steep inequality, dearth of
    /// basics, famine and war push it up; prosperity and commoner welfare pull it
    /// down. Crossing thresholds erupts: a riot (a production + stability shock) or,
    /// at the extreme, a REVOLT that topples the ruling council. This is where the
    /// social substrate (It. 2) finally bites back on the economy and politics.
    pub(crate) fn update_unrest(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let (mood, lackb, starv, prosp, atwar) = {
                let hub = &self.hubs[h];
                (hub.mood, hub.lack_basic.clamp(0.0, 1.0), hub.starving.clamp(0.0, 1.0),
                 hub.sent_prosperity, hub.war_with >= 0)
            };
            let (welfare, ineq) = {
                let so = &self.hubs[h].society;
                ((so.commoner_wealth / (so.commoner_wealth + 1.5)).clamp(0.0, 1.0), so.inequality)
            };
            // Cultures 2.0 · a large minority stirs unrest ONLY in a city that is already
            // struggling (dearth/famine/low mood) — a content, prosperous melting pot stays
            // calm. Bounded and small, so it can seed a revolt over years without runaway.
            let largest_min = self.hub_minorities.get(h)
                .map(|m| m.iter().fold(0.0f32, |a, (_, s)| a.max(*s))).unwrap_or(0.0);
            let stress = (lackb + starv + (1.0 - mood) * 0.5).clamp(0.0, 1.0);
            let minority_unrest = MINORITY_UNREST * largest_min.clamp(0.0, 1.0) * stress;
            // Cultures 2.0 · unmet cultural cravings stoke unrest (pop-weighted): a city
            // that never supplies its peoples the goods they prize grows restive.
            let cult_discontent = self.cultural_discontent(h);
            // DLC 4 · FIX_PLAN B3 — population-weighted militancy/consciousness (the
            // typed Pop layer, derived this same tick by `update_society` above via
            // `derive_pops`). Militancy adds the profession-MIX signal the aggregate
            // stats above don't carry; consciousness (below) scales how fast chronic
            // misery organizes into revolt.
            let (pop_mil, pop_con) = {
                let pops = &self.hubs[h].pops;
                let total: f32 = pops.iter().map(|p| p.size).sum();
                if total > 0.0 {
                    let mil = pops.iter().map(|p| p.militancy * p.size).sum::<f32>() / total / 10.0;
                    let con = pops.iter().map(|p| p.consciousness * p.size).sum::<f32>() / total / 10.0;
                    (mil.clamp(0.0, 1.0), con.clamp(0.0, 1.0))
                } else { (0.0, 0.0) }
            };
            let target = (0.42 * (1.0 - mood)
                + 0.30 * ineq
                + 0.32 * lackb
                + 0.22 * starv
                + if atwar { 0.12 } else { 0.0 }
                + minority_unrest
                + CULTURE_UNREST * cult_discontent
                + POP_MILITANCY_WEIGHT * pop_mil
                - 0.30 * welfare
                - 0.18 * prosp).clamp(0.0, 1.0);
            let u = {
                let so = &mut self.hubs[h].society;
                so.unrest += (target - so.unrest) * UNREST_EASE;
                so.unrest = so.unrest.clamp(0.0, 1.0);
                so.unrest
            };
            // Grievance memory: a year at (or above) the riot line adds to the city's
            // accumulated resentment; a calm year bleeds it off. Chronic, repeated
            // rioting therefore ratchets toward a revolt even without one acute spike.
            // Consciousness scales the ACCRUAL rate (organized discontent boils over
            // faster), not the cooling rate — a complacent populace still calms down
            // at the same rate once conditions improve.
            let grievance = {
                let con_scale = CONSCIOUSNESS_GRIEVANCE_MIN
                    + (CONSCIOUSNESS_GRIEVANCE_MAX - CONSCIOUSNESS_GRIEVANCE_MIN) * pop_con;
                let so = &mut self.hubs[h].society;
                if u >= RIOT_UNREST {
                    so.grievance += (u - RIOT_UNREST) / (1.0 - RIOT_UNREST) * con_scale;
                } else {
                    so.grievance *= GRIEVANCE_COOL;
                }
                so.grievance
            };
            // A revolt fires on an acute spike (REVOLT_UNREST) OR when slow-burn
            // grievance boils over. It topples & bars a seated council; where the seat
            // already stands vacant (e.g. the ruling house went bankrupt in the hard
            // times) it is a leaderless uprising — chronic misery erupts regardless.
            if u >= REVOLT_UNREST || grievance >= GRIEVANCE_REVOLT {
                self.trigger_revolt(h);
            } else if u >= RIOT_UNREST {
                let rioting = self.active_events.iter().any(|e| e.hub == h as i32 && e.kind == "riot");
                if !rioting {
                    self.active_events.push(ActiveEvent {
                        kind: "riot".into(), hub: h as i32, good: -1,
                        magnitude: RIOT_PROD_HIT, until_tick: tick + 150,
                    });
                    let name = self.hubs[h].name.clone();
                    self.journal.push(JournalEntry {
                        tick, kind: "unrest".into(), hub: h as i32, good: -1, value: u,
                        text: format!("Bread riots break out in {}", name),
                    });
                }
            }
        }
    }


    /// A revolt: the populace seizes a slice of every resident house's wealth (→ the
    /// city's `civic_pool`, i.e. the people), drives the ruling family from the
    /// council and BARS it for a generation (`ousted_until`), throws the city into a
    /// season of disorder (a production + stability shock), and vents the pressure.
    /// The seat falls vacant; next New Year `decide_polis_policy` re-seats whoever is
    /// eligible (the banned house excluded), so a rival faction tends to rise.
    pub(crate) fn trigger_revolt(&mut self, h: usize) {
        let tick = self.tick;
        let ousted = self.hubs[h].council_house;
        // Redistribute: every resident house loses a slice of its fortune to the people.
        let mut seized = 0.0f32;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].hub as usize != h { continue; }
            let take = self.houses[hi].wealth.max(0.0) * REVOLT_REDISTRIB;
            self.houses[hi].wealth -= take;
            seized += take;
        }
        self.hubs[h].civic_pool += seized;
        let hubname = self.hubs[h].name.clone();
        let oname = if ousted >= 0 {
            let oi = ousted as usize;
            self.houses[oi].prestige = (self.houses[oi].prestige - 0.25).max(0.0);
            let nm = self.houses[oi].name.clone();
            self.houses[oi].events.push(HouseEvent {
                tick, kind: "revolt".into(),
                text: format!("Driven from the council of {} by a popular revolt", hubname),
            });
            nm
        } else { String::new() };
        // Bar the ousted family from the seat; the council falls vacant for now.
        self.hubs[h].society.ousted_house = ousted;
        self.hubs[h].society.ousted_until = tick + REVOLT_BAN_YEARS * TICKS_PER_YEAR;
        self.hubs[h].council_house = -1;
        // A season of disorder: clear any riot, then crater production + stability.
        self.active_events.retain(|e| !(e.hub == h as i32 && e.kind == "riot"));
        self.active_events.push(ActiveEvent {
            kind: "revolt".into(), hub: h as i32, good: -1,
            magnitude: REVOLT_PROD_HIT, until_tick: tick + 120,
        });
        // Catharsis: the explosion vents the accumulated pressure and resentment.
        self.hubs[h].society.unrest = 0.25;
        self.hubs[h].society.grievance = 0.0;
        self.journal.push(JournalEntry {
            tick, kind: "revolt".into(), hub: h as i32, good: -1, value: 1.0,
            text: if oname.is_empty() { format!("A popular revolt convulses {}", hubname) }
                  else { format!("A popular revolt in {} topples {}", hubname, oname) },
        });
    }


    /// A gentle demand tilt from a city's class composition: a patrician-heavy city
    /// soaks up luxuries (tier 2), a commoner mass eats staples (tier 0). Normalized
    /// around an "average" society so total demand magnitude is preserved on average.
    pub(crate) fn society_demand_mult(&self, h: usize, need_tier: u8) -> f32 {
        let s = &self.hubs[h].society;
        let tot = s.patrician + s.burgher + s.commoner + s.underclass;
        if tot < 1e-3 { return 1.0; } // unseeded (e.g. estates)
        let elite = (s.patrician + s.burgher) / tot;
        let mass = (s.commoner + s.underclass) / tot;
        const ELITE_BASE: f32 = 0.18;
        const MASS_BASE: f32 = 0.82;
        match need_tier {
            2 => (1.0 + STRATA_DEMAND_TILT * (elite - ELITE_BASE) / ELITE_BASE).clamp(0.4, 1.8),
            0 => (1.0 + STRATA_DEMAND_TILT * (mass - MASS_BASE) / MASS_BASE).clamp(0.6, 1.4),
            _ => 1.0, // comfort tier neutral
        }
    }


    /// Phase G monthly wealth sinks: every house/guild pays UPKEEP (depreciation
    /// that counters BANK_INTEREST) and spends a slice on CONSUMPTION that flows
    /// into its home city's `civic_pool` (reaching the people). Both are a fraction
    /// of wealth, so a fortune bleeds proportionally and wealth PLATEAUS where trade
    /// income balances the sinks instead of compounding without end.
    /// A city-size multiplier on a warehouse's keep — a depot in a great entrepôt
    /// costs far more (rents, wages, guards) than one in a market town.
    pub fn city_size_factor(&self, hub: usize) -> f32 {
        let pop = self.hubs.get(hub).map(|h| h.population).unwrap_or(30_000.0);
        (pop / 30_000.0).clamp(0.3, 4.0)
    }


    /// Per-city trade-tax bracket: the rate scales up with the city's prosperity, so
    /// a wealthy hub taxes trade harder than a struggling one (`CITY_TAX_BRACKET`).
    pub(crate) fn city_tax_factor(&self, hub: usize) -> f32 {
        let prosp = self.hubs.get(hub).map(|h| h.sent_prosperity).unwrap_or(0.5).clamp(0.0, 1.0);
        1.0 + CITY_TAX_BRACKET * prosp
    }


    /// Years of unbroken wealth growth on record — the seller's track record that
    /// gates the futures-contract term it may offer. A civic guild is inherently
    /// stable, so its "record" is simply its age in years.
    pub(crate) fn stable_growth_years(&self, hi: usize) -> u32 {
        let h = &self.houses[hi];
        if h.is_guild {
            return self.tick.saturating_sub(h.founded_tick) / TICKS_PER_YEAR;
        }
        let wh = &h.wealth_history;
        if wh.len() < 2 { return 0; }
        let mut run = 0u32;
        for i in (1..wh.len()).rev() {
            if wh[i] >= wh[i - 1] * 0.98 { run += 1; } else { break; }
        }
        run
    }


    /// Deterministic per-culture MOBILITY (0..1). ~20% of peoples are travel-prone
    /// merchant diasporas (mobility ≥ 0.7); the rest are more sedentary. Modelled on
    /// historical trading minorities (Hanseatic Germans, Jewish/Armenian/Greek
    /// diasporas) whose communities spread through the trade network.
    pub(crate) fn culture_mobility(name: &str) -> f32 {
        if name.is_empty() || name == "—" { return 0.2; }
        let mut h = 0xcbf29ce484222325u64;
        for b in name.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        let r = (h % 1000) as f32 / 1000.0;
        if r > 0.80 { 0.7 + (r - 0.80) / 0.20 * 0.30 } else { 0.1 + r / 0.80 * 0.4 }
    }


    /// SETTLEMENT colony — an overcrowded, prosperous city founds a full new market
    /// hub on a fertile reachable site (heavy, joint-stock financing), seeding it
    /// with emigrants (relieving the parent). It graduates outpost→city in
    /// `colony_pass` and may later go autonomous.
    // ── Atlas 2.0 · the LIVING MAP: organic settlement death & birth ────────────

    /// Yearly. DEATH: a city that has spent ABANDON_YEARS shrunk to the famine
    /// floor while still starving/miserable is abandoned — survivors scatter to
    /// the nearest living towns and a † ruin remains. BIRTH: a thriving city
    /// bursting past its founding size swarms — a slice of its people walk out
    /// and found an independent town on nearby free land.
    pub(crate) fn lifecycle_pass(&mut self, expansion_ok: bool) {
        // ── DEATH ──
        let mut deaths = 0u32;
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || (hub.colony_kind == 1 && !hub.autonomous) {
                continue; // dependent colonies die by their own lifeline rules
            }
            if hub.population < 1.0 { continue; }
            let terminal = hub.population < hub.founding_pop * ABANDON_POP_FRAC
                && hub.starving > 0.5;
            if terminal {
                self.hubs[h].decline_years += 1.0;
            } else {
                self.hubs[h].decline_years = (self.hubs[h].decline_years - 0.5).max(0.0);
            }
            if self.hubs[h].decline_years < ABANDON_YEARS || deaths >= 2 { continue; }
            // Systemic anchors don't wink out mid-story: a city at war or hosting a
            // live bank holds on.
            if self.hubs[h].war_with >= 0 { continue; }
            if self.banks.iter().any(|b| !b.defunct && b.seat as usize == h) { continue; }
            // The PULL factor: abandonment is MIGRATION, not evaporation — it needs
            // somewhere better to go: a living, fed town of the same component.
            // (This also keeps a uniformly-starving world from grinding itself
            // empty: if everywhere is hungry, people endure where they are.)
            let comp = self.hubs[h].component;
            let has_haven = self.hubs.iter().enumerate().any(|(j, o)| j != h
                && !o.is_estate && !o.abandoned && o.component == comp
                && o.starving < 0.25 && o.population > o.founding_pop * 0.5);
            if !has_haven { continue; }
            self.abandon_hub(h);
            deaths += 1;
        }
        // ── BIRTH ──
        if expansion_ok && self.tick >= SWARM_START_TICK {
            self.maybe_swarm_town();
        }
    }


    /// Classify WHY a settlement died from its final decade: a plague strike, a
    /// war, a recorded supply-shock disaster — famine otherwise (the terminal
    /// condition is always famine-shaped; this names the deeper wound).
    pub(crate) fn abandon_cause(&self, h: usize) -> &'static str {
        let since = self.tick.saturating_sub(10 * 365);
        if self.epidemics.iter().any(|p| p.hub == h as u32 && p.start_tick >= since) {
            return "plague";
        }
        let (mut war, mut shock) = (false, false);
        for e in self.journal.iter().rev() {
            if e.tick < since { break; }
            if e.hub != h as i32 { continue; }
            match e.kind.as_str() {
                "war" => war = true,
                "event" => shock = true, // drought / blight / fishery collapse / embargo
                _ => {}
            }
        }
        if war { "war" } else if shock { "disaster" } else { "famine" }
    }


    /// Abandon a settlement: survivors migrate to the nearest living towns of the
    /// same component (recorded as refugee roads for the map's migration arrows),
    /// the ruin (population 0, `abandoned`, `died_cause`) stays on the map.
    pub(crate) fn abandon_hub(&mut self, h: usize) {
        let tick = self.tick;
        let nm = self.hubs[h].name.clone();
        let comp = self.hubs[h].component;
        let (hx, hy) = (self.hubs[h].x, self.hubs[h].y);
        let cause = self.abandon_cause(h);
        let mut dests: Vec<(usize, f32)> = (0..self.hubs.len())
            .filter(|&j| j != h && !self.hubs[j].is_estate && !self.hubs[j].abandoned
                && self.hubs[j].component == comp && self.hubs[j].population >= 1.0)
            .map(|j| {
                let mut dx = (self.hubs[j].x - hx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                (j, (dx * dx + (self.hubs[j].y - hy).powi(2)).sqrt())
            })
            .collect();
        dests.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        dests.truncate(3);
        let refugees = self.hubs[h].population * 0.6;
        let dest_name = dests.first().map(|&(j, _)| self.hubs[j].name.clone());
        if !dests.is_empty() {
            let share = refugees / dests.len() as f32;
            for &(j, _) in &dests {
                self.hubs[j].population += share;
                self.migrations.push([hx, hy, self.hubs[j].x, self.hubs[j].y, tick as f32]);
            }
            if self.migrations.len() > MIGRATION_ARROW_CAP {
                let excess = self.migrations.len() - MIGRATION_ARROW_CAP;
                self.migrations.drain(0..excess);
            }
        }
        let hub = &mut self.hubs[h];
        hub.abandoned = true;
        hub.died_tick = tick;
        hub.died_cause = cause.into();
        hub.population = 0.0;
        hub.starving = 0.0;
        hub.decline_years = 0.0;
        for v in hub.stock.iter_mut() { *v = 0.0; }
        for v in hub.production.iter_mut() { *v = 0.0; }
        self.total_abandonments += 1;
        self.routes_dirty = true;
        let why = match cause {
            "plague" => "the plague",
            "war" => "the war",
            "disaster" => "disaster upon disaster",
            _ => "famine",
        };
        self.journal.push(JournalEntry { tick, kind: "abandonment".into(), hub: h as i32,
            good: -1, value: 0.0, text: match dest_name {
                Some(d) => format!("{} is abandoned after years of {} — its last families take the road to {}", nm, why, d),
                None => format!("{} is abandoned after years of {} — the land lies empty", nm, why),
            } });
    }


    /// Create the swarm's daughter town: an ordinary FREE settlement (colony_kind 0)
    /// with a culture-styled name, farming what the site gives it.
    pub(crate) fn create_organic_town(&mut self, mother: usize, site: &ColonizeSite, seed_pop: f32) -> usize {
        let ng = self.goods.len();
        // Settlers carry their skills but raw land yields less at first; fertile
        // ground closes the gap.
        let base_per_capita: Vec<f32> = self.hubs[mother].base_per_capita.iter()
            .map(|v| v * (0.40 + 0.40 * site.fertility)).collect();
        let pop = seed_pop.max(1.0);
        let production: Vec<f32> = base_per_capita.iter().map(|v| v * pop).collect();
        let id = 100_000 + self.hubs.len() as u32;
        let name = crate::sim::names::gen_name(
            site.x.max(0.0) as u32, site.y.max(0.0) as u32,
            self.world_w as u32, self.world_h());
        let component = self.hubs[mother].component;
        self.hubs.push(TickHub {
            id, x: site.x, y: site.y, name, population: pop, founding_pop: pop,
            stock: {
                let mut s = vec![0.0f32; ng * GRADE_BANDS];
                for (g, &p) in production.iter().enumerate() { stock_set_total(&mut s, g, p); }
                s
            },
            price: self.goods.iter().map(|g| g.base_value).collect(),
            production, grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: site.koppen, coastal: site.coastal, component,
            export_earn: 0.0, import_spend: 0.0, mood: 0.62, sent_food: 0.7, sent_prosperity: 0.5,
            sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(), in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, last_upgrade_tick: self.tick, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, coin_history: Vec::new(), debt_principal: 0.0, debt_coupon: 0.0, debt_holders: Vec::new(), mint_bullion_ratio: 1.0, has_mint: false,
            quality: vec![0.0f32; ng], stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: self.tick, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
            tier: 0, standing: 0.0, war_cooldown_until: 0, captor_since: 0, realm: -1, realm_role: 0,
            wh_capacity: 0.0, wh_spoiled_month: Vec::new(), wh_last_month: Vec::new(), supply_accum: Vec::new(), shares: Vec::new(), monthly: Vec::new(),
        });
        self.routes_dirty = true;
        self.hubs.len() - 1
    }


    /// Yearly. ABSORPTION: a tiny, failing FREE town beside a big healthy city is taken
    /// under that city's wing as a SATELLITE instead of being left to die with its trade
    /// halted. The metropolis relocates settlers to shore it up, ships it a founding
    /// grant of food, and binds its trade — so it is fed by the satellite lifeline and
    /// can grow back. Its newcomers seed a quarter of the metropolis's people (cultural
    /// mixing). One absorption per call. (User: "large nearby cities can integrate those
    /// dying cities and become satellites of the bigger ones.")
    pub(crate) fn maybe_absorb_dying_city(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        let n = self.hubs.len();
        let reach = (SATELLITE_MAX_KM * self.world_w / EARTH_EQUATOR_KM).max(2.0);
        // Current satellite dependents per metropolis, precomputed once (keeps this an
        // O(n²) scan, not O(n³) if the map has many tiny towns and no eligible rescuer).
        let mut sat_deps = vec![0u32; n];
        for d in &self.hubs {
            if !d.abandoned && d.colony_kind == 3 && d.founder_hub >= 0 && (d.founder_hub as usize) < n {
                sat_deps[d.founder_hub as usize] += 1;
            }
        }
        for h in 0..self.hubs.len() {
            let c = &self.hubs[h];
            // A genuinely FREE, tiny, struggling town — not an estate, colony, satellite,
            // ruin, or a brand-new swarm town still finding its feet.
            if c.is_estate || c.abandoned || c.colony_kind != 0 { continue; }
            if c.population < 1.0 || c.population > ABSORB_POP_MAX { continue; }
            if tick.saturating_sub(c.founded_tick) < ABSORB_MIN_AGE_YEARS * TICKS_PER_YEAR { continue; }
            // It must actually be in trouble (poor/hungry/shrinking) — a happy hamlet is
            // left to grow on its own.
            let struggling = c.sent_prosperity < 0.45 || c.starving > 0.08 || c.decline_years > 2.0;
            if !struggling { continue; }
            let (cx, cy, comp) = (c.x, c.y, c.component);
            // Nearest big, healthy, free city within a day's reach in the same market.
            let mut best = (usize::MAX, f32::MAX);
            for m in 0..self.hubs.len() {
                if m == h { continue; }
                let mm = &self.hubs[m];
                if mm.is_estate || mm.abandoned || mm.colony_kind != 0 { continue; }
                if mm.component != comp { continue; }
                if mm.population < ABSORB_METRO_POP || mm.starving > 0.3 { continue; }
                // Don't let one metropolis hoard an unbounded number of dependents.
                if sat_deps[m] >= SATELLITE_MAX_PER_METRO { continue; }
                let mut dx = (mm.x - cx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = mm.y - cy;
                let d2 = dx * dx + dy * dy;
                if d2 > reach * reach { continue; }
                if d2 < best.1 { best = (m, d2); }
            }
            let Some(m) = (best.0 != usize::MAX).then_some(best.0) else { continue; };
            // Adopt it. Relocate a wave of settlers from the metropolis…
            let aid = (self.hubs[m].population * ABSORB_AID_FRAC).clamp(150.0, 1500.0);
            self.hubs[m].population = (self.hubs[m].population - aid)
                .max(self.hubs[m].founding_pop * 0.5);
            self.hubs[h].population += aid;
            // …and ship a founding grant of food so it starts fed, not starving.
            for g in 0..ng {
                if !self.goods[g].food { continue; }
                let grant = (stock_of(&self.hubs[m].stock, g) * 0.10).max(0.0);
                stock_take(&mut self.hubs[m].stock, g, grant);
                stock_add_ungraded(&mut self.hubs[h].stock, g, grant);
            }
            self.hubs[h].colony_kind = 3;              // a metropolis-BOUND satellite now
            self.hubs[h].founder_hub = m as i32;
            self.hubs[h].colony_founded_tick = tick;
            self.hubs[h].colony_stage = 0;
            self.hubs[h].build_stage = 0;
            self.hubs[h].decline_years = 0.0;
            self.hubs[h].reserve_food = 60.0;
            self.hubs[h].reserve_cap = 365.0;
            // Judge it as a fresh small satellite (so the abandon floor doesn't drag its
            // old, larger founding size behind it).
            self.hubs[h].founding_pop = self.hubs[h].population.max(1.0);
            // The metropolis's people settle in as a minority quarter (culture mixing).
            let mc = self.hub_culture.get(m).cloned().unwrap_or_default();
            self.record_migration_culture(h, m, aid);
            self.emit_migration_route(m, h, &mc, aid);
            self.routes_dirty = true;
            let (mn, cn) = (self.hubs[m].name.clone(), self.hubs[h].name.clone());
            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32, good: -1,
                value: aid, text: format!("{} takes the failing town of {} under its wing as a satellite", mn, cn) });
            return; // one absorption per call
        }
    }


    /// Refound a ruin as a living small town (restores its living settlement icon).
    pub(crate) fn revive_hub(&mut self, h: usize) {
        let tick = self.tick;
        let pop = RESETTLE_POP;
        {
            let hub = &mut self.hubs[h];
            hub.abandoned = false;
            hub.population = pop;
            hub.founding_pop = pop;
            hub.died_tick = 0;
            hub.died_cause = String::new();
            hub.founded_tick = tick;
            hub.decline_years = 0.0;
            hub.starving = 0.0;
            hub.food_balance = 1.0;
            hub.reserve_food = 30.0;
            hub.mood = 0.6;
            hub.sent_food = 0.7;
            hub.sent_prosperity = 0.4;
            hub.sent_stability = 0.6;
        }
        self.routes_dirty = true;
        self.total_foundings += 1;
        let nm = self.hubs[h].name.clone();
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: h as i32, good: -1, value: pop,
            text: format!("Pioneers resettle the ruins of {} — the town lives again", nm),
        });
    }


    /// Re-rank hubs into commercial classes from the trade that has ACTUALLY flowed
    /// this period (0 ordinary · 1 trade hub · 2 entrepôt). Called twice a year. An
    /// entrepôt is a great SEA pass-through market (Venice/Bruges); a trade hub is a
    /// busy secondary market. 3-check hysteresis (`class_momentum`) so status is earned
    /// and lost over time, not flickered. Uses year-to-date flows, so ranking is
    /// relative and cadence-independent.
    pub(crate) fn classify_hubs(&mut self) {
        let nn = self.hubs.len();
        if nn == 0 { return; }
        let mut throughput = vec![0.0f32; nn];
        // DETERMINISM: `flow_accum` is a HashMap, and this is a FLOAT accumulation.
        // Float addition is not associative, and Rust's RandomState gives every
        // HashMap instance its own iteration order — so summing in map order made
        // `throughput` (and therefore `score`, the sort, and every hub class that
        // follows from it) differ between two runs of the same seed. Sort by key
        // first: same numbers, same order, every time.
        let mut flows: Vec<((u32, u32), f32)> =
            self.flow_accum.iter().map(|(&k, &v)| (k, v)).collect();
        flows.sort_by_key(|&(k, _)| k);
        for ((a, b), v) in flows {
            if (a as usize) < nn { throughput[a as usize] += v; }
            if (b as usize) < nn { throughput[b as usize] += v; }
        }
        // Commercial standing = mostly trade throughput, but BLENDED with population so
        // the ranks genuinely move as cities rise and fall (user: entrepôts/trade hubs
        // must be dynamic, not frozen). A booming city climbs the ranks; a shrinking one
        // slips even if its old trade lingers. Both terms normalised to 0..1.
        let max_thr = throughput.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
        let max_pop = self.hubs.iter().map(|h| h.population).fold(1.0f32, f32::max);
        let score: Vec<f32> = (0..nn).map(|i| {
            0.65 * (throughput[i] / max_thr) + 0.35 * (self.hubs[i].population / max_pop)
        }).collect();
        let mut order: Vec<usize> = (0..nn)
            .filter(|&i| !self.hubs[i].is_estate && !self.hubs[i].abandoned
                && self.hubs[i].population >= 1.0)
            .collect();
        order.sort_by(|&a, &b| score[b].partial_cmp(&score[a]).unwrap_or(std::cmp::Ordering::Equal));
        let live = order.len().max(1);
        let hub_cut = ((live as f32) * 0.20).ceil() as usize;         // top 20% → trade hub
        let emp_cut = ((live as f32) * 0.05).ceil().max(1.0) as usize; // top 5% → entrepôt
        let mut rank = vec![usize::MAX; nn];
        for (r, &i) in order.iter().enumerate() { rank[i] = r; }
        for i in 0..nn {
            self.hubs[i].transit_year = throughput[i];
            if self.hubs[i].is_estate || self.hubs[i].abandoned || self.hubs[i].population < 1.0 {
                self.hubs[i].hub_class = 0;
                self.hubs[i].class_momentum = 0;
                continue;
            }
            let r = rank[i];
            let sc = score[i];
            // Desired class this period, with an absolute floor so a barely-active world
            // doesn't crown entrepôts. Entrepôts must be SEA ports.
            let desired: u8 = if r < emp_cut && self.hubs[i].coastal && sc >= 0.30 {
                2
            } else if r < hub_cut && sc >= 0.10 {
                1
            } else {
                0
            };
            let cur = self.hubs[i].hub_class;
            // Hysteresis: 2 confirming half-years to change a tier (faster than before so
            // the map keeps up with a shifting economy, still no per-check flicker).
            if desired > cur {
                let m = self.hubs[i].class_momentum.max(0) + 1;
                if m >= 2 { self.hubs[i].hub_class = cur + 1; self.hubs[i].class_momentum = 0; }
                else { self.hubs[i].class_momentum = m; }
            } else if desired < cur {
                let m = self.hubs[i].class_momentum.min(0) - 1;
                if m <= -2 { self.hubs[i].hub_class = cur - 1; self.hubs[i].class_momentum = 0; }
                else { self.hubs[i].class_momentum = m; }
            } else {
                let s = self.hubs[i].class_momentum.signum();
                self.hubs[i].class_momentum -= s;
            }
        }
    }


    /// Persist each hub's development tier with HYSTERESIS: a hub must confirm a higher
    /// (or lower) tier over 2 half-year checks before it actually moves, so status is
    /// earned/lost over ~a year and doesn't flicker. Stored in `dev_tier`; the panel reads
    /// the persisted value. A tier CAN fall (decline, war, plague strip institutions).
    pub(crate) fn classify_development(&mut self) {
        let nn = self.hubs.len();
        if self.dev_tier.len() != nn { self.dev_tier.resize(nn, 0); }
        if self.dev_momentum.len() != nn { self.dev_momentum.resize(nn, 0); }
        for i in 0..nn {
            let desired = self.development_tier(i);
            let cur = self.dev_tier[i];
            if desired > cur {
                let m = self.dev_momentum[i].max(0) + 1;
                if m >= 2 { self.dev_tier[i] = cur + 1; self.dev_momentum[i] = 0; }
                else { self.dev_momentum[i] = m; }
            } else if desired < cur {
                let m = self.dev_momentum[i].min(0) - 1;
                if m <= -2 { self.dev_tier[i] = cur - 1; self.dev_momentum[i] = 0; }
                else { self.dev_momentum[i] = m; }
            } else {
                let s = self.dev_momentum[i].signum();
                self.dev_momentum[i] -= s;
            }
        }
    }


    /// COLD START: wipe the entire economic superstructure so the world must build itself
    /// up from nothing when unpaused — no merchant houses, guilds, banks, coinage,
    /// warehouses, contracts, wars or trade flows, every hub reset to a small seed
    /// population with zero wealth and no institutions. Geography, cultures, goods, routes
    /// and production potential are KEPT, so on unpause cities discover partners, trade,
    /// found houses/guilds/coin and grow organically (the existing emergence order). Only
    /// meaningful on a fresh (tick 0) campaign; the command guards that.
    pub fn apply_cold_start(&mut self) {
        // Economic superstructure → gone.
        self.houses.clear();
        self.seed_house_count = 0;
        self.warehouses.clear();
        self.contracts.clear();
        self.banks.clear();
        self.crashes.clear();
        self.wars.clear();
        self.in_transit.clear();
        self.migration_routes.clear();
        self.colony_supply.clear();
        self.epidemics.clear();
        self.flow_year.clear();
        self.flow_accum.clear();
        self.dev_tier.clear();
        self.dev_momentum.clear();
        let ng = self.goods.len();
        let base_prices: Vec<f32> = self.goods.iter().map(|g| g.base_value).collect();
        for h in self.hubs.iter_mut() {
            if h.is_estate { continue; }
            // A humble seed population — cities GROW from this as trade builds.
            h.population = (h.founding_pop * 0.35).max(200.0);
            // Zero all wealth, coinage and institutions.
            h.trade_wealth = 0.0; h.grain_wealth = 0.0; h.treasury = 0.0; h.civic_pool = 0.0;
            h.export_earn = 0.0; h.import_spend = 0.0; h.in_by_sea = 0.0; h.in_by_land = 0.0;
            h.coin_name = String::new(); h.has_mint = false; h.coin_trust = 0.0;
            h.coin_basket = Vec::new(); h.settle_coin = -1; h.mint_fineness = 1.0;
            h.hub_class = 0; h.class_momentum = 0;
            h.owner_house = -1; h.council_house = -1; h.captor_house = -1;
            h.main_bank = -1; h.stake_bank = -1;
            h.govt_type = 0; h.officials.clear(); h.laws.clear();
            h.war_with = -1; h.tribute_to = -1;
            h.structures.clear();
            h.civic_goods = vec![0.0; ng];
            // Seed one tick of stock at the new size so prices start sane, prices at base.
            let mut totals: Vec<f32> = h.base_per_capita.iter().map(|&pc| (pc * h.population).max(0.0)).collect();
            if totals.len() != ng { totals.resize(ng, 0.0); }
            h.stock = vec![0.0f32; ng * GRADE_BANDS];
            for (g, &v) in totals.iter().enumerate() { stock_set_total(&mut h.stock, g, v); }
            h.production = totals;
            h.price = base_prices.clone();
            h.starving = 0.0; h.food_balance = 1.0;
            h.sent_food = 0.7; h.sent_prosperity = 0.3; h.sent_stability = 0.8; h.mood = 0.55;
            h.trade_last_year = 0.0;
        }
        self.routes_dirty = true;
    }


    /// Settlement DEVELOPMENT tier (0..5): how *organised / advanced* a place is —
    /// driven by INSTITUTIONS (government, trade sophistication, warehousing, civic
    /// works and FINANCE), NOT raw population. A compact but sophisticated city-state
    /// (Venice: own coin, banks, an entrepôt, deep institutions) outranks a larger but
    /// institutionally shallow town. Population is only a *soft floor* so a hamlet can't
    /// read as an emporium. Pure & read-only (safe to call anywhere); the yearly
    /// classifier + ability-gating wrap this. Ladder (user-approved, renamable):
    ///   1 Outpost · 2 Market · 3 Guild Town · 4 Free City · 5 Emporium.
    pub fn development_tier(&self, h: usize) -> u8 {
        let hub = &self.hubs[h];
        if hub.abandoned || hub.is_estate || hub.population < 1.0 { return 0; }
        let pop = hub.population;
        // ── pillar signals ──
        let stable    = hub.sent_stability >= 0.5 && hub.society.unrest < 0.5;
        let has_govt  = hub.govt_type > 0 || !hub.officials.is_empty();
        let has_laws  = !hub.laws.is_empty();
        let civic     = hub.structures.len();
        let health    = hub.public_health;
        let has_trade = hub.trade_last_year > 0.0;
        let trade_hub = hub.hub_class >= 1;      // classify_hubs: busy secondary market
        let entrepot  = hub.hub_class >= 2;      // classify_hubs: apex sea entrepôt
        // Biggest warehouse tier standing at this hub (0..5: Depot..Grand Entrepôt).
        let wh_tier = self.warehouses.iter()
            .filter(|w| w.hub as usize == h)
            .map(|w| w.tier).max().unwrap_or(0);
        // FINANCE sophistication: its own coinage / a mint / a banking stake.
        let own_coin = !hub.coin_name.is_empty();
        let finance  = hub.has_mint || own_coin || hub.stake_bank >= 0 || hub.main_bank >= 0;
        // A guild seated in this settlement.
        let has_guild = self.houses.iter()
            .any(|hh| hh.is_guild && !hh.defunct && hh.hub as usize == h);
        // Count of true booleans — for "at least N of these" supporting-milestone gates.
        let count = |bs: &[bool]| bs.iter().filter(|&&b| b).count();
        // ── tiers, top-down. Each tier = a POPULATION FLOOR (soft) + a small REQUIRED
        //    core + "at least N of M" SUPPORTING milestones, so there are several PATHS
        //    to each tier and no single rare flag can hard-block it (keeps tiers actually
        //    achievable while still meaning something). Advancement, not size, drives it.
        //
        // 5 Emporium — apex mercantile city: real trade eminence + finance, then 3 of the
        //   deep-institution set (its own coin, grand storage, many civic works, laws,
        //   stability, public health, a guild).
        if pop >= 20_000.0 && (entrepot || trade_hub) && finance
            && count(&[own_coin, wh_tier >= 4, civic >= 3, has_laws, stable,
                       health >= 0.3, has_guild]) >= 3 {
            return 5;
        }
        // 4 Free City — a trade hub that has attracted FINANCE, plus 2 supporting institutions.
        if pop >= 7_000.0 && trade_hub && finance
            && count(&[wh_tier >= 3, civic >= 2, has_guild, has_laws, stable]) >= 2 {
            return 4;
        }
        // 3 Guild Town — a governed town with a couple of real institutions.
        if pop >= 2_000.0 && has_govt
            && count(&[has_guild, wh_tier >= 1, civic >= 1, trade_hub]) >= 2 {
            return 3;
        }
        // 2 Market — a functioning local market: some trade, or at least a depot.
        if pop >= 700.0 && (has_trade || wh_tier >= 1 || trade_hub) {
            return 2;
        }
        // 1 Outpost — a bare founding settlement.
        1
    }


    /// Trade-tax multiplier for house `hi` trading AT `city`: a BAILO concession pays
    /// only a token toll; otherwise the city's trade DOMINATOR pays less and rival
    /// houses pay a little more than the base rate (a modest sway, not a stranglehold).
    pub(crate) fn house_city_tax_mult(&self, hi: usize, city: usize) -> f32 {
        if self.houses[hi].bailos.contains(&(city as u32)) { return BAILO_CONCESSION_TOLL; }
        match self.city_dominator.get(city).copied().unwrap_or(-1) {
            d if d == hi as i32 => DOMINATOR_TAX_MULT,
            d if d >= 0 => RIVAL_TAX_MULT,
            _ => 1.0,
        }
    }
}
