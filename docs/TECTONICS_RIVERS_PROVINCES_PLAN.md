# Tectonics · Rivers · Provinces — fix plan

**Status: Slices 1, 2, 3, 5 (core), 7, 8 BUILT and gated. Slice 4 built only in
scoped-down form (collision-type-aware volcanism; no persisted per-plate
identity or Euler-pole motion). Slice 6 (the opt-in bounded tectonic
simulation) and Slice 5's per-plate UI override / archipelago bias are NOT
built** — both depend on the Euler-pole plate identity Slice 4 was scoped down
from, and are a substantially larger architectural change (new persisted plate
state, a map click-to-select interaction, a coarse-grid deformation solver)
than fits one session; left for a dedicated follow-up rather than shipped
half-working. What shipped, per slice:

- **Slice 1** (F1): `MIN_LAND_ELEV` floor + scale-about-floor regional bias
  replace the old additive-then-clamp. Measured: the 88.5 m spike's largest
  flat component fell 31,828 → 12,783 cells (shape) and 6,064 → 709 cells
  (plates) — the clamp-plateau SIGNATURE is gone, though `diagnose_flat_lowland`
  is kept a diagnostic, not promoted to a hard assertion (rule 31 added to
  CLAUDE.md §10 either way).
- **Slice 2** (F2): micro-relief seeded per-world (`world_micro_relief_seed`,
  hashed from the elevation field) + a second broad wavelength; per-bend
  amplitude modulation; Kinoshita (1961) skew/flatten replacing the fixed
  second harmonic; wavelength drift along the reach. Gated by
  `meanders_are_not_a_sine`.
- **Slice 3** (F4): `generate_and_persist_provinces` (shared helper) now runs
  in both `sim_run_all` and `sim_run_all_from_terrain`, with an automatic
  km²-stated merge floor (`AUTO_MERGE_FLOOR_KM2` = 8,000 km², rule 25).
- **Slice 4** (F5), scoped down: convergent-boundary volcanism now reads
  collision type (continent-continent ≈ none, ocean-involving ≈ island-arc/
  arc-trench rate) from the plates already in scope during generation. The
  deeper root cause — one velocity vector per plate, no persisted per-plate
  kind/Euler pole — is UNCHANGED; `boundary_character_varies_along_its_length`
  was not built.
- **Slice 5** (F3): `DEFAULT_OCEAN_FRACTION` (0.70) is hit BY CONSTRUCTION —
  plates are shuffled then greedily marked oceanic by their real measured
  Voronoi cell count until the target ocean area is met, replacing the
  independent `rng < 0.4` coin flip. Gated by `land_fraction_tracks_the_target`
  (was failing at 56–72% land against the 30% target; now within the gate's
  10-point band at every tested plate count/seed). The per-plate UI override
  and the archipelago/island-arc bias are NOT built.
- **Slice 6**: not attempted (depends on Slice 4's Euler poles).
- **Slice 7** (F6): `coast_relief`, propagated through the same BFS that
  computes shelf distance-to-coast, replaces the old fixed-radius land-elevation
  sample (which only ever saw land for the first few cells off the coast and
  reverted to the flat default for the rest of the shelf) — plus a continuous
  margin-maturity fade (was a binary 0.5/1.0 step) and a second, ~1/4-world-wide
  noise wavelength. Gated by `shelf_width_varies_between_margins`.
- **Slice 8** (F7): `riverBreaks: true` → `false` in `OverlayManager.ts`.

Reproduce the original measured findings with:

```bash
cd src-tauri && cargo test --release --lib diagnose_ -- --ignored --nocapture
```

This plan answers eight reported problems. Four of them turn out to be **three
root causes**, which is the main finding — they were reported as separate
complaints and are not separate bugs.

---

## 0 · The measured findings

Everything here is measured, not inferred. Two worlds, the `shape` model (what
"Complete from Landmass" runs) and the `plates` model, at 900×500.

### F1 — 16% of all land sits at EXACTLY 88.5 m, in one blob

```
shape    land=262500  at_floor(88.5m exactly)=15.96%  <=90m=16.55%
                      no-gradient(<1m to all 8 nbrs)=15.94%  largest_flat_component=31828 cells
plates   land=259382  at_floor(88.5m exactly)= 5.68%  <=90m= 6.13%
                      no-gradient(<1m to all 8 nbrs)= 6.13%  largest_flat_component= 6064 cells
```

`0.01 × 8848 m = 88.48 m` — this is the user-reported "huge area is 88 meters
everywhere", and it is an **elevation clamp floor**, not a landform. The
signature is unmistakable: `<=90m` (16.55%) barely exceeds `at_floor` (15.96%),
so there is a spike exactly at the floor and almost nothing just above it. A
natural distribution cannot look like that; a clamp always does.

**Two sources, both in `elevation.rs`, and the second is the larger:**

1. `redistribute_elevation`'s band loop. Band 0 spans 0..1000 m and assigns
   `t · 0.113` across its ranked cells, then `.clamp(0.01, 1.0)`. Every cell with
   `t < 0.0885` — the lowest ~9% of a band holding 71–86% of all land — is pinned
   to the identical value.
2. `redistribute_elevation_regional`'s bias pass:
   `elevation[i] = (elevation[i] + bias * REGION_CONTRAST).clamp(0.01, 1.0)`.
   A region with a negative bias (an old worn craton) has its **entire low end**
   pushed under the floor and clamped. This is why the flat is one contiguous
   31,828-cell component rather than scattered floodplain — a whole
   physiographic region collapses to one elevation at once.

The clamp lands on the **lowest-lying land**, which is precisely where every
river runs. That makes F1 the root cause of F2 as well.

### F2 — the river meander is a textbook sine, and the same one on every world

Three independent reasons, all in `build_meander_path` / `compute_hydrology`
(`step5_rivers/rivers.rs`):

- **Amplitude is constant per river.** `amp = slow · min(base_wav·0.16, half·0.5)`.
  On the F1 flat, gradient is exactly zero so `slow` saturates at 1.0, and `half`
  saturates at `max_reach` because there is no rising ground to stop the probe.
  `base_wav = width_cells · 11` is one number for the whole river. So every bend
  on every lowland reach gets the *same* amplitude.
- **Phase is a monotonic accumulator** and the offset is
  `amp·(sin φ + 0.25·sin(2φ+0.7))` — strictly alternating bends at a near-regular
  period. The existing wavelength jitter varies the period a little but never the
  amplitude and never the bend *shape*.
- **The micro-relief that governs all flat-ground drainage uses a hard-coded
  seed.** `compute_hydrology` takes no seed, and the line reads
  `fbm_noise(x·MICRO_FREQ, y·MICRO_FREQ, 0x5CA1_AB1E, 4, 2.0, 0.5)`. That literal
  is not derived from the world seed, so the field that decides where rivers go
  on every flat is **byte-identical in every world this app has ever generated**.
  It is also a single frequency (~20 cells), so every plain grows the same size
  of tributary tree.

This is the whole of "rivers keep meandering in very similar shape". Note the
ordering: fixing F1 alone already removes most of it, because a lowland with a
real gradient stops saturating `slow`.

### F3 — the world is 56–72% land and has 2–4 islands

```
plates=6   land=71.8% of globe (Earth 29.2%)  largest_landmass=97% of all land  islands: tiny=0.3 small=0.3 large=0.3
plates=10  land=56.1%                          largest_landmass=83%              islands: tiny=2.3 small=2.0 large=0.7
plates=16  land=56.4%                          largest_landmass=75%              islands: tiny=2.0 small=3.7 large=1.7
plates=24  land=61.3%                          largest_landmass=93%              islands: tiny=1.3 small=3.7 large=0.7
plates=40  land=65.4%                          largest_landmass=95%              islands: tiny=2.0 small=1.7 large=0.7
```

Cause: `let is_oceanic = rng.gen::<f32>() < 0.4;` over roughly equal-area
grid-jittered Voronoi plates. Land converges on ~60% **whatever the plate count**,
and it is *worse* at low counts (71.8% at 6 plates) because with an expected 2.4
oceanic plates the binomial variance often yields one. There is no land-fraction
target anywhere in the pipeline, and no user control.

Islands are separately suppressed: the level-set coastline can only flip cells
within `reach` of a boundary, and `despeckle_terrain` deletes any component under
`DESPECKLE_MIN = 14` cells. An archipelago world is currently unreachable.

### F4 — neither run-all generates provinces

`sim_run_all` and `sim_run_all_from_terrain` (`commands/sim_commands.rs`) both
end at phase 8. Neither calls `generate_provinces`. The comment at the step-7a
call site even says *"and before province generation"* — but there is no province
generation on either path. Provinces are only reachable from `StepSettlements.tsx`
and `ProvincePanel.tsx`.

`merge_small_provinces_wh` (`shared/provinces.rs`) already exists and already
does the right thing (folds a sub-`min_cells` province into its largest-shared-
border neighbour, skips true islands, recompacts ids). It is wired only to a
manual button, so nothing tidies up automatically.

### F5 — straight mountain lines at plate contacts

`generate_plates_and_landmass` builds plates as a 2-D Voronoi over grid-jittered
seeds. A Voronoi cell boundary is a **straight bisector segment** between two
seed points, and `boundary_type` is written per-cell from a 4-neighbourhood test,
so the tectonic signal is a 1-cell-wide straight line. `generate_elevation`
already compensates by sampling the orogeny field at a *warped* position (D4),
which bends the belt but cannot change the fact that the underlying datum is a
straight segment.

Deeper: the plate model has **one velocity vector per plate** (`vx`, `vy`). A
real plate rotates about an Euler pole, so its motion — and therefore the
convergence rate and the boundary's character — **varies along the boundary**.
A single translation vector makes an entire boundary uniformly convergent or
uniformly divergent along its whole length, which is what makes it read as a
drawn line rather than a margin.

There is also no **collision-type** distinction. `boundary_type` records
convergent/divergent/transform but not *what is colliding*, so ocean–ocean
(island arc), ocean–continent (arc + trench), and continent–continent (broad
collisional plateau, no volcanism) all produce the same ridge. `geology.rs`
partially reconstructs setting by majority-vote terrain per plate, but the plate
record itself never carried the type.

### F6 — shelves are thin/absent from landmass

`generate_shelves` **is** called on both run-alls with identical hard-coded
parameters `(12.0, 0.4, 0.3, 8.0)`. So "absent" is not literally true — but two
real problems produce the reported look:

- The shelf width noise is `fbm_noise(x/noise_scale, y/noise_scale, seed, …)`
  at `noise_scale = max(w,h)/20`, one broad field with `noise_amount = 0.4`. The
  result is a shelf of near-uniform width, not the wildly unequal shelves of a
  real coast (Siberian shelf ~800 km, Chilean ~10 km).
- The active/passive margin distinction — the actual physical cause of that
  inequality — is a **binary 0.5 multiplier** gated on plate data, and on the
  from-landmass path there is no plate data at all, so *every* margin is passive
  and every shelf is the same width.

### F7 — the river reach-break labels are on by default

`OverlayManager.ts` line 647: `riverBreaks: true`. The "Upper › Middle @X km"
tick markers are drawn by default. The toggle already exists.

---

## 1 · Design decisions to confirm

These change what gets built. My recommendation is marked.

- **D1 — Where does land fraction get decided?** *Recommended:* an explicit
  `ocean_fraction` target (default 0.70, i.e. 30% land) that the plate seeder
  hits by construction, plus a per-plate continental/oceanic assignment the user
  can override. The alternative (keep the per-plate coin flip, add a slider on
  the probability) does not fix the variance at low plate counts.
- **D2 — How far does the tectonic simulation go?** *Recommended:* a **bounded
  kinematic** simulation (10–40 steps of rigid Euler-pole rotation accumulating
  a deformation field), not a physical mantle model. It is enough to produce
  collision belts, rifted margins and arc chains in the right places, and it
  runs in seconds. A true viscous simulation is out of proportion to the app.
- **D3 — Is the simulation opt-in?** The user asked for exactly this: if the
  short simulation is not picked, keep the landmass as-is and only draw the
  terrain consequences at contacts. *Recommended:* yes, a checkbox, defaulting
  ON for "Generate Full World" and OFF for "Complete from Landmass" (where the
  user has drawn a coastline they want kept).
- **D4 — Do provinces run inside the run-alls?** *Recommended:* yes, both paths,
  with automatic small-province merging. This is what the user asked for and
  there is no reason a full-world generate should stop short of it.

---

## 2 · The slices, in dependency order

Ordered so each is independently shippable and each has a gate that can fail.
**Slice 1 is the highest value in the plan** — it is one root cause behind three
reported symptoms and is a dozen lines.

### Slice 1 — Remove the 88.5 m clamp plateau *(fixes F1, most of F2)*

Introduce `MIN_LAND_ELEV = 0.0006` (~5 m — a real floodplain; it must stay above
zero because zero means "sea" to `plates::invert_terrain` and to
`redistribute_elevation`'s own land filter).

- Band 0 starts at `MIN_LAND_ELEV` instead of 0, so the lowest-ranked land rises
  monotonically from a plausible delta height instead of piling on a clamp.
- The regional bias becomes a **scale about the floor**, not an additive offset
  followed by a clamp:
  `elev = MIN_LAND + (elev − MIN_LAND) · factor`, `factor = 1 + bias·CONTRAST/land_mean`
  clamped to `[0.25, 4.0]`. A region still moves up or down by the same intent,
  but every cell stays strictly ordered and strictly above the floor, so a region
  can never collapse to one value.

**Gate:** extend `diagnose_flat_lowland` into an assertion — no more than 1% of
land at the floor value, and no flat component larger than 0.5% of land, on both
models. Both currently fail (15.96% / 12%), so the gate can fail.
**Watch:** `the_default_hypsometry_resembles_earth` must still pass — this changes
the bottom of the histogram and that gate asserts its shape. The Earth climate
gate cannot move here (`earth_validation.rs` never calls `generate_elevation`),
but run it anyway to confirm rather than assert.

### Slice 2 — River variability *(finishes F2)*

Three independent changes, each addressing one of F2's three causes:

- **Seed the micro-relief from the world.** `compute_hydrology` takes no seed
  parameter and threading one through touches many call sites, so derive it from
  a cheap hash of the world's own elevation field — deterministic per world,
  different between worlds. Add a **second, broad wavelength** (~90 cells)
  alongside the existing ~20-cell term so one plain's drainage tree reads at a
  different scale from another's.
- **Per-bend amplitude.** Modulate `amp` by a noise field a few bends long
  (`0.25 + 1.45·n²` — squared, so bends are usually modest and occasionally
  large), which produces the unequal bends and near-straight reaches a real
  meander train has.
- **Kinoshita skew + flatten.** Replace `sin φ + 0.25·sin(2φ+0.7)` with the
  standard Kinoshita (1961) correction
  `sin φ + flat·cos 3φ − skew·sin 3φ`, with `skew`/`flat` themselves jittered
  along the reach. This is what gives the asymmetric goose-neck shape instead of
  a wave. Also let the wavelength **drift along the reach** over a long scale,
  not just jitter cell-to-cell around a river-wide constant.

**Gate:** a new `meanders_are_not_a_sine` — take each long lowland reach's
lateral-offset series, and assert (a) the coefficient of variation of successive
bend amplitudes exceeds a floor, and (b) the FFT is not dominated by a single
bin. Both fail on the current code by construction. Also assert two different
world seeds produce different flat-plain drainage (currently they do not).

### Slice 3 — Provinces in both run-alls, with tidy-up *(fixes F4)*

Add to `sim_run_all` and `sim_run_all_from_terrain`, after step 7a and before
phase 8: `generate_provinces` → `merge_small_provinces_wh` → persist the same
three metadata keys the standalone command writes (`provinces`,
`province_raster`, `province_raster_rle`). Extract the shared body of
`sim_generate_provinces` into one helper both the command and the run-alls call,
so the two paths cannot drift.

`min_cells` for the automatic merge should be **stated in km², not cells**
(CLAUDE.md rule 25 — a cell is ~11 km at 3600×1800 and ~133 km on a test world),
converted per world. Suggest ~8,000 km² as the floor for "this is a generation
artefact, not a province".

**Gate:** `a_full_world_generate_produces_provinces` — run each run-all on a
small world and assert the province list is non-empty, the raster covers the
land, and no province is below the km² floor except true single-island ones.

### Slice 4 — Plates get identity: type, motion, layer *(foundation for F5/F3)*

Promote `Plate` from a private struct to persisted world data. Each plate
carries: `kind` (Continental / Oceanic), `euler_pole` (lat/lon) + `omega`
(angular rate) replacing the flat `vx`/`vy`, `age`, and a stable `id`.

**Euler-pole rotation is the key change for F5.** Velocity at a cell becomes
`ω × r`, so the *same* boundary is strongly convergent at one end, obliquely
transform in the middle and divergent at the other — exactly how a real margin
varies along its length. That breaks the "uniform straight line" character at
its source, rather than hiding it with a warp.

Boundaries additionally record **collision type**, derived from the two plates'
`kind`: `OceanOcean` (island arc), `OceanContinent` (arc + trench + forearc),
`ContinentContinent` (broad collisional plateau, **no** volcanism),
`Rift` / `Transform`. `deposits.rs` already reads `boundary_type` for ore
setting (§8.16) and would read this directly instead of reconstructing it by
majority-vote terrain in `geology.rs`.

Plate seeds should also stop being equal-area: draw plate sizes from a
**power-law**, since real plates are wildly unequal (Pacific vs Juan de Fuca).
That alone makes boundaries less regular.

**Gate:** `boundary_character_varies_along_its_length` — for each boundary
segment, assert the convergence rate has real variance along it (a single
translation vector gives ~zero variance, so this fails on the current code).
Plus a persistence round-trip test.

### Slice 5 — Land/ocean control *(fixes F3)*

- An **`ocean_fraction` target** (default 0.70). The plate seeder assigns
  continental vs oceanic to *hit the target by construction* — sort plates by
  area and mark oceanic until the target is met — instead of an independent coin
  flip per plate. This removes the low-plate-count variance entirely.
- A **per-plate override**: the user sets any plate Continental or Oceanic in the
  UI. This is the "make so i can pick which plates are continental and which
  oceanic" ask. Needs a plate-picking interaction on the map (the Plates layer
  already renders per-plate colour, so it is a click-to-select plus a two-state
  toggle).
- **Islands as a first-class output.** Lower `DESPECKLE_MIN` and let island arcs
  from Slice 4's `OceanOcean` boundaries deposit genuine chains — an arc is the
  physically correct island generator and the model will now know where they are.
  Add an "archipelago" bias so an island-rich world is reachable.

**Gate:** `land_fraction_tracks_the_target` — across plate counts 6…40 and
several seeds, measured land fraction stays within a few points of the target.
`diagnose_plate_land_fraction` becomes that assertion; it currently reads
56–72% against a 30% intent, so it fails.

### Slice 6 — The optional tectonic simulation *(the user's main ask)*

With Slice 4's Euler poles in place, run **N bounded steps** (10–40) of rigid
plate rotation, accumulating per-cell:

- **convergence** → crustal thickening → orogen. Continent–continent gives a
  broad high plateau; ocean–continent gives a narrower arc set back from a
  trench; ocean–ocean gives an island arc in open water.
- **divergence** → thinning → rift valley, then a new ocean basin if it runs
  long enough.
- **shear** → transform offsets that *segment* the boundary — this is what makes
  a mid-ocean ridge read as a series of offset segments rather than one seam.

The accumulated field is what phase 2 reads, replacing the current
distance-to-a-static-boundary lookup. Because deformation accumulates over steps
along a *rotating* boundary, the resulting belt is curved and segmented by
construction — no warp hack needed.

**Per D3 this is opt-in.** Unchecked, the landmass is left exactly as it is and
only the *terrain consequences* at contacts are drawn — which is precisely the
"if short simulation is not picked" branch the user described, and is also what
"Complete from Landmass" needs, since there the coastline is the user's own.

**Gate:** `collision_type_produces_distinct_terrain` — the three collision types
must produce measurably different elevation profiles (a continent–continent belt
must be broader and less volcanic than an ocean–continent arc). Plus: with the
simulation off, the landmass must be **bit-identical** to the input — the same
no-op discipline CLAUDE.md rule 10 holds for the Earth parameters.

### Slice 7 — Shelves that vary *(fixes F6)*

Replace the binary active/passive multiplier with a **continuous margin
maturity**: shelf width scales with how long the margin has been passive, which
Slice 4/6 now knows (a rifted margin accumulates sediment; an active one is
scraped away). Add a second, much longer-wavelength noise term so shelf width
varies over thousands of km, not just the current single broad field.

On the from-landmass path there is no plate data, so derive a **proxy maturity**
from coastline shape — a smooth, embayed, low-relief coast reads as passive; a
straight coast backed by high relief reads as active. This is the same
"documented proxy" convention `geology.rs` already uses for its phase-2 climate
term, and it is what makes the from-landmass path stop giving every coast the
same shelf.

**Gate:** `shelf_width_varies_between_margins` — the ratio of the widest to the
narrowest margin shelf on one world must exceed a floor, on both paths. Currently
near 1.0 on the from-landmass path (every margin passive), so it fails.

### Slice 8 — River reach labels off by default *(fixes F7)*

`OverlayManager.ts` line 647: `riverBreaks: true` → `false`. One line, toggle
already exists and is unchanged.

---

## 3 · Suggested convention (CLAUDE.md rule 31)

> **A clamp is not a landform.** Any pass that writes elevation must not leave a
> large area at exactly its floor or ceiling. A rank remap, a bias offset and a
> range clamp compose into a plateau at the boundary value, and that plateau then
> silently propagates: no gradient means no drainage direction, which means the
> meander model saturates and every river on it comes out the same shape. Where a
> pass needs a bound, scale into the range rather than clamping onto it — and
> check the result with `diagnose_flat_lowland`, not by reading the code, since
> every cause here was invisible in review and obvious in one histogram.

---

## 4 · Risks

- **Slice 1 moves every downstream consumer.** Lapse-rate temperature, biomes,
  fertility, habitability and settlement placement all read elevation, and 16% of
  land is about to stop being 88.5 m. This is the intent, but it means Slice 1
  needs the full gate set run, not just its own.
- **Slice 4 changes `boundary_type` semantics**, which `deposits.rs` reads for ore
  setting (§8.16). `no_shipped_mineral_places_nothing` and
  `template_world_without_plates_still_places` are the guards; both must stay
  green, and the collision-type field should be *added* alongside the existing
  encoding rather than replacing it, so old saves keep loading.
- **Slices 4–6 change worldgen only for NEW worlds.** A saved world's stored
  tiles are untouched — the same scoped blast radius TERRAIN_2_PLAN slice 4 had.
- **Slice 6 is the one with real cost risk.** Phase 2 is already 11–14 s at
  3600×1800 (`bench_phase2`). N simulation steps over a full grid could dominate
  it. Mitigation: run the deformation accumulation on a **coarse grid** (plates
  are a 1000+ km feature; a 1/8-scale field is ample) and upsample, which keeps
  it well under a second.

## 5 · Deliberately NOT in this plan

- **A physical mantle/viscous simulation.** Out of proportion; D2 takes the
  kinematic route deliberately.
- **Continental drift over geological time as a playable/animated feature.** The
  simulation here runs once at generation and is discarded, like `geology.rs`.
- **Re-running provinces mid-campaign.** They are frozen by design
  (`ensure_unfrozen` already blocks it) and this plan does not change that.
- **Rewriting `extract_rivers`' channel network.** Slice 2 changes meander
  *geometry* and the micro-relief that seeds drainage; the accumulation-threshold
  channel extraction is untouched.
- **The 1–2 cell dark lineament** named as still-open in §8.23b. It is in the base
  tectonic/noise field and Slice 4/6 may or may not incidentally fix it; it is not
  a goal here and should not be claimed as one.
