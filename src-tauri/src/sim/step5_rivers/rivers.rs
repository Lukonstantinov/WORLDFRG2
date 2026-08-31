use crate::sim::world_buffer::WorldBuffer;
use crate::sim::elevation::fbm_noise;
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
    /// Render-time meander amplitude scale in 0..1, set from the reach's mean
    /// gradient: ~0 for steep alpine/headwater reaches (the channel is pinned to
    /// the valley floor and runs nearly straight down the fall line) and ~1 for
    /// flat lowland floodplains (where a river is free to wander in broad bends).
    /// The frontend multiplies its cosmetic meander offset by this so rivers stop
    /// wiggling across the highlands and only meander where they physically would.
    /// Defaults to 1.0 so pre-existing saves render exactly as before.
    #[serde(default = "one_f32")]
    pub meander: f32,
    /// TRUE meander geometry â€” a smoothed, sub-cell render polyline (cell-index
    /// coordinates, same convention as `points`) that winds through the flat
    /// lowlands and runs straight down steep headwaters. Computed physically in
    /// `build_meander_path` (per-vertex gradient â†’ slowness, amplitude clamped to
    /// the valley floor, wavelength âˆ channel width, noise-jittered so bends read
    /// organic). The frontend draws THIS when present; `points` stays the true
    /// cell path used by settlement/ecology logic. Empty on old saves â†’ the
    /// frontend falls back to its cosmetic meander of `points`.
    #[serde(default)]
    pub render: Vec<(f32, f32)>,
    /// BRAIDED anabranches â€” thin secondary channels that split off and rejoin the
    /// main stem on the widest, flattest reaches of a great river (the
    /// sandbar-island braid of the lower Mississippi / Brahmaputra). Each is a
    /// short sub-cell strand (cell-index coords) drawn faint beside the trunk.
    /// Empty for small/steep rivers and old saves.
    #[serde(default)]
    pub braids: Vec<Vec<(f32, f32)>>,
}

fn one_f32() -> f32 { 1.0 }

/// Lake data
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Lake {
    pub cells: Vec<(u32, u32)>,
    pub elevation: f32,
    /// Lake origin: 0 = a normal depression-filled basin (`detect_lakes`), 1 = an
    /// OXBOW / backwater cut off from a meandering river (`extract_oxbows`). Drives
    /// a distinct render + its own limnology (still, weedy backwater ecology).
    #[serde(default)]
    pub kind: u8,
    /// True terminal SALT lake â€” an arid basin with no river outflow, where the sun
    /// alone empties it and brine concentrates (Caspian / Great Salt Lake / playa).
    /// Set by `classify_salt_lakes`; drives the pink brine tint, salt-good
    /// production and the salt-shore settlement draw.
    #[serde(default)]
    pub endorheic: bool,
    /// Approximate salinity in parts-per-thousand â€” ~0.2 for a freshwater lake,
    /// 12-120+ for a terminal salt lake (shallower & more arid = more concentrated).
    #[serde(default)]
    pub salinity_ppt: f32,
}

/// Salinity (PSU) window the persisted `salinity` u8 column encodes â€” must match
/// `query_commands` and `ocean.rs` so a salt-lake value round-trips consistently.
pub const SAL_MIN_PSU: f32 = 28.0;
pub const SAL_MAX_PSU: f32 = 42.0;
/// Encode a salinity in ppt into the u8 salinity-column value (clamped to window).
pub fn salinity_to_u8(ppt: f32) -> u8 {
    (((ppt - SAL_MIN_PSU) / (SAL_MAX_PSU - SAL_MIN_PSU)).clamp(0.0, 1.0) * 255.0).round() as u8
}
/// A terminal salt lake is considered salt-PRODUCING once its brine reaches at
/// least seawater strength (~35 ppt); milder brackish basins do not make salt.
pub const SALT_PRODUCTION_PPT: f32 = 35.0;

/// Classify each lake's SALINITY in place: a lake with no river outflow sitting in
/// an arid (BW/BS) basin is a terminal salt lake; the sun alone empties it and
/// brine concentrates (shallower â†’ stronger). Mirrors the Hydrology panel's
/// limnology so the map, the goods layer and the panel all agree. Freshwater lakes
/// are left at ~0.2 ppt. `rivers` supplies outflow detection (a river resuming at
/// the shore = the lake drains, so it is NOT terminal).
pub fn classify_salt_lakes(buf: &WorldBuffer, lakes: &mut [Lake], rivers: &[River]) {
    use crate::sim::koppen;
    let have_koppen = !buf.koppen.is_empty();
    // Cells at/near a lake, mapped to their lake index, for outflow attachment.
    let mut lake_of: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (li, lk) in lakes.iter().enumerate() {
        for &(x, y) in &lk.cells { lake_of.insert(buf.idx(x, y), li); }
    }
    let near_lake = |x: u32, y: u32| -> Option<usize> {
        if let Some(&l) = lake_of.get(&buf.idx(x, y)) { return Some(l); }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = y as i32 + dy;
            if ny < 0 || ny >= buf.height as i32 { continue; }
            if let Some(&l) = lake_of.get(&buf.idx(nx, ny as u32)) { return Some(l); }
        }
        None
    };
    // A river whose SOURCE resumes at a lake shore = that lake's outflow.
    let mut has_outflow = vec![false; lakes.len()];
    for r in rivers {
        if r.points.len() < 2 { continue; }
        let (sx, sy) = r.points[0];
        if let Some(l) = near_lake(sx, sy) { has_outflow[l] = true; }
    }
    for (li, lk) in lakes.iter_mut().enumerate() {
        let n = lk.cells.len().max(1);
        let mut arid_cells = 0usize;
        let mut max_depth = 0.0f32;
        for &(x, y) in &lk.cells {
            let ci = buf.idx(x, y);
            if have_koppen && matches!(buf.koppen[ci],
                koppen::BWH | koppen::BWK | koppen::BSH | koppen::BSK) { arid_cells += 1; }
            max_depth = max_depth.max((lk.elevation - buf.elevation[ci]).max(0.0));
        }
        let arid = arid_cells as f32 / n as f32 > 0.4;
        let endorheic = arid && !has_outflow[li];
        lk.endorheic = endorheic;
        lk.salinity_ppt = if endorheic {
            let depth_m = (max_depth * 8848.0 * 3.0 + 2.0).clamp(2.0, 1600.0);
            crate::sim::aquatic::lake_salinity_ppt(
                crate::sim::aquatic::LakeKind::SaltEndorheic, depth_m)
        } else {
            0.2
        };
    }
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
    /// Depression-filled elevation surface (â‰¥ original elevation).
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
/// across it (Barnes "Priority-Flood+Îµ"). Raising each newly reached cell to
/// *just above* its spill parent removes flats entirely: the filled surface is
/// strictly monotonic toward the sea, so steepest-descent drainage is always
/// well-defined and points down the true outlet direction instead of following
/// the flood's BFS ring (which cut straight across basins â€” the old "rivers
/// don't follow the terrain" artefact). 1e-6 in normalized-elevation units is
/// ~9 mm of the 8848 m range, negligible against relief and the ~18 m lake
/// threshold, but enough to survive f32 rounding across long flats.
const FILL_EPS: f32 = 1e-6;

/// Priority-flood hydrology (Barnes et al. + Îµ): fill depressions, then assign a
/// STEEPEST-DESCENT drainage direction on the filled surface and accumulate flow
/// downstream.
///
/// The flood grows outward from the sea (the only outlet) in order of increasing
/// filled elevation, raising each newly reached cell to just above the spill
/// level (`FILL_EPS`). This guarantees every land cell has a strictly monotonic
/// path to the sea, and any cell raised meaningfully above its true elevation
/// marks a filled depression â€” i.e. a lake. Flow direction is then taken as the
/// lowest of the eight filled neighbours, so on genuine slopes it tracks the
/// real gradient and across former flats it points down the Îµ-tilt toward the
/// outlet.
/// A cheap, deterministic per-world seed for the flat-plain micro-relief field
/// (Slice 2), derived from the elevation field itself rather than threading a
/// `seed` parameter through `compute_hydrology`'s many call sites. Sampled at a
/// stride so it stays O(n/32), not a full-grid pass; two worlds with different
/// terrain hash to different seeds, and the same world always hashes the same way.
fn world_micro_relief_seed(elevation: &[f32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    let stride = (elevation.len() / 4096).max(1);
    let mut i = 0usize;
    while i < elevation.len() {
        h ^= elevation[i].to_bits() as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3); // FNV-1a prime
        i += stride;
    }
    h
}

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

    // â”€â”€ Micro-relief: break the flat-plain drainage artefact â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // On a genuinely flat surface the priority-flood's Îµ-tilt is a SINGLE uniform
    // gradient away from the spill point, so every cell drains the same diagonal
    // way â†’ a fan of parallel, straight, look-alike "rivers" (the artefact the
    // flat green interiors showed). Real plains have a few metres of gentle
    // undulation that gathers runoff into a branching, dendritic tributary tree.
    // Add a small multi-octave noise to the elevation the hydrology reads. Its
    // amplitude (~5 m of the 8848 m range) is far below the ~18 m lake-fill
    // threshold, so it invents no lakes or hills; where there is real relief it is
    // negligible (rivers still follow the terrain), but on a flat it supplies the
    // tie-breaking micro-topography that turns the parallel streaks into natural
    // dendritic drainage. Deterministic (fixed seed) so a given world is stable.
    const MICRO_RELIEF: f32 = 0.0006;   // â‰ˆ5 m; < 0.002 (~18 m) lake threshold
    const MICRO_FREQ: f32 = 0.05;       // base feature â‰ˆ 20 cells (finer octaves nest)
    // WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 2 (F2): the seed used to be the
    // literal 0x5CA1_AB1E, not derived from the world at all -- so the field that
    // decides where every flat-plain river goes was byte-identical in every world
    // this app has ever generated. `world_micro_relief_seed` hashes the world's own
    // elevation field, so the seed is deterministic per world and different between
    // worlds. A second, much broader wavelength (~90 cells vs ~20) is blended in so
    // one plain's drainage tree reads at a different scale from another's, rather
    // than every plain growing the identical size of tributary tree.
    let world_seed = world_micro_relief_seed(&buf.elevation);
    const MICRO_FREQ_BROAD: f32 = 0.011; // base feature â‰ˆ 90 cells
    let elev: Vec<f32> = (0..total)
        .map(|i| {
            if buf.terrain[i] != 1 { return buf.elevation[i]; } // perturb land only
            let x = (i % w as usize) as f32;
            let y = (i / w as usize) as f32;
            let n_fine = crate::sim::elevation::fbm_noise(
                x * MICRO_FREQ, y * MICRO_FREQ, world_seed, 4, 2.0, 0.5);
            let n_broad = crate::sim::elevation::fbm_noise(
                x * MICRO_FREQ_BROAD, y * MICRO_FREQ_BROAD, world_seed ^ 0x9E37_79B9_7F4A_7C15, 3, 2.0, 0.5);
            let n = n_fine * 0.65 + n_broad * 0.35;
            buf.elevation[i] + (n - 0.5) * 2.0 * MICRO_RELIEF
        })
        .collect();

    // Seed the flood with every sea cell â€” these are the drainage outlets.
    for i in 0..total {
        filled[i] = elev[i];
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
            // Raise to just above the spill level (Îµ-tilt) so the filled surface
            // is strictly increasing away from the outlet â€” no flats to strand
            // the drainage direction on. `flow_dir` here is provisional (the spill
            // parent); it is overwritten by the steepest-descent pass below.
            let raised = elev[ni].max(c_filled + FILL_EPS);
            filled[ni] = raised;
            flow_dir[ni] = if c_is_sea { FLOW_SEA } else { idx as i32 };
            heap.push(FloodCell { elev: raised, seq, idx: ni });
            seq += 1;
        }
    }

    // â”€â”€ Steepest-descent drainage on the filled surface â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Replace the flood's spill-tree parent with the genuinely lowest of the
    // eight filled neighbours. On real slopes this follows the terrain gradient;
    // on Îµ-tilted former flats the lowest neighbour is the one toward the outlet.
    // A sea neighbour (filled == 0) always wins, so coastal cells drain to the
    // sea. The Îµ guarantee means every reached land cell has a strictly lower
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
    // Ice-cap (KÃ¶ppen EF) cells are permanent ice sheets: their precipitation is
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
    buf: &WorldBuffer, flow_dir: &[i32], acc: &[u32], filled: &[f32],
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
    // full upstream catchment â€” the river, the lake and its outflow read as one
    // connected system, just not one line over the water.
    let mut is_lake = vec![false; total];
    for lk in lakes {
        for &(lx, ly) in &lk.cells {
            is_lake[buf.idx(lx, ly)] = true;
        }
    }

    // PONDED cells -- the same rule as `is_lake`, applied to every basin the lake
    // pass did not emit. A river is routed on the priority-flood's FILLED surface,
    // which raises a depression to its spill level; where that fill is deep, the
    // TRUE ground under it rises steeply, so a channel drawn across it climbs the
    // hillside on the map. Measured on a 900x500 world before this existed: 83% of
    // all ascending river steps lay inside filled depressions, 57 of 286 rivers
    // climbed more than 100 m in total, and the worst climbed 1,940 m (6,392 m on
    // a plate world) -- the "rivers run upward, ignoring elevation" report.
    //
    // `detect_lakes` already excludes the basins it emits, but it only emits those
    // past its own depth threshold AND under `max_cells`; a basin that is too big,
    // or filled too shallowly to count as a lake, was left as open channel. Water
    // ponds there in either case, so it is not channel either way. Excluding these
    // cells reuses the lake mechanism exactly: the reach ends at the basin edge,
    // and the outflow below the basin starts a new reach (its upstream neighbour
    // is no longer a channel, so it becomes a source), which is how a river/lake/
    // outflow system already reads today.
    const POND_FILL: f32 = 10.0 / 8848.0; // >10 m of fill = standing water, not channel
    let ponded: Vec<bool> = (0..total)
        .map(|i| filled.get(i).map_or(false, |&f| f - buf.elevation[i] > POND_FILL))
        .collect();
    let area_ratio = (w * h) as f32 / 64800.0;
    let base = (40.0 * area_ratio.sqrt()).max(20.0);
    // density 0 â†’ 3Ã— threshold (sparse), 0.5 â†’ 1.9Ã—, 1 â†’ 0.7Ã—, 1.5 â†’ 0.25Ã— (dense).
    // Floors raised from the old (0.4Ã—/0.1Ã—) values so the network stays legible â€”
    // the previous low floor let the whole map fill with a woven mat of streams
    // (the "spaghetti" the user flagged). Fewer, better-defined channels now.
    let density_mult = (3.0 - 2.3 * density).max(0.25);
    let threshold = (base * density_mult).max(4.0) as u32;

    let use_koppen = !buf.koppen.is_empty();

    // â”€â”€ Build the channel network â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // A channel cell is land (not a permanent ice cap) carrying at least
    // `threshold` accumulated flow â€” a UNIFORM threshold, deliberately. A per-cell
    // (climate-weighted) threshold truncates rivers mid-course: a channel that
    // flows into a drier, higher-threshold cell before its accumulation catches up
    // is cut off, leaving a dangling stub on open land (the "river got truncated"
    // bug). With a single threshold, every channel runs unbroken to its true outlet
    // (sea / lake / confluence). Climate density instead prunes WHOLE small streams
    // in arid catchments after extraction (see the arid-tributary drop below), which
    // thins desert rivers without ever cutting a river in half.
    let is_channel = |i: usize| -> bool {
        buf.terrain[i] == 1
            && !is_lake[i]
            && !ponded[i]
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

    // CONFLUENCE cells â€” a channel with two or more channel inflows (its main-stem
    // child plus at least one tributary head). A tributary segment ENDS on exactly
    // such a cell (the appended junction), while the main stem passes THROUGH it as
    // an interior vertex. So the trunk's meander must be pinned to the cell centre
    // here; otherwise the trunk's rendered line wanders off-centre while the
    // tributary's endpoint stays on the centre â€” the visible "rivers don't join"
    // gap. Anchoring the confluence on the trunk makes both meet exactly.
    let confluence: Vec<bool> = (0..total).map(|i| up_count[i] >= 2).collect();

    // Global channel occupancy â€” every cell that carries a river. The meander pass
    // uses it to wall each river's bends off its neighbours so render paths never
    // cross (the drainage tree itself never crosses; this keeps the cosmetic render
    // faithful to that).
    let channel_cell: Vec<bool> = (0..total).map(|i| is_channel(i)).collect();

    // Walk one polyline per SOURCE (a channel cell with no channel inflow). Each
    // walk runs downstream while it stays the main branch, ending either at the
    // sea (a trunk river) or at the confluence where it merges into a larger
    // stream (a tributary â€” the junction cell is appended so the line connects).
    let mut rivers = Vec::new();
    let major_len = (w as f32 * 0.10).max(60.0);
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

        // â”€â”€ Physical river width = DISCHARGE â”€â”€ drainage area Ã— runoff, cut back
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
        let width = (raw * width_scale).clamp(0.6, 2.6);

        // Strahler-ish order from discharge (1 = creek). Ranks branches for
        // rendering / settlement scoring without a full stream-order pass.
        let order = (((outlet_acc / threshold as f32).max(1.0).log2().floor() as i32) + 1)
            .clamp(1, 7) as u8;

        // â”€â”€ CLIMATE DENSITY (whole-river prune) â”€â”€ Deserts carry few rivers: an arid
        // basin swallows its runoff to evaporation. Drop small headwater/low-order
        // streams whose reach is mostly arid, so dry regions read sparse while wet
        // regions keep their fine drainage. Never touches a river that reaches the
        // sea, nor any sizeable (order â‰¥ 3) river â€” trunks fed from wet uplands still
        // cross the desert to the coast (the Nile), so nothing is cut mid-course.
        if !is_mouth && order <= 2 && arid_frac > 0.55 { continue; }

        // â”€â”€ Meander scale (gradient-gated) â”€â”€ mean channel gradient over the reach
        // in normalized-elevation units per cell: steep headwaters run straight
        // down the fall line, flat lowland reaches are free to meander. Map a steep
        // gradient (â‰¥ ~0.004/cell â‰ˆ 35 m/cell of the 8848 m range) to 0 amplitude
        // and a near-flat one (â‰¤ ~0.0005/cell) to full amplitude, so the render
        // stops wiggling rivers across the highlands (the "doesn't follow the
        // terrain" artefact) and only bends them on true floodplains.
        let src_e = buf.elevation[buf.idx(points[0].0, points[0].1)];
        let out_e = buf.elevation[outlet];
        let grad_per_cell = (src_e - out_e).max(0.0) / length.max(1.0);
        let meander = (1.0 - (grad_per_cell - 0.0005) / (0.004 - 0.0005)).clamp(0.0, 1.0);
        // MAJOR = a long trunk OR a high-order channel â†’ darker render shade.
        let major = length >= major_len || order >= 4;
        // Navigable = enough discharge to float a barge (a real inland highway).
        let navigable = discharge >= threshold as f32 * 5.0 && width >= 1.8;
        let tributary = !is_mouth;

        // â”€â”€ Mouth landform (trunks only): depositional DELTA on a flat shallow
        // coast vs drowned ESTUARY on a steeper one. â”€â”€
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

        // TRUE meander geometry (sub-cell render polyline) â€” winds on flat
        // lowlands, straight in the steep headwaters, clamped to the valley floor.
        let mseed = (s as u64).wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x51ED);
        let render = build_meander_path(buf, &points, width, discharge, threshold as f32, mseed, &confluence, &channel_cell);
        // Braided anabranches on the widest, flattest reaches of a great river.
        let braids = build_braids(buf, &render, width, order, mseed ^ 0xB2A1);

        rivers.push(River { points, width, major, navigable, mouth_kind, delta, tributary, order, meander, render, braids });
    }

    rivers
}

/// Build the sub-cell MEANDER RENDER PATH for one river reach. Smooths the D8
/// staircase into a flowing centreline, then displaces it into physically-motivated
/// meanders whose amplitude grows where the river SLOWS (low local gradient) and
/// shrinks to zero on steep headwaters, clamped to the flat valley floor so the
/// channel never climbs the hillside. Wavelength â‰ˆ 10-14Ã— channel width (real
/// meander geometry) and is noise-jittered, and a weak second harmonic makes the
/// bends asymmetric, so the result reads organic rather than as a mechanical sine
/// wave. Coordinates are CELL-INDEX space (same as `points`); X is rewrapped.
fn build_meander_path(
    buf: &WorldBuffer, points: &[(u32, u32)],
    width_cells: f32, discharge: f32, threshold: f32, seed: u64,
    confluence: &[bool], channel_cell: &[bool],
) -> Vec<(f32, f32)> {
    let n = points.len();
    if n < 4 {
        return points.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
    }
    let w = buf.width as f32;
    let h = buf.height;

    // This river's OWN channel cells â€” so the "neighbouring river = wall" clamp
    // below doesn't wall a river against itself where it curves back near its path.
    let own: std::collections::HashSet<usize> =
        points.iter().map(|&(x, y)| buf.idx(x, y)).collect();
    // Does (fx,fy) sit on ANOTHER river's channel? Such cells act as valley walls
    // for the meander, so a river can never swing its bends across a neighbour â€” the
    // true drainage paths form a non-crossing forest, and this keeps the RENDER
    // paths non-crossing too (the "rivers intersect" weave the user flagged).
    let blocks_other = |fx: f32, fy: f32| -> bool {
        if channel_cell.is_empty() { return false; }
        let xi = buf.wrap_x(fx.round() as i32);
        let yi = (fy.round() as i32).clamp(0, h as i32 - 1) as u32;
        let idx = buf.idx(xi, yi);
        channel_cell.get(idx).copied().unwrap_or(false) && !own.contains(&idx)
    };

    // â”€â”€ Unroll X across the wrap seam so the meander math stays continuous â”€â”€
    let mut cx = vec![points[0].0 as f32; n];
    for k in 1..n {
        let prev = cx[k - 1];
        let raw = points[k].0 as f32;
        let mut best = raw;
        let mut bd = (raw - prev).abs();
        for m in [-1.0f32, 1.0] {
            let c = raw + m * w;
            if (c - prev).abs() < bd { bd = (c - prev).abs(); best = c; }
        }
        cx[k] = best;
    }
    let mut cy: Vec<f32> = points.iter().map(|&(_, y)| y as f32).collect();
    // Exact (unrolled) cell centres before smoothing/meandering â€” used to PIN any
    // confluence vertex so the trunk passes dead-centre through a junction, meeting
    // the tributary whose endpoint is anchored to the same cell.
    let ux0 = cx.clone();
    let uy0 = cy.clone();
    let is_anchor = |k: usize| -> bool {
        !confluence.is_empty() && {
            let idx = buf.idx(points[k].0, points[k].1);
            confluence.get(idx).copied().unwrap_or(false)
        }
    };

    // â”€â”€ Smooth the 8-neighbour staircase (windowed passes, endpoints anchored) â”€â”€
    // 4 passes (was 2) so the D8 staircase's abrupt 45Â°/90Â° corners round out into a
    // flowing centreline â€” sharp near-right-angle kinks in the render were the
    // "unnatural steep turns" the user flagged.
    for _ in 0..4 {
        let (sx, sy) = (cx.clone(), cy.clone());
        for k in 1..n - 1 {
            if is_anchor(k) { cx[k] = ux0[k]; cy[k] = uy0[k]; continue; } // pin confluence
            cx[k] = sx[k - 1] * 0.25 + sx[k] * 0.5 + sx[k + 1] * 0.25;
            cy[k] = sy[k - 1] * 0.25 + sy[k] * 0.5 + sy[k + 1] * 0.25;
        }
    }

    let sample_elev = |fx: f32, fy: f32| -> f32 {
        let xi = buf.wrap_x(fx.round() as i32);
        let yi = (fy.round() as i32).clamp(0, h as i32 - 1) as u32;
        buf.elevation[buf.idx(xi, yi)]
    };
    let is_land = |fx: f32, fy: f32| -> bool {
        let xi = buf.wrap_x(fx.round() as i32);
        let yi = (fy.round() as i32).clamp(0, h as i32 - 1) as u32;
        buf.terrain[buf.idx(xi, yi)] == 1
    };

    const STEEP_G: f32 = 0.004;    // â‰¥ this gradient/cell â†’ straight (slowness 0)
    const FLAT_G: f32 = 0.0005;    // â‰¤ this â†’ free to meander (slowness 1)
    const VALLEY_RISE: f32 = 0.0016; // ~14 m rise marks the valley wall
    // Keep the meander corridor tight: a smaller valley probe + longer, lower-
    // amplitude bends. The old Â±22-cell reach with 0.22Â·wavelength amplitude let
    // rivers wander far enough to weave into one another (the "spaghetti" look);
    // gentle broad bends read as natural meanders without the tangle.
    let max_reach = 10i32;
    // Meander wavelength follows the geomorphic scaling law Î» â‰ˆ 11Â·w (Leopold &
    // Wolman): one full S-curve spans about eleven channel widths. `width_cells`
    // is the channel's render width, so a great trunk winds in long broad bends and
    // a creek in short tight ones â€” the physically-correct relationship, computed
    // here in the BACKEND (the frontend just draws this path, no cosmetic meander).
    let base_wav = (width_cells * 11.0).clamp(7.0, 40.0);
    let _ = (discharge, threshold); // size is already encoded in channel width

    let mut out = vec![(0.0f32, 0.0f32); n];
    out[0] = (cx[0], cy[0]);
    out[n - 1] = (cx[n - 1], cy[n - 1]);
    let mut phase = {
        let s = (seed & 0xFFFF_FFFF) as f32;
        ((s * 12.9898).sin() * 43758.545).fract().abs() * std::f32::consts::TAU
    };
    for k in 1..n - 1 {
        // A confluence vertex stays pinned to its exact cell centre so the trunk
        // meets every joining tributary (whose endpoint sits on the same cell).
        if is_anchor(k) { out[k] = (ux0[k], uy0[k]); continue; }
        let (px, py) = (cx[k - 1], cy[k - 1]);
        let (x, y) = (cx[k], cy[k]);
        let (nx, ny) = (cx[k + 1], cy[k + 1]);
        let (mut tx, mut ty) = (nx - px, ny - py);
        let tl = (tx * tx + ty * ty).sqrt().max(1e-4);
        tx /= tl; ty /= tl;
        let (nrx, nry) = (-ty, tx); // left normal

        // Slowness from local gradient over a Â±win window.
        let win = 3usize.min(k).min(n - 1 - k);
        let e_up = sample_elev(cx[k - win], cy[k - win]);
        let e_dn = sample_elev(cx[k + win], cy[k + win]);
        let grad = (e_up - e_dn).max(0.0) / (2.0 * win as f32).max(1.0);
        let mut slow = ((1.0 - (grad - FLAT_G) / (STEEP_G - FLAT_G)) as f32).clamp(0.0, 1.0);
        slow = slow * slow * (3.0 - 2.0 * slow);

        // Advance meander phase along arc length, wavelength strongly noise-jittered
        // so no two bends share a period — a regular sine reads as mechanical, an
        // irregular one as a real river.
        let jit = fbm_noise(x * 0.05 + 3.1, y * 0.05 + 7.7, seed ^ 0xA53F, 3, 2.0, 0.5);
        // A slow drift over the whole reach (long wavelength noise), on top of the
        // per-bend jitter above, so a river's characteristic bend length itself
        // wanders along its course rather than jittering around one fixed constant.
        let drift = fbm_noise(x * 0.006 + 101.0, y * 0.006 + 203.0, seed ^ 0x0D71F7, 3, 2.0, 0.5);
        let wav = base_wav * (0.6 + 0.85 * jit) * (0.85 + 0.3 * drift);
        let seg = ((x - px).powi(2) + (y - py).powi(2)).sqrt();
        phase += seg * (std::f32::consts::TAU / wav);

        let mut off = 0.0f32;
        // Gate lowered (was 0.12) so gently-sloping ground also wanders — the diagonal
        // drift the user asked for lives on moderate slopes, not just dead-flat floodplain.
        if slow > 0.05 {
            // Measure the flat valley half-width on each bank so the meander is
            // clamped to the valley floor â€” the channel never climbs the hillside
            // (this is what keeps the bends looking natural). A valley wall is
            // rising ground, the sea, or the map edge.
            let base_e = sample_elev(x, y);
            let mut half = max_reach as f32;
            for sgn in [-1.0f32, 1.0] {
                let mut d = 1.0;
                while d <= max_reach as f32 {
                    let (sxp, syp) = (x + nrx * sgn * d, y + nry * sgn * d);
                    // Wall the meander at: the map edge, the sea, rising valley wall,
                    // OR another river's channel â€” so bends never swing across a
                    // neighbouring river (no render-path crossings).
                    if syp < 0.0 || syp >= h as f32 || !is_land(sxp, syp)
                        || sample_elev(sxp, syp) > base_e + VALLEY_RISE
                        || blocks_other(sxp, syp) {
                        half = half.min(d - 1.0);
                        break;
                    }
                    d += 1.0;
                }
            }
            // Amplitude â‰ˆ 0.16Ã— wavelength (channel width already encodes river
            // size), never more than ~0.5 of the flat half-width. Raised from 0.11 for
            // deeper, more obvious bends, still corridor-clamped so adjacent rivers
            // keep their own valley and can't weave together.
            //
            // Slice 2 (F2): the base amplitude was ONE number for the whole river,
            // so every bend on every lowland reach came out the same size -- the
            // "textbook sine, same on every world" complaint. `bend_mod` scales it
            // per-bend from a noise field a few bends long, squared so bends are
            // usually modest and occasionally large (real meander trains alternate
            // tight and broad loops, not a uniform train).
            let bend_n = fbm_noise(x * (0.9 / base_wav.max(1.0)) + 41.0,
                y * (0.9 / base_wav.max(1.0)) + 23.0, seed ^ 0x5EED, 3, 2.0, 0.5);
            let bend_mod = 0.25 + 1.45 * bend_n * bend_n;
            let amp = slow * bend_mod * (base_wav * 0.16).min(half * 0.5);
            // Kinoshita (1961) skewed/flattened correction, replacing the plain
            // `sin Ï† + 0.25Â·sin(2Ï†+0.7)` second harmonic. `sin Ï†` alone (or with a
            // fixed second harmonic) is still a regular wave; the Kinoshita terms
            // `flat Â·cos 3Ï†` and `-skew Â·sin 3Ï†` are what give a real meander its
            // asymmetric goose-neck shape (steep on the downstream limb, shallow on
            // the upstream one) rather than a symmetric ripple. `skew`/`flat` drift
            // slowly along the reach (their own low-frequency noise) so no two
            // bends share the identical shape either.
            let skew = 0.15 + 0.35 * fbm_noise(x * 0.02 + 61.0, y * 0.02 + 17.0, seed ^ 0x51EE, 3, 2.0, 0.5);
            let flat = 0.10 + 0.30 * fbm_noise(x * 0.02 + 5.0, y * 0.02 + 91.0, seed ^ 0x91A5, 3, 2.0, 0.5);
            let bend = amp * (phase.sin() + flat * (3.0 * phase).cos() - skew * (3.0 * phase).sin());
            // Low-frequency WANDER: a slow, noise-driven lateral drift over ~80-cell
            // scales, independent of the meander period, so the channel strays
            // unpredictably across its floodplain (diagonal reaches, no two bends
            // alike) instead of tracing a tidy repeating wave. Corridor-clamped like
            // the bend, so it can never climb the valley wall or cross a neighbour.
            let wnoise = fbm_noise(x * 0.012 + 11.0, y * 0.012 + 5.0, seed ^ 0x1D3B, 4, 2.0, 0.5) - 0.5;
            let wander = slow * (base_wav * 0.14).min(half * 0.45) * 2.0 * wnoise;
            off = bend + wander;
            // Final safety clamp to the measured corridor.
            let lim = (half * 0.6).max(0.0);
            off = off.clamp(-lim, lim);
        }
        let ry = (y + nry * off).clamp(0.0, h as f32 - 1.0);
        out[k] = (x + nrx * off, ry);
    }

    // â”€â”€ Post-smooth the displaced path to round any remaining sharp turns â”€â”€
    // The meander displacement (and the pinned confluence vertices) can leave the
    // occasional cusp / near-right-angle corner; a couple of light windowed passes
    // (endpoints + confluence anchors held fixed so connections survive) ease those
    // into gentle bends without materially shrinking the meander.
    for _ in 0..2 {
        let src = out.clone();
        for k in 1..n - 1 {
            if is_anchor(k) { continue; }
            out[k].0 = src[k - 1].0 * 0.2 + src[k].0 * 0.6 + src[k + 1].0 * 0.2;
            out[k].1 = src[k - 1].1 * 0.2 + src[k].1 * 0.6 + src[k + 1].1 * 0.2;
        }
    }

    // Rewrap X into [0, w).
    for p in out.iter_mut() {
        let mut xx = p.0 % w;
        if xx < 0.0 { xx += w; }
        p.0 = xx;
    }
    out
}

/// Build BRAIDED anabranches for a great river's widest, flattest reaches â€” thin
/// secondary channels that split off and rejoin the main stem around a sandbar
/// island. Only for large rivers (order â‰¥ 5); each strand bows a modest, valley-
/// clamped amount to one side over a short run, tapering in and out so it reads as
/// a splitting channel, not a parallel line. Conservative (â‰¤ 3 strands/river) so
/// the effect enriches the great trunks without cluttering the map.
fn build_braids(buf: &WorldBuffer, render: &[(f32, f32)], width_cells: f32, order: u8, _seed: u64) -> Vec<Vec<(f32, f32)>> {
    let n = render.len();
    if order < 5 || width_cells < 1.6 || n < 44 { return Vec::new(); }
    let w = buf.width as f32;
    let h = buf.height;

    let sample_elev = |fx: f32, fy: f32| -> f32 {
        let xi = buf.wrap_x(fx.round() as i32);
        let yi = (fy.round() as i32).clamp(0, h as i32 - 1) as u32;
        buf.elevation[buf.idx(xi, yi)]
    };
    let is_land = |fx: f32, fy: f32| -> bool {
        let xi = buf.wrap_x(fx.round() as i32);
        let yi = (fy.round() as i32).clamp(0, h as i32 - 1) as u32;
        buf.terrain[buf.idx(xi, yi)] == 1
    };

    const STEEP_G: f32 = 0.004;
    const FLAT_G: f32 = 0.0005;
    let island = 14usize;                 // strand length in vertices
    let side_amp = (width_cells * 0.9).clamp(1.5, 4.0);
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut side = 1.0f32;                 // alternate banks
    let mut k = n / 6;
    while k + island < n && out.len() < 3 {
        // Local slowness (flatness) at the strand's midpoint.
        let m = k + island / 2;
        let a = sample_elev(render[m - 3].0, render[m - 3].1);
        let b = sample_elev(render[m + 3].0, render[m + 3].1);
        let grad = (a - b).abs() / 6.0;
        let slow = ((1.0 - (grad - FLAT_G) / (STEEP_G - FLAT_G)) as f32).clamp(0.0, 1.0);
        if slow < 0.6 { k += 6; continue; }

        // Build the strand: offset each vertex perpendicular to the local flow,
        // with a sine envelope so it splits from and rejoins the trunk. Bail out
        // if it would leave the flat valley (rising ground / sea / edge).
        let mut strand: Vec<(f32, f32)> = Vec::with_capacity(island + 1);
        let mut ok = true;
        for j in 0..=island {
            let idx = k + j;
            let (px, py) = render[idx.saturating_sub(1)];
            let (nx, ny) = render[(idx + 1).min(n - 1)];
            let (mut tx, mut ty) = (nx - px, ny - py);
            let tl = (tx * tx + ty * ty).sqrt().max(1e-4);
            tx /= tl; ty /= tl;
            let (nrx, nry) = (-ty, tx);
            let env = (std::f32::consts::PI * j as f32 / island as f32).sin(); // 0..1..0
            let off = side * side_amp * env;
            let (sx, sy) = (render[idx].0 + nrx * off, render[idx].1 + nry * off);
            if sy < 0.0 || sy >= h as f32 || !is_land(sx, sy) { ok = false; break; }
            let mut xx = sx % w; if xx < 0.0 { xx += w; }
            strand.push((xx, sy));
        }
        if ok && strand.len() >= island {
            out.push(strand);
            side = -side;         // next island splits the other bank
            k += island * 2;      // leave a gap before the next braid
        } else {
            k += 6;
        }
    }
    out
}

/// Detect OXBOW / backwater lakes cut off from strongly meandering reaches. Scans
/// each river's meander render path for apexes that swing far from the straight
/// chord â€” which only happens on flat lowland reaches, since the meander amplitude
/// is zero on steep ground â€” and lays down a small crescent of still water on the
/// OUTER bank: an abandoned meander loop. Conservative on purpose (only sizeable
/// rivers, strong bends, land cells not already channel/lake, â‰¤2 per river) so the
/// map gains the occasional characterful backwater rather than clutter. Emitted
/// lakes carry `kind = 1` so the limnology layer gives them still-water ecology.
pub fn extract_oxbows(rivers: &[River], buf: &WorldBuffer, existing: &[Lake]) -> Vec<Lake> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();
    let wf = w as f32;

    // Occupied = existing lake cells + every river channel cell, so an oxbow never
    // overlaps real water or the live channel it split off from.
    let mut occupied = vec![false; total];
    for lk in existing { for &(x, y) in &lk.cells { occupied[buf.idx(x, y)] = true; } }
    for r in rivers { for &(x, y) in &r.points { occupied[buf.idx(x, y)] = true; } }

    let mut out: Vec<Lake> = Vec::new();
    for r in rivers {
        if r.order < 3 { continue; }        // only real rivers grow oxbows
        let path = &r.render;
        let n = path.len();
        if n < 12 { continue; }
        // Unroll X for continuous chord math across the wrap seam.
        let mut ux = vec![path[0].0; n];
        for k in 1..n {
            let prev = ux[k - 1];
            let raw = path[k].0;
            let mut best = raw;
            let mut bd = (raw - prev).abs();
            for m in [-wf, wf] {
                let c = raw + m;
                if (c - prev).abs() < bd { bd = (c - prev).abs(); best = c; }
            }
            ux[k] = best;
        }
        let q = 5usize;
        let mut made = 0u8;
        let mut k = q;
        while k + q < n {
            let (ax, ay) = (ux[k - q], path[k - q].1);
            let (bx, by) = (ux[k + q], path[k + q].1);
            let (cxp, cyp) = (ux[k], path[k].1);
            let (ex, ey) = (bx - ax, by - ay);
            let el = (ex * ex + ey * ey).sqrt().max(1e-3);
            // Signed perpendicular distance of the apex from the chord.
            let dev = ((cxp - ax) * (-ey) + (cyp - ay) * ex) / el;
            if dev.abs() >= 4.0 {
                let (nrx, nry) = (-ey / el, ex / el); // left-normal unit
                let s = dev.signum();
                let mut cells: Vec<(u32, u32)> = Vec::new();
                for j in (k - q)..=(k + q) {
                    // Push the loop ~2 cells past the channel apex onto the outer bank.
                    let fx = ux[j] + nrx * s * 2.0;
                    let fy = path[j].1 + nry * s * 2.0;
                    let xi = buf.wrap_x(fx.round() as i32);
                    let yi = (fy.round() as i32).clamp(0, h as i32 - 1) as u32;
                    let idx = buf.idx(xi, yi);
                    if buf.terrain[idx] == 1 && !occupied[idx] && !cells.contains(&(xi, yi)) {
                        cells.push((xi, yi));
                    }
                }
                let _ = ay;
                if cells.len() >= 4 {
                    let mut se = 0.0;
                    for &(x, y) in &cells { se += buf.elevation[buf.idx(x, y)]; occupied[buf.idx(x, y)] = true; }
                    let elev = se / cells.len() as f32;
                    out.push(Lake { cells, elevation: elev, kind: 1, endorheic: false, salinity_ppt: 0.2 });
                    made += 1;
                    if made >= 2 { break; }
                    k += 2 * q; // one bend â†’ one oxbow
                    continue;
                }
            }
            k += 1;
        }
    }
    out
}

/// Detect lakes from the depression-filled surface.
/// A lake is a connected region of land whose filled elevation sits
/// meaningfully above its true terrain (i.e. a filled depression).
///
/// `fill_depth` (0..1, in normalized-elevation units): minimum fill before a
/// depression counts â€” raising it keeps only genuinely deep basins, shrinking
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

            // A FILLED DEPRESSION IS A LAKE -- however big it is. Water runs
            // downhill; where it reaches a hollow it cannot escape, it stands
            // there. That is the whole of the rule.
            //
            // This used to read `cells.len() <= max_cells` ("giant flooded basins
            // are usually artefacts"), which DELETED the water body while leaving
            // the drainage routed straight through the basin -- so a river was
            // drawn across ground that truly rises, climbing the basin walls, with
            // no lake to explain it. `max_cells` was `total / 2000`, about 225
            // cells on a 900x500 world, so every genuinely large lake (a Caspian, a
            // Great Lake) was thrown away for being large. Measured before the fix:
            // 57 of 286 rivers climbed over 100 m, the worst 1,940 m.
            //
            // `max_cells` is now only a DEGENERACY guard, not a size filter: it
            // takes effect solely when a basin has swallowed an implausible share
            // of the whole world, which means the elevation field is broken rather
            // than that the lake is big. Even then the cells stay excluded from the
            // channel network (`ponded` in `extract_rivers`), so the one thing that
            // can never happen either way is a river drawn climbing out of it.
            let degenerate = cells.len() > (total / 4).max(max_cells);
            if cells.len() >= 2 && !degenerate {
                let elevation = sum_elev / cells.len() as f32;
                lakes.push(Lake { cells, elevation, kind: 0, endorheic: false, salinity_ppt: 0.0 });
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
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
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
            koppen: vec![crate::sim::koppen::CFB; n], // temperate, not EF/arid
            soil_type: Vec::new(),
            fertility: Vec::new(),
            fishery: Vec::new(),
            current_type: Vec::new(),
            wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(),
            distance_to_ocean: Vec::new(),
            habitability: Vec::new(),
            salinity: Vec::new(),
            shark_risk: Vec::new(),
            goods: Vec::new(),
            shipworm_risk: Vec::new(),
            storm_base: Vec::new(),
            reef_risk: Vec::new(),
            disease_risk: Vec::new(), precip_summer_frac: Vec::new(), seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        }
    }

    /// Item 2: every drainage step must go DOWNHILL on the filled surface â€” the
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
        let rivers = extract_rivers(&buf, &hy.flow_dir, &hy.acc, &hy.filled, 1.2, 1.0, &[]);
        let trunks = rivers.iter().filter(|r| !r.tributary).count();
        let tribs = rivers.iter().filter(|r| r.tributary).count();
        assert!(trunks >= 1, "expected at least one trunk reaching the sea");
        assert!(tribs >= 1, "expected tributary segments, got {} rivers ({} trunks)",
            rivers.len(), trunks);
        // Every extracted river now carries a meander render path of matching length.
        for r in &rivers {
            assert_eq!(r.render.len(), r.points.len(), "render path mirrors the cell path");
        }
        // RULE 2 â€” a tributary must VISUALLY JOIN the stream it flows into: its last
        // render vertex (the confluence) must coincide with a render vertex of some
        // OTHER river (the trunk anchors that confluence to the same cell centre).
        // This guards the "rivers don't join together" gap.
        for (ti, t) in rivers.iter().enumerate() {
            if !t.tributary { continue; }
            let end = *t.render.last().unwrap();
            let joined = rivers.iter().enumerate().any(|(oi, o)| {
                oi != ti && o.render.iter().any(|&(x, y)|
                    (x - end.0).abs() < 0.05 && (y - end.1).abs() < 0.05)
            });
            assert!(joined, "tributary {ti} ends at {:?} but no trunk shares that point", end);
        }
        // RULE 3 â€” a trunk must REACH THE SEA, never truncate on open land: its last
        // cell is adjacent to ocean. (With a uniform channel threshold, a river can
        // only stop at the sea, a lake shore, or a confluence â€” this world has no
        // lakes, so every trunk must be a river mouth.)
        for (ti, t) in rivers.iter().enumerate() {
            if t.tributary { continue; }
            let (lx, ly) = *t.points.last().unwrap();
            let touches_sea = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                let nx = buf.wrap_x(lx as i32 + dx);
                let ny = ly as i32 + dy;
                ny >= 0 && ny < buf.height as i32 && buf.terrain[buf.idx(nx, ny as u32)] == 0
            });
            assert!(touches_sea, "trunk {ti} ends at {:?} without reaching the sea (truncated)", (lx, ly));
        }
    }

    /// A river crossing a FLAT floodplain must actually meander â€” its render path
    /// swings off the straight centreline â€” while its endpoints stay pinned to the
    /// true source/mouth, and the path length matches the input.
    #[test]
    fn meander_bends_on_flat_ground() {
        let (w, h) = (80u32, 40u32);
        let n = (w * h) as usize;
        let buf = synth(w, h, vec![1u8; n], vec![0.1f32; n]); // all land, dead flat
        let pts: Vec<(u32, u32)> = (5..70).map(|x| (x, 20)).collect();
        let render = build_meander_path(&buf, &pts, 2.5, 4000.0, 20.0, 12345, &[], &[]);

        assert_eq!(render.len(), pts.len(), "render mirrors the cell path");
        assert!((render[0].0 - 5.0).abs() < 0.001 && (render[0].1 - 20.0).abs() < 0.001,
            "source endpoint pinned");
        assert!((render[render.len() - 1].1 - 20.0).abs() < 0.001, "mouth endpoint pinned");
        let max_dev = render.iter().map(|&(_, y)| (y - 20.0).abs()).fold(0.0f32, f32::max);
        assert!(max_dev > 0.8, "a flat reach must visibly meander: {max_dev}");
        // Every vertex stays finite and on the map.
        for &(x, y) in &render {
            assert!(x.is_finite() && y.is_finite() && x >= 0.0 && x < w as f32 && y >= 0.0 && y < h as f32,
                "render vertex in bounds: ({x},{y})");
        }
    }

    /// WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 2 gate (F2): a long flat-ground
    /// reach must NOT read as a textbook sine -- successive bend amplitudes must
    /// vary meaningfully (the per-bend `bend_mod` term), not repeat one constant
    /// amplitude the way the pre-Slice-2 code did.
    #[test]
    fn meanders_are_not_a_sine() {
        let (w, h) = (400u32, 80u32);
        let n = (w * h) as usize;
        let buf = synth(w, h, vec![1u8; n], vec![0.1f32; n]); // all land, dead flat
        let pts: Vec<(u32, u32)> = (5..380).map(|x| (x, 40)).collect();
        let render = build_meander_path(&buf, &pts, 3.0, 4000.0, 20.0, 999, &[], &[]);

        // Local extrema of the lateral offset from the centreline (y=40) are the
        // bend peaks/troughs; collect their absolute amplitudes.
        let offsets: Vec<f32> = render.iter().map(|&(_, y)| y - 40.0).collect();
        let mut amps: Vec<f32> = Vec::new();
        for k in 1..offsets.len() - 1 {
            let (a, b, c) = (offsets[k - 1], offsets[k], offsets[k + 1]);
            if (b - a).signum() != (c - b).signum() && b.abs() > 0.15 {
                amps.push(b.abs());
            }
        }
        assert!(amps.len() >= 6, "expected several bends on a 375-cell flat reach, got {}", amps.len());
        let mean: f32 = amps.iter().sum::<f32>() / amps.len() as f32;
        let var: f32 = amps.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / amps.len() as f32;
        let cv = var.sqrt() / mean.max(1e-6);
        assert!(cv > 0.15, "bend amplitudes read as one constant size (CV={cv:.3}, amps={amps:?})");

        // Two different world elevation fields must seed different flat-plain
        // micro-relief -- the old hard-coded 0x5CA1_AB1E seed made every world's
        // flat drainage byte-identical.
        let elev_a = vec![0.1f32; n];
        let mut elev_b = vec![0.1f32; n];
        elev_b[7] = 0.100001; // a different world, negligible physically
        let seed_a = world_micro_relief_seed(&elev_a);
        let seed_b = world_micro_relief_seed(&elev_b);
        assert_ne!(seed_a, seed_b, "different worlds must hash to different micro-relief seeds");
    }

    /// WATER RUNS DOWNHILL, AND WHERE IT CANNOT ESCAPE IT STANDS. Both halves are
    /// asserted here, because the bug was that neither held: a basin larger than
    /// `max_cells` had its LAKE deleted as "usually an artefact" while the drainage
    /// still routed through it, so a river was drawn straight across the basin,
    /// climbing its walls, with no water to explain it. Measured on a real 900x500
    /// world at the time: 57 of 286 rivers climbed over 100 m, the worst 1,940 m.
    ///
    /// The fixture is a plain draining east to the sea with a deep closed bowl
    /// astride the trunk's path. A lake must appear IN the bowl, and no river may
    /// climb out of it.
    #[test]
    fn a_river_never_climbs_out_of_a_basin() {
        // A long plain tilting EAST to the sea, with a deep closed bowl punched
        // into the middle of that drainage path. Every cell upstream drains
        // through the bowl, so a real trunk river reaches it -- which is the
        // condition the bug needs. (A bowl off to one side collects only its own
        // small catchment, never clears the channel threshold, and no channel is
        // drawn to climb out of it: the first version of this fixture made that
        // mistake and passed with the fix disabled, which is the one thing a gate
        // must never do.)
        let (w, h) = (160u32, 60u32);
        let n = (w * h) as usize;
        let mut terrain = vec![1u8; n];
        let mut elev = vec![0.0f32; n];
        let (bx, by, br) = (90.0f32, 30.0f32, 18.0f32);
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if x >= w - 2 { terrain[i] = 0; } // sea on the EAST edge
                // Tilt east, plus a shallow cross-valley so flow converges into one trunk.
                let mut e = 0.34 - x as f32 * 0.0018 + ((y as f32 - 30.0).abs()) * 0.0012;
                // The closed bowl: ~700 m deep at its centre, astride the trunk's path.
                let d = (((x as f32 - bx).powi(2) + (y as f32 - by).powi(2)).sqrt() / br).min(1.0);
                e -= (1.0 - d * d) * 0.08;
                elev[i] = e.max(0.005);
            }
        }
        let buf = synth(w, h, terrain, elev);
        let hy = compute_hydrology(&buf);
        // max_cells small on purpose: the bowl is NOT emitted as a lake, so it is
        // exactly the "filled but not a lake" basin the ponded rule exists for.
        let lakes = detect_lakes(&buf, &hy.filled, 0.004, 4);
        let rivers = extract_rivers(&buf, &hy.flow_dir, &hy.acc, &hy.filled, 1.2, 1.0, &lakes);
        assert!(!rivers.is_empty(), "the fixture must produce channels at all");

        // 1. The water has somewhere to be: a lake sits in the bowl.
        let bowl_lake = lakes.iter().any(|lk| {
            lk.cells.iter().any(|&(x, y)| {
                ((x as f32 - bx).powi(2) + (y as f32 - by).powi(2)).sqrt() < br
            })
        });
        assert!(bowl_lake,
            "no lake formed in a closed basin the drainage runs into ({} lakes found) — \
             the water was deleted instead of standing", lakes.len());

        // Per river, the climb above the lowest point it has reached so far — the
        // figure a viewer sees. A river crossing the bowl scores hundreds of metres.
        let mut worst = 0.0f32;
        for r in &rivers {
            let mut lowest = f32::MAX;
            for &(x, y) in &r.points {
                let e = buf.elevation[buf.idx(x, y)] * 8848.0;
                if e < lowest { lowest = e; }
                if e - lowest > worst { worst = e - lowest; }
            }
        }
        assert!(worst < 60.0, "a river climbs {worst:.0} m above its own low point — it is running uphill");
    }

    /// A steep headwater reach must NOT meander (channel pinned to the fall line).
    #[test]
    fn steep_reach_stays_straight() {
        let (w, h) = (80u32, 40u32);
        let n = (w * h) as usize;
        // Steep ramp: 0.02 rise per cell in x â‰« the STEEP_G threshold.
        let elev: Vec<f32> = (0..n).map(|i| 0.02 + (i as u32 % w) as f32 * 0.02).collect();
        let buf = synth(w, h, vec![1u8; n], elev);
        // Source high (large x) â†’ mouth low (small x): a real downhill reach.
        let pts: Vec<(u32, u32)> = (5..70).rev().map(|x| (x, 20)).collect();
        let render = build_meander_path(&buf, &pts, 2.5, 4000.0, 20.0, 999, &[], &[]);
        let max_dev = render.iter().map(|&(_, y)| (y - 20.0).abs()).fold(0.0f32, f32::max);
        assert!(max_dev < 0.35, "steep ground stays near-straight: {max_dev}");
    }

    /// A river must NOT meander across a neighbouring river: with another channel
    /// three cells away, the bend amplitude is walled well short of it, so render
    /// paths never weave/cross (the "rivers intersect" bug).
    #[test]
    fn meander_does_not_cross_a_neighbour() {
        let (w, h) = (80u32, 40u32);
        let n = (w * h) as usize;
        let buf = synth(w, h, vec![1u8; n], vec![0.1f32; n]); // flat â†’ wants to meander
        // Our river along y=20; a neighbouring river's channel along y=23.
        let pts: Vec<(u32, u32)> = (5..70).map(|x| (x, 20)).collect();
        let mut channel = vec![false; n];
        for x in 5..70u32 { channel[(23 * w + x) as usize] = true; }
        let render = build_meander_path(&buf, &pts, 2.5, 4000.0, 20.0, 12345, &[], &channel);
        // The neighbour sits at y=23 (3 cells away); the bend must stay comfortably
        // on our side of the gap â€” never reach the midpoint, let alone cross.
        let max_y = render.iter().map(|&(_, y)| y).fold(0.0f32, f32::max);
        assert!(max_y < 21.5, "meander swung toward the neighbour river (max y {max_y}); would cross");
    }

    /// Oxbows must be well-formed (land, kind=1) and never panic, even on a wildly
    /// meandering flat reach.
    #[test]
    fn oxbows_are_land_backwaters() {
        let (w, h) = (120u32, 60u32);
        let n = (w * h) as usize;
        let buf = synth(w, h, vec![1u8; n], vec![0.1f32; n]);
        let pts: Vec<(u32, u32)> = (5..110).map(|x| (x, 30)).collect();
        let render = build_meander_path(&buf, &pts, 3.0, 8000.0, 20.0, 7, &[], &[]);
        let river = River {
            points: pts, width: 3.0, major: true, navigable: true, mouth_kind: 0,
            delta: vec![], tributary: false, order: 5, meander: 1.0, render, braids: vec![],
        };
        let oxbows = extract_oxbows(&[river], &buf, &[]);
        for lk in &oxbows {
            assert_eq!(lk.kind, 1, "oxbow tagged as backwater");
            assert!(lk.cells.len() >= 4, "oxbow has a real footprint");
            for &(x, y) in &lk.cells {
                assert_eq!(buf.terrain[buf.idx(x, y)], 1, "oxbow cell is land");
            }
        }
    }

    /// A great river on a broad flat plain grows braided anabranches that stay on
    /// land and inside the map; a small river grows none.
    #[test]
    fn braids_only_on_great_flat_rivers() {
        let (w, h) = (140u32, 60u32);
        let n = (w * h) as usize;
        let buf = synth(w, h, vec![1u8; n], vec![0.1f32; n]);
        let pts: Vec<(u32, u32)> = (5..130).map(|x| (x, 30)).collect();
        let render = build_meander_path(&buf, &pts, 3.0, 9000.0, 20.0, 3, &[], &[]);

        let braids = build_braids(&buf, &render, 3.0, 6, 55);
        assert!(!braids.is_empty(), "a great flat river should braid");
        for strand in &braids {
            for &(x, y) in strand {
                assert!(x >= 0.0 && x < w as f32 && y >= 0.0 && y < h as f32, "braid in bounds");
                assert_eq!(buf.terrain[buf.idx(x.round() as u32 % w, y.round() as u32)], 1, "braid on land");
            }
        }
        // A small creek (low order, thin) never braids.
        assert!(build_braids(&buf, &render, 1.0, 2, 55).is_empty(), "creeks do not braid");
    }
}


