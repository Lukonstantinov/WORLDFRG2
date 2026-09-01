# World generation & trade — master plan

**One file. Three merged plans, three separate subjects, one dependency chain:**
the land decides where trade can go, so Part I (the world) is upstream of
Parts II and III (the campaign's trade and knowledge).

| Part | Subject | Status |
|---|---|---|
| **I** | Tectonics · rivers · provinces · shelves | Slices 1, 2, 3, 5, 7, 8 **BUILT**; 4 scoped down; 6 not built |
| **II** | Outpost connectivity & the entrepôt | Slices A, B built (fixes G1-G3); E1 built (fixes G6); Slices C (entrepôt) and D (transport modes) **not built** |
| **III** | Exploration, the known world, transport modes | §8.1/§8.2 built; §4 built IN PART (river data reaches `base_days`; validated positive on a real world, r=0.092 — mode/capacity split not built); knowledge/fog, exploration **not built** |

**Read the build order first (Part III §7).** It is not the order these parts are
written in: transport modes (III §4) come first because they are the biggest
lever, have an instrument that already exists, and may make several later items
unnecessary.

Reproduce every measured number in this file with:

```bash
cd src-tauri && cargo test --release --lib diagnose_ -- --ignored --nocapture
```

Merged from `TECTONICS_RIVERS_PROVINCES_PLAN.md`,
`OUTPOST_CONNECTIVITY_AND_ENTREPOT_PLAN.md` and
`EXPLORATION_AND_KNOWN_WORLD_PLAN.md`. Code comments citing those filenames
refer to Part I, II and III of this document respectively.

---

# Part I · Tectonics, rivers, provinces, shelves

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

### 0 · The measured findings

Everything here is measured, not inferred. Two worlds, the `shape` model (what
"Complete from Landmass" runs) and the `plates` model, at 900×500.

#### F1 — 16% of all land sits at EXACTLY 88.5 m, in one blob

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

#### F2 — the river meander is a textbook sine, and the same one on every world

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

#### F3 — the world is 56–72% land and has 2–4 islands

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

#### F4 — neither run-all generates provinces

`sim_run_all` and `sim_run_all_from_terrain` (`commands/sim_commands.rs`) both
end at phase 8. Neither calls `generate_provinces`. The comment at the step-7a
call site even says *"and before province generation"* — but there is no province
generation on either path. Provinces are only reachable from `StepSettlements.tsx`
and `ProvincePanel.tsx`.

`merge_small_provinces_wh` (`shared/provinces.rs`) already exists and already
does the right thing (folds a sub-`min_cells` province into its largest-shared-
border neighbour, skips true islands, recompacts ids). It is wired only to a
manual button, so nothing tidies up automatically.

#### F5 — straight mountain lines at plate contacts

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

#### F6 — shelves are thin/absent from landmass

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

#### F7 — the river reach-break labels are on by default

`OverlayManager.ts` line 647: `riverBreaks: true`. The "Upper › Middle @X km"
tick markers are drawn by default. The toggle already exists.

---

### 1 · Design decisions to confirm

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

### 2 · The slices, in dependency order

Ordered so each is independently shippable and each has a gate that can fail.
**Slice 1 is the highest value in the plan** — it is one root cause behind three
reported symptoms and is a dozen lines.

#### Slice 1 — Remove the 88.5 m clamp plateau *(fixes F1, most of F2)*

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

#### Slice 2 — River variability *(finishes F2)*

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

#### Slice 3 — Provinces in both run-alls, with tidy-up *(fixes F4)*

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

#### Slice 4 — Plates get identity: type, motion, layer *(foundation for F5/F3)*

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

#### Slice 5 — Land/ocean control *(fixes F3)*

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

#### Slice 6 — The optional tectonic simulation *(the user's main ask)*

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

#### Slice 7 — Shelves that vary *(fixes F6)*

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

#### Slice 8 — River reach labels off by default *(fixes F7)*

`OverlayManager.ts` line 647: `riverBreaks: true` → `false`. One line, toggle
already exists and is unchanged.

---

### 3 · Suggested convention (CLAUDE.md rule 31)

> **A clamp is not a landform.** Any pass that writes elevation must not leave a
> large area at exactly its floor or ceiling. A rank remap, a bias offset and a
> range clamp compose into a plateau at the boundary value, and that plateau then
> silently propagates: no gradient means no drainage direction, which means the
> meander model saturates and every river on it comes out the same shape. Where a
> pass needs a bound, scale into the range rather than clamping onto it — and
> check the result with `diagnose_flat_lowland`, not by reading the code, since
> every cause here was invisible in review and obvious in one histogram.

---

### 4 · Risks

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
  tiles are untouched — the same scoped blast radius Terrain 2.0 slice 4 had (CLAUDE.md §8.23b).
- **Slice 6 is the one with real cost risk.** Phase 2 is already 11–14 s at
  3600×1800 (`bench_phase2`). N simulation steps over a full grid could dominate
  it. Mitigation: run the deformation accumulation on a **coarse grid** (plates
  are a 1000+ km feature; a 1/8-scale field is ample) and upsample, which keeps
  it well under a second.

### 5 · Deliberately NOT in this plan

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

---

# Part II · Outpost connectivity & the entrepôt

**Status: Slices A and B BUILT (fix G1/G2/G3); E1 BUILT (fixes G6, folded into
`try_found_house_outpost` rather than shipped as a separate slice). Slice D
(transport modes, Part III §4) is BUILT IN PART — the river-data plumbing gap
that made it a no-op; the mode/capacity split is not. Slice C (the entrepôt) is
NOT built** — substantially larger and left for a dedicated follow-up. What
shipped:

- **Slice A** (G1): the Dynamic Trade Flow overlay (`commands/query_commands/
  flow.rs`) no longer drops every hub with `is_estate == true`. A co-located
  estate (`parent >= 0`) now resolves to its parent's coarse node (its flow
  is credited to the parent city, as `read_trade.rs`'s own `city_of` already
  did); a remote outpost (`parent < 0`) keeps its own node instead of being
  skipped outright. Applied to both `campaign_get_trade_flow` and
  `campaign_get_corridors`, which shared the same anti-pattern.
- **Slice B** (G2/G3): `CampaignSim::is_remote_site` (`tick/production.rs`) is
  the one predicate — an estate with no parent, not abandoned. `rebuild_routes`'
  `real` set (feeding all three no-dead-city guarantees: `MIN_GUARANTEED_
  PARTNERS`, the market lifeline, and coastal cabotage) now includes remote
  sites alongside ordinary settlements. `rescue_tiny_components`'s estate
  component-fixup now falls back to `founder_hub` when `parent < 0`, closing
  G3's latent trap. City rankings, society/pops and government are
  deliberately untouched — the predicate is about ROUTING, not promotion.
  Codified as CLAUDE.md rule 32.
- **E1** (G6): `try_found_house_outpost` now reads `delta`/`chokepoint` with
  the same premiums (`+0.60`/`+0.80`) `maybe_found_settlement_colony` already
  prices, and refuses a coastal site when the founding house's whole network
  (home + offices + own estates) has no coastal foothold — the same "no fleet
  tradition" rule the city-founded colony path already applies.

Not built from this Part: E2 (gate outpost founding on a prior expedition —
depends on Part III's knowledge/exploration state, not built), E3 (relay
outposts, deliberately sequenced after Slice D), and Slice C (two-leg routing
through an outlet + autonomous port founding) — the "real feature" of this
part, left for a dedicated session given its algorithmic and performance risk
(§4 of this Part).

Read alongside
`TECTONICS_RIVERS_PROVINCES_PLAN.md` (the world half); this is the campaign half.

Reported: outposts are not connected to their founding capital, no
understandable routes are drawn to them, and there is no natural port /
transshipment hub letting goods move from a hard-access interior to a river,
lake or open sea.

The first two are **bugs with a single shared cause**. The third is a real
design gap and is the larger piece of work.

---

### 0 · What an outpost actually is

`try_found_house_outpost` (`tick/houses.rs`) creates a hub via `create_estate`
with **`parent = -1`**, then sets `colony_kind = 2`. The `parent = -1` is
deliberate and documented — it is what keeps the outpost at its own remote site
coordinates instead of being co-located with its founder.

That produces a hub which is `is_estate == true` but **is not co-located with a
parent city**. Every other estate in the model is. The whole codebase branches on
`is_estate` in one of two ways, and an outpost is wrong under both:

- **"It's internal to its parent, collapse it there"** — correct for a farm
  outside a city, wrong for an outpost 2,000 km away with no parent at all.
- **"It's not a real hub, exclude it"** — correct for keeping estate dots off
  city rankings, wrong for a settlement that is genuinely a separate place.

An outpost is a **third category** — a remote, self-standing production site —
and no code path recognises it. That single fact explains both bugs.

*(One lead investigated and ruled out: `create_estate` does set
`self.routes_dirty = true`, so the route matrix IS rebuilt when an outpost is
founded. The rebuild is not the problem.)*

---

### 1 · The measured findings

#### G1 — The flow overlay silently drops every outpost's trade

`commands/query_commands/flow.rs`, the Dynamic Trade Flow overlay — the layer
that draws the trade lines on the map:

```rust
for h in &sim.hubs {
    if h.is_estate { continue; }        // ← outposts never get a node
    node_of.insert(h.id, cc.cidx(cx, cy));
}
...
for &(a_id, b_id, vol) in &sim.flow_year {
    let (s, g) = match (node_of.get(&a_id), node_of.get(&b_id)) {
        (Some(&s), Some(&g)) if s != g => (s, g),
        _ => continue,                  // ← outpost endpoint ⇒ flow discarded
    };
```

An outpost never enters `node_of`, so **every `flow_year` entry with an outpost
at either end fails the lookup and is dropped**. This is the direct answer to
"why are there no understandable routes there": the outpost's trade is not
missing from the simulation, it is missing from the *picture*.

It has a second, quieter cost: that volume is discarded rather than reassigned,
so the trunk widths on the rest of the map **under-report** by whatever the
outposts were carrying.

Note the contrast with `read_trade.rs`, which handles the same problem correctly
and independently:

```rust
let city_of = |h: u32| match sim.hubs.get(h as usize) {
    Some(x) if x.is_estate && x.parent >= 0 => x.parent as u32,   // collapse
    _ => h,                                                        // keep
};
```

That guard checks `parent >= 0`, so a co-located estate collapses to its city and
a **parentless outpost correctly keeps its own identity**. The flow overlay never
got the same treatment. Two places solve one problem, one of them right.

#### G2 — Outposts are excluded from all three "no dead city" lifelines

`rebuild_routes` (`tick/production.rs`) has three guarantees, added specifically
so no settlement is a dead dot that can never trade. All three open with the
same filter:

```rust
let real: Vec<usize> = (0..n)
    .filter(|&i| !self.hubs[i].is_estate && !self.hubs[i].abandoned)
    .collect();
```

- **#6 `MIN_GUARANTEED_PARTNERS`** (4 nearest same-component partners) — skipped.
- **#6b hub-and-spoke market lifeline** (a route to a real market) — skipped.
- **#6c coastal cabotage** (short-sea link to another component) — skipped.

So the one class of hub that is remote, tiny (`OUTPOST_MAX_POP` = 800) and
newly-founded — i.e. **the most likely to be stranded** — is the only class
denied every anti-stranding guarantee. An outpost gets a route only from the
generic pass, which requires both:

- within the trade horizon, `TRADE_MAX_DIST_FRAC = 0.24` of world width, and
- **`component[a] == component[b]`**, because `base_days` (the real pathfound
  lane matrix) only covers the founding hub set and an outpost's index is always
  `≥ base_n`.

#### G3 — An outpost's component can go stale and is never repaired

The outpost copies `component` from its founder at creation. The repair pass
`rescue_tiny_components` skips estates, and its estate fixup is:

```rust
if self.hubs[i].is_estate {
    let p = self.hubs[i].parent;
    if p >= 0 && ... { self.hubs[i].component = self.hubs[p as usize].component; }
}
```

`p >= 0` is **false for every outpost**, so an outpost is never re-synced to its
founder's component. Today `components_rescued` is a one-shot flag that fires at
campaign start, before any outpost exists, so this is currently latent rather
than active — but it is a live trap for any future pass that reassigns
components, and combined with G2 it means a mis-set component silently removes
an outpost from trade with no lifeline to catch it.

#### G4 — There is no transshipment anywhere in the model

This is the user's actual design point, and it is not a bug — the capability
does not exist.

`rebuild_routes` produces a **direct point-to-point** `days[a][b]` matrix, and
dispatch ships **origin → destination in one leg**. There is no notion of a
cargo moving inland-site → port → distant market. So a hard-access interior site
either reaches a market *directly* or does not trade at all. There is no way for
a port to earn its living by *handling other people's goods*, which is what an
entrepôt is and what nearly every real pre-modern trade city was.

Related state exists but is unused for this: `TickHub` already carries
`coastal`, and `in_by_sea` / `in_by_land`. Worldgen already finds the right
places — step 7a's `generate_trade_sites` scores straits, isthmuses, passes and
great river mouths precisely because "a great port need not sit on the best
farmland". The campaign never reads that idea.

---

### 1b · Is the outpost logic "explore, then site for movement, then relay"?

Asked directly: does the model explore first, site outposts where goods can
actually move, and chain several posts so cargo gets from cart to river boat to
ship? **Measured answer: no, on all four counts.** Each is a separate finding.

#### G5 — There is no exploration. Houses are omniscient from day one.

There is no `explored` / `discovered` / `surveyed` state anywhere in the tick.
`colonizable` is a **whole-world list snapshotted at campaign start**, and
`try_found_house_outpost` scans *all of it* on every call, taking the best-scoring
site within `COLONY_MAX_KM` of the founder's network. A house in year 1 already
knows the trade value of every site on the planet.

Expeditions do exist (`expedition_launch_pass`, `route_prospects`,
`envoys.rs`) — but they are a **parallel, ornamental system**: a prospect feeds
the overlay and a goal check, and gates nothing. Nothing an expedition discovers
is required before an outpost is planted there.

So an outpost is not a venture into the unknown; it is an optimal pick off a
complete map. That is the opposite of the intended fiction, and it is also why
outposts appear in surprising places — nothing stops a house reaching past
everything it knows.

#### G6 — Outpost siting ignores the very flags that mark a transshipment point

`ColonizeSite` already carries exactly the right data:

```rust
/// River-mouth / DELTA (fertile coastal alluvium — a natural port + granary).
pub delta: bool,
/// Land→sea CHOKEPOINT (strait / isthmus / portage where cargo transships and
/// tolls can be levied — Venice/Bruges/Constantinople-style prize sites).
pub chokepoint: bool,
```

`maybe_found_settlement_colony` (the **city**-founded path) reads both and prices
them heavily — `delta +0.60`, `chokepoint +0.80`, against `coastal +0.35`.

`try_found_house_outpost` (the **house** path — the outposts in question) scores:

```rust
let trade_score = s.trade_value + if s.coastal { 0.30 } else { 0.0 };
```

**Neither `delta` nor `chokepoint` is read.** A river mouth and a strait — the two
site kinds that exist in the data *precisely because* cargo transships there —
carry no weight at all when a merchant house picks where to trade from. The
outpost is sited on what the ground yields, with almost no weight on whether the
cargo can leave. That is the direct cause of the reported "hard-access" posts.

The city path also refuses to let an inland founder plant a coastal colony ("no
fleet tradition"); the house path has no equivalent rule.

#### G7 — No relay. Outposts are picked independently, never as a chain.

`maybe_found_house_outpost` takes the richest houses (`OUTPOST_MAX_PER_CALL` = 3)
and each independently scores its own best site. **No pass anywhere considers two
posts together**, or plants one *because* another needs an outlet. There is no
staging, no relay and no notion of a corridor of posts. The "several outposts, at
greater cost, so cargo can reach the water" pattern is not implemented in any
form.

#### G8 — Water is not cheaper than land, and rivers are not a mode at all

This is the deepest of the four, and it reaches well past outposts.

```rust
// Route mode: a sea voyage when both ends are coastal, else overland.
let sea = self.hubs[a].coastal && self.hubs[b].coastal;
```

Mode is decided solely by whether *both* endpoints are coastal, and all it does is
choose which fleet counter to decrement (`cap_sea` vs `cap_land`). The cost is
untouched: `days` comes from `rebuild_routes` as `dist · days_per_cell` with a
single global `days_per_cell` and **no land/sea/river distinction anywhere**.
`good_freight` then multiplies those same days by the good's `bulk`.

So a ton of stone crosses 500 km of mountain for exactly what it costs to sail it
500 km along a calm coast.

And rivers are not a mode at all:

```rust
cap_land[i] = (h.fleet_river + h.fleet_caravan) as i32;
```

**River barges and ox-carts are pooled into one interchangeable "land" capacity.**
A river confers no cost advantage, so nothing in the economy has any reason to
follow one.

That water carriage was roughly an order of magnitude cheaper than overland haulage
is arguably *the* organising economic fact of the pre-modern world — it is why
cities sit on rivers and estuaries at all. Its absence is very likely a major
contributor to the market-integration failure CLAUDE.md already names as the
largest known economy defect (the basket price/distance gradient reading **−0.064**,
with 0 of 6 goods showing any gradient, where the historically correct sign is
positive). If distance costs the same whatever the terrain, no trade concentrates
on water and no economic geography forms.

**This should be tested before it is designed around.** The claim "differentiated
transport cost is a main cause of the flat gradient" is a hypothesis with a clear
instrument — `econ_fidelity_scorecard` already measures the gradient — and
CLAUDE.md §8.15's cautionary tale is precisely about concluding from an untested
mechanism inside an already-distorted economy. Measure first.

#### Slice D — Transport modes with real relative costs *(G8; the largest lever)*

Give a route a **mode** with its own per-day cost: sea ≪ river < road < track.
Concretely: keep one `days_per_cell` but attach a per-mode multiplier, and split
`cap_land` back into `fleet_river` and `fleet_caravan` so a river barge is not an
ox-cart. A route is river-borne where both endpoints sit on the same navigable
system — worldgen already knows which rivers are navigable (`River.navigable`,
used by the province partition), so the data exists and only needs carrying into
the campaign snapshot.

**Gate:** the existing `econ_fidelity_scorecard` price/distance gradient. This is
the honest test of whether G8 is the cause: if differentiating transport cost does
not move the gradient off −0.064, the hypothesis is wrong and should be recorded
as a negative result rather than quietly kept.

**Order note:** Slice D plausibly subsumes much of Slice C. If water carriage is
genuinely cheap, an outpost near navigable water becomes valuable *automatically*
through the ordinary route cost, without a bespoke entrepôt rule. **Build D before
C2**, and re-measure whether C2 is still needed.

#### Slice E — Site outposts for movement, and explore first *(G5/G6/G7)*

- **E1 (small):** read `delta` and `chokepoint` in `try_found_house_outpost` with
  the same premiums the colony path already uses, and add the inland-founder rule.
  A few lines, and it directly targets the reported problem.
- **E2:** gate outpost founding on a prior expedition or route prospect reaching
  that region, turning the existing ornamental expedition system into the real
  precondition it was written to be. This is also `MERCHANT_VESSELS_AND_
  INFORMATION_PLAN.md` stages 5-6, which already own the survey-agent design —
  **coordinate rather than duplicate.**
- **E3 (only after D):** a relay pass — a house with a productive post whose
  outlet is poor plants an intermediate post toward navigable water, at a premium.
  This is the user's "several outposts at bigger price" and it is deliberately
  last: with Slice D in place the ordinary site scorer may already produce it.

**Gate:** `an_outpost_prefers_a_site_its_cargo_can_leave` — given two sites of
equal `trade_value`, one on a delta and one landlocked, the delta must win. Fails
today (the flags are not read).

---

### 2 · The slices

#### Slice A — Draw outpost trade *(fixes G1; smallest, most visible)*

In `flow.rs`, replace the `if h.is_estate { continue; }` skip with the same
`city_of` mapping `read_trade.rs` already uses: an estate **with** a parent maps
to its parent's coarse node (so its flow is *credited to the parent city*, not
discarded); an estate **without** a parent gets its own node.

This makes outpost trade visible AND stops the silent volume loss on the rest of
the map. Extract `city_of` into one shared helper so the two call sites cannot
drift again — they already have.

**Gate:** `outpost_flow_is_never_dropped` — with an outpost trading, assert the
sum of volume reaching the overlay equals the sum in `flow_year` (today it is
strictly less whenever an outpost trades).

#### Slice B — A remote site is a real trade node *(fixes G2/G3)*

Introduce one predicate and use it everywhere the three lifelines run:

```rust
/// A production site that stands on its own ground rather than inside a
/// parent city — today exactly the house trade outposts (colony_kind 2).
/// It is an estate for OWNERSHIP purposes and a real place for ROUTING.
fn is_remote_site(&self, i: usize) -> bool {
    self.hubs[i].is_estate && self.hubs[i].parent < 0 && !self.hubs[i].abandoned
}
```

Widen `real` in `rebuild_routes` to `!is_estate || is_remote_site`, so an outpost
gets `MIN_GUARANTEED_PARTNERS`, the market lifeline and cabotage like any other
settlement. Also let `rescue_tiny_components`' estate fixup fall back to the
**founder** (`founder_hub`) when `parent < 0`, closing G3.

Deliberately *not* widened: city rankings, society/pops, government — an outpost
should stay out of those. The predicate is about **routing**, not about promoting
outposts to cities.

**Gate:** `a_remote_outpost_always_has_a_market` — found an outpost at the edge of
its founder's range and assert it has ≥1 finite route to a same-component market.
Fails today.

**Watch:** this adds hubs to the lifeline loops, which are O(n²) over `real`. The
`econ_` bands and the dynamics run must both be re-checked — this changes who
trades, so it is *not* a bit-identical change and should not be claimed as one.

#### Slice C — The entrepôt *(G4 — the real feature)*

Two parts, in order. **C1 is worth building alone**; C2 only makes sense after it.

**C1 · Two-leg routing through an outlet.** For a hub with poor direct
connectivity, allow one intermediate leg: `days[a][b]` may be composed as
`days[a][p] + dwell(p) + days[p][b]` where `p` is an **outlet** — a coastal hub,
a navigable-river hub or a lake port, in the same component as `a`. Cap it at
**one** transshipment (a pre-modern cargo was not containerised; two-leg is the
honest ceiling and it keeps the matrix build from becoming an all-pairs shortest
path). `dwell(p)` is a real cost in days — that cost is exactly what makes a good
port valuable and a bad one bypassed.

The handling port should **earn** from it: route a share of the transit value to
`p`'s `treasury` / `trade_wealth`. That is the entrepôt's whole economic
character, and it gives the model something it currently lacks — a city that is
rich because of *where it is*, not what it grows.

**C2 · Founding a port where one is needed.** A new yearly pass, reusing
`maybe_found_house_outpost`'s machinery with a different site scorer: when a
house holds ≥2 remote sites in one region whose best outlet is poor, it founds a
**port** at the best coastal / river-mouth / lakeshore site *between them and
open water*. Score by outlet quality (navigable water, shelter, low approach
cost) rather than by `trade_value` — a port is not a plantation and the existing
scorer would never pick the right cell.

This is the "one more outpost as a trade hub which comes naturally" the user
described, and it is the historical pattern (Phoenician emporia, the Venetian
*fondaco*, Hudson's Bay factories). `maybe_graduate_outpost` already exists to
let such a post mature into a real city, so the lifecycle is already there.

**Gate:** `an_inland_site_trades_through_its_port` — an inland outpost with no
direct market route must reach one via an outlet, and the outlet's trade wealth
must rise as a result. And a companion the plan should not ship without:
`transshipment_does_not_inflate_total_trade` — two-leg routing must **move**
value, never create it (the same zero-sum discipline as
`a_division_moves_capital_and_creates_none`).

---

### 3 · Suggested convention (CLAUDE.md rule 32)

> **`is_estate` is an OWNERSHIP flag, not a geography flag.** An estate with
> `parent >= 0` is co-located inside its parent city and collapses to it; an
> estate with `parent < 0` is a remote place standing on its own ground and must
> be routed, drawn and rescued like any settlement. Code that branches on
> `is_estate` alone will be wrong for one of the two, and the failure is silent
> in both directions — a co-located estate draws a zero-length route, a remote
> outpost is dropped from the map entirely.

---

### 4 · Risks

- **Slice B changes who trades**, so it moves the economy. Run `econ_` and the
  dynamics test; expect small shifts and check they are in the right direction
  rather than asserting nothing moved.
- **Slice C1 is the cost risk.** Composing routes through outlets is a
  shortest-path over the hub graph, and `rebuild_routes` is already O(n²) and
  runs whenever a hub is added. Mitigation: restrict candidate outlets to a small
  precomputed set per component (the top few coastal/river hubs by population),
  which keeps the extra work O(n · |outlets|) with `|outlets|` in the tens.
- **Slice C2 adds hubs**, and `MAX_TOTAL_ESTATES` already bounds the hub list.
  A port should draw from the `OUTPOST_RESERVED_ESTATES` reservation rather than
  competing with ordinary estates — the same starvation problem, and the same
  fix, that outpost founding already needed once.

### 5 · Deliberately NOT in this plan

- **Individual vessels with manifests and locations.** A vessel is still three
  counters on `House` — see `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md`, which
  owns that work. Slice C1 routes *cargo*, not ships, and should not pretend
  otherwise.
- **More than one transshipment leg.** Capped at one, deliberately (§C1).
- **Promoting outposts to cities to fix this.** `maybe_graduate_outpost` already
  handles maturation on its own terms; widening it to paper over a routing bug
  would be the wrong fix.
- **Re-pathfinding real lanes for hubs founded mid-campaign.** A tick has no tile
  access; `terrain_route_mult` is the documented stand-in and stays.

---

# Part III · Exploration, the known world, transport modes

**Status: §8.1 (the manufactured-goods province-list bug) BUILT — a
`Distribution::Manufactured` good is now excluded from `campaign_province_
potential`'s goods list by construction (an `is_manufactured` map alongside the
existing `is_deposit` one), fixing books/etc. appearing as a province product
regardless of what put a stray non-zero byte in that good's belt slot. §8.2's
richer detail turned out to be MOSTLY already shipped by the time this plan was
written — `ProvinceInspector.tsx` already showed mean ore grade/workings/depth,
locality presence, potential vs actual exploitation and market share (via the
separate `campaign_province_goods` exploitation query) — so the only genuine
gap was the served `grade_label` WORD alongside the star rating; added as
`ProvinceGoodPotential.grade_word` (backend) and wired into the stars' tooltip
(frontend), both read-only over existing state, no new persisted field.
§4 (transport modes) is BUILT IN PART — see its own section for what and why.
Everything else in this Part — knowledge/fog/exploration (§1-§3) — is NOT
built.** All open questions are decided (§6);
this is buildable as written. Companion to
`OUTPOST_CONNECTIVITY_AND_ENTREPOT_PLAN.md`, which measured the problems it
answers.

---

### 0 · What the code does today (measured)

#### Three founders already exist, and the timing already fits

| Pass | Founder | Gate |
|---|---|---|
| `maybe_found_settlement_colony` | a **CITY** — large, food-secure, prosperous, under population pressure, with treasury | `COLONY_START_TICK` = **year 30** |
| `try_found_house_outpost` | a **HOUSE** — any clearing `OUTPOST_FOUND_WEALTH`, richest first, ≤3 per call | `OUTPOST_START_TICK` = **year 30** |
| `maybe_found_caravanserai` | a **CITY** — waystations on long land ties | `expansion_ok` |
| `maybe_graduate_outpost` | promotes a house outpost into a full colony in place | age + pop + wealth |

`expedition_launch_pass` is backed by **houses** (non-guild, wealth ≥
`EXP_MIN_HOUSE_WEALTH`), gated at `EXP_START_TICK` = **year 15**.

So the "exploration is the pre-stage for colonisation at year 30" structure the
brief asks for is **already the shape of the code** — expeditions run first, the
two founding passes open at year 30. What is missing is the *causal link*: today
the two are unrelated systems and expeditions gate nothing.

**Decision:** move `EXP_START_TICK` to **year 25**, per the brief. Five years of
exploration then feed the year-30 founding passes.

#### The three defects this design fixes

Measured in the companion plan: houses are **omniscient** (no
`explored`/`discovered`/`surveyed` state exists; `colonizable` is a whole-world
list scanned in full every call), outposts are **sited without regard to whether
cargo can leave** (`delta`/`chokepoint` exist on the site struct; the house path
reads neither), and **water carriage costs the same as overland** (one global
`days_per_cell`; `fleet_river` pooled with ox-carts in `cap_land`).

---

### 1 · Knowledge: two kinds, and they behave differently

The central design decision, and the one that makes this more than a map filter:

> **Where a place is** is shareable. **What it costs there** is not.

- **MAP knowledge** — that a province and its towns exist, roughly where, roughly
  what it yields. Acquired by expedition; **freely exchanged** between houses and
  settlements that trade with each other. This is what the fog layer draws.
- **MARKET knowledge** — live prices, stocks, what sells. Acquired **only by
  direct presence**: an office or bailo there, or your own traders arriving. Not
  transferable by contact, and it goes **stale** when presence lapses.

That split is historically right (portolan charts and the *Periplus* circulated
widely; a Venetian house's Alexandria price book did not) and it is what makes
information an *asset* rather than a display toggle.

#### 1.1 Encoding

Held **per province, per knower**. Provinces are already the world↔campaign join
(FIX_PLAN B1), already carry `prov_neighbors` for contiguity, and at 200-400 km
are about the resolution at which pre-modern knowledge actually existed. A
per-cell fog would be a large per-knower raster implying a precision nobody had —
and storage is the binding constraint: 40 houses × 300 provinces is trivial;
per-cell is not.

```rust
/// What one knower knows about one province. Serde-defaulted; an empty map
/// means "knows nothing", and §5's seeding is what keeps that from stranding
/// an existing campaign.
struct Known {
    level: u8,        // 0 unknown · 1 reported · 2 surveyed · 3 established
    since_tick: u32,  // when this level was reached — drives MARKET staleness
    source: i32,      // who told us (-1 = our own expedition)
}
```

**Knowers are both houses and cities** — forced by the code, not chosen: both
found things (§0), so both must know things. A city's knowledge lives on
`TickHub`, a house's on `House`.

#### 1.2 How the levels move

- **0 → 1 (reported):** contact. A trading partner that knows a province at ≥2
  passes it on at 1. Also what a *failed* expedition leaves behind — a partial
  report, because a total loss teaches the player nothing and reads as arbitrary.
- **1 → 2 (surveyed):** an expedition returns. This is the gate for founding.
- **2 → 3 (established):** we hold or trade there — an office, an outpost, a
  colony, a bailo.

Map knowledge **never decays**: a coast once charted stays charted. Market
knowledge is not a level at all — it is `since_tick` on a level-3 entry, and it
ages the moment presence lapses.

---

### 2 · Expeditions become the acquisition mechanism

`expedition_launch_pass` and `route_prospects` already exist and gate nothing.
This gives them their job.

- **Backers:** houses (as today) **and** cities, since cities found colonies too.
- **Routing:** follow known trade routes where they exist; otherwise **coasts and
  rivers**. This is the historical pattern (the coastal crawl before open-water
  navigation; the Russian frontier moved along river systems; the American west
  along the Missouri and the Platte) and it is cheap once Slice D makes navigable
  water known to the campaign.
- **Range:** bounded, and deliberately short-ish. Distance is what makes the
  hazard real rather than decorative.
- **Return:** the expedition raises every province along its path to level 2 for
  its backer, and neighbours to 1.

#### 2.1 Natives — the hazard

Per the brief: natives attack intruders (Cortés; the American frontier). This is
the mechanism that makes distance cost something.

**The honest constraint: the model has no native population.** Provinces carry
`prov_rural` but there is no notion of an unincorporated people, and inventing
one properly is a large design in its own right (arguably `historical-society`'s
question, not this plan's).

So build it as a **hazard field, not a polity** — explicitly a stand-in, in the
same documented-proxy tradition as `geology.rs`'s phase-2 climate term:

```
hostility(province) ≈ f(distance beyond the backer's known frontier,
                        province emptiness — no hub, low prov_rural,
                        terrain difficulty,
                        whether any prior expedition has been here)
```

An expedition rolls against it per leg. Outcomes: **returns** (full report),
**limps home** (level 1 only), **lost** (no report, backer eats the cost, a
chronicle entry naming the province). Hostility should **fall** once a province
is established — contact, or conquest, but the model need not say which.

This is deliberately not a native *faction*: no armies, no territory, no
diplomacy. If that is wanted later it replaces this field cleanly, because
nothing else reads it.

---

### 3 · What the fog gates — and what it must not

Per the brief: **trade mechanics are unchanged**. What changes is *reachability*
— a house or city cannot reach a city it has never heard of.

Concretely, knowledge gates:

- **founding** — an outpost/colony needs level ≥ 2 at that province;
- **goals** — a house goal cannot target an unknown province;
- **trade partners** — `rebuild_neighbors` prunes partners in provinces the
  knower does not know at ≥ 2.

It does **not** change price formation, dispatch, freight, or the market solver.
The `days` matrix, `good_freight`, and the needs ladder are untouched.

#### 3.1 The economic risk, and the mitigation

**Pruning trade partners moves the economy** — that is unavoidable, and it will
shift the `econ_` bands. It is the one part of this design that is not additive,
and it should not be shipped claiming otherwise.

Mitigation, and it is a good one: **seed knowledge at campaign start from the
existing trade network.** Every province containing one of a knower's holdings or
current trade partners starts at level 3, its neighbours at 1. Then:

- the **founding economy is unchanged** — day-one partners are all known;
- the fog constrains **expansion only**, which is exactly the intent;
- an existing save is not stranded (§5).

**Gate:** `econ_fidelity_scorecard` before/after with fog on and seeding in
place. A small drift is acceptable and expected; a large one means the seeding is
wrong, not that the fog is.

#### 3.2 The market-knowledge half

"Prices only where you have presence" is **already designed** as stage 4 of
`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` — a house trades on the price it
*believes*, with a spread set by how fresh its knowledge is (never been →
surveyed → office → controls the seat). That is the same mechanism the brief
describes.

**Do not build it twice.** This plan supplies the `Known` state and the
never-been / surveyed levels; that plan owns the belief-price and spread. They
should land together, and stage 4's own gate — long-haul trade volume must not
collapse — governs.

---

### 4 · Transport modes (build this FIRST)

Restated from the companion plan because everything above is easier with it.

**Status: the river-plumbing HALF of this is BUILT; the mode/capacity split is
NOT.** Measured before touching anything: the cost DIFFERENTIATION this section
asks for (sea ≪ river < road, priced ≈1:4:8 per Masschaele) already existed —
`commands/query_commands/mod.rs::build_coarse_cost` has carried it since an
earlier, undocumented-here session (CLAUDE.md §7's own comment cites "slice 5"
of the ports/junctions work). What was actually missing was much narrower and
entirely mechanical: `compute_route_days_matrix` — the function that builds the
campaign's real pathfound `base_days` at `campaign_start_sim` — passed
`rivers_json = ""` unconditionally, with a comment claiming "campaign has no
overlay JSON". River geometry (`sim::rivers::River`, incl. `.navigable`) was
never persisted to the world's `metadata["rivers"]` key by any WORLDGEN code
path — only `persist_overlays`, called by the frontend right before a manual
save, ever wrote it. So on the single most common flow — generate a world,
start the campaign, never having saved and reloaded first — the campaign's real
day-cost matrix had zero river data to read, and the sea:river:road
differentiation that already existed in the cost grid fired for sea vs. land
but **never once for a navigable river**, however genuinely navigable it was.

Fixed with no new mechanism: `sim_commands::persist_rivers` (called from all
four sites that compute `extracted_rivers` — `sim_rivers_hydrology`,
`sim_refresh_hydrology_biology`, `sim_run_all`, `sim_run_all_from_terrain`) now
writes the just-extracted river geometry straight to `metadata["rivers"]` the
moment it exists, rather than only on an explicit save; `compute_route_days_
matrix` reads that key (best-effort — a missing/unparseable value degrades to
the old empty string, so an unusual world is never worse off) instead of the
hardcoded `""`. `cached_coarse_cost`'s cache key already hashes `rivers_json`,
so this cannot collide with any other caller's grid.

**NOT built:** splitting `cap_land` into `fleet_river`/`fleet_caravan` for
CAPACITY purposes (G8's other half — "river barges and ox-carts pooled into one
interchangeable land capacity"). Doing that honestly needs a per-route MODE
signal (is this hub pair's real pathfound route river-dominant?), which nothing
currently records — `base_days` carries only a cost, not a mode — and deriving
one would mean either persisting a `route_mode` matrix alongside `base_days` or
re-deriving it per dispatch from the coarse-cost grid, either a real addition
beyond the plumbing fix above. Left for the same follow-up as Slice C.

**UPDATE — now validated.** `commands/real_world_diagnostics.rs` (test-only,
`#[ignore]`d) is the harness this section originally said was missing: it
builds a REAL world through the actual Tauri commands (`sim_run_all` →
`compute_economy` → `finalize_world` → `campaign_start_sim` → `campaign_advance`)
against a real `WorldDb`, via `tauri::test::mock_app()` for a genuine
`State<WorldDb>` outside a running app — not a hand-built fixture. Needs
`tauri = { features = ["test"] }` under `[dev-dependencies]` (Cargo.toml),
since a downstream crate's own `cfg(test)` does not turn on a dependency's
feature flags. Run it with:

```bash
cd src-tauri && cargo test --lib real_world_price_distance_gradient -- --ignored --nocapture
```

**Measured** (300×150 world, seed 424242, 224 real settlements/hubs, 20 years):
grain price-gap × distance **r = 0.092** over 36,868 hub pairs — POSITIVE, the
historically correct sign, on a real generated world with the river-plumbing
fix from this section in place. This is the first measurement of this
gradient anywhere in the codebase that isn't the synthetic 30-city reference
world's own r ≈ 0.11-0.14 (see below) — the two now roughly agree, which is
reassuring but not itself proof the river fix MOVED anything, since there is
no real-world "before" run to diff against (a 300×150 world takes ~6 minutes
end to end; a paired before/after run is future work for whoever tunes this
further). Record: the sign is right and the harness works. Whether the river
discount specifically is what keeps it positive on a real world, versus the
sea/land split alone already carrying it, is NOT separated out here.

Original framing, kept for the record — it explains why this was picked first,
before the number above existed. The full ask, unchanged: give a route a full
**mode** with its own per-day cost, capacity and risk — sea ≪ river < road <
track. Per the brief all three axes differ, and water wins on all three, which
is what makes it preferable with no special case.

**Why this was picked first:** it is the biggest lever on the known
market-integration defect (basket price/distance gradient **−0.064**, 0 of 6
goods showing any gradient, where the correct sign is positive — though this
session's own `econ_` run against the current tree read **+0.139** basket /
**+0.114** price with 5 of 6 goods showing a positive gradient, on the
synthetic reference world unaffected by anything here, meaning the −0.064
figure itself is now stale and should be re-measured before being cited
again); it is testable against an instrument that already exists; and **it may
make later work unnecessary** — if water carriage is genuinely cheap, a site
near navigable water becomes valuable through ordinary route cost, with no
bespoke entrepôt rule and no siting special case.

**Gate:** the price/distance gradient on a REAL generated world
(`real_world_price_distance_gradient`, above) — **partially cleared**: the
sign is positive (r = 0.092), which is the claim that matters most, but no
paired before/after run exists to attribute that specifically to the river
fix rather than to the sea/land split that predates it. Whoever next touches
transport cost should run a before/after pair on the SAME seed before
claiming credit either way — the instrument now exists to do it.

---

### 5 · Migration — existing campaigns

Every field is serde-defaulted; an absent knowledge map means "knows nothing",
which would strand a year-200 campaign. So on load, if the map is empty, **seed
it from current holdings and trade partners** (§3.1) rather than starting blind.
Same seeding path as a new campaign, so there is one code path, not two.

---

### 6 · Decisions (all previously-open questions, now closed)

| # | Question | Decision |
|---|---|---|
| 1 | Who is the knower? | **Both houses and cities** — forced by §0: both found things. Exploration initiator = the colony founder. |
| 2 | Does knowledge decay? | **Map: never.** Market: not a level — `since_tick` ages once presence lapses. |
| 3 | Does the fog gate trade? | **Yes, reachability only** — unknown cities are not partners. Price formation untouched. Start-seeded (§3.1) so only expansion is constrained. |
| 4 | Player verb or AI? | **AI-driven**, matching every other campaign system. A player verb is a clean later addition. |
| 5 | What are "natives"? | A **hostility field**, explicitly a documented proxy — not a polity. No native faction, armies or diplomacy. |
| 6 | Existing campaigns? | **Seed from holdings + partners** (§5). |
| 7 | Expedition timing | `EXP_START_TICK` → **year 25**; colonisation stays **year 30**. |

---

### 7 · Build order

1. **Transport modes** (§4). Biggest lever, existing instrument, may subsume later work.
2. **Manufactured-goods filter + province detail** (§8). Small, self-contained, visible.
3. **Outpost siting reads `delta`/`chokepoint`** (companion plan E1). A few lines.
4. **Knowledge + expeditions + natives** (§1-§3). The large one — and only after
   re-measuring whether 1 and 3 already produced sensible siting.

---

### 8 · Province view: the manufactured-goods bug, and richer detail

#### 8.1 The bug — measured

The Inspector lists **Books & Manuscripts** as a province good. It is manufactured
(`goods_spec.rs`: `mg("books", … inputs: paper 0.8, dyes 0.1)`, which sets
`Distribution::Manufactured`) and has neither belt nor deposit.

The chip row filters `!g.is_deposit` and nothing else; its source,
`campaign_province_potential`, filters only on magnitude:

```rust
for g in 0..ng {
    let belt = sim.prov_good_belt.get(idx).copied().unwrap_or(0.0);
    if belt <= PROV_GOOD_ABSENT_BELT { continue; }
    goods.push(ProvinceGoodPotential { … });
}
```

**Nothing anywhere filters `Distribution::Manufactured`.** The query already
builds an `is_deposit` map from the same specs and passes it through as a flag;
there is no manufactured equivalent.

The generation side is correct — `compute_trade_goods` writes
`buf.goods[slot] = vec![0u8; n]` for a manufactured good, and an all-zero belt
lands exactly on `PROV_GOOD_ABSENT_BELT` and is excluded. So the missing guard is
the whole defect.

**Honest limit:** that means something put a *non-zero* belt in the `books` slot
in this particular world, and which of two candidates it is cannot be told from
code alone — (a) a **stale column** at a reused index (the world predates `books`
in the spec, or the spec was edited after generation; CLAUDE.md §8.20's "fixed
indices in `TileData.goods`" is exactly this hazard), or (b) a length mismatch
between `Province.good_belt` and the campaign's `goods.len()` misaligning the flat
`prov_good_belt` row. **The fix does not depend on the answer** and should not
wait for it.

**Fix:** exclude manufactured goods **structurally**, as `is_deposit` already is,
and pass a belt-good mask into `generate_provinces` — which today receives no
goods spec at all and therefore *cannot* distinguish a manufactured column from a
belt column. That is the structural root, and fixing it makes the class of bug
impossible rather than repairing one world's data.

**Gate:** `a_province_never_lists_a_manufactured_good`.

#### 8.2 Richer detail — all from state that already exists

- **Per good:** belt quality **and its grade word** (the served
  `deposits::grade_label` vocabulary — coarse/ordinary/good/fine/exquisite,
  §8.19), area covered, whether a named **locality** sits here with its grade
  (`GoodLocality` already carries `name`/`grade`/`extent`/`river_fed`), live
  `exploitation` vs `potential` (§2.5 computes both), and `market_share`.
- **Per deposit:** `ProvinceDepositDot` already carries `grade`, `extent` and
  `depth` per working — **none of which is shown**. Depth especially: "flooded"
  is a real economic fact (§8.16's "visible but largely LOCKED"), and nothing
  reads `depth` anywhere yet — `DEPOSITS_AND_MINING_PLAN.md` slice 4's own note.

Both are **read-only presentation of existing state** — no new sim, no new
persisted field — which makes this the cheapest item in the set.

---

### 9 · Deliberately NOT built

- **A native faction** — armies, territory, diplomacy. §2.1 is a hazard field and
  says so.
- **Per-cell fog.** Province granularity, for storage and honesty (§1.1).
- **Belief-priced trade.** Owned by `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md`
  stage 4; this plan supplies the state it reads (§3.2).
- **A player expedition verb.** AI-driven first (§6.4).
- **Map-knowledge decay.** A charted coast stays charted (§6.2).
