//! Terrain 2.0 — transient geology fields (docs/TERRAIN_2_PLAN.md §2, §4 slices 2-3).
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

/// Multi-source BFS from every convergent/transform boundary land cell,
/// carrying the seed cell's setting + age forward to every cell it reaches
/// first (§8.9 rule 1: a sweep/BFS, never a per-cell search). `None` when
/// `boundary_type` is empty (no real plate data — the plate-free models use
/// the relief pseudo-setting instead, see `build_geo_context`).
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

    while let Some(ci) = queue.pop_front() {
        let d = dist[ci];
        if d >= max_reach { continue; }
        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;
        for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let nx = buf.wrap_x(cx + dx);
            let ny = (cy + dy).clamp(0, h as i32 - 1) as u32;
            let ni = buf.idx(nx, ny);
            if buf.terrain[ni] == 1 && dist[ni] > d + 1 {
                dist[ni] = d + 1;
                setting[ni] = setting[ci];
                age[ni] = age[ci];
                queue.push_back(ni);
            }
        }
    }

    Some(OrogenyField { dist, setting, age })
}

/// Belt strength (0..1) at a given distance from the boundary, shaped by
/// setting (D4: no longer a single symmetric smoothstep for every geometry).
/// An active margin's arc crest sits OFFSET inland of the trench with a narrow
/// seaward scarp and a broad inland piedmont; a collision is broad and roughly
/// symmetric; an island arc is narrow and close to the boundary.
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
        SETTING_COLLISION => 1.0 - d / (belt_reach * 1.5),
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
