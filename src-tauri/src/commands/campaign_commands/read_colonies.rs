//! read_colonies commands — split from the former monolithic campaign_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Colony detail for a settlement colony (None for non-colony hubs).
#[tauri::command]
pub fn campaign_get_colony(id: u32, db: State<'_, WorldDb>) -> Result<Option<ColonyDetail>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = match sim.hubs.iter().position(|h| h.id == id) { Some(i) => i, None => return Ok(None) };
    let hub = &sim.hubs[hi];
    if hub.colony_kind != 1 { return Ok(None); }
    let founder_name = (hub.founder_hub >= 0).then(|| sim.hubs.get(hub.founder_hub as usize))
        .flatten().map(|h| h.name.clone()).unwrap_or_default();
    let main_bank_name = (hub.main_bank >= 0).then(|| sim.banks.get(hub.main_bank as usize))
        .flatten().map(|b| b.name.clone()).unwrap_or_default();
    let age_years = (sim.tick.saturating_sub(hub.colony_founded_tick) / 365) as u32;
    let indep_in_years = 70i32 - age_years as i32;
    let backers = hub.backers.iter().map(|&(kind, idx, share)| {
        let (name, color) = match kind {
            0 => (sim.hubs.get(idx as usize).map(|h| h.name.clone()).unwrap_or_default(), "#7fb8ff".to_string()),
            1 => (sim.houses.get(idx as usize).map(|h| h.name.clone()).unwrap_or_default(), distinct_color(idx as usize)),
            2 => {
                let b = sim.banks.get(idx as usize);
                (b.map(|b| b.name.clone()).unwrap_or_default(),
                 b.map(|b| distinct_color(b.house as usize)).unwrap_or_else(|| "#cccccc".into()))
            }
            _ => (String::new(), "#cccccc".to_string()),
        };
        ColonyBackerRow { kind, name, color, share }
    }).collect();
    let supply = sim.colony_supply.iter().filter(|s| s.colony_hub == hi as u32).map(|s| {
        ColonySupplyRow {
            category: s.category,
            supplier: sim.hubs.get(s.supplier_hub as usize).map(|h| h.name.clone()).unwrap_or_default(),
            good: sim.goods.get(s.good).map(|g| g.name.clone()).unwrap_or_default(),
            qty: s.monthly_qty,
        }
    }).collect();
    let supply_source = (hub.supply_source >= 0)
        .then(|| sim.hubs.get(hub.supply_source as usize)).flatten()
        .map(|h| h.name.clone()).unwrap_or_default();
    Ok(Some(ColonyDetail {
        stage: hub.colony_stage, autonomous: hub.autonomous, founder_name, main_bank_name,
        coin_name: hub.coin_name.clone(), charter_open: hub.colony_kind == 1 && !hub.autonomous,
        supply_years: hub.supply_years, reserve_food: hub.reserve_food, reserve_cap: hub.reserve_cap,
        age_years, indep_in_years, backers, supply,
        supply_ships: hub.supply_ships,
        supply_capacity: hub.supply_ships as f32 * crate::sim::tick::SUPPLY_SHIP_CAPACITY,
        supply_delivered: hub.supply_delivered,
        supply_source,
    }))
}


#[tauri::command]
pub fn campaign_get_satellite(id: u32, db: State<'_, WorldDb>) -> Result<Option<SatelliteBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = match sim.hubs.iter().position(|h| h.id == id) { Some(i) => i, None => return Ok(None) };
    let hub = &sim.hubs[hi];
    if hub.build_stage == 0 { return Ok(None); } // not (or no longer) a construction site
    let m = hub.founder_hub;
    let (metropolis, metropolis_id, fund) = if m >= 0 && (m as usize) < sim.hubs.len() {
        (sim.hubs[m as usize].name.clone(), m, sim.hubs[m as usize].treasury)
    } else { ("—".to_string(), -1, 0.0) };
    let role = match hub.colony_stage { 1 => "Granary", 0 => "Port", _ => "Workshop" }.to_string();
    let stage = hub.build_stage.min(5);
    let overall = (((stage.saturating_sub(1)) as f32) + hub.build_progress.clamp(0.0, 1.0)) / 5.0;
    let met = hub.build_supply.iter().cloned().fold(1.0f32, f32::min).clamp(0.0, 1.0);
    let months_left = ((5 - stage) as f32 + (1.0 - hub.build_progress).max(0.0))
        * crate::sim::tick::SAT_STAGE_MONTHS;
    let eta_years = months_left / met.max(0.08) / 12.0;
    let monthly_cost = crate::sim::tick::SAT_CONVOY_UPKEEP * hub.build_convoys as f32;
    let runway_months = if monthly_cost > 0.0 { fund / monthly_cost } else { 0.0 };
    let cats = ["Food", "Preservables", "Construction"];
    let supply = (0..3usize).map(|c| {
        let g = hub.build_supply_good[c] as usize;
        SatSupplyRow {
            category: cats[c].to_string(),
            good: sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_else(|| "—".into()),
            source: metropolis.clone(),
            rate: crate::sim::tick::SAT_STAGE_QUOTA,
            met: hub.build_supply[c].clamp(0.0, 1.0),
        }
    }).collect();
    // Future exploits = the goods this site will actually produce once finished.
    let mut gi: Vec<usize> = (0..sim.goods.len()).collect();
    gi.sort_by(|&a, &b| hub.base_per_capita.get(b).copied().unwrap_or(0.0)
        .partial_cmp(&hub.base_per_capita.get(a).copied().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    let exploits = gi.into_iter()
        .filter(|&g| hub.base_per_capita.get(g).copied().unwrap_or(0.0) > 0.0)
        .take(4).map(|g| sim.goods[g].name.clone()).collect();
    Ok(Some(SatelliteBrief {
        id, name: hub.name.clone(), metropolis, metropolis_id, role,
        stage, progress: hub.build_progress.clamp(0.0, 1.0), overall, eta_years,
        monthly_cost, fund, runway_months, convoys: hub.build_convoys,
        idle_months: hub.build_idle_months, founded_year: hub.build_start_tick as f32 / 365.0,
        supply, exploits,
    }))
}


#[tauri::command]
pub fn campaign_get_provisioning(id: u32, db: State<'_, WorldDb>) -> Result<Option<ProvisioningBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = match sim.hubs.iter().position(|h| h.id == id) { Some(i) => i, None => return Ok(None) };
    let hub = &sim.hubs[hi];
    if hub.is_estate || hub.abandoned { return Ok(None); }
    let deps = sim.hubs.iter().filter(|d| d.founder_hub == hi as i32 && !d.abandoned
        && (d.colony_kind == 1 || d.colony_kind == 3 || d.build_stage > 0)).count() as u32;
    // Real trade-share dominance: houses carrying ≥60% of the city's trade (or a captured
    // government) suspend the council's right of first buy.
    let tw_total = hub.tw_house + hub.tw_local + hub.tw_guild;
    let dominant_share = if tw_total > 1e-6 { hub.tw_house / tw_total } else { 0.0 };
    let first_buy = dominant_share < 0.60 && hub.captor_house < 0;
    let dominant_house = if hub.captor_house >= 0 {
        sim.houses.get(hub.captor_house as usize).map(|h| h.name.clone()).unwrap_or_default()
    } else { String::new() };
    let bought_month = sim.council_bought_month.get(hi).copied().unwrap_or(0.0);
    let reserve_target = crate::sim::tick::COUNCIL_RESERVE_BASE * (1.0 + deps as f32)
        * (hub.population / 5_000.0).clamp(0.3, 4.0);
    let mut goods: Vec<ProvGoodRow> = (0..sim.goods.len())
        .filter(|&g| hub.civic_goods.get(g).copied().unwrap_or(0.0) > 0.01
            || (sim.goods[g].food && (deps > 0 || hub.food_balance <= 0.15)))
        .map(|g| ProvGoodRow {
            good: sim.goods[g].name.clone(),
            secured: hub.civic_goods.get(g).copied().unwrap_or(0.0),
            target: reserve_target,
            food: sim.goods[g].food,
        })
        .collect();
    goods.sort_by(|a, b| b.secured.partial_cmp(&a.secured).unwrap_or(std::cmp::Ordering::Equal));
    goods.truncate(10);
    Ok(Some(ProvisioningBrief { first_buy, dominant_house, dominant_share, dependents: deps, reserve_target, bought_month, goods }))
}


#[tauri::command]
pub fn campaign_get_migration_routes(db: State<'_, WorldDb>) -> Result<Vec<MigrationRouteBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    Ok(sim.migration_routes.iter().map(|r| MigrationRouteBrief {
        path: r.path.clone(), culture: r.culture.clone(), volume: r.volume,
        from_hub: r.from_hub, to_hub: r.to_hub,
        age_years: sim.tick.saturating_sub(r.tick) as f32 / 365.0,
    }).collect())
}


/// Empire-wide colony roster (settlement colonies + house outposts) for the
/// Colonial Office subwindow. Grouped client-side by `founder_hub`.
#[tauri::command]
pub fn campaign_get_colonies(db: State<'_, WorldDb>) -> Result<Vec<ColonySummary>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out = Vec::new();
    for hi in 0..sim.hubs.len() {
        if sim.hubs[hi].colony_kind != 0 {
            out.push(colony_summary(&sim, hi));
        }
    }
    Ok(out)
}


#[tauri::command]
pub fn campaign_colony_gates(db: State<'_, WorldDb>) -> Result<Option<ColonyGateStatus>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let year = sim.tick / 365;
    let year_ok = year >= GATE_START_YEAR;
    // Best qualifying founder (mirrors maybe_found_settlement_colony's filter/score).
    let mut founder: Option<usize> = None;
    let mut best = 0.0f32;
    for h in 0..sim.hubs.len() {
        let hub = &sim.hubs[h];
        if hub.is_estate || hub.colony_kind != 0 { continue; }
        if hub.population < GATE_MIN_POP || hub.starving > GATE_STARVING_MAX { continue; }
        if hub.sent_prosperity < GATE_PROSPERITY_MIN { continue; }
        let score = hub.population * hub.sent_prosperity.clamp(0.0, 1.0) * (0.2 + hub.treasury);
        if score > best { best = score; founder = Some(h); }
    }
    let founder_ok = founder.is_some();
    let qualifying_founder = founder.map(|h| sim.hubs[h].name.clone()).unwrap_or_default();
    // A bank on the founder's continent (component) — the hard, dominant gate.
    let comp = founder.map(|h| sim.hubs[h].component);
    let bank_on_continent = sim.banks.iter().any(|b| !b.defunct
        && sim.hubs.get(b.seat as usize)
            .map(|s| comp.map_or(true, |c| s.component == c)).unwrap_or(false));
    // Reachable fertile unsettled sites within the colony hop reach of the founder.
    let cap = sim.world_w * GATE_HOP_REACH_FRAC;
    let colonizable_sites_in_range = if let Some(h) = founder {
        let (fx, fy) = (sim.hubs[h].x, sim.hubs[h].y);
        sim.colonizable.iter().filter(|s| {
            if s.fertility < GATE_FERTILE_SITE && s.trade_value < GATE_TRADE_SITE { return false; }
            let mut dx = (s.x - fx).abs();
            if sim.world_w > 0.0 { dx = dx.min(sim.world_w - dx); } // cylindrical wrap on X
            let dy = s.y - fy;
            (dx * dx + dy * dy).sqrt() <= cap
        }).count() as u32
    } else {
        sim.colonizable.iter()
            .filter(|s| s.fertility >= GATE_FERTILE_SITE || s.trade_value >= GATE_TRADE_SITE)
            .count() as u32
    };
    let site_ok = colonizable_sites_in_range > 0;
    let settlement_colonies = sim.hubs.iter()
        .filter(|h| h.colony_kind == 1 && !h.autonomous).count() as u32;
    let at_colony_cap = settlement_colonies >= GATE_MAX_SETTLEMENT_COLONIES;
    let blocking_gate = if at_colony_cap { "cap" }
        else if !year_ok { "year" }
        else if !founder_ok { "founder" }
        else if !bank_on_continent { "bank" }
        else if !site_ok { "site" }
        else { "none" }.to_string();
    Ok(Some(ColonyGateStatus {
        year, start_year: GATE_START_YEAR, year_ok,
        qualifying_founder, founder_ok, bank_on_continent,
        colonizable_sites_in_range, site_ok,
        settlement_colonies, max_settlement_colonies: GATE_MAX_SETTLEMENT_COLONIES, at_colony_cap,
        min_pop: GATE_MIN_POP, blocking_gate,
    }))
}


/// Phase 6 · the Landmarks & Sacred Sites panel: the world's places of note.
#[tauri::command]
pub fn campaign_get_landmarks(db: State<'_, WorldDb>) -> Result<Vec<LandmarkBrief>, String> {
    use crate::sim::tick::WONDER_NAMES;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<LandmarkBrief> = Vec::new();
    let good = |g: i32| if g >= 0 {
        sim.goods.get(g as usize).map(|x| x.name.clone()).unwrap_or_default()
    } else { String::new() };
    let mut push = |hub: u32, kind: &str, label: String, detail: String, out: &mut Vec<LandmarkBrief>| {
        if let Some(h) = sim.hubs.get(hub as usize) {
            out.push(LandmarkBrief { hub, x: h.x, y: h.y, city: h.name.clone(),
                kind: kind.to_string(), label, detail });
        }
    };
    for (hub, tier) in &sim.wonders {
        push(*hub, "wonder", WONDER_NAMES.get(*tier as usize).copied().unwrap_or("a wonder").to_string(),
            String::new(), &mut out);
    }
    for s in &sim.holy_sites {
        push(s.hub, "temple", if s.tier >= 2 { "Great holy city" } else { "Temple city" }.to_string(),
            if s.patron_good >= 0 { format!("patron: {}", good(s.patron_good)) } else { String::new() }, &mut out);
    }
    for f in &sim.fairs {
        push(f.hub, "fair", "Trade fair".to_string(), format!("opens month {}", f.month), &mut out);
    }
    for g in &sim.guilds {
        if g.hall {
            push(g.hub, "guildhall", format!("{} guildhall", good(g.good as i32)), String::new(), &mut out);
        }
    }
    Ok(out)
}
