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
    pub neighbor: u32,
    /// Shared border length in cells (so a 1-cell touch is distinguishable from a
    /// 40-cell frontier).
    pub cells: u32,
    /// `BORDER_*` — the dominant feature along this frontier.
    pub kind: u8,
}

/// A province: a contiguous (mostly) patch of one island's land.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Province {
    pub id: u32,
    pub name: String,              // its OWN generated name (variable length), not the seat's
    pub seat_x: u32,
    pub seat_y: u32,
    pub cells: u32,                // area in cells
    pub area_km2: u32,             // latitude-aware real area
    pub island: u32,               // land connected-component id (coast = hard border)
    pub neighbors: Vec<u32>,
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

/// Sea sentinel in the per-cell province-id map. u32 (not u16) so a world can hold
/// more than 65 534 provinces without an id colliding with the sentinel — the cause of
/// whole regions rendering as unowned "green" land on very fine / very large worlds.
pub const NO_PROVINCE: u32 = u32::MAX;

/// Old worlds stored the sea sentinel as `u16::MAX` (65535); new ones use `u32::MAX`.
/// A raster that never mentions `u32::MAX` is the old format — remap its 65535 sentinel
/// so callers (and the frontend) always see one `NO_PROVINCE` value. A world with
/// >65534 provinces is only ever the NEW format, so a genuine id 65535 is never
/// mistaken for sea. Call on any raster read back from a stored `.worldforge`.
pub fn migrate_raster_sentinel(vals: &mut [u32]) {
    if vals.iter().any(|&v| v == NO_PROVINCE) { return; }
    for v in vals.iter_mut() { if *v == 65535 { *v = NO_PROVINCE; } }
}

/// Same, for the RLE `[val, count, val, count, …]` list — only the even (value) slots
/// are ids, so a run length that happens to equal 65535 is left alone.
pub fn migrate_rle_sentinel(rle: &mut [u32]) {
    if rle.iter().step_by(2).any(|&v| v == NO_PROVINCE) { return; }
    let mut i = 0;
    while i < rle.len() { if rle[i] == 65535 { rle[i] = NO_PROVINCE; } i += 2; }
}

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

/// Köppen-keyed province-size multiplier on the target AREA (not on the separation —
/// see `target_area_km2`). The truly hostile biomes hold VAST, thinly-administered
/// provinces on Earth — a single Nunavut, Siberian or Saharan district dwarfs a
/// European county — so their area budget is a multiple of a temperate one. Compounded
/// with the habitability ramp this reaches ≈15× the area of a fertile-lowland province.
fn koppen_area_mult(koppen: u8) -> f64 {
    use crate::sim::koppen as kp;
    match koppen {
        kp::EF => 6.0,                       // ice cap — Antarctic / Greenland interior
        kp::ET => 5.0,                       // tundra
        kp::DFD | kp::DWD => 4.0,            // extreme subarctic
        kp::DFC | kp::DWC | kp::DSD => 3.2,  // subarctic taiga
        kp::BWH | kp::BWK => 4.0,            // hot / cold desert
        kp::BSH | kp::BSK => 2.0,            // semi-arid steppe
        kp::H => 2.2,                        // high alpine
        _ => 1.0,                            // temperate / tropical / Mediterranean
    }
}

// ── Equal-area partition tuning (see the "three stages" note in the module header) ──
/// A province is MERGED away when its real area falls below this fraction of its own
/// climate budget, and SPLIT when it rises above `PROV_MAX_FRAC` of it. Together they
/// are the guarantee the old single-pass sliver merge could not give: every province on
/// a landmass big enough to hold one lands inside `[0.75, 1.45] × budget`. (Two
/// exceptions by design: a small ISLAND is its own province however tiny, and the polar
/// caps are merged whole — see `merge_polar_caps`.)
const PROV_MIN_FRAC: f64 = 0.75;
const PROV_MAX_FRAC: f64 = 1.45;
/// Re-floods run after the first one, each with the seed weights nudged by how far
/// that province's area missed its budget. Two is enough to pull the spread in hard
/// (the merge/split stages clean up the tail); each pass costs one full Dijkstra.
const BALANCE_PASSES: usize = 2;
/// Step size for the seed-weight update, as a fraction of the first-order correction
/// `radius · ln(area/budget)`. Below 1 so the iteration converges instead of ringing.
const BALANCE_GAIN: f64 = 0.8;
/// Clamp on a seed's accumulated weight, so one pathological seed (a province walled
/// in by crests it cannot grow past) can't be pushed to a cost that swallows a region.
const BALANCE_W_CAP: f64 = 80.0;
/// Absolute floor on seed separation, in cells — the partition must never shatter into
/// a speckle of near-single-cell provinces however fine the granularity or the climate.
const MIN_SEED_SEP: f64 = 8.0;
/// Poisson-disk separation as a fraction of the province SIDE. A saturated dart-throwing
/// packing with minimum distance `s` settles at roughly one point per `1.4·s²` of area,
/// so asking for a separation of `0.84 · side` is what actually yields provinces of
/// `side²`. Getting this constant wrong doesn't just scale every province — it scales
/// them by an amount that varies with how much of the map is at the separation floor,
/// which is how the climate-size relationship gets flattened.
const SEED_PACK: f64 = 0.84;
/// Candidate-grid step for the filler scatter, as a fraction of the FINEST separation.
/// Below 1 so the rejection radius (which knows the local climate) decides the density
/// rather than the walk (which does not).
const SEED_WALK_FRAC: f32 = 0.5;
/// Most sub-provinces one over-budget province may be split into in a single pass.
const MAX_SPLIT_PARTS: usize = 8;

/// Per-step cost for the flood to HOP a continental-shelf sea cell, so a cluster of
/// shelf-connected islands (an archipelago) merges into ONE province instead of each
/// islet becoming its own. High enough that a real open-ocean strait (deep, non-shelf
/// water) is never crossed — only the shallow water shared between neighbouring
/// islands, which is exactly what makes them read as a single region.
const SEA_HOP: f64 = 5.0;

/// True Köppen POLAR class of a cell: tundra (ET) or ice cap (EF). Hot/cold deserts are
/// deliberately NOT in here — they now follow the ordinary climate area budget
/// (`koppen_area_mult` ≈ 4× a temperate province), so a great desert reads as a handful
/// of big provinces rather than one continent-sized blob.
fn is_polar_koppen(koppen: u8) -> bool {
    use crate::sim::koppen as kp;
    matches!(koppen, kp::ET | kp::EF)
}

/// Collapse the POLAR CAPS: union every adjacent pair of polar provinces on the same
/// landmass, with NO area cap, so an ice cap / arctic waste is ONE province per polar
/// landmass (Antarctica reads as a single territory, as does a Greenland-style cap).
/// This is the one deliberate exception to the equal-area rule — it is administratively
/// truer than slicing an ice sheet into equal districts, and it is what makes the
/// climate budget elsewhere affordable. Operates on `owner` (pre-compaction ids);
/// deterministic (pairs sorted, union root = lower id).
fn merge_polar_caps(buf: &WorldBuffer, owner: &mut [u32], island: &[u32], total: usize) {
    if buf.koppen.is_empty() { return; }
    let Some(max_owner) = owner.iter().copied().filter(|&o| o != u32::MAX).max() else { return; };
    let n = max_owner as usize + 1;
    let w = buf.width;
    let hi = buf.height as i32;
    // Per-owner tallies: cells, polar cells, one island id.
    let mut cells = vec![0u32; n];
    let mut polar = vec![0u32; n];
    let mut isle = vec![u32::MAX; n];
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX { continue; }
        let oi = o as usize;
        cells[oi] += 1;
        isle[oi] = island[c];
        if is_polar_koppen(buf.koppen[c]) { polar[oi] += 1; }
    }
    // An owner counts as polar when a majority of its cells are.
    let group = |oi: usize| -> u8 {
        if polar[oi] * 2 > cells[oi].max(1) { 1 } else { 0 }
    };
    // Union-find over owners.
    fn find(p: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while p[r as usize] != r { r = p[r as usize]; }
        let mut c = x;
        while p[c as usize] != r { let nx = p[c as usize]; p[c as usize] = r; c = nx; }
        r
    }
    let mut parent: Vec<u32> = (0..n as u32).collect();
    // Adjacent same-group same-island owner pairs (right + down avoids duplicates).
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX { continue; }
        let g = group(o as usize);
        if g == 0 { continue; }
        let cx = (c as u32 % w) as i32;
        let cy = (c as u32 / w) as i32;
        for &(dx, dy) in &[(1i32, 0i32), (0, 1)] {
            let ny = cy + dy; if ny < 0 || ny >= hi { continue; }
            let no = owner[buf.widx(cx + dx, ny)];
            if no == u32::MAX || no == o { continue; }
            if group(no as usize) == g && isle[o as usize] == isle[no as usize] {
                let (a, b) = if o < no { (o, no) } else { (no, o) };
                pairs.push((a, b));
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    for (a, b) in pairs {
        let ra = find(&mut parent, a);
        let rb = find(&mut parent, b);
        if ra == rb { continue; }
        let (root, child) = if ra < rb { (ra, rb) } else { (rb, ra) };
        parent[child as usize] = root;
    }
    for o in owner.iter_mut() {
        if *o != u32::MAX { *o = find(&mut parent, *o); }
    }
}

/// Per-owner area accounting against the climate budget: how many km² it actually
/// holds, and how many km² the land it holds says it *should* hold.
struct AreaBook {
    /// km² claimed per owner.
    area: Vec<f64>,
    /// Mean per-cell target (km²) over the land the owner holds — its BUDGET.
    want: Vec<f64>,
    /// Land cells per owner (0 ⇒ the id is unused).
    cells: Vec<f64>,
}

/// Tally `area` / `want` / `cells` per owner id in one O(total) pass. Only true land
/// counts: shelf-sea conduit cells and lake cells carry no area budget.
fn tally_areas<F: Fn(usize) -> f64>(
    buf: &WorldBuffer, owner: &[u32], n: usize, row_km2: &[f64], target_km2: &F,
    is_lake: &[bool], total: usize,
) -> AreaBook {
    let w = buf.width as usize;
    let mut book = AreaBook { area: vec![0.0; n], want: vec![0.0; n], cells: vec![0.0; n] };
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX || buf.terrain[c] != 1 || is_lake[c] { continue; }
        let oi = o as usize;
        if oi >= n { continue; }
        book.area[oi] += row_km2[c / w];
        book.want[oi] += target_km2(c);
        book.cells[oi] += 1.0;
    }
    for i in 0..n {
        if book.cells[i] > 0.0 { book.want[i] /= book.cells[i]; }
    }
    book
}

/// **Stage 1b · split.** Any province holding more than `PROV_MAX_FRAC` of its own
/// climate budget is cut into `round(area / budget)` parts by a local farthest-point
/// flood *inside* that province, using the same crest/river costs as the main flood —
/// so the new internal borders land on real features, exactly like the outer ones.
///
/// This is the half of the size guarantee the flood alone cannot provide: a region that
/// happened to receive one seed (a long peninsula, an island bigger than one province,
/// a basin walled off by mountains) has no neighbour to be balanced against, so no seed
/// weight can shrink it. Polar-majority provinces are exempt — the caps are merged, not
/// split (see `merge_polar_caps`).
///
/// Assigns part 0 the original id and mints one new id per extra part, pushing its seed
/// cell onto `owner_seed` so naming still works. Deterministic: provinces are processed
/// in id order and every tie is broken on the cell index.
#[allow(clippy::too_many_arguments)]
fn split_oversized<F: Fn(usize) -> f64>(
    buf: &WorldBuffer, owner: &mut [u32], owner_seed: &mut Vec<u32>,
    ridge: &[f32], river_divide: &[f32], river_unite: &[bool], is_lake: &[bool],
    row_km2: &[f64], target_km2: &F, dcost: &mut [f64], total: usize,
) {
    let w = buf.width;
    let hi = buf.height as i32;
    let n0 = owner_seed.len();
    let book = tally_areas(buf, owner, n0, row_km2, target_km2, is_lake, total);

    // Which owners are over budget, and into how many parts. Polar-majority provinces
    // are skipped (the caps merge instead).
    let have_koppen = !buf.koppen.is_empty();
    let mut polar = vec![0u32; n0];
    if have_koppen {
        for c in 0..total {
            let o = owner[c];
            if o == u32::MAX || (o as usize) >= n0 { continue; }
            if is_polar_koppen(buf.koppen[c]) { polar[o as usize] += 1; }
        }
    }
    let mut targets: Vec<(u32, usize)> = Vec::new();
    for p in 0..n0 {
        if book.cells[p] < 2.0 || book.want[p] <= 0.0 { continue; }
        if polar[p] as f64 * 2.0 > book.cells[p] { continue; }
        let ratio = book.area[p] / book.want[p];
        if ratio <= PROV_MAX_FRAC { continue; }
        let k = (ratio.round() as usize).clamp(2, MAX_SPLIT_PARTS);
        targets.push((p as u32, k));
    }
    if targets.is_empty() { return; }

    // `dcost` is the caller's finished main-flood distance array, reused as scratch — a
    // second world-sized f64 buffer would cost 200 MB on a large world. Each split only
    // ever touches (and then restores) the cells of the province it is cutting.
    let mut cells: Vec<u32> = Vec::new();

    for (p, k) in targets {
        cells.clear();
        for c in 0..total { if owner[c] == p { cells.push(c as u32); } }
        if cells.len() < 4 { continue; }
        let new_lo = owner_seed.len() as u32;
        let new_hi = new_lo + (k as u32 - 1);
        // Membership: a cell belongs to this split as long as it is the original owner
        // or one of the parts minted for it. Checking the id RANGE means the local flood
        // can write its assignment straight into `owner` with no extra world-sized array.
        let mine = |o: u32| o == p || (o >= new_lo && o < new_hi);

        // ── Seeds by farthest-point sampling: start from the province's first cell, then
        //    repeatedly take the cell furthest (in hops, inside the province) from every
        //    seed so far. Disconnected fragments have infinite hop distance and are
        //    therefore picked first, which is exactly right — an enclave becomes its own
        //    part rather than being cut off from its seed.
        let mut seeds: Vec<u32> = vec![cells[0]];
        while seeds.len() < k {
            for &c in &cells { dcost[c as usize] = f64::INFINITY; }
            let mut q: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
            for &s in &seeds { dcost[s as usize] = 0.0; q.push_back(s); }
            while let Some(c) = q.pop_front() {
                let d = dcost[c as usize] + 1.0;
                let cx = (c % w) as i32; let cy = (c / w) as i32;
                for dy in -1i32..=1 {
                    let ny = cy + dy;
                    if ny < 0 || ny >= hi { continue; }
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let ni = buf.widx(cx + dx, ny);
                        if !mine(owner[ni]) || dcost[ni] <= d { continue; }
                        dcost[ni] = d;
                        q.push_back(ni as u32);
                    }
                }
            }
            // Farthest cell; ties (and the whole unreachable set) break on cell index.
            let mut best = (f64::NEG_INFINITY, u32::MAX);
            for &c in &cells {
                let d = dcost[c as usize];
                let d = if d.is_finite() { d } else { f64::MAX };
                if d > best.0 && !seeds.contains(&c) { best = (d, c); }
            }
            if best.1 == u32::MAX { break; }
            seeds.push(best.1);
        }
        if seeds.len() < 2 { continue; }

        // ── Local cost-flood from those seeds, same terrain terms as the main flood. ──
        for &c in &cells { dcost[c as usize] = f64::INFINITY; }
        let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
        for (si, &s) in seeds.iter().enumerate() {
            // Part 0 keeps the province's own id; the rest get fresh ids.
            let id = if si == 0 { p } else { new_lo + si as u32 - 1 };
            if si > 0 { owner_seed.push(s); }
            dcost[s as usize] = 0.0;
            owner[s as usize] = id;
            heap.push(HeapItem { cost: 0.0, cell: s, owner: id });
        }
        while let Some(HeapItem { cost, cell, owner: ow }) = heap.pop() {
            let ci = cell as usize;
            if cost > dcost[ci] || owner[ci] != ow { continue; }
            let cx = (cell % w) as i32; let cy = (cell / w) as i32;
            for dy in -1i32..=1 {
                let ny = cy + dy;
                if ny < 0 || ny >= hi { continue; }
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let ni = buf.widx(cx + dx, ny);
                    if !mine(owner[ni]) { continue; }
                    let diagonal = dx != 0 && dy != 0;
                    let corners = if diagonal {
                        Some((buf.widx(cx + dx, cy), buf.widx(cx, ny)))
                    } else { None };
                    if let Some((ca, cb)) = corners {
                        if is_lake[ca] && is_lake[cb] { continue; }
                    }
                    let mut step = if diagonal { 1.4142 } else { 1.0 };
                    if river_unite[ni] { step *= RIVER_UNITE; }
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
                    if nc < dcost[ni] {
                        dcost[ni] = nc;
                        owner[ni] = ow;
                        heap.push(HeapItem { cost: nc, cell: ni as u32, owner: ow });
                    }
                }
            }
        }
        for &c in &cells { dcost[c as usize] = f64::INFINITY; }
    }
}

/// **Stage 1c · merge.** Fold every province below `PROV_MIN_FRAC` of its budget into
/// the neighbour it shares the most border with, ITERATIVELY: after each fold the merged
/// area is re-checked, so a sliver that lands in another sliver keeps merging instead of
/// surviving as a slightly-bigger sliver (the flaw in the old single-pass version, where
/// a remap chain could terminate on a province that was itself under the floor).
///
/// A province with no land neighbour at all — a small island — is left alone: an island
/// IS its province, however small, and there is nothing to merge it into.
/// Deterministic: candidates are taken smallest-area first, and every tie is broken on
/// the province id.
fn merge_undersized<F: Fn(usize) -> f64>(
    buf: &WorldBuffer, owner: &mut [u32], n: usize,
    row_km2: &[f64], target_km2: &F, total: usize,
) {
    if n == 0 { return; }
    let w = buf.width;
    let hi = buf.height as i32;
    // No lake mask needed here: lake and sea cells already carry `u32::MAX` as owner.
    let mut area = vec![0.0f64; n];
    let mut want_sum = vec![0.0f64; n];
    let mut cells = vec![0.0f64; n];
    // Shared border length per ordered pair, as one map per province.
    let mut shared: Vec<std::collections::HashMap<u32, u32>> =
        (0..n).map(|_| std::collections::HashMap::new()).collect();
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX || (o as usize) >= n { continue; }
        let oi = o as usize;
        area[oi] += row_km2[c / w as usize];
        want_sum[oi] += target_km2(c);
        cells[oi] += 1.0;
        let cx = (c as u32 % w) as i32; let cy = (c as u32 / w) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = cy + dy; if ny < 0 || ny >= hi { continue; }
            let no = owner[buf.widx(cx + dx, ny)];
            if no != u32::MAX && no != o && (no as usize) < n {
                *shared[oi].entry(no).or_insert(0) += 1;
            }
        }
    }

    // Union-find over provinces; the surviving root is always the lower id.
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(p: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while p[r as usize] != r { r = p[r as usize]; }
        let mut c = x;
        while p[c as usize] != r { let nx = p[c as usize]; p[c as usize] = r; c = nx; }
        r
    }
    let under = |area: &[f64], want_sum: &[f64], cells: &[f64], p: usize| -> bool {
        if cells[p] < 1.0 { return false; }
        let want = want_sum[p] / cells[p];
        want > 0.0 && area[p] < PROV_MIN_FRAC * want
    };

    // Rounds, not one pass: each fold changes the area of the survivor, which can lift it
    // over the floor (done) or leave it under (merge again next round).
    const MAX_ROUNDS: usize = 12;
    for _ in 0..MAX_ROUNDS {
        let mut cand: Vec<u32> = (0..n as u32)
            .filter(|&p| find(&mut parent, p) == p && under(&area, &want_sum, &cells, p as usize))
            .collect();
        if cand.is_empty() { break; }
        // Smallest first, id as the tie-break — never `sort_by` on the float alone, or
        // two equal-area slivers merge in an unspecified order.
        cand.sort_by(|&a, &b| {
            area[a as usize].partial_cmp(&area[b as usize]).unwrap_or(Ordering::Equal)
                .then_with(|| a.cmp(&b))
        });
        let mut merged_any = false;
        for p in cand {
            let rp = find(&mut parent, p);
            if rp != p || !under(&area, &want_sum, &cells, p as usize) { continue; }
            // Best neighbour: longest shared frontier, then the SMALLER partner (so a
            // sliver joins the modest neighbour it completes rather than the giant next
            // door), then the lower id.
            let entries: Vec<(u32, u32)> = shared[p as usize].iter().map(|(&k, &v)| (k, v)).collect();
            let mut roots: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
            for (nid, len) in entries {
                let rn = find(&mut parent, nid);
                if rn == p { continue; }
                *roots.entry(rn).or_insert(0) += len;
            }
            let mut ranked: Vec<(u32, u32)> = roots.into_iter().collect();
            ranked.sort_by(|a, b| {
                b.1.cmp(&a.1)
                    .then_with(|| area[a.0 as usize].partial_cmp(&area[b.0 as usize]).unwrap_or(Ordering::Equal))
                    .then_with(|| a.0.cmp(&b.0))
            });
            // No land neighbour ⇒ a small island, which IS its province. Leave it.
            let Some(&(rn, _)) = ranked.first() else { continue };
            let (root, child) = if rn < p { (rn, p) } else { (p, rn) };
            parent[child as usize] = root;
            area[root as usize] += area[child as usize];
            want_sum[root as usize] += want_sum[child as usize];
            cells[root as usize] += cells[child as usize];
            area[child as usize] = 0.0; want_sum[child as usize] = 0.0; cells[child as usize] = 0.0;
            // Fold the child's frontiers into the survivor's so the next round sees the
            // merged province's real adjacency.
            let child_edges: Vec<(u32, u32)> =
                shared[child as usize].drain().collect();
            for (nid, len) in child_edges {
                if find(&mut parent, nid) == root as u32 { continue; }
                *shared[root as usize].entry(nid).or_insert(0) += len;
            }
            merged_any = true;
        }
        if !merged_any { break; }
    }

    for o in owner.iter_mut() {
        if *o != u32::MAX && (*o as usize) < n { *o = find(&mut parent, *o); }
    }
}

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
) -> (Vec<Province>, Vec<u32>) {
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

    // ── The AREA BUDGET: one number per cell, "how big should the province that owns
    //    this cell be, in km²". Everything downstream — where seeds go, how the flood
    //    is balanced, what gets merged and what gets split — is derived from it, so the
    //    three stages can never disagree about the intended size. ──
    //
    // Granularity → number of province "columns" across the map, and hence the side of
    // a temperate, fertile province in km at the equator.
    let cols = 18.0 + 92.0 * g as f64;
    let cell_km = 40075.0 / w as f64;             // width of one cell at the equator
    let base_side_km = (40075.0 / cols) * 0.5;    // side of the SMALLEST (prime-land) province
    // Real, latitude-aware area of one cell in each row. Province size is compared in
    // km², not in cells: a cell near the pole covers a fraction of an equatorial one, so
    // counting cells would make high-latitude provinces far smaller than they look.
    let row_km2: Vec<f64> = (0..h)
        .map(|y| {
            let latr = (buf.latitude(y) as f64).to_radians();
            cell_km * cell_km * latr.cos().max(0.05)
        })
        .collect();
    let hab_at = |i: usize| -> f32 {
        if buf.habitability.is_empty() { 0.5 } else { buf.habitability[i] as f32 / 255.0 }
    };
    let have_koppen = !buf.koppen.is_empty();
    let have_hab = !buf.habitability.is_empty();
    // Target area at a cell — the product of two independent levers:
    //   · CLIMATE (`koppen_area_mult`): the ice caps, tundra, deserts and taiga that hold
    //     genuinely continent-scale administrative blocks on Earth get 2-6× the area;
    //   · HABITABILITY: a general "fewer people ⇒ bigger units" ramp on top, so a barren
    //     temperate upland is still larger than the fertile valley beside it.
    let base_km2 = base_side_km * base_side_km;
    // Province SIDE at a cell, in cells at this latitude — the one primitive everything
    // else is derived from. The `MIN_SEED_SEP` floor is applied HERE, before the budget
    // is read back off it, so a floored cell's budget matches the province the seeding
    // can actually produce. (Flooring only the separation and not the budget would leave
    // every province on a fine grid permanently "under budget", and the merge stage would
    // dutifully dissolve the whole map into a few blobs.)
    let side_cells = |i: usize| -> f64 {
        let km = if have_koppen { koppen_area_mult(buf.koppen[i]) } else { 1.0 };
        let hostile = if have_hab {
            (1.0 - hab_at(i).clamp(0.0, 1.0) as f64).powf(1.4)
        } else { 0.0 };
        let want_km2 = base_km2 * km * (1.0 + 1.5 * hostile);
        (want_km2 / row_km2[i / w as usize].max(1e-6)).sqrt().max(MIN_SEED_SEP)
    };
    // The AREA BUDGET at a cell, in km² — the size the province owning it is aiming for.
    // Read straight back off `side_cells`, so seeding, balancing, splitting and merging
    // are all quoting the same number.
    let target_km2 = |i: usize| -> f64 {
        let s = side_cells(i);
        s * s * row_km2[i / w as usize]
    };
    // Seed separation: one province side, shaved slightly so the Poisson-disk rejection
    // does not systematically overshoot the intended spacing.
    let local_sep2 = |i: usize| -> i64 {
        let s = (side_cells(i) * SEED_PACK) as i64;
        (s * s).max(1)
    };
    // The finest separation anywhere (prime land at the equator) — the grid the filler
    // scatter walks and the yardstick for "how far may the snap stage move a line".
    let base_sep = ((base_side_km / cell_km) as f32).max(MIN_SEED_SEP as f32);
    // ── Seed rejection, on a SPATIAL HASH. ──
    // The scatter is a Poisson-disk: a candidate is kept only if no accepted seed lies
    // within the local separation. Testing that against every seed accepted so far is
    // O(seeds) per candidate — quadratic overall, and on a world-sized grid it was the
    // single most expensive thing in the seeding. Bucketing seeds by a coarse grid makes
    // it a scan of the few buckets the separation circle actually covers, which is what
    // lets the candidate walk below be FINE enough for the separation (not the walk) to
    // set the density. Without that the achieved spacing is the grid step, provinces come
    // out well over budget, and — worse — by a factor that differs per climate, which
    // silently flattens the whole climate-size relationship.
    let bucket = (base_side_km / cell_km).max(MIN_SEED_SEP) as i32;
    let bucket = bucket.max(4);
    let bw = (wi + bucket - 1) / bucket;
    let bh = (hi + bucket - 1) / bucket;
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); (bw * bh).max(1) as usize];
    let bucket_of = |bx: i32, by: i32| -> usize {
        let gx = (bx.rem_euclid(wi)) / bucket;
        let gy = by.clamp(0, hi - 1) / bucket;
        (gy * bw + gx) as usize
    };
    let too_close = |buckets: &[Vec<u32>], bx: i32, by: i32, sep2: i64| -> bool {
        let r = ((sep2 as f64).sqrt() as i32 / bucket) + 1;
        let gy0 = (by / bucket - r).max(0);
        let gy1 = (by / bucket + r).min(bh - 1);
        let gx0 = bx / bucket - r;
        for gy in gy0..=gy1 {
            for gxr in gx0..=(bx / bucket + r) {
                let gx = gxr.rem_euclid(bw);
                for &sc in &buckets[(gy * bw + gx) as usize] {
                    let sx = (sc % w) as i32; let sy = (sc / w) as i32;
                    let mut ddx = (sx - bx).abs(); if ddx > wi / 2 { ddx = wi - ddx; }
                    let dd = (ddx as i64) * (ddx as i64) + ((sy - by) as i64) * ((sy - by) as i64);
                    if dd < sep2 { return true; }
                }
            }
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
        if too_close(&buckets, bx, by, local_sep2(i)) { continue; }
        is_seed[i] = true; seed_cells.push(i as u32);
        buckets[bucket_of(bx, by)].push(i as u32);
    }
    // Filler seeds on a candidate grid FINER than the finest separation, so it is the
    // local separation — not the walk — that decides how many seeds land. The LOCAL
    // separation rejects most candidates in hostile land → few, large provinces there,
    // while habitable land keeps most → many, small provinces. Jittered off-lattice.
    // The candidate score prefers a LESSER RIVER's valley, so valleys become province
    // CORES and the interfluves become the borders ("rivers unite").
    let bspacing = ((base_sep * SEED_WALK_FRAC) as i32).max(3);
    let jit = (base_sep * 0.42) as i64;
    let win = (bspacing / 2).max(2);
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
                if !is_seed[bi] && !too_close(&buckets, bx, by, local_sep2(bi)) {
                    is_seed[bi] = true; seed_cells.push(bi as u32);
                    buckets[bucket_of(bx, by)].push(bi as u32);
                }
            }
            gx += bspacing;
        }
        gy += bspacing;
    }
    if seed_cells.is_empty() { return (Vec::new(), vec![NO_PROVINCE; total]); }

    // ── Multi-source cost-flood (Dijkstra) over land. Sets province COUNT, SIZE and
    //    TOPOLOGY; the border LINES are re-placed afterwards by the snap stage. ──
    //
    // The flood is run several times (see `BALANCE_PASSES`). Every pass is identical
    // except for `weights`, an ADDITIVE handicap on each seed's starting cost: a seed
    // whose province came out over its area budget starts the next pass deeper in debt,
    // so its frontier meets its neighbours' earlier and it claims less. This is an
    // additively-weighted (power-diagram) Voronoi, and it is the reason sizes can be
    // equalised WITHOUT abandoning the terrain costs — a border still falls on the crest
    // between two seeds, it just falls on a different crest. Weights are non-negative,
    // so Dijkstra's correctness is untouched.
    let have_shelf = !buf.is_shelf.is_empty();
    let mut owner = vec![u32::MAX; total];
    let mut dist = vec![f64::INFINITY; total];
    let flood = |weights: &[f64], owner: &mut Vec<u32>, dist: &mut Vec<f64>| {
        owner.iter_mut().for_each(|o| *o = u32::MAX);
        dist.iter_mut().for_each(|d| *d = f64::INFINITY);
        let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
        for (oi, &sc) in seed_cells.iter().enumerate() {
            let w0 = weights.get(oi).copied().unwrap_or(0.0);
            owner[sc as usize] = oi as u32;
            dist[sc as usize] = w0;
            heap.push(HeapItem { cost: w0, cell: sc, owner: oi as u32 });
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
                    // The flood normally runs on land, but it may also HOP a shelf-sea
                    // cell (shallow water shared between neighbouring islands) at a stiff
                    // flat cost, so a shelf-connected archipelago merges into one province
                    // rather than each islet becoming its own. Deep (non-shelf) ocean and
                    // lakes stay impassable, so real straits are never crossed.
                    let land = buf.terrain[ni] == 1;
                    let shelf_sea = have_shelf && buf.terrain[ni] == 0 && buf.is_shelf[ni] == 1;
                    if is_lake[ni] || (!land && !shelf_sea) { continue; }
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
                    let base_step = if diagonal { 1.4142 } else { 1.0 };
                    // A shelf-sea hop carries only the flat crossing cost; land carries the
                    // full valley-unite / altitude / crest / river-divide terrain terms.
                    let (step, alt, crest, cross) = if land {
                        let mut step = base_step;
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
                        (step, alt, crest, cross)
                    } else {
                        (base_step * SEA_HOP, 0.0, 0.0, 0.0)
                    };
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
    };

    // ── Balance the flood against the area budget. Each pass: flood, measure every
    //    province's real area against the mean budget of the land it holds, and nudge
    //    the seed weight by the first-order correction `radius · ln(area/budget)` (the
    //    extra cost that moves a frontier the right number of cells). ──
    let ns = seed_cells.len();
    let mut weights = vec![0.0f64; ns];
    for pass in 0..=BALANCE_PASSES {
        flood(&weights, &mut owner, &mut dist);
        if pass == BALANCE_PASSES { break; }
        let mut area = vec![0.0f64; ns];       // km² actually claimed
        let mut budget = vec![0.0f64; ns];     // Σ per-cell target
        let mut cells = vec![0.0f64; ns];
        for c in 0..total {
            let o = owner[c];
            // Shelf-sea conduits are not land and must not count toward an area budget.
            if o == u32::MAX || buf.terrain[c] != 1 || is_lake[c] { continue; }
            let oi = o as usize;
            area[oi] += row_km2[c / w as usize];
            budget[oi] += target_km2(c);
            cells[oi] += 1.0;
        }
        for s in 0..ns {
            if cells[s] < 1.0 || area[s] <= 0.0 { continue; }
            let want = budget[s] / cells[s];          // mean target over the land it holds
            let radius = (cells[s] / std::f64::consts::PI).sqrt().max(1.0);
            let err = (area[s] / want).ln();
            weights[s] = (weights[s] + BALANCE_GAIN * radius * err)
                .clamp(-BALANCE_W_CAP, BALANCE_W_CAP);
        }
        // Dijkstra needs non-negative starting costs. Only DIFFERENCES between weights
        // affect where frontiers meet, so re-basing the whole set on its minimum is a
        // no-op for the partition and keeps every start cost ≥ 0.
        let lo = weights.iter().copied().fold(f64::INFINITY, f64::min);
        if lo.is_finite() && lo != 0.0 { for x in weights.iter_mut() { *x -= lo; } }
    }

    // Shelf-sea cells were only conduits for island-merging: strip their ownership so
    // the partition covers land only (the islands they linked already share one owner).
    for c in 0..total {
        if buf.terrain[c] != 1 || is_lake[c] { owner[c] = u32::MAX; }
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
    // One seed cell per owner id, extended as the split stage below mints new ids.
    let mut owner_seed: Vec<u32> = seed_cells.clone();
    owner_seed.extend_from_slice(&extra_seed_cell);

    // ── Stage 1b · SPLIT what came out too big, then MERGE what came out too small.
    //    The balanced flood gets most provinces close to budget, but it cannot fix two
    //    cases on its own: a region holding a single seed (nothing to share with) comes
    //    out as one giant, and a seed boxed in by crests comes out as a sliver. Splitting
    //    and merging against the SAME budget closes both ends, which is what turns
    //    "mostly even" into a guaranteed size band. ──
    split_oversized(
        buf, &mut owner, &mut owner_seed, &ridge, &river_divide, &river_unite,
        &is_lake, &row_km2, &target_km2, &mut dist, total,
    );
    merge_undersized(buf, &mut owner, owner_seed.len(), &row_km2, &target_km2, total);

    // Collapse the POLAR CAPS into one province per polar landmass — the deliberate
    // exception to the size band (an ice sheet is one territory, not a grid of equal
    // districts). Deserts are NOT merged here; their 4× climate budget already makes
    // them large while keeping them bounded.
    merge_polar_caps(buf, &mut owner, &island, total);

    let seed_of = |oi: u32| -> u32 { owner_seed[oi as usize] };

    // ── Compact province ids to 0..n and build per-cell id map. ──
    let mut old_to_new = std::collections::HashMap::<u32, u32>::new();
    let mut province_id = vec![NO_PROVINCE; total];
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX { continue; }
        let next = old_to_new.len() as u32;
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
    let mut pair_kinds: std::collections::HashMap<(u32, u32), [u32; 4]> =
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
        let mut neighbors: Vec<u32> = neighbors_detail.iter().map(|b| b.neighbor).collect();
        neighbors.sort_unstable();

        provinces.push(Province {
            id: pid as u32,
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

    // Split lakes down the middle: fill each lake cell with its nearest-shore province
    // so a wide lake becomes a boundary that runs down its CENTRE (each side owns up to
    // the midline), instead of an unowned hole. Done last, so it only refines the raster
    // (borders, hit-testing, hub→province mapping) and leaves the land-only province
    // stats above untouched.
    assign_lakes_to_nearest(buf, &mut province_id, &is_lake);

    (provinces, province_id)
}

/// Fill lake cells by nearest-shore province (multi-source BFS from every province-
/// owned cell that touches a lake, expanding only across lake water). Two fronts meet
/// on the lake's medial axis, so a lake shared by two provinces is divided down its
/// centre. Deterministic: the seed owner ties break on the lower province id, and the
/// BFS front is FIFO from a scan-ordered seed list.
fn assign_lakes_to_nearest(buf: &WorldBuffer, province_id: &mut [u32], is_lake: &[bool]) {
    let w = buf.width;
    let hi = buf.height as i32;
    let total = province_id.len();
    let mut assigned = vec![NO_PROVINCE; total];
    let mut q: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
    // Seed: each lake cell bordering province-owned land takes the (lowest-id) owner.
    for c in 0..total {
        if !is_lake[c] { continue; }
        let cx = (c as u32 % w) as i32;
        let cy = (c as u32 / w) as i32;
        let mut best = NO_PROVINCE;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = cy + dy; if ny < 0 || ny >= hi { continue; }
            let ni = buf.widx(cx + dx, ny);
            if !is_lake[ni] {
                let pid = province_id[ni];
                if pid != NO_PROVINCE && pid < best { best = pid; }
            }
        }
        if best != NO_PROVINCE { assigned[c] = best; q.push_back(c as u32); }
    }
    // BFS across lake interiors — nearest shore (in hops) wins each cell.
    while let Some(cell) = q.pop_front() {
        let owner = assigned[cell as usize];
        let cx = (cell % w) as i32;
        let cy = (cell / w) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = cy + dy; if ny < 0 || ny >= hi { continue; }
            let ni = buf.widx(cx + dx, ny);
            if is_lake[ni] && assigned[ni] == NO_PROVINCE {
                assigned[ni] = owner;
                q.push_back(ni as u32);
            }
        }
    }
    for c in 0..total {
        if is_lake[c] && assigned[c] != NO_PROVINCE { province_id[c] = assigned[c]; }
    }
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
fn compute_label_anchors(buf: &WorldBuffer, province_id: &[u32], n: usize) -> Vec<(u32, u32, f32)> {
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
    province_id: &mut [u32],
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
            province_id[rep_cell[p] as usize] = p as u32;
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
                province_id[ni] = ow as u32;
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
    fn border_mask(buf: &WorldBuffer, ids: &[u32]) -> Vec<bool> {
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

    /// **The size guarantee.** On uniform land every province should come out close to
    /// the same real area: no slivers at the bottom, no continent at the top. This is
    /// what the balanced flood + split + iterative merge exist for — a plain cost-flood
    /// produced a long tail of tiny provinces wherever a seed was boxed in.
    #[test]
    fn province_areas_are_evenly_sized() {
        // Wide enough that a cell is small compared with a province: `MIN_SEED_SEP` is a
        // floor in CELLS, so on a coarse grid it binds everywhere and would make this
        // test pass for the wrong reason.
        let (gw, gh) = (480u32, 240u32);
        let mut buf = blank_world_sized(gw, gh);
        // Relief and fertility variation, so the flood's terrain costs really do vary
        // (a perfectly flat world would come out even for trivial reasons).
        for y in 0..gh {
            for x in 0..gw {
                let i = buf.idx(x, y);
                buf.elevation[i] = (((x * 7 + y * 13) % 11) as f32) / 60.0;
                buf.fertility[i] = (((x * 3 + y * 5) % 9) as f32) / 9.0;
            }
        }
        let towns = vec![settle("a", 20, 20, 9000), settle("b", 300, 150, 7000)];
        let (provs, _) = generate_provinces(&buf, &[], &[], &towns, 0.0);
        assert!(provs.len() > 20, "need a populated map, got {}", provs.len());

        let mut areas: Vec<f64> = provs.iter().map(|p| p.area_km2 as f64).collect();
        areas.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = areas[areas.len() / 2];
        let p10 = areas[areas.len() / 10];
        let p90 = areas[areas.len() * 9 / 10];
        let smallest = areas[0];
        let largest = areas[areas.len() - 1];
        eprintln!(
            "areas km²: min {smallest:.0} · p10 {p10:.0} · median {median:.0} · p90 {p90:.0} · max {largest:.0} \
             (p90/p10 = {:.2}, max/min = {:.2}, n = {})",
            p90 / p10.max(1.0), largest / smallest.max(1.0), provs.len()
        );
        assert!(p10 >= 0.55 * median, "a tenth of provinces are slivers: p10 {p10:.0} vs median {median:.0}");
        assert!(p90 <= 1.9 * median, "a tenth of provinces are giants: p90 {p90:.0} vs median {median:.0}");
        // The absolute tail matters too — one 1-cell province is one too many.
        assert!(smallest >= 0.30 * median,
            "smallest province {smallest:.0} km² is a fragment beside the median {median:.0} km²");
        assert!(largest <= 3.0 * median,
            "largest province {largest:.0} km² dwarfs the median {median:.0} km²");
    }

    /// PERF harness — the partition now runs the cost-flood `1 + BALANCE_PASSES` times
    /// plus a split and a merge, so its cost has to be watched:
    ///   `cargo test --release --lib bench_province_generation -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_province_generation() {
        use std::time::Instant;
        let (gw, gh) = (1800u32, 900u32);
        let mut buf = blank_world_sized(gw, gh);
        for y in 0..gh {
            for x in 0..gw {
                let i = buf.idx(x, y);
                // A continent-ish landmass with relief, so the flood does real work.
                let land = ((x as f32 / gw as f32 * 6.28).sin() + (y as f32 / gh as f32 * 3.14).sin()) > -0.4;
                buf.terrain[i] = if land { 1 } else { 0 };
                // Real relief: two mountain belts crossing the continent plus noise, so
                // the flood's crest/altitude costs vary the way they do on a real world.
                let fx = x as f32 / gw as f32;
                let fy = y as f32 / gh as f32;
                let belt = ((fx * 9.0).sin() * 0.5 + (fy * 7.0 + fx * 3.0).sin() * 0.5).abs();
                buf.elevation[i] = (belt.powf(3.0) * 0.75 + (((x * 7 + y * 13) % 23) as f32) / 300.0)
                    .clamp(0.0, 1.0);
                buf.koppen[i] = if y < gh / 8 || y > gh * 7 / 8 { crate::sim::koppen::ET }
                                else if y < gh / 3 { crate::sim::koppen::BWH }
                                else { crate::sim::koppen::CFB };
            }
        }
        let towns: Vec<Settlement> = (0..120)
            .map(|i| settle(&format!("t{i}"), (i * 37 % gw as usize) as u32,
                            (i * 53 % gh as usize) as u32, 20_000 - i as u32 * 100))
            .collect();
        let t0 = Instant::now();
        let (provs, _) = generate_provinces(&buf, &[], &[], &towns, 0.5);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        // Spread WITHIN one climate band, at mirrored latitudes — comparing every
        // province on the map would mostly measure the deliberate climate-size ladder.
        let mut areas: Vec<f64> = provs.iter()
            .filter(|p| p.koppen == crate::sim::koppen::CFB
                && p.seat_y > gh * 40 / 100 && p.seat_y < gh * 60 / 100)
            .map(|p| p.area_km2 as f64).collect();
        areas.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let (p10, med, p90) = (areas[areas.len() / 10], areas[areas.len() / 2], areas[areas.len() * 9 / 10]);
        eprintln!("[province-bench {gw}×{gh} = {} cells] {ms:.0} ms → {} provinces · \
                   temperate-band n={} p10 {p10:.0} median {med:.0} p90 {p90:.0} \
                   (p90/p10 {:.2}, min/med {:.2})",
                  gw as usize * gh as usize, provs.len(), areas.len(),
                  p90 / p10.max(1.0), areas[0] / med.max(1.0));
    }

    /// **Climate sets the size.** An arid band must hold provinces several times the area
    /// of the temperate band beside it (Saharan district vs. Italian county) — while both
    /// stay inside their own size band, which is what separates this from the old
    /// "collapse the desert into one blob" rule.
    #[test]
    fn arid_provinces_are_larger_than_temperate_ones() {
        let (gw, gh) = (480u32, 240u32);
        let mut buf = blank_world_sized(gw, gh);
        // Two land bands at MIRRORED latitudes — one hot desert, one oceanic temperate —
        // with ocean elsewhere. Mirroring matters: province area is measured in real km²,
        // so a band nearer a pole holds geographically smaller cells and the comparison
        // would be measuring latitude rather than climate.
        let (arid_lo, arid_hi) = (gh * 25 / 100, gh * 40 / 100);
        let (temp_lo, temp_hi) = (gh * 60 / 100, gh * 75 / 100);
        for y in 0..gh {
            let arid = y >= arid_lo && y < arid_hi;
            let temperate = y >= temp_lo && y < temp_hi;
            for x in 0..gw {
                let i = buf.idx(x, y);
                buf.terrain[i] = if arid || temperate { 1 } else { 0 };
                buf.elevation[i] = (((x * 7 + y * 13) % 11) as f32) / 60.0;
                buf.koppen[i] = if arid { crate::sim::koppen::BWH } else { crate::sim::koppen::CFB };
            }
        }
        let towns = vec![settle("a", 30, temp_lo + 4, 9000), settle("b", 300, arid_lo + 4, 7000)];
        let (provs, _) = generate_provinces(&buf, &[], &[], &towns, 0.0);
        assert!(provs.len() > 12, "need a populated map, got {}", provs.len());

        let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
        let arid: Vec<f64> = provs.iter()
            .filter(|p| p.koppen == crate::sim::koppen::BWH)
            .map(|p| p.area_km2 as f64).collect();
        let temperate: Vec<f64> = provs.iter()
            .filter(|p| p.koppen == crate::sim::koppen::CFB)
            .map(|p| p.area_km2 as f64).collect();
        assert!(arid.len() >= 3 && temperate.len() >= 3,
            "need both climates represented (arid {} · temperate {})", arid.len(), temperate.len());
        let (ma, mt) = (mean(&arid), mean(&temperate));
        eprintln!("mean area: arid {ma:.0} km² ({} provinces) · temperate {mt:.0} km² ({} provinces) — ratio {:.2}",
                  arid.len(), temperate.len(), ma / mt.max(1.0));
        assert!(ma > 2.0 * mt,
            "the arid band should hold much larger provinces (arid {ma:.0} vs temperate {mt:.0})");
        assert!(ma < 12.0 * mt,
            "the arid band ran away to near-continental provinces (arid {ma:.0} vs temperate {mt:.0})");
        // …but a desert is still a set of provinces, not one blob covering the band.
        assert!(arid.len() >= 4, "the desert collapsed into {} province(s)", arid.len());
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
