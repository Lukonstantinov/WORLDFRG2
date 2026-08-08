//! lifecycle commands — split from the former monolithic campaign_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Freeze the world's geography and record the fingerprint campaigns reference.
#[tauri::command]
pub fn finalize_world(db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let fp = world_fingerprint(&conn)?;
    metadata::set_meta(&conn, "frozen", "1").map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "finalized_fp", &format!("{},{}", fp.0, fp.1))
        .map_err(|e| e.to_string())?;
    Ok(())
}


/// Lift the freeze (geography becomes editable; campaigns made on the previous
/// finalized state will report a world mismatch when opened).
#[tauri::command]
pub fn unfreeze_world(db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "frozen", "0").map_err(|e| e.to_string())?;
    Ok(())
}


/// Start a fresh campaign on the (finalized) current world.
#[tauri::command]
pub fn new_campaign(name: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    if !is_frozen(&conn) {
        return Err("Finalize the world before starting a campaign.".into());
    }
    let world_ref = current_world_ref(&conn)?;
    conn.execute("DELETE FROM campaign", [])
        .map_err(|e| e.to_string())?;
    // Drop the resident sim so a fresh campaign truly starts from scratch — otherwise
    // the running-campaign restart guard in `campaign_start_sim` would see the stale
    // cached sim (tick > 0) and refuse to reseed.
    {
        let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
        cache.sim = None;
        cache.loaded = true; // don't reload the just-deleted row from the DB
        cache.dirty = false;
        cache.last_persist = None;
    }
    metadata::campaign_set(&conn, "name", &name).map_err(|e| e.to_string())?;
    metadata::campaign_set(
        &conn,
        "world_ref",
        &serde_json::to_string(&world_ref).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}


/// Write the current campaign to its own SQLite file (small: a handful of JSON
/// rows + the world reference).
#[tauri::command]
pub fn save_campaign_as(path: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Flush any resident in-memory ticks to the DB row first, so the file we copy
    // out reflects the latest simulated state.
    persist_campaign(&db, &conn)?;
    // Refresh the world reference so the file always names the world it was
    // saved against.
    let world_ref = current_world_ref(&conn)?;
    metadata::campaign_set(
        &conn,
        "world_ref",
        &serde_json::to_string(&world_ref).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let dest = Connection::open(&path).map_err(|e| e.to_string())?;
    dest.execute_batch(
        "DROP TABLE IF EXISTS campaign;
         CREATE TABLE campaign (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .map_err(|e| e.to_string())?;
    for key in CAMPAIGN_KEYS {
        if let Some(v) = metadata::campaign_get(&conn, key).map_err(|e| e.to_string())? {
            dest.execute(
                "INSERT OR REPLACE INTO campaign (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, v],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}


/// Load a `.campaign` file against the currently open world. The campaign data
/// is copied in regardless; `world_match` tells the frontend whether to warn
/// (different world, or the world was re-finalized since).
#[tauri::command]
pub fn open_campaign(path: String, db: State<'_, WorldDb>) -> Result<CampaignInfo, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let src = Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| e.to_string())?;

    let mut rows: Vec<(String, String)> = Vec::new();
    {
        let mut stmt = src
            .prepare("SELECT key, value FROM campaign")
            .map_err(|_| "Not a campaign file (no campaign table).".to_string())?;
        let mut q = stmt.query([]).map_err(|e| e.to_string())?;
        while let Some(row) = q.next().map_err(|e| e.to_string())? {
            rows.push((row.get(0).map_err(|e| e.to_string())?, row.get(1).map_err(|e| e.to_string())?));
        }
    }

    let file_ref: Option<WorldRef> = rows
        .iter()
        .find(|(k, _)| k == "world_ref")
        .and_then(|(_, v)| serde_json::from_str(v).ok());
    let world_match = match (&file_ref, current_world_ref(&conn).ok()) {
        (Some(fr), Some(cur)) => {
            fr.fingerprint == cur.fingerprint
                && fr.grid_width == cur.grid_width
                && fr.grid_height == cur.grid_height
        }
        _ => false,
    };

    conn.execute("DELETE FROM campaign", [])
        .map_err(|e| e.to_string())?;
    for (k, v) in &rows {
        metadata::campaign_set(&conn, k, v).map_err(|e| e.to_string())?;
    }
    // The resident sim now belongs to the old campaign — drop it so the next read
    // reloads from the freshly imported rows.
    db.invalidate_campaign();

    let name = rows
        .iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "Campaign".to_string());
    let campaign_progress = rows
        .iter()
        .find(|(k, _)| k == "campaign_progress")
        .map(|(_, v)| v.clone());
    Ok(CampaignInfo { name, world_match, campaign_progress })
}


/// Persist wizard progress. `scope` = "world" (steps 1-6, stored in metadata so
/// it travels with the world file) or "campaign" (steps 7-10).
#[tauri::command]
pub fn set_progress(scope: String, progress_json: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    match scope.as_str() {
        "world" => metadata::set_meta(&conn, "world_progress", &progress_json).map_err(|e| e.to_string()),
        "campaign" => metadata::campaign_set(&conn, "campaign_progress", &progress_json).map_err(|e| e.to_string()),
        other => Err(format!("Unknown progress scope: {other}")),
    }
}


/// Appearance palette (user-customized overlay/line colours) persisted in the
/// world `metadata` so it travels with the `.worldforge` file — a shared world
/// then looks the same for everyone. Stored as the sparse-override JSON the
/// frontend `settingsStore` produces (only keys that differ from the defaults).
#[tauri::command]
pub fn set_appearance(appearance_json: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "appearance", &appearance_json).map_err(|e| e.to_string())
}


/// Read the saved appearance override (None if the world never customized it).
#[tauri::command]
pub fn get_appearance(db: State<'_, WorldDb>) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    metadata::get_meta(&conn, "appearance").map_err(|e| e.to_string())
}


/// Frontend-invokable flush — called when the Play loop pauses and before the app
/// closes, so unsaved in-memory ticks reach the DB.
#[tauri::command]
pub fn campaign_persist(db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    persist_campaign(&db, &conn)
}


/// Seed a fresh living-trade simulation from the static economy snapshot.
#[tauri::command]
pub fn campaign_start_sim(seed: u64, db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // A RUNNING campaign is NEVER restarted/clobbered. If a sim has already advanced,
    // return it unchanged — starting a fresh game is the explicit New-Campaign action
    // (`campaign_new_game`), which preserves the current run in its own file first.
    if let Some(existing) = get_sim(&db, &conn)? {
        if existing.tick > 0 {
            return Ok(build_snapshot(&existing));
        }
    }
    crate::sim::cultures::ensure_active(&conn); // seed house names in their local culture
    let econ_json = metadata::campaign_get(&conn, "economy")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Build the economy (step 10) before starting the campaign sim.".to_string())?;
    let econ: EconomySnapshot =
        serde_json::from_str(&econ_json).map_err(|e| format!("economy parse: {e}"))?;
    if econ.hubs.len() < 2 {
        return Err("Need at least two trade hubs to run the economy.".into());
    }
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let world_ref = current_world_ref(&conn)?;
    let grid_w = world_ref.grid_width.max(1) as f32;

    // ── Goods ── (food = the solver's reserved cereal/protein/oil/sweetener cats)
    let gc = econ.goods.len();
    let food_cats = ["cereal", "protein", "oil", "sweetener"];
    let mut cat_ids: Vec<String> = food_cats.iter().map(|s| s.to_string()).collect();
    // good id → column index, for resolving Manufactured recipe inputs to columns.
    let id_to_col: std::collections::HashMap<&str, usize> =
        econ.goods.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let goods: Vec<TickGood> = (0..gc)
        .map(|g| {
            let spec = specs.get(g);
            let cat = spec.map(|s| s.category.clone()).unwrap_or_default();
            let category = if cat.is_empty() {
                i32::MAX
            } else if let Some(i) = cat_ids.iter().position(|x| *x == cat) {
                i as i32
            } else {
                cat_ids.push(cat.clone());
                (cat_ids.len() - 1) as i32
            };
            let food = category != i32::MAX && (category as usize) < food_cats.len();
            // Resolve recipe inputs (by good id) to (column, qty); drop unknowns.
            let inputs: Vec<(usize, f32)> = spec
                .map(|s| s.inputs.iter()
                    .filter_map(|inp| id_to_col.get(inp.good.as_str()).map(|&c| (c, inp.qty.max(0.0))))
                    .collect())
                .unwrap_or_default();
            TickGood {
                name: econ.goods[g].clone(),
                category,
                need_tier: spec.map(|s| s.need_tier).unwrap_or(1),
                base_value: spec.map(|s| s.base_value).filter(|v| *v > 0.0).unwrap_or(1.0),
                desire: spec.map(|s| s.desire).unwrap_or(0.4),
                food,
                fungible_input: crate::sim::manufacture::is_fungible_input_category(&cat),
                bulk: spec.map(|s| s.bulk).filter(|v| *v > 0.0).unwrap_or(1.0),
                perishable: spec.map(|s| s.perishable.max(0.0)).unwrap_or(0.0),
                inputs,
                labor: spec.map(|s| s.labor).filter(|v| *v > 0.0).unwrap_or(1.0),
                consumption_interval: spec.map(|s| s.consumption_interval).filter(|v| *v > 0.0).unwrap_or(30.0),
            }
        })
        .collect();

    // ── Hubs ── (cap to the strongest 250 to bound tick cost + state size). Raised
    // 150→250 so more notable cities are actually simulated — a "large" city ranked
    // 151+ was rendered but not a live hub, so it couldn't be clicked/inspected.
    let mut order: Vec<usize> = (0..econ.hubs.len()).collect();
    order.sort_by(|&a, &b| econ.hubs[b].population.cmp(&econ.hubs[a].population));
    // DECOUPLE: settlements ranked below the live cap aren't simulated, but they're
    // still real places — captured as `hinterland` so they stay drawn/clickable AND
    // their population is counted in the world census (fixes the Atlas undercount).
    let overflow: Vec<usize> = order.iter().skip(250).copied().collect();
    order.truncate(250);
    order.sort_unstable(); // keep snapshot index order stable
    // Tiered founding populations (user rule): every settlement starts HUMBLE and
    // grows — small towns begin at 500, medium at 2000, large at 10000 — and the
    // map's settlement icon then scales with live population as the city rises or
    // falls. Buckets are by worldgen-population RANK so the biggest sites start
    // "large". Balance-neutral: `base_per_capita` is derived from THIS founding
    // pop below, so day-0 production is unchanged and per-capita output stays
    // consistent as population changes.
    let mut ranked_pops: Vec<u32> = order.iter().map(|&h| econ.hubs[h].population).collect();
    ranked_pops.sort_unstable_by(|a, b| b.cmp(a));
    let rn = ranked_pops.len().max(1);
    let large_cut = ranked_pops[((rn as f32 * 0.15) as usize).min(rn - 1)]; // top 15% → large
    let medium_cut = ranked_pops[((rn as f32 * 0.50) as usize).min(rn - 1)]; // next 35% → medium
    let tier_founding = |p: u32| -> f32 {
        if p >= large_cut { 10_000.0 } else if p >= medium_cut { 2_000.0 } else { 500.0 }
    };
    let id_to_idx: std::collections::HashMap<u32, usize> =
        order.iter().enumerate().map(|(i, &h)| (econ.hubs[h].id, i)).collect();

    // Inert hinterland towns (below the sim cap) — same humble founding scale as the
    // live hubs so the world census is consistent. Counted + clickable, not simulated.
    let hinterland_towns: Vec<crate::sim::tick::HinterlandTown> = overflow
        .iter()
        .map(|&hi| {
            let eh = &econ.hubs[hi];
            crate::sim::tick::HinterlandTown {
                x: eh.x, y: eh.y, name: eh.name.clone(),
                population: tier_founding(eh.population),
                koppen: eh.koppen, coastal: eh.coastal,
                parent_hub: -1,
            }
        })
        .collect();

    let mut hubs: Vec<TickHub> = order
        .iter()
        .map(|&hi| {
            let eh = &econ.hubs[hi];
            let mut production = vec![0.0f32; gc];
            for p in &eh.produces {
                if p.good < gc {
                    production[p.good] += p.amount;
                }
            }
            // The static economy emits NORMALISED [0,1] outputs; lift them into
            // readable per-day quantities. Uniform & balance-neutral: need_scale,
            // stock and prices all scale with it, so only the displayed magnitudes
            // change (trade stops looking like "≈0").
            for v in production.iter_mut() { *v *= 1000.0; }
            // FOUNDING (autarky) prices. We deliberately do NOT seed prices from the
            // static `compute_economy` market solve: that solve is a single frozen
            // trade-geography *preview*, not the campaign's economic reality. The
            // campaign must derive prices live — the founding world hasn't traded yet,
            // so every hub starts at each good's intrinsic base_value and the tick's
            // stock-driven price relax (tick.rs) then discovers real, trade-driven
            // prices over the first weeks of sim. (Substrate crosses the boundary —
            // city sites, productive potential, routes — but economics is re-derived.)
            let price: Vec<f32> = goods.iter().map(|g| g.base_value).collect();
            let founding = tier_founding(eh.population);
            // Per-capita production: the static economy's output is for the founding
            // population, so production = base_per_capita · population thereafter.
            let base_per_capita: Vec<f32> = production.iter().map(|&p| p / founding).collect();
            TickHub {
                id: eh.id,
                x: eh.x,
                y: eh.y,
                name: eh.name.clone(),
                population: founding,
                founding_pop: founding,
                stock: production.clone(), // seed one tick of stock so prices start sane
                price,
                production,
                grain_wealth: 0.0,
                trade_wealth: 0.0,
                food_balance: 1.0,
                starving: 0.0,
                is_estate: false,
                parent: -1,
                koppen: eh.koppen,
                coastal: eh.coastal,
                component: 0,
                export_earn: 0.0,
                import_spend: 0.0,
                mood: 0.6,
                sent_food: 0.7,
                sent_prosperity: 0.5,
                sent_stability: 0.8,
                civic_pool: 0.0,
                history: Vec::new(),
                in_by_sea: 0.0,
                in_by_land: 0.0,
                base_per_capita,
                lack_basic: 0.0,
                lack_comfort: 0.0,
                lack_luxury: 0.0,
                society: crate::sim::tick::Society::default(),
                pops: Vec::new(),
                tw_house: 0.0,
                tw_local: 0.0,
                tw_guild: 0.0,
                estate_kind: 0,
                estate_tier: 0,
                last_upgrade_tick: 0,
                owner_house: -1,
                stake_bank: -1,
                stake_share: 0.0,
                damage: 0.0,
                structures: vec![],
                treasury: 0.0,
                tariff_export: 0.0,
                tariff_import: 0.0,
                mint_fineness: 1.0,
                council_house: -1,
                finance: crate::sim::tick::CityFinance::default(),
                war_with: -1,
                war_since: 0,
                war_effort: 0.0,
                tribute_to: -1,
                tribute_until: 0,
                coin_name: String::new(),
                coin_trust: 0.0,
                settle_coin: -1,
                coin_basket: Vec::new(),
                mint_fineness_prev: 0.0,
                price_level: 1.0,
                coin_circ_prev: 0.0,
                last_reform_tick: 0,
                reform_until: 0,
                coin_metal: 0,
                coin_history: Vec::new(),
                debt_principal: 0.0,
                debt_coupon: 0.0,
                debt_holders: Vec::new(),
                mint_bullion_ratio: 1.0,
                has_mint: false,
                quality: Vec::new(),
                stolen_good: -1,
                stolen_from: -1,
                colony_kind: 0,
                colony_stage: 0,
                autonomous: false,
                founder_hub: -1,
                backers: Vec::new(),
                reserve_food: 0.0,
                reserve_cap: 0.0,
                supply_years: 0.0,
                colony_founded_tick: 0,
                main_bank: -1,
                indep_cooldown_until: 0,
                plague_immune_until: 0,
                public_health: 0.0,
                supply_ships: 0,
                supply_source: -1,
                supply_delivered: 0.0,
                transit_year: 0.0,
                hub_class: 0,
                class_momentum: 0,
                build_stage: 0,
                build_progress: 0.0,
                build_supply: [0.0; 3],
                build_supply_good: [0; 3],
                build_idle_months: 0,
                build_convoys: 0,
                build_start_tick: 0,
                govt_type: 0,
                officials: Vec::new(),
                civic_goods: Vec::new(),
                laws: Vec::new(),
                captor_house: -1,
                // Atlas 2.0 lifecycle: primordial (worldgen) settlements.
                abandoned: false,
                decline_years: 0.0,
                founded_tick: 0,
                died_tick: 0,
                trade_last_year: 0.0,
                died_cause: String::new(),
                tier: 0,
                standing: 0.0,
                war_cooldown_until: 0,
            }
        })
        .collect();
    let nn = hubs.len();

    // ── Connectivity components: TOTAL TRADE within a landmass ────────────────
    // Components were previously derived ONLY from realized worldgen flows
    // (corridors + chains), so any city the static solve never shipped through —
    // typically INLAND towns — became a singleton component and could NEVER trade
    // (rebuild_routes marks every cross-component pair unreachable). We now build
    // components from geographic reachability so every settlement on a landmass
    // shares one trading market (the code's intended "continents = components"),
    // while distinct continents / remote islands stay separate. Campaign-only —
    // worldgen, compute_economy and the trade overlays are untouched.
    let mut parent: Vec<usize> = (0..nn).collect();
    // Keep the worldgen trade lanes as links (the sea routes the static solve found).
    for c in &econ.corridors {
        if let (Some(&ia), Some(&ib)) = (id_to_idx.get(&c.a), id_to_idx.get(&c.b)) {
            uf_union(&mut parent, ia, ib);
        }
    }
    for ch in &econ.chains {
        for w in ch.stops.windows(2) {
            if let (Some(&ia), Some(&ib)) = (id_to_idx.get(&w[0].hub), id_to_idx.get(&w[1].hub)) {
                uf_union(&mut parent, ia, ib);
            }
        }
    }
    // Geographic K-nearest union: each hub links to its nearest neighbours within a
    // max single-hop distance (≈30% of world width, mirroring the worldgen link
    // ceiling). Transitive chaining then fuses a whole continent into one component
    // regardless of its size, but a wide OCEAN gap (> the cap) is never bridged, so
    // separate continents / far islands remain their own markets.
    {
        const COMP_K: usize = 6;
        let world_w = world_ref.grid_width as f32;
        let max_link = (world_w * 0.30).max(1.0);
        let max_link2 = max_link * max_link;
        let d2 = |a: usize, b: usize| -> f32 {
            let mut dx = (hubs[a].x - hubs[b].x).abs();
            if world_w > 1.0 { dx = dx.min(world_w - dx); } // cylindrical wrap on X
            let dy = hubs[a].y - hubs[b].y;
            dx * dx + dy * dy
        };
        let real: Vec<usize> = (0..nn).filter(|&i| !hubs[i].is_estate).collect();
        let mut scratch: Vec<(usize, f32)> = Vec::with_capacity(real.len());
        for &i in &real {
            scratch.clear();
            for &j in &real {
                if j != i { scratch.push((j, d2(i, j))); }
            }
            scratch.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            for &(j, dd) in scratch.iter().take(COMP_K) {
                if dd <= max_link2 { uf_union(&mut parent, i, j); }
            }
        }
        // Rescue tiny/lone components: a settlement whose cluster has < 3 real hubs
        // can't sustain a market and appears as a dead "cosmetic" dot that never
        // trades (rebuild_routes marks it unreachable). Fuse each into the nearest
        // SUBSTANTIAL market (any distance) so every city is on a trading network.
        {
            let mut roots = vec![0usize; nn];
            for i in 0..nn { roots[i] = uf_find(&mut parent, i); }
            let mut size: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
            for &i in &real { *size.entry(roots[i]).or_default() += 1; }
            let big: Vec<usize> = real.iter().cloned()
                .filter(|&i| size.get(&roots[i]).copied().unwrap_or(0) >= 3).collect();
            if !big.is_empty() {
                let mut unions: Vec<(usize, usize)> = Vec::new();
                for &i in &real {
                    if size.get(&roots[i]).copied().unwrap_or(0) >= 3 { continue; }
                    let mut bj = None; let mut bd = f32::INFINITY;
                    for &j in &big { let d = d2(i, j); if d < bd { bd = d; bj = Some(j); } }
                    if let Some(j) = bj { unions.push((i, j)); }
                }
                for (i, j) in unions { uf_union(&mut parent, i, j); }
            }
        }
    }
    for i in 0..nn {
        hubs[i].component = uf_find(&mut parent, i) as u32;
    }

    // ── Balance factor so total need ≈ total production ──
    let total_pop: f32 = hubs.iter().map(|h| h.population).sum::<f32>().max(1.0);
    let total_prod: f32 = hubs.iter().flat_map(|h| h.production.iter()).sum::<f32>().max(1e-3);
    let tier_w = [1.0f32, 0.45, 0.22];
    let sum_tw_desire: f32 = goods
        .iter()
        .map(|g| tier_w[g.need_tier.min(2) as usize] * g.desire.max(0.0))
        .sum::<f32>()
        .max(1e-3);
    let need_scale = total_prod / (total_pop * sum_tw_desire);

    // ── Founding food-viability constraint ────────────────────────────────────
    // This is NOT a warm-start fudge to match the static solve — it is a physical
    // founding condition: a settlement would not exist where it cannot feed itself.
    // Because the campaign now starts at founding and evolves LIVE (no burn-in) with
    // autarky prices, the earliest years are the most fragile — a chronic food
    // deficit would snowball into world-wide famine → population collapse (the
    // 8M→1M crash) before the player could react. `need_scale` balances *total*
    // production against *total* need, but FOOD is only a fraction of all goods, so a
    // world rich in luxuries but modest in cereal could still starve. We therefore
    // measure the actual food need (same demand pressure the tick uses) vs food
    // production and, if food is short, raise every hub's food output to a viable
    // surplus. Production scales with live population thereafter, so this ratio is
    // population-invariant; tech growth then makes food steadily MORE abundant over
    // the campaign. We only ever raise food (`max(1.0)`), never cut a world that
    // already grows plenty — geography that supports a city is left untouched.
    const FOOD_SURPLUS: f32 = 1.5; // ~50% headroom for seasons, lean years, growth
    let mut total_food_need = 0.0f32;
    let mut total_food_prod = 0.0f32;
    for h in &hubs {
        for (g, tg) in goods.iter().enumerate() {
            if tg.food {
                total_food_need += h.population
                    * tier_w[tg.need_tier.min(2) as usize]
                    * tg.desire.max(0.0)
                    * need_scale
                    * crate::sim::tick::DEMAND_PRESSURE;
                total_food_prod += h.production[g];
            }
        }
    }
    if total_food_prod > 1e-3 {
        let food_scale = (total_food_need * FOOD_SURPLUS / total_food_prod).max(1.0);
        if food_scale > 1.0 {
            for h in hubs.iter_mut() {
                for (g, tg) in goods.iter().enumerate() {
                    if tg.food {
                        h.base_per_capita[g] *= food_scale;
                        h.production[g] *= food_scale;
                        h.stock[g] *= food_scale;
                    }
                }
            }
        }
    }

    // ── Merchant houses: the strongest hubs get a named trading FAMILY
    //    specializing in their top goods, with a head of family who will age,
    //    die and be succeeded over the campaign ──
    let (gw, gh) = (world_ref.grid_width, world_ref.grid_height);
    let mut hub_order: Vec<usize> = (0..nn).collect();
    hub_order.sort_by(|&a, &b| {
        hubs[b].population.partial_cmp(&hubs[a].population).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut houses: Vec<House> = Vec::new();
    // Seed houses ACROSS continents: group population-ranked hubs by connectivity
    // component and round-robin across components (largest landmass first each round)
    // so every continent gets trading families. The old "top 24 by population" seeded
    // them all on the single most-populous landmass, leaving other continents empty.
    let seed_hubs: Vec<usize> = {
        use std::collections::{BTreeMap, VecDeque};
        let mut by_comp: BTreeMap<u32, VecDeque<usize>> = BTreeMap::new();
        for &h in &hub_order {
            if hubs[h].is_estate { continue; }
            by_comp.entry(hubs[h].component).or_default().push_back(h);
        }
        let mut comp_keys: Vec<u32> = by_comp.keys().copied().collect();
        comp_keys.sort_by(|&a, &b| {
            let pa = by_comp[&a].front().map(|&h| hubs[h].population).unwrap_or(0.0);
            let pb = by_comp[&b].front().map(|&h| hubs[h].population).unwrap_or(0.0);
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut out: Vec<usize> = Vec::new();
        loop {
            let mut progressed = false;
            for &ck in &comp_keys {
                if let Some(h) = by_comp.get_mut(&ck).and_then(|q| q.pop_front()) {
                    out.push(h);
                    progressed = true;
                    if out.len() >= 24 { break; }
                }
            }
            if !progressed || out.len() >= 24 { break; }
        }
        out
    };
    // Only a FEW founding families exist at the dawn of the campaign; the rest of
    // the merchant class emerges over the first ~5 years (see HOUSE_RAMP_YEARS +
    // maybe_found_house). `seed_house_count` below is set to the FULL target so the
    // tick ramps up to it.
    // No houses at the dawn of a campaign — local merchants trade, guilds charter
    // from year 5, and only then (year 10+) do houses spin off a guild's trade.
    const INITIAL_SEED_HOUSES: usize = 0;
    for &h in seed_hubs.iter().take(INITIAL_SEED_HOUSES) {
        let mut gi: Vec<usize> = (0..gc).collect();
        gi.sort_by(|&a, &b| {
            hubs[h].production[b].partial_cmp(&hubs[h].production[a]).unwrap_or(std::cmp::Ordering::Equal)
        });
        let spec: Vec<usize> = gi.into_iter().filter(|&g| hubs[h].production[g] > 0.0).take(2).collect();
        let (hx, hy) = (hubs[h].x.max(0.0) as u32, hubs[h].y.max(0.0) as u32);
        // Globally-unique seed name: re-salt the surname until it doesn't collide
        // with an already-seeded house (expanded pools make this near-always 1 try).
        let mut name = String::new();
        for k in 0..32u64 {
            let family = crate::sim::names::gen_family_name(hx, hy, gw, gh, (h as u64) ^ k.wrapping_mul(0x9E3779B1));
            let cand = format!("House {family}");
            if !houses.iter().any(|hh: &House| hh.name == cand) { name = cand; break; }
        }
        if name.is_empty() { name = format!("House of {}", hubs[h].name); }
        let surname = name.strip_prefix("House ").unwrap_or(&name).to_string();
        let head = crate::sim::names::gen_head_name(hx, hy, gw, gh, &surname, 0x100 ^ h as u64);
        let founded = crate::sim::tick::HouseEvent {
            tick: 0, kind: "founded".into(),
            text: format!("Founded by {} in {}", head, hubs[h].name),
        };
        // Starting fleet by geography: coastal great houses are seafaring.
        let (fleet_sea, fleet_river, fleet_caravan) =
            if hubs[h].coastal { (2u32, 0u32, 1u32) } else { (0u32, 1u32, 2u32) };
        houses.push(House {
            name,
            hub: h as u32,
            wealth: 1.0,
            prestige: 0.0,
            spec,
            monopoly: vec![],
            rivals: vec![],
            generation: 1,
            events: vec![founded],
            good_profit: Vec::new(),
            good_volume: Vec::new(),
            mono50: Vec::new(),
            mono_ever: Vec::new(),
            dominant_seat: false,
            prev_wealth: 1.0,
            worst_loss: 0.0,
            fleet_sea,
            fleet_river,
            fleet_caravan,
            head_name: head,
            head_since: 0,
            // Assigned two steps below by `seed_house_lines`, once the seat's culture —
            // and so its law of inheritance — is known: the accession age a rule implies
            // is what sets the tenure (see `roll_tenure`).
            head_lifespan: 0,
            founded_tick: 0,
            political_power: 0.0,
            volume: 0.0,
            defunct: false,
            archetype: crate::sim::tick::pick_archetype(seed, h as u64),
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
            head_female: false, head_age: 0, line: Vec::new(), tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0, origin_house: -1, origin_kind: ORIGIN_NONE, crowned: false, realm: -1,
        });
    }

    // The TARGET baseline is the full seed-hub set (up to 24); only a few are
    // created now, the ramp fills the rest over the opening years.
    let houses_len = (seed_hubs.len() as u32).max(houses.len() as u32);
    let days_per_cell = ((40075.0 / grid_w) / 55.0).max(0.02); // ~55 km/day blended
    let mut sim = CampaignSim {
        seed,
        tick: 0,
        goods,
        hubs,
        in_transit: vec![],
        houses,
        active_events: vec![],
        journal: vec![JournalEntry {
            tick: 0,
            kind: "event".into(),
            hub: -1,
            good: -1,
            value: 0.0,
            text: "The age of trade begins.".into(),
        }],
        days_per_cell,
        // Higher per-day freight = dearer ship/caravan operation (#9). Lower trade
        // margin = thinner merchant markups (#9). Together with lower tariffs and
        // smaller feasts, profit is squeezed toward realistic levels.
        freight_per_day: 0.018,
        k: 0.6,
        margin: 0.035,
        need_scale,
        world_w: grid_w,
        world_h: world_ref.grid_height.max(1) as f32,
        last_tick_ms: 0.0,
        last_month_pop: 0.0,
        last_month_index: 0.0,
        seed_house_count: houses_len,
        culture_rules: Vec::new(),
        fleets_migrated: true, // new campaigns already seed fleets
        tech_factor: 1.0,
        percap_migrated: true, // hubs seeded with base_per_capita directly
        society_migrated: false, // strata seeded on first advance (seed_society)
        components_rescued: true, // start-time component build already fused tiny clusters
        house_ledger: Vec::new(),
        house_ledger_prev: Vec::new(),
        house_barred: Vec::new(),
        colonizable: econ.colonizable_sites.clone(),
        satellite_sites: vec![], // filled from tiles just after construction (below)
        hinterland: hinterland_towns,
        hub_patron: vec![],
        dev_tier: vec![],
        dev_momentum: vec![],
        base_days: vec![],
        base_n: 0,
        hub_culture: vec![],
        hub_minorities: vec![],
        estate_idle_years: vec![],
        colony_supply: Vec::new(),
        diag_shipments: 0,
        diag_by_house: 0,
        diag_by_guild: 0,
        diag_lost: 0,
        diag_volume: 0.0,
        recent_trades: vec![],
        spec_centers: vec![],
        spec_year: 0,
        spec_prev_profit: vec![],
        banks: vec![],
        crashes: vec![],
        wars: vec![],
        war_log: vec![],
        flow_year: vec![],
        flow_accum: std::collections::HashMap::new(),
        world_series: vec![],
        total_foundings: 0,
        total_abandonments: 0,
        migrations: vec![],
        migration_routes: vec![],
        creoles: vec![],
        lingua: vec![],
        culture_history: vec![],
        council_bought_month: vec![],
        good_flow_accum: vec![],
        hub_good_trade: vec![],
        year_frames: vec![],
        records: Default::default(),
        quality_migrated: false,
        days: vec![],
        neighbors: vec![],
        routes_dirty: false,
        warehouses: vec![],
        contracts: vec![],
        trade_cur: Default::default(),
        city_dominator: vec![],
        trade_last: vec![],
        trade_hist: vec![],
        figures: vec![],
        fairs: vec![],
        fairs_seeded: false,
        holy_sites: vec![],
        holy_seeded: false,
        alliances: vec![],
        guilds: vec![],
        guilds_seeded: false,
        wonders: vec![],
        epidemics: vec![],
        next_outbreak: 0,
        expansion_frozen_until: 0,
        expeditions: vec![],
        route_prospects: vec![],
        failed_expeditions: vec![],
        corridors: vec![],
        next_expedition_id: 0,
        prov_rural: vec![],
        prov_cap: vec![],
        prov_culture: vec![],
        prov_seat: vec![],
        hub_province: vec![],
        prov_net_mig: vec![],
        prov_neighbors: vec![],
        feuds: vec![],
        prov_forest: vec![],
        prov_arable: vec![],
        prov_pasture: vec![],
        prov_irrigated: vec![],
        prov_soil: vec![],
        prov_tenure: vec![],
        prov_tax: vec![],
        prov_arrears: vec![],
        prov_unrest: vec![],
        prov_surplus: vec![],
        prov_revenue: vec![],
        prov_holder: vec![],
        prov_holder_house: vec![],
        prov_works: vec![],
        prov_history: vec![],
        prov_events: vec![],
        prov_good_belt: vec![],
        prov_good_depletion: vec![],
        prov_good_yield_scale: 1.0,
        // Realms (R1). Seeded empty on EVERY campaign: a realm is proclaimed in play,
        // never generated with the world, and cannot be before `REALM_YEAR_FLOOR`.
        // `prov_realm` is sized alongside the rest of the land layer by
        // `ensure_province_land`, so it stays −1 (free) for every province here.
        realms: vec![],
        prov_realm: vec![],
    };
    // Backfill the colonization pool if the saved economy predates the feature (its
    // `colonizable_sites` deserialized to the serde default — empty). Without this a
    // campaign built on an older economy snapshot can NEVER found colonies/outposts.
    if sim.colonizable.is_empty() {
        let hub_xy: Vec<(f32, f32)> = sim.hubs.iter().map(|h| (h.x, h.y)).collect();
        if let Ok(sites) = recompute_colonizable(&db, &conn, &hub_xy) {
            sim.colonizable = sites;
        }
    }
    // Near-city satellite pool (Ostia→Rome). Always (re)computed at build — it's not
    // carried in the economy snapshot.
    if sim.satellite_sites.is_empty() {
        let hub_xy: Vec<(f32, f32)> = sim.hubs.iter().map(|h| (h.x, h.y)).collect();
        if let Ok(sites) = recompute_satellite_sites(&db, &conn, &hub_xy) {
            sim.satellite_sites = sites;
        }
    }
    // PATHFOUND routes: precompute the campaign's route-days matrix over the SAME coarse
    // cost grid the trade-route layer uses (passes / rivers / coast-hugging / sea crossings),
    // so campaign trade & migration follow real lanes, never straight lines. Best-effort —
    // on any failure `rebuild_routes` falls back to the straight-line estimate.
    {
        let hub_xy: Vec<(f32, f32)> = sim.hubs.iter().map(|h| (h.x, h.y)).collect();
        let comps: Vec<u32> = sim.hubs.iter().map(|h| h.component).collect();
        match crate::commands::query_commands::compute_route_days_matrix(
            &db, &conn, &hub_xy, &comps, sim.days_per_cell,
        ) {
            Ok(bd) if bd.len() == hub_xy.len() * hub_xy.len() => {
                sim.base_n = hub_xy.len();
                sim.base_days = bd;
            }
            _ => { /* keep Euclidean fallback */ }
        }
    }
    sim.rebuild_routes();
    sim.ensure_hub_cultures(); // seed each hub's majority people from the culture map
    // Phase 0.4 · resolve each people's LAW OF INHERITANCE once (line + division rule),
    // then open the founding head's record on every seeded house. Must run after the
    // cultures are known: the seat's rule decides the head's sex and accession age.
    sim.ensure_culture_rules();
    sim.seed_house_lines();
    sim.seed_initial_guilds(); // civic guilds for cities already ≥ 50k people
    // Provinces (Phase 2b): seed the rural reservoir from the stored partition so the
    // countryside can feed the cities. No-op when no province layer was generated.
    seed_campaign_provinces(&conn, &mut sim);
    // §2.5 · self-calibrate the goods-exploitation yield scalar against THIS world's
    // own starting production, so mean exploitation reads ≈1.0 on day one regardless
    // of world size or belt intensity (mirrors `need_scale`'s own calibration above).
    // No-op (leaves the serde-default 1.0) when no province layer was generated.
    sim.calibrate_province_good_yield();
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?; // write the fresh campaign to the DB immediately
    Ok(build_snapshot(&sim))
}

/// Load the stored province partition + raster and seed the campaign's rural
/// demography (per-province reservoir, capacity, culture, seat) and each hub's
/// province membership. Does nothing if no province layer exists — so campaigns on
/// worlds without provinces run exactly as before.
fn seed_campaign_provinces(conn: &Connection, sim: &mut CampaignSim) {
    let provs: Vec<crate::sim::provinces::Province> = match metadata::get_meta(conn, "provinces") {
        Ok(Some(s)) => serde_json::from_str(&s).unwrap_or_default(),
        _ => return,
    };
    if provs.is_empty() { return; }
    let (rw, _rh, gw, gh, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        match metadata::get_meta(conn, "province_raster") {
            Ok(Some(s)) => serde_json::from_str(&s).unwrap_or((0, 0, 0, 0, Vec::new())),
            _ => (0, 0, 0, 0, Vec::new()),
        };
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    let n = provs.iter().map(|p| p.id as usize + 1).max().unwrap_or(0);
    let mut rural = vec![0.0f32; n];
    let mut cap = vec![0.0f32; n];
    let mut culture = vec![String::new(); n];
    let mut seat = vec![[0.0f32; 2]; n];
    let mut neighbors = vec![Vec::new(); n];
    for p in &provs {
        let i = p.id as usize;
        neighbors[i] = p.neighbors.clone();
        // The land's rural CAPACITY comes from its food potential (same baseline the
        // panel shows); start the countryside partly filled so it has room to grow.
        cap[i] = (p.rural_pop as f32).max(50.0);
        rural[i] = cap[i] * 0.6;
        culture[i] = p.culture.clone();
        seat[i] = [p.seat_x as f32, p.seat_y as f32];
    }
    // Map each existing hub to its province via the raster (exact); fall back to
    // nearest seat when the raster is missing. This MUST match the downsample cap
    // `sim_generate_provinces` used to build the raster (sim_commands.rs, `cap =
    // 768u32`) — a mismatched cap recomputes the wrong step and silently misindexes
    // most hubs into the wrong (or no) province, leaving the true province empty of
    // members so `province_demography_pass` never migrates anyone into/out of it.
    let step = if gw == 0 { 1 } else { ((gw.max(gh) + 767) / 768).max(1) };
    let hub_prov: Vec<i32> = sim.hubs.iter().map(|h| {
        if !raster.is_empty() && gw > 0 && gh > 0 {
            let hx = (h.x.max(0.0) as u32).min(gw - 1);
            let hy = (h.y.max(0.0) as u32).min(gh - 1);
            let ri = ((hy / step) * rw + (hx / step)) as usize;
            if let Some(&pid) = raster.get(ri) {
                if pid != crate::sim::provinces::NO_PROVINCE { return pid as i32; }
            }
        }
        nearest_seat(&seat, h.x, h.y, sim.world_w)
    }).collect();
    sim.prov_rural = rural;
    sim.prov_cap = cap;
    sim.prov_culture = culture;
    sim.prov_seat = seat;
    sim.hub_province = hub_prov;
    sim.prov_net_mig = vec![0.0; n];
    sim.prov_neighbors = neighbors;
    seed_province_land(&provs, n, sim);
}

/// B1 · seed each province's mutable LAND state from the geography the world half
/// already computed. `CampaignSim::ensure_province_land` has a cap-based fallback for
/// provinces that arrive without this (a mid-campaign partition, an older save), but
/// the real Köppen/fertility/aridity figures give a far better starting landscape —
/// a wet temperate valley starts wooded, a steppe starts open, and neither is a guess.
fn seed_province_land(provs: &[crate::sim::provinces::Province], n: usize, sim: &mut CampaignSim) {
    let mut forest = vec![0.0f32; n];
    let mut arable = vec![0.0f32; n];
    let mut pasture = vec![0.0f32; n];
    let mut soil = vec![0.6f32; n];
    let mut tenure = vec![[0.18f32, 0.10, 0.09, 0.63]; n];
    // §2.5 · the frozen per-(province, good) belt score, flat `n * ng`. `good_belt`
    // is indexed identically to `sim.goods` (both trace back to `load_world_goods`),
    // so no remapping is needed — a province generated on an older world (no
    // `good_belt`, serde-defaulted to empty) simply seeds zeros here, and the
    // exploitation tracker's own early-return on an empty layer takes it from there.
    let ng = sim.goods.len();
    let mut good_belt = vec![0.0f32; n * ng];
    for p in provs {
        let i = p.id as usize;
        if i >= n { continue; }
        // Aridity is the first control on tree cover; the Köppen main class carries the
        // rest (A wet-forested, B open, C/D forested, E barren).
        let arid = p.arid_frac.clamp(0.0, 1.0);
        let main = p.koppen_shares.first().map(|(k, _)| *k).unwrap_or(p.koppen);
        let climate_wood = match main {
            // Codes are Köppen zone ids; group by the class letter the id falls in.
            k if k <= 3 => 0.78,   // A — tropical
            k if k <= 8 => 0.16,   // B — arid
            k if k <= 17 => 0.62,  // C — temperate
            k if k <= 29 => 0.66,  // D — continental
            _ => 0.06,             // E — polar / highland
        };
        let fert = p.mean_fertility.clamp(0.0, 1.0);
        let upland = match p.elevation_class { 0 => 1.0, 1 => 0.85, _ => 0.6 };
        forest[i] = (climate_wood * (1.0 - 0.7 * arid) * upland).clamp(0.02, 0.85);
        // Cleared land at campaign start scales with how much of the province the
        // countryside already works — good land is already partly under the plough.
        arable[i] = (0.06 + 0.34 * fert * (1.0 - arid)).clamp(0.02, 0.45)
            .min((1.0 - forest[i]).max(0.02));
        pasture[i] = ((1.0 - forest[i] - arable[i]).max(0.0) * 0.55).clamp(0.0, 1.0);
        soil[i] = (0.42 + 0.50 * fert).clamp(0.30, 0.95);
        // A province with a big seat city has more of its land already in private and
        // civic hands; a frontier is mostly common.
        let settled = (p.settlements.len() as f32 / 4.0).clamp(0.0, 1.0);
        tenure[i] = [0.14 + 0.12 * settled, 0.06 + 0.14 * settled, 0.08, 0.0];
        tenure[i][3] = (1.0 - tenure[i][0] - tenure[i][1] - tenure[i][2]).clamp(0.0, 1.0);
        if ng > 0 {
            let src = &p.good_belt;
            for g in 0..ng {
                good_belt[i * ng + g] = src.get(g).copied().unwrap_or(0.0);
            }
        }
    }
    sim.prov_forest = forest;
    sim.prov_arable = arable;
    sim.prov_pasture = pasture;
    sim.prov_irrigated = vec![0.0; n];
    sim.prov_soil = soil;
    sim.prov_tenure = tenure;
    sim.prov_tax = vec![0.12; n];
    sim.prov_arrears = vec![0.0; n];
    sim.prov_unrest = vec![0.0; n];
    sim.prov_good_belt = good_belt;
    sim.prov_good_depletion = vec![0.0; n * ng];
    sim.prov_surplus = vec![0.0; n];
    sim.prov_revenue = vec![0.0; n];
    sim.prov_holder = vec![-1; n];
    sim.prov_holder_house = vec![-1; n];
    sim.prov_history = vec![Vec::new(); n];
    sim.prov_events = vec![Vec::new(); n];
}

/// Nearest province seat to (x,y), cylindrical in X. -1 if no seats.
fn nearest_seat(seat: &[[f32; 2]], x: f32, y: f32, world_w: f32) -> i32 {
    let mut best = (-1i32, f32::INFINITY);
    for (i, s) in seat.iter().enumerate() {
        let mut dx = (s[0] - x).abs();
        if world_w > 1.0 && dx > world_w / 2.0 { dx = world_w - dx; }
        let d = dx * dx + (s[1] - y) * (s[1] - y);
        if d < best.1 { best = (i as i32, d); }
    }
    best.0
}


/// COLD START: zero the just-started campaign's entire economic superstructure (houses,
/// guilds, banks, coinage, warehouses, contracts, wealth, institutions) and reset every
/// city to a small seed population, so that on unpause the world builds its trade network,
/// finance and cities up from nothing — the "press a button to zero everything, then watch
/// it grow" flow. Only valid on a fresh, unadvanced campaign (tick 0).
#[tauri::command]
pub fn campaign_cold_start(db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim_arc = get_sim(&db, &conn)?
        .ok_or_else(|| "No campaign has been started to cold-start.".to_string())?;
    let mut sim = (*sim_arc).clone();
    if sim.tick != 0 {
        return Err("Cold Start can only be applied to a fresh, unadvanced campaign — \
                    start a new game first.".into());
    }
    sim.apply_cold_start();
    sim.rebuild_routes();
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?;
    Ok(build_snapshot(&sim))
}


/// Start a FRESH dynamic campaign on the SAME finalized world/economy — a "new game".
/// A running campaign is never restarted in place; the caller must first SAVE the
/// current run to its own `.campaign` file (so it's preserved). This then clears just
/// the resident/persisted sim (keeping the economy so the new game is immediately
/// dynamic) and reseeds with a fresh seed.
#[tauri::command]
pub fn campaign_new_game(seed: u64, db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
        cache.sim = None;
        cache.loaded = true; // don't reload the old sim from the DB row
        cache.dirty = false;
        cache.last_persist = None;
        metadata::campaign_set(&conn, "campaign_sim", "").map_err(|e| e.to_string())?;
    }
    // tick == 0 path now → `campaign_start_sim` reseeds fresh (the guard won't fire).
    campaign_start_sim(seed, db)
}


/// Advance the living-trade sim by `ticks` days. The sim is mutated in place in the
/// resident cache (no JSON round-trip per call) and flushed to the DB only on a year
/// boundary or every few seconds of wall time — see `CampaignCache`.
// `async` so Tauri runs the (heavy) tick on a worker thread instead of the main
// thread — a synchronous command would block the UI event loop for the whole
// advance, which is what made long campaigns feel frozen. There are no `.await`
// points, so the std `MutexGuard`s never cross one (the future stays `Send`).
#[tauri::command]
pub async fn campaign_advance(ticks: u32, db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let ticks_run = ticks.clamp(1, 3650);

    // Phase 1 — ensure the sim is resident (needs conn+campaign, in lock order) and
    // read the autosave-cadence flags, then release BOTH locks. The heavy compute
    // below deliberately does NOT hold `conn`, so conn-dependent commands (tile
    // rendering) stay responsive while a long batch runs.
    let (stale, year_before) = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
        ensure_campaign_loaded(&mut cache, &conn)?;
        // Self-heal the colonization pool BEFORE the tick if it has run dry — an old
        // save that predates the feature, or a long campaign that consumed its
        // one-time pool, would otherwise be stuck forever at "0 colony sites in
        // range". Gated on a low floor so the tile recompute runs rarely. We hold
        // `conn` here (needed for tiles); the heavy tick below still releases it.
        let need_sites = cache.sim.as_ref()
            .map(|s| s.colonizable.len() < COLONIZE_POOL_FLOOR).unwrap_or(false);
        if need_sites {
            let hub_xy: Vec<(f32, f32)> = cache.sim.as_ref().unwrap()
                .hubs.iter().map(|h| (h.x, h.y)).collect();
            if let Ok(sites) = recompute_colonizable(&db, &conn, &hub_xy) {
                if !sites.is_empty() {
                    if let Some(sim) = cache.sim.as_mut() {
                        std::sync::Arc::make_mut(sim).colonizable = sites;
                    }
                }
            }
        }
        // Same self-heal for the near-city SATELLITE pool (consumed as satellites are
        // founded; refilled from tiles when it drains).
        let need_sat = cache.sim.as_ref()
            .map(|s| s.satellite_sites.len() < COLONIZE_POOL_FLOOR).unwrap_or(false);
        if need_sat {
            let hub_xy: Vec<(f32, f32)> = cache.sim.as_ref().unwrap()
                .hubs.iter().map(|h| (h.x, h.y)).collect();
            if let Ok(sites) = recompute_satellite_sites(&db, &conn, &hub_xy) {
                if !sites.is_empty() {
                    if let Some(sim) = cache.sim.as_mut() {
                        std::sync::Arc::make_mut(sim).satellite_sites = sites;
                    }
                }
            }
        }
        // Wall-clock safety flush: an unattended fast-play still checkpoints even if it
        // hasn't crossed a 2-year boundary yet (kept long so the 2-year cadence leads).
        let stale = cache.last_persist.map(|t| t.elapsed().as_secs_f32() > AUTOSAVE_WALLCLOCK_SECS).unwrap_or(true);
        let year_before = cache.sim.as_ref()
            .ok_or_else(|| "No active campaign sim — start it first.".to_string())?
            .year();
        (stale, year_before)
    };

    // Phase 2 — the tick + snapshot + (maybe) serialize, holding ONLY the campaign
    // lock. RESILIENT: a panicking tick (an index/overflow bug, which the dev build
    // turns into a real panic) must NEVER stop the campaign. `advance_resilient` runs
    // the batch under `catch_unwind` and, on a fault, restores a clean checkpoint,
    // freezes territorial expansion, and — worst case — skips only the single poisoned
    // tick so the simulation ALWAYS keeps moving forward. State is preserved (never
    // discarded), so the run can always be continued. (A true allocation failure still
    // aborts — catch_unwind can't catch `abort()` — pointing at OOM, not a logic bug.)
    let t0 = std::time::Instant::now();
    let (snap, json) = {
        let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
        {
            // `make_mut` mutates in place; it only copies if a panel query holds an
            // Arc to the sim at this exact instant (rare, and then just once).
            let sim = std::sync::Arc::make_mut(cache.sim.as_mut()
                .ok_or_else(|| "No active campaign sim — start it first.".to_string())?);
            advance_resilient(sim, ticks_run);
            sim.last_tick_ms = t0.elapsed().as_secs_f32() * 1000.0 / ticks_run.max(1) as f32;
        }
        let sim = cache.sim.as_ref().expect("present after a resilient advance");
        let snap = build_snapshot(sim);
        // Serialize ONLY when it's time to autosave — every 2 sim-years (crossing an
        // even-year boundary) or the wall-clock safety; otherwise the change just stays
        // resident and dirty. `AUTOSAVE_EVERY_YEARS` = 2 per the campaign save cadence.
        let crossed_autosave = (sim.year() / AUTOSAVE_EVERY_YEARS) != (year_before / AUTOSAVE_EVERY_YEARS);
        let json = if crossed_autosave || stale {
            Some(encode_campaign_blob(sim.as_ref())?)
        } else {
            None
        };
        (snap, json)
    };

    // Phase 3 — persist (conn+campaign in lock order) or just mark dirty.
    match json {
        Some(j) => {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
            metadata::campaign_set(&conn, "campaign_sim", &j).map_err(|e| e.to_string())?;
            cache.dirty = false;
            cache.last_persist = Some(std::time::Instant::now());
        }
        None => {
            let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
            cache.dirty = true;
        }
    }
    Ok(snap)
}


/// Current sim state (inactive snapshot when no campaign sim has been started).
#[tauri::command]
pub fn campaign_get_state(db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(match get_sim(&db, &conn)? {
        Some(sim) => build_snapshot(&sim),
        None => inactive_snapshot(),
    })
}
