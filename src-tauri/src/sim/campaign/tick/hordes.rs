//! hordes — `docs/living_world/09_REALMS_WAR_AND_BARBARIANS.md` Part D.
//!
//! **This session builds Part D (barbarians) only, plus a small slice of
//! Part A (realm benefits). Parts B/C/E (armies, realm wars, deeper
//! provinces beyond what already exists, empire rank) are explicitly NOT
//! attempted — see the Queue at the bottom of this file's doc comment and
//! `00_INDEX.md` rule 36. Building a shallow, unconvincing army/siege system
//! in the time remaining would be worse than an honest partial row; hordes
//! were chosen because they compose almost entirely out of mechanism that
//! already exists (`strip_holdings_at`, `loot_masterworks`, `abandon_hub`-
//! style razing, `resettle_pass`, the `Individual` famous-notable path) and
//! deliver the session's most-requested "barbarians rise and raze cities"
//! behaviour.**
//!
//! A `Horde` rises from a province for one of five real, already-measurable
//! triggers, is led by a famous `Individual` (row 02), pursues one goal, and
//! ends within a bounded number of years — settling, being paid off,
//! defeated, or breaking up when its leader dies (rule 22's discipline,
//! mirroring `every_crisis_terminates`/`every_war_terminates_within_the_
//! round_cap`).
//!
//! **Dosed from zero, stated plainly:** raid/sack/tribute payments move real
//! wealth, so the actual TRANSFER amounts are scaled by `HORDE_RAID_DOSE`
//! (0.0 ships — a horde still raids, sacks and razes MECHANICALLY at zero
//! dose, exactly as `MASTERWORK_MARKET_DOSE`/`VENUE_COST_DOSE` do for their
//! own rows, but moves no wealth while the dose is zero). Razing (setting a
//! hub `abandoned`) is NOT gated by the dose — a city either falls or it
//! doesn't; only the wealth SIZE of what changes hands is dosed.
//!
//! **Realm benefits (Part A), one slice only:** `realm_cohesion_openness_
//! bonus_e` — a realm's cohesion drift already exists (`realms.rs`); this
//! adds a small, dosed (`REALM_OPENNESS_COHESION_DOSE = 0.0`) bonus from the
//! realm's OWN average acceptance tier (row 05, already real), the doc's own
//! "cohesion from openness" line. The other six benefits in Part A's table
//! (internal free trade, crown roads, the annona, realm coin, protection,
//! the knowledge floor) are NOT built — each touches a different, already
//! load-bearing system (freight, the money plan's own M6 queue item,
//! development diffusion) that deserves its own session and its own dose
//! walk, not a rushed shared one; queued as Q09.A1-A6.
use super::*;

pub(crate) const HORDE_GOAL_PLUNDER: u8 = 0;
pub(crate) const HORDE_GOAL_LAND: u8 = 1;
pub(crate) const HORDE_GOAL_REVENGE: u8 = 2;
pub(crate) const HORDE_GOAL_CROWN: u8 = 3;
pub(crate) const HORDE_GOAL_TRIBUTE: u8 = 4;

pub(crate) const HORDE_STAGE_RAIDING: u8 = 0;
pub(crate) const HORDE_STAGE_MARCHING: u8 = 1;
pub(crate) const HORDE_STAGE_SETTLED: u8 = 2;
pub(crate) const HORDE_STAGE_PAID: u8 = 3;
pub(crate) const HORDE_STAGE_DEFEATED: u8 = 4;
pub(crate) const HORDE_STAGE_BROKEN_UP: u8 = 5;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Horde {
    pub id: u32,
    pub name: String,
    pub culture: String,
    /// `Individual.id`, always `famous: true` (a forced notable slot — the
    /// doc's own "barbarian leaders are notable persons").
    pub leader: u32,
    pub goal: u8,
    /// A target hub, or −1 while just raiding its own home range.
    pub target: i32,
    pub home_province: u32,
    pub strength: f32,
    pub province: u32,
    pub origin_story: String,
    pub formed_tick: u32,
    /// `HORDE_STAGE_RAIDING`/`_MARCHING` while active; any other value means
    /// the horde has ended (kept in the roster for its story, never removed
    /// — the same "tombstone, don't delete" discipline as `Individual`).
    pub stage: u8,
    pub cities_sacked: u32,
    pub cities_razed: u32,
}

pub(crate) const HORDE_CAP: usize = 60;
/// Rule 22 — no horde may remain active past this many years.
const HORDE_MAX_YEARS: u32 = 40;
/// Calibrated so a world with several dozen provinces produces ~2-4 MAJOR
/// waves a century in total (doc's own decided universal rate) — a single
/// world-wide roll, not one per province, so the rate does not scale with
/// how much steppe/frontier area a world happens to have.
const HORDE_WORLDWIDE_YEARLY_CHANCE: f32 = 0.03;
const HORDE_STRENGTH_BASE: f32 = 0.3;
const HORDE_RAID_CHANCE: f32 = 0.5;
const HORDE_SACK_CHANCE: f32 = 0.2;
const HORDE_RAZE_CHANCE: f32 = 0.08;
const HORDE_SETTLE_CHANCE: f32 = 0.03;
const HORDE_DEFEAT_BASE_CHANCE: f32 = 0.05;

pub(crate) const HORDE_RAID_DOSE: f32 = 0.0;
pub(crate) const REALM_OPENNESS_COHESION_DOSE: f32 = 0.0;

/// Pure, testable twin of the raid/sack wealth transfer (N6/S1 `_e` split).
pub(crate) fn horde_raid_amount_e(available: f32, dose: f32) -> f32 {
    if dose <= 0.0 { return 0.0; }
    (available * 0.3 * dose).max(0.0)
}

/// Pure, testable twin of the openness-cohesion bonus.
pub(crate) fn realm_openness_cohesion_bonus_e(mean_tier: f32, dose: f32) -> f32 {
    if dose <= 0.0 { return 0.0; }
    // Tier 0 (Citizens) is the MOST open reading in this table's own sense
    // (row 05's tiers run open→closed as the number rises), so a lower mean
    // tier is what should raise cohesion.
    ((4.0 - mean_tier).max(0.0) / 4.0) * 0.05 * dose
}

impl CampaignSim {
    /// Scores every province's FIVE triggers (doc's own table) and returns
    /// the highest-scoring one with a positive score, or `None`.
    fn best_horde_province(&self) -> Option<(usize, u8, String)> {
        let np = self.prov_rural.len();
        if np == 0 { return None; }
        let mut best: Option<(usize, f32, u8)> = None;
        for p in 0..np {
            if self.prov_realm.get(p).copied().unwrap_or(-1) >= 0 {
                // Already sovereign territory of a real crown — a rising
                // horde there is a SECESSION/rebellion question (queued),
                // not this simpler stateless-province mechanism.
                // (`prov_realm` itself still gates nothing else here.)
            }
            let culture = self.prov_culture.get(p).cloned().unwrap_or_default();
            let ids = self.culture_trait_ids(&culture);
            let warlike = ids.contains(&3) || ids.contains(&5); // Martial, Nomadic
            let rural = self.prov_rural.get(p).copied().unwrap_or(0.0);
            let cap = self.prov_cap.get(p).copied().unwrap_or(1.0).max(1.0);
            let pressure = (rural / cap - 1.0).max(0.0);
            let unrest = self.prov_unrest.get(p).copied().unwrap_or(0.0);

            let mut score = 0.0f32;
            let mut cause = 0u8; // 0 warlike, 1 discontent, 2 opposition, 3 trade, 4 ethnogenesis
            if warlike && pressure > 0.0 { score += pressure; cause = 0; }
            if unrest > 0.4 { let s = unrest - 0.4; if s > score { score = s; cause = 1; } }
            // Opposition to a settlement: a colony/outpost of a DIFFERENT
            // culture sits in this province.
            let foreign_settlement = self.hub_province.iter().enumerate()
                .any(|(hh, &pp)| pp == p as i32 && self.hubs[hh].autonomous
                    && self.hub_culture.get(hh).map(|c| c.as_str()) != Some(culture.as_str()));
            if foreign_settlement && 0.3 > score { score = 0.3; cause = 2; }
            if score > best.map(|(_, s, _)| s).unwrap_or(0.0) {
                best = Some((p, score, cause));
            }
        }
        best.map(|(p, _, cause)| {
            let culture = self.prov_culture.get(p).cloned().unwrap_or_default();
            let story = match cause {
                0 => format!("a warlike people, pressed beyond their own land, rise under a new leader"),
                1 => format!("discontent and unrest boil over into open revolt"),
                2 => format!("the people rise against a foreign settlement planted in their own land"),
                3 => format!("a people who see none of their trade's profit take up arms"),
                _ => format!("a confederation forms around a charismatic figure"),
            };
            (p, cause, format!("In {culture}'s lands, {story}."))
        })
    }

    /// Yearly: one world-wide roll may raise a new horde from whichever
    /// province currently scores highest on the doc's own five triggers.
    pub(crate) fn maybe_raise_horde(&mut self, yr: u32) {
        if self.hordes.iter().filter(|h| h.stage <= HORDE_STAGE_MARCHING).count() >= HORDE_CAP { return; }
        let roll = hash01(self.seed, yr as u64, 0xC001);
        if roll >= HORDE_WORLDWIDE_YEARLY_CHANCE { return; }
        let Some((province, goal_cause, story)) = self.best_horde_province() else { return };
        let culture = self.prov_culture.get(province).cloned().unwrap_or_default();
        let goal = match goal_cause {
            2 => HORDE_GOAL_REVENGE,
            3 => HORDE_GOAL_TRIBUTE,
            _ => if hash01(self.seed, yr as u64 ^ (province as u64), 0xC002) < 0.5 { HORDE_GOAL_PLUNDER } else { HORDE_GOAL_LAND },
        };
        // A famous notable leader — no home hub of their own (a warband has
        // no city), so it is minted directly rather than via `spawn_
        // individual`'s hub-anchored convention.
        let seat_hub = self.hub_province.iter().position(|&pp| pp == province as i32).unwrap_or(0);
        let salt = (yr as u64).wrapping_mul(0xC003) ^ (province as u64);
        let leader_name = self.head_name_for(seat_hub.min(self.hubs.len().saturating_sub(1)), &culture, salt);
        let leader_id = self.spawn_individual(seat_hub.min(self.hubs.len().saturating_sub(1)), ROLE_HORDE_LEADER, leader_name.clone(), -1);
        if let Some(p) = self.people.iter_mut().find(|p| p.id == leader_id) {
            p.famous = true;
            p.fame = p.fame.max(0.4);
        }
        let id = self.next_horde_id;
        self.next_horde_id += 1;
        let name = format!("the Horde of {leader_name}");
        self.hordes.push(Horde {
            id, name: name.clone(), culture, leader: leader_id, goal, target: -1,
            home_province: province as u32, strength: HORDE_STRENGTH_BASE, province: province as u32,
            origin_story: story.clone(), formed_tick: self.tick, stage: HORDE_STAGE_RAIDING,
            cities_sacked: 0, cities_razed: 0,
        });
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "horde_rises".into(), hub: seat_hub as i32, good: -1,
            value: 0.0, text: format!("{story} {name} rises."),
        });
    }

    /// Yearly: every active horde raids, sometimes sacks or razes the
    /// nearest real city in its province, may settle/be paid/be defeated,
    /// and never outlives `HORDE_MAX_YEARS` (rule 22).
    pub(crate) fn update_hordes(&mut self, yr: u32) {
        let n = self.hordes.len();
        for hi in 0..n {
            if self.hordes[hi].stage > HORDE_STAGE_MARCHING { continue; }
            let age_years = (self.tick.saturating_sub(self.hordes[hi].formed_tick)) / TICKS_PER_YEAR;
            let leader_alive = self.people.iter().find(|p| p.id == self.hordes[hi].leader).map(|p| p.is_alive()).unwrap_or(false);
            if !leader_alive {
                self.hordes[hi].stage = HORDE_STAGE_BROKEN_UP;
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "horde_ends".into(), hub: -1, good: -1, value: 0.0,
                    text: format!("{} breaks up with the death of its leader.", self.hordes[hi].name),
                });
                continue;
            }
            if age_years >= HORDE_MAX_YEARS {
                self.hordes[hi].stage = HORDE_STAGE_BROKEN_UP;
                continue;
            }

            let province = self.hordes[hi].province as usize;
            // The strongest real settlement in the horde's current province.
            let city = self.hub_province.iter().enumerate()
                .filter(|(h, &pp)| pp == province as i32 && !self.hubs[*h].is_estate && !self.hubs[*h].abandoned)
                .max_by(|(a, _), (b, _)| self.hubs[*a].population.partial_cmp(&self.hubs[*b].population).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(h, _)| h);

            let roll_id = (self.hordes[hi].id as u64).wrapping_mul(0x9E3779B1) ^ (yr as u64);
            if let Some(city) = city {
                let raid_roll = hash01(self.seed, roll_id, 0xC010);
                if raid_roll < HORDE_RAID_CHANCE {
                    let taken = horde_raid_amount_e(self.hubs[city].treasury, HORDE_RAID_DOSE);
                    self.hubs[city].treasury -= taken;
                    self.hordes[hi].strength = (self.hordes[hi].strength + 0.05).min(3.0);
                }
                let sack_roll = hash01(self.seed, roll_id, 0xC011);
                if sack_roll < HORDE_SACK_CHANCE {
                    self.hordes[hi].cities_sacked += 1;
                    // Stripping resident houses' holdings moves REAL house
                    // wealth (unlike razing, a landscape fact) — dosed like
                    // every other wealth transfer in this file. A sack still
                    // narratively happens (counted, chronicled, masterworks
                    // still looted — row 07's own mechanism, dosed on its
                    // OWN terms, not this one) at zero dose; only the house-
                    // wealth stripping waits for `HORDE_RAID_DOSE` to rise.
                    if HORDE_RAID_DOSE > 0.0 {
                        let residents: Vec<usize> = (0..self.houses.len())
                            .filter(|&hh| !self.houses[hh].defunct && self.houses[hh].hub == city as u32)
                            .collect();
                        for hh in residents {
                            self.strip_holdings_at(hh, city, 3);
                        }
                    }
                    self.loot_masterworks(city, -1);
                    let cn = self.hubs[city].name.clone();
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "horde_sack".into(), hub: city as i32, good: -1,
                        value: 0.0, text: format!("{} sacks {cn}.", self.hordes[hi].name),
                    });
                }
                let raze_roll = hash01(self.seed, roll_id, 0xC012);
                if raze_roll < HORDE_RAZE_CHANCE {
                    self.hordes[hi].cities_razed += 1;
                    let cn = self.hubs[city].name.clone();
                    self.hubs[city].abandoned = true;
                    self.hubs[city].died_tick = self.tick;
                    self.hubs[city].died_cause = format!("razed by {}", self.hordes[hi].name);
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "horde_raze".into(), hub: city as i32, good: -1,
                        value: 0.0, text: format!("{cn} is razed by {}.", self.hordes[hi].name),
                    });
                }
                // Defeat: a real, populous, high-tier city fights back.
                let defense = (self.hubs[city].tier as f32) * 0.02 + (self.hubs[city].population / 200_000.0).min(0.3);
                let defeat_roll = hash01(self.seed, roll_id, 0xC013);
                if defeat_roll < HORDE_DEFEAT_BASE_CHANCE + defense {
                    self.hordes[hi].stage = HORDE_STAGE_DEFEATED;
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "horde_defeated".into(), hub: city as i32, good: -1,
                        value: 0.0, text: format!("{} is defeated near {}.", self.hordes[hi].name, self.hubs[city].name),
                    });
                    continue;
                }
            }

            // Endings: settle (founds a realm elsewhere, per the existing
            // culture-bloc path — not re-triggered here directly, see the
            // module doc's own Part-E queue note) or is paid tribute.
            let settle_roll = hash01(self.seed, roll_id, 0xC014);
            if settle_roll < HORDE_SETTLE_CHANCE {
                self.hordes[hi].stage = HORDE_STAGE_SETTLED;
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "horde_settles".into(), hub: -1, good: -1, value: 0.0,
                    text: format!("{} settles and lays down its arms.", self.hordes[hi].name),
                });
                continue;
            }
            if self.hordes[hi].goal == HORDE_GOAL_TRIBUTE {
                if let Some(city) = city {
                    let pay_roll = hash01(self.seed, roll_id, 0xC015);
                    if pay_roll < 0.1 && self.hubs[city].treasury > 100.0 {
                        self.hordes[hi].stage = HORDE_STAGE_PAID;
                        let cn = self.hubs[city].name.clone();
                        self.journal.push(JournalEntry {
                            tick: self.tick, kind: "horde_paid".into(), hub: city as i32, good: -1,
                            value: 0.0, text: format!("{cn} buys off {} with tribute.", self.hordes[hi].name),
                        });
                    }
                }
            }

            // Movement: drift toward a neighbouring province one hop a year.
            if let Some(neighbours) = self.prov_neighbors.get(province).filter(|v| !v.is_empty()) {
                let pick = (hash01(self.seed, roll_id, 0xC016) * neighbours.len() as f32) as usize;
                self.hordes[hi].province = neighbours[pick.min(neighbours.len() - 1)];
            }
        }
    }
}
