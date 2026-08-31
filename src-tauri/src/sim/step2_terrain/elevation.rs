use rand::prelude::*;
use std::collections::VecDeque;
use crate::sim::world_buffer::WorldBuffer;
use super::geology;
use super::landform;

// â”€â”€ Seeded noise helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Integer hash â†’ pseudo-random float in 0..1
fn hash_grid(x: i32, y: i32, seed: u64) -> f32 {
    let mut h = seed as i32;
    h = h.wrapping_mul(1).wrapping_add(x.wrapping_mul(374761393));
    h ^= y.wrapping_mul(668265263);
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    h = h.wrapping_mul(1911520717);
    h ^= h >> 16;
    (h as u32) as f32 / 4294967296.0
}

/// Value noise with cosine interpolation
fn smooth_noise(x: f32, y: f32, seed: u64) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let sx = (1.0 - (fx * std::f32::consts::PI).cos()) * 0.5;
    let sy = (1.0 - (fy * std::f32::consts::PI).cos()) * 0.5;
    let v00 = hash_grid(ix, iy, seed);
    let v10 = hash_grid(ix + 1, iy, seed);
    let v01 = hash_grid(ix, iy + 1, seed);
    let v11 = hash_grid(ix + 1, iy + 1, seed);
    let top = v00 * (1.0 - sx) + v10 * sx;
    let bottom = v01 * (1.0 - sx) + v11 * sx;
    top * (1.0 - sy) + bottom * sy
}

/// Fractal Brownian Motion noise. Output ~0..1. Reused by the biological layer's
/// per-mineral ore-province field (deposit placement).
pub(crate) fn fbm_noise(x: f32, y: f32, seed: u64, octaves: u32, lacunarity: f32, persistence: f32) -> f32 {
    let mut val = 0.0f32;
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut max_amp = 0.0f32;
    for i in 0..octaves {
        val += smooth_noise(x * freq, y * freq, seed.wrapping_add(i as u64 * 7919)) * amp;
        max_amp += amp;
        amp *= persistence;
        freq *= lacunarity;
    }
    val / max_amp
}

/// Ridged multifractal noise â€” creates sharp ridge lines naturally.
/// 1 - |2*noise - 1| produces v-shaped valleys â†’ peaks.
/// Successive octaves weighted by previous value for sub-ridges.
fn ridged_multifractal(x: f32, y: f32, seed: u64, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut val = 0.0f32;
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut weight = 1.0f32;
    for i in 0..octaves {
        let mut signal = smooth_noise(x * freq, y * freq, seed.wrapping_add(i as u64 * 7919));
        // Fold into ridges
        signal = 1.0 - (signal * 2.0 - 1.0).abs();
        // Cube for sharper, more defined ridge lines
        signal = signal * signal * signal;
        // Weight by previous octave
        signal *= weight;
        weight = (signal * gain).clamp(0.0, 1.0);
        val += signal * amp;
        amp *= 0.5;
        freq *= lacunarity;
    }
    val / (octaves as f32 * 0.5)
}

/// Domain warping â€” distorts coordinates by noise for organic shapes
fn warped_coords(x: f32, y: f32, seed: u64, strength: f32) -> (f32, f32) {
    let wx = fbm_noise(x + 5.2, y + 1.3, seed.wrapping_add(11111), 3, 2.0, 0.5) - 0.5;
    let wy = fbm_noise(x + 8.7, y + 2.9, seed.wrapping_add(22222), 3, 2.0, 0.5) - 0.5;
    (x + wx * strength, y + wy * strength)
}

// â”€â”€ Erosion â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Priority-flood over LAND only, seeded from every ocean cell at fixed sea
/// level (Barnes et al. 2014 depression-filling, the same family phase 5's
/// rivers already use -- kept as a separate implementation here rather than
/// shared, per TERRAIN_2_PLAN.md section 4 slice 1 risk 2: phase 2 runs long
/// before rivers exist and unifying the two would be its own, separately
/// gated change). Returns, per land cell: `flow_to` (the neighbour it drains
/// toward -- itself for an outlet cell directly on the coast) and `order`
/// (the pop order, ascending filled elevation -- headwaters last).
/// Test-only now that phase 2 does no fluvial carving (see "NO VALLEY CARVING"
/// below): `terrain_metrics` still reports drainage density from it, and phase 5
/// keeps its own separate implementation for the real river network.
#[cfg(test)]
fn priority_flood_flow(elevation: &[f32], terrain: &[u8], w: u32, h: u32) -> (Vec<usize>, Vec<usize>) {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    #[derive(Copy, Clone)]
    struct HeapItem { elev: f32, idx: usize }
    impl PartialEq for HeapItem { fn eq(&self, o: &Self) -> bool { self.elev == o.elev } }
    impl Eq for HeapItem {}
    impl Ord for HeapItem {
        fn cmp(&self, o: &Self) -> Ordering {
            // Min-heap: BinaryHeap is a max-heap, so reverse the float compare.
            o.elev.partial_cmp(&self.elev).unwrap_or(Ordering::Equal)
        }
    }
    impl PartialOrd for HeapItem { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

    let n = (w * h) as usize;
    let wi = w as i32;
    let hi = h as i32;
    const EPS: f32 = 0.00002;

    let mut visited = vec![false; n];
    let mut flow_to = vec![0usize; n];
    let mut order = Vec::with_capacity(n);
    let mut heap = BinaryHeap::new();

    for i in 0..n {
        if terrain[i] != 1 { visited[i] = true; }
    }
    for y in 0..hi {
        for x in 0..wi {
            let i = (y as u32 * w + x as u32) as usize;
            if terrain[i] != 1 { continue; }
            let mut coastal = false;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = (((x + dx) % wi) + wi) % wi;
                let ny = (y + dy).clamp(0, hi - 1);
                if terrain[(ny as u32 * w + nx as u32) as usize] != 1 { coastal = true; break; }
            }
            if coastal {
                visited[i] = true;
                flow_to[i] = i;
                order.push(i);
                heap.push(HeapItem { elev: elevation[i], idx: i });
            }
        }
    }
    // Fallback for a landmass with no coast at all (an all-land world): seed
    // from the single lowest land cell so the pass still does something sane
    // rather than silently erode nothing.
    if heap.is_empty() {
        if let Some(lowest) = (0..n).filter(|&i| terrain[i] == 1)
            .min_by(|&a, &b| elevation[a].partial_cmp(&elevation[b]).unwrap())
        {
            visited[lowest] = true;
            flow_to[lowest] = lowest;
            order.push(lowest);
            heap.push(HeapItem { elev: elevation[lowest], idx: lowest });
        }
    }

    while let Some(HeapItem { elev, idx }) = heap.pop() {
        let x = (idx as u32 % w) as i32;
        let y = (idx as u32 / w) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = (((x + dx) % wi) + wi) % wi;
            let ny = y + dy;
            if ny < 0 || ny >= hi { continue; }
            let ni = (ny as u32 * w + nx as u32) as usize;
            if visited[ni] || terrain[ni] != 1 { continue; }
            visited[ni] = true;
            flow_to[ni] = idx;
            order.push(ni);
            let filled = elevation[ni].max(elev + EPS);
            heap.push(HeapItem { elev: filled, idx: ni });
        }
    }

    (flow_to, order)
}


// ── NO VALLEY CARVING ───────────────────────────────────────────────────────
//
// Phase 2 no longer carves valleys, in any generator, by any mechanism. Three
// separate attempts to make channel-carving look right on an Earth-sized world
// all failed for the same underlying reason, and this note records it so a
// fourth is not attempted from scratch.
//
// The mechanisms that were removed, and what each drew on the map:
//
//   * `carve` / `fine_carve` -- an INVERTED ridged-multifractal field subtracted
//     from the elevation. An inverted ridged field is a dendritic tree BY
//     CONSTRUCTION, so this drew a branching dark scratch network across every
//     continent. `fine_carve` was the worst of them because it was an ABSOLUTE
//     subtraction: full strength on flat plains, where a drainage tree is
//     exactly what you do not want painted over an otherwise smooth interior.
//
//   * `stream_power_erosion` -- priority-flood + flow accumulation + `K*A^m*S^n`
//     incision (Whipple & Tucker 1999). Correct landscape-evolution physics at
//     the resolution it is meant for, and wrong here: a cell is `KM_EQUATOR / w`
//     wide (11 km on the default 3600x1800 grid), so the valley it models is
//     SUB-GRID -- the Grand Canyon is 16 km across and would not fill one cell.
//     Applied at cell resolution it cut one-cell trenches down every D8 path
//     plus the parallel single-cell rills D8 routing always produces on a planar
//     slope. Scaling and spreading it (the previous session's fix, measured to
//     cut grid-scale texture 82-93%) made it subtler but did not change what it
//     draws, because the STRUCTURE is what reads as wrong, not the amplitude.
//
// What remains is `thermal_erosion` -- hillslope slumping, which only ROUNDS
// and never incises -- plus `limit_grid_scale_relief`. Relief comes from the
// noise stack and the tectonic terms.
//
// Rivers are unaffected: phase 5 (`step5_rivers`) runs its own priority-flood
// fill and derives channels from the finished surface, so it never needed
// pre-cut channels to bed into. What phase 2 owes it is a surface with enough
// large-scale structure to route over, which is `apply_micro_relief`'s rolling
// undulation and the noise stack's own lows -- not a drawn-in drainage tree.

/// Thermal (hillslope) erosion: material slumps from steep slopes, rounding sharp
/// peaks into weathered massifs and filling the sharpest incisions. Strengthened
/// (talus 0.03→0.025 so slopes slump sooner, rate 0.4→0.55) so mountains read as
/// ERODED rather than knife-edged — the shaping the reduced valley-carving now
/// leaves to it (user: "erode the mounts so they are more realistic").
///
/// The update is SIMULTANEOUS: every cell's transfers are accumulated into a
/// delta buffer and applied at the end of the pass. The original wrote both
/// `elevation[idx]` and `elevation[ni]` in place while scanning rows in order,
/// so a cell was slumped into by its northern neighbour BEFORE it was itself
/// visited and the whole pass carried a north-to-south scan bias -- visible as
/// one-row horizontal striations across every flank in `dump_erosion_sheet`.
/// A relaxation scheme's result must not depend on the order its cells happen
/// to be stored in.
fn thermal_erosion(
    elevation: &mut [f32], terrain: &[u8],
    w: u32, h: u32, passes: u32,
) {
    let talus = 0.025f32;
    let rate = 0.55f32;
    let dirs: [(i32, i32); 8] = [(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)];

    let mut delta = vec![0.0f32; elevation.len()];
    for _ in 0..passes {
        delta.iter_mut().for_each(|d| *d = 0.0);
        for y in 1..h - 1 {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                if terrain[idx] != 1 { continue; }
                let el = elevation[idx];
                let mut max_diff = 0.0f32;
                let mut total_diff = 0.0f32;

                for &(dx, dy) in &dirs {
                    let nx = ((x as i32 + dx + w as i32) % w as i32) as u32;
                    let ny = (y as i32 + dy) as u32;
                    if ny >= h { continue; }
                    let ni = (ny * w + nx) as usize;
                    if terrain[ni] != 1 { continue; }
                    let diff = el - elevation[ni];
                    if diff > talus {
                        total_diff += diff - talus;
                        if diff > max_diff { max_diff = diff; }
                    }
                }
                if total_diff <= 0.0 { continue; }

                for &(dx, dy) in &dirs {
                    let nx = ((x as i32 + dx + w as i32) % w as i32) as u32;
                    let ny = (y as i32 + dy) as u32;
                    if ny >= h { continue; }
                    let ni = (ny * w + nx) as usize;
                    if terrain[ni] != 1 { continue; }
                    let diff = el - elevation[ni];
                    if diff > talus {
                        let transfer = (diff - talus) / total_diff * (max_diff - talus) * rate * 0.5;
                        delta[idx] -= transfer;
                        delta[ni] += transfer;
                    }
                }
            }
        }
        for i in 0..elevation.len() {
            if terrain[i] == 1 { elevation[i] = (elevation[i] + delta[i]).max(0.001); }
        }
    }
}

// â”€â”€ Isostasy â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Fraction of the (flexurally smoothed) eroded thickness that floats back up.
const ISOSTATIC_REBOUND: f32 = 0.30;
// Flexural smoothing radius in cells (isostasy responds to loads over a broad area,
// not one cell) â€” the removed thickness is box-blurred over this radius.
const ISOSTATIC_RADIUS: i32 = 6;
// Max buoyant lift (fraction of local height) for crust sitting on a thick root at a
// convergent / transform boundary; fades to 0 over ROOT_REACH cells.
const ROOT_BUOYANCY: f32 = 0.12;
const ROOT_REACH: u16 = 8;

/// Separable box blur with cylindrical X wrap and clamped Y (poles). Radius in cells.
pub(super) fn box_blur_wrap(src: &[f32], w: u32, h: u32, radius: i32) -> Vec<f32> {
    use rayon::prelude::*;
    let wi = w as i32;
    let hi = h as i32;
    let win = (2 * radius + 1) as f32;

    // Both sweeps are rayon-parallel over ROWS and each writes only its own
    // cells, so the result is bit-identical regardless of scheduling -- the
    // same discipline section 8.9 rule 2 already applies to the phase-3 row
    // loops. This is called several times per phase-2 run now (the fluvial
    // spread, and the grid-scale budget's iteration), so it stopped being
    // cheap enough to leave sequential.
    let mut tmp = vec![0.0f32; src.len()];
    tmp.par_chunks_mut(w as usize).enumerate().for_each(|(y, dst)| {
        let row = y * w as usize;
        for x in 0..wi {
            let mut sum = 0.0f32;
            for dx in -radius..=radius {
                let xx = (((x + dx) % wi) + wi) % wi;
                sum += src[row + xx as usize];
            }
            dst[x as usize] = sum / win;
        }
    });

    let mut out = vec![0.0f32; src.len()];
    out.par_chunks_mut(w as usize).enumerate().for_each(|(y, dst)| {
        let y = y as i32;
        for dy in -radius..=radius {
            let yy = (y + dy).clamp(0, hi - 1);
            let row = (yy as u32 * w) as usize;
            for x in 0..w as usize {
                dst[x] += tmp[row + x];
            }
        }
        for v in dst.iter_mut() { *v /= win; }
    });
    out
}

/// BFS distance (in cells, capped at `max_reach`) from the nearest convergent (1) or
/// transform (3) boundary land cell. `u16::MAX` where no boundary is in reach or where
/// there is no plate data (template worlds pass an empty `boundary_type`).
fn convergent_distance(
    terrain: &[u8], boundary_type: &[u8], w: u32, h: u32, max_reach: u16,
) -> Vec<u16> {
    let n = terrain.len();
    let mut dist = vec![u16::MAX; n];
    if boundary_type.len() != n {
        return dist;
    }
    let mut q: VecDeque<usize> = VecDeque::new();
    for i in 0..n {
        if terrain[i] == 1 && (boundary_type[i] == 1 || boundary_type[i] == 3) {
            dist[i] = 0;
            q.push_back(i);
        }
    }
    let wi = w as i32;
    let hi = h as i32;
    while let Some(i) = q.pop_front() {
        let d = dist[i];
        if d >= max_reach {
            continue;
        }
        let x = (i as u32 % w) as i32;
        let y = (i as u32 / w) as i32;
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = (((x + dx) % wi) + wi) % wi;
            let ny = y + dy;
            if ny < 0 || ny >= hi {
                continue;
            }
            let ni = (ny as u32 * w + nx as u32) as usize;
            if terrain[ni] == 1 && dist[ni] == u16::MAX {
                dist[ni] = d + 1;
                q.push_back(ni);
            }
        }
    }
    dist
}

/// Isostatic adjustment, applied AFTER erosion and BEFORE the rank-based hypsometric
/// redistribution. Two effects the erosion pipeline otherwise ignores:
///  1. **Erosional rebound** â€” stripping material unloads the crust, which floats back
///     up. We re-add `ISOSTATIC_REBOUND` Ã— the *smoothed* eroded thickness so ancient,
///     heavily dissected uplands (shields, old plateaus) keep gentle elevation instead
///     of grinding down to the hypsometric floor.
///  2. **Mountain roots** â€” crust thickened at a convergent/transform boundary sits on a
///     deep buoyant root and resists erosion; nearby cells get a small height-scaled lift.
///
/// Because `redistribute_elevation` is rank-based, this changes *which* cells rank high
/// (favouring rooted / heavily-eroded uplands) while the target histogram is preserved.
fn isostatic_adjust(
    elevation: &mut [f32], terrain: &[u8], boundary_type: &[u8], pre_erosion: &[f32],
    w: u32, h: u32,
) {
    let n = elevation.len();
    // 1. Erosional rebound.
    let mut removed = vec![0.0f32; n];
    for i in 0..n {
        if terrain[i] == 1 {
            removed[i] = (pre_erosion[i] - elevation[i]).max(0.0);
        }
    }
    let smoothed = box_blur_wrap(&removed, w, h, ISOSTATIC_RADIUS);
    for i in 0..n {
        if terrain[i] == 1 {
            elevation[i] += ISOSTATIC_REBOUND * smoothed[i];
        }
    }
    // 2. Mountain roots (skipped when there is no plate data).
    let root_dist = convergent_distance(terrain, boundary_type, w, h, ROOT_REACH);
    for i in 0..n {
        if terrain[i] != 1 || root_dist[i] >= ROOT_REACH {
            continue;
        }
        let prox = 1.0 - root_dist[i] as f32 / ROOT_REACH as f32; // 1 at boundary â†’ 0 at reach
        elevation[i] += ROOT_BUOYANCY * prox * elevation[i];
    }
}

// â”€â”€ Public elevation generators â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Generate elevation from plate tectonics.
///
/// v2: mountains are concentrated into OROGENIC BELTS that run along the
/// convergent plate boundaries (where crust collides and thickens â€” the Andes /
/// Himalaya / Alps geometry), then carved into ridge-and-valley relief and
/// matched to a realistic hypsometric curve â€” the same erosion + redistribution
/// pipeline the plate-free models use, so the plate path no longer produces
/// blander terrain than "Complete from Landmass". The old model spread a uniform
/// exponential bump from every convergent cell, which left ranges that didn't
/// track the boundaries and interiors that read flat.
pub fn generate_elevation(buf: &mut WorldBuffer, seed: u64) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let terrain = buf.terrain.clone();
    let have_boundary = !buf.boundary_type.is_empty();

    // Real orogeny field (TERRAIN_2_PLAN.md section 4 slice 3): BFS distance
    // from, and setting/age inherited from, the nearest convergent/transform
    // boundary land cell. `None` when there is no plate data at all (should
    // not happen on this path, but stay defensive).
    let orogeny = geology::compute_orogeny_field(buf, seed, 240);

    // Distance from the nearest DIVERGENT boundary land cell (rifts), same BFS
    // shape as `orogeny`'s own seeding. Sampled at the SAME warped position
    // below (D4) so the rift-valley pulldown doesn't scar a straight line
    // either -- it used to read `boundary_type[idx]` directly, a hard 1-cell
    // multiply exactly on the literal Voronoi edge.
    let mut rift_dist = vec![u16::MAX; n];
    if have_boundary {
        let mut rq = VecDeque::new();
        for i in 0..n {
            if terrain[i] == 1 && buf.boundary_type[i] == 2 {
                rift_dist[i] = 0;
                rq.push_back(i);
            }
        }
        while let Some(ci) = rq.pop_front() {
            let d = rift_dist[ci];
            if d >= 24 { continue; }
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(cx + dx);
                let ny = (cy + dy).clamp(0, h as i32 - 1) as u32;
                let ni = buf.idx(nx, ny);
                if terrain[ni] == 1 && rift_dist[ni] > d + 1 {
                    rift_dist[ni] = d + 1;
                    rq.push_back(ni);
                }
            }
        }
    }

    // Belt half-width (cells) scales with map size so ranges are a plausible
    // fraction of a continent wide at any resolution.
    let belt_reach = (w as f32 * 0.045).clamp(14.0, 90.0);

    // Absolute feature wavelengths (in cells) -> feature COUNT scales with the map.
    let f_base = 1.0 / 760.0;   // broad continental swell
    let f_range = 1.0 / 210.0;  // ridge wavelength
    let f_hill = 1.0 / 52.0;    // fine hills
    let warp = 1.8f32;
    const RIDGE_AMP: f32 = 0.95;
    const HILL_AMP: f32 = 0.07;

    // D4: a plate boundary is a straight Voronoi edge, and every prior version
    // of this belt read `orogeny.dist` at the cell's OWN position, so however
    // much `belt_noise` below varies the belt's STRENGTH, its CREST still
    // traced that literal straight line -- the diagonal-line artefact named in
    // TERRAIN_2_PLAN.md's own evidence column. So belt cells instead sample the
    // orogeny field at a WARPED position: which boundary point "nearest" means
    // wanders smoothly rather than being the true nearest, bending the belt's
    // geometry the way a real orogen curves along an irregular margin.
    let oro_warp_strength = belt_reach * 0.9;
    let oro_warp_freq = 1.0 / (belt_reach * 2.6);

    // -- Distance-from-coast (full flood) for the coastal falloff --
    let mut coast_dist = vec![0u16; n];
    {
        let mut visited = vec![false; n];
        let mut queue = VecDeque::new();
        for i in 0..n {
            if terrain[i] != 1 { visited[i] = true; queue.push_back(i); }
        }
        while let Some(ci) = queue.pop_front() {
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            let d = coast_dist[ci];
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
                let ny = cy + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ni = (ny as u32 * w + nx as u32) as usize;
                if visited[ni] { continue; }
                visited[ni] = true;
                coast_dist[ni] = d.saturating_add(1);
                queue.push_back(ni);
            }
        }
    }

    // Physiographic provinces (see `landform.rs`): the per-cell relief
    // amplitude, roughness and feature scale that make one continent a mosaic
    // of different country instead of one global noise recipe repeated
    // everywhere. Built BEFORE the noise loop because the noise reads it.
    let lf = landform::build_landform_field(
        &terrain, w, h, seed.wrapping_add(0x1A4D), &coast_dist, &vec![0.0f32; n],
        orogeny.as_ref().map(|o| o.dist.as_slice()), belt_reach,
    );

    let mut elevation = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 { continue; }
            let ax = x as f32;
            let ay = y as f32;

            // Warped sample position for the orogeny AND rift lookups (D4):
            // which boundary point "nearest" means wanders smoothly instead
            // of being the true nearest, so neither reads as a straight line.
            let warped_idx = if have_boundary {
                let wxn = fbm_noise(ax * oro_warp_freq + 31.0, ay * oro_warp_freq + 7.0,
                                    seed.wrapping_add(0xD4D4_0001), 3, 2.0, 0.5) - 0.5;
                let wyn = fbm_noise(ax * oro_warp_freq + 91.0, ay * oro_warp_freq + 47.0,
                                    seed.wrapping_add(0xD4D4_0002), 3, 2.0, 0.5) - 0.5;
                let sx = ax + wxn * oro_warp_strength;
                let sy = (ay + wyn * oro_warp_strength).clamp(0.0, h as f32 - 1.0);
                let sxi = buf.wrap_x(sx.round() as i32);
                let syi = sy.round() as u32;
                Some(buf.idx(sxi, syi))
            } else {
                None
            };
            let (od, oset, oage) = match (&orogeny, warped_idx) {
                (Some(field), Some(sidx)) => (field.dist[sidx], field.setting[sidx], field.age[sidx]),
                _ => (u16::MAX, 0u8, 0.5f32),
            };

            // Belt strength + relative ridge amplitude from the real orogeny
            // setting (D2/D3/D4): an active margin's arc offset from the
            // trench, a broad collision, a narrow island arc -- no longer one
            // symmetric smoothstep for every geometry.
            let (mut belt, setting_amp) = if od != u16::MAX {
                (geology::belt_profile(od, oset, belt_reach), geology::setting_ridge_amp(oset))
            } else {
                (0.0, 1.0)
            };
            belt = belt * belt * (3.0 - 2.0 * belt); // smoothstep
            let belt_noise = fbm_noise(ax * f_base * 2.3 + 19.0, ay * f_base * 2.3 + 5.0,
                                       seed.wrapping_add(0xB317), 4, 2.0, 0.5);
            belt *= 0.35 + 0.65 * belt_noise;
            // Age modulates amplitude: a young orogen stands sharper/taller,
            // an old one is already worn down -- an old range beside a young
            // one is the plan's own "single biggest visual win", read
            // straight off the age this belt cell inherited in the BFS.
            let age_amp = if od != u16::MAX { 1.25 - oage * 0.5 } else { 1.0 };

            // Province character (landform.rs). `inv_scale` retunes the
            // WAVELENGTH of the local terms, `rugged` sets how much of the local
            // relief is ridge-like versus billowy, and `amp` its strength -- so
            // a shield province comes out as broad smooth swells and a massif
            // beside it as tight ridges, from the same three noise fields.
            //
            // `base` is deliberately NOT modulated: it carries the
            // continental-scale form (where the land is high at all), and a
            // province decides how rugged its ground is, not where the
            // continent stands.
            let lf_amp = lf.amp[idx];
            let lf_rug = lf.rugged[idx];
            let lf_detail = lf.detail[idx];

            let base = fbm_noise(ax * f_base + 3.1, ay * f_base + 7.7, seed, 5, 2.0, 0.5);
            let (rx, ry) = warped_coords(ax * f_range, ay * f_range, seed.wrapping_add(0x9E37), warp);
            let ridge = ridged_multifractal(rx, ry, seed.wrapping_add(0x48271), 7, 2.1, 2.0);
            // A SWELL field at the same wavelength as the ridges: smooth, rounded
            // relief, so "not rugged" is a real landform rather than merely less
            // ridge. Blending the two by the province's `rugged` gives a massif
            // sharp ridges and the shield beside it broad domes, out of the same
            // wavelength -- and unlike retuning the FREQUENCY per cell, blending
            // two fixed-frequency fields introduces no sampling artefacts (see
            // `Character::detail`).
            let swell = fbm_noise(rx * 0.85 + 2.3, ry * 0.85 + 6.1,
                                  seed.wrapping_add(0x5EED), 4, 2.0, 0.5);
            let hill = fbm_noise(ax * f_hill, ay * f_hill, seed.wrapping_add(0xFEED), 3, 2.0, 0.45);

            let shape = ridge * lf_rug + swell * (1.0 - lf_rug);
            let local = shape * belt * RIDGE_AMP * setting_amp * age_amp
                + hill * HILL_AMP * lf_detail;
            let mut e = base * 0.42 + local * lf_amp;

            // Divergent boundaries are rifts (continental rift valleys / nascent
            // ocean) -- pull the surface DOWN a little where crust is
            // stretching. Read at the SAME warped position as the orogeny
            // lookup above (not `boundary_type[idx]` directly, which used to
            // pull down a hard 1-cell-wide straight line exactly on the raw
            // Voronoi edge) and fade smoothly with distance rather than
            // switching on/off, so the rift reads as a valley, not a scar.
            if let Some(sidx) = warped_idx {
                let rd = rift_dist[sidx];
                if rd != u16::MAX {
                    let t = (1.0 - rd as f32 / 22.0).clamp(0.0, 1.0);
                    let t2 = t * t * (3.0 - 2.0 * t);
                    e *= 1.0 - 0.32 * t2;
                }
            }

            // NO VALLEY INCISION. See the "no valley carving" note above
            // `thermal_erosion`: an inverted ridged field is a DENDRITIC TREE by
            // construction, and subtracting one drew a branching dark scratch
            // network over every continent -- the thing this generator was
            // repeatedly asked to stop doing. Relief comes from the noise stack
            // and the tectonic terms alone; hillslope slumping rounds it.
            elevation[idx] = e.clamp(0.01, 1.5);
        }
    }


    // Coastal taper that KEEPS coastal mountains (an active margin where a
    // cordillera meets the sea): only the plain component is pulled toward the
    // shore, a genuine coastal ridge holds most of its height.
    const COAST_DIST: u16 = 4;
    for i in 0..n {
        if terrain[i] != 1 { continue; }
        if coast_dist[i] < COAST_DIST {
            let ratio = coast_dist[i] as f32 / COAST_DIST as f32;
            let taper = 0.45 + 0.55 * ratio;
            let ridge_keep = ((elevation[i] - 0.35) / 0.65).clamp(0.0, 1.0);
            elevation[i] *= taper.max(ridge_keep);
        }
    }

    // Transient geology (TERRAIN_2_PLAN.md section 4 slices 1-2): lithology +
    // tectonic-setting/age erodibility, the phase-2 climate proxy, and the
    // per-plate region id used to regionalise the hypsometric redistribution.
    let geo = geology::build_geo_context(buf, seed, &elevation, &coast_dist, orogeny.as_ref());

    // -- Erosion (stream power + thermal slump) then hypsometric match --
    let pre_erosion = elevation.clone();
    thermal_erosion(&mut elevation, &terrain, w, h, 3);
    // Isostatic adjustment (erosional rebound + mountain roots) before the rank-based
    // hypsometric redistribution reshapes the histogram.
    isostatic_adjust(&mut elevation, &terrain, &buf.boundary_type, &pre_erosion, w, h);

    let mut max_h = 0.0f32;
    for i in 0..n {
        if terrain[i] == 1 && elevation[i] > max_h { max_h = elevation[i]; }
    }
    if max_h > 0.0 {
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] /= max_h; }
        }
        // Moderate defaults for the plate path (the per-step sliders belong to the
        // template models); density biased by how much of the world is orogenic.
        let height = 0.5f32;
        let density = 0.5f32;
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] = elevation[i].powf(2.0 - height); }
        }
        let target_cap = 0.35 + height * 0.60;
        let mut sorted: Vec<f32> = (0..n).filter(|&i| terrain[i] == 1).map(|i| elevation[i]).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if !sorted.is_empty() {
            let p998 = sorted[(sorted.len() as f32 * 0.998) as usize].max(0.01);
            let cap_scale = target_cap / p998;
            for i in 0..n {
                if terrain[i] == 1 { elevation[i] = (elevation[i] * cap_scale).clamp(0.01, 1.0); }
            }
        }
        let target = build_target_histogram(height, density);
        redistribute_elevation_regional(&mut elevation, &terrain, n, &target, &geo.region_id, geo.region_count);
    }

    // Plateau rims and closed basins. AFTER the rank-based redistribution, which
    // would otherwise fan a flat plateau's tied cells back out across a band --
    // see `landform::apply_landform_shaping`.
    landform::apply_landform_shaping(&mut elevation, &terrain, &lf);

    // Terrain-aware micro-relief so no land area is ever a perfectly flat, mono
    // plateau (plateaus stay smooth, hillsides roll, floodplains stay flat).
    limit_grid_scale_relief(&mut elevation, &terrain, w, h);
    apply_micro_relief(&mut elevation, &terrain, w, h, seed.wrapping_add(0x31C7));

    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
}

/// Compute sea depth for ocean cells.
pub fn compute_sea_depth(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;

    let mut dist = vec![u32::MAX; buf.total()];
    let mut queue = VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] == 1 {
                dist[idx] = 0;
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let idx = buf.idx(x, y);
        let d = dist[idx];
        if d >= 100 { continue; }
        for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
            let ni = buf.idx(nx, ny);
            if dist[ni] > d + 1 {
                dist[ni] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    let max_dist = 80.0f32;
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] == 0 {
                let d = dist[idx].min(100) as f32;
                let depth = if d <= 5.0 {
                    d / 5.0 * 0.15
                } else if d <= 20.0 {
                    0.15 + (d - 5.0) / 15.0 * 0.50
                } else {
                    (0.65 + (d - 20.0) / max_dist * 0.35).min(1.0)
                };
                buf.sea_depth[idx] = depth;
                buf.is_shelf[idx] = if depth < 0.15 { 1 } else { 0 };
                buf.is_shelf_edge[idx] = if (0.12..0.18).contains(&depth) { 1 } else { 0 };
            } else {
                buf.sea_depth[idx] = 0.0;
                buf.is_shelf[idx] = 0;
                buf.is_shelf_edge[idx] = 0;
            }
        }
    }
}

/// BFS distance (in cells) from an ocean cell to the nearest land cell whose
/// `boundary_type` is in `types`: first a bounded land-side BFS out to
/// `land_reach` from those boundary cells, then a bounded ocean-side flood out
/// to `cap` from ocean cells directly touching that land. Shared by the
/// active-margin shelf pinch and, since Terrain 2.0 slice 5 (D11), trench and
/// mid-ocean-ridge placement -- each cares about a different boundary subset
/// of the same underlying sweep. `[]` when there is no plate data at all.
fn ocean_distance_from_boundary(buf: &WorldBuffer, types: &[u8], land_reach: u16, cap: u16) -> Vec<u16> {
    if buf.boundary_type.is_empty() {
        return Vec::new();
    }
    let w = buf.width;
    let h = buf.height;
    let mut bdist = vec![u16::MAX; buf.total()];
    let mut q = VecDeque::new();
    for i in 0..buf.total() {
        if buf.terrain[i] == 1 && types.contains(&buf.boundary_type[i]) {
            bdist[i] = 0;
            q.push_back(i);
        }
    }
    while let Some(ci) = q.pop_front() {
        let d = bdist[ci];
        if d >= land_reach { continue; }
        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(cx + dx);
            let ny = (cy + dy).clamp(0, h as i32 - 1) as u32;
            let ni = buf.idx(nx, ny);
            if buf.terrain[ni] == 1 && bdist[ni] > d + 1 {
                bdist[ni] = d + 1;
                q.push_back(ni);
            }
        }
    }
    let mut adist = vec![u16::MAX; buf.total()];
    let mut oq = VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { continue; }
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
                if buf.terrain[ni] == 1 && bdist[ni] != u16::MAX {
                    adist[i] = 0;
                    oq.push_back(i);
                    break;
                }
            }
        }
    }
    while let Some(ci) = oq.pop_front() {
        let d = adist[ci];
        if d >= cap { continue; }
        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(cx + dx);
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] == 0 && adist[ni] > d + 1 {
                adist[ni] = d + 1;
                oq.push_back(ni);
            }
        }
    }
    adist
}

/// Generate continental shelves with configurable parameters.
pub fn generate_shelves(
    buf: &mut WorldBuffer, seed: u64,
    shelf_width: f32, noise_amount: f32, depth_profile: f32, dropoff_width: f32,
) {
    let w = buf.width;
    let h = buf.height;
    let base_width = shelf_width.clamp(1.0, 20.0);
    let drop_w = dropoff_width.clamp(1.0, 20.0);

    let mut dist = vec![u32::MAX; buf.total()];
    // Slice 7's relief-proxy signal (below): each SEA cell inherits, from the
    // BFS that finds its nearest coast, the RELIEF of the land it drains from --
    // not just whatever land happens to sit in a fixed small window around the
    // sea cell itself. A per-sea-cell fixed-radius scan only ever "sees" land for
    // the first few cells off the coast, so a shelf's outer half always fell back
    // to the same default regardless of which margin it belonged to. Propagating
    // the value outward through the same BFS that computes `dist` fixes that by
    // construction: every sea cell all the way to the shelf edge carries the
    // relief of the actual coast it came from.
    let mut coast_relief = vec![-1.0f32; buf.total()];
    let mut queue = VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] == 1 {
                dist[idx] = 0;
                coast_relief[idx] = buf.elevation[idx];
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let idx = buf.idx(x, y);
        let d = dist[idx];
        if d >= 100 { continue; }
        let relief = coast_relief[idx];
        for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
            let ni = buf.idx(nx, ny);
            if dist[ni] > d + 1 {
                dist[ni] = d + 1;
                coast_relief[ni] = relief;
                queue.push_back((nx, ny));
            }
        }
    }

    // â”€â”€ Active vs passive margins â”€â”€ An ACTIVE margin (a coast riding a
    // convergent/transform plate boundary â€” the Pacific "Ring of Fire" geometry)
    // has a NARROW, steep shelf plunging to a trench; a PASSIVE margin (trailing
    // edge, no nearby boundary â€” the Atlantic geometry) builds a BROAD shelf. We
    // find ocean cells whose nearest coast is active and shrink their shelf.
    // Falls back to all-passive when no plate data is loaded (template worlds).
    let ar = (w as f32 * 0.02).clamp(6.0, 40.0) as u16;
    let cap = (base_width + drop_w + 6.0) as u16;
    let active_dist = ocean_distance_from_boundary(buf, &[1, 3], ar, cap);
    // Terrain 2.0 slice 5 (D11): a mid-ocean ridge follows a DIVERGENT boundary
    // and a trench follows a CONVERGENT one specifically -- narrower than the
    // combined active-margin reach above, since a real ridge/trench is a sharp
    // feature, not the whole margin.
    let ridge_dist = ocean_distance_from_boundary(buf, &[2], ar, 28);
    let trench_dist = ocean_distance_from_boundary(buf, &[1], ar, 16);

    let noise_scale = w.max(h) as f32 / 20.0;
    // TECTONICS_RIVERS_PROVINCES_PLAN.md Slice 7 (F6): a second, much longer
    // wavelength noise term (~1/4 the world, i.e. thousands of km) so shelf width
    // varies at a continental scale too, not just the single ~1/20-world field
    // above -- before this every shelf on a world varied only around one narrow
    // band, which is why margins came out near-uniform width.
    let noise_scale_broad = w.max(h) as f32 / 4.0;
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 {
                buf.sea_depth[idx] = 0.0;
                buf.is_shelf[idx] = 0;
                buf.is_shelf_edge[idx] = 0;
                continue;
            }
            let d = dist[idx].min(100) as f32;
            if d < 1.0 {
                buf.sea_depth[idx] = 0.0;
                buf.is_shelf[idx] = 1;
                buf.is_shelf_edge[idx] = 0;
                continue;
            }
            let noise = fbm_noise(x as f32 / noise_scale, y as f32 / noise_scale, seed, 3, 2.0, 0.5);
            let noise_broad = fbm_noise(x as f32 / noise_scale_broad, y as f32 / noise_scale_broad, seed ^ 0x0FEE_7B0A, 3, 2.0, 0.5);
            let combined_noise = noise * 0.55 + noise_broad * 0.45;
            let local_width = (base_width * (1.0 + (combined_noise - 0.5) * 2.0 * noise_amount)).max(1.0);

            let mut land_count = 0u32;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    if buf.terrain[ni] == 1 { land_count += 1; }
                }
            }
            let gentle_factor = 1.0 + (land_count as f32 / 49.0) * 0.5;
            // Slice 7: a CONTINUOUS margin maturity, not the old binary 0.5/1.0
            // step. A margin's true "age" (how long it has been passive,
            // accumulating sediment) isn't tracked yet, so this reads it off two
            // proxies instead, exactly the "documented proxy" convention
            // `geology.rs`'s phase-2 climate term already uses:
            //  - with plate data: how close the coast sits to an active boundary,
            //    smoothly faded over the active-margin reach rather than a hard cut;
            //  - without plate data (a template/painted world, `active_dist` empty):
            //    nearby land RELIEF stands in for tectonic activity -- a mountainous
            //    coast reads as an active, scraped margin (narrow shelf), a low flat
            //    coastal plain as a mature passive one (broad shelf). This is what
            //    stops the from-landmass path giving every coast the identical shelf.
            let margin_factor = if !active_dist.is_empty() && active_dist[idx] != u16::MAX {
                let ad = active_dist[idx] as f32;
                let t = ((ad - d) / (ar as f32).max(1.0)).clamp(0.0, 1.0);
                // t=0 (right at the boundary) -> 0.35 (steep, pinched shelf);
                // t=1 (far from any boundary) -> 1.0 (full passive-margin width).
                0.35 + 0.65 * (t * t * (3.0 - 2.0 * t))
            } else if coast_relief[idx] >= 0.0 {
                // `coast_relief[idx]` is the elevation of the SPECIFIC coastal
                // land cell this sea cell's nearest-coast BFS came from -- valid
                // all the way to the shelf edge, unlike the small fixed-radius
                // `land_elev_sum` window above (which only sees land for the
                // first few cells off the coast and reverts to the flat default
                // for the rest of the shelf).
                // ~0 (flat coastal plain) -> 1.0; ~0.35 normalized elev (mountainous
                // coast) or higher -> 0.5. Clamped so a small local peak can't pinch
                // an otherwise broad passive shelf to nothing.
                (1.0 - (coast_relief[idx] / 0.35).clamp(0.0, 1.0) * 0.5).clamp(0.5, 1.0)
            } else {
                1.0
            };
            let effective_width = local_width * gentle_factor * margin_factor;

            let depth = if d <= effective_width {
                let t = (d - 1.0) / (effective_width - 1.0).max(1.0);
                let linear = t * 0.25;
                let exponential = (1.0 - (-3.0 * t).exp()) * 0.25;
                linear * (1.0 - depth_profile) + exponential * depth_profile
            } else if d <= effective_width + drop_w {
                0.25 + ((d - effective_width) / drop_w) * 0.40
            } else {
                (0.65 + (d - effective_width - drop_w) * 0.025).min(1.0)
            };

            buf.sea_depth[idx] = depth;
            // Wider shelf: tag the whole gentle-slope band (out to the start of
            // the steep drop-off) as shelf, not just the very shallowest water,
            // so the continental shelf reads as a broad apron rather than a thin
            // coastal ring.
            buf.is_shelf[idx] = if depth < 0.24 { 1 } else { 0 };
            buf.is_shelf_edge[idx] = if (0.20..0.28).contains(&depth) { 1 } else { 0 };
        }
    }

    // â”€â”€ Terrain 2.0 slice 5 (docs/TERRAIN_2_PLAN.md D11): seafloor structure.
    // Everything above is a pure function of distance-to-coast; real ocean
    // floor also carries a mid-ocean ridge along a divergent boundary, a
    // trench at a subducting convergent one, abyssal-hill texture in the deep,
    // and scattered seamounts/guyots -- none of which distance-to-coast alone
    // can produce. Applied past the shelf/dropoff so it never fights the
    // coastal profile above; each term is bounded and additive so a world with
    // no plate data (ridge_dist/trench_dist empty) just skips those two terms.
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 { continue; }
            let mut depth = buf.sea_depth[idx];
            let ax = x as f32;
            let ay = y as f32;

            if !ridge_dist.is_empty() && ridge_dist[idx] != u16::MAX {
                let rd = ridge_dist[idx] as f32;
                const REACH: f32 = 24.0;
                if rd < REACH {
                    let t = (1.0 - rd / REACH).clamp(0.0, 1.0);
                    // Along-ridge segmentation: transform offsets read as gaps
                    // in the crest rather than one unbroken seam.
                    let seg = fbm_noise(ax * 0.02 + 3.0, ay * 0.02 + 9.0, seed.wrapping_add(0x51DE_51DE), 3, 2.0, 0.5);
                    let seg_gate = (0.25 + 0.9 * seg).clamp(0.0, 1.0);
                    depth -= 0.30 * t * t * seg_gate;
                }
            }
            if !trench_dist.is_empty() && trench_dist[idx] != u16::MAX {
                let td = trench_dist[idx] as f32;
                const REACH: f32 = 13.0;
                if td < REACH {
                    let t = (1.0 - td / REACH).clamp(0.0, 1.0);
                    depth += 0.30 * t * t * t;
                }
            }
            // Abyssal hills: fine noise texture, everywhere, scaled by depth so
            // the shelf itself stays smooth and only the deep floor textures.
            let hills = fbm_noise(ax * 0.09 + 41.0, ay * 0.09 + 17.0, seed.wrapping_add(0x0AB7_0AB7), 3, 2.0, 0.5) - 0.5;
            depth += hills * 0.05 * depth.clamp(0.0, 1.0);
            // Scattered seamounts/guyots: sparse hotspot noise peaks read as
            // isolated shallow bumps in otherwise deep water (not traced as
            // literal chains -- a documented simplification of the plan's
            // "seamount chains").
            let hotspot = fbm_noise(ax * 0.05 + 91.0, ay * 0.05 + 37.0, seed.wrapping_add(0x5EA5_5EA5), 2, 2.0, 0.5);
            if hotspot > 0.80 && depth > 0.5 {
                let bump = ((hotspot - 0.80) / 0.20).clamp(0.0, 1.0);
                depth -= 0.22 * bump * bump;
            }

            buf.sea_depth[idx] = depth.clamp(0.0, 1.0);
            buf.is_shelf[idx] = if buf.sea_depth[idx] < 0.24 { 1 } else { 0 };
        }
    }
}

/// Generate realistic elevation from terrain (land/sea) data alone.
/// Port of WF1 random-elevation.ts: multi-octave noise + ridged multifractal
/// + domain warping + hydraulic erosion + thermal erosion.
/// Produces continuous mountain ridges like Earth's geography.
pub fn generate_elevation_from_terrain(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    let density = mountain_density.clamp(0.0, 1.0);
    let height = mountain_height.clamp(0.0, 1.0);
    let spread = mountain_spread.clamp(0.0, 1.0);
    let roughness = noise_roughness.clamp(0.0, 1.0);

    let terrain = buf.terrain.clone();

    // â”€â”€ Step 1: Generate base heightmap from multi-layer noise â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    let scale = w.max(h) as f32 / 8.0;

    // Noise weights â€” density/height/roughness control the mix
    let w_large = 0.55;
    let w_medium = 0.30;
    let w_small = 0.10 + roughness * 0.10;   // 0.10-0.20
    let w_ridge = 0.15 + density * 0.20;      // 0.15-0.35 (more density â†’ more ridges)
    let total_w = w_large + w_medium + w_small + w_ridge;
    let n_large = w_large / total_w;
    let n_medium = w_medium / total_w;
    let n_small = w_small / total_w;
    let n_ridge = w_ridge / total_w;

    let warp_strength = 0.3 + roughness * 0.3; // 0.3-0.6
    let med_scale = 2.5;
    // Mountain spread â†’ ridge frequency: narrow peaks (0) use a higher frequency
    // (tight, isolated ranges), wide ranges (1) a lower frequency (broad, long
    // cordillera). Spans roughly med_scaleÃ—1.7 â€¦ med_scaleÃ—0.6.
    let ridge_scale = med_scale * (1.7 - spread * 1.1);

    // â”€â”€ ABSOLUTE-wavelength interior relief (measured in CELLS, not map fractions).
    // The `scale`-based fields above tie every feature to the map size, so the
    // interior of a large continent spans barely one noise period and comes out a
    // smooth dome â€” which the hypsometric redistribution then flattens into a
    // uniform "green blob" (the user's report). These absolute-frequency belts +
    // hills add ranges and texture whose COUNT scales with continent AREA (the same
    // trick as generate_elevation_ridged), so interiors are never featureless.
    let f_belt_abs = 1.0 / 540.0;                     // orogenic-belt spacing (cells)
    let f_range_abs = 1.0 / (120.0 + spread * 230.0); // ridge wavelength (cells)
    let f_hill_abs = 1.0 / 52.0;                       // fine hills (cells)
    let warp_abs = 1.4 + roughness * 1.4;
    let ridge_amp_abs = 0.35 + density * 0.55;
    let hill_amp_abs = 0.05 + roughness * 0.12;

    // â”€â”€ Step 1a: distance-from-coast for every land cell â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Computed up front so interior cells can be lifted into a broad
    // continental rise â€” otherwise low-frequency noise leaves whole
    // interiors near-flat and only the coasts show relief.
    let mut coast_dist = vec![0u16; n];
    {
        let mut visited = vec![false; n];
        let mut queue = VecDeque::new();
        for i in 0..n {
            if terrain[i] != 1 {
                visited[i] = true;
                queue.push_back(i);
            }
        }
        while let Some(ci) = queue.pop_front() {
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            let d = coast_dist[ci];
            // Full flood â€” no distance cap. The old `d >= 250` early-out left the
            // deep interior of large continents unvisited (coast_dist stuck at 0),
            // so Step-2's coastal falloff multiplied those cells by 0.15 and
            // produced a flat low-elevation "green blob" with a sharp BFS-contour
            // edge. Clamp the stored value so it stays within u16.
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
                let ny = cy + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ni = (ny as u32 * w + nx as u32) as usize;
                if visited[ni] { continue; }
                visited[ni] = true;
                coast_dist[ni] = d.saturating_add(1);
                queue.push_back(ni);
            }
        }
    }

    // Distance at which the continental rise saturates (scales with world size).
    let inland_ref = (w.max(h) as f32 * 0.025).clamp(18.0, 60.0);

    // `inland_ref` retained for API symmetry; the WF1 algorithm shapes interiors
    // through histogram redistribution rather than a synthetic dome.
    let _ = inland_ref;

    // Physiographic provinces (landform.rs) -- the same regional mosaic the
    // plate model gets. No plate data on this path, so the archetype picker
    // falls back to local relief as an orogeny stand-in (its documented
    // fiction, never dressed up as real tectonics).
    let lf = landform::build_landform_field(
        &terrain, w, h, seed.wrapping_add(0x1A4D), &coast_dist, &vec![0.0f32; n], None, 0.0,
    );

    let mut elevation = vec![0.0f32; n];

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 { continue; }

            let nx = x as f32 / scale;
            let ny = y as f32 / scale;

            // Domain warping for organic shapes
            let (wnx, wny) = warped_coords(nx, ny, seed.wrapping_add(99999), warp_strength);

            // Large features (continental shapes)
            let large = fbm_noise(wnx, wny, seed, 6, 2.0, 0.5);

            // Medium features (mountain ranges)
            let medium = fbm_noise(wnx * med_scale, wny * med_scale, seed.wrapping_add(31337), 4, 2.2, 0.45);

            // Small features (hills) â€” two scales so interiors keep fine
            // texture the river router can follow into natural channels instead
            // of sheet-flowing across a smooth plain.
            let small_a = fbm_noise(wnx * 6.0, wny * 6.0, seed.wrapping_add(65521), 3, 2.0, 0.4);
            let small_b = fbm_noise(wnx * 13.0, wny * 13.0, seed.wrapping_add(0xF19E), 3, 2.0, 0.42);
            let small = small_a * 0.62 + small_b * 0.38;

            // Ridged multifractal â€” elongated ridge lines (the key for mountain chains).
            // Frequency set by mountain_spread (narrow peaks â†” wide ranges).
            let ridge = ridged_multifractal(wnx * ridge_scale, wny * ridge_scale, seed.wrapping_add(48271), 6, 2.1, 2.0);

            // Absolute-wavelength interior relief (cell-frequency belts + hills), so
            // even a huge continent interior gets ranges and texture instead of a
            // smooth dome. Belt-masked so ranges concentrate into plausible orogenic
            // bands rather than blanketing the whole interior.
            let ax = x as f32;
            let ay = y as f32;
            let belt_raw = fbm_noise(ax * f_belt_abs + 11.0, ay * f_belt_abs + 4.0,
                                     seed.wrapping_add(0xB317), 4, 2.0, 0.5);
            let mut belt = ((belt_raw - 0.46) / 0.26).clamp(0.0, 1.0);
            belt = belt * belt * (3.0 - 2.0 * belt); // smoothstep
            let (arx, ary) = warped_coords(ax * f_range_abs, ay * f_range_abs,
                                           seed.wrapping_add(0x9E37), warp_abs);
            let ridge_abs = ridged_multifractal(arx, ary, seed.wrapping_add(0x48271), 7, 2.1, 2.0);
            let hill_abs = fbm_noise(ax * f_hill_abs, ay * f_hill_abs,
                                     seed.wrapping_add(0xFEED), 3, 2.0, 0.45);
            let abs_relief = ridge_abs * belt * ridge_amp_abs + hill_abs * hill_amp_abs;

            // Combine with normalized weights (WF1 generateBaseHeightmap), then fold
            // in the absolute-scale interior relief. The map-scaled part keeps the
            // continental-scale structure (where it's high vs low); the absolute part
            // supplies the local ranges/hills that break up the interior. Normalize +
            // redistribution downstream only care about the RELATIVE pattern, so this
            // injects real rank variation into interiors â†’ visible relief after
            // redistribution instead of one flat band.
            // Province character. `large` is left alone -- it carries the
            // continental form (where the land is high at all); the province
            // decides how RUGGED its ground is and at what scale, not where the
            // continent stands. `swell` is the smooth companion at the ridge
            // wavelength, so "not rugged" is a real landform and not merely less
            // ridge; blending two FIXED-frequency fields avoids the sampling
            // artefacts a per-cell frequency multiplier produces (see
            // `landform::Character::detail`).
            let lf_amp = lf.amp[idx];
            let lf_rug = lf.rugged[idx];
            let lf_detail = lf.detail[idx];
            let swell = fbm_noise(wnx * ridge_scale * 0.85 + 4.1, wny * ridge_scale * 0.85 + 2.7,
                                  seed.wrapping_add(0x5EED), 4, 2.0, 0.5);
            let shaped_ridge = ridge * lf_rug + swell * (1.0 - lf_rug);

            let combined = large * n_large
                + (medium * n_medium
                   + small * n_small * lf_detail
                   + shaped_ridge * n_ridge
                   + abs_relief * 0.55) * lf_amp;

            // â”€â”€ Valley incision â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            // A second ridged field at higher frequency, INVERTED, carves
            // dendritic valley networks between the ranges (the troughs the
            // erosion alone left too shallow). Scaled by the local height so
            // highlands get dissected into ridge-and-valley relief while
            // lowlands stay broad. This is what was missing â€” "almost no valleys".
            // NO VALLEY INCISION -- see the note above `thermal_erosion`. This
            // path's absolute `fine_carve` term was the most visible of the lot:
            // an inverted ridged field subtracted EVERYWHERE, at full strength on
            // flat interiors, which is precisely a dendritic drainage tree drawn
            // across every plain on the map.
            elevation[idx] = combined.clamp(0.01, 1.0);
        }
    }

    // â”€â”€ Step 2: Coastal falloff â€” gentle shoreline taper that KEEPS coastal
    // mountains. The old falloff multiplied the outer ring down to 0.15, which
    // flattened every coast into a plain â€” even active margins where a cordillera
    // meets the sea (Andes, Norway, BC). Now the taper only pulls DOWN the low /
    // plain component: a genuine coastal ridge (high raw height) keeps most of its
    // elevation, while flats still ramp gently from the shore so there's no cliff.
    const COAST_DIST: u16 = 4;
    for i in 0..n {
        if terrain[i] != 1 { continue; }
        if coast_dist[i] < COAST_DIST {
            let ratio = coast_dist[i] as f32 / COAST_DIST as f32; // 0 shore .. 1 inland
            let taper = 0.45 + 0.55 * ratio;                      // shore keeps â‰¥45%
            let ridge_keep = ((elevation[i] - 0.35) / 0.65).clamp(0.0, 1.0); // mountainous?
            let factor = taper.max(ridge_keep);
            elevation[i] *= factor;
        }
    }

    // â”€â”€ Step 3: Hydraulic erosion â€” droplet simulation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Scale iterations with world size (small worlds ~15K, large ~100K)
    let pre_erosion = elevation.clone();
    // No real plate data on this path -- the relief pseudo-setting only
    // (TERRAIN_2_PLAN.md section 2's documented fiction, never real polarity/age).
    let geo = geology::build_geo_context(buf, seed, &pre_erosion, &coast_dist, None);

    // â”€â”€ Step 4: Thermal erosion â€” smooth sharp ridges â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Fewer passes than before (2-4) so the carved valley networks aren't
    // smoothed back out â€” thermal slump fills valleys, so over-applying it was a
    // second reason interiors read as flat.
    let thermal_passes = 2 + (roughness * 2.0) as u32; // 2-4 passes
    thermal_erosion(&mut elevation, &terrain, w, h, thermal_passes);
    isostatic_adjust(&mut elevation, &terrain, &buf.boundary_type, &pre_erosion, w, h);

    // â”€â”€ Step 5: Normalize with realistic altitude distribution â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Power curve pushes most land lower, percentile cap prevents every world
    // from having an 8848m peak
    let mut max_h = 0.0f32;
    for i in 0..n {
        if terrain[i] == 1 && elevation[i] > max_h { max_h = elevation[i]; }
    }
    if max_h > 0.0 {
        // Normalize to 0-1
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] /= max_h; }
        }
        // Power curve: exponent controlled by height parameter
        // Low height (0.1) â†’ exponent 2.0 (very flat), high (1.0) â†’ exponent 1.0 (tall peaks)
        let exponent = 2.0 - height;
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] = elevation[i].powf(exponent); }
        }
        // Percentile cap: find 99.8th percentile and scale to target
        // height parameter controls the target: 0.1 â†’ cap at 0.4 (~3500m), 1.0 â†’ cap at 0.95 (~8400m)
        let target_cap = 0.35 + height * 0.60; // 0.35-0.95
        let mut sorted: Vec<f32> = (0..n)
            .filter(|&i| terrain[i] == 1)
            .map(|i| elevation[i])
            .collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if !sorted.is_empty() {
            let p998 = sorted[(sorted.len() as f32 * 0.998) as usize].max(0.01);
            let cap_scale = target_cap / p998;
            for i in 0..n {
                if terrain[i] == 1 {
                    elevation[i] = (elevation[i] * cap_scale).clamp(0.01, 1.0);
                }
            }
        }

        // â”€â”€ Step 5b: Histogram redistribution (WF1 redistributeElevation) â”€â”€â”€
        // This is what gives WF1 its realistic, fully-differentiated terrain â€”
        // it spreads land elevations across 1000 m bands to a target hypsometric
        // curve (preserving relative order), so interiors are never a flat patch.
        // The `height` slider interpolates the target between a low, coastal
        // world and a dramatic alpine one; `density` biases toward more highland.
        let target = build_target_histogram(height, density);
        redistribute_elevation_regional(&mut elevation, &terrain, n, &target, &geo.region_id, geo.region_count);
    }

    // Plateau rims and closed basins -- AFTER the rank remap, which would
    // otherwise fan a flat plateau back out across a band.
    landform::apply_landform_shaping(&mut elevation, &terrain, &lf);

    // â”€â”€ Step 6: Terrain-aware micro-relief, then write back to buffer â”€â”€â”€â”€
    limit_grid_scale_relief(&mut elevation, &terrain, w, h);
    apply_micro_relief(&mut elevation, &terrain, w, h, seed.wrapping_add(0x31C7));
    for i in 0..n {
        if terrain[i] == 1 {
            buf.elevation[i] = elevation[i];
        } else {
            buf.elevation[i] = 0.0;
        }
    }
}

/// Plate-free, WORLD-SIZE-AWARE elevation model. Unlike
/// `generate_elevation_from_terrain` (whose feature size scales with the map, so
/// big worlds get only a few giant ranges and look best with the flat preset),
/// this model uses ABSOLUTE feature wavelengths measured in cells â€” so the number
/// of mountain ranges grows with the map and a world-size map gets many dispersed
/// ridged cordillera. Ranges are concentrated into plausible orogenic BELTS by a
/// low-frequency mask (no plates needed), then carved by the same hydraulic +
/// thermal erosion and matched to a realistic hypsometric curve.
pub fn generate_elevation_ridged(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let density = mountain_density.clamp(0.0, 1.0);
    let height = mountain_height.clamp(0.0, 1.0);
    let spread = mountain_spread.clamp(0.0, 1.0);
    let roughness = noise_roughness.clamp(0.0, 1.0);
    let terrain = buf.terrain.clone();

    // Absolute feature wavelengths (in cells) â†’ feature COUNT scales with map size.
    let f_base = 1.0 / 760.0;                       // broad continental swells
    let f_belt = 1.0 / 540.0;                       // orogenic-belt spacing
    let f_range = 1.0 / (120.0 + spread * 230.0);   // ridge wavelength (narrowâ†”broad)
    let f_hill = 1.0 / 52.0;                         // fine hills
    let warp = 1.4 + roughness * 1.4;
    let ridge_amp = 0.35 + density * 0.55;
    let hill_amp = 0.05 + roughness * 0.12;

    // â”€â”€ Distance-from-coast (full flood) for the coastal falloff below â”€â”€
    let mut coast_dist = vec![0u16; n];
    {
        let mut visited = vec![false; n];
        let mut queue = VecDeque::new();
        for i in 0..n {
            if terrain[i] != 1 { visited[i] = true; queue.push_back(i); }
        }
        while let Some(ci) = queue.pop_front() {
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            let d = coast_dist[ci];
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
                let ny = cy + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ni = (ny as u32 * w + nx as u32) as usize;
                if visited[ni] { continue; }
                visited[ni] = true;
                coast_dist[ni] = d.saturating_add(1);
                queue.push_back(ni);
            }
        }
    }

    // â”€â”€ Compose: continental base + belt-masked ridged ranges + fine hills â”€â”€
    let mut elevation = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 { continue; }
            let ax = x as f32;
            let ay = y as f32;
            let base = fbm_noise(ax * f_base + 3.1, ay * f_base + 7.7, seed, 5, 2.0, 0.5);
            // Orogenic belt mask: where mountain ranges concentrate.
            let belt_raw = fbm_noise(ax * f_belt + 11.0, ay * f_belt + 4.0, seed.wrapping_add(0xB317), 4, 2.0, 0.5);
            let mut belt = ((belt_raw - 0.46) / 0.26).clamp(0.0, 1.0);
            belt = belt * belt * (3.0 - 2.0 * belt); // smoothstep
            // Ridged ranges, domain-warped into organic curving chains.
            let (rx, ry) = warped_coords(ax * f_range, ay * f_range, seed.wrapping_add(0x9E37), warp);
            let ridge = ridged_multifractal(rx, ry, seed.wrapping_add(0x48271), 7, 2.1, 2.0);
            let hill = fbm_noise(ax * f_hill, ay * f_hill, seed.wrapping_add(0xFEED), 3, 2.0, 0.45);
            let e = base * 0.5 + ridge * belt * ridge_amp + hill * hill_amp;

            // NO VALLEY INCISION -- see the note above `thermal_erosion`.
            elevation[idx] = e.clamp(0.01, 1.5);
        }
    }

    // â”€â”€ Coastal falloff that KEEPS coastal mountains (only the plain component is
    // tapered toward the shore â€” see generate_elevation_from_terrain). â”€â”€
    const COAST_DIST: u16 = 4;
    for i in 0..n {
        if terrain[i] != 1 { continue; }
        if coast_dist[i] < COAST_DIST {
            let ratio = coast_dist[i] as f32 / COAST_DIST as f32;
            let taper = 0.45 + 0.55 * ratio;
            let ridge_keep = ((elevation[i] - 0.35) / 0.65).clamp(0.0, 1.0);
            let factor = taper.max(ridge_keep);
            elevation[i] *= factor;
        }
    }

    // â”€â”€ Erosion (hydraulic droplets + thermal slump) â”€â”€
    let pre_erosion = elevation.clone();
    let geo = geology::build_geo_context(buf, seed, &pre_erosion, &coast_dist, None);
    let thermal_passes = 2 + (roughness * 2.0) as u32; // fewer passes so valleys survive
    thermal_erosion(&mut elevation, &terrain, w, h, thermal_passes);
    isostatic_adjust(&mut elevation, &terrain, &buf.boundary_type, &pre_erosion, w, h);

    // â”€â”€ Normalize + hypsometric redistribution (realistic altitude spread) â”€â”€
    let mut max_h = 0.0f32;
    for i in 0..n {
        if terrain[i] == 1 && elevation[i] > max_h { max_h = elevation[i]; }
    }
    if max_h > 0.0 {
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] /= max_h; }
        }
        let exponent = 2.0 - height;
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] = elevation[i].powf(exponent); }
        }
        let target_cap = 0.35 + height * 0.60;
        let mut sorted: Vec<f32> = (0..n).filter(|&i| terrain[i] == 1).map(|i| elevation[i]).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if !sorted.is_empty() {
            let p998 = sorted[(sorted.len() as f32 * 0.998) as usize].max(0.01);
            let cap_scale = target_cap / p998;
            for i in 0..n {
                if terrain[i] == 1 { elevation[i] = (elevation[i] * cap_scale).clamp(0.01, 1.0); }
            }
        }
        let target = build_target_histogram(height, density);
        redistribute_elevation_regional(&mut elevation, &terrain, n, &target, &geo.region_id, geo.region_count);
    }

    // Terrain-aware micro-relief (plateaus smooth, hillsides roll, flats flat).
    limit_grid_scale_relief(&mut elevation, &terrain, w, h);
    apply_micro_relief(&mut elevation, &terrain, w, h, seed.wrapping_add(0x31C7));

    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
}

// ── Cordillera ──────────────────────────────────────────────────────────────

/// One traced mountain spine: a continuous crest line plus the reference
/// distance-from-coast it was traced along.
struct Spine {
    /// Crest cells in order along strike.
    points: Vec<(u32, u32)>,
    /// Distance-from-coast the crest holds — the reference that tells a cell
    /// whether it sits on the seaward or the inland flank.
    ref_coast: Vec<u16>,
}

/// Generate a **cordillera**: one or more long, continuous mountain chains that
/// run parallel to a continental margin, in the manner of the Andes, the Rockies
/// or the Sierra Madre.
///
/// This is structurally different from `generate_elevation_ridged`, which fills a
/// blobby noise mask with isotropic ridged multifractal — statistically mountainous,
/// but with no chain, no crest line, no consistent strike and no rain-shadow side.
/// A cordillera has all four, and they are what a reader of the map actually sees:
///
/// 1. **A traced spine.** Crests are walked as polylines along an iso-contour of
///    distance-from-coast, so the chain genuinely follows the coastline the way a
///    subduction-margin orogen does, instead of being wherever noise happened to
///    peak.
/// 2. **A continental divide.** The spine is continuous, so rivers part along it
///    and the drainage map inherits a real watershed backbone.
/// 3. **Asymmetric flanks.** The seaward side drops steeply to a narrow coastal
///    plain; the inland side lets down through a broad piedmont apron of
///    foothills. (Andes: a few tens of km to the Pacific, hundreds into the
///    Amazon basin.) The side is decided by comparing a cell's own
///    distance-from-coast to the crest's.
/// 4. **Parallel sub-ranges.** A cordillera is a *system* of ranges — Occidental,
///    Central, Oriental — separated by high intermontane basins. A cross-strike
///    modulation puts 2–3 sub-crests inside the envelope with plateaus between.
///
/// Along-strike the crest rises and falls, so the chain has summits and saddles
/// (passes) rather than a uniform wall.
///
/// Sliders: `mountain_density` → number of chains and sub-ranges,
/// `mountain_height` → summit altitude, `mountain_spread` → chain width and how
/// far inland it sits, `noise_roughness` → crest serration and erosion depth.
pub fn generate_elevation_cordillera(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let density = mountain_density.clamp(0.0, 1.0);
    let height = mountain_height.clamp(0.0, 1.0);
    let spread = mountain_spread.clamp(0.0, 1.0);
    let roughness = noise_roughness.clamp(0.0, 1.0);
    let terrain = buf.terrain.clone();

    // ── Distance-from-coast: the field the whole method is built on ──
    let coast_dist = coast_distance(&terrain, w, h);

    // ── Geometry, in CELLS, so a chain's proportions scale with the map ──
    // Half-width of the high range itself, and of the foothill apron beyond it.
    let crest_offset = 5.0 + spread * 22.0;      // how far inland the crest runs
    let range_half = 4.0 + spread * 14.0;        // high-range half-width
    let piedmont = range_half * (2.2 + spread * 1.8); // inland apron reach
    let seaward = range_half * 0.85;             // steep seaward flank reach
    let search_radius = (piedmont.max(seaward) + 4.0) as u16;

    // ── Trace the chains ──
    let spines = trace_spines(
        &terrain, &coast_dist, w, h, seed, crest_offset, density,
    );

    // ── Distance to the nearest crest, carrying that crest's reference data ──
    // A bounded multi-source BFS: cost is O(seeded area × search_radius), not
    // O(world × spine length).
    let mut sp_dist = vec![u16::MAX; n];
    let mut sp_ref_coast = vec![0u16; n];
    let mut sp_amp = vec![0.0f32; n];
    {
        let mut queue = VecDeque::new();
        for sp in &spines {
            let len = sp.points.len().max(1) as f32;
            for (k, &(px, py)) in sp.points.iter().enumerate() {
                let i = (py * w + px) as usize;
                if terrain[i] != 1 || sp_dist[i] == 0 {
                    continue;
                }
                // Along-strike height envelope: taper to nothing at both ends so
                // a chain emerges from and sinks back into the lowlands, and
                // undulate in between so it has summits and passes.
                let t = k as f32 / len;
                let taper = (t * std::f32::consts::PI).sin().max(0.0).powf(0.45);
                let undulate = 0.72
                    + 0.28 * fbm_noise(k as f32 / 26.0, 0.0, seed.wrapping_add(0xC0DE), 3, 2.0, 0.5);
                sp_dist[i] = 0;
                sp_ref_coast[i] = sp.ref_coast[k];
                sp_amp[i] = taper * undulate;
                queue.push_back(i);
            }
        }
        while let Some(ci) = queue.pop_front() {
            let d = sp_dist[ci];
            if d >= search_radius {
                continue;
            }
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let ny = cy + dy;
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
                let ni = (ny as u32 * w + nx as u32) as usize;
                if terrain[ni] != 1 || sp_dist[ni] <= d + 1 {
                    continue;
                }
                sp_dist[ni] = d + 1;
                sp_ref_coast[ni] = sp_ref_coast[ci];
                sp_amp[ni] = sp_amp[ci];
                queue.push_back(ni);
            }
        }
    }

    // ── Compose the elevation field ──
    let f_hill = 1.0 / 46.0;
    let f_crest = 1.0 / (18.0 + spread * 26.0); // serration wavelength along the crest
    // Sub-range count: 1 at low density, up to 3 (Occidental/Central/Oriental).
    let sub_ranges = 1.0 + (density * 2.2).floor();

    let mut elevation = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 {
                continue;
            }
            let ax = x as f32;
            let ay = y as f32;

            // Continental base: a low, broad swell so lowlands are not dead flat.
            let base = fbm_noise(ax / 620.0 + 3.1, ay / 620.0 + 7.7, seed, 4, 2.0, 0.5) * 0.16;
            let hill = fbm_noise(ax * f_hill, ay * f_hill, seed.wrapping_add(0xFEED), 3, 2.0, 0.45)
                * (0.03 + roughness * 0.07);

            let mut e = base + hill;

            if sp_dist[idx] != u16::MAX && sp_amp[idx] > 0.0 {
                let d = sp_dist[idx] as f32;
                // Which flank? Seaward cells sit closer to the coast than the crest.
                let seaward_side = (coast_dist[idx] as i32) < (sp_ref_coast[idx] as i32);
                let reach = if seaward_side { seaward } else { piedmont };
                let core = range_half;

                // Cross-strike profile: a flat-topped crest zone, then a flank
                // that falls off. The seaward flank uses a higher exponent, so it
                // drops fast; the inland flank a lower one, so it lets down long.
                let profile = if d <= core {
                    1.0 - 0.18 * (d / core.max(1.0)).powi(2)
                } else {
                    let t = ((d - core) / (reach - core).max(1.0)).clamp(0.0, 1.0);
                    let falloff = if seaward_side { 1.9 } else { 3.2 };
                    (1.0 - t).powf(falloff) * 0.82
                };
                if profile > 0.0 {
                    // Parallel sub-ranges: a cosine across strike puts extra
                    // crests either side of the main divide, with intermontane
                    // basins in the troughs between them.
                    let phase = (d / core.max(1.0)) * std::f32::consts::PI * sub_ranges;
                    let sub = 1.0 + 0.22 * phase.cos() * (1.0 - (d / reach).clamp(0.0, 1.0));
                    // Crest serration along strike, so the divide is a saw of
                    // summits rather than a smooth wall.
                    let serr = ridged_multifractal(
                        ax * f_crest, ay * f_crest, seed.wrapping_add(0x51DE), 5, 2.1, 2.0,
                    );
                    let serration = 0.80 + 0.40 * serr * (0.5 + roughness * 0.5);
                    e += profile * sp_amp[idx] * sub * serration * 0.95;
                }
            }
            elevation[idx] = e.max(0.01);
        }
    }

    // ── Coastal taper that keeps a coastal range (see generate_elevation_from_terrain) ──
    const COAST_D: u16 = 3;
    for i in 0..n {
        if terrain[i] != 1 || coast_dist[i] >= COAST_D {
            continue;
        }
        let ratio = coast_dist[i] as f32 / COAST_D as f32;
        let taper = 0.5 + 0.5 * ratio;
        let ridge_keep = ((elevation[i] - 0.35) / 0.65).clamp(0.0, 1.0);
        elevation[i] *= taper.max(ridge_keep);
    }

    // ── Erosion + the shared hypsometric pipeline ──
    let pre_erosion = elevation.clone();
    let geo = geology::build_geo_context(buf, seed, &pre_erosion, &coast_dist, None);
    thermal_erosion(&mut elevation, &terrain, w, h, 2 + (roughness * 2.0) as u32);
    isostatic_adjust(&mut elevation, &terrain, &buf.boundary_type, &pre_erosion, w, h);

    normalize_and_redistribute(&mut elevation, &terrain, n, height, density, &geo.region_id, geo.region_count);
    limit_grid_scale_relief(&mut elevation, &terrain, w, h);
    apply_micro_relief(&mut elevation, &terrain, w, h, seed.wrapping_add(0x31C7));

    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
}

/// Cells to the nearest sea cell, for every land cell (full flood, 4-connected,
/// X wrapping / Y clamping). Sea cells are 0.
fn coast_distance(terrain: &[u8], w: u32, h: u32) -> Vec<u16> {
    let n = terrain.len();
    let mut dist = vec![0u16; n];
    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();
    for i in 0..n {
        if terrain[i] != 1 {
            visited[i] = true;
            queue.push_back(i);
        }
    }
    while let Some(ci) = queue.pop_front() {
        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;
        let d = dist[ci];
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 {
                continue;
            }
            let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
            let ni = (ny as u32 * w + nx as u32) as usize;
            if visited[ni] {
                continue;
            }
            visited[ni] = true;
            dist[ni] = d.saturating_add(1);
            queue.push_back(ni);
        }
    }
    dist
}

/// Walk mountain crests along iso-contours of distance-from-coast.
///
/// This is what makes the result a *cordillera* rather than a noise field: the
/// walker steps perpendicular to ∇(distance-from-coast), which is by construction
/// parallel to the coastline, so the chain shadows the margin the way a
/// subduction orogen does. A slow drift in the target offset and a noise term on
/// the heading keep it from being a mechanical offset curve.
fn trace_spines(
    terrain: &[u8], coast_dist: &[u16], w: u32, h: u32,
    seed: u64, crest_offset: f32, density: f32,
) -> Vec<Spine> {
    let n = terrain.len();
    let land_cells = terrain.iter().filter(|&&t| t == 1).count();
    if land_cells == 0 {
        return Vec::new();
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0xC03D_1E5A_u64);

    // How many chains to attempt: scale with land AREA so a big world gets more
    // cordilleras rather than longer ones.
    let target = ((land_cells as f32 / 90_000.0) * (0.7 + density * 1.6)).round();
    let attempts = (target.clamp(1.0, 14.0) as usize) * 6;
    // A cordillera is LONG: reject anything that peters out early, or the map
    // fills with stubs.
    let min_len = ((w.max(h) as f32) * 0.10).max(24.0) as usize;
    let max_len = (w.max(h) as f32 * 1.2) as usize;

    // Candidate starts: land cells sitting near the target offset from a coast.
    let lo = (crest_offset * 0.6) as u16;
    let hi = (crest_offset * 1.7) as u16 + 2;
    let mut candidates: Vec<usize> = (0..n)
        .filter(|&i| terrain[i] == 1 && coast_dist[i] >= lo && coast_dist[i] <= hi)
        .collect();
    if candidates.is_empty() {
        return Vec::new();
    }
    candidates.shuffle(&mut rng);

    // Keep chains apart so they read as separate cordilleras.
    let spacing = (crest_offset * 2.5).max(18.0);
    let mut claimed: Vec<(f32, f32)> = Vec::new();
    let mut spines = Vec::new();

    for &start in candidates.iter().take(attempts) {
        if spines.len() >= target.clamp(1.0, 14.0) as usize {
            break;
        }
        let sx = (start % w as usize) as f32;
        let sy = (start / w as usize) as f32;
        // Reject a start too near an existing chain (X distance wraps).
        if claimed.iter().any(|&(cx, cy)| {
            let mut dx = (sx - cx).abs();
            if dx > w as f32 / 2.0 {
                dx = w as f32 - dx;
            }
            (dx * dx + (sy - cy) * (sy - cy)).sqrt() < spacing
        }) {
            continue;
        }

        // Walk both ways from the seed so the chain grows in both directions and
        // the seed ends up mid-chain rather than at one end.
        let dir0 = rng.gen_range(0.0f32..std::f32::consts::TAU);
        let mut back = walk_spine(terrain, coast_dist, w, h, seed, sx, sy, dir0 + std::f32::consts::PI, max_len / 2);
        let fwd = walk_spine(terrain, coast_dist, w, h, seed ^ 0x5A5A, sx, sy, dir0, max_len / 2);
        back.reverse();
        back.extend(fwd);

        if back.len() < min_len {
            continue;
        }
        let points: Vec<(u32, u32)> = back.iter().map(|&(p, _)| p).collect();
        let ref_coast: Vec<u16> = back.iter().map(|&(_, c)| c).collect();
        for &(px, py) in points.iter().step_by(12) {
            claimed.push((px as f32, py as f32));
        }
        spines.push(Spine { points, ref_coast });
    }
    spines
}

/// Walk one direction along a coast-parallel contour, returning the cells
/// crossed with the distance-from-coast each was at.
fn walk_spine(
    terrain: &[u8], coast_dist: &[u16], w: u32, h: u32, seed: u64,
    mut x: f32, mut y: f32, mut heading: f32, max_steps: usize,
) -> Vec<((u32, u32), u16)> {
    let idx = |x: f32, y: f32| -> Option<usize> {
        if y < 0.0 || y >= h as f32 {
            return None;
        }
        let xi = ((x as i32 % w as i32) + w as i32) % w as i32;
        Some(y as usize * w as usize + xi as usize)
    };
    let Some(i0) = idx(x, y) else { return Vec::new() };
    // Hold the offset the seed started at, drifting slowly so the chain wanders
    // toward and away from the coast instead of tracking it rigidly.
    let mut target = coast_dist[i0] as f32;

    let mut out = Vec::new();
    let mut last: Option<usize> = None;
    for step in 0..max_steps {
        let Some(i) = idx(x, y) else { break };
        if terrain[i] != 1 {
            break;
        }
        if last != Some(i) {
            out.push((((i % w as usize) as u32, (i / w as usize) as u32), coast_dist[i] as u16));
            last = Some(i);
        }

        // ∇(distance-from-coast) by central differences; the contour direction is
        // perpendicular to it.
        let sample = |dx: i32, dy: i32| -> f32 {
            match idx(x + dx as f32, y + dy as f32) {
                Some(j) => coast_dist[j] as f32,
                None => coast_dist[i] as f32,
            }
        };
        let gx = sample(1, 0) - sample(-1, 0);
        let gy = sample(0, 1) - sample(0, -1);
        let glen = (gx * gx + gy * gy).sqrt();

        if glen > 1e-3 {
            // Two perpendiculars; take the one that keeps going the way we were.
            let (px, py) = (-gy / glen, gx / glen);
            let dot = px * heading.cos() + py * heading.sin();
            let (px, py) = if dot >= 0.0 { (px, py) } else { (-px, -py) };
            // Correction back toward the target offset, so the chain does not
            // slide down the gradient into the sea or off into the interior.
            let err = (target - coast_dist[i] as f32).clamp(-6.0, 6.0);
            let cx = gx / glen * (err * 0.16);
            let cy = gy / glen * (err * 0.16);
            heading = (py + cy).atan2(px + cx);
        }
        // Organic wander, and a slow drift of the offset itself.
        heading += (fbm_noise(step as f32 / 34.0, seed as f32 % 97.0, seed, 3, 2.0, 0.5) - 0.5) * 0.42;
        target += (fbm_noise(step as f32 / 70.0, 11.0, seed.wrapping_add(7), 2, 2.0, 0.5) - 0.5) * 1.1;
        target = target.clamp(2.0, 90.0);

        x += heading.cos() * 1.6;
        y += heading.sin() * 1.6;
    }
    out
}

/// Normalize to 0..1, apply the height exponent + p99.8 cap, then the shared
/// hypsometric redistribution. Extracted so the cordillera path and the ridged
/// path cannot drift apart in how they set final altitudes.
fn normalize_and_redistribute(
    elevation: &mut [f32], terrain: &[u8], n: usize, height: f32, density: f32,
    region_id: &[u32], region_count: u32,
) {
    let mut max_h = 0.0f32;
    for i in 0..n {
        if terrain[i] == 1 && elevation[i] > max_h {
            max_h = elevation[i];
        }
    }
    if max_h <= 0.0 {
        return;
    }
    for i in 0..n {
        if terrain[i] == 1 {
            elevation[i] /= max_h;
        }
    }
    let exponent = 2.0 - height;
    for i in 0..n {
        if terrain[i] == 1 {
            elevation[i] = elevation[i].powf(exponent);
        }
    }
    let target_cap = 0.35 + height * 0.60;
    let mut sorted: Vec<f32> = (0..n).filter(|&i| terrain[i] == 1).map(|i| elevation[i]).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if !sorted.is_empty() {
        let p998 = sorted[(sorted.len() as f32 * 0.998) as usize].max(0.01);
        let cap_scale = target_cap / p998;
        for i in 0..n {
            if terrain[i] == 1 {
                elevation[i] = (elevation[i] * cap_scale).clamp(0.01, 1.0);
            }
        }
    }
    let target = build_target_histogram(height, density);
    redistribute_elevation_regional(elevation, terrain, n, &target, region_id, region_count);
}

/// Scale every land cell's elevation by `scale` (0.5 = halve heights, 1.5 =
/// raise 50%). If `lock_above` < 1.0, any cell at or above that normalized
/// height keeps its value, so the highest peaks stay fixed while the rest of the
/// relief is raised or lowered. Land/sea is untouched, so sea depth/shelves do
/// not need recomputing.
pub fn scale_elevation(buf: &mut WorldBuffer, scale: f32, lock_above: f32) {
    let scale = scale.clamp(0.05, 4.0);
    for i in 0..buf.total() {
        if buf.terrain[i] != 1 { continue; }
        if lock_above < 1.0 && buf.elevation[i] >= lock_above { continue; }
        buf.elevation[i] = (buf.elevation[i] * scale).clamp(0.01, 1.0);
    }
}

/// A user-drawn ridge line: a polyline spine (world-cell coordinates) plus the
/// footprint half-width (cells), peak height (0..1, opacity-coded) and character
/// (0..1 ruggedness). `erase` inverts the effect (Shift-draw) to flatten a range.
#[derive(Clone, serde::Deserialize)]
pub struct RidgeLine {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
    pub height: f32,
    pub character: f32,
    #[serde(default)]
    pub erase: bool,
    /// 0 = clean oval edge; 1 = heavily eroded/irregular boundary.
    #[serde(default)]
    pub noise: f32,
}

/// Turn hand-drawn ridge lines into natural mountain ranges. The line spine is
/// widened into a rounded ridge whose footprint width comes from `width` and
/// whose peak height comes from `height`; the crest is broken into sub-peaks by
/// the shared `ridged_multifractal` field (scaled by `character`), then the new
/// range is carved by the SAME stream-power + thermal erosion the other
/// generators use. Reuses `thermal_erosion`/
/// `ridged_multifractal`/`warped_coords` directly.
///
/// Design (see plan): SCREEN-blends onto existing elevation (so it also works on
/// a flat world), LAND ONLY (ocean/coastline/depth/shelf untouched), and erosion
/// is confined to the new ridge footprints by passing a MASKED terrain array â€”
/// the erosion functions already skip cells where `terrain != 1`, so existing
/// terrain outside the mask is left exactly as it was.
pub fn generate_ridges(buf: &mut WorldBuffer, seed: u64, lines: &[RidgeLine]) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    if lines.is_empty() { return; }
    let terrain = buf.terrain.clone();

    // Per-cell ridge attributes, propagated outward from the spine by BFS.
    let mut dist = vec![u16::MAX; n]; // cells to nearest spine (MAX = unreached)
    let mut half_w = vec![0.0f32; n];
    let mut peak = vec![0.0f32; n];
    let mut charc = vec![0.0f32; n];
    let mut erase = vec![false; n];
    let mut noise_amt = vec![0.0f32; n];
    let mut src_x = vec![0i32; n]; // nearest spine cell position (for Euclidean dist)
    let mut src_y = vec![0i32; n];
    // Along-strike position, 0 at one end of the drawn polyline to 1 at the
    // other -- feeds the same "taper to nothing at both ends" treatment
    // generate_elevation_cordillera's spines already get (ITCZ_AND_LAND_
    // TOOLS_PLAN.md Commit 2: "chains that die into their surroundings"), so a
    // hand-drawn range ends in foothills instead of stopping dead at the
    // cursor's last position.
    let mut t_along = vec![0.5f32; n];
    let mut queue: VecDeque<usize> = VecDeque::new();

    // â”€â”€ 1. Rasterize every polyline into spine cells (BFS seeds) â”€â”€
    let mut max_half = 1.0f32;
    for line in lines {
        let half = line.width.max(1.0);
        max_half = max_half.max(half);
        let pk = line.height.clamp(0.0, 1.0);
        let ch = line.character.clamp(0.0, 1.0);
        let er = line.erase;
        let pts = &line.points;
        if pts.is_empty() { continue; }
        let nseg = if pts.len() == 1 { 1 } else { pts.len() - 1 };
        // Total polyline length, so each rasterized point's along-strike
        // fraction is measured against the WHOLE drawn line, not its own segment.
        let mut seg_lens = vec![0.0f32; nseg];
        let mut total_len = 0.0f32;
        for seg in 0..nseg {
            let (x0, y0) = pts[seg];
            let (x1, y1) = if pts.len() == 1 { pts[0] } else { pts[seg + 1] };
            let l = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
            seg_lens[seg] = l;
            total_len += l;
        }
        let mut len_so_far = 0.0f32;
        for seg in 0..nseg {
            let (x0, y0) = pts[seg];
            let (x1, y1) = if pts.len() == 1 { pts[0] } else { pts[seg + 1] };
            let (dx, dy) = (x1 - x0, y1 - y0);
            let len = seg_lens[seg];
            let steps = ((len * 2.0).ceil() as i32).max(1); // ~0.5-cell increments
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let cx = (x0 + dx * t).round() as i32;
                let cy = (y0 + dy * t).round() as i32;
                if cy < 0 || cy >= h as i32 { continue; }
                let nx = buf.wrap_x(cx);
                let i = buf.idx(nx, cy as u32);
                if terrain[i] != 1 { continue; } // land only
                let along = if total_len > 1e-3 { ((len_so_far + len * t) / total_len).clamp(0.0, 1.0) } else { 0.5 };
                if dist[i] == 0 {
                    // Overlapping spine: keep the taller peak / wider footprint.
                    if half > half_w[i] { half_w[i] = half; }
                    if pk > peak[i] { peak[i] = pk; }
                    continue;
                }
                dist[i] = 0;
                half_w[i] = half;
                t_along[i] = along;
                peak[i] = pk;
                charc[i] = ch;
                erase[i] = er;
                noise_amt[i] = line.noise.clamp(0.0, 1.0);
                src_x[i] = cx;
                src_y[i] = cy;
                queue.push_back(i);
            }
            len_so_far += len;
        }
    }
    if queue.is_empty() { return; }

    // â”€â”€ 2. Multi-source BFS: carry the source attributes to each nearest cell,
    // out to the widest footprint (cylinder-aware, X wraps / Y clamps). â”€â”€
    let reach = (max_half * 1.5).ceil() as u16;
    while let Some(ci) = queue.pop_front() {
        let d = dist[ci];
        if d >= reach { continue; }
        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let nx = buf.wrap_x(cx + dx);
            let ni = buf.idx(nx, ny as u32);
            if terrain[ni] != 1 { continue; }
            if dist[ni] > d + 1 {
                dist[ni] = d + 1;
                half_w[ni] = half_w[ci];
                t_along[ni] = t_along[ci];
                peak[ni] = peak[ci];
                charc[ni] = charc[ci];
                erase[ni] = erase[ci];
                noise_amt[ni] = noise_amt[ci];
                src_x[ni] = src_x[ci];
                src_y[ni] = src_y[ci];
                queue.push_back(ni);
            }
        }
    }

    // â”€â”€ 3. Cross-ridge uplift profile + ridged crest, screen-blended in â”€â”€
    let mut elevation = buf.elevation.clone();
    let mut mask_terrain = vec![0u8; n];
    for i in 0..n {
        if terrain[i] != 1 || dist[i] == u16::MAX { continue; }
        let x = (i % w as usize) as f32;
        let y = (i / w as usize) as f32;
        let hw = half_w[i].max(1.0);
        // Euclidean distance to the nearest spine cell â€” gives a circular
        // cross-section instead of the octagon from 8-connected BFS integers.
        let mut dx = x - src_x[i] as f32;
        let wf = w as f32;
        if dx > wf * 0.5 { dx -= wf; } else if dx < -wf * 0.5 { dx += wf; }
        let dy = y - src_y[i] as f32;
        let dist_f = (dx * dx + dy * dy).sqrt();
        // Noise displacement: shifts the effective distance per-cell so the
        // footprint edge meanders rather than staying a clean oval.
        let na = noise_amt[i];
        let noise_disp = if na > 0.01 {
            let f_n = 0.008 + charc[i] * 0.006;
            let n01 = fbm_noise(x * f_n + 17.3, y * f_n + 5.1, seed.wrapping_add(0xC0FFEE), 4, 2.0, 0.5);
            (n01 * 2.0 - 1.0) * na * hw * 0.55
        } else { 0.0 };
        let t = 1.0 - (dist_f + noise_disp).max(0.0) / hw;
        if t <= 0.0 { continue; }
        mask_terrain[i] = 1;
        let profile = t * t * (3.0 - 2.0 * t);
        if erase[i] {
            // Flatten toward the low end so drawing over a range removes it.
            elevation[i] = (elevation[i] * (1.0 - profile * 0.9)).clamp(0.01, 1.0);
            continue;
        }
        // Ridged crest: break the spine into sub-peaks/passes. Frequency and
        // amplitude grow with character (smooth rounded â†” serrated/rugged).
        let f = 0.05 + charc[i] * 0.10;
        let (rx, ry) = warped_coords(x * f, y * f, seed.wrapping_add(0x81DE), 1.2 + charc[i]);
        let ridge = ridged_multifractal(rx, ry, seed.wrapping_add(0x48271), 6, 2.1, 2.0);
        let noise_factor = (1.0 - charc[i] * 0.55) + charc[i] * 1.1 * ridge;
        // Along-strike taper: full height mid-line, tapering to nothing at both
        // drawn ends (the SAME sin^0.45 envelope generate_elevation_cordillera's
        // spines use), so a hand-drawn range ends in foothills instead of
        // stopping dead at the cursor's last position -- a noise-modulated
        // falloff, not a symmetric one, so the two ends taper at slightly
        // different rates.
        let along_noise = 0.75 + 0.25 * fbm_noise(t_along[i] * 9.0 + 3.0, charc[i] * 5.0, seed.wrapping_add(0x7A9E), 3, 2.0, 0.5);
        let along_taper = (t_along[i] * std::f32::consts::PI).sin().max(0.0).powf(0.45) * along_noise;
        let target = (peak[i] * profile * noise_factor * along_taper.min(1.0)).clamp(0.0, 1.0);
        // Screen blend: full peak on flat ground, saturates â‰¤1 over existing peaks.
        elevation[i] = (elevation[i] + target * (1.0 - elevation[i])).clamp(0.01, 1.0);
    }

    // â”€â”€ 4. Localized erosion â€” a MASKED terrain confines droplets + thermal
    // slump to the new footprints; the shared erosion functions do the carving. â”€â”€
    let mask_count = mask_terrain.iter().filter(|&&m| m == 1).count();
    if mask_count > 0 {
        let mut char_sum = 0.0f32;
        for i in 0..n { if mask_terrain[i] == 1 { char_sum += charc[i]; } }
        let avg_char = char_sum / mask_count as f32;
        // Droplet density comparable to the whole-map generators (which use a
        // small fraction of a droplet per land cell) â€” enough to carve valleys
        // into the new range without eroding the crest away.
        let iters = ((mask_count as f32 * 0.6) as u32).clamp(1_000, 60_000);
        let passes = 2 + (avg_char * 3.0) as u32;
        thermal_erosion(&mut elevation, &mask_terrain, w, h, passes);
    }

    // â”€â”€ 5. Write back â€” land only; sea, coastline, depth & shelf untouched â”€â”€
    for i in 0..n {
        if terrain[i] == 1 { buf.elevation[i] = elevation[i].clamp(0.01, 1.0); }
    }
}

// ── The grid-scale relief budget ────────────────────────────────────────────
//
// A cell is a `km_per_cell`-wide AVERAGE of the landscape inside it, so relief
// at the ONE-CELL scale is, by construction, relief the grid cannot resolve.
// Real topography sampled at 11 km (the default 3600-wide world) is smooth at
// that scale: adjacent samples differ because the land is going somewhere, not
// because each sample has its own private bump.
//
// Phase 2's field did have private bumps, and the hypsometric redistribution
// then amplified them along with everything else -- measured on a plate world,
// `redistribute_elevation` multiplies landform relief by 7.7x and grid-scale
// relief by 8.9x, so whatever one-cell content the noise stack leaves is what
// the finished map ends up textured with. At 1800x900 that came out at 80 m RMS.
//
// 80 m is a lot, and the number that says so belongs to the RENDERER: shaded
// relief saturates its ambient-occlusion term at `AO_REF` = 240 m of concavity
// against the 8-neighbour mean (`render/tile_image.rs`, itself measured -- see
// section 8.21's account of AO at 44 m resolving as film grain). A world whose
// grid-scale RMS is a third of that is drawing AO texture on every cell of
// every plain, which is exactly the "too thin lines / far too eroded for an
// Earth-sized world" this budget answers.
//
// So: cap the one-cell component at a stated fraction of the renderer's own
// saturation scale, SELF-CALIBRATING (measure this world's grid-scale RMS and
// scale the detail band to fit) rather than by a hand-tuned amplitude -- the
// same discipline `need_scale` and `prov_good_yield_scale` already use in the
// campaign half, and for the same reason: the right constant depends on the
// world, and a fixed one is wrong on every world but the one it was tuned on.
//
// Three properties this deliberately keeps:
//   * It only ever SMOOTHS. A world already inside budget is returned
//     bit-identical, so this can never invent relief.
//   * It touches only the ONE-CELL band (the residual against a radius-1 box
//     blur). Landform relief -- the ranges, the massifs, the basins, which is
//     everything a reader actually looks at -- is untouched: measured, the
//     landform figure moves by well under 1%.
//   * It runs BEFORE `apply_micro_relief`, so the deliberate +/-14 m dither
//     that keeps plateaus from being perfectly flat survives it. That dither
//     sits an order of magnitude below `AO_REF` on purpose and is not what
//     this is aimed at.

/// Grid-scale relief RMS allowed, in metres.
///
/// CHOSEN BY LOOKING, over a sweep rendered through the real hillshade
/// (`dump_erosion_sheet` at 48 / 24 / 16 / 12 m on one world): 48 and 24 both
/// still comb every steep flank with visible ribbing, 16 reads as a massif
/// with smooth flanks, and 12 is not detectably better than 16. It sits at
/// about a fifteenth of the renderer's `AO_REF`, so one-cell concavity now
/// draws at a few percent of AO saturation instead of a third of it.
///
/// The HONEST CAVEAT, because this number is lower than a purely physical
/// argument would put it: real continental topography is roughly self-affine,
/// and scaling a ~400 m RMS relief at 100 km down to a 22 km cell with a Hurst
/// exponent near 0.5 suggests a legitimate one-cell residual nearer 60 m. Ours
/// cannot be spent that way, because our one-cell content is UNCORRELATED
/// between neighbours (independent noise at the top of an fbm/ridged stack)
/// while Earth's is STRUCTURED -- part of a cascade whose ridges and valleys
/// continue across cells. Identical RMS, completely different reading: noise
/// looks like grain, structure looks like terrain. Buying back that headroom
/// properly means generating structured sub-grid detail (a real multifractal
/// cascade, or erosion run at a finer scale and averaged down), which is a
/// terrain-generation change, not a shading one. Until then the budget is set
/// where the texture reads as terrain rather than where the variance argument
/// alone would allow.
const GRID_RELIEF_BUDGET_M: f32 = 16.0;

/// Cap the ONE-CELL component of the finished elevation field at
/// `GRID_RELIEF_BUDGET_M`, leaving everything at landform scale alone. Returns
/// the (before, after) grid-scale RMS in metres so callers and tests can report
/// it. A no-op (and bit-identical) on a world already inside the budget.
fn limit_grid_scale_relief(elevation: &mut [f32], terrain: &[u8], w: u32, h: u32) -> (f32, f32) {
    const MAX_ELEV: f32 = 8848.0;
    /// Correction passes. Scaling the detail band is NOT idempotent: writing
    /// `e' = m + k(e - m)` leaves `residual' = k(e - m) + (1 - k)(m - blur m)`,
    /// so the band between one and two cells leaks back in and one pass lands
    /// well short of the target -- measured on a real plate world, a single
    /// pass aimed at 16 m settled at 41 m. Iterating to the fixed point is the
    /// honest fix; it converges in two or three passes and this cap is only a
    /// termination guarantee.
    const MAX_PASSES: usize = 6;

    // A LAND-ONLY local mean: blur the elevation masked to land and divide by
    // the blurred mask. A plain blur would average a 3000 m coastal cell
    // against sea cells held at 0 and call the coastline itself "detail" --
    // which both swamps the measurement (the land-sea step is the largest
    // gradient on the map) and, worse, would then plane the coast DOWN toward
    // sea level, drawing exactly the hard dark rim this is meant to remove.
    let land: Vec<f32> = terrain.iter().map(|&t| if t == 1 { 1.0 } else { 0.0 }).collect();
    let den = box_blur_wrap(&land, w, h, 1);

    let local_mean = |e: &[f32]| -> Vec<f32> {
        let masked: Vec<f32> = (0..e.len())
            .map(|i| if terrain[i] == 1 { e[i] } else { 0.0 })
            .collect();
        let num = box_blur_wrap(&masked, w, h, 1);
        (0..e.len())
            .map(|i| if den[i] > 1e-6 { num[i] / den[i] } else { e[i] })
            .collect()
    };
    let residual_rms = |e: &[f32], m: &[f32]| -> (f32, u64) {
        let mut sq = 0.0f64;
        let mut cnt = 0u64;
        for i in 0..e.len() {
            if terrain[i] != 1 { continue; }
            let d = ((e[i] - m[i]) * MAX_ELEV) as f64;
            sq += d * d;
            cnt += 1;
        }
        if cnt == 0 { (0.0, 0) } else { ((sq / cnt as f64).sqrt() as f32, cnt) }
    };

    let mut smooth = local_mean(elevation);
    let (first, cnt) = residual_rms(elevation, &smooth);
    if cnt == 0 { return (0.0, 0.0); }
    let budget = GRID_RELIEF_BUDGET_M;
    let mut rms = first;
    if rms <= budget || rms <= 0.0 {
        return (first, first);
    }

    for _ in 0..MAX_PASSES {
        let keep = budget / rms;
        for i in 0..elevation.len() {
            if terrain[i] != 1 { continue; }
            elevation[i] = (smooth[i] + (elevation[i] - smooth[i]) * keep).clamp(0.01, 1.0);
        }
        smooth = local_mean(elevation);
        rms = residual_rms(elevation, &smooth).0;
        if rms <= budget { break; }
    }
    (first, rms)
}

/// Terrain-aware MICRO-RELIEF dither â€” guarantees there are no perfectly flat,
/// mono-height plateaus while keeping genuine flats (floodplains, high tablelands)
/// readable as flat. Two bands:
///   â€¢ FLOOR  (~Â±2 m): a fine, per-cell dither applied EVERYWHERE, so no two
///     adjacent land cells ever hold the exact same height â€” the "very minor
///     fluctuation" the map should always have.
///   â€¢ RELIEF (~Â±14 m): a rolling, few-cell undulation gated by LOCAL SLOPE, so
///     hillsides and mountain flanks get rolling texture while low-slope surfaces
///     (floodplains AND high plateaus) stay smooth â€” a real high desert/steppe
///     reads as a tableland, and lowland floodplains stay flat enough for rivers
///     to meander across them (see rivers.rs meander pass).
/// Runs on the finished (redistributed) surface, in normalized-elevation units,
/// with amplitudes far below the ~18 m lake-fill threshold so it never spawns
/// spurious lakes, yet far above the 9 mm drainage Îµ so drainage is unaffected.
fn apply_micro_relief(elevation: &mut [f32], terrain: &[u8], w: u32, h: u32, seed: u64) {
    const MAX_ELEV: f32 = 8848.0;
    let floor_amp = 2.0 / MAX_ELEV;    // ~Â±2 m everywhere
    let relief_amp = 14.0 / MAX_ELEV;  // ~Â±14 m on true slopes
    // Local slope (normalized units per cell) at which RELIEF saturates: ~53 m/cell.
    let slope_ref = 53.0 / MAX_ELEV;
    let s_fine = seed.wrapping_add(0x00D1_7737);
    let s_roll = seed.wrapping_add(0x00A5_1CE0);
    let wi = w as i32;
    let hi = h as i32;
    // Snapshot so the slope read is from the pre-dither surface.
    let base = elevation.to_vec();
    for y in 0..hi {
        for x in 0..wi {
            let i = (y * wi + x) as usize;
            if terrain[i] != 1 { continue; }
            // Local slope = max abs height difference to the 4-neighbours (X wraps,
            // Y clamps), the same cylindrical topology the rest of the sim uses.
            let mut slope = 0.0f32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = ((x + dx) % wi + wi) % wi;
                let ny = (y + dy).clamp(0, hi - 1);
                let ni = (ny * wi + nx) as usize;
                if terrain[ni] != 1 { continue; }
                slope = slope.max((base[i] - base[ni]).abs());
            }
            let mut rough = (slope / slope_ref).min(1.0);
            rough = rough * rough * (3.0 - 2.0 * rough); // smoothstep 0..1
            let ax = x as f32;
            let ay = y as f32;
            // Fine per-cell dither (âˆ’1..1) and a rolling few-cell undulation (âˆ’1..1).
            let fine = (fbm_noise(ax * 0.9 + 0.3, ay * 0.9 + 0.7, s_fine, 2, 2.0, 0.5) - 0.5) * 2.0;
            let roll = (fbm_noise(ax * 0.16 + 0.9, ay * 0.16 + 0.2, s_roll, 3, 2.0, 0.5) - 0.5) * 2.0;
            let delta = floor_amp * fine + relief_amp * rough * roll;
            elevation[i] = (base[i] + delta).clamp(0.01, 1.0);
        }
    }
}

/// The TARGET HYPSOMETRIC CURVE: what share of land sits in each 1000 m band.
///
/// These anchors were measurably wrong, and the error dominated every map's
/// appearance. At the default `height` = 0.5 the old pair produced **~21% of
/// land above 4000 m and only ~38% below 1000 m**, so nearly every world came
/// out a pale high plateau with the hypsometric tint saturated at its top end --
/// which hid all the relief underneath it, landform variety included.
///
/// Real Earth land, for comparison (ETOPO/Amante & Eakins, rounded):
///
/// | band | 0-1 km | 1-2 | 2-3 | 3-4 | 4-5 | 5-6 | 6-7 | 7-8 | 8+ |
/// |---|---|---|---|---|---|---|---|---|---|
/// | % of land | **71** | 18 | 6 | 3 | 1.3 | 0.5 | 0.15 | 0.04 | 0.01 |
///
/// The anchors below are set so the MIDPOINT (`height` = 0.5, the plate model's
/// own default and the middle of the slider) lands on that row. `LOW` is a
/// genuinely flat, coastal world; `HIGH` is a dramatic alpine one -- and even
/// `HIGH` keeps 56% of its land under 1000 m, because a world where a quarter of
/// the land is above 4 km is not "alpine", it is not a planet.
///
/// Note what this does NOT change: the Earth climate gate scores against the
/// baked GMT DEM and never calls this (section 2.3), so the 70.2 / 39.0 figures
/// are untouched by construction. What it does change is every GENERATED world's
/// temperature (lapse rate), biomes, habitability and settlement placement --
/// all of which were being computed on land that averaged nearly 3x too high.
fn build_target_histogram(height: f32, density: f32) -> [f32; 9] {
    // Anchors (each sums to ~100). LOW = flat/coastal, HIGH = alpine.
    const LOW:  [f32; 9] = [86.0, 10.0, 2.60, 0.90, 0.30, 0.15, 0.040, 0.008, 0.002];
    const HIGH: [f32; 9] = [56.0, 26.0, 9.50, 5.00, 2.40, 0.90, 0.250, 0.070, 0.020];
    let t = height.clamp(0.0, 1.0);
    let mut out = [0.0f32; 9];
    for b in 0..9 {
        out[b] = LOW[b] * (1.0 - t) + HIGH[b] * t;
    }
    // Density nudges mass out of the lowest band and up the curve. The weights
    // TAPER rather than spreading the mass evenly: an even split dumped as much
    // into the 5-6 km band as into the 1-2 km band, which took the alpine end of
    // the slider to 9.2% of land above 4 km -- more than fifteen times Earth's
    // share, i.e. not a planet. Real orogeny raises a lot of hill country and
    // very little summit.
    const SHIFT_W: [f32; 4] = [0.45, 0.30, 0.18, 0.07]; // into bands 1..4
    let shift = density.clamp(0.0, 1.0) * out[0] * 0.12;
    out[0] -= shift;
    for (k, wgt) in SHIFT_W.iter().enumerate() { out[1 + k] += shift * wgt; }
    out
}

/// TECTONICS_RIVERS_PROVINCES_PLAN.md Slice 1 (rule 31 -- "a clamp is not a
/// landform"): the lowest-ranked ~9% of band 0 used to be pinned to the
/// identical `0.01` clamp floor (88.5 m of the 8848 m range), which measured
/// as 16% of ALL land sitting at one exact elevation in one contiguous
/// component -- and because a clamp plateau has zero gradient, every river
/// crossing it saturated the meander model's `slow` term identically,
/// which is F2's root cause as much as F1's own. `MIN_LAND_ELEV` (~5 m, a
/// real floodplain height) replaces the clamp: land is bounded BELOW at this
/// floor instead of AT it, so the lowest-ranked cells still rise
/// monotonically from a plausible delta height rather than piling up on one
/// value. It must stay strictly above 0.0 -- 0.0 means "sea" to
/// `plates::invert_terrain` and to this function's own land filter.
const MIN_LAND_ELEV: f32 = 0.0006;

/// Redistribute land elevations to match a target per-1000 m-band histogram,
/// preserving relative ordering (peaks stay peaks). Port of WF1
/// `redistributeElevation`.
fn redistribute_elevation(elevation: &mut [f32], terrain: &[u8], n: usize, target_pcts: &[f32; 9]) {
    const MAX_ELEV: f32 = 8848.0;

    let mut land_indices: Vec<usize> = (0..n)
        .filter(|&i| terrain[i] == 1 && elevation[i] > 0.0)
        .collect();
    if land_indices.is_empty() { return; }
    land_indices.sort_by(|&a, &b| elevation[a].partial_cmp(&elevation[b]).unwrap());

    let total_land = land_indices.len();
    let total_pct: f32 = target_pcts.iter().sum();
    if total_pct <= 0.0 { return; }

    let mut cell_idx = 0usize;
    for band in 0..9 {
        let band_count = ((target_pcts[band] / total_pct) * total_land as f32).round() as usize;
        // Band 0 starts at MIN_LAND_ELEV rather than 0.0, so the lowest-ranked
        // cell in the lowest band lands at a real floodplain height instead of
        // the old clamp floor -- see MIN_LAND_ELEV's own doc comment.
        let band_min = if band == 0 { MIN_LAND_ELEV } else { (band as f32 * 1000.0) / MAX_ELEV };
        let band_max = ((band as f32 + 1.0) * 1000.0) / MAX_ELEV;
        let mut j = 0usize;
        while j < band_count && cell_idx < total_land {
            let idx = land_indices[cell_idx];
            let t = if band_count > 1 { j as f32 / (band_count - 1) as f32 } else { 0.5 };
            elevation[idx] = (band_min + t * (band_max - band_min)).clamp(MIN_LAND_ELEV, 1.0);
            j += 1;
            cell_idx += 1;
        }
    }
    // Any leftover cells (rounding) -> top band.
    let last_min = (8.0 * 1000.0) / MAX_ELEV;
    while cell_idx < total_land {
        elevation[land_indices[cell_idx]] = (last_min + 0.01).min(1.0);
        cell_idx += 1;
    }
}

/// `redistribute_elevation`, then a REGIONALISED correction (TERRAIN_2_PLAN.md
/// section 4 slice 2, D9). The plain global version squeezes every region's
/// land into one rank histogram, which sets a plausible overall hypsometric
/// curve but ACTIVELY ERASES between-region contrast: a region the erosion
/// pass just made systematically lower (soft lithology, an old worn craton)
/// reads identically to a resistant young belt once both are re-ranked into
/// the same global bands.
///
/// So: run the existing global pass first (it still owns the overall SHAPE),
/// then measure each region's own pre-redistribution character -- its mean
/// elevation relative to the whole map's land mean, captured before that
/// character gets rank-squeezed away -- and reapply a bounded fraction of it
/// as a per-region offset. A region with too few land cells to trust a mean
/// from is left alone.
fn redistribute_elevation_regional(
    elevation: &mut [f32], terrain: &[u8], n: usize, target_pcts: &[f32; 9],
    region_id: &[u32], region_count: u32,
) {
    if region_count == 0 || region_id.len() != n {
        redistribute_elevation(elevation, terrain, n, target_pcts);
        return;
    }
    let rc = region_count as usize;
    let mut region_sum = vec![0.0f64; rc];
    let mut region_cnt = vec![0u32; rc];
    let mut land_sum = 0.0f64;
    let mut land_cnt = 0u32;
    for i in 0..n {
        if terrain[i] != 1 || elevation[i] <= 0.0 { continue; }
        let r = region_id[i] as usize;
        if r < rc {
            region_sum[r] += elevation[i] as f64;
            region_cnt[r] += 1;
        }
        land_sum += elevation[i] as f64;
        land_cnt += 1;
    }
    if land_cnt == 0 { return; }
    let land_mean = land_sum / land_cnt as f64;
    let region_bias: Vec<f32> = (0..rc)
        .map(|r| if region_cnt[r] >= 8 { (region_sum[r] / region_cnt[r] as f64 - land_mean) as f32 } else { 0.0 })
        .collect();

    redistribute_elevation(elevation, terrain, n, target_pcts);

    const REGION_CONTRAST: f32 = 0.35;
    // Slice 1: a bounded SCALE about MIN_LAND_ELEV, not an additive offset
    // followed by a clamp. The old `(elev + bias*CONTRAST).clamp(0.01, 1.0)`
    // pushed an entire negative-bias region's low end under the clamp floor at
    // once -- a whole physiographic region collapsing to one elevation, which
    // is why the flat component was tens of thousands of cells rather than
    // scattered floodplain. Scaling keeps every cell strictly ordered and
    // strictly above the floor: a region still reads higher or lower by the
    // same intent, but it can never flatten.
    let land_mean = land_mean.max(1e-6);
    for i in 0..n {
        if terrain[i] != 1 { continue; }
        let r = region_id[i] as usize;
        if r >= rc { continue; }
        let bias = region_bias[r];
        if bias == 0.0 { continue; }
        let factor = (1.0 + bias as f64 * REGION_CONTRAST as f64 / land_mean)
            .clamp(0.25, 4.0) as f32;
        elevation[i] = (MIN_LAND_ELEV + (elevation[i] - MIN_LAND_ELEV) * factor)
            .clamp(MIN_LAND_ELEV, 1.0);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ITCZ_AND_LAND_TOOLS_PLAN.md Commit 2 — four new elevation models, each
// registered in `apply_elevation_model` (`commands/sim_commands.rs`) so both
// run-alls honour them. All four share the base model's final pipeline
// (coastal taper → thermal + isostatic erosion → the shared hypsometric
// redistribution → grid-scale relief limit → micro relief) and differ only in
// how the pre-erosion elevation field is built — the part that gives each its
// name.
// ═══════════════════════════════════════════════════════════════════════

/// The dominant strike direction of the world's divergent plate boundaries, as
/// the principal axis (PCA) of their cell positions — real data where plate
/// data exists. `None` when there is no plate data or too few divergent cells
/// to fix a direction (a template world, or one with no rifting at all); the
/// caller then falls back to a seeded regional strike, exactly the convention
/// `geology.rs`'s phase-2 climate proxy already uses for "no better data".
fn divergent_strike_angle(buf: &WorldBuffer) -> Option<f32> {
    if buf.boundary_type.is_empty() {
        return None;
    }
    let mut pts: Vec<(f32, f32)> = Vec::new();
    for y in 0..buf.height {
        for x in 0..buf.width {
            let i = buf.idx(x, y);
            if buf.boundary_type[i] == crate::sim::plates::BOUNDARY_DIVERGENT {
                pts.push((x as f32, y as f32));
            }
        }
    }
    if pts.len() < 8 {
        return None;
    }
    let n = pts.len() as f32;
    let mx = pts.iter().map(|p| p.0).sum::<f32>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f32>() / n;
    let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
    for &(x, y) in &pts {
        let (dx, dy) = (x - mx, y - my);
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    sxx /= n;
    syy /= n;
    sxy /= n;
    Some(0.5 * (2.0 * sxy).atan2(sxx - syy))
}

/// Shared tail: coastal taper (keeps a coastal range from flattening into a
/// plain), erosion, the hypsometric redistribution, and the final grid-scale
/// relief + micro-relief passes. Every one of the four new models below funnels
/// through this — it is the same tail `generate_elevation_cordillera` uses.
fn finish_elevation_field(
    buf: &mut WorldBuffer,
    seed: u64,
    height: f32,
    density: f32,
    terrain: &[u8],
    coast_dist: &[u16],
    mut elevation: Vec<f32>,
    thermal_passes: u32,
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    const COAST_D: u16 = 3;
    for i in 0..n {
        if terrain[i] != 1 || coast_dist[i] >= COAST_D {
            continue;
        }
        let ratio = coast_dist[i] as f32 / COAST_D as f32;
        let taper = 0.5 + 0.5 * ratio;
        let ridge_keep = ((elevation[i] - 0.35) / 0.65).clamp(0.0, 1.0);
        elevation[i] *= taper.max(ridge_keep);
    }
    let pre_erosion = elevation.clone();
    let geo = geology::build_geo_context(buf, seed, &pre_erosion, coast_dist, None);
    thermal_erosion(&mut elevation, terrain, w, h, thermal_passes);
    isostatic_adjust(&mut elevation, terrain, &buf.boundary_type, &pre_erosion, w, h);
    normalize_and_redistribute(&mut elevation, terrain, n, height, density, &geo.region_id, geo.region_count);
    limit_grid_scale_relief(&mut elevation, terrain, w, h);
    apply_micro_relief(&mut elevation, terrain, w, h, seed.wrapping_add(0x31C7));
    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
}

/// Rift/horst-graben elevation: parallel fault blocks — a tilted, asymmetric
/// HORST (steep scarp on one side, a gentle back-slope down to the next
/// graben) alternating with a flat-floored GRABEN. Strike follows the world's
/// own divergent-boundary trend where plate data exists, a seeded regional
/// strike otherwise.
pub fn generate_elevation_rift(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let density = mountain_density.clamp(0.0, 1.0);
    let height = mountain_height.clamp(0.0, 1.0);
    let spread = mountain_spread.clamp(0.0, 1.0);
    let roughness = noise_roughness.clamp(0.0, 1.0);
    let terrain = buf.terrain.clone();
    let coast_dist = coast_distance(&terrain, w, h);

    let theta = divergent_strike_angle(buf)
        .unwrap_or_else(|| hash_grid(0, 0, seed.wrapping_add(0x5717)) * std::f32::consts::PI);
    let (cos_a, sin_a) = (theta.cos(), theta.sin());
    let (cos_p, sin_p) = ((theta + std::f32::consts::FRAC_PI_2).cos(), (theta + std::f32::consts::FRAC_PI_2).sin());
    // A period covers one horst+graben pair; wider at high `spread` (broad rift
    // valleys) and narrower at low `spread` (tight fault-block terrain).
    let period = 34.0 + spread * 150.0;
    let f_serr = 1.0 / (16.0 + spread * 20.0);

    let mut elevation = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 {
                continue;
            }
            let (ax, ay) = (x as f32, y as f32);
            let base = fbm_noise(ax / 640.0 + 2.0, ay / 640.0 + 9.0, seed, 4, 2.0, 0.5) * 0.14;

            let u = ax * cos_p + ay * sin_p;
            let v = ax * cos_a + ay * sin_a;
            // Domain-warp the cross-strike coordinate so fault traces aren't
            // perfectly straight lines.
            let warp = (fbm_noise(u / 90.0 + 4.0, v / 240.0 + 1.0, seed.wrapping_add(0x9A17), 3, 2.0, 0.5) - 0.5)
                * period * 0.35;
            let uw = u + warp;
            let cell = (uw / period).floor();
            let t = uw / period - cell; // 0..1 within this horst+graben pair

            let block = if t < 0.5 {
                let tb = t / 0.5;
                let peak = 0.14;
                if tb < peak {
                    (tb / peak).powf(0.6)
                } else {
                    let tt = (tb - peak) / (1.0 - peak);
                    1.0 - tt * 0.68 // gentle dip-slope back down to the next graben
                }
            } else {
                // Flat graben floor, only textured — the down-dropped basin.
                0.06
            };
            let serr = ridged_multifractal(ax * f_serr, ay * f_serr, seed.wrapping_add(0x4171), 4, 2.0, 2.0);
            let texture = 0.04 + roughness * 0.10;

            elevation[idx] = (base + block * (0.55 + density * 0.45) + serr * texture).max(0.01);
        }
    }

    finish_elevation_field(buf, seed, height, density, &terrain, &coast_dist, elevation, 1 + (roughness * 2.0) as u32);
}

/// Latitude+altitude "ice mask" proxy (phase 2 has no climate — documented as a
/// proxy, same convention `geology.rs`'s phase-2 climate stand-in already uses).
fn glacial_ice_mask(buf: &WorldBuffer, terrain: &[u8], elevation: &[f32]) -> Vec<f32> {
    let n = terrain.len();
    let mut ice = vec![0.0f32; n];
    for y in 0..buf.height {
        for x in 0..buf.width {
            let i = buf.idx(x, y);
            if terrain[i] != 1 {
                continue;
            }
            let lat = crate::sim::world_buffer::lat_from_y(
                y as f32, buf.height as f32, buf.equator_offset, buf.lat_scale, buf.lat_ratio,
            ).abs();
            let lat_score = ((lat - 45.0) / 30.0).clamp(0.0, 1.0);
            let alt_score = ((elevation[i] - 0.45) / 0.35).clamp(0.0, 1.0);
            ice[i] = (lat_score + alt_score * 0.7).clamp(0.0, 1.0);
        }
    }
    ice
}

/// Glaciated / fjordland elevation: the shape model, then glacial modification
/// gated by the ice mask — U-valley broadening (extra rounding blended in by ice
/// presence), cirque hollows carved just below ice-zone summits, and
/// over-deepened troughs walked downhill from the strongest summits that BREACH
/// the coast (turn their final stretch to sea). This is the honest way to get
/// fjords, as opposed to notching a coastline with noise (§8.23).
pub fn generate_elevation_glaciated(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    generate_elevation_from_terrain(buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let roughness = noise_roughness.clamp(0.0, 1.0);
    let mut terrain = buf.terrain.clone();
    let mut elevation = buf.elevation.clone();

    let ice = glacial_ice_mask(buf, &terrain, &elevation);

    // U-valley broadening: extra thermal-erosion rounding, blended in by ice
    // presence so only the glaciated zone is affected.
    let mut eroded = elevation.clone();
    thermal_erosion(&mut eroded, &terrain, w, h, 5);
    for i in 0..n {
        if terrain[i] == 1 {
            elevation[i] = elevation[i] * (1.0 - ice[i]) + eroded[i] * ice[i];
        }
    }

    // Cirque hollows: a small bowl carved just below (not at) a local summit
    // that sits well inside the ice zone.
    let seeded = seed.wrapping_add(0xC12C);
    let mut cirque_seeds: Vec<usize> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if terrain[i] != 1 || ice[i] < 0.55 {
                continue;
            }
            let mut is_max = true;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny_i = y as i32 + dy;
                    if ny_i < 0 || ny_i >= h as i32 {
                        continue;
                    }
                    let ni = buf.idx(nx, ny_i as u32);
                    if terrain[ni] == 1 && elevation[ni] > elevation[i] {
                        is_max = false;
                    }
                }
            }
            if !is_max || hash_grid(x as i32, y as i32, seeded) > 0.35 {
                continue;
            }
            cirque_seeds.push(i);
            let ang = hash_grid(x as i32, y as i32, seeded.wrapping_add(1)) * std::f32::consts::TAU;
            let (ox, oy) = (ang.cos() * 2.0, ang.sin() * 2.0);
            let (cx, cy) = (x as f32 + ox, y as f32 + oy);
            let r = 2.0 + roughness * 2.0;
            let ri = r.ceil() as i32;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    if d > r {
                        continue;
                    }
                    let nx = buf.wrap_x(cx as i32 + dx);
                    let ny_i = cy as i32 + dy;
                    if ny_i < 0 || ny_i >= h as i32 {
                        continue;
                    }
                    let ni = buf.idx(nx, ny_i as u32);
                    if terrain[ni] != 1 {
                        continue;
                    }
                    let bowl = (1.0 - d / r) * 0.10 * ice[ni];
                    elevation[ni] = (elevation[ni] - bowl).max(0.01);
                }
            }
        }
    }

    // Over-deepened troughs: walk steepest-descent from the strongest ice-zone
    // summits toward the coast, carving a valley and breaching the final stretch
    // into the sea (a real fjord mouth), bounded to a handful of trails so this
    // stays a local, selection-sized cost rather than a full-grid search.
    cirque_seeds.sort_by(|&a, &b| (ice[b] * elevation[b]).partial_cmp(&(ice[a] * elevation[a])).unwrap());
    let trough_count = cirque_seeds.len().min(6);
    for &start in cirque_seeds.iter().take(trough_count) {
        let mut x = (start % w as usize) as f32;
        let mut y = (start / w as usize) as f32;
        let mut path = Vec::new();
        for _ in 0..(w.max(h) as usize) {
            let ix = x.round() as i32;
            let iy = y.round() as i32;
            if iy < 0 || iy >= h as i32 {
                break;
            }
            let wx = buf.wrap_x(ix);
            let i = buf.idx(wx, iy as u32);
            if terrain[i] != 1 {
                break; // reached the coast
            }
            path.push(i);
            // Steepest descent among the 8 neighbours.
            let mut best_e = elevation[i];
            let mut best_dir = None;
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let nx = buf.wrap_x(ix + dx);
                let ny_i = iy + dy;
                if ny_i < 0 || ny_i >= h as i32 {
                    continue;
                }
                let ni = buf.idx(nx, ny_i as u32);
                if elevation[ni] < best_e {
                    best_e = elevation[ni];
                    best_dir = Some((dx, dy));
                }
            }
            let Some((dx, dy)) = best_dir else { break };
            x = (ix + dx) as f32;
            y = (iy + dy) as f32;
        }
        let plen = path.len();
        if plen < 6 {
            continue; // never reached the coast — no honest fjord to draw
        }
        // Carve the trough, then breach the final ~20% into the sea.
        let breach_from = (plen as f32 * 0.8) as usize;
        for (k, &ci) in path.iter().enumerate() {
            let depth = ice[ci] * 0.18 * (k as f32 / plen as f32).min(1.0);
            elevation[ci] = (elevation[ci] - depth).max(0.01);
            if k >= breach_from {
                terrain[ci] = 0;
                elevation[ci] = 0.0;
            }
        }
    }

    buf.terrain = terrain.clone();
    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
}

/// Plateau & mesa elevation: quantised levels with SHARP escarpment rims (never
/// blurred, unlike the subtle `terrace` blend `landform.rs` already applies) plus
/// outlying buttes scattered near the plateau's own margins.
pub fn generate_elevation_plateau(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    generate_elevation_from_terrain(buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let density = mountain_density.clamp(0.0, 1.0);
    let terrain = buf.terrain.clone();
    let mut elevation = buf.elevation.clone();

    // Quantise into sharp levels — a step function IS the escarpment; no blur.
    let levels = (4.0 + density * 4.0).round().max(3.0);
    for i in 0..n {
        if terrain[i] != 1 {
            continue;
        }
        elevation[i] = ((elevation[i] * levels).round() / levels).clamp(0.02, 1.0);
    }

    // Outlying buttes: small isolated hills standing one level above their
    // surroundings, scattered across the plateau.
    let bseed = seed.wrapping_add(0xBE77E);
    let butte_count = (8.0 + density * 24.0) as usize;
    for k in 0..butte_count {
        let idx = ((hash_grid(k as i32, 17, bseed) * n as f32) as usize).min(n - 1);
        if terrain[idx] != 1 {
            continue;
        }
        let x = (idx % w as usize) as i32;
        let y = (idx / w as usize) as i32;
        let base_level = elevation[idx];
        let r = 1.5 + hash_grid(k as i32, 3, bseed) * 2.5;
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d > r {
                    continue;
                }
                let nx = buf.wrap_x(x + dx);
                let ny_i = y + dy;
                if ny_i < 0 || ny_i >= h as i32 {
                    continue;
                }
                let ni = buf.idx(nx, ny_i as u32);
                if terrain[ni] != 1 {
                    continue;
                }
                let bump = (1.0 - d / r) * (1.0 / levels) * 1.3;
                elevation[ni] = elevation[ni].max((base_level + bump).min(1.0));
            }
        }
    }

    apply_micro_relief(&mut elevation, &terrain, w, h, seed.wrapping_add(0x31C7));
    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
}

/// Volcanic hotspot elevation: a gentle low backdrop, shield cones stamped
/// (max-blended, so overlapping cones merge into ranges) on every `is_volcanic`
/// land cell, summit calderas on the densest clusters, and hotspot trails —
/// decreasing-height cones extending from an isolated seed across whatever
/// existing land lies in one seeded direction (an elevation generator never
/// creates new land, per rule 6/§8.23's discipline — only phase 1 or the
/// lasso tools do that).
pub fn generate_elevation_volcanic(
    buf: &mut WorldBuffer,
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let density = mountain_density.clamp(0.0, 1.0);
    let height = mountain_height.clamp(0.0, 1.0);
    let spread = mountain_spread.clamp(0.0, 1.0);
    let terrain = buf.terrain.clone();
    let coast_dist = coast_distance(&terrain, w, h);

    let scale = w.max(h) as f32 / 10.0;
    let mut elevation = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 {
                continue;
            }
            let base = fbm_noise(x as f32 / scale, y as f32 / scale, seed, 5, 2.0, 0.5);
            elevation[idx] = (base * 0.22).clamp(0.0, 0.5);
        }
    }

    let volc: Vec<(i32, i32, usize)> = (0..n)
        .filter(|&i| terrain[i] == 1 && buf.is_volcanic[i] == 1)
        .map(|i| ((i % w as usize) as i32, (i / w as usize) as i32, i))
        .collect();

    let base_r = 3.0 + spread * 7.0;
    // Bucket the volcanic points so local-density lookups are O(volc), never
    // O(volc²) (§8.9 rule 1's spirit — a world can carry thousands of these).
    let bucket_size = (base_r * 3.0).max(4.0);
    let mut buckets: std::collections::HashMap<(i32, i32), Vec<usize>> = std::collections::HashMap::new();
    for (vi, &(vx, vy, _)) in volc.iter().enumerate() {
        let bx = (vx as f32 / bucket_size).floor() as i32;
        let by = (vy as f32 / bucket_size).floor() as i32;
        buckets.entry((bx, by)).or_default().push(vi);
    }
    let neighbours_within = |vx: i32, vy: i32, r: f32| -> u32 {
        let bx = (vx as f32 / bucket_size).floor() as i32;
        let by = (vy as f32 / bucket_size).floor() as i32;
        let mut count = 0u32;
        for obx in bx - 1..=bx + 1 {
            for oby in by - 1..=by + 1 {
                let Some(list) = buckets.get(&(obx, oby)) else { continue };
                for &oi in list {
                    let (ox, oy, _) = volc[oi];
                    if ox == vx && oy == vy {
                        continue;
                    }
                    let mut dx = (vx - ox).abs();
                    if dx > w as i32 / 2 {
                        dx = w as i32 - dx;
                    }
                    let dy = vy - oy;
                    if (dx * dx + dy * dy) as f32 <= r * r {
                        count += 1;
                    }
                }
            }
        }
        count
    };

    for &(vx, vy, _) in &volc {
        let neighbours = neighbours_within(vx, vy, base_r * 3.0);
        let r = base_r * (1.0 + (neighbours.min(6) as f32) * 0.15);
        let hh = (0.35 + height * 0.55) * (0.5 + 0.5 * (neighbours.min(8) as f32) / 8.0);
        let ri = r.ceil() as i32;
        let is_caldera = neighbours >= 4;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d > r {
                    continue;
                }
                let nx = buf.wrap_x(vx + dx);
                let ny_i = vy + dy;
                if ny_i < 0 || ny_i >= h as i32 {
                    continue;
                }
                let ni = buf.idx(nx, ny_i as u32);
                if terrain[ni] != 1 {
                    continue;
                }
                let mut cone = (1.0 - d / r).powf(1.6) * hh;
                if is_caldera && d < r * 0.22 {
                    cone -= (1.0 - d / (r * 0.22)) * hh * 0.35;
                }
                elevation[ni] = elevation[ni].max(cone.max(0.0));
            }
        }

        // Hotspot trail: isolated seeds (no other volcano nearby) extend a chain
        // of shrinking cones in one seeded direction, across whatever land is
        // actually there.
        if neighbours == 0 {
            let ang = hash_grid(vx, vy, seed.wrapping_add(0x07A1)) * std::f32::consts::TAU;
            let (dirx, diry) = (ang.cos(), ang.sin());
            let mut tx = vx as f32;
            let mut ty = vy as f32;
            let mut tr = r;
            let mut thh = hh;
            for _ in 0..5 {
                tx += dirx * (base_r * 2.2);
                ty += diry * (base_r * 2.2);
                let ix = tx.round() as i32;
                let iy = ty.round() as i32;
                if iy < 0 || iy >= h as i32 {
                    break;
                }
                let wxi = buf.wrap_x(ix);
                let i = buf.idx(wxi, iy as u32);
                if terrain[i] != 1 {
                    break; // trail runs off the existing landmass — stop, don't invent land
                }
                tr *= 0.75;
                thh *= 0.65;
                let tri = tr.ceil() as i32;
                for dy in -tri..=tri {
                    for dx in -tri..=tri {
                        let d = ((dx * dx + dy * dy) as f32).sqrt();
                        if d > tr {
                            continue;
                        }
                        let nx = buf.wrap_x(ix + dx);
                        let ny_i = iy + dy;
                        if ny_i < 0 || ny_i >= h as i32 {
                            continue;
                        }
                        let ni = buf.idx(nx, ny_i as u32);
                        if terrain[ni] != 1 {
                            continue;
                        }
                        let cone = (1.0 - d / tr).powf(1.6) * thh;
                        elevation[ni] = elevation[ni].max(cone.max(0.0));
                    }
                }
            }
        }
    }

    finish_elevation_field(buf, seed, height, density, &terrain, &coast_dist, elevation, 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::sim::world_buffer::ColumnSet;

    /// A rectangular continent inside a sea frame, for the coast-relative tests.
    pub fn continent(w: u32, h: u32, margin: u32) -> WorldBuffer {
        let n = (w * h) as usize;
        let mut terrain = vec![1u8; n];
        for y in 0..h {
            for x in 0..w {
                if x < margin || x >= w - margin || y < margin || y >= h - margin {
                    terrain[(y * w + x) as usize] = 0;
                }
            }
        }
        WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain, elevation: vec![0.0; n],
            sea_depth: vec![0.0; n], is_shelf: vec![0u8; n], is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(), plate_index: Vec::new(), boundary_type: Vec::new(),
            is_volcanic: Vec::new(), temperature: Vec::new(), precipitation: Vec::new(),
            koppen: Vec::new(), soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(),
            seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        }
    }

    /// The defining property of a cordillera: its high ground forms a small number
    /// of LONG CONNECTED chains, not a scatter of independent blobs. Measured as
    /// the share of high cells that belong to the single largest connected
    /// component — a chain concentrates them, isotropic ridged noise does not.
    #[test]
    fn cordillera_high_ground_forms_connected_chains() {
        let (w, h) = (220u32, 150u32);
        let mut buf = continent(w, h, 6);
        generate_elevation_cordillera(&mut buf, 4242, 0.5, 0.7, 0.5, 0.4);

        let n = (w * h) as usize;
        // "High" = the top decile of land, so the test does not depend on the
        // absolute altitudes the hypsometric redistribution happens to pick.
        let mut land: Vec<f32> = (0..n).filter(|&i| buf.terrain[i] == 1).map(|i| buf.elevation[i]).collect();
        assert!(!land.is_empty());
        land.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let thresh = land[(land.len() as f32 * 0.90) as usize];

        let high: Vec<bool> = (0..n).map(|i| buf.terrain[i] == 1 && buf.elevation[i] >= thresh).collect();
        let total_high = high.iter().filter(|&&v| v).count();
        assert!(total_high > 50, "expected a meaningful high-ground population");

        // Largest 8-connected component of high ground.
        let mut seen = vec![false; n];
        let mut largest = 0usize;
        for start in 0..n {
            if !high[start] || seen[start] { continue; }
            let mut size = 0usize;
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(ci) = stack.pop() {
                size += 1;
                let cx = (ci % w as usize) as i32;
                let cy = (ci / w as usize) as i32;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let ny = cy + dy;
                        if ny < 0 || ny >= h as i32 { continue; }
                        let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
                        let ni = (ny as u32 * w + nx as u32) as usize;
                        if high[ni] && !seen[ni] { seen[ni] = true; stack.push(ni); }
                    }
                }
            }
            largest = largest.max(size);
        }
        let share = largest as f32 / total_high as f32;
        assert!(share > 0.30,
            "cordillera high ground must form connected chains, not scattered blobs: \
             largest component holds only {:.0}% of high cells", share * 100.0);
    }

    /// The property that actually separates a cordillera from ridged noise: its
    /// crest RUNS PARALLEL TO THE MARGIN, so the high ground clusters at one
    /// distance-from-coast. Isotropic ridged noise puts peaks wherever the noise
    /// peaked, scattered across every distance from the shore. Measured as the
    /// spread (standard deviation) of coast-distance over the top-decile cells,
    /// on the SAME landmass with the SAME sliders and seed.
    #[test]
    fn cordillera_crest_runs_parallel_to_the_coast() {
        let (w, h) = (220u32, 150u32);
        let mut cord = continent(w, h, 6);
        let mut noisy = continent(w, h, 6);
        generate_elevation_cordillera(&mut cord, 4242, 0.5, 0.7, 0.5, 0.4);
        generate_elevation_ridged(&mut noisy, 4242, 0.5, 0.7, 0.5, 0.4);

        let coast = coast_distance(&cord.terrain, w, h);
        let spread = |buf: &WorldBuffer| -> f64 {
            let n = (w * h) as usize;
            let mut land: Vec<f32> = (0..n).filter(|&i| buf.terrain[i] == 1).map(|i| buf.elevation[i]).collect();
            land.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let thresh = land[(land.len() as f32 * 0.90) as usize];
            let ds: Vec<f64> = (0..n)
                .filter(|&i| buf.terrain[i] == 1 && buf.elevation[i] >= thresh)
                .map(|i| coast[i] as f64)
                .collect();
            let mean = ds.iter().sum::<f64>() / ds.len() as f64;
            (ds.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / ds.len() as f64).sqrt()
        };
        let cord_spread = spread(&cord);
        let noise_spread = spread(&noisy);
        assert!(cord_spread < noise_spread * 0.80,
            "a cordillera's crest must hug one distance from the coast: \
             spread {cord_spread:.1} cells vs ridged noise {noise_spread:.1}");
    }

    /// The flanks must be ASYMMETRIC — the whole point of a subduction-margin
    /// range. Averaged over the chain, the inland side lets down through a much
    /// broader apron than the seaward side, so mean elevation stays higher
    /// further from the crest on the inland side.
    #[test]
    fn cordillera_flanks_are_asymmetric() {
        let (w, h) = (220u32, 150u32);
        let mut buf = continent(w, h, 6);
        generate_elevation_cordillera(&mut buf, 99, 0.5, 0.7, 0.6, 0.3);

        let coast = coast_distance(&buf.terrain, w, h);
        // Find the crest band: the coast-distance at which mean elevation peaks.
        let max_d = *coast.iter().max().unwrap() as usize;
        let mut sum = vec![0.0f64; max_d + 1];
        let mut cnt = vec![0u32; max_d + 1];
        for i in 0..(w * h) as usize {
            if buf.terrain[i] != 1 { continue; }
            let d = coast[i] as usize;
            sum[d] += buf.elevation[i] as f64;
            cnt[d] += 1;
        }
        let mean: Vec<f64> = (0..=max_d)
            .map(|d| if cnt[d] > 0 { sum[d] / cnt[d] as f64 } else { 0.0 })
            .collect();
        let crest = (0..=max_d).max_by(|&a, &b| mean[a].partial_cmp(&mean[b]).unwrap()).unwrap();
        assert!(crest > 2, "the crest must sit inland of the shoreline, got {crest}");

        // Compare how fast the profile decays either side of the crest, over the
        // same number of cells.
        let reach = (crest.min(max_d - crest)).min(20);
        assert!(reach >= 4, "need room either side of the crest to compare");
        let seaward: f64 = (1..=reach).map(|k| mean[crest - k]).sum::<f64>() / reach as f64;
        let inland: f64 = (1..=reach).map(|k| mean[crest + k]).sum::<f64>() / reach as f64;
        assert!(inland > seaward * 1.10,
            "inland piedmont must let down more gradually than the seaward scarp \
             (inland mean {inland:.4} vs seaward {seaward:.4})");
    }

    /// Same seed and sliders must give exactly the same mountains — the spine
    /// tracer uses an RNG and a shuffle, so this is the guard that it stays
    /// deterministic.
    #[test]
    fn cordillera_is_deterministic() {
        let (w, h) = (120u32, 90u32);
        let mut a = continent(w, h, 5);
        let mut b = continent(w, h, 5);
        generate_elevation_cordillera(&mut a, 7, 0.6, 0.6, 0.4, 0.5);
        generate_elevation_cordillera(&mut b, 7, 0.6, 0.6, 0.4, 0.5);
        assert_eq!(a.elevation, b.elevation);
    }

    /// A world with no land at all must not panic or hang.
    #[test]
    fn cordillera_handles_an_all_sea_world() {
        let (w, h) = (40u32, 30u32);
        let mut buf = continent(w, h, 5);
        for t in buf.terrain.iter_mut() { *t = 0; }
        generate_elevation_cordillera(&mut buf, 1, 0.5, 0.5, 0.5, 0.5);
        assert!(buf.elevation.iter().all(|&e| e == 0.0));
    }

    /// Micro-relief must (1) leave no perfectly flat mono-plateau â€” a uniform
    /// input comes out with adjacent cells differing â€” while (2) staying tiny
    /// (well under the ~18 m lake-fill threshold, and on a FLAT surface under the
    /// ~2 m floor since no slope means no rolling relief), and (3) be deterministic.
    #[test]
    fn micro_relief_dithers_flats_but_stays_bounded() {
        let (w, h) = (40u32, 24u32);
        let n = (w * h) as usize;
        let terrain = vec![1u8; n];
        let flat = 0.3f32;

        let mut a = vec![flat; n];
        apply_micro_relief(&mut a, &terrain, w, h, 777);

        // (2) bounded: a flat surface has zero slope â†’ only the ~2 m floor applies.
        let floor = 2.0 / 8848.0;
        let lake_thresh = 0.002; // ~18 m
        let mut max_dev = 0.0f32;
        for &v in &a {
            let d = (v - flat).abs();
            if d > max_dev { max_dev = d; }
            assert!(d < lake_thresh, "micro-relief must never approach the lake threshold: {d}");
        }
        assert!(max_dev <= floor * 1.05, "flat ground gets only the floor band: {max_dev} vs {floor}");
        assert!(max_dev > 0.0, "flat ground must actually be dithered");

        // (1) no mono: overwhelmingly, horizontally-adjacent cells now differ.
        let mut differ = 0usize;
        for y in 0..h { for x in 0..w - 1 {
            let i = (y * w + x) as usize;
            if a[i] != a[i + 1] { differ += 1; }
        }}
        let pairs = (h * (w - 1)) as usize;
        assert!(differ as f32 / pairs as f32 > 0.99, "flats must not stay mono-height: {differ}/{pairs}");

        // (3) deterministic: same seed â†’ identical result.
        let mut b = vec![flat; n];
        apply_micro_relief(&mut b, &terrain, w, h, 777);
        assert_eq!(a, b, "micro-relief must be reproducible for a given seed");
    }

    /// A LARGE continent interior must not read as a flat "green blob": the
    /// template elevation model must give deep-interior cells genuine relief
    /// (ranges/hills), not just the Â±2 m micro-relief floor. Guards the
    /// absolute-wavelength interior-relief injection in generate_elevation_from_terrain.
    #[test]
    fn template_interior_is_not_a_flat_blob() {
        use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
        let (w, h) = (180u32, 140u32);
        let n = (w * h) as usize;
        // A big landmass with a 4-cell sea frame (so coast-distance is well defined).
        let mut terrain = vec![1u8; n];
        for y in 0..h {
            for x in 0..w {
                if x < 4 || x >= w - 4 || y < 4 || y >= h - 4 {
                    terrain[(y * w + x) as usize] = 0;
                }
            }
        }
        let mut buf = WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: terrain.clone(), elevation: vec![0.0; n],
            sea_depth: vec![0.0; n], is_shelf: vec![0u8; n], is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(), plate_index: Vec::new(), boundary_type: Vec::new(),
            is_volcanic: Vec::new(), temperature: Vec::new(), precipitation: Vec::new(),
            koppen: Vec::new(), soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(), seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        };
        generate_elevation_from_terrain(&mut buf, 12345, 0.5, 0.5, 0.5, 0.5);

        // Deep interior = well away from the coast (central band). A genuine range-
        // and-hill interior has many cells with real local slope; a flat dome has
        // almost none beyond the ~2 m dither floor.
        let floor = 2.0 / 8848.0;
        let mut interior = 0usize;
        let mut relieved = 0usize;
        for y in 30..h - 30 {
            for x in 30..w - 30 {
                let i = (y * w + x) as usize;
                interior += 1;
                let mut slope = 0.0f32;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let ni = ((y as i32 + dy) as u32 * w + (x as i32 + dx) as u32) as usize;
                    slope = slope.max((buf.elevation[i] - buf.elevation[ni]).abs());
                }
                if slope > floor * 6.0 { relieved += 1; } // > ~12 m/cell = real relief
            }
        }
        let frac = relieved as f32 / interior as f32;
        assert!(frac > 0.15,
            "continent interior reads as a flat blob: only {:.1}% of interior cells have relief",
            frac * 100.0);
    }

    /// Sloped ground should get MORE relief than flat ground (rolling hills), so
    /// the dither is genuinely terrain-aware, not uniform noise.
    #[test]
    fn micro_relief_is_stronger_on_slopes() {
        let (w, h) = (48u32, 8u32);
        let n = (w * h) as usize;
        let terrain = vec![1u8; n];
        // A steep ramp in x (big local slope) vs the flat case above.
        let mut ramp: Vec<f32> = (0..n).map(|i| 0.05 + (i as u32 % w) as f32 * 0.02).collect();
        let base = ramp.clone();
        apply_micro_relief(&mut ramp, &terrain, w, h, 4242);
        // Deviation from the smooth ramp, away from the clamped edges.
        let mut max_dev = 0.0f32;
        for y in 0..h { for x in 4..w - 4 {
            let i = (y * w + x) as usize;
            let d = (ramp[i] - base[i]).abs();
            if d > max_dev { max_dev = d; }
        }}
        let floor = 2.0 / 8848.0;
        assert!(max_dev > floor * 2.0, "slopes get rolling relief beyond the floor: {max_dev}");
        assert!(max_dev < 0.002, "â€¦but still under the lake threshold: {max_dev}");
    }

    /// A single drawn ridge line on a FLAT all-land world must raise a band of
    /// cells along the spine, leave cells far from the line near zero, keep the
    /// ocean untouched, and produce only finite values in [0,1]. Guards
    /// generate_ridges (footprint width, land-only, localized effect).
    #[test]
    fn ridge_line_raises_a_band_on_flat_world() {
        use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
        let (w, h) = (120u32, 80u32);
        let n = (w * h) as usize;
        // All land except a one-cell frame of sea, plus a sea column to prove the
        // ridge never crosses water.
        let mut terrain = vec![1u8; n];
        for y in 0..h {
            for x in 0..w {
                if x == 0 || x == w - 1 || y == 0 || y == h - 1 {
                    terrain[(y * w + x) as usize] = 0;
                }
            }
        }
        let mut buf = WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: terrain.clone(),
            elevation: (0..n).map(|i| if terrain[i] == 1 { 0.01 } else { 0.0 }).collect(),
            sea_depth: vec![0.0; n], is_shelf: vec![0u8; n], is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(), plate_index: Vec::new(), boundary_type: Vec::new(),
            is_volcanic: Vec::new(), temperature: Vec::new(), precipitation: Vec::new(),
            koppen: Vec::new(), soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(), seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        };
        // A horizontal ridge across the middle: half-width 6 cells, tall, moderate character.
        let line = RidgeLine {
            points: vec![(20.0, 40.0), (100.0, 40.0)],
            width: 6.0, height: 0.9, character: 0.5, erase: false, noise: 0.0,
        };
        generate_ridges(&mut buf, 999, &[line]);

        // Finite + bounded everywhere; ocean stays flat at 0.
        for i in 0..n {
            let e = buf.elevation[i];
            assert!(e.is_finite() && (0.0..=1.0).contains(&e), "elevation out of range: {e}");
            if terrain[i] == 0 { assert_eq!(e, 0.0, "ocean must stay untouched"); }
        }
        // The spine band is high; a row far from the line stays near the 0.01 floor.
        let on_spine = buf.elevation[(40 * w + 60) as usize];
        let far = buf.elevation[(12 * w + 60) as usize];
        assert!(on_spine > 0.35, "ridge spine should be raised, got {on_spine}");
        assert!(far < 0.1, "cells far from the line stay low, got {far}");
        assert!(on_spine > far + 0.25, "spine must stand well above the surroundings");
    }

    /// Isostatic rebound must lift a heavily-eroded upland back up (partially,
    /// never past its pre-erosion height), only ever ADD on land, and be
    /// deterministic. Boundary data is empty here so this isolates the erosional
    /// rebound from the mountain-root term.
    #[test]
    fn isostatic_rebound_lifts_eroded_uplands() {
        let (w, h) = (64u32, 48u32);
        let n = (w * h) as usize;
        let terrain = vec![1u8; n]; // all land, to isolate isostasy

        // A central raised plateau over a low plain.
        let mut elevation = vec![0.05f32; n];
        for y in 12..36u32 {
            for x in 16..48u32 {
                elevation[(y * w + x) as usize] = 0.9;
            }
        }
        let pre = elevation.clone();

        // Erode it hard.
        thermal_erosion(&mut elevation, &terrain, w, h, 4);
        let eroded = elevation.clone();

        // Rebound only (empty boundary_type disables the root term).
        let mut adjusted = eroded.clone();
        isostatic_adjust(&mut adjusted, &terrain, &[], &pre, w, h);

        let plateau: Vec<usize> = (0..n)
            .filter(|&i| {
                let x = i as u32 % w;
                let y = i as u32 / w;
                (16..48).contains(&x) && (12..36).contains(&y)
            })
            .collect();
        let mean = |v: &[f32], idxs: &[usize]| {
            idxs.iter().map(|&i| v[i]).sum::<f32>() / idxs.len() as f32
        };
        let m_eroded = mean(&eroded, &plateau);
        let m_adj = mean(&adjusted, &plateau);
        let m_pre = mean(&pre, &plateau);

        // 1. Rebound raises the eroded upland.
        assert!(m_adj > m_eroded, "rebound should raise eroded uplands: {m_adj} !> {m_eroded}");
        // 2. But only a fraction is re-added — never above the pre-erosion height.
        assert!(m_adj <= m_pre + 1e-3, "rebound must not exceed pre-erosion: {m_adj} > {m_pre}");
        // 3. Finite everywhere and only ever ADDS on land.
        for i in 0..n {
            assert!(adjusted[i].is_finite(), "non-finite elevation at {i}");
            assert!(adjusted[i] >= eroded[i] - 1e-6, "isostasy must not lower land at {i}");
        }
        // 4. Deterministic.
        let mut again = eroded.clone();
        isostatic_adjust(&mut again, &terrain, &[], &pre, w, h);
        assert_eq!(again, adjusted, "isostatic_adjust must be deterministic");
    }

    /// PHASE-2 COST, the counterpart to `step3_ocean_atmo/bench.rs`. Phase 3 has a
    /// millisecond breakdown and phase 2 has never had one, so any claim about what
    /// terrain generation can afford has been guesswork.
    ///   cargo test --release --lib bench_phase2 -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_phase2() {
        use std::time::Instant;
        for (w, h) in [(1800u32, 900u32), (3600, 1800)] {
            let mut buf = continent(w, h, w / 12);
            // `continent` leaves the plate columns empty; the plate generator writes them.
            let n = buf.total();
            buf.plate_index = vec![0u16; n];
            buf.boundary_type = vec![0u8; n];
            buf.is_volcanic = vec![0u8; n];
            crate::sim::plates::generate_plates_and_landmass(&mut buf, 7, 14);
            let land = (0..buf.total()).filter(|&i| buf.terrain[i] == 1).count();

            let t = Instant::now();
            generate_elevation(&mut buf, 7);
            let plates_ms = t.elapsed().as_millis();

            let t = Instant::now();
            generate_elevation_from_terrain(&mut buf, 7, 0.5, 0.5, 0.5, 0.4);
            let shape_ms = t.elapsed().as_millis();

            let t = Instant::now();
            compute_sea_depth(&mut buf);
            let depth_ms = t.elapsed().as_millis();

            let iters = ((buf.total() as f32 * 0.012) as u32).clamp(15_000, 90_000);
            let land_frac = land as f32 / buf.total() as f32;
            println!(
                "{w}x{h}  cells={:>9}  land={:>5.1}%  | plates {plates_ms:>5} ms                   shape {shape_ms:>5} ms  sea_depth {depth_ms:>4} ms",
                buf.total(), 100.0 * land_frac);
            println!(
                "         erosion budget {iters} droplet-equivalents (now stream-power outer passes, not droplets), ~{:.0}% of the map is OCEAN",
                100.0 * (1.0 - land_frac));
        }
    }

    /// TERRAIN 2.0 INSTRUMENTATION (`TERRAIN_2_PLAN.md` section 3, "to build").
    /// One table per elevation model of the metrics the plan's own gates are
    /// stated in: RMS slope, slope SPREAD across windows (the headline gate —
    /// a world where every range shades alike scores near zero on this),
    /// drainage density (share of land cells carrying real accumulated flow,
    /// read straight off the same priority-flood/accumulation the erosion
    /// pass itself uses), the hypsometric integral, coast-on-plate-boundary
    /// fraction (should fall well under 100% after slice 4's decoupling), and
    /// the sea_depth<->distance-to-coast correlation (should fall after slice
    /// 5's seafloor structure). Printing only — no assertions here, exactly
    /// like `bench_phase2`; this is the read-it-yourself instrument the plan
    /// asks for, not a pass/fail gate.
    ///   cargo test --release --lib terrain_metrics -- --ignored --nocapture
    #[test]
    #[ignore]
    fn terrain_metrics() {
        fn rms_slope(elevation: &[f32], terrain: &[u8], w: u32, h: u32, xr: std::ops::Range<u32>, yr: std::ops::Range<u32>) -> f32 {
            let mut sum = 0.0f64;
            let mut cnt = 0u64;
            for y in yr {
                for x in xr.clone() {
                    let i = (y * w + x) as usize;
                    if terrain[i] != 1 { continue; }
                    let xr1 = ((x + 1) % w) as usize + (y * w) as usize;
                    let yr1 = (((y + 1).min(h - 1)) * w + x) as usize;
                    let dx = elevation[i] - elevation[xr1];
                    let dy = elevation[i] - elevation[yr1];
                    sum += (dx * dx + dy * dy) as f64;
                    cnt += 1;
                }
            }
            if cnt == 0 { 0.0 } else { (sum / cnt as f64).sqrt() as f32 }
        }

        fn slope_spread(elevation: &[f32], terrain: &[u8], w: u32, h: u32) -> f32 {
            const WIN: u32 = 8;
            let mut vals = Vec::new();
            for wy in 0..WIN {
                for wx in 0..WIN {
                    let x0 = wx * w / WIN;
                    let x1 = ((wx + 1) * w / WIN).max(x0 + 1);
                    let y0 = wy * h / WIN;
                    let y1 = ((wy + 1) * h / WIN).max(y0 + 1);
                    let s = rms_slope(elevation, terrain, w, h, x0..x1, y0..y1);
                    if s > 0.0 { vals.push(s as f64); }
                }
            }
            if vals.len() < 2 { return 0.0; }
            let mean: f64 = vals.iter().sum::<f64>() / vals.len() as f64;
            let var: f64 = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
            (var.sqrt() / mean.max(1e-9)) as f32 // coefficient of variation
        }

        fn drainage_density(buf: &WorldBuffer) -> f32 {
            let (flow_to, order) = priority_flood_flow(&buf.elevation, &buf.terrain, buf.width, buf.height);
            let n = buf.total();
            let mut area = vec![1.0f32; n];
            for &i in order.iter().rev() {
                let t = flow_to[i];
                if t != i { area[t] += area[i]; }
            }
            let land = order.len().max(1);
            let channels = order.iter().filter(|&&i| area[i] >= 40.0).count();
            channels as f32 / land as f32
        }

        fn hypsometric_integral(elevation: &[f32], terrain: &[u8]) -> f32 {
            let vals: Vec<f32> = (0..terrain.len()).filter(|&i| terrain[i] == 1).map(|i| elevation[i]).collect();
            if vals.is_empty() { return 0.0; }
            let min = vals.iter().cloned().fold(f32::MAX, f32::min);
            let max = vals.iter().cloned().fold(f32::MIN, f32::max);
            let mean = vals.iter().sum::<f32>() / vals.len() as f32;
            if max <= min { 0.0 } else { (mean - min) / (max - min) }
        }

        fn coast_on_boundary_fraction(buf: &WorldBuffer) -> f32 {
            if buf.boundary_type.is_empty() { return f32::NAN; }
            let w = buf.width;
            let h = buf.height;
            let mut coastal = 0u32;
            let mut on_boundary = 0u32;
            for y in 0..h {
                for x in 0..w {
                    let i = buf.idx(x, y);
                    if buf.terrain[i] != 1 { continue; }
                    let mut is_coast = false;
                    for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let nx = buf.wrap_x(x as i32 + dx);
                        let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        if buf.terrain[buf.idx(nx, ny)] != 1 { is_coast = true; break; }
                    }
                    if !is_coast { continue; }
                    coastal += 1;
                    if buf.boundary_type[i] != 0 { on_boundary += 1; continue; }
                    for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let nx = buf.wrap_x(x as i32 + dx);
                        let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        if buf.boundary_type[buf.idx(nx, ny)] != 0 { on_boundary += 1; break; }
                    }
                }
            }
            if coastal == 0 { 0.0 } else { on_boundary as f32 / coastal as f32 }
        }

        fn depth_distance_correlation(buf: &WorldBuffer) -> f32 {
            let n = buf.total();
            let mut dist = vec![0.0f64; n];
            {
                let w = buf.width;
                let h = buf.height;
                let mut d = vec![u32::MAX; n];
                let mut q = VecDeque::new();
                for i in 0..n { if buf.terrain[i] == 1 { d[i] = 0; q.push_back(i); } }
                while let Some(ci) = q.pop_front() {
                    let dd = d[ci];
                    if dd >= 200 { continue; }
                    let cx = (ci % w as usize) as i32;
                    let cy = (ci / w as usize) as i32;
                    for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let nx = buf.wrap_x(cx + dx);
                        let ny = (cy + dy).clamp(0, h as i32 - 1) as u32;
                        let ni = buf.idx(nx, ny);
                        if d[ni] > dd + 1 { d[ni] = dd + 1; q.push_back(ni); }
                    }
                }
                for i in 0..n { dist[i] = d[i].min(200) as f64; }
            }
            let pts: Vec<(f64, f64)> = (0..n)
                .filter(|&i| buf.terrain[i] == 0)
                .map(|i| (dist[i], buf.sea_depth[i] as f64))
                .collect();
            if pts.len() < 2 { return f32::NAN; }
            let mx = pts.iter().map(|p| p.0).sum::<f64>() / pts.len() as f64;
            let my = pts.iter().map(|p| p.1).sum::<f64>() / pts.len() as f64;
            let mut cov = 0.0f64;
            let mut vx = 0.0f64;
            let mut vy = 0.0f64;
            for &(x, y) in &pts {
                cov += (x - mx) * (y - my);
                vx += (x - mx).powi(2);
                vy += (y - my).powi(2);
            }
            if vx <= 0.0 || vy <= 0.0 { return f32::NAN; }
            (cov / (vx.sqrt() * vy.sqrt())) as f32
        }

        fn report(name: &str, buf: &WorldBuffer) {
            let rms = rms_slope(&buf.elevation, &buf.terrain, buf.width, buf.height, 0..buf.width, 0..buf.height);
            let spread = slope_spread(&buf.elevation, &buf.terrain, buf.width, buf.height);
            let drainage = drainage_density(buf);
            let hi = hypsometric_integral(&buf.elevation, &buf.terrain);
            let coast_frac = coast_on_boundary_fraction(buf);
            let depth_corr = depth_distance_correlation(buf);
            println!(
                "{name:<12} rms_slope={rms:.5}  slope_spread={spread:.3}  drainage_density={:.1}%  hypsometric={hi:.3}  coast_on_boundary={:.1}%  sea_depth_vs_dist_r={depth_corr:.3}",
                drainage * 100.0,
                if coast_frac.is_nan() { 0.0 } else { coast_frac * 100.0 },
            );
        }

        let (w, h) = (900u32, 500u32);
        println!("\n== TERRAIN 2.0 metrics @ {w}x{h} (docs/TERRAIN_2_PLAN.md section 3) ==");

        let mut plate_buf = continent(w, h, w / 12);
        let n = plate_buf.total();
        plate_buf.plate_index = vec![0u16; n];
        plate_buf.boundary_type = vec![0u8; n];
        plate_buf.is_volcanic = vec![0u8; n];
        crate::sim::plates::generate_plates_and_landmass(&mut plate_buf, 7, 14);
        generate_elevation(&mut plate_buf, 7);
        compute_sea_depth(&mut plate_buf);
        generate_shelves(&mut plate_buf, 7, 12.0, 0.4, 0.3, 8.0);
        report("plates", &plate_buf);

        let mut shape_buf = continent(w, h, w / 12);
        generate_elevation_from_terrain(&mut shape_buf, 7, 0.5, 0.5, 0.5, 0.4);
        compute_sea_depth(&mut shape_buf);
        generate_shelves(&mut shape_buf, 7, 12.0, 0.4, 0.3, 8.0);
        report("shape", &shape_buf);

        let mut ridged_buf = continent(w, h, w / 12);
        generate_elevation_ridged(&mut ridged_buf, 7, 0.5, 0.5, 0.5, 0.4);
        compute_sea_depth(&mut ridged_buf);
        generate_shelves(&mut ridged_buf, 7, 12.0, 0.4, 0.3, 8.0);
        report("ridged", &ridged_buf);

        let mut cordillera_buf = continent(w, h, w / 12);
        generate_elevation_cordillera(&mut cordillera_buf, 7, 0.5, 0.5, 0.5, 0.4);
        compute_sea_depth(&mut cordillera_buf);
        generate_shelves(&mut cordillera_buf, 7, 12.0, 0.4, 0.3, 8.0);
        report("cordillera", &cordillera_buf);

        println!("(coast_on_boundary is only meaningful for `plates`, the only model with real boundary_type)");
    }


    /// TERRAIN 2.0 slice 4 (D1/T1) gate: the coastline must depart from the
    /// raw plate Voronoi edge, not merely compute a numerically different
    /// crust field that happens to threshold into the identical `terrain`
    /// (the real failure mode this test caught during tuning -- see the
    /// comment on `warp_strength` in `plates.rs`). Two probe worlds/seeds
    /// so a single lucky configuration can't pass by chance.
    #[test]
    fn coastline_departs_from_the_plate_boundary() {
        for (seed, plate_count) in [(42u64, 10u32), (777, 16)] {
            let (w, h) = (400u32, 250u32);
            let mut buf = continent(w, h, w / 12);
            let n = buf.total();
            buf.plate_index = vec![0u16; n];
            buf.boundary_type = vec![0u8; n];
            buf.is_volcanic = vec![0u8; n];
            crate::sim::plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
            let mut coastal = 0u32;
            let mut on_boundary = 0u32;
            for y in 0..h { for x in 0..w {
                let i = buf.idx(x, y);
                if buf.terrain[i] != 1 { continue; }
                let mut is_coast = false;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    if buf.terrain[buf.idx(nx, ny)] != 1 { is_coast = true; break; }
                }
                if !is_coast { continue; }
                coastal += 1;
                if buf.boundary_type[i] != 0 { on_boundary += 1; continue; }
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    if buf.boundary_type[buf.idx(nx, ny)] != 0 { on_boundary += 1; break; }
                }
            }}
            assert!(coastal > 0, "seed {seed}: probe world grew no coastline at all");
            let frac = on_boundary as f32 / coastal as f32;
            assert!(frac < 0.85,
                "seed {seed}: coastline still reads as the raw Voronoi edge: {:.1}% of coastal \
                 land cells sit on a plate boundary (a pre-slice-4 world measures ~100%)",
                frac * 100.0);
        }
    }

    // ── EROSION TEXTURE DIAGNOSTIC ──────────────────────────────────────────
    //
    // The question this answers is the one a hillshade actually poses: how much
    // of the relief lives at the GRID SCALE (a one-cell notch, which the shading
    // draws as a thin dark scratch) versus at the LANDFORM scale (a massif,
    // which reads as a mountain). `rms_slope` in `terrain_metrics` cannot tell
    // those apart -- both raise it.
    //
    // `concavity` is exactly the quantity `render::tile_image::relief_at` feeds
    // its ambient-occlusion term: the cell's height against its 8-neighbour
    // mean. AO saturates at AO_REF = 240 m, so a cell sitting >120 m below its
    // own neighbourhood is already drawing at half the maximum darkening -- that
    // is the definition of a visible scratch, not an arbitrary cutoff.
    fn notch_metrics(elevation: &[f32], terrain: &[u8], w: u32, h: u32) -> (f32, f32, f32) {
        const M: f32 = 8848.0;
        let wi = w as i32;
        let mut notch = 0u64;
        let mut land = 0u64;
        let mut depth_sum = 0.0f64;
        let mut grid_energy = 0.0f64;
        for y in 1..h as i32 - 1 {
            for x in 0..wi {
                let i = (y * wi + x) as usize;
                if terrain[i] != 1 { continue; }
                let mut mean = 0.0f32;
                let mut ok = true;
                for (dx, dy) in [(-1i32,-1i32),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)] {
                    let nx = ((x + dx) % wi + wi) % wi;
                    let ni = ((y + dy) * wi + nx) as usize;
                    if terrain[ni] != 1 { ok = false; break; }
                    mean += elevation[ni];
                }
                if !ok { continue; }
                land += 1;
                let concavity = (mean / 8.0 - elevation[i]) * M;
                grid_energy += (concavity as f64) * (concavity as f64);
                if concavity > 120.0 { notch += 1; depth_sum += concavity as f64; }
            }
        }
        if land == 0 { return (0.0, 0.0, 0.0); }
        (
            notch as f32 / land as f32 * 100.0,
            if notch == 0 { 0.0 } else { (depth_sum / notch as f64) as f32 },
            (grid_energy / land as f64).sqrt() as f32,
        )
    }

    /// The size of the largest 8-CONNECTED chain of notch cells, as a fraction
    /// of land, meant to separate "a rough surface" from "a drawn drainage
    /// tree" (scattered roughness gives isolated specks; a carved network is one
    /// enormous component spanning a continent).
    ///
    /// NEGATIVE RESULT, recorded so it is not built into a gate: on the plate
    /// model it DOES NOT DISCRIMINATE. Re-adding `fine_carve` to a clean build
    /// and sweeping the visibility threshold gives 0.110 / 0.177 / 0.287% at
    /// 60 / 25 / 12 m against a clean 0.122 / 0.186 / 0.299% -- carved measures
    /// slightly LOWER at every threshold. The reason is `limit_grid_scale_relief`:
    /// it normalises the one-cell band to a fixed RMS, so whatever pattern the
    /// noise stack leaves is rescaled to the same amplitude either way, and no
    /// amplitude statistic downstream of it can see structure. The difference is
    /// real and obvious on the TEMPLATE path (`EROSION_MODEL=shape` in
    /// `dump_erosion_sheet`, where the carve was both absolute and stronger) --
    /// it is simply not a difference these numbers can hold. Look at the sheet.
    fn largest_notch_component(elevation: &[f32], terrain: &[u8], w: u32, h: u32) -> f32 {
        // 25 m, not the 120 m the notch-density metric uses. That figure is
        // calibrated to the HILLSHADE (half of `AO_REF`), but the flat
        // `elevation` tint has no shading at all, so a systematic pattern shows
        // there at a far shallower amplitude -- which is why the statistics once
        // read "fixed" while the map plainly was not.
        const NOTCH_VISIBLE_M: f32 = 25.0;
        let wi = w as i32;
        let n = elevation.len();
        let mut notch = vec![false; n];
        let mut land = 0u64;
        for y in 1..h as i32 - 1 {
            for x in 0..wi {
                let i = (y * wi + x) as usize;
                if terrain[i] != 1 { continue; }
                land += 1;
                let mut mean = 0.0f32;
                let mut ok = true;
                for (dx, dy) in [(-1i32,-1i32),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)] {
                    let nx = ((x + dx) % wi + wi) % wi;
                    let ni = ((y + dy) * wi + nx) as usize;
                    if terrain[ni] != 1 { ok = false; break; }
                    mean += elevation[ni];
                }
                if ok && (mean / 8.0 - elevation[i]) * 8848.0 > NOTCH_VISIBLE_M { notch[i] = true; }
            }
        }
        if land == 0 { return 0.0; }

        let mut seen = vec![false; n];
        let mut best = 0usize;
        let mut stack: Vec<usize> = Vec::new();
        for start in 0..n {
            if !notch[start] || seen[start] { continue; }
            seen[start] = true;
            stack.push(start);
            let mut size = 0usize;
            while let Some(i) = stack.pop() {
                size += 1;
                let x = (i % w as usize) as i32;
                let y = (i / w as usize) as i32;
                for (dx, dy) in [(-1i32,-1i32),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)] {
                    let ny = y + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let nx = ((x + dx) % wi + wi) % wi;
                    let ni = (ny * wi + nx) as usize;
                    if notch[ni] && !seen[ni] { seen[ni] = true; stack.push(ni); }
                }
            }
            if size > best { best = size; }
        }
        best as f32 / land as f32 * 100.0
    }

    /// Directional grid-scale curvature, in metres: RMS of the second difference
    /// along X and along Y separately. Isotropic terrain gives two similar
    /// numbers; a value much larger on one axis is a SCAN or SAMPLING artefact
    /// of that axis, not a landform -- terrain has no idea which way the array
    /// is stored.
    fn axis_curvature(elevation: &[f32], terrain: &[u8], w: u32, h: u32) -> (f32, f32) {
        let wi = w as i32;
        let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0u64);
        for y in 1..h as i32 - 1 {
            for x in 0..wi {
                let i = (y * wi + x) as usize;
                if terrain[i] != 1 { continue; }
                let xl = ((x - 1 + wi) % wi) as usize + (y * wi) as usize;
                let xr = ((x + 1) % wi) as usize + (y * wi) as usize;
                let yu = ((y - 1) * wi + x) as usize;
                let yd = ((y + 1) * wi + x) as usize;
                if terrain[xl] != 1 || terrain[xr] != 1 || terrain[yu] != 1 || terrain[yd] != 1 { continue; }
                let cx = (elevation[i] - 0.5 * (elevation[xl] + elevation[xr])) * 8848.0;
                let cy = (elevation[i] - 0.5 * (elevation[yu] + elevation[yd])) * 8848.0;
                sx += (cx as f64) * (cx as f64);
                sy += (cy as f64) * (cy as f64);
                n += 1;
            }
        }
        if n == 0 { return (0.0, 0.0); }
        ((sx / n as f64).sqrt() as f32, (sy / n as f64).sqrt() as f32)
    }

    /// Landform-scale relief: the standard deviation of a 5-cell box-blurred
    /// elevation, in metres. The pair (grid-scale RMS concavity, landform relief)
    /// is the scale separation -- big mountains, smooth flanks is a LOW first
    /// number and a HIGH second one.
    fn landform_relief(elevation: &[f32], terrain: &[u8], w: u32, h: u32) -> f32 {
        let blur = box_blur_wrap(elevation, w, h, 5);
        let mut sum = 0.0f64;
        let mut sq = 0.0f64;
        let mut n = 0u64;
        for i in 0..elevation.len() {
            if terrain[i] != 1 { continue; }
            let v = (blur[i] * 8848.0) as f64;
            sum += v; sq += v * v; n += 1;
        }
        if n == 0 { return 0.0; }
        let mean = sum / n as f64;
        ((sq / n as f64) - mean * mean).max(0.0).sqrt() as f32
    }

    /// Print the erosion texture numbers for every elevation model.
    ///   cargo test --release --lib erosion_texture_metrics -- --ignored --nocapture
    #[test]
    #[ignore]
    fn erosion_texture_metrics() {
        let (w, h) = std::env::var("EROSION_GRID").ok()
            .and_then(|s| { let mut p = s.split('x'); Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?)) })
            .unwrap_or((1800u32, 900u32));
        let km_per_cell = 40075.0 / w as f32;
        println!("\n== EROSION TEXTURE @ {w}x{h} ({km_per_cell:.1} km/cell) ==");
        println!("{:<12} {:>10} {:>12} {:>12} {:>12} {:>11}",
                 "model", "notch%", "notch_m", "gridRMS_m", "landform_m", "curvY/curvX");
        println!("(last column: largest connected notch chain, % of land -- a drawn drainage tree is ONE huge component)");
        let report = |name: &str, buf: &WorldBuffer| {
            let (nd, nm, ge) = notch_metrics(&buf.elevation, &buf.terrain, buf.width, buf.height);
            let lf = landform_relief(&buf.elevation, &buf.terrain, buf.width, buf.height);
            let (cx, cy) = axis_curvature(&buf.elevation, &buf.terrain, buf.width, buf.height);
            let comp = largest_notch_component(&buf.elevation, &buf.terrain, buf.width, buf.height);
            println!("{name:<12} {nd:>9.2}% {nm:>11.0}m {ge:>11.1}m {lf:>11.0}m {:>11.2} {comp:>11.3}%", cy / cx.max(1e-6));
        };

        let mut plate_buf = continent(w, h, w / 12);
        let n = plate_buf.total();
        plate_buf.plate_index = vec![0u16; n];
        plate_buf.boundary_type = vec![0u8; n];
        plate_buf.is_volcanic = vec![0u8; n];
        crate::sim::plates::generate_plates_and_landmass(&mut plate_buf, 7, 14);
        generate_elevation(&mut plate_buf, 7);
        report("plates", &plate_buf);

        let mut shape_buf = continent(w, h, w / 12);
        generate_elevation_from_terrain(&mut shape_buf, 7, 0.5, 0.5, 0.5, 0.4);
        report("shape", &shape_buf);

        let mut cord_buf = continent(w, h, w / 12);
        generate_elevation_cordillera(&mut cord_buf, 7, 0.5, 0.5, 0.5, 0.4);
        report("cordillera", &cord_buf);

        let mut ridged_buf = continent(w, h, w / 12);
        generate_elevation_ridged(&mut ridged_buf, 7, 0.5, 0.5, 0.5, 0.4);
        report("ridged", &ridged_buf);
    }

    /// Render a real generated world through the REAL `render_tile` path in the
    /// monochrome ANALYTICAL elevation style -- colour carries zero information
    /// there, so what is left on the page is exactly the shading, which is the
    /// thing under discussion. Writes the whole world plus a 4x crop of its most
    /// mountainous window, because grid-scale texture is invisible at world zoom
    /// (the lesson section 8.21 already paid for once with the fill light).
    ///   EROSION_SHEET_DIR=/tmp/e cargo test --release --lib dump_erosion_sheet -- --ignored --nocapture
    #[test]
    #[ignore]
    fn dump_erosion_sheet() {
        use crate::render::tile_image::{render_tile_with_neighbors_ctx, RenderCtx, TileNeighbors};
        use crate::tile::cell::TileData;
        use crate::tile::coords::TILE_SIZE;

        let (w, h) = std::env::var("EROSION_GRID").ok()
            .and_then(|s| { let mut p = s.split('x'); Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?)) })
            .unwrap_or((1800u32, 900u32));

        // EROSION_MODEL picks which generator to look at. `shape` is the
        // TEMPLATE path ("Complete from Landmass"), which is what a user who
        // imported a real-world coastline is actually running -- a different
        // generator from `plates`, with its own carve terms and its own taper.
        let model = std::env::var("EROSION_MODEL").unwrap_or_else(|_| "plates".into());
        let mut buf = continent(w, h, w / 12);
        let n = buf.total();
        buf.plate_index = vec![0u16; n];
        buf.boundary_type = vec![0u8; n];
        buf.is_volcanic = vec![0u8; n];
        match model.as_str() {
            "shape" => {
                let mut b = continent(w, h, w / 12);
                let bn = b.total();
                b.plate_index = vec![0u16; bn];
                b.boundary_type = vec![0u8; bn];
                b.is_volcanic = vec![0u8; bn];
                crate::sim::plates::generate_plates_and_landmass(&mut b, 7, 14);
                buf.terrain = b.terrain.clone(); // a real, irregular landmass to work from
                generate_elevation_from_terrain(&mut buf, 7, 0.5, 0.5, 0.5, 0.4);
            }
            "cordillera" => generate_elevation_cordillera(&mut buf, 7, 0.5, 0.5, 0.5, 0.4),
            "ridged" => generate_elevation_ridged(&mut buf, 7, 0.5, 0.5, 0.5, 0.4),
            _ => {
                crate::sim::plates::generate_plates_and_landmass(&mut buf, 7, 14);
                generate_elevation(&mut buf, 7);
            }
        }
        compute_sea_depth(&mut buf);
        generate_shelves(&mut buf, 7, 12.0, 0.4, 0.3, 8.0);

        let (nd, nm, ge) = notch_metrics(&buf.elevation, &buf.terrain, w, h);
        let lf = landform_relief(&buf.elevation, &buf.terrain, w, h);
        println!("notch={nd:.2}%  notch_m={nm:.0}  gridRMS={ge:.1}m  landform={lf:.0}m");

        let ts = TILE_SIZE as usize;
        let tw = (w as usize).div_ceil(ts);
        let th = (h as usize).div_ceil(ts);
        let mut tiles: Vec<TileData> = Vec::with_capacity(tw * th);
        for ty in 0..th {
            for tx in 0..tw {
                let mut t = TileData::new_sea();
                for ly in 0..ts {
                    for lx in 0..ts {
                        let (gx, gy) = (tx * ts + lx, ty * ts + ly);
                        if gx >= w as usize || gy >= h as usize { continue; }
                        let g = gy * w as usize + gx;
                        let l = ly * ts + lx;
                        t.terrain[l] = buf.terrain[g];
                        t.elevation[l] = buf.elevation[g];
                        t.sea_depth[l] = buf.sea_depth[g];
                    }
                }
                tiles.push(t);
            }
        }

        let dir = std::env::var("EROSION_SHEET_DIR")
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
        std::fs::create_dir_all(&dir).ok();
        let tag = std::env::var("SHEET_TAG").unwrap_or_default();

        for layer in ["terrain#style=analytical", "terrain", "elevation"] {
            let name = if layer.contains("analytical") { "analytical" } else { layer };
            let mut img = vec![0u8; w as usize * h as usize * 3];
            for ty in 0..th {
                for tx in 0..tw {
                    let at = |x: isize, y: isize| -> Option<&TileData> {
                        if x < 0 || y < 0 || x >= tw as isize || y >= th as isize { return None; }
                        tiles.get(y as usize * tw + x as usize)
                    };
                    let nb = TileNeighbors {
                        west: at(tx as isize - 1, ty as isize),
                        east: at(tx as isize + 1, ty as isize),
                        north: at(tx as isize, ty as isize - 1),
                        south: at(tx as isize, ty as isize + 1),
                    };
                    let ctx = RenderCtx { grid_w: w, grid_h: h, tx: tx as i32, ty: ty as i32, step: 1, isolate: None };
                    let rgba = render_tile_with_neighbors_ctx(&tiles[ty * tw + tx], layer, &nb, &ctx);
                    for ly in 0..ts {
                        for lx in 0..ts {
                            let (gx, gy) = (tx * ts + lx, ty * ts + ly);
                            if gx >= w as usize || gy >= h as usize { continue; }
                            let src = (ly * ts + lx) * 4;
                            let dst = (gy * w as usize + gx) * 3;
                            img[dst..dst + 3].copy_from_slice(&rgba[src..src + 3]);
                        }
                    }
                }
            }
            let path = format!("{dir}/erosion_{name}{tag}.png");
            image::save_buffer(&path, &img, w, h, image::ColorType::Rgb8).unwrap();
            println!("wrote {path}");

            let (cw, ch) = (200usize, 130usize);
            let mut best = (0usize, 0usize, -1.0f32);
            for oy in (0..h as usize - ch).step_by(30) {
                for ox in (0..w as usize - cw).step_by(30) {
                    let mut score = 0.0f32;
                    for y in (oy..oy + ch).step_by(3) {
                        for x in (ox..ox + cw).step_by(3) {
                            let g = y * w as usize + x;
                            if buf.terrain[g] == 1 { score += buf.elevation[g]; }
                        }
                    }
                    if score > best.2 { best = (ox, oy, score); }
                }
            }
            const M: usize = 4;
            let mut crop = vec![0u8; cw * M * ch * M * 3];
            for y in 0..ch * M {
                for x in 0..cw * M {
                    let src = ((best.1 + y / M) * w as usize + (best.0 + x / M)) * 3;
                    let dst = (y * cw * M + x) * 3;
                    crop[dst..dst + 3].copy_from_slice(&img[src..src + 3]);
                }
            }
            let cp = format!("{dir}/erosion_crop_{name}{tag}.png");
            image::save_buffer(&cp, &crop, (cw * M) as u32, (ch * M) as u32, image::ColorType::Rgb8).unwrap();
            println!("wrote {cp} (from {},{})", best.0, best.1);
        }
    }

    // ── Erosion appearance gates ────────────────────────────────────────────
    //
    // Three claims, one per fix, each falsifiable on its own without rendering
    // anything. The instruments that motivated them are `erosion_texture_
    // metrics` and `dump_erosion_sheet` above; these are what keeps the result.

    /// The default hypsometric target must resemble EARTH, not a plateau world.
    ///
    /// This is the regression that hid everything else: the old anchors put ~21%
    /// of land above 4000 m and only ~38% below 1000 m at the default `height`,
    /// so every generated world came out a pale high plateau with the tint ramp
    /// saturated at its top end -- which buried whatever relief was underneath.
    #[test]
    fn the_default_hypsometry_resembles_earth() {
        // ETOPO land hypsometry, % of land per 1000 m band.
        const EARTH: [f32; 9] = [71.0, 18.0, 6.0, 3.0, 1.3, 0.5, 0.15, 0.04, 0.01];
        let t = build_target_histogram(0.5, 0.0);
        let total: f32 = t.iter().sum();
        assert!((total - 100.0).abs() < 1.0, "target must sum to ~100%, got {total:.1}");

        for b in 0..9 {
            let tol = (EARTH[b] * 0.25).max(0.5);
            assert!(
                (t[b] - EARTH[b]).abs() <= tol,
                "band {b} ({}-{} km): target {:.2}% vs Earth {:.2}% (tolerance {tol:.2})",
                b, b + 1, t[b], EARTH[b],
            );
        }

        // The headline claim, stated separately because it is the one that was
        // wrong by a factor of forty.
        let above_4km: f32 = t[4..].iter().sum();
        let below_1km = t[0];
        println!("default target: {below_1km:.1}% below 1 km, {above_4km:.2}% above 4 km");
        assert!(below_1km > 65.0, "too little lowland: {below_1km:.1}%");
        assert!(above_4km < 2.5, "too much land above 4 km: {above_4km:.2}%");

        // The generators' REAL default is (0.5, 0.5), not (0.5, 0.0) -- assert
        // the shipped setting, not just the one easiest to reason about.
        let shipped = build_target_histogram(0.5, 0.5);
        let shipped_high: f32 = shipped[4..].iter().sum();
        println!("shipped default (0.5, 0.5): {:.1}% below 1 km, {shipped_high:.2}% above 4 km",
                 shipped[0]);
        assert!(shipped[0] > 63.0 && shipped_high < 3.0);

        // Even the ALPINE end of the slider must stay a planet.
        let alpine = build_target_histogram(1.0, 1.0);
        let alpine_high: f32 = alpine[4..].iter().sum();
        println!("alpine end (1.0, 1.0): {:.1}% below 1 km, {alpine_high:.2}% above 4 km", alpine[0]);
        assert!(alpine_high < 8.0, "alpine preset is not a planet: {alpine_high:.1}% above 4 km");
    }

    /// A relaxation pass's result must not depend on the order its cells happen
    /// to be stored in. `thermal_erosion` used to write both the donor and the
    /// recipient in place while scanning rows top-to-bottom, so a cell was
    /// slumped into before it was itself visited and the whole pass carried a
    /// north-to-south bias.
    ///
    /// Flipping the world in Y, eroding, and flipping back must give exactly
    /// the same field as eroding it upright -- the scan then runs the opposite
    /// way through the same terrain. The in-place version fails this; the
    /// simultaneous-update version passes it bit-exactly.
    #[test]
    fn thermal_erosion_does_not_depend_on_scan_order() {
        let (w, h) = (48u32, 32u32);
        let n = (w * h) as usize;
        let terrain = vec![1u8; n];

        // An asymmetric ridge, so a north-to-south bias has something to bite on.
        let mut a = vec![0.0f32; n];
        for y in 0..h {
            for x in 0..w {
                let fx = x as f32 / w as f32;
                let fy = y as f32 / h as f32;
                a[(y * w + x) as usize] =
                    0.15 + 0.7 * (1.0 - (fy - 0.35).abs() * 2.4).max(0.0) + 0.1 * (fx * 9.0).sin();
            }
        }

        let flip = |v: &[f32]| -> Vec<f32> {
            let mut o = vec![0.0f32; n];
            for y in 0..h {
                for x in 0..w {
                    o[(y * w + x) as usize] = v[((h - 1 - y) * w + x) as usize];
                }
            }
            o
        };

        let mut upright = a.clone();
        thermal_erosion(&mut upright, &terrain, w, h, 4);

        let mut flipped = flip(&a);
        thermal_erosion(&mut flipped, &terrain, w, h, 4);
        let unflipped = flip(&flipped);

        // Row 0 and row h-1 are skipped by the pass itself (`1..h-1`), so they
        // swap roles under the flip and are excluded rather than special-cased.
        for y in 1..h - 1 {
            for x in 0..w {
                let i = (y * w + x) as usize;
                assert!(
                    (upright[i] - unflipped[i]).abs() < 1e-6,
                    "thermal erosion is scan-order dependent at ({x},{y}): \
                     {} vs {} (flipped)",
                    upright[i], unflipped[i],
                );
            }
        }
    }


    /// `limit_grid_scale_relief` must cap the one-cell band, leave landform
    /// relief alone, and be a true no-op on a world already inside budget.
    /// All three halves matter: a limiter that also flattens the mountains
    /// would "pass" the first claim and ruin the map.
    #[test]
    fn the_grid_scale_budget_caps_texture_without_flattening_landforms() {
        let (w, h) = (128u32, 96u32);
        let n = (w * h) as usize;
        let terrain = vec![1u8; n];

        // Landform-scale relief plus heavy independent per-cell noise.
        let mut noisy = vec![0.0f32; n];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                let landform = 0.15
                    + 0.30 * (x as f32 / w as f32 * std::f32::consts::TAU).sin()
                    + 0.20 * (y as f32 / h as f32 * std::f32::consts::PI).sin();
                noisy[i] = (landform + (hash_grid(x as i32, y as i32, 4242) - 0.5) * 0.06).clamp(0.01, 1.0);
            }
        }

        let grid_rms = |v: &[f32]| -> f32 {
            let sm = box_blur_wrap(v, w, h, 1);
            let mut sq = 0.0f64;
            for i in 0..n { let d = ((v[i] - sm[i]) * 8848.0) as f64; sq += d * d; }
            (sq / n as f64).sqrt() as f32
        };
        let landform_rms = |v: &[f32]| -> f32 {
            let sm = box_blur_wrap(v, w, h, 5);
            let (mut s, mut q) = (0.0f64, 0.0f64);
            for i in 0..n { let e = (sm[i] * 8848.0) as f64; s += e; q += e * e; }
            let m = s / n as f64;
            ((q / n as f64) - m * m).max(0.0).sqrt() as f32
        };

        let before_grid = grid_rms(&noisy);
        let before_landform = landform_rms(&noisy);
        assert!(before_grid > GRID_RELIEF_BUDGET_M * 2.0,
                "fixture is not noisy enough to exercise the budget: {before_grid:.1} m");

        let mut capped = noisy.clone();
        let (reported_before, reported_after) = limit_grid_scale_relief(&mut capped, &terrain, w, h);
        assert!((reported_before - before_grid).abs() < 1.0, "reported RMS disagrees with the field");
        assert!(reported_after <= GRID_RELIEF_BUDGET_M * 1.001,
                "the iteration did not reach its own budget: {reported_after:.1} m");

        let after_grid = grid_rms(&capped);
        let after_landform = landform_rms(&capped);
        assert!(after_grid <= GRID_RELIEF_BUDGET_M * 1.05,
                "grid-scale relief not brought inside budget: {after_grid:.1} m");
        assert!((after_landform - before_landform).abs() / before_landform.max(1.0) < 0.01,
                "the budget flattened LANDFORM relief too: {before_landform:.0} m -> {after_landform:.0} m");

        // Already inside budget => bit-identical, so this can never invent relief.
        let smooth_field: Vec<f32> = box_blur_wrap(&capped, w, h, 3);
        let mut smooth_copy = smooth_field.clone();
        limit_grid_scale_relief(&mut smooth_copy, &terrain, w, h);
        assert_eq!(smooth_copy, smooth_field, "a world inside budget must be returned untouched");
    }

    /// TECTONICS_RIVERS_PROVINCES_PLAN.md Slice 7 gate (F6): on a world with NO
    /// plate data (the from-landmass path), every margin used to read as
    /// "passive" and get the identical shelf width. Two coasts on the same
    /// world -- one backed by a tall mountain range close to shore, one backed
    /// by a flat coastal plain -- must now come out with visibly different
    /// shelf widths (the relief-proxy margin maturity), not a near-1.0 ratio.
    #[test]
    fn shelf_width_varies_between_margins() {
        let (w, h) = (200u32, 100u32);
        let n = (w * h) as usize;
        let mut terrain = vec![0u8; n];
        let mut elevation = vec![0.0f32; n];
        // Two separate rectangular continents, side by side, both far enough
        // apart that their shelves can't interact.
        for y in 20..80u32 {
            for x in 10..80u32 {
                let i = (y * w + x) as usize;
                terrain[i] = 1;
                // A steep range right at the coast (x in 10..20) then flat interior.
                elevation[i] = if x < 20 { 0.55 } else { 0.15 };
            }
            for x in 120..190u32 {
                let i = (y * w + x) as usize;
                terrain[i] = 1;
                elevation[i] = 0.08; // flat coastal plain throughout
            }
        }
        let mut buf = continent(w, h, 0);
        buf.terrain = terrain;
        buf.elevation = elevation;
        buf.boundary_type = Vec::new(); // no plate data -> relief-proxy path
        buf.plate_index = Vec::new();

        generate_shelves(&mut buf, 42, 12.0, 0.4, 0.3, 8.0);

        let shelf_width_at = |cx: u32, cy: u32, dir: i32| -> u32 {
            let mut d = 0u32;
            let mut x = cx as i32;
            loop {
                x += dir;
                if x < 0 || x >= w as i32 { break; }
                let idx = buf.idx(x as u32, cy);
                if buf.terrain[idx] != 0 { break; }
                if buf.is_shelf[idx] == 1 { d += 1; } else { break; }
            }
            d
        };
        // West coast of the mountainous continent (x=10) vs west coast of the
        // flat continent (x=120): sample a few rows and average.
        let mountain_shelf: f32 = (30..70).step_by(10)
            .map(|y| shelf_width_at(10, y, -1) as f32).sum::<f32>() / 4.0;
        let flat_shelf: f32 = (30..70).step_by(10)
            .map(|y| shelf_width_at(120, y, -1) as f32).sum::<f32>() / 4.0;
        assert!(flat_shelf > mountain_shelf * 1.15,
            "flat-backed shelf ({flat_shelf}) should be visibly wider than the mountain-backed one ({mountain_shelf})");
    }
}

#[cfg(test)]
mod flat_diagnostic {
    use super::*;
    use super::tests::continent;

    /// DIAGNOSTIC (not a gate): how much of the land comes out at EXACTLY the
    /// 0.01 clamp floor (= 88.5 m of the 8848 m range), and how much of it is
    /// flat enough that a river running over it has no gradient to follow.
    ///
    /// The 0.01 floor is applied inside `redistribute_elevation`'s own band
    /// loop: band 0 spans 0..1000 m and assigns `t * 0.113` across its ranked
    /// cells, so every cell with `t < 0.0885` is clamped to the identical
    /// value. Those are by construction the LOWEST-lying land cells -- i.e.
    /// exactly the floodplains every river drains across.
    #[test]
    #[ignore]
    fn diagnose_flat_lowland() {
        for (name, mut buf) in [
            ("shape", continent(900, 500, 75)),
            ("plates", continent(900, 500, 75)),
        ] {
            let (w, h) = (buf.width, buf.height);
            if name == "plates" {
                let n = buf.total();
                buf.plate_index = vec![0u16; n];
                buf.boundary_type = vec![0u8; n];
                buf.is_volcanic = vec![0u8; n];
                crate::sim::plates::generate_plates_and_landmass(&mut buf, 7, 14);
                generate_elevation(&mut buf, 7);
            } else {
                generate_elevation_from_terrain(&mut buf, 7, 0.5, 0.5, 0.5, 0.4);
            }

            let land: Vec<usize> = (0..buf.total()).filter(|&i| buf.terrain[i] == 1).collect();
            let nl = land.len().max(1);
            let at_floor = land.iter().filter(|&&i| buf.elevation[i] <= 0.0100001).count();
            let under_1pct = land.iter().filter(|&&i| buf.elevation[i] <= 0.0102).count();

            // How many land cells have ALL 8 neighbours within 1 m of themselves
            // -- "no gradient for a river to follow".
            const ONE_M: f32 = 1.0 / 8848.0;
            let mut flat = 0usize;
            for &i in &land {
                let x = (i % w as usize) as i32;
                let y = (i / w as usize) as i32;
                let e = buf.elevation[i];
                let mut all_flat = true;
                for &(dx, dy) in &[(-1i32,-1i32),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)] {
                    let nx = buf.wrap_x(x + dx);
                    let ny = (y + dy).clamp(0, h as i32 - 1) as u32;
                    let ni = buf.idx(nx, ny);
                    if buf.terrain[ni] == 1 && (buf.elevation[ni] - e).abs() > ONE_M {
                        all_flat = false;
                        break;
                    }
                }
                if all_flat { flat += 1; }
            }

            // Largest 4-connected component of floor-valued cells.
            let mut seen = vec![false; buf.total()];
            let mut biggest = 0usize;
            for &s in &land {
                if seen[s] || buf.elevation[s] > 0.0100001 { continue; }
                let mut q = std::collections::VecDeque::new();
                q.push_back(s); seen[s] = true;
                let mut sz = 0usize;
                while let Some(c) = q.pop_front() {
                    sz += 1;
                    let x = (c % w as usize) as i32;
                    let y = (c / w as usize) as i32;
                    for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1)] {
                        let nx = buf.wrap_x(x + dx);
                        let ny = (y + dy).clamp(0, h as i32 - 1) as u32;
                        let ni = buf.idx(nx, ny);
                        if !seen[ni] && buf.terrain[ni] == 1 && buf.elevation[ni] <= 0.0100001 {
                            seen[ni] = true; q.push_back(ni);
                        }
                    }
                }
                if sz > biggest { biggest = sz; }
            }

            println!(
                "{name:<8} land={nl}  at_floor(88.5m exactly)={:.2}%  <=90m={:.2}%  \
                 no-gradient(<1m to all 8 nbrs)={:.2}%  largest_flat_component={} cells",
                at_floor as f32 * 100.0 / nl as f32,
                under_1pct as f32 * 100.0 / nl as f32,
                flat as f32 * 100.0 / nl as f32,
                biggest,
            );
        }
    }
}

#[cfg(test)]
mod plate_diagnostic {
    use super::tests::continent;

    /// DIAGNOSTIC (not a gate): the land/sea split the plate model actually
    /// produces, and how much of it is ISLAND rather than main continent.
    /// `is_oceanic = rng < 0.4` over roughly equal-area Voronoi plates means
    /// land converges on ~60% of the globe whatever the plate count; Earth is
    /// 29%. Also counts landmass components by size, because "too few islands"
    /// is a claim about the component-size distribution, not about land area.
    #[test]
    #[ignore]
    fn diagnose_plate_land_fraction() {
        for &count in &[6u32, 10, 16, 24, 40] {
            let mut land_fracs = Vec::new();
            let mut isl = (0usize, 0usize, 0usize); // small, medium, large-but-not-main
            let mut main_share = 0.0f32;
            for seed in [1u64, 2, 3] {
                let (w, h) = (600u32, 300u32);
                let mut buf = continent(w, h, 0);
                let n = buf.total();
                buf.plate_index = vec![0u16; n];
                buf.boundary_type = vec![0u8; n];
                buf.is_volcanic = vec![0u8; n];
                crate::sim::plates::generate_plates_and_landmass(&mut buf, seed, count);
                let land = buf.terrain.iter().filter(|&&t| t == 1).count();
                land_fracs.push(land as f32 / n as f32);

                // Landmass components (4-connected, wrap-aware).
                let mut seen = vec![false; n];
                let mut sizes = Vec::new();
                for s in 0..n {
                    if seen[s] || buf.terrain[s] != 1 { continue; }
                    let mut q = std::collections::VecDeque::new();
                    q.push_back(s); seen[s] = true;
                    let mut sz = 0usize;
                    while let Some(c) = q.pop_front() {
                        sz += 1;
                        let x = (c % w as usize) as i32;
                        let y = (c / w as usize) as i32;
                        for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1)] {
                            let nx = buf.wrap_x(x + dx);
                            let ny = (y + dy).clamp(0, h as i32 - 1) as u32;
                            let ni = buf.idx(nx, ny);
                            if !seen[ni] && buf.terrain[ni] == 1 { seen[ni] = true; q.push_back(ni); }
                        }
                    }
                    sizes.push(sz);
                }
                sizes.sort_unstable_by(|a, b| b.cmp(a));
                if let Some(&biggest) = sizes.first() {
                    main_share += biggest as f32 / land.max(1) as f32 / 3.0;
                }
                for (k, &sz) in sizes.iter().enumerate() {
                    if k == 0 { continue; }
                    if sz < 100 { isl.0 += 1; } else if sz < 2000 { isl.1 += 1; } else { isl.2 += 1; }
                }
            }
            let mean = land_fracs.iter().sum::<f32>() / land_fracs.len() as f32;
            println!(
                "plates={count:<3} land={:.1}% of globe (Earth 29.2%)  \
                 largest_landmass={:.0}% of all land  islands/world: tiny(<100c)={:.1} small={:.1} large={:.1}",
                mean * 100.0, main_share * 100.0,
                isl.0 as f32 / 3.0, isl.1 as f32 / 3.0, isl.2 as f32 / 3.0,
            );
        }
    }

    /// TECTONICS_RIVERS_PROVINCES_PLAN.md Slice 5 gate (F3): across plate counts
    /// and several seeds, measured land fraction must stay within a few points of
    /// `DEFAULT_OCEAN_FRACTION`'s target (30% land) -- this is
    /// `diagnose_plate_land_fraction` promoted from a diagnostic into an
    /// assertion, per the plan. It failed at 56-72% land against a 30% intent
    /// before Slice 5's construction-based oceanic/continental assignment.
    #[test]
    fn land_fraction_tracks_the_target() {
        let target_land = 1.0 - crate::sim::plates::DEFAULT_OCEAN_FRACTION;
        for &count in &[6u32, 10, 16, 24, 40] {
            for seed in [1u64, 2, 3] {
                let (w, h) = (600u32, 300u32);
                let mut buf = continent(w, h, 0);
                let n = buf.total();
                buf.plate_index = vec![0u16; n];
                buf.boundary_type = vec![0u8; n];
                buf.is_volcanic = vec![0u8; n];
                crate::sim::plates::generate_plates_and_landmass(&mut buf, seed, count);
                let land = buf.terrain.iter().filter(|&&t| t == 1).count();
                let frac = land as f32 / n as f32;
                assert!(
                    (frac - target_land).abs() < 0.10,
                    "plates={count} seed={seed}: land={:.1}% vs target {:.1}% (>10pt off)",
                    frac * 100.0, target_land * 100.0,
                );
            }
        }
    }
}
