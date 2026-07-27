# WorldForge 2

**A desktop fantasy-world map generator that simulates a planet from tectonics to a living trade economy.**

WorldForge 2 is a [Tauri](https://tauri.app/) desktop application that procedurally generates plausible, science-grounded world maps — and then lets a **living economy** play out on top of them. Starting from either tectonic plates or a hand-drawn/imported landmass, it runs a multi-stage physical simulation (elevation, ocean currents, salinity, climate, rivers, soils, settlements, trade goods), solves a market economy, and can then **advance time** as merchant houses, banks, coinage, wars and crashes shape the world. Everything renders as a fully zoomable, layered map.

It is a ground-up rewrite of the original browser-based WorldForge. The browser version hit a hard memory ceiling (~1.4 GB) and crashed on large grids; moving the simulation into a native Rust backend removes that limit, so WorldForge 2 can simulate worlds at 3600×1800 and beyond.

> **Status:** Active development, broadly feature-complete. The full world-generation pipeline, the market economy, and the **campaign tick simulation** ("Living Trade" and its finance/coin/credit layers) are implemented; the backend compiles cleanly and worlds generate and play end-to-end. Ongoing work focuses on tuning plausibility (climate, currents) and the depth of the economic dynamics.

---

## Highlights

### World generation
- **Whole-planet simulation, in order.** Dependent phases turn bare land into a living world — tectonics → elevation → ocean & atmosphere → climate → rivers → soil & fertility → settlements → biology & trade → politics → market economy.
- **Two ways to start.** Generate everything from procedural **plate tectonics**, or **import an image / paint** your own continents and let the simulation fill in everything else from the coastline.
- **Real climatology.** A 1-D diffusive **energy-balance model** (North–Budyko) with ice-albedo feedback, driven by the true **astronomical insolation** integral, so a world's axial tilt, rotation rate, stellar luminosity and greenhouse all move the climate for physically correct reasons. Circulation belts derive from the rotation rate; gyre interiors from the **Sverdrup** relation. Plus wind belts, ITCZ migration, orographic rain shadows, boundary currents, upwelling, low-level jets, a thermohaline "conveyor", and a full **Köppen** classification (31 zone codes + highland).
- **Measured against the real Earth.** The climate pipeline is scored against the canonical **Köppen-Geiger** reference map (Kottek & Rubel, 0.5°) by an in-repo validation harness, area-weighted by latitude, with a CI regression floor — so climate changes are judged by a number, not by eye.
- **Oceanography.** Sea-surface **salinity** (evaporation − precipitation, runoff freshening, enclosed-sea concentration) is advected along currents and coupled back into current strength.
- **Hazards & biology.** Shark and shipworm risk maps, plus storm, reef and disease fields, all derived from climate, depth, salinity and currents.

### Economy & the living campaign
- **A real market.** A stock-based **equilibrium solver** prices every good in a grain-equivalent numeraire with category substitution, freight (bulk + perishability), arbitrage and decaying spatial price gradients — no hand-waved per-hop markups.
- **45 trade goods with production chains.** Climate/terrain-distributed belts plus discrete mineral **deposits**, plus ~21 **manufactured** goods made in cities from recipes (e.g. wool + dyes → cloth) via a shared production-chain resolver.
- **Living Trade campaign.** Press play and time advances day by day: cities produce, consume and trade; **merchant houses** rise and go **defunct**; **banks** are chartered and **fail**; **poleis** set tariffs and **mint coin** (with seigniorage, debasement and coin-trust); **wars**, blockades and forced levies flare; and regional **crashes** ripple through credit. A year-grouped **chronicle** records it all.
- **World ↔ Campaign split.** The generated world (geography, climate, goods spec) is **finalized** and frozen into a self-contained `.worldforge` file; everything human (settlements, economy, houses, the clock) lives in a separate `.campaign` file played on top.

### Creator tools (this branch)
- **🧭 Itinerary / travel-time.** Pick two settlements and get a realistic journey time by foot / horse / cart (water legs by boat/ship) over the same least-cost grid trade uses, with the route drawn on the map.
- **📖 Goods Codex.** Per good: a **provenance** tracer (source → … → consumer routes and recipe inputs), a **real-world commodity-history** card (silk road, spice trade, salt-as-money…), and a **scarcity** overlay colouring cities cheap→dear.
- **📊 Economy Dashboard.** A basket **cost-of-living price index** comparing cities, and a wealth-**inequality** read (Gini, top-10% share, house turnover and a social-mobility score) from the live campaign.
- **🗺️ Geographic Toponyms.** Optionally name rivers, mountains, lakes and regions in each area's local culture style — then rename anything by hand; names persist with the world.

### Rendering & export
- **Layered, tiled rendering.** 18+ render layers (elevation, climate, biomes, soil, fertility, currents, salinity, hazards, …) drawn server-side and streamed as tiles to a PixiJS canvas, with an LOD pyramid for fast zoom, plus vector overlays for rivers, settlements, wind, currents, trade routes/flows, regions, and the creator overlays above.
- **Export.** 16-bit grayscale heightmap PNGs and per-layer image exports for use in other tools.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | [Tauri 2](https://tauri.app/) (Rust) |
| Backend | Rust — simulation, market/campaign sim, rendering, storage |
| Storage | SQLite (`rusqlite`, bundled) with zstd-compressed tile blobs; `.worldforge` + `.campaign` files |
| Frontend | React 18 + [PixiJS 8](https://pixijs.com/) + [Zustand](https://github.com/pmndrs/zustand) |
| Build | Vite 5 + TypeScript 5 |

---

## How It Works

### Tile architecture
The world is divided into **128×128-cell tiles** with cylindrical topology (the X axis wraps around the globe; the Y axis clamps at the poles). Each tile stores ~20 columnar per-cell fields (terrain, elevation, temperature, precipitation, Köppen zone, salinity, shark/shipworm risk, trade-good belts, storm/reef/disease, …) as a single zstd-compressed blob in SQLite. Blobs are self-describing and new fields are appended at the end, so older `.worldforge` saves keep loading. A persisted **LOD pyramid** (supertiles) keeps zoomed-out views fast.

### The WorldBuffer pattern
Simulation never mutates tiles cell-by-cell. Instead each phase:

```
load all tiles → flat WorldBuffer arrays → run the phase → write back to tiles
```

This keeps neighbour lookups (BFS flood-fills, flow accumulation, advection) simple and fast across tile boundaries. Each phase loads only the columns it touches (per-phase column masks), and `save()` merges back the untouched columns.

### Server-side rendering
Rust renders each map layer to RGBA pixels per tile and ships them to the frontend (packed binary, or base64 for compatibility), where PixiJS displays them as textures (LRU-cached, ~2000 tiles). The frontend is a pure viewer for tile imagery; all the heavy lifting is native. Vector data (rivers, settlements, wind/current arrows, trade routes, the creator overlays) is drawn as PixiJS Graphics overlays on top, never baked into the tiles.

```
UI action → bridge/tauri.ts (invoke) → commands/*.rs → sim/*.rs
  → WorldBuffer → SQLite tiles → tile_image.rs (render) → packed RGBA
  → TileManager.ts → PixiJS sprites
```

### The campaign tick
Once the world is finalized, the **campaign simulation** (`sim/tick.rs`) advances a day loop over a `CampaignSim` of hubs, houses, banks, contracts, poleis and wars — running production, consumption, pricing, dispatch, succession, coinage, banking and conflict. Read-only query commands expose snapshots (houses, finances, currencies, banks, crashes, wars, trade flows) to the panels.

---

## Simulation Pipeline

Phases run in order; each depends on the previous ones.

| # | Phase | Computes |
|---|-------|----------|
| 1 | **Plates** | Voronoi tectonic plates, boundaries, initial land/sea |
| 2 | **Elevation** | Elevation from plate boundaries (or from a template landmass via distance-to-coast) + continental shelves |
| 3 | **Ocean & Atmosphere** | Wind belts → **salinity** → ocean currents (salinity/density-coupled) → distance-to-ocean → temperature → upwelling → precipitation |
| 4 | **Climate** | Köppen classification (22+ zones) |
| 5 | **Rivers** | D8 flow direction, flow accumulation, rivers, lakes |
| 6 | **Soil & Fertility** | 11 soil types, fertility scoring, fisheries |
| 7 | **Settlements** | Habitability scoring → city placement + organic culture map |
| 8 | **Biological-Trade** | Shark + shipworm risk, trade-good belts & deposits, trade routes & trade matrix |
| 9 | **Political** | Settlements re-ranked by trade power; influence regions |
| 10 | **Economy** | Market equilibrium: stock-based prices in grain-equivalent, barter ratios, currency goods, supply chains & chokepoints |
| 11 | **Living Trade** | Advance the campaign in time — production/consumption/trade, houses, banks, coin, wars, crashes; history accrues in the chronicle |
| — | **Toponyms** *(optional)* | Name rivers, mountains, lakes and regions in the local culture's style (gated on Rivers + Settlements; editable) |

**Run All** generates phases 1–8 from plates; **Complete from Landmass** runs phases 2–8 while keeping an imported/painted coastline. Steps 7+ unlock once the world is **finalized** (locked & saved).

A taste of the science baked in:

- **Temperature** — latitude base curve + elevation lapse rate + warm/cold current influence decaying inland.
- **Precipitation** — downwind moisture advection, ITCZ boost near the equator, orographic lift and rain shadows, mid-latitude frontal storm tracks, subtropical-high and cold-coast drying.
- **Salinity** — `S ≈ 35 + (evaporation − ocean_precip)·k`, freshened by coastal runoff and concentrated in enclosed warm seas, then advected poleward by boundary currents.
- **Trade goods** — some goods (wheat, iron, timber, salt, furs, stockfish, whaling) appear in every suitable cell; most are seeded to a single suitability-weighted "homeland" and flood-filled; gemstones/metals are scattered as discrete deposit blobs; manufactured goods are made in cities from recipes.

See [`CLAUDE.md`](./CLAUDE.md) for the full architecture reference and exact formulas.

---

## Getting Started

### Prerequisites
- [Node.js](https://nodejs.org) (18+)
- [Rust toolchain](https://rustup.rs) (with the MSVC toolchain on Windows)
- The platform [Tauri prerequisites](https://tauri.app/start/prerequisites/) (WebView2 on Windows; on headless Linux, `libgtk-3-dev` + `libwebkit2gtk-4.1-dev` are needed to compile).

### Run in development

```bash
npm install
npm run tauri dev
```

On Windows you can instead double-click **`run.bat`**, which checks for Node/Rust, installs dependencies, and launches dev mode.

> **Note (Windows):** Rust commands (`cargo check`, builds) should be run from **cmd or PowerShell**, not Git Bash — the MSVC linker isn't on the Git Bash `PATH`.

### Type-check & test

```bash
cargo check                                   # from src-tauri/ — Rust
npx tsc --noEmit                              # TypeScript
cargo test --lib tick::tests                  # campaign-sim unit + dynamics tests
cargo test --lib simulate_decades_reports_dynamics -- --nocapture   # watch the living economy
```

### Build a release bundle

```bash
npm run tauri build
```

---

## Using the App

The window is laid out as **Workflow (left) · Map (center) · Toolbar (right) · Status bar (bottom)**, with a floating **window bar** of toggles for the economy/finance panels.

1. **Create a world** — choose a size, then either generate plates or **import a template image** (land–sea mask) or **paint** your own land with the brush tools.
2. **Walk the workflow panel top to bottom** — each step runs one simulation phase and checks its prerequisites, or hit **Run All / Complete from Landmass**.
3. **Finalize the world** — once geography is done (step 6), **Lock & Save the map**; this freezes the world to a `.worldforge` file and unlocks the campaign steps (settlements, trade, politics, economy).
4. **Switch layers and overlays** from the Toolbar — elevation, climate, biomes, soil, fertility, currents, salinity, hazards, per-good trade belts, trade routes, the political layer, plus the creator overlays (travel route, good scarcity, toponym labels).
5. **Play the campaign** — advance time (Living Trade) and watch houses, banks, coinage, wars and crashes unfold; open the finance panels to inspect Houses, Coin & Credit, Speculation/Poleis, City finances and the World News chronicle.
6. **Use the creator tools** — 🧭 **Itinerary** (travel time between cities), 📖 **Goods Codex** (provenance, real-world history, scarcity), 📊 **Economy Dashboard** (price index + inequality), and the optional **Toponyms** step to name geography.
7. **Inspect** any cell by right-clicking it (elevation, temperature, climate, salinity, goods, hazards, …), and **export** a 16-bit heightmap or individual layers as PNGs.

Worlds are saved as **`.worldforge`** files and campaigns as separate **`.campaign`** files (both SQLite databases), and can be reopened later.

> Some edits require re-running upstream phases. In particular, changes to salinity or currents mean re-running **Ocean & Atmosphere then Climate**, since temperature and current data are baked into the tiles.

---

## Project Structure

```
worldforge2/
├── src/                       React + PixiJS frontend
│   ├── bridge/tauri.ts        IPC wrappers around Rust commands
│   ├── canvas/                TileViewport, TileManager, OverlayManager, PixiApp
│   ├── state/                 Zustand stores (world / campaign / ui / viewport / goods)
│   ├── ui/                    App shell, MapCanvas, Toolbar, InfoPanel, and panels:
│   │   │                        Houses, CoinCredit, Speculation, Hub, GoodsCodex,
│   │   │                        EconomyDashboard, Itinerary, CityRanking, …
│   │   └── workflow/          Step1…Step12 wizard panels (incl. optional Toponyms)
│   ├── commodityHistory.ts    Real-world commodity-history cards (Goods Codex)
│   ├── types.ts               Shared TypeScript types
│   └── goods.ts               Trade-good definitions
├── src-tauri/                 Rust backend
│   └── src/
│       ├── lib.rs             Tauri entry, command registration
│       ├── db/                SQLite schema + zstd tile store + metadata
│       ├── tile/              TileData columns, coordinate math, LOD
│       ├── render/            tile_image.rs — render layers
│       ├── paint/             Brush + paint strokes
│       ├── sim/               world_buffer + per-phase modules, market.rs,
│       │                        manufacture.rs, cultures.rs, names.rs, toponyms.rs,
│       │                        tick.rs (the campaign simulation)
│       ├── commands/          Tauri handlers (sim / query / campaign / goods / file)
│       └── history/           Tile-level undo/redo journal
├── docs/                      Design docs + HTML mockups (docs/mockups/)
├── CLAUDE.md                  Full architecture + science reference
├── package.json
└── run.bat                    Windows dev launcher
```

---

## Related

- **WorldForge 1** — the original browser-based version this project supersedes.

## License

No license has been specified yet. All rights reserved by the author until one is added.
