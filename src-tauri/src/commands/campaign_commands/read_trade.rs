//! read_trade commands — split from the former monolithic campaign_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


#[tauri::command]
pub fn campaign_get_trade_basins(db: State<'_, WorldDb>) -> Result<Vec<TradeBasin>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    if sim.flow_year.is_empty() { return Ok(vec![]); }
    if let Ok(memo) = BASINS_MEMO.lock() {
        if let Some((seed, tick, cached)) = memo.as_ref() {
            if *seed == sim.seed && *tick == sim.tick {
                return Ok(cached.clone());
            }
        }
    }
    let idx_of: std::collections::HashMap<u32, usize> = sim.hubs.iter().enumerate()
        .filter(|(_, h)| !h.is_estate && !h.abandoned && h.population >= 1.0)
        .map(|(i, h)| (h.id, i))
        .collect();
    let mut edges: std::collections::HashMap<usize, Vec<(usize, f32)>> =
        std::collections::HashMap::new();
    for &(a, b, v) in &sim.flow_year {
        let (Some(&ia), Some(&ib)) = (idx_of.get(&a), idx_of.get(&b)) else { continue };
        edges.entry(ia).or_default().push((ib, v));
        edges.entry(ib).or_default().push((ia, v));
    }
    let n = sim.hubs.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for (&i, list) in edges.iter() {
        let mut l = list.clone();
        l.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
        for &(j, _) in l.iter().take(2) {
            let (ri, rj) = (uf_find(&mut parent, i), uf_find(&mut parent, j));
            if ri != rj { parent[ri] = rj; }
        }
    }
    let mut comp: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for &i in edges.keys() {
        let r = uf_find(&mut parent, i);
        comp.entry(r).or_default().push(i);
    }
    let mut internal: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
    for &(a, b, v) in &sim.flow_year {
        let (Some(&ia), Some(&ib)) = (idx_of.get(&a), idx_of.get(&b)) else { continue };
        let (ra, rb) = (uf_find(&mut parent, ia), uf_find(&mut parent, ib));
        if ra == rb { *internal.entry(ra).or_insert(0.0) += v; }
    }
    let (w, hgt) = (sim.world_w as u32, sim.world_h());
    let ng = sim.goods.len();
    let mut basins: Vec<TradeBasin> = comp.into_iter()
        .filter(|(_, m)| m.len() >= 2)
        .map(|(root, members)| {
            let cx = members.iter().map(|&i| sim.hubs[i].x).sum::<f32>() / members.len() as f32;
            let cy = members.iter().map(|&i| sim.hubs[i].y).sum::<f32>() / members.len() as f32;
            let top = members.iter()
                .max_by(|&&a, &&b| sim.hubs[a].trade_last_year
                    .partial_cmp(&sim.hubs[b].trade_last_year).unwrap_or(std::cmp::Ordering::Equal))
                .map(|&i| sim.hubs[i].name.clone()).unwrap_or_default();
            // Top goods from the per-hub per-good yearly ledger (empty pre-ledger).
            let mut totals = vec![0.0f32; ng];
            for &i in &members {
                for g in 0..ng {
                    totals[g] += sim.hub_good_trade.get(i * ng + g).copied().unwrap_or(0.0);
                }
            }
            let mut order: Vec<usize> = (0..ng).collect();
            order.sort_by(|&x, &y| totals[y].partial_cmp(&totals[x]).unwrap_or(std::cmp::Ordering::Equal));
            let top_goods: Vec<String> = order.into_iter().take(2)
                .filter(|&g| totals[g] > 0.0)
                .map(|g| sim.goods[g].name.clone()).collect();
            TradeBasin {
                name: crate::sim::names::region_name(cx.max(0.0) as u32, cy.max(0.0) as u32, w, hgt),
                volume: internal.get(&root).copied().unwrap_or(0.0),
                hub_ids: members.iter().map(|&i| sim.hubs[i].id).collect(),
                pts: members.iter().map(|&i| [sim.hubs[i].x, sim.hubs[i].y]).collect(),
                cx, cy,
                top_city: top,
                top_goods,
            }
        })
        .collect();
    basins.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    basins.truncate(12);
    if let Ok(mut memo) = BASINS_MEMO.lock() {
        *memo = Some((sim.seed, sim.tick, basins.clone()));
    }
    Ok(basins)
}


/// Batch 1 · per-good Trade Heat: each living town's LAST-YEAR throughput of one
/// good (by name), as `[x, y, volume]` points for the heat overlay.
#[tauri::command]
pub fn campaign_get_good_heat(good: String, db: State<'_, WorldDb>) -> Result<Vec<[f32; 3]>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let ng = sim.goods.len();
    let Some(g) = sim.goods.iter().position(|x| x.name == good) else { return Ok(vec![]) };
    if sim.hub_good_trade.is_empty() { return Ok(vec![]); }
    Ok(sim.hubs.iter().enumerate()
        .filter(|(_, h)| !h.is_estate && !h.abandoned)
        .filter_map(|(i, h)| {
            let v = sim.hub_good_trade.get(i * ng + g).copied().unwrap_or(0.0);
            (v > 0.0).then(|| [h.x, h.y, v])
        })
        .collect())
}


#[tauri::command]
pub fn campaign_get_era_frame(year: u32, db: State<'_, WorldDb>) -> Result<Option<EraFrame>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let Some(frame) = sim.year_frames.iter().find(|f| f.year == year) else { return Ok(None) };
    let year_end_tick = (year + 1) * 365;
    let hubs = frame.pop.iter().enumerate()
        .filter(|(_, &p)| p >= 0.0) // -1 = estate marker
        .map(|(i, &p)| {
            let h = &sim.hubs[i];
            EraHub {
                x: h.x, y: h.y, name: h.name.clone(),
                population: p,
                trade: frame.trade.get(i).copied().unwrap_or(0.0),
                dead: (h.died_tick > 0 && h.died_tick <= year_end_tick) || p < 100.0,
                is_new: h.founded_tick > 0 && h.founded_tick <= year_end_tick
                    && year_end_tick - h.founded_tick < 15 * 365,
            }
        })
        .collect();
    Ok(Some(EraFrame { year, hubs }))
}


/// DLC 4 · every good's quality rating + produced/traded totals (the Goods window).
#[tauri::command]
pub fn campaign_get_goods(db: State<'_, WorldDb>) -> Result<Vec<GoodMarketRow>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let ng = sim.goods.len();
    let mut produced = vec![0.0f32; ng];
    let mut q_sum = vec![0.0f32; ng];
    let mut q_w = vec![0.0f32; ng];
    let mut best = vec![(0.0f32, usize::MAX); ng]; // (quality, hub idx)
    let mut n_prod = vec![0u32; ng];
    // Per good × grade-tier: produced + #producers (traded filled from cargo below).
    let mut tier_prod = vec![[0.0f32; 5]; ng];
    let mut tier_traded = vec![[0.0f32; 5]; ng];
    let mut tier_n = vec![[0u32; 5]; ng];
    for (hi, h) in sim.hubs.iter().enumerate() {
        for g in 0..ng {
            let p = h.production.get(g).copied().unwrap_or(0.0);
            if p <= 0.0 { continue; }
            let q = h.quality.get(g).copied().unwrap_or(0.0);
            produced[g] += p; q_sum[g] += q * p; q_w[g] += p; n_prod[g] += 1;
            let t = grade_tier(q);
            tier_prod[g][t] += p; tier_n[g][t] += 1;
            if q > best[g].0 { best[g] = (q, hi); }
        }
    }
    let mut traded = vec![0.0f32; ng];
    for s in &sim.in_transit {
        if s.good >= ng { continue; }
        let amt = s.amount.max(0.0);
        traded[s.good] += amt;
        // Bucket the cargo by the ORIGIN hub's grade for the good (where it was made).
        let oq = sim.hubs.get(s.from as usize).and_then(|h| h.quality.get(s.good).copied()).unwrap_or(0.0);
        tier_traded[s.good][grade_tier(oq)] += amt;
    }
    let mut out: Vec<GoodMarketRow> = (0..ng).filter(|&g| produced[g] > 0.0 || traded[g] > 0.0).map(|g| {
        let avg = if q_w[g] > 0.0 { q_sum[g] / q_w[g] } else { 0.0 };
        let best_city = if best[g].1 != usize::MAX { sim.hubs[best[g].1].name.clone() } else { String::new() };
        let grades: Vec<GradeBucket> = (0..5).rev()
            .filter(|&t| tier_prod[g][t] > 0.0 || tier_traded[g][t] > 0.0)
            .map(|t| GradeBucket {
                grade: GRADE_NAMES[t].to_string(),
                produced: tier_prod[g][t], traded: tier_traded[g][t], n_producers: tier_n[g][t],
            }).collect();
        GoodMarketRow {
            good: sim.goods[g].name.clone(),
            best_quality: best[g].0,
            best_grade: crate::sim::tick::quality_grade(best[g].0).to_string(),
            best_city,
            avg_quality: avg,
            produced: produced[g],
            traded: traded[g],
            n_producers: n_prod[g],
            manufactured: !sim.goods[g].inputs.is_empty(),
            grades,
        }
    }).collect();
    out.sort_by(|a, b| b.produced.partial_cmp(&a.produced).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}


/// Aggregate the live in-transit cargo into per-holder, per-city-pair routes for
/// the merchant map layer — so the player can see which families/guilds are
/// running which corridors and what they carry each way (round-trip info).
#[tauri::command]
pub fn campaign_merchant_routes(db: State<'_, WorldDb>) -> Result<Vec<MerchantRoute>, String> {
    use std::collections::HashMap;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    struct Agg { vol: f32, sea: bool, river: bool, out: HashMap<usize, f32>, ret: HashMap<usize, f32> }
    let mut groups: HashMap<(usize, u32, u32), Agg> = HashMap::new();
    for s in &sim.in_transit {
        if s.owner < 0 { continue; }
        let (lo, hi) = (s.from.min(s.to), s.from.max(s.to));
        let e = groups.entry((s.owner as usize, lo, hi))
            .or_insert_with(|| Agg { vol: 0.0, sea: false, river: false, out: HashMap::new(), ret: HashMap::new() });
        let amt = s.amount.max(0.0);
        e.vol += amt;
        e.sea |= s.sea;
        e.river |= s.river;
        if s.from == lo { *e.out.entry(s.good).or_insert(0.0) += amt; }
        else { *e.ret.entry(s.good).or_insert(0.0) += amt; }
    }
    let gname = |g: usize| sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
    // Estates are INTERNAL to their parent city — collapse an estate endpoint to
    // its parent so the map draws routes between cities, never to estate dots.
    let city_of = |h: u32| -> u32 {
        match sim.hubs.get(h as usize) {
            Some(x) if x.is_estate && x.parent >= 0 => x.parent as u32,
            _ => h,
        }
    };
    let hname = |h: u32| sim.hubs.get(city_of(h) as usize).map(|x| x.name.clone()).unwrap_or_default();
    let pos = |h: u32| sim.hubs.get(city_of(h) as usize).map(|x| [x.x, x.y]).unwrap_or([0.0, 0.0]);
    let sort_goods = |m: HashMap<usize, f32>| {
        let mut v: Vec<(String, f32)> = m.into_iter().map(|(g, vol)| (gname(g), vol)).collect();
        v.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    };
    let mut out: Vec<MerchantRoute> = groups.into_iter().map(|((owner, lo, hi), a)| {
        let h = sim.houses.get(owner);
        MerchantRoute {
            a: pos(lo), b: pos(hi), a_name: hname(lo), b_name: hname(hi),
            holder: h.map(|x| x.name.clone()).unwrap_or_default(),
            color: distinct_color(owner),
            is_guild: h.map(|x| x.is_guild).unwrap_or(false),
            sea: a.sea, river: a.river, volume: a.vol,
            out_goods: sort_goods(a.out), ret_goods: sort_goods(a.ret),
        }
    }).collect();
    out.sort_by(|x, y| y.volume.partial_cmp(&x.volume).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(150);
    Ok(out)
}


/// Expose the active futures contracts as directional supply lanes for the map's
/// Futures overlay (source city → buyer city).
#[tauri::command]
pub fn campaign_futures_lanes(db: State<'_, WorldDb>) -> Result<Vec<FuturesLane>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    // Estate endpoints collapse to their parent city so lanes connect cities.
    let city_of = |h: u32| -> u32 {
        match sim.hubs.get(h as usize) {
            Some(x) if x.is_estate && x.parent >= 0 => x.parent as u32,
            _ => h,
        }
    };
    let hname = |h: u32| sim.hubs.get(city_of(h) as usize).map(|x| x.name.clone()).unwrap_or_default();
    let pos = |h: u32| sim.hubs.get(city_of(h) as usize).map(|x| [x.x, x.y]).unwrap_or([0.0, 0.0]);
    let tick = sim.tick;
    let mut out: Vec<FuturesLane> = sim.contracts.iter().map(|c| {
        let h = sim.houses.get(c.seller_house as usize);
        // % fulfilled = delivered vs what was DUE by now (monthly_qty × months elapsed).
        let months = (tick.min(c.end_tick).saturating_sub(c.start_tick)) as f32 / 30.0;
        let due = (c.monthly_qty * months).max(1e-6);
        let base_value = sim.goods.get(c.good).map(|x| x.base_value).unwrap_or(1.0);
        FuturesLane {
            a: pos(c.source_hub), b: pos(c.buyer_hub),
            a_name: hname(c.source_hub), b_name: hname(c.buyer_hub),
            holder: h.map(|x| x.name.clone()).unwrap_or_default(),
            color: distinct_color(c.seller_house as usize),
            is_guild: h.map(|x| x.is_guild).unwrap_or(false),
            good: sim.goods.get(c.good).map(|x| x.name.clone()).unwrap_or_default(),
            qty: c.monthly_qty, term: c.term_years,
            end_year: c.end_tick / TICKS_PER_YEAR,
            suspended: c.suspended_until > tick,
            delivered: c.delivered,
            fulfilled_pct: (c.delivered / due * 100.0).clamp(0.0, 100.0),
            value: c.delivered * base_value,
            sealed_at: hname(c.buyer_hub),
        }
    }).collect();
    out.sort_by(|x, y| y.qty.partial_cmp(&x.qty).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(200);
    Ok(out)
}


/// All house/guild warehouses (largest stock first) for the Warehouses panel.
#[tauri::command]
pub fn campaign_warehouses(db: State<'_, WorldDb>) -> Result<Vec<WarehouseInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let gname = |g: usize| sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
    let mut out: Vec<WarehouseInfo> = sim.warehouses.iter().filter(|w| w.owner >= 0).map(|w| {
        let h = sim.houses.get(w.owner as usize);
        let hub = sim.hubs.get(w.hub as usize);
        let mut goods: Vec<(String, f32)> = w.stock.iter().enumerate()
            .filter(|(_, &s)| s > 0.01)
            .map(|(g, &s)| (gname(g), s)).collect();
        goods.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        goods.truncate(8);
        let contracts = sim.contracts.iter()
            .filter(|c| c.seller_house == w.owner as u32 && c.source_hub == w.hub).count() as u32;
        WarehouseInfo {
            kind: "warehouse".into(),
            owner: h.map(|x| x.name.clone()).unwrap_or_default(),
            color: distinct_color(w.owner as usize),
            is_guild: h.map(|x| x.is_guild).unwrap_or(false),
            city: hub.map(|x| x.name.clone()).unwrap_or_default(),
            x: hub.map(|x| x.x).unwrap_or(0.0),
            y: hub.map(|x| x.y).unwrap_or(0.0),
            tier: w.tier,
            capacity: w.capacity,
            used: w.stock.iter().sum(),
            goods,
            contracts,
            damage: w.damage,
        }
    }).collect();
    // Estates & manufactories — the production sites that FEED the warehouses. Listed
    // alongside the depots so the player sees the whole asset chain in one place.
    let kind_label = |k: u8| -> &'static str {
        match k { 1 => "farm", 2 => "mine", 3 => "plantation", 4 => "fishery",
                  5 => "vineyard", 6 => "manufactory", _ => "estate" }
    };
    for hub in sim.hubs.iter().filter(|h| h.is_estate && h.owner_house >= 0) {
        let oi = hub.owner_house as usize;
        let h = sim.houses.get(oi);
        let mut goods: Vec<(String, f32)> = hub.production.iter().enumerate()
            .filter(|(_, &p)| p > 0.01)
            .map(|(g, &p)| (gname(g), p)).collect();
        goods.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        goods.truncate(8);
        let parent = sim.hubs.get(hub.parent.max(0) as usize).map(|x| x.name.clone()).unwrap_or_default();
        out.push(WarehouseInfo {
            kind: kind_label(hub.estate_kind).into(),
            owner: h.map(|x| x.name.clone()).unwrap_or_default(),
            color: distinct_color(oi),
            is_guild: h.map(|x| x.is_guild).unwrap_or(false),
            city: if parent.is_empty() { hub.name.clone() } else { format!("{} (by {})", hub.name, parent) },
            x: hub.x, y: hub.y,
            tier: hub.estate_tier,
            capacity: 0.0, // estates produce rather than store
            used: hub.production.iter().sum(),
            goods,
            contracts: 0,
            damage: 0.0,
        });
    }
    out.sort_by(|a, b| b.used.partial_cmp(&a.used).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(400);
    Ok(out)
}


/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.3 (D17) · the CITY's own warehouse —
/// distinct from a house/guild depot (`campaign_warehouses` above). One slot
/// grid's worth of data per good: total + the three grade bands (D3), this
/// month's movement, what rotted, and a cover-in-months reading. `None` for an
/// estate or an unknown hub (a city warehouse is a settlement thing).
#[tauri::command]
pub fn campaign_city_warehouse(hub: u32, db: State<'_, WorldDb>) -> Result<Option<CityWarehouseInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let Some(h) = sim.hubs.iter().position(|hb| hb.id == hub) else { return Ok(None) };
    let hb = &sim.hubs[h];
    if hb.is_estate { return Ok(None); }
    let ng = sim.goods.len();
    let tier_w = [1.0f32, 0.45, 0.22];
    let goods: Vec<CityWarehouseGood> = (0..ng).filter_map(|g| {
        let amount = crate::sim::tick::stock_of(&hb.stock, g);
        let last = hb.wh_last_month.get(g).copied().unwrap_or(amount);
        let spoiled = hb.wh_spoiled_month.get(g).copied().unwrap_or(0.0);
        if amount <= 0.5 && last <= 0.5 && spoiled <= 0.01 { return None; }
        let tg = &sim.goods[g];
        let base = g * crate::sim::tick::GRADE_BANDS;
        let monthly_need = hb.population
            * tier_w[tg.need_tier.min(2) as usize]
            * tg.desire.max(0.0)
            * sim.need_scale
            * crate::sim::tick::DEMAND_PRESSURE
            * 30.0;
        let cover_months = if monthly_need > 1e-3 { amount / monthly_need } else { f32::INFINITY };
        let sbase = g * crate::sim::tick::SUPPLY_CLASSES;
        let raw: [f32; 5] = std::array::from_fn(|c| hb.supply_accum.get(sbase + c).copied().unwrap_or(0.0).max(0.0));
        let raw_total: f32 = raw.iter().sum();
        let supply_shares = if raw_total > 1e-3 { raw.map(|v| v / raw_total) } else { [0.0; 5] };
        Some(CityWarehouseGood {
            good: g,
            name: tg.name.clone(),
            amount,
            coarse: hb.stock.get(base).copied().unwrap_or(0.0),
            common: hb.stock.get(base + 1).copied().unwrap_or(0.0),
            fine: hb.stock.get(base + 2).copied().unwrap_or(0.0),
            delta_month: amount - last,
            spoiled_month: spoiled,
            need_tier: tg.need_tier.min(2),
            cover_months,
            supply_shares,
        })
    }).collect();
    let used: f32 = goods.iter().map(|g| g.amount).sum();
    let spoiled_total_month: f32 = goods.iter().map(|g| g.spoiled_month).sum();
    let capacity = hb.wh_capacity;
    Ok(Some(CityWarehouseInfo {
        hub,
        city: hb.name.clone(),
        capacity,
        used,
        fill_frac: if capacity > 1e-3 { (used / capacity).min(2.0) } else { 0.0 },
        spoiled_total_month,
        goods,
    }))
}


/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (D15/D16/D2) · one works card:
/// rank/yield, condition, ownership bar and the twelve-month curves. `None`
/// for a non-estate or unknown hub — the card is an estate/manufactory view.
#[tauri::command]
pub fn campaign_works_card(hub: u32, db: State<'_, WorldDb>) -> Result<Option<WorksCardInfo>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let Some(h) = sim.hubs.iter().position(|hb| hb.id == hub) else { return Ok(None) };
    let hb = &sim.hubs[h];
    if !hb.is_estate { return Ok(None); }
    let Some((g, yield_index, rank, rank_of)) = sim.works_rank(h) else { return Ok(None); };
    let good_name = sim.goods.get(g).map(|tg| tg.name.clone()).unwrap_or_default();

    // Ownership bar: resolve each share row to a display name/colour; whatever
    // fraction the table doesn't claim still belongs to owner_house (or the
    // parent city), D1's own "empty ⇒ 100% to owner" convention generalized to
    // "unclaimed ⇒ the rest to owner".
    let claimed: f32 = hb.shares.iter().map(|s| s.frac.max(0.0)).sum();
    let mut owners: Vec<WorksOwnerShare> = hb.shares.iter().map(|s| {
        let (name, color) = match s.holder_kind {
            1 | 2 => {
                let hi = s.holder as usize;
                (sim.houses.get(hi).map(|x| x.name.clone()).unwrap_or_default(), distinct_color(hi))
            }
            3 => {
                let bi = s.holder as usize;
                (sim.banks.get(bi).map(|x| x.name.clone()).unwrap_or_default(), "#8a6fb0".into())
            }
            4 => {
                let ri = s.holder as usize;
                (sim.realms.get(ri).map(|x| x.name.clone()).unwrap_or_default(), "#d8b24a".into())
            }
            _ => (sim.hubs.get(hb.parent.max(0) as usize).map(|x| format!("City of {}", x.name)).unwrap_or_default(), "#6a86a6".into()),
        };
        WorksOwnerShare {
            holder_kind: s.holder_kind, name, color, frac: s.frac.max(0.0),
            payout: s.payout, instrument: s.instrument, term_years: s.term_years,
        }
    }).collect();
    let remainder = (1.0 - claimed).max(0.0);
    if remainder > 0.005 {
        let (name, color) = if hb.owner_house >= 0 {
            let oi = hb.owner_house as usize;
            (sim.houses.get(oi).map(|x| x.name.clone()).unwrap_or_default(), distinct_color(oi))
        } else {
            (sim.hubs.get(hb.parent.max(0) as usize).map(|x| format!("City of {}", x.name)).unwrap_or_default(), "#6a86a6".into())
        };
        owners.push(WorksOwnerShare {
            holder_kind: if hb.owner_house >= 0 { 1 } else { 0 }, name, color, frac: remainder,
            payout: 1, instrument: crate::sim::tick::share_instrument_for_kind(hb.estate_kind), term_years: 0,
        });
    }

    let monthly: Vec<WorksMonthPoint> = hb.monthly.iter()
        .map(|m| WorksMonthPoint { output: m.output, quality: m.quality, price: m.price }).collect();
    let monthly_output = hb.production.get(g).copied().unwrap_or(0.0);
    let prev_output = hb.monthly.iter().rev().nth(1).map(|m| m.output).unwrap_or(monthly_output);
    let brand = if hb.brand_chronicled {
        let place = crate::sim::tick::brand_place(&hb.name, crate::sim::tick::estate_kind_label(hb.estate_kind));
        Some(crate::sim::tick::brand_name(&place, &good_name))
    } else { None };

    Ok(Some(WorksCardInfo {
        hub,
        name: hb.name.clone(),
        kind: hb.estate_kind,
        kind_label: crate::sim::tick::estate_kind_label(hb.estate_kind).to_string(),
        tier: hb.estate_tier,
        good: g,
        good_name,
        condition: (1.0 - hb.damage).clamp(0.0, 1.0),
        damage: hb.damage,
        yield_index,
        yield_label: crate::sim::tick::yield_label(yield_index).to_string(),
        rank,
        rank_of,
        monthly_output,
        output_delta: monthly_output - prev_output,
        quality: hb.quality.get(g).copied().unwrap_or(0.0),
        owners,
        monthly,
        brand,
    }))
}


/// Live ranking of the wealthiest / busiest trading cities, with each city's share
/// of all world trade — top to bottom.
#[tauri::command]
pub fn campaign_city_ranking(db: State<'_, WorldDb>) -> Result<Vec<CityRank>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let trade_of = |h: &TickHub| (h.export_earn + h.import_spend).max(0.0);
    let total: f32 = sim.hubs.iter().filter(|h| !h.is_estate).map(trade_of).sum::<f32>().max(1e-6);
    // C1 · normalizers for the prosperity composite (max over living cities).
    let mut max_trade = 1e-6f32; let mut max_treas = 1e-6f32; let mut max_comm = 1e-6f32;
    for h in sim.hubs.iter().filter(|h| !h.is_estate) {
        max_trade = max_trade.max(trade_of(h));
        max_treas = max_treas.max(h.treasury.max(0.0));
        max_comm = max_comm.max(h.society.commoner_wealth.max(0.0));
    }
    let mut out: Vec<CityRank> = sim.hubs.iter().filter(|h| !h.is_estate).map(|h| {
        let trade = trade_of(h);
        let commoner_wealth = h.society.commoner_wealth.max(0.0);
        let inequality = h.society.inequality.clamp(0.0, 1.0);
        // Weighted, normalized blend — flow, public stock, broad prosperity, equity.
        let prosperity = 0.42 * (trade / max_trade)
            + 0.20 * (h.treasury.max(0.0) / max_treas)
            + 0.26 * (commoner_wealth / max_comm)
            + 0.12 * (1.0 - inequality);
        CityRank {
            id: h.id, name: h.name.clone(), population: h.population.max(0.0) as u32,
            wealth: h.grain_wealth + h.trade_wealth, trade, pct_world: trade / total * 100.0,
            prosperity, treasury: h.treasury, commoner_wealth, inequality,
        }
    }).collect();
    out.sort_by(|a, b| b.trade.partial_cmp(&a.trade).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(40);
    Ok(out)
}


#[tauri::command]
pub fn campaign_diagnostics(db: State<'_, WorldDb>) -> Result<Option<CampaignDiagnostics>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None) };
    let active: Vec<&crate::sim::tick::House> = sim.houses.iter().filter(|h| !h.defunct).collect();
    let (mut fs, mut fr, mut fc, mut wealth) = (0u32, 0u32, 0u32, 0.0f32);
    for h in &active {
        fs += h.fleet_sea; fr += h.fleet_river; fc += h.fleet_caravan; wealth += h.wealth;
    }
    let controlled = build_house_briefs(&sim).iter()
        .filter(|h| !h.defunct).map(|h| h.controls.len() as u32).sum();
    Ok(Some(CampaignDiagnostics {
        tick: sim.tick,
        year: sim.tick / 365,
        in_transit: sim.in_transit.len() as u32,
        shipments_last: sim.diag_shipments,
        by_house: sim.diag_by_house,
        by_guild: sim.diag_by_guild,
        lost_last: sim.diag_lost,
        volume_last: sim.diag_volume,
        houses_active: active.len() as u32,
        houses_defunct: (sim.houses.len() - active.len()) as u32,
        fleet_sea: fs, fleet_river: fr, fleet_caravan: fc,
        controlled_settlements: controlled,
        total_house_wealth: wealth,
    }))
}


/// Phase 6 · the Guilds & Crafts panel: every craft guild, highest quality first.
/// Exceptional crafts carry a place-brand so they read as distinct goods.
#[tauri::command]
pub fn campaign_get_guilds(db: State<'_, WorldDb>) -> Result<Vec<GuildBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let (gw, gh) = (sim.world_w as u32, (sim.world_w * 0.5) as u32);
    let mut out: Vec<GuildBrief> = sim.guilds.iter().filter_map(|g| {
        let (hub, good) = (g.hub as usize, g.good as usize);
        let h = sim.hubs.get(hub)?;
        let spec = sim.goods.get(good)?;
        let quality = h.quality.get(good).copied().unwrap_or(0.0);
        let exceptional = quality >= GUILD_EXCEPTIONAL;
        let culture = crate::sim::names::culture_label(
            h.x.max(0.0) as u32, h.y.max(0.0) as u32, gw, gh).to_string();
        // Brand by the city (a place of renown, like Murano glass).
        let brand = if exceptional { format!("{} {}", h.name, spec.name) } else { String::new() };
        Some(GuildBrief {
            hub: g.hub, x: h.x, y: h.y, city: h.name.clone(),
            good: g.good, good_name: spec.name.clone(),
            quality, output: h.production.get(good).copied().unwrap_or(0.0),
            strength: g.strength, hall: g.hall,
            luxury: spec.need_tier >= 2,
            exceptional, brand, culture,
        })
    }).collect();
    out.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}


/// DLC 3.5 · per-city schematics for every real city (non-estate), largest first.
#[tauri::command]
pub fn campaign_get_schematics(db: State<'_, WorldDb>) -> Result<Vec<CitySchematic>, String> {
    use crate::sim::tick::{structure_label, structure_effect};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let estate_label = |k: u8| match k {
        1 => "Farm", 2 => "Mine", 3 => "Plantation", 4 => "Fishery", 5 => "Vineyard", 6 => "Manufactory",
        _ => "Estate",
    };
    let mut out: Vec<CitySchematic> = Vec::new();
    for (i, h) in sim.hubs.iter().enumerate() {
        if h.is_estate || h.population < 1.0 { continue; }
        let buildings = h.structures.iter().map(|&s| SchematicBuilding {
            label: structure_label(s).to_string(), effect: structure_effect(s).to_string(),
        }).collect();
        // Estates parented to this city.
        let estates = sim.hubs.iter()
            .filter(|e| e.is_estate && e.parent == i as i32)
            .map(|e| {
                let owner = if e.owner_house >= 0 {
                    sim.houses.get(e.owner_house as usize).map(|x| x.name.clone()).unwrap_or_default()
                } else { "City".into() };
                let good = e.base_per_capita.iter().enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .and_then(|(g, _)| sim.goods.get(g)).map(|x| x.name.clone()).unwrap_or_default();
                SchematicEstate {
                    label: estate_label(e.estate_kind).to_string(),
                    tier: e.estate_tier.max(1), owner, good,
                }
            }).collect();
        let banks_seated = sim.banks.iter()
            .filter(|b| !b.defunct && b.seat as usize == i)
            .map(|b| b.name.clone()).collect();
        let bank_branches = sim.banks.iter()
            .filter(|b| !b.defunct && b.seat as usize != i && b.branches.contains(&(i as u32)))
            .map(|b| b.name.clone()).collect();
        let council = if h.council_house >= 0 {
            sim.houses.get(h.council_house as usize).map(|x| x.name.clone()).unwrap_or_default()
        } else { String::new() };
        out.push(CitySchematic {
            hub: h.id, name: h.name.clone(), x: h.x, y: h.y,
            population: h.population as u32,
            coin_name: h.coin_name.clone(), coin_trust: h.coin_trust,
            coin_metal: match h.coin_metal { 1 => "gold", 2 => "electrum", 3 => "bronze", _ => "silver" }.to_string(),
            council, buildings, estates, banks_seated, bank_branches,
        });
    }
    out.sort_by(|a, b| b.population.cmp(&a.population));
    Ok(out)
}


#[tauri::command]
pub fn campaign_trade_flows(id: u32, db: State<'_, WorldDb>) -> Result<Option<TradeFlows>, String> {
    use std::collections::HashMap;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None) };
    // `log_trade` records flows by ARRAY INDEX (the sim's internal hub key), not by
    // the external settlement `id` the UI passes — so resolve the index here and work
    // in index space throughout. (Mismatching the two showed the wrong, cross-ocean
    // partner and made most cities look like they had no trade at all.)
    let Some(hi) = sim.hubs.iter().position(|h| h.id == id) else { return Ok(None) };
    let hidx = hi as u32;
    let (hub_x, hub_y) = (sim.hubs[hi].x, sim.hubs[hi].y);
    // Estates/manufactories are NOT independent partners — fold them into their
    // PARENT settlement so the partner list shows real cities, not "House X
    // Manufactory" rows (#17). `pos` resolves the (folded) settlement's name/coords.
    let city_of = |idx: u32| -> u32 {
        match sim.hubs.get(idx as usize) {
            Some(x) if x.is_estate && x.parent >= 0 => x.parent as u32,
            _ => idx,
        }
    };
    let pos = |idx: u32| sim.hubs.get(city_of(idx) as usize).map(|h| (h.name.clone(), h.x, h.y));

    // ── Per-good last-year in/out + per-(good,partner,dir) route amounts ──
    let mut g_in: HashMap<u32, f32> = HashMap::new();
    let mut g_out: HashMap<u32, f32> = HashMap::new();
    let mut g_partners: HashMap<u32, std::collections::HashSet<u32>> = HashMap::new();
    let mut route_amt: HashMap<(u32, u32, u8), f32> = HashMap::new(); // (good,partner,dir)→amt
    let mut partner_vol: HashMap<u32, f32> = HashMap::new();
    // Split by direction as well as summed. `f.dir` is already in hand (the route
    // fold below keys on it); it was simply dropped when the partner totals were
    // rolled up, which conflated a supplier we depend on with a customer we sell to.
    let mut partner_in: HashMap<u32, f32> = HashMap::new();
    let mut partner_out: HashMap<u32, f32> = HashMap::new();
    let mut partner_goods: HashMap<u32, HashMap<u32, f32>> = HashMap::new(); // partner→good→amt
    // Transport split + carrier breakdown, per good and for the city as a whole.
    let mut g_sea: HashMap<u32, f32> = HashMap::new();
    let mut g_river: HashMap<u32, f32> = HashMap::new();
    let mut g_carriers: HashMap<u32, HashMap<u32, f32>> = HashMap::new(); // good→carrier→amt
    let mut route_sea: HashMap<(u32, u32, u8), f32> = HashMap::new();
    let mut route_river: HashMap<(u32, u32, u8), f32> = HashMap::new();
    for f in sim.trade_last.iter().filter(|f| f.hub == hidx) {
        let partner = city_of(f.partner); // fold estates/manufactories into their settlement
        if partner == hidx { continue; }  // skip self-trade after folding (own estate)
        if f.dir == 0 { *g_in.entry(f.good).or_insert(0.0) += f.amount; }
        else { *g_out.entry(f.good).or_insert(0.0) += f.amount; }
        g_partners.entry(f.good).or_default().insert(partner);
        *route_amt.entry((f.good, partner, f.dir)).or_insert(0.0) += f.amount;
        *partner_vol.entry(partner).or_insert(0.0) += f.amount;
        if f.dir == 0 { *partner_in.entry(partner).or_insert(0.0) += f.amount; }
        else { *partner_out.entry(partner).or_insert(0.0) += f.amount; }
        *partner_goods.entry(partner).or_default().entry(f.good).or_insert(0.0) += f.amount;
        *g_sea.entry(f.good).or_insert(0.0) += f.sea_amount;
        *g_river.entry(f.good).or_insert(0.0) += f.river_amount;
        *route_sea.entry((f.good, partner, f.dir)).or_insert(0.0) += f.sea_amount;
        *route_river.entry((f.good, partner, f.dir)).or_insert(0.0) += f.river_amount;
        let cg = g_carriers.entry(f.good).or_default();
        for &(who, amt) in &f.carriers { *cg.entry(who).or_insert(0.0) += amt; }
    }

    // ── Goods list: union of last-year flows + historical series ──
    let mut hist_by_good: HashMap<u32, &Vec<f32>> = HashMap::new();
    // `trade_hist` is keyed by the sim's ARRAY INDEX (`hidx`), exactly like `trade_last`
    // above — NOT the external settlement `id`. Filtering by `id` here left the history
    // empty whenever id≠index, so the avg showed "0.0/yr" and a "0-yr trend" even though
    // last-year flows (3.4k) were present.
    for h in sim.trade_hist.iter().filter(|h| h.hub == hidx) { hist_by_good.insert(h.good, &h.vols); }
    let mut good_ids: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for &g in g_in.keys().chain(g_out.keys()) { good_ids.insert(g); }
    for &g in hist_by_good.keys() { good_ids.insert(g); }
    let mut goods: Vec<TradeFlowGood> = good_ids.into_iter().map(|g| {
        let history: Vec<f32> = hist_by_good.get(&g).map(|v| (*v).clone()).unwrap_or_default();
        let avg = if history.is_empty() { 0.0 } else { history.iter().sum::<f32>() / history.len() as f32 };
        let iv = g_in.get(&g).copied().unwrap_or(0.0);
        let ov = g_out.get(&g).copied().unwrap_or(0.0);
        TradeFlowGood {
            good: g,
            name: sim.goods.get(g as usize).map(|x| x.name.clone()).unwrap_or_default(),
            avg_volume: avg,
            last_volume: iv + ov,
            in_volume: iv, out_volume: ov,
            route_count: g_partners.get(&g).map(|s| s.len() as u32).unwrap_or(0),
            history,
            sea_volume: g_sea.get(&g).copied().unwrap_or(0.0),
            river_volume: g_river.get(&g).copied().unwrap_or(0.0),
            carriers: {
                // Who actually moved this good, largest share first. A house index
                // resolves to its name and whether it is a GUILD (a civic body) or a
                // private HOUSE; `u32::MAX` is the residual — shipments with no named
                // owner, i.e. ordinary local merchants.
                let total = iv + ov;
                let mut v: Vec<TradeCarrier> = g_carriers.get(&g).map(|m| m.iter().map(|(&who, &amt)| {
                    let h = if who == u32::MAX { None } else { sim.houses.get(who as usize) };
                    TradeCarrier {
                        name: h.map(|x| x.name.clone()).unwrap_or_else(|| "local merchants".into()),
                        is_guild: h.map(|x| x.is_guild).unwrap_or(false),
                        house: if who == u32::MAX { -1 } else { who as i32 },
                        amount: amt,
                        pct: if total > 0.0 { amt / total * 100.0 } else { 0.0 },
                        color: if who == u32::MAX { RESIDUAL_TINT.into() } else { distinct_color(who as usize) },
                    }
                }).collect()).unwrap_or_default();
                v.sort_by(|a, b| b.amount.partial_cmp(&a.amount)
                    .unwrap_or(std::cmp::Ordering::Equal).then(a.name.cmp(&b.name)));
                v.truncate(6);
                v
            },
        }
    }).collect();
    goods.sort_by(|a, b| b.avg_volume.partial_cmp(&a.avg_volume).unwrap_or(std::cmp::Ordering::Equal));

    // ── Routes (per good, ranked; pct of that good's total flow) ──
    let mut good_total: HashMap<u32, f32> = HashMap::new();
    for (&(g, _, _), &amt) in &route_amt { *good_total.entry(g).or_insert(0.0) += amt; }
    let mut routes: Vec<TradeRouteFlow> = route_amt.iter().filter_map(|(&(g, partner, dir), &amount)| {
        let (pname, px, py) = pos(partner)?;
        let tot = good_total.get(&g).copied().unwrap_or(0.0).max(1e-6);
        Some(TradeRouteFlow {
            good: g, partner, partner_name: pname, px, py, dir, amount,
            pct: amount / tot * 100.0,
            sea_amount: route_sea.get(&(g, partner, dir)).copied().unwrap_or(0.0),
            river_amount: route_river.get(&(g, partner, dir)).copied().unwrap_or(0.0),
        })
    }).collect();
    routes.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));

    // ── Top partner cities (share of all this city's trade) ──
    let total_vol: f32 = partner_vol.values().sum::<f32>().max(1e-6);
    // Each direction is a share of ITS OWN book, not of total trade: a city that
    // imports a trickle and exports a flood would otherwise show every one of its
    // suppliers at ~0%, which is exactly the dependency worth seeing.
    let total_in: f32 = partner_in.values().sum::<f32>().max(1e-6);
    let total_out: f32 = partner_out.values().sum::<f32>().max(1e-6);
    let mut partners: Vec<TradePartner> = partner_vol.iter().filter_map(|(&p, &vol)| {
        let (pname, px, py) = pos(p)?;
        let iv = partner_in.get(&p).copied().unwrap_or(0.0);
        let ov = partner_out.get(&p).copied().unwrap_or(0.0);
        let mut gs: Vec<(u32, f32)> = partner_goods.get(&p).map(|m| m.iter().map(|(&g, &a)| (g, a)).collect()).unwrap_or_default();
        gs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let goods = gs.iter().take(4).filter_map(|(g, _)| sim.goods.get(*g as usize).map(|x| x.name.clone())).collect();
        Some(TradePartner {
            hub: p, name: pname, px, py, volume: vol, pct: vol / total_vol * 100.0, goods,
            in_volume: iv, out_volume: ov,
            in_pct: iv / total_in * 100.0, out_pct: ov / total_out * 100.0,
        })
    }).collect();
    partners.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    // 12 was enough for one combined list; the view now ranks the same set twice
    // (once per direction), so a city that is a big supplier but a small customer
    // has to survive the truncation to appear in the import column at all.
    partners.truncate(20);

    // ── TRADERS TAB ─────────────────────────────────────────────────────────
    // Who moved cargo AT THIS CITY, and who is established here. Two different
    // questions that routinely disagree — a house can seat the council and carry
    // nothing — so they are built as two lists, not one.
    //
    // Everything here is scoped to this city's own trade (`trade_last` is already
    // filtered to `hub == hidx`), which is what makes it a balance sheet rather
    // than a world view.
    let km_per_cell = if sim.world_w > 0.0 { 40075.0 / sim.world_w } else { 0.0 };
    let dist_km = |px: f32, py: f32| -> f32 {
        let mut dx = (px - hub_x).abs();
        if sim.world_w > 0.0 && dx > sim.world_w / 2.0 { dx = sim.world_w - dx; }
        let dy = py - hub_y;
        (dx * dx + dy * dy).sqrt() * km_per_cell
    };

    #[derive(Default)]
    struct Acc {
        vol: f32, inv: f32, outv: f32, sea: f32, river: f32,
        dist_wsum: f32,
        by_good_in: HashMap<u32, f32>,
        by_good_out: HashMap<u32, f32>,
    }
    let mut acc: HashMap<u32, Acc> = HashMap::new();   // carrier key (u32::MAX = ownerless)
    for f in sim.trade_last.iter().filter(|f| f.hub == hidx) {
        let partner = city_of(f.partner);
        if partner == hidx { continue; }
        let Some((_, px, py)) = pos(partner) else { continue };
        let d = dist_km(px, py);
        // `sea`/`river` are properties of the ROUTE, so every shipment in one
        // aggregate row shares them; attribute each carrier its pro-rata share
        // rather than inventing a per-carrier flag the sim never recorded.
        let sea_frac = if f.amount > 0.0 { (f.sea_amount / f.amount).clamp(0.0, 1.0) } else { 0.0 };
        let river_frac = if f.amount > 0.0 { (f.river_amount / f.amount).clamp(0.0, 1.0) } else { 0.0 };
        for &(who, amt) in &f.carriers {
            let e = acc.entry(who).or_default();
            e.vol += amt;
            e.sea += amt * sea_frac;
            e.river += amt * river_frac;
            e.dist_wsum += amt * d;
            if f.dir == 0 { e.inv += amt; *e.by_good_in.entry(f.good).or_insert(0.0) += amt; }
            else { e.outv += amt; *e.by_good_out.entry(f.good).or_insert(0.0) += amt; }
        }
    }
    let trade_total: f32 = acc.values().map(|a| a.vol).sum::<f32>().max(1e-6);

    // Standing at this city, independent of carriage.
    let council = sim.hubs[hi].council_house;
    let captor = sim.hubs[hi].captor_house;
    let standing = |hidx_house: i32| -> (bool, bool, bool, bool) {
        if hidx_house < 0 { return (false, false, false, false); }
        let h = match sim.houses.get(hidx_house as usize) { Some(h) => h, None => return (false, false, false, false) };
        (h.offices.contains(&hidx), h.bailos.contains(&hidx),
         council == hidx_house, captor == hidx_house)
    };

    let mut traders: Vec<CityTrader> = acc.iter().map(|(&who, a)| {
        let h = if who == u32::MAX { None } else { sim.houses.get(who as usize) };
        let hi_i = if who == u32::MAX { -1 } else { who as i32 };
        let (office, bailo, seats, capt) = standing(hi_i);
        // RE-EXPORT: per good, the part this trader both landed here and shipped
        // onward. See `CityTrader::reexport` for why it is not called "transit".
        let mut reexport = 0.0f32;
        for (g, &iv) in &a.by_good_in {
            if let Some(&ov) = a.by_good_out.get(g) { reexport += iv.min(ov); }
        }
        // The goods this trader actually moved here, biggest first.
        let mut gv: HashMap<u32, f32> = HashMap::new();
        for (g, v) in a.by_good_in.iter().chain(a.by_good_out.iter()) { *gv.entry(*g).or_insert(0.0) += v; }
        let mut gs: Vec<(u32, f32)> = gv.into_iter().collect();
        gs.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal).then(x.0.cmp(&y.0)));
        CityTrader {
            name: h.map(|x| x.name.clone()).unwrap_or_else(|| "local merchants".into()),
            is_guild: h.map(|x| x.is_guild).unwrap_or(false),
            house: hi_i,
            volume: a.vol, in_volume: a.inv, out_volume: a.outv, sea_volume: a.sea, river_volume: a.river,
            pct: a.vol / trade_total * 100.0,
            reexport,
            mean_route_km: if a.vol > 0.0 { a.dist_wsum / a.vol } else { 0.0 },
            goods: gs.iter().take(5)
                .filter_map(|(g, _)| sim.goods.get(*g as usize).map(|x| x.name.clone())).collect(),
            color: if hi_i < 0 { RESIDUAL_TINT.into() } else { distinct_color(hi_i as usize) },
            good_rows: gs.iter().take(8).filter_map(|(g, amt)| {
                sim.goods.get(*g as usize).map(|x| TraderGood {
                    name: x.name.clone(),
                    amount: *amt,
                    in_amount: a.by_good_in.get(g).copied().unwrap_or(0.0),
                    out_amount: a.by_good_out.get(g).copied().unwrap_or(0.0),
                })
            }).collect(),
            has_office: office, has_bailo: bailo, seats_council: seats, is_captor: capt,
        }
    }).collect();
    traders.sort_by(|a, b| b.volume.partial_cmp(&a.volume)
        .unwrap_or(std::cmp::Ordering::Equal).then(a.name.cmp(&b.name)));

    // Established here — every holder with an office/bailo/seat, carrying or not.
    let mut established: Vec<CityEstablished> = sim.houses.iter().enumerate()
        .filter_map(|(i, h)| {
            let iu = i as u32;
            let office = h.offices.contains(&hidx);
            let bailo = h.bailos.contains(&hidx);
            let seats = council == i as i32;
            let capt = captor == i as i32;
            if !(office || bailo || seats || capt) { return None; }
            Some(CityEstablished {
                name: h.name.clone(), is_guild: h.is_guild, house: i as i32,
                has_office: office, has_bailo: bailo, seats_council: seats, is_captor: capt,
                volume: acc.get(&iu).map(|a| a.vol).unwrap_or(0.0),
                color: distinct_color(i),
            })
        }).collect();
    // A bailo outranks an office, a seat outranks both — sort by standing, then by
    // what they actually move, so the list reads as a hierarchy.
    established.sort_by(|a, b| {
        let rank = |x: &CityEstablished| (x.is_captor as u8) * 8 + (x.seats_council as u8) * 4
            + (x.has_bailo as u8) * 2 + (x.has_office as u8);
        rank(b).cmp(&rank(a))
            .then(b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.name.cmp(&b.name))
    });

    // The city's own capacity, so trade reads against what this place makes and
    // eats rather than in isolation. Yearly figures from the per-day rates the
    // tick already keeps.
    let produced_here: f32 = sim.hubs[hi].production.iter().sum::<f32>() * 365.0;
    // `base_need` is the per-day demand the tick itself uses, so this is the city's
    // own consumption on the model's terms rather than a second estimate.
    let consumed_here: f32 = (0..sim.goods.len()).map(|g| sim.base_need(hi, g)).sum::<f32>() * 365.0;

    let carrier_why = CarrierWhy {
        shipments: sim.diag_shipments,
        by_house: sim.diag_by_house,
        ownerless: sim.diag_shipments.saturating_sub(sim.diag_by_house),
        why_nohouse: sim.diag_why_nohouse,
        why_slot: sim.diag_why_slot,
        why_cash: sim.diag_why_cash,
        why_barred: sim.diag_why_bar,
    };

    Ok(Some(TradeFlows {
        hub: id, hub_x, hub_y, goods, routes, partners,
        traders, established, carrier_why, produced_here, consumed_here,
    }))
}


// ═════════════════════════════════════════════════════════════════════════════════
//  GOODS ATLAS (`campaign_good_atlas`) — everything about ONE good, for the Goods
//  Atlas panel (the remade Codex). Four facets, all read from live campaign state that
//  already exists: quality (hub production×quality), trade volume + producers/consumers
//  (last year's `trade_last`, dir 1 = export/producer, dir 0 = import/consumer), control
//  (per-good house share `volume/good_vol` + each house's/guild's TOTAL trade volume),
//  and the per-good directed flow lanes (exporter→importer) the map draws for one good.
// ═════════════════════════════════════════════════════════════════════════════════

/// A city on one of the atlas lists (a producer, consumer, or top-quality source).
#[derive(Serialize, Clone)]
pub struct AtlasHub { pub hub: u32, pub name: String, pub x: f32, pub y: f32, pub amount: f32 }
/// A house or guild trading the good — its per-good SHARE and its overall trade VOLUME
/// (so the panel can rank "who trades the most", houses and guilds alike).
#[derive(Serialize, Clone)]
pub struct AtlasHouse {
    pub house: u32, pub name: String, pub is_guild: bool,
    pub share: f32, pub total_volume: f32,
}
/// One directed trade lane for this good over the last full year (exporter → importer).
#[derive(Serialize, Clone)]
pub struct AtlasFlow { pub from: u32, pub to: u32, pub from_x: f32, pub from_y: f32, pub to_x: f32, pub to_y: f32, pub amount: f32 }

#[derive(Serialize, Default)]
pub struct GoodAtlas {
    pub good: String,
    pub manufactured: bool,
    pub total_produced: f32,
    pub total_traded: f32,
    pub avg_quality: f32,
    /// 10 bins over quality 0..1 (count of producing cities per band).
    pub quality_hist: Vec<u32>,
    pub top_quality: Vec<AtlasHub>,
    pub producers: Vec<AtlasHub>,
    pub consumers: Vec<AtlasHub>,
    pub houses: Vec<AtlasHouse>,
    pub flows: Vec<AtlasFlow>,
}

#[tauri::command]
pub fn campaign_good_atlas(good: String, db: State<'_, WorldDb>) -> Result<GoodAtlas, String> {
    use std::collections::HashMap;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(GoodAtlas::default()) };
    // Resolve the good by NAME (robust to any index drift between frontend and sim).
    let g = match sim.goods.iter().position(|x| x.name == good) { Some(i) => i, None => return Ok(GoodAtlas::default()) };
    let good = g as u32;

    // ── Quality: histogram of producing cities' grade + the finest sources. ──
    let mut quality_hist = vec![0u32; 10];
    let mut top_quality: Vec<AtlasHub> = Vec::new();
    let (mut q_wsum, mut q_w, mut total_produced) = (0.0f32, 0.0f32, 0.0f32);
    for h in &sim.hubs {
        let p = h.production.get(g).copied().unwrap_or(0.0);
        if p <= 0.0 { continue; }
        let q = h.quality.get(g).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        total_produced += p;
        q_wsum += q * p; q_w += p;
        quality_hist[((q * 10.0) as usize).min(9)] += 1;
        top_quality.push(AtlasHub { hub: h.id, name: h.name.clone(), x: h.x, y: h.y, amount: q });
    }
    top_quality.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
    top_quality.truncate(12);
    let avg_quality = if q_w > 0.0 { q_wsum / q_w } else { 0.0 };

    // ── Trade: last full year's flows for this good. dir 1 = the hub EXPORTED to the
    //    partner (a producer/supplier), dir 0 = the hub IMPORTED (a consumer). ──
    let mut exp: HashMap<u32, f32> = HashMap::new();
    let mut imp: HashMap<u32, f32> = HashMap::new();
    let mut lanes: HashMap<(u32, u32), f32> = HashMap::new();
    let mut total_traded = 0.0f32;
    for f in &sim.trade_last {
        if f.good != good { continue; }
        if f.dir == 1 {
            *exp.entry(f.hub).or_insert(0.0) += f.amount;
            *lanes.entry((f.hub, f.partner)).or_insert(0.0) += f.amount;
            total_traded += f.amount;
        } else {
            *imp.entry(f.hub).or_insert(0.0) += f.amount;
        }
    }
    let hub_pos = |h: u32| sim.hubs.get(h as usize).map(|x| (x.x, x.y, x.name.clone()))
        .unwrap_or((0.0, 0.0, String::new()));
    let mut to_hubs = |m: &HashMap<u32, f32>| -> Vec<AtlasHub> {
        let mut v: Vec<AtlasHub> = m.iter().map(|(&h, &amt)| {
            let (x, y, name) = hub_pos(h);
            AtlasHub { hub: h, name, x, y, amount: amt }
        }).collect();
        v.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(15);
        v
    };
    let producers = to_hubs(&exp);
    let consumers = to_hubs(&imp);
    let mut flows: Vec<AtlasFlow> = lanes.iter().map(|(&(from, to), &amt)| {
        let (fx, fy, _) = hub_pos(from);
        let (tx, ty, _) = hub_pos(to);
        AtlasFlow { from, to, from_x: fx, from_y: fy, to_x: tx, to_y: ty, amount: amt }
    }).collect();
    flows.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
    flows.truncate(60);

    // ── Control: per-good house share (volume ÷ world volume in this good) plus each
    //    trading house's/guild's TOTAL volume, so the panel ranks who trades the most. ──
    let mut good_vol = 0.0f32;
    for h in &sim.houses {
        if h.defunct { continue; }
        if h.spec.contains(&g) { good_vol += h.volume.max(0.0); }
    }
    let mut houses: Vec<AtlasHouse> = sim.houses.iter().enumerate()
        .filter(|(_, h)| !h.defunct && h.spec.contains(&g))
        .map(|(hi, h)| AtlasHouse {
            house: hi as u32, name: h.name.clone(), is_guild: h.is_guild,
            share: if good_vol > 1e-3 { (h.volume.max(0.0) / good_vol).clamp(0.0, 1.0) } else { 0.0 },
            total_volume: h.volume.max(0.0),
        })
        .collect();
    houses.sort_by(|a, b| b.share.partial_cmp(&a.share).unwrap_or(std::cmp::Ordering::Equal));
    houses.truncate(15);

    Ok(GoodAtlas {
        good: sim.goods[g].name.clone(),
        manufactured: !sim.goods[g].inputs.is_empty(),
        total_produced, total_traded, avg_quality,
        quality_hist, top_quality, producers, consumers, houses, flows,
    })
}
