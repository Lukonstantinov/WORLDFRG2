//! Terrain 2.0 — transient geology fields (docs/CLAUDE.md §8.23b (Terrain 2.0, shipped) §2, §4 slices 2-3).
//!
//! Everything here is recomputed from `seed` + whatever phase 1 already persisted
//! (`plate_index`, `boundary_type`, `terrain`) every time phase 2 runs, used by the
//! erosion pass, then discarded — the plan's own "transient first" decision (§2):
//! zero blast radius, no tile-format change, no save-compat question, and
//! deterministic so re-running phase 2 reproduces it exactly.
//!
//! Where real plate data is absent (painted/template/imported land —
//! `boundary_type` empty), every tectonic-setting term falls back to a PSEUDO
//! setting inferred from relief and continentality alone. That fallback is a
//! documented fiction (§2's "all four models fully" risk note) — it must never
//! read as a claim about real polarity/age, and it must never make a plate-free
//! world worse than before this file existed.

use rand::prelude::*;
use rayon::prelude::*;
use crate::sim::world_buffer::WorldBuffer;
use super::elevation::fbm_noise;

// ── Orogeny (Terrain 2.0 slice 3, D2/D3/D4) ─────────────────────────────────

/// A convergent/transform land boundary cell's tectonic setting, inherited
/// outward through the belt by the same BFS that measures distance from it —
/// so a belt cell's setting/age comes from whichever boundary point is nearest
/// along the land, not resampled independently per cell.
pub const SETTING_NONE: u8 = 0;
/// This land is the OVERRIDING continental plate at an ocean-continent margin
/// (the Andes/Cascades geometry) — trench offshore, volcanic arc some distance
/// inland, broad piedmont beyond.
pub const SETTING_ACTIVE_MARGIN: u8 = 1;
/// Continent-continent collision (the Himalaya/Alps geometry) — broad, high,
/// roughly symmetric doubly-vergent belt.
pub const SETTING_COLLISION: u8 = 2;
/// This land sits on the SUBDUCTING oceanic plate itself, right at the trench
/// edge (an accretionary-wedge sliver — narrow, low).
pub const SETTING_SUBDUCTING_SIDE: u8 = 3;
/// Ocean-ocean convergence (the Japan/Aleutians geometry) — a narrow island arc.
pub const SETTING_ISLAND_ARC: u8 = 4;

pub struct OrogenyField {
    /// BFS distance in cells from the nearest convergent/transform boundary
    /// land cell. `u16::MAX` off any belt.
    pub dist: Vec<u16>,
    pub setting: Vec<u8>,
    /// 0 (young, freshly uplifted) .. 1 (old, worn down), inherited from the
    /// originating boundary point and so coherent along a belt's own strike.
    pub age: Vec<f32>,
}

/// Per-plate id: does this plate's land/sea footprint read as oceanic? Read
/// back from `terrain`/`plate_index` (majority vote) rather than the original
/// `Plate.is_oceanic` random draw, which phase 1 never persists (§2 "transient
/// first") — deterministic from the same persisted columns every time.
fn plate_oceanic_flags(buf: &WorldBuffer) -> Vec<bool> {
    let pc = buf.plate_index.iter().copied().max().map(|m| m as usize + 1).unwrap_or(0);
    if pc == 0 { return Vec::new(); }
    let mut land = vec![0u32; pc];
    let mut total = vec![0u32; pc];
    for i in 0..buf.total() {
        let p = buf.plate_index[i] as usize;
        total[p] += 1;
        if buf.terrain[i] == 1 { land[p] += 1; }
    }
    (0..pc).map(|p| total[p] == 0 || (land[p] as f32) < 0.5 * total[p] as f32).collect()
}

/// TECTONICS_AND_ISOLATION_PLAN.md Part B4 — a RELICT SUTURE bakes a former
/// collision belt into a world, entirely inside a plate's present-day interior
/// with no active boundary anywhere near it: the Urals, the Appalachians, the
/// Scottish Highlands and the Scandinavian Caledonides are all exactly this —
/// a healed collision the map still remembers, decades to hundreds of millions
/// of years after the boundary that made it stopped moving.
///
/// This is the "generate a past, not a simulation" answer the plan settles on:
/// a time-stepped tectonic model is Part I Slice 6, already deferred once, and
/// its output for THIS purpose — a range with an age — is almost exactly what
/// can be stated directly. `age` here is therefore not `fbm_noise` (§8.24b's own
/// negative result about that function's true range doesn't even apply — the
/// point is that noise is UNCORRELATED with anything, and a suture's age must
/// be a property of the whole suture, not of the cell): every cell of one
/// suture shares the SAME age, drawn once from an OLD or ANCIENT bucket, so a
/// whole range reads as one coherent age instead of dithering young/old along
/// its own strike the way the old per-cell noise term did on a REAL boundary.
struct ReliceSuture {
    /// Cells along the suture's own spine, seeded into the same multi-source
    /// BFS every active boundary already seeds — a relict range gets its WIDTH,
    /// its `belt_profile` shape and its erodibility term entirely for free,
    /// because downstream code cannot tell a suture seed from a real one.
    spine: Vec<usize>,
    age: f32,
}

/// AGE_OLD ≈ the Urals / Appalachians: `age_amp = 1.25 − age·0.5` (elevation.rs,
/// §8.24b) already turns a high age into a lower ridge amplitude — no elevation
/// code changes needed, the existing mechanism just needs a real age fed in.
const SUTURE_AGE_OLD: f32 = 0.80;
/// AGE_ANCIENT ≈ the Scottish Highlands / Scandinavian Caledonides — rolling
/// uplands rather than a real range.
const SUTURE_AGE_ANCIENT: f32 = 0.97;
/// How far a suture spine must stay from any ACTIVE boundary land cell, as a
/// fraction of world width — a relict suture inside the active belt's own
/// reach would just be swallowed by it (the active belt is younger and wins
/// on read order), and worse, would look like a claim that this particular
/// active margin is ALSO an old one, which is not what B4 is for.
const SUTURE_MIN_DIST_FROM_ACTIVE_FRAC: f32 = 0.06;
/// A suture's own length, as a fraction of world width — long enough to read
/// as a real range (the Urals run ~2,500 km, about 6% of Earth's circumference)
/// without so long that 2-4 of them cover a whole continent.
const SUTURE_LEN_FRAC: (f32, f32) = (0.10, 0.22);

/// Bake 2-4 relict sutures into the world's interior. Deterministic from
/// `seed`; empty on a plate-free world (no `plate_index` to place them inside)
/// or a world too small to hold one at the minimum distance from an active
/// boundary.
fn generate_relict_sutures(buf: &WorldBuffer, seed: u64) -> Vec<ReliceSuture> {
    if buf.plate_index.is_empty() { return Vec::new(); }
    let w = buf.width;
    let h = buf.height;
    let wf = w as f32;
    let n = buf.total();
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5375_7475_7265);

    // Active-boundary land cells, for the minimum-distance rejection below.
    // Bounded in practice (a boundary is a thin manifold, not the whole grid),
    // so a per-candidate scan against this list is cheap relative to the BFS
    // that follows it.
    let active: Vec<(i32, i32)> = (0..n)
        .filter(|&i| buf.terrain[i] == 1 && matches!(buf.boundary_type.get(i), Some(1) | Some(3)))
        .map(|i| ((i as u32 % w) as i32, (i as u32 / w) as i32))
        .collect();
    let min_dist = wf * SUTURE_MIN_DIST_FROM_ACTIVE_FRAC;
    let min_dist2 = min_dist * min_dist;
    let far_from_active = |x: i32, y: i32| -> bool {
        active.iter().all(|&(ax, ay)| {
            let mut dx = (x - ax).abs() as f32;
            if dx > wf / 2.0 { dx = wf - dx; }
            let dy = (y - ay) as f32;
            dx * dx + dy * dy >= min_dist2
        })
    };

    let count = 2 + (rng.gen::<u32>() % 3); // 2..=4
    let mut sutures = Vec::with_capacity(count as usize);
    for si in 0..count {
        // A handful of rejection tries for a starting point that is land, on a
        // single plate's now-continuous interior, and clear of any active
        // margin. Give up on this suture (not the whole world) if none is found
        // — a small or heavily-boundaried world may simply have no room for one,
        // which is an honest outcome, not a bug to force past.
        let mut start = None;
        for _ in 0..40 {
            let x = rng.gen_range(0..w) as i32;
            let y = rng.gen_range(0..h) as i32;
            let i = buf.idx(x as u32, y as u32);
            if buf.terrain[i] == 1 && far_from_active(x, y) { start = Some((x, y)); break; }
        }
        let Some((sx, sy)) = start else { continue };

        // Walk a gently curving spine: a fixed heading perturbed by a slow
        // noise term (never a hard random turn, which would draw a jagged
        // scratch rather than a range), stopping at the sea, at another
        // suture's own territory, or at the target length.
        let target_len = SUTURE_LEN_FRAC.0
            + rng.gen::<f32>() * (SUTURE_LEN_FRAC.1 - SUTURE_LEN_FRAC.0);
        let target_cells = (target_len * wf) as i32;
        let mut heading = rng.gen::<f32>() * std::f32::consts::TAU;
        let heading_seed = seed ^ (0x9E37_79B9 + si as u64 * 0x1000_0001);
        let (mut cx, mut cy) = (sx as f32, sy as f32);
        let mut spine = Vec::new();
        let mut steps = 0i32;
        while steps < target_cells {
            let xi = buf.wrap_x(cx.round() as i32);
            let yi = cy.round().clamp(0.0, h as f32 - 1.0) as u32;
            let i = buf.idx(xi, yi);
            if buf.terrain[i] != 1 { break; } // ran off the coast — a suture stays on land
            // The exclusion check at the START only guarantees the SEED is clear;
            // the heading drift below can still carry a later step back toward an
            // active margin. Stop the walk there instead of drawing a spine that
            // partway crosses into the exclusion zone.
            if !far_from_active(xi as i32, yi as i32) { break; }
            spine.push(i);
            // Slow heading drift from noise, not a fresh random turn each step,
            // so the spine reads as one continuous range rather than a random walk.
            let wobble = fbm_noise(steps as f32 * 0.02, si as f32 * 97.0, heading_seed, 2, 2.0, 0.5) - 0.5;
            heading += wobble * 0.10;
            cx += heading.cos();
            cy += heading.sin();
            steps += 1;
        }
        if spine.len() < 8 { continue; } // too short to read as a range at all

        let age = if rng.gen::<bool>() { SUTURE_AGE_OLD } else { SUTURE_AGE_ANCIENT };
        sutures.push(ReliceSuture { spine, age });
    }
    sutures
}

/// Multi-source BFS from every convergent/transform boundary land cell — AND
/// from every relict-suture spine cell (Part B4) — carrying the seed cell's
/// setting + age forward to every cell it reaches first (§8.9 rule 1: a
/// sweep/BFS, never a per-cell search). `None` when `boundary_type` is empty
/// (no real plate data — the plate-free models use the relief pseudo-setting
/// instead, see `build_geo_context`).
pub fn compute_orogeny_field(buf: &WorldBuffer, seed: u64, max_reach: u16) -> Option<OrogenyField> {
    if buf.boundary_type.is_empty() || buf.plate_index.is_empty() {
        return None;
    }
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let oceanic = plate_oceanic_flags(buf);

    let mut dist = vec![u16::MAX; n];
    let mut setting = vec![SETTING_NONE; n];
    let mut age = vec![0.5f32; n];
    let mut queue = std::collections::VecDeque::new();

    let age_f = 1.0 / 260.0;
    for i in 0..n {
        if buf.terrain[i] != 1 { continue; }
        let bt = buf.boundary_type[i];
        if bt != 1 && bt != 3 { continue; }
        let my_p = buf.plate_index[i] as usize;
        let x = (i as u32 % w) as i32;
        let y = (i as u32 / w) as i32;
        let mut other_p = my_p;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(x + dx);
            let ny = (y + dy).clamp(0, h as i32 - 1) as u32;
            let np = buf.plate_index[buf.idx(nx, ny)] as usize;
            if np != my_p { other_p = np; break; }
        }
        if other_p == my_p { continue; }
        let my_oc = oceanic.get(my_p).copied().unwrap_or(false);
        let other_oc = oceanic.get(other_p).copied().unwrap_or(false);
        setting[i] = match (my_oc, other_oc) {
            (false, true) => SETTING_ACTIVE_MARGIN,
            (true, false) => SETTING_SUBDUCTING_SIDE,
            (true, true) => SETTING_ISLAND_ARC,
            (false, false) => SETTING_COLLISION,
        };
        age[i] = fbm_noise(x as f32 * age_f, y as f32 * age_f, seed.wrapping_add(0x0A6E_A9E5), 3, 2.0, 0.5);
        dist[i] = 0;
        queue.push_back(i);
    }

    // Relict sutures (Part B4) seed the SAME queue, at the SAME dist=0 — the
    // BFS cannot tell a healed collision from an active one, which is exactly
    // what lets it inherit belt width, ridge amplitude and erodibility for
    // free. A suture cell that an active boundary already claimed (should be
    // rare, given `SUTURE_MIN_DIST_FROM_ACTIVE_FRAC`) is simply skipped —
    // the active, younger belt keeps it.
    for suture in generate_relict_sutures(buf, seed) {
        for i in suture.spine {
            if dist[i] == 0 { continue; } // already an active-boundary seed
            setting[i] = SETTING_COLLISION; // every attested relict suture is a healed collision
            age[i] = suture.age;
            dist[i] = 0;
            queue.push_back(i);
        }
    }

    // A plain 4-connected BFS propagates in unit hops along only the axis
    // directions, so its iso-distance contours are DIAMONDS (Manhattan
    // distance), not circles — visible directly wherever `belt_profile`
    // shades a compact, roughly-isolated source (a short relict-suture spine,
    // a small island-arc segment) into sharp geometric rings instead of an
    // organic massif. Fixed with a chamfer (3-4) distance transform: 8-
    // connected propagation with orthogonal steps costing 3 and diagonal
    // steps costing 4 (the classic cheap Euclidean approximation, ratio
    // 4/3 ≈ 1.333 against √2 ≈ 1.414, ~6% worst-case error) via a bounded
    // Dijkstra instead of a plain BFS queue — still linear in the reached
    // cell count, not the whole grid, since propagation still stops at
    // `max_reach` exactly as the BFS did. The scaled distance is divided
    // back down by the orthogonal weight before being returned, so `dist`
    // stays in the same CELL units `belt_profile`/`max_reach` already
    // expect and every existing caller is unaffected — only the CONTOUR
    // SHAPE changes, not the unit.
    const ORTHO: u32 = 3;
    const DIAG: u32 = 4;
    let mut dist_scaled: Vec<u32> = vec![u32::MAX; n];
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u32, usize)>> =
        std::collections::BinaryHeap::new();
    for &ci in &queue {
        dist_scaled[ci] = 0;
        heap.push(std::cmp::Reverse((0, ci)));
    }
    let max_reach_scaled = max_reach as u32 * ORTHO;
    while let Some(std::cmp::Reverse((d, ci))) = heap.pop() {
        if d > dist_scaled[ci] || d >= max_reach_scaled { continue; }
        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;
        for &(dx, dy, step) in &[
            (-1i32, 0i32, ORTHO), (1, 0, ORTHO), (0, -1, ORTHO), (0, 1, ORTHO),
            (-1, -1, DIAG), (-1, 1, DIAG), (1, -1, DIAG), (1, 1, DIAG),
        ] {
            let ny = cy + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            let nx = buf.wrap_x(cx + dx);
            let ni = buf.idx(nx, ny as u32);
            if buf.terrain[ni] != 1 { continue; }
            let nd = d + step;
            if nd < dist_scaled[ni] {
                dist_scaled[ni] = nd;
                setting[ni] = setting[ci];
                age[ni] = age[ci];
                heap.push(std::cmp::Reverse((nd, ni)));
            }
        }
    }
    for i in 0..n {
        dist[i] = if dist_scaled[i] == u32::MAX { u16::MAX }
                  else { ((dist_scaled[i] + ORTHO / 2) / ORTHO).min(u16::MAX as u32) as u16 };
    }

    Some(OrogenyField { dist, setting, age })
}

/// TECTONICS_AND_ISOLATION_PLAN.md Part B3 — why a collision belt is BROAD and
/// MULTI-RIDGE (Himalaya + Trans-Himalaya + the Tibetan plateau behind it)
/// while an active margin is narrow and single-crested (Andes). Real belts
/// differ by what's colliding: continent-continent thickens crust over a wide
/// front with several parallel sub-ranges and an elevated plateau between
/// them; ocean-continent concentrates uplift into one arc-parallel crest.
///
/// A single decay envelope (however modulated) cannot GUARANTEE two separate
/// crests — a cosine-lobed envelope was tried first and measured to fail:
/// multiplying a decaying envelope by an oscillation just gives one crest with
/// ripples on its downslope, never a genuine second local maximum, because the
/// envelope's own decay dominates the lobe amplitude by the time the second
/// lobe would peak. **Negative result, kept as the reason the shipped form
/// looks like this** (§2.4's own discipline). The fix is to stop multiplying
/// and instead take the MAX of two independent bump profiles — a main crest
/// right on the suture and a second, lower, offset crest (the Trans-Himalaya
/// beyond the Himalaya proper) — plus a broad low plateau floor between/behind
/// them (Tibet) so a trough reads as elevated tableland, never a valley cut
/// back toward zero (§8.23's own lesson about what carving looks like).
const COLLISION_REACH_MULT: f32 = 2.4;
/// Main-crest half-width, as a fraction of the collision reach.
const COLLISION_RIDGE1_WIDTH_FRAC: f32 = 0.42;
/// Second crest's distance from the boundary and its own half-width, both as a
/// fraction of the collision reach.
const COLLISION_RIDGE2_OFFSET_FRAC: f32 = 0.55;
const COLLISION_RIDGE2_WIDTH_FRAC: f32 = 0.30;
/// The second crest stands lower than the main range.
const COLLISION_RIDGE2_AMP: f32 = 0.72;
/// The plateau floor's peak strength (at d=0, decaying linearly to 0 at the
/// collision reach) — keeps the trough between the two crests an elevated
/// tableland rather than falling toward bare ground.
const COLLISION_PLATEAU_FLOOR: f32 = 0.35;

/// Belt strength (0..1) at a given distance from the boundary, shaped by
/// setting (D4: no longer a single symmetric smoothstep for every geometry).
/// An active margin's arc crest sits OFFSET inland of the trench with a narrow
/// seaward scarp and a broad inland piedmont; a collision is broad, MULTI-
/// RIDGE and roughly symmetric (Part B3); an island arc is narrow and close
/// to the boundary.
pub fn belt_profile(dist: u16, setting: u8, belt_reach: f32) -> f32 {
    let d = dist as f32;
    let raw = match setting {
        SETTING_ACTIVE_MARGIN => {
            let arc_offset = (belt_reach * 0.28).clamp(10.0, 30.0);
            let inland_w = belt_reach * 1.35;
            if d < arc_offset {
                d / arc_offset
            } else {
                1.0 - (d - arc_offset) / inland_w
            }
        }
        SETTING_COLLISION => {
            let reach = belt_reach * COLLISION_REACH_MULT;
            if d >= reach {
                0.0
            } else {
                let ridge1 = (1.0 - d / (reach * COLLISION_RIDGE1_WIDTH_FRAC)).max(0.0);
                let offset2 = reach * COLLISION_RIDGE2_OFFSET_FRAC;
                let ridge2 = COLLISION_RIDGE2_AMP
                    * (1.0 - (d - offset2).abs() / (reach * COLLISION_RIDGE2_WIDTH_FRAC)).max(0.0);
                let plateau = COLLISION_PLATEAU_FLOOR * (1.0 - d / reach).max(0.0);
                ridge1.max(ridge2).max(plateau)
            }
        }
        SETTING_SUBDUCTING_SIDE => 1.0 - d / (belt_reach * 0.45),
        SETTING_ISLAND_ARC => 1.0 - d / (belt_reach * 0.55),
        _ => 1.0 - d / belt_reach,
    };
    raw.clamp(0.0, 1.0)
}

/// Relative ridge amplitude by setting — a collision (Himalaya) stands taller
/// than an island arc (Japan) at the same belt strength.
pub fn setting_ridge_amp(setting: u8) -> f32 {
    match setting {
        SETTING_COLLISION => 1.18,
        SETTING_ACTIVE_MARGIN => 1.0,
        SETTING_ISLAND_ARC => 0.72,
        SETTING_SUBDUCTING_SIDE => 0.55,
        _ => 1.0,
    }
}

/// Erosion-resistance term by setting: young volcanic-arc ash/tuff is softer
/// than old collision-belt metamorphics.
fn setting_erodibility_term(setting: u8) -> f32 {
    match setting {
        SETTING_COLLISION => 1.20,
        SETTING_ACTIVE_MARGIN => 0.90,
        SETTING_ISLAND_ARC => 0.95,
        SETTING_SUBDUCTING_SIDE => 1.05,
        _ => 1.0,
    }
}

// ── Lithology (slice 2, D7) ──────────────────────────────────────────────────

/// Independent low-frequency noise bands so resistant rock holds ridges and
/// weak rock carves valleys, regardless of tectonic setting or plate data.
/// Ocean cells are never read downstream, so they're skipped rather than
/// costing a wasted `fbm_noise` call — this pass runs over the WHOLE grid
/// every phase-2 run and is `rayon`-parallel per §8.9 rule 2 for the same
/// reason (a per-cell map with no cross-cell dependency).
fn build_lithology(terrain: &[u8], w: u32, h: u32, seed: u64) -> Vec<f32> {
    let n = (w * h) as usize;
    let f = 1.0 / 340.0;
    (0..n)
        .into_par_iter()
        .map(|i| {
            if terrain[i] != 1 { return 1.0; }
            let x = (i as u32 % w) as f32;
            let y = (i as u32 / w) as f32;
            let noise = fbm_noise(x * f + 41.0, y * f + 17.0, seed.wrapping_add(0x7117_E0E0), 3, 2.1, 0.55);
            0.55 + noise * 1.05 // ~0.55 (weak) .. 1.6 (resistant)
        })
        .collect()
}

/// A relief-derived PSEUDO age term for the three plate-free models: land far
/// from the coast with a locally rugged pre-erosion surface reads as a young
/// upland (resistant); a low interior or a coastal fringe reads as an old,
/// weathered surface (erodes faster). This is deliberately NOT presented as
/// real orogeny — no polarity, no belt asymmetry, just a plausible erodibility
/// texture so plate-free worlds are not left flat (§2 "all four models fully").
fn relief_pseudo_term(pre_elev: &[f32], terrain: &[u8], coast_dist: &[u16], w: u32, h: u32) -> Vec<f32> {
    let n = (w * h) as usize;
    let max_coast = coast_dist.iter().copied().filter(|&d| d < u16::MAX).max().unwrap_or(1).max(1) as f32;
    (0..n)
        .into_par_iter()
        .map(|i| {
            if terrain[i] != 1 { return 1.0; }
            let mut local_max = pre_elev[i];
            let mut local_min = pre_elev[i];
            let x = (i as u32 % w) as i32;
            let y = (i as u32 / w) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = ((x + dx).rem_euclid(w as i32)) as u32;
                let ny = (y + dy).clamp(0, h as i32 - 1) as u32;
                let ni = (ny * w + nx) as usize;
                if terrain[ni] != 1 { continue; }
                local_max = local_max.max(pre_elev[ni]);
                local_min = local_min.min(pre_elev[ni]);
            }
            let rugged = (local_max - local_min).clamp(0.0, 0.3) / 0.3; // 0..1
            let continentality = (coast_dist[i].min(u16::MAX - 1) as f32 / max_coast).clamp(0.0, 1.0);
            // Rugged + interior reads "young/resistant" (low term); smooth +
            // coastal reads "old/weathered" (high term).
            (1.5 - rugged * 0.55 - continentality * 0.25).clamp(0.65, 1.5)
        })
        .collect()
}

// ── Climate erosion proxy (slice 2) ──────────────────────────────────────────

/// A phase-2 stand-in for the phase-3 precipitation field, which doesn't
/// exist yet at this point in the pipeline (§2 "climate PROXY inside phase
/// 2" — pipeline order untouched, nothing downstream re-runs). Wetter belts
/// (deep tropics, mid-latitude storm track) erode faster; the subtropical
/// dry belt and continental interiors erode slower. This WILL disagree with
/// the real phase-3 precipitation field later in the pipeline; that mismatch
/// is the accepted cost (§2), not hidden.
fn climate_erosion_proxy(buf: &WorldBuffer, coast_dist: &[u16]) -> Vec<f32> {
    let n = buf.total();
    let w = buf.width;
    let max_coast = coast_dist.iter().copied().filter(|&d| d < u16::MAX).max().unwrap_or(1).max(1) as f32;
    let terrain = &buf.terrain;
    let height = buf.height as f32;
    let (eq, ls, lr) = (buf.equator_offset, buf.lat_scale, buf.lat_ratio);
    (0..n)
        .into_par_iter()
        .map(|i| {
            if terrain[i] != 1 { return 1.0; }
            let y = (i as u32 / w) as f32;
            let lat = crate::sim::world_buffer::lat_from_y(y, height, eq, ls, lr).abs();
            // Wet at the equator and ~50-60° (storm track), dry ~15-30°
            // (subtropical highs) and drier again toward the poles -- a
            // coarse three-band proxy.
            let lat_term = if lat < 12.0 {
                1.35
            } else if lat < 30.0 {
                0.55 + (lat - 12.0) / 18.0 * 0.15
            } else if lat < 55.0 {
                0.70 + (lat - 30.0) / 25.0 * 0.75
            } else {
                (1.45 - (lat - 55.0) / 35.0 * 0.85).max(0.45)
            };
            let continentality = 1.0 - (coast_dist[i].min(u16::MAX - 1) as f32 / max_coast).clamp(0.0, 1.0);
            let coastal_bonus = 0.85 + continentality * 0.35; // wetter near the coast, drier deep inland
            (lat_term * coastal_bonus).clamp(0.4, 1.7)
        })
        .collect()
}

// ── Regions for the regionalised hypsometric redistribution (slice 2, D9) ──

/// A coarse grid used ONLY when there is no plate data to region by — big
/// enough that each region still gets a meaningful land sample, small enough
/// that regions read as genuinely different pieces of the map.
fn coarse_regions(w: u32, h: u32) -> (Vec<u32>, u32) {
    let blocks_x = (w / 90).max(2);
    let blocks_y = (h / 90).max(2);
    let bw = (w + blocks_x - 1) / blocks_x;
    let bh = (h + blocks_y - 1) / blocks_y;
    let n = (w * h) as usize;
    let mut out = vec![0u32; n];
    for i in 0..n {
        let x = i as u32 % w;
        let y = i as u32 / w;
        out[i] = (y / bh) * blocks_x + (x / bw);
    }
    (out, blocks_x * blocks_y)
}

// ── Public entry point ───────────────────────────────────────────────────────

pub struct GeoContext {
    /// Erosion-resistance multiplier (stream-power incision K term). Lower
    /// erodes SLOWER (more resistant).
    pub erodibility: Vec<f32>,
    /// Climate-driven erosion-rate multiplier (also a K term).
    pub climate: Vec<f32>,
    /// Per-cell region id for the regionalised hypsometric redistribution.
    pub region_id: Vec<u32>,
    pub region_count: u32,
}

/// Build the full transient geology context for one phase-2 run. `orogeny` is
/// `Some` only for the tectonic-plate model (real boundary data); the other
/// three models pass `None` and get the relief pseudo-setting instead — never
/// both, so a plate-free world is never given a false claim of real polarity.
pub fn build_geo_context(
    buf: &WorldBuffer,
    seed: u64,
    pre_elev: &[f32],
    coast_dist: &[u16],
    orogeny: Option<&OrogenyField>,
) -> GeoContext {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let mut erodibility = build_lithology(&buf.terrain, w, h, seed);

    if let Some(field) = orogeny {
        for i in 0..n {
            if buf.terrain[i] != 1 || field.dist[i] == u16::MAX { continue; }
            let age_term = 0.65 + field.age[i] * 0.9; // young resistant .. old soft
            erodibility[i] = (erodibility[i] * age_term * setting_erodibility_term(field.setting[i])).clamp(0.35, 2.2);
        }
    } else {
        let pseudo = relief_pseudo_term(pre_elev, &buf.terrain, coast_dist, w, h);
        for i in 0..n {
            erodibility[i] = (erodibility[i] * pseudo[i]).clamp(0.35, 2.2);
        }
    }

    let climate = climate_erosion_proxy(buf, coast_dist);

    let (region_id, region_count) = if !buf.plate_index.is_empty() {
        let count = buf.plate_index.iter().copied().max().map(|m| m as u32 + 1).unwrap_or(1);
        (buf.plate_index.iter().map(|&p| p as u32).collect(), count)
    } else {
        coarse_regions(w, h)
    };

    GeoContext { erodibility, climate, region_id, region_count }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;
    use crate::sim::world_buffer::ColumnSet;
    use rusqlite::Connection;

    fn gen_world(w: u32, h: u32, seed: u64, plate_count: u32) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        crate::sim::plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
        buf
    }

    /// The chamfer-distance fix's own gate: a plain 4-connected BFS propagates
    /// distance only along the axis directions, so a compact point source's
    /// iso-distance contour is a DIAMOND (Manhattan distance) — a diagonal cell
    /// reads roughly 1.4x farther than an axis cell at the same real distance
    /// (12 vs 8 cells here). The chamfer (3-4) transform brings that back in
    /// line with the true Euclidean distance. Built on a hand-crafted buffer
    /// with exactly ONE active-boundary seed cell (a real generated world's
    /// boundary is a whole curve, whose own contour shape this test cannot
    /// isolate) so the field is a clean point source.
    #[test]
    fn orogeny_distance_field_approximates_euclidean_not_manhattan() {
        let w = 200u32;
        let h = 100u32;
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        for t in buf.terrain.iter_mut() { *t = 1; }
        let sx = 100u32;
        let sy = 50u32;
        let seed_idx = buf.idx(sx, sy);
        let nb_idx = buf.idx(sx + 1, sy);
        buf.plate_index[nb_idx] = 1; // gives the seed cell a differing neighbour plate
        buf.boundary_type[seed_idx] = 1; // convergent — qualifies it as the one active seed

        let field = compute_orogeny_field(&buf, 42, 60).expect("plate data is present");
        let dist_at = |x: u32, y: u32| field.dist[buf.idx(x, y)];

        let axis = dist_at(sx + 8, sy); // real distance 8
        let diag = dist_at(sx + 6, sy + 6); // real distance 6*sqrt(2) ~= 8.49

        assert!(axis <= 9, "axis-direction distance should read close to the real 8 cells, got {axis}");
        assert!(diag <= axis + 2,
            "diagonal distance {diag} strayed far from the axis distance {axis} at the same \
             real distance — the iso-distance contour is diamond-shaped again, not round");
        assert!(diag < 12,
            "diagonal distance {diag} reached the OLD Manhattan-BFS value (12) the chamfer \
             fix exists to avoid");
    }

    /// TECTONICS_AND_ISOLATION_PLAN.md Part B4's own claim: a relict suture never
    /// sits inside an active boundary's own reach — otherwise it would either be
    /// swallowed by the younger active belt or, worse, read as a claim that a
    /// live, moving margin is somehow also an ancient healed one.
    #[test]
    fn relict_sutures_form_away_from_active_boundaries() {
        let mut found_any = false;
        for seed in 0..6u64 {
            let buf = gen_world(360, 180, seed, 10);
            let sutures = generate_relict_sutures(&buf, seed);
            let active: Vec<(i32, i32)> = (0..buf.total())
                .filter(|&i| buf.terrain[i] == 1 && matches!(buf.boundary_type.get(i), Some(1) | Some(3)))
                .map(|i| ((i as u32 % buf.width) as i32, (i as u32 / buf.width) as i32))
                .collect();
            let min_d = buf.width as f32 * SUTURE_MIN_DIST_FROM_ACTIVE_FRAC;
            for suture in &sutures {
                found_any = true;
                for &i in &suture.spine {
                    let (x, y) = ((i as u32 % buf.width) as i32, (i as u32 / buf.width) as i32);
                    for &(ax, ay) in &active {
                        let mut dx = (x - ax).abs() as f32;
                        if dx > buf.width as f32 / 2.0 { dx = buf.width as f32 - dx; }
                        let dy = (y - ay) as f32;
                        let d = (dx * dx + dy * dy).sqrt();
                        assert!(d >= min_d * 0.999,
                            "seed {seed}: a relict suture cell sits {d:.1} cells from an \
                             active boundary — inside its own {min_d:.1}-cell exclusion zone");
                    }
                }
                assert!(suture.age == SUTURE_AGE_OLD || suture.age == SUTURE_AGE_ANCIENT,
                    "a suture's age must come from the OLD/ANCIENT bucket, got {}", suture.age);
            }
        }
        assert!(found_any, "no relict suture formed on any of 6 seeds at a normal world size — \
             the placement search is too strict to ever succeed");
    }

    /// A suture's cells must all share ONE age (the whole point of B4 — a range
    /// reads as one coherent age, not per-cell noise dithering young/old along
    /// its own strike the way a REAL boundary's `fbm_noise` age term does).
    #[test]
    fn a_suture_carries_one_uniform_age() {
        for seed in 0..4u64 {
            let buf = gen_world(300, 150, seed, 9);
            for suture in generate_relict_sutures(&buf, seed) {
                let age0 = suture.age;
                assert!(suture.spine.iter().all(|_| age0 == suture.age),
                    "a suture must carry exactly one age for its whole spine");
            }
        }
    }

    /// Determinism (rule: the same seed must reproduce the identical world), since
    /// suture placement uses its own seeded RNG stream.
    #[test]
    fn relict_sutures_are_deterministic() {
        let buf = gen_world(300, 150, 555, 9);
        let a = generate_relict_sutures(&buf, 555);
        let b = generate_relict_sutures(&buf, 555);
        assert_eq!(a.len(), b.len());
        for (sa, sb) in a.iter().zip(b.iter()) {
            assert_eq!(sa.spine, sb.spine);
            assert_eq!(sa.age, sb.age);
        }
    }

    /// A plate-free world (no `plate_index`, e.g. a painted/template world) must
    /// get no sutures at all — there is no plate interior to place one inside,
    /// and inventing one would be exactly the "claim about real polarity" §2's
    /// own header warns against for the whole file.
    #[test]
    fn plate_free_world_gets_no_sutures() {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", "200".to_string()), ("grid_height", "100".to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        for i in 0..buf.total() { buf.terrain[i] = 1; }
        // `WorldBuffer::load_with` zero-fills `plate_index` to the grid size even
        // when no plate generation has run — explicitly empty it to model a real
        // plate-free (painted/template) world, matching `compute_orogeny_field`'s
        // own early-return check.
        buf.plate_index = Vec::new();
        buf.boundary_type = Vec::new();
        assert!(generate_relict_sutures(&buf, 1).is_empty());
    }

    /// The real claim `compute_orogeny_field` exists to serve: a relict suture's
    /// cells must come back OLDER (higher `age`) than a typical active-boundary
    /// cell, and that age must feed through to the field the elevation pass
    /// actually reads (not just exist in the intermediate `ReliceSuture` struct).
    #[test]
    fn orogeny_field_carries_the_suture_age_through() {
        let mut seen_old_or_ancient = false;
        for seed in 0..6u64 {
            let buf = gen_world(360, 180, seed, 10);
            let sutures = generate_relict_sutures(&buf, seed);
            if sutures.is_empty() { continue; }
            let field = compute_orogeny_field(&buf, seed, 60).expect("plate data present");
            for suture in &sutures {
                for &i in &suture.spine {
                    if field.dist[i] != 0 { continue; } // claimed by an active boundary instead
                    assert_eq!(field.age[i], suture.age,
                        "a suture cell's age did not survive into the orogeny field");
                    assert_eq!(field.setting[i], SETTING_COLLISION);
                    seen_old_or_ancient = true;
                }
            }
        }
        assert!(seen_old_or_ancient,
            "never observed a suture cell surviving into the orogeny field across 6 seeds");
    }

    /// Part B3's own required claim: a continent-continent collision belt must
    /// reach measurably FARTHER from the boundary than an ocean-continent
    /// active margin at the same `belt_reach` — the "broad" half of "broad,
    /// multi-ridge".
    #[test]
    fn collision_belt_is_wider_than_active_margin() {
        const EPS: f32 = 0.02;
        for belt_reach in [14.0f32, 30.0, 60.0, 90.0] {
            let last_active = (0..2000u16)
                .filter(|&d| belt_profile(d, SETTING_ACTIVE_MARGIN, belt_reach) > EPS)
                .last().unwrap_or(0);
            let last_collision = (0..2000u16)
                .filter(|&d| belt_profile(d, SETTING_COLLISION, belt_reach) > EPS)
                .last().unwrap_or(0);
            assert!(last_collision > last_active,
                "belt_reach={belt_reach}: collision belt (extends to {last_collision}) must reach \
                 farther than an active margin (extends to {last_active})");
        }
    }

    /// Part B3's other required claim: a collision belt's cross-section must be
    /// MULTI-CRESTED (a main range + at least one parallel sub-range, Himalaya +
    /// Trans-Himalaya), not one smooth decay — while an active margin stays
    /// single-crested (one arc, Andes-style). A flat single-ridge model fails
    /// this by construction, which is the property a real gate needs.
    #[test]
    fn collision_belt_is_multi_crested() {
        fn local_maxima(setting: u8, belt_reach: f32) -> usize {
            let samples: Vec<f32> = (0..=400)
                .map(|i| belt_profile((i as f32 * belt_reach * 1.2 / 400.0) as u16, setting, belt_reach))
                .collect();
            // `dist` is a u16 cell count, so at fine sub-cell sampling this
            // staircases into flat plateaus — a naive `>= next` test counts the
            // FIRST cell of every rising plateau as its own "peak". Treat each
            // plateau as one unit instead: only a run strictly higher than the
            // run before AND after it is a genuine local maximum.
            let mut peaks = 0;
            let mut i = 0;
            while i < samples.len() {
                let mut j = i;
                while j + 1 < samples.len() && samples[j + 1] == samples[i] { j += 1; }
                let higher_than_prev = i == 0 || samples[i] > samples[i - 1];
                let higher_than_next = j + 1 >= samples.len() || samples[i] > samples[j + 1];
                if samples[i] > 0.05 && higher_than_prev && higher_than_next {
                    peaks += 1;
                }
                i = j + 1;
            }
            peaks
        }
        for belt_reach in [14.0f32, 30.0, 60.0, 90.0] {
            let collision_peaks = local_maxima(SETTING_COLLISION, belt_reach);
            let margin_peaks = local_maxima(SETTING_ACTIVE_MARGIN, belt_reach);
            assert!(collision_peaks >= 2,
                "belt_reach={belt_reach}: a collision belt must be multi-crested, found {collision_peaks} peak(s)");
            assert!(margin_peaks <= 1,
                "belt_reach={belt_reach}: an active margin must stay single-crested, found {margin_peaks} peak(s)");
        }
    }
}
