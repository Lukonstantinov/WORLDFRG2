//! Save compatibility (owner decision 2026-10-15): only files written by THIS save
//! format load; anything older is refused with a clear message. These tests make a
//! world + campaign with the current build, save both, reopen them in a fresh app,
//! and check that an unstamped (older) file is refused without touching the open world.

use crate::db::WorldDb;
use crate::commands::{sim_commands, world_commands};
use crate::commands::file_commands::{open_world, save_world_as};
use crate::commands::query_commands::compute_economy;
use crate::commands::campaign_commands::*;
use tauri::Manager;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("wf2_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Small real world → campaign at day 30 → `X.worldforge` + `X.campaign` in `dir`.
async fn make_saves(dir: &std::path::Path) -> (String, String) {
    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().unwrap());
    let s = app.state::<WorldDb>();
    world_commands::new_world("roundtrip".into(), 256, 128, s.clone()).unwrap();
    let run = sim_commands::sim_run_all(7, 8, "plates".into(), 0.5, 0.5, 0.5, 0.5, s.clone()).unwrap();
    compute_economy(serde_json::to_string(&run.settlements).unwrap(), serde_json::to_string(&run.rivers).unwrap(),
        0, 0.15, false, 6, 0.5, 0.0, -1, 1, s.clone()).unwrap();
    finalize_world(s.clone()).unwrap();
    campaign_start_sim(7, s.clone()).unwrap();
    campaign_advance(30, s.clone()).await.unwrap();
    let w = dir.join("roundtrip.worldforge").to_string_lossy().into_owned();
    let c = dir.join("roundtrip.campaign").to_string_lossy().into_owned();
    save_world_as(w.clone(), s.clone()).unwrap();
    save_campaign_as(c.clone(), s.clone()).unwrap();
    (w, c)
}

#[tokio::test]
async fn a_new_world_and_campaign_save_reopen_and_resume() {
    let dir = temp_dir("roundtrip");
    let (w, c) = make_saves(&dir).await;

    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().unwrap());
    let s = app.state::<WorldDb>();
    let world = open_world(w, s.clone()).expect("world reopens");
    assert_eq!((world.meta.grid_width, world.meta.grid_height), (256, 128));
    // Slice 4: the matching .campaign beside it is offered.
    assert_eq!(world.sibling_campaign.as_deref(), Some(c.as_str()));
    // A world file carries its economy, so a fresh campaign can start straight from it.
    assert!(world_human_layer_status(s.clone()).unwrap().can_start_campaign);

    let info = open_campaign(c, s.clone()).expect("campaign reopens");
    assert!(info.world_match);
    let snap = campaign_start_sim(7, s.clone()).expect("resumes the saved run");
    assert!(snap.hubs.len() >= 2);
    assert_eq!(snap.clock.tick, 30, "resumes where it stopped, not from zero");
    let next = campaign_advance(1, s.clone()).await.unwrap();
    assert_eq!(next.clock.tick, 31);

    // The streamed save blob reloads to the same run.
    campaign_persist(s.clone()).unwrap();
    s.invalidate_campaign();
    let back = campaign_get_state(s.clone()).unwrap();
    assert_eq!((back.clock.tick, back.hubs.len()), (31, next.hubs.len()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn an_unstamped_older_save_is_refused_cleanly() {
    let dir = temp_dir("unstamped");
    let (w, c) = make_saves(&dir).await;
    // Simulate a file from before the format stamp existed.
    for p in [&w, &c] {
        rusqlite::Connection::open(p).unwrap().pragma_update(None, "user_version", 0).unwrap();
    }

    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().unwrap());
    let s = app.state::<WorldDb>();
    world_commands::new_world("current".into(), 128, 64, s.clone()).unwrap();
    let err = open_world(w.clone(), s.clone()).err().expect("an older world file is refused");
    assert!(err.contains("older version"), "{err}");
    let name: Option<String> = crate::db::metadata::get_meta(&s.conn.lock().unwrap(), "name").unwrap();
    assert_eq!(name.as_deref(), Some("current"), "a refused file must not replace the open world");
    assert!(open_campaign(c, s.clone()).err().expect("refused").contains("older version"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `advance_resilient` (now holding at most two copies of the sim): a persistent
/// fault on one tick must neither crash nor stall the run — the bad tick is skipped,
/// every other tick simulates, and the advance completes in full.
#[tokio::test]
async fn a_faulting_tick_is_skipped_and_the_advance_completes() {
    use crate::sim::tick::TEST_PANIC_AT_TICK;
    let dir = temp_dir("fault");
    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().unwrap());
    let s = app.state::<WorldDb>();
    world_commands::new_world("fault".into(), 256, 128, s.clone()).unwrap();
    let run = sim_commands::sim_run_all(7, 8, "plates".into(), 0.5, 0.5, 0.5, 0.5, s.clone()).unwrap();
    compute_economy(serde_json::to_string(&run.settlements).unwrap(), serde_json::to_string(&run.rivers).unwrap(),
        0, 0.15, false, 6, 0.5, 0.0, -1, 1, s.clone()).unwrap();
    finalize_world(s.clone()).unwrap();
    let start = campaign_start_sim(7, s.clone()).unwrap().clock.tick;

    TEST_PANIC_AT_TICK.with(|t| t.set(Some(start + 10)));
    let after = campaign_advance(30, s.clone()).await.expect("never errors on a tick fault");
    TEST_PANIC_AT_TICK.with(|t| t.set(None));
    assert_eq!(after.clock.tick, start + 30, "the advance completes in full");
    let conn = s.conn.lock().unwrap();
    let sim = get_sim(&s, &conn).unwrap().unwrap();
    assert!(sim.journal.iter().any(|j| j.text.contains("troubled season")),
        "the poisoned tick is recorded as skipped");
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}
