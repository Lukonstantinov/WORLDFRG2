//! economy commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Build the economy snapshot, persist it to metadata, and return it. Same
/// inputs as the matrix/political commands so the hubs stay consistent.
#[tauri::command]
pub fn compute_economy(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    luxury_bias: f32,
    piracy: f32,
    season: i32,
    months: u32,
    db: State<'_, WorldDb>,
) -> Result<EconomySnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let goods_names: Vec<String> = specs.iter().map(|s| s.id.clone()).collect();
    let empty = EconomySnapshot { hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![], corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names.clone(), colonizable_sites: vec![] };
    if grid_w == 0 || grid_h == 0 {
        return Ok(empty);
    }
    let world = db.cached_tiles_with_conn(&conn)?;
    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.len() < 2 {
        let _ = metadata::campaign_set(&conn, "economy", &serde_json::to_string(&empty).unwrap_or_default());
        return Ok(empty);
    }
    // Empty-land colonization candidates for the campaign (uses tile data here).
    let base_value: Vec<f32> = specs.iter().map(|s| s.base_value.max(0.0)).collect();
    let settle_xy: Vec<(f32, f32)> = settlements.iter().map(|s| (s.x as f32, s.y as f32)).collect();
    let province_raster = read_province_raster(&conn);
    let colonizable_sites = compute_colonizable_sites(&world, grid_w, grid_h, &settle_xy, &base_value, province_raster.as_ref());

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // ── Market nodes: EVERY settlement is an economic node (so any town is
    // clickable and inspectable), sorted by score — strongest first — so the hub
    // topology below mirrors compute_trade_routes exactly. ──
    let er = economic_regions.clamp(2, 40) as usize;
    let mut idx: Vec<usize> = (0..settlements.len()).collect();
    idx.sort_by(|&a, &b| settlements[b].score.partial_cmp(&settlements[a].score)
        .unwrap_or(std::cmp::Ordering::Equal));
    let nodes: Vec<&RouteSettlement> = idx.iter().map(|&i| &settlements[i]).collect();
    let nn = nodes.len();
    if nn < 2 {
        let _ = metadata::campaign_set(&conn, "economy", &serde_json::to_string(&empty).unwrap_or_default());
        return Ok(empty);
    }
    // Major hubs carry the primary network; lesser towns hang off their nearest
    // hub. `territory_n` bounds the (legible) trade-territory overlay.
    let major_n = nn.min((er * 6).clamp(10, 200));
    let territory_n = nn.min((er * 3).clamp(5, 60));

    // Cost grid built WITH the chosen season + piracy so seasonal closures and
    // raiders actually shape trade flows and prices (not just the drawn routes).
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, season, months)?;
    let cnode: Vec<usize> = nodes.iter().map(|s| {
        let cx = (s.x / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (s.y / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();
    // Sea access (real ocean port vs closed-lake / inland) + outpost flags per node.
    let node_sea: Vec<bool> = nodes.iter().map(|s| {
        let t = world.tile((s.x / TILE_SIZE) as i32, (s.y / TILE_SIZE) as i32);
        let ti = ((s.y % TILE_SIZE) * TILE_SIZE + (s.x % TILE_SIZE)) as usize;
        t.distance_to_ocean.get(ti).map(|&d| d < 0.06).unwrap_or(false)
    }).collect();
    let is_outpost: Vec<bool> = nodes.iter().map(|s| s.size == "outpost").collect();

    // ── Trade-route GRAPH ────────────────────────────────────────────────────
    // Every settlement is wired into the network exactly as compute_trade_routes
    // draws it: each major hub links to its 3 nearest neighbours, and every lesser
    // town links to its nearest major hub. An edge survives only if a real route
    // exists under the chosen reach, so goods can flow ONLY where there is a road /
    // sea-route — never beelining across an ocean that has no route. Each edge
    // carries its routed coarse path (for chokepoints) and least-cost weight.
    let nearest_k = |i: usize, k: usize, pool: usize| -> Vec<usize> {
        let mut d: Vec<(usize, i64)> = Vec::new();
        for j in 0..pool {
            if i == j { continue; }
            let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
            let dy = nodes[i].y as i64 - nodes[j].y as i64;
            d.push((j, dx * dx + dy * dy));
        }
        d.sort_by_key(|&(_, dd)| dd);
        d.iter().take(k).map(|&(j, _)| j).collect()
    };
    // Absolute link ceiling: no trade link may span more than this straight-line
    // distance, so a lone far island never wires itself to a distant continent
    // regardless of reach (the user: "no trade routes to far away islands which do
    // not adhere to trade distance"). The crossing-fraction reach test still applies
    // on top of this via path_allowed.
    let max_link2: i64 = {
        let d = (grid_w as f32 * 0.30).max(60.0);
        (d * d) as i64
    };
    let node_dist2 = |i: usize, j: usize| -> i64 {
        let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
        let dy = nodes[i].y as i64 - nodes[j].y as i64;
        dx * dx + dy * dy
    };
    let mut cand: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for i in 0..major_n {
        // 4 nearest neighbours (was 3) so a continent's hub graph is well enough
        // connected to form one trading market — goods can then reach across it.
        for j in nearest_k(i, 4, nn) {
            if node_dist2(i, j) <= max_link2 { cand.insert((i.min(j), i.max(j))); }
        }
    }
    for i in major_n..nn {
        if let Some(&j) = nearest_k(i, 1, major_n).first() {
            if node_dist2(i, j) <= max_link2 { cand.insert((i.min(j), i.max(j))); }
        }
    }
    // Direct maritime bypass legs: a ship can sail between two coastal hubs
    // WITHOUT calling at every intermediate port. Without these the hub graph is
    // only nearest-neighbour, so a long sea haul must chain A→B→C→D and the
    // displayed price ladder stacks a transit toll at every pass-through city —
    // the "price balloons jumping city to city" the user reported. Connect each
    // coastal major hub to its nearest OTHER coastal hubs (even distant ones,
    // within the link ceiling); the routed path between two ports is naturally
    // sea-dominant, and `path_allowed` below still drops crossings the chosen
    // trade reach forbids. The shortest-path tree then prefers the direct leg
    // when it beats the tolled chain, so the rebuilt chain has no pass-through
    // toll hubs to compound.
    {
        let coastal: Vec<usize> = (0..nn).filter(|&i| node_sea[i]).collect();
        for &i in coastal.iter().filter(|&&i| i < major_n) {
            let mut d: Vec<(usize, i64)> = coastal.iter().filter(|&&j| j != i)
                .map(|&j| (j, node_dist2(i, j))).collect();
            d.sort_by_key(|&(_, dd)| dd);
            for &(j, dd) in d.iter().take(5) {
                if dd <= max_link2 { cand.insert((i.min(j), i.max(j))); }
            }
        }
    }
    // Materialise each candidate as a routed edge → adjacency + per-edge coarse path.
    let mut adj: Vec<Vec<(usize, f32, usize)>> = vec![Vec::new(); nn]; // (to, cost, edge_id)
    let mut edge_paths: Vec<Vec<usize>> = Vec::new();
    for &(a, b) in &cand {
        let path = match coarse_dijkstra(&cc, cnode[a], cnode[b]) { Some(p) => p, None => continue };
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }
        let cost = coarse_path_cost(&cc, &path).max(0.01);
        let eid = edge_paths.len();
        edge_paths.push(path);
        adj[a].push((b, cost, eid));
        adj[b].push((a, cost, eid));
    }
    // Centrality = reachable degree in the trade graph.
    let centrality: Vec<f32> = (0..nn).map(|i| adj[i].len() as f32).collect();
    // Connected components: a flow may only run between settlements that share a
    // trade-network component → no unreachable cross-ocean origins, ever.
    let mut comp = vec![usize::MAX; nn];
    {
        let mut c = 0usize;
        for s in 0..nn {
            if comp[s] != usize::MAX { continue; }
            comp[s] = c;
            let mut stack = vec![s];
            while let Some(u) = stack.pop() {
                for &(v, _, _) in &adj[u] {
                    if comp[v] == usize::MAX { comp[v] = c; stack.push(v); }
                }
            }
            c += 1;
        }
    }
    // All-pairs cheapest route over the (small) graph: cost + parent for rebuilding
    // the actual road a shipment travels.
    let mut gdist: Vec<Vec<f32>> = vec![vec![f32::INFINITY; nn]; nn];
    let mut gpar: Vec<Vec<usize>> = vec![vec![usize::MAX; nn]; nn];
    for s in 0..nn {
        gdist[s][s] = 0.0;
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, s)));
        let mut done = vec![false; nn];
        while let Some(Reverse((_, u))) = heap.pop() {
            if done[u] { continue; }
            done[u] = true;
            let du = gdist[s][u];
            for &(v, w, _) in &adj[u] {
                let nd = du + w;
                if nd < gdist[s][v] {
                    gdist[s][v] = nd;
                    gpar[s][v] = u;
                    heap.push(Reverse(((nd * 1000.0) as i64, v)));
                }
            }
        }
    }

    // ── Territory overlay grid: each land coarse cell → nearest MAJOR hub. This is
    // the legible trade-REGION overlay ONLY (kept broad per major hub so regions read
    // cleanly); the actual goods COLLECTION below is bounded by a small per-hub radius.
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut owner = vec![u16::MAX; (cw * ch) as usize];
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            let mut best_t = 0usize;
            let mut bd_t = i64::MAX;
            for (ni, s) in nodes.iter().enumerate().take(territory_n) {
                let dx = wrap_dx(wx, s.x as i32) as i64;
                let dy = (wy - s.y as i32) as i64;
                let d = dx * dx + dy * dy;
                if d < bd_t { bd_t = d; best_t = ni; }
            }
            let tx = (wx as u32 / TILE_SIZE) as i32;
            let ty = (wy as u32 / TILE_SIZE) as i32;
            let lt = world.tile(tx, ty);
            let ti = ((wy as u32 % TILE_SIZE) * TILE_SIZE + (wx as u32 % TILE_SIZE)) as usize;
            if lt.terrain[ti] != 0 { owner[ci] = best_t as u16; }
        }
    }

    // ── Production: each hub gathers goods from a BOUNDED radius — ≈50 km for the
    // smallest towns / outposts, scaling up to ≈120 km for a great metropolis (user
    // rule). Replaces the old unbounded nearest-hub Voronoi, so a lone city no longer
    // harvests half a continent. Cells beyond EVERY hub's radius stay UNCLAIMED (a
    // colonization prize that feeds the colony/outpost site scoring). Overlapping
    // radii resolve to the nearest hub. Marine goods on nearby sea are collected too
    // (a port works its own fishery), so no land-only filter here.
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let hub_radius_cells: Vec<i32> = (0..nn)
        .map(|hh| {
            let km = if is_outpost[hh] {
                50.0
            } else {
                // log-scaled 50→120 km across ~500 … ~100k population (ln 6.2 … 11.5).
                let t = ((nodes[hh].population as f32).max(1.0).ln() - 6.2) / (11.5 - 6.2);
                50.0 + t.clamp(0.0, 1.0) * (120.0 - 50.0)
            };
            ((km / km_per_cell).round() as i32).max(1)
        })
        .collect();
    // cell key (wy*grid_w + wx) → (best distance², owning hub).
    let mut claim: std::collections::HashMap<u64, (i64, u32)> = std::collections::HashMap::new();
    for hh in 0..nn {
        let r = hub_radius_cells[hh];
        let r2 = (r as i64) * (r as i64);
        let (sx, sy) = (nodes[hh].x as i32, nodes[hh].y as i32);
        for dy in -r..=r {
            let wy = sy + dy;
            if wy < 0 || wy >= grid_h as i32 { continue; }
            let dy2 = (dy as i64) * (dy as i64);
            for dx in -r..=r {
                let d2 = (dx as i64) * (dx as i64) + dy2;
                if d2 > r2 { continue; }
                let wx = (((sx + dx) % grid_w as i32) + grid_w as i32) % grid_w as i32;
                let key = (wy as u64) * (grid_w as u64) + wx as u64;
                let e = claim.entry(key).or_insert((i64::MAX, u32::MAX));
                if d2 < e.0 { *e = (d2, hh as u32); }
            }
        }
    }
    let mut prod = vec![vec![0.0f32; gc]; nn];
    for (&key, &(_, hh)) in claim.iter() {
        if hh == u32::MAX { continue; }
        let wx = (key % grid_w as u64) as u32;
        let wy = (key / grid_w as u64) as u32;
        let tile = world.tile((wx / TILE_SIZE) as i32, (wy / TILE_SIZE) as i32);
        let ti = ((wy % TILE_SIZE) * TILE_SIZE + (wx % TILE_SIZE)) as usize;
        for g in 0..gc.min(tile.goods.len()) {
            prod[hh as usize][g] += tile.goods[g][ti] as f32 / 255.0;
        }
    }

    // ── COVERAGE BACKSTOP · every good that EXISTS in the world reaches a producer ──
    // A seeded good gets ONE small homeland belt; if that homeland fell outside every
    // settlement's catchment (the 50–120 km radii above) it was credited to no hub, so
    // the good read "0 src" and could never be produced or traded for the entire
    // campaign (wine was the reported case, and every other seeded good is exposed to
    // the same accident of geography). For each good with ZERO catchment production,
    // find its single strongest belt cell anywhere in the world and credit it to the
    // NEAREST hub — the good enters the economy through whichever town is closest to
    // its homeland, scarce but ALIVE. A good with no belt cell anywhere (no suitable
    // climate on this world, or a manufactured good with no belt) is left absent —
    // that is the honest reading, not a bug, and manufactured goods are made live in
    // the tick regardless.
    {
        let uncovered: Vec<usize> = (0..gc)
            .filter(|&g| prod.iter().all(|p| p[g] <= 0.0))
            .collect();
        if !uncovered.is_empty() && nn > 0 {
            let mut best = vec![(0.0f32, 0u32, 0u32); uncovered.len()]; // (belt value, wx, wy)
            let ntx = ((grid_w + TILE_SIZE - 1) / TILE_SIZE) as i32;
            let nty = ((grid_h + TILE_SIZE - 1) / TILE_SIZE) as i32;
            for ty in 0..nty {
                for tx in 0..ntx {
                    let tile = world.tile(tx, ty);
                    for ly in 0..TILE_SIZE {
                        let wy = ty as u32 * TILE_SIZE + ly;
                        if wy >= grid_h { break; }
                        for lx in 0..TILE_SIZE {
                            let wx = tx as u32 * TILE_SIZE + lx;
                            if wx >= grid_w { break; }
                            let ti = (ly * TILE_SIZE + lx) as usize;
                            for (k, &g) in uncovered.iter().enumerate() {
                                if g >= tile.goods.len() { continue; }
                                let v = tile.goods[g][ti] as f32;
                                if v > best[k].0 { best[k] = (v, wx, wy); }
                            }
                        }
                    }
                }
            }
            for (k, &g) in uncovered.iter().enumerate() {
                let (v, wx, wy) = best[k];
                if v <= 0.0 { continue; } // genuinely absent on this world — do not invent it
                // Nearest live hub to the homeland cell (X wraps, Y clamps).
                let mut nh = 0usize;
                let mut nd = i64::MAX;
                for hh in 0..nn {
                    let raw = (nodes[hh].x as i64 - wx as i64).rem_euclid(grid_w as i64);
                    let dx = raw.min(grid_w as i64 - raw);
                    let dy = nodes[hh].y as i64 - wy as i64;
                    let d2 = dx * dx + dy * dy;
                    if d2 < nd { nd = d2; nh = hh; }
                }
                // Enough to clear the `> 0.05` emit gate below and read as a real, if
                // scarce, source (belt value is 0..1 after the /255).
                prod[nh][g] += (v / 255.0).max(0.12);
            }
        }
    }

    // Absolute scarcity (#18) + deposit floor (#19): rescale by global abundance.
    let raw_total: Vec<f32> = (0..gc).map(|g| prod.iter().map(|p| p[g]).sum::<f32>()).collect();
    let raw_total_max = raw_total.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let is_deposit: Vec<bool> = (0..gc)
        .map(|g| specs.get(g)
            .map(|s| matches!(s.distribution, crate::sim::goods_spec::Distribution::Deposits))
            .unwrap_or(false))
        .collect();
    let abundance: Vec<f32> = (0..gc).map(|g| {
        let a = (raw_total[g] / raw_total_max).sqrt();
        if is_deposit[g] { a.max(0.6) } else { a }
    }).collect();
    let good_max: Vec<f32> = (0..gc).map(|g| prod.iter().map(|p| p[g]).fold(0.0f32, f32::max).max(1e-6)).collect();

    // DEPOSITS AND MINING PLAN slice 2: a deposit good's quality is the mean ORE
    // GRADE of the workings inside a hub's catchment (§ "Slice 2 — grade → quality
    // rewire"), not its share of world production. Share-based quality read
    // backwards: a big cheap deposit (many low-grade workings summing to a large
    // belt total) scored as fine stones. Grade already IS a 0..1 richness number
    // (`Deposit::grade`), so this is a direct read, not a proxy.
    let deposits: Vec<crate::sim::deposits::Deposit> = metadata::get_meta(&conn, "deposits")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let good_slot: std::collections::HashMap<&str, usize> =
        specs.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
    let mut grade_sum = vec![vec![0.0f32; gc]; nn];
    let mut grade_n = vec![vec![0u32; gc]; nn];
    for d in &deposits {
        let Some(&g) = good_slot.get(d.good.as_str()) else { continue };
        let key = (d.y as u64) * (grid_w as u64) + d.x as u64;
        if let Some(&(_, hh)) = claim.get(&key) {
            if hh != u32::MAX {
                grade_sum[hh as usize][g] += d.grade;
                grade_n[hh as usize][g] += 1;
            }
        }
    }

    // GOODS_LOCALITIES_PLAN.md Slice 7 (D2) — the non-mineral counterpart of the
    // deposit-grade read above. A locality's `grade` is a direct terroir-quality
    // number (Slice 3), so a hub whose catchment holds a fine locality of a good
    // should read finer than one that merely produces a lot of it.
    let localities: Vec<crate::sim::localities::GoodLocality> = metadata::get_meta(&conn, "good_localities")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut loc_grade_sum = vec![vec![0.0f32; gc]; nn];
    let mut loc_grade_n = vec![vec![0u32; gc]; nn];
    for l in &localities {
        let Some(&g) = good_slot.get(l.good.as_str()) else { continue };
        let key = (l.y as u64) * (grid_w as u64) + l.x as u64;
        if let Some(&(_, hh)) = claim.get(&key) {
            if hh != u32::MAX {
                loc_grade_sum[hh as usize][g] += l.grade;
                loc_grade_n[hh as usize][g] += 1;
            }
        }
    }

    // quality grade per hub/good: a deposit good reads its workings' mean grade; a
    // good with a locality in the hub's catchment blends that terroir grade in
    // (D2 — "feeds quality", not a replacement, since `share` below already partly
    // reflects Slice 3's modulated belt values); everything else keeps the old
    // richer-territory-share formula. All three get a small deterministic jitter so
    // equal inputs still differ between hubs/goods.
    let mut quality = vec![vec![0.0f32; gc]; nn];
    for hh in 0..nn {
        for g in 0..gc {
            let share = prod[hh][g] / good_max[g];
            let jitter = hash01q((hh as u64).wrapping_mul(0x9E3779B1) ^ (g as u64).wrapping_mul(0x85EBCA77)) * 0.24 - 0.12;
            let share_base = 0.30 + 0.62 * share;
            let base = if is_deposit[g] && grade_n[hh][g] > 0 {
                grade_sum[hh][g] / grade_n[hh][g] as f32
            } else if loc_grade_n[hh][g] > 0 {
                let loc_mean = loc_grade_sum[hh][g] / loc_grade_n[hh][g] as f32;
                0.5 * share_base + 0.5 * loc_mean
            } else {
                share_base
            };
            quality[hh][g] = (base + jitter).clamp(0.0, 1.0);
            // apply abundance scaling to production AFTER quality is read off share
            prod[hh][g] = prod[hh][g] / good_max[g] * abundance[g];
        }
    }

    // ── Monopoly + trade power + stars ──
    let mut good_total = vec![0.0f32; gc];
    for g in 0..gc { for hh in 0..nn { good_total[g] += prod[hh][g]; } }
    let mut monopoly = vec![0.0f32; nn];
    for hh in 0..nn {
        for g in 0..gc {
            if good_total[g] < 1e-4 || prod[hh][g] < 0.05 { continue; }
            let share = prod[hh][g] / good_total[g];
            monopoly[hh] += share * share;
        }
    }
    let cmax = centrality.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mmax = monopoly.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let pop_max = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    let mut power = vec![0.0f32; nn];
    for hh in 0..nn {
        // POPULATION is now a primary driver so the greatest trade hub is a big,
        // central city — not a tiny far-flung island that merely monopolises one
        // good (the user's "largest city on a far island" bug). Population + route
        // centrality dominate; monopoly is a smaller tie-breaker.
        let pop_n = (nodes[hh].population as f32 / pop_max).clamp(0.0, 1.0).sqrt();
        let mut p = 0.26 * nodes[hh].score.clamp(0.0, 1.0)
            + 0.30 * pop_n
            + 0.30 * (centrality[hh] / cmax)
            + 0.14 * (monopoly[hh] / mmax);
        // Sea ports outgrow lake/inland towns as great hubs; outposts never rank.
        if node_sea[hh] { p *= 1.18; } else { p *= 0.72; }
        if is_outpost[hh] { p *= 0.05; }
        power[hh] = p;
    }
    let pmax = power.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // ── Demand: population-scaled per-good desire (+ mercantile/subsistence) ──
    // Unified with compute_trade_matrix: a full-basket floor, a reach factor (how
    // open the network is), a homeland discount (a good is common/cheap where it is
    // produced, so its own region barely imports it) and a wealth feedback (richer
    // markets crave luxuries more). Staples are unaffected by reach/luxury terms.
    let max_pop = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    let desire: Vec<f32> = (0..gc)
        .map(|g| specs.get(g).filter(|s| s.enabled).map(|s| s.desire).unwrap_or(0.0))
        .collect();
    let is_luxury: Vec<bool> = (0..gc)
        .map(|g| specs.get(g).map(|s| s.network_luxury).unwrap_or(false))
        .collect();
    let lux_mult = (0.6 + luxury_bias.clamp(0.0, 1.0) * 1.4).clamp(0.3, 2.2);
    let reach_factor = match reach { 0 => 1.0, 1 => 0.7, _ => 0.45 };
    // Income proxy (pre-flow): value of what a hub produces. Richer hubs demand more
    // luxuries. Normalized 0..1.
    let income: Vec<f32> = (0..nn)
        .map(|hh| (0..gc).map(|g| prod[hh][g] * desire[g]).sum::<f32>())
        .collect();
    let inc_max = income.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    const BASKET_FLOOR: f32 = 0.35; // every good is at least modestly desired
    let mut net = vec![vec![0.0f32; gc]; nn];
    let mut demand_v = vec![vec![0.0f32; gc]; nn]; // per-hub per-good demand (for the class/luxury panel)
    for hh in 0..nn {
        let size = (nodes[hh].population as f32 / max_pop).max(0.25);
        // Prosperity: a wealthy city is a hungry market. Demand across the WHOLE
        // basket (not just luxuries) scales with the city's income/production
        // value, so richer hubs import more of everything (user request: "the more
        // wealthy the city is, the more demand"). A poor town gets ~0.55× the base,
        // a rich metropolis ~1.2×.
        let income_n = (income[hh] / inc_max).clamp(0.0, 1.0);
        let prosperity = size * (0.55 + 0.65 * income_n);
        for g in 0..gc {
            let mut d = prosperity * desire[g].max(BASKET_FLOOR);
            if is_luxury[g] {
                // `prod` is already rescaled to 0..abundance (max producer ≈ the
                // good's abundance), so use it directly as the local-share input —
                // identical to compute_trade_matrix's homeland discount.
                let homeland_discount = 1.0 - 0.6 * prod[hh][g].clamp(0.0, 1.0);
                // ELITE-DEMAND SYSTEM: luxuries are conspicuous consumption. A city's
                // appetite for them scales SUPER-LINEARLY with its prosperity — a
                // rich metropolis has a large patrician/court class that craves silk,
                // spices, gems and wine far out of proportion to its size, while a
                // poor town buys almost none. (income_n^1.4 → 0.35 poor … ~2.0 rich.)
                let income_n = (income[hh] / inc_max).clamp(0.0, 1.0);
                let elite = 0.35 + 1.65 * income_n.powf(1.4);
                d *= reach_factor * lux_mult * homeland_discount * elite;
            }
            demand_v[hh][g] = d;
            net[hh][g] = prod[hh][g] - d;
        }
    }

    // ── Market equilibrium (Part III): stock-based local prices in the GRAIN
    // numeraire replace per-hop markup compounding. The solver runs over the
    // SAME trade graph; freight is an additive cost per travel day, so
    // remoteness becomes a decaying price gradient instead of a capped
    // multiplier (the old ×9.6 plateau). ──
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let edge_days: Vec<f32> = edge_paths
        .iter()
        .map(|p| path_metrics(&cc, p, km_per_cell).1)
        .collect();
    // All-pairs travel DAYS over the graph (same Dijkstra as gdist, day weights).
    let mut gdays: Vec<Vec<f32>> = vec![vec![f32::INFINITY; nn]; nn];
    for src in 0..nn {
        gdays[src][src] = 0.0;
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, src)));
        let mut done = vec![false; nn];
        while let Some(Reverse((_, u))) = heap.pop() {
            if done[u] { continue; }
            done[u] = true;
            let du = gdays[src][u];
            for &(v, _, eid) in &adj[u] {
                let nd = du + edge_days[eid];
                if nd < gdays[src][v] {
                    gdays[src][v] = nd;
                    heap.push(Reverse(((nd * 1000.0) as i64, v)));
                }
            }
        }
    }
    // Intern good categories. The solver's food convention: ids 0..=3 are
    // cereal/protein/oil/sweetener (they feed grain_wealth).
    fn intern_cat(cat_ids: &mut Vec<String>, c: &str) -> usize {
        if c.is_empty() {
            return usize::MAX;
        }
        if let Some(i) = cat_ids.iter().position(|x| x == c) {
            i
        } else {
            cat_ids.push(c.to_string());
            cat_ids.len() - 1
        }
    }
    let mut cat_ids: Vec<String> =
        ["cereal", "protein", "oil", "sweetener"].iter().map(|s| s.to_string()).collect();
    let mgoods: Vec<market::MarketGood> = (0..gc)
        .map(|g| market::MarketGood {
            category: intern_cat(&mut cat_ids, &specs[g].category),
            need_tier: specs[g].need_tier,
            base_value: specs[g].base_value.max(0.05),
            desire: if specs[g].enabled { specs[g].desire.max(0.0) } else { 0.0 },
            bulk: specs[g].bulk.max(0.0),
            perishable: specs[g].perishable.max(0.0),
        })
        .collect();
    // Production chains: cities transform imported raws into finished exports
    // (wool→cloth, ore→arms). Run the shared resolver on the static per-hub
    // production BEFORE the market solves, so manufactured goods flow as exports
    // from populous hubs that hold the inputs.
    let hub_pop: Vec<f32> = (0..nn).map(|hh| nodes[hh].population.max(1) as f32).collect();
    let _manu_warnings =
        crate::sim::manufacture::apply_manufacturing(&mut prod, &specs, &hub_pop);
    let mhubs: Vec<market::MarketHub> = (0..nn)
        .map(|hh| market::MarketHub {
            population: hub_pop[hh],
            production: prod[hh].clone(),
        })
        .collect();
    let mut routes_m = market::RouteMatrix::new(nn);
    for a in 0..nn {
        for b in (a + 1)..nn {
            if comp[a] == comp[b] && gdays[a][b].is_finite() {
                routes_m.set(a, b, gdays[a][b], 0.0);
            }
        }
    }
    let mparams = market::MarketParams::default();
    let mut mkt = market::solve(&mhubs, &mgoods, &routes_m, &mparams);

    // GUILD manufacture from IMPORTS (the demand→production→raw-demand loop).
    // The first `apply_manufacturing` only consumed each hub's LOCAL production.
    // Now that the market has delivered raws, a hub's post-trade STOCK holds the
    // wool / salt / ore it imported, so its manufacture guilds can turn those into
    // finished exports too — e.g. a herring port that imports SALT now cures
    // SALTED HERRING, and the demand for it pulls more salt + herring in. We
    // manufacture from post-trade stock, credit the extra finished output to
    // production, and re-solve ONCE so the new goods flow out.
    {
        let mut avail: Vec<Vec<f32>> = mkt.stocks.clone();
        let before: Vec<Vec<f32>> = avail.clone();
        crate::sim::manufacture::apply_manufacturing(&mut avail, &specs, &hub_pop);
        let mut changed = false;
        for hh in 0..nn {
            for g in 0..gc {
                if matches!(specs[g].distribution, crate::sim::goods_spec::Distribution::Manufactured) {
                    let extra = avail[hh][g] - before[hh].get(g).copied().unwrap_or(0.0);
                    if extra > 1e-3 {
                        prod[hh][g] += extra;
                        changed = true;
                    }
                }
            }
        }
        if changed {
            let mhubs2: Vec<market::MarketHub> = (0..nn)
                .map(|hh| market::MarketHub { population: hub_pop[hh], production: prod[hh].clone() })
                .collect();
            mkt = market::solve(&mhubs2, &mgoods, &routes_m, &mparams);
        }
    }

    // Climate at each hub cell (for the merchant-story narrative + price notes).
    let node_koppen: Vec<u8> = nodes.iter().map(|s| {
        let t = world.tile((s.x / TILE_SIZE) as i32, (s.y / TILE_SIZE) as i32);
        let ti = ((s.y % TILE_SIZE) * TILE_SIZE + (s.x % TILE_SIZE)) as usize;
        t.koppen.get(ti).copied().unwrap_or(0)
    }).collect();
    // Toll hubs: the great sea markets (top territory_n that are sea ports) levy a
    // transit toll on goods passing THROUGH them, the way Venice / Constantinople did.
    let is_toll_hub: Vec<bool> = (0..nn).map(|h| h < territory_n && node_sea[h]).collect();
    // Narrow-water STRAIT cells (opposing land within `strait_r`) over the coarse cost
    // grid — a route threading one of these pays a strait toll (Sound Dues, etc.).
    let strait_r = 6i32;
    let strait_cell: Vec<bool> = {
        let land_within = |cx: i32, cy: i32, dirx: i32, diry: i32| -> bool {
            for k in 1..=strait_r {
                let nx = cx + dirx * k;
                let ny = cy + diry * k;
                if ny < 0 || ny >= cc.ch { return false; }
                if cc.is_land[cc.cidx(nx, ny)] { return true; }
            }
            false
        };
        let mut sc = vec![false; (cc.cw * cc.ch) as usize];
        for cy in 0..cc.ch {
            for cx in 0..cc.cw {
                let ci = (cy * cc.cw + cx) as usize;
                if cc.is_land[ci] { continue; }
                let ew = land_within(cx, cy, -1, 0) && land_within(cx, cy, 1, 0);
                let ns = land_within(cx, cy, 0, -1) && land_within(cx, cy, 0, 1);
                if ew || ns { sc[ci] = true; }
            }
        }
        sc
    };

    // ── Flows over the trade GRAPH → chains with per-hop prices + chokepoints ──
    // A deficit buys a good from the cheapest-by-ROUTE supplier in the SAME network
    // component (so the origin is always genuinely reachable over real roads). The
    // goods ride the actual route network (settlement → hub → … → consumer); price
    // rises per hop; and import demand is PRICE-ELASTIC — dear, far-hauled goods are
    // bought in smaller volume, so a market only reaches overseas for what it truly
    // cannot get nearer.
    let mut chains: Vec<EconChain> = Vec::new();
    // Per-hub outbound export legs (origin → consumer) for the export %/arrows panel.
    let mut export_acc: Vec<Vec<(usize, usize, f32, u32)>> = vec![Vec::new(); nn]; // (good, to_hub, amt, chain)
    let mut receives: Vec<Vec<EconReceive>> = vec![Vec::new(); nn];
    let mut recv_amt = vec![vec![0.0f32; gc]; nn]; // per-hub per-good received (for the panel)
    let mut exports = vec![0.0f32; nn];
    let mut imports = vec![0.0f32; nn];
    let mut edge_vol: std::collections::HashMap<(usize, usize), (f32, usize)> =
        std::collections::HashMap::new(); // coarse-edge volume + dominant good

    // Travel scale + directional corridor accumulation for the good-flow panel.
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let mut transit = vec![0.0f32; nn]; // goods passing THROUGH a hub (emporium score)
    #[derive(Default)]
    struct CorridorAcc {
        fwd_value: f32,
        bwd_value: f32,
        fwd_good: std::collections::HashMap<usize, f32>,
        bwd_good: std::collections::HashMap<usize, f32>,
        days: f32,
        km: f32,
        mode: u8,
    }
    let mut corridor_acc: std::collections::HashMap<(usize, usize), CorridorAcc> =
        std::collections::HashMap::new();

    // The solver's shipment lanes ARE the chains: each flow rides the real
    // route graph; the per-stop price ladder is the ADDITIVE delivered cost
    // (origin grain-eq price + freight per travel day, × tolls) shown as a
    // multiplier vs the origin — replacing per-hop markup compounding.
    let mut lanes: Vec<&market::MarketFlow> =
        mkt.flows.iter().filter(|fl| fl.amount > 0.01).collect();
    // Largest lanes first per good, capped so the snapshot stays legible.
    lanes.sort_by(|a, b| {
        a.good.cmp(&b.good).then(
            b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut lanes_per_good = vec![0u32; gc];

    for fl in lanes {
        let (si, di, g, amt) = (fl.from, fl.to, fl.good, fl.amount);
        if lanes_per_good[g] >= 220 {
            continue;
        }
        if !gdist[di][si].is_finite() {
            continue;
        }
        // Rebuild the graph path si → di (the real road the goods travel).
        // gpar[di] is the shortest-path tree rooted at di, so walking from
        // si via gpar[di][·] yields si … di directly.
        let mut path: Vec<usize> = Vec::new();
        let mut t = si;
        while t != usize::MAX {
            path.push(t);
            if t == di { break; }
            t = gpar[di][t];
        }
        if path.last() != Some(&di) {
            continue;
        }
        lanes_per_good[g] += 1;
        exports[si] += amt;
        imports[di] += amt;
        recv_amt[di][g] += amt;

        let chain_id = chains.len() as u32;
        // Goods passing THROUGH intermediate hubs make those hubs emporia.
        for k in 1..path.len().saturating_sub(1) { transit[path[k]] += amt; }
        let mut cum_km = 0.0f32;
        let mut cum_days = 0.0f32;
        let mut modes = [0u32; 3];
        let mut chain_stops: Vec<EconChainStop> = Vec::with_capacity(path.len());
        let mut points: Vec<[f32; 2]> = Vec::with_capacity(path.len());
        let origin_price = mkt.prices[si][g].max(1e-3);
        let mut delivered = origin_price; // grain-eq, accrues freight + tolls
        let last_k = path.len().saturating_sub(1);
        for (k, &h) in path.iter().enumerate() {
            let (mut leg_km, mut leg_days, mut leg_mode) = (0.0f32, 0.0f32, 0u8);
            let (mut freight_k, mut toll_k, mut crossed_strait) = (0.0f32, 0.0f32, false);
            if k > 0 {
                let prev = path[k - 1];
                if let Some(&(_, _, eid)) = adj[prev].iter().find(|&&(v, _, _)| v == h) {
                    if let Some(p) = edge_paths.get(eid) {
                        for win in p.windows(2) {
                            let e = (win[0].min(win[1]), win[0].max(win[1]));
                            edge_vol.entry(e).or_insert((0.0, g)).0 += amt;
                        }
                        if p.iter().any(|&c| strait_cell[c]) { crossed_strait = true; }
                        let (lk, ld, lm) = path_metrics(&cc, p, km_per_cell);
                        leg_km = lk; leg_days = ld; leg_mode = lm;
                        cum_km += lk; cum_days += ld; modes[lm as usize] += 1;
                    }
                }
                // Additive freight for this leg, then tolls on the value so far.
                freight_k = mparams.freight_per_day * leg_days;
                delivered += freight_k;
                if crossed_strait { toll_k += 0.30; }                 // strait dues
                if k < last_k && is_toll_hub[h] { toll_k += 0.20; }   // great-hub transit toll
                delivered *= 1.0 + toll_k;
            }
            // Unmet local need still reads as a "demand spike" in the story,
            // but it no longer multiplies the price — cost does.
            let demand_spike = if k > 0 { (mkt.unmet[h][g] * 0.45).clamp(0.0, 0.9) } else { 0.0 };
            let note = if k == 0 {
                "bought at source".to_string()
            } else {
                let mut parts: Vec<&str> = vec!["freight"];
                if crossed_strait { parts.push("strait toll"); }
                if k < last_k && is_toll_hub[h] { parts.push("hub transit toll"); }
                if demand_spike > 0.25 { parts.push("high local demand"); }
                parts.join(" · ")
            };
            chain_stops.push(EconChainStop {
                hub: h as u32,
                price: delivered / origin_price, // multiplier vs origin (UI compat)
                days: cum_days, km: cum_km,
                markup: freight_k / origin_price, // the leg's freight, as a fraction
                toll: toll_k, demand_spike,
                koppen: node_koppen[h], note,
            });
            points.push([nodes[h].x as f32, nodes[h].y as f32]);
            // Directional corridor cargo, valued at the delivered grain-eq cost.
            if k > 0 {
                let prev = path[k - 1];
                let (na, nb) = (prev.min(h), prev.max(h));
                let value_here = amt * delivered;
                let acc = corridor_acc.entry((na, nb)).or_default();
                if prev < h {
                    acc.fwd_value += value_here;
                    *acc.fwd_good.entry(g).or_insert(0.0) += value_here;
                } else {
                    acc.bwd_value += value_here;
                    *acc.bwd_good.entry(g).or_insert(0.0) += value_here;
                }
                acc.days = leg_days; acc.km = leg_km; acc.mode = leg_mode;
            }
        }
        let mult = chain_stops.last().map(|st| st.price).unwrap_or(1.0);
        let total_days = chain_stops.last().map(|st| st.days).unwrap_or(0.0);
        let total_km = chain_stops.last().map(|st| st.km).unwrap_or(0.0);
        let dom_mode = (0..3usize).max_by_key(|&i| modes[i]).unwrap_or(0) as u8;
        receives[di].push(EconReceive {
            good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
            amount: amt, price: mult, chain: chain_id, from_hub: si as u32,
        });
        chains.push(EconChain {
            id: chain_id, good: g,
            good_name: goods_names.get(g).cloned().unwrap_or_default(),
            stops: chain_stops, points,
            days: total_days, km: total_km, value: amt * delivered, mode: dom_mode,
        });
        export_acc[si].push((g, di, amt, chain_id));
    }

    // ── Export destinations + shares per hub ──
    let mut exports_to_v: Vec<Vec<EconExport>> = vec![Vec::new(); nn];
    for si in 0..nn {
        // Total exported per good from this hub (for the percentage).
        let mut good_tot: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for &(g, _, amt, _) in &export_acc[si] { *good_tot.entry(g).or_insert(0.0) += amt; }
        let mut v: Vec<EconExport> = export_acc[si].iter().map(|&(g, di, amt, ch)| {
            let tot = good_tot.get(&g).copied().unwrap_or(amt).max(1e-6);
            EconExport {
                good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                to_hub: di as u32, amount: amt, pct: (amt / tot * 100.0).clamp(0.0, 100.0), chain: ch,
            }
        }).collect();
        v.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        exports_to_v[si] = v;
    }
    // Per-(hub,good) exported amount (for good_stats top-exporter).
    let mut exp_by: Vec<Vec<f32>> = vec![vec![0.0f32; gc]; nn];
    for si in 0..nn { for &(g, _, amt, _) in &export_acc[si] { exp_by[si][g] += amt; } }

    // ── Shortages: goods a hub demands but can't fully obtain, with the reason ──
    // Reuses the component map + graph distances so the cause is accurate: produced
    // nowhere reachable, blocked across water, no port for a sea-borne good, or simply
    // out-produced by demand.
    let mut shortages_v: Vec<Vec<ShortageNote>> = vec![Vec::new(); nn];
    for hh in 0..nn {
        if is_outpost[hh] { continue; }
        let mut notes: Vec<(f32, ShortageNote)> = Vec::new();
        for g in 0..gc {
            let dmd = demand_v[hh][g];
            if dmd < 0.08 { continue; }
            if net[hh][g] >= 0.0 { continue; } // self-sufficient or a net producer
            let got = recv_amt[hh][g];
            let severity = (1.0 - got / dmd).clamp(0.0, 1.0);
            if severity < 0.45 { continue; }
            // Is there any reachable supplier (same component, finite graph distance)?
            let reachable = (0..nn).any(|si| si != hh && net[si][g] > 0.02
                && comp[si] == comp[hh] && gdist[hh][si].is_finite());
            let any_producer = (0..nn).any(|si| prod[si][g] > 0.02);
            let reason = if !any_producer { "no_supplier" }
                else if !reachable { if node_sea[hh] { "unreachable" } else { "no_port" } }
                else { "deficit" };
            notes.push((severity * (0.4 + desire[g]), ShortageNote {
                good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                reason: reason.to_string(), severity,
            }));
        }
        notes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        shortages_v[hh] = notes.into_iter().take(6).map(|(_, n)| n).collect();
    }

    // ── Per-good world statistics: top importer/exporter + which class craves it ──
    let good_stats: Vec<GoodStat> = (0..gc).filter_map(|g| {
        let top_imp = (0..nn).filter(|&h| recv_amt[h][g] > 0.0)
            .max_by(|&a, &b| recv_amt[a][g].partial_cmp(&recv_amt[b][g]).unwrap_or(std::cmp::Ordering::Equal));
        let top_exp = (0..nn).filter(|&h| exp_by[h][g] > 0.0)
            .max_by(|&a, &b| exp_by[a][g].partial_cmp(&exp_by[b][g]).unwrap_or(std::cmp::Ordering::Equal));
        let top_dmd = (0..nn).max_by(|&a, &b| demand_v[a][g].partial_cmp(&demand_v[b][g]).unwrap_or(std::cmp::Ordering::Equal));
        if top_imp.is_none() && top_exp.is_none() { return None; }
        // Which social class most desires this good: luxuries are the nobility's
        // conspicuous consumption; high-value trade goods the merchants'; bulk the
        // commoners'.
        let class = if is_luxury[g] { "nobility" }
            else if desire[g] >= 0.5 { "merchants" } else { "commoners" };
        Some(GoodStat {
            good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
            top_importer: top_imp.map(|h| h as i32).unwrap_or(-1),
            top_exporter: top_exp.map(|h| h as i32).unwrap_or(-1),
            biggest_desire_hub: top_dmd.map(|h| h as i32).unwrap_or(-1),
            biggest_desire_class: class.to_string(),
        })
    }).collect();

    // ── Wealth per hub: grain wealth (food security) + trade wealth (market
    // earnings) from the equilibrium, plus a small centrality term so pure
    // crossroads towns don't read as paupers. Normalized for the UI. ──
    let mut wealth = vec![0.0f32; nn];
    for hh in 0..nn {
        let m = &mkt.hubs[hh];
        wealth[hh] = m.grain_wealth.max(0.0)
            + 1.5 * m.trade_wealth.max(0.0)
            + 0.25 * (centrality[hh] / cmax);
    }
    let wmax = wealth.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // ── Directional corridors (hub↔hub) for the good-flow panel ──
    // Each corridor carries goods both ways; the overlay draws ONE net-direction
    // arrow per corridor (so direction only changes at hubs), and the cargo popup
    // reads the two per-direction good lists into side-by-side columns.
    let mut corridors: Vec<EconCorridor> = Vec::new();
    for ((na, nb), acc) in corridor_acc.into_iter() {
        let mut fwd_goods: Vec<CorridorGood> = acc.fwd_good.into_iter()
            .map(|(g, v)| CorridorGood { good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(), value: v })
            .collect();
        fwd_goods.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        let mut bwd_goods: Vec<CorridorGood> = acc.bwd_good.into_iter()
            .map(|(g, v)| CorridorGood { good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(), value: v })
            .collect();
        bwd_goods.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        corridors.push(EconCorridor {
            a: na as u32, b: nb as u32,
            points: vec![[nodes[na].x as f32, nodes[na].y as f32], [nodes[nb].x as f32, nodes[nb].y as f32]],
            fwd_value: acc.fwd_value, bwd_value: acc.bwd_value,
            fwd_goods, bwd_goods, days: acc.days, km: acc.km, mode: acc.mode,
        });
    }

    // ── Emporia: the few hubs the most goods are routed THROUGH (pass-through
    // transit volume). These are flagged so the Trade Hub layer can mark them red
    // as the "most important" hubs. Top ~8, and only ones with real throughput. ──
    let emporium_set: std::collections::HashSet<usize> = {
        let mut e: Vec<(f32, usize)> = (0..nn)
            .filter(|&h| node_sea[h] && !is_outpost[h]) // emporia are sea ports, never lake towns/outposts
            .map(|h| (transit[h], h))
            .filter(|&(v, _)| v > 0.0).collect();
        e.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        e.into_iter().take(8).map(|(_, h)| h).collect()
    };

    // Per-hub trade-statistics for the click window.
    let throughput_v: Vec<f32> = (0..nn).map(|h| exports[h] + imports[h] + transit[h]).collect();
    let tp_max = throughput_v.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let trade_max = (0..nn).map(|h| exports[h] + imports[h]).fold(0.0f32, f32::max).max(1e-6);
    // Monopolies: goods this hub is the dominant producer of (≥45% of world output).
    let monopolies_v: Vec<Vec<String>> = (0..nn).map(|hh| {
        let mut m: Vec<(f32, usize)> = (0..gc)
            .filter(|&g| good_total[g] > 1e-4 && prod[hh][g] > 0.1
                && prod[hh][g] / good_total[g] >= 0.45)
            .map(|g| (prod[hh][g] / good_total[g], g)).collect();
        m.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        m.into_iter().take(4).map(|(_, g)| goods_names.get(g).cloned().unwrap_or_default()).collect()
    }).collect();

    // ── Emit hubs ──
    let mut hubs: Vec<EconHub> = Vec::with_capacity(nn);
    for hh in 0..nn {
        let p = power[hh] / pmax;
        let stars = if p >= 0.80 { 5 } else if p >= 0.60 { 4 }
            else if p >= 0.42 { 3 } else if p >= 0.25 { 2 } else { 1 };
        let mut produces: Vec<EconHubGood> = (0..gc)
            .filter(|&g| prod[hh][g] > 0.05)
            .map(|g| {
                let q = quality[hh][g];
                let id = goods_names.get(g).cloned().unwrap_or_default();
                let flavor = good_flavor(&id, q);
                EconHubGood {
                    good: g, good_name: id, amount: prod[hh][g], quality: q,
                    grade: grade_name(q).to_string(), flavor,
                    // Local equilibrium price in grain-equivalent (quality
                    // nudges the realized sale price a little).
                    price: mkt.prices[hh][g] * (0.9 + 0.25 * q),
                }
            }).collect();
        produces.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        let mut recv = receives[hh].clone();
        recv.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));

        // ── Social classes: a population split driven by wealth and trade. The
        // wealthy/patrician class grows with the hub's relative wealth; the merchant
        // class with its trade activity (route centrality + total trade volume).
        let pop = nodes[hh].population as f32;
        let wealth_n = (wealth[hh] / wmax).clamp(0.0, 1.0);
        let cent_n = (centrality[hh] / cmax).clamp(0.0, 1.0);
        let trade_n = ((exports[hh] + imports[hh]) / trade_max).clamp(0.0, 1.0);
        let f_nob = 0.003 + 0.030 * wealth_n;            // 0.3%..3.3% of the populace
        let f_mer = 0.010 + 0.060 * cent_n + 0.040 * trade_n; // ~1%..11%
        let nobility = (pop * f_nob) as u32;
        let merchants = (pop * f_mer) as u32;
        let commoners = nodes[hh].population.saturating_sub(nobility).saturating_sub(merchants);
        // The good that brings the city the most WEALTH = production × local price
        // (volume × value), plus its share of the hub's total export value.
        let mut tv_best: (String, f32) = (String::new(), 0.0);
        let mut tv_total = 0.0f32;
        for g in 0..gc {
            if prod[hh][g] > 0.05 {
                let v = prod[hh][g] * mkt.prices[hh][g].max(0.01);
                tv_total += v;
                if v > tv_best.1 {
                    tv_best = (goods_names.get(g).cloned().unwrap_or_default(), v);
                }
            }
        }
        let top_export = tv_best.0;
        let top_export_share = if tv_total > 0.0 { tv_best.1 / tv_total } else { 0.0 };
        // Luxury market: demand vs what actually arrives + delivered price. Price
        // rises when little gets through (scarcity from hard-to-move routes).
        let mut luxuries: Vec<HubLuxury> = (0..gc)
            .filter(|&g| is_luxury[g] && demand_v[hh][g] > 0.04)
            .map(|g| {
                // Delivered multiplier if it arrives by trade, else the local
                // equilibrium price relative to the world standard (scarce here
                // → well above 1×).
                let price = receives[hh].iter().find(|r| r.good == g).map(|r| r.price)
                    .unwrap_or_else(|| {
                        (mkt.prices[hh][g] / specs[g].base_value.max(0.05)).max(1.0)
                    });
                HubLuxury {
                    good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    demand: demand_v[hh][g], received: recv_amt[hh][g], price,
                }
            }).collect();
        luxuries.sort_by(|a, b| b.demand.partial_cmp(&a.demand).unwrap_or(std::cmp::Ordering::Equal));
        luxuries.truncate(8);

        // ── Market panel: prices in grain-eq, barter ratios, currency goods ──
        let market_panel = if is_outpost[hh] { None } else {
            let m = &mkt.hubs[hh];
            // This hub's top exports (by outbound value) are what imports are
            // bartered against.
            let mut top_exp: Vec<(usize, f32)> = (0..gc)
                .filter(|&g| exp_by[hh][g] > 0.01 || prod[hh][g] > 0.2)
                .map(|g| (g, exp_by[hh][g].max(prod[hh][g] * 0.2) * mkt.prices[hh][g]))
                .collect();
            top_exp.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_exp.truncate(3);
            // The most market-relevant goods here: traded, produced, or notably
            // mispriced vs the world standard.
            let mut relevant: Vec<(usize, f32)> = (0..gc)
                .filter(|&g| specs[g].enabled)
                .map(|g| {
                    let dev = (mkt.prices[hh][g] / specs[g].base_value.max(0.05) - 1.0).abs();
                    let activity = (recv_amt[hh][g] + exp_by[hh][g]) * mkt.prices[hh][g];
                    (g, activity + dev * specs[g].base_value.sqrt())
                })
                .filter(|&(_, score)| score > 0.02)
                .collect();
            relevant.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            relevant.truncate(16);
            let prices_v: Vec<HubMarketGood> = relevant.iter().map(|&(g, _)| {
                let exchanged_for: Vec<ExchangeRate> = top_exp.iter()
                    .filter(|&&(eg, _)| eg != g)
                    .map(|&(eg, _)| ExchangeRate {
                        good_name: goods_names.get(eg).cloned().unwrap_or_default(),
                        // Units of the counter-good one unit of g buys here.
                        ratio: mkt.prices[hh][g] / mkt.prices[hh][eg].max(1e-3),
                    })
                    .collect();
                HubMarketGood {
                    good: g,
                    good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    price: mkt.prices[hh][g],
                    base_value: specs[g].base_value,
                    in_flow: recv_amt[hh][g],
                    out_flow: exp_by[hh][g],
                    exchanged_for,
                }
            }).collect();
            Some(HubMarket {
                grain_wealth: m.grain_wealth,
                trade_wealth: m.trade_wealth,
                currency_goods: m.currency_goods.iter()
                    .map(|c| goods_names.get(c.good).cloned().unwrap_or_default())
                    .collect(),
                currencies: m.currency_goods.iter()
                    .map(|c| HubCurrency {
                        good: c.good,
                        name: goods_names.get(c.good).cloned().unwrap_or_default(),
                        liquidity: c.liquidity,
                        value: c.value,
                        stability: c.stability,
                        price: mkt.prices[hh][c.good],
                    })
                    .collect(),
                prices: prices_v,
            })
        };

        hubs.push(EconHub {
            id: hh as u32,
            x: nodes[hh].x as f32,
            y: nodes[hh].y as f32,
            name: crate::sim::names::gen_name_epithet(
                nodes[hh].x, nodes[hh].y, grid_w, grid_h,
                if stars >= 5 { 2 } else if stars >= 4 { 1 } else { 0 }),
            power: p,
            stars,
            wealth: wealth[hh] / wmax,
            population: nodes[hh].population,
            emporium: emporium_set.contains(&hh),
            koppen: {
                let (hx, hy) = (nodes[hh].x, nodes[hh].y);
                let t = world.tile((hx / TILE_SIZE) as i32, (hy / TILE_SIZE) as i32);
                let ti = ((hy % TILE_SIZE) * TILE_SIZE + (hx % TILE_SIZE)) as usize;
                t.koppen.get(ti).copied().unwrap_or(0)
            },
            elevation: {
                let (hx, hy) = (nodes[hh].x, nodes[hh].y);
                let t = world.tile((hx / TILE_SIZE) as i32, (hy / TILE_SIZE) as i32);
                let ti = ((hy % TILE_SIZE) * TILE_SIZE + (hx % TILE_SIZE)) as usize;
                t.elevation.get(ti).copied().unwrap_or(0.0)
            },
            coastal: {
                let (hx, hy) = (nodes[hh].x, nodes[hh].y);
                let t = world.tile((hx / TILE_SIZE) as i32, (hy / TILE_SIZE) as i32);
                let ti = ((hy % TILE_SIZE) * TILE_SIZE + (hx % TILE_SIZE)) as usize;
                t.distance_to_ocean.get(ti).map(|&d| d < 0.06).unwrap_or(false)
            },
            nobility,
            merchants,
            commoners,
            elite_level: wealth_n,
            merchant_level: (cent_n * 0.5 + trade_n * 0.5),
            top_export,
            top_export_share,
            luxuries,
            sea_access: node_sea[hh],
            exports_to: exports_to_v[hh].clone(),
            shortages: shortages_v[hh].clone(),
            throughput: throughput_v[hh],
            exports: exports[hh],
            imports: imports[hh],
            partners: centrality[hh] as u32,
            ref_pct: (throughput_v[hh] / tp_max * 100.0).clamp(0.0, 100.0),
            nearest_ref: nearest_ref_hub((throughput_v[hh] / tp_max * 100.0).clamp(0.0, 100.0)).to_string(),
            monopolies: monopolies_v[hh].clone(),
            market: market_panel,
            produces,
            receives: recv,
        });
    }

    // ── Strategic chokepoints: geographic STRAITS + emporium CITIES ──
    // Two clean categories instead of arbitrary busy edges. A strait is a narrow
    // water cell with land close on two opposing sides that routed volume threads;
    // an emporium is a hub city a large share of routed goods passes THROUGH.
    let mut cell_vol = vec![0.0f32; (cc.cw * cc.ch) as usize];
    for ((a, b), (vol, _)) in &edge_vol {
        cell_vol[*a] += *vol;
        cell_vol[*b] += *vol;
    }
    let strait_r = 6i32;
    let land_within = |cx: i32, cy: i32, dirx: i32, diry: i32| -> bool {
        for k in 1..=strait_r {
            let nx = cx + dirx * k;
            let ny = cy + diry * k;
            if ny < 0 || ny >= cc.ch { return false; }
            if cc.is_land[cc.cidx(nx, ny)] { return true; }
        }
        false
    };
    let mut straits: Vec<(f32, usize)> = Vec::new();
    for cy in 0..cc.ch {
        for cx in 0..cc.cw {
            let ci = (cy * cc.cw + cx) as usize;
            if cc.is_land[ci] { continue; }
            let v = cell_vol[ci];
            if v <= 0.0 { continue; }
            let ew = land_within(cx, cy, -1, 0) && land_within(cx, cy, 1, 0);
            let ns = land_within(cx, cy, 0, -1) && land_within(cx, cy, 0, 1);
            if ew || ns { straits.push((v, ci)); }
        }
    }
    straits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let sep = (grid_w as f32 * 0.04).max(8.0);
    let sep2 = sep * sep;
    let mut cps: Vec<(f32, [f32; 2], String)> = Vec::new();
    for (v, ci) in straits {
        let wpt = cc.world_of(ci);
        let far = cps.iter().all(|(_, p, _)| {
            let mut dx = (p[0] - wpt[0]).abs();
            if dx > grid_w as f32 / 2.0 { dx = grid_w as f32 - dx; }
            let dy = p[1] - wpt[1];
            dx * dx + dy * dy >= sep2
        });
        if far { cps.push((v, wpt, "Strait".to_string())); }
        if cps.len() >= 8 { break; }
    }
    // Emporium cities: hubs ranked by pass-through (transit) volume.
    let mut emp: Vec<(f32, usize)> = (0..nn)
        .map(|h| (transit[h], h)).filter(|&(v, _)| v > 0.0).collect();
    emp.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (v, h) in emp.into_iter().take(8) {
        cps.push((v, [nodes[h].x as f32, nodes[h].y as f32], hubs[h].name.clone()));
    }
    cps.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    cps.truncate(12);
    let top_vol = cps.first().map(|c| c.0).unwrap_or(1.0).max(1e-6);
    let chokepoints: Vec<EconChokepoint> = cps.into_iter().map(|(v, p, name)| EconChokepoint {
        points: vec![p, p], volume: v, share: v / top_vol, name,
    }).collect();

    // ── Trade-region territories: group owned coarse cells per hub ──
    let mut region_cells: Vec<Vec<[f32; 2]>> = vec![Vec::new(); nn];
    for cy in 0..ch {
        for cx in 0..cw {
            let o = owner[(cy * cw + cx) as usize];
            if o == u16::MAX { continue; }
            region_cells[o as usize].push([(cx as u32 * f) as f32, (cy as u32 * f) as f32]);
        }
    }
    let mut regions: Vec<EconRegion> = Vec::new();
    for hh in 0..nn {
        if region_cells[hh].is_empty() { continue; }
        regions.push(EconRegion {
            hub: hh as u32,
            name: hubs[hh].name.clone(),
            cells: std::mem::take(&mut region_cells[hh]),
            cell_size: f as f32,
        });
    }

    // ── Statistics by node class: trade hubs vs emporiums vs local trade posts ──
    // Computed from the emitted hubs so the figures match what's drawn.
    let class_stats: Vec<ClassStats> = {
        let agg = |pred: &dyn Fn(usize) -> bool, label: &str| -> ClassStats {
            let idxs: Vec<usize> = (0..nn).filter(|&h| pred(h)).collect();
            let count = idxs.len() as u32;
            let population: u64 = idxs.iter().map(|&h| nodes[h].population as u64).sum();
            let throughput: f32 = idxs.iter().map(|&h| throughput_v[h]).sum();
            let avg_wealth = if count > 0 {
                idxs.iter().map(|&h| wealth[h] / wmax).sum::<f32>() / count as f32
            } else { 0.0 };
            ClassStats { label: label.to_string(), count, population, throughput, avg_wealth }
        };
        vec![
            agg(&|h| !is_outpost[h] && emporium_set.contains(&h), "emporiums"),
            agg(&|h| !is_outpost[h] && !emporium_set.contains(&h), "hubs"),
            agg(&|h| is_outpost[h], "outposts"),
        ]
    };

    let snapshot = EconomySnapshot { hubs, chains, chokepoints, regions, corridors, good_stats, class_stats, goods: goods_names, colonizable_sites };
    let _ = metadata::campaign_set(&conn, "economy", &serde_json::to_string(&snapshot).unwrap_or_default());
    Ok(snapshot)
}


/// Export the economy snapshot to `path`. `.json` writes the raw snapshot;
/// anything else writes a flat CSV (one row per hub × good, produce + receive).
#[tauri::command]
pub fn export_trade_data(path: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let json = metadata::campaign_get_or_meta(&conn, "economy").map_err(|e| e.to_string())?
        .ok_or_else(|| "No economy snapshot — run the Economy step (10) first.".to_string())?;
    if path.to_lowercase().ends_with(".json") {
        return std::fs::write(&path, json).map_err(|e| e.to_string());
    }
    let snap: EconomySnapshot = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let esc = |v: &str| -> String {
        if v.contains(',') || v.contains('"') || v.contains('\n') {
            format!("\"{}\"", v.replace('"', "\"\""))
        } else { v.to_string() }
    };
    let mut s = String::from("hub,x,y,stars,wealth,population,good,role,amount,quality,grade,flavor,price,counterparty\n");
    for h in &snap.hubs {
        for p in &h.produces {
            let row = [
                esc(&h.name), h.x.to_string(), h.y.to_string(), h.stars.to_string(),
                format!("{:.3}", h.wealth), h.population.to_string(),
                esc(&p.good_name), "produce".to_string(), format!("{:.3}", p.amount),
                format!("{:.3}", p.quality), esc(&p.grade), esc(&p.flavor),
                format!("{:.3}", p.price), String::new(),
            ];
            s.push_str(&row.join(",")); s.push('\n');
        }
        for r in &h.receives {
            let from = snap.hubs.iter().find(|x| x.id == r.from_hub)
                .map(|x| x.name.clone()).unwrap_or_default();
            let row = [
                esc(&h.name), h.x.to_string(), h.y.to_string(), h.stars.to_string(),
                format!("{:.3}", h.wealth), h.population.to_string(),
                esc(&r.good_name), "receive".to_string(), format!("{:.3}", r.amount),
                String::new(), String::new(), String::new(),
                format!("{:.3}", r.price), esc(&from),
            ];
            s.push_str(&row.join(",")); s.push('\n');
        }
    }
    std::fs::write(&path, s).map_err(|e| e.to_string())
}


/// Read the persisted economy snapshot (empty if none has been generated yet).
#[tauri::command]
pub fn get_economy(db: State<'_, WorldDb>) -> Result<EconomySnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let goods_names: Vec<String> = crate::commands::goods_commands::load_world_goods(&conn)
        .iter().map(|s| s.id.clone()).collect();
    match metadata::campaign_get_or_meta(&conn, "economy").map_err(|e| e.to_string())? {
        Some(json) => serde_json::from_str(&json)
            .map_err(|_| ()).or_else(|_| Ok::<_, String>(EconomySnapshot {
                hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![], corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names.clone(), colonizable_sites: vec![],
            })),
        None => Ok(EconomySnapshot { hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![], corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names, colonizable_sites: vec![] }),
    }
}
