use std::collections::VecDeque;
use super::world_buffer::WorldBuffer;

/// Compute wind belts based on latitude.
/// Trade winds (0-30), Westerlies (30-60), Polar easterlies (60-90).
/// Matches WF1 wind-belts.ts exactly.
pub fn compute_wind_belts(buf: &mut WorldBuffer) {
    for y in 0..buf.height {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();
        let sign = if lat >= 0.0 { 1.0 } else { -1.0 }; // NH=1, SH=-1

        let (vx, vy) = if abs_lat <= 30.0 {
            // Trade winds: blow toward equator and westward
            (-0.707, sign * 0.707)
        } else if abs_lat <= 60.0 {
            // Westerlies: blow eastward and slightly poleward
            (0.707, -sign * 0.707)
        } else {
            // Polar easterlies: blow westward and equatorward
            (-0.707, sign * 0.707)
        };

        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            buf.wind_vx[idx] = vx as f32;
            buf.wind_vy[idx] = vy as f32;
        }
    }
}

// ── Speed constants ──────────────────────────────────────────────────────────
const SPEED_BOUNDARY_WEST: f32 = 2.2;    // Gulf Stream, Kuroshio, Agulhas
const SPEED_BOUNDARY_EAST: f32 = 0.55;   // California, Canary, Benguela
const SPEED_INTERIOR: f32 = 0.30;        // Sargasso Sea / mid-gyre return flow
const SPEED_EQUATORIAL: f32 = 1.1;       // NEC, SEC (trade-wind driven)
const SPEED_ECC: f32 = 0.75;             // Equatorial Counter-Current
const SPEED_ACC: f32 = 1.6;              // Antarctic Circumpolar Current
const SPEED_SUBPOLAR: f32 = 0.65;        // Labrador / Irminger gyre

// ── Bathymetry constants ──────────────────────────────────────────────────────
const BATHY_STEER: f32 = 0.20;   // isobath-following strength
const BATHY_RIDGE: f32 = 0.35;   // extra deflection around shallow ridges

const CIRCUMPOLAR_FRAC: f32 = 0.10;
const DEFLECTION_PASSES: usize = 20;
const DIR_NONE: u8 = 255;

// ── Compass direction vectors ────────────────────────────────────────────────
// 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
const DIR_VEC: [(i32, i32); 8] = [
    ( 0, -1), // N
    ( 1, -1), // NE
    ( 1,  0), // E
    ( 1,  1), // SE
    ( 0,  1), // S
    (-1,  1), // SW
    (-1,  0), // W
    (-1, -1), // NW
];

const TURN_RIGHT: [u8; 8] = [2, 3, 4, 5, 6, 7, 0, 1];
const TURN_LEFT: [u8; 8] = [6, 7, 0, 1, 2, 3, 4, 5];
const REVERSE: [u8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

/// Discretize a continuous (vx, vy) vector to nearest compass index.
fn vec_to_dir(vx: f32, vy: f32) -> u8 {
    let angle = vy.atan2(vx);
    (((((angle / (std::f32::consts::PI / 4.0)).round() as i32) + 2) % 8 + 8) % 8) as u8
}

/// BFS flood from all land cells. Returns integer distance to nearest land.
fn compute_depth_bfs(terrain: &[u8], w: u32, h: u32) -> Vec<u16> {
    let n = (w * h) as usize;
    let mut depth = vec![0u16; n];
    let mut visited = vec![false; n];
    let mut queue = VecDeque::new();

    for i in 0..n {
        if terrain[i] != 0 {
            visited[i] = true;
            queue.push_back(i);
        }
    }

    while let Some(idx) = queue.pop_front() {
        let cx = (idx % w as usize) as i32;
        let cy = (idx / w as usize) as i32;
        let d = depth[idx];
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let ni = (ny as u32 * w + nx as u32) as usize;
            if visited[ni] { continue; }
            visited[ni] = true;
            depth[ni] = d + 1;
            queue.push_back(ni);
        }
    }
    depth
}

/// Compute depth gradient (points toward shallower water / land).
fn compute_depth_gradient(depth_field: &[u16], terrain: &[u8], w: u32, h: u32) -> (Vec<f32>, Vec<f32>) {
    let n = (w * h) as usize;
    let mut grad_x = vec![0.0f32; n];
    let mut grad_y = vec![0.0f32; n];

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if terrain[i] != 0 { continue; }
            let xm = ((x as i32 - 1 + w as i32) % w as i32) as u32;
            let xp = (x + 1) % w;
            let ym = if y > 0 { y - 1 } else { 0 };
            let yp = if y + 1 < h { y + 1 } else { h - 1 };
            grad_x[i] = (depth_field[(y * w + xm) as usize] as f32
                - depth_field[(y * w + xp) as usize] as f32) / 2.0;
            grad_y[i] = (depth_field[(ym * w + x) as usize] as f32
                - depth_field[(yp * w + x) as usize] as f32) / 2.0;
        }
    }
    (grad_x, grad_y)
}

/// Detect narrow ocean passages and return speed multiplier field.
fn detect_passages(terrain: &[u8], w: u32, h: u32) -> Vec<f32> {
    let n = (w * h) as usize;
    let mut accel = vec![1.0f32; n];
    let max_scan: i32 = 12;

    for y in 1..h - 1 {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if terrain[i] != 0 { continue; }

            // N-S span
            let mut ns = 1i32;
            for d in 1..=max_scan {
                if (y as i32 - d) < 0 || terrain[((y as i32 - d) as u32 * w + x) as usize] != 0 { break; }
                ns += 1;
            }
            for d in 1..=max_scan {
                if y as i32 + d >= h as i32 || terrain[((y as i32 + d) as u32 * w + x) as usize] != 0 { break; }
                ns += 1;
            }

            // E-W span
            let mut ew = 1i32;
            for d in 1..=max_scan {
                let nx = ((x as i32 - d + w as i32) % w as i32) as u32;
                if terrain[(y * w + nx) as usize] != 0 { break; }
                ew += 1;
            }
            for d in 1..=max_scan {
                let nx = ((x as i32 + d) % w as i32) as u32;
                if terrain[(y * w + nx) as usize] != 0 { break; }
                ew += 1;
            }

            let min_span = ns.min(ew);
            if min_span < 8 {
                accel[i] = 1.0 + (1.0f32).min((8 - min_span) as f32 * 0.14);
            }
        }
    }
    accel
}

/// Precompute west/east distances to nearest land for every ocean cell.
fn precompute_basin_dist(terrain: &[u8], w: u32, h: u32) -> (Vec<u16>, Vec<u16>) {
    let n = (w * h) as usize;
    let scan_limit = (w / 2) as u16;
    let mut dist_w = vec![scan_limit; n];
    let mut dist_e = vec![scan_limit; n];

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if terrain[i] != 0 { continue; }
            let mut dw = scan_limit;
            let mut de = scan_limit;
            for d in 1..=scan_limit as i32 {
                if dw == scan_limit {
                    let nx = ((x as i32 - d + w as i32) % w as i32) as u32;
                    if terrain[(y * w + nx) as usize] != 0 { dw = d as u16; }
                }
                if de == scan_limit {
                    let nx = ((x as i32 + d) % w as i32) as u32;
                    if terrain[(y * w + nx) as usize] != 0 { de = d as u16; }
                }
                if dw < scan_limit && de < scan_limit { break; }
            }
            dist_w[i] = dw;
            dist_e[i] = de;
        }
    }
    (dist_w, dist_e)
}

/// Precompute north/south distances to nearest land for every ocean cell.
fn precompute_basin_dist_ns(terrain: &[u8], w: u32, h: u32) -> (Vec<u16>, Vec<u16>) {
    let n = (w * h) as usize;
    let scan_limit = (h / 2) as u16;
    let mut dist_n = vec![scan_limit; n];
    let mut dist_s = vec![scan_limit; n];

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if terrain[i] != 0 { continue; }
            let mut dn = scan_limit;
            let mut ds = scan_limit;
            for d in 1..=scan_limit as i32 {
                if dn == scan_limit {
                    let ny = y as i32 - d;
                    if ny < 0 || terrain[(ny as u32 * w + x) as usize] != 0 { dn = d as u16; }
                }
                if ds == scan_limit {
                    let sy = y as i32 + d;
                    if sy >= h as i32 || terrain[(sy as u32 * w + x) as usize] != 0 { ds = d as u16; }
                }
                if dn < scan_limit && ds < scan_limit { break; }
            }
            dist_n[i] = dn;
            dist_s[i] = ds;
        }
    }
    (dist_n, dist_s)
}

/// Core gyre-aware current vector for one ocean cell.
/// Direct port of WF1 current-generator.ts gyreVector().
fn gyre_vector(
    _x: u32, y: u32,
    dw: u16, de: u16, dn: u16, ds: u16,
    grid_h: u32,
    equator_offset: f32,
    lat_scale: f32,
    circumpolar_active: bool,
    circumpolar_row: u32,
) -> (f32, f32) {
    if circumpolar_active && y >= circumpolar_row {
        return (SPEED_ACC, 0.0);
    }

    let lat = super::world_buffer::lat_from_y(y as f32, grid_h as f32, equator_offset, lat_scale);
    let abs_lat = lat.abs();
    let nh = lat >= 0.0;

    // Basin geometry
    let bw = (dw as f32).max(1.0);
    let be = (de as f32).max(1.0);
    let basin_pos = (bw - be) / (bw + be); // -1=west coast, +1=east coast
    let basin_width = bw + be;
    let ew_factor = (basin_width / 30.0).min(1.0);
    let ns_extent = dn as f32 + ds as f32;
    let ns_factor = (ns_extent / 20.0).min(1.0);
    let width_factor = ew_factor.min(ns_factor);
    let abs_b = basin_pos.abs();

    // Boundary current profiles (exponential decay from coast)
    let wb_profile = if basin_pos < -0.25 {
        (-((1.0 + basin_pos) / 0.30).powi(2)).exp()
    } else {
        0.0
    };
    let eb_profile = if basin_pos > 0.15 {
        (-((1.0 - basin_pos) / 0.45).powi(2)).exp()
    } else {
        0.0
    };

    // Latitude envelopes
    let subtrop_env = (-((abs_lat - 25.0).powi(2)) / (2.0 * 13.0 * 13.0)).exp();
    let subpol_env = 0.65 * (-((abs_lat - 55.0).powi(2)) / (2.0 * 8.0 * 8.0)).exp();

    // Meridional component
    let poleward = if nh { -1.0f32 } else { 1.0 };
    let equatorward = -poleward;

    let mut vy = 0.0f32;

    // Western boundary: strong poleward (subtropical) / equatorward (subpolar)
    if wb_profile > 0.05 {
        vy += poleward * wb_profile * subtrop_env * 2.8 * width_factor;
        vy += equatorward * wb_profile * subpol_env * 2.0 * width_factor;
    }

    // Eastern boundary: equatorward (subtropical) / poleward (subpolar)
    if eb_profile > 0.05 {
        vy += equatorward * eb_profile * subtrop_env * 1.2 * width_factor;
        vy += poleward * eb_profile * subpol_env * 0.8 * width_factor;
    }

    // Interior Sverdrup drift
    if abs_b < 0.75 && abs_lat > 5.0 && abs_lat < 50.0 {
        let interior_weight = 1.0 - abs_b / 0.75;
        let ns_suppress = (ns_extent / 25.0).min(1.0);
        let sverdrup = equatorward * interior_weight * subtrop_env * 0.40 * width_factor * ns_suppress;
        vy += sverdrup;
    }

    // Subpolar interior: gentle poleward drift
    if abs_b < 0.6 && abs_lat >= 45.0 && abs_lat < 68.0 {
        let interior_weight = 1.0 - abs_b / 0.6;
        vy += poleward * interior_weight * subpol_env * 0.20 * width_factor;
    }

    // Equatorial vy suppression (Coriolis -> 0 near equator)
    if abs_lat < 8.0 {
        vy *= abs_lat / 8.0;
    }

    // Zonal component
    let vx: f32;
    let band_speed: f32;

    if abs_lat < 2.0 {
        // Equatorial Undercurrent — weak eastward
        vx = 0.3;
        band_speed = SPEED_ECC * 0.5;
    } else if abs_lat <= 10.0 {
        // Equatorial counter-current (NECC / SECC)
        let ecc_str = if nh { 1.0 } else { 0.6 };
        vx = ecc_str * ((std::f32::consts::PI * (abs_lat - 2.0) / 8.0).sin());
        band_speed = SPEED_ECC * ecc_str;
    } else if abs_lat < 20.0 {
        // Trade wind drift — westward
        let trade_profile = 0.6 + 0.4 * ((std::f32::consts::PI * (abs_lat - 10.0) / 10.0).sin());
        vx = -trade_profile;
        band_speed = SPEED_EQUATORIAL;
    } else if abs_lat < 40.0 {
        // Smooth cosine transition trades -> westerlies. Centered so the
        // westward→eastward flip lands on 30° — the trade/westerly WIND-BELT
        // boundary (compute_wind_belts) — so the surface gyres line up with the
        // wind that drives them and with the 30° latitude reference line.
        let t = (abs_lat - 20.0) / 20.0; // t=0.5 at 30° → vx = 0
        vx = -(t * std::f32::consts::PI).cos();
        band_speed = SPEED_EQUATORIAL * (1.0 - t) + SPEED_INTERIOR * t;
    } else if abs_lat < 60.0 {
        // Westerly drift — eastward (matches the 30–60° westerly belt).
        vx = 1.0;
        band_speed = SPEED_INTERIOR;
    } else {
        // Polar easterlies — westward (matches the >60° belt).
        vx = -0.5;
        band_speed = SPEED_INTERIOR * 0.5;
    }

    // At boundary currents, zonal flow fades — current runs N-S along coast
    let bdy_zonal_suppress = wb_profile.max(eb_profile * 0.6);
    let vx = vx * (1.0 - bdy_zonal_suppress.min(0.85));

    // Speed magnitude
    let speed = if wb_profile > 0.3 {
        SPEED_INTERIOR + (SPEED_BOUNDARY_WEST - SPEED_INTERIOR) * (wb_profile * 1.5).min(1.0)
    } else if eb_profile > 0.2 {
        SPEED_INTERIOR + (SPEED_BOUNDARY_EAST - SPEED_INTERIOR) * (eb_profile * 1.3).min(1.0)
    } else if abs_lat < 20.0 {
        band_speed
    } else if abs_lat >= 50.0 && abs_lat < 70.0 {
        SPEED_SUBPOLAR
    } else {
        SPEED_INTERIOR
    } * width_factor;

    // Final normalized vector with magnitude = speed
    let raw_len = (vx * vx + vy * vy).sqrt();
    if raw_len < 0.001 { return (vx, vy); }
    ((vx / raw_len) * speed, (vy / raw_len) * speed)
}

fn is_land_at(x: i32, y: i32, terrain: &[u8], w: u32, h: u32) -> bool {
    if y < 0 || y >= h as i32 { return true; }
    let wx = ((x % w as i32) + w as i32) % w as i32;
    terrain[(y as u32 * w + wx as u32) as usize] != 0
}

/// Generate ocean currents with WF1's V3 gyre-aware algorithm.
/// Basin distances, boundary currents, coastline deflection, bathymetry steering.
pub fn generate_ocean_currents(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Build terrain bitmap for helper functions
    let terrain: Vec<u8> = buf.terrain.clone();
    // Shelf mask — the continental shelf is treated as a current boundary like the
    // coast: currents are steered along the shelf break and carry NO flow over the
    // shelf itself (real boundary currents follow the continental slope, not the
    // shallow shelf). Cloned so we can read it while writing current vectors.
    let shelf: Vec<u8> = buf.is_shelf.clone();

    // Phase 1: basin distances
    let (dist_w, dist_e) = precompute_basin_dist(&terrain, w, h);
    let (dist_n, dist_s) = precompute_basin_dist_ns(&terrain, w, h);

    // Phase 1b: bathymetry
    let depth_field = compute_depth_bfs(&terrain, w, h);
    let (grad_x, grad_y) = compute_depth_gradient(&depth_field, &terrain, w, h);
    let passage_accel = detect_passages(&terrain, w, h);

    // Detect Antarctic Circumpolar Current
    let circumpolar_row = (h as f32 * (1.0 - CIRCUMPOLAR_FRAC)) as u32;
    let mut circumpolar_active = true;
    for x in 0..w {
        if terrain[(circumpolar_row * w + x) as usize] != 0 {
            circumpolar_active = false;
            break;
        }
    }

    // Phase 2: assign gyre-aware base directions
    let mut dir = vec![DIR_NONE; n];
    let mut gyre_vx = vec![0.0f32; n];
    let mut gyre_vy = vec![0.0f32; n];

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if terrain[i] != 0 { continue; }

            if circumpolar_active && y >= circumpolar_row {
                dir[i] = 2; // E
                gyre_vx[i] = SPEED_ACC;
                gyre_vy[i] = 0.0;
                continue;
            }

            let (gvx, gvy) = gyre_vector(
                x, y, dist_w[i], dist_e[i], dist_n[i], dist_s[i],
                h, buf.equator_offset, buf.lat_scale,
                circumpolar_active, circumpolar_row,
            );

            // Bathymetry steering — rotate toward isobath direction
            let mut final_vx = gvx;
            let mut final_vy = gvy;
            let gx = grad_x[i];
            let gy = grad_y[i];
            let grad_len = (gx * gx + gy * gy).sqrt();

            if grad_len > 0.3 {
                let mut iso_x = -gy;
                let mut iso_y = gx;
                if iso_x * gvx + iso_y * gvy < 0.0 {
                    iso_x = -iso_x;
                    iso_y = -iso_y;
                }
                let iso_len = (iso_x * iso_x + iso_y * iso_y).sqrt();
                if iso_len > 0.01 {
                    iso_x /= iso_len;
                    iso_y /= iso_len;
                    let depth_val = depth_field[i];
                    let shallow_factor = if depth_val < 15 { BATHY_RIDGE } else { BATHY_STEER };
                    let steer_amount = shallow_factor * (grad_len / 3.0).min(1.0);
                    let orig_len = (final_vx * final_vx + final_vy * final_vy).sqrt();
                    final_vx = final_vx * (1.0 - steer_amount) + iso_x * orig_len * steer_amount;
                    final_vy = final_vy * (1.0 - steer_amount) + iso_y * orig_len * steer_amount;
                }
            }

            // Coast-tangent steering: within a few cells of the shore, remove the
            // velocity component pointing into land and renormalize back to the
            // original speed, so the current *turns to run along* the shelf/coast
            // instead of charging straight at it (real boundary currents hug the
            // continental slope). gx/gy point up the depth gradient ≈ toward the
            // shallower water / coastline.
            let dval = depth_field[i];
            // Steer along the shore/shelf when within a few cells of land OR at the
            // shelf break (this cell or a 4-neighbour is shelf), so currents turn to
            // run along the continental slope instead of riding up onto the shelf.
            let il = (y * w + (x + w - 1) % w) as usize;
            let ir = (y * w + (x + 1) % w) as usize;
            let iu = if y > 0 { ((y - 1) * w + x) as usize } else { i };
            let idn = if y + 1 < h { ((y + 1) * w + x) as usize } else { i };
            let near_shelf = shelf[i] == 1 || shelf[il] == 1 || shelf[ir] == 1
                || shelf[iu] == 1 || shelf[idn] == 1;
            if (dval >= 1 && dval <= 5) || near_shelf {
                let glen = (gx * gx + gy * gy).sqrt();
                if glen > 0.001 {
                    let nlx = gx / glen; // unit normal toward shore
                    let nly = gy / glen;
                    let into = final_vx * nlx + final_vy * nly;
                    if into > 0.0 {
                        let orig = (final_vx * final_vx + final_vy * final_vy).sqrt();
                        let strength = (1.2 - (dval as f32 - 1.0) / 5.0).clamp(0.4, 1.0);
                        final_vx -= into * nlx * strength;
                        final_vy -= into * nly * strength;
                        let newlen = (final_vx * final_vx + final_vy * final_vy).sqrt();
                        if newlen > 0.001 {
                            final_vx = final_vx / newlen * orig;
                            final_vy = final_vy / newlen * orig;
                        }
                    }
                }
            }

            // Passage acceleration
            let pa = passage_accel[i];
            if pa > 1.0 {
                final_vx *= pa;
                final_vy *= pa;
            }

            gyre_vx[i] = final_vx;
            gyre_vy[i] = final_vy;
            dir[i] = vec_to_dir(final_vx, final_vy);
        }
    }

    // Phase 3: coastline deflection (20 passes)
    let equator_row = buf.equator_row();
    for _pass in 0..DEFLECTION_PASSES {
        for cy in 0..h {
            if circumpolar_active && cy >= circumpolar_row { continue; }
            let nh = cy < equator_row;
            for cx in 0..w {
                let i = (cy * w + cx) as usize;
                if dir[i] == DIR_NONE { continue; }
                let d = dir[i] as usize;
                let orig_d = d;
                let (ndx, ndy) = DIR_VEC[d];
                let heading_to_land = is_land_at(cx as i32 + ndx, cy as i32 + ndy, &terrain, w, h);

                if heading_to_land {
                    let mut new_d = d;
                    let mut deflected = false;
                    for _ in 0..3 {
                        new_d = if nh { TURN_RIGHT[new_d] } else { TURN_LEFT[new_d] } as usize;
                        let (tx, ty) = DIR_VEC[new_d];
                        if !is_land_at(cx as i32 + tx, cy as i32 + ty, &terrain, w, h) {
                            deflected = true;
                            break;
                        }
                    }
                    if !deflected {
                        new_d = REVERSE[orig_d] as usize;
                    }
                    dir[i] = new_d as u8;
                }
            }
        }
    }

    // Phase 4: write output vectors with speed magnitude
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if terrain[i] != 0 { continue; }
            if dir[i] == DIR_NONE { continue; }

            let cvx = gyre_vx[i];
            let cvy = gyre_vy[i];
            let pre_dir = vec_to_dir(cvx, cvy);
            let speed = (cvx * cvx + cvy * cvy).sqrt();

            if dir[i] != pre_dir {
                // Deflected cell: discrete direction, preserve speed
                let (dvx, dvy) = DIR_VEC[dir[i] as usize];
                let d_len = ((dvx * dvx + dvy * dvy) as f32).sqrt();
                if d_len > 0.0 {
                    buf.current_vx[i] = (dvx as f32 / d_len) * speed;
                    buf.current_vy[i] = (dvy as f32 / d_len) * speed;
                } else {
                    buf.current_vx[i] = speed;
                    buf.current_vy[i] = 0.0;
                }
            } else {
                // Open ocean: continuous vector
                buf.current_vx[i] = cvx;
                buf.current_vy[i] = cvy;
            }

            // Assign current type (warm/cold).
            // Only MAJOR boundary currents carry a thermal signature — the
            // Gulf Stream / Kuroshio (warm, western boundary), the Canary /
            // Benguela / California (cold, eastern boundary), the subpolar
            // currents, and the ACC. Everything else is neutral, so climate
            // is driven by latitude rather than a sea of weak anomalies.
            let lat = buf.latitude(y);
            let abs_lat = lat.abs();
            let bw = dist_w[i] as f32;
            let be = dist_e[i] as f32;
            let basin_pos = (bw - be) / (bw.max(1.0) + be.max(1.0));

            buf.current_type[i] = if circumpolar_active && y >= circumpolar_row {
                2 // ACC (cold)
            } else if basin_pos < -0.5 && abs_lat > 18.0 && abs_lat < 55.0 && speed > 1.2 {
                1 // warm western-boundary current
            } else if basin_pos > 0.45 && abs_lat > 12.0 && abs_lat < 48.0 && speed > 0.35 {
                2 // cold eastern-boundary current
            } else if abs_lat >= 50.0 && abs_lat < 68.0 && basin_pos < -0.3 && speed > 0.45 {
                2 // cold subpolar boundary current
            } else {
                0 // neutral
            };
        }
    }

    // ── Phase 5: downstream thermal advection (Gulf Stream → North Atlantic Drift) ──
    // The geometric tagging above only marks water warm while it hugs the western
    // boundary. In reality the warmth advects *with* the flow: the Gulf Stream /
    // Kuroshio turn poleward and cross the basin as the North Atlantic / North
    // Pacific Drift, warming the eastern continent's high-latitude coast (NW
    // Europe, Alaska). Without this, those coasts are left neutral or cold and
    // freeze into tundra. We follow each warm boundary cell downstream and extend
    // the warm tag across the basin, stopping at land, at a real cold current, or
    // once the water has carried far poleward enough to have cooled.
    // Only current_type is both read (as `base_type`) and later written, so it
    // needs a snapshot; current_vx/vy are read-only here, so we read them from
    // the buffer directly instead of cloning two full f32 fields (saves ~26 MB
    // each — a meaningful chunk at large grid sizes).
    // Moderate-thermohaline coupling: salinity/density boosts current speed and
    // returns a reach multiplier for the warm conveyor (extends the downstream
    // warm tag toward high-latitude sinking zones). Requires compute_salinity to
    // have run first; if salinity is all-zero (e.g. called standalone) it falls
    // back to a temperature-only density and a ~1.0 reach.
    let reach_mult = apply_thermohaline(buf);
    extend_warm_tag(buf, reach_mult);

    // ── Phase 6: small-sea climate gate ──────────────────────────────────────
    // Currents in small / marginal seas (Mediterranean / Red Sea / Persian Gulf
    // scale) should not drive continental climate — only the great oceans carry a
    // basin-scale thermal signal. Flood-fill connected ocean bodies and strip the
    // warm/cold tag (→ neutral) from any body smaller than LARGE_SEA_FRAC of the
    // world, so small seas affect neither temperature, precipitation nor Köppen.
    // The current vectors themselves are kept (they still render); only the
    // thermal classification is cleared.
    let large_min = ((n as f32) * 0.03) as usize;
    let mut comp = vec![u32::MAX; n];
    let mut sizes: Vec<usize> = Vec::new();
    for start in 0..n {
        if terrain[start] != 0 || comp[start] != u32::MAX { continue; }
        let id = sizes.len() as u32;
        let mut count = 0usize;
        let mut q = VecDeque::new();
        comp[start] = id;
        q.push_back(start);
        while let Some(ci) = q.pop_front() {
            count += 1;
            let cx = (ci % w as usize) as i32;
            let cy = (ci / w as usize) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(cx + dx);
                let ny = cy + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                let ni = buf.idx(nx, ny as u32);
                if terrain[ni] == 0 && comp[ni] == u32::MAX {
                    comp[ni] = id;
                    q.push_back(ni);
                }
            }
        }
        sizes.push(count);
    }
    for i in 0..n {
        if terrain[i] == 0 && comp[i] != u32::MAX && sizes[comp[i] as usize] < large_min {
            buf.current_type[i] = 0;
        }
    }

    // ── Phase 7: clear flow over the continental shelf ────────────────────────
    // Done LAST (after warm-tag advection has already carried warmth across the
    // basin, so coastal climate is unaffected): the rendered current arrows /
    // streamlines should stop at the shelf break and not draw over the shallow
    // shelf. Boundary currents run along the continental slope, not the apron.
    for i in 0..n {
        if shelf[i] == 1 {
            buf.current_vx[i] = 0.0;
            buf.current_vy[i] = 0.0;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Salinity & thermohaline coupling
// ─────────────────────────────────────────────────────────────────────────────

/// Salinity is stored as u8 (0..255) mapping a ~28-42 PSU range.
const SAL_MIN_PSU: f32 = 28.0;
const SAL_MAX_PSU: f32 = 42.0;

#[inline]
fn psu_to_u8(psu: f32) -> u8 {
    (((psu - SAL_MIN_PSU) / (SAL_MAX_PSU - SAL_MIN_PSU)) * 255.0).clamp(0.0, 255.0) as u8
}

/// Sea-surface-temperature estimate purely from latitude. Used by the salinity
/// model so it does NOT depend on the `temperature` field (which is computed
/// *after* currents): that would create a salinity↔temperature↔current cycle.
#[inline]
fn sst_estimate(abs_lat: f32) -> f32 {
    let t = if abs_lat < 30.0 {
        28.0 - 0.30 * abs_lat
    } else if abs_lat < 60.0 {
        19.0 - 0.45 * (abs_lat - 30.0)
    } else {
        5.5 - 0.25 * (abs_lat - 60.0)
    };
    t.max(-2.0) // ocean water doesn't drop below ~-2°C
}

/// Latitude profile of ocean precipitation (0..1): ITCZ peak at the equator, a
/// secondary mid-latitude storm-track maximum (~52°), dry subtropics in between.
#[inline]
fn ocean_precip_norm(abs_lat: f32) -> f32 {
    let itcz = (-(abs_lat * abs_lat) / (2.0 * 8.0 * 8.0)).exp();
    let storm = 0.7 * (-((abs_lat - 52.0).powi(2)) / (2.0 * 10.0 * 10.0)).exp();
    ((0.25 + itcz + storm) / 1.30).min(1.0)
}

/// Compute sea-surface salinity from evaporation − precipitation (wind- and
/// latitude-driven), coastal river/runoff freshening, and enclosed-sea
/// concentration. Writes `buf.salinity` (u8) for sea cells; land stays 0.
///
/// Drivers (real oceanography):
///   • High where evaporation > precipitation: warm, windy, dry subtropics
///     (the great subtropical salinity maxima ~20-35°).
///   • Low where precipitation > evaporation: the equatorial ITCZ band and the
///     cool, wet high latitudes.
///   • Freshened along coasts that receive heavy continental runoff (river
///     proxy = adjacent land precipitation — keeps this self-contained in the
///     ocean step, before rivers are extracted).
///   • Concentrated in narrow, warm, enclosed seas (Red Sea / Mediterranean /
///     Persian Gulf), which become hypersaline.
pub fn compute_salinity(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let terrain = buf.terrain.clone();

    // Narrowness from basin distances (cells to nearest land in each direction).
    let (dist_w, dist_e) = precompute_basin_dist(&terrain, w, h);
    let (dist_n, dist_s) = precompute_basin_dist_ns(&terrain, w, h);

    // Resolution-aware "narrow sea" threshold (~Mediterranean width in cells).
    let narrow_cells = ((w as f32) * 0.10).max(8.0);

    let mut psu = vec![0.0f32; n];

    for y in 0..h {
        let abs_lat = buf.abs_latitude(y);
        let sst = sst_estimate(abs_lat);
        let sst_norm = ((sst + 2.0) / 30.0).clamp(0.0, 1.0);
        let op = ocean_precip_norm(abs_lat);

        for x in 0..w {
            let i = buf.idx(x, y);
            if terrain[i] != 0 { continue; }

            // Evaporation: warm + windy water evaporates fastest.
            let wind_speed = (buf.wind_vx[i] * buf.wind_vx[i]
                + buf.wind_vy[i] * buf.wind_vy[i]).sqrt().clamp(0.0, 1.0);
            let evap = (0.35 + 0.65 * sst_norm) * (0.55 + 0.45 * wind_speed);

            // E − P → salinity anomaly around the 35 PSU mean.
            let ep = evap - op;
            let mut s = 35.0 + ep * 5.5;

            // Enclosed-sea concentration: a warm, narrow, land-locked basin can't
            // exchange water freely and evaporates to hypersaline.
            let open = (dist_w[i].min(dist_e[i]) as f32).min(dist_n[i].min(dist_s[i]) as f32);
            if open < narrow_cells && abs_lat < 45.0 {
                let narrowness = 1.0 - (open / narrow_cells); // 0..1
                s += narrowness * narrowness * 4.0 * (0.4 + 0.6 * sst_norm);
            }

            psu[i] = s;
        }
    }

    // Coastal runoff freshening: BFS a freshening "plume" out from coasts,
    // weighted by the precipitation of the adjacent land (a stand-in for river
    // discharge, since rivers are extracted in a later step). Wet coasts (Amazon,
    // SE Asia, NW Europe) get markedly fresher shelf water.
    let mut fresh = vec![0.0f32; n];
    let mut dist = vec![u32::MAX; n];
    let mut load = vec![0.0f32; n];
    let mut queue = VecDeque::new();
    let max_plume = 5u32;
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if terrain[i] != 0 { continue; }
            // Seed sea cells that touch land, carrying that land's precipitation.
            let mut best = 0.0f32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(x as i32 + dx);
                let ny = buf.clamp_y(y as i32 + dy);
                let ni = buf.idx(nx, ny);
                if terrain[ni] == 1 {
                    let p = (buf.precipitation[ni] / 2000.0).clamp(0.0, 1.0);
                    if p > best { best = p; }
                }
            }
            if best > 0.0 {
                load[i] = best;
                dist[i] = 0;
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let i = buf.idx(x, y);
        let d = dist[i];
        if d >= max_plume { continue; }
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if terrain[ni] != 0 { continue; }
            if dist[ni] > d + 1 {
                dist[ni] = d + 1;
                if load[i] > load[ni] { load[ni] = load[i]; }
                queue.push_back((nx, ny));
            }
        }
    }
    for i in 0..n {
        if dist[i] < u32::MAX {
            let d = dist[i] as f32;
            fresh[i] = load[i] * (-d * d / 12.0).exp();
        }
    }

    for i in 0..n {
        if terrain[i] != 0 { continue; }
        let s = psu[i] - fresh[i] * 5.0; // up to ~5 PSU freshening at wet coasts
        buf.salinity[i] = psu_to_u8(s);
    }
}

/// Surface water-density index (0..1, higher = denser) from stored salinity and
/// the latitude SST estimate. Cold + salty = dense (sinks). Transient — used by
/// the current coupling, not persisted.
fn density_index(buf: &WorldBuffer) -> Vec<f32> {
    let n = buf.total();
    let mut d = vec![0.0f32; n];
    for y in 0..buf.height {
        let sst_norm = ((sst_estimate(buf.abs_latitude(y)) + 2.0) / 30.0).clamp(0.0, 1.0);
        for x in 0..buf.width {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { continue; }
            let sal_norm = buf.salinity[i] as f32 / 255.0;
            // Weight salinity and (inverse) temperature roughly equally.
            d[i] = (0.55 * sal_norm + 0.55 * (1.0 - sst_norm)).clamp(0.0, 1.0);
        }
    }
    d
}

/// Apply the moderate-thermohaline coupling onto the already-generated current
/// field: dense (cold + salty) water drives faster flow, and a strong
/// high-latitude sinking ("deep-water formation") zone pulls the warm surface
/// drift further downstream — the overturning conveyor. Returns a multiplier on
/// the warm-advection reach so the caller can extend the Gulf-Stream→Europe tag.
fn apply_thermohaline(buf: &mut WorldBuffer) -> f32 {
    let n = buf.total();
    let density = density_index(buf);

    // Mean ocean density (reference for the per-cell speed boost).
    let mut sum = 0.0f64;
    let mut cnt = 0u64;
    for i in 0..n {
        if buf.terrain[i] == 0 { sum += density[i] as f64; cnt += 1; }
    }
    let mean = if cnt > 0 { (sum / cnt as f64) as f32 } else { 0.5 };

    // Per-cell density anomaly → mild speed boost (denser ⇒ stronger flow).
    for i in 0..n {
        if buf.terrain[i] != 0 { continue; }
        let boost = (1.0 + 0.4 * (density[i] - mean)).clamp(0.80, 1.25);
        buf.current_vx[i] *= boost;
        buf.current_vy[i] *= boost;
    }

    // Overturning strength = how much dense water sits at high latitude (where
    // it sinks). A vigorous conveyor extends the warm-drift reach.
    let mut sink = 0.0f64;
    let mut sink_cnt = 0u64;
    for y in 0..buf.height {
        if buf.abs_latitude(y) < 50.0 { continue; }
        for x in 0..buf.width {
            let i = buf.idx(x, y);
            if buf.terrain[i] == 0 { sink += density[i] as f64; sink_cnt += 1; }
        }
    }
    let conveyor = if sink_cnt > 0 { (sink / sink_cnt as f64) as f32 } else { 0.5 };
    // Reach multiplier 1.0..~1.5 as the high-latitude sinking gets stronger.
    1.0 + 0.6 * conveyor.clamp(0.0, 0.85)
}

/// Mean high-latitude (>50°) surface density — the strength of the overturning
/// "conveyor". Returned as a warm-advection reach multiplier (1.0..~1.6).
fn thermohaline_reach(buf: &WorldBuffer) -> f32 {
    let density = density_index(buf);
    let mut sink = 0.0f64;
    let mut cnt = 0u64;
    for y in 0..buf.height {
        if buf.abs_latitude(y) < 50.0 { continue; }
        for x in 0..buf.width {
            let i = buf.idx(x, y);
            if buf.terrain[i] == 0 { sink += density[i] as f64; cnt += 1; }
        }
    }
    let conveyor = if cnt > 0 { (sink / cnt as f64) as f32 } else { 0.5 };
    1.0 + 0.6 * conveyor.clamp(0.0, 0.85)
}

/// Follow each warm boundary cell downstream along the current field and extend
/// the warm `current_type` tag across the basin (Gulf Stream → North Atlantic
/// Drift). `reach_mult` scales how far the warmth carries. Factored out so it can
/// run both in `generate_ocean_currents` and again after salinity advection.
fn extend_warm_tag(buf: &mut WorldBuffer, reach_mult: f32) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let base_type = buf.current_type.clone();
    let mut warm_add = vec![false; n];
    let max_steps = (w as f32 * 0.75 * reach_mult) as usize;
    let step_len = 1.5f32;

    for sy in 0..h {
        for sx in 0..w {
            let si = (sy * w + sx) as usize;
            if base_type[si] != 1 { continue; } // seed only on warm cells

            let mut px = sx as f32 + 0.5;
            let mut py = sy as f32 + 0.5;
            for _ in 0..max_steps {
                let xi = px.floor() as i32;
                let yi = py.floor() as i32;
                if yi < 0 || yi >= h as i32 { break; }
                let i = buf.idx(buf.wrap_x(xi), yi as u32);
                if buf.terrain[i] != 0 { break; }    // reached the coast
                if base_type[i] == 2 { break; }       // ran into a cold current

                let vmx = buf.current_vx[i];
                let vmy = buf.current_vy[i];
                let mag = (vmx * vmx + vmy * vmy).sqrt();
                // The North Atlantic/Pacific Drift is a slow, broad flow; a 0.12
                // floor cut the warm tag off mid-basin (the Gulf Stream went
                // "grey" before reaching Europe). Carry warmth through weaker drift.
                if mag < 0.05 { break; }
                let alat = buf.latitude(yi as u32).abs();
                if alat > 74.0 { break; }

                warm_add[i] = true;

                let mut hx = vmx / mag;
                let mut hy = vmy / mag;
                if alat > 35.0 {
                    let drift = ((alat - 35.0) / 18.0).clamp(0.0, 1.0);
                    hx += drift * 1.1;
                    hy *= 1.0 - 0.55 * drift;
                    let hl = (hx * hx + hy * hy).sqrt();
                    if hl > 0.001 { hx /= hl; hy /= hl; }
                }
                px += hx * step_len;
                py += hy * step_len;
            }
        }
    }

    for i in 0..n {
        if warm_add[i] && buf.terrain[i] == 0 {
            buf.current_type[i] = 1;
        }
    }
}

/// Advect salinity along the current field and re-couple it to the currents.
///
/// Real thermohaline circulation is driven by salinity imbalance: warm boundary
/// currents (Gulf Stream / Kuroshio) carry salty subtropical water poleward into
/// the fresher high latitudes, where it cools, sinks, and pulls the conveyor.
/// We therefore (1) transport salinity *downstream* along the currents so salt
/// reaches the fresh north, (2) strengthen the currents where the salinity
/// gradient (the imbalance) is steep, and (3) re-extend the warm tag along the
/// now-stronger flow. Runs after `generate_ocean_currents`.
pub fn advect_salinity_and_recouple(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // Base E−P salinity to relax toward (keeps the subtropical source salty).
    let base: Vec<f32> = buf.salinity.iter().map(|&s| s as f32).collect();
    let mut sal = base.clone();

    let dt = 2.0f32;       // cells advected per iteration
    let iters = 10;
    for _ in 0..iters {
        let src = sal.clone();
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                if buf.terrain[i] != 0 { continue; }
                // Back-trace upstream along the current and take that salinity
                // (semi-Lagrangian) → salt is carried in the flow direction.
                let vx = buf.current_vx[i];
                let vy = buf.current_vy[i];
                let px = x as f32 - vx * dt;
                let py = y as f32 - vy * dt;
                let xi = px.round() as i32;
                let yi = py.round() as i32;
                let up = if yi < 0 || yi >= h as i32 {
                    src[i]
                } else {
                    let j = buf.idx(buf.wrap_x(xi), yi as u32);
                    if buf.terrain[j] != 0 { src[i] } else { src[j] }
                };
                // Blend advected value with a little relaxation to the E−P base.
                sal[i] = 0.82 * up + 0.18 * base[i];
            }
        }
    }
    for i in 0..n {
        if buf.terrain[i] == 0 {
            buf.salinity[i] = sal[i].clamp(0.0, 255.0) as u8;
        }
    }

    // Current-power boost from the salinity gradient (the imbalance that drives
    // the flow). Steep salty↔fresh fronts strengthen the current there.
    let snap: Vec<f32> = buf.salinity.iter().map(|&s| s as f32).collect();
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { continue; }
            let sxp = snap[buf.widx(x as i32 + 1, y as i32)];
            let sxm = snap[buf.widx(x as i32 - 1, y as i32)];
            let syp = snap[buf.widx(x as i32, y as i32 + 1)];
            let sym = snap[buf.widx(x as i32, y as i32 - 1)];
            let grad = (((sxp - sxm) * 0.5).powi(2) + ((syp - sym) * 0.5).powi(2)).sqrt() / 255.0;
            let boost = (1.0 + 2.8 * grad).clamp(1.0, 1.30);
            buf.current_vx[i] *= boost;
            buf.current_vy[i] *= boost;
        }
    }

    // Re-extend the warm tag along the now stronger / saltier conveyor.
    let reach = thermohaline_reach(buf);
    extend_warm_tag(buf, reach);
}

/// Compute upwelling zones: shelf + cold current + equatorward flow + eastern boundary.
pub fn compute_upwelling_zones(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    // Per-source cooling kept small and accumulated into a delta buffer that is
    // clamped before being applied. Previously each upwelling ocean cell wrote
    // -6°C directly onto every land cell in its 5×5 footprint, so a coastal cell
    // overlapped by several upwelling sources stacked -12/-18°C and froze into
    // tundra (the "cold Mediterranean coast" artefact). Bound the total so a cool
    // upwelling coast (Peru / Namibia / California fog deserts) stays cool, not
    // glacial.
    let per_source = 1.2f32;
    let mut delta = vec![0.0f32; buf.total()];

    for y in 0..h {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();

        if abs_lat < 10.0 || abs_lat > 45.0 { continue; }

        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 { continue; }
            if buf.is_shelf[idx] == 0 { continue; }
            if buf.current_type[idx] != 2 { continue; }

            let equatorward = if lat > 0.0 {
                buf.current_vy[idx] > 0.1
            } else {
                buf.current_vy[idx] < -0.1
            };
            if !equatorward { continue; }

            let has_land_east = (1..=5).any(|dx| {
                let nx = buf.wrap_x(x as i32 + dx);
                buf.terrain[buf.idx(nx, y)] == 1
            });
            if !has_land_east { continue; }

            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny = buf.clamp_y(y as i32 + dy);
                    let ni = buf.idx(nx, ny);
                    if buf.terrain[ni] == 1 {
                        delta[ni] -= per_source;
                    }
                }
            }
        }
    }

    for i in 0..buf.total() {
        if buf.terrain[i] == 1 {
            buf.temperature[i] += delta[i].max(-4.0);
        }
    }
}

/// BFS distance from ocean for all land cells, normalized 0-1.
pub fn compute_distance_to_ocean(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let max_dist = 60u32;

    let mut dist = vec![u32::MAX; buf.total()];
    let mut queue = VecDeque::new();

    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] == 0 {
                dist[idx] = 0;
                queue.push_back((x, y));
            }
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        let idx = buf.idx(x, y);
        let d = dist[idx];
        if d >= max_dist { continue; }

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

    for i in 0..buf.total() {
        let d = dist[i].min(max_dist) as f32;
        buf.distance_to_ocean[i] = d / max_dist as f32;
    }
}
