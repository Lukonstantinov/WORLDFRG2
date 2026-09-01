//! colonies — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

impl CampaignSim {

    /// Monthly · a prosperous, CROWDED polis with treasury to spare SPONSORS
    /// Append a fading migration arrow (a "refugee road" / drift line) for the map,
    /// bounded so the snapshot stays small.
    pub(crate) fn push_migration_arrow(&mut self, sx: f32, sy: f32, tx: f32, ty: f32) {
        self.migrations.push([sx, sy, tx, ty, self.tick as f32]);
        if self.migrations.len() > MIGRATION_ARROW_CAP {
            let excess = self.migrations.len() - MIGRATION_ARROW_CAP;
            self.migrations.drain(0..excess);
        }
    }


    /// Emit a route-bound migration flow: trace the trade-route polyline from `src`→`dst`
    /// and record it (culture + volume) for the reworked overlay. Falls back to a legacy
    /// endpoint arrow too, so the old overlay stays populated. Returns whether a route
    /// existed (callers treat "no route" as "no move" when they want strict routing).
    pub(crate) fn emit_migration_route(&mut self, src: usize, dst: usize, culture: &str, volume: f32) -> bool {
        let Some(chain) = self.neighbor_path(src, dst) else { return false; };
        let path: Vec<[f32; 2]> = chain.iter().map(|&h| [self.hubs[h].x, self.hubs[h].y]).collect();
        let (sx, sy) = (self.hubs[src].x, self.hubs[src].y);
        let (dx, dy) = (self.hubs[dst].x, self.hubs[dst].y);
        self.migration_routes.push(MigrationRoute {
            path, culture: culture.to_string(), volume, tick: self.tick,
            from_hub: src as i32, to_hub: dst as i32,
        });
        if self.migration_routes.len() > MIGRATION_ROUTE_CAP {
            let excess = self.migration_routes.len() - MIGRATION_ROUTE_CAP;
            self.migration_routes.drain(0..excess);
        }
        self.push_migration_arrow(sx, sy, dx, dy);
        true
    }


    /// Cultures · the city's MAJORITY people is whoever actually has the largest share
    /// (plurality). After migration/assimilation a minority quarter can outgrow the
    /// old majority — when it does, promote it and demote the former majority to a
    /// quarter, so the "majority" the UI shows is always the dominant people (fixes a
    /// founding people staying nominal majority at 2% while incomers hold 74%).
    pub(crate) fn rebalance_hub_majorities(&mut self) {
        let n = self.hubs.len();
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.hub_minorities[h].is_empty() { continue; }
            let maj = match self.hub_culture.get(h) {
                Some(c) if !c.is_empty() && c != "—" => c.clone(),
                _ => continue,
            };
            let minsum: f32 = self.hub_minorities[h].iter().map(|(_, s)| *s).sum();
            let maj_share = (1.0 - minsum).clamp(0.0, 1.0);
            // largest minority quarter
            let mut bi = 0usize; let mut bshare = -1.0f32;
            for (i, (_, s)) in self.hub_minorities[h].iter().enumerate() {
                if *s > bshare { bshare = *s; bi = i; }
            }
            if bshare > maj_share + 1e-4 {
                let new_maj = self.hub_minorities[h][bi].0.clone();
                self.hub_minorities[h].remove(bi);
                if maj_share > 0.005 { self.hub_minorities[h].push((maj, maj_share)); }
                self.hub_culture[h] = new_maj;
                self.hub_minorities[h].sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
    }


    /// Migrants carry their home culture. Where it differs from the destination's
    /// majority, it swells a minority quarter there (as a share of destination pop).
    pub(crate) fn record_migration_culture(&mut self, dest: usize, src: usize, movers: f32) {
        let src_culture = match self.hub_culture.get(src) {
            Some(c) if !c.is_empty() && c != "—" => c.clone(),
            _ => return,
        };
        self.add_minority(dest, &src_culture, movers);
    }


    /// Grow a culture's minority quarter at `dest` by `movers` people (no-op if it
    /// is already the majority culture there).
    pub(crate) fn add_minority(&mut self, dest: usize, culture: &str, movers: f32) {
        if dest >= self.hub_minorities.len() || culture.is_empty() || culture == "—" { return; }
        if self.hub_culture.get(dest).map(|c| c == culture).unwrap_or(false) { return; }
        let dest_pop = self.hubs[dest].population.max(1.0);
        let add = (movers / dest_pop).clamp(0.0, 0.95);
        if add <= 0.0 { return; }
        let list = &mut self.hub_minorities[dest];
        if let Some(e) = list.iter_mut().find(|(c, _)| c == culture) {
            e.1 = (e.1 + add).min(0.95);
        } else {
            list.push((culture.to_string(), add));
        }
    }


    /// #23 · Yearly DIASPORA spread: a travel-prone culture present in a city (as its
    /// majority OR an existing minority — enabling chain migration) sends a trickle
    /// of settlers ALONG a trade tie to a partner city, seeding/growing a minority
    /// quarter there. So merchant peoples visibly spread across the trade map.
    pub(crate) fn diaspora_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let mut sent = 0u32;
        for src in 0..n {
            if sent >= DIASPORA_MAX_PER_YEAR { break; }
            if self.hubs[src].is_estate || self.hubs[src].abandoned { continue; }
            if self.hubs[src].population < DIASPORA_MIN_POP { continue; }
            // The most travel-prone culture PRESENT here (majority or minority).
            let mut cand: Option<(String, f32)> = None;
            if let Some(maj) = self.hub_culture.get(src) {
                let m = Self::culture_mobility(maj);
                if m >= DIASPORA_MOBILITY_GATE { cand = Some((maj.clone(), m)); }
            }
            if let Some(mins) = self.hub_minorities.get(src) {
                for (c, s) in mins {
                    if *s < 0.02 { continue; }
                    let m = Self::culture_mobility(c);
                    if m >= DIASPORA_MOBILITY_GATE && cand.as_ref().map_or(true, |(_, cm)| m > *cm) {
                        cand = Some((c.clone(), m));
                    }
                }
            }
            let Some((culture, mob)) = cand else { continue; };
            if hash01(self.seed, tick as u64 ^ 0xD1A5, src as u64) > 0.15 * mob { continue; }
            // Destination: a TRADE PARTNER (route neighbour — never a direct geographic
            // jump) where this culture is still under-represented.
            let max_cells = MIGRATION_MAX_KM / (EARTH_EQUATOR_KM / self.world_w.max(1.0)).max(1e-3);
            let dst = self.neighbors.get(src).and_then(|v| v.iter().map(|&x| x as usize)
                .find(|&d| d < n && d != src && !self.hubs[d].is_estate && !self.hubs[d].abandoned
                    && self.hub_cell_dist(src, d) <= max_cells
                    && self.culture_share_at(d, &culture) < DIASPORA_MAX_MINORITY));
            let Some(dst) = dst else { continue; };
            let movers = (self.hubs[src].population * DIASPORA_SEND_FRAC).clamp(5.0, 200.0);
            self.hubs[src].population = (self.hubs[src].population - movers)
                .max(self.hubs[src].founding_pop * 0.3);
            self.hubs[dst].population += movers;
            self.add_minority(dst, &culture, movers);
            self.emit_migration_route(src, dst, &culture, movers); // 1-hop trade tie
            sent += 1;
            if hash01(self.seed, tick as u64 ^ 0x9A17, src as u64) < 0.10 {
                let dn = self.hubs[dst].name.clone();
                self.journal.push(JournalEntry { tick, kind: "migration".into(), hub: dst as i32, good: -1,
                    value: movers, text: format!("{:.0} {} traders settle in {}", movers, culture, dn) });
            }
        }
    }


    /// Yearly HINTERLAND pass: connect every sub-cap village to the trade network by
    /// tying it to its nearest live market town (satellite trade), and let that town
    /// earn a small, bounded civic toll from the hinterland trade. This is how villages
    /// below the sim cap "join the economy" without becoming O(n²) full hubs — they feed
    /// a real hub and show up in its books, instead of sitting inert and disconnected.
    pub(crate) fn hinterland_pass(&mut self) {
        let n = self.hubs.len();
        if self.hinterland.is_empty() || n == 0 { return; }
        // (Re)assign each village its nearest live market town — only when unlinked or
        // its town has died (cheap no-op once every village is settled on a live hub).
        for ti in 0..self.hinterland.len() {
            let cur = self.hinterland[ti].parent_hub;
            let ok = cur >= 0 && (cur as usize) < n
                && !self.hubs[cur as usize].is_estate && !self.hubs[cur as usize].abandoned;
            if ok { continue; }
            let (vx, vy) = (self.hinterland[ti].x, self.hinterland[ti].y);
            let mut best = (-1i32, f32::MAX);
            for h in 0..n {
                if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
                let mut dx = (self.hubs[h].x - vx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = self.hubs[h].y - vy;
                let d = dx * dx + dy * dy;
                if d < best.1 { best = (h as i32, d); }
            }
            self.hinterland[ti].parent_hub = best.0;
        }
        // Market toll: each town earns a small civic income from its satellite villages.
        for ti in 0..self.hinterland.len() {
            let p = self.hinterland[ti].parent_hub;
            if p < 0 || (p as usize) >= n { continue; }
            self.hubs[p as usize].civic_pool += self.hinterland[ti].population * HINTERLAND_TOLL;
        }
        // Villages BREATHE: grow toward a modest ceiling under a thriving, well-fed market,
        // slip when their market starves or has died — so the sub-cap dots aren't frozen.
        for ti in 0..self.hinterland.len() {
            let p = self.hinterland[ti].parent_hub;
            let (prosp, food, alive) = if p >= 0 && (p as usize) < n {
                let hb = &self.hubs[p as usize];
                (hb.sent_prosperity.clamp(0.0, 1.0), hb.sent_food.clamp(0.0, 1.0), !hb.abandoned && hb.starving < 0.5)
            } else { (0.15, 0.4, false) };
            let pop = self.hinterland[ti].population.max(1.0);
            let cap = (HINTERLAND_BASE_CAP * (0.4 + 1.4 * prosp) * (0.5 + 0.5 * food)).max(50.0);
            let np = if alive && pop < cap {
                pop + HINTERLAND_GROWTH * pop * (1.0 - pop / cap)
            } else {
                pop * (1.0 - HINTERLAND_DECLINE)
            };
            self.hinterland[ti].population = np.clamp(0.0, cap.max(pop));
        }
    }


    /// The lingua-franca language family of a trade region, if one has emerged.
    pub(crate) fn lingua_family_for(&self, comp: u32) -> Option<&str> {
        self.lingua.iter().find(|l| l.component == comp).map(|l| l.family.as_str())
    }


    /// Cultures 2.0 · recompute each trade region's LINGUA FRANCA — the tongue of the
    /// culture that dominates the region's cities (population-weighted). Emerges at
    /// `LINGUA_DOMINANCE`; if no culture dominates any more, the old tongue LINGERS as
    /// a legacy trade language (Latin after Rome) rather than vanishing.
    pub(crate) fn compute_lingua(&mut self) {
        use std::collections::HashMap;
        let yr = self.tick / TICKS_PER_YEAR;
        let mut tally: HashMap<u32, HashMap<String, f32>> = HashMap::new();
        let mut total: HashMap<u32, f32> = HashMap::new();
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let cul = self.hub_culture.get(h).cloned().unwrap_or_default();
            if cul.is_empty() || cul == "—" { continue; }
            let comp = self.hubs[h].component;
            let w = self.hubs[h].population.max(0.0);
            *tally.entry(comp).or_default().entry(cul).or_default() += w;
            *total.entry(comp).or_default() += w;
        }
        let mut next: Vec<LinguaFranca> = Vec::new();
        // DETERMINISM: iterate components in KEY order (the Vec built below is kept), and
        // break the max_by tie on the culture NAME — two peoples of equal weight would
        // otherwise pick a different lingua franca depending on hash order.
        let mut comps: Vec<u32> = tally.keys().copied().collect();
        comps.sort_unstable();
        for comp in &comps {
            let cultures = &tally[comp];
            let tot = total.get(comp).copied().unwrap_or(0.0).max(1.0);
            let (best_cul, best_w) = cultures.iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                    .then(b.0.cmp(a.0)))
                .map(|(c, w)| (c.clone(), *w)).unwrap_or_default();
            let share = best_w / tot;
            let prev = self.lingua.iter().find(|l| l.component == *comp).cloned();
            if share >= LINGUA_DOMINANCE {
                let family = self.culture_family(&best_cul);
                if family.is_empty() { continue; }
                let (since, shifted) = match &prev {
                    Some(p) if p.family == family => (p.since_year, false),
                    _ => (yr, true),
                };
                if shifted {
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "culture".into(), hub: -1, good: -1, value: share,
                        text: format!("The tongue of {} becomes the trade language of its region.", best_cul),
                    });
                }
                next.push(LinguaFranca { component: *comp, family, culture: best_cul, share, since_year: since, legacy: false });
            } else if let Some(mut p) = prev {
                // No culture dominates now → the established tongue lingers as a relic.
                if !p.legacy {
                    let cul = p.culture.clone();
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "culture".into(), hub: -1, good: -1, value: share,
                        text: format!("The {} tongue endures as the region's trade language though its people no longer rule.", cul),
                    });
                }
                p.legacy = true; p.share = share;
                next.push(p);
            }
        }
        self.lingua = next;
    }


    /// VIGOROUS YOUTH (see the doc comment on `CREOLE_VIGOR_YEARS` in `mod.rs`) — how
    /// much of its birth vigor a creole culture has left, 1.0 at `born_tick` decaying
    /// linearly to 0.0 by `CREOLE_VIGOR_YEARS` later. 0.0 for anything that isn't a
    /// live creole (a hearth culture, an unknown name), so every call site is a true
    /// no-op off the creole path.
    pub(crate) fn creole_vigor(&self, name: &str) -> f32 {
        let Some(cr) = self.creoles.iter().find(|c| c.name == name) else { return 0.0; };
        let age_years = self.tick.saturating_sub(cr.born_tick) as f32 / TICKS_PER_YEAR as f32;
        (1.0 - age_years / CREOLE_VIGOR_YEARS).clamp(0.0, 1.0)
    }


    /// Cultures 2.0 · Yearly assimilation — minority quarters blend into the majority,
    /// FASTER when they share a language family with the local majority (mutual
    /// intelligibility) and in big, prosperous "melting-pot" cities; SLOWER across
    /// distant families. A regional LINGUA FRANCA bridges distant families. Bounded.
    /// A freshly-formed creole majority/minority additionally carries its VIGOR bonus
    /// (see `creole_vigor`) — it resists dissolving as a minority and, while it holds
    /// a hub's majority, pulls other minorities into itself faster.
    pub(crate) fn assimilation_pass(&mut self) {
        let n = self.hubs.len();
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.hub_minorities[h].is_empty() { continue; }
            let maj_name = self.hub_culture.get(h).cloned().unwrap_or_default();
            let maj_fam = self.culture_family(&maj_name);
            let maj_env = crate::sim::cultures::people_env(&maj_name);
            // If the local majority speaks the region's trade tongue, that lingua franca
            // bridges assimilation for everyone here — even distant language families.
            let lingua_bridges = self.lingua_family_for(self.hubs[h].component)
                .map(|lf| !lf.is_empty() && lf == maj_fam).unwrap_or(false);
            // Prestige / melting-pot pressure: large, wealthy cities assimilate faster.
            let prestige = (self.hubs[h].population / 20_000.0).clamp(0.0, 1.0) * 0.5
                + self.hubs[h].trade_wealth.clamp(0.0, 1.0) * 0.5;
            // A vigorous young creole majority pulls its minorities in faster.
            let maj_vigor = self.creole_vigor(&maj_name);
            let mins = std::mem::take(&mut self.hub_minorities[h]);
            let mut kept: Vec<(String, f32)> = Vec::with_capacity(mins.len());
            for (c, s) in mins {
                let fam = self.culture_family(&c);
                let related = !fam.is_empty() && !maj_fam.is_empty() && fam == maj_fam;
                // Same family → 2×; distant family → 0.6×; unknown → 1×. Prestige adds up to +100%.
                let mut kin: f32 = if fam.is_empty() || maj_fam.is_empty() { 1.0 }
                    else if related { 2.0 } else { 0.6 };
                // A shared lingua franca lifts the distant-family penalty toward parity.
                if lingua_bridges && !related && !fam.is_empty() {
                    kin = kin.max(LINGUA_BRIDGE);
                }
                // Light ETHNIC-APPEARANCE tie: peoples of the same appearance group (climate/
                // dress) blend a little more readily even across language families.
                let look = match (maj_env, crate::sim::cultures::people_env(&c)) {
                    (Some(a), Some(b)) if a == b && !related => APPEARANCE_ASSIM_BONUS,
                    _ => 1.0,
                };
                // TRAIT resistance: Insular / Xenophobic / Diaspora peoples keep their
                // quarters intact (a real self-contained minority, e.g. a diaspora
                // trading community); Assimilative peoples dissolve faster.
                let resist = crate::sim::cultures::traits_resist_assimilation(&self.culture_trait_ids(&c));
                let mut rate = (MINORITY_ASSIM_RATE * kin * look * resist * (1.0 + prestige)).clamp(0.0, 0.25);
                // Vigorous young majority: pull minorities in faster (converts quicker).
                if maj_vigor > 0.0 { rate *= 1.0 + (CREOLE_VIGOR_PULL - 1.0) * maj_vigor; }
                // Vigorous young minority (the creole itself, still finding its feet
                // somewhere it isn't yet the majority): resist being assimilated away.
                let self_vigor = self.creole_vigor(&c);
                if self_vigor > 0.0 { rate *= 1.0 - CREOLE_VIGOR_RESIST * self_vigor; }
                let rate = rate.clamp(0.0, 0.25);
                let ns = s * (1.0 - rate);
                if ns > 0.005 { kept.push((c, ns)); }
            }
            kept.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            kept.truncate(5);
            self.hub_minorities[h] = kept;
        }
    }


    /// Cultures 2.0 · Ethnogenesis — where a large minority has long shared a city with
    /// the majority, the two blend into a NEW creole people with a synthesized name and
    /// its own origin card. Bounded (a global cap + a low yearly chance), so a handful
    /// of creoles arise over a long campaign rather than a flood.
    pub(crate) fn ethnogenesis_pass(&mut self, year: u32) {
        if self.creoles.len() >= CREOLE_MAX { return; }
        let tick = self.tick;
        let n = self.hubs.len();
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.creoles.len() >= CREOLE_MAX { break; }
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < CREOLE_MIN_POP { continue; }
            // Largest minority quarter here.
            let (mut bi, mut bs) = (usize::MAX, 0.0f32);
            for (i, (_, s)) in self.hub_minorities[h].iter().enumerate() {
                if *s > bs { bs = *s; bi = i; }
            }
            if bi == usize::MAX || bs < CREOLE_MIN_MINORITY { continue; }
            if hash01(self.seed, tick as u64 ^ 0xE7_1409E5, h as u64) > CREOLE_YEARLY_CHANCE { continue; }
            let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
            let minc = self.hub_minorities[h][bi].0.clone();
            if maj.is_empty() || maj == "—" || maj == minc { continue; }
            // Don't re-spawn a creole already born of this same pair.
            let pair_fam = format!("Creole ({} · {})", maj, minc);
            if self.creoles.iter().any(|c| c.family == pair_fam) { continue; }
            let (name, color, kit_a, kit_b) = self.synth_creole_name(&maj, &minc, h, year);
            if name.is_empty() || self.creoles.iter().any(|c| c.name == name) { continue; }
            // Seed: half the minority quarter becomes the creole (intermarriage with locals).
            let take = bs * CREOLE_SEED_FRAC;
            self.hub_minorities[h][bi].1 -= take;
            self.hub_minorities[h].push((name.clone(), take));
            let mob = crate::sim::cultures::people_mobility(&name);
            let temperament = if mob >= 0.6 { "an outward-looking people, quick to take to the trade roads" }
                else { "a settled people, rooted where they were born" };
            let origin = format!(
                "{name} — a creole people born in {place} around year {year}, where {maj} and {minc} quarters blended into one; {temperament}.",
                name = name, place = self.hubs[h].name, year = year, maj = maj, minc = minc, temperament = temperament);
            self.creoles.push(Creole {
                name: name.clone(), family: pair_fam, origin, color,
                born_tick: tick, birthplace: self.hubs[h].name.clone(), kit_a, kit_b,
            });
            self.journal.push(JournalEntry {
                tick, kind: "ethnogenesis".into(), hub: h as i32, good: -1, value: take,
                text: format!("A new people, the {}, is born in {} as {} and {} blend into one.",
                    name, self.hubs[h].name, maj, minc) });
        }
    }


    /// EXPEDITIONS — a wealthy, fleet-owning house rarely mounts an expedition to a far
    /// isolated settlement (≤1 trade tie) that the network doesn't normally reach: the
    /// "casual merchants arriving on rare occasions" a remote outpost sees. It delivers a
    /// batch of the house's specialty good (relieving a little local scarcity) at a modest
    /// cost to the house (reach & prestige, not profit). Bounded and rare.
    pub(crate) fn expedition_pass(&mut self, year: u32) {
        let tick = self.tick;
        let n = self.hubs.len();
        let ng = self.goods.len();
        if ng == 0 { return; }
        let remotes: Vec<usize> = (0..n).filter(|&h|
            !self.hubs[h].is_estate && !self.hubs[h].abandoned
            && self.hubs[h].population > 50.0
            && self.neighbors.get(h).map(|v| v.len()).unwrap_or(0) <= 1
        ).collect();
        if remotes.is_empty() { return; }
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            let fleet = self.houses[hi].fleet_sea + self.houses[hi].fleet_river + self.houses[hi].fleet_caravan;
            if fleet == 0 || self.houses[hi].wealth < EXPEDITION_MIN_WEALTH { continue; }
            let chance = EXPEDITION_YEARLY_CHANCE * (1.0 + (fleet as f32).min(6.0) * 0.15);
            if hash01(self.seed, tick as u64 ^ 0xE59_ED17, hi as u64) > chance { continue; }
            let pick = (crate::sim::cultures::hash64(self.seed ^ (hi as u64) ^ ((year as u64) << 8)) as usize) % remotes.len();
            let t = remotes[pick];
            if t == self.houses[hi].hub as usize { continue; }
            let g = self.houses[hi].spec.first().copied().filter(|&g| g < ng).unwrap_or(0);
            let pop = self.hubs[t].population.max(0.0);
            let batch = (pop * 0.02).clamp(2.0, 60.0);
            let price = self.hubs[t].price.get(g).copied().unwrap_or(self.goods[g].base_value).max(EPS);
            if g < ng { stock_add_ungraded(&mut self.hubs[t].stock, g, batch); }
            // The expedition COSTS the house (reach, not profit) — bounded, only reduces wealth.
            let cost = (batch * price * 0.05).min(self.houses[hi].wealth.max(0.0) * 0.02);
            self.houses[hi].wealth -= cost;
            let (gn, hn, tn) = (self.goods[g].name.clone(), self.houses[hi].name.clone(), self.hubs[t].name.clone());
            self.journal.push(JournalEntry {
                tick, kind: "expedition".into(), hub: t as i32, good: g as i32, value: batch,
                text: format!("An expedition of {hn} reaches the remote {tn}, trading {gn}, in year {year}.") });
        }
    }


    /// Cultures 3.0 · SPLINTERING — a large, far-flung community of one people, long
    /// isolated from its hearth, drifts into a NEW daughter people with its own name and
    /// origin card (same kit/appearance as the parent). Rare and distance-biased, capped
    /// by the shared creole limit, so a few daughter peoples arise over a long campaign.
    pub(crate) fn splinter_pass(&mut self, year: u32) {
        if self.creoles.len() >= CREOLE_MAX { return; }
        let tick = self.tick;
        let n = self.hubs.len();
        let ww = self.world_w.max(1.0);
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.creoles.len() >= CREOLE_MAX { break; }
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < CREOLE_MIN_POP { continue; }
            let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
            if maj.is_empty() || maj == "—" { continue; }
            // Only a rooted HEARTH people splinters (creoles are already new peoples).
            let kit = match crate::sim::cultures::kit_of_people(&maj) { Some(k) => k, None => continue };
            // The parent must clearly dominate this city (a coherent community).
            let minsum: f32 = self.hub_minorities[h].iter().map(|(_, s)| *s).sum();
            let maj_share = (1.0 - minsum).clamp(0.0, 1.0);
            if maj_share < SPLINTER_MIN_SHARE { continue; }
            // Divergence rises with DISTANCE from the parent hearth — a far-flung,
            // isolated community drifts into its own people.
            let dist = crate::sim::cultures::active().and_then(|m| {
                m.hearths.iter().find(|hh| hh.people == maj).map(|hh| {
                    let mut dx = (hh.x - self.hubs[h].x).abs();
                    if ww > 1.0 { dx = dx.min(ww - dx); }
                    let dy = hh.y - self.hubs[h].y;
                    (dx * dx + dy * dy).sqrt()
                })
            }).unwrap_or(0.0);
            let far = (dist / (ww * 0.25)).clamp(0.0, 1.0); // 0 at the hearth, 1 a quarter-world away
            if far < 0.35 { continue; } // must be genuinely far-flung
            // ISOLATION drives the (otherwise rare) chance: distance from the hearth
            // (steeply, far^1.6) AND how few trade ties the settlement has — a lonely
            // outpost with one or no trade partners drifts into its own people far more
            // readily than a well-connected city. A hub with ≤1 tie gets up to ~3×.
            let ties = self.neighbors.get(h).map(|v| v.len()).unwrap_or(0);
            let lonely = 1.0 + (1.0 - (ties as f32 / 4.0).min(1.0)) * 2.0; // 3× at 0 ties → 1× at ≥4
            let chance = SPLINTER_YEARLY_CHANCE * far.powf(1.6) * lonely;
            if hash01(self.seed, tick as u64 ^ 0x5D1B_7E20, h as u64) > chance { continue; }

            let nseed = crate::sim::cultures::hash64(
                self.seed ^ (h as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ ((year as u64) << 20) ^ 0xDA02);
            let name = crate::sim::cultures::gen_people_name(kit, nseed, self.hubs[h].x as u32, self.hubs[h].y as u32);
            if name.is_empty() || name == maj
                || self.creoles.iter().any(|c| c.name == name)
                || crate::sim::cultures::active().map(|m| m.hearths.iter().any(|hh| hh.people == name)).unwrap_or(false) {
                continue;
            }
            // Daughter colour: the parent's, nudged so the two read apart on the map.
            let base = crate::sim::cultures::color_of_people(&maj).unwrap_or([150, 140, 170]);
            let shift = |c: u8, d: i32| (c as i32 + d).clamp(30, 235) as u8;
            let s0 = (crate::sim::cultures::hash64(nseed ^ 0x515F) % 3) as usize;
            let color = [shift(base[0], if s0 == 0 { 40 } else { -25 }),
                         shift(base[1], if s0 == 1 { 40 } else { -25 }),
                         shift(base[2], if s0 == 2 { 40 } else { -25 })];
            // The daughter secedes from the local majority (seeded as its own quarter).
            let take = maj_share * SPLINTER_SEED_FRAC;
            self.hub_minorities[h].push((name.clone(), take));
            let origin = format!(
                "{name} — a daughter people of the {maj}, arisen around year {year} in far-off {place}. Generations removed from the {maj} heartland, their speech and ways drifted until they became a people of their own.",
                name = name, maj = maj, year = year, place = self.hubs[h].name);
            self.creoles.push(Creole {
                name: name.clone(), family: format!("Branch of {maj}"), origin, color,
                born_tick: tick, birthplace: self.hubs[h].name.clone(), kit_a: kit as u8, kit_b: kit as u8,
            });
            self.journal.push(JournalEntry {
                tick, kind: "ethnogenesis".into(), hub: h as i32, good: -1, value: take,
                text: format!("A new people, the {}, splinters from the {} in far-off {}.",
                    name, maj, self.hubs[h].name) });
        }
    }


    /// Synthesize a creole's name + colour + parent kits from its two parent peoples.
    /// Falls back to default kits when a parent isn't a worldgen hearth (creole-of-creole).
    pub(crate) fn synth_creole_name(&self, a: &str, b: &str, h: usize, year: u32) -> (String, [u8; 3], u8, u8) {
        use crate::sim::cultures as cul;
        let ka = cul::kit_of_people(a).unwrap_or(0);
        let kb = cul::kit_of_people(b).unwrap_or(1);
        let seed = cul::hash64(self.seed ^ (h as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ ((year as u64) << 12));
        let name = cul::blend_name(ka, kb, seed);
        let ca = cul::color_of_people(a).unwrap_or([170, 130, 190]);
        let cb = cul::color_of_people(b).unwrap_or([130, 170, 150]);
        let color = [((ca[0] as u16 + cb[0] as u16) / 2) as u8,
            ((ca[1] as u16 + cb[1] as u16) / 2) as u8,
            ((ca[2] as u16 + cb[2] as u16) / 2) as u8];
        (name, color, ka as u8, kb as u8)
    }


    /// #23 · Yearly economic migration: within a trade component, people drift from
    /// low-opportunity (poor / hungry) cities to the most thriving reachable one.
    /// Emits fading arrows + mixes cultures. Small fractions keep populations bounded.
    pub(crate) fn economic_migration_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        // Opportunity per hub (prosperity − starvation); precomputed so the inner
        // destination scan is cheap and borrow-safe.
        let opp: Vec<f32> = self.hubs.iter()
            .map(|h| if h.is_estate || h.abandoned { f32::MIN } else { h.sent_prosperity - h.starving })
            .collect();
        for src in 0..n {
            let s = &self.hubs[src];
            if s.is_estate || s.abandoned || s.population < ECON_MIG_MIN_POP { continue; }
            let src_opp = opp[src];
            if src_opp > ECON_MIG_STAY_ABOVE { continue; } // content cities keep their people
            let src_pop = s.population;
            // STRICT ROUTE RULE: people may only move to a DIRECT TRADE PARTNER (a single
            // trade tie), never a straight geographic jump to a distant city. They chain
            // city→city→city across the network over successive years, so every migration
            // line lies exactly on a real trade route. (User: "no migration except via
            // trade routes.") Pick the best-opportunity direct neighbour.
            let src_culture = self.hub_culture.get(src).cloned().unwrap_or_default();
            let mut dest = (usize::MAX, src_opp); // must beat the home city's own opportunity
            if let Some(nbrs) = self.neighbors.get(src) {
                for &bn in nbrs {
                    let d = bn as usize;
                    if d >= n || d == src { continue; }
                    let o = &self.hubs[d];
                    if o.is_estate || o.abandoned || o.food_balance < 0.0 { continue; }
                    // Cultures 2.0 · HOMOPHILY: kin already settled at a destination make it
                    // more attractive — people prefer to move where their own people are.
                    let o_eff = opp[d] + HOMOPHILY_PULL * self.culture_share_at(d, &src_culture);
                    if o_eff > dest.1 { dest = (d, o_eff); }
                }
            }
            let Some(di) = (dest.0 != usize::MAX).then_some(dest.0) else { continue; };
            if dest.1 - src_opp < ECON_MIG_GRADIENT { continue; } // needs a real pull
            let movers = (src_pop * ECON_MIG_FRAC).clamp(10.0, 800.0);
            if movers >= src_pop * 0.5 { continue; }
            // STRICT: people move only if a trade-route chain connects the two cities.
            let culture = src_culture;
            if !self.emit_migration_route(src, di, &culture, movers) { continue; }
            self.hubs[src].population -= movers;
            self.hubs[di].population += movers;
            self.record_migration_culture(di, src, movers);
            if hash01(self.seed, tick as u64 ^ 0x5EED_0C, src as u64) < 0.08 {
                let (sn, dn) = (self.hubs[src].name.clone(), self.hubs[di].name.clone());
                self.journal.push(JournalEntry { tick, kind: "migration".into(), hub: di as i32, good: -1,
                    value: movers, text: format!("{:.0} people leave {} seeking work in {}", movers, sn, dn) });
            }
        }
    }


    /// emigration: it spends a slice of its treasury to move a wave of people to an
    /// under-populated, food-secure city in its own trade region (preferring its own
    /// colonies). This both relieves the sponsor's crowding and drains the treasuries
    /// that would otherwise hoard wealth forever. (User-requested: "poleis should spend
    /// a percentage of wealth each month on funding colonies and for people migration.")
    pub(crate) fn poleis_sponsor_migration(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        for src in 0..n {
            let s = &self.hubs[src];
            if s.is_estate { continue; }
            if s.population < COLONY_PARENT_MIN_POP || s.starving > 0.5 { continue; }
            if s.treasury < POLIS_MIGRATION_MIN_TREASURY || s.sent_prosperity < 0.3 { continue; }
            // Sponsor roughly every other month per eligible city.
            if hash01(self.seed, tick as u64 ^ 0x319A7E, src as u64) > 0.5 { continue; }
            let comp = self.hubs[src].component;
            let pop = self.hubs[src].population;
            let km_per_cell = EARTH_EQUATOR_KM / self.world_w.max(1.0);
            let max_cells = MIGRATION_MAX_KM / km_per_cell.max(1e-3);
            // Destination: the most under-populated food-secure city in the same trade
            // region, strongly preferring a colony this polis founded.
            let mut dest = (usize::MAX, f32::MAX);
            for d in 0..n {
                if d == src || self.hubs[d].is_estate || self.hubs[d].component != comp { continue; }
                if self.hubs[d].food_balance < 0.0 { continue; }
                // Settlers move to NEARBY cities only — no cross-map sponsorship. Being
                // in the same trade component spans the whole continent/ocean, so gate on
                // real distance (≤ MIGRATION_MAX_KM) like the other migration passes.
                if self.hub_cell_dist(src, d) > max_cells { continue; }
                if self.hubs[d].population >= pop * 0.5 { continue; } // must have room to grow
                let own_colony = self.hubs[d].founder_hub == src as i32;
                let rank = self.hubs[d].population * if own_colony { 0.4 } else { 1.0 };
                if rank < dest.1 { dest = (d, rank); }
            }
            let Some(di) = (dest.0 != usize::MAX).then_some(dest.0) else { continue; };
            let movers = (pop * POLIS_MIGRATION_POP_FRAC).clamp(20.0, 1500.0);
            let spend = self.hubs[src].treasury * POLIS_MIGRATION_SPEND;
            if spend <= 0.0 { continue; }
            self.hubs[src].treasury -= spend;
            self.hubs[src].population = (pop - movers).max(self.hubs[src].founding_pop * 0.5);
            self.hubs[di].population += movers;
            self.hubs[di].treasury += spend * 0.5; // settlement aid travels with the migrants
            self.record_migration_culture(di, src, movers);
            let culture = self.hub_culture.get(src).cloned().unwrap_or_default();
            self.emit_migration_route(src, di, &culture, movers); // routed along the trade network
            if hash01(self.seed, tick as u64 ^ 0x4D161, src as u64) < 0.15 {
                let (sn, dn) = (self.hubs[src].name.clone(), self.hubs[di].name.clone());
                self.journal.push(JournalEntry { tick, kind: "migration".into(), hub: di as i32, good: -1,
                    value: movers, text: format!("{} sponsors {:.0} settlers to {}", sn, movers, dn) });
            }
        }
    }


    /// SATELLITE cities: a large metropolis founds a SHORT-RANGE satellite to serve a
    /// concrete NEED it cannot fit inside itself — a PORT (a big INLAND trade hub
    /// wants a harbour at the land/sea interface), a GRANARY (food-short core), or a
    /// WORKSHOP (a very large core outgrows its works). Council-approved, funded from
    /// the city treasury, seeded with relocated townsfolk. Historical: Ostia/Portus→
    /// Rome, Piraeus→Athens, Westminster & Southwark→London, Galata→Constantinople.
    /// One per call (yearly).
    pub(crate) fn maybe_found_satellite(&mut self, expansion_ok: bool) {
        if !expansion_ok || self.satellite_sites.is_empty() { return; }
        let tick = self.tick;
        // Satellites hug the metropolis: ≤ 500 km (a day's ride), from the dedicated
        // near-city pool — NOT the far colony pool. `SATELLITE_REACH_FRAC` is no longer
        // the leash; the absolute km cap is.
        let reach = (SATELLITE_MAX_KM * self.world_w / EARTH_EQUATOR_KM).max(2.0);
        for m in 0..self.hubs.len() {
            let metro = &self.hubs[m];
            if metro.is_estate || metro.abandoned || metro.colony_kind != 0 { continue; }
            if metro.population < SATELLITE_METRO_POP || metro.treasury < SATELLITE_COST { continue; }
            // TIER GATE (ability unlock): only a Free City (tier 4) or greater — a real
            // metropolitan power — spins off a satellite suburb. A 25k+ metropolis has
            // normally reached it, so this rarely blocks and mostly adds thematic order.
            if self.dev_tier.get(m).copied().unwrap_or(0) < 4 { continue; }
            // NO SPAM: only ONE satellite may be under construction per metropolis — it must
            // be FINISHED before the council breaks ground on the next (user rule).
            if self.hubs.iter().any(|h| h.founder_hub == m as i32 && h.build_stage > 0 && !h.abandoned) { continue; }
            // Cap the total dependent satellites (kind 3) a single metropolis sustains.
            let sat_count = self.hubs.iter()
                .filter(|h| h.founder_hub == m as i32 && !h.abandoned && h.colony_kind == 3)
                .count() as u32;
            if sat_count >= SATELLITE_MAX_PER_METRO { continue; }
            // The NEED decides the role (priority: survival → trade → industry).
            let need_granary = metro.food_balance < 0.0;
            let need_port = !metro.coastal && metro.trade_wealth > 0.0;
            let need_workshop = metro.population >= SATELLITE_WORKSHOP_POP;
            let role: u8 = if need_granary { 1 } else if need_port { 0 }
                else if need_workshop { 2 } else { continue };
            if hash01(self.seed, tick as u64 ^ 0x5A7E11, m as u64) > 0.5 { continue; }
            // Nearest reachable NEAR-city site (a PORT needs a COASTAL one — the
            // land/sea transshipment point).
            let (mx, my) = (metro.x, metro.y);
            let mut best = (usize::MAX, f32::MAX);
            for (i, site) in self.satellite_sites.iter().enumerate() {
                if role == 0 && !site.coastal { continue; }
                let mut dx = (site.x - mx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = site.y - my;
                let d2 = dx * dx + dy * dy;
                if d2 > reach * reach { continue; }
                if d2 < best.1 { best = (i, d2); }
            }
            let Some(si) = (best.0 != usize::MAX).then_some(best.0) else { continue; };
            // Council funds it + relocates settlers from the crowded core.
            self.hubs[m].treasury -= SATELLITE_COST;
            let seed = SATELLITE_SEED_POP.min(self.hubs[m].population * 0.12);
            self.hubs[m].population = (self.hubs[m].population - seed)
                .max(self.hubs[m].founding_pop * 0.5);
            let site = self.satellite_sites.swap_remove(si);
            let new = self.create_organic_town(m, &site, seed);
            self.hubs[new].founder_hub = m as i32; // tracks the metropolis it serves
            self.hubs[new].colony_kind = 3;        // SATELLITE (dependent on its metropolis)
            self.hubs[new].colony_founded_tick = tick;
            if role == 0 { self.hubs[new].coastal = true; } // PORT — the harbour at the coast
            // Break ground on the CONSTRUCTION project (10y, decay model — the role bias +
            // future-exploit production activate only on COMPLETION; the site just consumes
            // hauled supply while it's built). `colony_stage` parks the intended role until
            // then.
            self.hubs[new].build_stage = 1;
            self.hubs[new].build_start_tick = tick;
            self.hubs[new].build_convoys = SAT_BUILD_CONVOYS;
            self.hubs[new].colony_stage = role; // 0 port · 1 granary · else workshop
            for c in 0u8..3 {
                self.hubs[new].build_supply_good[c as usize] = self.pick_build_supply_good(m, c);
            }
            let role_name = match role { 1 => "granary", 0 => "port", _ => "workshop" };
            let (metro_name, sat_name) = (self.hubs[m].name.clone(), self.hubs[new].name.clone());
            self.journal.push(JournalEntry {
                tick, kind: "founding".into(), hub: new as i32, good: -1, value: seed,
                text: format!("The council of {} breaks ground on the {} town of {}", metro_name, role_name, sat_name),
            });
            self.total_foundings += 1;
            self.routes_dirty = true;
            return; // one satellite per call
        }
    }


    /// Monthly · RIGHT OF FIRST BUY (staple right). Each solvent city council pre-empts
    /// the goods it needs (food always; plus preservables/construction when it provisions
    /// colonies or a satellite build) out of its OWN market stock — i.e. it buys from the
    /// merchants arriving in the city before the open market clears — and secures them in
    /// the civic warehouse (`civic_goods`). A council whose city has been CAPTURED by a
    /// house (the house dominates its trade) loses first refusal: it can still stock up,
    /// but only at a retail premium after the house has creamed the market. This is the
    /// investment that lets colonies grow (their build/supply draws the civic reserve
    /// first) and pays back later as those colonies ship goods home. User-requested.
    pub(crate) fn council_provision_pass(&mut self) {
        let ng = self.goods.len();
        if ng == 0 { return; }
        let n = self.hubs.len();
        if self.council_bought_month.len() != n { self.council_bought_month = vec![0.0; n]; }
        // Precompute each city's dependent count ONCE (O(n)) instead of scanning all hubs per
        // hub (that O(n²) monthly scan was part of the late-campaign slowdown).
        let mut deps_count = vec![0usize; n];
        for d in &self.hubs {
            if d.abandoned { continue; }
            if d.founder_hub >= 0 && (d.colony_kind == 1 || d.colony_kind == 3 || d.build_stage > 0) {
                let f = d.founder_hub as usize;
                if f < n { deps_count[f] += 1; }
            }
        }
        for h in 0..n {
            self.council_bought_month[h] = 0.0;
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < 1.0 { continue; }
            if self.hubs[h].treasury < COUNCIL_PROVISION_MIN_TREASURY { continue; }
            let deps = deps_count[h];
            // A city that neither provisions a colony nor is food-stressed needn't hoard.
            if deps == 0 && self.hubs[h].food_balance > 0.15 { continue; }
            // Right of first buy is suspended when merchant HOUSES dominate the city's trade
            // (carry ≥60% of it) OR a house has captured the government. Then the council can
            // still stock up, but only at a retail premium after the houses take the market.
            let dominated = hub_house_trade_share(&self.hubs[h]) >= COUNCIL_DOMINANCE_THRESHOLD
                || self.hubs[h].captor_house >= 0;
            let first_buy = !dominated;
            let unit_mult = if first_buy { COUNCIL_BUY_PRICE } else { COUNCIL_RETAIL_PRICE };
            let target = self.council_reserve_target(h, deps);
            if self.hubs[h].civic_goods.len() < ng { self.hubs[h].civic_goods.resize(ng, 0.0); }
            let budget = self.hubs[h].treasury * COUNCIL_PROVISION_BUDGET_FRAC;
            let mut spent = 0.0f32;
            for g in 0..ng {
                // Need it? Food is always secured; other goods only when feeding dependents.
                if !(self.goods[g].food || deps > 0) { continue; }
                let have = self.hubs[h].civic_goods[g];
                if have >= target { continue; }
                let avail = stock_of(&self.hubs[h].stock, g).max(0.0);
                let price = self.hubs[h].price.get(g).copied()
                    .unwrap_or(self.goods[g].base_value).max(0.01) * unit_mult;
                let afford = (budget - spent).max(0.0) / price;
                let buy = (target - have).min(avail).min(afford);
                if buy <= 0.0 { continue; }
                stock_take(&mut self.hubs[h].stock, g, buy);          // pre-empted out of the open market
                self.hubs[h].civic_goods[g] += buy;    // …into the civic warehouse
                spent += buy * price;
            }
            self.hubs[h].treasury -= spent;
            self.council_bought_month[h] = spent; // grain-eq secured this month (UI)
        }
    }


    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.2 (D17/F6/D4) · monthly: size every
    /// non-estate hub's OWN warehouse capacity from its population + Granary/
    /// Warehouse structures (the two already exist for a production bonus — this
    /// gives them a second, obvious job rather than inventing a new structure),
    /// then spoil `stock` (all three grade bands alike — D4, one rate per good) and
    /// `civic_goods` at a per-good rate derived from `GoodSpec.perishable`,
    /// tallying what rotted into `wh_spoiled_month` for the warehouse panel (§4.3).
    /// Stock held past capacity spoils faster (`SPOIL_OVERFLOW_MULT`) — an
    /// overflowing store rots quicker than a well-kept one.
    pub(crate) fn warehouse_and_spoilage_pass(&mut self) {
        let ng = self.goods.len();
        if ng == 0 { return; }
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let has_granary = self.hub_has_struct(h, STRUCT_GRANARY);
            let has_wh = self.hub_has_struct(h, STRUCT_WAREHOUSE);
            let struct_bonus = CITY_WH_STRUCT_BONUS
                * (has_granary as u8 as f32 + has_wh as u8 as f32);
            self.hubs[h].wh_capacity =
                CITY_WH_CAP_BASE + self.hubs[h].population * CITY_WH_CAP_PER_POP + struct_bonus;

            let cap = self.hubs[h].wh_capacity;
            let total_held: f32 = (0..ng).map(|g| stock_of(&self.hubs[h].stock, g)).sum();
            let over_frac = if cap > EPS { (total_held / cap - 1.0).max(0.0) } else { 0.0 };
            let overflow_mult = 1.0 + over_frac.min(1.0) * (SPOIL_OVERFLOW_MULT - 1.0);

            if self.hubs[h].wh_spoiled_month.len() != ng { self.hubs[h].wh_spoiled_month = vec![0.0; ng]; }
            for g in 0..ng {
                let mut rate = (self.goods[g].perishable.max(0.0) * SPOIL_PER_PERISHABLE).min(SPOIL_RATE_CAP);
                if rate <= 0.0 { self.hubs[h].wh_spoiled_month[g] = 0.0; continue; }
                if has_granary && self.goods[g].food { rate *= SPOIL_GRANARY_FOOD_MULT; }
                if has_wh { rate *= SPOIL_WAREHOUSE_MULT; }
                rate = (rate * overflow_mult).clamp(0.0, 0.95);
                let mut spoiled = 0.0f32;
                let before = stock_of(&self.hubs[h].stock, g);
                if before > EPS {
                    let s = stock_take(&mut self.hubs[h].stock, g, before * rate);
                    spoiled += s;
                }
                if g < self.hubs[h].civic_goods.len() {
                    let cbefore = self.hubs[h].civic_goods[g].max(0.0);
                    if cbefore > EPS {
                        let cs = cbefore * rate;
                        self.hubs[h].civic_goods[g] -= cs;
                        spoiled += cs;
                    }
                }
                self.hubs[h].wh_spoiled_month[g] = spoiled;
            }
            // Snapshot this month's closing totals — next month's delta baseline
            // (`wh_last_month`, read by the warehouse panel's "▲+340" line).
            if self.hubs[h].wh_last_month.len() != ng { self.hubs[h].wh_last_month = vec![0.0; ng]; }
            for g in 0..ng { self.hubs[h].wh_last_month[g] = stock_of(&self.hubs[h].stock, g); }
        }
    }


    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (§3) · monthly: every ESTATE
    /// samples its own DOMINANT good's output/quality/price into its 12-month
    /// ring — the works card's curves. A non-estate hub is a village in the
    /// plan's own sense (D14) and carries no card, so nothing is sampled for it.
    pub(crate) fn works_monthly_pass(&mut self) {
        let ng = self.goods.len();
        if ng == 0 { return; }
        for h in 0..self.hubs.len() {
            if !self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let g = (0..ng.min(self.hubs[h].production.len()))
                .max_by(|&a, &b| self.hubs[h].production[a]
                    .partial_cmp(&self.hubs[h].production[b]).unwrap_or(std::cmp::Ordering::Equal));
            let Some(g) = g else { continue };
            let sample = MonthSample {
                output: self.hubs[h].production.get(g).copied().unwrap_or(0.0),
                quality: self.hubs[h].quality.get(g).copied().unwrap_or(0.0),
                price: self.hubs[h].price.get(g).copied().unwrap_or(0.0),
            };
            self.hubs[h].monthly.push(sample);
            if self.hubs[h].monthly.len() > WORKS_MONTHLY_CAP {
                let excess = self.hubs[h].monthly.len() - WORKS_MONTHLY_CAP;
                self.hubs[h].monthly.drain(0..excess);
            }
            // 4.13 (A3) · a works reaching GREAT or better for the first time
            // earns a toponymic brand and a chronicle line — pure flavour off
            // works_rank's own statistic, no premium wired (A3's own "pure
            // flavour off a statistic §4.6 already computes").
            if !self.hubs[h].brand_chronicled {
                if let Some((bg, yield_index, ..)) = self.works_rank(h) {
                    if yield_index >= BRAND_YIELD_FLOOR {
                        let place = brand_place(&self.hubs[h].name, estate_kind_label(self.hubs[h].estate_kind));
                        let brand = brand_name(&place, &self.goods[bg].name);
                        let (parent, tick) = (self.hubs[h].parent, self.tick);
                        self.hubs[h].brand_chronicled = true;
                        self.journal.push(JournalEntry {
                            tick, kind: "works".into(), hub: parent, good: bg as i32, value: yield_index,
                            text: format!("{} — {} now commands the name \"{}\" in distant markets",
                                self.hubs[h].name, yield_label(yield_index), brand),
                        });
                    }
                }
            }
        }
    }


    /// A mature, sizeable SATELLITE eventually outgrows its dependency and becomes a
    /// free city in its own right (colony_kind→0), keeping `founder_hub` so the map
    /// and panels can still show which metropolis raised it.
    pub(crate) fn satellite_independence_pass(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].colony_kind != 3 || self.hubs[h].abandoned || self.hubs[h].build_stage > 0 { continue; }
            let age = tick.saturating_sub(self.hubs[h].colony_founded_tick) / TICKS_PER_YEAR;
            if age >= SATELLITE_INDEP_YEARS && self.hubs[h].population >= SATELLITE_INDEP_POP {
                self.hubs[h].colony_kind = 0;
                self.hubs[h].autonomous = true; // free city (founder_hub kept = its heritage)
                let nm = self.hubs[h].name.clone();
                let metro = self.hubs[h].founder_hub;
                let mn = if metro >= 0 && (metro as usize) < self.hubs.len() {
                    self.hubs[metro as usize].name.clone()
                } else { String::new() };
                self.journal.push(JournalEntry {
                    tick, kind: "colony".into(), hub: h as i32, good: -1, value: 0.0,
                    text: if mn.is_empty() { format!("{} grows into a free city in its own right", nm) }
                        else { format!("{} outgrows {} and becomes a free city", nm, mn) },
                });
            }
        }
    }


    pub(crate) fn corridor_exists(&self, a: usize, b: usize) -> bool {
        let (lo, hi) = ((a.min(b)) as u32, (a.max(b)) as u32);
        self.corridors.iter().any(|c| c.a.min(c.b) == lo && c.a.max(c.b) == hi)
    }


    /// YEARLY: prune finished ventures + stale prospects, then let a wealthy house
    /// or two bankroll a new expedition toward a distant, unconnected, valuable city.
    pub(crate) fn expedition_launch_pass(&mut self, expansion_ok: bool) {
        let tick = self.tick;
        // Keep finished ventures ~3 years for the map/panel, then drop them.
        self.expeditions.retain(|e| e.status <= 2
            || tick.saturating_sub(e.launched_tick) < 3 * TICKS_PER_YEAR);
        self.route_prospects.retain(|p| p.established
            || tick.saturating_sub(p.last_tick) < EXP_PROSPECT_TTL);
        if !expansion_ok || tick < EXP_START_TICK { return; }
        if self.expeditions.iter().filter(|e| e.status <= 2).count() >= EXP_MAX_ACTIVE { return; }
        let n = self.hubs.len();
        let min_gap = self.world_w * EXP_MIN_GAP_FRAC;
        let max_gap = self.world_w * EXP_MAX_GAP_FRAC;
        let mut backers: Vec<usize> = (0..self.houses.len())
            .filter(|&h| !self.houses[h].defunct && !self.houses[h].is_guild
                && self.houses[h].wealth >= EXP_MIN_HOUSE_WEALTH)
            .collect();
        if backers.is_empty() { return; }
        backers.sort_by(|&x, &y| self.houses[y].wealth
            .partial_cmp(&self.houses[x].wealth).unwrap_or(std::cmp::Ordering::Equal));
        let mut launched = 0u32;
        for &hi in backers.iter().take(6) {
            if launched >= 2 { break; }
            if self.expeditions.iter().filter(|e| e.status <= 2).count() >= EXP_MAX_ACTIVE { break; }
            if hash01(self.seed, tick as u64 ^ 0xE79E, hi as u64) > 0.35 { continue; }
            let origin = self.houses[hi].hub as usize;
            if origin >= n || self.hubs[origin].is_estate || self.hubs[origin].abandoned { continue; }
            // A costly foreign venture is only mounted from a PROSPEROUS, well-fed
            // seat — a starving or unstable city has no surplus to gamble on reach.
            if self.hubs[origin].starving > 0.35 || self.hubs[origin].sent_prosperity < 0.35 { continue; }
            // Target: a far, populous, unconnected city (fatigued by past failures).
            let mut best = (usize::MAX, 0.0f32);
            for d in 0..n {
                if d == origin { continue; }
                let hd = &self.hubs[d];
                if hd.is_estate || hd.abandoned || hd.population < 400.0 { continue; }
                let dist = self.hub_cell_dist(origin, d);
                // A venture reaches for a REGIONAL unconnected settlement, not a
                // hemisphere away — bounded both sides (diagnosed: with no ceiling,
                // scoring below used to reward raw distance forever, so ventures
                // systematically reached for the single farthest reachable city).
                if dist < min_gap || dist > max_gap { continue; }
                if self.corridor_exists(origin, d) { continue; }
                let (lo, hiid) = ((origin.min(d)) as u32, (origin.max(d)) as u32);
                let (attempts, successes) = self.route_prospects.iter()
                    .find(|p| p.a == lo && p.b == hiid)
                    .map(|p| (p.attempts, p.successes)).unwrap_or((0, 0));
                // A route with prior SUCCESSES is worth finishing (cement it into a
                // corridor); only FAILED attempts fatigue the appetite.
                let promise = 1.0 + successes as f32 * 0.9;
                let fatigue = 1.0 / (1.0 + attempts.saturating_sub(successes) as f32 * 0.5);
                let jitter = 0.7 + 0.6 * hash01(self.seed, d as u64, (tick as u64) ^ (hi as u64));
                // Reach a sweet spot just past the near floor rather than maximising
                // distance: peaks at ~1.5× the minimum gap, tapering both directions
                // so a nearer prospect can still win on population/promise alone.
                let ideal = min_gap * 1.5;
                let reach = (1.0 - ((dist - ideal).abs() / (max_gap - min_gap).max(1.0)).min(1.0) * 0.7).max(0.3);
                let score = hd.population.sqrt() * reach * fatigue * promise * jitter;
                if score > best.1 { best = (d, score); }
            }
            let Some(dest) = (best.0 != usize::MAX).then_some(best.0) else { continue };
            self.launch_expedition(hi, origin, dest);
            launched += 1;
        }
    }


    /// Outfit + register + dispatch one expedition (cost debited from the house).
    pub(crate) fn launch_expedition(&mut self, hi: usize, origin: usize, dest: usize) {
        let tick = self.tick;
        let ng = self.goods.len();
        // Chief export of the origin = the cargo.
        let (mut good, mut bestp) = (0usize, 0.0f32);
        let plen = self.hubs[origin].production.len().min(ng);
        for g in 0..plen {
            let p = self.hubs[origin].production[g];
            if p > bestp { bestp = p; good = g; }
        }
        let dist = self.hub_cell_dist(origin, dest);
        let km = dist * EARTH_EQUATOR_KM / self.world_w.max(1.0);
        let wealth = self.houses[hi].wealth;
        let rand = 0.7 + 0.6 * hash01(self.seed, tick as u64, (origin as u64) ^ ((dest as u64) << 16));
        let units = (((km / EXP_REF_KM) * (wealth / 80.0).clamp(0.3, 3.0) * 4.0) * rand)
            .round().clamp(2.0, 24.0) as u16;
        let sea_route = self.hubs[origin].coastal && self.hubs[dest].coastal;
        let (ships, caravans) = if sea_route { (units, 0u16) } else { (0u16, units) };
        let cost = units as f32 * EXP_UNIT_COST
            * (1.0 + Self::koppen_peril(self.hubs[dest].koppen) + if sea_route { 0.3 } else { 0.0 });
        let cargo_qty = units as f32 * 3.0;
        self.houses[hi].wealth -= cost;
        let travel = (dist * self.days_per_cell).round().clamp(18.0, 320.0) as u32;
        let leader = {
            let h = &self.houses[hi];
            let head = if h.head_name.is_empty() { h.name.clone() } else { h.head_name.clone() };
            format!("{} of {}", head, h.name)
        };
        let id = self.next_expedition_id;
        self.next_expedition_id = self.next_expedition_id.wrapping_add(1);
        let (ox, oy) = (self.hubs[origin].x, self.hubs[origin].y);
        let (dx, dy) = (self.hubs[dest].x, self.hubs[dest].y);
        let (on, dn) = (self.hubs[origin].name.clone(), self.hubs[dest].name.clone());
        self.journal.push(JournalEntry {
            tick, kind: "expedition".into(), hub: origin as i32, good: good as i32, value: -cost,
            text: format!("{} funds an expedition from {} to {} — {} {}",
                leader, on, dn, units, if sea_route { "ships" } else { "caravans" }),
        });
        let dest_province = self.hub_province.get(dest).copied().unwrap_or(-1);
        self.expeditions.push(Expedition {
            id, house: hi as u32, leader, origin: origin as u32, dest: dest as u32,
            ox, oy, dx, dy, launched_tick: tick, travel_ticks: travel, pos: 0.0, outbound: true,
            caravans, ships, good: good as u16, cargo_qty, cost, revenue: 0.0,
            arrived_frac: 1.0, status: 0, hazards: Vec::new(), dest_province,
        });
    }


    /// EVERY TICK (cheap when idle): advance ventures, roll hazards, resolve
    /// arrivals/returns, and establish a corridor once a route is proven.
    pub(crate) fn expedition_travel_pass(&mut self) {
        if self.expeditions.is_empty() { return; }
        let tick = self.tick;
        let seed = self.seed;
        for ei in 0..self.expeditions.len() {
            let mut e = self.expeditions[ei].clone();
            if e.status >= 3 { continue; }
            let origin = e.origin as usize;
            let dest = e.dest as usize;
            let good = e.good as usize;
            let owner = e.house as usize;
            let ko = self.hubs.get(origin).map(|h| h.koppen).unwrap_or(0);
            let kd = self.hubs.get(dest).map(|h| h.koppen).unwrap_or(0);
            let sea = e.ships > e.caravans;
            // advance along the active leg
            let step = 1.0 / e.travel_ticks.max(1) as f32;
            e.pos += step;
            let t = e.pos.clamp(0.0, 1.0);
            let (sx, sy, gx, gy) = if e.outbound { (e.ox, e.oy, e.dx, e.dy) } else { (e.dx, e.dy, e.ox, e.oy) };
            let (cx, cy) = (sx + (gx - sx) * t, sy + (gy - sy) * t);
            // hazard roll
            let climate = (Self::koppen_peril(ko) + Self::koppen_peril(kd)) * 0.5;
            // Part III §2.1 — a HOSTILITY proxy, not a native faction (the model
            // has no unincorporated population to draw one from honestly — see
            // the plan's own "explicitly a hazard field" framing). A tick has no
            // per-cell geography to test "which province is this expedition in
            // right now", so the proxy is coarser than the plan's ask: venturing
            // toward a destination the backer has never even heard reported of
            // is riskier than a return trip to a province it already knows
            // something about. Falls to zero once the destination is at least
            // `KNOWN_REPORTED` — contact (or conquest) lowering hostility, per
            // the plan, without the model having to say which.
            // An EMPTY `known` map means "never seeded" (a hand-built fixture, or
            // a save mid-migration before `seed_knowledge` has run) — read as
            // KNOWN_ESTABLISHED here, the same permissive default `house_knows`/
            // `hub_knows` already use for the founding gate. Without this, every
            // expedition from a house whose knowledge was never seeded suffered
            // the worst-case hostility penalty for its entire existence — caught
            // by `a_house_records_every_head_it_has_had` going bankrupt on
            // repeated doomed expeditions.
            let dest_level = if e.dest_province < 0 || self.houses.get(owner).is_some_and(|h| h.known.is_empty()) {
                KNOWN_ESTABLISHED
            } else {
                self.houses.get(owner)
                    .and_then(|h| h.known.get(&(e.dest_province as u32)))
                    .map_or(0, |k| k.level)
            };
            let hostility = if dest_level >= KNOWN_SURVEYED { 0.0 }
                else if dest_level >= KNOWN_REPORTED { 0.15 } else { 0.35 };
            let peril = EXP_HAZARD_BASE * (0.7 + 1.3 * climate + hostility + if sea { 0.5 } else { 0.3 });
            let roll = hash01(seed, (tick as u64) ^ (e.id as u64).wrapping_mul(0x2545F4914F6CDD1D),
                (e.pos * 997.0) as u64);
            if roll < peril {
                let kr = hash01(seed, e.id as u64, tick as u64 ^ 0x51ED);
                let kind = Self::pick_hazard_kind(sea, ko, kd, kr);
                let loss = 0.12 + 0.34 * hash01(seed, e.id as u64 ^ 0xA5, tick as u64);
                e.arrived_frac -= loss;
                e.hazards.push(HazardEvent { tick, x: cx, y: cy, kind, losses: loss.min(1.0) });
                if e.arrived_frac <= 0.0 {
                    e.arrived_frac = 0.0;
                    e.status = 4;
                    // WORLD_AND_TRADE_MASTER_PLAN.md Part III §2.1 — a lost
                    // expedition still teaches its backer SOMETHING: the
                    // destination province is now `KNOWN_REPORTED` (never
                    // downgrading a level it already held).
                    if e.dest_province >= 0 {
                        let e2 = self.houses[owner].known.entry(e.dest_province as u32)
                            .or_insert(Known { level: KNOWN_REPORTED, since_tick: tick, source: -1 });
                        if e2.level < KNOWN_REPORTED { e2.level = KNOWN_REPORTED; e2.since_tick = tick; }
                    }
                    self.failed_expeditions.push(HazardEvent { tick, x: cx, y: cy, kind, losses: 1.0 });
                    if self.failed_expeditions.len() > EXP_FAILED_CAP {
                        let ov = self.failed_expeditions.len() - EXP_FAILED_CAP;
                        self.failed_expeditions.drain(0..ov);
                    }
                    let pi = self.prospect_idx(origin, dest);
                    self.route_prospects[pi].attempts = self.route_prospects[pi].attempts.saturating_add(1);
                    self.route_prospects[pi].last_tick = tick;
                    let (on, dn) = (self.hubs.get(origin).map(|h| h.name.clone()).unwrap_or_default(),
                                    self.hubs.get(dest).map(|h| h.name.clone()).unwrap_or_default());
                    self.journal.push(JournalEntry { tick, kind: "expedition".into(), hub: origin as i32,
                        good: -1, value: 0.0,
                        text: format!("{}'s expedition to {} is lost ({})", e.leader, dn,
                            HAZARD_LABEL[kind as usize]) });
                    let _ = on;
                    self.expeditions[ei] = e;
                    continue;
                }
            }
            // arrival / return
            if e.pos >= 1.0 {
                if e.outbound {
                    let base = self.goods.get(good).map(|g| g.base_value).unwrap_or(1.0).max(0.2);
                    e.revenue = e.cargo_qty * base * 2.2 * e.arrived_frac;
                    e.outbound = false;
                    e.pos = 0.0;
                    e.status = 2; // returning
                } else {
                    e.status = 3; // succeeded (banked on return)
                    let profit = e.revenue - e.cost;
                    if owner < self.houses.len() && !self.houses[owner].defunct {
                        self.houses[owner].wealth += e.revenue;
                        // Part III §1.2/§2 — a RETURNED expedition surveys its
                        // destination (the founding gate) and reports its
                        // neighbours, exactly `seed_knowledge`'s own `establish`.
                        if e.dest_province >= 0 {
                            let dp = e.dest_province as u32;
                            self.houses[owner].known.insert(dp,
                                Known { level: KNOWN_SURVEYED, since_tick: tick, source: -1 });
                            for &np in self.prov_neighbors.get(dp as usize).map(|v| v.as_slice()).unwrap_or(&[]) {
                                let e2 = self.houses[owner].known.entry(np)
                                    .or_insert(Known { level: KNOWN_REPORTED, since_tick: tick, source: -1 });
                                if e2.level < KNOWN_REPORTED { e2.level = KNOWN_REPORTED; e2.since_tick = tick; }
                            }
                        }
                        // Phase 3.1 · a GOAL_REACH_PROVINCE goal succeeds when a
                        // BACKED expedition completes its round trip to the target
                        // province. `update_house_goal`'s yearly pass reads this state
                        // and does the actual closing/chronicling.
                        for g in self.houses[owner].goals.iter_mut() {
                            if g.kind == GOAL_REACH_PROVINCE && g.state == GOAL_PURSUING
                                && g.target_province == e.dest_province {
                                g.state = GOAL_ACHIEVED;
                            }
                        }
                    }
                    let pi = self.prospect_idx(origin, dest);
                    self.route_prospects[pi].attempts = self.route_prospects[pi].attempts.saturating_add(1);
                    self.route_prospects[pi].successes = self.route_prospects[pi].successes.saturating_add(1);
                    self.route_prospects[pi].cum_profit += profit;
                    self.route_prospects[pi].last_tick = tick;
                    let est = !self.route_prospects[pi].established
                        && self.route_prospects[pi].successes >= EXP_MIN_SUCCESSES
                        && self.route_prospects[pi].cum_profit >= EXP_EST_PROFIT;
                    let (att, suc) = (self.route_prospects[pi].attempts, self.route_prospects[pi].successes);
                    let dn = self.hubs.get(dest).map(|h| h.name.clone()).unwrap_or_default();
                    self.journal.push(JournalEntry { tick, kind: "expedition".into(), hub: dest as i32,
                        good: good as i32, value: profit,
                        text: format!("{}'s expedition to {} returns ({}% of the fleet survived)",
                            e.leader, dn, (e.arrived_frac * 100.0) as i32) });
                    if est {
                        self.route_prospects[pi].established = true;
                        self.establish_corridor(origin, dest, owner, good as u16, att, suc);
                    }
                }
            }
            self.expeditions[ei] = e;
        }
    }


    /// Establish a permanent corridor + found its port (coast) / caravanserai (land)
    /// villages on real geographic sites near the route. Culture assigned at founding.
    pub(crate) fn establish_corridor(&mut self, origin: usize, dest: usize, owner: usize,
        good: u16, attempts: u16, successes: u16) {
        let tick = self.tick;
        let (ox, oy) = (self.hubs[origin].x, self.hubs[origin].y);
        let (dx, dy) = (self.hubs[dest].x, self.hubs[dest].y);
        let km = self.hub_cell_dist(origin, dest) * EARTH_EQUATOR_KM / self.world_w.max(1.0);
        let sea_route = self.hubs[origin].coastal && self.hubs[dest].coastal;
        // Count is FORMULA-driven + randomised (not fixed spacing).
        let n_port = if sea_route {
            1 + (hash01(self.seed, tick as u64, origin as u64) < 0.4) as u32
        } else { 0 };
        let n_car = if sea_route { 0 } else {
            (km / EXP_DAY_MARCH_KM * (0.75 + 0.5 * hash01(self.seed, tick as u64 ^ 0xCA, dest as u64)))
                .floor().clamp(0.0, 3.0) as u32
        };
        let total = n_port + n_car;
        let culture = self.hub_culture.get(origin).cloned().unwrap_or_default();
        let (mut ports, mut caravanserais) = (Vec::new(), Vec::new());
        for i in 0..total {
            let want_coastal = i < n_port; // ports first (the sea landfalls)
            let f = (i as f32 + 1.0) / (total as f32 + 1.0);
            let (mx, my) = (ox + (dx - ox) * f, oy + (dy - oy) * f);
            let mut best = (usize::MAX, f32::MAX);
            for (si, s) in self.colonizable.iter().enumerate() {
                if s.coastal != want_coastal { continue; }
                let d2 = (s.x - mx).powi(2) + (s.y - my).powi(2);
                if d2 < best.1 { best = (si, d2); }
            }
            if best.0 == usize::MAX { continue; }
            if best.1.sqrt() > self.world_w * 0.10 { continue; } // must be near the route
            let site = self.colonizable.swap_remove(best.0);
            let idx = self.create_organic_town(origin, &site, CARAVAN_SEED_POP);
            if self.hub_culture.len() <= idx { self.hub_culture.resize(idx + 1, String::new()); }
            if !culture.is_empty() { self.hub_culture[idx] = culture.clone(); }
            let nm = self.hubs[idx].name.clone();
            let hid = self.hubs[idx].id;
            self.total_foundings += 1;
            if want_coastal {
                ports.push(hid);
                self.journal.push(JournalEntry { tick, kind: "founding".into(), hub: idx as i32,
                    good: -1, value: CARAVAN_SEED_POP,
                    text: format!("The port of {} is founded on the new sea-road", nm) });
            } else {
                caravanserais.push(hid);
                self.journal.push(JournalEntry { tick, kind: "founding".into(), hub: idx as i32,
                    good: -1, value: CARAVAN_SEED_POP,
                    text: format!("A caravanserai rises at {} along the new corridor", nm) });
            }
        }
        let owner_name = self.houses.get(owner).map(|h| h.name.clone()).unwrap_or_else(|| "a house".into());
        let good_name = self.goods.get(good as usize).map(|g| g.name.clone()).unwrap_or_default();
        let (on, dn) = (self.hubs[origin].name.clone(), self.hubs[dest].name.clone());
        self.journal.push(JournalEntry { tick, kind: "corridor".into(), hub: origin as i32,
            good: good as i32, value: successes as f32,
            text: format!("After {} ventures, {} opens a lasting {} corridor from {} to {}",
                attempts, owner_name, good_name, on, dn) });
        self.corridors.push(Corridor {
            a: origin as u32, b: dest as u32, owner: owner as i32, good,
            founded_tick: tick, attempts, successes, ports, caravanserais,
        });
    }


    /// CARAVANSERAIS: found a waystation near the midpoint of a long INLAND trade tie
    /// between two sizeable cities (a Silk-Road-style day's halt). It seeds small and
    /// can grow into a town. At most a few per year.
    pub(crate) fn maybe_found_caravanserai(&mut self, expansion_ok: bool) {
        if !expansion_ok || self.colonizable.is_empty() { return; }
        let tick = self.tick;
        let n = self.hubs.len();
        let min_gap = self.world_w * CARAVAN_MIN_GAP_FRAC;
        let near = self.world_w * CARAVAN_NEAR_MIDPOINT;
        let clear = self.world_w * CARAVAN_CLEAR_RADIUS;
        let mut made = 0u32;
        for a in 0..n {
            if made >= CARAVAN_MAX_PER_YEAR { break; }
            let ha = &self.hubs[a];
            if ha.is_estate || ha.abandoned || ha.coastal || ha.population < CARAVAN_CITY_MIN_POP { continue; }
            let comp = ha.component;
            let (ax, ay) = (ha.x, ha.y);
            // A distant INLAND trade partner (long land haul).
            let partner = self.neighbors.get(a).and_then(|v| v.iter().map(|&x| x as usize).find(|&b| {
                b < n && b != a && !self.hubs[b].is_estate && !self.hubs[b].abandoned
                    && !self.hubs[b].coastal && self.hubs[b].component == comp
                    && self.hub_cell_dist(a, b) >= min_gap
            }));
            let Some(b) = partner else { continue };
            if a > b { continue; } // each pair once
            if hash01(self.seed, tick as u64 ^ 0xCA5A, (a as u64) ^ ((b as u64) << 20)) > 0.4 { continue; }
            let (bx, by) = (self.hubs[b].x, self.hubs[b].y);
            let (mx, my) = ((ax + bx) * 0.5, (ay + by) * 0.5); // midpoint (cross-seam ties rare)
            // Already served? (a town near the midpoint)
            let occupied = (0..n).any(|h| h != a && h != b && !self.hubs[h].is_estate
                && !self.hubs[h].abandoned
                && ((self.hubs[h].x - mx).powi(2) + (self.hubs[h].y - my).powi(2)).sqrt() < clear);
            if occupied { continue; }
            // Nearest INLAND colonizable site to the midpoint.
            let mut best = (usize::MAX, near * near);
            for (i, s) in self.colonizable.iter().enumerate() {
                if s.coastal { continue; }
                let d2 = (s.x - mx).powi(2) + (s.y - my).powi(2);
                if d2 < best.1 { best = (i, d2); }
            }
            let Some(si) = (best.0 != usize::MAX).then_some(best.0) else { continue };
            let site = self.colonizable.swap_remove(si);
            let idx = self.create_organic_town(a, &site, CARAVAN_SEED_POP);
            let (an, bn, cn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone(), self.hubs[idx].name.clone());
            self.total_foundings += 1;
            self.journal.push(JournalEntry { tick, kind: "founding".into(), hub: idx as i32, good: -1,
                value: CARAVAN_SEED_POP,
                text: format!("A caravanserai rises at {} on the road between {} and {}", cn, an, bn) });
            made += 1;
        }
    }


    /// Whether province `p` already has a LIVE settlement in it. `hub_province`
    /// only covers hubs that existed at campaign start (colonies founded mid-
    /// campaign never get an entry — a known, bounded imprecision: at worst this
    /// under-detects "already settled" for a province a colony claimed earlier in
    /// THIS run, which just makes the empty-province bonus fire a little more
    /// often than strictly necessary, never incorrectly withholds it).
    fn province_is_settled(&self, p: i32) -> bool {
        if p < 0 { return true; }
        self.hub_province.iter().enumerate()
            .any(|(h, &hp)| hp == p && self.hubs.get(h).map(|x| !x.abandoned).unwrap_or(false))
    }

    pub(crate) fn maybe_found_settlement_colony(&mut self) {
        if self.colonizable.is_empty() { return; }
        let n_settle = self.hubs.iter().filter(|h| h.colony_kind == 1 && !h.autonomous).count();
        if n_settle >= MAX_SETTLEMENT_COLONIES { return; }
        // Founder: a large, food-secure, prosperous ordinary city under population
        // pressure with some treasury to commit.
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.colony_kind != 0 { continue; }
            // Large, prosperous, not in severe famine — the pressure to export people.
            if hub.population < COLONY_PARENT_MIN_POP || hub.starving > 0.7 { continue; }
            if hub.sent_prosperity < 0.25 { continue; }
            let score = hub.population * hub.sent_prosperity.clamp(0.0, 1.0) * (0.2 + hub.treasury);
            if score > best.1 { best = (h, score); }
        }
        let Some(founder) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // Best reachable FERTILE site (from the city; its prior colonies could relay
        // too, but the city alone is enough for v1).
        let nodes = vec![(self.hubs[founder].x, self.hubs[founder].y)];
        let cap = COLONY_MAX_KM * self.world_w / EARTH_EQUATOR_KM; // ≤ 2500 km from the metropolis
        let founder_coastal = self.hubs[founder].coastal;
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            // INLAND cities have no fleet tradition — they can only found INLAND
            // colonies; only a coastal metropolis colonizes the sea (user rule).
            if !founder_coastal && s.coastal { continue; }
            // WORLD_AND_TRADE_MASTER_PLAN.md Part III §1.2/§3 — a city cannot
            // found a colony in a province it has never surveyed.
            if !self.hub_knows(founder, s.province) { continue; }
            // Skip only the genuinely worthless: too lean to part-feed itself AND
            // poor in trade goods. Otherwise a colony may settle less-fertile land —
            // a trade-rich frontier is worth founding even on lean soil (its food
            // lifeline contracts cover the deficit).
            if s.fertility < COLONY_MIN_FERTILE && s.trade_value < COLONY_MIN_TRADE { continue; }
            let d = self.nearest_node_dist(&nodes, s.x, s.y);
            if d > cap { continue; }
            // Weight the PRIZE (trade goods) over self-sufficiency, PLUS the prized
            // site premiums: sea shore, river delta, land→sea chokepoint (toll points).
            let site_bonus = if s.coastal { 0.35 } else { 0.0 }
                + if s.delta { 0.60 } else { 0.0 }
                + if s.chokepoint { 0.80 } else { 0.0 };
            // EMPTY-PROVINCE bonus (user rule): a province with no settlement in it
            // yet is a real gap on the map, worth reaching for over a marginally
            // better site inside an already-settled one. `s.province` is a one-time
            // snapshot from generation time (-1 = unknown, e.g. no province layer —
            // never bonused, so a provinceless world is bit-identical to before).
            let province_bonus = if s.province >= 0 && !self.province_is_settled(s.province) {
                EMPTY_PROVINCE_FOUND_BONUS
            } else { 0.0 };
            let score = (0.25 + 0.35 * s.fertility + 1.0 * s.trade_value + site_bonus + province_bonus) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        // ── Raise the capital (affordability checked BEFORE anything is committed) ──
        let need = COLONY_FOUND_COST;
        // The city funds as much as its treasury allows (can self-fund); a resident
        // house and a local bank top up any shortfall (the joint-stock part).
        let city_put = self.hubs[founder].treasury.min(need).max(0.0);
        let house_idx = self.strongest_house_at(founder).filter(|&h| !self.houses[h].defunct);
        let house_put = house_idx.map(|h| (need - city_put).min(self.houses[h].wealth * 0.3).max(0.0)).unwrap_or(0.0);
        // A bank ON THE SAME CONTINENT (component) is REQUIRED — it stakes the venture
        // and its family becomes the colony's bank + mint. No such bank → no colony.
        let comp = self.hubs[founder].component;
        let bank_idx = self.banks.iter().position(|b| !b.defunct
            && self.hubs.get(b.seat as usize).map(|s| s.component == comp).unwrap_or(false));
        let Some(bank_idx) = bank_idx else { return };
        let bank_lend = (need - city_put - house_put).min(self.banks[bank_idx].reserves * 0.5).max(0.0);
        let raised = city_put + house_put + bank_lend;
        if raised < need * 0.8 { return; } // not enough backing — wait
        // Commit: debit each backer and record proportional shares.
        let mut backers: Vec<(u8, u32, f32)> = Vec::new();
        if city_put > 0.5 { self.hubs[founder].treasury -= city_put; backers.push((0, founder as u32, city_put / raised)); }
        if let (Some(h), true) = (house_idx, house_put > 0.5) { self.houses[h].wealth -= house_put; backers.push((1, h as u32, house_put / raised)); }
        if bank_lend > 0.5 {
            self.banks[bank_idx].reserves -= bank_lend;
            self.banks[bank_idx].loans.push(Loan {
                borrower_house: -1, borrower_polis: founder as i32, principal: bank_lend,
                outstanding: bank_lend, rate: BANK_LOAN_RATE, start_tick: self.tick,
                term_ticks: TICKS_PER_YEAR * 8, purpose: "colony".into(),
            });
        }
        // The bank is always a backer (its family will mint the colony's coin).
        backers.push((2, bank_idx as u32, (bank_lend.max(0.1)) / raised));
        let site = self.colonizable.swap_remove(si);
        // Migration: emigrants seed the colony and RELIEVE the crowded parent.
        let seed = (self.hubs[founder].population * COLONY_MIGRATION_FRAC).max(80.0);
        self.hubs[founder].population = (self.hubs[founder].population - seed)
            .max(self.hubs[founder].founding_pop * 0.3);
        let new = self.create_market_colony(founder, &site, backers, seed);
        // The backing merchant house OPENS AN OFFICE in the new colony — a permanent
        // foothold in its market (the family that staked the venture trades there).
        if let Some(h) = house_idx {
            if !self.houses[h].offices.contains(&(new as u32)) {
                self.houses[h].offices.push(new as u32);
                let (cn, city) = (self.houses[h].name.clone(), self.hubs[new].name.clone());
                self.houses[h].events.push(HouseEvent { tick: self.tick, kind: "branch".into(),
                    text: format!("{} opens a counting-house in {}", cn, city) });
            }
        }
        // The backing bank's family becomes the colony's main bank + mint: it seats
        // the colony's council and strikes its coin.
        self.hubs[new].main_bank = bank_idx as i32;
        let bank_house = self.banks[bank_idx].house as i32;
        self.hubs[new].council_house = bank_house;
        let cshort = self.hubs[new].name.replace(" (colony)", "");
        self.hubs[new].coin_name = format!("{} mark", cshort);
        self.hubs[new].coin_trust = 0.40;
        self.hubs[new].mint_fineness = 1.0;
        // Metropolis trade MONOPOLY: bar non-backer houses from the colony's market.
        self.apply_colony_charter(new);
        // Designate the food source + commission the founding grain fleet. Seed the
        // reserve so a fresh colony isn't instantly food-short before its first run.
        self.hubs[new].reserve_food = 60.0;
        let est_deficit_monthly = (self.hubs[new].population * 0.30).max(300.0);
        self.designate_colony_supply(new, est_deficit_monthly);
        let (pname, cname) = (self.hubs[founder].name.clone(), self.hubs[new].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "colony".into(), hub: new as i32, good: -1, value: 1.0,
            text: format!("{} founds the settlement colony {}", pname, cname),
        });
    }


    /// GRAIN COLONY (the Greek Crimea pattern): a large city gripped by a SUSTAINED food
    /// shortage plants a farming colony on the most FERTILE reachable site to secure its
    /// grain. Unlike the crowding-driven settlement colony this is a survival move — the
    /// city SELF-FUNDS it (no bank required, so it can act on hunger alone) and it can
    /// sit near existing cities. The colony is biased to farm; its surplus flows back to
    /// the hungry metropolis through the ordinary market.
    pub(crate) fn maybe_found_food_colony(&mut self) {
        if self.colonizable.is_empty() { return; }
        let n_settle = self.hubs.iter().filter(|h| h.colony_kind == 1 && !h.autonomous).count();
        if n_settle >= MAX_SETTLEMENT_COLONIES { return; }
        // Founder: a big city under real food stress with some treasury to commit.
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.colony_kind != 0 { continue; }
            if hub.population < COLONY_PARENT_MIN_POP || hub.treasury < FOOD_COLONY_MIN_TREASURY { continue; }
            let stress = hub.starving.max((-hub.food_balance).max(0.0)).clamp(0.0, 1.0);
            if stress < FOOD_COLONY_STARVE_MIN { continue; } // not hungry enough
            let score = hub.population * stress * (0.2 + hub.treasury);
            if score > best.1 { best = (h, score); }
        }
        let Some(founder) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // Best reachable FERTILE (grain) site — fertility dominates, nearer is better.
        let nodes = vec![(self.hubs[founder].x, self.hubs[founder].y)];
        let cap = COLONY_MAX_KM * self.world_w / EARTH_EQUATOR_KM; // ≤ 2500 km from the metropolis
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            if s.fertility < COLONY_MIN_FERTILE { continue; } // must be farmable
            // WORLD_AND_TRADE_MASTER_PLAN.md Part III §1.2/§3 — same survey gate.
            if !self.hub_knows(founder, s.province) { continue; }
            let d = self.nearest_node_dist(&nodes, s.x, s.y);
            if d > cap { continue; }
            let score = (0.2 + 1.6 * s.fertility) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        // Self-funded survival venture: the city commits the founding cost; a resident
        // house may chip in. NO bank required (that's the whole point — hunger acts).
        let need = COLONY_FOUND_COST;
        let city_put = self.hubs[founder].treasury.min(need).max(0.0);
        if city_put < need * 0.5 { return; } // can't afford even half → wait
        let house_idx = self.strongest_house_at(founder).filter(|&h| !self.houses[h].defunct);
        let house_put = house_idx.map(|h| (need - city_put).min(self.houses[h].wealth * 0.3).max(0.0)).unwrap_or(0.0);
        let raised = (city_put + house_put).max(EPS);
        let mut backers: Vec<(u8, u32, f32)> = Vec::new();
        self.hubs[founder].treasury -= city_put;
        backers.push((0, founder as u32, city_put / raised));
        if let (Some(h), true) = (house_idx, house_put > 0.5) {
            self.houses[h].wealth -= house_put; backers.push((1, h as u32, house_put / raised));
        }
        let comp = self.hubs[founder].component;
        let bank_idx = self.banks.iter().position(|b| !b.defunct
            && self.hubs.get(b.seat as usize).map(|s| s.component == comp).unwrap_or(false));
        let site = self.colonizable.swap_remove(si);
        let seed = (self.hubs[founder].population * COLONY_MIGRATION_FRAC).max(80.0);
        self.hubs[founder].population = (self.hubs[founder].population - seed)
            .max(self.hubs[founder].founding_pop * 0.3);
        let new = self.create_market_colony(founder, &site, backers, seed);
        // Bias it to FARM — its grain surplus is the whole reason it exists.
        let ng = self.goods.len();
        let pop = self.hubs[new].population;
        for g in 0..ng {
            if self.goods[g].food {
                self.hubs[new].base_per_capita[g] *= FOOD_COLONY_FARM_MULT;
                self.hubs[new].production[g] = self.hubs[new].base_per_capita[g] * pop;
                let v = self.hubs[new].production[g];
                stock_set_total(&mut self.hubs[new].stock, g, v);
            }
        }
        self.hubs[new].reserve_food = 60.0;
        let cshort = self.hubs[new].name.replace(" (colony)", "");
        self.hubs[new].coin_name = format!("{} mark", cshort);
        self.hubs[new].coin_trust = 0.40;
        if let Some(b) = bank_idx { self.hubs[new].main_bank = b as i32; }
        self.apply_colony_charter(new);
        // A grain colony is food-secure by design (no import fleet); just log it.
        let (pname, cname) = (self.hubs[founder].name.clone(), self.hubs[new].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "colony".into(), hub: new as i32, good: -1, value: 1.0,
            text: format!("Hungry {} plants the grain colony {} to secure its bread", pname, cname),
        });
    }


    /// Found a full MARKET hub (is_estate = false) as a settlement colony — a mini
    /// version of its founder so it has a real local economy from day one.
    pub(crate) fn create_market_colony(&mut self, founder: usize, site: &ColonizeSite,
                            backers: Vec<(u8, u32, f32)>, seed_pop: f32) -> usize {
        let ng = self.goods.len();
        // F6 (CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 6) — the site was
        // CHOSEN for its resources (`trade_value` dominates the scoring in
        // `compute_colonizable_sites`); a colony that then produces a flat 60% of
        // whatever the METROPOLIS produced ignores the very reason the spot was
        // picked. `site.belt` is a real per-good 0..1 physical yield potential at
        // the site itself (empty on a pre-slice-6 save — a true no-op, the founder-
        // basket fallback below is exactly the old behaviour). Blended, not a
        // straight replacement: a colony still needs SOME of everything a
        // settlement needs (a smith, a weaver) even on a site whose belt names only
        // one or two goods — `try_found_house_outpost`'s single-commodity Kontor is
        // the correct model for a post that exists to work ONE cargo; a settlement
        // colony is a town, not a factory.
        let base_per_capita: Vec<f32> = if site.belt.len() == ng {
            let founder_bpc = &self.hubs[founder].base_per_capita;
            let belt_sum: f32 = site.belt.iter().sum();
            (0..ng).map(|g| {
                let from_founder = founder_bpc[g] * 0.6;
                if belt_sum > 1e-3 {
                    // Reallocate the SAME total output the founder-basket approach
                    // would seed, but toward the goods this site's own belt names —
                    // the site's product mix, not its production LEVEL (which the
                    // founder's own basket scale still anchors).
                    let total: f32 = founder_bpc.iter().map(|v| v * 0.6).sum();
                    let from_site = total * (site.belt[g] / belt_sum);
                    // Blend rather than replace: a colony still needs a little of
                    // everything a town needs, not only its named export.
                    from_founder * 0.35 + from_site * 0.65
                } else {
                    from_founder
                }
            }).collect()
        } else {
            self.hubs[founder].base_per_capita.iter().map(|v| v * 0.6).collect()
        };
        let pop = seed_pop.max(1.0);
        let production: Vec<f32> = base_per_capita.iter().map(|v| v * pop).collect();
        let id = 100_000 + self.hubs.len() as u32;
        // A fresh culture-styled name (like any founded town) so a metropolis's
        // several colonies are DISTINCT places, not duplicate "New X (colony)".
        // The `colony_kind` field + the Colonial-panel tree carry the relationship.
        let name = crate::sim::names::gen_name(
            site.x.max(0.0) as u32, site.y.max(0.0) as u32,
            self.world_w as u32, self.world_h());
        let component = self.hubs[founder].component;
        self.hubs.push(TickHub {
            known: std::collections::HashMap::new(),
            id, x: site.x, y: site.y, name, population: pop, founding_pop: pop,
            stock: {
                let mut s = vec![0.0f32; ng * GRADE_BANDS];
                for (g, &p) in production.iter().enumerate() { stock_set_total(&mut s, g, p); }
                s
            },
            price: self.goods.iter().map(|g| g.base_value).collect(),
            production, grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: site.koppen, coastal: site.coastal, river: false, component,
            export_earn: 0.0, import_spend: 0.0, mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5,
            sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(), in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, last_upgrade_tick: self.tick, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, coin_history: Vec::new(), debt_principal: 0.0, debt_coupon: 0.0, debt_holders: Vec::new(), mint_bullion_ratio: 1.0, has_mint: false,
            quality: vec![0.0f32; ng], stolen_good: -1, stolen_from: -1,
            colony_kind: 1, colony_stage: 1, autonomous: false, founder_hub: founder as i32, backers,
            reserve_food: 30.0, reserve_cap: 365.0, supply_years: 0.0, colony_founded_tick: self.tick,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), food_export_lock: 0, export_ban_until: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: self.tick, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
            tier: 0, standing: 0.0, war_cooldown_until: 0, captor_since: 0, realm: -1, realm_role: 0,
            wh_capacity: 0.0, wh_spoiled_month: Vec::new(), wh_last_month: Vec::new(), supply_accum: Vec::new(), shares: Vec::new(), monthly: Vec::new(), brand_chronicled: false, bad_years: 0, disaster_repair_mult: 0.0,
        });
        self.total_foundings += 1; // Atlas 2.0 lifecycle counter (colony ventures too)
        self.routes_dirty = true;
        self.hubs.len() - 1
    }


    /// (Re)sign the colony's civic supply contracts — fill food / reserve /
    /// preservative slots from the nearest reachable suppliers on the metropolis's
    /// continent (same `component`): the metropolis itself, other food-producing
    /// cities/estates, and a salt-type producer for preservatives. Stale rows whose
    /// (Re)designate a settlement colony's food SOURCE — the reachable same-component
    /// hub with the largest shippable grain surplus (the metropolis gets a nearness
    /// edge) — and INVEST the backers' money in enough dedicated supply ships to carry
    /// the monthly deficit steadily. Also refreshes the display roster row.
    pub(crate) fn designate_colony_supply(&mut self, c: usize, deficit_monthly: f32) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let comp = self.hubs[c].component;
        let metro = self.hubs[c].founder_hub;
        // Best food source: the biggest reachable grain PRODUCER (a stable choice from day
        // one — before any stock has accumulated), nudged toward the metropolis/nearer.
        // The daily run is still capped by the source's real spare STOCK, so a designated
        // source that has nothing spare simply ships nothing until its granary fills.
        let food_output = |me: &Self, h: usize| -> f32 {
            let mut p = 0.0;
            for g in 0..me.goods.len() { if me.goods[g].food { p += me.hubs[h].production[g].max(0.0); } }
            p
        };
        let mut best = (-1i32, 0.0f32);
        for h in 0..n {
            if h == c || self.hubs[h].component != comp || self.hubs[h].population <= 1.0 { continue; }
            let d = self.days.get(h * n + c).copied().unwrap_or(f32::INFINITY);
            if !d.is_finite() { continue; }
            let fp = food_output(self, h);
            if fp <= EPS { continue; }
            let score = fp * (if h as i32 == metro { 1.3 } else { 1.0 }) / (1.0 + d * 0.02);
            if score > best.1 { best = (h as i32, score); }
        }
        self.hubs[c].supply_source = best.0;
        // Invest in ships until the fleet can carry the monthly deficit (steady supply),
        // as far as the backers can afford — cities pour money into the grain run. A
        // food-secure colony (no deficit) needs no fleet.
        let required = if deficit_monthly <= EPS { 0 } else {
            ((deficit_monthly / SUPPLY_SHIP_CAPACITY).ceil().max(SUPPLY_SHIPS_AT_FOUNDING as f32)
                as u32).min(MAX_SUPPLY_SHIPS)
        };
        while self.hubs[c].supply_ships < required {
            if !self.buy_colony_supply_ship(c) { break; }
        }
        // Display roster: the designated source on the grain run + a preservative row.
        self.colony_supply.retain(|s| s.colony_hub != c as u32);
        if best.0 >= 0 {
            let fg = (0..ng).find(|&g| self.goods[g].food).unwrap_or(0);
            let carried = (self.hubs[c].supply_ships as f32 * SUPPLY_SHIP_CAPACITY)
                .min(deficit_monthly.max(1.0));
            self.colony_supply.push(ColonySupply {
                colony_hub: c as u32, supplier_hub: best.0 as u32, good: fg,
                monthly_qty: carried, category: 0,
            });
            if let Some(pg) = (0..ng).find(|&g| self.goods[g].name.to_lowercase().contains("salt")) {
                self.colony_supply.push(ColonySupply {
                    colony_hub: c as u32, supplier_hub: best.0 as u32, good: pg,
                    monthly_qty: carried * 0.2, category: 2,
                });
            }
        }
    }


    /// Commission ONE more dedicated supply ship for colony `c`, paid by its backers
    /// (metropolis treasury first, then the backing bank's reserves, then a backing
    /// house's wealth). Returns false — and buys nothing — if they can't afford it.
    pub(crate) fn buy_colony_supply_ship(&mut self, c: usize) -> bool {
        if self.hubs[c].supply_ships >= MAX_SUPPLY_SHIPS { return false; }
        let cost = SUPPLY_SHIP_COST;
        // Tally what the backers can put up, in payment order.
        let m = self.hubs[c].founder_hub;
        let metro_pool = if m >= 0 && (m as usize) < self.hubs.len() {
            self.hubs[m as usize].treasury.max(0.0)
        } else { 0.0 };
        let backers = self.hubs[c].backers.clone();
        let mut bank_pool = 0.0;
        let mut house_pool = 0.0;
        for (kind, idx, _) in &backers {
            match kind {
                2 => { if let Some(b) = self.banks.get(*idx as usize) { if !b.defunct { bank_pool += b.reserves.max(0.0); } } }
                1 => { if let Some(hh) = self.houses.get(*idx as usize) { if !hh.defunct { house_pool += hh.wealth.max(0.0); } } }
                _ => {}
            }
        }
        if metro_pool + bank_pool + house_pool < cost { return false; }
        // Debit metro → bank(s) → house(s) in order.
        let mut owed = cost;
        if m >= 0 && (m as usize) < self.hubs.len() {
            let take = self.hubs[m as usize].treasury.max(0.0).min(owed);
            self.hubs[m as usize].treasury -= take; owed -= take;
        }
        for (kind, idx, _) in &backers {
            if owed <= EPS { break; }
            if *kind == 2 { if let Some(b) = self.banks.get_mut(*idx as usize) { if !b.defunct {
                let take = b.reserves.max(0.0).min(owed); b.reserves -= take; owed -= take; } } }
        }
        for (kind, idx, _) in &backers {
            if owed <= EPS { break; }
            if *kind == 1 { if let Some(hh) = self.houses.get_mut(*idx as usize) { if !hh.defunct {
                let take = hh.wealth.max(0.0).min(owed); hh.wealth -= take; owed -= take; } } }
        }
        self.hubs[c].supply_ships += 1;
        true
    }


    /// Yearly REVIVAL: a long-dead ruin whose region has since recovered is
    /// resettled — pioneers refound a small town on the old site (its buried
    /// productive potential, `base_per_capita`, was preserved at abandonment, so
    /// the extractor refills its trade next tick). Needs a thriving, food-secure
    /// living city nearby in the same trade region and a cooling-off period.
    pub(crate) fn resettle_pass(&mut self, expansion_ok: bool) {
        if !expansion_ok { return; }
        let tick = self.tick;
        let reach = (self.world_w * RESETTLE_REACH_FRAC).max(1.0);
        let mut revivals = 0u32;
        for h in 0..self.hubs.len() {
            if revivals >= RESETTLE_MAX_PER_YEAR { break; }
            let hub = &self.hubs[h];
            if !hub.abandoned || hub.is_estate { continue; }
            if tick.saturating_sub(hub.died_tick) < RESETTLE_COOLDOWN_YEARS * TICKS_PER_YEAR { continue; }
            // The old site must still carry productive potential to refound onto.
            if hub.base_per_capita.iter().all(|&v| v <= 0.0) { continue; }
            let comp = hub.component;
            let (hx, hy) = (hub.x, hub.y);
            let mut patron_ok = false;
            for o in 0..self.hubs.len() {
                if o == h { continue; }
                let nb = &self.hubs[o];
                if nb.is_estate || nb.abandoned || nb.component != comp { continue; }
                if nb.population < RESETTLE_PATRON_MIN_POP || nb.sent_prosperity < 0.5
                    || nb.food_balance < 0.1 { continue; }
                let mut dx = (nb.x - hx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = nb.y - hy;
                if dx * dx + dy * dy <= reach * reach { patron_ok = true; break; }
            }
            if !patron_ok { continue; }
            if hash01(self.seed, tick as u64 ^ 0x2E51_1F, h as u64) > RESETTLE_PROB { continue; }
            self.revive_hub(h);
            revivals += 1;
        }
    }


    /// Yearly: settlement colonies (re)sign supply, may collapse if starved, graduate
    /// (gated by ≥5yr supply + population + buildings), pay dividends, and — once a
    /// mature self-sustaining city at year 70 — wage a war of independence.
    pub(crate) fn colony_pass(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].colony_kind != 1 || self.hubs[h].autonomous { continue; }
            // (The food source + dedicated fleet are refreshed monthly by the daily
            // lifeline in `update_food_and_starvation`, so no yearly re-sign here.)
            // COLLAPSE: empty reserve + severe sustained starvation → the lifeline failed.
            if self.hubs[h].reserve_food <= 0.0 && self.hubs[h].starving > 0.8 {
                self.collapse_colony(h);
                continue;
            }
            let pop = self.hubs[h].population;
            // GROWTH GATE: advance only with ≥5yr unbroken supply AND population AND
            // enough buildings (stage 2 needs 1, stage 3 needs 2, stage 4 needs 3).
            let pop_stage: u8 = if pop >= 40_000.0 { 4 } else if pop >= 15_000.0 { 3 }
                else if pop >= 4_000.0 { 2 } else { 1 };
            let nbuild = self.hubs[h].structures.len() as u8;
            let supplied = self.hubs[h].supply_years >= 5.0;
            let mut new_stage = self.hubs[h].colony_stage.max(1);
            while new_stage < pop_stage && supplied && nbuild >= new_stage /* stageN→N+1 needs N buildings */ {
                new_stage += 1;
            }
            if new_stage > self.hubs[h].colony_stage {
                self.hubs[h].colony_stage = new_stage;
                let nm = self.hubs[h].name.clone();
                let label = ["", "outpost", "colony", "town", "city"][new_stage as usize];
                self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32,
                    good: -1, value: new_stage as f32, text: format!("{} grows into a {}", nm, label) });
            }
            // Dividends from the colony's trade surplus — small & capped so fortunes
            // stay bounded (the colony's earnings would otherwise pool locally).
            let surplus = (self.hubs[h].trade_wealth.max(0.0) * pop * COLONY_DIVIDEND_RATE).min(2.0);
            if surplus > 0.01 {
                for (kind, idx, share) in self.hubs[h].backers.clone() {
                    let cut = surplus * share;
                    match kind {
                        0 => { if (idx as usize) < self.hubs.len() { self.hubs[idx as usize].treasury += cut; } }
                        1 => { if (idx as usize) < self.houses.len() && !self.houses[idx as usize].defunct { self.houses[idx as usize].wealth += cut; } }
                        2 => { if (idx as usize) < self.banks.len() && !self.banks[idx as usize].defunct { self.banks[idx as usize].reserves += cut; } }
                        _ => {}
                    }
                }
            }
            // INDEPENDENCE: a mature (≥50y), TOWN-stage-or-better, well-supplied
            // colony rebels — war if the metropolis still stands, peacefully if it
            // has fallen. Relaxed from ≥70y/city-stage(40k) to ≥50y/town-stage(15k)
            // so colonies actually reach independence and become full free cities
            // (make_colony_independent already promotes colony_kind→0).
            let age = tick.saturating_sub(self.hubs[h].colony_founded_tick) / TICKS_PER_YEAR;
            if age >= 50 && self.hubs[h].colony_stage >= 3 && supplied
                && tick > self.hubs[h].indep_cooldown_until && self.hubs[h].war_with < 0 {
                let m = self.hubs[h].founder_hub;
                let metro_alive = m >= 0 && (m as usize) < self.hubs.len()
                    && self.hubs[m as usize].population >= 100.0 && self.hubs[m as usize].war_with < 0;
                if metro_alive {
                    self.declare_independence_war(h, m as usize);
                } else {
                    // Metropolis has fallen (Tyre besieged) → peaceful drift, founding
                    // dynasty inherits.
                    self.make_colony_independent(h, false);
                }
            }
        }
    }


    /// Metropolis trade MONOPOLY (charter): while the colony is a dependency, bar
    /// every house that is NOT a backer or the metropolis's council family from the
    /// colony's market. Reuses the `house_barred` exclusion list (hub indices).
    pub(crate) fn apply_colony_charter(&mut self, h: usize) {
        let allow: std::collections::HashSet<u32> = self.hubs[h].backers.iter()
            .filter(|(k, _, _)| *k == 1).map(|(_, i, _)| *i).collect();
        let metro = self.hubs[h].founder_hub;
        let metro_house = if metro >= 0 && (metro as usize) < self.hubs.len() {
            self.hubs[metro as usize].council_house
        } else { -1 };
        self.house_barred.resize(self.houses.len(), Vec::new());
        for hi in 0..self.houses.len() {
            if allow.contains(&(hi as u32)) || hi as i32 == metro_house { continue; }
            if let Some(v) = self.house_barred.get_mut(hi) {
                if !v.contains(&(h as u32)) { v.push(h as u32); }
            }
        }
    }


    /// Lift a colony's charter (on independence/collapse): unbar every house.
    pub(crate) fn lift_colony_charter(&mut self, h: usize) {
        for v in self.house_barred.iter_mut() { v.retain(|&x| x != h as u32); }
    }


    /// A colony's lifeline failed: its bank's loan defaults (a loss that can sink the
    /// bank → crash), backers lose their stake, and the settlement dies out.
    pub(crate) fn collapse_colony(&mut self, h: usize) {
        let tick = self.tick;
        let nm = self.hubs[h].name.clone();
        let mb = self.hubs[h].main_bank;
        if mb >= 0 && (mb as usize) < self.banks.len() {
            let bi = mb as usize;
            let mut writeoff = 0.0;
            self.banks[bi].loans.retain(|l| {
                if l.borrower_polis == h as i32 && l.purpose == "colony" { writeoff += l.outstanding; false } else { true }
            });
            self.banks[bi].losses += writeoff;
        }
        self.colony_supply.retain(|s| s.colony_hub != h as u32);
        self.lift_colony_charter(h);
        self.hubs[h].colony_kind = 0;
        self.hubs[h].autonomous = false;
        self.hubs[h].founder_hub = -1;
        self.hubs[h].main_bank = -1;
        self.hubs[h].backers.clear();
        self.hubs[h].population = 0.0; // the dead-city marker handles the rest
        // Atlas 2.0: mark it truly DEAD — without this the founding-pop floor in the
        // population pass resurrected collapsed colonies at 10% strength next tick.
        self.hubs[h].abandoned = true;
        self.hubs[h].died_tick = tick;
        self.hubs[h].died_cause = "famine".into(); // the lifeline failed
        for v in self.hubs[h].stock.iter_mut() { *v = 0.0; }
        for v in self.hubs[h].production.iter_mut() { *v = 0.0; }
        self.total_abandonments += 1;
        self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32, good: -1,
            value: 0.0, text: format!("{} collapses — its food lifeline failed and the settlement is abandoned", nm) });
    }


    /// Free a colony from its metropolis (drop dependency, the `(colony)` tag, charter
    /// and backers) and hand over CONTROL, Carthage-style:
    ///   • peaceful (via_war=false) → the FOUNDING HOUSE (the trade dynasty that built
    ///     it — the Magonids of Carthage) inherits the free city's council;
    ///   • won by WAR (via_war=true) → the mother city's house is EXPELLED and a NEW
    ///     revolutionary trade family rises to lead it.
    pub(crate) fn make_colony_independent(&mut self, h: usize, via_war: bool) {
        let tick = self.tick;
        let nm = self.hubs[h].name.replace(" (colony)", "");
        // Founding merchant house = the house backer of the joint-stock venture.
        let founding_house = self.hubs[h].backers.iter()
            .find(|(k, _, _)| *k == 1).map(|&(_, i, _)| i as usize)
            .filter(|&i| i < self.houses.len() && !self.houses[i].defunct);
        let metro = self.hubs[h].founder_hub;
        let metro_house = if metro >= 0 && (metro as usize) < self.hubs.len() {
            self.hubs[metro as usize].council_house
        } else { -1 };
        self.hubs[h].colony_kind = 0;
        self.hubs[h].autonomous = true;
        self.hubs[h].founder_hub = -1;
        self.hubs[h].backers.clear();
        self.colony_supply.retain(|s| s.colony_hub != h as u32);
        self.lift_colony_charter(h);
        self.hubs[h].name = nm.clone();
        if via_war {
            // Expel the mother city's house (its office here + bar it), then seat a NEW
            // revolutionary family that leads the freed city.
            if metro_house >= 0 && (metro_house as usize) < self.houses.len() {
                let mh = metro_house as usize;
                self.houses[mh].offices.retain(|&o| o as usize != h);
                self.house_barred.resize(self.houses.len(), Vec::new());
                if let Some(v) = self.house_barred.get_mut(mh) {
                    if !v.contains(&(h as u32)) { v.push(h as u32); }
                }
            }
            let new_house = self.found_house_at(h);
            self.hubs[h].council_house = new_house.map(|i| i as i32).unwrap_or(-1);
            let mn = if metro >= 0 && (metro as usize) < self.hubs.len() {
                self.hubs[metro as usize].name.clone()
            } else { "its metropolis".into() };
            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32,
                good: -1, value: 0.0,
                text: format!("{} throws off {} in a war of independence; a new merchant family takes the reins", nm, mn) });
        } else {
            // Peaceful drift: the founding dynasty inherits the free city.
            if let Some(fh) = founding_house {
                self.hubs[h].council_house = fh as i32;
                if !self.houses[fh].offices.contains(&(h as u32)) {
                    self.houses[fh].offices.push(h as u32);
                }
                let (hn, cn) = (self.houses[fh].name.clone(), nm.clone());
                self.houses[fh].events.push(HouseEvent { tick, kind: "colony".into(),
                    text: format!("{} inherits {} as a free city", hn, cn) });
            }
            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32,
                good: -1, value: 0.0,
                text: format!("{} comes peacefully into its own as a free city under its founding house", nm) });
        }
    }


    /// Phase 5 (flavour) · DIASPORA quarters: a house with a far-flung office founds a
    /// resident QUARTER (fondaco) there — a small standing-influence foothold + a beat.
    pub(crate) fn run_diaspora(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0xD1A5, 0) >= DIASPORA_YEARLY_CHANCE { return; }
        let world_w = self.world_w.max(1.0);
        // Collect (house, distant office hub) candidates.
        let mut cand: Vec<(usize, u32)> = Vec::new();
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub as usize;
            if home >= self.hubs.len() { continue; }
            let (hx, hy) = (self.hubs[home].x, self.hubs[home].y);
            for &off in &self.houses[hi].offices {
                let o = off as usize;
                if o >= self.hubs.len() { continue; }
                let mut dx = (self.hubs[o].x - hx).abs();
                if world_w > 1.0 { dx = dx.min(world_w - dx); }
                let dy = self.hubs[o].y - hy;
                if (dx * dx + dy * dy).sqrt() > world_w * 0.20 { cand.push((hi, off)); }
            }
        }
        if cand.is_empty() { return; }
        let (hi, off) = cand[((hash01(self.seed, yr as u64, 0xD1B) * cand.len() as f32) as usize) % cand.len()];
        // Small standing-influence foothold at the host city.
        if let Some(e) = self.houses[hi].influence.iter_mut().find(|e| e.0 == off) {
            e.1 = (e.1 + 0.05).min(1.0);
        } else {
            self.houses[hi].influence.push((off, 0.05));
        }
        let (hn, city) = (self.houses[hi].name.clone(),
            self.hubs.get(off as usize).map(|h| h.name.clone()).unwrap_or_default());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "diaspora".into(), hub: off as i32, good: -1, value: 0.0,
            text: format!("{} founds a merchant quarter in {} — a diaspora takes root.", hn, city),
        });
    }
}
