//! IMPROVEMENT_PLAN Phase 1 — old saves must keep working. The fixtures in
//! `src-tauri/test_fixtures/` were written by commit 231f75b, the last build before
//! `save_world_as` stopped stripping the economy (c9c3ad4): a 256×128 world, a
//! campaign advanced 30 days. Never regenerate them with the current build — a
//! fixture written by today's code proves nothing about yesterday's files.

use crate::db::WorldDb;
use crate::commands::file_commands::open_world;
use crate::commands::campaign_commands::{open_campaign, campaign_start_sim, campaign_advance};
use tauri::Manager;

fn fixture(name: &str) -> String {
    format!("{}/test_fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[tokio::test]
async fn an_old_world_and_its_campaign_still_open_and_run() {
    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().unwrap());
    let s = app.state::<WorldDb>();

    let world = open_world(fixture("old_world.worldforge"), s.clone()).expect("old world opens");
    assert_eq!((world.meta.grid_width, world.meta.grid_height), (256, 128));
    // Slice 4: the matching .campaign beside it is found and offered.
    assert_eq!(world.sibling_campaign.as_deref(), Some(fixture("old_world.campaign").as_str()));
    // Pre-fix world files carry no economy: this is the documented limitation (I-4).
    assert!(campaign_start_sim(7, s.clone()).is_err(),
        "an old world file alone has no economy; the recovery path is its .campaign");

    let info = open_campaign(fixture("old_world.campaign"), s.clone()).expect("old campaign opens");
    assert!(info.world_match, "the campaign must recognise the world it was saved against");
    assert!(info.world_name.is_some(), "a campaign names the world it belongs to");

    let snap = campaign_start_sim(7, s.clone()).expect("resumes the saved run");
    assert!(snap.hubs.len() >= 2, "campaign has hubs, got {}", snap.hubs.len());
    assert_eq!(snap.clock.tick, 30, "the saved run resumes where it stopped, not from zero");

    let next = campaign_advance(1, s.clone()).await.expect("runs one tick");
    assert_eq!(next.clock.tick, 31);
}

/// Slice 4's other half: no campaign file beside the world, nothing offered.
#[test]
fn a_world_with_no_campaign_beside_it_offers_none() {
    let dir = std::env::temp_dir().join(format!("wf2_sibling_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lone = dir.join("lone.worldforge");
    std::fs::copy(fixture("old_world.worldforge"), &lone).unwrap();
    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().unwrap());
    let res = open_world(lone.to_string_lossy().into_owned(), app.state::<WorldDb>()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(res.sibling_campaign, None);
}
