//! The CAMPAIGN LIBRARY — a real folder on the user's disk holding their `.campaign`
//! saves, listed in-app with each save's year so a run can be picked up without
//! hunting through a file dialog.
//!
//! Two rules shape this module:
//!
//! * **The folder is the user's, not the app's.** It defaults somewhere they can find
//!   (Documents), it is openable in their file manager, and it is re-pointable. The app
//!   never hides saves in an opaque application-support directory.
//! * **Listing must never parse a `campaign_sim` blob.** A long run serializes to
//!   megabytes; a folder of twenty would make the list unusable. Every save writes a
//!   small `campaign_summary` header (see `campaign_commands::CampaignSummary`) and the
//!   listing reads only that, falling back to a bounded scan for older files.

use tauri::{Manager, State};
use crate::db::{WorldDb, metadata};
use crate::commands::campaign_commands::{CampaignSummary, WorldRef, current_world_ref};

/// One `.campaign` file as the library shows it.
#[derive(serde::Serialize)]
pub struct CampaignFileInfo {
    pub path: String,
    pub file_name: String,
    pub name: String,
    pub world_name: String,
    pub year: u32,
    pub tick: u32,
    pub hubs: u32,
    pub houses: u32,
    /// Unix seconds; falls back to the file's own mtime when the save carries no header.
    pub saved_at: u64,
    pub size_bytes: u64,
    /// True when this save's world matches the world currently open, i.e. it can be
    /// loaded right now. False also when no world is open.
    pub world_match: bool,
    /// False when the file carried no `campaign_summary` — the year shown was
    /// recovered by scanning, so the UI can mark it as approximate.
    pub has_header: bool,
}

/// Where the library lives. A path the user chose (persisted in the app config dir),
/// else `<Documents>/WorldForge2 Campaigns`, else an app-data fallback for a machine
/// with no Documents folder. Always created if missing.
pub fn library_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = if let Some(custom) = read_configured_dir(app) {
        custom
    } else {
        app.path()
            .document_dir()
            .map(|d| d.join("WorldForge2 Campaigns"))
            .or_else(|_| app.path().app_data_dir().map(|d| d.join("campaigns")))
            .map_err(|e| e.to_string())?
    };
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create the campaigns folder {}: {e}", dir.display()))?;
    Ok(dir)
}

fn config_file(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("campaign_library_dir.txt"))
}

fn read_configured_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let p = config_file(app).ok()?;
    let s = std::fs::read_to_string(p).ok()?;
    let s = s.trim();
    if s.is_empty() { None } else { Some(std::path::PathBuf::from(s)) }
}

/// The library folder's absolute path (created on first call).
#[tauri::command]
pub fn campaign_library_dir(app: tauri::AppHandle) -> Result<String, String> {
    Ok(library_dir(&app)?.to_string_lossy().to_string())
}

/// Point the library at a different folder. An empty string restores the default.
#[tauri::command]
pub fn set_campaign_library_dir(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let cfg = config_file(&app)?;
    let trimmed = path.trim();
    if trimmed.is_empty() {
        let _ = std::fs::remove_file(&cfg);
    } else {
        std::fs::write(&cfg, trimmed).map_err(|e| e.to_string())?;
    }
    campaign_library_dir(app)
}

/// Open the library folder in the OS file manager.
#[tauri::command]
pub fn reveal_campaign_library(app: tauri::AppHandle) -> Result<(), String> {
    let dir = library_dir(&app)?;
    // No opener plugin is installed, and this is the one place the app needs one.
    #[cfg(target_os = "windows")]
    let cmd = ("explorer", vec![dir.to_string_lossy().to_string()]);
    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![dir.to_string_lossy().to_string()]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let cmd = ("xdg-open", vec![dir.to_string_lossy().to_string()]);

    std::process::Command::new(cmd.0)
        .args(&cmd.1)
        .spawn()
        .map_err(|e| format!("could not open {}: {e}", dir.display()))?;
    Ok(())
}

/// Every `.campaign` file in the library, newest save first.
#[tauri::command]
pub fn list_campaigns(
    app: tauri::AppHandle,
    db: State<'_, WorldDb>,
) -> Result<Vec<CampaignFileInfo>, String> {
    let dir = library_dir(&app)?;
    let current: Option<WorldRef> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        current_world_ref(&conn).ok()
    };

    let mut out: Vec<CampaignFileInfo> = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(out), // an unreadable folder lists as empty, never as an error
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("campaign") {
            continue;
        }
        let meta = entry.metadata().ok();
        let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // A file we cannot read is skipped rather than failing the whole listing —
        // one corrupt save must not hide the other nineteen.
        if let Some(mut info) = read_campaign_file(&path, current.as_ref()) {
            info.size_bytes = size_bytes;
            if info.saved_at == 0 { info.saved_at = mtime; }
            out.push(info);
        }
    }
    out.sort_by(|a, b| b.saved_at.cmp(&a.saved_at).then_with(|| a.file_name.cmp(&b.file_name)));
    Ok(out)
}

/// Read one `.campaign` file's header without loading its simulation state.
fn read_campaign_file(
    path: &std::path::Path,
    current: Option<&WorldRef>,
) -> Option<CampaignFileInfo> {
    let src = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    let get = |key: &str| -> Option<String> {
        src.query_row(
            "SELECT value FROM campaign WHERE key = ?1",
            rusqlite::params![key],
            |r| r.get::<_, String>(0),
        )
        .ok()
    };

    let file_ref: Option<WorldRef> = get("world_ref").and_then(|v| serde_json::from_str(&v).ok());
    let world_match = match (&file_ref, current) {
        (Some(fr), Some(cur)) => {
            fr.fingerprint == cur.fingerprint
                && fr.grid_width == cur.grid_width
                && fr.grid_height == cur.grid_height
        }
        _ => false,
    };

    let header: Option<CampaignSummary> =
        get("campaign_summary").and_then(|v| serde_json::from_str(&v).ok());
    let has_header = header.is_some();
    let mut s = header.unwrap_or_default();
    if s.name.is_empty() {
        s.name = get("name").unwrap_or_else(|| "Campaign".to_string());
    }
    if s.world_name.is_empty() {
        s.world_name = file_ref.map(|f| f.world_name).unwrap_or_default();
    }
    // Pre-header saves: recover the tick with a bounded scan rather than deserializing
    // a multi-megabyte sim just to show a year in a list.
    if !has_header {
        if let Some(tick) = get("campaign_sim").and_then(|blob| scan_tick(&blob)) {
            s.tick = tick;
            s.year = tick / 365;
        }
    }

    Some(CampaignFileInfo {
        path: path.to_string_lossy().to_string(),
        file_name: path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
        name: s.name,
        world_name: s.world_name,
        year: s.year,
        tick: s.tick,
        hubs: s.hubs,
        houses: s.houses,
        saved_at: s.saved_at,
        size_bytes: 0,
        world_match,
        has_header,
    })
}

/// Find the serialized `CampaignSim`'s own `"tick":N` in the first stretch of the blob.
/// `tick` is a top-level field of the struct, so it appears early; capping the search
/// keeps this O(1) on a huge save instead of scanning megabytes.
fn scan_tick(blob: &str) -> Option<u32> {
    const WINDOW: usize = 4096;
    let head = &blob[..blob.len().min(WINDOW)];
    let at = head.find("\"tick\":")? + "\"tick\":".len();
    let digits: String = head[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Save the current campaign into the library folder under a filesystem-safe name
/// derived from the campaign's own name. Returns the path written.
#[tauri::command]
pub fn save_campaign_to_library(
    app: tauri::AppHandle,
    name: Option<String>,
    db: State<'_, WorldDb>,
) -> Result<String, String> {
    let dir = library_dir(&app)?;
    let label = match name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            let conn = db.conn.lock().map_err(|e| e.to_string())?;
            metadata::campaign_get(&conn, "name")
                .map_err(|e| e.to_string())?
                .unwrap_or_else(|| "campaign".to_string())
        }
    };
    let path = dir.join(format!("{}.campaign", safe_file_stem(&label)));
    let path_str = path.to_string_lossy().to_string();
    crate::commands::campaign_commands::save_campaign_as(path_str.clone(), db)?;
    Ok(path_str)
}

/// Delete a save from the library. Refuses any path outside the library folder, so a
/// bad argument can never delete something else on the user's disk.
#[tauri::command]
pub fn delete_campaign_file(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let dir = library_dir(&app)?.canonicalize().map_err(|e| e.to_string())?;
    let target = std::path::PathBuf::from(&path)
        .canonicalize()
        .map_err(|e| format!("{path}: {e}"))?;
    if !target.starts_with(&dir) {
        return Err("Refusing to delete a file outside the campaigns folder.".into());
    }
    if target.extension().and_then(|e| e.to_str()) != Some("campaign") {
        return Err("Only .campaign files can be deleted from the library.".into());
    }
    std::fs::remove_file(&target).map_err(|e| e.to_string())
}

/// Collapse a campaign name into something every filesystem accepts, without
/// producing an empty stem.
fn safe_file_stem(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
        .collect();
    // Trim the SUBSTITUTES too, not just the original punctuation: every disallowed
    // char has already become `_` by this point, so a name that was all punctuation
    // would otherwise survive as "___" — a valid filename, but not a usable label.
    let trimmed = cleaned.trim().trim_matches(|c: char| c == '.' || c == '_' || c == ' ').to_string();
    if trimmed.is_empty() { "campaign".to_string() } else { trimmed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_campaign_name_always_yields_a_usable_file_stem() {
        assert_eq!(safe_file_stem("House of Aral"), "House of Aral");
        assert_eq!(safe_file_stem("a/b:c*d?"), "a_b_c_d");
        // The cases that would otherwise write ".campaign" (hidden) or "/.campaign".
        assert_eq!(safe_file_stem(""), "campaign");
        assert_eq!(safe_file_stem("   "), "campaign");
        assert_eq!(safe_file_stem("..."), "campaign");
    }

    #[test]
    fn the_tick_scan_reads_a_header_without_parsing_the_blob() {
        assert_eq!(scan_tick(r#"{"seed":7,"tick":18250,"hubs":[]}"#), Some(18250));
        assert_eq!(scan_tick(r#"{"seed":7,"hubs":[]}"#), None);
        // A blob whose tick sits past the window is reported as unknown rather than
        // costing a megabyte-long scan — the listing shows "—", it does not lie.
        let far = format!("{}\"tick\":42", " ".repeat(9000));
        assert_eq!(scan_tick(&far), None);
    }
}
