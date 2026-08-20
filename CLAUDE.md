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
After ANY change touching `sim/campaign/tick/` (economy, houses, banks, coinage, war,
crashes, trade) you MUST run the living simulation and read the dynamics, not just
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

Measured baseline: **main-class 70.2%**, **exact-zone 39.0%** (was 66.2 / 29.0 at
`d53fdc9`; main-class was 70.1 before `TERRAIN_2_PLAN.md` slice 5's seafloor
structure — the one part of that plan touching `compute_sea_depth`/`generate_shelves`
— nudged it to 70.2, floor raised to 70.15). BOTH are now asserted — `EARTH_MAIN_FLOOR`
**and** `EARTH_EXACT_FLOOR`.
A third gate, `earth_monsoon_wind_reverses`, asserts the PHYSICS rather than the
score: monsoon winds must reverse between the two seasons and the mid-latitude
controls must not. It exists because the main-class floor was deliberately lowered
once (70.6 → 70.0) to adopt the seasonal monsoon, and a point spent on realism has
to be defended by something.
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
(Allen · Federico · Persson · De Vries · Alfani · Van Zanden). After ANY change to
`tick/` you MUST run:

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
| 7 | `sim_generate_settlements` | Habitability scoring → city placement |
| 7b | `sim_generate_provinces` | Cost-flood + feature-snap province partition (AFTER settlements) |
| 8 | `sim_biological` | Shark + shipworm risk + trade-good belts + ORE DEPOSITS (§8.16; `gem_deposits` now means ORE DISTRICTS) |
| 9 | `compute_political` | (query-only) Re-rank settlements by trade power + influence discs |
| 10 | `compute_economy` | (query-only) **Market equilibrium**: stock-based prices, barter, currency goods, wealth, chokepoints |
| All | `sim_run_all` | Phases 1-8 from plates |
| All | `sim_run_all_from_terrain` | Phases 2alt-8 keeping existing landmass |

**FOUR elevation MODELS, one selector.** `sim_commands::apply_elevation_model` is
the single place a mode string picks a generator — `plates` (the tectonic model,
`generate_elevation`, the ONLY one that reads `boundary_type`) · `shape` ·
`cordillera` (§8.13) · `ridged`. Both run-alls used to HARDCODE a generator and
silently discard the user's pick and all four sliders, so "Generate Full World"
produced the same relief however `StepElevation`'s picker was set; the models were
reachable only from step 2's own button. Two rules: the tectonic model is offered
only where plate data exists (`landmassSource === "plates"`) and degrades to the
shape model otherwise, and an UNRECOGNISED mode must still build terrain — a bad
string may never leave a world with no elevation. Gated by
`elevation_model_tests`, which asserts the four disagree on >25% of land (a
picker that does not reach the generator makes them identical).

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
  deliberately NOT built: a great partnership needs alliance-linked tier rises, a legendary
  head needs goals (Phase 3, unbuilt) — both deferred, not built silently short.
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
  `trigger_regional_crash` contagion via `fail_bank` & `maybe_pop_bubbles`.
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
     clearance/drainage/irrigation/road, funded yearly, **stalling** when unpaid),
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
  estate-KIND-aware rate (`dominant_estate_kind`): a mine barely recovers ("exhausts"), a
  fishery recovers fast ("collapses and recovers"), a vineyard doesn't accrue depletion at
  all (doesn't lose tonnage — the "raises grade instead" half is not tracked), a plantation
  also nudges `prov_soil` down under pressure ("wears soil"). A manufactory is excluded
  structurally, not by a special case — `Manufactured` goods have no belt score to begin
  with. Exposed via `campaign_province_goods`; the Province Inspector's Land tab shows it
  in place of the frozen quality/rank list the moment a campaign is actually producing
  something. Because it only WRITES `prov_good_depletion` (never touches hub production,
  stock or price), it cannot move the `econ_` bands or the dynamics test by construction —
  verified, not just argued: both are bit-identical/unchanged with this pass wired in.

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
- **Growth is exogenous.** `tech_factor *= 1.015^(1/365)` per tick is the entire
  technology + growth model. There are no capital goods, no fuel inputs and no labour
  market, so nothing in the economy can influence its own growth rate (Part C of the
  fix plan). Don't mistake the finance layer for a growth engine — it redistributes.
- **`Pop` is inert.** `hubs[h].pops` is written yearly in `cities.rs` and read ONLY by
  `campaign_get_pops` for display; `militancy`/`consciousness` are computed and
  discarded. The live social model is the abstract `Society` shares (item B3).

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
                                  Departure schism / independent founding). Four
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
  step2_terrain/elevation.rs    ← Ph2: plate-based + template-based elevation.
                                  `stream_power_erosion` (priority-flood + flow
                                  accumulation + K·A^m·S^n incision) replaced the old
                                  droplet simulation (Terrain 2.0 slice 1); outer-pass
                                  count is keyed to GRID SIZE, not the `iterations`
                                  strength knob (see `terrain_metrics`, §3 below)
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
                                  permanent record — see §5); schism.rs = Phase 4.1,
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
  TileViewport.ts               ← Pan/zoom, screenToWorld, getVisibleTileRange
  TileManager.ts                ← LRU tile cache, base64→texture, sprite management
  OverlayManager.ts             ← ALL vector overlays, drawn in CANVAS 2D — not Pixi
                                  (~4.6k lines: rivers, settlements, wind,
                                  trunks, routes, dynamic flow, regions). visibility[type] gates
                                  each. Also holds the two live appearance registries the
                                  Settings store drives: `lineColors` (overlay lines) and
                                  `labelStyles` (map label typography — see §8.11).
                                  GOODS_LOCALITIES_PLAN.md Slice 5 · a trade-good belt's
                                  FILL now comes from a FULL-RESOLUTION mask
                                  (`drawGoodBeltMasks`/`buildGoodMaskRender`), not the
                                  coarse ~8-cell blocks `GoodRegion` carries — see §8.19.
                                  `GoodRegion` still supplies each belt's LABEL, and its
                                  old coarse fill remains the fallback for a good whose
                                  mask hasn't arrived
  PaintOverlay.ts               ← Brush preview, paint stamps
  projection.ts                 ← lat/lon ↔ world-cell projection helpers
  goodIcons.ts                  ← good → emoji/texture for overlays

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
  HydrologyPanel.tsx            ← Rivers/lakes + aquatic (fish assemblage, limnology)
  ProvincePanel.tsx             ← 🗺 Provinces BROWSER (sort/filter/compare + generate)
  ProvinceInspector.tsx         ← 🏞 Dossier for ONE province, opened by CLICKING the map.
                                  FOUR TABS (Land · People · Holdings · Chronicle) over
                                  the layered survey plate, plus a YEAR SLIDER that
                                  scrubs `ProvinceLand.history` — a plate that differs
                                  between year 1 and year 500 is the visible proof the
                                  two halves are one simulation. Holdings carries the
                                  CONTROL verbs (dues slider, begin/abandon a work),
                                  read-only on a province no town administers. Phase 5 ·
                                  the "writ of {holder_name}" line, granary note and
                                  works-funding note all read correctly whether a CITY
                                  or a HOUSE holds the province's writ
                                  (`ProvinceLand.holder_house`).
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
                                  (GOODS_LOCALITIES_PLAN.md Slice 6): a `GoodLocality`
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
                                  `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` §2). The
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
  CityView.tsx · SettlementScene.tsx ← Isometric city view + scene
  HousesPanel/DynastiesPanel/GuildsPanel.tsx ← Merchant houses, dynasties, guilds.
                                  HousesPanel has a world ⚔ Feuds tab; the list is
                                  GROUPED BY TIER (Phase 1.1, Tier 3/4 collapsed by
                                  default). A ⚖ Compare button opens `HouseCompareWindow`
                                  (`HouseCompare.tsx`) — a search-bar-driven two-house
                                  side-by-side: ruler figures, every stat (standing ·
                                  trade/transport · trading strategy · monopolies),
                                  and a minimal `OperationsMap` plotting both houses'
                                  seat/offices/controlled settlements so a rivalry's
                                  footprint reads at a glance. Pure frontend aggregation
                                  of `HouseBrief` fields already fetched — no new backend
                                  command. Its per-house detail (`HouseDetail`) opens as
                                  a BIG FLOATING WINDOW (~2.5x the old size, still
                                  draggable) on a portrait — `cultureFigureSVG` in the
                                  seat culture's kit and the head's own sex, now with
                                  POSE (tilt/mirror) and ACCESSORY (pin) variation axes
                                  on top of the existing build/skin-tone jitter, so two
                                  heads read as two different people at a glance — a
                                  coloured frame standing in for a garment recolour, a
                                  `CoatOfArms` badge at the shoulder, occasion set by
                                  tier (Phase 1.2) — and its subtabs are
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
                                  ⚖ Standing/⚔ Feuds/🏦 Bank/📒 Accountant
  HouseDossier.tsx              ← The House Dossier's two views: `HouseStandingView`
                                  (five stability gauges — solvency COUNTDOWN, liquidity
                                  runway, concentration exposure, succession, cohesion —
                                  plus liabilities) and `FeudsView` (cause · temperature
                                  · stage · ending, with each feud's episode log).
                                  Pips + a PHRASE, never a raw 0..1; a healthy gauge
                                  stays quiet so the warning colour still means something
  BankPanel/MoneyFinancePanel.tsx ← Bank T-accounts, currencies/mints/monetary chronicle
  SpeculationPanel.tsx          ← DLC 3: Speculation why-chain / Poleis (treasury/tariff/mint/coin)
  CoinCreditPanel.tsx           ← Currencies / Banks / Wars / Crashes / Schematics tabs
  WarehousesPanel.tsx           ← House warehouses / reserves
  FuturesPanel/FuturesLanePanel.tsx ← Futures contracts + lanes
  EconomyDashboardPanel.tsx     ← #30/#29 Price Index (CPI) + Inequality (Gini/mobility) tabs
  CityRankingPanel.tsx          ← Richest/busiest cities
  FlowsView.tsx                 ← Realized trade at a settlement (post-campaign)
  ColonialPanel.tsx             ← Colonies / colony gates / lifelines
  ImmigrationPanel.tsx          ← Route-bound migration corridors
  SatelliteConstructionPanel.tsx← Satellite (suburb) construction projects
  PlaguePanel.tsx               ← Historical epidemics
  PeoplesPanel.tsx              ← Cultures / peoples / pops
  FiguresPanel/CultureFigures/CultureDonut.tsx ← Key figures/notables + culture charts
  LandmarksPanel.tsx            ← Notable landmarks
  AtlasPanel.tsx                ← Atlas 2.0 (eras / world frame)
  NewsFeedPanel.tsx             ← Campaign news feed
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
  cultureFigure.ts · chronicleTheme.ts · settlementArt.ts ← helpers/themes/art

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
- **UNLIMITED** (stockfish, furs, timber, salt, whaling, wheat, iron) — every
  suitable cell produces.
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
ONE world four times, changing only the law, and asserts the rule is wired (partible
divides, the rest do not) and that it MATTERS — **more houses ever founded** (now by a
≥1.05× margin rather than a bare `>`, which on a near-tie is a coin flip dressed as a
gate: crisis relief once flipped it at 190 against 196) **and lower mean wealth per
house**. Note what it does *not* claim: the top share and Gini do not fall under
partible, because a division adds small firms at the bottom as fast as it trims the top.

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

| contrast | @0.60 (broken) | @0.30 (shipped) |
|---|---|---|
| more houses ever founded | 6/6 | 5/6 |
| more houses still standing | 4/6 | 2/6 |
| lower top share | 2/6 | 3/6 |
| **lower mean wealth per house** | **1/6** | **5/6** |
| no MORE capital in total | 1/6 | 5/6 |

The claim is real; the dose genuinely broke it. **A seed sweep only tells you about the
world you ran it in** — measuring robustness inside an already-distorted economy produced
a thorough, plausible, false conclusion ("the merchant pool is not conserved; firm count
is a multiplier on merchant wealth"), which is an artefact of the 0.60 dose and not a
property of the model. Before concluding an assertion is unsound, check that the world you
measured in is not itself the thing that is broken.

One residual caveat, flagged rather than resolved: 0.30 was chosen because it restores
this gate, which is weak grounds for a demand parameter. The dose-dependence above shows
the gate is measuring something real rather than noise, so the choice is defensible — but
`COMFORT_IMPORT_FRAC` still has no independent justification of its own.

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

> **Still not built:** nothing reads `depth` yet. A mine is still a substring
> match on the good's name (`tick/mod.rs`) and is mechanically identical to a
> farm — no mining capability, no depth gating, no mine-vs-quarry split. See
> `docs/DEPOSITS_AND_MINING_PLAN.md` slice 4.

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

### 8.19 Goods localities — the agricultural/biological hierarchy (`docs/GOODS_LOCALITIES_PLAN.md`)

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
Terrain 2.0's own slices 3-4 — see `TERRAIN_2_PLAN.md` in §9 — fixed and gated
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

## 9. Docs (`docs/`)

**START HERE — the current plan**
```
FIX_PLAN.md                       ← ⭐ Measured Earth-fidelity baseline + the prioritised
                                    fix plan (climate · one-simulator · economy · society),
                                    with a regression gate per item. Read before planning work.
SCOREBOARD.md                     ← ⭐ The project held as ~12 NUMBERS instead of 89k lines:
                                    both fidelity scorecards, test counts, perf, and an
                                    explicit list of what is still UNMEASURED. See §2.6.
```

**Live operational docs** (these describe the project as it is)
```
PROVINCE_SYSTEM_PLAN.md           ← The province layer's design + status (see FIX_PLAN B1);
                                    the shipped algorithm itself is §8.10 above
DEPOSITS_AND_MINING_PLAN.md       ← ⭐ APPROVED, SLICES 1-3 BUILT. Ore geology
                                    (§8.16) + grade→quality rewire + txt goods
                                    import & 8 new minerals — all done, all gated
                                    → mining as an industry (depth gating, mine vs
                                    quarry) → quarry window, mining settlements,
                                    growing settlement catchment (slices 4-5,
                                    unbuilt). Carries the
                                    measured findings that motivated it and its own
                                    "deliberately not built" list
TERRAIN_2_PLAN.md                 ← ⭐ ALL SIX SLICES BUILT. `hydraulic_erosion`
                                    (droplets, ~1.4 visits/land cell measured) is
                                    replaced by `stream_power_erosion` — priority-
                                    flood fill + flow accumulation + `K·A^m·S^n`
                                    incision (`step2_terrain/elevation.rs`), touching
                                    every land cell's real flow path every outer
                                    pass instead of sampling a fraction of them.
                                    New transient module `step2_terrain/geology.rs`
                                    (§2's "transient first" — recomputed from seed +
                                    persisted plate data every phase-2 run, zero tile-
                                    format change) supplies: LITHOLOGY (independent
                                    noise bands so resistant rock holds ridges),
                                    OROGENY setting+age (real, from `density`'s
                                    oceanic/continental split reconstructed via
                                    majority-vote terrain per plate — active-margin
                                    arc-offset-from-trench / collision / island-arc,
                                    inherited outward through a belt by the same BFS
                                    that measures distance from it, so an old worn
                                    range sits beside a young sharp one), a phase-2
                                    CLIMATE PROXY (latitude + continentality — an
                                    explicit stand-in for phase-3 precipitation,
                                    documented to disagree with it), and REGIONALISED
                                    hypsometric redistribution (D9 — a region's own
                                    pre-redistribution character survives the global
                                    rank-squeeze instead of being erased by it).
                                    `plates.rs`'s boundary classification is fixed
                                    (D3): the normal is now the true Voronoi-bisector
                                    direction between two plates' seed points, not
                                    one plate's centre to the cell, and a triple
                                    junction classifies by strongest signal across
                                    every differing neighbour instead of scan order.
                                    Coastlines are DECOUPLED from the Voronoi edge
                                    (D1/T1) by a LEVEL SET, the third pass at this
                                    (see `docs/SCOREBOARD.md`'s 2026-08-19e entry):
                                    a signed distance-to-nearest-boundary field
                                    (BFS; positive on the land side, negative on
                                    the sea side) perturbed by noise and re-
                                    thresholded at zero, so only cells within
                                    `reach` (~0.55×plate-size) of a real boundary
                                    can ever flip — no far-flung speckle islands by
                                    construction. The first two passes each
                                    measured a real number (90%, then 62.5% coast-
                                    on-boundary) that still looked, in
                                    `dump_natural_sheet`, like an unmodified
                                    Voronoi edge or a scatter of dots bolted onto
                                    one — a plain 2-D noise threshold has no notion
                                    of "near the true 1-D boundary curve", so
                                    wherever it flipped a cell, the flip was
                                    uncorrelated with where the coastline actually
                                    needed to bend. The level set fixes this by
                                    construction rather than by amplitude tuning:
                                    measured coast-on-plate-boundary 6-7% (both the
                                    `terrain_metrics` config and the exact
                                    `dump_natural_sheet` world a maintainer
                                    screenshot came from), gated permanently by
                                    `coastline_departs_from_the_plate_boundary`. A
                                    despeckle pass (component flood-fill, now
                                    `DESPECKLE_MIN`=14 cells) is a safety net, not
                                    the mechanism. The SAME screenshot also showed
                                    straight diagonal "scar" lines across
                                    continental interiors — a SEPARATE bug, the
                                    divergent-boundary rift pulldown reading
                                    `boundary_type[idx]` at the cell's own unwarped
                                    position; fixed the same way as the D4 orogeny
                                    belt (read at the warped position, fade
                                    smoothly with distance). A percentile threshold
                                    (not a fixed cutoff) still holds the total land
                                    fraction exactly what the plate mix implied
                                    throughout. Seafloor gets ridges/trenches/abyssal
                                    hills/scattered seamounts in `generate_shelves`
                                    (measured sea_depth↔distance-to-coast r ≈
                                    0.66-0.74, down from ~1.0 — a real decorrelation,
                                    §2's ONE slice that touches the Earth gate, which
                                    HELD: 70.1%→70.2% main-class, floor raised to
                                    70.15). Render follow-up: a bounded, honest
                                    approximation of texture shading (widens the
                                    existing direction-independent AO term
                                    specifically on the lee/shadowed side, never a
                                    second light — a true multi-scale fractional-
                                    Laplacian transform needs a wider cross-tile halo
                                    this renderer doesn't carry) plus the elevation
                                    ramp's white blow-out softened. `terrain_metrics`
                                    (§3's harness) prints RMS slope / slope spread /
                                    drainage density / hypsometric integral / coast-
                                    on-boundary / sea_depth-correlation per model —
                                    the FIRST measurement of these, so it establishes
                                    the baseline rather than clearing a pre-set floor.
                                    NEGATIVE RESULT kept as a record, not chased
                                    further: stream-power's outer-pass count had to
                                    be decoupled from the old `iterations` (droplet-
                                    budget) knob and keyed to GRID SIZE instead —
                                    keying it to `iterations` gave a SMALL world
                                    fewer passes than a large one, which is backwards
                                    for performance AND broke
                                    `cordillera_crest_runs_parallel_to_the_coast`
                                    (a real world's stream-power run stays at 4-6
                                    outer passes for the plan's own perf budget, but
                                    every unit-test-sized fixture gets 8 for free).
                                    Even so, phase-2 cost real time — `bench_phase2`
                                    @ 3600×1800: plates 8.5s→11.4s, shape
                                    11.4s→13.9s (rayon-parallelised where a pass has
                                    no cross-cell dependency: the Voronoi assignment,
                                    boundary classification and lithology/climate-
                                    proxy maps all moved to `into_par_iter`, but the
                                    priority-flood queue itself is inherently
                                    sequential and stayed the dominant cost) — short
                                    of the "no slower" target, recorded rather than
                                    hidden. A downstream consequence also recorded
                                    rather than hidden: the fixed-seed 300×150 goods-
                                    coverage reference world now places `pearls`'
                                    inshore homeland outside every settlement's
                                    catchment at that seed/scale — real generation
                                    regenerates settlements FROM the decoupled
                                    coastline, so this is a fixed-fixture sampling
                                    artefact (a second, honestly-labelled exception
                                    in `goods_validation.rs`, not folded into the
                                    pre-existing `dyes` one). Slice 4's blast radius is real but scoped
                                    exactly as flagged: only NEW generation changes;
                                    a saved world's stored tiles are untouched
CITY_PROVINCE_WAR_PLAN.md         ← ⭐ APPROVED, NOT YET BUILT. The next three workstreams:
                                    the settlement panel rework · provinces (enclave fix,
                                    sizing, real-terrain view, goods & exploitation) ·
                                    the political layer (city leader as a house office,
                                    city tiers, the city-as-state, and war). Carries its
                                    own caveat list (§5) — incl. that it REVERSES
                                    PROVINCE_SYSTEM_PLAN's "enclaves survive" decision —
                                    and its own "deliberately not built" list (§6)
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
                                    and its own "deliberately not built" list. Four
                                    design decisions are recorded at the top
                                    (individual vessels · houses+guilds only ·
                                    privilege staged as terms-then-price · no code yet)
GOODS_LOCALITIES_PLAN.md          ← ⭐ ALL 8 SLICES BUILT. Trade goods got what
                                    minerals already had (§8.16): belt → LOCALITY →
                                    cell, persisted to `metadata["good_localities"]`
                                    like `deposits`, with a size ladder in km (a
                                    staple region is 900 km — the chernozem case —
                                    against a 45 km ore district). Fixed the three
                                    things the measured findings turned up: placement
                                    now reads rivers (floodplain/irrigation/riverbank/
                                    float_out, §8.19), marine goods split into
                                    Inshore/Bank bands, and the overlay draws a
                                    FULL-RESOLUTION mask instead of coarse 8-cell
                                    blocks that spilled past the coast. Two layers per
                                    good — coverage ("can it grow here") and quality
                                    ("is it fine here") — off one u8 column, on one
                                    absolute ramp shared by every good (§8.19). Slice 0
                                    is the COVERAGE DIAGNOSTIC every later slice is
                                    measured against; Slice 7 (economy wiring) went
                                    last, gated on `econ_`/dynamics. See §8.19 for the
                                    full account, including its own risks (§5 of the
                                    plan — full modulation vs "goods must keep
                                    reaching settlements") and "deliberately not
                                    built" list (§6)
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
                                    contiguous single-culture bloc of ≥4 provinces
                                    unifies under its largest city, over
                                    `prov_culture` + `prov_neighbors`).
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
                                    and "deliberately not built" list (§6); §7's
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
12. **After any `tick/` change** → run the dynamics test (§2.1). **After any
    `step3_ocean_atmo/` or `step4_climate/` change** → run the Earth fidelity gate (§2.3)
    and re-read §8.9 (no per-cell outward scans; keep the row loops parallel).
    **After any verified change** → push to `main` (§2.2), and keep this file true (§2.7).
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
