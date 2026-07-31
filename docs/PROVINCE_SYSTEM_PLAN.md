# Province System — Design & Implementation Plan

*An administrative layer between tiles and settlements: the land is partitioned into
provinces whose borders follow natural features (mountain crests, trunk rivers, coasts,
islands). Each province carries its own population, culture and goods, and interacts
with the settlements that stand in it — an Europa-Universalis-style political/economic
substrate grafted onto WorldForge 2's existing World + Campaign split.*

> **Note on "watershed".** This document was written around a drainage-divide
> partition seeded from `flow_dir`/`acc`. What shipped is **not** that: the divider is
> a CREST-PROMINENCE field, not a drainage divide, and the seeds are settlements plus a
> habitability-scaled scatter, not sub-basin outlets. The visible property the
> watershed framing was after — *borders on the ridgelines, rivers not straddled* — is
> delivered instead by a second **marker-controlled watershed snap stage** over that
> crest/river relief. See CLAUDE.md §8.10 and the module header of
> `sim/shared/provinces.rs`; read §4 below as design intent, not as shipped code.

Status: **Phase 1 shipped · Phase 2a shipped · Phase 2b (living demography) shipped ·
Phase 1.5 (honest natural borders + the Province Inspector) shipped ·
Phase 2c (deeper demography, §7.2) + Phase 3 (control) pending.** Extends the
settlement model reviewed in `SETTLEMENT_BELIEVABILITY_ANALYSIS.md`.

**Phase 1.5 — natural borders that actually are natural, plus map selection.** An audit
found the partition did not do what its own header claimed. Fixed, each with a test in
`provinces::tests`:
- borders now ride **mountain CRESTS** (`compute_ridge` prominence), not absolute
  altitude — so sub-2300 m ranges divide and high plateaus stop speckling. **3.1×** more
  border cells sit on a crest than chance;
- **great rivers divide, lesser rivers unite** (trunks are a crossing penalty, small
  rivers a travel discount along their own valley, so the river ends up at the
  province's spine — §4 step 3's original intent);
- **diagonal rivers/lakes no longer leak.** A channel traced by following flow is an
  8-connected staircase and a diagonal step cut clean between two of its cells without
  entering either, so diagonal trunks cost *nothing* to cross. Crossings are now charged
  on the EDGE. A diagonal trunk is a border **3.3×** more often than chance;
- a **marker-controlled watershed snap stage** re-places the border lines onto those
  features — a cost-flood alone can only bias a border by ≈P/2 cells, never pin it;
- **determinism bug fixed**: two `HashMap::iter().max_by_key` reductions had no tie-break,
  so the same seed could produce different partitions across runs;
- province **culture is now the plurality over its cells** (it was sampled at the seat
  cell alone) with minority shares; good **quality is a top-decile mean**, not the single
  best cell, and carries a world **rank**;
- new stats: elevation min/mean/max + relief, Köppen shares, temperature/rainfall/
  seasonality/aridity/disease, coast & river length, carrying capacity, and
  **neighbours with shared border LENGTH + which feature divides them**;
- **map selection**: clicking a province opens `ProvinceInspector`, a dossier window;
  selection is two-way with the Provinces browser and a Borders row walks to the
  neighbour. Hit-testing reads the client-side raster — no new command, no IPC.

> **Note on enclaves.** §4.3 below and the "surviving enclaves" line just under this
> note describe Phase 1 as shipped. `CITY_PROVINCE_WAR_PLAN.md` §2.1 REVERSES that
> decision on the maintainer's explicit judgement: a province enclosed by a single
> neighbour is now merged into it (an enclave read as a generation artefact, not
> history), unless the province is genuinely its own island. Read §4.3 as the
> original Phase 1 rationale, not the current behaviour.

**Implemented so far** (branch `claude/settlement-generation-analysis-wwr0ox`):
- **Phase 1** — `sim/shared/provinces.rs` cost-flood partition (coasts/islands +
  watershed divides + trunk-river crossings + border noise; enclaves survived until
  `CITY_PROVINCE_WAR_PLAN.md` §2.1 reversed that — see the note above);
  `cultures::province_name` (own variable-length names); `sim_generate_provinces`
  (runs after settlements, persists `metadata["provinces"]`) + `get_provinces`.
  Frontend: Province types, bridge, `worldStore`, **ProvincePanel (Variant B)** with
  sort/filter + goods-as-quality + analog + generated history, culture-tinted map
  overlay with natural borders, `🗺 Provinces` WindowBar chip. `cargo check` + `tsc`
  clean.
- **Phase 2a (safe foundation, no dynamics change)** — raster persisted to
  `metadata["province_raster"]`; `get_province_layer` restores list + overlay on world
  open; `campaign_province_state` (read-only join: baseline rural + live urban per
  province from the running sim); panel shows live urban during a campaign. The
  standing dynamics test still passes (economy untouched).
- **Phase 2b (living demography, gated)** — the province countryside is now a living
  reservoir that feeds the cities: rural natural increase → carrying capacity,
  urban-graveyard mortality on the largest cities, and opportunity-weighted
  rural→urban migration carrying culture. See §7.1. A dedicated test proves cities
  grow via migration while total population stays finite/bounded; the standing
  dynamics test (no provinces) is unchanged and green.

---

## 1. Goals

1. Give the world a **legible political/economic map** (fill by culture, trade good,
   population, or owner) — the iconic EU4 map feel.
2. Cleanly separate **rural (province) population** from **urban (settlement)
   population**, fixing the urban/rural conflation flagged in the believability
   analysis (§2.1 there).
3. Provide the **authoritative spatial unit** that unifies today's overlapping
   region concepts (food catchments, influence discs, trade-matrix clusters, culture
   hearths, good belts).
4. Make provinces **living**: rural population grows, migrates into cities, shifts
   culture, and — later — is the unit of political control and taxation.
5. Keep the campaign tick **tile-free and deterministic** (CLAUDE.md §5).

---

## 2. Core concepts

- **Province** — a contiguous patch of one island's land, bordered by drainage
  divides / trunk rivers / coast. The atomic administrative unit.
- **Seat** — a province's capital: its largest / most-developed live settlement.
  Recomputed as cities rise and fall (not fixed).
- **Subordinate towns** — any other settlements in the same province. A province may
  hold **0, 1, or N** cities (see §8).
- **Frontier / empty province** — a province with no settlement: wilderness, marches,
  or a colonisation target. Still has area, climate, goods, rural pop, culture.
- **Region hierarchy** — tile → **province** → area → region → continent
  (`component`). Provinces are the missing fine grain beneath the existing culture
  regions and trade components.

### 2.1 The World / Campaign split applies

- **Frozen (worldgen):** the *partition itself* — borders, cell→province map, island
  id, seat geography. Computed once at **Finalize World**, deterministic per seed,
  invalidated only if geography changes. Same lifecycle as the route-days matrix.
- **Living (campaign):** province *state* — `rural_pop`, culture shares, controller,
  goods extraction. Stored in the campaign, serde-defaulted so old saves load.

---

## 3. Data model

### 3.1 Frozen partition (worldgen → metadata)

```rust
// sim/shared/provinces.rs
pub struct Province {
    pub id: u16,
    pub name: String,             // the province's OWN generated name (§4.1), not the seat's
    pub seat_cell: (u32, u32),    // outlet/confluence or anchor settlement cell
    pub cells: u32,               // area (cell count → km² via lat-aware cell area)
    pub island: u32,              // land connected-component id (coast = hard border)
    pub neighbors: Vec<u16>,      // adjacency (for borders, migration, control)
    // ── aggregated geography (static) ──
    pub koppen: u8,               // plurality climate
    pub biome: u8,
    pub mean_fertility: f32,
    pub food_capacity: f32,       // Σ compute_food_capacity → rural carrying capacity
    pub elevation_class: u8,      // 0 lowland · 1 hill · 2 upland
    // ── economy — WHICH goods + WHAT QUALITY, never an "amount" ──
    // Each good the province's land can yield, with an environmental-suitability
    // QUALITY 0..1 (Phase 1: from the belt's good_score / fertility here; Phase 2:
    // refined by real producer `quality`). The panel shows goods + quality stars,
    // not tonnages.
    pub goods: Vec<ProvinceGood>, // { good: u8, quality: f32 }, best-first
    // ── people (static baseline; campaign overrides live) ──
    pub culture0: String,         // founding plurality culture (hearth map)
    pub rural_pop0: u32,          // baseline rural population at finalize
    // ── flavour (deterministic, generated) ──
    pub analog: String,           // "looks most like…" real-world regions (§4.2)
    // `history` short narrative is generated on the frontend (like settlementStory.ts)
    // so it can weave in live culture/goods; see §4.2.
}
```

Stored as `metadata["provinces"]` (JSON) plus a world-sized `province_id: Vec<u16>`
as one **zstd blob** (`metadata["province_id"]`), cached by fingerprint like
`world_cache`. No tile-blob schema change → no save migration.

### 3.2 Living state (campaign)

```rust
// per province, indexed by province id, inside CampaignSim (serde-defaulted)
pub struct ProvinceState {
    pub rural_pop: f32,           // the countryside reservoir (grows/migrates)
    pub culture: String,          // majority (drifts via migration/assimilation)
    pub minorities: Vec<(String, f32)>,
    pub seat_hub: i32,            // hub id of the current seat, -1 if frontier
    pub town_hubs: Vec<u32>,      // all live settlements in the province
    pub unrest: f32,              // rural jacquerie pressure (Phase 2/3)
    pub goods_quality: Vec<f32>,  // live producer quality per good (refines the static)
    // NOTE: no `owner` field yet — provinces are UNOWNED for now (user decision).
    // Political control is deferred to a later phase and deliberately left out.
}
```

A compact **province-lookup grid** (coarse downsample of `province_id`, route-matrix
resolution) is baked into the campaign seed so a settlement founded at `(x,y)` during
the campaign resolves its province with no tile access (§6).

### 3.3 Frontend type

`Province` (+ live fields) mirrored in `types/world.ts` (convention #9); the panel and
overlay read a merged `ProvinceView { …static, rural_pop, urban_pop, total_pop,
culture, settlements, owner }`.

---

## 4. Partition algorithm (Phase 1, frozen)

Runs at `finalize_world`, with `WorldBuffer` + stored `rivers`/`lakes` available.
Substrate: the hydrology pass already yields per-cell **`flow_dir`** (`FLOW_SEA` /
`FLOW_SINK` / downstream index) and **`acc`** (flow accumulation).

1. **Islands.** Label land connected-components (8-neighbour, `widx` wrap). Provinces
   never cross a coast.
2. **Drainage.** Reuse/recompute `flow_dir` + `acc` (deterministic priority-flood).
   Following `flow_dir` gives each cell its outlet (river mouth or endorheic sink).
3. **Seeds** (unioned):
   - **Sub-basin outlets** — each river `mouth`, and each **confluence** where a
     tributary above an `acc` threshold joins a trunk → one seed. Puts borders on the
     ridgelines *between* valleys, river at the province's spine ("rivers unite").
   - **Settlements** — every settlement cell seeds a province, so each has a natural
     anchor and rural provinces exist between cities.
   - Count scales with the **granularity slider** (§9) and settlement count.
4. **Grow — cost-flood** (multi-source Dijkstra from seeds). Step cost cheap
   within-basin/downhill; **expensive to cross** a ridgeline (filled-elevation gap), a
   **wide/navigable trunk river**, or a coast. Borders snap to watersheds, big rivers,
   coasts. Wrap-aware. A small **per-cell noise term** (deterministic value-noise) is
   added to the step cost so borders **wobble organically** instead of tracing a clean
   Voronoi/gradient line — see §4.3 for why this matters and how enclaves arise.
5. **De-sliver.** Merge sub-threshold provinces into the neighbour they share the most
   border with; deterministic tie-break by lowest id. Merging is by **shared-border
   length, not simple connectivity**. A later pass, added by `CITY_PROVINCE_WAR_PLAN.md`
   §2.1 and run after the border-snap stage (CLAUDE.md §8.10), additionally merges any
   province left bordering exactly one neighbour — an enclave — **unless it is its
   own island** (§4.3, reversing this phase's original "enclaves survive" decision).
6. **Aggregate** static stats from the cells (§3.1); give the province **its own name**
   (§4.1).
7. **Extract borders** — cells whose neighbour differs in `province_id` are border
   edges; stitch into closed polylines, simplify, hand to the overlay.

O(cells), one-time. Debug asserts: total land coverage, one island per province, each
settlement resolves, borders closed.

### 4.1 Province naming — its own identity, variable length

A province is a **land**, not a town, so it gets its **own** name — distinct from any
settlement in it (a province named *Shasatra* may contain the city of *Aquentia*).
Names are deterministic from the province's seed cell + its culture kit, drawn from the
same syllable engine as `cultures::place_name` but with a **wide length distribution**
so the map reads varied and organic:

- **Very short (≈15%):** 1 short syllable — *Ab*, *Ou*, *Ys*, *Mor*, *En*.
- **Short (≈35%):** 2 syllables — *Shasatra* → *Sha·sa·tra*-ish, *Velk*, *Toma*, *Nira*.
- **Medium (≈35%):** 2–3 syllables, sometimes a soft ending — *Kadresh*, *Ovanni*,
  *Serelu*.
- **Long / compound (≈15%):** hyphenated or double-rooted — *Gennma-moa*,
  *Ashkar-Vel*, *Toloma-nir*.

Rules: length bucket picked by hashing the seed cell (deterministic); syllable
inventory taken from the province's culture kit (so a Norse province sounds Norse, a
Sinitic one Sinitic); a small chance of a **geographic suffix** in the local tongue
(*-land*, *-mark*, *-shar*, *-wu*) for medium/long names. Uniqueness pass: if two
provinces collide, salt the loser's hash and regenerate. Implemented as
`cultures::province_name(kit, seed, length_bucket)` alongside the existing name
generators.

### 4.2 History & real-world analog — the historian's blurb

Each province carries two pieces of deterministic flavour:

- **`analog`** — *"looks most like…"* a curated list of real-world regions, matched
  from the province's `(koppen, elevation_class, coastal, primary good)` archetype.
  Several examples per archetype so it reads rich, e.g.:
  - **Mediterranean coast + wine/olives →** *Provence · Tuscany · the Levant coast ·
    coastal Anatolia · Catalonia · the Peloponnese.*
  - **Great river lowland + wheat/rice →** *the Nile Delta · the Po valley · the
    Ganges plain · Mesopotamia · the Mekong · the lower Yangtze.*
  - **Semi-arid hills + horses/wool →** *the Anatolian plateau · the Iberian meseta ·
    the Iranian highlands · the Kazakh steppe · the Maghreb high plains.*
  - **Cold conifer coast + stockfish/furs/timber →** *Norway · Newfoundland · the
    Baltic shore · Hokkaidō · coastal Alaska · Kamchatka.*
  - **Tropical wet + spices/sugar →** *Java · Kerala · the Caribbean · coastal Brazil ·
    the Guinea coast · Sri Lanka.*
  - **Desert oasis + dates/salt →** *the Saharan oases · the Arabian Nejd · the
    Taklamakan rim · the Atacama · the Nile beyond the cataracts.*
  - **Alpine upland + ore/gems →** *the Alps · the Andes · the Caucasus · the
    Carpathians · the Ethiopian highlands · the Harz.*
  - **Temperate oceanic plain + cloth/wheat →** *Flanders · the Île-de-France · the
    English Midlands · the North German plain · the Ohio country.*
  - **Savanna + grain/cattle/gold →** *the Sahel · the East African highlands · the
    Deccan · the Llanos · the Guinea savanna.*
  - (Extendable — the table is a plain match list; add archetypes freely.)
- **`history`** — a 1–3 sentence account generated **on the frontend** (like
  `settlementStory.ts`) so it can weave in *live* facts: the province's culture (and
  whether it arrived by migration), its founding vs current majority, its signature
  good and terrain, and whether it is a frontier, a crowded heartland, or a
  contested march. Deterministic per seed; regenerates as culture shifts.

### 4.3 Organic borders & enclaves — no straight lines

Real administrative borders are jagged and occasionally discontinuous; a clean
Voronoi map looks artificial. Three things keep ours believable, taking real
provincial borders as the reference (German *Länder*, Swiss cantons, the old HRE,
Italian *comuni*):

- **Natural tracing.** The cost-flood already follows ridgelines, rivers and coasts,
  which are themselves irregular — so borders inherit real terrain crinkle, not
  geometry.
- **Border noise (§4-step 4).** A small deterministic noise term on the crossing cost
  makes the divide **wobble** a cell or two either side of the exact gradient, killing
  any residual straight/Voronoi feel.
- **Enclaves & exclaves — REVERSED, see `CITY_PROVINCE_WAR_PLAN.md` §2.1/§5.1.**
  Phase 1 originally left provinces free to be non-simply-connected: a pocket
  cheapest-reached from a distant seat (an upland basin draining the "wrong" way, a
  valley town's hinterland across a trunk river) stayed attached to that seat,
  producing genuine enclaves — the *Büsingen / Baarle / Llívia / Campione /
  Kaliningrad* pattern. On the maintainer's explicit judgement this now reads as a
  generation artefact rather than history: a post-snap pass merges any province
  bordering exactly one neighbour into it, **unless the province is its own
  island** — a genuinely separate landmass still stands alone.

---

## 5. Multi-city provinces & the seat rule (§8 detail)

A province holds **0, 1, or N cities** — required, because dynamically-founded
colonies/swarms/satellites fall into existing provinces (§6) and we never re-partition.
It is also historically correct (the Po, the Nile, Flanders packed many towns into one
natural region).

- **One seat** = largest/most-developed live settlement; carries the province's
  identity, culture-of-record and (Phase 3) ruler. Recomputed yearly — a booming
  subordinate can *become* the seat.
- **Subordinate towns** share the one rural reservoir.
- **Satellites & suburbs** (`colony_kind 3`, `build_stage`) deliberately cluster with
  their parent → they land in the **same province** as the metropolis. A city-state +
  its *contado* reads as one unit. *(This is the user's explicit requirement.)*
- **Strict membership:** a city belongs to exactly one province (its cell). A
  metropolis may still *import food* from neighbouring provinces via the market, but it
  only administers the one it stands in.

---

## 6. Dynamic settlements & colonies (frozen partition, live membership)

The partition is **total** (every land cell owned), so anything founded later lands in
a province that already exists — no re-partition. Every founding path
(`create_market_colony`, `create_organic_town`, caravanserai, `revive_hub`,
`maybe_absorb_dying_city`) calls `province_of(x,y)` on the baked lookup grid and:

- sets the hub's `province_id`;
- pushes the hub into `ProvinceState.town_hubs`, recomputes `seat_hub`;
- **activates** a frontier province when it gains its first city.

Empty fertile provinces are the **colonisation prizes** — they map onto the existing
`colonizable` sites; a province's dormant `rural_pop` + unexploited `primary_good` is
the "prize" `maybe_found_settlement_colony` / `maybe_found_food_colony` already weigh.
Rural over-pressure in an under-citied province is what triggers a swarm town
(`maybe_swarm_town`) — new settlements are *born from provincial demographics*.

---

## 7. Province ↔ settlement living logic (Phase 2, touches the tick)

Two-tier population stock: **province = rural reservoir, settlement = urban core.**

- **Population.** `rural_pop` (countryside) + Σ urban = region total. Rural carrying
  capacity = `food_capacity` − grain exported to its cities. Surplus emigrates.
- **Rural→urban migration = the growth engine.** Yearly, a fraction of rural surplus
  migrates to the province's cities (weighted by size/pull), then along trade routes to
  hungry cities elsewhere (reusing `migration_routes`). Cities grow by **pulling in
  country folk** — fixing the "urban graveyard" inversion (analysis §2.2) and making a
  plague/war that empties a province visibly starve its city of migrants.
- **Food & production hierarchy.** Province = primary sector (raw food + raw good
  belts); settlement = secondary/tertiary (manufacture + trade). A city outgrowing its
  province must import (existing `food_balance`/`starving`, now province-sourced).
- **Culture, two-way.** Rural culture sticky; migrants carry province culture into the
  city (`hub_minorities`, `record_migration_culture`); the city radiates its lingua
  back (`compute_lingua`, `assimilation_pass`); settler colonies drive creolisation
  (`ethnogenesis_pass`).
- **Rural unrest.** Province-level jacqueries (hungry, over-taxed countryside) distinct
  from urban riots.

All of this lands in `sim/campaign/tick/` → the standing dynamics test (§2.1) is
mandatory for Phase 2.

### 7.1 Shipped in Phase 2b (living demography, gated)

The whole layer is **gated on a seeded province partition**, so a world without
provinces — and the dynamics test, which never seeds one — runs exactly as before
(base economy untouched, verified green).

- **Rural reservoir** — `CampaignSim.prov_rural/prov_cap/prov_culture/prov_seat/
  hub_province/prov_net_mig` (all serde-defaulted). Seeded at campaign start from the
  stored partition; each hub mapped to its province via the raster (nearest-seat
  fallback); colonies/swarm towns self-heal their membership.
- **`province_demography_pass`** (yearly): (1) rural **natural increase** toward the
  land's carrying capacity with a Malthusian check above it; (2) **urban-graveyard
  mortality** on the largest cities (crowding + endemic disease, eased by public
  health) so a metropolis genuinely depends on a fed hinterland; (3) **opportunity-
  weighted rural→urban migration** (prosperity · fed · commercial standing) that
  **carries the province's culture** into the city.
- Read-out: `campaign_province_state` reports the live rural pool + net migration;
  the panel shows rural drawdown and a "↗ N/yr to cities" source line.

### 7.2 Rethought — candidate additions (Phase 2c+)

Ideas surfaced while building 2b, ranked by believability-per-effort:

0. **Shipped** — buildings→province linkage (`campaign_province_detail`: estates,
   manufactories, warehouses, banks, mints mapped into each province, surfaced in the
   C-1 subwindow with custom minimalist icons + hover stats); plague→countryside
   (a strike ravages the province rural pool); **cross-province plague hop** (a
   plague creeps overland into an *adjacent* province's city via seeded
   `prov_neighbors`, reusing every spread guard). All gated on a seeded partition.
1. **Damp the intrinsic urban birth surplus when provinces exist** so migration
   becomes the *primary* engine of urban growth (a fuller urban-graveyard model),
   not an additive bonus on top of the daily logistic. The safest next tuning step.
2. **Land improvement / assarting** — a province near prosperous, peaceful cities
   slowly raises its rural *capacity* (forest clearance, drainage, terracing: the
   medieval great clearances, Dutch polders). Rich regions support denser
   countrysides over centuries; war/plague/abandonment degrade it back.
3. **Grain hinterland contract** — a city draws food from its own province first
   (cheap) before importing, so a big city in a poor province (a desert port) is
   structurally import-dependent and fragile. Couples province agriculture to the
   existing food-balance/starvation loop.
4. **Jacqueries** — province-level peasant revolts triggered by famine + heavy urban
   grain extraction; they cut the province's output and migrant flow, hurting the
   extracting city (1358 Jacquerie, the German Peasants' War, 1381).
5. **Route-bound migration corridors** — rural migrants move province→province along
   the adjacency/trade graph toward opportunity (not teleporting), drawing the
   existing migration arrows; a plague or blockade on the road starves a metropolis
   of migrants.
6. **Reversion to wilderness** — when a province's cities die and its rural pool
   collapses, it reverts toward frontier and its improved land degrades; later
   resettlement restarts the cycle (ties to the abandon/resettle system).
7. **Demographic identity in the panel** — label each province's role from its net
   migration and trend: *breadbasket source*, *migrant-sink metropolis region*,
   *emptying frontier* — and chart its population over time.
8. **Rural culture as the ethnic wellspring** — since migrants carry province culture
   into cities, a city surrounded by culture-X provinces should trend X-majority over
   centuries regardless of its founding people (cities absorbing their hinterland).
   2b lays the mechanism (culture-carrying migration); 2c would let it actually tip
   the city majority via the existing rebalance/assimilation passes.

---

## 8. Granularity control

A **partition-density slider** at generation (like the settlement-realism lever):
coarse (few large "areas", many multi-city heartlands) ↔ fine (many one-city
provinces, sub-dividing big basins at high-`acc` confluences). Default tuned so **most
provinces have 0–1 city**, multi-city only in genuinely dense breadbaskets.

---

## 9. UI

### 9.1 Overlay
`OverlayManager` "provinces" layer: filled polygons + border strokes, gated by
`uiStore.overlayVisibility["provinces"]`, Toolbar toggle. **Fill modes:** culture ·
primary trade good (the goods map) · population (choropleth) · political owner (Phase
3).

### 9.2 Province panel — **Variant B "Split"** (approved)
A `ProvincePanel`: a narrow ranked/filterable **list rail** on the left + a rich
**detail card** on the right for the selected province.
- **Sorts:** area, rural / urban / total population, **good quality**, fertility.
  *(No good "amount" — provinces show WHICH goods and at WHAT QUALITY, never tonnage.)*
- **Filters:** culture, climate, empty/frontier vs has-city, specific good.
- Row click → highlight province on map + fill the detail card.

### 9.3 Detail card (right pane of Variant B)
Shows: name (own, variable-length) · **culture + minorities** (migration-driven, can
differ from the founding culture) · area · rural / urban / total population · settlements
(seat marked, subordinates listed) · **goods with quality stars** (no amount) · mean
fertility · climate / terrain · **real-world analog** ("looks most like…") · **short
history**. **No owner field** — provinces are unowned for now.

---

## 10. Political control (Phase 3, deferred)

Province as the unit of ownership/occupation, tax base, war goals — wires into
`update_wars` (blockades, reparations, war goals already exist). Data (`owner`, `seat`)
laid in Phase 1–2; simulation later. Cores/claims optional.

---

## 11. Determinism, topology, performance

- Partition computed once at finalize, cached by fingerprint; wrap-aware; seeded merges
  → stable per seed. O(cells).
- Campaign references a compact lookup grid only — no tile access in the tick.
- Province live state is O(provinces) per year — cheap.

---

## 12. File-by-file touch list

| Layer | File | Change | Phase |
|---|---|---|---|
| Sim | `sim/shared/provinces.rs` *(new)* | partition + aggregation | 1 |
| Sim | `sim/mod.rs` | `pub use` re-export | 1 |
| Cmd | `commands/query_commands/provinces.rs` *(new)* + `mod.rs` | `compute_provinces` query | 1 |
| Cmd | `commands/campaign_commands/lifecycle.rs` | persist partition in `finalize_world`; bake lookup grid into seed | 1 |
| Reg | `lib.rs` | register command | 1 |
| DB | `db/metadata.rs` | `provinces` JSON + `province_id` blob helpers | 1 |
| FE | `bridge/query.ts`, `types/world.ts` | wrapper + `Province` type | 1 |
| FE | `canvas/OverlayManager.ts`, `state/uiStore.ts`, `ui/world/Toolbar.tsx` | render, fill modes, toggle | 1 |
| FE | `ui/world/ProvincePanel.tsx` *(new)*, `ui/world/InfoPanel.tsx` | panel + inspector | 1 |
| Sim | `sim/campaign/tick/{mod,colonies,disease}.rs` | `ProvinceState`, membership hooks, rural pop, migration engine, culture two-way | 2 |
| Sim | `sim/campaign/tick/tests.rs` | province dynamics assertions | 2 |
| Sim | `sim/campaign/tick/war.rs` + panels | ownership/occupation | 3 |

---

## 13. Phasing & verification

- **Phase 1 — partition + map + panel (pure data/render).** No tick coupling, no save
  migration. Verify: `cargo check`, `npx tsc --noEmit`, partition debug asserts.
- **Phase 2 — rural population + migration + culture (living).** Verify additionally
  with `cargo test --lib simulate_decades_reports_dynamics -- --nocapture` (§2.1).
- **Phase 3 — political control / tax / war.** Dynamics test + war scenarios.

---

## 14. Decisions (locked) & open items

**Locked (user):**
- Panel layout = **Variant B "Split"** (list rail + detail card).
- Goods shown as **which goods + quality**, never an amount/tonnage.
- Culture is **migration-driven** and may differ from the founding culture.
- **No province owner** yet (political control fully deferred).
- Provinces carry a **short history** + a **real-world analog** blurb (§4.2).
- Borders must be **organic (noise)** (§4.3). Occasional enclaves/exclaves were a
  locked Phase 1 decision; `CITY_PROVINCE_WAR_PLAN.md` §2.1 reversed it (see the
  note at the top of §2) — an enclave now merges into its sole neighbour unless the
  province is its own island.
- Provinces have **their own variable-length names** (§4.1).

**Open:**
1. Granularity: fixed default vs user slider (§8) — leaning **slider**.
2. km² conversion: expose real areas (lat-aware cell area) or abstract "size" units.
