# Generator & Goods Redesign — deep analysis (2026-10-17)

Owner brief: the step-by-step generator is hard to understand, has missing factors and
weak UI/UX; goods feel wrong (gold and silver read as casual goods); wants a **goods
generator** (pick which goods exist, create new ones, categories that cannot break the
world); and wants to judge two game modes on top — a **trade-house player game** and a
**history simulator** for story makers. The generator is the product to polish first.

This document is analysis + questions only. Mock-ups come after the owner answers §6.
It builds on `GENERATION_REALISM_PLAN.md` (physics batches 0-6) and
`GENERATION_UX_REDESIGN_PLAN.md` (slices 1-11); items already planned there are marked
**[planned]** so nothing is proposed twice.

---

## 1. Why the generator feels like a mess — five root problems

These are cross-cutting. Fixing a single step will not fix them.

**P1 · The order and numbering are confusing.** The wizard runs
`Landmass(1) → World Characteristics(0) → Elevation(2) → Ocean(3) → Biomes & Climate(4) →
Rivers(5) → Soil(6) → Settlements(7) → Toponyms(12) → Goods(8) → Political(9) → Economy(10)`
(`WorkflowPanel.tsx:25-47`). Twelve steps, numbers out of order, and three of them are
"optional" or "preview" steps that look the same as the core ones.

**P2 · Three steps have no controls at all.** Ocean & Atmosphere, Biomes & Climate and
Soil & Resources contain only "show layer" buttons and a Generate button
(`StepOceanAtmo.tsx`, `StepClimate.tsx`, `StepSoilResources.tsx`). The user presses
Generate, a map changes, and nothing explains what happened or what they could change.
That is the main reason the process reads as "not understandable".

**P3 · Step names don't match what the steps do.**
- "Biomes & Climate" (4) does not make biomes; its own tooltip says biomes come from Step 6.
- "Soil & Resources" (6) makes soils, fertility, fisheries *and* biomes, but no resources.
- "Political" (9) makes no polities. It re-ranks settlements by trade power and draws
  influence circles.
- "Economy" (10) solves a market that the campaign then throws away on purpose
  (`lifecycle.rs:524-532`: "We deliberately do NOT seed prices from the static
  `compute_economy` market solve").

**P4 · The engine knows far more than the UI shows.** Settlement siting already scores
estuaries, deltas, mountain passes/saddles, straits, malaria lowlands and harbour premiums
(`settlements.rs:92-402, 753`). Deposits use real ore-genesis models (Potosí-style arcs,
cratons, placer gold; `deposits.rs:303-330`). None of this is visible to the user. The
world is more plausible than it looks, and the user can't tell *why* anything sits where
it does.

**P5 · No cause → effect explanation anywhere.** Nobody can click a cell and ask "why is
this a desert?" or "why is this city here?". Users trust a world generator when they can
read its reasoning. That is where "realistic" comes from.

**Proposed structure: 12 steps → 7 stages** (each stage has sub-views, not sub-steps):

| Stage | Contains today's | One sentence for the user |
|---|---|---|
| 1 Planet | World Characteristics (0) + a new planet size | "What kind of planet is this?" |
| 2 Land | Landmass (1) + sea level + plates | "Where is land and where is sea?" |
| 3 Relief | Elevation (2) + coasts, passes, volcanoes | "How high, how rough, where are the passes and harbours?" |
| 4 Climate | Ocean & Atmo (3) + Köppen (4) + ice | "Wind, sea, rain, seasons → climate zones" |
| 5 Water & Life | Rivers (5) + Soil/Biomes (6) + disease/forest | "Rivers, soils, forests, swamps, where people can live" |
| 6 Resources | Goods (8), redesigned (§3-4) | "What this world produces and how hard it is to get" |
| 7 People | Settlements (7) + Toponyms (12) + Political (9) | "Cities, cultures, names, first borders" |
| → Report | Economy (10) becomes a read-only report, not a step | "What trades with what, and why" |

Every stage gets the same frame: **Uses / Makes** line [planned], 3-5 main controls plus
Advanced, the staleness chip (done, `53747bb`), and a **"Why?" inspector** (new, §2.0).

---

## 2. Step-by-step analysis — at least 2 fixes/features per step

Format: **What it does now** · **Fix** (something broken or misleading) · **Feature** (new) ·
**Plausibility factor** (what real worlds have that this one lacks).

### 2.0 Cross-cutting: the "Why?" inspector (new, highest UX value)
Click any cell or city and get a short causal chain built from the values the engine
already stores, for example:
> *Hot desert (BWh).* 24° N, under the subtropical high. Cold current offshore
> (upwelling −3 °C) → dry onshore air. Rain 140 mm/yr. Rain shadow of the Kesh range to the west.

> *Port of Ammar.* Delta mouth (+++), sheltered coast (++), malaria lowland (−),
> 2 trade goods within 80 km, crossroads of 3 routes.

No new simulation is needed. The data is already in `CellInfo` and the settlement scoring
terms, so this is a formatting job, and it solves P4 and P5 at once.

### Stage 1 — Planet (today: World Characteristics, step 0)
Now: rotation (incl. retrograde), tilt, sunlight, greenhouse, eccentricity, dryness,
latitude frame, archetype presets. This is the best-built step.
- **Fix:** eccentricity either has to act on the climate or lose its slider **[planned]**.
  Today the user moves it and sees little.
- **Fix:** the step sits *after* Landmass but is numbered 0. Either put it first, or call it
  "Planet & Latitude" and say plainly "you can come back here after painting land".
- **Feature: planet size.** Distances are hard-wired to Earth (`KM_EQUATOR = 40075` is
  repeated in `query_commands/mod.rs:243`, `sim_commands.rs:1549`, `tick/mod.rs:441`).
  A fantasy author often wants a smaller or larger world. Size changes travel times, how far
  climate reaches inland, and the size of trade empires. One constant becomes a setting.
- **Feature: sea level / ice-age dial.** One slider that moves global temperature, ice
  volume and sea level together: an ice age exposes land bridges and widens the shelves.
- **Plausibility: tides.** Tidal range sets estuary ports, tidal flats, bay salt and
  harbour difficulty. It can be cheap: shelf width × basin shape → low/medium/high per coast.

### Stage 2 — Land (today: Landmass, step 1)
Now: paint, image template, plates (with seeds you can place), fjords/islands/smooth lasso tools.
- **Fix:** painted and template worlds have **no plate data**, so the geology downstream
  (mountain belts, ore models) falls back to guesses (`deposits.rs:1783` comment: "Template
  worlds carry no plate data"). Infer plausible plate boundaries from the painted coastline,
  or let the user draw boundary lines. Otherwise a painted world gets geology-blind mountains and mines.
- **Fix:** ocean fraction / continent-count controls were unreachable (UX plan F2/F3).
  Verify they are now on screen; if not, slices 1-2 of that plan come first.
- **Feature: land readout.** Land %, number of continents, largest landmass in km²,
  islands, coastline length. Authors need to know what they made.
- **Plausibility: passive vs active margins** **[planned]**. Atlantic-style wide shelves and
  Pacific-style trench coasts are why some coasts have great harbours and others have cliffs.

### Stage 3 — Relief (today: Elevation, step 2)
Now: modes, 4 sliders, presets, ridge-line drawing, scale/lock peaks, shelves. The
noise-hash fix is in progress (uncommitted `elevation.rs` diff), which is batch-0 item 1.
- **Fix:** finish the hash fix and re-measure the constants that depend on it (rivers,
  biological, deposits) **[planned, in progress]**.
- **Feature: erosion** (stream-power valleys) **[planned]**. This is the biggest visual
  realism jump left, because mountains currently look stamped on.
- **Feature: named passes as map objects.** Passes are already detected (`settlements.rs:361`)
  but only used as a score. Show them with names and make them toll and war chokepoints. The
  Alps, the Khyber and the Silk Road gates shaped history as much as rivers did.
- **Feature: coast types.** Cliff / beach / ria / fjord / delta / lagoon, plus a **harbour
  quality** value (shelter × depth × tide). Today a harbour is just `near_coast → 0.6`
  (`settlements.rs:396`). Port cities depend on this more than on anything else.
- **Plausibility: volcanoes and hotspot chains** with volcanic soils (Java, Naples, Etna
  slopes = dense farming) and hazard zones.

### Stage 4 — Climate (today: Ocean & Atmosphere 3 + Biomes & Climate 4)
Now: two Generate buttons and layer buttons. No controls, no explanation. The physics
underneath is strong (scored 68.8% against real Earth Köppen).
- **Fix:** the Köppen legend lists **16 classes**; the engine produces **33**
  (`StepClimate.tsx` vs `koppen.rs`). Show only the classes present on *this* map, with % of
  land. A legend that lacks what's on screen reads as "broken".
- **Fix:** merge steps 3 and 4. A user can't do anything useful between them, and the
  "re-run Ocean THEN Climate" rule has tripped users before (memory notes on batches 10-11).
- **Feature: seasonal sailing calendar.** Derive the monsoon reversal and storm season per
  sea from the wind fields that already exist. Today "trade season" is a hand knob in
  `TradeSettings.tsx`; it should come from the world.
- **Feature: climate brush (author override).** "Make this region wetter, drier, warmer",
  then everything downstream recomputes. Authors will always want "a desert here", and
  forcing it via physics knobs is frustrating.
- **Plausibility: rain variability / drought risk.** Not only average rain, but how much it
  swings year to year (high in steppe, monsoon margins and Mediterranean; low in oceanic
  climates). This is the missing link to the goods failure mechanic (§4.3): harvests should
  fail where weather is unreliable, not at random.
- Also **[planned]**: Stommel gyre solver, sea ice, Kottek s/w, tilt-driven monsoons.

### Stage 5 — Water & Life (today: Rivers 5 + Soil & Resources 6)
Now: rivers have 2 sliders; soil has none. Biomes are classified here despite step 4's name.
- **Fix:** runoff-based discharge and real river order **[planned]**. Boreal rivers were 2×
  too big until the cos-latitude fix (`ab29c3e`, done).
- **Feature: navigable reaches drawn on the map.** The code already finds mouths,
  confluences and the head of navigation (`lifecycle.rs` river landmarks). Draw
  "navigable to here" so the user sees why inland river towns exist.
- **Feature: hand-drawn rivers and lakes** for authors (draw a line, the solver carves it).
- **Feature: land cover as a resource**: forest reserve, pasture, wetland, permafrost. A
  campaign can then deforest, drain and overgraze.
- **Plausibility: disease ecology map.** Malaria is already used inside settlement scoring
  (`settlements.rs:402`) but never shown. A visible "fever lowlands / plague steppe /
  healthy highlands" layer explains why the Andes and the Ethiopian highlands were
  populated before the coasts. It feeds `campaign/tick/disease.rs` too.
- **Plausibility: carrying capacity** (people per km²) as a layer. It is the honest
  bridge from "fertile" to "where cities can grow".

### Stage 6 — Resources (today: Goods & Trade, step 8)
Now: 2 sliders (ore richness, climate strictness), salinity/storm/reef layers, show/hide goods.
The full analysis is in §3-4. Step-level fixes:
- **Fix:** the trade settings belong in People/Report, not here **[planned]**.
- **Fix:** storm and reef hazards are climate and ocean facts; move them to Stage 4.
- **Feature:** the Goods Generator (§4) is this stage.

### Stage 7 — People (today: Settlements 7 + Toponyms 12 + Political 9)
Now: density, limit, cultures, province size, manual placement and editing (good); names
(good); "political" = influence circles only.
- **Fix: settlements are placed before goods exist**, so no city is founded *because of* a
  mine or a pearl bank. Potosí sat at 4,000 m only because of silver. Add a second pass after
  Resources: "resource towns" (mining camps, salt towns, fishing villages) founded where a
  rich deposit has no city.
- **Fix: Political makes no polities.** Authors need borders at year 0. Grow initial
  realms / city-states / leagues outward from the strongest centres along travel cost,
  stopped by mountains and seas. The campaign already has `realms.rs` and `league.rs`, but
  generation has nothing.
- **Feature: "why here" card per city** (the §2.0 inspector), plus city age: old cores vs
  young frontier towns, so not every city is born on the same day.
- **Feature: history pre-roll.** "Run 50/100/200 years before I start", which feeds the
  history-simulator mode (§5.2) and gives authors lived-in borders.

### Report (today: Economy, step 10)
- **Fix:** it is a *preview* the campaign discards, so presenting it as a step misleads. Make
  it a read-only **World Trade Report**: who makes what, the top routes, chokepoints,
  monopolies. Or make the campaign actually start from it.
- **Feature:** export the report as part of the atlas PDF (UX plan slice 8).

---

## 3. Goods today — the diagnosis

### 3.1 Inventory
About **110 goods**: 45 built-in (`biological.rs:23-74`), 26 declarative
(`goods_spec.rs:639-752`: silver, gems, endemics, naval stores…), 29 manufactured
(cloth, jewelry, books, armour…), plus the deposit slice-3 minerals (mercury, alum, lapis,
turquoise, calamine, cobalt…). They sit in 15 loose "need categories".

Overlaps that muddy the market:
- `gemstones` **and** ruby/sapphire/emerald/diamond/amethyst/topaz
- `salt` **and** `bay_salt`
- `spices` **and** pepper/cloves/cinnamon/nutmeg/mace
- `incense` **and** `frankincense`
- `dyes` (marine) **and** indigo/tyrian_purple/fixed_dye
- `ceramics` and `glassware` exist both as **placed belt goods** and as **manufactured** goods

### 3.2 Why gold and silver feel casual — four concrete causes in the code

1. **Every good's output is normalised to its best producer.**
   `economy.rs:652`: `prod = prod / good_max[g] * abundance[g]`, then ×1000 per day
   (`lifecycle.rs:523`). So the world's best gold town makes roughly as many "units"
   as the best grain town.
2. **Deposit goods get an abundance floor of 0.6; luxuries get 0.15**
   (`economy.rs:548, 563`). So gold, by rule, is **4× more abundant than wine or silk**
   in volume terms. That is the opposite of reality.
3. **Gold sits in the same substitution category as iron, copper, tin, silver and lead**
   (`"metal"`, `biological.rs:206` and `goods_spec.rs:348`). The market
   (`market.rs:224-250`) shifts a city's need between category members by price, so a
   city short of iron will "substitute" gold. Gold is treated as a tool metal.
4. **The value gap is too small and has no physical unit.** Gold `base_value = 50` against
   grain = 1 (`biological.rs:228`). Prices are also clamped to 0.15×-12× base
   (`market.rs:153-154`). Historically, by weight, silver was worth ~2,000× wheat and gold
   ~20,000×. (13th-c. England: ~100 g of silver bought a quarter, ~218 kg, of wheat; gold:silver ≈ 1:10-12.)
   And because a "unit" has no weight, "1 unit of gold" and "1 unit of grain" are the same
   amount of stuff.

Also: built-in gold ships with neutral rarity (0.5, the same as grain), while declarative
silver is 0.74 (`biological.rs:138-145` vs `goods_spec.rs:746`).

**Conclusion:** the problem is not tuning. Goods have no **physical scale** (how much
exists, in what unit) and no **role** (food vs tool vs money). A goods system has to be
built on both.

---

## 4. Proposal — Goods DNA and the Goods Generator

### 4.1 Every good is described by six genes

| Gene | Values | What it drives |
|---|---|---|
| **Role** (why people want it) | Staple · Essential · Strategic · Comfort · Luxury · Treasure | Who buys it, how much, how urgently, and what substitutes for it |
| **Origin** (how it is obtained) | Grown · Raised · Gathered (wild) · Caught · Mined · Crafted | Which failure mechanics apply, labour, start-up time |
| **Geography** | Everywhere · Belt · Few homelands · One place (endemic) · Deposit | Map placement (existing `Distribution` enum maps 1:1) |
| **Value tier** (per weight) | Bulk ×1 · Common ×3-10 · Valuable ×30-100 · Precious ×300-1,000 · Treasure ×2,000+ | Price, and how far it is worth carrying |
| **Difficulty** 1-5 | ★ anyone can · ★★★★★ specialists, capital, years | How many producers can exist, chance of failure |
| **Keeping** | Perishable · Seasonal · Storable · Eternal | Spoilage, hoarding, whether it can be money |

Rarity then stops being one free slider. It comes out of **Geography × Difficulty ×
Value tier**: gold = Deposit × ★★★★ × Treasure, so very little of it exists in tonnes, but
it carries enormous value.

### 4.2 Physical units replace normalised output
Every good gets a real unit and a realistic yield, and production is computed in that
unit instead of being normalised to its best producer:
- crops: tonnes per km² of suitable land per year (wheat ≈ 0.5-1 t/ha medieval)
- mines: tonnes per working per year, set by grade (a rich gold district yields *kilograms*)
- fish: tonnes per km² of productive shelf
- crafted: from inputs × labour

The UI shows familiar units (sacks, barrels, bolts, ingots, ounces), and the market prices
per unit. Gold then becomes scarce automatically, with no special case.

### 4.3 Getting a good is hard, and it can fail
Each Origin has its own failure mode. Each mode should come from the world's geography,
not from a random roll:

| Origin | Difficulty comes from | Failure mode (event) | World source |
|---|---|---|---|
| Grown | soil, climate fit, years to first harvest (vines 3-5 yrs) | **Harvest failure**, correlated across a region | rain variability (new Stage-4 layer) |
| Raised | pasture, winter fodder | **Murrain** (herd disease), harsh winter | climate + disease layer |
| Gathered | distance into wild land, regrowth rate | **Over-harvest collapse** (furs, ivory, whales) | regrowth per good |
| Caught | shelf productivity, boats | **Stock collapse / shoal migration** (the Baltic herring that made and broke the Hansa) | fishery field + variability |
| Mined | **prospecting** (deposit hidden until found), capital, shaft depth | **Flood / collapse**, grade decline, exhaustion (depletion exists: `cities.rs:820-909`) | deposit grade/depth (exists) |
| Crafted | skilled guild, inputs, a **secret technique** (porcelain, silk) | **Lost know-how / stolen secret**, input shortage | city skill + diffusion |

Prospecting is the key new mechanic for treasure: at world start only some deposits are
*known*. The rest exist in the data and are found by expeditions (which already exist,
including hazards and failure), by mining colonies, or by chance. Gold rushes become possible.

### 4.4 Treasure goods behave like money, not like food
Gold, silver and gems (Role = Treasure, Keeping = Eternal):
- are **not consumed daily**. Demand comes from hoarding, coinage, dowries and temples,
  so they accumulate.
- never substitute for tool metals. Silver ↔ gold substitute **with each other** only,
  and the ratio between them (≈1:10-15) emerges from relative supply.
- link directly to `coinage.rs`: a city with a silver mine mints, and a city without one
  imports bullion or debases its coin. A new silver find causes **inflation** across its
  trade network, as Potosí did to Europe.

### 4.5 Categories that cannot break the world — "essential slots"
A world needs at least one good in each **essential slot** per inhabited region, or its
cities cannot grow. The generator checks this and warns; it does not refuse.

| Slot | Satisfied by (examples) |
|---|---|
| Staple grain | wheat, barley, rice, millet, maize, or any custom grain |
| Protein | fish, herds, legumes |
| Preservative | rock salt, bay salt |
| Fibre | wool, flax, cotton, hemp |
| Fuel & building | timber, peat, stone |
| Tool metal | iron (or copper + tin = bronze) |

Above those slots, every other category is optional: Comforts, Luxuries, Treasure, Naval
stores, Mounts. With this rule a world can have as few as **~8 goods** (one per slot plus
gold and one luxury) and still run. Presets:
- **Minimal** (~10): grain, fish, salt, wool, timber, iron, wine, spice, silver, gold
- **Classic medieval** (~30)
- **Age of spice** (~60, today's set cleaned up)
- **Fantasy**: start from any preset, add custom goods (mithril, dragon bone, moon-silk)
  through the same six genes

### 4.6 Cleaned-up default list (proposal)
- Merge the overlaps from §3.1. Spices become a *family*: pepper, cloves and nutmeg are
  members with shared demand. `gemstones` becomes the family name for the gem types,
  and the duplicate belt goods go.
- Crafted goods are never *placed* on the map. They appear only where inputs plus a skilled
  city exist.
- Each good shows a one-line **card**: icon · role · tier · difficulty stars · where it comes from
  · its failure risk.

### 4.7 The Goods Generator screen (to be mocked up after answers)
1. **Pick a preset** (Minimal / Classic / Spice / Fantasy).
2. **Goods table grouped by Role**, with toggles, and a slot checker showing ✓/⚠ per essential slot.
3. **Good card editor**: the six genes as dropdowns, then "where it grows" (the existing
   envelope editor in `GoodsEditor.tsx`: temperature, rain, elevation, soil).
4. **Live preview on the map**: belts and deposits appear as the user toggles goods. The
   existing `preview_good_score` path does this.
5. **World balance readout**: "Gold: 3 districts, ~2 t/yr, worth ≈ 40% of all grain";
   "Region X has no tool metal ⚠".

---

## 5. The two game modes — feasibility

### 5.1 Trade house / guild player mode ("Europa Universalis of trade")
**What exists already:** AI merchant houses with warehouses and futures, banks, coinage,
leagues, realms, wars, colonies, expeditions, plague, per-hub price **knowledge**
(`TickHub.known`, i.e. information is already local), carriage with vessels and transit
calendars. There is no player-controlled house (no `is_player` anywhere).

**What the mode needs:**
1. **Agency**: the player issues orders to *one* house: buy/sell, ship from A to B,
   build a warehouse, hire a captain, finance an expedition, lobby for an embargo.
   AI houses keep their current logic.
2. **Fog of trade**: the player only sees prices their factors and ships have reported, and
   those reports age. The `known` map is the seed of this. Scouting means sending agents
   or ships to learn markets. This is the core loop the owner described.
3. **Rival actions with consequences**: feuds, price wars (dumping), embargoes through a
   realm or league, sabotage, marriage alliances. Most "world" responses exist
   (leagues, wars, crashes); the player-facing verbs do not.
4. **Pacing**: today a full-size world takes **17-20 s per sim year** (SCOREBOARD
   2026-10-16), about 1.5 s per month. That is playable for a monthly-turn game, too slow
   for real-time days. A smaller default world for the game mode is the cheap fix.

**Verdict:** feasible, and the economy engine is the hard part, which is already built.
The new work is player verbs, the information fog UI, and goals/score. It depends on §4:
a trading game where gold is "casual" has no tension.

### 5.2 History simulator for story makers
**What exists already:** most of it. Realms, leagues, polis, government, war, schism,
hordes, culture acceptance, individuals, life events, succession: `campaign/tick/*`
runs all of it already, AI-only.

**What the mode needs:**
1. **A chronicle**: a readable timeline ("1043: the Ammar League forms against the
   Kesh salt monopoly"), with filters by realm, house and city, exportable as text.
2. **Map over time**: a borders/leagues slider, year by year (snapshots per decade).
3. **Author controls**: "what if" interventions (kill a ruler, cause a drought, discover
   silver here), then re-run from that point.
4. **Speed**: 200 years ≈ 1 hour at full size today. Story makers will accept a
   background "simulate overnight" run. Players will not.

**Verdict:** the closest of the three to done. It is mostly *presentation* of a simulation
that already runs. It also shares the most with the generator ("history pre-roll", §2 Stage 7).

### 5.3 Recommended order
1. Generator polish: §1 structure + §2 per-stage fixes.
2. Goods DNA + Goods Generator (§4). Both game modes depend on it, so it comes before either.
3. History simulator (mostly UI on the existing campaign).
4. Trade-house player mode (new agency layer).

---

## 6. Questions for the owner (answers drive the mock-ups)

**Generator**
1. Accept the **12 → 7 stages** structure, or keep the steps but rename and reorder them?
2. Is the **"Why?" inspector** (click → causal explanation) the right first UX win?
3. Should authors get **override brushes** (climate, rivers), even when the result is less
   physically "honest"? Or should physics always win?
4. **Planet size** as a setting: yes / no?
5. Should Stage 7 generate **initial borders/realms** at year 0, or should borders only come
   from a history pre-roll?

**Goods**
6. Accept **physical units** (tonnes, ounces, barrels) and real yields instead of normalised
   output? This changes every price in the campaign and needs re-tuning of the economy tests.
7. Should **gold/silver/gems be money-like** (hoarded, minted, causing inflation) rather than
   consumed?
8. **Prospecting**: should most deposits start *hidden* until found by expeditions?
9. Should **failures be geography-driven** (drought risk, disease map) or partly random
   events? (Recommendation: geography sets the odds, the dice decide the year.)
10. How **small a minimum goods set** should the generator allow? (Proposal: the 6 essential
    slots, warn but don't block.)
11. Should **fantasy goods** (mithril, dragon bone) get special genes such as "magical" or
    "monster-guarded", or should the same six genes be enough?
12. Merge the overlapping goods (§3.1)? This breaks nothing in saves, because old saves
    already don't load (`SAVE_FORMAT` gate).

**Game modes**
13. For the trade game: should the player be **one house in one city**, or a family that can
    open branches anywhere?
14. Should the trade game allow **violence** (hire privateers, sabotage), or only economic
    weapons (embargo, dumping, monopoly)?
15. Is the **history simulator** a separate product mode, or the same app's campaign with a
    chronicle view?
16. The feature freeze from `/court` (2026-10-15) is still formally on for everything except
    generation realism. Does this plan **lift it for goods** too? Goods sit on the boundary
    between generation and campaign.
