//! read_hubs commands — split from the former monolithic campaign_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Journal rows, optionally filtered to a hub and/or good (−1 = any), for the
/// settlement window's price/event history.
#[tauri::command]
pub fn campaign_get_journal(
    hub: i32,
    good: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<JournalEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s,
        None => return Ok(vec![]),
    };
    Ok(sim
        .journal
        .iter()
        .filter(|e| (hub < 0 || e.hub == hub) && (good < 0 || e.good == good))
        .cloned()
        .collect())
}


/// Full live detail for one settlement (sentiment, market, history) for the
/// redesigned settlement window. Returns None-equivalent error string handling
/// via an empty Option when no sim / hub.
/// The live city list the MARKETS window's picker searches — id, name, population
/// and position for every non-estate, non-abandoned hub.
///
/// Deliberately NOT served from the worldgen `economy` snapshot the rest of the UI
/// picks cities from: that snapshot is frozen at finalize, so towns founded DURING
/// the campaign (swarm towns, finished satellites, independent ex-colonies) are
/// absent from it and would simply be unreachable in the picker.
#[tauri::command]
pub fn campaign_market_cities(db: State<'_, WorldDb>) -> Result<Vec<MarketCity>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s,
        None => return Ok(vec![]),
    };
    let mut out: Vec<MarketCity> = sim.hubs.iter().enumerate()
        .filter(|(_, h)| !h.is_estate && !h.abandoned && h.population >= 1.0)
        .map(|(i, h)| MarketCity {
            id: i as u32, name: h.name.clone(), population: h.population, x: h.x, y: h.y,
        })
        .collect();
    out.sort_by(|a, b| b.population.partial_cmp(&a.population).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}


#[tauri::command]
pub fn campaign_get_hub(id: u32, db: State<'_, WorldDb>) -> Result<Option<HubDetail>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s,
        None => return Ok(None),
    };
    let hi = match sim.hubs.iter().position(|h| h.id == id) {
        Some(i) => i,
        None => return Ok(None),
    };
    let hub = &sim.hubs[hi];
    let ng = sim.goods.len();
    // The persisted per-(hub, good) price/volume series for THIS hub, indexed by
    // good. `trade_hist` is a sparse row list (only pairs that actually traded),
    // so this is one linear pass rather than a scan per good.
    let mut price_hist: Vec<&[f32]> = vec![&[]; ng];
    let mut vol_hist: Vec<&[f32]> = vec![&[]; ng];
    for th in &sim.trade_hist {
        if th.hub as usize != hi { continue; }
        let g = th.good as usize;
        if g < ng { price_hist[g] = &th.prices; vol_hist[g] = &th.vols; }
    }
    // Per-good world cheapest/dearest (×-world price) across hubs.
    let goods: Vec<HubGoodDetail> = (0..ng)
        .map(|g| {
            let base = sim.goods[g].base_value.max(1e-3);
            let mut wmin = (f32::INFINITY, "");
            let mut wmax = (f32::NEG_INFINITY, "");
            let mut wsum = 0.0f32;
            for h in &sim.hubs {
                let xw = h.price[g] / base;
                if xw < wmin.0 { wmin = (xw, h.name.as_str()); }
                if xw > wmax.0 { wmax = (xw, h.name.as_str()); }
                wsum += xw;
            }
            let world_avg = if !sim.hubs.is_empty() { wsum / sim.hubs.len() as f32 } else { 1.0 };
            HubGoodDetail {
                good: g,
                name: sim.goods[g].name.clone(),
                price: hub.price[g],
                base_value: sim.goods[g].base_value,
                stock: crate::sim::tick::stock_of(&hub.stock, g),
                // The REAL per-tick demand the sim consumes (matches base_need in
                // tick.rs): pop × tier-weight × desire × need_scale × demand pressure.
                // Showing raw desire×pop made every good read "0% of need".
                need: hub.population
                    * [1.0f32, 0.45, 0.22][sim.goods[g].need_tier.min(2) as usize]
                    * sim.goods[g].desire.max(0.0)
                    * sim.need_scale
                    * crate::sim::tick::DEMAND_PRESSURE,
                production: hub.production[g],
                world_min: if wmin.0.is_finite() { wmin.0 } else { 0.0 },
                world_min_hub: wmin.1.to_string(),
                world_max: if wmax.0.is_finite() { wmax.0 } else { 0.0 },
                world_max_hub: wmax.1.to_string(),
                world_avg,
                quality: hub.quality.get(g).copied().unwrap_or(0.0),
                grade: if hub.production[g] > 0.0 {
                    crate::sim::tick::quality_grade(hub.quality.get(g).copied().unwrap_or(0.0)).to_string()
                } else { String::new() },
                price_hist: price_hist[g].to_vec(),
                vol_hist: vol_hist[g].to_vec(),
                supply_shares: {
                    let sbase = g * crate::sim::tick::SUPPLY_CLASSES;
                    let raw: [f32; 5] = std::array::from_fn(|c|
                        hub.supply_accum.get(sbase + c).copied().unwrap_or(0.0).max(0.0));
                    let total: f32 = raw.iter().sum();
                    if total > 1e-3 { raw.map(|v| v / total) } else { [0.0; 5] }
                },
                depot_stock: sim.warehouses.iter()
                    .filter(|w| w.owner >= 0 && w.hub as usize == hi)
                    .map(|w| w.stock.get(g).copied().unwrap_or(0.0))
                    .sum(),
                depot_holders: {
                    let mut rows: Vec<(String, bool, f32)> = sim.warehouses.iter()
                        .filter(|w| w.owner >= 0 && w.hub as usize == hi)
                        .filter_map(|w| {
                            let amt = w.stock.get(g).copied().unwrap_or(0.0);
                            if amt <= 0.01 { return None; }
                            let h = sim.houses.get(w.owner as usize)?;
                            Some((h.name.clone(), h.is_guild, amt))
                        }).collect();
                    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
                    rows.truncate(6);
                    rows
                },
            }
        })
        .collect();
    // ── Market flow: in-flight shipments arriving at / departing this hub, each
    //    tagged with its owner (house/guild/local) and round-trip return legs,
    //    ranked by value (highest first). ──
    let owner_name = |o: i32| if o >= 0 {
        sim.houses.get(o as usize).map(|h| h.name.clone()).unwrap_or_else(|| "—".into())
    } else { "Local merchants".into() };
    let owner_color = |o: i32| if o >= 0 { distinct_color(o as usize) } else { "#7a8aa0".to_string() };
    let owner_is_guild = |o: i32| o >= 0 && sim.houses.get(o as usize).map(|h| h.is_guild).unwrap_or(false);
    let cname = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let mk_row = |s: &crate::sim::tick::InTransit, other: u32| -> ShipmentRow {
        let g = s.good.min(ng.saturating_sub(1));
        let base = sim.goods[g].base_value.max(1e-3);
        ShipmentRow {
            owner: owner_name(s.owner), color: owner_color(s.owner), is_guild: owner_is_guild(s.owner),
            other: cname(other), good: sim.goods[g].name.clone(), amount: s.amount,
            price: hub.price[g] / base, value: s.amount * hub.price[g], sea: s.sea,
            returning_home: s.phase == 1,
            // The price the cargo actually left at — 0 on a pre-existing save whose
            // in-flight legs predate the field, which the view renders as "—" rather
            // than as a free good.
            deal_price: if s.price > 0.0 { s.price / base } else { 0.0 },
            eta_days: s.eta_tick.saturating_sub(sim.tick),
            age_days: 0,
        }
    };
    let mut arrivals: Vec<ShipmentRow> = Vec::new();
    let mut departures: Vec<ShipmentRow> = Vec::new();
    for s in &sim.in_transit {
        if s.good >= ng { continue; }
        if s.to as usize == hi { arrivals.push(mk_row(s, s.from)); }
        if s.from as usize == hi { departures.push(mk_row(s, s.to)); }
    }
    let by_value = |a: &ShipmentRow, b: &ShipmentRow|
        b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal);
    arrivals.sort_by(by_value); arrivals.truncate(40);
    departures.sort_by(by_value); departures.truncate(40);
    // Recently completed deals (most recent first), split by direction at this hub.
    let mk_recent = |r: &crate::sim::tick::RecentTrade, other: u32| -> ShipmentRow {
        let g = r.good.min(ng.saturating_sub(1));
        let base = sim.goods[g].base_value.max(1e-3);
        ShipmentRow {
            owner: owner_name(r.owner), color: owner_color(r.owner), is_guild: owner_is_guild(r.owner),
            other: cname(other), good: sim.goods[g].name.clone(), amount: r.amount,
            price: hub.price[g] / base, value: r.amount * r.price, sea: r.sea, returning_home: false,
            deal_price: r.price / base,
            eta_days: 0,
            age_days: sim.tick.saturating_sub(r.tick),
        }
    };
    let mut recent_arrivals: Vec<ShipmentRow> = Vec::new();
    let mut recent_departures: Vec<ShipmentRow> = Vec::new();
    for r in sim.recent_trades.iter().rev() {
        if r.good >= ng { continue; }
        if r.to as usize == hi && recent_arrivals.len() < 12 { recent_arrivals.push(mk_recent(r, r.from)); }
        if r.from as usize == hi && recent_departures.len() < 12 { recent_departures.push(mk_recent(r, r.to)); }
        if recent_arrivals.len() >= 12 && recent_departures.len() >= 12 { break; }
    }
    let bought = hub.import_spend;
    let sold = hub.export_earn;
    // ── Estates & manufactories in this city's hinterland ──
    let estates_here: Vec<EstateRow> = sim.hubs.iter()
        .filter(|e| e.is_estate && e.parent == hi as i32)
        .map(|e| {
            let g = e.base_per_capita.iter().enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i).unwrap_or(0);
            let (owner, owner_is_guild) = if e.owner_house >= 0 {
                let h = sim.houses.get(e.owner_house as usize);
                (h.map(|x| x.name.clone()).unwrap_or_default(), h.map(|x| x.is_guild).unwrap_or(false))
            } else {
                (format!("City of {}", hub.name), false)
            };
            EstateRow {
                hub: e.id,
                name: e.name.clone(),
                kind: e.estate_kind,
                good: sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default(),
                output: e.production.get(g).copied().unwrap_or(0.0),
                owner,
                owner_is_guild,
                owner_is_civic: e.owner_house < 0,
                tier: e.estate_tier.max(1),
                damage: e.damage,
            }
        })
        .collect();

    // ── THE VESSEL REGISTRY ──────────────────────────────────────────────────
    // "How many ships and caravans are here" has no literal answer: a vessel is
    // not an entity (`fleet_*` are three counters on `House`, no identity, no
    // position — `docs/MERCHANT_VESSELS_AND_INFORMATION_PLAN.md`). What IS real
    // and worth a reader's time is the REGISTRY: the hulls of the houses and
    // guilds seated at this city, and how many of those slots are carrying
    // something right now.
    //
    // `away` uses the identical accounting `dispatch` runs when it builds
    // `cap_sea`/`cap_land` — one in-flight `InTransit` leg occupies exactly one
    // slot of its mode, whatever its quantity. No new state, no new pass, and
    // nothing claimed that the model cannot support.
    let vessels = {
        let seated: Vec<usize> = sim.houses.iter().enumerate()
            .filter(|(_, h)| !h.defunct && h.hub as usize == hi)
            .map(|(i, _)| i).collect();
        let (mut reg_sea, mut reg_riv, mut reg_car) = (0u32, 0u32, 0u32);
        for &i in &seated {
            let h = &sim.houses[i];
            reg_sea += h.fleet_sea;
            reg_riv += h.fleet_river;
            reg_car += h.fleet_caravan;
        }
        // Legs flown on a SEATED house's account, by mode. `InTransit.river`
        // distinguishes a barge leg from a caravan one; it is meaningless when
        // `sea` is set, exactly as the sim documents.
        let (mut away_sea, mut away_riv, mut away_car) = (0u32, 0u32, 0u32);
        let (mut inbound, mut outbound, mut soonest) = (0u32, 0u32, u32::MAX);
        for c in &sim.in_transit {
            if c.to as usize == hi {
                inbound += 1;
                soonest = soonest.min(c.eta_tick.saturating_sub(sim.tick));
            }
            if c.from as usize == hi { outbound += 1; }
            if c.owner >= 0 && seated.contains(&(c.owner as usize)) {
                if c.sea { away_sea += 1; } else if c.river { away_riv += 1; } else { away_car += 1; }
            }
        }
        let cls = |kind: &str, reg: u32, away: u32| VesselClass {
            kind: kind.into(), registered: reg, away, idle: reg.saturating_sub(away),
        };
        CityVessels {
            classes: vec![
                cls("sea", reg_sea, away_sea),
                cls("river", reg_riv, away_riv),
                cls("caravan", reg_car, away_car),
            ],
            houses: seated.len() as u32,
            inbound_cargoes: inbound,
            inbound_eta: if soonest == u32::MAX { 0 } else { soonest },
            outbound_cargoes: outbound,
            // `dispatch` pools river + caravan into one `cap_land`, so a barge leg
            // can be flying on a caravan's slot. The land TOTAL is exact; the
            // split between the two land classes is indicative, and the UI says so.
            land_pooled: true,
        }
    };
    // Journal stores the hub INDEX (not id), so filter by the resolved index.
    let events: Vec<JournalEntry> = sim
        .journal
        .iter()
        .filter(|e| e.hub == hi as i32 && e.kind != "price" && e.kind != "voyage_loss")
        .cloned()
        .collect();
    let (pop_house, pop_local, pop_guild) = crate::sim::tick::merchant_pops(hub);
    // Social strata (None for estates / unseeded hubs where the shares are blank).
    let so = &hub.society;
    let society = if !hub.is_estate && (so.patrician + so.burgher + so.commoner + so.underclass) > 1e-3 {
        let welfare = (so.commoner_wealth / (so.commoner_wealth + 1.5)).clamp(0.0, 1.0);
        Some(SocietyBrief {
            patrician: so.patrician, burgher: so.burgher, commoner: so.commoner, underclass: so.underclass,
            commoner_wealth: so.commoner_wealth, inequality: so.inequality, welfare, unrest: so.unrest,
        })
    } else { None };
    // Estate descriptors for the inspector (kind, owner, worked good).
    let (estate_owner, estate_good) = if hub.is_estate {
        let owner = if hub.owner_house >= 0 {
            sim.houses.get(hub.owner_house as usize).map(|h| h.name.clone()).unwrap_or_default()
        } else if hub.parent >= 0 {
            sim.hubs.get(hub.parent as usize).map(|h| format!("City of {}", h.name)).unwrap_or_default()
        } else { String::new() };
        // The worked good = the one this estate actually produces (max per-capita).
        let g = hub.base_per_capita.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i).unwrap_or(0);
        (owner, sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default())
    } else { (String::new(), String::new()) };
    // ── Foreign offices hosted here ──────────────────────────────────────────
    // Live throughput touching this hub, split by the owning holder, + the goods
    // each currently moves through here. Only holders with an OFFICE here listed.
    let mut hub_total = 0.0f32;
    let mut per_holder: std::collections::HashMap<usize, (f32, std::collections::HashSet<usize>)> =
        std::collections::HashMap::new();
    for s in &sim.in_transit {
        if s.from as usize == hi || s.to as usize == hi {
            hub_total += s.amount.max(0.0);
            if s.owner >= 0 {
                let e = per_holder.entry(s.owner as usize).or_default();
                e.0 += s.amount.max(0.0);
                e.1.insert(s.good);
            }
        }
    }
    let offices_here: Vec<OfficeHere> = sim.houses.iter().enumerate()
        .filter(|(_, h)| !h.defunct && h.offices.contains(&(hi as u32)))
        .map(|(idx, h)| {
            let (vol, gset) = per_holder.get(&idx).cloned().unwrap_or((0.0, Default::default()));
            OfficeHere {
                holder: h.name.clone(),
                color: distinct_color(idx),
                is_guild: h.is_guild,
                origin: sim.hubs.get(h.hub as usize).map(|x| x.name.clone()).unwrap_or_default(),
                throughput_pct: if hub_total > 1e-6 { vol / hub_total * 100.0 } else { 0.0 },
                goods: gset.iter()
                    .filter_map(|&g| sim.goods.get(g).map(|x| x.name.clone()))
                    .collect(),
            }
        })
        .collect();
    // ── DLC 3 · the polis government of this seat (non-estate settlements only) ──
    let government = if hub.is_estate {
        None
    } else {
        use crate::sim::tick::{archetype_label, office_title, govt_type_name, govt_head_title,
            EXPORT_TAX_RATE, IMPORT_TAX_RATE, OFFICIAL_CAPTURE};
        let ci = hub.council_house;
        let (council, council_color, council_archetype, council_is_guild, council_power) =
            if ci >= 0 {
                if let Some(house) = sim.houses.get(ci as usize) {
                    (house.name.clone(), distinct_color(ci as usize),
                     archetype_label(house.archetype).to_string(), house.is_guild, house.political_power)
                } else { ("—".into(), "#7a8aa0".into(), String::new(), false, 0.0) }
            } else { ("—".into(), "#7a8aa0".into(), String::new(), false, 0.0) };
        let tariff_default = hub.tariff_export <= 0.0 && hub.tariff_import <= 0.0;
        let tariff_export = if hub.tariff_export > 0.0 { hub.tariff_export } else { EXPORT_TAX_RATE };
        let tariff_import = if hub.tariff_import > 0.0 { hub.tariff_import } else { IMPORT_TAX_RATE };
        let spec = sim.spec_centers.iter().find(|c| c.hub == hub.id);
        let (spec_risk, spec_tier, spec_stars, spec_pattern, spec_drivers, spec_watch) = match spec {
            Some(c) => (c.risk, c.tier.clone(), c.stars, c.pattern_tag.clone(),
                c.drivers.iter().map(|d| d.detail.clone()).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                c.watch_goods.clone()),
            None => (0.0, String::new(), 0, String::new(), vec![], vec![]),
        };
        // Key figures with their allegiance status.
        let hname = |hi: i32| -> (String, String) {
            if hi >= 0 { sim.houses.get(hi as usize)
                .map(|h| (h.name.clone(), distinct_color(hi as usize)))
                .unwrap_or_default() } else { (String::new(), String::new()) }
        };
        let officials: Vec<OfficialRow> = hub.officials.iter().map(|o| {
            let (an, ac) = hname(o.house);
            let status = if o.kin { "kin" } else if o.house < 0 { "neutral" }
                else if o.control >= OFFICIAL_CAPTURE { "controlled" } else { "leaning" };
            let role = if o.role == 0 { govt_head_title(hub.govt_type).to_string() }
                else { office_title(o.role).to_string() };
            OfficialRow { role, name: o.name.clone(), allegiance: an, allegiance_color: ac,
                control: o.control, status: status.to_string() }
        }).collect();
        // Family influence over the government: controlled-figure weight (×3) + council
        // seat (×2) + commercial influence here, normalised across houses to a %.
        let mut infl: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for o in &hub.officials {
            if o.house >= 0 && (o.kin || o.control >= OFFICIAL_CAPTURE) {
                let w = match o.role { 0 => 2.0, 1 => 1.4, _ => 1.0 };
                *infl.entry(o.house as usize).or_insert(0.0) += 3.0 * w;
            }
        }
        if ci >= 0 { *infl.entry(ci as usize).or_insert(0.0) += 2.0; }
        for (hidx, house) in sim.houses.iter().enumerate() {
            if house.defunct || house.is_guild { continue; }
            if let Some((_, v)) = house.influence.iter().find(|(c, _)| *c == hub.id
                || sim.hubs.get(*c as usize).map(|hh| hh.id == hub.id).unwrap_or(false)) {
                if *v > 0.02 { *infl.entry(hidx).or_insert(0.0) += *v; }
            }
        }
        let tot_infl: f32 = infl.values().sum::<f32>().max(1e-6);
        let mut family_influence: Vec<InfluenceRow> = infl.iter().map(|(&hi, &v)| InfluenceRow {
            name: sim.houses.get(hi).map(|h| h.name.clone()).unwrap_or_default(),
            color: distinct_color(hi), pct: v / tot_infl,
        }).filter(|r| !r.name.is_empty() && r.pct >= 0.01).collect();
        family_influence.sort_by(|a, b| b.pct.partial_cmp(&a.pct).unwrap_or(std::cmp::Ordering::Equal));
        family_influence.truncate(8);
        // Enacted-law log (newest first), rendered to text.
        let mut laws: Vec<LawRow> = hub.laws.iter().rev().take(8).map(|l| {
            let hn = hname(l.house).0;
            let gn = if l.good >= 0 { sim.goods.get(l.good as usize).map(|g| g.name.clone()).unwrap_or_default() } else { String::new() };
            let text = match l.kind {
                0 => format!("Favoured-house charter → {}", if hn.is_empty() { "a family".into() } else { hn }),
                1 => "Protectionist tariff raised".to_string(),
                2 => "Free-trade tariff cut".to_string(),
                3 => "The coin is debased".to_string(),
                4 => "Grain law — civic granary stocked".to_string(),
                5 => format!("Guild monopoly on {}", gn),
                _ => "A decree is issued".to_string(),
            };
            LawRow { year: l.year, text }
        }).collect();
        laws.retain(|l| !l.text.is_empty());
        // Government stores it holds (top few goods by amount).
        let mut civic_goods: Vec<CivicGoodRow> = hub.civic_goods.iter().enumerate()
            .filter(|(_, &a)| a > 0.5)
            .filter_map(|(g, &a)| sim.goods.get(g).map(|gd| CivicGoodRow { name: gd.name.clone(), amount: a }))
            .collect();
        civic_goods.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        civic_goods.truncate(6);
        // Years until the soonest seat turns over.
        let soonest = hub.officials.iter().map(|o| o.term_end).min().unwrap_or(sim.tick);
        let next_election_years = ((soonest.saturating_sub(sim.tick)) / 365) as i32;
        let (captor, captor_color) = hname(hub.captor_house);
        // §3.1 · the office as a person: captor (a majority CAPTURE) outranks a
        // merely-dominant council, since it's the stronger real hold on the seat.
        let leader_house = if hub.captor_house >= 0 { hub.captor_house } else { hub.council_house };
        let leader = leader_house.try_into().ok()
            .and_then(|hi: usize| sim.houses.get(hi).map(|h| (hi, h)))
            .filter(|(_, h)| !h.defunct)
            .and_then(|(hi, h)| h.kin.first().map(|head| CityLeader {
                house: h.name.clone(), house_color: distinct_color(hi), is_guild: h.is_guild,
                is_captor: hub.captor_house == hi as i32,
                head_name: head.name.clone(), female: head.female,
                character_phrase: crate::sim::tick::character_phrase(head.character),
                vice: crate::sim::tick::vice_label(sim.head_vice(hi)).to_string(),
            }));
        Some(Government {
            council, council_color, council_archetype, council_is_guild, council_power,
            tariff_export, tariff_import, tariff_default,
            mint_fineness: if hub.mint_fineness <= 0.0 { 1.0 } else { hub.mint_fineness },
            treasury: hub.treasury, civic_pool: hub.civic_pool,
            spec_risk, spec_tier, spec_stars, spec_pattern, spec_drivers, spec_watch,
            govt_type: govt_type_name(hub.govt_type).to_string(),
            next_election_years, captor, captor_color,
            officials, family_influence, laws, civic_goods, leader,
        })
    };
    // ── DLC 3.5 · Carrying trade ("transit"): in-flight shipments run by THIS
    //    city's resident merchants (or via an office here) between two OTHER
    //    cities — the entrepôt handling-trade. Settled in a reserve coin if either
    //    endpoint mints one, else barter (priced in wheat-equivalent). ──
    let reserve_coin = |idx: usize| -> Option<String> {
        sim.hubs.get(idx).filter(|c| !c.coin_name.is_empty() && c.coin_trust >= 0.55)
            .map(|c| c.coin_name.clone())
    };
    let mut transit: Vec<TransitRow> = sim.in_transit.iter().filter_map(|s| {
        if s.owner < 0 { return None; }
        let oi = s.owner as usize;
        let house = sim.houses.get(oi)?;
        let resident = house.hub as usize == hi || house.offices.contains(&(hi as u32));
        let (from, to) = (s.from as usize, s.to as usize);
        if !resident || from == hi || to == hi { return None; }
        let price = sim.hubs.get(to).map(|c| c.price.get(s.good).copied().unwrap_or(0.0)).unwrap_or(0.0);
        let coin = reserve_coin(to).or_else(|| reserve_coin(from)).unwrap_or_default();
        let barter = if coin.is_empty() { format!("~{:.1} wheat/unit", price.max(0.0)) } else { String::new() };
        Some(TransitRow {
            merchant: house.name.clone(), is_guild: house.is_guild, color: distinct_color(oi),
            good: sim.goods.get(s.good).map(|g| g.name.clone()).unwrap_or_default(),
            amount: s.amount, value: s.amount * price,
            from_name: sim.hubs.get(from).map(|c| c.name.clone()).unwrap_or_default(),
            to_name: sim.hubs.get(to).map(|c| c.name.clone()).unwrap_or_default(),
            sea: s.sea, coin, barter,
        })
    }).collect();
    transit.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    transit.truncate(20);
    let war_with = if hub.war_with >= 0 {
        sim.hubs.get(hub.war_with as usize).map(|c| c.name.clone()).unwrap_or_default()
    } else { String::new() };
    let coin_value = if hub.coin_name.is_empty() { 0.0 }
        else { crate::sim::tick::coin_value(hub.mint_fineness, hub.coin_trust) };
    // Currency basket: which coins circulate here + their share (main coin first).
    let coin_basket: Vec<CoinShare> = hub.coin_basket.iter().filter_map(|&(k, share)| {
        let c = sim.hubs.get(k as usize)?;
        if c.coin_name.is_empty() { return None; }
        Some(CoinShare {
            coin_name: c.coin_name.clone(),
            share,
            main: hub.settle_coin == k as i32,
            reserve: hub.settle_coin != k as i32 && c.coin_trust >= 0.55,
        })
    }).collect();

    // ── City STORES: the civic warehouse + all goods physically at the city, valued in
    //    the grain-eq numeraire (base_value), so the panel/council can read supplies + riches.
    let city_stores = {
        let val = |g: usize| sim.goods.get(g).map(|gd| gd.base_value.max(0.0)).unwrap_or(1.0);
        // Civic (city-owned) reserve.
        let mut reserve: Vec<CivicGoodRow> = hub.civic_goods.iter().enumerate()
            .filter(|(_, &a)| a > 0.5)
            .filter_map(|(g, &a)| sim.goods.get(g).map(|gd| CivicGoodRow { name: gd.name.clone(), amount: a }))
            .collect();
        reserve.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        let reserve_value: f32 = hub.civic_goods.iter().enumerate()
            .map(|(g, &a)| a.max(0.0) * val(g)).sum();
        reserve.truncate(8);
        let food_reserve: f32 = (0..ng)
            .filter(|&g| sim.goods.get(g).map(|gd| gd.food).unwrap_or(false))
            .map(|g| hub.civic_goods.get(g).copied().unwrap_or(0.0).max(0.0)).sum::<f32>()
            + hub.reserve_food.max(0.0);
        // ALL goods held at the city = local merchant pool + every house/guild depot here.
        let mut held: Vec<f32> = (0..ng).map(|g| crate::sim::tick::stock_of(&hub.stock, g)).collect();
        for w in &sim.warehouses {
            if w.hub as usize == hi {
                for g in 0..ng.min(w.stock.len()) { held[g] += w.stock[g].max(0.0); }
            }
        }
        let goods_value: f32 = (0..ng).map(|g| held[g].max(0.0) * val(g)).sum();
        let goods_units: f32 = held.iter().map(|q| q.max(0.0)).sum();
        let mut top_goods: Vec<(CivicGoodRow, f32)> = (0..ng)
            .filter(|&g| held[g] > 0.5)
            .filter_map(|g| sim.goods.get(g)
                .map(|gd| (CivicGoodRow { name: gd.name.clone(), amount: held[g] }, held[g] * val(g))))
            .collect();
        top_goods.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_goods: Vec<CivicGoodRow> = top_goods.into_iter().take(8).map(|(r, _)| r).collect();
        CityStores { reserve, reserve_value, food_reserve, top_goods, goods_value, goods_units }
    };
    // Prefer the PERSISTED (hysteresis) tier; fall back to a live compute for older saves
    // or a hub not yet classified this run.
    let dev_tier = match sim.dev_tier.get(hi).copied() {
        Some(t) if t > 0 => t,
        _ => sim.development_tier(hi),
    };
    Ok(Some(HubDetail {
        id: hub.id,
        name: hub.name.clone(),
        x: hub.x,
        y: hub.y,
        population: hub.population.max(0.0) as u32,
        koppen: hub.koppen,
        coastal: hub.coastal,
        is_estate: hub.is_estate,
        mood: hub.mood,
        sent_food: hub.sent_food,
        sent_prosperity: hub.sent_prosperity,
        sent_stability: hub.sent_stability,
        grain_wealth: hub.grain_wealth,
        trade_wealth: hub.trade_wealth,
        food_balance: hub.food_balance,
        starving: hub.starving,
        goods,
        history: hub.history.clone(),
        events,
        houses: build_house_briefs(&sim).into_iter()
            .filter(|hb| hb.home_hub == hub.id)
            .collect(),
        in_by_sea: hub.in_by_sea,
        in_by_land: hub.in_by_land,
        lack_basic: hub.lack_basic,
        lack_comfort: hub.lack_comfort,
        lack_luxury: hub.lack_luxury,
        pop_house,
        pop_local,
        pop_guild,
        society,
        estate_kind: hub.estate_kind,
        estate_owner,
        estate_good,
        offices_here,
        arrivals,
        departures,
        recent_arrivals,
        recent_departures,
        bought,
        sold,
        estates_here,
        structures: hub.structures.iter().map(|&s| (
            crate::sim::tick::structure_label(s).to_string(),
            crate::sim::tick::structure_effect(s).to_string(),
        )).collect(),
        buildings: {
            // Resident merchant families here, richest first — COMMERCIAL buildings
            // (shipyard/guildhall/workshop) are attributed to them round-robin, so a
            // multi-house city reads as a patchwork of owner colours; civic works
            // (granary/warehouse) stay the commons' slate. Resolved live, so tiles
            // recolour as houses rise/fall.
            let mut residents: Vec<(usize, f32)> = sim.houses.iter().enumerate()
                .filter(|(_, hh)| !hh.defunct && hh.hub as usize == hi)
                .map(|(idx, hh)| (idx, hh.wealth))
                .collect();
            residents.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let mut out: Vec<BuildingInfo> = Vec::new();
            let mut comm = 0usize;
            for &s in &hub.structures {
                let label = crate::sim::tick::structure_label(s).to_string();
                let effect = crate::sim::tick::structure_effect(s).to_string();
                let civic = matches!(s, 1 | 2); // Granary / Warehouse
                let (owner, owner_kind, color) = if civic || residents.is_empty() {
                    ("Civic".to_string(), "civic".to_string(), CIVIC_TINT.to_string())
                } else {
                    let (hidx, _) = residents[comm % residents.len()];
                    comm += 1;
                    (sim.houses[hidx].name.clone(), "house".to_string(), distinct_color(hidx))
                };
                out.push(BuildingInfo { emoji: building_emoji(&label).to_string(), label, effect, owner, owner_kind, color });
            }
            // A FONDACO per significant diaspora quarter — a foreign people's own
            // trade house, tinted by that people's hearth colour.
            for (people, share) in sim.hub_people_display(hi).1 {
                if share < 0.05 { continue; }
                let color = crate::sim::cultures::color_of_people(&people)
                    .map(rgb_hex).unwrap_or_else(|| CIVIC_TINT.to_string());
                out.push(BuildingInfo {
                    emoji: building_emoji("Fondaco").to_string(),
                    label: "Fondaco".to_string(),
                    effect: format!("{} quarter · {:.0}% of the city", people, share * 100.0),
                    owner: people, owner_kind: "fondaco".to_string(), color,
                });
            }
            out
        },
        patron: sim.hub_patron.get(hi).copied().filter(|&p| p >= 0)
            .and_then(|p| sim.houses.get(p as usize)).map(|h| h.name.clone()).unwrap_or_default(),
        culture: sim.hub_people_display(hi).0,
        minorities: sim.hub_people_display(hi).1,
        culture_moods: {
            // How content is each resident people here — are its prized goods well-supplied?
            let name_to_g: std::collections::HashMap<&str, usize> =
                sim.goods.iter().enumerate().map(|(g, tg)| (tg.name.as_str(), g)).collect();
            let mut present: Vec<(String, f32)> = Vec::new();
            let minsum: f32 = sim.hub_minorities.get(hi).map(|m| m.iter().map(|(_, s)| *s).sum()).unwrap_or(0.0);
            let maj = sim.hub_culture.get(hi).cloned().unwrap_or_default();
            if !maj.is_empty() && maj != "—" { present.push((maj, (1.0 - minsum).clamp(0.0, 1.0))); }
            if let Some(mins) = sim.hub_minorities.get(hi) {
                for (c, s) in mins { if *s > 0.02 { present.push((c.clone(), *s)); } }
            }
            present.into_iter().map(|(name, share)| {
                let desired = sim.culture_desired_goods(&name);
                let (mut met, mut unmet) = (Vec::new(), Vec::new());
                let (mut sum, mut cnt) = (0.0f32, 0u32);
                for nm in &desired {
                    if let Some(&g) = name_to_g.get(nm) {
                        let base = sim.goods[g].base_value.max(0.01);
                        let price = hub.price.get(g).copied().unwrap_or(base).max(0.01);
                        let avail = (base * 1.3 / price).clamp(0.0, 1.0);
                        sum += avail; cnt += 1;
                        if avail >= 0.6 { met.push((*nm).to_string()); }
                        else if avail < 0.4 { unmet.push((*nm).to_string()); }
                    }
                }
                let satisfaction = if cnt > 0 { sum / cnt as f32 } else { 0.5 };
                CultureMood { color: culture_color(&name), name, share, satisfaction, met, unmet }
            }).collect()
        },
        government,
        public_health: hub.public_health,
        satellites: sim.hinterland.iter()
            .filter(|v| v.parent_hub == hi as i32)
            .map(|v| HinterlandVillage { name: v.name.clone(), population: v.population.round().max(0.0) as u32, x: v.x, y: v.y })
            .collect(),
        treasury: hub.treasury,
        finance: Some(hub.finance.clone()),
        war_with,
        coin_name: hub.coin_name.clone(),
        coin_trust: hub.coin_trust,
        coin_value,
        coin_basket,
        transit,
        stolen_good: if hub.stolen_good >= 0 {
            sim.goods.get(hub.stolen_good as usize).map(|g| g.name.clone()).unwrap_or_default()
        } else { String::new() },
        stolen_from: if hub.stolen_from >= 0 {
            sim.hubs.iter().find(|h| h.id == hub.stolen_from as u32).map(|h| h.name.clone()).unwrap_or_default()
        } else { String::new() },
        // Colonies/outposts this city founded (founder_hub is a hub INDEX == hi).
        related_colonies: (0..sim.hubs.len())
            .filter(|&ci| sim.hubs[ci].colony_kind != 0 && sim.hubs[ci].founder_hub == hi as i32)
            .map(|ci| colony_summary(&sim, ci))
            .collect(),
        city_stores,
        vessels,
        dev_tier,
    }))
}


/// All merchant families (active first, richest first) for the Houses panel.
#[tauri::command]
pub fn campaign_get_houses(db: State<'_, WorldDb>) -> Result<Vec<HouseBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Needed for the seat's culture kit (the dossier's culture-dress figure, Phase 1.2).
    crate::sim::cultures::ensure_active(&conn);
    Ok(match get_sim(&db, &conn)? {
        Some(sim) => build_house_briefs(&sim),
        None => vec![],
    })
}


/// The yearly T-account ledger for one house/guild (Accountant view).
#[tauri::command]
pub fn campaign_house_ledger(db: State<'_, WorldDb>, house: usize) -> Result<Option<HouseLedger>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None); };
    if house >= sim.houses.len() {
        return Ok(None);
    }
    let h = &sim.houses[house];
    // Prefer the last COMPLETED year; fall back to the running year early on.
    let prev_ok = sim
        .house_ledger_prev
        .get(house)
        .map(|l| l.year > 0 || !l.trade_profit_by_city.is_empty());
    let led = if prev_ok == Some(true) {
        &sim.house_ledger_prev[house]
    } else if let Some(l) = sim.house_ledger.get(house) {
        l
    } else {
        return Ok(None);
    };
    let city_name = |idx: u32| sim.hubs.get(idx as usize).map(|hb| hb.name.clone()).unwrap_or_default();
    let to_lines = |v: &Vec<(u32, f32)>| -> Vec<LedgerLine> {
        let mut out: Vec<LedgerLine> = v
            .iter()
            .map(|&(c, a)| LedgerLine { label: city_name(c), amount: a })
            .collect();
        out.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        out
    };
    let trade_profit = to_lines(&led.trade_profit_by_city);
    let import_tax = to_lines(&led.import_tax_by_city);
    let export_tax = to_lines(&led.export_tax_by_city);
    let income_total = trade_profit.iter().map(|l| l.amount).sum::<f32>() + led.office_income + led.estate_income;
    let tax_total = import_tax.iter().map(|l| l.amount).sum::<f32>() + export_tax.iter().map(|l| l.amount).sum::<f32>();
    let expense_total = tax_total
        + led.estate_tax
        + led.upkeep
        + led.fleet_cost
        + led.lost_cargo
        + led.events
        + led.consumption
        + led.inflation
        + led.war_levy
        + led.war_damage;
    // Warehouse = the home city's stored goods (what a warehouse fire destroys).
    let (warehouse, warehouse_city) = match sim.hubs.get(h.hub as usize) {
        Some(hb) => {
            let mut w: Vec<LedgerLine> = (0..sim.goods.len())
                .map(|g| (g, crate::sim::tick::stock_of(&hb.stock, g)))
                .filter(|&(_, s)| s > 0.5)
                .map(|(g, s)| LedgerLine {
                    label: sim.goods.get(g).map(|gg| gg.name.clone()).unwrap_or_default(),
                    amount: s,
                })
                .collect();
            w.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
            w.truncate(10);
            (w, hb.name.clone())
        }
        None => (vec![], String::new()),
    };
    Ok(Some(HouseLedger {
        name: h.name.clone(),
        is_guild: h.is_guild,
        year: led.year,
        trade_profit,
        office_income: led.office_income,
        estate_income: led.estate_income,
        income_total,
        import_tax,
        export_tax,
        estate_tax: led.estate_tax,
        upkeep: led.upkeep,
        fleet_cost: led.fleet_cost,
        lost_cargo: led.lost_cargo,
        events: led.events,
        consumption: led.consumption,
        inflation: led.inflation,
        war_levy: led.war_levy,
        war_damage: led.war_damage,
        expense_total,
        net: income_total - expense_total,
        wealth_graph: led.wealth_samples.clone(),
        wealth_years: {
            // Last 10 YEARLY wealth samples (oldest→newest) for the growth graph.
            let wh = &sim.houses[house].wealth_history;
            let n = wh.len().min(10);
            wh[wh.len() - n..].to_vec()
        },
        wealth_start_year: {
            let wh = &sim.houses[house].wealth_history;
            let n = wh.len().min(10) as u32;
            (sim.tick / 365).saturating_sub(n.saturating_sub(1))
        },
        warehouse_city,
        warehouse,
    }))
}


/// The timeline / chronicle of one house, looked up by name.
#[tauri::command]
pub fn campaign_get_house_history(name: String, db: State<'_, WorldDb>) -> Result<Option<HouseHistory>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None) };
    let idx = sim.houses.iter().position(|h| h.name == name);
    let Some(idx) = idx else { return Ok(None) };
    let h = &sim.houses[idx];
    let year = |t: u32| t / 365;
    let events: Vec<HouseTimelineEvent> = h.events.iter()
        .filter(|e| e.kind != "voyage_loss") // hide shipwreck/ambush noise from the family chronicle
        .map(|e| HouseTimelineEvent {
            year: year(e.tick), kind: e.kind.clone(), text: e.text.clone(),
        }).collect();
    let mut top_goods: Vec<(String, f32)> = h.good_profit.iter().enumerate()
        .filter(|(_, &p)| p > 0.0)
        .map(|(g, &p)| (sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default(), p))
        .collect();
    top_goods.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    top_goods.truncate(5);
    let founder = h.events.iter().find(|e| e.kind == "founded")
        .map(|e| e.text.clone()).unwrap_or_default();
    // Colonies this house owns (outposts, owner_house) or backed (kind-1 joint-stock).
    let colonies: Vec<ColonySummary> = (0..sim.hubs.len()).filter(|&ci| {
        let c = &sim.hubs[ci];
        c.colony_kind != 0 && (c.owner_house == idx as i32
            || c.backers.iter().any(|(k, i, _)| *k == 1 && *i == idx as u32))
    }).map(|ci| colony_summary(&sim, ci)).collect();
    let line: Vec<HeadBrief> = h.line.iter().map(|p| HeadBrief {
        name: p.name.clone(), female: p.female, generation: p.generation,
        since_year: year(p.since), until_year: if p.until > 0 { year(p.until) } else { 0 },
        age_at_accession: p.age_at_accession, age_at_death: p.age_at_death,
        wealth_start: p.wealth_start, wealth_end: p.wealth_end,
        accession: p.accession.clone(), epithet: p.epithet.clone(),
    }).collect();
    // #8 · principal stone for this house's generic "gemstones" trade — same
    // GEM_STONES source + home-hub hash the HouseBrief uses, so both views agree.
    let gem_variety = {
        let gi = sim.goods.iter().position(|g| g.name == "gemstones");
        let deals = gi.map_or(false, |g|
            h.good_profit.get(g).copied().unwrap_or(0.0) > 0.0 || h.spec.contains(&g));
        match (deals, sim.hubs.get(h.hub as usize)) {
            (true, Some(hub)) => {
                let hh = (hub.x as i64).wrapping_mul(73856093) ^ (hub.y as i64).wrapping_mul(19349663);
                crate::sim::biological::GEM_STONES
                    [(hh.unsigned_abs() as usize) % crate::sim::biological::GEM_STONES.len()].to_string()
            }
            _ => String::new(),
        }
    };
    Ok(Some(HouseHistory {
        name: h.name.clone(),
        color: distinct_color(idx),
        founder,
        founded_year: year(h.founded_tick),
        events,
        top_goods,
        defunct: h.defunct,
        gem_variety,
        colonies,
        line,
    }))
}
