# WorldForge 2

**A desktop fantasy-world map generator that simulates a planet from tectonics to trade.**

WorldForge 2 is a [Tauri](https://tauri.app/) desktop application that procedurally generates plausible, science-grounded world maps. Starting from either tectonic plates or a hand-drawn/imported landmass, it runs a multi-stage physical simulation — elevation, ocean currents, salinity, climate, rivers, soils, settlements, and trade — and renders the result as a fully zoomable, layered map.

It is a ground-up rewrite of the original browser-based WorldForge. The browser version hit a hard memory ceiling (~1.4 GB) and crashed on large grids; moving the simulation into a native Rust backend removes that limit, so WorldForge 2 can simulate worlds at 3600×1800 and beyond.

> **Status:** Active development. The backend compiles cleanly and all simulation phases are implemented. Worlds generate end-to-end; ongoing work focuses on the plausibility of climate, currents, and the trade/political layers.

---

## Highlights

- **Whole-planet simulation, in order.** Nine dependent phases turn bare land into a living world — tectonics → elevation → ocean & atmosphere → climate → rivers → soil & fertility → settlements → biology & trade → politics.
- **Two ways to start.** Generate everything from procedural **plate tectonics**, or **import an image / paint** your own continents and let the simulation fill in everything else from the coastline.
- **Real climatology.** Wind belts, ITCZ migration, orographic rain shadows, ocean gyres, boundary currents, upwelling, a thermohaline "conveyor", and a full **22-zone Köppen** classification.
- **Oceanography.** Sea-surface **salinity** (evaporation − precipitation, runoff freshening, enclosed-sea concentration) is advected along currents and coupled back into current strength.
- **Living economy.** 21 **trade goods** distributed by climate/terrain suitability, **shark** and **shipworm** hazard maps, least-cost **trade routes** (mountain passes, navigable rivers, coast-hugging sea lanes), a **trade matrix** of production/demand/flows, and a **political layer** ranking settlements by trade power.
- **Layered, tiled rendering.** 18 render layers (elevation, climate, biomes, soil, fertility, currents, salinity, etc.) drawn server-side and streamed as tiles to a PixiJS canvas, plus vector overlays for rivers, settlements, wind, currents, and trade.
- **Export.** 16-bit grayscale heightmap PNGs and per-layer image exports for use in other tools.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | [Tauri 2](https://tauri.app/) (Rust) |
| Backend | Rust — simulation, rendering, storage |
| Storage | SQLite (`rusqlite`, bundled) with zstd-compressed tile blobs |
| Frontend | React 18 + [PixiJS 8](https://pixijs.com/) + [Zustand](https://github.com/pmndrs/zustand) |
| Build | Vite 5 + TypeScript 5 |

---

## How It Works

### Tile architecture
The world is divided into **128×128-cell tiles** with cylindrical topology (the X axis wraps around the globe; the Y axis clamps at the poles). Each tile stores ~20 columnar per-cell fields (terrain, elevation, temperature, precipitation, Köppen zone, salinity, shark/shipworm risk, trade-good belts, …) as a single zstd-compressed blob in SQLite. New fields are appended at the end of the serialization so older `.worldforge` saves keep loading.

### The WorldBuffer pattern
Simulation never mutates tiles cell-by-cell. Instead each phase:

```
load all tiles → flat WorldBuffer arrays → run the phase → write back to tiles
```

This keeps neighbour lookups (BFS flood-fills, flow accumulation, advection) simple and fast across tile boundaries.

### Server-side rendering
Rust renders each map layer to RGBA pixels per tile and ships them as base64 to the frontend, where PixiJS displays them as textures (LRU-cached, 2000 tiles). The frontend is a pure viewer for tile imagery; all the heavy lifting is native. Vector data (rivers, settlements, wind/current arrows, trade routes) is drawn as PixiJS Graphics overlays on top, never baked into the tiles.

```
UI action → bridge/tauri.ts (invoke) → commands/*.rs → sim/*.rs
  → WorldBuffer → SQLite tiles → tile_image.rs (render) → base64 RGBA
  → TileManager.ts → PixiJS sprites
```

---

## Simulation Pipeline

Phases run in order; each depends on the previous ones.

| # | Phase | Computes |
|---|-------|----------|
| 1 | **Plates** | Voronoi tectonic plates, boundaries, initial land/sea |
| 2 | **Elevation** | Elevation from plate boundaries (or from a template landmass via distance-to-coast) + continental shelves |
| 3 | **Ocean & Atmosphere** | Wind belts → **salinity** → ocean currents (salinity/density-coupled) → distance-to-ocean → temperature → upwelling → precipitation |
| 4 | **Climate** | 22-zone Köppen classification |
| 5 | **Rivers** | D8 flow direction, flow accumulation, rivers, lakes |
| 6 | **Soil & Fertility** | 11 soil types, fertility scoring, fisheries |
| 7 | **Settlements** | Habitability scoring → city placement |
| 8 | **Biological-Trade** | Shark + shipworm risk, 21 trade-good belts, trade routes & trade matrix |
| 9 | **Political** | Settlements re-ranked by trade power; influence regions |

**Run All** generates phases 1–8 from plates; **Complete from Landmass** runs phases 2–8 while keeping an imported/painted coastline.

A taste of the science baked in:

- **Temperature** — latitude base curve + elevation lapse rate + warm/cold current influence decaying inland.
- **Precipitation** — downwind moisture advection, ITCZ boost near the equator, orographic lift and rain shadows, mid-latitude frontal storm tracks, subtropical-high and cold-coast drying.
- **Salinity** — `S ≈ 35 + (evaporation − ocean_precip)·k`, freshened by coastal runoff and concentrated in enclosed warm seas, then advected poleward by boundary currents.
- **Trade goods** — some goods (wheat, iron, timber, salt, furs, stockfish, whaling) appear in every suitable cell; most are seeded to a single suitability-weighted "homeland" and flood-filled; gemstones are scattered as discrete highland-locked deposits.

See [`CLAUDE.md`](./CLAUDE.md) for the full architecture reference and exact formulas.

---

## Getting Started

### Prerequisites
- [Node.js](https://nodejs.org) (18+)
- [Rust toolchain](https://rustup.rs) (with the MSVC toolchain on Windows)
- The platform [Tauri prerequisites](https://tauri.app/start/prerequisites/) (WebView2 on Windows, etc.)

### Run in development

```bash
npm install
npm run tauri dev
```

On Windows you can instead double-click **`run.bat`**, which checks for Node/Rust, installs dependencies, and launches dev mode.

> **Note (Windows):** Rust commands (`cargo check`, builds) should be run from **cmd or PowerShell**, not Git Bash — the MSVC linker isn't on the Git Bash `PATH`.

### Type-check only

```bash
cargo check            # from src-tauri/  — Rust
npx tsc --noEmit       # TypeScript
```

### Build a release bundle

```bash
npm run tauri build
```

---

## Using the App

The window is laid out as **Workflow (left) · Map (center) · Toolbar (right) · Status bar (bottom)**.

1. **Create a world** — choose a size, then either generate plates or **import a template image** (black/white land–sea mask) or **paint** your own land with the brush tools.
2. **Walk the workflow panel top to bottom** — each step runs one simulation phase and checks that its prerequisites are done, or hit **Run All / Complete from Landmass**.
3. **Switch layers and overlays** from the Toolbar — elevation, climate, biomes, soil, fertility, currents, salinity, shark/shipworm hazards, per-good trade belts, trade routes, and the political layer.
4. **Inspect** any cell by right-clicking it to open the info panel (elevation, temperature, climate, salinity, goods, hazards, …).
5. **Export** a 16-bit heightmap or individual layers as PNGs.

Worlds are saved as self-contained **`.worldforge`** files (SQLite databases) and can be reopened later.

> Some edits require re-running upstream phases. In particular, changes to salinity or currents mean re-running **Ocean & Atmosphere then Climate**, since temperature and current data are baked into the tiles.

---

## Project Structure

```
worldforge2/
├── src/                       React + PixiJS frontend
│   ├── bridge/tauri.ts        IPC wrappers around Rust commands
│   ├── canvas/                TileViewport, TileManager, OverlayManager, PixiApp
│   ├── state/                 Zustand stores (world / ui / viewport)
│   ├── ui/                    App shell, MapCanvas, Toolbar, InfoPanel, panels
│   │   └── workflow/          Step1…Step9 wizard panels
│   ├── types.ts               Shared TypeScript types
│   └── goods.ts               Trade-good definitions
├── src-tauri/                 Rust backend
│   └── src/
│       ├── lib.rs             Tauri entry, command registration
│       ├── db/                SQLite schema + zstd tile store
│       ├── tile/              TileData columns, coordinate math
│       ├── render/            tile_image.rs — 18 render layers
│       ├── paint/             Brush + paint strokes
│       ├── sim/               world_buffer + per-phase simulation modules
│       ├── commands/          Tauri command handlers (sim / query / file / template)
│       └── history/           Tile-level undo/redo journal
├── CLAUDE.md                  Full architecture + science reference
├── package.json
└── run.bat                    Windows dev launcher
```

---

## Related

- **WorldForge 1** — the original browser-based version this project supersedes.

## License

No license has been specified yet. All rights reserved by the author until one is added.
