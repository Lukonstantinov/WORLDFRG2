//! Province partition — a cost-flood + feature-snap administrative layer between
//! tiles and settlements (the EU4-style political/economic map). Runs AFTER the
//! settlement step (settlements seed the partition) and is a SEPARATE layer.
//!
//! The land is split into provinces whose borders follow VISIBLE natural features:
//! - **coasts / islands** — each land connected-component is partitioned on its own,
//!   so no province spans open sea;
//! - **mountain CRESTS** — the divider is a ridgeline, not an altitude. `compute_ridge`
//!   scores a cell by how far it stands above BOTH sides along some axis, so a sharp
//!   low range divides just as a tall one does, while the interior of a high plateau
//!   (where there is no crest) does not divide at all;
//! - **great rivers divide, small rivers unite** — a navigable/major trunk is expensive
//!   to CROSS, so it becomes a frontier; every lesser river instead *discounts* travel
//!   along its valley, so a province spreads up and down its own river and stops at the
//!   interfluves. Lakes are impassable;
//! - **organic noise** — a small per-edge noise term wobbles borders off any clean
//!   Voronoi/gradient line, and provinces are NOT forced simply-connected, so genuine
//!   enclaves/exclaves survive.
//!
//! ## Why there are two stages
//!
//! A cost-flood alone can only ever *bias* a border toward a feature, never pin it to
//! one: the frontier falls where the two seeds' CUMULATIVE costs tie, so a barrier of
//! penalty `P` merely displaces that tie-line by about `P/2` cells. A river even a few
//! cells off the tie-line is simply crossed. So the flood is used for what it is
//! actually good at — province count, size and topology — and a second stage
//! (`snap_borders_to_features`, a marker-controlled watershed) re-places the border
//! LINES onto the crests and channels themselves.
//!
//! Pure, deterministic, cylindrical (X wraps, Y clamps). No DB, no tile writes — the
//! caller persists the result. See `docs/PROVINCE_SYSTEM_PLAN.md`.

use crate::sim::world_buffer::{WorldBuffer, SEASON_AMP_SCALE};
use crate::sim::rivers::{River, Lake};
use crate::sim::settlements::Settlement;
use crate::sim::{names, cultures};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// One good a province's land can yield, with an environmental-suitability QUALITY
/// (0..1) — never an amount/tonnage. Best-first.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ProvinceGood {
    pub good: u8,
    pub quality: f32,
    /// World rank for this good, 1 = finest land on the map. 0 when unranked
    /// (worlds generated before ranking existed).
    #[serde(default)]
    pub rank: u16,
    /// How many provinces yield this good at all (the "of N" in "#3 of N").
    #[serde(default)]
    pub of: u16,
}

/// What separates two neighbouring provinces where they touch.
pub const BORDER_OPEN: u8 = 0;
pub const BORDER_RIDGE: u8 = 1;
pub const BORDER_RIVER: u8 = 2;
pub const BORDER_LAKE: u8 = 3;

/// One shared frontier: which neighbour, how long, and what natural feature runs
/// along it. Longest frontier first.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ProvinceBorder {
    pub neighbor: u16,
    /// Shared border length in cells (so a 1-cell touch is distinguishable from a
    /// 40-cell frontier).
    pub cells: u32,
    /// `BORDER_*` — the dominant feature along this frontier.
    pub kind: u8,
}

/// A province: a contiguous (mostly) patch of one island's land.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Province {
    pub id: u16,
    pub name: String,              // its OWN generated name (variable length), not the seat's
    pub seat_x: u32,
    pub seat_y: u32,
    pub cells: u32,                // area in cells
    pub area_km2: u32,             // latitude-aware real area
    pub island: u32,               // land connected-component id (coast = hard border)
    pub neighbors: Vec<u16>,
    // ── geography ──
    pub koppen: u8,                // plurality climate
    pub elevation_class: u8,       // 0 lowland · 1 hill · 2 upland
    pub mean_fertility: f32,
    pub coastal: bool,
    // ── economy — WHICH goods + WHAT QUALITY (no amount) ──
    pub goods: Vec<ProvinceGood>,
    // ── people ──
    pub culture: String,           // plurality over the province's cells (campaign may shift it)
    pub rural_pop: u32,            // baseline countryside population
    // ── flavour ──
    pub analog: String,            // "looks most like…" real-world regions
    // ── membership ──
    pub settlements: Vec<String>,  // settlement ids whose cell falls inside (seat first)

    // ─────────────────────────────────────────────────────────────────────────
    // Everything below is appended and serde-defaulted, so worlds saved before
    // these stats existed still load (the panels degrade rather than blank).
    // ─────────────────────────────────────────────────────────────────────────
    /// Climate mix, share of cells per Köppen code, largest first (top 4).
    #[serde(default)] pub koppen_shares: Vec<(u8, f32)>,
    #[serde(default)] pub elev_min_m: i32,
    #[serde(default)] pub elev_mean_m: i32,
    #[serde(default)] pub elev_max_m: i32,
    /// `elev_max_m - elev_min_m` — how broken the country is.
    #[serde(default)] pub relief_m: i32,
    #[serde(default)] pub temp_mean: f32,      // °C
    #[serde(default)] pub precip_mean: f32,    // mm/yr
    #[serde(default)] pub season_amp: f32,     // °C, seasonal half-range
    /// Share of cells in a desert/steppe Köppen class.
    #[serde(default)] pub arid_frac: f32,
    #[serde(default)] pub disease_mean: f32,   // 0..1
    #[serde(default)] pub coast_cells: u32,
    #[serde(default)] pub river_cells: u32,
    #[serde(default)] pub navigable_river: bool,
    /// Lake cells touching the province (its own lakeshore).
    #[serde(default)] pub lake_cells: u32,
    /// Peoples present, share of cells each, plurality first.
    #[serde(default)] pub culture_shares: Vec<(String, f32)>,
    /// Σ per-cell food capacity — the rural carrying capacity this province's land supports.
    #[serde(default)] pub food_capacity: f32,
    /// `food_capacity` expressed as a population ceiling, so the UI can show saturation.
    #[serde(default)] pub rural_cap: u32,
    /// Neighbours with shared length + what feature divides them, longest first.
    #[serde(default)] pub neighbors_detail: Vec<ProvinceBorder>,
    /// **Label anchor** — the province's POLE OF INACCESSIBILITY (the centre of its
    /// largest inscribed circle), not its seat and not its centroid. The seat is a
    /// city and often sits near an edge; a centroid can fall in a NEIGHBOUR when the
    /// province is crescent- or hook-shaped. The pole is always inside the province,
    /// which is what putting a name on the right land requires.
    #[serde(default)] pub label_x: u32,
    #[serde(default)] pub label_y: u32,
    /// Radius of that inscribed circle, in cells — how much room the name has, so the
    /// renderer can size the label to the province instead of to the zoom level.
    #[serde(default)] pub label_r: f32,
}

/// Sea sentinel in the per-cell province-id map.
pub const NO_PROVINCE: u16 = u16::MAX;

// ── Partition tuning ──────────────────────────────────────────────────────────
/// Cost per unit of CREST PROMINENCE. Elevation is normalised by 8848 m, so ~250 m
/// of prominence over one cell (≈0.028) costs ≈7.3 — comparable to a navigable
/// river, and reachable by ranges far below the old 2300 m altitude gate.
const K_RIDGE: f64 = 260.0;
/// Ceiling so one freak cell can't wall off a whole flood front.
const RIDGE_CAP: f64 = 9.0;
/// A weak ABSOLUTE-altitude term above ≈4000 m, so the great massifs still resist
/// bodily. Deliberately far below the old `K_MOUNTAIN = 18`, which made every high
/// plateau uniformly expensive and therefore preferred no border line at all.
const ALT_THRESH: f32 = 0.45;
const K_ALT: f64 = 5.0;
/// Crossing penalties for rivers that DIVIDE.
const RIVER_NAVIGABLE: f32 = 6.0;
const RIVER_MAJOR: f32 = 3.5;
/// Step-cost multiplier along a lesser river — it UNITES, pulling the flood up and
/// down its own valley so the river ends up at the province's spine. Stays positive,
/// so Dijkstra's non-negativity holds.
const RIVER_UNITE: f64 = 0.55;
/// How far from the flood's border the snap stage may move a line, in cells.
const SNAP_R: u32 = 3;
/// Height of the "keep it where it was" ridge the snap raises along the original
/// border, so featureless terrain doesn't let the line wander. Well under a real
/// crest or trunk river, so those win wherever they exist.
const FLAT_ANCHOR: f64 = 0.8;
/// Crest prominence that counts as a ridge frontier when labelling border kinds.
const RIDGE_MIN_BORDER: f32 = 0.010;
/// Belt-value histogram resolution for the robust good-quality statistic.
const GOOD_BINS: usize = 16;

struct HeapItem { cost: f64, cell: u32, owner: u32 }
impl PartialEq for HeapItem { fn eq(&self, o: &Self) -> bool { self.cost == o.cost } }
impl Eq for HeapItem {}
impl Ord for HeapItem {
    // Min-heap on cost (BinaryHeap is a max-heap → invert). The cell-index tie-break
    // is what keeps both floods deterministic when costs are equal.
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
            .then_with(|| self.cell.cmp(&o.cell))
    }
}
impl PartialOrd for HeapItem { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

#[inline]
fn hash2(a: u64, b: u64) -> u64 {
    cultures::hash64(a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
}

/// Per-cell CREST PROMINENCE: how far a cell stands above **both** sides along its
/// best axis. This is the field borders should ride, and it behaves the way absolute
/// elevation does not:
///
/// - a sharp 600 m ridge scores like a sharp 6000 m one (both are crests), so ranges
///   under the old 2300 m gate finally divide;
/// - the interior of a high plateau scores ~0 (nothing stands above its neighbours),
///   so borders stop speckling arbitrarily across a meseta.
///
/// Elevation is box-blurred 3×3 first so a crest resolves as a continuous LINE rather
/// than a scatter of one-cell local maxima.
fn compute_ridge(buf: &WorldBuffer) -> Vec<f32> {
    let w = buf.width;
    let wi = w as i32;
    let hi = buf.height as i32;
    let total = buf.total();

    let mut es = vec![0f32; total];
    for y in 0..hi {
        for x in 0..wi {
            let mut s = 0.0f32;
            let mut n = 0.0f32;
            for dy in -1i32..=1 {
                let yy = y + dy;
                if yy < 0 || yy >= hi { continue; }
                for dx in -1i32..=1 {
                    s += buf.elevation[buf.widx(x + dx, yy)];
                    n += 1.0;
                }
            }
            es[buf.idx(x as u32, y as u32)] = s / n.max(1.0);
        }
    }

    // One half of each axis; the opposite side is the negated offset.
    const AXES: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
    // Prominence is measured over a BASELINE of several cells, not against the
    // immediate neighbours. Two reasons, both of which a ±1 sample gets wrong:
    //   · a broad, smooth range is nearly flat at its own summit, so a ±1 sample
    //     reads almost no prominence exactly where the crest is;
    //   · the 3×3 blur above spreads a narrow ridge across three columns, which
    //     would make a ±1 sample read that ridge as a flat little plateau.
    // Sampling at two radii catches both the sharp ridge and the broad range, and
    // leaves a positive band a couple of cells wide for the border to settle in.
    const RADII: [i32; 2] = [2, 4];
    let mut ridge = vec![0f32; total];
    for y in 0..hi {
        for x in 0..wi {
            let i = buf.idx(x as u32, y as u32);
            if buf.terrain[i] != 1 { continue; }
            let e = es[i];
            let mut best = 0f32;
            for (dx, dy) in AXES {
                for r in RADII {
                    let (ya, yb) = (y + dy * r, y - dy * r);
                    if ya < 0 || ya >= hi || yb < 0 || yb >= hi { continue; }
                    let a = es[buf.widx(x + dx * r, ya)];
                    let b = es[buf.widx(x - dx * r, yb)];
                    let p = (e - a).min(e - b);
                    if p > best { best = p; }
                }
            }
            ridge[i] = best;
        }
    }
    ridge
}

/// Ridge crossing cost at a cell, capped.
#[inline]
fn ridge_cost(ridge: f32) -> f64 { ((ridge as f64) * K_RIDGE).min(RIDGE_CAP) }

/// Partition all land into provinces. `granularity` 0..1: 0 = few large provinces,
/// 1 = many small ones. Returns the province list and a per-cell province-id map
/// (`NO_PROVINCE` on sea), row-major over the full world grid.
pub fn generate_provinces(
    buf: &WorldBuffer,
    rivers: &[River],
    lakes: &[Lake],
    settlements: &[Settlement],
    granularity: f32,
) -> (Vec<Province>, Vec<u16>) {
    let w = buf.width;
    let h = buf.height;
    let wi = w as i32;
    let hi = h as i32;
    let total = buf.total();
    let g = granularity.clamp(0.0, 1.0);

    // Per-cell food (rural carrying capacity) reused from the settlement model.
    let food = crate::sim::settlements::compute_food_capacity(buf, rivers);

    // ── River roles, split by size (see the module header) ──
    //   `river_divide` — navigable/major trunks: expensive to CROSS, so they become
    //                    frontiers. Charged on the crossing, not on the cell (below).
    //   `river_unite`  — every lesser river: cheap to travel ALONG, so a province
    //                    spreads through its own valley and halts at the interfluves.
    let mut river_divide = vec![0.0f32; total];
    let mut river_unite = vec![false; total];
    let mut river_any = vec![false; total];
    let mut navigable_here = vec![false; total];
    for r in rivers {
        let divide = if r.navigable {
            RIVER_NAVIGABLE
        } else if r.major {
            RIVER_MAJOR
        } else {
            0.0
        };
        for &(rx, ry) in &r.points {
            let i = buf.idx(rx.min(w - 1), ry.min(h - 1));
            river_any[i] = true;
            if r.navigable { navigable_here[i] = true; }
            if divide > 0.0 {
                if divide > river_divide[i] { river_divide[i] = divide; }
            } else {
                river_unite[i] = true;
            }
        }
    }
    // A cell that carries a trunk is never also a "unite" valley.
    for i in 0..total { if river_divide[i] > 0.0 { river_unite[i] = false; } }

    // Lake cells are impassable to the flood (a lake is a natural divide).
    let mut is_lake = vec![false; total];
    for lk in lakes {
        for &(lx, ly) in &lk.cells { is_lake[buf.idx(lx.min(w - 1), ly.min(h - 1))] = true; }
    }
    // Cells with a lake on their doorstep — used to label lakeshore frontiers.
    let mut lake_adj = vec![false; total];
    for y in 0..hi {
        for x in 0..wi {
            let i = buf.idx(x as u32, y as u32);
            if is_lake[i] { continue; }
            'adj: for dy in -1i32..=1 {
                let yy = y + dy;
                if yy < 0 || yy >= hi { continue; }
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    if is_lake[buf.widx(x + dx, yy)] { lake_adj[i] = true; break 'adj; }
                }
            }
        }
    }

    // ── The divider field: crest prominence (see `compute_ridge`). ──
    let ridge = compute_ridge(buf);

    // ── Island labelling (land connected-components, 8-neighbour, X-wrap). ──
    let mut island = vec![u32::MAX; total];
    let mut n_islands = 0u32;
    let mut stack: Vec<u32> = Vec::new();
    for start in 0..total {
        if buf.terrain[start] != 1 || island[start] != u32::MAX { continue; }
        let id = n_islands; n_islands += 1;
        island[start] = id;
        stack.push(start as u32);
        while let Some(c) = stack.pop() {
            let cx = (c % w) as i32;
            let cy = (c / w) as i32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let ny = cy + dy;
                    if ny < 0 || ny >= hi { continue; }
                    let ni = buf.widx(cx + dx, ny);
                    if buf.terrain[ni] == 1 && island[ni] == u32::MAX {
                        island[ni] = id;
                        stack.push(ni as u32);
                    }
                }
            }
        }
    }

    // ── Seeds: prominent settlements + a jittered filler scatter, with seed DENSITY
    //    scaled by HABITABILITY — dense (small provinces) in fertile temperate/tropical
    //    land, sparse (big provinces) in the hostile fringe: polar, cold taiga, desert
    //    and high mountain ranges. Granularity g sets the base spacing. ──
    // Granularity → number of province "columns" across the map. Wider range so the
    // size slider spans genuinely large (g→0) to small (g→1) provinces.
    let cols = 18.0 + 92.0 * g;
    let spacing = ((w as f32 / cols).round() as i32).max(4);
    // The finest separation (most habitable land); scales UP to ~3.8× in the least
    // habitable land, so provinces there come out far larger. FLOORED at 10 cells so
    // even the most habitable land / highest granularity never shatters into a speckle
    // of 1-cell provinces (the min province is then ≈100 cells).
    let base_sep = ((spacing as f32) * 0.5).max(10.0);
    let hab_at = |i: usize| -> f32 {
        if buf.habitability.is_empty() { 0.5 } else { buf.habitability[i] as f32 / 255.0 }
    };
    // Local min-separation² at a cell: small where habitable, large where hostile.
    let local_sep2 = |i: usize| -> i64 {
        let hostile = (1.0 - hab_at(i).clamp(0.0, 1.0)).powf(1.4);
        let s = (base_sep * (1.0 + 2.8 * hostile)) as i64;
        (s * s).max(1)
    };
    let too_close = |seeds: &[u32], bx: i32, by: i32, sep2: i64| -> bool {
        for &sc in seeds {
            let sx = (sc % w) as i32; let sy = (sc / w) as i32;
            let mut ddx = (sx - bx).abs(); if ddx > wi / 2 { ddx = wi - ddx; }
            let dd = (ddx as i64) * (ddx as i64) + ((sy - by) as i64) * ((sy - by) as i64);
            if dd < sep2 { return true; }
        }
        false
    };
    let mut seed_cells: Vec<u32> = Vec::new();
    let mut is_seed = vec![false; total];
    // Biggest cities first so the important ones win a seat; nearby smaller towns are
    // absorbed into that province (a metro region = ONE province with several towns).
    let mut sorted_settle: Vec<&Settlement> = settlements.iter().collect();
    sorted_settle.sort_by(|a, b| b.population.cmp(&a.population).then_with(|| a.id.cmp(&b.id)));
    for s in sorted_settle {
        let i = buf.idx(s.x.min(w - 1), s.y.min(h - 1));
        if buf.terrain[i] != 1 || is_lake[i] || is_seed[i] { continue; }
        let bx = (i as u32 % w) as i32; let by = (i as u32 / w) as i32;
        if too_close(&seed_cells, bx, by, local_sep2(i)) { continue; }
        is_seed[i] = true; seed_cells.push(i as u32);
    }
    // Filler seeds on a FINE base grid (finest = the habitable separation). The LOCAL
    // separation rejects most candidates in hostile land → few, large provinces there,
    // while habitable land keeps most → many, small provinces. Jittered off-lattice.
    // The candidate score prefers a LESSER RIVER's valley, so valleys become province
    // CORES and the interfluves become the borders ("rivers unite").
    let bspacing = (base_sep as i32).max(4);
    let jit = (base_sep * 0.42) as i64;
    let win = (bspacing / 3).max(2);
    let mut gy = bspacing / 2;
    while gy < hi {
        let mut gx = 0i32;
        while gx < wi {
            let hb = hash2(gx as u64, (gy as u64) ^ 0x51ED_A5A5);
            let span = (2 * jit + 1) as u64;
            let jx = (hb % span) as i64 - jit;
            let jy = ((hb >> 21) % span) as i64 - jit;
            let cxb = gx + jx as i32;
            let cyb = gy + jy as i32;
            // Most fertile land cell in a small window around the jittered centre.
            let mut best = (u32::MAX, -1.0f32);
            for oy in -win..=win {
                let cy = cyb + oy;
                if cy < 0 || cy >= hi { continue; }
                for ox in -win..=win {
                    let ci = buf.widx(cxb + ox, cy);
                    if buf.terrain[ci] != 1 || is_lake[ci] { continue; }
                    let mut sc = buf.fertility[ci] + food[ci] * 0.01;
                    if river_unite[ci] { sc += 0.35; }   // seat the province in the valley
                    if sc > best.1 { best = (ci as u32, sc); }
                }
            }
            if best.0 != u32::MAX {
                let bi = best.0 as usize;
                let bx = (bi as u32 % w) as i32;
                let by = (bi as u32 / w) as i32;
                if !is_seed[bi] && !too_close(&seed_cells, bx, by, local_sep2(bi)) {
                    is_seed[bi] = true; seed_cells.push(bi as u32);
                }
            }
            gx += bspacing;
        }
        gy += bspacing;
    }
    if seed_cells.is_empty() { return (Vec::new(), vec![NO_PROVINCE; total]); }

    // ── Multi-source cost-flood (Dijkstra) over land. Sets province COUNT, SIZE and
    //    TOPOLOGY; the border LINES are re-placed afterwards by the snap stage. ──
    let mut owner = vec![u32::MAX; total];
    let mut dist = vec![f64::INFINITY; total];
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
    for (oi, &sc) in seed_cells.iter().enumerate() {
        owner[sc as usize] = oi as u32;
        dist[sc as usize] = 0.0;
        heap.push(HeapItem { cost: 0.0, cell: sc, owner: oi as u32 });
    }
    while let Some(HeapItem { cost, cell, owner: ow }) = heap.pop() {
        let ci = cell as usize;
        if cost > dist[ci] { continue; }
        if owner[ci] != ow { continue; }
        let cx = (cell % w) as i32;
        let cy = (cell / w) as i32;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ny = cy + dy;
                if ny < 0 || ny >= hi { continue; }
                let ni = buf.widx(cx + dx, ny);
                if buf.terrain[ni] != 1 || is_lake[ni] { continue; }
                let diagonal = dx != 0 && dy != 0;
                // A river/lake traced by following flow one cell per step is an
                // 8-connected STAIRCASE, and a diagonal step can cut clean between two
                // of its cells without entering either — which is why diagonal rivers
                // used to cost nothing at all. Charge the crossing on the EDGE by
                // inspecting the two corner cells the step passes between.
                let corners = if diagonal {
                    Some((buf.widx(cx + dx, cy), buf.widx(cx, ny)))
                } else { None };
                if let Some((ca, cb)) = corners {
                    if is_lake[ca] && is_lake[cb] { continue; }  // no squeezing past a lake
                }
                let mut step = if diagonal { 1.4142 } else { 1.0 };
                if river_unite[ni] { step *= RIVER_UNITE; }
                // Crest prominence divides; a weak absolute term keeps the great
                // massifs bodily expensive.
                let em = buf.elevation[ni];
                let alt = if em > ALT_THRESH { ((em - ALT_THRESH) as f64) * K_ALT } else { 0.0 };
                let crest = ridge_cost(ridge[ni]);
                let mut cross = river_divide[ni] as f64;
                if let Some((ca, cb)) = corners {
                    let (pa, pb) = (river_divide[ca], river_divide[cb]);
                    if pa > 0.0 && pb > 0.0 { cross += pa.min(pb) as f64; }
                }
                let noise = (hash2(cell as u64, ni as u64) % 1000) as f64 / 1000.0 * 0.35;
                let nc = cost + step + alt + crest + cross + noise;
                if nc < dist[ni] {
                    dist[ni] = nc;
                    owner[ni] = ow;
                    heap.push(HeapItem { cost: nc, cell: ni as u32, owner: ow });
                }
            }
        }
    }

    // ── Any unowned land (tiny islands with no seed): give each unowned land
    //    connected-component its own province. ──
    let mut extra_seed_cell: Vec<u32> = Vec::new();
    for start in 0..total {
        if buf.terrain[start] != 1 || is_lake[start] || owner[start] != u32::MAX { continue; }
        let oi = (seed_cells.len() + extra_seed_cell.len()) as u32;
        extra_seed_cell.push(start as u32);
        owner[start] = oi;
        stack.push(start as u32);
        while let Some(c) = stack.pop() {
            let cx = (c % w) as i32; let cy = (c / w) as i32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let ny = cy + dy;
                    if ny < 0 || ny >= hi { continue; }
                    let ni = buf.widx(cx + dx, ny);
                    if buf.terrain[ni] == 1 && !is_lake[ni] && owner[ni] == u32::MAX {
                        owner[ni] = oi;
                        stack.push(ni as u32);
                    }
                }
            }
        }
    }
    let n_seeds = seed_cells.len() + extra_seed_cell.len();
    let seed_of = |oi: u32| -> u32 {
        let oi = oi as usize;
        if oi < seed_cells.len() { seed_cells[oi] } else { extra_seed_cell[oi - seed_cells.len()] }
    };

    // ── Merge slivers: a province below the area floor is folded into the neighbour
    //    it shares the most border with. One pass (deterministic). ──
    let mut cell_count = vec![0u32; n_seeds];
    for &o in owner.iter() { if o != u32::MAX { cell_count[o as usize] += 1; } }
    // Sliver floor keyed off the FINE (habitable) separation, not the coarse spacing,
    // so the intentionally-small habitable provinces survive; only true slivers merge.
    let min_cells = (((base_sep * base_sep) / 6.0) as u32).max(6);
    let mut remap: Vec<u32> = (0..n_seeds as u32).collect();
    // Provinces below the area floor get folded into the neighbour they share the most
    // border with. PERF: this used to rescan ALL cells once per small province — an
    // O(n_small × total) trap that dominated generation on large worlds (hundreds of
    // slivers × 6.5 M cells). Now it's a SINGLE O(total) pass: for every cell owned by a
    // small province, tally its cross-border neighbours into that province's map.
    let is_small: Vec<bool> = cell_count.iter().map(|&c| c > 0 && c < min_cells).collect();
    let mut shared: Vec<std::collections::HashMap<u32, u32>> =
        (0..n_seeds).map(|_| std::collections::HashMap::new()).collect();
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX || !is_small[o as usize] { continue; }
        let cx = (c as u32 % w) as i32; let cy = (c as u32 / w) as i32;
        for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1)] {
            let ny = cy + dy; if ny < 0 || ny >= hi { continue; }
            let no = owner[buf.widx(cx + dx, ny)];
            if no != u32::MAX && no != o { *shared[o as usize].entry(no).or_insert(0) += 1; }
        }
    }
    for p in 0..n_seeds {
        if !is_small[p] { continue; }
        // DETERMINISM: `HashMap` iteration order is randomised per process, so an
        // un-broken `max_by_key` on the count alone made the same seed produce
        // different partitions across runs whenever two neighbours tied. Break the tie
        // on the neighbour id.
        let best = shared[p].iter()
            .max_by_key(|(&nid, &v)| (v, std::cmp::Reverse(nid)))
            .map(|(&nid, _)| nid);
        if let Some(best) = best { remap[p] = best; }
    }
    // Resolve remap chains, then relabel.
    for c in 0..total {
        if owner[c] == u32::MAX { continue; }
        let mut o = owner[c];
        let mut guard = 0;
        while remap[o as usize] != o && guard < 8 { o = remap[o as usize]; guard += 1; }
        owner[c] = o;
    }

    // ── Compact province ids to 0..n and build per-cell u16 map. ──
    let mut old_to_new = std::collections::HashMap::<u32, u16>::new();
    let mut province_id = vec![NO_PROVINCE; total];
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX { continue; }
        let next = old_to_new.len() as u16;
        let nid = *old_to_new.entry(o).or_insert(next);
        province_id[c] = nid;
    }
    let n = old_to_new.len();
    if n == 0 { return (Vec::new(), province_id); }
    // Map new id → an old owner value (to recover the seed cell for naming).
    let mut new_to_old = vec![0u32; n];
    for (&old, &nid) in old_to_new.iter() { new_to_old[nid as usize] = old; }

    // ── Stage 2: snap the border LINES onto the crests and channels. ──
    snap_borders_to_features(buf, &mut province_id, n, &ridge, &river_divide, &is_lake);

    // ── Label anchors: the pole of inaccessibility per province (see `Province`). ──
    let poles = compute_label_anchors(buf, &province_id, n);

    // ── Aggregate per-province stats. ──
    let ng = buf.goods.len();
    let n_kits = cultures::KITS.len().max(1);
    const N_KOPPEN: usize = 40;
    struct Acc {
        cells: u32, fert: f64, elev: f64, food: f64, coastal: bool,
        elev_min: f32, elev_max: f32,
        temp: f64, precip: f64, season: f64, disease: f64,
        coast_cells: u32, river_cells: u32, navigable: bool, lake_cells: u32,
        koppen: [u32; N_KOPPEN],
        kits: Vec<u32>,
        goods_hist: Vec<u32>,   // ng × GOOD_BINS, flat
        island: u32, area: f64,
    }
    let mut accs: Vec<Acc> = (0..n).map(|_| Acc {
        cells: 0, fert: 0.0, elev: 0.0, food: 0.0, coastal: false,
        elev_min: f32::MAX, elev_max: f32::MIN,
        temp: 0.0, precip: 0.0, season: 0.0, disease: 0.0,
        coast_cells: 0, river_cells: 0, navigable: false, lake_cells: 0,
        koppen: [0u32; N_KOPPEN],
        kits: vec![0u32; n_kits],
        goods_hist: vec![0u32; ng * GOOD_BINS],
        island: 0, area: 0.0,
    }).collect();

    // Pre-fetch the culture map ONCE — `names::resolve_kit` takes a process-global
    // RwLock per call, which would be ruinous across a world-sized scan.
    let cmap = cultures::active();
    let cmap_ref = cmap.as_deref();

    // Per-pair frontier tally: (lo, hi) → [len per BORDER_* kind].
    let mut pair_kinds: std::collections::HashMap<(u16, u16), [u32; 4]> =
        std::collections::HashMap::new();
    let has_temp = !buf.temperature.is_empty();
    let has_precip = !buf.precipitation.is_empty();
    let has_season = !buf.seasonal_amp.is_empty();
    let has_disease = !buf.disease_risk.is_empty();

    for c in 0..total {
        let pid = province_id[c];
        if pid == NO_PROVINCE { continue; }
        let cx = (c as u32 % w) as i32;
        let cy = (c as u32 / w) as i32;
        let a = &mut accs[pid as usize];
        a.cells += 1;
        let e = buf.elevation[c];
        a.fert += buf.fertility[c] as f64;
        a.elev += e as f64;
        if e < a.elev_min { a.elev_min = e; }
        if e > a.elev_max { a.elev_max = e; }
        a.food += food[c] as f64;
        a.island = island[c];
        if has_temp { a.temp += buf.temperature[c] as f64; }
        if has_precip { a.precip += buf.precipitation[c] as f64; }
        if has_season { a.season += buf.seasonal_amp[c] as f64 / SEASON_AMP_SCALE as f64; }
        if has_disease { a.disease += buf.disease_risk[c] as f64 / 255.0; }
        // Latitude-aware cell area (cos(lat)); base cell ≈ (earth circ / width) km wide.
        let latr = (buf.latitude(c as u32 / w) as f64).to_radians();
        let cell_km = 40075.0 / w as f64;
        a.area += cell_km * cell_km * latr.cos().max(0.05);
        if buf.distance_to_ocean[c] < 0.05 { a.coastal = true; a.coast_cells += 1; }
        if river_any[c] { a.river_cells += 1; }
        if navigable_here[c] { a.navigable = true; }
        if lake_adj[c] { a.lake_cells += 1; }
        a.koppen[(buf.koppen[c] as usize).min(N_KOPPEN - 1)] += 1;
        a.kits[names::kit_at_with(cmap_ref, cx as u32, cy as u32, w, h).min(n_kits - 1)] += 1;
        for gd in 0..ng {
            let bin = (buf.goods[gd][c] as usize / (256 / GOOD_BINS)).min(GOOD_BINS - 1);
            a.goods_hist[gd * GOOD_BINS + bin] += 1;
        }
        // Frontier scan (4-dir): shared length + which natural feature runs along it.
        for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1)] {
            let nyy = cy + dy; if nyy < 0 || nyy >= hi { continue; }
            let nc = buf.widx(cx + dx, nyy);
            let np = province_id[nc];
            if np == NO_PROVINCE || np == pid { continue; }
            let kind = if river_divide[c] > 0.0 || river_divide[nc] > 0.0 {
                BORDER_RIVER
            } else if lake_adj[c] || lake_adj[nc] {
                BORDER_LAKE
            } else if ridge[c] > RIDGE_MIN_BORDER || ridge[nc] > RIDGE_MIN_BORDER {
                BORDER_RIDGE
            } else {
                BORDER_OPEN
            };
            let key = if pid < np { (pid, np) } else { (np, pid) };
            pair_kinds.entry(key).or_insert([0u32; 4])[kind as usize] += 1;
        }
    }

    // Frontier tally → per-province neighbour lists (longest first, deterministic).
    let mut borders: Vec<Vec<ProvinceBorder>> = vec![Vec::new(); n];
    for (&(p, q), counts) in pair_kinds.iter() {
        let cells: u32 = counts.iter().sum();
        // Dominant feature; ties resolve toward the more distinctive frontier.
        let kind = (0..4u8)
            .max_by_key(|&k| (counts[k as usize], k))
            .unwrap_or(BORDER_OPEN);
        // Each edge is counted from both sides, so the two directions already agree.
        borders[p as usize].push(ProvinceBorder { neighbor: q, cells, kind });
        borders[q as usize].push(ProvinceBorder { neighbor: p, cells, kind });
    }
    for b in borders.iter_mut() {
        b.sort_by(|x, y| y.cells.cmp(&x.cells).then_with(|| x.neighbor.cmp(&y.neighbor)));
    }

    // Settlements per province (seat = largest population).
    let mut prov_settlements: Vec<Vec<(String, u32)>> = vec![Vec::new(); n];
    for s in settlements {
        let pid = province_id[buf.idx(s.x.min(w - 1), s.y.min(h - 1))];
        if pid != NO_PROVINCE {
            prov_settlements[pid as usize].push((s.id.clone(), s.population));
        }
    }

    let mut provinces: Vec<Province> = Vec::with_capacity(n);
    let mut used_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for pid in 0..n {
        let a = &accs[pid];
        if a.cells == 0 { continue; }
        let cellsf = a.cells as f64;
        let mean_fert = (a.fert / cellsf) as f32;
        let mean_elev = (a.elev / cellsf) as f32;
        let elevation_class: u8 = if mean_elev < 0.30 { 0 } else if mean_elev < 0.55 { 1 } else { 2 };
        // Plurality climate. Fixed-size array → scan order, so no HashMap randomness.
        let koppen = (0..N_KOPPEN)
            .max_by_key(|&k| (a.koppen[k], std::cmp::Reverse(k)))
            .map(|k| k as u8).unwrap_or(0);
        let mut koppen_shares: Vec<(u8, f32)> = (0..N_KOPPEN)
            .filter(|&k| a.koppen[k] > 0)
            .map(|k| (k as u8, a.koppen[k] as f32 / a.cells as f32))
            .collect();
        koppen_shares.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(Ordering::Equal)
            .then_with(|| x.0.cmp(&y.0)));
        koppen_shares.truncate(4);
        // Desert/steppe share (Köppen BW*/BS* occupy codes 4..7).
        let arid_frac = (4..8).map(|k| a.koppen[k]).sum::<u32>() as f32 / a.cells as f32;

        // Seat: largest settlement, else the seed cell.
        let mut towns = prov_settlements[pid].clone();
        towns.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
        let (seat_cell, settlement_ids): (u32, Vec<String>) = if !towns.is_empty() {
            // seat = cell of the largest settlement
            let seat = settlements.iter()
                .find(|s| s.id == towns[0].0)
                .map(|s| buf.idx(s.x.min(w - 1), s.y.min(h - 1)) as u32)
                .unwrap_or_else(|| seed_of(new_to_old[pid]));
            (seat, towns.iter().map(|t| t.0.clone()).collect())
        } else {
            (seed_of(new_to_old[pid]), Vec::new())
        };
        let sx = seat_cell % w;
        let sy = seat_cell / w;

        // Goods quality: the mean of the province's BEST DECILE of cells, not the
        // single best cell — one freak cell used to award five stars for silk across
        // a whole region.
        let mut goods: Vec<ProvinceGood> = (0..ng)
            .map(|gd| {
                let hist = &a.goods_hist[gd * GOOD_BINS..(gd + 1) * GOOD_BINS];
                (gd, top_decile_mean(hist, a.cells))
            })
            .filter(|&(_, q)| q >= 40.0)
            .map(|(gd, q)| ProvinceGood { good: gd as u8, quality: q / 255.0, rank: 0, of: 0 })
            .collect();
        goods.sort_by(|x, y| y.quality.partial_cmp(&x.quality).unwrap_or(Ordering::Equal)
            .then_with(|| x.good.cmp(&y.good)));
        goods.truncate(6);

        // Culture: the PLURALITY over the province's cells (it used to be sampled at
        // the seat cell alone, which mislabels any province straddling a hearth edge).
        let mut culture_shares: Vec<(String, f32)> = (0..n_kits)
            .filter(|&k| a.kits[k] > 0)
            .map(|k| (names::kit_label(k).to_string(), a.kits[k] as f32 / a.cells as f32))
            .collect();
        culture_shares.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(Ordering::Equal)
            .then_with(|| x.0.cmp(&y.0)));
        culture_shares.truncate(4);
        let culture = culture_shares.first().map(|c| c.0.clone())
            .unwrap_or_else(|| names::culture_label(sx, sy, w, h).to_string());

        // Name (its OWN — a province is a land, not a town). The generator is a pure
        // hash of the seat cell, so two provinces CAN collide; a province that shares
        // its name with another does not really have one. Re-draw the loser from a
        // salted position until the map is unique (deterministic: provinces are named
        // in id order). See PROVINCE_SYSTEM_PLAN.md §4.1.
        let (kit, ms) = names::resolve_kit(sx, sy, w, h);
        let bucket = name_length_bucket(sx, sy);
        let mut name = cultures::province_name(kit, ms, sx, sy, bucket);
        if !used_names.insert(name.clone()) {
            for salt in 1u32..40 {
                let h2 = hash2(seat_cell as u64, salt as u64 ^ 0x9A5D_1CE7);
                let jx = (h2 % 4096) as u32;
                let jy = ((h2 >> 20) % 4096) as u32;
                let b2 = ((h2 >> 44) % 4) as u8;
                let cand = cultures::province_name(kit, ms, jx, jy, b2);
                if used_names.insert(cand.clone()) { name = cand; break; }
            }
        }
        let analog = real_world_analog(koppen, elevation_class, a.coastal).to_string();
        let rural_pop = (a.food * 18.0).round().max(0.0) as u32;
        let neighbors_detail = borders[pid].clone();
        let mut neighbors: Vec<u16> = neighbors_detail.iter().map(|b| b.neighbor).collect();
        neighbors.sort_unstable();

        provinces.push(Province {
            id: pid as u16,
            name,
            seat_x: sx, seat_y: sy,
            cells: a.cells,
            area_km2: a.area.round().max(0.0) as u32,
            island: a.island,
            neighbors,
            koppen,
            elevation_class,
            mean_fertility: mean_fert,
            coastal: a.coastal,
            goods,
            culture,
            rural_pop,
            analog,
            settlements: settlement_ids,

            koppen_shares,
            elev_min_m: (a.elev_min.max(0.0) * 8848.0).round() as i32,
            elev_mean_m: (mean_elev * 8848.0).round() as i32,
            elev_max_m: (a.elev_max.max(0.0) * 8848.0).round() as i32,
            relief_m: ((a.elev_max - a.elev_min).max(0.0) * 8848.0).round() as i32,
            temp_mean: (a.temp / cellsf) as f32,
            precip_mean: (a.precip / cellsf) as f32,
            season_amp: (a.season / cellsf) as f32,
            arid_frac,
            disease_mean: (a.disease / cellsf) as f32,
            coast_cells: a.coast_cells,
            river_cells: a.river_cells,
            navigable_river: a.navigable,
            lake_cells: a.lake_cells,
            culture_shares,
            food_capacity: a.food as f32,
            rural_cap: (a.food * 18.0).round().max(0.0) as u32,
            neighbors_detail,
            // Pole of inaccessibility; fall back to the seat if the province somehow
            // yielded no interior cell (a 1-cell island).
            label_x: if poles[pid].2 > 0.0 { poles[pid].0 } else { sx },
            label_y: if poles[pid].2 > 0.0 { poles[pid].1 } else { sy },
            label_r: poles[pid].2.max(0.5),
        });
    }

    rank_goods_worldwide(&mut provinces);
    (provinces, province_id)
}

/// Per-province **pole of inaccessibility**: the cell furthest from any boundary, plus
/// the radius of the inscribed circle it centres. Returned as `(x, y, radius)` indexed
/// by province id.
///
/// The boundary here is everything that ends the province: a different province, the
/// sea, or a lake. Seeding all three means an island province's name lands inland
/// rather than out on a headland, and the radius is real room for text in every
/// direction.
///
/// One multi-source BFS over land — the same shape as the `d_border` sweep in
/// `snap_borders_to_features`, but uncapped and including coast/lake edges. The
/// 8-neighbour BFS measures CHEBYSHEV distance, which favours diagonally-elongated
/// shapes slightly over a true Euclidean transform; for choosing where a label sits
/// that is well within tolerance, and it wraps in X for free through `widx`.
fn compute_label_anchors(buf: &WorldBuffer, province_id: &[u16], n: usize) -> Vec<(u32, u32, f32)> {
    let w = buf.width;
    let hi = buf.height as i32;
    let total = buf.total();
    let mut dist = vec![u32::MAX; total];
    let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();

    for c in 0..total {
        let pid = province_id[c];
        if pid == NO_PROVINCE { continue; }
        let cx = (c as u32 % w) as i32;
        let cy = (c as u32 / w) as i32;
        let mut edge = false;
        'e: for dy in -1i32..=1 {
            let ny = cy + dy;
            // The map's top/bottom are boundaries too — a label shouldn't ride the pole.
            if ny < 0 || ny >= hi { edge = true; break 'e; }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                if province_id[buf.widx(cx + dx, ny)] != pid { edge = true; break 'e; }
            }
        }
        if edge { dist[c] = 0; queue.push_back(c as u32); }
    }

    while let Some(c) = queue.pop_front() {
        let d = dist[c as usize];
        let cx = (c % w) as i32;
        let cy = (c / w) as i32;
        let pid = province_id[c as usize];
        for dy in -1i32..=1 {
            let ny = cy + dy;
            if ny < 0 || ny >= hi { continue; }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ni = buf.widx(cx + dx, ny);
                // Stay inside the province — the distance is to ITS OWN boundary.
                if province_id[ni] != pid { continue; }
                if dist[ni] > d + 1 { dist[ni] = d + 1; queue.push_back(ni as u32); }
            }
        }
    }

    // Furthest cell per province. Ties break on the lowest cell index so the anchor is
    // deterministic (a province is usually widest over a plateau of equal distances).
    let mut best = vec![(0u32, 0u32, 0f32, u32::MAX); n];
    for c in 0..total {
        let pid = province_id[c];
        if pid == NO_PROVINCE { continue; }
        let d = dist[c];
        if d == u32::MAX { continue; }
        let slot = &mut best[pid as usize];
        // `slot.3` is the incumbent distance; u32::MAX marks "nothing chosen yet".
        if slot.3 == u32::MAX || d > slot.3 {
            *slot = ((c as u32) % w, (c as u32) / w, d as f32 + 0.5, d);
        }
    }
    best.into_iter().map(|(x, y, r, _)| (x, y, r)).collect()
}

/// Mean of the best decile of a province's cells for one good, from a coarse
/// histogram. Robust where a plain `max` is not: a single exceptional cell can no
/// longer speak for a whole province, but a good confined to part of the province
/// still registers (unlike a median, which would erase it).
fn top_decile_mean(hist: &[u32], cells: u32) -> f32 {
    if cells == 0 { return 0.0; }
    let target = (((cells as f64) * 0.10).ceil() as u32).max(1);
    let bin_w = 256 / GOOD_BINS;
    let mut taken = 0u32;
    let mut acc = 0f64;
    for b in (0..GOOD_BINS).rev() {
        let c = hist[b];
        if c == 0 { continue; }
        let take = c.min(target - taken);
        let mid = (b * bin_w + bin_w / 2) as f64;
        acc += mid * take as f64;
        taken += take;
        if taken >= target { break; }
    }
    if taken == 0 { 0.0 } else { (acc / taken as f64) as f32 }
}

/// Fill each `ProvinceGood`'s world rank ("#3 of 214 — the finest fleece on the map").
/// Ranked only among provinces that yield the good at all.
fn rank_goods_worldwide(provinces: &mut [Province]) {
    let mut by_good: std::collections::HashMap<u8, Vec<(usize, f32)>> =
        std::collections::HashMap::new();
    for (i, p) in provinces.iter().enumerate() {
        for g in &p.goods { by_good.entry(g.good).or_default().push((i, g.quality)); }
    }
    for (good, mut rows) in by_good {
        // Descending quality; ties broken by province index so the order is stable.
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0)));
        let of = rows.len().min(u16::MAX as usize) as u16;
        for (r, (pi, _)) in rows.into_iter().enumerate() {
            if let Some(g) = provinces[pi].goods.iter_mut().find(|g| g.good == good) {
                g.rank = (r + 1).min(u16::MAX as usize) as u16;
                g.of = of;
            }
        }
    }
}

/// **Stage 2 — snap the borders onto the features.**
///
/// The cost-flood decides which land belongs to which province, but it cannot put a
/// border *on* a ridge or river (see the module header). This is a marker-controlled
/// watershed transform, the standard way to pull region boundaries onto the crests of
/// a relief:
///
/// 1. erode every province by `SNAP_R` — what survives is that province's **marker**;
/// 2. build a relief from the real dividers, `barrier = crest + trunk river`, plus an
///    **anchor** ridge along the flood's own border so featureless terrain keeps the
///    line it already had instead of letting it wander;
/// 3. flood outward from the markers, always taking the LOWEST relief cell next
///    (Meyer's algorithm). Two floods therefore meet on the highest ground between
///    them — the crest, or the middle of the channel.
///
/// Only cells within `SNAP_R` of the original border can be relabelled, so no province
/// can lose its core and no fragment deeper than the snap radius can appear.
fn snap_borders_to_features(
    buf: &WorldBuffer,
    province_id: &mut [u16],
    n: usize,
    ridge: &[f32],
    river_divide: &[f32],
    is_lake: &[bool],
) {
    let w = buf.width;
    let hi = buf.height as i32;
    let total = buf.total();
    let passable = |i: usize| buf.terrain[i] == 1 && !is_lake[i];

    // ── 1. Distance (in cells) from each land cell to the nearest province border.
    //       Multi-source BFS over land only — the coast is a HARD border and must not
    //       move, so only land/land cross-province edges seed it.
    let mut d_border = vec![u32::MAX; total];
    let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
    for c in 0..total {
        if !passable(c) { continue; }
        let pid = province_id[c];
        if pid == NO_PROVINCE { continue; }
        let cx = (c as u32 % w) as i32;
        let cy = (c as u32 / w) as i32;
        let mut on_border = false;
        'b: for dy in -1i32..=1 {
            let ny = cy + dy;
            if ny < 0 || ny >= hi { continue; }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ni = buf.widx(cx + dx, ny);
                if passable(ni) && province_id[ni] != NO_PROVINCE && province_id[ni] != pid {
                    on_border = true; break 'b;
                }
            }
        }
        if on_border { d_border[c] = 0; queue.push_back(c as u32); }
    }
    if queue.is_empty() { return; }   // a single province — nothing to snap
    while let Some(c) = queue.pop_front() {
        let d = d_border[c as usize];
        if d >= SNAP_R { continue; }   // never needed beyond the snap band
        let cx = (c % w) as i32;
        let cy = (c / w) as i32;
        for dy in -1i32..=1 {
            let ny = cy + dy;
            if ny < 0 || ny >= hi { continue; }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ni = buf.widx(cx + dx, ny);
                if !passable(ni) || province_id[ni] == NO_PROVINCE { continue; }
                if d_border[ni] > d + 1 { d_border[ni] = d + 1; queue.push_back(ni as u32); }
            }
        }
    }

    // ── 2. Markers = the eroded cores. Everything inside the band is released.
    let original = province_id.to_vec();
    let mut marker_count = vec![0u32; n];
    let mut rep_cell = vec![u32::MAX; n];
    for c in 0..total {
        let pid = original[c];
        if pid == NO_PROVINCE { continue; }
        let p = pid as usize;
        if rep_cell[p] == u32::MAX { rep_cell[p] = c as u32; }
        if d_border[c] >= SNAP_R { marker_count[p] += 1; } else { province_id[c] = NO_PROVINCE; }
    }
    // A province thinner than 2·SNAP_R erodes to nothing — keep its first cell as a
    // marker so it cannot be dissolved by the snap.
    for p in 0..n {
        if marker_count[p] == 0 && rep_cell[p] != u32::MAX {
            province_id[rep_cell[p] as usize] = p as u16;
        }
    }

    // ── 3. Relief: the real dividers, plus a low anchor ridge on the old border.
    let relief_at = |i: usize| -> f64 {
        let barrier = ridge_cost(ridge[i]) + river_divide[i] as f64;
        let d = d_border[i].min(SNAP_R) as f64;
        let anchor = FLAT_ANCHOR * (1.0 - d / SNAP_R as f64).max(0.0);
        let noise = (hash2(i as u64, 0x5EED_B0DE) % 997) as f64 / 997.0 * 0.02;
        barrier + anchor + noise
    };

    // ── 4. Meyer flooding from the markers — always take the lowest relief next.
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
    for c in 0..total {
        let pid = province_id[c];
        if pid == NO_PROVINCE || !passable(c) { continue; }
        let cx = (c as u32 % w) as i32;
        let cy = (c as u32 / w) as i32;
        let mut touches_free = false;
        'f: for dy in -1i32..=1 {
            let ny = cy + dy;
            if ny < 0 || ny >= hi { continue; }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ni = buf.widx(cx + dx, ny);
                if passable(ni) && province_id[ni] == NO_PROVINCE && original[ni] != NO_PROVINCE {
                    touches_free = true; break 'f;
                }
            }
        }
        if touches_free {
            heap.push(HeapItem { cost: relief_at(c), cell: c as u32, owner: pid as u32 });
        }
    }
    while let Some(HeapItem { cell, owner: ow, .. }) = heap.pop() {
        let cx = (cell % w) as i32;
        let cy = (cell / w) as i32;
        for dy in -1i32..=1 {
            let ny = cy + dy;
            if ny < 0 || ny >= hi { continue; }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ni = buf.widx(cx + dx, ny);
                if !passable(ni) || province_id[ni] != NO_PROVINCE { continue; }
                if original[ni] == NO_PROVINCE { continue; }   // never claim new land
                if dx != 0 && dy != 0 {
                    // The same staircase leak the cost-flood has to pay for: a
                    // diagonal step cuts between two channel/lake cells without
                    // entering either. Here the relief IS the barrier, so letting the
                    // step through would carry the label straight past a trunk river
                    // instead of stopping the two floods on it. Block it outright.
                    let (ca, cb) = (buf.widx(cx + dx, cy), buf.widx(cx, ny));
                    if is_lake[ca] && is_lake[cb] { continue; }
                    if river_divide[ca] > 0.0 && river_divide[cb] > 0.0 { continue; }
                }
                province_id[ni] = ow as u16;
                heap.push(HeapItem { cost: relief_at(ni), cell: ni as u32, owner: ow });
            }
        }
    }

    // ── 5. Anything the flood could not reach (a band cut off by a lake) keeps the
    //       ownership the cost-flood gave it. The partition is never left with holes.
    for c in 0..total {
        if province_id[c] == NO_PROVINCE && original[c] != NO_PROVINCE {
            province_id[c] = original[c];
        }
    }
}

/// Deterministic name-length bucket from the seat cell: 0 very short · 1 short ·
/// 2 medium · 3 long/compound (≈15/35/35/15 split).
fn name_length_bucket(x: u32, y: u32) -> u8 {
    let r = hash2(x as u64, y as u64 ^ 0xA5A5) % 100;
    if r < 15 { 0 } else if r < 50 { 1 } else if r < 85 { 2 } else { 3 }
}

/// Match a province's climate + terrain + coast to a curated list of the real-world
/// regions it most resembles. Deterministic; extend the arms freely.
pub fn real_world_analog(koppen: u8, elev_class: u8, coastal: bool) -> &'static str {
    let upland = elev_class == 2;
    match koppen {
        // Mediterranean (Csa/Csb/Csc)
        8 | 9 | 10 => if coastal {
            "the Provençal coast, Tuscany, the Levant shore, coastal Anatolia and Catalonia"
        } else {
            "the Spanish meseta, inland Anatolia and the hills of Greece"
        },
        // Humid subtropical / oceanic (Cfa/Cfb/Cfc)
        11 | 12 | 13 => if upland {
            "the Appalachians, the Cévennes and the hills of Honshū"
        } else if coastal {
            "the Carolinas, the Rías of Galicia, southern Brazil and coastal Japan"
        } else {
            "the Po valley, the Île-de-France, the English Midlands and the North German plain"
        },
        // Savanna / monsoon (Aw/Am)
        2 | 3 => if coastal {
            "the Coromandel coast, the Guinea shore and coastal Yucatán"
        } else {
            "the Sahel, the Deccan, the East African highlands and the Llanos"
        },
        // Tropical rainforest (Af)
        1 => "the Congo basin, Amazonia, Borneo and the Kerala coast",
        // Steppe (BSh/BSk)
        6 | 7 => if upland {
            "the Iranian plateau, the Anatolian steppe and the Altiplano"
        } else {
            "the Kazakh steppe, the Maghreb high plains, the Great Plains and the Pontic steppe"
        },
        // Desert (BWh/BWk)
        4 | 5 => "the Saharan oases, the Arabian Nejd, the Taklamakan rim and the Atacama",
        // Warm-ish continental (Dfa/Dfb + Ds continental Med)
        14 | 15 | 18 | 19 | 20 => if upland {
            "the Carpathians, the Caucasus foothills and the Appalachians"
        } else {
            "the Ukrainian black-earth, the American Midwest and the Manchurian plain"
        },
        // Cold continental / subarctic (Dfc/Dfd + Dw)
        16 | 17 | 29 | 30 => if coastal {
            "Norway, Newfoundland, the Baltic shore, Hokkaidō and Kamchatka"
        } else {
            "the taiga of Sweden, interior Canada and Siberia"
        },
        // Tundra (ET)
        21 => "Iceland, coastal Greenland, Lapland and the Aleutians",
        // Ice cap (EF)
        22 => "the ice sheets of Greenland and Antarctica",
        _ => if upland {
            "the Alps, the Andes, the Ethiopian highlands and the Harz"
        } else {
            "a temperate, river-fed heartland"
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;
    use crate::sim::world_buffer::ColumnSet;
    use rusqlite::Connection;

    const TW: u32 = 96;
    const TH: u32 = 64;

    fn blank_world() -> WorldBuffer { blank_world_sized(TW, TH) }

    /// A blank all-land WorldBuffer, with uniform fertility and a habitability that
    /// keeps the seed spacing in its normal range. The seed separation has a hard floor
    /// of 10 cells, so a test that needs MANY provinces has to ask for a bigger grid
    /// rather than a higher granularity.
    fn blank_world_sized(gw: u32, gh: u32) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", gw.to_string()), ("grid_height", gh.to_string())] {
            conn.execute("INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v]).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        for i in 0..buf.total() {
            buf.terrain[i] = 1;
            buf.elevation[i] = 0.0;
            buf.fertility[i] = 0.5;
            buf.habitability[i] = 140.0;
            buf.distance_to_ocean[i] = 0.5;
            buf.temperature[i] = 12.0;
            buf.precipitation[i] = 800.0;
            buf.koppen[i] = 12;
        }
        buf
    }

    fn settle(id: &str, x: u32, y: u32, pop: u32) -> Settlement {
        Settlement {
            id: id.to_string(), x, y, name: id.to_string(), size: "town".into(),
            population: pop, score: 1.0, culture: String::new(), region: String::new(),
            site: String::new(),
        }
    }

    /// Rivers carry a dozen serde-defaulted render fields; build them from JSON so a
    /// test only has to state the parts it actually cares about.
    fn river(points: Vec<(u32, u32)>, width: f32, navigable: bool, major: bool) -> River {
        serde_json::from_value(serde_json::json!({
            "points": points, "width": width, "navigable": navigable, "major": major,
        })).unwrap()
    }

    /// Every land cell that touches a DIFFERENT province (4-dir) — the border line.
    fn border_mask(buf: &WorldBuffer, ids: &[u16]) -> Vec<bool> {
        let w = buf.width as i32;
        let h = buf.height as i32;
        let mut m = vec![false; ids.len()];
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x as u32, y as u32);
                if ids[i] == NO_PROVINCE { continue; }
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let ny = y + dy;
                    if ny < 0 || ny >= h { continue; }
                    let ni = buf.widx(x + dx, ny);
                    if ids[ni] != NO_PROVINCE && ids[ni] != ids[i] { m[i] = true; break; }
                }
            }
        }
        m
    }

    /// `compute_ridge` must score a CREST, not an altitude: a sharp low ridge is a
    /// divider, the flat interior of a high plateau is not. This is precisely the
    /// distinction the old `(elev - 0.26) * 18` cost could not make.
    #[test]
    fn ridge_scores_crests_not_altitude() {
        let mut buf = blank_world();
        // A high, wide plateau on the left half (0.60 ≈ 5300 m) and a sharp but LOW
        // ridge line on the right (0.12 ≈ 1060 m, far under the old 2300 m gate).
        for y in 0..TH {
            for x in 0..TW {
                let i = buf.idx(x, y);
                buf.elevation[i] = if x < 30 { 0.60 } else if x == 70 { 0.12 } else { 0.0 };
            }
        }
        let ridge = compute_ridge(&buf);
        let plateau = ridge[buf.idx(15, 32)];
        let crest = ridge[buf.idx(70, 32)];
        assert!(plateau < 0.005, "plateau interior must not read as a divide, got {plateau}");
        assert!(crest > 0.02, "a sharp LOW ridge must read as a divide, got {crest}");
        assert!(crest > plateau * 4.0,
            "the low crest ({crest}) must divide more strongly than 5300 m of flat plateau ({plateau})");
    }

    /// The partition must be a pure function of its inputs. Before the tie-breaks in
    /// the de-sliver and plurality reductions, `HashMap` iteration order made the same
    /// seed produce different partitions across runs whenever two counts tied.
    #[test]
    fn partition_is_deterministic() {
        let mut buf = blank_world();
        // Some relief and variety so ties actually arise.
        for y in 0..TH {
            for x in 0..TW {
                let i = buf.idx(x, y);
                buf.elevation[i] = (((x * 7 + y * 13) % 11) as f32) / 60.0;
                buf.fertility[i] = (((x * 3 + y * 5) % 9) as f32) / 9.0;
                buf.koppen[i] = ((x / 8 + y / 8) % 4 + 11) as u8;
            }
        }
        let towns = vec![settle("a", 12, 12, 9000), settle("b", 60, 40, 7000),
                         settle("c", 80, 18, 5000)];
        let rivers = vec![river((0..TH).map(|y| (48u32, y)).collect(), 3.0, true, true)];
        let (p1, r1) = generate_provinces(&buf, &rivers, &[], &towns, 0.5);
        let (p2, r2) = generate_provinces(&buf, &rivers, &[], &towns, 0.5);
        assert_eq!(r1, r2, "per-cell province map must be identical across runs");
        assert_eq!(p1.len(), p2.len());
        for (a, b) in p1.iter().zip(p2.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.cells, b.cells);
            assert_eq!(a.culture, b.culture);
            assert_eq!(a.koppen, b.koppen);
            assert_eq!(a.neighbors, b.neighbors);
        }
    }

    /// Borders must actually RIDE the crests. A cost-flood alone can only lean toward
    /// a barrier; the marker-controlled watershed snap is what puts the line on it.
    #[test]
    fn borders_ride_mountain_crests() {
        let mut buf = blank_world();
        // Parallel low ridges every 16 cells, all peaking at 0.11 (≈970 m) — under the
        // old 2300 m threshold, so the previous cost model ignored them entirely.
        for y in 0..TH {
            for x in 0..TW {
                let i = buf.idx(x, y);
                let d = (x % 16) as i32 - 8;
                buf.elevation[i] = if d == 0 { 0.11 } else if d.abs() == 1 { 0.05 } else { 0.0 };
            }
        }
        let towns = vec![settle("a", 4, 10, 9000), settle("b", 52, 40, 6000)];
        let (_, ids) = generate_provinces(&buf, &[], &[], &towns, 0.5);
        let ridge = compute_ridge(&buf);
        let mask = border_mask(&buf, &ids);

        let mut on_crest = 0u32;   // border cells sitting on a crest line
        let mut borders = 0u32;
        let mut crest_cells = 0u32;
        let mut land = 0u32;
        for i in 0..buf.total() {
            if ids[i] == NO_PROVINCE { continue; }
            land += 1;
            let is_crest = ridge[i] > 0.01;
            if is_crest { crest_cells += 1; }
            if mask[i] { borders += 1; if is_crest { on_crest += 1; } }
        }
        assert!(borders > 50, "expected a real border network, got {borders} cells");
        let border_share = on_crest as f32 / borders as f32;
        let land_share = crest_cells as f32 / land as f32;
        eprintln!("crest-riding: {:.1}% of border cells on a crest vs {:.1}% of land \
                   ({:.1}× lift)", border_share * 100.0, land_share * 100.0,
                   border_share / land_share.max(1e-6));
        assert!(border_share > land_share * 2.0,
            "borders must prefer crests: {:.1}% of border cells are on a crest vs \
             {:.1}% of land overall", border_share * 100.0, land_share * 100.0);
    }

    /// A DIAGONAL navigable river used to cost exactly nothing to cross: the channel
    /// is an 8-connected staircase and a diagonal step cuts clean between two of its
    /// cells without entering either. Charging the crossing on the EDGE closes that,
    /// so a diagonal trunk now attracts borders like any other.
    #[test]
    fn diagonal_trunk_rivers_attract_borders() {
        let buf = blank_world();
        // A pure diagonal trunk: (x, x) for x in 0..TH — the exact case that leaked.
        let pts: Vec<(u32, u32)> = (0..TH).map(|k| (k, k)).collect();
        let towns = vec![settle("a", 10, 40, 9000), settle("b", 60, 20, 6000)];
        let rivers = vec![river(pts.clone(), 4.0, true, true)];
        let (_, ids) = generate_provinces(&buf, &rivers, &[], &towns, 0.4);
        let mask = border_mask(&buf, &ids);

        let mut on_river = 0u32;
        let mut river_cells = 0u32;
        for &(x, y) in &pts {
            let i = buf.idx(x, y);
            if ids[i] == NO_PROVINCE { continue; }
            river_cells += 1;
            if mask[i] { on_river += 1; }
        }
        let (mut borders, mut land) = (0u32, 0u32);
        for i in 0..buf.total() {
            if ids[i] == NO_PROVINCE { continue; }
            land += 1;
            if mask[i] { borders += 1; }
        }
        assert!(river_cells > 40, "river should lie on land");
        let river_rate = on_river as f32 / river_cells as f32;
        let land_rate = borders as f32 / land as f32;
        eprintln!("diagonal trunk: {:.1}% of its cells are a province border vs {:.1}% \
                   of land ({:.1}× lift)", river_rate * 100.0, land_rate * 100.0,
                   river_rate / land_rate.max(1e-6));
        assert!(river_rate > land_rate * 2.0,
            "a diagonal trunk must attract borders: {:.1}% of its cells are on a \
             province border vs {:.1}% of land overall", river_rate * 100.0, land_rate * 100.0);
    }

    /// A province's label anchor must land INSIDE that province — the whole point of
    /// using the pole of inaccessibility rather than a centroid (which falls in a
    /// neighbour whenever the province is crescent- or hook-shaped) or the seat (a
    /// city, frequently near an edge). Its radius must also track the province's size,
    /// so the renderer can scale the name to the land.
    #[test]
    fn label_anchor_is_inside_its_own_province_and_scales() {
        let mut buf = blank_world();
        for y in 0..TH {
            for x in 0..TW {
                let i = buf.idx(x, y);
                if x < 3 || x > TW - 4 { buf.terrain[i] = 0; }
                buf.elevation[i] = (((x * 5 + y * 3) % 13) as f32) / 50.0;
            }
        }
        let towns = vec![settle("a", 20, 20, 9000), settle("b", 60, 44, 4000)];
        let (provs, ids) = generate_provinces(&buf, &[], &[], &towns, 0.35);
        assert!(provs.len() > 3, "need several provinces to compare, got {}", provs.len());

        for p in &provs {
            let i = buf.idx(p.label_x.min(TW - 1), p.label_y.min(TH - 1));
            assert_eq!(ids[i], p.id,
                "province {} '{}' anchors its label on province {} — wrong land",
                p.id, p.name, ids[i]);
            assert!(p.label_r > 0.0, "province {} has no inscribed radius", p.id);
        }

        // Bigger province ⇒ more room for its name. Compare the extremes rather than
        // every pair: a long thin province can legitimately hold a small circle.
        let mut by_area = provs.iter().collect::<Vec<_>>();
        by_area.sort_by_key(|p| p.cells);
        let (small, big) = (by_area[0], by_area[by_area.len() - 1]);
        assert!(big.label_r > small.label_r,
            "the largest province ({} cells, r={}) should have more label room than \
             the smallest ({} cells, r={})", big.cells, big.label_r, small.cells, small.label_r);
    }

    /// A province that shares its name with another does not really have its own name.
    #[test]
    fn province_names_are_unique() {
        // A big grid, because the seed separation floors at 10 cells — a crowded map is
        // where the name hash actually collides.
        let (gw, gh) = (220u32, 150u32);
        let mut buf = blank_world_sized(gw, gh);
        for y in 0..gh {
            for x in 0..gw {
                let i = buf.idx(x, y);
                buf.elevation[i] = (((x * 7 + y * 13) % 11) as f32) / 60.0;
            }
        }
        let towns = vec![settle("a", 12, 12, 9000), settle("b", 160, 90, 7000)];
        let (provs, _) = generate_provinces(&buf, &[], &[], &towns, 1.0);
        assert!(provs.len() > 20, "need a crowded map to exercise collisions, got {}", provs.len());
        let mut seen = std::collections::HashSet::new();
        for p in &provs {
            assert!(!p.name.is_empty(), "province {} has no name", p.id);
            assert!(seen.insert(p.name.clone()),
                "duplicate province name '{}' (province {})", p.name, p.id);
        }

        // Prove the pass is not vacuous: recompute what the RAW generator would have
        // produced for each seat and count how many names it would have doubled up.
        let mut raw = std::collections::HashSet::new();
        let mut collisions = 0;
        for p in &provs {
            let (kit, ms) = names::resolve_kit(p.seat_x, p.seat_y, gw, gh);
            let bucket = name_length_bucket(p.seat_x, p.seat_y);
            if !raw.insert(cultures::province_name(kit, ms, p.seat_x, p.seat_y, bucket)) {
                collisions += 1;
            }
        }
        eprintln!("names: {} provinces · {} raw collisions resolved by the salt pass",
                  provs.len(), collisions);
        assert!(collisions > 0,
            "this map produced no raw name collisions, so it does not exercise the \
             uniqueness pass — make the test map denser");
    }

    /// Every land cell must end up owned, and the new stats must be self-consistent —
    /// the snap stage must never punch holes in the partition.
    #[test]
    fn partition_covers_all_land_and_stats_are_sane() {
        let mut buf = blank_world();
        for y in 0..TH {
            for x in 0..TW {
                let i = buf.idx(x, y);
                // Carve an ocean margin so coasts and islands are exercised too.
                if x < 3 || x > TW - 4 { buf.terrain[i] = 0; }
                buf.elevation[i] = (((x * 5 + y * 3) % 13) as f32) / 50.0;
            }
        }
        let towns = vec![settle("a", 20, 20, 9000), settle("b", 60, 44, 4000)];
        let (provs, ids) = generate_provinces(&buf, &[], &[], &towns, 0.5);
        assert!(!provs.is_empty());
        let mut counted = vec![0u32; provs.len()];
        for i in 0..buf.total() {
            if buf.terrain[i] == 1 {
                assert_ne!(ids[i], NO_PROVINCE, "every land cell must belong to a province");
                counted[ids[i] as usize] += 1;
            } else {
                assert_eq!(ids[i], NO_PROVINCE, "sea must stay unowned");
            }
        }
        for p in &provs {
            assert_eq!(p.cells, counted[p.id as usize], "cell count must match the raster");
            assert!(p.elev_max_m >= p.elev_min_m);
            assert_eq!(p.relief_m, p.elev_max_m - p.elev_min_m);
            assert!(p.neighbors.len() == p.neighbors_detail.len());
            let share: f32 = p.culture_shares.iter().map(|c| c.1).sum();
            assert!(share > 0.99 || p.culture_shares.len() == 4,
                "culture shares must cover the province (got {share})");
            assert!(!p.culture.is_empty());
            for g in &p.goods {
                assert!(g.rank >= 1 && g.rank <= g.of, "good rank {} of {}", g.rank, g.of);
            }
        }
    }
}
