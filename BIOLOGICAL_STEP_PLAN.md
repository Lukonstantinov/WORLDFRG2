# WorldForge 2 — Salinity + Biological Step: Implementation Plan

Status: **PLAN (not yet implemented)**. Authored 2026-06-02.

Decisions locked with the user:
- Salinity = **moderate thermohaline**, coupling into **currents + climate** (re-run Ocean → Climate).
- Salinity folded into the existing **Step 3 (Ocean & Atmosphere)**; one new **Step 8 (Biological)**.
- Shark zones = **habitat-based** (bull/tiger-shark warm shallow water + frequented coasts), independent of people.
- Trade goods = all four groups, **separate sublayers**, spanning **land + marine** cells.
- Trade matrix = **settlement-cluster regions**, region↔region flows, **panel table + flow lines**.
- Storage = **persist everything as `u8` columns**; **clear `target/` cache before building** (disk ~97% full — last session's ENOSPC truncated `elevation.rs`).

---

## 0. Disk / build safety (DO FIRST)

- C: is ~97% full (16 GB free). `src-tauri/target` was ~16 GB last session and an ENOSPC mid-write **truncated `elevation.rs` to empty** (worldforge2 is **not** under git → no restore).
- Before any `cargo` build: delete `src-tauri/target/debug` to reclaim ~15 GB.
- Each persisted `u8` field ≈ 6.5 MB RAM at default 3600×1800, ×4 at Large 7200×3600. We add ~17 new `u8` columns → ~+110 MB at default, ~+440 MB at Large. zstd compresses the sparse belt fields well on disk. Acceptable per "persist everything"; flagged here so it can be revisited.
- Save-file back-compat: **append every new column LAST** in `compress`/`decompress` (existing pattern — trailing reads pad with zeros, so old `.worldforge` saves still load with the new fields = 0).

---

## 1. Ocean Salinity (Step 3 enhancement)

### 1.1 New persisted field
- `TileData` + `WorldBuffer`: `salinity: Vec<u8>` (0..255 ↔ ~28–42 PSU). Append last in `new_sea`, `compress`, `decompress`, `load`, `save`.

### 1.2 Physics (`sim/ocean.rs` new fns)
Surface salinity drivers (ocean cells only):
- **Base** 35 PSU.
- **Evaporation E** ∝ SST estimate × wind speed. SST estimate uses a **latitude curve** (NOT the temperature field — avoids the salinity↔temperature circular dependency, since temperature is computed *after* currents). Wind speed = |wind_vx,wind_vy| from `compute_wind_belts`.
- **Ocean precipitation P** = latitude-band function (ITCZ peak ≈ equator, mid-lat storm-track secondary max ≈ 50°, subtropical minima ≈ 20–30°). Reuse the belt shapes from `precipitation.rs`.
- **E − P** drives salinity up (subtropics: salty) or down (equator + high lat: fresh).
- **Runoff freshening**: coastal sea cells adjacent to high-`precipitation` land are freshened (BFS plume, like `fertility::compute_fisheries` nutrient plume). Uses land precip as a river proxy so salinity stays **self-contained in Step 3** (real rivers are Step 5).
- **Enclosed-sea concentration**: narrow warm seas (Red Sea / Med / Gulf) become hypersaline — reuse `compute_enclosed_suppression` narrowness logic already in `precipitation.rs`.
- Clamp ~28–42 PSU → normalize → `u8`.

`compute_salinity(buf)` writes `buf.salinity`.

### 1.3 Density + sinking (transient, not stored)
- `water_density_index = a*salinity_norm − b*sst_norm` (cold + salty = dense).
- **Deep-water formation / sinking zones**: high latitude + dense surface = sinking (North-Atlantic-Drift terminus / Antarctic). Returned as a transient `Vec<f32>`/`Vec<bool>` for the current step (not persisted — keeps field count down).

### 1.4 Coupling into currents (moderate thermohaline)
Modify `generate_ocean_currents` to accept salinity/density:
- **Boundary-current strengthening**: scale western-boundary speed by the downstream surface-density contrast (salty subtropical source feeding a poleward current ⇒ stronger Gulf-Stream-analog).
- **Conveyor pull / extended warm advection**: the Phase-5 downstream warm-tag advection reaches **further** where the terminus has a strong sinking zone (the overturning "pull"). This is the mechanism that, per the user's "currents + climate" choice, makes high-latitude downstream coasts milder.

### 1.5 Step-3 ordering (`commands/sim_commands.rs::sim_ocean_atmosphere` + both run-all fns)
```
1. compute_wind_belts
2. compute_salinity            (wind + latitude SST estimate + runoff proxy + enclosed seas)
3. (density + sinking, transient)
4. generate_ocean_currents     (now salinity/density-aware)
5. compute_distance_to_ocean
6. compute_temperature         (stronger/longer currents ⇒ milder downstream coasts = climate feedback)
7. compute_upwelling_zones
8. compute_precipitation
```
Single ocean pass; gives currents+climate coupling without re-solving twice.

### 1.6 Render + UI
- `render/tile_image.rs`: new `render_salinity` (sea ramp e.g. teal→blue→violet→white for hypersaline; land transparent/dim). Add `"salinity" =>` arm.
- `types.ts` `ActiveLayer` += `"salinity"`; Toolbar **Ocean** group += Salinity.
- `query_commands::get_cell_info` + `CellInfo` + `InfoPanel` += salinity (PSU).
- Optional standalone `sim_compute_salinity` command for re-running just salinity (low priority).

**Re-run note for users:** salinity/current changes require **Ocean & Atmosphere THEN Climate** re-run (temps + current_type bake into tiles).

---

## 2. Biological Step (new Step 8)

### 2.1 Workflow plumbing
- `types.ts`: `WorkflowStep = 1|2|3|4|5|6|7|8`.
- `uiStore.ts`: `STEP_DEFAULTS[8]` (e.g. layer `"land"`, tool `"select"`); add overlay-visibility keys (below).
- `WorkflowPanel.tsx`: `STEP_INFO` += `{ step: 8, label: "Biological", desc: "Shark waters, trade-good belts, and the regional trade matrix." }`; `goNext`/`goBack` bounds 1..8; the `for (i=1..7)`/`(i=2..7)` completion loops in Run-All → `..8`; render `<StepBiological/>`; optionally call the new biological command at the end of `simRunAll`/`simRunAllFromTerrain`.
- New `ui/workflow/StepBiological.tsx` (prereqs: Step 6 fertility/fishery + Step 7 settlements done).

### 2.2 Persisted fields (`u8`)
Append last (after `salinity`), in order, to `TileData`/`WorldBuffer` everywhere:
- `shark_risk: u8`
- One field per good (intensity 0..255). Goods (separate sublayers):
  - **Named five**: `good_silk`, `good_wine`, `good_oliveoil`, `good_sugar`, `good_frankincense`, `good_stockfish`
  - **Spices/tea/coffee**: `good_spices`, `good_tea`, `good_coffee`
  - **Furs/timber/amber**: `good_furs`, `good_timber`, `good_amber`
  - **Salt/dyes/incense**: `good_salt`, `good_dyes`, `good_incense`
- Total new biological fields: 1 (shark) + 15 (goods) = 16. (Implementation note: consider a Rust `const GOODS: &[(name, fn)]` table + a macro/helper to cut the per-field boilerplate in compress/decompress/load/save; the existing code is explicit per field, so weigh consistency vs. churn.)

### 2.3 Shark waters (`sim/biological.rs::compute_shark_risk`)
Habitat model (sea cells), people-independent:
- Warm/temperate SST band (bull + tiger sharks → warm tropical–subtropical strongest; taper into temperate).
- Shallow water: `is_shelf` and/or low `sea_depth` (photic, coastal).
- "Frequented coasts": proximity to coastline (BFS from land, decaying).
- River-mouth / brackish bonus (bull sharks): plume from river mouths (rivers passed in; Step 5 done).
- Optional prey proxy: blend a little `fishery`.
- Output 0..1 → `shark_risk` u8.
- Render layer `render_shark` (sea heatmap). `ActiveLayer += "shark"`; Toolbar Ocean group += "Shark Waters".

### 2.4 Trade-good belts (`sim/biological.rs::compute_trade_goods`)
Each good = scoring fn over `koppen`, `temperature`, `precipitation`, `elevation`, `fertility`/`soil_type`, coast/`distance_to_ocean`, `sea_depth`/`is_shelf`, `fishery`, `salinity`. Output 0..1 → u8. Sketch of rules (tunable):
- **Silk (sericulture)** — humid subtropical/temperate (Cfa/Cwa/Cfb), mild winters, moderate elevation, fertile (mulberry).
- **Wine** — Mediterranean (Csa/Csb) + warm-temperate hill margins; specific temp/precip window.
- **Olive oil** — Mediterranean (Csa/Csb), frost-limited (mild winters), coastal-ish.
- **Sugar (saccharin → cane)** — tropical wet (Af/Am/Aw), hot + wet + lowland + fertile.
- **Frankincense/myrrh** — hot arid/semi-arid subtropical (BWh/BSh), dry escarpment/coast (Arabian/Horn analog).
- **Stockfish & salt-cod (marine)** — cold productive shelf: high `fishery` + cold SST + near coast (Lofoten analog).
- **Spices** — tropical monsoon coasts/islands (Am/Aw, coastal/archipelago, hot wet).
- **Tea** — tropical/subtropical highland (Cwb/Cwa + elevation, humid).
- **Coffee** — tropical highland (Aw/Cwb + mid elevation, humid).
- **Furs** — boreal/taiga & tundra margin (Dfc/Dfd/Dwc/ET), cold forest.
- **Timber / naval stores** — boreal/temperate forest (Dfb/Dfc/Cfb), forested + fertile.
- **Amber (marine/coastal)** — cold-temperate coasts (Cfb/Dfb coast; Baltic analog).
- **Salt (land + marine)** — arid coasts (BWh/BSh coastal, high evaporation) + hypersaline enclosed seas (`salinity` high) + endorheic salt lakes.
- **Dyes (murex, marine/coastal)** — warm subtropical coasts.
- **Incense / aromatics** — arid/semi-arid (overlaps frankincense by design; kept distinct to honor the group).

### 2.5 Command + wiring
- `commands/sim_commands.rs::sim_biological(rivers_json, db)` → load buf, `compute_shark_risk`, `compute_trade_goods`, save. (Settlements not needed to *compute* fields — only the matrix needs them.)
- Register in `lib.rs`. Bridge `simBiological(riversJson)` in `bridge/tauri.ts`.
- Optionally call inside `sim_run_all` / `sim_run_all_from_terrain` as Phase 8.

### 2.6 Visualization — "regions marked + emoji inside"
Per the user: outlined/tinted **regions** with **emoji** glyphs inside (Tauri WebView2 = Chromium → `ctx.fillText` emoji works).
- **Shark zones**: query `compute_shark_zones()` clusters high `shark_risk` shelf cells into regions (coarse-grid flood-fill like `compute_fishery_banks`). `OverlayManager.drawSharkZones` → translucent region + 🦈 at centroid. Overlay toggle `sharkZones`.
- **Trade-good regions**: query `compute_good_regions()` returns, per good, clustered regions `{good, x, y, radius, score}`. `OverlayManager.drawGoodRegions` → faint tinted region + the good's emoji at centroid. Emoji map: silk 🧵, wine 🍷, olive oil 🫒, sugar 🍬, frankincense 🪔, stockfish 🐟, spices 🌶️, tea 🍵, coffee ☕, furs 🦊, timber 🪵, amber 🟠, salt 🧂, dyes 🐚, incense 💨. Toggle per good (a collapsible **Goods** subsection in Toolbar overlays) so each is its own sublayer.

### 2.7 Trade matrix (settlement-cluster regions)
- Query `compute_trade_matrix(settlements_json)`:
  1. **Cluster** top-N settlements into regions by proximity (wrap-aware, e.g. grid-bucket or simple distance merge).
  2. Per region: **production** of each good = aggregate good intensity over the region's territory (cells near its settlements); **demand** = population-weighted baseline + goods the region lacks.
  3. **Net balance** per good per region → match exporters↔importers → **flows** `{from, to, good, weight}`.
  4. Return `{ regions:[{id,name,x,y,production[],demand[]}], flows:[...], goods:[names] }`.
- **Display**:
  - `TradeMatrixPanel.tsx` — table (regions × net export/import per good), toggleable.
  - `OverlayManager.drawTradeFlows` — weighted lines between region centers (thickness ∝ volume, seam-broken like existing routes). v1 = straight region-center links; v2 (optional) routes them through the existing `compute_trade_routes` cost grid.
- Overlay toggle `tradeFlows`. MapCanvas fetches on settlements/tileVersion change.

---

## 3. File-by-file change list

**Rust**
- `sim/world_buffer.rs` — +`salinity` +`shark_risk` +15 good fields (struct, `load`, `save`).
- `tile/cell.rs` — same fields in `TileData` (`new_sea`, `compress`, `decompress`).
- `sim/ocean.rs` — `compute_salinity`, density/sinking helper; make `generate_ocean_currents` salinity-aware; extend Phase-5 advection by sinking pull.
- `sim/biological.rs` (NEW) — `compute_shark_risk`, `compute_trade_goods` (+ per-good scoring), good name table.
- `sim/mod.rs` — `pub mod biological;`.
- `commands/sim_commands.rs` — new Step-3 ordering; `sim_biological`; add Phase 8 to both run-all fns; (optional `sim_compute_salinity`).
- `commands/query_commands.rs` — `compute_shark_zones`, `compute_good_regions`, `compute_trade_matrix`; add `salinity`/`shark_risk`/goods to `CellInfo` + `get_cell_info`.
- `render/tile_image.rs` — `render_salinity`, `render_shark`; match arms.
- `lib.rs` — register all new commands.

**Frontend**
- `types.ts` — `WorkflowStep` 1..8; `ActiveLayer` += `salinity`,`shark`; new types `SharkZone`, `GoodRegion`, `TradeRegion`, `TradeFlow`, `TradeMatrix`; `CellInfo` += salinity/shark/goods.
- `bridge/tauri.ts` — `simBiological`, `computeSharkZones`, `computeGoodRegions`, `computeTradeMatrix` (+ optional `simComputeSalinity`).
- `state/uiStore.ts` — `STEP_DEFAULTS[8]`; overlay keys `sharkZones`, `tradeFlows`, per-good `good_*`.
- `ui/Toolbar.tsx` — Ocean group += Salinity, Shark Waters layers; Overlays += Shark Zones, Trade Flows, **Goods** subsection (per-good toggles).
- `ui/workflow/WorkflowPanel.tsx` — Step 8 entry, nav bounds, run-all loops, render StepBiological, optional biological in run-all.
- `ui/workflow/StepBiological.tsx` (NEW) — generate button (calls `simBiological`), layer/overlay shortcuts, opens TradeMatrixPanel.
- `ui/TradeMatrixPanel.tsx` (NEW) — matrix table.
- `canvas/OverlayManager.ts` — `drawSharkZones`, `drawGoodRegions`, `drawTradeFlows`, emoji map, `clear()` updates.
- `ui/MapCanvas.tsx` — fetch shark zones / good regions / trade matrix on version changes; wire visibility.
- `ui/InfoPanel.tsx` — show salinity, shark risk, goods present.

**Docs**
- `CLAUDE.md` — new field list, Step 3 reorder, Step 8, salinity/shark/good science formulas, render layers (15 → 17), re-run order. (`PORTING_REFERENCE.md` stays stale.)

---

## 4. Re-run order (for the user, after build)
- Salinity/currents/climate changes → re-run **Ocean & Atmosphere THEN Climate**.
- Biological fields (sharks, goods) → run **Biological (Step 8)** (needs Rivers + Soil/Fertility + Settlements done).
- Trade matrix/flows → recompute when settlements or goods change.

## 5. Risks / watch-items
- **Disk**: clear `target/` first; 16 new `u8` fields add ~110 MB RAM at default / ~440 MB at Large.
- **Circular dep** salinity↔temperature → resolved via latitude SST estimate inside `compute_salinity`.
- **Boilerplate**: 16 fields × 4 sites (new_sea/compress/decompress/load+save) — mechanical; consider a helper/macro.
- **Tuning**: good belts + shark habitat need visual iteration on screenshots (compiling ≠ correct).
- **Back-compat**: append-last keeps old saves loadable (fields read as 0).
- **Verify**: `cargo check` (from cmd/PowerShell, not git bash) + `npx tsc --noEmit`, then `npm run tauri dev` and eyeball.
