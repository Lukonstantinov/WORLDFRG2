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
const CAMPAIGN_KEYS: [&str; 6] = [
    "name",
    "settlements",
    "economy",
    "bio_params",
    "campaign_progress",
    "world_ref",
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
