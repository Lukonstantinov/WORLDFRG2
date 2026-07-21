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
            self.flow_year = self.flow_accum.iter().map(|(&(a, b), &v)| (a, b, v)).collect();
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
    /// population. Read-only foundation for the Nations & POPs layer — nothing
    /// consumes these yet, so the economy is unchanged; refreshed yearly so the
    /// future Population panel and (later) pop-driven demand can read them.
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
        let base_mil = (so.unrest * 10.0).clamp(0.0, 10.0);
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
            let target = (0.42 * (1.0 - mood)
                + 0.30 * ineq
                + 0.32 * lackb
                + 0.22 * starv
                + if atwar { 0.12 } else { 0.0 }
                + minority_unrest
                + CULTURE_UNREST * cult_discontent
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
            let grievance = {
                let so = &mut self.hubs[h].society;
                if u >= RIOT_UNREST {
                    so.grievance += (u - RIOT_UNREST) / (1.0 - RIOT_UNREST);
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
    pub(crate) fn city_size_factor(&self, hub: usize) -> f32 {
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
            stock: production.clone(), price: self.goods.iter().map(|g| g.base_value).collect(),
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
                let grant = (self.hubs[m].stock[g] * 0.10).max(0.0);
                self.hubs[m].stock[g] -= grant;
                self.hubs[h].stock[g] += grant;
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
        for (&(a, b), &v) in self.flow_accum.iter() {
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
            h.stock = h.base_per_capita.iter().map(|&pc| (pc * h.population).max(0.0)).collect();
            if h.stock.len() != ng { h.stock.resize(ng, 0.0); }
            h.production = h.stock.clone();
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
