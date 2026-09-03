//! `docs/YARDS_VESSELS_AND_DEPOTS_PLAN.md` — Part I (the yard, S1-S3) and the
//! depot slices of Part II (W2-W5) that are safe to ship additively.
//!
//! **D5's order — measurement → structure → identity → ownership → economics —
//! is followed literally.** S1 (the yard estate + material draw), S2 (the
//! `Vessel` list) and S3 (fractional shares on a newly-built hull) are pure
//! STRUCTURE: they add real bookkeeping (a hull genuinely gets built, genuinely
//! gets shared out) but nothing downstream of `dispatch`/`decide_fleets` reads
//! any of it yet, so none of it can move `econ_`'s numbers — the same
//! "additive, currently inert" shape this codebase already uses for goals
//! (Phase 3.1) and the crisis engine's earlier slices. The two mechanisms that
//! actually change what the campaign DOES — S4 (capacity binding) and the
//! guild axis — and the three depot slices that touch dispatch's supply
//! (`LANDED_CARGO_TO_DEPOT_DOSE`, `WH_RELEASE_DOSE`,
//! `DEPOT_TO_DEPOT_TRANSFER_ENABLED`) all ship DOSE-WALKED at their zero/no-op
//! setting, exactly as §2.8's own routing-table discipline and this plan's own
//! §6 caveat ("S4 and S5 are the only slices that can move the fidelity
//! numbers, and both ship at zero dose") require. W5 (the fondaco) ships as a
//! structural stub only — `N2` broke the hard wealth bound twice on a
//! market-closure mechanism of this exact shape, and the plan's own risk
//! register says to zero-dose it first.
use super::*;

impl CampaignSim {
    /// S1 · a coastal or river city large enough to matter, with no yard yet,
    /// gets one — owned by its craft guild if it has one, else its strongest
    /// resident house, else the city itself (mirrors `maybe_found_guild_
    /// workshop`'s ownership rule exactly). At most one per call, same
    /// "legible chronicle event" discipline every founding pass here uses.
    pub(crate) fn maybe_found_yards(&mut self) {
        if self.estate_count() >= MAX_TOTAL_ESTATES.saturating_sub(OUTPOST_RESERVED_ESTATES) { return; }
        let n = self.hubs.len();
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..n {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || hub.population < YARD_MIN_POP { continue; }
            if !(hub.coastal || hub.river) { continue; }
            if self.hubs.iter().any(|e| e.is_estate && e.estate_kind == YARD_ESTATE_KIND && e.parent == h as i32) {
                continue;
            }
            let score = hub.population;
            if score > best.1 { best = (h, score); }
        }
        let (h, _) = best;
        if h == usize::MAX { return; }
        let owner = self.guild_or_strong_owner_at(h);
        // D1 — a yard is founded on WHATEVER hull material this city already
        // has (grown or imported); it names its estate's dominant good after
        // whichever bulk timber good exists in this goods spec, purely for
        // the journal/inspector label. Neither existing means no goods spec
        // supports shipbuilding at all here, which is a world config choice,
        // not a bug — the founding still happens, tagged on index 0.
        let g0 = self.goods.iter().position(|g| g.name == "timber")
            .or_else(|| self.goods.iter().position(|g| g.name == "hardwoods"))
            .unwrap_or(0);
        let est_pop = self.hubs[h].founding_pop * 0.05;
        let off = hash01(self.seed, self.tick as u64 ^ 0x7A2D, h as u64);
        let ex = self.hubs[h].x + (off - 0.5) * self.world_w * 0.02;
        let ey = self.hubs[h].y
            + (hash01(self.seed, h as u64, self.tick as u64 ^ 0x7A2E) - 0.5) * self.world_w * 0.02;
        let (koppen, coastal, component) =
            (self.hubs[h].koppen, self.hubs[h].coastal, self.hubs[h].component);
        self.create_estate(h as i32, ex, ey, g0, YARD_ESTATE_KIND, owner, koppen, coastal, component,
            est_pop, 0.05);
        let city = self.hubs[h].name.clone();
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "estate".into(), hub: h as i32, good: g0 as i32, value: 7.0,
            text: format!("A shipyard opens at {}", city),
        });
    }

    /// S1 · monthly: every yard draws down its parent city's LOCAL SURPLUS of
    /// hull material (D1 — hardwoods/timber, both `GOOD_UNLIMITED`, so the
    /// draw binds nowhere a suitable climate exists — and never a fixed
    /// recipe) into `TickHub.yard_progress`. A hull completes once progress
    /// clears `HULL_BUILD_POINTS`, at which point `spawn_vessel` (S2/S3)
    /// subscribes the parent city's own houses into it. Pitch/hemp, if
    /// present, only raise the hull's `quality` — D1's "the mix sets what
    /// KIND of vessel comes out, never whether one can be built".
    pub(crate) fn yard_build_pass(&mut self) {
        let ng = self.goods.len();
        let n = self.hubs.len();
        let timber_g = self.goods.iter().position(|g| g.name == "timber");
        let hardwoods_g = self.goods.iter().position(|g| g.name == "hardwoods");
        let pitch_g = self.goods.iter().position(|g| g.name == "pitch");
        let hemp_g = self.goods.iter().position(|g| g.name == "hemp");
        for ei in 0..n {
            if !self.hubs[ei].is_estate || self.hubs[ei].estate_kind != YARD_ESTATE_KIND { continue; }
            let parent = self.hubs[ei].parent;
            if parent < 0 || parent as usize >= n { continue; }
            let parent = parent as usize;
            let mut drawn = 0.0f32;
            for gopt in [hardwoods_g, timber_g] {
                let Some(g) = gopt else { continue };
                if g >= ng { continue; }
                let surplus = stock_of(&self.hubs[parent].stock, g);
                if surplus <= EPS { continue; }
                let take = (surplus * YARD_MATERIAL_DRAW_FRAC).min(HULL_BUILD_POINTS);
                if take <= EPS { continue; }
                drawn += stock_take(&mut self.hubs[parent].stock, g, take);
            }
            // `a_yard_with_no_material_builds_nothing` — no draw, no progress.
            if drawn <= EPS { continue; }
            let have_pitch = pitch_g.is_some_and(|g| g < ng && stock_of(&self.hubs[parent].stock, g) > EPS);
            let have_hemp = hemp_g.is_some_and(|g| g < ng && stock_of(&self.hubs[parent].stock, g) > EPS);
            self.hubs[ei].yard_progress += drawn;
            if self.hubs[ei].yard_progress >= HULL_BUILD_POINTS {
                self.hubs[ei].yard_progress -= HULL_BUILD_POINTS;
                let quality: f32 = 0.6 + if have_pitch { 0.2 } else { 0.0 } + if have_hemp { 0.2 } else { 0.0 };
                let sea = self.hubs[ei].coastal;
                self.spawn_vessel(parent, sea, quality.min(1.0));
            }
        }
    }

    /// S2/S3 · a hull is complete — record it as a `Vessel` and subscribe the
    /// parent city's solvent resident houses into its `parts` (D3), richest
    /// first, weighted by wealth, capped to the 4 largest so a share stays
    /// meaningful. If no resident house can afford a stake, the hull is still
    /// recorded with an EMPTY `parts` list — city property, the same
    /// convention a city-owned estate's `owner_house == -1` already uses —
    /// rather than being silently dropped.
    fn spawn_vessel(&mut self, parent: usize, sea: bool, quality: f32) {
        let tick = self.tick;
        let capacity = if sea { SHIP_CAPACITY } else { BOAT_CAPACITY };
        let mut candidates: Vec<(usize, f32)> = self.houses.iter().enumerate()
            .filter(|(_, h)| !h.defunct && h.hub as usize == parent && h.wealth > SHIP_COST)
            .map(|(i, h)| (i, h.wealth.max(1.0)))
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(4);
        let mut parts: Vec<VesselShare> = Vec::new();
        if !candidates.is_empty() {
            let total_w: f32 = candidates.iter().map(|&(_, w)| w).sum();
            let mut assigned = 0u8;
            let last = candidates.len() - 1;
            for (idx, &(hi, w)) in candidates.iter().enumerate() {
                let share = if idx == last {
                    VESSEL_PARTS_TOTAL - assigned
                } else {
                    (((w / total_w) * VESSEL_PARTS_TOTAL as f32).round() as u8).min(VESSEL_PARTS_TOTAL - assigned)
                };
                if share == 0 { continue; }
                assigned += share;
                parts.push(VesselShare { house: hi as u32, parts: share });
            }
            if assigned < VESSEL_PARTS_TOTAL {
                if let Some(first) = parts.first_mut() { first.parts += VESSEL_PARTS_TOTAL - assigned; }
            }
        }
        let id = self.next_vessel_id;
        self.next_vessel_id += 1;
        let kind_word = if sea { "hull" } else { "river boat" };
        let city = self.hubs[parent].name.clone();
        self.vessels.push(Vessel {
            id, name: format!("{} {} No.{}", city, kind_word, id),
            kind: if sea { 0 } else { 1 }, home_hub: parent as u32, at_hub: parent as u32,
            capacity, quality, condition: 1.0, parts, built_tick: tick,
        });
        self.journal.push(JournalEntry {
            tick, kind: "vessel".into(), hub: parent as i32, good: -1, value: capacity,
            text: format!("A new {} is launched at {}", kind_word, city),
        });
    }

    /// S2 · seed one `Vessel` per pre-existing sea/river hull counter, at
    /// campaign start, so the representation change is `seeding_one_whole_
    /// hull_per_counter_is_bit_identical`: every house's summed vessel
    /// capacity equals `fleet_sea * SHIP_CAPACITY` / `fleet_river *
    /// BOAT_CAPACITY` exactly. Caravans are deliberately NOT seeded (D4 —
    /// overland capacity is hired, not owned; `fleet_caravan` stays a bare
    /// counter). Idempotent: a second call on an already-seeded sim is a
    /// no-op, so it is safe to call unconditionally from `campaign_start_sim`.
    pub(crate) fn seed_vessels_from_fleets(&mut self) {
        if !self.vessels.is_empty() || self.next_vessel_id != 0 { return; }
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            let home = self.houses[hi].hub as usize;
            let city = self.hubs.get(home).map(|h| h.name.clone()).unwrap_or_default();
            for _ in 0..self.houses[hi].fleet_sea {
                let id = self.next_vessel_id; self.next_vessel_id += 1;
                self.vessels.push(Vessel {
                    id, name: format!("{} hull No.{}", city, id), kind: 0,
                    home_hub: home as u32, at_hub: home as u32, capacity: SHIP_CAPACITY,
                    quality: 1.0, condition: 1.0,
                    parts: vec![VesselShare { house: hi as u32, parts: VESSEL_PARTS_TOTAL }],
                    built_tick: tick,
                });
            }
            for _ in 0..self.houses[hi].fleet_river {
                let id = self.next_vessel_id; self.next_vessel_id += 1;
                self.vessels.push(Vessel {
                    id, name: format!("{} river boat No.{}", city, id), kind: 1,
                    home_hub: home as u32, at_hub: home as u32, capacity: BOAT_CAPACITY,
                    quality: 1.0, condition: 1.0,
                    parts: vec![VesselShare { house: hi as u32, parts: VESSEL_PARTS_TOTAL }],
                    built_tick: tick,
                });
            }
        }
    }

    /// A house's total vessel capacity, by kind (0 = sea, 1 = river), summed
    /// from its `parts` share of every `Vessel` it holds a stake in — the
    /// derived reading `seeding_one_whole_hull_per_counter_is_bit_identical`
    /// checks against the bare `fleet_sea`/`fleet_river` counters. Not yet
    /// called by `dispatch` (S4 is dosed at zero) or `decide_fleets`.
    #[allow(dead_code)]
    pub(crate) fn house_vessel_capacity(&self, house: usize, kind: u8) -> f32 {
        self.vessels.iter().filter(|v| v.kind == kind)
            .map(|v| {
                let frac: f32 = v.parts.iter()
                    .filter(|p| p.house as usize == house)
                    .map(|p| p.parts as f32 / VESSEL_PARTS_TOTAL as f32)
                    .sum();
                v.capacity * frac
            })
            .sum()
    }

    /// S3 · lose a vessel entirely (wreck, war, age). Debits every part
    /// owner proportionally to its stake, each capped to half that house's
    /// current wealth — fractional ownership's whole reason for existing
    /// (D3): a bad voyage is a setback for several houses, never a ruin for
    /// one. Not yet wired into `damage_fleet`/war spoils (those still act on
    /// the bare counters, per S1's "nothing consumes it yet").
    #[allow(dead_code)]
    pub(crate) fn lose_vessel(&mut self, vi: usize) {
        if vi >= self.vessels.len() { return; }
        let v = self.vessels.remove(vi);
        let unit_cost = if v.kind == 0 { SHIP_COST } else { RIVER_COST };
        let value = unit_cost * (v.capacity / if v.kind == 0 { SHIP_CAPACITY } else { BOAT_CAPACITY }).max(0.0);
        for sh in &v.parts {
            let hi = sh.house as usize;
            if hi >= self.houses.len() { continue; }
            let loss = value * (sh.parts as f32 / VESSEL_PARTS_TOTAL as f32);
            let cap = self.houses[hi].wealth.max(0.0) * 0.5;
            self.houses[hi].wealth -= loss.min(cap);
        }
    }

    /// S4, dose-walked (`CAPACITY_BIND_DOSE`) · how many EXTRA vessel slots a
    /// shipment of `amount` needs beyond the one every shipment already
    /// reserves (F4 — today a shipment consumes one slot "regardless of
    /// quantity"). At `CAPACITY_BIND_DOSE == 0.0` this is always 0 — a proven
    /// no-op (`n_yards_s4_capacity_bind_at_zero_is_a_noop`) — so `dispatch`'s
    /// existing one-slot rule is exactly what a house pays until this is
    /// dosed above zero.
    pub(crate) fn capacity_bind_extra_slots(&self, amount: f32, unit_capacity: f32) -> i32 {
        if CAPACITY_BIND_DOSE <= 0.0 || unit_capacity <= EPS { return 0; }
        (amount * CAPACITY_BIND_DOSE / unit_capacity).floor().max(0.0) as i32
    }

    // ── Part II depot slices ────────────────────────────────────────────

    /// W3, dose-walked (`WH_RELEASE_DOSE`) · the release verb F6 says a depot
    /// never has: once a hub's price for a warehouse-held good clears
    /// `WH_RELEASE_PRICE_MULT × base_value`, the depot sells a slice back into
    /// the pool at that price — the house's speculative payoff, symmetric to
    /// the stocking cost `sync_and_stock_warehouses` already charges it. At
    /// `WH_RELEASE_DOSE == 0.0` this is a proven no-op
    /// (`n_yards_w3_release_at_zero_is_a_noop`).
    pub(crate) fn warehouse_release_pass(&mut self, needs: &[Vec<f32>]) {
        if WH_RELEASE_DOSE <= 0.0 { return; }
        let ng = self.goods.len();
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= self.houses.len() || self.houses[oi].defunct { continue; }
            let hub = self.warehouses[wi].hub as usize;
            if hub >= self.hubs.len() || hub >= needs.len() { continue; }
            for g in 0..ng.min(self.warehouses[wi].stock.len()) {
                let held = self.warehouses[wi].stock[g];
                if held <= EPS { continue; }
                let base = self.goods[g].base_value.max(EPS);
                let price = self.live_price(self.hub_stock(hub, g), needs[hub][g], base);
                if price < base * WH_RELEASE_PRICE_MULT { continue; }
                let release = held * WH_RELEASE_DOSE;
                if release <= EPS { continue; }
                self.warehouses[wi].stock[g] -= release;
                stock_add_ungraded(&mut self.hubs[hub].stock, g, release);
                self.houses[oi].wealth += release * price;
            }
        }
    }

    /// W2, dose-walked (`LANDED_CARGO_TO_DEPOT_DOSE`) · a house-owned arrival
    /// splits between the carrier's own depot at the destination (room
    /// permitting) and the undifferentiated pool, instead of landing wholly
    /// in the pool (F8). At `LANDED_CARGO_TO_DEPOT_DOSE == 0.0` every unit
    /// still lands in the pool — a proven no-op
    /// (`n_yards_w2_landed_cargo_to_depot_at_zero_is_a_noop`). Returns the
    /// amount actually diverted into a depot, so the caller adds the
    /// remainder to the pool exactly as before.
    pub(crate) fn landed_cargo_to_depot(&mut self, to: usize, g: usize, amt: f32, owner: i32) -> f32 {
        if LANDED_CARGO_TO_DEPOT_DOSE <= 0.0 || owner < 0 { return 0.0; }
        let target = amt * LANDED_CARGO_TO_DEPOT_DOSE;
        if target <= EPS { return 0.0; }
        let Some(wi) = self.warehouses.iter().position(|w| w.owner == owner && w.hub as usize == to) else { return 0.0 };
        let used: f32 = self.warehouses[wi].stock.iter().sum();
        let room = (self.warehouses[wi].capacity - used).max(0.0);
        let take = target.min(room);
        if take <= EPS { return 0.0; }
        if let Some(slot) = self.warehouses[wi].stock.get_mut(g) { *slot += take; }
        take
    }

    /// W4, dose-walked (`DEPOT_TO_DEPOT_TRANSFER_ENABLED`) · an office ships a
    /// slice of one of its own depots' surplus to another of its own depots
    /// on its own account — needs S2's `Vessel`s to mean a real location
    /// (the plan's "the one dependency worth naming"). Disabled by default;
    /// the function still runs (and immediately returns) so its own
    /// no-op gate exercises the real code path rather than a dead branch.
    #[allow(dead_code)]
    pub(crate) fn depot_to_depot_transfer_pass(&mut self) {
        if !DEPOT_TO_DEPOT_TRANSFER_ENABLED { return; }
        // Real transfer logic is future work once S2's vessels carry a real
        // location — left unimplemented on purpose rather than faked, per
        // this plan's own D5 ordering.
    }

    /// W5, zero-dose stub (`Fondaco`) · never founds one (`FONDACO_FORM_CHANCE
    /// == 0.0`). Exists so the data shape and the call site are real and a
    /// future session can dose it up without another migration, while a
    /// world with no fondaco stays exactly as it is today — the same
    /// "structure first, mechanism later" discipline S1-S3 use, applied to
    /// the plan's highest-risk piece (§3's own note that `N2` broke the hard
    /// wealth bound twice on a market-closure mechanism of this shape).
    #[allow(dead_code)]
    pub(crate) fn maybe_found_fondaco(&mut self) {
        const FONDACO_FORM_CHANCE: f32 = 0.0;
        if FONDACO_FORM_CHANCE <= 0.0 { return; }
    }
}
