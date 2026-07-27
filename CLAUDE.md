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

### 2.4 Keep this file true
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
  `sst` f32 · `snow_frac` (**currently last**). Blobs are **v2 self-describing**
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
A world carries four planetary knobs in `metadata`, read into `WorldBuffer`
(`obliquity_deg`, `rotation_rate`, `solar_lum`, `greenhouse`; defaults in
`world_commands.rs`, edited via `LatitudeControl.tsx`). They drive real physics:
obliquity → the insolation integral, rotation → circulation-belt latitudes AND
EBM heat transport, luminosity/greenhouse → the energy budget.

**At Earth values every one of them is a no-op by construction** — the EBM is solved
twice and only the anomaly `T_world(φ) − T_earth(φ)` is applied, and `Circulation`
returns exactly 30°/60°. That's what keeps the Earth calibration bit-for-bit intact
while the knobs still move real physics. Preserve this property in any change here.

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
| 7 | `sim_generate_settlements` | Habitability scoring → city placement |
| 7b | `sim_generate_provinces` | Watershed/cost-flood province partition (AFTER settlements) |
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

Extras: `sim_generate_terrain_ridged`, `sim_scale_elevation`, `sim_invert_terrain`,
`sim_generate_toponyms` (#26, gated on cultures+rivers), `sim_refresh_hydrology_biology`.

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

Tests live in `tick/tests.rs` — incl. `simulate_decades_reports_dynamics`
(the standing dynamics run) and `bench_campaign_tick` (ignored). See the DLC docs
in §9 for design detail.

### 5.1 Known structural limits (read before extending)
Three facts about the campaign that are easy to miss and shape any change here:

- **One mutating verb.** Of 60+ campaign commands, exactly one mutates a running sim:
  `campaign_advance(ticks)`. The UI is play/pause + week/month/year. Every AI
  `decide_*` function is a *latent player verb* — see item B2 in `docs/FIX_PLAN.md`.
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
  goods_commands.rs             ← Goods spec CRUD, default_custom_goods, backfill
  import_commands.rs            ← import_world_layers (layered world import)
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
  mod.rs · tile_image.rs        ← 25 render layers (land, elevation, climate, … see §8.7)

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
  step4_climate/                ← Ph4: koppen.rs (31 zone codes + H) ·
      earth_validation.rs         THE EARTH FIDELITY GATE (§2.3) + fixtures/
  step5_rivers/                 ← Ph5: rivers.rs (priority-flood/rivers/lakes) · aquatic.rs
                                  (freshwater ecology: fish assemblage, lake limnology)
  step6_soil_fertility/         ← Ph6: soil.rs (12 soil types) · fertility.rs (fisheries)
  step7_settlements/settlements.rs ← Ph7: habitability → city placement (Settlement struct)
  step8_biological_goods/       ← Ph8: biological.rs (shark/shipworm + belts + deposits)
                                  · goods_spec.rs (GoodSpec, 45 belts + ~21 manufactured)
  shared/                       ← cultures.rs (organic peoples map + 14 traits) · toponyms.rs
                                  · names.rs (deterministic place/family/head names)
                                  · provinces.rs (watershed/cost-flood partition — the
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
  settingsStore.ts              ← app appearance/settings

canvas/
  PixiApp.ts                    ← PixiJS 8 application init
  TileViewport.ts               ← Pan/zoom, screenToWorld, getVisibleTileRange
  TileManager.ts                ← LRU tile cache, base64→texture, sprite management
  OverlayManager.ts             ← ALL vector overlays (~4k lines: rivers, settlements, wind,
                                  trunks, routes, dynamic flow, regions). visibility[type] gates each
  PaintOverlay.ts               ← Brush preview, paint stamps
  projection.ts                 ← lat/lon ↔ world-cell projection helpers
  goodIcons.ts                  ← good → emoji/texture for overlays

ui/world/  — map & world
  MapCanvas.tsx                 ← PixiJS canvas, pointer events, painting, draws every overlay
  Toolbar.tsx                   ← Tools, layer selector, overlay toggles (RIGHT side)
  StatusBar.tsx · WindowBar.tsx ← Bottom status / window chrome
  InfoPanel.tsx                 ← Right-click cell inspector
  ElevationLegend/Histogram.tsx ← Elevation legend + distribution chart
  LatitudeControl.tsx           ← Latitude band config
  climate.ts                    ← Köppen → human phrase helpers
  HydrologyPanel.tsx            ← Rivers/lakes + aquatic (fish assemblage, limnology)
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
  HousesPanel/DynastiesPanel/GuildsPanel.tsx ← Merchant houses, dynasties, guilds
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
land, elevation, terrain (hillshade), plates, shelf, ridges, fisheries, currents,
**sst** (sea-surface temperature), temperature, precipitation, wind, **windspeed**
(low-level wind intensity incl. jets), **snow** (annual snow-cover fraction), climate,
biomes, soil, fertility, salinity, habitability, shark, shipworm, reef, storm, disease.
Paint: Pan, Paint Land (0/1), Elevation (f32 0-1), Paint Shelf (u8 0/1), Place Volcano (u8 0/1).

### 8.8 File operations
Save/Open via SQLite backup API (`.worldforge` / `.campaign`); Export Heightmap
(16-bit grayscale PNG from elevation); Export Layers / trade data; Import Template
(image → land/sea auto-detection).

---

## 9. Docs (`docs/`)

**START HERE — the current plan**
```
FIX_PLAN.md                       ← ⭐ Measured Earth-fidelity baseline + the prioritised
                                    fix plan (climate · one-simulator · economy · society),
                                    with a regression gate per item. Read before planning work.
```
**World / trade / architecture**
```
FEATURES_AND_PROPOSALS.md         ← Feature catalog, step-by-step generation, per-step
                                    improvement ideas & campaign-integration proposals
REDESIGN_AND_DLC_PLAN.md          ← World/trade split, perf overhaul & DLC master plan
IMPLEMENTATION_PLAN.md            ← Redesign implementation plan & status
PERFORMANCE_OPTIMIZATION_PLAN.md  ← Campaign performance & stability plan
TRADE_CARTOGRAPHY_SPEC.md         ← Trade cartography & good-flow spec
TRADE_SYSTEM_REVIEW.md            ← Trade & economy system review
TRADE_BASE_MECHANIC_PLAN.md       ← Houses develop small cities as bases
trade-goods-and-hazards-design.md ← Goods & seasonal hazards design
CLIMATE_CORRELATION_BRAINSTORM.md ← Real-climate correlation & biology proposals
```
**Finance / economy DLCs**
```
FINANCE_POLIS_SPECULATION_PLAN.md ← DLC 3: finance, the polis & speculation
SOCIAL_ECONOMIC_WEALTH_PROPOSAL.md← Social/economic/wealth analysis & proposals
SYSTEMS_21_PROPOSALS.md           ← Systems 2.1: perf · manufactories · banks · houses
FUTURES_CONTRACTS_PLAN.md         ← Futures contracts design
HERALDRY_AND_NAMES_VARIANTS.md    ← Heraldry, house names & guilds variants
```
**Expansion systems**
```
EXPEDITIONS_CORRIDORS_PLAN.md            ← Expeditions & corridors
SATELLITE_CONSTRUCTION_AND_MIGRATION_PLAN.md ← Satellite build + route-bound migration
FUTURE_SYSTEMS_PLAN.md                   ← Future systems feature plan
ROADMAP_BATCHES.md                       ← The 24 picked features, batched
VICTORIA2_DLC_IMPLEMENTATION.md          ← Victoria-2 layer, DLC-by-DLC
VICTORIA2_REDESIGN_PROPOSAL.md           ← Victoria-2-style UI/UX redesign
```
**Analyses & newer systems**
```
POPULATION_AND_NETWORK_DYNAMICS_ANALYSIS.md ← Growth/trade-network analysis & fix plan
SETTLEMENT_BELIEVABILITY_ANALYSIS.md        ← Settlement generation believability review
SETTLEMENT_TIER_REQUIREMENTS.md             ← Settlement development ladder (5 tiers)
PROVINCE_SYSTEM_PLAN.md                     ← Province partition layer (see FIX_PLAN B1)
GROWTH_AND_GRAVITY_SCHEMATICS.md            ← Growth & gravity model schematics
CITY_STORES_PANEL_SCHEMATIC.md              ← City stores panel design
IN_APP_VERIFICATION_CHECKLIST.md            ← Manual in-app verification checklist
```
`docs/` also has: `BIOLOGICAL_STEP_PLAN.md`, `C_BATCH_PLAN.md`,
`FIFTEENTH_BATCH_PLAN.md`, `PORTING_REFERENCE.md`. Historical HTML/SVG mockups are
archived under `docs/mockups/_archive/`; a stray reference image lives in
`docs/reference/`. The repo root now holds only `README.md` and `CLAUDE.md`.

> **Scope warning.** There are 30+ planning docs here and far more written-down ideas
> than can be built. Most are proposals, NOT commitments — treat them as a menu, and
> check `FIX_PLAN.md` for what's actually prioritised before starting new work.

---

## 10. Conventions checklist

1. **Steps run in order** — each phase checks prerequisites, warns if missing.
2. **WorldBuffer is the sim unit** — load all → compute → save; never per-cell during sim.
3. **Undo is tile-level** — every stroke/phase journals prior tile state.
4. **Overlays are separate from tiles** — PixiJS Graphics on OverlayManager, gated by `visibility[type]`.
5. **Rendering is server-side** — Rust renders RGBA, frontend only displays.
6. **Cylindrical wrapping** — X wraps, Y clamps; all BFS/paint/sim respect it.
7. **New tile fields append LAST** — v2 self-describing blobs; trailing reads pad zeros (old saves load).
8. **Every `#[tauri::command]` is registered in `lib.rs`** and gets a wrapper in `bridge/` (via the @bridge barrel).
9. **New TS types mirror Rust serde structs** in `types/` (world/campaign/goods).
10. **Earth params must stay a no-op** — the EBM anomaly and `Circulation` return zero /
    exactly 30°-60° at Earth settings. Never let a planetary knob shift the Earth baseline.
11. **Phase 3's order is duplicated** in `sim_commands.rs` and `earth_validation.rs` —
    change one, change both, or the fidelity gate stops testing the real pipeline.
12. **After any `tick/` change** → run the dynamics test (§2.1). **After any
    `step3_ocean_atmo/` or `step4_climate/` change** → run the Earth fidelity gate (§2.3).
    **After any verified change** → push to `main` (§2.2), and keep this file true (§2.4).
