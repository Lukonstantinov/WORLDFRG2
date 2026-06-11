//! World/campaign split: the WORLD (tiles + metadata: geography, climate,
//! rivers, goods belts…) is frozen by `finalize_world`; everything human —
//! settlements, economy, step 7-10 progress — lives in the `campaign` table
//! and is saved/opened as a separate `.campaign` file referencing the world by
//! its frozen fingerprint.
//!
//! Campaign steps (settlements, biological-trade) still write DERIVED tile
//! columns (habitability, hazard, goods belts) on a frozen world — that's
//! allowed: the freeze protects user-authored geography (paint + phases 1-6),
//! and the campaign's `world_ref` is the fingerprint RECORDED AT FINALIZE,
//! which later derived-column writes don't touch.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::{metadata, WorldDb};

/// Campaign keys carried by `.campaign` files (and stripped from world saves
/// via the whole-table strip — this list is what save/open copies explicitly).
const CAMPAIGN_KEYS: [&str; 7] = [
    "name",
    "settlements",
    "economy",
    "bio_params",
    "campaign_progress",
    "world_ref",
    // DLC 1 "Living Trade" tick-simulation state (serialized CampaignSim incl.
    // the append-only journal). Travels with the .campaign file.
    "campaign_sim",
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldRef {
    pub fingerprint: (i64, i64),
    pub grid_width: u32,
    pub grid_height: u32,
    pub world_name: String,
}

#[derive(Serialize)]
pub struct CampaignInfo {
    pub name: String,
    /// False when the campaign was made on a different (or re-finalized) world.
    pub world_match: bool,
    /// JSON step-completion map for the campaign wizard (steps 7-10).
    pub campaign_progress: Option<String>,
}

/// Fingerprint of the world's base tiles (lod 0 only — the LOD pyramid is a
/// derived cache). Stable across save/open round-trips because the SQLite
/// backup copies tile versions verbatim.
pub fn world_fingerprint(conn: &Connection) -> Result<(i64, i64), String> {
    conn.query_row(
        "SELECT COALESCE(SUM(version), 0), COUNT(*) FROM tiles WHERE lod = 0",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| e.to_string())
}

pub fn is_frozen(conn: &Connection) -> bool {
    metadata::get_meta(conn, "frozen").ok().flatten().as_deref() == Some("1")
}

/// Guard for commands that edit world geography (paint, template import,
/// sim phases 1-6 and the run-alls).
pub fn ensure_unfrozen(conn: &Connection) -> Result<(), String> {
    if is_frozen(conn) {
        return Err(
            "World is finalized (frozen). Unfreeze it to edit geography — note that \
             existing campaigns will no longer match the changed world."
                .into(),
        );
    }
    Ok(())
}

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

fn finalized_fp(conn: &Connection) -> Option<(i64, i64)> {
    let s = metadata::get_meta(conn, "finalized_fp").ok().flatten()?;
    let (a, b) = s.split_once(',')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

fn current_world_ref(conn: &Connection) -> Result<WorldRef, String> {
    let fingerprint = finalized_fp(conn)
        .ok_or_else(|| "Finalize the world before starting a campaign.".to_string())?;
    let grid_width: u32 = metadata::get_meta_required(conn, "grid_width")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let grid_height: u32 = metadata::get_meta_required(conn, "grid_height")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let world_name = metadata::get_meta(conn, "name")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Untitled".to_string());
    Ok(WorldRef { fingerprint, grid_width, grid_height, world_name })
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

// ═══════════════════════════════════════════════════════════════════════════
// DLC 1 "Living Trade" — tick simulation commands.
// ═══════════════════════════════════════════════════════════════════════════

use crate::sim::tick::{CampaignSim, House, JournalEntry, TickGood, TickHub};
use crate::commands::query_commands::EconomySnapshot;

/// Compact clock for the Campaign Clock UI.
#[derive(Serialize)]
pub struct CampaignClock {
    pub tick: u32,
    pub year: u32,
    pub day: u32,
    pub season: String,
    pub last_tick_ms: f32,
}

/// Per-hub summary for the campaign map/list (no per-good vectors).
#[derive(Serialize)]
pub struct HubBrief {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub population: u32,
    pub grain_wealth: f32,
    pub trade_wealth: f32,
    pub starving: f32,
    pub is_estate: bool,
    /// Overall population mood 0..1 (for an optional map tint).
    pub mood: f32,
    /// Month-over-month population growth fraction (+0.05 = +5%); 0 until two
    /// monthly history samples exist. Drives the "which cities grow/shrink" list.
    pub growth: f32,
}

/// What `campaign_start_sim` / `campaign_advance` / `campaign_get_state` return.
#[derive(Serialize)]
pub struct CampaignSnapshot {
    pub active: bool,
    pub clock: CampaignClock,
    pub hubs: Vec<HubBrief>,
    /// Most recent journal events (newest last), capped.
    pub recent_events: Vec<JournalEntry>,
    /// Population-weighted world price index (1.0 = baseline).
    pub price_index: f32,
    pub in_transit: u32,
    /// Total population across all hubs (shown as a number in the world pulse).
    pub total_population: u32,
    /// Population change since the last monthly chronicle sample.
    pub population_delta: i32,
    /// World price-index change since the last monthly chronicle sample.
    pub price_index_delta: f32,
}

/// M6 world-economy panel datum: one good's world price + who moves it.
#[derive(Serialize)]
pub struct WorldGoodPrice {
    pub good: usize,
    pub name: String,
    pub world_price: f32, // mean local price / base_value across hubs
    pub producers: u32,
    pub top_hub: String,
}

#[derive(Serialize)]
pub struct WorldEconomy {
    pub goods: Vec<WorldGoodPrice>,
    /// World price-index time series (tick, value) from the journal.
    pub index_series: Vec<[f32; 2]>,
}

/// One good's live state at a hub, for the settlement-window Market tab.
#[derive(Serialize)]
pub struct HubGoodDetail {
    pub good: usize,
    pub name: String,
    pub price: f32,        // local price in the grain-eq numeraire
    pub base_value: f32,   // world-standard value (price/base = ×-world)
    pub stock: f32,
    pub need: f32,         // approx per-tick demand (desire × population)
    pub production: f32,
    /// Cheapest / dearest world price (×-world) and the hub names there.
    pub world_min: f32,
    pub world_min_hub: String,
    pub world_max: f32,
    pub world_max_hub: String,
}

/// Full per-settlement detail for the redesigned settlement window (live campaign
/// state): sentiment, market, and history.
#[derive(Serialize)]
pub struct HubDetail {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub koppen: u8,
    pub coastal: bool,
    pub is_estate: bool,
    // sentiment
    pub mood: f32,
    pub sent_food: f32,
    pub sent_prosperity: f32,
    pub sent_stability: f32,
    // wealth / food
    pub grain_wealth: f32,
    pub trade_wealth: f32,
    pub food_balance: f32,
    pub starving: f32,
    pub goods: Vec<HubGoodDetail>,
    pub history: Vec<crate::sim::tick::HubSample>,
    pub events: Vec<JournalEntry>,
    /// Merchant families resident in this city (richest first).
    #[serde(default)]
    pub houses: Vec<HouseBrief>,
}

fn get_sim(conn: &Connection) -> Result<Option<CampaignSim>, String> {
    let raw = metadata::campaign_get(conn, "campaign_sim").map_err(|e| e.to_string())?;
    match raw {
        Some(s) if !s.is_empty() => {
            let mut sim: CampaignSim =
                serde_json::from_str(&s).map_err(|e| format!("campaign_sim parse: {e}"))?;
            sim.rebuild_routes(); // `days` is not serialized
            Ok(Some(sim))
        }
        _ => Ok(None),
    }
}

fn set_sim(conn: &Connection, sim: &CampaignSim) -> Result<(), String> {
    let json = serde_json::to_string(sim).map_err(|e| e.to_string())?;
    metadata::campaign_set(conn, "campaign_sim", &json).map_err(|e| e.to_string())
}

fn build_snapshot(sim: &CampaignSim) -> CampaignSnapshot {
    let hubs = sim
        .hubs
        .iter()
        .map(|h| {
            // Month-over-month growth from the last two per-hub history samples.
            let n = h.history.len();
            let growth = if n >= 2 && h.history[n - 2].population > 0.0 {
                (h.history[n - 1].population - h.history[n - 2].population)
                    / h.history[n - 2].population
            } else { 0.0 };
            HubBrief {
                id: h.id,
                x: h.x,
                y: h.y,
                name: h.name.clone(),
                population: h.population.max(0.0) as u32,
                grain_wealth: h.grain_wealth,
                trade_wealth: h.trade_wealth,
                starving: h.starving,
                is_estate: h.is_estate,
                mood: h.mood,
                growth,
            }
        })
        .collect();
    let total_population: f32 = sim.hubs.iter().map(|h| h.population.max(0.0)).sum();
    let population_delta = if sim.last_month_pop > 0.0 {
        (total_population - sim.last_month_pop) as i32
    } else { 0 };
    let recent_events: Vec<JournalEntry> = sim
        .journal
        .iter()
        .filter(|e| e.kind != "price")
        .rev()
        .take(40)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let price_index = sim
        .journal
        .iter()
        .rev()
        .find(|e| e.kind == "price")
        .map(|e| e.value)
        .unwrap_or(1.0);
    let price_index_delta = if sim.last_month_index > 0.0 {
        price_index - sim.last_month_index
    } else { 0.0 };
    CampaignSnapshot {
        active: true,
        clock: CampaignClock {
            tick: sim.tick,
            year: sim.year(),
            day: sim.day_of_year(),
            season: sim.season().to_string(),
            last_tick_ms: sim.last_tick_ms,
        },
        hubs,
        recent_events,
        price_index,
        in_transit: sim.in_transit.len() as u32,
        total_population: total_population.max(0.0) as u32,
        population_delta,
        price_index_delta,
    }
}

fn inactive_snapshot() -> CampaignSnapshot {
    CampaignSnapshot {
        active: false,
        clock: CampaignClock { tick: 0, year: 0, day: 0, season: "Spring".into(), last_tick_ms: 0.0 },
        hubs: vec![],
        recent_events: vec![],
        price_index: 1.0,
        in_transit: 0,
        total_population: 0,
        population_delta: 0,
        price_index_delta: 0.0,
    }
}

/// Deterministic initial head lifespan (≈45–75 years, in ticks).
fn seed_lifespan(seed: u64, salt: u64) -> u32 {
    let mut z = seed.wrapping_add(salt.wrapping_mul(0x9E3779B97F4A7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    let r = (z >> 40) as f32 / (1u64 << 24) as f32;
    ((45.0 + r * 30.0) * 365.0) as u32
}

fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let ra = uf_find(&mut *parent, a);
    let rb = uf_find(&mut *parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

/// Seed a fresh living-trade simulation from the static economy snapshot.
#[tauri::command]
pub fn campaign_start_sim(seed: u64, db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
            TickGood {
                name: econ.goods[g].clone(),
                category,
                need_tier: spec.map(|s| s.need_tier).unwrap_or(1),
                base_value: spec.map(|s| s.base_value).filter(|v| *v > 0.0).unwrap_or(1.0),
                desire: spec.map(|s| s.desire).unwrap_or(0.4),
                food,
            }
        })
        .collect();

    // ── Hubs ── (cap to the strongest 150 to bound tick cost + state size)
    let mut order: Vec<usize> = (0..econ.hubs.len()).collect();
    order.sort_by(|&a, &b| econ.hubs[b].population.cmp(&econ.hubs[a].population));
    order.truncate(150);
    order.sort_unstable(); // keep snapshot index order stable
    let id_to_idx: std::collections::HashMap<u32, usize> =
        order.iter().enumerate().map(|(i, &h)| (econ.hubs[h].id, i)).collect();

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
            let mut price: Vec<f32> = goods.iter().map(|g| g.base_value).collect();
            if let Some(m) = &eh.market {
                for mg in &m.prices {
                    if mg.good < gc {
                        price[mg.good] = mg.price.max(0.01);
                    }
                }
            }
            TickHub {
                id: eh.id,
                x: eh.x,
                y: eh.y,
                name: eh.name.clone(),
                population: (eh.population.max(1)) as f32,
                founding_pop: (eh.population.max(1)) as f32,
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
                history: Vec::new(),
            }
        })
        .collect();
    let nn = hubs.len();

    // ── Connectivity components from corridors + chains (goods move only within
    //    a component, so continents stay separate markets) ──
    let mut parent: Vec<usize> = (0..nn).collect();
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

    // ── Merchant houses: the strongest hubs get a named trading FAMILY
    //    specializing in their top goods, with a head of family who will age,
    //    die and be succeeded over the campaign ──
    let (gw, gh) = (world_ref.grid_width, world_ref.grid_height);
    let mut hub_order: Vec<usize> = (0..nn).collect();
    hub_order.sort_by(|&a, &b| {
        hubs[b].population.partial_cmp(&hubs[a].population).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut houses: Vec<House> = Vec::new();
    for &h in hub_order.iter().take(24) {
        let mut gi: Vec<usize> = (0..gc).collect();
        gi.sort_by(|&a, &b| {
            hubs[h].production[b].partial_cmp(&hubs[h].production[a]).unwrap_or(std::cmp::Ordering::Equal)
        });
        let spec: Vec<usize> = gi.into_iter().filter(|&g| hubs[h].production[g] > 0.0).take(2).collect();
        let (hx, hy) = (hubs[h].x.max(0.0) as u32, hubs[h].y.max(0.0) as u32);
        let family = crate::sim::names::gen_family_name(hx, hy, gw, gh, h as u64);
        let name = format!("House {family}");
        let head = crate::sim::names::gen_head_name(hx, hy, gw, gh, &family, 0x100 ^ h as u64);
        houses.push(House {
            name,
            hub: h as u32,
            wealth: 1.0,
            prestige: 0.0,
            spec,
            monopoly: vec![],
            rivals: vec![],
            generation: 1,
            head_name: head,
            head_since: 0,
            head_lifespan: seed_lifespan(seed, h as u64),
            founded_tick: 0,
            political_power: 0.0,
            volume: 0.0,
            defunct: false,
        });
    }

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
        freight_per_day: 0.012,
        k: 0.6,
        margin: 0.05,
        need_scale,
        world_w: grid_w,
        last_tick_ms: 0.0,
        last_month_pop: 0.0,
        last_month_index: 0.0,
        days: vec![],
    };
    sim.rebuild_routes();
    set_sim(&conn, &sim)?;
    Ok(build_snapshot(&sim))
}

/// Advance the living-trade sim by `ticks` days (autosaving the new state).
#[tauri::command]
pub fn campaign_advance(ticks: u32, db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut sim = get_sim(&conn)?
        .ok_or_else(|| "No active campaign sim — start it first.".to_string())?;
    let t0 = std::time::Instant::now();
    sim.advance(ticks.clamp(1, 3650));
    sim.last_tick_ms = t0.elapsed().as_secs_f32() * 1000.0 / ticks.max(1) as f32;
    set_sim(&conn, &sim)?; // append-only journal → incremental autosave
    Ok(build_snapshot(&sim))
}

/// Current sim state (inactive snapshot when no campaign sim has been started).
#[tauri::command]
pub fn campaign_get_state(db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(match get_sim(&conn)? {
        Some(sim) => build_snapshot(&sim),
        None => inactive_snapshot(),
    })
}

/// Journal rows, optionally filtered to a hub and/or good (−1 = any), for the
/// settlement window's price/event history.
#[tauri::command]
pub fn campaign_get_journal(
    hub: i32,
    good: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<JournalEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&conn)? {
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
#[tauri::command]
pub fn campaign_get_hub(id: u32, db: State<'_, WorldDb>) -> Result<Option<HubDetail>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&conn)? {
        Some(s) => s,
        None => return Ok(None),
    };
    let hi = match sim.hubs.iter().position(|h| h.id == id) {
        Some(i) => i,
        None => return Ok(None),
    };
    let hub = &sim.hubs[hi];
    let ng = sim.goods.len();
    // Per-good world cheapest/dearest (×-world price) across hubs.
    let goods: Vec<HubGoodDetail> = (0..ng)
        .map(|g| {
            let base = sim.goods[g].base_value.max(1e-3);
            let mut wmin = (f32::INFINITY, "");
            let mut wmax = (f32::NEG_INFINITY, "");
            for h in &sim.hubs {
                let xw = h.price[g] / base;
                if xw < wmin.0 { wmin = (xw, h.name.as_str()); }
                if xw > wmax.0 { wmax = (xw, h.name.as_str()); }
            }
            HubGoodDetail {
                good: g,
                name: sim.goods[g].name.clone(),
                price: hub.price[g],
                base_value: sim.goods[g].base_value,
                stock: hub.stock[g],
                need: sim.goods[g].desire * hub.population,
                production: hub.production[g],
                world_min: if wmin.0.is_finite() { wmin.0 } else { 0.0 },
                world_min_hub: wmin.1.to_string(),
                world_max: if wmax.0.is_finite() { wmax.0 } else { 0.0 },
                world_max_hub: wmax.1.to_string(),
            }
        })
        .collect();
    // Journal stores the hub INDEX (not id), so filter by the resolved index.
    let events: Vec<JournalEntry> = sim
        .journal
        .iter()
        .filter(|e| e.hub == hi as i32 && e.kind != "price")
        .cloned()
        .collect();
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
    }))
}

/// One merchant family for the Houses panel / settlement window.
#[derive(Serialize)]
pub struct HouseBrief {
    pub name: String,        // "House Cassii"
    pub head_name: String,   // "Marcus Cassii"
    pub home_hub: u32,       // home hub id
    pub home_name: String,
    pub wealth: f32,
    pub prestige: f32,
    pub political_power: f32,
    #[serde(default)] pub volume: f32, // recent trade volume — "trade amount" the house moves
    pub generation: u32,
    pub head_age: u32,       // years the current head has led
    pub specialties: Vec<String>,       // good names
    pub monopolies: Vec<(String, f32)>, // good name + share 0..1
    pub rivals: Vec<String>,            // rival house names
    pub defunct: bool,
    /// Stable, maximally-distinct colour for this house (hex, golden-angle hue).
    /// Same colour used everywhere: map overlay, houses panel, settlement pie.
    #[serde(default)] pub color: String,
    /// Home-seat position (world cell coords) — where the family is based.
    #[serde(default)] pub seat: [f32; 2],
    /// True when this house holds >=50% of its seat city's merchant trade — i.e.
    /// it controls that settlement. Only controlled seats are tinted on the map.
    #[serde(default)] pub dominant: bool,
    /// Trade-partner settlements (world cell coords): the cities its seat exchanges
    /// goods with (upstream + downstream). Used to colour the routes it controls.
    #[serde(default)] pub partners: Vec<[f32; 2]>,
    /// Names of the cities this house trades with (seat first, then partners) —
    /// shown in the Houses menu.
    #[serde(default)] pub cities: Vec<String>,
}

/// Golden-angle hue → a distinct, saturated hex colour. `i` is a stable index so
/// each house keeps its colour across refreshes.
fn distinct_color(i: usize) -> String {
    let hue = (i as f32 * 137.508) % 360.0;
    // Fixed S/L for vivid but readable colours on the green map.
    let (s, l) = (0.68f32, 0.55f32);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = hue / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    format!("#{:02x}{:02x}{:02x}", to(r1), to(g1), to(b1))
}

fn build_house_briefs(sim: &CampaignSim) -> Vec<HouseBrief> {
    let gname = |g: usize| sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
    let hub_name = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let seat_pos = |hub: usize| -> [f32; 2] {
        sim.hubs.get(hub).map(|x| [x.x, x.y]).unwrap_or([0.0, 0.0])
    };

    // ── Per-seat dominance: a house controls its seat city iff it holds >=50% of
    //    that city's merchant-house trade volume (its "trade influence"). ───────
    let nhubs = sim.hubs.len();
    let mut hub_house_vol: Vec<f32> = vec![0.0; nhubs];   // total resident house volume
    let mut hub_top: Vec<(usize, f32)> = vec![(usize::MAX, 0.0); nhubs]; // (house, vol)
    for (hi, h) in sim.houses.iter().enumerate() {
        if h.defunct { continue; }
        let hub = h.hub as usize;
        if hub >= nhubs { continue; }
        let v = h.volume.max(0.0001); // tiny floor so a lone house still "controls"
        hub_house_vol[hub] += v;
        if v > hub_top[hub].1 { hub_top[hub] = (hi, v); }
    }
    let dominant_of = |hi: usize| -> bool {
        let hub = sim.houses[hi].hub as usize;
        if hub >= nhubs || hub_top[hub].0 != hi { return false; }
        let share = hub_top[hub].1 / hub_house_vol[hub].max(1e-6);
        share >= 0.5
    };

    // ── Trade partners: cities the seat exchanges goods with (in-flight shipments
    //    to/from the seat). Captures both upstream suppliers and downstream buyers.
    let partners_of = |hub: usize| -> Vec<usize> {
        let mut set: Vec<usize> = Vec::new();
        for s in &sim.in_transit {
            let other = if s.from as usize == hub { Some(s.to as usize) }
                else if s.to as usize == hub { Some(s.from as usize) }
                else { None };
            if let Some(o) = other { if o != hub && o < nhubs && !set.contains(&o) { set.push(o); } }
        }
        set
    };

    let mut out: Vec<HouseBrief> = sim.houses.iter().enumerate().map(|(hi, h)| {
        let hub = h.hub as usize;
        let dominant = dominant_of(hi);
        // Only a controlling (dominant) house projects onto its trade routes.
        let partner_hubs = if dominant { partners_of(hub) } else { Vec::new() };
        let partners: Vec<[f32; 2]> = partner_hubs.iter().map(|&p| seat_pos(p)).collect();
        let mut cities: Vec<String> = Vec::new();
        if hub < nhubs { cities.push(hub_name(hub as u32)); }
        for &p in &partner_hubs { cities.push(hub_name(p as u32)); }
        HouseBrief {
            name: h.name.clone(),
            head_name: h.head_name.clone(),
            home_hub: sim.hubs.get(hub).map(|x| x.id).unwrap_or(0),
            home_name: hub_name(h.hub),
            wealth: h.wealth,
            prestige: h.prestige,
            political_power: h.political_power,
            volume: h.volume,
            generation: h.generation,
            head_age: sim.tick.saturating_sub(h.head_since) / 365,
            specialties: h.spec.iter().map(|&g| gname(g)).collect(),
            monopolies: h.monopoly.iter().map(|&(g, s)| (gname(g), s)).collect(),
            rivals: h.rivals.iter().filter_map(|&r| sim.houses.get(r).map(|x| x.name.clone())).collect(),
            defunct: h.defunct,
            color: distinct_color(hi), // stable per-house index → stable colour
            seat: seat_pos(hub),
            dominant,
            partners,
            cities,
        }
    }).collect();
    // Active first, then richest first.
    out.sort_by(|a, b| (a.defunct, -a.wealth).partial_cmp(&(b.defunct, -b.wealth))
        .unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// All merchant families (active first, richest first) for the Houses panel.
#[tauri::command]
pub fn campaign_get_houses(db: State<'_, WorldDb>) -> Result<Vec<HouseBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(match get_sim(&conn)? {
        Some(sim) => build_house_briefs(&sim),
        None => vec![],
    })
}

/// World-economy panel (M6): per-good world prices + the price-index series.
#[tauri::command]
pub fn campaign_get_world_economy(db: State<'_, WorldDb>) -> Result<WorldEconomy, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&conn)? {
        Some(s) => s,
        None => return Ok(WorldEconomy { goods: vec![], index_series: vec![] }),
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
    Ok(WorldEconomy { goods, index_series })
}

/// Migrate pre-split keys living in `metadata` into the campaign table (runs on
/// every world open; a no-op for already-split worlds). Returns true when
/// anything was migrated (i.e. the file was a legacy single-file save).
pub fn migrate_legacy_campaign_keys(conn: &Connection) -> Result<bool, String> {
    let mut migrated = false;
    for key in ["settlements", "economy"] {
        if let Some(v) = metadata::get_meta(conn, key).map_err(|e| e.to_string())? {
            // Don't clobber campaign data that's already present.
            if metadata::campaign_get(conn, key).map_err(|e| e.to_string())?.is_none() && !v.is_empty() {
                metadata::campaign_set(conn, key, &v).map_err(|e| e.to_string())?;
                migrated = true;
            }
            conn.execute("DELETE FROM metadata WHERE key = ?1", rusqlite::params![key])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(migrated)
}
