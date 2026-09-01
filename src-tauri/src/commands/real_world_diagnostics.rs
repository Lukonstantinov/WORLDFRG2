//! WORLD_AND_TRADE_MASTER_PLAN.md Part III §4 — the missing validation harness.
//!
//! Every economy gate in `economy_validation.rs` runs on a SYNTHETIC in-memory
//! `CampaignSim` (hand-built hubs, straight-line `days`) that never calls
//! `campaign_start_sim` and never touches a `WorldDb`/SQLite/tile pipeline. That
//! is deliberate for those gates' own purpose (isolate the economy mechanism
//! from worldgen noise), but it means NOTHING in this codebase's test suite can
//! answer "does a change to the real worldgen→campaign pipeline actually move
//! the market-integration numbers on a real generated world?" — exactly the
//! question Part III §4's own gate needs answered (its river-metadata-plumbing
//! fix could not be validated against `econ_fidelity_scorecard` for precisely
//! this reason; see that commit).
//!
//! This module builds a REAL world end-to-end through the actual Tauri command
//! functions (`sim_run_all` → `compute_economy` → `campaign_start_sim` →
//! `campaign_advance`), using `tauri::test::mock_app()` for a real (if headless)
//! `State<WorldDb>` — not a hand-built fixture — so a change anywhere in that
//! chain, worldgen included, is actually exercised. Test-only; adds nothing to
//! the shipped binary.
//!
//! ```bash
//! cargo test --lib real_world_price_distance_gradient -- --ignored --nocapture
//! ```

use crate::db::WorldDb;
use crate::commands::{sim_commands, world_commands};
use crate::commands::query_commands::compute_economy;
use crate::commands::campaign_commands::{finalize_world, campaign_start_sim, campaign_advance, get_sim};
use tauri::Manager;

fn pearson(xs: &[f32], ys: &[f32]) -> f32 {
    let n = xs.len().min(ys.len());
    if n < 3 { return 0.0; }
    let mx = xs[..n].iter().sum::<f32>() / n as f32;
    let my = ys[..n].iter().sum::<f32>() / n as f32;
    let mut num = 0.0;
    let (mut dx, mut dy) = (0.0f32, 0.0f32);
    for i in 0..n {
        let (a, b) = (xs[i] - mx, ys[i] - my);
        num += a * b;
        dx += a * a;
        dy += b * b;
    }
    if dx < 1e-12 || dy < 1e-12 { return 0.0; }
    num / (dx.sqrt() * dy.sqrt())
}

/// Build a small real world (through the actual `sim_run_all` command, so it
/// carries real provinces/rivers — CLAUDE.md rule 11 discipline: mirror the
/// real pipeline, don't shortcut it), start a real campaign on it, advance it,
/// and report the grain price×distance gradient `econ_fidelity_scorecard`
/// tracks. `#[ignore]`d — a full world-gen + N-year campaign run is not a fast
/// unit test.
#[tokio::test]
#[ignore]
async fn real_world_price_distance_gradient() {
    let app = tauri::test::mock_app();
    let db = WorldDb::in_memory().expect("in-memory WorldDb");
    app.manage(db);
    let state = app.state::<WorldDb>();

    // A small-but-real world: big enough for real provinces/rivers/hub spread,
    // small enough that plates→biological and a real campaign run finish in a
    // reasonable diagnostic time rather than minutes — campaign_advance runs on
    // a worker thread in the real app specifically because a real campaign at
    // real scale is genuinely heavy (its own doc comment), which this harness
    // inherits: keep the world and the run small on purpose.
    eprintln!("[diagnostic] generating world…");
    world_commands::new_world("diagnostic".into(), 300, 150, state.clone())
        .expect("new_world");

    let seed = 424242u64;
    let run = sim_commands::sim_run_all(
        seed, 10, "plates".into(), 0.5, 0.5, 0.5, 0.5, state.clone(),
    ).expect("sim_run_all");
    assert!(run.settlements.len() >= 8, "reference world needs a real hub spread, got {}", run.settlements.len());
    eprintln!("[diagnostic] world generated: {} settlements, {} rivers", run.settlements.len(), run.rivers.len());

    let settlements_json = serde_json::to_string(&run.settlements).unwrap();
    let rivers_json = serde_json::to_string(&run.rivers).unwrap();

    let econ = compute_economy(
        settlements_json, rivers_json, 0, 0.15, false, 6, 0.5, 0.0, -1, 1, state.clone(),
    ).expect("compute_economy");
    assert!(econ.hubs.len() >= 8, "economy snapshot needs real hubs, got {}", econ.hubs.len());
    eprintln!("[diagnostic] economy built: {} hubs", econ.hubs.len());

    finalize_world(state.clone()).expect("finalize_world");
    campaign_start_sim(seed, state.clone()).expect("campaign_start_sim");
    eprintln!("[diagnostic] campaign started, advancing…");

    // 20 years, in 3650-tick (10-year) chunks — campaign_advance clamps a single
    // call to 3650 ticks.
    for i in 0..2 {
        campaign_advance(3650, state.clone()).await.expect("campaign_advance");
        eprintln!("[diagnostic] advanced decade {}", i + 1);
    }

    let (gradient, n_pairs) = {
        let conn = state.conn.lock().unwrap();
        let sim = get_sim(&state, &conn).unwrap().expect("a campaign sim must be resident by now");
        const GRAIN: usize = 0; // reference world's goods[0] is always the first food good
        let live: Vec<usize> = (0..sim.hubs.len())
            .filter(|&i| !sim.hubs[i].abandoned && sim.hubs[i].population > 1.0)
            .collect();
        let n = sim.hubs.len();
        let mut dists = Vec::new();
        let mut gaps = Vec::new();
        for (ai, &a) in live.iter().enumerate() {
            for &b in live.iter().skip(ai + 1) {
                let d = sim.days.get(a * n + b).copied().unwrap_or(f32::INFINITY);
                if !d.is_finite() || d <= 0.0 { continue; }
                let (pa, pb) = (sim.hubs[a].price[GRAIN], sim.hubs[b].price[GRAIN]);
                if !(pa.is_finite() && pb.is_finite() && pa > 0.0 && pb > 0.0) { continue; }
                dists.push(d);
                gaps.push((pa.ln() - pb.ln()).abs());
            }
        }
        (pearson(&dists, &gaps), dists.len())
    };

    println!();
    println!("═══ Real-world price/distance gradient (WORLD_AND_TRADE_MASTER_PLAN.md Part III §4) ═══");
    println!("  hub pairs measured: {n_pairs}");
    println!("  grain price gap × distance (r): {gradient:.3}   (positive = distance costs, per Federico/Persson)");
    println!("═══════════════════════════════════════════════════════════════════════════════════════");

    assert!(n_pairs >= 3, "need at least 3 finite-distance, priced hub pairs to measure a gradient");
}
