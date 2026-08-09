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
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            if self.hubs[h].realm >= 0 { continue; } // already sovereign or a member
            if self.hubs[h].tribute_to >= 0 { continue; } // a tributary makes no claims of its own
            let hi = self.hubs[h].captor_house;
            if hi < 0 { continue; }
            let hi = hi as usize;
            if hi >= self.houses.len() { continue; }
            if !self.houses[hi].is_merchant() { continue; } // dead, or already crowned elsewhere
            if self.houses[hi].is_guild { continue; } // a civic office does not found a dynasty
            if tick.saturating_sub(self.hubs[h].captor_since) < REALM_CAPTOR_YEARS * TICKS_PER_YEAR { continue; }
            if self.houses[hi].tier == 0 || self.houses[hi].tier > REALM_PROCLAIM_TIER_MAX { continue; }
            if self.hubs[h].tier == 0 || self.hubs[h].tier > REALM_PROCLAIM_TIER_MAX { continue; }
            if !self.prov_holder.iter().any(|&ph| ph == h as i32) { continue; }
            if self.houses[hi].wealth < REALM_PROCLAIM_TREASURY_MIN { continue; }
            if self.houses[hi].prestige < REALM_PROCLAIM_PRESTIGE_MIN { continue; }
            let bold = self.head_axis(hi, 0) as f32;
            let expansive = self.head_axis(hi, 3) as f32;
            let chance = (REALM_PROCLAIM_CHANCE * (1.0 + 0.15 * (bold + expansive))).max(0.0);
            if hash01(self.seed, tick as u64 ^ 0xC0_10A6, (h as u64) << 16 ^ yr as u64) > chance { continue; }
            self.promote_house_to_realm(hi, h, yr);
        }
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

        let city_name = self.hubs[seat].name.clone();
        // Placeholder naming (see the module doc) — a culture-vocabulary namer that
        // reads `prov_culture` at the capital is real follow-up work, not built here.
        let name_seed = (seat as u64).wrapping_mul(2654435761) ^ tick as u64;
        let style = REALM_NAME_STYLES[(name_seed as usize) % REALM_NAME_STYLES.len()];
        let name = style.replace("{c}", &city_name);
        let title = REALM_TITLES[(name_seed.rotate_left(7) as usize) % REALM_TITLES.len()].to_string();

        let treasury = self.houses[hi].wealth.max(0.0);
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
            ruling_house: hi as u32, rank: REALM_CITY_STATE, autonomy: AUTONOMY_CORE_PERIPHERY,
            provinces: provinces.clone(), cities: vec![seat as u32], vassals: Vec::new(),
            treasury, debts, legitimacy: REALM_FOUNDING_LEGITIMACY, cohesion: REALM_FOUNDING_COHESION,
            founded_tick: tick, fallen_tick: 0,
            events: vec![RealmEvent {
                tick, kind: "founded".into(),
                text: format!("{} of House {} proclaims {}, {} of {}",
                    title, house_name, name, head_name, city_name),
            }],
            ruler: 0, regent: -1, family: vec![founder],
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
    pub(crate) fn resolve_realm_succession(&mut self, ri: usize, yr: u32) {
        let _ = yr;
        let tick = self.tick;
        let dead = self.realms[ri].ruler as usize;
        let capital = self.realms[ri].capital_hub as usize;
        let (line, _rule) = self.rules_for_hub(capital);

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
            for p in self.realms[ri].provinces.clone() {
                if (p as usize) < self.prov_realm.len() { self.prov_realm[p as usize] = -1; }
            }
            self.realms[ri].fallen_tick = tick;
            let text = format!("The line of {} ends — {} has no heir and dissolves", dead_name, realm_name);
            self.realms[ri].events.push(RealmEvent { tick, kind: "dynasty_ended".into(), text: text.clone() });
            self.journal.push(JournalEntry {
                tick, kind: "realm_fallen".into(), hub: self.realms[ri].capital_hub as i32,
                good: -1, value: 0.0, text,
            });
            return;
        };

        let heir_age = tick.saturating_sub(self.realms[ri].family[heir_idx].born_tick) / TICKS_PER_YEAR;
        self.realms[ri].ruler = heir_idx as i32;
        self.realms[ri].family[heir_idx].reign_start = tick;

        let mut regency_note = String::new();
        if heir_age < PERSON_ADULT_AGE {
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
            if let Some(rg) = regent {
                let rn = self.realms[ri].family[rg].name.clone();
                regency_note = format!(" — too young to rule; {} governs as regent", rn);
            } else {
                regency_note = " — too young to rule, and no regent can be found".into();
            }
        } else {
            self.realms[ri].regent = -1;
        }

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
        if mother_age < PERSON_FERTILE_MIN || mother_age > PERSON_FERTILE_MAX { return; }

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
