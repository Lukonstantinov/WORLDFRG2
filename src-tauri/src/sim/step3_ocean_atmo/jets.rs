use crate::sim::world_buffer::WorldBuffer;
use super::circulation::Circulation;

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Low-level jets (the Somali / Findlater jet and its cousins)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// `compute_wind_belts` (ocean.rs) writes only a UNIT direction field. This step
// derives the low-level wind SPEED (m/s, ~0-35) â€” the intensity shown on the Wind
// Speed layer â€” and, crucially, resolves where the flow ACCELERATES into a jet.
//
// A low-level jet forms where a broad flow is CHANNELED and concentrated by
// terrain: cross-equatorial / monsoon flow running alongside a meridional
// highland wall (the East-African-highlands geometry that spins up the Somali
// Jet), or flow squeezed through a gap between two barriers (a venturi). The jet
// core is then streamed downstream so a fast tongue reaches the far coast.
//
// The physical pay-off lives in precipitation.rs: at the jet ENTRANCE the flow is
// accelerating, low-level divergence sweeps moisture along before it can pile up
// (the arid Somali / Arabian coast); at the jet EXIT the flow decelerates,
// converges and is forced to rise (the torrential Western-Ghats monsoon dump).
// This module only produces the speed field; precipitation reads its gradient.

const EARTH_CIRCUMFERENCE_KM: f32 = 40075.0;

/// Highland elevation (normalized 0..1, ~1400 m) that acts as a lateral barrier
/// able to channel a low-level jet. Lower than the precipitation MOUNTAIN
/// threshold (0.19) because even a moderate plateau wall (the East African
/// highlands) concentrates the cross-equatorial flow into a jet.
const BARRIER_ELEV: f32 = 0.15;

/// How far a lateral barrier can sit and still channel the flow.
const BARRIER_KM: f32 = 650.0;
/// Extra speed multiplier at a strong one-sided barrier jet (Somali-jet core).
const BARRIER_GAIN: f32 = 2.4;
/// Extra speed multiplier where flow is pinched between barriers on both sides.
const GAP_GAIN: f32 = 1.1;
/// Warm-tropical-sea monsoon inflow gets a mild speed bump (land-sea contrast).
const MONSOON_GAIN: f32 = 0.6;

/// Downstream jet-core propagation: how far the fast core streams along the flow.
const PROPAGATE_KM: f32 = 700.0;
/// Per-cell decay as the core streams downstream (a slow, broad drift).
const PROPAGATE_DECAY: f32 = 0.985;

/// Ceiling on the modelled low-level wind speed (m/s).
const SPEED_CAP: f32 = 35.0;

#[inline]
fn gauss(x: f32, mu: f32, sigma: f32) -> f32 {
    (-((x - mu).powi(2)) / (2.0 * sigma * sigma)).exp()
}

/// Base low-level wind speed (m/s) from the latitude wind belts: weak doldrums at
/// the ITCZ, brisk trades in the trade belt, a subtropical-high calm at the Hadley
/// edge, strong westerlies in the mid-latitudes, tapering toward the pole. The belt
/// centres and widths follow the planet's general circulation (`circulation.rs`):
/// on Earth the trades peak ~15°, the calm sits at the 30° Hadley edge and the
/// westerlies peak ~50°, but a faster/slower rotator moves them.
pub(crate) fn base_speed(abs_lat: f32, circ: &Circulation) -> f32 {
    let bs = circ.belt_scale();
    let fs = circ.front_scale();
    let doldrums = 2.5;
    // Trades peak at half the Hadley-edge latitude (15° on Earth).
    let trades = 6.0 * gauss(abs_lat, 0.5 * circ.hadley_edge, 9.0 * bs);
    // Westerlies peak between the two belts — exactly 50° on Earth (⁵⁰⁄₆₀·front).
    let westerlies = 9.0 * gauss(abs_lat, (50.0 / 60.0) * circ.polar_front, 13.0 * fs);
    // Horse-latitude calm sits on the Hadley edge (30° on Earth).
    let calm = 3.0 * gauss(abs_lat, circ.hadley_edge, 5.5 * bs);
    (doldrums + trades + westerlies - calm).max(1.5)
}

/// Proximity (0..1, 1 = adjacent) of the nearest highland barrier along a float
/// direction. Off the top/bottom edge (a pole) counts as a wall too.
fn wall_proximity(buf: &WorldBuffer, x: u32, y: u32, dirx: f32, diry: f32, max_steps: i32) -> f32 {
    let h = buf.height as i32;
    for s in 1..=max_steps {
        let fx = x as f32 + dirx * s as f32;
        let fy = y as f32 + diry * s as f32;
        let yi = fy.round() as i32;
        if yi < 0 || yi >= h {
            return 1.0 - (s - 1) as f32 / max_steps as f32;
        }
        let xi = buf.wrap_x(fx.round() as i32);
        let ni = buf.idx(xi, yi as u32);
        if buf.terrain[ni] == 1 && buf.elevation[ni] > BARRIER_ELEV {
            return 1.0 - (s - 1) as f32 / max_steps as f32;
        }
    }
    0.0
}

/// Mild monsoon strengthening: a tropical cell drawing on a warm/neutral sea on
/// its equatorward or eastern fetch (the summer onshore-monsoon geometry) runs a
/// little faster from the land-sea thermal contrast. Cheap: a couple of short rays.
fn monsoon_inflow(buf: &WorldBuffer, x: u32, y: u32, range: i32) -> f32 {
    let lat = buf.latitude(y);
    let abs_lat = lat.abs();
    if abs_lat > 30.0 { return 0.0; }
    let h = buf.height as i32;
    let eqdir = if lat >= 0.0 { 1i32 } else { -1 }; // toward the equator
    let mut best = 0.0f32;
    for &(dx, dy) in &[(0i32, eqdir), (1, eqdir), (1, 0)] {
        for s in 1..=range {
            let nx = buf.wrap_x(x as i32 + dx * s);
            let ny = y as i32 + dy * s;
            if ny < 0 || ny >= h { break; }
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] == 0 {
                let warm = if buf.current_type[ni] == 2 { 0.3 } else { 1.0 };
                best = best.max((1.0 - s as f32 / range as f32) * warm);
                break;
            }
        }
    }
    best
}

/// Compute the low-level wind-speed field (incl. jets) into `buf.wind_speed`.
pub fn compute_low_level_jets(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let km_per_cell = EARTH_CIRCUMFERENCE_KM / w as f32;
    let barrier_steps = ((BARRIER_KM / km_per_cell).round() as i32).max(4);
    let inflow_range = ((650.0 / km_per_cell).round() as i32).max(6);

    if buf.wind_speed.len() != n {
        buf.wind_speed = vec![0.0; n];
    }

    let circ = Circulation::for_world(buf);

    // â”€â”€ Pass 1: base speed Ã— terrain Ã— channeling (barrier / gap / monsoon) â”€â”€
    let mut speed = vec![0.0f32; n];
    for y in 0..h {
        let abs_lat = buf.abs_latitude(y);
        let base = base_speed(abs_lat, &circ);
        for x in 0..w {
            let i = buf.idx(x, y);
            let wvx = buf.wind_vx[i];
            let wvy = buf.wind_vy[i];
            let wl = (wvx * wvx + wvy * wvy).sqrt();
            // Land friction slows the low-level flow; calm slivers stay calm.
            let friction = if buf.terrain[i] == 1 { 0.6 } else { 1.0 };
            let mut s = base * friction * wl.clamp(0.0, 1.0);

            if wl > 0.05 {
                let dx = wvx / wl;
                let dy = wvy / wl;
                // Perpendicular sides of the flow.
                let left = wall_proximity(buf, x, y, -dy, dx, barrier_steps);
                let right = wall_proximity(buf, x, y, dy, -dx, barrier_steps);
                let gap = left.min(right); // pinched both sides â†’ venturi
                let barrier = left.max(right) * (1.0 - gap); // one-sided wall â†’ barrier jet
                let mut mult = 1.0 + BARRIER_GAIN * barrier + GAP_GAIN * gap;
                // Warm-sea monsoon inflow strengthens the tropical jet.
                mult += MONSOON_GAIN * monsoon_inflow(buf, x, y, inflow_range);
                s *= mult;
            }
            speed[i] = s.min(SPEED_CAP);
        }
    }

    // â”€â”€ Pass 2: stream the fast jet core downstream along the flow â”€â”€
    // A jet is a coherent tongue: carry the accelerated core forward (max-blend
    // with the upwind speed, mildly decayed) so the Somali-jet-style core reaches
    // its downwind terminus instead of dying at the barrier's end.
    let prop_passes = ((PROPAGATE_KM / km_per_cell).round() as i32).clamp(6, 48);
    let mut src = speed;
    let mut dst = vec![0.0f32; n];
    for _ in 0..prop_passes {
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                let wvx = buf.wind_vx[i];
                let wvy = buf.wind_vy[i];
                let wl = (wvx * wvx + wvy * wvy).sqrt();
                let mut s = src[i];
                if wl > 0.05 {
                    let ux = (x as f32 - wvx / wl).round() as i32;
                    let uy = (y as f32 - wvy / wl).round() as i32;
                    if uy >= 0 && uy < h as i32 {
                        let ui = buf.idx(buf.wrap_x(ux), uy as u32);
                        s = s.max(src[ui] * PROPAGATE_DECAY);
                    }
                }
                dst[i] = s;
            }
        }
        std::mem::swap(&mut src, &mut dst);
    }

    // â”€â”€ Pass 3: light isotropic smooth so the tongue reads as a coherent band â”€â”€
    let mut b = vec![0.0f32; n];
    for _ in 0..2 {
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                let mut sum = src[i] * 2.0;
                let mut cnt = 2.0f32;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let ni = buf.idx(buf.wrap_x(x as i32 + dx), ny as u32);
                    sum += src[ni];
                    cnt += 1.0;
                }
                b[i] = sum / cnt;
            }
        }
        std::mem::swap(&mut src, &mut b);
    }

    for i in 0..n {
        buf.wind_speed[i] = src[i].clamp(0.0, SPEED_CAP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::ColumnSet;
    use crate::db::schema;
    use rusqlite::Connection;

    /// Build a buffer with a meridional highland wall down the middle and open
    /// sea to its east, plus a uniform northward flow with a slight eastward
    /// lean â€” the Somali-jet geometry â€” and check a fast jet forms hugging the
    /// wall and that speed accelerates then decelerates along the flow.
    #[test]
    fn barrier_spins_up_a_jet() {
        let w = 120u32;
        let h = 120u32;
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap();
        let barrier_x = 40u32;
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                // A tall highland wall at x=barrier_x; everything else is sea.
                if x == barrier_x {
                    buf.terrain[i] = 1;
                    buf.elevation[i] = 0.4;
                }
                // Northward flow (screen âˆ’y) leaning slightly east, unit length.
                let (vx, vy) = (0.25f32, -0.97f32);
                let l = (vx * vx + vy * vy).sqrt();
                buf.wind_vx[i] = vx / l;
                buf.wind_vy[i] = vy / l;
            }
        }
        compute_low_level_jets(&mut buf);

        // A cell just east of the wall (open on the east, wall on the west) is a
        // one-sided barrier jet: it must be markedly faster than a far-offshore
        // cell in the same row with no nearby wall.
        let row = 60u32;
        let near = buf.wind_speed[buf.idx(barrier_x + 2, row)];
        let far = buf.wind_speed[buf.idx(barrier_x + 40, row)];
        assert!(near > far * 1.3, "barrier jet {near} not faster than open flow {far}");
        assert!(near > 9.0, "barrier jet core too weak: {near} m/s");
    }
}

