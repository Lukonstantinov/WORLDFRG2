# WorldForge 2 — Fantasy World Map Generator

Tauri 2 desktop app that procedurally generates a fantasy world (tectonics →
climate → hydrology → life → trade) and then runs a **living campaign economy**
on top of it (merchant houses, banks, coinage, wars, plagues, colonies).

**Stack:** Tauri 2 (Rust) · React 18 · PixiJS 8 · Zustand · SQLite (rusqlite) · zstd
**Layout:** `WorkflowPanel (left) | Map (center) | Toolbar (right) | StatusBar (bottom)`
**Two halves:** a **World** pipeline (`sim/step*/`, frozen on finalize) and a
**Campaign** simulation (`sim/campaign/tick/`, ~16.7k lines split by theme).

---

## 0. ACTIVE PRIORITY — Release-first plan (read before ANY work)

**As of 2026-10-15 the project is in a FEATURE FREEZE.** A `/court` trial of four
claims (12 juror agents each; verdicts in `docs/court/2026-10-15/`) found the engine
sound but the ORDER OF WORK wrong: nothing ships, no outsider has used it, and saved
files and peak memory are unproven. Full issue list and phased plan:
**`docs/IMPROVEMENT_PLAN.txt`** (PDF copy alongside it).

At the start of every session: read that file and tell the owner which phase is
current and which items are still open, before starting other work.

- **Frozen:** new Living World rows, people-layer features, new goods/institutions/panels.
- **Allowed:** bug fixes, performance, save safety, measurement, legal, release/packaging.
- **Order:** P1 save safety (old-save fixture test, sibling-file pairing) → P2 measure
  peak memory on a long 3600×1800 campaign → P3 decide one app vs two + one-line pitch →
  P4 LICENSE/EULA/dependency+data licence audit → P5 installer to 5-10 outside testers
  + priced waitlist → P6 let testers decide the campaign's depth.
- **Exception (owner, 2026-10-16):** world-GENERATION realism (steps 1-8 physics,
  ice, per-step UI, cross-fade/tracer/season animations) is UNFROZEN — plan and
  decisions in `docs/GENERATION_REALISM_PLAN.md`.
- **Exception (owner, 2026-10-17):** the GENERATOR REDESIGN (stage structure, layer
  cleanup) and the GOODS redesign (goods DNA, physical units, cultural valuations,
  reach, hidden deposits, Goods Builder) are UNFROZEN — analysis, decisions and open
  questions in `docs/GENERATOR_AND_GOODS_REDESIGN.md`.
- If the owner asks for a frozen feature, remind them of the freeze and of the next
  open item, then do what they decide.
- When an item lands: tick it in `docs/IMPROVEMENT_PLAN.txt`, then run
  `python scripts/plan_to_pdf.py`. When Phase 5 is done, rewrite this section.

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
# FULL-SIZE campaign: s/yr + peak memory on a 3600×1800 mock world, cached in target/
# (first run generates it; WF2_REGEN_WORLD=1 after worldgen changes). WF2_MEASURE_YEARS=N,
# WF2_PROFILE=1 adds the per-phase and per-call breakdown (SCOREBOARD 2026-10-16).
cargo test --release --lib measure_full_size_campaign -- --ignored --nocapture
cargo test --release --lib diag_what_grows -- --ignored --nocapture   # which state grows
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
  tail is, in order: `salinity` u8 · `shark_risk` u8 (retired, always 0) · `goods: Vec<Vec<u8>>`
  (`GOODS_COUNT`=45 belts) · `shipworm_risk` (retired, always 0) · `storm_base` · `reef_risk` ·
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
phases 1-6 + run-alls call `ensure_unfrozen`. **Only current-format saves load**
(owner decision 2026-10-15): both file kinds carry `SAVE_FORMAT` in their SQLite
`user_version`; `open_world`/`open_campaign` refuse anything else with a clear message
BEFORE touching the open world (gate: `commands::save_roundtrip_tests`). Bump
`SAVE_FORMAT` when the format changes incompatibly — there is no migration path. `import_world_layers`
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
| 8 | `sim_biological` | Storm + reef hazards + trade-good belts + ORE DEPOSITS (§8.16; `gem_deposits` now means ORE DISTRICTS) |
| 9 | `compute_political` | (query-only) Re-rank settlements by trade power + influence discs |
| 10 | `compute_economy` | (query-only) **Market equilibrium**: stock-based prices, barter, currency goods, wealth, chokepoints |
| All | `sim_run_all` | Phases 1-8 from plates |

**Stage order (2026-10-17, `docs/GENERATOR_AND_GOODS_REDESIGN.md` §8):** Goods (8) run
BEFORE Settlements (7), in both run-alls and the wizard: fever risk → culture map →
habitability (written) → goods → settlements → provinces. Settlement placement reads
nothing the goods phase writes, so this reorder changed no generated world; it is
what lets cities be founded where resources are. `sim_biological` prepares fever,
culture and habitability itself (`prepare_people_layer`, shared with
`sim_generate_settlements`), so Goods no longer needs Settlements. Step IDs are NOT
renumbered; `uiStore.PIPELINE` holds the run order for the staleness chip.
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

**2026-10-16 — a real full-size world is a different regime** (SCOREBOARD 2026-10-16).
`bench_campaign_tick_large` (synthetic, 8-10 s/yr of trade) badly understates a real
3600×1800 campaign: 40-60 s per simulated year, of which trade ~20 s and ARRIVALS
~12 s — mostly N1c relay hops (~4.6-7 M a year) plus the daily sweep of ~800k
in-flight cargoes. Measure with `measure_full_size_campaign` (cached mock world).
`WF2_PROFILE=1` now also prints `weekly/yearly/pre/post/arrive` per year and a
per-`campaign_advance` line (site recompute · advance · snapshot · autosave encode) —
cheap per-tick `Instant`s like the original four, not per-shipment counters.
`recent_trades` is a `VecDeque` ring buffer: as a `Vec` its `drain(0..1)` ran on every
shipment and relay hop (−15% of a year). FxHash on the trade-flow maps was measured
and REVERTED (no gain beyond noise: cache misses, not hashing).

**2026-10-16b — the carriage data, restructured (SCOREBOARD 2026-10-16b, bit-identical).**
Three daily costs scaled with the STOCK of cargo in flight (~800k) rather than the
day's flow; all three are gone, and the full-size harness's per-year `sim_fingerprint`
proves nothing moved. (1) `in_transit` is a `TransitBook` (`tick/transit.rs`): cargo
filed by `eta_tick`, landed with `take_due(tick)` (same order the old `retain` gave),
plus running per-house busy-vessel counters (`busy(oi)`: all owned / owned non-contract,
sea and land) in place of three full scans. It serialises as the same flat list, and
`for c in &sim.in_transit` still works — **push through `in_transit.push`, never
rebuild the list by hand, or the counters drift.** (2) `trade_cur` stores each flow
ONCE, keyed `(from, to, good, season)`, carriers as a short `Vec`; `fold_trade_year`
expands it into the inbound and outbound rows, which always held identical sums.
(3) `staged_stop(a, b)` caches `staging_hop` for the day (`stage_memo`) — everything it
reads is fixed within a tick. Result: 17-20 s per simulated year, flat through year 12.

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

### 5.11 `docs/living_world/06_IDEOLOGY_AND_SCHOLARS.md` — axes, named ideologies, scholars, schools (Living World row 06)

Four axes (Authority/Tradition/Openness/Economy, −5..+5), 16 named traits, 7
canonical named ideologies plus scholar-founded custom ones, per-city meters
(nobles/commons/government — the government one a real prestige-weighted
read of seated officials' own `Individual.ideology`), and scholars with a
real life-stage arc (spawn → study under a real teacher → teach/return/
patron → politics/exile), all in `sim/campaign/tick/ideology.rs`. Row 02
already reserved `ROLE_SCHOLAR`/`ROLE_PHILOSOPHER`/`ROLE_IDEOLOGUE` and
`Individual.ideology: [f32;4]` for exactly this row. Institutions (tutor →
school → library → academy → university) are NOT a new field — row 03's
own `TRACK_IDEOLOGICAL` building names already are this ladder; a founded
`School` is the scholar-level record on top of it. Meter drift includes a
real "a demand MET by a live edict entrenches its ideology faster than an
unmet one" mechanic. Two forward-looking hooks ship dosed at zero:
`IDEOLOGY_GOV_HOOK_DOSE` (row 04's `gov_position` would track this row's
live government meter instead of a one-time culture seed) and
`IDEOLOGY_UNREST_DOSE` (an unmet demand would raise unrest). UI is a
compact section in the Government panel, not the doc's own "Schools & Great
Minds" world window (queued, `06_IDEOLOGY_AND_SCHOLARS.md` Q06.3).

### 5.12 `docs/living_world/07_ARTISANS_AND_MASTERWORKS.md` — masterworks, guild culture ceilings, theft and looting (Living World row 07)

`sim/campaign/tick/masterworks.rs`: a real `Masterwork` record (title/kind/
maker/year/owner/material/location/condition/prestige/provenance), created
by a guild whose quality clears a threshold at ideological level ≥ 3 (now
capped by a SECOND, cultural ceiling — `cultural_quality_cap_e`, blended
into `run_craft_guilds`, dosed at `CULTURAL_QUALITY_CAP_DOSE`) or by a
notable `ROLE_ARTISAN`'s talent alone, no ceiling. `loot_masterworks(hub,
to_hub)` is real and CALLED — row 09's hordes sack cities with it this same
session, closing the design's own "row 09's sacks are the eventual caller"
note in one sitting. Theft is a real yearly house event resisted by a
city's own guild secrecy; invitations (commission/relocate/stay) use row
02's `decide()` 75% rule. `MASTERWORK_DEV_BONUS_DOSE`/`MASTERWORK_MARKET_
DOSE` (a bounded development bonus; real house purchases) ship at zero. UI
is a compact "Gallery" section in the Government panel, not a dedicated
City Gallery / House Dossier tab / world "Great Works" list (queued).

### 5.13 `docs/living_world/08_LEISURE_AND_GAMES.md` — leisure families, venues, games, decline (Living World row 08)

`sim/campaign/tick/venues.rs`: 14 universal leisure types, the doc's own
18-shipped-kit → 3-preference table plus a trait-weighted fallback (every
culture always resolves exactly three — no Southeast-Asian/Polynesian kit
was added this session, a worldgen change out of scope here, so that
family is reached only via the fallback, queued `08_LEISURE_AND_GAMES.md`
Q08.3). `Venue` records carry a real population-weighted popularity, hold
2-6 games a year plus festivals, and enter DISTRESS → DECLINE → ABANDONMENT
(never converted) within a bounded run of years. Performers spawn as real
`ROLE_PERFORMER` individuals; an Arena's fighters are captives only where
row 05's `bondage_permitted` allows it. `VENUE_COST_DOSE`/`VENUE_SPONSOR_
CONTROL_DOSE`/`GAMES_TRUCE_DOSE` (building/upkeep spend; a sponsor's
control over the aedile; the Olympic-truce war-chance reduction) all ship
at zero — a venue still founds, holds games and is abandoned regardless. UI
is a compact "Venues" section in the Government panel, not the doc's own
dedicated venue subpanel.

### 5.14 `docs/living_world/09_REALMS_WAR_AND_BARBARIANS.md` — barbarians and hordes (Living World row 09, Part D only)

**Partial by design, not by accident.** Of the row's five parts (A realm
benefits, B armies, C deeper provinces, D barbarians, E empires), only Part
D and one Part-A benefit shipped this session — armies and realm-vs-realm
war are flagged by the row's own doc as the highest-risk items in the whole
Living World plan (population and wealth moved at scale), and a rushed
version would have been worse than an honest partial row.
`sim/campaign/tick/hordes.rs`: a `Horde` (real famous `Individual` leader,
already-reserved `ROLE_HORDE_LEADER`) rises from whichever province scores
highest on the doc's own five triggers (warlike-and-over-capacity, high
discontent, a foreign settlement, and — where the signal exists — trade
imbalance/ethnogenesis), via ONE world-wide yearly roll calibrated toward
the doc's own decided "2-4 major waves a century" (never scaled by a
world's own steppe area). A horde raids (dosed wealth transfer), sacks
(calls `strip_holdings_at` + row 07's real `loot_masterworks`), razes
(`abandoned = true`, `died_cause` names it — the EXISTING `resettle_pass`
threshold picks it up unchanged), and always ends — settled, paid
tribute, defeated, or broken up on its leader's death — within
`HORDE_MAX_YEARS` (rule 22's discipline). One Part-A benefit also shipped:
`realm_openness_cohesion_bonus_e`, wired into `realms.rs`'s own `update_
realm_cohesion`, dosed at `REALM_OPENNESS_COHESION_DOSE = 0.0`. A compact
"🏇 Barbarian Tribes" panel lists every horde; the doc's own richer
conflict-map layer (smoke plumes, army/horde tokens, occupation hatching)
and the other six Part-A benefits, all of Parts B/C/E, are explicitly
QUEUED (`09_REALMS_WAR_AND_BARBARIANS.md`'s own Queue names each and what
it waits for) — not silently dropped.

### 5.15 `docs/living_world/10_SETTLEMENT_OVERVIEW.md` — the settlement Overview tab (Living World row 10, partial)

A new "Overview" tab in `HubPanel.tsx` (`ui/campaign/SettlementOverviewTab.tsx`),
now the first/default tab on selecting a city: government, ideology,
culture tiers, a venues/masterworks count, and the last 5 chronicle lines.
Built as a CLIENT-SIDE composition of rows 04/06/07/08's own already-
shipped read commands, deliberately NOT a new Rust aggregator — those
commands each lock the same `std::sync::Mutex` internally, so calling one
from inside a new command would deadlock. Development-track levels, city
identity/portrait, a war/realm/horde-threat block and an economy-at-a-
glance summary are queued (`10_SETTLEMENT_OVERVIEW.md`'s own Queue) rather
than folded in this session.

### 5.16 The 2026-09-30 Living World audit — what was dead, and what now lives

A measured audit of rows 02-10 (`econ_measure_living_world_census`, `#[ignore]`d,
prints a census every 20 years on `realm_reference_world`) found the layer mostly
built and mostly idle. Baseline over 100 years: **0 notables ever** (max fame 0.10 vs
the 0.65 threshold), **no city ever held an ideology** (the 3.0 "hold" radius was
smaller than any canonical doctrine's distance from a city), **0 schools**, **0 edicts
ever failed**, and **~68% of cities "Free Commune"** (every town under 15k was seeded
one, forever). Fixed:

- **Government forms** (`government.rs::govt_form_for`, `office_title_for`,
  `KIT_GOV_TITLES`): a form is NAMED in its culture's own kit and by size (a Norse
  assembly is an *Althing*, a Slavic one a *Veche*, a steppe ruler a *Khan*, a
  culture-less hamlet a "Village Assembly") — closes Q04.4, and is what fixes the
  VISIBLE "Free Commune everywhere". The mechanical redistribution
  (`govt_type_for`, a size × culture-ideal draw) is built but ships at
  **`GOV_FORM_DRAW_DOSE = 0.0` — a measured negative result**: at 1.0 it broke the
  multi-seed inheritance gate (seed 3) and the dense-world relay-volume ratio
  (0.37-0.55 vs 0.89), via form → seats/terms → capture → charters/regime
  changes/wars. `legacy_govt_type` still seeds; `migrate_government_forms` is a
  no-op at zero. Its dose walk is queued (04 doc); instrument
  `diag_forms_vs_relay_volume`.
- **Ideology does things** (`government.rs`/`ideology.rs`): an edict now has a
  DIRECTION (`GovEdict.dir`, +1 open/enact · −1 restrict/repeal — before, demands
  matched a FIXED per-family tag, so 10 of 14 canonical demands were unmeetable);
  the prevailing doctrine sets the agenda (`IDEOLOGY_AGENDA_DOSE` 0.5), seat holders
  vote their OWN ideology (`official_edict_lean`, `IDEOLOGY_VOTE_DOSE` 0.5), unmet
  demands feed unrest (`TickHub.ideology_unmet`, `IDEOLOGY_UNREST_DOSE` 0.5), the
  government meter drives `gov_position` (`IDEOLOGY_GOV_HOOK_DOSE` 0.3), revolution
  reads the commons-vs-government gap (`TickHub.ideology_gap`,
  `IDEOLOGY_REVOLUTION_DOSE` 0.5). A city holds the doctrine it LEANS toward
  (`dominant_ideology_for`, projection ≥ 1); people are seeded from their home meter
  + their traits (`trait_ideology_push`) + a personal spread; cities drift back toward
  their culture (`IDEOLOGY_CULTURE_ANCHOR`), without which one doctrine took 61/72.
- **Character progression** (`life_choices.rs`, new): 35% of life events are DILEMMAS
  resolved by `decide()` against the person's traits/modifiers; the choice gains,
  deepens or FLIPS a trait (`apply_trait_gain`, opposites in `opposite_trait`), moves
  fame, leaves a modifier and nudges ideology; its reasons are stored
  (`IndividualLifeEntry.why`). Career MILESTONES (`MS_*`, template ids 900-999) are KEY
  entries never pruned ahead of chatter (`push_life_entry`, rule 20 applied to a
  person). Yearly RENOWN (`people_renown_pass`) by role/office × city tier is what
  finally fills the notable roster.
- **Scholars** earn fame teaching and from students; founding a school makes a
  PHILOSOPHER, setting down a doctrine an IDEOLOGUE (with real demands); exile is a
  cause célèbre (once a decade); a renowned thinker may take an open seat
  (`SCHOLAR_SEAT_CHANCE`) and enter politics. Schools capped per city by learning track.
- **Travel** (`people_travel_pass`): only scholars and (untraced) artisans ever moved;
  now merchants, performers, diplomats, explorers, demagogues tour, visit and relocate
  (relocation is a `decide()` against their character), and go home. Every move/visit
  is a milestone; `places_of` serves the road (`PlaceBrief`).
- **UI**: `LifeStory.tsx` (key-moments filter, stages of life, choices with reasons,
  trait changes, the road), `CouncilChamber.tsx` (seats on a semicircle ringed by
  allegiance, office/path/trait icons, personal ideology bars, live vote lean, the
  motion with round pips, doctrine demands met/unmet), `traitIcons.tsx` (one icon
  vocabulary for traits/offices/paths/axes).

After (100 y, shipped config): 40 notables (29 philosophers, 33 scholars, 17
ideologues, 4 artisans, 3 officials — overlapping roles), 103 schools, 13 doctrines
holding cities (largest 41/72), 262 relocations, 410 journeys, 13k decisions, 512
character flips. Gates: `tick::tests` 405/405, `econ_` 6/6. **Still open**: hordes
never rise on this fixture (0 in 100 y — unmeasured why); one doctrine still wins
most of a culture-less world;
performers never reach the roster; tier-less towns' heads rarely do.

### 5.16b The Character window, and the audit's combinations (2026-09-30b)

**Every decision is recorded in full.** `IndividualLifeEntry` gained `odds`
(`decide()`'s probability per option, per mille), `pick` and `dk` (`DK_CHOICE` a
life dilemma · `DK_JOURNEY` settle-or-go-home at a journey's end · `DK_STUDY` go
away to study or stay · `DK_CAREER` a scholar's road · `DK_COMMISSION` an
artisan called by a greater city) — all serde-default, so an old log still loads
and simply shows "odds not recorded". `why` now holds reasons on BOTH sides:
`decision_reasons` (`life_choices.rs`, pure) keeps the top traits/modifiers that
pulled toward the option taken and flags with `REASON_AGAINST` the ones that
pulled the other way, plus situational `REASON_CONTEXT + CTX_*` codes (a rival
school, far from home, a patron nearby…). A dilemma's `args[2..=4]` store what it
actually did to the character (`TC_GAINED/DEEPENED/REPLACED/NO_ROOM/WEAKENED/
CONFIRMED`, planned by the pure `plan_trait_change`, which is also where the
trait soft cap now lives). Read side: `LifeEntryBrief.decision: DecisionBrief`
(the situation, every option with odds, trait/fame/modifier/ideology
consequences, reasons for and against), `PlaceBrief` gained coordinates, origin,
reason, km and stay odds, `IndividualBrief` gained trait origins, live modifiers,
teacher/students/house links and world size. Shown by `PersonWindow.tsx` (§7).
Gates: `decisions_are_recorded_in_full` (odds form a distribution, the stored
trait outcome is what actually happened), `decision_reasons_split_for_and_against`.

**The combinations the audit recommended, and what became of each:**
- **Scholar career as recorded decisions** (built): the study trip is now a
  `decide()` against the scholar's traits instead of a coin flip, the career
  review gains trait terms, and both are logged with odds even when the scholar
  stays (`MS_CAREER`, a KEY moment only when the road actually changed).
- **One movement system** (built): every change of city goes through
  `relocate_person` — scholars, artisans, travellers, homecomings — and every
  caller logs the milestone the road is read from. The artisan's call
  (stay / commission / move) gained trait terms and is recorded.
- **Performers tour real venues** (built): a performer's destinations weight
  cities with live venues, the visit records the venue it played (`MS_VISIT`
  `args[3]`, read as "to play the Great Arena"), and playing earns renown
  (`PERFORMANCE_FAME` × prestige) — people-only, touching no venue books.
- **One civic agenda** (Q05.3, built on the read side): `GovernmentBrief.
  culture_motions` lists every open culture-tier question beside the council's
  motion in the chamber. The two still debate on separate tracks in the sim (a
  single slot would starve one or the other — §5.10).
- **Beliefs drive culture acceptance** (built, live at `IDEOLOGY_ACCEPTANCE_DOSE`
  1.0): the government meter's Openness axis is a drift term ("open gates" /
  "blood and soil") and scales the persecution chance. Economically inert, since
  every acceptance effect is still dosed at zero.
- **Figure → Individual** (partly built): a figure carries `individual_id`, links
  lazily by name and role (never city — people travel now), and a linked
  figure's career and its person's life end together (`link_figures_to_people`).
  **Minting a person for every NEW figure is a measured negative result**
  (`FIGURES_BECOME_PEOPLE = false`): it shifted every later individual id, and so
  the rolls that seat officials, and the dense-world trade-volume ratio fell to
  0.20 against its 0.25 floor.
- **Guildmaster/alderman take council seats** (built, **measured negative result**,
  `GUILD_SEAT_CHANCE = 0.0`): bisected, it alone inverted the inheritance gate on
  seed 42. `try_seat_guildmaster` stays built and tested at a live chance.
- **The rows 05/07/08 prestige dose walk**: not started; it moves wealth and needs
  its own multi-seed walk (queued).

The lesson both negative results share: the government layer is now wired into
the economy (seats → votes → edicts), so anything that changes WHO SITS moves the
chaotic 60-year inheritance contrast. A people-layer change must be bisected
against that gate before it ships, not argued past it.

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
      cell.rs · routing.rs        cell_info + explain_cell (the "Why?" inspector: belt,
                                  rain shadow, offshore current, soil, fever — read-only);
                                  trade routes/matrix/trunks + compute_itinerary
      overlays.rs · political.rs  reef/storm/monsoon hazard zones + good/culture regions;
                                  overlays.rs also serves `get_deposit_sites` (goods G5:
                                  every working + whether towns KNOW it at year 0);
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
  mod.rs · tile_image.rs        ← 20 render layers (land, elevation, climate, … see §8.7);
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
  step8_biological_goods/       ← Ph8: biological.rs (storm/reef hazards + belts)
                                  · deposits.rs (THE ORE GEOLOGY — 11 deposit
                                    models scored from plate boundaries/volcanism,
                                    belt→district→working hierarchy, per-working
                                    grade/extent/depth — see §8.16)
                                  · localities.rs (THE AGRICULTURAL/BIOLOGICAL
                                    counterpart of deposits.rs — belt→locality
                                    hierarchy + full modulation for every
                                    Global/Local good, see §8.19)
                                  · goods_dna.rs (THE SIX GOODS PROPERTIES — role, origin,
                                    value tier, difficulty, keeping; hand table + heuristic;
                                    `GoodSpec.dna`, filled by backfill_market_fields)
                                  · goods_spec.rs (GoodSpec, 45 belts + ~21 manufactured;
                                    `marine_band` — Inshore/Bank/Either, §8.19;
                                    `origins` — independent homelands per good;
                                    `soil`/`relief` — the FINE-GRAIN terroir terms,
                                    §8.20; `Distribution::Endemic`, §8.20)
                                  · goods_validation.rs (test-only — Slice 0's coverage
                                    diagnostic, §8.19)
  shared/                       ← culture_values.rs (goods G7: each people's Prestige/Sacred/
                                    Staple/Taboo/Currency goods; derived or hand-set)
                                  · cultures.rs (organic peoples map + 14 traits + per-kit
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
                                  the ~40-template starter set; life_choices.rs =
                                  the 2026-09-30 audit's choice events, milestones,
                                  renown and travel (§5.16); culture_acceptance.rs =
                                  Living World row 05 (§5.10); ideology.rs = row 06,
                                  axes/named ideologies/scholars/schools (§5.11);
                                  masterworks.rs = row 07, masterworks + guild
                                  cultural caps + theft/looting (§5.12); venues.rs =
                                  row 08, leisure families + venues + games (§5.13);
                                  hordes.rs = row 09 Part D, barbarians/hordes (§5.14);
                                  transit.rs = the ARRIVAL CALENDAR (`TransitBook`:
                                  cargo in flight by arrival day + busy-vessel
                                  counters, §5.5); schism.rs = Phase 4.1,
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
                                  render layers into FIVE groups — Terrain · Ocean ·
                                  Atmosphere · Climate & Biomes · Settlement.
                                  Below the plates: MY LAYER SETS (`LayerSets.tsx`,
                                  the user's own saved views, localStorage).
                                  Overlays a step switches on are ON LOAN
                                  (`uiStore.borrowOverlay`/`borrowedOverlays`):
                                  they switch off when the step or mode changes;
                                  any user toggle keeps them
                                  A STYLE picker (§8.22) appears only while
                                  "elevation"/"terrain" is active, reading the served
                                  `elevation_styles` list — never a hard-coded one
  mapThemes.ts                  ← MAP PLATES (§8.17): 11 named compositions of base
                                  layer + overlays + label typography + line palette.
                                  `MANAGED_OVERLAYS` is DERIVED
                                  from the plates, so a plate can only ever clear an
                                  overlay some plate uses
  StatusBar.tsx · WindowBar.tsx ← Bottom status / window chrome. StatusBar carries the
                                  HOVER READOUT (§8.18): lat/lon, cell and the ACTIVE
                                  LAYER's own value with its unit, which is the map key
                                  for every layer that has no exact legend
  hoverReadout.ts               ← active layer + CellInfo → the readout string. Values
                                  come from `get_cell_info`, never re-derived from a
                                  pixel colour
  InfoPanel.tsx                 ← Right-click cell inspector + "Why is it like this?" (explain_cell)
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
  CouncilChamber.tsx            ← The council chamber (§5.16) — shared by HubPanel's
                                  Government tab and GovernmentPanel
  LifeStory.tsx                 ← A structured life story + the travel road (§5.16) —
                                  FiguresPanel, personShared, NotablesPanel, the chamber
  PersonWindow.tsx              ← THE CHARACTER WINDOW (§5.16b), app-wide via
                                  `uiStore.personWindow` (`PersonWindowHost` in App):
                                  Life (a biography written only from the record +
                                  the timeline) · Decisions (every decision in full —
                                  situation, each option's odds and consequences,
                                  reasons for and against, what it did to them) ·
                                  Travels (a map over the world thumbnail, the
                                  itinerary, "Show on the world map" →
                                  `uiStore.personRoad` → `OverlayManager.
                                  renderPersonRoad`) · Character (trait origins, the
                                  arc of their nature, beliefs pole by pole, live
                                  modifiers, teacher/students/house). `OpenLifeButton`
                                  is the one entry point every person view carries
  CouncilRoom.tsx               ← The council's ROOM, three ways (design handoff 7a-7c):
                                  pixel chamber · table from above · hemicycle by bloc,
                                  each authored on a fixed stage and scaled to the panel
                                  (overlay text hides below 0.6 scale). Pixel scenes in
                                  `canvas/councilArt.ts`; `CouncilChamber` hosts it with a
                                  room switcher (localStorage `wf.councilRoom`)
  PixelBust.tsx                 ← `PixelBust`/`PixelFigure`: canvas wrappers over a `Person`
  (cultureDress.ts, 2026-10)    ← now also a per-person FACE GENOME: `makePerson` (eyes, nose,
                                  brows, jaw, lips, hair, beard, age, skin by homeland
                                  latitude; weights per `PHENO_GROUP`), women's headwear
                                  101-107, `individualPerson(IndividualBrief)` (cached).
                                  Opt-in via `DressOpts.person`; omitted ⇒ bit-identical.
                                  `NotablesPanel` is now the roster (filters, away strip,
                                  cards with career ladder); `WorksCard` Production tab is a
                                  Sankey + tier ladder. NOT built: the design's sources/
                                  buyers/upgrades/master (not served by the sim), light theme
  traitIcons.tsx                ← ONE icon vocabulary: traits, offices, seat paths,
                                  ideology axes (`TraitChip`, `IdeologyBars`)
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
  YearChronicle.tsx             ← SHARED year-grouped expandable chronicle, set as a
                                  `<Ledger>` page (2026-10-16: "game frame, ledger inside" —
                                  `chronicleTheme.T` is the bronze frame palette in plain hex;
                                  `TV` the same tokens as CSS variables, which the `@ui/kit`
                                  primitives use so `<Ledger>` (paper, dark ink, red-ink
                                  headings) repaints them; `useFloatingWindow`'s rootStyle
                                  gives every floating window the bronze frame; the
                                  slate-blue chrome hand-coded across ~80 panels was turned to
                                  the bronze hue at the SAME lightness, water/sea lines kept blue)
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
  WorkflowPanel.tsx             ← Generation wizard: SIX STAGES + Report (`STAGES` —
                                  World & Land · Relief · Climate · Water & Life ·
                                  Resources · People · Report), each with Uses/Makes,
                                  part tabs and a re-run chip, over the old step ids.
                                  StepWorldCharacteristics renders split by `part`
                                  ("frame" in World & Land, "planet" in Climate)
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

The full science/systems reference — climate physics, trade goods & production chains,
market economy, ore geology, biomes, elevation styles, hydrology, map rendering,
inheritance law, and more — lives in **[`docs/REFERENCE.md`](docs/REFERENCE.md)**, moved
out of this always-loaded file to keep it lean. Read it on demand.

Sections there: 8.1 Overlays · 8.2 Climate physics & formulas · 8.3 Salinity ·
8.4 Trade goods & production chains · 8.5 Market economy (`market.rs::solve`) · 8.6 Köppen current overrides ·
8.7 Render layers & paint tools · 8.8 File operations · 8.9 Phase-3 performance shape ·
8.10 Province borders · 8.11 Map label typography · 8.12 Biomes · 8.13 Cordillera elevation ·
8.14 Settings preview · 8.15 Law of inheritance · 8.16 Ore deposits · 8.17 Map plates ·
8.18 Palette serving · 8.19 Goods localities · 8.20 Goods climate/terroir · 8.21 Natural colour ·
8.22 Elevation styles · 8.23 No valley carving · 8.23b Grid-scale relief budget ·
8.24 Physiographic provinces · 8.24a2–8.24d tectonics/rivers/lakes · 8.25 Stage-1 area tools.

## 9. Docs (`docs/`)

The catalogue of plan & design docs in `docs/` lives in
**[`docs/DOCS_INDEX.md`](docs/DOCS_INDEX.md)** — moved out to keep this file lean. Read the
background doc behind a feature when you touch it.

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
7. **New tile fields append LAST** — v2 self-describing blobs; trailing reads pad zeros. (Files from before `SAVE_FORMAT` are refused outright — §3.4 — but appending keeps saves within one format readable across builds.)
8. **Every `#[tauri::command]` is registered in `lib.rs`** and gets a wrapper in `bridge/` (via the @bridge barrel).
9. **New TS types mirror Rust serde structs** in `types/` (world/campaign/goods). Keep the SAME NAME as the Rust struct: `node scripts/check_types.mjs` (CI) fails when a TS type declares a field its same-named Rust struct never sends. A field the UI adds itself goes in that script's `CLIENT_ONLY` list.
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
14. *(Removed 2026-10-16 by the owner: where settings and controls live is no longer
    fixed. The number stays because code comments cite rules by number.)*
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
37. **A test that opens or generates a real world must not run in the shared test process.**
    `open_world`/`sim_run_all` set the PROCESS-GLOBAL active culture map
    (`cultures::ensure_active`), which every economy test reads when naming houses —
    so such a test running beside `econ_`/`tick::tests` changes their results and they
    flake (CI was red for three pushes on 2026-10-15/16 from exactly this). Mark it
    `#[ignore]` and give it its own CI step (`save_roundtrip_tests` is the pattern).
