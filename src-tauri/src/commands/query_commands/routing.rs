//! routing commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Compute plausible trade routes between the major settlements over the shared
/// coarse cost grid (mountain passes, rivers, coast-hugging all priced in), with
/// the chosen trade reach limiting how far trade may cross open water.
#[tauri::command]
pub fn compute_trade_routes(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    piracy: f32,
    season: i32,
    months: u32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TradeRoute>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    compute_trade_routes_impl(
        &db, &conn, &settlements_json, &rivers_json, reach, max_crossing,
        desert_routes, economic_regions, piracy, season, months,
    )
}

/// The logic behind `compute_trade_routes`, extracted so it can be exercised
/// directly in tests without constructing a Tauri-managed `State` (B3's own
/// gate needs this — see `routing_fallback_tests` below).
fn compute_trade_routes_impl(
    db: &WorldDb,
    conn: &rusqlite::Connection,
    settlements_json: &str,
    rivers_json: &str,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    piracy: f32,
    season: i32,
    months: u32,
) -> Result<Vec<TradeRoute>, String> {
    let grid_w: u32 = metadata::get_meta(conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }

    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(settlements_json).unwrap_or_default();
    if settlements.len() < 2 { return Ok(vec![]); }

    let world = db.cached_tiles_with_conn(conn)?;
    let cc = cached_coarse_cost(db, &world, world.fingerprint, grid_w, grid_h, rivers_json, reach == 2, desert_routes, piracy, season, months)?;
    let (cw, f) = (cc.cw, cc.f);

    // Map EVERY settlement to a coarse node (sorted by score, strongest first).
    // The major network is built among the top hubs (3 nearest neighbours each),
    // but every remaining settlement still gets at least one minor road to its
    // nearest neighbour, so no town is left unconnected.
    let mut nodes: Vec<(i32, i32, f32)> = settlements.iter()
        .map(|s| ((s.x / f).min(cw as u32 - 1) as i32, (s.y / f).min(cc.ch as u32 - 1) as i32, s.score))
        .collect();
    nodes.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let nn = nodes.len();
    if nn < 2 { return Ok(vec![]); }
    // Primary-network node count scales with the chosen economic granularity
    // (default 14 regions ≈ 84 hubs, matching the legacy fixed 80).
    let hubs = nn.min(((economic_regions.clamp(2, 40) as usize) * 6).clamp(10, 200));

    // Candidate links: top hubs link to their 3 nearest neighbours. Minor
    // (lesser-town) links are built below with a ranked fallback shortlist
    // (B3), not a single nearest-neighbour pick.
    let mut major_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let nearest_k = |i: usize, k: usize| -> Vec<usize> {
        let mut dists: Vec<(usize, i64)> = Vec::with_capacity(nn - 1);
        for j in 0..nn {
            if i == j { continue; }
            let mut dx = (nodes[i].0 - nodes[j].0).abs();
            if dx > cw / 2 { dx = cw - dx; }
            let dy = nodes[i].1 - nodes[j].1;
            dists.push((j, (dx * dx + dy * dy) as i64));
        }
        dists.sort_by_key(|&(_, d)| d);
        dists.iter().take(k).map(|&(j, _)| j).collect()
    };
    for i in 0..hubs {
        for j in nearest_k(i, 3) { major_edges.insert((i.min(j), i.max(j))); }
    }
    // B3 (ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md §4/§9) — each lesser town gets
    // a RANKED shortlist of candidate hubs (nearest first), not just the single
    // nearest. The single-candidate version dropped a town SILENTLY whenever its
    // one pick failed — Dijkstra found nothing, the path's open-water run broke
    // the trade reach, or two towns shared a coarse cell — despite the comment
    // above it claiming "no town is left unconnected". Trying the shortlist in
    // order until one is actually accepted is what makes that guarantee true.
    const MINOR_FALLBACK_K: usize = 5;
    let nearest_hubs_ranked = |i: usize, k: usize| -> Vec<usize> {
        let mut dists: Vec<(usize, i64)> = Vec::with_capacity(hubs);
        for j in 0..hubs {
            if i == j { continue; }
            let mut dx = (nodes[i].0 - nodes[j].0).abs();
            if dx > cw / 2 { dx = cw - dx; }
            let dy = nodes[i].1 - nodes[j].1;
            dists.push((j, (dx * dx + dy * dy) as i64));
        }
        dists.sort_by_key(|&(_, d)| d);
        dists.iter().take(k).map(|&(j, _)| j).collect()
    };
    // Per lesser town, its ranked candidate hubs (empty for major-network hubs).
    let mut town_candidates: Vec<Vec<usize>> = vec![Vec::new(); nn];
    for i in hubs..nn {
        town_candidates[i] = nearest_hubs_ranked(i, MINOR_FALLBACK_K);
    }

    let all_edges: Vec<((usize, usize), bool)> = major_edges.iter().map(|&e| (e, false)).collect();
    // ONE DIJKSTRA PER SOURCE, NOT PER EDGE (P2/A1) — every edge/candidate pair
    // is grouped by its coarse SOURCE node before batching, so testing several
    // fallback candidates per town costs nothing beyond what testing only the
    // first candidate already cost per source.
    let major_pairs: Vec<(usize, usize)> = all_edges.iter()
        .map(|&((a, b), _)| (cc.cidx(nodes[a].0, nodes[a].1), cc.cidx(nodes[b].0, nodes[b].1)))
        .collect();
    let mut town_pairs: Vec<(usize, usize)> = Vec::new();
    for (i, cands) in town_candidates.iter().enumerate() {
        for &j in cands {
            town_pairs.push((cc.cidx(nodes[i].0, nodes[i].1), cc.cidx(nodes[j].0, nodes[j].1)));
        }
    }
    let mut all_pairs = major_pairs;
    all_pairs.extend_from_slice(&town_pairs);
    let all_paths = coarse_dijkstra_batch(&cc, &all_pairs);
    let (major_paths, town_paths) = all_paths.split_at(all_edges.len());

    let mut routes: Vec<TradeRoute> = Vec::new();
    let mut push_route = |path: Vec<usize>, minor: bool, routes: &mut Vec<TradeRoute>| {
        let mut sea_cells = 0u32;
        let mut river_cells = 0u32;
        let pts: Vec<[f32; 2]> = path.iter().map(|&c| {
            if !cc.is_land[c] { sea_cells += 1; }
            if cc.is_land[c] && cc.is_river[c] { river_cells += 1; }
            cc.world_of(c)
        }).collect();
        let kind = if sea_cells as usize * 3 >= path.len() {
            1
        } else if river_cells as usize * 4 >= path.len() {
            2
        } else {
            0
        };
        routes.push(TradeRoute { points: pts, kind, minor });
    };
    for (&(_, minor), path) in all_edges.iter().zip(major_paths.iter()) {
        let path = match path { Some(p) => p.clone(), None => continue };
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }
        push_route(path, minor, &mut routes);
    }
    // Consume `town_paths` in the same flattened order `town_pairs` was built in
    // (per town, candidates in rank order); take the FIRST candidate that both
    // found a path and clears the trade reach, so a rejected top pick falls
    // through instead of leaving the town unconnected. `unrouted` collects any
    // town whose whole shortlist failed — genuinely reported rather than
    // silently dropped, though surfacing it to the UI is not yet built (a
    // frontend/bridge addition, out of this pass's scope).
    let mut unrouted: Vec<usize> = Vec::new();
    let mut cursor = 0usize;
    for i in hubs..nn {
        let k = town_candidates[i].len();
        let mut accepted = false;
        for path in &town_paths[cursor..cursor + k] {
            if let Some(p) = path {
                if path_allowed(&cc, p, reach, max_crossing, grid_w) {
                    push_route(p.clone(), true, &mut routes);
                    accepted = true;
                    break;
                }
            }
        }
        if !accepted && k > 0 { unrouted.push(i); }
        cursor += k;
    }
    if !unrouted.is_empty() {
        log::warn!(
            "compute_trade_routes: {} of {} settlements have no legal route to any of their \
             {MINOR_FALLBACK_K} nearest hubs under the current trade reach",
            unrouted.len(), nn - hubs
        );
    }

    Ok(routes)
}


/// Travel-time / itinerary calculator (#23). Least-cost route between two world
/// cells over the same coarse grid trade uses, then per-mode journey times. Reuses
/// `cached_coarse_cost`, `coarse_dijkstra`, `path_metrics`'s per-segment medium
/// classification and `world_of` (no new pathfinding). `reach == 2` (continental)
/// blocks open-sea crossings so the route stays on one landmass.
#[tauri::command]
pub fn compute_itinerary(
    from_x: u32,
    from_y: u32,
    to_x: u32,
    to_y: u32,
    rivers_json: String,
    reach: u8,
    desert_routes: bool,
    db: State<'_, WorldDb>,
) -> Result<Itinerary, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let empty = Itinerary {
        points: vec![], reachable: false, km: 0.0, land_km: 0.0, river_km: 0.0,
        sea_km: 0.0, days_foot: 0.0, days_horse: 0.0, days_cart: 0.0, dominant_mode: 0,
    };
    if grid_w == 0 || grid_h == 0 { return Ok(empty); }

    let world = db.cached_tiles_with_conn(&conn)?;
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h,
        &rivers_json, reach == 2, desert_routes, 0.0, 0, 0)?;
    let km_per_cell = KM_EQUATOR / grid_w as f32;

    let start = cc.cidx((from_x / cc.f).min(cc.cw as u32 - 1) as i32,
                        (from_y / cc.f).min(cc.ch as u32 - 1) as i32);
    let goal = cc.cidx((to_x / cc.f).min(cc.cw as u32 - 1) as i32,
                       (to_y / cc.f).min(cc.ch as u32 - 1) as i32);

    let path = match coarse_dijkstra(&cc, start, goal) {
        Some(p) => p,
        None => return Ok(Itinerary {
            points: vec![cc.world_of(start), cc.world_of(goal)], ..empty
        }),
    };

    // Per-medium effective distance (km × terrain factor) mirrors `path_metrics`:
    // relief slows land legs; storm/reef hazard slows sea legs.
    let (mut land_km, mut river_km, mut sea_km) = (0.0f32, 0.0f32, 0.0f32);
    let (mut land_eff, mut river_eff, mut sea_eff) = (0.0f32, 0.0f32, 0.0f32);
    let (mut land_n, mut sea_n, mut river_n) = (0u32, 0u32, 0u32);
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ax = (a as i32) % cc.cw; let ay = (a as i32) / cc.cw;
        let bx = (b as i32) % cc.cw; let by = (b as i32) / cc.cw;
        let mut dx = (ax - bx).abs(); if dx > cc.cw / 2 { dx = cc.cw - dx; }
        let dy = (ay - by).abs();
        let diag = dx != 0 && dy != 0;
        let seg_km = (if diag { std::f32::consts::SQRT_2 } else { 1.0 }) * cc.f as f32 * km_per_cell;
        let (a_land, b_land) = (cc.is_land[a], cc.is_land[b]);
        let (a_riv, b_riv) = (cc.is_river[a], cc.is_river[b]);
        if a_land && b_land && (a_riv || b_riv) {
            river_n += 1;
            river_km += seg_km;
            river_eff += seg_km * (1.0 + 0.4 * ((cc.elev[a] + cc.elev[b]) * 0.5));
        } else if !a_land || !b_land {
            sea_n += 1;
            sea_km += seg_km;
            let hz = ((cc.sea_hazard[a] + cc.sea_hazard[b]) * 0.5).clamp(0.0, 1.0);
            sea_eff += seg_km * (1.0 + 0.8 * hz);
        } else {
            land_n += 1;
            land_km += seg_km;
            land_eff += seg_km * (1.0 + 1.6 * ((cc.elev[a] + cc.elev[b]) * 0.5));
        }
    }
    let water_days = river_eff / SPEED_RIVER_KMD + sea_eff / SPEED_SHIP_KMD;
    let dominant_mode = if sea_n >= land_n && sea_n >= river_n { 1 }
        else if river_n > land_n { 2 } else { 0 };

    Ok(Itinerary {
        points: path.iter().map(|&c| cc.world_of(c)).collect(),
        reachable: true,
        km: land_km + river_km + sea_km,
        land_km, river_km, sea_km,
        days_foot: land_eff / SPEED_FOOT_KMD + water_days,
        days_horse: land_eff / SPEED_HORSE_KMD + water_days,
        days_cart: land_eff / SPEED_CART_KMD + water_days,
        dominant_mode,
    })
}


/// Build a region↔region commodity-flow matrix. Settlements are clustered into
/// economic regions; each region's production of every good is summed from the
/// belt fields in its territory, demand is scaled by economic size, and net
/// surpluses are matched to deficits as flows along straight inter-region links.
#[tauri::command]
pub fn compute_trade_matrix(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    luxury_bias: f32,
    piracy: f32,
    db: State<'_, WorldDb>,
) -> Result<TradeMatrix, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    // Active (editable) good specs drive names/desire/luxury so the matrix tracks
    // the world's authored goods.
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let goods_names: Vec<String> = specs.iter().map(|s| s.id.clone()).collect();
    if grid_w == 0 || grid_h == 0 {
        return Ok(TradeMatrix { regions: vec![], flows: vec![], trunks: vec![], goods: goods_names });
    }
    let world = db.cached_tiles_with_conn(&conn)?;

    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.len() < 2 {
        return Ok(TradeMatrix { regions: vec![], flows: vec![], trunks: vec![], goods: goods_names });
    }

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // ── 1. Cluster settlements into regions (greedy seeds by score, spaced) ──
    let mut sorted: Vec<&RouteSettlement> = settlements.iter().collect();
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    // Region granularity is user-controlled (default 14). More regions → tighter
    // spacing so they can all fit; fewer → wider, more spaced economic blocs.
    let er = economic_regions.clamp(2, 40) as usize;
    let min_sep = (grid_w / (er as u32 / 2 + 2)).max(1) as i32;
    let mut seeds: Vec<(i32, i32)> = Vec::new();
    for s in &sorted {
        let (sx, sy) = (s.x as i32, s.y as i32);
        let far = seeds.iter().all(|&(qx, qy)| {
            let dx = wrap_dx(sx, qx);
            let dy = sy - qy;
            ((dx * dx + dy * dy) as f32).sqrt() >= min_sep as f32
        });
        if far { seeds.push((sx, sy)); }
        if seeds.len() >= er { break; }
    }
    if seeds.is_empty() {
        return Ok(TradeMatrix { regions: vec![], flows: vec![], trunks: vec![], goods: goods_names });
    }

    // Assign every settlement to its nearest seed; accumulate weighted centers.
    let nr = seeds.len();
    let mut acc_x = vec![0.0f64; nr];
    let mut acc_y = vec![0.0f64; nr];
    let mut acc_w = vec![0.0f64; nr];
    let assign = |sx: i32, sy: i32| -> usize {
        let mut best = 0usize;
        let mut bd = i64::MAX;
        for (ri, &(qx, qy)) in seeds.iter().enumerate() {
            let dx = wrap_dx(sx, qx) as i64;
            let dy = (sy - qy) as i64;
            let d = dx * dx + dy * dy;
            if d < bd { bd = d; best = ri; }
        }
        best
    };
    for s in &settlements {
        let ri = assign(s.x as i32, s.y as i32);
        let wgt = (s.score.max(0.05)) as f64;
        // unwrap x toward the seed for a correct mean across the seam
        let mut sx = s.x as i32;
        let seedx = seeds[ri].0;
        if sx - seedx > grid_w as i32 / 2 { sx -= grid_w as i32; }
        else if seedx - sx > grid_w as i32 / 2 { sx += grid_w as i32; }
        acc_x[ri] += sx as f64 * wgt;
        acc_y[ri] += s.y as f64 * wgt;
        acc_w[ri] += wgt;
    }
    let wrap_x = |x: i32| -> i32 { ((x % grid_w as i32) + grid_w as i32) % grid_w as i32 };
    let mut centers: Vec<(i32, i32)> = Vec::with_capacity(nr);
    let mut region_weight = vec![0.0f32; nr];
    for ri in 0..nr {
        let cw_ = acc_w[ri].max(1e-6);
        let cx = wrap_x((acc_x[ri] / cw_).round() as i32);
        let cy = (acc_y[ri] / cw_).round() as i32;
        centers.push((cx, cy));
        region_weight[ri] = acc_w[ri] as f32;
    }

    // ── 2. Production: sum each good over the region's territory ──────────────
    // Coarse pass; assign each coarse cell to its nearest region center.
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut production = vec![vec![0.0f32; gc]; nr];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    // Sum goods into a coarse grid first (cheap), then assign coarse cells.
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
            // world coords of this coarse cell centre
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            // nearest region
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (ri, &(qx, qy)) in centers.iter().enumerate() {
                let dx = wrap_dx(wx, qx) as i64;
                let dy = (wy - qy) as i64;
                let d = dx * dx + dy * dy;
                if d < bd { bd = d; best = ri; }
            }
            if bd > max_reach * max_reach { continue; }
            for g in 0..gc {
                production[best][g] += coarse[ci][g];
            }
        }
    }

    // Per-good raw totals (absolute footprint) BEFORE normalization, so true
    // scarcity survives: a good present in only a sliver of one region must not
    // read as "plentiful" merely because it normalizes to 1.0 (#18). Deposit
    // goods (gems/metals) are spatially tiny but high-value, so they earn an
    // abundance floor to ensure they still drive trade (#19).
    let raw_total: Vec<f32> = (0..gc)
        .map(|g| production.iter().map(|p| p[g]).sum::<f32>())
        .collect();
    let raw_total_max = raw_total.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let is_deposit: Vec<bool> = (0..gc)
        .map(|g| specs.get(g)
            .map(|s| matches!(s.distribution, crate::sim::goods_spec::Distribution::Deposits))
            .unwrap_or(false))
        .collect();
    let abundance: Vec<f32> = (0..gc).map(|g| {
        let a = (raw_total[g] / raw_total_max).sqrt(); // soft scarcity curve
        if is_deposit[g] { a.max(0.6) } else { a }
    }).collect();

    // Normalize production per good across regions to 0..1 (relative pattern),
    // then rescale by global abundance so absolute scarcity carries through to net.
    for g in 0..gc {
        let mx = production.iter().map(|p| p[g]).fold(0.0f32, f32::max);
        if mx > 0.0 {
            for ri in 0..nr { production[ri][g] = production[ri][g] / mx * abundance[g]; }
        }
    }

    // ── 3. Demand: economic size × per-good base demand (from the active spec) ──
    let max_rw = region_weight.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    // Per-good desire / luxury flag come from the world's editable good spec, so
    // disabled goods demand nothing and custom desires take effect.
    let desire: Vec<f32> = (0..gc)
        .map(|g| specs.get(g).filter(|s| s.enabled).map(|s| s.desire).unwrap_or(0.0))
        .collect();
    let is_luxury: Vec<bool> = (0..gc)
        .map(|g| specs.get(g).map(|s| s.network_luxury).unwrap_or(false))
        .collect();
    // Luxuries are only prized across a large/open trade network; tighter reaches
    // (more closed networks) realize less of that demand.
    let reach_factor = match reach { 0 => 1.0, 1 => 0.7, _ => 0.45 };
    // Mercantile ↔ subsistence bias (#4): >0.5 prizes distant luxuries
    // (silk/spices), <0.5 makes the world care mostly about staples. Neutral 0.5
    // leaves the legacy behaviour (lux_mult == 1.0). Staples are unaffected.
    let lux_mult = (0.6 + luxury_bias.clamp(0.0, 1.0) * 1.4).clamp(0.3, 2.2);
    // Full-basket floor, unified with compute_economy: every good is at least
    // modestly desired so regions import what they don't produce (not only the
    // few high-`desire` staples), and the two engines agree on who imports what.
    const BASKET_FLOOR: f32 = 0.35;
    let mut demand = vec![vec![0.0f32; gc]; nr];
    for ri in 0..nr {
        let size = region_weight[ri] / max_rw; // 0..1
        for g in 0..gc {
            let mut d = size * desire[g].max(BASKET_FLOOR);
            if is_luxury[g] {
                // Discount in the good's own producing homeland (it's local/common
                // there) and scale by how open the trade network is + the world's
                // taste for luxuries.
                let homeland_discount = 1.0 - 0.6 * production[ri][g].clamp(0.0, 1.0);
                d *= reach_factor * homeland_discount * lux_mult;
            }
            demand[ri][g] = d;
        }
    }

    // Net = production − demand.
    let mut regions: Vec<TradeRegion> = Vec::with_capacity(nr);
    for ri in 0..nr {
        let net: Vec<f32> = (0..gc).map(|g| production[ri][g] - demand[ri][g]).collect();
        regions.push(TradeRegion {
            id: ri as u32,
            name: format!("Region {}", ri + 1),
            x: centers[ri].0 as f32,
            y: centers[ri].1 as f32,
            production: production[ri].clone(),
            demand: demand[ri].clone(),
            net,
        });
    }

    // ── 4. Flows: match surpluses to deficits per good, routed over the trade
    // network and bundled into trunks. A supplier can only serve an importer if
    // a route between their regions exists under the chosen trade reach (so when
    // continents are too far / the ocean too wide, trade stays within reach and
    // no flow is drawn). Flows that share a corridor stack onto the same trunk.
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, 0, 0)?;
    let region_node: Vec<usize> = centers.iter().map(|&(x, y)| {
        let cx = (x.max(0) as u32 / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (y.max(0) as u32 / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();

    // Lazily-computed least-cost path per region pair (None = unreachable).
    let mut pair_path: std::collections::HashMap<(usize, usize), Option<Vec<usize>>> =
        std::collections::HashMap::new();
    // ONE DIJKSTRA PER SOURCE, NOT PER PAIR (P2/A1). Which (supply, deficit) pairs
    // are ever queried is only known lazily — the greedy match below can satisfy a
    // deficit from its first supplier and never reach the rest — so unlike the
    // fixed-edge-list call sites this cannot pre-batch. Instead a source's full
    // single-source Dijkstra is run at most ONCE and kept here; a supply region that
    // recurs across many goods (the common case — the outer loop is per-good) then
    // answers every later pair from the cached `(dist, prev)` instead of re-searching
    // the whole grid.
    let mut source_dist_prev: std::collections::HashMap<usize, (Vec<i64>, Vec<usize>)> =
        std::collections::HashMap::new();
    // Per coarse-edge accumulation for the bundled trunks: total volume, volume
    // by good (to name the corridor by its dominant commodity), and directional
    // volume (which way the goods are pulled — toward the consuming hub).
    #[derive(Default)]
    struct EdgeAcc {
        total: f32,
        fwd: f32, // volume flowing min→max node
        bwd: f32, // volume flowing max→min node
        by_good: std::collections::HashMap<usize, f32>,
    }
    let mut edge_acc: std::collections::HashMap<(usize, usize), EdgeAcc> =
        std::collections::HashMap::new();

    let mut flows: Vec<TradeFlow> = Vec::new();
    for g in 0..gc {
        let mut supply: Vec<(usize, f32)> = (0..nr)
            .filter_map(|ri| { let n = regions[ri].net[g]; if n > 0.05 { Some((ri, n)) } else { None } })
            .collect();
        let mut deficit: Vec<(usize, f32)> = (0..nr)
            .filter_map(|ri| { let n = regions[ri].net[g]; if n < -0.05 { Some((ri, -n)) } else { None } })
            .collect();
        if supply.is_empty() || deficit.is_empty() { continue; }
        deficit.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for &mut (di, mut need) in deficit.iter_mut() {
            let (dx0, dy0) = centers[di];
            supply.sort_by_key(|&(si, _)| {
                let (sx, sy) = centers[si];
                let dx = wrap_dx(sx, dx0) as i64;
                let dy = (sy - dy0) as i64;
                dx * dx + dy * dy
            });
            for s in supply.iter_mut() {
                if need <= 0.05 { break; }
                if s.1 <= 0.05 { continue; }
                if s.0 == di { continue; }
                let key = (s.0.min(di), s.0.max(di));
                if !pair_path.contains_key(&key) {
                    // Trade flows must NEVER cross open ocean: reject any routed
                    // path that touches an open-water (no-land-neighbour) cell.
                    // Coastal-sea hugging is still allowed (is_open_sea is false
                    // there), so island/coastal trade via short coastal hops works,
                    // but no trunk beelines across a basin between continents.
                    let src = region_node[s.0];
                    let (dist, prev) = source_dist_prev.entry(src)
                        .or_insert_with(|| coarse_dijkstra_dist_prev(&cc, src));
                    let p = coarse_path_from_dist_prev(dist, prev, src, region_node[di])
                        .filter(|p| path_allowed(&cc, p, reach, max_crossing, grid_w))
                        .filter(|p| !p.iter().any(|&c| cc.is_open_sea[c]));
                    pair_path.insert(key, p);
                }
                if pair_path[&key].is_none() { continue; } // unreachable under this reach
                let amt = need.min(s.1);
                s.1 -= amt;
                need -= amt;
                // Accumulate this flow onto every coarse edge of its routed path,
                // in the consumer-ward direction (supply s.0 → deficit di).
                if let Some(Some(path)) = pair_path.get(&key) {
                    let forward = path.first() == Some(&region_node[s.0]);
                    for w in path.windows(2) {
                        let (from, to) = if forward { (w[0], w[1]) } else { (w[1], w[0]) };
                        let e = (from.min(to), from.max(to));
                        let acc = edge_acc.entry(e).or_default();
                        acc.total += amt;
                        if from < to { acc.fwd += amt; } else { acc.bwd += amt; }
                        *acc.by_good.entry(g).or_insert(0.0) += amt;
                    }
                }
                let (sx, sy) = centers[s.0];
                flows.push(TradeFlow {
                    from: s.0 as u32,
                    to: di as u32,
                    good: g,
                    good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    weight: amt,
                    points: vec![[sx as f32, sy as f32], [dx0 as f32, dy0 as f32]],
                });
            }
        }
    }

    // Build bundled trunks from the per-edge accumulation: width ∝ volume, points
    // ordered in the dominant flow direction (source→consumer), dominant good, and
    // a corridor name for the major arteries.
    let mut edges_v: Vec<((usize, usize), EdgeAcc)> = edge_acc.into_iter().collect();
    edges_v.sort_by(|a, b| b.1.total.partial_cmp(&a.1.total).unwrap_or(std::cmp::Ordering::Equal));
    let max_total = edges_v.first().map(|e| e.1.total).unwrap_or(1.0).max(1e-6);
    let mut trunks: Vec<TradeTrunk> = Vec::new();
    // Name ONE corridor per dominant commodity (its single highest-volume edge,
    // since edges are volume-sorted), so many goods get a named road without the
    // same name repeating along every segment of a corridor.
    let mut named_goods: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for ((a, b), acc) in edges_v.into_iter() {
        if acc.total < 0.02 { continue; }
        let good = acc.by_good.iter()
            .max_by(|x, y| x.1.partial_cmp(y.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(g, _)| *g);
        let (p_from, p_to) = if acc.fwd >= acc.bwd { (a, b) } else { (b, a) };
        let road = match good {
            Some(g) if acc.total >= 0.06 * max_total && named_goods.insert(g) => {
                goods_names.get(g).map(|id| road_name(id)).unwrap_or_default()
            }
            _ => String::new(),
        };
        trunks.push(TradeTrunk {
            points: vec![cc.world_of(p_from), cc.world_of(p_to)],
            volume: acc.total,
            good: good.map(|g| g as i32).unwrap_or(-1),
            road,
        });
    }

    Ok(TradeMatrix { regions, flows, trunks, goods: goods_names })
}

#[cfg(test)]
mod routing_fallback_tests {
    use super::*;
    use crate::db::{schema, WorldDb};
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
    use rusqlite::Connection;

    /// B3 (ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md §4/§9) — a lesser town whose
    /// geometrically NEAREST hub sits across a strait wider than the trade
    /// reach must still get a route, by falling through to its next-nearest
    /// hub, rather than being dropped because its first pick failed. Builds a
    /// mainland (hub B + the lesser town T, all-land, always reachable) and a
    /// small island (hub A) separated from it by open water far wider than
    /// the chosen `max_crossing`, with T placed closer to A by straight-line
    /// distance than to B — the single-nearest-candidate code picked A,
    /// failed `path_allowed`, and dropped T with no fallback.
    #[test]
    fn a_lesser_town_falls_through_to_its_second_hub_when_the_first_is_unreachable() {
        let w = 3600u32;
        let h = 20u32;
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", &w.to_string()), ("grid_height", &h.to_string())] {
            metadata::set_meta(&conn, k, v).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                // Mainland x < 3000; a small island at [3280, 3320); open sea
                // elsewhere (a ~280-cell-wide strait on both sides of the
                // island, far past any reasonable crossing limit).
                let land = x < 3000 || (3280..3320).contains(&x);
                buf.terrain[idx] = if land { 1 } else { 0 };
                buf.elevation[idx] = if land { 0.1 } else { 0.0 };
                buf.koppen[idx] = 12; // Cfb, no surcharge
                buf.temperature[idx] = 18.0; // above every freeze threshold
            }
        }
        buf.save(&conn, "test").unwrap();
        let db = WorldDb::new(conn);
        let conn = db.conn.lock().unwrap();

        // 10 far-off mainland fillers + hub A (island) + hub B (mainland) all
        // tied at score 100 clear the hub cutoff (economic_regions=2 → top 12);
        // T is scored last so it alone falls into the fallback path.
        let fillers = [300, 500, 700, 900, 1100, 1300, 1500, 1700, 1900, 2100];
        let mut json = String::from("[");
        for x in fillers { json += &format!(r#"{{"x":{x},"y":10,"score":100}},"#); }
        json += r#"{"x":3300,"y":10,"score":100},"#; // hub A — island, across the strait
        json += r#"{"x":100,"y":10,"score":100},"#;  // hub B — mainland, reachable by land
        json += r#"{"x":2990,"y":10,"score":1}"#;    // T — nearer A by straight line, but A is unreachable
        json += "]";

        // reach=1 (coastal/short crossings), max_crossing=0.02 (72 fine cells
        // at this grid) — the ~280-cell strait to the island fails this by a
        // wide margin; an all-land route to hub B does not.
        let routes = compute_trade_routes_impl(
            &db, &conn, &json, "", 1, 0.02, false, 2, 0.0, -1, 12,
        ).expect("compute_trade_routes_impl failed");

        // T's own position (world coords) must appear as an ENDPOINT of some
        // returned route — proof it got a road at all, not just that routes
        // exist in general.
        let t_pt = [2990.0f32, 10.0];
        let t_has_route = routes.iter().any(|r| {
            r.points.first().is_some_and(|&p| close(p, t_pt))
                || r.points.last().is_some_and(|&p| close(p, t_pt))
        });
        assert!(t_has_route,
            "the lesser town must fall through to hub B once hub A fails the \
             crossing limit, not be silently dropped — routes found: {}",
            routes.len());
    }

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 6.0 && (a[1] - b[1]).abs() < 6.0
    }
}
