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

/// ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md Stage D3 — the gate the plan names:
/// "the share of hubs flagged riverine must fall substantially; a flag almost
/// everything sets is the thing being fixed." Measures the NEW landmark-based
/// `hub.river` flag `campaign_start_sim` actually ships against a reconstruction
/// of the OLD blanket "within 1% of world width of any navigable-river point"
/// rule, on the SAME real generated world and the SAME `rivers_json` — a real
/// paired before/after rather than two separately-run worlds, so the comparison
/// cannot be an artefact of world-to-world noise. `#[ignore]`d for the same
/// reason `real_world_price_distance_gradient` is: a full worldgen run.
#[tokio::test]
#[ignore]
async fn river_class_flag_is_tighter_than_the_old_navigable_radius() {
    let app = tauri::test::mock_app();
    let db = WorldDb::in_memory().expect("in-memory WorldDb");
    app.manage(db);
    let state = app.state::<WorldDb>();

    // D3's radius is stated in km and this test must judge it at something
    // close to PRODUCTION resolution (3600 wide, ~11 km/cell) — the standard
    // 300-wide diagnostic world used elsewhere in this file is ~134 km/cell,
    // which makes a 25 km radius sub-cell and would fail this gate on grid
    // coarseness alone rather than on the mechanism (rule 21/25's own lesson).
    const DIAG_W: u32 = 1800;
    const DIAG_H: u32 = 900;
    world_commands::new_world("diagnostic".into(), DIAG_W, DIAG_H, state.clone())
        .expect("new_world");
    let seed = 424242u64;
    let run = sim_commands::sim_run_all(
        seed, 10, "plates".into(), 0.5, 0.5, 0.5, 0.5, state.clone(),
    ).expect("sim_run_all");
    assert!(run.settlements.len() >= 8, "reference world needs a real hub spread, got {}", run.settlements.len());

    let settlements_json = serde_json::to_string(&run.settlements).unwrap();
    let rivers_json = serde_json::to_string(&run.rivers).unwrap();
    let econ = compute_economy(
        settlements_json, rivers_json, 0, 0.15, false, 6, 0.5, 0.0, -1, 1, state.clone(),
    ).expect("compute_economy");

    eprintln!("[diagnostic] world generated: {} settlements, {} rivers ({} mouths, {} tributaries, {} navigable trunks)",
        run.settlements.len(), run.rivers.len(),
        run.rivers.iter().filter(|r| r.mouth_kind != 0).count(),
        run.rivers.iter().filter(|r| r.tributary).count(),
        run.rivers.iter().filter(|r| r.navigable && !r.tributary).count());

    finalize_world(state.clone()).expect("finalize_world");
    campaign_start_sim(seed, state.clone()).expect("campaign_start_sim");

    let new_share = {
        let conn = state.conn.lock().unwrap();
        let sim = get_sim(&state, &conn).unwrap().expect("a campaign sim must be resident");
        let n = sim.hubs.len().max(1);
        sim.hubs.iter().filter(|h| h.river).count() as f32 / n as f32
    };

    // Reconstruct the OLD rule directly (blanket radius, every navigable point)
    // on the same hubs/rivers, so the comparison is paired rather than a
    // separately-generated world.
    let grid_w = DIAG_W as f32;
    let nav_pts: Vec<(f32, f32)> = run.rivers.iter().filter(|r| r.navigable)
        .flat_map(|r| r.points.iter().map(|&(x, y)| (x as f32, y as f32)))
        .collect();
    let old_max_d2 = (grid_w * 0.01).powi(2);
    let old_share = if nav_pts.is_empty() { 0.0 } else {
        let n = econ.hubs.len().max(1);
        econ.hubs.iter().filter(|eh| {
            nav_pts.iter().any(|&(rx, ry)| {
                let (dx, dy) = (eh.x - rx, eh.y - ry);
                dx * dx + dy * dy <= old_max_d2
            })
        }).count() as f32 / n as f32
    };

    println!();
    println!("═══ River-hub flag share, old blanket radius vs new landmark class (D3) ═══");
    println!("  old (any navigable point, ~400 km): {:.1}%", old_share * 100.0);
    println!("  new (mouth/confluence/head-of-nav, 25 km): {:.1}%", new_share * 100.0);
    println!("═══════════════════════════════════════════════════════════════════════════");

    assert!(
        new_share <= old_share * 0.75 || (old_share - new_share) >= 0.10,
        "the tightened river flag should cover substantially fewer hubs: old {old_share:.3}, new {new_share:.3}"
    );
}


/// `docs/INSTITUTIONS_BUILD_ORDER.md` Phase 0.2 · DIAGNOSTIC, not a gate.
///
/// Answers the question the maintainer deferred rather than guessing at it:
/// **how wide is the world's craft-quality spread today, and is the finest
/// maker of a good simply the biggest city that makes it?**
///
/// `update_good_quality` (`tick/production.rs`) sets each hub's ceiling at
/// `0.62 + size_bonus(<=0.20) + struct_bonus(<=0.14)` and `QUALITY_LEARN_RATE`
/// (0.04/month) drives every producer to its own cap within a few years, so the
/// prediction from reading the code is that quality tracks POPULATION almost
/// perfectly and the realised price spread (`quality_value_mult` = 0.6 + 0.9q)
/// is narrow. This measures whether that is actually so.
///
/// It must run on a REAL world, not on `economy_validation.rs`'s synthetic
/// `reference_world`: that fixture ships six goods and **not one of them has a
/// recipe**, so it has no manufactured goods, no manufactories and no quality
/// learning to measure. A diagnostic run there would report a tidy zero and
/// measure the fixture (§8.20/§8.24c's own repeated lesson).
///
/// ```bash
/// cargo test --lib real_world_craft_spread -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore]
async fn real_world_craft_spread() {
    let app = tauri::test::mock_app();
    let db = WorldDb::in_memory().expect("in-memory WorldDb");
    app.manage(db);
    let state = app.state::<WorldDb>();

    eprintln!("[diagnostic] generating world…");
    world_commands::new_world("craft-spread".into(), 300, 150, state.clone())
        .expect("new_world");
    let seed = 424242u64;
    let run = sim_commands::sim_run_all(
        seed, 10, "plates".into(), 0.5, 0.5, 0.5, 0.5, state.clone(),
    ).expect("sim_run_all");
    let settlements_json = serde_json::to_string(&run.settlements).unwrap();
    let rivers_json = serde_json::to_string(&run.rivers).unwrap();
    let econ = compute_economy(
        settlements_json, rivers_json, 0, 0.15, false, 6, 0.5, 0.0, -1, 1, state.clone(),
    ).expect("compute_economy");
    eprintln!("[diagnostic] economy built: {} hubs", econ.hubs.len());

    finalize_world(state.clone()).expect("finalize_world");
    campaign_start_sim(seed, state.clone()).expect("campaign_start_sim");
    // 40 years: manufactories have to be FOUNDED before any quality can
    // accumulate, and `maybe_found_guild_workshop` is a yearly roll — 20 years
    // measures the founding lag as much as the spread.
    eprintln!("[diagnostic] campaign started, advancing 40y…");
    for i in 0..4 {
        campaign_advance(3650, state.clone()).await.expect("campaign_advance");
        eprintln!("[diagnostic] advanced decade {}", i + 1);
    }

    let conn = state.conn.lock().unwrap();
    let sim = get_sim(&state, &conn).unwrap().expect("a campaign sim must be resident");
    let ng = sim.goods.len();

    println!("\n── craft spread · real world · 40y ──────────────────────────────");
    println!("  ceiling today = 0.62 + size_bonus(<=0.20) + struct_bonus(<=0.14)");
    println!("  price effect  = quality_value_mult(q) = 0.6 + 0.9q\n");
    println!("  {:<18} {:>5} {:>6} {:>6} {:>6} {:>7} {:>5} {}",
             "manufactured good", "hubs", "minQ", "medQ", "maxQ", "price×", "@top", "leader");

    // Does the FINEST maker also happen to be the LARGEST maker? Counted across
    // every good with at least two makers — the headline claim to falsify.
    let (mut goods_measured, mut leader_is_biggest) = (0usize, 0usize);
    let mut spreads: Vec<f32> = Vec::new();

    for g in 0..ng {
        if sim.goods[g].inputs.is_empty() { continue; } // manufactured only
        let mut makers: Vec<(usize, f32, f32)> = Vec::new(); // (hub, quality, population)
        for h in 0..sim.hubs.len() {
            if sim.hubs[h].abandoned { continue; }
            if sim.hubs[h].production.get(g).copied().unwrap_or(0.0) <= 0.0 { continue; }
            let q = sim.hubs[h].quality.get(g).copied().unwrap_or(0.0);
            makers.push((h, q, sim.hubs[h].population));
        }
        if makers.is_empty() { continue; }
        let mut qs: Vec<f32> = makers.iter().map(|m| m.1).collect();
        qs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let (lo, hi) = (qs[0], qs[qs.len() - 1]);
        let med = qs[qs.len() / 2];
        let mult = |q: f32| 0.6 + 0.9 * q.clamp(0.0, 1.0);
        let spread = if mult(lo) > 1e-6 { mult(hi) / mult(lo) } else { 1.0 };

        // SATURATION is the thing the first run actually found, so measure it
        // rather than leaving it to be inferred from "median == max": how many
        // makers sit within a whisker of this good's own best. Where that is
        // most of them there is no "finest maker" at all, only a tie, and the
        // leader/biggest comparison below is reporting an arbitrary tie-break.
        let at_top = makers.iter().filter(|m| m.1 >= hi - 0.005).count();
        let best_q = makers.iter().cloned()
            .max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        let biggest = makers.iter().cloned()
            .max_by(|a, b| a.2.total_cmp(&b.2)).unwrap();
        let same = best_q.0 == biggest.0;
        if makers.len() >= 2 {
            goods_measured += 1;
            if same { leader_is_biggest += 1; }
            spreads.push(spread);
        }
        println!("  {:<18} {:>5} {:>6.3} {:>6.3} {:>6.3} {:>6.2}× {:>5} {}",
                 sim.goods[g].name, makers.len(), lo, med, hi, spread,
                 format!("{}", at_top),
                 if makers.len() < 2 { "sole maker" }
                 else if same { "= BIGGEST CITY" } else { "not the biggest" });
    }

    if goods_measured == 0 {
        println!("\n  NO manufactured good has two or more makers on this world.");
        println!("  That is itself the finding: report it rather than a spread of nothing.");
        return;
    }
    spreads.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean_spread = spreads.iter().sum::<f32>() / spreads.len() as f32;
    println!("\n  goods with >=2 makers            {goods_measured}");
    println!("  finest maker IS the biggest city {leader_is_biggest}  ({:.0}%)",
             100.0 * leader_is_biggest as f32 / goods_measured as f32);
    println!("  price spread best/worst · mean   {mean_spread:.2}×   median {:.2}×   max {:.2}×",
             spreads[spreads.len() / 2], spreads[spreads.len() - 1]);
    println!("\n  '@top' = makers within 0.005 of that good's best. Where it is most of");
    println!("  them, quality has SATURATED at the shared ceiling and 'the finest maker'");
    println!("  is an arbitrary tie-break, not a fact about the world.");
    println!("  → Phase 2.1's target, restated by this measurement: the problem is not");
    println!("    that the biggest city wins, it is that EVERY established maker reaches");
    println!("    the same ceiling, so no city is distinctive at anything.");
}

/// Peak working set of THIS process in MB (cumulative, never decreases). No extra
/// crates: Linux reads `VmHWM`, Windows asks PowerShell about our own PID.
fn peak_rss_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        return s.lines().find(|l| l.starts_with("VmHWM:"))
            .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
            .map_or(f64::NAN, |kb| kb / 1024.0);
    }
    #[cfg(windows)]
    {
        let cmd = format!("(Get-Process -Id {}).PeakWorkingSet64", std::process::id());
        let out = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &cmd]).output();
        return out.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok())
            .map_or(f64::NAN, |b| b / 1_048_576.0);
    }
    #[allow(unreachable_code)]
    f64::NAN
}

/// IMPROVEMENT_PLAN Phase 2 — peak memory and seconds per simulated year on a
/// FULL-SIZE world (3600×1800), well past the old year-30 OOM, through the real
/// commands. Target machine: an 8 GB RAM laptop; budget: 3 GB peak working set
/// (`MEM_BUDGET_MB`) — leaves room for the OS and the WebView. Override the length
/// with `WF2_MEASURE_YEARS` (default 60). Run in release:
/// `cargo test --release --lib measure_full_size_campaign -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn measure_full_size_campaign() {
    const MEM_BUDGET_MB: f64 = 3072.0;
    let years: u32 = std::env::var("WF2_MEASURE_YEARS").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().expect("in-memory WorldDb"));
    let state = app.state::<WorldDb>();
    let t0 = std::time::Instant::now();

    world_commands::new_world("measure".into(), 3600, 1800, state.clone()).expect("new_world");
    let seed = 20261015u64;
    let run = sim_commands::sim_run_all(seed, 10, "plates".into(), 0.5, 0.5, 0.5, 0.5, state.clone())
        .expect("sim_run_all");
    let econ = compute_economy(
        serde_json::to_string(&run.settlements).unwrap(), serde_json::to_string(&run.rivers).unwrap(),
        0, 0.15, false, 6, 0.5, 0.0, -1, 1, state.clone(),
    ).expect("compute_economy");
    let gen_s = t0.elapsed().as_secs_f64();
    let gen_mb = peak_rss_mb();
    println!("[measure] worldgen 3600x1800: {} settlements, {} hubs, {gen_s:.0} s, peak {gen_mb:.0} MB",
        run.settlements.len(), econ.hubs.len());

    finalize_world(state.clone()).expect("finalize_world");
    campaign_start_sim(seed, state.clone()).expect("campaign_start_sim");
    let mut worst_s = 0.0f64;
    let camp0 = std::time::Instant::now();
    for y in 1..=years {
        let ty = std::time::Instant::now();
        campaign_advance(365, state.clone()).await.expect("campaign_advance");
        let s = ty.elapsed().as_secs_f64();
        worst_s = worst_s.max(s);
        if y % 10 == 0 || y == 1 {
            println!("[measure] year {y:>3}: {s:6.1} s/yr, peak {:6.0} MB", peak_rss_mb());
        }
    }
    let peak = peak_rss_mb();
    println!("[measure] {years} years: mean {:.1} s/yr, worst {worst_s:.1} s/yr, PEAK {peak:.0} MB (budget {MEM_BUDGET_MB:.0})",
        camp0.elapsed().as_secs_f64() / years as f64);
    assert!(peak.is_nan() || peak <= MEM_BUDGET_MB, "peak {peak:.0} MB is over the {MEM_BUDGET_MB:.0} MB budget");
}

/// Phase 2 follow-up: WHAT grows with campaign age. Every 5 years prints the serialized
/// size of every top-level `CampaignSim` field, of every per-hub / per-house sub-field
/// summed across all hubs / houses, and the in-memory SQLite size — the biggest growers
/// first. Serialized bytes are a proxy for heap, good enough to rank. Quarter-size world
/// to keep it fast; `WF2_MEASURE_YEARS` (default 30), `WF2_MEASURE_W` (default 1800).
/// `cargo test --release --lib diag_what_grows -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn diag_what_grows() {
    use std::collections::BTreeMap;
    let years: u32 = std::env::var("WF2_MEASURE_YEARS").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    let w: u32 = std::env::var("WF2_MEASURE_W").ok().and_then(|v| v.parse().ok()).unwrap_or(1800);
    let app = tauri::test::mock_app();
    app.manage(WorldDb::in_memory().expect("in-memory WorldDb"));
    let state = app.state::<WorldDb>();
    world_commands::new_world("grow".into(), w, w / 2, state.clone()).expect("new_world");
    let seed = 20261015u64;
    let run = sim_commands::sim_run_all(seed, 10, "plates".into(), 0.5, 0.5, 0.5, 0.5, state.clone()).unwrap();
    compute_economy(serde_json::to_string(&run.settlements).unwrap(), serde_json::to_string(&run.rivers).unwrap(),
        0, 0.15, false, 6, 0.5, 0.0, -1, 1, state.clone()).unwrap();
    finalize_world(state.clone()).unwrap();
    campaign_start_sim(seed, state.clone()).unwrap();

    let snapshot = |state: &tauri::State<'_, WorldDb>| -> BTreeMap<String, usize> {
        let conn = state.conn.lock().unwrap();
        let sim = get_sim(state, &conn).unwrap().unwrap();
        let mut m = BTreeMap::new();
        let v = serde_json::to_value(&*sim).unwrap();
        for (k, fv) in v.as_object().unwrap() {
            m.insert(format!("sim.{k}"), fv.to_string().len());
            if let Some(arr) = fv.as_array() {
                if k == "hubs" || k == "houses" {
                    for item in arr {
                        if let Some(o) = item.as_object() {
                            for (sk, sv) in o {
                                *m.entry(format!("{k}[].{sk}")).or_default() += sv.to_string().len();
                            }
                        }
                    }
                }
            }
        }
        let db: i64 = conn.query_row("SELECT page_count * page_size FROM pragma_page_count, pragma_page_size", [], |r| r.get(0)).unwrap_or(0);
        m.insert("sqlite.in_memory_db".into(), db as usize);
        m.insert("count.hubs".into(), sim.hubs.len());
        m.insert("count.houses".into(), sim.houses.len());
        m
    };

    let mut base = snapshot(&state);
    println!("[grow] start: {} hubs, peak {:.0} MB", base["count.hubs"], peak_rss_mb());
    for y in 1..=years {
        let t = std::time::Instant::now();
        campaign_advance(365, state.clone()).await.unwrap();
        if y % 5 != 0 { continue; }
        let s = t.elapsed().as_secs_f64();
        let now = snapshot(&state);
        let mut growth: Vec<(&String, i64, usize)> = now.iter()
            .map(|(k, &v)| (k, v as i64 - *base.get(k).unwrap_or(&0) as i64, v)).collect();
        growth.sort_by_key(|g| -g.1);
        println!("[grow] year {y}: {s:.1} s/yr, peak {:.0} MB, hubs {} houses {}  — top growers over the last 5 years:",
            peak_rss_mb(), now["count.hubs"], now["count.houses"]);
        for (k, d, v) in growth.iter().take(14) {
            println!("[grow]   {:>+9.2} MB  now {:>8.2} MB  {k}", *d as f64 / 1e6, *v as f64 / 1e6);
        }
        base = now;
    }
}
