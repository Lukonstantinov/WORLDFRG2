//! Realms — `docs/REALM_AND_GOVERNMENT_PLAN.md`, phase R1b. The proclamation trigger
//! and the house→crown transfer. Everything here is a no-op on a campaign with no
//! province layer (rule 25) or before `REALM_YEAR_FLOOR`, exactly like the land pass
//! it depends on.
//!
//! **Scope note.** This ships the entity, the trigger, and the wealth/debt/territory
//! transfer, plus the two guards that keep a coronation from being immediately undone
//! (`update_solvency` and `apply_wealth_sinks` in `mod.rs` now skip a crowned house).
//! It does NOT yet redirect a crowned house's estates/fleets/warehouses/monopoly
//! participation onto the realm — `recompute_monopolies_and_power`,
//! `pay_to_regain_markets` and `sync_and_stock_warehouses` still read/write the
//! (now-frozen) house record exactly as before a coronation. Untangling "the crown
//! operates its inherited trade assets" from "the dynasty used to trade them" is a
//! real design question, not a guard to improvise under time pressure — it is named
//! here rather than silently left implicit, and belongs to a follow-up pass before a
//! realm can be called a genuinely operating trading crown in the full sense §3.2
//! describes.
use super::*;

/// A small, deterministic naming set — placeholder for the culture-derived
/// vocabulary the plan's §7 brainstorms (titles drawn from a people's own language
/// kit, "lugal"/"nesu"/"basileus" rather than a global word list). Getting the ENTITY
/// right is R1b's job; a richer namer is a follow-up that touches no game state.
const REALM_NAME_STYLES: [&str; 5] =
    ["{c}", "the Kingdom of {c}", "the Crown of {c}", "{c} and its lands", "the Realm of {c}"];

/// Name styles for a realm founded by a PEOPLE rather than by a city — `{p}` is
/// the culture, `{c}` its leading city. France is not "the Kingdom of Paris", and
/// styling every realm after a town was the main reason the names read wrong.
const REALM_NAME_STYLES_PEOPLE: [&str; 5] = [
    "the Realm of the {p}", "{p}", "the {p} Crown", "the Lands of the {p}", "Greater {p}",
];

/// Ruler styles by RANK, for a crown that passes by blood. The old code had one
/// flat four-name list, so a house holding a single town was styled "King" — the
/// title claimed something the realm was not. Indexed by `Realm.rank`.
const REALM_TITLES_DYNASTIC: [[&str; 3]; 4] = [
    ["Lord", "Prince", "Ruler"],                    // 0 city-state
    ["King", "Rex", "Lugal"],                       // 1 kingdom
    ["Great King", "Basileus", "Shah"],             // 2 great power
    ["Emperor", "Khagan", "Chakravartin"],          // 3 hegemon
];

/// Ruler styles for a CIVIC crown — an office, not a bloodline. A republic is
/// never styled "King" at any rank, which is the whole reason `Realm.government`
/// exists. The top two ranks reuse the third row: a republic that grew into a
/// great power still elects a magistrate, it does not crown an emperor.
const REALM_TITLES_CIVIC: [[&str; 3]; 4] = [
    ["Doge", "Archon", "Podesta"],
    ["First Citizen", "Gonfalonier", "Consul"],
    ["Grand Consul", "Serene Prince", "Protector"],
    ["Grand Consul", "Serene Prince", "Protector"],
];

/// The style of a realm's ruler, from its rank and its government. Deterministic
/// in the realm's own id, so a realm's title is stable across saves and does not
/// re-roll every time it is re-styled.
pub(crate) fn realm_title_for(rank: u8, government: u8, salt: u64) -> String {
    let table = if government == REALM_GOV_CIVIC { &REALM_TITLES_CIVIC } else { &REALM_TITLES_DYNASTIC };
    let row = &table[(rank as usize).min(table.len() - 1)];
    row[(salt.wrapping_mul(2654435761).rotate_left(7) as usize) % row.len()].to_string()
}

impl CampaignSim {
    /// How far into the state-formation era this year is, 0..1 — see
    /// `REALM_RAMP_YEARS`. Multiplies every proclamation chance so the political
    /// layer FADES IN after the floor instead of switching on in a single year.
    fn realm_epoch_ramp(&self, yr: u32) -> f32 {
        if yr <= REALM_YEAR_FLOOR { return 0.0; }
        (((yr - REALM_YEAR_FLOOR) as f32) / REALM_RAMP_YEARS).clamp(0.0, 1.0)
    }

    /// Yearly · the whole of §3.1. Iterates CITIES (sovereignty is claimed by a seat,
    /// not chosen by a house in the abstract) rather than houses, because the trigger
    /// is fundamentally about a captured government, not about wealth alone.
    pub(crate) fn maybe_proclaim_realms(&mut self, yr: u32) {
        if yr < REALM_YEAR_FLOOR { return; }
        if self.suppress_realms { return; } // see the field — the inheritance gate only
        if self.prov_holder.is_empty() { return; } // rule 25 — no province layer, no sovereignty
        // CULTURE FIRST. A people that can unify does so before its individual
        // cities break away — run last, a single city proclaiming anywhere in a
        // bloc permanently foreclosed that people's nationhood, which measured as
        // Path C firing exactly ZERO times. It is also the wrong history in the
        // other direction: nations DO form, so the model must at least allow the
        // larger event to happen before the smaller ones fragment the ground it
        // needs. (Italy's communes foreclosing Italian unity is the other real
        // case, and it still happens here whenever the roll simply fails.)
        self.maybe_proclaim_culture_realms(yr);
        let tick = self.tick;
        let n = self.hubs.len();
        // ADAPTIVE founding cost (see `realm_founding_cost`) — scales to THIS world so it
        // is always "a great sum only a top house can pay".
        let cost = self.realm_founding_cost();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            if self.hubs[h].realm >= 0 { continue; } // already sovereign or a member
            if self.hubs[h].tribute_to >= 0 { continue; } // a tributary makes no claims of its own
            // Control only becomes a REALM through a province's LARGEST city (its seat,
            // `province_seat_hub` → `prov_holder`). This is the maintainer's model: the
            // province is controlled iff its largest settlement is. Shared by every
            // candidate below, so check it once.
            if !self.prov_holder.contains(&(h as i32)) { continue; }
            // ...and at least one of them must still be free (see the helper).
            if !self.has_free_province_at(h) { continue; }
            // Who CONTROLS this settlement — three candidates, tried best-first:
            //   1. `captor_house` — seized the government outright.
            //   2. `council_house` — holds the council.
            //   3. the DOMINANT TRADE DYNASTY here (`dominant_house_at`) — the strongest
            //      merchant presence, which may OUTRANK a weaker formal holder. This is the
            //      maintainer's rule ("a trade dynasty, if powerful enough, controls the
            //      settlement, then the province") and it is what `econ_measure_realm_
            //      formation` showed was missing: the houses formally holding the big seat
            //      cities were too weak to found a crown, while the powerful trade dynasties
            //      that actually dominated those cities' commerce held no office there and so
            //      never qualified. A candidate that has since been crowned elsewhere or
            //      isn't a tier 1-2 merchant able to afford the cost is simply skipped, so a
            //      weak formal holder no longer BLOCKS a strong trade dynasty at the same seat.
            let candidates = [
                self.hubs[h].captor_house,
                self.hubs[h].council_house,
                self.dominant_house_at(h),
            ];
            for &c in candidates.iter() {
                if c < 0 { continue; }
                let hi = c as usize;
                if hi >= self.houses.len() { continue; }
                if !self.houses[hi].is_merchant() || self.houses[hi].is_guild { continue; }
                if self.houses[hi].tier == 0 || self.houses[hi].tier > REALM_PROCLAIM_TIER_MAX { continue; }
                if self.houses[hi].wealth < cost { continue; } // must afford the (adaptive) founding spend
                let bold = self.head_axis(hi, 0) as f32;
                let expansive = self.head_axis(hi, 3) as f32;
                let chance = (REALM_PROCLAIM_CHANCE * (1.0 + 0.15 * (bold + expansive))
                    * self.realm_epoch_ramp(yr)).max(0.0);
                // The roll folds in `hi`, so each candidate gets its own independent chance
                // rather than all three sharing one seed.
                let salt = ((h as u64) << 20 ^ (yr as u64) << 4).wrapping_add(hi as u64);
                if hash01(self.seed, tick as u64 ^ 0xC0_10A6, salt) > chance { continue; }
                self.promote_house_to_realm(hi, h, yr);
                break; // one realm per seat per year
            }
        }
        // ── SECOND ELIGIBILITY PATH · trade dominance (`PROV_TRADE_CONTROL_FRAC`) ──
        // A house commanding a fifth of a whole PROVINCE's trade may crown itself over
        // that province even with no office at, and even without the province being
        // administered from within — the Venice/Genoa case, and exactly the funnel
        // collapse `econ_measure_realm_formation` measured (24 tier-1-2 dynasties, only
        // 3 hold a seat writ). Runs AFTER the seat-office loop so an office holder gets
        // first refusal at any seat. Founds at the province's OWN largest city (so a
        // province administered from OUTSIDE — the "writ of X" case — still works, which
        // the seat-office loop structurally could not reach), with two rules relaxed for
        // this path: the global tier gate is WAIVED (regional trade dominance is itself
        // the qualification, and such a house is often globally minor), and the cost is
        // scaled to the founding house's OWN fortune (`realm_founding_cost_for_house`)
        // so it can pay for the small city-state its trade entitles it to.
        self.maybe_proclaim_trade_realms(yr);
        self.maybe_proclaim_city_realms(yr);
    }

    /// PATH B · a powerful CITY proclaims for itself, with no merchant house
    /// required (`docs/WORLD_REALISM_REVIEW.md` §3.3).
    ///
    /// The state this reads was built for exactly this and had no readers:
    /// `assign_city_tiers` computes `hub.tier`/`hub.standing` from population,
    /// trade wealth, treasury, territory administered and the ruling house's own
    /// standing, and CLAUDE.md records it as "query-side only — nothing downstream
    /// reads `hub.tier`". This is that reader.
    ///
    /// Tier 1 already carries its own ABSOLUTE standing floor, which is what lets
    /// this path be an emergent condition rather than a calendar date: a young
    /// world simply has no tier-1 city, so no city-path realm can form, without
    /// anyone having to pick a year.
    ///
    /// GOVERNMENT is decided here and only here. A city whose government is
    /// dominated by one house crowns that house (Rome's own path, and the
    /// Medici's). A city with no dominant house proclaims as a REPUBLIC — an
    /// office rather than a bloodline, with no `family` and no succession by
    /// birth. That is the branch that lets Venice and Castile coexist in one
    /// model instead of forcing every polity through a dynasty.
    fn maybe_proclaim_city_realms(&mut self, yr: u32) {
        let tick = self.tick;
        // The bar scales to the world: a multiple of the MEDIAN live city
        // treasury, so it means "rich among its peers" on a poor world and a rich
        // one alike, rather than an absolute figure that is either trivial or
        // unreachable (the same lesson `realm_founding_cost` already records).
        let mut treasuries: Vec<f32> = (0..self.hubs.len())
            .filter(|&h| !self.hubs[h].is_estate && !self.hubs[h].abandoned)
            .map(|h| self.hubs[h].treasury.max(0.0))
            .collect();
        if treasuries.is_empty() { return; }
        treasuries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // LOWER median (`(n-1)/2`), not `n/2`. With an even, small number of cities
        // the upper median is the richer one, so on a two-city world the wealthiest
        // city was measured against ITSELF and could never clear its own bar — the
        // richest city in the world being structurally unable to raise a crown is
        // exactly the funnel collapse `realm_founding_cost` already had to fix once.
        let median = treasuries[(treasuries.len() - 1) / 2];
        let bar = median * REALM_CITY_PATH_TREASURY_MULT;

        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            if self.hubs[h].realm >= 0 || self.hubs[h].tribute_to >= 0 { continue; }
            if self.hubs[h].tier == 0 || self.hubs[h].tier > REALM_CITY_PATH_TIER_MAX { continue; }
            // It must actually administer its own province — a city that does not
            // hold a writ has nothing to raise a crown over.
            if !self.prov_holder.contains(&(h as i32)) { continue; }
            if !self.has_free_province_at(h) { continue; }
            if self.hubs[h].treasury < bar { continue; }

            let salt = ((h as u64) << 20) ^ ((yr as u64) << 4);
            let chance = REALM_CITY_PATH_CHANCE * self.realm_epoch_ramp(yr);
            if hash01(self.seed, tick as u64 ^ 0xC17_9_1A1E, salt) > chance { continue; }

            // Whoever holds the government here, if anyone does strongly enough.
            let dominant = if self.hubs[h].captor_house >= 0 { self.hubs[h].captor_house }
                else { self.hubs[h].council_house };
            let dominant = usize::try_from(dominant).ok()
                .filter(|&hi| hi < self.houses.len())
                .filter(|&hi| self.houses[hi].is_merchant() && !self.houses[hi].is_guild);
            let civic = match dominant {
                None => true, // nobody holds it — it can only be an office
                Some(_) => hash01(self.seed, tick as u64 ^ 0x0C1B_1C00_u64, salt) < REALM_CIVIC_CHANCE,
            };

            match (civic, dominant) {
                (false, Some(hi)) => {
                    let cost = self.realm_founding_cost_for_house(hi, self.realm_founding_cost());
                    if self.houses[hi].wealth < cost { continue; }
                    self.promote_house_to_realm_with_cost(hi, h, yr, cost, REALM_PATH_CITY);
                }
                _ => { self.found_civic_realm(h, yr, REALM_PATH_CITY); }
            }
        }
    }

    /// PATH C · CULTURAL DOMINATION — a contiguous single-culture bloc unifies
    /// under its largest city (`docs/WORLD_REALISM_REVIEW.md` §3.3).
    ///
    /// The substrate was already present and unused: `prov_culture` (one culture
    /// per province) plus `prov_neighbors` (adjacency) make a connected
    /// same-culture component a free computation. This is the ethnogenesis path —
    /// Franks, Poles, Rus' — and the only one of the three whose borders a player
    /// can read straight off the culture map, because the frontier IS where a
    /// people ends.
    ///
    /// It also produces the tightest realms by construction (`REALM_COHESION_
    /// TARGET[REALM_PATH_CULTURE]`), which is the point: a state made of one
    /// people governs its own ground better than a trade network does.
    fn maybe_proclaim_culture_realms(&mut self, yr: u32) {
        let tick = self.tick;
        let np = self.prov_count();
        if np == 0 || self.prov_neighbors.is_empty() { return; }

        let mut seen = vec![false; np];
        for start in 0..np {
            if seen[start] { continue; }
            let Some(culture) = self.prov_culture.get(start).cloned() else { continue };
            if culture.is_empty() { seen[start] = true; continue; }

            // Flood the CONTIGUOUS same-culture component containing `start`.
            let mut bloc: Vec<usize> = Vec::new();
            let mut queue = std::collections::VecDeque::new();
            seen[start] = true;
            queue.push_back(start);
            while let Some(p) = queue.pop_front() {
                bloc.push(p);
                for &q in self.prov_neighbors.get(p).map(|v| v.as_slice()).unwrap_or(&[]) {
                    let q = q as usize;
                    if q >= np || seen[q] { continue; }
                    if self.prov_culture.get(q).map(|c| c == &culture).unwrap_or(false) {
                        seen[q] = true;
                        queue.push_back(q);
                    }
                }
            }
            if bloc.len() < REALM_CULTURE_MIN_PROVINCES { continue; }
            // A people unifies out of whatever of itself is still FREE. Requiring
            // the whole bloc to be unclaimed is what made this path fire exactly
            // zero times on a measurable world: from year 50 the merchant and city
            // paths take provinces one at a time, and a single sovereign province
            // anywhere in a bloc killed that people's chance permanently.
            //
            // It is also the wrong history. Unification characteristically happens
            // AGAINST existing statelets, not in a vacuum — Piedmont unified Italy
            // and Prussia unified Germany out of lands already full of principalities.
            // So the bloc must still be MOSTLY free (a people already largely ruled
            // by others cannot spontaneously unify — that would be conquest, which
            // this path is not), and only its free provinces join.
            let free: Vec<usize> = bloc.iter().copied()
                .filter(|&p| self.prov_realm.get(p).copied().unwrap_or(-1) < 0)
                .collect();
            if free.len() < REALM_CULTURE_MIN_PROVINCES { continue; }
            if (free.len() as f32) < bloc.len() as f32 * REALM_CULTURE_MIN_FREE_FRAC { continue; }
            let bloc = free;

            // The bloc's largest live city becomes the capital.
            let capital = bloc.iter()
                .filter_map(|&p| self.province_seat_hub(p))
                .filter(|&h| !self.hubs[h].is_estate && !self.hubs[h].abandoned
                    && self.hubs[h].realm < 0 && self.hubs[h].tribute_to < 0)
                .max_by(|&a, &b| self.hubs[a].population.partial_cmp(&self.hubs[b].population)
                    .unwrap_or(std::cmp::Ordering::Equal));
            let Some(capital) = capital else { continue };

            let salt = ((start as u64) << 20) ^ ((yr as u64) << 4);
            let chance = REALM_CULTURE_PATH_CHANCE * self.realm_epoch_ramp(yr);
            if hash01(self.seed, tick as u64 ^ 0x0C17_09E5, salt) > chance { continue; }

            // A people unifying under its own great city crowns the house that
            // holds that city if one does; otherwise the city itself does it.
            let dominant = if self.hubs[capital].captor_house >= 0 { self.hubs[capital].captor_house }
                else { self.hubs[capital].council_house };
            let dominant = usize::try_from(dominant).ok()
                .filter(|&hi| hi < self.houses.len())
                .filter(|&hi| self.houses[hi].is_merchant() && !self.houses[hi].is_guild);

            // The whole bloc's writ moves to the new capital, so the coronation's
            // own territory sweep folds every province of the people into the realm.
            for &p in &bloc {
                if p < self.prov_holder.len() { self.prov_holder[p] = capital as i32; }
            }
            match dominant {
                Some(hi) => {
                    let cost = self.realm_founding_cost_for_house(hi, self.realm_founding_cost());
                    if self.houses[hi].wealth >= cost {
                        self.promote_house_to_realm_with_cost(hi, capital, yr, cost, REALM_PATH_CULTURE);
                        continue;
                    }
                    self.found_civic_realm(capital, yr, REALM_PATH_CULTURE);
                }
                None => { self.found_civic_realm(capital, yr, REALM_PATH_CULTURE); }
            }
        }
    }

    /// Found a realm with NO founding house — a city or a people crowning an
    /// OFFICE rather than a family.
    ///
    /// Deliberately NOT a variant of `promote_house_to_realm`: that function's
    /// whole job is the house→crown TRANSFER (wealth, debts, estates-to-crown-lease,
    /// the `crowned` flag, the merchant ladder exit), and none of it applies when
    /// there is no house. Forcing the two through one path would have meant a
    /// pile of `if house.is_some()` branches through the most consequential
    /// function in the file.
    ///
    /// The crown's opening treasury is a SHARE of the founding city's own, not an
    /// invention: a republic is funded by the city that raised it.
    pub(crate) fn found_civic_realm(&mut self, seat: usize, yr: u32, path: u8) -> u32 {
        let tick = self.tick;
        let id = self.realms.len() as u32;

        let mut provinces: Vec<u32> = Vec::new();
        for p in 0..self.prov_holder.len() {
            if self.prov_holder.get(p).copied().unwrap_or(-1) != seat as i32 { continue; }
            // Rule 24: a province a HOUSE holds as dues is that house's territory,
            // not the city's, and a civic founding does not seize it.
            if self.prov_holder_house.get(p).copied().unwrap_or(-1) >= 0 { continue; }
            // And never a province another crown already holds (see the same
            // guard in `promote_house_to_realm_with_cost`).
            if self.prov_realm.get(p).copied().unwrap_or(-1) >= 0 { continue; }
            provinces.push(p as u32);
            if p < self.prov_realm.len() { self.prov_realm[p] = id as i32; }
        }

        let (name, _) = self.generate_realm_name_for(seat, tick as u64 ^ 0x0C1B_1C55, path);
        let title = realm_title_for(REALM_CITY_STATE, REALM_GOV_CIVIC, id as u64);
        let endowment = (self.hubs[seat].treasury.max(0.0) * REALM_PROCLAIM_COST_FRAC).max(0.0);
        self.hubs[seat].treasury -= endowment;

        let city_name = self.hubs[seat].name.clone();
        let text = format!("{} proclaims {} — a commonwealth, answering to no house", city_name, name);
        let realm = Realm {
            id, name: name.clone(), title, capital_hub: seat as u32,
            origin_realm: -1,
            // No dynasty. `ruling_house` is a u32 with no niche for "none", so a
            // civic realm points at `u32::MAX` and every reader that resolves it
            // through `self.houses.get(..)` already yields None — no new guard
            // needed at any existing call site.
            ruling_house: u32::MAX,
            rank: REALM_CITY_STATE, autonomy: AUTONOMY_CORE_PERIPHERY,
            provinces: provinces.clone(), vassals: Vec::new(),
            treasury: endowment, debts: 0.0,
            legitimacy: REALM_FOUNDING_LEGITIMACY,
            cohesion: REALM_COHESION_TARGET[(path as usize).min(2)],
            founded_tick: tick, fallen_tick: 0,
            events: vec![RealmEvent { tick, kind: "founded".into(), text: text.clone() }],
            // A republic has no family, so `realm_family_pass` finds an empty
            // roster and leaves it alone — there is no ruler to age or bury.
            ruler: -1, regent: -1, family: Vec::new(),
            tax_rates: [0.0; 2], tithe_last_year: 0.0, tax_farm: None,
            founding_path: path, government: REALM_GOV_CIVIC,
        };
        self.realms.push(realm);

        self.hubs[seat].realm = id as i32;
        self.hubs[seat].realm_role = REALM_ROLE_SEAT;
        for &p in &provinces {
            if let Some(ph) = self.prov_holder.get(p as usize).copied().filter(|&h| h >= 0) {
                let ph = ph as usize;
                if ph != seat && ph < self.hubs.len() && self.hubs[ph].realm < 0 {
                    self.hubs[ph].realm = id as i32;
                    self.hubs[ph].realm_role = REALM_ROLE_SUBJECT;
                }
            }
        }
        self.journal.push(JournalEntry {
            tick, kind: "realm_founded".into(), hub: seat as i32, good: -1, value: 0.0, text,
        });
        let _ = yr;
        id
    }

    /// The trade-dominance proclamation pass (see the tail of `maybe_proclaim_realms`).
    /// `world_cost` is the ordinary adaptive founding cost, used only as the CEILING on
    /// the house-scaled cost this path charges.
    fn maybe_proclaim_trade_realms(&mut self, yr: u32) {
        for p in 0..self.prov_count() {
            if self.prov_realm.get(p).copied().unwrap_or(-1) >= 0 { continue; } // already sovereign
            // The province's own largest LIVE city, if it has one. A FRONTIER province
            // whose only settlements have died/been abandoned (the very "dead cities"
            // the player is seeing) has none — `province_seat_hub` returns None — and the
            // realm then seats at the dominant house's OWN home city instead (below), so a
            // dead-cored province no longer silently blocks its trade master's crown.
            let province_seat = self.province_seat_hub(p);
            let mut shares = self.province_trade_shares(p);
            // Strongest trade presence first, so the province's dominant house is tried
            // before any lesser one that also happens to clear the threshold.
            shares.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (hi, share) in shares {
                if share < PROV_TRADE_CONTROL_FRAC { break; } // sorted → no later one clears it either
                if hi >= self.houses.len() { continue; }
                if !self.houses[hi].is_merchant() || self.houses[hi].is_guild { continue; }
                // Seat the crown at the province's own city, else at the dominant house's
                // home city (a house crowns from its base and annexes the province whose
                // trade it runs). Either must be a live, non-estate settlement.
                let home = self.houses[hi].hub as usize;
                let seat = province_seat.or_else(|| {
                    (home < self.hubs.len() && !self.hubs[home].is_estate && !self.hubs[home].abandoned)
                        .then_some(home)
                });
                let Some(seat) = seat else { continue };
                // The ONLY hard block: a seat already inside another realm cannot found
                // a second one (rule 27 — taking sovereign ground needs a war, not a
                // coronation). A tributary seat is NOT blocked — proclaiming sovereignty
                // is exactly how a tributary throws off its overlord, so we clear the
                // tribute below rather than let it veto the crown.
                if self.hubs[seat].realm >= 0 { continue; }
                // PURE TRADE DOMINANCE, no dice. A private house that commands at least
                // `PROV_TRADE_CONTROL_FRAC` of the province's trade AND holds at least
                // `REALM_TRADE_MIN_WEALTH` proclaims a realm — deterministically, the
                // same year it becomes eligible. The founding costs that flat price; the
                // house keeps the rest as the new crown's treasury. (Per the maintainer:
                // "no chances — just pure trade dominance and a price of 50k.")
                if self.houses[hi].wealth < REALM_TRADE_MIN_WEALTH { continue; }
                let cost = REALM_TRADE_MIN_WEALTH;
                // The province breaks away under its dominant merchant: its writ moves to
                // the crown's seat, which then administers it directly, and the seat sheds
                // any tributary bond as it claims sovereignty.
                self.hubs[seat].tribute_to = -1;
                if p < self.prov_holder.len() { self.prov_holder[p] = seat as i32; }
                self.promote_house_to_realm_with_cost(hi, seat, yr, cost, REALM_PATH_MERCHANT);
                break; // one realm per province per year
            }
        }
    }

    /// Does this seat administer at least one province NOT already under a crown?
    ///
    /// A realm is a claim to LAND. Without this a city could proclaim over a
    /// province some earlier realm already held — the coronation's own
    /// already-sovereign guard would then skip every province and leave a LANDLESS
    /// realm, a sovereign entity holding nothing at all. That is not a rare edge
    /// case: measured, it was most of them (45 live realms against 24 provinces).
    fn has_free_province_at(&self, seat: usize) -> bool {
        (0..self.prov_holder.len()).any(|p| {
            self.prov_holder.get(p).copied().unwrap_or(-1) == seat as i32
                && self.prov_realm.get(p).copied().unwrap_or(-1) < 0
        })
    }

    /// The founding cost for a TRADE-DOMINANCE proclamation: a third of the founding
    /// house's OWN fortune (a small city-state it can pay for), never more than the
    /// world-scaled great-power price. **No absolute floor** — deliberately, unlike the
    /// seat-office `realm_founding_cost`: the house that dominates a SINGLE province's
    /// trade is often a POOR local monopolist (especially in a hard, starving world where
    /// trade has collapsed — the rich houses spread their commerce thin and dominate no
    /// one province, so the 20% bar selects exactly the poor local ones). A 1000-flat
    /// floor priced those houses out entirely and NO trade-realm ever formed on such a
    /// world, which is the bug this removes. `cost = min(0.35·wealth, world_cost)` is
    /// affordable for ANY solvent house by construction, so the cost never blocks the
    /// trade path — a poor province simply founds a poor realm. See `PROV_TRADE_CONTROL_FRAC`.
    fn realm_founding_cost_for_house(&self, hi: usize, world_cost: f32) -> f32 {
        let own = REALM_PROCLAIM_COST_FRAC * self.houses[hi].wealth.max(0.0);
        own.min(world_cost.max(0.0))
    }

    /// The house with the strongest merchant presence at hub `h` — the trade dynasty
    /// that effectively controls the city's commerce even without holding a formal
    /// office (the maintainer's "a powerful enough trade dynasty controls the
    /// settlement"). Only a SUBSTANTIAL presence counts: influence ≥ `GOVT_MIN_INFLUENCE`,
    /// the same bar `update_government` uses to let a house court a seat, so an incidental
    /// trader passing through is never read as the city's master. Weighted by wealth so
    /// the RICHEST dominant house wins a tie. Returns −1 if no house dominates here.
    pub(crate) fn dominant_house_at(&self, h: usize) -> i32 {
        let mut best = (-1i32, 0.0f32);
        for hi in 0..self.houses.len() {
            let hh = &self.houses[hi];
            if hh.is_guild || !hh.is_merchant() { continue; }
            let inf = hh.influence.iter().find(|(c, _)| *c == h as u32)
                .map(|(_, v)| *v).unwrap_or(0.0);
            if inf < GOVT_MIN_INFLUENCE { continue; }
            let score = inf * (hh.wealth.max(0.0) + 1.0).sqrt();
            if score > best.1 { best = (hi as i32, score); }
        }
        best.0
    }

    /// Each live house's SHARE (0..1) of all merchant-house trade volume across the
    /// cities of province `p` — the basis for the `PROV_TRADE_CONTROL_FRAC` realm
    /// eligibility path and for the province-trade panel. Volume is `House.trade_at`
    /// (the decaying per-hub tally `bump_trade_at` maintains) summed over the hubs
    /// whose `hub_province == p`; the denominator is every house's total there, so a
    /// share is "of the organized merchant trade", guilds included in the base.
    /// Returns `(house_idx, share)` for houses with a positive presence, unsorted.
    /// Empty when no province layer, no such province, or no trade there yet.
    pub(crate) fn province_trade_shares(&self, p: usize) -> Vec<(usize, f32)> {
        if self.hub_province.is_empty() { return Vec::new(); }
        let mut per_house = vec![0.0f32; self.houses.len()];
        let mut total = 0.0f32;
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct { continue; }
            let mut v = 0.0f32;
            for &(hub, vol) in &hh.trade_at {
                if self.hub_province.get(hub as usize).copied().unwrap_or(-1) == p as i32 {
                    v += vol.max(0.0);
                }
            }
            per_house[hi] = v;
            total += v;
        }
        if total <= EPS { return Vec::new(); }
        per_house.into_iter().enumerate()
            .filter(|(_, v)| *v > 0.0)
            .map(|(hi, v)| (hi, v / total))
            .collect()
    }

    /// The ADAPTIVE realm founding cost for THIS world: `REALM_PROCLAIM_COST_FRAC` of the
    /// wealthiest live merchant house's fortune (floored), so a crown always costs "a
    /// great sum only a top house can pay" regardless of the world's absolute wealth
    /// scale. The richest house always clears its own bar, so a realm CAN always form.
    pub(crate) fn realm_founding_cost(&self) -> f32 {
        // Reference the wealth SCALE of the world's top houses — but ROBUSTLY. The single
        // richest merchant is almost always a pure-trade dynasty that never bothered to
        // capture a government; pinning the bar to its fortune (0.6 × the global max) set a
        // sum no actual CAPTOR — a house that spent heavily bribing its way into a seat, and
        // so sits well below that outlier — could ever reach, and NO realm ever formed on a
        // real world (only the synthetic single-house test cleared it). Use a high
        // PERCENTILE of merchant wealth instead: still "a great sum only a top house can
        // pay" (and the gate already requires the captor be tier 1-2, i.e. exactly that top
        // stratum), but a lone outlier can no longer veto every coronation.
        let mut wealths: Vec<f32> = self.houses.iter()
            .filter(|h| h.is_merchant() && !h.is_guild)
            .map(|h| h.wealth.max(0.0))
            .collect();
        if wealths.is_empty() { return REALM_PROCLAIM_COST_FLOOR; }
        wealths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // 80th-percentile fortune: the scale of the world's top fifth of houses. With one
        // house this is its own wealth (the test's happy path); with two it is still the
        // richer (so "a far richer rival raises the bar" holds); at scale it tracks the top
        // stratum without being pinned to a single anomalous fortune.
        let idx = (((wealths.len() - 1) as f32) * 0.80).round() as usize;
        let reference = wealths[idx.min(wealths.len() - 1)];
        (REALM_PROCLAIM_COST_FRAC * reference).max(REALM_PROCLAIM_COST_FLOOR)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  COHESION, RANK, AND THE TWO NON-MERCHANT FORMATION PATHS.
    //
    //  Before this, three `Realm` fields were dead: `cohesion` was set to 1.0 at
    //  founding and never written again (so `realm_collection_efficiency` — the
    //  plan's headline "a state is limited by what it can COLLECT" — reduced to
    //  distance alone), `rank` was never promoted off `REALM_CITY_STATE` despite
    //  its own doc describing a percentile ladder, and `legitimacy` was written by
    //  two paths and read as a decision input by none.
    //
    //  They are fixed together because they are one mechanism: the path a realm
    //  formed by sets what its cohesion tends toward, cohesion decides what it can
    //  collect, what it collects decides how big it grows, and rank is the reading
    //  of that. Fixing any one alone would have left it inert in a different way.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Yearly · move each realm's cohesion toward the target its founding path and
    /// its current territory imply.
    ///
    /// Three terms, in order of weight:
    ///   1. THE PATH's own target (`REALM_COHESION_TARGET`) — a merchant crown
    ///      assembled out of trade interests never grips like a unified people.
    ///   2. CULTURAL DISTANCE: every province whose culture differs from the
    ///      capital's drags the target down, in proportion. This is the brake on
    ///      unlimited expansion and the reason the three paths DIVERGE over time
    ///      instead of converging — conquering foreign ground is exactly how a
    ///      tight realm becomes a loose one.
    ///   3. LEGITIMACY, lightly. A contested or regency-weakened crown governs
    ///      worse, but legitimacy is about the RULER's right to rule and cohesion
    ///      about the realm's grip on its land — related, not the same.
    ///
    /// Drift is slow (`REALM_COHESION_DRIFT`) because a realm's grip is a
    /// generational property, not something one bad year swings.
    pub(crate) fn update_realm_cohesion(&mut self) {
        if self.realms.is_empty() { return; }
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            let path = (self.realms[ri].founding_path as usize).min(REALM_COHESION_TARGET.len() - 1);
            let mut target = REALM_COHESION_TARGET[path];

            // Cultural distance, measured against the CAPITAL's own province.
            let capital = self.realms[ri].capital_hub as usize;
            let home_culture = self.hub_province.get(capital).copied()
                .filter(|&p| p >= 0)
                .and_then(|p| self.prov_culture.get(p as usize).cloned());
            if let Some(home) = home_culture {
                let provs = &self.realms[ri].provinces;
                if !provs.is_empty() {
                    let foreign = provs.iter()
                        .filter(|&&p| {
                            self.prov_culture.get(p as usize).map(|c| c != &home).unwrap_or(false)
                        })
                        .count();
                    let frac = foreign as f32 / provs.len() as f32;
                    target -= REALM_COHESION_FOREIGN_PENALTY * frac;
                }
            }

            let legit = self.realms[ri].legitimacy.clamp(0.0, 1.0);
            target += REALM_LEGITIMACY_TO_COHESION * (legit - REALM_FOUNDING_LEGITIMACY);
            let target = target.clamp(0.05, 1.0);

            let cur = self.realms[ri].cohesion;
            self.realms[ri].cohesion = (cur + (target - cur) * REALM_COHESION_DRIFT).clamp(0.05, 1.0);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  CONSOLIDATION. Tilly's ~500 European polities c.1500 fall to ~25 by 1900,
    //  and the model had only the first half of that curve — realms formed and
    //  fragmented, nothing ever merged, so a world reached 1500 and stayed there.
    //
    //  Three passes, in the order they run: GROW into free land, ABSORB a weaker
    //  neighbour, and LOSE ground that cannot be held. The third is not a
    //  concession — a model where realms only ever grow converges on one colour
    //  just as surely as one where they only fragment (the realm plan's own §5.6:
    //  "the gate that matters is not do realms form, it is do realms END").
    //
    //  All three are CONTIGUITY-DRIVEN, over `prov_neighbors`. That is what makes
    //  a realm read as a country rather than as a scatter of provinces: territory
    //  is only ever gained next to territory already held.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Province-graph hop distance between two provinces, capped so a
    /// disconnected pair is merely "far" rather than infinite. Used to seed a
    /// partition's shares far apart from one another.
    pub(crate) fn province_hops(&self, from: u32, to: u32) -> u32 {
        if from == to { return 0; }
        const CAP: u32 = 24;
        let np = self.prov_neighbors.len();
        if np == 0 { return CAP; }
        let mut seen = vec![false; np];
        let mut q = std::collections::VecDeque::new();
        if (from as usize) < np { seen[from as usize] = true; q.push_back((from, 0u32)); }
        while let Some((p, d)) = q.pop_front() {
            if d >= CAP { break; }
            for &nb in self.prov_neighbors.get(p as usize).map(|v| v.as_slice()).unwrap_or(&[]) {
                if nb == to { return d + 1; }
                if (nb as usize) < np && !seen[nb as usize] {
                    seen[nb as usize] = true;
                    q.push_back((nb, d + 1));
                }
            }
        }
        CAP
    }

    /// The provinces this realm holds, as a set for adjacency tests.
    fn realm_province_set(&self, ri: usize) -> std::collections::BTreeSet<usize> {
        (0..self.prov_realm.len())
            .filter(|&p| self.prov_realm[p] == ri as i32)
            .collect()
    }

    /// Attach province `p` to realm `ri`, with its seat city. The ONE place
    /// territory is added, so expansion, integration and war can never drift on
    /// what "gaining a province" means.
    fn attach_province(&mut self, ri: usize, p: usize) {
        if p >= self.prov_realm.len() { return; }
        self.prov_realm[p] = ri as i32;
        if !self.realms[ri].provinces.contains(&(p as u32)) {
            self.realms[ri].provinces.push(p as u32);
        }
        if let Some(seat) = self.province_seat_hub(p) {
            if self.hubs[seat].realm < 0 {
                self.hubs[seat].realm = ri as i32;
                self.hubs[seat].realm_role = REALM_ROLE_SUBJECT;
            }
        }
    }

    /// Yearly · a realm annexes ONE adjacent province that no crown holds.
    ///
    /// Contiguous by construction: candidates come from `prov_neighbors` of land
    /// already held, so a realm grows outward as a blob rather than acquiring
    /// scattered enclaves. Gated on cohesion and treasury — a crown that cannot
    /// govern what it has does not reach for more, which is also what stops one
    /// realm running away with the whole map.
    pub(crate) fn realm_expansion_pass(&mut self, yr: u32) {
        if self.realms.is_empty() || self.prov_neighbors.is_empty() { return; }
        let tick = self.tick;
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            let cohesion = self.realms[ri].cohesion;
            if cohesion < REALM_EXPAND_MIN_COHESION { continue; }
            let held = self.realm_province_set(ri);
            if held.is_empty() { continue; }
            let need = held.len() as f32 * REALM_EXPAND_TREASURY_PER_PROV;
            if self.realms[ri].treasury < need { continue; }

            // Rank widens the reach: a great power annexes where a city-state
            // cannot. Cohesion scales it because an ungovernable realm has no
            // surplus attention for new ground.
            let rank_boost = 1.0 + 0.25 * self.realms[ri].rank as f32;
            let chance = REALM_EXPAND_CHANCE * cohesion * rank_boost;
            let salt = ((ri as u64) << 20) ^ ((yr as u64) << 4);
            if hash01(self.seed, tick as u64 ^ 0x6209_0077, salt) > chance { continue; }

            // The best free neighbour: prefer land of the realm's OWN culture,
            // then the richest. Culture first is not flavour — a realm that grows
            // along its own people keeps the cohesion to keep growing, and one
            // that swallows foreigners loses it (see `update_realm_cohesion`), so
            // this is the choice that decides whether it can expand again.
            let capital = self.realms[ri].capital_hub as usize;
            let home = self.hub_province.get(capital).copied()
                .filter(|&p| p >= 0)
                .and_then(|p| self.prov_culture.get(p as usize).cloned());
            let mut best: Option<(u8, u32, usize)> = None; // (rank, -cap, p)
            for &p in &held {
                for &q in self.prov_neighbors.get(p).map(|v| v.as_slice()).unwrap_or(&[]) {
                    let q = q as usize;
                    if q >= self.prov_realm.len() || self.prov_realm[q] >= 0 { continue; }
                    let same = home.as_ref()
                        .map(|h| self.prov_culture.get(q).map(|c| c == h).unwrap_or(false))
                        .unwrap_or(false);
                    let cap = self.prov_cap.get(q).copied().unwrap_or(0.0).max(0.0) as u32;
                    let cand = (if same { 0u8 } else { 1u8 }, u32::MAX - cap, q);
                    if best.map(|b| cand < b).unwrap_or(true) { best = Some(cand); }
                }
            }
            let Some((_, _, p)) = best else { continue };
            self.attach_province(ri, p);
            self.realms[ri].treasury -= need * 0.25; // the cost of taking it in hand
            let realm_name = self.realms[ri].name.clone();
            let people = self.prov_culture.get(p).cloned().unwrap_or_default();
            let text = if people.is_empty() {
                format!("{} extends its writ over a neighbouring province", realm_name)
            } else {
                format!("{} extends its writ over {} lands", realm_name, people)
            };
            self.realms[ri].events.push(RealmEvent { tick, kind: "annexed".into(), text });
        }
    }

    /// Yearly · a strong realm makes a weaker ADJACENT one its vassal, and after
    /// a long term may integrate it outright.
    ///
    /// This is the mechanism that actually bends Tilly's curve back down, and it
    /// is deliberately the slowest of the three: vassalage first, integration only
    /// after `REALM_VASSAL_INTEGRATE_YEARS`, because swallowing a neighbouring
    /// crown whole was rare and took generations when it happened at all.
    ///
    /// `Realm.vassals` has existed since R1 and had no writer until now.
    pub(crate) fn realm_vassalage_pass(&mut self, yr: u32) {
        if self.realms.len() < 2 || self.prov_neighbors.is_empty() { return; }
        let tick = self.tick;
        let strength = |me: &Self, ri: usize| -> f32 {
            me.realms[ri].provinces.len() as f32 * (1.0 + 0.4 * me.realms[ri].rank as f32)
        };
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            // ── 1. INTEGRATE a vassal already held long enough ──
            let vassals = self.realms[ri].vassals.clone();
            for &v in &vassals {
                let vi = v as usize;
                if vi >= self.realms.len() || self.realms[vi].fallen_tick > 0 { continue; }
                let held_years = tick.saturating_sub(self.realms[vi].founded_tick) / TICKS_PER_YEAR;
                if held_years < REALM_VASSAL_INTEGRATE_YEARS { continue; }
                let salt = ((ri as u64) << 24) ^ ((vi as u64) << 8) ^ yr as u64;
                if hash01(self.seed, tick as u64 ^ 0x1076_6247, salt) > REALM_INTEGRATE_CHANCE { continue; }
                self.integrate_vassal(ri, vi);
            }
            // ── 2. IMPOSE vassalage on a weaker adjacent realm ──
            let mine = strength(self, ri);
            if mine <= 0.0 { continue; }
            let held = self.realm_province_set(ri);
            if held.is_empty() { continue; }
            let mut target: Option<usize> = None;
            for &p in &held {
                for &q in self.prov_neighbors.get(p).map(|v| v.as_slice()).unwrap_or(&[]) {
                    let r = self.prov_realm.get(q as usize).copied().unwrap_or(-1);
                    if r < 0 || r == ri as i32 { continue; }
                    let other = r as usize;
                    if self.realms[other].fallen_tick > 0 { continue; }
                    // Never a realm that already answers to someone.
                    if self.realms.iter().any(|x| x.vassals.contains(&(other as u32))) { continue; }
                    if self.realms[other].vassals.contains(&(ri as u32)) { continue; }
                    if mine < strength(self, other) * REALM_VASSAL_STRENGTH_RATIO { continue; }
                    target = Some(other);
                    break;
                }
                if target.is_some() { break; }
            }
            let Some(vi) = target else { continue };
            let salt = ((ri as u64) << 24) ^ ((vi as u64) << 8) ^ ((yr as u64) << 1);
            if hash01(self.seed, tick as u64 ^ 0x7A55_A100, salt) > REALM_VASSAL_CHANCE { continue; }
            self.realms[ri].vassals.push(vi as u32);
            // A vassal's own cities read as tributary, not conquered: it keeps its
            // crown, its dynasty and its land — only its independence is gone.
            for h in 0..self.hubs.len() {
                if self.hubs[h].realm == vi as i32 && self.hubs[h].realm_role != REALM_ROLE_SEAT {
                    self.hubs[h].realm_role = REALM_ROLE_TRIBUTARY;
                }
            }
            let (over, under) = (self.realms[ri].name.clone(), self.realms[vi].name.clone());
            let text = format!("{} submits to {} — a crown kept, an independence lost", under, over);
            self.realms[ri].events.push(RealmEvent { tick, kind: "vassalized".into(), text: text.clone() });
            self.realms[vi].events.push(RealmEvent { tick, kind: "submitted".into(), text: text.clone() });
            self.journal.push(JournalEntry {
                tick, kind: "realm_vassal".into(), hub: self.realms[vi].capital_hub as i32,
                good: -1, value: 0.0, text,
            });
        }
    }

    /// A vassal is absorbed outright: its land, its treasury and its cities pass
    /// to the overlord and its own crown ends. Routed through `dissolve_realm` so
    /// there is still exactly ONE place a realm's life ends, then the territory is
    /// re-attached — rather than a second, separately-drifting teardown.
    fn integrate_vassal(&mut self, ri: usize, vi: usize) {
        let tick = self.tick;
        let provs: Vec<usize> = self.realm_province_set(vi).into_iter().collect();
        let treasury = self.realms[vi].treasury.max(0.0);
        let (over, under) = (self.realms[ri].name.clone(), self.realms[vi].name.clone());
        let text = format!("{} is absorbed into {} — its crown ends and its lands pass whole", under, over);
        self.dissolve_realm(vi, "integrated", text.clone());
        for p in provs { self.attach_province(ri, p); }
        self.realms[ri].treasury += treasury;
        self.realms[ri].vassals.retain(|&v| v != vi as u32);
        // Absorbing a foreign crown is a shock to the grip, not a free win: the
        // new subjects are somebody else's people, and `update_realm_cohesion`
        // will keep charging for them every year they are held.
        self.realms[ri].cohesion = (self.realms[ri].cohesion * 0.85).max(0.05);
        self.realms[ri].events.push(RealmEvent { tick, kind: "integrated".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "realm_integrated".into(), hub: self.realms[ri].capital_hub as i32,
            good: -1, value: 0.0, text,
        });
    }

    /// Yearly · a province the crown can no longer hold BREAKS AWAY.
    ///
    /// The counterweight to expansion. Without it a realm can only ever grow, and
    /// a world of only-growing realms converges on one colour exactly as surely as
    /// one that only fragments — which is what the realm plan's §5.6 means by "the
    /// gate that matters is not do realms form, it is do realms END".
    ///
    /// Conditions are cumulative and all three must hold: the crown's cohesion has
    /// collapsed, the province is culturally FOREIGN to the capital, and it is not
    /// the capital's own. A realm losing its last province falls entirely.
    pub(crate) fn realm_secession_pass(&mut self, yr: u32) {
        if self.realms.is_empty() { return; }
        let tick = self.tick;
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            if self.realms[ri].cohesion > REALM_SECEDE_MAX_COHESION { continue; }
            let capital = self.realms[ri].capital_hub as usize;
            let capital_prov = self.hub_province.get(capital).copied().unwrap_or(-1);
            let home = (capital_prov >= 0)
                .then(|| self.prov_culture.get(capital_prov as usize).cloned())
                .flatten();
            let Some(home) = home else { continue };

            let mut lost: Vec<usize> = Vec::new();
            for p in self.realm_province_set(ri) {
                if capital_prov >= 0 && p == capital_prov as usize { continue; }
                let foreign = self.prov_culture.get(p).map(|c| c != &home).unwrap_or(false);
                if !foreign { continue; }
                let salt = ((ri as u64) << 24) ^ ((p as u64) << 8) ^ yr as u64;
                let chance = REALM_SECEDE_CHANCE * (1.0 - self.realms[ri].cohesion);
                if hash01(self.seed, tick as u64 ^ 0x5ECE_5510, salt) > chance { continue; }
                lost.push(p);
            }
            for p in &lost {
                self.prov_realm[*p] = -1;
                self.realms[ri].provinces.retain(|&q| q as usize != *p);
                if let Some(seat) = self.province_seat_hub(*p) {
                    if self.hubs[seat].realm == ri as i32 {
                        self.hubs[seat].realm = -1;
                        self.hubs[seat].realm_role = 0;
                    }
                }
            }
            if lost.is_empty() { continue; }
            let realm_name = self.realms[ri].name.clone();
            let people = self.prov_culture.get(lost[0]).cloned().unwrap_or_default();
            let text = if people.is_empty() {
                format!("{} loses a province to revolt", realm_name)
            } else {
                format!("The {} throw off the writ of {}", people, realm_name)
            };
            self.realms[ri].events.push(RealmEvent { tick, kind: "seceded".into(), text: text.clone() });
            self.journal.push(JournalEntry {
                tick, kind: "realm_seceded".into(), hub: capital as i32, good: -1, value: 0.0, text,
            });
            // A crown with nothing left to rule is no longer a crown.
            if self.realms[ri].provinces.is_empty() {
                let n = self.realms[ri].name.clone();
                self.dissolve_realm(ri, "collapsed", format!("{} collapses — no province still answers it", n));
            }
        }
    }

    /// Yearly · the realm RANK ladder — a direct mirror of `assign_city_tiers`
    /// (percentile among LIVE realms + an absolute floor on the top rank +
    /// hysteresis), which is the shape `Realm.rank`'s own doc-comment already
    /// described and which had simply never been written.
    ///
    /// Percentile rather than absolute thresholds for the same reason house and
    /// city tiers use it: "great power" has to mean *relative to the other states
    /// of this world*, not a fixed province count that means nothing as a world
    /// grows. The top rank carries an additional absolute floor so a young world,
    /// where every realm is a single city and its hinterland, correctly has no
    /// hegemon at all.
    pub(crate) fn assign_realm_ranks(&mut self) {
        let tick = self.tick;
        let live: Vec<usize> = (0..self.realms.len())
            .filter(|&ri| self.realms[ri].fallen_tick == 0)
            .collect();
        let n = live.len();
        if n == 0 { return; }

        // Population under each realm's writ, precomputed once.
        let mut pop_of = vec![0.0f32; self.realms.len()];
        for h in 0..self.hubs.len() {
            let r = self.hubs[h].realm;
            if r >= 0 && (r as usize) < pop_of.len() && !self.hubs[h].abandoned {
                pop_of[r as usize] += self.hubs[h].population.max(0.0);
            }
        }

        let provs: Vec<f32> = live.iter().map(|&ri| self.realms[ri].provinces.len() as f32).collect();
        let pops: Vec<f32> = live.iter().map(|&ri| pop_of[ri]).collect();
        let treas: Vec<f32> = live.iter().map(|&ri| self.realms[ri].treasury.max(0.0)).collect();
        let pr = rank_norm(&provs);
        let pp = rank_norm(&pops);
        let tr = rank_norm(&treas);

        // Four axes, the fourth being COHESION itself — a sprawling realm that
        // cannot collect from its own land is not a great power, and this is the
        // one place the two repaired fields meet.
        let mut standings = vec![0.0f32; n];
        for k in 0..n {
            let coh = self.realms[live[k]].cohesion.clamp(0.0, 1.0);
            standings[k] = (0.35 * pr[k] + 0.25 * pp[k] + 0.20 * tr[k] + 0.20 * coh).clamp(0.0, 1.0);
        }

        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| standings[b].partial_cmp(&standings[a]).unwrap_or(std::cmp::Ordering::Equal));
        let mut pct = vec![0.0f32; n];
        for (rank, &k) in order.iter().enumerate() {
            pct[k] = if n > 1 { rank as f32 / (n - 1) as f32 } else { 0.0 };
        }

        for k in 0..n {
            let ri = live[k];
            let prev = self.realms[ri].rank;
            let new_rank = Self::realm_rank_with_hysteresis(prev, pct[k], standings[k]);
            if new_rank > prev {
                // A RISE is chronicled; a fall is not — the same asymmetry house
                // and city tiers already use.
                let name = self.realms[ri].name.clone();
                let text = format!("{} is now reckoned a {}", name, REALM_RANK_NAMES[new_rank as usize]);
                self.realms[ri].events.push(RealmEvent { tick, kind: "rank".into(), text: text.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "realm_rank".into(), hub: self.realms[ri].capital_hub as i32,
                    good: -1, value: new_rank as f32, text,
                });
                // The ruler's STYLE follows the rank: a lord of one city is not a
                // king, and a king of twenty provinces is not a lord. Re-styled on
                // a rise only, so a realm never loses its title in a bad decade.
                let gov = self.realms[ri].government;
                self.realms[ri].title = realm_title_for(new_rank, gov, self.realms[ri].id as u64);
            }
            self.realms[ri].rank = new_rank;
        }
    }

    /// The rank a realm holds this year given the one it held last year. `rank` 3
    /// (hegemon) carries an absolute standing floor on top of the percentile cut,
    /// with its own hysteresis, exactly as tier 1 does for houses and cities.
    fn realm_rank_with_hysteresis(prev: u8, pct: f32, standing: f32) -> u8 {
        let mut cuts = REALM_RANK_PCT_CUTS;
        // Widen the band the realm is currently IN, in both directions.
        if (1..=3).contains(&prev) { cuts[(prev - 1) as usize] += REALM_RANK_PCT_DEAD_BAND; }
        if prev <= 2 { cuts[prev as usize] -= REALM_RANK_PCT_DEAD_BAND; }
        let floor = if prev == 3 { REALM_RANK_TOP_STANDING_EXIT } else { REALM_RANK_TOP_STANDING_ENTER };
        if pct < cuts[0] && standing >= floor { 3 }
        else if pct < cuts[1] { 2 }
        else if pct < cuts[2] { 1 }
        else { 0 }
    }

    /// Placeholder naming (see the module doc) — a culture-vocabulary namer that
    /// reads `prov_culture` at the capital is real follow-up work, not built here.
    /// Shared by every realm-CREATING path (a coronation, R5's partible division)
    /// so a cadet realm is named by the same rule its parent was.
    fn generate_realm_name(&self, capital: usize, salt: u64) -> (String, String) {
        self.generate_realm_name_for(capital, salt, REALM_PATH_MERCHANT)
    }

    /// As `generate_realm_name`, but a realm founded by CULTURAL DOMINATION is
    /// named for its PEOPLE, not for the city that happened to lead them.
    ///
    /// This is the difference between "the Kingdom of Vashira" (a city that grew)
    /// and "the Aioran Realm" (a people that unified), and it is the whole reason
    /// the title read oddly before: every realm, however founded, was styled after
    /// a town. France is not "the Kingdom of Paris".
    fn generate_realm_name_for(&self, capital: usize, salt: u64, path: u8) -> (String, String) {
        let city_name = self.hubs[capital].name.clone();
        let name_seed = (capital as u64).wrapping_mul(2654435761) ^ salt;
        let people = self.hub_province.get(capital).copied()
            .filter(|&p| p >= 0)
            .and_then(|p| self.prov_culture.get(p as usize).cloned())
            .filter(|c| !c.is_empty());
        let name = match (path, people) {
            (REALM_PATH_CULTURE, Some(people)) => {
                let style = REALM_NAME_STYLES_PEOPLE[(name_seed as usize) % REALM_NAME_STYLES_PEOPLE.len()];
                style.replace("{p}", &people).replace("{c}", &city_name)
            }
            _ => {
                let style = REALM_NAME_STYLES[(name_seed as usize) % REALM_NAME_STYLES.len()];
                style.replace("{c}", &city_name)
            }
        };
        // A newly proclaimed realm is always rank 0 — `assign_realm_ranks` promotes
        // it (and re-styles it) once it has earned the standing.
        let title = realm_title_for(REALM_CITY_STATE, REALM_GOV_DYNASTIC, name_seed);
        (name, title)
    }

    /// The house is ELEVATED — its wealth and trade assets become the crown's and it
    /// leaves the merchant world (`REALM_AND_GOVERNMENT_PLAN.md` §3.2). Returns the
    /// new realm's id. Charges the ordinary world-scaled founding cost; the trade-
    /// dominance path calls `promote_house_to_realm_with_cost` with a house-scaled one.
    pub(crate) fn promote_house_to_realm(&mut self, hi: usize, seat: usize, yr: u32) -> u32 {
        let cost = self.realm_founding_cost();
        self.promote_house_to_realm_with_cost(hi, seat, yr, cost, REALM_PATH_MERCHANT)
    }

    /// As `promote_house_to_realm`, but the founding `cost` (spent from the house's
    /// wealth into the new crown's treasury) is supplied by the caller — the seat-office
    /// path passes the world-scaled `realm_founding_cost`, the trade-dominance path a
    /// house-scaled `realm_founding_cost_for_house`.
    pub(crate) fn promote_house_to_realm_with_cost(
        &mut self, hi: usize, seat: usize, yr: u32, cost: f32, path: u8,
    ) -> u32 {
        let tick = self.tick;

        // Debts inherited whole — a crown can default (plan §3.2/§5.1).
        let mut debts = 0.0f32;
        for b in self.banks.iter_mut() {
            if b.defunct { continue; }
            for l in b.loans.iter_mut() {
                if l.borrower_house == hi as i32 && l.outstanding > 0.0 {
                    debts += l.outstanding;
                    l.borrower_house = -2; // −2: "owed by a realm now" — never a live house index again
                }
            }
        }

        // Territory: the seat's own administered provinces, plus any this house held
        // as dues (`prov_holder_house`, the Stato da Mar case) — those fold into
        // direct sovereignty rather than staying a separate payee relationship, since
        // the payee and the new sovereign are now the same crown.
        let mut provinces: Vec<u32> = Vec::new();
        let id = self.realms.len() as u32;
        for p in 0..self.prov_holder.len() {
            let admin_here = self.prov_holder.get(p).copied().unwrap_or(-1) == seat as i32;
            let house_held = self.prov_holder_house.get(p).copied().unwrap_or(-1) == hi as i32;
            if !admin_here && !house_held { continue; }
            // ALREADY SOVEREIGN: a province belonging to another crown is not
            // available to a new one. Administration (`prov_holder`) and
            // sovereignty (`prov_realm`) are independent layers (rule 27), so a
            // city can perfectly well administer a province that another realm
            // owns — and without this check the new realm listed it too, which is
            // how `provinces under a crown` measured 36 of 24. Taking it needs a
            // war, not a coronation.
            if self.prov_realm.get(p).copied().unwrap_or(-1) >= 0 { continue; }
            provinces.push(p as u32);
            if house_held { self.prov_holder_house[p] = -1; } // the crown administers directly now
            if p < self.prov_realm.len() { self.prov_realm[p] = id as i32; }
        }

        let (name, title) = self.generate_realm_name_for(seat, tick as u64, path);

        // The house SPENDS the founding cost the caller set — a court, a retinue, a
        // crown's apparatus — deducted from the wealth that becomes the new crown's
        // treasury, so proclaiming is a real outlay, not a free relabelling. Floored at 0
        // (the gate already required `wealth >= cost`, so this only guards a rounding edge).
        let treasury = (self.houses[hi].wealth - cost).max(0.0);
        let house_name = self.houses[hi].name.clone();
        let head_name = self.houses[hi].head_name.clone();

        // R2 · the founding generation — a real Person, not a snapshot. Character
        // and skill are read from the house's own kin[0], which "always mirrors the
        // current head" (CLAUDE.md §5); a house with no roster (an old save) falls
        // back to a deterministic hash rather than an invented zero, matching the
        // "no roster ⇒ nothing is known, not a bland default" discipline Phase 2.5
        // already established for stewards.
        let (character, skill) = match self.houses[hi].kin.first() {
            Some(k) => (k.character, k.skill),
            None => {
                let s = (hi as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ tick as u64;
                let axis = |n: u64| ((hash01(self.seed, s, n) * 5.0) as i8) - 2;
                ([axis(0), axis(1), axis(2), axis(3)], hash01(self.seed, s, 9))
            }
        };
        let head_age_years = self.houses[hi].head_age.max(18);
        let founder = Person {
            name: head_name.clone(),
            female: self.houses[hi].head_female,
            born_tick: tick.saturating_sub(head_age_years * TICKS_PER_YEAR),
            died_tick: 0, father: -1, mother: -1, spouse: -1,
            character, skill,
            epithet: String::new(),
            reign_start: tick, reign_end: 0,
        };

        let realm = Realm {
            id, name: name.clone(), title: title.clone(), capital_hub: seat as u32,
            origin_realm: -1, // an original proclamation, not a partible offshoot
            ruling_house: hi as u32, rank: REALM_CITY_STATE, autonomy: AUTONOMY_CORE_PERIPHERY,
            provinces: provinces.clone(), vassals: Vec::new(),
            treasury, debts, legitimacy: REALM_FOUNDING_LEGITIMACY,
            // Cohesion starts AT the path's own target rather than at a universal
            // 1.0 — a merchant crown never had the grip a unified people does, and
            // starting every realm at perfect cohesion is what made the field inert.
            cohesion: REALM_COHESION_TARGET[(path as usize).min(2)],
            founded_tick: tick, fallen_tick: 0,
            events: vec![RealmEvent {
                tick, kind: "founded".into(),
                text: format!("{} of House {} proclaims {}, {} of {}",
                    title, house_name, name, head_name, self.hubs[seat].name),
            }],
            ruler: 0, regent: -1, family: vec![founder],
            tax_rates: [0.0; 2], tithe_last_year: 0.0, tax_farm: None,
            founding_path: path, government: REALM_GOV_DYNASTIC,
        };
        self.realms.push(realm);

        // The house is ELEVATED, not dissolved — see the module + §5.1 for why this
        // must never be `defunct`. The pot moves whole; nothing about its identity
        // (name, arms, `line`, `origin_house` lineage) is touched.
        self.houses[hi].wealth = 0.0;
        self.houses[hi].crowned = true;
        self.houses[hi].realm = id as i32;
        self.houses[hi].political_power = 0.0;
        self.houses[hi].tier = 0; // leaves the 1-4 merchant ladder's "assigned" range

        self.hubs[seat].realm = id as i32;
        self.hubs[seat].realm_role = REALM_ROLE_SEAT;
        for &p in &provinces {
            // A province's own seat city (if different from the capital and it has
            // one) is a member of the new realm too, even before any conquest.
            if let Some(ph) = self.prov_holder.get(p as usize).copied().filter(|&h| h >= 0) {
                let ph = ph as usize;
                if ph != seat && ph < self.hubs.len() && self.hubs[ph].realm < 0 {
                    self.hubs[ph].realm = id as i32;
                    self.hubs[ph].realm_role = REALM_ROLE_SUBJECT;
                }
            }
        }

        // The first realm in a WORLD is unmissable; every later one is still a real
        // event but the journal need not shout every time (§3.1 — only the FIRST is
        // a world event by explicit design; the plan is silent on later ones, so this
        // keeps every proclamation chronicled without over-claiming novelty language
        // for the second, tenth, hundredth).
        let is_first = self.realms.len() == 1;
        let text = if is_first {
            format!("{} proclaims {} — the first crown this world has known", house_name, name)
        } else {
            format!("{} proclaims {}", house_name, name)
        };
        self.houses[hi].events.push(HouseEvent { tick, kind: "crowned".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "realm_founded".into(), hub: seat as i32, good: -1, value: 0.0, text,
        });

        // ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.10 (D12) · every estate this
        // house directly owned converts to CROWN TITLE: any pre-existing
        // minority share (a bank stake, an earlier envoy partial buy) is
        // grandfathered as a time-limited operating LEASE rather than swept
        // aside (`instrument` → 1, a `LEASE_TERM_YEARS` term); whatever
        // fraction wasn't already claimed goes to the crown itself as a new
        // Share row — reusing the fully-wired holder_kind=4 (realm) path both
        // `production.rs`'s dividend cut and `offtake_delivery_pass` already
        // credit to `Realm.treasury`. For an extraction estate this crown
        // share is payout=0 (offtake), which — since it delivers PHYSICAL
        // goods rather than cash — already satisfies A7's "royalty in kind"
        // by construction, no separate mechanism needed. `owner_house` is
        // cleared to −1 so the old private-owner cut path (which only checks
        // `!defunct`, not `is_merchant()` — rule 25's own warning) never
        // credits the now-crowned house again.
        for ei in 0..self.hubs.len() {
            if !self.hubs[ei].is_estate || self.hubs[ei].owner_house != hi as i32 { continue; }
            let payout: u8 = if self.hubs[ei].estate_kind == 6 { 1 } else { 0 };
            for sh in self.hubs[ei].shares.iter_mut() {
                if sh.instrument == 0 { sh.instrument = 1; sh.term_years = LEASE_TERM_YEARS; }
            }
            let held: f32 = self.hubs[ei].shares.iter().map(|s| s.frac.max(0.0)).sum();
            let crown_frac = (1.0 - held).max(0.0);
            if crown_frac > EPS {
                self.hubs[ei].shares.push(Share {
                    holder_kind: 4, holder: id, frac: crown_frac, payout,
                    acquired_tick: tick, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
                });
            }
            self.hubs[ei].owner_house = -1;
        }
        // D13 (revocation/war/intrigue lease loss) is deliberately NOT built
        // here — it's a real, separate decision system (a stated, chronicled
        // reason; war reusing `apply_war_goal`/`strip_holdings_at`; intrigue
        // reusing `foreign_hand.rs`/the crisis engine), out of scope for this
        // pass, flagged rather than silently skipped.

        let _ = yr;
        id
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  R2 · GENEALOGY (`REALM_AND_GOVERNMENT_PLAN.md` §3.7). A real family, not a
    //  regenerated roster: `Realm.family` is seeded once at the coronation
    //  (`promote_house_to_realm`, above) and only ever GROWS — every index a
    //  `father`/`mother`/`spouse` field points at stays valid for the realm's whole
    //  life, the same discipline `House.kin`/`House.line` already keep.
    //
    //  Deliberately kept to the RULER'S OWN direct line (spouse + children), not a
    //  full extended tree of cousins and in-laws — that keeps the family bounded
    //  and matches the actual job (produce a real heir, a real regency, a real
    //  succession), while every non-inheriting sibling still PERSISTS in `family`
    //  and remains eligible as a fallback heir if the direct line runs out (see
    //  `resolve_realm_succession`). Cross-realm marriage, bastards/pretenders, and
    //  a per-figure power ledger are all explicitly deferred (plan §6).
    // ═══════════════════════════════════════════════════════════════════════════

    /// Yearly · mortality, succession, marriage, births — in that order, so a
    /// ruler who dies THIS year is succeeded before anyone tries to act as them.
    pub(crate) fn realm_family_pass(&mut self, yr: u32) {
        if self.realms.is_empty() { return; }
        let tick = self.tick;
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            if self.realms[ri].family.is_empty() { continue; } // defensive; coronation always seeds one

            // 1) Mortality over every LIVING member — ruler included. A death here
            //    only marks `died_tick`; consequences (succession) are resolved next.
            let n = self.realms[ri].family.len();
            for pi in 0..n {
                if self.realms[ri].family[pi].died_tick > 0 { continue; }
                let born = self.realms[ri].family[pi].born_tick;
                let age = tick.saturating_sub(born) / TICKS_PER_YEAR;
                let hazard = person_mortality_hazard(age);
                let roll = hash01(self.seed, (ri as u64) << 24 | pi as u64, tick as u64 ^ 0xDEAD_10CC);
                if roll < hazard { self.realms[ri].family[pi].died_tick = tick; }
            }

            // 2) Succession, if the ruler is among today's dead.
            let ruler = self.realms[ri].ruler;
            if ruler >= 0 {
                let ru = ruler as usize;
                if ru < self.realms[ri].family.len() && self.realms[ri].family[ru].died_tick == tick {
                    self.resolve_realm_succession(ri, yr);
                    if self.realms[ri].fallen_tick > 0 { continue; } // dynasty ended this year
                }
            }

            // 3) Marriage, then 4) births — the (possibly new) ruler's own line only.
            self.maybe_marry_ruler(ri);
            self.maybe_birth_heir(ri);
        }
    }

    /// The ruler has just died. Picks the next ruler — the eldest ELIGIBLE living
    /// child, sex-filtered by the capital's own `LineRule` (rule 23); widens to any
    /// living family member if the direct line has no survivor; and if NO one
    /// anywhere in the family qualifies, the dynasty ends — sovereignty is released
    /// cleanly (never left pointing at a fallen realm) rather than the realm being
    /// left in an undefined state (the same "must always terminate" discipline
    /// rule 22 already holds a house crisis to).
    /// The dynasty is gone and nothing can carry the realm forward — release
    /// every province's sovereignty AND every member city's `hub.realm`/
    /// `realm_role` (never leave state pointing at a fallen realm — rule 25),
    /// mark `fallen_tick`, and chronicle it. The ONE place a realm's life ends,
    /// reused by every path that can end one (no heir at succession, a capital
    /// abandoned with nowhere left to relocate to). Keeping this in one place is
    /// what stopped R4's capital-city release bug from becoming two separately-
    /// drifting copies of the same fix.
    fn dissolve_realm(&mut self, ri: usize, kind: &str, text: String) {
        let tick = self.tick;
        for p in self.realms[ri].provinces.clone() {
            if (p as usize) < self.prov_realm.len() { self.prov_realm[p as usize] = -1; }
        }
        for h in 0..self.hubs.len() {
            if self.hubs[h].realm == ri as i32 {
                self.hubs[h].realm = -1;
                self.hubs[h].realm_role = 0;
            }
        }
        self.realms[ri].fallen_tick = tick;
        self.realms[ri].events.push(RealmEvent { tick, kind: kind.into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "realm_fallen".into(), hub: self.realms[ri].capital_hub as i32,
            good: -1, value: 0.0, text,
        });
    }

    /// R5 · relocate a realm's capital to another of its own member cities —
    /// reassigns `realm_role` (the old capital becomes a plain SUBJECT, the new
    /// one SEAT), chronicles the move. The Karakorum rule cuts both ways: a
    /// capital need not be the largest city, but when IT is lost, something has
    /// to become the seat. Callers are responsible for `new_capital` actually
    /// being a member (`hub.realm == ri`) — this does not re-check it, matching
    /// every other realm-internal helper here.
    pub(crate) fn move_realm_capital(&mut self, ri: usize, new_capital: usize) {
        let tick = self.tick;
        let old_capital = self.realms[ri].capital_hub as usize;
        if old_capital == new_capital { return; }
        self.hubs[old_capital].realm_role = REALM_ROLE_SUBJECT;
        self.hubs[new_capital].realm_role = REALM_ROLE_SEAT;
        self.realms[ri].capital_hub = new_capital as u32;
        let (old_name, new_name) = (self.hubs[old_capital].name.clone(), self.hubs[new_capital].name.clone());
        let realm_name = self.realms[ri].name.clone();
        let text = format!("The capital of {} moves from {} to {}", realm_name, old_name, new_name);
        self.realms[ri].events.push(RealmEvent { tick, kind: "capital_moved".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "capital_moved".into(), hub: new_capital as i32, good: -1, value: 0.0, text,
        });
    }

    /// Yearly · the one capital-move TRIGGER this pass wires: a capital that has
    /// been ABANDONED (the city-lifecycle system, Atlas 2.0 — famine/plague/war
    /// can empty a city) can no longer serve as anyone's seat. A speculative
    /// "chase prosperity" relocation policy is NOT built — this is a defensive
    /// case, not an AI preference, and inventing the latter's trigger conditions
    /// under time pressure would be exactly the kind of untested behaviour
    /// change this project's culture (CLAUDE.md §2.4) warns against. Relocates to
    /// the realm's largest surviving member city; with none left, the realm
    /// follows its capital into extinction via the same `dissolve_realm` every
    /// other ending uses.
    pub(crate) fn maybe_relocate_abandoned_capitals(&mut self, yr: u32) {
        let _ = yr;
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            let capital = self.realms[ri].capital_hub as usize;
            if capital >= self.hubs.len() || !self.hubs[capital].abandoned { continue; }
            let next = (0..self.hubs.len())
                .filter(|&h| h != capital && self.hubs[h].realm == ri as i32
                    && !self.hubs[h].is_estate && !self.hubs[h].abandoned)
                .max_by(|&a, &b| self.hubs[a].population.partial_cmp(&self.hubs[b].population).unwrap());
            match next {
                Some(nc) => self.move_realm_capital(ri, nc),
                None => {
                    let realm_name = self.realms[ri].name.clone();
                    let text = format!("{} falls with its abandoned capital — no city remains to carry it", realm_name);
                    self.dissolve_realm(ri, "dynasty_ended", text);
                }
            }
        }
    }

    pub(crate) fn resolve_realm_succession(&mut self, ri: usize, yr: u32) {
        let _ = yr;
        let tick = self.tick;
        let dead = self.realms[ri].ruler as usize;
        let capital = self.realms[ri].capital_hub as usize;
        let (line, rule) = self.rules_for_hub(capital);

        let reign_start = self.realms[ri].family[dead].reign_start;
        let reign_years = tick.saturating_sub(reign_start) / TICKS_PER_YEAR;
        let age_at_death = tick.saturating_sub(self.realms[ri].family[dead].born_tick) / TICKS_PER_YEAR;
        let r = hash01(self.seed, ri as u64, tick as u64 ^ 0xE917_1E17);
        let epithet = realm_ruler_epithet(reign_years, age_at_death, r);
        self.realms[ri].family[dead].reign_end = tick;
        self.realms[ri].family[dead].epithet = epithet.clone();
        let dead_name = self.realms[ri].family[dead].name.clone();
        let realm_name = self.realms[ri].name.clone();

        let fam = &self.realms[ri].family;
        let son_exists = fam.iter().any(|p| p.died_tick == 0 && !p.female
            && (p.father == dead as i32 || p.mother == dead as i32));
        let eligible = |p: &Person| -> bool {
            match line {
                LineRule::Agnatic => !p.female,
                LineRule::Enatic => p.female,
                LineRule::Absolute => true,
                // "No eligible son" — the same real allowance `heir_is_female`
                // documents for a merchant house, applied here to an ACTUAL
                // roster instead of an invented one.
                LineRule::AgnaticCognatic => !p.female || !son_exists,
            }
        };
        // The dead ruler's own children first, eldest-eligible.
        let mut heir = fam.iter().enumerate()
            .filter(|(_, p)| p.died_tick == 0 && (p.father == dead as i32 || p.mother == dead as i32))
            .filter(|(_, p)| eligible(p))
            .min_by_key(|(_, p)| p.born_tick)
            .map(|(i, _)| i);
        // No surviving child of this ruler — widen to any other living family
        // member (a sibling, an uncle) rather than end the dynasty prematurely.
        if heir.is_none() {
            heir = fam.iter().enumerate()
                .filter(|(i, p)| *i != dead && p.died_tick == 0)
                .filter(|(_, p)| eligible(p))
                .min_by_key(|(_, p)| p.born_tick)
                .map(|(i, _)| i);
        }

        let Some(heir_idx) = heir else {
            let text = format!("The line of {} ends — {} has no heir and dissolves", dead_name, realm_name);
            self.dissolve_realm(ri, "dynasty_ended", text);
            return;
        };

        // R5 · Path A — PARTIBLE division. Only when the dead ruler leaves
        // MULTIPLE eligible living children of their OWN — a co-heir found only
        // through the "widen to any living relative" fallback above is never
        // split further; that fallback already means the direct line failed,
        // and dividing a rescue heir's inheritance would compound a crisis
        // rather than model the law. The number of co-heirs is capped by the
        // SAME distribution `divide_estate` already uses for a merchant house
        // (`partible_heirs` — two to four, weighted toward two), floored by how
        // many eligible children actually survive.
        if matches!(rule, InheritanceRule::Partible) {
            let mut co_heirs: Vec<usize> = self.realms[ri].family.iter().enumerate()
                .filter(|(_, p)| p.died_tick == 0 && (p.father == dead as i32 || p.mother == dead as i32))
                .filter(|(_, p)| eligible(p))
                .map(|(i, _)| i)
                .collect();
            co_heirs.sort_by_key(|&i| self.realms[ri].family[i].born_tick);
            if co_heirs.len() >= 2 {
                let roll = crate::sim::inheritance::partible_heirs(ri as u64 ^ tick as u64, self.seed) as usize;
                co_heirs.truncate(roll.clamp(2, co_heirs.len()));
                self.partition_realm(ri, &co_heirs, &dead_name, &realm_name);
                return;
            }
        }

        let regency_note = self.install_ruler(ri, heir_idx);
        let heir_name = self.realms[ri].family[heir_idx].name.clone();
        let epithet_note = if epithet.is_empty() { String::new() } else { format!(" ({})", epithet) };
        let reign_note = if reign_years > 0 { format!(", after a reign of {} years", reign_years) } else { String::new() };
        let text = format!("{}{} dies{} — {} succeeds as ruler of {}{}",
            dead_name, epithet_note, reign_note, heir_name, realm_name, regency_note);
        self.realms[ri].events.push(RealmEvent { tick, kind: "succession".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "realm_succession".into(), hub: self.realms[ri].capital_hub as i32,
            good: -1, value: 0.0, text,
        });
    }

    /// Installs `heir_idx` as `ri`'s ruler, starting their reign and — if
    /// they're a minor — a regency (their own living mother, else the eldest
    /// other living adult), at the usual legitimacy cost. Shared by ordinary
    /// succession and the eldest co-heir's share of a partible division, so the
    /// two paths can never drift apart on how a regency is decided. Returns the
    /// narration clause to append to whichever event text called it.
    fn install_ruler(&mut self, ri: usize, heir_idx: usize) -> String {
        let tick = self.tick;
        let heir_age = tick.saturating_sub(self.realms[ri].family[heir_idx].born_tick) / TICKS_PER_YEAR;
        self.realms[ri].ruler = heir_idx as i32;
        self.realms[ri].family[heir_idx].reign_start = tick;
        if heir_age >= PERSON_ADULT_AGE {
            self.realms[ri].regent = -1;
            return String::new();
        }
        let mother = self.realms[ri].family[heir_idx].mother;
        let regent = if mother >= 0 && (mother as usize) < self.realms[ri].family.len()
            && self.realms[ri].family[mother as usize].died_tick == 0 {
            Some(mother as usize)
        } else {
            self.realms[ri].family.iter().enumerate()
                .filter(|(i, p)| *i != heir_idx && p.died_tick == 0
                    && tick.saturating_sub(p.born_tick) / TICKS_PER_YEAR >= PERSON_ADULT_AGE)
                .min_by_key(|(_, p)| p.born_tick)
                .map(|(i, _)| i)
        };
        self.realms[ri].regent = regent.map(|i| i as i32).unwrap_or(-1);
        self.realms[ri].legitimacy = (self.realms[ri].legitimacy - REGENCY_LEGITIMACY_HIT).max(0.0);
        match regent {
            Some(rg) => {
                let rn = self.realms[ri].family[rg].name.clone();
                format!(" — too young to rule; {} governs as regent", rn)
            }
            None => " — too young to rule, and no regent can be found".into(),
        }
    }

    /// R5 · the partible division itself. `heirs[0]` (the eldest) keeps the
    /// ORIGINAL realm — its id, name, capital, full family history — shrunk to
    /// its own share of the provinces; every other heir founds a BRAND NEW
    /// realm, with a fresh, minimal family (a branch boundary, exactly
    /// `House.origin_house` records inter-house lineage as a POINTER rather
    /// than a duplicated ancestor list) and its own crowned house (`origin_
    /// kind: ORIGIN_DIVISION`, mirroring the merchant `divide_estate`'s own "a
    /// co-heir inherits capital and no fleet" discipline — nothing is invented
    /// that wasn't actually earned). Provinces divide round-robin by index
    /// (deterministic); the eldest's share always includes the ORIGINAL
    /// capital's own province, so the parent realm's identity survives
    /// coherently rather than landing in an arbitrary round-robin slot.
    /// Treasury and debts divide in the SAME proportion as the land — no money
    /// created or destroyed, the same accounting `divide_estate` already holds
    /// a merchant division to.
    fn partition_realm(&mut self, ri: usize, heirs: &[usize], dead_name: &str, realm_name: &str) {
        let tick = self.tick;
        let n = heirs.len();
        let provinces = self.realms[ri].provinces.clone();
        let total_provs = provinces.len().max(1) as f32;
        let total_treasury = self.realms[ri].treasury;
        let total_debts = self.realms[ri].debts;
        let capital = self.realms[ri].capital_hub as usize;
        let capital_prov = self.hub_province.get(capital).copied().unwrap_or(-1);

        // ── CONTIGUOUS shares, not round-robin by index ──────────────────────
        // Round-robin gave each heir every n-th province by ID, which produces
        // interleaved checkerboard realms — the single worst thing this layer did
        // to the map's readability. Real divisions took coherent blocks: Verdun
        // split the Frankish empire into three north-south strips, the Mongol
        // uluses divided by campaign theatre. Each heir is seeded on a province
        // far from the others and grows a connected share outward from it, so a
        // partition yields n countries rather than n confetti patterns.
        let mut shares: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut unassigned: std::collections::BTreeSet<u32> = provinces.iter().copied().collect();
        let mut frontier: Vec<std::collections::VecDeque<u32>> = vec![Default::default(); n];

        // The eldest is seeded on the ORIGINAL capital's province, so the parent
        // realm's identity survives coherently rather than landing wherever an
        // index happened to fall.
        if capital_prov >= 0 && unassigned.remove(&(capital_prov as u32)) {
            shares[0].push(capital_prov as u32);
            frontier[0].push_back(capital_prov as u32);
        }
        // Every other heir starts on whichever remaining province is FURTHEST
        // (in province-graph hops) from the seeds already chosen.
        for k in 1..n {
            let Some(&seed_p) = unassigned.iter().max_by_key(|&&q| {
                shares.iter().flatten().map(|&s| self.province_hops(s, q)).min().unwrap_or(u32::MAX)
            }) else { break };
            unassigned.remove(&seed_p);
            shares[k].push(seed_p);
            frontier[k].push_back(seed_p);
        }
        // Grow all shares outward together, so they stay roughly even in size.
        let mut progress = true;
        while !unassigned.is_empty() && progress {
            progress = false;
            for k in 0..n {
                let mut took = false;
                while let Some(p) = frontier[k].pop_front() {
                    let mut grabbed = false;
                    for &q in self.prov_neighbors.get(p as usize).map(|v| v.as_slice()).unwrap_or(&[]) {
                        if unassigned.remove(&q) {
                            shares[k].push(q);
                            frontier[k].push_back(q);
                            grabbed = true;
                            break;
                        }
                    }
                    if grabbed { frontier[k].push_front(p); took = true; break; }
                }
                if took { progress = true; }
            }
        }
        // Anything unreachable (a province with no neighbour graph, or an island
        // of the realm's territory) falls to the eldest rather than vanishing.
        for p in unassigned { shares[0].push(p); }

        for (k, &heir_idx) in heirs.iter().enumerate().skip(1) {
            let heir_provs = shares[k].clone();
            let new_capital = heir_provs.iter()
                .filter_map(|&p| self.province_seat_hub(p as usize))
                .max_by(|&a, &b| self.hubs[a].population.partial_cmp(&self.hubs[b].population).unwrap());
            let Some(new_capital) = new_capital else {
                // No city anywhere in this heir's share — nothing to found a
                // realm ON. Their share stays part of the ORIGINAL realm rather
                // than vanishing into an ownerless gap; a rare edge case, not
                // the common path, so folding it back is honest rather than
                // inventing a capital that isn't there.
                shares[0].extend(heir_provs);
                continue;
            };
            let new_id = self.realms.len() as u32;
            let (new_name, new_title) = self.generate_realm_name(new_capital, tick as u64 ^ heir_idx as u64);
            let heir_person = self.realms[ri].family[heir_idx].clone();
            let frac = heir_provs.len() as f32 / total_provs;
            let new_wealth = total_treasury * frac;
            let new_debts = total_debts * frac;
            let old_house = self.realms[ri].ruling_house as i32;

            let new_house = House {
                name: format!("House {}", heir_person.name), hub: new_capital as u32,
                wealth: new_wealth, prestige: 0.0, spec: Vec::new(), monopoly: Vec::new(), rivals: Vec::new(),
                generation: 1, events: Vec::new(), good_profit: Vec::new(), good_volume: Vec::new(),
                mono50: Vec::new(), mono_ever: Vec::new(), dominant_seat: false, prev_wealth: new_wealth,
                worst_loss: 0.0, fleet_sea: 0, fleet_river: 0, fleet_caravan: 0,
                head_name: heir_person.name.clone(), head_since: tick, head_lifespan: 0,
                founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
                archetype: 0, charters: Vec::new(), is_guild: false, offices: Vec::new(),
                trade_at: Vec::new(), debt_since: 0, wealth_history: Vec::new(), office_leases: Vec::new(),
                influence: Vec::new(), bailos: Vec::new(), head_female: heir_person.female, head_age: 0,
                line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: new_wealth, peak_wealth_tick: tick,
                wealth_last_check: new_wealth, golden_age_months: 0, golden_age_chronicled: false,
                dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(),
                crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0,
                origin_house: old_house, origin_kind: ORIGIN_DIVISION, crowned: true, realm: new_id as i32,
            };
            let new_house_idx = self.houses.len() as u32;
            self.houses.push(new_house);

            let new_person = Person {
                name: heir_person.name.clone(), female: heir_person.female,
                born_tick: heir_person.born_tick, died_tick: 0,
                father: -1, mother: -1, spouse: -1, // a branch boundary — ancestry doesn't carry over
                character: heir_person.character, skill: heir_person.skill,
                epithet: String::new(), reign_start: tick, reign_end: 0,
            };
            let text = format!("{} breaks from {} at the death of {} — a co-heir's share, by partible law",
                new_name, realm_name, dead_name);
            let new_realm = Realm {
                id: new_id, name: new_name, title: new_title, capital_hub: new_capital as u32,
                origin_realm: ri as i32, ruling_house: new_house_idx, rank: REALM_CITY_STATE,
                autonomy: AUTONOMY_CORE_PERIPHERY, provinces: heir_provs.clone(), vassals: Vec::new(),
                treasury: new_wealth, debts: new_debts,
                legitimacy: REALM_FOUNDING_LEGITIMACY, cohesion: REALM_FOUNDING_COHESION,
                founded_tick: tick, fallen_tick: 0,
                events: vec![RealmEvent { tick, kind: "partitioned".into(), text: text.clone() }],
                ruler: 0, regent: -1, family: vec![new_person],
                tax_rates: [0.0; 2], tithe_last_year: 0.0, tax_farm: None,
                // A cadet realm inherits its parent's path and government: a
                // partition splits a state, it does not re-found one.
                founding_path: self.realms[ri].founding_path,
                government: self.realms[ri].government,
            };
            self.realms.push(new_realm);
            for &p in &heir_provs {
                if (p as usize) < self.prov_realm.len() { self.prov_realm[p as usize] = new_id as i32; }
                if let Some(seat) = self.province_seat_hub(p as usize) {
                    self.hubs[seat].realm = new_id as i32;
                    self.hubs[seat].realm_role = if seat == new_capital { REALM_ROLE_SEAT } else { REALM_ROLE_SUBJECT };
                }
            }
            self.journal.push(JournalEntry {
                tick, kind: "realm_partitioned".into(), hub: new_capital as i32, good: -1, value: 0.0, text,
            });
        }

        // The eldest heir keeps the original realm, shrunk to its own share
        // (which may have grown back if any sibling's share had no city to
        // found on, above).
        let kept = shares[0].clone();
        let keep_frac = kept.len() as f32 / total_provs;
        self.realms[ri].provinces = kept;
        self.realms[ri].treasury = total_treasury * keep_frac;
        self.realms[ri].debts = total_debts * keep_frac;
        let eldest = heirs[0];
        let regency_note = self.install_ruler(ri, eldest);
        let eldest_name = self.realms[ri].family[eldest].name.clone();
        let text = format!("{} dies — the realm of {} divides among {} heirs by partible law; {} keeps {}{}",
            dead_name, realm_name, n, eldest_name, realm_name, regency_note);
        self.realms[ri].events.push(RealmEvent { tick, kind: "partitioned".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "realm_partitioned".into(), hub: self.realms[ri].capital_hub as i32,
            good: -1, value: 0.0, text,
        });
    }

    /// A ruler with no spouse, past `PERSON_MARRY_AGE`, may marry — a spouse
    /// generated locally (cross-realm marriage, personal unions, is deferred,
    /// plan §6). Named for the RULING HOUSE, not the realm's own (often titled)
    /// name, since `head_name_sexed_for` expects a house-name-shaped surname.
    fn maybe_marry_ruler(&mut self, ri: usize) {
        let tick = self.tick;
        let ruler = self.realms[ri].ruler;
        if ruler < 0 { return; }
        let ruler = ruler as usize;
        if ruler >= self.realms[ri].family.len() { return; }
        let rp = &self.realms[ri].family[ruler];
        if rp.died_tick > 0 || rp.spouse >= 0 { return; }
        let age = tick.saturating_sub(rp.born_tick) / TICKS_PER_YEAR;
        if age < PERSON_MARRY_AGE { return; }
        let salt = tick as u64 ^ (ruler as u64) << 8 ^ (ri as u64) << 40;
        let roll = hash01(self.seed, salt, 0);
        if roll > PERSON_MARRY_CHANCE { return; }

        let female = !self.realms[ri].family[ruler].female;
        let spouse_age = PERSON_MARRY_AGE + (hash01(self.seed, salt, 1) * 18.0) as u32; // 18..36
        let house_name = self.houses.get(self.realms[ri].ruling_house as usize)
            .map(|h| h.name.clone()).unwrap_or_default();
        let name = self.head_name_sexed_for(self.realms[ri].capital_hub as usize, &house_name, salt, female);
        let axis = |n: u64| ((hash01(self.seed, salt, n) * 5.0) as i8) - 2;
        let spouse = Person {
            name: name.clone(), female,
            born_tick: tick.saturating_sub(spouse_age * TICKS_PER_YEAR), died_tick: 0,
            father: -1, mother: -1, spouse: ruler as i32,
            character: [axis(2), axis(3), axis(4), axis(5)], skill: hash01(self.seed, salt, 6),
            epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        let spouse_idx = self.realms[ri].family.len();
        self.realms[ri].family.push(spouse);
        self.realms[ri].family[ruler].spouse = spouse_idx as i32;

        let ruler_name = self.realms[ri].family[ruler].name.clone();
        let realm_name = self.realms[ri].name.clone();
        let text = format!("{} of {} marries {}", ruler_name, realm_name, name);
        self.realms[ri].events.push(RealmEvent { tick, kind: "marriage".into(), text });
    }

    /// The ruler and spouse, if both alive and married, may have a child — fertility
    /// by whichever of the two is female, per `PERSON_FERTILE_MIN`/`_MAX`. Birth SEX
    /// is an unbiased coin flip (biology, not inheritance law) — `LineRule` decides
    /// who INHERITS among however many sons/daughters are born, never who is born.
    fn maybe_birth_heir(&mut self, ri: usize) {
        let tick = self.tick;
        let ruler = self.realms[ri].ruler;
        if ruler < 0 { return; }
        let ruler = ruler as usize;
        if ruler >= self.realms[ri].family.len() { return; }
        let spouse = self.realms[ri].family[ruler].spouse;
        if spouse < 0 { return; }
        let spouse = spouse as usize;
        if self.realms[ri].family[ruler].died_tick > 0 { return; }
        if self.realms[ri].family[spouse].died_tick > 0 { return; }

        let (father, mother) = if self.realms[ri].family[ruler].female { (spouse, ruler) }
            else if self.realms[ri].family[spouse].female { (ruler, spouse) }
            else { return; }; // same-sex pairing: no birth in this minimal model
        let mother_age = tick.saturating_sub(self.realms[ri].family[mother].born_tick) / TICKS_PER_YEAR;
        if !(PERSON_FERTILE_MIN..=PERSON_FERTILE_MAX).contains(&mother_age) { return; }

        let salt = tick as u64 ^ (mother as u64) << 12 ^ (ri as u64) << 44;
        let roll = hash01(self.seed, salt, 0);
        if roll > PERSON_BIRTH_CHANCE { return; }

        let female = hash01(self.seed, salt, 1) < 0.5;
        let house_name = self.houses.get(self.realms[ri].ruling_house as usize)
            .map(|h| h.name.clone()).unwrap_or_default();
        let name = self.head_name_sexed_for(self.realms[ri].capital_hub as usize, &house_name, salt, female);
        let axis = |n: u64| ((hash01(self.seed, salt, n) * 5.0) as i8) - 2;
        let child = Person {
            name: name.clone(), female, born_tick: tick, died_tick: 0,
            father: father as i32, mother: mother as i32, spouse: -1,
            character: [axis(2), axis(3), axis(4), axis(5)], skill: hash01(self.seed, salt, 6),
            epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        self.realms[ri].family.push(child);
        let realm_name = self.realms[ri].name.clone();
        let text = format!("A child, {}, is born to the ruling house of {}", name, realm_name);
        self.realms[ri].events.push(RealmEvent { tick, kind: "birth".into(), text });
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  R3 · TAXATION (`REALM_AND_GOVERNMENT_PLAN.md` §3.3). "Pre-modern states
    //  were not limited by what they charged, but by what they could collect" —
    //  `realm_collection_efficiency` is the whole mechanism; everything else here
    //  is a levy that reads it or a decision about rates.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Wrap-aware straight-line distance between two hubs, in cells — the same
    /// pattern `hubs_within_war_reach` already uses.
    fn hub_distance_cells(&self, a: usize, b: usize) -> f32 {
        let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
        if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
        let dy = self.hubs[a].y - self.hubs[b].y;
        (dx * dx + dy * dy).sqrt()
    }

    /// `efficiency(cohesion, distance, autonomy)` — a per-province `integration`
    /// axis (R4/R5's conquest-vs-founding distinction) still isn't built (a
    /// founding-era province starts fully integrated by construction, since it
    /// was never conquered), so this pass is honestly the terms the plan's §3.3
    /// formula names that already have real state behind them, now including R5's
    /// autonomy multiplier on the distance term (`autonomy_distance_mult` — a
    /// centralized realm feels distance harder, an autonomous one barely does).
    /// Distance is normalised by world width, so it reads the same on any map
    /// size; a founding realm's own capital is at distance 0 (efficiency ≈
    /// cohesion), which is why R3 barely moved anything on its own — the term
    /// only bites once a realm actually spans distance, R4/R5's job.
    pub(crate) fn realm_collection_efficiency(&self, ri: usize, seat: usize) -> f32 {
        let capital = self.realms[ri].capital_hub as usize;
        let dist = self.hub_distance_cells(capital, seat);
        let frac = if self.world_w > 0.0 { dist / self.world_w } else { 0.0 };
        let decay = REALM_DISTANCE_DECAY * autonomy_distance_mult(self.realms[ri].autonomy);
        let distance_term = 1.0 / (1.0 + decay * frac);
        self.realms[ri].cohesion.clamp(0.0, 1.0) * distance_term
    }

    /// Yearly · poll tax + customs share, the crown's own two set-rate levies
    /// (the tithe is efficiency-scaled automatically in `province_land_pass`,
    /// never a realm-set rate — the province tax slider stays the player's verb).
    /// Both drain the TAXED CITY's own treasury rather than inventing a new pool —
    /// a pre-modern hearth/customs levy really was collected by the local
    /// government and forwarded, so this is two further skims off money the city
    /// already holds, exactly as the tithe skims off what the countryside grows.
    pub(crate) fn collect_realm_levies(&mut self) {
        if self.realms.is_empty() { return; }
        for ri in 0..self.realms.len() {
            if self.realms[ri].fallen_tick > 0 { continue; }
            // Includes the capital itself — its own `hub.realm` was set at the
            // coronation (`REALM_ROLE_SEAT`), so no separate case is needed.
            let member_cities: Vec<usize> = (0..self.hubs.len())
                .filter(|&h| self.hubs[h].realm == ri as i32 && !self.hubs[h].is_estate
                    && !self.hubs[h].abandoned)
                .collect();
            let poll_rate = self.realms[ri].tax_rates[TAX_POLL];
            let customs_rate = self.realms[ri].tax_rates[TAX_CUSTOMS];
            let revenue_mult = autonomy_revenue_mult(self.realms[ri].autonomy);
            for h in member_cities {
                let efficiency = self.realm_collection_efficiency(ri, h) * revenue_mult;

                if poll_rate > 0.0 {
                    let base = self.hubs[h].population.max(0.0);
                    let assessed = base * poll_rate * efficiency;
                    let paid = assessed.min(self.hubs[h].treasury.max(0.0));
                    if paid > 0.0 {
                        self.hubs[h].treasury -= paid;
                        self.realms[ri].treasury += paid;
                        self.hubs[h].mood = (self.hubs[h].mood - POLL_TAX_MOOD_COST * poll_rate).max(0.0);
                    }
                }
                if customs_rate > 0.0 {
                    let base = self.hubs[h].trade_wealth.max(0.0);
                    let assessed = base * customs_rate * efficiency;
                    let paid = assessed.min(self.hubs[h].treasury.max(0.0));
                    if paid > 0.0 {
                        self.hubs[h].treasury -= paid;
                        self.realms[ri].treasury += paid;
                    }
                }
            }
        }
    }

    /// Yearly · sets next year's poll/customs rates and decides whether to farm
    /// out the tithe. Cheap AI, not optimisation: a treasury below `REALM_
    /// TREASURY_COMFORT` nudges rates UP (toward `REALM_TAX_MAX`); at or above it,
    /// they ease back toward zero. Unrest is what actually punishes overreach —
    /// via the mood cost each levy already carries and the province unrest a high
    /// `prov_tax` already risks — so this needs no separate brake of its own.
    pub(crate) fn decide_realm_taxes(&mut self, ri: usize, yr: u32) {
        if self.realms[ri].fallen_tick > 0 { return; }
        let treasury = self.realms[ri].treasury;
        let need = ((REALM_TREASURY_COMFORT - treasury) / REALM_TREASURY_COMFORT).clamp(-1.0, 1.0);
        for (k, &max) in REALM_TAX_MAX.iter().enumerate() {
            let cur = self.realms[ri].tax_rates[k];
            let target = if need > 0.0 { max } else { 0.0 };
            self.realms[ri].tax_rates[k] = (cur + (target - cur) * REALM_TAX_DRIFT * need.abs())
                .clamp(0.0, max);
        }
        self.maybe_farm_tithe(ri, yr);
    }

    /// A tax farm is a DISTRESS SALE, not standing policy — only considered when
    /// the crown is genuinely short (`REALM_FARM_TREASURY_FLOOR`) and only ever
    /// one active at a time. Priced off `tithe_last_year`, a real figure the crown
    /// actually just collected, discounted the way `publicani`/*iltizam* both
    /// were: the buyer pays less than the full expected value, because the whole
    /// point of selling is cash NOW.
    fn maybe_farm_tithe(&mut self, ri: usize, yr: u32) {
        let tick = self.tick;
        if self.realms[ri].tax_farm.is_some() {
            // Expire a completed term — collection reverts to the crown.
            let f = self.realms[ri].tax_farm.as_ref().unwrap();
            if tick.saturating_sub(f.started_tick) >= f.years * TICKS_PER_YEAR {
                let house = f.house;
                self.realms[ri].tax_farm = None;
                let realm_name = self.realms[ri].name.clone();
                let hname = self.houses.get(house as usize).map(|h| h.name.clone()).unwrap_or_default();
                let text = format!("{}'s tax farm over {} expires — the crown collects directly again", hname, realm_name);
                self.realms[ri].events.push(RealmEvent { tick, kind: "farm_expired".into(), text });
            }
            return;
        }
        if self.realms[ri].treasury >= REALM_FARM_TREASURY_FLOOR { return; }
        if self.realms[ri].tithe_last_year <= 0.0 { return; } // nothing worth farming
        let capital = self.realms[ri].capital_hub as usize;
        // A wealthy, willing house near the seat — reuses the same MERCHANT
        // eligibility every other realm-facing pass filters on.
        let candidate = self.houses.iter().enumerate()
            .filter(|(_, h)| h.is_merchant() && !h.is_guild && h.wealth > REALM_FARM_TREASURY_FLOOR)
            .max_by(|(_, a), (_, b)| a.wealth.partial_cmp(&b.wealth).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(hi, _)| hi);
        let Some(hi) = candidate else { return };
        let lump_sum = self.realms[ri].tithe_last_year * TAX_FARM_YEARS as f32 * TAX_FARM_DISCOUNT;
        if lump_sum <= 0.0 || self.houses[hi].wealth < lump_sum { return; }
        let roll = hash01(self.seed, (ri as u64) << 8 | hi as u64, tick as u64 ^ 0x07A2_FA12);
        if roll > 0.5 { return; } // not every eligible year sells — a real decision, not automatic
        self.houses[hi].wealth -= lump_sum;
        self.realms[ri].treasury += lump_sum;
        self.realms[ri].tax_farm = Some(TaxFarm { house: hi as u32, started_tick: tick, years: TAX_FARM_YEARS });
        let realm_name = self.realms[ri].name.clone();
        let hname = self.houses[hi].name.clone();
        let text = format!("{} buys the tithe of {} for {} years, {:.0} paid to the crown up front",
            hname, realm_name, TAX_FARM_YEARS, lump_sum);
        self.realms[ri].events.push(RealmEvent { tick, kind: "tax_farmed".into(), text: text.clone() });
        self.journal.push(JournalEntry {
            tick, kind: "tax_farmed".into(), hub: capital as i32, good: -1, value: lump_sum, text,
        });
        let _ = yr;
    }
}

/// Per-year mortality hazard by age, chosen to be a broadly plausible pre-modern
/// curve rather than a fidelity-gated one (no `econ_` oracle scores genealogy) —
/// low through early adulthood, rising after ~60, steep past ~75.
/// `PERSON_CHILD_MORTALITY` covers the under-5 band on its own (the plan's own
/// fragmentation engine, §3.8 path B, depends on it landing near real-world scale).
fn person_mortality_hazard(age_years: u32) -> f32 {
    match age_years {
        0..=4 => PERSON_CHILD_MORTALITY,
        5..=14 => 0.004,
        15..=39 => 0.006,
        40..=59 => 0.015,
        60..=74 => 0.04,
        _ => (0.12 + (age_years as f32 - 75.0) * 0.02).min(0.9),
    }
}

/// A ruler's epithet, mirroring `head_epithet`'s own deterministic two-option-pick
/// shape but keyed on reign length + age at death (a realm has no per-ruler wealth
/// series to score against, the axis a merchant house's own epithet reads).
fn realm_ruler_epithet(reign_years: u32, age_at_death: u32, r: f32) -> String {
    let pick = |a: &'static str, b: &'static str| if r < 0.5 { a } else { b };
    if reign_years < 3 { return pick("the Brief", "the Untimely").into(); }
    if reign_years >= 30 { return pick("the Long-Reigning", "the Steadfast").into(); }
    if age_at_death >= 75 { return pick("the Old", "the Venerable").into(); }
    if age_at_death < 25 { return pick("the Young", "the Untimely").into(); }
    String::new()
}
