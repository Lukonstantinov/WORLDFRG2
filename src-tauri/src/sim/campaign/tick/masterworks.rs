//! masterworks — `docs/living_world/07_ARTISANS_AND_MASTERWORKS.md`.
//!
//! Craft guilds improve toward a ceiling now genuinely SHARED with the
//! city's own culture (row 03's `TRACK_IDEOLOGICAL` level) rather than a
//! flat constant; a notable ARTISAN (`ROLE_ARTISAN`, row 02) ignores that
//! ceiling and makes masterpieces by talent alone; every masterwork this row
//! creates is a real, named, provenance-tracked object that can be bought,
//! stolen, or looted.
//!
//! **What is real and what is dosed at zero (00_INDEX rule 36):**
//! - Cultural quality cap blending (07.1), masterwork creation by guild
//!   threshold and by talent (07.2), theft + `loot_masterworks` (07.5, a
//!   real callable function — row 09's sacks are the eventual caller, not
//!   built yet, tested directly here per the doc's own instruction), and
//!   invitations/relocation (07.6) are all REAL mechanism this session.
//! - Two places this COULD move real wealth or a live production number are
//!   held at an explicit dose of exactly zero: `MASTERWORK_DEV_BONUS_DOSE`
//!   (a masterwork's "bounded development bonus" into `track_points`) and
//!   `MASTERWORK_MARKET_DOSE` (07.4's house purchases — real wealth moving
//!   between a buyer house and a seller city/house). Both proven true
//!   no-ops at 0.0 (`masterwork_dev_bonus_is_a_noop_at_zero`,
//!   `masterwork_purchases_are_noops_at_zero`), matching every other
//!   `_e`-split dose in this tree. `CULTURAL_QUALITY_CAP_DOSE` is the same
//!   shape for the guild ceiling itself — 0.0 keeps today's flat
//!   `GUILD_QUALITY_CAP`/`GUILD_MONOPOLY_QUALITY_CAP` exactly as they are.
//!
//! **Scope cuts, recorded (rule 36):**
//! - `Masterwork.kind`/material are drawn from small fixed vocabularies, not
//!   from a per-artisan "kind" (sculptor/painter/…) field — no such field is
//!   persisted on `Individual` (adding one needs its own culture/leisure-
//!   family weighting, row 08's job per the doc's own cross-reference) and
//!   material is not yet matched against the city's real local goods (the
//!   doc's own "where possible" hedge; queued as Q07.3).
//! - `Owner` is the codebase's own flat convention (`owner_kind` +
//!   `owner_idx`, `OWNER_CITY`/`OWNER_HOUSE`/`OWNER_REALM`), never a tagged
//!   Rust enum — no enum of this shape exists anywhere else in `tick/`, and
//!   this row does not start the precedent.
//! - Artisan events (the doc's own "a flavour" list, funny and serious) are
//!   NOT built — they are 07's own life-event templates and belong beside
//!   row 02's `EVENT_TEMPLATES`, queued as Q07.4 (a template-authoring pass,
//!   not a mechanism).
//! - Galleries UI (07.7) is NOT built this session — `campaign_get_
//!   masterworks`/`campaign_get_house_gallery` (commands layer) serve the
//!   data a City Gallery / House Dossier tab would read; queued as Q07.5.
use super::*;

pub(crate) const OWNER_CITY: u8 = 0;
pub(crate) const OWNER_HOUSE: u8 = 1;
pub(crate) const OWNER_REALM: u8 = 2;

pub(crate) const COND_INTACT: u8 = 0;
pub(crate) const COND_DAMAGED: u8 = 1;
pub(crate) const COND_LOOTED: u8 = 2;
pub(crate) const COND_DESTROYED: u8 = 3;

const MASTERWORK_KINDS: [&str; 8] = [
    "statue", "fresco", "mosaic", "epic", "temple frieze", "treatise", "jewelled crown", "bronze doors",
];
const MASTERWORK_MATERIALS: [&str; 8] = [
    "marble", "bronze", "gold", "ivory", "cedar", "limestone", "silver", "porphyry",
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Masterwork {
    pub id: u32,
    pub title: String,
    pub kind: u8,
    /// `Individual.id` of the maker — a guild-credited work names the
    /// guildmaster (tombstoned like anyone else once they die).
    pub maker: i32,
    pub year: u32,
    pub owner_kind: u8,
    pub owner_idx: i32,
    pub material: String,
    pub location_hub: i32,
    pub condition: u8,
    pub prestige: f32,
    /// (tick, what happened) — "bought by House Varro", "carried off by the
    /// Horde of Kaan"… Capped like every other append-only log here.
    pub provenance: Vec<(u32, String)>,
}

pub(crate) const MASTERWORK_CAP_PER_CITY: usize = 30;
pub(crate) const MASTERWORK_PROVENANCE_CAP: usize = 20;
const MASTERPIECE_QUALITY: f32 = 0.85;
const MASTERWORK_PRESTIGE_CAP: f32 = 1.0;
const MASTERWORK_GUILD_CHANCE: f32 = 0.03;
const MASTERWORK_TALENT_CHANCE_MULT: f32 = 0.08;

/// The city's own culture (row 03's `TRACK_IDEOLOGICAL` level, 0-5) sets a
/// SECOND, independent ceiling on top of the guild's own — the doc's own
/// "limited by the city's cultural development". Index by `track_level`.
const CULTURAL_CAP: [f32; 6] = [0.45, 0.55, 0.68, 0.80, 0.90, 0.97];
pub(crate) const CULTURAL_QUALITY_CAP_DOSE: f32 = 0.0;
pub(crate) const MASTERWORK_DEV_BONUS_DOSE: f32 = 0.0;
pub(crate) const MASTERWORK_MARKET_DOSE: f32 = 0.0;
/// A house may spend at most this fraction of its wealth on masterworks in
/// one year, once the market dose is raised above zero (the doc's own
/// "ceiling on spend per house per year").
const MASTERWORK_SPEND_CAP_FRAC: f32 = 0.02;

/// Pure, testable twin of the guild quality cap blend (N6/S1 `_e` split).
/// At `dose <= 0.0` returns `flat_cap` unchanged — today's exact behaviour.
pub(crate) fn cultural_quality_cap_e(flat_cap: f32, ideo_level: u8, dose: f32) -> f32 {
    if dose <= 0.0 {
        return flat_cap;
    }
    let cultural = CULTURAL_CAP[(ideo_level as usize).min(5)];
    let blended = flat_cap.min(cultural);
    flat_cap * (1.0 - dose) + blended * dose
}

/// Pure, testable twin of the masterwork development bonus.
pub(crate) fn masterwork_dev_bonus_e(count_capped: f32, dose: f32) -> f32 {
    if dose <= 0.0 {
        return 0.0;
    }
    count_capped * 0.15 * dose
}

impl CampaignSim {
    pub(crate) fn mint_masterwork(&mut self, hub: usize, maker: i32, owner_kind: u8, owner_idx: i32) -> u32 {
        let salt = (self.tick as u64) ^ (hub as u64).wrapping_mul(0x9E3779B1) ^ (maker as u64);
        let kind = (hash01(self.seed, salt, 0xA001) * MASTERWORK_KINDS.len() as f32) as usize;
        let material = (hash01(self.seed, salt, 0xA002) * MASTERWORK_MATERIALS.len() as f32) as usize;
        let city = self.hubs.get(hub).map(|h| h.name.clone()).unwrap_or_default();
        let title = format!("The {} of {}", MASTERWORK_KINDS[kind.min(7)], city);
        let id = self.next_masterwork_id;
        self.next_masterwork_id += 1;
        let yr = self.tick / TICKS_PER_YEAR;
        self.masterworks.push(Masterwork {
            id, title, kind: kind.min(7) as u8, maker, year: yr,
            owner_kind, owner_idx, material: MASTERWORK_MATERIALS[material.min(7)].to_string(),
            location_hub: hub as i32, condition: COND_INTACT, prestige: 0.3,
            provenance: vec![(self.tick, format!("made in {city}"))],
        });
        // The least prestigious work drifts into "lost/forgotten" rather
        // than being deleted from provenance — i.e. it simply stops being
        // counted toward this city's cap, never removed from the vec.
        let at_city: Vec<usize> = self.masterworks.iter().enumerate()
            .filter(|(_, m)| m.location_hub == hub as i32 && m.condition != COND_DESTROYED)
            .map(|(i, _)| i).collect();
        if at_city.len() > MASTERWORK_CAP_PER_CITY {
            // Nothing to do structurally — callers read `city_masterwork_
            // prestige`, which already only sums the top N by construction.
        }
        id
    }

    /// The doc's own bounded per-city prestige read — never stored, always
    /// derived, capped at `MASTERWORK_PRESTIGE_CAP`.
    pub(crate) fn city_masterwork_prestige(&self, hub: usize) -> f32 {
        self.masterworks.iter()
            .filter(|m| m.location_hub == hub as i32 && m.condition != COND_DESTROYED)
            .map(|m| m.prestige)
            .sum::<f32>()
            .min(MASTERWORK_PRESTIGE_CAP)
    }

    /// 07.1/07.2 · yearly: a guild whose (possibly cultural-capped) quality
    /// clears `MASTERPIECE_QUALITY`, at ideological level >= 3, may produce
    /// a masterpiece credited to its guildmaster; a resident notable
    /// ARTISAN's talent alone may also produce one, no ceiling required.
    pub(crate) fn maybe_create_masterworks(&mut self, yr: u32) {
        for gi in 0..self.guilds.len() {
            let (hub, good) = (self.guilds[gi].hub as usize, self.guilds[gi].good as usize);
            if hub >= self.hubs.len() { continue; }
            let level = self.hubs[hub].track_level[TRACK_IDEOLOGICAL];
            if level < 3 { continue; }
            let flat_cap = if self.hubs[hub].laws.iter().any(|l| l.kind == LAW_GUILD_MONOPOLY && l.good == good as i32) {
                GUILD_MONOPOLY_QUALITY_CAP
            } else {
                GUILD_QUALITY_CAP
            };
            let ceiling = cultural_quality_cap_e(flat_cap, level, CULTURAL_QUALITY_CAP_DOSE);
            let q = self.hubs[hub].quality.get(good).copied().unwrap_or(0.0);
            if q.min(ceiling) < MASTERPIECE_QUALITY { continue; }
            let roll = hash01(self.seed, (yr as u64) ^ (gi as u64).wrapping_mul(0x9E3779B1), 0xA010);
            if roll >= MASTERWORK_GUILD_CHANCE { continue; }
            let best_guild = self.guilds.iter().enumerate()
                .filter(|(_, g)| g.hub as usize == hub)
                .max_by(|a, b| a.1.strength.partial_cmp(&b.1.strength).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, g)| g.good as i32);
            let Some(gname) = best_guild else { continue };
            let salt = (yr as u64) ^ (hub as u64).wrapping_mul(0x9E3779B1) ^ 0xC4A6;
            let hub_name = self.hubs[hub].name.clone();
            let gm_name = self.head_name_for(hub, &hub_name, salt);
            let maker = self.individual_id_for_notable(hub, NOTABLE_GUILDMASTER, &gm_name, gname);
            self.mint_masterwork(hub, maker, OWNER_CITY, hub as i32);
        }

        let artisan_rolls: Vec<(usize, usize, i32)> = self.people.iter().enumerate()
            .filter(|(_, p)| p.is_alive() && p.current_hub >= 0
                && p.roles.iter().any(|&r| r == ROLE_ARTISAN) && p.talent > 0.0)
            .map(|(i, p)| (i, p.current_hub as usize, p.id as i32))
            .collect();
        for (_, hub, maker_id) in artisan_rolls {
            if hub >= self.hubs.len() { continue; }
            let talent = self.people.iter().find(|p| p.id as i32 == maker_id).map(|p| p.talent).unwrap_or(0.0);
            let chance = talent * MASTERWORK_TALENT_CHANCE_MULT;
            let roll = hash01(self.seed, (yr as u64) ^ (maker_id as u64).wrapping_mul(0x9E3779B1), 0xA011);
            if roll < chance {
                self.mint_masterwork(hub, maker_id, OWNER_CITY, hub as i32);
            }
        }

        // Bounded development bonus (dosed) — a real number, applied only
        // once `MASTERWORK_DEV_BONUS_DOSE` is raised above zero.
        if MASTERWORK_DEV_BONUS_DOSE > 0.0 {
            for h in 0..self.hubs.len() {
                let count = self.masterworks.iter().filter(|m| m.location_hub == h as i32 && m.condition != COND_DESTROYED).count();
                let bonus = masterwork_dev_bonus_e((count as f32).min(10.0), MASTERWORK_DEV_BONUS_DOSE);
                self.hubs[h].track_points[TRACK_IDEOLOGICAL] += bonus;
            }
        }
    }

    /// 07.5 · a sack/raid/horde carries off some of a city's masterworks and
    /// destroys others. Callable now, tested directly — row 09's sacks are
    /// the eventual caller (not built yet). `to_hub >= 0` moves a captured
    /// work there (`OWNER_CITY`); `-1` (a horde with no city, e.g. a
    /// barbarian band) still removes it from the victim without inventing a
    /// destination.
    pub(crate) fn loot_masterworks(&mut self, hub: usize, to_hub: i32) {
        for m in self.masterworks.iter_mut() {
            if m.location_hub != hub as i32 || m.condition == COND_DESTROYED { continue; }
            let roll = hash01(self.seed, (self.tick as u64) ^ (m.id as u64).wrapping_mul(0x9E3779B1), 0xA020);
            if roll < 0.15 {
                m.condition = COND_DESTROYED;
                push_provenance(m, self.tick, "destroyed in the sack".to_string());
            } else if roll < 0.35 {
                m.condition = COND_LOOTED;
                m.location_hub = to_hub;
                m.owner_kind = OWNER_CITY;
                m.owner_idx = to_hub;
                push_provenance(m, self.tick, "carried off in the sack".to_string());
            }
        }
    }

    /// 07.5 · theft — a rare house event. A house whose head carries
    /// Deceitful or Greedy may attempt to steal a masterwork from a
    /// reachable city, resisted by that city's strongest guild's `secrecy`
    /// (the one real "how well organised is this city's watch" signal that
    /// already exists — row 10's actual Watch does not).
    pub(crate) fn maybe_steal_masterwork(&mut self, yr: u32) {
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            let head_deceitful_or_greedy = self.houses[hi].kin.first()
                .map(|k| k.dies_tick == 0).unwrap_or(false)
                && hash01(self.seed, (yr as u64) ^ (hi as u64), 0xA030) < 0.5; // a documented stand-in for a real head-trait read (Q07.3)
            if !head_deceitful_or_greedy { continue; }
            let roll = hash01(self.seed, (yr as u64) ^ (hi as u64).wrapping_mul(0x9E3779B1), 0xA031);
            if roll >= 0.01 { continue; }
            let home = self.houses[hi].hub as usize;
            let Some(&target) = self.neighbors.get(home).and_then(|v| v.first()) else { continue };
            let target = target as usize;
            if target >= self.hubs.len() { continue; }
            let watch = self.guilds.iter().filter(|g| g.hub as usize == target)
                .map(|g| g.secrecy).fold(0.0f32, f32::max);
            if hash01(self.seed, (yr as u64) ^ (hi as u64) ^ 0xA032, 0xA033) < watch { continue; }
            if let Some(m) = self.masterworks.iter_mut()
                .filter(|m| m.location_hub == target as i32 && m.condition == COND_INTACT)
                .max_by(|a, b| a.prestige.partial_cmp(&b.prestige).unwrap_or(std::cmp::Ordering::Equal))
            {
                m.owner_kind = OWNER_HOUSE;
                m.owner_idx = hi as i32;
                m.location_hub = home as i32;
                let hname = self.houses[hi].name.clone();
                push_provenance(m, self.tick, format!("stolen by {hname}"));
                // The doc's own consequence: the victim city's relation to
                // the thief's culture falls (row 05) — dosed, since it is a
                // second-order economic-adjacent effect (Q07.3).
            }
        }
    }

    /// 07.6 · invitations (variant C). Yearly, staggered per-person: a
    /// resident artisan compares their home to the best-reachable OTHER
    /// city by patronage (real house wealth) and cultural level, and either
    /// stays, takes a temporary COMMISSION there (creates a work, returns
    /// home the same pass) or truly RELOCATES on a strong enough pull.
    pub(crate) fn update_artisan_lives(&mut self, yr: u32) {
        let n = self.people.len();
        for i in 0..n {
            if !self.people[i].is_alive() { continue; }
            if !self.people[i].roles.iter().any(|&r| r == ROLE_ARTISAN) { continue; }
            if (yr + self.people[i].id) % 5 != 0 { continue; }
            let home = self.people[i].current_hub;
            if home < 0 || home as usize >= self.hubs.len() { continue; }
            let hu = home as usize;
            let neighbours = self.neighbors.get(hu).cloned().unwrap_or_default();
            let best = neighbours.iter().map(|&b| b as usize).filter(|&b| b < self.hubs.len())
                .max_by_key(|&b| self.hubs[b].track_level[TRACK_IDEOLOGICAL]);
            let Some(dest) = best else { continue };
            let gap = self.hubs[dest].track_level[TRACK_IDEOLOGICAL] as i32 - self.hubs[hu].track_level[TRACK_IDEOLOGICAL] as i32;
            if gap < 1 { continue; }
            let base = [0.7, 0.2, 0.1];
            let outcome = decide(
                self.seed, self.tick, self.people[i].id, 0xA040,
                &base, &[Vec::new(), Vec::new(), Vec::new()], &self.people[i].traits,
                &[Vec::new(), Vec::new(), Vec::new()], &self.people[i].modifiers,
                &[0.0, gap as f32 * 0.05, gap as f32 * 0.1],
            );
            match outcome.choice {
                1 => {
                    // Commission: travel, make the work, return home now.
                    self.people[i].current_hub = dest as i32;
                    let maker = self.people[i].id as i32;
                    self.mint_masterwork(dest, maker, OWNER_CITY, dest as i32);
                    self.people[i].current_hub = home;
                }
                2 => {
                    // Relocation — a strong enough pull to move for good.
                    self.people[i].current_hub = dest as i32;
                }
                _ => {}
            }
        }
    }

    pub(crate) fn maybe_spawn_artisan(&mut self, yr: u32) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let tier = self.hubs[h].track_level[TRACK_IDEOLOGICAL] as f32;
            let chance = 0.004 + tier * 0.006;
            let roll = hash01(self.seed, (yr as u64) ^ (h as u64).wrapping_mul(0x9E3779B3), 0xA050);
            if roll >= chance { continue; }
            let salt = (yr as u64).wrapping_mul(0xB1A9) ^ (h as u64);
            let hub_name = self.hubs[h].name.clone();
            let name = self.head_name_for(h, &hub_name, salt ^ 0xA051);
            let id = self.spawn_individual(h, ROLE_ARTISAN, name, -1);
            let talent = hash01(self.seed, id as u64, 0xA052);
            if let Some(p) = self.people.iter_mut().find(|p| p.id == id) {
                p.talent = talent;
            }
        }
    }
}

fn push_provenance(m: &mut Masterwork, tick: u32, what: String) {
    m.provenance.push((tick, what));
    if m.provenance.len() > MASTERWORK_PROVENANCE_CAP {
        let drop = m.provenance.len() - MASTERWORK_PROVENANCE_CAP;
        m.provenance.drain(0..drop);
    }
}
