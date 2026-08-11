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
const REALM_TITLES: [&str; 4] = ["King", "Sovereign", "Lord Protector", "High Ruler"];

impl CampaignSim {
    /// Yearly · the whole of §3.1. Iterates CITIES (sovereignty is claimed by a seat,
    /// not chosen by a house in the abstract) rather than houses, because the trigger
    /// is fundamentally about a captured government, not about wealth alone.
    pub(crate) fn maybe_proclaim_realms(&mut self, yr: u32) {
        if yr < REALM_YEAR_FLOOR { return; }
        if self.prov_holder.is_empty() { return; } // rule 25 — no province layer, no sovereignty
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
                let chance = (REALM_PROCLAIM_CHANCE * (1.0 + 0.15 * (bold + expansive))).max(0.0);
                // The roll folds in `hi`, so each candidate gets its own independent chance
                // rather than all three sharing one seed.
                let salt = ((h as u64) << 20 ^ (yr as u64) << 4).wrapping_add(hi as u64);
                if hash01(self.seed, tick as u64 ^ 0xC0_10A6, salt) > chance { continue; }
                self.promote_house_to_realm(hi, h, yr);
                break; // one realm per seat per year
            }
        }
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

    /// Placeholder naming (see the module doc) — a culture-vocabulary namer that
    /// reads `prov_culture` at the capital is real follow-up work, not built here.
    /// Shared by every realm-CREATING path (a coronation, R5's partible division)
    /// so a cadet realm is named by the same rule its parent was.
    fn generate_realm_name(&self, capital: usize, salt: u64) -> (String, String) {
        let city_name = self.hubs[capital].name.clone();
        let name_seed = (capital as u64).wrapping_mul(2654435761) ^ salt;
        let style = REALM_NAME_STYLES[(name_seed as usize) % REALM_NAME_STYLES.len()];
        let name = style.replace("{c}", &city_name);
        let title = REALM_TITLES[(name_seed.rotate_left(7) as usize) % REALM_TITLES.len()].to_string();
        (name, title)
    }

    /// The house is ELEVATED — its wealth and trade assets become the crown's and it
    /// leaves the merchant world (`REALM_AND_GOVERNMENT_PLAN.md` §3.2). Returns the
    /// new realm's id.
    pub(crate) fn promote_house_to_realm(&mut self, hi: usize, seat: usize, yr: u32) -> u32 {
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
            provinces.push(p as u32);
            if house_held { self.prov_holder_house[p] = -1; } // the crown administers directly now
            if p < self.prov_realm.len() { self.prov_realm[p] = id as i32; }
        }

        let (name, title) = self.generate_realm_name(seat, tick as u64);

        // The house SPENDS the (adaptive) founding cost — a court, a retinue, a crown's
        // apparatus — deducted from the wealth that becomes the new crown's treasury, so
        // proclaiming is a real outlay, not a free relabelling. Floored at 0 (the gate
        // already required `wealth >= cost`, so this only guards a rounding edge).
        let cost = self.realm_founding_cost();
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
            treasury, debts, legitimacy: REALM_FOUNDING_LEGITIMACY, cohesion: REALM_FOUNDING_COHESION,
            founded_tick: tick, fallen_tick: 0,
            events: vec![RealmEvent {
                tick, kind: "founded".into(),
                text: format!("{} of House {} proclaims {}, {} of {}",
                    title, house_name, name, head_name, self.hubs[seat].name),
            }],
            ruler: 0, regent: -1, family: vec![founder],
            tax_rates: [0.0; 2], tithe_last_year: 0.0, tax_farm: None,
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

        let mut shares: Vec<Vec<u32>> = vec![Vec::new(); n];
        if capital_prov >= 0 { shares[0].push(capital_prov as u32); }
        let rest: Vec<u32> = provinces.iter().copied()
            .filter(|&p| capital_prov < 0 || p != capital_prov as u32).collect();
        for (i, &p) in rest.iter().enumerate() { shares[i % n].push(p); }

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
