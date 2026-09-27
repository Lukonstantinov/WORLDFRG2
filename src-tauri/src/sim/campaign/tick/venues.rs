//! venues — `docs/living_world/08_LEISURE_AND_GAMES.md`.
//!
//! Every culture prefers three of fourteen universal leisure TYPES; a city
//! builds venues its residents actually want, holds games there, and lets an
//! unviable one decline and be abandoned rather than converted.
//!
//! **Scope decided at 08.1, recorded rather than silently made** (the doc's
//! own open question): no new culture kit is added for Southeast Asian/
//! Polynesian leisure this session — that is a worldgen change owing its own
//! `goods_`/naming checks, out of a Living World row's normal budget. Every
//! one of the 18 SHIPPED kits gets the doc's own literal table
//! (`KIT_LEISURE`); any other culture (a legacy name, or one of the two
//! families with no kit) falls through to the doc's own trait-weighted rule
//! 2, which still guarantees three for EVERY culture. Adding the kit remains
//! the faithful option and is queued (Q08.3).
//!
//! **Real mechanism vs dosed-from-zero, stated plainly (rule 36):**
//! - Leisure preferences (08.1), venue popularity (08.2), the games
//!   calendar/festivals (08.4), distress/decline/abandonment (08.5) and
//!   performer spawning (08.6) are REAL yearly mechanism.
//! - Everything that would move real wealth is held at an explicit dose of
//!   zero: `VENUE_COST_DOSE` (building/upkeep spend and loans — 08.3),
//!   `VENUE_SPONSOR_CONTROL_DOSE` (a sponsoring house's `Official.control`
//!   rising over the aedile — 08.4's "marionette" effect),
//!   `GAMES_TRUCE_DOSE` (the Olympic-truce war-chance reduction — 08.7).
//!   All proven true no-ops at 0.0. A venue is still FOUNDED, still holds
//!   GAMES, still enters DISTRESS and is still ABANDONED at zero dose — only
//!   the money/control/war-chance SIDE EFFECTS wait for a future session's
//!   dose walk, exactly 07's `MASTERWORK_MARKET_DOSE` precedent.
//! - International games (08.7) ships as a real FLAG
//!   (`Venue.international_host`, tier >= 4) with no wired truce effect yet
//!   — `GAMES_TRUCE_DOSE` is the queued rest of that slice.
//!
//! **Further scope cuts (rule 36):** gladiators are not a separate
//! sub-record — an Arena venue only ever spawns a performer at all where
//! `bondage_permitted` allows it OR the culture doesn't need bondage
//! (a free professional either way; the doc's own "free professionals
//! elsewhere" — `Individual` carries no captive/free tag, since nothing yet
//! reads one). Fan-faction riots and ideology alignment (row 06) are NOT
//! built — queued as Q08.4, waiting on row 06's meters being a live input
//! rather than a descriptive one. The financing table (treasury / house
//! sponsor / bank loan / guild / liturgy) is recorded on the venue
//! (`funder_kind`) but every path is currently free (`VENUE_COST_DOSE`).
use super::*;

pub(crate) const LEISURE_ARENA: u8 = 0;
pub(crate) const LEISURE_RACING: u8 = 1;
pub(crate) const LEISURE_THEATRE: u8 = 2;
pub(crate) const LEISURE_ATHLETICS: u8 = 3;
pub(crate) const LEISURE_WRESTLING: u8 = 4;
pub(crate) const LEISURE_BALL_GAME: u8 = 5;
pub(crate) const LEISURE_POLO: u8 = 6;
pub(crate) const LEISURE_WATER_GAMES: u8 = 7;
pub(crate) const LEISURE_PLEASURE_DISTRICT: u8 = 8;
pub(crate) const LEISURE_BOARD_GAMES: u8 = 9;
pub(crate) const LEISURE_FAIR_FEAST: u8 = 10;
pub(crate) const LEISURE_RECITAL_EPIC: u8 = 11;
pub(crate) const LEISURE_RIVER_FESTIVAL: u8 = 12;
pub(crate) const LEISURE_BATHS: u8 = 13;
pub(crate) const LEISURE_TYPE_COUNT: usize = 14;

pub fn leisure_type_name(t: u8) -> &'static str {
    [
        "Arena", "Racing", "Theatre", "Athletics", "Wrestling", "Ball game", "Polo/horsemanship",
        "Water games", "Pleasure district", "Board and mind games", "Fair and feast",
        "Recital and epic", "River festival", "Baths",
    ].get(t as usize).copied().unwrap_or("Leisure")
}

/// The 18 shipped kits (`cultures::KITS`, fixed index order) → their three
/// preferred leisure types, the doc's own literal table (CLAUDE.md 08's
/// "18 shipped kits" section, verbatim).
const KIT_LEISURE: [[u8; 3]; 18] = [
    [LEISURE_ARENA, LEISURE_RACING, LEISURE_THEATRE],                 // 0 Roman
    [LEISURE_ARENA, LEISURE_RACING, LEISURE_THEATRE],                 // 1 Hellene
    [LEISURE_RACING, LEISURE_WATER_GAMES, LEISURE_FAIR_FEAST],        // 2 Punic
    [LEISURE_POLO, LEISURE_RECITAL_EPIC, LEISURE_BATHS],              // 3 Persian
    [LEISURE_WRESTLING, LEISURE_RECITAL_EPIC, LEISURE_FAIR_FEAST],    // 4 Norse
    [LEISURE_BALL_GAME, LEISURE_RECITAL_EPIC, LEISURE_FAIR_FEAST],    // 5 Celtic
    [LEISURE_RECITAL_EPIC, LEISURE_RACING, LEISURE_FAIR_FEAST],       // 6 Arab
    [LEISURE_WRESTLING, LEISURE_BOARD_GAMES, LEISURE_THEATRE],        // 7 Indic
    [LEISURE_PLEASURE_DISTRICT, LEISURE_BALL_GAME, LEISURE_BOARD_GAMES], // 8 Sinitic
    [LEISURE_FAIR_FEAST, LEISURE_WRESTLING, LEISURE_RECITAL_EPIC],    // 9 Slavic
    [LEISURE_BALL_GAME, LEISURE_FAIR_FEAST, LEISURE_RIVER_FESTIVAL],  // 10 Nahua
    [LEISURE_WRESTLING, LEISURE_POLO, LEISURE_RACING],                // 11 Turkic
    [LEISURE_RIVER_FESTIVAL, LEISURE_BOARD_GAMES, LEISURE_WATER_GAMES], // 12 Nilotic
    [LEISURE_RACING, LEISURE_RECITAL_EPIC, LEISURE_FAIR_FEAST],       // 13 Amazigh
    [LEISURE_WRESTLING, LEISURE_THEATRE, LEISURE_BALL_GAME],          // 14 Yamato
    [LEISURE_WRESTLING, LEISURE_POLO, LEISURE_RACING],                // 15 Mongol
    [LEISURE_FAIR_FEAST, LEISURE_RACING, LEISURE_RIVER_FESTIVAL],     // 16 Quechua
    [LEISURE_RECITAL_EPIC, LEISURE_WRESTLING, LEISURE_FAIR_FEAST],    // 17 Mande
];

/// Trait fallback (doc's own rule 2), for a culture whose kit maps to no
/// family above (none do, today) or an unknown/legacy culture name.
/// `cultures::TRAITS` index order (see `culture_ideals.rs`'s own comment):
/// Mercantile 0 · Seafaring 1 · Insular 2 · Martial 3 · Devout 4 · Nomadic 5
/// · Diaspora 6 · Assimilative 7 · Clannish 8 · Scholarly 9 · Agrarian 10 ·
/// Pastoral 11 · Artisan 12 · Xenophobic 13.
fn trait_leisure_weight(trait_idx: usize, leisure: u8) -> f32 {
    match (trait_idx, leisure) {
        (3, LEISURE_ARENA) | (3, LEISURE_WRESTLING) => 1.0,
        (5, LEISURE_POLO) | (5, LEISURE_RACING) | (11, LEISURE_POLO) | (11, LEISURE_RACING) => 1.0,
        (9, LEISURE_THEATRE) | (9, LEISURE_BOARD_GAMES) => 1.0,
        (12, LEISURE_THEATRE) | (12, LEISURE_PLEASURE_DISTRICT) => 1.0,
        (1, LEISURE_WATER_GAMES) => 1.0,
        (0, LEISURE_FAIR_FEAST) => 1.0,
        (10, LEISURE_FAIR_FEAST) | (10, LEISURE_RIVER_FESTIVAL) => 1.0,
        (8, LEISURE_WRESTLING) | (8, LEISURE_RECITAL_EPIC) => 1.0,
        _ => 0.0,
    }
}

/// Pure — the trait-weighted top three, ties broken by `tie_seed` (a hash
/// keyed off the culture's own name at the call site) so the result is
/// deterministic and repeatable. Split out of `culture_leisure_prefs` so it
/// (and `KIT_LEISURE` via `leisure_prefs_for_kit`) can be unit-tested
/// directly — `kit_of_people`/`culture_trait_ids` both resolve through the
/// ACTIVE worldgen culture map (`cultures::active()`, real hearths), which
/// no `tick::tests` fixture ever populates (§8.20/§8.19's own documented
/// reason not to touch that process-global from a test), so a fake culture
/// name string in a unit fixture can never exercise the kit path at all.
pub(crate) fn leisure_prefs_for_traits(ids: &[usize], tie_seed: u64) -> [u8; 3] {
    let mut scored: Vec<(u8, f32)> = (0..LEISURE_TYPE_COUNT as u8)
        .map(|t| {
            let w: f32 = ids.iter().map(|&tid| trait_leisure_weight(tid, t)).sum();
            let tie = hash01(tie_seed, t as u64 ^ 0xB001, 0xB002);
            (t, w + tie * 0.001)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    [scored[0].0, scored[1].0, scored[2].0]
}

/// Pure — the kit table read, clamped to a valid index.
pub(crate) fn leisure_prefs_for_kit(kit: usize) -> [u8; 3] {
    KIT_LEISURE[kit.min(KIT_LEISURE.len() - 1)]
}

impl CampaignSim {
    /// The doc's own three-preference rule: kit table first, else
    /// trait-weighted top three. Every culture gets exactly three.
    pub(crate) fn culture_leisure_prefs(&self, culture: &str) -> [u8; 3] {
        if let Some(kit) = crate::sim::cultures::kit_of_people(culture) {
            return leisure_prefs_for_kit(kit);
        }
        let ids = self.culture_trait_ids(culture);
        let tie_seed = self.seed ^ culture.bytes().fold(0xcbf29ce484222325u64, |a, b| (a ^ b as u64).wrapping_mul(0x100000001b3));
        leisure_prefs_for_traits(&ids, tie_seed)
    }

    /// Population-weighted share of this city's residents whose culture
    /// prefers `leisure` — the majority culture plus every recorded
    /// minority (`hub_minorities`, row 05's own sparse residency table).
    pub(crate) fn venue_popularity(&self, hub: usize, leisure: u8) -> f32 {
        let majority = self.hub_culture.get(hub).map(|c| c.as_str()).unwrap_or_default();
        if majority.is_empty() { return 0.0; }
        let minorities = self.hub_minorities.get(hub).cloned().unwrap_or_default();
        let minority_sum: f32 = minorities.iter().map(|(_, s)| *s).sum();
        let mut pop = 0.0f32;
        if self.culture_leisure_prefs(majority).contains(&leisure) {
            pop += (1.0 - minority_sum).max(0.0);
        }
        for (c, share) in &minorities {
            if self.culture_leisure_prefs(c).contains(&leisure) {
                pop += share;
            }
        }
        pop.clamp(0.0, 1.0)
    }
}

pub(crate) const COND_THRIVING: u8 = 0;
pub(crate) const COND_DECLINING: u8 = 1;
pub(crate) const COND_ABANDONED: u8 = 2;

pub(crate) const FUND_TREASURY: u8 = 0;
pub(crate) const FUND_HOUSE: u8 = 1;
pub(crate) const FUND_BANK: u8 = 2;
pub(crate) const FUND_GUILD: u8 = 3;
pub(crate) const FUND_LITURGY: u8 = 4;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Venue {
    pub id: u32,
    pub hub: i32,
    pub leisure_type: u8,
    pub tier: u8,
    pub name: String,
    pub condition: u8,
    pub funder_kind: u8,
    /// A sponsoring house, or −1 (city/treasury/guild/liturgy).
    pub sponsor_house: i32,
    pub built_tick: u32,
    pub games_held: u32,
    pub last_game_tick: u32,
    /// Consecutive years below the viability floor — resets the moment
    /// popularity/upkeep recovers. Past `VENUE_DISTRESS_LIMIT_YEARS` the
    /// venue is abandoned, never converted (the doc's own rule).
    pub distress_years: u32,
    pub prestige: f32,
    pub international_host: bool,
}

pub(crate) const VENUE_CAP_PER_CITY: usize = 6;
const VENUE_MIN_POPULARITY: f32 = 0.15;
const VENUE_DISTRESS_LIMIT_YEARS: u32 = 8;
const VENUE_PRESTIGE_CAP: f32 = 1.0;
const VENUE_FOUND_CHANCE: f32 = 0.02;
const VENUE_TIER4_MIN_POP: f32 = 80_000.0;
const VENUE_TIER5_MIN_POP: f32 = 200_000.0;

pub(crate) const VENUE_COST_DOSE: f32 = 0.0;
pub(crate) const VENUE_SPONSOR_CONTROL_DOSE: f32 = 0.0;
pub(crate) const GAMES_TRUCE_DOSE: f32 = 0.0;

/// Pure, testable twin of the sponsor-control bump (N6/S1 `_e` split).
pub(crate) fn venue_sponsor_control_e(control: f32, dose: f32) -> f32 {
    if dose <= 0.0 { return control; }
    (control + 0.05 * dose).min(1.0)
}

impl CampaignSim {
    /// 08.2/08.3 · yearly: a city whose Civil level supports it may found a
    /// new venue of its own MOST POPULAR type it doesn't already have, or
    /// nothing (below the cap `VENUE_CAP_PER_CITY`). Financing is CHOSEN
    /// (recorded as `funder_kind`) but currently free (`VENUE_COST_DOSE`).
    pub(crate) fn maybe_found_venue(&mut self, yr: u32) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let civil = self.hubs[h].track_level[TRACK_CIVIL];
            if civil < 1 { continue; }
            let existing = self.venues.iter().filter(|v| v.hub == h as i32 && v.condition != COND_ABANDONED).count();
            if existing >= VENUE_CAP_PER_CITY { continue; }
            let roll = hash01(self.seed, (yr as u64) ^ (h as u64).wrapping_mul(0x9E3779B1), 0xB010);
            if roll >= VENUE_FOUND_CHANCE { continue; }
            let best = (0..LEISURE_TYPE_COUNT as u8)
                .filter(|&t| !self.venues.iter().any(|v| v.hub == h as i32 && v.leisure_type == t && v.condition != COND_ABANDONED))
                .map(|t| (t, self.venue_popularity(h, t)))
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let Some((leisure, pop)) = best else { continue };
            if pop < VENUE_MIN_POPULARITY { continue; }
            let pop_size = self.hubs[h].population;
            let tier = if pop_size >= VENUE_TIER5_MIN_POP { 5 }
                else if pop_size >= VENUE_TIER4_MIN_POP { 4 }
                else { (civil).clamp(1, 3) };
            let funder_kind = [FUND_TREASURY, FUND_HOUSE, FUND_BANK, FUND_GUILD, FUND_LITURGY]
                [(hash01(self.seed, (yr as u64) ^ (h as u64), 0xB011) * 5.0) as usize % 5];
            let sponsor_house = if funder_kind == FUND_HOUSE {
                self.houses.iter().enumerate().filter(|(_, hh)| !hh.defunct && hh.hub == h as u32)
                    .max_by(|a, b| a.1.wealth.partial_cmp(&b.1.wealth).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i as i32).unwrap_or(-1)
            } else { -1 };
            let city = self.hubs[h].name.clone();
            let salt = (yr as u64) ^ (h as u64).wrapping_mul(0x9E3779B1) ^ 0xB012;
            let name = self.head_name_for(h, &city, salt);
            let id = self.next_venue_id;
            self.next_venue_id += 1;
            self.venues.push(Venue {
                id, hub: h as i32, leisure_type: leisure, tier, name: format!("the {} of {}", leisure_type_name(leisure), name),
                condition: COND_THRIVING, funder_kind, sponsor_house, built_tick: self.tick,
                games_held: 0, last_game_tick: 0, distress_years: 0, prestige: 0.2,
                international_host: tier >= 4,
            });
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "venue_founded".into(), hub: h as i32, good: -1,
                value: 0.0, text: format!("{} raises a venue for {}.", city, leisure_type_name(leisure).to_lowercase()),
            });
        }
    }

    /// 08.4 · games calendar + festivals; 08.5 · distress/decline/
    /// abandonment. One yearly pass over the live roster.
    pub(crate) fn run_venues(&mut self, yr: u32) {
        for vi in 0..self.venues.len() {
            if self.venues[vi].condition == COND_ABANDONED { continue; }
            let h = self.venues[vi].hub;
            if h < 0 || h as usize >= self.hubs.len() { continue; }
            let hu = h as usize;
            let pop = self.venue_popularity(hu, self.venues[vi].leisure_type);

            // Games: 2-6 a year, scaled by popularity and tier.
            let n_games = 2 + ((pop * 4.0) as u32).min(4);
            self.venues[vi].games_held += n_games;
            self.venues[vi].last_game_tick = self.tick;
            self.venues[vi].prestige = (self.venues[vi].prestige + 0.02 * n_games as f32).min(VENUE_PRESTIGE_CAP);

            // Major festival every 3-5 years (a deterministic stagger by id).
            if (yr + self.venues[vi].id) % 4 == 0 {
                self.venues[vi].prestige = (self.venues[vi].prestige + 0.05).min(VENUE_PRESTIGE_CAP);
            }

            // A sponsoring house's control over the aedile (dosed).
            if self.venues[vi].sponsor_house >= 0 && VENUE_SPONSOR_CONTROL_DOSE > 0.0 {
                if let Some(off) = self.hubs[hu].officials.iter_mut().find(|o| o.role == 3 /* Magistrate/Aedile */) {
                    off.control = venue_sponsor_control_e(off.control, VENUE_SPONSOR_CONTROL_DOSE);
                    off.house = self.venues[vi].sponsor_house;
                }
            }

            // Viability: below the popularity floor accrues distress; a
            // recovery resets it. Distress always resolves within a bound.
            if pop < VENUE_MIN_POPULARITY {
                self.venues[vi].distress_years += 1;
                self.venues[vi].condition = COND_DECLINING;
                if self.venues[vi].distress_years >= VENUE_DISTRESS_LIMIT_YEARS {
                    self.venues[vi].condition = COND_ABANDONED;
                    let city = self.hubs[hu].name.clone();
                    let name = self.venues[vi].name.clone();
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "venue_abandoned".into(), hub: h, good: -1,
                        value: 0.0, text: format!("{name} in {city} is abandoned, its games ended."),
                    });
                }
            } else {
                self.venues[vi].distress_years = 0;
                self.venues[vi].condition = COND_THRIVING;
            }
        }
    }

    pub(crate) fn maybe_spawn_performer(&mut self, yr: u32) {
        for vi in 0..self.venues.len() {
            if self.venues[vi].condition == COND_ABANDONED { continue; }
            let h = self.venues[vi].hub;
            if h < 0 || h as usize >= self.hubs.len() { continue; }
            let hu = h as usize;
            // An Arena venue's fighters are captives only where this city's
            // culture actually permits bondage — everywhere else (and every
            // OTHER leisure type, always) a free professional performs.
            if self.venues[vi].leisure_type == LEISURE_ARENA && !self.bondage_permitted(hu) {
                // Still a real venue with free professional performers —
                // just no captive-sourced fighters, per the doc's own rule.
            }
            let roll = hash01(self.seed, (yr as u64) ^ (self.venues[vi].id as u64).wrapping_mul(0x9E3779B1), 0xB020);
            if roll >= 0.01 { continue; }
            let salt = (yr as u64).wrapping_mul(0xB1AB) ^ (self.venues[vi].id as u64);
            let hub_name = self.hubs[hu].name.clone();
            let name = self.head_name_for(hu, &hub_name, salt ^ 0xB021);
            self.spawn_individual(hu, ROLE_PERFORMER, name, -1);
        }
    }
}
