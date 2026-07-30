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

Measured baseline (commit `d53fdc9`): **main-class 66.2%**, **exact-zone 29.0%**.
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
| 4 | `sim_classify_climate` | Köppen classification (31 zone codes + H highland) |
| 5 | `sim_rivers_hydrology` | Priority-flood (Barnes et al. + ε) → rivers → lakes → aquatic ecology |
| 6 | `sim_soil_fertility` | Soil types (12) → fertility → fisheries |
| 6b | `sim_classify_biomes` | **41 ecological biomes** (needs rivers+lakes) — see §8.12 |
| 7 | `sim_generate_settlements` | Habitability scoring → city placement |
| 7b | `sim_generate_provinces` | Cost-flood + feature-snap province partition (AFTER settlements) |
| 8 | `sim_biological` | Shark + shipworm risk + trade-good belts (takes seed + gem_deposits) |
| 9 | `compute_political` | (query-only) Re-rank settlements by trade power + influence discs |
| 10 | `compute_economy` | (query-only) **Market equilibrium**: stock-based prices, barter, currency goods, wealth, chokepoints |
| All | `sim_run_all` | Phases 1-8 from plates |
| All | `sim_run_all_from_terrain` | Phases 2alt-8 keeping existing landmass |

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
  (`sim::tick`) that names only the notable axes — nothing in the tick reads
  `Kin.character` yet, wiring an axis to the decision it names is Phase 2.4,
  deliberately deferred (see below). `kin_power_shares` (Phase 2.6) turns role × skill
  × loyalty into a 0..100 share per kin that always sums to exactly 100
  (`power_shares_always_sum_to_100`) — pure display, nothing else. **The widow as a
  capable merchant**: a purely `Agnatic` line otherwise never produces a female head
  (`heir_is_female` always returns false for it), so `succeed_house` rolls an
  independent `WIDOW_REGENCY_CHANCE`=8% chance of a widow regent instead — the roster
  doesn't yet track marriages, so this can't be conditioned on "is there actually a
  widow". `HousesPanel`'s Summary tab tags a family-run holding with its posted kin's
  name (silent = hired, the same "quiet unless it matters" rule as everywhere else
  here); the dossier's 👪 Kin tab lists the full roster. **Phase 2.4 (character wired
  to real decisions) and 2.5 (stewards with skim/wage mechanics) are NOT built** —
  both would move house wealth directly and need `econ_` verification per knob as
  they're built, not a single check at the end.
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
  ships), route-bound migration corridors, expeditions.
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
      economy.rs · flow.rs        political ranking; market economy; dynamic trade flow
  campaign_commands/            ← finalize/unfreeze, campaign lifecycle + ALL campaign read
                                  queries. Split into a folder (mod.rs re-exports children;
                                  campaign_commands::* paths unchanged):
      lifecycle.rs                finalize/new/save/open/progress/persist/start/advance/state
      read_hubs.rs · read_money.rs  hubs/journal/houses; coin/bank/crash/war/inequality/poleis
      read_people.rs · read_colonies.rs  cultures/pops/figures/dynasties; colonies/migration
      read_trade.rs               goods/routes/futures/warehouses/guilds/schematics/diagnostics
      read_houses.rs              House Dossier reads: the five STABILITY gauges
                                  (campaign_house_stability) + the FEUD board
                                  (campaign_get_feuds) + the KIN roster
                                  (campaign_get_house_kin, Phase 2.1). Four of five
                                  gauges are pure derivations of state the sim already
                                  held; kin_power_shares/character_phrase (Phase 2.6/
                                  2.3) live in sim::tick so they're gated by tests, not
                                  just called from here.
      province.rs                 province LAND state (campaign_province_land[_all]) +
                                  the CONTROL VERBS — the only mutating campaign
                                  commands besides campaign_advance (§5.1)
  goods_commands.rs             ← Goods spec CRUD, default_custom_goods, backfill
  import_commands.rs            ← import_world_layers (layered world import)
  preview_commands.rs           ← preview_zonal_profile / preview_coarse_climate (§8.14)
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
  mod.rs · tile_image.rs        ← 25 render layers (land, elevation, climate, … see §8.7);
                                  the biomes layer carries PROCEDURAL PATTERN FILLS (§8.12)

paint/
  mod.rs · stroke.rs            ← PaintValue enum (terrain/elevation/shelf/volcanic)
  brush.rs                      ← circle_brush with cylindrical wrapping

import/mod.rs                   ← TileData::merge_columns (layer-group import)
history/mod.rs · undo.rs        ← Tile-level undo/redo journal

sim/                            ← organised into per-phase step folders; mod.rs re-exports
                                  each leaf module so paths stay sim::plates, sim::tick, …
  mod.rs                        ← Sim module declarations + `pub use` re-exports
  world_buffer.rs               ← WorldBuffer: flat arrays + per-phase ColumnSet masks
  step1_plates/plates.rs        ← Ph1: Voronoi plate tectonics
  step2_terrain/elevation.rs    ← Ph2: plate-based + template-based elevation
  step3_ocean_atmo/             ← Ph3 (the physics core — see §8.2):
      ocean.rs                    winds · Sverdrup gyres · currents · salinity · thermohaline · SST
      insolation.rs               astronomical daily-mean insolation (ANY obliquity)
      ebm.rs                      1-D diffusive North–Budyko energy balance + ice-albedo
      circulation.rs              Hadley edge / polar front derived from ROTATION rate
      temperature.rs              base curve + EBM anomaly + lapse + currents + coastal damping
      jets.rs · seasonal.rs       low-level jets (Somali) · two-season winds & monsoon
      precipitation.rs            advection-decay moisture + ITCZ/orographic/frontal/jet terms
      preview.rs                  SETTINGS PREVIEW (§8.14): 1-D zonal profile + coarse
                                  climate map — read-only, never touches a tile
      bench.rs                    (test-only) phase-3 PERF harness + field checksums — see §8.9
  step4_climate/                ← Ph4: koppen.rs (31 zone codes + H) ·
      earth_validation.rs         THE EARTH FIDELITY GATE (§2.3) + fixtures/
  step5_rivers/                 ← Ph5: rivers.rs (priority-flood/rivers/lakes) · aquatic.rs
                                  (freshwater ecology: fish assemblage, lake limnology)
  step6_soil_fertility/         ← Ph6: soil.rs (12 soil types) · fertility.rs (fisheries)
                                  · biome.rs (Ph6b: 41 ECOLOGICAL BIOMES — see §8.12)
  step7_settlements/settlements.rs ← Ph7: habitability → city placement (Settlement struct)
  step8_biological_goods/       ← Ph8: biological.rs (shark/shipworm + belts + deposits)
                                  · goods_spec.rs (GoodSpec, 45 belts + ~21 manufactured)
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
      tick/                       THE CAMPAIGN TICK SIM (~16.7k lines, by theme). See §5.
                                  mod.rs = structs/consts/free-fns/advance()/impl Bank/…/
                                  residual impl CampaignSim; methods grouped into money/war/
                                  disease/colonies/polis/cities/houses/production child impls
                                  (pub(crate), `use super::*`); tests.rs = the dynamics tests
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
                                  `labelStyles` (map label typography — see §8.11)
  PaintOverlay.ts               ← Brush preview, paint stamps
  projection.ts                 ← lat/lon ↔ world-cell projection helpers
  goodIcons.ts                  ← good → emoji/texture for overlays

ui/SettingsPanel.tsx            ← ⚙ Appearance modal, two tabs: Overlay lines (the line
                                  palette) and Map labels (typography theme + per-class
                                  face/colour, each row set in its OWN style so the list
                                  doubles as a live specimen sheet). See §8.11.

ui/world/  — map & world
  MapCanvas.tsx                 ← PixiJS canvas, pointer events, painting, draws every overlay
  Toolbar.tsx                   ← Tools, layer selector, overlay toggles (RIGHT side)
  StatusBar.tsx · WindowBar.tsx ← Bottom status / window chrome
  InfoPanel.tsx                 ← Right-click cell inspector
  ElevationLegend/Histogram.tsx ← Elevation legend + distribution chart
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
                                  read-only on a province no town administers.
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
                                  invent a spatial layout the model does not hold
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
                                  Transit, year-grouped Chronicle). ~2k lines
  CityView.tsx · SettlementScene.tsx ← Isometric city view + scene
  HousesPanel/DynastiesPanel/GuildsPanel.tsx ← Merchant houses, dynasties, guilds.
                                  HousesPanel has a world ⚔ Feuds tab; the list is
                                  GROUPED BY TIER (Phase 1.1, Tier 3/4 collapsed by
                                  default). Its per-house detail (`HouseDetail`) opens
                                  on a portrait — `cultureFigureSVG` in the seat
                                  culture's kit and the head's own sex, a coloured
                                  frame standing in for a garment recolour, a
                                  `CoatOfArms` badge at the shoulder, occasion set by
                                  tier (Phase 1.2) — and its subtabs are
                                  CHRONICLE-FIRST (Phase 1.4, the default tab):
                                  the Phase 0.4 succession line inline, then the
                                  year-grouped event log (`ChronicleTab`), before
                                  Summary (now tags family-run holdings, Phase 2.2)/
                                  👪 Kin (the roster, Phase 2.1/2.3/2.6)/
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
  ChroniclePanel.tsx            ← World ledger — reading matter (left rail)
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
- `compute_good_regions` → per-good belt cell-masks (fill + outline + emoji; gems
  carry a `sublabel` naming the stone; one toggle per good under Trade Goods).
- `compute_trade_routes(settlements, rivers, reach, max_crossing)` → least-cost routes
  over the shared coarse cost grid (passes / rivers / coast-hugging); reach limits
  open-water crossings.
- `compute_trade_matrix(...)` → settlement-cluster regions, per-good prod/demand/net +
  per-good `flows` + **routed & bundled `trunks`** (edge width ∝ volume). Sea-
  impassable pairs (under the reach) get no flow.
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

**Ocean currents** are a gyre-aware relaxation, not a solve: the *interior* comes from
the Sverdrup relation (curl of belt wind stress on a β-plane — sign and latitude
structure EMERGE), while boundary speeds are prescribed constants
(`SPEED_BOUNDARY_WEST` 2.2 vs `..._EAST` 0.55 = western intensification), then 20
deflection passes + bathymetry steering. The field is **not divergence-free** and
currents are **annual-mean only** even though the winds have two seasons.

**Known fidelity gaps** (measured, with fixes planned — see `docs/FIX_PLAN.md`):
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
- **GEMSTONES / metals (`Deposits`)** — discrete blobs (`place_deposits`), each with
  its own per-mineral ore-province noise field gated by an elevation floor.
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

### 8.7 Render layers (25) & paint tools (5)
The **biomes** layer is no longer a Köppen recolour — it reads the `biome` column
and draws procedural pattern fills (§8.12).
land, elevation, terrain (hillshade), plates, shelf, ridges, fisheries, currents,
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
- **`TIDAL_MAX_M` is 80 m, not a true tidal elevation.** A cell is 30–110 km
  across, so the honest question is "is this a low depositional coast" — tightening
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
- **Contrast has a floor and a ceiling.** Under about 0.15 between mark and
  ground a pattern is technically present and visually absent (the first cut of
  the scrub dots and the marsh dashes both were); over ~0.20 it stops reading as
  texture and starts reading as a different colour, so two biomes blur together.
  `pattern_amplitude_stays_within_a_readable_band` holds the ceiling — it caught
  peat bog stacking its dash and hummock layers to 0.25.
- **They are SYMBOLS, not surface texture.** Holding a fixed pixel scale across
  the LOD pyramid is correct — that is how printed map hatching behaves.
- **`cargo test --lib render::tile_image::tests::dump_biome_swatch_sheet --
  --ignored --nocapture`** writes a swatch sheet + a tile-seam proof to
  `$BIOME_SHEET_DIR`, rendered through the real `render_tile` path. Use it to
  eyeball a palette or pattern change instead of guessing.

`biome_color` (render) and `BIOME_SWATCH` (`StepSoilResources.tsx` legend) are
two copies of the same palette — change one, change both, or the legend lies.
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
divides, the rest do not) and that it MATTERS (more houses ever founded, lower mean wealth
per house). Note what it does *not* claim: the top share and Gini do not fall under
partible, because a division adds small firms at the bottom as fast as it trims the top.

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
