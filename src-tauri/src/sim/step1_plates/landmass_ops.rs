//! Freehand area tools for the Landmass step (`ITCZ_AND_LAND_TOOLS_PLAN.md`
//! Commit 1). A `Lasso` is a user-drawn polygon in world-cell coordinates; the
//! four ops here (`smooth_roughen`, `fjords`, `island_chain`, `fill`) all mutate
//! `terrain`/`elevation`/`is_volcanic` only within it. Each op loads
//! `ColumnSet::PHASE_PLATES` and the caller calls `buf.save`, which already
//! pushes exactly one `undo_journal` entry — so every op here is undoable and
//! re-rollable for free, no new history code (see the command wrappers in
//! `commands/sim_commands.rs`).

use crate::sim::world_buffer::WorldBuffer;
use crate::sim::step2_terrain::elevation::fbm_noise;

const KM_EQUATOR: f32 = 40075.0;
/// Cells over which every op fades to nothing at the lasso boundary (rule 2).
/// Kept in CELLS, not km — a lasso is drawn at whatever zoom the user is at,
/// so its own size already carries the world's resolution.
const FEATHER_CELLS: f32 = 5.0;

/// A freehand selection polygon in world-cell coordinates (rule 6: X wraps).
/// Vertices arrive from the frontend possibly straddling the antimeridian — a
/// polygon drawn across the seam has points on both edges of the array. A
/// naive point-in-polygon test over the raw coordinates selects the
/// *complement* of what the user actually circled, because the ray-casting
/// winding no longer closes correctly once the shape has an artificial seam
/// cut through it. `Lasso::new` re-expresses every vertex in one continuous,
/// unwrapped frame anchored on the first point; `contains`/`signed_dist` then
/// re-test at `x`, `x-w` and `x+w` so a query cell hits whichever wrapped copy
/// of the (now non-wrapping) polygon it actually falls inside.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Lasso {
    pub points: Vec<(f32, f32)>,
}

impl Lasso {
    pub fn new(raw_points: Vec<(f32, f32)>, world_w: u32) -> Self {
        let w = world_w as f32;
        if raw_points.is_empty() {
            return Lasso { points: raw_points };
        }
        let mut points = Vec::with_capacity(raw_points.len());
        let mut prev = raw_points[0];
        points.push(prev);
        for &(x, y) in raw_points.iter().skip(1) {
            // Unwrap: pick whichever of x, x-w, x+w sits closest to the
            // previous (already-unwrapped) vertex, so a seam crossing becomes
            // a continuous coordinate instead of a jump back to 0.
            let mut best = x;
            let mut best_d = (x - prev.0).abs();
            for cand in [x - w, x + w] {
                let d = (cand - prev.0).abs();
                if d < best_d {
                    best = cand;
                    best_d = d;
                }
            }
            prev = (best, y);
            points.push(prev);
        }
        Lasso { points }
    }

    fn bounds(&self) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(x, y) in &self.points {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        (min_x, min_y, max_x, max_y)
    }

    fn contains_raw(&self, x: f32, y: f32) -> bool {
        let n = self.points.len();
        if n < 3 {
            return false;
        }
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = self.points[i];
            let (xj, yj) = self.points[j];
            if (yi > y) != (yj > y) {
                let x_cross = (xj - xi) * (y - yi) / (yj - yi) + xi;
                if x < x_cross {
                    inside = !inside;
                }
            }
            j = i;
        }
        inside
    }

    fn dist_to_boundary_raw(&self, x: f32, y: f32) -> f32 {
        let n = self.points.len();
        if n < 2 {
            return f32::MAX;
        }
        let mut best = f32::MAX;
        let mut j = n - 1;
        for i in 0..n {
            let (x1, y1) = self.points[j];
            let (x2, y2) = self.points[i];
            let d = point_segment_dist(x, y, x1, y1, x2, y2);
            if d < best {
                best = d;
            }
            j = i;
        }
        best
    }

    /// True if world cell `(x, y)` falls inside the polygon, trying every
    /// wrapped copy of the (unwrapped) shape.
    pub fn contains(&self, x: f32, y: f32, world_w: u32) -> bool {
        let w = world_w as f32;
        [0.0, -w, w].iter().any(|&shift| self.contains_raw(x + shift, y))
    }

    /// Signed distance in cells to the polygon boundary — positive inside,
    /// negative outside. The feather kernel every op reads (rule 2).
    pub fn signed_dist(&self, x: f32, y: f32, world_w: u32) -> f32 {
        let w = world_w as f32;
        let mut best = f32::MAX;
        let mut best_inside = false;
        for &shift in &[0.0, -w, w] {
            let px = x + shift;
            let d = self.dist_to_boundary_raw(px, y);
            if d < best {
                best = d;
                best_inside = self.contains_raw(px, y);
            }
        }
        if best_inside {
            best
        } else {
            -best
        }
    }

    /// A soft 0..1 membership: 1 well inside, 0 well outside, a smooth ramp
    /// across `FEATHER_CELLS` at the boundary. This is what makes every op
    /// fade out rather than printing the lasso gesture as a hard edge.
    pub fn blend(&self, x: f32, y: f32, world_w: u32) -> f32 {
        let d = self.signed_dist(x, y, world_w) / FEATHER_CELLS;
        smoothstep((d + 1.0) * 0.5)
    }

    /// Every `(world x, world y)` cell within `pad` cells of the polygon,
    /// scanning only its own bounding box — never the whole world (rule 4).
    pub fn candidate_cells(&self, world_w: u32, world_h: u32, pad: f32) -> Vec<(u32, u32)> {
        let (min_x, min_y, max_x, max_y) = self.bounds();
        if min_x > max_x {
            return Vec::new();
        }
        let y0 = (min_y - pad).floor().max(0.0) as i32;
        let y1 = (max_y + pad).ceil().min(world_h as f32 - 1.0) as i32;
        let span = (max_x - min_x + pad * 2.0).ceil() as i32;
        let mut out = Vec::new();
        if span >= world_w as i32 {
            for y in y0..=y1 {
                for x in 0..world_w as i32 {
                    out.push((x as u32, y as u32));
                }
            }
            return out;
        }
        let x0 = (min_x - pad).floor() as i32;
        let x1 = (max_x + pad).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let wx = (((x % world_w as i32) + world_w as i32) % world_w as i32) as u32;
                out.push((wx, y as u32));
            }
        }
        out
    }

    fn centroid(&self) -> (f32, f32) {
        let n = self.points.len().max(1) as f32;
        let (sx, sy) = self.points.iter().fold((0.0, 0.0), |(sx, sy), &(x, y)| (sx + x, sy + y));
        (sx / n, sy / n)
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn point_segment_dist(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 1e-6 {
        (((px - x1) * dx + (py - y1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cx = x1 + dx * t;
    let cy = y1 + dy * t;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Deterministic per-cell hash in 0..1, used to decide whether a feathered
/// cell flips — a soft edge that is still bit-reproducible for a given seed,
/// rather than an RNG draw that would make the same lasso op non-reproducible
/// on re-roll with the same seed.
fn cell_hash(x: i32, y: i32, seed: u64) -> f32 {
    let mut h = seed as u32;
    h ^= (x as u32).wrapping_mul(0x9E3779B1);
    h ^= (y as u32).wrapping_mul(0x85EBCA77);
    h = h.wrapping_mul(0xC2B2AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x27D4EB2F);
    h ^= h >> 15;
    (h as f32) / (u32::MAX as f32)
}

/// A feathered decision: true if this cell should take the op's new value,
/// given a soft membership `blend` — a coin toss weighted by `blend`, drawn
/// from a hash of the cell + seed so the same op on the same seed always
/// feathers the same cells (never a per-run RNG draw).
fn feathered_take(x: u32, y: u32, blend: f32, seed: u64) -> bool {
    blend > 0.0 && cell_hash(x as i32, y as i32, seed) < blend
}

// ── 1. Smooth / roughen ─────────────────────────────────────────────────

/// One bipolar control: negative smooths the coastline (repeated majority
/// filter over a disc), positive roughens it. `amount` in -1..1.
///
/// Roughening is a LEVEL SET (rule 3), never a per-cell dice roll: a signed
/// distance-to-coast field, local to the lasso's own bounding box, perturbed
/// by fbm and re-thresholded at zero. A per-cell threshold on raw noise
/// scatters speckle islands across deep ocean by construction — a level set
/// can only ever move the coastline that is already there, bounded by
/// `reach` cells either side of it.
pub fn smooth_roughen(buf: &mut WorldBuffer, lasso: &Lasso, amount: f32, seed: u64) {
    let amount = amount.clamp(-1.0, 1.0);
    if amount == 0.0 {
        return;
    }
    let w = buf.width;
    let h = buf.height;
    let pad = FEATHER_CELLS + 24.0;
    let cells = lasso.candidate_cells(w, h, pad);
    if cells.is_empty() {
        return;
    }

    if amount < 0.0 {
        let strength = -amount;
        let radius = (1.0 + strength * 3.0).round() as i32;
        let passes = (1.0 + strength * 2.0).round() as usize;
        let mut terrain = buf.terrain.clone();
        for _pass in 0..passes {
            let snapshot = terrain.clone();
            for &(x, y) in &cells {
                let blend = lasso.blend(x as f32, y as f32, w) * strength;
                if !feathered_take(x, y, blend, seed.wrapping_add(_pass as u64)) {
                    continue;
                }
                let mut land = 0i32;
                let mut total = 0i32;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx * dx + dy * dy > radius * radius {
                            continue;
                        }
                        let sx = buf.wrap_x(x as i32 + dx);
                        let sy_i = y as i32 + dy;
                        if sy_i < 0 || sy_i >= h as i32 {
                            continue;
                        }
                        let si = buf.idx(sx, sy_i as u32);
                        total += 1;
                        if snapshot[si] == 1 {
                            land += 1;
                        }
                    }
                }
                if total > 0 {
                    let majority = if land * 2 >= total { 1u8 } else { 0u8 };
                    let i = buf.idx(x, y);
                    terrain[i] = majority;
                }
            }
        }
        apply_terrain(buf, &terrain, &cells);
    } else {
        let reach = 6.0 + amount * 18.0;
        let coast = local_coast_field(buf, &cells, w, h, reach + 4.0);
        let mut terrain = buf.terrain.clone();
        for &(x, y) in &cells {
            let blend = lasso.blend(x as f32, y as f32, w);
            if blend <= 0.0 {
                continue;
            }
            let Some(&signed) = coast.get(&(x, y)) else { continue };
            if signed.abs() > reach {
                continue; // outside the bounded reach — level set leaves it alone
            }
            let n = fbm_noise(x as f32 / 9.0, y as f32 / 9.0, seed, 4, 2.05, 0.55) - 0.5;
            let perturbed = signed + n * reach * amount * 1.6;
            let new_land = if perturbed >= 0.0 { 1u8 } else { 0u8 };
            if !feathered_take(x, y, blend, seed.wrapping_add(9001)) {
                continue;
            }
            let i = buf.idx(x, y);
            terrain[i] = new_land;
        }
        apply_terrain(buf, &terrain, &cells);
    }
}

fn apply_terrain(buf: &mut WorldBuffer, new_terrain: &[u8], cells: &[(u32, u32)]) {
    for &(x, y) in cells {
        let i = buf.idx(x, y);
        if buf.terrain[i] != new_terrain[i] {
            let became_sea = new_terrain[i] == 0 && buf.terrain[i] == 1;
            buf.terrain[i] = new_terrain[i];
            if became_sea {
                buf.elevation[i] = 0.0;
                buf.is_volcanic[i] = 0;
            } else if new_terrain[i] == 1 && buf.elevation[i] <= 0.0 {
                buf.elevation[i] = 0.08;
            }
        }
    }
}

/// Multi-source BFS signed distance-to-coast, LOCAL to the given cell list's
/// bounding box (never a whole-world scan — rule 4 / §8.9 rule 1). Positive on
/// land, negative at sea. Cells whose true nearest coast lies outside the
/// scanned margin are simply left at +-`margin` (a saturated bound, not a
/// wrong one — the level set only ever acts within `reach < margin`).
fn local_coast_field(
    buf: &WorldBuffer,
    cells: &[(u32, u32)],
    world_w: u32,
    world_h: u32,
    margin: f32,
) -> std::collections::HashMap<(u32, u32), f32> {
    use std::collections::{HashMap, VecDeque};
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    for &(x, y) in cells {
        min_x = min_x.min(x as i32);
        max_x = max_x.max(x as i32);
        min_y = min_y.min(y as i32);
        max_y = max_y.max(y as i32);
    }
    if min_x > max_x {
        return HashMap::new();
    }
    let pad = margin.ceil() as i32;
    let y0 = (min_y - pad).max(0);
    let y1 = (max_y + pad).min(world_h as i32 - 1);
    let x0 = min_x - pad;
    let x1 = max_x + pad;
    let bw = (x1 - x0 + 1) as usize;
    let bh = (y1 - y0 + 1) as usize;
    if bw == 0 || bh == 0 {
        return HashMap::new();
    }
    let local_idx = |gx: i32, gy: i32| -> Option<usize> {
        if gy < y0 || gy > y1 {
            return None;
        }
        let lx = gx - x0;
        let ly = gy - y0;
        if lx < 0 || lx as usize >= bw {
            return None;
        }
        Some(ly as usize * bw + lx as usize)
    };
    let mut land = vec![false; bw * bh];
    let mut dist = vec![f32::MAX; bw * bh];
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    for ly in 0..bh {
        for lx in 0..bw {
            let gx = x0 + lx as i32;
            let gy = y0 + ly as i32;
            let wx = buf.wrap_x(gx);
            let wi = buf.idx(wx, gy as u32);
            let is_land = buf.terrain[wi] == 1;
            land[ly * bw + lx] = is_land;
        }
    }
    // Seed the BFS from every land/sea boundary cell.
    for ly in 0..bh {
        for lx in 0..bw {
            let here = land[ly * bw + lx];
            let mut boundary = false;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nlx = lx as i32 + dx;
                let nly = ly as i32 + dy;
                if nlx < 0 || nly < 0 || nlx as usize >= bw || nly as usize >= bh {
                    continue;
                }
                if land[nly as usize * bw + nlx as usize] != here {
                    boundary = true;
                    break;
                }
            }
            if boundary {
                let li = ly * bw + lx;
                dist[li] = 0.0;
                queue.push_back((x0 + lx as i32, y0 + ly as i32));
            }
        }
    }
    while let Some((gx, gy)) = queue.pop_front() {
        let Some(li) = local_idx(gx, gy) else { continue };
        let d = dist[li];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let ngx = gx + dx;
            let ngy = gy + dy;
            let Some(nli) = local_idx(ngx, ngy) else { continue };
            if dist[nli] > d + 1.0 {
                dist[nli] = d + 1.0;
                queue.push_back((ngx, ngy));
            }
        }
    }
    let mut out = HashMap::with_capacity(cells.len());
    for &(x, y) in cells {
        if let Some(li) = local_idx(x as i32, y as i32) {
            let sign = if land[li] { 1.0 } else { -1.0 };
            let d = if dist[li] == f32::MAX { margin } else { dist[li] };
            out.insert((x, y), sign * d);
        } else {
            let _ = world_w;
        }
    }
    out
}

// ── 2. Fjords ────────────────────────────────────────────────────────────

/// Walked from a sea cell *inland* up the coast-distance gradient: sinuous,
/// tapering to the head, occasionally branching. This is the honest way to
/// draw a fjord — carving a real channel — as opposed to notching a
/// coastline with noise (see §8.23's record of why noise-carved channels
/// read as a drawn scratch, not a landform).
pub fn fjords(buf: &mut WorldBuffer, lasso: &Lasso, count: u32, length_km: f32, width: f32, seed: u64) {
    let w = buf.width;
    let h = buf.height;
    let km_per_cell = KM_EQUATOR / w as f32;
    let steps = ((length_km / km_per_cell).round() as usize).clamp(3, 400);
    let half_w = width.max(1.0);

    let coast_cells: Vec<(u32, u32)> = lasso
        .candidate_cells(w, h, FEATHER_CELLS)
        .into_iter()
        .filter(|&(x, y)| {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 0 {
                return false;
            }
            if lasso.blend(x as f32, y as f32, w) < 0.5 {
                return false;
            }
            // adjacent to land
            [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                let sx = buf.wrap_x(x as i32 + dx);
                let sy = y as i32 + dy;
                sy >= 0 && sy < h as i32 && buf.terrain[buf.idx(sx, sy as u32)] == 1
            })
        })
        .collect();
    if coast_cells.is_empty() {
        return;
    }

    let mut terrain = buf.terrain.clone();
    let mut elevation = buf.elevation.clone();
    for f in 0..count {
        let pick = (cell_hash(f as i32 * 97, seed as i32, seed.wrapping_add(f as u64)) * coast_cells.len() as f32)
            as usize
            % coast_cells.len();
        let (sx, sy) = coast_cells[pick];
        // Initial heading: away from the sea, toward the nearest land neighbour.
        let mut heading = {
            let mut best = 0.0f32;
            let mut best_land = false;
            for a in 0..8 {
                let ang = a as f32 * std::f32::consts::PI / 4.0;
                let tx = buf.wrap_x(sx as i32 + (ang.cos() * 2.0).round() as i32);
                let ty = (sy as i32 + (ang.sin() * 2.0).round() as i32).clamp(0, h as i32 - 1) as u32;
                if buf.terrain[buf.idx(tx, ty)] == 1 && !best_land {
                    best = ang;
                    best_land = true;
                }
            }
            best
        };
        let mut x = sx as f32;
        let mut y = sy as f32;
        let branch_seed = seed.wrapping_add(f as u64 * 131);
        let mut path = Vec::with_capacity(steps);
        for s in 0..steps {
            heading += (fbm_noise(s as f32 / 12.0, f as f32 * 3.1, branch_seed, 3, 2.0, 0.5) - 0.5) * 0.5;
            x += heading.cos();
            y += heading.sin();
            let wx = buf.wrap_x(x.round() as i32);
            let wy_i = y.round() as i32;
            if wy_i < 0 || wy_i >= h as i32 {
                break;
            }
            let wy = wy_i as u32;
            path.push((wx, wy));
            if lasso.blend(wx as f32, wy as f32, w) <= 0.0 {
                break;
            }
        }
        let n = path.len().max(1) as f32;
        for (s, &(px, py)) in path.iter().enumerate() {
            let taper = 1.0 - (s as f32 / n); // full width at the mouth, zero at the head
            let radius = (half_w * taper).max(0.6);
            let ri = radius.ceil() as i32;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    if d > radius {
                        continue;
                    }
                    let cx = buf.wrap_x(px as i32 + dx);
                    let cy_i = py as i32 + dy;
                    if cy_i < 0 || cy_i >= h as i32 {
                        continue;
                    }
                    let cy = cy_i as u32;
                    let cblend = lasso.blend(cx as f32, cy as f32, w) * (1.0 - d / radius.max(0.01));
                    if !feathered_take(cx, cy, cblend.min(1.0), seed.wrapping_add(7 + s as u64)) {
                        continue;
                    }
                    let ci = buf.idx(cx, cy);
                    terrain[ci] = 0;
                    elevation[ci] = 0.0;
                }
            }
        }
    }
    buf.terrain = terrain;
    buf.elevation = elevation;
}

// ── 3. Island chains ────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IslandKind {
    Arc,
    Scatter,
    Single,
}

/// Plants `count` island blobs inside the lasso. `Arc` islands are marked
/// `is_volcanic` — real data, since `deposits.rs`'s `VolcanicArc` model scores
/// off that exact column (§8.16), so a planted arc can carry a genuine ore
/// province later.
pub fn island_chain(buf: &mut WorldBuffer, lasso: &Lasso, count: u32, kind: IslandKind, size: f32, seed: u64) {
    if count == 0 {
        return;
    }
    let w = buf.width;
    let h = buf.height;
    let (cx0, cy0, cx1, cy1) = lasso.bounds();
    if cx0 > cx1 {
        return;
    }
    let (ccx, ccy) = lasso.centroid();
    let radius_bound = (((cx1 - cx0).max(cy1 - cy0)) * 0.5).max(4.0);

    let mut centers: Vec<(f32, f32)> = Vec::with_capacity(count as usize);
    match kind {
        IslandKind::Single => centers.push((ccx, ccy)),
        IslandKind::Arc => {
            let arc_r = radius_bound * 0.65;
            let a0 = cell_hash(0, 0, seed) * std::f32::consts::TAU;
            let sweep = std::f32::consts::PI * (0.5 + cell_hash(1, 1, seed) * 0.6);
            for i in 0..count {
                let t = if count > 1 { i as f32 / (count - 1) as f32 } else { 0.0 };
                let a = a0 + sweep * t;
                let jitter = (cell_hash(i as i32, 5, seed) - 0.5) * arc_r * 0.15;
                centers.push((ccx + (arc_r + jitter) * a.cos(), ccy + (arc_r + jitter) * a.sin()));
            }
        }
        IslandKind::Scatter => {
            for i in 0..count {
                let rx = (cell_hash(i as i32, 11, seed) - 0.5) * (cx1 - cx0);
                let ry = (cell_hash(i as i32, 23, seed) - 0.5) * (cy1 - cy0);
                centers.push((ccx + rx, ccy + ry));
            }
        }
    }

    let mut terrain = buf.terrain.clone();
    let mut elevation = buf.elevation.clone();
    let mut volcanic = buf.is_volcanic.clone();
    for (ci, &(cx, cy)) in centers.iter().enumerate() {
        // A center that landed OUTSIDE the selection (Scatter spreads them across the
        // whole bounding box, and an irregular lasso leaves gaps) is pulled back toward
        // the centroid until it's inside — otherwise most scattered islands were simply
        // skipped and few or none appeared. The centroid of a drawn lasso is inside it.
        let (mut cx, mut cy) = (cx, cy);
        let mut tries = 0;
        while lasso.blend(cx, cy, w) <= 0.0 && tries < 8 {
            cx += (ccx - cx) * 0.4;
            cy += (ccy - cy) * 0.4;
            tries += 1;
        }
        if lasso.blend(cx, cy, w) <= 0.0 {
            continue;
        }
        let base_r = size.max(1.0);
        let ri = (base_r * 1.6).ceil() as i32;
        let iy_c = cy.round() as i32;
        if iy_c < 0 || iy_c >= h as i32 {
            continue;
        }
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let jitter = 1.0
                    + (fbm_noise(
                        (cx + dx as f32) / 4.0,
                        (cy + dy as f32) / 4.0,
                        seed.wrapping_add(ci as u64 * 17),
                        3,
                        2.0,
                        0.5,
                    ) - 0.5)
                        * 0.6;
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                let r = base_r * jitter;
                if d > r {
                    continue;
                }
                let px = cx.round() as i32 + dx;
                let py = iy_c + dy;
                if py < 0 || py >= h as i32 {
                    continue;
                }
                let wx = buf.wrap_x(px);
                let wy = py as u32;
                let blend = lasso.blend(wx as f32, wy as f32, w) * (1.0 - d / r.max(0.01)).max(0.0);
                if !feathered_take(wx, wy, blend.min(1.0), seed.wrapping_add(3 + ci as u64)) {
                    continue;
                }
                let idx = buf.idx(wx, wy);
                terrain[idx] = 1;
                elevation[idx] = elevation[idx].max(0.10 + 0.20 * (1.0 - d / r.max(0.01)).max(0.0));
                if kind == IslandKind::Arc {
                    volcanic[idx] = 1;
                }
            }
        }
    }
    buf.terrain = terrain;
    buf.elevation = elevation;
    buf.is_volcanic = volcanic;
}

// ── 4. Fill ──────────────────────────────────────────────────────────────

/// Bulk-set every cell inside the lasso to land or sea, feathered at the edge
/// like every other op here (rule 2).
pub fn fill(buf: &mut WorldBuffer, lasso: &Lasso, land: bool) {
    let w = buf.width;
    let h = buf.height;
    let cells = lasso.candidate_cells(w, h, FEATHER_CELLS);
    let target = if land { 1u8 } else { 0u8 };
    let mut terrain = buf.terrain.clone();
    for &(x, y) in &cells {
        let blend = lasso.blend(x as f32, y as f32, w);
        if blend < 0.5 {
            continue; // fill is decisive, not stochastic; only commit past the midline
        }
        let i = buf.idx(x, y);
        terrain[i] = target;
    }
    apply_terrain(buf, &terrain, &cells);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::ColumnSet;

    fn test_buf(w: u32, h: u32) -> WorldBuffer {
        let n = (w * h) as usize;
        WorldBuffer {
            cols: ColumnSet::PHASE_PLATES, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0, obliquity: 23.44,
            rotation_rate: 1.0, solar_lum: 1.0, greenhouse: 1.0, eccentricity: 0.0167, dryness: 1.0,
            terrain: vec![0u8; n], elevation: vec![0.0; n],
            sea_depth: vec![0.0; n], is_shelf: vec![0u8; n], is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(), plate_index: Vec::new(), boundary_type: Vec::new(),
            is_volcanic: vec![0u8; n], temperature: Vec::new(), precipitation: Vec::new(),
            koppen: Vec::new(), soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(), wind_speed: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(), precip_summer_frac: Vec::new(),
            seasonal_amp: Vec::new(), sst: Vec::new(), snow_frac: Vec::new(), biome: Vec::new(),
        }
    }

    // Rule 1: the lasso is unwrapped, not clamped.
    #[test]
    fn lasso_across_antimeridian_selects_what_was_drawn() {
        let w = 200u32;
        // A small square straddling the seam: from x=195 to x=205 (wraps to 5).
        let raw = vec![(195.0, 10.0), (205.0, 10.0), (205.0, 20.0), (195.0, 20.0)];
        let lasso = Lasso::new(raw, w);
        // A point just inside the drawn square on the east side of the seam.
        assert!(lasso.contains(198.0, 15.0, w), "east side of seam should be inside");
        // A point just inside on the wrapped (west) side of the seam.
        assert!(lasso.contains(3.0, 15.0, w), "wrapped west side of seam should be inside");
        // A point on the far side of the world (the complement a naive test
        // would have picked) must NOT be selected.
        assert!(!lasso.contains(100.0, 15.0, w), "far side of the world must stay unselected");
    }

    // Rule 2: every op feathers to the lasso edge — nothing changes strictly
    // outside the feather band.
    #[test]
    fn op_feathers_to_lasso_edge() {
        let mut buf = test_buf(300, 150);
        // A half-land, half-sea world so smoothing/roughening has something to do.
        for y in 0..buf.height {
            for x in 0..buf.width {
                let i = buf.idx(x, y);
                buf.terrain[i] = if x < 150 { 1 } else { 0 };
            }
        }
        let before = buf.terrain.clone();
        let lasso = Lasso::new(vec![(60.0, 40.0), (100.0, 40.0), (100.0, 80.0), (60.0, 80.0)], buf.width);
        smooth_roughen(&mut buf, &lasso, 0.8, 42);
        for y in 0..buf.height {
            for x in 0..buf.width {
                let d = lasso.signed_dist(x as f32, y as f32, buf.width);
                if d < -FEATHER_CELLS {
                    let i = buf.idx(x, y);
                    assert_eq!(buf.terrain[i], before[i], "cell strictly outside the feather band changed");
                }
            }
        }
    }

    // Rule 3: roughening is a level set bounded by reach, not a per-cell dice
    // roll — it must never plant land far out in open ocean.
    #[test]
    fn roughening_is_bounded_by_reach_not_scattered() {
        let mut buf = test_buf(300, 150);
        for y in 0..buf.height {
            for x in 0..buf.width {
                let i = buf.idx(x, y);
                buf.terrain[i] = if x < 150 { 1 } else { 0 };
            }
        }
        // Lasso far out in open ocean, well past any bounded reach from the coast.
        let lasso = Lasso::new(vec![(250.0, 40.0), (290.0, 40.0), (290.0, 80.0), (250.0, 80.0)], buf.width);
        smooth_roughen(&mut buf, &lasso, 1.0, 7);
        for y in 40..80 {
            for x in 250..290u32 {
                let i = buf.idx(x, y);
                assert_eq!(buf.terrain[i], 0, "deep-ocean cell far from any coast must stay sea");
            }
        }
    }

    // Rule 4: ops iterate the selection, never the world.
    #[test]
    fn ops_iterate_selection_not_world() {
        let mut buf = test_buf(4000, 2000); // a "large" world
        let lasso = Lasso::new(vec![(10.0, 10.0), (16.0, 10.0), (16.0, 16.0), (10.0, 16.0)], buf.width);
        let start = std::time::Instant::now();
        fill(&mut buf, &lasso, true);
        let elapsed = start.elapsed();
        // A whole-grid sweep of 8M cells would show up here; a bbox-scoped op
        // touching a few hundred cells should be essentially instant.
        assert!(elapsed.as_millis() < 200, "fill on a tiny lasso took {:?} — looks like a full-grid scan", elapsed);
        // And it must have actually done something inside the lasso.
        let i = buf.idx(13, 13);
        assert_eq!(buf.terrain[i], 1);
        // Nothing far away was touched.
        let j = buf.idx(2000, 1000);
        assert_eq!(buf.terrain[j], 0);
    }

    #[test]
    fn fjords_carve_from_sea_and_taper_to_a_head() {
        let mut buf = test_buf(300, 150);
        for y in 0..buf.height {
            for x in 0..buf.width {
                let i = buf.idx(x, y);
                buf.terrain[i] = if x < 150 { 1 } else { 0 };
            }
        }
        let lasso = Lasso::new(vec![(140.0, 60.0), (160.0, 60.0), (160.0, 90.0), (140.0, 90.0)], buf.width);
        fjords(&mut buf, &lasso, 1, 300.0, 3.0, 5);
        let mut carved = 0;
        for y in 55..95u32 {
            for x in 130..170u32 {
                let i = buf.idx(x, y);
                if buf.terrain[i] == 0 && x < 150 {
                    carved += 1;
                }
            }
        }
        assert!(carved > 0, "fjord should carve at least some land into sea");
    }

    #[test]
    fn arc_islands_are_volcanic_and_scatter_islands_are_not() {
        let mut buf = test_buf(300, 150);
        let lasso = Lasso::new(vec![(50.0, 50.0), (250.0, 50.0), (250.0, 100.0), (50.0, 100.0)], buf.width);
        island_chain(&mut buf, &lasso, 4, IslandKind::Arc, 3.0, 11);
        assert!(buf.is_volcanic.iter().any(|&v| v == 1), "arc islands should carry is_volcanic");
        let mut buf2 = test_buf(300, 150);
        island_chain(&mut buf2, &lasso, 4, IslandKind::Scatter, 3.0, 11);
        assert!(buf2.is_volcanic.iter().all(|&v| v == 0), "scatter islands should not be marked volcanic");
        assert!(buf2.terrain.iter().any(|&t| t == 1), "scatter islands should place some land");
    }
}
