//! THE ONE REALM WINDOW's reads and the world's starting realms
//! (`docs/PROVINCES_REALMS_LOGISTICS_REDESIGN.md` steps 4-6).
//!
//! `campaign_realm_overview` composes everything the window shows for one live realm in
//! a single read (it would otherwise take ~40 calls a year). The royal council is
//! DERIVED here from state the sim already holds — the ruler's family, the realm's
//! richest houses, famous people living in its cities, its largest member city — so it
//! can never move the economy. The start-realm commands edit world metadata before a
//! campaign exists.

use super::*;
use crate::sim::start_realms::{generate_start_realms, StartRealm};
use crate::sim::tick::TICKS_PER_YEAR;
use crate::sim::tick::crown::{SUCCESSION_NAMES, TOLERANCE_NAMES, TRADE_NAMES};

#[derive(Serialize, Clone)]
pub struct RealmGood { pub good: String, pub amount: f32 }

#[derive(Serialize, Clone)]
pub struct GovernorBrief { pub name: String, pub loyalty: f32, pub honesty: f32, pub years: u32, pub skim: f32, pub breakaway_risk: f32 }

/// A town of a province, for the province view.
#[derive(Serialize, Clone)]
pub struct TownBrief { pub hub: u32, pub name: String, pub population: f32, pub capital: bool, pub tier: u8, pub hub_class: u8, pub colony: bool }

#[derive(Serialize, Clone)]
pub struct RealmProvinceRow {
    pub id: u32, pub name: String, pub rural: f32, pub urban: f32, pub tax: f32, pub unrest: f32, pub culture: String,
    pub exports: Vec<RealmGood>, pub governor: Option<GovernorBrief>, pub capital: bool, pub town_hub: i32,
    /// The dues rate the province pays (0..), the surplus the countryside made beyond its own
    /// food, and the arrears owed — what `tax` is made of, so a zero can be explained.
    pub tax_rate: f32, pub surplus: f32, pub arrears: f32,
    /// Who the dues go to: a realm's crown, a city treasury or a house.
    pub dues_to: String,
    pub towns: Vec<TownBrief>,
}

/// One seat of the royal council. `kind`: 0 ruler · 1 the ruler's kin · 2 a great house ·
/// 3 a famous notable · 4 a city's delegate. `individual_id` links a notable to the life
/// window (−1 where the person is only a name in a family or house roster).
#[derive(Serialize, Clone)]
pub struct CouncilSeat {
    pub office: String, pub name: String, pub from: String, pub stance: String,
    pub female: bool, pub age: u32, pub individual_id: i32, pub house: i32, pub house_name: String, pub kind: u8,
}

#[derive(Serialize, Clone)]
pub struct RealmLawsBrief {
    pub tax: u8, pub levy: u8, pub succession: String, pub trade: String, pub town_fee: u8, pub tolerance: String,
}

#[derive(Serialize, Clone)]
pub struct RealmOverview {
    pub id: u32, pub name: String, pub title: String, pub color: [u8; 3], pub rank: u8,
    pub capital_hub: u32, pub capital_name: String, pub ruler: String, pub heir: String, pub regent: String,
    pub legitimacy: f32, pub cohesion: f32, pub treasury: f32, pub revenue: f32,
    pub economy_index: f32, pub power: f32, pub power_word: String, pub urban: f32, pub rural: f32,
    pub provinces: Vec<RealmProvinceRow>, pub goods: Vec<RealmGood>, pub cultures: Vec<RealmGood>,
    pub council: Vec<CouncilSeat>, pub laws: RealmLawsBrief, pub recent: Vec<String>,
}

/// Realm power = population × cohesion × (1 + (treasury + revenue) / 40k).
fn realm_power(sim: &CampaignSim, ri: usize) -> f32 {
    let r = &sim.realms[ri];
    let pop: f32 = r.provinces.iter().map(|&p| sim.prov_rural.get(p as usize).copied().unwrap_or(0.0)).sum::<f32>()
        + sim.hubs.iter().filter(|h| h.realm == ri as i32 && !h.is_estate).map(|h| h.population).sum::<f32>();
    pop / 1000.0 * r.cohesion.max(0.05) * (1.0 + (r.treasury.max(0.0) + r.tithe_last_year.max(0.0)) / 40_000.0)
}

/// Value of goods made per head, realm (or the world when `ri` is None).
fn value_per_head(sim: &CampaignSim, ri: Option<usize>) -> f32 {
    let (mut v, mut pop) = (0.0f32, 0.0f32);
    for h in &sim.hubs {
        if h.is_estate || h.abandoned { continue; }
        if let Some(ri) = ri { if h.realm != ri as i32 { continue; } }
        for (g, &q) in h.production.iter().enumerate() { v += q.max(0.0) * sim.goods.get(g).map(|tg| tg.base_value).unwrap_or(1.0); }
        pop += h.population;
    }
    if pop > 0.0 { v / pop } else { 0.0 }
}

#[tauri::command]
pub fn campaign_realm_overview(realm: u32, db: State<'_, WorldDb>) -> Result<Option<RealmOverview>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None) };
    let ri = realm as usize;
    let Some(r) = sim.realms.get(ri) else { return Ok(None) };
    let names: Vec<String> = metadata::get_meta(&conn, "provinces").ok().flatten()
        .and_then(|s| serde_json::from_str::<Vec<crate::sim::provinces::Province>>(&s).ok())
        .map(|ps| ps.into_iter().map(|p| p.name).collect()).unwrap_or_default();
    let ng = sim.goods.len();
    let np = sim.prov_realm.len();
    let capital_prov = sim.hub_province.get(r.capital_hub as usize).copied().unwrap_or(-1);
    let mut goods_net = vec![0.0f32; ng];
    let mut cultures: std::collections::BTreeMap<String, f32> = Default::default();
    let mut rows = Vec::new();
    let (mut urban_all, mut rural_all) = (0.0f32, 0.0f32);
    for &p in &r.provinces {
        let p = p as usize;
        if p >= np { continue; }
        let mut urban = 0.0f32;
        let mut town: (i32, f32) = (-1, -1.0);
        let mut towns: Vec<TownBrief> = Vec::new();
        for (h, hub) in sim.hubs.iter().enumerate() {
            if hub.is_estate || hub.abandoned || sim.hub_province.get(h).copied().unwrap_or(-1) != p as i32 { continue; }
            urban += hub.population;
            if hub.population > town.1 { town = (h as i32, hub.population); }
            towns.push(TownBrief { hub: h as u32, name: hub.name.clone(), population: hub.population,
                capital: h as u32 == r.capital_hub, tier: hub.tier, hub_class: hub.hub_class, colony: hub.colony_kind != 0 });
        }
        towns.sort_by(|a, b| b.population.partial_cmp(&a.population).unwrap_or(std::cmp::Ordering::Equal));
        towns.truncate(14);
        let rural = sim.prov_rural.get(p).copied().unwrap_or(0.0);
        let mut ex: Vec<RealmGood> = Vec::new();
        if sim.prov_export_year.len() == np * ng {
            for g in 0..ng {
                let e = sim.prov_export_year[p * ng + g];
                let i = sim.prov_import_year.get(p * ng + g).copied().unwrap_or(0.0);
                goods_net[g] += e - i;
                if e > 0.0 { ex.push(RealmGood { good: sim.goods[g].name.clone(), amount: e }); }
            }
        }
        ex.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        ex.truncate(3);
        let culture = sim.prov_culture.get(p).cloned().unwrap_or_default();
        *cultures.entry(culture.clone()).or_default() += rural + urban;
        urban_all += urban; rural_all += rural;
        let governor = sim.prov_governor.get(p).filter(|g| g.realm == ri as i32).map(|g| GovernorBrief {
            name: g.name.clone(), loyalty: g.loyalty, honesty: g.honesty,
            years: sim.tick.saturating_sub(g.since_tick) / TICKS_PER_YEAR, skim: g.skim(), breakaway_risk: sim.breakaway_risk(p),
        });
        let dues_to = {
            let hh = sim.prov_holder_house.get(p).copied().unwrap_or(-1);
            if hh >= 0 && (hh as usize) < sim.houses.len() && !sim.houses[hh as usize].defunct {
                format!("House {}", sim.houses[hh as usize].name)
            } else if let Some(seat) = sim.province_seat_hub(p) {
                format!("the treasury of {}", sim.hubs[seat].name)
            } else { "no one (free land)".into() }
        };
        rows.push(RealmProvinceRow {
            id: p as u32, name: names.get(p).cloned().unwrap_or_else(|| format!("Province {p}")),
            rural, urban, tax: sim.prov_revenue.get(p).copied().unwrap_or(0.0), unrest: sim.prov_unrest.get(p).copied().unwrap_or(0.0),
            culture, exports: ex, governor, capital: p as i32 == capital_prov, town_hub: town.0,
            tax_rate: sim.prov_tax.get(p).copied().unwrap_or(0.0), surplus: sim.prov_surplus.get(p).copied().unwrap_or(0.0),
            arrears: sim.prov_arrears.get(p).copied().unwrap_or(0.0), dues_to, towns,
        });
    }
    let total: f32 = cultures.values().sum::<f32>().max(1.0);
    let mut cult: Vec<RealmGood> = cultures.into_iter().filter(|(c, _)| !c.is_empty()).map(|(c, v)| RealmGood { good: c, amount: v / total }).collect();
    cult.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
    let mut goods: Vec<RealmGood> = (0..ng).filter(|&g| goods_net[g].abs() > 1e-3)
        .map(|g| RealmGood { good: sim.goods[g].name.clone(), amount: goods_net[g] }).collect();
    goods.sort_by(|a, b| b.amount.abs().partial_cmp(&a.amount.abs()).unwrap_or(std::cmp::Ordering::Equal));
    goods.truncate(16);

    // Crown: ruler, heir (the ruler's eldest living child), regent.
    let person = |i: i32| if i >= 0 { r.family.get(i as usize).map(|p| p.name.clone()).unwrap_or_default() } else { String::new() };
    let ruler = if r.ruler >= 0 { person(r.ruler) } else { format!("the council of {}", sim.hubs.get(r.capital_hub as usize).map(|h| h.name.as_str()).unwrap_or("?")) };
    let heir = if r.ruler >= 0 {
        r.family.iter().filter(|p| (p.father == r.ruler || p.mother == r.ruler) && p.died_tick == 0)
            .min_by_key(|p| p.born_tick).map(|p| p.name.clone()).unwrap_or_default()
    } else { String::new() };

    // The royal council, derived: kin, great houses, famous people, the largest city. Each
    // seat carries the person's sex and age (for a portrait) and, where the sim tracks one
    // as an Individual, the id that opens their life.
    let age_of = |born: u32| sim.tick.saturating_sub(born) / TICKS_PER_YEAR;
    let ruler_person = if r.ruler >= 0 { r.family.get(r.ruler as usize) } else { None };
    let mut council = vec![CouncilSeat {
        office: "Ruler".into(), name: ruler.clone(), from: r.title.clone(), stance: "The crown first".into(),
        female: ruler_person.map(|p| p.female).unwrap_or(false), age: ruler_person.map(|p| age_of(p.born_tick)).unwrap_or(45),
        individual_id: -1, house: -1, house_name: String::new(), kind: 0,
    }];
    let kin: Vec<&crate::sim::tick::Person> = r.family.iter().enumerate()
        .filter(|(i, p)| *i as i32 != r.ruler && p.died_tick == 0).map(|(_, p)| p).collect();
    let mut houses: Vec<(usize, &House)> = sim.houses.iter().enumerate()
        .filter(|(_, h)| !h.defunct && (h.hub as usize) < sim.hubs.len() && sim.hubs[h.hub as usize].realm == ri as i32).collect();
    houses.sort_by(|a, b| b.1.wealth.partial_cmp(&a.1.wealth).unwrap_or(std::cmp::Ordering::Equal));
    let mut famous: Vec<&crate::sim::tick::Individual> = sim.people.iter()
        .filter(|i| i.death_tick == 0 && i.current_hub >= 0 && (i.current_hub as usize) < sim.hubs.len() && sim.hubs[i.current_hub as usize].realm == ri as i32).collect();
    famous.sort_by(|a, b| b.fame.partial_cmp(&a.fame).unwrap_or(std::cmp::Ordering::Equal));
    let mut kin_i = kin.into_iter();
    let mut house_i = houses.into_iter();
    let mut fame_i = famous.into_iter();
    let house_seat = |(hi, h): (usize, &House)| {
        let k = h.kin.first();
        (k.map(|k| k.name.clone()).unwrap_or_default(), format!("House {}", h.name), k.map(|k| k.female).unwrap_or(false),
         k.map(|k| age_of(k.born_tick)).unwrap_or(45), hi as i32, h.name.clone())
    };
    for (office, stance) in [("Chancellor", "Keep the peace with neighbours"), ("Treasurer", "Lower tolls, more trade"),
                             ("Marshal", "Raise the levies"), ("Steward", "Roads and granaries")] {
        let seat = match office {
            "Treasurer" => house_i.next().map(|h| { let (n, f, fe, ag, hi, hn) = house_seat(h); (n, f, fe, ag, -1, hi, hn, 2u8) }),
            "Steward" => fame_i.next().map(|i| (i.name.clone(), "a famous notable".to_string(), i.female, age_of(i.birth_tick), i.id as i32, -1, String::new(), 3u8)),
            _ => kin_i.next().map(|p| (p.name.clone(), "the ruler's kin".to_string(), p.female, age_of(p.born_tick), -1, -1, String::new(), 1u8))
                .or_else(|| house_i.next().map(|h| { let (n, f, fe, ag, hi, hn) = house_seat(h); (n, f, fe, ag, -1, hi, hn, 2u8) })),
        };
        if let Some((name, from, female, age, ind, house, house_name, kind)) = seat {
            council.push(CouncilSeat { office: office.into(), name, from, stance: stance.into(), female, age, individual_id: ind, house, house_name, kind });
        }
    }
    if let Some((h, hub)) = sim.hubs.iter().enumerate()
        .filter(|(h, x)| x.realm == ri as i32 && !x.is_estate && *h != r.capital_hub as usize)
        .max_by(|a, b| a.1.population.partial_cmp(&b.1.population).unwrap_or(std::cmp::Ordering::Equal)) {
        let _ = h;
        council.push(CouncilSeat { office: "City delegate".into(), name: format!("Delegate of {}", hub.name), from: format!("{}'s own council", hub.name),
            stance: "Keep the town fee low".into(), female: false, age: 40, individual_id: -1, house: -1, house_name: String::new(), kind: 4 });
    }

    let powers: Vec<f32> = (0..sim.realms.len()).filter(|&k| sim.realms[k].fallen_tick == 0).map(|k| realm_power(&sim, k)).collect();
    let mean = (powers.iter().sum::<f32>() / powers.len().max(1) as f32).max(1e-3);
    let power = realm_power(&sim, ri);
    let rel = power / mean;
    let world = value_per_head(&sim, None).max(1e-6);
    let laws = &r.laws;
    Ok(Some(RealmOverview {
        id: realm, name: r.name.clone(), title: r.title.clone(),
        color: r.color.unwrap_or_else(|| super::province::state_color(r.id as usize)),
        rank: r.rank, capital_hub: r.capital_hub,
        capital_name: sim.hubs.get(r.capital_hub as usize).map(|h| h.name.clone()).unwrap_or_default(),
        ruler, heir, regent: person(r.regent), legitimacy: r.legitimacy, cohesion: r.cohesion,
        treasury: r.treasury, revenue: r.tithe_last_year,
        economy_index: (100.0 * value_per_head(&sim, Some(ri)) / world).round(),
        power, power_word: (if rel > 2.0 { "Dominant" } else if rel > 1.0 { "Strong" } else if rel > 0.5 { "Middling" } else { "Weak" }).into(),
        urban: urban_all, rural: rural_all, provinces: rows, goods, cultures: cult, council,
        laws: RealmLawsBrief {
            tax: laws.tax.max(1), levy: laws.levy.max(1),
            succession: SUCCESSION_NAMES[(laws.succession as usize).min(3)].into(),
            trade: TRADE_NAMES[(laws.trade as usize).min(3)].into(),
            town_fee: laws.town_fee.max(1), tolerance: TOLERANCE_NAMES[(laws.tolerance as usize).min(2)].into(),
        },
        recent: r.events.iter().rev().take(4).map(|e| format!("Year {} · {}", e.tick / TICKS_PER_YEAR, e.text)).collect(),
    }))
}

// ── The world's starting realms (before a campaign) ─────────────────────────────

fn stored_provinces(conn: &Connection) -> Vec<crate::sim::provinces::Province> {
    metadata::get_meta(conn, "provinces").ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// Roll and store fresh starting realms for the stored provinces (also called whenever
/// provinces are generated, so a world always has countries to edit).
pub(crate) fn roll_start_realms(conn: &Connection, seed: u64) -> Result<Vec<StartRealm>, String> {
    let realms = generate_start_realms(&stored_provinces(conn), seed);
    metadata::set_meta(conn, "start_realms", &serde_json::to_string(&realms).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(realms)
}

#[tauri::command]
pub fn get_start_realms(db: State<'_, WorldDb>) -> Result<Vec<StartRealm>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(metadata::get_meta(&conn, "start_realms").ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
}

#[tauri::command]
pub fn set_start_realms(realms: Vec<StartRealm>, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "start_realms", &serde_json::to_string(&realms).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn regenerate_start_realms(db: State<'_, WorldDb>) -> Result<Vec<StartRealm>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
    roll_start_realms(&conn, seed)
}

#[tauri::command]
pub fn get_realm_start_mode(db: State<'_, WorldDb>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(metadata::get_meta(&conn, "realm_start_mode").ok().flatten().unwrap_or_else(|| "continue".into()))
}

#[tauri::command]
pub fn set_realm_start_mode(mode: String, db: State<'_, WorldDb>) -> Result<(), String> {
    if !matches!(mode.as_str(), "continue" | "random" | "zero") { return Err(format!("unknown mode {mode}")); }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "realm_start_mode", &mode).map_err(|e| e.to_string())
}

/// At campaign start: make the world's starting realms live (or roll new ones, or none).
pub(crate) fn apply_start_realms(conn: &Connection, sim: &mut CampaignSim) {
    let mode = metadata::get_meta(conn, "realm_start_mode").ok().flatten().unwrap_or_else(|| "continue".into());
    if mode == "zero" || sim.prov_realm.is_empty() { return; }
    let realms: Vec<StartRealm> = if mode == "random" {
        generate_start_realms(&stored_provinces(conn), sim.seed ^ 0x5EA1)
    } else {
        metadata::get_meta(conn, "start_realms").ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    };
    let prov_names: Vec<String> = stored_provinces(conn).into_iter().map(|p| p.name).collect();
    for r in &realms {
        let provs: Vec<u32> = r.provinces.iter().copied().filter(|&p| (p as usize) < sim.prov_realm.len()).collect();
        if provs.is_empty() { continue; }
        // The seat: the capital province's largest town, else any town in the realm.
        let seat = std::iter::once(r.capital).chain(provs.iter().copied())
            .find_map(|p| sim.province_seat_hub(p as usize));
        let Some(seat) = seat else { continue };
        // A realm generated as "Kingdom of <capital PROVINCE>" is named for its capital CITY
        // once the campaign knows it (province and town names differ). A name the author
        // edited by hand is left exactly as written.
        let mut name = r.name.clone();
        if let (Some(pn), Some(hub)) = (prov_names.get(r.capital as usize), sim.hubs.get(seat)) {
            if !pn.is_empty() && name.ends_with(pn.as_str()) { name = format!("{}{}", &name[..name.len() - pn.len()], hub.name); }
        }
        sim.found_start_realm(seat, name, r.color, &provs);
    }
}

// ── Atlas standings & the political map's goods ────────────────────────────────

#[derive(Serialize, Clone)]
pub struct StandingRow {
    pub id: u32, pub name: String, pub color: [u8; 3], pub realm: i32, pub realm_name: String,
    pub population: f32, pub economy_index: f32, pub x: f32, pub y: f32,
}

#[derive(Serialize, Clone)]
pub struct Standings { pub realms: Vec<StandingRow>, pub provinces: Vec<StandingRow> }

/// Every living realm and every province with its population and economy index
/// (output value per head, world average = 100) — the Atlas ranks these, so the
/// window needs one read instead of one overview per realm.
#[tauri::command]
pub fn campaign_standings(db: State<'_, WorldDb>) -> Result<Option<Standings>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None) };
    let names: Vec<String> = stored_provinces(&conn).into_iter().map(|p| p.name).collect();
    let np = sim.prov_realm.len();
    let world = value_per_head(&sim, None).max(1e-6);
    let (mut urban, mut value) = (vec![0.0f32; np], vec![0.0f32; np]);
    for (h, hub) in sim.hubs.iter().enumerate() {
        if hub.is_estate || hub.abandoned { continue; }
        let p = sim.hub_province.get(h).copied().unwrap_or(-1);
        if p < 0 || p as usize >= np { continue; }
        let p = p as usize;
        urban[p] += hub.population;
        for (g, &q) in hub.production.iter().enumerate() { value[p] += q.max(0.0) * sim.goods.get(g).map(|tg| tg.base_value).unwrap_or(1.0); }
    }
    let realm_color = |ri: usize| sim.realms[ri].color.unwrap_or_else(|| super::province::state_color(sim.realms[ri].id as usize));
    let mut realms = Vec::new();
    for (ri, r) in sim.realms.iter().enumerate() {
        if r.fallen_tick != 0 { continue; }
        let pop: f32 = r.provinces.iter().map(|&p| sim.prov_rural.get(p as usize).copied().unwrap_or(0.0) + urban.get(p as usize).copied().unwrap_or(0.0)).sum();
        let cap = sim.hubs.get(r.capital_hub as usize);
        realms.push(StandingRow {
            id: ri as u32, name: r.name.clone(), color: realm_color(ri), realm: ri as i32, realm_name: r.name.clone(),
            population: pop, economy_index: (100.0 * value_per_head(&sim, Some(ri)) / world).round(),
            x: cap.map(|h| h.x).unwrap_or(0.0), y: cap.map(|h| h.y).unwrap_or(0.0),
        });
    }
    let mut provinces = Vec::new();
    for p in 0..np {
        let pop = sim.prov_rural.get(p).copied().unwrap_or(0.0) + urban[p];
        if pop < 1.0 { continue; }
        let ri = sim.prov_realm[p];
        let (color, realm_name) = if ri >= 0 && (ri as usize) < sim.realms.len() { (realm_color(ri as usize), sim.realms[ri as usize].name.clone()) } else { ([150, 140, 120], String::new()) };
        let seat = sim.prov_seat.get(p).copied().unwrap_or([0.0, 0.0]);
        provinces.push(StandingRow {
            id: p as u32, name: names.get(p).cloned().unwrap_or_else(|| format!("Province {p}")), color, realm: ri, realm_name,
            population: pop, economy_index: (100.0 * (value[p] / pop) / world).round(), x: seat[0], y: seat[1],
        });
    }
    Ok(Some(Standings { realms, provinces }))
}

#[derive(Serialize, Clone)]
pub struct ProvinceGoodShare { pub province: u32, pub goods: Vec<RealmGood>, pub unrest: f32, pub culture: String, pub rural: f32, pub urban: f32 }

/// Each province's production split by VALUE across the goods it makes (shares sum
/// to 1; goods under 8% are dropped, at most four kept, largest first). The
/// political map shows a good's icon where its share reaches 30%, so a province
/// with one icon is a specialist and one with three is balanced.
#[tauri::command]
pub fn campaign_province_good_shares(db: State<'_, WorldDb>) -> Result<Vec<ProvinceGoodShare>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(vec![]) };
    let ng = sim.goods.len();
    if ng == 0 { return Ok(vec![]); }
    let actual = sim.province_good_actual();
    let np = actual.len() / ng;
    let mut urban_by = vec![0.0f32; np];
    for (h, hub) in sim.hubs.iter().enumerate() {
        if hub.is_estate || hub.abandoned { continue; }
        let p = sim.hub_province.get(h).copied().unwrap_or(-1);
        if p >= 0 && (p as usize) < np { urban_by[p as usize] += hub.population; }
    }
    let mut out = Vec::new();
    for p in 0..np {
        let (rural, urban) = (sim.prov_rural.get(p).copied().unwrap_or(0.0), urban_by[p]);
        let vals: Vec<f32> = (0..ng).map(|g| actual[p * ng + g].max(0.0) * sim.goods[g].base_value.max(0.0)).collect();
        let total: f32 = vals.iter().sum();
        let unrest = sim.prov_unrest.get(p).copied().unwrap_or(0.0);
        let culture = sim.prov_culture.get(p).cloned().unwrap_or_default();
        if total <= 0.0 { out.push(ProvinceGoodShare { province: p as u32, goods: vec![], unrest, culture, rural, urban }); continue; }
        let mut goods: Vec<RealmGood> = (0..ng).filter(|&g| vals[g] / total >= 0.08)
            .map(|g| RealmGood { good: sim.goods[g].name.clone(), amount: vals[g] / total }).collect();
        goods.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        goods.truncate(4);
        out.push(ProvinceGoodShare { province: p as u32, goods, unrest, culture, rural, urban });
    }
    Ok(out)
}
