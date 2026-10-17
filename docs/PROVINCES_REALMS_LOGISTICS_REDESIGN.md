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

## 8. Mock-up 1 review and government decisions (2026-10-17)
Owner: the one-window mock-up "looks good", implement that view; re-check its logic.
**Logic fixes found on re-check (apply in the real build):** (1) country→market arrows must
go to the province's own market town(s), then the 1-2 nearest towns — the mock sent them
to the neighbouring province with most townsfolk, even across a realm frontier; (2) the
Goods tab's consumption must be real use by the realm's people, not constants; (3) taking
a realm's capital province away must pick a new capital or be refused; (4) realm power
uses live cohesion; (5) economy index is computed, never a fixed number.

**What government exists today:** cities have the full Living World government (form,
seats/officials, debates, edicts, ideology meters); realms have ruler/heir/regent/family,
legitimacy, cohesion, autonomy, rank, ruling house, vassals — no council, offices or laws;
provinces have only a holder (city/house/crown), a tax rate, works and unrest.

| Topic | Decision |
|---|---|
| Realm government | **Crown + royal council**: ruler, heir, regent, 3-5 great offices (chancellor, treasurer, marshal, steward…) filled from houses and notables; realm laws (tax level, levies, succession law, trade policy); driven by legitimacy, cohesion, autonomy |
| Province government | **Governor** appointed by the holder (crown, house or city); loyalty and honesty; sets tax, levy and works; can skim or rebel |
| Control | **Set when building the world**: the author makes the map and decides countries and their government types. **At campaign start: continue as authored, randomise, or clear to nothing** so the whole history is simulated from zero |
| Layout | **Same window**, Realm › Province › City breadcrumb, a Government tab at each level |
| Realm laws | **All four**: taxes & levies; succession law; trade policy (open / tolls / staple rights / embargo); towns & cultures — since cities always keep their own council, "towns" means TOWN PRIVILEGES (the fee and tax share a city pays the crown), plus minority tolerance |
| Cities in a realm | **Always their own council**; they pay the crown taxes/fees and send a delegate to the royal council |
| Royal council | **Houses, notables and cities**: offices to the ruler's kin, powerful houses and famous notables; large cities send a delegate; each member has character and stance (as the city council chamber) |
| Governors | **Skim taxes** (shown in the accounts), **rebel or break away** (a disloyal governor of a distant restless province founds a one-province realm), **be replaced** (by the holder automatically, or by the author in the world builder) |

## 9. Implementation progress
**Step 1a-1d (2026-10-17, done):**
- **Strikes work:** `manufacture_pass` now reads the day's event multiplier, so a guild
  strike halves output (it changed nothing before). Crafted goods can **lose their
  know-how** (`roll_origin_failures`): output halves for two years and the city's craft
  tradition drops 40%.
- **Goods families** (`step8_biological_goods/goods_family.rs`): wool (fleece, llama),
  plant fibre, dye, gem, bullion, spice, incense, salt, iron, timber, fish. A recipe
  takes ANY member of its input's family (both the year-0 economy and the campaign), and
  the product is named after what it was made from: `TickHub.made_from` →
  `HubGoodDetail.material` → the city market reads "Carnelian-gold jewelry",
  "Llama-wool carpets". Minor inputs (<30% of the main one) never name the product; the
  lighter ornament leads. **Indigo is switched back on** as the everyday plant dye — with
  `dyes` retired, no dye existed and cloth, carpets, brocade and books were never made.
- **Favourite member:** a people whose taste names a generic good ("spices",
  "gemstones", "dyes") craves its own favourite member instead, chosen stably per people
  (`culture_values::favourite`).
- **Cargo bug 1 fixed:** a relayed cargo that runs out of stops sails its last leg to the
  buyer who paid, instead of being unloaded in a town nobody chose.
- **Cargo bug 2 — measured negative result:** counting cargo already on its way as stock
  (`INBOUND_ROOM_DOSE`) broke `tick::tests` at 1.0 (6 failures), 0.5 (5) and 0.25 (3):
  the charter squeeze chokes, houses go broke, dense-world staging concentrates wealth.
  The economy is calibrated around the over-shipping. Built (the arrival calendar now
  keeps a running inbound tally) and shipped at 0.0; queued with the province economy,
  which changes who supplies what.
Gates: `tick::tests` 417/417, `econ_` 6/6 (inheritance numbers unchanged), `goods_` 27/27,
`manufacture` 7/7, new tests for strikes, families/naming and favourites; tsc; check_types.

**Step 1g: metric units (2026-10-17, done; not seen in the app).** `campaign_unit_scale`
reports how many cereal units one person eats a day; `src/units.ts` sets that equal to
0.6 kg of grain and gives every good kilograms per unit from its value and value per
weight (DNA tier: bulk ×1 … treasure ×20,000; gold 20,000×, silver 2,000×, so gold:silver
by weight comes out 10:1). Quantities show in the unit people used: t / kg / g, litres
and hectolitres for drink and oil, metres for cloth, head for horses; imperial = tons /
lb, gallons, yards, troy ounces for treasure. Settings → Appearance has the Units
switch (metric default). Shown first in the city market (made here, arrivals,
departures); the rest of the panels still show model units — queued, panel by panel.

**Step 1e: the money ladder (2026-10-17, done).** `coin_metal_for` used to treat any
region with no gold, silver, copper or tin as if it imported silver. It now climbs down
the historical ladder: gold / silver / electrum → bronze (copper or tin) → **iron**
(`COIN_IRON`, struck as "Iron Bar of …": Spartan iron, Celtic currency bars, Song iron
cash) → **commodity money** (`COIN_COMMODITY`, the region's most money-like good it
actually makes, weighted salt 1.0 · cloth 0.9 · linen 0.85 · tea 0.8 · fleece 0.7 ·
hides/horses 0.6 · grain 0.45-0.5 — "Salt Bar", "Cloth Ell", "Tea Brick", "Head of
Cattle", "Grain Measure"; `TickHub.coin_good`) → **grain reckoning** (`COIN_RECKONING`,
barter with a unit of account). Lower rungs strike petty denominations only. Their
economic effect (`MONEY_LADDER_DOSE`): cheaper exchange value than silver (iron 0.04,
commodity 0.25, reckoning 0.5 of a silver coin) and a weaker pull as a store of value
(0.6 / 0.5 / 0.4). **Dose walk:** 1.0 broke `econ_inheritance_rules_fragment_differently`;
0.5 passes it and all `tick::tests` (418) — shipped at 0.5. Gate:
`a_region_without_precious_metal_climbs_down_the_money_ladder`.
**Queued:** commodity-money inflation from its own supply (a salt glut inflates prices;
the cowrie-flood case) and trade friction outside the producing region.

**Step 1f: unexploited goods tinted in a campaign (2026-10-17, done; not seen in the
app).** The map always shows every good's belt. During a campaign,
`campaign_exploited_goods` lists, per good, the provinces that actually produce it
(`province_good_actual > 0`); `OverlayManager` draws the belt at full colour there and as
a faint grey hint everywhere else. Refreshed each campaign year. Outside a campaign
nothing is tinted.

**Step 2a: organic province borders (2026-10-17, done; not seen in the app).** The flood
that draws provinces used a WHITE cost noise (a fresh random number per step), which
averages away along a path and left borders on straight weighted-Voronoi lines. It now
multiplies each step's cost by e^(±0.55) from a SMOOTH noise field (`border_noise`: a
~65 km layer warped by a ~28 km one, X-wrapped, at least 2 cells per wave), so the line
where two provinces meet meanders at 20-80 km. Within ~3 cells of a dividing river, a
lake or a crest the noise is off (fading in to 6 cells), so borders still ride the
features and never zig-zag across a river. The white term stays only as a tie-breaker
(0.02). Size by density already existed (fertile heartlands get smaller provinces,
hostile land larger, `local_sep2`). Gates: `provinces` 25/25, incl. the existing
crest/river-coincidence tests and new `border_noise_is_smooth_varied_and_wraps`.
**Queued:** smoothing the drawn borders (they are drawn as raw cell edges) and a
connected-component guard if the snap stage ever splits a province.

**Step 3a: the countryside goes to market every month (2026-10-17, done).** The yearly
lump of food surplus dumped into the seat city at the year's end is replaced: each
month `province_market_pass` carries 1/12 of it to the province's one or two largest
towns (split by size, falling back to the seat), and the countryside takes 12% of that
value back in town goods it cannot make (`RURAL_BASKET`: salt, iron, metalware, cloth,
linen, beer, wine) from those towns' own markets — a barter, no money created, so a
countryside short of salt makes salt dearer in its market town. Colony maturity still
scales what a young colony receives. `PROVINCE_MONTHLY_DOSE` walked: 0.0 bit-identical,
**1.0 passes `econ_` 6/6** (inheritance gate holds) and, after updating the two land tests
that read the seat's stock right after the harvest to let the year's monthly deliveries
run first, `tick::tests` 419/419 — shipped at 1.0. Gate: new
`the_countryside_goes_to_market_each_month_and_buys_salt_back`.
**Next (3b):** the province's rural population takes over RAW production from all its
land (today only land inside a city's 50-120 km circle produces), sold to its market
towns — the largest economic change of the plan, walked in doses with `econ_` per step.
Then 3c (rural workforce staffs mines and estates), 4 (realms at world end), 5 (crown
council, laws, governors), 6 (the one window).

**Step 3b: the countryside takes over raw production (2026-10-17, done).** Each town's
own catchment rate is blended (`PROVINCE_PRODUCTION_DOSE`) with its population share of
its province's countryside output: `province_good_potential` (belt × land use × depletion)
× workforce (`prov_rural / prov_cap`, clamped 0.2..1.5), refreshed yearly
(`refresh_province_production`). **The first cut failed** the inheritance gate at 1.0
(419/419 tick tests passed, `econ_` 5/6): the potential is calibrated to *mean
exploitation ≈ 1*, not to the towns' totals, so the takeover changed how much of each good
the world made, not just where. **Fix:** each good's provincial output is rescaled to sum
to exactly what the towns made on their own (`prov_prod_goods` marks goods the land can
produce; others stay with the towns). The takeover moves output toward better land and a
fuller workforce and never changes the total. Shipped at **1.0**: `tick::tests` 419/419,
`econ_` 6/6 (partible 82533/85359/64485 vs primogeniture 93544/105556/85893 per seed).
Gate: `the_countryside_takes_over_raw_production_without_changing_the_total`.

**Step 4: starting realms (2026-10-17, done).** `sim/shared/start_realms.rs`: whenever
provinces are generated, edited or merged, `generate_start_realms` groups them into
contiguous realms of random size (about 30% single-province free cities, the rest
2..16 provinces), each with a palette colour no neighbour uses, a capital (the most
populous province) and a name by size (Free City · County · Duchy · Kingdom), stored as
metadata `start_realms`. Commands `get/set/regenerate_start_realms`,
`get/set_realm_start_mode` ("continue" · "random" · "zero"). At campaign start
`apply_start_realms` makes them live through `found_start_realm` (a commonwealth founded
at the capital's seat, then given exactly the authored provinces, name and colour);
`Realm.color` now drives the map tint (`compute_states`). Gates:
`start_realms_cover_every_province_once_contiguously_with_varied_sizes`,
`a_start_realm_goes_live_with_its_provinces_and_the_crown_seats_governors`. The seed is
mixed with splitmix64 first: the plain `seed | 1` made seeds 42 and 43 identical, which
the test caught.

**Step 6: the one realm window (2026-10-17, built; not yet seen in a running app).**
`ui/campaign/RealmWindow.tsx` replaces the Realms panel (same toggle, `uiStore.showStates`)
and follows mock-up v2. On the left, a realm map drawn from the province raster, with Fill
(Realms · Cultures · Public order) and Show (Borders · Goods exports · Cities ·
Country → market). On the right, tabs: Overview · Government · Provinces · Goods. A
breadcrumb runs Realm › Province › town: click a province to open it (Province ·
Government tabs), then its town to open the city's own council (`CouncilChamber`).
Everything comes from one read, `campaign_realm_overview`: province rows with top-3
exports, the realm's goods surplus/deficit, cultures, economy index, power and its word,
the council, laws and governors. **Before a campaign** the same window edits the world's
starting realms: recolour from the 16-colour palette or a free picker, rename, assign
provinces by clicking the map, add a new realm, regenerate, and choose the campaign-start
mode. It opens from the Provinces browser ("👑 Starting realms"). The royal council is
derived where it is read (the ruler, kin, the richest houses, a famous notable, and a
delegate of the largest member city), so it cannot move the economy.

**Step 3c: the countryside staffs mines and estates (2026-10-17, done).** An estate's
output (mine, plantation, vineyard…) scales with its province's workforce,
`prov_rural / prov_cap` clamped 0.2..1.5 (`ESTATE_WORKFORCE_DOSE`): a mine in an emptied
province slows and one in a crowded province runs full. An estate with no province data
counts as fully staffed. Walked at 1.0: `tick::tests` 422/422, `econ_` 6/6; the numbers
moved (partible 73563 vs primogeniture 111250), so the gate did test it. Shipped at 1.0.

**Step 5: the crown's government (2026-10-17, done).** `tick/crown.rs`, yearly after
the realm consolidation passes:
- **Laws** per realm (`Realm.laws`), seated once from the realm's seed: crown tax and
  levies (light · moderate · heavy), succession, trade policy, the town fee, and minority
  tolerance.
- **Governors** for every province of a realm except the capital's
  (`CampaignSim.prov_governor`), named in the province's own culture, with loyalty and
  honesty. Loyalty drifts with the realm's cohesion and the province's unrest.
  - A dishonest governor SKIMS up to a quarter of the crown's dues. The skim stays in the
    seat town's treasury (he lives there), so no money vanishes.
  - The crown REPLACES a disloyal or greedy governor, and the realm chronicle records it.
  - A disloyal governor over restless people far from the capital may BREAK AWAY. His
    province becomes its own one-province realm, at most one a year world-wide.
- **Money:** the tax law scales what the crown collects (×0.8 / ×1 / ×1.25). Member
  cities (never the seat) pay the crown a town fee of 1-4% of their treasury a year. They
  always keep their own councils.

`CROWN_DOSE` was walked 0 → 1.0: `tick::tests` 421/421 and `econ_` 6/6 at both, with
**identical** `econ_` numbers. **No economy fixture has a realm holding provinces under a
governor, so the gate cannot see this mechanism.** It is shipped at 1.0, and
`a_disloyal_governor_breaks_away_with_his_province` and
`a_start_realm_goes_live_with_its_provinces_and_the_crown_seats_governors` are what test
it.

The **royal council** has no state of its own. It is derived when read (step 6), so it
cannot move the economy.

**Queued (rule 36):**
- Q-R1: levy, succession, trade-policy and tolerance laws have NO effect yet. They are
  shown, not enforced. Each needs its reader (levy → `raise_manpower_levy`; succession →
  `realm_family_pass`; trade → tariffs in `dispatch`; tolerance → row 05's acceptance
  drift) and its own dose walk.
- Q-R2: an economy fixture with start realms, so `econ_` can actually measure `CROWN_DOSE`.
- Q-R3: the player cannot yet change a law or dismiss a governor. These are verbs, the
  `campaign_set_province_tax` pattern.
- Q-R4: the realm window has not been opened in a running app.
