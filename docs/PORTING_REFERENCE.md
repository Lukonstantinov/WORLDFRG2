# WorldForge 2 - Porting Reference from WorldForge 1

This document maps every WorldForge 1 feature to its WorldForge 2 status (done/todo) and notes the WF1 source file to reference during porting.

---

## Architecture Differences

| Aspect | WF1 | WF2 |
|--------|-----|-----|
| Runtime | Browser (Chrome) | Desktop (Tauri 2) |
| Backend | Express.js server | Rust (native) |
| Simulation | Web Workers (JS) | Rust (rayon parallelism) |
| Storage | JSON/gzip files | SQLite (.worldforge) |
| Memory model | Single cells[] array | 128x128 tile-based |
| Rendering | PixiJS per-cell | PixiJS per-tile (RGBA from Rust) |
| Memory limit | ~1.4 GB (V8 heap) | No practical limit |
| State | Zustand | Zustand (frontend) + Rust state |

---

## Feature Status Matrix

### Legend
- DONE = implemented and compiles in WF2
- PARTIAL = scaffolded/stubbed, needs logic
- TODO = not started

---

## 1. Core Infrastructure

| Feature | Status | WF2 Location | WF1 Source |
|---------|--------|-------------|-----------|
| Tile coordinate system | DONE | `tile/coords.rs` | N/A (new) |
| Cell data structure (21 fields) | DONE | `tile/cell.rs` | `shared/src/types.ts` (MapCell) |
| Columnar compress/decompress | DONE | `tile/cell.rs` | N/A (new, replaces cells[]) |
| SQLite schema (5 tables) | DONE | `db/schema.rs` | N/A (new, replaces JSON files) |
| Tile CRUD (save/load/version) | DONE | `db/tile_store.rs` | `server/src/utils/worldStorage.ts` |
| World metadata KV store | DONE | `db/metadata.rs` | `server/src/routes/maps.ts` |
| New world creation | DONE | `commands/world_commands.rs` | `client/src/state/worldStore.ts` |
| Undo/redo (SQLite journal) | DONE | `history/undo.rs` | `client/src/state/historyStore.ts` |
| IPC bridge (Tauri invoke) | DONE | `commands/*.rs` + `bridge/tauri.ts` | N/A (was Express REST) |
| Base64 binary transfer | DONE | `commands/tile_commands.rs` | N/A (was Web Worker buffers) |

---

## 2. Rendering Layers

| Layer | Status | WF2 Renderer | WF1 Source |
|-------|--------|-------------|-----------|
| Land/Sea | DONE | `render/tile_image.rs::render_land` | `layers/land/LandLayer.ts` |
| Elevation | DONE | `render/tile_image.rs::render_elevation` | `layers/elevation/ElevationLayer.ts` |
| Climate (Koppen colors) | DONE | `render/tile_image.rs::render_climate` | `layers/climate/ClimateLayer.ts` + `climate/koppen-colors.ts` |
| Temperature | DONE | `render/tile_image.rs::render_temperature` | (embedded in ClimateLayer) |
| Precipitation | DONE | `render/tile_image.rs::render_precipitation` | `layers/precipitation/PrecipitationLayer.ts` |
| Soil types | DONE | `render/tile_image.rs::render_soil` | `layers/soil/SoilLayer.ts` |
| Fertility | DONE | `render/tile_image.rs::render_fertility` | `layers/fertility/FertilityLayer.ts` |
| Plates | DONE | `render/tile_image.rs::render_plates` | `layers/plates/PlatesLayer.ts` |
| Shallow Ocean / Shelf | TODO | - | `layers/shallow-ocean/ShallowOceanLayer.ts` |
| Mountains (markers) | TODO | - | `layers/mountains/MountainLayer.ts` |
| Volcanic (markers) | TODO | - | `layers/volcanic/VolcanicLayer.ts` |
| Wind Belts (vectors) | TODO | - | `layers/wind-belts/WindBeltsLayer.ts` |
| Ocean Currents (vectors) | TODO | - | `layers/ocean-currents/OceanCurrentsLayer.ts` |
| Rivers (vector overlay) | TODO | - | `layers/rivers/RiversLayer.ts` |
| Watersheds | TODO | - | `layers/watersheds/WatershedLayer.ts` |
| Biomes | TODO | - | `layers/biomes/BiomeLayer.ts` |
| Fisheries | TODO | - | `layers/fisheries/FisheriesLayer.ts` |
| Population / Settlements | TODO | - | `layers/population/PopulationLayer.ts` |
| Trade Routes | TODO | - | `layers/trade-routes/TradeRoutesLayer.ts` |
| Latitude Lines | TODO | - | `layers/latitude-lines/LatitudeLinesLayer.ts` |
| Ridges | TODO | - | `layers/ridges/RidgeLayer.ts` |
| Terrain (shaded relief) | TODO | - | `layers/terrain/TerrainLayer.ts` |

---

## 3. Simulation Algorithms

### 3a. Tectonic Plates (Phase 5)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Plate seed generation (Voronoi) | TODO | `sim/plates.rs` | `science/plates/plate-generator.ts` (68) |
| Cell-to-plate assignment | TODO | `sim/plates.rs` | `science/plates/plate-generator.ts` |
| Boundary classification (conv/div/transform) | TODO | `sim/plates.rs` | `science/plates/plate-boundaries.ts` (55) |
| Plate stress computation | TODO | `sim/plates.rs` | `science/plates/plate-stress.ts` (47) |
| Land derivation from plate density | TODO | `sim/plates.rs` | `science/plates/derive-landmass.ts` (53) |
| Image template import | TODO | `sim/plates.rs` | `science/plates/image-template.ts` (80) |

### 3b. Terrain Generation (Phase 5)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Multi-octave noise + FBM | TODO | `sim/terrain.rs` | `science/terrain/random-elevation.ts` (521) |
| Hydraulic erosion (droplet sim) | TODO | `sim/terrain.rs` | `science/terrain/random-elevation.ts` |
| Thermal erosion | TODO | `sim/terrain.rs` | `science/terrain/random-elevation.ts` |
| Ridge-based elevation (Gaussian falloff) | TODO | `sim/terrain.rs` | `science/terrain/ridge-elevation.ts` (309) |
| Mountain ridge generation | TODO | `sim/terrain.rs` | `science/terrain/mountain-ridges.ts` (42) |
| Volcanic zone placement | TODO | `sim/terrain.rs` | `science/terrain/volcanic-zones.ts` (68) |
| Sea depth / bathymetry | TODO | `sim/terrain.rs` | `science/terrain/sea-depth.ts` (249) |
| Elevation redistribution | TODO | `sim/terrain.rs` | `science/terrain/elevation-redistribute.ts` (71) |
| Elevation presets (flat/hilly/mountain) | TODO | `sim/terrain.rs` | `science/terrain/elevation-presets.ts` (146) |
| Ridge presets | TODO | `sim/terrain.rs` | `science/terrain/ridge-presets.ts` (91) |
| Gaussian elevation from mountain points | TODO | `sim/terrain.rs` | `science/terrain/elevation.ts` (34) |
| Continental shelf generation | TODO | `sim/terrain.rs` | `science/terrain/sea-depth.ts` |
| Heightmap export (16-bit PNG) | TODO | `sim/terrain.rs` | `science/terrain/heightmap-export.ts` (112) |

### 3c. Ocean Currents (Phase 6)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Gyre-based current generator | TODO | `sim/ocean.rs` | `science/ocean/current-generator.ts` (638) |
| Western boundary intensification | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| Equatorial current systems | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| Subpolar gyres | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| Antarctic Circumpolar Current | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| Isobath steering (bathymetry) | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| Venturi strait constriction | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| 20-pass coastline deflection | TODO | `sim/ocean.rs` | (inside current-generator.ts) |
| Current temperature (warm/cold) | TODO | `sim/ocean.rs` | `science/ocean/current-temperature.ts` (103) |
| Coastal upwelling zones | TODO | `sim/ocean.rs` | `science/ocean/upwelling-zones.ts` (68) |

### 3d. Atmosphere (Phase 6)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Wind belt assignment | TODO | `sim/atmosphere.rs` | `science/atmosphere/wind-belts.ts` (39) |
| ITCZ shift computation | TODO | `sim/atmosphere.rs` | `science/atmosphere/itcz.ts` (158) |
| Precipitation (base + ITCZ + orographic) | TODO | `sim/atmosphere.rs` | `science/atmosphere/precipitation.ts` (220) |
| Orographic lift/rain shadow | TODO | `sim/atmosphere.rs` | `science/atmosphere/orographic-lift.ts` (65) |
| Ocean-atmosphere buffer solver | TODO | `sim/atmosphere.rs` | `science/atmosphere/ocean-atmosphere-buffer.ts` (672) |

### 3e. Climate Classification (Phase 7)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Temperature computation | TODO | `sim/climate.rs` | `science/climate/temperature.ts` (117) |
| Koppen classification (22 zones) | TODO | `sim/climate.rs` | `science/climate/koppen.ts` (488) |
| Climate settings / thresholds | TODO | `sim/climate.rs` | `science/climate/climate-settings.ts` (30) |

### 3f. Rivers & Hydrology (Phase 8)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| D8 flow direction | TODO | `sim/hydrology.rs` | `science/hydrology/river-flow.ts` (465) |
| Flow accumulation | TODO | `sim/hydrology.rs` | (inside river-flow.ts) |
| River extraction (threshold) | TODO | `sim/hydrology.rs` | (inside river-flow.ts) |
| River mouth detection | TODO | `sim/hydrology.rs` | (inside river-flow.ts) |
| Upstream tracing | TODO | `sim/hydrology.rs` | (inside river-flow.ts) |
| Silt load calculation | TODO | `sim/hydrology.rs` | `science/hydrology/silt-transport.ts` (179) |
| Lake/sink detection (BFS) | TODO | `sim/hydrology.rs` | `science/hydrology/lake-detection.ts` (121) |
| Watershed delineation | TODO | `sim/hydrology.rs` | `science/hydrology/watershed.ts` (52) |

### 3g. Fertility & Resources (Phase 9)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Soil type assignment (Koppen-based) | TODO | `sim/fertility.rs` | `science/fertility/soil-types.ts` (70) |
| Alluvial soil / floodplain enrichment | TODO | `sim/fertility.rs` | `science/fertility/fertility-buffer.ts` (327) |
| Fertility scoring (multi-factor) | TODO | `sim/fertility.rs` | `science/fertility/fertility-score.ts` (88) |
| Fishery scoring (upwelling+shelf) | TODO | `sim/fertility.rs` | `science/fertility/fishery-score.ts` (174) |

### 3h. Human Geography (Phase 9)

| Algorithm | Status | WF2 Target | WF1 Source (lines) |
|-----------|--------|-----------|-------------------|
| Habitability scoring | TODO | `sim/human.rs` | `science/human/habitability.ts` (257) |
| Settlement generation | TODO | `sim/human.rs` | `science/human/settlement-generator.ts` (162) |
| Procedural naming | TODO | `sim/human.rs` | (inside settlement-generator.ts) |
| Territory computation | TODO | `sim/human.rs` | `science/human/territory.ts` (128) |
| Trade route A* pathfinding | TODO | `sim/human.rs` | `science/human/trade-routes.ts` (234) |

### 3i. Simulation Orchestrator

| Feature | Status | WF2 Target | WF1 Source (lines) |
|---------|--------|-----------|-------------------|
| Simulation scope dispatch | TODO | `sim/mod.rs` | `workers/simulation.worker.ts` (409) |
| Progress events (sim-progress) | TODO | `sim/mod.rs` | `workers/simulationRunner.ts` (261) |
| Full grid load for simulation | TODO | `sim/mod.rs` | N/A (new - load all tiles into memory) |
| Write tiles back after sim | TODO | `sim/mod.rs` | N/A (new - chunk results into tiles) |

---

## 4. Tools

| Tool | Status | WF2 Location | WF1 Source (lines) |
|------|--------|-------------|-------------------|
| Pan tool | DONE | `ui/MapCanvas.tsx` | `tools/PanTool.ts` (18) |
| Paint land/sea | DONE | `ui/MapCanvas.tsx` + `paint/stroke.rs` | `tools/PaintTool.ts` (141) |
| Elevation paint | DONE | `ui/MapCanvas.tsx` + `paint/stroke.rs` | `tools/ElevationTool.ts` (150) |
| Circle brush | DONE | `paint/brush.rs` | (inside PaintTool.ts) |
| Select/inspect cell | PARTIAL | `commands/query_commands.rs` (backend done) | `tools/SelectTool.ts` (58) |
| Shelf paint tool | TODO | - | `tools/ShelfPaintTool.ts` (184) |
| Ridge paint tool | TODO | - | `tools/RidgePaintTool.ts` (109) |
| Mountain placement | TODO | - | `tools/MountainTool.ts` (38) |
| Volcano placement | TODO | - | `tools/VolcanoTool.ts` (42) |
| River seed tool | TODO | - | `tools/RiverSeedTool.ts` (127) |

---

## 5. UI Components

| Component | Status | WF2 Location | WF1 Source |
|-----------|--------|-------------|-----------|
| New World dialog | DONE | `App.tsx` | (in WF1 client) |
| Header bar | DONE | `App.tsx` | header component |
| Toolbar (tools + layers) | DONE | `ui/Toolbar.tsx` | left toolbar |
| Status bar (coords) | DONE | `ui/StatusBar.tsx` | status bar |
| Map canvas (PixiJS) | DONE | `ui/MapCanvas.tsx` | canvas component |
| Brush radius slider | DONE | `ui/Toolbar.tsx` | tool options bar |
| Elevation height slider | DONE | `ui/Toolbar.tsx` | tool options bar |
| Workflow panel (7 steps) | TODO | - | workflow panel |
| Layer panel (toggle/opacity) | TODO | - | layer panel |
| Info panel (cell details) | TODO | - | info panel |
| Elevation legend | TODO | - | elevation legend |
| Debug log panel | TODO | - | debug panel |
| Shelf generator dialog | TODO | - | shelf dialog |
| Climate settings dialog | TODO | - | climate settings |
| Template manager | TODO | - | template dialog |
| Elevation distribution | TODO | - | histogram |
| Open/save world dialogs | TODO | - | file dialogs |

---

## 6. File Operations

| Feature | Status | WF2 Target | WF1 Source |
|---------|--------|-----------|-----------|
| Save world (SQLite = file) | PARTIAL | `db/WorldDb` | `server/src/utils/worldStorage.ts` |
| Open world dialog | TODO | file dialog + `open_world` command | N/A |
| Save As dialog | TODO | file dialog + `save_world_as` command | N/A |
| WF1 import (.json.gz) | TODO | `import/v1_import.rs` | `state/compactSave.ts` |
| Heightmap export (PNG) | TODO | - | `science/terrain/heightmap-export.ts` |

---

## 7. Keyboard Shortcuts

| Shortcut | Status | Notes |
|----------|--------|-------|
| Ctrl+Z (Undo) | DONE | MapCanvas.tsx |
| Ctrl+Y / Ctrl+Shift+Z (Redo) | DONE | MapCanvas.tsx |
| Mouse wheel (Zoom) | DONE | TileViewport.ts |
| Middle-click drag (Pan) | DONE | MapCanvas.tsx |
| Ctrl+Shift+D (Debug panel) | TODO | |
| Shift+drag (Erase mode) | TODO | |

---

## Implementation Priority Order

Based on the plan phases:

1. **Phase 5 - Tectonics + Terrain** (~2,000 lines TS to port)
   - Port `plate-generator.ts`, `plate-boundaries.ts`, `plate-stress.ts`
   - Port `random-elevation.ts` (noise, erosion), `ridge-elevation.ts`
   - Port `volcanic-zones.ts`, `mountain-ridges.ts`, `sea-depth.ts`
   - Build simulation orchestrator (`sim/mod.rs`)
   - Add sim progress events

2. **Phase 6 - Ocean + Atmosphere** (~1,960 lines TS to port)
   - Port `current-generator.ts` (most complex algorithm)
   - Port `wind-belts.ts`, `itcz.ts`, `precipitation.ts`, `orographic-lift.ts`
   - Port `current-temperature.ts`, `upwelling-zones.ts`

3. **Phase 7 - Climate** (~695 lines TS to port)
   - Port `temperature.ts`, `koppen.ts`
   - Add climate layer rendering

4. **Phase 8 - Rivers** (~817 lines TS to port)
   - Port `river-flow.ts` (D8 algorithm)
   - Port `lake-detection.ts`, `silt-transport.ts`, `watershed.ts`
   - River vector overlay in frontend (PixiJS Graphics)

5. **Phase 9 - Fertility + Human** (~1,440 lines TS to port)
   - Port `soil-types.ts`, `fertility-buffer.ts`, `fertility-score.ts`, `fishery-score.ts`
   - Port `habitability.ts`, `settlement-generator.ts`, `trade-routes.ts`

6. **Phase 10-12 - Polish**
   - LOD generation
   - WF1 import
   - Full UI (workflow panel, layer panel, info panel, dialogs)
   - Keyboard shortcuts, theme support

---

## Key WF1 Files to Reference During Porting

### Most Complex (port carefully)
1. `science/ocean/current-generator.ts` (638 lines) - Gyre system
2. `science/terrain/random-elevation.ts` (521 lines) - Noise + erosion
3. `science/climate/koppen.ts` (488 lines) - Climate rules
4. `science/hydrology/river-flow.ts` (465 lines) - D8 flow
5. `science/atmosphere/ocean-atmosphere-buffer.ts` (672 lines) - Coupled solver

### Data Format Reference
- `shared/src/types.ts` - All data structures
- `workers/cellPack.ts` (239 lines) - Binary field layout
- `state/compactSave.ts` (298 lines) - Save format (for v1 import)

### Grid Conventions
- Index: `y * width + x` (row-major)
- Wrapping: X wraps (cylinder world), Y clamps
- Elevation: 0.0-1.0 normalized (0 = sea level, 1.0 = 8848m)
- Terrain: 0 = sea, 1 = land
- Lock bits: bitfield per cell preserving manual edits across re-simulation
