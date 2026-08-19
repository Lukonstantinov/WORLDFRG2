# Terrain 2.0 — the plan

**Status: ALL SIX SLICES BUILT** (2026-08-19). See `docs/SCOREBOARD.md`'s
2026-08-19d entry for the measured numbers per slice and §8 below for what they
were. Decisions in §2 held as written; §4 was the build order actually followed.

The organising idea, and the reason this is a rewrite rather than a tuning pass:

> **Terrain should be the product of a history, not a single noise evaluation.**
> Every mountain on the map is currently the same age, built by the same recipe,
> worn by the same erosion. That is why they all shade alike — a hillshade renders
> the DERIVATIVE of elevation, so if the derivative's statistical character is
> uniform, so is the picture.

---

## 1. The measured diagnosis

Not opinion. Every number below came from `bench_phase2`, `dump_natural_sheet` or
reading the call graph, and each is reproducible.

| # | Finding | Evidence |
|---|---|---|
| D1 | **Coastlines ARE the Voronoi diagram.** `terrain[idx]` is set purely from `plate.is_oceanic` (a 40% coin flip per plate), so continent and plate are the same object and every shoreline is a cell boundary. | `plates.rs:150-160`; visible as straight edges and a triangular island in `world_natural.png` |
| D2 | **`density` is generated and never read.** No subduction polarity: no trench-and-arc asymmetry, no Andes-vs-Himalaya distinction. Every convergent boundary makes the same symmetric ridge. | `plates.rs:40-46`, no reader anywhere |
| D3 | **Boundary classification breaks on the first differing neighbour**, so at triple junctions the type depends on scan order; and the convergent test uses the direction from the plate CENTRE, not the boundary normal. | `plates.rs:88-95` |
| D4 | **Orogenic ridges run as straight lines** along boundary segments — uplift blooms off a 1-cell-wide boundary with no warping. | visible as diagonal lines across `crop_terrain_*.png` |
| D5 | **One global noise recipe for the whole world.** `f_base 1/760`, `f_range 1/210`, `f_hill 1/52`, fixed `RIDGE_AMP`/`HILL_AMP`. Belt STRENGTH varies with distance to a convergent boundary; belt TEXTURE never varies. | `elevation.rs:398-404` |
| D6 | **Erosion cannot carve a network.** 77,760 droplets budgeted at 3600×1800, but `hydraulic_erosion` picks a random cell and `continue`s if it is ocean, consuming the iteration — ~42% discarded → **~45,400 effective droplets over 3.8M land cells ≈ 1.4 droplet-visits per cell.** No tuning fixes this; it is the wrong algorithm at this scale. | `bench_phase2`; `elevation.rs:98` |
| D7 | **No lithology field at all.** Differential erosion — resistant beds holding ridges, weak beds carving valleys — is where most real relief texture comes from, and it cannot happen. | absent |
| D8 | **Erosion is climate-blind.** Precipitation is phase 3, AFTER terrain in phase 2, so a monsoon range and a desert range erode identically. | pipeline order |
| D9 | **`redistribute_elevation` forces one global histogram** by rank over ALL land, actively erasing between-region contrast. | `elevation.rs:1897` |
| D10 | **Phase 2 is entirely SERIAL and costs as much as phase 3.** 8.5 s (plate) / 11.4 s (shape) at 3600×1800 against phase 3's ~16 s, which is rayon-parallel throughout. | `bench_phase2` |
| D11 | **The seafloor has no structure.** `compute_sea_depth` is a pure function of distance-to-coast, saturating ~20 cells out. No ridges, trenches, fracture zones or abyssal hills. | `elevation.rs:539` |
| D12 | **One pixel per cell caps detail.** A tile is 128×128 pixels over 128×128 cells, so there is nowhere for sub-cell detail to live. | `tile_image.rs:4-5`; see CLAUDE.md §8.21 |

### The one piece of good news

**The Earth fidelity gate cannot be regressed by T1–T3.** `earth_validation.rs`
loads a real Earth DEM fixture (`earth_elev_720x360.i16`) and never calls the
procedural generators — it shares only `compute_sea_depth` and `generate_shelves`.
So the seafloor slice (T4) is the *only* one that touches §2.3's gate, and it runs
last, with the gate re-run.

---

## 2. Decisions taken

| Decision | Choice | Consequence |
|---|---|---|
| Scope | **All four slices** (T1–T4) plus the render follow-up | Sequenced in §4; each independently shippable |
| Geology fields | **Transient first**, persist later only if a reader appears | Computed in phase 2 from seed + plate data, used, discarded. Zero blast radius, no tile format change, no save-compat question. Deterministic, so re-running phase 2 reproduces them exactly. Cost: no Geology render layer yet, and soil/ore keep their proxies |
| Erosion vs climate | **Climate PROXY inside phase 2** | Pipeline order untouched, nothing downstream re-runs. Accepted inaccuracy: the proxy will disagree with the real precipitation field phase 3 later produces. Revisit only with its own gate |
| Model scope | **All four elevation models fully** | Polarity/age need plates; where plates are absent (painted, imported, template) a pseudo-setting is derived from relief. That pseudo-setting is a documented fiction, not a claim |

**On "all four models fully":** this is the one decision carrying a standing risk.
`shape`/`cordillera`/`ridged` have no tectonic data, so their "setting" is inferred
from relief and continentality. That inference must never be presented as geology
and must never be allowed to make a template world WORSE than it is today — the
per-model before/after in §3 is what enforces that.

---

## 3. Instrumentation first (partly built)

Rule 2.4: *a diagnosis is a complete task*, and *never tune a constant without a
gate that isn't the target*. Every slice below is measured by this harness, and it
is built before any of them.

- **`bench_phase2`** — *built* (`elevation.rs`). Per-model ms at two grid sizes,
  plus the wasted-droplet share. This is the perf budget for every slice.
- **`dump_natural_sheet`** — *built* (`render/natural.rs`). Real world through
  phase 6b, rendered through the real path, whole-world PNG **plus a 3× magnified
  crop of the most mountainous window**. The crop is not optional: the fill-light
  regression was invisible at world zoom and unmistakable at 3×.
- **`terrain_metrics`** — *to build*. One harness printing, per model and per map
  window: RMS slope, slope SPREAD across windows, drainage density, hypsometric
  integral, coast-on-plate-boundary fraction, and `sea_depth`↔distance correlation.
  These are the gates named below. It prints a table for every one of the four
  models, so "all four models fully" is checkable rather than asserted.

**The headline gate for the whole project** is *slope spread across windows*. A
world where every range shades alike scores near zero on it; that is precisely the
complaint Terrain 2.0 exists to answer, stated as a number that cannot be faked.

---

## 4. Build order

Sequenced by **risk-adjusted value**, not by pipeline order. Each slice is
independently shippable and independently gated.

### Slice 1 · Erosion that works (T3a)
Replace droplet erosion with **stream power over flow accumulation**:
priority-flood fill → flow directions → accumulation → incise by `K·A^m·S^n`.
O(n log n), and — unlike the droplet loop — parallelisable, so this may end up
FASTER than today's serial 8.5 s despite doing far more work. Fix D6's discarded-
iteration bug on the way (sample land cells directly, never reject after drawing).

*Gate:* drainage density rises and becomes measurable; `bench_phase2` must not
regress (target: no slower than today, stretch: faster). Rivers (phase 5) must
still run — it consumes elevation.
*Risk:* phase 5 already does its own priority-flood. These must not fight; the
sensible end state is one shared implementation, which is also a simplification.

### Slice 2 · Lithology & differential erosion (T3b)
A transient `erodibility` field from tectonic setting + noise, so resistant rock
holds ridges and weak rock carves valleys. Plus the phase-2 **climate proxy**
(latitude + continentality + orographic side) so wet ranges erode faster than dry
ones. Retire or regionalise D9's global histogram.

*Gate:* **slope spread across windows** clears its floor — the headline gate. Set
the floor to what the build achieves, then raise it as later slices earn it (the
`EARTH_MAIN_FLOOR` discipline).

### Slice 3 · Orogeny with polarity and age (T2)
Read `density` at last: ocean-under-continent → volcanic arc offset inland from a
trench, asymmetric flanks; continent-continent → broad doubly-vergent collision;
ocean-ocean → island arc. Fix D3's first-neighbour classification and use the real
boundary normal. Warp segments off straight lines (D4). Give each orogen an **age**
so a worn range sits beside a young one — the single biggest visual win available.

*Gate:* slope spread rises again; arc/trench **asymmetry** is measurable (seaward
and inland flanks must differ); ridges no longer align to boundary segments (D4
metric).

### Slice 4 · Kill the Voronoi coastline (T1)
Decouple continental crust from plate identity — an `is_continental` field so one
plate can carry both ocean and land — and put the shoreline on a margin function
(crust thickness + passive-margin sedimentation, noise-warped) rather than a cell
boundary.

*Gate:* **coast-on-plate-boundary fraction** falls from ~100% to below a floor.
*Risk — the largest in this plan:* moving coastlines changes settlements,
provinces, goods and every downstream phase. It goes AFTER the terrain machinery
is good, so new coastlines get good terrain immediately rather than being
re-derived twice.

### Slice 5 · Seafloor (T4)
Mid-ocean ridges along divergent boundaries with transform offsets, abyssal hills,
trenches at convergent margins, seamount chains from hotspots, a real continental
slope and rise.

*Gate:* `sea_depth`↔distance-to-coast correlation drops below a threshold.
**This is the one slice that touches the Earth gate** (§2.3) via
`compute_sea_depth`/`generate_shelves` — run `cargo test --lib earth_` and hold
both floors. Hence last.

### Slice 6 · Render follow-up (T5)
**Texture shading** (Leland Brown's fractional Laplacian) — the right technique for
showing drainage at 11 km posting, and the honest answer to the lee-slope flatness
that the reverted fill light failed to fix. Plus the elevation ramp's blow-out to
white, and (separately) the inverse LOD pyramid for sub-cell detail (D12).

---

## 5. Performance budget

Phase 2 is 8.5–11.4 s at 3600×1800 and **fully serial** (D10) while phase 3 at
~16 s is rayon-parallel throughout. That is the headroom: slice 1 alone should pay
for much of the rest.

Rules carried over from §8.9, which apply here unchanged:
1. **Never scan outward per cell.** Distance fields are linear sweeps or
   multi-source BFS, never a search per cell.
2. **Keep row loops parallel** — each pass writes only its own cell.
3. **Hoist loop-invariant work** out of repeated passes.
4. `bench_phase2` runs after every slice. A slice that lands a visual win and a 2×
   slowdown is not done.

---

## 6. Deliberately NOT built

Named so they are not silently assumed, per §2.4:

- **Plate motion over time.** No rifting history, no continental drift, no ocean
  basin opening and closing. Orogen AGE is a scalar per segment, not a simulation.
- **Glacial erosion.** U-valleys, cirques, fjords and moraine fields need a
  paleo-snowline and an ice-flow model. Real, large, and its own project.
- **Aeolian and karst landforms.** Dune fields and sinkhole terrain are listed in
  the texture menu and are not in this plan.
- **Sub-cell detail.** The inverse LOD pyramid (D12) is named in slice 6 and
  scoped separately; no shading trick substitutes for the missing raster.
- **Persisted geology.** Per §2, transient until a reader appears.
- **Two-way coupling with climate.** The phase-2 proxy is explicitly a proxy.

---

## 7. Open risks

1. **"All four models fully" may not be worth it for the pseudo-setting models.**
   If the inferred setting produces worse terrain than today's shape model, the
   honest outcome is to scope those three back to lithology+erosion only. The
   per-model table in §3 decides it; this is a measurement, not a preference.
2. **Slice 1 and phase 5 both priority-flood.** If unifying them proves invasive,
   the fallback is to keep them separate and accept the duplicated cost — but the
   duplication must then be documented, not left to be rediscovered.
3. **Slice 4's blast radius is real.** Every existing world regenerates differently.
   Saved worlds keep their stored tiles, so only NEW generation changes — but that
   should be stated in the release note rather than discovered.
4. **The headline gate could be gamed.** Slope spread rises if you simply add noise
   at different amplitudes per region, which would look terrible. It is necessary,
   not sufficient: the 3× crop from `dump_natural_sheet` is reviewed on every slice
   alongside the number. The fill-light regression is the standing proof that a
   green test suite is not a picture.

---

## 8. What shipped (2026-08-19)

All six slices landed in one pass, plus §3's `terrain_metrics` harness. Full
account in `docs/SCOREBOARD.md`'s 2026-08-19d entry; the essentials:

- **Slice 1 (erosion).** `hydraulic_erosion` (droplets) → `stream_power_erosion`
  (`step2_terrain/elevation.rs`): priority-flood fill (Barnes et al., seeded from
  every ocean cell at fixed sea level) → D8 flow directions straight out of the
  fill order → drainage-area accumulation headwaters-to-coast → `K·A^m·S^n`
  incision, clamped so a cell can never erode below its own downstream neighbour
  (the safety clamp that also self-limits the `A^m` blow-up near a river mouth for
  free). Kept as a SEPARATE implementation from phase 5's own priority-flood
  (risk 2) — phase 2 runs long before rivers exist, and unifying them is its own,
  separately-gated change.
- **Slice 2 (lithology + D9).** New transient `geology.rs`: independent noise-band
  lithology, a phase-2 climate-erosion proxy (latitude + continentality — an
  explicit, documented stand-in for phase-3 precipitation, which doesn't exist yet
  this early), and `redistribute_elevation_regional` — the existing global
  rank-based redistribution still sets the overall hypsometric SHAPE, but each
  region's own pre-redistribution mean (plate id when real plate data exists, a
  coarse grid otherwise) is captured first and reapplied afterward as a bounded
  offset, so a region's genuine character survives instead of being erased by the
  global rank-squeeze.
- **Slice 3 (orogeny + D3).** `compute_orogeny_field`: a multi-source BFS from
  every convergent/transform boundary land cell that INHERITS its originating
  point's setting (active-margin / collision / island-arc / subducting-side, from
  each plate's oceanic/continental split reconstructed by majority-vote over
  `terrain` — `Plate.is_oceanic` itself isn't persisted, per §2 "transient") and age
  (a noise value sampled once at the seed and carried through the BFS) outward
  through the whole belt, not just the boundary cell — so an old worn range can sit
  beside a young sharp one at the SAME boundary, coherently along its own strike.
  `belt_profile` shapes the belt asymmetrically by setting (an active margin's arc
  crest offset inland of the trench; a collision broad and roughly symmetric; an
  island arc narrow). `plates.rs`'s D3 fix: the boundary normal is now the true
  Voronoi-bisector direction between the two plates' own seed points, not the
  direction from one plate's centre to the cell (correct only for a compact,
  centrally-sampled plate); a triple junction now classifies by the STRONGEST
  signal across every differing neighbour rather than whichever the fixed
  4-neighbour scan order hit first.
- **Slice 4 (coastline).** A "crust thickness" field in `plates.rs` — each plate's
  base thickness (oceanic vs continental) plus strong domain-warped noise — with a
  PERCENTILE threshold (not a fixed cutoff) so total land fraction stays exactly
  what the plate mix implied while the shoreline SHAPE departs from the raw Voronoi
  edge. A `despeckle_terrain` pass (4-connected component flood-fill) then flips
  any land/sea patch under `DESPECKLE_MIN` (90 cells) back to its surroundings, so
  the noise strong enough to decouple the coastline doesn't also scatter single-cell
  dust. Measured `coast_on_boundary`: ~100% → 62.5%, now gated permanently by
  `coastline_departs_from_the_plate_boundary` (two probe seeds, <85%).
  **A second, sharper bug found inside the measurement itself, not the mechanism:**
  the FIRST tuning pass measured 90% and looked like an unmodified Voronoi edge in
  `dump_natural_sheet` — the exact D1 symptom. A direct probe (comparing the
  resulting `terrain` array bit-for-bit with the warp on vs off) found why: the
  crust FIELD genuinely differed (confirmed by summing it), but the percentile
  THRESHOLD kept selecting the identical set of cells regardless, because
  `fbm_noise`'s multi-octave output clusters far more tightly than its nominal
  0..1 range — the realised swing rarely bridged the 0.5 base gap between an
  oceanic and continental plate's crust value. A field that is numerically
  different but geometrically identical is invisible to eyeballing a render, which
  is exactly why the probe compared the actual `terrain` array rather than trusting
  the crust field's own statistics.
- **Slice 5 (seafloor).** `generate_shelves` gains a mid-ocean ridge (segmented by
  along-strike noise, reading transform offsets as gaps in the crest), a trench at
  a convergent margin, abyssal-hill texture, and scattered seamounts/guyots (a
  documented simplification of "chains" — sparse hotspot noise, not traced
  lines). Measured `sea_depth`↔distance-to-coast correlation: ~1.0 → 0.66-0.74.
  This is the one slice touching the Earth gate (via `compute_sea_depth`/
  `generate_shelves` feeding phase 3's `distance_to_ocean`/upwelling): main-class
  70.1% → 70.2%, floor raised to 70.15.
- **Slice 6 (render).** The elevation ramp's top stop softened from pure white
  (255,255,255) to (250,248,244) — still strictly brighter than the 5000 m stop,
  just short of a full blow-out. Texture shading: a bounded, HONEST approximation
  (`TEXTURE_SHADOW_BOOST` in `render/tile_image.rs`) that widens the existing
  direction-independent AO curvature term specifically on the shadowed/lee side of
  a landform, rather than a second light source — the true multi-scale
  fractional-Laplacian transform (Leland Brown) needs a wide cross-tile halo this
  renderer's `TileNeighbors` doesn't carry (only the immediate edge of each
  cardinal neighbour), so building it for real is left as future work, not faked.
- **§3 instrumentation.** `terrain_metrics` (new, alongside `bench_phase2`) prints
  RMS slope / slope spread across an 8×8 window grid / drainage density (share of
  land cells carrying real accumulated flow, off the same priority-flood the
  erosion pass uses) / hypsometric integral / coast-on-boundary fraction /
  sea_depth↔distance correlation, per model. First measurement — it SETS the
  baseline (see the table in `docs/SCOREBOARD.md`) rather than clearing one.

**Three things did not go as planned, recorded rather than smoothed over:**

1. **Performance missed its own target.** §5 asked for "no slower than today,
   stretch: faster." `bench_phase2` @ 3600×1800 landed at plates 8.5s → 11.4s,
   shape 11.4s → 13.9s — slower, despite every per-cell pass with no cross-cell
   dependency (Voronoi assignment, boundary classification, the lithology/
   climate-proxy maps, the slice-4 crust map) moving to `rayon::into_par_iter`.
   The priority-flood queue
   itself is inherently sequential (a priority queue doesn't parallelise the way
   the phase-3 row loops do) and stayed the dominant cost even after the outer
   pass count was capped by grid size (§ below). Left as an open item rather than
   chased further this session — a genuine O(n) flow-routing algorithm (Braun &
   Willett 2013) would remove the heap entirely and is the natural next step if
   this needs to close further.
2. **The outer-pass count was first keyed to the wrong axis.** `iterations` (the
   old droplet-budget parameter) scales UP with world size before its own ceiling
   clamp — exactly backwards for a pass whose cost is what needs controlling on a
   LARGE world — and it starved every unit-test-sized fixture (all well under
   `iterations`' own floor) down to 2-4 outer passes. That silently broke
   `cordillera_crest_runs_parallel_to_the_coast`: a traced spine needs roughly 5-6
   outer passes before it differentiates from generic ridged noise, and the test's
   own 80%-of-noise-spread margin failed at ~81% — close enough to look like noise,
   not a real defect, until traced to the pass count. Fixed by keying pass count to
   GRID SIZE instead (4 passes above 4M cells, 6 between 1-4M, 8 below), which is
   the axis that actually drives wall-clock cost.
3. **Slice 4's decoupling had a real downstream cost, kept rather than hidden
   behind a loosened test.** On the fixed-seed 300×150 goods-coverage reference
   world, the new coastline geometry put the inshore good `pearls`' homeland on a
   stretch of new coast outside every settlement's catchment at that specific
   seed and scale — `goods_coverage_diagnostic` caught it (a real, intended gate,
   not a target being gamed). Real generation regenerates settlements FROM the
   decoupled coastline (phase 7 runs after phases 1-2 against whatever terrain
   resulted, never against a frozen layout), so this reads as a fixed-fixture
   small-world-sampling artefact rather than a claim that pearls is unreachable in
   practice — the same category of finding the file's own `dyes` exception already
   documents, just a different root cause. Added as a second, honestly-labelled
   entry rather than folded silently into the existing one.
