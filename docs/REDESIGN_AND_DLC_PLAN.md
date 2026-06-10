# WorldForge 2 — World/Trade Split, Performance Overhaul & DLC Plan

Master plan for turning WorldForge from a one-shot world *generator* into a
static-world + living-trade *simulator*. Decisions below were locked with the
project owner (June 2026):

| Decision | Choice |
|---|---|
| World vs trade storage | **Two files**: immutable `.worldforge` world + mutable `.campaign` trade/history file |
| Tick model | **Manual advance button** (day / week / month / year steps); auto-play later as a timer on top |
| Barter model | **Hybrid**: world-standard prices in a numeraire (grain-equivalent), exchanges presented as good-for-good barter; per-city *currency goods* are emergent flavour; events drive production → price/inflation graphs |
| Food estates | **Emergent in the tick sim** (DLC 1), not static generation |
| Goods | Curate for **~1400 CE**, organized into **categories with continental alternatives** |
| Merchant families | **Abstract houses in DLC 1 → named dynasties with trees in DLC 2** |
| Max world size | **Keep 7200×3600 working** (peak RAM ≤ ~3–4 GB, no crashes; large worlds may stay slower) |

---

## Part I — Performance overhaul

### I.1 Diagnosis (measured/verified in code)

At the largest preset (7200×3600 cells = 25.9 M cells, 57×29 = 1,653 tiles):

| Problem | Where | Cost at max size |
|---|---|---|
| Whole-rectangle refetch on every pan | `TileManager.loadVisibleTiles` computed `needed` but requested the full visible range | every pan re-renders the whole screen |
| No request cap / no LOD | min zoom 0.05 fits the whole world on screen → ONE invoke returns 1,653 tiles of base64 RGBA | **~140 MB JSON IPC payload → webview crash** (the "largest scale" crash) |
| Decompressed-world query cache | `WorldTiles` keeps every tile decompressed in RAM, held until the next query | **~2.7 GB resident** |
| Sim world buffer | `WorldBuffer` = ~65 flat columns (15×f32 + u8/u16 + 38 goods×u8) | **~2.7 GB** per sim phase; stacked on the query cache → ~5.4 GB peak → OOM |
| Cell-by-cell scatter/gather | load/save copied 25.9 M cells × ~40 fields one assignment at a time (goods loop = 38 indexed writes per cell) | seconds of pure copy per phase |
| Per-tile commits | `save()` ran 1,653 separate `INSERT OR REPLACE`, each its own implicit transaction | seconds of SQLite overhead per phase |
| Undo snapshots | every sim phase journals ALL tile blobs (~400 MB compressed); cap was count-only (50) | **20+ GB** journal possible on a large world |
| Base64 inflation | RGBA → base64 → JSON adds +33 % and an `atob` pass per tile | slower loads everywhere |

### I.2 Fixed in this branch

1. **Fetch only missing tiles** — `TileManager` now requests exactly the
   uncached tiles (`get_tiles`), not the whole rectangle.
2. **Chunked IPC** — tile requests go out in chunks of ≤64 tiles, applied
   progressively; a single invoke can no longer carry an unbounded payload.
3. **Real LOD pyramid** — `lod` L ∈ 0..4 is now implemented: one 128×128
   response image covers a 2^L×2^L block of base tiles (cell-stride sampling
   server-side). Fully zoomed out, the largest world now transfers ~8 supertiles
   (~0.7 MB) instead of 1,653 tiles (~140 MB). Cache keys, draw scaling and
   invalidation are LOD-aware; evicted canvases are explicitly freed.
4. **Row-wise scatter/gather** — `WorldBuffer::load/save` copy whole 128-cell
   rows per column (`copy_from_slice`) instead of per-cell assignments.
5. **Parallel tile build + compress, one transaction** — `save()` gathers and
   zstd-compresses tiles on rayon workers in 256-tile batches and writes all
   precompressed blobs inside a single SQLite transaction
   (`tile_store::save_tile_blob`).
6. **Query-cache release during sim** — every sim command calls
   `WorldDb::clear_caches()` before allocating its `WorldBuffer`, so the
   decompressed snapshot and the sim buffer are never resident together.
   Compressed blob vectors are consumed during parallel decompress
   (`into_par_iter`) instead of being held alongside the result.
7. **Undo byte budget** — the journal is now capped at **512 MB total**
   (oldest entries dropped first; the newest entry always survives), in
   addition to the 50-entry cap.

Expected effect: largest-world zoom-out no longer crashes; pan/zoom traffic
drops by ~10–100×; sim-phase save time drops from seconds of I/O to roughly the
cost of parallel zstd; peak RAM during sim drops from ~5.4 GB toward ~3 GB.

### I.3 Next performance steps (ordered, not yet implemented)

1. **Column-masked WorldBuffer loads** (largest remaining RAM win).
   Each sim phase declares the columns it reads/writes; `load_with(conn, mask)`
   leaves the rest as empty Vecs and `save` merges unmodified columns from the
   old blob (which it already reads for undo). The 38-column goods block
   (~1 GB) is only needed by Phase 8. Peak sim RAM → ~1.5–1.8 GB.
2. **Binary IPC for tiles.** Tauri 2 commands can return raw bytes
   (`tauri::ipc::Response`). Pack `[count][tx,ty,lod,version,layer,len][rgba…]`
   and parse an ArrayBuffer on the JS side: kills base64 (+33 %), `atob`, and
   JSON parsing of multi-MB strings.
3. **Persist rendered LOD images** keyed by tile version (the `tiles.lod`
   column already exists in the schema) so zoom-out after a sim phase doesn't
   re-decompress the world; invalidate by version bump.
4. **Quantize f32 columns** that don't need full precision (temperature,
   precipitation, fertility, fishery, habitability, distance_to_ocean →
   u16/u8 with fixed scales). Halves both tile blobs and buffer RAM. Needs a
   tile-format version bump with append-only fallback (see Part II.4).
5. **Banded streaming sim** (long-term): phases that are local (temperature,
   koppen, soil, fertility) can process the world in horizontal bands of tiles;
   only global passes (BFS oceans, rivers, currents) need full residency.
   This is what ultimately makes 14400×7200 worlds possible.
6. **`WorldTiles` cache policy**: skip caching (decompress-on-demand per query)
   above a size threshold, or cache compressed-only and decompress per coarse
   cost-grid build.

---

## Part II — Architecture redesign: static world, living trade

### II.1 The split

**World file (`.worldforge`) — frozen after generation.**
Phases 1–6 output: terrain, elevation, shelves, ocean/atmosphere, climate,
rivers/lakes, soil, fertility, hazards (sharks/shipworm/storms/reefs/disease),
trade-good *suitability belts*, plus the goods spec. After "Finalize World" it
is read-only: opening it never mutates it.

**Campaign file (`.campaign`) — everything human.**
References the world by a content hash (so a campaign refuses to open against
the wrong/edited world). Contains:

- `world_ref` (hash + dimensions + goods-spec snapshot)
- `settlements` (placed at campaign start from habitability, then *owned* by the campaign: population, development, food security evolve here)
- `economy_state` (current per-hub stocks, prices, wealth)
- `tick_journal` (append-only event/price history — the source for graphs)
- `houses` / `dynasties` (DLC 1 / DLC 2)
- `facilities` (DLC 2)

Multiple campaigns per world; deleting a campaign never risks the world.
Both stay SQLite (same backup-API save path as today).

### II.2 Layered import ("upload world, choose layers")

New **Import World** dialog: open any `.worldforge`, tick which layer groups to
bring into the current world (terrain/elevation; climate; hydrology; soil &
fertility; hazards; goods belts). Unticked groups are regenerated by the normal
pipeline. Implementation is a column-wise tile merge (the column-mask machinery
from I.3-1 is reused directly). Imports respect step order: importing climate
without terrain is refused with the usual prerequisite warning.

### II.3 Pipeline re-cut

Steps 1–6 (plates → soil/fertility) stay the **World** wizard. Step 7+
(settlements, biological-trade economy, political) move to a **Campaign**
wizard that runs against a finalized world. `sim_run_all` is split accordingly
(`generate_world` / `start_campaign`). The Biological step's *suitability*
computation (shark/shipworm/goods belts) is world-side (it's geography);
production/trade/politics are campaign-side.

### II.4 Backwards compatibility

- Existing `.worldforge` files keep loading exactly as today (append-only tile
  format unchanged).
- On open of a legacy file, offer **"Split into world + campaign"**: geography
  columns → frozen world file; settlements/economy metadata → a new campaign.
- Tile format gets an explicit `format_version` in the header going forward
  (the v2 self-describing goods count already exists; extend the same idea), so
  the f32→u16 quantization in I.3-4 can coexist with old blobs.
- Every future campaign-schema change is append-only tables/columns; the
  tick_journal is versioned per entry type so old campaigns replay cleanly.

---

## Part III — Pricing fix & barter economy (pre-DLC core work)

### III.1 Why prices jump to ~×9.6 today

`compute_trade_matrix` accrues price along each route: every hub handoff
multiplies by a markup (~1.5–2.05) plus transport/toll/demand factors, then
clamps to a per-good ceiling `terminal_cap = 2.2 + 9.0·(scarcity−1) + luxury_bonus`
(min 3.0, max 32). Compounding hits the ceiling after ~2–3 hubs and sticks —
for a moderately scarce good the cap computes to ≈9.6, which is exactly the
observed plateau. Price is a *route artifact*, not a market state.

### III.2 New model: stock-based local prices in a grain numeraire

Replace per-hop compounding with **per-settlement market state**:

```
stock[h][g]    units on hand (production + arrivals − consumption)
need[h][g]     basic-needs demand first (food, fuel, salt, cloth, timber, iron),
               then comfort, then luxury — filled in that order
price[h][g]    = base_value[g] · (need[h][g] / max(stock[h][g], ε))^k   (k≈0.5–0.7)
               smoothed over ticks (no instant ×9.6 jumps; scarcity builds)
```

- `base_value[g]` is expressed in the **numeraire: kg-grain-equivalent** — the
  "world standard" price the user asked to keep visible everywhere.
- Transport adds *cost*, not compounding markup: delivered cost = origin price
  + freight (days × mode rate) + tolls; merchants move a good only when
  destination price − delivered cost > margin. Arbitrage, not decree, creates
  the spatial gradient — and it now *decays* as goods flow (self-limiting
  instead of capped).
- **Inflation/price graphs**: every tick appends `(tick, hub, good, price)` to
  the journal; the settlement window plots price history and a city-level
  price index (basket-weighted) = the inflation curve. Events (Part IV) shift
  production or stocks and the graphs show it.

### III.3 Barter presentation & emergent currency goods

Exchanges are *recorded* as good-for-good swaps. For each city and tick:

- **Currency score** per good = liquidity (how many distinct trades it appears
  in) × divisibility class × value density × stock stability. The top good(s)
  are labeled that city's **currency goods** (silver, salt, grain, cloth…).
- The **settlement window gets a Market subpanel**: table of goods with
  *price (grain-eq), trend sparkline, in/out flow, and "exchanged for"* — the
  top counter-goods by traded value (e.g. `wool → wine (×3), salt (×1.5)`).
  Currency goods get a badge.

### III.4 Per-capita wealth: grain wealth vs trade wealth

Two explicit, per-settlement measures (replacing the current single normalized
`wealth`):

```
grain_wealth[h] = food_stock_value / population        (food security; drives
                  growth, estate founding, famine risk)
trade_wealth[h] = (export_earnings + entrepôt_margins − import_spend)
                  / population                          (commercial prosperity;
                  drives merchant houses, facilities, luxury demand)
wealth_pc[h]    = grain_wealth + trade_wealth           (shown per capita)
```

Both are displayed in the hub panel and journaled per tick for graphs. Demand
becomes **income-closed**: a city's imports per tick are budgeted by its
earnings (fixes the open-budget gap flagged in TRADE_SYSTEM_REVIEW §3.2).

### III.5 Goods curation (~1400 CE, categories with continental alternatives)

Reorganize the flat 38+12 list into **categories**; within a category,
different climates/continents produce different *alternatives* that satisfy the
same need (key for barter: a city short of wheat can substitute rice or rye —
at a substitution penalty):

| Category (need) | Alternatives (climate-gated) |
|---|---|
| **Cereal** (food-staple) | wheat, barley/rye (cool), rice (warm-wet), millet (steppe/arid) |
| **Protein** | stockfish, herring (everyday fish), salt-meat/hides cattle, dates (oasis) |
| **Fat/oil** | olive oil, butter/lard (north), sesame/palm analogue (tropics) |
| **Fiber/cloth** | wool (fleece), cotton, linen (flax), silk (luxury tier) |
| **Drink** | wine, beer/ale (grain north), tea, coffee |
| **Sweetener** | honey (default north), sugar (tropical) |
| **Preservative** | salt (deposits + bay-salt merge) |
| **Fuel** | firewood/charcoal (from timber belts), peat (cold bogs) |
| **Metal** | iron, copper+tin (bronze pair), gold, silver, lead |
| **Construction** | timber, marble/stone, hardwoods |
| **Dye/colour** | dyes (merge indigo + tyrian purple as regional variants), saffron |
| **Aromatic** | incense (merge frankincense into it), spices, pepper, cloves, cinnamon |
| **Craft (urban)** | ceramics, glassware, paper |
| **Prestige** | gemstones, jade, amber, ambergris, pearls, ivory, furs |
| **Livestock/transport** | horses, camels-analogue (unlocks cheap desert legs — see review §3.14) |

Changes from today: **merge** frankincense→incense, indigo+tyrian→dyes
variants, bay_salt→salt; **drop** tobacco (post-1400 for the “old world” feel;
keep as an optional spec); **add** barley/rye, rice, millet, herring, honey,
hides/leather, beer, firewood/charcoal, butter/oil alternative. Net ≈ 40 goods
in 15 categories, every category having a cold/temperate/tropical/arid answer.
All additions ride the existing append-only goods serialization and the
declarative `GoodSpec` editor; category + need-tier become new `GoodSpec`
fields.

---

## Part IV — DLC 1: "Living Trade" (tick simulation module)

**Premise:** after the river step, the human layer moves into its own module —
cities live, trade flows per tick, prices fluctuate, history accumulates.

### IV.1 Engine

- **Tick = 1 day** internally; the UI advances by day/week/month/year (a month
  button runs 30 ticks). Deterministic per (campaign seed, tick).
- Per tick: production (seasonal phase from the existing month machinery) →
  consumption (needs ladder) → merchant dispatch (arbitrage decisions over the
  existing coarse route graph, reusing Dijkstra results cached per route) →
  arrivals (goods in transit have ETAs from the travel-days work) → price
  update (III.2) → events → journal append.
- Target budget: ≤50 ms per tick at 40 hubs / 40 goods so a year (365 ticks)
  fast-forwards in ~15–20 s with a progress bar. All hub-level math is tiny
  compared to worldgen; the cost discipline is *not touching tiles per tick*.
- **Caravans/ships as entities** (id, route, cargo, ETA, owner house) — they
  make trade visible on the map and give events something to hit.

### IV.2 Events (production & price shocks)

Weighted random + condition-triggered, journaled, all affecting *stocks or
production multipliers* so the price graphs respond naturally:

- **Natural:** drought (cereal −40 % in a region for N ticks), blight, fishery
  collapse/recovery (the depletion mechanic from review §4.1), harsh winter
  (passes shut longer), storm sinks ships (cargo lost).
- **Settlement:** fire (stock loss), plague (population + demand drop), festival
  (luxury demand spike), new mine opened / vein exhausted (deposit goods).
- **Mercantile:** house feud (embargo between two hubs), price war (margin cut),
  caravan robbed (piracy level), monopoly corner (house buys out a good →
  price spike until it sells).
- Every event is a row: `(tick, type, target, magnitude, duration, text)` —
  the settlement window shows a city's event history; the world log shows all.

### IV.3 Food estates & starvation

- Each tick, big cities compute **food balance** = local food production +
  food imports − population need (via the Cereal/Protein/Fat categories).
- A city with sustained surplus wealth but food deficit **founds an estate**:
  a new `estate` settlement seeded in the nearest high-cereal-suitability
  cells within reach (river/coast preferred), tied to its parent city.
  Estates ship food on dedicated routes; they can grow into towns.
- If food balance stays negative (estate failed, route cut by event, war):
  `grain_wealth → 0` → **starvation**: population decline, demand collapse,
  out-migration event; severe cases downgrade size tier. This is the failure
  loop the user asked for ("…and it to starve").

### IV.4 Merchant houses (abstract, DLC 1 scope)

- Per major hub, 2–4 **houses**: `{name, wealth, specialization goods, routes
  held, monopoly share, rivalry map, prestige}`.
- Houses *own* the caravans/ships; profits accrue to houses; per-good monopoly
  share shifts margins (corner → price up). Feuds = embargo/price-war events
  between named houses (visible flavour for the dynasty DLC later).
- Succession: every ~30 sim-years a house rolls succession (thrive/split/
  decline) — a wealth-over-generations chart per house, no individuals yet.

### IV.5 UI

- **Time bar** (bottom): date, advance buttons, tick-progress.
- **Settlement window** additions: Market subpanel (III.3), price/inflation
  graphs, food balance + estate list, event log, houses tab.
- **World economy panel**: per-good world price graph, biggest movers, currency-
  goods map mode.
- **Save**: campaign autosaves per advance; journal is append-only so saves are
  incremental and fast.

### IV.6 Milestones

1. Campaign file + world freeze/split (Part II) — prerequisite.
2. Tick engine with production/consumption/prices, no events (graphs working).
3. Merchant dispatch + in-transit entities + arbitrage pricing.
4. Events + food estates + starvation.
5. Abstract houses + feud events.
6. Polish: world economy panel, currency detection, autosave/replay.

---

## Part V — DLC 2: "Merchant Dynasties & City Facilities"

Builds strictly on DLC 1 journals (campaigns from DLC 1 upgrade in place:
abstract houses become founding generations).

1. **Named dynasties**: house → family of individuals (head, heirs, spouses);
   births/marriages/deaths per tick using the existing `names.rs` machinery
   extended with family surnames. Marriages between houses = alliances
   (rivalry map becomes a web of pacts/feuds with personal causes).
2. **Family tree view**: per-dynasty tree (generations, portraits-by-trait,
   reign periods over the family ledger), plus a **history log** ("In Spring
   721, Matriarch Yelena cornered the salt of Kessa…") generated from journal
   rows — the journal schema from DLC 1 is already sufficient.
3. **City facilities**: houses (or cities) build **facilities** on real map
   cells near their hub: warehouse (stock cap +, spoilage −), shipyard (cheaper
   ships, opens longer crossings), guild hall (margin +), mint (currency-good
   stabilizer), kontor/quarter in *foreign* cities (Hanseatic mechanic: trade
   rights + toll exemption), irrigation/terraces (estate yield +), mine works
   (deposit yield +, exhaustion risk).
4. **Monopolies as objects**: a house can hold a *charter* on a good in a
   region (granted by prestige + paying the city) → other houses smuggle or
   contest → feud events. Monopoly share already exists numerically in DLC 1;
   this gives it agency.
5. **Dynastic victory/legacy screen**: richest lineages over campaign history,
   surviving monopolies, built facilities — the "see the dynastic merchant
   family line" payoff.

---

## Part VI — Suggested build order (across releases)

| Release | Content |
|---|---|
| **R1 (perf)** | Part I.2 (done) + I.3-1..3 (column masks, binary IPC, persisted LOD) |
| **R2 (split)** | Part II: world/campaign files, layered import, legacy migration |
| **R3 (economy core)** | Part III: stock prices, numeraire, barter ledger UI, per-capita wealth, goods curation |
| **DLC 1** | Part IV milestones 2–6 |
| **DLC 2** | Part V |

Open questions to settle before R2 implementation: campaign file naming/UX
(one-click "New campaign on this world"?), whether estates are paintable/
movable by the user, and how much of the old static "Biological-Trade step"
remains as a preview mode for users who don't buy DLC 1.
