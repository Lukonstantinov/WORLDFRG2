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

// ═══════════════════════════════════════════════════════════════════════════
// DLC 1 "Living Trade" — tick simulation commands.
// ═══════════════════════════════════════════════════════════════════════════

use crate::sim::tick::{CampaignSim, House, JournalEntry, SpecCenter, TickGood, TickHub};
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
    /// Current population-weighted fraction of demand unmet, by need tier.
    #[serde(default)] pub lack_basic: f32,
    #[serde(default)] pub lack_comfort: f32,
    #[serde(default)] pub lack_luxury: f32,
    /// Current world merchant population totals by class.
    #[serde(default)] pub pop_house: f32,
    #[serde(default)] pub pop_local: f32,
    #[serde(default)] pub pop_guild: f32,
    /// World time series `[tick, basic, comfort, luxury]` (population-weighted unmet).
    #[serde(default)] pub lack_series: Vec<[f32; 4]>,
    /// World time series `[tick, houses, local, guild]` merchant population totals.
    #[serde(default)] pub merchant_series: Vec<[f32; 4]>,
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
    /// Mean ×-world price across all settlements right now (the "world average"
    /// reference line on the Market price graphs).
    #[serde(default)] pub world_avg: f32,
}

/// One shipment touching a settlement (Market tab arrivals/departures), with its
/// owner so houses/guilds and round-trip return legs are identifiable.
#[derive(Serialize, Clone)]
pub struct ShipmentRow {
    pub owner: String,          // house / guild name, or "Local merchants"
    pub color: String,
    pub is_guild: bool,
    pub other: String,          // origin city (arrivals) or destination (departures)
    pub good: String,
    pub amount: f32,
    pub price: f32,             // ×-world price
    pub value: f32,             // amount × local price (the ranking key)
    pub sea: bool,
    pub returning_home: bool,   // a round-trip RETURN leg (goods bought abroad, coming home)
}

/// DLC 3 · a polis seat's government, for the settlement Government subtab: the
/// council house and the fiscal policy it sets, plus this seat's speculation read.
#[derive(Serialize, Clone, Default)]
pub struct Government {
    /// Governing (dominant, non-guild) house name, or "—" if none holds the seat.
    pub council: String,
    pub council_color: String,
    pub council_archetype: String,
    pub council_is_guild: bool,
    /// The council house's soft political power 0..1.
    pub council_power: f32,
    /// Effective export / import tariff fractions in force (council policy, or the
    /// global default until a council sets one).
    pub tariff_export: f32,
    pub tariff_import: f32,
    /// True when these are the global defaults (no council policy set yet).
    pub tariff_default: bool,
    /// Mint fineness (1.0 = full-bodied coin, < 1 = debased "cheap money").
    pub mint_fineness: f32,
    pub treasury: f32,
    pub civic_pool: f32,
    /// This seat's speculation read (empty tier = none / below the noise floor).
    pub spec_risk: f32,
    pub spec_tier: String,
    pub spec_stars: u8,
    pub spec_pattern: String,
    /// The ranked causal "why" clauses (largest driver first).
    pub spec_drivers: Vec<String>,
    pub spec_watch: Vec<String>,
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
    /// Recent supply arriving by SEA (ships) vs LAND (caravans) — decaying tally.
    #[serde(default)] pub in_by_sea: f32,
    #[serde(default)] pub in_by_land: f32,
    /// Current fraction of demand UNMET, by need tier (0 = supplied, 1 = none met).
    #[serde(default)] pub lack_basic: f32,
    #[serde(default)] pub lack_comfort: f32,
    #[serde(default)] pub lack_luxury: f32,
    /// Estimated merchant population in this city by class.
    #[serde(default)] pub pop_house: f32,
    #[serde(default)] pub pop_local: f32,
    #[serde(default)] pub pop_guild: f32,
    /// For an estate: its kind (0 none / 1 farm / 2 mine / 3 plantation / 4 fishery /
    /// 5 vineyard), who owns it, and the good it works — for the inspector.
    #[serde(default)] pub estate_kind: u8,
    #[serde(default)] pub estate_owner: String,
    #[serde(default)] pub estate_good: String,
    /// Buildings erected here: (name, one-line effect) — for the inspector.
    #[serde(default)] pub structures: Vec<(String, String)>,
    /// DLC 3 · the polis government of this seat (council + fiscal policy + this
    /// city's speculation read). None for estates / unsettled hubs.
    #[serde(default)] pub government: Option<Government>,
    /// FOREIGN merchant offices hosted in this settlement (houses/guilds based
    /// elsewhere who have opened a counting-house here).
    #[serde(default)] pub offices_here: Vec<OfficeHere>,
    /// Market flow: in-flight shipments arriving / departing (ranked by value).
    #[serde(default)] pub arrivals: Vec<ShipmentRow>,
    #[serde(default)] pub departures: Vec<ShipmentRow>,
    /// Recently COMPLETED deals (most recent first), by direction.
    #[serde(default)] pub recent_arrivals: Vec<ShipmentRow>,
    #[serde(default)] pub recent_departures: Vec<ShipmentRow>,
    /// Recent trade value bought (imports) vs sold (exports), decaying tallies.
    #[serde(default)] pub bought: f32,
    #[serde(default)] pub sold: f32,
    /// Estates & manufactories in this city's hinterland.
    #[serde(default)] pub estates_here: Vec<EstateRow>,
}

/// One estate / manufactory in a settlement's hinterland (host-side view).
#[derive(Serialize)]
pub struct EstateRow {
    pub name: String,
    pub kind: u8,            // 1 farm/2 mine/3 plantation/4 fishery/5 vineyard/6 manufactory
    pub good: String,
    pub output: f32,         // current production/day of its good
    pub owner: String,       // owning house/guild, or "City of …"
    pub owner_is_guild: bool,
    pub tier: u8,            // upgrade tier 1..5
}

/// One foreign merchant's office hosted in a settlement (host-side view).
#[derive(Serialize)]
pub struct OfficeHere {
    pub holder: String,        // house / guild name
    pub color: String,         // its stable map colour
    pub is_guild: bool,
    pub origin: String,        // the city the holder is based in
    pub throughput_pct: f32,   // % of THIS settlement's live trade it handles
    pub goods: Vec<String>,    // goods it currently moves through here
}

/// Parse the campaign sim from the DB into the resident cache ONCE. After this the
/// query commands read the live object instead of re-parsing JSON every call.
fn ensure_campaign_loaded(cache: &mut crate::db::CampaignCache, conn: &Connection) -> Result<(), String> {
    if cache.loaded {
        return Ok(());
    }
    // Make the organic culture map active so houses/guilds founded this campaign are
    // named in their home city's culture (no-op when none is stored).
    crate::sim::cultures::ensure_active(conn);
    let raw = metadata::campaign_get(conn, "campaign_sim").map_err(|e| e.to_string())?;
    cache.sim = match raw {
        Some(s) if !s.is_empty() => {
            let mut sim: CampaignSim =
                serde_json::from_str(&s).map_err(|e| format!("campaign_sim parse: {e}"))?;
            sim.rebuild_routes(); // `days` / `neighbors` are not serialized
            Some(sim)
        }
        _ => None,
    };
    cache.loaded = true;
    cache.dirty = false;
    Ok(())
}

/// A CLONE of the resident sim (loading it from the DB once if needed). Read-only
/// commands use this; it never re-parses JSON after the first load.
fn get_sim(db: &WorldDb, conn: &Connection) -> Result<Option<CampaignSim>, String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    ensure_campaign_loaded(&mut cache, conn)?;
    Ok(cache.sim.clone())
}

/// Replace the resident sim and mark it dirty (the DB row is flushed later by
/// `persist_campaign` / the advance cadence). Used by commands that build or rewrite
/// the whole sim.
fn set_sim(db: &WorldDb, sim: &CampaignSim) -> Result<(), String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    cache.sim = Some(sim.clone());
    cache.loaded = true;
    cache.dirty = true;
    Ok(())
}

/// Flush the resident sim to the DB metadata row if it has unsaved changes. Called on
/// pause/save/close and periodically from `campaign_advance`. Caller holds `conn`.
fn persist_campaign(db: &WorldDb, conn: &Connection) -> Result<(), String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    if cache.dirty {
        if let Some(sim) = &cache.sim {
            let json = serde_json::to_string(sim).map_err(|e| e.to_string())?;
            metadata::campaign_set(conn, "campaign_sim", &json).map_err(|e| e.to_string())?;
        }
        cache.dirty = false;
        cache.last_persist = Some(std::time::Instant::now());
    }
    Ok(())
}

/// Frontend-invokable flush — called when the Play loop pauses and before the app
/// closes, so unsaved in-memory ticks reach the DB.
#[tauri::command]
pub fn campaign_persist(db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    persist_campaign(&db, &conn)
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
            // The static economy emits NORMALISED [0,1] outputs; lift them into
            // readable per-day quantities. Uniform & balance-neutral: need_scale,
            // stock and prices all scale with it, so only the displayed magnitudes
            // change (trade stops looking like "≈0").
            for v in production.iter_mut() { *v *= 1000.0; }
            let mut price: Vec<f32> = goods.iter().map(|g| g.base_value).collect();
            if let Some(m) = &eh.market {
                for mg in &m.prices {
                    if mg.good < gc {
                        price[mg.good] = mg.price.max(0.01);
                    }
                }
            }
            let founding = (eh.population.max(1)) as f32;
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
                tw_house: 0.0,
                tw_local: 0.0,
                tw_guild: 0.0,
                estate_kind: 0,
                estate_tier: 0,
                owner_house: -1,
                structures: vec![],
                treasury: 0.0,
                tariff_export: 0.0,
                tariff_import: 0.0,
                mint_fineness: 1.0,
                council_house: -1,
                coin_name: String::new(),
                coin_trust: 0.0,
                mint_fineness_prev: 0.0,
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

    // ── Guarantee a FOOD surplus ──────────────────────────────────────────────
    // `need_scale` balances *total* production against *total* need, but FOOD is
    // only a fraction of all goods. A world rich in luxuries but modest in cereal
    // would seed a chronic food deficit → world-wide famine → population collapse
    // (the 8M→1M crash). So measure the actual food need (with the same demand
    // pressure the tick uses) vs food production and, if food is short, scale every
    // hub's food output up to a healthy surplus. Production scales with live
    // population thereafter, so this ratio is population-invariant; tech growth then
    // makes food steadily MORE abundant over the campaign. We only ever raise food
    // (`max(1.0)`), never cut a world that already grows plenty.
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
    for &h in hub_order.iter().take(24) {
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
            head_lifespan: seed_lifespan(seed, h as u64),
            founded_tick: 0,
            political_power: 0.0,
            volume: 0.0,
            defunct: false,
            archetype: crate::sim::tick::pick_archetype(seed, h as u64),
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
        });
    }

    let houses_len = houses.len() as u32; // baseline house count (before `houses` is moved)
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
        world_h: world_ref.grid_height.max(1) as f32,
        last_tick_ms: 0.0,
        last_month_pop: 0.0,
        last_month_index: 0.0,
        seed_house_count: houses_len,
        fleets_migrated: true, // new campaigns already seed fleets
        tech_factor: 1.0,
        percap_migrated: true, // hubs seeded with base_per_capita directly
        house_ledger: Vec::new(),
        house_ledger_prev: Vec::new(),
        house_barred: Vec::new(),
        colonizable: econ.colonizable_sites.clone(),
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
        days: vec![],
        neighbors: vec![],
        routes_dirty: false,
        warehouses: vec![],
        contracts: vec![],
        trade_cur: Default::default(),
        city_dominator: vec![],
        trade_last: vec![],
        trade_hist: vec![],
    };
    sim.rebuild_routes();
    sim.seed_initial_guilds(); // civic guilds for cities already ≥ 50k people
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?; // write the fresh campaign to the DB immediately
    Ok(build_snapshot(&sim))
}

/// Advance the living-trade sim by `ticks` days. The sim is mutated in place in the
/// resident cache (no JSON round-trip per call) and flushed to the DB only on a year
/// boundary or every few seconds of wall time — see `CampaignCache`.
#[tauri::command]
pub fn campaign_advance(ticks: u32, db: State<'_, WorldDb>) -> Result<CampaignSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    ensure_campaign_loaded(&mut cache, &conn)?;
    // Whether enough wall time has passed to warrant a flush (read before borrowing
    // `sim`, which borrows the cache).
    let stale = cache.last_persist.map(|t| t.elapsed().as_secs_f32() > 5.0).unwrap_or(true);
    let (snap, json) = {
        let sim = cache.sim.as_mut()
            .ok_or_else(|| "No active campaign sim — start it first.".to_string())?;
        let year_before = sim.year();
        let t0 = std::time::Instant::now();
        sim.advance(ticks.clamp(1, 3650));
        sim.last_tick_ms = t0.elapsed().as_secs_f32() * 1000.0 / ticks.max(1) as f32;
        let snap = build_snapshot(sim);
        // Serialize ONLY when it's time to autosave (year rolled over or wall-clock
        // cadence); otherwise the change just stays resident and dirty.
        let json = if sim.year() != year_before || stale {
            Some(serde_json::to_string(sim).map_err(|e| e.to_string())?)
        } else {
            None
        };
        (snap, json)
    };
    match json {
        Some(j) => {
            metadata::campaign_set(&conn, "campaign_sim", &j).map_err(|e| e.to_string())?;
            cache.dirty = false;
            cache.last_persist = Some(std::time::Instant::now());
        }
        None => cache.dirty = true,
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

/// Journal rows, optionally filtered to a hub and/or good (−1 = any), for the
/// settlement window's price/event history.
#[tauri::command]
pub fn campaign_get_journal(
    hub: i32,
    good: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<JournalEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
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
    let sim = match get_sim(&db, &conn)? {
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
            let mut wsum = 0.0f32;
            for h in &sim.hubs {
                let xw = h.price[g] / base;
                if xw < wmin.0 { wmin = (xw, h.name.as_str()); }
                if xw > wmax.0 { wmax = (xw, h.name.as_str()); }
                wsum += xw;
            }
            let world_avg = if !sim.hubs.is_empty() { wsum / sim.hubs.len() as f32 } else { 1.0 };
            HubGoodDetail {
                good: g,
                name: sim.goods[g].name.clone(),
                price: hub.price[g],
                base_value: sim.goods[g].base_value,
                stock: hub.stock[g],
                // The REAL per-tick demand the sim consumes (matches base_need in
                // tick.rs): pop × tier-weight × desire × need_scale × demand pressure.
                // Showing raw desire×pop made every good read "0% of need".
                need: hub.population
                    * [1.0f32, 0.45, 0.22][sim.goods[g].need_tier.min(2) as usize]
                    * sim.goods[g].desire.max(0.0)
                    * sim.need_scale
                    * crate::sim::tick::DEMAND_PRESSURE,
                production: hub.production[g],
                world_min: if wmin.0.is_finite() { wmin.0 } else { 0.0 },
                world_min_hub: wmin.1.to_string(),
                world_max: if wmax.0.is_finite() { wmax.0 } else { 0.0 },
                world_max_hub: wmax.1.to_string(),
                world_avg,
            }
        })
        .collect();
    // ── Market flow: in-flight shipments arriving at / departing this hub, each
    //    tagged with its owner (house/guild/local) and round-trip return legs,
    //    ranked by value (highest first). ──
    let owner_name = |o: i32| if o >= 0 {
        sim.houses.get(o as usize).map(|h| h.name.clone()).unwrap_or_else(|| "—".into())
    } else { "Local merchants".into() };
    let owner_color = |o: i32| if o >= 0 { distinct_color(o as usize) } else { "#7a8aa0".to_string() };
    let owner_is_guild = |o: i32| o >= 0 && sim.houses.get(o as usize).map(|h| h.is_guild).unwrap_or(false);
    let cname = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let mk_row = |s: &crate::sim::tick::InTransit, other: u32| -> ShipmentRow {
        let g = s.good.min(ng.saturating_sub(1));
        let base = sim.goods[g].base_value.max(1e-3);
        ShipmentRow {
            owner: owner_name(s.owner), color: owner_color(s.owner), is_guild: owner_is_guild(s.owner),
            other: cname(other), good: sim.goods[g].name.clone(), amount: s.amount,
            price: hub.price[g] / base, value: s.amount * hub.price[g], sea: s.sea,
            returning_home: s.phase == 1,
        }
    };
    let mut arrivals: Vec<ShipmentRow> = Vec::new();
    let mut departures: Vec<ShipmentRow> = Vec::new();
    for s in &sim.in_transit {
        if s.good >= ng { continue; }
        if s.to as usize == hi { arrivals.push(mk_row(s, s.from)); }
        if s.from as usize == hi { departures.push(mk_row(s, s.to)); }
    }
    let by_value = |a: &ShipmentRow, b: &ShipmentRow|
        b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal);
    arrivals.sort_by(by_value); arrivals.truncate(40);
    departures.sort_by(by_value); departures.truncate(40);
    // Recently completed deals (most recent first), split by direction at this hub.
    let mk_recent = |r: &crate::sim::tick::RecentTrade, other: u32| -> ShipmentRow {
        let g = r.good.min(ng.saturating_sub(1));
        let base = sim.goods[g].base_value.max(1e-3);
        ShipmentRow {
            owner: owner_name(r.owner), color: owner_color(r.owner), is_guild: owner_is_guild(r.owner),
            other: cname(other), good: sim.goods[g].name.clone(), amount: r.amount,
            price: r.price / base, value: r.amount * r.price, sea: r.sea, returning_home: false,
        }
    };
    let mut recent_arrivals: Vec<ShipmentRow> = Vec::new();
    let mut recent_departures: Vec<ShipmentRow> = Vec::new();
    for r in sim.recent_trades.iter().rev() {
        if r.good >= ng { continue; }
        if r.to as usize == hi && recent_arrivals.len() < 12 { recent_arrivals.push(mk_recent(r, r.from)); }
        if r.from as usize == hi && recent_departures.len() < 12 { recent_departures.push(mk_recent(r, r.to)); }
        if recent_arrivals.len() >= 12 && recent_departures.len() >= 12 { break; }
    }
    let bought = hub.import_spend;
    let sold = hub.export_earn;
    // ── Estates & manufactories in this city's hinterland ──
    let estates_here: Vec<EstateRow> = sim.hubs.iter()
        .filter(|e| e.is_estate && e.parent == hi as i32)
        .map(|e| {
            let g = e.base_per_capita.iter().enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i).unwrap_or(0);
            let (owner, owner_is_guild) = if e.owner_house >= 0 {
                let h = sim.houses.get(e.owner_house as usize);
                (h.map(|x| x.name.clone()).unwrap_or_default(), h.map(|x| x.is_guild).unwrap_or(false))
            } else {
                (format!("City of {}", hub.name), false)
            };
            EstateRow {
                name: e.name.clone(),
                kind: e.estate_kind,
                good: sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default(),
                output: e.production.get(g).copied().unwrap_or(0.0),
                owner,
                owner_is_guild,
                tier: e.estate_tier.max(1),
            }
        })
        .collect();
    // Journal stores the hub INDEX (not id), so filter by the resolved index.
    let events: Vec<JournalEntry> = sim
        .journal
        .iter()
        .filter(|e| e.hub == hi as i32 && e.kind != "price" && e.kind != "voyage_loss")
        .cloned()
        .collect();
    let (pop_house, pop_local, pop_guild) = crate::sim::tick::merchant_pops(hub);
    // Estate descriptors for the inspector (kind, owner, worked good).
    let (estate_owner, estate_good) = if hub.is_estate {
        let owner = if hub.owner_house >= 0 {
            sim.houses.get(hub.owner_house as usize).map(|h| h.name.clone()).unwrap_or_default()
        } else if hub.parent >= 0 {
            sim.hubs.get(hub.parent as usize).map(|h| format!("City of {}", h.name)).unwrap_or_default()
        } else { String::new() };
        // The worked good = the one this estate actually produces (max per-capita).
        let g = hub.base_per_capita.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i).unwrap_or(0);
        (owner, sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default())
    } else { (String::new(), String::new()) };
    // ── Foreign offices hosted here ──────────────────────────────────────────
    // Live throughput touching this hub, split by the owning holder, + the goods
    // each currently moves through here. Only holders with an OFFICE here listed.
    let mut hub_total = 0.0f32;
    let mut per_holder: std::collections::HashMap<usize, (f32, std::collections::HashSet<usize>)> =
        std::collections::HashMap::new();
    for s in &sim.in_transit {
        if s.from as usize == hi || s.to as usize == hi {
            hub_total += s.amount.max(0.0);
            if s.owner >= 0 {
                let e = per_holder.entry(s.owner as usize).or_default();
                e.0 += s.amount.max(0.0);
                e.1.insert(s.good);
            }
        }
    }
    let offices_here: Vec<OfficeHere> = sim.houses.iter().enumerate()
        .filter(|(_, h)| !h.defunct && h.offices.contains(&(hi as u32)))
        .map(|(idx, h)| {
            let (vol, gset) = per_holder.get(&idx).cloned().unwrap_or((0.0, Default::default()));
            OfficeHere {
                holder: h.name.clone(),
                color: distinct_color(idx),
                is_guild: h.is_guild,
                origin: sim.hubs.get(h.hub as usize).map(|x| x.name.clone()).unwrap_or_default(),
                throughput_pct: if hub_total > 1e-6 { vol / hub_total * 100.0 } else { 0.0 },
                goods: gset.iter()
                    .filter_map(|&g| sim.goods.get(g).map(|x| x.name.clone()))
                    .collect(),
            }
        })
        .collect();
    // ── DLC 3 · the polis government of this seat (non-estate settlements only) ──
    let government = if hub.is_estate {
        None
    } else {
        use crate::sim::tick::{archetype_label, EXPORT_TAX_RATE, IMPORT_TAX_RATE};
        let ci = hub.council_house;
        let (council, council_color, council_archetype, council_is_guild, council_power) =
            if ci >= 0 {
                if let Some(house) = sim.houses.get(ci as usize) {
                    (house.name.clone(), distinct_color(ci as usize),
                     archetype_label(house.archetype).to_string(), house.is_guild, house.political_power)
                } else { ("—".into(), "#7a8aa0".into(), String::new(), false, 0.0) }
            } else { ("—".into(), "#7a8aa0".into(), String::new(), false, 0.0) };
        let tariff_default = hub.tariff_export <= 0.0 && hub.tariff_import <= 0.0;
        let tariff_export = if hub.tariff_export > 0.0 { hub.tariff_export } else { EXPORT_TAX_RATE };
        let tariff_import = if hub.tariff_import > 0.0 { hub.tariff_import } else { IMPORT_TAX_RATE };
        let spec = sim.spec_centers.iter().find(|c| c.hub == hub.id);
        let (spec_risk, spec_tier, spec_stars, spec_pattern, spec_drivers, spec_watch) = match spec {
            Some(c) => (c.risk, c.tier.clone(), c.stars, c.pattern_tag.clone(),
                c.drivers.iter().map(|d| d.detail.clone()).filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                c.watch_goods.clone()),
            None => (0.0, String::new(), 0, String::new(), vec![], vec![]),
        };
        Some(Government {
            council, council_color, council_archetype, council_is_guild, council_power,
            tariff_export, tariff_import, tariff_default,
            mint_fineness: if hub.mint_fineness <= 0.0 { 1.0 } else { hub.mint_fineness },
            treasury: hub.treasury, civic_pool: hub.civic_pool,
            spec_risk, spec_tier, spec_stars, spec_pattern, spec_drivers, spec_watch,
        })
    };
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
        in_by_sea: hub.in_by_sea,
        in_by_land: hub.in_by_land,
        lack_basic: hub.lack_basic,
        lack_comfort: hub.lack_comfort,
        lack_luxury: hub.lack_luxury,
        pop_house,
        pop_local,
        pop_guild,
        estate_kind: hub.estate_kind,
        estate_owner,
        estate_good,
        offices_here,
        arrivals,
        departures,
        recent_arrivals,
        recent_departures,
        bought,
        sold,
        estates_here,
        structures: hub.structures.iter().map(|&s| (
            crate::sim::tick::structure_label(s).to_string(),
            crate::sim::tick::structure_effect(s).to_string(),
        )).collect(),
        government,
    }))
}

/// One merchant family for the Houses panel / settlement window.
#[derive(Serialize)]
pub struct HouseBrief {
    /// Index into `sim.houses` — the key for per-house detail queries (the ledger).
    #[serde(default)] pub idx: u32,
    /// Phase G — city names this house is currently BARRED from (active trade wars).
    #[serde(default)] pub barred: Vec<String>,
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
    /// Settlements this house CONTROLS (world cell coords): any city — its seat OR
    /// a distant outpost — where it handles >=50% of the trade throughput. A
    /// big-city family can completely control a remote outpost it supplies and
    /// funnel its wealth home.
    #[serde(default)] pub controls: Vec<[f32; 2]>,
    /// Trade-partner / handled settlements (world cell coords) used to colour the
    /// routes the house runs.
    #[serde(default)] pub partners: Vec<[f32; 2]>,
    /// Names of the cities this house trades with / controls (seat first) — shown
    /// in the Houses menu.
    #[serde(default)] pub cities: Vec<String>,
    /// Archetype + its label/perk and any goods it holds city charters on.
    #[serde(default)] pub archetype: u8,
    #[serde(default)] pub archetype_label: String,
    #[serde(default)] pub archetype_perk: String,
    #[serde(default)] pub charters: Vec<String>,
    /// The house's transport capital — each vessel is one concurrent shipment slot.
    #[serde(default)] pub fleet_sea: u32,
    #[serde(default)] pub fleet_river: u32,
    #[serde(default)] pub fleet_caravan: u32,
    /// True = a civic Merchant Guild (acts for its home city), not a private house.
    #[serde(default)] pub is_guild: bool,
    /// Foreign cities where this holder has opened an OFFICE: `(name, [x,y])`.
    #[serde(default)] pub offices: Vec<(String, [f32; 2])>,
    /// Estates/manufactories this holder owns: `(good, host-city)`.
    #[serde(default)] pub estates: Vec<(String, String)>,
    /// Cities this house is active in, ranked MOST → LEAST influential, each tagged
    /// with its role (seat / bailo / dominant / office / trade) + contested flag.
    #[serde(default)] pub active: Vec<HouseCity>,
}

/// One city a house operates in, for the influence-ranked "Active in" list.
#[derive(Serialize, Clone)]
pub struct HouseCity {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub influence: f32,
    /// "seat" | "bailo" | "dominant" | "office" | "trade".
    pub role: String,
    /// A rival also holds significant influence here.
    pub contested: bool,
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

    // ── Throughput-based control ─────────────────────────────────────────────
    // Every in-flight shipment touches two cities (its source and destination). A
    // house that OWNS the shipment "handles" that trade at both ends. A house
    // controls a settlement when it handles >=50% of that settlement's total trade
    // throughput — its own seat OR a remote outpost it supplies. Guild/independent
    // trade (owner -1) counts toward the total but controls nothing, so a city
    // mostly served by guilds has no controlling house.
    let nhubs = sim.hubs.len();
    let nh = sim.houses.len();
    let mut hub_total: Vec<f32> = vec![0.0; nhubs];        // all throughput per hub
    let mut hub_house: Vec<f32> = vec![0.0; nhubs * nh.max(1)]; // [hub*nh + house]
    for s in &sim.in_transit {
        let amt = s.amount.max(0.0);
        for &h in &[s.from as usize, s.to as usize] {
            if h < nhubs {
                hub_total[h] += amt;
                if s.owner >= 0 {
                    let oi = s.owner as usize;
                    if oi < nh { hub_house[h * nh + oi] += amt; }
                }
            }
        }
    }
    // Per house: the hubs it controls (>=50% throughput) and the hubs it trades at.
    let mut controlled: Vec<Vec<usize>> = vec![Vec::new(); nh];
    let mut handled: Vec<Vec<usize>> = vec![Vec::new(); nh];
    for h in 0..nhubs {
        if hub_total[h] <= 1e-6 { continue; }
        for oi in 0..nh {
            let v = hub_house[h * nh + oi];
            if v <= 0.0 { continue; }
            handled[oi].push(h);
            // A house controls a city when it handles >=40% of the city's trade
            // throughput (lowered from 50% so dominance actually emerges now that
            // importer-side houses also carry trade).
            if v / hub_total[h] >= 0.4 { controlled[oi].push(h); }
        }
    }

    // Per-hub second-highest influence → the "contested" flag for the Active-in list.
    let mut hub_top2 = vec![(0.0f32, 0.0f32); nhubs]; // (top, second)
    for h in &sim.houses {
        if h.defunct { continue; }
        for &(hb, x) in &h.influence {
            let c = hb as usize; if c >= nhubs { continue; }
            if x > hub_top2[c].0 { hub_top2[c].1 = hub_top2[c].0; hub_top2[c].0 = x; }
            else if x > hub_top2[c].1 { hub_top2[c].1 = x; }
        }
    }

    let mut out: Vec<HouseBrief> = sim.houses.iter().enumerate().map(|(hi, h)| {
        let hub = h.hub as usize;
        // Influence-ranked "Active in" list (h.influence is kept sorted desc).
        let active: Vec<HouseCity> = h.influence.iter().map(|&(hb, infl)| {
            let c = hb as usize;
            let role = if hb == h.hub { "seat" }
                else if h.bailos.contains(&hb) { "bailo" }
                else if sim.city_dominator.get(c).copied().unwrap_or(-1) == hi as i32 { "dominant" }
                else if h.offices.contains(&hb) { "office" }
                else { "trade" };
            let contested = c < nhubs && hub_top2[c].1 >= 0.30;
            let p = seat_pos(c);
            HouseCity { name: hub_name(hb), x: p[0], y: p[1], influence: infl, role: role.into(), contested }
        }).collect();
        // A house always counts its seat among the cities listed.
        let mut ctrl_hubs = std::mem::take(&mut controlled[hi]);
        // A house that DOMINATES its seat by trade volume (>=50% of resident-house
        // trade, the `dominant_seat` signal) also colours its home city — even when
        // in-flight throughput is momentarily sparse, so the map isn't all grey.
        if h.dominant_seat && hub < nhubs && !ctrl_hubs.contains(&hub) {
            ctrl_hubs.push(hub);
        }
        let trade_hubs = std::mem::take(&mut handled[hi]);
        let dominant = !ctrl_hubs.is_empty(); // controls at least one settlement
        let controls: Vec<[f32; 2]> = ctrl_hubs.iter().map(|&p| seat_pos(p)).collect();
        let partners: Vec<[f32; 2]> = trade_hubs.iter().filter(|&&p| p != hub)
            .map(|&p| seat_pos(p)).collect();
        let mut cities: Vec<String> = Vec::new();
        if hub < nhubs { cities.push(hub_name(hub as u32)); }
        for &p in &ctrl_hubs { if p != hub { cities.push(format!("{} (controlled)", hub_name(p as u32))); } }
        for &p in &trade_hubs { if p != hub && !ctrl_hubs.contains(&p) { cities.push(hub_name(p as u32)); } }
        HouseBrief {
            idx: hi as u32,
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
            controls,
            partners,
            cities,
            barred: sim.house_barred.get(hi).map(|v| v.iter().map(|&c| hub_name(c)).collect()).unwrap_or_default(),
            archetype: h.archetype,
            archetype_label: crate::sim::tick::archetype_label(h.archetype).to_string(),
            archetype_perk: crate::sim::tick::archetype_perk(h.archetype).to_string(),
            charters: h.charters.iter().map(|&g| gname(g)).collect(),
            fleet_sea: h.fleet_sea,
            fleet_river: h.fleet_river,
            fleet_caravan: h.fleet_caravan,
            is_guild: h.is_guild,
            offices: h.offices.iter()
                .map(|&oh| (hub_name(oh), seat_pos(oh as usize)))
                .collect(),
            estates: sim.hubs.iter()
                .filter(|e| e.is_estate && e.owner_house == hi as i32)
                .map(|e| {
                    let g = e.base_per_capita.iter().enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i).unwrap_or(0);
                    let city = if e.parent >= 0 {
                        sim.hubs.get(e.parent as usize).map(|p| p.name.clone()).unwrap_or_default()
                    } else { String::new() };
                    (gname(g), city)
                })
                .collect(),
            active,
        }
    }).collect();
    // Active first, then richest first.
    out.sort_by(|a, b| (a.defunct, -a.wealth).partial_cmp(&(b.defunct, -b.wealth))
        .unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// One ACTIVE merchant route for the campaign merchant overlay: a holder's live
/// shipments between two cities, aggregated, with the goods carried each way.
#[derive(Serialize)]
pub struct MerchantRoute {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub a_name: String,
    pub b_name: String,
    pub holder: String,
    pub color: String,
    pub is_guild: bool,
    pub sea: bool,
    pub volume: f32,
    /// Goods flowing a→b and b→a (name, volume), each sorted by volume.
    pub out_goods: Vec<(String, f32)>,
    pub ret_goods: Vec<(String, f32)>,
}

/// Aggregate the live in-transit cargo into per-holder, per-city-pair routes for
/// the merchant map layer — so the player can see which families/guilds are
/// running which corridors and what they carry each way (round-trip info).
#[tauri::command]
pub fn campaign_merchant_routes(db: State<'_, WorldDb>) -> Result<Vec<MerchantRoute>, String> {
    use std::collections::HashMap;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    struct Agg { vol: f32, sea: bool, out: HashMap<usize, f32>, ret: HashMap<usize, f32> }
    let mut groups: HashMap<(usize, u32, u32), Agg> = HashMap::new();
    for s in &sim.in_transit {
        if s.owner < 0 { continue; }
        let (lo, hi) = (s.from.min(s.to), s.from.max(s.to));
        let e = groups.entry((s.owner as usize, lo, hi))
            .or_insert_with(|| Agg { vol: 0.0, sea: false, out: HashMap::new(), ret: HashMap::new() });
        let amt = s.amount.max(0.0);
        e.vol += amt;
        e.sea |= s.sea;
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
            sea: a.sea, volume: a.vol,
            out_goods: sort_goods(a.out), ret_goods: sort_goods(a.ret),
        }
    }).collect();
    out.sort_by(|x, y| y.volume.partial_cmp(&x.volume).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(150);
    Ok(out)
}

/// One active FUTURES CONTRACT as a directional supply lane for the Futures map
/// layer: the seller's source city → the buyer city, with the good, monthly volume,
/// term and end year. Distinct from `MerchantRoute` (live spot voyages) — these are
/// standing contractual obligations.
#[derive(Serialize)]
pub struct FuturesLane {
    pub a: [f32; 2],      // source (producer/warehouse) city
    pub b: [f32; 2],      // buyer (receiver) city
    pub a_name: String,
    pub b_name: String,
    pub holder: String,   // seller house/guild
    pub color: String,
    pub is_guild: bool,
    pub good: String,
    pub qty: f32,         // monthly delivered quantity
    pub term: u8,         // 1 / 3 / 5 / 7 years
    pub end_year: u32,    // campaign year the contract expires
    pub suspended: bool,  // force-majeure (plague lockup) right now
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
        }
    }).collect();
    out.sort_by(|x, y| y.qty.partial_cmp(&x.qty).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(200);
    Ok(out)
}

/// One house/guild warehouse for the Warehouses infographic: where it sits, its
/// tier/capacity/fill, the goods it holds, and how many futures contracts it supplies.
#[derive(Serialize)]
pub struct WarehouseInfo {
    /// "warehouse" for a depot, else the estate kind (farm/mine/plantation/fishery/
    /// vineyard/manufactory).
    pub kind: String,
    pub owner: String,
    pub color: String,
    pub is_guild: bool,
    pub city: String,
    pub x: f32,
    pub y: f32,
    pub tier: u8,
    pub capacity: f32,
    pub used: f32,
    /// (good name, amount): a warehouse's stock, or an estate's production — largest first.
    pub goods: Vec<(String, f32)>,
    /// Active futures contracts this depot is the SOURCE of.
    pub contracts: u32,
    pub damage: f32,
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

/// One city in the live "richest cities" ranking.
#[derive(Serialize)]
pub struct CityRank {
    pub id: u32,
    pub name: String,
    pub population: u32,
    pub wealth: f32,      // grain + trade wealth
    pub trade: f32,       // throughput (value bought + sold)
    pub pct_world: f32,   // share of all world trade (%)
}

/// Live ranking of the wealthiest / busiest trading cities, with each city's share
/// of all world trade — top to bottom.
#[tauri::command]
pub fn campaign_city_ranking(db: State<'_, WorldDb>) -> Result<Vec<CityRank>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let trade_of = |h: &TickHub| (h.export_earn + h.import_spend).max(0.0);
    let total: f32 = sim.hubs.iter().filter(|h| !h.is_estate).map(trade_of).sum::<f32>().max(1e-6);
    let mut out: Vec<CityRank> = sim.hubs.iter().filter(|h| !h.is_estate).map(|h| {
        let trade = trade_of(h);
        CityRank {
            id: h.id, name: h.name.clone(), population: h.population.max(0.0) as u32,
            wealth: h.grain_wealth + h.trade_wealth, trade, pct_world: trade / total * 100.0,
        }
    }).collect();
    out.sort_by(|a, b| b.trade.partial_cmp(&a.trade).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(40);
    Ok(out)
}

/// Trade diagnostics — a snapshot to answer "is trade actually moving?".
#[derive(Serialize)]
pub struct CampaignDiagnostics {
    pub tick: u32,
    pub year: u32,
    pub in_transit: u32,          // shipments currently in flight
    pub shipments_last: u32,      // shipments dispatched over the last advance
    pub by_house: u32,            // ...financed by a merchant house
    pub by_guild: u32,            // ...carried by local merchants/guilds
    pub lost_last: u32,           // voyages lost (storm/ambush) last advance
    pub volume_last: f32,         // goods volume shipped last advance
    pub houses_active: u32,
    pub houses_defunct: u32,
    pub fleet_sea: u32,
    pub fleet_river: u32,
    pub fleet_caravan: u32,
    pub controlled_settlements: u32, // cities a house controls (>=50% throughput)
    pub total_house_wealth: f32,
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

/// All merchant families (active first, richest first) for the Houses panel.
#[tauri::command]
pub fn campaign_get_houses(db: State<'_, WorldDb>) -> Result<Vec<HouseBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(match get_sim(&db, &conn)? {
        Some(sim) => build_house_briefs(&sim),
        None => vec![],
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

/// DLC 3 · the POLEIS as actors — each seat city's treasury, council-set tariff /
/// mint policy, and the house that governs it. For the Polis panel.
#[derive(Serialize, Clone)]
pub struct PolisBrief {
    pub hub: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub treasury: f32,
    /// Effective export / import tariff fractions in force.
    pub tariff_export: f32,
    pub tariff_import: f32,
    /// Mint fineness (1.0 = full coin, < 1 = debased "cheap money").
    pub mint_fineness: f32,
    /// Governing house ("—" if none) + its archetype label.
    pub council: String,
    pub council_archetype: String,
    pub council_color: String,
    /// DLC 3.5 · the polis's named coin ("" = none) + its acceptance/trust 0..1.
    pub coin_name: String,
    pub coin_trust: f32,
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
            PolisBrief {
                hub: h.id, name: h.name.clone(), x: h.x, y: h.y,
                population: h.population as u32, treasury: h.treasury,
                tariff_export: h.tariff_export, tariff_import: h.tariff_import,
                mint_fineness: if h.mint_fineness <= 0.0 { 1.0 } else { h.mint_fineness },
                council, council_archetype: arch, council_color: color,
                coin_name: h.coin_name.clone(), coin_trust: h.coin_trust,
            }
        })
        .collect();
    out.sort_by(|a, b| b.treasury.partial_cmp(&a.treasury).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

/// DLC 3.5 · one currency in the world reserve-currency ranking. Surfaced in the
/// Coin & Credit panel's "Currencies" tab.
#[derive(Serialize, Clone)]
pub struct CurrencyBrief {
    pub hub: u32,
    pub city: String,
    pub coin_name: String,
    /// Acceptance / trust 0..1.
    pub trust: f32,
    /// Mint fineness (1 = full coin, < 1 = debased).
    pub fineness: f32,
    /// Recent trade throughput at the issuing city (the reserve weight).
    pub throughput: f32,
    /// True once trust clears the reserve-currency floor (accepted abroad).
    pub is_reserve: bool,
    pub color: String,
}

/// DLC 3.5 · the world's coinage, ranked by reserve strength (trust × throughput).
#[tauri::command]
pub fn campaign_get_currencies(db: State<'_, WorldDb>) -> Result<Vec<CurrencyBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<CurrencyBrief> = sim.hubs.iter().enumerate()
        .filter(|(_, h)| !h.is_estate && !h.coin_name.is_empty())
        .map(|(i, h)| {
            let throughput = h.tw_house + h.tw_local + h.tw_guild;
            CurrencyBrief {
                hub: h.id, city: h.name.clone(), coin_name: h.coin_name.clone(),
                trust: h.coin_trust,
                fineness: if h.mint_fineness <= 0.0 { 1.0 } else { h.mint_fineness },
                throughput,
                is_reserve: h.coin_trust >= 0.55,
                color: distinct_color(i),
            }
        })
        .collect();
    out.sort_by(|a, b| (b.trust * (b.throughput + 1.0))
        .partial_cmp(&(a.trust * (a.throughput + 1.0)))
        .unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

/// DLC 3.5 · one bank's balance sheet + reach, for the Coin & Credit panel.
#[derive(Serialize, Clone)]
pub struct BankBrief {
    pub name: String,
    pub seat: String,
    pub owner: String,
    pub color: String,
    pub founded_year: u32,
    pub defunct: bool,
    // ── Balance sheet (grain-eq) ──
    pub reserves: f32,
    pub loans_out: f32,
    pub real_estate: f32,
    pub deposits: f32,
    pub notes_issued: f32,
    pub equity: f32,
    pub reserve_ratio: f32,
    pub n_loans: u32,
    pub interest_earned: f32,
    pub losses: f32,
    /// Cities hosting a counting-house branch.
    pub branches: Vec<String>,
    /// Recent chronicle lines (founding, branches, defaults, failure).
    pub events: Vec<String>,
}

/// DLC 3.5 · all chartered banks, richest (by equity) first.
#[tauri::command]
pub fn campaign_get_banks(db: State<'_, WorldDb>) -> Result<Vec<BankBrief>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<BankBrief> = sim.banks.iter().map(|b| {
        let seat = sim.hubs.get(b.seat as usize).map(|h| h.name.clone()).unwrap_or_default();
        let owner = sim.houses.get(b.house as usize).map(|h| h.name.clone()).unwrap_or_default();
        let branches = b.branches.iter()
            .filter_map(|&hb| sim.hubs.get(hb as usize).map(|h| h.name.clone()))
            .collect();
        let events = b.events.iter().rev().take(8)
            .map(|e| e.text.clone()).collect();
        BankBrief {
            name: b.name.clone(), seat, owner,
            color: distinct_color(b.house as usize),
            founded_year: b.founded_tick / TICKS_PER_YEAR,
            defunct: b.defunct,
            reserves: b.reserves, loans_out: b.loans_outstanding(), real_estate: b.real_estate,
            deposits: b.deposits, notes_issued: b.notes_issued,
            equity: b.equity(), reserve_ratio: b.reserve_ratio(),
            n_loans: b.loans.iter().filter(|l| l.outstanding > 0.01).count() as u32,
            interest_earned: b.interest_earned, losses: b.losses,
            branches, events,
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

/// DLC 3.5 · one city's "schematic" — its standing buildings, estates, bank
/// presence and coin, for the Schematics (blueprint) view.
#[derive(Serialize, Clone)]
pub struct SchematicBuilding { pub label: String, pub effect: String }
#[derive(Serialize, Clone)]
pub struct SchematicEstate { pub label: String, pub tier: u8, pub owner: String, pub good: String }
#[derive(Serialize, Clone)]
pub struct CitySchematic {
    pub hub: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub coin_name: String,
    pub coin_trust: f32,
    pub council: String,
    pub buildings: Vec<SchematicBuilding>,
    pub estates: Vec<SchematicEstate>,
    /// Banks seated in this city.
    pub banks_seated: Vec<String>,
    /// Banks with a counting-house branch here.
    pub bank_branches: Vec<String>,
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
            council, buildings, estates, banks_seated, bank_branches,
        });
    }
    out.sort_by(|a, b| b.population.cmp(&a.population));
    Ok(out)
}

// ── Trade Flows subtab (per-settlement realized-trade breakdown) ──────────────
/// One traded good at a settlement: its average + last-year volume, how many
/// partner routes carried it, and its yearly volume series (the trend graph).
#[derive(Serialize, Clone)]
pub struct TradeFlowGood {
    pub good: u32,
    pub name: String,
    pub avg_volume: f32,
    pub last_volume: f32,
    pub in_volume: f32,
    pub out_volume: f32,
    pub route_count: u32,
    pub history: Vec<f32>,
}
/// One good's flow along one partner route (for the per-good route list + map).
#[derive(Serialize, Clone)]
pub struct TradeRouteFlow {
    pub good: u32,
    pub partner: u32,
    pub partner_name: String,
    pub px: f32,
    pub py: f32,
    pub dir: u8,        // 0 = inbound to this city, 1 = outbound
    pub amount: f32,
    pub pct: f32,       // share of this good's flow at this city
}
/// A top partner city: its share of ALL this city's trade + the goods exchanged.
#[derive(Serialize, Clone)]
pub struct TradePartner {
    pub hub: u32,
    pub name: String,
    pub px: f32,
    pub py: f32,
    pub volume: f32,
    pub pct: f32,
    pub goods: Vec<String>,
}
/// The settlement Flows subtab payload (last completed year + history).
#[derive(Serialize, Clone, Default)]
pub struct TradeFlows {
    pub hub: u32,
    pub hub_x: f32,
    pub hub_y: f32,
    pub goods: Vec<TradeFlowGood>,
    pub routes: Vec<TradeRouteFlow>,
    pub partners: Vec<TradePartner>,
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
    let pos = |idx: u32| sim.hubs.get(idx as usize).map(|h| (h.name.clone(), h.x, h.y));

    // ── Per-good last-year in/out + per-(good,partner,dir) route amounts ──
    let mut g_in: HashMap<u32, f32> = HashMap::new();
    let mut g_out: HashMap<u32, f32> = HashMap::new();
    let mut g_partners: HashMap<u32, std::collections::HashSet<u32>> = HashMap::new();
    let mut route_amt: HashMap<(u32, u32, u8), f32> = HashMap::new(); // (good,partner,dir)→amt
    let mut partner_vol: HashMap<u32, f32> = HashMap::new();
    let mut partner_goods: HashMap<u32, HashMap<u32, f32>> = HashMap::new(); // partner→good→amt
    for f in sim.trade_last.iter().filter(|f| f.hub == hidx) {
        if f.dir == 0 { *g_in.entry(f.good).or_insert(0.0) += f.amount; }
        else { *g_out.entry(f.good).or_insert(0.0) += f.amount; }
        g_partners.entry(f.good).or_default().insert(f.partner);
        *route_amt.entry((f.good, f.partner, f.dir)).or_insert(0.0) += f.amount;
        *partner_vol.entry(f.partner).or_insert(0.0) += f.amount;
        *partner_goods.entry(f.partner).or_default().entry(f.good).or_insert(0.0) += f.amount;
    }

    // ── Goods list: union of last-year flows + historical series ──
    let mut hist_by_good: HashMap<u32, &Vec<f32>> = HashMap::new();
    for h in sim.trade_hist.iter().filter(|h| h.hub == id) { hist_by_good.insert(h.good, &h.vols); }
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
        })
    }).collect();
    routes.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));

    // ── Top partner cities (share of all this city's trade) ──
    let total_vol: f32 = partner_vol.values().sum::<f32>().max(1e-6);
    let mut partners: Vec<TradePartner> = partner_vol.iter().filter_map(|(&p, &vol)| {
        let (pname, px, py) = pos(p)?;
        let mut gs: Vec<(u32, f32)> = partner_goods.get(&p).map(|m| m.iter().map(|(&g, &a)| (g, a)).collect()).unwrap_or_default();
        gs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let goods = gs.iter().take(4).filter_map(|(g, _)| sim.goods.get(*g as usize).map(|x| x.name.clone())).collect();
        Some(TradePartner { hub: p, name: pname, px, py, volume: vol, pct: vol / total_vol * 100.0, goods })
    }).collect();
    partners.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
    partners.truncate(12);

    Ok(Some(TradeFlows { hub: id, hub_x, hub_y, goods, routes, partners }))
}

/// One labelled money line in the Accountant view (a city's tax/profit, or a
/// warehouse good). Per-city lists are sorted largest → lowest.
#[derive(Serialize)]
pub struct LedgerLine {
    pub label: String,
    pub amount: f32,
}

/// A house/guild's yearly books for the Accountant tab (the last COMPLETED year).
#[derive(Serialize)]
pub struct HouseLedger {
    pub name: String,
    pub is_guild: bool,
    pub year: u32,
    // ── Income ──
    pub trade_profit: Vec<LedgerLine>, // per city, largest first
    pub office_income: f32,
    pub estate_income: f32,
    pub income_total: f32,
    // ── Expenditure ──
    pub import_tax: Vec<LedgerLine>,
    pub export_tax: Vec<LedgerLine>,
    pub estate_tax: f32,
    pub upkeep: f32,
    pub fleet_cost: f32,
    pub lost_cargo: f32,
    pub events: f32,
    pub consumption: f32,
    pub inflation: f32,
    pub expense_total: f32,
    pub net: f32,
    /// Monthly wealth samples through the year (for the Accountant's wealth graph).
    pub wealth_graph: Vec<f32>,
    // Warehouse stock held at the home city (what a fire/spoilage destroys).
    pub warehouse_city: String,
    pub warehouse: Vec<LedgerLine>,
}

/// The yearly T-account ledger for one house/guild (Accountant view).
#[tauri::command]
pub fn campaign_house_ledger(db: State<'_, WorldDb>, house: usize) -> Result<Option<HouseLedger>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None); };
    if house >= sim.houses.len() {
        return Ok(None);
    }
    let h = &sim.houses[house];
    // Prefer the last COMPLETED year; fall back to the running year early on.
    let prev_ok = sim
        .house_ledger_prev
        .get(house)
        .map(|l| l.year > 0 || !l.trade_profit_by_city.is_empty());
    let led = if prev_ok == Some(true) {
        &sim.house_ledger_prev[house]
    } else if let Some(l) = sim.house_ledger.get(house) {
        l
    } else {
        return Ok(None);
    };
    let city_name = |idx: u32| sim.hubs.get(idx as usize).map(|hb| hb.name.clone()).unwrap_or_default();
    let to_lines = |v: &Vec<(u32, f32)>| -> Vec<LedgerLine> {
        let mut out: Vec<LedgerLine> = v
            .iter()
            .map(|&(c, a)| LedgerLine { label: city_name(c), amount: a })
            .collect();
        out.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        out
    };
    let trade_profit = to_lines(&led.trade_profit_by_city);
    let import_tax = to_lines(&led.import_tax_by_city);
    let export_tax = to_lines(&led.export_tax_by_city);
    let income_total = trade_profit.iter().map(|l| l.amount).sum::<f32>() + led.office_income + led.estate_income;
    let tax_total = import_tax.iter().map(|l| l.amount).sum::<f32>() + export_tax.iter().map(|l| l.amount).sum::<f32>();
    let expense_total = tax_total
        + led.estate_tax
        + led.upkeep
        + led.fleet_cost
        + led.lost_cargo
        + led.events
        + led.consumption
        + led.inflation;
    // Warehouse = the home city's stored goods (what a warehouse fire destroys).
    let (warehouse, warehouse_city) = match sim.hubs.get(h.hub as usize) {
        Some(hb) => {
            let mut w: Vec<LedgerLine> = hb
                .stock
                .iter()
                .enumerate()
                .filter(|(_, &s)| s > 0.5)
                .map(|(g, &s)| LedgerLine {
                    label: sim.goods.get(g).map(|gg| gg.name.clone()).unwrap_or_default(),
                    amount: s,
                })
                .collect();
            w.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
            w.truncate(10);
            (w, hb.name.clone())
        }
        None => (vec![], String::new()),
    };
    Ok(Some(HouseLedger {
        name: h.name.clone(),
        is_guild: h.is_guild,
        year: led.year,
        trade_profit,
        office_income: led.office_income,
        estate_income: led.estate_income,
        income_total,
        import_tax,
        export_tax,
        estate_tax: led.estate_tax,
        upkeep: led.upkeep,
        fleet_cost: led.fleet_cost,
        lost_cargo: led.lost_cargo,
        events: led.events,
        consumption: led.consumption,
        inflation: led.inflation,
        expense_total,
        net: income_total - expense_total,
        wealth_graph: led.wealth_samples.clone(),
        warehouse_city,
        warehouse,
    }))
}

#[derive(Serialize)]
pub struct HouseTimelineEvent {
    pub year: u32,        // campaign year (tick / 365)
    pub kind: String,     // founded | succession | monopoly | control_gained | control_lost | branch | loss | dissolved
    pub text: String,
}

/// A house's full chronicle for the timeline view.
#[derive(Serialize)]
pub struct HouseHistory {
    pub name: String,
    pub color: String,
    pub founder: String,       // founding head + circumstances
    pub founded_year: u32,
    pub events: Vec<HouseTimelineEvent>,
    pub top_goods: Vec<(String, f32)>, // most profitable resources (name + cumulative profit)
    pub defunct: bool,
}

/// The timeline / chronicle of one house, looked up by name.
#[tauri::command]
pub fn campaign_get_house_history(name: String, db: State<'_, WorldDb>) -> Result<Option<HouseHistory>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let Some(sim) = get_sim(&db, &conn)? else { return Ok(None) };
    let idx = sim.houses.iter().position(|h| h.name == name);
    let Some(idx) = idx else { return Ok(None) };
    let h = &sim.houses[idx];
    let year = |t: u32| t / 365;
    let events: Vec<HouseTimelineEvent> = h.events.iter()
        .filter(|e| e.kind != "voyage_loss") // hide shipwreck/ambush noise from the family chronicle
        .map(|e| HouseTimelineEvent {
            year: year(e.tick), kind: e.kind.clone(), text: e.text.clone(),
        }).collect();
    let mut top_goods: Vec<(String, f32)> = h.good_profit.iter().enumerate()
        .filter(|(_, &p)| p > 0.0)
        .map(|(g, &p)| (sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default(), p))
        .collect();
    top_goods.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    top_goods.truncate(5);
    let founder = h.events.iter().find(|e| e.kind == "founded")
        .map(|e| e.text.clone()).unwrap_or_default();
    Ok(Some(HouseHistory {
        name: h.name.clone(),
        color: distinct_color(idx),
        founder,
        founded_year: year(h.founded_tick),
        events,
        top_goods,
        defunct: h.defunct,
    }))
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
            lack_series: vec![], merchant_series: vec![],
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
    })
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
