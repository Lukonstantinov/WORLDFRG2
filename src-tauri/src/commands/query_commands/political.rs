//! political commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Re-rank settlements by trade power and emit influence centers. Power blends
/// base habitability (settlement score), route centrality (how many trade links
/// a settlement anchors under the chosen reach) and trade monopoly (being the
/// dominant producer of goods — strongest for the seeded one-homeland goods).
/// The frontend draws translucent influence discs sized by power.
#[tauri::command]
pub fn compute_political(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    piracy: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<PoliticalCenter>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;

    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.is_empty() { return Ok(vec![]); }

    // Top settlements by base score (the political contenders). Both the
    // contender pool and the emitted-hub count scale with economic granularity
    // (default 14 → 84 contenders / 42 hubs, matching the legacy 80 / 40).
    let er = economic_regions.clamp(2, 40) as usize;
    let contenders = (er * 6).clamp(10, 200);
    // Emit only the MOST IMPORTANT hubs (the user asked for far fewer trade hubs);
    // the contender pool stays large so ranking is accurate, but the rendered set
    // is trimmed hard. Lesser towns still exist in the settlement list for later
    // layers — they just aren't drawn as trade hubs.
    let hub_out = er.clamp(6, 20);
    let mut idx: Vec<usize> = (0..settlements.len()).collect();
    idx.sort_by(|&a, &b| settlements[b].score.partial_cmp(&settlements[a].score)
        .unwrap_or(std::cmp::Ordering::Equal));
    idx.truncate(contenders);
    let nn = idx.len();
    let nodes: Vec<&RouteSettlement> = idx.iter().map(|&i| &settlements[i]).collect();

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // Route centrality: count reachable nearest-neighbour links per node.
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, 0, 0)?;
    let cnode: Vec<usize> = nodes.iter().map(|s| {
        let cx = (s.x / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (s.y / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();
    // Sea access (real ocean port, NOT a closed lake) + outpost flags per node.
    // distance_to_ocean only measures distance to the OPEN OCEAN, so a lakeshore
    // town reads as inland here — exactly what we want: lake hubs can't be emporia.
    let sea_access: Vec<bool> = nodes.iter().map(|s| {
        let t = world.tile((s.x / TILE_SIZE) as i32, (s.y / TILE_SIZE) as i32);
        let ti = ((s.y % TILE_SIZE) * TILE_SIZE + (s.x % TILE_SIZE)) as usize;
        t.distance_to_ocean.get(ti).map(|&d| d < 0.06).unwrap_or(false)
    }).collect();
    let is_outpost: Vec<bool> = nodes.iter().map(|s| s.size == "outpost").collect();
    let mut edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for i in 0..nn {
        let mut dists: Vec<(usize, i64)> = Vec::new();
        for j in 0..nn {
            if i == j { continue; }
            let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
            let dy = nodes[i].y as i64 - nodes[j].y as i64;
            dists.push((j, dx * dx + dy * dy));
        }
        dists.sort_by_key(|&(_, d)| d);
        for &(j, _) in dists.iter().take(3) {
            edges.insert((i.min(j), i.max(j)));
        }
    }
    let mut centrality = vec![0.0f32; nn];
    for &(a, b) in &edges {
        if let Some(path) = coarse_dijkstra(&cc, cnode[a], cnode[b]) {
            if path_allowed(&cc, &path, reach, max_crossing, grid_w) {
                centrality[a] += 1.0;
                centrality[b] += 1.0;
            }
        }
    }

    // Monopoly: assign each good-bearing coarse cell to its nearest node, then
    // measure how large a share of each goods total production a node holds.
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let mut prod = vec![vec![0.0f32; gc]; nn];
    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    let mut coarse = vec![vec![0.0f32; gc]; (cw * ch) as usize];
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    for g in 0..gc.min(tile.goods.len()) {
                        coarse[ci][g] += tile.goods[g][ti] as f32 / 255.0;
                    }
                }
            }
        }
    }
    let max_reach = (grid_w / 4) as i64;
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (ni, s) in nodes.iter().enumerate() {
                let dx = wrap_dx(wx, s.x as i32) as i64;
                let dy = (wy - s.y as i32) as i64;
                let d = dx * dx + dy * dy;
                if d < bd { bd = d; best = ni; }
            }
            if bd > max_reach * max_reach { continue; }
            for g in 0..gc {
                prod[best][g] += coarse[ci][g];
            }
        }
    }
    let mut good_total = vec![0.0f32; gc];
    for g in 0..gc {
        for ni in 0..nn { good_total[g] += prod[ni][g]; }
    }
    let mut monopoly = vec![0.0f32; nn];
    let mut monopolies: Vec<Vec<String>> = vec![Vec::new(); nn];
    for ni in 0..nn {
        for g in 0..gc {
            if good_total[g] < 1e-4 || prod[ni][g] < 0.05 { continue; }
            let share = prod[ni][g] / good_total[g];
            monopoly[ni] += share * share; // squared → rewards true dominance
            if share > 0.55 {
                monopolies[ni].push(specs[g].id.clone());
            }
        }
    }

    // Combine into trade power and normalize.
    let cmax = centrality.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mmax = monopoly.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mut power = vec![0.0f32; nn];
    let pop_max = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    for ni in 0..nn {
        // Population-weighted (matches compute_economy) so the top hub — the one
        // marked with the golden square — is a large central city, not a remote
        // island that just monopolises a good.
        let pop_n = (nodes[ni].population as f32 / pop_max).clamp(0.0, 1.0).sqrt();
        let mut p = 0.26 * nodes[ni].score.clamp(0.0, 1.0)
            + 0.30 * pop_n
            + 0.30 * (centrality[ni] / cmax)
            + 0.14 * (monopoly[ni] / mmax);
        // Great trade hubs are SEA PORTS — a sea-accessible node gets a boost, and a
        // lake-locked / inland one is held back, so the largest hubs sit on the coast
        // of the open sea, not a closed lake (the user's request).
        if sea_access[ni] { p *= 1.18; } else { p *= 0.72; }
        // Tiny trade outposts are never hub contenders — floor their power so they
        // can't be ranked, marked, or named as trade hubs (they get black dots).
        if is_outpost[ni] { p *= 0.05; }
        power[ni] = p;
    }
    let pmax = power.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    let mut order: Vec<usize> = (0..nn).collect();
    order.sort_by(|&a, &b| power[b].partial_cmp(&power[a]).unwrap_or(std::cmp::Ordering::Equal));

    // Throughput: total goods handled in the hub's hinterland, lifted by how many
    // trade links it anchors (a central node moves more than its own production).
    let mut throughput = vec![0.0f32; nn];
    for ni in 0..nn {
        let prod_sum: f32 = prod[ni].iter().sum();
        throughput[ni] = prod_sum * (1.0 + 0.5 * centrality[ni] / cmax);
    }
    let tp_max = throughput.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // Emporia: the few greatest entrepôts by throughput (rendered red). They MUST be
    // sea ports — a great pass-through entrepôt is a maritime hub, never a lake town.
    let emporia: std::collections::HashSet<usize> = {
        let mut t: Vec<(f32, usize)> = (0..nn)
            .filter(|&ni| sea_access[ni] && !is_outpost[ni])
            .map(|ni| (throughput[ni], ni)).collect();
        t.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        t.into_iter().take(6).filter(|&(v, _)| v > 0.0).map(|(_, ni)| ni).collect()
    };

    let base_r = grid_w as f32 * 0.018;
    let span_r = grid_w as f32 * 0.075;
    let mut out: Vec<PoliticalCenter> = Vec::new();
    for (rank, &ni) in order.iter().enumerate().take(hub_out) {
        let p = power[ni] / pmax;
        // Power tier → 1..5 stars; only the dominant hubs reach 5 (Venice/Genoa).
        let stars = if p >= 0.80 { 5 } else if p >= 0.60 { 4 }
            else if p >= 0.42 { 3 } else if p >= 0.25 { 2 } else { 1 };
        let ref_pct = (throughput[ni] / tp_max * 100.0).clamp(0.0, 100.0);
        let mut gv: Vec<(usize, f32)> = (0..gc)
            .map(|g| (g, prod[ni][g]))
            .filter(|&(_, a)| a > 0.01)
            .collect();
        gv.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        gv.truncate(6);
        let top_goods: Vec<HubGood> = gv.into_iter()
            .map(|(g, a)| HubGood { name: specs[g].id.clone(), amount: a })
            .collect();
        out.push(PoliticalCenter {
            x: nodes[ni].x as f32,
            y: nodes[ni].y as f32,
            power: p,
            rank: rank as u32,
            radius: base_r + span_r * p,
            stars,
            population: nodes[ni].population,
            monopolies: monopolies[ni].clone(),
            name: crate::sim::names::gen_name_epithet(
                nodes[ni].x, nodes[ni].y, grid_w, grid_h,
                if stars >= 5 { 2 } else if stars >= 4 { 1 } else { 0 },
            ),
            throughput: throughput[ni],
            ref_pct,
            nearest_hub: nearest_ref_hub(ref_pct).to_string(),
            top_goods,
            emporium: emporia.contains(&ni),
        });
    }
    Ok(out)
}


/// Trade-development feedback: after the economy is solved, grow each settlement by
/// the trade wealth of its matching economy hub, so real emporia / chokepoint
/// cities swell into metropolises while inland subsistence towns stay put. One-way
/// (does NOT re-run the economy) and bounded (≤ ×3). Returns the updated list.
#[tauri::command]
pub fn compute_settlement_development(
    settlements_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<crate::sim::settlements::Settlement>, String> {
    use crate::sim::settlements::Settlement;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut settlements: Vec<Settlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.is_empty() { return Ok(settlements); }

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let snap: EconomySnapshot = match metadata::campaign_get_or_meta(&conn, "economy").map_err(|e| e.to_string())? {
        Some(json) => match serde_json::from_str(&json) { Ok(s) => s, Err(_) => return Ok(settlements) },
        None => return Ok(settlements),
    };
    if snap.hubs.is_empty() { return Ok(settlements); }

    let wrap_dx = |a: f32, b: f32| -> f32 {
        let mut d = (a - b).abs();
        if grid_w > 0 && d > grid_w as f32 / 2.0 { d = grid_w as f32 - d; }
        d
    };
    const DEV_GAIN: f32 = 2.0; // hub wealth 0..1 → up to ~×3 growth
    for s in settlements.iter_mut() {
        // Match to the nearest economy hub (settlements ARE the hubs, so this is
        // essentially itself) and read its normalized trade wealth.
        let mut best_w = 0.0f32;
        let mut bd = f32::INFINITY;
        for hub in &snap.hubs {
            let dx = wrap_dx(s.x as f32, hub.x);
            let dy = s.y as f32 - hub.y;
            let d = dx * dx + dy * dy;
            if d < bd { bd = d; best_w = hub.wealth; }
        }
        let grown = (s.population as f32 * (1.0 + DEV_GAIN * best_w.clamp(0.0, 1.0)))
            .min(s.population as f32 * 3.0);
        s.population = grown.round() as u32;
        s.size = if s.population >= 100_000 { "capital" }
            else if s.population >= 30_000 { "city" }
            else if s.population >= 5_000 { "town" }
            else { "village" }.to_string();
    }
    Ok(settlements)
}
