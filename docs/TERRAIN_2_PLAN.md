# Terrain 2.0 — the plan

**Status: APPROVED, NOT YET BUILT.** Decisions in §2 are settled; §4 is the build
order. Nothing here has shipped except the instrumentation in §3.

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
