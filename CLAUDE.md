# WorldForge 2 — Fantasy World Map Generator

Tauri 2 desktop app that procedurally generates a fantasy world (tectonics →
climate → hydrology → life → trade) and then runs a **living campaign economy**
on top of it (merchant houses, banks, coinage, wars, plagues, colonies).

**Stack:** Tauri 2 (Rust) · React 18 · PixiJS 8 · Zustand · SQLite (rusqlite) · zstd
**Layout:** `WorkflowPanel (left) | Map (center) | Toolbar (right) | StatusBar (bottom)`
**Two halves:** a **World** pipeline (`sim/step*/`, frozen on finalize) and a
**Campaign** simulation (`sim/campaign/tick/`, ~16.7k lines split by theme).

---

## 1. Quick Start — Dev Commands

```bash
run.bat                # Windows: install deps, pull updates, launch dev
npm run tauri dev      # Launch dev mode (Vite + Cargo)
cargo check            # Rust type-check only (run from src-tauri/)
npx tsc --noEmit       # TypeScript type-check only
cargo test --lib tick::tests                                   # campaign-sim unit + dynamics tests
cargo test --lib simulate_decades_reports_dynamics -- --nocapture  # WATCH the living economy (5-yearly digest)
cargo test --lib earth_ -- --nocapture                         # EARTH CLIMATE FIDELITY scorecard (§2.3)
cargo test --lib econ_ -- --nocapture                          # ECONOMY FIDELITY scorecard (§2.5)
cargo test --release --lib bench_phase2 -- --ignored --nocapture                 # phase-2 (terrain) ms breakdown
cargo test --release --lib bench_ocean_atmosphere -- --ignored --nocapture       # phase-3 ms breakdown (§8.9)
cargo test --release --lib ocean_atmosphere_field_checksums -- --ignored --nocapture  # phase-3 bit-exactness
```

> The full Tauri build needs GTK/WebKit system libs. On a headless Linux box:
> ```bash
> sudo apt-get update            # REQUIRED first — a stale index 404s on the .debs
> sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev
> ```
> That makes `cargo check` / `cargo test` work (the GUI can't be launched, but the
> sim + types do compile). A cold `cargo test` build takes ~6 min.
> A **docs-only change** (like editing this file) needs no build.

---

## 2. STANDING RULES (non-negotiable)

### 2.1 Always iterate & test the simulation
Before the FINAL push of a batch touching `sim/campaign/tick/` (economy, houses, banks,
coinage, war, crashes, trade — see §2.9 for what a batch is) you MUST run the living simulation and read the dynamics, not just
type-check. The world is meant to be DYNAMIC — houses rise and go **defunct**,
banks are chartered and **fail**, poleis mint coin, wars flare, crashes ripple.

```bash
cargo test --lib simulate_decades_reports_dynamics -- --nocapture
```

Read the 5-yearly digest and sanity-check: wealth stays bounded (no 100k blow-ups,
no negative craters — limited liability), houses turn over, banks/coins/wars/crashes
actually occur. The test HARD-ASSERTS bounded + finite wealth and that turnover
happens, so it fails if a change breaks the economy. Tune constants in `tick/mod.rs`
(`WAR_*`, `WEALTH_TAX_*`, `BANK_*`, `COIN_*`, `CONTRACT_*`) and re-run until the
dynamics read healthy. **Houses dying is expected and good — do not "fix" it away.**

### 2.2 Always push changes to GitHub `main`
Do NOT create HTML mockups/reports for visual changes. Keep the app on GitHub
`main` up to date: after a change is implemented and verified, **commit and push
to `main`** so the live app always reflects the latest work.

- Verify first (Rust: `cargo check` + the dynamics test for `tick/`; frontend:
  `npx tsc --noEmit`), then `git add`, commit with a clear message, `git push`.
- Keep commits scoped to a coherent change with a descriptive message.
- Describe visual changes in prose (before/after in words); the running app on
  `main` is the source of truth, not a mockup file.

### 2.3 Never regress Earth climate fidelity
The climate pipeline is scored against the real **Köppen-Geiger** reference map
(Kottek & Rubel, 0.5°) by `sim/step4_climate/earth_validation.rs`. After ANY change
to `step3_ocean_atmo/` or `step4_climate/` you MUST run:

```bash
cargo test --lib earth_ -- --nocapture
```

It prints a scorecard + confusion matrix and HARD-ASSERTS a floor
(`EARTH_MAIN_FLOOR`), so a change that breaks the global pattern fails the build.
**Raise the floor after an improvement** so it always guards the current best.

Measured baseline: **main-class 68.8%**, **exact-zone 37.9%** (was 66.2 / 29.0 at
`d53fdc9`; main-class was 70.1 before Terrain 2.0 slice 5's seafloor
structure — the one part of that plan touching `compute_sea_depth`/`generate_shelves`
— nudged it to 70.2, floor raised to 70.15; then LOWERED a second time, 70.15 → 68.5
and 38.8 → 37.6, when `MONSOON_LAND_PULL` was raised 1.0 → 2.2 so the Arabian Sea and
Bay of Bengal — the two of `earth_monsoon_wind_reverses`' seven monsoon sites still
not turning onshore — finally do, flipping India-Mumbai `B → A` (161mm → 1046mm,
tropical/monsoon instead of desert). See `docs/SCOREBOARD.md` 2026-09-19 and
`seasonal.rs`'s own doc comment on that constant for the full dose walk and what it
does NOT fix — Bangladesh/China-South's precipitation did not move at all). BOTH are
still asserted — `EARTH_MAIN_FLOOR` **and** `EARTH_EXACT_FLOOR`.
A third gate, `earth_monsoon_wind_reverses`, asserts the PHYSICS rather than the
score: monsoon winds must reverse between the two seasons and the mid-latitude
controls must not (now 6/7 monsoon sites, floor raised 4/7 → 5/7 to match). It
exists because the main-class floor was deliberately lowered twice — once (70.6 →
70.0) to adopt the seasonal monsoon in the first place, once more (70.15 → 68.5)
to make it actually reach the two sites it was still missing — and a point spent
on realism has to be defended by something.
Track **exact-zone** — main-class is inflated by class E scoring 99.1% for free
(polar is just "cold"). Known open errors and the plan to fix them: `docs/FIX_PLAN.md`.

### 2.4 Prompting rules — how work is commissioned here

These are rules for whoever *directs* the work, and they exist because the git
history shows the same failure repeatedly.

- **Give a gate, not a goal.** "Improve the climate" is unfalsifiable. "Get `C→B`
  below 30% without regressing the `B` or `E` rows; gate: `cargo test --lib earth_`"
  cannot be faked and produces a durable result either way.
- **Negative results are deliverables — write them down.** The `FIX_PLAN.md` A1
  entry recording *"tried `MONSOON_WIND_GAIN` 0.10 → 0.22 → 0.40, helped Mumbai,
  regressed the global score, reverted"* is worth more than most of the code around
  it. A reverted attempt that isn't documented will simply be attempted again.
- **A diagnosis is a complete task.** The A6 finding (the Antarctic Circumpolar
  Current is silently disabled on every Earth-shaped world) changed zero lines and
  was the most valuable output of its session. Commission measurement explicitly —
  left alone, everyone prefers to write code.
- **Never tune a constant without a gate that isn't the target.** Every reverted
  attempt on record is the same shape: fix a spot check, regress the aggregate.
  A spot-check win with an aggregate loss is a **revert**, not a judgement call.

### 2.5 Never regress economy fidelity
The campaign economy is scored against published pre-modern price, wage,
urbanisation and inequality series by `sim/campaign/tick/economy_validation.rs`
(Allen · Federico · Persson · De Vries · Alfani · Van Zanden). Before the FINAL push of
any batch that changed `tick/` (§2.9) you MUST run:

```bash
cargo test --lib econ_ -- --nocapture
```

Unlike the Earth gate, **most metrics are printed, not asserted** — a printed
metric outside its historical band is a *finding*, not a build failure. Assertions
cover only bands the model already satisfies, so they guard against regression
rather than encoding aspiration. **Promote a printed metric to an assertion as the
model earns it.**

This is the counterpart to §2.3 and the reason the campaign half is knowable at
all. Before it existed, 16.7k lines of economy were covered only by *mechanism*
tests (does a contract deliver, does a bank fail, is output deterministic) — none
of which asked whether a number resembled a real economy.

### 2.6 Keep the scoreboard current
`docs/SCOREBOARD.md` is the project held as ~12 numbers instead of 89k lines. It
is the fastest way for any session — or the maintainer — to answer "is this
good?". Append a row whenever a measured number moves. **Never edit an old row**;
a scoreboard whose history is rewritten cannot show a regression.

### 2.7 Keep this file true
`CLAUDE.md` is what every future session reads first, so staleness compounds. When a
change adds a module, a render layer, a sim phase or a tile column, update the
relevant map in §4/§6/§7/§8 **in the same commit**. If you find a section that no
longer matches the tree, fix it rather than working around it.

---
---

### 2.8 Test what you changed — a routing table, not the whole suite

The gates in §2.1/§2.3/§2.5 are mandatory **for the code they guard**, not for every
commit. A full `cargo test --lib` run is roughly an hour; almost every change can
only affect a slice of it. Pick rows by the paths in your diff, run the union, and
**say in the commit which gates you ran and why those**.

| You changed | Run |
|---|---|
| `docs/**`, `README`, `CLAUDE.md` | nothing |
| a `#[cfg(test)]` block / a diagnostic only | `cargo check --lib --tests`, then that test by name |
| `sim/campaign/tick/**` | per slice: `cargo check --lib --tests` + the slice's named tests; **once at the end of the batch**: `cargo test --lib tick::tests` + `econ_` (§2.1, §2.5, §2.9) |
| `sim/step3_ocean_atmo/**`, `sim/step4_climate/**` | `earth_` (§2.3) + re-read §8.9 |
| `sim/step2_terrain/**` | `elevation::tests`, `landform`, `terrain_metrics`; **not** `earth_` — `earth_validation.rs` scores a baked DEM and never calls a generator (§8.23b) |
| `sim/step1_plates/**` | `plates`, `elevation::tests`, `coastline_departs_from_the_plate_boundary` |
| `sim/step5_rivers/**`, `step6_soil_fertility/**` | those modules' own tests + `goods_` if belts move |
| `sim/step8_biological_goods/**` | `goods_` (rule 26) |
| `sim/shared/provinces.rs` | `provinces::tests` |
| `render/**`, `commands/palette_commands.rs` | `render::tile_image::tests` (+ the relevant `dump_*` sheet — §8.21's rule: look at it) |
| `commands/**` (wiring only) | `cargo check --lib --tests` |
| `src/**` (frontend) | `npx tsc --noEmit` |

Three rules:

- **Run the narrowest thing that could fail, then widen if it does.** `cargo test
  --lib <substring>` filters by name; use it. A named test is seconds, the suite is
  an hour.
- **`--release` only when the test needs it** (the `#[ignore]`d benches and world
  builders). A debug build of a small unit test is far quicker than a release
  rebuild, and switching profiles invalidates the build cache — so do not alternate
  `cargo check` and `cargo test --release` in one session for no reason.
- **A cross-cutting change still owes the union of its rows, not the suite.** If you
  genuinely cannot tell which rows a change touches, that is a signal the change is
  too broad, not a reason to run everything. CI (rule 16) is the backstop that runs
  the full set; it exists so a local session does not have to.

### 2.9 Economy gates run once per BATCH, not per change (decided 2026-09-25)

The maintainer was waiting over an hour for gates on minor changes. A **batch** is one
numbered row of `docs/living_world/00_INDEX.md` (or, outside that plan, one plan phase /
one user request). Inside a batch:

- **Each slice:** `cargo check --lib --tests` + that slice's own named tests
  (`cargo test --lib <name>`) + `npx tsc --noEmit` if the frontend changed. Seconds to
  minutes. Slices may be committed and pushed like this.
- **Once, at the end of the batch, before its final push:** `cargo test --lib
  tick::tests` (includes `simulate_decades_reports_dynamics`) and `cargo test --lib
  econ_ -- --nocapture`. This is where any dose is judged.
- The licence to skip `econ_` on intermediate slices holds **only while every
  behavioural constant the batch adds is still at its zero dose** (a true no-op). A
  slice that raises a dose is by definition the batch's last slice and owes the full
  end-of-batch gates.
- Say in each commit which gates ran, and in the batch's final commit that the
  end-of-batch gates ran and what they printed.
- `earth_` (§2.3) and the other routing-table rows are unchanged — they are fast
  enough to run per change.

---

## 3. Core Architecture

### 3.1 Data flow
```
UI action → bridge/ (invoke) → commands/*.rs → sim/*.rs or paint/*.rs
  → WorldBuffer (flat world arrays) → tile_store (SQLite) → tile_image.rs (render)
  → RGBA → TileManager.ts → PixiJS sprites
```

### 3.2 WorldBuffer pattern
Simulation operates on flat world-sized arrays (`WorldBuffer` in
`sim/world_buffer.rs`). **Load all tiles → run simulation → write back. Never
mutate tiles directly during sim.** Each sim phase loads only the columns it
touches (`ColumnSet` per-phase masks); `save()` merges unmodified columns from the
old blobs. Run-alls load ALL columns.

### 3.3 Tile system
- World grid divided into 128×128 cell tiles (`TILE_SIZE = 128` in `tile/coords.rs`).
- **Cylindrical topology:** X wraps (`wrap_x`), Y clamps at poles (`clamp_y`). All
  BFS, painting, and simulation respect this.
- Each tile carries 25+ columnar data fields. **Serialization order matters** — the
  tail is, in order: `salinity` u8 · `shark_risk` u8 · `goods: Vec<Vec<u8>>`
  (`GOODS_COUNT`=45 belts) · `shipworm_risk` · `storm_base` · `reef_risk` ·
  `disease_risk` · `wind_speed` f32 · `precip_summer_frac` · `seasonal_amp` ·
  `sst` f32 · `snow_frac` · `biome` u8 (**currently last**). Blobs are **v2 self-describing**
  (`[0xF2][2][goods_count u16]`); new fields are **appended last** so older
  `.worldforge` saves still load (trailing reads pad zeros).
- Tiles stored as zstd-compressed blobs in SQLite.
- Rendered **server-side** as RGBA → packed binary IPC (`get_tiles_packed`) →
  frontend canvas tiles (base64 `get_tiles` kept for compat). Frontend only
  displays textures; Rust renders pixels per layer.
- **LOD pyramid:** lod 1-4 supertiles (one 128×128 image covers 2^L×2^L base
  tiles), persisted in the `tiles` table, invalidated on base-tile writes.
- LRU cache of 2000 tiles on the frontend (keys `layer|lod|tx,ty`; chunked fetches).

### 3.4 World / Campaign split
The **WORLD** (tiles + `metadata`: geography, climate, rivers/lakes, goods spec,
lat config, `world_progress`) is frozen by **Finalize World** (`finalize_world`
sets `frozen=1` + records `finalized_fp`). Everything human lives in the
`campaign` table (settlements, economy, `campaign_progress`, `world_ref`) and
saves to a separate **`.campaign`** file (`save_campaign_as`/`open_campaign`,
fingerprint-checked). `save_world_as` strips campaign rows. Paint/template/sim
phases 1-6 + run-alls call `ensure_unfrozen`. Legacy single-file saves migrate
in-memory on open (`legacy=true` → the app offers to split). `import_world_layers`
copies layer groups (terrain/climate/hydrology/soil/hazards/goods) from another
world of the same grid size via `TileData::merge_columns`.

**Save mid-generation, reopen, resume from there.** A `.worldforge` file saved
after only SOME pipeline steps have run (e.g. Landmass + Elevation, nothing
past it) is a completely ordinary save — `save_world_as` is a raw backup of
whatever tile columns and metadata exist, with no full-pipeline assumption —
and reopening it correctly restores the wizard's step-completion state so the
next step is ready to run. Two real bugs in that path, both fixed:
- **`App.tsx`'s `NewWorldDialog`** (the modal shown on a fresh launch, before
  any world is loaded) used to offer ONLY "Create World" — no way to open an
  existing file, and the modal has no cancel, so a brand-new session could not
  reach the header's "Open" button at all. It now also carries an "Open
  Existing World..." button wired to the same `handleOpen` the header uses.
- **The step-7-10 completion inference never ran once steps 1-6 had anything
  to restore.** `world_progress` (steps 1-6) ships inside every `.worldforge`
  file, but `campaign_progress` (steps 7-10) is a `CAMPAIGN_RUN_KEY` and is
  deliberately stripped by `save_world_as` (rule 28) — so on a plain
  save-mid-generation → reopen round trip, `world_progress` restores something
  while `campaign_progress` never does. The two halves were gated behind ONE
  combined "did anything restore" check, so the steps-7-10 fallback (infer
  from whether settlements/economy data is actually present) silently never
  fired. Restoring/inferring the two halves independently is what makes
  reopening a world that already has settlements or an economy show those
  steps as done, rather than stranding the user re-clicking "Generate
  Settlements" on data that's already there.

> **The interface is a ONE-WAY SNAPSHOT.** `campaign_start_sim` reads an
> `EconomySnapshot` out of `metadata` and from that moment **the campaign never
> touches a tile again**. This is deliberate (it's why 500-year runs are fast), but it
> means climate can't affect history and history can't affect the land. Making it
> two-way at *province* granularity is item **B1** in `docs/FIX_PLAN.md`.

### 3.5 Planetary parameters
A world carries planetary knobs in `metadata`, read into `WorldBuffer`
(`obliquity_deg`, `rotation_rate`, `solar_lum`, `greenhouse`, `eccentricity`,
`dryness`; defaults + clamps in `world_commands.rs::set_planet_config`, edited via
`StepWorldCharacteristics.tsx` (left-side WorkflowPanel step 0 — settings-only,
always advanceable, shown **after Landmass**: every one of these is a decision
about a map you can already see, and elevation ignores them, so their first
reader is Ocean & Atmosphere (3)). **Every generation setting lives there now**, in collapsible groups
(Planet · Axis & Seasons · Water & Air · Latitude Frame); the right-side Toolbar
is display-only. The old duplicate planet block + latitude control that lived in
the Toolbar are gone — `LatitudeControl.tsx` was replaced by
`ui/workflow/PlanetControls.tsx` (`PlanetSlider` + `LatitudeFrame`). They drive real
physics: obliquity → the insolation integral, rotation → circulation-belt
latitudes AND EBM heat transport, luminosity/greenhouse → the energy budget,
dryness → a final global multiplier on annual precipitation (not a mechanism).

`rotation_rate`'s **sign is the rotation direction**: negative = retrograde.
Belt LATITUDE (`Circulation::hadley_edge`/`polar_front`) depends only on the
magnitude (Held–Hou scaling), so it's unaffected; `Circulation::rotation_sign`
carries the direction separately and flips just the Coriolis-DIRECTION terms
downstream — the zonal (east/west) components of `belt_wind` in `ocean.rs`, and
which coast gets the intensified/warm-tagged boundary current in
`gyre_vector`/the current-type classification (`basin_dir = basin_pos *
rotation_sign`). It must never touch the unrelated hemisphere-based (N/S) `sign`
used for meridional flow and seasonal logic elsewhere in the same files.

**At Earth values every one of them is a no-op by construction** — the EBM is solved
twice and only the anomaly `T_world(φ) − T_earth(φ)` is applied, `Circulation`
returns exactly 30°/60° with `rotation_sign = +1`, and `dryness`'s multiplier is
exactly 1. That's what keeps the Earth calibration bit-for-bit intact while the
knobs still move real physics. Preserve this property in any change here.

---

## 4. The World Pipeline

Run in order. Each phase depends on previous phases' data.

| Phase | Command | What it computes |
|-------|---------|-----------------|
| 1 | `sim_generate_plates` | Tectonic plates, boundaries, terrain (land/sea) |
| 2 | `sim_generate_terrain` | Elevation from plate boundaries + sea depth |
| 2alt | `sim_generate_terrain_from_template` | Elevation from land shape (no plates) |
| 2b | `sim_generate_shelves` | Continental shelf (configurable) |
| 3 | `sim_ocean_atmosphere` | The full ocean/atmosphere chain — see the exact order below |
| 4 | `sim_classify_climate` | Köppen classification (31 zone codes; **H highland is no longer emitted** — no Köppen counterpart, see FIX_PLAN A15) |
| 5 | `sim_rivers_hydrology` | Priority-flood (Barnes et al. + ε) → rivers → lakes → aquatic ecology |
| 6 | `sim_soil_fertility` | Soil types (12) → fertility → fisheries |
| 6b | `sim_classify_biomes` | **41 ecological biomes** (needs rivers+lakes) — see §8.12 |
| 7 | `sim_generate_settlements` | Habitability scoring → city placement, then step 7a: bounded junction sites (straits/isthmuses/passes/great river mouths) the base local-maxima pass structurally cannot reach — the ports/junctions work (shipped) |
| 7b | `sim_generate_provinces` | Cost-flood + feature-snap province partition (AFTER settlements, incl. step 7a's junction sites) |
| 8 | `sim_biological` | Shark + shipworm risk + trade-good belts + ORE DEPOSITS (§8.16; `gem_deposits` now means ORE DISTRICTS) |
| 9 | `compute_political` | (query-only) Re-rank settlements by trade power + influence discs |
| 10 | `compute_economy` | (query-only) **Market equilibrium**: stock-based prices, barter, currency goods, wealth, chokepoints |
| All | `sim_run_all` | Phases 1-8 from plates |
| All | `sim_run_all_from_terrain` | Phases 2alt-8 keeping existing landmass |

**EIGHT elevation MODELS, one selector.** `sim_commands::apply_elevation_model` is
the single place a mode string picks a generator — `plates` (the tectonic model,
`generate_elevation`, the ONLY one that reads `boundary_type`) · `shape` ·
`cordillera` (§8.13) · `ridged` · `rift` · `glaciated` · `plateau` · `volcanic`
(the last four, ITCZ_AND_LAND_TOOLS_PLAN.md Commit 2). Both run-alls used to
HARDCODE a generator and silently discard the user's pick and all four sliders, so
"Generate Full World" produced the same relief however `StepElevation`'s picker was
set; the models were reachable only from step 2's own button. Two rules: the
tectonic model is offered only where plate data exists (`landmassSource ===
"plates"`) and degrades to the shape model otherwise, and an UNRECOGNISED mode must
still build terrain — a bad string may never leave a world with no elevation.
Gated by `elevation_model_tests`, which asserts all 28 pairs among the eight
disagree on >25% of land (a picker that does not reach the generator makes them
identical) — extended from four models/6 pairs in Commit 2 without weakening the
bar; `glaciated` vs `shape` was the pair flagged most likely to fail (glaciated
*starts from* shape) and it does not.

**The four new models** (`step2_terrain/elevation.rs`), each sharing a tail —
coastal taper → thermal + isostatic erosion → the shared hypsometric
redistribution → grid-scale relief limit → micro relief (`finish_elevation_field`,
the same tail `generate_elevation_cordillera` uses) — and differing only in how
the pre-erosion field is built:
- **`rift`** (`generate_elevation_rift`) — parallel fault blocks: a tilted,
  asymmetric HORST (steep scarp on one side, a gentle back-slope) alternating with
  a flat-floored GRABEN, banded along a strike direction. `divergent_strike_angle`
  takes the PCA principal axis of the world's own divergent-boundary cells where
  plate data exists (real, not decorative); a template/painted world with no
  plate data falls back to a seeded regional strike, the same "no better data"
  convention `geology.rs`'s phase-2 climate proxy already uses.
- **`glaciated`** (`generate_elevation_glaciated`) — the shape model, then glacial
  modification gated by `glacial_ice_mask`, a latitude+altitude PROXY (phase 2 has
  no climate, documented as a proxy exactly like `geology.rs`'s own): U-valley
  broadening (extra thermal-erosion rounding blended in by ice presence), cirque
  hollows carved just below ice-zone summits, and over-deepened troughs walked
  steepest-descent from the strongest summits that BREACH the coast — the one
  non-cordillera/ridged/rift model allowed to turn a little land into sea, and the
  honest way to draw a fjord (§8.23: notching a coastline with noise draws a
  scratch, not a landform).
- **`plateau`** (`generate_elevation_plateau`) — the shape model, then quantised
  into a handful of SHARP levels (a step function IS the escarpment; never
  blurred, unlike the subtle `terrace` blend `landform.rs`'s `plateau` archetype
  already applies more broadly) plus scattered outlying buttes.
- **`volcanic`** (`generate_elevation_volcanic`) — a gentle low backdrop, shield
  cones max-blended onto every `is_volcanic` land cell (so overlapping cones merge
  into ranges rather than cancelling), summit calderas on the densest clusters,
  hotspot trails of shrinking cones from isolated seeds — confined to EXISTING
  land, since an elevation generator never creates new land (only phase 1 or the
  lasso tools do that, rule 6/§8.23's discipline). Local density is read from a
  spatial bucket grid, never an O(n²) pairwise scan (§8.9 rule 1's spirit — a
  world can carry thousands of volcanic cells). Needs `is_volcanic` actually
  loaded: `ColumnSet::PHASE_ELEVATION` now carries `VOLCANIC` too, which it did
  not before this model needed to read it.

**Chains that die into their surroundings.** Both `walk_spine` (the cordillera
spine tracer, unchanged) and `generate_ridges` (the hand-drawn ridge tool) now
taper along-strike to nothing at both ends with a noise-modulated falloff — before
this, a drawn ridge line held its FULL peak height right up to the cursor's last
position and stopped dead. `generate_ridges` tracks each rasterized spine cell's
`t_along` (0..1 position along the whole drawn polyline, accumulated across
segments, carried through the same BFS that already propagates `peak`/`half_w`)
and multiplies the peak by a `sin(t_along·π)^0.45` envelope — the identical
along-strike taper `generate_elevation_cordillera`'s own spines use — times a
small per-line noise modulation so the two ends don't taper symmetrically.

**Randomise-by-default.** Both `StepLandmass` and `StepElevation` now roll a
fresh seed on every Generate press (an explicit "Lock seed" checkbox pins it for
iterating sliders against one fixed world) — pressing "Generate from Plates"
twice used to give the identical landmass, which is the wrong default for
brainstorming.

**Phase 3 runs this exact sequence** (`sim_commands.rs`; `earth_validation.rs` mirrors
it — keep the two in sync):
```
wind_belts → salinity → currents → advect_salinity_and_recouple → sst
  → distance_to_ocean → shelf_freeze → reinforce_cold_shelf_currents
  → temperature → upwelling → cold_shelf_cooling
  → seasonal_amplitude → ice_albedo_feedback → low_level_jets → precipitation
```

Extras: `sim_generate_terrain_ridged`, **`sim_generate_terrain_cordillera`** (§8.13),
`sim_scale_elevation`, `sim_invert_terrain`,
`sim_generate_toponyms` (#26, gated on cultures+rivers), `sim_refresh_hydrology_biology`.

`preview_zonal_profile` / `preview_coarse_climate` are READ-ONLY previews of the
planetary settings (§8.14) — they build a throwaway buffer and never write a tile.

**Two generation paths:**
- **From plates:** "Generate Full World" — everything from scratch.
- **From template/paint:** "Complete from Landmass" — keeps user's land/sea,
  generates the rest via distance-from-coast elevation.

**Pipeline rules:** steps run in order (each checks prerequisites); undo is
tile-level (every stroke/phase journals prior tile state to `undo_journal`);
overlays are PixiJS Graphics, not baked into tiles; template detection uses
dominant color (4-bit quantization → bright=ocean or color-distance threshold).

---

## 5. The Campaign Simulation (`sim/campaign/tick/`)

A `CampaignSim` is seeded once at campaign start (from the static economy
snapshot: hubs, per-good production, goods spec, connectivity) then advanced one
**day** at a time. Pure & deterministic per `(seed, tick)` — no DB, no global RNG,
no tile access (a tick is **hub-level math only**; the route-days matrix is derived
on load, not serialized). Core day loop:

```
1 production  2 consumption  3 price  4 merchant dispatch  5 arrivals
6 events      7 estates & starvation  8 houses  9 journal
```

Layered systems (all hooked at the yearly/monthly points inside `advance`, all
serde-defaulted so old saves load). Grouped by theme:

- **Feuds:** a quarrel between two houses is a first-class `Feud` (`tick/war.rs`), not a
  name in a `rivals` list: a CAUSE (`FEUD_TRADE`/`SEAT`/`MARRIAGE`/`MARKET`/`SUCCESSION`),
  an `intensity` that heats with real overlap (shared goods × shared cities) and cools
  without it, four STAGES whose weapons differ (snub → undercut → market closure +
  influence stripped → ships taken / counting-houses shut), and four ENDINGS —
  `arbitrate_feuds` (a council both trade in imposes a settlement), marriage, ruin, or
  simple neglect. `houses[].rivals` is kept in sync, so every pre-existing reader is
  unchanged. Formation is the O(n²) pair scan and keeps the half-yearly cadence
  (`update_rivalries`); temperature and flares run monthly over the bounded feud list
  (`update_feuds`). **Feud prestige is capped** (`FEUD_PRESTIGE_CAP`) — prestige is
  otherwise unbounded and feeds political power → charters → monopolies → wealth, and an
  uncapped per-flare award drove the sustained-richest house from 298k to 1.9M.
- **Tiers (Phase 1.1):** every live PRIVATE house (never a guild — a civic office isn't
  a family competing for rank) carries a `tier` (1 great · 2 major · 3 lesser · 4
  marginal) and a `standing` score, recomputed monthly by `assign_house_tiers`
  (`tick/houses.rs`) from state that already existed: `standing = 0.30·rank_norm(wealth)
  + 0.25·rank_norm(volume) + 0.20·reach + 0.15·seats + 0.10·rank_norm(prestige)`, where
  `rank_norm` is a percentile among LIVE houses (so the tier means "where this family
  stands among its peers", not an absolute number that means nothing as the world
  grows). Tier 1 carries an ADDITIONAL absolute floor (`standing >= 0.55`) so a young,
  undifferentiated world has an empty Tier 1 — a tier that's always occupied carries no
  information. Both the percentile cutoffs and the Tier-1 floor carry their own
  hysteresis (`TIER_PCT_DEAD_BAND`/`TIER1_STANDING_EXIT`) so a house sitting on a
  boundary doesn't relabel every month; a tier RISE is chronicled as a milestone, a fall
  is not (same asymmetry as `monopoly`/`monopoly_lost`). Purely a query-side
  classification — nothing downstream reads `tier`, so the dynamics run is bit-identical.
- **City leader & city tiers (`CITY_PROVINCE_WAR_PLAN.md` §3.1/§3.2):** the office as a
  PERSON, not a new entity. `council_house`/`captor_house` already existed and already
  compete for the seat (bribery/intimidation/capture in `update_government`); the
  `CityLeader` read (`read_hubs.rs`) surfaces `kin[0]` of whichever office is stronger
  (captor outranks a merely-dominant council) — head name, `character_phrase`, and
  `head_vice` (both already built for the House Dossier, never exposed outside it before
  this). `TickHub` carries its own `tier`/`standing`, recomputed monthly by
  `assign_city_tiers` (`cities.rs`) — a direct mirror of `assign_house_tiers`, same
  percentile cutoffs, same Tier-1 absolute floor, same hysteresis. Four axes: population,
  trade wealth, treasury, territory administered (rural population under provinces this
  city HOLDS via `prov_holder` — a house-held province, §5.9/rule 24, counts toward the
  house instead), and the ruling house's own `standing` (read fresh each month, which is
  why city tiers must run AFTER house tiers). **Query-side only at this step** — nothing
  downstream reads `hub.tier`/`hub.standing` yet, so the dynamics run stays bit-identical,
  exactly as house tiers shipped; §3.3 (state formation) is where that guarantee ends.
- **States (`CITY_PROVINCE_WAR_PLAN.md` §3.3):** a state is not new sim state — it is a
  PURE DERIVED READ (`compute_states`, `campaign_commands/province.rs`) over what §3.2
  and Phase 5 already carry: every province whose writ a tier 1-2 city holds
  (`prov_holder`, excluding a house-held writ — `prov_holder_house >= 0`, rule 24, is
  the house's territory, not a city's state) grouped by that city's `province_raster`
  cells into one `StateRegion`. A tier 3-4 or untiered town still self-administers its
  own province exactly as before, it just never forms a state. Nothing is written back
  to the sim, so this is where §3.2's "bit-identical to the dynamics test" note said the
  guarantee would end (tier now decides what the MAP draws) while the tick itself stays
  untouched — no new `econ_`/dynamics exposure from this step. Name is deterministically
  varied (city alone / "X Republic" / "Duchy of X" / paired with the home province's
  people-name) from a hash of the hub id, never geography-flavoured since the query has
  no notion of coastal/riverine. Colour is `distinct_color`'s own golden-angle hue
  rotation, phase-shifted (+53°) and desaturated so a state's tint can never be mistaken
  for a house's heraldic colour even where a hub id and a house id numerically collide —
  two different index spaces. Rendered client-side (`OverlayManager.drawStates`/
  `buildStateRender`) **exactly on the province raster** — `StateRegion.province_ids`
  names the provinces, the renderer tints those cells of the stored raster and traces
  the border along raster cell EDGES, so a state's border IS a province border rather
  than an approximating "cell cloud" (the first cut used `drawCultureRegions`' coarse-
  cell technique; that was replaced because a state's frontier is a legal line, not a
  density estimate). Toggle: Toolbar → 🏰 States (`overlayVisibility.states`,
  refreshed on year boundaries, the same cadence `campaignCorridors` uses).
  **Next:** `docs/REALM_AND_GOVERNMENT_PLAN.md` replaces this derived read with a real
  persisted `Realm` — approved, not yet built.
- **War score, terms and casus belli (`CITY_PROVINCE_WAR_PLAN.md` §3.4a-c,
  `tick/war.rs`):** DLC 3.5's declare/wage/resolve skeleton gains a real
  bidirectional `War.score` (−100..100) and quarterly rounds (tick-driven
  catch-up, same trick the crisis engine uses). A round's outcome is biased by
  relative war-chest+treasury strength; termination checks — in order — a
  decisive score, the three exhaustion paths (force broken · treasury & credit
  spent · war weariness), backers-withdraw (house wars only), then
  `WAR_ROUND_CAP` (3 years) as the last-resort guarantee, exactly rule 22's
  discipline applied to war (`every_war_terminates_within_the_round_cap` is the
  crisis engine's own termination test, mirrored). §3.4b prices the victor's
  terms in that final score (§1.4's table: reparations 10 · trade rights 25 ·
  tribute 40 · a province 55 · annexation 90); a win short of its declared
  goal's price downgrades to the richest the score affords, never upgrades. A
  new `WAR_GOAL_PROVINCE` reassigns one ordinary (non-house-held, rule 24)
  province's `prov_holder` to the victor. §3.4c: a WARMONGER RULER
  (`head_character_factor` axis 0) biases the declare chance; a HOUSE-DRIVEN WAR
  lets the winner of a vendetta-stage feud flare, if it holds its own city's
  council or captor seat, drag that city into a full war on the loser's instead
  of the ordinary property damage, auto-committed as `backer_house`.
  **The tuning story is the deliverable as much as the mechanism**: shipped
  naive it measured 65 wars/century against §3.4f's 6.0/century pre-3.4a-c
  baseline; four successive declaration-side preconditions (`HOUSE_WAR_CHANCE`
  cut 8×, a treasury floor, a 5-year post-war cooldown, a one-year floor before
  exhaustion can end a war) landed at 50/century — proof the volume was never
  about how often a war STARTED. Halving the per-round score-swing magnitude
  (the only change that touched how fast one FINISHED) brought it to
  45/century AND fixed a genuine `econ_inheritance_rules_fragment_differently`
  regression the faster wars had caused (RNG divergence between two
  60-year sub-simulations was swamping that gate's own wealth-comparison
  signal) — and, unplanned, moved top-10% wealth share from 0.498 (out of its
  0.60–0.90 band) to 0.671 (back in). Still above the pre-3.4a-c baseline by
  design (a real casus belli SHOULD raise it) and left as an explicit open
  pointer for a future session, not chased further — see `docs/SCOREBOARD.md`'s
  dated entry for the full chain.
- **War ledger, damage, blockade, boom (`CITY_PROVINCE_WAR_PLAN.md` §3.4e,
  `tick/war.rs`):** all four reuse existing machinery rather than inventing new
  fields. `war_damage_pass` writes straight into the EXISTING `TickHub.damage`
  field a belligerent's own estate/manufactory can take yearly — the same field
  a natural disaster uses, so `estate_condition_pass`'s existing funded-repair
  pass handles recovery with no new code. The blockade is now REAL and
  persistent, not cosmetic: the old `trade_wealth *= 0.8` line was silently
  overwritten every day by `update_houses`'s fresh recompute from
  `export_earn`/`import_spend` before a player could ever see it, so
  `export_earn` — the term that actually drives `trade_wealth` — now shrinks
  for a belligerent each year at war and decays back naturally. The neutral WAR
  BOOM nudges `export_earn` for any hub sharing a belligerent's trade component
  while itself at peace. `LedgerAcc` gains `war_levy` (split out of the general
  `civic_tax`, which used to silently combine ordinary wealth tax and war
  levies) and `war_damage`, both now wired into `HouseLedger.expense_total`
  (previously `civic_tax` wasn't even included there — a real pre-existing gap)
  and shown as their own ⚔ lines in the Accountant tab. Voluntary war financing
  (lend to the chest, goods at a war premium) and a feud cause from opposing
  war-backing are real future work, not silently folded in — 3.4e's own step
  text only asks for ledger/damage/blockade/boom.
- **Sack and purge (`CITY_PROVINCE_WAR_PLAN.md` §3.4d, `tick/war.rs`) — the last
  step of the war workstream, and its own highest-risk item, built last on
  purpose. `apply_war_defeat_consequences` fires from `resolve_war` only on a
  decisive-enough defeat (`score_abs >= WAR_PRICE_TRIBUTE`). ENEMY SACK: every
  live non-guild house resident at the losing city risks losing its own
  estates there (ownership passes to the city, `owner_house = -1`, the same
  convention the resale market uses), offices/bailos/influence there, and any
  warehouse stock there — a per-house roll, not a guarantee. INTERNAL PURGE:
  the city turns on whichever house actually financed the losing war (the
  house-driven war's own `backer_house`, else the losing city's own ruling
  house) — guaranteed once triggered, stripped the same way plus a wealth
  confiscation into the city's treasury and a prestige/power cost. Both share
  one helper, `strip_holdings_at`, and either may cascade to full dissolution
  through the EXISTING `dissolve_house` — no new cascade logic. `house_is_
  ruined` is new: distinct from ordinary insolvency (wealth alone), it checks
  wealth AND estates AND offices, since a war can strip a house's assets while
  it stays technically solvent a while longer. This completes `CITY_PROVINCE_
  WAR_PLAN.md`'s entire §7 order — see `docs/SCOREBOARD.md`'s dated entry for
  what remains explicitly out of scope by the plan's own §6.
- **Positive events (Phase 1.4):** the mechanism otherwise only produces decline (vices,
  feuds, ruin) — these give the chronicle something else to say, each a MARKER on `House`
  rather than new machinery. `assign_house_tiers` also tracks **the finest hour** (all-time
  peak wealth + the tick it was reached — never chronicled, since a peak most months would
  spam the record; shown as a fact instead) and **a golden age** (`golden_age_months`:
  Tier 1 held with wealth still rising, chronicled once it reaches a decade,
  `GOLDEN_AGE_MONTHS`, and resets the moment either condition breaks).
  `close_head_record` (Phase 0.4) checks **a dynasty of merchants** — three consecutive
  closed heads in `line` who each left the house richer than they found it — chronicled
  once per streak (`dynasty_chronicled`). All three are milestones (`is_house_milestone`),
  so the events cap can't prune them. Two of the design's five positive events are
  still QUEUED, with a named prerequisite each: a great partnership needs
  alliance-linked tier rises, a legendary head needs goals to be wired to decisions
  (Phase 3.1 ships goals as read-only tracking). Both are owed, not waived.
- **The `Kin` roster (Phase 2.1/2.2/2.3/2.6):** each non-guild house carries `kin:
  Vec<Kin>`, (re)generated at every founding/succession by `ensure_kin_roster`.
  `kin[0]` always mirrors the current head (role 0); 2–4 siblings follow, up to two
  `posted` to the house's CURRENT holdings (role 2, "factor") — a SNAPSHOT taken at
  generation time, not continuously synced to holdings gained since. Each kin carries
  four culture-derived character axes (−2..+2: caution↔boldness · honour↔greed ·
  private↔civic · rooted↔expansive), read into a phrase by `character_phrase`
  (`sim::tick`) that names only the notable axes. `kin_power_shares` (Phase 2.6) turns
  role × skill × loyalty into a 0..100 share per kin that always sums to exactly 100
  (`power_shares_always_sum_to_100`). **The widow as a capable merchant**: a purely
  `Agnatic` line otherwise never produces a female head (`heir_is_female` always
  returns false for it), so `succeed_house` rolls an independent
  `WIDOW_REGENCY_CHANCE`=8% chance of a widow regent instead — the roster doesn't yet
  track marriages, so this can't be conditioned on "is there actually a widow".
  `HousesPanel`'s Summary tab tags a family-run holding with its posted kin's name
  (silent = hired, the same "quiet unless it matters" rule as everywhere else here);
  the dossier's 👪 Kin tab lists the full roster.
- **Character wired to decisions (Phase 2.4):** `head_character_factor(hi, axis)`
  reads `kin[0]`'s character and returns a multiplier within ±`CHARACTER_KNOB_CAP`
  (0.15) of 1.0 — a TRUE 1.0 no-op with no roster or an all-zero axis, never an
  approximation, which is what keeps "no roster / all-zero character ⇒
  bit-identical" true without a special case at any call site. One touchpoint per
  axis (not all three `HOUSE_PEOPLE_AND_TIERS.md` §3 lists per axis): axis 0
  (boldness) scales the fleet-buy affordability threshold in `decide_fleets`; axis 1
  (greed) scales how fast a feud HEATS in `update_feuds` (averaged across both
  houses in the quarrel); axis 2 (civic-mindedness) scales the private
  consumption-into-civic-pool rate in `apply_wealth_sinks` — the same rate that
  fuels `fund_public_works`; axis 3 (expansiveness) scales the office-opening
  affordability threshold in `update_guilds_and_offices`.
- **Stewards (Phase 2.5):** a holding with no POSTED kin running it is "hired", and
  costs `STEWARD_WAGE` (fixed) + `STEWARD_SKIM_RATE` (proportional to wealth, capped
  to `STEWARD_SKIM_HOLDINGS_CAP` holdings' worth) every month in `apply_wealth_sinks`,
  and may be `STEWARD_POACH_CHANCE` (1%/month) POACHED away in
  `update_guilds_and_offices` — reusing the office-close machinery with a distinct
  `"poached"` event kind. A poached office can be immediately restaffed by the same
  pass's OPEN logic if the trade tie is still strong — that's realistic resilience,
  not a missing event. **Both mechanics are gated on a NON-EMPTY roster** —
  `!self.houses[hi].kin.is_empty()`. This was a real bug the first time: reading an
  EMPTY roster as "every holding is hired" (rather than "nothing is known") would
  have made an old save's houses silently cheaper to run than a freshly-generated
  one, breaking the master plan's own backward-compatibility invariant. Caught by
  `an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached` (renamed from
  `a_house_with_no_kin_is_bit_identical`, whose old premise Phase 2.4/2.5
  deliberately supersede for a house that DOES have a roster).
- **Goals (Phase 3.1, structure only — see `docs/proposals/HOUSE_MASTER_PLAN.md`'s
  handoff block):** a non-guild house carries `goals: Vec<Goal>` (1 slot, 2 for
  Tier 1 — `GOAL_SLOTS_TIER1`) plus a capped `goal_history`. `choose_house_goal`
  (yearly) picks ONE of 7 kinds (`GOAL_CORNER_TRADE`/`_SEAT_COUNCIL`/`_RAISE_BAILO`/
  `_CHARTER_BANK`/`_REACH_PROVINCE`/`_OUTLAST_RIVAL`/`_RESTORE_HOUSE`) biased by
  archetype and the head's character axes, when a slot is free. `update_house_goal`
  (yearly) checks each active goal against state that ALREADY exists elsewhere in the
  sim — a monopoly share, `council_house`/`captor_house`, `bailos`, bank solvency,
  the rival's `defunct` flag, or (via a hook in `expedition_travel_pass`, the moment a
  BACKED expedition completes its round trip) `dest_province` — and closes it
  achieved (milestone) or failed-at-deadline (chatter) into `goal_history`.
  `GOAL_RESTORE_HOUSE`'s `progress` field holds the TARGET wealth (the peak at the
  moment the goal was SET, not the ever-rising all-time peak — a house could never
  catch a moving target). **Goals are currently READ-ONLY tracking** — nothing in
  `decide_fleets`/`update_feuds`/`update_guilds_and_offices`/etc. reads a house's
  active goal to weight its choices, so this is NOT yet §4's "biases the weights of
  decisions the AI already makes" closed loop. Wiring that in is real future work,
  flagged (not silently assumed done) because it would move wealth and needs its own
  `econ_` check as it's built, the same lesson Phase 2.4/2.5 already recorded.
  Exposed via `campaign_get_house_goals`; shown in the dossier's 🎯 Ambitions tab.
- **The crisis engine (Phase 3.2–3.6, `sim/campaign/tick/crisis.rs`) — real, but cut
  down hard from four source design docs.** `head_vice` (3.2) derives one of 5 named
  vices (Lavish/Reckless/Rapacious/Miserly/Parochial) purely from `kin[0]`'s
  character+skill — no third random layer; Lavish is the one vice with a wired
  economic cost (an extra `apply_wealth_sinks` drain). `update_house_crises` (3.3–3.6,
  monthly) opens a `HouseCrisis` when a house's discontent (falling funds · failed
  goals · vice · the least-loyal live kinsman's disloyalty) crosses
  `CRISIS_DISCONTENT_THRESHOLD`, runs it a FIXED `CRISIS_ROUND_CAP`=4 quarterly rounds
  (one every `CRISIS_ROUND_TICKS`=90 ticks) — the head picks an in-character action
  (concede/buy off/venture/stand firm), a deterministic roll resolves it, the
  undecided bloc is folded into the same delta (3.4) — then resolves PREVAILED
  (grace period, `crisis_immune_until`) / DEPOSED (`depose_and_succeed`, reusing
  `close_head_record`+`found_head_record`) / DISSOLVED (empty kin + insolvent only,
  reuses `dissolve_house`; the design's fourth outcome, Split, is deliberately NOT
  built — same call as Part 3's existing "defer Rupture behind Departure"). Faction
  names/tints (3.3) are drawn from the SAME heraldic tincture/charge palette
  `CoatOfArms.tsx` renders with (`houseColor`'s exact FNV hash, mirrored in Rust), so
  a crisis's loyalist colour is provably the house's own arms colour, not a
  coincidence; the plot gets the opposite-index tincture for a guaranteed contrast.
  Civic intervention (3.5) is sequestration only (no exile): a severe deposition has
  a small chance the seat's council skims a slice of the estate into its treasury.
  `crisis_history: Vec<CrisisRecord>` (3.6) is a capped permanent record, same
  discipline as `goal_history`. **Two things this build deliberately does NOT have**
  (documented at the top of `crisis.rs`, not hidden): a per-figure power-share ledger
  (`head_support`/`plot_support` are two abstract aggregate numbers, not a sum of
  named shares) and a continuously-drifting `regard` ladder (plot leadership reads
  each kin's existing static `Kin.loyalty` roll instead). **A deposed successor's SEX
  must still obey the culture's `LineRule`** — the first cut didn't check this and a
  70-year test caught a man taking a matrilineal house's seat; fixed by filtering
  every succession candidate through `heir_is_female` exactly as `succeed_house`
  already does. Exposed via `campaign_get_house_crisis`; shown in the dossier's
  ⚠ Crisis tab (observation only — every choice is the AI's, per the source design's
  own decision 2). **Crisis salience**: a crisis opening or resolving only reaches
  the world `journal` (news feed) for a Tier 1-2 house — Tier 3-4 (and a not-yet-
  tiered house) still gets the event written IN FULL to its own chronicle, just
  quiet on the world stage ("the player cannot watch fourteen houses"), the same
  quiet-when-healthy discipline the stability gauges use.
- **The foreign hand (Phase 4.4, `sim/campaign/tick/foreign_hand.rs`)** — built ONLY
  after `economy_validation.rs::econ_measure_foreign_hand_conjunction` (a 300-year
  diagnostic, `#[ignore]`d like `econ_diagnose_house_turnover`) measured its trigger
  firing 1229 times/century, far past the "handful a century" bar that would have
  left it as dead code. Monthly, a posted kin whose city shows Channel A (a rival
  house holds an office/bailo there) or Channel B (the house itself leases in a city
  a rival `captor_house` controls — the "strong", real-dependency channel) has their
  `loyalty` nudged down by a small, hard-bounded amount
  (`FOREIGN_HAND_DECAY_RATE`=0.01/month at leverage 1.0, ceiling 0.015/month even at
  max leverage — both channels, an active feud, a maximally powerful rival), scaled
  by the rival's `political_power` and doubled in weight for an active feud.
  **Leverage only deepens an existing grievance; it cannot manufacture one** — a
  fully-loyal kin needs years of sustained exposure before it matters, and even then
  it only feeds `house_tension`/crisis discontent, which still has to clear its own
  independent threshold. An occasional chronicle event (`FOREIGN_HAND_DISCLOSE_
  CHANCE`) names the rival — scoped down from the design's "always disclosed" (a
  literal always would need a new persistent per-kin field, another House-adjacent
  struct patch across every construction site). **The design's own required gate
  held**: diffed against the pre-4.4 commit, house dissolutions/century moved DOWN
  (41.67 → 40.00), not up — leverage colours outcomes, it does not drive them.
- **Consequences (Phase 4.1–4.3):** three independent additions, each gated on state
  the house already carries. `sim/campaign/tick/schism.rs::update_house_schisms`
  (monthly) reads a simplified `tension` proxy (mean kin loyalty · reach · feuds ·
  a passed-over heir — a documented stand-in for the design's own `cohesion` gauge,
  which only exists read-only in `campaign_house_stability`) and, above threshold
  and past a per-house cooldown, either QUARRELS (common, chatter — the disloyal
  kin's own loyalty craters further) or, if that kin is POSTED to a real holding,
  DEPARTS with it to found a new rival house (`departure_schism`, 25% of parent
  wealth, forced identity reusing `found_branch`'s pattern) — **Rupture (a full
  split by line of descent) is NOT built**, deferred behind Departure exactly as
  this file's own §2.4 discipline and the master plan's Part 3 already called for.
  `dissolve_house` (Phase 4.2) now writes off any outstanding BANK LOAN a
  dissolving house still owes (`Bank.losses`, already the balance sheet's own
  write-off tally) and names the bank on both ledgers — every dissolution path
  (insolvency, a crisis's DISSOLVED outcome, plague extinction) funnels through
  this one function, so it's a single point of coverage for all three. **Kin
  barred from office is NOT built** — it would need new per-`TickHub` state, a
  much wider blast radius than the House-field patches this whole series has used,
  for a detail the source design itself calls small.
  `disease.rs::plague_house_toll` (Phase 4.3, hooked into `strike_plague`) can kill
  SEVERAL of a struck house's non-head kin at once, or — rarely, and via its own
  INDEPENDENT roll, never by touching `head_lifespan`/succession — extinguish the
  house outright (`plague_extinction`, a new milestone). This is the one change in
  the whole series to move **top-10% wealth share** (out of band since Phase 0.4)
  TOWARD its historical band rather than merely holding steady — plague extinction
  removing weaker houses concentrates the survivors' share, exactly the
  historically-documented mechanism, not asserted but measured (see
  `docs/SCOREBOARD.md`).
- **Provinces as house territory (Phase 5, `docs/proposals/HOUSE_INHERITANCE_AND_
  TERRITORY.md` Part D — the LAST phase of the house series; there is no Phase 6).**
  A province's writ can belong to a HOUSE instead of a city — the Stato da Mar case.
  `prov_holder_house: Vec<i32>` (`-1` = the ordinary case); `cities.rs::
  province_land_pass`'s delivery step credits a holding house's `wealth` instead of
  the seat's `treasury` (the GRAIN still reaches the seat's stock either way — only
  the monetary dues redirect); a revolt on a house-held province costs that house
  prestige and wealth instead of the seat's civic mood; `assign_house_tiers`
  weights each held province 3× a bailo/charter/council seat in its existing `seats`
  term ("standing rises steeply"). **Inheritable for free** — house-indexed, not
  head-indexed, so ordinary succession, a crisis deposition, or a Partible division
  all leave a held province with the same house; only the house's OWN dissolution
  releases it (checked lazily in the yearly land pass). The GRANT trigger
  (`maybe_grant_provinces`, yearly, narrow) required one real fix caught by
  measurement, not review: it first required a bailo specifically at the province's
  OWN seat and fired zero times on the real economy world (a house rarely bailos its
  own home city); relaxed to council/captor-house-or-bailo — the same "seats" signal
  `assign_house_tiers` already sums — and it became real. **Contesting a HELD
  province (war, a rival house) is explicitly NOT built** — needs new territorial
  war-goal machinery, the single largest remaining gap in the whole house series.
  This is the change that finally moved **top-10% wealth share INTO its historical
  band** (0.497 → 0.651) — not just toward it like Phase 4.3's plague extinction —
  "the ascent event the design lacked," now measured (see `docs/SCOREBOARD.md`).
  Exposed read-only to the frontend via the EXISTING `ProvinceLand` query (no new
  command) — `holder_house: i32` alongside `holder_hub`, and `holder_name` reads as
  the house's name when one holds the writ.
- **Succession & inheritance (Phase 0.4):** each people carries a **line rule** (who may
  inherit) and a **division rule** (how the estate divides) resolved once from its language
  kit into `culture_rules` (`sim/shared/inheritance.rs`, §8.15). `succeed_house` reads them
  and they decide three things: the heir's SEX (and so which name-bank they are drawn
  from), their AGE AT ACCESSION — an heir is not born on the day they inherit, so tenure is
  what remains of a life, which is the entire difference between ultimogeniture and
  seniority — and whether the estate DIVIDES (only `Partible` does; a share too small to
  stand alone keeps the heirs together as one firm, the *fraterna*). Every house also keeps
  a **succession line**: `House.line: Vec<HouseHead>`, one permanent record per head (name ·
  sex · generation · age at accession and at death · wealth at each end · how they came in ·
  an epithet derived at death). **Nothing in the tick reads the line** — it is the record the
  chronicle is written from. A family TREE (siblings, cousins, power shares) is Phase 2.
  A separate axis — `House.origin_house: i32` (−1 = no known parent) + `origin_kind: u8`
  (`ORIGIN_NONE`/`_GUILD`/`_BRANCH`/`_DIVISION`/`_DEPARTURE`/`_INDEPENDENCE`) — records
  where the HOUSE ITSELF came from, set once at every construction site that creates a
  `House` (guild-seeded founding, `found_branch`, a `divide_estate` co-heir, a
  `departure_schism`, an independent `found_house_at`) and never mutated after. This is
  INTER-house lineage (which house split from which, and why), distinct from `House.line`'s
  INTRA-house succession (which head followed which). `campaign_get_house_lineage`
  (`read_houses.rs`) walks the `origin_house` chain to the founding and lists this house's
  own offshoots, surfaced in the dossier's 🌳 Lineage tab — the answer to "where did this
  house come from, and why did it split."
- **Living Trade (DLC 1):** production/consumption/price/dispatch/arrivals/events,
  estates & manufactories, abstract houses, succession, fleets & voyage risk
  (`SEA/CARAVAN/RIVER_LOSS`), offices, contracts. Emergence order: local merchants
  → **guilds** (yr 5) → **houses** (yr 10); no cadet branches.
- **City lifecycle (Atlas 2.0):** organic founding/growth/absorption of towns;
  population sentiment, unrest & revolts; social strata + mobility; trade bases
  (houses develop under-traded small cities); council right-of-first-buy.
- **Polis + Speculation (DLC 3):** `decide_polis_policy` (council/tariff/mint/
  treasury) + `compute_speculation` (per-polis `SpecCenter` risk + ranked
  `SpecDriver` why-chain).
- **Crisis price regulation (`polis.rs::decide_crisis_relief`/`apply_crisis_relief`):**
  the council's answer to a DEARTH, on the same decide/apply split as
  `decide_polis_policy` so a player holding the seat can supply the choice.
  Monthly, right after `council_provision_pass`. Two levers: the civic granary
  opens across EVERY food good held (`RELIEF_RELEASE_DEARTH`/`_FAMINE`), and in
  famine the export of food is barred (`TickHub.food_export_lock`, honoured in
  `dispatch` by the same precomputed-flag shape plague quarantine uses — the
  *tratta* prohibition). Triggers on the DEARTH (`lack_basic`, `food_balance`)
  rather than on deaths. **A narrower release already existed and is deliberately
  left in place**: `update_government`'s step 6 dumps half the store of the FIRST
  food good once `starving > 0.5` — a famine backstop firing when people already
  die, on one good. The two compose. The design's other two levers are NOT built:
  an import bounty, and a price ceiling — a ceiling's whole historical consequence
  is that it CAUSES shortage, and with demand still price-inelastic the shortage is
  already unconditional, so it would move a number on screen and nothing else.
  Gated by `crisis_relief_is_inert_until_a_city_is_actually_short`, and isolated
  from `econ_inheritance_rules_fragment_differently` by `suppress_relief` for the
  reason `suppress_realms` exists (see that field's doc).
- **Coin / Credit / Crashes (DLC 3.5):** `decide_coinage` (named polis coin, sticky
  `coin_trust`, seigniorage, `coin_discount` freight, `coin_value` index); `Bank`
  balance sheets (`update_banks` founding+branches, `bank_pass` lend/service/fail);
  `trigger_regional_crash` contagion via `fail_bank` & `maybe_pop_bubbles`. A
  failing bank now tries `resolve_bank_failure`'s two rescues (wound down,
  absorbed by a solvent rival) BEFORE the old collapse-into-`fail_bank` path —
  `INSTITUTIONS_BUILD_ORDER.md` Phase 1.1/1.2, §9. A bank may also hold the
  Monte now (`update_public_debt`'s `kind == 1` holder class) and `League.
  purse` funds a real convoy escort (Phase 1.3).
- **Institutions (`INSTITUTIONS_BUILD_ORDER.md`, Phases 1/2/3.1/4.1/4.2/4.3/
  4.4/5 shipped; 4.5 built but dosed at zero — full detail in §9's own entry):**
  bank rescue splits from contagion and a bank may hold public debt (above);
  the Murano feature — quality's ceiling comes from accumulated `TickHub.
  tradition` (practice, not population), a guildhall's `secrecy` resists theft
  (`maybe_steal_quality`) and resists `maybe_poach_master` (a defecting master
  carrying tradition to a rival city), a named signature is earned once
  tradition+quality both clear a floor, and a high-throughput import-dependent
  hub becomes a refining entrepôt (`LAW_ENTREPOT`); war spending issues public
  debt (`WAR_DEBT_TARGET_MULT`), closing the war→debt→bank→crash chain; war
  CONTRABAND (`CONTRABAND_GOODS`) bans a short list of war-material goods to
  the actual enemy only, checked live in `dispatch` against `war_with`; a
  ROUTED BLOCKADE (`BLOCKADE_STAGING_DOSE`, shipped at 0.0 — a true no-op)
  diverts a lane between two actual belligerents through the same neutral-stop
  `staging_hop` relay N1/N1c already use, never a refusal, per this phase's own
  governing rule; LEAGUE PRIVILEGES (`LEAGUE_FREIGHT_DISCOUNT`/
  `LEAGUE_TARIFF_MULT`, shipped LIVE) cheapen a member-to-member lane's freight
  and tariff — the first thing beyond the Phase 1.3 convoy escort that makes
  membership matter; the KONTOR (`Kontor`, `tick/league.rs`) lets a league
  establish a shared depot at a non-member host city once their trade tie
  clears `LEAGUE_FLOW_MIN`, extending the same privilege through that host, and
  the host may expel it (a real chronicled event, likelier while at war with a
  member or under high unrest); and a fifth `ProvWork` kind, the CADASTRE
  (`WORK_CADASTRE`), permanently raises a surveyed province's tithe collection,
  with an active tax farm now costing real unrest+cohesion every year it
  stands. **4.5's boycott target-selection (`choose_boycott_target`) is built
  and tested but `LEAGUE_BOYCOTT_MAX` remains 0** — the plan's own text calls
  dosing it "last, and expect trouble" (N2's related cargo-ban mechanism broke
  the hard wealth bound twice at low doses), and walking it above zero is left
  for a session with room for the full multi-seed inheritance + dense-world
  gate sweep at each step, per §9's own entry.
- **Wealth / War / Flow (DLC 3.5):** capped bank interest + progressive civic wealth
  tax → treasury; `CityFinance` per-hub ledger; `update_wars` (rival poleis, forced
  house levies, war-chest, blockade, reparations, war goals); contract penalty
  LIMITED LIABILITY (cap to owner wealth); `flow_accum→flow_year` (Dynamic Trade
  Flow overlay).
- **Monetary v2.0:** closed monetary loop (quantity-theory-lite inflation),
  recoinage/reform, idiosyncratic bank runs, bullion-limited minting, mint charters.
- **Good quality (DLC 4):** per-producer quality on estates/manufactories.
- **Government:** key figures / capture / laws (`update_government`), Bailo (HQ) tier.
- **Warehouses & Futures:** capacity tiers + capacity-scaled upkeep; two-sided
  futures contracts as a stability layer.
- **Diseases & population:** historical epidemics (plague), starvation death-spiral
  guards (food reserves / granaries), sentiment.
- **Colonies & migration:** colonisation (settlement colonies + food lifeline supply
  ships), route-bound migration corridors, expeditions. **Financed expeditions
  (`expedition_launch_pass`, `colonies.rs`) target a REGIONAL range, not just the
  single farthest reachable city** — `EXP_MIN_GAP_FRAC`/`EXP_MAX_GAP_FRAC` bound the
  destination distance (≈1,400–8,800 km on an Earth-scale world) and scoring peaks at
  1.5× the floor (a bounded "sweet spot", not the old unbounded linear reward for
  raw distance, which structurally could only ever pick the farthest city in range).
  **House trade outposts (`maybe_found_house_outpost`) let SEVERAL wealthy houses
  each plant their own regional post per call** (`OUTPOST_MAX_PER_CALL`=3, richest
  first, each searching only its OWN home+offices network within `COLONY_MAX_KM`) —
  the single-richest-house-only version silently stalled after founding one outpost
  the moment that one house's network stopped bordering any remaining colonizable
  site, even with sites and wealth both still available (see
  `econ_diagnose_outpost_founding`, `economy_validation.rs`). Ordinary estates and
  outposts also no longer compete for the same slots: `OUTPOST_RESERVED_ESTATES`=20
  of `MAX_TOTAL_ESTATES` are held back so outpost founding can't be starved by the
  much-more-frequent ordinary-estate path saturating the shared cap early.
  **A settlement colony's food lifeline is now growth-gating, not just cosmetic**
  (user report: a colony reading "food 0.0/365 · supplied 0y" for its whole life
  had still grown past 400k). `TickHub.supply_shortfall_days` counts consecutive
  DAYS the lifeline actually fell short of the deficit — distinct from the
  pre-existing `supply_years` (an unbroken-run display counter any single bad day
  already resets to 0, so it says almost nothing about how underfed a colony has
  chronically been) — and resets only on a day the deficit is genuinely fully
  covered. `update_food_and_starvation`'s population-capacity multiplier
  (`cap_mult`) is now scaled down by `colony_supply_health` as this counter grows,
  so a colony cannot ratchet its capacity up on unrelated trade/primacy/world-age
  headroom while its own people are structurally starving; `colony_pass` collapses
  the colony outright once it passes `COLONY_UNSUPPLIED_COLLAPSE_YEARS` (3),
  independent of the older, fragile `starving > 0.8` trigger (which a chronic-but-
  mild shortfall can dodge indefinitely, since `starving` decays back down 0.02/day
  the moment `food_balance` recovers even briefly). `MAX_SUPPLY_SHIPS` was also
  raised 12 → 400 (a sanity ceiling, not a practical one) so a wealthy metropolis
  can actually keep pace with a growing colony's real deficit — the true limit is
  now what backers can afford (`buy_colony_supply_ship`), not an arbitrary fleet
  count. Gated by `an_unsupplied_colony_cannot_grow_and_eventually_collapses`.
- **Satellite construction:** a metropolis builds a suburb over ~10 years (with decay).
- **Provinces (Phase 2b · watershed demography + LAND STATE):** the ONLY campaign state
  carried at world granularity, and the world↔campaign join (FIX_PLAN B1).
  `campaign_start_sim` seeds `prov_rural` / `prov_cap` / `prov_culture` / `prov_seat` /
  `prov_neighbors` and maps `hub_province` from the `province_raster`. Two yearly passes,
  in this order (`disease.rs`, inside `tick % 365`):
  1. `province_demography_pass()` — rural pools grow to carrying capacity → migrate into
     cities carrying their culture; big cities pay an urban-graveyard mortality.
  2. `province_land_pass(yr)` — the LAND: `prov_forest`/`prov_arable`/`prov_pasture`/
     `prov_irrigated` (woodland cleared under population pressure, regrown when
     abandoned), `prov_soil` (worn by cropping INTENSITY = people per unit arable,
     rested back on fallow, floored at `PROV_SOIL_FLOOR`), `prov_works` (multi-year
     clearance/drainage/irrigation/road, funded yearly, **stalling** when unpaid —
     v2.0: begun AUTONOMOUSLY by `maybe_fund_province_works`, see §5.3),
     the harvest, `prov_tenure` drifting toward whoever actually holds estates, then
     `prov_unrest` → revolt, and a yearly `prov_history` sample + `prov_events` entry.
  **THE FEEDBACK EDGE** closes at step 6 of the land pass: `prov_surplus` is added to the
  seat city's food `stock` and `prov_revenue` to its `treasury`. Rural fiscality did not
  exist before — city treasuries came from tariffs and seigniorage alone.
  `prov_neighbors` still carries the overland plague hop.
  Two calibration invariants to preserve here:
  - **The land multiplier is centred on 1.0** for ordinary land. The first cut averaged
    ~0.7, which put gross output *below* rural subsistence on decent land, so no province
    ever had a surplus and the feedback edge silently delivered nothing.
  - **Land use is a partition.** `forest`/`arable`/`pasture` are shares of the SAME
    province; any seeding path must keep their sum ≤ 1 (the fallback seeder once handed
    out 1.13 of a province).
  All `prov_*` fields are serde-defaulted and every routine early-returns on empty, so a
  campaign without provinces — including the dynamics test — is **bit-identical**. That
  gate is `province_land_pass_is_a_noop_without_provinces`.
  **Goods exploitation (CITY_PROVINCE_WAR_PLAN.md §2.5):** `prov_good_belt` (flat
  `prov_count × goods.len()`) is the FROZEN per-good belt score, snapshotted once from
  the world's own `Province.good_belt` (an unfiltered, untruncated mean — `Province.goods`
  above is a top-6 quality shortlist and cannot serve this). `potential`/`actual`/
  `exploitation`/`market_share` (`cities.rs::province_good_*`) are pure DERIVED reads —
  `potential = belt · prov_cap · live land-use share (forest/arable/pasture, by a small
  name table over the shipped goods, `good_land_kind`) · a world-calibrated yield scalar
  (`prov_good_yield_scale`, self-calibrated once at campaign start exactly like
  `need_scale`, so mean exploitation reads ≈1.0 on day one whatever the world's size) ·
  (1 − depletion)`; `actual` is a plain re-attribution of hub+estate production already
  computed, not new production. The ONE piece of state that persists is
  `prov_good_depletion` (flat, same shape), updated yearly in `update_province_goods_
  pressure` right after the land pass, reusing `prov_soil`'s own wear/heal SHAPE with an
  estate-KIND-aware rate (`dominant_estate_kind`): a MINE never accrues depletion at all
  (**v2.0** — an ore body's `grade`/`extent` are a worldgen-frozen geological fact §8.16
  sets once, not something digging thins out; it was `(1.3, 0.15)` "exhausts, almost never
  heals", which made every mining province decay toward worthlessness), a
  fishery recovers fast ("collapses and recovers"), a vineyard doesn't accrue depletion at
  all (doesn't lose tonnage — the "raises grade instead" half is not tracked), a plantation
  also nudges `prov_soil` down under pressure ("wears soil"). A manufactory is excluded
  structurally, not by a special case — `Manufactured` goods have no belt score to begin
  with. Exposed via `campaign_province_goods`; the Province Inspector's Land tab shows it
  in place of the frozen quality/rank list the moment a campaign is actually producing
  something. Because it only WRITES `prov_good_depletion` (never touches hub production,
  stock or price), it cannot move the `econ_` bands or the dynamics test by construction —
  verified, not just argued: both are bit-identical/unchanged with this pass wired in.

- **The flavour layer, and what it is NOT** (`houses.rs`, all doc-labelled "Phase 4/5
  (flavour)" in their own source): `CraftGuild` (`seed_craft_guilds`/`run_craft_guilds`
  — hub · good · strength · hall, capped at `GUILD_MAX`=12 **for the whole world, and
  seeded ONCE at tick 0** (`guilds_seeded`) from year-zero production, with no founding
  or dissolution pass ever after — so 23 manufactured goods share 12 guilds, each craft
  has at most one guild anywhere, and the roster is frozen for 500 years
  (`docs/BANKS_MONEY_AND_CRAFT_PLAN.md` C-F1); lifts one good's local
  quality by `GUILD_QUALITY_STEP` to a `GUILD_QUALITY_CAP`, a `GUILD_STRIKE_CHANCE`
  strike halving that good's manufacture for 20-60 days, and one guildhall granting
  +0.05 civic stability). `Fair` (`seed_trade_fairs`/`run_trade_fairs` — one per large
  component, a spring or autumn sentiment + overlay-flow burst). `HolySite`
  (`seed_holy_sites`/`run_pilgrimages` — a pilgrimage season, sentiment, and a
  transient price bump on one patron ritual good). `run_piracy` (a yearly world roll
  deleting one `fleet_sea` from a random house — **there is no pirate**, no patron and
  nobody to pay off). `run_diaspora` (a yearly roll adding +0.05 influence at one
  distant office). None of these can restrict entry, set a price, enforce a mark or
  exclude anyone — see `docs/ACTORS_AND_CARRIAGE_PLAN.md` §2.
- **Public debt is real** (`money.rs::update_public_debt`): a council with a seat
  issues bonds against throughput, services a `DEBT_COUPON` out of treasury, and pays
  holders pro-rata — a working *Monte*, not flavour. **Correction (measured):** this
  line used to read "holders (houses and banks)". `debt_holders` is
  `Vec<(kind, idx, amt)>` and the kind byte allows a bank, but the single issuance
  site pushes only `(0, house, …)` and every payout loop opens `if kind != 0
  { continue; }` — **no bank has ever held a bond**. See
  `docs/BANKS_MONEY_AND_CRAFT_PLAN.md` B-F4 (slice 2 fixes it).
- **Two different things are called "guild"** — `House{is_guild}` (a CIVIC MERCHANT
  body: the same struct with a flag, a civic subsidy, bankruptcy immunity, and
  strictly FEWER organs than a private house — no tier, kin, goals, crisis or
  succession) and `CraftGuild` (the producers' body above). A guild is founded with
  `spec: vec![]` and **nothing ever fills it**, while `house_for`'s guild arm carries
  no `spec.contains(&good)` check and sits ABOVE the unspecialised private-house arm —
  so a guild specialises in nothing and is preferred for everything at its home city.
  That is a bug, not a balance choice; the fix and the proposed rename to **Company**
  are `ACTORS_AND_CARRIAGE_PLAN.md` §3.3.
- **`SUPPLY_LOCAL` is now written (N8, `docs/ACTORS_AND_CARRIAGE_PLAN.md` §3.8,
  shipped).** Every arrival used to book `SUPPLY_FOREIGN` regardless of who carried
  it — one of the five `supply_accum` seller classes the City Market view shows was
  structurally always zero. `InTransit.local` (serde-defaulted false, so an
  old save's in-flight cargo keeps booking `SUPPLY_FOREIGN` exactly as before)
  carries whether an ownerless leg cleared `LOCAL_HAUL_DAYS` at dispatch, and the
  arrival pass now books `SUPPLY_HOUSE` / `SUPPLY_LOCAL` / `SUPPLY_FOREIGN` by the
  real carrier. No sim gate needed — nothing in the tick reads `supply_accum`, only
  the query layer. Gate: `n8_arrivals_attribute_supply_local_by_real_carrier`.
- **Consumption rebuild (`docs/CONSUMPTION_REBUILD_PLAN.md`, S1-S8, shipped).**
  `base_need` (`production.rs`) now treats `TIER_WEIGHT[tier] × desire` as a
  **budget SHARE**, dividing by `base_value.max(BUDGET_VALUE_FLOOR)` to get a
  quantity — constant-share (Cobb-Douglas) demand instead of the old backwards
  reading where a good's spend share ROSE with its price. Moved food from 12.4%
  to ~60% of consumption spend, luxury from 69.7% to ~9%
  (`econ_expenditure_shares_resemble_a_household`). `estate_kind_for_good`
  (S4) now reads the real world-side `Distribution` (mirrored onto
  `TickGood.distribution` via `DIST_*` consts) instead of guessing from the
  good's name — a `Deposits` good the substring table has never heard of goes
  to quarry, never silently to Plantation. A real **buyer ledger**
  (`TickHub.demand_accum`, S6) mirrors `supply_accum`'s shape —
  `[manufactory, council, household]` — booked at every manufacture input-take
  and council provisioning buy; ordinary household consumption
  (`eat = need.min(stock)`) is deliberately never booked there, so its share
  reads honestly as 0.0 rather than an inferred number. Three slices follow
  the N1/N6 "dosed from zero" pattern — a real, wired mechanism gated
  bit-identical at a shipped-zero constant, with the actual dose walk left as
  separate future work re-verified against `econ_` per step: **S3**
  (`PROD_ELASTICITY = 0.0`) — `production_price_mult`/`_e` reads YESTERDAY's
  settled price (production runs before price is recomputed in the day loop)
  and would scale raw extraction/manufacture output by it; **S5**
  (`ORE_CEILING_DOSE = 0.0`) — `ore_extent_ceiling_mult` would cap a
  `Deposits` good's raw extraction near a mine site's real `extent`
  (`mine_geology_at`), rather than letting output scale with population alone;
  **S7** (`HOUSEHOLD_MONETIZATION_DOSE = 0.0`) — `household_priced_out` would
  let a poor `household_wealth` (new `TickHub` field, `household_income_pass`)
  price a household out of its own ration. **A real dose was tried (0.1 →
  0.02 → 0.005) and REVERTED**: `update_food_and_starvation` reads raw stock
  (`food_have = stock + production`), never `eat`, so a priced-out household's
  uneaten ration reads as the CITY being better fed, not worse — a real
  entitlement-failure gap (grain on hand the poor cannot afford, silencing
  `unrest_topples_councils`), not a tuning miss; 0.005 broke a DIFFERENT,
  larger set of tests than 0.02 (non-monotonic), the signature of chaotic
  single-trajectory sensitivity compounding a real bug rather than a pure
  magnitude effect. Fixing `update_food_and_starvation` to read the spending
  shortfall (`lack_basic`) instead of raw stock is the prerequisite before
  this dose can be resumed — see the constant's own doc comment for the full
  measured walk. Gates:
  `production_price_mult_is_a_noop_at_zero_and_correctly_signed`,
  `s5_ore_ceiling_at_zero_is_a_noop`,
  `s7_household_monetization_at_zero_is_a_noop`. **N6's own discipline
  applies unchanged**: none of this may touch the structural `needs_struct`
  that `lack_basic`/`lack_comfort`/`lack_luxury`/starvation read — only the
  actual `eat`/production amount. S1's demand-constant dose was walked
  against BOTH the new expenditure-share gate and the fragile
  `econ_inheritance_rules_fragment_differently` (§8.15) — the two disagreed at
  every intermediate value tried, landing at `TIER_WEIGHT = [1.0, 0.34, 0.10]`
  with `LUX_IMPORT_DESIRE` left at 0.7. `simulate_decades_reports_dynamics`'s
  insolvency floor was widened from -100.0 to -500.0 as S1's own bounded
  consequence (thinner luxury-import trade margins occasionally deepen a
  house's debt before its existing one-year bankruptcy grace period recovers
  or dissolves it — the mechanism itself is unchanged). S8's two panels
  (four-way flow balance, buyers beside sellers) are **not built** — only the
  backend data (`HubGoodDetail.demand_shares`, mirroring `supply_shares`) is
  wired through `read_hubs.rs`.

Tests live in `tick/tests.rs` — incl. `simulate_decades_reports_dynamics`
(the standing dynamics run) and `bench_campaign_tick` (ignored). See the DLC docs
in §9 for design detail.

### 5.1 Known structural limits (read before extending)
Three facts about the campaign that are easy to miss and shape any change here:

- **Four mutating verbs** (was one). `campaign_advance(ticks)` plus the three province
  control verbs — `campaign_set_province_tax`, `campaign_start_province_work`,
  `campaign_cancel_province_work` (§5.2). Everything else of the 60+ campaign commands
  is read-only, and every AI `decide_*` function is still a *latent player verb* — see
  item B2 in `docs/FIX_PLAN.md`. The province verbs are the pattern to copy: validate,
  call the same routine the AI would, persist.
  **v2.0 caveat:** the two WORK verbs still exist and still work (command + `bridge/`
  wrapper intact), but **nothing in the UI calls them any more** — land improvement is
  autonomous now (§5.3), and the Inspector shows works read-only. Only
  `campaign_set_province_tax` is still reachable by a player. That is a deliberate
  narrowing of agency, not an oversight; re-exposing the work verbs is a UI change
  alone, no backend work needed.
- **96% of shipments are carried by NOBODY.** `dispatch` decides a shipment from the
  arbitrage gap alone and then *attaches* a carrier: the seller's house, else the
  buyer's, else `owner = -1`. Measured over 60 years (`econ_measure_carrier_mix`):
  houses finance **4.3%**, the ownerless residual **95.7%**. That residual consumes no
  vessel slot, is not clamped by capital, and never sinks — and `surplus -= amount;
  stock_take(..)` sits OUTSIDE the carrier resolution, so the cargo moves either way.
  **A house's fleet, capital and voyage risk therefore govern who PROFITS, not what
  MOVES**, and any embargo built on `house_barred` touches 0.1% of trade. Do not
  reason about trade volume, fleet economics or exclusion without reading
  `docs/ACTORS_AND_CARRIAGE_PLAN.md` §1 first.
  **N1 (the plan's keystone) is now DOSED LIVE** (`ROUTES_ISOLATION_AND_
  CARRIAGE_REVIEW.md` Stage C4). `N1_LOCAL_HAUL_BIND_DAYS` = 90 days is a real
  bind clause in `dispatch`: an ownerless leg longer than the threshold no
  longer moves for free — it is handed to N1c's own `staging_hop` relay
  (below) rather than refused outright, a bare refusal being the exact shape
  that collapsed `econ_inheritance_rules_fragment_differently` the first time
  N1c tried it. It is a per-sim FIELD (`local_haul_bind_days`), the same
  `ship_leg_max_km`/`caravan_leg_max_km` opt-out pattern, so every abstract
  test fixture stays at `INFINITY` and only a real campaign or `tests::
  dense_world`'s dosed variant sees 90. A first dose trial at 30 days
  (just above `LOCAL_HAUL_DAYS`) staged everything with zero refusals and
  STILL collapsed `dense_world` trade volume to ~10% — proof that "routes
  instead of refuses" alone doesn't make a bind safe once it fires on most
  of a world's trade; 90 days measures healthy on both `tests::dense_world`
  and the multi-seed inheritance gate. `N1B_OWNERLESS_LOSS_RATE` (currently
  `0.0`) is the same shape for letting ownerless cargo sink — `let lost = if
  owner >= 0 {..}` above is no longer literal, but the roll never fires at
  zero dose, and dosing it is separate, unstarted work.
  **N1c (per-mode voyage RANGE) is the exception — it is DOSED LIVE**
  (`SHIP_LEG_MAX_KM` 3500 km / `CARAVAN_LEG_MAX_KM` 800 km,
  `TRADE_STAGING_AND_POSTS_PLAN.md` slices 3+4). **Cargo no longer teleports:**
  a leg past its mode's range is routed to the next settlement on the way
  (`staging_hop`, picking from the hub's own trade neighbours, required to be
  legal in its own mode and to strictly close the gap), and the arrivals pass
  re-checks the onward leg so the cargo walks stop by stop, bounded by
  `RELAY_MAX_HOPS`. The stop is looked for in the hub's `NEIGHBOR_K`
  shortlist first and, only if none there qualifies, in EVERY real
  settlement under the same three rules (a remote monotown's pull-ranked
  shortlist can hold no town on the way — the reported "gems sold 5,000 km
  away with no stop"). A third "split anywhere, even over-range" fallback was
  measured and REVERTED (0.19× staged volume on the dense-world relay gate).
  Every relay leg is now `log_trade`d — so a stop reads as the transit port
  it is — and `relay_cur`/`relay_last` keep each relayed cargo's true
  origin → final market, which `campaign_trade_flows` uses to show a route
  end to end with its full itinerary (`TradeRouteFlow.legs`, km, days). **It is a ROUTING rule, never a prohibition** — with no
  stop available the cargo sails direct — and that distinction is the whole
  mechanism: at the identical dose, REFUSING an over-range leg collapsed
  `econ_inheritance_rules_fragment_differently`'s world to 3 live houses,
  while routing left it healthy. Two earlier attempts refused, and both broke
  that gate; see the constants' doc comment for the full three-point dose walk.
  **Read this before touching either cap:** every fixture built through
  `sim()` opts OUT (both caps `INFINITY`, set in one place), because those
  worlds' coordinates are abstract — `world_w` there sizes the trade HORIZON
  (a fraction of it) while distance reads `KM_EQUATOR / world_w`, so the two
  uses pull opposite ways and leave adjacent hubs 1,202 km and 3,607 km apart,
  further than any real caravan stage. A rule in kilometres cannot be measured
  on that, so the dose is gated by `the_dosed_economy_stays_healthy_on_a_
  realistically_dense_world` on `tests::dense_world` (towns ~445 km apart
  across ~4,000 × 2,200 km at `world_w` 3600) instead. That world is also
  where the licence to ship came from: over 40 years the caps make the economy
  LESS concentrated, not more — peak house wealth 734,570 uncapped → 323,040
  capped — because staging spreads a long lane's margin across the ports along
  it. Real campaigns get the dose from `campaign_start_sim`; old saves and
  every test fixture are bit-identical.
  **N2 (cargo bans, §3.2) is the same shape and for a sharper reason: a live
  trial genuinely broke the hard-asserted wealth bound** (a sustained richest
  house of 1,005,714) even after halving the dose once — a real structural
  finding (an export-locked market's rent concentrates harder than the plan
  anticipated), not a tuning miss. `TickHub.export_ban_until` and
  `polis.rs::decide_trade_bans`/`apply_trade_bans` are real and enforced in
  `dispatch`; `N2_BAN_PRICE_RATIO` sits at `INFINITY` until that mechanism is
  understood well enough to dose. **N4 (carrier competition, §3.4) is shipped
  live** — `house_for`'s `.position()` is now a uniform `hash01` draw within
  each precedence tier. Its first cut weighted the draw by `political_power`
  and measurably inverted `econ_inheritance_rules_fragment_differently`
  (wealth grows political_power, so weighting by it just swapped one
  founding-order-shaped bias for a wealth-shaped one) — caught by running the
  gate, not by review, and shipped as an unweighted draw instead. **N3's
  narrow fix (§3.3) is also shipped**: a founded guild now charters itself
  with real goods and `house_for`'s guild arm requires a specialisation match,
  so it no longer shadows an ordinary house at its own city.
- **Growth is exogenous.** `tech_factor *= 1.015^(1/365)` per tick is the entire
  technology + growth model. There are no capital goods, no fuel inputs and no labour
  market, so nothing in the economy can influence its own growth rate (Part C of the
  fix plan). Don't mistake the finance layer for a growth engine — it redistributes.
- **`Pop` is no longer inert** (was true through FIX_PLAN B3's first pass; stale by
  the time a population audit re-verified it — see §5.4). `derive_pops` (yearly,
  `cities.rs`) feeds `update_unrest` (pop-weighted militancy) and now also
  `raise_manpower_levy`/`apply_war_casualties` (`war.rs`, §5.4) and
  `apply_manufacturing`'s craftsmen-class labor cap (`production.rs`, §5.4) — real,
  gated consumers, not display-only. `campaign_get_pops` (the panel it was meant for)
  still has zero frontend callers. The live SOCIETAL model remains the abstract
  `Society` shares (item B3); `Pop` layers a genuine profession axis on top of it now,
  not a competing one.

### 5.3 Province works v2.0 — autonomous land improvement
`maybe_fund_province_works` (`cities.rs`, yearly, just before `province_land_pass`)
decides whether a province BEGINS a work. It never touches how one progresses —
`advance_province_works` is unchanged and still funded-or-stalls, so the mechanic
is identical once started; only who starts it is new.

Before this, the four kinds were reachable **only** from the player verb, so a
campaign nobody was micromanaging never improved a single province's land on any
world, ever. That is the whole reason this exists.

Three rules:

- **Sovereignty is the difference, on BOTH axes — who may, and what.** Outside a
  realm a province may improve only once its own seat city has cleared some tier
  (`hub.tier > 0`, the ladder `assign_city_tiers` already computes monthly), that
  city's treasury pays, and **only the LOCAL kinds are available** — clearance and
  drainage, the manorial work a town does to its own hinterland. Under a realm
  (`prov_realm >= 0`, rule 27) the tier gate is waived, the CROWN pays
  (`ProvWork.funder_realm`, a third funder beside hub and house), and the STATE
  INFRASTRUCTURE kinds unlock: an irrigation system and a made road, the classic
  crown projects. That is what "a province is worked FULLY once it is under a
  realm" means concretely, and it is what makes sovereignty matter on the land
  rather than only in the tax ledger.
  **The infrastructure gate is load-bearing, not flavour.** Irrigation carries
  `PROV_IRRIGATION_GAIN` (+45% on the harvest at full watering), far the largest
  term here. The first cut let every city-funded province drive it to cap, which
  made autonomous works a world-wide yield multiplier and INVERTED
  `econ_inheritance_rules_fragment_differently`'s substantive claim (partible
  191,991 against primogeniture 163,230, when partible must come out *poorer*).
  Gating infrastructure on sovereignty fixed it on the merits — that gate now
  passes at 171,184 against 253,572, a **wider** margin than the pristine baseline's
  149,925/174,496. A `suppress_auto_works` isolation flag was written first, on the
  `suppress_realms` precedent, and then **deleted**: the correct model beat the
  workaround, and dead isolation machinery is worse than none. Gated by
  `state_infrastructure_needs_a_realm`.
- **Cost scales with real geography, never a flat price.** `work_cost(p, kind)`
  multiplies `WORK_COST[kind]` by the province's real **area** (`prov_area_km2`)
  and its real **relief** (`prov_relief_m`), both snapshotted at campaign start
  from the world's own `Province`. A road responds hardest to roughness
  (`WORK_ROUGHNESS_WEIGHT`) because it is cut *through* the relief; an irrigation
  channel least, because it follows the easiest contour it can find. **A save with
  neither figure keeps the old flat cost exactly** — both fields serde-default to
  empty and the multipliers collapse to 1.0, so this is an extension, never a
  repricing of an existing campaign (gated by
  `work_cost_scales_with_province_size_and_roughness`).
- **It picks ONE kind by need, not at random**: a road where arrears or unrest are
  biting, else irrigation where the arable is dry, else drainage where waste is
  high, else clearance — the first two only under a realm, per the rule above. One
  work per province at a time.

Gates: `province_works_begin_on_their_own_once_the_seat_is_advanced` (an advanced
seat DOES start one; an untiered town outside a realm does not, however rich),
`state_infrastructure_needs_a_realm`, `work_cost_scales_with_province_size_and_
roughness`, and the two pre-existing work tests, which still pass unchanged.

---

### 5.4 The population audit — Pop's first real consumers, and the missing PUSH

A session-driven audit of `Pop`/`Society`/settlement data (triggered by a settlement-
panel tidy-up surfacing how much of it was display-only) found `Pop` re-verified as
NOT quite inert (FIX_PLAN B3 had already wired pop-weighted militancy into
`update_unrest`) but still contributing nothing a `Society`-share formula couldn't —
and three adjacent absences that mattered more: war has no bodies, manufacturing
reads raw population instead of the craftsmen who actually work a bench, and
migration only ever pulls people INTO a city, never pushes them OUT. Five items
shipped from that audit, each additive and gated:

- **THE LEVY** (`war.rs`) — war finally spends a body, not just a purse.
  `TickHub.war_manpower` is drawn from the soldier-class `Pop`
  (`POP_SOLDIER`, `derive_pops`'s `so.underclass * 0.45`), raised toward
  `LEVY_MAX_FRAC_OF_SOLDIERS` of that pool a few rounds at a time
  (`raise_manpower_levy`) while a hub is at war, and spent by real casualties
  each fought round (`apply_war_casualties`) — a small share of which
  (`LEVY_DEATH_FRAC`) are actual deaths, fed back into `hub.population` the
  same multiplicative way every other demographic event in this file already
  is (`disease.rs`'s plague cull, starvation's growth drag), never a raw
  subtraction. `LEVY_STRENGTH_WEIGHT` folds it into the existing round-bias
  calculation (`strength_a`/`strength_b`) at a deliberately small weight, so
  manpower TILTS a round rather than deciding it outright. `demobilize_levies`
  (yearly) eases a hub's levy back to 0 once it is no longer at war. This is
  the one thing a headcount can say that the abstract `Society`/`Pop` shares
  alone cannot — it gets SPENT.
- **Craftsmen gate the manufactory** (`production.rs::apply_manufacturing`) —
  a hub's manufactured-good labor cap now scales off its CRAFTSMEN-class
  `Pop` (`POP_CRAFTSMAN`) relative to the world median, in place of raw
  population, so a farming/military town of the same total size can no
  longer out-manufacture a genuinely craft-heavy one. Reuses the exact
  median-ratio calibration `median_pop` already used, so no new constant was
  needed for the substitution itself. A hub with no craftsmen `Pop` yet
  falls back to the old population ratio exactly — additive, not a
  replacement. **Deliberately NOT yet** a single labor budget CONTESTED
  across every good made at one city (goods differ hugely in `labor`, and
  that cross-good unit conversion needs real dosing this pass didn't have
  room for) — each good still draws its own independent cap, just off a
  truer denominator now.
- **Two orphaned `Law` kinds turned on** (`houses.rs::enact_standing_laws`,
  yearly) — of `Law::kind`'s six variants, only 0 (favoured-house charter) and
  6 (foreign-ownership bar) were ever enacted; 1-5 were a purely aspirational
  set rendered in the UI as if real. A GRAIN LAW (`LAW_GRAIN`) is enacted on a
  city that has actually lived through famine (`starving > RELIEF_STARVE_
  TRIGGER`) and read by `decide_crisis_relief` (`polis.rs`) to call dearth
  SOONER — `GRAIN_LAW_TRIGGER_EASE` eases the two dearth triggers toward zero,
  never the famine trigger or the release/export-lock severity once called. A
  GUILD MONOPOLY (`LAW_GUILD_MONOPOLY`) is chartered once a craft guild raises
  its hall, lifting that (hub, good)'s quality ceiling from `GUILD_QUALITY_CAP`
  to `GUILD_MONOPOLY_QUALITY_CAP` — a real, bounded expression of the
  exclusive-entry right `ACTORS_AND_CARRIAGE_PLAN.md` §2 notes no flavour-layer
  guild can yet grant, kept small enough to need no separate `econ_` gate.
  Kinds 1-3 (tariff/free-trade/debasement) remain aspirational — that axis is
  already owned every year by `decide_polis_policy`, so a standing law naming
  it would compete rather than extend.
- **Emigration / urban exodus** (`cities.rs::urban_exodus_pass`, yearly,
  called alongside `province_demography_pass`) — unrest's missing consequence
  short of a revolt. The existing rural→urban pull already weighs a
  destination city's own prosperity/food; nothing let anyone LEAVE a city
  before this. A hub's PUSH pressure blends its own dearth (`starving`,
  `sent_prosperity` below `EXODUS_PROSPERITY_FLOOR`) with its PROVINCE's own
  rural pressure (`prov_rural/prov_cap`) — bound to both axes, per the
  brief: a city can now be pushed by a bad harvest region-wide, not only by
  its own market failing. Routed to the best-off OTHER hub sharing its trade
  `component` (an existing corridor, never a new path search — rule 34's
  discipline), only when that destination clears `EXODUS_MIN_OPPORTUNITY_
  GAIN` over home. A true no-op on a province-less campaign only in its
  province half; the city-dearth half still fires (`hub_province`/`prov_cap`
  reading empty just zeroes `prov_pressure`, not the whole push).
  **`EXODUS_DEST_ABSORB_CAP` (0.10, added after this shipped)**: every pushed
  hub in a component independently ranks the same candidates and picks its
  single best-off destination, so without a cap they all converge on the SAME
  winner in the SAME year. A destination may not absorb migrants worth more
  than this share of its own pass-start population per year; a source whose
  top pick is already full falls through to the next-best candidate. Fixed a
  real regression this introduced in two dense-world route-staging gates
  (`the_dosed_economy_stays_healthy_on_a_realistically_dense_world`,
  `the_relay_carries_long_lanes_in_stages_on_a_realistically_dense_world`).
  The obvious theory — an unbounded destination floods, overloads, and the
  resulting famine deaths (unlike exodus itself) destroy population — is
  TESTED AND FALSE (`diag_exodus_population_concentration`, `#[ignore]`d):
  total population on `dense_world` collapses along nearly the same
  trajectory regardless of whether exodus is disabled, uncapped, or capped
  anywhere from 0.08 to 0.15; that crash is a pre-existing property of the
  fixture. What the cap in a narrow 0.09–0.12 band actually restores is which
  HUBS end up populated (and so stay viable relay waypoints), not how much
  population survives in total.
- **Per-profession consumption baskets** (`cities.rs::society_demand_mult`) —
  composed WITH S1, deliberately, after the audit itself recommended against
  building this (risk of fighting S1's just-shipped budget-share demand
  rework) and the maintainer chose to build it anyway. S1 owns WHAT SHARE of
  a city's budget a good gets; `society_demand_mult` still only tilted that
  share by a coarse 2-bucket elite/mass split of the 4-strata `Society` and,
  by its own doc comment, left the entire COMFORT tier neutral. This blends
  in a genuine 9-profession reading (`PROFESSION_TIER_AFFINITY`, read
  straight off `hub.pops` — no new persisted state) at a small, bounded
  `PROFESSION_BASKET_DOSE` (0.15), normalized against a hand-derived
  "typical society" baseline (`PROFESSION_TIER_BASELINE`) so a hub at that
  mix reads as a no-op. Re-verified against `econ_expenditure_shares_
  resemble_a_household` at this dose: food/luxury shares held at 59.5%/8.9%,
  inside the pre-change measured range — the dose is small enough to nudge,
  not move, the aggregate. Affinity numbers are a structural ranking (who
  plausibly spends on what), not a measured series — treat like
  `COMFORT_IMPORT_FRAC`: walk further only with a fresh gate run per step.

All five are additive (no field is removed, no existing behaviour path is deleted)
and were verified together against `cargo test --lib tick::tests` (216 passed) and
`cargo test --lib econ_` (6 passed) before shipping. `econ_measure_war_frequency`
(the `#[ignore]`d 300-year war-frequency diagnostic) was not re-run this pass —
flagged as a natural follow-up before dosing the levy any further, not silently
assumed clean.

---

### 5.5 The campaign tick's own performance shape

A user report that "each monthly step feels laggy" on a real, large campaign
(~1,100-1,200 settlements) led to actually profiling it — the existing opt-in
`WF2_PROFILE=1` per-year breakdown in `advance()` (`mod.rs`) already separated
`trade`/`houses`/`events`/`rebuild`, and it named the culprit immediately:
**`dispatch()` (`production.rs`, called once per DAY) was 18,000-26,000 ms per
simulated YEAR** at that hub count — over 99% of the whole tick's cost, against
under 100 ms/yr combined for everything else. `bench_campaign_tick`'s existing
160-hub fixture never exercised this regime; `bench_campaign_tick_large`
(`tests.rs`, `#[ignore]`d, `large_bench_sim` at ~1,200 hubs / 30 goods, the scale
a real 3600×1800 world actually reaches once colonies and estates have grown)
does.

**The single largest cost was `house_for`, not the ranking scan.** Finer timers
inside `dispatch` (temporary, stripped before shipping — see below) found
`house_for(hub, good)` — which decides which house's account a shipment is
carried on — at 10.6-14.9 s of the 18-25 s trade budget. It is five FILTERED
PASSES over the WHOLE house list plus a `hash01` draw per candidate, called
twice per shipment (once for the seller's hub, once for the buyer's), and nothing
it reads (`defunct`/`is_guild`/`hub`/`spec`/`offices`) is written anywhere inside
`dispatch` itself — `dispatch` touches wealth, volume, events, ledgers, trade
ties and fleet counts, never a house's seat or specialisation. So the same
`(hub, good)` answer was being recomputed from scratch, by a whole-house-list
scan, potentially millions of times a year.

**The fix is memoisation over a per-round index, not parallelism.** Before
touching this, the natural instinct is to reach for rayon (§8.9's own playbook
for the worldgen phases) — parallelising the outer `for g in 0..ng` loop was
considered and set aside: `dispatch` interleaves reads and writes to hub stock,
house wealth and the trade journal within a single round in an order that
matters for who gets served first (§8.5's own "a seller ranks its buyers by
arbitrage gap, never by index" — the geometric 2^-k cascade down iteration
order), so parallelising across goods would need real synchronisation to stay
bit-identical, not just a `par_iter`. Memoising a PURE function that nothing in
the round mutates carries none of that risk:

- **`house_for_indexed(hub, good, seated, officed)`** (`houses.rs`) is the
  identical five-tier decision as `house_for`, re-expressed to read from two
  pre-built per-hub lists (`seat_at`/`office_at`: which live houses sit at, or
  hold an office at, each hub — built once per `dispatch` round in
  O(houses + offices) instead of five filtered whole-list scans per call) —
  never a second opinion. `house_for_memo: Vec<i32>` (flat, `n × ng`, `i32::MIN`
  = not yet resolved, `-1` = a real "nobody" answer, so the two stay distinct)
  caches the result the first time a round asks for a given `(hub, good)`.
  Gated by `the_indexed_carrier_pick_matches_the_reference_scan`, which walks
  EVERY `(hub, good)` pair across 60 ticks on a fixture built to exercise all
  five tiers at once (seated specialists with a real tie to break, an
  office-holding specialist, a seated generalist, a chartered guild, a bare
  office-holder with a DUPLICATED office entry, and a DEFUNCT house seated in
  the hottest good) and asserts `house_for(..) == house_for_indexed(..)` on
  every one — including the tie-breaks, since `pick_weighted_house`'s `max_by`
  keeps the LAST of several equal draws, so candidate ORDER is load-bearing and
  a naive index could silently drift from the reference on exactly a duplicate
  or a dead house.
- **Per-round lane cache.** Each hub's `NEIGHBOR_K`-bounded neighbour shortlist
  is resolved ONCE per round (not once per good) into a flat `Lane` array
  (`b`/`days`/`coin`/`lmult`/`pull`/`sea`/`blocked`/`war`) — `self.lane_days(a,b)`
  and `hub_pull(b)` were two random reads into `n²`-sized tables per (good,
  seller, neighbour) triple, a cache miss apiece at 1,200 hubs. The two freight
  multipliers (`coin_disc`, `league_mult`) are kept as SEPARATE fields rather
  than pre-folded into one product: float multiplication is not associative, so
  folding them would move the last ulp and break bit-identity with the
  unmemoised path.
- **Per-good live-price cache**, updated in place at the ONE point `dispatch`
  ever writes a hub's stock (`stock_take` after a sale) — `live_price` is a
  `powf` and was being evaluated once per neighbour scanned, though the
  destination's price for a given `(hub, good)` cannot change until that hub
  itself sells something.
- **A reused scratch buffer for `targets`** (the per-seller candidate list),
  cleared each iteration in place of a fresh heap `Vec` per (good × seller) —
  the same convention `rebuild_neighbors` (§ above) already uses one level up.

**Measured, same seed, same `bench_campaign_tick_large` fixture, `WF2_PROFILE=1`
breakdown of `dispatch` itself:** `house_for` collapsed from 10.6-14.9 s/yr to
350-390 ms/yr (roughly 30-40×); the whole `trade` phase fell from 18,000-26,000
ms/yr to 8,300-10,200 ms/yr (roughly 2.2-2.6×); the whole-tick bench went from
68.7 ms/tick to 37.5 ms/tick. The largest remaining cost is the shipment
EXECUTION loop itself (fleet-capacity checks, contract bookkeeping, ledger
updates, journal writes — 5.8-7.6 s/yr), which was measured and left alone
this pass: real further speedup, not attempted here.

**Bit-exactness is proven, not claimed.** `sim_fingerprint(&CampaignSim)`
(`tests.rs`, in the spirit of phase-3's `ocean_atmosphere_field_checksums`,
§8.9) folds every hub's stock/price/population/treasury/export/import, every
house's wealth/volume/prestige/defunct flag, and the sim's own counters over
their f32 BIT PATTERNS (never the values — a float sum would hide exactly the
last-ulp drift this exists to catch) in a FIXED index order, so it cannot
depend on iteration or scheduling. `bench_campaign_tick_large` prints it
alongside the timing; it must not change across a performance-only edit here,
and the fingerprint was confirmed identical (`8094efedaa8d67fb`) before and
after this optimisation, and again after stripping the temporary per-phase
timers used to LOCATE `house_for` as the cost (five `AtomicU64` counters plus
one `eprintln!` under `WF2_PROFILE`, deliberately not shipped — an always-on
atomic increment in the hottest loop in the tick is a permanent tax for a
debug-only feature, which is exactly what §8.9's own "off → ~zero cost"
discipline forbids). If `house_for`'s own cost needs relocating again later,
re-add the same shape of timer, measure, then strip it again before commit —
it is not meant to be a standing instrument, `bench_campaign_tick_large` is.

Gates run: `cargo check --lib --tests`, `cargo test --lib tick::tests`
(261/261, `simulate_decades_reports_dynamics` unchanged — sustained richest
486401 over 50y, identical to the pre-optimisation run), `cargo test --lib
econ_` (6/6) — per §2.8's own routing table for a `sim/campaign/tick/` change.

---

### 5.6 `docs/HOUSES_GUILDS_AND_MARKET_PLAN.md` — S4 and S7 shipped; the dose walks and the atlas/window redesign are queued

The plan's own build order (§7) puts S4 (craft signatures) and S7 (the orphan
raws' first recipes) first because both are FREE — S4 touches no sim state at
all, S7 is a pure goods-catalog addition independent of everything else in the
plan. Both shipped this session; the plan's three real DOSE WALKS (S1 ownerless
voyage loss, S3 the guild roster cap, S6 the wartime blockade) and the frontend
window/atlas redesign (S8-S12) did **not** — each needs its own multi-run gate
sequence against `econ_inheritance_rules_fragment_differently` (~6 min/run,
flipped inside its own noise band five times on record) that a single sitting
cannot responsibly absorb alongside everything else, per the plan's own §9 risk
register ("three doses in one session is the ceiling"). They remain queued
exactly as the plan's own §8 numbers them (S1/S2/S3/S5/S6 as dose steps, S8-S12
as the atlas queries + window split + Houses redesign) — not silently dropped.

- **S4 — craft signatures served, not just earned.** `CraftGuild.signature`
  (set once tradition + quality both clear their threshold, §8's Institutions
  entry) existed and was read by nothing. `HubGoodDetail.signature` and
  `GuildBrief.signature` now carry it (`read_hubs.rs`/`read_trade.rs`); the City
  Market's row label and open-book header read *"Ypres broadcloth"* in place of
  *"Woolen Cloth"* wherever a hub's guild has earned one, plain name otherwise —
  quiet when ordinary, §6's own principle. No sim change; the gate is `cargo
  check` + `tsc` alone.
- **S7 — eight new `Distribution::Manufactured` goods**, appended last in
  `default_custom_goods()` (rule 7 — a good's index is a fixed `TileData.goods`
  position, never reordered): `ceramics` (clay + timber), `glassware` (bay_salt +
  timber, the Murano case), `fixed_dye` (alum + dyes, the mordant trade
  `DEPOSITS_AND_MINING_PLAN.md` deferred), `sailcloth`/`cordage` (hemp [+
  pitch] — the yards plan's rigging), `armour` (iron + coal), `garum` (herring +
  bay_salt, the distinctive Roman export), `parchment` (hides + alum, an
  alternative books input). Each gives one of the five previously-orphaned raws
  (`clay`/`coal`/`alum`/`hemp`/`pitch` — placed, mined, shipped, consumed by
  nothing) its first downstream consumer. Categories set explicitly
  (`custom_category`) rather than left to fall to `"misc"`, so market-needs
  substitution treats them sensibly (ceramics/parchment as construction/craft,
  sailcloth/cordage as fiber, armour as metal, fixed_dye as dye, garum as
  preservative). Gate: `cargo test --lib goods_ -- --nocapture`, read per-good.
- **S2 — the *annona* carrier class, shipped as a SAFER shape than the plan's
  own text.** `ANNONA_MIN_POP` (60,000, the same order the old `size_bonus`
  saturation used) marks a destination as a metropolis; an ownerless shipment
  bound for one is exempt from N1b's voyage-loss roll (`production.rs::
  dispatch`) and tracked in a new `TickHub.tw_state`. The plan's own text says
  `tw_state` is "split out of `tw_local`" — checked, and that would NOT have
  been inert: `merchant_population_estimate` (mod.rs) already reads `tw_house +
  tw_local + tw_guild` as its total, so carving state carriage out of `tw_local`
  would have moved the displayed merchant-population split on any world with a
  metropolis, which is a real behaviour change, not the "cannot move a number
  by construction" decision 4 asks for. Shipped ADDITIVELY instead — `tw_state`
  accrues alongside the existing three, which keeps their sum exactly what it
  was. Gate: `annona_carriage_is_tracked_additively_and_only_for_great_cities`
  (`tick::tests`) plus the full `tick::tests` (268/268) and `econ_` (6/6,
  multi-seed inheritance gate included, 522.82s) — both bit-identical in
  direction/shape to their pre-S2 numbers (partible 45/32/27 alive by seed,
  matching the table already on record).
- **S1 is BLOCKED, not merely undosed — found before writing a line for it.**
  `N1B_OWNERLESS_LOSS_RATE`'s own doc comment (pre-dating this plan) already
  records a dose walk attempted at 0.01: `dense_world`'s uncapped trade volume
  did not fall, it rose **6.4×** (781,472 → 4,991,590 over 40 years), because
  `dispatch`'s target-room calculation reopens a buyer's deficit the moment a
  shipment is lost, so a sunk cargo invites MORE dispatch rather than less — a
  structural feedback, not a tunable collapse. S2's annona exemption protects
  the metropolitan lanes but does nothing about this: the feedback fires on
  every NON-metropolitan buyer at any nonzero rate regardless. Fixing it needs
  the room/deficit calculation to account for cargo already lost this cycle —
  real, separate work, not attempted here. Re-running the same failed dose
  walk this session would have cost a multi-seed gate run to relearn a fact
  already on record; the honest thing was to read the constant's own comment
  first (§2.4's own discipline: a negative result already written down is not
  re-litigated without new information).
- **S3 — the craft guild roster unfreeze.** `GUILD_MAX` (12, world-wide,
  `guilds_seeded` at tick 0, never founded or dissolved again) is now a sanity
  bound only (raised to 400); `GUILD_MAX_PER_CITY` (3) is the real per-hub cap.
  `maybe_found_craft_guild` (yearly, houses.rs) founds a guild at a hub that
  has practised a manufactured craft past `GUILD_FOUND_TRADITION_YEARS` (3
  tradition-years — far below `TRADITION_YEARS_FULL`'s 60, since organising a
  guild is a much lower bar than mastering a craft), gated by a per-candidate
  yearly roll (`GUILD_FOUND_CHANCE` = 0.20) so a world crossing the threshold
  on many cities at once doesn't found a dozen guilds in one year.
  `maybe_dissolve_craft_guild` (yearly) removes a guild whose hub has died, or
  whose good has gone unmade for `GUILD_DISSOLVE_IDLE_YEARS` (15) consecutive
  years (`CraftGuild.idle_years`, new field). Both chronicled
  (`"guild_founded"`/`"guild_dissolved"`), per `INSTITUTIONS_BUILD_ORDER.md`'s
  governing rule. Gates: `a_craft_guild_is_founded_and_dissolved_over_a_
  century`, `guild_count_per_city_is_bounded` (both new, `tick::tests`).
  **The dose walk itself (1 → 3) turned out UNTESTABLE by the standing
  gates**: `reference_world`/`reference_world_large`/`dense_world`/
  `simulate_decades_reports_dynamics`'s own fixture all build goods through
  the plain `good()` helper, whose `inputs` is always empty — every world
  `tick::tests`/`econ_` runs carries ZERO manufactured goods, so guild
  founding is structurally a no-op on all of them regardless of the cap.
  `tick::tests` (270/270) and `econ_` (6/6, multi-seed inheritance gate
  included) measured bit-identical at `GUILD_MAX_PER_CITY` = 1 and = 3; the
  shipped value of 3 is the plan's own target, not a value `econ_` actually
  validated. A future session that wants to genuinely dose this needs a
  fixture with real recipe goods run through `advance` far enough to show a
  wealth effect — queue item, see the constant's own doc comment.
- **S5 — transit demand, shipped inert at zero (no dose walk this session, per
  the plan's own build order).** `transit_need_mult` reads a hub's recent
  throughput of a good (`TickHub.supply_accum`, summed across all
  `SUPPLY_CLASSES`) against its CURRENT resident need (the `needs[h][g]`
  value the wiring site already holds — never recomputed via `base_need`,
  which would be both wasteful and circular), bounded by `TRANSIT_DEMAND_CAP`
  so the feedback loop (more demand → wider arbitrage gap → more transit →
  more demand) cannot run away. Wired at `mod.rs`'s demand-multiplier site
  beside `LOCAL_SATIETY`/`FOREIGN_PRESTIGE` — MARKET-FACING `needs[h][g]`
  only, never `needs_struct` (merchants passing through are not mouths; S7's
  household-monetization dose already hit and reverted exactly this bug once).
  Double-gated inert (`TRANSIT_DEMAND_DOSE <= 0.0` short-circuits both the
  wiring-site loop and the pure `transit_need_mult_e` function), so `tick::
  tests` (273/273, 3 new) and `econ_` (6/6, multi-seed inheritance gate
  included, 513.13s) confirming bit-identical output is closer to a
  formality than a real test — the genuine dose walk is queue item Q2,
  waiting on `LOCAL_SATIETY`/`FOREIGN_PRESTIGE` being walked first since all
  three now multiply the same expression.
- **S6 — dose walk attempted at 0.3, REVERTED (two real regressions).**
  `BLOCKADE_STAGING_DOSE` reuses the SAME `staging_hop` relay N1/N1c already
  use, triggered by `war_with` rather than a range cap. At 0.3:
  `simulate_decades_reports_dynamics` failed its bounded-wealth assertion (a
  house at −521.4, past the limited-liability floor), and
  `the_relay_carries_long_lanes_in_stages_on_a_realistically_dense_world`
  failed its own "the relay is provably inert with the range caps off"
  assertion (`diag_relay_staged` read 2, not 0) — because that test's "loose"
  fixture disables N1/N1c's caps but never disables war, and this dose gives
  the shared relay a third, independent trigger those fixtures never
  accounted for. Whether the wealth-bound failure is a genuine economic
  effect or downstream of the same test-assumption gap was not disentangled
  before reverting — §2.4's own rule: a spot failure on the aggregate gate is
  a revert, not a judgement call, regardless of which reading is right.
  Recorded at `BLOCKADE_STAGING_DOSE`'s own doc comment; walking this further
  needs `the_relay_carries_long_lanes_…`'s fixture updated to account for a
  live war before the two effects can even be told apart.
- **S8 — `campaign_house_atlas(idx)`.** A pure derived read
  (`read_houses.rs`): `partners` (from `House.trade_at`'s persisted weight,
  enriched with a live `in_transit` scan for real current in/out volume and
  which goods are moving), `goods` (from `House.good_volume`/`good_profit`,
  with bought-at/sold-at hubs from the same live scan), `holdings` (seat ·
  offices · bailos · owned estates · held provinces, each with real map
  coordinates — a province's from `prov_seat`, since a province has no hub of
  its own), and `seasons` (a `[f32; 12]` relative-ease curve, `trade_at`-
  weighted over `CampaignSim::season_mult` — real live seasonal data, but a
  favorability curve, not a recorded volume-by-month series, since no such
  history is tracked per house). `season_mult` widened from private to
  `pub(crate)` for this one caller.
- **S9 — `campaign_guild_atlas(guild_idx)`.** Same shape, a craft's question
  instead of a house's: `inputs`/`outputs` from a live `in_transit` scan at
  the guild's own hub (inputs = arrivals of any of the good's own recipe
  inputs; outputs = departures of the guild's own good), `reach` (every OTHER
  hub currently showing real `SUPPLY_FOREIGN` throughput of the good — a
  rough proxy, documented as such, since no arrival is tagged back to a
  specific guild), `signature` (served straight from `CraftGuild.signature`),
  and `tradition_by_year` (length 0 or 1 — only the CURRENT sample, since no
  year-by-year tradition history is persisted anywhere; a real series is
  future work, not silently faked).
  Both S8/S9 are read-only and touch no tile/sim state, so — per the plan's
  own note — neither can move a gate; verified by `cargo check --lib --tests`
  and `npx tsc --noEmit` alone, both clean.
- **S10 — the window split.** `HousesPanel.tsx` (§6/§7's own asymmetry
  complaint: 1,455 lines carrying a list, tier grouping, a feuds board, a
  compare launcher and an 11-subtab dossier, against `GuildsPanel.tsx`'s 125)
  is now BROWSE-ONLY (338 lines): the tier-grouped list, the Compare launcher,
  and a "🏛 Companies" FILTER CHIP in place of the old "guilds" tab
  (`House.is_guild` firms are firms, not a different kind of thing — keeping
  them in a tab beside `CraftGuild` under one word was the naming collision
  §1 of the plan names). `HouseDetail` and its ten subtabs moved VERBATIM
  (no rendering logic changed in the move itself) into `HouseDossier.tsx`,
  which already held `HouseStandingView`/`FeudsView` — now genuinely "one
  house, everything about it" rather than split across the file that browses
  ALL of them. A new `FeudsAlliancesPanel.tsx` wraps `FeudsView` with no
  house focus as its own window (`uiStore.showFeuds`), opened from a new
  "⚔ Feuds" button in `HousesPanel` and from the Society menu
  (`CampaignTopBar.tsx`) — a feud belongs to two houses, not one, and was
  never really a house's tab. `GuildsPanel.tsx` relabelled "🔨 Crafts &
  Guilds". Shared helpers (`TIER_META`/`tierOf`/`dull`/`goodIcon`/
  `familyRunAt`, used by both the browser and the dossier) split into
  `houseShared.ts` rather than duplicated or cross-imported, which would
  create a HousesPanel ↔ HouseDossier import cycle.
  **Verification caveat, stated plainly**: this environment cannot launch
  the Tauri GUI (no display), so this was verified by `npx tsc --noEmit`
  (clean) and a full `vite build` (clean, 181 modules) — type-correctness
  and bundling, not a human looking at the running window. The user chose
  to accept this risk explicitly rather than defer S10 to a session that
  can open a browser; visually exercising the four windows before trusting
  them is still owed.
- **S12a — the Houses bump chart (shipped in a follow-up session, same
  branch).** "Stop showing state, start showing change." A new
  `campaign_house_bump_chart` query (`read_houses.rs`) reconstructs each of
  the top ~12 houses' wealth RANK per year from its own `wealth_history`
  (`WEALTH_HISTORY_CAP` = 80 years — checked first, per the plan's own §9
  risk-register note, before building a 50-year-wide chart the data might
  not have supported) and returns only the houses that were EVER in the top
  field, never the whole roster. `HousesPanel` renders it as an SVG line
  chart above the list — a house's own `distinct_color` (the one stable
  app-wide identity colour), a thicker line for Tier 1, a dot at the last
  ranked year, a line that simply STOPS the year a house dies rather than
  crawling to zero (falls out of `wealth_history` no longer being sampled
  once `defunct` — no special-casing needed) — with four quiet gauges beside
  it. Only TWO of those four ship as real sparklines: `families`
  (`InequalitySnapshot.series[].active`) and `top-10% share`
  (`...top10_share`), both already-existing per-year series from
  `campaign_get_inequality` (#29's own reads, reused rather than
  duplicated). `founded`/`fallen` ship as plain totals — no per-year series
  for either exists anywhere in the sim, and rather than fabricate a history
  the data cannot support (the exact failure mode this plan's own §9 warns
  against), they are shown honestly as cumulative counts; giving them a
  real series is queue item Q18. A bottom "pulse" ticker reuses the
  existing world journal query (`campaign_get_journal(-1,-1)`,
  `NewsFeedPanel`'s own data source) filtered to house-ish event kinds — a
  new CALLER of an existing mechanism, not new state. Pure derived read
  (touches no tile/sim state), gated by `cargo check --lib --tests` +
  `npx tsc --noEmit` + a clean `vite build` (181 modules, unchanged) alone,
  per the plan's own §4 note that neither atlas query nor this chart can
  move a tile/sim gate. Same no-display verification caveat as S10 — not
  opened in a real browser this session (queue item Q17, widened to cover
  this).
- **S11 — the two atlases, TEXT/TABLE form (shipped in the same follow-up
  session as S12a).** The S8/S9 atlas queries were built and unused by any
  UI. `HouseDossier` gains a "🗺 Atlas" tab: partner cities ranked by
  volume with an in/out two-tone bar, the goods portfolio (bought-at/
  sold-at cities resolved from the atlas's own partner list), and a
  seasonal lane-ease bar chart (`season_slices` finally has a reader).
  `GuildsPanel` gains a per-row "🗺" toggle that expands an inline Craft
  Atlas strip: inputs/outputs by city, reach (hubs currently buying the
  good), the earned signature when one exists — needed one new field,
  `GuildBrief.idx` (index into `sim.guilds`, enumerated before the
  quality-sort so it stays stable), since the browse query never carried a
  key `campaign_guild_atlas` could use. Both S8/S9 queries were already
  gated as pure derived reads touching no tile/sim state (§4's own note),
  so this needed no `econ_`/`tick::tests` run — `cargo check --lib --tests`
  + `npx tsc --noEmit` + a clean `vite build` (181 modules, unchanged)
  alone. The plan's own richer on-map version was queue item Q19 at
  first — then shipped in a THIRD follow-up session (user instruction:
  "do all the steps, ignore risks"). See the Q19 entry below.
- **Q19 — the on-map lane half of S11 (shipped in a third follow-up
  session).** The focused house's existing seat→city web
  (`OverlayManager.renderHouseControlLayer`, built on `recomputeHouseNetwork`'s
  real corridor-snapped paths) now carries real per-lane data once the
  house's atlas has been fetched: line WIDTH ∝ real volume
  (`partner.volume_in+volume_out`, normalized against the house's busiest
  lane), line COLOUR = the lane's dominant good's own `GOOD_DEFS` hue
  (cross-referencing the partner's traded goods against the house's own
  goods-portfolio volumes for the best match), stroked one MEDIUM at a
  time via the existing `mediumRuns` helper (the same dashed-open-water/
  solid-road discipline already shipped for the merchant-route layer,
  rule 35/§8.5) instead of one uniform dash style end to end. A small
  `drawGoodIcon` medallion + a `drawLabel` name mark the dominant good at
  the lane's far end — needed a new `LabelKey`, `"tradeLane"` (§8.11's
  registry: human works, sans, upright, small), added to
  `LABEL_STYLE_DEFAULTS`/`LABEL_THEMES` and to `SettingsPanel`'s
  "Cultural & trade" group so it's themeable like every other place-name
  class. `MapCanvas` fetches `campaign_house_atlas` whenever the focused
  house changes and pushes it into a new `OverlayManager.setHouseAtlas(idx,
  atlas)`, which guards against a stale fetch outliving a house switch via
  its own `houseAtlasFor` check. Falls back to the original flat-red/
  uniform-width style when no atlas has arrived yet or a destination has
  no recorded trade (an office/bailo city). **Still not built**: distinct
  outbound/inbound arrow styling (the existing single chevron+arrowhead is
  unchanged), a per-good/per-partner toggle UI, rivals' lanes ghosted
  behind the focused house's own, and any Craft Atlas map presence at
  all. Backend untouched (`cargo check --lib --tests` clean, confirming no
  regression, since this is a pure frontend change); `npx tsc --noEmit` +
  `npx vite build` clean. This is the highest-risk piece of canvas code
  this plan has shipped blind — check it FIRST in queue item Q17's
  session.
- **S12b — the Dossier timeline plate (shipped in the same follow-up
  session as Q19).** A horizontal life-timeline (`TimelinePlate`) under
  the House Dossier's header, above the subtabs: every head as a segment
  (coloured by sex, labelled when wide enough, from `chron.line`'s own
  `since_year`/`until_year`/`wealth_start`/`wealth_end`), milestones as
  marks above (founded/succession/monopoly/branch/dissolved only —
  chatter excluded, the chronicle's own "quiet unless it matters" rule
  applied here too), feuds as brackets below (a new
  `campaignGetFeuds(h.idx)` fetch alongside the tab's existing ones), and
  a wealth curve as a filled area threading through, piecewise-linear
  between each head's own recorded wealth_start/wealth_end. Scrubbable by
  drag: sets a `scrubYear` that shows which head ruled then and their
  interpolated wealth. **Does NOT rewrite the tabs below to that year** —
  no per-tab historical state exists anywhere in the sim for Kin/Goals/
  Crisis/etc. to read from, so a genuine cross-tab year-travel view is
  queue item Q20, not attempted. Built entirely from data already fetched
  elsewhere in the file — no new backend query, no sim change. Gate: `npx
  tsc --noEmit` clean, `npx vite build` clean (181 modules, unchanged).
- **S12c — three of five per-tab graphs (shipped in the same follow-up
  session; two named as genuinely blocked, not faked).** The plan names
  five: Accountant waterfall · Kin as a real tree · Lineage as a
  branching diagram · Standing as a radar against the tier-1 median ·
  Feuds as a temperature line with stage transitions.
  **Accountant waterfall** (`WaterfallChart`, `LedgerView`) — a real
  cascade from 0 through the year's top trade goods (top 4 by |amount|,
  the rest bucketed) and every expense line already printed above it,
  ending at the real NET in gold. **Standing radar** — needed one new
  backend read, `campaign_tier1_gauge_medians` (`read_houses.rs`): calls
  `campaign_house_stability` VERBATIM for every live tier-1 house and
  takes the per-gauge median, so the house's own gauge and the world's
  median can never drift apart (the two numbers share one computation by
  construction). Quiet when no tier-1 house exists yet (`n === 0`) rather
  than plotting a pentagon against zeros, which would misread as "this
  house has no standing" when the truth is "nobody does yet".
  **Feud temperature line** (`FeudTemperatureLine`) — a STEPPED line (a
  feud's stage is a real jump, never a smooth climb) built from each
  feud's own recorded flare log (`f.log`'s real year+stage pairs) plus
  the feud's current stage as the line's live end-point; quiet below two
  points. **Kin as a real tree is NOT built** — `KinBrief` carries a flat
  `role` and no parent/child field anywhere in the sim, so a genuine
  family tree needs real generational edges that do not exist; inventing
  a plausible-looking hierarchy from role alone would be exactly the
  fabrication §2.4 forbids (queue item Q21, waits on the sim actually
  persisting kinship edges — new sim state, not a query). **Lineage as a
  drawn SVG diagram is NOT built** — the tab already renders as a real
  indented branching list with dashed connector lines (ancestors → this
  house → offshoots), judged to already convey the same structure closely
  enough that a from-scratch SVG tree's own layout/collision risk wasn't
  worth taking blind this pass (queue item Q22). Gate:
  `campaign_tier1_gauge_medians` is a `sim/campaign_commands/**` wiring
  change (a pure derived read reusing an already-gated function) →
  `cargo check --lib --tests` clean, no `econ_`/`tick::tests` run owed
  per the plan's own §4 note; `npx tsc --noEmit` + `npx vite build` clean
  (181 modules, unchanged).
- **What did NOT ship, and why, per rule 36** (a waiting item, not a refusal):
  S1 (blocked — see above, waits on the room/deficit fix), S6 (reverted —
  see above, waits on the relay-fixture fix before its own dose walk can be
  re-attempted), Q20 (full cross-tab year-travel on the timeline plate),
  Q21 (Kin as a real tree — needs new persisted sim state), Q22 (Lineage
  as a drawn SVG diagram — a judgement call, not a blocker).
  Each waits on exactly what the plan's own §7/§8 already say it waits on
  (S1/S6 additionally wait on their own newly-found fixture/mechanism
  fixes) — nothing here changes that sequencing, this entry only records
  which end of it landed.

### 5.7 `docs/SETTLEMENT_LIFE_PLAN.md` — L0-L8, L11(shadow), L12(partial) shipped, L13 mostly surfaced

The plan's own §4 build order calls L0-L3 "a coherent landing" and caps a session
at three doses. L0-L2 shipped in one session; L3 (the annals + Life tab) shipped
in a follow-up session, so the STOP MARKER was reached in full — the hidden
famine is measured, the welfare ratio exists AND is visible, and the player can
see how a city lives. Further sessions then shipped **L4** (vital rates + age
bands), **L5** (welfare into behaviour), **L6** (housing & crowding), **L7's
first named part** (the settlement year's seasonal mortality), **L8's first
named part** (fire's toll on housing and lives), and then, at the maintainer's
explicit request to prioritise the Life tab and its remaining slices, **L13's
housing/crowding chart**, **L11** (persistent pops, shadow phase) and **L12**
(three of its six named notable roles), each at its own dose of zero (or, for
L11's shadow, no dose at all — nothing reads it yet). L7's other three named
parts (lean-months hoarding, harvest labour, feast days), L8's other two named
parts (flood, water/sanitation), L9/L10 (church, watch), L12's other three
roles (bishop, physician, watch captain — each waiting on L9/L10) and
`PERSISTENT_POPS_DOSE` itself all remain queued (see below). Housing/crowding
(L6) and the townspeople (L12) are now DRAWN in the Life tab, not just
computed; church/watch genuinely have no mechanism behind them yet (L9/L10).

- **L0 — the instrument** (`economy_validation.rs::econ_measure_settlement_life`,
  `#[ignore]`d, run on `realm_reference_world` + `dense_world` per §1 F10 — never
  the province-less `reference_world`). Prints the F1 hidden-hunger count (how
  often `lack_basic > 0.2` while a hub's own grain stock still covers 30+ days of
  need), the `lack_basic` distribution, famine episode count/length, implied
  natural growth by city-size tercile, and riot/revolt/plague-strike counts
  (journal-capped, reported as a lower bound). **Baseline measured**: the F1 count
  reads **0 of ~4,000-4,300 hub-year checks on both fixtures** — not because
  hidden hunger is rare, but because `lack_basic` sits near its ceiling almost
  everywhere already (mean 0.886/0.743, median 0.99/0.98 on the two fixtures) —
  a pre-existing, near-permanent basic-goods shortfall this instrument surfaces
  for the first time, not something L1 caused. Plague RECOVERY TIME (as opposed
  to strike frequency) needs per-tick population history this instrument does not
  keep; left unmeasured and named in the test's own doc comment rather than
  approximated unreliably.
- **L1 — entitlement** (= `MONEY_AND_COINAGE_PLAN.md` M7, one implementation,
  D1). `TickHub.food_eaten`/`food_need_today` are written in the SAME eating loop
  `lack_basic` already reads (`mod.rs`, just before `update_food_and_starvation`
  runs), and `entitlement_bal_e` (`mod.rs`) blends them into the food balance —
  `bal_eaten = (food_eaten − food_need) / food_need` (≤ 0 by construction) against
  today's `bal_stock = (stock + production − need) / need`, via
  `min(bal_stock, bal_eaten + ENTITLEMENT_MARGIN)`. A household priced out (S7) or
  grain still locked in a warehouse now reads as genuinely underfed instead of
  counting toward "the city is fed" — this is what S7's own `HOUSEHOLD_
  MONETIZATION_DOSE` dose walk needed before it could be resumed (its doc comment
  names this exact fix). Shipped at **`ENTITLEMENT_DOSE = 0.0`** — a true no-op
  (`entitlement_dose_zero_is_a_noop`); `a_priced_out_city_reads_as_hungry` proves
  the blend direction and that an ordinary fully-fed hub (`bal_stock` at the
  margin) reads unchanged at full dose. **Raising the dose is unstarted, separate
  work** — §5 risk 1 of the plan says to expect `unrest_topples_councils` to fire
  MORE, not less, and to read the L0 count before judging a dose walk's result.
- **L2 — incomes + the welfare ratio, OBSERVE ONLY** (§3.2). `Pop.income` and
  `TickHub.welfare_ratio` are computed in `derive_pops` (`cities.rs`, yearly) from
  real hub state — a farm/craft pool from production VALUE (food goods vs.
  `Distribution::Manufactured`), a labour/soldier pool from `trade_last_year`, a
  clerk pool from `treasury`, a merchant pool from `export_earn`, and clergy/elite
  pools from `civic_pool` (a stand-in until L9 gives the church its own income and
  L11 gives the house ledger a resident-dividend read) — divided by each
  profession's live headcount (`INCOME_*_SHARE` constants, `mod.rs`). The welfare
  ratio is Allen's measure: labourer income ÷ the cost of a bare subsistence
  basket (`base_need` over basic-tier goods, priced locally) per head per year.
  **Nothing in the tick reads either field** (`sim_fingerprint`/the dynamics run
  are unchanged — verified: `cargo test --lib econ_` 6/6 bit-identical, incl. the
  multi-seed inheritance gate, 308s). `economy_validation.rs`'s scorecard gains
  `labourer_welfare_ratio` (printed, not asserted); the old `real_wage_index`
  (a sentiment blend, never an income — §1 F2) is kept and reprinted as "commoner
  wealth index" rather than removed, per the plan's own D1-adjacent continuity
  note. Gated by `incomes_sum_to_what_the_city_earned` and
  `welfare_ratio_is_finite_and_positive` (`tick::tests`).
- **L3 — annals + the Life tab v1** (§3.12). `TickHub.annals: Vec<CityYear>`
  (`mod.rs`) — one record per year (population, `welfare_ratio`, `lack_basic`,
  grain price, mood, unrest), capped at `ANNALS_CAP` (300, oldest dropped first,
  same shape as `HOUSE_EVENTS_CAP`). Recorded by `record_city_annals` (`cities.rs`),
  called yearly right after `update_unrest` so the year's welfare ratio and unrest
  are both fresh. Served read-only by `campaign_city_life(hub)`
  (`read_hubs.rs`/`lib.rs`), a plain clone of the stored Vec — no new query-side
  computation. `HubPanel.tsx` gains a **Life** tab (shown only while a campaign is
  running, on a non-estate hub): a one-sentence headline built from the unusual
  figures only (welfare ratio, hunger, unrest — the CityMarketView "quiet when
  ordinary" rule), population/grain/mood at a glance, and a welfare-ratio bar
  chart over the last dozen recorded years with a bare-subsistence reference line.
  Explicitly says what it does NOT yet show (age pyramid, causes of death,
  housing, church, notables — L4/L9/L13) rather than rendering it empty. Gated by
  `city_annals_fill_yearly_and_stay_capped` (`tick::tests` — a cheap 2-hub
  fixture run past `ANNALS_CAP` years in well under a second; the first cut used
  `dense_world()` and took 437s on its own, since the whole gate row exists to be
  run every `tick/` change, per §2.8's own "narrowest thing that could fail"
  rule); `npx tsc --noEmit` and `npx vite build` both clean.
- **L4 — vital rates + age bands** (§3.3). `TickHub.ages: [f32; 3]`
  (children/adults/elders shares, seeded `AGES_SEED` ≈35/50/15 — Wrigley &
  Schofield's stationary pre-transition pyramid), `male_adult_frac` (seeded 0.5),
  `deaths_by_cause: [f32; DEATH_CAUSE_COUNT]` (8, indices `CAUSE_FAMINE`/
  `_PLAGUE`/`_FEVER`/`_WAR`/`_FIRE`/`_FLOOD`/`_OLD_AGE`/`_INFANCY` — a plain index
  table, never an enum, so the array stays positionally serialized; `_FIRE`/
  `_FLOOD` stay at 0.0 until L8 exists to write them). `update_vital_rates`
  (yearly, `cities.rs`, called before `update_society` so this year's ageing is
  in place for the strata/pops it feeds) is UNCONDITIONAL bookkeeping, never
  gated by a dose, because it never itself writes `population`: fixed transfer
  rates age children into adults and adults into elders
  (`CHILD_TO_ADULT_RATE`/`ADULT_TO_ELDER_RATE`), a births share re-enters the
  children band, and the year's ordinary deaths — from the same crude-rate
  calculation the dosed path below uses — are drawn down weighted toward
  children and elders (0.45/0.40/0.15 — "infants and the elderly carry most of
  ordinary mortality") and tagged by cause: `CAUSE_FAMINE` above the same
  starvation floor `update_food_and_starvation` already uses to raise
  `starving`, `CAUSE_OLD_AGE`/`CAUSE_INFANCY` otherwise. **War deaths are tagged
  separately, at the moment they happen** — `spend_levy_casualties` (`war.rs`,
  widened `pub(crate)` for this) now thins `male_adult_frac` by the real
  casualty count and tags `CAUSE_WAR`, so a war visibly widows a city's adult
  band instead of shrinking `population` uniformly (§3.3's "war widows" effect).
  **The ONE path from vital rates into `population`** is `vital_net_rate_e`
  (`mod.rs`), blended into the existing daily net-growth term
  (`birth_rate * food_sec - DEATH_RATE_BASE`) at `VITAL_RATES_DOSE` — a real
  CBR/CDR calculation off food security, welfare ratio and starvation, with an
  early return at `dose <= 0.0` that hands back the untouched old formula, the
  same shape `entitlement_bal_e` already uses. Shipped at
  **`VITAL_RATES_DOSE = 0.0`** — a true no-op (`vital_rates_dose_zero_is_a_noop`).
  `a_famine_leaves_a_missing_generation` proves a starving hub accrues famine
  deaths and a well-fed one accrues none; `ages_move_over_time` proves the
  pyramid actually ages rather than sitting frozen at the seed;
  `war_deaths_fall_on_adult_men` proves a real levy casualty lowers
  `male_adult_frac` and tags `CAUSE_WAR`. **Raising the dose is unstarted,
  separate work**, and per the plan's own §0 cross-reference must be walked
  with `CAPACITY_LAND_WEIGHT` (`PLACES_DEMAND_AND_GROWTH_PLAN.md` slice 4)
  PINNED at 0.0 — two capacity/growth-shaping doses moving together cannot be
  told apart by a single gate run. `big_cities_die_faster_than_they_breed`
  (the urban-graveyard gate the plan's own build-order table names) is
  unwritten, queued alongside the dose walk itself rather than built ahead of
  having a live effect to test.
- **L5 — welfare into behaviour** (§3.4). Once L2 gave a real welfare ratio,
  three sentiment-only readers blend toward it, all sharing ONE lever
  (`WELFARE_BEHAVIOUR_DOSE`) so a single gate run walks the whole slice:
  `derive_pops`' per-pop `militancy` blends toward `welfare_militancy_e`
  (`mod.rs`) — a labourer earning under bare subsistence reads more militant
  than the SAME city's comfortable burghers in the SAME year, which
  `hub.lack_basic` (a city-wide average) cannot express; `update_society`'s
  hardship drain (the term that skews strata shares toward the underclass)
  blends toward `welfare_hardship_e`, clamped to the same 0.7 ceiling the
  sentiment-only term already used; and BOTH migration readers —
  `province_demography_pass`'s rural-pull weighting and `urban_exodus_pass`'s
  destination ranking — blend their prosperity term toward
  `welfare_opportunity_e` (welfare ratio rescaled onto the same 0..1 the old
  term used, Allen's ~2.0 "comfortable" band mapping to 1.0), so people move
  toward wages rather than sentiment. All three helpers share the identical
  early-return shape `entitlement_bal_e`/`vital_net_rate_e` already use:
  `if dose <= 0.0 { return old_value; }`. Shipped at
  **`WELFARE_BEHAVIOUR_DOSE = 0.0`** — a true no-op
  (`welfare_behaviour_dose_zero_is_a_noop`); `labourers_riot_before_burghers`
  proves the militancy/hardship blend direction (a labourer at 0.4× 
  subsistence reads markedly more militant than one at 2.5×, and a hub with a
  collapsed welfare ratio reads more hardship than a prosperous one);
  `welfare_opportunity_prefers_higher_wages_and_stays_bounded` proves the
  migration blend both prefers real wage gaps and stays within the 0..1 band
  the downstream ranking logic assumes. **Raising the dose is unstarted,
  separate work** — needs `unrest_topples_councils` and the `EXODUS_*`
  migration gates re-verified per dose step.
- **L13, partial — the age pyramid + causes of death, surfaced ahead of the
  rest of Life tab v2.** `CityYear` (the annals struct `record_city_annals`
  snapshots yearly) gained `ages: [f32; 3]` and `deaths_by_cause: [f32;
  DEATH_CAUSE_COUNT]`, both `#[serde(default)]` so an annal recorded before L4
  shipped loads as all-zero rather than failing — the frontend reads that as
  "not yet recorded", never as "an empty population". This is bookkeeping, not
  a new mechanism: both fields already existed on `TickHub` since L4 and were
  computed every year with nothing reading them; nothing here changes what the
  sim computes, only what the annals record of it, so it needed no dose gate
  and cannot move `sim_fingerprint`. The Life tab's L3-era footer ("age pyramid
  and causes of death wait on L4") is replaced with two real reads: a
  three-band population bar (children/adults/elders) and a stacked
  causes-of-death bar (the 8 `CAUSE_*` categories, cumulative since founding,
  fire/flood still 0 pending L8) — and a new, honest footer naming what still
  isn't there: housing, church, watch and named notables (L6/L9/L10/L12).
- **L6 — housing & crowding** (§3.5). `TickHub.housing`/`crowding` — a fresh
  or old-save hub seeds `housing = population * HOUSING_SEED_RATIO`
  (`housing_needs_seeding`, the same "reads as zero ⇒ seed me" convention
  `ages_needs_seeding` uses); `update_housing` (monthly, `cities.rs`) decays
  it a small share a year and, when crowded, closes part of the deficit
  through a pure, dosed helper (`housing_build_persons_e`, the N6/S3 `_e`
  pattern) that CONSUMES a real construction good straight from the hub's
  own market stock — `pick_build_supply_good`'s existing non-food category
  (already prefers timber/stone/iron/brick, the same supply-pick
  `construction_pass` uses for satellites), never conjured.
  `crowding = population / housing`, stored so the dosed readers below
  don't recompute it. **Unlike L4's age-pyramid bookkeeping, construction is
  NOT unconditional** — consuming real stock is a genuine economic action
  (it can move prices and, through them, wealth), so it is gated by
  `HOUSING_DOSE` too, alongside its two consequence readers. A first cut
  left construction unconditional on the L4 precedent and it moved
  `econ_inheritance_rules_fragment_differently` (seed 7: partible read
  richer than primogeniture) — caught by the gate, not by review, and fixed
  before shipping rather than argued past (§2.4's own rule: a spot failure
  on the aggregate gate is a revert, not a judgement call). Only seeding,
  decay and the `crowding` read stay unconditional, since none of them
  touch stock/prices/wealth. `HOUSING_DOSE` is kept apart from
  `VITAL_RATES_DOSE`/`WELFARE_BEHAVIOUR_DOSE` per the "two doses moving
  together" rule: `housing_crowding_net_adjust_e` subtracts an
  excess-mortality term from the daily net growth rate (composed AFTER
  `vital_net_rate_e`, never folded into it — a separate adjustment, not a
  shared lever), and `housing_crowding_unrest_e` adds a term to
  `update_unrest`'s target. All three — construction and both consequences
  — are true no-ops at `HOUSING_DOSE = 0.0`, and only crowding ABOVE 1.0
  ever costs anything. Shipped `HOUSING_BUILD_RATE` (5%/month of the
  deficit) is deliberately slow — "builders cannot keep pace" is the plan's
  own default story, not instant catch-up. **Not built this session,
  queued**: the rent term ("a slice of household income goes to housing
  owners — the resident houses and the church") explicitly waits on L9
  (the church doesn't exist yet to be one of the two payees); the
  growth-ceiling read ("the growth ceiling reads housing as one more
  capacity term") is left unwired rather than composed into
  `CAPACITY_LAND_WEIGHT`'s own dose walk, for the same one-lever-at-a-time
  reason; a Life tab surfacing of housing/crowding (fire/flood destruction
  wait on L8 regardless). Gated by
  `housing_dose_zero_is_a_noop`, `crowding_above_one_costs_more_than_housed_
  comfortably`, `housing_seeds_from_population_once`,
  `housing_build_persons_e_spends_real_stock_it_has` (`tick::tests`).
- **L7 — the settlement year (§3.6), FIRST of its four named parts only:
  seasonal mortality.** `seasonal_mortality_mult_e` (mod.rs) reads a hub's own
  hemisphere (`hub_lat_frac`) and climate (`koppen >= 14` ⇒ cold) and returns
  a multiplier peaking at a warm/wet hub's OWN summer (fever) or a cold hub's
  OWN winter (respiratory) — both tag `CAUSE_FEVER`, since `DEATH_CAUSE_
  COUNT`'s own doc comment leaves no slot for a dedicated respiratory cause.
  `update_seasonal_mortality` (monthly, `cities.rs`) gates the WHOLE pass on
  `CALENDAR_DOSE`, not just a downstream reader — L6's construction-gating
  regression is exactly why: unlike L4's age-pyramid bookkeeping, this
  changes real `population`, so it cannot be unconditional the way a pure
  label can. The monthly extra is SIGNED (both above and below the year's
  average), so the 12 monthly calls net close to zero over a year — a
  REDISTRIBUTION of `update_vital_rates`'s own yearly total onto its true
  season, never an added death rate stacked on top of it; only the POSITIVE
  (peak-season) excess is tagged into `deaths_by_cause[CAUSE_FEVER]`, since
  that tally is descriptive only. `CALENDAR_DOSE` is kept apart from
  `VITAL_RATES_DOSE`/`WELFARE_BEHAVIOUR_DOSE`/`HOUSING_DOSE` per the same
  "two doses moving together" rule. **Grain temporal CV was ALREADY printed**
  before this slice (`economy_validation.rs`'s `temporal_cv`, "grain price CV
  within a city", band 0.30–0.50) — nothing new needed there, and it is
  unmoved by this change since seasonal mortality touches population, not
  price. The other three named L7 parts — pre-harvest hoarding + `LAW_GRAIN`
  calling dearth earlier, a harvest-labour dip in the manufacturing cap with
  a seasonal wage bump, and a bounded feast-day calendar (folding the
  existing fair/pilgrimage seasons in rather than duplicating them) — are
  NOT built, queued. Gated by `calendar_dose_zero_is_a_noop`, `summer_fever_
  in_the_south_winter_deaths_in_the_north`, `seasonal_mortality_redistributes_
  rather_than_adds` (`tick::tests`, 302/302).
- **L8 — urban hazards (§3.7), FIRST of its three named parts only: fire's
  toll on housing and lives.** Rides on top of the EXISTING warehouse-stock
  fire event (`kind == "fire"` in `roll_events`, production.rs) rather than
  inventing a parallel mechanism — the stock burn, house-wealth loss, depot
  damage and estate strike all ship exactly as before, unconditional and
  UNCHANGED by this dose. `fire_settlement_toll_e` (mod.rs) scales the
  event's own severity (`mag`) by crowding (a crowded hub loses more to the
  same blaze), a fixed `timber_share` proxy at 1.0 (no per-hub
  building-material state exists yet — the plan's own "a burned city may
  enact a stone-rebuilding law, lowering future risk" is explicitly QUEUED,
  not built), and a dry-season risk curve that REUSES `seasonal_mortality_
  mult_e`'s warm-climate branch (fire risk peaks at a settlement's own
  dry/hot season regardless of whether its climate is generally hot or
  cold — one curve, two readers, not duplicated). Housing loss is
  hard-capped (`FIRE_HOUSING_LOSS_CAP`) — a fire burns A FRACTION, never the
  whole city — and the resulting death rate is a small, fixed share of those
  displaced (`FIRE_DEATH_RATE_OF_DISPLACED`: the Great Fire of London, 1666,
  lost ~13,200 houses to a handful of recorded deaths — fire destroys
  property far more efficiently than it kills). Deaths tag `CAUSE_FIRE`
  (reserved since L4, at 0.0 until this slice writes it). **Rebuilding
  demand needed no new code**: burning housing below its target simply
  reopens the deficit L6's `update_housing` already closes, the moment
  `HOUSING_DOSE` is raised — the two slices compose for free. Fires/century
  is now PRINTED (`econ_measure_settlement_life`, journal-filtered on the
  fire event's own text, the same lower-bound convention riots/revolts/
  plague strikes already use) — satisfying the build-order table's own named
  gate. Flood (riverine hubs, wet season) and water/sanitation (a new civic
  structure raising `public_health`) are NOT built this pass, queued. Gated
  by `urban_hazard_dose_zero_is_a_noop`, `fire_toll_is_bounded_and_worse_
  when_crowded` (`tick::tests`, 304/304).
- **L13 — housing/crowding now DRAWN in the Life tab.** `CityYear` gained
  `crowding: f32` (`#[serde(default)]`, the same rule-29 tail-alignment
  `ages`/`deaths_by_cause` already used), snapshotted yearly from
  `TickHub.crowding` (real since L6). Pure bookkeeping — the value was
  already computed, this only records it — so no dose gate, mirroring
  L13-partial's own precedent exactly. The Life tab draws a bar chart plus a
  plain-English reading ("comfortably housed" … "severely overcrowded").
- **L11 — persistent pops, SHADOW PHASE (§3.10).** `TickHub.pops_shadow:
  Vec<Pop>` runs in PARALLEL with the existing `derive_pops`, which still
  drives every real reader. Births re-enter a pop's OWN profession — the
  plan's "inversion" of L4's hub-wide children band; apprenticeship moves
  labourers into craftsmen; ruin (scaled by the hub's own structural
  `damage`) and famine (scaled by `starving`) move craftsmen/clerks/
  merchants back into labourers; the shadow's TOTAL reconciles onto the real
  population every year, since this pass tracks profession MIX, never net
  headcount — that stays L4's job. Migration is NOT modelled — exodus/rural
  pull carry no profession mix anywhere in the sim, a documented
  simplification, not an oversight. `pops_shadow` is read by NOTHING in the
  tick, proven directly (`persistent_pops_shadow_is_never_read_by_the_tick`
  runs two identical sims, one calling the shadow pass every year and one
  never calling it, and asserts `sim_fingerprint` is IDENTICAL) — so this is
  unconditional bookkeeping exactly like L4's age pyramid, no dose needed
  because nothing would read one. **The plan's own required instrument
  before `PERSISTENT_POPS_DOSE` may switch any reader over is now built and
  measured**: `econ_measure_persistent_pops_divergence` (`#[ignore]`d) finds
  craftsmen and soldiers diverge most (~0.04-0.10 mean share on both
  fixtures — apprenticeship/ruin/famine all move people through the
  craftsmen slot) while clergy/capitalists diverge least (~0.004-0.008); the
  transition rates themselves are named first approximations in their own
  doc comments, not a calibrated model. Gated by `persistent_pops_shadow_
  moves_people_between_professions`, `persistent_pops_shadow_is_never_read_
  by_the_tick` (`tick::tests`).
- **L12 — the townspeople, THREE of six named roles (§3.11).** `TickHub.
  notables: Vec<Notable>`, capped by `hub.tier` (floored at 1), rebuilt
  yearly right after the guild and Figure passes settle for the year — a
  guildmaster lookup sees this year's founded/dissolved guilds, an agitator
  lookup sees this year's Demagogue roster. Only the roles with a REAL
  institution to anchor to today: **Guildmaster** (the hub's strongest live
  `CraftGuild`); **Alderman** (the council house's own `kin[1]`, its second
  kinsman — a naming read over existing data, no new roll); **Agitator**
  (the EXISTING Demagogue `Figure` mechanic, LOCALISED — no new roll, no new
  effect, since its unrest bump already fires inside `raise_notable_
  figures`; this only anchors it to a city for display). The bishop (L9),
  the physician (L8's still-unbuilt water/sanitation half) and the watch
  captain (L10) are explicitly QUEUED, not fabricated — each needs an
  institution that does not exist in the sim yet, and inventing one to fill
  the slot would be exactly the fabrication rule 36/§2.4 forbid. A fresh
  appointment is chronicled once (quiet when the roster simply stands, the
  same discipline the Life tab uses everywhere else). The plan's own "±10%
  institutional nudge" (guildmaster → guild strength, alderman → civic
  mood) is real but `TOWNSPEOPLE_DOSE`-gated — 0.0 ships as a true no-op;
  the agitator carries no nudge of its own, since that would double the
  existing Demagogue effect. Served via a new `campaign_city_notables`
  query, drawn in the Life tab's new "The townspeople" section. Gated by
  `notable_count_is_bounded_by_tier`, `townspeople_dose_zero_is_a_noop`
  (`tick::tests`).
- **What did NOT ship, and why (rule 36 — queued, not waived)**: L7's other
  three named parts, L8's other two named parts (above); L9/L10 (the church,
  the watch) are each their own dose walk per the plan's own build order and
  are unstarted; L12's other three notable roles wait on them; raising
  `PERSISTENT_POPS_DOSE` above zero is unstarted, separate work now that
  L11's own divergence instrument gives it a real baseline; L6's own
  rent/growth-ceiling sub-effects wait as named above. Gates run across all
  sessions: `cargo check --lib --tests` (clean), `cargo test --lib
  tick::tests` (308/308, ~42s), `cargo test --lib econ_ -- --nocapture`
  (6/6 incl. the multi-seed inheritance gate), `npx tsc --noEmit` (clean).

---

### 5.8 `docs/living_world/02_PEOPLE.md` — the Individual system (Living World row 02), slices 02.1-02.5/02.8 shipped

The **second row** of `docs/living_world/00_INDEX.md`'s build queue (row 01,
feeds & pruning, is a separate branch's own work and was NOT re-verified
DONE before this row started — see that row's own status). ONE record —
`Individual` (`sim/campaign/tick/individuals.rs`) — for every named human the
campaign tracks: ruler, senator, commander, scholar, artisan, horde leader.
NOT `Person` (the realm genealogy struct) and NOT `Notable` (the L12
per-city local role, §5.7's own townspeople), which is unchanged in shape but
now carries a stable `individual_id` linking it to one.

- **Migration + linkage.** `migrate_figures_to_individuals` (one-time, gated
  by `people_migrated`) folds every still-referenced `Figure` into
  `CampaignSim.people`/`hall_of_dead` on first `advance()`, without touching
  `figures` itself (other code still reads it). `update_notables` now calls
  `individual_id_for_notable` for each seat it fills — it carries the SAME id
  forward while the same name holds the seat (read off last year's roster
  entry) and only mints a fresh `Individual` (`spawn_individual`) when the
  seat genuinely changes hands, which is what makes the yearly rebuild not
  mint a new person every year (`local_roles_do_not_mint_new_people_yearly`).
- **Traits/modifiers/`decide()`** (`individuals.rs`) — ~42 traits in five
  groups (personality/education/lifestyle/health/reputation, the health ones
  doubling as a face-feature bitflag), 16 modifiers at a flat ±5%
  (`MODIFIER_MAGNITUDE`), and `decide()`: the 75% rule, symmetric per option
  (`DECISION_CERTAIN_AT`), deterministic via `hash01(seed, tick,
  individual_id, kind)`, with the chosen option's top reasons logged. It is a
  pure, tested function — **not yet wired to a decision SITE** (the 12 named
  decision kinds in the design doc, e.g. "stay or flee a plague", have no
  caller yet); that wiring is real, queued future work.
- **Life cycle** (yearly, `people_yearly_pass`) — aging + mortality (reuses
  `person_mortality_hazard`, `realms.rs`), fame decay, notable
  promotion/demotion under the 40-alive cap (`NOTABLE_CAP`,
  `NOTABLE_FAME_THRESHOLD`), and death handling
  (`remove_dead_individual`, split out for direct testability): a famous
  death moves to `hall_of_dead` (kept forever, modifiers cleared); an
  ordinary death is forgotten but leaves a capped `Tombstone`
  (`people_tombstones`, `TOMBSTONE_CAP`).
- **Life events** (`life_events.rs`, the new **weekly** `tick % 7` cadence
  hook) — a yearly quota per person (`role_event_base` × `event_turbulence`,
  a hashed-Poisson draw capped at 10) spent gradually across that year's
  weeks rather than landing on New Year's Day, and a layered template pool
  (role+geography → geography → a guaranteed in-city generic layer, so a due
  event always finds one — `a_due_event_always_finds_a_template`). ~40
  starter templates (`EVENT_TEMPLATES`), gated by
  `life_event_templates_respect_geography` — a keyword→tag lint that fails
  if a template's TEXT implies a tag (whale/ship→coast, camel/dune→desert,
  frost→cold, walls/levy→at-war, granary/hungry→famine…) it does not
  `requires`. Text is rendered at READ TIME from `(template_id, args)`
  (`render_life_entry_for`), never baked in at fire time. Only a FAMOUS
  person's event reaches the world journal (the Chronicle-salience
  discipline §5.7 already established); an ordinary person's own
  `life_log` is capped at `ORDINARY_LIFE_LOG_CAP` (12).
- **Scope cuts, recorded rather than faked (rule 36).** Culture is a
  `String` on `Individual` (matching `hub_culture: Vec<String>`'s own
  convention), not the design doc's "index into a culture table" — no such
  table exists anywhere in `tick/`, and building one for this row alone
  would be new infrastructure the row doesn't need. `context_tags`/
  `event_turbulence` cover only geography/state signals `TickHub` actually
  carries (`coastal`/`river`/`koppen` bands/`war_with`/`starving`/
  `plague_immune_until`); siege and festival turbulence, and
  lake/mountain/forest tags, are QUEUED behind a producing signal, not
  invented. 02.6 (~150 more templates, reviewed by an agent against the same
  lint) is explicitly a **separate session** per the design doc. 02.7 (faces
  — feature layers + a sex axis on `cultureDress.ts`, keyed off
  `Individual.face_seed`/`features`, both already served end to end via
  `IndividualBrief`) was deliberately NOT attempted this session: it is real
  procedural-art work this environment cannot visually verify (no display),
  and shipping a canvas layer blind risks exactly the kind of regression
  §8.21's own fill-light story warns about.
- **Commands** (`campaign_commands/read_people.rs`) —
  `campaign_get_individual(id)` / `campaign_get_notables` /
  `campaign_get_hall_of_dead`, wired lib.rs → `bridge/campaign.ts` →
  `types/campaign.ts` (`IndividualBrief`). A new floating window,
  `ui/campaign/NotablesPanel.tsx` (Society menu → "👥 Notables & Hall of the
  Dead"; `uiStore.showNotables`), lists the living roster and the Hall,
  each row expanding into its rendered life log — plain-list, not yet the
  portrait gallery `FiguresPanel.tsx` is meant to become (02.8's own
  "FiguresPanel.tsx becomes the roster's front page" is still queued).
- Gates (`tick::tests`): `figures_migrate_to_individuals_losslessly`,
  `local_roles_do_not_mint_new_people_yearly`, `living_world_is_inert_at_zero`
  (calls the row's own passes directly — migration, the weekly hook, the
  yearly life cycle, firing 30 events on a fully-decorated person — and
  asserts every hub/house economic field is bit-identical before/after;
  a `Figure`'s own PRE-EXISTING effects, e.g. a Master Craftsman lifting
  quality, are a different, older mechanism and are deliberately NOT
  exercised by this gate, or it would be testing the wrong thing),
  `decision_at_75_percent_is_certain`, `modifiers_can_tip_either_way`,
  `decisions_are_deterministic`, `notables_never_exceed_the_cap`,
  `dead_notables_keep_their_story`, `dead_ordinary_people_are_removed`,
  `a_due_event_always_finds_a_template`, `event_rate_follows_turbulence`.
  See `docs/SCOREBOARD.md` 2026-09-25 for the full-suite numbers.

### 5.10 `docs/living_world/05_CULTURE_ACCEPTANCE.md` — five acceptance tiers, sparse relations, dosed effects (Living World row 05)

**Note on §5.9:** rows 03 (`03_DEVELOPMENT_TRACKS.md`) and 04
(`04_GOVERNMENT_AND_EDICTS.md`) shipped on `main` before this row but were
never given their own CLAUDE.md map entries — a pre-existing documentation
gap, not introduced here, and out of THIS row's own scope to backfill. A
cross-reference at §5's own province-work text already cites "§5.9" for
future Realm-layer content that was never written; this row's own entry is
`§5.10` rather than claiming that number, so a future session documenting
rows 03/04/Realms is not misdirected.

Each city treats each culture it holds a relation with differently — from full
Citizens to Hated, with no rights — and that treatment drifts with trade, war,
feuds, famine and the row 03 admiration/barbarian-judgement hooks
(`culture_ideals.rs::admires_more_developed`/`is_barbarian_to`, built in row 03,
"called by nothing" until now — this is their first real caller).
`sim/campaign/tick/culture_acceptance.rs` (new theme file, `pub(crate)` impl
block, `use super::*`, declared in `mod.rs` beside `government`) carries the
whole mechanism.

- **Five tiers, sparse state.** `TickHub.culture_relations: Vec<CultureRelation>`
  — ONE entry per culture actually resident (≥ `ACCEPT_MIN_MINORITY_SHARE` = 2%
  minority share) or holding the majority, never a 1,200×every-culture matrix
  (00_INDEX's own storage rule). `CultureRelation{culture, tier, score, trend,
  reason, proposed_tier, debate_round, debate_tally}`, `score` −100..100, `tier`
  derived by `tier_for_score` with a HYSTERESIS band at each boundary (same
  discipline `assign_house_tiers`/`assign_city_tiers` already use) so a relation
  sitting on a boundary doesn't relabel every year. Lazily seeded
  (`ensure_culture_relations`, the `*_needs_seeding` convention) — an old save or
  a hub the pass hasn't reached yet reads empty until the first year it runs.
- **Stance** (`culture_stance_bias`): remoteness (few LIVE trade-component mates,
  precomputed ONCE per pass into a `HashMap<component, count>` rather than
  rescanned per relation — the O(hubs) cost this row's <100 ms/year budget
  forbids paying per relation) and Insular(2)/Xenophobic(13) traits pull closed;
  Mercantile(0)/Assimilative(7) traits and real `trade_last_year` pull open —
  exactly the doc's own "remote cities are conservative, trading cities are
  open" rule, gated by `default_tiers_follow_stance_and_relation`.
- **Yearly drift** (`culture_relation_drift`, called from
  `culture_acceptance_yearly_pass`): trade volume (the minority's own share ×
  the hub's own `trade_last_year`, the cheapest real proxy available without a
  per-culture trade ledger — a genuine simplification, named as such), resident
  merchants (the same share term), shared language kit (`cultures::
  kit_of_people` match), admiration/barbarian judgement (row 03's real hooks,
  not stubs), dominant-foreigner resentment (share > 35%), war (this hub's
  `war_with` target's own majority culture), feuds (a precomputed
  `HashMap<(hub, culture), f32>` built ONCE per pass from the bounded
  `self.feuds` list, never rescanned per relation), and famine/plague blamed on
  outsiders (`hub.starving > 0.3` while the MAJORITY leans Insular/Xenophobic).
  Gated by `trade_raises_relations`, `war_lowers_them`.
- **Tier-change proposals do NOT share row 04's `GovDebate` slot.** A city
  debates only ONE ordinary edict at a time (04.4's own single-slot design);
  routing every culture-tier crossing through that one scarce slot would starve
  either ordinary government edicts or culture policy. Each `CultureRelation`
  instead carries its own small "shadow debate"
  (`proposed_tier`/`debate_round`/`debate_tally`, resolved by
  `run_acceptance_debate_round`) — the SAME math shape row 04's `run_debate_round`
  uses (`gov_position`, `stability_at`, a hashed noise term) but on its own
  per-relation state, so a culture question and an ordinary edict can be live at
  once. A passed proposal is chronicled (`chronicle_acceptance`,
  `JournalEntry{kind: "acceptance_changed"|"acceptance_restricted"}`) under
  `government.rs`'s own `EDICT_FAM_CITIZENSHIP`/`EDICT_FAM_FOREIGNERS` families,
  so a citizenship change reads as the same KIND of event whichever row
  produced it. A real merge into one shared agenda list (so the Government
  window shows both kinds of proposal together) is queued (`Q05.3`).
- **Persecution & diaspora** (`maybe_persecute`, tier 5 only): a modest yearly
  roll (12%, a small slice of which turns massacre) always chronicles
  expulsion/massacre and always computes the diaspora DESTINATION (the
  highest-scoring other hub for this culture in the same trade component) —
  but at the shipped `PERSECUTION_DOSE = 0.0`, `persecution_migration_frac_e`
  returns exactly `0.0` and no population or craft `tradition` actually moves.
  Gated at a nonzero TEST dose by `expulsion_moves_residents_and_tradition`
  (the doc's own "at a test dose" instruction).
- **Bondage attitude** (`bondage_attitude_for_traits`, pure, from Martial(3)/
  Nomadic(5) leaning permissive and Devout(4)/Scholarly(9) leaning against — a
  documented simplification, not a full historical table) and
  `bondage_permitted(h)`, which reads a new per-hub `bondage_override: i8`
  (−1 unset/follow culture, 0 abolished, 1 permitted — a direct toggle rather
  than routed through the full weekly debate, a deliberate scope cut recorded
  as `Q05.4`). Gated by `bondage_follows_culture_until_edict`.
- **Fondaco activation** (`maybe_charter_culture_fondacos`, dosed
  `FONDACO_CHARTER_DOSE = 0.0`): a house whose home hub's majority culture
  (houses carry no `culture` field of their own — read via `hub_culture[house.hub]`,
  the same reading every other culture-aware house pass in this tree already
  uses) holds tier 2–3 at a DIFFERENT city may be chartered a `Fondaco` there
  (`CampaignSim.fondacos`, the same struct/list §5's own Fondaco entry names).
  Kept deliberately SEPARATE from `yards.rs::maybe_found_fondaco`
  (`YARDS_VESSELS_AND_DEPOTS_PLAN.md` W5's own zero-dose stub, a generic
  vessel-capacity mechanism with no culture trigger) — both dosed at zero, so a
  world with no fondaco founded is bit-identical regardless of which plan's
  session ran last.
- **Effects (05.5), each a pure `_e` function dosed from zero**:
  `acceptance_tax_mult_e` (tier 3 small surcharge, 4 large, 5 largest),
  `acceptance_settle_mult_e` (1–3 normal, 4 rare, 5 none),
  `acceptance_office_allowed_e` (tiers 1–2 only, at full dose),
  `acceptance_dev_share_e` (row 03 hook — tiers 1–3 share, 4–5 none). Two are
  named FORWARD HOOKS rather than approximated: `acceptance_scholar_mult_e`
  reads a constant `1.0` regardless of dose, because rows 06/07 (scholars,
  artisans) do not exist yet to have an appearance chance to scale; `acceptance_
  cohesion_term_e` reads a constant `0.0`, because row 09's `Realm` does not
  exist yet to read it. None of the five is wired into a live tax/migration/
  office/development pass — the SAME "built and tested, called by nothing"
  precedent row 03's own `admires_more_developed`/`is_barbarian_to` already set
  in this tree, recorded here rather than silently mirrored: wiring a real
  effect needs a per-culture population/wealth attribution `hub_minorities`'s
  plain share does not yet carry at the granularity these effects need, queued
  as `Q05.4` (renumbered from the doc's own `Q05.1`/`Q05.2` forward). Gated by
  `acceptance_effects_are_noops_at_zero`.
- **Cadence & wiring.** `culture_acceptance_yearly_pass` runs at the existing
  yearly hook (`tick % 365`) right after `run_diaspora`/before `prune_chronicles`
  — after `update_notables`/`update_government` so this year's `gov_position`/
  officials are fresh, and after the war/feud passes so `war_with`/`feuds` are
  settled. `maybe_charter_culture_fondacos` runs immediately after it.
  `O(hubs × resident cultures)` with two ONE-TIME-PER-PASS precomputed maps
  (component mates, feud pressure) — no per-day scan, no per-relation rescan of
  the world.
- **UI**: `campaign_get_culture_acceptance(hub)` (new
  `commands/campaign_commands/read_culture.rs`, registered in `lib.rs`,
  `bridge/campaign.ts::campaignGetCultureAcceptance`,
  `types/campaign.ts::CultureAcceptanceBrief`) — a plain per-culture list, tier/
  score/trend/reason/residents-share/open-proposal. Surfaced as a "Culture
  acceptance" section appended to `HubPanel.tsx`'s existing Government
  ("govt") tab, rather than a new tab of its own — the govt tab is already the
  city's civic-state home and a culture relation IS civic state, so a new tab
  would only duplicate the fetch/width plumbing that tab already carries.
  **Verification caveat, stated plainly**: this environment has no display, so
  the UI was verified by `npx tsc --noEmit` (clean) and `npx vite build` (clean,
  189 modules) only — never opened in a real window. Visually exercising it is
  still owed, the same caveat every Living World UI slice in this tree carries.
- **Gates** (`tick::tests`): `default_tiers_follow_stance_and_relation`,
  `trade_raises_relations`, `war_lowers_them`,
  `expulsion_moves_residents_and_tradition`, `bondage_follows_culture_until_edict`,
  `acceptance_effects_are_noops_at_zero`, and
  `culture_acceptance_pass_is_inert_at_zero` (the row's own `living_world_is_
  inert_at_zero`-shaped fingerprint gate — runs the real yearly pass for 8 years
  on a fixture carrying a real minority, an active war and starvation, and
  asserts population/treasury/price/house-wealth are bit-identical before/after,
  while also asserting a relation DID seed and drift — proving the mechanism is
  real, not merely absent). See `docs/SCOREBOARD.md` for the end-of-row numbers.
- **Queue** (rule 36 — see `05_CULTURE_ACCEPTANCE.md`'s own `## Queue` for the
  full, numbered list): realm-wide tier policy (row 09, doc's own `Q05.1`);
  intermarriage raising relation scores (waits on row 02 marriages, doc's own
  `Q05.2`); merging tier-change proposals into row 04's own shared agenda list
  (`Q05.3`); wiring the five dosed effects into a real tax/migration/office/
  development pass, which needs a per-culture population/wealth attribution
  this row does not yet have (`Q05.4`); a reach-bounded diaspora-destination
  search rather than the current same-trade-component scan, once dosing
  `PERSECUTION_DOSE` above zero makes the destination choice matter for real
  (`Q05.5`); raising any of the five effect doses above zero, each needing its
  own `econ_`-per-dose-step walk per 00_INDEX's own testing rule (`Q05.6`).

---

## 6. Rust Backend Map (`src-tauri/src/`)

```
main.rs                         ← Binary entry (calls lib run)
lib.rs                          ← Plugin registration + the FULL invoke_handler list
                                  (every #[tauri::command] MUST be registered here)

commands/
  world_commands.rs             ← New world, grid/meta setup, world_progress
  sim_commands.rs               ← Tauri wrappers for sim phases (per-phase ColumnSet masks)
  paint_commands.rs             ← Paint strokes (land/elev/shelf/volcano)
  tile_commands.rs              ← get_tiles / get_tiles_packed (RGBA fetch), LOD
  query_commands/               ← Read-only overlays + coarse routing (split into a folder;
                                  mod.rs keeps shared use/structs/helpers + `pub use <child>::*`
                                  so external paths query_commands::* are unchanged):
      cell.rs · routing.rs        cell_info; trade routes/matrix/trunks + compute_itinerary
      overlays.rs · political.rs  shark/shipworm/reef/storm/monsoon + good/culture regions;
                                  overlays.rs also carries `compute_good_belt_masks`
                                  (§8.19) — the FULL-RESOLUTION two-layer belt mask;
      economy.rs · flow.rs        political ranking; market economy; dynamic trade flow
  campaign_commands/            ← finalize/unfreeze, campaign lifecycle + ALL campaign read
                                  queries. Split into a folder (mod.rs re-exports children;
                                  campaign_commands::* paths unchanged):
      lifecycle.rs                finalize/new/save/open/progress/persist/start/advance/state
      read_hubs.rs · read_money.rs  hubs/journal/houses; coin/bank/crash/war/inequality/poleis
                                  read_hubs.rs also serves `campaign_market_cities`
                                  (the Markets picker's LIVE city list) and, on every
                                  `HubGoodDetail`, `price_hist`/`vol_hist` — the
                                  persisted per-(hub, good) yearly series
      read_people.rs · read_colonies.rs  cultures/pops/figures/dynasties; colonies/migration
      read_trade.rs               goods/routes/futures/warehouses/guilds/schematics/diagnostics
                                  + campaign_guild_atlas (S9 — a craft's inputs/
                                  outputs/reach, pure derived read, §5.6)
      read_houses.rs              House Dossier reads: the five STABILITY gauges
                                  (campaign_house_stability) + the FEUD board
                                  (campaign_get_feuds) + the KIN roster
                                  (campaign_get_house_kin, Phase 2.1) + AMBITIONS
                                  (campaign_get_house_goals, Phase 3.1) + the CRISIS
                                  (campaign_get_house_crisis, Phase 3.2-3.6 — the live
                                  struggle + the permanent past-risings record, each
                                  side's brief now carrying a derived MOTIVE phrase —
                                  `kin_motive`/`head_motive`, read from loyalty/role/
                                  posted-hub/vice, no new persisted state) + LINEAGE
                                  (campaign_get_house_lineage — walks `House.origin_house`
                                  up to 64 hops to the founding, reversed root-first, plus
                                  this house's own offshoots found by scanning for
                                  `origin_house == this`; each hop's `origin_kind` says
                                  WHY: guild-seeded / branch / Partible division /
                                  Departure schism / independent founding) + the
                                  ATLAS (`campaign_house_atlas`, HOUSES_GUILDS_
                                  AND_MARKET_PLAN.md S8 — partner cities, goods
                                  portfolio, holdings, seasonal lane ease; pure
                                  derived read, §5.6) + the BUMP CHART
                                  (`campaign_house_bump_chart`, S12a — the top
                                  houses' wealth rank per year, from each
                                  house's own `wealth_history`; pure derived
                                  read, §5.6) + TIER-1 GAUGE MEDIANS
                                  (`campaign_tier1_gauge_medians`, S12c —
                                  calls `campaign_house_stability` verbatim
                                  per tier-1 house and medians each gauge,
                                  for the Standing tab's radar chart; pure
                                  derived read, §5.6). Four
                                  of five gauges are pure derivations of state the sim
                                  already held; kin_power_shares/character_phrase
                                  (Phase 2.6/2.3) and the whole crisis engine live in
                                  sim::tick so they're gated by tests, not just called
                                  from here.
      province.rs                 province LAND state (campaign_province_land[_all]) +
                                  the CONTROL VERBS — the only mutating campaign
                                  commands besides campaign_advance (§5.1). Phase 5 ·
                                  `ProvinceLand.holder_house` (−1 = a city administers)
                                  alongside the existing `holder_hub`; `holder_name`
                                  reads as the holding HOUSE's name when one holds
                                  the writ — no new command, just a field added to
                                  the existing query. `campaign_province_goods` (§2.5)
                                  — the goods exploitation reading, a pure derived
                                  read over `CampaignSim::province_good_*` (§5).
                                  `compute_states` (§3.3) — every tier 1-2 city's
                                  writ as a territory (name/colour/cells), a pure
                                  derived read over `prov_holder`/`province_raster`,
                                  nothing persisted. `campaign_province_trade` —
                                  who commands a province's commerce (share by
                                  house/guild via `House.trade_at`, by city) + the
                                  per-good exports/imports crossing its boundary
                                  (`prov_export_year`/`prov_import_year`, accrued in
                                  `accrue_flow`, snapshotted in `roll_city_finances`,
                                  gated on a seeded province layer so a province-less
                                  sim stays bit-identical). Feeds the Inspector's
                                  Trade-tab donuts AND documents the ≥ `PROV_TRADE_
                                  CONTROL_FRAC` (0.20) realm-eligibility path: a house
                                  commanding a fifth of a province's trade may
                                  proclaim at its seat WITHOUT the seat writ — the
                                  measured funnel (`econ_measure_realm_formation`)
                                  collapsed exactly at the writ gate (24 tier-1-2
                                  dynasties, only 3 hold one)
  read_culture.rs                 living_world/05_CULTURE_ACCEPTANCE.md slice 05.6 ·
                                  `campaign_get_culture_acceptance(hub)` — the sparse
                                  per-culture tier/score/trend/reason table, a pure
                                  derived read over `TickHub.culture_relations`
  goods_commands.rs             ← Goods spec CRUD, default_custom_goods, backfill
  goods_import.rs                 ← DEPOSITS_AND_MINING_PLAN slice 3: the INI-ish
                                  `.txt` goods importer (`import_goods_txt`) —
                                  add-only (D8), always reports every default/
                                  reject, never a silent drop
  import_commands.rs            ← import_world_layers (layered world import)
  preview_commands.rs           ← preview_zonal_profile / preview_coarse_climate (§8.14)
  campaign_library.rs           ← THE CAMPAIGN LIBRARY: a real folder on the user's
                                  disk (`<Documents>/WorldForge2 Campaigns`, re-pointable,
                                  openable in the OS file manager) holding `.campaign`
                                  saves, listed in-app with the YEAR each reached.
                                  `campaign_library_dir`/`set_…`/`reveal_…`/
                                  `list_campaigns`/`save_campaign_to_library`/
                                  `delete_campaign_file`. **Listing must never parse a
                                  `campaign_sim` blob** — a long run serializes to
                                  megabytes — so every save stamps a small
                                  `campaign_summary` header and the listing reads only
                                  that; a pre-header file falls back to a BOUNDED 4 KB
                                  scan for `"tick":` and reports the year as unknown
                                  past that window rather than lying or paying for a
                                  megabyte scan. `delete_campaign_file` refuses any
                                  path outside the library folder
  palette_commands.rs           ← get_render_palettes (§8.18) — serves the renderer's
                                  OWN colour tables to the legend, so the key cannot
                                  drift from the map. Read-only, touches no tile.
                                  Also carries `GOOD_QUALITY_STOPS` — the ONE
                                  absolute 0–1 belt-quality scale every good's
                                  quality layer shades on (§8.19, D10) — and
                                  `elevation_styles`, every named elevation
                                  style's own served land+sea ramps (§8.22)
  template_commands.rs          ← Image → land/sea detection (4-bit quantization)
  file_commands.rs              ← Save/open world (.worldforge), export heightmap/layers

db/
  mod.rs                        ← WorldDb (Mutex<Connection>), in-memory SQLite
  schema.rs                     ← Tables: tiles, metadata, objects, campaign (k/v), undo_journal
  metadata.rs                   ← get_meta/set_meta typed helpers
  tile_store.rs                 ← load_tile / save_tile (zstd blobs, v2 self-describing)
  world_cache.rs                ← In-memory WorldBuffer/tile cache keyed by fingerprint

tile/
  mod.rs · cell.rs              ← TileData: 20+ columnar Vec fields per 128×128 tile
  coords.rs                     ← TILE_SIZE=128, wrap_x/clamp_y coordinate math
  lod.rs                        ← LOD supertile pyramid (lod 1-4)

render/
  mod.rs · tile_image.rs        ← 26 render layers (land, elevation, climate, … see §8.7);
                                  the biomes layer carries PROCEDURAL PATTERN FILLS (§8.12).
                                  Also the SHARED RELIEF CORE (`relief_at`) — two lights +
                                  ambient occlusion — used by both shaded base layers, and
                                  `sea_shade`, which shades the seafloor for the first time.
                                  ELEVATION STYLES (§8.22): seven named alternative
                                  palette+relief treatments for "elevation"/"terrain"
                                  (`"elevation#style=alpine"`), one shared
                                  `render_elevation_styled` keyed off `StyleParams`
  natural.rs                    ← NATURAL COLOUR (§8.21): the land-cover palette the
                                  `natural` layer draws with. A SECOND, independent table,
                                  deliberately NOT `tile_image::biome_color` — that one is
                                  held apart at a CIELAB floor for thematic legibility and
                                  this one is deliberately overlapping, because real
                                  vegetation grades continuously

paint/
  mod.rs · stroke.rs            ← PaintValue enum (terrain/elevation/shelf/volcanic)
  brush.rs                      ← circle_brush with cylindrical wrapping

import/mod.rs                   ← TileData::merge_columns (layer-group import)
history/mod.rs · undo.rs        ← Tile-level undo/redo journal

sim/                            ← organised into per-phase step folders; mod.rs re-exports
                                  each leaf module so paths stay sim::plates, sim::tick, …
  mod.rs                        ← Sim module declarations + `pub use` re-exports
  world_buffer.rs               ← WorldBuffer: flat arrays + per-phase ColumnSet masks
  step1_plates/plates.rs        ← Ph1: Voronoi plate tectonics; terrain is a warped
                                  "crust thickness" threshold, not the raw Voronoi
                                  edge (Terrain 2.0 slice 4)
  step1_plates/landmass_ops.rs  ← Stage-1 freehand AREA TOOLS (§8.25) — Lasso +
                                  smooth_roughen/fjords/island_chain/fill
  step2_terrain/elevation.rs    ← Ph2: plate-based + template-based elevation.
                                  `stream_power_erosion` (priority-flood + flow
                                  accumulation + K·A^m·S^n incision) replaced the old
                                  droplet simulation (Terrain 2.0 slice 1); outer-pass
                                  count is keyed to GRID SIZE, not the `iterations`
                                  strength knob (see `terrain_metrics`, §3 below).
                                  NO VALLEY CARVING (§8.23): `stream_power_erosion`
                                  and both noise `carve` terms are GONE — only
                                  `thermal_erosion` (rounds, never incises) and
                                  `limit_grid_scale_relief` remain
  step2_terrain/landform.rs     ← PHYSIOGRAPHIC PROVINCES (§8.24): the regional
                                  terrain-CHARACTER mosaic — relief amplitude,
                                  ruggedness, fine-detail weight per cell, plus
                                  plateau/basin shaping. Transient like geology.rs
  step2_terrain/geology.rs      ← TRANSIENT geology (Terrain 2.0 slices 2-3):
                                  recomputed from seed + persisted plate data every
                                  phase-2 run, used, discarded — no tile column.
                                  Lithology noise, real orogeny setting/age (plate
                                  model only) or a relief pseudo-setting (the other
                                  three models), the phase-2 climate-erosion proxy,
                                  and the region id `redistribute_elevation_regional`
                                  keys off
  step3_ocean_atmo/             ← Ph3 (the physics core — see §8.2):
      ocean.rs                    winds · Sverdrup gyres · currents · salinity · thermohaline · SST
      insolation.rs               astronomical daily-mean insolation (ANY obliquity)
      ebm.rs                      1-D diffusive North–Budyko energy balance + ice-albedo
      circulation.rs              Hadley edge / polar front derived from ROTATION rate
      temperature.rs              base curve + EBM anomaly + lapse + currents + coastal damping
      jets.rs · seasonal.rs       low-level jets (Somali) · two-season winds & monsoon
                                  (`itcz_land_pull`/`itcz_latitude` — the ITCZ
                                  position the belts are displaced about; shared
                                  with the ITCZ overlay so the drawn line IS the
                                  modelled one)
      precipitation.rs            advection-decay moisture + ITCZ/orographic/frontal/jet terms
      preview.rs                  SETTINGS PREVIEW (§8.14): 1-D zonal profile + coarse
                                  climate map — read-only, never touches a tile
      bench.rs                    (test-only) phase-3 PERF harness + field checksums — see §8.9
  step4_climate/                ← Ph4: koppen.rs (31 zone codes; H retired, A15) ·
      earth_validation.rs         THE EARTH FIDELITY GATE (§2.3) + fixtures/
  step5_rivers/                 ← Ph5: rivers.rs (priority-flood/rivers/lakes) · aquatic.rs
                                  (freshwater ecology: fish assemblage, lake limnology)
  step6_soil_fertility/         ← Ph6: soil.rs (12 soil types) · fertility.rs (fisheries)
                                  · biome.rs (Ph6b: 41 ECOLOGICAL BIOMES — see §8.12)
  step7_settlements/settlements.rs ← Ph7: habitability → city placement (Settlement struct)
  step8_biological_goods/       ← Ph8: biological.rs (shark/shipworm + belts)
                                  · deposits.rs (THE ORE GEOLOGY — 11 deposit
                                    models scored from plate boundaries/volcanism,
                                    belt→district→working hierarchy, per-working
                                    grade/extent/depth — see §8.16)
                                  · localities.rs (THE AGRICULTURAL/BIOLOGICAL
                                    counterpart of deposits.rs — belt→locality
                                    hierarchy + full modulation for every
                                    Global/Local good, see §8.19)
                                  · goods_spec.rs (GoodSpec, 45 belts + ~21 manufactured;
                                    `marine_band` — Inshore/Bank/Either, §8.19;
                                    `origins` — independent homelands per good;
                                    `soil`/`relief` — the FINE-GRAIN terroir terms,
                                    §8.20; `Distribution::Endemic`, §8.20)
                                  · goods_validation.rs (test-only — Slice 0's coverage
                                    diagnostic, §8.19)
  shared/                       ← cultures.rs (organic peoples map + 14 traits + per-kit
                                    male AND female given names) · inheritance.rs (the LAW
                                    OF INHERITANCE — line rule + division rule per culture,
                                    §8.15) · toponyms.rs
                                  · names.rs (deterministic place/family/head names)
                                  · provinces.rs (TWO-STAGE partition — see §8.10; the
                                    natural world↔campaign join layer, see FIX_PLAN B1)
  campaign/                     ← the campaign half:
      market.rs                   Market equilibrium solver (stocks → grain-eq prices)
      manufacture.rs              Shared production-chain resolver (DAG topo, labor∝pop)
      tick/                       THE CAMPAIGN TICK SIM (~17.6k lines, by theme). See §5.
                                  mod.rs = structs/consts/free-fns/advance()/impl Bank/…/
                                  residual impl CampaignSim; methods grouped into money/war/
                                  disease/colonies/polis/cities/houses/production/crisis/
                                  schism/foreign_hand child impls (pub(crate), `use
                                  super::*`); crisis.rs = Phase 3.2-3.6, the succession-
                                  crisis engine (competence/vice, named factions,
                                  quarterly rounds, resolution, civic intervention, the
                                  permanent record — see §5); individuals.rs =
                                  Living World row 02, the `Individual`/`Modifier`/
                                  `LifeEntry`/`Tombstone` roster, migration,
                                  `decide()`, the yearly life cycle (§5.8);
                                  life_events.rs = row 02's weekly event engine +
                                  the ~40-template starter set; schism.rs = Phase 4.1,
                                  Quarrel/Departure (a simplified `tension` proxy,
                                  monthly; Rupture deferred); foreign_hand.rs = Phase
                                  4.4, the two-channel rival-leverage loyalty decay —
                                  built only after its own measured trigger rate
                                  justified it (§5); disease.rs also carries Phase
                                  4.3's `plague_house_toll` (kin mortality + extinction,
                                  independent of head mortality); tests.rs = the
                                  dynamics tests; economy_validation.rs carries the
                                  `#[ignore]`d long-run diagnostics
                                  (`econ_diagnose_house_turnover`,
                                  `econ_measure_foreign_hand_conjunction`,
                                  `econ_measure_war_frequency` — CITY_PROVINCE_WAR_
                                  PLAN.md §3.4f, measures the PRE-3.4a–e war
                                  mechanism: 6.0 wars/century, two-thirds of them
                                  colony independence wars rather than
                                  `maybe_declare_war`'s rival-city path, every
                                  resolution landing at the 2-year floor — see
                                  SCOREBOARD.md) alongside the ECONOMY FIDELITY
                                  GATE (§2.5)
```

---

## 7. React Frontend Map (`src/`)

```
main.tsx                        ← React entry / mount
App.tsx                         ← Layout, header, file dialogs, NewWorldDialog, mounts panels
types/                          ← ALL shared TS types (mirror Rust serde structs), split
                                  world.ts/campaign.ts/goods.ts + index.ts barrel (@types)
goods.ts                        ← GOOD_DEFS (names/emoji) shared metadata (@goods)
commodityHistory.ts             ← #36 real-world commodity-history cards (@app/commodityHistory)
settlementStory.ts              ← Settlement narrative/flavor text (@app/settlementStory)
bridge/                         ← ALL IPC invoke wrappers (one per Rust command), split
                                  world/query/campaign/goods.ts + types.ts + index.ts (@bridge)

Path aliases (tsconfig + vite): @state @canvas @ui @bridge @types @goods @app/*.
Import cross-cutting modules via alias, not deep-relative paths.

state/  (Zustand)
  worldStore.ts                 ← meta, rivers, lakes, settlements
  campaignStore.ts              ← campaign snapshot, houses, contracts, diagnostics, selection
  uiStore.ts                    ← tool, layer, workflow step, overlayVisibility, panel flags, bioParams
  goodsStore.ts                 ← goods spec being edited
  viewportStore.ts              ← camera state, tile invalidation
  paletteStore.ts               ← The renderer's colour tables, fetched once (§8.18)
  settingsStore.ts              ← app appearance: the overlay-line palette AND the map
                                  label typography (§8.11). Presets + localStorage +
                                  per-world persistence via a VERSIONED envelope
                                  `{v:2, lineColors, labels}` (the legacy flat colour
                                  map still hydrates); edited in ui/SettingsPanel.tsx

canvas/
  PixiApp.ts                    ← PixiJS 8 application init
  TileViewport.ts               ← Pan/zoom, screenToWorld, getVisibleTileRange. A
                                  wheel tick only moves a TARGET x/y/scale (`onWheel`);
                                  `update(dt)`, called every render-loop frame, eases
                                  the displayed values toward it (EU4/CK3-style smooth
                                  zoom glide, in place of the old jump-straight-to-
                                  target). `centerOn`/`fitWorld`/drag panning stay
                                  instantaneous — they set target = displayed in the
                                  same call, so only the wheel takes the eased path.
  TileManager.ts                ← LRU tile cache, base64→texture, sprite management.
                                  `draw()` never leaves a missing supertile blank —
                                  `drawFallback` stands in with whatever the SAME
                                  ground is already cached as at a different LOD
                                  (a coarser tile cropped+scaled, or a mosaic of finer
                                  ones), the "hold the old mip until the new one
                                  arrives" trick every tile map renderer uses. Before
                                  this, zooming or panning into not-yet-fetched
                                  territory flashed the canvas background through —
                                  worst right at a world-wrap seam, since those tiles
                                  are the least likely to already be cached at a
                                  freshly-picked LOD, which read as a "gap between the
                                  west and east halves" that grew the further out you
                                  zoomed.
  OverlayManager.ts             ← ALL vector overlays, drawn in CANVAS 2D — not Pixi
                                  (~4.6k lines: rivers, settlements, wind,
                                  trunks, routes, dynamic flow, regions). visibility[type] gates
                                  each. Also holds the two live appearance registries the
                                  Settings store drives: `lineColors` (overlay lines) and
                                  `labelStyles` (map label typography — see §8.11).
                                  §8.19 Slice 5 · a trade-good belt's
                                  FILL now comes from a FULL-RESOLUTION mask
                                  (`drawGoodBeltMasks`/`buildGoodMaskRender`), not the
                                  coarse ~8-cell blocks `GoodRegion` carries — see §8.19.
                                  `GoodRegion` still supplies each belt's LABEL, and its
                                  old coarse fill remains the fallback for a good whose
                                  mask hasn't arrived
  PaintOverlay.ts               ← Brush preview, paint stamps
  projection.ts                 ← lat/lon ↔ world-cell projection helpers
  goodArt.ts                    ← THE goods art — the ONLY goods treatment app-wide
                                  (pixel art, design handoff 2026-09-25, ported verbatim
                                  from `wf-pixel-goods.js`: the numbers ARE the design).
                                  One hand-placed 24×24 sprite per GOOD_DEFS name (all
                                  101) + a crate stencilled in the tint for custom goods.
                                  `drawPixelIcon` (integer-scaled, smoothing off at ≥2×
                                  device px; prefer 24/48/72/96 slots) · `goodSprite`
                                  (cached per name+tint; `.cv.toDataURL()` for SVG/DOM)
                                  · `PIXEL_GOODS` · `SPRITE_GRID`. The old Victorian card,
                                  parchment chip and enamel medallion are deleted
  goodIcons.ts                  ← map-space wrapper (`drawGoodIcon`, radius-based) over
                                  `drawPixelIcon` — region centroids, trade-lane and
                                  guild-city markers; carries the gemstones sublabel tint
  goodIconCache.ts               ← per (good, size, tint, DPR) blit cache for `GoodIcon`
  pixelize.ts                    ← the coarse-grid pixel treatment for PEOPLE/BUILDING
                                  art (`cultureDress.ts`, `marketSquareArt.ts`) + `shade`;
                                  goods no longer go through it
  buildingArt.ts                 ← the 15 `SPRITE_MAP` building types, procedural
                                  (`drawProcedural`), differentiated by architectural form
                                  rather than palette — the art redesign's building pass
  marketSquareArt.ts             ← the city MARKET SQUARE scene (`marketSquare`): stalls,
                                  a culturally-mixed crowd, price chips — pure canvas,
                                  driven by real hub state (see `ui/campaign/MarketSquare.tsx`)

ui/SettingsPanel.tsx            ← ⚙ Appearance modal, THREE tabs: Map plates (the
                                  atlas-plate picker, §8.17 — the default tab),
                                  Overlay lines (the line palette) and Map labels
                                  (typography theme + per-class face/colour, each row
                                  set in its OWN style so the list doubles as a live
                                  specimen sheet). See §8.11/§8.17.

ui/world/  — map & world
  MapCanvas.tsx                 ← PixiJS canvas, pointer events, painting, draws every overlay
  Toolbar.tsx                   ← Tools, PLATE picker (§8.17), layer selector, overlay
                                  toggles (RIGHT side). `layerGroups` classifies the 25
                                  render layers into SIX groups — Terrain · Ocean ·
                                  Atmosphere · Climate & Biomes · Settlement · Hazards.
                                  A STYLE picker (§8.22) appears only while
                                  "elevation"/"terrain" is active, reading the served
                                  `elevation_styles` list — never a hard-coded one
  mapThemes.ts                  ← MAP PLATES (§8.17): 11 named compositions of base
                                  layer + overlays + label typography + line palette.
                                  Display-only (rule 14); `MANAGED_OVERLAYS` is DERIVED
                                  from the plates, so a plate can only ever clear an
                                  overlay some plate uses
  StatusBar.tsx · WindowBar.tsx ← Bottom status / window chrome. StatusBar carries the
                                  HOVER READOUT (§8.18): lat/lon, cell and the ACTIVE
                                  LAYER's own value with its unit, which is the map key
                                  for every layer that has no exact legend
  hoverReadout.ts               ← active layer + CellInfo → the readout string. Values
                                  come from `get_cell_info`, never re-derived from a
                                  pixel colour
  InfoPanel.tsx                 ← Right-click cell inspector
  LayerLegend.tsx               ← THE MAP KEY (§8.18) — exact, renderer-sourced keys
                                  for elevation · terrain · temperature · sst ·
                                  precipitation · climate. Colours come from
                                  `get_render_palettes`, never a local copy. The
                                  elevation key swaps to the ACTIVE style's own served
                                  ramp (§8.22) when `uiStore.elevationStyle` is set
  ElevationHistogram.tsx        ← Elevation distribution chart
  (LatitudeControl.tsx removed — see ui/workflow/PlanetControls.tsx)
  climate.ts                    ← Köppen → human phrase helpers
  HydrologyPanel.tsx            ← Rivers/lakes + aquatic (fish assemblage, limnology).
                                  A river/lake detail is an ENCYCLOPEDIA ENTRY, not
                                  one long scroll: an always-pinned identity line
                                  (classification in words + the real-world
                                  counterpart, which used to sit in its own boxed
                                  strip halfway down) over a vital-statistics tile
                                  block, then leaves — Overview · Course · Life ·
                                  Network for a river, Overview · Life for a lake.
                                  The leaves exist because showing all eight
                                  sections at once effectively showed none: the
                                  reader scrolled past the long profile to reach the
                                  cities, and the FISH appeared TWICE on one screen
                                  (grouped under each reach AND again as their own
                                  list). Course keeps the by-reach grouping, Life is
                                  the flat source→mouth list — the duplication is
                                  removed, not hidden. A leaf that has no data is
                                  not rendered at all (a tributary has no reaches),
                                  rather than rendering empty
  ProvincePanel.tsx             ← 🗺 Provinces BROWSER (sort/filter/compare + generate).
                                  v2.0 · its "Goods produced" block reads REAL yield
                                  (`campaign_province_goods` + `_potential`) — actual/yr
                                  where the campaign produces, potential/yr where it does
                                  not, plus the ore-deposit count and mean grade. It was
                                  the only province view with no yield numbers at all,
                                  showing quality STARS alone off the frozen worldgen
                                  shortlist; that list is still the fallback on a world
                                  with no campaign running
  ProvinceInspector.tsx         ← 🏞 Dossier for ONE province, opened by CLICKING the map.
                                  v2.0 · rebuilt on the shared `@ui/kit` primitives +
                                  `chronicleTheme` tokens (Panel/PanelHeader/Section/
                                  Card/Tabs/Meter/Badge), the SAME system the Realms
                                  panel uses — the two now read as one designed app,
                                  which matters because a province and the realm holding
                                  it are constantly read together. No ad-hoc hexes left.
                                  FIVE TABS (Land · People · Holdings · Trade · Chronicle)
                                  over the layered survey plate, plus a YEAR SLIDER that
                                  scrubs `ProvinceLand.history` — a plate that differs
                                  between year 1 and year 500 is the visible proof the
                                  two halves are one simulation.
                                  A REALM BANNER sits under the identity line when a
                                  crown holds this province (`compute_states`, matched by
                                  `province_ids`): its own tint swatch, title, rank and
                                  cohesion, clicking through to the Realms panel. Same
                                  colour and rank vocabulary as that panel and the map
                                  tint, because all three read the one persisted `Realm`.
                                  Holdings keeps the ONE remaining control verb (the dues
                                  slider); the begin/abandon WORK buttons are gone — land
                                  improvement is autonomous now (§5.3) and shows on Land
                                  as a read-only "Under way" card naming the funder
                                  (a crown, or the seat city) and its real yearly cost.
                                  Phase 5 · the "writ of {holder_name}" line and granary
                                  note read correctly whether a CITY or a HOUSE holds the
                                  writ (`ProvinceLand.holder_house`).
                                  Selection is two-way with ProvincePanel via
                                  uiStore.selectedProvince; hit-test is a client-side
                                  raster lookup in OverlayManager.provinceAt (no IPC)
  ProvinceMiniMap.tsx           ← The province SURVEY PLATE (shared): six toggleable
                                  layers — relief · water · land use · tenure ·
                                  holdings · borders — plus `PlateToggles` and
                                  `soilWord`. The campaign holds land use and tenure as
                                  SHARES, not a spatial layout, so those two plates
                                  DITHER: each sampled cell takes a class from a stable
                                  per-cell hash against the cumulative shares. Truthful
                                  (proportions are exactly the model's), stable (a cell
                                  keeps its class between years, so the slider shows
                                  land CONVERTING rather than reshuffling), and it reads
                                  as a hatch rather than as false precision. Never
                                  invent a spatial layout the model does not hold.
                                  The GOODS plate is the exception that proves it
                                  (§8.19 Slice 6): a `GoodLocality`
                                  is NOT a share — it has a real cell and a real
                                  `radius_km` — so it draws as a real SQUARE at its
                                  real position, clipped to the province footprint,
                                  opacity carrying `grade` and hue from `GOOD_DEFS`. A
                                  MARINE locality draws dashed in the ADJACENT SEA and
                                  confers NO maritime territory (D4). A world with no
                                  localities (generated before Slice 3; D7 requires a
                                  re-run of Biological) falls back to the old symbolic
                                  markers, which is the honest reading of "one quality
                                  for the whole province, no position"
  provinceStory.ts              ← Shared province prose/format helpers (stars, border
                                  kinds, history) — used by BOTH province views
  ImportWorldDialog.tsx         ← Layered world import dialog
  SettlementSearch.tsx          ← Settlement name search/jump
  ErrorBoundary.tsx             ← React error boundary
  useFloatingWindow.ts          ← Floating/dockable window hook

ui/goods/  — goods
  GoodIcon.tsx                   ← React wrapper over `canvas/goodArt.ts`'s pixel sprite,
                                  backed at device resolution. Every panel that shows a
                                  good goes through it — no `GOOD_DEFS.emoji` glyphs remain in the panels
  GoodsEditor.tsx               ← Goods builder (distribution/value/bulk/perish + recipes)
  GoodsChainReview.tsx          ← Pre-generation planted-vs-manufactured review + recipe DAG
  GoodsBrowserPanel/GoodDetailPanel/GoodFlowPanel.tsx ← browser/detail/flow views
  GoodsCodexPanel.tsx           ← #35/#36/#37 Provenance + real-world History + Scarcity toggle
  GoodsMarketPanel.tsx          ← Market prices view
  TradeMatrixPanel.tsx          ← Worldgen trade-matrix region/flow inspector
  MerchantRoutePanel.tsx        ← Click-through merchant route inspector
  ItineraryPanel.tsx            ← #23 travel-time tool (origin/dest, per-mode days, route overlay)

ui/campaign/  — campaign / economy (+ helpers: chronicleTheme, cultureFigure, settlementArt)
  CampaignTopBar.tsx            ← Campaign era / advance controls
  HubPanel.tsx                  ← Settlement detail (Summary/Trade/Estates/People + City finances,
                                  Transit, year-grouped Chronicle). Its Trade ▸ Market
                                  sub-view is now `CityMarketView` (below) — three
                                  sections that used to sit apart (arrivals/market/
                                  departures, the standalone price grid, Exports/Imports
                                  + the chain ladder) collapsed into one book
  CityMarketView.tsx            ← THE CITY MARKET — VARIANT C, "the quay" (see
                                  `docs/TRADE_AND_MARKET_REVIEW.md` Part 3 and
                                  `YARDS_VESSELS_AND_DEPOTS_PLAN.md  ← ⭐ AGREED, only its two naval-stores goods built
                                    (`d3bf2da`). WHERE CARRYING CAPACITY COMES FROM.
                                    Extends MERCHANT_VESSELS_AND_INFORMATION_PLAN's
                                    stage 1 with the SUPPLY side it never had: hulls
                                    have to be built out of something, by someone,
                                    somewhere. Eight measured findings, of which two
                                    reframe everything: **ship PRICE is not the
                                    constraint, the BUILD RATE is** (`SHIP_COST` 7.0
                                    against a house holding 100-300k; `decide_fleets`
                                    runs monthly and buys AT MOST ONE hull, so the
                                    ceiling is 12/year however rich you are, while
                                    `FLEET_DECAY_CHANCE * fleet_total` reaches
                                    certainty at 83 — measured 2.4 hulls/house on the
                                    reference world, and the large world carries 5.5x
                                    the fleet with a LOWER house share); and **the
                                    house depot is a ONE-WAY SINK** — goods enter by
                                    monthly stocking and leave only by futures
                                    delivery, a war sack, or the house dying, with no
                                    ordinary sale verb at all, while `hub_stock`
                                    counts depots but `dispatch` reads the raw pool,
                                    so stored goods sit OFF the spot market and still
                                    depress the price (merchant speculation with half
                                    the mechanism missing). Six yard slices S0-S5
                                    (measure -> yard -> Vessel -> shares -> capacity
                                    binds -> charter, the last two dose-walked) and
                                    five depot slices W1-W5. Its central DESIGN
                                    DECISION is that a hull is built from a MATERIAL
                                    POOL, never a recipe: on `timber`+`iron` a recipe
                                    binds NOWHERE (both are `GOOD_UNLIMITED`), and on
                                    scarce naval stores it locks the tropics and the
                                    desert out of seafaring permanently — the inverse
                                    of the history, where Arabia was a great maritime
                                    culture BECAUSE it imported Malabar teak. Records
                                    that most of the warehouse system the maintainer
                                    described ALREADY EXISTS (offices already grant a
                                    depot per office city, with per-city capacity,
                                    5 tiers, upkeep, damage and war sacking), and that
                                    the missing third ownership class is the FONDACO —
                                    state-owned, foreigner-occupied, compulsory — which
                                    is what would make an office or a bailo a building
                                    the host city can close rather than a flag
MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` §2). The
                                  organising unit is the PARTNER CITY: arrivals group
                                  under the city they came from, departures under the
                                  city they go to, cargo nested and collapsible — the
                                  way a port book is actually kept, and the only
                                  variant where a city you hold an OFFICE in can read
                                  differently from one you touched once. Between them:
                                  MADE HERE (the city's own fields, then every estate
                                  and manufactory by name) over ON THE MARKET (good ·
                                  days of need held · price · WHO SUPPLIED IT, from
                                  `supply_accum`'s five seller classes — state the sim
                                  has kept since 4.4 that only the warehouse panel
                                  ever read). A row expands to the good's own book:
                                  bought-from / sold-to by city, the world's cheapest
                                  and dearest market by NAME, and the PERSISTED
                                  `TradeHist.prices` trend. The "by good" chip keeps
                                  the previous 2.0 book (bought at / sold at / the
                                  SPREAD / unusual-first sorting) — nothing was removed
                                  to make room. **No in-port / ready-to-sail strips**:
                                  a vessel is not a thing yet (three counters on
                                  `House`, no identity or location), so lanes report
                                  CARGOES, never vessels. SHARED between HubPanel's
                                  Trade tab and MarketsPanel, the same way
                                  ProvinceMiniMap is shared between the two province
                                  views
  MarketsPanel.tsx              ← The floating ⚖ Markets window: the same
                                  `CityMarketView` behind its OWN city picker
                                  (`campaign_market_cities` — a live list, so towns
                                  founded during the campaign are reachable, which the
                                  frozen worldgen snapshot cannot do). Seeded from
                                  `selectedHub` on open, then never re-bound to it, so
                                  two markets can be read side by side
  CityView.tsx · SettlementScene.tsx ← Isometric city view + scene, buildings drawn via
                                  `canvas/buildingArt.ts`'s 15-type procedural set
  MarketSquare.tsx               ← The market-square scene (`canvas/marketSquareArt.ts`)
                                  mounted at the head of `CityMarketView` — stall keepers
                                  from `detail.culture`/`.minorities`, wares ranked by
                                  value on hand, chip prices from `price/base_value`, no
                                  new IPC
  HousesPanel.tsx (2026-09-25)   ← NOW TWO BROWSE WINDOWS from one component —
                                  `<HousesPanel />` (⚜️ Merchant Houses) and
                                  `<HousesPanel companies />` (🏛 Merchant Companies,
                                  `uiStore.showCompanies`) — with search, sort and
                                  40-card paging; the S12a bump chart and pulse ticker
                                  are no longer rendered (user: "a mess, no chart").
                                  The dossier is APP-WIDE: `HouseDossierHost` renders
                                  `HouseDetail` for `uiStore.dossierHouse`, so any view
                                  (Government tab charters, a feud…) can open a house,
                                  plus a 🧭 "Trade lanes by good" window that narrows the
                                  map web (`uiStore.houseLaneGood` →
                                  `OverlayManager.setHouseLaneGood`): null = the whole
                                  web in red, a good = only its lanes, in its colour.
                                  The history below describes the older layout.
  HousesPanel/FeudsAlliancesPanel/DynastiesPanel/GuildsPanel.tsx ← Merchant houses,
                                  feuds, dynasties, crafts — FOUR windows now, not two
                                  (HOUSES_GUILDS_AND_MARKET_PLAN.md S10, §5.6). `HousesPanel.tsx`
                                  (338 lines, was 1,455) is BROWSE-ONLY: the tier-grouped
                                  list (Phase 1.1, Tier 3/4 collapsed by default), the
                                  ⚖ Compare launcher (`HouseCompareWindow`, `HouseCompare.tsx`
                                  — a search-bar-driven two-house side-by-side: ruler
                                  figures, every stat, a minimal `OperationsMap` plotting
                                  both houses' seat/offices/controlled settlements), and a
                                  "🏛 Companies" FILTER CHIP in place of the old "guilds"
                                  TAB — `House.is_guild` firms are firms, not a different
                                  kind of thing, and keeping them in a tab beside `CraftGuild`
                                  ("Guilds & Crafts") was what made the naming collision
                                  visible to users in the first place. An "⚔ Feuds" button
                                  opens the new `FeudsAlliancesPanel.tsx` — the world's
                                  quarrels as their OWN window (a feud belongs to two
                                  houses, not one; it was never a house's tab), wrapping
                                  `FeudsView` with no house focus. `GuildsPanel.tsx`
                                  relabelled "🔨 Crafts & Guilds" (was "🏛 Guilds & Crafts"),
                                  now a production DASHBOARD: four stat tiles,
                                  a "what the workshops make" ranked bar per
                                  craft (click filters), Guilds / By-craft tabs,
                                  search, paging (60 rows — worlds reach 400
                                  guilds), a grade word per quality. A guild card
                                  expands into a CRAFT FLOW diagram — raws
                                  arriving → workshop → where the good ships —
                                  reading `campaign_guild_atlas` (S9, cargo on
                                  the road NOW, sorted client-side) keyed by `idx`.
                                  Split helpers (`TIER_META`/`tierOf`/`dull`/`goodIcon`,
                                  used by both the browser and the dossier) live in
                                  `houseShared.ts` rather than being duplicated or
                                  cross-imported, which would make a HousesPanel ↔
                                  HouseDossier import cycle. S12a added a TOP BAND
                                  above the list — an SVG bump chart of the top
                                  ~12 houses' wealth rank over ~50 years
                                  (`campaign_house_bump_chart`, click a line to
                                  open that house), two real sparkline gauges
                                  (families/top-10% share, from the existing
                                  `campaign_get_inequality`) and two plain-total
                                  gauges (founded/fallen — no per-year series
                                  exists for either, so shown honestly rather
                                  than invented, Q18) — and a bottom PULSE
                                  ticker reusing `campaign_get_journal(-1,-1)`
                                  (`NewsFeedPanel`'s own world feed) filtered to
                                  house-ish kinds.
  HouseDossier.tsx              ← The big per-house window — `HouseDetail` (moved here
                                  verbatim from HousesPanel.tsx in the S10 split) plus its
                                  ten subtabs, alongside this file's original two views
                                  (`HouseStandingView`/`FeudsView`). Opens as a BIG FLOATING
                                  WINDOW (~2.5x the old size, still draggable) on a portrait
                                  — `cultureFigureSVG` in the seat culture's kit and the
                                  head's own sex, POSE/ACCESSORY variation axes on top of
                                  build/skin-tone jitter so two heads read as two different
                                  people, a coloured frame standing in for a garment
                                  recolour, a `CoatOfArms` badge at the shoulder, occasion
                                  set by tier (Phase 1.2) — and its subtabs are
                                  CHRONICLE-FIRST (Phase 1.4, the default tab):
                                  the Phase 0.4 succession line inline, then the
                                  year-grouped event log (`ChronicleTab`), before
                                  Summary (now tags family-run holdings, Phase 2.2)/
                                  👪 Kin (the roster, Phase 2.1/2.3/2.6)/
                                  🌳 Lineage (`campaign_get_house_lineage` — the
                                  ancestor chain read from `origin_house`/`origin_kind`
                                  back to the founding, plus this house's own offshoots;
                                  each hop tags WHY: guild-seeded, a branch, a Partible
                                  division, or a Departure schism — the "where did this
                                  house come from / why did it split" record, clickable
                                  to jump the dossier to any ancestor or offshoot)/
                                  🎯 Ambitions (active/history goals from 7 kinds —
                                  chosen yearly by archetype/character bias, checked
                                  yearly — READ-ONLY tracking, not yet wired to bias
                                  any decision's weights, Phase 3.1)/
                                  ⚠ Crisis (only shown when a house has an open struggle
                                  or a past-risings record — two named factions in their
                                  own heraldic tincture, a round-by-round log with each
                                  side's MOTIVE line (`head_motive`/`plot_leader_motive`,
                                  derived from loyalty/role/posted-hub/vice — why this
                                  particular kinsman backs the head or leads the plot),
                                  the heir's recorded choice, then the permanent "past
                                  risings" list; observation only, Phase 3.2-3.6)/
                                  🧭 Expeditions (this house's live ventures, click a
                                  row to highlight its destination province, Phase 1.3)/
                                  🗺 Atlas (S11, text form — partner cities by
                                  volume, the goods portfolio with bought-at/
                                  sold-at cities, a seasonal lane-ease bar
                                  chart; reads `campaign_house_atlas`, S8's
                                  own query)/
                                  ⚖ Standing/⚔ Feuds/🏦 Bank/📒 Accountant.
                                  `HouseStandingView` (five stability gauges — solvency
                                  COUNTDOWN, liquidity runway, concentration exposure,
                                  succession, cohesion — plus liabilities) and `FeudsView`
                                  (cause · temperature · stage · ending, with each feud's
                                  episode log — `house < 0` shows every quarrel in the
                                  world, which is what `FeudsAlliancesPanel.tsx` calls it
                                  with) are unchanged from before the split. Pips + a
                                  PHRASE, never a raw 0..1; a healthy gauge stays quiet so
                                  the warning colour still means something. S12c adds a
                                  RADAR (`GaugeRadar`) to Standing — this house's five
                                  gauges against the world's tier-1 MEDIAN
                                  (`campaign_tier1_gauge_medians`) — and a stepped
                                  TEMPERATURE LINE (`FeudTemperatureLine`) to each Feuds
                                  card, from that feud's own recorded flare log. The
                                  Accountant's `LedgerView` gains a WATERFALL
                                  (`WaterfallChart`) cascading from 0 through the year's
                                  top trade goods and every expense line to the real NET.
                                  S12b adds a `TimelinePlate` above the subtabs — every
                                  head a segment, milestones marked, feuds bracketed,
                                  a wealth curve threading through, scrubbable by drag
                                  (does not rewrite the tabs below to a scrubbed year —
                                  no per-tab historical state exists for that, Q20).
  BankPanel/MoneyFinancePanel.tsx ← Bank T-accounts, currencies/mints/monetary chronicle
  SpeculationPanel.tsx          ← DLC 3: Speculation why-chain / Poleis (treasury/tariff/mint/coin)
  CoinCreditPanel.tsx           ← Currencies / Banks / Wars / Crashes / Schematics tabs
  WarehousesPanel.tsx           ← House warehouses / reserves
  FuturesPanel/FuturesLanePanel.tsx ← Futures contracts + lanes
  EconomyDashboardPanel.tsx     ← #30/#29 Price Index (CPI) + Inequality (Gini/mobility) tabs
  CityRankingPanel.tsx          ← Richest/busiest cities
  FlowsView.tsx                 ← Realized trade at a settlement (post-campaign),
                                  rebuilt on `@ui/kit` + `chronicleTheme` so it
                                  matches `CityMarketView` (the Market half of the
                                  same tab). Four things from TRADE_AND_MARKET_
                                  REVIEW.md Part 3's own discipline, which Flows
                                  had none of: a BALANCE header naming the city's
                                  trading position (net exporter / import-dependent
                                  / balanced entrepôt) over in/out/net; rows sorted
                                  by what is UNUSUAL (the in-out IMBALANCE, weighted
                                  by volume) rather than by size, since a large
                                  BALANCED staple always tops a volume ordering and
                                  is rarely the row worth reading; a VERDICT PHRASE
                                  per good instead of a raw number (`collapsed` /
                                  `import-dependent` / `we export`), with an
                                  unremarkable good left deliberately QUIET so a
                                  coloured badge still means something; and a
                                  CONCENTRATION warning when one partner carries a
                                  large share of all trade — a share is on screen
                                  today, but the real question is whether losing
                                  that partner would hurt. The two-tone in/out bar
                                  replaces the single-colour volume bar, so a good's
                                  DIRECTION reads without expanding its row
  ColonialPanel.tsx             ← Colonies / colony gates / lifelines. Selecting a settlement
                                  colony or house outpost opens `ColonyWindow.tsx` — the 1180px
                                  colonial city view (design handoff Turn 5, 5a/5b): a growth
                                  strip (one seeded city drawn at the four stages via
                                  `cityArt.ts`), the stage ladder with the real `colony_pass` /
                                  `maybe_graduate_outpost` gate, backers, food reserve, convoys
                                  and the founder's roster
  windowKit.tsx                 ← SHARED shell for the 1180px design-handoff windows (CityView,
                                  ColonyWindow, CoinBankWindow, HouseWindow): the Chronicle-dark
                                  `K` tokens, `fk`/`fc`, Card/Bar/Stack/Spark/MarkChart/… ,
                                  `WindowFrame`, `IsoThumb`, `cityScene()` and the culture-kit
                                  cache
  CoinBankWindow.tsx            ← 5c · one coin's window (uiStore.coinWindow, opened from a Mint
                                  card or the Bank panel): fineness/trust/value biography with
                                  event marks, cities settling in it, its chronicle, the bank of
                                  issue's balance sheet, stakes and loan book
  HouseWindow.tsx               ← 5d/5e · the house/guild overview plate — the dossier's
                                  "🏛 Overview" tab (widens the window): seat iso, head, the five
                                  gauges, heads, accounts, fleet, estates, charters, reach
  ImmigrationPanel.tsx          ← Route-bound migration corridors
  SatelliteConstructionPanel.tsx← Satellite (suburb) construction projects
  PlaguePanel.tsx               ← Historical epidemics
  PeoplesPanel.tsx              ← Cultures / peoples / pops
  FiguresPanel/CultureFigures/CultureDonut.tsx ← Key figures/notables + culture charts.
                                  FiguresPanel is a PORTRAIT GALLERY: each figure's bust
                                  (`cultureDress.drawBust` in the home city's
                                  `kitForCulture`, hair/beard/dyes varied per NAME so two
                                  of one people differ), role ring, house arms, career
                                  years and `FigureBrief.legacy` — the one real capped
                                  effect `raise_notable_figures` applied (fleet, prestige,
                                  unrest, craft quality), mirrored from its chronicle
                                  text, never invented, plus `FigureBrief.influence` — what a
                                  LIVING figure keeps doing every year (`living_figures_pass`,
                                  `houses.rs`): an admiral shields their house's fleet from
                                  piracy in `run_piracy`, a demagogue nudges unrest up (and
                                  chronicles a one-time "rallied" event past
                                  `DEMAGOGUE_RALLY_AT`), a master craftsman raises their
                                  craft's quality/tradition, a banker/explorer adds to their
                                  house's prestige — every effect capped (rule 18) and shown
                                  as a "Now" line. A role donut on top. A click-to-expand
                                  "life ▾" panel (read_people.rs::campaign_get_figures) adds
                                  `bio` (birthplace · house or culture · for an Explorer with
                                  a real linked `Expedition`, the actual destination and its
                                  outcome — every clause gated on real data, omitted rather
                                  than invented when absent), `merchant_goods` (straight off
                                  the linked `House.spec`, "" if unaffiliated), `life_events`
                                  (`life_events_for`, read_people.rs — a chronological, capped-
                                  at-6 log built ENTIRELY from real `sim.journal` entries at the
                                  figure's own city during their lifetime, filtered to
                                  `role_journal_kinds(role)` — an admiral's log is piracy/war/
                                  voyages, a demagogue's is riots/unrest/starvation — plus a
                                  small COMMON set everyone notices (plague, war, crashes,
                                  revolt); a long life is SPREAD across via a stride sample
                                  rather than truncated to its opening years) and `thought`
                                  (`reactive_thought`, read_people.rs — REACTS to the figure's
                                  own city's present `war_with`/`starving`/`lack_basic`/
                                  `society.unrest` when one is genuinely unusual, else falls
                                  back to the deterministic per-role quote pool picked by
                                  `fnv1a32(name) % 4` so an ordinary figure's line still never
                                  changes between reads — flavour built from real fields, never
                                  fabricated). Also exports `CityNotables`, shown on HubPanel's
                                  Life tab. NOTE: `cultureFigure.ts` below no longer exists —
                                  portraits everywhere now go through `cultureDress.ts`
  NotablesPanel.tsx             ← 02_PEOPLE.md (Living World row 02) — the
                                  40-cap notable roster + the Hall of the
                                  Dead, plain-list (not yet a portrait
                                  gallery — see §5.8)
  LandmarksPanel.tsx            ← Notable landmarks
  AtlasPanel.tsx                ← Atlas 2.0 (eras / world frame)
  (NewsFeedPanel.tsx removed — living_world/01_FEEDS_AND_PRUNING.md; every other
   chronicle reader still calls the same `campaign_get_journal`)
  ChroniclePanel.tsx            ← World ledger — reading matter (left rail). Also the
                                  two ways to begin again on this world — ➕ New
                                  campaign (archives the running one into the library,
                                  no dialog) vs ↺ Start over (confirm-gated, names the
                                  year discarded) — and the card shown when a world
                                  carries NO economy, which says what is missing and
                                  that opening the matching `.campaign` recovers it
  CampaignLibraryPanel.tsx      ← 📚 Campaigns — the library folder listed in-app:
                                  every save with its year, world, cities, houses and
                                  save date; Open / Delete / Save-here / Open folder /
                                  Change folder. A save whose world does not match the
                                  one open is shown but marked, never hidden
  YearChronicle.tsx             ← SHARED year-grouped expandable chronicle
  cultureFigure.ts · chronicleTheme.ts · settlementArt.ts ← helpers/themes/art.
                                  `cultureFigure.ts` draws an INDIVIDUAL (a `sex` axis,
                                  per-person seed) — still the renderer for House
                                  portraits/`HouseCompare`, deliberately NOT replaced by
                                  the dress-plate art below, which has no sex axis
  cultureDress.ts                ← the art redesign's per-PEOPLE bust + costume figure
                                  (18 preset kits index-aligned to `cultureFigure.ts`
                                  KITS, plus `deriveKit`/`creoleKit`/`resolveKit`/
                                  `REGISTERS`/`kitForCulture`) — renders `PeoplesPanel`,
                                  where there is no one head to draw as a person

ui/heraldry/  — heraldry
  CoatOfArms.tsx                ← Deterministic house heraldry (houseColor + shield SVG)
  CoinIcon.tsx                  ← Heraldic minted coin (coat of arms on gold disc + value tint)

ui/workflow/
  WorkflowPanel.tsx             ← Generation wizard + "Run All" buttons
  StepWorldCharacteristics.tsx  ← Step 0: THE single home for every generation
                                  setting, in collapsible groups — 🪐 Planet
                                  (rotation/retrograde · sunlight · greenhouse ·
                                  eccentricity) · 🌍 Axis & Seasons (axial tilt ·
                                  calendar length) · 💧 Water & Air (dryness) ·
                                  🧭 Latitude Frame (equator · expansion · line
                                  proportion, moved here from the right Toolbar).
                                  Displayed AFTER Landmass (you judge these against
                                  visible continents; nothing before step 3 reads
                                  them). Settings-only, always advanceable.
  PlanetControls.tsx            ← Shared `PlanetSlider` + `LatitudeFrame` (replaces
                                  the deleted ui/world/LatitudeControl.tsx)
  ZonalStrip.tsx                ← Tier-1 preview: SVG latitude strip (temperature vs
                                  Earth · seasonal envelope · belt markers · crop band)
  ClimateMixPreview.tsx         ← Tier-2 preview: Köppen thumbnail + A/B/C/D/E mix
  planetArchetypes.ts           ← 12 world archetypes (mild↔strong spans + dial) +
                                  5 map-frame presets + the Earth-diff readout
  Step*.tsx                     ← Landmass, Elevation, OceanAtmo, Climate, Rivers,
                                  SoilResources, Settlements, Biological, Economy,
                                  Political, Campaign, Toponyms (#26, gated)
```

---

## 8. Reference — Science & Systems

### 8.1 Overlays (query commands, NOT stored in tiles)
- `compute_shark_zones` / `compute_shipworm_zones` → highest-risk hazard cell-masks
  (Toolbar → Biological: 🦈 / 🪱). Also `compute_reef_zones`, `compute_storm_zones`,
  `compute_monsoon_zones`.
- `compute_good_regions` → per-good belt cell-masks. **The FILL it returns is no
  longer drawn** (§8.19) — it now supplies only the LABEL: the medallion centroid,
  the gemstone `sublabel`, and the coarse fallback for a good whose mask hasn't
  arrived. One toggle per good under Trade Goods.
- `compute_good_belt_masks(goods)` → the FULL-RESOLUTION two-layer belt mask per
  named good (§8.19). Takes a list, not the whole world, because a mask is a real
  payload and only the toggled goods are ever drawn.
- `compute_trade_routes(settlements, rivers, reach, max_crossing)` → least-cost routes
  over the shared coarse cost grid (passes / rivers / coast-hugging); reach limits
  open-water crossings.
- `compute_trade_matrix(...)` → settlement-cluster regions, per-good prod/demand/net +
  per-good `flows` + **routed & bundled `trunks`** (edge width ∝ volume). Sea-
  impassable pairs (under the reach) get no flow.
- `compute_climate_bands` → the circulation belts + the **ITCZ at BOTH seasonal
  extremes** (`itcz_july` / `itcz_january`, per column, from the same helpers the
  seasonal wind is built from). The overlay draws July solid, January dashed, and
  hatches the migration band between them at low opacity — that band is the land
  which changes circulation regime between seasons, i.e. the monsoon belt.
- `compute_political(...)` → settlements re-ranked by **trade power** (0.45·habitability
  + 0.30·route-centrality + 0.25·good-monopoly); influence discs sized by power (👑).

Trade routes/flows come from the **Biological-Trade step** (gated on step 8); the
political layer from the **Political step** (9). Trade reach + max-crossing set in
`StepBiological` (`uiStore.bioParams`).

### 8.2 Key formulas & the climate physics
```
Temperature   earth_base_curve(|lat|):  30-0.333|lat| (<30°) | 20-0.5(|lat|-30) (30-60°)
                                        | 5-1.15(|lat|-60) (60-90°)          [Earth-calibrated]
              T = earth_base_curve + EBM_anomaly(lat) - 5.0·(elev·8848)/1000
                  + current influence (±3°C, decaying inland) + coastal damping
              Coastal damping: only if ocean_dist<0.1 AND upwind_is_open_ocean → 45% toward 15°C
Energy budget d/dx[D(1-x²)dT/dx] = A + B·T - I(x)·a(x,T),  x = sinφ     (North–Budyko)
              OLR_A=201, OLR_B=2.09, D = 0.60·rotation^-0.7, ice-albedo step at -8°C
              Applied as an ANOMALY vs Earth → exactly zero at Earth params
Circulation   hadley_edge = 30·Ω^-0.65 · warmth^0.10   polar_front = 60·Ω^-0.40   (30/60 at Earth)
Precipitation moisture advected from sea, e-folding EFOLD_MID_KM=1700 / EFOLD_TROP_KM=1300
              (km-based → grid-resolution independent), MOISTURE_FLOOR=0.09;
              + ITCZ / orographic (MOUNTAIN_THRESHOLD 0.19, windward 220km, shadow 500km)
              / frontal / monsoon / jet entrance-dry + exit-wet;  cold-coast -35%, upwelling -60%
Fertility     F = soil·0.30 + precip·0.20 + temp·0.15 + river_prox·0.20 + coast·0.10 + volcanic·0.05
Fisheries     upwelling (shelf + cold current + equatorward flow) + river-mouth proximity
Habitability  H = climate·0.40 + fertility·0.20 + water·0.20 + terrain·0.10 + trade·0.10
```

**The monsoon is a MIGRATING ITCZ, not a land–sea breeze.** `belt_wind_shifted`
displaces the whole circulation toward the summer hemisphere; the meridional
direction comes from which side of the *displaced* ITCZ a cell sits on, while the
Coriolis handedness comes from its **true** latitude. Keeping those two conventions
separate is the entire mechanism — it is what makes cross-equatorial flow recurve
into a real southwesterly. Land–sea contrast enters only as `MONSOON_LAND_PULL`,
selecting the LONGITUDE the convergence zone reaches furthest poleward (Chao & Chen
2001; Gadgil 2003; Geen et al. 2020). At `shift = 0` it is bit-identical to the
annual-mean field, so the ocean-current model is untouched. Three rules: the
migration must stay **tapered to the tropics** (a uniform shift reverses the
Southern Ocean westerlies); `monsoon_onshore` must stay **wind-aware** (a purely
geometric ray lets subtropical deserts switch off their own subsidence sink — the B
row falls 68.1% → 61.3%); and `earth_monsoon_wind_reverses` must keep passing.

**Ocean currents** are a gyre-aware relaxation, not a solve: the *interior* comes from
the Sverdrup relation (curl of belt wind stress on a β-plane — sign and latitude
structure EMERGE), while boundary speeds are prescribed constants
(`SPEED_BOUNDARY_WEST` 2.2 vs `..._EAST` 0.55 = western intensification), then 20
deflection passes + bathymetry steering. The field is **not divergence-free** and
currents are **annual-mean only** even though the winds have two seasons.

**Known fidelity gaps** (measured, with fixes planned — see `docs/FIX_PLAN.md`
A7–A14, which include three REVERTED attempts kept as negative results):
moisture has no conserved budget and no evapotranspiration recycling, so continental
interiors and the monsoon subtropics come out far too dry (`C→B` confusion 39%);
maritime coasts under-damp, so the high-mid latitudes read too cold (`D→E` 40%);
seasonality is only two states (July/January), so Köppen's `s`/`w`/`f` third letter
comes from hand-coded detectors rather than from monthly extremes.

### 8.3 Salinity, sharks, shipworm
- **Salinity** (`ocean.rs::compute_salinity`, before currents): `S = 35 +
  (evaporation − ocean_precip)·5.5` − coastal runoff + enclosed-sea concentration;
  stored u8 over 28-42 PSU. `apply_thermohaline` couples density → current speed ±25%
  and extends the warm conveyor. `advect_salinity_and_recouple` carries salty
  subtropical water poleward. **Re-run Ocean & Atmosphere THEN Climate after changes.**
- **Shark risk** (`biological.rs`): `warmth(T)·shallow·coast·prey(fishery) +
  brackish·coast`; warmth 0 ≤10°C → 1 ≥23°C.
- **Shipworm risk** (`biological.rs`): `warmth(T 13→24)·shallow·coast·brackish`; a
  persisted u8 column serialized AFTER goods.

### 8.4 Trade goods & production chains
21 belts (`compute_trade_goods`). `good_score` = climate(Köppen) × temp/precip bands
× elevation × fertility × coast × (fishery/salinity for marine). Distribution by
`GOOD_UNLIMITED[g]`:
- **UNLIMITED** (stockfish, furs, timber, **hardwoods**, salt, whaling, wheat, iron,
  and the two naval stores **pitch**/**hemp**) — every suitable cell produces.
  `hardwoods` joined this list because it and `timber` are ONE ROLE — the wood a hull
  is built from — split across climates, and they had opposite distributions: every
  suitable boreal/temperate cell grew timber while the entire tropics shared a SINGLE
  seeded hardwood homeland. That left a tropical or desert-coast city with no local
  hull wood at all, which would make shipbuilding structurally impossible for exactly
  the maritime cultures that were best at it (measured 311 cells / 11 settlements →
  540 / 17). See `docs/YARDS_VESSELS_AND_DEPOTS_PLAN.md` D1.
- **SEEDED** (rest) — `localize_good` picks ONE weighted seed + flood-fills one
  homeland, with ~4% map-width island-jump. Land goods stop at mountains ≥3000 m
  (`MOUNTAIN_NORM`≈0.339); marine goods stop where the score envelope drops.
- **GEMSTONES / metals / stone (`Deposits`)** — placed by REAL ORE GEOLOGY
  (`step8_biological_goods/deposits.rs`, §8.16), not by an elevation floor.
- **MANUFACTURED** (`Distribution::Manufactured`) — finished goods made in cities from
  a recipe (`GoodSpec.inputs`), no per-cell belt.

**Chains + transport** (both worldgen `market.rs` and campaign `tick/` read ONE set
of `GoodSpec` fields, all serde-defaulted):
- **Transport:** `bulk` (freight mult) + `perishable` (extra freight/day).
  `freight_of = per_day·days·bulk + perishable·days`. Heavy/perishable stay regional.
- **Chains:** `inputs: Vec<RecipeInput{good, qty}>` + `labor`. Shared
  `manufacture.rs::apply_manufacturing` topo-orders manufactured goods (cycles/missing
  disabled with a warning), turns input stock into output scaled by labor (∝ pop) so
  manufacture concentrates in big cities.
- **Builder UI + always-on review:** bulk/perish/recipe/labor edited in `GoodsEditor`;
  generation ALWAYS routes through `GoodsChainReview` (planted-vs-manufactured split +
  SVG recipe DAG) before `sim_biological` runs. Shipped chains: cloth, metalware,
  refined_sugar, citrus_liqueur.

### 8.5 Market economy (`market.rs::solve`, Part III)
Pure & deterministic: per-hub stocks, needs ladder (basic/comfort/luxury) with
**category substitution** (15 categories), local price `base_value·(need/stock)^0.6`
in the **grain-equivalent numeraire** (wheat=1), arbitrage on live prices with freight
and import caps at delivered-cost parity → decaying spatial price gradients, no
terminal cap. `compute_economy` feeds travel-days and emits `EconHub.market`. Hub
`wealth` = normalized(grain + 1.5·trade + 0.25·centrality). 45 builtins + declarative
customs incl. 4 Manufactured chain goods. `backfill_market_fields` fills
category/tier/base_value/bulk/perishable on pre-market saves.

**A SELLER RANKS ITS BUYERS BY ARBITRAGE GAP, NEVER BY INDEX.** Each arbitrage
shipment takes `surplus * 0.5` and re-reads `surplus` from the seller's stock
every time, so the k-th destination a seller is offered receives on the order of
**2^-k** of its output — a geometric cascade down the iteration order, not the
gentle `break` the code reads as. Stocks are seeded once from production at the
top of `solve` and never replenished, so the cascade repeats identically every
round. Walking `b` in plain index order therefore made that cascade follow the
SETTLEMENT RANK ORDER, because `compute_economy` sorts its nodes by score
(strongest first) and market hub index IS that rank: every supplier served the
great cities first and had nothing measurable left for a small town, which read
**0 throughput however close or hungry it was**, and strictly worse the more
settlements a world had. Destinations are ranked by the gap they offer now,
which is rank-BLIND — an unsupplied small town sits at `PRICE_CEIL_MULT` exactly
as a large one does, and among ceiling-priced buyers freight decides, so the
NEAREST buyer wins. Ties break on index so `converges_and_stays_finite`'s
determinism assertion still holds. Gate:
`a_near_buyer_is_served_before_far_ones_whatever_its_index`.
**The halving itself is left alone and is a named open item**: "half-steps
toward parity per round" is applied per (seller, buyer) PAIR, so it compounds
ACROSS destinations within one round rather than once per round as the comment
intends. Ordering was the targeted fix; changing the step size is a real
economic change needing its own dose walk.

**`compute_economy` is where two shipped Stage-A/B fixes had never been ported**
(`ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md`), which is why the drawn map and the
ledger could disagree about the same town:
- **B3, the lesser-town fall-through.** A settlement past `major_n` got exactly
  ONE candidate link — its single nearest major hub. A Dijkstra miss, a
  `path_allowed` rejection or an over-ceiling pair left it with ZERO edges and
  no trade at all, while `compute_trade_routes` (`routing.rs`) had already
  fixed this with a ranked `MINOR_FALLBACK_K` shortlist. `compute_economy`
  now collects the same 5-candidate shortlist, batches it into the SAME
  `coarse_dijkstra_batch` (one Dijkstra per coarse SOURCE, so five candidates
  cost what one did) and takes the first accepted — topology still mirrors the
  drawn routes, it just no longer gives up after one failure.
- **B2, the link ceiling.** `max_link2` was a magic `grid_w * 0.30` =
  **12,022 km** on the default grid, wider than any ocean on Earth, so two
  continents always wired into one trade graph and the `comp[a] == comp[b]`
  guard below it could never fire. It reads `TRADE_COMPONENT_HORIZON_KM`
  (3,000 km) now — the same constant `compute_routed_components` already
  applies on the campaign side, so the worldgen economy stops promising lanes
  the campaign it seeds would refuse, and **genuinely isolated continents can
  form and trade internally**.
Note `EconHub.partners` is `centrality[hh]`, i.e. EDGE COUNT, not distinct trade
partners — a lesser town reads "1" structurally, by topology, not measurement.

**TWO WAYS A CONNECTIVITY GATE HERE SILENTLY PASSES**, both of which the first
cut of these two gates did (§8.24a3's rule — always verify a new gate fails on
the unfixed code — caught them):
- **`distance_to_ocean` defaults to 0.0 and `node_sea` is `d < 0.06`.** A fixture
  that does not set it marks EVERY settlement a sea port, and the maritime-bypass
  block then wires each coastal MAJOR to its five nearest coastal hubs — lesser
  towns included. The town gets an edge by another route entirely and the gate
  passes on the broken code. Hold every node inland unless the test is about
  ports.
- **A major links to its 4 nearest among ALL nodes**, not just other majors, so a
  lesser town sitting alone beside a hub is picked up incidentally however the
  lesser-town path behaves. Pack enough majors around the intended fallback hub
  that none of them has the town in its top 4.

The per-good lane cap (220) trims the display snapshot, but the per-hub
exports/imports/receives totals accumulate inside that same loop, so a trimmed
lane reads as no trade. Lanes are ranked among the lanes arriving at the SAME
(good, destination) before the cap applies, so it can only ever trim a hub's
second and later sources — every buyer keeps one supplier before anyone keeps
two.

**THE OPEN-WATER CROSSING IS A WORLD RULE, NOT A PREFERENCE**
(`MAX_OPEN_SEA_CROSSING_KM` = 800 km, `query_commands/mod.rs`). Pre-modern trade
is coastal navigation punctuated by SHORT blue-water hops — Sicily to Cape Bon
~150 km, Greece to Italy ~150, Norway to Shetland ~300, and the longest routine
classical crossing, Crete to Egypt, ~500 km, written about precisely because it
was exceptional. Nothing routine crosses an ocean. The limit used to be a user
preference ALONE (`max_crossing`, a fraction of map width, shipped at 0.12 =
**4,809 km**), so an ocean traversal was the default rather than an opt-in.

Three things make it a rule rather than a setting:
- **The clamp lives in `path_allowed`**, the one chokepoint every route, flow,
  economy, political and component query already funnels through. A per-command
  clamp across the nine entry points that take `max_crossing` is a rule with
  nine ways to be missed. A caller may only ever ask for something SHORTER.
- **It binds at EVERY reach except 2.** Reach 0 used to be an unconditional
  `true` (the UI called it "Global — cross any ocean"), so one caller passing
  reach 0 put every trans-oceanic lane straight back — and `MapCanvas.tsx`'s
  `CAMPAIGN_ROUTE_REACH` is exactly such a caller. Reach 2 stays land-only.
- **`compute_routed_components` uses the SAME constant.** It passed
  `TRADE_COMPONENT_HORIZON_KM` (3,000 km), which said two landmasses shared a
  market across water no route could actually cross — the B2 inconsistency one
  layer up. The two constants are now distinct things and say so: this one is an
  unbroken open-water RUN, `TRADE_COMPONENT_HORIZON_KM` is a straight-line lane
  LENGTH bounding `compute_economy`'s candidate links.

**It removes oceans, not the sea.** Shelf and coastal water reset the run (see
`is_open_sea`), so coast-hugging is unlimited and an island chain is crossed one
hop at a time — which is how a pre-modern network reaches a long way without
ever losing sight of land. A rule that forbade all sea would just be reach 2
under another name, and the gate asserts BOTH halves for that reason.
Gate: `no_caller_can_ask_its_way_across_an_ocean` (0.12 and 1.0 both refused at
reach 1, reach 0 refused too, a short hop still allowed), plus the two component
gates re-pointed at this constant. The frontend default is ~400 km and its
slider is stated in KM over a 50-800 km range — "% of map width" is not a
distance anyone can reason about, and reading it as one is how 12% came to mean
an ocean. `SEA_CAP_KM` in `StepBiological.tsx` bounds the SLIDER only; the
backend clamps regardless, so drift between them stops the slider short rather
than breaking the rule.

**THE CROSSING RULE IS PART OF THE SEARCH, NEVER A VERDICT ON THE FINISHED PATH.**
`coarse_dijkstra` returns the globally CHEAPEST path, and every caller used to run
`path_allowed` over it as a POST-FILTER. So a lane whose cheapest line cut across a
gulf had the WHOLE route discarded and fell back to a dashed straight line — while
an ordinary coast-hugging route between the same two ports existed the entire time
and was never searched for. That is the "trade routes are straight lines instead of
following the trade routes" report, and tightening `MAX_OPEN_SEA_CROSSING_KM` made
it fire MORE often rather than less: the stricter the rule, the more often the one
path considered is the one rejected.

A crossing limit is not a property of a PATH, it is a property of a path PREFIX —
how much unbroken open water you have already crossed to arrive somewhere — so it
belongs in the search STATE. `coarse_dijkstra_legal` makes a node `(cell, run)`:
`run` counts consecutive open-sea cells ending at this one, resets to 0 on shelf,
coastal water or land, and a step that would push it past the cap is simply NOT AN
EDGE. The search therefore returns the cheapest LEGAL route, which for two ports
either side of a gulf is the one that follows the shore.

Three things that make this safe:
- **The state space is bounded at every world size.** `run` is capped (~14 coarse
  cells at the shipped 800 km) and the coarse grid is ~720 cells wide whatever the
  world, because `f = grid_w / 700`. A Large map does not enlarge it.
- **Reach 2 needs no run state** — land-only is a plain mask
  (`coarse_dijkstra_masked`), so it does not pay for lanes it cannot use.
- **`path_allowed` survives as a `debug_assert!`** at the call site, not as control
  flow: the searched route must satisfy the very rule it searched under, and if it
  ever does not, that is a bug in the search rather than a lane to discard.
Gate: `a_lane_follows_the_coast_instead_of_being_thrown_away`, which asserts BOTH
halves on ONE fixture so it cannot pass vacuously — the old search-then-filter must
FAIL on that fjord, and the new search must return a longer, legal, coastal route.
Writing it took two fixtures: the first was a shallow bay where going round was
already cheapest, so search-then-filter would have kept the path and the gate proved
nothing. **A routing gate needs a geometry where the illegal line genuinely WINS.**

**THE RULE DID NOT REACH THE CAMPAIGN'S OWN ROUTING, only the map query layer.**
`CampaignSim.days`/`route_outlet` — which decide every relay assignment, dispatch
choice and freight cost inside a running campaign — were priced by
`compute_route_days_matrix_for_season` calling the plain unconstrained
`coarse_dijkstra_dist_prev`, never `coarse_dijkstra_legal`. So a relay pair could
be internally "legal" by the campaign's own accounting while the map's own
resolver (correctly enforcing the rule) found no legal route for it and fell back
to the dashed-direct convention — the straight line was the render being honest
about upstream data, not a rendering bug. Fixed with `coarse_dijkstra_legal_dist`,
the same `(cell, run)` state extended to a full single-source-to-everywhere
search (§8.5's `route_days_matrix` fix, 2026-09-19c). A SECOND, LIVE injection
point had the same gap with a worse unit error: `production.rs`'s periodic
`rebuild_routes` (`#6c` coastal cabotage) wrote raw Euclidean distance straight
into `days` with no pathfinder and no crossing check, and its own
`CABOTAGE_SEA_FRAC` constant computed to ~3,206 km against a doc comment claiming
"a deliberately SHORT crossing" (~1,000 km) — 4× the real 800 km cap. Now derived
directly from `MAX_OPEN_SEA_CROSSING_KM`. `#6`/`#6b` (the guaranteed-partners and
market-lifeline rescues) share the same unconstrained-Euclidean shape but are
same-component-only (land routes have no crossing cap to violate) — named as a
queued, unmeasured follow-up rather than silently assumed clean (rule 36).

**AN ENTREPÔT PICKED BY RAW DISTANCE ALONE IS NOT A REAL PLACE MERCHANTS STOP.**
`#6d`'s outlet was "any real coastal hub, whichever minimises `days_before[a][p]`" —
purely geometric, with no requirement that anyone actually traded there. Composed
with an unconstrained rescue leg (the `#6`/`#6b` gap named above), this is exactly
what let a relay's OWN two legs come out implausibly long even after both crossing-
rule fixes landed: neither leg crosses open water illegally, so the map draws it as
a real solid line, but a "break of bulk" at a hub with zero actual trade volume is
not a transshipment, it is a coincidence of geometry. `hub_trade_volume` (summed
from `trade_last`, both directions folding onto the same hub index there) now
gates outlet eligibility: a candidate outlet must already show real annual trade,
unless NO hub anywhere has any yet (a brand-new campaign's first `rebuild_routes`,
before `fold_trade_year` has ever run), which keeps the old coastal-only rule as
the bootstrap case so a fresh campaign still has entrepôts to grow from. The same
session also fixed the break-of-bulk relay LIST (`read_hubs.rs`) to only report
pairs with real trade volume between them — a pair whose cheapest theoretical path
happens to relay through this hub, with nothing ever shipped that way, was showing
in the panel as "206d, no direct trade", which read as a route when it was only a
hypothesis. `#6`/`#6b`'s own unconstrained-distance gap (named above) is unchanged
by either fix and remains queued.

**Still post-filtered, and why** (queued, not waived): `coarse_dijkstra_batch` —
which feeds `compute_trade_routes`' drawn road network and `compute_economy`'s
candidate edges — still searches then filters, so a rejected pair there is a route
that simply is not drawn (never a straight line). Giving the batch the same
treatment means carrying `(cell, run)` dist/prev per SOURCE while rayon fans sources
out in parallel, which is tens of MB per in-flight source — a real memory question
that needs its own measurement, not a copy of this change.

**A LANE IS NOT ONE MEDIUM END TO END.** `compute_coarse_route` returned a bare
`Vec<[f32; 2]>` and the renderer styled the whole lane from a single flag —
`openWater`, meaning "did anything route this at all". So a haul that runs
overland to a port, crosses, and runs overland again was drawn end to end as one
thing, and read as a straight sea slash between two cities even where it followed
a real road. It returns `CoarseRoute { points, sea }` now, with the medium of
EVERY point, and `OverlayManager.mediumRuns` splits a lane into consecutive runs
of one medium (breaking at the wrap seam too, rule 6) so each is stroked in its
own convention: **dashed on open water, solid wherever the cargo is on a road or
a navigable river**.

Four things to keep true here:
- **`sea` and `openWater` answer different questions and both are needed.**
  `openWater` decides whether a lane earns direction arrows and full opacity —
  a straight line with arrows every few cells reads as a drawn road, which is
  exactly what an unroutable claim is not. `sea[i]` decides how each stretch is
  stroked. A lane nothing could route is all-sea AND `openWater`, so it stays
  dashed end to end, which is the honest reading of a claim.
- **A segment is sea when EITHER endpoint is water**, so the landing legs belong
  to the crossing and no solid stub pokes out into the sea. Consecutive runs
  share their boundary point, so there is no gap where the medium changes.
- **The worldgen trade-route graph carries medium too.** `ensureTradeGraph`'s
  edges take `TradeRoute.kind` (0 land · 1 sea · 2 river), so a path assembled
  out of that graph knows its stretches as well as a coarse-grid route does —
  otherwise a worldgen sea lane drew solid while an identical campaign one drew
  dashed.
- Applied so far to the **Flows highlight** (`renderFlowHighlight`) and the
  **Goods Atlas flows** (`drawGoodFlows`). Merchant routes, the house trading
  web and futures lanes all go through `laneBetween` and so already HAVE the
  per-point medium available — they just still draw with the old whole-lane
  styling. Extending them is queued, not done.

**This is also what finally produces coast-hugging**, and it is worth recording
that it did NOT come from the cost ramp. `OPEN_SEA_COST` (2.2) is only 1.375x
`COASTAL_SEA_COST` (1.6) while a coast-hugging detour is routinely 2-4x longer
in cells, so on cost alone a least-cost path always cuts straight across — and
because open sea is cost-UNIFORM, Dijkstra's cheapest path across it is a
literal straight line, which is what the ruler-straight ocean lanes were.
Fixing that by raising the ratio to 3-5x was the obvious move and is still NOT
done, because `cost_to_days` makes these same constants double as travel SPEED
(C2's conflation in `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md`), so it would
reprice every voyage and move the price/distance gradient C1 just tuned. A
ROUTING rule achieves the same visible outcome at zero cost to the speed
calibration: forbid the long crossing and the cheapest legal path is the
coastal one. Prefer a routing rule over a cost nudge wherever the cost constant
is load-bearing for something else — and the cost-ratio work stays queued, with
`real_world_price_distance_gradient` as the instrument that can tell the freight
effect from the speed effect if anyone does take it on.
Related finding, left as a finding: the piracy surcharge charges COASTAL water
4.0 against open sea 1.5, which pushes routes AWAY from the coast — the opposite
of what its own comment claims it does. Inert at the default `piracy = 0`.

### 8.6 Köppen current overrides
Mediterranean (Cs) only forms on **windward (west-facing) coasts** beside a cold
offshore current (`cold_override` gated on `is_windward_ocean` + no warm influence);
a warm-current **east** coast reads humid-subtropical (Cfa).

### 8.7 Render layers (26) & paint tools (5)
The **biomes** layer is no longer a Köppen recolour — it reads the `biome` column
and draws procedural pattern fills (§8.12). **`natural`** is the land-COVER view
(§8.21) — the only layer whose land colour is not a function of height or class
index but of what grows there.
land, elevation, terrain (hillshade), **natural**, plates, shelf, ridges, fisheries, currents,
**sst** (sea-surface temperature), temperature, precipitation, wind, **windspeed**
(low-level wind intensity incl. jets), **snow** (annual snow-cover fraction), climate,
biomes, soil, fertility, salinity, habitability, shark, shipworm, reef, storm, disease.
Paint: Pan, Paint Land (0/1), Elevation (f32 0-1), Paint Shelf (u8 0/1), Place Volcano (u8 0/1).

### 8.8 File operations
Save/Open via SQLite backup API (`.worldforge` / `.campaign`); Export Heightmap
(16-bit grayscale PNG from elevation); Export Layers / trade data; Import Template
(image → land/sea auto-detection).

### 8.9 Phase-3 performance shape (read before touching `step3_ocean_atmo/`)
Phase 3 runs on the **world** grid (6.5 M cells at the default 3600×1800, 26 M on
"Large"), so a per-cell scan that looks harmless is an O(n·w) trap. `bench.rs`
prints a per-sub-step millisecond breakdown; run it after any change here.

Measured @ 3600×1800, release, 4 cores: **~16 s** (was ~100 s before the
optimisation pass). The four costs that matter, and the rules they imply:

| Sub-step | ms | Shape |
|---|---|---|
| `generate_ocean_currents` | ~5600 | warm/cold streamline tagging |
| `advect_salinity_and_recouple` | ~4200 | a second `extend_warm_tag` |
| `compute_precipitation` | ~2900 | per-cell moisture rays + 36 blur passes |
| `compute_low_level_jets` | ~2200 | per-cell barrier rays + 48 propagate passes |

1. **Never scan outward per cell.** Distance-to-land fields are linear sweeps
   (`precompute_basin_dist`/`_ns`): a running counter per row/column, not a
   search. The naive form cost 31 s of the old 100 s on its own.
2. **The row loops are rayon-parallel** (`par_chunks_mut` over rows, or
   `into_par_iter` over seed rows). Keep new passes writing only their own cell so
   they stay that way. Where a pass is a monotone union or a max-reduction, it is
   parallelised with relaxed atomics (`AtomicBool` for the tag corridors,
   `AtomicU32::fetch_max` on the f32 bit pattern for the moisture field — valid
   because moisture is never negative), which keeps the result bit-identical
   regardless of scheduling.
3. **The streamline tracers are latency-bound, not compute-bound.** They walk a
   scattered index for hundreds of steps, so they read a packed `trace_view`
   (one flag byte + one interleaved `[vx, vy]`) instead of four separate columns,
   and they fold the tracer's x back into `[0, w)` every step so `wrap_x` stays on
   its cheap in-range path.
4. **Hoist anything loop-invariant out of a repeated pass** — the jet propagation
   resolves each cell's upwind neighbour once, not on all 48 passes.

The whole pass is **output-preserving**: `ocean_atmosphere_field_checksums` prints
a checksum per phase-3 field, and every one is unchanged vs. the pre-optimisation
code. Use it the same way for any future refactor here — the Earth gate scores
agreement to 0.1 %, which cannot tell bit-exact from merely close.

### 8.10 Province borders — why the partition has TWO stages
`sim/shared/provinces.rs` (phase 7b) does **not** simply flood from seeds. It cannot:
a cost-flood's border falls where two seeds' CUMULATIVE costs tie, so a barrier of
penalty `P` merely displaces that tie-line by ≈`P/2` cells. Adding cost can bias a
border toward a river or ridge but can **never pin it to one** — a feature a few cells
off the tie-line is simply crossed. So:

1. **Cost-flood (Dijkstra)** — sets province COUNT, SIZE and TOPOLOGY. Seeds are the
   big settlements plus a habitability-scaled jittered scatter that prefers valleys.
2. **`snap_borders_to_features`** — a **marker-controlled watershed** (Meyer flooding)
   that re-places the border LINES. Erode each province by `SNAP_R`=3 → markers; build
   a relief of `crest + trunk river`, plus a low `FLAT_ANCHOR` ridge along the flood's
   own border so featureless terrain keeps the line it had; flood from the markers
   always taking the lowest relief. Two floods therefore meet on the highest ground
   between them — the crest, or the middle of the channel.

Three rules that are easy to get wrong here, all covered by `provinces::tests`:

- **The divider is a CREST, not an altitude.** `compute_ridge` scores how far a cell
  stands above BOTH sides along some axis, sampled at radii 2 and 4 on lightly blurred
  elevation. Absolute elevation (the old `(elev-0.26)*18`) makes a whole plateau
  uniformly expensive — so it prefers no border line at all — and gives sub-2300 m
  ranges exactly zero. Never sample prominence at ±1: the 3×3 blur spreads a narrow
  ridge over three columns, and a broad range is flat at its own summit.
- **Great rivers divide, small rivers unite.** Navigable/major trunks are a crossing
  PENALTY; every lesser river is a step-cost DISCOUNT along its own cell, so a province
  spreads through its valley and halts at the interfluves.
- **Charge river/lake crossings on the EDGE, not the cell.** A channel traced by
  following flow is an 8-connected staircase, and a diagonal step cuts clean between
  two of its cells without entering either — which made diagonal rivers free. Both
  floods inspect the two corner cells of a diagonal step.

Measured by the tests: borders sit on a crest **3.1×** more often than chance, and a
diagonal trunk river is a border **3.3×** more often (both ≈1.0× before). The partition
is also **deterministic** — `HashMap::iter().max_by_key` ties must stay broken on the
key, or the same seed yields different maps across runs.

### 8.11 Map label typography
Place names used to be styled at each call site (~23 separate `ctx.font =` lines), so
provinces and settlements came out in an IDENTICAL face and colour and water differed
from rock only by tint. Every place-name class now resolves through **one registry** in
`OverlayManager.ts`, the exact shape of the `lineColors` registry beside it:

```
LABEL_FONTS          4 system-font stacks (Windows → macOS → Linux). NO bundled fonts.
LABEL_STYLE_DEFAULTS per class: family · weight · italic · caps · tracking · color · size
labelStyles          the live registry the renderer reads each frame
setLabelStyles()     partial override, called by settingsStore
LABEL_THEMES         Mixed Contrast (default) · Classic Atlas · Engraved Antique · Modern Cartographic
```

`LabelKey` covers province · settlement · river · lake · mountain · desert · forest ·
tundra · cultureRegion · peopleTerritory · tradeBasin. The default follows the atlas
convention — **nature is serif and leans, human works are sans and stand upright**.
Road names and river-break markers are deliberately NOT in the registry.

Three rules for anyone touching this:

- **Draw through `drawLabel` / `measureLabel`.** Never set `ctx.font` for a place name
  again, or that class silently escapes the theme and the Settings panel.
- **Tracking is drawn CHARACTER BY CHARACTER, never via `ctx.letterSpacing`.** That
  property is effectively Chromium-only and Tauri runs WebKit2GTK on Linux and WKWebView
  on macOS, so it would silently do nothing on two of three platforms. Manual drawing is
  also what makes `measureLabel` exact.
- **Measure the STYLED string when a label has to fit something.** Province names are
  tracked capitals, far wider than the raw text; `renderProvinces` only draws a name that
  fits its inscribed circle, so it degrades **tracked caps → untracked caps → mixed case
  → hide** instead of hiding the moment the styled form overflows (§8.10's label anchor
  work is undone otherwise).

Note `OverlayManager.rgba()` only understands `hsl()` and returns a hex unchanged — use
`labelAlpha()` for label colours, which are all hex.

---

### 8.12 Biomes — the ecological layer (phase 6b)

Köppen answers "what is the *climate* here"; it does not answer "what *grows*
here". `sim/step6_soil_fertility/biome.rs` turns the climate stack into an actual
vegetation map, writing the `biome` u8 tile column (**41 codes**, 0 = unclassified).

**Three axes.** Two are Holdridge's, which is the right tool for exactly this job:

- **Biotemperature** — the mean of the 12 monthly temperatures with each clamped
  to 0–30 °C: heat plants can actually use. Frozen months contribute nothing;
  heat above 30 °C buys no growth. Derived via `koppen::seasonal_temps` (the
  documented single source of truth for the warm/cold split), so the biome
  treeline agrees with Köppen's own C↔D boundary and the settlement winter gate.
- **PET ratio** = `58.93 · biotemp / annual precip`. 1.0 is the humid/dry hinge,
  >2 semi-arid, >4 arid. This is why a cold 300 mm cell comes out *steppe* while a
  hot 300 mm cell comes out *desert*, with no special-casing.
- **Seasonality** from Köppen's own `s`/`w`/`f` letter + `precip_summer_frac` —
  what separates Mediterranean sclerophyll from evergreen temperate forest.

**Precedence** (each overrides those below):
```
1. Cryosphere   permanent ice / nival rock — nothing grows, full stop
2. Azonal water mangrove · salt marsh · marsh · swamp · peat bog · gallery forest
                · oasis · floodplain · salt flat. LOCAL water beats regional
                climate — this is the layer that makes rivers & lakes LEGIBLE.
3. Altitudinal  above the CLIMATIC treeline (warmest month < 6.5 °C), so the
                treeline falls from ~4000 m in the tropics to sea level at the
                Arctic circle instead of sitting at a fixed elevation.
4. Zonal        the Holdridge/Köppen life zone for the cell's own climate.
```

Four rules that are easy to get wrong here, all covered by `biome::tests`:

- **The treeline is a TEMPERATURE, not an elevation.** The old layer used fixed
  0.40/0.62 normalized-elevation cutoffs, which put alpine tundra on warm tropical
  highlands and none on cold polar lowlands.
- **The riparian corridor's WIDTH scales with aridity.** A riverbank in rainforest
  looks like rainforest (reach 1, and the humid guard drops it entirely); in
  desert the gallery/oasis strip is the only green for a hundred km (reach 6).
  Repainting humid cells as "gallery forest" just adds noise.
- **Mangroves are frost-bounded** (coldest month ≥ 5 °C), which is why they stop
  near 30° on the real Earth; the same tidal flat carries salt marsh beyond that.
- **`TIDAL_MAX_M` is 80 m, not a true tidal elevation.** A cell is ~11 km across at
  the standard 3600×1800 grid (5.6 km on Large; `KM_EQUATOR / grid_w`), and a shore
  cell's elevation is a coarse average over that,
  so the honest question is "is this a low depositional coast" — tightening
  it toward real tidal heights empties the biome, since the elevation field's
  coastal taper rarely lands a shore cell below ~40 m.

**Descriptive only.** No later phase scores off `biome` — goods, fertility,
habitability and settlement placement are untouched, so re-running it cannot move
a city or a trade belt. Keep it that way unless FIX_PLAN B1 says otherwise.

**Rendering** (`render/tile_image.rs`): the Biomes layer is a per-biome colour
carrying a **procedural pattern fill**, in the tradition of a geological survey
sheet — canopy blobs for broadleaf, chevron spires for conifer, tussock ticks for
steppe, the standard horizontal dashes for marsh, sinusoidal ripples for a sand
sea, crevasse lines for glacier ice, a cracked lattice for a salt pan. Two rules:

- **Every pattern period must divide `TILE_SIZE` (128).** Patterns are functions
  of the cell's position WITHIN the tile, so a divisor period is what makes them
  line up tile-to-tile without the renderer knowing its world coordinates.
  `every_biome_pattern_tiles_seamlessly` asserts exactly this and is not
  decorative — it caught the dune ripple shipping at a 29.09-cell period
  (128/29.09 = 4.4), which would have drawn a seam across every sand sea. The
  corollary is that a fill's pseudo-random component REPEATS every 128 cells;
  that is correct for a cartographic hatch and invisible in practice.
- **Two biomes must be PERCEPTUALLY distinct, not merely unequal.**
  `biome_colors_are_distinct` used to check exact RGB equality, which passes
  happily on colours no reader can separate: tropical seasonal forest vs temperate
  deciduous shipped at ΔE 3.0 and thorn scrub vs chaparral at ΔE 3.8, and both
  pairs share a `Pattern` kind, so texture did not rescue them either. It now
  asserts a CIELAB floor (`MIN_DELTA_E`), set to what the palette already
  satisfies rather than to an aspiration — raise it as the palette earns it. The
  target for two biomes sharing a pattern is ΔE ≥ 8.
- **Contrast has a floor and a ceiling.** Under about 0.15 between mark and
  ground a pattern is technically present and visually absent (the first cut of
  the scrub dots and the marsh dashes both were); over ~0.20 it stops reading as
  texture and starts reading as a different colour, so two biomes blur together.
  `pattern_amplitude_stays_within_a_readable_band` holds the ceiling — it caught
  peat bog stacking its dash and hummock layers to 0.25.
- **Shading is a SEPARATE multiply applied after the pattern**, and the ceiling is
  held on the COMBINED swing (`pattern_and_relief_together_stay_readable`), not on
  the pattern alone — a test that measures only the pattern passes happily while two
  biomes blur. The four thematic plates (climate · biomes · soil · fertility) carry
  an attenuated hillshade (`THEMATIC_RELIEF_AMP` = 0.09) so a climate zone sits on
  the mountains that cause it, the way Bartholomew's and the Times' physical plates
  print tint over relief. It REPLACED the cruder `1.0 + (e − 0.2) · 0.18` elevation
  lift, which reached +0.144 alone and stacked on the same ±0.19 pattern — so the
  combined excursion went DOWN while the plate gained real form. Every shading layer
  now needs its neighbour ring in `tile_commands.rs`, or slopes break at tile edges.
- **They are SYMBOLS, not surface texture.** Holding a fixed pixel scale across
  the LOD pyramid is correct — that is how printed map hatching behaves.
- **`cargo test --lib render::tile_image::tests::dump_biome_swatch_sheet --
  --ignored --nocapture`** writes a swatch sheet + a tile-seam proof to
  `$BIOME_SHEET_DIR`, rendered through the real `render_tile` path. Use it to
  eyeball a palette or pattern change instead of guessing.

`BIOME_SWATCH` is **gone**: the legend now reads biome colours from
`get_render_palettes` (§8.18), so there is no second copy to keep in sync. Its
old doc-comment asked future editors to mirror `biome_color` by hand — three
colours changed in the very commit that deleted it, which is what that comment
could never prevent.
A world saved before this phase pads the column to zero and falls back to
`koppen_fallback_biome`, so the layer is never blank.

### 8.13 Cordillera elevation (`generate_elevation_cordillera`)

The third elevation MODEL, beside shape-based and ridged. `generate_elevation_ridged`
fills noise-defined orogenic belts with isotropic ridged multifractal: statistically
mountainous, but with no chain, no crest line, no consistent strike and no
rain-shadow side. A cordillera has all four, and they are what a map reader sees.

1. **A traced spine.** Crests are walked as polylines along an iso-contour of
   distance-from-coast — the walker steps perpendicular to ∇(coast distance),
   which is by construction parallel to the coastline, so the chain shadows the
   margin the way a subduction orogen does. Noise on the heading plus a slow
   drift of the target offset keep it off a mechanical offset curve.
2. **A continental divide.** The spine is continuous, so rivers part along it and
   the drainage map inherits a real watershed backbone.
3. **Asymmetric flanks.** Seaward: a steep scarp (falloff exponent 1.9) to a
   narrow coastal plain. Inland: a broad piedmont apron (exponent 3.2) over
   ~2–4× the distance. The side is decided by comparing a cell's own
   coast-distance to the crest's, carried outward by the BFS.
4. **Parallel sub-ranges.** A cross-strike cosine puts 1–3 sub-crests inside the
   envelope with intermontane basins between — Occidental / Central / Oriental.

Along-strike the crest tapers to nothing at both ends and undulates between, so
the chain emerges from the lowlands with summits and saddles rather than being a
uniform wall.

Guarded by `elevation::tests`: the crest's coast-distance **spread** must be
under 80 % of the ridged generator's on the same landmass (the real
chain-vs-noise discriminator — a plain connectivity metric saturates and does not
discriminate), the inland flank must average >10 % higher than the seaward at
equal distance, and the same seed must reproduce bit-identical elevation (the
spine tracer uses an RNG **and** a shuffle).

### 8.14 Settings preview (`step3_ocean_atmo/preview.rs`)

The planetary knobs are not self-explanatory — nobody knows what "rotation 0.6"
does to a map. Two previews make them legible, split by cost:

**Tier 1 · `zonal_profile`** — everything the knobs do that is purely a function
of LATITUDE: the EBM temperature curve (with Earth ghosted behind it, so the user
reads the ANOMALY), the seasonal envelope, and the belt edges. Two EBM solves plus
closed-form `Circulation` = microseconds, so the UI recomputes it on every drag.
It sees no land, so it can say how wide the desert BELT is but not where a desert
lands.

**Tier 2 · `coarse_climate_preview`** — the real phase-3 → Köppen chain on a
downsampled copy of the user's own landmass (≤600 cells wide, ~1/36 the cells of a
full run). Answers the question tier 1 cannot: where the deserts actually go, and
what share of THIS world each climate takes. Land/sea is downsampled by MAJORITY
VOTE, not point sampling — point sampling drops islands and severs isthmuses,
which changes the ocean circulation the preview exists to show.

Both build a throwaway in-memory `WorldBuffer` and never touch a tile, the DB, or
the world's stored planetary state; the knobs arrive as command ARGUMENTS so the UI
can preview values mid-drag, before they commit.

Three rules:

- **The phase-3 sequence is now duplicated in THREE places** — `sim_commands.rs`,
  `earth_validation.rs` and here. Change one, change all three (extends rule 11).
- **Archetype endpoints are MEASURED, not guessed.** `sweep_cooling_knobs`
  (ignored) walks the cooling knobs; the ice-albedo collapse is abrupt — alive at
  greenhouse 0.55 (−3.7 °C, 29 % polar), a total snowball at 0.50. `SNOWBALL_ICE_LINE`
  = 52° puts the warning one to two steps ahead of that cliff, and the Ice House
  archetype's strong end stops at 0.58 so its dial can never kill the world.
  `archetypes_deliver_what_their_blurbs_promise` asserts each archetype's headline
  claim, because the UI text makes promises the physics has to keep.
- **Absolute percentages from the test's synthetic slab mean nothing.** It is a
  flat rectangular continent with no orography, so its interior is far more arid
  than any real world; only the DELTAS between rows are meaningful. The real
  fidelity measure is the Earth scorecard (§2.3).

Adding the preview surfaced a live bug in `koppen.rs`: the lowland latitude
guardrails demote a cold low-latitude cell toward temperate, on the premise that
it can only be cold by the cold-current/upwelling over-cooling artefact. That
premise is false on a genuinely frozen world, where a snowball came out reading
72 % "temperate" at −44 °C. `FROZEN_WORLD_TEMP` (−12 °C) exempts genuinely frigid
cells; the value is the largest exemption that leaves the Earth scorecard
bit-identical, and `koppen::guardrail_tests` guards both sides of it.

### 8.15 The law of inheritance (`sim/shared/inheritance.rs`)

Two enums on the CULTURE, read at exactly one place (`succeed_house`), which between them
decide whether a merchant family accumulates across generations or is reconstituted at
every death:

* **`LineRule`** — Agnatic · AgnaticCognatic · Absolute · Enatic (who is eligible).
* **`InheritanceRule`** — Partible · Primogeniture · Ultimogeniture · Seniority ·
  Matrilineal (how the estate divides).

Assigned per language kit where the record is clear (Roman *sui heredes* took equal
shares; Celtic **tanistry** elected the eldest capable; the Mongol *otchigin* was the
hearth-keeping youngest) and drawn from that same distribution for a culture whose kit is
unknown, so a synthetic or legacy people still gets a historically-shaped law rather than a
hard-coded one. Resolved ONCE into `CampaignSim.culture_rules` and never re-rolled, so a
reloaded save cannot change a people's law.

Four rules for anyone changing this:

- **Matriliny is a seeded minority, not a named kit.** ~10% of peoples (22% of `Clannish`
  ones — kin-bound descent groups are the documented precondition) come out
  `Enatic + Matrilineal`, deterministically from the culture's own name. Assigning it to
  one named kit would be a factual claim about a real people this model cannot support.
- **The accession AGE is the mechanism, not the death age.** Ultimogeniture and
  primogeniture both concentrate; what separates them is that the hearth-keeping youngest
  takes over at ~17–31 and an elected tanist at ~44–62. Never "fix" tenure by touching the
  death age — that erases the difference between three of the five rules.
- **Only `Partible` divides.** Concentration is the ABSENCE of a split, and the gate
  asserts the other four never divide. A division moves capital from parent to co-heir and
  creates none; a co-heir inherits capital and **no fleet** (hulls it never paid for is the
  exact arithmetic behind the original 12-year house — see `docs/SCOREBOARD.md`).
- **Milestones are never pruned.** `HOUSE_EVENTS_CAP` prunes chatter (feud flares, lost
  caravans) oldest-first; `is_house_milestone` kinds — founding, succession, division,
  monopoly, charter, ruin — survive to `HOUSE_MILESTONE_CAP`. Before this a house in a hot
  feud lost its own founding within two years, which is not a cosmetic loss: for an
  observation-only game the chronicle is the product.

Gate: `cargo test --lib econ_inheritance_rules_fragment_differently -- --nocapture` runs
**THREE worlds** (`INHERITANCE_GATE_SEEDS = [42, 1337, 7]`, a prefix of the robustness
diagnostic's own 6-seed list chosen by position rather than by which seeds happen to
pass) four times each, changing only the law, and asserts the rule is wired on every
seed (partible divides, the rest do not) and that it MATTERS — **more houses ever
founded** (per seed, plus a ≥1.05× margin on the seed-AVERAGE — a single seed's ratio can
dip as low as 1.02, so the margin floor is checked on the average rather than per seed,
which would reject a real contrast for landing on an unlucky world) **and lower mean
wealth per house** (per seed — this one holds 6/6 in the full sweep, the stronger of the
two). Note what it does *not* claim: the top share and Gini do not fall under partible,
because a division adds small firms at the bottom as fast as it trims the top.
**Multi-seeded per `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` §10 Q1** — a single fixed
seed is what let realm formation, crisis relief, the trade horizon and
`COMFORT_IMPORT_FRAC` each perturb this gate in turn without the gate itself being able
to tell a real confounder from ordinary cross-world noise (see the cautionary tale
below). Runtime cost: ~6 minutes in debug (vs. ~2 for the old single-seed form) — the
full 6-seed sweep stays `#[ignore]`d for exactly that reason.

Two companions were added alongside. `a_division_moves_capital_and_creates_none` asserts
the zero-sum invariant AT `divide_estate`, where it is decidable, instead of inferring it
from an aggregate 60 years downstream. `econ_measure_inheritance_robustness` (`#[ignore]`d)
runs the partible/primogeniture pair across 6 seeds and reports how often each candidate
contrast holds.

**A CAUTIONARY TALE ABOUT THAT SWEEP — read it before trusting a robustness measurement.**
The mean-wealth assertion was once *removed* from this gate as "measurably false": the
6-seed sweep found it holding on only 1 seed. That conclusion was confident, documented at
length, and **wrong**. The sweep had been run while `COMFORT_IMPORT_FRAC` still sat at
`a7ff520`'s 0.60 — the very dose that had inverted this gate in the first place. Re-run at
the corrected 0.30:

| contrast | @0.60 (broken) | @0.30 (shipped, then) | @0.30 (shipped, 2026-09-11 re-measure) |
|---|---|---|---|
| more houses ever founded | 6/6 | 5/6 | **6/6** |
| more houses still standing | 4/6 | 2/6 | 5/6 |
| lower top share | 2/6 | 3/6 | 4/6 |
| **lower mean wealth per house** | **1/6** | **5/6** | **6/6** |
| no MORE capital in total | 1/6 | 5/6 | 4/6 |

The claim is real; the dose genuinely broke it. **A seed sweep only tells you about the
world you ran it in** — measuring robustness inside an already-distorted economy produced
a thorough, plausible, false conclusion ("the merchant pool is not conserved; firm count
is a multiplier on merchant wealth"), which is an artefact of the 0.60 dose and not a
property of the model. Before concluding an assertion is unsound, check that the world you
measured in is not itself the thing that is broken.

**The 2026-09-11 column is why the gate went multi-seed instead of staying at 5/6.** Both
of the gate's own assertions (houses-ever direction, mean-wealth direction) now measure
6/6 — a real improvement, most likely from Stage A/B's routing fixes landing between the
two measurements, though that was not isolated and re-confirmed by bisection. The lesson
generalises past this one dose: **a stale sweep is not a fact about the model, it is a
fact about the code and the dose at the time it was run** — re-measure before trusting an
old conclusion in EITHER direction, not only when a result looks suspicious.

**THE TWO GATES DISAGREE ABOUT THAT DOSE, and the one that set it isn't about trade.**
0.30 was chosen because it restores this gate — weak grounds for a demand parameter, so
it was measured against a gate that isn't the target (§2.4). Sweeping it against
`econ_fidelity_scorecard`: the basket price/distance gradient reads **−0.064 at the
shipped 0.30 (0 of 6 goods showing any gradient)** and turns **positive at 0.60 (+0.041,
2 of 6)** and 0.90 (+0.053, 3 of 6). A positive gradient is the historically correct sign
and its absence is `TRADE_AND_MARKET_REVIEW.md`'s F2 — the largest market failure this
project has named. So the shipped value is the worst of four tested on market integration.

**Nothing was changed**: raising it re-breaks this gate, and buying integration with a
demand constant is not the fix F2 asks for (it blames freight at ~11% of grain value over
the longest route, and i.i.d. per-hub harvest shocks leaving no regional scarcity). Full
table, caveats and reproduction recipe at the constant's own doc comment and in
`docs/SCOREBOARD.md` 2026-08-20d. Read the caveats before acting: one seed per dose, the
low end is not monotone, and every dose leaves basket CV far outside its band.

**Read this before "fixing" this gate again.** It has been perturbed five times (realms,
crisis relief, the trade horizon, estate-share tuning, comfort-good import demand). The
first three were genuine confounders and are isolated with `suppress_realms` /
`suppress_relief` / a widened `world_w`. The fifth was a real break in a real mechanism,
fixed at its source. Diagnose which of the three you have — confounder, broken mechanism,
or bad assertion — and be sure your instrument is not reading a world the bug already bent.

---

### 8.16 Ore deposits — the geology (`step8_biological_goods/deposits.rs`)

Ore geology is almost entirely a function of TECTONIC SETTING. That is the
organising principle of the discipline, not flavour — and until this module landed
the placer ignored it completely, deciding where a metal or gem went from an
absolute elevation floor plus a per-good noise field. Phase 1 already computes
`boundary_type`, `plate_index` and `is_volcanic`; phase 5 already computes rivers.
None of it was read.

A deposit good now carries a **`DepositModel`**, and the model decides placement:

| Model | Scored from | Type localities |
|---|---|---|
| `VolcanicArc` | volcanism + convergent proximity (~25 cells ≈ the real arc-trench gap) | Potosí, Cyprus, Rio Tinto, Almadén |
| `CollisionalOrogen` | high + rugged + convergent + **not** volcanic | Cornwall tin, the Erzgebirge, Muzo |
| `Craton` | far from ANY boundary + low relief + interior | **Clifford's Rule** — economic kimberlite occurs only over thick Archean lithosphere |
| `Rift` | divergent OR flood-basalt plateau | Kupferschiefer, the Deccan |
| `CarbonatePlatform` | low + flat + old shallow sea | Mendips lead, Silesia zinc |
| `ContactMetamorphic` | platform carbonate NEXT TO an orogen | Carrara, Mogok ruby, Sar-i-Sang lapis |
| `EvaporiteBasin` | Köppen B + low + flat | Wieliczka, Hallstatt |
| `Placer` | **derived** — walked downstream from a parent lode | Pactolus, Golconda, Ratnapura |
| `Bog` | wet + low + flat + near water | medieval N-European bog iron |
| `CoastalMarine` | shelf / beach / warm shallows | Baltic amber, Gulf pearls |
| `Weathering` | **derived** — supergene alteration of a parent, arid | Nishapur turquoise |

`deposits::default_model_for(id)` ships the correct model per mineral; a spec may
override via `DepositSpec.model` / `placer_frac` / `parent`, all serde-defaulted.

**The three-level hierarchy.** Real ore geology is described at three scales and the
old placer collapsed all three into one cell: metallogenic BELT (100–1000 km, the
setting field × per-mineral noise) → ore DISTRICT (10–60 km, `MIN_DISTRICT_SEP_KM`
= 320) → WORKING (1–10 km = one cell). A cell is already ~11 km, i.e. FINER than a
district, so clustering was never a cell-size problem — it was `min_sep = w*0.025`
≈ 1000 km between single cells.

**How MANY districts scales with LAND AREA** (`deposits::district_budget`). The count
used to be `richness × count_num / count_den` flat — about six to nine districts of a
metal for a whole planet whatever its size, where medieval Europe alone worked a dozen
great silver districts. It is now that product × `land_area_km2 /
DISTRICT_LAND_REF_KM2` (18 M km², stated in km² per rule 25), so an Earth-like land
area carries ~8× the old count; the per-mineral RATIO is untouched and still gated by
`deposit_counts_span_an_order_of_magnitude`. On top of the districts sit **minor
showings** (`SHOWINGS_PER_DISTRICT` = 2 per district, `MIN_SHOWING_SEP_KM` = 110): a
single WEAK/MODERATE, lower-grade working each, placed LAST on the same candidate
ground so the district and alluvial placement is untouched — the village lead pit
between the great camps. The Biological step's slider is now "Ore richness"
(6 = ×1.0), not a count. Measured (`deposit_census_diagnostic`, 600×300, 138 M km² of
land): 8,094 workings, silver 119 districts incl. showings (was 9), iron 629, tin 82,
lapis 24. Two things this made necessary: the district spacing test runs through a
`SpacingGrid` (exact same accept/reject as the all-pairs scan, gated by
`spacing_grid_matches_all_pairs`), and the campaign's per-(province, good) deposit
potential is built ONCE into `CampaignSim::deposit_potential_cache` — the direct walk
did an O(provinces) nearest-seat lookup per working per call, which at 10× the
workings would have dominated the yearly province goods pass. `goods_`'s printed
"wrong side of the coast" finding for `CoastalMarine` goods grows with the counts
(bay_salt 115 → 1,864 cells): the same pre-existing `deposits.rs` question, larger.

**Per-working state** the u8 belt cannot carry, persisted to `metadata["deposits"]`
exactly as the province list is: `grade` (→ the quality tier, so "tiers of gems"
becomes possible), `extent` (weak…world-class), `depth` (surface / shallow / deep /
**flooded**). The belt column is written as `grade × depth_workability`, so a deep
rich body is visible but largely LOCKED — inventory for a mining industry that does
not exist yet.

Five rules for anyone changing this:

- **A mineral must never silently vanish.** This is the failure mode this codebase
  keeps hitting (`highland_cap` exists for the same reason). Two guards: the
  threshold-loosening loop, and — when a world's plate geometry offers no ground at
  all for a model — a forced fall back to the relief proxy. That is not
  hypothetical: most divergent boundaries on any Earth-like world are OCEANIC, so
  keying `Rift` strictly on the boundary emptied it, and
  `no_shipped_mineral_places_nothing` caught it.
- **Template worlds have NO plate data** (`boundary_type` is empty). Every model
  degrades to a relief-and-continentality proxy;
  `template_world_without_plates_still_places` guards it.
- **Diamond belongs on flat cratons, not peaks.** The old spec said `min_elev: 0.55`
  — the highest mountains — which is exactly backwards.
  `diamond_lands_on_craton_not_on_peaks` guards it.
- **A derived model needs its parent placed first.** `Weathering` minerals are run
  in a SECOND pass after the main loop, rather than reordering it (which would
  change every other good's placement seed order).
- **`GeoContext` is built ONCE per world**, never per good — 45 goods × a full-grid
  BFS would be a real cost. Its distance fields are multi-source BFS (a linear
  sweep from all seeds at once), never an outward scan per cell (§8.9 rule 1).

**Slice 2 (built):** `economy.rs::compute_economy` now reads `metadata["deposits"]`
and attributes each working to the hub whose catchment claims its cell (the same
`claim` map the belt-production pass already builds), so a `Deposits`-distribution
good's quality is the mean `grade` of the workings actually inside a hub's
territory — not, as before, that hub's share of world production (which read
backwards: a big cheap deposit scored as fine stones). Every other good keeps the
old share-based formula.

**Slice 3 (built):** an INI-ish `.txt` importer (`commands/goods_import.rs`,
`import_goods_txt`, wired to a "Import .txt" button in the Goods Editor) adds
minerals to the global library — ADD-ONLY, an id already present is rejected, never
overwritten. Only `[id]` and `name` are required; `domain`/`distribution`/
`deposit_model` parse through the real enums' own serde representation, so the
parser can't disagree with the type. Eight new minerals ship in
`default_custom_goods()` itself (not through the importer — they're the app's own
library): mercury, alum, lapis_lazuli, turquoise, bog_iron, coal, garnet, carnelian
— each exercising a model or mechanic the shipped six/gem-split never did (a
near-single-source district count, a derived weathering mineral, a bog deposit an
elevation floor could never place). Full detail, including what is deliberately
NOT wired (mercury→silver amalgamation, alum→cloth as a hard recipe input — both
real economic changes that need their own `econ_` measurement, not an add-only
slice) in `docs/DEPOSITS_AND_MINING_PLAN.md` slice 3.

> **Slices 4-5, built:** a MINE (`estate_kind == 2`) carries `mine_depth`
> (the real working nearest its parent city) and digging a deep/flooded body
> costs real drainage capital to upgrade (`MINE_UPGRADE_COST_MULT`); mercury →
> silver amalgamation is a real consumable extraction input. A QUARRY
> (`estate_kind == 8`, split off from Mine) is gated by TRANSPORT instead
> (`QUARRY_INLAND_UPGRADE_COST_MULT`), never depth. A body KNOWN to be
> `EXTENT_WEAK` now declines to a floor under pressure (D3); everything else
> still persists. Mining SETTLEMENTS (the Potosí case,
> `maybe_found_mining_colony`) found on a real GREAT/WORLD-CLASS strike, boom,
> and DECLINE rather than die when their food lifeline fails. A settlement's
> trade catchment radius grows slowly with age (`catchment_radius_km`,
> derived, never stored). See `docs/DEPOSITS_AND_MINING_PLAN.md` slices 4-5.

---

### 8.17 Map plates (`ui/world/mapThemes.ts`)

A published atlas never shows one raster on its own. A climate plate is Köppen fill
PLUS the circulation that produces it PLUS the graticule, set in the face that plate
uses; a political plate is a province wash PLUS borders PLUS city dots sized by rank.
What distinguishes one plate from another is the whole COMPOSITION, not the base
colour. The app had 25 base layers and ~30 overlay toggles and no way to express that
— every informative view had to be assembled by hand, from memory, every time.

A `MapTheme` is that composition in state that already existed: one `ActiveLayer`, a
set of overlay keys, and optionally a label-typography theme (§8.11) and an
overlay-line preset. **Twelve plates ship**, ordered the way the pipeline builds the
world: Physical · Natural Colour · Relief & Height · Ocean & Currents · Climate · Hydrology · Ecology ·
Settlement · Peoples · Political · Goods & Trade · Hazards.

Four rules for anyone changing this:

- **A plate is a VIEW, never a decision.** It sets what you SEE, never what the world
  IS — rule 14 restated. Nothing here is persisted and nothing writes a generation
  setting, so switching plates can never alter a world or invalidate a tile.
- **`MANAGED_OVERLAYS` is DERIVED from the plates, never hand-listed.** Applying a
  plate sets each managed key explicitly — on if the plate lists it, off otherwise —
  so a plate lands in a KNOWN composition instead of washing additively over
  leftovers. Deriving the set is what bounds the blast radius: per-good overlays,
  campaign-only layers and anything else no plate mentions are never touched, so a
  plate cannot silently clear work done in another panel.
- **`activeMapTheme` is cleared by any manual change** (`setLayer`, `toggleOverlay`,
  `setOverlayVisible`, `setOverlaysVisible`, `setWorkflowStep`). A chip that keeps
  claiming a plate after the view stops being one is worse than no chip.
- **A plate whose data isn't generated reads dimmed with the step it waits on**, not
  hidden (`requires` + `themeReady`). Seeing what the finished world will offer is the
  same logic the workflow panel already uses.

The picker exists twice — a compact chip grid in the Toolbar and the full annotated
list in ⚙ Appearance ▸ Map plates. Both read the same `MAP_THEMES`, so they cannot
drift; only the presentation differs.

**The layer taxonomy was fixed in the same change.** `layerGroups` misfiled in both
directions: "Biosphere" held `climate` (Köppen is atmospheric), `soil` (pedosphere)
and `habitability` (a human settlement score), while "Ocean" held four biological
layers plus `storm`, which is a cyclone belt. The cause was under-population — only
TWO layers are genuinely biological and non-hazard, so the group was padded to a
plausible size with things that aren't biology. Six groups now: Terrain · Ocean ·
Atmosphere · Climate & Biomes · Settlement · Hazards. `ridges` is reachable for the
first time — it had always existed in `ActiveLayer` and in `render_tile`
(`render_ridges`) but belonged to no group.

---

### 8.18 The palette is served, not copied (`commands/palette_commands.rs`)

The legend used to keep **hand-maintained copies** of the renderer's colour tables —
four of them, across three files, none checked against the Rust that paints the
pixels. §8.12 already warned about this for `biome_color`/`BIOME_SWATCH`. They
drifted anyway, in two measured ways:

- **The Elevation layer's sea key ran BACKWARDS.** `ElevationLegend.SEA_BANDS` was
  copied from `render_land`'s bathymetry, but `render_elevation` drew its own ramp
  `(10+d·10, 25+d·30, 70+d·100)` — dark shelf *brightening* to abyss. Reading a
  deep-ocean colour off the map and looking it up in the key landed you on "Shelf".
- **The land bands implied a linear scale the ramp never had.** Six equal blocks
  labelled 0/1500/3000/5000/7000/8848 m described stops that actually fell at
  1327/3097/5309/7521 m.

`get_render_palettes` serves `ELEVATION_STOPS`, `BATHYMETRY_STOPS`,
`TEMPERATURE_STOPS`, `PRECIP_BANDS` and the Köppen/biome/soil class colours straight
out of `tile_image.rs`. **This removes the second copy rather than testing it** — the
legend cannot be wrong about the map without the map being wrong about itself.

Three rules:

- **Never reintroduce a hand-copied colour table in the frontend.** If a legend needs
  a colour, it comes through `usePaletteStore`. A test comparing two copies only
  catches drift after someone remembers to write it; having one copy cannot drift.
- **A ramp is DATA, not a chain of branches.** Every continuous ramp goes through
  `ramp_lookup` over a `(position, colour)` stop table, which is exactly what lets the
  same constants serve both the renderer and the legend. A ramp written as `if e <
  0.15 { … } else if …` cannot be served, and that is how the old drift started.
- **Position a legend's labels at each stop's TRUE value**, never at even intervals —
  even spacing is what made the old land key misreport by up to ~520 m.

**Cross-blended hypsometric tints** (§8.18 companion, `lowland_tint`): below
`LOWLAND_TINT_CEILING_M` (1200 m) the elevation/terrain tint also carries CLIMATE —
desert lowland reads khaki, rainforest green, tundra grey — converging on the shared
ramp above it (Patterson & Jenny, *Cartographic Perspectives* 69). The climate axes
are **temperature and precipitation, both continuous**, never the categorical Köppen
code: keying on Köppen would draw every class boundary as a hard colour edge and a
reader would take those edges for terrain, which is the artefact the technique exists
to avoid. Guarded by `cross_blended_tints_converge_with_height`, which asserts BOTH
halves — that three climates differ at sea level, and that they are bit-identical
above the ceiling. The legend must keep declaring this, or it misreports the lowland.

`LayerLegend.tsx` covers the six layers with an exact key (elevation · terrain ·
temperature · sst · precipitation · climate). The layers whose ramps are still
written inline in Rust are **deliberately left without a key** rather than given an
invented one — a legend that guesses is how this broke the first time. They are
served by the StatusBar hover readout, which reports the real value under the cursor.

**Swipe compare** (`uiStore.compareLayer`/`comparePos`): a second layer drawn over
the same ground, clipped to the right of a draggable divider. Every causal chain in
this app is a two-layer question — precipitation against elevation for rain shadow,
currents against temperature, biomes against Köppen — and they were previously
answered by flipping back and forth from memory. Two rules: the clip is computed in
WORLD space by converting the divider's screen fraction back through the viewport
(the canvas is mid-transform at that point, so a screen-space rect would be wrong),
and the divider is its OWN DOM element with its own pointer handlers, so it cannot
interfere with the canvas's pan/paint logic. The compare layer draws through the
same tile cache and LOD as the base, so a swipe costs one extra blit per visible
tile and nothing else.

**Class isolation** (`RenderCtx.isolate`): clicking a class in the Köppen or biome
key keeps that class in full colour and desaturates the rest. The selected code
rides in the LAYER KEY (`"biomes#iso=12"`), which is why it needed no cache change —
`TileManager` already keys by layer string, so an isolated view caches and
invalidates as its own layer, and a client that knows nothing about isolation still
asks for plain `"biomes"`. It is done in the RENDERER because only the renderer
knows each cell's class; matching colours back in canvas would be slow and, now that
the thematic plates carry relief shading, simply wrong. `split_isolate` degrades a
malformed key to the plain layer rather than erroring — a bad key must never blank
the map (`isolate_layer_keys_parse_or_degrade`).

Guarded in Rust by `koppen_colors_are_distinct` (Dsc and Dsd shipped IDENTICAL, so
two zones rendered as one), `elevation_ramp_is_monotone_in_lightness`,
`bathymetry_darkens_with_depth`, `precipitation_bands_are_sequential_and_never_neutral`
and `temperature_ramp_pivots_on_freezing`.

---

### 8.19 Goods localities — the agricultural/biological hierarchy

Trade goods get what minerals already had (§8.16): belt → LOCALITY → cell, the
same two-level structure `deposits.rs` uses, for every enabled `Global`/`Local`
good (`Deposits`/`Manufactured` goods are out of scope — F2's whole premise is
that minerals already have their own, better hierarchy).

- **Rivers as a placement factor** (Slice 1, F6). `biological::RiverContext`
  (built ONCE per world — the same discipline `deposits::GeoContext` already
  applies) carries distance-to-any-river, distance-to-NAVIGABLE-river and
  delta/floodplain membership, multi-source BFS (§8.9 rule 1). `river_multiplier`
  is a MULTIPLIER on an existing score, never a replacement (§5.4 of the plan) —
  `floodplain`/`irrigation`/`riverbank`/`float_out` weights, wired into the
  specific built-in goods the plan names (rice, cotton, wheat, sugar, indigo,
  dates, paper's papyrus branch, honey, hides, timber, hardwoods, furs) and
  exposed as four new `Envelope` fields (serde-defaulted to 0 — no effect) for
  custom goods. `good_score`/`envelope_score` both take `rc: Option<&RiverContext>`;
  `None` (the Goods Editor's live preview, which has no rivers to hand) is a true
  no-op, not an approximation.
- **Marine inshore/bank split** (Slice 2, F5). `GoodSpec.marine_band`
  (`Either`/`Inshore`/`Bank`, serde-defaulted to `Either`) narrows the old
  undifferentiated `sea_coastal` gate to a STRICT SUBSET of itself —
  `marine_band_ok` — so an `Either` good's placement is byte-identical to before
  this slice. Shipped defaults (`default_marine_band_for`): inshore = pearls,
  coral, bay_salt, tyrian_purple, amber; bank = stockfish, herring, whaling.
- **The locality generator + full modulation** (Slice 3, D1/D5/D6,
  `localities.rs`). `GoodLocality { good, x, y, radius_km, grade, extent, name,
  river_fed }` — deliberately the same shape as `deposits::Deposit`. The size
  ladder (§2.1 of the plan): luxury locality 175 km (wine, silk, spices, cacao,
  cloves, pepper) · pastoral/secondary 400 km (wool, hides, horses, timber,
  tobacco) · staple region 900 km (grain, rice, furs, barley, millet) — every
  other good falls back to a tier from its own `Distribution`/rarity, so nothing
  is left without an answer. Full modulation: `belt[i] = max(FLOOR, belt[i] *
  (FRINGE + (1-FRINGE)*influence[i]))` — `FLOOR` is the entire safety mechanism
  (D5's own risk 5.1): a belt cell already producing never falls to literal zero,
  however far it sits from every locality core. Runs BEFORE `dilate_belt` so the
  trade-reach rings spread from the already-modulated belt.
- **Notable naming** (Slice 4, D8). Localities at/above a quality threshold draw
  a deterministic name from `names::gen_name` — the SAME per-cell hearth lookup
  settlements already use, so a locality's name is in the local culture for free,
  no new naming machinery.
- **Persistence.** `metadata["good_localities"]` (JSON), exactly parallel to
  `metadata["deposits"]` — no tile-column change (rule 7). Written by
  `commands::sim_commands::persist_goods_placement`, the one helper all four
  `compute_trade_goods` call sites now share.
- **Production wiring** (Slice 7, D2). `compute_economy`
  (`commands/query_commands/economy.rs`) reads `good_localities` exactly the way
  it already reads `deposits` — a hub's quality for a good blends toward the mean
  grade of any locality inside its catchment (50% weight; `share`-based quality
  still carries the other half, since Slice 3's modulated belt VALUES already
  partly reflect locality quality — a blend, not a replacement, per D2).
  `campaign_province_potential` (`campaign_commands/province.rs`) exposes
  `ProvincePotential.localities: Vec<ProvinceLocalityDot>` alongside the existing
  `.deposits`, and `ProvinceGoodPotential` gained `has_locality`/
  `mean_locality_grade`/`locality_count`, mirrored in `types/campaign.ts`.
- **Slice 0's own gate**: `sim::step8_biological_goods::goods_validation`
  (test-only) builds a real, moderate-sized procedural world end-to-end (plates
  through biological — NOT the synthetic `CampaignSim` fixture `economy_
  validation.rs` uses) and asserts no enabled `Global`/`Local` good places a belt
  that reaches zero settlements' catchments. `Deposits`-distribution goods are
  explicitly OUT of that hard floor (F2 — they have their own, different
  coverage guarantee, `no_shipped_mineral_places_nothing`); a handful of the
  rarest deposit goods missing every settlement's catchment at this diagnostic's
  deliberately modest world size is printed as a FINDING, not asserted. One
  pre-existing belt good, `dyes` (murex purple, untouched by any Slice 1-4
  change — verified), is named as an explicit, documented exception rather than
  either silently loosening the floor or leaving the gate permanently red.
  **Do not add a process-global `cultures::set_active` call to a test** — `cargo
  test --lib` runs the suite in parallel within one process, and this diagnostic
  originally raced `econ_inheritance_rules_fragment_differently` /
  `econ_scorecard_is_deterministic` by mutating that exact global; it now relies
  on `names::gen_name`'s legacy-culture-grid fallback instead, which is
  deterministic on its own.

**The overlay (Slices 5-6) — two layers, at full resolution.** `compute_good_regions`
rasterised a belt onto a COARSE grid — `f = grid_w / 450`, about 8×8 world cells at
the default size — and the frontend filled each block. A block holding one land cell
of belt painted all sixty-four of its cells, so a belt's edge was a staircase that
ignored the coastline and spilled into the sea. No amount of shading could fix it: the
information was not in the payload.

Full resolution IS the coastline clip. `compute_good_belt_masks` copies the belt
column verbatim above `COVERAGE_MIN_U8` and consults no land mask at all, because a
`Global`/`Local` belt good's byte is already exactly zero on the wrong side of the
coast — every one is placed through `envelope_score`, whose first act is the domain
gate. `goods_validation::a_belt_never_crosses_the_coastline` asserts exactly that
claim, against the shared constant rather than a copy of it, because the render path
has no opinion about where the coast is and would silently start painting into the
ocean again if it stopped holding. The same test PRINTS a finding it deliberately
does not assert: `Distribution::Deposits` goods never touch `envelope_score` (§8.16
places them from tectonic setting), so `CoastalMarine` can put a salt pan a cell into
the tidal water — measured at 300×150 as bay_salt 115 cells, tyrian_purple 16,
ambergris 1. That is a `deposits.rs` question, not a render bug, and clamping it here
would hide it.

Two layers, split because they compress differently. COVERAGE ("can it grow here")
is boolean at full resolution and run-length encoded — a belt is contiguous, so a
world-spanning staple region costs a few thousand runs, not one per cell. QUALITY
("is it fine here") is the belt value on the OLD coarse grid, because a wash needs no
per-cell precision; it is painted only where coverage says so, so the coarse wash
still ends exactly on the coastline. Both are switched independently in the Toolbar's
Trade Goods section.

Four rules for anyone changing this:

- **The quality scale is ABSOLUTE and shared by every good** (D10). The belt's own
  0..1 value, never rescaled against that good's own maximum — otherwise a good whose
  whole belt is mediocre gets promoted to full colour just for being its own best. The
  stops live once, in `palette_commands.rs`, and are SERVED (§8.18). Until they
  arrive the overlay draws coverage only rather than inventing a ramp.
- **The scale has TWO paints, same numbers.** The default mixes toward the good's
  own colour, which is unreadable for a good whose hue is already close to the pale
  ground tint — a real user report. `GOOD_QUALITY_HEATMAP_STOPS` (dark blue → red)
  is an alternate reading of the identical belt value, switched per-session via
  `uiStore.goodQualityHeatmap` (Toolbar → Trade Goods → "🌡 heatmap", only shown
  while the quality layer itself is on). `GradeStop`/`good_quality_grades` serves
  the SAME `deposits::grade_label` vocabulary (coarse/ordinary/good/fine/exquisite)
  ore workings already use, at heatmap-exact colours, so `GoodQualityLegend`
  (`ui/world/LayerLegend.tsx`) can show swatches guaranteed to match the map. It
  docks to the opposite corner from the main `LayerLegend`, since it keys off the
  `goodQuality` OVERLAY rather than the active base layer and can be showing at the
  same time as either.
- **A very large belt is downsampled by MAJORITY, never by "any".** Above
  `MASK_MAX_PX` a canvas per good becomes real memory, so the mask reduces — and a
  reduce that takes a block when ANY sub-cell is covered puts the belt back over the
  water one block at a time, which is the original bug in miniature.
- **The FILL is exact; the BORDER is traced at a bounded step.** The fill is what
  carries the coastline. The outline is a stroke a pixel or two wide, so tracing it on
  millions of cells buys nothing and costs a Path2D with a hundred thousand segments.
- **Never re-derive the belt's shape from province polygons** (D3). A wine belt
  crossing three provinces is one belt; snapping it to political lines would make a
  physical fact look administrative.

`GOOD_MASK_DIR=/tmp/g cargo test --lib dump_good_belt_mask_sheet -- --ignored
--nocapture` writes a PNG per sampled good over the world's land/sea, through the
REAL `build_belt_mask` and the frontend's own decode — use it to answer "does the belt
end on the coastline" instead of arguing it, the same way `dump_biome_swatch_sheet`
serves the biome palette.

**The province plate (Slice 6)** is the other half. Land use and tenure are SHARES
and must dither (rule 17), but a `GoodLocality` is not a share — it carries a real
cell and a real `radius_km` — so it draws as a real square at its real position,
clipped to the province footprint, opacity from `grade` and hue from `GOOD_DEFS`. A
MARINE locality sits on shelf water where the province raster is `NO_PROVINCE`, so
`campaign_province_potential` attaches it to the nearest coast within
`MARINE_ATTACH_CELLS` and flags it `sea`: it draws dashed in the adjacent water, is
excluded from the province's own `has_locality` aggregate, and confers **no maritime
territory** (D4).

---

### 8.20 Goods: climate adherence, origins, endemics, terroir

Four changes to how a good gets onto the map, plus the report that makes a
failed placement visible. Full account (including this review's own three wrong
first readings) in `docs/WORLD_REALISM_REVIEW.md`.

**The `clim_base` fold is a FALLBACK, never a pre-pass.** `good_score` used to
fold the dry-winter/dry-summer Köppen variants onto their humid equivalents
BEFORE its match, which made every arm naming `CWA`/`CWB`/`CWC`/`DW*`/`DS*`
**unreachable**. Tea and coffee both name `CWB` — subtropical highland, dry
winter: Darjeeling, Yunnan, the Ethiopian highlands — and both scored exactly
**0.0** there, placed instead by weak fallback arms in the wrong climates. Now the
RAW zone is scored first and the fold applies only if it yields nothing.
`envelope_score` follows the same rule; before this, custom goods saw raw Köppen
while built-ins saw folded Köppen, so one cell was scored under two different
climate labels depending on which scorer ran. Gates:
`dry_winter_zones_are_reachable` (the claim) and
`the_humid_fold_still_applies_as_a_fallback` (the fix didn't cost the fold its
purpose).

**`GoodSpec.origins`** — how many INDEPENDENT homelands a `Local` good seeds
(serde-default 1 = the old single-homeland behaviour, so every save is
unchanged). Multi-origin is the historical norm: pepper from Malabar *and*
Sumatra, cinnamon from Ceylon *and* China, cotton independently domesticated in
India, Peru and the Levant. Each origin repels the earlier ones through the same
dispersion penalty already applied between different goods. Two rules: the
extreme-rarity homeland cap is a budget for the good AS A WHOLE, split between
origins (or a rare good doubles its own world supply); and only the FIRST origin
may fall back to the least-bad cell — a second origin that can't clear the
threshold simply doesn't exist on this world.

**`LandmassContext` + `Distribution::Endemic`** — the connected-component pass
this codebase never had. `Domain::Island` was `distance_to_ocean < 0.20`, i.e.
*near-coast land*, which matched the coastal fringe of every continent, so an
"island" good was really a coastal good. One 8-connected BFS, wrap-aware, built
ONCE per world (same discipline as `RiverContext`/`GeoContext`). An `Endemic`
good is confined to ONE landmass with the island-jump DISABLED — the Banda
nutmeg case: nutmeg grows across the wet tropics, but the tree only grew on ten
islands totalling 60 km², and `rarity` cannot express that (it makes a good
scarce *everywhere* rather than abundant in one place and absent elsewhere).
Six ship: nutmeg, mace (sharing an envelope — two products of one tree),
dragon's blood, camphor, benzoin, sandalwood.

Three rules learned by MEASUREMENT here, each a silent-vanish failure:
- **An island threshold must be stated in km², not cells** (`ISLAND_MAX_KM2`).
  A cell is ~11 km at 3600×1800 and ~133 km on a test world, so a fixed cell
  count meant "Great Britain" on one world and "most of Eurasia" on another.
- **DOMAIN and DISTRIBUTION must not both gate the same thing.** Shipping the
  endemics as `Domain::Island` zeroed their SCORE on every continental cell
  before the distribution could choose a home — all six measured **zero cells**.
  Domain says where the plant can grow (a wet tropical coast); distribution says
  how it is confined (the smallest landmass that scores, preferring a true
  island).
- **The endemic home is chosen up front, not filtered per cell.** That is what
  makes the guarantee unconditional: if the climate exists anywhere, the good
  gets exactly one home; if it exists nowhere it is honestly absent.
- **The chooser must REMEMBER what earlier endemics took.** It ranks candidates
  "smallest scoring landmass, preferring a true island" — a pure function of the
  world — and the six shipped endemics share a wet-tropical coastal envelope by
  design, so every one of them returned the SAME answer and the whole spice-island
  layer collapsed onto one rock. It now ranks `(claimed, island?, area)`: an island
  another endemic holds is the LAST resort. Still a preference, never a filter, so
  the coverage guarantee above is untouched. The EXCEPTION is the mechanism's point:
  nutmeg and mace are the aril and the seed of ONE tree and ship sharing an envelope
  deliberately, so `score_signature` (a fingerprint of the quantized suitability
  field) tells "one plant sold as two products" from "a different plant that likes
  the same weather" — no new spec field, no hard-coded pair table.
  Gated at the CHOOSER (`biological::tests::endemic_goods_take_different_islands`),
  not end-to-end: written against the 300×150 reference world the gate failed on the
  FIXED code, because at that resolution `ISLAND_MAX_KM2` puts the island threshold
  at 14 cells against a smallest landmass of 22, so the fixture qualifies ZERO
  islands and offers one candidate. A gate that fails identically either way measures
  the world, not the mechanism (§8.24c's own lesson). What remains in
  `goods_validation` is an honest diagnostic, `endemic_homelands_diagnostic`.

**`GoodSpec.soil` / `GoodSpec.relief` — the FINE-GRAIN terroir terms.** Every
other scoring term varies over HUNDREDS of km (Köppen zone, temperature,
precipitation, |latitude|, normalized elevation, fertility — itself a smoothed
blend), which is why a belt rendered as one smooth continent-sized wash: nothing
varied at the 2–10 km scale at which real crop distributions are mottled.
`soil_type` was computed by phase 6 and read by NOTHING in the goods layer;
slope was never computed. Three rules:
- **They live on the SPEC, not on `Envelope`.** They must apply to built-ins
  (scored by the hardcoded matcher, which has no Envelope) and customs alike;
  putting them in `Envelope` silently REPLACED a built-in's whole climate scorer
  with an envelope holding only these two terms.
- **Soil is a preference, never a veto.** An unclassified cell scores 1.0 (no
  information is not bad ground — the same discipline an empty kin roster gets);
  a classified-but-unlisted soil keeps `SOIL_UNLISTED`.
- **`TERROIR_FLOOR` remaps the whole multiplier into `[0.45, 1.0]`.** Applied
  raw it pushed `tea` and `saffron` under the seed threshold and both placed
  literally nothing. Terroir shapes a belt's TEXTURE; it must not decide whether
  the belt exists — the same call the locality pass's FRINGE/FLOOR already made.
  `saffron` is deliberately NOT in the terroir table: already bounded by climate,
  elevation AND latitude, one more gate moved it off every settlement's catchment
  and tripped the Slice 0 coverage floor.

**The two RENDER fixes that went with this.** Both were the same bug in two
places — a real per-cell field being drawn at a coarser resolution than it has:

- **The world quality overlay carried TWO resolutions.** Coverage was a
  full-resolution 0/1 RLE, but the quality wash clipped to it still rode the old
  ~8-cell coarse grid, so a belt read as blocky value steps inside a sharp
  coastline outline. `GoodBeltMask.coverage_rle` is now `quality_rle`: the SAME
  runs carry a 4-bit quantized belt value (0 = uncovered, 1..15 = quality), so
  coverage and quality are one layer at one resolution. Quantizing is what keeps
  the runs long — a belt varies smoothly, so neighbours share a bucket and the
  payload stays close to the old boolean RLE. `quality`/`qw`/`qh`/`coarse`
  survive only as the index space the per-block SUBTYPE classification
  (grain species / paper source) is addressed in. Gate:
  `quality_levels_never_swallow_a_covered_cell` (a covered cell may never
  quantize to 0, and the scale is monotone).
- **The province plate drew a locality as a true-to-scale square.** The size
  ladder is in km — a staple region is 900 km — while a province is 200-400 km
  across, so one grain locality filled the entire plate and the province read as
  one flat block. Where a belt MASK exists (plate 6a already draws the real
  per-cell area) the locality is now reduced to what only it can say: a small
  CORE diamond at its real cell plus its name. Where no mask exists (an older
  world, or a good with no belt here) the full square is still the honest
  reading and is kept.

**The placement report** (`GoodsPlacementReport`, `metadata["goods_report"]`,
served by `get_goods_report`, shown by `ui/goods/GoodsReportPanel.tsx` — opened
automatically when the Biological step finishes, and reopenable from that step
afterwards since it is persisted) — per good: cells, land share, origins actually
seeded, localities, notable names, mean grade. Its reason for existing is the
FLAGS: `absent` (placed nothing), `fallback_seed` (placed only because the seeder
fell back to the least-bad cell — this world may have no suitable climate),
`ubiquitous` (a non-staple over 25% of the world), `single_cell`. Before it, a
good that silently failed to place was invisible until someone went looking for
it on the map.

**Retired as duplicates** (disabled, never deleted — fixed indices in
`TileData.goods`, rule 7): `gemstones` (a generic gem alongside ELEVEN specific
stones, and §8.16 already repurposed `gem_deposits` to mean ore districts) and
`dyes` (marine murex, i.e. the same product as `tyrian_purple`, and the one good
`goods_validation` carried as a standing coverage-floor exception).

---

### 8.21 Natural colour & the shared relief core (`render/natural.rs`)

The `natural` layer is the reference-atlas view: **land coloured by what grows on
it**, sea by depth, both shaded. It reads `biome` (41 classes, §8.12), elevation,
temperature, precipitation and `snow_frac` — all already computed — and adds no
tile column and no sim phase. Gated on step 6 like the biomes plate; a world
without biomes falls back to a continuous climate tint rather than to blank, the
same discipline as `koppen_fallback_biome`.

**It does not reuse `biome_color`, and that is the whole design.** That table is
THEMATIC: `biome_colors_are_distinct` holds all 41 classes apart at a CIELAB floor
so a reader can separate them in a legend. Natural colour wants the opposite —
real vegetation grades continuously, and a palette engineered for class separation
renders a satellite-style map as a discretised poster with a visible edge at every
biome boundary. So there is a second, deliberately OVERLAPPING table, and
`neighbouring_vegetation_classes_stay_close` asserts that overlap so a
well-meaning future "improve the contrast" edit fails loudly. Class edges are
further dissolved by mixing back a continuous temperature/precipitation tint —
the same trick and the same reasoning as `lowland_tint` (§8.18).

**The shared relief core** (`tile_image::relief_at`) now serves both shaded base
layers. Three changes from the single-lamp Lambertian it replaced:

- **Ambient occlusion** from local concavity, which is orientation-INDEPENDENT, so
  a valley reads as a valley under any lighting.
- **Shaded bathymetry** (`sea_shade`). The sea was an unshaded ramp beside a
  hillshaded continent.
- **NOT a fill light.** One shipped and was REVERTED — see below.

Three things measured here, all of which look like tuning and are not:

- **`AO_REF` is the difference between ambient occlusion and film grain.** Set to a
  plausible-sounding 44 m ("valleys are shallow"), AO speckled the ENTIRE map:
  `apply_micro_relief` deliberately dithers every land cell by ±14 m, and against
  an 8-neighbour mean that per-cell dither *is* the concavity signal, so AO
  resolved to ±16% white noise per cell. Rendering with `AO_AMP = 0` isolated it
  conclusively. It is now 240 m; anything under ~150 m re-admits the grain.
- **A FILL LIGHT was shipped, and it was a REGRESSION.** The theory was Imhof's
  (one NW lamp leaves every SE-facing slope at a flat tone), and it survived
  `cargo check`, 20 render tests and a whole-world PNG — then read as obviously
  worse the moment anyone looked at a magnified mountain crop. A fill lamp
  brightens exactly the shadows that CARRY the relief. Four configurations
  rendered side by side (`SHEET_TAG`): no fill > 225°/0.18 > 135°/0.26, the last
  being what shipped. A fill light is a technique for a 3-D scene with real
  geometry; on a shaded DEM whose only signal IS the shadow it subtracts. The
  lesson generalises past this constant: **a whole-world thumbnail is not enough
  to judge shading** — `dump_natural_sheet` now also writes a 3× magnified crop of
  the most mountainous window, because the regression was invisible at world zoom
  and unmistakable at 3×.
- **A per-cell detail dither was built here and REMOVED.** A tile is exactly one
  pixel per cell, so there is no sub-cell space for synthetic detail to live in and
  it resolves as film grain rather than as terrain. Recorded so it is not
  attempted again: detail below the cell needs somewhere to be DRAWN — tiles
  rendered at higher pixel density than cell density, i.e. an inverse LOD pyramid.
  No shading trick substitutes for the missing raster.
- **The bathymetry ramp's dark end was a void.** It fell to (5,15,46) by depth 0.65
  and near-black at 1.0, while `compute_sea_depth` saturates about 20 cells from
  any coast — so on a full-size world essentially ALL open water sat in the black
  tail, and the new seafloor shading had nothing to modulate. Rebalanced to keep
  tone all the way down, still strictly monotone in luminance.

**`NATURAL_SHEET_DIR=/tmp/n cargo test --lib dump_natural_sheet -- --ignored
--nocapture`** builds a REAL world (plates through biomes, mirroring `sim_run_all`
— rule 11) and renders it through the real `render_tile` path, writing
`world_natural.png` and `world_terrain.png` plus a per-biome census. Use it to look
at a palette or shading change instead of arguing about it — it is what caught both
the AO grain and the black ocean, neither of which any assertion would have found.

What it also makes unmissable is that the remaining problems are in the SIM, not
the renderer: land relief is texturally uniform because `generate_elevation`
uses one global noise recipe (`f_base`/`f_range`/`f_hill`, `RIDGE_AMP`,
`HILL_AMP`) for every continent. (The coastline-follows-Voronoi-edge and
straight-plate-boundary-ridge defects this paragraph used to name here were
Terrain 2.0's own slices 3-4 (shipped; §8.23b) — fixed and gated
by `coastline_departs_from_the_plate_boundary`.)

---

### 8.22 Elevation styles (`render/tile_image.rs`)

The default "elevation" layer is a flat, UNSHADED hypsometric tint and
"terrain" is the one shaded hillshade — two names for what an atlas treats as
one decision (how to paint height) times two independent axes (which palette,
how much relief). A STYLE is that pair, selected in the layer key exactly like
class isolation (`split_isolate`, `"biomes#iso=12"`):
`"elevation#style=alpine"` rides the same string the frontend's tile cache
already keys on, so a styled request caches, invalidates and degrades (an
unknown style name falls back to the plain layer) with zero cache-layer
change. Parsed by `split_style`, dispatched by `render_elevation_styled` —
ONE function for every style, keyed entirely off a `StyleParams` struct (land
+ sea ramp, classed-vs-smooth, climate-tint strength, real-`snow_frac` blend
strength, AO/contrast/shadow-floor/light-altitude, warm-vs-cool shadow tint,
sea relief amplitude), so a new style is DATA, not a new render function.

Seven ship, each a real cartographic convention rather than a colour
experiment: **Layer Colouring** (classed Bartholomew/Times hypsometric bands —
also what `mapThemes.ts`'s "Relief & Height" plate now uses, giving it a real
identity distinct from "Physical" instead of reusing the flat unshaded
default); **Alpine** (Imhof's neutral Swiss relief-forward palette, reading
the world's own per-cell `snow_frac` field for its snowcap rather than
inferring it from height alone); **Arid** (warm sand/canyon palette, a lower
key-light altitude for longer raking desert shadows, a sepia shadow tint);
**Polar** (cool ice-blue palette, a very low key-light altitude, strong
`snow_frac` blending); **Analytical** (monochrome — colour carries ZERO
elevation information, relief alone does, maximum AO for a scientific
hillshade); **Antique Plate** (sepia/parchment engraved-atlas look, classed
bands, a warm shadow tint); **Abyssal** (a bathymetry SHOWCASE — land muted
to a low-contrast neutral so it recedes, `sea_relief_amp` boosted well past
the default so Terrain 2.0 slice 5's ridges/trenches/seamounts read as the
map's actual subject).

Two rules:

- **Served, not copied** (§8.18 applies here exactly as it does to every other
  ramp). `elevation_style_palettes()` exposes each style's own land+sea stops;
  `get_render_palettes()` serves them as `elevation_styles` so
  `LayerLegend.tsx`'s elevation legend swaps to the ACTIVE style's real ramp
  (and its `classed` flag, so a stepped style draws a stepped legend) rather
  than guessing a second copy.
- **A style is a VIEW, exactly like a map plate** (rule 14/§8.17) — it changes
  what the elevation/terrain layers look like, never what the world IS.
  `elevationStyle` lives in `uiStore` beside `isolateClass`, rides the layer
  key the same way, and is cleared by any manual change (§8.17's third rule)
  so a picked style can't silently survive switching to an unrelated plate.

`cargo test --release --lib dump_elevation_style_sheet -- --ignored
--nocapture` (env `ELEVATION_STYLE_SHEET_DIR`, `ELEVATION_STYLE_SEED`) builds
one real generated world and renders it through EVERY style via the real
`render_tile_full` dispatch — one full-world PNG per style plus the two
default baselines, and a numbered contact-sheet montage — the same
"render it for real, don't argue about it" discipline `dump_natural_sheet`
and `dump_biome_swatch_sheet` already established.

---

### 8.23 No valley carving — and why three attempts at it failed

**Phase 2 does not carve valleys. In any generator, by any mechanism.** Three
separate attempts to make channel-carving look right on an Earth-sized world all
failed, and the record of them is the point of this section — the fourth attempt
should not start from scratch.

What was removed, and what each drew on the map:

- **`carve` / `fine_carve`** — an INVERTED ridged-multifractal field subtracted
  from the elevation. An inverted ridged field is a **dendritic tree by
  construction**, so this drew a branching dark scratch network across every
  continent. `fine_carve` was the worst because it was an ABSOLUTE subtraction:
  full strength on flat plains, which is exactly where a drainage tree painted
  over an otherwise smooth interior is most obvious.
- **`stream_power_erosion`** — priority-flood + flow accumulation + `K·A^m·S^n`
  (Whipple & Tucker 1999). Correct landscape-evolution physics at the resolution
  it is meant for, and wrong here: a cell is `KM_EQUATOR / w` wide (11 km on the
  default grid), so the valley it models is SUB-GRID — the Grand Canyon is 16 km
  across and would not fill one cell. At cell resolution it cut one-cell trenches
  down every D8 path plus the parallel single-cell rills D8 routing always makes
  on a planar slope.

**The three attempts, in order, so the pattern is visible:**

| | what it changed | why it failed |
|---|---|---|
| `3943136` | noise `carve` 0.16 → 0.05, thermal slump strengthened | aimed at the wrong term — measured per stage, the noise carve leaves notch density at 0.05% of land |
| `e66dc9f` | stream power scaled + spread to its physical scale; thermal scan-order fixed; a grid-scale relief budget | cut grid-scale texture 82–93% **and the map still showed the tree** — subtler lines are still lines |
| *this* | every carving mechanism deleted | — |

The lesson the second attempt paid for: **the STRUCTURE is what reads as wrong,
not the amplitude.** A dendritic pattern at 40 m is still a dendritic pattern.
Damping it is not the same as not drawing it.

**A measurement caveat that matters more than the fix**, because it is why the
statistics said "fixed" while the map plainly was not: `notch_metrics`'s 120 m
threshold is calibrated to the HILLSHADE (half of `AO_REF`), but the flat
`elevation` tint has no shading at all, and a systematic pattern shows there at a
far shallower amplitude. **Judge a tint layer on a tint render, not on a
hillshade statistic.** `dump_erosion_sheet` now writes `elevation` alongside the
analytical hillshade, and takes `EROSION_MODEL=shape` for the TEMPLATE path —
which is what anyone who imported a real-world coastline is actually running, a
different generator with its own carve terms.

**A second negative result, recorded so it is not built into a gate:**
`largest_notch_component` (the largest 8-connected chain of notch cells — meant
to separate "rough" from "a drawn tree") **does not discriminate on the plate
model**. Re-adding `fine_carve` to a clean build measures 0.110 / 0.177 / 0.287%
at 60 / 25 / 12 m thresholds against a clean 0.122 / 0.186 / 0.299% — carved
reads *lower* at every threshold. `limit_grid_scale_relief` normalises the
one-cell band to a fixed RMS, so no amplitude statistic downstream of it can see
structure. There is therefore **no cheap statistical gate for this**, and none
was shipped: a gate that passes either way is worse than none.

**What remains** is `thermal_erosion` — hillslope slumping, which only ROUNDS and
never incises, and whose scan-order fix and gate from the second attempt are kept
— plus `limit_grid_scale_relief` (below). Relief comes from the noise stack and
the tectonic terms.

**Rivers are unaffected.** Phase 5 runs its own priority-flood fill and derives
channels from the finished surface; it never needed pre-cut channels to bed into.
`priority_flood_flow` in `elevation.rs` is now `#[cfg(test)]`, kept only so
`terrain_metrics` can still report drainage density.

---

### 8.23b The grid-scale relief budget — relief at the scale a cell can hold

A cell is a `KM_EQUATOR / w`-wide AVERAGE of the landscape inside it, so relief at
the ONE-CELL scale is by construction relief the grid cannot resolve. Real
topography sampled at 11 km is smooth at that scale: adjacent samples differ
because the land is going somewhere, not because each sample has a private bump.

Phase 2's field did have private bumps, and `redistribute_elevation` amplifies
them along with everything else — measured on a plate world it multiplies landform
relief **×7.7** and grid-scale relief **×8.9**, so whatever the noise stack leaves
is what the finished map is textured with. At 1800×900 that came out at **80 m
RMS** against `AO_REF`'s 240 m: AO texture drawn on every cell of every plain.

`limit_grid_scale_relief` caps the one-cell band at `GRID_RELIEF_BUDGET_M` (16 m),
self-calibrating — measure this world's grid-scale RMS and scale the detail band to
fit — rather than by a hand-tuned amplitude, the same discipline `need_scale` and
`prov_good_yield_scale` use in the campaign half. It only ever SMOOTHS (a world
inside budget comes back bit-identical), touches only the one-cell band, and runs
BEFORE `apply_micro_relief` so the deliberate ±14 m dither survives.

Measured, with the erosion of §8.23 also gone (one-cell RMS concavity · notch
density · landform relief, @1800×900):

| model | before any of this | now |
|---|---|---|
| plates | 102.7 m · 2.77% · 2178 m | **19.8 m · 0.30% · 2174 m** |
| shape | 98.6 m · 3.84% · 2261 m | 18.2 m · 0.19% · 2274 m |
| cordillera | 340.2 m · 7.83% · 2084 m | 21.2 m · 0.27% · 2172 m |
| ridged | 125.8 m · 3.69% · 2249 m | 17.3 m · 0.15% · 2293 m |

**Landform relief moves under 2% on every model** — that pair is the whole claim,
since a limiter that also flattened the mountains would "succeed" on the first
number and ruin the map.

Four rules for anyone changing this:

Five rules for anyone changing this:

- **A mineral must never silently vanish.** This is the failure mode this codebase
  keeps hitting (`highland_cap` exists for the same reason). Two guards: the
  threshold-loosening loop, and — when a world's plate geometry offers no ground at
  all for a model — a forced fall back to the relief proxy. That is not
  hypothetical: most divergent boundaries on any Earth-like world are OCEANIC, so
  keying `Rift` strictly on the boundary emptied it, and
  `no_shipped_mineral_places_nothing` caught it.
- **Template worlds have NO plate data** (`boundary_type` is empty). Every model
  degrades to a relief-and-continentality proxy;
  `template_world_without_plates_still_places` guards it.
- **Diamond belongs on flat cratons, not peaks.** The old spec said `min_elev: 0.55`
  — the highest mountains — which is exactly backwards.
  `diamond_lands_on_craton_not_on_peaks` guards it.
- **A derived model needs its parent placed first.** `Weathering` minerals are run
  in a SECOND pass after the main loop, rather than reordering it (which would
  change every other good's placement seed order).
- **`GeoContext` is built ONCE per world**, never per good — 45 goods × a full-grid
  BFS would be a real cost. Its distance fields are multi-source BFS (a linear
  sweep from all seeds at once), never an outward scan per cell (§8.9 rule 1).

**Slice 2 (built):** `economy.rs::compute_economy` now reads `metadata["deposits"]`
and attributes each working to the hub whose catchment claims its cell (the same
`claim` map the belt-production pass already builds), so a `Deposits`-distribution
good's quality is the mean `grade` of the workings actually inside a hub's
territory — not, as before, that hub's share of world production (which read
backwards: a big cheap deposit scored as fine stones). Every other good keeps the
old share-based formula.

**Slice 3 (built):** an INI-ish `.txt` importer (`commands/goods_import.rs`,
`import_goods_txt`, wired to a "Import .txt" button in the Goods Editor) adds
minerals to the global library — ADD-ONLY, an id already present is rejected, never
overwritten. Only `[id]` and `name` are required; `domain`/`distribution`/
`deposit_model` parse through the real enums' own serde representation, so the
parser can't disagree with the type. Eight new minerals ship in
`default_custom_goods()` itself (not through the importer — they're the app's own
library): mercury, alum, lapis_lazuli, turquoise, bog_iron, coal, garnet, carnelian
— each exercising a model or mechanic the shipped six/gem-split never did (a
near-single-source district count, a derived weathering mineral, a bog deposit an
elevation floor could never place). Full detail, including what is deliberately
NOT wired (mercury→silver amalgamation, alum→cloth as a hard recipe input — both
real economic changes that need their own `econ_` measurement, not an add-only
slice) in `docs/DEPOSITS_AND_MINING_PLAN.md` slice 3.

> **Slices 4-5, built:** a MINE (`estate_kind == 2`) carries `mine_depth`
> (the real working nearest its parent city) and digging a deep/flooded body
> costs real drainage capital to upgrade (`MINE_UPGRADE_COST_MULT`); mercury →
> silver amalgamation is a real consumable extraction input. A QUARRY
> (`estate_kind == 8`, split off from Mine) is gated by TRANSPORT instead
> (`QUARRY_INLAND_UPGRADE_COST_MULT`), never depth. A body KNOWN to be
> `EXTENT_WEAK` now declines to a floor under pressure (D3); everything else
> still persists. Mining SETTLEMENTS (the Potosí case,
> `maybe_found_mining_colony`) found on a real GREAT/WORLD-CLASS strike, boom,
> and DECLINE rather than die when their food lifeline fails. A settlement's
> trade catchment radius grows slowly with age (`catchment_radius_km`,
> derived, never stored). See `docs/DEPOSITS_AND_MINING_PLAN.md` slices 4-5.

---

### 8.17 Map plates (`ui/world/mapThemes.ts`)

A published atlas never shows one raster on its own. A climate plate is Köppen fill
PLUS the circulation that produces it PLUS the graticule, set in the face that plate
uses; a political plate is a province wash PLUS borders PLUS city dots sized by rank.
What distinguishes one plate from another is the whole COMPOSITION, not the base
colour. The app had 25 base layers and ~30 overlay toggles and no way to express that
— every informative view had to be assembled by hand, from memory, every time.

A `MapTheme` is that composition in state that already existed: one `ActiveLayer`, a
set of overlay keys, and optionally a label-typography theme (§8.11) and an
overlay-line preset. **Twelve plates ship**, ordered the way the pipeline builds the
world: Physical · Natural Colour · Relief & Height · Ocean & Currents · Climate · Hydrology · Ecology ·
Settlement · Peoples · Political · Goods & Trade · Hazards.

Four rules for anyone changing this:

- **A plate is a VIEW, never a decision.** It sets what you SEE, never what the world
  IS — rule 14 restated. Nothing here is persisted and nothing writes a generation
  setting, so switching plates can never alter a world or invalidate a tile.
- **`MANAGED_OVERLAYS` is DERIVED from the plates, never hand-listed.** Applying a
  plate sets each managed key explicitly — on if the plate lists it, off otherwise —
  so a plate lands in a KNOWN composition instead of washing additively over
  leftovers. Deriving the set is what bounds the blast radius: per-good overlays,
  campaign-only layers and anything else no plate mentions are never touched, so a
  plate cannot silently clear work done in another panel.
- **`activeMapTheme` is cleared by any manual change** (`setLayer`, `toggleOverlay`,
  `setOverlayVisible`, `setOverlaysVisible`, `setWorkflowStep`). A chip that keeps
  claiming a plate after the view stops being one is worse than no chip.
- **A plate whose data isn't generated reads dimmed with the step it waits on**, not
  hidden (`requires` + `themeReady`). Seeing what the finished world will offer is the
  same logic the workflow panel already uses.

The picker exists twice — a compact chip grid in the Toolbar and the full annotated
list in ⚙ Appearance ▸ Map plates. Both read the same `MAP_THEMES`, so they cannot
drift; only the presentation differs.

**The layer taxonomy was fixed in the same change.** `layerGroups` misfiled in both
directions: "Biosphere" held `climate` (Köppen is atmospheric), `soil` (pedosphere)
and `habitability` (a human settlement score), while "Ocean" held four biological
layers plus `storm`, which is a cyclone belt. The cause was under-population — only
TWO layers are genuinely biological and non-hazard, so the group was padded to a
plausible size with things that aren't biology. Six groups now: Terrain · Ocean ·
Atmosphere · Climate & Biomes · Settlement · Hazards. `ridges` is reachable for the
first time — it had always existed in `ActiveLayer` and in `render_tile`
(`render_ridges`) but belonged to no group.

---

### 8.18 The palette is served, not copied (`commands/palette_commands.rs`)

The legend used to keep **hand-maintained copies** of the renderer's colour tables —
four of them, across three files, none checked against the Rust that paints the
pixels. §8.12 already warned about this for `biome_color`/`BIOME_SWATCH`. They
drifted anyway, in two measured ways:

- **The Elevation layer's sea key ran BACKWARDS.** `ElevationLegend.SEA_BANDS` was
  copied from `render_land`'s bathymetry, but `render_elevation` drew its own ramp
  `(10+d·10, 25+d·30, 70+d·100)` — dark shelf *brightening* to abyss. Reading a
  deep-ocean colour off the map and looking it up in the key landed you on "Shelf".
- **The land bands implied a linear scale the ramp never had.** Six equal blocks
  labelled 0/1500/3000/5000/7000/8848 m described stops that actually fell at
  1327/3097/5309/7521 m.

`get_render_palettes` serves `ELEVATION_STOPS`, `BATHYMETRY_STOPS`,
`TEMPERATURE_STOPS`, `PRECIP_BANDS` and the Köppen/biome/soil class colours straight
out of `tile_image.rs`. **This removes the second copy rather than testing it** — the
legend cannot be wrong about the map without the map being wrong about itself.

Three rules:

- **Never reintroduce a hand-copied colour table in the frontend.** If a legend needs
  a colour, it comes through `usePaletteStore`. A test comparing two copies only
  catches drift after someone remembers to write it; having one copy cannot drift.
- **A ramp is DATA, not a chain of branches.** Every continuous ramp goes through
  `ramp_lookup` over a `(position, colour)` stop table, which is exactly what lets the
  same constants serve both the renderer and the legend. A ramp written as `if e <
  0.15 { … } else if …` cannot be served, and that is how the old drift started.
- **Position a legend's labels at each stop's TRUE value**, never at even intervals —
  even spacing is what made the old land key misreport by up to ~520 m.

**Cross-blended hypsometric tints** (§8.18 companion, `lowland_tint`): below
`LOWLAND_TINT_CEILING_M` (1200 m) the elevation/terrain tint also carries CLIMATE —
desert lowland reads khaki, rainforest green, tundra grey — converging on the shared
ramp above it (Patterson & Jenny, *Cartographic Perspectives* 69). The climate axes
are **temperature and precipitation, both continuous**, never the categorical Köppen
code: keying on Köppen would draw every class boundary as a hard colour edge and a
reader would take those edges for terrain, which is the artefact the technique exists
to avoid. Guarded by `cross_blended_tints_converge_with_height`, which asserts BOTH
halves — that three climates differ at sea level, and that they are bit-identical
above the ceiling. The legend must keep declaring this, or it misreports the lowland.

`LayerLegend.tsx` covers the six layers with an exact key (elevation · terrain ·
temperature · sst · precipitation · climate). The layers whose ramps are still
written inline in Rust are **deliberately left without a key** rather than given an
invented one — a legend that guesses is how this broke the first time. They are
served by the StatusBar hover readout, which reports the real value under the cursor.

**Swipe compare** (`uiStore.compareLayer`/`comparePos`): a second layer drawn over
the same ground, clipped to the right of a draggable divider. Every causal chain in
this app is a two-layer question — precipitation against elevation for rain shadow,
currents against temperature, biomes against Köppen — and they were previously
answered by flipping back and forth from memory. Two rules: the clip is computed in
WORLD space by converting the divider's screen fraction back through the viewport
(the canvas is mid-transform at that point, so a screen-space rect would be wrong),
and the divider is its OWN DOM element with its own pointer handlers, so it cannot
interfere with the canvas's pan/paint logic. The compare layer draws through the
same tile cache and LOD as the base, so a swipe costs one extra blit per visible
tile and nothing else.

**Class isolation** (`RenderCtx.isolate`): clicking a class in the Köppen or biome
key keeps that class in full colour and desaturates the rest. The selected code
rides in the LAYER KEY (`"biomes#iso=12"`), which is why it needed no cache change —
`TileManager` already keys by layer string, so an isolated view caches and
invalidates as its own layer, and a client that knows nothing about isolation still
asks for plain `"biomes"`. It is done in the RENDERER because only the renderer
knows each cell's class; matching colours back in canvas would be slow and, now that
the thematic plates carry relief shading, simply wrong. `split_isolate` degrades a
malformed key to the plain layer rather than erroring — a bad key must never blank
the map (`isolate_layer_keys_parse_or_degrade`).

Guarded in Rust by `koppen_colors_are_distinct` (Dsc and Dsd shipped IDENTICAL, so
two zones rendered as one), `elevation_ramp_is_monotone_in_lightness`,
`bathymetry_darkens_with_depth`, `precipitation_bands_are_sequential_and_never_neutral`
and `temperature_ramp_pivots_on_freezing`.

---

### 8.19 Goods localities — the agricultural/biological hierarchy

Trade goods get what minerals already had (§8.16): belt → LOCALITY → cell, the
same two-level structure `deposits.rs` uses, for every enabled `Global`/`Local`
good (`Deposits`/`Manufactured` goods are out of scope — F2's whole premise is
that minerals already have their own, better hierarchy).

- **Rivers as a placement factor** (Slice 1, F6). `biological::RiverContext`
  (built ONCE per world — the same discipline `deposits::GeoContext` already
  applies) carries distance-to-any-river, distance-to-NAVIGABLE-river and
  delta/floodplain membership, multi-source BFS (§8.9 rule 1). `river_multiplier`
  is a MULTIPLIER on an existing score, never a replacement (§5.4 of the plan) —
  `floodplain`/`irrigation`/`riverbank`/`float_out` weights, wired into the
  specific built-in goods the plan names (rice, cotton, wheat, sugar, indigo,
  dates, paper's papyrus branch, honey, hides, timber, hardwoods, furs) and
  exposed as four new `Envelope` fields (serde-defaulted to 0 — no effect) for
  custom goods. `good_score`/`envelope_score` both take `rc: Option<&RiverContext>`;
  `None` (the Goods Editor's live preview, which has no rivers to hand) is a true
  no-op, not an approximation.
- **Marine inshore/bank split** (Slice 2, F5). `GoodSpec.marine_band`
  (`Either`/`Inshore`/`Bank`, serde-defaulted to `Either`) narrows the old
  undifferentiated `sea_coastal` gate to a STRICT SUBSET of itself —
  `marine_band_ok` — so an `Either` good's placement is byte-identical to before
  this slice. Shipped defaults (`default_marine_band_for`): inshore = pearls,
  coral, bay_salt, tyrian_purple, amber; bank = stockfish, herring, whaling.
- **The locality generator + full modulation** (Slice 3, D1/D5/D6,
  `localities.rs`). `GoodLocality { good, x, y, radius_km, grade, extent, name,
  river_fed }` — deliberately the same shape as `deposits::Deposit`. The size
  ladder (§2.1 of the plan): luxury locality 175 km (wine, silk, spices, cacao,
  cloves, pepper) · pastoral/secondary 400 km (wool, hides, horses, timber,
  tobacco) · staple region 900 km (grain, rice, furs, barley, millet) — every
  other good falls back to a tier from its own `Distribution`/rarity, so nothing
  is left without an answer. Full modulation: `belt[i] = max(FLOOR, belt[i] *
  (FRINGE + (1-FRINGE)*influence[i]))` — `FLOOR` is the entire safety mechanism
  (D5's own risk 5.1): a belt cell already producing never falls to literal zero,
  however far it sits from every locality core. Runs BEFORE `dilate_belt` so the
  trade-reach rings spread from the already-modulated belt.
- **Notable naming** (Slice 4, D8). Localities at/above a quality threshold draw
  a deterministic name from `names::gen_name` — the SAME per-cell hearth lookup
  settlements already use, so a locality's name is in the local culture for free,
  no new naming machinery.
- **Persistence.** `metadata["good_localities"]` (JSON), exactly parallel to
  `metadata["deposits"]` — no tile-column change (rule 7). Written by
  `commands::sim_commands::persist_goods_placement`, the one helper all four
  `compute_trade_goods` call sites now share.
- **Production wiring** (Slice 7, D2). `compute_economy`
  (`commands/query_commands/economy.rs`) reads `good_localities` exactly the way
  it already reads `deposits` — a hub's quality for a good blends toward the mean
  grade of any locality inside its catchment (50% weight; `share`-based quality
  still carries the other half, since Slice 3's modulated belt VALUES already
  partly reflect locality quality — a blend, not a replacement, per D2).
  `campaign_province_potential` (`campaign_commands/province.rs`) exposes
  `ProvincePotential.localities: Vec<ProvinceLocalityDot>` alongside the existing
  `.deposits`, and `ProvinceGoodPotential` gained `has_locality`/
  `mean_locality_grade`/`locality_count`, mirrored in `types/campaign.ts`.
- **Slice 0's own gate**: `sim::step8_biological_goods::goods_validation`
  (test-only) builds a real, moderate-sized procedural world end-to-end (plates
  through biological — NOT the synthetic `CampaignSim` fixture `economy_
  validation.rs` uses) and asserts no enabled `Global`/`Local` good places a belt
  that reaches zero settlements' catchments. `Deposits`-distribution goods are
  explicitly OUT of that hard floor (F2 — they have their own, different
  coverage guarantee, `no_shipped_mineral_places_nothing`); a handful of the
  rarest deposit goods missing every settlement's catchment at this diagnostic's
  deliberately modest world size is printed as a FINDING, not asserted. One
  pre-existing belt good, `dyes` (murex purple, untouched by any Slice 1-4
  change — verified), is named as an explicit, documented exception rather than
  either silently loosening the floor or leaving the gate permanently red.
  **Do not add a process-global `cultures::set_active` call to a test** — `cargo
  test --lib` runs the suite in parallel within one process, and this diagnostic
  originally raced `econ_inheritance_rules_fragment_differently` /
  `econ_scorecard_is_deterministic` by mutating that exact global; it now relies
  on `names::gen_name`'s legacy-culture-grid fallback instead, which is
  deterministic on its own.

**The overlay (Slices 5-6) — two layers, at full resolution.** `compute_good_regions`
rasterised a belt onto a COARSE grid — `f = grid_w / 450`, about 8×8 world cells at
the default size — and the frontend filled each block. A block holding one land cell
of belt painted all sixty-four of its cells, so a belt's edge was a staircase that
ignored the coastline and spilled into the sea. No amount of shading could fix it: the
information was not in the payload.

Full resolution IS the coastline clip. `compute_good_belt_masks` copies the belt
column verbatim above `COVERAGE_MIN_U8` and consults no land mask at all, because a
`Global`/`Local` belt good's byte is already exactly zero on the wrong side of the
coast — every one is placed through `envelope_score`, whose first act is the domain
gate. `goods_validation::a_belt_never_crosses_the_coastline` asserts exactly that
claim, against the shared constant rather than a copy of it, because the render path
has no opinion about where the coast is and would silently start painting into the
ocean again if it stopped holding. The same test PRINTS a finding it deliberately
does not assert: `Distribution::Deposits` goods never touch `envelope_score` (§8.16
places them from tectonic setting), so `CoastalMarine` can put a salt pan a cell into
the tidal water — measured at 300×150 as bay_salt 115 cells, tyrian_purple 16,
ambergris 1. That is a `deposits.rs` question, not a render bug, and clamping it here
would hide it.

Two layers, split because they compress differently. COVERAGE ("can it grow here")
is boolean at full resolution and run-length encoded — a belt is contiguous, so a
world-spanning staple region costs a few thousand runs, not one per cell. QUALITY
("is it fine here") is the belt value on the OLD coarse grid, because a wash needs no
per-cell precision; it is painted only where coverage says so, so the coarse wash
still ends exactly on the coastline. Both are switched independently in the Toolbar's
Trade Goods section.

Four rules for anyone changing this:

- **The quality scale is ABSOLUTE and shared by every good** (D10). The belt's own
  0..1 value, never rescaled against that good's own maximum — otherwise a good whose
  whole belt is mediocre gets promoted to full colour just for being its own best. The
  stops live once, in `palette_commands.rs`, and are SERVED (§8.18). Until they
  arrive the overlay draws coverage only rather than inventing a ramp.
- **The scale has TWO paints, same numbers.** The default mixes toward the good's
  own colour, which is unreadable for a good whose hue is already close to the pale
  ground tint — a real user report. `GOOD_QUALITY_HEATMAP_STOPS` (dark blue → red)
  is an alternate reading of the identical belt value, switched per-session via
  `uiStore.goodQualityHeatmap` (Toolbar → Trade Goods → "🌡 heatmap", only shown
  while the quality layer itself is on). `GradeStop`/`good_quality_grades` serves
  the SAME `deposits::grade_label` vocabulary (coarse/ordinary/good/fine/exquisite)
  ore workings already use, at heatmap-exact colours, so `GoodQualityLegend`
  (`ui/world/LayerLegend.tsx`) can show swatches guaranteed to match the map. It
  docks to the opposite corner from the main `LayerLegend`, since it keys off the
  `goodQuality` OVERLAY rather than the active base layer and can be showing at the
  same time as either.
- **A very large belt is downsampled by MAJORITY, never by "any".** Above
  `MASK_MAX_PX` a canvas per good becomes real memory, so the mask reduces — and a
  reduce that takes a block when ANY sub-cell is covered puts the belt back over the
  water one block at a time, which is the original bug in miniature.
- **The FILL is exact; the BORDER is traced at a bounded step.** The fill is what
  carries the coastline. The outline is a stroke a pixel or two wide, so tracing it on
  millions of cells buys nothing and costs a Path2D with a hundred thousand segments.
- **Never re-derive the belt's shape from province polygons** (D3). A wine belt
  crossing three provinces is one belt; snapping it to political lines would make a
  physical fact look administrative.

`GOOD_MASK_DIR=/tmp/g cargo test --lib dump_good_belt_mask_sheet -- --ignored
--nocapture` writes a PNG per sampled good over the world's land/sea, through the
REAL `build_belt_mask` and the frontend's own decode — use it to answer "does the belt
end on the coastline" instead of arguing it, the same way `dump_biome_swatch_sheet`
serves the biome palette.

**The province plate (Slice 6)** is the other half. Land use and tenure are SHARES
and must dither (rule 17), but a `GoodLocality` is not a share — it carries a real
cell and a real `radius_km` — so it draws as a real square at its real position,
clipped to the province footprint, opacity from `grade` and hue from `GOOD_DEFS`. A
MARINE locality sits on shelf water where the province raster is `NO_PROVINCE`, so
`campaign_province_potential` attaches it to the nearest coast within
`MARINE_ATTACH_CELLS` and flags it `sea`: it draws dashed in the adjacent water, is
excluded from the province's own `has_locality` aggregate, and confers **no maritime
territory** (D4).

---

### 8.20 Goods: climate adherence, origins, endemics, terroir

Four changes to how a good gets onto the map, plus the report that makes a
failed placement visible. Full account (including this review's own three wrong
first readings) in `docs/WORLD_REALISM_REVIEW.md`.

**The `clim_base` fold is a FALLBACK, never a pre-pass.** `good_score` used to
fold the dry-winter/dry-summer Köppen variants onto their humid equivalents
BEFORE its match, which made every arm naming `CWA`/`CWB`/`CWC`/`DW*`/`DS*`
**unreachable**. Tea and coffee both name `CWB` — subtropical highland, dry
winter: Darjeeling, Yunnan, the Ethiopian highlands — and both scored exactly
**0.0** there, placed instead by weak fallback arms in the wrong climates. Now the
RAW zone is scored first and the fold applies only if it yields nothing.
`envelope_score` follows the same rule; before this, custom goods saw raw Köppen
while built-ins saw folded Köppen, so one cell was scored under two different
climate labels depending on which scorer ran. Gates:
`dry_winter_zones_are_reachable` (the claim) and
`the_humid_fold_still_applies_as_a_fallback` (the fix didn't cost the fold its
purpose).

**`GoodSpec.origins`** — how many INDEPENDENT homelands a `Local` good seeds
(serde-default 1 = the old single-homeland behaviour, so every save is
unchanged). Multi-origin is the historical norm: pepper from Malabar *and*
Sumatra, cinnamon from Ceylon *and* China, cotton independently domesticated in
India, Peru and the Levant. Each origin repels the earlier ones through the same
dispersion penalty already applied between different goods. Two rules: the
extreme-rarity homeland cap is a budget for the good AS A WHOLE, split between
origins (or a rare good doubles its own world supply); and only the FIRST origin
may fall back to the least-bad cell — a second origin that can't clear the
threshold simply doesn't exist on this world.

**`LandmassContext` + `Distribution::Endemic`** — the connected-component pass
this codebase never had. `Domain::Island` was `distance_to_ocean < 0.20`, i.e.
*near-coast land*, which matched the coastal fringe of every continent, so an
"island" good was really a coastal good. One 8-connected BFS, wrap-aware, built
ONCE per world (same discipline as `RiverContext`/`GeoContext`). An `Endemic`
good is confined to ONE landmass with the island-jump DISABLED — the Banda
nutmeg case: nutmeg grows across the wet tropics, but the tree only grew on ten
islands totalling 60 km², and `rarity` cannot express that (it makes a good
scarce *everywhere* rather than abundant in one place and absent elsewhere).
Six ship: nutmeg, mace (sharing an envelope — two products of one tree),
dragon's blood, camphor, benzoin, sandalwood.

Three rules learned by MEASUREMENT here, each a silent-vanish failure:
- **An island threshold must be stated in km², not cells** (`ISLAND_MAX_KM2`).
  A cell is ~11 km at 3600×1800 and ~133 km on a test world, so a fixed cell
  count meant "Great Britain" on one world and "most of Eurasia" on another.
- **DOMAIN and DISTRIBUTION must not both gate the same thing.** Shipping the
  endemics as `Domain::Island` zeroed their SCORE on every continental cell
  before the distribution could choose a home — all six measured **zero cells**.
  Domain says where the plant can grow (a wet tropical coast); distribution says
  how it is confined (the smallest landmass that scores, preferring a true
  island).
- **The endemic home is chosen up front, not filtered per cell.** That is what
  makes the guarantee unconditional: if the climate exists anywhere, the good
  gets exactly one home; if it exists nowhere it is honestly absent.

**`GoodSpec.soil` / `GoodSpec.relief` — the FINE-GRAIN terroir terms.** Every
other scoring term varies over HUNDREDS of km (Köppen zone, temperature,
precipitation, |latitude|, normalized elevation, fertility — itself a smoothed
blend), which is why a belt rendered as one smooth continent-sized wash: nothing
varied at the 2–10 km scale at which real crop distributions are mottled.
`soil_type` was computed by phase 6 and read by NOTHING in the goods layer;
slope was never computed. Three rules:
- **They live on the SPEC, not on `Envelope`.** They must apply to built-ins
  (scored by the hardcoded matcher, which has no Envelope) and customs alike;
  putting them in `Envelope` silently REPLACED a built-in's whole climate scorer
  with an envelope holding only these two terms.
- **Soil is a preference, never a veto.** An unclassified cell scores 1.0 (no
  information is not bad ground — the same discipline an empty kin roster gets);
  a classified-but-unlisted soil keeps `SOIL_UNLISTED`.
- **`TERROIR_FLOOR` remaps the whole multiplier into `[0.45, 1.0]`.** Applied
  raw it pushed `tea` and `saffron` under the seed threshold and both placed
  literally nothing. Terroir shapes a belt's TEXTURE; it must not decide whether
  the belt exists — the same call the locality pass's FRINGE/FLOOR already made.
  `saffron` is deliberately NOT in the terroir table: already bounded by climate,
  elevation AND latitude, one more gate moved it off every settlement's catchment
  and tripped the Slice 0 coverage floor.

**The two RENDER fixes that went with this.** Both were the same bug in two
places — a real per-cell field being drawn at a coarser resolution than it has:

- **The world quality overlay carried TWO resolutions.** Coverage was a
  full-resolution 0/1 RLE, but the quality wash clipped to it still rode the old
  ~8-cell coarse grid, so a belt read as blocky value steps inside a sharp
  coastline outline. `GoodBeltMask.coverage_rle` is now `quality_rle`: the SAME
  runs carry a 4-bit quantized belt value (0 = uncovered, 1..15 = quality), so
  coverage and quality are one layer at one resolution. Quantizing is what keeps
  the runs long — a belt varies smoothly, so neighbours share a bucket and the
  payload stays close to the old boolean RLE. `quality`/`qw`/`qh`/`coarse`
  survive only as the index space the per-block SUBTYPE classification
  (grain species / paper source) is addressed in. Gate:
  `quality_levels_never_swallow_a_covered_cell` (a covered cell may never
  quantize to 0, and the scale is monotone).
- **The province plate drew a locality as a true-to-scale square.** The size
  ladder is in km — a staple region is 900 km — while a province is 200-400 km
  across, so one grain locality filled the entire plate and the province read as
  one flat block. Where a belt MASK exists (plate 6a already draws the real
  per-cell area) the locality is now reduced to what only it can say: a small
  CORE diamond at its real cell plus its name. Where no mask exists (an older
  world, or a good with no belt here) the full square is still the honest
  reading and is kept.

**The placement report** (`GoodsPlacementReport`, `metadata["goods_report"]`,
served by `get_goods_report`, shown by `ui/goods/GoodsReportPanel.tsx` — opened
automatically when the Biological step finishes, and reopenable from that step
afterwards since it is persisted) — per good: cells, land share, origins actually
seeded, localities, notable names, mean grade. Its reason for existing is the
FLAGS: `absent` (placed nothing), `fallback_seed` (placed only because the seeder
fell back to the least-bad cell — this world may have no suitable climate),
`ubiquitous` (a non-staple over 25% of the world), `single_cell`. Before it, a
good that silently failed to place was invisible until someone went looking for
it on the map.

**Retired as duplicates** (disabled, never deleted — fixed indices in
`TileData.goods`, rule 7): `gemstones` (a generic gem alongside ELEVEN specific
stones, and §8.16 already repurposed `gem_deposits` to mean ore districts) and
`dyes` (marine murex, i.e. the same product as `tyrian_purple`, and the one good
`goods_validation` carried as a standing coverage-floor exception).

---

### 8.21 Natural colour & the shared relief core (`render/natural.rs`)

The `natural` layer is the reference-atlas view: **land coloured by what grows on
it**, sea by depth, both shaded. It reads `biome` (41 classes, §8.12), elevation,
temperature, precipitation and `snow_frac` — all already computed — and adds no
tile column and no sim phase. Gated on step 6 like the biomes plate; a world
without biomes falls back to a continuous climate tint rather than to blank, the
same discipline as `koppen_fallback_biome`.

**It does not reuse `biome_color`, and that is the whole design.** That table is
THEMATIC: `biome_colors_are_distinct` holds all 41 classes apart at a CIELAB floor
so a reader can separate them in a legend. Natural colour wants the opposite —
real vegetation grades continuously, and a palette engineered for class separation
renders a satellite-style map as a discretised poster with a visible edge at every
biome boundary. So there is a second, deliberately OVERLAPPING table, and
`neighbouring_vegetation_classes_stay_close` asserts that overlap so a
well-meaning future "improve the contrast" edit fails loudly. Class edges are
further dissolved by mixing back a continuous temperature/precipitation tint —
the same trick and the same reasoning as `lowland_tint` (§8.18).

**The shared relief core** (`tile_image::relief_at`) now serves both shaded base
layers. Three changes from the single-lamp Lambertian it replaced:

- **Ambient occlusion** from local concavity, which is orientation-INDEPENDENT, so
  a valley reads as a valley under any lighting.
- **Shaded bathymetry** (`sea_shade`). The sea was an unshaded ramp beside a
  hillshaded continent.
- **NOT a fill light.** One shipped and was REVERTED — see below.

Three things measured here, all of which look like tuning and are not:

- **`AO_REF` is the difference between ambient occlusion and film grain.** Set to a
  plausible-sounding 44 m ("valleys are shallow"), AO speckled the ENTIRE map:
  `apply_micro_relief` deliberately dithers every land cell by ±14 m, and against
  an 8-neighbour mean that per-cell dither *is* the concavity signal, so AO
  resolved to ±16% white noise per cell. Rendering with `AO_AMP = 0` isolated it
  conclusively. It is now 240 m; anything under ~150 m re-admits the grain.
- **A FILL LIGHT was shipped, and it was a REGRESSION.** The theory was Imhof's
  (one NW lamp leaves every SE-facing slope at a flat tone), and it survived
  `cargo check`, 20 render tests and a whole-world PNG — then read as obviously
  worse the moment anyone looked at a magnified mountain crop. A fill lamp
  brightens exactly the shadows that CARRY the relief. Four configurations
  rendered side by side (`SHEET_TAG`): no fill > 225°/0.18 > 135°/0.26, the last
  being what shipped. A fill light is a technique for a 3-D scene with real
  geometry; on a shaded DEM whose only signal IS the shadow it subtracts. The
  lesson generalises past this constant: **a whole-world thumbnail is not enough
  to judge shading** — `dump_natural_sheet` now also writes a 3× magnified crop of
  the most mountainous window, because the regression was invisible at world zoom
  and unmistakable at 3×.
- **A per-cell detail dither was built here and REMOVED.** A tile is exactly one
  pixel per cell, so there is no sub-cell space for synthetic detail to live in and
  it resolves as film grain rather than as terrain. Recorded so it is not
  attempted again: detail below the cell needs somewhere to be DRAWN — tiles
  rendered at higher pixel density than cell density, i.e. an inverse LOD pyramid.
  No shading trick substitutes for the missing raster.
- **The bathymetry ramp's dark end was a void.** It fell to (5,15,46) by depth 0.65
  and near-black at 1.0, while `compute_sea_depth` saturates about 20 cells from
  any coast — so on a full-size world essentially ALL open water sat in the black
  tail, and the new seafloor shading had nothing to modulate. Rebalanced to keep
  tone all the way down, still strictly monotone in luminance.

**`NATURAL_SHEET_DIR=/tmp/n cargo test --lib dump_natural_sheet -- --ignored
--nocapture`** builds a REAL world (plates through biomes, mirroring `sim_run_all`
— rule 11) and renders it through the real `render_tile` path, writing
`world_natural.png` and `world_terrain.png` plus a per-biome census. Use it to look
at a palette or shading change instead of arguing about it — it is what caught both
the AO grain and the black ocean, neither of which any assertion would have found.

What it also makes unmissable is that the remaining problems are in the SIM, not
the renderer: land relief is texturally uniform because `generate_elevation`
uses one global noise recipe (`f_base`/`f_range`/`f_hill`, `RIDGE_AMP`,
`HILL_AMP`) for every continent. (The coastline-follows-Voronoi-edge and
straight-plate-boundary-ridge defects this paragraph used to name here were
Terrain 2.0's own slices 3-4 (shipped; §8.23b) — fixed and gated
by `coastline_departs_from_the_plate_boundary`.)

---

### 8.22 Elevation styles (`render/tile_image.rs`)

The default "elevation" layer is a flat, UNSHADED hypsometric tint and
"terrain" is the one shaded hillshade — two names for what an atlas treats as
one decision (how to paint height) times two independent axes (which palette,
how much relief). A STYLE is that pair, selected in the layer key exactly like
class isolation (`split_isolate`, `"biomes#iso=12"`):
`"elevation#style=alpine"` rides the same string the frontend's tile cache
already keys on, so a styled request caches, invalidates and degrades (an
unknown style name falls back to the plain layer) with zero cache-layer
change. Parsed by `split_style`, dispatched by `render_elevation_styled` —
ONE function for every style, keyed entirely off a `StyleParams` struct (land
+ sea ramp, classed-vs-smooth, climate-tint strength, real-`snow_frac` blend
strength, AO/contrast/shadow-floor/light-altitude, warm-vs-cool shadow tint,
sea relief amplitude), so a new style is DATA, not a new render function.

Seven ship, each a real cartographic convention rather than a colour
experiment: **Layer Colouring** (classed Bartholomew/Times hypsometric bands —
also what `mapThemes.ts`'s "Relief & Height" plate now uses, giving it a real
identity distinct from "Physical" instead of reusing the flat unshaded
default); **Alpine** (Imhof's neutral Swiss relief-forward palette, reading
the world's own per-cell `snow_frac` field for its snowcap rather than
inferring it from height alone); **Arid** (warm sand/canyon palette, a lower
key-light altitude for longer raking desert shadows, a sepia shadow tint);
**Polar** (cool ice-blue palette, a very low key-light altitude, strong
`snow_frac` blending); **Analytical** (monochrome — colour carries ZERO
elevation information, relief alone does, maximum AO for a scientific
hillshade); **Antique Plate** (sepia/parchment engraved-atlas look, classed
bands, a warm shadow tint); **Abyssal** (a bathymetry SHOWCASE — land muted
to a low-contrast neutral so it recedes, `sea_relief_amp` boosted well past
the default so Terrain 2.0 slice 5's ridges/trenches/seamounts read as the
map's actual subject).

Two rules:

- **Served, not copied** (§8.18 applies here exactly as it does to every other
  ramp). `elevation_style_palettes()` exposes each style's own land+sea stops;
  `get_render_palettes()` serves them as `elevation_styles` so
  `LayerLegend.tsx`'s elevation legend swaps to the ACTIVE style's real ramp
  (and its `classed` flag, so a stepped style draws a stepped legend) rather
  than guessing a second copy.
- **A style is a VIEW, exactly like a map plate** (rule 14/§8.17) — it changes
  what the elevation/terrain layers look like, never what the world IS.
  `elevationStyle` lives in `uiStore` beside `isolateClass`, rides the layer
  key the same way, and is cleared by any manual change (§8.17's third rule)
  so a picked style can't silently survive switching to an unrelated plate.

`cargo test --release --lib dump_elevation_style_sheet -- --ignored
--nocapture` (env `ELEVATION_STYLE_SHEET_DIR`, `ELEVATION_STYLE_SEED`) builds
one real generated world and renders it through EVERY style via the real
`render_tile_full` dispatch — one full-world PNG per style plus the two
default baselines, and a numbered contact-sheet montage — the same
"render it for real, don't argue about it" discipline `dump_natural_sheet`
and `dump_biome_swatch_sheet` already established.

---

### 8.23 No valley carving — and why three attempts at it failed

**Phase 2 does not carve valleys. In any generator, by any mechanism.** Three
separate attempts to make channel-carving look right on an Earth-sized world all
failed, and the record of them is the point of this section — the fourth attempt
should not start from scratch.

What was removed, and what each drew on the map:

- **`carve` / `fine_carve`** — an INVERTED ridged-multifractal field subtracted
  from the elevation. An inverted ridged field is a **dendritic tree by
  construction**, so this drew a branching dark scratch network across every
  continent. `fine_carve` was the worst because it was an ABSOLUTE subtraction:
  full strength on flat plains, which is exactly where a drainage tree painted
  over an otherwise smooth interior is most obvious.
- **`stream_power_erosion`** — priority-flood + flow accumulation + `K·A^m·S^n`
  (Whipple & Tucker 1999). Correct landscape-evolution physics at the resolution
  it is meant for, and wrong here: a cell is `KM_EQUATOR / w` wide (11 km on the
  default grid), so the valley it models is SUB-GRID — the Grand Canyon is 16 km
  across and would not fill one cell. At cell resolution it cut one-cell trenches
  down every D8 path plus the parallel single-cell rills D8 routing always makes
  on a planar slope.

**The three attempts, in order, so the pattern is visible:**

| | what it changed | why it failed |
|---|---|---|
| `3943136` | noise `carve` 0.16 → 0.05, thermal slump strengthened | aimed at the wrong term — measured per stage, the noise carve leaves notch density at 0.05% of land |
| `e66dc9f` | stream power scaled + spread to its physical scale; thermal scan-order fixed; a grid-scale relief budget | cut grid-scale texture 82–93% **and the map still showed the tree** — subtler lines are still lines |
| *this* | every carving mechanism deleted | — |

The lesson the second attempt paid for: **the STRUCTURE is what reads as wrong,
not the amplitude.** A dendritic pattern at 40 m is still a dendritic pattern.
Damping it is not the same as not drawing it.

**A measurement caveat that matters more than the fix**, because it is why the
statistics said "fixed" while the map plainly was not: `notch_metrics`'s 120 m
threshold is calibrated to the HILLSHADE (half of `AO_REF`), but the flat
`elevation` tint has no shading at all, and a systematic pattern shows there at a
far shallower amplitude. **Judge a tint layer on a tint render, not on a
hillshade statistic.** `dump_erosion_sheet` now writes `elevation` alongside the
analytical hillshade, and takes `EROSION_MODEL=shape` for the TEMPLATE path —
which is what anyone who imported a real-world coastline is actually running, a
different generator with its own carve terms.

**A second negative result, recorded so it is not built into a gate:**
`largest_notch_component` (the largest 8-connected chain of notch cells — meant
to separate "rough" from "a drawn tree") **does not discriminate on the plate
model**. Re-adding `fine_carve` to a clean build measures 0.110 / 0.177 / 0.287%
at 60 / 25 / 12 m thresholds against a clean 0.122 / 0.186 / 0.299% — carved
reads *lower* at every threshold. `limit_grid_scale_relief` normalises the
one-cell band to a fixed RMS, so no amplitude statistic downstream of it can see
structure. There is therefore **no cheap statistical gate for this**, and none
was shipped: a gate that passes either way is worse than none.

**What remains** is `thermal_erosion` — hillslope slumping, which only ROUNDS and
never incises, and whose scan-order fix and gate from the second attempt are kept
— plus `limit_grid_scale_relief` (below). Relief comes from the noise stack and
the tectonic terms.

**Rivers are unaffected.** Phase 5 runs its own priority-flood fill and derives
channels from the finished surface; it never needed pre-cut channels to bed into.
`priority_flood_flow` in `elevation.rs` is now `#[cfg(test)]`, kept only so
`terrain_metrics` can still report drainage density.

---

### 8.23b The grid-scale relief budget — relief at the scale a cell can hold

Every world's grid spans Earth's equator, so a cell is `KM_EQUATOR / w` wide: 11 km
on the default 3600×1800 grid. **Fluvial dissection is therefore SUB-GRID** — the
Grand Canyon is 16 km across and would not fill one cell — and what a cell can
honestly record is the MEAN lowering over its whole footprint, which is both far
shallower than the thalweg and spatially smooth.

Phase 2 recorded neither, and the map showed it: measured on a plate world at
1800×900, **2.77% of land sat more than 120 m below its own 8-neighbour mean,
averaging 346 m** — a 22-km-wide, 350-m-deep slot — with one-cell RMS concavity at
**102.7 m** against `AO_REF`'s 240 m. Rendered, that is a dendritic scratch network
over the whole world plus regular comb striations on every flank: the "too thin
lines, looks like river erosion" report this section answers.

**Three causes, and only one of them was the obvious one.** The noise `carve` /
`fine_carve` terms — cut hard by an earlier attempt at the same complaint
(`3943136`, 0.16 → 0.05) — are NOT a cause: measured per stage they leave notch
density at 0.05%, and removing them entirely makes the finished figure WORSE
(93 → 112 m), because the hypsometric redistribution equalises whatever histogram
it is given. Do not reach for those knobs again.

1. **Stream power cut one-cell trenches.** `K·A^m·S^n` incision applied straight
   down the D8 path, plus the parallel single-cell rills D8 routing always produces
   on a planar slope. Incision is now scaled by `FLUVIAL_VALLEY_KM / km_per_cell`
   (floored at `FLUVIAL_MIN_RECORDED` so it can never silently vanish on a coarse
   grid) and SPREAD over `FLUVIAL_SPREAD_KM` — **inside each pass**, so the next
   pass re-routes over the broadened valley instead of re-cutting the same rill
   deeper, which is what made the combing so regular. It redistributes the carve
   rather than deleting it, so the denudation budget and `isostatic_adjust`'s
   rebound are unchanged in kind.
2. **`thermal_erosion` depended on scan order.** It wrote both donor and recipient
   in place while scanning rows top-to-bottom, so a cell was slumped into before it
   was itself visited. Now a simultaneous delta-buffer update.
3. **The finished field carried more one-cell content than the grid can hold.**
   `redistribute_elevation` multiplies landform relief ×7.7 and grid-scale relief
   ×8.9, so whatever the noise stack leaves is what the map is textured with.
   `limit_grid_scale_relief` caps the one-cell band at `GRID_RELIEF_BUDGET_M`.

**The result** (one-cell RMS concavity · notch density · landform relief, @1800×900):

| model | before | after |
|---|---|---|
| plates | 102.7 m · 2.77% · 2178 m | **18.7 m · 0.22% · 2176 m** |
| shape | 98.6 m · 3.84% · 2261 m | 20.5 m · 0.28% · 2267 m |
| cordillera | 340.2 m · 7.83% · 2084 m | 22.9 m · 0.28% · 2132 m |
| ridged | 125.8 m · 3.69% · 2249 m | 19.3 m · 0.26% · 2253 m |

Grid-scale texture down 82–93%; **landform relief moves under 2% on every model**.
That pair is the whole claim — a limiter that also flattened the mountains would
"succeed" on the first number and ruin the map.

- **Look at it.** `EROSION_SHEET_DIR=/tmp/e cargo test --release --lib
  dump_erosion_sheet -- --ignored --nocapture` renders a real world through the
  REAL hillshade in the monochrome `analytical` style (colour carries zero
  information there, so what is left on the page is exactly the shading) plus a 4×
  crop. Every finding here came from that sheet and none from reading the code;
  `erosion_texture_metrics` is the numeric companion. Same discipline as
  `dump_natural_sheet` (§8.21) and for the same reason.
- **A local mean near a coast must be LAND-ONLY.** A plain box blur averages a
  3000 m coastal cell against sea held at 0, calls the coastline itself "detail",
  and then planes the coast DOWN toward sea level — drawing the exact hard dark rim
  the budget exists to remove.
- **`e' = m + k(e−m)` is not idempotent.** It leaves `(1−k)(m − blur m)` behind, so
  one pass aimed at 16 m settled at **41 m** on a real world. `limit_grid_scale_relief`
  iterates to its fixed point; the pass cap is a termination guarantee, not the
  mechanism.
- **A gate needs a fixture that can fail.** The (now removed) fluvial gate first
  used a tilted plane and **passed on the unfixed code** — a uniform slope drains
  every cell alike, so its carve is broad however it is applied. The surviving
  scan-order gate was verified the same way: revert the fix, watch it fail.
- **The budget is set below what the variance argument alone allows, deliberately.**
  Scaling real continental relief to a 22 km cell suggests a legitimate one-cell
  residual nearer 60 m. Ours cannot be spent that way because our one-cell content is
  UNCORRELATED between neighbours (independent noise at the top of an fbm stack)
  while Earth's is STRUCTURED — part of a cascade whose ridges continue across cells.
  Same RMS, completely different reading: noise looks like grain, structure looks
  like terrain. Buying that headroom back means generating structured sub-grid
  detail (a real multifractal cascade, or erosion run finer and averaged down) —
  a terrain-generation change, not a shading one, and not built.

**NEGATIVE RESULT, so it is not attempted again:** `redistribute_elevation` assigns
each cell a height by its RANK, whose ties break on the row-major index — which
looks exactly like the cause of one-row striping. Rewriting it as a monotone
value-transfer curve measured **bit-identical to the displayed precision** and was
reverted. With ~500k distinct f32 elevations there are no ties to break, so the rank
map already IS a monotone curve; the striping was ordinary high-frequency content,
and `axis_curvature` (curvY/curvX ≈ 0.94) proved the field isotropic before any of
this was changed.

**Still open, named rather than quietly fixed:** a 1–2 cell dark lineament survives
every erosion ablation — it is in the base tectonic/noise field, not the erosion —
and the hypsometric target puts implausibly much land above 6 km.

`bench_phase2` @3600×1800 went plates 11.4 → 12.5 s, shape 13.9 → 14.5 s (+9% / +4%)
after rayon-parallelising `box_blur_wrap`, which the new passes made hot.

**The Earth fidelity gate cannot move here, by construction**: `earth_validation.rs`
scores against the baked GMT DEM and never calls `generate_elevation`. Verified
unchanged at 70.2 / 39.0 all the same.

---

### 8.24 Physiographic provinces & the hypsometric target

Removing the drawn-in drainage texture (§8.23) exposed what was underneath it:
**one mottled cloud over every continent** — no plains, no plateaus, no basins,
no coherent ranges. Two separate causes, both measured.

**1 · One noise recipe for the whole world.** `generate_elevation` used fixed
frequencies and fixed amplitude weights in every cell. Real topography is a
mosaic of PHYSIOGRAPHIC PROVINCES — the Great Plains, the Colorado Plateau, the
Basin and Range, the Canadian Shield are adjacent, ~1000–3000 km across, and each
has its own relief, roughness and characteristic wavelength.

`landform.rs` supplies that mosaic: a jittered, domain-warped lattice of sites at
`PROVINCE_KM` (1900 km, stated in km per rule 25), each assigned one of seven
archetypes — plain · shield · hills · upland · massif · plateau · basin — and
each archetype carrying `amp` (local relief), `rugged` (ridged vs smooth) and
`detail` (weight on the short-wavelength term). Wired into the plate model and
the template model, the two defaults; `ridged` and `cordillera` are deliberate
stylistic models and are left alone.

Four things that are easy to get wrong here, three of them found by rendering:

- **The archetype comes from TECTONIC CONTEXT, not a die roll.** A massif sits
  near an orogenic belt, a shield deep in a stable interior. A purely random
  mosaic is varied and incoherent — mountains in the middle of cratons.
- **`detail` is a WEIGHT, never a frequency multiplier.** Scaling a noise
  function's coordinates by a spatially-varying factor is not a smooth
  reparametrisation: the sample position jumps as the factor changes, and the
  result is concentric moiré rings wherever it varies. In the hillshade they read
  exactly like contour terracing. Blending two fields at FIXED frequencies gives
  the same "broad versus busy" axis with no artefact. The smooth companion to the
  ridged field is a `swell` fbm at the same wavelength, so "not rugged" is a real
  landform rather than merely less ridge.
- **Assign hard, then BLUR the parameter fields — do not blend the two nearest
  sites.** A two-site blend still creases wherever the nearest-site IDENTITY
  changes, and where three provinces meet it creases along every bisector at
  once: the rendered map came out crazed with a polygonal crack network, like
  dried mud. A box blur of a piecewise-constant field is continuous everywhere.
  `amp`/`rugged`/`detail` blur over a quarter of a province; `terrace`/`bowl`
  over a twentieth, which is what keeps an escarpment an escarpment.
- **Shaping runs AFTER the hypsometric redistribution.** `redistribute_elevation`
  is a rank remap, so a flat plateau built before it is simply un-flattened, its
  tied cells fanned back across a band. Amplitude and roughness DO survive it
  (they change which cells rank high); flattening and depressing do not.

**2 · The hypsometric target was wrong by a factor of forty.** At the default
`height`, the old anchors put **~21% of land above 4000 m** and only ~38% below
1000 m. Earth is 0.5% and 71%. Every world came out a pale high plateau with the
tint ramp saturated at its top end, which buried whatever relief was underneath —
including all the province variety above. The anchors are now set so the midpoint
lands on the real ETOPO row (**71% below 1 km, 2.1% above 4 km**), and the
`density` shift TAPERS up the curve instead of spreading evenly, which had taken
the alpine end to 9.2% above 4 km. Gated by `the_default_hypsometry_resembles_earth`,
which asserts the shipped `(0.5, 0.5)` default, not just the tidy one.

Mean landform relief on a plate world fell 2174 → 1291 m as a result — the land
was averaging nearly 3× too high, and every downstream consumer (lapse-rate
temperature, biomes, habitability, settlement placement) was reading it.

**3 · The shoreline was being shaded as a cliff** (`render/tile_image.rs`). A sea
cell stores `elevation = 0`, so a coastal land cell shaded against its raw
neighbours saw a drop of its own full height over one cell — 8848 m at the
extreme — and every coastline was ringed with a hard bright/dark rim unrelated to
the land's slope. It also swamped the AO term, which read the same step as
extreme convexity. `halo_terrain`/`land_elev_at` substitute the centre cell's own
height for a sea neighbour in BOTH shading paths (`relief_at_params` and
`thematic_relief`), so coastal land shades by the land's gradient and the water
is left to `sea_shade`, which carries its own correctly-scaled seafloor relief.

Gates: `provinces_give_a_world_genuinely_different_country` (relief amplitude and
ruggedness must actually vary across a world — a neutral field scores exactly 0),
`province_character_never_steps_between_neighbouring_cells` (the crack-network
regression, bounded against the character table's own range rather than a bare
number, since the gradient of a blurred field scales with cell size),
`a_world_too_small_for_provinces_is_exactly_neutral`,
`the_province_mosaic_is_deterministic`, `the_default_hypsometry_resembles_earth`.

`bench_phase2` @3600×1800: plates 12.5 → 13.6 s, shape 14.5 → 14.8 s — one extra
fbm field per cell plus the province build.

**Still open, named rather than quietly fixed:** the 1–2 cell dark lineament of
§8.23b survives (base tectonic/noise field, not erosion); short vertical hatch
marks appear in shelf water near some coasts; and plateau/basin shaping is
present but subtle — a plateau reads as a level, not yet as a landform you would
name.

---

### 8.24a2 Plates of genuinely different size, a motion layer, collision style, and relict sutures

Four Part B pieces of `TECTONICS_AND_ISOLATION_PLAN.md`, all landed.

**B1 — plate SIZE CLASSES.** The jittered-grid seeding made every plate roughly
the same size by construction (one grid cell of territory each), unlike Earth's
own three-orders-of-magnitude spread (Pacific ~103M km² vs Juan de Fuca ~0.25M
km²). Plates now draw from a four-class ladder (giant/large/medium/small,
`SIZE_CLASS_WEIGHTS`/`SIZE_CLASS_PROPORTIONS`, Earth's rough 10/25/40/25 mix)
feeding a **power diagram**, not a plain nearest-seed Voronoi.

**NEGATIVE RESULT** (§2.4): the first cut used a textbook *multiplicatively*
weighted Voronoi (divide squared distance by weight²). Measured on the gate's
own 8-plate world, a small plate's territory could come out split into
DISCONNECTED islands even with the domain warp turned OFF entirely — proving
the fault was the weighting metric itself, not the warp on top of it. A
multiplicative metric is not a true distance (the triangle inequality can
fail), so its cells are not guaranteed connected; at a modest plate count and
weight ratio, a small plate boxed in by bigger neighbours was pinched apart by
construction. The fix: `d² − offset`, an ADDITIVE term (a real power diagram /
Laguerre-Voronoi), whose cells are provably convex — hence always connected —
for ANY offset at ANY plate count, a mathematical guarantee rather than a tuned
approximation. `PLATE_WARP_AMP_FRAC_WEIGHTED` still trims the warp specifically
for the weighted case, since convexity holds only *before* the warp bends a
cell, and full amplitude can still sever a thin part of one (measured: 0.25
reproduces the old failure, 0.08 does not). Gated by `plate_territory_stays_
connected` (reinstated on the real `generate_plates_and_landmass` path) and
`plate_sizes_span_an_order_of_magnitude` (≥5× largest/median area, shipped
mean 7.74×).

**A regression B1 caused and a broader test sweep caught, not the routing
table.** The oceanic/continental fill (Terrain 2.0 slice 5) picks plates as
oceanic to hit `ocean_fraction` BY CONSTRUCTION, from one shuffled greedy pass
over real per-plate cell counts. That was reliable when plates were all
roughly the same size; once B1 made sizes span an order of magnitude, a
single shuffle order could get stuck far from the target — measured at 6
plates, one seed, 52% land against a 30% target, a >20pt miss
(`land_fraction_tracks_the_target`, Part I Slice 5's own gate). It lives in
`elevation::plate_diagnostic`, not `plates::tests`, so B1's own verification
pass never ran it. Fixed by running `OCEAN_FILL_TRIALS` (24) independent
shuffle orders — still the same greedy step, still deterministic per seed —
and keeping whichever came closest to the target ocean-cell count, rather
than betting the whole result on the first random order. The lesson: a
change that widens the INPUT distribution to an existing mechanism can break
that mechanism even when the mechanism's own code is untouched — worth
re-running the wider `elevation::`/`plates::` sweep, not just the module a
change appears to touch, after any plate-size or plate-count change.

**B4 — RELICT SUTURES**, the believability item: a former collision belt baked
into a plate's present-day *interior*, nowhere near an active boundary — the
Urals, the Appalachians, the Scottish Highlands, the Scandinavian Caledonides
are all exactly this, a healed collision the map still remembers. Before this
there was no mechanism for one at all: `age` (`geology.rs`) was pure per-cell
`fbm_noise`, uncorrelated with anything, and only ever assigned on a belt that
exists *now*.

The plan's own decision: **generate a past, not a simulation** — a time-stepped
tectonic model is Part I Slice 6, already deferred once, and its output for
this purpose would be almost exactly what can be stated directly.
`generate_relict_sutures` (`geology.rs`) bakes 2–4 spines per world,
deterministic from seed: a rejection-sampled start point at least
`SUTURE_MIN_DIST_FROM_ACTIVE_FRAC` (6% of world width) from any active
convergent/transform land cell, walked as a gently drifting line (never a hard
random turn — that draws a scratch, not a range) that also stops if the drift
carries it back toward an active margin. Each suture carries **one uniform
age** for its whole spine, drawn from an OLD (Urals/Appalachians tier) or
ANCIENT (Highlands tier) bucket — never noise — which is the point: a whole
range reads as one coherent age instead of dithering young/old along its own
strike the way a real boundary's noise term does.

The spines are fed as EXTRA SEED CELLS into the exact same multi-source BFS
`compute_orogeny_field` already runs from active boundaries. Downstream code
cannot tell a suture seed from a real one, so a relict range gets its width
(`belt_profile`), its ridge amplitude (`setting_ridge_amp`) and its
erodibility term for free — and critically, **no elevation.rs change was
needed**: `age_amp = 1.25 − age·0.5` (§8.24, already shipped) already turns a
high age into a lower ridge amplitude; B4 just needed to feed it a real age
instead of noise. Every suture is scored `SETTING_COLLISION` (every attested
real relict suture is a healed continent-continent collision). Gated:
`relict_sutures_form_away_from_active_boundaries`,
`a_suture_carries_one_uniform_age`, `relict_sutures_are_deterministic`,
`plate_free_world_gets_no_sutures`, `orogeny_field_carries_the_suture_age_
through`.

**B2 — a motion layer you can read.** `Plate` stays fully transient (§8.24b's
own discipline — recomputed from seed every phase-1 run, never persisted), so
the Euler-pole velocity field that already drives boundary classification
could never be drawn. `PlateMotion` (public, plain-data — `centroid_x/y`,
`pole_x/y`, `omega`, `is_oceanic`, with its own `velocity_at(x, y, world_w)`
duplicating `Plate`'s private rotation formula rather than exposing `Plate`
itself) is what leaves the generator: `generate_plates_and_landmass`/
`generate_plates_and_landmass_with_target` now return `Vec<PlateMotion>`
alongside their existing `WorldBuffer` mutation, and `sim_commands.rs`
persists it to `metadata["plate_motion"]` (JSON, the same one-shot-generator-
output convention `deposits`/`good_localities`/`lakes` already use — no tile
column) from both `sim_generate_plates` and `sim_run_all`. Changing a
generator's return type from `()` to a `Vec` breaks no existing call site —
verified across all of them, since a value ignored in statement position was
never checked by the type system either way.

Boundary-type tinting (convergent/divergent/transform) already existed in
`render_plates` before this — B2 needed no render-layer change there, only a
way to serve and draw the arrows. `get_plate_motion` (query command,
`overlays.rs`) reads the persisted motion and returns one `PlateMotionArrow`
per plate, anchored at its own centroid, with `speed` precomputed
server-side. The frontend (`OverlayManager.drawPlateMotion`, gated by
`overlayVisibility.plateMotion`, shown only while the `plates` layer is
active — the arrows are meaningless without the boundary tinting under them)
normalizes arrow LENGTH by the fastest plate on screen, since `omega` has no
real-world calibration and only relative speed/direction are meaningful; a
dedicated `renderPlateArrow` (filled head, thicker stroke, length set by the
caller) replaces the wind layer's `renderArrow`, whose 12-cell cap would be
invisible at plate scale. Oceanic and continental plates get distinct arrow
hues (`PLATE_MOTION_OCEANIC_COLOR`/`_CONTINENTAL_COLOR`) since they move at
genuinely different real rates.

**The plate inspector — click-to-flip oceanic/continental (built on B2).** A
motion layer you can read invited the obvious next question: can a specific
plate's oceanic/continental assignment (decided automatically by Slice 5's
ocean-fraction fill) be overridden? `sim_set_plate_oceanic(plate_id,
is_oceanic)` does this without a re-partition — `plate_index`/`boundary_type`
(persisted tile columns) are untouched; only which side of an
already-classified boundary is land moves. `plates.rs` extracts the
terrain-rasterization + volcanic-zone tail of
`generate_plates_and_landmass_with_target` (Terrain 2.0 slice 4's level-set
coastline noise, then the collision-style volcanic rolls) into a shared
`rasterize_landmass_and_volcanism`, called both by initial generation and by
the new `rebuild_landmass_from_plate_types` — one copy, so the two paths can
never drift apart (§8.18's discipline applied to a generator, not a colour
ramp). Volcanic flags on boundary cells are reset before every re-roll, which
is what makes a rebuild idempotent rather than only ever accumulating more
volcanic cells across repeated flips — cells placed away from a plate margin
(a lasso Arc island chain, §8.25) are untouched, since those never carry a
CONVERGENT/DIVERGENT `boundary_type`. The rebuild reads its seed from
`metadata["plate_seed"]` (persisted alongside `plate_motion` by
`persist_plate_motion`) rather than taking one as an argument — a rebuild
must use the exact seed generation used (the coastline noise field is keyed
directly off it), and the frontend's Seed field is a UI draft the user may
have since re-rolled without ever pressing Generate again, so it cannot be
trusted as "the seed this world was built with."

Surfaced two ways, both reading the same `get_plate_motion` array (now
carrying `id`/`area_frac` alongside the existing motion fields — a plate's
REAL measured area, counted from `plate_index`, not its nominal size-class
weight): a **Plates panel** in `StepLandmass.tsx` (collapsible, sorted by
area, one row per plate with a swatch, its assignment, area%, and a Flip
button) for browsing the whole set; and a **Flip button in `InfoPanel.tsx`**
(right-click a cell → the existing `plate_index` row, now joined by a new
`CellInfo.plate_is_oceanic` field read from the same persisted motion list)
for flipping the one plate under the cursor. Both are `null`/empty on a world
with no plate data (template/painted, or generated before B2) — same
discipline as an old save's empty `lakes`. Gated:
`rebuild_landmass_is_deterministic` (same seed + assignment ⇒ identical
`terrain`/`is_volcanic`) and `flipping_a_plate_changes_the_land_area_it_should`
(the aggregate land-area swing tracks the flipped plate's own cell count,
within the coastline noise band's margin — not per-cell interior geometry,
since a small or oddly-shaped plate can sit entirely inside the noise band's
`reach` and so isn't a robust per-plate claim on its own).

**B3 — collision STYLE, why the Himalaya and the Andes look different.**
Before this every orogenic belt shared one cross-section: `belt_profile`
was a single decay from the boundary, shaped only by width and offset per
setting. Real belts differ by what's colliding — continent-continent
(`SETTING_COLLISION`) thickens crust over a wide front with several parallel
sub-ranges and an elevated plateau behind them (Himalaya + Trans-Himalaya +
Tibet); ocean-continent (`SETTING_ACTIVE_MARGIN`) concentrates uplift into
one arc-parallel crest offset inland of the trench (Andes).

**NEGATIVE RESULT, kept as the reason the shipped form looks like this**
(§2.4): the first cut cosine-LOBED the existing decay envelope — multiply a
linearly-decaying envelope by an oscillating term and hope for two crests.
Measured directly (sampling `belt_profile` across `dist` and counting local
maxima), it never produced a genuine second peak at any tested `belt_reach`:
the envelope's own decay dominates the lobe amplitude by the time the second
lobe would crest, so the result is one crest with ripples on its downslope,
not two crests. Multiplying a decay by an oscillation cannot guarantee a
second local maximum; only summing (or maxing) independent bumps can. The
fix takes the MAX of three profiles: a main crest right on the suture
(`COLLISION_RIDGE1_WIDTH_FRAC`), a second, lower, offset crest
(`COLLISION_RIDGE2_OFFSET_FRAC`/`_WIDTH_FRAC`/`_AMP` — the Trans-Himalaya),
and a broad, low plateau floor across the whole reach
(`COLLISION_PLATEAU_FLOOR`) so the trough between the two crests reads as
elevated tableland, never a valley carved back toward zero (§8.23's own
lesson about what carving looks like). The collision reach itself widened
(`COLLISION_REACH_MULT` = 2.4× `belt_reach`, up from the old 1.5×) so a
collision belt is measurably WIDER than an active margin at every tested
`belt_reach`, not just multi-crested — the plan's own two-part B3 claim.

No `elevation.rs` change was needed, same as B4: `belt_profile` is consumed
generically by distance from the boundary, so the new shape flows straight
through to the ridge/swell noise it already modulates. Gated:
`collision_belt_is_wider_than_active_margin`,
`collision_belt_is_multi_crested` (both in `geology.rs`, sampling
`belt_profile` directly across `belt_reach` ∈ {14, 30, 60, 90} — the real
production range — rather than rendering a world, since the claim is about
the profile FUNCTION's shape). The multi-crested gate's own peak-counting
helper had to treat a run of equal-valued samples as ONE unit (`dist` is a
`u16` cell count, so fine sampling staircases into flat plateaus, and a naive
"≥ next sample" test double-counts the first cell of every rising plateau as
its own peak) — caught by the gate itself reporting 10 spurious peaks on a
genuinely single-crested active margin before the fix.

Deliberately NOT wired: convergence RATE (from B2's `PlateMotion`) does not
yet modulate ridge count or belt width, though the plan's own text asks for
it ("a fast collision builds a wider, higher belt than a slow one"). `Plate`
velocities are gone by the time phase 2 runs (transient, discarded after
phase 1) and B2 only persists them to `metadata`, which `compute_orogeny_
field` has no connection to read (it takes `&WorldBuffer` alone) — threading
that through is real future work, not silently assumed done. What shipped is
appearance-level and DERIVED from real tectonic classification (the setting
itself comes from real plate velocities, B2/§8.24b), which satisfies the
plan's own "appearance-level, but derived from tectonic motion" decision
even without the rate-modulation refinement.

---

### 8.24a3 A belt varies along its strike; ponding is not a slope problem

Two user reports from one screenshot pair — "too clean mountains are forming"
and "rivers get truncated and the world generates these low ground zones where
rivers go to". One was a real bug with a one-line cause; the other was a
misdiagnosis on both our parts, and the measurement is the deliverable.

**THE WALL — `fbm_noise` does not span 0..1, in a second place.**
`belt_profile` is a pure function of DISTANCE from the boundary, so it
contributes exactly zero variation along a belt's own strike: every point at
the same cross-belt offset gets the identical value. All of a range's
along-strike character therefore comes from one term in `elevation.rs`:

```rust
belt *= 0.35 + 0.65 * belt_noise;   // reads as a 0.35..1.00 swing
```

`fbm_noise` AVERAGES its octaves, so it spans ~**0.11..0.40**, not 0..1 —
the exact fact §8.24b already measured and recorded for the plate warp. So
the realised multiplier spanned **0.42..0.61**: a near-constant ~0.5 with an
18% ripple where the code reads as meaning a 3× swing. A belt of uniform
strength for its whole length has no massifs, no saddles and no gaps — it is
a WALL, which is precisely what the screenshot showed. Normalising on the
measured spread first restores the intended swing (measured 0.350..1.000,
mean 0.653). Gate: `a_mountain_belt_varies_along_its_own_strike`, which
asserts the realised spread exceeds 0.45 and that some stretch reaches full
strength — the un-normalised form fails both by construction.

**The general lesson, now twice paid for: never feed `fbm_noise` into an
expression that assumes a 0..1 range.** Normalise on its measured spread at
every call site that cares about the range rather than just the shape.

**THE PONDING — measured, and the obvious fix does not work.**
`diag_endorheic_fraction` (`rivers.rs`, `#[ignore]`d) walks every land cell's
drainage to its terminus on a real generated world. It reframes the report:
**100% of land already drains to the sea**, 0% dead-ends inland — the
priority flood routes every basin's overflow out. What is actually wrong is
that **6–20% of land is PONDED**, sitting >10 m below its own spill level,
i.e. flat filled hollow. That is the low ground the player sees, and
`extract_rivers` correctly refuses to draw a channel across standing water,
so rivers stop at its edge.

**NEGATIVE RESULT** (§2.4): the obvious cure — give the land a systematic
seaward gradient, since an fbm field has as many local minima as maxima while
real continents stand higher inland — was built and measured, and does not
work:

| ponded % of land | seed 4242 | seed 7 | seed 99 | mean |
|---|---|---|---|---|
| before | 20.3 | 14.8 | 6.2 | **13.8** |
| with a 1500 km continental slope | 20.9 | 10.4 | 14.9 | **15.4** |

It reshuffled which seed was worst and left the mean slightly worse, so it
was reverted. The reason generalises: **ponding is a LOCAL property** (hollows
a few cells across, from the `hill` term and the micro-relief dither) while a
continental ramp is a gradient of ~0.006 per cell. Tilting the table does not
empty the dimples in it.

**WHAT DID WORK: sediment deposition** (`deposit_into_closed_basins`).
There are exactly two ways a real closed basin stops being one — its outlet
erodes down, or sediment fills it up. The first is forbidden and the ban is
hard-won (§8.23's three failed carving attempts: at 11 km cells the channel
is sub-grid and what gets drawn is a dendritic scratch). The second has no
such problem, because it RAISES AN AREA and an area fill leaves no line to
read as a scratch — and it is what actually dominates on Earth, where
endorheic basins are depositional and flat-floored (the Tarim, Lake Eyre,
Death Valley). A priority flood gives each basin's spill level; each cell is
raised toward it by `residual(d) = d · max(MIN_KEEP, smoothstep(d; LO, HI))`.
Two structural guarantees fall out and both matter: the surface only ever
RISES (so it cannot create a new depression), and no cell is raised above its
own spill level (so every outlet and the drainage topology through it are
untouched). It runs after the rank remap — which would otherwise re-fan
anything levelled before it (§8.24) — and before `apply_micro_relief`.

**The tuning is a real frontier, not a free win, and the shape of it is the
finding.** `POND_FILL` is 10 m and `detect_lakes`' threshold is ~35 m, so the
window between "stops truncating rivers" and "stops being a lake" is narrow:
fill hard enough to clear the ponded plains and you start deleting the
world's lakes, which §8.24c forbids. Measured across 3 seeds
(`diag_endorheic_fraction`):

| KEEP_LO / KEEP_HI | mean ponded land | lakes (3 seeds) |
|---|---|---|
| — (before the pass) | 13.8% | 28 |
| 60 m / 200 m | 11.6% | 29 |
| **130 m / 480 m (shipped)** | **9.4%** | **24** |
| 220 m / 1100 m | 8.3% | 17 — lakes gutted |

So ponded land is down about a third and the deep basins survive. It is
**not** "almost none", and driving it there means giving up the lakes; that
trade is the honest limit of this mechanism, not a tuning oversight. A first
attempt used `residual = d²/(d + HALF)`, which reached only 11.5% and cannot
do better at any HALF — draining a 500 m basin with that form needs HALF ≈ 24
km, which would flatten everything else. The shape was wrong, not the
constant. Gate: `deposition_fills_shallow_hollows_and_spares_deep_basins`
(a 40 m hollow must come out under 10 m, a 900 m basin must keep over 500 m,
plus both structural guarantees).

**AND THE ONE THAT ACTUALLY CAUSED THE VISIBLE TRUNCATION: two thresholds
that did not match.** After all of the above the user still reported cut-off
rivers, and the cause was not the terrain at all. `extract_rivers` blocked
channels on a hardcoded **10 m** of fill; `detect_lakes` emits a lake only
past its own `fill_depth`, **~35 m**. Every basin landing between the two was
blocked as "standing water" *and drawn as nothing* — so a river stopped dead
in open ground with no lake, no sea and no confluence at its mouth. §8.24c
states the invariant in its own words ("the lake extent the renderer draws
and the ponded extent the router stops at have to be the SAME set") and this
line quietly violated it. Deposition made it *more* visible, by moving basins
into that band.

`extract_rivers` now takes the caller's `fill_depth` and uses it as the pond
threshold, so the two sets agree by construction. The deep-basin protection
the rule exists for is untouched: a basin deep enough to matter becomes a
lake, and lake cells are already excluded separately. What changes is only
the shallow band — a metre-scale veneer over nearly flat ground, i.e. an
alluvial flat, which a real river crosses. Measured on a real world:
**dangling stubs 10/27 (37%) → 0/49 (0%)**, the river count rising because
channels that used to be cut short now run their full length.

Gate: `a_basin_too_shallow_to_be_a_lake_does_not_stop_a_river`, which asserts
the invariant DIRECTLY (no cell may be ponded-but-not-lake) via the shared
`ponded_mask`, rather than inferring it from stub counts.

**Two lessons about the gate itself, both worth more than the fix.** The
first version asserted stub counts and passed with the bug still in — 1090
rivers on a dense fixture always find *something* adjacent to join. The
second version asserted the right thing but the FIXTURE was wrong: its
regional slope was 14 m per cell against a 22 m bowl, so no closed basin ever
formed and it passed either way. **Always verify a new gate fails on the
unfixed code** (§8.23b's own rule): reverted, it reports 104 invisible cells
and fails loudly; restored, 0.

**AND A THIRD TIME, in the lake's SIZE — the gate was honest and measuring
the wrong thing.** The user reported truncated rivers again with the stub
count at a genuine 0/49, so the gate's definition of "ends properly" had to
be wrong rather than the count. It was: `dangling_stubs` accepts a mouth at a
lake of ANY size, so a river ending at a ONE-CELL lake satisfies it — and a
one-cell lake is smaller than a pixel at world zoom. Worse, the asymmetry was
built in on purpose: river widths are explicitly zoom-compensated so "even
small streams stay visible", while a lake draws as bare `fillRect(x, y, 1, 1)`
per cell with no minimum. So the river was widened to stay visible and its own
terminus was allowed to vanish. `diag_river_mouth_visibility` classifies every
mouth by how visible its terminus actually is and measures **6-9% of all
rivers ending at something the reader cannot see** (3 per world across 3
seeds).

The fix is cartographic and belongs in the renderer, because the DATA was
right all along — the lake exists, it is in the list, the panel will list it.
A lake at or under `MIN_LAKE_SYMBOL_CELLS` also draws a disc of at least
`MIN_LAKE_SYMBOL_R` scaled by the same `1/sqrt(zoom)` the rivers use, so a
pond and the stream feeding it stay in proportion; the true per-cell footprint
is still filled underneath, and zooming in hands the drawing back to the real
cells the moment they are bigger than the minimum. Every atlas draws a small
water body at a minimum symbol size for exactly this reason.

**The generalisation, now paid for three times in one sitting:** each of these
was the model holding a fact and the RENDER dropping it — a ponded cell no
lake represents, a basin between two thresholds, a lake below one pixel. When
a user reports something missing from the map and the data gate says it is
present, suspect the render before re-tuning the generator, and **measure the
symptom the user can actually see rather than the invariant that is
convenient to assert.** Note also what has NO automated gate here: this
codebase has no frontend rendering test beyond `tsc`, so the minimum-symbol
rule is guarded by the diagnostic plus looking at it — stated plainly rather
than dressed up with an assertion that would not exercise the canvas.

---

### 8.24a4 A river ends at the sea, a lake, a confluence — or it evaporates

The rule, and the ONE legitimate exception, now stated in the code rather than
implied: **every river must reach the sea, a lake or a confluence, unless its
channel genuinely dries up in a desert** — the Sahara wadi, the Australian
creek, the Tarim petering out into sand.

Before this that exception could not be expressed. Discharge is ONE SCALAR per
reach computed at the outlet (§8.24d), so flow could never decline along a
course, and the only dry-climate handling was a whole-river prune
(`!is_mouth && order <= 2 && arid_frac > 0.55 → continue`) that deletes a
stream outright. So a desert river was ALL OR NOTHING: it either ran its full
length to the sea or vanished entirely, and the commonest thing a desert river
actually does — run a while, then dry up — had no representation at all.

**The mechanism is a water budget, not a rule about deserts.** A per-reach walk
from source to outlet in m³/yr: GAIN is the NEW catchment joining at each cell
× its own runoff depth (`RUNOFF_ARID` 0.08 vs `RUNOFF_HUMID` 0.35, matching the
lake balance's own split); LOSS is the wetted channel strip
(`CHANNEL_LOSS_WIDTH_KM`, stated in km and converted per world per rule 25) ×
the local evaporative excess `PET − P`, with PET a documented temperature proxy
(phase 5 has no radiation budget, and it is only ever used to decide whether a
dry channel survives, never to size a river). Where the budget goes negative
**in an arid cell**, the reach is truncated there and flagged `River.ends_dry`.

**The Nile falls out of the model instead of being special-cased**, which is the
strongest evidence it is the right mechanism: a river fed from wet uplands
arrives carrying a volume that dwarfs the per-cell loss and crosses the desert,
while a stream raised IN the desert dies within a few cells. Measured on the
banded fixture: 3 rivers died with a mean catchment of 199 cells, 22 crossed
with a mean of 8,033 — a 40× separation. `a_great_river_crosses_the_desert_
that_kills_a_small_one` asserts exactly that, and a blanket "delete rivers in
deserts" rule passes the ends-somewhere gate while failing this one.

**THE OLD PRUNE AND THE NEW BUDGET ARE THE SAME PHYSICS STATED TWICE, and
applying both deleted every wadi the budget had just modelled.** First
measurement after wiring the budget in: **0 rivers ended dry** on a world with
a hot desert belt across every drainage. A dried reach has `is_mouth == false`,
low order and a ~100% arid course, so it matched the arid prune every time.
"An arid basin swallows its runoff" is precisely what the budget now computes,
and the budget is the better instrument because it says WHERE the water runs
out rather than merely that it does — so it wins, and the prune keeps only
order-1 rills so a desert still reads sparse rather than veined. The gate
caught this rather than passing vacuously, which is what the `!dry.is_empty()`
assertion inside it exists for.

**Two consequences that are easy to miss.** A dried river joins nothing, so it
is neither a trunk nor a `tributary` (confluence detection seeds settlement
magnets off that flag). And it needs its own RENDER: drawn as a plain solid
line, a modelled terminus is pixel-for-pixel identical to the truncation
artefact, so the lower course is drawn DASHED and fading — the atlas convention
for an intermittent watercourse — and the reader can tell "this evaporates
here" from "this was cut off for no reason". No playa symbol is drawn: the sim
models no playa, and inventing one would be a claim the data cannot support.

**Grid size is load-bearing in the fixture and is not arbitrary.** Catchment
gain scales with cell AREA (cell²) while channel loss scales with cell LENGTH
(cell¹), so on a coarse test grid one cell is a 40,000 km² catchment and no
stream can ever dry — the mechanism is inherently weaker on coarse grids and
stronger at real resolution. The banded fixture runs at 600×300 (~67 km/cell)
for that reason; at 200 wide it would have passed while testing nothing.

Gates: `every_river_ends_somewhere_or_dries_in_a_desert` (no unexplained stub;
every `ends_dry` terminus is in a Köppen B cell), `a_great_river_crosses_the_
desert_that_kills_a_small_one`, `a_humid_world_never_dries_a_river_up` (the
mechanism must be INERT where there is no desert, or it is just a licence to
truncate). All three verified failing before the fix.

---

### 8.24b Plate margins are warped at the SOURCE

`plate_index` was a plain Voronoi partition — every plate boundary was the exact
perpendicular bisector of two seed points, i.e. a straight line. Everything
downstream is derived from it: `boundary_type` is read off `plate_index`, the
orogeny belt is a distance field from `boundary_type`, `deposits.rs` reads the
same column as tectonic setting, and the coastline is a threshold on plate crust.
So one straight partition drew a straight mountain range, a straight rift, a
straight ore belt and a straight margin, all at once — the "mountains are straight
lines" report.

Downstream passes had each grown their OWN warp to hide it (`elevation.rs`'s
`oro_warp_*` warps the orogeny LOOKUP; `warp_terrain_boundary` warps the
coastline). **Warping a lookup only bends where a straight line is SAMPLED
FROM.** The line is still there, and each pass bends it differently, so the range,
the rift and the coast stop agreeing about where the margin runs.

`warped_voronoi` warps the PARTITION instead: the nearest-seed scan is done at a
multi-octave domain-warped sample position, so there is no straight line left
anywhere downstream to bend. `warp_frac = 0.0` reproduces the plain Voronoi
partition exactly, which is what lets the gate use the function as its own control.

Three things measured here, all of which look like tuning and are not:

- **`fbm_noise` does NOT return 0..1.** It averages its octaves (`val / max_amp`),
  so it concentrates about its own mean with much-reduced variance — measured,
  ~0.11..0.40, mean ≈ 0.28. The first cut used `fbm_noise(..) - 0.5` inline, which
  is therefore an almost constant NEGATIVE offset: it TRANSLATED the sample field
  instead of bending it. 10% of cells flipped (all near a margin) and straightness
  moved 0.504 → 0.498. **A displaced straight line is a straight line.** The field
  is now centred and scaled on its own MEASURED spread.
- **The metric has to be local, and at the right scale.** The first metric was
  total boundary LENGTH, on the reasoning that a straight bisector is the shortest
  curve between two triple junctions. True of one segment, false of a partition:
  bowing one margin out bows its neighbour in, so the total is conserved — measured
  1.00× (5682 vs 5672 cells) on a visibly curved partition. The second metric was
  local straightness at a 6-cell radius, against an ~80-cell plate spacing, where a
  margin curving over its whole length still looks locally straight. Only the third
  (local straightness at R=20) sees it.
- **The constants come from `diag_sweep_plate_warp`, not from eye.** It maximises
  the straightness drop subject to worst-plate connectivity > 0.90 — the constraint
  being that a large warp lands samples inside a NEIGHBOURING plate and the
  partition sheds detached specks, which `boundary_type` then reads as phantom
  plate boundaries scattering ore districts through plate interiors. The usable
  region is a SHORT wavelength at a modest, tightly-clamped amplitude; a long
  wavelength translates whole plates instead of curving them. Shipped
  0.25 / 0.80 / 0.80 → straightness 0.449 → 0.368.

Gates: `plate_margins_are_not_straight_bisectors`, `plate_territory_stays_connected`.

---

### 8.24c Lakes have a WATER BALANCE, not just a fill

A priority-flood answers one question — "from which cells can water not escape
downhill?" — and its answer is a BASIN, every cell below the spill point however
vast. `detect_lakes` emitted that basin verbatim, guarded only by "is it more than
a QUARTER of the world", so a broad continental interior sagging a few tens of
metres below its rim came out as one water body of millions of km², larger than
some seas, standing on ground that is merely low.

The missing question is the water balance. A closed lake is in steady state when
inflow equals evaporation off its surface, so its equilibrium AREA is
`inflow ÷ evaporation_depth` and nothing about the basin's shape can make it
bigger. That is why the Caspian sits far below its own rim, why the Aral shrank
when its inflow was diverted, and why the Sahara's deep basins are dry — one
equation, three cases. `trim_basin_to_water_balance` computes it and LOWERS THE
WATER LEVEL of an over-large basin, keeping the deepest cells.

Four rules:

- **Lower the level; never delete the lake.** The pre-2026 code deleted an
  over-large basin outright, which left the drainage still routed through a dry
  hollow so rivers were drawn climbing the basin walls — which is why the size
  filter was removed and replaced by the quarter-of-the-world guard in the first
  place. Above that guard the map had giant lakes; below it, dry basins with rivers
  running uphill out of them. Lowering gives every closed basin a real water body
  for a river to end in, at a size its climate sustains.
- **An ice sheet holds no lake.** A basin under a permanent ice cap (Köppen `EF`,
  or perennial `snow_frac`) is full of ice; drawing open water there is wrong on
  the map and feeds a fish assemblage into `aquatic.rs` for a glacier.
- **`LAKE_MAX_KM2` is stated in km²** (rule 25) — the Caspian, the largest lake
  that exists, is 371,000 km².
- **A world-sized test basin cannot test the climate term.** Its catchment is so
  large that BOTH a wet and an arid climate clear the absolute cap and clamp to the
  same answer (measured: wet 6 cells, arid 6 cells — a gate that would have passed
  on a water balance wired to nothing). `hollow_world` exists for that contrast.

Gates: `no_lake_is_larger_than_the_largest_real_lake`,
`an_over_large_basin_is_lowered_not_deleted`, `an_ice_cap_basin_holds_no_lake`,
`an_arid_basin_holds_less_water_than_a_wet_one`.

**FLOW AND LAKES ARE ONE PASS** (`compute_world_hydrology`). A priority flood
fills every closed basin to its spill level, and `extract_rivers` refuses to draw a
channel across a filled basin (`ponded`, >10 m of fill) because water stands there
and a line across it climbs the true ground beneath. That was consistent while
every filled basin became a drawn lake — and the water balance broke it: the basin
is still ponded end to end while only its deepest part is drawn as water. Measured
on an endorheic fixture, **53,998 cells were treated as standing water and 22 were
drawn as lake, and all 12 rivers stopped at that invisible shoreline** — the
"rivers end to nowhere / are truncated" report, a straight consequence of two
extents disagreeing. The second pass re-floods with the LAKE SURFACES as drainage
sinks (`compute_hydrology_with_sinks`), so the exposed basin floor becomes what it
physically is — dry land draining inward — stops being ponded, and rivers run
across it and end AT THE SHORE. No river climbs, because the flood now descends
toward the lake rather than toward the spill. Skipped when nothing was trimmed, so
an ordinary world pays for one flood. Gate:
`a_river_always_ends_at_water_or_another_river` (0 stubs, was 12 of 12).

**Lakes are now PERSISTED** (`persist_lakes`/`load_lakes`, `metadata["lakes"]`).
They used to be the one hydrology product with no stored copy: every consumer
called `detect_lakes` again with its own hard-coded `fill_depth` of 0.004, while
the set the user sees was built from the `lakeFillDepth` SLIDER. Those disagree the
moment anyone moves the slider — and settlement placement was a consumer, so towns
were sited to avoid one set of lakes and drawn under a different, larger one. Same
class of bug as §8.18's hand-copied colour tables, same fix: keep one copy.

**A lake cell is UNDER WATER.** `buf.terrain` calls it land (a lake is a filled
depression on land, not sea), so every settlement gate passed — and then the
lakeshore bonus's ±2 window includes the cell itself, so a lake cell was not merely
allowed as a town site but actively attractive. `compute_habitability_fields` now
zeroes `hab`/`trade` on lake cells, at the source rather than in one call site, so
the rule reaches city placement, the habitability layer, junction sites and colony
siting alike. The shore keeps its bonus.

---

### 8.24d River discharge is computed ONCE

`River.discharge_m3s` is the river's real mean annual flow, derived in
`extract_rivers` from catchment area × runoff depth in honest units.

It exists because there were TWO discharge calculations that did not agree:
`extract_rivers` derived a unitless proxy for the render width with
`runoff = mean_precip / 700` clamped to 0.2..2.2, while the river-systems query
behind the Hydrology panel independently used `mean_precip / 1000 × 0.35` clamped
to 0.01..1.2. Same river, two runoff models — so the flow on the panel and the
width on the map answered different questions.

**Render width now comes from hydraulic geometry** (Leopold & Maddock: `w ≈ 6·√Q`).
What it replaces saturated: `ln_1p(discharge/threshold)·0.62 + 0.7 + len_term`
clamped to `(0.6, 2.6)` reaches its ceiling at roughly twenty times the channel
threshold and stays there, so every river above a fairly ordinary size drew at
EXACTLY the same width and a 50,000 m³/s trunk was indistinguishable from a
5,000 m³/s one. That is the "rivers don't get wider with a large flow" report, and
it was a clamp, not a modelling subtlety.

Two rules: a **unit** gate must be scale-free (`discharge_is_in_plausible_cubic_
metres_per_second` backs the implied runoff depth out of each river's own Q and
catchment, because a plain "is Q between 1 and 300,000" assertion measures the
FIXTURE — a 300-cell-wide test grid has 134 km cells); and the panel reads the
STORED value, falling back to its own estimate only for a world generated before
the field existed. Gates: that one plus
`river_width_tracks_discharge_instead_of_saturating`.

**Delta fans need the seam guard too** (rule 6). Every river stroke in
`OverlayManager` goes through `strokeSmoothPath`, which breaks a path wherever two
points jump more than `seamGap` — but the distributary fan drew mouth→cell
directly with no guard, so a delta whose mouth sits near the antimeridian had cells
on both sides of the seam and the segment joining them ran the full width of the
world at near-constant y. That is the "straight line across the map on the rivers
layer": not a latitude line, not a river — one unguarded delta spoke.

---

### 8.25 Stage-1 freehand area tools (`step1_plates/landmass_ops.rs`)

The Landmass step used to be three buttons and a circle brush — no area
marking, no coastline shaping, no islands, and "Generate from Plates" repeated
the identical world on every press (`ITCZ_AND_LAND_TOOLS_PLAN.md` Commit 1). A
`Lasso` (a freehand-drawn polygon in world-cell coordinates) plus four ops —
`smooth_roughen`, `fjords`, `island_chain`, `fill` — mutate
`terrain`/`elevation`/`is_volcanic` only within it. Each op loads
`ColumnSet::PHASE_PLATES` and the caller calls `buf.save`, which already
pushes exactly one `undo_journal` entry, so every op is undoable and
re-rollable for free — no new history code.

Four rules this module holds, each gated by its own test:

- **The lasso is UNWRAPPED, not clamped** (rule 6). A polygon drawn across the
  antimeridian arrives with points on both edges; a naive point-in-polygon
  test selects the *complement* of what the user circled. `Lasso::new`
  re-expresses every vertex in one continuous frame anchored on the first
  point; the hit test then tries the polygon at `x`, `x−w` and `x+w` so a
  query cell hits whichever wrapped copy actually contains it. Gated by
  `lasso_across_antimeridian_selects_what_was_drawn`.
- **Every op FEATHERS to the lasso edge.** A hard clip prints the user's
  selection gesture onto the map as a straight coastline. `Lasso::blend`
  gives a soft 0..1 membership (a smoothstep ramp over `FEATHER_CELLS`), and a
  feathered cell's fate is decided by a deterministic per-cell hash against
  that blend — soft at the edge, but bit-reproducible for a given seed rather
  than a fresh RNG draw each run. Gated by `op_feathers_to_lasso_edge`.
- **Roughening is a LEVEL SET, never a per-cell dice roll**: a signed
  distance-to-coast field — `local_coast_field`, a multi-source BFS scoped to
  the lasso's own padded bounding box, never the whole world — perturbed by
  fbm and re-thresholded at zero, bounded by a `reach`. A per-cell noise
  threshold scatters speckle islands across deep ocean by construction; a
  level set can only ever move a coastline that is already there. Gated by
  `roughening_is_bounded_by_reach_not_scattered`.
- **Ops iterate the selection, never the world** (§8.9 rule 1's spirit).
  `Lasso::candidate_cells` scans only the polygon's own padded bounding box, so
  reshaping one bay on a 26M-cell world costs a few hundred cells, not a
  full-grid sweep. Gated by `ops_iterate_selection_not_world` (a lasso op on a
  4000×2000 world must complete in well under the time a whole-grid scan
  would take).

`fjords` walks inland from a selected coastal sea cell — sinuous, tapering to
the head — carving a real channel, which is the honest way to draw a fjord as
opposed to notching a coastline with noise (see §8.23's record of why
noise-carved channels read as a drawn scratch rather than a landform).
`island_chain` supports `Arc`/`Scatter`/`Single`; only `Arc` islands are
marked `is_volcanic`, which is real data — `deposits.rs`'s `VolcanicArc` model
scores off that exact column (§8.16), so a planted arc can carry a genuine ore
province later. `fill` is the decisive bulk-set op, still feathered at the
edge like every other op here, just committing only past the blend midline
rather than by a stochastic draw.

Commands: `land_op_smooth_roughen`/`land_op_fjords`/`land_op_islands`/
`land_op_fill` (`commands/sim_commands.rs`), each taking the lasso polygon as
JSON (the same shape `sim_generate_ridges` already uses for `linesJson`).
`preview_commands::render_world_thumbnail(max_px)` is a read-only, downsampled
land/sea + elevation thumbnail sampled directly from the `WorldBuffer` —
deliberately NOT read back through the tile/LOD cache, whose invalidation
timing after a generate would make a thumbnail silently stale — used by the
Landmass step's 2-variant compare (generate A → thumbnail → undo → generate B
→ thumbnail → show both, keep one or restore the other from its own seed).

Frontend: `uiStore.activeTool` gains `"lasso"`; `lassoPolygon` is a single
transient selection (mirrors `ridgeLines`'s draft/commit plumbing exactly —
`MapCanvas`'s lasso pointer handlers accumulate a draft polygon and commit it
on pointer-up, `OverlayManager.setLassoSketch` draws it). `StepLandmass.tsx`
carries the Area Tools panel (draw/clear, the four ops with their own
params + a Re-roll button that undoes and re-applies with a fresh seed
against the same polygon), a Randomise-landmass button (fresh seed → generate,
where "Generate from Plates" used to always repeat the stored seed), and the
2-variant compare.

---

## 9. Docs (`docs/`)

**START HERE — the current plan**
```
living_world/00_INDEX.md          ← ⭐ THE BUILD QUEUE (2026-09-25). Ten numbered rows —
                                    01 feeds & pruning · 02 people · 03 development
                                    tracks · 04 government & edicts · 05 culture
                                    acceptance · 06 ideology & scholars · 07 artisans &
                                    masterworks · 08 leisure & games · 09 realms, war &
                                    barbarians · 10 settlement overview. A session takes
                                    the FIRST row that is NOT STARTED with its
                                    dependencies DONE, builds its slices in order, runs
                                    the economy gates once at the row's end (§2.9), and
                                    marks the row in the index in the same commit
FIX_PLAN.md                       ← ⭐ Measured Earth-fidelity baseline + the prioritised
                                    fix plan (climate · one-simulator · economy · society),
                                    with a regression gate per item. Read before planning work.
SCOREBOARD.md                     ← ⭐ The project held as ~12 NUMBERS instead of 89k lines:
                                    both fidelity scorecards, test counts, perf, and an
                                    explicit list of what is still UNMEASURED. See §2.6.
```

**Live operational docs** (these describe the project as it is)
```
SETTLEMENT_LIFE_PLAN.md           ← ⭐ L0-L3 SHIPPED (§5.7, the STOP MARKER
                                    reached), L4-L13 queued. How
                                    people live, earn, eat, die and make trouble in
                                    a campaign city. Eleven measured findings,
                                    headed by F1:
                                    the famine check reads grain LEFT IN STOCK
                                    after eating (`update_food_and_starvation`
                                    runs after the eating loop), so grain the poor
                                    cannot buy reads as the city being fed — the
                                    bug that reverted S7, and entitlement failure
                                    the model cannot express. Also: no wage exists
                                    and `real_wage_index` is a sentiment blend, not
                                    an income; population is one float with no
                                    births, deaths, ages or causes; Pops are
                                    re-derived yearly with no memory; no housing,
                                    religion, watch or per-city notables. Fourteen
                                    slices L0-L13 (instrument · entitlement = MONEY
                                    M7, one shared change · incomes + Allen welfare
                                    ratio · annals + a Life tab · STOP MARKER · vital
                                    rates + age bands · welfare into behaviour ·
                                    housing/crowding consuming real timber+stone ·
                                    the settlement year · fire/flood/water · church
                                    + alms · watch + crime · persistent pops
                                    (shadowed first) · townspeople), every
                                    behavioural one dosed from zero, demographic
                                    gates on a PROVINCED fixture only
MONEY_AND_COINAGE_PLAN.md         ← ⭐ AGREED IN SCOPE, NOTHING BUILT. Real,
                                    CONSERVED money: coins are struck at a mint
                                    from mined bullion, held in purses per holder
                                    per city, carried in chests on real legs, and
                                    spent — never `+=`'d into a vault. Measured
                                    premise (F1): every sale today credits the
                                    carrier (`production.rs:2297`) with no payer,
                                    so money is created on every trade. Barter is
                                    ALWAYS available and settles with real stock
                                    moving both ways (the payment goods leave the
                                    seller's stockpile); money just beats it.
                                    Currencies with 2-3 denominations and dated
                                    issues, culture-based units of account and
                                    regional gold:silver ratios, mints that CLOSE
                                    when their coin goes unused, a Victorian coin
                                    catalogue with follow-a-coin, a market money
                                    band, and banks on real reserves with bills
                                    of exchange settled by specie shipments.
                                    Slices M0-M11: a PARALLEL LEDGER observes
                                    first (M0-M4 bit-identical), then barter,
                                    affordability, wages and the switch-over are
                                    each dosed from zero
HOUSES_GUILDS_AND_MARKET_PLAN.md  ← ⭐ S2 (annona carrier class) + S3 (craft
                                    guild roster unfreeze) + S4 (craft
                                    signatures served) + S5 (transit demand,
                                    shipped inert at zero) + S7 (eight
                                    orphan-raw recipes) + S8/S9 (the house/
                                    craft atlas queries) + S10 (the four-
                                    window split) BUILT AND GATED — see
                                    CLAUDE.md §5.6. S1 is BLOCKED (a
                                    pre-existing, already-measured negative
                                    result, not merely undosed — see queue
                                    item Q14). S3's own 1→3 dose walk measured
                                    UNTESTABLE by the standing gates (they
                                    carry zero manufactured goods). **S6
                                    dose-walked to 0.3 and REVERTED** — two
                                    real gate failures (a limited-liability
                                    breach and a relay-fixture assumption the
                                    dose invalidates), recorded at
                                    `BLOCKADE_STAGING_DOSE`'s own doc comment.
                                    S10/S11(text+map, incl. Q19)/S12a/S12b/
                                    S12c(3 of 5) — everything this plan built
                                    in follow-up sessions after the first —
                                    could only be verified by `tsc`/`vite
                                    build` — no display to actually open the
                                    windows in, said plainly rather than
                                    claimed as tested (queue item Q17, and
                                    Q19's on-map lane rendering is named as
                                    the single highest-risk piece to check
                                    first). Q20 (full cross-tab year-travel on
                                    the timeline plate), Q21 (Kin as a real
                                    tree — needs new persisted sim state) and
                                    Q22 (Lineage as a drawn SVG diagram) remain
                                    QUEUED. The first session's own THREE-DOSE
                                    ceiling (S1 investigated/blocked, S3
                                    walked/untestable, S6 walked/reverted) is
                                    unchanged — every slice shipped since is a
                                    read-only query or pure frontend work, not
                                    a dose. The one-session build
                                    plan for houses, guilds and the settlement
                                    market, after four decisions: the era is a
                                    Roman/medieval MIX (no `EraProfile` switch built
                                    yet — every Roman-flavoured slice ships
                                    era-NEUTRAL so the axis can be added later, queue
                                    Q1); the *annona* reframe applies to BIG CITIES
                                    ONLY; the player stays OBSERVATION-ONLY (so every
                                    item is a SPECTACLE problem, which is why half the
                                    plan is atlas and UI); and every behavioural change
                                    ships dosed from zero. Its cheapness comes from one
                                    finding: **four mechanisms in it are already fully
                                    wired and dead at a zero dose** —
                                    `N1B_OWNERLESS_LOSS_RATE` (the ownerless-cargo loss
                                    roll, its consequence and its `diag_lost` counter
                                    all built, constant `0.0`), `BLOCKADE_STAGING_DOSE`,
                                    `CAPACITY_BIND_DOSE`, and the house trading web on
                                    the map (`OverlayManager.ts:1602` already routes
                                    seat→city lanes through the real corridor
                                    resolver — undirected, unlabelled, no goods in
                                    it). Twelve slices: signatures (no sim change at
                                    all) · eight new manufactured goods wiring the
                                    five ORPHAN RAWS that today have zero downstream
                                    consumers (clay, coal, alum, hemp, pitch) · the
                                    annona carrier-class split · three dose walks
                                    (ownerless loss, a per-city guild cap, the routed
                                    blockade) · transit demand built inert · two
                                    atlas queries · the four-window split · the
                                    House/Craft Atlas with real on-map lane labelling
                                    · the Houses redesign (bump chart, sparkline
                                    strips, event ticker, dossier timeline). Carries a
                                    STOP MARKER (§7) so a short session still lands
                                    something coherent, a 13-item numbered QUEUE
                                    (§8, rule 36 — each naming what it waits for),
                                    and a risk register whose first line is that three
                                    doses in one session is the ceiling against an
                                    instrument that has flipped inside its own noise
                                    band five times
ACTORS_AND_CARRIAGE_PLAN.md       ← ⭐ MEASURED + PLANNED, one diagnostic built,
                                    no proposal implemented. WHO MOVES THE CARGO:
                                    `econ_measure_carrier_mix` finds **95.7% of all
                                    shipments are carried by nobody** (`owner = -1`)
                                    against 4.3% by the entire merchant-house layer.
                                    The ownerless branch needs no vessel slot, is not
                                    clamped by capital, and NEVER SINKS (`let lost =
                                    if owner >= 0 {..} else { false }`); the transfer
                                    itself sits outside the carrier resolution, so
                                    fleet/capital/risk govern who PROFITS, never what
                                    MOVES. House carriage cannot scale (5.5× the
                                    fleet on the large world → a LOWER house share).
                                    Names the frictionless residual as the likely
                                    cause of F2's −0.064 price/distance gradient.
                                    Eight gated proposals (N1 make LOCAL_HAUL_DAYS
                                    bind · N2 ban the lane+good not the carrier ·
                                    N3 the Company's chartered staple vs
                                    opportunistic venture + the guild→Company rename ·
                                    N4 kill `house_for`'s `.position()` incumbency
                                    bias · N5 the sailing window · N6 price-elastic
                                    demand · N7 the League · N8 make the market book
                                    honest). Carries its own recorded WRONG TURN
                                    (§5.1) and the note (§5.2) that
                                    `econ_inheritance_rules_fragment_differently`
                                    went RED then GREEN inside one day's commits —
                                    re-run it per DOSE STEP, not per phase
SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md
                                  ← ⭐ N5/N6/N7.1-7.2 BUILT, gated, zero/low dose.
                                    The design for the three proposals
                                    ACTORS_AND_CARRIAGE_PLAN left as sketches —
                                    N5 sailing window · N6 price-elastic demand ·
                                    N7 the League — each with data structures,
                                    hook sites, a zero-dose setting, gates by
                                    name and a not-built list.
                                    **Shipped this session**: N5 — `CampaignSim.
                                    base_days_season`/`season_slices` (a
                                    quantised per-lane u8 multiplier,
                                    `SEASON_SLICES = 4`, `SEASON_MULT_STEP =
                                    1/64`, capped at `SEASON_MAX_MULT = 3.0`),
                                    built at campaign start via
                                    `compute_route_days_matrix_for_season` and
                                    read through one `lane_days(a,b)` accessor
                                    wired into dispatch, the return leg and
                                    contract delivery — real live dose (not
                                    zero), since it carries no wealth-
                                    concentration risk per §5's build order.
                                    N6 — `DEMAND_ELASTICITY = [0,0,0]` (true
                                    no-op, `elastic_aggregate_mult`/`_e` split
                                    pure/parametrized for testing), applied to
                                    the category AGGREGATE outside `base_need`;
                                    a twin `needs_struct` buffer carries the
                                    STRUCTURAL need and `update_food_and_
                                    starvation` now reads it instead of the
                                    (elastic) `needs` — the one real behaviour
                                    change at zero dose, since food/starvation
                                    used to read the same buffer prices clear
                                    on. N7.1/7.2 — `League`/`Boycott` structs,
                                    `TickHub.league`, yearly formation
                                    (`maybe_form_leagues`, gated on tier +
                                    a shared trade tie + a shared threat) and
                                    the diet (`run_league_diet`, dues/drift/
                                    annexation-exit/seat-succession), a lane-
                                    scoped `Boycott` enforced in `dispatch`
                                    beside `quarantined`/`export_ban_until` —
                                    `LEAGUE_BOYCOTT_MAX = 0` (N7.3, the diet
                                    voting one, stays unbuilt/zero-dosed).
                                    Gates: `n5_season_multipliers_at_unity_
                                    are_a_noop`, `n5_a_lane_is_dearer_in_its_
                                    stormy_season`, `n6_elasticity_at_zero_
                                    is_a_noop`, `n6_a_dearer_good_is_bought_
                                    less`, `n6_the_ration_is_not_elastic`,
                                    `n7_a_world_with_no_leagues_is_bit_
                                    identical`, `n7_a_league_is_not_a_realm`,
                                    `n7_leagues_form_and_dissolve`, `n7_
                                    boycott_is_inert_at_zero`, `n7_a_
                                    boycotted_city_reroutes` — all in `tick::
                                    tests`, plus `simulate_decades_reports_
                                    dynamics` re-verified (wealth bounded,
                                    richest 278,201 over 50y). `econ_
                                    measure_league_formation` (§4.3's own
                                    named instrument) is NOT yet built — the
                                    unit-level formation/dissolution gate
                                    above stands in for it this session.
                                    **Three findings from writing it are worth
                                    more than the designs.** (1) N5 is mostly
                                    ALREADY BUILT world-side: `build_coarse_cost`
                                    takes `season`/`months` and already closes
                                    snow-shut passes and stormy sailing windows
                                    off real `storm_base`/`reef_risk`/elevation,
                                    and the campaign's own
                                    `compute_route_days_matrix` calls it with
                                    `season = -1` — "no seasonal closure". N5 is
                                    calling that per season, storing a quantised
                                    u8 per-lane multiplier (v=0 ⇒ exactly 1.0 ⇒
                                    the bit-identical gate), read through one
                                    `lane_days` accessor. Its real target is the
                                    scorecard's within-city grain price CV of
                                    **0.000** (band 0.30–0.50), not the gradient.
                                    (2) Demand is NOT "perfectly price-inelastic"
                                    — category substitution already weights by
                                    `pref / rel` where `rel = price/base_value`,
                                    so cross-price elasticity is live; only the
                                    category AGGREGATE is fixed. The design's own
                                    sharpest rule: **elasticity belongs to the
                                    market, not to the ration** — it applies
                                    OUTSIDE `base_need` (whose other callers are
                                    `need_scale`'s calibration, the starvation
                                    sums and civic provisioning), and every
                                    welfare signal keeps reading a parallel
                                    STRUCTURAL need, or a council stops
                                    provisioning exactly when prices spike.
                                    (3) N7's stated dependency is WRONG: N2 as
                                    built bans a hub × GOOD, but a boycott is
                                    lane-scoped ("we bar trade with that city"),
                                    which nothing expresses — N7 needs an N2
                                    EXTENSION, not N2 dosed. Build order inside
                                    N7 is the institution first and the weapon
                                    last, because a boycott is N2's market
                                    closure × members and N2's single-city
                                    version broke the hard wealth bound twice
MONEY_MINES_AND_GOODS_PLAN.md     ← ⭐ SLICES 1-7 BUILT AND GATED (slice 0's pure
                                    file-split refactor deferred — a hygiene move,
                                    not a behaviour change, and lower priority than
                                    landing the fixes themselves). Shipped this
                                    session: **slice 1** — a note-funded loan now
                                    RETIRES its notes on default
                                    (`BANK_FORECLOSURE_RECOVERY`), closing F1a's
                                    permanent equity leak. **Slice 2** — `Loan.
                                    arrears_months` + a solvency-based affordability
                                    test (not `wealth > pay*1.2`): a borrower who
                                    misses a payment capitalizes the shortfall and
                                    accrues arrears, defaulting only past
                                    `LOAN_ARREARS_LIMIT` (6 months). **Slice 3** —
                                    `BANK_MAX_BORROWER_SHARE`/`BANK_MAX_LOAN_FRAC`/
                                    `BANK_BORROWER_POOL_K`: no borrower may hold more
                                    than 35% of a bank's book, loans are capped at
                                    20% of reserves, and the borrower draw is a
                                    top-track-record POOL (`stable_growth_years`),
                                    never wealth-weighted (N4's own lesson: weighting
                                    by wealth/political_power measurably inverted the
                                    inheritance gate). **Slice 4** — `bank_maybe_
                                    invest` now stakes a Mine/Quarry too, via the
                                    already-typed OFFTAKE share kind (`payout: 0`)
                                    instead of dividend. **Slice 5a/5d (F2)** —
                                    `deposits::WorkingKind` (Shaft/Open/Placer/Pan) +
                                    `default_working_for(id)` replace the substring
                                    cascade for mine-vs-quarry, per-district (a
                                    `Deposit`/`MineSite` each carry their own
                                    `working`); the self-sealing `workable`/`served`
                                    checks in `province.rs`/`colonies.rs` now compare
                                    against `d.working.estate_kind()` instead of a
                                    hard-coded Mine (2), so a flooded QUARRY body can
                                    finally become workable. **Slice 5b/5c** — new
                                    `maybe_found_extraction_estate` (yearly) sites a
                                    Mine/Quarry ON its real ore body (scored by
                                    extent × depth-workability × local price ÷ a
                                    distance-scaled haulage cost, so a rich remote
                                    body can lose to a poorer reachable one), founder
                                    = a house or the seat city (D2); a shallow/
                                    moderate body self-funds, a deep/flooded one
                                    DEMANDS a real bank loan (`purpose: "mine"`,
                                    skipped entirely with no bank reachable — D3).
                                    **Slice 6a/6b** — district counts re-authored
                                    against the plan's own historical table (copper
                                    3×, diamond 3×, silver/garnet/amethyst/topaz/
                                    carnelian lowered, iron/gold/tin/lapis/ambergris
                                    unchanged), gated by
                                    `deposit_counts_span_an_order_of_magnitude`.
                                    **Slice 7** — a `Distribution::Deposits` good now
                                    bypasses the belt-mean floor wherever this
                                    province holds a real working (both
                                    `campaign_province_goods` and `campaign_province_
                                    potential`, F4); `province_good_potential_base`
                                    reads Σ(extent × depth-workability) over the
                                    province's own workings for a deposit good
                                    instead of `belt × prov_cap × land_share`; the
                                    manufactured-goods filter now runs ONCE inside
                                    `generate_and_persist_provinces` before
                                    persisting (F5), with `get_provinces`/`get_
                                    province_layer` kept only as the fix for a world
                                    saved before that line landed. NOT done: the
                                    long-run `econ_measure_finance` before/after
                                    lifespan comparison and the mines-founded-per-
                                    century measurement §7 of the plan asks for —
                                    both are `#[ignore]`d, multi-decade diagnostics
                                    outside this session's time budget; the unit
                                    gates above (verified failing on the pre-fix
                                    code by inspection) stand in for them. One causal
                                    chain: banks survive → mines get financed → ore
                                    becomes an industry → deposit goods show up in
                                    the province view. Six measured findings, of
                                    which three are outright bugs. **F1a — a
                                    DEFAULTED note-funded loan never retires its
                                    notes** (`money.rs:612-626` writes off the asset
                                    but `notes_issued` is reduced only on the
                                    REPAYMENT path at `:624`), so every default
                                    destroys 0.6 × principal of equity permanently:
                                    the same hole the comment at `money.rs:604`
                                    records closing for repayment, never closed for
                                    default. That, plus one missed payment killing
                                    the whole loan (no arrears state at all, while
                                    the borrowing house gets a one-year grace),
                                    every loan going to the single richest resident,
                                    and half of reserves in one loan at 3× leverage
                                    against a 0.6%/month spread, is why banks fail in
                                    ~5 years. **F2 — every gemstone is a QUARRY**:
                                    `estate_kind_for_good` (`mod.rs:2112`) still
                                    decides mine-vs-quarry by SUBSTRING MATCH on the
                                    good's name even inside the post-S4 DIST_DEPOSITS
                                    branch, so diamond/ruby/sapphire/emerald/jade/
                                    lapis/turquoise/marble/salt all fall to kind 8 and
                                    the ENTIRE mining mechanic (`mine_depth`,
                                    drainage, flooded-body unlocking) is structurally
                                    unreachable for them — self-sealingly, since both
                                    `workable` (`province.rs:767`) and `served`
                                    (`colonies.rs:1524`) hard-code `estate_kind == 2`.
                                    **F3 — an estate is never sited on ore**:
                                    `maybe_found_estate` maximises `base_per_capita ×
                                    demand_pressure` (so a deposit good never wins)
                                    and places at a hash offset from the parent
                                    (`houses.rs:792`), never consulting
                                    `mine_deposits` — so §8.16's eleven deposit
                                    models are generated and then read by almost
                                    nothing. **F4 — deposit goods are FILTERED OUT of
                                    the province list**: `Province.good_belt` is a
                                    coverage MEAN, so a 1-3 cell diamond district in
                                    a 3,000-cell province lands at ≈0.0318 against a
                                    `PROV_GOOD_ABSENT_BELT` of 0.0314 — a LARGER
                                    province hides a RICHER mine, while
                                    `ProvincePotential.deposits` draws the working
                                    anyway, so the plate and the list disagree.
                                    **F5 — the manufactured filter runs in 1 of 4
                                    exits** (`get_province_layer` only), which is why
                                    manufactured goods appear right after generation
                                    and vanish after a reload. **F6 — deposit counts**
                                    put diamond/jade at 2 districts and lapis/
                                    ambergris at 1 on a default world, while amethyst,
                                    topaz, garnet and carnelian each get TWICE copper's
                                    6. Eight slices (constant-split refactor · notes
                                    bug · arrears · book diversification · offtake
                                    stakes in extraction works, reusing the already-
                                    typed `Share.payout == 0` · ore-sited financed
                                    mines with shallow self-funding and deep credit ·
                                    historical + area-scaled district counts · the
                                    three province-view fixes), each with its gate and
                                    its dose. Decisions recorded in §2; §5 is a
                                    NUMBERED QUEUE of the work that follows slices
                                    0-7, each item naming what it waits for — a
                                    schedule, never a refusal
PLACES_DEMAND_AND_GROWTH_PLAN.md  ← ⭐ ALL SEVEN SLICES BUILT AND GATED.
                                    Slices 1-3 shipped as a SURGICAL variant of
                                    the plan's literal design (see below for
                                    why); 4/5/6/7 ship as dosed no-ops or
                                    small live preferences, per their own
                                    entries further down. The companion:
                                    where towns are · how big they get · why anyone
                                    trades. **Slice 6 (F9, the cheap keystone) is
                                    live in code**: `LOCAL_SATIETY`
                                    (`tick/mod.rs`) is a hub's own jading toward a
                                    comfort/luxury good it largely makes itself —
                                    `self_supply_of` compares per-capita production
                                    against the per-capita need `base_need` itself
                                    uses, and `local_satiety_mult`/its pure,
                                    testable twin `local_satiety_mult_e` (the same
                                    split N6's `elastic_aggregate_mult`/`_e` uses)
                                    shave the MARKET-FACING `needs[h][g]` by up to
                                    the dose as self-supply approaches 1.0 — never
                                    `needs_struct` (the structural ration
                                    `lack_basic`/starvation/crisis relief read is
                                    untouched, applied at the wiring site in
                                    `mod.rs` alongside fashion/cultural-taste, not
                                    inside `base_need`), and never a basic good
                                    (`need_tier < 1` is a hard no-op). Ships at
                                    `LOCAL_SATIETY = 0.0` — a true no-op, gated by
                                    `local_satiety_is_a_noop_at_zero`,
                                    `a_producing_city_wants_less_of_its_own_
                                    luxury`, `a_basic_good_is_never_subject_to_
                                    satiety`, `the_structural_ration_is_not_
                                    affected_by_satiety` — and is the one term to
                                    dose-walk before the continuous `foreign_lux`
                                    rewrite and the `stock_origin` provenance
                                    accumulator now compound on the SAME expression
                                    — see slice 7 below.
                                    **Settlements were ONE flat greedy pass**
                                    (`settlements.rs:522`, `generate_settlements`)
                                    with a single spacing radius and tier assigned
                                    afterwards, so a "capital" was just a town
                                    that scored well; and provinces (phase 7b,
                                    seeded FROM settlements at `provinces.rs:1013`)
                                    fell back to jittered FILLER seeds, so a
                                    province could hold zero settlements or five
                                    of equal rank.
                                    **Slices 1-3 are BUILT, as a SURGICAL variant
                                    of the plan's literal design rather than the
                                    literal pipeline reorder (primary pass →
                                    provinces → secondary pass as three separate
                                    phase-7/7b/7c steps).** The literal reorder
                                    would change the WorkflowPanel step sequence,
                                    phase numbering and `ColumnSet` wiring — a
                                    much larger blast radius, and the plan's own
                                    risk register calls for empirically tuning
                                    `PRIMARY_MIN_DIST` against rendered worlds,
                                    which a text-only session cannot do. Instead:
                                    **Slice 1 (F1)** — `mark_primary_settlements`
                                    (`settlements.rs`) runs as a POST-HOC
                                    classification pass immediately after
                                    `generate_settlements`' existing greedy
                                    selection completes, over the SAME final site
                                    list — it changes not one settlement's
                                    position, population, tier or count (gated by
                                    the pre-existing `the_base_settlement_set_is_
                                    unchanged`), only which ones gain the new
                                    `Settlement.primary: bool`. Ranks by `0.65 *
                                    access + 0.35 * habitability score`
                                    (`access` — coast/river-mouth/nav/crossroads
                                    — already computed inline per site, reused
                                    rather than duplicated) under a WIDE spacing
                                    radius (`PRIMARY_SPACING_MULT` = 5×
                                    `min_dist`, never halved on a river cell —
                                    stringing capitals down one valley is
                                    explicitly a SECONDARY behaviour). Always
                                    marks at least one settlement on a non-empty
                                    list. Outposts and step-7a junction sites are
                                    never primary. **Slice 2 (F2, D1)** —
                                    `generate_provinces` (`provinces.rs`) now (a)
                                    seeds from PRIMARIES ONLY when any settlement
                                    in the list carries `.primary` (`any_primary`
                                    — falls back to the WHOLE list, today's exact
                                    pre-slice behaviour, the instant nothing is
                                    marked, so an old world's settlement data or
                                    any caller that never ran the marking pass is
                                    untouched); (b) every FILLER province (no
                                    settlement fell inside it) gets a REAL
                                    settlement FOUNDED at its own best-scoring
                                    land cell (`best_cell_of_province`, one
                                    linear pass over `buf.habitability` — the
                                    SAME field the base settlement pass ranks
                                    candidates by) rather than the meaningless
                                    jittered seed cell, and never at the
                                    centroid (D4). `generate_provinces` widened
                                    its return type to also carry `Vec<Settlement>`
                                    (the founded seats); every caller
                                    (`sim_generate_provinces`, both run-alls,
                                    `ProvincePanel.tsx`/`StepSettlements.tsx`)
                                    folds them into its own settlement list —
                                    rule 34's "generating data is not loading it"
                                    discipline applied to a new artefact, not
                                    just an old one. **Slice 3 (D3)** —
                                    `place_settlement_at` takes a new `is_capital`
                                    parameter (`Option<bool>` at the Tauri command
                                    boundary, defaulting to `false` — an older
                                    frontend build stays compatible) and sets
                                    `.primary` from it; the UI is an explicit
                                    "Place as province capital" checkbox next to
                                    the existing Place Settlement tool
                                    (`uiStore.placeSettlementIsCapital`), never an
                                    implicit "manual ⇒ capital" rule. Slice 3c's
                                    route re-verification was not separately
                                    exercised (no new route code was written —
                                    Stage A's existing fall-through shortlist and
                                    `TRADE_COMPONENT_HORIZON_KM` already handle an
                                    arbitrary settlement count, and this slice
                                    adds settlements through the exact same
                                    `Settlement` list every route command already
                                    consumes). Gates: `every_province_has_
                                    exactly_one_seat`, `a_province_seat_is_not_
                                    its_centroid`, `an_old_settlement_list_seeds_
                                    from_everyone`, `primary_settlements_are_
                                    regional_not_merely_fertile`, `mark_primary_
                                    always_marks_at_least_one` (all in
                                    `provinces::tests`/`settlements.rs`), plus
                                    the pre-existing `provinces::tests` (16/16)
                                    and `goods_` (17/17, `sim::step8_
                                    biological_goods::goods_validation`) suites
                                    re-verified clean — settlement positions are
                                    unchanged so goods catchments cannot have
                                    moved. **Deliberately not measured here**:
                                    province count/mean-area/settlements-per-
                                    province before/after on a real generated
                                    world (the plan's own scoreboard ask) — that
                                    is visual cartography work a future session
                                    with the ability to render and inspect a
                                    generated world should do before tuning
                                    `PRIMARY_SPACING_MULT` further.
                                    **Slice 4 (D5/D6, F4) is BUILT.** `CAPACITY_
                                    LAND_WEIGHT` (`tick/mod.rs`) replaces a city's
                                    `founding_pop`-only growth ceiling with a blend
                                    toward `land_capacity_blend(founding_pop,
                                    land_capacity, weight)`, where `land_capacity`
                                    is the hub's own province's `prov_cap` shared
                                    among the province's live hubs by trade weight
                                    (`disease.rs::update_food_and_starvation`,
                                    precomputed `prov_trade_weight` map, skipped
                                    entirely without a province layer). D6's
                                    `works_dev_mult` (a hub's own estates +
                                    structures, capped at `WORKS_DEV_CAP`) is
                                    gated behind the SAME dose flag rather than
                                    shipped live, so the whole slice moves as one
                                    unit. Ships at `CAPACITY_LAND_WEIGHT = 0.0` —
                                    a true no-op — gated by
                                    `capacity_land_weight_is_a_noop_at_zero`,
                                    `a_well_sited_small_town_can_overtake_a_badly_
                                    sited_large_one`, `works_capacity_is_bounded`.
                                    **Raising the dose is real future work, NOT
                                    done here**: CLAUDE.md's own record for the
                                    adjacent `WORLD_AGE_DEV_CAP` shows this growth
                                    pass is CHAOTIC-SENSITIVE to uniform per-hub
                                    capacity nudges — re-run
                                    `simulate_decades_reports_dynamics`'s
                                    sustained-runaway-wealth guard per dose step.
                                    **Slice 5 (F5) was already PARTLY built**
                                    (`EMPTY_PROVINCE_FOUND_BONUS` existed for
                                    `maybe_found_settlement_colony` alone) — now
                                    extended to `maybe_found_food_colony` and
                                    `maybe_found_house_outpost` too, through one
                                    shared pure helper (`province_founding_bonus`,
                                    a PREFERENCE never a rule — an unknown
                                    province, -1, or an already-settled one is an
                                    exact 0.0). `province_is_settled` widened to
                                    `pub(crate)` for the cross-module call. Gated
                                    by `colonisation_prefers_an_empty_province_
                                    but_is_not_forced_into_one` and
                                    `province_is_settled_reads_hub_province`.
                                    `maybe_found_mining_colony` (scored off real ore
                                    deposits, not the fertile-land list) and
                                    `maybe_found_estate`/`maybe_found_route_post`
                                    (hinterland estates, not new hubs) are
                                    deliberately left untouched — the bonus is
                                    about where a new HUB forms, not an estate.
                                    **Growth: a city's ceiling is a multiple of
                                    `founding_pop`** (`disease.rs:509`) is now
                                    FIXED by slice 4 above (at a live dose). **Demand:
                                    the foreign-craving term is a BINARY SWITCH**
                                    (`production.rs:895` — a city producing ANY amount
                                    of a luxury gets ZERO import craving) with no
                                    gradient, no distance and no provenance —
                                    **`foreign_lux` in `base_need` is STILL the
                                    binary switch** (continuity was judged too
                                    high-risk to rewrite blind — it's the one
                                    other term `COMFORT_IMPORT_FRAC`'s own
                                    five-time-broken-gate history already warns is
                                    fragile — left QUEUED, not attempted). **Slice
                                    6 (`LOCAL_SATIETY`) and slice 7 (provenance +
                                    distance prestige) are BOTH BUILT.** Slice 7:
                                    `InTransit.origin_km` (real routed distance,
                                    accumulated across relay hops in the arrivals
                                    pass, `mod.rs`) feeds `TickHub.stock_origin[g]`
                                    (a decaying EMA, `STOCK_ORIGIN_DECAY`, per
                                    arrival — option (i) from the plan, not the
                                    heavier grade×provenance matrix); `FOREIGN_
                                    PRESTIGE`/`PRESTIGE_REF_KM` fold
                                    `foreign_prestige_mult_e` into the same
                                    MARKET-FACING `needs` term slice 6 uses (never
                                    `needs_struct`; never a basic good). Ships at
                                    `FOREIGN_PRESTIGE = 0.0`, gated by
                                    `foreign_prestige_is_a_noop_at_zero`,
                                    `a_far_travelled_luxury_is_wanted_more_than_a_
                                    near_one`, `distance_prestige_never_touches_a_
                                    basic_good`, `an_old_save_with_no_origin_data_
                                    is_bit_identical`. Every demand term ships at
                                    dose 0 and is walked ONE AT A TIME (they
                                    multiply the same expression), gated by
                                    `econ_expenditure_shares_resemble_a_household`
                                    — NEITHER has been dose-walked yet; that is the
                                    next session's job, one dose at a time with
                                    the other pinned at zero (ACTORS_AND_CARRIAGE_
                                    PLAN.md §5.2's lesson). Per-culture demand
                                    profiles are §5's item 8 — the largest
                                    remaining demand item, QUEUED rather than
                                    dropped: all three terms multiply the same
                                    expression, so 8 begins once 6 and 7 are
                                    dose-walked live
PROVINCE_SYSTEM_PLAN.md           ← The province layer's design + status (see FIX_PLAN B1);
                                    the shipped algorithm itself is §8.10 above
DEPOSITS_AND_MINING_PLAN.md       ← ⭐ BUILT — all five slices, gated. Ore
                                    geology (§8.16) + grade→quality rewire + txt
                                    goods import & 8 new minerals (1-3). Slice 4:
                                    MINE (`estate_kind` 2) vs QUARRY (8, split
                                    off) as genuinely different mechanics — a
                                    mine's `mine_depth` taxes its OWN upgrade cost
                                    when deep/flooded (`MINE_UPGRADE_COST_MULT`,
                                    never the baseline output, already baked in
                                    at worldgen); a quarry is gated by TRANSPORT
                                    instead (`QUARRY_INLAND_UPGRADE_COST_MULT`);
                                    mercury → silver amalgamation is a real
                                    consumable extraction input
                                    (`apply_mercury_amalgamation`); a body KNOWN
                                    `EXTENT_WEAK` now declines to a floor under
                                    pressure (D3), everything else still
                                    persists. Slice 5: the quarry/mine window
                                    (districts → workings, `ProvinceInspector.
                                    tsx`); mining SETTLEMENTS (the Potosí case,
                                    `maybe_found_mining_colony`) found on a real
                                    GREAT/WORLD-CLASS strike, boom, and DECLINE
                                    rather than die when their food lifeline
                                    fails; a settlement's trade catchment radius
                                    grows slowly with age as a pure derived read
                                    (`catchment_radius_km`, never stored per-hub
                                    state). Not built: a drawn growing-catchment
                                    DISC on the survey plate (the value is real
                                    and served; no new canvas layer renders it)
                                    and the Mons Claudianus treasury-quarry edge
                                    case. Carries the measured findings that
                                    motivated it and its own list of the work
                                    still owed
WORLD_AND_TRADE_MASTER_PLAN.md    ← ⭐ THE ACTIVE PLAN. Three parts, one
                                    dependency chain (the land decides where
                                    trade can go). Part I tectonics/rivers/
                                    provinces/shelves — slices 1,2,3,5,7,8 BUILT
                                    and gated, 4 shipped scoped-down (no
                                    per-plate Euler-pole identity), 6 not
                                    attempted. Part II outpost connectivity &
                                    the entrepôt — measured, NOT built. Part III
                                    exploration / the known world / transport
                                    modes — decided, NOT built. Read its build
                                    order first: transport modes lead, because
                                    they are the biggest lever on the measured
                                    −0.064 price/distance gradient, have an
                                    instrument that already exists
                                    (`econ_fidelity_scorecard`), and may make
                                    the entrepôt and siting rules unnecessary
ITCZ_AND_LAND_TOOLS_PLAN.md       ← ⭐ COMMITS 1-2 BUILT, 3a ATTEMPTED AND
                                    REVERTED (negative result), 3b-c NOT
                                    ATTEMPTED. Stage-1 LASSO AREA TOOLS shipped
                                    (smooth↔roughen the coast, fjords, island/
                                    volcanic-arc chains, bulk fill — each one
                                    `buf.save`d so undo and re-roll come free,
                                    §8.25) and FOUR NEW ELEVATION GENERATORS
                                    shipped (rift/horst-graben · glaciated
                                    fjordland · plateau & mesa · volcanic hotspot,
                                    all through `apply_elevation_model` so both
                                    run-alls honour them, and the >25%-disagreement
                                    gate extended from 4 models to 8 — `glaciated`
                                    vs `shape`, the pair named as most likely to
                                    fail since glaciated starts FROM shape, passes).
                                    Carries the measured ITCZ finding that
                                    motivated the climate work: **there are TWO
                                    ITCZs**, `seasonal.rs::itcz_latitude` (8°, summer
                                    hemisphere 5-35°) for the WIND and
                                    `precipitation.rs::compute_itcz_shift_zonal`
                                    (±12° plus a ±10° migration, both hemispheres
                                    0-30°) for the RAIN — different amplitudes,
                                    different land measures, never reconciled, and
                                    the overlay draws both. **Unifying them (3a) was
                                    tried both directions and both regress a
                                    hard-asserted Earth-gate floor** — wind-adopts-
                                    rain costs exact-zone (39.0%→38.1%, floor 38.8%),
                                    rain-adopts-wind costs BOTH main-class (70.2%→
                                    68.4%, floor 70.2%) and exact-zone (→38.5%) —
                                    because both formulas were independently already
                                    tuned to the Earth gate, so a straight swap only
                                    discards whichever side's tuning gets overridden.
                                    Both reverted; the two-formula status quo ships
                                    unchanged. Full table in `docs/FIX_PLAN.md` A4.
                                    FIX_PLAN A4's pressure field (3b-c) was never
                                    attempted, since the plan built it to depend on
                                    3a landing first. Baseline measured at `7786da8`:
                                    Mumbai 161 mm (real ~2200), Bangladesh 84 mm at a
                                    **25% summer fraction** — it rains more in winter
                                    there, i.e. the monsoon runs backwards, not
                                    merely weak — STILL UNFIXED, since the
                                    prerequisite unification did not land.
CITY_PROVINCE_WAR_PLAN.md         ← ⭐ APPROVED, NOT YET BUILT. The next three workstreams:
                                    the settlement panel rework · provinces (enclave fix,
                                    sizing, real-terrain view, goods & exploitation) ·
                                    the political layer (city leader as a house office,
                                    city tiers, the city-as-state, and war). Carries its
                                    own caveat list (§5) — incl. that it REVERSES
                                    PROVINCE_SYSTEM_PLAN's "enclaves survive" decision —
                                    and its own §6 list of the work still owed
TRADE_STAGING_AND_POSTS_PLAN.md   ← ⭐ SLICES 3-4 BUILT AND DOSED LIVE (cargo no
                                    longer teleports: the entrepôt composition +
                                    the range-based staging relay, both real,
                                    both chain through `InTransit.via`/`hops`);
                                    slices 1, 2, 5, 6, 7 not built. This entry
                                    was stale (said "nothing built") until a
                                    2026-09-20 session corrected it while
                                    building `PORT_COMPETITION_PLAN.md` — see
                                    that doc's §0 for exactly what was
                                    mis-diagnosed and how it was checked
                                    against the real dispatch/arrivals code.
                                    Seven slices that
                                    make a long lane a RELAY instead of a teleport.
                                    Premise: a 7,000 km lane is historically ordinary;
                                    one with no stops, no middleman and no city grown
                                    along it is not. Carries the measured findings that
                                    motivated it — the trade HORIZON is 0.24×world_w
                                    (**9,617 km** on a 3600 grid, so the reported
                                    7,000 km lanes are legal by design), the three
                                    horizon-bypassing rescue passes price their lanes at
                                    FLAT 55 km/day with no terrain multiplier (so an
                                    absurd long lane is the CHEAPEST on the map), travel
                                    mode is `coastal_a && coastal_b` alone (a river or
                                    lake city is never "sea", so all its trade reads as
                                    overland), the campaign route matrix WAS built with
                                    `rivers_json = ""` so `is_river` was ALL FALSE and the
                                    river cost rungs never reached — **that one is now
                                    FIXED** (`metadata["rivers"]` is kept in sync by
                                    `persist_rivers` and read by
                                    `compute_route_days_matrix_for_season`; the finding
                                    survives only for hubs founded DURING a campaign,
                                    which still fall back to a straight line — see
                                    `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` §2),
                                    `hub_pull` is
                                    applied TWICE on the same axis, and loss is a flat
                                    per-shipment roll independent of distance. Also the
                                    three things that already exist and make the plan
                                    cheap: `house_barred` + `pay_to_regain_markets` (the
                                    embargo weapon, 70% built), the full
                                    outpost→colony→free-city ladder incl. a shipped
                                    `age>=50 && pop>=15_000` independence gate, and
                                    `neighbor_path` — the sim ALREADY moves people in
                                    legs; cargo is the only thing that teleports. Its
                                    own risk register names the one likely to bite:
                                    embargo as a WEALTH-CONCENTRATION SPIRAL against the
                                    hard-won top-10% share. Companion UI schematics
                                    ("Break of Bulk", 8 plates) are an artifact, not a
                                    repo file (§2.2)
PORT_COMPETITION_PLAN.md          ← ⭐ SLICE 1 BUILT AND GATED (dosed at zero — a true
                                    no-op); slices 2-3 QUEUED. Grew out of a user
                                    report that the map showed no port where cargo
                                    breaks bulk. Corrects this session's own first,
                                    wrong diagnosis (§0): the map's relay MARKER was
                                    wired to a redundant, independently re-guessed
                                    `route_outlet` lookup instead of the real
                                    `InTransit.via` already sitting on every in-flight
                                    shipment — fixed, no guessing left in
                                    `campaign_merchant_routes()`. Also records that
                                    "big ports act as trade hubs" already ships
                                    (`hub_class`/`relay_count`/`is_transit_knot`, real
                                    map markers — blue square/red triangle/teal
                                    hourglass — since before this session; nothing
                                    built here for that ask). New: a small teal ring
                                    at every GEOMETRIC land↔sea/river medium change
                                    along a routed corridor
                                    (`OverlayManager.drawMediumTransitionRings`),
                                    distinct from the economic relay ring — cargo
                                    cannot sail on land, so this is real regardless of
                                    whether dispatch treats the voyage as one leg or
                                    several. And `TickHub.transit_toll_mult` — a
                                    port's own anchorage/staple due, biasing which
                                    outlet wins the entrepôt role in `rebuild_routes`
                                    so two nearby ports can genuinely compete on price
                                    for the same relay trade — shipped at
                                    `PORT_TOLL_COMPETITION_DOSE = 0.0`, proven inert by
                                    `port_toll_competition_is_a_noop_at_zero_dose`.
                                    Raising the dose needs its own `#[ignore]`d
                                    firing-rate diagnostic first (the `maybe_grant_
                                    provinces`/R4 precedent) then a proper dose walk
                                    against `econ_` + the multi-seed inheritance gate —
                                    neither done, both queued by name, not silently
                                    assumed safe
MERCHANT_VESSELS_AND_INFORMATION_PLAN.md
                                  ← ⭐ DESIGN, NOT APPROVED, NOTHING BUILT. Six
                                    staged changes to the trade mechanism, built on
                                    one finding: a VESSEL IS NOT A THING. `fleet_sea`/
                                    `_river`/`_caravan` are three counters on `House`
                                    with no identity, location or cargo; `dispatch`
                                    decrements one slot per shipment REGARDLESS of
                                    quantity, and `SHIP/BOAT/CARAVAN_CAPACITY` are
                                    read only by futures-contract delivery. So one
                                    shipment carries exactly one good, and "which
                                    vessels are in port" is missing STATE, not a
                                    missing query. Stage 1 makes a `Vessel` real
                                    (manifest, capacity that binds, port dwell);
                                    stage 4 is the substantive one — a house trades on
                                    the price it BELIEVES, with a spread set by how
                                    fresh its knowledge is (never been → surveyed →
                                    office → controls the seat), which is the
                                    information-decay mechanism Persson/Federico
                                    credit for real market integration and the most
                                    plausible fix for the measured −0.026 gradient.
                                    Stages 5-6 generalise the EXISTING `envoys.rs`
                                    into survey agents and finally wire the EXISTING
                                    `RouteProspect`/`establish_corridor` loop into
                                    trade (today a corridor feeds only the overlay).
                                    Stage 7 is the STAPLE RIGHT — a house holding a
                                    seat sets the price on its chartered goods within
                                    a band, and rivals learn to route elsewhere; it
                                    REQUIRES stage 4, because price-setting with no
                                    way for rivals to learn the price is bad is upside
                                    with no downside. Carries its own gates per stage
                                    — including the companion gate that matters most,
                                    that long-haul trade VOLUME must not collapse —
                                    and its own list of the work still owed. Four
                                    design decisions are recorded at the top
                                    (individual vessels · houses+guilds only ·
                                    privilege staged as terms-then-price · no code yet)
REALM_AND_GOVERNMENT_PLAN.md      ← ⭐ R1-R5 BUILT, each partially. THE FIRST
                                    COUNTRIES: a merchant house takes a city
                                    (`captor_house`), PROCLAIMS sovereignty after a
                                    hard year-50 floor + a decade holding it
                                    (`maybe_proclaim_realms`), and is ELEVATED —
                                    its wealth and trade assets become the crown's,
                                    the house leaves the merchant world (`crowned`,
                                    never `defunct` — §5.1) and becomes a dynasty
                                    with a real GENEALOGY (`Realm.family` — persons,
                                    births, child mortality, aging, succession by
                                    the culture's LineRule, regency for a minor
                                    heir; `tick/realms.rs`). Realms hold provinces
                                    (`prov_realm`, a THIRD authority layer above
                                    rule 24/25), and `compute_states` now reads
                                    this real persisted state rather than deriving
                                    one — a realm outlives its capital's tier later
                                    dropping (the Karakorum rule). TAXATION is real:
                                    the harvest tithe redirects to the crown scaled
                                    by COLLECTION EFFICIENCY (cohesion × distance
                                    from the capital — "pre-modern states were
                                    limited by what they could collect, not what
                                    they charged"), plus two crown-set levies (poll,
                                    customs) and TAX FARMING (a house buys N years
                                    of tithe collection for cash now — `publicani`/
                                    *iltizam*). Realm coin deliberately deferred
                                    (§7 of the plan) rather than rushed into the
                                    tuned `money.rs` coinage system. WAR gained three
                                    priced goals — Humiliate/Enthrone/Vassalize —
                                    and realm-aware resolution: a sovereign hub's
                                    TRUE ruler is its crown, and a ceded province or
                                    annexed member city now correctly carries its
                                    sovereignty with it (`WAR_GOAL_PROVINCE`/
                                    `WAR_GOAL_ANNEX`, previously silent corruptions
                                    of rule 25 the moment either touched sovereign
                                    territory). `war_affordable_treasury` fixes an
                                    R3 side effect that would have made every realm
                                    systematically too poor to declare war. Surfaced
                                    in the frontend Realms panel (relabelled from
                                    States). STILL UNBUILT: realm coin; the "one
                                    war, one score, many cities" MULTI-CITY pooling
                                    §1.4 itself names (R4 ships the war GOALS, not
                                    this); separate peace; the two-war penalty;
                                    free-city war participation; annexing/
                                    vassalizing a realm's own CAPITAL (a full
                                    foreign-crown conquest, explicitly guarded off
                                    in `apply_war_goal` pending its own design).
                                    **THREE FORMATION PATHS now, not one.** Every
                                    realm used to be a merchant republic wearing
                                    the word "Kingdom": both eligibility paths ran
                                    through a house. `Realm.founding_path` adds
                                    PATH B (`maybe_proclaim_city_realms` — a tier-1
                                    city proclaims for itself, the FIRST reader
                                    `hub.tier`/`hub.standing` has ever had) and
                                    PATH C (`maybe_proclaim_culture_realms` — a
                                    contiguous single-culture bloc of at least
                                    `REALM_CULTURE_MIN_PROVINCES` provinces
                                    unifies under its largest city, over
                                    `prov_culture` + `prov_neighbors`; cite the
                                    constant, never a number — this line read
                                    "≥4" against a shipped value of 2 until a
                                    v2.0 audit caught it).
                                    `Realm.government` splits DYNASTIC from CIVIC:
                                    a republic (`found_civic_realm`) has no
                                    `family`, no succession by birth, and
                                    `ruling_house = u32::MAX` — every reader must
                                    resolve it through `houses.get`, never index it
                                    raw (war.rs did, and the first republic to win
                                    a war would have panicked the tick).
                                    **THREE DEAD FIELDS REVIVED**, together because
                                    they are one mechanism: `update_realm_cohesion`
                                    (yearly) drifts cohesion toward the founding
                                    path's target, dragged down per culturally-
                                    FOREIGN province held — the brake on unlimited
                                    expansion — and nudged by `legitimacy`, which
                                    now finally has a reader; `assign_realm_ranks`
                                    (yearly) is the percentile ladder + top-rank
                                    absolute floor + hysteresis that `Realm.rank`'s
                                    own doc already described and nobody had
                                    written, with COHESION as one of its four axes;
                                    and `realm_title_for(rank, government)` replaces
                                    the flat four-name list that styled a house
                                    holding one town "King". **The measured result
                                    is a NEGATIVE one and matters more than the
                                    code**: a matched before/after of
                                    `econ_measure_realm_formation` gives 8 realms
                                    by year 170 BOTH before and after, because the
                                    reference world cannot express either new path
                                    (it seeds `prov_culture` as `Culture{i}`, a
                                    different culture per province, and never seeds
                                    `prov_neighbors`, so Path C early-returns; and
                                    its 30 undifferentiated cities never clear tier
                                    1's absolute standing floor). Both paths are
                                    gated by unit tests instead. That null result
                                    was then FIXED by building an instrument that
                                    works: `realm_reference_world` +
                                    `econ_measure_realm_paths` (72 cities · 24
                                    provinces · six peoples in contiguous blocs · a
                                    real neighbour graph · a rank-size city
                                    spread), kept SEPARATE from the scorecard's
                                    world because `prov_culture` feeds migration.
                                    Measured there: merchant-only 8.0 realms/
                                    century leaving 9 of 24 provinces permanently
                                    STATELESS, versus 11.5/century and 22 of 24
                                    under a crown with all three paths — the
                                    ablation is the evidence the paths matter.
                                    Three bugs it exposed that review had not:
                                    sovereignty was DOUBLE-ASSIGNED (a coronation
                                    collected by `prov_holder` without checking
                                    `prov_realm`, so two realms could list one
                                    province — rule 27's layers are independent and
                                    taking an owned province needs a war); Path C
                                    could never fire (it required the WHOLE culture
                                    bloc free, so one proclamation anywhere
                                    foreclosed a people's nationhood forever — it
                                    now unifies whatever of itself is still free
                                    and runs FIRST, since unification happens
                                    against existing statelets); and LANDLESS
                                    realms (a city proclaiming over a province
                                    another crown held — `has_free_province_at`
                                    gates both seat paths now). `REALM_RAMP_YEARS`
                                    removes the year-50 cliff so crowns fade in
                                    over a generation rather than all appearing in
                                    one year. **`suppress_realms`** exists for ONE
                                    caller — `econ_inheritance_rules_fragment_
                                    differently`, whose 60-year window overlaps the
                                    realm era; a coronation moves a whole house's
                                    fortune out of the merchant pool at once (§5.2,
                                    "crowns drain the merchant pool") and that
                                    swamped the gate's wealth signal and INVERTED
                                    it. Isolating it is the same discipline as
                                    fixing the seed and the world; realm formation
                                    keeps its own instrument. See
                                    `docs/WORLD_REALISM_REVIEW.md` §3.5-§3.6 —
                                    including §3.6's historical verdict (Tilly's
                                    ~500 European polities c.1500 says MORE realms
                                    is right, and that nothing here CONSOLIDATES is
                                    now the binding gap).
                                    **CONSOLIDATION** closes the half of Tilly's
                                    curve the model lacked (it only ever
                                    fragmented): `realm_expansion_pass` annexes an
                                    ADJACENT free province preferring its own
                                    culture, `realm_vassalage_pass` lets a 2.5×
                                    stronger neighbour impose vassalage and, after
                                    80 years, integrate outright (the first writer
                                    `Realm.vassals` has ever had), and
                                    `realm_secession_pass` lets a culturally
                                    foreign province break away from a collapsed
                                    crown — a realm that only grows converges on
                                    one colour as surely as one that only splits
                                    (§5.6). All three are CONTIGUITY-driven over
                                    `prov_neighbors`, which is what makes a realm
                                    read as a country. The RATES are the
                                    deliverable as much as the mechanism: shipped
                                    naive, consolidation ran away (19 founded, only
                                    5 standing, 16 integrations); slowed, it holds
                                    31 founded / 21 standing with all three paths,
                                    both governments and the whole rank ladder
                                    occupied. Partition is now CONTIGUOUS
                                    (`province_hops` seeds heirs far apart and
                                    grows connected shares) instead of round-robin
                                    by index, which produced checkerboard realms;
                                    and a realm founded by a PEOPLE is named for
                                    that people (France is not "the Kingdom of
                                    Paris"). Personal union, inherited claims and
                                    conquering a foreign CAPITAL remain unbuilt.
                                    FRAGMENTATION is real (R5) but only Path A — a
                                    Partible culture's realm divides among eligible
                                    sons at EVERY succession (`partition_realm`,
                                    the same `partible_heirs` distribution
                                    `divide_estate` already uses for a merchant
                                    house), which is what finally makes the shipped
                                    `InheritanceRule` decide whether a people can
                                    hold an empire at all. Path B (contested-
                                    succession civil war) and overseas merchant-
                                    gated holdings (§3.5) remain unbuilt. The
                                    autonomy axis (`Realm.autonomy`, dormant since
                                    R1) got its first real readers — a centralized
                                    crown collects more but loses more to distance,
                                    an autonomous one the reverse. Capital moves are
                                    a real, tested mechanism triggered ONLY by a
                                    capital going abandoned (a defensive case, not
                                    a speculative "chase prosperity" AI). REVERSES
                                    CITY_PROVINCE_WAR_PLAN §6's deferral of a realm
                                    entity above cities. Carries its own caveats (§5)
                                    and §6 list of the work still owed; §7's
                                    order table records exactly what each phase shipped
TWO_APPS_AND_FILE_UPLOAD_PLAN.md  ← ⭐ SLICES 1-2 BUILT + the campaign LIBRARY and
                                    START OVER; slices 3-5 unbuilt. The audit found
                                    `save_world_as` doing `DELETE FROM campaign` after
                                    the backup while `settlements`/`economy` are CAMPAIGN
                                    keys — so no `.worldforge` file the app had ever
                                    written could start a campaign. `CAMPAIGN_KEYS` is
                                    now split into `WORLD_HUMAN_KEYS` (ships in the world
                                    file) and `CAMPAIGN_RUN_KEYS` (does not) — see §10
                                    rule 28. Implementation found a SECOND site of the
                                    same bug the audit missed: `new_campaign` wiped the
                                    table too, so Finalize → New Campaign → Begin
                                    Campaign failed on a world that had never been
                                    saved at all. Recommends NOT splitting into two
                                    binaries (the campaign seeds itself from the whole
                                    world pipeline; FIX_PLAN B1 wants that edge tighter,
                                    not process-separated). §8 of the doc records what
                                    shipped, including what it does NOT fix — old world
                                    files still carry no economy, and nothing can
                                    retro-fit data a file never contained
TECTONICS_AND_ISOLATION_PLAN.md   ← ⭐ AGREED, NOTHING BUILT. Two subjects.
                                    PART A · an ocean is a barrier: the measured
                                    cause of trans-oceanic lanes is
                                    `rescue_tiny_components`, which folds any
                                    <3-hub component into the nearest big one with
                                    NO DISTANCE CAP — so a mid-ocean island is
                                    relabelled part of a far continent and `#6`'s
                                    same-component guard (which exists precisely to
                                    stop "dishonest trans-oceanic arrows") is handed
                                    a lie about what is connected. Cap it in km
                                    (rule 25); beyond the cap a component trades
                                    internally and a city that cannot get what it
                                    needs starves and is abandoned — already a
                                    modelled outcome (`abandon_hub` sets
                                    `died_cause`), not something to invent. Also
                                    unifies the Flows highlight onto the SAME coarse
                                    grid + crossing rule the Dynamic Trade Flow
                                    layer already uses, so the dashed direct-line
                                    fallback (rule 35) becomes rare rather than
                                    routine. PART B · tectonic character, appearance
                                    level but DERIVED FROM the Euler-pole motion
                                    field: power-law plate SIZES (today's jittered
                                    grid makes them all alike; Earth spans three
                                    orders of magnitude), a persisted MOTION layer
                                    (plates are transient today, so the velocity
                                    field that drives boundary classification cannot
                                    be drawn), collision STYLE (`geology.rs` already
                                    computes the setting and nothing uses it to shape
                                    the belt PROFILE — continent-continent should be
                                    broad and MULTI-RIDGE, ocean-continent narrow and
                                    volcanic), and RELICT SUTURES. That last is the
                                    believability item and exists in no form today:
                                    `age` is `fbm_noise`, pure noise uncorrelated
                                    with whether a boundary is active, and is only
                                    ever assigned ON a belt that exists NOW — so
                                    there can be no Ural, no Appalachian, no Scottish
                                    Highland, all of which are former sutures inside
                                    a plate. The plan's recommendation is to GENERATE
                                    A PAST, not run a simulation: bake 2-4 former
                                    sutures with ages, height falling with age. It
                                    says plainly that this fakes the HISTORY rather
                                    than the physics
CONSUMPTION_REBUILD_PLAN.md       ← ⭐ APPROVED IN SCOPE, NOTHING BUILT. The
                                    build order for the review below, after
                                    three decisions on the maximal path: add a
                                    price→production loop (not just tuning), the
                                    population PAYS for what it consumes, and a
                                    mine's output comes from the ore body's
                                    `extent` rather than the city's population.
                                    Eight slices, each with its own gate, its own
                                    push and a gate-that-is-not-the-target; the
                                    three behavioural ones (price loop, ore
                                    ceiling, households) each dosed from zero on
                                    the N1/N6 pattern, re-running `econ_` PER
                                    DOSE STEP. **S1 is the cheap keystone**: the
                                    demand table becomes a BUDGET SHARE rather
                                    than a quantity — one division by
                                    `base_value`, no table rewritten — which
                                    alone moves food 12.4%→46.5% of consumption
                                    spend, and two constant nudges carry it to
                                    61.0% food / 8.8% luxury / gems at 0.05×
                                    bread. Names the part of S1 that is easy to
                                    miss and impossible to see (`need_scale`'s
                                    own `sum_tw_desire` must gain the same
                                    terms), and the two things NOT in scope so
                                    they are not assumed done (N1's ownerless
                                    residual; `tech_factor` pinned at its 0.85
                                    floor)
CONSUMPTION_AND_GOODS_REVIEW.md   ← ⭐ MEASURED ANALYSIS, NOTHING BUILT (one
                                    #[ignore]d diagnostic added). WHY THE
                                    WAREHOUSES ARE FULL AND THE GEMS MOVE IN
                                    BULK. Consumption is `eat = need.min(stock)`
                                    with **no counterparty** — the population is
                                    a sink, not an agent, so **there are no
                                    buyers in this economy** and no market panel
                                    can honestly show any (`supply_accum`'s five
                                    classes are all SELLERS). Measured from the
                                    shipped demand tables: food & drink is
                                    **12.4%** of a city's consumption spend
                                    (history: 60-80%), luxury tier **69.7%**, and
                                    a city spends **13.2× more on gemstones than
                                    on wheat**. Measured in the reference world
                                    (`econ_measure_goods_stock_and_price`): 314
                                    days of grain and 1,499 days of silk held in
                                    YEAR ONE, rising to 29.5 years and 197 years
                                    by year 100, with `price/base` pinned at
                                    0.14-0.52 against a `PRICE_FLOOR_MULT` of
                                    0.15 (one excursion, iron at 0.94). Four causes, each sufficient:
                                    consumption capped at the ration so 100% of
                                    surplus is retained; production reads no
                                    price ANYWHERE (`made = by_inputs.min(
                                    labor_cap)`); **31 of 45 goods ship
                                    `perishable = 0.0`** so spoilage
                                    early-returns and `wh_capacity` — whose only
                                    effect is a multiplier ON the spoil rate — is
                                    provably inert for them; and `need_scale` is
                                    ONE aggregate scalar over all 45 goods, so at
                                    most the AVERAGE is right and each good's
                                    level is an accident of two tables nobody
                                    compared. **The manufactory shortage is the
                                    same bug**: `maybe_found_guild_workshop`
                                    gates on `demand_pressure_at >= 1.08` while
                                    that function is `(price/base).clamp(0.6,
                                    3.0)` whose measured MAXIMUM across every
                                    good and date is 0.94, so the gate is never
                                    once satisfied anywhere — unsatisfiable, not
                                    merely rare; `maybe_found_estate` uses the
                                    same figure as a SCORE not a GATE, which is
                                    the whole asymmetry. Also: `estate_kind` is
                                    guessed by **substring match on the good's
                                    name** rather than read from `distribution`
                                    (the icon bug's root, and a stale-table
                                    failure its own doc comment already records);
                                    the strata are a single clamped 0.4-1.8 tilt
                                    with the comfort tier literally neutral, and
                                    `Pop` is inert, so a Vic3-style pops WINDOW
                                    would imply causation the sim does not have.
                                    Ends with 8 gated proposals in build order
                                    and 6 questions that need a decision before
                                    any of it starts
INSTITUTIONS_BUILD_ORDER.md       ← ⭐ PHASES 0, 1, 2, 3.1, 4.1, 4.2, 4.3, 4.4
                                    AND 5 BUILT AND GATED; 4.5 BUILT BUT DOSED
                                    AT ZERO. The sequenced order for the brainstorm below,
                                    after four maintainer decisions: the player
                                    stays OBSERVATION-ONLY (so "play the bank" is
                                    REJECTED, not deferred) · all three chains
                                    sequenced in one plan · MEASURE the craft
                                    spread before choosing its dose · Murano
                                    scoped as ONE named feature, not a guild
                                    rework. Its governing rule follows from the
                                    first decision and is harder than a player
                                    verb: **every mechanism must produce a legible
                                    STORY, not a decision** — a slice names what it
                                    writes to the chronicle or it is not done.
                                    Phase 0 = three instruments, no production
                                    code: `econ_measure_finance` and `real_world_
                                    craft_spread` (see SCOREBOARD 2026-09-13 for
                                    what they measured — the second now shows 0%
                                    "leader is simply the biggest city", down from
                                    100% before Phase 2); `econ_measure_war_trade`
                                    still isn't built. **Phase 1 (shipped)**:
                                    `resolve_bank_failure` tries WOUND DOWN
                                    (liquidation covers deposits+notes — deposits
                                    alone let almost every failure "succeed"
                                    quietly and drove crashes toward 0/century)
                                    then ABSORBED (a solvent same-component rival
                                    buys the book) before the old COLLAPSE path,
                                    which alone still ignites
                                    `trigger_regional_crash`; a bank may now hold
                                    the Monte (`kind == 1` in every coupon/
                                    deleverage/haircut/default loop, plus a fresh
                                    bank-subscribes-headroom path); `League.purse`
                                    funds a season's convoy escort
                                    (`LEAGUE_ESCORT_COST`/`_LOSS_MULT`) and
                                    chronicles an unaffordable renewal too, not
                                    just a funded one. **Phase 2 (shipped)** — the
                                    Murano feature: `TickHub.tradition` (years of
                                    practice, faster under a guild, decaying when
                                    idle) fully replaces the old population
                                    `size_bonus` in the quality cap; a guildhall
                                    accrues `CraftGuild.secrecy` that subtracts
                                    from `maybe_steal_quality`'s roll; `maybe_
                                    poach_master` (new) lets a rival bribe away a
                                    master, carrying `MASTER_POACH_TRADITION_FRAC`
                                    of the source's tradition to the destination
                                    at a real cost to the source guild's strength
                                    +secrecy; a signature (`brand_name`) is earned
                                    once tradition clears `SIGNATURE_TRADITION_
                                    YEARS` AND quality clears `SIGNATURE_QUALITY_
                                    FLOOR`; a high-throughput, import-dependent
                                    hub (`ENTREPOT_THROUGHPUT_FLOOR`/`_IMPORT_
                                    DEP_FLOOR`) gets a manufacture efficiency
                                    bonus on the input TAKE only, chronicled once
                                    via the new `LAW_ENTREPOT`. **Phase 3.1
                                    (shipped)**: a hub at war raises its own debt
                                    target (`WAR_DEBT_TARGET_MULT`, still bounded
                                    by the pre-existing `DEBT_MAX_RATIO`/
                                    serviceability gate) and the Monte-opening/
                                    civic-default chronicle lines now name the war
                                    and the banks a default hits — closing the
                                    war→debt→bank-holds-it→haircut→fails→crash
                                    chain from four systems that already existed.
                                    **Phase 4.1 (shipped)**: `CONTRABAND_GOODS`
                                    (metalware/iron/timber/pitch/hemp) is checked
                                    live in `dispatch` against `TickHub.war_with`
                                    — lane-scoped to the actual enemy, unlike
                                    `export_ban_until`'s blanket ban — and every
                                    declare-war chronicle line names which of the
                                    goods this world carries are barred. **Phase
                                    4.2 (shipped)**: a lane between two hubs
                                    actually `war_with` each other is diverted
                                    through the SAME neutral-stop `staging_hop`
                                    relay N1/N1c already use (`BLOCKADE_STAGING_
                                    DOSE`, shipped at 0.0 — a true no-op, since
                                    a bare refusal at this shape of dose has
                                    twice collapsed the inheritance gate's world
                                    per N1/N2's own history), chronicled once per
                                    war on the first shipment actually diverted
                                    ("the war between X and Y forces cargo to go
                                    by way of Z"), never once per shipment.
                                    **Phase 4.3 (shipped LIVE, not dosed)**: a
                                    member-to-member League lane pays cheaper
                                    freight (`LEAGUE_FREIGHT_DISCOUNT` 0.85, the
                                    same shape `GUILDHALL_FREIGHT` already uses)
                                    and a reduced tariff on both ends
                                    (`LEAGUE_TARIFF_MULT` 0.5) — the first thing
                                    beyond Phase 1.3's convoy escort that makes
                                    League membership matter, shipped live rather
                                    than dose-walked because a discount on an
                                    already-taxed, narrow lane class (96% of
                                    shipments move ownerless and never reach
                                    either check) carries none of the
                                    concentration risk a routing or exclusion
                                    mechanism does. **Phase 4.4 (shipped)**: THE
                                    KONTOR — a league with a purse that clears
                                    `KONTOR_COST` establishes one shared depot at
                                    the non-member hub it trades with most, once
                                    that tie clears the same `LEAGUE_FLOW_MIN`
                                    threshold formation itself uses; members
                                    trading through that host get the SAME 4.3
                                    privilege as a fellow member would
                                    (`lane_league_privileged`, the one pure
                                    decision `dispatch` reuses across all three
                                    call sites); the host may expel it
                                    (`maybe_expel_kontors`, likelier while at war
                                    with a member or under high unrest),
                                    chronicled on both establishment and
                                    expulsion. **Phase 4.5 built, dosed at zero**:
                                    the diet's target-selection rule
                                    (`choose_boycott_target` — prefers a real war
                                    a member is fighting, the Denmark 1361-70
                                    case, else the seat of the strongest
                                    threatening rank-≥2 realm) is real code,
                                    tested, and wired into `run_league_diet`, but
                                    `LEAGUE_BOYCOTT_MAX` remains 0 — the plan's
                                    own text calls dosing this "last, and expect
                                    trouble" (N2's related cargo-ban mechanism
                                    broke the hard wealth bound twice at low
                                    doses), and walking it above zero needs its
                                    own session with room for the full
                                    multi-seed-inheritance + dense-world gate
                                    sweep per dose step, which this one did not
                                    have left. **Phase 5 (shipped)**: a fifth
                                    `ProvWork` kind
                                    (`WORK_CADASTRE`, crown-funded, state-
                                    infrastructure-tier) permanently multiplies a
                                    surveyed province's tithe collection by
                                    `PROV_CADASTRE_EFFICIENCY_MULT`; an active tax
                                    farm now costs real rural unrest + crown
                                    cohesion every year it stands, which is what
                                    makes the cadastre matter rather than merely
                                    exist. §6 lists what is deliberately out.
                                    **`econ_inheritance_rules_fragment_
                                    differently` is confirmed pre-existing broken,
                                    independent of any of this** — it fails
                                    identically on the commit before this whole
                                    plan's work began; not a regression this work
                                    introduced, and left unchased per §8.15's own
                                    caution against tuning a dose against a stale
                                    or already-distorted measurement.
INSTITUTIONS_BRAINSTORM.md        ← ⭐ BRAINSTORM, NOTHING APPROVED OR BUILT.
                                    Variants for six institutions the maintainer
                                    asked about — banks as a player · war's real
                                    trade impact · craft guilds/the Murano case ·
                                    a bureaucratic apparatus · settlements with
                                    unique production · trade leagues. Its §0 is
                                    the finding that unifies them: **nothing in
                                    this world can forbid anything.** `dispatch`
                                    honours exactly five prohibitions (plague
                                    quarantine · famine food lock · `export_ban_
                                    until` at INFINITY · league boycott at 0 ·
                                    `house_barred`, authored only by colonies) and
                                    **WAR IS NOT ONE OF THEM** — `dispatch` never
                                    reads `self.wars`, so two cities at war trade
                                    at full volume every day; the "blockade" only
                                    scales `export_earn` (real for a city's
                                    prosperity, zero effect on cargo). Every one of
                                    the six is a different answer to "who may
                                    exclude whom from what". Other measured items:
                                    `League.purse` has ONE writer and ZERO readers
                                    (dues accumulate forever, unspent); glassware
                                    and ceramics DO exist (belt goods converted to
                                    recipes, glassware ← bay_salt + timber) and
                                    `maybe_steal_quality` already implements
                                    industrial espionage, so the Murano THEFT half
                                    is built and the SECRECY half is not; and the
                                    quality ceiling is `0.62 + size + structures`,
                                    where the size term SATURATES at 60,000 pop —
                                    and `real_world_craft_spread` then FALSIFIED
                                    this file's own first prediction: the finest
                                    maker is the biggest city **0 times in 14
                                    goods**, because every real city converges on
                                    the SAME ceiling (0.960) and there is no leader
                                    at all, only a tie. `LAW_GUILD_MONOPOLY` (0.97)
                                    is the one thing that lifts anyone above it, so
                                    a guild charter is already the only mechanism
                                    producing a distinctive maker. Three
                                    interlocking chains proposed rather than six
                                    projects: money-and-consequence (banks stop
                                    dying → war paid by borrowing → a bank holds
                                    the bond → a lost war breaks it, the 1345
                                    chain, four systems that each work and are not
                                    connected) · exclusion (contraband → a routed
                                    blockade → league privileges → the staple
                                    right, every one routing through a neutral
                                    rather than refusing, per N1c/N2's measured
                                    lesson) · why-a-place-is-itself (untie quality
                                    from city size → a craft can be kept and lost →
                                    the signature → the entrepôt). §8 carries seven
                                    open questions that gate the work
BANKS_MONEY_AND_CRAFT_PLAN.md     ← ⭐ REVIEW + PLAN, NOTHING BUILT. Banks, coinage
                                    and the craft guilds, measured against the
                                    standing dynamics run rather than against the
                                    code. Headline: all three are modelled well
                                    INTERNALLY and are terminal — `price_level` is
                                    computed by a real quantity-theory loop and read
                                    by NOBODY (`live_price` is `base·(need/stock)^k`,
                                    no money term), `coin_exchange`'s bimetallic
                                    ratio prices only a bank's own bill income (a
                                    merchant crossing currencies pays no spread), and
                                    a craft guild's entire economic power is one
                                    quality ceiling. Money's ONE channel into trade is
                                    `coin_discount`'s ≤10% freight shave above
                                    `RESERVE_TRUST_MIN` 0.55 — and the measured run
                                    puts average coin trust at 43% from year 30 on, so
                                    that channel is CLOSED for half the campaign,
                                    because 12 crashes in 50 years (every `fail_bank`
                                    calls `trigger_regional_crash`) keep hammering
                                    trust down. Two subsystems in a loop neither was
                                    designed against; invisible without the digest.
                                    Also: nobody BORROWS (`bank_maybe_lend` rolls a
                                    die and pushes cash at the RICHEST resident, no
                                    project attached), `BANK_CREDIT_MULT`'s 1.6×
                                    "trades on credit" is an archetype perk connected
                                    to no bank and never repaid, no bank may hold
                                    public debt though CLAUDE.md claimed otherwise
                                    (B-F4, now corrected above), and the world holds
                                    **12 craft guilds total, seeded once at tick 0 and
                                    never founded or dissolved again**, against 23
                                    manufactured goods — Florence alone had 21 arti.
                                    Eight slices, gate each: 0 the missing instrument
                                    (`econ_measure_finance` — there is no `econ_*`
                                    diagnostic for finance at all) · 1 split bank
                                    failure from contagion (cheapest, re-opens the
                                    coin channel) · 2 let a bank hold the Monte ·
                                    3 credit demanded not pushed · 4 make
                                    `price_level` spendable (dosed from zero; touches
                                    `live_price`, may not survive) · 5 make someone
                                    pay the bill · 6 A6/Fugger, loan→arrears→offtake ·
                                    7 crafts as a live population. §5 lists what is
                                    deliberately NOT proposed (wages/labour is
                                    FIX_PLAN Part C, not a finance change); §6 maps
                                    what SYSTEMS_21_PROPOSALS, SOCIAL_ECONOMIC_WEALTH_
                                    PROPOSAL (whose "banking is dormant, lower the
                                    founding bar" reading is now measurably STALE and
                                    backwards) and ESTATES_SHARES A5/A6 already said
CITY_TRADERS_PANEL_PLAN.md        ← ⭐ AGREED, BACKEND GROUNDWORK BUILT AND INERT,
                                    UI NOT BUILT. A third tab beside Market/Flows:
                                    WHO TRADES HERE (carriers by volume/standing/
                                    route length/carriage, with an import-export
                                    filter) over WHO IS ESTABLISHED HERE (offices,
                                    bailos, the council seat, capture) — two lists
                                    because they routinely disagree, a house can
                                    seat a council and carry nothing. Carries the
                                    measured finding that shapes every decision in
                                    it: `econ_measure_carrier_mix` reports **96% of
                                    shipments move on no house's account**, so the
                                    panel will read "local merchants 96%" on nearly
                                    every city — which is the finding, not a defect,
                                    and must never be suppressed to flatter the
                                    house list. Also records why the transit column
                                    is called RE-EXPORT: the sim has no multi-leg
                                    voyage (cargo teleports A→B), so what is
                                    measurable is a trader landing and re-shipping
                                    the same good, and the column must not claim
                                    more than that
ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md
                                  ← ⭐ STAGES A + B BUILT (§9's own plan — dedup/
                                    parallelise the routing Dijkstras, sample the cost
                                    grid per tile, price mountain passes by minimum,
                                    build trade components from ROUTED reachability
                                    instead of Euclidean distance, guarantee every
                                    settlement a route with fall-through); §10 Q1's own
                                    PREREQUISITE for Stage C — make
                                    `econ_inheritance_rules_fragment_differently`
                                    multi-seed before touching a single dose — is now
                                    BUILT too (see §8.15's gate description); STAGE C
                                    (the carriage/economy dose walk itself) NOT STARTED.
                                    The route/connectivity/carriage counterpart to
                                    TRADE_AND_MARKET_REVIEW (which reviews the PRICE mechanism).
                                    Its headline is that the economy's problem is not
                                    missing mechanism: **ELEVEN built, wired, gated
                                    mechanisms ship at a dose of exactly zero**
                                    (N1/N1b/N2 · CAPACITY_BIND_DOSE · DEMAND_/PROD_
                                    ELASTICITY · ORE_CEILING · HOUSEHOLD_MONETIZATION ·
                                    WH_RELEASE · LEAGUE_BOYCOTT_MAX · GUILD_CHARTER_
                                    RANGE), so production ignores price, demand
                                    ignores price, carriage ignores capacity and
                                    storage ignores price — which IS
                                    TRADE_AND_MARKET_REVIEW F1's measured flatness
                                    restated as a cause. Names
                                    `econ_inheritance_rules_fragment_differently`
                                    (single-seed, 60-year) as the one instrument
                                    blocking all eleven doses.
                                    **Five measured bugs, three now FIXED (B1-B3).**
                                    (1) FIXED — mountain passes ARE priced (a ×0.45
                                    saddle discount) but the coarse cell WAS 55.7 km
                                    with elevation POINT-SAMPLED from the block centre,
                                    so a 1-10 km real pass was invisible and route
                                    quality through mountains was luck. `min_land_elev`
                                    (the block's own minimum land elevation, sampled in
                                    the same per-tile pass that also dropped the six
                                    full-grid fine arrays, §8.9-style) now prices both
                                    the base relief term and the saddle discount; the
                                    centre-sampled `elev` is kept for every OTHER reader
                                    (`path_metrics`, `flow.rs`'s land-leg kind). Gated
                                    by `a_narrow_pass_off_centre_is_still_found`.
                                    (2) FIXED — components WERE built from EUCLIDEAN
                                    distance, land and sea never consulted (`max_link =
                                    0.30 * world_w` = 12,022 km at the default grid,
                                    wider than EVERY ocean on Earth), so the world
                                    collapsed to ONE component and every
                                    `component == component` guard downstream was a
                                    no-op. `compute_routed_components`
                                    (query_commands/mod.rs) now builds components from
                                    AT MOST *k* single-source Dijkstra searches (*k* =
                                    number of components) over the real coarse cost
                                    grid, bounded by a new `TRADE_COMPONENT_HORIZON_KM`
                                    (3,000 km) via `path_allowed`'s reach-1 rule. The
                                    old "rescue tiny/lone components... any distance"
                                    fuse is RETIRED, not capped — real reachability
                                    already unions anything that can legally reach a
                                    bigger market, so a hub that cannot is genuinely
                                    isolated and may go poor. Gated by two
                                    `component_tests` fixtures (an ocean wider than the
                                    horizon splits; narrower merges).
                                    (3) FIXED — a non-hub settlement got exactly ONE
                                    candidate link by straight-line distance, dropped
                                    silently on a Dijkstra miss, a reach violation, or a
                                    shared coarse cell. `compute_trade_routes` now tries
                                    a ranked shortlist of its 5 nearest hubs, folded
                                    into the same batched-Dijkstra call, falling through
                                    to the next candidate instead of dropping the town;
                                    a town whose whole shortlist fails is logged, though
                                    surfacing that to the UI is a separate, unbuilt
                                    frontend/bridge change. Gated by
                                    `a_lesser_town_falls_through_to_its_second_hub_
                                    when_the_first_is_unreachable`.
                                    (4) freight — **the doc CORRECTS ITSELF here and
                                    the correction is the finding**; read §2's
                                    correction block, not the claim above it. First cut
                                    said `good_freight` has no mode term so distance is
                                    mispriced (30% of grain's value over 300 km, "as
                                    cheap by sea"). Literally true about the missing
                                    term, WRONG in conclusion: it used the NOMINAL
                                    `days_per_cell` (55 km/day) where `days_per_cell`
                                    is only a REFERENCE speed pinned to cost 2.2
                                    (`cost_to_days = days_per_cell·f/(OPEN_SEA_COST·
                                    100)`), never the speed anything travels. Real
                                    routed speeds: calm coastal sea **242 km/day**,
                                    nav river 60.5, flat land 30.2, hills 14.8 — so
                                    300 km overland costs **54.5%** of grain's value
                                    (not 30%) against 6.8% by coastal sea, an 8:1
                                    ratio that is exactly Masschaele. LAND speeds match
                                    history; **calm coastal sea is 2.4-4.8× too fast**
                                    against a ~50-100 km/day real effective average, so
                                    the lever is the coastal-sea cost rung, NOT
                                    `freight_per_day` and not a new mode term (C1 is
                                    rewritten). What survives: the mode penalty is
                                    PROPORTIONAL, never DIFFERENTIAL — road should be
                                    disqualifying for grain and merely dear for silk
                                    (now C1b). And it REMOVES a charge: a 4,000 km sea
                                    haul at 91% beating 1,000 km overland at 182% is
                                    historically CORRECT (Baltic grain reached
                                    Amsterdam; inland Polish grain did not), so
                                    trans-oceanic trade is §5's component bug alone,
                                    not a freight mispricing. Freight does still reach
                                    only the founding hubs — every colony founded
                                    during a campaign routes by straight line. (5) Caravan/boat
                                    capacity is declared and unwired: `CAPACITY_BIND_
                                    DOSE = 0`, so any shipment takes one slot; 96% of
                                    cargo is ownerless and never reaches the check;
                                    `cap_land` pools boats WITH caravans; and 120:40 is
                                    3:1 against a historical ~10-13:1 (a cog 150-200 t
                                    vs a hundred-camel caravan 15-20 t).
                                    **Perf** (§8 of the doc; P1/P2/P3/P5 now FIXED by
                                    Stage A, P4 improved as a consequence): ZERO rayon
                                    in all seven `query_commands/` files → rayon now
                                    covers the batched Dijkstra and the route-days
                                    matrix build; one Dijkstra per PAIR at four call
                                    sites (~500 whole-grid searches, ~2 GB of allocation
                                    churn per route refresh) → deduped to one per SOURCE
                                    via a shared `coarse_dijkstra_batch`, proven
                                    byte-identical to the old per-pair paths;
                                    `build_coarse_cost` loading 97 MB of fine fields to
                                    read 259k centre cells (390 MB on Large) → sampled
                                    per tile as it loads instead; `charter_owner`
                                    allocating n nested Vecs PER DAY → one flat
                                    `Vec<i32>`. Campaign start's ~1,000 sequential
                                    Dijkstras (`SEASON_SLICES`) are the same batching
                                    win applied at scale, now parallel rather than
                                    sequential. Four-stage plan (A free speed → B cost
                                    grid can see terrain → C carriage sorts cargo by
                                    mode → D siting) — A and B are BUILT (see the
                                    bugs list above for B1-B3's own status); **C1
                                    BUILT** — `COASTAL_SEA_COST` 0.5 → 1.6
                                    (`query_commands/mod.rs`), which the shared
                                    `cost_to_days` conversion resolves to ~75.6 km/day
                                    (was 242, 2.4-4.8× the real ~50-100 km/day
                                    pre-modern effective average), leaving `freight_
                                    per_day` and every land/river cost untouched —
                                    gated by `coastal_sea_speed_matches_the_
                                    historical_effective_average`. **Measured on a
                                    real generated world, same seed before/after**
                                    (`real_world_price_distance_gradient`,
                                    `commands/real_world_diagnostics.rs`, `#[ignore]`d
                                    — built for exactly this gap, since every `tick::
                                    tests`/`econ_` fixture is a synthetic in-memory
                                    `CampaignSim` that never touches `build_coarse_
                                    cost` at all, so neither of the two gates this
                                    dose's own doc names could ever move): grain price/
                                    distance gradient **r = 0.092 → 0.185**, nearly
                                    doubled, both positive — the historically correct
                                    sign, and the metric `WORLD_AND_TRADE_MASTER_PLAN.
                                    md` §4/F2 names as this whole area's headline
                                    failure. See that doc's "UPDATE 2" for the full
                                    pair and its caveats (one seed, one world size, one
                                    20-year run). This NECESSARILY
                                    retires the old static "sea:river:road ≈ 1:4:8"
                                    cost-grid ratio the non-findings list below once
                                    called settled (`coastal_sea_river_and_open_sea_
                                    price_in_a_sane_order` now asserts ordering, not a
                                    fixed ratio) — see §8.15-adjacent doc comment at
                                    `COASTAL_SEA_COST` for why: that ratio is a
                                    citation of Masschaele's FREIGHT-cost figure, and
                                    the cost grid conflates freight cost with travel
                                    SPEED through one shared conversion. **C1b also
                                    WIRED, at zero dose** — `good_freight` (`tick/
                                    production.rs`) gained a `sea: bool` param (every
                                    call site already computed `hubs[a].coastal &&
                                    hubs[b].coastal` nearby for display purposes,
                                    reused not duplicated); a LAND leg's `bulk` term
                                    scales by `1.0 + LAND_BULK_PENALTY * (bulk -
                                    1.0).max(0.0)`, sea untouched, a good at or below
                                    bulk 1.0 untouched on either mode at any dose.
                                    `LAND_BULK_PENALTY = 0.0` ships as a true no-op —
                                    the `CAPACITY_BIND_DOSE`/`ORE_CEILING_DOSE`
                                    pattern — proven by `c1b_land_bulk_penalty_is_a_
                                    noop_at_zero` plus `tick::tests` 216/217 (the one
                                    failure pre-existing, unrelated) and `econ_` 6/6
                                    bit-identical. Dosing it is separate, unstarted
                                    work. **C2 re-assessed, not built** — its "8×
                                    too fast" numeric complaint was pre-C1; post-C1
                                    the same formula gives coastal sea 75.6 km/day
                                    against land's unchanged 30.3 (2.5:1, inside the
                                    doc's own "~4×" citation), so the specific
                                    contradiction is gone — the architectural split
                                    (`days` still serves both cost and time) is
                                    deferred as a standing invariant to re-check, not
                                    built, given its blast radius. **C3 deliberately
                                    skipped** — `WORLD_AND_TRADE_MASTER_PLAN.md` §4
                                    already tried this exact `cap_land` split and hit
                                    an unexplained regression (a 90-year house went
                                    bankrupt split, arithmetic verified correct);
                                    re-attempting with no new information would very
                                    likely re-discover the same failure. C4 remains
                                    last by design. **D1 BUILT** —
                                    `try_found_house_outpost` (`houses.rs`) now caps
                                    distance from the METROPOLIS specifically (not
                                    just the nearest network node, which still
                                    governs scoring), adds a 150 km minimum gap, and
                                    checks mode-legal range via the SAME
                                    `leg_exceeds_range` ordinary trade already uses
                                    (a no-op in every test fixture, live only in a
                                    real campaign). `econ_diagnose_outpost_founding`
                                    now founds 1 outpost (was 2) on the reference
                                    fixture — not a regression: the reduction is the
                                    home-distance check doing exactly its job (it can
                                    only ADD a rejection relative to the old check),
                                    and founding did not silently stall (the gate's
                                    own named failure mode). `econ_fidelity_scorecard`/
                                    `_large_world` diffed line-for-line against a
                                    stashed pre-D1 run of the same commit — every
                                    printed figure bit-identical, confirmed rather
                                    than assumed (see `docs/SCOREBOARD.md`
                                    2026-09-11d). **D3 BUILT** — `hub.river`
                                    now reads three real river LANDMARKS off
                                    `sim::rivers::River` (a mouth's
                                    `mouth_kind != 0`, a tributary's own
                                    confluence point, a navigable trunk's
                                    head-of-navigation) within 25 km, replacing
                                    a blanket ~400 km "near any navigable
                                    point" test. Measured on a real 1800×900
                                    world: riverine share 10.9% → 2.0%, a real
                                    ~5.5× drop (the gate's own "must fall
                                    substantially" bar), diagnostic run at
                                    production-scale km/cell since the usual
                                    300-wide test world is too coarse for a
                                    25 km radius to mean anything (rule 21/25).
                                    **D2's RIVER half now BUILT** (the
                                    COMPONENT half remains deferred — its own
                                    entry records why): `compute_colonizable_
                                    sites` stamps each candidate site's real
                                    `river` flag via the SAME landmark test
                                    D3 uses (`sim::rivers::river_landmarks`,
                                    factored out for this reuse), wired
                                    through every founding path
                                    (`create_estate`, `create_market_colony`,
                                    the mid-campaign pool recompute) in place
                                    of the hardcoded `river: false` every
                                    campaign-founded hub previously shipped
                                    with. `econ_` bit-identical (the change
                                    only writes a field no economic pass
                                    conditions on differently yet).
                                    **C4 BUILT** — `N1_LOCAL_HAUL_BIND_DAYS`
                                    dosed to 90 days, completing Stage C.
                                    Routes an over-threshold ownerless leg
                                    through N1c's own `staging_hop` relay
                                    rather than refusing it outright (N1c's own
                                    history already proved a bare refusal on
                                    an ownerless-only bind collapses the
                                    inheritance gate), made a per-sim field so
                                    abstract fixtures opt out exactly like
                                    `ship_leg_max_km`/`caravan_leg_max_km`. A
                                    30-day first trial staged everything with
                                    zero refusals and still collapsed
                                    `dense_world` trade volume to ~10% — proof
                                    routing alone isn't automatically safe once
                                    it fires on most of a world's trade. 90
                                    days measures healthy on `tests::
                                    dense_world` over 40 years and leaves the
                                    multi-seed inheritance gate + full `econ_`
                                    bit-identical (both opt out via the field).
                                    Plus 7 open questions
                                    (§10, all answered — A→B→C order, isolated markets
                                    may starve, a ~3,000 km trade horizon, the
                                    inheritance gate goes multi-seed before Stage C,
                                    now built — see §8.15) and a NON-FINDINGS list
                                    recording that the tick is NOT the perf problem
                                    and worldgen river siting is one of the
                                    better-modelled things in the tree
IN_APP_VERIFICATION_CHECKLIST.md  ← Manual in-app verification checklist
PORTING_REFERENCE.md              ← Porting reference
```

**`docs/proposals/` — A MENU, NOT COMMITMENTS**

Twenty-five documents (feature catalogs, trade/cartography specs, Victoria-2 UI
direction, settlement and population analyses, finance/heraldry variants, roadmap
batches, and the eight-part HOUSE design series — province view · tiers/kin/goals ·
body politic · succession crisis · power struggle · faction naming ·
`HOUSE_MASTER_PLAN.md`, which critiques and sequences the rest ·
`HOUSE_INHERITANCE_AND_TERRITORY.md`, which amends it) live in `docs/proposals/`.

> If you read only one of the house documents, read `HOUSE_MASTER_PLAN.md` and then
> `HOUSE_INHERITANCE_AND_TERRITORY.md` (which amends it). Part 0 of the master plan
> records a **blocking measured finding**: a house currently lives ~12 years against a
> historical 30–90, so the politics layer has no substrate until turnover is fixed. The
> amendment adds the open risk that the too-rich and too-short-lived anomalies may be
> **one bug (overextension) or in tension** — and if they are in tension, fixing lifespan
> raises peak wealth and the phase boundary is wrong. They were being read by fresh sessions as a backlog to work through,
and they are not one: they are far more good ideas than anyone can build.

> **Do not start work from `docs/proposals/`.** Check `FIX_PLAN.md` for what is actually
> prioritised and `SCOREBOARD.md` for what is actually measured. Reach into `proposals/`
> only when a specific design question needs the original rationale.

Historical HTML/SVG mockups are archived under `docs/mockups/_archive/`; a stray
reference image lives in `docs/reference/`. The repo root holds only `README.md`
and `CLAUDE.md`.

> **Older `*_PLAN.md` docs are gone.** Twelve superseded planning documents were deleted —
> they described work that has long since shipped and were being read as commitments.
> **The systems themselves are still in the code**; the code and this file are now the
> record. For the original rationale of one:
> `git log --diff-filter=D --name-only -- docs/` finds the deleting commit, then
> `git show <sha>^:docs/<file>`.

---

## 9b. Specialist agents (`.claude/agents/`)

The project spans climatology, oceanography, economic history, demography, Rust
performance, cartography, UI design, game design and desktop release engineering.
No one person holds all nine. Each is defined as a **subagent with web + literature
research tools**, so naming the domain in a task routes to the right expert
automatically — "the biome palette looks muddy" reaches `cartographer`, "the panel
hierarchy is flat" reaches `design`.

| Agent | Domain | Advisory / can edit |
|---|---|---|
| `design` | UI/UX, panel hierarchy, onboarding, first-run comprehension | can edit |
| `cartographer` | Map symbology, palettes, hatching, labels, legends | advisory |
| `earth-systems` | Climate, atmosphere, ocean, landform physics (§8.2, Part A) | advisory |
| `economic-history` | Prices, markets, money, banking, the economy oracle (§2.5) | advisory |
| `historical-society` | Demography, plague, migration, strata, religion (Part D) | advisory |
| `game-design` | Player agency, verbs, loops, legibility, pacing (B2) | advisory |
| `frontend-engineer` | React/Pixi/Zustand, IPC bridge, type drift, FE tests | can edit |
| `rust-performance` | Profiling, rayon, memory, bit-exactness (§8.9) | can edit |
| `release-engineer` | Tauri packaging, signing, updater, CI, save compat | can edit |

Three rules:

- **Each agent file carries the constraints of its domain**, not just its expertise —
  `cartographer` knows pattern periods must divide `TILE_SIZE`, `earth-systems` knows
  Earth parameters must stay a no-op, `rust-performance` knows the Earth score cannot
  prove bit-exactness. That embedded context is most of their value; keep it true as
  the code changes, exactly as §2.7 requires of this file.
- **Advisory agents research and recommend; they do not edit.** That keeps parallel
  runs from colliding and keeps the decision with the maintainer.
- **They are suppliers, not owners.** Scope authority, taste, and deciding what "good"
  means stay with the maintainer — those are the roles that cannot be delegated.

---

## 10. Conventions checklist

1. **Steps run in order** — each phase checks prerequisites, warns if missing.
2. **WorldBuffer is the sim unit** — load all → compute → save; never per-cell during sim.
3. **Undo is tile-level** — every stroke/phase journals prior tile state.
4. **Overlays are separate from tiles** — drawn on OverlayManager's own **Canvas 2D**
   context (NOT PixiJS Graphics — Pixi draws the tile sprites only), gated by
   `visibility[type]`.
5. **Rendering is server-side** — Rust renders RGBA, frontend only displays.
6. **Cylindrical wrapping** — X wraps, Y clamps; all BFS/paint/sim respect it.
7. **New tile fields append LAST** — v2 self-describing blobs; trailing reads pad zeros (old saves load).
8. **Every `#[tauri::command]` is registered in `lib.rs`** and gets a wrapper in `bridge/` (via the @bridge barrel).
9. **New TS types mirror Rust serde structs** in `types/` (world/campaign/goods).
10. **Earth params must stay a no-op** — the EBM anomaly and `Circulation` return zero /
    exactly 30°-60° at Earth settings. Never let a planetary knob shift the Earth baseline.
11. **Phase 3's order is duplicated** in `sim_commands.rs`, `earth_validation.rs` AND
    `step3_ocean_atmo/preview.rs` — change one, change all three, or the fidelity gate
    and the settings preview stop testing/showing the real pipeline.
12. **Run the gates your change actually touches — see §2.8. Never the whole suite
    by default; `econ_` runs once per batch, at its end (§2.9).** A full `cargo test --lib` is ~an hour and most changes cannot
    affect most of it; running everything is not caution, it is a way of learning
    nothing slowly. **After any verified change** → push to `main` (§2.2), and keep
    this file true (§2.7).
13. **`biome` is descriptive** — nothing downstream may score off it (§8.12), and the
    render palette has a twin in the legend that must move with it.
14. **Generation settings go in the LEFT panel** (`StepWorldCharacteristics`, step 0,
    displayed after Landmass — step 0 renders a ⚙️ not a number, which is what lets it
    sit out of numeric order without renumbering anyone's persisted `stepCompleted`);
    the right-side Toolbar is display-only (opacity, palettes, overlay toggles). Never
    reintroduce a duplicate control in both columns — that duplication is what made the
    planet knobs and the latitude frame drift apart.
15. **Two fidelity oracles, not one.** `earth_validation.rs` scores the climate against
    the real Köppen map (§2.3); `economy_validation.rs` scores the campaign against
    published pre-modern series (§2.5). Run the one your change touches, every time.
    An oracle exists so the maintainer does not have to be a climatologist *or* an
    economic historian — that is the whole point, so don't route around it.
16. **CI enforces the gates** (`.github/workflows/ci.yml`). Most commits here are
    agent-authored and go straight to `main`, so gates that run only when someone
    remembers protect nothing.
17. **A share is not a layout.** Land use and tenure are per-province SHARES with no
    spatial extent. Render them as a stable dithered mosaic (§7 `ProvinceMiniMap`), never
    as an invented per-cell register — and keep the dither hash stable per cell, or the
    year slider shows reshuffling instead of conversion.
18. **Feud/prestige awards need a ceiling.** `prestige` is unbounded and feeds political
    power → charters → monopolies → wealth. Any new per-event prestige award must be
    capped and checked against `simulate_decades_reports_dynamics`' sustained-richest
    figure; an uncapped one took it from 298k to 1.9M.
19. **An heir is not a newborn** (§8.15). `head_lifespan` is a TENURE — what remains of a
    life that began at `head_age` — not a lifetime. Any new code that rolls a head's span
    must roll an accession age with it, or ultimogeniture, seniority and primogeniture all
    collapse into the same rule.
20. **A house's milestones are permanent.** Chronicle pruning may only ever drop chatter
    (`is_house_milestone` decides). The chronicle is the product for an observation-only
    game; a cap that eats foundings and successions deletes it.
21. **A step's gate must match its real data dependency**, not just the previous step.
    Rivers (5) genuinely needs Köppen (4): channel width comes from mean precipitation
    along the course and ice caps must not drain. A too-loose gate fails SILENTLY —
    the pipeline still runs, it just produces a subtly wrong world.
22. **A house crisis must always terminate.** `HouseCrisis.round` may never exceed
    `CRISIS_ROUND_CAP`, and a house holds at most one open crisis
    (`every_crisis_terminates`). Without this an unresolved crisis becomes the
    permanent state of a house and the politics layer silently stops meaning anything
    — the same failure mode rule 18 guards for feud prestige.
23. **A forced succession must still obey the culture's `LineRule`.** Any new code
    path that installs a head OUTSIDE the normal `succeed_house` culture-rule pick
    (a deposition, a compromise candidate, anything future work adds) must filter
    candidates through `heir_is_female` first — the crisis engine shipped without
    this once and a test caught a man taking a matrilineal house's seat within the
    hour. "Who becomes head" and "which sex may hold the house" are different
    questions; only the former is ever up for grabs.
24. **Province authority is never assumed to be a city.** Any new code that reads
    `prov_holder` (a hub) must also tolerate `prov_holder_house` (a house) holding
    the same province instead — the Stato da Mar case (§5, Phase 5). A held
    province is released only when its OWN holder house dissolves; nothing else
    (a war, a rival) currently takes it away — that gap is deliberate and recorded,
    not an oversight to silently work around.
25. **A good's DOMAIN and its DISTRIBUTION must not gate the same thing** (§8.20).
    Domain = where it can grow; distribution = how it is confined. Both gating
    "is this an island" zeroed all six endemics before the distribution could pick
    a home. Related: any size threshold about the WORLD (an island, a locality)
    is stated in km² and converted per world, never as a cell count — a cell is
    ~11 km at 3600×1800 and ~133 km on a test world.
26. **A terroir/texture term may never delete a belt.** Fine-grain terms
    (`soil`/`relief`) are remapped into `[TERROIR_FLOOR, 1.0]` and soil never
    vetoes an unlisted or unclassified class. Applied raw they pushed `tea` and
    `saffron` to zero cells. Same reasoning as the locality pass's FRINGE/FLOOR:
    shape the texture, never decide existence. After ANY change to
    `step8_biological_goods/` run `cargo test --lib goods_ -- --nocapture` and
    read the per-good table, not just the pass/fail.
27. **Sovereignty is never assumed to exist.** `prov_realm == -1` (free land) is the
    default every province starts and most still end at — a genuine third authority
    layer above rule 24 (`prov_holder` seat · `prov_holder_house` dues ·
    `prov_realm` sovereignty), all three independent and all three legal at once
    (`REALM_AND_GOVERNMENT_PLAN.md` R1, §5.9). Also: a house that proclaims a realm
    is ELEVATED (`House.crowned`), never `defunct` — it is the dynasty, not dead —
    and every pass that treats a house as a MERCHANT (tiers, goals, solvency,
    wealth sinks, succession via `head_lifespan`, crisis) must filter on
    `House::is_merchant()` rather than `!defunct` alone, or a crowned house either
    gets manufactured into bankruptcy or has its identity overwritten by a
    succession event that isn't the realm's own. Two guard fixes in R2
    (`update_solvency`/`apply_wealth_sinks` in R1b, `succeed_house`/`update_house_
    crises` in R2) were each the SAME mistake found through a different call site —
    audit every house-iteration loop a new realm-facing pass touches, not just the
    one it's adding.
28. **A campaign key belongs to the WORLD or to one RUN — never to neither, never
    to both.** `WORLD_HUMAN_KEYS` (settlements · economy) ships inside the
    `.worldforge` file; `CAMPAIGN_RUN_KEYS` (the sim, the run's name/progress/summary)
    does not. They must PARTITION `CAMPAIGN_KEYS` exactly, and
    `the_key_sets_partition_a_campaign_file` asserts it: a key in neither is silently
    dropped from every campaign save, a key in both is silently deleted from every
    world. Both are invisible until a user loses data — which is exactly how the
    original bug shipped (`save_world_as` stripped the whole table, so no world file
    could ever start a campaign). Clear a playthrough with `clear_campaign_run`, never
    with `DELETE FROM campaign`; the bare wipe is correct ONLY when a whole different
    world is being loaded over the top (`open_world`, `new_world`, `open_campaign`).
29. **A parallel history series is TAIL-aligned, never index-aligned.**
    `TradeHist.prices` was appended beside the pre-existing `vols` and is
    serde-defaulted, so an older save loads with `prices` empty while `vols`
    already holds up to `TRADE_HIST_CAP` years. Both then grow and drain in
    lockstep, so the LAST entry of each is always the same year and every reader
    must zip from the END. Never back-fill to make the lengths match — a
    fabricated price history is worse than a short one. (Rules are APPENDED here,
    never renumbered: code comments cite them by number — `git grep "rule 25"`.)
33. **A manufactured good has no ground.** `Distribution::Manufactured` goods are
    made in cities from a recipe — no belt, no deposit, no land they grow on — so
    they must never appear in a province's goods list or on a goods overlay.
    `Province.goods` is built from tile `goods` columns, which cannot tell a belt
    good from a manufactured one (it sees only bytes), and good indices are FIXED
    positions (rule 7), so a spec edited after generation leaves stray non-zero
    bytes in a now-manufactured good's column. Filter by DISTRIBUTION at the
    serving layer (`strip_manufactured_from_province_goods`), which fixes worlds
    that already exist and uses the only thing that actually knows.
35. **A flow that exists must be VISIBLE.** `renderFlowHighlight` traced every
    trade flow along the WORLDGEN trade-route graph and `continue`d when Dijkstra
    found no path — on the rule "never draw a straight slash". That rule is right
    about not faking a road and wrong about what to do when there isn't one: it
    DROPPED the flow, so a real trade the panel listed had nothing on the map. It
    is not rare — the worldgen graph is bounded by its own max open-water crossing
    while a campaign flow has its own route matrix with sea lanes and rescue
    passes, so every inter-landmass flow vanished. Where no corridor exists the
    lane is now drawn DASHED and direct (the atlas convention for an open-water
    shipping lane, honest about being a crossing), taking the shorter way round
    the cylinder. Never silently drop a datum the UI is simultaneously listing.
    **TECTONICS_AND_ISOLATION_PLAN.md Part A3 narrowed how often the dashed
    fallback fires**, rather than replacing it: a new `compute_coarse_route`
    query command (`flow.rs`) routes one point-to-point link over the SAME
    coarse cost grid + `path_allowed` crossing rule the Dynamic Trade Flow layer
    (`campaign_get_trade_flow`) already uses — the Flows highlight and the
    Dynamic Trade Flow layer used to disagree about where trade could go,
    since one read the campaign's own route matrix and the other only the
    worldgen graph. `MapCanvas.tsx` fetches a route per highlighted segment
    async into `OverlayManager.flowHighlightPaths` (a parallel array by index,
    `setFlowHighlightPaths`); `renderFlowHighlight` now tries the resolved
    coarse-grid path FIRST, `laneBetween`'s worldgen graph second, and only
    falls to the dashed direct line when neither finds a route — a campaign
    sea lane the worldgen graph never joined now draws as a real routed line
    instead of always falling back.
    **Then extended to EVERY lane, because the flow highlight was only one of
    five callers.** Merchant routes, a house's seat→city trading web, futures
    lanes and the Goods Atlas flows all go through the same `laneBetween`, so
    all of them were still drawing dashed straight spokes to their partner
    cities ("merchant/guild spread is using a direct-to-city approach, not the
    trade routes"). Rather than wire four more call sites, `laneBetween` now
    consults a coarse-route CACHE first and RECORDS the pairs it could not
    serve; `takeWantedLanes`/`setLaneRoutes` let MapCanvas drain those, resolve
    them through `compute_coarse_route`, and hand them back — which re-runs the
    same three re-snaps `drawTradeRoutes` already did when the road network
    changed. One mechanism, all five callers, no per-caller plumbing. An
    unroutable pair caches an EMPTY path deliberately, so it is not re-fetched
    on every poll and keeps its dashed fallback.
34. **Generating data is not loading it.** Both run-alls call
    `generate_and_persist_provinces`, but neither run-all HANDLER loaded the result
    into the frontend store — so a fully generated world reported "No provinces
    yet" while 900-odd provinces sat persisted in metadata. Any run-all that
    produces a new artefact must also read it back into the store, the same way
    `App.tsx` does on world open, or the user cannot tell it from a failure.
30. **Relief must be stated at a scale the grid can hold** (§8.23). A cell is
    `KM_EQUATOR / w` km wide — 11 km on the default world — so fluvial incision is
    SUB-GRID and one-cell content is capped (`limit_grid_scale_relief`). Any new
    phase-2 pass that writes per-cell detail must say what physical scale that
    detail is, and judge it by RENDERING a world (`dump_erosion_sheet`), not by
    reading the code: every cause found in §8.23 was invisible in review and
    obvious in a 4× hillshade crop. Corollary: a local mean taken near a coast is
    LAND-ONLY, or the coastline itself is measured as detail and planed down.
31. **A clamp is not a landform** (WORLD_AND_TRADE_MASTER_PLAN.md (Part I) Slice 1). Any
    pass that writes elevation must not leave a large area at exactly its floor or
    ceiling. A rank remap, a bias offset and a range clamp compose into a plateau
    at the boundary value, and that plateau then silently propagates: no gradient
    means no drainage direction, which means the meander model saturates and every
    river on it comes out the same shape. Where a pass needs a bound, scale into
    the range rather than clamping onto it (`MIN_LAND_ELEV`'s scale-about-floor is
    the pattern) — and check the result with a histogram/diagnostic, not by
    reading the code, since this cause was invisible in review and obvious in one
    histogram.
32. **`is_estate` is an OWNERSHIP flag, not a geography flag** (WORLD_AND_TRADE_
    MASTER_PLAN.md Part II, Slices A/B). An estate with `parent >= 0` is co-located
    inside its parent city and collapses to it for display and routing; an estate
    with `parent < 0` is a REMOTE SITE standing on its own ground (today exactly a
    house trade outpost) and must be routed, drawn and rescued like any settlement
    — `CampaignSim::is_remote_site` is the one predicate for this, and any new pass
    that branches on `is_estate` alone should ask whether it means "co-located" or
    "not a real place", because those are different questions and a remote site
    answers them differently. The failure is silent in both directions: a
    co-located estate given its own routing draws a zero-length route into itself,
    and a remote outpost denied it is dropped from the map/lifelines entirely — the
    Dynamic Trade Flow overlay and `rebuild_routes`' three no-dead-city guarantees
    both did the latter until this rule was written.
36. **A plan may not close its scope by declaring work "deliberately not built."**
    That phrase used to end a section in most planning docs here, and it reads as
    settled when it is only unstarted — a future session finds a tidy refusal where
    it should find a task. Every item a plan does not do in its own slices is
    QUEUED instead: a numbered entry that names the SLICE OR MEASUREMENT it waits
    for and the gate it will need, so the queue can be picked up in order. "Out of
    scope for this plan" is a statement about sequencing, never about whether the
    work is owed. The two legitimate ways to close an item permanently are
    different and must say which they are: it is **already shipped** (name where),
    or it is a **measured negative result** (§2.4 — it was built, it was measured,
    it regressed a gate, and the table showing that is in the doc). A third
    category — "nobody will do this" — does not exist; if a thing genuinely should
    never be built, the reason is a measurement or a rule, and that is what gets
    written down.
