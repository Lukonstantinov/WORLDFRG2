use rand::prelude::*;
use std::collections::VecDeque;
use super::world_buffer::WorldBuffer;

// ── Seeded noise helpers ────────────────────────────────────────────────────

/// Integer hash → pseudo-random float in 0..1
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

/// Ridged multifractal noise — creates sharp ridge lines naturally.
/// 1 - |2*noise - 1| produces v-shaped valleys → peaks.
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

/// Domain warping — distorts coordinates by noise for organic shapes
fn warped_coords(x: f32, y: f32, seed: u64, strength: f32) -> (f32, f32) {
    let wx = fbm_noise(x + 5.2, y + 1.3, seed.wrapping_add(11111), 3, 2.0, 0.5) - 0.5;
    let wy = fbm_noise(x + 8.7, y + 2.9, seed.wrapping_add(22222), 3, 2.0, 0.5) - 0.5;
    (x + wx * strength, y + wy * strength)
}

// ── Erosion ─────────────────────────────────────────────────────────────────

/// Hydraulic erosion: droplet simulation for valleys and channels
fn hydraulic_erosion(
    elevation: &mut [f32], terrain: &[u8],
    w: u32, h: u32, seed: u64, iterations: u32,
) {
    let mut rng = StdRng::seed_from_u64(seed);
    let wf = w as f32;
    let hf = h as f32;

    for _ in 0..iterations {
        let mut px = rng.gen::<f32>() * wf;
        let mut py = rng.gen::<f32>() * hf;
        let start_idx = (py as u32 * w + px as u32) as usize;
        if start_idx >= terrain.len() || terrain[start_idx] != 1 { continue; }

        let mut dir_x = 0.0f32;
        let mut dir_y = 0.0f32;
        let mut water = 1.0f32;
        let mut sediment = 0.0f32;
        let mut speed = 0.5f32;

        for _ in 0..120 {
            let ix = px as i32;
            let iy = py as i32;
            if ix < 0 || ix >= w as i32 || iy < 0 || iy >= h as i32 { break; }
            let idx = (iy as u32 * w + ix as u32) as usize;
            if terrain[idx] != 1 { break; }

            // Gradient
            let x0 = ((ix - 1 + w as i32) % w as i32) as u32;
            let x1 = ((ix + 1) % w as i32) as u32;
            let y0 = (iy - 1).max(0) as u32;
            let y1 = ((iy + 1) as u32).min(h - 1);
            let gx = (elevation[(iy as u32 * w + x1) as usize] - elevation[(iy as u32 * w + x0) as usize]) * 0.5;
            let gy = (elevation[(y1 * w + ix as u32) as usize] - elevation[(y0 * w + ix as u32) as usize]) * 0.5;

            // Update direction with inertia
            dir_x = dir_x * 0.3 - gx * 0.7;
            dir_y = dir_y * 0.3 - gy * 0.7;
            let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
            if dir_len < 0.0001 {
                let a = rng.gen::<f32>() * std::f32::consts::TAU;
                dir_x = a.cos();
                dir_y = a.sin();
            } else {
                dir_x /= dir_len;
                dir_y /= dir_len;
            }

            let npx = px + dir_x;
            let npy = py + dir_y;
            let nix = npx as i32;
            let niy = npy as i32;
            if nix < 0 || nix >= w as i32 || niy < 0 || niy >= h as i32 { break; }
            let nidx = (niy as u32 * w + nix as u32) as usize;
            let height_diff = elevation[nidx] - elevation[idx];

            let slope = (-height_diff).max(0.005);
            let capacity = (slope * speed * water * 6.0).max(0.0);

            if sediment > capacity || height_diff > 0.0 {
                let deposit = if height_diff > 0.0 {
                    sediment.min(height_diff)
                } else {
                    (sediment - capacity) * 0.02
                };
                elevation[idx] += deposit;
                sediment -= deposit;
            } else {
                let erode = ((capacity - sediment) * 0.03).min(elevation[idx] - 0.01);
                if erode > 0.0 {
                    elevation[idx] -= erode;
                    sediment += erode;
                }
            }

            speed = (speed * speed + height_diff * 4.0).max(0.0).sqrt();
            water *= 0.985;
            if water < 0.01 { break; }
            px = npx;
            py = npy;
        }
    }
}

/// Thermal erosion: material slumps from steep slopes
fn thermal_erosion(
    elevation: &mut [f32], terrain: &[u8],
    w: u32, h: u32, passes: u32,
) {
    let talus = 0.03f32;
    let rate = 0.4f32;
    let dirs: [(i32, i32); 8] = [(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)];

    for _ in 0..passes {
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
                        elevation[idx] -= transfer;
                        elevation[ni] += transfer;
                    }
                }
            }
        }
    }
}

// ── Public elevation generators ─────────────────────────────────────────────

/// Generate elevation from plate tectonics.
///
/// v2: mountains are concentrated into OROGENIC BELTS that run along the
/// convergent plate boundaries (where crust collides and thickens — the Andes /
/// Himalaya / Alps geometry), then carved into ridge-and-valley relief and
/// matched to a realistic hypsometric curve — the same erosion + redistribution
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

    // ── Orogenic front: distance (in cells) from the nearest convergent boundary
    // land cell. Ranges bloom here and fade inland. Transform boundaries add a
    // weaker uplift (transpressional ranges). ──
    let mut orogeny_dist = vec![u16::MAX; n];
    {
        let mut queue = VecDeque::new();
        for i in 0..n {
            if terrain[i] != 1 || !have_boundary { continue; }
            let b = buf.boundary_type[i];
            if b == 1 || b == 3 {
                orogeny_dist[i] = 0;
                queue.push_back(i);
            }
        }
        while let Some(ci) = queue.pop_front() {
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            let d = orogeny_dist[ci];
            if d >= 240 { continue; }
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(cx + dx);
                let ny = (cy + dy).clamp(0, h as i32 - 1) as u32;
                let ni = buf.idx(nx, ny);
                if terrain[ni] == 1 && orogeny_dist[ni] > d + 1 {
                    orogeny_dist[ni] = d + 1;
                    queue.push_back(ni);
                }
            }
        }
    }
    // Belt half-width (cells) scales with map size so ranges are a plausible
    // fraction of a continent wide at any resolution.
    let belt_reach = (w as f32 * 0.045).clamp(14.0, 90.0);

    // Absolute feature wavelengths (in cells) → feature COUNT scales with the map.
    let f_base = 1.0 / 760.0;   // broad continental swell
    let f_range = 1.0 / 210.0;  // ridge wavelength
    let f_hill = 1.0 / 52.0;    // fine hills
    let warp = 1.8f32;
    const RIDGE_AMP: f32 = 0.95;
    const HILL_AMP: f32 = 0.07;

    let mut elevation = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            if terrain[idx] != 1 { continue; }
            let ax = x as f32;
            let ay = y as f32;

            // Belt strength from convergent proximity (smoothstep), multiplied by
            // a low-frequency noise so ranges break into segments and passes
            // instead of forming one unbroken wall on the boundary line.
            let od = orogeny_dist[idx];
            let mut belt = if od == u16::MAX {
                0.0
            } else {
                (1.0 - od as f32 / belt_reach).clamp(0.0, 1.0)
            };
            belt = belt * belt * (3.0 - 2.0 * belt); // smoothstep
            let belt_noise = fbm_noise(ax * f_base * 2.3 + 19.0, ay * f_base * 2.3 + 5.0,
                                       seed.wrapping_add(0xB317), 4, 2.0, 0.5);
            belt *= 0.35 + 0.65 * belt_noise;

            let base = fbm_noise(ax * f_base + 3.1, ay * f_base + 7.7, seed, 5, 2.0, 0.5);
            let (rx, ry) = warped_coords(ax * f_range, ay * f_range, seed.wrapping_add(0x9E37), warp);
            let ridge = ridged_multifractal(rx, ry, seed.wrapping_add(0x48271), 7, 2.1, 2.0);
            let hill = fbm_noise(ax * f_hill, ay * f_hill, seed.wrapping_add(0xFEED), 3, 2.0, 0.45);
            let mut e = base * 0.42 + ridge * belt * RIDGE_AMP + hill * HILL_AMP;

            // Divergent boundaries are rifts (continental rift valleys / nascent
            // ocean) — pull the surface DOWN a little where crust is stretching.
            if have_boundary && buf.boundary_type[idx] == 2 {
                e *= 0.7;
            }

            // Valley incision: a higher-frequency inverted ridged field dissects
            // the ranges into ridge-and-valley relief.
            let vridge = ridged_multifractal(rx * 1.9, ry * 1.9, seed.wrapping_add(0x5A1F), 5, 2.0, 2.0);
            let carve = (1.0 - vridge).powi(2) * 0.16 * e;
            // Fine dendritic drainage: a small ABSOLUTE incision everywhere so
            // even lowland interiors get subtle channels for rivers to bed into
            // (matches the template path; keeps plains from sheet-flowing).
            let dridge = ridged_multifractal(rx * 3.4, ry * 3.4, seed.wrapping_add(0x0DDA), 4, 2.0, 2.0);
            let fine_carve = (1.0 - dridge).powi(2) * 0.045;
            elevation[idx] = (e - carve - fine_carve).clamp(0.01, 1.5);
        }
    }

    // ── Distance-from-coast (full flood) for the coastal falloff ──
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

    // ── Erosion (hydraulic droplets + thermal slump) then hypsometric match ──
    let hydro_iterations = ((n as f32 * 0.012) as u32).clamp(15_000, 90_000);
    hydraulic_erosion(&mut elevation, &terrain, w, h, seed.wrapping_add(42), hydro_iterations);
    thermal_erosion(&mut elevation, &terrain, w, h, 3);

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
        redistribute_elevation(&mut elevation, &terrain, n, &target);
    }

    // Terrain-aware micro-relief so no land area is ever a perfectly flat, mono
    // plateau (plateaus stay smooth, hillsides roll, floodplains stay flat).
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

    // ── Active vs passive margins ── An ACTIVE margin (a coast riding a
    // convergent/transform plate boundary — the Pacific "Ring of Fire" geometry)
    // has a NARROW, steep shelf plunging to a trench; a PASSIVE margin (trailing
    // edge, no nearby boundary — the Atlantic geometry) builds a BROAD shelf. We
    // find ocean cells whose nearest coast is active and shrink their shelf.
    // Falls back to all-passive when no plate data is loaded (template worlds).
    let active_dist: Vec<u16> = if buf.boundary_type.is_empty() {
        Vec::new()
    } else {
        let ar = (w as f32 * 0.02).clamp(6.0, 40.0) as u16;
        // Boundary proximity over land (BFS from convergent/transform cells).
        let mut bdist = vec![u16::MAX; buf.total()];
        let mut q = VecDeque::new();
        for i in 0..buf.total() {
            if buf.terrain[i] == 1 && matches!(buf.boundary_type[i], 1 | 3) {
                bdist[i] = 0;
                q.push_back(i);
            }
        }
        while let Some(ci) = q.pop_front() {
            let d = bdist[ci];
            if d >= ar { continue; }
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
        // Flood into the ocean from cells adjacent to an ACTIVE coast (coastal
        // land within `ar` of a boundary), capped at the shelf's reach.
        let cap = (base_width + drop_w + 6.0) as u16;
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
    };

    let noise_scale = w.max(h) as f32 / 20.0;
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
            let local_width = (base_width * (1.0 + (noise - 0.5) * 2.0 * noise_amount)).max(1.0);

            let mut land_count = 0u32;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    if buf.terrain[ni] == 1 { land_count += 1; }
                }
            }
            let gentle_factor = 1.0 + (land_count as f32 / 49.0) * 0.5;
            // Active margin (nearest coast is on a plate boundary): pinch the
            // shelf to a narrow, steep apron. Passive margin keeps the broad shelf.
            let margin_factor = if !active_dist.is_empty()
                && active_dist[idx] != u16::MAX
                && (active_dist[idx] as f32) <= d + 1.5
            {
                0.5
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

    // ── Step 1: Generate base heightmap from multi-layer noise ───────────
    let scale = w.max(h) as f32 / 8.0;

    // Noise weights — density/height/roughness control the mix
    let w_large = 0.55;
    let w_medium = 0.30;
    let w_small = 0.10 + roughness * 0.10;   // 0.10-0.20
    let w_ridge = 0.15 + density * 0.20;      // 0.15-0.35 (more density → more ridges)
    let total_w = w_large + w_medium + w_small + w_ridge;
    let n_large = w_large / total_w;
    let n_medium = w_medium / total_w;
    let n_small = w_small / total_w;
    let n_ridge = w_ridge / total_w;

    let warp_strength = 0.3 + roughness * 0.3; // 0.3-0.6
    let med_scale = 2.5;
    // Mountain spread → ridge frequency: narrow peaks (0) use a higher frequency
    // (tight, isolated ranges), wide ranges (1) a lower frequency (broad, long
    // cordillera). Spans roughly med_scale×1.7 … med_scale×0.6.
    let ridge_scale = med_scale * (1.7 - spread * 1.1);

    // ── ABSOLUTE-wavelength interior relief (measured in CELLS, not map fractions).
    // The `scale`-based fields above tie every feature to the map size, so the
    // interior of a large continent spans barely one noise period and comes out a
    // smooth dome — which the hypsometric redistribution then flattens into a
    // uniform "green blob" (the user's report). These absolute-frequency belts +
    // hills add ranges and texture whose COUNT scales with continent AREA (the same
    // trick as generate_elevation_ridged), so interiors are never featureless.
    let f_belt_abs = 1.0 / 540.0;                     // orogenic-belt spacing (cells)
    let f_range_abs = 1.0 / (120.0 + spread * 230.0); // ridge wavelength (cells)
    let f_hill_abs = 1.0 / 52.0;                       // fine hills (cells)
    let warp_abs = 1.4 + roughness * 1.4;
    let ridge_amp_abs = 0.35 + density * 0.55;
    let hill_amp_abs = 0.05 + roughness * 0.12;

    // ── Step 1a: distance-from-coast for every land cell ────────────────
    // Computed up front so interior cells can be lifted into a broad
    // continental rise — otherwise low-frequency noise leaves whole
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
            // Full flood — no distance cap. The old `d >= 250` early-out left the
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

            // Small features (hills) — two scales so interiors keep fine
            // texture the river router can follow into natural channels instead
            // of sheet-flowing across a smooth plain.
            let small_a = fbm_noise(wnx * 6.0, wny * 6.0, seed.wrapping_add(65521), 3, 2.0, 0.4);
            let small_b = fbm_noise(wnx * 13.0, wny * 13.0, seed.wrapping_add(0xF19E), 3, 2.0, 0.42);
            let small = small_a * 0.62 + small_b * 0.38;

            // Ridged multifractal — elongated ridge lines (the key for mountain chains).
            // Frequency set by mountain_spread (narrow peaks ↔ wide ranges).
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
            // injects real rank variation into interiors → visible relief after
            // redistribution instead of one flat band.
            let combined = large * n_large + medium * n_medium + small * n_small
                + ridge * n_ridge + abs_relief * 0.55;

            // ── Valley incision ─────────────────────────────────────────────
            // A second ridged field at higher frequency, INVERTED, carves
            // dendritic valley networks between the ranges (the troughs the
            // erosion alone left too shallow). Scaled by the local height so
            // highlands get dissected into ridge-and-valley relief while
            // lowlands stay broad. This is what was missing — "almost no valleys".
            let vridge = ridged_multifractal(wnx * ridge_scale * 1.8, wny * ridge_scale * 1.8, seed.wrapping_add(0x5A1F), 5, 2.0, 2.0);
            let carve = (1.0 - vridge).powi(2) * (0.14 + 0.22 * roughness) * combined;

            // ── Fine dendritic drainage (moderate, natural) ─────────────────
            // A high-frequency inverted ridged field carves shallow valleys
            // EVERYWHERE — a small ABSOLUTE incision independent of local height,
            // so broad lowland interiors also get subtle channels for rivers to
            // bed into. Without it, plains stayed too flat and rivers ran straight
            // or braided across sheet-flow terrain.
            let dridge = ridged_multifractal(wnx * ridge_scale * 3.1, wny * ridge_scale * 3.1, seed.wrapping_add(0x0DDA), 4, 2.0, 2.0);
            let fine_carve = (1.0 - dridge).powi(2) * (0.035 + 0.05 * roughness);

            elevation[idx] = (combined - carve - fine_carve).clamp(0.01, 1.0);
        }
    }

    // ── Step 2: Coastal falloff — gentle shoreline taper that KEEPS coastal
    // mountains. The old falloff multiplied the outer ring down to 0.15, which
    // flattened every coast into a plain — even active margins where a cordillera
    // meets the sea (Andes, Norway, BC). Now the taper only pulls DOWN the low /
    // plain component: a genuine coastal ridge (high raw height) keeps most of its
    // elevation, while flats still ramp gently from the shore so there's no cliff.
    const COAST_DIST: u16 = 4;
    for i in 0..n {
        if terrain[i] != 1 { continue; }
        if coast_dist[i] < COAST_DIST {
            let ratio = coast_dist[i] as f32 / COAST_DIST as f32; // 0 shore .. 1 inland
            let taper = 0.45 + 0.55 * ratio;                      // shore keeps ≥45%
            let ridge_keep = ((elevation[i] - 0.35) / 0.65).clamp(0.0, 1.0); // mountainous?
            let factor = taper.max(ridge_keep);
            elevation[i] *= factor;
        }
    }

    // ── Step 3: Hydraulic erosion — droplet simulation ──────────────────
    // Scale iterations with world size (small worlds ~15K, large ~100K)
    let erosion_scale = 0.5 + roughness * 0.5; // rougher = more erosion detail
    let hydro_iterations = ((n as f32 * 0.015 * erosion_scale) as u32).clamp(15_000, 100_000);
    hydraulic_erosion(&mut elevation, &terrain, w, h, seed.wrapping_add(42), hydro_iterations);

    // ── Step 4: Thermal erosion — smooth sharp ridges ───────────────────
    // Fewer passes than before (2-4) so the carved valley networks aren't
    // smoothed back out — thermal slump fills valleys, so over-applying it was a
    // second reason interiors read as flat.
    let thermal_passes = 2 + (roughness * 2.0) as u32; // 2-4 passes
    thermal_erosion(&mut elevation, &terrain, w, h, thermal_passes);

    // ── Step 5: Normalize with realistic altitude distribution ──────────
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
        // Low height (0.1) → exponent 2.0 (very flat), high (1.0) → exponent 1.0 (tall peaks)
        let exponent = 2.0 - height;
        for i in 0..n {
            if terrain[i] == 1 { elevation[i] = elevation[i].powf(exponent); }
        }
        // Percentile cap: find 99.8th percentile and scale to target
        // height parameter controls the target: 0.1 → cap at 0.4 (~3500m), 1.0 → cap at 0.95 (~8400m)
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

        // ── Step 5b: Histogram redistribution (WF1 redistributeElevation) ───
        // This is what gives WF1 its realistic, fully-differentiated terrain —
        // it spreads land elevations across 1000 m bands to a target hypsometric
        // curve (preserving relative order), so interiors are never a flat patch.
        // The `height` slider interpolates the target between a low, coastal
        // world and a dramatic alpine one; `density` biases toward more highland.
        let target = build_target_histogram(height, density);
        redistribute_elevation(&mut elevation, &terrain, n, &target);
    }

    // ── Step 6: Terrain-aware micro-relief, then write back to buffer ────
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
/// this model uses ABSOLUTE feature wavelengths measured in cells — so the number
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

    // Absolute feature wavelengths (in cells) → feature COUNT scales with map size.
    let f_base = 1.0 / 760.0;                       // broad continental swells
    let f_belt = 1.0 / 540.0;                       // orogenic-belt spacing
    let f_range = 1.0 / (120.0 + spread * 230.0);   // ridge wavelength (narrow↔broad)
    let f_hill = 1.0 / 52.0;                         // fine hills
    let warp = 1.4 + roughness * 1.4;
    let ridge_amp = 0.35 + density * 0.55;
    let hill_amp = 0.05 + roughness * 0.12;

    // ── Distance-from-coast (full flood) for the coastal falloff below ──
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

    // ── Compose: continental base + belt-masked ridged ranges + fine hills ──
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

            // Valley incision: a higher-frequency inverted ridged field carves
            // dendritic valleys through the ranges so highlands read as proper
            // ridge-and-valley relief instead of smooth domes.
            let vridge = ridged_multifractal(rx * 2.1, ry * 2.1, seed.wrapping_add(0x5A1F), 5, 2.0, 2.0);
            let carve = (1.0 - vridge).powi(2) * (0.14 + 0.20 * roughness) * e;
            // Fine dendritic drainage everywhere (see generate_elevation_from_terrain).
            let dridge = ridged_multifractal(rx * 3.6, ry * 3.6, seed.wrapping_add(0x0DDA), 4, 2.0, 2.0);
            let fine_carve = (1.0 - dridge).powi(2) * (0.035 + 0.05 * roughness);

            elevation[idx] = (e - carve - fine_carve).clamp(0.01, 1.5);
        }
    }

    // ── Coastal falloff that KEEPS coastal mountains (only the plain component is
    // tapered toward the shore — see generate_elevation_from_terrain). ──
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

    // ── Erosion (hydraulic droplets + thermal slump) ──
    let erosion_scale = 0.5 + roughness * 0.5;
    let hydro_iterations = ((n as f32 * 0.015 * erosion_scale) as u32).clamp(15_000, 100_000);
    hydraulic_erosion(&mut elevation, &terrain, w, h, seed.wrapping_add(42), hydro_iterations);
    let thermal_passes = 2 + (roughness * 2.0) as u32; // fewer passes so valleys survive
    thermal_erosion(&mut elevation, &terrain, w, h, thermal_passes);

    // ── Normalize + hypsometric redistribution (realistic altitude spread) ──
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
        redistribute_elevation(&mut elevation, &terrain, n, &target);
    }

    // Terrain-aware micro-relief (plateaus smooth, hillsides roll, flats flat).
    apply_micro_relief(&mut elevation, &terrain, w, h, seed.wrapping_add(0x31C7));

    for i in 0..n {
        buf.elevation[i] = if terrain[i] == 1 { elevation[i] } else { 0.0 };
    }
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

/// Terrain-aware MICRO-RELIEF dither — guarantees there are no perfectly flat,
/// mono-height plateaus while keeping genuine flats (floodplains, high tablelands)
/// readable as flat. Two bands:
///   • FLOOR  (~±2 m): a fine, per-cell dither applied EVERYWHERE, so no two
///     adjacent land cells ever hold the exact same height — the "very minor
///     fluctuation" the map should always have.
///   • RELIEF (~±14 m): a rolling, few-cell undulation gated by LOCAL SLOPE, so
///     hillsides and mountain flanks get rolling texture while low-slope surfaces
///     (floodplains AND high plateaus) stay smooth — a real high desert/steppe
///     reads as a tableland, and lowland floodplains stay flat enough for rivers
///     to meander across them (see rivers.rs meander pass).
/// Runs on the finished (redistributed) surface, in normalized-elevation units,
/// with amplitudes far below the ~18 m lake-fill threshold so it never spawns
/// spurious lakes, yet far above the 9 mm drainage ε so drainage is unaffected.
fn apply_micro_relief(elevation: &mut [f32], terrain: &[u8], w: u32, h: u32, seed: u64) {
    const MAX_ELEV: f32 = 8848.0;
    let floor_amp = 2.0 / MAX_ELEV;    // ~±2 m everywhere
    let relief_amp = 14.0 / MAX_ELEV;  // ~±14 m on true slopes
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
            // Fine per-cell dither (−1..1) and a rolling few-cell undulation (−1..1).
            let fine = (fbm_noise(ax * 0.9 + 0.3, ay * 0.9 + 0.7, s_fine, 2, 2.0, 0.5) - 0.5) * 2.0;
            let roll = (fbm_noise(ax * 0.16 + 0.9, ay * 0.16 + 0.2, s_roll, 3, 2.0, 0.5) - 0.5) * 2.0;
            let delta = floor_amp * fine + relief_amp * rough * roll;
            elevation[i] = (base[i] + delta).clamp(0.01, 1.0);
        }
    }
}

/// Build a 9-band target elevation histogram (% of land per 1000 m band).
/// `height` interpolates between a low coastal world and a dramatic alpine one;
/// `density` shifts a little extra mass into the highland bands.
fn build_target_histogram(height: f32, density: f32) -> [f32; 9] {
    // Anchors (must each sum to ~100). LOW = flat/coastal, HIGH = alpine.
    const LOW:  [f32; 9] = [62.0, 20.0, 9.0, 4.0, 2.5, 1.5, 0.6, 0.3, 0.1];
    const HIGH: [f32; 9] = [22.0, 14.0, 14.0, 13.0, 12.0, 10.0, 8.0, 4.5, 2.5];
    let t = height.clamp(0.0, 1.0);
    let mut out = [0.0f32; 9];
    for b in 0..9 {
        out[b] = LOW[b] * (1.0 - t) + HIGH[b] * t;
    }
    // Density nudges mass from the lowest band into the mid/high bands.
    let shift = density.clamp(0.0, 1.0) * out[0] * 0.20;
    out[0] -= shift;
    for b in 2..6 { out[b] += shift / 4.0; }
    out
}

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
        let band_min = (band as f32 * 1000.0) / MAX_ELEV;
        let band_max = ((band as f32 + 1.0) * 1000.0) / MAX_ELEV;
        let mut j = 0usize;
        while j < band_count && cell_idx < total_land {
            let idx = land_indices[cell_idx];
            let t = if band_count > 1 { j as f32 / (band_count - 1) as f32 } else { 0.5 };
            elevation[idx] = (band_min + t * (band_max - band_min)).clamp(0.01, 1.0);
            j += 1;
            cell_idx += 1;
        }
    }
    // Any leftover cells (rounding) → top band.
    let last_min = (8.0 * 1000.0) / MAX_ELEV;
    while cell_idx < total_land {
        elevation[land_indices[cell_idx]] = (last_min + 0.01).min(1.0);
        cell_idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Micro-relief must (1) leave no perfectly flat mono-plateau — a uniform
    /// input comes out with adjacent cells differing — while (2) staying tiny
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

        // (2) bounded: a flat surface has zero slope → only the ~2 m floor applies.
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

        // (3) deterministic: same seed → identical result.
        let mut b = vec![flat; n];
        apply_micro_relief(&mut b, &terrain, w, h, 777);
        assert_eq!(a, b, "micro-relief must be reproducible for a given seed");
    }

    /// A LARGE continent interior must not read as a flat "green blob": the
    /// template elevation model must give deep-interior cells genuine relief
    /// (ranges/hills), not just the ±2 m micro-relief floor. Guards the
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
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0,
            terrain: terrain.clone(), elevation: vec![0.0; n],
            sea_depth: vec![0.0; n], is_shelf: vec![0u8; n], is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(), plate_index: Vec::new(), boundary_type: Vec::new(),
            is_volcanic: Vec::new(), temperature: Vec::new(), precipitation: Vec::new(),
            koppen: Vec::new(), soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(),
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
        assert!(max_dev < 0.002, "…but still under the lake threshold: {max_dev}");
    }
}
