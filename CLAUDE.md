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
  `salinity` u8, `shark_risk` u8, `goods: Vec<Vec<u8>>` of `GOODS_COUNT`=21
  trade-good belts, and `shipworm_risk` u8 — serialized LAST, after goods). New fields are **appended last** in `compress`/`decompress`
  so older `.worldforge` saves still load (trailing reads pad with zeros).
- Tiles stored as zstd-compressed blobs in SQLite
- Rendered server-side as RGBA → base64 → transferred to frontend as PixiJS textures
- LRU cache of 2000 tiles on frontend

### Data Flow
```
UI action → bridge/tauri.ts (invoke) → commands/*.rs → sim/*.rs or paint/*.rs
  → WorldBuffer (flat world arrays) → tile_store (SQLite) → tile_image.rs (render)
  → base64 RGBA → TileManager.ts → PixiJS sprites
```

### WorldBuffer Pattern
Simulation operates on flat world-sized arrays (`WorldBuffer` in `sim/world_buffer.rs`).
Load all tiles → run simulation → write back. Never mutate tiles directly during sim.

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
commands/sim_commands.rs        ← Tauri commands wrapping sim phases
commands/template_commands.rs   ← Image → land/sea detection (4-bit quantization)
commands/file_commands.rs       ← Save/open world, export heightmap
history/undo.rs                 ← Tile-level undo/redo journal
```

### React Frontend (`src/`)
```
App.tsx                         ← Layout, header, file dialogs, NewWorldDialog
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
- **GEMSTONES** — special: `place_gem_deposits` scatters `gem_deposits` discrete
  **highland-locked** blobs (elev ≥ 0.40) worldwide (global, not climate-bound);
  `compute_good_regions` names each deposit a stone (Ruby/Sapphire/Emerald/…).
Goods (`GOOD_NAMES`; `GOOD_MARINE` flags sea goods): silk, wine, oliveoil, sugar,
frankincense, stockfish, spices, tea, coffee, furs, timber, amber, salt, dyes,
incense, pearls, whaling, **wheat, iron, cotton, gemstones**.

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
