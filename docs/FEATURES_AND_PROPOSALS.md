# WorldForge 2 — Features, Generation Pipeline & Improvement Proposals

A single reference for **what the app does today**, the **step-by-step world
generation**, concrete **per-step improvement ideas**, and a design discussion on
**how the campaign could be integrated differently**.

---

## Part 1 — Feature Catalog

### 1.1 Procedural world generation
- **Tectonics → climate → hydrology → life → trade** pipeline (10 phases, see Part 2).
- Two entry paths: **from plates** ("Generate Full World") or **from a painted /
  imported landmass** ("Complete from Landmass", distance-from-coast elevation).
- Deterministic per seed; every phase checks its prerequisites.

### 1.2 Editing & painting
- Paint tools: **Land (0/1), Elevation (f32), Shelf, Volcano**, plus a **draw-a-ridge**
  terrain tool (sketch a ridge line → eroded mountain range).
- **Tile-level undo/redo** — every stroke and phase journals prior tile state.
- **Import a template image** (4-bit quantization → land/sea auto-detection) and
  **import layer groups** from another world of the same grid size.

### 1.3 World data & rendering
- 128×128 tiles, **cylindrical topology** (X wraps, Y clamps), 20+ columnar fields
  per tile, zstd-compressed v2 self-describing blobs, **LOD 1–4 supertile pyramid**.
- **Server-side rendering**: Rust renders 19 RGBA layers; the frontend only displays.
- **~30 vector overlays** (rivers, settlements, wind, currents, trade routes/trunks,
  good belts, hazard zones, culture regions, dynamic trade flow…), each toggle-gated.

### 1.4 Trade goods & market economy (world)
- **45 trade goods** with categories/tiers/value/bulk/perishability and **production
  chains** (recipe DAG, labor ∝ population).
- Belts: **unlimited / seeded / gemstone-deposit / manufactured** distributions.
- **Market equilibrium solver** (stock-based prices in a grain-equivalent numeraire,
  15-category substitution, arbitrage with freight, spatial price gradients).
- Trade cartography: least-cost **routes, matrix, bundled trunks**, chokepoints,
  travel-time itineraries.

### 1.5 The living campaign (on top of a finalized world)
Merchant houses, guilds, dynasties; estates & manufactories; **banks, coinage,
credit, financial crashes**; the **polis** (councils, tariffs, mints, treasuries);
**wars, blockades, reparations**; **colonies & migration corridors**; **epidemics &
starvation**; **city lifecycle** (founding, growth, absorption, unrest, revolts);
**satellite (suburb) construction**; futures contracts & warehouses; government &
key figures. Advanced one **day** at a time, deterministic per `(seed, tick)`.

### 1.6 Reading matter & analysis
Chronicle / news feed / year-grouped histories; economy dashboard (CPI, Gini,
mobility); city rankings; heraldry (coats of arms, minted coins); a goods codex with
real-world commodity history.

---

## Part 2 — Step-by-Step World Generation

| # | Phase (command) | Input → Output | Core method |
|---|-----------------|----------------|-------------|
| 1 | **Plates** (`sim_generate_plates`) | seed → plates, boundaries, land/sea | Voronoi plate tectonics |
| 2 | **Terrain** (`sim_generate_terrain` / `_from_template`) | plates *or* land shape → elevation | boundary stress / distance-from-coast |
| 2b | **Shelves** (`sim_generate_shelves`) | coast → continental shelf | configurable shelf width |
| 3 | **Ocean & Atmosphere** (`sim_ocean_atmosphere`) | elevation+lat → winds, **salinity**, currents, temperature, jets, precip | wind belts → thermohaline → moisture advection |
| 4 | **Climate** (`sim_classify_climate`) | temp+precip → 22 Köppen zones | Köppen thresholds + current overrides |
| 5 | **Rivers & Hydrology** (`sim_rivers_hydrology`) | elevation → rivers, lakes | D8 flow accumulation |
| 6 | **Soil & Fertility** (`sim_soil_fertility`) | climate → 11 soils → fertility, fisheries | soil model + weighted fertility |
| 7 | **Settlements** (`sim_generate_settlements`) | habitability → city placement | H = climate·0.40 + fertility·0.20 + water·0.20 + terrain·0.10 + trade·0.10 |
| 8 | **Biological & Goods** (`sim_biological`) | seed → shark/shipworm risk, good belts, deposits | score envelopes + flood-fill + ore-province noise |
| 9 | **Political** (`compute_political`) | settlements → trade-power ranking, influence discs | 0.45·habitability + 0.30·centrality + 0.25·monopoly |
| 10 | **Economy** (`compute_economy`) | hubs → market equilibrium, prices, wealth | stock-based grain-eq solver |

**Extras:** ridged terrain, elevation scaling/inversion, toponyms (#26), hydrology/
biology refresh. **Run-alls:** `sim_run_all` (1→8 from plates), `sim_run_all_from_terrain`
(2alt→8 keeping the landmass). **Finalize World** freezes tiles + metadata; the
campaign then lives in a separate `.campaign` file.

---

## Part 3 — Proposed Improvements, Step by Step

Grouped as **quick wins** (localized, low-risk) vs **deeper** (new data/passes).

### Phase 1 — Plates
- *Quick:* give each plate a **velocity vector**; classify boundaries as
  **convergent / divergent / transform** from relative motion so Phase 2 can raise
  arcs at convergence, rift valleys at divergence, and offset ridges at transforms.
- *Deeper:* **hotspots / mantle plumes** producing age-progressive island chains
  (Hawaii-style); **microplates** for realistic coastline complexity.

### Phase 2 — Terrain / Elevation
- *Quick:* **isostatic smoothing** so thick crust floats higher; taper elevation into
  shelves rather than a hard coast step.
- *Deeper:* a **hydraulic + thermal erosion** pass (droplet or stream-power) so
  valleys, alluvial fans and drainage divides are physically consistent with Phase 5
  rivers — the single biggest realism upgrade for the whole map.

### Phase 3 — Ocean & Atmosphere
- *Quick:* replace the two-cell wind model with a proper **three-cell (Hadley /
  Ferrel / Polar)** structure and explicit **subtropical highs** → sharper deserts at
  ~30°.
- *Deeper:* derive **ocean gyres from wind-stress curl** (not just belts); add
  **interannual variability (ENSO-like)** the campaign can read for good/bad years.

### Phase 4 — Climate
- *Quick:* add **highland (H) climates** gated on elevation; tighten the **Cs/Cw**
  (Mediterranean vs monsoon) split.
- *Deeper:* **microclimate** blending across coast/valley/rain-shadow gradients rather
  than per-cell hard classification.

### Phase 5 — Rivers & Hydrology
- *Quick:* carve **lake outlets** and tag each cell with a **watershed/basin ID**
  (useful for polities, trade, and naming).
- *Deeper:* **D-∞ / multiple-flow-direction** routing for **deltas, distributaries and
  braided reaches**; couple to the Phase 2 erosion pass so channels match the terrain.

### Phase 6 — Soil & Fertility
- *Quick:* boost **floodplain alluvium** near large rivers; penalize **salinized** soil
  near evaporitic/enclosed basins (salinity is already computed in Phase 3).
- *Deeper:* let **parent geology** (from plate/volcanic provenance) shape soil, so
  volcanic and karst regions read distinctly.

### Phase 7 — Settlements
- *Quick:* add **defensibility** (chokepoints, river confluences, peninsulas) and a
  **coast/confluence bonus** to habitability.
- *Deeper:* a **central-place / gravity model** so settlements compete for a hinterland
  and space out into a realistic size hierarchy instead of clustering on the best cells.

### Phase 8 — Biological & Goods
- *Quick:* tie **ore genesis to tectonic setting** (porphyry copper & gold at
  convergent arcs, tin/rare metals at rifts, placer gold below eroding highlands).
- *Deeper:* biome-linked **species pools** for fauna/flora goods; make key resources
  **finite & depletable** so the campaign can exhaust a mine or overfish a bank.

### Phase 9 — Political
- *Deeper:* grow **actual polities** (borders, city-states vs empires, vassalage) from
  the influence field, rather than only ranking + discs — this gives the campaign real
  starting states.

### Phase 10 — Economy
- *Quick:* per-good **price elasticity** instead of a single exponent.
- *Deeper:* model the **transport network explicitly** (roads/rivers/sea lanes as a
  graph with capacity) so chokepoints and infrastructure matter to prices.

---

## Part 4 — Should the Campaign Be Added Differently?

### 4.1 How it works today
A `CampaignSim` is **seeded once** from a static economy snapshot when the world is
**finalized (frozen)**, then advanced **one day at a time**. It is pure and
deterministic per `(seed, tick)`: **no DB, no global RNG, and — importantly — no tile
access**. A tick is **hub-level aggregate math**; the route-days matrix is derived on
load. It persists to a separate **`.campaign`** file, fingerprint-checked against its
world.

**Strengths worth keeping:** determinism (replayable, testable — the standing dynamics
test depends on it), performance (hub-level math scales), and a clean **frozen-world /
mutable-campaign** contract that keeps saves stable.

### 4.2 The core limitation
The campaign is **aspatial**. Wars, colonies, plague, migration and trade are computed
between *hubs* with a precomputed distance matrix, but they never touch the actual map:
a plague can't sweep down a river valley, a blockade can't close a specific strait, and
the world never changes in response (no deforestation, urban sprawl, resource
depletion, or climate drift). The world is a static backdrop, not a participant.

### 4.3 Options (recommended first)

1. **Coarse campaign field layer (recommended).** Add a low-resolution spatial grid
   (e.g. downsampled from tiles at load, held in memory, *not* in `tick.rs`'s
   serialized state) that the sim can read/write for **plague spread, blockade
   geography, migration corridors and frontier expansion**. Keeps determinism and the
   frozen base world, but makes the campaign *geographic*. Highest value-to-risk.

2. **Bounded world feedback.** Let the campaign write back a **small, additive overlay**
   (cleared land, city footprints, depleted deposits) rendered on top of the frozen
   tiles — without mutating the immutable base. Preserves the freeze contract while
   letting the map visibly evolve over a campaign.

3. **Optional agent granularity.** Keep the hub-level core, but allow a handful of
   **named agents** (a famous merchant, an admiral) to be simulated at finer grain for
   narrative texture, layered over the aggregate math.

4. **Event-sourced timeline.** Store the campaign as an **append-only event log** rather
   than only snapshots, enabling replay, save-scumming-free branching, and richer
   chronicles — a natural fit for the already-deterministic tick.

5. **Earlier / first-class integration.** Promote the campaign to an explicit
   **"Phase 11"** in the workflow with a dedicated seeding UI (starting polities, era,
   scenario), instead of it feeling bolted on after finalize.

**Recommendation:** pursue **#1 (coarse field layer)** and **#2 (bounded write-back
overlay)** together — they address the aspatial limitation, keep determinism and
performance, and respect the frozen-world contract, while making the living economy
actually inhabit its geography.
