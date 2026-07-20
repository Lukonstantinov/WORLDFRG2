//! flow commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// DLC 3.5 · the live campaign's DYNAMIC TRADE FLOW: last year's actual shipped
/// volume per hub-pair, routed over the SAME coarse cost grid as the static trunks
/// (so flows follow real roads/sea-lanes, not straight lines) and bundled per
/// coarse edge — width ∝ the year's volume on that segment, so the busiest arteries
/// read thickest. Recomputed once a year inside the tick; this just routes + bundles
/// the snapshot. Returns the same `TradeTrunk` shape the trunk overlay renders.
#[tauri::command]
pub fn campaign_get_trade_flow(
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TradeTrunk>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let sim = match crate::commands::campaign_commands::get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(vec![]),
    };
    if sim.flow_year.is_empty() { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h,
        &rivers_json, reach == 2, true, 0.0, -1, 12)?;

    // Map each real hub (by stable id) to its coarse node.
    let mut node_of: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for h in &sim.hubs {
        if h.is_estate { continue; }
        let cx = ((h.x.max(0.0) as u32) / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = ((h.y.max(0.0) as u32) / cc.f).min(cc.ch as u32 - 1) as i32;
        node_of.insert(h.id, cc.cidx(cx, cy));
    }
    // Route each pair and bundle its volume onto every coarse edge it traverses.
    let mut edge: std::collections::HashMap<(usize, usize), f32> = std::collections::HashMap::new();
    for &(a_id, b_id, vol) in &sim.flow_year {
        if vol <= 0.0 { continue; }
        let (s, g) = match (node_of.get(&a_id), node_of.get(&b_id)) {
            (Some(&s), Some(&g)) if s != g => (s, g),
            _ => continue,
        };
        let path = match coarse_dijkstra(&cc, s, g) { Some(p) => p, None => continue };
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }
        for w in path.windows(2) {
            let e = (w[0].min(w[1]), w[0].max(w[1]));
            *edge.entry(e).or_insert(0.0) += vol;
        }
    }
    let mut trunks: Vec<TradeTrunk> = edge.into_iter()
        .filter(|(_, v)| *v > 0.0)
        .map(|((a, b), v)| TradeTrunk {
            points: vec![cc.world_of(a), cc.world_of(b)], volume: v, good: -1, road: String::new(),
        })
        .collect();
    trunks.sort_by(|x, y| y.volume.partial_cmp(&x.volume).unwrap_or(std::cmp::Ordering::Equal));
    Ok(trunks)
}


#[tauri::command]
pub fn campaign_get_corridors(
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TradeCorridor>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let sim = match crate::commands::campaign_commands::get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(vec![]),
    };
    // Event-driven: corridors are the ESTABLISHED routes earned by expeditions
    // (docs/EXPEDITIONS_CORRIDORS_PLAN.md), not the top-N yearly flows. There are
    // only a handful, so routing them is cheap — this is what removes the year-tick
    // recompute storm the old flow_year source caused.
    if sim.corridors.is_empty() { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;
    // Same river/terrain-prone coarse grid the trade routes ride.
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h,
        &rivers_json, reach == 2, true, 0.0, -1, 12)?;
    let km_per_cell = KM_EQUATOR / grid_w as f32;

    // Hub id → (index, coarse node).
    let mut idx_of_id: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut node_of: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (i, h) in sim.hubs.iter().enumerate() {
        if h.is_estate { continue; }
        idx_of_id.insert(h.id, i);
        let cx = ((h.x.max(0.0) as u32) / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = ((h.y.max(0.0) as u32) / cc.f).min(cc.ch as u32 - 1) as i32;
        node_of.insert(h.id, cc.cidx(cx, cy));
    }
    // Strongest resident house per hub INDEX — the corridor's owner-from-home.
    let mut owner_of: std::collections::HashMap<usize, (usize, f32)> = std::collections::HashMap::new();
    for (hi, hh) in sim.houses.iter().enumerate() {
        if hh.defunct { continue; }
        let e = owner_of.entry(hh.hub as usize).or_insert((usize::MAX, f32::MIN));
        if hh.wealth > e.1 { *e = (hi, hh.wealth); }
    }

    // The established corridors (as id-pairs with a volume proxy = successful
    // ventures) — routed geographically below, strongest first.
    let mut flows: Vec<(u32, u32, f32)> = sim.corridors.iter().filter_map(|c| {
        let a = sim.hubs.get(c.a as usize)?;
        let b = sim.hubs.get(c.b as usize)?;
        Some((a.id, b.id, (c.successes.max(1)) as f32))
    }).collect();
    flows.sort_by(|x, y| y.2.partial_cmp(&x.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    let mut corridors: Vec<TradeCorridor> = Vec::new();
    for (a_id, b_id, vol) in flows {
        if vol <= 0.0 { continue; }
        if corridors.len() >= 24 { break; }
        let key = (a_id.min(b_id), a_id.max(b_id));
        if !seen.insert(key) { continue; }
        let (sa, sb) = match (node_of.get(&a_id), node_of.get(&b_id)) {
            (Some(&s), Some(&g)) if s != g => (s, g), _ => continue,
        };
        let path = match coarse_dijkstra(&cc, sa, sb) { Some(p) => p, None => continue };
        if path.len() < 6 { continue; }                                   // long hauls only
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; } // no ocean crossing
        let (km, days, _mode) = path_metrics(&cc, &path, km_per_cell);
        let (ai, bi) = match (idx_of_id.get(&a_id), idx_of_id.get(&b_id)) {
            (Some(&ai), Some(&bi)) => (ai, bi), _ => continue,
        };
        // Origin = the endpoint whose resident house is stronger (the backer).
        let (oa, ob) = (owner_of.get(&ai).copied(), owner_of.get(&bi).copied());
        let (origin_idx, dest_idx, owner) = match (oa, ob) {
            (Some(x), Some(y)) => if x.1 >= y.1 { (ai, bi, Some(x.0)) } else { (bi, ai, Some(y.0)) },
            (Some(x), None) => (ai, bi, Some(x.0)),
            (None, Some(y)) => (bi, ai, Some(y.0)),
            (None, None) => (ai, bi, None),
        };
        let (owner_name, color) = match owner {
            Some(hi) if hi < sim.houses.len() => (sim.houses[hi].name.clone(), house_hex(hi)),
            _ => ("civic".to_string(), "#7a8aa0".to_string()),
        };
        // The corridor carries the origin's chief export.
        let good = {
            let prod = &sim.hubs[origin_idx].production;
            let mut best = (usize::MAX, 0.0f32);
            for (g, &p) in prod.iter().enumerate() { if p > best.1 { best = (g, p); } }
            if best.0 != usize::MAX && best.0 < sim.goods.len() {
                sim.goods[best.0].name.clone()
            } else { String::new() }
        };
        // Waystations at regular intervals, typed by the cell's terrain; the whole
        // path's cells are tallied for the leg breakdown.
        let spacing = (path.len() / 6).max(2);
        let mut waystations: Vec<CorridorWaystation> = Vec::new();
        let (mut land_legs, mut river_legs, mut sea_legs) = (0u32, 0u32, 0u32);
        let last = path.len() - 1;
        for (k, &c) in path.iter().enumerate() {
            let kind = if !cc.is_land[c] { sea_legs += 1; 3u8 }
                else if cc.is_river[c] { river_legs += 1; 1 }
                else if cc.elev[c] > 0.62 { land_legs += 1; 4 }
                else { land_legs += 1; 2 };
            if k == 0 || k == last || k % spacing != 0 { continue; }
            let w = cc.world_of(c);
            waystations.push(CorridorWaystation { x: w[0], y: w[1], kind });
        }
        let points: Vec<[f32; 2]> = path.iter().map(|&c| cc.world_of(c)).collect();
        corridors.push(TradeCorridor {
            origin: sim.hubs[origin_idx].name.clone(),
            dest: sim.hubs[dest_idx].name.clone(),
            owner: owner_name, color, good, volume: vol,
            km, days, land_legs, river_legs, sea_legs,
            points, waystations,
        });
    }
    Ok(corridors)
}


#[tauri::command]
pub fn campaign_get_expeditions(db: State<'_, WorldDb>) -> Result<ExpeditionsPayload, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match crate::commands::campaign_commands::get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(ExpeditionsPayload { active: vec![], failed: vec![] }),
    };
    let mut active = Vec::new();
    for e in sim.expeditions.iter() {
        if e.status >= 3 { continue; } // en-route / arrived / returning only
        let t = e.pos.clamp(0.0, 1.0);
        let (sx, sy, gx, gy) = if e.outbound { (e.ox, e.oy, e.dx, e.dy) } else { (e.dx, e.dy, e.ox, e.oy) };
        let (x, y) = (sx + (gx - sx) * t, sy + (gy - sy) * t);
        let progress = if e.outbound { t * 0.5 } else { 0.5 + t * 0.5 };
        let origin = sim.hubs.get(e.origin as usize).map(|h| h.name.clone()).unwrap_or_default();
        let dest = sim.hubs.get(e.dest as usize).map(|h| h.name.clone()).unwrap_or_default();
        let good = sim.goods.get(e.good as usize).map(|g| g.name.clone()).unwrap_or_default();
        let hazards = e.hazards.iter().rev().take(6).map(|h| [h.x, h.y, h.kind as f32]).collect();
        active.push(ExpeditionView {
            id: e.id, leader: e.leader.clone(), origin, dest,
            x, y, ox: e.ox, oy: e.oy, dx: e.dx, dy: e.dy,
            progress, outbound: e.outbound, status: e.status,
            caravans: e.caravans, ships: e.ships, good,
            survived: e.arrived_frac, cost: e.cost, launched_year: e.launched_tick / 365, hazards,
        });
    }
    let failed = sim.failed_expeditions.iter().rev().take(40)
        .map(|h| ExpeditionFail { x: h.x, y: h.y, kind: h.kind }).collect();
    Ok(ExpeditionsPayload { active, failed })
}
