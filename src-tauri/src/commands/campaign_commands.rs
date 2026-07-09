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

/// Extract a human-readable message from a `catch_unwind` panic payload (the
/// payload is `&str` for `panic!("…")` and `String` for `format!`-style panics;
/// anything else is opaque). The full file:line is captured separately by the
/// startup panic hook in `lib.rs`.
fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic (see the panic log in the app data dir)".to_string()
    }
}

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
    /// Colony state for map markers: 0 none · 1 settlement colony · 2 house outpost.
    pub colony_kind: u8,
    pub colony_stage: u8,
    /// Owner house index (house outposts) — frontend maps it to the house colour.
    pub owner_house: i32,
    /// Founder/owner-home hub INDEX (lane endpoint); −1 if none.
    pub founder_hub: i32,
    pub autonomous: bool,
    /// Atlas 2.0 · the settlement is a DEAD ruin († marker, skipped by the sim).
    pub abandoned: bool,
    /// Tick founded mid-campaign (0 = primordial) — drives the "new town" badge.
    pub founded_tick: u32,
    /// Last full year's trade throughput (grain-eq, in+out) — Trade Heat overlay.
    pub trade_volume: f32,
    /// Dynamically-earned commercial class (re-ranked twice a year): 0 ordinary ·
    /// 1 trade hub · 2 entrepôt. Drives the distinct map marker (blue diamond / red
    /// triangle). 0 until a campaign has run.
    #[serde(default)] pub hub_class: u8,
    /// Satellite CONSTRUCTION stage: 0 = finished/not a build site · 1..=5 = building.
    /// Drives the map's under-construction marker + opens the construction window.
    #[serde(default)] pub build_stage: u8,
    /// Why the settlement died ("famine"/"plague"/"war"/"disaster"; "" = alive).
    pub died_cause: String,
    /// Downsampled population history (≤30 points, oldest first) — the census
    /// sparkline. Empty until the hub has history samples.
    pub pop_spark: Vec<f32>,
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
    /// Atlas 2.0 · recent refugee roads `[from_x, from_y, to_x, to_y, tick]` —
    /// the map draws them as fading migration arrows for ~4 years.
    pub migrations: Vec<[f32; 5]>,
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
    /// Atlas 2.0 — yearly world samples `[year, population, trade volume, live
    /// hubs, cumulative foundings, cumulative abandonments]` for the Atlas graphs.
    #[serde(default)] pub world_series: Vec<[f32; 6]>,
    /// Batch 1 — the Hall of Records (all-time world records).
    #[serde(default)] pub records: crate::sim::tick::WorldRecords,
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
    /// DLC 4 · this hub's production quality 0..1 for the good + its grade label.
    #[serde(default)] pub quality: f32,
    #[serde(default)] pub grade: String,
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
    // ── Government layer (key figures, capture, laws, stores) ──
    /// Regime type label ("Merchant Council" / "Principality" / "Free Commune").
    pub govt_type: String,
    /// Years until the next seat turns over (regime change).
    pub next_election_years: i32,
    /// The house that CONTROLS this government (captured a majority of its figures), or "".
    pub captor: String,
    pub captor_color: String,
    /// The city's key figures (mayor/treasurer/harbormaster/magistrate).
    pub officials: Vec<OfficialRow>,
    /// Each family's influence over the government, as a normalised %.
    pub family_influence: Vec<InfluenceRow>,
    /// Recent enacted laws (newest first).
    pub laws: Vec<LawRow>,
    /// The goods the government itself holds (civic granary/stockpile).
    pub civic_goods: Vec<CivicGoodRow>,
}

/// One government key figure for the Government subtab.
#[derive(Serialize, Clone)]
pub struct OfficialRow {
    pub role: String,
    pub name: String,
    /// The house it serves ("" = neutral) + colour + a status word.
    pub allegiance: String,
    pub allegiance_color: String,
    pub control: f32,
    /// "neutral" | "leaning" | "controlled" | "kin".
    pub status: String,
}

/// One family's share of government influence.
#[derive(Serialize, Clone)]
pub struct InfluenceRow { pub name: String, pub color: String, pub pct: f32 }

/// One enacted-law log row (rendered text).
#[derive(Serialize, Clone)]
pub struct LawRow { pub year: u32, pub text: String }

/// One good the government holds in its stores.
#[derive(Serialize, Clone)]
pub struct CivicGoodRow { pub name: String, pub amount: f32 }

/// One coin in a city's currency basket (for the settlement-view pie).
#[derive(Serialize)]
pub struct CoinShare {
    pub coin_name: String,
    pub share: f32,    // 0..1 of the city's circulation
    pub main: bool,    // the city's main settling coin
    pub reserve: bool, // a foreign reserve coin circulating here
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
    /// Abstract social strata of this settlement (shares + inequality + welfare).
    #[serde(default)] pub society: Option<SocietyBrief>,
    /// For an estate: its kind (0 none / 1 farm / 2 mine / 3 plantation / 4 fishery /
    /// 5 vineyard), who owns it, and the good it works — for the inspector.
    #[serde(default)] pub estate_kind: u8,
    #[serde(default)] pub estate_owner: String,
    #[serde(default)] pub estate_good: String,
    /// Buildings erected here: (name, one-line effect) — for the inspector.
    #[serde(default)] pub structures: Vec<(String, String)>,
    /// Trade-base patron: the merchant house developing this city as a base of
    /// operations (empty = none). See docs/TRADE_BASE_MECHANIC_PLAN.md.
    #[serde(default)] pub patron: String,
    /// #23 · the majority PEOPLE of this settlement + its minority quarters
    /// `(people, population share)` — grown by in-migration of a different culture
    /// and slowly eroded by assimilation. Display-only.
    #[serde(default)] pub culture: String,
    #[serde(default)] pub minorities: Vec<(String, f32)>,
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
    // ── DLC 3.5 · treasury, finances, war, and the carrying trade ──
    /// Retained civic treasury (grain-eq).
    #[serde(default)] pub treasury: f32,
    /// The city's treasury books (current running year + last completed in `prev`).
    #[serde(default)] pub finance: Option<crate::sim::tick::CityFinance>,
    /// Name of the polis this city is at war with ("" = at peace).
    #[serde(default)] pub war_with: String,
    /// Its coin, if it mints one.
    #[serde(default)] pub coin_name: String,
    #[serde(default)] pub coin_trust: f32,
    #[serde(default)] pub coin_value: f32,
    /// The city's CURRENCY BASKET — which coins circulate here and their share
    /// (main coin first). For the settlement-view currency-basket pie.
    #[serde(default)] pub coin_basket: Vec<CoinShare>,
    /// Carrying trade: in-flight shipments THIS city's merchants run between OTHER
    /// cities (the entrepôt "transit" — goods that pass through its houses' hands).
    #[serde(default)] pub transit: Vec<TransitRow>,
    /// DLC 4 · espionage: if this estate/manufactory stole a quality technique, the
    /// good's name + the city it was stolen from ("" = none).
    #[serde(default)] pub stolen_good: String,
    #[serde(default)] pub stolen_from: String,
    /// Colonies & outposts this city FOUNDED (its metropolis roster).
    #[serde(default)] pub related_colonies: Vec<ColonySummary>,
}

/// Abstract social strata of a settlement for the HubPanel "Society" block:
/// the four population shares (Σ=1), an inequality index, and a derived welfare.
#[derive(Serialize, Clone)]
pub struct SocietyBrief {
    pub patrician: f32,
    pub burgher: f32,
    pub commoner: f32,
    pub underclass: f32,
    pub commoner_wealth: f32,
    pub inequality: f32,
    /// 0 = destitute, 1 = comfortable — commoner welfare for the meter (derived).
    pub welfare: f32,
    /// 0 = content … 1 = boiling — civil unrest (It. 3).
    #[serde(default)] pub unrest: f32,
}

/// DLC 3.5 · one leg of a city's carrying trade (a merchant of this city hauling
/// goods between two OTHER cities), for the settlement "Transit" section.
#[derive(Serialize, Clone)]
pub struct TransitRow {
    pub merchant: String,
    pub is_guild: bool,
    pub color: String,
    pub good: String,
    pub amount: f32,
    pub value: f32,
    pub from_name: String,
    pub to_name: String,
    pub sea: bool,
    /// Coin the deal settles in ("" → barter).
    pub coin: String,
    /// Barter ratio when no reserve coin applies (e.g. "~3.2 wheat/unit").
    pub barter: String,
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
    pub owner_is_civic: bool, // city-financed (locally owned), no private house/guild
    pub tier: u8,            // upgrade tier 1..5
    pub damage: f32,         // disaster damage 0 (intact) .. 1 (ruined); suppresses output
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
            let mut sim = decode_campaign_blob(&s)?;
            sim.rebuild_routes(); // `days` / `neighbors` are not serialized
            Some(std::sync::Arc::new(sim))
        }
        _ => None,
    };
    cache.loaded = true;
    cache.dirty = false;
    Ok(())
}

/// Batch 1 · campaign blob codec: `"Z1:" + base64(zstd(json))` — ~5-10× smaller
/// DB rows and `.campaign` files. JSON stays the wire format underneath
/// (self-describing, so the append-`serde(default)` save-compat strategy is
/// untouched); legacy plain-JSON rows load via the prefix check.
fn compress_blob(json: &str) -> Result<String, String> {
    use base64::Engine as _;
    let z = zstd::encode_all(json.as_bytes(), 3).map_err(|e| e.to_string())?;
    Ok(format!("Z1:{}", base64::engine::general_purpose::STANDARD.encode(z)))
}

/// Returns the JSON inside `raw` — decompressing a `Z1:` blob, passing a legacy
/// plain-JSON row through untouched.
fn decompress_blob(raw: &str) -> Result<std::borrow::Cow<'_, str>, String> {
    use base64::Engine as _;
    if let Some(b64) = raw.strip_prefix("Z1:") {
        let z = base64::engine::general_purpose::STANDARD.decode(b64)
            .map_err(|e| format!("campaign_sim base64: {e}"))?;
        let bytes = zstd::decode_all(&z[..]).map_err(|e| format!("campaign_sim zstd: {e}"))?;
        Ok(std::borrow::Cow::Owned(
            String::from_utf8(bytes).map_err(|e| format!("campaign_sim utf8: {e}"))?))
    } else {
        Ok(std::borrow::Cow::Borrowed(raw))
    }
}

fn encode_campaign_blob(sim: &CampaignSim) -> Result<String, String> {
    let json = serde_json::to_string(sim).map_err(|e| e.to_string())?;
    compress_blob(&json)
}

fn decode_campaign_blob(raw: &str) -> Result<CampaignSim, String> {
    let json = decompress_blob(raw)?;
    serde_json::from_str(&json).map_err(|e| format!("campaign_sim parse: {e}"))
}

#[cfg(test)]
mod blob_tests {
    use super::{compress_blob, decompress_blob};

    /// Batch 1 · the save-blob codec round-trips and shrinks; legacy plain-JSON
    /// rows pass through untouched.
    #[test]
    fn campaign_blob_codec_roundtrip_and_legacy() {
        // A repetitive JSON-ish payload (like real sim state) compresses well.
        let json = format!("{{\"hubs\":[{}]}}",
            (0..500).map(|i| format!("{{\"id\":{i},\"stock\":[0.0,1.5,2.25]}}"))
                .collect::<Vec<_>>().join(","));
        let blob = compress_blob(&json).expect("compress");
        assert!(blob.starts_with("Z1:"));
        assert!(blob.len() < json.len() / 3, "blob {} vs json {}", blob.len(), json.len());
        assert_eq!(decompress_blob(&blob).expect("decompress").as_ref(), json);
        // Legacy row (no prefix) passes through untouched.
        assert_eq!(decompress_blob(&json).expect("legacy").as_ref(), json);
    }
}

/// A HANDLE to the resident sim (loading it from the DB once if needed) — an Arc
/// pointer bump, NOT a deep copy. The old version returned `CampaignSim.clone()`,
/// which copied every hub's per-good vectors, the per-hub histories and the
/// 20k-entry journal on EVERY read-only panel query — megabytes per HUD refresh.
/// The campaign lock is released before the caller computes, so heavy queries
/// never stall the Play loop.
pub(crate) fn get_sim(db: &WorldDb, conn: &Connection)
    -> Result<Option<std::sync::Arc<CampaignSim>>, String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    ensure_campaign_loaded(&mut cache, conn)?;
    Ok(cache.sim.clone())
}

/// Replace the resident sim and mark it dirty (the DB row is flushed later by
/// `persist_campaign` / the advance cadence). Used by commands that build or rewrite
/// the whole sim.
fn set_sim(db: &WorldDb, sim: &CampaignSim) -> Result<(), String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    cache.sim = Some(std::sync::Arc::new(sim.clone()));
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
            let blob = encode_campaign_blob(sim.as_ref())?;
            metadata::campaign_set(conn, "campaign_sim", &blob).map_err(|e| e.to_string())?;
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
                colony_kind: h.colony_kind,
                colony_stage: h.colony_stage,
                owner_house: h.owner_house,
                founder_hub: h.founder_hub,
                autonomous: h.autonomous,
                abandoned: h.abandoned,
                founded_tick: h.founded_tick,
                trade_volume: h.trade_last_year,
                hub_class: h.hub_class,
                build_stage: h.build_stage,
                died_cause: h.died_cause.clone(),
                pop_spark: {
                    // ≤30 evenly-spaced population samples for the census sparkline.
                    let step = (h.history.len() / 30).max(1);
                    h.history.iter().step_by(step).map(|s| s.population).collect()
                },
            }
        })
        .collect();
    // Census = live simulated hubs + the inert hinterland towns below the sim cap
    // (decouple: counted even though they aren't ticked, so Atlas isn't undercounting).
    let total_population: f32 = sim.hubs.iter().map(|h| h.population.max(0.0)).sum::<f32>()
        + sim.hinterland.iter().map(|t| t.population.max(0.0)).sum::<f32>();
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
        // Refugee roads still fresh enough to draw (fade over ~4 years).
        migrations: sim.migrations.iter()
            .filter(|m| sim.tick as f32 - m[4] < 4.0 * 365.0)
            .cloned()
            .collect(),
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
        migrations: vec![],
    }
}

/// Atlas 2.0 · one NAMED TRADE BASIN — a cluster of living market towns whose
/// strongest trade ties bind them together: "the regions where trade happens".
#[derive(Serialize, Clone)]
pub struct TradeBasin {
    /// Culture-styled region name from the basin's heart (e.g. "Vexillia").
    pub name: String,
    /// Total yearly flow volume INTERNAL to the basin (grain-eq).
    pub volume: f32,
    pub hub_ids: Vec<u32>,
    /// Member positions (world cells) — the frontend hulls + labels them.
    pub pts: Vec<[f32; 2]>,
    pub cx: f32,
    pub cy: f32,
    /// Busiest member (highest yearly throughput).
    pub top_city: String,
    /// Batch 1 · the basin's top traded goods (≤2, by yearly volume).
    pub top_goods: Vec<String>,
}

/// Cluster the yearly flow ledger into named trade basins: each town keeps only
/// its TOP-2 flow edges, and the connected components of that sparse
/// "strongest-partner" graph are the basins — weak long-range ties don't merge
/// the whole world into one blob. Volume counts ALL flow internal to a basin.
/// Basin memo: the clustering is deterministic in the sim state and can only
/// change when time advances, so repeated panel opens / overlay refreshes at the
/// same (seed, tick) are answered from this cache.
static BASINS_MEMO: std::sync::Mutex<Option<(u64, u32, Vec<TradeBasin>)>> =
    std::sync::Mutex::new(None);

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

/// Batch 1 · era scrubber: the world as it stood at the end of `year` — marker +
/// heat data reconstructed from the yearly frame ring. None if that year isn't
/// in the (bounded) ring.
#[derive(Serialize)]
pub struct EraFrame {
    pub year: u32,
    pub hubs: Vec<EraHub>,
}
#[derive(Serialize)]
pub struct EraHub {
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub population: f32,
    pub trade: f32,
    pub dead: bool,
    pub is_new: bool,
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

// ── #1/#23 · Peoples (cultures) ─────────────────────────────────────────────────
#[derive(Serialize)]
pub struct CultureBrief {
    pub name: String,
    pub color: [u8; 3],
    pub population: u32,
    pub towns: u32,      // settlements where this culture is the MAJORITY
    pub presence: u32,   // settlements where it is present (≥ 5%)
    pub mobility: f32,   // 0..1 travel-proneness (≥0.7 = merchant diaspora)
    pub top_cities: Vec<(String, u32)>, // by this culture's population, top 4
    pub houses: Vec<String>,            // merchant houses of this people
}

/// Culture colour: the worldgen hearth colour if known, else a deterministic tint.
fn culture_color(name: &str) -> [u8; 3] {
    if let Some(m) = crate::sim::cultures::active() {
        if let Some(h) = m.hearths.iter().find(|h| h.people == name) {
            return h.color;
        }
    }
    let mut x = 0xcbf29ce484222325u64;
    for b in name.bytes() { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); }
    [(80 + (x % 150)) as u8, (80 + ((x >> 8) % 150)) as u8, (80 + ((x >> 16) % 150)) as u8]
}

/// Per-culture census: population, town count, top cities, houses, mobility. Sorted
/// by population (largest people first). Powers the Peoples panel.
#[tauri::command]
pub fn campaign_get_cultures(db: State<'_, WorldDb>) -> Result<Vec<CultureBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    use std::collections::HashMap;
    struct Acc { pop: f64, towns: u32, presence: u32, cities: Vec<(String, f64)> }
    let mut map: HashMap<String, Acc> = HashMap::new();
    let add = |map: &mut HashMap<String, Acc>, c: &str, name: &str, p: f64, major: bool, present: bool| {
        let e = map.entry(c.to_string()).or_insert_with(|| Acc { pop: 0.0, towns: 0, presence: 0, cities: vec![] });
        e.pop += p;
        if major { e.towns += 1; }
        if present { e.presence += 1; }
        e.cities.push((name.to_string(), p));
    };
    for i in 0..sim.hubs.len() {
        let h = &sim.hubs[i];
        if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
        let pop = h.population.max(0.0) as f64;
        let maj = sim.hub_culture.get(i).cloned().unwrap_or_default();
        if maj.is_empty() || maj == "—" { continue; }
        let mshare: f32 = sim.hub_minorities.get(i).map(|m| m.iter().fold(0.0f32, |a, (_, s)| a + *s)).unwrap_or(0.0);
        let maj_share = (1.0 - mshare).clamp(0.0, 1.0) as f64;
        add(&mut map, &maj, &h.name, pop * maj_share, true, true);
        if let Some(mins) = sim.hub_minorities.get(i) {
            for (c, s) in mins {
                if *s <= 0.005 { continue; }
                add(&mut map, c, &h.name, pop * (*s as f64), false, *s >= 0.05);
            }
        }
    }
    let mut houses_by: HashMap<String, Vec<String>> = HashMap::new();
    for hh in &sim.houses {
        if hh.defunct { continue; }
        if let Some(c) = sim.hub_culture.get(hh.hub as usize) {
            if !c.is_empty() && c != "—" {
                houses_by.entry(c.clone()).or_default().push(hh.name.clone());
            }
        }
    }
    let mut out: Vec<CultureBrief> = map.into_iter().map(|(name, mut acc)| {
        acc.cities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<(String, u32)> = acc.cities.into_iter().take(4).map(|(n, p)| (n, p as u32)).collect();
        let mut houses = houses_by.remove(&name).unwrap_or_default();
        houses.sort(); houses.dedup(); houses.truncate(8);
        CultureBrief {
            color: culture_color(&name),
            mobility: crate::sim::tick::CampaignSim::culture_mobility(&name),
            population: acc.pop as u32, towns: acc.towns, presence: acc.presence,
            top_cities: top, houses, name,
        }
    }).collect();
    out.sort_by(|a, b| b.population.cmp(&a.population));
    Ok(out)
}

/// Per-hub share of ONE culture: `[x, y, share]` for every settlement where the
/// people is present (≥ 5%). Drives the map's culture-share overlay.
#[tauri::command]
pub fn campaign_culture_hubs(name: String, db: State<'_, WorldDb>) -> Result<Vec<[f32; 3]>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out = vec![];
    for i in 0..sim.hubs.len() {
        let h = &sim.hubs[i];
        if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
        let share = sim.culture_share_at(i, &name);
        if share > 0.05 { out.push([h.x, h.y, share]); }
    }
    Ok(out)
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
                coin_name: String::new(),
                coin_trust: 0.0,
                settle_coin: -1,
                coin_basket: Vec::new(),
                mint_fineness_prev: 0.0,
                price_level: 1.0,
                coin_circ_prev: 0.0,
                last_reform_tick: 0,
                reform_until: 0,
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
        fleets_migrated: true, // new campaigns already seed fleets
        tech_factor: 1.0,
        percap_migrated: true, // hubs seeded with base_per_capita directly
        society_migrated: false, // strata seeded on first advance (seed_society)
        house_ledger: Vec::new(),
        house_ledger_prev: Vec::new(),
        house_barred: Vec::new(),
        colonizable: econ.colonizable_sites.clone(),
        satellite_sites: vec![], // filled from tiles just after construction (below)
        hinterland: hinterland_towns,
        hub_patron: vec![],
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
    sim.rebuild_routes();
    sim.ensure_hub_cultures(); // seed each hub's majority people from the culture map
    sim.seed_initial_guilds(); // civic guilds for cities already ≥ 50k people
    set_sim(&db, &sim)?;
    persist_campaign(&db, &conn)?; // write the fresh campaign to the DB immediately
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

/// Autosave cadence: the campaign is flushed to disk every this many SIM-years
/// (crossing an even-year boundary), with a wall-clock safety flush so an unattended
/// fast-play still checkpoints between boundaries.
const AUTOSAVE_EVERY_YEARS: u32 = 2;
const AUTOSAVE_WALLCLOCK_SECS: f32 = 120.0;

/// Below this many remaining sites, `campaign_advance` recomputes the colonization
/// pool from live tiles. The tick sim has no WorldBuffer, so it can never refill the
/// pool itself; gating on a low floor keeps the (cheap) recompute rare.
const COLONIZE_POOL_FLOOR: usize = 8;

/// Recompute the empty-land colonization pool from current tile data, excluding land
/// near any EXISTING hub (cities + already-founded colonies/outposts, so consumed
/// sites are never re-added). Mirrors the Economy step's `compute_colonizable_sites`
/// inputs. This is what keeps colonization alive across a long campaign and unsticks
/// old saves whose one-time pool was empty or drained.
fn recompute_colonizable(
    db: &WorldDb,
    conn: &Connection,
    hub_xy: &[(f32, f32)],
) -> Result<Vec<crate::sim::tick::ColonizeSite>, String> {
    let grid_w: u32 = metadata::get_meta(conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let specs = crate::commands::goods_commands::load_world_goods(conn);
    let base_value: Vec<f32> = specs.iter().map(|s| s.base_value.max(0.0)).collect();
    let world = db.cached_tiles_with_conn(conn)?;
    Ok(crate::commands::query_commands::compute_colonizable_sites(
        &world, grid_w, grid_h, hub_xy, &base_value))
}

/// Recompute the NEAR-city satellite site pool from current tiles (≤500 km from a
/// city). Mirrors `recompute_colonizable` but calls the near-city variant. Used to
/// (re)fill `CampaignSim::satellite_sites` at build and when it drains.
fn recompute_satellite_sites(
    db: &WorldDb,
    conn: &Connection,
    hub_xy: &[(f32, f32)],
) -> Result<Vec<crate::sim::tick::ColonizeSite>, String> {
    let grid_w: u32 = metadata::get_meta(conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let specs = crate::commands::goods_commands::load_world_goods(conn);
    let base_value: Vec<f32> = specs.iter().map(|s| s.base_value.max(0.0)).collect();
    let world = db.cached_tiles_with_conn(conn)?;
    Ok(crate::commands::query_commands::compute_satellite_sites(
        &world, grid_w, grid_h, hub_xy, &base_value))
}

/// Advance the sim resiliently — the campaign must ALWAYS move forward and NEVER crash
/// out. The batch runs in year-sized chunks under `catch_unwind`. On a tick fault:
///   1. restore the pre-chunk checkpoint (a cheap in-memory clone — NaN-safe, unlike a
///      JSON round-trip) and RETRY the chunk with territorial expansion frozen (the
///      year-30 founding paths are the usual suspects);
///   2. if it STILL faults, step the chunk tick-by-tick, skipping ONLY the single
///      poisoned tick (the clock still advances) and simulating every good tick.
/// State is preserved throughout (never discarded), so any batch — even one that hits a
/// deterministic bug — completes and the run stays continuable.
fn advance_resilient(sim: &mut crate::sim::tick::CampaignSim, ticks: u32) {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use crate::sim::tick::TICKS_PER_YEAR;
    if ticks == 0 { return; }
    let chunk_len = TICKS_PER_YEAR.max(1);
    let mut checkpoint = sim.clone();
    let mut done = 0u32;
    while done < ticks {
        let chunk = (ticks - done).min(chunk_len);
        // Fast path — the chunk simulates cleanly.
        if catch_unwind(AssertUnwindSafe(|| sim.advance(chunk))).is_ok() {
            checkpoint = sim.clone();
            done += chunk;
            continue;
        }
        // Recovery step 1 — restore, freeze expansion, retry the whole chunk once.
        *sim = checkpoint.clone();
        sim.expansion_frozen_until = sim.tick + chunk_len + chunk;
        if catch_unwind(AssertUnwindSafe(|| sim.advance(chunk))).is_ok() {
            checkpoint = sim.clone();
            done += chunk;
            continue;
        }
        // Recovery step 2 — restore and step tick-by-tick, skipping only bad ticks so
        // the run can never stall on a persistent, deterministic fault.
        *sim = checkpoint.clone();
        let mut tick_cp = checkpoint.clone();
        for _ in 0..chunk {
            match catch_unwind(AssertUnwindSafe(|| sim.advance(1))) {
                Ok(()) => tick_cp = sim.clone(),
                Err(payload) => {
                    // Log the fault so a recurring deterministic bug is diagnosable, then
                    // roll back to the pre-tick checkpoint and skip just this tick.
                    eprintln!("[campaign] tick fault skipped: {}", panic_payload_message(&payload));
                    *sim = tick_cp.clone();
                    sim.skip_poisoned_tick();
                    tick_cp = sim.clone();
                }
            }
        }
        checkpoint = sim.clone();
        done += chunk;
    }
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

/// DLC 4 · one typed population unit for the (future) Population panel.
#[derive(Serialize)]
pub struct PopBrief {
    pub profession: String,
    pub size: f32,
    pub money: f32,
    pub needs_life: f32,
    pub needs_everyday: f32,
    pub needs_luxury: f32,
    pub consciousness: f32,
    pub militancy: f32,
}

/// DLC 4 · the derived Pops of one hub (read-only foundation of the Nations & POPs
/// layer). Empty when no campaign is active or the hub index is unknown.
#[tauri::command]
pub fn campaign_get_pops(db: State<'_, WorldDb>, hub: u32) -> Result<Vec<PopBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let h = hub as usize;
    if h >= sim.hubs.len() { return Ok(vec![]); }
    Ok(sim.hubs[h].pops.iter().map(|p| PopBrief {
        profession: crate::sim::tick::POP_PROFESSIONS
            .get(p.profession as usize).copied().unwrap_or("?").to_string(),
        size: p.size,
        money: p.money,
        needs_life: p.needs_life,
        needs_everyday: p.needs_everyday,
        needs_luxury: p.needs_luxury,
        consciousness: p.consciousness,
        militancy: p.militancy,
    }).collect())
}

/// One backer of a colony venture (city / house / bank), for the Supply subtab.
#[derive(Serialize)]
pub struct ColonyBackerRow { pub kind: u8, pub name: String, pub color: String, pub share: f32 }

/// One civic supply contract row feeding a colony (the supplier roster).
#[derive(Serialize)]
pub struct ColonySupplyRow { pub category: u8, pub supplier: String, pub good: String, pub qty: f32 }

/// Full colony detail for the HubPanel "Supply" subtab.
#[derive(Serialize)]
pub struct ColonyDetail {
    pub stage: u8,
    pub autonomous: bool,
    pub founder_name: String,
    pub main_bank_name: String,
    pub coin_name: String,
    pub charter_open: bool,
    pub supply_years: f32,
    pub reserve_food: f32,
    pub reserve_cap: f32,
    pub age_years: u32,
    /// Years until the colony may seek independence (≤0 = eligible now).
    pub indep_in_years: i32,
    pub backers: Vec<ColonyBackerRow>,
    pub supply: Vec<ColonySupplyRow>,
    /// Dedicated grain-run supply ships the metropolis keeps on this colony.
    pub supply_ships: u32,
    /// Total monthly carriage of that fleet (ships × ship capacity).
    pub supply_capacity: f32,
    /// Food actually delivered last month by the fleet.
    pub supply_delivered: f32,
    /// The city currently designated as the colony's food source (empty = none found).
    pub supply_source: String,
}

/// Colony detail for a settlement colony (None for non-colony hubs).
#[tauri::command]
pub fn campaign_get_colony(id: u32, db: State<'_, WorldDb>) -> Result<Option<ColonyDetail>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = match sim.hubs.iter().position(|h| h.id == id) { Some(i) => i, None => return Ok(None) };
    let hub = &sim.hubs[hi];
    if hub.colony_kind != 1 { return Ok(None); }
    let founder_name = (hub.founder_hub >= 0).then(|| sim.hubs.get(hub.founder_hub as usize))
        .flatten().map(|h| h.name.clone()).unwrap_or_default();
    let main_bank_name = (hub.main_bank >= 0).then(|| sim.banks.get(hub.main_bank as usize))
        .flatten().map(|b| b.name.clone()).unwrap_or_default();
    let age_years = (sim.tick.saturating_sub(hub.colony_founded_tick) / 365) as u32;
    let indep_in_years = 70i32 - age_years as i32;
    let backers = hub.backers.iter().map(|&(kind, idx, share)| {
        let (name, color) = match kind {
            0 => (sim.hubs.get(idx as usize).map(|h| h.name.clone()).unwrap_or_default(), "#7fb8ff".to_string()),
            1 => (sim.houses.get(idx as usize).map(|h| h.name.clone()).unwrap_or_default(), distinct_color(idx as usize)),
            2 => {
                let b = sim.banks.get(idx as usize);
                (b.map(|b| b.name.clone()).unwrap_or_default(),
                 b.map(|b| distinct_color(b.house as usize)).unwrap_or_else(|| "#cccccc".into()))
            }
            _ => (String::new(), "#cccccc".to_string()),
        };
        ColonyBackerRow { kind, name, color, share }
    }).collect();
    let supply = sim.colony_supply.iter().filter(|s| s.colony_hub == hi as u32).map(|s| {
        ColonySupplyRow {
            category: s.category,
            supplier: sim.hubs.get(s.supplier_hub as usize).map(|h| h.name.clone()).unwrap_or_default(),
            good: sim.goods.get(s.good).map(|g| g.name.clone()).unwrap_or_default(),
            qty: s.monthly_qty,
        }
    }).collect();
    let supply_source = (hub.supply_source >= 0)
        .then(|| sim.hubs.get(hub.supply_source as usize)).flatten()
        .map(|h| h.name.clone()).unwrap_or_default();
    Ok(Some(ColonyDetail {
        stage: hub.colony_stage, autonomous: hub.autonomous, founder_name, main_bank_name,
        coin_name: hub.coin_name.clone(), charter_open: hub.colony_kind == 1 && !hub.autonomous,
        supply_years: hub.supply_years, reserve_food: hub.reserve_food, reserve_cap: hub.reserve_cap,
        age_years, indep_in_years, backers, supply,
        supply_ships: hub.supply_ships,
        supply_capacity: hub.supply_ships as f32 * crate::sim::tick::SUPPLY_SHIP_CAPACITY,
        supply_delivered: hub.supply_delivered,
        supply_source,
    }))
}

/// One construction-supply tab for the satellite window (food / preservables / construction).
#[derive(Serialize)]
pub struct SatSupplyRow {
    pub category: String,
    pub good: String,
    pub source: String,
    pub rate: f32,
    pub met: f32,
}

/// Live state of a satellite still UNDER CONSTRUCTION (build_stage>0) — everything the
/// construction window needs. `None` once finished (it becomes a normal bound city).
#[derive(Serialize)]
pub struct SatelliteBrief {
    pub id: u32,
    pub name: String,
    pub metropolis: String,
    pub metropolis_id: i32,
    pub role: String,
    pub stage: u8,
    pub progress: f32,   // within the current stage
    pub overall: f32,    // 0..1 across all 5 stages
    pub eta_years: f32,
    pub monthly_cost: f32,
    pub fund: f32,
    pub runway_months: f32,
    pub convoys: u8,
    pub idle_months: u8,
    pub founded_year: f32,
    pub supply: Vec<SatSupplyRow>,
    pub exploits: Vec<String>,
}

#[tauri::command]
pub fn campaign_get_satellite(id: u32, db: State<'_, WorldDb>) -> Result<Option<SatelliteBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = match sim.hubs.iter().position(|h| h.id == id) { Some(i) => i, None => return Ok(None) };
    let hub = &sim.hubs[hi];
    if hub.build_stage == 0 { return Ok(None); } // not (or no longer) a construction site
    let m = hub.founder_hub;
    let (metropolis, metropolis_id, fund) = if m >= 0 && (m as usize) < sim.hubs.len() {
        (sim.hubs[m as usize].name.clone(), m, sim.hubs[m as usize].treasury)
    } else { ("—".to_string(), -1, 0.0) };
    let role = match hub.colony_stage { 1 => "Granary", 0 => "Port", _ => "Workshop" }.to_string();
    let stage = hub.build_stage.min(5);
    let overall = (((stage.saturating_sub(1)) as f32) + hub.build_progress.clamp(0.0, 1.0)) / 5.0;
    let met = hub.build_supply.iter().cloned().fold(1.0f32, f32::min).clamp(0.0, 1.0);
    let months_left = ((5 - stage) as f32 + (1.0 - hub.build_progress).max(0.0))
        * crate::sim::tick::SAT_STAGE_MONTHS;
    let eta_years = months_left / met.max(0.08) / 12.0;
    let monthly_cost = crate::sim::tick::SAT_CONVOY_UPKEEP * hub.build_convoys as f32;
    let runway_months = if monthly_cost > 0.0 { fund / monthly_cost } else { 0.0 };
    let cats = ["Food", "Preservables", "Construction"];
    let supply = (0..3usize).map(|c| {
        let g = hub.build_supply_good[c] as usize;
        SatSupplyRow {
            category: cats[c].to_string(),
            good: sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_else(|| "—".into()),
            source: metropolis.clone(),
            rate: crate::sim::tick::SAT_STAGE_QUOTA,
            met: hub.build_supply[c].clamp(0.0, 1.0),
        }
    }).collect();
    // Future exploits = the goods this site will actually produce once finished.
    let mut gi: Vec<usize> = (0..sim.goods.len()).collect();
    gi.sort_by(|&a, &b| hub.base_per_capita.get(b).copied().unwrap_or(0.0)
        .partial_cmp(&hub.base_per_capita.get(a).copied().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    let exploits = gi.into_iter()
        .filter(|&g| hub.base_per_capita.get(g).copied().unwrap_or(0.0) > 0.0)
        .take(4).map(|g| sim.goods[g].name.clone()).collect();
    Ok(Some(SatelliteBrief {
        id, name: hub.name.clone(), metropolis, metropolis_id, role,
        stage, progress: hub.build_progress.clamp(0.0, 1.0), overall, eta_years,
        monthly_cost, fund, runway_months, convoys: hub.build_convoys,
        idle_months: hub.build_idle_months, founded_year: hub.build_start_tick as f32 / 365.0,
        supply, exploits,
    }))
}

/// One secured good in a council's civic warehouse (Provisioning tab).
#[derive(Serialize)]
pub struct ProvGoodRow {
    pub good: String,
    pub secured: f32,
    pub target: f32,
    pub food: bool,
}

/// Council RIGHT-OF-FIRST-BUY / provisioning state for a city (Provisioning tab).
#[derive(Serialize)]
pub struct ProvisioningBrief {
    pub first_buy: bool,
    pub dominant_house: String,
    /// Fraction of the city's trade carried by merchant houses (0..1).
    pub dominant_share: f32,
    pub dependents: u32,
    pub reserve_target: f32,
    /// Grain-eq secured into the civic warehouse last month.
    pub bought_month: f32,
    pub goods: Vec<ProvGoodRow>,
}

#[tauri::command]
pub fn campaign_get_provisioning(id: u32, db: State<'_, WorldDb>) -> Result<Option<ProvisioningBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = match sim.hubs.iter().position(|h| h.id == id) { Some(i) => i, None => return Ok(None) };
    let hub = &sim.hubs[hi];
    if hub.is_estate || hub.abandoned { return Ok(None); }
    let deps = sim.hubs.iter().filter(|d| d.founder_hub == hi as i32 && !d.abandoned
        && (d.colony_kind == 1 || d.colony_kind == 3 || d.build_stage > 0)).count() as u32;
    // Real trade-share dominance: houses carrying ≥60% of the city's trade (or a captured
    // government) suspend the council's right of first buy.
    let tw_total = hub.tw_house + hub.tw_local + hub.tw_guild;
    let dominant_share = if tw_total > 1e-6 { hub.tw_house / tw_total } else { 0.0 };
    let first_buy = dominant_share < 0.60 && hub.captor_house < 0;
    let dominant_house = if hub.captor_house >= 0 {
        sim.houses.get(hub.captor_house as usize).map(|h| h.name.clone()).unwrap_or_default()
    } else { String::new() };
    let bought_month = sim.council_bought_month.get(hi).copied().unwrap_or(0.0);
    let reserve_target = crate::sim::tick::COUNCIL_RESERVE_BASE * (1.0 + deps as f32)
        * (hub.population / 5_000.0).clamp(0.3, 4.0);
    let mut goods: Vec<ProvGoodRow> = (0..sim.goods.len())
        .filter(|&g| hub.civic_goods.get(g).copied().unwrap_or(0.0) > 0.01
            || (sim.goods[g].food && (deps > 0 || hub.food_balance <= 0.15)))
        .map(|g| ProvGoodRow {
            good: sim.goods[g].name.clone(),
            secured: hub.civic_goods.get(g).copied().unwrap_or(0.0),
            target: reserve_target,
            food: sim.goods[g].food,
        })
        .collect();
    goods.sort_by(|a, b| b.secured.partial_cmp(&a.secured).unwrap_or(std::cmp::Ordering::Equal));
    goods.truncate(10);
    Ok(Some(ProvisioningBrief { first_buy, dominant_house, dominant_share, dependents: deps, reserve_target, bought_month, goods }))
}

/// A route-bound migration flow for the reworked Migration overlay (polyline along the
/// trade network + culture + volume + age for fade).
#[derive(Serialize)]
pub struct MigrationRouteBrief {
    pub path: Vec<[f32; 2]>,
    pub culture: String,
    pub volume: f32,
    pub from_hub: i32,
    pub to_hub: i32,
    pub age_years: f32,
}

#[tauri::command]
pub fn campaign_get_migration_routes(db: State<'_, WorldDb>) -> Result<Vec<MigrationRouteBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    Ok(sim.migration_routes.iter().map(|r| MigrationRouteBrief {
        path: r.path.clone(), culture: r.culture.clone(), volume: r.volume,
        from_hub: r.from_hub, to_hub: r.to_hub,
        age_years: sim.tick.saturating_sub(r.tick) as f32 / 365.0,
    }).collect())
}

/// One roster row for the Colonial Office subwindow — a light projection over a
/// colony hub, covering BOTH settlement colonies (`colony_kind==1`) and house
/// trade outposts (`colony_kind==2`). The heavy per-colony detail (joint-stock
/// backers + supply contracts) stays in `ColonyDetail`/`campaign_get_colony`.
#[derive(Serialize)]
pub struct ColonySummary {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub colony_kind: u8,
    pub colony_stage: u8,
    pub autonomous: bool,
    pub population: f32,
    pub founder_hub: i32,
    pub founder_name: String,
    /// The metropolis's map coordinates (−1 if none) — for the click-highlight line.
    #[serde(default)] pub founder_x: f32,
    #[serde(default)] pub founder_y: f32,
    pub main_bank_name: String,
    pub coin_name: String,
    pub charter_open: bool,
    pub reserve_food: f32,
    pub reserve_cap: f32,
    pub supply_years: f32,
    pub age_years: u32,
    pub indep_in_years: i32,
    /// Owning house (house outposts, kind 2) — empty for settlement colonies.
    pub owner_house_name: String,
    pub owner_color: String,
    /// Dedicated grain-run supply ships + what they delivered last month.
    pub supply_ships: u32,
    pub supply_delivered: f32,
}

/// Assemble a `ColonySummary` for colony hub `hi` (caller guarantees it IS a colony).
fn colony_summary(sim: &CampaignSim, hi: usize) -> ColonySummary {
    let hub = &sim.hubs[hi];
    let founder_hub_ref = (hub.founder_hub >= 0)
        .then(|| sim.hubs.get(hub.founder_hub as usize)).flatten();
    let founder_name = founder_hub_ref.map(|h| h.name.clone()).unwrap_or_default();
    let (founder_x, founder_y) = founder_hub_ref.map(|h| (h.x, h.y)).unwrap_or((-1.0, -1.0));
    let main_bank_name = (hub.main_bank >= 0)
        .then(|| sim.banks.get(hub.main_bank as usize))
        .flatten().map(|b| b.name.clone()).unwrap_or_default();
    // Age / independence only meaningful for a settlement colony (outposts don't
    // record a founding tick and never become independent).
    let (age_years, indep_in_years) = match hub.colony_kind {
        1 => { let a = (sim.tick.saturating_sub(hub.colony_founded_tick) / 365) as u32; (a, 70i32 - a as i32) }
        3 => { let a = (sim.tick.saturating_sub(hub.colony_founded_tick) / 365) as u32; (a, 40i32 - a as i32) }
        _ => (0, 0),
    };
    // House outpost owner (kind 2): the owning house, coloured like the Houses panel.
    let (owner_house_name, owner_color) = if hub.colony_kind == 2 && hub.owner_house >= 0 {
        let oh = hub.owner_house as usize;
        (sim.houses.get(oh).map(|h| h.name.clone()).unwrap_or_default(), distinct_color(oh))
    } else {
        (String::new(), String::new())
    };
    ColonySummary {
        id: hub.id, name: hub.name.clone(), x: hub.x, y: hub.y,
        colony_kind: hub.colony_kind, colony_stage: hub.colony_stage, autonomous: hub.autonomous,
        population: hub.population,
        founder_hub: hub.founder_hub, founder_name, founder_x, founder_y, main_bank_name,
        coin_name: hub.coin_name.clone(),
        charter_open: hub.colony_kind == 1 && !hub.autonomous,
        reserve_food: hub.reserve_food, reserve_cap: hub.reserve_cap, supply_years: hub.supply_years,
        age_years, indep_in_years,
        owner_house_name, owner_color,
        supply_ships: hub.supply_ships, supply_delivered: hub.supply_delivered,
    }
}

/// Empire-wide colony roster (settlement colonies + house outposts) for the
/// Colonial Office subwindow. Grouped client-side by `founder_hub`.
#[tauri::command]
pub fn campaign_get_colonies(db: State<'_, WorldDb>) -> Result<Vec<ColonySummary>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out = Vec::new();
    for hi in 0..sim.hubs.len() {
        if sim.hubs[hi].colony_kind != 0 {
            out.push(colony_summary(&sim, hi));
        }
    }
    Ok(out)
}

// Colony-founding gate thresholds — MIRRORED (read-only) from `sim/tick.rs`. The
// Colonial Office REPORTS these to explain why no colony exists yet; it does NOT
// change them. Keep in sync with the consts in tick.rs (COLONY_START_TICK = 30y,
// COLONY_PARENT_MIN_POP, COLONY_MIN_FERTILE/COLONY_MIN_TRADE, COLONY_HOP_REACH_FRAC,
// MAX_SETTLEMENT_COLONIES, and the founder filter in maybe_found_settlement_colony).
const GATE_START_YEAR: u32 = 30;
const GATE_MIN_POP: f32 = 5_000.0;
const GATE_PROSPERITY_MIN: f32 = 0.25;
const GATE_STARVING_MAX: f32 = 0.7;
// A site counts as colonisable when it can part-feed itself OR is rich in trade
// goods (matches maybe_found_settlement_colony's relaxed viability floor).
// Mirror of COLONY_MIN_FERTILE / COLONY_MIN_TRADE / COLONY_HOP_REACH_FRAC in
// sim/tick.rs — keep these in lockstep or the gate will lie about "0 in range".
const GATE_FERTILE_SITE: f32 = 0.12;
const GATE_TRADE_SITE: f32 = 0.18;
const GATE_HOP_REACH_FRAC: f32 = 0.42;
const GATE_MAX_SETTLEMENT_COLONIES: u32 = 24;

/// Read-only snapshot of the settlement-colony founding gates, for the Colonial
/// Office "why no colonies yet?" empty state. Mirrors the conditions in
/// `maybe_found_settlement_colony` WITHOUT changing them.
#[derive(Serialize)]
pub struct ColonyGateStatus {
    pub year: u32,
    pub start_year: u32,
    pub year_ok: bool,
    pub qualifying_founder: String,
    pub founder_ok: bool,
    pub bank_on_continent: bool,
    pub colonizable_sites_in_range: u32,
    pub site_ok: bool,
    pub settlement_colonies: u32,
    pub max_settlement_colonies: u32,
    pub at_colony_cap: bool,
    pub min_pop: f32,
    /// First failing gate: "cap" | "year" | "founder" | "bank" | "site" | "none".
    pub blocking_gate: String,
}

#[tauri::command]
pub fn campaign_colony_gates(db: State<'_, WorldDb>) -> Result<Option<ColonyGateStatus>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let year = sim.tick / 365;
    let year_ok = year >= GATE_START_YEAR;
    // Best qualifying founder (mirrors maybe_found_settlement_colony's filter/score).
    let mut founder: Option<usize> = None;
    let mut best = 0.0f32;
    for h in 0..sim.hubs.len() {
        let hub = &sim.hubs[h];
        if hub.is_estate || hub.colony_kind != 0 { continue; }
        if hub.population < GATE_MIN_POP || hub.starving > GATE_STARVING_MAX { continue; }
        if hub.sent_prosperity < GATE_PROSPERITY_MIN { continue; }
        let score = hub.population * hub.sent_prosperity.clamp(0.0, 1.0) * (0.2 + hub.treasury);
        if score > best { best = score; founder = Some(h); }
    }
    let founder_ok = founder.is_some();
    let qualifying_founder = founder.map(|h| sim.hubs[h].name.clone()).unwrap_or_default();
    // A bank on the founder's continent (component) — the hard, dominant gate.
    let comp = founder.map(|h| sim.hubs[h].component);
    let bank_on_continent = sim.banks.iter().any(|b| !b.defunct
        && sim.hubs.get(b.seat as usize)
            .map(|s| comp.map_or(true, |c| s.component == c)).unwrap_or(false));
    // Reachable fertile unsettled sites within the colony hop reach of the founder.
    let cap = sim.world_w * GATE_HOP_REACH_FRAC;
    let colonizable_sites_in_range = if let Some(h) = founder {
        let (fx, fy) = (sim.hubs[h].x, sim.hubs[h].y);
        sim.colonizable.iter().filter(|s| {
            if s.fertility < GATE_FERTILE_SITE && s.trade_value < GATE_TRADE_SITE { return false; }
            let mut dx = (s.x - fx).abs();
            if sim.world_w > 0.0 { dx = dx.min(sim.world_w - dx); } // cylindrical wrap on X
            let dy = s.y - fy;
            (dx * dx + dy * dy).sqrt() <= cap
        }).count() as u32
    } else {
        sim.colonizable.iter()
            .filter(|s| s.fertility >= GATE_FERTILE_SITE || s.trade_value >= GATE_TRADE_SITE)
            .count() as u32
    };
    let site_ok = colonizable_sites_in_range > 0;
    let settlement_colonies = sim.hubs.iter()
        .filter(|h| h.colony_kind == 1 && !h.autonomous).count() as u32;
    let at_colony_cap = settlement_colonies >= GATE_MAX_SETTLEMENT_COLONIES;
    let blocking_gate = if at_colony_cap { "cap" }
        else if !year_ok { "year" }
        else if !founder_ok { "founder" }
        else if !bank_on_continent { "bank" }
        else if !site_ok { "site" }
        else { "none" }.to_string();
    Ok(Some(ColonyGateStatus {
        year, start_year: GATE_START_YEAR, year_ok,
        qualifying_founder, founder_ok, bank_on_continent,
        colonizable_sites_in_range, site_ok,
        settlement_colonies, max_settlement_colonies: GATE_MAX_SETTLEMENT_COLONIES, at_colony_cap,
        min_pop: GATE_MIN_POP, blocking_gate,
    }))
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
                quality: hub.quality.get(g).copied().unwrap_or(0.0),
                grade: if hub.production[g] > 0.0 {
                    crate::sim::tick::quality_grade(hub.quality.get(g).copied().unwrap_or(0.0)).to_string()
                } else { String::new() },
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
                owner_is_civic: e.owner_house < 0,
                tier: e.estate_tier.max(1),
                damage: e.damage,
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
    // Social strata (None for estates / unseeded hubs where the shares are blank).
    let so = &hub.society;
    let society = if !hub.is_estate && (so.patrician + so.burgher + so.commoner + so.underclass) > 1e-3 {
        let welfare = (so.commoner_wealth / (so.commoner_wealth + 1.5)).clamp(0.0, 1.0);
        Some(SocietyBrief {
            patrician: so.patrician, burgher: so.burgher, commoner: so.commoner, underclass: so.underclass,
            commoner_wealth: so.commoner_wealth, inequality: so.inequality, welfare, unrest: so.unrest,
        })
    } else { None };
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
        use crate::sim::tick::{archetype_label, office_title, govt_type_name, govt_head_title,
            EXPORT_TAX_RATE, IMPORT_TAX_RATE, OFFICIAL_CAPTURE};
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
        // Key figures with their allegiance status.
        let hname = |hi: i32| -> (String, String) {
            if hi >= 0 { sim.houses.get(hi as usize)
                .map(|h| (h.name.clone(), distinct_color(hi as usize)))
                .unwrap_or_default() } else { (String::new(), String::new()) }
        };
        let officials: Vec<OfficialRow> = hub.officials.iter().map(|o| {
            let (an, ac) = hname(o.house);
            let status = if o.kin { "kin" } else if o.house < 0 { "neutral" }
                else if o.control >= OFFICIAL_CAPTURE { "controlled" } else { "leaning" };
            let role = if o.role == 0 { govt_head_title(hub.govt_type).to_string() }
                else { office_title(o.role).to_string() };
            OfficialRow { role, name: o.name.clone(), allegiance: an, allegiance_color: ac,
                control: o.control, status: status.to_string() }
        }).collect();
        // Family influence over the government: controlled-figure weight (×3) + council
        // seat (×2) + commercial influence here, normalised across houses to a %.
        let mut infl: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for o in &hub.officials {
            if o.house >= 0 && (o.kin || o.control >= OFFICIAL_CAPTURE) {
                let w = match o.role { 0 => 2.0, 1 => 1.4, _ => 1.0 };
                *infl.entry(o.house as usize).or_insert(0.0) += 3.0 * w;
            }
        }
        if ci >= 0 { *infl.entry(ci as usize).or_insert(0.0) += 2.0; }
        for (hidx, house) in sim.houses.iter().enumerate() {
            if house.defunct || house.is_guild { continue; }
            if let Some((_, v)) = house.influence.iter().find(|(c, _)| *c == hub.id
                || sim.hubs.get(*c as usize).map(|hh| hh.id == hub.id).unwrap_or(false)) {
                if *v > 0.02 { *infl.entry(hidx).or_insert(0.0) += *v; }
            }
        }
        let tot_infl: f32 = infl.values().sum::<f32>().max(1e-6);
        let mut family_influence: Vec<InfluenceRow> = infl.iter().map(|(&hi, &v)| InfluenceRow {
            name: sim.houses.get(hi).map(|h| h.name.clone()).unwrap_or_default(),
            color: distinct_color(hi), pct: v / tot_infl,
        }).filter(|r| !r.name.is_empty() && r.pct >= 0.01).collect();
        family_influence.sort_by(|a, b| b.pct.partial_cmp(&a.pct).unwrap_or(std::cmp::Ordering::Equal));
        family_influence.truncate(8);
        // Enacted-law log (newest first), rendered to text.
        let mut laws: Vec<LawRow> = hub.laws.iter().rev().take(8).map(|l| {
            let hn = hname(l.house).0;
            let gn = if l.good >= 0 { sim.goods.get(l.good as usize).map(|g| g.name.clone()).unwrap_or_default() } else { String::new() };
            let text = match l.kind {
                0 => format!("Favoured-house charter → {}", if hn.is_empty() { "a family".into() } else { hn }),
                1 => "Protectionist tariff raised".to_string(),
                2 => "Free-trade tariff cut".to_string(),
                3 => "The coin is debased".to_string(),
                4 => "Grain law — civic granary stocked".to_string(),
                5 => format!("Guild monopoly on {}", gn),
                _ => "A decree is issued".to_string(),
            };
            LawRow { year: l.year, text }
        }).collect();
        laws.retain(|l| !l.text.is_empty());
        // Government stores it holds (top few goods by amount).
        let mut civic_goods: Vec<CivicGoodRow> = hub.civic_goods.iter().enumerate()
            .filter(|(_, &a)| a > 0.5)
            .filter_map(|(g, &a)| sim.goods.get(g).map(|gd| CivicGoodRow { name: gd.name.clone(), amount: a }))
            .collect();
        civic_goods.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        civic_goods.truncate(6);
        // Years until the soonest seat turns over.
        let soonest = hub.officials.iter().map(|o| o.term_end).min().unwrap_or(sim.tick);
        let next_election_years = ((soonest.saturating_sub(sim.tick)) / 365) as i32;
        let (captor, captor_color) = hname(hub.captor_house);
        Some(Government {
            council, council_color, council_archetype, council_is_guild, council_power,
            tariff_export, tariff_import, tariff_default,
            mint_fineness: if hub.mint_fineness <= 0.0 { 1.0 } else { hub.mint_fineness },
            treasury: hub.treasury, civic_pool: hub.civic_pool,
            spec_risk, spec_tier, spec_stars, spec_pattern, spec_drivers, spec_watch,
            govt_type: govt_type_name(hub.govt_type).to_string(),
            next_election_years, captor, captor_color,
            officials, family_influence, laws, civic_goods,
        })
    };
    // ── DLC 3.5 · Carrying trade ("transit"): in-flight shipments run by THIS
    //    city's resident merchants (or via an office here) between two OTHER
    //    cities — the entrepôt handling-trade. Settled in a reserve coin if either
    //    endpoint mints one, else barter (priced in wheat-equivalent). ──
    let reserve_coin = |idx: usize| -> Option<String> {
        sim.hubs.get(idx).filter(|c| !c.coin_name.is_empty() && c.coin_trust >= 0.55)
            .map(|c| c.coin_name.clone())
    };
    let mut transit: Vec<TransitRow> = sim.in_transit.iter().filter_map(|s| {
        if s.owner < 0 { return None; }
        let oi = s.owner as usize;
        let house = sim.houses.get(oi)?;
        let resident = house.hub as usize == hi || house.offices.contains(&(hi as u32));
        let (from, to) = (s.from as usize, s.to as usize);
        if !resident || from == hi || to == hi { return None; }
        let price = sim.hubs.get(to).map(|c| c.price.get(s.good).copied().unwrap_or(0.0)).unwrap_or(0.0);
        let coin = reserve_coin(to).or_else(|| reserve_coin(from)).unwrap_or_default();
        let barter = if coin.is_empty() { format!("~{:.1} wheat/unit", price.max(0.0)) } else { String::new() };
        Some(TransitRow {
            merchant: house.name.clone(), is_guild: house.is_guild, color: distinct_color(oi),
            good: sim.goods.get(s.good).map(|g| g.name.clone()).unwrap_or_default(),
            amount: s.amount, value: s.amount * price,
            from_name: sim.hubs.get(from).map(|c| c.name.clone()).unwrap_or_default(),
            to_name: sim.hubs.get(to).map(|c| c.name.clone()).unwrap_or_default(),
            sea: s.sea, coin, barter,
        })
    }).collect();
    transit.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    transit.truncate(20);
    let war_with = if hub.war_with >= 0 {
        sim.hubs.get(hub.war_with as usize).map(|c| c.name.clone()).unwrap_or_default()
    } else { String::new() };
    let coin_value = if hub.coin_name.is_empty() { 0.0 }
        else { crate::sim::tick::coin_value(hub.mint_fineness, hub.coin_trust) };
    // Currency basket: which coins circulate here + their share (main coin first).
    let coin_basket: Vec<CoinShare> = hub.coin_basket.iter().filter_map(|&(k, share)| {
        let c = sim.hubs.get(k as usize)?;
        if c.coin_name.is_empty() { return None; }
        Some(CoinShare {
            coin_name: c.coin_name.clone(),
            share,
            main: hub.settle_coin == k as i32,
            reserve: hub.settle_coin != k as i32 && c.coin_trust >= 0.55,
        })
    }).collect();

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
        society,
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
        patron: sim.hub_patron.get(hi).copied().filter(|&p| p >= 0)
            .and_then(|p| sim.houses.get(p as usize)).map(|h| h.name.clone()).unwrap_or_default(),
        culture: sim.hub_culture.get(hi).cloned().unwrap_or_default(),
        minorities: sim.hub_minorities.get(hi).cloned().unwrap_or_default(),
        government,
        treasury: hub.treasury,
        finance: Some(hub.finance.clone()),
        war_with,
        coin_name: hub.coin_name.clone(),
        coin_trust: hub.coin_trust,
        coin_value,
        coin_basket,
        transit,
        stolen_good: if hub.stolen_good >= 0 {
            sim.goods.get(hub.stolen_good as usize).map(|g| g.name.clone()).unwrap_or_default()
        } else { String::new() },
        stolen_from: if hub.stolen_from >= 0 {
            sim.hubs.iter().find(|h| h.id == hub.stolen_from as u32).map(|h| h.name.clone()).unwrap_or_default()
        } else { String::new() },
        // Colonies/outposts this city founded (founder_hub is a hub INDEX == hi).
        related_colonies: (0..sim.hubs.len())
            .filter(|&ci| sim.hubs[ci].colony_kind != 0 && sim.hubs[ci].founder_hub == hi as i32)
            .map(|ci| colony_summary(&sim, ci))
            .collect(),
    }))
}

/// DLC 4 · one good's world-wide quality + trade picture, for the floating Goods
/// window. Aggregated across all producing hubs + in-flight cargo.
#[derive(Serialize, Clone)]
pub struct GoodMarketRow {
    pub good: String,
    pub best_quality: f32,
    pub best_grade: String,
    pub best_city: String,
    pub avg_quality: f32,
    pub produced: f32,
    pub traded: f32,
    pub n_producers: u32,
    pub manufactured: bool,
    /// Per quality-grade breakdown (Exquisite→Coarse), only non-empty tiers — so the
    /// Goods window can show e.g. Wheat · Fine / Standard / Coarse separately.
    pub grades: Vec<GradeBucket>,
}

/// DLC 4 · one quality tier of a good: how much is produced & in trade at that grade.
#[derive(Serialize, Clone)]
pub struct GradeBucket {
    pub grade: String,   // "Exquisite" | "Fine" | "Standard" | "Common" | "Coarse"
    pub produced: f32,
    pub traded: f32,
    pub n_producers: u32,
}

/// Grade tier index 0..4 (Coarse..Exquisite) for a quality 0..1.
fn grade_tier(q: f32) -> usize {
    if q >= 0.85 { 4 } else if q >= 0.68 { 3 } else if q >= 0.50 { 2 } else if q >= 0.32 { 1 } else { 0 }
}
const GRADE_NAMES: [&str; 5] = ["Coarse", "Common", "Standard", "Fine", "Exquisite"];

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
    pub top_goods: Vec<String>,         // top exported/traded goods by cumulative profit
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
    // ── DLC 3.5 · richer individual stats + bank/coin links ──
    /// This family owns a chartered bank (→ 🏦 badge + underline + Bank subtab).
    #[serde(default)] pub owns_bank: bool,
    /// Year the house was founded (→ "est. 71 · 88y old").
    #[serde(default)] pub founded_year: u32,
    /// The worst single-month loss the family ever took (grain-eq).
    #[serde(default)] pub worst_loss: f32,
    /// Count of goods the house has EVER monopolised (all-time).
    #[serde(default)] pub mono_ever_count: u32,
    /// If this family's council governs a minting city, the coin it issues (else "").
    #[serde(default)] pub coin_name: String,
    #[serde(default)] pub coin_value: f32,
    #[serde(default)] pub coin_trust: f32,
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
        // Influence-ranked "Active in" list. Estates/manufactories are NOT independent
        // places — they belong to a settlement — so fold their influence into the
        // PARENT settlement (an office there already links the house to that
        // manufactory). Keep the strongest influence seen at the settlement.
        let display_hub = |c: usize| -> usize {
            if c < nhubs && sim.hubs[c].is_estate && sim.hubs[c].parent >= 0 {
                sim.hubs[c].parent as usize
            } else { c }
        };
        let mut agg: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for &(hb, infl) in &h.influence {
            let d = display_hub(hb as usize);
            let e = agg.entry(d).or_insert(0.0);
            if infl > *e { *e = infl; }
        }
        let mut active: Vec<HouseCity> = agg.into_iter().map(|(c, infl)| {
            let hb = c as u32;
            // Does this house OWN an estate/manufactory sited at this settlement? Then
            // it's the OWNER (it just transports its own goods) rather than a buyer.
            let owns_here = sim.hubs.iter().any(|e|
                e.is_estate && e.owner_house == hi as i32 && e.parent == c as i32);
            let role = if hb == h.hub { "seat" }
                else if h.bailos.contains(&hb) { "bailo" }
                else if sim.city_dominator.get(c).copied().unwrap_or(-1) == hi as i32 { "dominant" }
                else if owns_here { "owner" }
                else if h.offices.contains(&hb) { "office" }
                else { "trade" };
            let contested = c < nhubs && hub_top2[c].1 >= 0.30;
            let p = seat_pos(c);
            HouseCity { name: hub_name(hb), x: p[0], y: p[1], influence: infl, role: role.into(), contested }
        }).collect();
        active.sort_by(|a, b| b.influence.partial_cmp(&a.influence).unwrap_or(std::cmp::Ordering::Equal));
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
        // The coin this family mints: if it is the council of a city with a coin.
        let minting = sim.hubs.iter()
            .find(|c| !c.is_estate && c.council_house == hi as i32 && !c.coin_name.is_empty());
        let (coin_for_house, coin_val_house, coin_trust_house) = match minting {
            Some(c) => (c.coin_name.clone(), crate::sim::tick::coin_value(c.mint_fineness, c.coin_trust), c.coin_trust),
            None => (String::new(), 0.0, 0.0),
        };
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
            top_goods: {
                // Top goods this family is KNOWN FOR — most profitable goods it has
                // traded (up to 3), so the list shows its trade identity (#14).
                let mut tg: Vec<(usize, f32)> = h.good_profit.iter().enumerate()
                    .filter(|(_, &p)| p > 0.0).map(|(g, &p)| (g, p)).collect();
                tg.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                tg.into_iter().take(3).map(|(g, _)| gname(g)).collect()
            },
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
            owns_bank: sim.banks.iter().any(|b| !b.defunct && b.house as usize == hi),
            founded_year: h.founded_tick / crate::sim::tick::TICKS_PER_YEAR,
            worst_loss: h.worst_loss,
            mono_ever_count: h.mono_ever.len() as u32,
            // The coin this family mints, if its council governs a minting city.
            coin_name: coin_for_house.clone(),
            coin_value: coin_val_house,
            coin_trust: coin_trust_house,
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
    #[serde(default)] pub delivered: f32,      // running total delivered to date
    #[serde(default)] pub fulfilled_pct: f32,  // delivered vs what was due by now (0-100)
    #[serde(default)] pub value: f32,          // grain-eq value moved so far
    #[serde(default)] pub sealed_at: String,   // city where the deal was struck (buyer)
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

/// #29 · one year's wealth-inequality reading among the merchant houses.
#[derive(Serialize, Clone, Default)]
pub struct InequalityPoint {
    pub year: u32,
    pub gini: f32,        // 0 = perfectly equal, →1 = one house holds it all
    pub active: u32,      // houses with positive wealth that year
    pub mean_wealth: f32,
    pub top10_share: f32, // wealth share of the richest 10%
}

/// #29 · the wealth-inequality & social-mobility snapshot for the Economy
/// Dashboard. Pure read over the campaign sim — Gini now, a yearly Gini/top-share
/// trend reconstructed from each house's `wealth_history`, plus turnover stats.
#[derive(Serialize, Clone, Default)]
pub struct InequalitySnapshot {
    pub active: bool,
    pub year: u32,
    pub gini_now: f32,
    pub top10_share_now: f32,
    pub active_houses: u32,
    pub defunct_houses: u32,
    pub founded_total: u32,
    /// 0 = a frozen pecking order, →1 = ranks reshuffle wildly year to year.
    pub rank_churn: f32,
    pub series: Vec<InequalityPoint>, // oldest → newest
}

/// Gini coefficient of a list of (positive) wealths. 0 = equal, →1 = concentrated.
fn gini_of(mut v: Vec<f32>) -> f32 {
    v.retain(|&x| x > 0.0);
    let n = v.len();
    if n < 2 { return 0.0; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sum: f32 = v.iter().sum();
    if sum <= 0.0 { return 0.0; }
    let mut cum = 0.0f32;
    for (i, &x) in v.iter().enumerate() { cum += (i as f32 + 1.0) * x; }
    (((2.0 * cum) / (n as f32 * sum)) - (n as f32 + 1.0) / n as f32).clamp(0.0, 1.0)
}

/// Wealth share held by the richest 10% (at least one house).
fn top10_share_of(mut v: Vec<f32>) -> f32 {
    v.retain(|&x| x > 0.0);
    if v.is_empty() { return 0.0; }
    v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let total: f32 = v.iter().sum();
    if total <= 0.0 { return 0.0; }
    let k = ((v.len() as f32 * 0.1).ceil() as usize).max(1);
    v.iter().take(k).sum::<f32>() / total
}

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
    /// Coin value index (≈1.2 = strong agio, <1 = debased/distrusted).
    pub coin_value: f32,
    /// Issuing house (the council) whose arms ride the coin; "" → use the city.
    pub coin_issuer: String,
    /// The polis this city is at war with ("" = at peace).
    pub war_with: String,
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
    /// Value index (≈1.2 strong agio, <1 debased). Display metric.
    pub value: f32,
    /// Issuing house (council) whose arms ride the coin; "" → use the city.
    pub issuer: String,
    /// Circulating amount = Σ over all holding cities of (throughput × share) — the
    /// coin's effective money supply across the world (display-only for now).
    pub circulating: f32,
    /// How many settlements hold this coin in their basket.
    pub held_in: u32,
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

/// v2.0 · one MINT/polis in the unified "Coin & Mints" view — the polis (treasury,
/// tariffs, council, war) AND its coin (strength, drivers, reach, price level)
/// fused into a single card, replacing the old split Poleis + Currencies tabs.
#[derive(Serialize, Clone)]
pub struct MintBrief {
    pub hub: u32,
    pub city: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    // ── civic (the polis behind the mint) ──
    pub treasury: f32,
    pub tariff_export: f32,
    pub tariff_import: f32,
    pub council: String,
    pub council_archetype: String,
    pub council_color: String,
    pub war_with: String,
    // ── the coin ("" coin_name = this polis mints none) ──
    pub coin_name: String,
    pub issuer: String,
    pub trust: f32,
    pub fineness: f32,
    pub value: f32,
    /// Single headline 0..100 (fineness × acceptance) — the number the card leads with.
    pub strength: f32,
    pub throughput: f32,
    pub is_reserve: bool,
    pub circulating: f32,
    pub held_in: u32,
    pub abroad: u32,
    // ── v2.0 monetary loop + reform ──
    /// Local price-level index (1.0 = par at start). Rises with debasement/money growth.
    pub price_level: f32,
    /// Honest-money mandate currently in force (no debasement allowed).
    pub under_mandate: bool,
    /// This mint has reformed its coinage at least once.
    pub reformed: bool,
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
                trust: h.coin_trust,
                fineness,
                value: if has_coin { coin_value(h.mint_fineness, h.coin_trust) } else { 0.0 },
                strength: if has_coin { coin_strength(fineness, h.coin_trust) } else { 0.0 },
                throughput,
                is_reserve: has_coin && h.coin_trust >= 0.55,
                circulating, held_in, abroad,
                price_level: if h.price_level <= 0.0 { 1.0 } else { h.price_level },
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

/// v2.0 · one entry in the MONETARY CHRONICLE — the dated story of money (mints,
/// debasements, reforms, bank foundings, runs, crashes) for the Shocks timeline.
#[derive(Serialize, Clone)]
pub struct MonetaryEvent {
    pub year: u32,
    pub tick: u32,
    pub kind: String,   // coinage | reform | run | bank | crash
    pub city: String,   // resolved hub name ("" = world)
    pub value: f32,
    pub text: String,
}

/// v2.0 · the monetary chronicle: journal rows about money & credit, newest first.
#[tauri::command]
pub fn campaign_monetary_chronicle(db: State<'_, WorldDb>) -> Result<Vec<MonetaryEvent>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    const KINDS: [&str; 5] = ["coinage", "reform", "run", "bank", "crash"];
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

/// One city's USE of a coin (for the coin-usage overlay + per-coin breakdown chart).
#[derive(Serialize)]
pub struct CoinUseCity {
    pub coin: u32,           // issuing-mint hub id — which coin this city settles in
    pub coin_name: String,
    pub city: u32,           // the city hub id using it
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub volume: f32,         // trade throughput settled in this coin at this city
    pub mint: bool,          // this city is the coin's own mint
    pub reserve_reach: bool, // a foreign reserve coin circulating here
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
            out.push(CoinUseCity {
                coin: coin_hub.id,
                coin_name: coin_hub.coin_name.clone(),
                city: h.id,
                name: h.name.clone(),
                x: h.x, y: h.y,
                volume: thru * share,
                mint,
                reserve_reach: !mint && coin_hub.coin_trust >= 0.55,
            });
        }
    }
    Ok(out)
}

/// DLC 3.5 · one bank's balance sheet + reach, for the Coin & Credit panel.
#[derive(Serialize, Clone)]
pub struct BankBrief {
    pub name: String,
    pub seat: String,
    /// The coin the bank banks in (its seat city's coin) + its value (×grain), so the
    /// balance sheet can be shown denominated in real money.
    pub coin_name: String,
    pub coin_value: f32,
    pub owner: String,
    /// Owning house index — lets the Houses panel match a bank to the selected house.
    pub owner_idx: u32,
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
    /// Book value of equity stakes + cumulative stake dividends collected.
    pub stake_book: f32,
    pub dividends_earned: f32,
    /// Seat coordinates (cell space) — for the bank icon on the map.
    pub seat_x: f32,
    pub seat_y: f32,
    /// Cities hosting a counting-house branch.
    pub branches: Vec<String>,
    /// Recent chronicle lines (founding, branches, defaults, failure).
    pub events: Vec<String>,
    /// Yearly balance-sheet history (charts).
    pub history: Vec<crate::sim::tick::BankSnapshot>,
    /// Every live loan/deal on the books, with its agreement terms.
    pub loans: Vec<BankLoanRow>,
    /// Equity stakes the bank holds in manufactories.
    pub stakes: Vec<BankStakeRow>,
}

/// One loan/deal on a bank's books, with the agreement terms (for the deals list).
#[derive(Serialize, Clone)]
pub struct BankLoanRow {
    /// Borrower name (house, guild, or city).
    pub borrower: String,
    /// "house" | "guild" | "polis".
    pub borrower_kind: String,
    /// "trade" | "guild_factory" | "guild_civic" | "treasury" | "colony".
    pub purpose: String,
    pub principal: f32,
    pub outstanding: f32,
    /// Monthly interest rate.
    pub rate: f32,
    pub start_year: u32,
    pub term_years: f32,
}

/// One equity stake a bank holds in a manufactory.
#[derive(Serialize, Clone)]
pub struct BankStakeRow {
    pub works: String,
    pub good: String,
    pub share: f32,
    pub basis: f32,
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

/// DLC 3.5 · the economic-war picture: active wars + the concluded-war log.
#[derive(Serialize, Clone)]
pub struct WarBrief {
    pub a: String,
    pub b: String,
    pub start_year: u32,
    pub years: u32,
    pub chest_a: f32,
    pub chest_b: f32,
    pub levies: f32,
    pub cause: String,
}
#[derive(Serialize, Clone)]
pub struct WarsPayload {
    pub active: Vec<WarBrief>,
    pub log: Vec<crate::sim::tick::WarRecord>,
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

/// Phase 6 · one plague-struck city inside an epidemic (for the Plagues panel + map).
#[derive(Serialize, Clone)]
pub struct PlagueCityBrief {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub deaths: u32,
    /// Population that survived the strike (immediate aftermath).
    pub pop: u32,
    /// Still under quarantine right now.
    pub active: bool,
    /// The city the pestilence was carried from ("" = spontaneous origin).
    pub from_name: String,
    /// Year this city was struck.
    pub year: u32,
    /// Spread step: 0 = the origin, 1,2,… as the pestilence travelled the lanes.
    pub order: u32,
}

/// Phase 6 · an EPIDEMIC = a contagion chain (all strikes sharing an outbreak id).
#[derive(Serialize, Clone)]
pub struct EpidemicBrief {
    pub id: u32,
    pub name: String,
    /// The city the outbreak began in.
    pub origin_name: String,
    pub start_year: u32,
    pub end_year: u32,
    pub active: bool,
    pub total_dead: u32,
    /// Plague category: 1 = Great Plague (rare, reaches ~4000 km along the lanes),
    /// 2 = Regional (reaches one further city), 3 = Local outbreak (stays put).
    pub category: u8,
    /// The named DISEASE (Bubonic Plague, Cholera, Malaria, …) + its transmission mode.
    #[serde(default)] pub disease: String,
    #[serde(default)] pub transmission: String,
    /// Cities hit, in SPREAD ORDER (origin first). Each `from_name`→`name` is a
    /// contagion route; the panel can re-sort by deaths.
    pub cities: Vec<PlagueCityBrief>,
}

/// Phase 6 · the Plagues & Epidemics panel: outbreaks grouped from the strike log,
/// active first then deadliest. The panel re-sorts client-side.
#[tauri::command]
pub fn campaign_get_epidemics(db: State<'_, WorldDb>) -> Result<Vec<EpidemicBrief>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    use std::collections::BTreeMap;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let name = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let mut groups: BTreeMap<u32, Vec<&crate::sim::tick::PlagueStrike>> = BTreeMap::new();
    for s in &sim.epidemics { groups.entry(s.outbreak).or_default().push(s); }
    let mut out: Vec<EpidemicBrief> = groups.into_iter().map(|(id, mut strikes)| {
        let start = strikes.iter().map(|s| s.start_tick).min().unwrap_or(0);
        let end = strikes.iter().map(|s| s.until_tick).max().unwrap_or(0);
        let active = strikes.iter().any(|s| s.until_tick > sim.tick);
        let total_dead: f32 = strikes.iter().map(|s| s.deaths).sum();
        // The outbreak's category = the (most severe) category recorded on its strikes
        // (legacy strikes stored 0 → treat as local cat-3).
        let category = strikes.iter().map(|s| if s.category == 0 { 3 } else { s.category })
            .min().unwrap_or(3);
        // Origin = the spontaneous strike (source < 0), else the earliest.
        let origin_hub = strikes.iter().find(|s| s.source < 0).map(|s| s.hub)
            .unwrap_or_else(|| strikes.iter().min_by_key(|s| s.start_tick).map(|s| s.hub).unwrap_or(0));
        // Chronological → the spread history (origin first, then each city as reached).
        strikes.sort_by_key(|s| s.start_tick);
        let cities: Vec<PlagueCityBrief> = strikes.iter().enumerate().map(|(i, s)| PlagueCityBrief {
            hub: s.hub,
            x: sim.hubs.get(s.hub as usize).map(|h| h.x).unwrap_or(0.0),
            y: sim.hubs.get(s.hub as usize).map(|h| h.y).unwrap_or(0.0),
            name: name(s.hub),
            deaths: s.deaths.round() as u32,
            pop: s.pop_at.round() as u32,
            active: s.until_tick > sim.tick,
            from_name: if s.source >= 0 { name(s.source as u32) } else { String::new() },
            year: s.start_tick / TICKS_PER_YEAR,
            order: i as u32,
        }).collect();
        // The named disease = the disease of the spontaneous (origin) strike.
        let dz = strikes.iter().find(|s| s.source < 0).map(|s| s.disease)
            .unwrap_or_else(|| strikes.first().map(|s| s.disease).unwrap_or(0));
        let dspec = crate::sim::tick::DISEASES.get(dz as usize);
        let disease = dspec.map(|s| s.name.to_string()).unwrap_or_else(|| "Pestilence".into());
        let transmission = match dspec.map(|s| s.mode).unwrap_or(0) {
            1 => "water-borne", 2 => "airborne", 3 => "vector · locale", _ => "trade-borne",
        }.to_string();
        EpidemicBrief {
            id,
            name: format!("{} of {}", disease, name(origin_hub)),
            origin_name: name(origin_hub),
            start_year: start / TICKS_PER_YEAR,
            end_year: end / TICKS_PER_YEAR,
            active,
            total_dead: total_dead.round() as u32,
            category,
            disease,
            transmission,
            cities,
        }
    }).collect();
    out.sort_by(|a, b| b.active.cmp(&a.active)
        .then(b.total_dead.cmp(&a.total_dead))
        .then(b.end_year.cmp(&a.end_year)));
    Ok(out)
}

/// Phase 6 · one craft guild (for the Guilds & Crafts panel + map).
#[derive(Serialize, Clone)]
pub struct GuildBrief {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub city: String,
    pub good: u32,
    pub good_name: String,
    pub quality: f32,
    pub output: f32,
    pub strength: f32,
    pub hall: bool,
    pub luxury: bool,
    /// True when the guild's craft is EXCEPTIONAL (renowned enough to be branded).
    pub exceptional: bool,
    /// Place-brand for an exceptional craft ("Veyra cloth" — like Murano glass /
    /// Damascus steel), else "". `culture` names the people whose style it is.
    pub brand: String,
    pub culture: String,
}

/// Quality at/above which a guild's craft earns a place-brand (renowned).
const GUILD_EXCEPTIONAL: f32 = 0.80;

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

/// Phase 6 · one notable figure (Great Lives roster).
#[derive(Serialize, Clone)]
pub struct FigureBrief {
    pub name: String,
    pub role: String,
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub city: String,
    /// The craft a master craftsman is renowned for (else "").
    pub good_name: String,
    pub born_year: u32,
    pub died_year: u32,
    pub alive: bool,
}

/// Phase 6 · the Notable Figures panel: the campaign's great lives, living first.
#[tauri::command]
pub fn campaign_get_figures(db: State<'_, WorldDb>) -> Result<Vec<FigureBrief>, String> {
    use crate::sim::tick::{TICKS_PER_YEAR, FIGURE_KINDS};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<FigureBrief> = sim.figures.iter().map(|f| {
        let h = sim.hubs.get(f.hub as usize);
        FigureBrief {
            name: f.name.clone(),
            role: FIGURE_KINDS.get(f.kind as usize).copied().unwrap_or("Figure").to_string(),
            hub: f.hub,
            x: h.map(|x| x.x).unwrap_or(0.0),
            y: h.map(|x| x.y).unwrap_or(0.0),
            city: h.map(|x| x.name.clone()).unwrap_or_default(),
            good_name: if f.good >= 0 {
                sim.goods.get(f.good as usize).map(|g| g.name.clone()).unwrap_or_default()
            } else { String::new() },
            born_year: f.born_tick / TICKS_PER_YEAR,
            died_year: if f.dead { f.dies_tick / TICKS_PER_YEAR } else { 0 },
            alive: !f.dead,
        }
    }).collect();
    out.sort_by(|a, b| b.alive.cmp(&a.alive).then(b.born_year.cmp(&a.born_year)));
    Ok(out)
}

/// Phase 6 · one landmark / place of note (wonders, holy cities, fair towns, guildhalls).
#[derive(Serialize, Clone)]
pub struct LandmarkBrief {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub city: String,
    /// "wonder" | "temple" | "fair" | "guildhall".
    pub kind: String,
    pub label: String,
    pub detail: String,
}

/// Phase 6 · the Landmarks & Sacred Sites panel: the world's places of note.
#[tauri::command]
pub fn campaign_get_landmarks(db: State<'_, WorldDb>) -> Result<Vec<LandmarkBrief>, String> {
    use crate::sim::tick::WONDER_NAMES;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<LandmarkBrief> = Vec::new();
    let good = |g: i32| if g >= 0 {
        sim.goods.get(g as usize).map(|x| x.name.clone()).unwrap_or_default()
    } else { String::new() };
    let mut push = |hub: u32, kind: &str, label: String, detail: String, out: &mut Vec<LandmarkBrief>| {
        if let Some(h) = sim.hubs.get(hub as usize) {
            out.push(LandmarkBrief { hub, x: h.x, y: h.y, city: h.name.clone(),
                kind: kind.to_string(), label, detail });
        }
    };
    for (hub, tier) in &sim.wonders {
        push(*hub, "wonder", WONDER_NAMES.get(*tier as usize).copied().unwrap_or("a wonder").to_string(),
            String::new(), &mut out);
    }
    for s in &sim.holy_sites {
        push(s.hub, "temple", if s.tier >= 2 { "Great holy city" } else { "Temple city" }.to_string(),
            if s.patron_good >= 0 { format!("patron: {}", good(s.patron_good)) } else { String::new() }, &mut out);
    }
    for f in &sim.fairs {
        push(f.hub, "fair", "Trade fair".to_string(), format!("opens month {}", f.month), &mut out);
    }
    for g in &sim.guilds {
        if g.hall {
            push(g.hub, "guildhall", format!("{} guildhall", good(g.good as i32)), String::new(), &mut out);
        }
    }
    Ok(out)
}

/// Phase 7 · a link between two houses (a marriage alliance or a feud).
#[derive(Serialize, Clone)]
pub struct HouseLink {
    pub a_name: String,
    pub b_name: String,
    pub a_hub: u32,
    pub b_hub: u32,
    pub ax: f32,
    pub ay: f32,
    pub bx: f32,
    pub by: f32,
    pub a_city: String,
    pub b_city: String,
}

/// Phase 7 · the Dynasties & Alliances panel: marriage alliances + feuds.
#[derive(Serialize, Clone)]
pub struct DynastiesPayload {
    pub alliances: Vec<HouseLink>,
    pub feuds: Vec<HouseLink>,
}

/// Phase 7 · marriage alliances (from `sim.alliances`) + feuds (from `House.rivals`)
/// between living houses, with their seat cities for the map.
#[tauri::command]
pub fn campaign_get_dynasties(db: State<'_, WorldDb>) -> Result<DynastiesPayload, String> {
    use std::collections::HashSet;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(DynastiesPayload { alliances: vec![], feuds: vec![] }),
    };
    let houses = &sim.houses;
    let link = |a: usize, b: usize| -> Option<HouseLink> {
        let (ha, hb) = (houses.get(a)?, houses.get(b)?);
        if ha.defunct || hb.defunct { return None; }
        let (ca, cb) = (sim.hubs.get(ha.hub as usize)?, sim.hubs.get(hb.hub as usize)?);
        Some(HouseLink {
            a_name: ha.name.clone(), b_name: hb.name.clone(),
            a_hub: ha.hub, b_hub: hb.hub,
            ax: ca.x, ay: ca.y, bx: cb.x, by: cb.y,
            a_city: ca.name.clone(), b_city: cb.name.clone(),
        })
    };
    let alliances: Vec<HouseLink> = sim.alliances.iter()
        .filter_map(|&(a, b)| link(a as usize, b as usize)).collect();
    // Feuds: unique (min,max) pairs from rivals lists among living houses.
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut feuds: Vec<HouseLink> = Vec::new();
    for (i, h) in houses.iter().enumerate() {
        if h.defunct { continue; }
        for &r in &h.rivals {
            let (lo, hi) = if i < r { (i, r) } else { (r, i) };
            if lo == hi || !seen.insert((lo, hi)) { continue; }
            if let Some(l) = link(lo, hi) { feuds.push(l); }
        }
    }
    Ok(DynastiesPayload { alliances, feuds })
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
    let mut partner_goods: HashMap<u32, HashMap<u32, f32>> = HashMap::new(); // partner→good→amt
    for f in sim.trade_last.iter().filter(|f| f.hub == hidx) {
        let partner = city_of(f.partner); // fold estates/manufactories into their settlement
        if partner == hidx { continue; }  // skip self-trade after folding (own estate)
        if f.dir == 0 { *g_in.entry(f.good).or_insert(0.0) += f.amount; }
        else { *g_out.entry(f.good).or_insert(0.0) += f.amount; }
        g_partners.entry(f.good).or_default().insert(partner);
        *route_amt.entry((f.good, partner, f.dir)).or_insert(0.0) += f.amount;
        *partner_vol.entry(partner).or_insert(0.0) += f.amount;
        *partner_goods.entry(partner).or_default().entry(f.good).or_insert(0.0) += f.amount;
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
    /// YEARLY wealth, oldest→newest, last ~10 years (the multi-year growth graph).
    pub wealth_years: Vec<f32>,
    /// Campaign year of the FIRST `wealth_years` sample (for the graph's X axis).
    pub wealth_start_year: u32,
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
        wealth_years: {
            // Last 10 YEARLY wealth samples (oldest→newest) for the growth graph.
            let wh = &sim.houses[house].wealth_history;
            let n = wh.len().min(10);
            wh[wh.len() - n..].to_vec()
        },
        wealth_start_year: {
            let wh = &sim.houses[house].wealth_history;
            let n = wh.len().min(10) as u32;
            (sim.tick / 365).saturating_sub(n.saturating_sub(1))
        },
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
    /// Colonies/outposts this house OWNS (outposts) or BACKED (joint-stock share).
    #[serde(default)] pub colonies: Vec<ColonySummary>,
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
    // Colonies this house owns (outposts, owner_house) or backed (kind-1 joint-stock).
    let colonies: Vec<ColonySummary> = (0..sim.hubs.len()).filter(|&ci| {
        let c = &sim.hubs[ci];
        c.colony_kind != 0 && (c.owner_house == idx as i32
            || c.backers.iter().any(|(k, i, _)| *k == 1 && *i == idx as u32))
    }).map(|ci| colony_summary(&sim, ci)).collect();
    Ok(Some(HouseHistory {
        name: h.name.clone(),
        color: distinct_color(idx),
        founder,
        founded_year: year(h.founded_tick),
        events,
        top_goods,
        defunct: h.defunct,
        colonies,
    }))
}

/// One city's live cost-of-living basket index (#30 Economy Dashboard · Prices).
#[derive(serde::Serialize)]
pub struct CityPriceIndex {
    pub name: String,
    /// Need-tier-weighted mean of price ÷ base_value across goods, ×100 (100 = world
    /// standard). Leans on staples (tier 0) over luxuries (tier 2).
    pub index: f32,
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
