//! Trade-good localities — the agricultural/biological counterpart of
//! `deposits.rs`. See `docs/GOODS_LOCALITIES_PLAN.md`.
//!
//! # Why this module exists
//!
//! `deposits.rs` (§8.16) placed ore by real geology with a three-level hierarchy —
//! metallogenic belt → ore district → working — and per-working state. Everything
//! else `localize_good` places (a wine belt, a grain region, a wool country) is
//! still one smooth wash: quality inside a belt is nothing but the raw climate
//! score, which varies over hundreds of km with no internal structure at all
//! (F1). This module gives every non-deposit good the same TWO extra levels the
//! minerals already have — locality (a terroir patch) inside the belt (the
//! climate envelope) — without touching the belt's own placement (Slices 1-2 run
//! first and change WHERE a belt sits; this module only changes its texture).
//!
//! # The size ladder (§2.1)
//!
//! A cell is ~11 km at 3600×1800 (5.6 km on Large), so every size below is stated
//! in km and converted per world, never in cells. Three tiers, all far larger than
//! an ore district's 45 km (D6 — the chernozem case: a grain region is not a farm):
//!
//! | Tier | Span | Goods |
//! |---|---|---|
//! | Luxury locality | 175 km | wine, silk, spices, cacao, cloves, pepper |
//! | Pastoral / secondary | 400 km | wool, hides, horses, timber, tobacco |
//! | Staple region | 900 km | grain, rice, furs, barley, millet |
//!
//! Every other enabled, non-`Deposits`, non-`Manufactured` good (including every
//! custom good) falls back to a tier picked from its own `Distribution`/rarity —
//! see `locality_radius_km` — so no good silently gets zero localities.
//!
//! # Full modulation, with a floor (D5, §5.1)
//!
//! ```text
//! belt[i] = max(FLOOR, belt[i] * (FRINGE + (1 - FRINGE) * locality_influence[i]))
//! ```
//!
//! `FLOOR` is the entire safety mechanism: a cell already in the belt never falls
//! to literal zero, so a good's last producer near some port cannot be thinned
//! away by this pass (the maintainer's own standing constraint — a coverage loss
//! here is a revert or a retune, never a judgement call, §2.4). `FRINGE` is the
//! baseline a cell keeps even at zero locality influence; both are small, tuned
//! constants, not per-good knobs.
//!
//! Minerals are deliberately OUT OF SCOPE here — `Distribution::Deposits` goods
//! already have their own, better hierarchy (grade/extent/depth per working) and
//! are skipped entirely (F2's whole premise: minerals already have what goods
//! lack). `Distribution::Manufactured` goods have no belt to cluster.

use serde::{Deserialize, Serialize};

use crate::sim::world_buffer::WorldBuffer;
use super::biological::RiverContext;
use super::deposits::hash01;
use super::goods_spec::{Distribution, GoodSpec};

/// §2.1 tiers, in km.
pub const TIER_LUXURY_KM: f32 = 175.0;
pub const TIER_PASTORAL_KM: f32 = 400.0;
pub const TIER_STAPLE_KM: f32 = 900.0;

const KM_EQUATOR: f32 = 40075.0;
/// The baseline a belt cell keeps even at zero locality influence (D5).
const FRINGE: f32 = 0.32;
/// The ABSOLUTE floor (0..1 belt value) — the entire safety mechanism for D5. A
/// cell already in the belt never falls below this, however far it sits from
/// every locality core.
const FLOOR: f32 = 0.10;
/// Quality threshold above which a locality is "notable" enough to draw a name
/// (D8) — tuned so a typical world yields tens, not thousands.
const NOTABLE_GRADE_THRESHOLD: f32 = 0.72;
/// Hard cap on localities placed per good, so a huge Global belt (wheat can cover
/// a third of a continent) still costs a bounded number of bounded-radius scans.
const MAX_LOCALITIES: u32 = 40;

/// One locality: a terroir patch inside a good's belt. Deliberately the same
/// shape as `deposits::Deposit` so a future province-attribution pass can reuse
/// the same raster-lookup code rather than inventing a second one.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GoodLocality {
    pub good: String,
    pub x: u32,
    pub y: u32,
    pub radius_km: f32,
    /// 0..1 mean quality of the patch.
    pub grade: f32,
    /// Cells the patch covers, binned 0 (a pocket) .. 3 (a great homeland).
    pub extent: u8,
    /// Empty unless notable (D8) — filled by `name_notable_localities`.
    pub name: String,
    /// This patch owes its grade to a nearby river (Slice 1 factors).
    pub river_fed: bool,
}

/// §2.1's tier for a good, from its id where the plan names one explicitly, else
/// a sensible fallback from its own `Distribution`/rarity so every good (built-in
/// or custom) gets a real answer.
fn locality_radius_km(spec: &GoodSpec) -> f32 {
    match spec.id.as_str() {
        "wine" | "silk" | "spices" | "cacao" | "cloves" | "pepper" => TIER_LUXURY_KM,
        "wool_fleece" | "wool_llama" | "hides" | "horses" | "timber" | "tobacco" => TIER_PASTORAL_KM,
        "wheat" | "rice" | "furs" | "barley" | "millet" => TIER_STAPLE_KM,
        _ => {
            if matches!(spec.distribution, Distribution::Global) {
                TIER_STAPLE_KM
            } else if spec.network_luxury || spec.rarity > 0.65 {
                TIER_LUXURY_KM
            } else {
                TIER_PASTORAL_KM
            }
        }
    }
}

/// Place localities inside an already-placed belt for one good, modulating the
/// belt in place (full modulation, D5) and returning the discrete localities.
/// `belt` is the u8 column just produced by `localize_good`, BEFORE dilation —
/// dilation should spread the already-thinned belt, not the raw one.
pub fn place_localities(
    buf: &WorldBuffer, belt: &mut [u8], spec: &GoodSpec, rc: Option<&RiverContext>, seed: u64,
) -> Vec<GoodLocality> {
    if !matches!(spec.distribution, Distribution::Deposits | Distribution::Manufactured) {
        return place_localities_inner(buf, belt, spec, rc, seed);
    }
    Vec::new()
}

fn place_localities_inner(
    buf: &WorldBuffer, belt: &mut [u8], spec: &GoodSpec, rc: Option<&RiverContext>, seed: u64,
) -> Vec<GoodLocality> {
    let w = buf.width;
    let h = buf.height;
    let n = belt.len();
    if n == 0 { return Vec::new(); }
    let km_per_cell = KM_EQUATOR / w.max(1) as f32;
    let radius_km = locality_radius_km(spec);
    let radius_cells = (radius_km / km_per_cell).max(1.5);

    let cells: Vec<usize> = (0..n).filter(|&i| belt[i] > 0).collect();
    if cells.is_empty() { return Vec::new(); }
    let area = cells.len() as f32;
    let n_loc = ((area / (std::f32::consts::PI * radius_cells * radius_cells)).ceil() as u32)
        .clamp(1, MAX_LOCALITIES);

    let gs = seed ^ crate::sim::goods_spec::id_salt(&spec.id) ^ 0x1004_A117_7000_0001;
    let mut ranked: Vec<(usize, f32)> = cells.iter().map(|&i| {
        let v = belt[i] as f32 / 255.0;
        let key = (v + 0.05) * hash01(gs ^ (i as u64).wrapping_mul(0x100000001B3));
        (i, key)
    }).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.cmp(&b.0)));

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > w as i32 / 2 { d = w as i32 - d; }
        d
    };
    let mut centers: Vec<(i32, i32, f32)> = Vec::new();
    for &(i, _) in &ranked {
        if centers.len() as u32 >= n_loc { break; }
        let cx = (i as u32 % w) as i32;
        let cy = (i as u32 / w) as i32;
        let far_enough = centers.iter().all(|&(qx, qy, _)| {
            let dx = wrap_dx(cx, qx) as f32;
            let dy = (cy - qy) as f32;
            (dx * dx + dy * dy).sqrt() >= radius_cells * 0.9
        });
        if !far_enough { continue; }
        centers.push((cx, cy, belt[i] as f32 / 255.0));
    }
    if centers.is_empty() { return Vec::new(); }

    // ── Full modulation (D5): a locality-influence field, then thin the belt. ──
    let mut influence = vec![0.0f32; n];
    let extent_r = (radius_cells * 1.6).ceil() as i32;
    for &(cx, cy, _) in &centers {
        for dy in -extent_r..=extent_r {
            let wy = cy + dy;
            if wy < 0 || wy >= h as i32 { continue; }
            for dx in -extent_r..=extent_r {
                let d2 = (dx * dx + dy * dy) as f32;
                let reach = radius_cells * 1.6;
                if d2 > reach * reach { continue; }
                let wx = buf.wrap_x(cx + dx);
                let idx = buf.idx(wx, wy as u32);
                if belt[idx] == 0 { continue; }
                let t = (d2.sqrt() / radius_cells.max(1.0)).min(1.6) / 1.6;
                let fall = (1.0 - t).clamp(0.0, 1.0).powf(1.4);
                if fall > influence[idx] { influence[idx] = fall; }
            }
        }
    }
    for i in 0..n {
        if belt[i] == 0 { continue; }
        let b0 = belt[i] as f32 / 255.0;
        let modulated = (b0 * (FRINGE + (1.0 - FRINGE) * influence[i])).max(FLOOR.min(b0));
        belt[i] = (modulated.clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    // ── Locality records ──
    let mut out = Vec::with_capacity(centers.len());
    let r_i = radius_cells.round() as i32;
    let core_area = std::f32::consts::PI * radius_cells * radius_cells;
    for (k, &(cx, cy, cval)) in centers.iter().enumerate() {
        let cyu = cy.clamp(0, h as i32 - 1) as u32;
        let cxu = buf.wrap_x(cx);
        let ci = buf.idx(cxu, cyu);
        let ks = gs ^ (k as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut covered = 0u32;
        for dy in -r_i..=r_i {
            let wy = cy + dy;
            if wy < 0 || wy >= h as i32 { continue; }
            for dx in -r_i..=r_i {
                if (dx * dx + dy * dy) as f32 > radius_cells * radius_cells { continue; }
                let wx = buf.wrap_x(cx + dx);
                if belt[buf.idx(wx, wy as u32)] > 0 { covered += 1; }
            }
        }
        let frac = covered as f32 / core_area.max(1.0);
        let extent = if frac > 0.70 { 3 } else if frac > 0.40 { 2 } else if covered > 3 { 1 } else { 0 };
        let jitter = hash01(ks ^ 0x51) * 0.22 - 0.11;
        let grade = (cval * 0.75 + 0.20 + jitter).clamp(0.05, 1.0);
        let river_fed = rc.map(|r| {
            r.floodplain.get(ci).copied().unwrap_or(0) == 1
                || (r.dist_any.get(ci).copied().unwrap_or(u16::MAX) as f32) <= 3.0
        }).unwrap_or(false);
        out.push(GoodLocality {
            good: spec.id.clone(),
            x: cxu,
            y: cyu,
            radius_km,
            grade,
            extent,
            name: String::new(),
            river_fed,
        });
    }
    out
}

/// GOODS_LOCALITIES_PLAN.md Slice 4 (D8) — name every locality whose grade clears
/// the notable threshold, deterministically, in the LOCAL culture (the same
/// per-cell hearth lookup `names::gen_name` already uses for settlements). Must
/// run after Phase 7's culture map is active (`sim_run_all`'s own ordering already
/// puts settlements — and so the culture map — before Phase 8).
pub fn name_notable_localities(buf: &WorldBuffer, localities: &mut [GoodLocality]) {
    for loc in localities.iter_mut() {
        if loc.grade >= NOTABLE_GRADE_THRESHOLD {
            loc.name = crate::sim::names::gen_name(loc.x, loc.y, buf.width, buf.height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
    use crate::sim::goods_spec::{Domain, MarineBand};

    fn wheat_spec() -> GoodSpec {
        GoodSpec {
            id: "wheat".into(), name: "Grain".into(), icon: "".into(), color: "#fff".into(),
            enabled: true, domain: Domain::Continental, distribution: Distribution::Global,
            rarity: 0.5, desire: 0.85, network_luxury: false, builtin: true,
            deposit: None, scoring: None, marine_band: MarineBand::Either, origins: 1, soil: Vec::new(), relief: None,
            category: "cereal".into(), need_tier: 0, base_value: 1.0, bulk: 3.0, perishable: 0.02,
            inputs: Vec::new(), labor: 1.0, consumption_interval: 7.0,
        }
    }

    fn test_buf(w: u32, h: u32) -> WorldBuffer {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute("INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)", rusqlite::params![k, v]).unwrap();
        }
        WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap()
    }

    /// A big, uniform belt (a whole continent at full value) must still fracture
    /// into several localities rather than one, and the thinned-between-cores
    /// cells must never drop to literal zero (D5's floor).
    #[test]
    fn a_large_belt_fractures_into_several_localities_with_a_floor() {
        let w = 360u32; let h = 180u32;
        let buf = test_buf(w, h);
        let n = (w * h) as usize;
        let mut belt = vec![0u8; n];
        // A big rectangular "continent" of belt, uniform value.
        for y in 40..140u32 {
            for x in 60..300u32 {
                belt[(y * w + x) as usize] = 220;
            }
        }
        let spec = wheat_spec();
        let localities = place_localities(&buf, &mut belt, &spec, None, 12345);
        assert!(localities.len() >= 3, "a continent-sized belt should hold several localities, got {}", localities.len());
        // Every originally-belted cell must keep SOME production (the floor).
        for y in 40..140u32 {
            for x in 60..300u32 {
                let v = belt[(y * w + x) as usize];
                assert!(v > 0, "belt cell ({x},{y}) fell to zero — the D5 floor was violated");
            }
        }
        // The modulated field must have real texture — not every cell can be at the
        // original ceiling, or nothing actually thinned.
        let at_ceiling = (40..140u32).flat_map(|y| (60..300u32).map(move |x| (x, y)))
            .filter(|&(x, y)| belt[(y * w + x) as usize] >= 218).count();
        let total = 100 * 240;
        assert!(at_ceiling < total, "belt shows no modulation at all");
    }

    /// Deposits and Manufactured goods are out of scope (F2 / no belt to cluster).
    #[test]
    fn deposit_and_manufactured_goods_get_no_localities() {
        let w = 64u32; let h = 32u32;
        let buf = test_buf(w, h);
        let n = (w * h) as usize;
        let mut belt = vec![200u8; n];
        let mut spec = wheat_spec();
        spec.distribution = Distribution::Deposits;
        assert!(place_localities(&buf, &mut belt, &spec, None, 1).is_empty());
        spec.distribution = Distribution::Manufactured;
        assert!(place_localities(&buf, &mut belt, &spec, None, 1).is_empty());
    }

    /// Naming only touches localities at/above the notable threshold, and is
    /// deterministic for a fixed seed/world.
    #[test]
    fn only_notable_localities_are_named_and_naming_is_deterministic() {
        let w = 240u32; let h = 120u32;
        let buf = test_buf(w, h);
        let mut locs = vec![
            GoodLocality { good: "wine".into(), x: 10, y: 10, radius_km: 175.0, grade: 0.90, extent: 2, name: String::new(), river_fed: false },
            GoodLocality { good: "wine".into(), x: 20, y: 20, radius_km: 175.0, grade: 0.30, extent: 1, name: String::new(), river_fed: false },
        ];
        name_notable_localities(&buf, &mut locs);
        assert!(!locs[0].name.is_empty(), "a high-grade locality should be named");
        assert!(locs[1].name.is_empty(), "a low-grade locality should stay unnamed");
        let mut locs2 = vec![
            GoodLocality { good: "wine".into(), x: 10, y: 10, radius_km: 175.0, grade: 0.90, extent: 2, name: String::new(), river_fed: false },
        ];
        name_notable_localities(&buf, &mut locs2);
        assert_eq!(locs[0].name, locs2[0].name, "naming must be deterministic for the same cell");
    }
}
