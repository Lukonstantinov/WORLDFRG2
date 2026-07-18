use rand::prelude::*;
use crate::sim::world_buffer::WorldBuffer;

/// Boundary types
const BOUNDARY_NONE: u8 = 0;
const BOUNDARY_CONVERGENT: u8 = 1;
const BOUNDARY_DIVERGENT: u8 = 2;
const BOUNDARY_TRANSFORM: u8 = 3;

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

    // Assign cells to nearest plate (Voronoi)
    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            let mut best_dist = f32::MAX;
            let mut best_plate = 0u16;

            for (pi, plate) in plates.iter().enumerate() {
                let mut dx = x as f32 - plate.cx;
                // Wrap distance for cylindrical topology
                if dx > w / 2.0 { dx -= w; }
                if dx < -w / 2.0 { dx += w; }
                let dy = y as f32 - plate.cy;
                let dist = dx * dx + dy * dy;
                if dist < best_dist {
                    best_dist = dist;
                    best_plate = pi as u16;
                }
            }

            buf.plate_index[idx] = best_plate;
        }
    }

    // Classify boundaries and set terrain
    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            let my_plate = buf.plate_index[idx] as usize;

            // Check if this is a boundary cell (neighbor has different plate)
            let mut is_boundary = false;
            let mut neighbor_plate = my_plate;

            for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = (y as i32 + dy).clamp(0, buf.height as i32 - 1) as u32;
                let ni = buf.idx(nx, ny);
                let np = buf.plate_index[ni] as usize;
                if np != my_plate {
                    is_boundary = true;
                    neighbor_plate = np;
                    break;
                }
            }

            if is_boundary && neighbor_plate < plates.len() {
                let p1 = &plates[my_plate];
                let p2 = &plates[neighbor_plate];

                // Relative motion at boundary
                let rel_vx = p1.vx - p2.vx;
                let rel_vy = p1.vy - p2.vy;

                // Direction from p1 center to boundary
                let mut bx = x as f32 - p1.cx;
                if bx > w / 2.0 { bx -= w; }
                if bx < -w / 2.0 { bx += w; }
                let by = y as f32 - p1.cy;
                let blen = (bx * bx + by * by).sqrt().max(0.001);
                let bnx = bx / blen;
                let bny = by / blen;

                // Dot product: positive = divergent, negative = convergent
                let dot = rel_vx * bnx + rel_vy * bny;
                // Cross product magnitude for transform
                let cross = (rel_vx * bny - rel_vy * bnx).abs();

                if dot < -0.3 {
                    buf.boundary_type[idx] = BOUNDARY_CONVERGENT;
                } else if dot > 0.3 {
                    buf.boundary_type[idx] = BOUNDARY_DIVERGENT;
                } else if cross > 0.3 {
                    buf.boundary_type[idx] = BOUNDARY_TRANSFORM;
                } else {
                    buf.boundary_type[idx] = BOUNDARY_NONE;
                }
            } else {
                buf.boundary_type[idx] = BOUNDARY_NONE;
            }

            // Set terrain based on plate type
            if my_plate < plates.len() {
                let plate = &plates[my_plate];
                if plate.is_oceanic {
                    buf.terrain[idx] = 0; // sea
                } else {
                    buf.terrain[idx] = 1; // land
                    buf.elevation[idx] = 0.05 + rng.gen::<f32>() * 0.05; // low base elevation
                }
            }
        }
    }

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

