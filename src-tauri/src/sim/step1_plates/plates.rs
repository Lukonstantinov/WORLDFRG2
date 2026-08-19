use rand::prelude::*;
use rayon::prelude::*;
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

/// Generate tectonic plates and derive landmass from plate types.
/// Matches WF1 plate-generator.ts algorithm.
pub fn generate_plates_and_landmass(buf: &mut WorldBuffer, seed: u64, plate_count: u32) {
    let mut rng = StdRng::seed_from_u64(seed);
    let w = buf.width as f32;
    let h = buf.height as f32;
    let count = plate_count.max(2) as usize;

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

    // ── Terrain 2.0 slice 4 (docs/TERRAIN_2_PLAN.md D1/T1): decouple the
    // coastline from the plate Voronoi edge. The old code below set terrain
    // purely from `plate.is_oceanic`, so every shoreline was literally a cell
    // boundary — visible as straight edges and a triangular island. A "crust
    // thickness" field (each plate's base thickness plus strong domain-warped
    // noise) lets the shoreline wander off the plate edge instead — a
    // microcontinent stranded on an oceanic plate, an embayment cutting into
    // a continental one — while the TOTAL land fraction stays exactly what
    // the plate mix already implied (a percentile threshold, not a fixed
    // cutoff), so this changes coastline SHAPE, never overall land/sea balance.

    let ls = ((cell_w + cell_h) * 0.5).max(4.0); // natural length scale: ~plate size
    // MEASURED, not guessed: an initial pass (0.25/0.75 base split, ±0.31
    // noise swing, warp 0.35·ls) left `coast_on_boundary` around 90% and
    // looked, in `dump_natural_sheet`, like an unmodified Voronoi edge. A
    // direct probe (comparing the resulting `terrain` bit-for-bit between
    // warp on and off) found why: `fbm_noise`'s 4-octave output clusters far
    // more tightly than its theoretical 0..1 range, so the realised swing
    // rarely bridged the 0.5 base gap at all -- the crust FIELD differed
    // measurably (confirmed), but the percentile THRESHOLD kept selecting the
    // identical set of cells regardless, because too few cells' rank ever
    // crossed near the cut. Widened to a ±0.9 swing (amplitude 1.8) and a
    // warp beyond a full plate-size wander -- verified by
    // `coastline_departs_from_the_plate_boundary`, which failed at the
    // intermediate 1.4/0.9 tuning on one of its two probe seeds (88.6%, still
    // over its own 85% gate) and passes both at these values.
    let warp_strength = ls * 1.15;
    let crust_freq = 1.0 / (ls * 0.30);
    // Rayon-parallel per-cell noise map (§8.9 rule 2) -- this is the biggest
    // fixed cost this slice adds to phase 2 (three `fbm_noise` evaluations
    // over the WHOLE grid), so it stays off the single-core path.
    let crust: Vec<f32> = (0..buf.total())
        .into_par_iter()
        .map(|idx| {
            let plate = &plates[buf.plate_index[idx] as usize];
            let base = if plate.is_oceanic { 0.25 } else { 0.75 };
            let ax = (idx as u32 % buf.width) as f32;
            let ay = (idx as u32 / buf.width) as f32;
            let wx = fbm_noise(ax / ls + 5.2, ay / ls + 1.3, seed.wrapping_add(0x1111_1111), 3, 2.0, 0.5) - 0.5;
            let wy = fbm_noise(ax / ls + 8.7, ay / ls + 2.9, seed.wrapping_add(0x2222_2222), 3, 2.0, 0.5) - 0.5;
            let px = ax + wx * warp_strength;
            let py = ay + wy * warp_strength;
            let noise = fbm_noise(px * crust_freq, py * crust_freq, seed.wrapping_add(0x3333_3333), 4, 2.0, 0.5);
            base + (noise - 0.5) * 1.8
        })
        .collect();

    let target_land: usize = (0..buf.total())
        .filter(|&i| !plates[buf.plate_index[i] as usize].is_oceanic)
        .count();
    let mut sorted_crust = crust.clone();
    sorted_crust.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending: highest crust first
    let threshold = if target_land == 0 {
        f32::INFINITY
    } else if target_land >= sorted_crust.len() {
        f32::NEG_INFINITY
    } else {
        sorted_crust[target_land - 1]
    };
    for i in 0..buf.total() {
        if crust[i] >= threshold {
            buf.terrain[i] = 1;
            buf.elevation[i] = 0.05 + rng.gen::<f32>() * 0.05; // low base elevation
        } else {
            buf.terrain[i] = 0;
        }
    }
    // A per-cell noise threshold at this amplitude (needed above to actually
    // decouple the coastline, not just perturb its numbers) also throws off a
    // scatter of single/few-cell specks -- dust that reads as a rendering
    // glitch, not a real archipelago. Flip any land/sea component smaller than
    // `DESPECKLE_MIN` back to its surroundings; a genuine small island or
    // inlet is still free to exist above that floor.
    despeckle_terrain(buf, DESPECKLE_MIN);

    // Classify boundaries. Rayon-parallel: each cell only reads plate_index
    // (never boundary_type), so this map has no cross-cell dependency.
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

    // Generate volcanic zones at divergent boundaries
    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            if buf.boundary_type[idx] == BOUNDARY_DIVERGENT {
                if rng.gen::<f32>() < 0.15 {
                    buf.is_volcanic[idx] = 1;
                }
            }
            // Subduction volcanism near convergent boundaries
            if buf.boundary_type[idx] == BOUNDARY_CONVERGENT {
                if rng.gen::<f32>() < 0.08 {
                    buf.is_volcanic[idx] = 1;
                }
            }
        }
    }
}

/// Below this many connected cells, a land or sea patch reads as noise dust
/// rather than a real feature (Terrain 2.0 slice 4's despeckle pass).
const DESPECKLE_MIN: usize = 90;

/// Flip any 4-connected land or sea component smaller than `min_size` cells to
/// its opposite value. The crust-threshold noise strong enough to actually
/// decouple the coastline from the Voronoi edge (see `warp_strength`'s doc
/// comment) also throws a scatter of single/few-cell specks — dust that reads
/// as a rendering glitch, not a real archipelago or lake. A genuine island or
/// inlet at or above `min_size` is untouched.
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
