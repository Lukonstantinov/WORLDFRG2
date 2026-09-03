use rand::prelude::*;
use rayon::prelude::*;
use std::collections::VecDeque;
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
    /// WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 4 (F5's real root cause) —
    /// an Euler pole + angular rate REPLACES the old flat `(vx, vy)`
    /// translation. A real plate rotates about a pole, so its velocity — and
    /// therefore a boundary's convergence rate and character — varies ALONG
    /// the boundary's length; a single translation vector makes an entire
    /// boundary uniformly convergent/divergent/transform end to end, which is
    /// what made it read as a drawn line rather than a margin.
    /// TECTONICS_AND_ISOLATION_PLAN.md Part B2 — the `Plate` struct itself is
    /// still transient (recomputed from seed every phase-1 run, `geology.rs`'s
    /// own "used, then discarded" discipline), but the field VALUES now leave
    /// this function via `PlateMotion` (below) and are persisted to
    /// `metadata["plate_motion"]` by the caller — the "promote to persisted
    /// world data" step the doc comment used to say was out of scope, done at
    /// the minimum needed for a read-only motion layer rather than the full
    /// per-plate UI override (Slice 5) the plan still defers.
    pole_x: f32,
    pole_y: f32,
    /// Signed angular rate. `velocity_at` turns this into a real (vx, vy) at
    /// any cell position — never read directly elsewhere.
    omega: f32,
    /// TECTONICS_AND_ISOLATION_PLAN.md Part B1 — this plate's SIZE CLASS weight,
    /// fed into `warped_voronoi_weighted` (a POWER DIAGRAM, not a plain
    /// nearest-seed Voronoi) so a "giant" plate captures genuinely more
    /// territory than an ordinary one, the way Earth's Pacific plate (~103M km²)
    /// dwarfs its Juan de Fuca (~0.25M km²) — nearly three orders of magnitude
    /// the old jittered-grid seeding could never produce, since every plate
    /// claimed roughly one grid cell of territory by construction.
    size_weight: f32,
}

/// TECTONICS_AND_ISOLATION_PLAN.md Part B2 — the PERSISTED, PUBLIC form of one
/// plate's motion, returned by `generate_plates_and_landmass[_with_target]` and
/// written by the caller to `metadata["plate_motion"]` (the same pattern
/// `deposits`/`good_localities` already use for a generator's other one-shot
/// outputs — a JSON blob under a metadata key, not a new tile column).
///
/// This is the FIRST time any plate data has left `plates.rs` — before B2,
/// `Plate` was fully transient and the Euler-pole velocity field that already
/// drives boundary classification (Slice 4) could never be drawn. `centroid_*`
/// is stated separately from the pole: the pole is what physics needs (`v = ω
/// × r`), the centroid is what a renderer needs (where to plant an arrow so it
/// reads as "this plate", not "this point in space").
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PlateMotion {
    pub centroid_x: f32,
    pub centroid_y: f32,
    pub pole_x: f32,
    pub pole_y: f32,
    pub omega: f32,
    pub is_oceanic: bool,
}

impl PlateMotion {
    /// The same `v = ω × r` rigid-body rotation `plate_velocity_at` computes for
    /// the private `Plate` — duplicated rather than shared, because `Plate`
    /// itself stays private (only its VALUES are public, via this struct) and a
    /// public function taking a private type is the wrong shape for a query
    /// command outside this module to call.
    pub fn velocity_at(&self, x: f32, y: f32, world_w: f32) -> (f32, f32) {
        let mut rx = x - self.pole_x;
        if world_w > 1.0 {
            if rx > world_w / 2.0 { rx -= world_w; }
            if rx < -world_w / 2.0 { rx += world_w; }
        }
        let ry = y - self.pole_y;
        (-self.omega * ry, self.omega * rx)
    }
}

/// Rigid-body rotation velocity at world position (x, y) for a plate rotating
/// about its own Euler pole: v = ω × r, r = position − pole (cylindrical-X
/// wrapped so a pole placed across the seam still gives a continuous field).
fn plate_velocity_at(plate: &Plate, x: f32, y: f32, world_w: f32) -> (f32, f32) {
    let mut rx = x - plate.pole_x;
    if rx > world_w / 2.0 { rx -= world_w; }
    if rx < -world_w / 2.0 { rx += world_w; }
    let ry = y - plate.pole_y;
    (-plate.omega * ry, plate.omega * rx)
}

/// WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 5 (F3, D1): the target fraction of
/// the globe that ends up OCEAN. The old per-plate `is_oceanic = rng < 0.4` coin
/// flip converged on ~60% land whatever the plate count, and was *worse* at low
/// plate counts (71.8% at 6 plates) because the expected 2.4 oceanic plates has
/// real binomial variance — an unlucky draw easily yields one or zero. This
/// constant is hit BY CONSTRUCTION instead: plates are sorted by their actual
/// Voronoi cell count and marked oceanic until the target ocean AREA is met, so
/// the result is close to 70% ocean (Earth is ~71%) regardless of plate count or
/// the luck of one coin-flip sequence.
pub const DEFAULT_OCEAN_FRACTION: f32 = 0.70;

/// Domain-warp amplitude for the plate partition, as a fraction of the mean
/// plate spacing (see the long note in `warped_voronoi`).
///
/// These three constants were set by `diag_sweep_plate_warp`, not by eye — the
/// sweep maximises the fall in local margin straightness (the goal) subject to
/// worst-plate connectivity staying above 0.90 (the constraint: warp a sample
/// far enough and it lands inside a neighbouring plate, so the partition sheds
/// detached specks that `boundary_type` then reads as phantom plate boundaries).
/// Measured over 3 seeds at 360×180, 10 plates — plain Voronoi scores 0.449:
///
/// | amp | wav | clamp | straightness | worst conn |
/// |----|----|----|----|----|
/// | 0.25 | 2.50 | 0.80 | 0.424 | 1.000 |
/// | 0.35 | 2.50 | 0.80 | 0.407 | 0.991 |
/// | **0.25** | **0.80** | **0.80** | **0.357** | **0.942** | ← shipped
/// | 0.25 | 0.80 | 1.20 | 0.339 | 0.908 | (too near the bar)
/// | 0.35 | 0.80 | 0.80 | 0.307 | 0.518 | (shredded)
///
/// Note the shape of it: a LONG wavelength barely curves a margin at all (it
/// translates whole plates instead), and a large amplitude shreds them. The
/// usable region is a short wavelength at a modest, tightly-clamped amplitude.
const PLATE_WARP_AMP_FRAC: f32 = 0.25;
/// Wavelength of the warp's LOWEST octave, in plate spacings. See the table
/// above `PLATE_WARP_AMP_FRAC`.
const PLATE_WARP_WAVELENGTHS: f32 = 0.80;
/// How many standard deviations of the normalized warp field a single cell may
/// be displaced by. The field is near-gaussian, so without a clamp its tail
/// cells travel several sigma — deep into a neighbouring plate — and the
/// partition sheds detached specks. See the table above `PLATE_WARP_AMP_FRAC`.
const PLATE_WARP_SIGMA_CLAMP: f32 = 0.80;

/// TECTONICS_AND_ISOLATION_PLAN.md Part B1 — plate SIZE CLASS weights (giant /
/// large / medium / small), fed into `warped_voronoi_weighted`'s power diagram.
/// Only RATIOS between classes matter (the offset formula subtracts `weight −
/// 1.0`, so weight 1.0 is the neutral/unweighted case).
///
/// **NEGATIVE RESULT, recorded so it is not repeated (§2.4).** The first cut used
/// a MULTIPLICATIVELY weighted Voronoi (divide squared distance by weight²) —
/// the textbook approach. Measured on the gate's own 8-plate world, a small
/// plate could come out with its territory split into DISCONNECTED islands even
/// with the warp turned off entirely (`plate_territory_stays_connected` failed
/// at 87% connectivity at `warp_frac = 0.0`), which proved the fault was the
/// weighting metric itself, not the warp on top of it: a multiplicative metric
/// is not a true distance (the triangle inequality can fail), so its cells are
/// not guaranteed connected, and at only 8 plates and a 4× weight ratio a small
/// plate boxed in by bigger neighbours was pinched into pieces by construction.
///
/// The fix is a POWER DIAGRAM (Laguerre-Voronoi) instead: `d² − offset`, an
/// ADDITIVE term subtracted from the squared distance rather than a
/// multiplicative divisor. A power diagram's cells are provably convex — hence
/// always connected — for ANY offsets, at ANY plate count, which is what makes
/// `PLATE_WARP_AMP_FRAC_WEIGHTED` free to stay small without hunting for a
/// razor's-edge amplitude: the mathematics guarantees the property the old
/// approach could only approximate by tuning. `POWER_DIAGRAM_OFFSET_SCALE`
/// converts `size_weight` into that offset.
const SIZE_CLASS_WEIGHTS: [f32; 4] = [2.2, 1.6, 1.0, 0.55];
/// Proportions for the classes above, in the SAME order — must sum to 1.0.
/// Earth's own rough mix: one Pacific-scale giant among many, several
/// continent-scale large plates, a broad middle, and a crowd of small ones.
const SIZE_CLASS_PROPORTIONS: [f32; 4] = [0.10, 0.25, 0.40, 0.25];
/// Warp amplitude for the WEIGHTED (production) partition. Smaller than
/// `PLATE_WARP_AMP_FRAC`: the power diagram's convexity guarantee holds only
/// BEFORE the warp is applied, and the warp can still bend a thin part of an
/// otherwise-convex cell enough to sever it at full amplitude (measured: 0.25
/// reproduces the multiplicative failure, 0.08 does not, over the same 3 seeds
/// `plate_territory_stays_connected` checks).
const PLATE_WARP_AMP_FRAC_WEIGHTED: f32 = 0.08;
/// Scales the power-diagram offset as a fraction of `plate_spacing²`, so the
/// offset is dimensionally a squared distance regardless of world size or plate
/// count. Set by measurement: large enough that the size gate
/// (`plate_sizes_span_an_order_of_magnitude`, ≥5×) clears with real margin
/// (shipped: 7.74× mean, over 5 seeds) — connectivity does not depend on this
/// constant at all (a power diagram's cells are convex at any offset), so it
/// only has to satisfy the area target.
const POWER_DIAGRAM_OFFSET_SCALE: f32 = 2.2;

/// Assign every cell to its nearest plate seed — the Voronoi partition — but at
/// a DOMAIN-WARPED sample position rather than the cell's own.
///
/// `warp_frac` is the warp amplitude as a fraction of the mean plate spacing.
/// **Zero reproduces the plain Voronoi partition exactly**, which is what makes
/// this function its own control: `plate_margins_are_not_straight_bisectors`
/// calls it twice on identical seeds, once at 0 and once at the shipped value,
/// so the gate compares the warp against the real thing it replaced instead of
/// against a reconstruction that could silently drift out of step.
///
/// Rayon-parallel: each cell's nearest-plate scan is independent of every
/// other cell's (§8.9 rule 2).
pub(crate) fn warped_voronoi(
    seeds: &[(f32, f32)], width: u32, height: u32, count: usize, seed: u64, warp_frac: f32,
) -> Vec<u16> {
    warped_voronoi_tuned(seeds, width, height, count, seed, warp_frac,
                         PLATE_WARP_WAVELENGTHS, PLATE_WARP_SIGMA_CLAMP)
}

/// `warped_voronoi`, but each seed carries a SIZE WEIGHT — a multiplicatively
/// weighted Voronoi (a plate's captured territory grows with its weight, not
/// just its position), which is how TECTONICS_AND_ISOLATION_PLAN.md Part B1
/// gives plates genuinely different sizes without abandoning the even spatial
/// spread the jittered grid seeding already provides (moving seeds around
/// instead would cluster them and re-open the coverage gaps that seeding was
/// written to avoid). `weights[i] == 1.0` for every plate reproduces the
/// unweighted partition exactly, which is what lets `warped_voronoi` stay a
/// thin wrapper around this rather than a second implementation.
pub(crate) fn warped_voronoi_weighted(
    seeds: &[(f32, f32)], weights: &[f32], width: u32, height: u32, count: usize,
    seed: u64, warp_frac: f32,
) -> Vec<u16> {
    warped_voronoi_tuned_weighted(seeds, weights, width, height, count, seed, warp_frac,
                                  PLATE_WARP_WAVELENGTHS, PLATE_WARP_SIGMA_CLAMP)
}

/// `warped_voronoi` with its two shape knobs exposed, so `diag_sweep_plate_warp`
/// can measure the (amplitude, wavelength, clamp) space instead of guessing at
/// it. Production always goes through `warped_voronoi`, which pins them to the
/// shipped constants.
pub(crate) fn warped_voronoi_tuned(
    seeds: &[(f32, f32)], width: u32, height: u32, count: usize, seed: u64, warp_frac: f32,
    wavelengths: f32, sigma_clamp: f32,
) -> Vec<u16> {
    warped_voronoi_tuned_weighted(seeds, &vec![1.0f32; seeds.len()], width, height, count, seed,
                                  warp_frac, wavelengths, sigma_clamp)
}

/// `warped_voronoi_tuned`, weighted (see `warped_voronoi_weighted`).
pub(crate) fn warped_voronoi_tuned_weighted(
    seeds: &[(f32, f32)], weights: &[f32], width: u32, height: u32, count: usize, seed: u64,
    warp_frac: f32, wavelengths: f32, sigma_clamp: f32,
) -> Vec<u16> {
    let w = width as f32;
    let h = height as f32;
    let total = (width as usize) * (height as usize);
    // Amplitude is stated as a fraction of the mean PLATE SPACING, never in
    // cells (rule 25's spirit): the warp has to scale with the feature it
    // perturbs, or a 6-plate world is unrecognisably scrambled while a 40-plate
    // world is untouched.
    let plate_spacing = (w * h / count.max(1) as f32).sqrt();
    let warp_amp = plate_spacing * warp_frac;
    let warp_freq = 1.0 / (plate_spacing * wavelengths);

    // ── The warp field, NORMALIZED EMPIRICALLY ──────────────────────────────
    // NEGATIVE RESULT, recorded so it is not repeated (§2.4): the first version
    // used `fbm_noise(..) - 0.5` inline, on the documented assumption that fbm
    // returns 0..1 and so centres on 0.5. It does not. `fbm_noise` AVERAGES its
    // octaves (`val / max_amp`), and an average of octaves concentrates hard
    // about its own mean with much-reduced variance — measured on this world it
    // spans about 0.11..0.40, mean ≈ 0.28. Subtracting 0.5 therefore yields an
    // almost entirely NEGATIVE, nearly constant number, which TRANSLATES the
    // sample field instead of bending it. The partition duly changed (10% of
    // cells flipped, all of them near a margin) while the margins stayed exactly
    // as straight as before — the gate measured 0.504 → 0.498 straightness and
    // was right to fail. A displaced straight line is a straight line.
    //
    // So the field is centred and scaled on its OWN measured spread rather than
    // an assumed range. This is robust to whatever distribution `fbm_noise`
    // actually has, which is the property that was missing.
    let raw: Vec<(f32, f32)> = (0..total)
        .into_par_iter()
        .map(|idx| {
            let x = (idx as u32 % width) as f32;
            let y = (idx as u32 / width) as f32;
            // 4 octaves: the low ones give the margin its broad sweep (an arc, a
            // re-entrant), the high ones its ragged detail. A single octave
            // bends a margin into a plain sine; real plate margins are fractal.
            (
                fbm_noise(x * warp_freq + 13.0, y * warp_freq + 71.0,
                          seed ^ 0x9E37_79B9_7F4A_7C15, 4, 2.0, 0.5),
                fbm_noise(x * warp_freq + 157.0, y * warp_freq + 29.0,
                          seed ^ 0xC2B2_AE3D_27D4_EB4F, 4, 2.0, 0.5),
            )
        })
        .collect();
    // Centre on the mean and scale by the standard deviation, so `warp_amp` is a
    // real 1-sigma displacement in cells whatever fbm's own spread turns out to
    // be. Scaling by min/max instead would let one outlier cell set the gain for
    // the whole world.
    let inv_n = 1.0 / total.max(1) as f32;
    let (mx, my) = raw.iter().fold((0.0f32, 0.0f32), |a, r| (a.0 + r.0, a.1 + r.1));
    let (mx, my) = (mx * inv_n, my * inv_n);
    let (vx, vy) = raw.iter().fold((0.0f32, 0.0f32), |a, r| {
        (a.0 + (r.0 - mx) * (r.0 - mx), a.1 + (r.1 - my) * (r.1 - my))
    });
    let sx_scale = (vx * inv_n).sqrt().max(1e-6);
    let sy_scale = (vy * inv_n).sqrt().max(1e-6);

    (0..total)
        .into_par_iter()
        .map(|idx| {
            let x = (idx as u32 % width) as f32;
            let y = (idx as u32 / width) as f32;
            let (sx, sy) = if warp_amp > 0.0 {
                // CLAMPED in sigma: the normalized field is roughly gaussian, so
                // its tail cells would otherwise be displaced several sigma — far
                // enough to land deep inside a neighbouring plate and shred the
                // partition (`plate_territory_stays_connected`'s failure).
                let wx = ((raw[idx].0 - mx) / sx_scale).clamp(-sigma_clamp, sigma_clamp);
                let wy = ((raw[idx].1 - my) / sy_scale).clamp(-sigma_clamp, sigma_clamp);
                // Y is CLAMPED, never wrapped (rule 6): a warp that pushed a
                // polar sample past the pole and out the far side would tear
                // the partition along the top and bottom rows.
                (x + wx * warp_amp, (y + wy * warp_amp).clamp(0.0, h - 1.0))
            } else {
                (x, y)
            };
            let mut best_dist = f32::MAX;
            let mut best_plate = 0u16;
            for (pi, &(cx, cy)) in seeds.iter().enumerate() {
                let mut dx = sx - cx;
                // Wrap distance for cylindrical topology
                if dx > w / 2.0 { dx -= w; }
                if dx < -w / 2.0 { dx += w; }
                let dy = sy - cy;
                // A POWER DIAGRAM (Laguerre-Voronoi), not a multiplicatively
                // weighted Voronoi.
                //
                // NEGATIVE RESULT, recorded so it is not attempted again (§2.4):
                // the first cut divided the squared distance by weight² (the
                // textbook multiplicatively-weighted metric). Measured on the
                // gate's own 8-plate world, a small plate could come out with its
                // territory split into disconnected islands EVEN AT ZERO WARP —
                // `plate_territory_stays_connected` failed at 87% connectivity
                // with the warp amplitude turned off entirely
                // (`diag_check_zero_warp_shred`), which proved the fault was the
                // weighting itself, not the warp on top of it. A multiplicative
                // metric is not a true distance (the triangle inequality can
                // fail), and its Voronoi cells are not guaranteed connected or
                // even simply-connected — with only 8 plates and a 4× weight
                // ratio, a small plate boxed in by bigger neighbours can be
                // pinched into separate pieces by construction.
                //
                // A power diagram instead SUBTRACTS an additive offset from the
                // squared distance: `d² − offset`. This is the same family as an
                // ordinary weighted Voronoi generalises to when built from a
                // proper distance metric, and its cells are provably convex —
                // hence always connected — for ANY offsets, at ANY plate count.
                // The offset is scaled by `plate_spacing²` so it is dimensionally
                // a squared distance too, and centred on weight 1.0 so a
                // uniform-weight world (every weight = 1.0) is bit-identical to
                // the unweighted partition.
                let wt = weights.get(pi).copied().unwrap_or(1.0);
                let offset = (wt - 1.0) * plate_spacing * plate_spacing * POWER_DIAGRAM_OFFSET_SCALE;
                let dist = dx * dx + dy * dy - offset;
                if dist < best_dist {
                    best_dist = dist;
                    best_plate = pi as u16;
                }
            }
            best_plate
        })
        .collect()
}

/// Generate tectonic plates and derive landmass from plate types. Returns each
/// plate's motion (Part B2) so the caller can persist it — existing callers
/// that only want the side effect on `buf` are unaffected, since ignoring a
/// non-`#[must_use]` return value in statement position is not an error.
/// Matches WF1 plate-generator.ts algorithm.
pub fn generate_plates_and_landmass(buf: &mut WorldBuffer, seed: u64, plate_count: u32) -> Vec<PlateMotion> {
    generate_plates_and_landmass_with_target(buf, seed, plate_count, DEFAULT_OCEAN_FRACTION)
}

/// As `generate_plates_and_landmass`, but with an explicit ocean-area target
/// (Slice 5). `ocean_fraction` is clamped to a sane range so a pathological
/// input can never empty the world of land or of sea entirely.
pub fn generate_plates_and_landmass_with_target(
    buf: &mut WorldBuffer, seed: u64, plate_count: u32, ocean_fraction: f32,
) -> Vec<PlateMotion> {
    let mut rng = StdRng::seed_from_u64(seed);
    let w = buf.width as f32;
    let h = buf.height as f32;
    let count = plate_count.max(2) as usize;
    let ocean_fraction = ocean_fraction.clamp(0.05, 0.95);

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
        // is_oceanic is decided BELOW, once each plate's real Voronoi area is
        // known (Slice 5) -- this coin flip only seeds the density jitter, which
        // reads independently of the final oceanic/continental assignment.
        let is_oceanic = rng.gen::<f32>() < 0.4;
        let density = if is_oceanic {
            0.7 + rng.gen::<f32>() * 0.3
        } else {
            0.4 + rng.gen::<f32>() * 0.2
        };
        // Slice 4 — the pole sits well OFF the plate's own centroid (a real
        // Euler pole commonly sits far from the plate it rotates), at a
        // distance drawn from a real fraction of world width so the boundary
        // this plate touches spans a genuinely varying distance-and-bearing
        // from it. `omega` is picked so |v| at the CENTROID roughly matches
        // the old model's 0.5..1.0 speed range — continuity of scale, not of
        // mechanism — while cells elsewhere along a shared boundary see a
        // different distance/angle from the pole and so a different velocity.
        let pole_angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let pole_dist = (0.15 + rng.gen::<f32>() * 0.35) * w;
        let pole_x = cx + pole_angle.cos() * pole_dist;
        let pole_y = cy + pole_angle.sin() * pole_dist;
        let speed = 0.5 + rng.gen::<f32>() * 0.5;
        let omega = (if rng.gen::<bool>() { 1.0 } else { -1.0 }) * speed / pole_dist.max(1.0);
        // Part B1 — a SIZE CLASS ladder, not a smooth distribution: a real
        // continuous power law drawn independently per plate mostly regresses to
        // the mean once there are more than a handful of plates (the law of large
        // numbers working against the very unevenness it is meant to produce).
        // Discrete classes with fixed proportions guarantee the shape survives
        // whatever the plate count: SIZE_CLASS_WEIGHTS[SIZE_CLASS_PROPORTIONS]
        // pairs giant (10%) / large (25%) / medium (40%) / small (25%) plates,
        // Earth's own rough mix of a Pacific-scale giant, several Africa/
        // Eurasia-scale large plates, and a crowd of Nazca/Caribbean-scale small
        // ones. `size_weight` feeds `warped_voronoi_weighted` (a multiplicatively
        // weighted Voronoi), so RELATIVE weight is all that matters — the
        // absolute numbers are chosen so the extremes differ by the ~16-25×
        // AREA ratio the gate checks for, not by eye.
        let class_roll = rng.gen::<f32>();
        let mut acc = 0.0f32;
        let mut size_weight = *SIZE_CLASS_WEIGHTS.last().unwrap();
        for (&class_w, &class_p) in SIZE_CLASS_WEIGHTS.iter().zip(SIZE_CLASS_PROPORTIONS.iter()) {
            acc += class_p;
            if class_roll < acc { size_weight = class_w; break; }
        }
        plates.push(Plate {
            cx, cy, is_oceanic, density,
            pole_x, pole_y, omega, size_weight,
        });
    }

    // Assign cells to nearest plate (Voronoi) — but at a DOMAIN-WARPED sample
    // position, not the cell's own. Rayon-parallel: each cell's nearest-plate
    // scan is independent of every other cell's (§8.9 rule 2).
    //
    // ── WHY THE WARP IS HERE AND NOT DOWNSTREAM ──────────────────────────────
    // A plain Voronoi partition's every boundary is a straight line — the
    // perpendicular bisector of two seed points, exactly. `boundary_type` is
    // read off `plate_index`, the orogeny belt is a distance field from
    // `boundary_type`, and the coastline is a threshold on plate crust, so a
    // straight partition makes a straight mountain range, a straight rift and a
    // straight margin, all at once. Downstream passes each grew their OWN warp
    // to hide this (`elevation.rs`'s `oro_warp_*` warps the orogeny LOOKUP;
    // `warp_terrain_boundary` warps the coastline) — but warping a lookup only
    // bends where a straight line is SAMPLED FROM. The line is still there, and
    // each pass bends it differently, so the range, the rift and the coast stop
    // agreeing with each other about where the margin runs.
    //
    // Warping the PARTITION fixes all of them at once and keeps them consistent:
    // there is no straight line left anywhere downstream to bend, because the
    // plate margin itself is now an irregular curve. Real plate boundaries are
    // fractal at every scale (the Andean margin, the Mid-Atlantic Ridge's
    // transform-offset staircase) — which is what a multi-octave warp draws and
    // a single-octave one cannot.
    //
    // Amplitude is stated as a fraction of the mean PLATE SPACING, never in
    // cells (rule 25's spirit): the warp has to scale with the feature it
    // perturbs, or a 6-plate world is unrecognisably scrambled while a 40-plate
    // world is untouched. Kept well under 0.5 so the warp bends a margin without
    // detaching territory from its own plate.
    let seeds: Vec<(f32, f32)> = plates.iter().map(|p| (p.cx, p.cy)).collect();
    let weights: Vec<f32> = plates.iter().map(|p| p.size_weight).collect();
    // Part B1's own measured constraint: a WEIGHTED Voronoi already pulls a small
    // plate's boundary in tight against its bigger neighbours, so the SAME warp
    // amplitude that only bent an unweighted margin (§8.24b) now reaches deep
    // enough to sever pieces of it — `plate_territory_stays_connected` caught this
    // directly (worst connectivity fell to 80-85%, under the 90% bar). Rather than
    // widen the bar (which would readmit the exact phantom-boundary failure it
    // exists to catch, per §8.16), the weighted call uses its own, smaller
    // amplitude — still enough to keep every margin visibly non-straight, just not
    // enough to cut a small plate adrift.
    buf.plate_index = warped_voronoi_tuned_weighted(
        &seeds, &weights, buf.width, buf.height, count, seed,
        PLATE_WARP_AMP_FRAC_WEIGHTED, PLATE_WARP_WAVELENGTHS, PLATE_WARP_SIGMA_CLAMP);

    // Slice 5: reassign is_oceanic to hit `ocean_fraction` BY CONSTRUCTION, from
    // each plate's REAL cell count (measured from the Voronoi assignment just
    // computed, not an area estimate) — shuffled first so the greedy fill order
    // isn't "biggest plates always become ocean", then accumulated until the
    // target ocean area is met.
    //
    // B1's power-law size classes made a single greedy pass unreliable: this is
    // really a subset-sum/partition problem, and one shuffle order can get stuck
    // far from the target when a handful of plates carry very unequal weight
    // (`land_fraction_tracks_the_target` caught a 6-plate world landing at 52%
    // land against a 30% target — a >20pt miss). Trying several shuffle orders
    // and keeping the closest is the same greedy step, just no longer betting
    // the whole result on the first random order — still deterministic per seed
    // since every trial draws from the same seeded `rng` in sequence.
    let mut plate_cells = vec![0u32; plates.len()];
    for &pi in &buf.plate_index { plate_cells[pi as usize] += 1; }
    let target_ocean_cells = (ocean_fraction as f64 * buf.total() as f64).round() as i64;
    let mut best_oceanic = vec![false; plates.len()];
    let mut best_err = i64::MAX;
    const OCEAN_FILL_TRIALS: usize = 24;
    for _ in 0..OCEAN_FILL_TRIALS {
        let mut order: Vec<usize> = (0..plates.len()).collect();
        order.shuffle(&mut rng);
        let mut oceanic = vec![false; plates.len()];
        let mut ocean_cells_so_far: i64 = 0;
        for &pi in &order {
            let cells = plate_cells[pi] as i64;
            // Take this plate as oceanic if doing so gets closer to the target
            // than leaving it continental would.
            let with = (ocean_cells_so_far + cells - target_ocean_cells).abs();
            let without = (ocean_cells_so_far - target_ocean_cells).abs();
            if with <= without {
                oceanic[pi] = true;
                ocean_cells_so_far += cells;
            }
        }
        let err = (ocean_cells_so_far - target_ocean_cells).abs();
        if err < best_err {
            best_err = err;
            best_oceanic = oceanic;
        }
    }
    for (pi, p) in plates.iter_mut().enumerate() { p.is_oceanic = best_oceanic[pi]; }

    // Classify boundaries FIRST (needed by slice 4 below). Rayon-parallel:
    // each cell only reads plate_index (never boundary_type), so this map has
    // no cross-cell dependency.
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

                // Slice 4 — velocity evaluated AT THIS CELL's own position, not
                // at the plate centroid: the same boundary is strongly
                // convergent near one pole-relative bearing and obliquely
                // transform or divergent elsewhere along its length, exactly
                // how a real rotating margin varies.
                let (p1vx, p1vy) = plate_velocity_at(p1, x as f32, y as f32, w);
                let (p2vx, p2vy) = plate_velocity_at(p2, x as f32, y as f32, w);
                let rel_vx = p1vx - p2vx;
                let rel_vy = p1vy - p2vy;
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

    // Terrain 2.0 slice 4 (coastline decoupled from the plate Voronoi edge) +
    // volcanic-zone generation both live in `rasterize_landmass_and_volcanism`
    // now, shared with the plate-inspector's click-to-flip rebuild
    // (`rebuild_landmass_from_plate_types`) so the two can never drift apart —
    // the same "one copy, not two" discipline §8.18 applies to a colour ramp.
    let is_oceanic: Vec<bool> = plates.iter().map(|p| p.is_oceanic).collect();
    rasterize_landmass_and_volcanism(buf, seed, &mut rng, &is_oceanic);

    plates.iter().map(|p| PlateMotion {
        centroid_x: p.cx, centroid_y: p.cy,
        pole_x: p.pole_x, pole_y: p.pole_y, omega: p.omega,
        is_oceanic: p.is_oceanic,
    }).collect()
}

/// Re-rasterize landmass (terrain/elevation) and re-roll volcanic zones from a
/// per-plate oceanic/continental assignment, given plate GEOMETRY that is
/// already set (`buf.plate_index`/`buf.boundary_type`, persisted tile
/// columns). Boundary classification (convergent/divergent/transform) depends
/// only on the Euler-pole velocity field, never on which side is oceanic, so
/// it is never recomputed here — only which SIDE of an already-classified
/// boundary is land moves. Shared by initial generation
/// (`generate_plates_and_landmass_with_target`) and by the plate inspector's
/// click-to-flip rebuild (`rebuild_landmass_from_plate_types`), so the two
/// can never drift apart — the same failure mode §8.18 warns about for a
/// hand-copied colour table. `is_oceanic` is indexed by plate id exactly like
/// `buf.plate_index`.
fn rasterize_landmass_and_volcanism(
    buf: &mut WorldBuffer, seed: u64, rng: &mut StdRng, is_oceanic: &[bool],
) {
    let count = is_oceanic.len().max(1);
    let w = buf.width as f32;
    let h = buf.height as f32;
    let cols = (count as f32).sqrt().ceil() as usize;
    let rows = (count + cols - 1) / cols.max(1);
    let cell_w = w / cols.max(1) as f32;
    let cell_h = h / rows.max(1) as f32;

    // Provisional terrain straight from plate identity -- needed below to
    // know which SIDE of a boundary a cell started on before the noise
    // perturbation bends the actual coastline away from it.
    let orig_terrain: Vec<u8> = (0..buf.total())
        .into_par_iter()
        .map(|idx| {
            let pi = buf.plate_index[idx] as usize;
            if is_oceanic.get(pi).copied().unwrap_or(false) { 0u8 } else { 1u8 }
        })
        .collect();

    // ── Terrain 2.0 slice 4 (docs/CLAUDE.md §8.23b (Terrain 2.0, shipped) D1/T1): decouple the
    // coastline from the plate Voronoi edge. Two earlier passes on this
    // measured wrong: the first found the crust field genuinely differed but
    // the percentile threshold kept selecting the identical `terrain` (the
    // realised noise swing rarely bridged the base gap between an oceanic and
    // continental plate's crust value); the second widened the swing until
    // `terrain` genuinely moved, but at a single frequency uncorrelated with
    // WHERE the true boundary actually ran -- so it flipped scattered
    // isolated cells (speckle islands) far out on plates instead of bending
    // the coastline itself, since a boundary is a THIN, roughly 1-D curve and
    // a 2-D noise threshold has no notion of "near that curve".
    //
    // This pass fixes that by construction: it perturbs a SIGNED DISTANCE
    // TO THE NEAREST BOUNDARY (positive on the land side, negative on the
    // sea side) with noise, then re-thresholds at zero -- the level-set
    // technique real coastline generators use. Only cells within `REACH` of
    // an actual boundary can ever flip (a deep continental interior or open
    // ocean is never in play), and the noise frequency is tuned to complete
    // several cycles within that reach, so wherever a stretch of coast DOES
    // move, it moves as a coherent bulge (a peninsula or a bay), not a dot.
    let ls = ((cell_w + cell_h) * 0.5).max(4.0); // natural length scale: ~plate size
    let reach = (ls * 0.55).max(12.0);
    let reach_u16 = reach.ceil() as u16 + 4;

    // Distance (cells) to the nearest ANY-type boundary cell, full grid.
    let mut bnd_dist = vec![u16::MAX; buf.total()];
    {
        let mut q = VecDeque::new();
        for i in 0..buf.total() {
            if buf.boundary_type[i] != BOUNDARY_NONE {
                bnd_dist[i] = 0;
                q.push_back(i);
            }
        }
        while let Some(ci) = q.pop_front() {
            let d = bnd_dist[ci];
            if d >= reach_u16 { continue; }
            let cx = (ci % buf.width as usize) as i32;
            let cy = (ci / buf.width as usize) as i32;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = buf.wrap_x(cx + dx);
                let ny = (cy + dy).clamp(0, buf.height as i32 - 1) as u32;
                let ni = buf.idx(nx, ny);
                if bnd_dist[ni] > d + 1 {
                    bnd_dist[ni] = d + 1;
                    q.push_back(ni);
                }
            }
        }
    }

    // Broad wavelength (sweeping bulges) + a shorter one (headlands/inlets on
    // a bulge's own edge), both scaled to complete multiple cycles across
    // `reach` so any boundary stretch gets real, coherent wobble.
    let freq_a = 1.0 / (reach * 0.9);
    let freq_b = 1.0 / (reach * 0.28);
    let amp = reach * 1.7;
    let score: Vec<f32> = (0..buf.total())
        .into_par_iter()
        .map(|idx| {
            let d = bnd_dist[idx];
            let signed = if orig_terrain[idx] == 1 { d as f32 } else { -(d as f32) };
            if d as f32 > reach + 6.0 {
                // Well outside any boundary's reach: keep exactly as before,
                // no noise evaluation needed (and no risk of a stray flip).
                return signed;
            }
            let ax = (idx as u32 % buf.width) as f32;
            let ay = (idx as u32 / buf.width) as f32;
            let a = fbm_noise(ax * freq_a + 11.0, ay * freq_a + 4.0, seed.wrapping_add(0x5A17_0001), 3, 2.0, 0.5);
            let b = fbm_noise(ax * freq_b + 71.0, ay * freq_b + 29.0, seed.wrapping_add(0x5A17_0002), 3, 2.0, 0.5);
            let combined = a * 0.7 + b * 0.3;
            signed + amp * (combined - 0.5) * 2.0
        })
        .collect();

    let target_land: usize = orig_terrain.iter().filter(|&&t| t == 1).count();
    let mut sorted_score = score.clone();
    sorted_score.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending
    let threshold = if target_land == 0 {
        f32::INFINITY
    } else if target_land >= sorted_score.len() {
        f32::NEG_INFINITY
    } else {
        sorted_score[target_land - 1]
    };
    for i in 0..buf.total() {
        if score[i] >= threshold {
            buf.terrain[i] = 1;
            buf.elevation[i] = 0.05 + rng.gen::<f32>() * 0.05; // low base elevation
        } else {
            buf.terrain[i] = 0;
        }
    }
    // A safety net, not the mechanism: the level-set construction above
    // can't produce a far-flung speckle island (only cells within `reach` of
    // a real boundary are ever eligible to flip), but a very tight noise
    // trough can still pinch off a handful of cells right at a headland.
    // Flip anything under `DESPECKLE_MIN` back to its surroundings.
    despeckle_terrain(buf, DESPECKLE_MIN);

    // Reset volcanic flags on boundary cells before re-rolling, so a REBUILD
    // (a plate flipped, this function run a second time on the same world) is
    // idempotent rather than only ever accumulating more volcanic cells from
    // stacked rolls under different assignments. Never touches a volcanic
    // cell placed away from a plate margin (a lasso Arc island chain, §8.25),
    // since those never carry a CONVERGENT/DIVERGENT boundary_type.
    for i in 0..buf.total() {
        if buf.boundary_type[i] == BOUNDARY_DIVERGENT || buf.boundary_type[i] == BOUNDARY_CONVERGENT {
            buf.is_volcanic[i] = 0;
        }
    }

    // Generate volcanic zones at divergent boundaries, and at convergent ones
    // by COLLISION TYPE (WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 4, scoped-down:
    // real collision-type differentiation, without the full persisted per-plate
    // identity / Euler-pole rewrite the plan's F5 root cause needs -- that is a
    // much larger change, deliberately not attempted this session). A
    // continent-continent collision (Himalaya-style) raises a broad plateau with
    // essentially NO volcanism; ocean-ocean (island arc) and ocean-continent
    // (arc + trench) margins are exactly where real subduction volcanism
    // concentrates. Before this every convergent cell got the identical flat 8%
    // roll regardless of what was actually colliding.
    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            if buf.boundary_type[idx] == BOUNDARY_DIVERGENT {
                if rng.gen::<f32>() < 0.15 {
                    buf.is_volcanic[idx] = 1;
                }
            }
            if buf.boundary_type[idx] == BOUNDARY_CONVERGENT {
                let my_plate = buf.plate_index[idx] as usize;
                let my_oceanic = is_oceanic.get(my_plate).copied().unwrap_or(false);
                let mut touches_oceanic = my_oceanic;
                let mut touches_continental = !my_oceanic;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ny = (y as i32 + dy).clamp(0, buf.height as i32 - 1) as u32;
                    let np = buf.plate_index[buf.idx(nx, ny)] as usize;
                    if np == my_plate || np >= is_oceanic.len() { continue; }
                    if is_oceanic[np] { touches_oceanic = true; } else { touches_continental = true; }
                }
                let chance = if touches_continental && !touches_oceanic {
                    0.01 // continent-continent: broad collisional plateau, ~no volcanism
                } else if touches_oceanic {
                    0.14 // ocean-ocean island arc / ocean-continent arc+trench
                } else {
                    0.08
                };
                if rng.gen::<f32>() < chance {
                    buf.is_volcanic[idx] = 1;
                }
            }
        }
    }
}

/// The plate inspector's click-to-flip rebuild (`sim_set_plate_oceanic`): keep
/// the SAME plate geometry (`plate_index`/`boundary_type`, already persisted —
/// no re-partition, no re-roll of plate poles/positions) and re-rasterize
/// landmass from an EDITED per-plate oceanic/continental assignment.
/// `generate_plates_and_landmass` already decides `is_oceanic` per plate to
/// hit a target ocean fraction (Slice 5); this exposes that decision so a
/// specific plate can override it after the fact. `is_oceanic` must be
/// indexed the same way `buf.plate_index` is — i.e. the same order
/// `PlateMotion` was returned in; the caller reads and rewrites the persisted
/// `metadata["plate_motion"]` list to keep both in step.
pub fn rebuild_landmass_from_plate_types(buf: &mut WorldBuffer, seed: u64, is_oceanic: &[bool]) {
    // A fresh RNG stream, deliberately NOT a continuation of generation's own
    // (that stream is long gone by the time a rebuild runs, and its position
    // depended on plate_count/ocean-fill-trial draws a rebuild never repeats)
    // — salted so it can never coincidentally replay the same draws generation
    // made at some other offset.
    let mut rng = StdRng::seed_from_u64(seed ^ 0xB1A5_5EED_u64);
    rasterize_landmass_and_volcanism(buf, seed, &mut rng, is_oceanic);
}

/// Below this many connected cells, a land or sea patch reads as noise dust
/// rather than a real feature (Terrain 2.0 slice 4's despeckle pass).
const DESPECKLE_MIN: usize = 14;

/// Flip any 4-connected land or sea component smaller than `min_size` cells to
/// its opposite value -- the level-set noise construction (see the "score"
/// pass above) can pinch off a handful of cells right at a tight headland; a
/// genuine island or inlet at or above `min_size` is untouched.
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
        generate_plates_and_landmass(&mut buf, seed, plate_count);
        buf
    }

    /// WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 4 — the real claim: an Euler
    /// pole makes a boundary's CHARACTER vary along its length, unlike a single
    /// translation vector (which classifies an entire shared boundary as
    /// uniformly convergent/divergent/transform, start to end, by construction —
    /// `rel_vx`/`rel_vy` was one constant number for the whole pair). Measured
    /// directly on `boundary_type`: for at least one plate pair with a real
    /// multi-cell shared boundary, the classification is NOT the same at every
    /// cell of it. A flat-vector model fails this by construction; it cannot
    /// pass by accident, because a constant relative-velocity vector dotted
    /// against a per-cell normal only changes classification where the BISECTOR
    /// direction itself bends (rare on a Voronoi edge, and never the mechanism
    /// this gate is checking for).
    #[test]
    fn boundary_character_varies_along_its_length() {
        let mut found_varying_pair = false;
        for seed in 0..8u64 {
            let buf = gen_world(360, 180, seed, 10);
            // (plate_a, plate_b) -> set of boundary types seen along their shared edge.
            let mut pair_types: std::collections::HashMap<(u16, u16), std::collections::HashSet<u8>> =
                std::collections::HashMap::new();
            for y in 0..buf.height {
                for x in 0..buf.width {
                    let idx = buf.idx(x, y);
                    let bt = buf.boundary_type[idx];
                    if bt == BOUNDARY_NONE { continue; }
                    let my_plate = buf.plate_index[idx];
                    let nx = buf.wrap_x(x as i32 + 1);
                    let np = buf.plate_index[buf.idx(nx, y)];
                    if np != my_plate {
                        let key = (my_plate.min(np), my_plate.max(np));
                        pair_types.entry(key).or_default().insert(bt);
                    }
                    if y + 1 < buf.height {
                        let np2 = buf.plate_index[buf.idx(x, y + 1)];
                        if np2 != my_plate {
                            let key = (my_plate.min(np2), my_plate.max(np2));
                            pair_types.entry(key).or_default().insert(bt);
                        }
                    }
                }
            }
            if pair_types.values().any(|types| types.len() >= 2) {
                found_varying_pair = true;
                break;
            }
        }
        assert!(found_varying_pair,
            "expected at least one plate pair, across 8 seeds, whose shared boundary \
             shows more than one boundary_type along its length — an Euler-pole \
             velocity field should produce this; a flat per-plate vector cannot");
    }

    /// The Earth-parameter no-op discipline (rule 10) doesn't apply to plate
    /// generation directly, but DETERMINISM does: the same seed must reproduce
    /// the identical plate layout and boundary classification (the pole/omega
    /// draw uses the same seeded RNG stream as everything else here).
    #[test]
    fn plate_generation_is_deterministic() {
        let a = gen_world(200, 100, 12345, 8);
        let b = gen_world(200, 100, 12345, 8);
        assert_eq!(a.terrain, b.terrain);
        assert_eq!(a.boundary_type, b.boundary_type);
        assert_eq!(a.plate_index, b.plate_index);
    }

    /// LOCAL STRAIGHTNESS of a partition's margins, in 0..1 — the metric this
    /// gate turns on, and the second one tried.
    ///
    /// NEGATIVE RESULT, recorded so it is not attempted again (§2.4): the first
    /// version measured TOTAL BOUNDARY LENGTH, on the reasoning that a straight
    /// bisector is the shortest curve between two triple junctions so curvature
    /// can only lengthen it. That is true of one segment and false of a
    /// partition: a warp that bows one margin outward bows its neighbour inward
    /// by the same amount, so the total is very nearly conserved. Measured, the
    /// warped partition came out at 1.00× the unwarped one (5682 vs 5672 cells)
    /// while the margins were visibly, strongly curved. A global length metric
    /// cannot see this; the question is local.
    ///
    /// So: for each boundary cell, take the boundary cells within a disc of
    /// radius R and compare the SPAN (greatest distance between any two of them)
    /// to how many there are. A straight margin puts ~2R+1 cells in the disc
    /// spanning ~2R, scoring ~1.0. A margin that wanders inside the same disc
    /// packs in more cells for the same span, scoring lower. Returns the mean
    /// over a sample of boundary cells.
    fn margin_straightness(index: &[u16], w: u32, h: u32, r: i32) -> f32 {
        let R: i32 = r;
        let idx = |x: u32, y: u32| (y * w + x) as usize;
        let wrap = |x: i32| ((x % w as i32) + w as i32) % w as i32;
        let is_boundary = |x: u32, y: u32| -> bool {
            let me = index[idx(x, y)];
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let nx = wrap(x as i32 + dx) as u32;
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                if index[idx(nx, ny as u32)] != me { return true; }
            }
            false
        };
        let (mut sum, mut n) = (0.0f32, 0usize);
        // Sample every 3rd row/col: the measure is a mean, and a full scan of a
        // disc per boundary cell is O(area · R²) for no extra signal.
        let mut y = R as u32;
        while y + (R as u32) < h {
            let mut x = 0u32;
            while x < w {
                if is_boundary(x, y) {
                    let mut pts: Vec<(f32, f32)> = Vec::new();
                    for dy in -R..=R {
                        for dx in -R..=R {
                            if dx * dx + dy * dy > R * R { continue; }
                            let nx = wrap(x as i32 + dx) as u32;
                            let ny = y as i32 + dy;
                            if ny < 0 || ny >= h as i32 { continue; }
                            if is_boundary(nx, ny as u32) {
                                pts.push((dx as f32, dy as f32));
                            }
                        }
                    }
                    if pts.len() >= 4 {
                        let mut span = 0.0f32;
                        for i in 0..pts.len() {
                            for j in (i + 1)..pts.len() {
                                let d = ((pts[i].0 - pts[j].0).powi(2)
                                    + (pts[i].1 - pts[j].1).powi(2)).sqrt();
                                if d > span { span = d; }
                            }
                        }
                        sum += span / pts.len() as f32;
                        n += 1;
                    }
                }
                x += 3;
            }
            y += 3;
        }
        if n == 0 { 0.0 } else { sum / n as f32 }
    }

    /// THE STRAIGHT-MOUNTAIN GATE. A plain Voronoi partition's boundaries are
    /// straight bisectors BY CONSTRUCTION, and every downstream landform is
    /// derived from them: `boundary_type` is read off `plate_index`, the orogeny
    /// belt is a distance field from `boundary_type`, and `deposits.rs` reads the
    /// same column as tectonic setting. A straight partition therefore draws a
    /// straight mountain range, a straight rift and a straight ore belt at once —
    /// the artefact this warp exists to remove.
    ///
    /// Compares the shipped warp against `warp_frac = 0.0` on the SAME plate
    /// seeds — the genuine pre-warp behaviour, not a reconstruction — so the
    /// gate cannot pass by drifting out of step with the generator.
    #[test]
    fn plate_margins_are_not_straight_bisectors() {
        // Radius must be comparable to a PLATE, not to a cell. The first version
        // used a fixed 6-cell disc and measured 0.504 → 0.484 on a warp that was
        // plainly bending the margins: at 6 cells against an ~80-cell plate
        // spacing, a margin curving over its whole length still looks locally
        // straight. This is the same class of mistake as the boundary-length
        // metric it replaced — measuring at the wrong scale, not measuring the
        // wrong thing.
        const R: i32 = 20;
        const SEEDS: u64 = 4;
        let (mut straight_sum, mut warped_sum) = (0.0f32, 0.0f32);
        for seed in 0..SEEDS {
            let (plain, w, h, _) = partition_for(seed, 10, 360, 180, 0.0, 1.0, 1.0);
            let (warped, _, _, _) = partition_for(
                seed, 10, 360, 180,
                PLATE_WARP_AMP_FRAC, PLATE_WARP_WAVELENGTHS, PLATE_WARP_SIGMA_CLAMP);
            straight_sum += margin_straightness(&plain, w, h, R);
            warped_sum += margin_straightness(&warped, w, h, R);
        }
        let straight = straight_sum / SEEDS as f32;
        let warped = warped_sum / SEEDS as f32;
        println!("margin straightness @R={}: plain Voronoi {:.3} → warped {:.3}",
                 R, straight, warped);
        assert!(warped < straight * 0.93,
            "plate margins are still essentially straight bisectors: local \
             straightness only fell from {:.3} (plain Voronoi) to {:.3} (warped) \
             over {} seeds. If the warp is not reaching plate_index, every \
             downstream feature — orogeny belt, rift, coastline, ore setting — is \
             still being drawn along a straight line.",
            straight, warped, SEEDS);
    }

    /// THE SWEEP that set `PLATE_WARP_AMP_FRAC`, `PLATE_WARP_WAVELENGTHS` and
    /// `PLATE_WARP_SIGMA_CLAMP` (§2.4: never tune a constant without a gate that
    /// is not the target). Prints, for each candidate, how much local margin
    /// straightness FALLS (the goal) against the worst plate connectivity it
    /// leaves behind (the constraint). The shipped triple is the biggest
    /// straightness drop whose connectivity still clears the 0.90 bar.
    ///
    /// `#[ignore]`d: it is a measurement instrument, not a gate.
    #[test]
    #[ignore]
    fn diag_sweep_plate_warp() {
        println!("{:>6} {:>6} {:>6}  {:>10} {:>10}", "amp", "wav", "clamp", "straight", "worst_conn");
        for &amp in &[0.0f32, 0.25, 0.35, 0.45, 0.60, 0.80] {
            for &wav in &[0.8f32, 1.2, 1.7, 2.5] {
                for &clamp in &[0.8f32, 1.2, 2.0] {
                    let (mut str_sum, mut worst_conn) = (0.0f32, 1.0f32);
                    const SEEDS: u64 = 3;
                    for seed in 0..SEEDS {
                        let (idx, w, h, _) = partition_for(seed, 10, 360, 180, amp, wav, clamp);
                        str_sum += margin_straightness(&idx, w, h, 20);
                        let c = worst_plate_connectivity(&idx, w, h);
                        if c < worst_conn { worst_conn = c; }
                    }
                    println!("{:>6.2} {:>6.2} {:>6.2}  {:>10.3} {:>10.3}",
                             amp, wav, clamp, str_sum / SEEDS as f32, worst_conn);
                }
            }
        }
    }

    /// Rebuild one world's plate SEED POINTS exactly as the generator draws them,
    /// then partition at an arbitrary (amp, wavelength, clamp). Shared by the
    /// sweep and by the straightness gate, so both measure the same thing.
    fn partition_for(
        seed: u64, count: usize, width: u32, height: u32,
        amp: f32, wav: f32, clamp: f32,
    ) -> (Vec<u16>, u32, u32, Vec<(f32, f32)>) {
        let mut rng = StdRng::seed_from_u64(seed);
        let (w, h) = (width as f32, height as f32);
        let cols = (count as f32).sqrt().ceil() as usize;
        let rows = (count + cols - 1) / cols;
        let (cell_w, cell_h) = (w / cols as f32, h / rows as f32);
        let mut seeds: Vec<(f32, f32)> = Vec::with_capacity(count);
        for i in 0..count {
            let cx = ((i % cols) as f32 + rng.gen::<f32>()) * cell_w;
            let cy = ((i / cols) as f32 + rng.gen::<f32>()) * cell_h;
            // Consume exactly the draws the generator makes per plate so the
            // seed positions match: is_oceanic, density, pole_angle, pole_dist,
            // speed, omega sign.
            let _ = rng.gen::<f32>();
            let _ = rng.gen::<f32>();
            let _ = rng.gen::<f32>();
            let _ = rng.gen::<f32>();
            let _ = rng.gen::<f32>();
            let _ = rng.gen::<bool>();
            seeds.push((cx, cy));
        }
        let idx = warped_voronoi_tuned(&seeds, width, height, count, seed, amp, wav, clamp);
        (idx, width, height, seeds)
    }

    /// Smallest "largest connected component / total area" ratio over all plates
    /// — the shredding measure `plate_territory_stays_connected` asserts on.
    fn worst_plate_connectivity(index: &[u16], w: u32, h: u32) -> f32 {
        let idx = |x: u32, y: u32| (y * w + x) as usize;
        let wrap = |x: i32| ((x % w as i32) + w as i32) % w as i32;
        let total = index.len();
        let mut seen = vec![false; total];
        let mut largest: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
        let mut area: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
        for &p in index { *area.entry(p).or_insert(0) += 1; }
        for y in 0..h {
            for x in 0..w {
                let start = idx(x, y);
                if seen[start] { continue; }
                let plate = index[start];
                let mut size = 0usize;
                let mut q = VecDeque::new();
                q.push_back((x, y));
                seen[start] = true;
                while let Some((cx, cy)) = q.pop_front() {
                    size += 1;
                    for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let nx = wrap(cx as i32 + dx) as u32;
                        let ny = cy as i32 + dy;
                        if ny < 0 || ny >= h as i32 { continue; }
                        let ni = idx(nx, ny as u32);
                        if seen[ni] || index[ni] != plate { continue; }
                        seen[ni] = true;
                        q.push_back((nx, ny as u32));
                    }
                }
                let e = largest.entry(plate).or_insert(0);
                if size > *e { *e = size; }
            }
        }
        area.iter()
            .map(|(p, &a)| largest[p] as f32 / a as f32)
            .fold(1.0f32, f32::min)
    }

    /// THE SHREDDING GATE, on the REAL production path (`generate_plates_and_
    /// landmass`, which now assigns `plate_index` through `warped_voronoi_
    /// weighted` — Part B1). A weighted Voronoi is exactly the kind of change
    /// that risks the failure `PLATE_WARP_AMP_FRAC` was tuned against: warp a
    /// sample far enough AND let a giant plate's wide capture radius pull it
    /// across a neighbour's territory, and a small plate can be reduced to
    /// scattered specks inside a big one — which `boundary_type` then reads as
    /// phantom plate boundaries scattering ore districts through a plate
    /// interior (§8.16's own failure mode). Same 0.90 bar the unweighted warp
    /// was held to; B1 must not spend that margin.
    #[test]
    fn plate_territory_stays_connected() {
        for seed in 0..3u64 {
            let buf = gen_world(240, 120, seed, 8);
            let conn = worst_plate_connectivity(&buf.plate_index, buf.width, buf.height);
            assert!(conn > 0.90,
                "seed {seed}: some plate's largest connected piece is only {:.0}% of \
                 its total area — the weighted warp is shredding a plate into \
                 detached specks.", conn * 100.0);
        }
    }

    /// TECTONICS_AND_ISOLATION_PLAN.md Part B1 — plates of GENUINELY different
    /// size, the way Earth's Pacific plate (~103M km²) dwarfs its Juan de Fuca
    /// (~0.25M km²). The old jittered-grid seeding made every plate roughly the
    /// same size by construction (one grid cell of territory each), which this
    /// gate would fail outright — it is measuring a real, previously-absent
    /// property, not tuning an existing one.
    #[test]
    fn plate_sizes_span_an_order_of_magnitude() {
        let mut ratios: Vec<f32> = Vec::new();
        for seed in 0..5u64 {
            let buf = gen_world(360, 180, seed, 14);
            let mut area: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
            for &p in &buf.plate_index { *area.entry(p).or_insert(0) += 1; }
            let mut areas: Vec<usize> = area.values().copied().collect();
            areas.sort_unstable();
            if areas.len() < 4 { continue; }
            let median = areas[areas.len() / 2] as f32;
            let largest = *areas.last().unwrap() as f32;
            if median > 0.0 { ratios.push(largest / median); }
        }
        let mean_ratio = ratios.iter().sum::<f32>() / ratios.len().max(1) as f32;
        println!("largest-plate / median-plate area ratio, mean over {} seeds: {:.2}×",
                 ratios.len(), mean_ratio);
        assert!(mean_ratio >= 5.0,
            "the largest plate is only {:.2}× the median plate's area, averaged over \
             {} seeds — plates still read as roughly uniform in size, which is exactly \
             what the old jittered-grid seeding produced and this gate exists to catch.",
            mean_ratio, ratios.len());
    }

    /// The plate inspector's click-to-flip rebuild must be deterministic per
    /// (seed, assignment) — the same "same seed, same result" discipline rule
    /// 10 asks of every other world-mutating entry point.
    #[test]
    fn rebuild_landmass_is_deterministic() {
        let mut buf_a = gen_world(240, 120, 55, 8);
        let mut buf_b = gen_world(240, 120, 55, 8);
        let oceanic = vec![true, false, true, false, true, false, true, false];
        rebuild_landmass_from_plate_types(&mut buf_a, 55, &oceanic);
        rebuild_landmass_from_plate_types(&mut buf_b, 55, &oceanic);
        assert_eq!(buf_a.terrain, buf_b.terrain);
        assert_eq!(buf_a.is_volcanic, buf_b.is_volcanic);
    }

    /// Flipping one plate's oceanic/continental assignment must actually move
    /// the map by roughly that plate's own share, not just re-dress the noise
    /// at existing boundaries. Measures the AGGREGATE land swing rather than
    /// per-cell interior geometry (a small/oddly-shaped plate can sit entirely
    /// inside the coastline noise band's `reach`, so "does every interior cell
    /// flip" is not a robust per-plate claim — "does total land move by
    /// roughly the flipped plate's cell count" is, and is exactly the
    /// mechanism (`target_land` recomputed from the edited assignment)).
    #[test]
    fn flipping_a_plate_changes_the_land_area_it_should() {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", "240".to_string()), ("grid_height", "120".to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        let motion = generate_plates_and_landmass(&mut buf, 77, 8);

        let mut counts: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
        for &p in &buf.plate_index { *counts.entry(p).or_insert(0) += 1; }
        let (&target, &target_cells) = counts.iter().max_by_key(|(_, &c)| c).unwrap();

        let land_before: usize = buf.terrain.iter().filter(|&&t| t == 1).count();
        let mut oceanic: Vec<bool> = motion.iter().map(|p| p.is_oceanic).collect();
        oceanic[target as usize] = !oceanic[target as usize];

        rebuild_landmass_from_plate_types(&mut buf, 77, &oceanic);
        let land_after: usize = buf.terrain.iter().filter(|&&t| t == 1).count();

        // `target_land` (the count `rasterize_landmass_and_volcanism` thresholds
        // to) is a direct sum of each CONTINENTAL plate's own cell count, so
        // flipping one plate moves it by exactly that plate's cell count —
        // land does not also grow somewhere else to compensate, since "sea" is
        // simply "not currently continental". The coastline noise band nudges
        // the realised swing by a few cells either way at plate margins.
        let expected_swing = target_cells as f64;
        let actual_swing = (land_after as f64 - land_before as f64).abs();
        assert!(actual_swing > expected_swing * 0.9,
            "flipping plate {target} ({target_cells} cells, {:.1}% of the world) only \
             moved total land by {actual_swing} cells (expected roughly {expected_swing:.0}) \
             — the rebuild does not appear to be responding to the edited assignment",
            target_cells as f32 / buf.total() as f32 * 100.0);
    }
}
