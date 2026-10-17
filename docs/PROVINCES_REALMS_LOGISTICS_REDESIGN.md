# Provinces, Realms & Logistics Redesign — research and decisions (2026-10-17)

Owner request (2026-10-17): check the logistics system; overhaul provinces (organic,
intertwined borders); a province POPULATION that consumes and exports to nearby cities;
provinces and realms generated when world generation finishes, with the author able to
assign provinces to realms and create realms; a redesigned province view (minimap of raw
goods only) and a realm view ("Nation Overview": realm minimap with its provinces, each
province's main export goods → a realm goods-export map) — all in ONE window with
minimap toggles. Questions first, then a mock-up, then build.

Four research reports fed this doc (code audit; cartography of borders; province economy
+ money history; window design). Their findings, condensed:

## 1. Logistics today (code audit)
- **Chain:** daily `dispatch` (`tick/production.rs` ~1497): a seller with stock above
  reserve ships to ≤3 of its 32 trade neighbours, ranked by price gap; amount
  `min(surplus, room×0.5)`; carrier attached AFTER the decision (seller's house, buyer's
  house, else nobody). Both sides are paid at departure; cargo lands on its arrival day.
- **96% of cargo has no carrier**, never sinks (`N1B_OWNERLESS_LOSS_RATE` = 0).
- **Relays:** ship legs ≤3,500 km, caravan ≤800 km; over-range legs hop via `staging_hop`
  up to `RELAY_MAX_HOPS` (6).
- **Bugs / unrealistic parts found:**
  1. **Cargo lands in the wrong market when it runs out of hops** (`mod.rs` ~11112): the
     buyer already paid, the goods are unloaded at the last stop. BUG.
  2. **In-transit cargo is ignored when computing a buyer's room** (`production.rs` ~1883):
     cities over-ship (the same loop that blocked the S1 voyage-loss dose).
  3. Over-range cargo with no legal stop still sails the whole leg in one jump.
  4. "Sea" = both ends coastal; range checks use straight km while days use real paths.
  5. Vessel capacity binds nothing (`CAPACITY_BIND_DOSE` 0).
  6. Towns founded during the campaign get straight-line × climate routes (no terrain).
  7. Relay stops are picked from trade-pull neighbours, not geography (16.9% detours).
- **The countryside barely exists:** `province_land_pass` grows only the first food good,
  delivers the surplus ONCE A YEAR as one lump to the seat city; rural people buy nothing;
  all raw goods belong to the 50-120 km city catchments frozen at campaign start (land
  outside every circle produces nothing); food is likely counted twice (catchment belts +
  rural harvest). Towns are matched to provinces on the 720×360 sampled raster.

## 2. Why province borders look straight (cartography + audit)
1. The map draws borders from a raster shrunk to ≤768 wide (~55 km cells) as raw cell
   edges — any 10-40 km wiggle is thrown away before drawing.
2. The flood's noise is WHITE (fresh per step, `provinces.rs` ~1320), averages out along a
   path → borders fall on straight weighted-Voronoi lines with 120° triple corners.
3. The snap pass anchors flat ground to the old straight line (`FLAT_ANCHOR` 0.8).
4. Seeds on a jittered regular grid → same-shaped neighbours.
**Recommended algorithm** (deterministic, ~tens of ms): smooth value-noise fields (≈65 km
and ≈28 km wavelength, X-wrapped, one domain warp) multiplying flood step costs, masked to
zero within ~3-6 cells of rivers/lakes/crests so borders still follow nature; snap stage
with noise-shaped anchor; a connected-component pass so no province splits; seed spacing ∝
food_capacity^-½ (small provinces in crowded fertile land, large in steppe/desert/mountain);
borders sent as full-resolution vectors (marching squares + Douglas-Peucker), open borders
smoothed client-side (Chaikin), river borders drawn on the river. Gates: open-border
sinuosity 1.15-1.35, crest/river coincidence kept, zero split provinces, deterministic.

## 3. Province economy (game design + economic history)
- Comparable games: Imperator (per-province trade-good surplus), EU4 (one good per
  province → trade nodes), Victoria 3 (pops + market access; Paradox later simplified local
  trade to a world market), Anno (visible carts read most realistic).
- History: peasants eat their own staples; ~10-30% of output marketed (band, not a sourced
  constant); market towns within ~1 day's haul (25-30 km); buy salt, iron tools, cloth,
  drink; bad years flip the countryside from seller to buyer.
- **Owner decision (round A): aggregate + workforce** — one rural population per province
  that eats its staples, sells the surplus of its main raw goods to its 1-2 market towns
  and buys comforts back, AND staffs the province's estates and mines.
- Cost: ~900 provinces × ≤6 goods × 12 months ≈ 65k updates/yr → <10 ms/yr. Must NOT enter
  the daily `dispatch` (+75% sellers ≈ +6 s/yr) or become cities (n² route tables).
- Must not double count: city catchment production and province production of the same
  raw goods cannot both exist — the province takes over raw production, or relabels it.

## 4. Money with no gold or silver (economic history) — owner item 5
Ladder: gold/silver/electrum → copper/bronze (Roman aes, Chinese cash) → **iron** (Spartan
pelanor, Celtic currency bars, Song iron cash — so heavy it gave rise to paper money) →
**commodity money from the region's own goods** (salt bars — Ethiopian amole to 1911; cloth —
Icelandic vaðmál, West African strip-cloth; tea bricks — Tibet; cattle; cowries on tropical
coasts, inflating when flooded) → **barter + unit of account** (grain reckoning, tallies).
Today `coin_metal_for` stops at bronze and otherwise "assumes imported silver".

## 5. Window design (design)
One window (~1000×640, docked right): left = realm/province map, right = tabs; a breadcrumb
(Realm › Province) drills down in place. Map toggles: **fills are radio** (Realm / Culture /
Order) and **marks are checkboxes** (Borders · Goods exports · Cities · Trade). Realm tabs:
Overview (culture bar + busts, chief exports, standing, recent chronicle) · Provinces
(table: population urban/rural, tax, order, exports, holder) · Goods (net surplus/deficit
diverging bars, click → map shows that good) · Dynasty. Province pane: identity line,
main exports pinned, urban/rural bar, five existing tabs; goods in three states: exploited
(full colour) · known untapped (washed) · undiscovered (hatched). The mock's "economy
index" and "realm power" do not exist in the sim and need defined formulas. Existing
defects to fix in passing: realm minimap is a blurry fixed canvas, goods icons drawn at
16 px (blurry), hard-coded colours outside the theme, a second token set in windowKit.

## 6. Owner decisions so far
| Topic | Decision |
|---|---|
| Province population | Aggregate per province, monthly, + workforce for estates/mines |
| Realms at world end | Always generated, then editable (assign provinces, create realms) |
| Borders | Follow rivers/ridges/coasts, roughened 10-40 km elsewhere; size by density |
| Realm window | One window: minimap + tabs, province drill-down in place |
| Goods visibility | World map shows all goods; in a campaign the unexploited are tinted, exploited clear |
| Raw production | **Provinces take over**: each province's rural population produces its raw goods from ALL its land and sells to its market towns; cities keep crafts and trade. Phased in, econ_ gates per step |
| Cargo bugs | **Fix both now** (wrong-town unload after max hops; in-transit cargo ignored in buyer room) |
| Starting realms | **Randomised sizes** — some one province, others many; each a distinct colour, with a colour-picker palette to change it |
| Realm stats | **Define both**: Economy index = trade + production value per head (100 = world average); Realm power = population × cohesion × (treasury + revenue), with a word (Weak … Dominant) |

## 7. Owner decisions on the goods follow-ups (same day)
1. Manufactured goods take ANY available input of the right category, and the product is
   named by its material: "Llama-wool carpets", "Carnelian jewelry", "Carnelian-gold jewelry".
2. Fix guild strikes (manufacture must read event effects).
3. Metric units as standard (imperial as an option).
4. Families (spice, gem, dye, incense, salt…), and each people has its FAVOURITE member
   ("spices" → nutmeg for one people, pepper for another).
5. Regions without gold/silver use alternative money — iterate on the historical ladder (§4).
6. The world map shows all goods; in a campaign some become tinted (unexploited) and the
   exploited ones stay clear.
