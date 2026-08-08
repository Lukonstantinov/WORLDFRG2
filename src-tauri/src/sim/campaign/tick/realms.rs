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

/// Starting legitimacy/cohesion for a freshly proclaimed realm — high but not
/// perfect: the founding generation's own claim is the strongest a dynasty will ever
/// have, and both gauges are designed to be spent down by real events (§5 of the
/// plan), not to start at an artificial ceiling.
const REALM_FOUNDING_LEGITIMACY: f32 = 0.70;
const REALM_FOUNDING_COHESION: f32 = 1.0;

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
}
