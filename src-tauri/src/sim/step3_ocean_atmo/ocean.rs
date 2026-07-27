use std::collections::VecDeque;
use crate::sim::world_buffer::WorldBuffer;
use super::circulation::Circulation;

/// Prevailing surface-wind unit vector for a latitude, blended across the three
/// belts (trades → westerlies → polar easterlies). The belt boundaries come from
/// the planet's general circulation (`circulation.rs`): the trade→westerly
/// boundary sits at `circ.hadley_edge` (30° on Earth) and the westerly→polar
/// boundary at `circ.polar_front` (60° on Earth), so a faster/slower rotator moves
/// the wind belts. Factored out of `compute_wind_belts` so the seasonal-wind model
/// (`seasonal.rs`) can build on the same annual-mean belt before adding its monsoon
/// perturbation. `y` is down, so `+vy` is southward.
pub fn belt_wind(lat: f32, circ: &Circulation) -> (f32, f32) {
    // Smoothstep helper (0 below a, 1 above b, S-curve between).
    let smooth = |a: f32, b: f32, x: f32| -> f32 {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let abs_lat = lat.abs();
    let sign = if lat >= 0.0 { 1.0f32 } else { -1.0 }; // NH=1, SH=-1
    // Coriolis-deflection direction: +1 prograde (Earth-like), -1 retrograde.
    // Flips ONLY the zonal (east/west) terms below — never `sign` above, which
    // is the unrelated hemisphere (N/S) convention used for meridional flow.
    let rot = circ.rotation_sign;

    // The three belt vectors (y is down, so +y is southward).
    let trade = (-0.707f32 * rot, sign * 0.707); // toward equator + west
    let west = (0.707f32 * rot, -sign * 0.707);  // toward pole + east
    let polar = (-0.707f32 * rot, sign * 0.707); // toward equator + west

    // Blend with transitions centred on the circulation's belt boundaries (30°/60°
    // on Earth). The ±4° half-width scales with the belt so the transition stays
    // proportionally sharp on wider/narrower circulations.
    let hw_h = 4.0 * circ.belt_scale();
    let hw_f = 4.0 * circ.front_scale();
    let to_west = smooth(circ.hadley_edge - hw_h, circ.hadley_edge + hw_h, abs_lat);
    let to_polar = smooth(circ.polar_front - hw_f, circ.polar_front + hw_f, abs_lat);
    let w_polar = to_polar;
    let w_west = to_west * (1.0 - to_polar);
    let w_trade = (1.0 - to_west).max(0.0);
    let sumw = (w_trade + w_west + w_polar).max(1e-4);
    let mut vx = (trade.0 * w_trade + west.0 * w_west + polar.0 * w_polar) / sumw;
    let mut vy = (trade.1 * w_trade + west.1 * w_west + polar.1 * w_polar) / sumw;
    // Renormalize to unit strength so moisture advection keeps full force right
    // through the transition (no spurious dry/wet bands at 30Â°/60Â°).
    let mag = (vx * vx + vy * vy).sqrt();
    if mag > 0.2 { vx /= mag; vy /= mag; }
    (vx, vy)
}

/// Compute wind belts based on latitude.
/// Trade winds (0-30), Westerlies (30-60), Polar easterlies (60-90).
/// Matches WF1 wind-belts.ts exactly.
pub fn compute_wind_belts(buf: &mut WorldBuffer) {
    let circ = Circulation::for_world(buf);
    for y in 0..buf.height {
        let (vx, vy) = belt_wind(buf.latitude(y), &circ);
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            buf.wind_vx[idx] = vx;
            buf.wind_vy[idx] = vy;
        }
    }
}

// â”€â”€ Speed constants â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const SPEED_BOUNDARY_WEST: f32 = 2.2;    // Gulf Stream, Kuroshio, Agulhas
const SPEED_BOUNDARY_EAST: f32 = 0.55;   // California, Canary, Benguela
const SPEED_INTERIOR: f32 = 0.30;        // Sargasso Sea / mid-gyre return flow
const SPEED_EQUATORIAL: f32 = 1.1;       // NEC, SEC (trade-wind driven)
const SPEED_ECC: f32 = 1.3;              // Equatorial Counter-Current (raised so the
                                         // eastward counter-current seeds a visible streamline)
const SPEED_ACC: f32 = 1.6;              // Antarctic Circumpolar Current
const SPEED_SUBPOLAR: f32 = 0.65;        // Labrador / Irminger gyre

// â”€â”€ Bathymetry constants â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
const BATHY_STEER: f32 = 0.20;   // isobath-following strength
const BATHY_RIDGE: f32 = 0.35;   // extra deflection around shallow ridges

const CIRCUMPOLAR_FRAC: f32 = 0.10;
const DEFLECTION_PASSES: usize = 20;
const DIR_NONE: u8 = 255;

// â”€â”€ Compass direction vectors â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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

/// Interior Sverdrup meridional velocity (NORTHWARD positive) at a latitude,
/// derived from the curl of the belt wind stress over a β-plane — the physically
/// correct interior of a wind-driven gyre. Belt winds are ~zonal, so the vertical
/// wind-stress curl is ≈ −∂τ_x/∂y and the Sverdrup relation βV = curl(τ)/ρ gives
/// `V ∝ (−∂τ_x/∂φ)/cosφ` (the planet radius cancels between the curl and β = 2Ω
/// cosφ/R). This makes the sign and latitude structure of the gyre interior EMERGE
/// from the wind shear — subtropical interior equatorward, subpolar interior
/// poleward — instead of the old hand-tuned `subtrop_env`/`subpol_env` envelopes.
/// Normalized to ~unit peak so it drops into the existing velocity scale.
fn sverdrup_interior_north(lat: f32, circ: &Circulation) -> f32 {
    // Zonal wind stress τ_x ∝ speed² · (signed zonal unit wind). base_speed carries
    // the trade/westerly/polar speed profile; belt_wind's x-component carries the
    // sign (easterly trades & polar easterlies negative, westerlies positive). Both
    // are circulation-aware, so the wind-driven interior drift follows the belts.
    let tau_x = |l: f32| -> f32 {
        let s = crate::sim::jets::base_speed(l.abs(), circ);
        s * s * belt_wind(l, circ).0
    };
    let dl = 1.5f32; // finite-difference half-step in degrees
    let dtau_dphi = (tau_x(lat + dl) - tau_x(lat - dl)) / (2.0 * dl.to_radians());
    let cosphi = lat.to_radians().cos().max(0.15); // guard the pole
    (-dtau_dphi / cosphi / 300.0).clamp(-1.3, 1.3)
}

/// Core gyre-aware current vector for one ocean cell.
/// Direct port of WF1 current-generator.ts gyreVector().
fn gyre_vector(
    _x: u32, y: u32,
    dw: u16, de: u16, dn: u16, ds: u16,
    grid_h: u32,
    equator_offset: f32,
    lat_scale: f32,
    lat_ratio: f32,
    circumpolar_active: bool,
    circumpolar_row: u32,
    circ: &Circulation,
) -> (f32, f32) {
    if circumpolar_active && y >= circumpolar_row {
        return (SPEED_ACC, 0.0);
    }

    let lat_true = crate::sim::world_buffer::lat_from_y(y as f32, grid_h as f32, equator_offset, lat_scale, lat_ratio);
    // Surface gyres are wind-driven, so their subtropical/subpolar bands track the
    // wind belts. Work the band logic in the circulation's "belt-space" latitude
    // (true latitude ÷ belt_scale) so the gyres widen/narrow with the Hadley cell;
    // at Earth belt_scale = 1 so this is exactly the true latitude.
    let lat = circ.belt_lat(lat_true);
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
    // Western intensification (Gulf-Stream-style boundary currents hugging the
    // west coast) is itself a Coriolis effect — flip which side is
    // "intensified" under retrograde rotation. `basin_dir` is `basin_pos` for
    // prograde worlds (bit-identical to before) and its mirror for retrograde.
    let basin_dir = basin_pos * circ.rotation_sign;

    // Boundary current profiles (exponential decay from coast)
    let wb_profile = if basin_dir < -0.25 {
        (-((1.0 + basin_dir) / 0.30).powi(2)).exp()
    } else {
        0.0
    };
    let eb_profile = if basin_dir > 0.15 {
        (-((1.0 - basin_dir) / 0.45).powi(2)).exp()
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

    // Interior Sverdrup drift — now DERIVED from wind-stress curl over a β-plane
    // (see `sverdrup_interior_north`) rather than the old hand-tuned subtropical-
    // equatorward + subpolar-poleward envelopes. One term covers both gyres and both
    // hemispheres: its sign emerges from the belt-wind shear. `subtrop_env`/`subpol_env`
    // are no longer used for the interior (they remain for the boundary terms above).
    if abs_b < 0.75 && abs_lat > 5.0 && abs_lat < 68.0 {
        let interior_weight = (1.0 - abs_b / 0.75).max(0.0);
        let ns_suppress = (ns_extent / 25.0).min(1.0);
        // v_north is +northward; screen y is DOWN, so northward flow is −vy.
        // Sverdrup uses the TRUE latitude (its belt_wind/base_speed rescale
        // internally — passing belt-space here would double-count the scaling).
        let v_north = sverdrup_interior_north(lat_true, circ);
        vy += -v_north * 0.5 * interior_weight * width_factor * ns_suppress;
    }

    // Mid-latitude westerly DRIFT carries surface water poleward (Ekman + the
    // North Atlantic / North Pacific Drift): the warm water that leaves the
    // western boundary turns east and crosses the basin angling ~25-30Â° poleward
    // rather than running purely horizontal. Apply only on the poleward limb of
    // the gyre (â‰¥38Â°, where the zonal flow is eastward) and away from the coastal
    // boundary currents, so the Sverdrup interior (20-38Â°, equatorward) is kept.
    if wb_profile < 0.05 && eb_profile < 0.05 && abs_lat >= 38.0 && abs_lat < 62.0 {
        let env = (-((abs_lat - 46.0).powi(2)) / (2.0 * 12.0 * 12.0)).exp();
        vy += poleward * (0.32 + 0.34 * env) * width_factor;
    }

    // Equatorial vy suppression (Coriolis -> 0 near equator)
    if abs_lat < 8.0 {
        vy *= abs_lat / 8.0;
    }

    // Zonal component
    let vx: f32;
    let band_speed: f32;

    if abs_lat < 2.0 {
        // Equatorial Undercurrent â€” weak eastward
        vx = 0.3;
        band_speed = SPEED_ECC * 0.5;
    } else if abs_lat <= 12.0 {
        // Equatorial counter-current (NECC / SECC) â€” a distinct EASTWARD band
        // running against the trade-wind drift on either side of the equator.
        // Strengthened and widened (â‰¤12Â°) so it actually shows up as its own
        // streamline between the two westward equatorial currents.
        let ecc_str = if nh { 1.0 } else { 0.85 };
        vx = ecc_str * ((std::f32::consts::PI * (abs_lat - 2.0) / 10.0).sin().max(0.0));
        band_speed = SPEED_ECC * ecc_str;
    } else if abs_lat < 20.0 {
        // Trade wind drift â€” westward
        let trade_profile = 0.6 + 0.4 * ((std::f32::consts::PI * (abs_lat - 10.0) / 10.0).sin());
        vx = -trade_profile;
        band_speed = SPEED_EQUATORIAL;
    } else if abs_lat < 40.0 {
        // Smooth cosine transition trades -> westerlies. Centered so the
        // westwardâ†’eastward flip lands on 30Â° â€” the trade/westerly WIND-BELT
        // boundary (compute_wind_belts) â€” so the surface gyres line up with the
        // wind that drives them and with the 30Â° latitude reference line.
        let t = (abs_lat - 20.0) / 20.0; // t=0.5 at 30Â° â†’ vx = 0
        vx = -(t * std::f32::consts::PI).cos();
        band_speed = SPEED_EQUATORIAL * (1.0 - t) + SPEED_INTERIOR * t;
    } else if abs_lat < 60.0 {
        // Westerly drift â€” eastward (matches the 30â€“60Â° westerly belt).
        vx = 1.0;
        band_speed = SPEED_INTERIOR;
    } else {
        // Polar easterlies â€” westward (matches the >60Â° belt).
        vx = -0.5;
        band_speed = SPEED_INTERIOR * 0.5;
    }

    // At boundary currents, zonal flow fades â€” current runs N-S along coast
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
    // Shelf mask â€” the continental shelf is treated as a current boundary like the
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

    // The general circulation sets the wind belts that drive the gyres.
    let circ = Circulation::for_world(buf);

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
                h, buf.equator_offset, buf.lat_scale, buf.lat_ratio,
                circumpolar_active, circumpolar_row, &circ,
            );

            // Bathymetry steering â€” rotate toward isobath direction
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
            // continental slope). gx/gy point up the depth gradient â‰ˆ toward the
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
            // Only MAJOR boundary currents carry a thermal signature â€” the
            // Gulf Stream / Kuroshio (warm, western boundary), the Canary /
            // Benguela / California (cold, eastern boundary), the subpolar
            // currents, and the ACC. Everything else is neutral, so climate
            // is driven by latitude rather than a sea of weak anomalies.
            let lat = buf.latitude(y);
            let abs_lat = lat.abs();
            let bw = dist_w[i] as f32;
            let be = dist_e[i] as f32;
            let basin_pos = (bw - be) / (bw.max(1.0) + be.max(1.0));
            // Same Coriolis mirror as gyre_vector's basin_dir: the actual computed
            // current speed spike (from Phase 2) already sits on the mirrored coast
            // under retrograde rotation, so the thermal-tagging thresholds below
            // must check the same mirrored side or they'd never fire.
            let basin_dir = basin_pos * circ.rotation_sign;

            buf.current_type[i] = if circumpolar_active && y >= circumpolar_row {
                2 // ACC (cold)
            } else if basin_dir < -0.5 && abs_lat > 18.0 && abs_lat < 42.0 && speed > 1.2 {
                1 // warm western-boundary current. Capped at ~42Â° â€” the SEPARATION
                  // latitude: a western-boundary current (Kuroshio/Gulf Stream)
                  // peels away from the coast here and crosses the basin as the
                  // eastward drift (extend_warm_tag carries the warm tag across).
                  // Poleward of this the western boundary is bathed by the COLD
                  // subpolar return (Oyashio/Labrador) below â€” so Kamchatka,
                  // Hokkaido and Labrador read cold, not warm.
            } else if basin_dir > 0.45 && abs_lat > 23.0 && abs_lat < 48.0 && speed > 0.35 {
                2 // cold eastern-boundary current. Floor raised to 23Â° (Tropic of
                  // Cancer) so the whole tropical belt â€” e.g. the seas around the
                  // Indian subcontinent â€” reads NEUTRAL/grey, never cold. Tropical
                  // currents (monsoon drift, equatorial) are not cold.
            } else if abs_lat >= 48.0 && abs_lat < 68.0 && basin_dir < -0.3 && speed > 0.45 {
                2 // cold subpolar boundary current (Oyashio/Labrador). Onset at 48Â°
                  // (Kamchatka â‰ˆ53Â° â†’ cold âœ“) leaves a NEUTRAL gap 42â€“48Â° on the W
                  // boundary = the warmâ†’cold transition after the warm current
                  // separates at 42Â°, instead of snapping straight to cold blue.
            } else {
                0 // neutral
            };
        }
    }

    // â”€â”€ Phase 5: downstream thermal advection (Gulf Stream â†’ North Atlantic Drift) â”€â”€
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
    // each â€” a meaningful chunk at large grid sizes).
    // Moderate-thermohaline coupling: salinity/density boosts current speed and
    // returns a reach multiplier for the warm conveyor (extends the downstream
    // warm tag toward high-latitude sinking zones). Requires compute_salinity to
    // have run first; if salinity is all-zero (e.g. called standalone) it falls
    // back to a temperature-only density and a ~1.0 reach.
    let reach_mult = apply_thermohaline(buf);
    extend_warm_tag(buf, reach_mult);
    // Cold return limb: tag the equatorward-flowing currents cold (after warm, so
    // warm is never overwritten). The neutral gap the warm tag leaves at the bend
    // is the transition before warmth becomes cold.
    extend_cold_tag(buf);

    // â”€â”€ Phase 6: small-sea climate gate â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Currents in small / marginal seas (Mediterranean / Red Sea / Persian Gulf
    // scale) should not drive continental climate â€” only the great oceans carry a
    // basin-scale thermal signal. Flood-fill connected ocean bodies and strip the
    // warm/cold tag (â†’ neutral) from any body smaller than LARGE_SEA_FRAC of the
    // world, so small seas affect neither temperature, precipitation nor KÃ¶ppen.
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

    // â”€â”€ Phase 7: clear flow over the continental shelf â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Salinity & thermohaline coupling
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Salinity is stored as u8 (0..255) mapping a ~28-42 PSU range.
const SAL_MIN_PSU: f32 = 28.0;
const SAL_MAX_PSU: f32 = 42.0;

#[inline]
fn psu_to_u8(psu: f32) -> u8 {
    (((psu - SAL_MIN_PSU) / (SAL_MAX_PSU - SAL_MIN_PSU)) * 255.0).clamp(0.0, 255.0) as u8
}

/// Sea-surface-temperature estimate purely from latitude. Used by the salinity
/// model so it does NOT depend on the `temperature` field (which is computed
/// *after* currents): that would create a salinityâ†”temperatureâ†”current cycle.
#[inline]
fn sst_estimate(abs_lat: f32) -> f32 {
    let t = if abs_lat < 30.0 {
        28.0 - 0.30 * abs_lat
    } else if abs_lat < 60.0 {
        19.0 - 0.45 * (abs_lat - 30.0)
    } else {
        5.5 - 0.25 * (abs_lat - 60.0)
    };
    t.max(-2.0) // ocean water doesn't drop below ~-2Â°C
}

/// Latitude profile of ocean precipitation (0..1): ITCZ peak at the equator, a
/// secondary mid-latitude storm-track maximum (~52Â°), dry subtropics in between.
#[inline]
fn ocean_precip_norm(abs_lat: f32) -> f32 {
    let itcz = (-(abs_lat * abs_lat) / (2.0 * 8.0 * 8.0)).exp();
    let storm = 0.7 * (-((abs_lat - 52.0).powi(2)) / (2.0 * 10.0 * 10.0)).exp();
    ((0.25 + itcz + storm) / 1.30).min(1.0)
}

/// Compute sea-surface salinity from evaporation âˆ’ precipitation (wind- and
/// latitude-driven), coastal river/runoff freshening, and enclosed-sea
/// concentration. Writes `buf.salinity` (u8) for sea cells; land stays 0.
///
/// Drivers (real oceanography):
///   â€¢ High where evaporation > precipitation: warm, windy, dry subtropics
///     (the great subtropical salinity maxima ~20-35Â°).
///   â€¢ Low where precipitation > evaporation: the equatorial ITCZ band and the
///     cool, wet high latitudes.
///   â€¢ Freshened along coasts that receive heavy continental runoff (river
///     proxy = adjacent land precipitation â€” keeps this self-contained in the
///     ocean step, before rivers are extracted).
///   â€¢ Concentrated in narrow, warm, enclosed seas (Red Sea / Mediterranean /
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

            // E âˆ’ P â†’ salinity anomaly around the 35 PSU mean.
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
/// the latitude SST estimate. Cold + salty = dense (sinks). Transient â€” used by
/// the current coupling, not persisted.
/// Sea-surface-temperature anomaly (°C) a current imparts, scaled by its speed as a
/// "volume" proxy. Mirrors the land current-influence in `temperature.rs` so the
/// ocean and the coasts it warms/cools tell the same story. Warm western-boundary
/// currents (Gulf Stream / Kuroshio) raise SST; cold eastern-boundary / subpolar
/// currents lower it.
#[inline]
fn current_sst_anomaly(current_type: u8, speed: f32) -> f32 {
    let vol = (speed / 1.4).clamp(0.35, 1.3);
    match current_type {
        1 => 3.0 * vol,  // warm
        2 => -2.2 * vol, // cold
        _ => 0.0,
    }
}

/// Actual sea-surface temperature (°C) at an ocean cell: the latitude estimate plus
/// the local current anomaly. This CLOSES THE LOOP — the currents that were just
/// generated feed back into the SST the density/thermohaline coupling reads, rather
/// than density being driven by latitude alone.
#[inline]
fn cell_sst(buf: &WorldBuffer, x: u32, y: u32) -> f32 {
    let i = buf.idx(x, y);
    let speed = (buf.current_vx[i] * buf.current_vx[i] + buf.current_vy[i] * buf.current_vy[i]).sqrt();
    sst_estimate(buf.abs_latitude(y)) + current_sst_anomaly(buf.current_type[i], speed)
}

/// Persist the annual-mean SST field (latitude + current anomaly) for ocean cells so
/// it can be rendered and inspected. Call after the current tags are final (after
/// `advect_salinity_and_recouple`). Land stays 0.
pub fn compute_sst(buf: &mut WorldBuffer) {
    if buf.sst.is_empty() {
        return;
    }
    let (w, h) = (buf.width, buf.height);
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            buf.sst[i] = if buf.terrain[i] == 0 { cell_sst(buf, x, y) } else { 0.0 };
        }
    }
}

/// Surface water-density index (0..1, higher = denser) from stored salinity and the
/// current-influenced SST (`cell_sst`). Cold + salty = dense (sinks). Transient —
/// used by the current coupling, not persisted.
fn density_index(buf: &WorldBuffer) -> Vec<f32> {
    let n = buf.total();
    let mut d = vec![0.0f32; n];
    for y in 0..buf.height {
        for x in 0..buf.width {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 { continue; }
            let sst_norm = ((cell_sst(buf, x, y) + 2.0) / 30.0).clamp(0.0, 1.0);
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
/// drift further downstream â€” the overturning conveyor. Returns a multiplier on
/// the warm-advection reach so the caller can extend the Gulf-Streamâ†’Europe tag.
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

    // Per-cell density anomaly â†’ mild speed boost (denser â‡’ stronger flow).
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

/// Mean high-latitude (>50Â°) surface density â€” the strength of the overturning
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
/// the warm `current_type` tag across the basin (Gulf Stream â†’ North Atlantic
/// Drift). `reach_mult` scales how far the warmth carries. Factored out so it can
/// run both in `generate_ocean_currents` and again after salinity advection.
fn extend_warm_tag(buf: &mut WorldBuffer, reach_mult: f32) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let base_type = buf.current_type.clone();
    let mut warm_add = vec![false; n];
    // step_len 1.5 â†’ this is ~0.9Â·w CELLS of reach (plenty to cross any basin),
    // not 1.35Â·w. Capped down from 0.9 to bound pathological open-ocean traces
    // (the Pacific / Southern Ocean) that made the ocean step slow.
    let max_steps = (w as f32 * 0.6 * reach_mult) as usize;
    let step_len = 1.5f32;
    // Corridor half-width: the warm drift is tagged as a BAND, not a 1-cell trace,
    // so the cross-basin flow reads warm instead of leaving grey streamlines
    // threading between sparse warm rays (the user's "warm current becomes grey").
    // Widened so the broad cross-basin drift stays one coherent warm stream.
    let half_w = ((w as f32 / 150.0).round() as i32).clamp(1, 5);

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
                let lat = buf.latitude(yi as u32);
                let alat = lat.abs();
                if alat > 78.0 { break; }

                // STOP the warm tag once the flow has clearly BENT EQUATORWARD: the
                // warm poleward/cross limb ends here, and the equatorward RETURN
                // limb is a COLD current (handled by extend_cold_tag). poleward = up
                // in the N hemisphere. The gap this leaves at the bend is the
                // neutral transition the user wanted before warmth turns to cold.
                //
                // Previously this broke on ANY equatorward lean (>0.30Â·mag) above
                // 18Â°, which severed the warm tag at the subtropical gyre bend so the
                // cross-basin North Atlantic/Pacific Drift went grey before reaching
                // the far coast. Now the tropical return limb still cuts off, but the
                // broad mid-latitude eastward drift (which leans only gently) keeps
                // its warmth all the way across â€” only a STRONG, sustained reversal
                // (>0.60Â·mag) ends it at higher latitudes.
                let nh = lat >= 0.0;
                let equatorward_vy = if nh { vmy } else { -vmy }; // >0 = toward equator
                let break_eq = if alat < 35.0 {
                    equatorward_vy > 0.30 * mag && alat > 18.0
                } else {
                    equatorward_vy > 0.60 * mag
                };
                if break_eq { break; }

                let hx0 = vmx / mag;
                let hy0 = vmy / mag;
                // Tag the cell plus a perpendicular corridor.
                warm_add[i] = true;
                let (perp_x, perp_y) = (-hy0, hx0);
                for k in 1..=half_w {
                    for sgn in [-1.0f32, 1.0] {
                        let nx = (px + perp_x * k as f32 * sgn).floor() as i32;
                        let ny = (py + perp_y * k as f32 * sgn).floor() as i32;
                        if ny < 0 || ny >= h as i32 { continue; }
                        let ni = buf.idx(buf.wrap_x(nx), ny as u32);
                        if buf.terrain[ni] == 0 && base_type[ni] != 2 { warm_add[ni] = true; }
                    }
                }

                let mut hx = hx0;
                let mut hy = hy0;
                if alat > 32.0 {
                    let drift = ((alat - 32.0) / 18.0).clamp(0.0, 1.0);
                    hx += drift * 1.4;
                    hy *= 1.0 - 0.55 * drift;
                    let hl = (hx * hx + hy * hy).sqrt();
                    if hl > 0.001 { hx /= hl; hy /= hl; }
                }
                // Western-boundary SEPARATION: a warm boundary current hugging the
                // east coast of a continent (land a few cells to its WEST, âˆ’x)
                // must peel off and run EAST across the basin once it reaches the
                // separation latitude (~40Â°). Otherwise the warm tag rides the coast
                // all the way to the subpolar zone and warms Kamchatka / Labrador.
                // The cross-basin drift then arrives at the FAR coast from open
                // water (land to its EAST, not west), so this never fires there and
                // the Norwegian-Current-style poleward warming of west coasts is
                // preserved. Detect "land to the west" within a few cells.
                if alat > 42.0 {
                    let mut land_west = false;
                    for d in 1..=5 {
                        let wx = buf.wrap_x(xi - d);
                        if buf.terrain[buf.idx(wx, yi as u32)] != 0 { land_west = true; break; }
                    }
                    if land_west {
                        // Peel OFF the coast: cancel the poleward (coast-hugging)
                        // component and push east into the basin so the current
                        // separates instead of riding the coast to the subpolar
                        // zone (Kamchatka). It does NOT force a fixed heading â€”
                        // once off the coast (no land to the west) the natural
                        // field resumes and can still carry the warm drift poleward
                        // up the FAR (eastern) coast (the Norwegian Current â†’ warm
                        // Arctic approach), keeping that limb warm.
                        hx = hx.abs().max(0.5);
                        if nh { hy = hy.max(0.0); } else { hy = hy.min(0.0); }
                        let hl = (hx * hx + hy * hy).sqrt();
                        if hl > 0.001 { hx /= hl; hy /= hl; }
                    }
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

/// Cold return limb. A current is only COLD once it is flowing EQUATORWARD (the
/// user's rule: cold appears when the flow is already moving back toward the
/// equator). Seeds on the geometric cold currents (eastern-boundary / subpolar /
/// ACC) and advects the cold tag downstream along the flow as a BAND, mirroring
/// the warm corridor, so the cold current reads as one coherent stream rather than
/// snapping on instantly. Runs AFTER extend_warm_tag so it never overwrites warm.
fn extend_cold_tag(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let base_type = buf.current_type.clone();
    let mut cold_add = vec![false; n];
    // Shorter reach + a single-cell-wide band than the warm corridor: cold currents
    // were over-tagged (too much blue), and tropical water must stay grey.
    let max_steps = (w as f32 * 0.30) as usize;
    let step_len = 1.5f32;
    let half_w = 1i32;

    for sy in 0..h {
        for sx in 0..w {
            let si = (sy * w + sx) as usize;
            if base_type[si] != 2 { continue; } // seed on cold currents

            let mut px = sx as f32 + 0.5;
            let mut py = sy as f32 + 0.5;
            for _ in 0..max_steps {
                let xi = px.floor() as i32;
                let yi = py.floor() as i32;
                if yi < 0 || yi >= h as i32 { break; }
                let i = buf.idx(buf.wrap_x(xi), yi as u32);
                if buf.terrain[i] != 0 { break; }   // coast
                if base_type[i] == 1 { break; }      // ran into a warm current

                let vmx = buf.current_vx[i];
                let vmy = buf.current_vy[i];
                let mag = (vmx * vmx + vmy * vmy).sqrt();
                if mag < 0.05 { break; }
                let alat = buf.latitude(yi as u32).abs();
                // Tropical band stays NEUTRAL: a current only reads cold once it is
                // out of the tropics (~22Â°, just shy of the Tropic of Cancer). This
                // keeps the seas around India / SE Asia grey rather than letting a
                // cold tag advect down into the monsoon tropics.
                if alat < 22.0 { break; }

                cold_add[i] = true;
                let (hx, hy) = (vmx / mag, vmy / mag);
                let (perp_x, perp_y) = (-hy, hx);
                for k in 1..=half_w {
                    for sgn in [-1.0f32, 1.0] {
                        let nx = (px + perp_x * k as f32 * sgn).floor() as i32;
                        let ny = (py + perp_y * k as f32 * sgn).floor() as i32;
                        if ny < 0 || ny >= h as i32 { continue; }
                        let ni = buf.idx(buf.wrap_x(nx), ny as u32);
                        if buf.terrain[ni] == 0 && base_type[ni] != 1 { cold_add[ni] = true; }
                    }
                }
                px += hx * step_len;
                py += hy * step_len;
            }
        }
    }

    for i in 0..n {
        if cold_add[i] && buf.terrain[i] == 0 && buf.current_type[i] != 1 {
            buf.current_type[i] = 2;
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

    // Base Eâˆ’P salinity to relax toward (keeps the subtropical source salty).
    let base: Vec<f32> = buf.salinity.iter().map(|&s| s as f32).collect();
    let mut sal = base.clone();

    let dt = 3.3f32;       // cells advected per iteration (raised so fewer iters reach as far)
    let iters = 11;        // more iterations â†’ salt reaches further poleward
    // Ping-pong between two buffers instead of cloning the whole grid every
    // iteration (16 full-grid clones were a big chunk of the ocean step's cost).
    let mut src = sal.clone();
    for _ in 0..iters {
        std::mem::swap(&mut src, &mut sal); // read from `src`, write to `sal`
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                if buf.terrain[i] != 0 { continue; }
                // Back-trace upstream along the current and take that salinity
                // (semi-Lagrangian) â†’ salt is carried in the flow direction.
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
                // Weak relaxation to the Eâˆ’P base so the advected salty tongue
                // persists much further up-current into the fresh north before it
                // mixes back down (Gulf Stream salting the sub-polar North Atlantic).
                sal[i] = 0.90 * up + 0.10 * base[i];
            }
        }
    }
    // Warm-conveyor salt signature: where a warm boundary current pushes into the
    // fresher mid/high latitudes (current_type==1, abs_lat>32Â°), raise salinity
    // toward a salty subtropical-source value so the current reads as a visible
    // salty corridor threading the fresher surrounding water â€” exactly the "salty
    // water pulled into the northern latitudes by the current" the map should show.
    let warm_psu = psu_to_u8(36.8) as f32;
    for y in 0..h {
        if buf.abs_latitude(y) < 32.0 { continue; }
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 || buf.current_type[i] != 1 { continue; }
            let alat = buf.abs_latitude(y);
            // Stronger lift the further north the warm tongue has carried.
            let lift = ((alat - 32.0) / 28.0).clamp(0.0, 1.0);
            sal[i] = sal[i] * (1.0 - 0.45 * lift) + warm_psu * (0.45 * lift);
        }
    }
    for i in 0..n {
        if buf.terrain[i] == 0 {
            buf.salinity[i] = sal[i].clamp(0.0, 255.0) as u8;
        }
    }

    // Current-power boost from the salinity gradient (the imbalance that drives
    // the flow). Steep saltyâ†”fresh fronts strengthen the current there.
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

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Cold shelf seas â€” seasonal sea ice as a high-latitude "refrigerator"
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// Shallow, semi-enclosed seas at high latitude (Hudson Bay, the Baltic, the Sea
// of Okhotsk) freeze over each winter and behave as a COLD RESERVOIR rather than
// a maritime moderator. The ice caps the oceanâ†’air heat flux (so the sea gives
// NO winter warming â€” the maritime moderation switches off) and the cold melt
// pool chills the surrounding land into summer, dragging the local climate one or
// two KÃ¶ppen belts colder than latitude alone would give (the tundra fingers that
// reach south of Hudson Bay).
//
// This is a four-term competition â€” winter cooling demand vs. the heat available
// to resist it â€” NOT latitude+depth alone. A sea bathed by a warm current (the
// Norwegian shelf, ice-free at >70Â°N) does not freeze, while Labrador freezes at
// a lower latitude. The freeze index Î¦ âˆˆ [0,1] therefore is:
//
//   Î¦ = lat_gate                   net annual radiation deficit   smoothstep 45â†’62Â°
//     Â· 1[is_shelf]                low heat capacity C = ÏÂ·cpÂ·H    (shallow shelf)
//     Â· (1 âˆ’ warm)                 no advective heat import        (no warm current near)
//     Â· (0.55 + 0.45Â·brackish)     freezing point raised by low S  (Millero/UNESCO Tf)
//     Â· (0.45 + 0.55Â·enclosure)    poor exchange with open ocean   (BFS to open water)
//
// Science anchors: seawater freezing point Tf â‰ˆ âˆ’0.055Â·S (Millero/UNESCO, TEOS-10);
// sea-ice albedo 0.5â€“0.85 vs 0.06 open water and the ice insulation flux cut
// (Curry et al. 1995; Hill et al. 2019); the latent-heat reservoir ÏÂ·LfÂ·h that
// delays summer warming, measured as a >3 MJ mâ»Â³ Hudson Bay heat anomaly (Joly et
// al. 2010); NSIDC sea-ice background.

/// Latitude onset/saturation of the seasonal-ice regime.
const SEA_ICE_LAT_LO: f32 = 45.0;
const SEA_ICE_LAT_HI: f32 = 62.0;

/// Smoothstep S-curve (0 below `a`, 1 above `b`).
#[inline]
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Freezing point of seawater from stored salinity (linearized Millero/UNESCO,
/// surface pressure): Tf â‰ˆ âˆ’0.055Â·S. salinity is u8 over SAL_MIN..SAL_MAX PSU.
#[inline]
fn seawater_freezing_point(sal_u8: u8) -> f32 {
    let psu = SAL_MIN_PSU + (sal_u8 as f32 / 255.0) * (SAL_MAX_PSU - SAL_MIN_PSU);
    -0.055 * psu
}

/// True if a WARM current (type 1) runs within `r` cells â€” this keeps a shelf
/// ice-free despite high latitude (the Norwegian Current vs. Labrador contrast).
fn warm_current_near(buf: &WorldBuffer, x: u32, y: u32, r: i32) -> bool {
    if buf.current_type.is_empty() { return false; }
    for dy in -r..=r {
        let ny = y as i32 + dy;
        if ny < 0 || ny >= buf.height as i32 { continue; }
        for dx in -r..=r {
            let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
            if buf.terrain[ni] == 0 && buf.current_type[ni] == 1 { return true; }
        }
    }
    false
}

/// BFS distance (in cells, capped) from every ocean cell to the nearest OPEN
/// ocean â€” deep (non-shelf) water. An enclosed shelf basin is far from open water
/// â†’ poor heat exchange â†’ freezes harder.
fn open_ocean_distance(buf: &WorldBuffer, cap: u16) -> Vec<u16> {
    let (w, h, n) = (buf.width, buf.height, buf.total());
    let mut dist = vec![u16::MAX; n];
    let mut q = VecDeque::new();
    for i in 0..n {
        if buf.terrain[i] == 0 && buf.is_shelf[i] == 0 {
            dist[i] = 0;
            q.push_back(i);
        }
    }
    while let Some(i) = q.pop_front() {
        let d = dist[i];
        if d >= cap { continue; }
        let cx = (i % w as usize) as i32;
        let cy = (i / w as usize) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let ni = buf.idx(buf.wrap_x(cx + dx), ny as u32);
            if buf.terrain[ni] == 0 && dist[ni] > d + 1 {
                dist[ni] = d + 1;
                q.push_back(ni);
            }
        }
    }
    for v in dist.iter_mut() {
        if *v == u16::MAX { *v = cap; }
    }
    dist
}

/// Compute the seasonal-sea-ice freeze index Î¦ per ocean cell (see the section
/// header). Pure read; returns a world-sized f32 field (0 over land and over
/// ice-free water). Requires shelf + salinity + currents to have been computed.
pub fn compute_shelf_freeze(buf: &WorldBuffer) -> Vec<f32> {
    let (w, h, n) = (buf.width, buf.height, buf.total());
    // Defensive: without these columns loaded there is nothing to test.
    if buf.is_shelf.is_empty() || buf.salinity.is_empty() || buf.current_type.is_empty() {
        return vec![0.0; n];
    }
    let open = open_ocean_distance(buf, 16);
    let mut phi = vec![0.0f32; n];
    for y in 0..h {
        let lat_gate = smoothstep(SEA_ICE_LAT_LO, SEA_ICE_LAT_HI, buf.abs_latitude(y));
        if lat_gate <= 0.0 { continue; }
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 || buf.is_shelf[i] == 0 { continue; }
            if warm_current_near(buf, x, y, 3) { continue; } // Norwegian-shelf exception
            // Fresher water freezes at a warmer Tf â†’ freezes more readily. Map the
            // salinity-dependent Tf (âˆ’0.4 near-fresh .. âˆ’2.3 briny) to brackish 0..1.
            let tf = seawater_freezing_point(buf.salinity[i]);
            let brackish = ((tf + 2.3) / 1.9).clamp(0.0, 1.0);
            let encl = (open[i] as f32 / 12.0).clamp(0.0, 1.0);
            phi[i] = (lat_gate * (0.55 + 0.45 * brackish) * (0.45 + 0.55 * encl)).clamp(0.0, 1.0);
        }
    }
    phi
}

/// Ocean-current refinement from seasonal sea ice (brine rejection). A freezing
/// shelf sea ejects salt and forms cold, dense water that sinks and is exported
/// as a COLD current (Sea of Okhotsk â†’ East Sakhalin Current â†’ Oyashio; Labrador
/// shelf â†’ Labrador Current). We tag strongly-freezing shelf water as cold, then
/// trace a short cold plume downstream into the adjacent open ocean and give it a
/// small speed boost (the dense-water outflow). This feeds the EXISTING cold-
/// current climate machinery: the temperature current-influence pass, the
/// precipitation cold-coast/monsoon terms, and KÃ¶ppen. Call BEFORE
/// `compute_temperature` so those consumers see the new tags.
pub fn reinforce_cold_shelf_currents(buf: &mut WorldBuffer, freeze: &[f32]) {
    let (w, h, n) = (buf.width, buf.height, buf.total());
    if buf.current_type.is_empty() || freeze.len() != n { return; }

    // 1. The freezing shelf water itself reads as ice-margin cold water.
    for i in 0..n {
        if buf.terrain[i] == 0 && freeze[i] > 0.35 && buf.current_type[i] != 1 {
            buf.current_type[i] = 2;
        }
    }

    // 2. Brine-rejection export: from strongly-freezing shelf cells that border
    //    OPEN (non-shelf) water, trace a short downstream plume, cold-tag it, and
    //    accelerate the dense-water outflow. Bounded reach keeps it marginal.
    let base_freeze = freeze.to_vec();
    const MAX_STEPS: usize = 10;
    const STEP: f32 = 1.5;
    const OUTFLOW_BOOST: f32 = 1.12;
    for sy in 0..h {
        for sx in 0..w {
            let si = (sy * w + sx) as usize;
            if base_freeze[si] <= 0.5 { continue; }
            let borders_open = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                let ny = sy as i32 + dy;
                if ny < 0 || ny >= h as i32 { return false; }
                let ni = buf.idx(buf.wrap_x(sx as i32 + dx), ny as u32);
                buf.terrain[ni] == 0 && buf.is_shelf[ni] == 0
            });
            if !borders_open { continue; }

            let mut px = sx as f32 + 0.5;
            let mut py = sy as f32 + 0.5;
            for _ in 0..MAX_STEPS {
                let (xi, yi) = (px.floor() as i32, py.floor() as i32);
                if yi < 0 || yi >= h as i32 { break; }
                let i = buf.idx(buf.wrap_x(xi), yi as u32);
                if buf.terrain[i] != 0 { break; }      // reached the coast
                if buf.current_type[i] == 1 { break; } // don't overwrite warm water
                buf.current_type[i] = 2;
                let (vmx, vmy) = (buf.current_vx[i], buf.current_vy[i]);
                let mag = (vmx * vmx + vmy * vmy).sqrt();
                if mag < 0.05 { break; }
                buf.current_vx[i] *= OUTFLOW_BOOST;
                buf.current_vy[i] *= OUTFLOW_BOOST;
                px += vmx / mag * STEP;
                py += vmy / mag * STEP;
            }
        }
    }
}

/// Apply the seasonal-sea-ice cooling onto the temperature field: drag the frozen
/// water's own annual mean toward its freezing point (ice + cold pool), and chill
/// the adjacent land â€” the "refrigerator". The land term is accumulated then
/// clamped (the upwelling pattern) so a shore ringed by ice on several sides can't
/// stack an unphysical deep-freeze. Call AFTER `compute_temperature`, which
/// rewrites the temperature field from scratch (an earlier cooling would be lost).
pub fn apply_cold_shelf_cooling(buf: &mut WorldBuffer, freeze: &[f32]) {
    let (w, h, n) = (buf.width, buf.height, buf.total());
    if freeze.len() != n { return; }

    // (1) The frozen water: pull its mean toward Tf (albedo + insulation flip).
    for i in 0..n {
        if buf.terrain[i] == 0 && freeze[i] > 0.0 {
            let tf = seawater_freezing_point(buf.salinity[i]);
            let t = buf.temperature[i];
            if t > tf { buf.temperature[i] = t + (tf - t) * 0.75 * freeze[i]; }
        }
    }

    // (2) Chill the adjacent land. Reach ~3 cells, Gaussian falloff, clamped so a
    //     fully ice-ringed shore reaches ~MAX_COOL (calibrated to the ~one-KÃ¶ppen-
    //     belt / Joly-2010 Hudson Bay coastal cooling).
    const REACH: i32 = 3;
    const MAX_COOL: f32 = 8.0;
    const SIGMA2: f32 = 4.0;
    let kernel = |dx: i32, dy: i32| (-(((dx * dx + dy * dy) as f32) / (2.0 * SIGMA2))).exp();
    let mut norm = 0.0f32;
    for dy in -REACH..=REACH {
        for dx in -REACH..=REACH {
            if dx == 0 && dy == 0 { continue; }
            norm += kernel(dx, dy);
        }
    }
    let scale = MAX_COOL / norm.max(1e-6);

    let mut delta = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 || freeze[i] <= 0.0 { continue; }
            let src = freeze[i];
            for dy in -REACH..=REACH {
                let ny = buf.clamp_y(y as i32 + dy);
                for dx in -REACH..=REACH {
                    let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny);
                    if buf.terrain[ni] != 1 { continue; }
                    delta[ni] -= src * kernel(dx, dy) * scale;
                }
            }
        }
    }
    for i in 0..n {
        if buf.terrain[i] == 1 {
            buf.temperature[i] += delta[i].max(-MAX_COOL);
        }
    }
}

/// Compute upwelling zones: shelf + cold current + equatorward flow + eastern boundary.
pub fn compute_upwelling_zones(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    // Per-source cooling kept small and accumulated into a delta buffer that is
    // clamped before being applied. Previously each upwelling ocean cell wrote
    // -6Â°C directly onto every land cell in its 5Ã—5 footprint, so a coastal cell
    // overlapped by several upwelling sources stacked -12/-18Â°C and froze into
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::ColumnSet;

    /// The wind-stress-curl Sverdrup interior reverses sense between the subtropical
    /// and subpolar gyres, and is antisymmetric across the equator — emergent from
    /// the belt-wind shear, not hand-coded.
    #[test]
    fn sverdrup_interior_reverses_between_gyres() {
        // NH subtropical interior flows EQUATORWARD (southward → v_north < 0);
        // NH subpolar interior flows POLEWARD (northward → v_north > 0).
        let c = Circulation::earth();
        assert!(sverdrup_interior_north(25.0, &c) < 0.0, "NH subtropical interior should be equatorward");
        assert!(sverdrup_interior_north(55.0, &c) > 0.0, "NH subpolar interior should be poleward");
        // SH mirror (northward = equatorward there; southward = poleward).
        assert!(sverdrup_interior_north(-25.0, &c) > 0.0, "SH subtropical interior should be equatorward");
        assert!(sverdrup_interior_north(-55.0, &c) < 0.0, "SH subpolar interior should be poleward");
    }

    /// Build a 16Ã—12 all-land test world. With equator_offset 0.5 and unit lat
    /// scaling, abs_lat(row) = 15Â·|6âˆ’row|, so row 2 = 60Â°N, row 6 = equator.
    fn land_world() -> WorldBuffer {
        let (w, h) = (16u32, 12u32);
        let n = (w * h) as usize;
        WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: vec![1u8; n], elevation: vec![0.1; n], sea_depth: vec![0.0; n],
            is_shelf: vec![0u8; n], is_shelf_edge: vec![0u8; n], locked_bits: Vec::new(),
            plate_index: Vec::new(), boundary_type: Vec::new(), is_volcanic: Vec::new(),
            temperature: vec![5.0f32; n], precipitation: Vec::new(), koppen: Vec::new(),
            soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: vec![0u8; n], wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: vec![0.0f32; n], current_vy: vec![0.0f32; n],
            distance_to_ocean: Vec::new(), habitability: Vec::new(),
            salinity: vec![128u8; n], shark_risk: Vec::new(), goods: Vec::new(),
            shipworm_risk: Vec::new(), storm_base: Vec::new(), reef_risk: Vec::new(),
            disease_risk: Vec::new(), precip_summer_frac: Vec::new(), seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(),
        }
    }

    /// Turn a cell into an enclosed brackish shelf-sea cell (Hudson-Bay-like).
    fn make_cold_sea(buf: &mut WorldBuffer, x: u32, y: u32) {
        let i = buf.idx(x, y);
        buf.terrain[i] = 0;
        buf.is_shelf[i] = 1;
        buf.salinity[i] = 40; // ~30 PSU, brackish â†’ freezes readily
    }

    #[test]
    fn shelf_freeze_index_gates_on_latitude_shelf_and_warm_current() {
        let mut buf = land_world();
        // Enclosed cold shelf sea at 60Â°N (row 2), all-land surroundings.
        make_cold_sea(&mut buf, 5, 2);
        make_cold_sea(&mut buf, 6, 2);
        // A high-lat shelf sea bathed by a warm current (Norwegian-shelf analogue).
        make_cold_sea(&mut buf, 10, 2);
        make_cold_sea(&mut buf, 11, 2);
        let warm_idx = buf.idx(11, 2);
        buf.current_type[warm_idx] = 1; // warm current on the pair
        // A tropical shelf sea (row 6, equator) â€” latitude gate should zero it.
        make_cold_sea(&mut buf, 5, 6);

        let phi = compute_shelf_freeze(&buf);

        assert!(phi[buf.idx(5, 2)] > 0.35, "enclosed high-lat brackish shelf freezes");
        assert_eq!(phi[buf.idx(10, 2)], 0.0, "warm current keeps the shelf ice-free");
        assert_eq!(phi[buf.idx(11, 2)], 0.0, "warm current cell itself never freezes");
        assert_eq!(phi[buf.idx(5, 6)], 0.0, "tropical shelf never freezes");
    }

    #[test]
    fn cold_shelf_refrigerates_adjacent_land_and_tags_currents() {
        let mut buf = land_world();
        // A sizeable enclosed high-latitude basin (rows 1-2 = 75Â°/60Â°N), like the
        // real Hudson Bay â€” big enough that its shore is chilled from several sides.
        for y in 1..=2 {
            for x in 3..=7 {
                make_cold_sea(&mut buf, x, y);
            }
        }

        let phi = compute_shelf_freeze(&buf);
        reinforce_cold_shelf_currents(&mut buf, &phi);
        // The freezing water reads as a cold current (brine-rejection tag).
        assert_eq!(buf.current_type[buf.idx(5, 2)], 2, "frozen shelf tagged cold");

        apply_cold_shelf_cooling(&mut buf, &phi);
        // The water's own annual mean is dragged well below the 5Â°C start toward
        // its freezing point (not all the way â€” it's a seasonal, ~0.75Â·Î¦ pull).
        assert!(buf.temperature[buf.idx(5, 2)] < 3.0, "frozen sea mean cooled: {}", buf.temperature[buf.idx(5, 2)]);
        // Land on the ice-facing shore (row 3, below the basin) is chilled well
        // below the 5Â°C baseline (the refrigerator reaching inland).
        let shore = buf.idx(5, 3);
        assert!(buf.temperature[shore] < 4.0, "shore cooled: {}", buf.temperature[shore]);
        // Far-field land (the opposite side of the world) is untouched.
        let far = buf.idx(13, 9);
        assert_eq!(buf.temperature[far], 5.0, "distant land unaffected");
    }
}


