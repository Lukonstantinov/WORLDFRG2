# WorldForge 2 — Fantasy World Map Generator

Tauri 2 desktop app. Rust backend, React + PixiJS frontend.

## Dev Commands

```bash
run.bat                # Windows: install deps, pull updates, launch dev
npm run tauri dev      # Launch dev mode (Vite + Cargo)
cargo check            # Rust type-check only (run from src-tauri/)
npx tsc --noEmit       # TypeScript type-check only
```

## Architecture

**Stack:** Tauri 2 (Rust) + React 18 + PixiJS 8 + Zustand + SQLite (rusqlite) + zstd compression

**Layout:** `WorkflowPanel (left) | Map (center) | Toolbar (right) | StatusBar (bottom)`

### Tile System
- World grid divided into 128x128 cell tiles (`TILE_SIZE = 128` in `tile/coords.rs`)
- Cylindrical topology: X wraps, Y clamps at poles
- Each tile has columnar data fields (terrain, elevation, temperature, … plus
  `salinity` u8, `shark_risk` u8, `goods: Vec<Vec<u8>>` of `GOODS_COUNT`=45
  trade-good belts, then shipworm/storm/reef/disease u8). Blobs are v2
  self-describing (`[0xF2][2][goods_count u16]`); new fields are **appended
  last** so older `.worldforge` saves still load (trailing reads pad zeros).
- **Column-masked sim loads**: each sim phase loads only the `WorldBuffer`
  columns it touches (`ColumnSet` per-phase masks in `sim/world_buffer.rs`);
  `save()` merges unmodified columns from the old blobs. Run-alls load ALL.
- Tiles stored as zstd-compressed blobs in SQLite
- Rendered server-side as RGBA → packed binary IPC (`get_tiles_packed`) →
  frontend canvas tiles (base64 `get_tiles` kept for compat)
- **LOD pyramid**: lod 1-4 supertiles (one 128×128 image covers 2^L×2^L base
  tiles), persisted in the `tiles` table, invalidated on base-tile writes
- LRU cache of 2000 tiles on frontend (keys layer|lod|tx,ty; chunked fetches)

### Data Flow
```
UI action → bridge/tauri.ts (invoke) → commands/*.rs → sim/*.rs or paint/*.rs
  → WorldBuffer (flat world arrays) → tile_store (SQLite) → tile_image.rs (render)
  → base64 RGBA → TileManager.ts → PixiJS sprites
```

### WorldBuffer Pattern
Simulation operates on flat world-sized arrays (`WorldBuffer` in `sim/world_buffer.rs`).
Load all tiles → run simulation → write back. Never mutate tiles directly during sim.

## World / Campaign split

The WORLD (tiles + `metadata`: geography, climate, rivers/lakes, goods spec,
lat config, `world_progress`) is frozen by **Finalize World**
(`finalize_world` sets `frozen=1` + records `finalized_fp`). Everything human
lives in the `campaign` table (settlements, economy, `campaign_progress`,
`world_ref`) and saves to a separate **`.campaign`** file
(`save_campaign_as`/`open_campaign`, fingerprint-checked). `save_world_as`
strips campaign rows. Paint/template/sim phases 1-6 + run-alls call
`ensure_unfrozen`. Legacy single-file saves migrate in-memory on open
(`legacy=true` → the app offers to split). `import_world_layers` copies layer
groups (terrain/climate/hydrology/soil/hazards/goods) from another world of
the same grid size via `TileData::merge_columns`.

## Key Files

### Rust Backend (`src-tauri/src/`)
```
lib.rs                          ← Entry: plugin registration, command handlers
db/mod.rs                       ← WorldDb (Mutex<Connection>), in-memory SQLite
db/schema.rs                    ← Tables: tiles, metadata, objects, sim_state, undo_journal
db/tile_store.rs                ← load_tile / save_tile (zstd compressed)
tile/cell.rs                    ← TileData: 20 columnar Vec fields per 128x128 tile
tile/coords.rs                  ← TILE_SIZE=128, coordinate math
render/tile_image.rs            ← 15 render layers (land, elevation, climate, etc.)
paint/stroke.rs                 ← PaintValue enum (terrain/elevation/shelf/volcanic)
paint/brush.rs                  ← circle_brush with cylindrical wrapping
sim/world_buffer.rs             ← WorldBuffer: flat arrays, load/save from tiles
sim/plates.rs                   ← Phase 1: Voronoi plate tectonics
sim/elevation.rs                ← Phase 2: plate-based + template-based elevation
sim/ocean.rs                    ← Phase 3a: wind belts, ocean currents, upwelling
sim/temperature.rs              ← Phase 3b: latitude + lapse + current influence
sim/precipitation.rs            ← Phase 3c: moisture advection, ITCZ, orographic
sim/koppen.rs                   ← Phase 4: 22 Köppen climate zones
sim/rivers.rs                   ← Phase 5: D8 flow, rivers, lakes
sim/soil.rs                     ← Phase 6a: 11 soil types from climate
sim/fertility.rs                ← Phase 6b: fertility scoring, fisheries
sim/settlements.rs              ← Phase 7: habitability → city placement
sim/biological.rs               ← Phase 8: shark-habitat risk + trade-good belts
sim/ocean.rs (compute_salinity) ← Phase 3 add-on: wind/E-P salinity + thermohaline coupling
sim/market.rs                   ← Market equilibrium solver (stocks → grain-eq prices → arbitrage; bulk/perish freight)
sim/manufacture.rs              ← Shared production-chain resolver (apply_manufacturing: DAG topo, labor∝pop)
sim/tick.rs (DLC 3)             ← Polis agent (decide_polis_policy: council/tariff/mint/treasury) + Speculation why-engine (compute_speculation: yearly per-polis SpecCenter risk + ranked SpecDriver reason-chain), both at the yearly hook
sim/tick.rs (DLC 3.5)           ← Coin/Credit/Crashes: decide_coinage (named polis coin + sticky coin_trust + seigniorage; reserve coins shave dispatch freight via coin_discount), Bank entity (balance sheet: reserves/loans/real_estate vs deposits/notes; update_banks yearly founding+branches, bank_pass monthly lend/service/fail), trigger_regional_crash (per-component contagion: trust collapse + house haircut + "panic" ActiveEvents + bank runs) fired by fail_bank & maybe_pop_bubbles
commands/sim_commands.rs        ← Tauri commands wrapping sim phases (per-phase ColumnSet masks)
commands/campaign_commands.rs   ← finalize/unfreeze, new/save/open campaign, set_progress
commands/import_commands.rs     ← import_world_layers (layered world import)
commands/template_commands.rs   ← Image → land/sea detection (4-bit quantization)
commands/file_commands.rs       ← Save/open world, export heightmap
history/undo.rs                 ← Tile-level undo/redo journal
```

### React Frontend (`src/`)
```
App.tsx                         ← Layout, header, file dialogs, NewWorldDialog
ui/SpeculationPanel.tsx         ← DLC 3 Finance panel: Speculation (per-polis bubble risk + why-chain) / Poleis (treasury/tariff/mint/council + coin) tabs
ui/CoinCreditPanel.tsx          ← DLC 3.5 Coin/Credit panel: Currencies (reserve ranking) / Banks (T-account balance sheets) / Crashes (regional crisis log) / Schematics (per-city building+estate+bank blueprint) tabs
types.ts                        ← All shared types (WorldMeta, PaintValue, etc.)
bridge/tauri.ts                 ← All IPC invoke wrappers
state/worldStore.ts             ← Zustand: meta, rivers, lakes, settlements
state/uiStore.ts                ← Zustand: tool, layer, workflow step, overlays
state/viewportStore.ts          ← Zustand: camera state, tile invalidation
canvas/TileViewport.ts          ← Pan/zoom, screenToWorld, getVisibleTileRange
canvas/TileManager.ts           ← LRU tile cache, base64→texture, sprite management
canvas/OverlayManager.ts        ← Vector overlays (rivers, settlements, wind arrows)
canvas/PaintOverlay.ts          ← Brush preview, paint stamps
canvas/PixiApp.ts               ← PixiJS 8 application init
ui/workflow/WorkflowPanel.tsx   ← 7-step generation wizard + "Run All" buttons
ui/workflow/Step*.tsx            ← Individual step UIs with prerequisite checks
ui/GoodsEditor.tsx              ← Goods builder: distribution/value/bulk/perish + Manufactured recipe rows
ui/GoodsChainReview.tsx         ← Always-on pre-generation review: planted vs manufactured + SVG recipe DAG
ui/Toolbar.tsx                  ← Tools, layer selector, overlays (RIGHT side)
ui/MapCanvas.tsx                ← PixiJS canvas, pointer events, painting
ui/InfoPanel.tsx                ← Right-click cell inspector
ui/ElevationLegend.tsx          ← Gradient legend (bottom-left, elevation layer only)
ui/ElevationHistogram.tsx       ← Collapsible elevation distribution chart
ui/StatusBar.tsx                ← Bottom status bar
```

## Simulation Pipeline

Run in order. Each phase depends on previous phases' data.

| Phase | Command | What it computes |
|-------|---------|-----------------|
| 1 | `sim_generate_plates` | Tectonic plates, boundaries, terrain (land/sea) |
| 2 | `sim_generate_terrain` | Elevation from plate boundaries + sea depth |
| 2alt | `sim_generate_terrain_from_template` | Elevation from land shape (no plates needed) |
| 2b | `sim_generate_shelves` | Continental shelf with configurable parameters |
| 3 | `sim_ocean_atmosphere` | Wind belts → **salinity** → currents (salinity/density-coupled) → distance_to_ocean → temperature → upwelling → precipitation |
| 4 | `sim_classify_climate` | Köppen classification (22 zones) |
| 5 | `sim_rivers_hydrology` | D8 flow → rivers → lakes (returns overlay data) |
| 6 | `sim_soil_fertility` | Soil types → fertility → fisheries |
| 7 | `sim_generate_settlements` | Habitability scoring → city placement |
| 8 | `sim_biological` | Shark + shipworm risk + trade-good belts (Biological-Trade step; takes seed + gem_deposits) |
| 9 | `compute_political` | (query-only, no tile write) Re-rank settlements by trade power + influence discs |
| 10 | `compute_economy` | (query-only) **Market equilibrium**: stock-based prices in grain-equivalent, barter ratios, currency goods, grain/trade wealth, chains & chokepoints |
| All | `sim_run_all` | Phases 1-8 from plates |
| All | `sim_run_all_from_terrain` | Phases 2alt-8 keeping existing landmass |

### Two Generation Paths
- **From plates:** "Generate Full World" — creates everything from scratch
- **From template/paint:** "Complete from Landmass" — keeps user's land/sea, generates everything else using distance-from-coast elevation

## Rules

1. **Steps must run in order.** Each step checks prerequisites and shows warnings if missing.
2. **WorldBuffer is the simulation unit.** Load all tiles → compute → save. Never modify tiles cell-by-cell during sim.
3. **Undo is tile-level.** Every paint stroke and sim phase saves previous tile states to undo_journal.
4. **Overlays are separate from tiles.** Rivers, settlements, wind arrows are PixiJS Graphics on OverlayManager, not baked into tile images.
5. **Tile rendering is server-side.** Rust renders RGBA pixels per layer, sends base64 to frontend. Frontend only displays textures.
6. **Cylindrical wrapping.** X coordinate wraps (`wrap_x`), Y clamps (`clamp_y`). All BFS, painting, and simulation respect this.
7. **Template detection uses dominant color.** 4-bit quantization → most frequent color → bright=ocean or color-distance threshold.
8. **All sim data stored in TileData columns.** Temperature, precipitation, koppen, etc. are per-cell fields persisted with tiles.

## Science Formulas

### Temperature
```
T_base = 30 - 0.4*|lat|           (0-30°)
       = 18 - 0.7*(|lat|-30)      (30-60°)
       = -3 - 1.2*(|lat|-60)      (60-90°)
T_land = T_base - 5.0 * (elevation * 8848) / 1000    (lapse rate)
T_current = ±3°C from warm/cold ocean currents (decaying inland)
```

### Precipitation
```
1. Moisture advection from ocean downwind (decay 0.935-0.960 per cell)
2. ITCZ boost: +400mm * exp(-lat²/50) within 15° of equator
3. Orographic lift: +50mm per 100m windward slope
4. Frontal: Gaussian peak at 50° latitude
5. Cold coast: -35%, Subtropical high (25-35°): -30%
6. Clamped 50-3000 mm/yr
```

### Fertility
```
F = soil_base*0.30 + precip*0.20 + temp*0.15 + river_prox*0.20 + coast*0.10 + volcanic*0.05
```

### Fisheries
```
Upwelling (shelf + cold current + equatorward flow) + river mouth proximity
```

### Settlement Habitability
```
H = climate*0.40 + fertility*0.20 + water*0.20 + terrain*0.10 + trade*0.10
```

## Render Layers (18)

land, elevation, terrain (hillshade), plates, shelf, fisheries, currents,
temperature, precipitation, wind, climate, biomes, soil, fertility, ridges,
salinity, shark, shipworm

## Biological / salinity overlays (query commands, not stored in tiles)

- `compute_shark_zones` / `compute_shipworm_zones` → highest-risk hazard **cell-mask areas** (overlay under Toolbar → Biological: 🦈 / 🪱)
- `compute_good_regions` → per-good belt **cell-mask areas** (filled cells + outline + emoji; gemstone deposits carry a `sublabel` naming the stone; one toggle per good under Toolbar → Trade Goods)
- `compute_trade_routes(settlements, rivers, reach, max_crossing)` → least-cost routes over the shared coarse cost grid (mountain passes / rivers / coast-hugging), with trade reach limiting open-water crossings.
- `compute_trade_matrix(settlements, rivers, reach, max_crossing)` → settlement-cluster regions, per-good production/demand/net + per-good `flows` (data) + **routed & bundled `trunks`** (the rendered network: each coarse edge's width ∝ total goods volume). Sea-impassable pairs (under the reach) get no flow.
- `compute_political(settlements, rivers, reach, max_crossing)` → settlements re-ranked by **trade power** (0.45·habitability + 0.30·route-centrality + 0.25·good-monopoly); influence discs sized by power (overlay: 👑).
- Trade routes/flows are generated by the **Biological-Trade step** (gated on step 8); the political layer by the **Political step** (9). Trade reach (Global/Coastal+short/Continental) + max-crossing are set in StepBiological (`uiStore.bioParams`).

## Science (salinity / sharks / goods)

### Salinity (`sim/ocean.rs::compute_salinity`, before currents)
```
S(PSU) = 35 + (evaporation − ocean_precip)·5.5
  evaporation  ∝ SST_estimate(lat) · wind_speed   (warm + windy = saltier)
  ocean_precip = ITCZ peak + mid-lat storm track   (latitude bands)
  − coastal runoff freshening (BFS plume from wet coasts)
  + enclosed-sea concentration (narrow warm basins → hypersaline)
stored u8 over 28-42 PSU
```
Thermohaline coupling (`apply_thermohaline`): density = f(salinity, −SST);
denser water boosts current speed ±25%; strong high-latitude sinking extends the
warm-drift reach (the conveyor) → milder downstream coasts. **Currents + climate**
coupling: re-run **Ocean & Atmosphere THEN Climate** after changes.

`advect_salinity_and_recouple` (after `generate_ocean_currents`): semi-Lagrangian
**advection of salinity along the currents** — warm boundary currents carry salty
subtropical water poleward into the fresh high latitudes (Gulf Stream → salty
North Atlantic). The salinity *gradient* (imbalance) then adds a further current-
power boost, and the warm tag is re-extended along the stronger conveyor.

### Shark risk (`sim/biological.rs::compute_shark_risk`)
```
risk = warmth(T) · shallow(shelf/depth) · coast_proximity · prey(fishery)
       + brackish(river mouths + low-salinity) · coast
warmth: 0 ≤10°C → 1 ≥23°C (bull/tiger-shark warm habitat)
```

### Trade goods (`sim/biological.rs::compute_trade_goods`, 21 belts)
`good_score` = climate(Köppen) × temp/precip bands × elevation × fertility × coast
× (fishery/salinity for marine). Distribution depends on `GOOD_UNLIMITED[g]`:
- **UNLIMITED** (stockfish, furs, timber, salt, whaling, wheat, iron) — every
  suitable cell produces (many producers).
- **SEEDED** (the rest) — `localize_good` picks ONE suitability-weighted random
  seed and flood-fills one homeland, with an **island-jump** of ~4% of the map
  width so thin straits / island chains don't chop the belt apart. Land goods
  stop at mountains ≥3000 m (`MOUNTAIN_NORM`≈0.339); marine goods stop where the
  score envelope drops below threshold.
- **GEMSTONES / metals (`Deposits`)** — discrete blobs placed by `place_deposits`.
  Each deposit good now has its **own ore-province noise field** (per-mineral
  `salt`-seeded `fbm_noise`, frequency = `DepositSpec.province_scale`), gated by an
  elevation floor — so tin/copper/gold light up *different* ranges instead of all
  clustering on the single tallest peak (the old pure-elevation candidate bug).
- **MANUFACTURED** (`Distribution::Manufactured`) — finished goods made in cities
  from a recipe (`GoodSpec.inputs`), **no per-cell belt** (placement engine skips
  them). See "Production chains" below. Overlay = the input-belt supply zone.
Goods (`GOOD_NAMES`; `GOOD_MARINE` flags sea goods): silk, wine, oliveoil, sugar,
frankincense, stockfish, spices, tea, coffee, furs, timber, amber, salt, dyes,
incense, pearls, whaling, **wheat, iron, cotton, gemstones**.

### Production chains + transport (`GoodSpec` recipe/transport fields)
Both worldgen (`compute_economy`/`market.rs`) and the campaign tick (`tick.rs`)
read ONE set of fields on `GoodSpec` (all serde-defaulted → old saves load):
- **Transport**: `bulk` (freight weight mult; 1=silk, 3-4=bulky staple) and
  `perishable` (extra freight/day from spoilage). `market.rs::freight_of(good,
  per_day, days) = per_day·days·bulk + perishable·days`; `tick.rs::good_freight`
  mirrors it. Heavy/perishable goods stay regional; silk crosses the world.
- **Chains**: `inputs: Vec<RecipeInput{good, qty}>` + `labor`. The shared
  `sim/manufacture.rs::apply_manufacturing(prod, specs, hub_pop)` topo-orders
  manufactured goods (raws first, cycles/missing inputs disabled with a warning)
  and, per hub, turns input stock into finished output scaled by labor capacity
  (∝ population), so manufacture concentrates in big cities. Worldgen calls it on
  per-hub production before the market solves; the tick runs `manufacture_pass`
  each day (recipe goods are skipped by the per-capita extractor).
- **Builder UI + always-on review**: bulk/perish/recipe/labor are edited in
  `ui/GoodsEditor.tsx` (Manufactured distribution → recipe rows). Goods generation
  **always** routes through `ui/GoodsChainReview.tsx` (a standing convention, not
  optional): a planted-vs-manufactured split + an SVG layered recipe DAG, shown by
  `StepBiological` before `sim_biological` runs (confirm → generate). Shipped chain
  library (`default_custom_goods`): cloth, metalware, refined_sugar, citrus_liqueur.

### Shipworm risk (`sim/biological.rs::compute_shipworm_risk`)
```
risk = warmth(T 13→24) · shallow(shelf/depth) · coast_proximity · brackish
brackish = low-salinity bonus + river-mouth bonus  (Teredo wooden-hull hazard)
```
A separate persisted u8 column (serialized AFTER goods). `compute_shipworm_zones`
clusters only the highest-risk water (Biological hazard sublayer, mirrors sharks).

### Köppen current overrides
Mediterranean (Cs) only forms on **windward (west-facing) coasts** beside a cold
offshore current (`cold_override` gated on `is_windward_ocean` + no warm
influence) — a warm-current **east** coast now reads humid-subtropical (Cfa).

## Paint Tools (5)

Pan, Paint Land (terrain 0/1), Elevation (f32 0-1), Paint Shelf (u8 0/1), Place Volcano (u8 0/1)

## File Operations

- **Save/Open:** SQLite backup API (`.worldforge` files)
- **Export Heightmap:** 16-bit grayscale PNG from elevation data
- **Import Template:** Image → land/sea auto-detection → terrain mask

## Market economy (Part III, replaces per-hop markup pricing)

`sim/market.rs::solve` — pure & deterministic: per-hub stocks (production),
needs ladder (basic/comfort/luxury) with **category substitution** (15
categories on `GoodSpec`: cereal/protein/oil/sweetener/fiber/drink/…; short of
wheat → buys rice at a penalty), local price `base_value·(need/stock)^0.6` in
the **grain-equivalent numeraire** (wheat=1, `GoodSpec.base_value`), arbitrage
on live prices with freight (`freight_of` = `per_day·days·bulk + perishable·days`,
so bulky/perishable goods stay regional) and import caps at delivered-cost parity
→ decaying spatial price gradients, no terminal cap.
`compute_economy` feeds it travel-days over its trade graph and emits
`EconHub.market` (prices vs world standard, in/out flows, exchange ratios
against the hub's top exports, currency goods, grain_wealth + trade_wealth).
Hub `wealth` = normalized(grain + 1.5·trade + 0.25·centrality).

Goods: 45 builtins (38 + rice, barley, millet, herring, honey, hides, beer) +
declarative customs incl. 4 **Manufactured** chain goods (cloth, metalware,
refined_sugar, citrus_liqueur); tobacco/frankincense/indigo ship disabled (~1400
curation). `backfill_market_fields` fills category/tier/base_value **and
bulk/perishable** on specs from pre-market/pre-transport saves.

Parts IV-V (tick simulation "Living Trade", merchant dynasties) are FUTURE
DLC — see docs/REDESIGN_AND_DLC_PLAN.md Parts IV-V.
