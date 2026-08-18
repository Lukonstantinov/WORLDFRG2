//! houses — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

/// One house's fleet CHOICE for the tick — the wealth/fleet-composition state
/// it ends up in after upkeep, decay, and the buy-or-sell decision. Each house
/// is independent (only reads its own fields plus the precomputed `used_sea`/
/// `used_land` occupancy, both fixed before any house is processed), so unlike
/// `decide_coinage` there is no cross-house sequencing to replay — just the
/// per-house sequential steps (upkeep before the buy/sell check reads the
/// post-upkeep wealth), captured here via local shadow variables.
/// `decide_fleets` computes this read-only; `apply_fleets` writes it
/// (FIX_PLAN B2 — a future player-run house would call `apply_fleets` with a
/// hand-picked choice instead of going through `decide_fleets`).
pub(crate) struct FleetChoice {
    wealth: f32,
    fleet_sea: u32,
    fleet_river: u32,
    fleet_caravan: u32,
    fleet_cost_booked: f32,
}

impl CampaignSim {

    /// Deliver every DUE futures contract — runs BEFORE the spot `dispatch`, so the
    /// contracted quantity is reserved from the seller's source depot before the
    /// open market can compete for it (the buyer's forward security). A quarantine
    /// at either end suspends the contract (force majeure, no penalty); a seller that
    /// can't supply DEFAULTS and pays the buyer a term-scaled penalty.
    pub(crate) fn fulfill_contracts(&mut self, needs: &[Vec<f32>]) {
        if self.contracts.is_empty() { return; }
        let n = self.hubs.len();
        let ng = self.goods.len();
        let tick = self.tick;
        // Built ONCE per pass: quarantine end-tick per hub, and a (owner,hub)→depot
        // index — so the per-contract loop avoids re-scanning events and warehouses.
        let mut q_until = vec![0u32; n];
        for e in &self.active_events {
            if e.kind == "plague_lockup" && e.until_tick > tick && e.hub >= 0 && (e.hub as usize) < n {
                let h = e.hub as usize;
                if e.until_tick > q_until[h] { q_until[h] = e.until_tick; }
            }
        }
        let mut whidx: std::collections::HashMap<(i32, u32), usize> =
            std::collections::HashMap::with_capacity(self.warehouses.len());
        for (i, w) in self.warehouses.iter().enumerate() { whidx.insert((w.owner, w.hub), i); }
        // Fleet slots free THIS tick per house (fleet minus cargo already in flight).
        // Contracts run before `dispatch`, so they get first call on the vessels; a
        // house with no free ship/caravan for a due delivery is in logistics breach.
        let nh = self.houses.len();
        let mut cap_sea: Vec<i32> = vec![0; nh];
        let mut cap_land: Vec<i32> = vec![0; nh];
        for (i, h) in self.houses.iter().enumerate() {
            if h.defunct { continue; }
            cap_sea[i] = h.fleet_sea as i32;
            cap_land[i] = (h.fleet_river + h.fleet_caravan) as i32;
        }
        for c in &self.in_transit {
            if c.owner >= 0 { let oi = c.owner as usize;
                if oi < nh { if c.sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; } } }
        }
        let mut remove: Vec<usize> = Vec::new();
        for ci in 0..self.contracts.len() {
            let c = self.contracts[ci].clone();
            if tick >= c.end_tick { remove.push(ci); continue; }
            let (buyer, src, seller, g) =
                (c.buyer_hub as usize, c.source_hub as usize, c.seller_house as usize, c.good);
            if buyer >= n || src >= n || g >= ng
                || seller >= self.houses.len() || self.houses[seller].defunct {
                remove.push(ci); continue;
            }
            // Force majeure: a quarantine at either end suspends deliveries (no penalty).
            if q_until[buyer] > tick || q_until[src] > tick {
                self.contracts[ci].suspended_until = q_until[buyer].max(q_until[src]).max(tick + 1);
                continue;
            }
            if c.suspended_until > tick { continue; }
            // Monthly cadence (and a first delivery no sooner than one period after signing).
            if tick.saturating_sub(c.start_tick) < CONTRACT_DELIVER_DAYS { continue; }
            if c.last_fulfilled != 0 && tick.saturating_sub(c.last_fulfilled) < CONTRACT_DELIVER_DAYS { continue; }
            let days = self.days[src * n + buyer];
            if !days.is_finite() {
                self.contracts[ci].suspended_until = tick + CONTRACT_DELIVER_DAYS; // route gone
                continue;
            }
            let spot = self.live_price(self.hub_stock(buyer, g), needs[buyer][g], self.goods[g].base_value);
            let wi = whidx.get(&(seller as i32, src as u32)).copied();
            // On-demand restock: if the source depot is short for this delivery, the
            // house BUYS the shortfall from the source city's spare stock at the local
            // spot price. `form_contracts` may source from any network node that
            // PRODUCES the good, but a depot only refills monthly — without this a
            // contract whose depot hasn't refilled yet defaults every cycle and voids
            // (the "futures contracts always fail" bug). Only the city's surplus above
            // its own reserve is for sale, so a source that genuinely can't supply still
            // defaults.
            if let Some(i) = wi {
                let have0 = self.warehouses[i].stock.get(g).copied().unwrap_or(0.0);
                if have0 < c.monthly_qty {
                    let reserve = needs[src][g] * TRADE_RESERVE_MULT;
                    let avail = (stock_of(&self.hubs[src].stock, g) - reserve).max(0.0);
                    let want = (c.monthly_qty - have0).min(avail);
                    if want > EPS {
                        let src_price = self.live_price(self.hub_stock(src, g), needs[src][g], self.goods[g].base_value);
                        stock_take(&mut self.hubs[src].stock, g, want);
                        self.warehouses[i].stock[g] += want;
                        self.houses[seller].wealth -= want * src_price;
                        self.hubs[src].civic_pool += want * src_price;
                    }
                }
            }
            let have = wi.map(|i| self.warehouses[i].stock.get(g).copied().unwrap_or(0.0)).unwrap_or(0.0);
            if wi.is_none() || have < c.monthly_qty {
                // SELLER DEFAULT — can't deliver. Compensate the buyer above its spot
                // fallback, scaled by term (longer commitments hurt more to break).
                let ti = Self::term_index(c.term_years);
                // LIMITED LIABILITY: a default forfeits the compensation, but can never
                // drive the house more than a small buffer into debt — beyond that it
                // simply goes bankrupt (handled by `update_solvency`). Without this cap a
                // string of over-committed defaults could crater a house to deep negative
                // wealth (the −12k debt the futures fix exposed).
                let raw_penalty = c.monthly_qty * spot * TERM_PENALTY_MULT[ti];
                let penalty = raw_penalty
                    .min((self.houses[seller].wealth + CONTRACT_LIABILITY_FLOOR).max(0.0));
                self.houses[seller].wealth -= penalty;
                self.hubs[buyer].civic_pool += penalty;
                self.houses[seller].prestige = (self.houses[seller].prestige - 0.02).max(0.0);
                self.contracts[ci].defaults += 1;
                self.contracts[ci].last_fulfilled = tick;
                let (hn, cn, gn) = (self.houses[seller].name.clone(),
                    self.hubs[buyer].name.clone(), self.goods[g].name.clone());
                let txt = format!("{} defaults on its {} supply contract to {} (forfeits {:.0})", hn, gn, cn, penalty);
                self.houses[seller].events.push(HouseEvent { tick, kind: "disaster".into(), text: txt.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: buyer as i32, good: g as i32, value: penalty, text: txt });
                if self.contracts[ci].defaults >= 3 { remove.push(ci); }
                continue;
            }
            // ── Multimodal convoy ────────────────────────────────────────────────
            // The route's legs come from the endpoints: a SEA leg if either end is
            // coastal, a LAND leg if either is inland — so a coast↔inland route is
            // MIXED and must reserve BOTH a ship and a land vessel. A big delivery
            // fans out over MANY vessels (qty ÷ per-vessel capacity); each rolls its
            // own loss, so the cargo can arrive PARTIALLY (e.g. 10 of 12 ships).
            let qty = c.monthly_qty;
            let (src_coastal, buyer_coastal) = (self.hubs[src].coastal, self.hubs[buyer].coastal);
            let need_sea = src_coastal || buyer_coastal;     // ≥1 coastal → a sea leg
            let need_land = !(src_coastal && buyer_coastal);  // ≥1 inland → a land leg
            // Each required leg's monthly carrying capacity (free vessels × per-vessel
            // hold). The journey is limited by its TIGHTEST leg. A land vessel's hold
            // is the house's boat/caravan mix average (river boats carry more than
            // caravans), so a riverine house moves more overland per slot.
            let rv = self.houses[seller].fleet_river as f32;
            let cv = self.houses[seller].fleet_caravan as f32;
            let land_per = if rv + cv > 0.0 {
                (rv * BOAT_CAPACITY + cv * CARAVAN_CAPACITY) / (rv + cv)
            } else { CARAVAN_CAPACITY };
            let sea_cap = if need_sea { cap_sea[seller].max(0) as f32 * SHIP_CAPACITY } else { f32::INFINITY };
            let land_cap = if need_land { cap_land[seller].max(0) as f32 * land_per } else { f32::INFINITY };
            let leg_cap = sea_cap.min(land_cap);
            if leg_cap <= 0.0 {
                // A required leg has NO vessel free → LOGISTICS BREACH (penalty + strike).
                let ti = Self::term_index(c.term_years);
                let penalty = qty * spot * TERM_PENALTY_MULT[ti];
                self.houses[seller].wealth -= penalty;
                self.hubs[buyer].civic_pool += penalty;
                self.houses[seller].prestige = (self.houses[seller].prestige - 0.02).max(0.0);
                self.contracts[ci].defaults += 1;
                self.contracts[ci].last_fulfilled = tick;
                let (hn, cn, gn) = (self.houses[seller].name.clone(),
                    self.hubs[buyer].name.clone(), self.goods[g].name.clone());
                let txt = format!("{} has no vessel free for its {} contract to {} — breach (forfeits {:.0})", hn, gn, cn, penalty);
                self.houses[seller].events.push(HouseEvent { tick, kind: "disaster".into(), text: txt.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: buyer as i32, good: g as i32, value: penalty, text: txt });
                if self.contracts[ci].defaults >= 3 { remove.push(ci); }
                continue;
            }
            let loadable = qty.min(leg_cap); // can't ship more than the fleet can carry
            let ships_used = if need_sea { (loadable / SHIP_CAPACITY).ceil() as i32 } else { 0 };
            let landv_used = if need_land { (loadable / land_per).ceil() as i32 } else { 0 };
            cap_sea[seller] -= ships_used;
            cap_land[seller] -= landv_used;
            // Reserve the loaded goods from the source depot (sunk cargo is lost).
            let wi = wi.unwrap();
            self.warehouses[wi].stock[g] -= loadable;
            // Per-vessel loss. On a mixed route a unit must survive BOTH legs, so the
            // combined risk is 1−(1−p_sea)(1−p_land). The convoy = its binding leg's
            // vessels, each carrying an equal share of the load.
            let sea_p = if need_sea {
                if self.houses[seller].archetype == ARCH_FLEET { SEA_LOSS * FLEET_LOSS_MULT } else { SEA_LOSS }
            } else { 0.0 };
            let land_p = if need_land {
                let cv = self.houses[seller].fleet_caravan as f32;
                let rv = self.houses[seller].fleet_river as f32;
                let tot = (cv + rv).max(1.0);
                let base = CARAVAN_LOSS * (cv / tot) + RIVER_LOSS * (rv / tot);
                if self.houses[seller].archetype == ARCH_FLEET { base * FLEET_LOSS_MULT } else { base }
            } else { 0.0 };
            let route_loss = 1.0 - (1.0 - sea_p) * (1.0 - land_p);
            let vessels = ships_used.max(landv_used).max(1);
            let per = loadable / vessels as f32;
            let mut delivered_qty = 0.0;
            let mut sunk = 0;
            for k in 0..vessels {
                let lost = hash01(self.seed,
                    (tick as u64) ^ 0xC0117 ^ ((src as u64) << 8) ^ (buyer as u64),
                    (g as u64) ^ ((k as u64) << 24)) < route_loss;
                if lost {
                    sunk += 1;
                    if need_sea { self.damage_fleet(seller, true); }
                    if need_land { self.damage_fleet(seller, false); }
                    self.diag_lost += 1;
                } else {
                    delivered_qty += per;
                }
            }
            self.contracts[ci].last_fulfilled = tick;
            let sea = need_sea; // tag the in-transit leg as a sea voyage when one exists
            if sunk > 0 {
                let gn = self.goods[g].name.clone();
                let txt = format!("{} of {} {} convoys carrying {} are lost en route to {}",
                    sunk, vessels, if need_sea { "ship" } else { "caravan" }, gn,
                    self.hubs[buyer].name.clone());
                self.houses[seller].events.push(HouseEvent { tick, kind: "disaster".into(), text: txt.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: buyer as i32, good: g as i32, value: 0.0, text: txt });
            }
            // A significant shortfall (heavy losses OR too few vessels to carry the
            // contracted amount) counts as a missed delivery; minor storm losses don't.
            // A GOOD delivery CLEARS the strike count, so only 3 CONSECUTIVE failures
            // void the contract — otherwise a multi-year contract inevitably accrues 3
            // scattered misses and voids before it can ever reach term (the "no
            // contracts ever finish" bug).
            if delivered_qty < qty * 0.5 {
                self.contracts[ci].defaults += 1;
                if self.contracts[ci].defaults >= 3 { remove.push(ci); }
            } else {
                self.contracts[ci].defaults = 0;
            }
            if delivered_qty <= EPS { continue; } // total loss — nothing ships/sells
            // Paid price drifts toward spot but stays within the band around the strike.
            let pt = (0.7 * c.strike_price + 0.3 * spot).clamp(
                c.strike_price * (1.0 - CONTRACT_PRICE_BAND),
                c.strike_price * (1.0 + CONTRACT_PRICE_BAND));
            let value = delivered_qty * pt;
            let freight = delivered_qty * self.good_freight(g, self.freight_per_day, days);
            self.houses[seller].wealth += value - freight;
            // Toll-free network transit: when the cargo moves between the house's OWN
            // cities (its gates), it pays reduced civic tolls at both ends.
            let toll = if self.is_house_node(seller, src as u32) && self.is_house_node(seller, buyer as u32) {
                NETWORK_TOLL_DISCOUNT
            } else { 1.0 };
            let export_tax = value * EXPORT_TAX_RATE * self.city_tax_factor(src) * toll * self.house_city_tax_mult(seller, src);
            let import_tax = value * IMPORT_TAX_RATE * self.city_tax_factor(buyer) * toll * self.house_city_tax_mult(seller, buyer);
            self.houses[seller].wealth -= export_tax + import_tax;
            self.hubs[src].civic_pool += export_tax;
            self.hubs[buyer].civic_pool += import_tax;
            self.hubs[src].export_earn += value;
            self.hubs[buyer].import_spend += value;
            self.houses[seller].volume += delivered_qty;
            self.in_transit.push(InTransit {
                from: src as u32, to: buyer as u32, good: g, amount: delivered_qty,
                eta_tick: tick + (days.ceil() as u32).max(1),
                owner: seller as i32, sea, phase: 1, home: -1, // one-way: no return leg
                contract: true, // its vessel is held by the standing contract reservation
            });
            self.bump_trade_at(seller, src, delivered_qty);
            self.bump_trade_at(seller, buyer, delivered_qty);
            self.log_trade(src as u32, buyer as u32, g, delivered_qty, seller as i32, sea, pt);
            self.contracts[ci].delivered += delivered_qty;
        }
        for &ci in remove.iter().rev() { self.contracts.remove(ci); }
    }


    /// Monthly: a seated house with an office in a city that is a STRUCTURAL importer
    /// of one of its specialty goods — and which the house can source from its home
    /// depot — offers that city a futures contract, for the longest term its record
    /// allows, covering only the spare slice under the per-good coverage cap (so the
    /// spot market keeps the rest and prices still form).
    /// A city the house operates from — its home or any office.
    pub(crate) fn is_house_node(&self, hi: usize, hub: u32) -> bool {
        self.houses[hi].hub == hub || self.houses[hi].offices.contains(&hub)
    }

    /// True while the house holds a live lease on `hub`.
    pub(crate) fn office_leased(&self, hi: usize, hub: u32) -> bool {
        self.houses[hi].office_leases.iter().any(|&(h, until)| h == hub && until > self.tick)
    }

    /// True while an active contract relies on the house's base at `hub` (as buyer or
    /// source) — such an office must stay open for the life of the contract.
    pub(crate) fn backs_active_contract(&self, hi: usize, hub: u32) -> bool {
        self.contracts.iter().any(|c| c.seller_house as usize == hi
            && (c.buyer_hub == hub || c.source_hub == hub) && self.tick < c.end_tick)
    }

    /// Lease `hub` as a durable office for `years`: ensures it's an office, pays the
    /// city an upfront fee (once), and records/extends the lease end-tick.
    pub(crate) fn lease_office(&mut self, hi: usize, hub: u32, years: u32) {
        let until = self.tick + years * TICKS_PER_YEAR;
        if let Some(e) = self.houses[hi].office_leases.iter_mut().find(|(h, _)| *h == hub) {
            if until > e.1 { e.1 = until; }
        } else {
            self.houses[hi].office_leases.push((hub, until));
            let fee = OFFICE_LEASE_FEE * self.city_size_factor(hub as usize);
            self.houses[hi].wealth -= fee;
            if (hub as usize) < self.hubs.len() { self.hubs[hub as usize].civic_pool += fee; }
        }
        if !self.houses[hi].offices.contains(&hub) { self.houses[hi].offices.push(hub); }
    }


    pub(crate) fn form_contracts(&mut self, needs: &[Vec<f32>]) {
        if self.contracts.len() >= MAX_CONTRACTS { return; }
        let n = self.hubs.len();
        // A colony/estate may have been founded earlier THIS tick (hubs grew, but the
        // `days` travel matrix is only rebuilt at the next tick start). Skip forming
        // until the matrix matches the hub count — avoids an out-of-bounds index.
        if self.days.len() != n * n { return; }
        let ng = self.goods.len();
        let tick = self.tick;
        // Goods that are a RAW INPUT to some manufacturing recipe. A house will sign
        // PROCUREMENT futures for these (beyond its own speciality) so a manufactory's
        // raw supply stays steady — the "manufactures urged to hold futures for a
        // stable goods flow" rule. Manufacturing demand is already folded into `needs`
        // (add_manufacturing_demand), so a structural input shortfall shows as
        // production < need at the working city.
        let is_input: Vec<bool> = {
            let mut s = vec![false; ng];
            for g in 0..ng { for &(idx, _) in &self.goods[g].inputs { if idx < ng { s[idx] = true; } } }
            s
        };
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].offices.is_empty() { continue; }
            if hash01(self.seed, tick as u64 ^ 0xC047, hi as u64) > CONTRACT_FORM_CHANCE { continue; }
            let ti = self.max_term_index(hi);
            let term = TERM_YEARS[ti];
            let offices = self.houses[hi].offices.clone();
            let specs = self.houses[hi].spec.clone();
            // The house's NETWORK nodes (home + offices) — any can SOURCE a contract,
            // letting a distant office-city be supplied a good the house makes far away.
            let nodes: Vec<u32> = std::iter::once(self.houses[hi].hub)
                .chain(offices.iter().copied()).collect();
            'outer: for &off in &offices {
                let buyer = off as usize;
                if buyer >= n { continue; }
                // A futures market needs a SOLVENT counterparty: a city gripped by
                // dearth or too poor to be a reliable buyer won't be signed long luxury
                // supply contracts (it can't pay, and its people need bread, not silk).
                // This also keeps contracts from artificially propping up destitute
                // cities — regional disparities and unrest stay real.
                if self.hubs[buyer].starving > 0.25
                    || self.hubs[buyer].sent_prosperity < CONTRACT_BUYER_MIN_PROSPERITY {
                    continue;
                }
                // Candidate goods = the house's specialities (it SELLS these) PLUS any
                // manufacturing INPUT this office-city is structurally short of (it BUYS
                // these to keep its workshops fed). The input branch is what turns a
                // house into a raw-materials PROCUREMENT agent for the manufactory,
                // giving finished-good production a stable input flow.
                let mut cand = specs.clone();
                for g in 0..ng {
                    if !is_input[g] || self.goods[g].food || specs.contains(&g) { continue; }
                    if needs[buyer][g] > EPS
                        && self.hubs[buyer].production.get(g).copied().unwrap_or(0.0) < needs[buyer][g] * 0.8 {
                        cand.push(g);
                    }
                }
                for &g in &cand {
                    if g >= ng || self.goods[g].food { continue; }
                    // Structural deficit: the city produces well under its own need.
                    if self.hubs[buyer].production.get(g).copied().unwrap_or(0.0) >= needs[buyer][g] * 0.8 { continue; }
                    let cap = CONTRACT_COVERAGE_CAP * needs[buyer][g] * 30.0;
                    if cap <= EPS { continue; }
                    let existing: f32 = self.contracts.iter()
                        .filter(|c| c.buyer_hub as usize == buyer && c.good == g)
                        .map(|c| c.monthly_qty).sum();
                    let room = (cap - existing).max(0.0);
                    if room <= EPS { continue; }
                    if self.contracts.iter().any(|c|
                        c.seller_house as usize == hi && c.buyer_hub as usize == buyer && c.good == g) { continue; }
                    // Pick the NEAREST reachable network node that can actually SUPPLY g
                    // — a depot holding it, or a city producing a genuine SURPLUS (output
                    // above its own need). Requiring surplus (not merely any production) is
                    // what fixed the "no futures contracts ever form" bug WITHOUT letting a
                    // house over-commit to a source that consumes its own output: the old
                    // code chose the nearest *producer* even when its surplus was nil, so
                    // `supply_cap` came out 0 and no contract could sign. Now the source is
                    // a real exporter, so the contract is both signable and fulfillable.
                    let src = nodes.iter().copied()
                        .filter(|&nd| nd as usize != buyer && (nd as usize) < n
                            && self.days[nd as usize * n + buyer].is_finite())
                        .filter(|&nd| {
                            let has_depot = self.warehouses.iter().any(|w| w.owner == hi as i32
                                && w.hub == nd && w.stock.get(g).copied().unwrap_or(0.0) > 0.0);
                            let ndx = nd as usize;
                            let surplus = self.hubs.get(ndx).and_then(|h| h.production.get(g)).copied().unwrap_or(0.0)
                                - needs.get(ndx).and_then(|r| r.get(g)).copied().unwrap_or(0.0);
                            has_depot || surplus > EPS
                        })
                        .min_by(|&a, &b| self.days[a as usize * n + buyer]
                            .partial_cmp(&self.days[b as usize * n + buyer]).unwrap_or(std::cmp::Ordering::Equal));
                    let src = match src { Some(s) => s as usize, None => continue };
                    // Size the monthly quantity to what the seller can REALISTICALLY
                    // SUPPLY and CARRY, not just to the buyer's need — otherwise the
                    // depot/fleet can't meet it and the contract defaults every month
                    // until it voids (the "contracts cancel a month later" bug).
                    //   supply = depot stock already at src + the sustainable rate the
                    //            depot restocks from the source city's monthly surplus.
                    let depot_stock: f32 = self.warehouses.iter()
                        .filter(|w| w.owner == hi as i32 && w.hub == src as u32)
                        .map(|w| w.stock.get(g).copied().unwrap_or(0.0)).sum();
                    let src_surplus = (self.hubs[src].production.get(g).copied().unwrap_or(0.0)
                        - needs[src][g]).max(0.0);
                    // Size to the source's true monthly SURPLUS (what it can spare above its
                    // own need) plus any depot stock — the deliverable, sustainable amount.
                    // The src selection above guarantees this is > 0, so contracts form
                    // without over-committing to a source that can't actually supply.
                    let supply_cap = depot_stock + src_surplus * 30.0 * WH_STOCK_FRAC;
                    //   carry = the seller's fleet capacity on this route's binding leg
                    //   (coast↔coast = sea; any inland end needs a land leg).
                    let (sc, bc) = (self.hubs[src].coastal, self.hubs[buyer].coastal);
                    let need_sea = sc || bc;
                    let need_land = !(sc && bc);
                    let rv = self.houses[hi].fleet_river as f32;
                    let cv = self.houses[hi].fleet_caravan as f32;
                    let land_per = if rv + cv > 0.0 {
                        (rv * BOAT_CAPACITY + cv * CARAVAN_CAPACITY) / (rv + cv)
                    } else { CARAVAN_CAPACITY };
                    let sea_carry = if need_sea { self.houses[hi].fleet_sea as f32 * SHIP_CAPACITY } else { f32::INFINITY };
                    let land_carry = if need_land { (rv + cv) * land_per } else { f32::INFINITY };
                    let carry_cap = sea_carry.min(land_carry);
                    // Transport must be SPARE: subtract what this house's existing
                    // contracts already claim, and require a 20% headroom on top of the
                    // new quota. A house without the right vessels for this route
                    // (carry_cap 0) — or already fully committed — simply can't sign.
                    let committed: f32 = self.contracts.iter()
                        .filter(|c| c.seller_house as usize == hi)
                        .map(|c| c.monthly_qty)
                        .sum();
                    let spare_carry = (carry_cap - committed).max(0.0);
                    let monthly_qty = room.min(supply_cap).min(spare_carry / CONTRACT_TRANSPORT_MARGIN);
                    if monthly_qty <= EPS { continue; } // not enough spare transport → don't sign
                    let strike = self.live_price(self.hub_stock(buyer, g), needs[buyer][g],
                        self.goods[g].base_value) * TERM_STRIKE_FACTOR[ti];
                    self.contracts.push(Contract {
                        seller_house: hi as u32, buyer_hub: buyer as u32, source_hub: src as u32,
                        good: g, monthly_qty, strike_price: strike, term_years: term,
                        start_tick: tick, end_tick: tick + term as u32 * TICKS_PER_YEAR,
                        delivered: 0.0, last_fulfilled: 0, suspended_until: 0, defaults: 0,
                        coin: self.hubs[buyer].settle_coin, // struck in the buyer city's main coin
                    });
                    // Lease BOTH ends for the contract's life (≥ its term) so the bases
                    // can't lapse under it — the durable spine of the trade network.
                    let lease_years = (term as u32).max(OFFICE_LEASE_YEARS);
                    self.lease_office(hi, buyer as u32, lease_years);
                    if src != self.houses[hi].hub as usize { self.lease_office(hi, src as u32, lease_years); }
                    let (hn, cn, sn, gn) = (self.houses[hi].name.clone(), self.hubs[buyer].name.clone(),
                        self.hubs[src].name.clone(), self.goods[g].name.clone());
                    self.journal.push(JournalEntry {
                        tick, kind: "charter".into(), hub: buyer as i32, good: g as i32, value: term as f32,
                        text: format!("{} signs a {}-year {} supply contract: {} → {}", hn, term, gn, sn, cn) });
                    break 'outer; // at most one new contract per house per pass
                }
            }
        }
    }


    /// Index helper so the borrow checker is happy reading b's stock in dispatch.
    pub(crate) fn house_for(&self, hub: usize, good: usize) -> i32 {
        // Private merchant houses TAKE OVER a city's trade in their specialty: a
        // seated specialist wins its own good first, then a specialist that holds an
        // OFFICE here (offices project real trading power into the city). Only then
        // does the civic GUILD carry the rest (the city's general/needs trade), then
        // any seated house, then any office-holder. This lets dynamic houses grow
        // dominant instead of the guild monopolising everything at home.
        let off = hub as u32;
        self.houses.iter()
            .position(|h| !h.defunct && !h.is_guild && h.hub as usize == hub && h.spec.contains(&good))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && !h.is_guild && h.offices.contains(&off) && h.spec.contains(&good)))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && h.is_guild && h.hub as usize == hub))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && !h.is_guild && h.hub as usize == hub))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && h.offices.contains(&off)))
            .map(|i| i as i32)
            .unwrap_or(-1)
    }


    /// The strongest resident house at `hub` (richest, non-defunct), if any.
    pub(crate) fn strongest_house_at(&self, hub: usize) -> Option<usize> {
        let mut best = (usize::MAX, 0.0f32);
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct || hh.hub as usize != hub { continue; }
            if hh.wealth >= best.1 { best = (hi, hh.wealth); }
        }
        (best.0 != usize::MAX).then_some(best.0)
    }


    /// Push a new estate hub working good `g0` (kind `kind`) at `(x,y)`, owned by
    /// `owner_house` (−1 = the parent city). `percap` is the estate's dedicated
    /// per-capita output rate. Shared by neighbour-estates and new-land colonies.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_estate(&mut self, parent: i32, x: f32, y: f32, g0: usize, kind: u8,
                     owner_house: i32, koppen: u8, coastal: bool, component: u32,
                     base_pop: f32, percap: f32) {
        let ng = self.goods.len();
        let est_pop = base_pop.max(1.0);
        let mut base_per_capita = vec![0.0f32; ng];
        base_per_capita[g0] = percap.max(0.05);
        let mut production = vec![0.0f32; ng];
        production[g0] = base_per_capita[g0] * est_pop;
        let id = 100_000 + self.hubs.len() as u32;
        // Holdings are LINKED to their parent settlement: co-locate the estate /
        // manufactory at the parent's coordinates so it is not a separate point on
        // the map and all its trade routes through the parent city. The passed
        // `x,y` (a small offset near the parent) is kept only as a fallback when
        // there is no parent. Terroir/quality here is seeded by kind, not the cell,
        // so co-locating costs no fidelity.
        let (x, y) = if parent >= 0 && (parent as usize) < self.hubs.len() {
            (self.hubs[parent as usize].x, self.hubs[parent as usize].y)
        } else { (x, y) };
        let owner_label = if owner_house >= 0 && (owner_house as usize) < self.houses.len() {
            self.houses[owner_house as usize].name.clone()
        } else if parent >= 0 && (parent as usize) < self.hubs.len() {
            self.hubs[parent as usize].name.clone()
        } else { "New".into() };
        let name = format!("{} {}", owner_label, estate_kind_label(kind));
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "estate".into(), hub: parent, good: g0 as i32, value: 0.0,
            text: format!("{} establishes {} ({})", owner_label, name, self.goods[g0].name),
        });
        self.hubs.push(TickHub {
            id, x, y, name, population: est_pop, founding_pop: est_pop,
            stock: vec![0.0; ng * GRADE_BANDS], price: self.goods.iter().map(|g| g.base_value).collect(),
            production, grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: true, parent, koppen, coastal, component,
            export_earn: 0.0, import_spend: 0.0, mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5,
            sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(), in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: kind, estate_tier: 1, last_upgrade_tick: self.tick, owner_house, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, coin_history: Vec::new(), debt_principal: 0.0, debt_coupon: 0.0, debt_holders: Vec::new(), mint_bullion_ratio: 1.0, has_mint: false,
            // DLC 4 · seed the new estate's quality (length ng) so it's graded from
            // day one — a manufactory (kind 6) starts as a humble workshop and learns.
            quality: { let mut q = vec![0.0f32; ng]; if g0 < ng { q[g0] = if kind == 6 { 0.34 } else { 0.46 }; } q },
            stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: self.tick, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
            tier: 0, standing: 0.0, war_cooldown_until: 0, captor_since: 0, realm: -1, realm_role: 0,
            wh_capacity: 0.0, wh_spoiled_month: Vec::new(), wh_last_month: Vec::new(), supply_accum: Vec::new(), shares: Vec::new(), monthly: Vec::new(), brand_chronicled: false, bad_years: 0, disaster_repair_mult: 0.0,
        });
        // Defer the O(n²) route/neighbour rebuild to the next tick (batched).
        self.routes_dirty = true;
    }


    /// Count satellite production sites (estates + colonies). Used to keep the hub
    /// list — and therefore every per-tick loop — bounded over a long campaign.
    pub(crate) fn estate_count(&self) -> usize {
        self.hubs.iter().filter(|h| h.is_estate).count()
    }


    /// Yearly: a MANUFACTORY (estate_kind 6) that hasn't turned a profit for 4 years
    /// is SHUT DOWN — its owner recoups part of the works' capital and the city's
    /// estate slot is freed. "Unprofitable" = it makes nothing (input/labor-starved)
    /// OR its output piles up unsold (a persistent glut). Raw estates are left alone.
    pub(crate) fn manufactory_solvency_pass(&mut self) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        if self.estate_idle_years.len() < n { self.estate_idle_years.resize(n, 0); }
        for h in 0..n {
            if !self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].estate_kind != 6 {
                if h < self.estate_idle_years.len() { self.estate_idle_years[h] = 0; }
                continue;
            }
            // Output good = the good it produces most.
            let g = (0..ng).max_by(|&a, &b| self.hubs[h].production[a]
                .partial_cmp(&self.hubs[h].production[b]).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0);
            let prod = self.hubs[h].production.get(g).copied().unwrap_or(0.0);
            let stock = stock_of(&self.hubs[h].stock, g);
            // Unprofitable this year: makes nothing (starved) OR a year-plus unsold glut.
            let unprofitable = prod <= EPS || stock > prod * 360.0;
            if unprofitable {
                self.estate_idle_years[h] = self.estate_idle_years[h].saturating_add(1);
            } else {
                self.estate_idle_years[h] = 0;
            }
            if self.estate_idle_years[h] >= 4 {
                self.close_manufactory(h);
                self.estate_idle_years[h] = 0;
            }
        }
    }


    /// Shut an unprofitable manufactory: the owner recoups 40% of its capital value,
    /// the works go inert and detach from their city (freeing the estate slot).
    pub(crate) fn close_manufactory(&mut self, h: usize) {
        let tick = self.tick;
        let tier = self.hubs[h].estate_tier.max(1) as f32;
        let recoup = 0.4 * tier * BANK_STAKE_VALUE_PER_TIER;
        let owner = self.hubs[h].owner_house;
        if owner >= 0 && (owner as usize) < self.houses.len() && !self.houses[owner as usize].defunct {
            self.houses[owner as usize].wealth += recoup;
        }
        let owner_name = if owner >= 0 && (owner as usize) < self.houses.len() {
            self.houses[owner as usize].name.clone()
        } else { "The city".to_string() };
        let (ename, parent) = (self.hubs[h].name.clone(), self.hubs[h].parent);
        let ng = self.goods.len();
        {
            let e = &mut self.hubs[h];
            e.abandoned = true; e.estate_tier = 0; e.owner_house = -1; e.parent = -1;
            e.production = vec![0.0; ng];
            for v in e.base_per_capita.iter_mut() { *v = 0.0; }
        }
        self.journal.push(JournalEntry {
            tick, kind: "disaster".into(), hub: parent, good: -1, value: 0.0,
            text: format!("{} shutters its unprofitable works at {}", owner_name, ename) });
    }


    /// Local DEMAND PRESSURE on good `g` at hub `h`: how dear it is relative to its
    /// intrinsic worth (`price` is in the grain-equivalent numeraire, `base_value` the
    /// good's intrinsic value), clamped so it re-weights a founder's choice of good
    /// without dominating it. A value above 1 means the city is UNDER-SUPPLIED and a
    /// new producer would be profitable — this is the signal that turns unmet demand
    /// into new estates and manufactories (a wine-capable city short of wine plants a
    /// vineyard, and so on for every good). Returns a flat 1.0 (no effect) on a
    /// province-less world, so the province-less dynamics gate stays bit-identical.
    pub(crate) fn demand_pressure_at(&self, h: usize, g: usize) -> f32 {
        if self.prov_cap.is_empty() { return 1.0; }
        let base = self.goods[g].base_value.max(1e-3);
        let price = self.hubs[h].price.get(g).copied().unwrap_or(base);
        (price / base).clamp(0.6, 3.0)
    }

    pub(crate) fn maybe_found_estate(&mut self) {
        // Reserve `OUTPOST_RESERVED_ESTATES` slots off the shared budget so ordinary
        // estates (founded far more often than an outpost) cannot starve the outpost
        // path entirely — see that constant's own doc for the diagnosed bug.
        if self.estate_count() >= MAX_TOTAL_ESTATES.saturating_sub(OUTPOST_RESERVED_ESTATES) { return; }
        let n = self.hubs.len();
        let ng = self.goods.len();
        // Founder: a LARGE, commercially successful, non-estate city (rank by
        // population × trade wealth) — big entrepôts plant estates, not tiny hubs.
        let mut best: Option<usize> = None;
        let mut best_score = 0.0f32;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].trade_wealth <= 0.15 { continue; }
            if self.hubs[h].food_balance < -0.1 { continue; } // a hungry city doesn't expand
            // Per-city cap (was MISSING here → the single richest city amassed dozens of
            // same-good estates). Skip cities already at their estate cap so the
            // hinterland spreads across the region instead of one monoculture.
            let on_city = self.hubs.iter().filter(|e| e.is_estate && e.parent == h as i32).count();
            let cap = if self.hubs[h].population >= ESTATE_BIG_CITY_POP { MAX_ESTATES_BIG_CITY } else { MAX_ESTATES_PER_CITY };
            if on_city >= cap { continue; }
            let score = self.hubs[h].population * self.hubs[h].trade_wealth.max(0.0);
            if score > best_score { best_score = score; best = Some(h); }
        }
        let Some(parent) = best else { return };
        // Which good the estate works, and so its kind (farm / mine / plantation /
        // vineyard / fishery). Among goods the LAND here can actually yield
        // (`base_per_capita > 0`), prefer the one under the most local DEMAND PRESSURE
        // — dear relative to its base value, i.e. under-supplied — rather than merely
        // the largest existing output. That is what makes unmet demand for a good like
        // wine URGE a city sitting on wine-capable land to plant a vineyard instead of
        // endlessly reinforcing what it already exports.
        let mut bestg = (usize::MAX, 0.0f32);
        for g in 0..ng {
            let pc = self.hubs[parent].base_per_capita.get(g).copied().unwrap_or(0.0);
            if pc <= 0.0 { continue; }
            let score = pc * self.demand_pressure_at(parent, g);
            if score > bestg.1 { bestg = (g, score); }
        }
        let Some(mut g0) = (bestg.0 != usize::MAX).then_some(bestg.0) else { return };
        let mut kind = estate_kind_for_good(&self.goods[g0].name, self.goods[g0].food);
        // A fishery needs a coast; inland, fall back to the strongest food good (a farm).
        if kind == 4 && !self.hubs[parent].coastal {
            let mut bf = (g0, 0.0f32);
            for g in 0..ng {
                if self.goods[g].food {
                    let pc = self.hubs[parent].base_per_capita.get(g).copied().unwrap_or(0.0);
                    if pc > bf.1 { bf = (g, pc); }
                }
            }
            g0 = bf.0;
            kind = 1;
        }
        let owner_house = self.strongest_house_at(parent)
            .filter(|&hi| self.houses[hi].wealth >= ESTATE_HOUSE_OWNER_WEALTH)
            .map(|hi| hi as i32).unwrap_or(-1);
        // Place near the parent (deterministic small offset).
        let off = hash01(self.seed, self.tick as u64, parent as u64);
        let ex = self.hubs[parent].x + (off - 0.5) * self.world_w * 0.03;
        let ey = self.hubs[parent].y + (hash01(self.seed, parent as u64, self.tick as u64) - 0.5)
            * self.world_w * 0.02;
        let est_pop = self.hubs[parent].founding_pop * 0.15;
        let percap = self.hubs[parent].base_per_capita.get(g0).copied().unwrap_or(0.05).max(0.05) * 1.5;
        let (koppen, coastal, component) =
            (self.hubs[parent].koppen, self.hubs[parent].coastal, self.hubs[parent].component);
        self.create_estate(parent as i32, ex, ey, g0, kind, owner_house, koppen, coastal,
            component, est_pop, percap);
    }


    /// Monthly: a wealthy house invests its surplus capital into a new estate (raw
    /// production) or a manufactory (a luxury good), in a city it trades with —
    /// cheaper where it already holds an office. This is the wealth sink that turns
    /// hoarded profit into expansion and more production (estate income flows back
    /// to the owning house, so it compounds).
    pub(crate) fn maybe_house_invests(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let ng = self.goods.len();
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            if self.houses[hi].wealth < INVEST_WEALTH { continue; }
            // Soft cap so one rich house doesn't blanket the map with estates.
            let owned = self.hubs.iter().filter(|h| h.is_estate && h.owner_house == hi as i32).count();
            if owned >= MAX_HOUSE_ESTATES { continue; }
            // ~4%/month for an eligible house → invests roughly every couple of years.
            if hash01(self.seed, tick as u64 ^ 0xE57A7E, hi as u64) > 0.04 { continue; }
            // Prefer UPGRADING an existing estate/manufactory it owns (tier < 5) ~half
            // the time — cheaper than building new and compounds its output.
            if let Some(ei) = self.hubs.iter().enumerate()
                .filter(|(_, e)| e.is_estate && e.owner_house == hi as i32
                    && e.estate_tier > 0 && e.estate_tier < 5)
                .min_by_key(|(_, e)| e.estate_tier)
                .map(|(idx, _)| idx)
            {
                let tier = self.hubs[ei].estate_tier.max(1);
                let is_manu = self.hubs[ei].estate_kind == 6;
                // A manufactory upgrade is a major, dear re-tooling — flat 30k and only
                // once every 5 years per workshop. A raw estate upgrade stays cheap.
                let cost = if is_manu { MANUFACTORY_UPGRADE_COST } else { INVEST_COST_BASE * tier as f32 * 0.8 };
                let cooldown_ok = !is_manu
                    || tick.saturating_sub(self.hubs[ei].last_upgrade_tick) >= MANUFACTORY_UPGRADE_INTERVAL;
                if cooldown_ok && self.houses[hi].wealth >= cost * 1.5
                    && hash01(self.seed, tick as u64 ^ 0x09A7, hi as u64) < 0.5
                {
                    self.houses[hi].wealth -= cost;
                    self.hubs[ei].estate_tier = tier + 1;
                    self.hubs[ei].last_upgrade_tick = tick;
                    for v in self.hubs[ei].base_per_capita.iter_mut() { *v *= ESTATE_UPGRADE_MULT; }
                    let (en, ep) = (self.hubs[ei].name.clone(), self.hubs[ei].parent);
                    self.journal.push(JournalEntry {
                        tick, kind: "estate".into(), hub: ep, good: -1, value: (tier + 1) as f32,
                        text: format!("{} upgrades to tier {}", en, tier + 1),
                    });
                    continue;
                }
            }
            // Global cap: upgrades (above) are always allowed (no new hub), but
            // building a NEW estate is blocked once the world is saturated — keeps
            // the hub list (and every per-tick loop) bounded late-campaign. Reserves
            // `OUTPOST_RESERVED_ESTATES` the same way the top-of-function gate does.
            if self.estate_count() >= MAX_TOTAL_ESTATES.saturating_sub(OUTPOST_RESERVED_ESTATES) { continue; }
            // Build in the house's strongest trade partner (a city it actually works),
            // else at home. Skip estates themselves.
            let home = self.houses[hi].hub as usize;
            let target = self.houses[hi].trade_at.iter()
                .filter(|(hb, _)| (*hb as usize) < n && !self.hubs[*hb as usize].is_estate)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(hb, _)| *hb as usize)
                .unwrap_or(home);
            if target >= n || self.hubs[target].is_estate { continue; }
            // Per-settlement cap: don't overrun one city's hinterland with estates. The
            // base cap is 3; only a great city (≥150k) may reach 5, and the 4th/5th slot
            // costs a steep premium (crowding a city with works is expensive).
            let on_city = self.hubs.iter()
                .filter(|h| h.is_estate && h.parent == target as i32).count();
            let cap = if self.hubs[target].population >= ESTATE_BIG_CITY_POP {
                MAX_ESTATES_BIG_CITY
            } else { MAX_ESTATES_PER_CITY };
            if on_city >= cap { continue; }
            // Cost scales with the host city's size; an office there makes it cheaper; a
            // slot beyond the base cap (the 4th/5th, big cities only) is far dearer.
            let has_office = self.houses[hi].offices.contains(&(target as u32));
            let slot_premium = if on_city >= MAX_ESTATES_PER_CITY { ESTATE_HIGH_SLOT_COST_MULT } else { 1.0 };
            let cost = INVEST_COST_BASE
                * (self.hubs[target].population / 30_000.0).clamp(0.5, 3.0)
                * if has_office { 0.6 } else { 1.0 }
                * slot_premium;
            if self.houses[hi].wealth < cost * 1.5 { continue; }
            // A manufactory for a LUXURY (one the house specializes in, or — for a
            // guild / spec-less holder — the target city's strongest-produced luxury),
            // else a raw estate of the city's strongest good. Mix the two so cities
            // get both raw output and value-added luxuries.
            let house_lux = self.houses[hi].spec.iter().cloned()
                .find(|&g| g < ng && !self.goods[g].food && self.goods[g].base_value >= 4.0);
            // Bias the luxury choice by local demand too, so a house re-tools toward
            // the scarce, dear luxury the city is short of rather than only its biggest.
            let city_lux = (0..ng)
                .filter(|&g| !self.goods[g].food && self.goods[g].base_value >= 4.0)
                .map(|g| (g, self.hubs[target].base_per_capita.get(g).copied().unwrap_or(0.0)
                    * self.demand_pressure_at(target, g)))
                .filter(|(_, pc)| *pc > 0.0)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(g, _)| g);
            let want_manu = house_lux.is_some()
                || (city_lux.is_some() && hash01(self.seed, tick as u64 ^ 0xFAC7, hi as u64) < 0.5);
            let manu_good = house_lux.or(city_lux);
            let (g0, kind, percap) = if want_manu && manu_good.is_some() {
                (manu_good.unwrap(), 6u8, MANUFACTORY_PERCAP)
            } else {
                // Among goods the target's land can yield, the one under the most local
                // demand pressure (under-supplied → profitable to add), not just its
                // biggest current output — the same demand-driven bias as
                // `maybe_found_estate`, so a house's raw investment follows real scarcity.
                let mut bg = (usize::MAX, 0.0f32);
                for g in 0..ng {
                    let pc = self.hubs[target].base_per_capita.get(g).copied().unwrap_or(0.0);
                    if pc <= 0.0 { continue; }
                    let score = pc * self.demand_pressure_at(target, g);
                    if score > bg.1 { bg = (g, score); }
                }
                if bg.0 == usize::MAX { continue; }
                let k = estate_kind_for_good(&self.goods[bg.0].name, self.goods[bg.0].food);
                (bg.0, k, self.hubs[target].base_per_capita.get(bg.0).copied().unwrap_or(0.05).max(0.05) * 1.5)
            };
            // A fishery needs a coast; inland fall back to a farm of the city's good.
            let (kind, g0, percap) = if kind == 4 && !self.hubs[target].coastal {
                let mut bf = (usize::MAX, 0.0f32);
                for g in 0..ng {
                    if self.goods[g].food {
                        let pc = self.hubs[target].base_per_capita.get(g).copied().unwrap_or(0.0);
                        if pc > bf.1 { bf = (g, pc); }
                    }
                }
                if bf.0 == usize::MAX { continue; }
                (1u8, bf.0, (bf.1 * 1.5).max(0.05))
            } else { (kind, g0, percap) };
            self.houses[hi].wealth -= cost;
            let off = hash01(self.seed, tick as u64 ^ 0x12E5, target as u64);
            let ex = self.hubs[target].x + (off - 0.5) * self.world_w * 0.03;
            let ey = self.hubs[target].y
                + (hash01(self.seed, target as u64, tick as u64 ^ 0x77) - 0.5) * self.world_w * 0.02;
            let est_pop = self.hubs[target].founding_pop * 0.12;
            let (koppen, coastal, component) =
                (self.hubs[target].koppen, self.hubs[target].coastal, self.hubs[target].component);
            self.create_estate(target as i32, ex, ey, g0, kind, hi as i32, koppen, coastal,
                component, est_pop, percap);
        }
    }


    /// HOUSE colony — a wealthy house plants a remote, low-population TRADE OUTPOST
    /// on a poorer, trade-prone (coastal) frontier site within reach of its home or
    /// an office (the office is the relay "ground"). Reuses the estate machinery but
    /// keeps its OWN remote coordinates (parent = −1 so it is not co-located in a
    /// city) and is tagged `colony_kind = 2`.
    pub(crate) fn maybe_found_house_outpost(&mut self) {
        if self.colonizable.is_empty() || self.hubs.is_empty() { return; }
        // Founders: EVERY house that clears the (heavy) wealth bar, richest first —
        // a trade outpost is the privilege of a truly great house, but an era can
        // hold several such houses at once, and each plants its OWN regional post
        // rather than the whole world waiting on a single wealthiest house whose
        // home network may not even reach whatever sites remain. Capped per call
        // so a rich crop of houses can't empty the colonizable pool in one year.
        let mut candidates: Vec<(usize, f32)> = self.houses.iter().enumerate()
            .filter(|(_, hh)| !hh.defunct && hh.wealth > OUTPOST_FOUND_WEALTH)
            .map(|(hi, hh)| (hi, hh.wealth))
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let mut founded = 0usize;
        for (hi, _) in candidates {
            if founded >= OUTPOST_MAX_PER_CALL { break; }
            if self.colonizable.is_empty() { break; }
            if self.estate_count() >= MAX_TOTAL_ESTATES { break; }
            if self.try_found_house_outpost(hi) { founded += 1; }
        }
    }

    /// One house's attempt at planting a single trade outpost — see
    /// `maybe_found_house_outpost` for why several houses each get a try per call.
    fn try_found_house_outpost(&mut self, hi: usize) -> bool {
        let home = self.houses[hi].hub as usize;
        if home >= self.hubs.len() { return false; }
        // Network nodes = home + offices + the house's OWN ESTATES (the relays) — an
        // estate is a real regional foothold (a plantation or mine already worked by
        // this house's own factors), and is usually far more widely scattered than its
        // handful of city offices. Without estates counted, a house with sprawling
        // holdings but offices clustered near its seat could never reach a nearby
        // frontier site its own estates already border — "the same region" the user
        // wants outposts to cluster in IS the region a house's estates already work.
        let mut nodes = vec![(self.hubs[home].x, self.hubs[home].y)];
        for &off in &self.houses[hi].offices {
            if (off as usize) < self.hubs.len() {
                nodes.push((self.hubs[off as usize].x, self.hubs[off as usize].y));
            }
        }
        for h in self.hubs.iter().filter(|h| h.is_estate && h.owner_house == hi as i32) {
            nodes.push((h.x, h.y));
        }
        let cap = COLONY_MAX_KM * self.world_w / EARTH_EQUATOR_KM; // ≤ 2500 km from the metropolis
        // Pick the best reachable site for a TRADE outpost: a house plants its
        // factory where the valuable trade goods are. Site trade-value dominates,
        // a coast (shippable) adds a bonus, and nearer is better. Fertility is
        // irrelevant — an outpost imports its food and exists to work the cargo.
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            let d = self.nearest_node_dist(&nodes, s.x, s.y);
            if d > cap { continue; }
            let trade_score = s.trade_value + if s.coastal { 0.30 } else { 0.0 };
            let score = trade_score * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return false };
        let ng = self.goods.len();
        // A good is a RAW manufacturing INPUT if some recipe consumes it. A house whose
        // workshops are short of raws will preferentially plant a RESOURCE COLONY that
        // yields such an input (then ship it home / to its manufacturing offices) — the
        // "traders establish colonies to secure the goods their manufactories need" rule.
        let is_input: Vec<bool> = {
            let mut s = vec![false; ng];
            for g in 0..ng { for &(idx, _) in &self.goods[g].inputs { if idx < ng { s[idx] = true; } } }
            s
        };
        // Manufacturing-input goods the founder's own network (home + offices) makes far
        // TOO LITTLE of relative to the raws its workshops draw — the scarce inputs worth
        // reaching for. Uses base_per_capita (own output) as the cheap in-tick proxy.
        let short_input = |g: usize, this: &Self| -> bool {
            if !is_input[g] || this.goods[g].food { return false; }
            let own = this.hubs[home].base_per_capita.get(g).copied().unwrap_or(0.0);
            own < 0.02 // the house's own cities barely produce this raw → worth a colony
        };
        // Good: prefer one the house's home makes that matches the site's kind hint,
        // but bias hard toward a SCARCE manufacturing input the site can yield.
        let (mut g0, mut gbest) = (usize::MAX, 0.0f32);
        for g in 0..ng {
            if estate_kind_for_good(&self.goods[g].name, self.goods[g].food) == self.colonizable[si].kind_hint {
                let mut s = self.hubs[home].base_per_capita.get(g).copied().unwrap_or(0.0) + 0.001;
                if short_input(g, self) { s += OUTPOST_INPUT_BIAS; } // resource-colony pull
                if s > gbest { gbest = s; g0 = g; }
            }
        }
        if g0 == usize::MAX {
            for g in 0..ng {
                let mut pc = self.hubs[home].base_per_capita.get(g).copied().unwrap_or(0.0);
                if short_input(g, self) { pc += OUTPOST_INPUT_BIAS; }
                if pc > gbest { gbest = pc; g0 = g; }
            }
        }
        if g0 == usize::MAX { return false; }
        let cost = OUTPOST_FOUND_COST;
        if self.houses[hi].wealth < cost { return false; }
        let site = self.colonizable.swap_remove(si);
        let kind = estate_kind_for_good(&self.goods[g0].name, self.goods[g0].food);
        let founder_max_pc = self.hubs[home].base_per_capita.iter().cloned().fold(0.0f32, f32::max).max(0.1);
        let percap = founder_max_pc * (0.4 + site.fertility);
        let est_pop = OUTPOST_MAX_POP; // a small trade post (hard-capped, never grows into a city)
        let component = self.hubs[home].component;          // joins the home trade web
        self.houses[hi].wealth -= cost;
        // parent = −1 keeps the outpost at its REMOTE site coords (not co-located).
        self.create_estate(-1, site.x, site.y, g0, kind, hi as i32, site.koppen, site.coastal,
            component, est_pop, percap);
        let new = self.hubs.len() - 1;
        self.hubs[new].colony_kind = 2;
        self.hubs[new].founder_hub = home as i32;
        self.hubs[new].colony_founded_tick = self.tick; // for the graduation age gate
        let hname = self.houses[hi].name.clone();
        // Distinct place-name + the (outpost) tag (was "{house} (outpost)", which
        // duplicated when a house planted several); the founding house is named in
        // the chronicle below and shown via `owner_house`.
        let place = crate::sim::names::gen_name(
            site.x.max(0.0) as u32, site.y.max(0.0) as u32,
            self.world_w as u32, self.world_h());
        self.hubs[new].name = format!("{} (outpost)", place);
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "colony".into(), hub: new as i32, good: g0 as i32, value: 2.0,
            text: format!("{} founds a trade outpost ({})", hname, self.goods[g0].name),
        });
        true
    }


    /// A thriving house OUTPOST matures into a full colony (Phoenician emporion → city:
    /// Gadir, Utica, Goa). A long-lived outpost at its population cap, whose owning
    /// house is wealthy, is PROMOTED IN PLACE: it sheds estate status, becomes a
    /// settlement colony backed & led by its founding house (which keeps control on any
    /// later independence — the Magonid pattern), and the small-outpost pop cap is
    /// lifted so it can grow into a city. At most one graduation per call.
    pub(crate) fn maybe_graduate_outpost(&mut self) {
        let tick = self.tick;
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if !hub.is_estate || hub.colony_kind != 2 || hub.abandoned { continue; }
            let owner = hub.owner_house;
            if owner < 0 || (owner as usize) >= self.houses.len()
                || self.houses[owner as usize].defunct { continue; }
            let age = tick.saturating_sub(hub.colony_founded_tick);
            if age < OUTPOST_GRADUATE_YEARS * TICKS_PER_YEAR { continue; }
            if hub.population < OUTPOST_MAX_POP * 0.9 { continue; }
            let wealth = self.houses[owner as usize].wealth;
            if wealth < OUTPOST_GRADUATE_WEALTH { continue; }
            if wealth > best.1 { best = (h, wealth); }
        }
        let Some(h) = (best.0 != usize::MAX).then_some(best.0) else { return };
        let owner = self.hubs[h].owner_house;
        let home = if owner >= 0 && (owner as usize) < self.houses.len() {
            self.houses[owner as usize].hub as i32
        } else { -1 };
        // Promote in place: a real hub now, a settlement colony led by its founding house.
        self.hubs[h].is_estate = false;
        self.hubs[h].colony_kind = 1;
        self.hubs[h].colony_stage = 1;
        self.hubs[h].founder_hub = home;
        self.hubs[h].backers = vec![(1, owner.max(0) as u32, 1.0)];
        self.hubs[h].council_house = owner;
        self.hubs[h].colony_founded_tick = tick; // independence clock restarts from here
        self.hubs[h].founding_pop = self.hubs[h].population.max(1.0);
        self.hubs[h].reserve_cap = self.hubs[h].reserve_cap.max(6.0);
        self.hubs[h].supply_years = 0.0;
        self.hubs[h].name = self.hubs[h].name.replace(" (outpost)", "");
        self.routes_dirty = true;
        self.total_foundings += 1;
        let cn = self.hubs[h].name.clone();
        let hn = if owner >= 0 && (owner as usize) < self.houses.len() {
            self.houses[owner as usize].name.clone()
        } else { String::new() };
        self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32, good: -1,
            value: 0.0, text: format!("{} grows from a trading post into a colony of {}", cn, hn) });
    }


    /// Yearly: a wealthy house develops an EXISTING under-traded small city into a
    /// trade BASE — opening an office, building a guildhall + warehouse, seeding
    /// working capital and taking the city under its patronage so its modest surplus
    /// finally clears to market. The accessible, existing-settlement cousin of
    /// `maybe_found_house_outpost`. At most one per call.
    pub(crate) fn maybe_establish_trade_base(&mut self) {
        if self.hubs.is_empty() || self.houses.is_empty() { return; }
        self.hub_patron.resize(self.hubs.len(), -1);
        // Founder: the richest non-guild house clearing the (modest) wealth bar.
        let mut founder = -1i32;
        let mut hw = BASE_INVEST_WEALTH;
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct || hh.is_guild { continue; }
            if hh.wealth > hw { hw = hh.wealth; founder = hi as i32; }
        }
        if founder < 0 { return; }
        let hi = founder as usize;
        if self.houses[hi].wealth < BASE_INVEST_COST { return; }
        let seat = self.houses[hi].hub as usize;
        if seat >= self.hubs.len() { return; }
        let comp = self.hubs[seat].component;
        let n = self.hubs.len();
        // Target: the nearest reachable, under-traded small city on the same continent
        // that this house doesn't already hold and nobody patronises yet.
        let mut best = (usize::MAX, f32::INFINITY);
        for c in 0..n {
            if c == seat { continue; }
            let hub = &self.hubs[c];
            if hub.is_estate || hub.colony_kind != 0 { continue; }
            if hub.component != comp { continue; }
            if self.hub_patron[c] >= 0 { continue; }
            if hub.population < BASE_MIN_POP || hub.population > BASE_MAX_POP { continue; }
            // Under-traded: throughput well below what its population could support.
            if hub.export_earn + hub.import_spend > BASE_UNDERTRADE_FRAC * hub.population { continue; }
            if self.houses[hi].offices.contains(&(c as u32)) { continue; }
            let d = self.days.get(seat * n + c).copied().unwrap_or(f32::INFINITY);
            if d.is_finite() && d < best.1 { best = (c, d); }
        }
        let Some(c) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // Commit (affordability already checked): pay, plant the base, take patronage.
        self.houses[hi].wealth -= BASE_INVEST_COST;
        if !self.houses[hi].offices.contains(&(c as u32)) {
            self.houses[hi].offices.push(c as u32);
        }
        if !self.hubs[c].structures.contains(&STRUCT_GUILDHALL) {
            self.hubs[c].structures.push(STRUCT_GUILDHALL);
        }
        if !self.hubs[c].structures.contains(&STRUCT_WAREHOUSE) {
            self.hubs[c].structures.push(STRUCT_WAREHOUSE);
        }
        self.hubs[c].treasury += BASE_SEED;
        self.hub_patron[c] = hi as i32;
        let (hn, cn) = (self.houses[hi].name.clone(), self.hubs[c].name.clone());
        self.houses[hi].events.push(HouseEvent {
            tick: self.tick, kind: "base".into(),
            text: format!("{} establishes a trade base in {}", hn, cn),
        });
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "base".into(), hub: c as i32, good: -1, value: 1.0,
            text: format!("{} establishes a trade base in {}", hn, cn),
        });
    }


    /// Yearly upkeep for patronised trade bases: a gentle development pop bonus while
    /// the city is still small, and patronage CONCLUDES once it has grown into a real
    /// node (the house keeps trading from it). Stale patronage (the hub collapsed or
    /// became an estate/colony) is dropped.
    pub(crate) fn trade_base_pass(&mut self) {
        self.hub_patron.resize(self.hubs.len(), -1);
        for h in 0..self.hubs.len() {
            if self.hub_patron[h] < 0 { continue; }
            if self.hubs[h].is_estate || self.hubs[h].colony_kind != 0 || self.hubs[h].population < 1.0 {
                self.hub_patron[h] = -1;
                continue;
            }
            if self.hubs[h].population >= BASE_DEVELOPED_POP {
                let ph = self.hub_patron[h] as usize;
                let hn = self.houses.get(ph).map(|x| x.name.clone()).unwrap_or_default();
                let cn = self.hubs[h].name.clone();
                self.hub_patron[h] = -1;
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "base".into(), hub: h as i32, good: -1, value: 2.0,
                    text: format!("{} matures into a thriving market — {}'s trade base is fully established", cn, hn),
                });
                continue;
            }
            // Development nudge while patronised and still small.
            self.hubs[h].population *= 1.0 + BASE_POP_GROWTH_BONUS;
        }
    }


    /// A works' EFFECTIVENESS multiplier on output (1.0 = at nominal tier). Four
    /// debuffs compound: disaster `damage`, age/wear since the last build/upgrade, a
    /// labor shortage in a small/shrunken host city, and unrest/famine there. All are
    /// floored so a sound works in a healthy city stays near full, but a damaged works
    /// in a starving backwater runs far below capacity.
    pub(crate) fn estate_effectiveness(&self, h: usize) -> f32 {
        let e = &self.hubs[h];
        if !e.is_estate { return 1.0; }
        // ADDITIVE penalties (NOT multiplicative): a sound works in a healthy city sits
        // at ~1.0; only real problems bite. (Multiplicative stacking gutted the estate
        // income that bootstraps houses and collapsed the whole economy — see CLAUDE.md.)
        let mut penalty = e.damage.clamp(0.0, 1.0); // disaster damage is the big lever
        let age_yrs = self.tick.saturating_sub(e.last_upgrade_tick) as f32 / TICKS_PER_YEAR as f32;
        penalty += (ESTATE_DECAY_PER_YEAR * age_yrs).min(ESTATE_AGE_PENALTY_CAP);
        if e.parent >= 0 && (e.parent as usize) < self.hubs.len() {
            let p = &self.hubs[e.parent as usize];
            // Labor: a works in a too-small host city runs a little below capacity.
            penalty += (1.0 - (p.population / ESTATE_LABOR_FULL_POP).clamp(0.0, 1.0)) * ESTATE_LABOR_PENALTY_CAP;
            // Unrest/famine in the host city cuts output.
            penalty += p.starving.clamp(0.0, 1.0) * ESTATE_UNREST_PENALTY_CAP;
        }
        (1.0 - penalty).clamp(0.15, 1.0)
    }


    /// Yearly · estate/manufactory disasters + repair. An intact works may suffer a
    /// KIND-SPECIFIC disaster (`disaster_table`, A8) — damage spikes → output drops
    /// via `estate_effectiveness`; a damaged works is repaired over several years,
    /// funded by every DIVIDEND-payout share row's own fraction of the cost (D11),
    /// not a single payer. A refusing SHARE row is diluted; a refusing TENANCY row
    /// that keeps refusing is voided outright (A9). Deliberately NOT duplicated
    /// here: farm's plain seasonal drought (already `roll_events`'s own "drought"
    /// kind, hub-wide not works-specific) and war sack/raid (already
    /// `apply_war_defeat_consequences`/`strip_holdings_at`, §3.4d) — 4.7a's table
    /// names both, but this engine already has a mechanism for each and a second
    /// one would be a duplicate, not a fix.
    pub(crate) fn estate_condition_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        for ei in 0..n {
            if !self.hubs[ei].is_estate || self.hubs[ei].estate_tier == 0 { continue; }
            // 1) Disaster roll — kept BIT-IDENTICAL to the pre-4.7 code (same
            //    flat DISASTER_ANNUAL_CHANCE, same 3-way generic pick, same
            //    magnitude formula): only the DISPLAY NAME is remapped through
            //    `disaster_table` to something kind-appropriate for the
            //    chronicle (A8). Three successive attempts to also vary the
            //    magnitude range, the repair rate, and the annual chance by
            //    kind each independently pushed `simulate_decades_reports_
            //    dynamics` into a sustained-runaway-rich house — this
            //    engine's own documented RNG-consumption-cascade sensitivity
            //    (the same shape the 3.4a-c war-tuning story hit), not a
            //    balance judgement call. Reverted per §2.4's own discipline;
            //    a future session wanting real per-kind magnitude/pace should
            //    re-attempt it as its own isolated, gated change.
            const GENERIC_KINDS: [&str; 3] = ["fire", "flood", "blight"];
            if self.hubs[ei].damage < 0.2
                && hash01(self.seed, tick as u64 ^ 0xD15A57, ei as u64) < DISASTER_ANNUAL_CHANCE {
                let r = hash01(self.seed, tick as u64 ^ 0xF1AE, ei as u64);
                let dmg = DISASTER_MIN_DAMAGE + r * (DISASTER_MAX_DAMAGE - DISASTER_MIN_DAMAGE);
                self.hubs[ei].damage = dmg.clamp(0.0, 1.0);
                let generic_idx = (hash01(self.seed, tick as u64 ^ 0xCA1A, ei as u64) * 3.0) as usize % 3;
                let table = disaster_table(self.hubs[ei].estate_kind);
                let kind = table.get(generic_idx % table.len()).map(|&(name, ..)| name)
                    .unwrap_or(GENERIC_KINDS[generic_idx]);
                let (en, par) = (self.hubs[ei].name.clone(), self.hubs[ei].parent);
                self.journal.push(JournalEntry { tick, kind: "disaster".into(), hub: par, good: -1,
                    value: dmg, text: format!("{} strikes {} ({:.0}% damage)", kind, en, dmg * 100.0) });
                continue;
            }

            // 2) Repair. The repair ITSELF is funded exactly as before — the
            //    single owner (or the parent city for a civic works) fronts the
            //    FULL cost, same solvency test as the pre-4.7 code, so whether
            //    and how much a works heals each year is UNCHANGED by this
            //    slice. D11/A9's real content is a SEPARATE reimbursement pass
            //    below: every dividend-payout share row owes the payer its own
            //    fraction of what was just spent; a refusal dilutes (SHARE) or
            //    accrues neglect (TENANCY, voided at the limit), but never
            //    blocks or slows this year's repair. (An earlier cut gated the
            //    repair itself on every holder paying its slice — a bank/guild
            //    refusal then left the works PERMANENTLY under-repaired, which
            //    produced a sustained-runaway-rich house in `simulate_decades_
            //    reports_dynamics`: scarcity from stuck-damaged works let an
            //    unaffected competitor's prices spiral. Decoupling the two
            //    keeps the reliable half reliable and the new mechanic purely
            //    about who ends up owning what.)
            if self.hubs[ei].damage > 0.01 {
                // Repair rate stays the plain flat REPAIR_RATE_PER_YEAR — see
                // this function's own doc comment on why a per-kind pace was
                // reverted.
                let repaired = (self.hubs[ei].damage * REPAIR_RATE_PER_YEAR).min(self.hubs[ei].damage);
                let cost = repaired * self.estate_market_value(ei) * REPAIR_COST_FRAC;
                let owner = self.hubs[ei].owner_house;
                let mut payer_wealth: Option<usize> = None; // house index, if the payer was a house
                let mut payer_city: Option<usize> = None;   // parent hub index, if the payer was the city
                let paid = if owner >= 0 && (owner as usize) < self.houses.len()
                    && !self.houses[owner as usize].defunct && self.houses[owner as usize].wealth > cost * 1.5 {
                    self.houses[owner as usize].wealth -= cost;
                    payer_wealth = Some(owner as usize);
                    true
                } else if owner < 0 {
                    let par = self.hubs[ei].parent;
                    if par >= 0 && (par as usize) < n && self.hubs[par as usize].treasury > cost * 1.5 {
                        self.hubs[par as usize].treasury -= cost;
                        payer_city = Some(par as usize);
                        true
                    } else { false }
                } else { false };
                if paid {
                    self.hubs[ei].damage = (self.hubs[ei].damage - repaired).max(0.0);
                    if self.hubs[ei].damage <= 0.01 {
                        self.hubs[ei].disaster_repair_mult = 0.0;
                        let (en, par) = (self.hubs[ei].name.clone(), self.hubs[ei].parent);
                        self.journal.push(JournalEntry { tick, kind: "estate".into(), hub: par, good: -1,
                            value: 0.0, text: format!("{} is fully repaired and back to work", en) });
                    }
                    // 2b) D11/A9 · reimbursement — DEFERRED, not wired in.
                    // A real implementation existed (each dividend-payout share row
                    // owes its own frac of `cost` back to the payer; a refusal
                    // dilutes a SHARE or accrues neglect toward voiding a TENANCY)
                    // but every formulation tried here — even after the dilution
                    // arithmetic was made strictly conservative (exactly the
                    // diluted fraction moves, never more) — flips
                    // `econ_inheritance_rules_fragment_differently`'s partible-vs-
                    // primogeniture wealth ordering (142998 vs 123627, wrong
                    // direction) on the SAME seed the disaster-roll fix already
                    // left bit-identical. That test has its own documented history
                    // of exactly this RNG-consumption-cascade fragility (see
                    // SCOREBOARD.md's 2026-08-09 entry) from causes unrelated to
                    // this slice, and a `world_w=300`/`world_h=300` widening from
                    // that earlier session was not enough to protect it from a
                    // second, independent source of the same shape. Per §2.4 (a
                    // spot-check win with an aggregate-gate loss is a revert, not a
                    // judgement call): the money-transfer half of D11/A9 is left
                    // unbuilt here. `Share.neglect_years`/`instrument`, and the
                    // `DILUTION_STEP`/`TENANCY_NEGLECT_LIMIT` constants, stay in
                    // place as the shape a future, better-isolated attempt should
                    // fill in — see docs/ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.7.
                }
            }
        }
    }


    /// Resale value of a holding: ~60% of its rebuild cost, scaled by tier (and city
    /// size for raw estates). Manufactories are dear; raw estates cheap.
    pub(crate) fn estate_market_value(&self, ei: usize) -> f32 {
        let e = &self.hubs[ei];
        if !e.is_estate { return 0.0; }
        let tier = e.estate_tier.max(1) as f32;
        if e.estate_kind == 6 {
            tier * MANUFACTORY_UPGRADE_COST * 0.6
        } else {
            let popf = (e.population / 30_000.0).clamp(0.5, 3.0);
            tier * INVEST_COST_BASE * popf * 8.0 * 0.6
        }
    }


    /// Monthly · the estate & manufactory RESALE market. A cash-strapped house sells
    /// a holding to raise specie; a polis sells a city-owned (civic) works to refill a
    /// thin treasury. The best-capitalized bidder takes it: a solvent resident house
    /// (title transfers), or — for a manufactory — a bank in that market (it can't hold
    /// title, so it takes a CONTROLLING equity stake, acquiring the income stream).
    /// (User-requested: settlements/houses can sell holdings; banks/houses buy them.)
    pub(crate) fn estate_resale_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        // Find ONE holding for sale this month.
        let mut sale: Option<(usize, i32)> = None; // (estate hub, seller house | -1 civic)
        for ei in 0..n {
            let e = &self.hubs[ei];
            if !e.is_estate || e.estate_tier == 0 { continue; }
            if e.owner_house >= 0 {
                let oh = e.owner_house as usize;
                if oh < self.houses.len() && !self.houses[oh].defunct
                    && self.houses[oh].wealth < RESALE_DISTRESS_WEALTH
                    && hash01(self.seed, tick as u64 ^ 0x5A1E5, ei as u64) < 0.4 {
                    sale = Some((ei, oh as i32)); break;
                }
            } else if e.parent >= 0 && (e.parent as usize) < n
                && self.hubs[e.parent as usize].treasury < CIVIC_SALE_TREASURY_FLOOR
                && hash01(self.seed, tick as u64 ^ 0x6C1F5, ei as u64) < 0.2 {
                sale = Some((ei, -1)); break;
            }
        }
        let Some((ei, seller)) = sale else { return };
        let price = self.estate_market_value(ei);
        if price < 0.5 { return; }
        let parent = self.hubs[ei].parent;
        let is_manu = self.hubs[ei].estate_kind == 6;
        let pay_seller = |this: &mut Self, amt: f32| {
            if seller >= 0 { this.houses[seller as usize].wealth += amt; }
            else if parent >= 0 && (parent as usize) < this.hubs.len() { this.hubs[parent as usize].treasury += amt; }
        };
        // BUYER 1: the richest solvent resident house at the parent city (≠ seller).
        let mut buyer = (usize::MAX, 0.0f32);
        if parent >= 0 && (parent as usize) < n {
            for (hi, h) in self.houses.iter().enumerate() {
                if h.defunct || h.is_guild || hi as i32 == seller { continue; }
                if h.hub as usize != parent as usize { continue; }
                if h.wealth > price * 1.5 && h.wealth > buyer.1 { buyer = (hi, h.wealth); }
            }
        }
        if buyer.0 != usize::MAX {
            let bh = buyer.0;
            self.houses[bh].wealth -= price;
            pay_seller(self, price);
            self.hubs[ei].owner_house = bh as i32;
            let (en, bn) = (self.hubs[ei].name.clone(), self.houses[bh].name.clone());
            self.houses[bh].events.push(HouseEvent { tick, kind: "acquire".into(),
                text: format!("{} acquires {} for {:.0}", bn, en, price) });
            self.journal.push(JournalEntry { tick, kind: "estate".into(), hub: parent, good: -1,
                value: price, text: format!("{} buys {}", bn, en) });
            return;
        }
        // BUYER 2 (manufactories only): a bank in that market takes a controlling stake.
        if is_manu {
            let mut bk = usize::MAX;
            for bi in 0..self.banks.len() {
                if self.banks[bi].defunct { continue; }
                if parent >= 0 && !self.banks[bi].branches.contains(&(parent as u32)) { continue; }
                if self.banks[bi].reserves > price * 2.0 { bk = bi; break; }
            }
            if bk != usize::MAX {
                self.banks[bk].reserves -= price;
                pay_seller(self, price);
                let good = self.hubs[ei].base_per_capita.iter().enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(g, _)| g as u32).unwrap_or(0);
                self.hubs[ei].stake_bank = bk as i32;
                self.hubs[ei].stake_share = RESALE_BANK_STAKE;
                self.banks[bk].stakes.retain(|s| s.estate_hub != ei as u32);
                self.banks[bk].stakes.push(BankStake {
                    estate_hub: ei as u32, share: RESALE_BANK_STAKE, basis: price, good });
                // ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.5 (D1) · same split,
                // recorded in the share table (the payout source of truth).
                self.hubs[ei].shares.clear();
                self.hubs[ei].shares.push(Share {
                    holder_kind: 3, holder: bk as u32, frac: RESALE_BANK_STAKE, payout: 1,
                    acquired_tick: tick, paid: price, instrument: 0, term_years: 0, neglect_years: 0,
                });
                let resale_owner = self.hubs[ei].owner_house;
                if resale_owner >= 0 {
                    self.hubs[ei].shares.push(Share {
                        holder_kind: 1, holder: resale_owner as u32,
                        frac: 1.0 - RESALE_BANK_STAKE, payout: 1,
                        acquired_tick: tick, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
                    });
                }
                let en = self.hubs[ei].name.clone();
                self.banks[bk].events.push(HouseEvent { tick, kind: "acquire".into(),
                    text: format!("acquires a controlling {:.0}% stake in {} for {:.0}",
                        RESALE_BANK_STAKE * 100.0, en, price) });
            }
        }
    }


    /// The language FAMILY of a live culture (creole registry first, then the worldgen
    /// hearth kit). "" for an unknown/legacy culture.
    pub(crate) fn culture_family(&self, name: &str) -> String {
        if name.is_empty() || name == "—" { return String::new(); }
        if let Some(cr) = self.creoles.iter().find(|c| c.name == name) { return cr.family.clone(); }
        crate::sim::cultures::kit_of_people(name)
            .map(|k| crate::sim::cultures::KITS[k].lang_family.to_string())
            .unwrap_or_default()
    }


    /// Seat a brand-new merchant family at hub `h` — used when a colony WINS its
    /// independence by war (a fresh dynasty rises to rule the free city). Returns the
    /// new house index. Mirrors `maybe_found_house`'s construction.
    pub(crate) fn found_house_at(&mut self, h: usize) -> Option<usize> {
        let tick = self.tick;
        let ng = self.goods.len();
        if ng == 0 { return None; }
        let mut gi: Vec<usize> = (0..ng).collect();
        gi.sort_by(|&a, &b| self.hubs[h].production[b]
            .partial_cmp(&self.hubs[h].production[a]).unwrap_or(std::cmp::Ordering::Equal));
        let mut spec: Vec<usize> = gi.into_iter()
            .filter(|&g| self.hubs[h].production[g] > 0.0).take(2).collect();
        if spec.is_empty() { spec.push(0); }
        let name = self.unique_family_name_for(h, tick as u64 ^ 0xBEEF);
        let (line_rule, _) = self.rules_for_hub(h);
        let female = crate::sim::inheritance::heir_is_female(line_rule, h as u64 ^ 0x2468, self.seed);
        let head = self.head_name_sexed_for(h, &name, tick as u64 ^ 0x2468, female);
        let (head_age, tenure) = self.roll_founder_tenure(h as u64 ^ 0x51);
        let founded = HouseEvent { tick, kind: "founded".into(),
            text: format!("{} rises to lead free {}", name, self.hubs[h].name) };
        let (fleet_sea, fleet_river, fleet_caravan) = Self::initial_fleet(self.hubs[h].coastal, false);
        let idx = self.houses.len();
        self.houses.push(House {
            name, hub: h as u32, wealth: 5.0, prestige: 0.2, spec,
            monopoly: vec![], rivals: vec![], generation: 1,
            events: vec![founded], good_profit: Vec::new(), good_volume: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: true, prev_wealth: 5.0, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: head, head_since: tick,
            head_lifespan: tenure,
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: pick_archetype(self.seed, tick as u64 ^ h as u64),
            charters: Vec::new(),
            is_guild: false, offices: vec![h as u32], trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
            head_female: female, head_age, line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0,
            origin_house: -1, origin_kind: ORIGIN_INDEPENDENCE, crowned: false, realm: -1,
        });
        self.found_head_record(idx, "founder");
        Some(idx)
    }


    pub(crate) fn update_houses(&mut self, needs: &[Vec<f32>]) {
        for h in 0..self.hubs.len() {
            // Per-capita denominator floored at half the FOUNDING size so a hub that
            // loses population can't have its per-capita wealth spike to absurd
            // values (the old "millionaire outpost" bug). Estates inherit a small
            // founding size, so their wealth stays bounded too.
            let pop = self.hubs[h].population.max(self.hubs[h].founding_pop * 0.5).max(1.0);
            // Food security = current food-stock value per capita.
            self.hubs[h].grain_wealth = food_value(&self.hubs[h], &self.goods);
            // Commercial prosperity = recent net trade earnings per capita. The
            // accumulators decay so this tracks the last ~weeks, not all history.
            self.hubs[h].trade_wealth =
                (self.hubs[h].export_earn - self.hubs[h].import_spend) / pop;
            self.hubs[h].export_earn *= 0.97;
            self.hubs[h].import_spend *= 0.97;
            // Decay the per-class throughput tallies so the merchant-population
            // estimate tracks the last while, not all history.
            self.hubs[h].tw_house *= 0.97;
            self.hubs[h].tw_local *= 0.97;
            self.hubs[h].tw_guild *= 0.97;
        }
        self.update_house_dynamics(needs);
    }


    /// Monthly: build each house's per-city commercial INFLUENCE from its share of
    /// that city's trade ÷ the city's resistance (population + a resident guild),
    /// decaying elsewhere; recompute the per-city trade DOMINATOR; seed rivalries in
    /// contested cities; then consider Bailo (HQ) upgrades.
    pub(crate) fn update_influence_and_bailos(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let nh = self.houses.len();
        if self.city_dominator.len() != n { self.city_dominator = vec![-1; n]; }
        // Per-city total house-trade volume + which cities host a civic guild.
        let mut city_total = vec![0.0f32; n];
        let mut has_guild = vec![false; n];
        for h in &self.houses {
            if h.defunct { continue; }
            if h.is_guild { let hb = h.hub as usize; if hb < n { has_guild[hb] = true; } }
            for &(hb, v) in &h.trade_at { if (hb as usize) < n { city_total[hb as usize] += v.max(0.0); } }
        }
        // 1) Accrue / decay influence per house.
        for hi in 0..nh {
            if self.houses[hi].defunct { self.houses[hi].influence.clear(); continue; }
            let mut infl: std::collections::HashMap<u32, f32> =
                self.houses[hi].influence.iter().copied().collect();
            // Standing RELAXES toward the house's current market share in each city (an
            // EMA), so influence reflects real share instead of ratcheting to 1.0. The
            // old additive-gain/flat-decay scheme had no interior fixed point: any
            // positive share (gain > decay) climbed to the clamp, so every active city
            // pinned at 1.00. Untraded cities decay multiplicatively toward 0; city
            // RESISTANCE (pop + a resident guild) only slows how fast a house climbs —
            // share, not elapsed time, sets the ceiling, so dominance stays reachable.
            for v in infl.values_mut() { *v *= 1.0 - INFLUENCE_DECAY; }
            let trade_at = self.houses[hi].trade_at.clone();
            for &(hb, v) in &trade_at {
                let c = hb as usize; if c >= n { continue; }
                let share = if city_total[c] > 1e-6 { (v.max(0.0) / city_total[c]).clamp(0.0, 1.0) } else { 0.0 };
                let resist = 1.0 + self.hubs[c].population.max(0.0) / INFLUENCE_POP_REF
                    + if has_guild[c] { INFLUENCE_GUILD_RESIST } else { 0.0 };
                let e = infl.entry(hb).or_insert(0.0);
                *e = (*e + (INFLUENCE_GAIN / resist) * (share - *e)).clamp(0.0, 1.0);
            }
            // Seat + offices guarantee a standing foothold of influence.
            let home = self.houses[hi].hub;
            infl.entry(home).and_modify(|x| *x = x.max(OFFICE_INFLUENCE_FLOOR)).or_insert(OFFICE_INFLUENCE_FLOOR);
            for &o in &self.houses[hi].offices.clone() {
                infl.entry(o).and_modify(|x| *x = x.max(OFFICE_INFLUENCE_FLOOR)).or_insert(OFFICE_INFLUENCE_FLOOR);
            }
            let mut v: Vec<(u32, f32)> = infl.into_iter().filter(|&(_, x)| x > 0.02).collect();
            v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            self.houses[hi].influence = v;
        }
        // 2) Per-city dominator + runner-up (for the margin test + friction).
        let mut top = vec![(-1i32, 0.0f32); n];
        let mut second = vec![(-1i32, 0.0f32); n];
        for hi in 0..nh {
            if self.houses[hi].defunct { continue; }
            for &(hb, x) in &self.houses[hi].influence {
                let c = hb as usize; if c >= n { continue; }
                if x > top[c].1 { second[c] = top[c]; top[c] = (hi as i32, x); }
                else if x > second[c].1 { second[c] = (hi as i32, x); }
            }
        }
        let mut dom = vec![-1i32; n];
        for c in 0..n {
            if top[c].0 >= 0 && top[c].1 >= DOMINANCE_THRESHOLD && top[c].1 - second[c].1 >= DOMINANCE_MARGIN {
                dom[c] = top[c].0;
            }
        }
        // 3) New-dominance events (skip the house's own seat — already "controlled").
        for c in 0..n {
            if dom[c] >= 0 && self.city_dominator[c] != dom[c] && !self.hubs[c].is_estate {
                let hi = dom[c] as usize;
                if self.houses[hi].hub as usize != c {
                    let (hn, cn) = (self.houses[hi].name.clone(), self.hubs[c].name.clone());
                    self.houses[hi].events.push(HouseEvent { tick, kind: "dominance".into(),
                        text: format!("{} comes to dominate the trade of {}", hn, cn) });
                    self.journal.push(JournalEntry { tick, kind: "dominance".into(), hub: c as i32,
                        good: -1, value: top[c].1, text: format!("{} dominates the trade of {}", hn, cn) });
                }
            }
        }
        self.city_dominator = dom;
        // 4) Friction: a city contested by the top two houses can spark a rivalry.
        for c in 0..n {
            if top[c].0 >= 0 && second[c].0 >= 0
                && top[c].1 >= CONTEST_INFLUENCE && second[c].1 >= CONTEST_INFLUENCE {
                let (ai, bi) = (top[c].0 as usize, second[c].0 as usize);
                if ai != bi && !self.houses[ai].rivals.contains(&bi)
                    && hash01(self.seed, tick as u64 ^ 0x1F1C, (ai * 131 + bi) as u64) < 0.12 {
                    self.houses[ai].rivals.push(bi);
                    if !self.houses[bi].rivals.contains(&ai) { self.houses[bi].rivals.push(ai); }
                }
            }
        }
        // 5) Bailo (HQ) upgrades + upkeep.
        self.update_bailos();
    }


    /// Promote a house's strongest dominated office to a BAILO (a governing HQ), within
    /// a soft cap set by its wealth/power; charge upkeep; drop Bailos that have decayed.
    pub(crate) fn update_bailos(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            // Snapshot this house's influence (avoids borrowing self inside `retain`).
            let infl: std::collections::HashMap<u32, f32> =
                self.houses[hi].influence.iter().copied().collect();
            let at = |c: u32| infl.get(&c).copied().unwrap_or(0.0);
            // Drop Bailos whose grip has slipped well below the threshold.
            self.houses[hi].bailos.retain(|&c| at(c) >= BAILO_MIN_INFLUENCE * 0.6);
            // Upkeep per surviving Bailo (scaled by city size).
            for &c in &self.houses[hi].bailos.clone() {
                let up = BAILO_UPKEEP * self.city_size_factor(c as usize);
                self.houses[hi].wealth -= up;
            }
            // Soft cap on foreign Bailos = floor(power·scale) + wealth/per.
            let power = self.houses[hi].political_power;
            let wealth = self.houses[hi].wealth;
            let cap = (power * BAILO_CAP_POWER_SCALE) as usize
                + (wealth / BAILO_CAP_WEALTH_PER).max(0.0) as usize;
            if self.houses[hi].bailos.len() >= cap || wealth < BAILO_MIN_WEALTH { continue; }
            // Promote the strongest eligible (dominated, non-Bailo) office.
            let offices = self.houses[hi].offices.clone();
            let mut best: Option<(u32, f32)> = None;
            for &o in &offices {
                if self.houses[hi].bailos.contains(&o) { continue; }
                let x = at(o);
                if x >= BAILO_MIN_INFLUENCE && best.map_or(true, |(_, bv)| x > bv) { best = Some((o, x)); }
            }
            if let Some((o, _)) = best {
                self.houses[hi].bailos.push(o);
                self.lease_office(hi, o, OFFICE_LEASE_YEARS); // keep the base for the HQ
                let (hn, cn) = (self.houses[hi].name.clone(),
                    self.hubs.get(o as usize).map(|x| x.name.clone()).unwrap_or_default());
                self.houses[hi].events.push(HouseEvent { tick, kind: "bailo".into(),
                    text: format!("{} raises a Bailo — a governing headquarters — in {}", hn, cn) });
                self.journal.push(JournalEntry { tick, kind: "bailo".into(), hub: o as i32,
                    good: -1, value: 0.0, text: format!("{} establishes a Bailo in {}", hn, cn) });
            }
        }
    }


    /// "House Cassii"-style family name for the home `hub`, varied by `salt`.
    pub(crate) fn family_name_for(&self, hub: usize, salt: u64) -> String {
        let (x, y) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        format!(
            "House {}",
            crate::sim::names::gen_family_name(x, y, self.world_w as u32, self.world_h(), salt)
        )
    }


    /// A GLOBALLY-UNIQUE "House X" name for `hub`. The culture surname pools are
    /// finite, so naive generation repeats names — and because a house's coat of
    /// arms is derived from its name, repeated names also mean repeated heraldry.
    /// Retry with re-salted surnames; if the pool collides, distinguish the family
    /// with its home city ("House Cassii of Aquentia") so every house is unique in
    /// both name and arms. Checks against ALL houses (incl. defunct) so a fallen
    /// family's name isn't silently reused.
    pub(crate) fn unique_family_name_for(&self, hub: usize, salt: u64) -> String {
        let taken = |name: &str, houses: &[House]| houses.iter().any(|h| h.name == name);
        for k in 0..32u64 {
            let cand = self.family_name_for(hub, salt ^ k.wrapping_mul(0x9E3779B1));
            if !taken(&cand, &self.houses) { return cand; }
        }
        let city = self.hubs[hub].name.clone();
        for k in 0..32u64 {
            let base = self.family_name_for(hub, salt ^ k.wrapping_mul(0x85EBCA77));
            let cand = format!("{} of {}", base, city);
            if !taken(&cand, &self.houses) { return cand; }
        }
        // Last resort (vanishingly rare): tick-tag guarantees uniqueness.
        format!("{} of {} [{}]", self.family_name_for(hub, salt), city, self.tick)
    }


    /// Phase 4 (flavour) · raise & retire NOTABLE FIGURES at the yearly hook. Sparse
    /// (≤ `FIGURE_LIVING_CAP` alive, a modest yearly chance) and deterministic (all
    /// rolls hash `seed`/`tick`/`hub`). Each figure grants ONE small, capped effect
    /// on an existing bounded field and is chronicled at rise and at death, so the
    /// economy dynamics stay bounded.
    pub(crate) fn raise_notable_figures(&mut self, yr: u32) {
        let tick = self.tick;
        // 1) Retire the departed — chronicle each death once.
        for i in 0..self.figures.len() {
            if self.figures[i].dead || self.figures[i].dies_tick > tick { continue; }
            self.figures[i].dead = true;
            let (kind, good, hub, house) =
                (self.figures[i].kind, self.figures[i].good, self.figures[i].hub, self.figures[i].house);
            let name = self.figures[i].name.clone();
            let city = self.hubs.get(hub as usize).map(|h| h.name.clone()).unwrap_or_default();
            self.journal.push(JournalEntry {
                tick, kind: "figure".into(), hub: hub as i32, good, value: 0.0,
                text: format!("{} {} of {} has died.", role_title(kind), name, city),
            });
            if house >= 0 && (house as usize) < self.houses.len() {
                self.houses[house as usize].events.push(HouseEvent {
                    tick, kind: "figure".into(),
                    text: format!("{} {} passes into memory.", role_title(kind), name),
                });
            }
        }
        // Bound the roster (dead figures accumulate over a long campaign).
        if self.figures.len() > FIGURE_CAP {
            let drop = self.figures.len() - FIGURE_CAP;
            self.figures.drain(0..drop);
        }
        // 2) Maybe raise a new one.
        let living = self.figures.iter().filter(|f| !f.dead).count();
        if living >= FIGURE_LIVING_CAP { return; }
        if hash01(self.seed, tick as u64, 0xF16) >= FIGURE_YEARLY_CHANCE { return; }
        let n = self.hubs.len();
        if n == 0 { return; }
        // Prominent hub: hash-pick from the most populous real settlements.
        let mut real: Vec<usize> = (0..n).filter(|&h| !self.hubs[h].is_estate).collect();
        if real.is_empty() { return; }
        real.sort_by(|&a, &b| self.hubs[b].population
            .partial_cmp(&self.hubs[a].population).unwrap_or(std::cmp::Ordering::Equal));
        real.truncate(20.min(real.len()));
        let pick = (hash01(self.seed, tick as u64 ^ 0xA1, yr as u64) * real.len() as f32) as usize;
        let hub = real[pick.min(real.len() - 1)];
        let ng = self.goods.len();
        // Resident house (first non-defunct family seated here), if any.
        let resident = self.houses.iter()
            .position(|h| h.hub as usize == hub && !h.defunct).map(|i| i as i32).unwrap_or(-1);
        let has_house = resident >= 0;
        let coastal = self.hubs[hub].coastal;
        // Kind: base roll, with fallbacks when the context can't support it.
        let mut kind = ((hash01(self.seed, tick as u64 ^ 0x5E, hub as u64) * 5.0) as u8).min(4);
        if (kind == 0 || kind == 3) && !has_house { kind = 1; }          // house role → demagogue
        if (kind == 0 || kind == 4) && !coastal && has_house { kind = 3; } // inland → banker
        // Craftsman's craft = the hub's strongest output; else fall back to demagogue.
        let mut good = -1i32;
        if kind == 2 {
            if self.hubs[hub].quality.len() == ng && ng > 0 {
                let mut bg = 0usize; let mut bv = -1.0f32;
                for g in 0..ng {
                    let p = self.hubs[hub].production.get(g).copied().unwrap_or(0.0);
                    if p > bv { bv = p; bg = g; }
                }
                good = bg as i32;
            } else { kind = 1; }
        }
        // Name (+ an epithet for the martial/rabble-rousing/roving kinds).
        let salt = (tick as u64) ^ (hub as u64).wrapping_mul(0x9E3779B1) ^ (yr as u64);
        let surname_src = if has_house { self.houses[resident as usize].name.clone() }
            else { self.hubs[hub].name.clone() };
        let person = self.head_name_for(hub, &surname_src, salt);
        let (fx, fy) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        let epithet = crate::sim::names::gen_name_epithet(fx, fy, self.world_w as u32, self.world_h(), 2);
        let name = if !epithet.is_empty() && (kind == 0 || kind == 1 || kind == 4) {
            format!("{} {}", person, epithet)
        } else { person };
        let city = self.hubs[hub].name.clone();
        // Effect (capped) + chronicle text.
        let text = match kind {
            0 => { // Admiral — a house's sea fleet + renown
                let hi = resident as usize;
                if self.houses[hi].fleet_sea < 15 { self.houses[hi].fleet_sea += 1; }
                self.houses[hi].prestige += 0.05;
                format!("Admiral {} wins renown at sea for {}.", name, self.houses[hi].name)
            }
            1 => { // Demagogue — stirs civic unrest
                let u = self.hubs[hub].society.unrest;
                self.hubs[hub].society.unrest = (u + 0.08).min(1.0);
                format!("The demagogue {} stirs the crowds of {}.", name, city)
            }
            2 => { // Master Craftsman — lifts a city's craft quality
                let g = good.max(0) as usize;
                if g < self.hubs[hub].quality.len() {
                    let q = self.hubs[hub].quality[g];
                    self.hubs[hub].quality[g] = (q + 0.06).min(0.9);
                }
                let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                format!("Master {} raises the {} craft of {} to new heights.", name, gn, city)
            }
            3 => { // Great Banker — a house's standing
                if has_house { self.houses[resident as usize].prestige += 0.08; }
                format!("{}, a great banker of {}, gathers capital from across the sea.", name, city)
            }
            _ => { // Explorer — a house's standing + a distant venture
                if has_house { self.houses[resident as usize].prestige += 0.06; }
                format!("{} sets out from {} to chart distant shores.", name, city)
            }
        };
        self.journal.push(JournalEntry {
            tick, kind: "figure".into(), hub: hub as i32, good, value: 0.0, text,
        });
        if has_house {
            self.houses[resident as usize].events.push(HouseEvent {
                tick, kind: "figure".into(),
                text: format!("{} {} brings the family renown.", role_title(kind), name),
            });
        }
        let span = ((15.0 + hash01(self.seed, tick as u64 ^ 0xCA6, hub as u64) * 25.0)
            * TICKS_PER_YEAR as f32) as u32;
        self.figures.push(Figure {
            name, kind, hub: hub as u32, house: resident, good,
            born_tick: tick, dies_tick: tick + span, dead: false,
        });
    }


    /// Phase 5 (flavour) · CIVIC WONDERS: a prosperous city occasionally raises a
    /// monument (lighthouse → market hall → cathedral) for prestige & stability.
    pub(crate) fn run_civic_wonders(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0x0A0E, 0) >= WONDER_YEARLY_CHANCE { return; }
        let n = self.hubs.len();
        // The most prosperous real city that still has a wonder tier to build.
        let mut best = usize::MAX; let mut best_score = -1.0f32;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let built = self.wonders.iter().filter(|w| w.0 == h as u32).count();
            if built >= WONDER_NAMES.len() { continue; }
            let score = self.hubs[h].population.max(0.0)
                * self.hubs[h].sent_prosperity.clamp(0.0, 1.0);
            if score > best_score { best_score = score; best = h; }
        }
        if best == usize::MAX { return; }
        let tier = self.wonders.iter().filter(|w| w.0 == best as u32).count() as u8;
        self.wonders.push((best as u32, tier));
        self.hubs[best].sent_stability = (self.hubs[best].sent_stability + 0.06).min(1.0);
        let city = self.hubs[best].name.clone();
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "wonder".into(), hub: best as i32, good: -1, value: 0.0,
            text: format!("{} completes {} — a wonder of the age.", city, WONDER_NAMES[tier.min(2) as usize]),
        });
    }


    /// Phase 5 (flavour) · PIRACY: corsairs seize a merchant house's galley, costing
    /// it one sea-fleet asset (floored at 0). Bounded; a chronicle beat.
    pub(crate) fn run_piracy(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0x9A17, 0) >= PIRACY_YEARLY_CHANCE { return; }
        let cand: Vec<usize> = (0..self.houses.len())
            .filter(|&i| !self.houses[i].defunct && self.houses[i].fleet_sea > 0).collect();
        if cand.is_empty() { return; }
        let hi = cand[((hash01(self.seed, yr as u64, 0x9A2) * cand.len() as f32) as usize) % cand.len()];
        self.houses[hi].fleet_sea = self.houses[hi].fleet_sea.saturating_sub(1);
        let (hn, hub) = (self.houses[hi].name.clone(), self.houses[hi].hub as i32);
        let city = self.hubs.get(hub as usize).map(|h| h.name.clone()).unwrap_or_default();
        self.houses[hi].events.push(HouseEvent {
            tick: self.tick, kind: "piracy".into(), text: format!("Corsairs seize one of our galleys off {}.", city) });
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "piracy".into(), hub, good: -1, value: 0.0,
            text: format!("Corsairs seize a {} galley off {}.", hn, city),
        });
    }


    /// Phase 5 (flavour) · seed CRAFT GUILDS once — the strongest manufacturing city
    /// for each manufactured (recipe) good gets a guild of its masters. Deterministic;
    /// capped at `GUILD_MAX`.
    pub(crate) fn seed_craft_guilds(&mut self) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let mut guilds: Vec<CraftGuild> = Vec::new();
        for g in 0..ng {
            if self.goods[g].inputs.is_empty() { continue; } // only manufactured goods
            // The city that makes the most of this good.
            let mut best_hub = usize::MAX; let mut best = 0.0f32;
            for h in 0..n {
                if self.hubs[h].is_estate { continue; }
                let p = self.hubs[h].production.get(g).copied().unwrap_or(0.0);
                if p > best { best = p; best_hub = h; }
            }
            if best_hub != usize::MAX && best > 0.0 {
                guilds.push(CraftGuild { hub: best_hub as u32, good: g as u32, strength: 0.3, hall: false });
            }
        }
        // Keep the strongest guilds (by their host city's output) if we overflow.
        guilds.sort_by(|a, b| {
            let pa = self.hubs[a.hub as usize].production.get(a.good as usize).copied().unwrap_or(0.0);
            let pb = self.hubs[b.hub as usize].production.get(b.good as usize).copied().unwrap_or(0.0);
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        });
        guilds.truncate(GUILD_MAX);
        guilds.sort_by_key(|gd| (gd.hub, gd.good)); // stable order
        self.guilds = guilds;
    }


    /// Phase 5 (flavour) · run the CRAFT GUILDS yearly: lift their good's local
    /// quality, occasionally strike (a short manufacture dent), and raise a guildhall
    /// once the guild is strong. Bounded quality/stability; chronicle beats.
    pub(crate) fn run_craft_guilds(&mut self, yr: u32) {
        let ng = self.goods.len();
        for gi in 0..self.guilds.len() {
            let (hub, good) = (self.guilds[gi].hub as usize, self.guilds[gi].good as usize);
            if hub >= self.hubs.len() || good >= ng { continue; }
            // Master the craft: a steady, capped quality lift.
            if self.hubs[hub].quality.len() == ng {
                let q = self.hubs[hub].quality[good];
                self.hubs[hub].quality[good] = (q + GUILD_QUALITY_STEP).min(GUILD_QUALITY_CAP);
            }
            self.guilds[gi].strength = (self.guilds[gi].strength + 0.04).min(1.0);
            // Raise a guildhall once the guild is well-established (one-time monument).
            if !self.guilds[gi].hall && self.guilds[gi].strength >= GUILD_HALL_STRENGTH {
                self.guilds[gi].hall = true;
                self.hubs[hub].sent_stability = (self.hubs[hub].sent_stability + 0.05).min(1.0);
                let (city, gn) = (self.hubs[hub].name.clone(), self.goods[good].name.clone());
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "guildhall".into(), hub: hub as i32, good: good as i32,
                    value: 0.0, text: format!("The {} guild of {} raises a grand guildhall.", gn, city),
                });
            }
            // A strike: the masters down tools, halting the craft for a spell.
            if hash01(self.seed, yr as u64 ^ 0x6111D, gi as u64) < GUILD_STRIKE_CHANCE {
                let dur = 20 + (hash01(self.seed, yr as u64, gi as u64) * 40.0) as u32;
                self.active_events.push(ActiveEvent {
                    kind: "guild_strike".into(), hub: hub as i32, good: good as i32,
                    magnitude: GUILD_STRIKE_MAG, until_tick: self.tick + dur,
                });
                let (city, gn) = (self.hubs[hub].name.clone(), self.goods[good].name.clone());
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "guild_strike".into(), hub: hub as i32, good: good as i32,
                    value: dur as f32, text: format!("The {} guild of {} downs tools in a strike.", gn, city),
                });
            }
        }
    }


    /// Phase 5 (flavour) · dynastic MARRIAGES between houses. Once a year a prominent
    /// house may wed another — ending any feud between them, sealing an alliance, and
    /// exchanging a capped dowry. A broken match rekindles the feud. Deterministic;
    /// wealth only moves between houses so the economy stays bounded.
    /// Do two houses' MERCHANTS reach each other? True if they trade in a shared city,
    /// or any of one house's network nodes (home + offices) lies within a merchant's
    /// practical reach of any of the other's (a real trade route exists — finite travel
    /// days — AND the two sit within `MARRIAGE_REACH_KM`). This replaces any crude
    /// same-continent test: contact is grounded in actual commerce.
    pub(crate) fn houses_in_contact(&self, a: usize, b: usize) -> bool {
        let n = self.hubs.len();
        if n == 0 || self.days.len() != n * n { return false; }
        if a >= self.houses.len() || b >= self.houses.len() { return false; }
        let reach_cells = MARRIAGE_REACH_KM * self.world_w / EARTH_EQUATOR_KM;
        let nodes = |h: usize| -> Vec<usize> {
            let mut v = vec![self.houses[h].hub as usize];
            for &off in &self.houses[h].offices { v.push(off as usize); }
            v
        };
        let (na, nb) = (nodes(a), nodes(b));
        for &x in &na {
            if x >= n { continue; }
            for &y in &nb {
                if y >= n { continue; }
                if x == y { return true; } // both trade the same city → certain contact
                if self.days[x * n + y].is_finite() && self.hub_cell_dist(x, y) <= reach_cells {
                    return true;
                }
            }
        }
        false
    }


    pub(crate) fn arrange_marriages(&mut self, yr: u32) {
        let nh = self.houses.len();
        if nh < 2 { return; }
        // ── A match sours: dissolve an alliance back into feud (rare). ──
        let mut kept: Vec<(u32, u32)> = Vec::with_capacity(self.alliances.len());
        for (a, b) in self.alliances.clone() {
            let (ua, ub) = (a as usize, b as usize);
            if ua >= nh || ub >= nh { continue; }
            let broke = hash01(self.seed, (yr as u64) ^ ((a as u64) << 20) ^ (b as u64), 0xF00D)
                < MARRIAGE_BREAK_CHANCE;
            if broke && !self.houses[ua].defunct && !self.houses[ub].defunct {
                // A soured match is its own kind of feud, and a hot one: it starts well
                // above the temperature a mere trade rivalry does.
                let (_, good, hub) = self.feud_overlap(ua, ub);
                self.open_feud(ua, ub, FEUD_MARRIAGE, good, hub, 0.45);
                let (na, nb) = (self.houses[ua].name.clone(), self.houses[ub].name.clone());
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "feud".into(), hub: self.houses[ua].hub as i32,
                    good: -1, value: 0.0,
                    text: format!("The alliance of {} and {} collapses into feud.", na, nb),
                });
            } else {
                kept.push((a, b));
            }
        }
        self.alliances = kept;
        // ── Arrange one new marriage this year (deterministic). ──
        if hash01(self.seed, yr as u64 ^ 0x1EDD, 0) >= MARRIAGE_YEARLY_CHANCE { return; }
        let mut cand: Vec<usize> = (0..nh).filter(|&i| {
            let h = &self.houses[i];
            !h.defunct && !h.is_guild && h.wealth > MARRIAGE_MIN_WEALTH
        }).collect();
        if cand.len() < 2 { return; }
        cand.sort_by(|&a, &b| self.houses[b].wealth
            .partial_cmp(&self.houses[a].wealth).unwrap_or(std::cmp::Ordering::Equal));
        let top = cand.len().min(12);
        let a = cand[((hash01(self.seed, yr as u64, 0x0A) * top as f32) as usize) % top];
        // A marriage only forms with a house whose MERCHANTS ACTUALLY REACH this one —
        // they trade in a shared city, or their networks lie within a caravan/voyage of
        // each other. No trade contact ⇒ no match (grounded in commerce, not geography).
        let reachable: Vec<usize> = cand.iter().copied()
            .filter(|&x| x != a && self.houses_in_contact(a, x)).collect();
        if reachable.is_empty() { return; } // this year's suitor's traders reach no peer
        let b = reachable[((hash01(self.seed, yr as u64, 0x0B) * reachable.len() as f32) as usize)
            % reachable.len()];
        if a == b { return; }
        let (lo, hi) = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        if self.alliances.contains(&(lo, hi)) { return; } // already wed
        // End any feud, seal the alliance. A match is the classic way two merchant
        // families ended a quarrel, so it is recorded as the feud's OUTCOME rather than
        // just quietly dropping the rival entries.
        if let Some(fi) = self.feud_between(a, b) { self.close_feud(fi, FEUD_WED); }
        self.houses[a].rivals.retain(|&r| r != b);
        self.houses[b].rivals.retain(|&r| r != a);
        self.alliances.push((lo, hi));
        // Dowry: a capped transfer from the wealthier to the other + prestige both.
        let (rich, poor) = if self.houses[a].wealth >= self.houses[b].wealth { (a, b) } else { (b, a) };
        let dowry = (self.houses[rich].wealth * MARRIAGE_DOWRY_FRAC).min(MARRIAGE_DOWRY_CAP).max(0.0);
        self.houses[rich].wealth -= dowry;
        self.houses[poor].wealth += dowry;
        self.houses[a].prestige += 0.05;
        self.houses[b].prestige += 0.05;
        let (na, nb) = (self.houses[a].name.clone(), self.houses[b].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "marriage".into(), hub: self.houses[a].hub as i32, good: -1,
            value: 0.0, text: format!("{} and {} are joined in marriage, sealing an alliance.", na, nb),
        });
        self.houses[a].events.push(HouseEvent {
            tick: self.tick, kind: "marriage".into(), text: format!("Wed into {} — a new alliance.", nb) });
        self.houses[b].events.push(HouseEvent {
            tick: self.tick, kind: "marriage".into(), text: format!("Wed into {} — a new alliance.", na) });
    }


    /// Phase 4 (flavour) · seed one seasonal TRADE FAIR per large trading component,
    /// at its crossroads market town (prominence + a mild inland bias — the historic
    /// Champagne/Leipzig pattern). Deterministic; runs once (before routes are built,
    /// so it scores by population rather than live connectivity).
    pub(crate) fn seed_trade_fairs(&mut self) {
        use std::collections::HashMap;
        let n = self.hubs.len();
        let mut best: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut count: HashMap<u32, u32> = HashMap::new();
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let comp = self.hubs[h].component;
            *count.entry(comp).or_insert(0) += 1;
            let inland = if self.hubs[h].coastal { 1.0 } else { 1.25 };
            let score = self.hubs[h].population.max(0.0) * inland;
            let e = best.entry(comp).or_insert((h, -1.0));
            if score > e.1 { *e = (h, score); }
        }
        let mut fairs: Vec<Fair> = Vec::new();
        for (comp, (hub, _)) in best {
            if count.get(&comp).copied().unwrap_or(0) < FAIR_MIN_COMPONENT_HUBS { continue; }
            // Spring (month 4) or autumn (month 9) opening, by a deterministic roll.
            let month = if hash01(self.seed, hub as u64, 0xFA12) < 0.5 { 4 } else { 9 };
            fairs.push(Fair { hub: hub as u32, month });
        }
        fairs.sort_by_key(|f| f.hub); // HashMap order isn't stable → sort for determinism
        self.fairs = fairs;
    }


    /// Phase 4 (flavour) · open the fairs whose month begins today: a civic boon, a
    /// burst of converging trade on the nearest lanes (overlay-only), a chronicle
    /// beat, and an `ActiveEvent{kind:"fair"}` while the season lasts.
    pub(crate) fn run_trade_fairs(&mut self, doy: u32) {
        if self.fairs.is_empty() { return; }
        let tick = self.tick;
        let month_len = TICKS_PER_YEAR / 12;
        let opening: Vec<u32> = self.fairs.iter()
            .filter(|f| (f.month.saturating_sub(1) as u32) * month_len == doy)
            .map(|f| f.hub).collect();
        for hub in opening {
            let h = hub as usize;
            if h >= self.hubs.len() { continue; }
            self.hubs[h].sent_prosperity = (self.hubs[h].sent_prosperity + FAIR_PROSPERITY).min(1.0);
            self.hubs[h].sent_stability = (self.hubs[h].sent_stability + FAIR_STABILITY).min(1.0);
            let nbrs: Vec<usize> = self.neighbors.get(h)
                .map(|v| v.iter().take(FAIR_LANES).map(|&x| x as usize).collect())
                .unwrap_or_default();
            for nb in nbrs { self.accrue_flow(nb, h, usize::MAX, FAIR_FLOW); }
            self.active_events.push(ActiveEvent {
                kind: "fair".into(), hub: hub as i32, good: -1,
                magnitude: 1.0, until_tick: tick + month_len,
            });
            let city = self.hubs[h].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "fair".into(), hub: hub as i32, good: -1, value: 0.0,
                text: format!("The Fair of {} opens; merchants converge from across the region.", city),
            });
        }
    }


    /// Phase 4 (flavour) · seed one HOLY CITY per large component, at a prominent
    /// settlement distinct (where possible) from its fair town, with a patron ritual
    /// good drawn from the world's goods. Deterministic; runs once. A temple beat is
    /// chronicled at founding.
    pub(crate) fn seed_holy_sites(&mut self) {
        use std::collections::{HashMap, HashSet};
        let n = self.hubs.len();
        // Ritual goods actually present in this world (by name → index).
        let present: Vec<usize> = RITUAL_GOODS.iter()
            .filter_map(|name| self.goods.iter().position(|g| g.name == *name))
            .collect();
        let fair_hubs: HashSet<u32> = self.fairs.iter().map(|f| f.hub).collect();
        // Per component, the most prominent real hub that is NOT already a fair town
        // (fall back to allowing a fair town if that's all there is).
        let mut best: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut best_any: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut count: HashMap<u32, u32> = HashMap::new();
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let comp = self.hubs[h].component;
            *count.entry(comp).or_insert(0) += 1;
            let score = self.hubs[h].population.max(0.0);
            let a = best_any.entry(comp).or_insert((h, -1.0));
            if score > a.1 { *a = (h, score); }
            if !fair_hubs.contains(&(h as u32)) {
                let e = best.entry(comp).or_insert((h, -1.0));
                if score > e.1 { *e = (h, score); }
            }
        }
        let mut sites: Vec<HolySite> = Vec::new();
        let mut comps: Vec<u32> = count.keys().copied().collect();
        comps.sort_unstable();
        for comp in comps {
            if count.get(&comp).copied().unwrap_or(0) < HOLY_MIN_COMPONENT_HUBS { continue; }
            let hub = best.get(&comp).or_else(|| best_any.get(&comp)).map(|&(h, _)| h);
            let hub = match hub { Some(h) => h, None => continue };
            let patron_good = if present.is_empty() { -1 } else {
                let pick = (hash01(self.seed, hub as u64, 0x40FE) * present.len() as f32) as usize;
                present[pick.min(present.len() - 1)] as i32
            };
            let tier = if hash01(self.seed, hub as u64, 0x7E3) < 0.4 { 2 } else { 1 };
            let month = 1 + (hash01(self.seed, hub as u64, 0xB105) * 12.0) as u8;
            sites.push(HolySite { hub: hub as u32, patron_good, tier, month: month.min(12).max(1) });
            let city = self.hubs[hub].name.clone();
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "temple".into(), hub: hub as i32, good: patron_good, value: 0.0,
                text: format!("The great temple of {} is renowned across the land.", city),
            });
        }
        self.holy_sites = sites;
    }


    /// Phase 4 (flavour) · open any PILGRIMAGE season beginning today: a civic boon,
    /// inbound pilgrim traffic (overlay-only), a transient demand spike for the patron
    /// ritual good (self-relaxes), an `ActiveEvent{kind:"pilgrimage"}`, and a beat.
    pub(crate) fn run_pilgrimages(&mut self, doy: u32) {
        if self.holy_sites.is_empty() { return; }
        let tick = self.tick;
        let month_len = TICKS_PER_YEAR / 12;
        let opening: Vec<(u32, i32)> = self.holy_sites.iter()
            .filter(|s| (s.month.saturating_sub(1) as u32) * month_len == doy)
            .map(|s| (s.hub, s.patron_good)).collect();
        for (hub, patron) in opening {
            let h = hub as usize;
            if h >= self.hubs.len() { continue; }
            self.hubs[h].sent_prosperity = (self.hubs[h].sent_prosperity + PILGRIM_PROSPERITY).min(1.0);
            self.hubs[h].sent_stability = (self.hubs[h].sent_stability + PILGRIM_STABILITY).min(1.0);
            let nbrs: Vec<usize> = self.neighbors.get(h)
                .map(|v| v.iter().take(PILGRIM_LANES).map(|&x| x as usize).collect())
                .unwrap_or_default();
            for nb in nbrs { self.accrue_flow(nb, h, usize::MAX, PILGRIM_FLOW); }
            // Pilgrim demand bids up the patron ritual good (transient — the price
            // relax pulls it back over the following days). Capped for safety.
            if patron >= 0 {
                let g = patron as usize;
                if g < self.hubs[h].price.len() && g < self.goods.len() {
                    let cap = self.goods[g].base_value.max(0.01) * 5.0;
                    self.hubs[h].price[g] = (self.hubs[h].price[g] * PILGRIM_PRICE_BUMP).min(cap);
                }
            }
            self.active_events.push(ActiveEvent {
                kind: "pilgrimage".into(), hub: hub as i32, good: patron,
                magnitude: 1.0, until_tick: tick + month_len,
            });
            let city = self.hubs[h].name.clone();
            let good_txt = if patron >= 0 {
                self.goods.get(patron as usize).map(|g| format!(" — {} for the altar", g.name)).unwrap_or_default()
            } else { String::new() };
            self.journal.push(JournalEntry {
                tick, kind: "pilgrimage".into(), hub: hub as i32, good: patron, value: 0.0,
                text: format!("Pilgrims throng to {} for the holy season{}.", city, good_txt),
            });
        }
    }


    /// Monthly: found guilds for cities that have grown past the threshold, pay the
    /// civic subsidy into guild treasuries, and open/close offices for every holder.
    pub(crate) fn update_guilds_and_offices(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        // 1) A prospering town charters a civic GUILD (from year 5). Lower population
        //    bar than the initial-seed GUILD_MIN_POP so guilds are the common early
        //    institution — but gated on PROSPERITY so they cluster in commercially
        //    successful cities (poor backwaters stay guild-less → the world stays
        //    differentiated, and there are far more than a handful of guilds).
        if self.tick >= GUILD_START_YEAR * TICKS_PER_YEAR {
            for h in 0..n {
                if self.hubs[h].is_estate { continue; }
                if self.hubs[h].population < GUILD_FORM_POP { continue; }
                if self.hubs[h].sent_prosperity < GUILD_FORM_PROSPERITY { continue; }
                let has_guild = self.houses.iter()
                    .any(|g| !g.defunct && g.is_guild && g.hub as usize == h);
                if !has_guild {
                    self.found_guild(h);
                }
            }
        }
        // 2) Civic subsidy: the home city funds its guild (scaled by size + prosperity).
        for gi in 0..self.houses.len() {
            if self.houses[gi].defunct || !self.houses[gi].is_guild { continue; }
            let hub = self.houses[gi].hub as usize;
            if hub >= n { continue; }
            let pop = self.hubs[hub].population.max(0.0);
            let prosp = self.hubs[hub].sent_prosperity.clamp(0.1, 1.0);
            self.houses[gi].wealth += (pop / 1000.0) * GUILD_SUBSIDY_PER_1K * prosp;
        }
        // 3) Open / close offices for every active holder.
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub as usize;
            // Expired leases lapse; surviving leases each cost a monthly rent paid to
            // the host city (the durable-base running cost).
            self.houses[hi].office_leases.retain(|&(_, until)| until > tick);
            let leases = self.houses[hi].office_leases.clone();
            for &(lh, _) in &leases {
                let rent = OFFICE_LEASE_RENT * self.city_size_factor(lh as usize);
                self.houses[hi].wealth -= rent;
                if (lh as usize) < n { self.hubs[lh as usize].civic_pool += rent; }
            }
            // Strongest partner volume (scale-invariant trigger).
            let max_vol = self.houses[hi].trade_at.iter().map(|(_, v)| *v).fold(0.0f32, f32::max);
            // CLOSE: an office whose tie has withered, or a (private house) gone broke.
            let close_floor = (max_vol * 0.1).max(OFFICE_CLOSE_VOLUME);
            let broke = !self.houses[hi].is_guild && self.houses[hi].wealth < HOUSE_BANKRUPT;
            let offices = self.houses[hi].offices.clone();
            for &ohub in &offices {
                // A LEASED office, or one a live contract relies on, never auto-closes —
                // the network base is guaranteed for the contract's life.
                if self.office_leased(hi, ohub) || self.backs_active_contract(hi, ohub) { continue; }
                let vol = self.houses[hi].trade_at.iter()
                    .find(|(hb, _)| *hb == ohub).map(|(_, v)| *v).unwrap_or(0.0);
                // Phase 2.5 · POACHING — a HIRED (unposted) office's steward is able,
                // and can be lured away. A posted kin is loyal to the family and is
                // never poached; a guild has no stewards to hire at all; NO ROSTER
                // means no known steward to poach (same "no roster ⇒ bit-identical"
                // reasoning as the wage/skim gate above).
                let hired = !self.houses[hi].is_guild && !self.houses[hi].kin.is_empty()
                    && !self.houses[hi].kin.iter().any(|k| k.role == 2 && k.posted == ohub as i32);
                let poached = hired
                    && hash01(self.seed, tick as u64 ^ 0xF0EA, ohub as u64 ^ (hi as u64) << 24) < STEWARD_POACH_CHANCE;
                if broke || vol < close_floor {
                    self.houses[hi].offices.retain(|&x| x != ohub);
                    let cn = self.houses[hi].name.clone();
                    let city = self.hubs.get(ohub as usize).map(|x| x.name.clone()).unwrap_or_default();
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "office_closed".into(),
                        text: format!("{} abandons its office in {}", cn, city),
                    });
                    self.journal.push(JournalEntry {
                        tick, kind: "office_closed".into(), hub: ohub as i32, good: -1, value: 0.0,
                        text: format!("{} closes its office in {}", cn, city),
                    });
                } else if poached {
                    self.houses[hi].offices.retain(|&x| x != ohub);
                    let cn = self.houses[hi].name.clone();
                    let city = self.hubs.get(ohub as usize).map(|x| x.name.clone()).unwrap_or_default();
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "poached".into(),
                        text: format!("{}'s steward in {} is lured away by a rival", cn, city),
                    });
                    self.journal.push(JournalEntry {
                        tick, kind: "poached".into(), hub: ohub as i32, good: -1, value: 0.0,
                        text: format!("{} loses its office in {} to a poached steward", cn, city),
                    });
                }
            }
            // OPEN: the strongest non-home partner with a real tie the holder can afford.
            if max_vol <= 0.0 { continue; }
            let mut cand: Option<(usize, f32)> = None;
            for &(hb, v) in &self.houses[hi].trade_at {
                let hb = hb as usize;
                if hb == home || hb >= n { continue; }
                if self.houses[hi].offices.contains(&(hb as u32)) { continue; }
                // Relaxed the relative gate 0.5→0.3 so a house also plants offices at
                // its SECOND-tier partners, not only its single dominant one — this
                // spreads several competing houses' counting-houses across each city
                // (was: one house monopolising a settlement).
                if v < OFFICE_OPEN_VOLUME || v < max_vol * 0.3 { continue; }
                if cand.map_or(true, |(_, bv)| v > bv) { cand = Some((hb, v)); }
            }
            if let Some((hb, _)) = cand {
                // Cost scales with the host city's importance (population).
                let cost = OFFICE_COST_BASE * (1.0 + self.hubs[hb].population / 50_000.0);
                // Phase 2.4 · an EXPANSIVE head opens a foothold on a thinner cushion, a
                // ROOTED one wants a fatter one first — axis 3, ±15% capped.
                let afford_mult = 1.5 * self.head_character_factor(hi, 3).recip();
                if self.houses[hi].wealth >= cost * afford_mult {
                    self.houses[hi].wealth -= cost;
                    self.houses[hi].offices.push(hb as u32);
                    let cn = self.houses[hi].name.clone();
                    let city = self.hubs[hb].name.clone();
                    let verb = if self.houses[hi].is_guild { "establishes a factory" } else { "opens a counting-house" };
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "branch".into(),
                        text: format!("{} {} in {}", cn, verb, city),
                    });
                    self.journal.push(JournalEntry {
                        tick, kind: "office".into(), hub: hb as i32, good: -1, value: 0.0,
                        text: format!("{} {} in {}", cn, verb, city),
                    });
                }
            }
        }
    }


    /// Seed civic guilds for every city already at/above the population threshold
    /// when the campaign begins (more emerge later as cities grow — see
    /// `update_guilds_and_offices`).
    pub fn seed_initial_guilds(&mut self) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].population >= GUILD_MIN_POP {
                self.found_guild(h);
            }
        }
    }


    /// Found a civic Merchant Guild for city `h` (≥ GUILD_MIN_POP). Distinct name,
    /// a starting treasury and fleet sized to the city; acts in the city's interest.
    pub(crate) fn found_guild(&mut self, h: usize) {
        let tick = self.tick;
        // A guildmaster holds an OFFICE, not a patrimony: the same tenure roll, but the
        // guild itself never divides (see `divide_estate`).
        let (guild_age, guild_term) = self.roll_founder_tenure(h as u64 ^ 0x6111);
        let coastal = self.hubs[h].coastal;
        let pop = self.hubs[h].population.max(1.0);
        let name = self.guild_name_for(h);
        let (fleet_sea, fleet_river, fleet_caravan) = Self::initial_fleet(coastal, true);
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("Chartered by the merchants of {}", self.hubs[h].name),
        };
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: h as i32, good: -1, value: 0.0,
            text: format!("{} is chartered in {}", name, self.hubs[h].name),
        });
        self.houses.push(House {
            name, hub: h as u32, wealth: (pop / 1000.0).max(1.0), prestige: 0.2,
            spec: vec![], monopoly: vec![], rivals: vec![], generation: 1,
            events: vec![founded], good_profit: Vec::new(), good_volume: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: 0.0, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: format!("Guildmaster of {}", self.hubs[h].name),
            head_since: tick, head_lifespan: guild_term,
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: ARCH_SPECIALTY, charters: Vec::new(),
            is_guild: true, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
            head_female: false, head_age: guild_age, line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0, origin_house: -1, origin_kind: ORIGIN_NONE, crowned: false, realm: -1,
        });
        let ni = self.houses.len() - 1;
        self.found_head_record(ni, "founder");
    }


    /// A distinct guild name for a city, styled by its CULTURE (e.g. "Collegium of
    /// Aquentia (wine)", "Suq of Madinah", "Hang of Linzhou") — so a guild reads in
    /// its home people's idiom and tags the city's chief trade.
    pub(crate) fn guild_name_for(&self, h: usize) -> String {
        let city = self.hubs[h].name.clone();
        let (x, y) = (self.hubs[h].x.max(0.0) as u32, self.hubs[h].y.max(0.0) as u32);
        // The city's strongest-produced good flavours the guild ("of wine").
        let specialty = {
            let mut bg = (usize::MAX, 0.0f32);
            for g in 0..self.goods.len() {
                let p = self.hubs[h].production.get(g).copied().unwrap_or(0.0);
                if p > bg.1 { bg = (g, p); }
            }
            if bg.0 != usize::MAX { self.goods.get(bg.0).map(|x| x.name.clone()) } else { None }
        };
        crate::sim::names::gen_guild_name(
            x, y, self.world_w as u32, self.world_h(), &city, specialty.as_deref(), h as u64 ^ 0x6111)
    }


    pub(crate) fn update_house_dynamics(&mut self, needs: &[Vec<f32>]) {
        let tick = self.tick;
        // Decay recent-volume so monopoly tracks the last while, not all history.
        // Per-hub trade ties decay more slowly (an office relationship is built and
        // lost over months, not days).
        for hh in &mut self.houses {
            if !hh.defunct {
                hh.volume *= 0.98;
                for e in &mut hh.trade_at { e.1 *= 0.997; }
                hh.trade_at.retain(|(_, v)| *v > 0.01);
            }
        }
        // Succession: the head dies at the end of their lifespan; an heir takes over.
        for hi in 0..self.houses.len() {
            // R2 · a crowned house's `head_lifespan` is a leftover from BEFORE the
            // coronation and must never fire `succeed_house` here — that would
            // rewrite `head_name`/`kin`/`line` out from under the realm's OWN
            // genealogy (`Realm.family`/`ruler`, `realm_family_pass`), the same
            // identity-corruption trap §5.1 names for `dissolve_house` and
            // `GOAL_OUTLAST_RIVAL`, a fourth path into it.
            if !self.houses[hi].is_merchant() { continue; }
            let h = &self.houses[hi];
            if h.head_lifespan > 0 && tick.saturating_sub(h.head_since) >= h.head_lifespan {
                self.succeed_house(hi);
            }
        }
        // Per tick: a new house may appear (probabilistic — see maybe_found_house).
        self.maybe_found_house();
        // Monthly: monopolies, political power, branching, extinction, feuds.
        if tick % 30 == 0 {
            // Merchant bankers earn a modest return on LIQUID capital — but only up
            // to a cap, so it's a starting perk, not an exponential engine. Real
            // banking growth now comes from OWNING a bank (bank_pass dividends).
            // DLC 3.5 rebalance: the old uncapped 1%/mo on the whole fortune
            // compounded houses to absurd wealth (100k in a decade).
            for hh in &mut self.houses {
                if !hh.defunct && hh.archetype == ARCH_BANKING && hh.wealth > 0.0 {
                    hh.wealth += hh.wealth.min(BANK_INTEREST_CAP) * BANK_INTEREST;
                }
            }
            // Phase 2: ensure/stock/expand house warehouses BEFORE upkeep, so the
            // capacity-scaled upkeep below sees each house's current depots.
            self.sync_and_stock_warehouses(needs);
            // Phase G: wealth bleeds (upkeep + consumption) so it plateaus and some
            // flows to the people — runs right after interest so it offsets it.
            self.apply_wealth_sinks();
            self.pay_to_regain_markets();
            self.recompute_monopolies_and_power();
            // Phase 1.1 · reads political_power/monopoly/dominant_seat just refreshed
            // above, so it must run after them.
            self.assign_house_tiers();
            // §3.2 · city tiers, right after house tiers — reads each city's ruling
            // house's freshly-updated `standing` as one of its four axes.
            self.assign_city_tiers();
            self.manage_fleets();
            self.update_structures();
            self.fund_public_works();
            // DLC 3.5 · wealthy poleis spend a slice of treasury sponsoring migration
            // (relieves crowding, drains hoarded treasuries).
            self.poleis_sponsor_migration();
            if ENABLE_CADET_BRANCHES { self.maybe_branch_houses(); } // disabled (user rule)
            self.maybe_house_invests();
            // DLC 3.5 · resale market: distressed houses / thin-treasury poleis sell
            // holdings; solvent houses & banks buy them.
            self.estate_resale_pass();
            // 4.9 (D7/D8) · out-of-town acquisition: dispatch, then resolve any
            // envoy that has arrived. Both read the same for-sale picture
            // `estate_resale_pass` just left, so run right after it.
            self.envoy_dispatch_pass();
            self.envoy_travel_pass();
            // 4.8 (D1, D5) · offtake-payout shares (extraction works only) draw
            // their physical cut of accumulated stock into the holder's own
            // depot, finest grade first — right after the two passes above,
            // since an envoy's PARTIAL outcome is currently the only source of
            // an offtake row.
            self.offtake_delivery_pass();
            // 4.12 (A2) · adulteration (`certification::adulteration_pass`) is
            // implemented but deliberately NOT called here — see that
            // function's own doc comment. Its trigger is gated on estate-
            // owner DISTRESS, a condition that differs structurally between
            // inheritance regimes by construction (that is what `econ_
            // inheritance_rules_fragment_differently` measures), so unlike
            // the certification fee above it isn't a case dose-tuning can
            // fix — deferred per the same §2.4 discipline 4.7's D11/A9 piece
            // already used, not silently dropped.
            self.update_guilds_and_offices();
            // Offices (re)settled → update commercial influence, dominance & Bailos.
            self.update_influence_and_bailos();
            // Offices are (re)settled above → now offer futures contracts from them.
            self.form_contracts(needs);
            // DLC 3.5 · banks service loans, take deposits, lend, and may fail.
            self.bank_pass();
            // DLC 4 · manufactories refine their craft (quality climbs toward a cap).
            self.update_good_quality();
            // Local line's debt-based solvency check supersedes the old volume-based
            // dissolve (a house in the red ≥1yr goes bankrupt; guilds get bailouts).
            self.update_solvency();
            // Feuds: heat/cool each live quarrel by how much the two houses still get
            // in each other's way, re-derive its stage, and let it flare. Runs over the
            // BOUNDED feud list, so unlike formation (below) it is not O(houses²).
            self.update_feuds();
            // Phase 4.4 · the foreign hand: a posted kin exposed to a rival's
            // commercial footprint has their loyalty nudged down. Runs BEFORE the
            // crisis/schism passes below so this month's pressure is already
            // reflected in their discontent/tension reads, not a tick behind.
            self.apply_foreign_hand();
            // Phase 3.3-3.6 · open/progress/resolve succession crises. Quarterly rounds
            // are gated inside on `(tick - opened_tick) % CRISIS_ROUND_TICKS`, so this
            // monthly call is a no-op for a house mid-crisis except on a quarter
            // boundary; opening a NEW crisis is checked here directly (monthly).
            self.update_house_crises();
            // Phase 4.1 · a house under enough internal tension quarrels or, rarely,
            // loses a posted kin to Departure — the same monthly cadence as crises.
            self.update_house_schisms();
        }
        // Feud FORMATION keeps the old half-yearly cadence — it is the O(n²) pair scan.
        if tick % 180 == 0 { self.update_rivalries(); }
    }


    /// Heir succession: new generation, a freshly-named head, a new lifespan, and
    /// occasionally the house splits a branch off into another city.
    ///
    /// Phase 0.4 · this is where the culture's LAW OF INHERITANCE is read. It decides
    /// three things, and each of them is the historical payoff of the rule:
    ///
    /// * **who** — the line rule sets the heir's sex (`sim::inheritance::heir_is_female`);
    /// * **how old** — an heir is not born on the day they inherit. An eldest son takes
    ///   over in his thirties, a Mongol *otchigin* youngest as a young man, an elected
    ///   tanist at sixty. Accession age sets the TENURE, so ultimogeniture gives long
    ///   reigns that open weak and seniority gives a churn of short ones;
    /// * **how much** — partible inheritance DIVIDES the estate among the heirs, which
    ///   is the fragmentation the whole rule exists to produce. Every other rule
    ///   concentrates, and concentration is simply the absence of a split.
    pub(crate) fn succeed_house(&mut self, hi: usize) {
        let tick = self.tick;
        let gen = self.houses[hi].generation + 1;
        let hub = self.houses[hi].hub as usize;
        let name = self.houses[hi].name.clone();
        let (line_rule, inh) = self.rules_for_hub(hub);
        // Close the outgoing head's record before the new one is written.
        self.close_head_record(hi);
        let mut female = crate::sim::inheritance::heir_is_female(
            line_rule, (hi as u64) << 8 ^ gen as u64, self.seed);
        // Phase 2.1 · the widow as a capable merchant
        // (`HOUSE_INHERITANCE_AND_TERRITORY.md` Part C.1). A purely agnatic line never
        // produces a female heir by descent — but in Italian and Hanseatic practice a
        // widow could, and often did, hold and run the firm herself. Independent of
        // whether a spouse is on the (still sparse) kin roster, since marriages aren't
        // tracked yet: this gives agnatic houses the same occasional female regency
        // every other line rule already has some route to.
        let widow_regent = line_rule == LineRule::Agnatic
            && hash01(self.seed, hi as u64 ^ gen as u64 ^ 0x715D, 0) < WIDOW_REGENCY_CHANCE;
        if widow_regent { female = true; }
        let heir = self.head_name_sexed_for(hub, &name, gen as u64 ^ 0x5151, female);
        let (age, lifespan) = self.roll_tenure(inh, hi as u64 ^ (gen as u64) << 16);
        // Archetype can PIVOT at a generational change to reflect what the family has
        // become: a rich lender (or one that owns a bank) turns merchant-banker; a big
        // shipper a fleet dynasty; a council power a political house; else a specialist.
        let new_arch = {
            let h = &self.houses[hi];
            let fleet = h.fleet_sea + h.fleet_river + h.fleet_caravan;
            let owns_bank = self.banks.iter().any(|b| !b.defunct && b.house == hi as u32);
            if owns_bank || h.wealth >= BANK_FOUND_WEALTH_RICH { ARCH_BANKING }
            else if fleet >= 12 { ARCH_FLEET }
            else if h.political_power >= 0.6 || h.dominant_seat { ARCH_POLITICAL }
            else { ARCH_SPECIALTY }
        };
        let old_arch = self.houses[hi].archetype;
        // How this head came in — the rule's own phrase, and (for the matrilineal case)
        // which of the two attested variants this house follows.
        let avuncular = inh == InheritanceRule::Matrilineal
            && crate::sim::inheritance::is_avunculate(hi as u64, self.seed);
        let accession = match inh {
            InheritanceRule::Ultimogeniture => "the hearth-keeper",
            InheritanceRule::Seniority => "eldest capable",
            InheritanceRule::Matrilineal if avuncular => "sister's son",
            InheritanceRule::Matrilineal => "daughter of the house",
            _ if widow_regent => "widow regent",
            _ if female => "daughter of the house",
            _ => "heir",
        };
        // A funeral is not an achievement. Standing is gained only where the LAST
        // generation actually built something, and only up to a ceiling — prestige
        // feeds political power → charters → monopolies → wealth (rule 18), and heads
        // now turn over two to three times a century rather than once.
        let grew = self.houses[hi].line.last().is_some_and(|p| p.wealth_end > p.wealth_start);
        {
            let h = &mut self.houses[hi];
            h.generation = gen;
            h.head_name = heir.clone();
            h.head_since = tick;
            h.head_lifespan = lifespan;
            h.head_female = female;
            h.head_age = age;
            if grew && h.prestige < SUCCESSION_PRESTIGE_CAP {
                h.prestige = (h.prestige + SUCCESSION_PRESTIGE).min(SUCCESSION_PRESTIGE_CAP);
            }
            h.archetype = new_arch;
            h.line.push(HouseHead {
                name: heir.clone(), female, generation: gen,
                since: tick, until: 0, age_at_accession: age, age_at_death: 0,
                wealth_start: h.wealth, wealth_end: 0.0,
                accession: accession.into(), epithet: String::new(),
            });
            h.events.push(HouseEvent {
                tick, kind: "succession".into(),
                text: match accession {
                    "heir" => format!("{} succeeds as head at {} (generation {})", heir, age, gen),
                    a => format!("{} succeeds as head at {} — {} (generation {})", heir, age, a, gen),
                },
            });
            if new_arch != old_arch {
                h.events.push(HouseEvent {
                    tick, kind: "archetype".into(),
                    text: format!("the family reinvents itself as {}", archetype_label(new_arch)),
                });
            }
        }
        self.journal.push(JournalEntry {
            tick, kind: "succession".into(), hub: hub as i32, good: -1,
            value: self.houses[hi].generation as f32,
            text: format!("{} succeeds as head of {}", heir, name),
        });
        self.ensure_kin_roster(hi);
        // ── The division ────────────────────────────────────────────────────────
        // Partible inheritance splits the capital at every generation, which is why
        // firms under it had to be RECONSTITUTED each time and why so many did not
        // survive it. Every other rule leaves the estate whole.
        if inh.divides() { self.divide_estate(hi, gen); }
        // A wealthy house founds a cadet BRANCH in a city it trades with. Lowered
        // to gen>=2 (was gen>=3 ≈ 150+ yrs, which essentially never happened in a
        // normal playthrough). Periodic branching also runs monthly (see
        // maybe_branch_houses), so expansion no longer depends solely on a death.
        if self.houses[hi].wealth > HOUSE_BRANCH_WEALTH && gen >= 2 {
            if let Some(dest) = self.pick_branch_hub(hub) {
                let parent = name.clone();
                self.found_branch(hi, dest, parent);
            }
        }
    }


    // ── Phase 1.1 · house tiers ──────────────────────────────────────────────────
    // `HOUSE_PEOPLE_AND_TIERS.md` §1: one standing score built entirely from state that
    // already exists, banded into a rank among LIVE peers (not an absolute number that
    // means nothing as the world grows), with hysteresis so a house sitting near a
    // boundary doesn't relabel every month.

    /// Assign every live private house its `tier` (1 great .. 4 marginal) and its
    /// `standing` score. Guilds are never tiered — a civic office is not a family
    /// competing for rank. Called monthly alongside `recompute_monopolies_and_power`,
    /// which by then has already refreshed `political_power`/`monopoly`/`dominant_seat`
    /// for this same tick.
    pub(crate) fn assign_house_tiers(&mut self) {
        let tick = self.tick;
        let live: Vec<usize> = (0..self.houses.len())
            // R1b · a crowned house has left the merchant world (`is_merchant`) —
            // it ranks on the realm ladder instead, never the tier 1-4 one.
            .filter(|&i| self.houses[i].is_merchant() && !self.houses[i].is_guild)
            .collect();
        let n = live.len();
        if n == 0 { return; }

        let wealths: Vec<f32> = live.iter().map(|&i| self.houses[i].wealth.max(0.0)).collect();
        let volumes: Vec<f32> = live.iter().map(|&i| self.houses[i].volume.max(0.0)).collect();
        let prestiges: Vec<f32> = live.iter().map(|&i| self.houses[i].prestige.max(0.0)).collect();
        let wr = rank_norm(&wealths);
        let vr = rank_norm(&volumes);
        let pr = rank_norm(&prestiges);
        // Phase 5 · a held province, precomputed once (not per-house — O(provinces)
        // total instead of O(houses·provinces)). Weighted 3× a bailo/charter/council
        // seat in the same `seats` term — "territory is the strongest tier input
        // there is" (`HOUSE_INHERITANCE_AND_TERRITORY.md` Part D) — rather than a new
        // top-level term, which would need its own weight recalibration across every
        // already-measured tier distribution.
        let mut territory = vec![0u32; self.houses.len()];
        for &h in &self.prov_holder_house { if h >= 0 { territory[h as usize] += 1; } }

        let mut standings = vec![0.0f32; n];
        for (k, &hi) in live.iter().enumerate() {
            // reach: commercial influence already caps itself at 1 by construction.
            let reach = self.houses[hi].influence.iter().map(|&(_, v)| v).sum::<f32>().clamp(0.0, 1.0);
            // seats: captured city councils + Bailo seats + city charters + provinces
            // held (each weighted 3×, per the doc comment above).
            let seats_raw = self.hubs.iter()
                .filter(|h| !h.is_estate && (h.council_house == hi as i32 || h.captor_house == hi as i32))
                .count()
                + self.houses[hi].bailos.len() + self.houses[hi].charters.len()
                + territory[hi] as usize * 3;
            let seats = (seats_raw as f32 / TIER_SEATS_SOFT_CAP).min(1.0);
            let s = 0.30 * wr[k] + 0.25 * vr[k] + 0.20 * reach + 0.15 * seats + 0.10 * pr[k];
            standings[k] = s.clamp(0.0, 1.0);
        }

        // Percentile position: 0 = the most prominent live house, 1 = the least.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| standings[b].partial_cmp(&standings[a]).unwrap_or(std::cmp::Ordering::Equal));
        let mut pct = vec![0.0f32; n];
        for (rank, &k) in order.iter().enumerate() {
            pct[k] = if n > 1 { rank as f32 / (n - 1) as f32 } else { 0.0 };
        }

        for k in 0..n {
            let hi = live[k];
            self.houses[hi].standing = standings[k];
            let prev = self.houses[hi].tier;
            let new_tier = if prev == 0 {
                // First assignment for this house: no hysteresis, no chronicle.
                Self::tier_band(pct[k], standings[k])
            } else {
                Self::tier_with_hysteresis(prev, pct[k], standings[k])
            };
            if new_tier != prev && prev != 0 {
                let name = self.houses[hi].name.clone();
                let (kind, text) = if new_tier < prev {
                    ("tier_up".to_string(),
                     format!("{} is now counted among the {} houses", name, TIER_NAMES[new_tier as usize]))
                } else {
                    ("tier_down".to_string(),
                     format!("{} is no longer counted among the {} houses", name, TIER_NAMES[prev as usize]))
                };
                self.houses[hi].events.push(HouseEvent { tick, kind, text: text.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "tier".into(), hub: self.houses[hi].hub as i32, good: -1,
                    value: new_tier as f32, text,
                });
            }
            self.houses[hi].tier = new_tier;
            self.mark_positive_events(hi, new_tier);
        }
    }

    /// Phase 1.4 · the two positive events measurable from state `assign_house_tiers`
    /// already touches this month. `HOUSE_PEOPLE_AND_TIERS.md` §2.2's other three
    /// (a legendary head, a great partnership, the finest hour as a chronicled EVENT
    /// rather than a fact) are deferred — the first needs goals (Phase 3, unbuilt), the
    /// second needs alliance-linked tier rises (a bigger join than this pass does), and
    /// spamming a peak-wealth event most months is worse than the obituary problem it's
    /// meant to fix. Recording the fact (below) without an event is the honest partial.
    fn mark_positive_events(&mut self, hi: usize, tier: u8) {
        let tick = self.tick;
        let wealth = self.houses[hi].wealth;

        // ── The house's finest hour — a marker, never an event. ──────────────────
        if wealth > self.houses[hi].peak_wealth {
            self.houses[hi].peak_wealth = wealth;
            self.houses[hi].peak_wealth_tick = tick;
        }

        // ── A golden age — Tier 1 held, wealth still rising, for a decade. ───────
        let rising = wealth > self.houses[hi].wealth_last_check;
        if tier == 1 && rising {
            self.houses[hi].golden_age_months += 1;
        } else {
            self.houses[hi].golden_age_months = 0;
            self.houses[hi].golden_age_chronicled = false;
        }
        if self.houses[hi].golden_age_months >= GOLDEN_AGE_MONTHS && !self.houses[hi].golden_age_chronicled {
            self.houses[hi].golden_age_chronicled = true;
            let name = self.houses[hi].name.clone();
            self.houses[hi].events.push(HouseEvent {
                tick, kind: "golden_age".into(),
                text: format!("A golden age for {} — a decade at the height of its power", name),
            });
        }
        self.houses[hi].wealth_last_check = wealth;
    }

    /// The RAW tier a (percentile, standing) pair bands into, with no memory of what
    /// tier the house held before. Used only for a house's first-ever assignment.
    fn tier_band(pct: f32, standing: f32) -> u8 {
        if pct < TIER_PCT_CUTS[0] && standing >= TIER1_STANDING_ENTER { 1 }
        else if pct < TIER_PCT_CUTS[1] { 2 }
        else if pct < TIER_PCT_CUTS[2] { 3 }
        else { 4 }
    }

    /// The tier a house holds THIS month, given the tier it held last month. Only the
    /// two percentile cutoffs bordering `prev`'s own band are widened by the dead band —
    /// a house must cross a boundary it isn't already past to change tier, which is what
    /// stops a score sitting near a cutoff from relabelling every month. Tier 1
    /// additionally carries its own absolute-floor hysteresis (`TIER1_STANDING_ENTER`/
    /// `_EXIT`), independent of rank.
    fn tier_with_hysteresis(prev: u8, pct: f32, standing: f32) -> u8 {
        let mut cuts = TIER_PCT_CUTS;
        if (2..=4).contains(&prev) { cuts[(prev - 2) as usize] -= TIER_PCT_DEAD_BAND; }
        if (1..=3).contains(&prev) { cuts[(prev - 1) as usize] += TIER_PCT_DEAD_BAND; }
        let by_rank = if pct < cuts[0] { 1 } else if pct < cuts[1] { 2 }
            else if pct < cuts[2] { 3 } else { 4 };
        if by_rank == 1 {
            let floor = if prev == 1 { TIER1_STANDING_EXIT } else { TIER1_STANDING_ENTER };
            if standing >= floor { 1 } else { 2 }
        } else {
            by_rank
        }
    }

    // ── Phase 0.4 · the law of inheritance ──────────────────────────────────────

    /// Resolve each live people's law of inheritance ONCE and keep it. Called yearly
    /// (and at campaign start) — a culture already in the registry is never re-read, so
    /// a people's law is fixed for the life of the world even if the worldgen culture
    /// map is unavailable on a later load.
    pub(crate) fn ensure_culture_rules(&mut self) {
        // Hub cultures first (deterministic hub order), then creoles as they arise.
        let mut names: Vec<String> = Vec::new();
        for c in self.hub_culture.iter() {
            if c.is_empty() || c == "—" { continue; }
            if !names.iter().any(|n| n == c) { names.push(c.clone()); }
        }
        for c in self.creoles.iter() {
            if !names.iter().any(|n| *n == c.name) { names.push(c.name.clone()); }
        }
        for n in names {
            if self.culture_rules.iter().any(|r| r.culture == n) { continue; }
            // A creole reckons descent as its first parent did; a named people by its
            // own language kit; anything else falls back to the kit distribution.
            let kit = self.creoles.iter().find(|c| c.name == n).map(|c| c.kit_a as usize)
                .or_else(|| crate::sim::cultures::kit_of_people(&n));
            // Clannish (trait index 8) — kin-bound descent groups, the precondition for
            // reckoning through the female line.
            let clannish = kit.is_some_and(|k| {
                crate::sim::cultures::kit_traits(k, 0.35, self.seed ^ k as u64).contains(&8)
            });
            let (line, rule) = crate::sim::inheritance::rules_for(&n, kit, clannish, self.seed);
            self.culture_rules.push(CultureRule {
                culture: n, line: line.as_u8(), rule: rule.as_u8(),
            });
        }
    }

    /// The law of inheritance in force at `hub` — its majority people's. Pure: an
    /// unregistered culture resolves from its own name, so this is never undefined and
    /// never depends on when `ensure_culture_rules` last ran.
    pub(crate) fn rules_for_hub(&self, hub: usize) -> (LineRule, InheritanceRule) {
        let culture = self.hub_culture.get(hub).cloned().unwrap_or_default();
        if let Some(r) = self.culture_rules.iter().find(|r| r.culture == culture) {
            return (LineRule::from_u8(r.line), InheritanceRule::from_u8(r.rule));
        }
        crate::sim::inheritance::rules_for(&culture, None, false, self.seed)
    }

    /// A head's name, drawn from the culture's female bank where the line rule puts a
    /// woman at the head of the house.
    pub(crate) fn head_name_sexed_for(&self, hub: usize, house_name: &str, salt: u64, female: bool) -> String {
        if !female { return self.head_name_for(hub, house_name, salt); }
        let surname = house_name.strip_prefix("House ").unwrap_or(house_name);
        let (x, y) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        crate::sim::names::gen_head_name_sexed(
            x, y, self.world_w as u32, self.world_h(), surname, salt, true)
    }

    /// `(age at accession, tenure in ticks)` for an heir under `rule`.
    ///
    /// The point is that an heir is NOT a newborn: they inherit at whatever age the
    /// rule implies and rule only for what remains of their life. An eldest son takes
    /// over in his thirties; the Mongol *otchigin* — the hearth-keeping youngest — as a
    /// young man, so his tenure is long but opens under a weak, inexperienced head; an
    /// elected tanist is by definition the eldest capable, so his is short. That is the
    /// whole difference between concentration rules that otherwise look identical.
    pub(crate) fn roll_tenure(&self, rule: InheritanceRule, salt: u64) -> (u32, u32) {
        let ra = hash01(self.seed, self.tick as u64 ^ 0x11FE, salt);
        let rd = hash01(self.seed, self.tick as u64 ^ 0x2FED, salt ^ 0x5A5A);
        let (lo, span) = match rule {
            InheritanceRule::Ultimogeniture => (HEIR_AGE_YOUNGEST, 14.0),
            InheritanceRule::Seniority => (HEIR_AGE_ELECTED, 18.0),
            _ => (HEIR_AGE_ELDEST, 18.0),
        };
        let age = lo + ra * span;
        // Age at death, for someone who has already survived to adulthood. Pre-modern
        // life expectancy AT BIRTH was low because so many died as infants; a merchant
        // who lived to hold a house commonly saw his sixties or seventies.
        let death = (HEAD_DEATH_AGE_MIN + rd * HEAD_DEATH_AGE_SPAN).max(age + MIN_TENURE_YEARS);
        (age as u32, ((death - age) * TICKS_PER_YEAR as f32) as u32)
    }

    /// Open the founding head's record on every house seeded at campaign start, and
    /// bring each head into line with the seat culture's rule — a people that reckons
    /// descent through its daughters must not be handed a founding roster of men.
    /// Idempotent: a house that already has a line is left alone.
    pub(crate) fn seed_house_lines(&mut self) {
        for hi in 0..self.houses.len() {
            if !self.houses[hi].line.is_empty() { continue; }
            let hub = self.houses[hi].hub as usize;
            if hub >= self.hubs.len() { continue; }
            if !self.houses[hi].is_guild {
                let (line_rule, _) = self.rules_for_hub(hub);
                let female = crate::sim::inheritance::heir_is_female(
                    line_rule, hi as u64 ^ 0x5EED, self.seed);
                if female {
                    let name = self.houses[hi].name.clone();
                    self.houses[hi].head_name =
                        self.head_name_sexed_for(hub, &name, 0x100 ^ hi as u64, true);
                }
                self.houses[hi].head_female = female;
            }
            // Always assign the tenure, never only when unset: the head fields on a
            // freshly-seeded house are placeholders, and a house whose founder outlives
            // the campaign never reaches a succession at all — which silently switches
            // off everything downstream of one.
            let (age, tenure) = self.roll_founder_tenure(hi as u64 ^ 0xA11);
            self.houses[hi].head_age = age;
            self.houses[hi].head_lifespan = tenure;
            self.found_head_record(hi, "founder");
        }
    }

    /// `(age, tenure ticks)` for a FOUNDER — a merchant who built the house, so already
    /// established in life rather than an heir arriving by descent.
    pub(crate) fn roll_founder_tenure(&self, salt: u64) -> (u32, u32) {
        let ra = hash01(self.seed, self.tick as u64 ^ 0x30F1, salt);
        let rd = hash01(self.seed, self.tick as u64 ^ 0x30F2, salt ^ 0x1234);
        let age = 30.0 + ra * 16.0;
        let death = (HEAD_DEATH_AGE_MIN + rd * HEAD_DEATH_AGE_SPAN).max(age + MIN_TENURE_YEARS);
        (age as u32, ((death - age) * TICKS_PER_YEAR as f32) as u32)
    }

    /// Open the founding head's record on a brand-new house. A founder is a grown
    /// merchant, not an heir — they take the house at the age they built it.
    pub(crate) fn found_head_record(&mut self, hi: usize, accession: &str) {
        let tick = self.tick;
        let (name, female, age, wealth, generation) = {
            let h = &self.houses[hi];
            (h.head_name.clone(), h.head_female, h.head_age, h.wealth, h.generation)
        };
        self.houses[hi].line.push(HouseHead {
            name, female, generation,
            since: tick, until: 0, age_at_accession: age, age_at_death: 0,
            wealth_start: wealth, wealth_end: 0.0,
            accession: accession.into(), epithet: String::new(),
        });
        self.ensure_kin_roster(hi);
    }

    // ── Phase 2.1 · the Kin roster ───────────────────────────────────────────────

    /// Four culture-derived axes, −2..+2 (§3): 0 caution↔boldness · 1 honour↔greed ·
    /// 2 private↔civic · 3 rooted↔expansive. Read by `head_character_factor` (Phase
    /// 2.4) to bound-modify one real decision per axis.
    fn roll_character(&self, salt: u64) -> [i8; 4] {
        let mut out = [0i8; 4];
        for (i, o) in out.iter_mut().enumerate() {
            let r = hash01(self.seed, salt ^ 0xCACA_0000 ^ (i as u64) << 8, i as u64);
            *o = ((r - 0.5) * 5.0).round().clamp(-2.0, 2.0) as i8;
        }
        out
    }

    /// (Re)generate a house's kin roster: `kin[0]` mirrors the current head, plus 2–4
    /// siblings/adult children — some `posted` to the house's current holdings (up to
    /// two, "factor" role), the rest `idle`. Called on every founding/succession, so
    /// the roster stays a snapshot of "who's around right now" rather than drifting
    /// stale for a family that never changes head. A guild gets no roster — it's a
    /// civic office, not a family.
    pub(crate) fn ensure_kin_roster(&mut self, hi: usize) {
        if self.houses[hi].is_guild { return; }
        let tick = self.tick;
        let seed = self.seed;
        let hub = self.houses[hi].hub as usize;
        if hub >= self.hubs.len() { return; }
        let (head_name, head_female, head_age, gen) = {
            let h = &self.houses[hi];
            (h.head_name.clone(), h.head_female, h.head_age, h.generation as u64)
        };
        let head_born = tick.saturating_sub((head_age as u32).saturating_mul(TICKS_PER_YEAR));
        let mut roster = vec![Kin {
            name: head_name, female: head_female, born_tick: head_born, dies_tick: 0,
            role: 0, posted: -1,
            character: self.roll_character((hi as u64) << 8 ^ gen << 20 ^ 0x4EAD),
            loyalty: 1.0,
            skill: 0.5 + hash01(seed, hi as u64 ^ gen ^ 0x5C11, 0) * 0.4,
            parent: -1,
        }];
        // Up to two of the house's current holdings get a POSTED kin (a "factor" from
        // the family); the house's own seat is never one — the head is already there.
        let holdings: Vec<i32> = self.hubs.iter().enumerate()
            .filter(|&(hi2, x)| x.is_estate && x.owner_house == hi as i32 && hi2 != hub)
            .map(|(i, _)| i as i32)
            .chain(self.houses[hi].offices.iter().copied().map(|o| o as i32).filter(|&o| o as usize != hub))
            .take(2)
            .collect();
        let n_extra = 2 + (crate::sim::cultures::hash64(seed ^ (hi as u64) << 16 ^ gen) % 3) as usize;
        for k in 0..n_extra {
            let salt = (hi as u64) << 24 ^ gen << 8 ^ k as u64;
            let kfemale = hash01(seed, salt ^ 0x5345, 0) < 0.5;
            let surname = self.houses[hi].name.strip_prefix("House ").unwrap_or(&self.houses[hi].name).to_string();
            let kname = self.head_name_sexed_for(hub, &surname, salt ^ 0x1D1D, kfemale);
            let posted = holdings.get(k).copied().unwrap_or(-1);
            let role: u8 = if posted >= 0 { 2 } else { 3 };
            let age_years = 16.0 + hash01(seed, salt, 1) * 34.0;
            roster.push(Kin {
                name: kname, female: kfemale,
                born_tick: tick.saturating_sub((age_years as u32).saturating_mul(TICKS_PER_YEAR)),
                dies_tick: 0, role, posted,
                character: self.roll_character(salt ^ 0x4EAD),
                loyalty: 0.4 + hash01(seed, salt, 2) * 0.6,
                skill: 0.3 + hash01(seed, salt, 3) * 0.6,
                parent: 0,
            });
        }
        self.houses[hi].kin = roster;
    }

    // ── Phase 2.4 · character wired to a real decision, ±15% capped ─────────────
    // Four axes, four touchpoints — one real decision each, per §3's own rule that an
    // axis that doesn't move something is horoscope text. Deliberately ONE touchpoint
    // per axis rather than all three the design lists (e.g. boldness ALSO names
    // expedition launch odds and contract terms) — enough to prove the axis is wired,
    // not decoration, without the larger surface area of every listed knob at once.
    //
    //   0 caution↔boldness   → the fleet-buy affordability threshold (`decide_fleets`)
    //   1 honour↔greed       → how fast a feud HEATS (`update_feuds`)
    //   2 private↔civic      → the house's conspicuous-consumption rate into its
    //                          seat's civic pool (`apply_wealth_sinks`), which is what
    //                          FUNDS `fund_public_works` in the first place
    //   3 rooted↔expansive   → the affordability threshold for opening a new office
    //                          (`update_guilds_and_offices`)
    //
    /// A bounded ±`CHARACTER_KNOB_CAP` modifier from the HEAD's own character
    /// (`kin[0]`) on `axis`, as a multiplier centred on 1.0. Returns exactly 1.0 (a
    /// true no-op, not an approximation) when there's no roster or the axis rolled 0
    /// — which is what keeps "no roster / all-zero character ⇒ bit-identical" true
    /// without any special-casing at the call sites.
    pub(crate) fn head_character_factor(&self, hi: usize, axis: usize) -> f32 {
        let Some(head) = self.houses.get(hi).and_then(|h| h.kin.first()) else { return 1.0; };
        let v = head.character.get(axis).copied().unwrap_or(0) as f32;
        1.0 + (v / 2.0) * CHARACTER_KNOB_CAP
    }

    /// The head's raw character axis, 0 if no roster — used to BIAS which goal a
    /// house picks (§4), not to modify a decision (that's `head_character_factor`).
    pub(crate) fn head_axis(&self, hi: usize, axis: usize) -> i8 {
        self.houses.get(hi).and_then(|h| h.kin.first())
            .and_then(|k| k.character.get(axis).copied()).unwrap_or(0)
    }

    // ── Phase 3.1 · goals ──────────────────────────────────────────────────────
    // §4: "a goal must be able to SUCCEED or FAIL and be recorded, or it is
    // decoration." Chosen yearly if a house has a free slot; checked yearly against
    // state that already exists elsewhere in the sim. A goal never adds a new
    // action — it is read-only against everything except its own `progress`/`state`.

    /// Give every live, non-guild house a chance to take up a new ambition if it has
    /// a free slot (1, or `GOAL_SLOTS_TIER1` for a Tier 1 house). Called yearly.
    pub(crate) fn choose_house_goal(&mut self, hi: usize) {
        if self.houses[hi].defunct || self.houses[hi].is_guild { return; }
        let slots = if self.houses[hi].tier == 1 { GOAL_SLOTS_TIER1 } else { GOAL_SLOTS_OTHER };
        if self.houses[hi].goals.len() >= slots { return; }
        let tick = self.tick;
        let hub = self.houses[hi].hub as usize;
        let seed = self.seed;

        // Each candidate: (kind, target_good, target_hub, target_house, target_province, score).
        let mut cand: Vec<(u8, i32, i32, i32, i32, f32)> = Vec::new();
        let bold = self.head_axis(hi, 0) as f32;
        let greed = self.head_axis(hi, 1) as f32;
        let civic = self.head_axis(hi, 2) as f32;
        let expansive = self.head_axis(hi, 3) as f32;

        // Corner the trade: pick the house's own top specialty, if not already held.
        if let Some(&g) = self.houses[hi].spec.first() {
            let already = self.houses[hi].monopoly.iter().any(|&(mg, s)| mg == g && s >= 0.6);
            if !already {
                let score = 1.0 + greed.max(0.0) + if self.houses[hi].archetype == ARCH_SPECIALTY { 1.0 } else { 0.0 };
                cand.push((GOAL_CORNER_TRADE, g as i32, -1, -1, -1, score));
            }
        }
        // Seat the council of the house's own seat, if it doesn't already.
        if self.hubs.get(hub).is_some_and(|h| h.council_house != hi as i32) {
            let score = 1.0 + civic.max(0.0)
                + if self.houses[hi].archetype == ARCH_POLITICAL { 1.5 } else { 0.0 };
            cand.push((GOAL_SEAT_COUNCIL, -1, hub as i32, -1, -1, score));
        }
        // Raise a bailo: an owned office not yet one.
        if let Some(&oh) = self.houses[hi].offices.iter().find(|&&o| !self.houses[hi].bailos.contains(&o)) {
            let score = 1.0 + expansive.max(0.0)
                + if self.houses[hi].archetype == ARCH_FLEET { 1.0 } else { 0.0 };
            cand.push((GOAL_RAISE_BAILO, -1, oh as i32, -1, -1, score));
        }
        // Charter a bank (and hold it solvent for a decade) — only if it doesn't
        // already own one.
        if !self.banks.iter().any(|b| !b.defunct && b.house == hi as u32) {
            let score = 1.0 + if self.houses[hi].archetype == ARCH_BANKING { 1.5 } else { 0.0 };
            cand.push((GOAL_CHARTER_BANK, -1, -1, -1, -1, score));
        }
        // Reach a distant province — needs the province layer AND a bold/expansive
        // pull, or it's just noise on a world that mostly doesn't use it.
        if !self.prov_seat.is_empty() && (bold > 0.0 || expansive > 0.0) {
            let home_prov = self.hub_province.get(hub).copied().unwrap_or(-1);
            let n = self.prov_seat.len();
            let roll = (crate::sim::cultures::hash64(seed ^ (hi as u64) << 8 ^ tick as u64) % n as u64) as i32;
            if roll != home_prov {
                let score = 0.5 + bold.max(0.0) + expansive.max(0.0);
                cand.push((GOAL_REACH_PROVINCE, -1, -1, -1, roll, score));
            }
        }
        // Outlast a rival: the hottest live feud this house is party to.
        if let Some(rival) = self.hottest_rival(hi) {
            let score = 0.5 + greed.max(0.0);
            cand.push((GOAL_OUTLAST_RIVAL, -1, -1, rival as i32, -1, score));
        }
        // Restore the house: only eligible once it has genuinely fallen — half its
        // own all-time peak or worse. A house that never fell has nothing to restore.
        if self.houses[hi].peak_wealth > 0.0
            && self.houses[hi].wealth < self.houses[hi].peak_wealth * 0.5 {
            cand.push((GOAL_RESTORE_HOUSE, -1, -1, -1, -1, 2.0)); // urgent when eligible
        }

        if cand.is_empty() { return; }
        // Highest score wins; a small deterministic jitter breaks ties without
        // always favouring the first-listed kind.
        let (kind, tg, th, thh, tp, _) = cand.into_iter()
            .max_by(|a, b| {
                let ja = a.5 + hash01(seed, tick as u64 ^ (hi as u64) << 16, a.0 as u64) * 0.3;
                let jb = b.5 + hash01(seed, tick as u64 ^ (hi as u64) << 16, b.0 as u64) * 0.3;
                ja.partial_cmp(&jb).unwrap_or(std::cmp::Ordering::Equal)
            }).unwrap();
        let deadline_years = GOAL_DEADLINE_YEARS.get(kind as usize).copied().unwrap_or(20.0);
        // RESTORE_HOUSE's "progress" field holds the TARGET wealth (the peak at the
        // moment the goal was set) — see the kind's own doc note in `update_house_goal`.
        let progress0 = if kind == GOAL_RESTORE_HOUSE { self.houses[hi].peak_wealth } else { 0.0 };
        self.houses[hi].goals.push(Goal {
            kind, target_good: tg, target_hub: th, target_house: thh, target_province: tp,
            set_tick: tick, deadline_tick: tick + (deadline_years * TICKS_PER_YEAR as f32) as u32,
            progress: progress0, state: GOAL_PURSUING,
        });
        let text = self.goal_set_text(hi, kind, tg, th, thh, tp);
        self.houses[hi].events.push(HouseEvent { tick, kind: "goal_set".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "goal_set".into(), hub: hub as i32, good: tg, value: 0.0, text,
        });
    }

    /// The live rival this house feuds hottest with, if any.
    fn hottest_rival(&self, hi: usize) -> Option<usize> {
        self.feuds.iter()
            .filter(|f| f.outcome == FEUD_RUNNING && (f.a as usize == hi || f.b as usize == hi))
            .max_by(|a, b| a.intensity.partial_cmp(&b.intensity).unwrap_or(std::cmp::Ordering::Equal))
            .map(|f| if f.a as usize == hi { f.b as usize } else { f.a as usize })
    }

    fn goal_set_text(&self, hi: usize, kind: u8, tg: i32, th: i32, thh: i32, tp: i32) -> String {
        let name = self.houses[hi].name.clone();
        match kind {
            GOAL_CORNER_TRADE => format!("{} sets its sights on cornering the {} trade", name,
                self.goods.get(tg as usize).map(|g| g.name.as_str()).unwrap_or("local")),
            GOAL_SEAT_COUNCIL => format!("{} resolves to seat the council of {}", name,
                self.hubs.get(th as usize).map(|h| h.name.as_str()).unwrap_or("its seat")),
            GOAL_RAISE_BAILO => format!("{} means to raise a bailo at {}", name,
                self.hubs.get(th as usize).map(|h| h.name.as_str()).unwrap_or("its office")),
            GOAL_CHARTER_BANK => format!("{} sets out to charter a bank of its own", name),
            GOAL_REACH_PROVINCE => format!("{} vows to reach the far province beyond {}", name,
                self.hubs.get(self.houses[hi].hub as usize).map(|h| h.name.as_str()).unwrap_or("home")),
            GOAL_OUTLAST_RIVAL => format!("{} swears to outlast {}", name,
                self.houses.get(thh as usize).map(|h| h.name.as_str()).unwrap_or("its rival")),
            GOAL_RESTORE_HOUSE => format!("{} vows to restore the house to its former wealth", name),
            _ => format!("{} takes up a new ambition", name),
        }
    }

    /// Check one house's active goals for success/failure. Called yearly. `progress`
    /// is advanced or reset per-kind; a goal that neither succeeds nor expires stays
    /// `GOAL_PURSUING` untouched.
    pub(crate) fn update_house_goal(&mut self, hi: usize) {
        if self.houses[hi].goals.is_empty() { return; }
        let tick = self.tick;
        let mut done: Vec<usize> = Vec::new();
        for gi in 0..self.houses[hi].goals.len() {
            let (kind, tg, th, thh, tp, deadline, state) = {
                let g = &self.houses[hi].goals[gi];
                (g.kind, g.target_good, g.target_hub, g.target_house, g.target_province, g.deadline_tick, g.state)
            };
            let mut achieved = state == GOAL_ACHIEVED;
            match kind {
                GOAL_CORNER_TRADE => {
                    let share = self.houses[hi].monopoly.iter()
                        .find(|&&(g, _)| g as i32 == tg).map(|&(_, s)| s).unwrap_or(0.0);
                    let g = &mut self.houses[hi].goals[gi];
                    if share >= 0.6 { g.progress += 1.0; } else { g.progress = 0.0; }
                    achieved = g.progress >= GOAL_HOLD_YEARS_TRADE;
                }
                GOAL_SEAT_COUNCIL => {
                    achieved = self.hubs.get(th as usize)
                        .is_some_and(|h| h.council_house == hi as i32 || h.captor_house == hi as i32);
                }
                GOAL_RAISE_BAILO => {
                    achieved = self.houses[hi].bailos.contains(&(th as u32));
                }
                GOAL_CHARTER_BANK => {
                    let solvent = self.banks.iter().any(|b| !b.defunct && b.house == hi as u32);
                    let g = &mut self.houses[hi].goals[gi];
                    if solvent { g.progress += 1.0; } else { g.progress = 0.0; }
                    achieved = g.progress >= GOAL_HOLD_YEARS_BANK;
                }
                GOAL_REACH_PROVINCE => {
                    // Achieved externally: `expedition_travel_pass` sets `state` to
                    // GOAL_ACHIEVED directly the moment a BACKED expedition completes
                    // its round trip to `target_province` (already captured above as
                    // `achieved`). Nothing to poll here but the deadline.
                    let _ = tp;
                }
                GOAL_OUTLAST_RIVAL => {
                    achieved = self.houses.get(thh as usize).is_some_and(|r| r.defunct);
                }
                GOAL_RESTORE_HOUSE => {
                    // `progress` holds the TARGET — the peak wealth at the moment the
                    // goal was set, not a running counter.
                    let target = self.houses[hi].goals[gi].progress;
                    achieved = self.houses[hi].wealth >= target;
                }
                _ => {}
            }
            if achieved {
                self.houses[hi].goals[gi].state = GOAL_ACHIEVED;
                done.push(gi);
            } else if tick >= deadline {
                self.houses[hi].goals[gi].state = GOAL_FAILED;
                done.push(gi);
            }
        }
        for &gi in done.iter().rev() { self.close_goal(hi, gi); }
    }

    /// Close a goal — chronicle it (achieved is a MILESTONE, failed is chatter, same
    /// asymmetry as `monopoly`/`monopoly_lost` and `tier_up`/`tier_down`), move it to
    /// `goal_history`, and remove it from the active list.
    fn close_goal(&mut self, hi: usize, gi: usize) {
        let tick = self.tick;
        let g = self.houses[hi].goals.remove(gi);
        let name = self.houses[hi].name.clone();
        let what = self.goal_kind_phrase(g.kind, g.target_good, g.target_hub, g.target_house);
        if g.state == GOAL_ACHIEVED {
            self.houses[hi].events.push(HouseEvent {
                tick, kind: "goal_achieved".into(),
                text: format!("{} achieves its ambition — {}", name, what),
            });
            self.journal.push(JournalEntry {
                tick, kind: "goal_achieved".into(), hub: self.houses[hi].hub as i32, good: -1, value: 0.0,
                text: format!("{} achieves its ambition: {}", name, what),
            });
        } else {
            self.houses[hi].events.push(HouseEvent {
                tick, kind: "goal_failed".into(),
                text: format!("{} abandons hope of {}", name, what),
            });
        }
        self.houses[hi].goal_history.push(g);
        if self.houses[hi].goal_history.len() > GOAL_HISTORY_CAP {
            let drop = self.houses[hi].goal_history.len() - GOAL_HISTORY_CAP;
            self.houses[hi].goal_history.drain(0..drop);
        }
    }

    fn goal_kind_phrase(&self, kind: u8, tg: i32, th: i32, thh: i32) -> String {
        match kind {
            GOAL_CORNER_TRADE => format!("cornering the {} trade",
                self.goods.get(tg as usize).map(|g| g.name.as_str()).unwrap_or("local")),
            GOAL_SEAT_COUNCIL => format!("seating the council of {}",
                self.hubs.get(th as usize).map(|h| h.name.as_str()).unwrap_or("its seat")),
            GOAL_RAISE_BAILO => format!("raising a bailo at {}",
                self.hubs.get(th as usize).map(|h| h.name.as_str()).unwrap_or("its office")),
            GOAL_CHARTER_BANK => "chartering a bank".into(),
            GOAL_REACH_PROVINCE => "reaching the far province".into(),
            GOAL_OUTLAST_RIVAL => format!("outlasting {}",
                self.houses.get(thh as usize).map(|h| h.name.as_str()).unwrap_or("its rival")),
            GOAL_RESTORE_HOUSE => "restoring the house's former wealth".into(),
            _ => "its ambition".into(),
        }
    }

    /// Close the outgoing head's record: death tick, age, closing wealth, and the
    /// by-name their tenure earned. Descriptive only — derived from what measurably
    /// happened, never fed back into the sim.
    pub(crate) fn close_head_record(&mut self, hi: usize) {
        let tick = self.tick;
        let wealth = self.houses[hi].wealth;
        let (start, since, age0) = match self.houses[hi].line.last() {
            Some(p) => (p.wealth_start, p.since, p.age_at_accession),
            // A house from a save written before the line existed: reconstruct the one
            // record we can honestly write, from the head fields it does carry.
            None => {
                let h = &self.houses[hi];
                let (nm, f, g, s, a) =
                    (h.head_name.clone(), h.head_female, h.generation, h.head_since, h.head_age);
                self.houses[hi].line.push(HouseHead {
                    name: nm, female: f, generation: g, since: s, until: 0,
                    age_at_accession: a, age_at_death: 0,
                    wealth_start: wealth, wealth_end: 0.0,
                    accession: "heir".into(), epithet: String::new(),
                });
                (wealth, s, a)
            }
        };
        let years = tick.saturating_sub(since) / TICKS_PER_YEAR;
        let epithet = Self::head_epithet(start, wealth, years, age0 + years,
            hash01(self.seed, tick as u64 ^ 0xEB17, hi as u64));
        if let Some(p) = self.houses[hi].line.last_mut() {
            p.until = tick;
            p.wealth_end = wealth;
            p.age_at_death = age0 + years;
            p.epithet = epithet;
        }
        self.maybe_chronicle_dynasty(hi);
    }

    /// "A dynasty of merchants" (§2.2) — three consecutive heads who EACH left the
    /// house richer than they found it. Purely derived from `line`, which Phase 0.4
    /// already writes; chronicled once per streak so it doesn't refire at every
    /// qualifying succession after the third.
    fn maybe_chronicle_dynasty(&mut self, hi: usize) {
        let tick = self.tick;
        let line = &self.houses[hi].line;
        if line.len() < DYNASTY_HEADS { return; }
        let recent = &line[line.len() - DYNASTY_HEADS..];
        let streak = recent.iter().all(|p| p.until > 0 && p.wealth_end > p.wealth_start);
        if !streak {
            self.houses[hi].dynasty_chronicled = false;
            return;
        }
        if self.houses[hi].dynasty_chronicled { return; }
        self.houses[hi].dynasty_chronicled = true;
        let name = self.houses[hi].name.clone();
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "dynasty".into(),
            text: format!("A dynasty of merchants — {} generations of {} in a row have left the house richer than they found it",
                DYNASTY_HEADS, name),
        });
    }

    /// The by-name a tenure earned. Most heads get NONE — an epithet everyone carries
    /// says nothing about anyone, which is the same discipline the stability gauges
    /// keep by staying quiet when healthy.
    fn head_epithet(start: f32, end: f32, years: u32, age: u32, r: f32) -> String {
        let grew = if start.abs() > 1e-3 { end / start } else { 1.0 };
        let pick = |a: &'static str, b: &'static str| if r < 0.5 { a } else { b };
        if years < 5 && end > 0.0 { return pick("the Brief", "the Untimely").into(); }
        if grew >= 3.0 && years >= 12 { return pick("the Great", "the Magnificent").into(); }
        if grew >= 1.8 { return pick("the Fortunate", "the Bold").into(); }
        if grew <= 0.4 { return pick("the Unlucky", "the Prodigal").into(); }
        if age >= 78 { return pick("the Old", "the Long-lived").into(); }
        if years >= 30 && grew >= 1.0 { return pick("the Steady", "the Patient").into(); }
        String::new()
    }

    /// PARTIBLE inheritance — the estate divides in equal shares among the heirs.
    ///
    /// Two outcomes, and both are historical. Where the shares are large enough to
    /// stand alone the co-heirs set up as separate firms at the same seat, which is the
    /// fragmentation partible inheritance is famous for. Where they are not, the
    /// brothers keep the capital together and trade as one — the Italian *fraterna* —
    /// and the house survives the generation whole. No money is created or destroyed
    /// either way: a share leaves the parent exactly as it arrives at the co-heir.
    pub(crate) fn divide_estate(&mut self, hi: usize, gen: u32) {
        // A civic guild is not a family and has no estate to divide.
        if self.houses[hi].is_guild || self.houses[hi].defunct { return; }
        let wealth = self.houses[hi].wealth;
        if wealth <= 0.0 { return; }
        let heirs = crate::sim::inheritance::partible_heirs(hi as u64 ^ gen as u64, self.seed);
        let share = wealth / heirs as f32;
        let hub = self.houses[hi].hub as usize;
        let tick = self.tick;
        // Too small to stand alone → the heirs hold jointly. Recorded, because a
        // generation that did NOT split is as much a consequence of the rule as one
        // that did.
        if share < HOUSE_SEED_MIN {
            self.houses[hi].events.push(HouseEvent {
                tick, kind: "inheritance".into(),
                text: format!("the {} heirs keep the capital together and trade as one house", heirs),
            });
            return;
        }
        let live = self.houses.iter().filter(|h| !h.defunct && !h.is_guild).count();
        let room = HOUSE_MAX_TOTAL.saturating_sub(live).min(PARTIBLE_MAX_SPLIT);
        let splits = ((heirs - 1) as usize).min(room);
        if splits == 0 { return; }
        let parent = self.houses[hi].name.clone();
        let spec = self.houses[hi].spec.clone();
        let arch = self.houses[hi].archetype;
        let (line_rule, inh) = self.rules_for_hub(hub);
        for k in 0..splits {
            let salt = (hi as u64) << 20 ^ (gen as u64) << 8 ^ k as u64;
            let cname = self.coheir_name_for(&parent, k);
            if self.houses.iter().any(|h| !h.defunct && h.name == cname) { continue; }
            let female = crate::sim::inheritance::heir_is_female(line_rule, salt, self.seed);
            let chead = self.head_name_sexed_for(hub, &cname, salt ^ 0xC0DE, female);
            let (age, tenure) = self.roll_tenure(inh, salt ^ 0x7777);
            self.houses[hi].wealth -= share;
            let founded = HouseEvent {
                tick, kind: "founded".into(),
                text: format!("{} takes a co-heir's share of {} and sets up on their own account",
                    chead, parent),
            };
            self.houses.push(House {
                name: cname.clone(), hub: hub as u32, wealth: share, prestige: 0.05,
                spec: spec.clone(), monopoly: vec![], rivals: vec![], generation: gen,
                events: vec![founded], good_profit: Vec::new(), good_volume: Vec::new(), mono50: Vec::new(),
                mono_ever: Vec::new(), dominant_seat: false, prev_wealth: share, worst_loss: 0.0,
                // A co-heir's share is CAPITAL, not ships: the vessels stay with the
                // parent firm. Founding a house with hulls it cannot crew is exactly the
                // arithmetic that killed every new house before Phase 0.2.
                fleet_sea: 0, fleet_river: 0, fleet_caravan: 0,
                head_name: chead.clone(), head_since: tick, head_lifespan: tenure,
                founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
                archetype: arch, charters: Vec::new(),
                is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
                wealth_history: Vec::new(), office_leases: Vec::new(),
                influence: Vec::new(), bailos: Vec::new(),
                head_female: female, head_age: age, line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0,
                origin_house: hi as i32, origin_kind: ORIGIN_DIVISION, crowned: false, realm: -1,
            });
            let ni = self.houses.len() - 1;
            self.found_head_record(ni, "co-heir");
            self.journal.push(JournalEntry {
                tick, kind: "founding".into(), hub: hub as i32, good: -1, value: share,
                text: format!("{} divides: {} takes a co-heir's share and opens their own house",
                    parent, chead),
            });
        }
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "inheritance".into(),
            text: format!("the estate is divided in {} shares among the heirs", heirs),
        });
    }

    /// A co-heir's house name — the same family, a distinguished line: "House Cassii
    /// (the Younger Line)". Keeps the surname so the division reads as one family
    /// splitting rather than a stranger appearing.
    pub(crate) fn coheir_name_for(&self, parent_name: &str, k: usize) -> String {
        const LINES: [&str; 3] = ["the Younger Line", "the Cadet Line", "the Lesser Line"];
        // Keep the parent's name EXACTLY as it reads (it already carries whatever
        // "House"/guild prefix its culture gave it) and distinguish the line — a second
        // "House" prefix would read as a different family, which is the opposite of
        // what a division is.
        let base = parent_name.split(" (").next().unwrap_or(parent_name).trim();
        format!("{} ({})", base, LINES[k.min(LINES.len() - 1)])
    }

    /// A branch name that KEEPS the family identity: "House Cassii of <City>", so
    /// the same family visibly spreads across cities (instead of inventing an
    /// unrelated surname). Unique per city.
    pub(crate) fn branch_name_for(&self, parent_name: &str, dest: usize) -> String {
        let surname = parent_name.strip_prefix("House ").unwrap_or(parent_name);
        let base = surname.split(" of ").next().unwrap_or(surname).trim();
        let city = self.hubs[dest].name.clone();
        let cand = format!("House {} of {}", base, city);
        if !self.houses.iter().any(|h| h.name == cand) { return cand; }
        format!("House {} of {} [{}]", base, city, self.tick)
    }


    /// Split a cadet branch of house `hi` into hub `dest`, carrying ~30% of the
    /// parent's wealth and its specialties, named to keep the family identity.
    pub(crate) fn found_branch(&mut self, hi: usize, dest: usize, parent: String) {
        let tick = self.tick;
        let bname = self.branch_name_for(&parent, dest);
        // Don't stack two branches of the same family in one city.
        if self.houses.iter().any(|h| !h.defunct && h.hub as usize == dest && h.name == bname) {
            return;
        }
        let split = self.houses[hi].wealth * 0.30;
        self.houses[hi].wealth -= split;
        let spec = self.houses[hi].spec.clone();
        let (line_rule, inh) = self.rules_for_hub(dest);
        let bfemale = crate::sim::inheritance::heir_is_female(
            line_rule, tick as u64 ^ hi as u64, self.seed);
        let bhead = self.head_name_sexed_for(dest, &bname, tick as u64 ^ hi as u64 ^ 0x9001, bfemale);
        let (bage, btenure) = self.roll_tenure(inh, dest as u64 ^ 0xCC);
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("Founded by {} as a branch of {} in {}", bhead, parent, self.hubs[dest].name),
        };
        self.houses.push(House {
            name: bname.clone(), hub: dest as u32, wealth: split, prestige: 0.1,
            spec, monopoly: vec![], rivals: vec![hi], generation: 1,
            events: vec![founded], good_profit: Vec::new(), good_volume: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: split, worst_loss: 0.0,
            // A cadet branch is endowed with CAPITAL, not with hulls. It used to be
            // handed `initial_fleet`'s two or three vessels it had never paid for and
            // could not crew — and the diagnosis says that is what killed it: branches
            // were 19 of 35 deaths, 74% of them houses that never traded at all, dead
            // at a mean age of 8 years. That is the same arithmetic Phase 0.2 found
            // behind the original 12-year house, arriving by a second door. It buys
            // ships out of its own share, through `manage_fleets`, when its trade
            // justifies them.
            fleet_sea: 0, fleet_river: 0, fleet_caravan: 0,
            head_name: bhead.clone(), head_since: tick,
            head_lifespan: btenure,
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: self.houses[hi].archetype, // a cadet branch keeps the family trade
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
            head_female: bfemale, head_age: bage, line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0,
            origin_house: hi as i32, origin_kind: ORIGIN_BRANCH, crowned: false, realm: -1,
        });
        let ni = self.houses.len() - 1;
        self.found_head_record(ni, "founder");
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: dest as i32, good: -1, value: 0.0,
            text: format!("{} founds a branch of {} in {}", bhead, parent, self.hubs[dest].name),
        });
    }


    /// Monthly: wealthy, established houses occasionally branch into a city they
    /// trade with — so a family network spreads across the map within a normal
    /// playthrough, not only on the rare succession that meets the old gen-3 bar.
    pub(crate) fn maybe_branch_houses(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; } // guilds expand via offices, not cadet branches
            if self.houses[hi].generation < 2 { continue; }
            if self.houses[hi].wealth <= HOUSE_BRANCH_WEALTH { continue; }
            // ~2.5%/month for an eligible house → a branch every few years.
            if hash01(self.seed, tick as u64 ^ 0xBA11, hi as u64) > 0.025 { continue; }
            let hub = self.houses[hi].hub as usize;
            if let Some(dest) = self.pick_branch_hub(hub) {
                let parent = self.houses[hi].name.clone();
                self.found_branch(hi, dest, parent);
            }
        }
    }


    /// A destination hub for a new branch: the house's strongest trade partner-ish
    /// — here, the nearest reachable hub in the same component that isn't home.
    pub(crate) fn pick_branch_hub(&self, home: usize) -> Option<usize> {
        let n = self.hubs.len();
        let comp = self.hubs[home].component;
        let mut best = (usize::MAX, f32::INFINITY);
        for b in 0..n {
            if b == home || self.hubs[b].component != comp { continue; }
            if self.hubs[b].is_estate { continue; } // branch into a real city, not an estate
            let d = self.days.get(home * n + b).copied().unwrap_or(f32::INFINITY);
            if d.is_finite() && d > 1.0 && d < best.1 { best = (b, d); }
        }
        if best.0 == usize::MAX { None } else { Some(best.0) }
    }


    /// A starting fleet for a new house, sized to its home geography: coastal
    /// seats are seafaring (ships + a caravan), inland ones overland (caravans +
    /// a river boat). `big` gives the seeded great houses a slightly larger fleet.
    pub(crate) fn initial_fleet(coastal: bool, big: bool) -> (u32, u32, u32) {
        match (coastal, big) {
            (true, true) => (2, 0, 1),
            (true, false) => (1, 0, 1),
            (false, true) => (0, 1, 2),
            (false, false) => (0, 1, 1),
        }
    }


    /// A lost voyage sometimes takes the vessel/caravan with it (~30%).
    pub(crate) fn damage_fleet(&mut self, hi: usize, sea: bool) {
        if hash01(self.seed, self.tick as u64 ^ 0xDEAD, hi as u64) > 0.30 { return; }
        let h = &mut self.houses[hi];
        if sea {
            if h.fleet_sea > 0 { h.fleet_sea -= 1; }
        } else if h.fleet_caravan > 0 {
            h.fleet_caravan -= 1;
        } else if h.fleet_river > 0 {
            h.fleet_river -= 1;
        }
    }


    /// Monthly fleet management: a profitable house whose vessels are all busy
    /// buys another (more capacity → more trade carried → more market share); a
    /// failing house with idle ships scraps one for a little cash. This capital
    /// churn — build-up, over-extension, loss, recovery — keeps the trade network
    /// perpetually shifting instead of settling into a static equilibrium.
    pub(crate) fn decide_fleets(&self) -> Vec<FleetChoice> {
        let tick = self.tick;
        let nh = self.houses.len();
        let mut used_sea = vec![0i32; nh];
        let mut used_land = vec![0i32; nh];
        for c in &self.in_transit {
            if c.owner >= 0 {
                let oi = c.owner as usize;
                if oi < nh { if c.sea { used_sea[oi] += 1; } else { used_land[oi] += 1; } }
            }
        }
        let mut out = Vec::with_capacity(nh);
        for hi in 0..nh {
            if self.houses[hi].defunct {
                out.push(FleetChoice {
                    wealth: self.houses[hi].wealth, fleet_sea: self.houses[hi].fleet_sea,
                    fleet_river: self.houses[hi].fleet_river, fleet_caravan: self.houses[hi].fleet_caravan,
                    fleet_cost_booked: 0.0,
                });
                continue;
            }
            let mut wealth = self.houses[hi].wealth;
            let mut fleet_sea = self.houses[hi].fleet_sea;
            let mut fleet_river = self.houses[hi].fleet_river;
            let mut fleet_caravan = self.houses[hi].fleet_caravan;
            let mut fleet_cost_booked = 0.0f32;
            // Phase G: fleet upkeep (a steady sink scaling with fleet size) + slow
            // decay (an occasional vessel lost to wear), so a big fleet costs money
            // to keep and must be continually rebuilt.
            let fleet_total = fleet_sea + fleet_river + fleet_caravan;
            if fleet_total > 0 {
                let fleet_cost = fleet_total as f32 * SHIP_COST * FLEET_UPKEEP_FRAC;
                wealth -= fleet_cost;
                fleet_cost_booked += fleet_cost;
                if hash01(self.seed, tick as u64 ^ 0x5EA1, hi as u64)
                    < FLEET_DECAY_CHANCE * fleet_total as f32
                {
                    if fleet_sea > 0 {
                        fleet_sea -= 1;
                    } else if fleet_caravan > 0 {
                        fleet_caravan -= 1;
                    } else if fleet_river > 0 {
                        fleet_river -= 1;
                    }
                }
            }
            let coastal = self.hubs.get(self.houses[hi].hub as usize).map(|x| x.coastal).unwrap_or(false);
            let w = wealth;
            let sea_slots = fleet_sea as i32;
            let land_slots = (fleet_river + fleet_caravan) as i32;
            let sea_busy = used_sea[hi] >= sea_slots;
            let land_busy = used_land[hi] >= land_slots;
            // Shipping dynasties build vessels at a discount.
            let disc = if self.houses[hi].archetype == ARCH_FLEET { FLEET_SHIP_DISCOUNT } else { 1.0 };
            // BUY: capital to spare and every vessel of the favoured kind is busy.
            // Phase 2.4 · a bold head buys on thinner margins (a lower multiplier), a
            // cautious one waits for a fatter cushion — axis 0, ±15% capped.
            let buy_mult = 2.5 * self.head_character_factor(hi, 0).recip();
            if coastal && sea_busy && w > SHIP_COST * buy_mult {
                wealth -= SHIP_COST * disc;
                fleet_sea += 1;
            } else if !coastal && land_busy && w > CARAVAN_COST * buy_mult {
                if hash01(self.seed, tick as u64 ^ 0x21B0, hi as u64) < 0.30 {
                    wealth -= RIVER_COST * disc;
                    fleet_river += 1;
                } else {
                    wealth -= CARAVAN_COST * disc;
                    fleet_caravan += 1;
                }
            } else if w < HOUSE_BRANCH_WEALTH * 0.15 {
                // SELL: a struggling house with an idle vessel scraps it for cash.
                if used_sea[hi] < sea_slots && fleet_sea > 0 {
                    fleet_sea -= 1;
                    wealth += SHIP_COST * 0.4;
                } else if used_land[hi] < land_slots {
                    if fleet_caravan > 0 {
                        fleet_caravan -= 1;
                        wealth += CARAVAN_COST * 0.4;
                    } else if fleet_river > 0 {
                        fleet_river -= 1;
                        wealth += RIVER_COST * 0.4;
                    }
                }
            }
            out.push(FleetChoice { wealth, fleet_sea, fleet_river, fleet_caravan, fleet_cost_booked });
        }
        out
    }

    /// Carries out a tick's `FleetChoice`s — the only part of fleet management
    /// that mutates house state. See `decide_fleets`'s doc comment (FIX_PLAN B2).
    pub(crate) fn apply_fleets(&mut self, choices: Vec<FleetChoice>) {
        for (hi, c) in choices.into_iter().enumerate() {
            self.houses[hi].wealth = c.wealth;
            self.houses[hi].fleet_sea = c.fleet_sea;
            self.houses[hi].fleet_river = c.fleet_river;
            self.houses[hi].fleet_caravan = c.fleet_caravan;
            if c.fleet_cost_booked != 0.0 && hi < self.house_ledger.len() {
                self.house_ledger[hi].fleet_cost += c.fleet_cost_booked;
            }
        }
    }

    /// The tick loop's entry point: AI decides, sim applies. A future player-run
    /// house would call `apply_fleets` directly with its own `FleetChoice` instead
    /// of going through `decide_fleets` (FIX_PLAN B2).
    pub(crate) fn manage_fleets(&mut self) {
        let choices = self.decide_fleets();
        self.apply_fleets(choices);
    }


    /// Recompute each house's per-good monopoly shares (volume among houses that
    /// specialize in the good) and its political power (wealth + monopoly +
    /// prestige). The dominant house concentrates a little wealth into its home
    /// city's commercial prosperity.
    pub(crate) fn recompute_monopolies_and_power(&mut self) {
        let ng = self.goods.len();
        let tick = self.tick;
        let nhubs = self.hubs.len();
        // Volume per good (across speccing houses) + per-hub resident-house volume
        // and the strongest resident (for >=50% city control).
        let mut good_vol = vec![0.0f32; ng];
        let mut hub_vol = vec![0.0f32; nhubs];
        let mut hub_top = vec![(usize::MAX, 0.0f32); nhubs];
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct { continue; }
            for &g in &hh.spec {
                if g < ng { good_vol[g] += hh.volume; }
            }
            let hub = hh.hub as usize;
            if hub < nhubs {
                let v = hh.volume.max(0.0001);
                hub_vol[hub] += v;
                if v > hub_top[hub].1 { hub_top[hub] = (hi, v); }
            }
        }
        let wmax = self.houses.iter().filter(|h| !h.defunct)
            .map(|h| h.wealth).fold(1.0f32, f32::max);
        let pmax = self.houses.iter().filter(|h| !h.defunct)
            .map(|h| h.prestige).fold(0.5f32, f32::max);
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let mut mono: Vec<(usize, f32)> = Vec::new();
            let mut top_share = 0.0f32;
            let mut shares: Vec<(usize, f32)> = Vec::new();
            let (spec, vol) = (self.houses[hi].spec.clone(), self.houses[hi].volume);
            for &g in &spec {
                if g >= ng { continue; }
                let share = if good_vol[g] > 1e-3 { (vol / good_vol[g]).clamp(0.0, 1.0) } else { 0.0 };
                if share > 0.25 { mono.push((g, share)); }
                top_share = top_share.max(share);
                shares.push((g, share));
            }
            // ── Monopoly milestones with HYSTERESIS ──────────────────────────
            // A monopoly is WON when share first reaches >=50% (recorded once);
            // it's only LOST when share falls below 10% (a genuine collapse, not
            // noise around the 50% line); a later re-win reads "regained". This
            // kills the per-month "won a monopoly" spam.
            let mut held = std::mem::take(&mut self.houses[hi].mono50);
            let mut ever = std::mem::take(&mut self.houses[hi].mono_ever);
            for &(g, share) in &shares {
                let is_held = held.contains(&g);
                if share >= 0.5 && !is_held {
                    let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                    let regained = ever.contains(&g);
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "monopoly".into(),
                        text: if regained { format!("Regained the monopoly on {}", gn) }
                              else { format!("Won a monopoly on {}", gn) },
                    });
                    held.push(g);
                    if !regained { ever.push(g); }
                } else if share < 0.10 && is_held {
                    let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "monopoly_lost".into(),
                        text: format!("Lost the monopoly on {}", gn),
                    });
                    held.retain(|&x| x != g);
                }
            }
            self.houses[hi].mono50 = held;
            self.houses[hi].mono_ever = ever;
            let wn = (self.houses[hi].wealth.max(0.0) / wmax).clamp(0.0, 1.0);
            let pn = (self.houses[hi].prestige / pmax).clamp(0.0, 1.0);
            // Political houses wield extra influence in their city's council.
            let arch_bonus = if self.houses[hi].archetype == ARCH_POLITICAL { POLITICAL_POWER_BONUS } else { 0.0 };
            let power = (0.45 * wn + 0.35 * top_share + 0.20 * pn + arch_bonus).clamp(0.0, 1.0);
            self.houses[hi].monopoly = mono;
            self.houses[hi].political_power = power;

            // Control of the seat city (>=50% of its resident-house trade).
            let hub = self.houses[hi].hub as usize;
            let now_dom = hub < nhubs && hub_top[hub].0 == hi
                && (hub_top[hub].1 / hub_vol[hub].max(1e-6)) >= 0.5;
            if now_dom != self.houses[hi].dominant_seat {
                let cn = self.hubs.get(hub).map(|x| x.name.clone()).unwrap_or_default();
                let (kind, text) = if now_dom {
                    ("control_gained", format!("Gained control of {}", cn))
                } else {
                    ("control_lost", format!("Lost control of {}", cn))
                };
                self.houses[hi].events.push(HouseEvent { tick, kind: kind.into(), text });
                self.houses[hi].dominant_seat = now_dom;
            }
            // Settlement grant: a POLITICAL house that controls its seat is granted a
            // city CHARTER on its specialty goods — a standing rent monopoly.
            if now_dom && self.houses[hi].archetype == ARCH_POLITICAL {
                let spec = self.houses[hi].spec.clone();
                let cn = self.hubs.get(hub).map(|x| x.name.clone()).unwrap_or_default();
                for g in spec {
                    if !self.houses[hi].charters.contains(&g) {
                        self.houses[hi].charters.push(g);
                        let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                        self.houses[hi].events.push(HouseEvent {
                            tick, kind: "charter".into(),
                            text: format!("{} grants a charter on {}", cn, gn),
                        });
                    }
                }
            }

            // Worst single-month loss (a sharp wealth fall — rivals, embargo, crash).
            let prev = self.houses[hi].prev_wealth;
            let drop = prev - self.houses[hi].wealth;
            if drop > 2.0 && drop > self.houses[hi].worst_loss {
                self.houses[hi].worst_loss = drop;
                self.houses[hi].events.push(HouseEvent {
                    tick, kind: "loss".into(),
                    text: format!("Its most devastating loss — {:.0} wealth lost in a month", drop),
                });
            }
            self.houses[hi].prev_wealth = self.houses[hi].wealth;
        }
    }


    /// A booming merchant city with no strong resident house occasionally spawns a
    /// new trading family.
    pub(crate) fn maybe_found_house(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        // Candidate: the richest-trade hub with ROOM for a new family (no strong
        // resident house, and fewer than 2 nascent ones so we don't stack). Also
        // track the world's max trade wealth to flag "large" hubs.
        let mut best = (usize::MAX, 0.0f32);
        let mut max_tw = 1e-6f32;
        for h in 0..self.hubs.len() {
            // Estates / manufactories are production satellites, not market cities —
            // a merchant family never seats itself there (they belong in poleis).
            if self.hubs[h].is_estate { continue; }
            max_tw = max_tw.max(self.hubs[h].trade_wealth);
            let tw = self.hubs[h].trade_wealth;
            if tw <= 0.05 { continue; } // any hub with a little trade can seed a family
            // A house SPINS OFF a guild's trade — so it only appears in a city that
            // already has a GUILD (local merchants → guild → a family separating out).
            let has_guild = self.houses.iter()
                .any(|g| !g.defunct && g.is_guild && g.hub as usize == h);
            if !has_guild { continue; }
            let count = self.houses.iter()
                .filter(|hs| !hs.defunct && hs.hub as usize == h).count() as u32;
            // Several rival families may SHARE a city (that is the competition the
            // map should show), capped by hub size. The old `strongest > 8.0` block
            // permanently locked out new houses the moment any incumbent grew past a
            // trivial wealth — collapsing every city to one dynasty and, over a
            // century, the whole world to a handful of houses. Cap only, no wealth
            // lockout: rich cities host up to 3 competing houses, smaller ones 2.
            let cap = if tw >= 0.5 * max_tw { 3 } else { 2 };
            if count >= cap { continue; }
            if tw > best.1 { best = (h, tw); }
        }
        let Some(hub) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // ── Probabilistic founding (per tick) ────────────────────────────────
        // A house does NOT auto-appear just because a city is guild-run. Per tick:
        //   • below the seeded baseline → 10% (the world repopulates its houses)
        //   • a large trade hub (>=50% of the richest hub's trade) → 5%
        //   • otherwise → 2%
        let active = self.houses.iter().filter(|h| !h.defunct && !h.is_guild).count() as u32;
        // Hard cap on live houses, and houses only start appearing from year 10 (after
        // guilds have emerged from year 5). Prefer to spin a house off a city that
        // already has a GUILD (a family separating a share of the guild's trade).
        if active as usize >= HOUSE_MAX_TOTAL { return; }
        if self.tick < HOUSE_START_YEAR * TICKS_PER_YEAR { return; }
        let target = if self.seed_house_count > 0 { self.seed_house_count } else { 24 };
        // Houses appear GRADUALLY over the first ~5 years, not all at once: the
        // effective baseline ramps linearly from a small start up to the full target,
        // so the merchant class emerges over the opening decade instead of instantly.
        let ramp = (self.tick as f32 / (HOUSE_RAMP_YEARS * TICKS_PER_YEAR as f32)).clamp(0.0, 1.0);
        let baseline = ((target as f32) * ramp).ceil() as u32;
        let large = best.1 >= 0.5 * max_tw;
        // A house is a RARE spin-off from a guild's trade (user: "small chance") — much
        // lower per-tick odds than before, so guilds stay the norm and houses the
        // exception (was 10/5/2% → a runaway 178 houses vs 2 guilds).
        let prob = if active < baseline { 0.02 } else if large { 0.01 } else { 0.004 };
        if hash01(self.seed, tick as u64 ^ 0xF0F0, hub as u64) > prob { return; }
        // Specialty = the hub's top-2 produced goods.
        let mut gi: Vec<usize> = (0..ng).collect();
        gi.sort_by(|&a, &b| self.hubs[hub].production[b]
            .partial_cmp(&self.hubs[hub].production[a]).unwrap_or(std::cmp::Ordering::Equal));
        let spec: Vec<usize> = gi.into_iter().filter(|&g| self.hubs[hub].production[g] > 0.0)
            .take(2).collect();
        if spec.is_empty() { return; }
        // ── SEED CAPITAL (Phase 0.1 fix) ─────────────────────────────────────
        // A house used to be founded with `wealth: 1.0` AND a two-or-three vessel
        // fleet. The arithmetic was fatal at birth: 2 hulls cost
        // 2·SHIP_COST·FLEET_UPKEEP_FRAC = 0.70/month, so 1.0 of capital was ~1.4
        // months of runway. The house went negative in its second month,
        // `update_solvency` started its twelve-month clock, and it died at ≈13.4
        // months. The measured median age at death was 1.1 years — the arithmetic to
        // two significant figures. 73% of all dissolutions were houses that never
        // traded at all, which is why the scorecard read ~307 dissolutions/century
        // against Greif's 30–90-year firm: the metric was counting stillbirths, not
        // failures. See `econ_diagnose_house_turnover`.
        //
        // The fix is not a bigger constant. `maybe_found_house` already REQUIRES a
        // guild at the hub — a family separating out of it — so the capital comes
        // FROM that guild, exactly as it historically did. Three properties follow:
        // no money is created; a poor guild cannot spawn a house it can't endow (the
        // churn is stopped at its source); and the seed is automatically scaled to
        // how rich the local trade actually is.
        let guild_i = self.houses.iter().position(|g| !g.defunct && g.is_guild
            && g.hub as usize == hub);
        let Some(gi_seed) = guild_i else { return };
        let seed_cap = (self.houses[gi_seed].wealth * HOUSE_SEED_GUILD_SHARE)
            .clamp(0.0, HOUSE_SEED_CAP_MAX);
        // Too poor to launch a family that can survive its own first year → no house.
        if seed_cap < HOUSE_SEED_MIN { return; }
        self.houses[gi_seed].wealth -= seed_cap;
        let name = self.unique_family_name_for(hub, tick as u64 ^ 0xF00D);
        let (line_rule, _) = self.rules_for_hub(hub);
        let female = crate::sim::inheritance::heir_is_female(line_rule, hub as u64 ^ 0x1234, self.seed);
        let head = self.head_name_sexed_for(hub, &name, tick as u64 ^ 0x1234, female);
        let (head_age, tenure) = self.roll_founder_tenure(hub as u64 ^ 0x7E);
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: hub as i32, good: spec[0] as i32, value: 0.0,
            text: format!("{} establishes {} on the {} trade", head, name,
                self.goods.get(spec[0]).map(|g| g.name.as_str()).unwrap_or("local")),
        });
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("Founded by {} in {} on the {} trade", head, self.hubs[hub].name,
                self.goods.get(spec[0]).map(|g| g.name.as_str()).unwrap_or("local")),
        };
        let (fleet_sea, fleet_river, fleet_caravan) =
            Self::initial_fleet(self.hubs[hub].coastal, false);
        self.houses.push(House {
            name, hub: hub as u32, wealth: seed_cap, prestige: 0.0, spec,
            monopoly: vec![], rivals: vec![], generation: 1,
            events: vec![founded], good_profit: Vec::new(), good_volume: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: seed_cap, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: head, head_since: tick,
            head_lifespan: tenure,
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: pick_archetype(self.seed, tick as u64 ^ hub as u64),
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
            head_female: female, head_age, line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0,
            origin_house: gi_seed as i32, origin_kind: ORIGIN_GUILD, crowned: false, realm: -1,
        });
        let ni = self.houses.len() - 1;
        self.found_head_record(ni, "founder");
    }


    pub(crate) fn dissolve_house(&mut self, hi: usize) {
        let tick = self.tick;
        let (name, hub) = (self.houses[hi].name.clone(), self.houses[hi].hub as i32);
        self.houses[hi].defunct = true;
        self.houses[hi].political_power = 0.0;
        self.houses[hi].monopoly.clear();
        // A ruined house's depots are wound up: their stock spills back onto the
        // local market (the −1 pool, stored on the hub) and the buildings are dropped.
        for w in &self.warehouses {
            if w.owner == hi as i32 && (w.hub as usize) < self.hubs.len() {
                let h = w.hub as usize;
                for g in 0..w.stock.len() {
                    stock_add_ungraded(&mut self.hubs[h].stock, g, w.stock[g]);
                }
            }
        }
        self.warehouses.retain(|w| w.owner != hi as i32);
        // Every quarrel this family was part of ends in its ruin — recorded as the
        // feud's outcome so the survivor's record shows how it won.
        self.end_feuds_of(hi);
        // Phase 4.2 · bankruptcy aftermath ("Bankruptcy has no aftermath" 1.4): a
        // failure is an event with a TAIL, not a deletion. Any bank still owed money
        // by this house writes the loss down (`Bank.losses`, already the balance
        // sheet's own write-off tally) and the loss is NAMED on both sides. Every
        // dissolution path (insolvency, a crisis outcome, plague extinction) funnels
        // through this one function, so this is a single point of coverage for all
        // of them. **Cut from the design's own "small" scope**: kin barred from
        // office in that city for a period is NOT built — it would need new per-city
        // state (a wide-blast-radius `TickHub` field, touching every hub-construction
        // site the way the House-field patches already do for House) for a minor
        // flavour detail the source doc itself calls small; not worth that risk here.
        let mut creditors: Vec<(String, f32)> = Vec::new();
        for b in self.banks.iter_mut() {
            if b.defunct { continue; }
            let mut lost = 0.0f32;
            for l in b.loans.iter_mut() {
                if l.borrower_house == hi as i32 && l.outstanding > 0.0 {
                    lost += l.outstanding;
                    l.outstanding = 0.0;
                }
            }
            if lost > 0.0 {
                b.losses += lost;
                b.events.push(HouseEvent {
                    tick, kind: "bad_debt".into(),
                    text: format!("writes off {:.0} owed by {}, now dissolved", lost, name),
                });
                creditors.push((b.name.clone(), lost));
            }
        }
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "dissolved".into(),
            text: if creditors.is_empty() {
                "Fell into ruin and was dissolved".into()
            } else {
                format!("Fell into ruin and was dissolved, leaving {} owed to {}",
                    creditors.iter().map(|(_, v)| format!("{:.0}", v)).collect::<Vec<_>>().join("+"),
                    creditors.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>().join(", "))
            },
        });
        self.journal.push(JournalEntry {
            tick, kind: "extinction".into(), hub, good: -1, value: 0.0,
            text: format!("{} falls into ruin and is dissolved", name),
        });
    }
}

// ── Phase 2.6 · a house's internal power shares ──────────────────────────────────
// Pure functions (no `&self`) so they're callable from a bridge command AND directly
// gated by a test — the whole point of "power shares always sum to 100" is that it
// must hold for ANY roster, not just ones the tick happens to produce.

/// Per-kin power weight (before normalising): role first, then skill and loyalty as
/// modest multipliers — a capable, loyal factor counts for more than an idle kin, but
/// role is what mostly decides a hand in the family's affairs.
const KIN_ROLE_WEIGHT: [f32; 6] = [3.0, 1.6, 1.2, 0.8, 0.0, 0.0]; // head·heir·factor·idle·married out·dead

/// Each kin's share of the house's internal "power", 0..100, summing to exactly 100
/// across the roster (or empty for an empty roster — there is no house to share power
/// in). This is READ-ONLY: nothing in the tick consults it; it exists for the dossier.
pub fn kin_power_shares(kin: &[Kin]) -> Vec<f32> {
    if kin.is_empty() { return Vec::new(); }
    let raw: Vec<f32> = kin.iter().map(|k| {
        let w = KIN_ROLE_WEIGHT.get(k.role.min(5) as usize).copied().unwrap_or(0.0);
        w * (0.5 + k.skill.clamp(0.0, 1.0) * 0.5) * (0.5 + k.loyalty.clamp(0.0, 1.0) * 0.5)
    }).collect();
    let total: f32 = raw.iter().sum();
    if total <= 1e-9 {
        // Every weight rounded to zero (an all-dead/married-out roster) — split evenly
        // rather than divide by zero, so the invariant still holds.
        let even = 100.0 / kin.len() as f32;
        return vec![even; kin.len()];
    }
    let mut shares: Vec<f32> = raw.iter().map(|&r| r / total * 100.0).collect();
    // Floating-point division won't sum to EXACTLY 100 — hand the residual to the
    // largest share, which is where a rounding error is least noticeable.
    let sum: f32 = shares.iter().sum();
    if let Some((i, _)) = shares.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
        shares[i] += 100.0 - sum;
    }
    shares
}

const CHAR_ADJ: [[&str; 4]; 4] = [
    ["hoarding", "cautious", "bold", "reckless"],
    ["scrupulous", "honourable", "grasping", "ruthless"],
    ["close-fisted", "private", "civic-minded", "openhanded"],
    ["insular", "rooted", "expansive", "far-reaching"],
];

/// A phrase from the four character axes (§3: "presented as a phrase, never four
/// numbers" — the same discipline the stability gauges use). Only axes that read as
/// notable (|value| >= 1) appear, most extreme first; a middling character says
/// nothing, same as a healthy gauge staying quiet.
pub fn character_phrase(c: [i8; 4]) -> String {
    let mut notable: Vec<(usize, i8)> = c.iter().enumerate()
        .filter(|&(_, &v)| v != 0).map(|(i, &v)| (i, v)).collect();
    notable.sort_by(|a, b| b.1.abs().cmp(&a.1.abs()));
    notable.truncate(3);
    if notable.is_empty() { return String::new(); }
    let words: Vec<&str> = notable.iter().map(|&(axis, v)| {
        // v is one of {-2,-1,1,2} (0 filtered out above) -> index {0,1,2,3}.
        let pole = if v > 0 { v + 1 } else { v + 2 } as usize;
        CHAR_ADJ[axis][pole.min(3)]
    }).collect();
    let mut phrase = words.join(", ");
    if let Some(ch) = phrase.get_mut(0..1) { ch.make_ascii_uppercase(); }
    format!("{}.", phrase)
}
