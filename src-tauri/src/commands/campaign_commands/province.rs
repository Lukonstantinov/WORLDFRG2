//! province commands — the LIVE land state of a province (FIX_PLAN B1) plus the
//! control verbs a holder has over it. Lives in `campaign_commands/` rather than
//! beside the read-only `campaign_province_state` in `sim_commands.rs` because the
//! verbs MUTATE the running sim, which needs `set_sim` / `persist_campaign`.
//!
//! Two things to keep in mind here:
//!
//! * `campaign_advance` was the only command that mutated a running sim. These are the
//!   first others, so each one follows the same discipline: validate, apply the SAME
//!   function the AI would call, then persist. No verb reaches into land state directly
//!   in a way the yearly pass could not also produce.
//! * Everything degrades to empty on a world with no province layer, exactly as the
//!   sim side does — a campaign without provinces must behave as it always did.
use super::*;
use crate::sim::tick::{
    PROV_TAX_MAX, ProvWork, WORK_COST, WORK_KINDS, WORK_YEARS,
};

/// One province's mutable land state, joined for the Province panel's Land tab.
#[derive(Serialize, Clone)]
pub struct ProvinceLand {
    pub id: u32,
    // ── land use (shares of the province) ──
    pub forest: f32,
    pub arable: f32,
    pub pasture: f32,
    /// Share of the arable under irrigation.
    pub irrigated: f32,
    /// Whatever is neither wood, crop nor pasture — moor, marsh, rock, sand.
    pub waste: f32,
    // ── condition ──
    pub soil: f32,
    pub unrest: f32,
    // ── people ──
    pub rural: f32,
    pub rural_cap: f32,
    pub urban: f32,
    /// rural ÷ capacity — above 1 the countryside is over its Malthusian limit.
    pub saturation: f32,
    // ── the economy of the land ──
    /// Last year's food surplus above rural subsistence (grain-eq).
    pub surplus: f32,
    /// Of that, what reached the holder's treasury as dues.
    pub revenue: f32,
    pub arrears: f32,
    pub tax_rate: f32,
    pub tax_max: f32,
    // ── tenure ──
    /// [civic/crown, house/noble, temple, common].
    pub tenure: [f32; 4],
    /// Houses holding an estate here, with their colour, for the tenure plate.
    pub holders: Vec<ProvinceHolder>,
    // ── administration ──
    /// Hub whose writ runs here, −1 = a frontier nobody collects from.
    pub holder_hub: i32,
    /// Phase 5 · a HOUSE whose writ runs here instead, −1 = the ordinary case (a
    /// city administers). The Stato da Mar case — see `HOUSE_INHERITANCE_AND_
    /// TERRITORY.md` Part D.
    pub holder_house: i32,
    /// The seat city's name, or the holding HOUSE's name when one holds this
    /// province instead.
    pub holder_name: String,
    /// Works under way.
    pub works: Vec<ProvinceWorkRow>,
    /// Yearly series for the time slider (oldest → newest).
    pub history: Vec<ProvinceLandSample>,
    /// The province's own chronicle.
    pub events: Vec<ProvinceEventRow>,
}

#[derive(Serialize, Clone)]
pub struct ProvinceHolder {
    pub house: u32,
    pub name: String,
    pub color: String,
    /// Estates this house holds in the province.
    pub estates: u32,
}

#[derive(Serialize, Clone)]
pub struct ProvinceWorkRow {
    pub kind: u8,
    pub label: String,
    pub progress: f32,
    /// Years still to run at full funding.
    pub years_left: f32,
    pub yearly_cost: f32,
    pub funder: String,
    /// True when the work is stalled for want of money.
    pub stalled: bool,
}

#[derive(Serialize, Clone)]
pub struct ProvinceLandSample {
    pub year: u32,
    pub rural: f32,
    pub urban: f32,
    pub forest: f32,
    pub arable: f32,
    pub pasture: f32,
    pub irrigated: f32,
    pub soil: f32,
    pub unrest: f32,
    pub surplus: f32,
}

#[derive(Serialize, Clone)]
pub struct ProvinceEventRow {
    pub year: u32,
    pub kind: String,
    pub text: String,
}

/// Live land state for one province, or `None` when no campaign / no province layer.
#[tauri::command]
pub fn campaign_province_land(id: u32, db: State<'_, WorldDb>) -> Result<Option<ProvinceLand>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    Ok(build_province_land(&sim, id))
}

/// Every province's land state at once — what the browser sorts and filters on.
#[tauri::command]
pub fn campaign_province_land_all(db: State<'_, WorldDb>) -> Result<Vec<ProvinceLand>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let np = sim.prov_rural.len();
    Ok((0..np).filter_map(|p| build_province_land(&sim, p as u32)).collect())
}

fn build_province_land(sim: &CampaignSim, id: u32) -> Option<ProvinceLand> {
    let p = id as usize;
    if p >= sim.prov_rural.len() || sim.prov_forest.len() <= p { return None; }
    let forest = sim.prov_forest[p];
    let arable = sim.prov_arable[p];
    let pasture = sim.prov_pasture[p];
    let mut urban = 0.0f32;
    // Estates per owning house, for the tenure plate's colour blocks.
    let mut by_house: Vec<(u32, u32)> = Vec::new();
    for h in 0..sim.hubs.len() {
        if sim.hubs[h].abandoned { continue; }
        if sim.hub_province.get(h).copied().unwrap_or(-1) != p as i32 { continue; }
        if sim.hubs[h].is_estate {
            let oh = sim.hubs[h].owner_house;
            if oh >= 0 {
                match by_house.iter_mut().find(|(x, _)| *x == oh as u32) {
                    Some(e) => e.1 += 1,
                    None => by_house.push((oh as u32, 1)),
                }
            }
        } else {
            urban += sim.hubs[h].population.max(0.0);
        }
    }
    by_house.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let holders = by_house.iter().filter_map(|&(hi, n)| {
        sim.houses.get(hi as usize).map(|h| ProvinceHolder {
            house: hi, name: h.name.clone(), color: distinct_color(hi as usize), estates: n,
        })
    }).collect();
    let holder_hub = sim.prov_holder.get(p).copied().unwrap_or(-1);
    // Phase 5 · a HOUSE may hold this province's writ instead of the seat city (the
    // Stato da Mar case) — `holder_house` names it explicitly, and `holder_name`
    // reads as the house's own name rather than the city's when one does.
    let holder_house = sim.prov_holder_house.get(p).copied().unwrap_or(-1);
    let holder_name = if holder_house >= 0 {
        sim.houses.get(holder_house as usize).map(|h| h.name.clone()).unwrap_or_default()
    } else if holder_hub >= 0 {
        sim.hubs.get(holder_hub as usize).map(|h| h.name.clone()).unwrap_or_default()
    } else {
        // Not yet run a land pass — name the largest town anyway so the panel is not
        // blank on a campaign that has not ticked a year.
        sim.province_seat_hub(p).map(|h| sim.hubs[h].name.clone()).unwrap_or_default()
    };
    let works = sim.prov_works.iter().filter(|w| w.province == id).map(|w| {
        let k = (w.kind as usize).min(WORK_KINDS.len() - 1);
        let funder = if w.funder_hub >= 0 {
            sim.hubs.get(w.funder_hub as usize).map(|h| h.name.clone()).unwrap_or_default()
        } else if w.funder_house >= 0 {
            sim.houses.get(w.funder_house as usize).map(|h| h.name.clone()).unwrap_or_default()
        } else { String::new() };
        ProvinceWorkRow {
            kind: w.kind, label: WORK_KINDS[k].to_string(), progress: w.progress,
            years_left: ((1.0 - w.progress) * WORK_YEARS[k]).max(0.0),
            yearly_cost: WORK_COST[k], funder, stalled: w.idle_years > 0,
        }
    }).collect();
    let cap = sim.prov_cap.get(p).copied().unwrap_or(0.0);
    let rural = sim.prov_rural[p];
    Some(ProvinceLand {
        id,
        forest, arable, pasture,
        irrigated: sim.prov_irrigated.get(p).copied().unwrap_or(0.0),
        waste: (1.0 - forest - arable - pasture).clamp(0.0, 1.0),
        soil: sim.prov_soil.get(p).copied().unwrap_or(0.0),
        unrest: sim.prov_unrest.get(p).copied().unwrap_or(0.0),
        rural, rural_cap: cap, urban,
        saturation: if cap > 0.0 { rural / cap } else { 0.0 },
        surplus: sim.prov_surplus.get(p).copied().unwrap_or(0.0),
        revenue: sim.prov_revenue.get(p).copied().unwrap_or(0.0),
        arrears: sim.prov_arrears.get(p).copied().unwrap_or(0.0),
        tax_rate: sim.prov_tax.get(p).copied().unwrap_or(0.0),
        tax_max: PROV_TAX_MAX,
        tenure: sim.prov_tenure.get(p).copied().unwrap_or([0.0; 4]),
        holders,
        holder_hub, holder_house, holder_name,
        works,
        history: sim.prov_history.get(p).map(|v| v.iter().map(|s| ProvinceLandSample {
            year: s.year, rural: s.rural, urban: s.urban, forest: s.forest,
            arable: s.arable, pasture: s.pasture, irrigated: s.irrigated,
            soil: s.soil, unrest: s.unrest, surplus: s.surplus,
        }).collect()).unwrap_or_default(),
        events: sim.prov_events.get(p).map(|v| v.iter().map(|e| ProvinceEventRow {
            year: e.year, kind: e.kind.clone(), text: e.text.clone(),
        }).collect()).unwrap_or_default(),
    })
}

// ═════════════════════════════════════════════════════════════════════════════════
//  CONTROL VERBS
//
//  Before these, `campaign_advance(ticks)` was the ONLY command that mutated a running
//  simulation (FIX_PLAN B2). A province is the right place to break that, because a
//  decision about a province has a PLACE — the player can see what changed.
// ═════════════════════════════════════════════════════════════════════════════════

/// Set a province's rural tax rate. This is the holder's decision, so it is refused
/// for a province nobody administers — a frontier has no-one to collect the dues.
#[tauri::command]
pub fn campaign_set_province_tax(id: u32, rate: f32, db: State<'_, WorldDb>) -> Result<f32, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // The resident sim is shared behind an `Arc`; a mutating verb takes its own copy,
    // edits it, and hands it back through `set_sim` — the same shape `campaign_advance`
    // uses via `Arc::make_mut`. These verbs are rare player actions, so the clone is
    // not on any hot path.
    let mut sim = match get_sim(&db, &conn)? {
        Some(s) => (*s).clone(), None => return Err("no campaign running".into()),
    };
    let p = id as usize;
    if p >= sim.prov_rural.len() { return Err("no such province".into()); }
    sim.ensure_province_land(sim.prov_rural.len());
    if sim.province_seat_hub(p).is_none() {
        return Err("no town administers this province — there is nobody to collect".into());
    }
    let clamped = rate.clamp(0.0, PROV_TAX_MAX);
    let before = sim.prov_tax[p];
    sim.prov_tax[p] = clamped;
    if (clamped - before).abs() > 0.001 {
        let yr = sim.tick / crate::sim::tick::TICKS_PER_YEAR;
        let pn = sim.province_name(p);
        let dir = if clamped > before { "raised" } else { "lowered" };
        sim.push_prov_event(p, yr, "tax",
            format!("Dues {} to {:.0}% of the surplus in {}", dir, clamped * 100.0, pn));
    }
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?;
    Ok(clamped)
}

/// Begin a multi-year land improvement in a province, funded by a city treasury (a
/// polis) or a house's wealth. Deliberately reuses the same funded-progress shape the
/// satellite-construction system uses: the work stalls when unpaid rather than failing.
#[tauri::command]
pub fn campaign_start_province_work(
    id: u32, kind: u8, funder_hub: i32, funder_house: i32, db: State<'_, WorldDb>,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // The resident sim is shared behind an `Arc`; a mutating verb takes its own copy,
    // edits it, and hands it back through `set_sim` — the same shape `campaign_advance`
    // uses via `Arc::make_mut`. These verbs are rare player actions, so the clone is
    // not on any hot path.
    let mut sim = match get_sim(&db, &conn)? {
        Some(s) => (*s).clone(), None => return Err("no campaign running".into()),
    };
    let p = id as usize;
    if p >= sim.prov_rural.len() { return Err("no such province".into()); }
    let k = kind as usize;
    if k >= WORK_KINDS.len() { return Err("no such work".into()); }
    sim.ensure_province_land(sim.prov_rural.len());
    if sim.prov_works.iter().any(|w| w.province == id && w.kind == kind) {
        return Err(format!("{} is already under way here", WORK_KINDS[k]));
    }
    // A funder must exist and be able to carry the first year.
    let cost = WORK_COST[k];
    if funder_hub >= 0 {
        let fh = funder_hub as usize;
        match sim.hubs.get(fh) {
            Some(h) if h.treasury >= cost => {}
            Some(h) => return Err(format!("{} cannot fund the first year ({:.0} needed, {:.0} in treasury)",
                h.name, cost, h.treasury.max(0.0))),
            None => return Err("no such city".into()),
        }
    } else if funder_house >= 0 {
        let fs = funder_house as usize;
        match sim.houses.get(fs) {
            Some(h) if !h.defunct && h.wealth >= cost * 1.5 => {}
            Some(h) => return Err(format!("{} cannot carry the cost", h.name)),
            None => return Err("no such house".into()),
        }
    } else {
        return Err("a work needs a funder".into());
    }
    // Clearance needs woodland to clear; irrigation cannot exceed the whole arable.
    if kind == crate::sim::tick::WORK_CLEAR && sim.prov_forest[p] < 0.08 {
        return Err("there is no woodland left worth clearing here".into());
    }
    if kind == crate::sim::tick::WORK_IRRIGATE && sim.prov_irrigated[p] >= 0.95 {
        return Err("the land here is already watered".into());
    }
    sim.prov_works.push(ProvWork {
        province: id, kind, progress: 0.0,
        funder_hub, funder_house, start_tick: sim.tick, idle_years: 0,
    });
    let yr = sim.tick / crate::sim::tick::TICKS_PER_YEAR;
    let pn = sim.province_name(p);
    let label = WORK_KINDS[k];
    sim.push_prov_event(p, yr, label, format!("{} begun in {}", label, pn));
    let msg = format!("{} begun in {} — about {:.0} years at {:.0}/yr",
        label, pn, WORK_YEARS[k], cost);
    sim.journal.push(crate::sim::tick::JournalEntry {
        tick: sim.tick, kind: "public_works".into(), hub: funder_hub, good: -1, value: 0.0,
        text: msg.clone(),
    });
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?;
    Ok(msg)
}

/// Abandon a work in progress. What has been paid is sunk — no refund, which is what
/// makes committing to one a real decision.
#[tauri::command]
pub fn campaign_cancel_province_work(id: u32, kind: u8, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // The resident sim is shared behind an `Arc`; a mutating verb takes its own copy,
    // edits it, and hands it back through `set_sim` — the same shape `campaign_advance`
    // uses via `Arc::make_mut`. These verbs are rare player actions, so the clone is
    // not on any hot path.
    let mut sim = match get_sim(&db, &conn)? {
        Some(s) => (*s).clone(), None => return Err("no campaign running".into()),
    };
    sim.prov_works.retain(|w| !(w.province == id && w.kind == kind));
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?;
    Ok(())
}

/// CITY_PROVINCE_WAR_PLAN.md §2.5 · one good's exploitation reading in a province.
/// Pure derived read — `potential`/`actual`/`exploitation`/`market_share` are all
/// recomputed fresh from current state every call; only `depletion` is persisted
/// simulation state (see `update_province_goods_pressure`).
#[derive(Serialize)]
pub struct ProvinceGoodExploit {
    pub good: u8,
    /// What this land could yield at full, undepleted, appropriately-worked
    /// capacity — belt score × live land-use share × the world's calibrated yield.
    pub potential: f32,
    /// Actual production of hubs + estates here this tick.
    pub actual: f32,
    /// `actual / potential`. Below 1.0 = slack; above 1.0 = the land is being
    /// pushed past what it can sustainably give (the soft cap — see `depletion`).
    pub exploitation: f32,
    /// 0..1 accumulated overexploitation pressure eroding `potential` — a mine
    /// that "exhausts" barely recovers, a fishery that "collapses and recovers"
    /// bounces back fast, a vineyard doesn't accrue this at all (§1.2).
    pub depletion: f32,
    /// Share of `actual` that leaves the province via trade rather than being
    /// consumed by the very population that produced it (§2.5's market↔local
    /// split, from the real per-hub local-demand formula).
    pub market_share: f32,
}

/// Every good this province actually produces (or has produced recently enough
/// that depletion hasn't fully healed) — "only goods actually produced here," no
/// unexploited-opportunity view (§1.2/§5.5). Sorted by output, largest first.
#[tauri::command]
pub fn campaign_province_goods(id: u32, db: State<'_, WorldDb>) -> Result<Vec<ProvinceGoodExploit>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let ng = sim.goods.len();
    let np = if ng > 0 { sim.prov_good_belt.len() / ng } else { 0 };
    let p = id as usize;
    if p >= np { return Ok(vec![]); }
    let actual_all = sim.province_good_actual();
    let mut out = Vec::new();
    for g in 0..ng {
        let idx = p * ng + g;
        let belt = sim.prov_good_belt.get(idx).copied().unwrap_or(0.0);
        if belt <= 0.001 { continue; } // never producible here at all
        let actual = actual_all.get(idx).copied().unwrap_or(0.0);
        let depletion = sim.prov_good_depletion.get(idx).copied().unwrap_or(0.0);
        // §5.5 (simplified — no separate "last produced" year is tracked): a good
        // stays listed while it is producing now OR depletion hasn't healed away,
        // which already covers a recently-worked-then-idled mine or fishery.
        if actual <= 1e-4 && depletion <= 1e-4 { continue; }
        let potential = sim.province_good_potential(p, g);
        let exploitation = if potential > 1e-6 { actual / potential } else { 0.0 };
        let market_share = sim.province_good_market_share(p, g, actual);
        out.push(ProvinceGoodExploit { good: g as u8, potential, actual, exploitation, depletion, market_share });
    }
    out.sort_by(|a, b| b.actual.partial_cmp(&a.actual).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

/// #9 · One good this province COULD yield, whether or not it is worked today —
/// the "unexploited opportunity" view `campaign_province_goods` deliberately omits.
#[derive(Serialize)]
pub struct ProvinceGoodPotential {
    pub good: u8,
    pub name: String,       // good id (frontend maps to label/emoji)
    /// Live potential yield (belt × land-use share × world yield scale).
    pub potential: f32,
    /// Frozen belt richness 0..1 — how good the LAND is for this good, ignoring use.
    pub belt: f32,
    /// Actual production here this tick (0 = an untapped opportunity).
    pub actual: f32,
    /// An ore/mineral good (placed by real geology, richness = deposit grade).
    pub is_deposit: bool,
    /// A sea/coast good (fisheries, whaling, pearls, coral, …) — belongs on the
    /// province's COAST, not its interior, so the survey plate can place it there.
    pub is_marine: bool,
    /// For a deposit good: mean working grade in this province (0..1).
    pub mean_grade: f32,
    /// For a deposit good: number of ore workings of this good in the province.
    pub workings: u32,
    /// For a deposit good: the deepest/least-workable depth present (0 surface … 3 flooded).
    pub best_depth: u8,
    /// GOODS_LOCALITIES_PLAN.md Slice 7 — this good has at least one locality
    /// (terroir patch) inside the province.
    #[serde(default)]
    pub has_locality: bool,
    /// Mean locality grade (0..1) in this province, when `has_locality`.
    #[serde(default)]
    pub mean_locality_grade: f32,
    /// Count of localities of this good inside the province.
    #[serde(default)]
    pub locality_count: u32,
}

/// #9 · One ore working located in a province, for the survey-plate goods layer.
#[derive(Serialize)]
pub struct ProvinceDepositDot {
    pub good: String,
    pub x: u32,
    pub y: u32,
    pub grade: f32,
    pub extent: u8,
    pub depth: u8,
}

/// GOODS_LOCALITIES_PLAN.md Slice 6 — one terroir locality located in a province,
/// for the survey-plate goods layer. Deliberately the same shape as
/// `ProvinceDepositDot` (Slice 3's own design note: `GoodLocality` mirrors
/// `deposits::Deposit` for exactly this reuse).
#[derive(Serialize)]
pub struct ProvinceLocalityDot {
    pub good: String,
    pub x: u32,
    pub y: u32,
    pub grade: f32,
    pub extent: u8,
    pub radius_km: f32,
    pub name: String,
    pub river_fed: bool,
}

#[derive(Serialize)]
pub struct ProvincePotential {
    /// Every good the land here can yield (belt > 0), richest potential first.
    pub goods: Vec<ProvinceGoodPotential>,
    /// The individual ore workings inside this province (real cell coords), so the
    /// minimap can plot where the deposits are.
    pub deposits: Vec<ProvinceDepositDot>,
    /// The individual goods localities inside this province (real cell coords) —
    /// the non-mineral counterpart of `deposits` (Slice 6).
    #[serde(default)]
    pub localities: Vec<ProvinceLocalityDot>,
}

/// #9 · The full goods PICTURE for one province: what the land could yield (with
/// richness), and where the ore workings actually sit — so a province that produces
/// nothing today still shows its potential and its mineral deposits instead of a
/// bare "no notable produce". Pure read; nothing is written.
#[tauri::command]
pub fn campaign_province_potential(id: u32, db: State<'_, WorldDb>) -> Result<ProvincePotential, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s,
        None => return Ok(ProvincePotential { goods: vec![], deposits: vec![], localities: vec![] }),
    };
    let ng = sim.goods.len();
    let np = if ng > 0 { sim.prov_good_belt.len() / ng } else { 0 };
    let p = id as usize;
    if p >= np { return Ok(ProvincePotential { goods: vec![], deposits: vec![], localities: vec![] }); }
    let actual_all = sim.province_good_actual();

    // Which goods are ore/mineral deposits (richness = deposit grade, not a belt).
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let is_deposit: std::collections::HashMap<String, bool> = specs.iter()
        .map(|s| (s.id.clone(), matches!(s.distribution, crate::sim::goods_spec::Distribution::Deposits)))
        .collect();
    // Sea/coast goods — placed on the province coast, not its interior.
    use crate::sim::goods_spec::Domain;
    let is_marine: std::collections::HashMap<String, bool> = specs.iter()
        .map(|s| (s.id.clone(), matches!(s.domain, Domain::Marine | Domain::Coastal)))
        .collect();

    // Ore workings, attributed to a province via the downsampled raster.
    let deposits: Vec<crate::sim::deposits::Deposit> = metadata::get_meta(&conn, "deposits")
        .ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let (rw, rh, gw, gh, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or((0, 0, 0, 0, Vec::new()));
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    let prov_at = |x: u32, y: u32| -> i32 {
        if raster.is_empty() || gw == 0 || gh == 0 { return -1; }
        let rx = ((x as u64 * rw as u64) / gw as u64).min(rw as u64 - 1) as usize;
        let ry = ((y as u64 * rh as u64) / gh as u64).min(rh as u64 - 1) as usize;
        match raster.get(ry * rw as usize + rx).copied() {
            Some(v) if v != crate::sim::provinces::NO_PROVINCE => v as i32,
            _ => -1,
        }
    };

    // Aggregate deposits in this province by good, and collect the per-working dots.
    let mut dots: Vec<ProvinceDepositDot> = Vec::new();
    let mut agg: std::collections::HashMap<String, (f32, u32, u8)> = std::collections::HashMap::new(); // (grade_sum, n, max_depth)
    for d in &deposits {
        if prov_at(d.x, d.y) != id as i32 { continue; }
        dots.push(ProvinceDepositDot { good: d.good.clone(), x: d.x, y: d.y, grade: d.grade, extent: d.extent, depth: d.depth });
        let e = agg.entry(d.good.clone()).or_insert((0.0, 0, 0));
        e.0 += d.grade; e.1 += 1; e.2 = e.2.max(d.depth);
    }

    // GOODS_LOCALITIES_PLAN.md Slice 6 — the non-mineral counterpart of the
    // deposit aggregation above, same province-attribution helper (`prov_at`).
    let localities: Vec<crate::sim::localities::GoodLocality> = metadata::get_meta(&conn, "good_localities")
        .ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut loc_dots: Vec<ProvinceLocalityDot> = Vec::new();
    let mut loc_agg: std::collections::HashMap<String, (f32, u32)> = std::collections::HashMap::new(); // (grade_sum, n)
    for l in &localities {
        if prov_at(l.x, l.y) != id as i32 { continue; }
        loc_dots.push(ProvinceLocalityDot {
            good: l.good.clone(), x: l.x, y: l.y, grade: l.grade, extent: l.extent,
            radius_km: l.radius_km, name: l.name.clone(), river_fed: l.river_fed,
        });
        let e = loc_agg.entry(l.good.clone()).or_insert((0.0, 0));
        e.0 += l.grade; e.1 += 1;
    }

    let mut goods: Vec<ProvinceGoodPotential> = Vec::new();
    for g in 0..ng {
        let idx = p * ng + g;
        let belt = sim.prov_good_belt.get(idx).copied().unwrap_or(0.0);
        if belt <= 0.001 { continue; } // the land genuinely can't yield this
        let name = sim.goods[g].name.clone();
        let dep = is_deposit.get(&name).copied().unwrap_or(false);
        let mar = is_marine.get(&name).copied().unwrap_or(false);
        let (mean_grade, workings, best_depth) = agg.get(&name)
            .map(|&(s, n, bd)| (if n > 0 { s / n as f32 } else { 0.0 }, n, bd))
            .unwrap_or((0.0, 0, 0));
        let (has_locality, mean_locality_grade, locality_count) = loc_agg.get(&name)
            .map(|&(s, n)| (n > 0, if n > 0 { s / n as f32 } else { 0.0 }, n))
            .unwrap_or((false, 0.0, 0));
        goods.push(ProvinceGoodPotential {
            good: g as u8,
            name,
            potential: sim.province_good_potential(p, g),
            belt,
            actual: actual_all.get(idx).copied().unwrap_or(0.0),
            is_deposit: dep,
            is_marine: mar,
            mean_grade, workings, best_depth,
            has_locality, mean_locality_grade, locality_count,
        });
    }
    // Richest potential first (a deposit good's richness is reflected in its belt).
    goods.sort_by(|a, b| b.potential.partial_cmp(&a.potential).unwrap_or(std::cmp::Ordering::Equal)
        .then(b.belt.partial_cmp(&a.belt).unwrap_or(std::cmp::Ordering::Equal)));
    Ok(ProvincePotential { goods, deposits: dots, localities: loc_dots })
}

// ═════════════════════════════════════════════════════════════════════════════════
//  REALMS (`REALM_AND_GOVERNMENT_PLAN.md`, phase R1) — a proclaimed country made
//  into a territory for the map.
//
//  Through R1a/R1b a "state" was a pure DERIVED read: whichever provinces a tier
//  1-2 city administered, grouped by that city, recomputed from scratch every call
//  and never remembered. That derivation is gone. `sim.realms` + `sim.prov_realm`
//  are now the source of truth — a realm is proclaimed once (`promote_house_to_
//  realm`, `tick/realms.rs`) and persists in the save exactly like a house does, so
//  a capital's tier later dropping below 2 no longer makes its realm vanish from
//  the map (the Karakorum rule, plan §1.3/§4.3 — that guarantee was the entire
//  point of moving off a derived read). This query still costs nothing per tick
//  and cannot desync, for the same "display-time read" reason `get_province_
//  terrain_crop` gives for §2.3 — it just now reads REAL state instead of
//  recomputing an approximation of it.
// ═════════════════════════════════════════════════════════════════════════════════

#[derive(Serialize, Clone)]
pub struct StateRegion {
    /// Index into `sim.realms` — pass to `campaign_get_realm_family`.
    pub id: u32,
    pub capital_hub: u32,
    pub name: String,
    /// The ruler's style — "King", "Sovereign" — drawn at proclamation (placeholder
    /// vocabulary; see `tick/realms.rs`'s module doc for the culture-derived namer
    /// this is standing in for).
    pub title: String,
    pub color: [u8; 3],
    pub cells: Vec<[f32; 2]>,
    pub cell_size: f32,
    /// Label centroid.
    pub x: f32,
    pub y: f32,
    pub province_count: u32,
    /// The province ids this realm holds. The frontend tints exactly these cells of
    /// the province raster, so a realm's border is the province border — never an
    /// approximating "cell cloud".
    pub province_ids: Vec<u32>,
    /// `REALM_CITY_STATE`..`REALM_HEGEMON`.
    pub rank: u8,
    pub founded_tick: u32,
    /// The ruling dynasty's name — the house that proclaimed this realm, ELEVATED
    /// (`House.crowned`) rather than dissolved. Empty if the record is somehow gone
    /// (should not happen; a realm's `ruling_house` index is never reused).
    pub ruling_house: String,
    pub legitimacy: f32,
    pub cohesion: f32,
    pub treasury: f32,
    pub debts: f32,
}

/// A realm's territory tint — the same golden-angle hue rotation `distinct_color`
/// (house heraldry) uses, phase-shifted and desaturated so a realm's fill never
/// reads as a house's coat-of-arms colour on the same map even where a REALM id and
/// a HOUSE id numerically collide — two different index spaces. Keyed on the
/// realm's own id (stable for its whole life), not its capital hub, so a future
/// capital move (the Karakorum rule) never reassigns the colour.
fn state_color(realm_id: usize) -> [u8; 3] {
    let hue = (realm_id as f32 * 137.508 + 53.0) % 360.0;
    let (s, l) = (0.40f32, 0.44f32);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = hue / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [to(r1), to(g1), to(b1)]
}

/// Every realm ever proclaimed and still standing (a fallen realm — `fallen_tick >
/// 0` — is kept in `sim.realms` for the record, exactly as a defunct house is, but
/// draws no territory). Empty on a world with no realms, no province layer, or no
/// campaign, exactly as `campaign_province_land_all` degrades.
#[tauri::command]
pub fn compute_states(db: State<'_, WorldDb>) -> Result<Vec<StateRegion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    if sim.realms.is_empty() { return Ok(vec![]); }
    let (rw, rh, gw, gh, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        match metadata::get_meta(&conn, "province_raster").map_err(|e| e.to_string())? {
            Some(s) => serde_json::from_str(&s).unwrap_or((0, 0, 0, 0, Vec::new())),
            None => return Ok(vec![]),
        };
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    if raster.is_empty() || gw == 0 || gh == 0 || rw == 0 || rh == 0 { return Ok(vec![]); }

    let csx = gw as f32 / rw as f32;
    let csy = gh as f32 / rh as f32;
    let cell_size = csx.max(csy);
    let mut by_realm: std::collections::HashMap<u32, Vec<[f32; 2]>> = std::collections::HashMap::new();
    let mut prov_ids: std::collections::HashMap<u32, std::collections::HashSet<u32>> = std::collections::HashMap::new();
    for ry in 0..rh {
        for rx in 0..rw {
            let idx = (ry * rw + rx) as usize;
            let pid = raster.get(idx).copied().unwrap_or(crate::sim::provinces::NO_PROVINCE);
            if pid == crate::sim::provinces::NO_PROVINCE { continue; }
            let p = pid as usize;
            let rid = sim.prov_realm.get(p).copied().unwrap_or(-1);
            if rid < 0 { continue; }
            let rid = rid as u32;
            by_realm.entry(rid).or_default().push([rx as f32 * csx, ry as f32 * csy]);
            prov_ids.entry(rid).or_default().insert(pid);
        }
    }

    let mut out: Vec<StateRegion> = Vec::new();
    for realm in &sim.realms {
        if realm.fallen_tick > 0 { continue; } // kept for the record, draws nothing
        let Some(cells) = by_realm.get(&realm.id) else { continue }; // no mapped territory (shouldn't happen)
        if cells.is_empty() { continue; }
        let n = cells.len() as f32;
        let (sx, sy) = cells.iter().fold((0.0f32, 0.0f32), |(ax, ay), c| (ax + c[0], ay + c[1]));
        let ids: Vec<u32> = prov_ids.get(&realm.id).map(|s| {
            let mut v: Vec<u32> = s.iter().copied().collect();
            v.sort_unstable();
            v
        }).unwrap_or_default();
        let ruling_house = sim.houses.get(realm.ruling_house as usize)
            .map(|h| h.name.clone()).unwrap_or_default();
        out.push(StateRegion {
            id: realm.id,
            capital_hub: realm.capital_hub,
            name: realm.name.clone(),
            title: realm.title.clone(),
            color: state_color(realm.id as usize),
            cell_size,
            x: sx / n + cell_size * 0.5,
            y: sy / n + cell_size * 0.5,
            cells: cells.clone(),
            province_count: ids.len() as u32,
            province_ids: ids,
            rank: realm.rank,
            founded_tick: realm.founded_tick,
            ruling_house,
            legitimacy: realm.legitimacy,
            cohesion: realm.cohesion,
            treasury: realm.treasury,
            debts: realm.debts,
        });
    }
    out.sort_by(|a, b| b.cells.len().cmp(&a.cells.len()));
    Ok(out)
}

// ═════════════════════════════════════════════════════════════════════════════════
//  GENEALOGY (R2, `REALM_AND_GOVERNMENT_PLAN.md` §3.7) — a flat, read-only list of
//  a realm's family. Not yet the SVG family tree the plan's §4.2 sketches (that is
//  real follow-up work — laying out generations/marriages legibly is its own
//  design problem); this is the minimum that makes "who rules, who's next, who was
//  born" actually visible in the app, which is the part with load-bearing risk.
// ═════════════════════════════════════════════════════════════════════════════════

#[derive(Serialize, Clone)]
pub struct PersonBrief {
    /// Index into the realm's `family` — stable for the realm's whole life, used
    /// as the React key and to resolve `father`/`mother`/`spouse` client-side.
    pub idx: u32,
    pub name: String,
    pub female: bool,
    pub age: u32,
    pub alive: bool,
    pub father: i32,
    pub mother: i32,
    pub spouse: i32,
    pub is_ruler: bool,
    pub is_regent: bool,
    pub epithet: String,
    /// 0 if this person has never reigned.
    pub reign_years: u32,
}

/// One realm's family, in the order it was built (the founder first, then whoever
/// married in or was born, chronologically — `family` is append-only so this is
/// also insertion order). Empty if the realm id is unknown or its family hasn't
/// family hasn't been seeded yet (should not happen post-coronation — `promote_
/// house_to_realm` always seeds the founder — but degrades rather than erroring,
/// the same discipline every other campaign query here follows).
#[tauri::command]
pub fn campaign_get_realm_family(realm_id: u32, db: State<'_, WorldDb>) -> Result<Vec<PersonBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let Some(realm) = sim.realms.get(realm_id as usize) else { return Ok(vec![]) };
    let tick = sim.tick;
    let ticks_per_year = crate::sim::tick::TICKS_PER_YEAR;
    let out: Vec<PersonBrief> = realm.family.iter().enumerate().map(|(i, p)| {
        let alive = p.died_tick == 0;
        let age_at = if alive { tick } else { p.died_tick };
        let reign_years = if p.reign_start == 0 { 0 } else {
            let end = if p.reign_end == 0 { tick } else { p.reign_end };
            end.saturating_sub(p.reign_start) / ticks_per_year
        };
        PersonBrief {
            idx: i as u32, name: p.name.clone(), female: p.female,
            age: age_at.saturating_sub(p.born_tick) / ticks_per_year, alive,
            father: p.father, mother: p.mother, spouse: p.spouse,
            is_ruler: realm.ruler == i as i32, is_regent: realm.regent == i as i32,
            epithet: p.epithet.clone(), reign_years,
        }
    }).collect();
    Ok(out)
}
