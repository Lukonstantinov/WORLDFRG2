use rand::prelude::*;
use rayon::prelude::*;
use std::collections::VecDeque;
use crate::sim::world_buffer::WorldBuffer;
use crate::sim::step2_terrain::elevation::fbm_noise;

/// Boundary types. Public because the ORE-DEPOSIT placer (`sim::deposits`) reads
/// `boundary_type` to decide a mineral's tectonic setting — arc / orogen / rift /
/// craton — which is the organising principle of economic geology.
pub const BOUNDARY_NONE: u8 = 0;
pub const BOUNDARY_CONVERGENT: u8 = 1;
pub const BOUNDARY_DIVERGENT: u8 = 2;
pub const BOUNDARY_TRANSFORM: u8 = 3;

struct Plate {
    cx: f32,
    cy: f32,
    is_oceanic: bool,
    density: f32,
    vx: f32,
    vy: f32,
}

/// WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 5 (F3, D1): the target fraction of
/// the globe that ends up OCEAN. The old per-plate `is_oceanic = rng < 0.4` coin
/// flip converged on ~60% land whatever the plate count, and was *worse* at low
/// plate counts (71.8% at 6 plates) because the expected 2.4 oceanic plates has
/// real binomial variance — an unlucky draw easily yields one or zero. This
/// constant is hit BY CONSTRUCTION instead: plates are sorted by their actual
/// Voronoi cell count and marked oceanic until the target ocean AREA is met, so
/// the result is close to 70% ocean (Earth is ~71%) regardless of plate count or
/// the luck of one coin-flip sequence.
pub const DEFAULT_OCEAN_FRACTION: f32 = 0.70;

/// Generate tectonic plates and derive landmass from plate types.
/// Matches WF1 plate-generator.ts algorithm.
pub fn generate_plates_and_landmass(buf: &mut WorldBuffer, seed: u64, plate_count: u32) {
    generate_plates_and_landmass_with_target(buf, seed, plate_count, DEFAULT_OCEAN_FRACTION);
}

/// As `generate_plates_and_landmass`, but with an explicit ocean-area target
/// (Slice 5). `ocean_fraction` is clamped to a sane range so a pathological
/// input can never empty the world of land or of sea entirely.
pub fn generate_plates_and_landmass_with_target(
    buf: &mut WorldBuffer, seed: u64, plate_count: u32, ocean_fraction: f32,
) {
    let mut rng = StdRng::seed_from_u64(seed);
    let w = buf.width as f32;
    let h = buf.height as f32;
    let count = plate_count.max(2) as usize;
    let ocean_fraction = ocean_fraction.clamp(0.05, 0.95);

    // Generate plate seeds with grid-jittered distribution
    let cols = (count as f32).sqrt().ceil() as usize;
    let rows = (count + cols - 1) / cols;
    let cell_w = w / cols as f32;
    let cell_h = h / rows as f32;

    let mut plates: Vec<Plate> = Vec::with_capacity(count);
    for i in 0..count {
        let col = i % cols;
        let row = i / cols;
        let cx = (col as f32 + rng.gen::<f32>()) * cell_w;
        let cy = (row as f32 + rng.gen::<f32>()) * cell_h;
        // is_oceanic is decided BELOW, once each plate's real Voronoi area is
        // known (Slice 5) -- this coin flip only seeds the density jitter, which
        // reads independently of the final oceanic/continental assignment.
        let is_oceanic = rng.gen::<f32>() < 0.4;
        let density = if is_oceanic {
            0.7 + rng.gen::<f32>() * 0.3
        } else {
            0.4 + rng.gen::<f32>() * 0.2
        };
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let speed = 0.5 + rng.gen::<f32>() * 0.5;
        plates.push(Plate {
            cx, cy, is_oceanic, density,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
        });
    }

    // Assign cells to nearest plate (Voronoi). Rayon-parallel: each cell's
    // nearest-plate scan is independent of every other cell's (§8.9 rule 2).
    buf.plate_index = (0..buf.total())
        .into_par_iter()
        .map(|idx| {
            let x = (idx as u32 % buf.width) as f32;
            let y = (idx as u32 / buf.width) as f32;
            let mut best_dist = f32::MAX;
            let mut best_plate = 0u16;
            for (pi, plate) in plates.iter().enumerate() {
                let mut dx = x - plate.cx;
                // Wrap distance for cylindrical topology
                if dx > w / 2.0 { dx -= w; }
                if dx < -w / 2.0 { dx += w; }
                let dy = y - plate.cy;
                let dist = dx * dx + dy * dy;
                if dist < best_dist {
                    best_dist = dist;
                    best_plate = pi as u16;
                }
            }
            best_plate
        })
        .collect();

    // Slice 5: reassign is_oceanic to hit `ocean_fraction` BY CONSTRUCTION, from
    // each plate's REAL cell count (measured from the Voronoi assignment just
    // computed, not an area estimate) — shuffled first so the greedy fill order
    // isn't "biggest plates always become ocean", then accumulated until the
    // target ocean area is met.
    let mut plate_cells = vec![0u32; plates.len()];
    for &pi in &buf.plate_index { plate_cells[pi as usize] += 1; }
    let mut order: Vec<usize> = (0..plates.len()).collect();
    order.shuffle(&mut rng);
    let target_ocean_cells = (ocean_fraction as f64 * buf.total() as f64).round() as i64;
    let mut ocean_cells_so_far: i64 = 0;
    for &pi in &order {
        let cells = plate_cells[pi] as i64;
        // Take this plate as oceanic if doing so gets closer to the target than
        // leaving it continental would.
        let with = (ocean_cells_so_far + cells - target_ocean_cells).abs();
        let without = (ocean_cells_so_far - target_ocean_cells).abs();
        if with <= without {
            plates[pi].is_oceanic = true;
            ocean_cells_so_far += cells;
        } else {
            plates[pi].is_oceanic = false;
        }
    }

    // Provisional terrain straight from plate identity -- needed below to
    // know which SIDE of a boundary a cell started on before slice 4 bends
    // the actual coastline away from it.
    let orig_terrain: Vec<u8> = (0..buf.total())
        .into_par_iter()
        .map(|idx| if plates[buf.plate_index[idx] as usize].is_oceanic { 0u8 } else { 1u8 })
        .collect();

    // Classify boundaries FIRST (needed by slice 4 below). Rayon-parallel:
    // each cell only reads plate_index (never boundary_type), so this map has
    // no cross-cell dependency.
    buf.boundary_type = (0..buf.total())
        .into_par_iter()
        .map(|idx| {
            let x = (idx as u32 % buf.width) as i32;
            let y = (idx as u32 / buf.width) as i32;
            let my_plate = buf.plate_index[idx] as usize;

            // Gather every DISTINCT neighbouring plate id in the 4-neighbourhood
            // (a triple junction can touch up to three).
            let mut neighbor_plates: Vec<usize> = Vec::new();
            for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
                let nx = buf.wrap_x(x + dx);
                let ny = (y + dy).clamp(0, buf.height as i32 - 1) as u32;
                let np = buf.plate_index[buf.idx(nx, ny)] as usize;
                if np != my_plate && !neighbor_plates.contains(&np) {
                    neighbor_plates.push(np);
                }
            }

            if neighbor_plates.is_empty() || my_plate >= plates.len() {
                return BOUNDARY_NONE;
            }

            let p1 = &plates[my_plate];
            // Classify against EVERY differing neighbour and keep the
            // strongest signal (fixes D3: at a triple junction the old code
            // classified against whichever neighbour the fixed scan order
            // (-1,0)/(1,0)/(0,-1)/(0,1) happened to hit first, so the same
            // physical junction could read differently depending on which
            // side of it a cell sat). The NORMAL is now the direction between
            // the two plates' own seed points — the true Voronoi bisector
            // normal — rather than the direction from `p1`'s centre to this
            // cell, which is only a good proxy for a compact, centrally
            // sampled plate and is wrong everywhere else.
            let mut best_type = BOUNDARY_NONE;
            let mut best_signal = 0.0f32;
            for &np in &neighbor_plates {
                if np >= plates.len() { continue; }
                let p2 = &plates[np];
                let mut bx = p2.cx - p1.cx;
                if bx > w / 2.0 { bx -= w; }
                if bx < -w / 2.0 { bx += w; }
                let by = p2.cy - p1.cy;
                let blen = (bx * bx + by * by).sqrt().max(0.001);
                let bnx = bx / blen;
                let bny = by / blen;

                let rel_vx = p1.vx - p2.vx;
                let rel_vy = p1.vy - p2.vy;
                let dot = rel_vx * bnx + rel_vy * bny;
                let cross = (rel_vx * bny - rel_vy * bnx).abs();

                let (btype, signal) = if dot < -0.3 {
                    (BOUNDARY_CONVERGENT, -dot)
                } else if dot > 0.3 {
                    (BOUNDARY_DIVERGENT, dot)
                } else if cross > 0.3 {
                    (BOUNDARY_TRANSFORM, cross)
                } else {
                    (BOUNDARY_NONE, 0.0)
                };
                if signal > best_signal {
                    best_signal = signal;
                    best_type = btype;
                }
            }
            best_type
        })
        .collect();

    // ── Terrain 2.0 slice 4 (docs/TERRAIN_2_PLAN.md D1/T1): decouple the
    // coastline from the plate Voronoi edge. Two earlier passes on this
    // measured wrong: the first found the crust field genuinely differed but
    // the percentile threshold kept selecting the identical `terrain` (the
    // realised noise swing rarely bridged the base gap between an oceanic and
    // continental plate's crust value); the second widened the swing until
    // `terrain` genuinely moved, but at a single frequency uncorrelated with
    // WHERE the true boundary actually ran -- so it flipped scattered
    // isolated cells (speckle islands) far out on plates instead of bending
    // the coastline itself, since a boundary is a THIN, roughly 1-D curve and
    // a 2-D noise threshold has no notion of "near that curve".
    //
    // This pass fixes that by construction: it perturbs a SIGNED DISTANCE
    // TO THE NEAREST BOUNDARY (positive on the land side, negative on the
    // sea side) with noise, then re-thresholds at zero -- the level-set
    // technique real coastline generators use. Only cells within `REACH` of
    // an actual boundary can ever flip (a deep continental interior or open
    // ocean is never in play), and the noise frequency is tuned to complete
    // several cycles within that reach, so wherever a stretch of coast DOES
    // move, it moves as a coherent bulge (a peninsula or a bay), not a dot.
    let ls = ((cell_w + cell_h) * 0.5).max(4.0); // natural length scale: ~plate size
    let reach = (ls * 0.55).max(12.0);
    let reach_u16 = reach.ceil() as u16 + 4;

    // Distance (cells) to the nearest ANY-type boundary cell, full grid.
    let mut bnd_dist = vec![u16::MAX; buf.total()];
    {
        let mut q = VecDeque::new();
        for i in 0..buf.total() {
            if buf.boundary_type[i] != BOUNDARY_NONE {
                bnd_dist[i] = 0;
                q.push_back(i);
            }
        }
        while let Some(ci) = q.pop_front() {
            let d = bnd_dist[ci];
            if d >= reach_u16 { continue; }
            let cx = (ci % buf.width as usize) as i32;
            let cy = (ci / buf.width as usize) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(cx + dx);
                let ny = (cy + dy).clamp(0, buf.height as i32 - 1) as u32;
                let ni = buf.idx(nx, ny);
                if bnd_dist[ni] > d + 1 {
                    bnd_dist[ni] = d + 1;
                    q.push_back(ni);
                }
            }
        }
    }

    // Broad wavelength (sweeping bulges) + a shorter one (headlands/inlets on
    // a bulge's own edge), both scaled to complete multiple cycles across
    // `reach` so any boundary stretch gets real, coherent wobble.
    let freq_a = 1.0 / (reach * 0.9);
    let freq_b = 1.0 / (reach * 0.28);
    let amp = reach * 1.7;
    let score: Vec<f32> = (0..buf.total())
        .into_par_iter()
        .map(|idx| {
            let d = bnd_dist[idx];
            let signed = if orig_terrain[idx] == 1 { d as f32 } else { -(d as f32) };
            if d as f32 > reach + 6.0 {
                // Well outside any boundary's reach: keep exactly as before,
                // no noise evaluation needed (and no risk of a stray flip).
                return signed;
            }
            let ax = (idx as u32 % buf.width) as f32;
            let ay = (idx as u32 / buf.width) as f32;
            let a = fbm_noise(ax * freq_a + 11.0, ay * freq_a + 4.0, seed.wrapping_add(0x5A17_0001), 3, 2.0, 0.5);
            let b = fbm_noise(ax * freq_b + 71.0, ay * freq_b + 29.0, seed.wrapping_add(0x5A17_0002), 3, 2.0, 0.5);
            let combined = a * 0.7 + b * 0.3;
            signed + amp * (combined - 0.5) * 2.0
        })
        .collect();

    let target_land: usize = orig_terrain.iter().filter(|&&t| t == 1).count();
    let mut sorted_score = score.clone();
    sorted_score.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending
    let threshold = if target_land == 0 {
        f32::INFINITY
    } else if target_land >= sorted_score.len() {
        f32::NEG_INFINITY
    } else {
        sorted_score[target_land - 1]
    };
    for i in 0..buf.total() {
        if score[i] >= threshold {
            buf.terrain[i] = 1;
            buf.elevation[i] = 0.05 + rng.gen::<f32>() * 0.05; // low base elevation
        } else {
            buf.terrain[i] = 0;
        }
    }
    // A safety net, not the mechanism: the level-set construction above
    // can't produce a far-flung speckle island (only cells within `reach` of
    // a real boundary are ever eligible to flip), but a very tight noise
    // trough can still pinch off a handful of cells right at a headland.
    // Flip anything under `DESPECKLE_MIN` back to its surroundings.
    despeckle_terrain(buf, DESPECKLE_MIN);

    // Generate volcanic zones at divergent boundaries, and at convergent ones
    // by COLLISION TYPE (WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 4, scoped-down:
    // real collision-type differentiation, without the full persisted per-plate
    // identity / Euler-pole rewrite the plan's F5 root cause needs -- that is a
    // much larger change, deliberately not attempted this session). A
    // continent-continent collision (Himalaya-style) raises a broad plateau with
    // essentially NO volcanism; ocean-ocean (island arc) and ocean-continent
    // (arc + trench) margins are exactly where real subduction volcanism
    // concentrates. Before this every convergent cell got the identical flat 8%
    // roll regardless of what was actually colliding.
    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            if buf.boundary_type[idx] == BOUNDARY_DIVERGENT {
                if rng.gen::<f32>() < 0.15 {
                    buf.is_volcanic[idx] = 1;
                }
            }
            if buf.boundary_type[idx] == BOUNDARY_CONVERGENT {
                let my_plate = buf.plate_index[idx] as usize;
                let my_oceanic = plates.get(my_plate).map(|p| p.is_oceanic).unwrap_or(false);
                let mut touches_oceanic = my_oceanic;
                let mut touches_continental = !my_oceanic;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny = (y as i32 + dy).clamp(0, buf.height as i32 - 1) as u32;
                    let np = buf.plate_index[buf.idx(nx, ny)] as usize;
                    if np == my_plate || np >= plates.len() { continue; }
                    if plates[np].is_oceanic { touches_oceanic = true; } else { touches_continental = true; }
                }
                let chance = if touches_continental && !touches_oceanic {
                    0.01 // continent-continent: broad collisional plateau, ~no volcanism
                } else if touches_oceanic {
                    0.14 // ocean-ocean island arc / ocean-continent arc+trench
                } else {
                    0.08
                };
                if rng.gen::<f32>() < chance {
                    buf.is_volcanic[idx] = 1;
                }
            }
        }
    }
}

/// Below this many connected cells, a land or sea patch reads as noise dust
/// rather than a real feature (Terrain 2.0 slice 4's despeckle pass).
const DESPECKLE_MIN: usize = 14;

/// Flip any 4-connected land or sea component smaller than `min_size` cells to
/// its opposite value -- the level-set noise construction (see the "score"
/// pass above) can pinch off a handful of cells right at a tight headland; a
/// genuine island or inlet at or above `min_size` is untouched.
fn despeckle_terrain(buf: &mut WorldBuffer, min_size: usize) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let mut visited = vec![false; n];
    let mut queue = std::collections::VecDeque::new();
    let mut component = Vec::new();
    for start in 0..n {
        if visited[start] { continue; }
        let value = buf.terrain[start];
        visited[start] = true;
        component.clear();
        component.push(start);
        queue.push_back(start);
        while let Some(ci) = queue.pop_front() {
            let cx = (ci as u32 % w) as i32;
            let cy = (ci as u32 / w) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(cx + dx);
                let ny = (cy + dy).clamp(0, h as i32 - 1) as u32;
                let ni = buf.idx(nx, ny);
                if !visited[ni] && buf.terrain[ni] == value {
                    visited[ni] = true;
                    component.push(ni);
                    queue.push_back(ni);
                }
            }
        }
        if component.len() < min_size {
            let flipped = if value == 1 { 0 } else { 1 };
            for &ci in &component {
                buf.terrain[ci] = flipped;
                buf.elevation[ci] = if flipped == 1 { 0.05 } else { 0.0 };
            }
        }
    }
}

/// Invert land and sea
pub fn invert_terrain(buf: &mut WorldBuffer) {
    for i in 0..buf.total() {
        buf.terrain[i] = if buf.terrain[i] == 0 { 1 } else { 0 };
        if buf.terrain[i] == 0 {
            buf.elevation[i] = 0.0;
        } else if buf.elevation[i] == 0.0 {
            buf.elevation[i] = 0.05;
        }
    }
}
