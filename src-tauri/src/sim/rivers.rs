use super::world_buffer::WorldBuffer;
use std::collections::{BinaryHeap, VecDeque};
use std::cmp::Ordering;

/// River data extracted from flow simulation
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct River {
    pub points: Vec<(u32, u32)>,
    pub width: f32,
    /// True once the river has run far enough to count as a MAJOR river (a long
    /// continental trunk). Drives a darker render shade so the eye can pick out
    /// the great rivers from the thin headwater streams. Set purely from length,
    /// not discharge, so the colour change marks where a stream "becomes" a river.
    #[serde(default)]
    pub major: bool,
    /// Big enough to carry boats (a trade artery), set from discharge/width.
    #[serde(default)]
    pub navigable: bool,
    /// Mouth landform: 0 = plain/inland, 1 = delta (depositional fan on a flat
    /// shallow coast), 2 = estuary (drowned tidal mouth on a steeper/deeper coast).
    #[serde(default)]
    pub mouth_kind: u8,
    /// Distributary / wetland cells of a delta fan (rendered as braided water +
    /// marsh). Empty for non-delta rivers.
    #[serde(default)]
    pub delta: Vec<(u32, u32)>,
}

/// Lake data
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Lake {
    pub cells: Vec<(u32, u32)>,
    pub elevation: f32,
}

/// D8 flow directions (index of downhill neighbor, or special values)
const FLOW_SEA: i32 = -1;
const FLOW_SINK: i32 = -2;

/// D8 neighbor offsets
const D8: [(i32, i32); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];

/// Result of the priority-flood hydrology pass.
pub struct Hydrology {
    /// Per-cell drainage direction: FLOW_SEA, FLOW_SINK, or a downstream cell index.
    pub flow_dir: Vec<i32>,
    /// Upstream contributing-cell count (flow accumulation).
    pub acc: Vec<u32>,
    /// Depression-filled elevation surface (≥ original elevation).
    pub filled: Vec<f32>,
}

/// Min-heap entry for the priority flood.
/// `BinaryHeap` is a max-heap, so `Ord` is inverted: the lowest elevation
/// (and, among ties, the earliest-enqueued cell) compares as "greatest" and
/// is therefore popped first. The sequence tiebreak gives flats a consistent
/// gradient toward the outlet instead of pooling.
struct FloodCell {
    elev: f32,
    seq: u64,
    idx: usize,
}
impl PartialEq for FloodCell {
    fn eq(&self, o: &Self) -> bool {
        self.elev == o.elev && self.seq == o.seq
    }
}
impl Eq for FloodCell {}
impl Ord for FloodCell {
    fn cmp(&self, o: &Self) -> Ordering {
        match o.elev.partial_cmp(&self.elev) {
            Some(Ordering::Equal) | None => o.seq.cmp(&self.seq),
            Some(ord) => ord,
        }
    }
}
impl PartialOrd for FloodCell {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Priority-flood hydrology (Barnes et al.): fill depressions and assign a
/// drainage direction to every land cell in a single pass, then accumulate
/// flow downstream.
///
/// The flood grows outward from the sea (the only outlet) in order of
/// increasing filled elevation. When a cell is first reached it drains to the
/// cell it was reached from, and its surface is raised to at least the spill
/// elevation. This guarantees every land cell has a monotonic path to the sea
/// (no spurious internal sinks), and any cell raised above its true elevation
/// marks a filled depression — i.e. a lake.
pub fn compute_hydrology(buf: &WorldBuffer) -> Hydrology {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();

    let mut filled = vec![0.0f32; total];
    let mut flow_dir = vec![FLOW_SINK; total];
    let mut visited = vec![false; total];
    let mut heap: BinaryHeap<FloodCell> = BinaryHeap::new();
    let mut seq: u64 = 0;
    let mut order: Vec<usize> = Vec::with_capacity(total);

    // Seed the flood with every sea cell — these are the drainage outlets.
    for i in 0..total {
        filled[i] = buf.elevation[i];
        if buf.terrain[i] == 0 {
            visited[i] = true;
            flow_dir[i] = FLOW_SEA;
            heap.push(FloodCell { elev: filled[i], seq, idx: i });
            seq += 1;
        }
    }

    while let Some(FloodCell { idx, .. }) = heap.pop() {
        order.push(idx);
        let cx = (idx % w as usize) as i32;
        let cy = (idx / w as usize) as i32;
        let c_is_sea = buf.terrain[idx] == 0;
        let c_filled = filled[idx];

        for &(dx, dy) in &D8 {
            let nx = buf.wrap_x(cx + dx);
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let ni = buf.idx(nx, ny as u32);
            if visited[ni] { continue; }
            if buf.terrain[ni] != 1 { continue; } // only land drains

            visited[ni] = true;
            // Raise to the spill level if the original terrain dips below it.
            let raised = buf.elevation[ni].max(c_filled);
            filled[ni] = raised;
            flow_dir[ni] = if c_is_sea { FLOW_SEA } else { idx as i32 };
            heap.push(FloodCell { elev: raised, seq, idx: ni });
            seq += 1;
        }
    }

    // Flow accumulation: each land cell contributes itself, propagated
    // downstream. `order` is outlet-first, so iterating it in reverse visits
    // every cell before its (earlier-popped) downstream target.
    // Ice-cap (Köppen EF) cells are permanent ice sheets: their precipitation is
    // locked up as ice rather than liquid runoff, so they contribute nothing to
    // flow accumulation. A drainage basin of pure ice therefore stays below the
    // river threshold (no polar rivers); mixed basins are truncated at the ice
    // margin by `extract_rivers`.
    let mut acc = vec![0u32; total];
    for i in 0..total {
        if buf.terrain[i] == 1 && buf.koppen[i] != crate::sim::koppen::EF { acc[i] = 1; }
    }
    for &idx in order.iter().rev() {
        if buf.terrain[idx] != 1 { continue; }
        let dir = flow_dir[idx];
        if dir >= 0 {
            acc[dir as usize] += acc[idx];
        }
    }

    Hydrology { flow_dir, acc, filled }
}

/// Extract river networks from flow accumulation.
/// Returns rivers as ordered point sequences with width.
///
/// `density` (0..1): how readily a channel becomes a river. Low = only major
/// trunk rivers (high accumulation threshold), high = many small tributaries.
/// `width_scale` (0.2..2): multiplies rendered river width and lowers the cap so
/// rivers can be made thinner.
pub fn extract_rivers(
    buf: &WorldBuffer, flow_dir: &[i32], acc: &[u32],
    density: f32, width_scale: f32,
) -> Vec<River> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();
    let density = density.clamp(0.0, 1.5);
    let width_scale = width_scale.clamp(0.2, 2.0);
    let area_ratio = (w * h) as f32 / 64800.0;
    let base = (40.0 * area_ratio.sqrt()).max(20.0);
    // density 0 → 3× threshold (sparse), 0.5 → 1.7×, 1 → 0.4×, 1.5 → 0.1× (very
    // dense — many small tributaries become rivers). Floor low so the high end
    // really does spawn a large number of small streams.
    let density_mult = (3.0 - 2.6 * density).max(0.1);
    let threshold = (base * density_mult).max(4.0) as u32;

    // Find river mouths: land cells that drain to sea with high accumulation
    let mut mouths: Vec<(usize, u32)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            if buf.koppen[idx] == crate::sim::koppen::EF { continue; } // no river off an ice cap
            if acc[idx] < threshold { continue; }
            if flow_dir[idx] != FLOW_SEA { continue; }
            mouths.push((idx, acc[idx]));
        }
    }

    // Sort by accumulation (biggest first)
    mouths.sort_by(|a, b| b.1.cmp(&a.1));

    // Trace each river upstream
    let mut claimed = vec![false; total];
    let mut rivers = Vec::new();

    for (mouth_idx, _) in &mouths {
        let mut points = Vec::new();
        let mut current = *mouth_idx;

        // Trace upstream: find the upstream cell that flows into current
        loop {
            if claimed[current] { break; }
            // Truncate at an ice sheet: rivers stop at the ice margin, never
            // tracing up into a permanent ice cap.
            if buf.koppen[current] == crate::sim::koppen::EF { break; }
            claimed[current] = true;

            let x = (current % w as usize) as u32;
            let y = (current / w as usize) as u32;
            points.push((x, y));

            // Find best upstream neighbor
            let mut best_upstream = None;
            let mut best_acc = 0u32;

            for &(dx, dy) in &D8 {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ny = ny as u32;
                let ni = buf.idx(nx, ny);
                if flow_dir[ni] == current as i32 && acc[ni] > best_acc && acc[ni] >= threshold / 2 {
                    best_acc = acc[ni];
                    best_upstream = Some(ni);
                }
            }

            match best_upstream {
                Some(ni) => current = ni,
                None => break,
            }
        }

        if points.len() >= 3 {
            let mouth_acc = acc[*mouth_idx] as f32;
            // ── Physical river width = DISCHARGE, not just drainage area ──
            // Discharge ≈ drainage area × runoff, where runoff scales with the
            // basin's precipitation and is cut back in arid climates (the river
            // loses water to evaporation/infiltration — a desert river is a thin
            // wadi even with a big catchment). River length adds a little (a long
            // trunk has gathered more). The width_scale slider is no longer the
            // driver (the user asked for physics, not a slider): it only trims the
            // overall look.
            let length = points.len() as f32;
            let (mut psum, mut arid) = (0.0f32, 0.0f32);
            for &(px, py) in &points {
                let pi = buf.idx(px, py);
                psum += buf.precipitation[pi];
                if matches!(
                    buf.koppen[pi],
                    crate::sim::koppen::BWH | crate::sim::koppen::BWK
                        | crate::sim::koppen::BSH | crate::sim::koppen::BSK
                ) { arid += 1.0; }
            }
            let mean_p = psum / length.max(1.0);              // mm/yr along the stem
            let runoff = (mean_p / 700.0).clamp(0.2, 2.2);    // wet basin → more flow
            let arid_frac = arid / length.max(1.0);
            let discharge = mouth_acc * runoff * (1.0 - 0.45 * arid_frac);
            let len_term = (length / (220.0 * area_ratio.sqrt())).min(1.5) * 0.5;
            // Width kept in a NARROW band: every river renders 1–3 px wide (per the
            // user's rule). We still compute a physical discharge so the *relative*
            // ordering is sane, but squash it into [0.8, 3.0] so no river ever
            // renders as a fat blob.
            let raw = (discharge / threshold as f32).sqrt() * 1.1 + len_term + 0.5;
            let width = (raw * width_scale).clamp(0.8, 3.0);
            // MAJOR river = a long trunk. Threshold scales with map size (~10% of
            // the world width, floored) so it's resolution-independent: a great
            // river that has gathered a long course flips to the darker shade.
            let major_len = (w as f32 * 0.10).max(60.0);
            let major = length >= major_len;
            points.reverse(); // source to mouth

            // ── Mouth landform: delta vs estuary, + navigability ──
            // A big river hits the sea as either a depositional DELTA (a fan of
            // distributary channels + marsh on a flat, shallow coast) or a drowned
            // ESTUARY (a single deeper tidal mouth on a steeper coast).
            let navigable = width >= 2.6;
            let mut mouth_kind = 0u8;
            let mut delta: Vec<(u32, u32)> = Vec::new();
            let big = mouth_acc >= threshold as f32 * 3.0;
            if big {
                let (mx, my) = (*mouth_idx % w as usize, *mouth_idx / w as usize);
                let (mx, my) = (mx as i32, my as i32);
                // Gather nearby shallow/shelf sea cells (the platform a delta builds
                // on); count them to judge how flat & shallow the coast is.
                let mut shelf_cnt = 0i32;
                for dy in -3i32..=3 {
                    for dx in -3i32..=3 {
                        if dx * dx + dy * dy > 9 { continue; }
                        let nx = buf.wrap_x(mx + dx);
                        let ny = my + dy;
                        if ny < 0 || ny >= h as i32 { continue; }
                        let ni = buf.idx(nx, ny as u32);
                        if buf.terrain[ni] == 0
                            && (buf.is_shelf[ni] == 1 || buf.sea_depth[ni] < 0.12)
                        {
                            shelf_cnt += 1;
                            if delta.len() < 24 { delta.push((nx, ny as u32)); }
                        }
                    }
                }
                // Flat shallow shelf around the mouth → delta; otherwise a deeper,
                // narrower opening → estuary.
                mouth_kind = if shelf_cnt >= 6 { 1 } else { 2 };
                if mouth_kind == 2 { delta.clear(); } // estuaries aren't drawn as a fan
            }

            rivers.push(River { points, width, major, navigable, mouth_kind, delta });
        }
    }

    rivers
}

/// Detect lakes from the depression-filled surface.
/// A lake is a connected region of land whose filled elevation sits
/// meaningfully above its true terrain (i.e. a filled depression).
///
/// `fill_depth` (0..1, in normalized-elevation units): minimum fill before a
/// depression counts — raising it keeps only genuinely deep basins, shrinking
/// the lake count/extent. `max_cells`: connected basins larger than this are
/// dropped (run-away flooded interiors that read as implausible inland seas).
pub fn detect_lakes(buf: &WorldBuffer, filled: &[f32], fill_depth: f32, max_cells: usize) -> Vec<Lake> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();
    // Default ~18 m (0.002) of fill; configurable so the user can suppress
    // shallow puddles or, conversely, allow more lakes.
    let eps = fill_depth.clamp(0.0005, 0.05);

    let mut visited = vec![false; total];
    let mut lakes = Vec::new();

    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            if visited[idx] { continue; }
            if filled[idx] - buf.elevation[idx] <= eps { continue; }

            // BFS the connected filled region
            let mut cells = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back((x, y));
            visited[idx] = true;
            let mut sum_elev = 0.0f32;

            while let Some((cx, cy)) = queue.pop_front() {
                let ci = buf.idx(cx, cy);
                cells.push((cx, cy));
                sum_elev += filled[ci];

                for &(dx, dy) in &D8 {
                    let nx = buf.wrap_x(cx as i32 + dx);
                    let ny = cy as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let ny = ny as u32;
                    let ni = buf.idx(nx, ny);
                    if visited[ni] { continue; }
                    if buf.terrain[ni] != 1 { continue; }
                    if filled[ni] - buf.elevation[ni] <= eps { continue; }
                    visited[ni] = true;
                    queue.push_back((nx, ny));
                }
            }

            // Keep only plausibly-sized lakes: at least 2 cells, no larger than
            // `max_cells` (giant flooded basins are usually artefacts).
            if cells.len() >= 2 && cells.len() <= max_cells {
                let elevation = sum_elev / cells.len() as f32;
                lakes.push(Lake { cells, elevation });
            }
        }
    }

    lakes
}
