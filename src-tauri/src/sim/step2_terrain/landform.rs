//! Physiographic provinces -- regional terrain CHARACTER.
//!
//! Before this module, every elevation generator used ONE global noise recipe
//! for the whole world: fixed frequencies, fixed amplitude weights, the same
//! ridged/smooth balance in every cell. CLAUDE.md section 8.21 had already named
//! the consequence ("land relief is texturally uniform"), and once the drawn-in
//! drainage texture of section 8.23 was removed it became the only thing left to
//! see: a single mottled cloud over every continent, with no plains, no
//! plateaus, no basins and no coherent ranges.
//!
//! Real topography is not one texture. It is a mosaic of PHYSIOGRAPHIC
//! PROVINCES -- the Great Plains, the Colorado Plateau, the Basin and Range, the
//! Canadian Shield are adjacent, roughly 1000-3000 km across, and each has its
//! own relief, its own roughness and its own characteristic wavelength. That
//! mosaic is what makes a real map unpredictable at a glance, and it is what
//! this module supplies.
//!
//! Three design decisions worth stating, because each has a wrong-looking
//! alternative that is easy to reach for:
//!
//!  * **Provinces are DISCRETE, with soft edges of VARYING width -- not a smooth
//!    blend everywhere.** Two low-frequency noise fields read as a "style space"
//!    would give organic variation with no polygon edges at all, which sounds
//!    strictly better and is not: a plateau needs a RIM. What makes the Colorado
//!    Plateau legible is precisely that it stops. So each archetype carries its
//!    own `edge` width: a plateau or a basin gets a narrow escarpment, a plain
//!    or a shield fades over hundreds of km.
//!
//!  * **The archetype is chosen from TECTONIC CONTEXT, not at random.** A massif
//!    sits near an orogenic belt, a shield sits deep in a stable interior. A
//!    purely random mosaic would be varied and incoherent -- mountains in the
//!    middle of cratons. Context makes the variety mean something.
//!
//!  * **Province borders are DOMAIN-WARPED before the site lookup**, so they are
//!    organic curves rather than Voronoi polygon edges. `TERRAIN_2_PLAN.md`
//!    slice 4 spent three passes learning that a straight geometric edge reads as
//!    an artefact however good the numbers behind it are; that lesson is imported
//!    here rather than re-paid for.
//!
//! Everything here is TRANSIENT, exactly like `geology.rs`: recomputed from the
//! seed every phase-2 run, used, discarded. No tile column, no save-format
//! change (rule 7).

use super::elevation::{box_blur_wrap, fbm_noise};

/// Archetype ids. Public so `terrain_metrics` and the dump sheets can report a
/// province census.
pub const LF_PLAIN: u8 = 0;
pub const LF_SHIELD: u8 = 1;
pub const LF_HILLS: u8 = 2;
pub const LF_UPLAND: u8 = 3;
pub const LF_MASSIF: u8 = 4;
pub const LF_PLATEAU: u8 = 5;
pub const LF_BASIN: u8 = 6;
pub const LF_KINDS: usize = 7;

pub fn kind_name(k: u8) -> &'static str {
    match k {
        LF_PLAIN => "plain",
        LF_SHIELD => "shield",
        LF_HILLS => "hills",
        LF_UPLAND => "upland",
        LF_MASSIF => "massif",
        LF_PLATEAU => "plateau",
        LF_BASIN => "basin",
        _ => "?",
    }
}

/// Typical span of a physiographic province, in km. The Colorado Plateau is
/// ~840 km across, the Great Plains ~1600, the Tibetan Plateau ~2500, the
/// Canadian Shield far larger. 1900 km puts roughly 30-60 provinces on the land
/// of an Earth-sized world -- enough that a continent is a mosaic rather than
/// one texture, few enough that each one reads at world zoom.
///
/// Stated in km and converted per world (rule 25): a cell is ~11 km at
/// 3600x1800 and ~130 km on a test fixture, so a spacing in CELLS would mean
/// completely different things on the two.
const PROVINCE_KM: f32 = 1900.0;

/// Earth's equatorial circumference in km -- a world's grid width spans it.
const KM_EQUATOR: f32 = 40075.0;

/// How far the province lattice is jittered off a regular grid, as a fraction of
/// the spacing. High enough that no lattice is visible, below 0.5 so sites
/// cannot swap order and leave a province with no cells.
const SITE_JITTER: f32 = 0.42;

/// Domain-warp strength for the border lookup, in units of the site spacing.
/// This is what turns Voronoi polygons into organic outlines.
///
/// Tuned by rendering: at 0.55 with a 1.6x-spacing wavelength the borders curved
/// but still held long straight runs, and a plateau RIM -- the one parameter
/// deliberately kept sharp -- traced them visibly as straight segments across the
/// map. Raising the amplitude and shortening the wavelength breaks those runs up.
/// Do not push much past this: a warp comparable to the spacing itself starts
/// detaching province fragments from their own sites, and a plateau whose rim
/// encloses nothing is worse than a straight one.
const BORDER_WARP: f32 = 0.80;
/// Warp wavelength, in units of the site spacing.
const BORDER_WARP_WAVE: f32 = 1.05;

/// Per-archetype character.
#[derive(Clone, Copy)]
struct Character {
    /// Multiplier on LOCAL relief (the ridged/medium/small terms). Never on the
    /// continental-scale term -- a province decides how rugged its ground is,
    /// not where the continent is high.
    amp: f32,
    /// 0 = smooth billowy relief, 1 = ridged/rugged relief.
    rugged: f32,
    /// How much FINE detail this province carries, as a weight on the
    /// short-wavelength term. 0 = broad landforms only, 1.5 = busy with small
    /// ones.
    ///
    /// NOT a frequency multiplier, and that distinction cost a render to find:
    /// scaling a noise function's COORDINATES by a spatially-varying factor is
    /// not a smooth reparametrisation -- the sample position jumps as the factor
    /// changes, and the result is concentric moire rings wherever the factor
    /// varies. It looked exactly like contour terracing in the hillshade.
    /// Varying the WEIGHT of two fields evaluated at FIXED frequencies is the
    /// artifact-free way to get the same "broad versus busy" axis.
    detail: f32,
    /// Plateau shaping: flatten the top and lift it, so the province ends in a rim.
    terrace: f32,
    /// Basin shaping: a closed depression centred on the province.
    bowl: f32,
}

const CHARACTERS: [Character; LF_KINDS] = [
    // plain -- almost nothing, and what there is, is broad
    Character { amp: 0.28, rugged: 0.10, detail: 0.30, terrace: 0.0, bowl: 0.0 },
    // shield -- old, worn, broad swells, very little fine texture
    Character { amp: 0.52, rugged: 0.18, detail: 0.35, terrace: 0.0, bowl: 0.0 },
    // hills -- moderate everything
    Character { amp: 0.88, rugged: 0.45, detail: 1.10, terrace: 0.0, bowl: 0.0 },
    // upland -- dissected: the busiest fine texture on the table
    Character { amp: 1.05, rugged: 0.68, detail: 1.60, terrace: 0.0, bowl: 0.0 },
    // massif -- high and ridge-dominated
    Character { amp: 1.70, rugged: 0.96, detail: 1.25, terrace: 0.0, bowl: 0.0 },
    // plateau -- elevated, SMOOTH on top, sharp rim
    Character { amp: 0.50, rugged: 0.25, detail: 0.25, terrace: 1.0, bowl: 0.0 },
    // basin -- low, smooth, closed
    Character { amp: 0.36, rugged: 0.20, detail: 0.30, terrace: 0.0, bowl: 1.0 },
];

/// Per-cell resolved character, plus the province id each cell belongs to.
pub struct LandformField {
    pub amp: Vec<f32>,
    pub rugged: Vec<f32>,
    pub detail: Vec<f32>,
    /// Plateau weight, already faded to 0 at the province border.
    pub terrace: Vec<f32>,
    /// Basin weight, already shaped into a bowl (1 at the centre, 0 at the rim).
    pub bowl: Vec<f32>,
    /// Nearest province id per cell.
    pub province: Vec<u32>,
    pub province_count: u32,
    /// Archetype per province.
    pub province_kind: Vec<u8>,
}

impl LandformField {
    /// An all-neutral field: every multiplier exactly 1.0, no shaping. Used when
    /// a world is too small to hold even one province, so the generators need no
    /// special case -- a true no-op, not an approximation.
    pub fn neutral(n: usize) -> Self {
        LandformField {
            amp: vec![1.0; n],
            rugged: vec![0.5; n],
            detail: vec![1.0; n],
            terrace: vec![0.0; n],
            bowl: vec![0.0; n],
            province: vec![0; n],
            province_count: 0,
            province_kind: Vec::new(),
        }
    }
}

fn hash01(a: u32, b: u32, seed: u64) -> f32 {
    let mut h = (seed as u32) ^ a.wrapping_mul(0x9E37_79B9);
    h ^= b.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 13;
    (h as f32) / (u32::MAX as f32)
}

/// Choose an archetype for a province from its tectonic and geographic context.
///
/// `oro` is 0..1 orogenic proximity (1 = on a belt), `cont` is 0..1
/// continentality (1 = deep interior). `r` is the province's own hash roll, so
/// two provinces in identical settings still differ.
fn pick_kind(oro: f32, cont: f32, r: f32) -> u8 {
    if oro > 0.60 {
        // Right on a belt: mountains, with the occasional high plateau between
        // the ranges (the Altiplano / Anatolian case).
        if r < 0.62 { LF_MASSIF } else if r < 0.85 { LF_UPLAND } else { LF_PLATEAU }
    } else if oro > 0.30 {
        // The foreland: dissected uplands and hills, sometimes a basin caught
        // between ranges.
        if r < 0.42 { LF_UPLAND } else if r < 0.78 { LF_HILLS } else { LF_BASIN }
    } else if cont > 0.55 {
        // Stable deep interior: shields, plateaus, endorheic basins, and the
        // great interior plains.
        if r < 0.30 { LF_SHIELD }
        else if r < 0.55 { LF_PLATEAU }
        else if r < 0.72 { LF_BASIN }
        else { LF_PLAIN }
    } else {
        // Coastal margin: plains and low hills, occasionally a coastal upland.
        if r < 0.46 { LF_PLAIN } else if r < 0.80 { LF_HILLS } else { LF_UPLAND }
    }
}

/// Build the province mosaic and resolve per-cell character.
///
/// `coast_dist` is distance-to-sea in cells (the generators all compute one).
/// `orogeny_dist`/`orogeny_reach` describe proximity to a real orogenic belt and
/// are `None` on the three plate-free models, which fall back to using local
/// relief as a stand-in -- the same documented fiction `geology.rs` uses, and
/// never dressed up as real tectonics.
pub fn build_landform_field(
    terrain: &[u8],
    w: u32,
    h: u32,
    seed: u64,
    coast_dist: &[u16],
    pre_elev: &[f32],
    orogeny_dist: Option<&[u16]>,
    orogeny_reach: f32,
) -> LandformField {
    let n = (w as usize) * (h as usize);
    let km_per_cell = KM_EQUATOR / w.max(1) as f32;
    let spacing = (PROVINCE_KM / km_per_cell).round();

    // A province has to be some cells across before it means anything -- below
    // this it is not a region with a character, it is per-cell noise wearing the
    // word "province". A world coarse enough to trip this (a 64-cell test
    // fixture is 626 km per cell) gets the exact no-op field instead, so the
    // generators need no special case.
    const MIN_SPACING_CELLS: f32 = 8.0;
    let cols = ((w as f32 / spacing).round() as i32).max(1);
    let rows = ((h as f32 / spacing).round() as i32).max(1);
    if spacing < MIN_SPACING_CELLS || cols < 2 || rows < 2 {
        return LandformField::neutral(n);
    }
    let cell_w = w as f32 / cols as f32;
    let cell_h = h as f32 / rows as f32;

    // ── Site positions on a jittered lattice ────────────────────────────────
    let count = (cols * rows) as usize;
    let mut sx = vec![0.0f32; count];
    let mut sy = vec![0.0f32; count];
    for gy in 0..rows {
        for gx in 0..cols {
            let i = (gy * cols + gx) as usize;
            let jx = (hash01(gx as u32, gy as u32, seed ^ 0x51A3) - 0.5) * 2.0 * SITE_JITTER;
            let jy = (hash01(gx as u32, gy as u32, seed ^ 0x7C11) - 0.5) * 2.0 * SITE_JITTER;
            sx[i] = (gx as f32 + 0.5 + jx) * cell_w;
            sy[i] = ((gy as f32 + 0.5 + jy) * cell_h).clamp(0.0, h as f32 - 1.0);
        }
    }

    // ── Context at each site, then its archetype ────────────────────────────
    // Sampled at the site's own cell rather than averaged over the province:
    // one sample is enough to place a province in its setting, and averaging
    // would need the assignment that does not exist yet.
    let max_coast = coast_dist
        .iter()
        .zip(terrain)
        .filter(|(_, &t)| t == 1)
        .map(|(&d, _)| d)
        .max()
        .unwrap_or(1)
        .max(1) as f32;

    let mut kind = vec![LF_PLAIN; count];
    for i in 0..count {
        let cx = (sx[i] as u32).min(w - 1);
        let cy = (sy[i] as u32).min(h - 1);
        let idx = (cy * w + cx) as usize;

        let cont = (coast_dist[idx] as f32 / max_coast).clamp(0.0, 1.0);
        let oro = match orogeny_dist {
            Some(d) if d[idx] != u16::MAX && orogeny_reach > 0.0 => {
                (1.0 - d[idx] as f32 / orogeny_reach).clamp(0.0, 1.0)
            }
            // Plate-free stand-in: local height. Documented fiction, never
            // presented as a real orogenic belt (see `geology::relief_pseudo_term`).
            _ => (pre_elev[idx] * 1.6).clamp(0.0, 1.0),
        };
        let r = hash01(i as u32, 0xA17E, seed ^ 0x3C5D);
        kind[i] = if terrain[idx] == 1 { pick_kind(oro, cont, r) } else {
            // A site whose own cell is sea still governs coastal land nearby;
            // give it a margin character rather than an interior one.
            if r < 0.55 { LF_PLAIN } else { LF_HILLS }
        };
    }

    // ── Assign every cell to its nearest site, then BLUR the parameter fields ──
    //
    // The obvious alternative -- blend the two nearest sites by how close the
    // cell sits to their bisector -- was built first and looked wrong: a
    // two-site blend still creases exactly where the nearest-site IDENTITY
    // changes, and where three provinces meet it creases along every bisector at
    // once. Rendered, the map came out crazed with a polygonal crack network,
    // like dried mud. Assigning hard and then blurring the parameter fields has
    // no such seam by construction: a box blur of a piecewise-constant field is
    // continuous everywhere.
    //
    // The two groups get DIFFERENT radii, and that is the whole reason a plateau
    // still has a rim. `amp`/`rugged`/`detail` blur over a quarter of a province
    // -- hundreds of km, so you can never see where one province's ruggedness
    // gives way to the next -- while `terrace`/`bowl` blur over a twentieth,
    // which is what keeps an escarpment an escarpment.
    let mut f = LandformField {
        amp: vec![1.0; n],
        rugged: vec![0.5; n],
        detail: vec![1.0; n],
        terrace: vec![0.0; n],
        bowl: vec![0.0; n],
        province: vec![0; n],
        province_count: count as u32,
        province_kind: kind.clone(),
    };

    let warp_freq = 1.0 / (spacing * BORDER_WARP_WAVE);
    let warp_amp = spacing * BORDER_WARP;
    let wf = w as f32;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;

            // Domain-warp the lookup so borders are organic curves rather than
            // Voronoi polygon edges (TERRAIN_2_PLAN slice 4's lesson, imported
            // rather than re-paid for).
            let fx = x as f32;
            let fy = y as f32;
            let wx = fx + (fbm_noise(fx * warp_freq + 3.7, fy * warp_freq + 1.1,
                                     seed ^ 0x2B7F, 3, 2.0, 0.5) - 0.5) * 2.0 * warp_amp;
            let wy = fy + (fbm_noise(fx * warp_freq + 8.2, fy * warp_freq + 5.9,
                                     seed ^ 0x6D31, 3, 2.0, 0.5) - 0.5) * 2.0 * warp_amp;

            // Search the 3x3 lattice neighbourhood of the warped position: sites
            // are jittered by less than half a cell, so no nearer site can lie
            // outside it. X wraps, Y clamps (rule 6).
            let gx0 = (wx / cell_w).floor() as i32;
            let gy0 = (wy / cell_h).floor() as i32;
            let mut best = f32::MAX;
            let mut bi = 0usize;
            for dy in -1..=1 {
                let gy = (gy0 + dy).clamp(0, rows - 1);
                for dx in -1..=1 {
                    let gx = ((gx0 + dx) % cols + cols) % cols;
                    let si = (gy * cols + gx) as usize;
                    let mut ddx = sx[si] - wx;
                    if ddx > wf * 0.5 { ddx -= wf; } else if ddx < -wf * 0.5 { ddx += wf; }
                    let ddy = sy[si] - wy;
                    let d = ddx * ddx + ddy * ddy;
                    if d < best { best = d; bi = si; }
                }
            }

            let c = CHARACTERS[kind[bi] as usize];
            f.province[idx] = bi as u32;
            f.amp[idx] = c.amp;
            f.rugged[idx] = c.rugged;
            f.detail[idx] = c.detail;
            f.terrace[idx] = c.terrace;

            // The bowl is shaped here, where the distance to the province's own
            // site is already known: deepest at the centre, gone by the rim.
            if c.bowl > 0.0 {
                let r = (best.max(0.0).sqrt() / (spacing * 0.62)).clamp(0.0, 1.0);
                f.bowl[idx] = c.bowl * (1.0 - r * r).max(0.0);
            }
        }
    }

    let wide = (spacing * 0.25).round().max(1.0) as i32;
    let sharp = (spacing * 0.05).round().max(1.0) as i32;
    f.amp = box_blur_wrap(&f.amp, w, h, wide);
    f.rugged = box_blur_wrap(&f.rugged, w, h, wide);
    f.detail = box_blur_wrap(&f.detail, w, h, wide);
    f.terrace = box_blur_wrap(&f.terrace, w, h, sharp);
    f.bowl = box_blur_wrap(&f.bowl, w, h, sharp);

    f
}

/// Plateau lift, in normalised elevation (~530 m). Applied where `terrace` is
/// full and fading to nothing at the rim, which is what draws the escarpment.
const PLATEAU_LIFT: f32 = 0.060;
/// How much of a plateau's internal relief is flattened away at full weight.
const PLATEAU_FLATTEN: f32 = 0.80;
/// Basin depth at the centre of a bowl province (~620 m).
const BASIN_DEPTH: f32 = 0.070;

/// Apply the shaping operators -- the ones that need ABSOLUTE heights and so
/// must run AFTER the hypsometric redistribution.
///
/// This ordering is not incidental. `redistribute_elevation` is a RANK remap: it
/// re-spreads land across the target height bands in sorted order, so a flat
/// plateau built before it would simply be un-flattened, its tied cells fanned
/// back out across a band. Amplitude and roughness modulation survives
/// redistribution (it changes which cells rank high, which is preserved);
/// flattening and depressing do not, and belong here.
pub fn apply_landform_shaping(
    elevation: &mut [f32],
    terrain: &[u8],
    f: &LandformField,
) {
    if f.province_count == 0 {
        return;
    }

    // Per-province reference level for terracing: the 60th percentile of that
    // province's own land, so the plateau surface sits a little above its median
    // ground rather than at an absolute height that would mean different things
    // on different worlds.
    let pc = f.province_count as usize;
    let mut buckets: Vec<Vec<f32>> = vec![Vec::new(); pc];
    let mut any_terrace = false;
    for i in 0..elevation.len() {
        if terrain[i] != 1 || f.terrace[i] <= 0.0 { continue; }
        any_terrace = true;
        buckets[f.province[i] as usize].push(elevation[i]);
    }

    let mut level = vec![0.0f32; pc];
    if any_terrace {
        for (p, b) in buckets.iter_mut().enumerate() {
            if b.is_empty() { continue; }
            b.sort_by(|a, c| a.partial_cmp(c).unwrap());
            level[p] = b[(b.len() as f32 * 0.60) as usize % b.len()];
        }
    }

    for i in 0..elevation.len() {
        if terrain[i] != 1 { continue; }
        let mut e = elevation[i];

        let t = f.terrace[i];
        if t > 0.0 {
            let lvl = level[f.province[i] as usize];
            if lvl > 0.0 {
                e = lvl + (e - lvl) * (1.0 - PLATEAU_FLATTEN * t);
                e += PLATEAU_LIFT * t;
            }
        }

        let b = f.bowl[i];
        if b > 0.0 {
            e -= BASIN_DEPTH * b;
        }

        elevation[i] = e.clamp(0.01, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangular continent in a sea frame, with a coast-distance field --
    /// enough context for the archetype picker to work with.
    fn fixture(w: u32, h: u32) -> (Vec<u8>, Vec<u16>, Vec<f32>) {
        let n = (w * h) as usize;
        let mut terrain = vec![1u8; n];
        let m = w / 10;
        for y in 0..h {
            for x in 0..w {
                if x < m || x >= w - m || y < m || y >= h - m {
                    terrain[(y * w + x) as usize] = 0;
                }
            }
        }
        // Chebyshev distance to the frame, which is the coast here.
        let mut coast = vec![0u16; n];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if terrain[i] != 1 { continue; }
                let d = (x as i32 - m as i32)
                    .min(w as i32 - 1 - m as i32 - x as i32)
                    .min(y as i32 - m as i32)
                    .min(h as i32 - 1 - m as i32 - y as i32)
                    .max(0);
                coast[i] = d as u16;
            }
        }
        let elev = vec![0.3f32; n];
        (terrain, coast, elev)
    }

    /// THE CLAIM this module exists for: a world must not be one noise recipe
    /// repeated everywhere. Different parts of it must carry genuinely different
    /// relief character.
    ///
    /// A neutral field (every multiplier 1.0) scores exactly 0 on both spreads,
    /// so this cannot pass by accident on a world that has no provinces.
    #[test]
    fn provinces_give_a_world_genuinely_different_country() {
        let (w, h) = (700u32, 400u32);
        let (terrain, coast, elev) = fixture(w, h);
        let f = build_landform_field(&terrain, w, h, 4242, &coast, &elev, None, 0.0);
        assert!(f.province_count > 4, "expected a real mosaic, got {} provinces", f.province_count);

        let mut amps: Vec<f32> = Vec::new();
        let mut rugs: Vec<f32> = Vec::new();
        for i in 0..terrain.len() {
            if terrain[i] != 1 { continue; }
            amps.push(f.amp[i]);
            rugs.push(f.rugged[i]);
        }
        amps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        rugs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |v: &Vec<f32>, q: f32| v[((v.len() - 1) as f32 * q) as usize];

        let amp_spread = p(&amps, 0.90) - p(&amps, 0.10);
        let rug_spread = p(&rugs, 0.90) - p(&rugs, 0.10);
        println!("amp p10..p90 = {:.2}..{:.2}  rugged p10..p90 = {:.2}..{:.2}",
                 p(&amps, 0.10), p(&amps, 0.90), p(&rugs, 0.10), p(&rugs, 0.90));
        assert!(amp_spread > 0.35,
                "relief amplitude barely varies across the world: p90-p10 = {amp_spread:.3}");
        assert!(rug_spread > 0.15,
                "ruggedness barely varies across the world: p90-p10 = {rug_spread:.3}");
    }

    /// The parameter fields must be SPATIALLY SMOOTH. This is the crack-network
    /// regression: the first cut blended the two nearest sites, which still
    /// creases wherever the nearest-site identity changes, and the rendered map
    /// came out crazed like dried mud. A step in `amp` is a step in elevation.
    ///
    /// `terrace` and `bowl` are deliberately excluded -- they are the shaping
    /// terms and a plateau rim is SUPPOSED to be abrupt.
    #[test]
    fn province_character_never_steps_between_neighbouring_cells() {
        let (w, h) = (700u32, 400u32);
        let (terrain, coast, elev) = fixture(w, h);
        let f = build_landform_field(&terrain, w, h, 99, &coast, &elev, None, 0.0);

        let mut worst = 0.0f32;
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let i = (y * w as i32 + x) as usize;
                if terrain[i] != 1 { continue; }
                for (dx, dy) in [(1i32, 0i32), (0, 1)] {
                    let nx = (x + dx) % w as i32;
                    let ny = y + dy;
                    if ny >= h as i32 { continue; }
                    let j = (ny * w as i32 + nx) as usize;
                    if terrain[j] != 1 { continue; }
                    worst = worst
                        .max((f.amp[i] - f.amp[j]).abs())
                        .max((f.rugged[i] - f.rugged[j]).abs())
                        .max((f.detail[i] - f.detail[j]).abs());
                }
            }
        }
        // The bound is stated against the TABLE'S OWN RANGE rather than as a
        // bare number, because the per-cell gradient of a blurred field scales
        // with the blur window, which scales with the world's cell size -- a
        // fixed constant would mean different things on different fixtures. An
        // unblurred hard assignment steps by the full range in one cell, so this
        // fails by more than an order of magnitude on the regression.
        let amp_range = CHARACTERS.iter().map(|c| c.amp).fold(0.0f32, f32::max)
            - CHARACTERS.iter().map(|c| c.amp).fold(f32::MAX, f32::min);
        println!("worst neighbouring-cell parameter step = {worst:.4} (table range {amp_range:.2})");
        assert!(worst < amp_range / 8.0,
                "province character steps between adjacent cells: {worst:.4}");
    }

    /// A world too coarse to hold even a 2x2 lattice of provinces gets a TRUE
    /// no-op field -- every multiplier exactly 1.0 and no shaping -- so the
    /// generators need no special case for it.
    #[test]
    fn a_world_too_small_for_provinces_is_exactly_neutral() {
        // 64 cells wide is 626 km per cell -- the `isostatic_rebound` fixture's
        // size, and far too coarse for a 1900 km province to be more than a
        // couple of cells.
        let (w, h) = (64u32, 48u32);
        let (terrain, coast, elev) = fixture(w, h);
        let f = build_landform_field(&terrain, w, h, 7, &coast, &elev, None, 0.0);
        assert_eq!(f.province_count, 0, "a 64-cell-wide world spans 626 km per cell");
        assert!(f.amp.iter().all(|&v| v == 1.0));
        assert!(f.terrace.iter().all(|&v| v == 0.0));
        assert!(f.bowl.iter().all(|&v| v == 0.0));

        // And shaping a neutral field must be bit-identical.
        let mut e = vec![0.4f32; (w * h) as usize];
        let before = e.clone();
        apply_landform_shaping(&mut e, &terrain, &f);
        assert_eq!(e, before, "neutral shaping must not touch the field");
    }

    /// Deterministic: same seed, same mosaic.
    #[test]
    fn the_province_mosaic_is_deterministic() {
        let (w, h) = (400u32, 240u32);
        let (terrain, coast, elev) = fixture(w, h);
        let a = build_landform_field(&terrain, w, h, 31337, &coast, &elev, None, 0.0);
        let b = build_landform_field(&terrain, w, h, 31337, &coast, &elev, None, 0.0);
        assert_eq!(a.province, b.province);
        assert_eq!(a.amp, b.amp);
        assert_eq!(a.province_kind, b.province_kind);
    }
}
