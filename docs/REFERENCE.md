# WorldForge 2 — Science & Systems Reference

> Extracted from CLAUDE.md to keep the always-loaded file lean. This is the deep
> reference for the world pipeline and campaign systems; read sections on demand.
> Linked from CLAUDE.md §8.

## 8. Reference — Science & Systems

### 8.1 Overlays (query commands, NOT stored in tiles)
- `compute_shark_zones` / `compute_shipworm_zones` → highest-risk hazard cell-masks
  (Toolbar → Biological: 🦈 / 🪱). Also `compute_reef_zones`, `compute_storm_zones`,
  `compute_monsoon_zones`.
- `compute_good_regions` → per-good belt cell-masks. **The FILL it returns is no
  longer drawn** (§8.19) — it now supplies only the LABEL: the medallion centroid,
  the gemstone `sublabel`, and the coarse fallback for a good whose mask hasn't
  arrived. One toggle per good under Trade Goods.
- `compute_good_belt_masks(goods)` → the FULL-RESOLUTION two-layer belt mask per
  named good (§8.19). Takes a list, not the whole world, because a mask is a real
  payload and only the toggled goods are ever drawn.
- `compute_trade_routes(settlements, rivers, reach, max_crossing)` → least-cost routes
  over the shared coarse cost grid (passes / rivers / coast-hugging); reach limits
  open-water crossings.
- `compute_trade_matrix(...)` → settlement-cluster regions, per-good prod/demand/net +
  per-good `flows` + **routed & bundled `trunks`** (edge width ∝ volume). Sea-
  impassable pairs (under the reach) get no flow.
- `compute_climate_bands` → the circulation belts + the **ITCZ at BOTH seasonal
  extremes** (`itcz_july` / `itcz_january`, per column, from the same helpers the
  seasonal wind is built from). The overlay draws July solid, January dashed, and
  hatches the migration band between them at low opacity — that band is the land
  which changes circulation regime between seasons, i.e. the monsoon belt.
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

**The monsoon is a MIGRATING ITCZ, not a land–sea breeze.** `belt_wind_shifted`
displaces the whole circulation toward the summer hemisphere; the meridional
direction comes from which side of the *displaced* ITCZ a cell sits on, while the
Coriolis handedness comes from its **true** latitude. Keeping those two conventions
separate is the entire mechanism — it is what makes cross-equatorial flow recurve
into a real southwesterly. Land–sea contrast enters only as `MONSOON_LAND_PULL`,
selecting the LONGITUDE the convergence zone reaches furthest poleward (Chao & Chen
2001; Gadgil 2003; Geen et al. 2020). At `shift = 0` it is bit-identical to the
annual-mean field, so the ocean-current model is untouched. Three rules: the
migration must stay **tapered to the tropics** (a uniform shift reverses the
Southern Ocean westerlies); `monsoon_onshore` must stay **wind-aware** (a purely
geometric ray lets subtropical deserts switch off their own subsidence sink — the B
row falls 68.1% → 61.3%); and `earth_monsoon_wind_reverses` must keep passing.

**Ocean currents** are a gyre-aware relaxation, not a solve: the *interior* comes from
the Sverdrup relation (curl of belt wind stress on a β-plane — sign and latitude
structure EMERGE), while boundary speeds are prescribed constants
(`SPEED_BOUNDARY_WEST` 2.2 vs `..._EAST` 0.55 = western intensification), then 20
deflection passes + bathymetry steering. The field is **not divergence-free** and
currents are **annual-mean only** even though the winds have two seasons.

**Known fidelity gaps** (measured, with fixes planned — see `docs/FIX_PLAN.md`
A7–A14, which include three REVERTED attempts kept as negative results):
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
- **UNLIMITED** (stockfish, furs, timber, **hardwoods**, salt, whaling, wheat, iron,
  and the two naval stores **pitch**/**hemp**) — every suitable cell produces.
  `hardwoods` joined this list because it and `timber` are ONE ROLE — the wood a hull
  is built from — split across climates, and they had opposite distributions: every
  suitable boreal/temperate cell grew timber while the entire tropics shared a SINGLE
  seeded hardwood homeland. That left a tropical or desert-coast city with no local
  hull wood at all, which would make shipbuilding structurally impossible for exactly
  the maritime cultures that were best at it (measured 311 cells / 11 settlements →
  540 / 17). See `docs/YARDS_VESSELS_AND_DEPOTS_PLAN.md` D1.
- **SEEDED** (rest) — `localize_good` picks ONE weighted seed + flood-fills one
  homeland, with ~4% map-width island-jump. Land goods stop at mountains ≥3000 m
  (`MOUNTAIN_NORM`≈0.339); marine goods stop where the score envelope drops.
- **GEMSTONES / metals / stone (`Deposits`)** — placed by REAL ORE GEOLOGY
  (`step8_biological_goods/deposits.rs`, §8.16), not by an elevation floor.
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

**A SELLER RANKS ITS BUYERS BY ARBITRAGE GAP, NEVER BY INDEX.** Each arbitrage
shipment takes `surplus * 0.5` and re-reads `surplus` from the seller's stock
every time, so the k-th destination a seller is offered receives on the order of
**2^-k** of its output — a geometric cascade down the iteration order, not the
gentle `break` the code reads as. Stocks are seeded once from production at the
top of `solve` and never replenished, so the cascade repeats identically every
round. Walking `b` in plain index order therefore made that cascade follow the
SETTLEMENT RANK ORDER, because `compute_economy` sorts its nodes by score
(strongest first) and market hub index IS that rank: every supplier served the
great cities first and had nothing measurable left for a small town, which read
**0 throughput however close or hungry it was**, and strictly worse the more
settlements a world had. Destinations are ranked by the gap they offer now,
which is rank-BLIND — an unsupplied small town sits at `PRICE_CEIL_MULT` exactly
as a large one does, and among ceiling-priced buyers freight decides, so the
NEAREST buyer wins. Ties break on index so `converges_and_stays_finite`'s
determinism assertion still holds. Gate:
`a_near_buyer_is_served_before_far_ones_whatever_its_index`.
**The halving itself is left alone and is a named open item**: "half-steps
toward parity per round" is applied per (seller, buyer) PAIR, so it compounds
ACROSS destinations within one round rather than once per round as the comment
intends. Ordering was the targeted fix; changing the step size is a real
economic change needing its own dose walk.

**`compute_economy` is where two shipped Stage-A/B fixes had never been ported**
(`ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md`), which is why the drawn map and the
ledger could disagree about the same town:
- **B3, the lesser-town fall-through.** A settlement past `major_n` got exactly
  ONE candidate link — its single nearest major hub. A Dijkstra miss, a
  `path_allowed` rejection or an over-ceiling pair left it with ZERO edges and
  no trade at all, while `compute_trade_routes` (`routing.rs`) had already
  fixed this with a ranked `MINOR_FALLBACK_K` shortlist. `compute_economy`
  now collects the same 5-candidate shortlist, batches it into the SAME
  `coarse_dijkstra_batch` (one Dijkstra per coarse SOURCE, so five candidates
  cost what one did) and takes the first accepted — topology still mirrors the
  drawn routes, it just no longer gives up after one failure.
- **B2, the link ceiling.** `max_link2` was a magic `grid_w * 0.30` =
  **12,022 km** on the default grid, wider than any ocean on Earth, so two
  continents always wired into one trade graph and the `comp[a] == comp[b]`
  guard below it could never fire. It reads `TRADE_COMPONENT_HORIZON_KM`
  (3,000 km) now — the same constant `compute_routed_components` already
  applies on the campaign side, so the worldgen economy stops promising lanes
  the campaign it seeds would refuse, and **genuinely isolated continents can
  form and trade internally**.
Note `EconHub.partners` is `centrality[hh]`, i.e. EDGE COUNT, not distinct trade
partners — a lesser town reads "1" structurally, by topology, not measurement.

**TWO WAYS A CONNECTIVITY GATE HERE SILENTLY PASSES**, both of which the first
cut of these two gates did (§8.24a3's rule — always verify a new gate fails on
the unfixed code — caught them):
- **`distance_to_ocean` defaults to 0.0 and `node_sea` is `d < 0.06`.** A fixture
  that does not set it marks EVERY settlement a sea port, and the maritime-bypass
  block then wires each coastal MAJOR to its five nearest coastal hubs — lesser
  towns included. The town gets an edge by another route entirely and the gate
  passes on the broken code. Hold every node inland unless the test is about
  ports.
- **A major links to its 4 nearest among ALL nodes**, not just other majors, so a
  lesser town sitting alone beside a hub is picked up incidentally however the
  lesser-town path behaves. Pack enough majors around the intended fallback hub
  that none of them has the town in its top 4.

The per-good lane cap (220) trims the display snapshot, but the per-hub
exports/imports/receives totals accumulate inside that same loop, so a trimmed
lane reads as no trade. Lanes are ranked among the lanes arriving at the SAME
(good, destination) before the cap applies, so it can only ever trim a hub's
second and later sources — every buyer keeps one supplier before anyone keeps
two.

**THE OPEN-WATER CROSSING IS A WORLD RULE, NOT A PREFERENCE**
(`MAX_OPEN_SEA_CROSSING_KM` = 800 km, `query_commands/mod.rs`). Pre-modern trade
is coastal navigation punctuated by SHORT blue-water hops — Sicily to Cape Bon
~150 km, Greece to Italy ~150, Norway to Shetland ~300, and the longest routine
classical crossing, Crete to Egypt, ~500 km, written about precisely because it
was exceptional. Nothing routine crosses an ocean. The limit used to be a user
preference ALONE (`max_crossing`, a fraction of map width, shipped at 0.12 =
**4,809 km**), so an ocean traversal was the default rather than an opt-in.

Three things make it a rule rather than a setting:
- **The clamp lives in `path_allowed`**, the one chokepoint every route, flow,
  economy, political and component query already funnels through. A per-command
  clamp across the nine entry points that take `max_crossing` is a rule with
  nine ways to be missed. A caller may only ever ask for something SHORTER.
- **It binds at EVERY reach except 2.** Reach 0 used to be an unconditional
  `true` (the UI called it "Global — cross any ocean"), so one caller passing
  reach 0 put every trans-oceanic lane straight back — and `MapCanvas.tsx`'s
  `CAMPAIGN_ROUTE_REACH` is exactly such a caller. Reach 2 stays land-only.
- **`compute_routed_components` uses the SAME constant.** It passed
  `TRADE_COMPONENT_HORIZON_KM` (3,000 km), which said two landmasses shared a
  market across water no route could actually cross — the B2 inconsistency one
  layer up. The two constants are now distinct things and say so: this one is an
  unbroken open-water RUN, `TRADE_COMPONENT_HORIZON_KM` is a straight-line lane
  LENGTH bounding `compute_economy`'s candidate links.

**It removes oceans, not the sea.** Shelf and coastal water reset the run (see
`is_open_sea`), so coast-hugging is unlimited and an island chain is crossed one
hop at a time — which is how a pre-modern network reaches a long way without
ever losing sight of land. A rule that forbade all sea would just be reach 2
under another name, and the gate asserts BOTH halves for that reason.
Gate: `no_caller_can_ask_its_way_across_an_ocean` (0.12 and 1.0 both refused at
reach 1, reach 0 refused too, a short hop still allowed), plus the two component
gates re-pointed at this constant. The frontend default is ~400 km and its
slider is stated in KM over a 50-800 km range — "% of map width" is not a
distance anyone can reason about, and reading it as one is how 12% came to mean
an ocean. `SEA_CAP_KM` in `StepBiological.tsx` bounds the SLIDER only; the
backend clamps regardless, so drift between them stops the slider short rather
than breaking the rule.

**THE CROSSING RULE IS PART OF THE SEARCH, NEVER A VERDICT ON THE FINISHED PATH.**
`coarse_dijkstra` returns the globally CHEAPEST path, and every caller used to run
`path_allowed` over it as a POST-FILTER. So a lane whose cheapest line cut across a
gulf had the WHOLE route discarded and fell back to a dashed straight line — while
an ordinary coast-hugging route between the same two ports existed the entire time
and was never searched for. That is the "trade routes are straight lines instead of
following the trade routes" report, and tightening `MAX_OPEN_SEA_CROSSING_KM` made
it fire MORE often rather than less: the stricter the rule, the more often the one
path considered is the one rejected.

A crossing limit is not a property of a PATH, it is a property of a path PREFIX —
how much unbroken open water you have already crossed to arrive somewhere — so it
belongs in the search STATE. `coarse_dijkstra_legal` makes a node `(cell, run)`:
`run` counts consecutive open-sea cells ending at this one, resets to 0 on shelf,
coastal water or land, and a step that would push it past the cap is simply NOT AN
EDGE. The search therefore returns the cheapest LEGAL route, which for two ports
either side of a gulf is the one that follows the shore.

Three things that make this safe:
- **The state space is bounded at every world size.** `run` is capped (~14 coarse
  cells at the shipped 800 km) and the coarse grid is ~720 cells wide whatever the
  world, because `f = grid_w / 700`. A Large map does not enlarge it.
- **Reach 2 needs no run state** — land-only is a plain mask
  (`coarse_dijkstra_masked`), so it does not pay for lanes it cannot use.
- **`path_allowed` survives as a `debug_assert!`** at the call site, not as control
  flow: the searched route must satisfy the very rule it searched under, and if it
  ever does not, that is a bug in the search rather than a lane to discard.
Gate: `a_lane_follows_the_coast_instead_of_being_thrown_away`, which asserts BOTH
halves on ONE fixture so it cannot pass vacuously — the old search-then-filter must
FAIL on that fjord, and the new search must return a longer, legal, coastal route.
Writing it took two fixtures: the first was a shallow bay where going round was
already cheapest, so search-then-filter would have kept the path and the gate proved
nothing. **A routing gate needs a geometry where the illegal line genuinely WINS.**

**THE RULE DID NOT REACH THE CAMPAIGN'S OWN ROUTING, only the map query layer.**
`CampaignSim.days`/`route_outlet` — which decide every relay assignment, dispatch
choice and freight cost inside a running campaign — were priced by
`compute_route_days_matrix_for_season` calling the plain unconstrained
`coarse_dijkstra_dist_prev`, never `coarse_dijkstra_legal`. So a relay pair could
be internally "legal" by the campaign's own accounting while the map's own
resolver (correctly enforcing the rule) found no legal route for it and fell back
to the dashed-direct convention — the straight line was the render being honest
about upstream data, not a rendering bug. Fixed with `coarse_dijkstra_legal_dist`,
the same `(cell, run)` state extended to a full single-source-to-everywhere
search (§8.5's `route_days_matrix` fix, 2026-09-19c). A SECOND, LIVE injection
point had the same gap with a worse unit error: `production.rs`'s periodic
`rebuild_routes` (`#6c` coastal cabotage) wrote raw Euclidean distance straight
into `days` with no pathfinder and no crossing check, and its own
`CABOTAGE_SEA_FRAC` constant computed to ~3,206 km against a doc comment claiming
"a deliberately SHORT crossing" (~1,000 km) — 4× the real 800 km cap. Now derived
directly from `MAX_OPEN_SEA_CROSSING_KM`. `#6`/`#6b` (the guaranteed-partners and
market-lifeline rescues) share the same unconstrained-Euclidean shape but are
same-component-only (land routes have no crossing cap to violate) — named as a
queued, unmeasured follow-up rather than silently assumed clean (rule 36).

**AN ENTREPÔT PICKED BY RAW DISTANCE ALONE IS NOT A REAL PLACE MERCHANTS STOP.**
`#6d`'s outlet was "any real coastal hub, whichever minimises `days_before[a][p]`" —
purely geometric, with no requirement that anyone actually traded there. Composed
with an unconstrained rescue leg (the `#6`/`#6b` gap named above), this is exactly
what let a relay's OWN two legs come out implausibly long even after both crossing-
rule fixes landed: neither leg crosses open water illegally, so the map draws it as
a real solid line, but a "break of bulk" at a hub with zero actual trade volume is
not a transshipment, it is a coincidence of geometry. `hub_trade_volume` (summed
from `trade_last`, both directions folding onto the same hub index there) now
gates outlet eligibility: a candidate outlet must already show real annual trade,
unless NO hub anywhere has any yet (a brand-new campaign's first `rebuild_routes`,
before `fold_trade_year` has ever run), which keeps the old coastal-only rule as
the bootstrap case so a fresh campaign still has entrepôts to grow from. The same
session also fixed the break-of-bulk relay LIST (`read_hubs.rs`) to only report
pairs with real trade volume between them — a pair whose cheapest theoretical path
happens to relay through this hub, with nothing ever shipped that way, was showing
in the panel as "206d, no direct trade", which read as a route when it was only a
hypothesis. `#6`/`#6b`'s own unconstrained-distance gap (named above) is unchanged
by either fix and remains queued.

**Still post-filtered, and why** (queued, not waived): `coarse_dijkstra_batch` —
which feeds `compute_trade_routes`' drawn road network and `compute_economy`'s
candidate edges — still searches then filters, so a rejected pair there is a route
that simply is not drawn (never a straight line). Giving the batch the same
treatment means carrying `(cell, run)` dist/prev per SOURCE while rayon fans sources
out in parallel, which is tens of MB per in-flight source — a real memory question
that needs its own measurement, not a copy of this change.

**A LANE IS NOT ONE MEDIUM END TO END.** `compute_coarse_route` returned a bare
`Vec<[f32; 2]>` and the renderer styled the whole lane from a single flag —
`openWater`, meaning "did anything route this at all". So a haul that runs
overland to a port, crosses, and runs overland again was drawn end to end as one
thing, and read as a straight sea slash between two cities even where it followed
a real road. It returns `CoarseRoute { points, sea }` now, with the medium of
EVERY point, and `OverlayManager.mediumRuns` splits a lane into consecutive runs
of one medium (breaking at the wrap seam too, rule 6) so each is stroked in its
own convention: **dashed on open water, solid wherever the cargo is on a road or
a navigable river**.

Four things to keep true here:
- **`sea` and `openWater` answer different questions and both are needed.**
  `openWater` decides whether a lane earns direction arrows and full opacity —
  a straight line with arrows every few cells reads as a drawn road, which is
  exactly what an unroutable claim is not. `sea[i]` decides how each stretch is
  stroked. A lane nothing could route is all-sea AND `openWater`, so it stays
  dashed end to end, which is the honest reading of a claim.
- **A segment is sea when EITHER endpoint is water**, so the landing legs belong
  to the crossing and no solid stub pokes out into the sea. Consecutive runs
  share their boundary point, so there is no gap where the medium changes.
- **The worldgen trade-route graph carries medium too.** `ensureTradeGraph`'s
  edges take `TradeRoute.kind` (0 land · 1 sea · 2 river), so a path assembled
  out of that graph knows its stretches as well as a coarse-grid route does —
  otherwise a worldgen sea lane drew solid while an identical campaign one drew
  dashed.
- Applied so far to the **Flows highlight** (`renderFlowHighlight`) and the
  **Goods Atlas flows** (`drawGoodFlows`). Merchant routes, the house trading
  web and futures lanes all go through `laneBetween` and so already HAVE the
  per-point medium available — they just still draw with the old whole-lane
  styling. Extending them is queued, not done.

**This is also what finally produces coast-hugging**, and it is worth recording
that it did NOT come from the cost ramp. `OPEN_SEA_COST` (2.2) is only 1.375x
`COASTAL_SEA_COST` (1.6) while a coast-hugging detour is routinely 2-4x longer
in cells, so on cost alone a least-cost path always cuts straight across — and
because open sea is cost-UNIFORM, Dijkstra's cheapest path across it is a
literal straight line, which is what the ruler-straight ocean lanes were.
Fixing that by raising the ratio to 3-5x was the obvious move and is still NOT
done, because `cost_to_days` makes these same constants double as travel SPEED
(C2's conflation in `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md`), so it would
reprice every voyage and move the price/distance gradient C1 just tuned. A
ROUTING rule achieves the same visible outcome at zero cost to the speed
calibration: forbid the long crossing and the cheapest legal path is the
coastal one. Prefer a routing rule over a cost nudge wherever the cost constant
is load-bearing for something else — and the cost-ratio work stays queued, with
`real_world_price_distance_gradient` as the instrument that can tell the freight
effect from the speed effect if anyone does take it on.
Related finding, left as a finding: the piracy surcharge charges COASTAL water
4.0 against open sea 1.5, which pushes routes AWAY from the coast — the opposite
of what its own comment claims it does. Inert at the default `piracy = 0`.

### 8.6 Köppen current overrides
Mediterranean (Cs) only forms on **windward (west-facing) coasts** beside a cold
offshore current (`cold_override` gated on `is_windward_ocean` + no warm influence);
a warm-current **east** coast reads humid-subtropical (Cfa).

### 8.7 Render layers (26) & paint tools (5)
The **biomes** layer is no longer a Köppen recolour — it reads the `biome` column
and draws procedural pattern fills (§8.12). **`natural`** is the land-COVER view
(§8.21) — the only layer whose land colour is not a function of height or class
index but of what grows there.
land, elevation, terrain (hillshade), **natural**, plates, shelf, ridges, fisheries, currents,
**sst** (sea-surface temperature), temperature, precipitation, wind, **windspeed**
(low-level wind intensity incl. jets), **snow** (annual snow-cover fraction), climate,
biomes, soil, fertility, salinity, habitability, shark, shipworm, reef, storm, disease.
Paint: Pan, Paint Land (0/1), Elevation (f32 0-1), Paint Shelf (u8 0/1), Place Volcano (u8 0/1).

### 8.8 File operations
Save/Open via SQLite backup API (`.worldforge` / `.campaign`); Export Heightmap
(16-bit grayscale PNG from elevation); Export Layers / trade data; Import Template
(image → land/sea auto-detection).

### 8.9 Phase-3 performance shape (read before touching `step3_ocean_atmo/`)
Phase 3 runs on the **world** grid (6.5 M cells at the default 3600×1800, 26 M on
"Large"), so a per-cell scan that looks harmless is an O(n·w) trap. `bench.rs`
prints a per-sub-step millisecond breakdown; run it after any change here.

Measured @ 3600×1800, release, 4 cores: **~16 s** (was ~100 s before the
optimisation pass). The four costs that matter, and the rules they imply:

| Sub-step | ms | Shape |
|---|---|---|
| `generate_ocean_currents` | ~5600 | warm/cold streamline tagging |
| `advect_salinity_and_recouple` | ~4200 | a second `extend_warm_tag` |
| `compute_precipitation` | ~2900 | per-cell moisture rays + 36 blur passes |
| `compute_low_level_jets` | ~2200 | per-cell barrier rays + 48 propagate passes |

1. **Never scan outward per cell.** Distance-to-land fields are linear sweeps
   (`precompute_basin_dist`/`_ns`): a running counter per row/column, not a
   search. The naive form cost 31 s of the old 100 s on its own.
2. **The row loops are rayon-parallel** (`par_chunks_mut` over rows, or
   `into_par_iter` over seed rows). Keep new passes writing only their own cell so
   they stay that way. Where a pass is a monotone union or a max-reduction, it is
   parallelised with relaxed atomics (`AtomicBool` for the tag corridors,
   `AtomicU32::fetch_max` on the f32 bit pattern for the moisture field — valid
   because moisture is never negative), which keeps the result bit-identical
   regardless of scheduling.
3. **The streamline tracers are latency-bound, not compute-bound.** They walk a
   scattered index for hundreds of steps, so they read a packed `trace_view`
   (one flag byte + one interleaved `[vx, vy]`) instead of four separate columns,
   and they fold the tracer's x back into `[0, w)` every step so `wrap_x` stays on
   its cheap in-range path.
4. **Hoist anything loop-invariant out of a repeated pass** — the jet propagation
   resolves each cell's upwind neighbour once, not on all 48 passes.

The whole pass is **output-preserving**: `ocean_atmosphere_field_checksums` prints
a checksum per phase-3 field, and every one is unchanged vs. the pre-optimisation
code. Use it the same way for any future refactor here — the Earth gate scores
agreement to 0.1 %, which cannot tell bit-exact from merely close.

### 8.10 Province borders — why the partition has TWO stages
`sim/shared/provinces.rs` (phase 7b) does **not** simply flood from seeds. It cannot:
a cost-flood's border falls where two seeds' CUMULATIVE costs tie, so a barrier of
penalty `P` merely displaces that tie-line by ≈`P/2` cells. Adding cost can bias a
border toward a river or ridge but can **never pin it to one** — a feature a few cells
off the tie-line is simply crossed. So:

1. **Cost-flood (Dijkstra)** — sets province COUNT, SIZE and TOPOLOGY. Seeds are the
   big settlements plus a habitability-scaled jittered scatter that prefers valleys.
2. **`snap_borders_to_features`** — a **marker-controlled watershed** (Meyer flooding)
   that re-places the border LINES. Erode each province by `SNAP_R`=3 → markers; build
   a relief of `crest + trunk river`, plus a low `FLAT_ANCHOR` ridge along the flood's
   own border so featureless terrain keeps the line it had; flood from the markers
   always taking the lowest relief. Two floods therefore meet on the highest ground
   between them — the crest, or the middle of the channel.

Three rules that are easy to get wrong here, all covered by `provinces::tests`:

- **The divider is a CREST, not an altitude.** `compute_ridge` scores how far a cell
  stands above BOTH sides along some axis, sampled at radii 2 and 4 on lightly blurred
  elevation. Absolute elevation (the old `(elev-0.26)*18`) makes a whole plateau
  uniformly expensive — so it prefers no border line at all — and gives sub-2300 m
  ranges exactly zero. Never sample prominence at ±1: the 3×3 blur spreads a narrow
  ridge over three columns, and a broad range is flat at its own summit.
- **Great rivers divide, small rivers unite.** Navigable/major trunks are a crossing
  PENALTY; every lesser river is a step-cost DISCOUNT along its own cell, so a province
  spreads through its valley and halts at the interfluves.
- **Charge river/lake crossings on the EDGE, not the cell.** A channel traced by
  following flow is an 8-connected staircase, and a diagonal step cuts clean between
  two of its cells without entering either — which made diagonal rivers free. Both
  floods inspect the two corner cells of a diagonal step.

Measured by the tests: borders sit on a crest **3.1×** more often than chance, and a
diagonal trunk river is a border **3.3×** more often (both ≈1.0× before). The partition
is also **deterministic** — `HashMap::iter().max_by_key` ties must stay broken on the
key, or the same seed yields different maps across runs.

### 8.11 Map label typography
Place names used to be styled at each call site (~23 separate `ctx.font =` lines), so
provinces and settlements came out in an IDENTICAL face and colour and water differed
from rock only by tint. Every place-name class now resolves through **one registry** in
`OverlayManager.ts`, the exact shape of the `lineColors` registry beside it:

```
LABEL_FONTS          4 system-font stacks (Windows → macOS → Linux). NO bundled fonts.
LABEL_STYLE_DEFAULTS per class: family · weight · italic · caps · tracking · color · size
labelStyles          the live registry the renderer reads each frame
setLabelStyles()     partial override, called by settingsStore
LABEL_THEMES         Mixed Contrast (default) · Classic Atlas · Engraved Antique · Modern Cartographic
                     · Atlas · Parchment · Game Board (the three MAP STYLES' own themes)
```

`LabelKey` covers province · settlement · river · lake · mountain · desert · forest ·
tundra · cultureRegion · peopleTerritory · tradeBasin. The default follows the atlas
convention — **nature is serif and leans, human works are sans and stand upright**.
Road names and river-break markers are deliberately NOT in the registry.

Each class also carries a `halo` ("" = the draw site's own dark halo); the light map
styles set a pale one. Technical note: tracking is drawn character by character because
`ctx.letterSpacing` is effectively Chromium-only and does nothing in the WebKit engines
Tauri uses on Linux and macOS.

Note `OverlayManager.rgba()` only understands `hsl()` and returns a hex unchanged — use
`labelAlpha()` for label colours, which are all hex.

---

### 8.12 Biomes — the ecological layer (phase 6b)

Köppen answers "what is the *climate* here"; it does not answer "what *grows*
here". `sim/step6_soil_fertility/biome.rs` turns the climate stack into an actual
vegetation map, writing the `biome` u8 tile column (**41 codes**, 0 = unclassified).

**Three axes.** Two are Holdridge's, which is the right tool for exactly this job:

- **Biotemperature** — the mean of the 12 monthly temperatures with each clamped
  to 0–30 °C: heat plants can actually use. Frozen months contribute nothing;
  heat above 30 °C buys no growth. Derived via `koppen::seasonal_temps` (the
  documented single source of truth for the warm/cold split), so the biome
  treeline agrees with Köppen's own C↔D boundary and the settlement winter gate.
- **PET ratio** = `58.93 · biotemp / annual precip`. 1.0 is the humid/dry hinge,
  >2 semi-arid, >4 arid. This is why a cold 300 mm cell comes out *steppe* while a
  hot 300 mm cell comes out *desert*, with no special-casing.
- **Seasonality** from Köppen's own `s`/`w`/`f` letter + `precip_summer_frac` —
  what separates Mediterranean sclerophyll from evergreen temperate forest.

**Precedence** (each overrides those below):
```
1. Cryosphere   permanent ice / nival rock — nothing grows, full stop
2. Azonal water mangrove · salt marsh · marsh · swamp · peat bog · gallery forest
                · oasis · floodplain · salt flat. LOCAL water beats regional
                climate — this is the layer that makes rivers & lakes LEGIBLE.
3. Altitudinal  above the CLIMATIC treeline (warmest month < 6.5 °C), so the
                treeline falls from ~4000 m in the tropics to sea level at the
                Arctic circle instead of sitting at a fixed elevation.
4. Zonal        the Holdridge/Köppen life zone for the cell's own climate.
```

Four rules that are easy to get wrong here, all covered by `biome::tests`:

- **The treeline is a TEMPERATURE, not an elevation.** The old layer used fixed
  0.40/0.62 normalized-elevation cutoffs, which put alpine tundra on warm tropical
  highlands and none on cold polar lowlands.
- **The riparian corridor's WIDTH scales with aridity.** A riverbank in rainforest
  looks like rainforest (reach 1, and the humid guard drops it entirely); in
  desert the gallery/oasis strip is the only green for a hundred km (reach 6).
  Repainting humid cells as "gallery forest" just adds noise.
- **Mangroves are frost-bounded** (coldest month ≥ 5 °C), which is why they stop
  near 30° on the real Earth; the same tidal flat carries salt marsh beyond that.
- **`TIDAL_MAX_M` is 80 m, not a true tidal elevation.** A cell is ~11 km across at
  the standard 3600×1800 grid (5.6 km on Large; `KM_EQUATOR / grid_w`), and a shore
  cell's elevation is a coarse average over that,
  so the honest question is "is this a low depositional coast" — tightening
  it toward real tidal heights empties the biome, since the elevation field's
  coastal taper rarely lands a shore cell below ~40 m.

**Descriptive only.** No later phase scores off `biome` — goods, fertility,
habitability and settlement placement are untouched, so re-running it cannot move
a city or a trade belt. Keep it that way unless FIX_PLAN B1 says otherwise.

**Rendering** (`render/tile_image.rs`): the Biomes layer is a per-biome colour
carrying a **procedural pattern fill**, in the tradition of a geological survey
sheet — canopy blobs for broadleaf, chevron spires for conifer, tussock ticks for
steppe, the standard horizontal dashes for marsh, sinusoidal ripples for a sand
sea, crevasse lines for glacier ice, a cracked lattice for a salt pan. Two rules:

- **Every pattern period must divide `TILE_SIZE` (128).** Patterns are functions
  of the cell's position WITHIN the tile, so a divisor period is what makes them
  line up tile-to-tile without the renderer knowing its world coordinates.
  `every_biome_pattern_tiles_seamlessly` asserts exactly this and is not
  decorative — it caught the dune ripple shipping at a 29.09-cell period
  (128/29.09 = 4.4), which would have drawn a seam across every sand sea. The
  corollary is that a fill's pseudo-random component REPEATS every 128 cells;
  that is correct for a cartographic hatch and invisible in practice.
- **Two biomes must be PERCEPTUALLY distinct, not merely unequal.**
  `biome_colors_are_distinct` used to check exact RGB equality, which passes
  happily on colours no reader can separate: tropical seasonal forest vs temperate
  deciduous shipped at ΔE 3.0 and thorn scrub vs chaparral at ΔE 3.8, and both
  pairs share a `Pattern` kind, so texture did not rescue them either. It now
  asserts a CIELAB floor (`MIN_DELTA_E`), set to what the palette already
  satisfies rather than to an aspiration — raise it as the palette earns it. The
  target for two biomes sharing a pattern is ΔE ≥ 8.
- **Contrast has a floor and a ceiling.** Under about 0.15 between mark and
  ground a pattern is technically present and visually absent (the first cut of
  the scrub dots and the marsh dashes both were); over ~0.20 it stops reading as
  texture and starts reading as a different colour, so two biomes blur together.
  `pattern_amplitude_stays_within_a_readable_band` holds the ceiling — it caught
  peat bog stacking its dash and hummock layers to 0.25.
- **Shading is a SEPARATE multiply applied after the pattern**, and the ceiling is
  held on the COMBINED swing (`pattern_and_relief_together_stay_readable`), not on
  the pattern alone — a test that measures only the pattern passes happily while two
  biomes blur. The four thematic plates (climate · biomes · soil · fertility) carry
  an attenuated hillshade (`THEMATIC_RELIEF_AMP` = 0.09) so a climate zone sits on
  the mountains that cause it, the way Bartholomew's and the Times' physical plates
  print tint over relief. It REPLACED the cruder `1.0 + (e − 0.2) · 0.18` elevation
  lift, which reached +0.144 alone and stacked on the same ±0.19 pattern — so the
  combined excursion went DOWN while the plate gained real form. Every shading layer
  now needs its neighbour ring in `tile_commands.rs`, or slopes break at tile edges.
- **They are SYMBOLS, not surface texture.** Holding a fixed pixel scale across
  the LOD pyramid is correct — that is how printed map hatching behaves.
- **`cargo test --lib render::tile_image::tests::dump_biome_swatch_sheet --
  --ignored --nocapture`** writes a swatch sheet + a tile-seam proof to
  `$BIOME_SHEET_DIR`, rendered through the real `render_tile` path. Use it to
  eyeball a palette or pattern change instead of guessing.

`BIOME_SWATCH` is **gone**: the legend now reads biome colours from
`get_render_palettes` (§8.18), so there is no second copy to keep in sync. Its
old doc-comment asked future editors to mirror `biome_color` by hand — three
colours changed in the very commit that deleted it, which is what that comment
could never prevent.
A world saved before this phase pads the column to zero and falls back to
`koppen_fallback_biome`, so the layer is never blank.

### 8.13 Cordillera elevation (`generate_elevation_cordillera`)

The third elevation MODEL, beside shape-based and ridged. `generate_elevation_ridged`
fills noise-defined orogenic belts with isotropic ridged multifractal: statistically
mountainous, but with no chain, no crest line, no consistent strike and no
rain-shadow side. A cordillera has all four, and they are what a map reader sees.

1. **A traced spine.** Crests are walked as polylines along an iso-contour of
   distance-from-coast — the walker steps perpendicular to ∇(coast distance),
   which is by construction parallel to the coastline, so the chain shadows the
   margin the way a subduction orogen does. Noise on the heading plus a slow
   drift of the target offset keep it off a mechanical offset curve.
2. **A continental divide.** The spine is continuous, so rivers part along it and
   the drainage map inherits a real watershed backbone.
3. **Asymmetric flanks.** Seaward: a steep scarp (falloff exponent 1.9) to a
   narrow coastal plain. Inland: a broad piedmont apron (exponent 3.2) over
   ~2–4× the distance. The side is decided by comparing a cell's own
   coast-distance to the crest's, carried outward by the BFS.
4. **Parallel sub-ranges.** A cross-strike cosine puts 1–3 sub-crests inside the
   envelope with intermontane basins between — Occidental / Central / Oriental.

Along-strike the crest tapers to nothing at both ends and undulates between, so
the chain emerges from the lowlands with summits and saddles rather than being a
uniform wall.

Guarded by `elevation::tests`: the crest's coast-distance **spread** must be
under 80 % of the ridged generator's on the same landmass (the real
chain-vs-noise discriminator — a plain connectivity metric saturates and does not
discriminate), the inland flank must average >10 % higher than the seaward at
equal distance, and the same seed must reproduce bit-identical elevation (the
spine tracer uses an RNG **and** a shuffle).

### 8.14 Settings preview (`step3_ocean_atmo/preview.rs`)

The planetary knobs are not self-explanatory — nobody knows what "rotation 0.6"
does to a map. Two previews make them legible, split by cost:

**Tier 1 · `zonal_profile`** — everything the knobs do that is purely a function
of LATITUDE: the EBM temperature curve (with Earth ghosted behind it, so the user
reads the ANOMALY), the seasonal envelope, and the belt edges. Two EBM solves plus
closed-form `Circulation` = microseconds, so the UI recomputes it on every drag.
It sees no land, so it can say how wide the desert BELT is but not where a desert
lands.

**Tier 2 · `coarse_climate_preview`** — the real phase-3 → Köppen chain on a
downsampled copy of the user's own landmass (≤600 cells wide, ~1/36 the cells of a
full run). Answers the question tier 1 cannot: where the deserts actually go, and
what share of THIS world each climate takes. Land/sea is downsampled by MAJORITY
VOTE, not point sampling — point sampling drops islands and severs isthmuses,
which changes the ocean circulation the preview exists to show.

Both build a throwaway in-memory `WorldBuffer` and never touch a tile, the DB, or
the world's stored planetary state; the knobs arrive as command ARGUMENTS so the UI
can preview values mid-drag, before they commit.

Three rules:

- **The phase-3 sequence is now duplicated in THREE places** — `sim_commands.rs`,
  `earth_validation.rs` and here. Change one, change all three (extends rule 11).
- **Archetype endpoints are MEASURED, not guessed.** `sweep_cooling_knobs`
  (ignored) walks the cooling knobs; the ice-albedo collapse is abrupt — alive at
  greenhouse 0.55 (−3.7 °C, 29 % polar), a total snowball at 0.50. `SNOWBALL_ICE_LINE`
  = 52° puts the warning one to two steps ahead of that cliff, and the Ice House
  archetype's strong end stops at 0.58 so its dial can never kill the world.
  `archetypes_deliver_what_their_blurbs_promise` asserts each archetype's headline
  claim, because the UI text makes promises the physics has to keep.
- **Absolute percentages from the test's synthetic slab mean nothing.** It is a
  flat rectangular continent with no orography, so its interior is far more arid
  than any real world; only the DELTAS between rows are meaningful. The real
  fidelity measure is the Earth scorecard (§2.3).

Adding the preview surfaced a live bug in `koppen.rs`: the lowland latitude
guardrails demote a cold low-latitude cell toward temperate, on the premise that
it can only be cold by the cold-current/upwelling over-cooling artefact. That
premise is false on a genuinely frozen world, where a snowball came out reading
72 % "temperate" at −44 °C. `FROZEN_WORLD_TEMP` (−12 °C) exempts genuinely frigid
cells; the value is the largest exemption that leaves the Earth scorecard
bit-identical, and `koppen::guardrail_tests` guards both sides of it.

### 8.15 The law of inheritance (`sim/shared/inheritance.rs`)

Two enums on the CULTURE, read at exactly one place (`succeed_house`), which between them
decide whether a merchant family accumulates across generations or is reconstituted at
every death:

* **`LineRule`** — Agnatic · AgnaticCognatic · Absolute · Enatic (who is eligible).
* **`InheritanceRule`** — Partible · Primogeniture · Ultimogeniture · Seniority ·
  Matrilineal (how the estate divides).

Assigned per language kit where the record is clear (Roman *sui heredes* took equal
shares; Celtic **tanistry** elected the eldest capable; the Mongol *otchigin* was the
hearth-keeping youngest) and drawn from that same distribution for a culture whose kit is
unknown, so a synthetic or legacy people still gets a historically-shaped law rather than a
hard-coded one. Resolved ONCE into `CampaignSim.culture_rules` and never re-rolled, so a
reloaded save cannot change a people's law.

Four rules for anyone changing this:

- **Matriliny is a seeded minority, not a named kit.** ~10% of peoples (22% of `Clannish`
  ones — kin-bound descent groups are the documented precondition) come out
  `Enatic + Matrilineal`, deterministically from the culture's own name. Assigning it to
  one named kit would be a factual claim about a real people this model cannot support.
- **The accession AGE is the mechanism, not the death age.** Ultimogeniture and
  primogeniture both concentrate; what separates them is that the hearth-keeping youngest
  takes over at ~17–31 and an elected tanist at ~44–62. Never "fix" tenure by touching the
  death age — that erases the difference between three of the five rules.
- **Only `Partible` divides.** Concentration is the ABSENCE of a split, and the gate
  asserts the other four never divide. A division moves capital from parent to co-heir and
  creates none; a co-heir inherits capital and **no fleet** (hulls it never paid for is the
  exact arithmetic behind the original 12-year house — see `docs/SCOREBOARD.md`).
- **Milestones are never pruned.** `HOUSE_EVENTS_CAP` prunes chatter (feud flares, lost
  caravans) oldest-first; `is_house_milestone` kinds — founding, succession, division,
  monopoly, charter, ruin — survive to `HOUSE_MILESTONE_CAP`. Before this a house in a hot
  feud lost its own founding within two years, which is not a cosmetic loss: for an
  observation-only game the chronicle is the product.

Gate: `cargo test --lib econ_inheritance_rules_fragment_differently -- --nocapture` runs
**THREE worlds** (`INHERITANCE_GATE_SEEDS = [42, 1337, 7]`, a prefix of the robustness
diagnostic's own 6-seed list chosen by position rather than by which seeds happen to
pass) four times each, changing only the law, and asserts the rule is wired on every
seed (partible divides, the rest do not) and that it MATTERS — **more houses ever
founded** (per seed, plus a ≥1.05× margin on the seed-AVERAGE — a single seed's ratio can
dip as low as 1.02, so the margin floor is checked on the average rather than per seed,
which would reject a real contrast for landing on an unlucky world) **and lower mean
wealth per house** (per seed — this one holds 6/6 in the full sweep, the stronger of the
two). Note what it does *not* claim: the top share and Gini do not fall under partible,
because a division adds small firms at the bottom as fast as it trims the top.
**Multi-seeded per `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` §10 Q1** — a single fixed
seed is what let realm formation, crisis relief, the trade horizon and
`COMFORT_IMPORT_FRAC` each perturb this gate in turn without the gate itself being able
to tell a real confounder from ordinary cross-world noise (see the cautionary tale
below). Runtime cost: ~6 minutes in debug (vs. ~2 for the old single-seed form) — the
full 6-seed sweep stays `#[ignore]`d for exactly that reason.

Two companions were added alongside. `a_division_moves_capital_and_creates_none` asserts
the zero-sum invariant AT `divide_estate`, where it is decidable, instead of inferring it
from an aggregate 60 years downstream. `econ_measure_inheritance_robustness` (`#[ignore]`d)
runs the partible/primogeniture pair across 6 seeds and reports how often each candidate
contrast holds.

**A CAUTIONARY TALE ABOUT THAT SWEEP — read it before trusting a robustness measurement.**
The mean-wealth assertion was once *removed* from this gate as "measurably false": the
6-seed sweep found it holding on only 1 seed. That conclusion was confident, documented at
length, and **wrong**. The sweep had been run while `COMFORT_IMPORT_FRAC` still sat at
`a7ff520`'s 0.60 — the very dose that had inverted this gate in the first place. Re-run at
the corrected 0.30:

| contrast | @0.60 (broken) | @0.30 (shipped, then) | @0.30 (shipped, 2026-09-11 re-measure) |
|---|---|---|---|
| more houses ever founded | 6/6 | 5/6 | **6/6** |
| more houses still standing | 4/6 | 2/6 | 5/6 |
| lower top share | 2/6 | 3/6 | 4/6 |
| **lower mean wealth per house** | **1/6** | **5/6** | **6/6** |
| no MORE capital in total | 1/6 | 5/6 | 4/6 |

The claim is real; the dose genuinely broke it. **A seed sweep only tells you about the
world you ran it in** — measuring robustness inside an already-distorted economy produced
a thorough, plausible, false conclusion ("the merchant pool is not conserved; firm count
is a multiplier on merchant wealth"), which is an artefact of the 0.60 dose and not a
property of the model. Before concluding an assertion is unsound, check that the world you
measured in is not itself the thing that is broken.

**The 2026-09-11 column is why the gate went multi-seed instead of staying at 5/6.** Both
of the gate's own assertions (houses-ever direction, mean-wealth direction) now measure
6/6 — a real improvement, most likely from Stage A/B's routing fixes landing between the
two measurements, though that was not isolated and re-confirmed by bisection. The lesson
generalises past this one dose: **a stale sweep is not a fact about the model, it is a
fact about the code and the dose at the time it was run** — re-measure before trusting an
old conclusion in EITHER direction, not only when a result looks suspicious.

**THE TWO GATES DISAGREE ABOUT THAT DOSE, and the one that set it isn't about trade.**
0.30 was chosen because it restores this gate — weak grounds for a demand parameter, so
it was measured against a gate that isn't the target (§2.4). Sweeping it against
`econ_fidelity_scorecard`: the basket price/distance gradient reads **−0.064 at the
shipped 0.30 (0 of 6 goods showing any gradient)** and turns **positive at 0.60 (+0.041,
2 of 6)** and 0.90 (+0.053, 3 of 6). A positive gradient is the historically correct sign
and its absence is `TRADE_AND_MARKET_REVIEW.md`'s F2 — the largest market failure this
project has named. So the shipped value is the worst of four tested on market integration.

**Nothing was changed**: raising it re-breaks this gate, and buying integration with a
demand constant is not the fix F2 asks for (it blames freight at ~11% of grain value over
the longest route, and i.i.d. per-hub harvest shocks leaving no regional scarcity). Full
table, caveats and reproduction recipe at the constant's own doc comment and in
`docs/SCOREBOARD.md` 2026-08-20d. Read the caveats before acting: one seed per dose, the
low end is not monotone, and every dose leaves basket CV far outside its band.

**Read this before "fixing" this gate again.** It has been perturbed five times (realms,
crisis relief, the trade horizon, estate-share tuning, comfort-good import demand). The
first three were genuine confounders and are isolated with `suppress_realms` /
`suppress_relief` / a widened `world_w`. The fifth was a real break in a real mechanism,
fixed at its source. Diagnose which of the three you have — confounder, broken mechanism,
or bad assertion — and be sure your instrument is not reading a world the bug already bent.

---

### 8.16 Ore deposits — the geology (`step8_biological_goods/deposits.rs`)

Ore geology is almost entirely a function of TECTONIC SETTING. That is the
organising principle of the discipline, not flavour — and until this module landed
the placer ignored it completely, deciding where a metal or gem went from an
absolute elevation floor plus a per-good noise field. Phase 1 already computes
`boundary_type`, `plate_index` and `is_volcanic`; phase 5 already computes rivers.
None of it was read.

A deposit good now carries a **`DepositModel`**, and the model decides placement:

| Model | Scored from | Type localities |
|---|---|---|
| `VolcanicArc` | volcanism + convergent proximity (~25 cells ≈ the real arc-trench gap) | Potosí, Cyprus, Rio Tinto, Almadén |
| `CollisionalOrogen` | high + rugged + convergent + **not** volcanic | Cornwall tin, the Erzgebirge, Muzo |
| `Craton` | far from ANY boundary + low relief + interior | **Clifford's Rule** — economic kimberlite occurs only over thick Archean lithosphere |
| `Rift` | divergent OR flood-basalt plateau | Kupferschiefer, the Deccan |
| `CarbonatePlatform` | low + flat + old shallow sea | Mendips lead, Silesia zinc |
| `ContactMetamorphic` | platform carbonate NEXT TO an orogen | Carrara, Mogok ruby, Sar-i-Sang lapis |
| `EvaporiteBasin` | Köppen B + low + flat | Wieliczka, Hallstatt |
| `Placer` | **derived** — walked downstream from a parent lode | Pactolus, Golconda, Ratnapura |
| `Bog` | wet + low + flat + near water | medieval N-European bog iron |
| `CoastalMarine` | shelf / beach / warm shallows | Baltic amber, Gulf pearls |
| `Weathering` | **derived** — supergene alteration of a parent, arid | Nishapur turquoise |

`deposits::default_model_for(id)` ships the correct model per mineral; a spec may
override via `DepositSpec.model` / `placer_frac` / `parent`, all serde-defaulted.

**The three-level hierarchy.** Real ore geology is described at three scales and the
old placer collapsed all three into one cell: metallogenic BELT (100–1000 km, the
setting field × per-mineral noise) → ore DISTRICT (10–60 km, `MIN_DISTRICT_SEP_KM`
= 320) → WORKING (1–10 km = one cell). A cell is already ~11 km, i.e. FINER than a
district, so clustering was never a cell-size problem — it was `min_sep = w*0.025`
≈ 1000 km between single cells.

**How MANY districts scales with LAND AREA** (`deposits::district_budget`). The count
used to be `richness × count_num / count_den` flat — about six to nine districts of a
metal for a whole planet whatever its size, where medieval Europe alone worked a dozen
great silver districts. It is now that product × `land_area_km2 /
DISTRICT_LAND_REF_KM2` (18 M km², stated in km² per rule 25), so an Earth-like land
area carries ~8× the old count; the per-mineral RATIO is untouched and still gated by
`deposit_counts_span_an_order_of_magnitude`. On top of the districts sit **minor
showings** (`SHOWINGS_PER_DISTRICT` = 2 per district, `MIN_SHOWING_SEP_KM` = 110): a
single WEAK/MODERATE, lower-grade working each, placed LAST on the same candidate
ground so the district and alluvial placement is untouched — the village lead pit
between the great camps. The Biological step's slider is now "Ore richness"
(6 = ×1.0), not a count. Measured (`deposit_census_diagnostic`, 600×300, 138 M km² of
land): 8,094 workings, silver 119 districts incl. showings (was 9), iron 629, tin 82,
lapis 24. Two things this made necessary: the district spacing test runs through a
`SpacingGrid` (exact same accept/reject as the all-pairs scan, gated by
`spacing_grid_matches_all_pairs`), and the campaign's per-(province, good) deposit
potential is built ONCE into `CampaignSim::deposit_potential_cache` — the direct walk
did an O(provinces) nearest-seat lookup per working per call, which at 10× the
workings would have dominated the yearly province goods pass. `goods_`'s printed
"wrong side of the coast" finding for `CoastalMarine` goods grows with the counts
(bay_salt 115 → 1,864 cells): the same pre-existing `deposits.rs` question, larger.

**Per-working state** the u8 belt cannot carry, persisted to `metadata["deposits"]`
exactly as the province list is: `grade` (→ the quality tier, so "tiers of gems"
becomes possible), `extent` (weak…world-class), `depth` (surface / shallow / deep /
**flooded**). The belt column is written as `grade × depth_workability`, so a deep
rich body is visible but largely LOCKED — inventory for a mining industry that does
not exist yet.

Five rules for anyone changing this:

- **A mineral must never silently vanish.** This is the failure mode this codebase
  keeps hitting (`highland_cap` exists for the same reason). Two guards: the
  threshold-loosening loop, and — when a world's plate geometry offers no ground at
  all for a model — a forced fall back to the relief proxy. That is not
  hypothetical: most divergent boundaries on any Earth-like world are OCEANIC, so
  keying `Rift` strictly on the boundary emptied it, and
  `no_shipped_mineral_places_nothing` caught it.
- **Template worlds have NO plate data** (`boundary_type` is empty). Every model
  degrades to a relief-and-continentality proxy;
  `template_world_without_plates_still_places` guards it.
- **Diamond belongs on flat cratons, not peaks.** The old spec said `min_elev: 0.55`
  — the highest mountains — which is exactly backwards.
  `diamond_lands_on_craton_not_on_peaks` guards it.
- **A derived model needs its parent placed first.** `Weathering` minerals are run
  in a SECOND pass after the main loop, rather than reordering it (which would
  change every other good's placement seed order).
- **`GeoContext` is built ONCE per world**, never per good — 45 goods × a full-grid
  BFS would be a real cost. Its distance fields are multi-source BFS (a linear
  sweep from all seeds at once), never an outward scan per cell (§8.9 rule 1).

**Slice 2 (built):** `economy.rs::compute_economy` now reads `metadata["deposits"]`
and attributes each working to the hub whose catchment claims its cell (the same
`claim` map the belt-production pass already builds), so a `Deposits`-distribution
good's quality is the mean `grade` of the workings actually inside a hub's
territory — not, as before, that hub's share of world production (which read
backwards: a big cheap deposit scored as fine stones). Every other good keeps the
old share-based formula.

**Slice 3 (built):** an INI-ish `.txt` importer (`commands/goods_import.rs`,
`import_goods_txt`, wired to a "Import .txt" button in the Goods Editor) adds
minerals to the global library — ADD-ONLY, an id already present is rejected, never
overwritten. Only `[id]` and `name` are required; `domain`/`distribution`/
`deposit_model` parse through the real enums' own serde representation, so the
parser can't disagree with the type. Eight new minerals ship in
`default_custom_goods()` itself (not through the importer — they're the app's own
library): mercury, alum, lapis_lazuli, turquoise, bog_iron, coal, garnet, carnelian
— each exercising a model or mechanic the shipped six/gem-split never did (a
near-single-source district count, a derived weathering mineral, a bog deposit an
elevation floor could never place). Full detail, including what is deliberately
NOT wired (mercury→silver amalgamation, alum→cloth as a hard recipe input — both
real economic changes that need their own `econ_` measurement, not an add-only
slice) in `docs/DEPOSITS_AND_MINING_PLAN.md` slice 3.

> **Slices 4-5, built:** a MINE (`estate_kind == 2`) carries `mine_depth`
> (the real working nearest its parent city) and digging a deep/flooded body
> costs real drainage capital to upgrade (`MINE_UPGRADE_COST_MULT`); mercury →
> silver amalgamation is a real consumable extraction input. A QUARRY
> (`estate_kind == 8`, split off from Mine) is gated by TRANSPORT instead
> (`QUARRY_INLAND_UPGRADE_COST_MULT`), never depth. A body KNOWN to be
> `EXTENT_WEAK` now declines to a floor under pressure (D3); everything else
> still persists. Mining SETTLEMENTS (the Potosí case,
> `maybe_found_mining_colony`) found on a real GREAT/WORLD-CLASS strike, boom,
> and DECLINE rather than die when their food lifeline fails. A settlement's
> trade catchment radius grows slowly with age (`catchment_radius_km`,
> derived, never stored). See `docs/DEPOSITS_AND_MINING_PLAN.md` slices 4-5.

---

### 8.17 Map plates (`ui/world/mapThemes.ts`)

A published atlas never shows one raster on its own. A climate plate is Köppen fill
PLUS the circulation that produces it PLUS the graticule, set in the face that plate
uses; a political plate is a province wash PLUS borders PLUS city dots sized by rank.
What distinguishes one plate from another is the whole COMPOSITION, not the base
colour. The app had 25 base layers and ~30 overlay toggles and no way to express that
— every informative view had to be assembled by hand, from memory, every time.

A `MapTheme` is that composition in state that already existed: one `ActiveLayer`, a
set of overlay keys, and optionally a label-typography theme (§8.11) and an
overlay-line preset. **Twelve plates ship**, ordered the way the pipeline builds the
world: Physical · Natural Colour · Relief & Height · Ocean & Currents · Climate · Hydrology · Ecology ·
Settlement · Peoples · Political · Goods & Trade · Hazards.

How it works today: applying a plate switches each overlay in `MANAGED_OVERLAYS`
(derived from the plates) on or off, any manual change clears `activeMapTheme`, and a
plate whose data isn't generated yet reads dimmed with the step it waits on.

**Map styles (2026-10-16)** are a second, independent axis: a plate says WHAT the map
shows, a map style says HOW it is drawn. `settingsStore.MAP_STYLES` — Classic · Atlas ·
Parchment · Game Board — each bundles a Rust terrain style (`ElevationStyle::Atlas`/
`Parchment`/`GameBoard`, with a drawn coastline and optional paper grain), a label
theme, a line palette and symbol styles (border colour/width, city-dot colour,
province-wash strength). Plates on the `land` or `terrain` base are drawn on the
style's terrain (`MapCanvas` layer key); while a style other than Classic is active,
plates no longer change labels or line inks. Switcher: Toolbar ▸ Plates.

The picker exists twice — a compact chip grid in the Toolbar and the full annotated
list in ⚙ Appearance ▸ Map plates. Both read the same `MAP_THEMES`, so they cannot
drift; only the presentation differs.

**The layer taxonomy was fixed in the same change.** `layerGroups` misfiled in both
directions: "Biosphere" held `climate` (Köppen is atmospheric), `soil` (pedosphere)
and `habitability` (a human settlement score), while "Ocean" held four biological
layers plus `storm`, which is a cyclone belt. The cause was under-population — only
TWO layers are genuinely biological and non-hazard, so the group was padded to a
plausible size with things that aren't biology. Six groups now: Terrain · Ocean ·
Atmosphere · Climate & Biomes · Settlement · Hazards. `ridges` is reachable for the
first time — it had always existed in `ActiveLayer` and in `render_tile`
(`render_ridges`) but belonged to no group.

---

### 8.18 The palette is served, not copied (`commands/palette_commands.rs`)

The legend used to keep **hand-maintained copies** of the renderer's colour tables —
four of them, across three files, none checked against the Rust that paints the
pixels. §8.12 already warned about this for `biome_color`/`BIOME_SWATCH`. They
drifted anyway, in two measured ways:

- **The Elevation layer's sea key ran BACKWARDS.** `ElevationLegend.SEA_BANDS` was
  copied from `render_land`'s bathymetry, but `render_elevation` drew its own ramp
  `(10+d·10, 25+d·30, 70+d·100)` — dark shelf *brightening* to abyss. Reading a
  deep-ocean colour off the map and looking it up in the key landed you on "Shelf".
- **The land bands implied a linear scale the ramp never had.** Six equal blocks
  labelled 0/1500/3000/5000/7000/8848 m described stops that actually fell at
  1327/3097/5309/7521 m.

`get_render_palettes` serves `ELEVATION_STOPS`, `BATHYMETRY_STOPS`,
`TEMPERATURE_STOPS`, `PRECIP_BANDS` and the Köppen/biome/soil class colours straight
out of `tile_image.rs`. **This removes the second copy rather than testing it** — the
legend cannot be wrong about the map without the map being wrong about itself.

Three rules:

- **Never reintroduce a hand-copied colour table in the frontend.** If a legend needs
  a colour, it comes through `usePaletteStore`. A test comparing two copies only
  catches drift after someone remembers to write it; having one copy cannot drift.
- **A ramp is DATA, not a chain of branches.** Every continuous ramp goes through
  `ramp_lookup` over a `(position, colour)` stop table, which is exactly what lets the
  same constants serve both the renderer and the legend. A ramp written as `if e <
  0.15 { … } else if …` cannot be served, and that is how the old drift started.
- **Position a legend's labels at each stop's TRUE value**, never at even intervals —
  even spacing is what made the old land key misreport by up to ~520 m.

**Cross-blended hypsometric tints** (§8.18 companion, `lowland_tint`): below
`LOWLAND_TINT_CEILING_M` (1200 m) the elevation/terrain tint also carries CLIMATE —
desert lowland reads khaki, rainforest green, tundra grey — converging on the shared
ramp above it (Patterson & Jenny, *Cartographic Perspectives* 69). The climate axes
are **temperature and precipitation, both continuous**, never the categorical Köppen
code: keying on Köppen would draw every class boundary as a hard colour edge and a
reader would take those edges for terrain, which is the artefact the technique exists
to avoid. Guarded by `cross_blended_tints_converge_with_height`, which asserts BOTH
halves — that three climates differ at sea level, and that they are bit-identical
above the ceiling. The legend must keep declaring this, or it misreports the lowland.

`LayerLegend.tsx` covers the six layers with an exact key (elevation · terrain ·
temperature · sst · precipitation · climate). The layers whose ramps are still
written inline in Rust are **deliberately left without a key** rather than given an
invented one — a legend that guesses is how this broke the first time. They are
served by the StatusBar hover readout, which reports the real value under the cursor.

**Swipe compare** (`uiStore.compareLayer`/`comparePos`): a second layer drawn over
the same ground, clipped to the right of a draggable divider. Every causal chain in
this app is a two-layer question — precipitation against elevation for rain shadow,
currents against temperature, biomes against Köppen — and they were previously
answered by flipping back and forth from memory. Two rules: the clip is computed in
WORLD space by converting the divider's screen fraction back through the viewport
(the canvas is mid-transform at that point, so a screen-space rect would be wrong),
and the divider is its OWN DOM element with its own pointer handlers, so it cannot
interfere with the canvas's pan/paint logic. The compare layer draws through the
same tile cache and LOD as the base, so a swipe costs one extra blit per visible
tile and nothing else.

**Class isolation** (`RenderCtx.isolate`): clicking a class in the Köppen or biome
key keeps that class in full colour and desaturates the rest. The selected code
rides in the LAYER KEY (`"biomes#iso=12"`), which is why it needed no cache change —
`TileManager` already keys by layer string, so an isolated view caches and
invalidates as its own layer, and a client that knows nothing about isolation still
asks for plain `"biomes"`. It is done in the RENDERER because only the renderer
knows each cell's class; matching colours back in canvas would be slow and, now that
the thematic plates carry relief shading, simply wrong. `split_isolate` degrades a
malformed key to the plain layer rather than erroring — a bad key must never blank
the map (`isolate_layer_keys_parse_or_degrade`).

Guarded in Rust by `koppen_colors_are_distinct` (Dsc and Dsd shipped IDENTICAL, so
two zones rendered as one), `elevation_ramp_is_monotone_in_lightness`,
`bathymetry_darkens_with_depth`, `precipitation_bands_are_sequential_and_never_neutral`
and `temperature_ramp_pivots_on_freezing`.

---

### 8.19 Goods localities — the agricultural/biological hierarchy

Trade goods get what minerals already had (§8.16): belt → LOCALITY → cell, the
same two-level structure `deposits.rs` uses, for every enabled `Global`/`Local`
good (`Deposits`/`Manufactured` goods are out of scope — F2's whole premise is
that minerals already have their own, better hierarchy).

- **Rivers as a placement factor** (Slice 1, F6). `biological::RiverContext`
  (built ONCE per world — the same discipline `deposits::GeoContext` already
  applies) carries distance-to-any-river, distance-to-NAVIGABLE-river and
  delta/floodplain membership, multi-source BFS (§8.9 rule 1). `river_multiplier`
  is a MULTIPLIER on an existing score, never a replacement (§5.4 of the plan) —
  `floodplain`/`irrigation`/`riverbank`/`float_out` weights, wired into the
  specific built-in goods the plan names (rice, cotton, wheat, sugar, indigo,
  dates, paper's papyrus branch, honey, hides, timber, hardwoods, furs) and
  exposed as four new `Envelope` fields (serde-defaulted to 0 — no effect) for
  custom goods. `good_score`/`envelope_score` both take `rc: Option<&RiverContext>`;
  `None` (the Goods Editor's live preview, which has no rivers to hand) is a true
  no-op, not an approximation.
- **Marine inshore/bank split** (Slice 2, F5). `GoodSpec.marine_band`
  (`Either`/`Inshore`/`Bank`, serde-defaulted to `Either`) narrows the old
  undifferentiated `sea_coastal` gate to a STRICT SUBSET of itself —
  `marine_band_ok` — so an `Either` good's placement is byte-identical to before
  this slice. Shipped defaults (`default_marine_band_for`): inshore = pearls,
  coral, bay_salt, tyrian_purple, amber; bank = stockfish, herring, whaling.
- **The locality generator + full modulation** (Slice 3, D1/D5/D6,
  `localities.rs`). `GoodLocality { good, x, y, radius_km, grade, extent, name,
  river_fed }` — deliberately the same shape as `deposits::Deposit`. The size
  ladder (§2.1 of the plan): luxury locality 175 km (wine, silk, spices, cacao,
  cloves, pepper) · pastoral/secondary 400 km (wool, hides, horses, timber,
  tobacco) · staple region 900 km (grain, rice, furs, barley, millet) — every
  other good falls back to a tier from its own `Distribution`/rarity, so nothing
  is left without an answer. Full modulation: `belt[i] = max(FLOOR, belt[i] *
  (FRINGE + (1-FRINGE)*influence[i]))` — `FLOOR` is the entire safety mechanism
  (D5's own risk 5.1): a belt cell already producing never falls to literal zero,
  however far it sits from every locality core. Runs BEFORE `dilate_belt` so the
  trade-reach rings spread from the already-modulated belt.
- **Notable naming** (Slice 4, D8). Localities at/above a quality threshold draw
  a deterministic name from `names::gen_name` — the SAME per-cell hearth lookup
  settlements already use, so a locality's name is in the local culture for free,
  no new naming machinery.
- **Persistence.** `metadata["good_localities"]` (JSON), exactly parallel to
  `metadata["deposits"]` — no tile-column change (rule 7). Written by
  `commands::sim_commands::persist_goods_placement`, the one helper all four
  `compute_trade_goods` call sites now share.
- **Production wiring** (Slice 7, D2). `compute_economy`
  (`commands/query_commands/economy.rs`) reads `good_localities` exactly the way
  it already reads `deposits` — a hub's quality for a good blends toward the mean
  grade of any locality inside its catchment (50% weight; `share`-based quality
  still carries the other half, since Slice 3's modulated belt VALUES already
  partly reflect locality quality — a blend, not a replacement, per D2).
  `campaign_province_potential` (`campaign_commands/province.rs`) exposes
  `ProvincePotential.localities: Vec<ProvinceLocalityDot>` alongside the existing
  `.deposits`, and `ProvinceGoodPotential` gained `has_locality`/
  `mean_locality_grade`/`locality_count`, mirrored in `types/campaign.ts`.
- **Slice 0's own gate**: `sim::step8_biological_goods::goods_validation`
  (test-only) builds a real, moderate-sized procedural world end-to-end (plates
  through biological — NOT the synthetic `CampaignSim` fixture `economy_
  validation.rs` uses) and asserts no enabled `Global`/`Local` good places a belt
  that reaches zero settlements' catchments. `Deposits`-distribution goods are
  explicitly OUT of that hard floor (F2 — they have their own, different
  coverage guarantee, `no_shipped_mineral_places_nothing`); a handful of the
  rarest deposit goods missing every settlement's catchment at this diagnostic's
  deliberately modest world size is printed as a FINDING, not asserted. One
  pre-existing belt good, `dyes` (murex purple, untouched by any Slice 1-4
  change — verified), is named as an explicit, documented exception rather than
  either silently loosening the floor or leaving the gate permanently red.
  **Do not add a process-global `cultures::set_active` call to a test** — `cargo
  test --lib` runs the suite in parallel within one process, and this diagnostic
  originally raced `econ_inheritance_rules_fragment_differently` /
  `econ_scorecard_is_deterministic` by mutating that exact global; it now relies
  on `names::gen_name`'s legacy-culture-grid fallback instead, which is
  deterministic on its own.

**The overlay (Slices 5-6) — two layers, at full resolution.** `compute_good_regions`
rasterised a belt onto a COARSE grid — `f = grid_w / 450`, about 8×8 world cells at
the default size — and the frontend filled each block. A block holding one land cell
of belt painted all sixty-four of its cells, so a belt's edge was a staircase that
ignored the coastline and spilled into the sea. No amount of shading could fix it: the
information was not in the payload.

Full resolution IS the coastline clip. `compute_good_belt_masks` copies the belt
column verbatim above `COVERAGE_MIN_U8` and consults no land mask at all, because a
`Global`/`Local` belt good's byte is already exactly zero on the wrong side of the
coast — every one is placed through `envelope_score`, whose first act is the domain
gate. `goods_validation::a_belt_never_crosses_the_coastline` asserts exactly that
claim, against the shared constant rather than a copy of it, because the render path
has no opinion about where the coast is and would silently start painting into the
ocean again if it stopped holding. The same test PRINTS a finding it deliberately
does not assert: `Distribution::Deposits` goods never touch `envelope_score` (§8.16
places them from tectonic setting), so `CoastalMarine` can put a salt pan a cell into
the tidal water — measured at 300×150 as bay_salt 115 cells, tyrian_purple 16,
ambergris 1. That is a `deposits.rs` question, not a render bug, and clamping it here
would hide it.

Two layers, split because they compress differently. COVERAGE ("can it grow here")
is boolean at full resolution and run-length encoded — a belt is contiguous, so a
world-spanning staple region costs a few thousand runs, not one per cell. QUALITY
("is it fine here") is the belt value on the OLD coarse grid, because a wash needs no
per-cell precision; it is painted only where coverage says so, so the coarse wash
still ends exactly on the coastline. Both are switched independently in the Toolbar's
Trade Goods section.

Four rules for anyone changing this:

- **The quality scale is ABSOLUTE and shared by every good** (D10). The belt's own
  0..1 value, never rescaled against that good's own maximum — otherwise a good whose
  whole belt is mediocre gets promoted to full colour just for being its own best. The
  stops live once, in `palette_commands.rs`, and are SERVED (§8.18). Until they
  arrive the overlay draws coverage only rather than inventing a ramp.
- **The scale has TWO paints, same numbers.** The default mixes toward the good's
  own colour, which is unreadable for a good whose hue is already close to the pale
  ground tint — a real user report. `GOOD_QUALITY_HEATMAP_STOPS` (dark blue → red)
  is an alternate reading of the identical belt value, switched per-session via
  `uiStore.goodQualityHeatmap` (Toolbar → Trade Goods → "🌡 heatmap", only shown
  while the quality layer itself is on). `GradeStop`/`good_quality_grades` serves
  the SAME `deposits::grade_label` vocabulary (coarse/ordinary/good/fine/exquisite)
  ore workings already use, at heatmap-exact colours, so `GoodQualityLegend`
  (`ui/world/LayerLegend.tsx`) can show swatches guaranteed to match the map. It
  docks to the opposite corner from the main `LayerLegend`, since it keys off the
  `goodQuality` OVERLAY rather than the active base layer and can be showing at the
  same time as either.
- **A very large belt is downsampled by MAJORITY, never by "any".** Above
  `MASK_MAX_PX` a canvas per good becomes real memory, so the mask reduces — and a
  reduce that takes a block when ANY sub-cell is covered puts the belt back over the
  water one block at a time, which is the original bug in miniature.
- **The FILL is exact; the BORDER is traced at a bounded step.** The fill is what
  carries the coastline. The outline is a stroke a pixel or two wide, so tracing it on
  millions of cells buys nothing and costs a Path2D with a hundred thousand segments.
- **Never re-derive the belt's shape from province polygons** (D3). A wine belt
  crossing three provinces is one belt; snapping it to political lines would make a
  physical fact look administrative.

`GOOD_MASK_DIR=/tmp/g cargo test --lib dump_good_belt_mask_sheet -- --ignored
--nocapture` writes a PNG per sampled good over the world's land/sea, through the
REAL `build_belt_mask` and the frontend's own decode — use it to answer "does the belt
end on the coastline" instead of arguing it, the same way `dump_biome_swatch_sheet`
serves the biome palette.

**The province plate (Slice 6)** is the other half. Land use and tenure are SHARES
and must dither (rule 17), but a `GoodLocality` is not a share — it carries a real
cell and a real `radius_km` — so it draws as a real square at its real position,
clipped to the province footprint, opacity from `grade` and hue from `GOOD_DEFS`. A
MARINE locality sits on shelf water where the province raster is `NO_PROVINCE`, so
`campaign_province_potential` attaches it to the nearest coast within
`MARINE_ATTACH_CELLS` and flags it `sea`: it draws dashed in the adjacent water, is
excluded from the province's own `has_locality` aggregate, and confers **no maritime
territory** (D4).

---

### 8.20 Goods: climate adherence, origins, endemics, terroir

Four changes to how a good gets onto the map, plus the report that makes a
failed placement visible. Full account (including this review's own three wrong
first readings) in `docs/WORLD_REALISM_REVIEW.md`.

**The `clim_base` fold is a FALLBACK, never a pre-pass.** `good_score` used to
fold the dry-winter/dry-summer Köppen variants onto their humid equivalents
BEFORE its match, which made every arm naming `CWA`/`CWB`/`CWC`/`DW*`/`DS*`
**unreachable**. Tea and coffee both name `CWB` — subtropical highland, dry
winter: Darjeeling, Yunnan, the Ethiopian highlands — and both scored exactly
**0.0** there, placed instead by weak fallback arms in the wrong climates. Now the
RAW zone is scored first and the fold applies only if it yields nothing.
`envelope_score` follows the same rule; before this, custom goods saw raw Köppen
while built-ins saw folded Köppen, so one cell was scored under two different
climate labels depending on which scorer ran. Gates:
`dry_winter_zones_are_reachable` (the claim) and
`the_humid_fold_still_applies_as_a_fallback` (the fix didn't cost the fold its
purpose).

**`GoodSpec.origins`** — how many INDEPENDENT homelands a `Local` good seeds
(serde-default 1 = the old single-homeland behaviour, so every save is
unchanged). Multi-origin is the historical norm: pepper from Malabar *and*
Sumatra, cinnamon from Ceylon *and* China, cotton independently domesticated in
India, Peru and the Levant. Each origin repels the earlier ones through the same
dispersion penalty already applied between different goods. Two rules: the
extreme-rarity homeland cap is a budget for the good AS A WHOLE, split between
origins (or a rare good doubles its own world supply); and only the FIRST origin
may fall back to the least-bad cell — a second origin that can't clear the
threshold simply doesn't exist on this world.

**`LandmassContext` + `Distribution::Endemic`** — the connected-component pass
this codebase never had. `Domain::Island` was `distance_to_ocean < 0.20`, i.e.
*near-coast land*, which matched the coastal fringe of every continent, so an
"island" good was really a coastal good. One 8-connected BFS, wrap-aware, built
ONCE per world (same discipline as `RiverContext`/`GeoContext`). An `Endemic`
good is confined to ONE landmass with the island-jump DISABLED — the Banda
nutmeg case: nutmeg grows across the wet tropics, but the tree only grew on ten
islands totalling 60 km², and `rarity` cannot express that (it makes a good
scarce *everywhere* rather than abundant in one place and absent elsewhere).
Six ship: nutmeg, mace (sharing an envelope — two products of one tree),
dragon's blood, camphor, benzoin, sandalwood.

Three rules learned by MEASUREMENT here, each a silent-vanish failure:
- **An island threshold must be stated in km², not cells** (`ISLAND_MAX_KM2`).
  A cell is ~11 km at 3600×1800 and ~133 km on a test world, so a fixed cell
  count meant "Great Britain" on one world and "most of Eurasia" on another.
- **DOMAIN and DISTRIBUTION must not both gate the same thing.** Shipping the
  endemics as `Domain::Island` zeroed their SCORE on every continental cell
  before the distribution could choose a home — all six measured **zero cells**.
  Domain says where the plant can grow (a wet tropical coast); distribution says
  how it is confined (the smallest landmass that scores, preferring a true
  island).
- **The endemic home is chosen up front, not filtered per cell.** That is what
  makes the guarantee unconditional: if the climate exists anywhere, the good
  gets exactly one home; if it exists nowhere it is honestly absent.
- **The chooser must REMEMBER what earlier endemics took.** It ranks candidates
  "smallest scoring landmass, preferring a true island" — a pure function of the
  world — and the six shipped endemics share a wet-tropical coastal envelope by
  design, so every one of them returned the SAME answer and the whole spice-island
  layer collapsed onto one rock. It now ranks `(claimed, island?, area)`: an island
  another endemic holds is the LAST resort. Still a preference, never a filter, so
  the coverage guarantee above is untouched. The EXCEPTION is the mechanism's point:
  nutmeg and mace are the aril and the seed of ONE tree and ship sharing an envelope
  deliberately, so `score_signature` (a fingerprint of the quantized suitability
  field) tells "one plant sold as two products" from "a different plant that likes
  the same weather" — no new spec field, no hard-coded pair table.
  Gated at the CHOOSER (`biological::tests::endemic_goods_take_different_islands`),
  not end-to-end: written against the 300×150 reference world the gate failed on the
  FIXED code, because at that resolution `ISLAND_MAX_KM2` puts the island threshold
  at 14 cells against a smallest landmass of 22, so the fixture qualifies ZERO
  islands and offers one candidate. A gate that fails identically either way measures
  the world, not the mechanism (§8.24c's own lesson). What remains in
  `goods_validation` is an honest diagnostic, `endemic_homelands_diagnostic`.

**`GoodSpec.soil` / `GoodSpec.relief` — the FINE-GRAIN terroir terms.** Every
other scoring term varies over HUNDREDS of km (Köppen zone, temperature,
precipitation, |latitude|, normalized elevation, fertility — itself a smoothed
blend), which is why a belt rendered as one smooth continent-sized wash: nothing
varied at the 2–10 km scale at which real crop distributions are mottled.
`soil_type` was computed by phase 6 and read by NOTHING in the goods layer;
slope was never computed. Three rules:
- **They live on the SPEC, not on `Envelope`.** They must apply to built-ins
  (scored by the hardcoded matcher, which has no Envelope) and customs alike;
  putting them in `Envelope` silently REPLACED a built-in's whole climate scorer
  with an envelope holding only these two terms.
- **Soil is a preference, never a veto.** An unclassified cell scores 1.0 (no
  information is not bad ground — the same discipline an empty kin roster gets);
  a classified-but-unlisted soil keeps `SOIL_UNLISTED`.
- **`TERROIR_FLOOR` remaps the whole multiplier into `[0.45, 1.0]`.** Applied
  raw it pushed `tea` and `saffron` under the seed threshold and both placed
  literally nothing. Terroir shapes a belt's TEXTURE; it must not decide whether
  the belt exists — the same call the locality pass's FRINGE/FLOOR already made.
  `saffron` is deliberately NOT in the terroir table: already bounded by climate,
  elevation AND latitude, one more gate moved it off every settlement's catchment
  and tripped the Slice 0 coverage floor.

**The two RENDER fixes that went with this.** Both were the same bug in two
places — a real per-cell field being drawn at a coarser resolution than it has:

- **The world quality overlay carried TWO resolutions.** Coverage was a
  full-resolution 0/1 RLE, but the quality wash clipped to it still rode the old
  ~8-cell coarse grid, so a belt read as blocky value steps inside a sharp
  coastline outline. `GoodBeltMask.coverage_rle` is now `quality_rle`: the SAME
  runs carry a 4-bit quantized belt value (0 = uncovered, 1..15 = quality), so
  coverage and quality are one layer at one resolution. Quantizing is what keeps
  the runs long — a belt varies smoothly, so neighbours share a bucket and the
  payload stays close to the old boolean RLE. `quality`/`qw`/`qh`/`coarse`
  survive only as the index space the per-block SUBTYPE classification
  (grain species / paper source) is addressed in. Gate:
  `quality_levels_never_swallow_a_covered_cell` (a covered cell may never
  quantize to 0, and the scale is monotone).
- **The province plate drew a locality as a true-to-scale square.** The size
  ladder is in km — a staple region is 900 km — while a province is 200-400 km
  across, so one grain locality filled the entire plate and the province read as
  one flat block. Where a belt MASK exists (plate 6a already draws the real
  per-cell area) the locality is now reduced to what only it can say: a small
  CORE diamond at its real cell plus its name. Where no mask exists (an older
  world, or a good with no belt here) the full square is still the honest
  reading and is kept.

**The placement report** (`GoodsPlacementReport`, `metadata["goods_report"]`,
served by `get_goods_report`, shown by `ui/goods/GoodsReportPanel.tsx` — opened
automatically when the Biological step finishes, and reopenable from that step
afterwards since it is persisted) — per good: cells, land share, origins actually
seeded, localities, notable names, mean grade. Its reason for existing is the
FLAGS: `absent` (placed nothing), `fallback_seed` (placed only because the seeder
fell back to the least-bad cell — this world may have no suitable climate),
`ubiquitous` (a non-staple over 25% of the world), `single_cell`. Before it, a
good that silently failed to place was invisible until someone went looking for
it on the map.

**Retired as duplicates** (disabled, never deleted — fixed indices in
`TileData.goods`, rule 7): `gemstones` (a generic gem alongside ELEVEN specific
stones, and §8.16 already repurposed `gem_deposits` to mean ore districts) and
`dyes` (marine murex, i.e. the same product as `tyrian_purple`, and the one good
`goods_validation` carried as a standing coverage-floor exception).

---

### 8.21 Natural colour & the shared relief core (`render/natural.rs`)

The `natural` layer is the reference-atlas view: **land coloured by what grows on
it**, sea by depth, both shaded. It reads `biome` (41 classes, §8.12), elevation,
temperature, precipitation and `snow_frac` — all already computed — and adds no
tile column and no sim phase. Gated on step 6 like the biomes plate; a world
without biomes falls back to a continuous climate tint rather than to blank, the
same discipline as `koppen_fallback_biome`.

**It does not reuse `biome_color`, and that is the whole design.** That table is
THEMATIC: `biome_colors_are_distinct` holds all 41 classes apart at a CIELAB floor
so a reader can separate them in a legend. Natural colour wants the opposite —
real vegetation grades continuously, and a palette engineered for class separation
renders a satellite-style map as a discretised poster with a visible edge at every
biome boundary. So there is a second, deliberately OVERLAPPING table, and
`neighbouring_vegetation_classes_stay_close` asserts that overlap so a
well-meaning future "improve the contrast" edit fails loudly. Class edges are
further dissolved by mixing back a continuous temperature/precipitation tint —
the same trick and the same reasoning as `lowland_tint` (§8.18).

**The shared relief core** (`tile_image::relief_at`) now serves both shaded base
layers. Three changes from the single-lamp Lambertian it replaced:

- **Ambient occlusion** from local concavity, which is orientation-INDEPENDENT, so
  a valley reads as a valley under any lighting.
- **Shaded bathymetry** (`sea_shade`). The sea was an unshaded ramp beside a
  hillshaded continent.
- **NOT a fill light.** One shipped and was REVERTED — see below.

Three things measured here, all of which look like tuning and are not:

- **`AO_REF` is the difference between ambient occlusion and film grain.** Set to a
  plausible-sounding 44 m ("valleys are shallow"), AO speckled the ENTIRE map:
  `apply_micro_relief` deliberately dithers every land cell by ±14 m, and against
  an 8-neighbour mean that per-cell dither *is* the concavity signal, so AO
  resolved to ±16% white noise per cell. Rendering with `AO_AMP = 0` isolated it
  conclusively. It is now 240 m; anything under ~150 m re-admits the grain.
- **A FILL LIGHT was shipped, and it was a REGRESSION.** The theory was Imhof's
  (one NW lamp leaves every SE-facing slope at a flat tone), and it survived
  `cargo check`, 20 render tests and a whole-world PNG — then read as obviously
  worse the moment anyone looked at a magnified mountain crop. A fill lamp
  brightens exactly the shadows that CARRY the relief. Four configurations
  rendered side by side (`SHEET_TAG`): no fill > 225°/0.18 > 135°/0.26, the last
  being what shipped. A fill light is a technique for a 3-D scene with real
  geometry; on a shaded DEM whose only signal IS the shadow it subtracts. The
  lesson generalises past this constant: **a whole-world thumbnail is not enough
  to judge shading** — `dump_natural_sheet` now also writes a 3× magnified crop of
  the most mountainous window, because the regression was invisible at world zoom
  and unmistakable at 3×.
- **A per-cell detail dither was built here and REMOVED.** A tile is exactly one
  pixel per cell, so there is no sub-cell space for synthetic detail to live in and
  it resolves as film grain rather than as terrain. Recorded so it is not
  attempted again: detail below the cell needs somewhere to be DRAWN — tiles
  rendered at higher pixel density than cell density, i.e. an inverse LOD pyramid.
  No shading trick substitutes for the missing raster.
- **The bathymetry ramp's dark end was a void.** It fell to (5,15,46) by depth 0.65
  and near-black at 1.0, while `compute_sea_depth` saturates about 20 cells from
  any coast — so on a full-size world essentially ALL open water sat in the black
  tail, and the new seafloor shading had nothing to modulate. Rebalanced to keep
  tone all the way down, still strictly monotone in luminance.

**`NATURAL_SHEET_DIR=/tmp/n cargo test --lib dump_natural_sheet -- --ignored
--nocapture`** builds a REAL world (plates through biomes, mirroring `sim_run_all`
— rule 11) and renders it through the real `render_tile` path, writing
`world_natural.png` and `world_terrain.png` plus a per-biome census. Use it to look
at a palette or shading change instead of arguing about it — it is what caught both
the AO grain and the black ocean, neither of which any assertion would have found.

What it also makes unmissable is that the remaining problems are in the SIM, not
the renderer: land relief is texturally uniform because `generate_elevation`
uses one global noise recipe (`f_base`/`f_range`/`f_hill`, `RIDGE_AMP`,
`HILL_AMP`) for every continent. (The coastline-follows-Voronoi-edge and
straight-plate-boundary-ridge defects this paragraph used to name here were
Terrain 2.0's own slices 3-4 (shipped; §8.23b) — fixed and gated
by `coastline_departs_from_the_plate_boundary`.)

---

### 8.22 Elevation styles (`render/tile_image.rs`)

The default "elevation" layer is a flat, UNSHADED hypsometric tint and
"terrain" is the one shaded hillshade — two names for what an atlas treats as
one decision (how to paint height) times two independent axes (which palette,
how much relief). A STYLE is that pair, selected in the layer key exactly like
class isolation (`split_isolate`, `"biomes#iso=12"`):
`"elevation#style=alpine"` rides the same string the frontend's tile cache
already keys on, so a styled request caches, invalidates and degrades (an
unknown style name falls back to the plain layer) with zero cache-layer
change. Parsed by `split_style`, dispatched by `render_elevation_styled` —
ONE function for every style, keyed entirely off a `StyleParams` struct (land
+ sea ramp, classed-vs-smooth, climate-tint strength, real-`snow_frac` blend
strength, AO/contrast/shadow-floor/light-altitude, warm-vs-cool shadow tint,
sea relief amplitude), so a new style is DATA, not a new render function.

Seven ship, each a real cartographic convention rather than a colour
experiment: **Layer Colouring** (classed Bartholomew/Times hypsometric bands —
also what `mapThemes.ts`'s "Relief & Height" plate now uses, giving it a real
identity distinct from "Physical" instead of reusing the flat unshaded
default); **Alpine** (Imhof's neutral Swiss relief-forward palette, reading
the world's own per-cell `snow_frac` field for its snowcap rather than
inferring it from height alone); **Arid** (warm sand/canyon palette, a lower
key-light altitude for longer raking desert shadows, a sepia shadow tint);
**Polar** (cool ice-blue palette, a very low key-light altitude, strong
`snow_frac` blending); **Analytical** (monochrome — colour carries ZERO
elevation information, relief alone does, maximum AO for a scientific
hillshade); **Antique Plate** (sepia/parchment engraved-atlas look, classed
bands, a warm shadow tint); **Abyssal** (a bathymetry SHOWCASE — land muted
to a low-contrast neutral so it recedes, `sea_relief_amp` boosted well past
the default so Terrain 2.0 slice 5's ridges/trenches/seamounts read as the
map's actual subject).

Two rules:

- **Served, not copied** (§8.18 applies here exactly as it does to every other
  ramp). `elevation_style_palettes()` exposes each style's own land+sea stops;
  `get_render_palettes()` serves them as `elevation_styles` so
  `LayerLegend.tsx`'s elevation legend swaps to the ACTIVE style's real ramp
  (and its `classed` flag, so a stepped style draws a stepped legend) rather
  than guessing a second copy.
- **A style is a VIEW, exactly like a map plate** (rule 14/§8.17) — it changes
  what the elevation/terrain layers look like, never what the world IS.
  `elevationStyle` lives in `uiStore` beside `isolateClass`, rides the layer
  key the same way, and is cleared by any manual change (§8.17's third rule)
  so a picked style can't silently survive switching to an unrelated plate.

`cargo test --release --lib dump_elevation_style_sheet -- --ignored
--nocapture` (env `ELEVATION_STYLE_SHEET_DIR`, `ELEVATION_STYLE_SEED`) builds
one real generated world and renders it through EVERY style via the real
`render_tile_full` dispatch — one full-world PNG per style plus the two
default baselines, and a numbered contact-sheet montage — the same
"render it for real, don't argue about it" discipline `dump_natural_sheet`
and `dump_biome_swatch_sheet` already established.

---

### 8.23 No valley carving — and why three attempts at it failed

**Phase 2 does not carve valleys. In any generator, by any mechanism.** Three
separate attempts to make channel-carving look right on an Earth-sized world all
failed, and the record of them is the point of this section — the fourth attempt
should not start from scratch.

What was removed, and what each drew on the map:

- **`carve` / `fine_carve`** — an INVERTED ridged-multifractal field subtracted
  from the elevation. An inverted ridged field is a **dendritic tree by
  construction**, so this drew a branching dark scratch network across every
  continent. `fine_carve` was the worst because it was an ABSOLUTE subtraction:
  full strength on flat plains, which is exactly where a drainage tree painted
  over an otherwise smooth interior is most obvious.
- **`stream_power_erosion`** — priority-flood + flow accumulation + `K·A^m·S^n`
  (Whipple & Tucker 1999). Correct landscape-evolution physics at the resolution
  it is meant for, and wrong here: a cell is `KM_EQUATOR / w` wide (11 km on the
  default grid), so the valley it models is SUB-GRID — the Grand Canyon is 16 km
  across and would not fill one cell. At cell resolution it cut one-cell trenches
  down every D8 path plus the parallel single-cell rills D8 routing always makes
  on a planar slope.

**The three attempts, in order, so the pattern is visible:**

| | what it changed | why it failed |
|---|---|---|
| `3943136` | noise `carve` 0.16 → 0.05, thermal slump strengthened | aimed at the wrong term — measured per stage, the noise carve leaves notch density at 0.05% of land |
| `e66dc9f` | stream power scaled + spread to its physical scale; thermal scan-order fixed; a grid-scale relief budget | cut grid-scale texture 82–93% **and the map still showed the tree** — subtler lines are still lines |
| *this* | every carving mechanism deleted | — |

The lesson the second attempt paid for: **the STRUCTURE is what reads as wrong,
not the amplitude.** A dendritic pattern at 40 m is still a dendritic pattern.
Damping it is not the same as not drawing it.

**A measurement caveat that matters more than the fix**, because it is why the
statistics said "fixed" while the map plainly was not: `notch_metrics`'s 120 m
threshold is calibrated to the HILLSHADE (half of `AO_REF`), but the flat
`elevation` tint has no shading at all, and a systematic pattern shows there at a
far shallower amplitude. **Judge a tint layer on a tint render, not on a
hillshade statistic.** `dump_erosion_sheet` now writes `elevation` alongside the
analytical hillshade, and takes `EROSION_MODEL=shape` for the TEMPLATE path —
which is what anyone who imported a real-world coastline is actually running, a
different generator with its own carve terms.

**A second negative result, recorded so it is not built into a gate:**
`largest_notch_component` (the largest 8-connected chain of notch cells — meant
to separate "rough" from "a drawn tree") **does not discriminate on the plate
model**. Re-adding `fine_carve` to a clean build measures 0.110 / 0.177 / 0.287%
at 60 / 25 / 12 m thresholds against a clean 0.122 / 0.186 / 0.299% — carved
reads *lower* at every threshold. `limit_grid_scale_relief` normalises the
one-cell band to a fixed RMS, so no amplitude statistic downstream of it can see
structure. There is therefore **no cheap statistical gate for this**, and none
was shipped: a gate that passes either way is worse than none.

**What remains** is `thermal_erosion` — hillslope slumping, which only ROUNDS and
never incises, and whose scan-order fix and gate from the second attempt are kept
— plus `limit_grid_scale_relief` (below). Relief comes from the noise stack and
the tectonic terms.

**Rivers are unaffected.** Phase 5 runs its own priority-flood fill and derives
channels from the finished surface; it never needed pre-cut channels to bed into.
`priority_flood_flow` in `elevation.rs` is now `#[cfg(test)]`, kept only so
`terrain_metrics` can still report drainage density.

---

### 8.23b The grid-scale relief budget — relief at the scale a cell can hold

Every world's grid spans Earth's equator, so a cell is `KM_EQUATOR / w` wide: 11 km
on the default 3600×1800 grid. **Fluvial dissection is therefore SUB-GRID** — the
Grand Canyon is 16 km across and would not fill one cell — and what a cell can
honestly record is the MEAN lowering over its whole footprint, which is both far
shallower than the thalweg and spatially smooth.

Phase 2 recorded neither, and the map showed it: measured on a plate world at
1800×900, **2.77% of land sat more than 120 m below its own 8-neighbour mean,
averaging 346 m** — a 22-km-wide, 350-m-deep slot — with one-cell RMS concavity at
**102.7 m** against `AO_REF`'s 240 m. Rendered, that is a dendritic scratch network
over the whole world plus regular comb striations on every flank: the "too thin
lines, looks like river erosion" report this section answers.

**Three causes, and only one of them was the obvious one.** The noise `carve` /
`fine_carve` terms — cut hard by an earlier attempt at the same complaint
(`3943136`, 0.16 → 0.05) — are NOT a cause: measured per stage they leave notch
density at 0.05%, and removing them entirely makes the finished figure WORSE
(93 → 112 m), because the hypsometric redistribution equalises whatever histogram
it is given. Do not reach for those knobs again.

1. **Stream power cut one-cell trenches.** `K·A^m·S^n` incision applied straight
   down the D8 path, plus the parallel single-cell rills D8 routing always produces
   on a planar slope. Incision is now scaled by `FLUVIAL_VALLEY_KM / km_per_cell`
   (floored at `FLUVIAL_MIN_RECORDED` so it can never silently vanish on a coarse
   grid) and SPREAD over `FLUVIAL_SPREAD_KM` — **inside each pass**, so the next
   pass re-routes over the broadened valley instead of re-cutting the same rill
   deeper, which is what made the combing so regular. It redistributes the carve
   rather than deleting it, so the denudation budget and `isostatic_adjust`'s
   rebound are unchanged in kind.
2. **`thermal_erosion` depended on scan order.** It wrote both donor and recipient
   in place while scanning rows top-to-bottom, so a cell was slumped into before it
   was itself visited. Now a simultaneous delta-buffer update.
3. **The finished field carried more one-cell content than the grid can hold.**
   `redistribute_elevation` multiplies landform relief ×7.7 and grid-scale relief
   ×8.9, so whatever the noise stack leaves is what the map is textured with.
   `limit_grid_scale_relief` caps the one-cell band at `GRID_RELIEF_BUDGET_M`.

**The result** (one-cell RMS concavity · notch density · landform relief, @1800×900):

| model | before | after |
|---|---|---|
| plates | 102.7 m · 2.77% · 2178 m | **18.7 m · 0.22% · 2176 m** |
| shape | 98.6 m · 3.84% · 2261 m | 20.5 m · 0.28% · 2267 m |
| cordillera | 340.2 m · 7.83% · 2084 m | 22.9 m · 0.28% · 2132 m |
| ridged | 125.8 m · 3.69% · 2249 m | 19.3 m · 0.26% · 2253 m |

Grid-scale texture down 82–93%; **landform relief moves under 2% on every model**.
That pair is the whole claim — a limiter that also flattened the mountains would
"succeed" on the first number and ruin the map.

- **Look at it.** `EROSION_SHEET_DIR=/tmp/e cargo test --release --lib
  dump_erosion_sheet -- --ignored --nocapture` renders a real world through the
  REAL hillshade in the monochrome `analytical` style (colour carries zero
  information there, so what is left on the page is exactly the shading) plus a 4×
  crop. Every finding here came from that sheet and none from reading the code;
  `erosion_texture_metrics` is the numeric companion. Same discipline as
  `dump_natural_sheet` (§8.21) and for the same reason.
- **A local mean near a coast must be LAND-ONLY.** A plain box blur averages a
  3000 m coastal cell against sea held at 0, calls the coastline itself "detail",
  and then planes the coast DOWN toward sea level — drawing the exact hard dark rim
  the budget exists to remove.
- **`e' = m + k(e−m)` is not idempotent.** It leaves `(1−k)(m − blur m)` behind, so
  one pass aimed at 16 m settled at **41 m** on a real world. `limit_grid_scale_relief`
  iterates to its fixed point; the pass cap is a termination guarantee, not the
  mechanism.
- **A gate needs a fixture that can fail.** The (now removed) fluvial gate first
  used a tilted plane and **passed on the unfixed code** — a uniform slope drains
  every cell alike, so its carve is broad however it is applied. The surviving
  scan-order gate was verified the same way: revert the fix, watch it fail.
- **The budget is set below what the variance argument alone allows, deliberately.**
  Scaling real continental relief to a 22 km cell suggests a legitimate one-cell
  residual nearer 60 m. Ours cannot be spent that way because our one-cell content is
  UNCORRELATED between neighbours (independent noise at the top of an fbm stack)
  while Earth's is STRUCTURED — part of a cascade whose ridges continue across cells.
  Same RMS, completely different reading: noise looks like grain, structure looks
  like terrain. Buying that headroom back means generating structured sub-grid
  detail (a real multifractal cascade, or erosion run finer and averaged down) —
  a terrain-generation change, not a shading one, and not built.

**NEGATIVE RESULT, so it is not attempted again:** `redistribute_elevation` assigns
each cell a height by its RANK, whose ties break on the row-major index — which
looks exactly like the cause of one-row striping. Rewriting it as a monotone
value-transfer curve measured **bit-identical to the displayed precision** and was
reverted. With ~500k distinct f32 elevations there are no ties to break, so the rank
map already IS a monotone curve; the striping was ordinary high-frequency content,
and `axis_curvature` (curvY/curvX ≈ 0.94) proved the field isotropic before any of
this was changed.

**Still open, named rather than quietly fixed:** a 1–2 cell dark lineament survives
every erosion ablation — it is in the base tectonic/noise field, not the erosion —
and the hypsometric target puts implausibly much land above 6 km.

`bench_phase2` @3600×1800 went plates 11.4 → 12.5 s, shape 13.9 → 14.5 s (+9% / +4%)
after rayon-parallelising `box_blur_wrap`, which the new passes made hot.

**The Earth fidelity gate cannot move here, by construction**: `earth_validation.rs`
scores against the baked GMT DEM and never calls `generate_elevation`. Verified
unchanged at 70.2 / 39.0 all the same.

`limit_grid_scale_relief` only ever SMOOTHS (a world already inside budget returns
bit-identical) and touches only the one-cell band; it is self-calibrating in the
same spirit as `need_scale` / `prov_good_yield_scale`, and runs BEFORE
`apply_micro_relief` so the deliberate ±14 m dither survives.

---

### 8.24 Physiographic provinces & the hypsometric target

Removing the drawn-in drainage texture (§8.23) exposed what was underneath it:
**one mottled cloud over every continent** — no plains, no plateaus, no basins,
no coherent ranges. Two separate causes, both measured.

**1 · One noise recipe for the whole world.** `generate_elevation` used fixed
frequencies and fixed amplitude weights in every cell. Real topography is a
mosaic of PHYSIOGRAPHIC PROVINCES — the Great Plains, the Colorado Plateau, the
Basin and Range, the Canadian Shield are adjacent, ~1000–3000 km across, and each
has its own relief, roughness and characteristic wavelength.

`landform.rs` supplies that mosaic: a jittered, domain-warped lattice of sites at
`PROVINCE_KM` (1900 km, stated in km per rule 25), each assigned one of seven
archetypes — plain · shield · hills · upland · massif · plateau · basin — and
each archetype carrying `amp` (local relief), `rugged` (ridged vs smooth) and
`detail` (weight on the short-wavelength term). Wired into the plate model and
the template model, the two defaults; `ridged` and `cordillera` are deliberate
stylistic models and are left alone.

Four things that are easy to get wrong here, three of them found by rendering:

- **The archetype comes from TECTONIC CONTEXT, not a die roll.** A massif sits
  near an orogenic belt, a shield deep in a stable interior. A purely random
  mosaic is varied and incoherent — mountains in the middle of cratons.
- **`detail` is a WEIGHT, never a frequency multiplier.** Scaling a noise
  function's coordinates by a spatially-varying factor is not a smooth
  reparametrisation: the sample position jumps as the factor changes, and the
  result is concentric moiré rings wherever it varies. In the hillshade they read
  exactly like contour terracing. Blending two fields at FIXED frequencies gives
  the same "broad versus busy" axis with no artefact. The smooth companion to the
  ridged field is a `swell` fbm at the same wavelength, so "not rugged" is a real
  landform rather than merely less ridge.
- **Assign hard, then BLUR the parameter fields — do not blend the two nearest
  sites.** A two-site blend still creases wherever the nearest-site IDENTITY
  changes, and where three provinces meet it creases along every bisector at
  once: the rendered map came out crazed with a polygonal crack network, like
  dried mud. A box blur of a piecewise-constant field is continuous everywhere.
  `amp`/`rugged`/`detail` blur over a quarter of a province; `terrace`/`bowl`
  over a twentieth, which is what keeps an escarpment an escarpment.
- **Shaping runs AFTER the hypsometric redistribution.** `redistribute_elevation`
  is a rank remap, so a flat plateau built before it is simply un-flattened, its
  tied cells fanned back across a band. Amplitude and roughness DO survive it
  (they change which cells rank high); flattening and depressing do not.

**2 · The hypsometric target was wrong by a factor of forty.** At the default
`height`, the old anchors put **~21% of land above 4000 m** and only ~38% below
1000 m. Earth is 0.5% and 71%. Every world came out a pale high plateau with the
tint ramp saturated at its top end, which buried whatever relief was underneath —
including all the province variety above. The anchors are now set so the midpoint
lands on the real ETOPO row (**71% below 1 km, 2.1% above 4 km**), and the
`density` shift TAPERS up the curve instead of spreading evenly, which had taken
the alpine end to 9.2% above 4 km. Gated by `the_default_hypsometry_resembles_earth`,
which asserts the shipped `(0.5, 0.5)` default, not just the tidy one.

Mean landform relief on a plate world fell 2174 → 1291 m as a result — the land
was averaging nearly 3× too high, and every downstream consumer (lapse-rate
temperature, biomes, habitability, settlement placement) was reading it.

**3 · The shoreline was being shaded as a cliff** (`render/tile_image.rs`). A sea
cell stores `elevation = 0`, so a coastal land cell shaded against its raw
neighbours saw a drop of its own full height over one cell — 8848 m at the
extreme — and every coastline was ringed with a hard bright/dark rim unrelated to
the land's slope. It also swamped the AO term, which read the same step as
extreme convexity. `halo_terrain`/`land_elev_at` substitute the centre cell's own
height for a sea neighbour in BOTH shading paths (`relief_at_params` and
`thematic_relief`), so coastal land shades by the land's gradient and the water
is left to `sea_shade`, which carries its own correctly-scaled seafloor relief.

Gates: `provinces_give_a_world_genuinely_different_country` (relief amplitude and
ruggedness must actually vary across a world — a neutral field scores exactly 0),
`province_character_never_steps_between_neighbouring_cells` (the crack-network
regression, bounded against the character table's own range rather than a bare
number, since the gradient of a blurred field scales with cell size),
`a_world_too_small_for_provinces_is_exactly_neutral`,
`the_province_mosaic_is_deterministic`, `the_default_hypsometry_resembles_earth`.

`bench_phase2` @3600×1800: plates 12.5 → 13.6 s, shape 14.5 → 14.8 s — one extra
fbm field per cell plus the province build.

**Still open, named rather than quietly fixed:** the 1–2 cell dark lineament of
§8.23b survives (base tectonic/noise field, not erosion); short vertical hatch
marks appear in shelf water near some coasts; and plateau/basin shaping is
present but subtle — a plateau reads as a level, not yet as a landform you would
name.

---

### 8.24a2 Plates of genuinely different size, a motion layer, collision style, and relict sutures

Four Part B pieces of `TECTONICS_AND_ISOLATION_PLAN.md`, all landed.

**B1 — plate SIZE CLASSES.** The jittered-grid seeding made every plate roughly
the same size by construction (one grid cell of territory each), unlike Earth's
own three-orders-of-magnitude spread (Pacific ~103M km² vs Juan de Fuca ~0.25M
km²). Plates now draw from a four-class ladder (giant/large/medium/small,
`SIZE_CLASS_WEIGHTS`/`SIZE_CLASS_PROPORTIONS`, Earth's rough 10/25/40/25 mix)
feeding a **power diagram**, not a plain nearest-seed Voronoi.

**NEGATIVE RESULT** (§2.4): the first cut used a textbook *multiplicatively*
weighted Voronoi (divide squared distance by weight²). Measured on the gate's
own 8-plate world, a small plate's territory could come out split into
DISCONNECTED islands even with the domain warp turned OFF entirely — proving
the fault was the weighting metric itself, not the warp on top of it. A
multiplicative metric is not a true distance (the triangle inequality can
fail), so its cells are not guaranteed connected; at a modest plate count and
weight ratio, a small plate boxed in by bigger neighbours was pinched apart by
construction. The fix: `d² − offset`, an ADDITIVE term (a real power diagram /
Laguerre-Voronoi), whose cells are provably convex — hence always connected —
for ANY offset at ANY plate count, a mathematical guarantee rather than a tuned
approximation. `PLATE_WARP_AMP_FRAC_WEIGHTED` still trims the warp specifically
for the weighted case, since convexity holds only *before* the warp bends a
cell, and full amplitude can still sever a thin part of one (measured: 0.25
reproduces the old failure, 0.08 does not). Gated by `plate_territory_stays_
connected` (reinstated on the real `generate_plates_and_landmass` path) and
`plate_sizes_span_an_order_of_magnitude` (≥5× largest/median area, shipped
mean 7.74×).

**A regression B1 caused and a broader test sweep caught, not the routing
table.** The oceanic/continental fill (Terrain 2.0 slice 5) picks plates as
oceanic to hit `ocean_fraction` BY CONSTRUCTION, from one shuffled greedy pass
over real per-plate cell counts. That was reliable when plates were all
roughly the same size; once B1 made sizes span an order of magnitude, a
single shuffle order could get stuck far from the target — measured at 6
plates, one seed, 52% land against a 30% target, a >20pt miss
(`land_fraction_tracks_the_target`, Part I Slice 5's own gate). It lives in
`elevation::plate_diagnostic`, not `plates::tests`, so B1's own verification
pass never ran it. Fixed by running `OCEAN_FILL_TRIALS` (24) independent
shuffle orders — still the same greedy step, still deterministic per seed —
and keeping whichever came closest to the target ocean-cell count, rather
than betting the whole result on the first random order. The lesson: a
change that widens the INPUT distribution to an existing mechanism can break
that mechanism even when the mechanism's own code is untouched — worth
re-running the wider `elevation::`/`plates::` sweep, not just the module a
change appears to touch, after any plate-size or plate-count change.

**B4 — RELICT SUTURES**, the believability item: a former collision belt baked
into a plate's present-day *interior*, nowhere near an active boundary — the
Urals, the Appalachians, the Scottish Highlands, the Scandinavian Caledonides
are all exactly this, a healed collision the map still remembers. Before this
there was no mechanism for one at all: `age` (`geology.rs`) was pure per-cell
`fbm_noise`, uncorrelated with anything, and only ever assigned on a belt that
exists *now*.

The plan's own decision: **generate a past, not a simulation** — a time-stepped
tectonic model is Part I Slice 6, already deferred once, and its output for
this purpose would be almost exactly what can be stated directly.
`generate_relict_sutures` (`geology.rs`) bakes 2–4 spines per world,
deterministic from seed: a rejection-sampled start point at least
`SUTURE_MIN_DIST_FROM_ACTIVE_FRAC` (6% of world width) from any active
convergent/transform land cell, walked as a gently drifting line (never a hard
random turn — that draws a scratch, not a range) that also stops if the drift
carries it back toward an active margin. Each suture carries **one uniform
age** for its whole spine, drawn from an OLD (Urals/Appalachians tier) or
ANCIENT (Highlands tier) bucket — never noise — which is the point: a whole
range reads as one coherent age instead of dithering young/old along its own
strike the way a real boundary's noise term does.

The spines are fed as EXTRA SEED CELLS into the exact same multi-source BFS
`compute_orogeny_field` already runs from active boundaries. Downstream code
cannot tell a suture seed from a real one, so a relict range gets its width
(`belt_profile`), its ridge amplitude (`setting_ridge_amp`) and its
erodibility term for free — and critically, **no elevation.rs change was
needed**: `age_amp = 1.25 − age·0.5` (§8.24, already shipped) already turns a
high age into a lower ridge amplitude; B4 just needed to feed it a real age
instead of noise. Every suture is scored `SETTING_COLLISION` (every attested
real relict suture is a healed continent-continent collision). Gated:
`relict_sutures_form_away_from_active_boundaries`,
`a_suture_carries_one_uniform_age`, `relict_sutures_are_deterministic`,
`plate_free_world_gets_no_sutures`, `orogeny_field_carries_the_suture_age_
through`.

**B2 — a motion layer you can read.** `Plate` stays fully transient (§8.24b's
own discipline — recomputed from seed every phase-1 run, never persisted), so
the Euler-pole velocity field that already drives boundary classification
could never be drawn. `PlateMotion` (public, plain-data — `centroid_x/y`,
`pole_x/y`, `omega`, `is_oceanic`, with its own `velocity_at(x, y, world_w)`
duplicating `Plate`'s private rotation formula rather than exposing `Plate`
itself) is what leaves the generator: `generate_plates_and_landmass`/
`generate_plates_and_landmass_with_target` now return `Vec<PlateMotion>`
alongside their existing `WorldBuffer` mutation, and `sim_commands.rs`
persists it to `metadata["plate_motion"]` (JSON, the same one-shot-generator-
output convention `deposits`/`good_localities`/`lakes` already use — no tile
column) from both `sim_generate_plates` and `sim_run_all`. Changing a
generator's return type from `()` to a `Vec` breaks no existing call site —
verified across all of them, since a value ignored in statement position was
never checked by the type system either way.

Boundary-type tinting (convergent/divergent/transform) already existed in
`render_plates` before this — B2 needed no render-layer change there, only a
way to serve and draw the arrows. `get_plate_motion` (query command,
`overlays.rs`) reads the persisted motion and returns one `PlateMotionArrow`
per plate, anchored at its own centroid, with `speed` precomputed
server-side. The frontend (`OverlayManager.drawPlateMotion`, gated by
`overlayVisibility.plateMotion`, shown only while the `plates` layer is
active — the arrows are meaningless without the boundary tinting under them)
normalizes arrow LENGTH by the fastest plate on screen, since `omega` has no
real-world calibration and only relative speed/direction are meaningful; a
dedicated `renderPlateArrow` (filled head, thicker stroke, length set by the
caller) replaces the wind layer's `renderArrow`, whose 12-cell cap would be
invisible at plate scale. Oceanic and continental plates get distinct arrow
hues (`PLATE_MOTION_OCEANIC_COLOR`/`_CONTINENTAL_COLOR`) since they move at
genuinely different real rates.

**The plate inspector — click-to-flip oceanic/continental (built on B2).** A
motion layer you can read invited the obvious next question: can a specific
plate's oceanic/continental assignment (decided automatically by Slice 5's
ocean-fraction fill) be overridden? `sim_set_plate_oceanic(plate_id,
is_oceanic)` does this without a re-partition — `plate_index`/`boundary_type`
(persisted tile columns) are untouched; only which side of an
already-classified boundary is land moves. `plates.rs` extracts the
terrain-rasterization + volcanic-zone tail of
`generate_plates_and_landmass_with_target` (Terrain 2.0 slice 4's level-set
coastline noise, then the collision-style volcanic rolls) into a shared
`rasterize_landmass_and_volcanism`, called both by initial generation and by
the new `rebuild_landmass_from_plate_types` — one copy, so the two paths can
never drift apart (§8.18's discipline applied to a generator, not a colour
ramp). Volcanic flags on boundary cells are reset before every re-roll, which
is what makes a rebuild idempotent rather than only ever accumulating more
volcanic cells across repeated flips — cells placed away from a plate margin
(a lasso Arc island chain, §8.25) are untouched, since those never carry a
CONVERGENT/DIVERGENT `boundary_type`. The rebuild reads its seed from
`metadata["plate_seed"]` (persisted alongside `plate_motion` by
`persist_plate_motion`) rather than taking one as an argument — a rebuild
must use the exact seed generation used (the coastline noise field is keyed
directly off it), and the frontend's Seed field is a UI draft the user may
have since re-rolled without ever pressing Generate again, so it cannot be
trusted as "the seed this world was built with."

Surfaced two ways, both reading the same `get_plate_motion` array (now
carrying `id`/`area_frac` alongside the existing motion fields — a plate's
REAL measured area, counted from `plate_index`, not its nominal size-class
weight): a **Plates panel** in `StepLandmass.tsx` (collapsible, sorted by
area, one row per plate with a swatch, its assignment, area%, and a Flip
button) for browsing the whole set; and a **Flip button in `InfoPanel.tsx`**
(right-click a cell → the existing `plate_index` row, now joined by a new
`CellInfo.plate_is_oceanic` field read from the same persisted motion list)
for flipping the one plate under the cursor. Both are `null`/empty on a world
with no plate data (template/painted, or generated before B2) — same
discipline as an old save's empty `lakes`. Gated:
`rebuild_landmass_is_deterministic` (same seed + assignment ⇒ identical
`terrain`/`is_volcanic`) and `flipping_a_plate_changes_the_land_area_it_should`
(the aggregate land-area swing tracks the flipped plate's own cell count,
within the coastline noise band's margin — not per-cell interior geometry,
since a small or oddly-shaped plate can sit entirely inside the noise band's
`reach` and so isn't a robust per-plate claim on its own).

**B3 — collision STYLE, why the Himalaya and the Andes look different.**
Before this every orogenic belt shared one cross-section: `belt_profile`
was a single decay from the boundary, shaped only by width and offset per
setting. Real belts differ by what's colliding — continent-continent
(`SETTING_COLLISION`) thickens crust over a wide front with several parallel
sub-ranges and an elevated plateau behind them (Himalaya + Trans-Himalaya +
Tibet); ocean-continent (`SETTING_ACTIVE_MARGIN`) concentrates uplift into
one arc-parallel crest offset inland of the trench (Andes).

**NEGATIVE RESULT, kept as the reason the shipped form looks like this**
(§2.4): the first cut cosine-LOBED the existing decay envelope — multiply a
linearly-decaying envelope by an oscillating term and hope for two crests.
Measured directly (sampling `belt_profile` across `dist` and counting local
maxima), it never produced a genuine second peak at any tested `belt_reach`:
the envelope's own decay dominates the lobe amplitude by the time the second
lobe would crest, so the result is one crest with ripples on its downslope,
not two crests. Multiplying a decay by an oscillation cannot guarantee a
second local maximum; only summing (or maxing) independent bumps can. The
fix takes the MAX of three profiles: a main crest right on the suture
(`COLLISION_RIDGE1_WIDTH_FRAC`), a second, lower, offset crest
(`COLLISION_RIDGE2_OFFSET_FRAC`/`_WIDTH_FRAC`/`_AMP` — the Trans-Himalaya),
and a broad, low plateau floor across the whole reach
(`COLLISION_PLATEAU_FLOOR`) so the trough between the two crests reads as
elevated tableland, never a valley carved back toward zero (§8.23's own
lesson about what carving looks like). The collision reach itself widened
(`COLLISION_REACH_MULT` = 2.4× `belt_reach`, up from the old 1.5×) so a
collision belt is measurably WIDER than an active margin at every tested
`belt_reach`, not just multi-crested — the plan's own two-part B3 claim.

No `elevation.rs` change was needed, same as B4: `belt_profile` is consumed
generically by distance from the boundary, so the new shape flows straight
through to the ridge/swell noise it already modulates. Gated:
`collision_belt_is_wider_than_active_margin`,
`collision_belt_is_multi_crested` (both in `geology.rs`, sampling
`belt_profile` directly across `belt_reach` ∈ {14, 30, 60, 90} — the real
production range — rather than rendering a world, since the claim is about
the profile FUNCTION's shape). The multi-crested gate's own peak-counting
helper had to treat a run of equal-valued samples as ONE unit (`dist` is a
`u16` cell count, so fine sampling staircases into flat plateaus, and a naive
"≥ next sample" test double-counts the first cell of every rising plateau as
its own peak) — caught by the gate itself reporting 10 spurious peaks on a
genuinely single-crested active margin before the fix.

Deliberately NOT wired: convergence RATE (from B2's `PlateMotion`) does not
yet modulate ridge count or belt width, though the plan's own text asks for
it ("a fast collision builds a wider, higher belt than a slow one"). `Plate`
velocities are gone by the time phase 2 runs (transient, discarded after
phase 1) and B2 only persists them to `metadata`, which `compute_orogeny_
field` has no connection to read (it takes `&WorldBuffer` alone) — threading
that through is real future work, not silently assumed done. What shipped is
appearance-level and DERIVED from real tectonic classification (the setting
itself comes from real plate velocities, B2/§8.24b), which satisfies the
plan's own "appearance-level, but derived from tectonic motion" decision
even without the rate-modulation refinement.

---

### 8.24a3 A belt varies along its strike; ponding is not a slope problem

Two user reports from one screenshot pair — "too clean mountains are forming"
and "rivers get truncated and the world generates these low ground zones where
rivers go to". One was a real bug with a one-line cause; the other was a
misdiagnosis on both our parts, and the measurement is the deliverable.

**THE WALL — `fbm_noise` does not span 0..1, in a second place.**
`belt_profile` is a pure function of DISTANCE from the boundary, so it
contributes exactly zero variation along a belt's own strike: every point at
the same cross-belt offset gets the identical value. All of a range's
along-strike character therefore comes from one term in `elevation.rs`:

```rust
belt *= 0.35 + 0.65 * belt_noise;   // reads as a 0.35..1.00 swing
```

`fbm_noise` AVERAGES its octaves, so it spans ~**0.11..0.40**, not 0..1 —
the exact fact §8.24b already measured and recorded for the plate warp. So
the realised multiplier spanned **0.42..0.61**: a near-constant ~0.5 with an
18% ripple where the code reads as meaning a 3× swing. A belt of uniform
strength for its whole length has no massifs, no saddles and no gaps — it is
a WALL, which is precisely what the screenshot showed. Normalising on the
measured spread first restores the intended swing (measured 0.350..1.000,
mean 0.653). Gate: `a_mountain_belt_varies_along_its_own_strike`, which
asserts the realised spread exceeds 0.45 and that some stretch reaches full
strength — the un-normalised form fails both by construction.

**The general lesson, now twice paid for: never feed `fbm_noise` into an
expression that assumes a 0..1 range.** Normalise on its measured spread at
every call site that cares about the range rather than just the shape.

**THE PONDING — measured, and the obvious fix does not work.**
`diag_endorheic_fraction` (`rivers.rs`, `#[ignore]`d) walks every land cell's
drainage to its terminus on a real generated world. It reframes the report:
**100% of land already drains to the sea**, 0% dead-ends inland — the
priority flood routes every basin's overflow out. What is actually wrong is
that **6–20% of land is PONDED**, sitting >10 m below its own spill level,
i.e. flat filled hollow. That is the low ground the player sees, and
`extract_rivers` correctly refuses to draw a channel across standing water,
so rivers stop at its edge.

**NEGATIVE RESULT** (§2.4): the obvious cure — give the land a systematic
seaward gradient, since an fbm field has as many local minima as maxima while
real continents stand higher inland — was built and measured, and does not
work:

| ponded % of land | seed 4242 | seed 7 | seed 99 | mean |
|---|---|---|---|---|
| before | 20.3 | 14.8 | 6.2 | **13.8** |
| with a 1500 km continental slope | 20.9 | 10.4 | 14.9 | **15.4** |

It reshuffled which seed was worst and left the mean slightly worse, so it
was reverted. The reason generalises: **ponding is a LOCAL property** (hollows
a few cells across, from the `hill` term and the micro-relief dither) while a
continental ramp is a gradient of ~0.006 per cell. Tilting the table does not
empty the dimples in it.

**WHAT DID WORK: sediment deposition** (`deposit_into_closed_basins`).
There are exactly two ways a real closed basin stops being one — its outlet
erodes down, or sediment fills it up. The first is forbidden and the ban is
hard-won (§8.23's three failed carving attempts: at 11 km cells the channel
is sub-grid and what gets drawn is a dendritic scratch). The second has no
such problem, because it RAISES AN AREA and an area fill leaves no line to
read as a scratch — and it is what actually dominates on Earth, where
endorheic basins are depositional and flat-floored (the Tarim, Lake Eyre,
Death Valley). A priority flood gives each basin's spill level; each cell is
raised toward it by `residual(d) = d · max(MIN_KEEP, smoothstep(d; LO, HI))`.
Two structural guarantees fall out and both matter: the surface only ever
RISES (so it cannot create a new depression), and no cell is raised above its
own spill level (so every outlet and the drainage topology through it are
untouched). It runs after the rank remap — which would otherwise re-fan
anything levelled before it (§8.24) — and before `apply_micro_relief`.

**The tuning is a real frontier, not a free win, and the shape of it is the
finding.** `POND_FILL` is 10 m and `detect_lakes`' threshold is ~35 m, so the
window between "stops truncating rivers" and "stops being a lake" is narrow:
fill hard enough to clear the ponded plains and you start deleting the
world's lakes, which §8.24c forbids. Measured across 3 seeds
(`diag_endorheic_fraction`):

| KEEP_LO / KEEP_HI | mean ponded land | lakes (3 seeds) |
|---|---|---|
| — (before the pass) | 13.8% | 28 |
| 60 m / 200 m | 11.6% | 29 |
| **130 m / 480 m (shipped)** | **9.4%** | **24** |
| 220 m / 1100 m | 8.3% | 17 — lakes gutted |

So ponded land is down about a third and the deep basins survive. It is
**not** "almost none", and driving it there means giving up the lakes; that
trade is the honest limit of this mechanism, not a tuning oversight. A first
attempt used `residual = d²/(d + HALF)`, which reached only 11.5% and cannot
do better at any HALF — draining a 500 m basin with that form needs HALF ≈ 24
km, which would flatten everything else. The shape was wrong, not the
constant. Gate: `deposition_fills_shallow_hollows_and_spares_deep_basins`
(a 40 m hollow must come out under 10 m, a 900 m basin must keep over 500 m,
plus both structural guarantees).

**AND THE ONE THAT ACTUALLY CAUSED THE VISIBLE TRUNCATION: two thresholds
that did not match.** After all of the above the user still reported cut-off
rivers, and the cause was not the terrain at all. `extract_rivers` blocked
channels on a hardcoded **10 m** of fill; `detect_lakes` emits a lake only
past its own `fill_depth`, **~35 m**. Every basin landing between the two was
blocked as "standing water" *and drawn as nothing* — so a river stopped dead
in open ground with no lake, no sea and no confluence at its mouth. §8.24c
states the invariant in its own words ("the lake extent the renderer draws
and the ponded extent the router stops at have to be the SAME set") and this
line quietly violated it. Deposition made it *more* visible, by moving basins
into that band.

`extract_rivers` now takes the caller's `fill_depth` and uses it as the pond
threshold, so the two sets agree by construction. The deep-basin protection
the rule exists for is untouched: a basin deep enough to matter becomes a
lake, and lake cells are already excluded separately. What changes is only
the shallow band — a metre-scale veneer over nearly flat ground, i.e. an
alluvial flat, which a real river crosses. Measured on a real world:
**dangling stubs 10/27 (37%) → 0/49 (0%)**, the river count rising because
channels that used to be cut short now run their full length.

Gate: `a_basin_too_shallow_to_be_a_lake_does_not_stop_a_river`, which asserts
the invariant DIRECTLY (no cell may be ponded-but-not-lake) via the shared
`ponded_mask`, rather than inferring it from stub counts.

**Two lessons about the gate itself, both worth more than the fix.** The
first version asserted stub counts and passed with the bug still in — 1090
rivers on a dense fixture always find *something* adjacent to join. The
second version asserted the right thing but the FIXTURE was wrong: its
regional slope was 14 m per cell against a 22 m bowl, so no closed basin ever
formed and it passed either way. **Always verify a new gate fails on the
unfixed code** (§8.23b's own rule): reverted, it reports 104 invisible cells
and fails loudly; restored, 0.

**AND A THIRD TIME, in the lake's SIZE — the gate was honest and measuring
the wrong thing.** The user reported truncated rivers again with the stub
count at a genuine 0/49, so the gate's definition of "ends properly" had to
be wrong rather than the count. It was: `dangling_stubs` accepts a mouth at a
lake of ANY size, so a river ending at a ONE-CELL lake satisfies it — and a
one-cell lake is smaller than a pixel at world zoom. Worse, the asymmetry was
built in on purpose: river widths are explicitly zoom-compensated so "even
small streams stay visible", while a lake draws as bare `fillRect(x, y, 1, 1)`
per cell with no minimum. So the river was widened to stay visible and its own
terminus was allowed to vanish. `diag_river_mouth_visibility` classifies every
mouth by how visible its terminus actually is and measures **6-9% of all
rivers ending at something the reader cannot see** (3 per world across 3
seeds).

The fix is cartographic and belongs in the renderer, because the DATA was
right all along — the lake exists, it is in the list, the panel will list it.
A lake at or under `MIN_LAKE_SYMBOL_CELLS` also draws a disc of at least
`MIN_LAKE_SYMBOL_R` scaled by the same `1/sqrt(zoom)` the rivers use, so a
pond and the stream feeding it stay in proportion; the true per-cell footprint
is still filled underneath, and zooming in hands the drawing back to the real
cells the moment they are bigger than the minimum. Every atlas draws a small
water body at a minimum symbol size for exactly this reason.

**The generalisation, now paid for three times in one sitting:** each of these
was the model holding a fact and the RENDER dropping it — a ponded cell no
lake represents, a basin between two thresholds, a lake below one pixel. When
a user reports something missing from the map and the data gate says it is
present, suspect the render before re-tuning the generator, and **measure the
symptom the user can actually see rather than the invariant that is
convenient to assert.** Note also what has NO automated gate here: this
codebase has no frontend rendering test beyond `tsc`, so the minimum-symbol
rule is guarded by the diagnostic plus looking at it — stated plainly rather
than dressed up with an assertion that would not exercise the canvas.

---

### 8.24a4 A river ends at the sea, a lake, a confluence — or it evaporates

The rule, and the ONE legitimate exception, now stated in the code rather than
implied: **every river must reach the sea, a lake or a confluence, unless its
channel genuinely dries up in a desert** — the Sahara wadi, the Australian
creek, the Tarim petering out into sand.

Before this that exception could not be expressed. Discharge is ONE SCALAR per
reach computed at the outlet (§8.24d), so flow could never decline along a
course, and the only dry-climate handling was a whole-river prune
(`!is_mouth && order <= 2 && arid_frac > 0.55 → continue`) that deletes a
stream outright. So a desert river was ALL OR NOTHING: it either ran its full
length to the sea or vanished entirely, and the commonest thing a desert river
actually does — run a while, then dry up — had no representation at all.

**The mechanism is a water budget, not a rule about deserts.** A per-reach walk
from source to outlet in m³/yr: GAIN is the NEW catchment joining at each cell
× its own runoff depth (`RUNOFF_ARID` 0.08 vs `RUNOFF_HUMID` 0.35, matching the
lake balance's own split); LOSS is the wetted channel strip
(`CHANNEL_LOSS_WIDTH_KM`, stated in km and converted per world per rule 25) ×
the local evaporative excess `PET − P`, with PET a documented temperature proxy
(phase 5 has no radiation budget, and it is only ever used to decide whether a
dry channel survives, never to size a river). Where the budget goes negative
**in an arid cell**, the reach is truncated there and flagged `River.ends_dry`.

**The Nile falls out of the model instead of being special-cased**, which is the
strongest evidence it is the right mechanism: a river fed from wet uplands
arrives carrying a volume that dwarfs the per-cell loss and crosses the desert,
while a stream raised IN the desert dies within a few cells. Measured on the
banded fixture: 3 rivers died with a mean catchment of 199 cells, 22 crossed
with a mean of 8,033 — a 40× separation. `a_great_river_crosses_the_desert_
that_kills_a_small_one` asserts exactly that, and a blanket "delete rivers in
deserts" rule passes the ends-somewhere gate while failing this one.

**THE OLD PRUNE AND THE NEW BUDGET ARE THE SAME PHYSICS STATED TWICE, and
applying both deleted every wadi the budget had just modelled.** First
measurement after wiring the budget in: **0 rivers ended dry** on a world with
a hot desert belt across every drainage. A dried reach has `is_mouth == false`,
low order and a ~100% arid course, so it matched the arid prune every time.
"An arid basin swallows its runoff" is precisely what the budget now computes,
and the budget is the better instrument because it says WHERE the water runs
out rather than merely that it does — so it wins, and the prune keeps only
order-1 rills so a desert still reads sparse rather than veined. The gate
caught this rather than passing vacuously, which is what the `!dry.is_empty()`
assertion inside it exists for.

**Two consequences that are easy to miss.** A dried river joins nothing, so it
is neither a trunk nor a `tributary` (confluence detection seeds settlement
magnets off that flag). And it needs its own RENDER: drawn as a plain solid
line, a modelled terminus is pixel-for-pixel identical to the truncation
artefact, so the lower course is drawn DASHED and fading — the atlas convention
for an intermittent watercourse — and the reader can tell "this evaporates
here" from "this was cut off for no reason". No playa symbol is drawn: the sim
models no playa, and inventing one would be a claim the data cannot support.

**Grid size is load-bearing in the fixture and is not arbitrary.** Catchment
gain scales with cell AREA (cell²) while channel loss scales with cell LENGTH
(cell¹), so on a coarse test grid one cell is a 40,000 km² catchment and no
stream can ever dry — the mechanism is inherently weaker on coarse grids and
stronger at real resolution. The banded fixture runs at 600×300 (~67 km/cell)
for that reason; at 200 wide it would have passed while testing nothing.

Gates: `every_river_ends_somewhere_or_dries_in_a_desert` (no unexplained stub;
every `ends_dry` terminus is in a Köppen B cell), `a_great_river_crosses_the_
desert_that_kills_a_small_one`, `a_humid_world_never_dries_a_river_up` (the
mechanism must be INERT where there is no desert, or it is just a licence to
truncate). All three verified failing before the fix.

---

### 8.24b Plate margins are warped at the SOURCE

`plate_index` was a plain Voronoi partition — every plate boundary was the exact
perpendicular bisector of two seed points, i.e. a straight line. Everything
downstream is derived from it: `boundary_type` is read off `plate_index`, the
orogeny belt is a distance field from `boundary_type`, `deposits.rs` reads the
same column as tectonic setting, and the coastline is a threshold on plate crust.
So one straight partition drew a straight mountain range, a straight rift, a
straight ore belt and a straight margin, all at once — the "mountains are straight
lines" report.

Downstream passes had each grown their OWN warp to hide it (`elevation.rs`'s
`oro_warp_*` warps the orogeny LOOKUP; `warp_terrain_boundary` warps the
coastline). **Warping a lookup only bends where a straight line is SAMPLED
FROM.** The line is still there, and each pass bends it differently, so the range,
the rift and the coast stop agreeing about where the margin runs.

`warped_voronoi` warps the PARTITION instead: the nearest-seed scan is done at a
multi-octave domain-warped sample position, so there is no straight line left
anywhere downstream to bend. `warp_frac = 0.0` reproduces the plain Voronoi
partition exactly, which is what lets the gate use the function as its own control.

Three things measured here, all of which look like tuning and are not:

- **`fbm_noise` does NOT return 0..1.** It averages its octaves (`val / max_amp`),
  so it concentrates about its own mean with much-reduced variance — measured,
  ~0.11..0.40, mean ≈ 0.28. The first cut used `fbm_noise(..) - 0.5` inline, which
  is therefore an almost constant NEGATIVE offset: it TRANSLATED the sample field
  instead of bending it. 10% of cells flipped (all near a margin) and straightness
  moved 0.504 → 0.498. **A displaced straight line is a straight line.** The field
  is now centred and scaled on its own MEASURED spread.
- **The metric has to be local, and at the right scale.** The first metric was
  total boundary LENGTH, on the reasoning that a straight bisector is the shortest
  curve between two triple junctions. True of one segment, false of a partition:
  bowing one margin out bows its neighbour in, so the total is conserved — measured
  1.00× (5682 vs 5672 cells) on a visibly curved partition. The second metric was
  local straightness at a 6-cell radius, against an ~80-cell plate spacing, where a
  margin curving over its whole length still looks locally straight. Only the third
  (local straightness at R=20) sees it.
- **The constants come from `diag_sweep_plate_warp`, not from eye.** It maximises
  the straightness drop subject to worst-plate connectivity > 0.90 — the constraint
  being that a large warp lands samples inside a NEIGHBOURING plate and the
  partition sheds detached specks, which `boundary_type` then reads as phantom
  plate boundaries scattering ore districts through plate interiors. The usable
  region is a SHORT wavelength at a modest, tightly-clamped amplitude; a long
  wavelength translates whole plates instead of curving them. Shipped
  0.25 / 0.80 / 0.80 → straightness 0.449 → 0.368.

Gates: `plate_margins_are_not_straight_bisectors`, `plate_territory_stays_connected`.

---

### 8.24c Lakes have a WATER BALANCE, not just a fill

A priority-flood answers one question — "from which cells can water not escape
downhill?" — and its answer is a BASIN, every cell below the spill point however
vast. `detect_lakes` emitted that basin verbatim, guarded only by "is it more than
a QUARTER of the world", so a broad continental interior sagging a few tens of
metres below its rim came out as one water body of millions of km², larger than
some seas, standing on ground that is merely low.

The missing question is the water balance. A closed lake is in steady state when
inflow equals evaporation off its surface, so its equilibrium AREA is
`inflow ÷ evaporation_depth` and nothing about the basin's shape can make it
bigger. That is why the Caspian sits far below its own rim, why the Aral shrank
when its inflow was diverted, and why the Sahara's deep basins are dry — one
equation, three cases. `trim_basin_to_water_balance` computes it and LOWERS THE
WATER LEVEL of an over-large basin, keeping the deepest cells.

Four rules:

- **Lower the level; never delete the lake.** The pre-2026 code deleted an
  over-large basin outright, which left the drainage still routed through a dry
  hollow so rivers were drawn climbing the basin walls — which is why the size
  filter was removed and replaced by the quarter-of-the-world guard in the first
  place. Above that guard the map had giant lakes; below it, dry basins with rivers
  running uphill out of them. Lowering gives every closed basin a real water body
  for a river to end in, at a size its climate sustains.
- **An ice sheet holds no lake.** A basin under a permanent ice cap (Köppen `EF`,
  or perennial `snow_frac`) is full of ice; drawing open water there is wrong on
  the map and feeds a fish assemblage into `aquatic.rs` for a glacier.
- **`LAKE_MAX_KM2` is stated in km²** (rule 25) — the Caspian, the largest lake
  that exists, is 371,000 km².
- **A world-sized test basin cannot test the climate term.** Its catchment is so
  large that BOTH a wet and an arid climate clear the absolute cap and clamp to the
  same answer (measured: wet 6 cells, arid 6 cells — a gate that would have passed
  on a water balance wired to nothing). `hollow_world` exists for that contrast.

Gates: `no_lake_is_larger_than_the_largest_real_lake`,
`an_over_large_basin_is_lowered_not_deleted`, `an_ice_cap_basin_holds_no_lake`,
`an_arid_basin_holds_less_water_than_a_wet_one`.

**FLOW AND LAKES ARE ONE PASS** (`compute_world_hydrology`). A priority flood
fills every closed basin to its spill level, and `extract_rivers` refuses to draw a
channel across a filled basin (`ponded`, >10 m of fill) because water stands there
and a line across it climbs the true ground beneath. That was consistent while
every filled basin became a drawn lake — and the water balance broke it: the basin
is still ponded end to end while only its deepest part is drawn as water. Measured
on an endorheic fixture, **53,998 cells were treated as standing water and 22 were
drawn as lake, and all 12 rivers stopped at that invisible shoreline** — the
"rivers end to nowhere / are truncated" report, a straight consequence of two
extents disagreeing. The second pass re-floods with the LAKE SURFACES as drainage
sinks (`compute_hydrology_with_sinks`), so the exposed basin floor becomes what it
physically is — dry land draining inward — stops being ponded, and rivers run
across it and end AT THE SHORE. No river climbs, because the flood now descends
toward the lake rather than toward the spill. Skipped when nothing was trimmed, so
an ordinary world pays for one flood. Gate:
`a_river_always_ends_at_water_or_another_river` (0 stubs, was 12 of 12).

**Lakes are now PERSISTED** (`persist_lakes`/`load_lakes`, `metadata["lakes"]`).
They used to be the one hydrology product with no stored copy: every consumer
called `detect_lakes` again with its own hard-coded `fill_depth` of 0.004, while
the set the user sees was built from the `lakeFillDepth` SLIDER. Those disagree the
moment anyone moves the slider — and settlement placement was a consumer, so towns
were sited to avoid one set of lakes and drawn under a different, larger one. Same
class of bug as §8.18's hand-copied colour tables, same fix: keep one copy.

**A lake cell is UNDER WATER.** `buf.terrain` calls it land (a lake is a filled
depression on land, not sea), so every settlement gate passed — and then the
lakeshore bonus's ±2 window includes the cell itself, so a lake cell was not merely
allowed as a town site but actively attractive. `compute_habitability_fields` now
zeroes `hab`/`trade` on lake cells, at the source rather than in one call site, so
the rule reaches city placement, the habitability layer, junction sites and colony
siting alike. The shore keeps its bonus.

---

### 8.24d River discharge is computed ONCE

`River.discharge_m3s` is the river's real mean annual flow, derived in
`extract_rivers` from catchment area × runoff depth in honest units.

It exists because there were TWO discharge calculations that did not agree:
`extract_rivers` derived a unitless proxy for the render width with
`runoff = mean_precip / 700` clamped to 0.2..2.2, while the river-systems query
behind the Hydrology panel independently used `mean_precip / 1000 × 0.35` clamped
to 0.01..1.2. Same river, two runoff models — so the flow on the panel and the
width on the map answered different questions.

**Render width now comes from hydraulic geometry** (Leopold & Maddock: `w ≈ 6·√Q`).
What it replaces saturated: `ln_1p(discharge/threshold)·0.62 + 0.7 + len_term`
clamped to `(0.6, 2.6)` reaches its ceiling at roughly twenty times the channel
threshold and stays there, so every river above a fairly ordinary size drew at
EXACTLY the same width and a 50,000 m³/s trunk was indistinguishable from a
5,000 m³/s one. That is the "rivers don't get wider with a large flow" report, and
it was a clamp, not a modelling subtlety.

Two rules: a **unit** gate must be scale-free (`discharge_is_in_plausible_cubic_
metres_per_second` backs the implied runoff depth out of each river's own Q and
catchment, because a plain "is Q between 1 and 300,000" assertion measures the
FIXTURE — a 300-cell-wide test grid has 134 km cells); and the panel reads the
STORED value, falling back to its own estimate only for a world generated before
the field existed. Gates: that one plus
`river_width_tracks_discharge_instead_of_saturating`.

**Delta fans need the seam guard too** (rule 6). Every river stroke in
`OverlayManager` goes through `strokeSmoothPath`, which breaks a path wherever two
points jump more than `seamGap` — but the distributary fan drew mouth→cell
directly with no guard, so a delta whose mouth sits near the antimeridian had cells
on both sides of the seam and the segment joining them ran the full width of the
world at near-constant y. That is the "straight line across the map on the rivers
layer": not a latitude line, not a river — one unguarded delta spoke.

---

### 8.25 Stage-1 freehand area tools (`step1_plates/landmass_ops.rs`)

The Landmass step used to be three buttons and a circle brush — no area
marking, no coastline shaping, no islands, and "Generate from Plates" repeated
the identical world on every press (`ITCZ_AND_LAND_TOOLS_PLAN.md` Commit 1). A
`Lasso` (a freehand-drawn polygon in world-cell coordinates) plus four ops —
`smooth_roughen`, `fjords`, `island_chain`, `fill` — mutate
`terrain`/`elevation`/`is_volcanic` only within it. Each op loads
`ColumnSet::PHASE_PLATES` and the caller calls `buf.save`, which already
pushes exactly one `undo_journal` entry, so every op is undoable and
re-rollable for free — no new history code.

Four rules this module holds, each gated by its own test:

- **The lasso is UNWRAPPED, not clamped** (rule 6). A polygon drawn across the
  antimeridian arrives with points on both edges; a naive point-in-polygon
  test selects the *complement* of what the user circled. `Lasso::new`
  re-expresses every vertex in one continuous frame anchored on the first
  point; the hit test then tries the polygon at `x`, `x−w` and `x+w` so a
  query cell hits whichever wrapped copy actually contains it. Gated by
  `lasso_across_antimeridian_selects_what_was_drawn`.
- **Every op FEATHERS to the lasso edge.** A hard clip prints the user's
  selection gesture onto the map as a straight coastline. `Lasso::blend`
  gives a soft 0..1 membership (a smoothstep ramp over `FEATHER_CELLS`), and a
  feathered cell's fate is decided by a deterministic per-cell hash against
  that blend — soft at the edge, but bit-reproducible for a given seed rather
  than a fresh RNG draw each run. Gated by `op_feathers_to_lasso_edge`.
- **Roughening is a LEVEL SET, never a per-cell dice roll**: a signed
  distance-to-coast field — `local_coast_field`, a multi-source BFS scoped to
  the lasso's own padded bounding box, never the whole world — perturbed by
  fbm and re-thresholded at zero, bounded by a `reach`. A per-cell noise
  threshold scatters speckle islands across deep ocean by construction; a
  level set can only ever move a coastline that is already there. Gated by
  `roughening_is_bounded_by_reach_not_scattered`.
- **Ops iterate the selection, never the world** (§8.9 rule 1's spirit).
  `Lasso::candidate_cells` scans only the polygon's own padded bounding box, so
  reshaping one bay on a 26M-cell world costs a few hundred cells, not a
  full-grid sweep. Gated by `ops_iterate_selection_not_world` (a lasso op on a
  4000×2000 world must complete in well under the time a whole-grid scan
  would take).

`fjords` walks inland from a selected coastal sea cell — sinuous, tapering to
the head — carving a real channel, which is the honest way to draw a fjord as
opposed to notching a coastline with noise (see §8.23's record of why
noise-carved channels read as a drawn scratch rather than a landform).
`island_chain` supports `Arc`/`Scatter`/`Single`; only `Arc` islands are
marked `is_volcanic`, which is real data — `deposits.rs`'s `VolcanicArc` model
scores off that exact column (§8.16), so a planted arc can carry a genuine ore
province later. `fill` is the decisive bulk-set op, still feathered at the
edge like every other op here, just committing only past the blend midline
rather than by a stochastic draw.

Commands: `land_op_smooth_roughen`/`land_op_fjords`/`land_op_islands`/
`land_op_fill` (`commands/sim_commands.rs`), each taking the lasso polygon as
JSON (the same shape `sim_generate_ridges` already uses for `linesJson`).
`preview_commands::render_world_thumbnail(max_px)` is a read-only, downsampled
land/sea + elevation thumbnail sampled directly from the `WorldBuffer` —
deliberately NOT read back through the tile/LOD cache, whose invalidation
timing after a generate would make a thumbnail silently stale — used by the
Landmass step's 2-variant compare (generate A → thumbnail → undo → generate B
→ thumbnail → show both, keep one or restore the other from its own seed).

Frontend: `uiStore.activeTool` gains `"lasso"`; `lassoPolygon` is a single
transient selection (mirrors `ridgeLines`'s draft/commit plumbing exactly —
`MapCanvas`'s lasso pointer handlers accumulate a draft polygon and commit it
on pointer-up, `OverlayManager.setLassoSketch` draws it). `StepLandmass.tsx`
carries the Area Tools panel (draw/clear, the four ops with their own
params + a Re-roll button that undoes and re-applies with a fresh seed
against the same polygon), a Randomise-landmass button (fresh seed → generate,
where "Generate from Plates" used to always repeat the stored seed), and the
2-variant compare.

---

