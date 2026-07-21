//! read_money commands — split from the former monolithic campaign_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


#[tauri::command]
pub fn campaign_get_inequality(db: State<'_, WorldDb>) -> Result<InequalitySnapshot, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(InequalitySnapshot::default()) };
    let cur_year = sim.tick / TICKS_PER_YEAR;

    // Current inequality among living houses (live wealth).
    let now_vals: Vec<f32> = sim.houses.iter().filter(|h| !h.defunct).map(|h| h.wealth).collect();
    let gini_now = gini_of(now_vals.clone());
    let top10_share_now = top10_share_of(now_vals);
    let active_houses = sim.houses.iter().filter(|h| !h.defunct).count() as u32;
    let defunct_houses = sim.houses.iter().filter(|h| h.defunct).count() as u32;

    // Yearly trend reconstructed from per-house wealth_history (aligned from the
    // most-recent sample backwards; each house contributes only the years it has).
    let maxlen = sim.houses.iter().map(|h| h.wealth_history.len()).max().unwrap_or(0);
    let span = maxlen.min(40);
    let mut series: Vec<InequalityPoint> = Vec::new();
    for o in (0..span).rev() {
        let mut vals: Vec<f32> = Vec::new();
        for h in &sim.houses {
            let len = h.wealth_history.len();
            if len > o {
                let w = h.wealth_history[len - 1 - o];
                if w > 0.0 { vals.push(w); }
            }
        }
        if vals.len() < 2 { continue; }
        let active = vals.len() as u32;
        let mean = vals.iter().sum::<f32>() / active as f32;
        series.push(InequalityPoint {
            year: cur_year.saturating_sub(o as u32),
            gini: gini_of(vals.clone()),
            active,
            mean_wealth: mean,
            top10_share: top10_share_of(vals),
        });
    }

    // Social mobility: average normalized change in wealth RANK between the two
    // most recent yearly samples (0 = unchanged hierarchy, higher = more churn).
    let pairs: Vec<(f32, f32)> = sim.houses.iter().filter_map(|h| {
        let len = h.wealth_history.len();
        if len >= 2 {
            let (now, prev) = (h.wealth_history[len - 1], h.wealth_history[len - 2]);
            if now > 0.0 && prev > 0.0 { Some((now, prev)) } else { None }
        } else { None }
    }).collect();
    let rank_churn = if pairs.len() >= 2 {
        let n = pairs.len();
        let rank_by = |sel: &dyn Fn(&(f32, f32)) -> f32| -> Vec<usize> {
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|&a, &b| sel(&pairs[b]).partial_cmp(&sel(&pairs[a])).unwrap_or(std::cmp::Ordering::Equal));
            let mut rank = vec![0usize; n];
            for (r, &idx) in order.iter().enumerate() { rank[idx] = r; }
            rank
        };
        let rank_now = rank_by(&|p| p.0);
        let rank_prev = rank_by(&|p| p.1);
        let sum: f32 = (0..n).map(|k| (rank_now[k] as i32 - rank_prev[k] as i32).unsigned_abs() as f32).sum();
        (sum / n as f32) / ((n as f32 - 1.0).max(1.0))
    } else { 0.0 };

    Ok(InequalitySnapshot {
        active: true,
        year: cur_year,
        gini_now,
        top10_share_now,
        active_houses,
        defunct_houses,
        founded_total: sim.houses.len() as u32,
        rank_churn,
        series,
    })
}


/// DLC 3 · the cached yearly speculation read (per-polis bubble risk + the
/// generated causal reason-chain). Empty until the campaign passes its first
/// New Year. Mirrors the `compute_political` overlay payload.
#[tauri::command]
pub fn campaign_get_speculation(db: State<'_, WorldDb>) -> Result<Vec<SpecCenter>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(match get_sim(&db, &conn)? {
        Some(sim) => sim.spec_centers.clone(),
        None => vec![],
    })
}


#[tauri::command]
pub fn campaign_get_poleis(db: State<'_, WorldDb>) -> Result<Vec<PolisBrief>, String> {
    use crate::sim::tick::{archetype_label};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<PolisBrief> = sim.hubs.iter()
        .filter(|h| !h.is_estate && h.population >= 1.0)
        .map(|h| {
            let ci = h.council_house;
            let (council, arch, color) = if ci >= 0 {
                if let Some(house) = sim.houses.get(ci as usize) {
                    (house.name.clone(), archetype_label(house.archetype).to_string(), distinct_color(ci as usize))
                } else { ("—".into(), String::new(), "#7a8aa0".into()) }
            } else { ("—".into(), String::new(), "#7a8aa0".into()) };
            let coin_issuer = if ci >= 0 { council.clone() } else { String::new() };
            let war_with = if h.war_with >= 0 {
                sim.hubs.get(h.war_with as usize).map(|x| x.name.clone()).unwrap_or_default()
            } else { String::new() };
            PolisBrief {
                hub: h.id, name: h.name.clone(), x: h.x, y: h.y,
                population: h.population as u32, treasury: h.treasury,
                tariff_export: h.tariff_export, tariff_import: h.tariff_import,
                mint_fineness: if h.mint_fineness <= 0.0 { 1.0 } else { h.mint_fineness },
                council, council_archetype: arch, council_color: color,
                coin_name: h.coin_name.clone(), coin_trust: h.coin_trust,
                coin_value: if h.coin_name.is_empty() { 0.0 } else { crate::sim::tick::coin_value(h.mint_fineness, h.coin_trust) },
                coin_issuer, war_with,
            }
        })
        .collect();
    out.sort_by(|a, b| b.treasury.partial_cmp(&a.treasury).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}


/// DLC 3.5 · the world's coinage, ranked by reserve strength (trust × throughput).
#[tauri::command]
pub fn campaign_get_currencies(db: State<'_, WorldDb>) -> Result<Vec<CurrencyBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    // Circulating amount + holder count per coin (by mint hub INDEX) from baskets.
    let mut circ: std::collections::HashMap<usize, (f32, u32)> = std::collections::HashMap::new();
    for h in sim.hubs.iter() {
        if h.is_estate { continue; }
        let thru = h.tw_house + h.tw_local + h.tw_guild;
        for &(k, share) in &h.coin_basket {
            let e = circ.entry(k as usize).or_insert((0.0, 0));
            e.0 += thru * share; e.1 += 1;
        }
    }
    let mut out: Vec<CurrencyBrief> = sim.hubs.iter().enumerate()
        .filter(|(_, h)| !h.is_estate && !h.coin_name.is_empty())
        .map(|(i, h)| {
            let throughput = h.tw_house + h.tw_local + h.tw_guild;
            let issuer = if h.council_house >= 0 {
                sim.houses.get(h.council_house as usize).map(|x| x.name.clone()).unwrap_or_default()
            } else { String::new() };
            let (circulating, held_in) = circ.get(&i).copied().unwrap_or((0.0, 0));
            CurrencyBrief {
                hub: h.id, city: h.name.clone(), coin_name: h.coin_name.clone(),
                trust: h.coin_trust,
                fineness: if h.mint_fineness <= 0.0 { 1.0 } else { h.mint_fineness },
                throughput,
                is_reserve: h.coin_trust >= 0.55,
                color: distinct_color(i),
                value: crate::sim::tick::coin_value(h.mint_fineness, h.coin_trust),
                issuer,
                circulating, held_in,
            }
        })
        .collect();
    out.sort_by(|a, b| (b.trust * (b.throughput + 1.0))
        .partial_cmp(&(a.trust * (a.throughput + 1.0)))
        .unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}


/// v2.0 · every polis (council seat) as a unified mint card, ranked by coin
/// strength then treasury. Coinless poleis are included (empty `coin_name`).
#[tauri::command]
pub fn campaign_get_mints(db: State<'_, WorldDb>) -> Result<Vec<MintBrief>, String> {
    use crate::sim::tick::{archetype_label, coin_value, coin_strength};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let n = sim.hubs.len();
    // Circulating supply + holder/abroad counts per coin (by mint hub INDEX).
    let mut circ: std::collections::HashMap<usize, (f32, u32, u32)> = std::collections::HashMap::new();
    for (hi, h) in sim.hubs.iter().enumerate() {
        if h.is_estate { continue; }
        let thru = h.tw_house + h.tw_local + h.tw_guild;
        for &(k, share) in &h.coin_basket {
            let e = circ.entry(k as usize).or_insert((0.0, 0, 0));
            e.0 += thru * share; e.1 += 1;
            if k as usize != hi { e.2 += 1; }
        }
    }
    let mut out: Vec<MintBrief> = sim.hubs.iter().enumerate()
        .filter(|(_, h)| !h.is_estate && h.population >= 1.0 && h.council_house >= 0)
        .map(|(i, h)| {
            let ci = h.council_house;
            let (council, arch, color) = if ci >= 0 {
                if let Some(house) = sim.houses.get(ci as usize) {
                    (house.name.clone(), archetype_label(house.archetype).to_string(), distinct_color(ci as usize))
                } else { ("—".into(), String::new(), "#7a8aa0".into()) }
            } else { ("—".into(), String::new(), "#7a8aa0".into()) };
            let war_with = if h.war_with >= 0 {
                sim.hubs.get(h.war_with as usize).map(|x| x.name.clone()).unwrap_or_default()
            } else { String::new() };
            let fineness = if h.mint_fineness <= 0.0 { 1.0 } else { h.mint_fineness };
            let throughput = h.tw_house + h.tw_local + h.tw_guild;
            let (circulating, held_in, abroad) = circ.get(&i).copied().unwrap_or((0.0, 0, 0));
            let has_coin = !h.coin_name.is_empty();
            MintBrief {
                hub: h.id, city: h.name.clone(), x: h.x, y: h.y, population: h.population as u32,
                treasury: h.treasury, tariff_export: h.tariff_export, tariff_import: h.tariff_import,
                council, council_archetype: arch, council_color: color, war_with,
                coin_name: h.coin_name.clone(),
                issuer: if ci >= 0 { sim.houses.get(ci as usize).map(|x| x.name.clone()).unwrap_or_default() } else { String::new() },
                metal: match h.coin_metal { 1 => "gold", 2 => "electrum", 3 => "bronze", _ => "silver" }.to_string(),
                trust: h.coin_trust,
                fineness,
                value: if has_coin { coin_value(h.mint_fineness, h.coin_trust) } else { 0.0 },
                exchange: if has_coin { crate::sim::tick::coin_exchange(h.coin_metal, fineness, h.coin_trust) } else { 0.0 },
                strength: if has_coin { coin_strength(fineness, h.coin_trust) } else { 0.0 },
                throughput,
                is_reserve: has_coin && h.coin_trust >= 0.55,
                circulating, held_in, abroad,
                price_level: if h.price_level <= 0.0 { 1.0 } else { h.price_level },
                bullion: if !has_coin { String::new() }
                    else if h.mint_bullion_ratio >= 1.0 { "ample".into() }
                    else if h.mint_bullion_ratio >= 0.5 { "tight".into() }
                    else { "scarce".into() },
                has_mint: h.has_mint,
                under_mandate: h.reform_until > sim.tick,
                reformed: h.last_reform_tick != 0,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        let sa = if a.coin_name.is_empty() { -1.0 } else { a.strength };
        let sb = if b.coin_name.is_empty() { -1.0 } else { b.strength };
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            .then(b.treasury.partial_cmp(&a.treasury).unwrap_or(std::cmp::Ordering::Equal))
    });
    Ok(out)
}


/// A3 · one coin's yearly BIOGRAPHY — the fineness/trust/value/price-level series
/// (oldest→newest) for the Money panel sparklines. `hub` is the mint's hub id.
#[tauri::command]
pub fn campaign_coin_history(db: State<'_, WorldDb>, hub: u32) -> Result<Vec<crate::sim::tick::CoinSnapshot>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    Ok(sim.hubs.iter().find(|h| h.id == hub).map(|h| h.coin_history.clone()).unwrap_or_default())
}


/// v2.0 · the monetary chronicle: journal rows about money & credit, newest first.
#[tauri::command]
pub fn campaign_monetary_chronicle(db: State<'_, WorldDb>) -> Result<Vec<MonetaryEvent>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    const KINDS: [&str; 6] = ["coinage", "charter", "reform", "run", "bank", "crash"];
    let mut out: Vec<MonetaryEvent> = sim.journal.iter()
        .filter(|e| KINDS.contains(&e.kind.as_str()))
        .map(|e| MonetaryEvent {
            year: e.tick / TICKS_PER_YEAR,
            tick: e.tick,
            kind: e.kind.clone(),
            city: if e.hub >= 0 { sim.hubs.get(e.hub as usize).map(|h| h.name.clone()).unwrap_or_default() } else { String::new() },
            value: e.value,
            text: e.text.clone(),
        })
        .collect();
    out.reverse(); // newest first
    Ok(out)
}


/// Per-city coin usage: which coin each settlement settles its trade in + the
/// volume, from the yearly `settle_coin` assignment. The frontend groups by `coin`
/// for the donut/bar breakdown and tints the map by it for the usage overlay.
#[tauri::command]
pub fn campaign_coin_usage(db: State<'_, WorldDb>) -> Result<Vec<CoinUseCity>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<CoinUseCity> = Vec::new();
    // A city now holds a BASKET of coins — emit one row per (city, coin) with the
    // city's throughput weighted by that coin's share, so the chart/overlay reflect
    // real circulation (a city can appear under several coins).
    for (i, h) in sim.hubs.iter().enumerate() {
        if h.is_estate { continue; }
        let thru = h.tw_house + h.tw_local + h.tw_guild;
        for &(k, share) in &h.coin_basket {
            let j = k as usize;
            let Some(coin_hub) = sim.hubs.get(j) else { continue };
            if coin_hub.coin_name.is_empty() { continue; }
            let mint = j == i;
            let primary = h.settle_coin == j as i32;
            let color = if coin_hub.council_house >= 0 {
                distinct_color(coin_hub.council_house as usize)
            } else { distinct_color(j) };
            out.push(CoinUseCity {
                coin: coin_hub.id,
                coin_name: coin_hub.coin_name.clone(),
                city: h.id,
                name: h.name.clone(),
                x: h.x, y: h.y,
                volume: thru * share,
                share,
                mint,
                primary,
                reserve_reach: !primary && coin_hub.coin_trust >= 0.55,
                color,
            });
        }
    }
    Ok(out)
}


/// v2.0 · currency reserves for every holder. Cities carry an explicit `coin_basket`;
/// BANKS and HOUSES don't hold per-coin balances in the sim, so their composition is
/// DERIVED at read time — a bank's specie is attributed across its seat + branch
/// cities' baskets, a house's wealth across the baskets of its home hub + offices.
/// (Pure presentation: the underlying wealth dynamics are untouched.)
#[tauri::command]
pub fn campaign_reserves(db: State<'_, WorldDb>) -> Result<ReservesPayload, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(ReservesPayload { cities: vec![], banks: vec![], houses: vec![] }) };
    let n = sim.hubs.len();
    let metal_name = |m: u8| match m { 1 => "gold", 2 => "electrum", 3 => "bronze", _ => "silver" }.to_string();
    // Per-coin metadata (name/colour/metal) keyed by issuing-mint hub INDEX.
    let coin_meta = |k: usize| -> Option<(String, String, String)> {
        let h = sim.hubs.get(k)?;
        if h.coin_name.is_empty() { return None; }
        let color = if h.council_house >= 0 { distinct_color(h.council_house as usize) } else { distinct_color(k) };
        Some((h.coin_name.clone(), color, metal_name(h.coin_metal)))
    };
    // Blend a set of (hub, weight) presences into a normalized coin→share basket.
    let blend = |presence: &[(usize, f32)], main_hub: Option<usize>| -> Vec<ReserveSlice> {
        let mut w: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for &(hub, wt) in presence {
            if hub >= n || wt <= 0.0 { continue; }
            for &(k, share) in &sim.hubs[hub].coin_basket { *w.entry(k as usize).or_insert(0.0) += share * wt; }
        }
        let tot: f32 = w.values().sum::<f32>().max(1e-6);
        let main_coin = main_hub.and_then(|h| sim.hubs.get(h)).map(|h| h.settle_coin).unwrap_or(-1);
        let mut slices: Vec<ReserveSlice> = w.into_iter().filter_map(|(k, v)| {
            let (coin_name, color, metal) = coin_meta(k)?;
            Some(ReserveSlice {
                coin_name, color, metal, share: v / tot,
                primary: k as i32 == main_coin,
                mint: main_hub == Some(k),
            })
        }).collect();
        slices.sort_by(|a, b| b.share.partial_cmp(&a.share).unwrap_or(std::cmp::Ordering::Equal));
        slices.truncate(6);
        slices
    };

    // Cities — their explicit basket.
    let mut cities: Vec<ReserveHolder> = Vec::new();
    for (i, h) in sim.hubs.iter().enumerate() {
        if h.is_estate || h.population < 1.0 || h.coin_basket.is_empty() { continue; }
        let total = h.tw_house + h.tw_local + h.tw_guild;
        let slices = blend(&[(i, 1.0)], Some(i));
        if slices.is_empty() { continue; }
        cities.push(ReserveHolder { kind: "city".into(), name: h.name.clone(), seat: String::new(), total, slices });
    }
    cities.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));
    cities.truncate(40);

    // Banks — specie attributed across seat (heavy) + branch cities' baskets.
    let mut banks: Vec<ReserveHolder> = Vec::new();
    for b in sim.banks.iter().filter(|b| !b.defunct) {
        let seat = b.seat as usize;
        let mut presence: Vec<(usize, f32)> = vec![(seat, 2.0)];
        for &br in &b.branches { if br as usize != seat { presence.push((br as usize, 1.0)); } }
        let slices = blend(&presence, Some(seat));
        if slices.is_empty() { continue; }
        banks.push(ReserveHolder {
            kind: "bank".into(), name: b.name.clone(),
            seat: sim.hubs.get(seat).map(|h| h.name.clone()).unwrap_or_default(),
            total: b.reserves, slices,
        });
    }
    banks.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));

    // Houses — wealth attributed across home hub (heavy) + offices + bailos.
    let mut houses: Vec<ReserveHolder> = Vec::new();
    for h in sim.houses.iter().filter(|h| !h.defunct && h.wealth > 1.0) {
        let home = h.hub as usize;
        let mut presence: Vec<(usize, f32)> = vec![(home, 2.0)];
        for &o in &h.offices { if o as usize != home { presence.push((o as usize, 1.0)); } }
        for &b in &h.bailos { if b as usize != home { presence.push((b as usize, 1.0)); } }
        let slices = blend(&presence, Some(home));
        if slices.is_empty() { continue; }
        houses.push(ReserveHolder {
            kind: "house".into(), name: h.name.clone(),
            seat: sim.hubs.get(home).map(|x| x.name.clone()).unwrap_or_default(),
            total: h.wealth, slices,
        });
    }
    houses.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));
    houses.truncate(40);

    Ok(ReservesPayload { cities, banks, houses })
}


/// DLC 3.5 · all chartered banks, richest (by equity) first.
#[tauri::command]
pub fn campaign_get_banks(db: State<'_, WorldDb>) -> Result<Vec<BankBrief>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<BankBrief> = sim.banks.iter().map(|b| {
        let seat = sim.hubs.get(b.seat as usize).map(|h| h.name.clone()).unwrap_or_default();
        // The bank banks in its seat city's coin (if minted) — denominate the sheet in it.
        let (coin_name, coin_value) = sim.hubs.get(b.seat as usize)
            .filter(|h| !h.coin_name.is_empty())
            .map(|h| (h.coin_name.clone(), crate::sim::tick::coin_value(h.mint_fineness, h.coin_trust)))
            .unwrap_or_else(|| (String::new(), 0.0));
        let owner = sim.houses.get(b.house as usize).map(|h| h.name.clone()).unwrap_or_default();
        let branches = b.branches.iter()
            .filter_map(|&hb| sim.hubs.get(hb as usize).map(|h| h.name.clone()))
            .collect();
        let events = b.events.iter().rev().take(12)
            .map(|e| e.text.clone()).collect();
        let (seat_x, seat_y) = sim.hubs.get(b.seat as usize).map(|h| (h.x, h.y)).unwrap_or((0.0, 0.0));
        // Live loans → deals list with borrower names + agreement terms.
        let loans: Vec<BankLoanRow> = b.loans.iter().filter(|l| l.outstanding > 0.01).map(|l| {
            let (borrower, kind) = if l.borrower_house >= 0 {
                match sim.houses.get(l.borrower_house as usize) {
                    Some(h) => (h.name.clone(), if h.is_guild { "guild" } else { "house" }.to_string()),
                    None => ("—".into(), "house".into()),
                }
            } else if l.borrower_polis >= 0 {
                (sim.hubs.get(l.borrower_polis as usize).map(|h| h.name.clone()).unwrap_or_default(), "polis".into())
            } else { ("—".into(), "house".into()) };
            BankLoanRow {
                borrower, borrower_kind: kind, purpose: l.purpose.clone(),
                principal: l.principal, outstanding: l.outstanding, rate: l.rate,
                start_year: l.start_tick / TICKS_PER_YEAR,
                term_years: l.term_ticks as f32 / TICKS_PER_YEAR as f32,
            }
        }).collect();
        // Equity stakes → works name + good.
        let stakes: Vec<BankStakeRow> = b.stakes.iter().map(|s| BankStakeRow {
            works: sim.hubs.get(s.estate_hub as usize).map(|h| h.name.clone()).unwrap_or_default(),
            good: sim.goods.get(s.good as usize).map(|g| g.name.clone()).unwrap_or_default(),
            share: s.share, basis: s.basis,
        }).collect();
        BankBrief {
            name: b.name.clone(), seat, coin_name, coin_value, owner,
            owner_idx: b.house,
            color: distinct_color(b.house as usize),
            founded_year: b.founded_tick / TICKS_PER_YEAR,
            defunct: b.defunct,
            reserves: b.reserves, loans_out: b.loans_outstanding(), real_estate: b.real_estate,
            deposits: b.deposits, notes_issued: b.notes_issued,
            equity: b.equity(), reserve_ratio: b.reserve_ratio(),
            n_loans: b.loans.iter().filter(|l| l.outstanding > 0.01).count() as u32,
            interest_earned: b.interest_earned, losses: b.losses,
            stake_book: b.stake_book(), dividends_earned: b.dividends_earned,
            seat_x, seat_y,
            branches, events,
            history: b.history.clone(), loans, stakes,
        }
    }).collect();
    out.sort_by(|a, b| {
        a.defunct.cmp(&b.defunct).then(
            b.equity.partial_cmp(&a.equity).unwrap_or(std::cmp::Ordering::Equal))
    });
    Ok(out)
}


/// DLC 3.5 · the log of regional financial crashes (newest first).
#[tauri::command]
pub fn campaign_get_crashes(db: State<'_, WorldDb>)
    -> Result<Vec<crate::sim::tick::CrashRecord>, String>
{
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out = sim.crashes.clone();
    out.reverse();
    Ok(out)
}


#[tauri::command]
pub fn campaign_get_wars(db: State<'_, WorldDb>) -> Result<WarsPayload, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(WarsPayload { active: vec![], log: vec![] }) };
    let name = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let active = sim.wars.iter().map(|w| WarBrief {
        a: name(w.a), b: name(w.b),
        start_year: w.start_tick / TICKS_PER_YEAR,
        years: sim.tick.saturating_sub(w.start_tick) / TICKS_PER_YEAR,
        chest_a: w.chest_a, chest_b: w.chest_b, levies: w.levies, cause: w.cause.clone(),
    }).collect();
    let mut log = sim.war_log.clone();
    log.reverse();
    Ok(WarsPayload { active, log })
}


/// LIVE per-city price baskets from the running campaign (the Economy Dashboard's
/// Prices tab). Unlike the frozen worldgen `EconomySnapshot`, this reads the sim's
/// current per-hub prices each call, so the panel updates as the campaign advances.
#[tauri::command]
pub fn campaign_city_price_index(db: State<'_, WorldDb>) -> Result<Vec<CityPriceIndex>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s,
        None => return Ok(vec![]),
    };
    let ng = sim.goods.len();
    let mut out: Vec<CityPriceIndex> = sim.hubs.iter().map(|h| {
        let mut num = 0.0f32;
        let mut den = 0.0f32;
        for g in 0..ng {
            let base = sim.goods[g].base_value;
            if base <= 0.0 { continue; }
            // Staples (low need_tier) weigh most, mirroring the frontend basket.
            let w = (3i32 - sim.goods[g].need_tier as i32).max(1) as f32;
            num += w * (h.price[g] / base);
            den += w;
        }
        CityPriceIndex { name: h.name.clone(), index: if den > 0.0 { (num / den) * 100.0 } else { 100.0 } }
    }).collect();
    out.sort_by(|a, b| a.index.partial_cmp(&b.index).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}


/// World-economy panel (M6): per-good world prices + the price-index series.
#[tauri::command]
pub fn campaign_get_world_economy(db: State<'_, WorldDb>) -> Result<WorldEconomy, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s,
        None => return Ok(WorldEconomy {
            goods: vec![], index_series: vec![],
            lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0,
            pop_house: 0.0, pop_local: 0.0, pop_guild: 0.0,
            lack_series: vec![], merchant_series: vec![], world_series: vec![],
            records: Default::default(),
        }),
    };
    let ng = sim.goods.len();
    let mut goods: Vec<WorldGoodPrice> = (0..ng)
        .map(|g| {
            let base = sim.goods[g].base_value.max(1e-3);
            let mut sum = 0.0;
            let mut producers = 0u32;
            let mut top = ("", 0.0f32);
            for h in &sim.hubs {
                sum += h.price[g] / base;
                if h.production[g] > 0.0 {
                    producers += 1;
                }
                if h.production[g] > top.1 {
                    top = (h.name.as_str(), h.production[g]);
                }
            }
            WorldGoodPrice {
                good: g,
                name: sim.goods[g].name.clone(),
                world_price: sum / sim.hubs.len().max(1) as f32,
                producers,
                top_hub: top.0.to_string(),
            }
        })
        .collect();
    goods.sort_by(|a, b| b.world_price.partial_cmp(&a.world_price).unwrap_or(std::cmp::Ordering::Equal));
    let index_series: Vec<[f32; 2]> = sim
        .journal
        .iter()
        .filter(|e| e.kind == "price" && e.hub == -1)
        .map(|e| [e.tick as f32, e.value])
        .collect();

    // ── World rollups: current population-weighted shortage + merchant totals ──
    let mut wpop = 0.0f32;
    let (mut lb, mut lc, mut ll) = (0.0f32, 0.0f32, 0.0f32);
    let (mut ph, mut pl, mut pg) = (0.0f32, 0.0f32, 0.0f32);
    for h in &sim.hubs {
        let p = h.population.max(0.0);
        wpop += p;
        lb += h.lack_basic * p;
        lc += h.lack_comfort * p;
        ll += h.lack_luxury * p;
        let (a, b, c) = crate::sim::tick::merchant_pops(h);
        ph += a; pl += b; pg += c;
    }
    let wp = wpop.max(1.0);

    // World time series, aggregated across hub histories (all sampled on the same
    // monthly ticks): population-weighted shortage + merchant-class totals.
    use std::collections::BTreeMap;
    let mut acc: BTreeMap<u32, [f32; 8]> = BTreeMap::new(); // [pop, lb, lc, ll, ph, pl, pg, _]
    for h in &sim.hubs {
        for s in &h.history {
            let e = acc.entry(s.tick).or_insert([0.0; 8]);
            let p = s.population.max(0.0);
            e[0] += p;
            e[1] += s.lack_basic * p;
            e[2] += s.lack_comfort * p;
            e[3] += s.lack_luxury * p;
            e[4] += s.pop_house;
            e[5] += s.pop_local;
            e[6] += s.pop_guild;
        }
    }
    let lack_series: Vec<[f32; 4]> = acc.iter()
        .map(|(&t, e)| { let p = e[0].max(1.0); [t as f32, e[1] / p, e[2] / p, e[3] / p] })
        .collect();
    let merchant_series: Vec<[f32; 4]> = acc.iter()
        .map(|(&t, e)| [t as f32, e[4], e[5], e[6]])
        .collect();

    Ok(WorldEconomy {
        goods, index_series,
        lack_basic: lb / wp, lack_comfort: lc / wp, lack_luxury: ll / wp,
        pop_house: ph, pop_local: pl, pop_guild: pg,
        lack_series, merchant_series,
        world_series: sim.world_series.clone(),
        records: sim.records.clone(),
    })
}
