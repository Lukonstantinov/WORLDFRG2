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
    /// True when this segment is a TRIBUTARY that ends where it joins a larger
    /// stream (its last point is the confluence cell), rather than a trunk that
    /// reaches the sea. Drives confluence detection (settlement magnets) and a
    /// paler render shade for the branches.
    #[serde(default)]
    pub tributary: bool,
    /// Strahler stream order (1 = headwater creek, higher = larger trunk). Lets
    /// downstream systems (rendering, settlement scoring) rank branches by size
    /// without re-deriving the drainage tree.
    #[serde(default)]
    pub order: u8,
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

/// Tiny elevation increment applied to every cell as the priority flood spills
/// across it (Barnes "Priority-Flood+ε"). Raising each newly reached cell to
/// *just above* its spill parent removes flats entirely: the filled surface is
/// strictly monotonic toward the sea, so steepest-descent drainage is always
/// well-defined and points down the true outlet direction instead of following
/// the flood's BFS ring (which cut straight across basins — the old "rivers
/// don't follow the terrain" artefact). 1e-6 in normalized-elevation units is
/// ~9 mm of the 8848 m range, negligible against relief and the ~18 m lake
/// threshold, but enough to survive f32 rounding across long flats.
const FILL_EPS: f32 = 1e-6;

/// Priority-flood hydrology (Barnes et al. + ε): fill depressions, then assign a
/// STEEPEST-DESCENT drainage direction on the filled surface and accumulate flow
/// downstream.
///
/// The flood grows outward from the sea (the only outlet) in order of increasing
/// filled elevation, raising each newly reached cell to just above the spill
/// level (`FILL_EPS`). This guarantees every land cell has a strictly monotonic
/// path to the sea, and any cell raised meaningfully above its true elevation
/// marks a filled depression — i.e. a lake. Flow direction is then taken as the
/// lowest of the eight filled neighbours, so on genuine slopes it tracks the
/// real gradient and across former flats it points down the ε-tilt toward the
/// outlet.
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
            // Raise to just above the spill level (ε-tilt) so the filled surface
            // is strictly increasing away from the outlet — no flats to strand
            // the drainage direction on. `flow_dir` here is provisional (the spill
            // parent); it is overwritten by the steepest-descent pass below.
            let raised = buf.elevation[ni].max(c_filled + FILL_EPS);
            filled[ni] = raised;
            flow_dir[ni] = if c_is_sea { FLOW_SEA } else { idx as i32 };
            heap.push(FloodCell { elev: raised, seq, idx: ni });
            seq += 1;
        }
    }

    // ── Steepest-descent drainage on the filled surface ──────────────────────
    // Replace the flood's spill-tree parent with the genuinely lowest of the
    // eight filled neighbours. On real slopes this follows the terrain gradient;
    // on ε-tilted former flats the lowest neighbour is the one toward the outlet.
    // A sea neighbour (filled == 0) always wins, so coastal cells drain to the
    // sea. The ε guarantee means every reached land cell has a strictly lower
    // neighbour, so the direction is defined and the graph stays acyclic.
    for &idx in &order {
        if buf.terrain[idx] != 1 { continue; }
        let cx = (idx % w as usize) as i32;
        let cy = (idx / w as usize) as i32;
        let c_filled = filled[idx];
        let mut best_dir = FLOW_SINK;
        let mut best_filled = c_filled;
        for &(dx, dy) in &D8 {
            let nx = buf.wrap_x(cx + dx);
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let ni = buf.idx(nx, ny as u32);
            if filled[ni] < best_filled {
                best_filled = filled[ni];
                best_dir = if buf.terrain[ni] == 0 { FLOW_SEA } else { ni as i32 };
            }
        }
        flow_dir[idx] = best_dir;
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
    density: f32, width_scale: f32, lakes: &[Lake],
) -> Vec<River> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();
    let density = density.clamp(0.0, 1.5);
    let width_scale = width_scale.clamp(0.2, 2.0);

    // Cells that belong to a lake are OPEN WATER, not channel: a river must stop
    // where it meets a lake and resume at the outlet, never draw a line straight
    // across the basin (the "river crosses the lake" artefact). Flow accumulation
    // still passes through the filled basin, so the outflow reach is sized by the
    // full upstream catchment — the river, the lake and its outflow read as one
    // connected system, just not one line over the water.
    let mut is_lake = vec![false; total];
    for lk in lakes {
        for &(lx, ly) in &lk.cells {
            is_lake[buf.idx(lx, ly)] = true;
        }
    }
    let area_ratio = (w * h) as f32 / 64800.0;
    let base = (40.0 * area_ratio.sqrt()).max(20.0);
    // density 0 → 3× threshold (sparse), 0.5 → 1.7×, 1 → 0.4×, 1.5 → 0.1× (very
    // dense — many small tributaries become rivers). Floor low so the high end
    // really does spawn a large number of small streams.
    let density_mult = (3.0 - 2.6 * density).max(0.1);
    let threshold = (base * density_mult).max(4.0) as u32;

    // ── Build the channel network ────────────────────────────────────────────
    // A channel cell is land (not a permanent ice cap) carrying at least
    // `threshold` accumulated flow. These form drainage FORESTS rooted at the
    // sea. We walk them as trees so every TRIBUTARY is emitted as its own
    // segment, not silently discarded at each confluence (the old tracer only
    // ever followed the single largest upstream branch — hence "no tributaries").
    let is_channel = |i: usize| -> bool {
        buf.terrain[i] == 1
            && !is_lake[i]
            && buf.koppen[i] != crate::sim::koppen::EF
            && acc[i] >= threshold
    };

    // For each channel cell D: its channel inflow count, and `main_child` = the
    // upstream channel carrying the most flow (the main-stem continuation). An
    // upstream channel that is NOT the main child is the head of a tributary.
    let mut main_child = vec![-1i32; total];
    let mut up_count = vec![0u32; total];
    for i in 0..total {
        if !is_channel(i) { continue; }
        let x = (i % w as usize) as i32;
        let y = (i / w as usize) as i32;
        let (mut best, mut best_ni) = (0u32, -1i32);
        for &(dx, dy) in &D8 {
            let nx = buf.wrap_x(x + dx);
            let ny = y + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let ni = buf.idx(nx, ny as u32);
            if !is_channel(ni) || flow_dir[ni] != i as i32 { continue; }
            up_count[i] += 1;
            if acc[ni] > best || (acc[ni] == best && (ni as i32) > best_ni) {
                best = acc[ni];
                best_ni = ni as i32;
            }
        }
        main_child[i] = best_ni;
    }

    // Walk one polyline per SOURCE (a channel cell with no channel inflow). Each
    // walk runs downstream while it stays the main branch, ending either at the
    // sea (a trunk river) or at the confluence where it merges into a larger
    // stream (a tributary — the junction cell is appended so the line connects).
    let mut rivers = Vec::new();
    let major_len = (w as f32 * 0.10).max(60.0);
    let use_koppen = !buf.koppen.is_empty();
    for s in 0..total {
        if !is_channel(s) || up_count[s] != 0 { continue; }

        let mut points: Vec<(u32, u32)> = Vec::new();
        let mut cur = s;
        let mut is_mouth = false;
        loop {
            points.push(((cur % w as usize) as u32, (cur / w as usize) as u32));
            let dir = flow_dir[cur];
            if dir == FLOW_SEA { is_mouth = true; break; }
            if dir < 0 { break; }
            let nxt = dir as usize;
            if !is_channel(nxt) { break; }
            if main_child[nxt] != cur as i32 {
                // Merge point into a larger stream: append the junction and stop.
                points.push(((nxt % w as usize) as u32, (nxt / w as usize) as u32));
                break;
            }
            cur = nxt;
        }
        if points.len() < 3 { continue; }

        // Outlet = this segment's own downstream-most channel cell (the mouth for
        // a trunk, or the cell just before the confluence for a tributary), so a
        // tributary is sized by ITS OWN discharge, not the trunk it joins.
        let outlet = if is_mouth {
            buf.idx(points[points.len() - 1].0, points[points.len() - 1].1)
        } else {
            buf.idx(points[points.len() - 2].0, points[points.len() - 2].1)
        };
        let outlet_acc = acc[outlet] as f32;

        // ── Physical river width = DISCHARGE ── drainage area × runoff, cut back
        // in arid climates (a desert river is a thin wadi even with a big
        // catchment). Log-scaled so a great trunk reads distinctly wider than a
        // headwater stream while staying inside a narrow render band.
        let length = points.len() as f32;
        let (mut psum, mut arid) = (0.0f32, 0.0f32);
        for &(px, py) in &points {
            let pi = buf.idx(px, py);
            psum += buf.precipitation[pi];
            if use_koppen && matches!(
                buf.koppen[pi],
                crate::sim::koppen::BWH | crate::sim::koppen::BWK
                    | crate::sim::koppen::BSH | crate::sim::koppen::BSK
            ) { arid += 1.0; }
        }
        let mean_p = psum / length.max(1.0);
        let runoff = (mean_p / 700.0).clamp(0.2, 2.2);
        let arid_frac = arid / length.max(1.0);
        let discharge = outlet_acc * runoff * (1.0 - 0.45 * arid_frac);
        let len_term = (length / (220.0 * area_ratio.sqrt())).min(1.5) * 0.4;
        let raw = (discharge / threshold as f32).max(0.0).ln_1p() * 0.62 + 0.7 + len_term;
        let width = (raw * width_scale).clamp(0.8, 3.4);

        // Strahler-ish order from discharge (1 = creek). Ranks branches for
        // rendering / settlement scoring without a full stream-order pass.
        let order = (((outlet_acc / threshold as f32).max(1.0).log2().floor() as i32) + 1)
            .clamp(1, 7) as u8;
        // MAJOR = a long trunk OR a high-order channel → darker render shade.
        let major = length >= major_len || order >= 4;
        // Navigable = enough discharge to float a barge (a real inland highway).
        let navigable = discharge >= threshold as f32 * 5.0 && width >= 1.8;
        let tributary = !is_mouth;

        // ── Mouth landform (trunks only): depositional DELTA on a flat shallow
        // coast vs drowned ESTUARY on a steeper one. ──
        let mut mouth_kind = 0u8;
        let mut delta: Vec<(u32, u32)> = Vec::new();
        if is_mouth && outlet_acc >= threshold as f32 * 3.0 {
            let (mx, my) = ((outlet % w as usize) as i32, (outlet / w as usize) as i32);
            let mut shelf_cnt = 0i32;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    if dx * dx + dy * dy > 9 { continue; }
                    let nx = buf.wrap_x(mx + dx);
                    let ny = my + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let ni = buf.idx(nx, ny as u32);
                    if buf.terrain[ni] == 0 && (buf.is_shelf[ni] == 1 || buf.sea_depth[ni] < 0.12) {
                        shelf_cnt += 1;
                        if delta.len() < 24 { delta.push((nx, ny as u32)); }
                    }
                }
            }
            mouth_kind = if shelf_cnt >= 6 { 1 } else { 2 };
            if mouth_kind == 2 { delta.clear(); }
        }

        rivers.push(River { points, width, major, navigable, mouth_kind, delta, tributary, order });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};

    /// Build a bare WorldBuffer with just the columns hydrology/extraction read.
    fn synth(w: u32, h: u32, terrain: Vec<u8>, elevation: Vec<f32>) -> WorldBuffer {
        let n = (w * h) as usize;
        let sea_depth: Vec<f32> = (0..n).map(|i| if terrain[i] == 0 { 0.05 } else { 0.0 }).collect();
        let is_shelf: Vec<u8> = (0..n).map(|i| (terrain[i] == 0) as u8).collect();
        WorldBuffer {
            cols: ColumnSet::ALL,
            width: w, height: h,
            tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0,
            terrain,
            elevation,
            sea_depth,
            is_shelf,
            is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(),
            plate_index: Vec::new(),
            boundary_type: Vec::new(),
            is_volcanic: Vec::new(),
            temperature: Vec::new(),
            precipitation: vec![1200.0f32; n], // wet, so channels form
            koppen: vec![super::super::koppen::CFB; n], // temperate, not EF/arid
            soil_type: Vec::new(),
            fertility: Vec::new(),
            fishery: Vec::new(),
            current_type: Vec::new(),
            wind_vx: Vec::new(), wind_vy: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(),
            distance_to_ocean: Vec::new(),
            habitability: Vec::new(),
            salinity: Vec::new(),
            shark_risk: Vec::new(),
            goods: Vec::new(),
            shipworm_risk: Vec::new(),
            storm_base: Vec::new(),
            reef_risk: Vec::new(),
            disease_risk: Vec::new(),
        }
    }

    /// Item 2: every drainage step must go DOWNHILL on the filled surface — the
    /// flow direction follows the elevation, never routes uphill.
    #[test]
    fn flow_follows_elevation_downhill() {
        let (w, h) = (40u32, 20u32);
        let n = (w * h) as usize;
        let mut terrain = vec![1u8; n];
        let mut elev = vec![0.0f32; n];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if x == 0 { terrain[i] = 0; } // sea on the west edge
                // Gentle west-draining slope + a central valley so streams converge.
                elev[i] = 0.02 + x as f32 * 0.02 + ((y as i32 - h as i32 / 2).abs() as f32) * 0.015;
            }
        }
        let buf = synth(w, h, terrain, elev);
        let hy = compute_hydrology(&buf);
        for i in 0..n {
            if buf.terrain[i] != 1 { continue; }
            let d = hy.flow_dir[i];
            if d >= 0 {
                assert!(hy.filled[d as usize] < hy.filled[i],
                    "cell {i} drains uphill: {} -> {}", hy.filled[i], hy.filled[d as usize]);
            }
        }
    }

    /// Item 1: a valley basin fed from both flanks must yield BOTH a trunk that
    /// reaches the sea AND tributary segments (the old tracer produced only the
    /// single trunk).
    #[test]
    fn extraction_emits_tributaries() {
        let (w, h) = (60u32, 30u32);
        let n = (w * h) as usize;
        let mut terrain = vec![1u8; n];
        let mut elev = vec![0.0f32; n];
        let y0 = h as f32 / 2.0;
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if x == 0 { terrain[i] = 0; }
                // A single main valley along y0 draining west, with sloped flanks
                // so every flank cell drains into the central trunk (tributaries).
                elev[i] = 0.02 + x as f32 * 0.015 + ((y as f32 - y0).abs()) * 0.02;
            }
        }
        let buf = synth(w, h, terrain, elev);
        let hy = compute_hydrology(&buf);
        let rivers = extract_rivers(&buf, &hy.flow_dir, &hy.acc, 1.2, 1.0, &[]);
        let trunks = rivers.iter().filter(|r| !r.tributary).count();
        let tribs = rivers.iter().filter(|r| r.tributary).count();
        assert!(trunks >= 1, "expected at least one trunk reaching the sea");
        assert!(tribs >= 1, "expected tributary segments, got {} rivers ({} trunks)",
            rivers.len(), trunks);
    }
}
