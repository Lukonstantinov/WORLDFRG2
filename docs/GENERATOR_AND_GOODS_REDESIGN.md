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

---
---

# ROUND 2 (2026-10-17, after the owner's first answers)

## 7. Owner decisions, round 1

| # | Question | Decision |
|---|---|---|
| 1 | 12 → 7 stages | **Yes, but iterate again to be sure.** Done in §8; the result is 6 stages + a report. |
| 2 | Author override brushes | **Yes.** |
| 3 | Physical units | **Yes. Metric (European) by default; imperial selectable in options.** |
| 4 | Treasure behaves like money | **Yes, and some goods get special value in certain regions/cultures** (§9.2). **By default the author picks how far each good has spread from its homeland, from endemic to everywhere** (§9.3). |
| 5 | Hidden deposits | **Tinted on the map while unknown; shown clearly once discovered** (§9.4). |
| 6 | Trade game: branches anywhere + violence | **Yes to all.** Deeper mechanics exist but are parked for now. |
| 7 | Lift the feature freeze for goods | **Yes.** Recorded in CLAUDE.md §0. |
| new | Layers | **Default layers turning on by themselves is annoying. Audit them and say what to delete.** That audit is §10. |

---

## 8. Stage structure, iterated: 6 stages + report

Re-checking §1's seven stages against the **real data dependencies** in the code found two
errors in the first draft.

**Error 1: the "Planet" stage reads nothing before Climate.** Every World Characteristics
knob (tilt, rotation, sunlight, greenhouse, eccentricity, dryness) is first read by
Ocean & Atmosphere (CLAUDE.md §3.5: "elevation ignores them"). A first stage holding only
these knobs is a stage whose choices do nothing visible until three stages later. That is
exactly the confusion P5 complains about. **Fix:** the knobs move *into* the Climate stage,
next to the Generate button they affect. This also solves P2, because the Climate stage
finally has controls. Two things stay up front, because they really are read early:
- **planet size**: every km-based constant in plates, noise and erosion reads it;
- **map frame** (where the equator sits, line proportion): you place it while looking at
  your land.

A **world archetype** picker ("Ice world", "Desert world", "Earth-like"…) also stays up
front as a one-click preset that pre-fills the Climate knobs. You state your intent early;
the physics acts where it belongs.

**Error 2: Resources must come before People, not after.** Checked in code:
- goods placement (`sim_biological`) takes rivers and the seed, not settlements;
- it reads **habitability**, a land property that today is only computed inside the
  settlements step (`settlements::write_habitability`);
- it reads the **culture map**, only to *name* notable localities
  (`localities.rs:241-251`, `name_notable_localities`);
- the trade matrix needs settlements, but that is the Report, not placement.

So: move habitability into Water & Life (it *is* the carrying-capacity layer §2 proposed),
move locality naming into People, and then People can read Resources. Mining towns, salt
towns and pearl-fishing villages are then founded **because** the resource is there (§2
Stage 7's "resource towns" fix), and the order matches history.

**Final structure:**

| # | Stage | Does | First reader of its output |
|---|---|---|---|
| 1 | **World & Land** | planet size · map frame · archetype preset · paint/template/plates · ocean % · continents · land readout | Relief |
| 2 | **Relief** | elevation model · mountains & passes · volcanoes · coasts & harbours · shelves · erosion | Climate, Water |
| 3 | **Climate** | the planet knobs + Generate → winds, currents, temperature, rain, Köppen, ice · seasons view · climate brush | Water & Life |
| 4 | **Water & Life** | rivers & lakes (+ draw tools) · soils · biomes · forest/pasture · disease · **habitability** | Resources, People |
| 5 | **Resources** | the Goods Builder (§9) → belts, homelands, deposits | People |
| 6 | **People** | cultures · settlements incl. resource towns · provinces · names (places, rivers, *localities*) · starting realms · which deposits are already known | Report, Campaign |
| → | **Report & Export** | trade report (today's Economy step) · atlas PDF · "Start campaign" | — |

What disappears as a separate step: World Characteristics (split between stages 1 and 3),
Toponyms (part of People), Political (replaced by real starting realms in People), and
Economy (becomes the Report).

**Gates this restructure must keep** (from the existing rules): rule 21 (Rivers needs
Köppen, still true: Water comes after Climate), §3.5 Earth-no-op of the planet knobs
(unchanged, they just live on a different panel), D1-D6 per-step = Run-All (commit
`423d219`). The Run-All must run the same new order.

---

## 9. Goods, iteration 2

### 9.1 Units (decision 3)
- **Stored internally in SI** (kg, litres, km). Prices are per kg or per litre. Display
  converts, so changing the setting never changes the simulation.
- **Display defaults: metric.** Grain and bulk goods in tonnes, liquids in litres or
  hectolitres, precious metals and gems in grams or kg, cloth in metres, timber in m³,
  livestock in head.
- **Option:** imperial (lb, bushels, gallons, troy ounces for gold and silver, yards). Lives in
  Settings → Appearance next to label typography, one switch.
- **Trade units as flavour, optional:** "sack", "barrel", "bolt", "ingot" can be *named* per
  good and per culture ("a last of herring", "a picul of pepper"). They display alongside
  metric and never replace it.

### 9.2 Regional and cultural value (decision 4), iterated
Prices already differ by region through supply and transport. That is the market, and it
stays. On top of it, each **culture** gets a small **valuation profile**: up to ~4 goods with
a role that changes how that people *wants* them, not what they cost to make.

| Valuation | Effect | Historical pattern it reproduces |
|---|---|---|
| **Sacred** | steady demand that ignores price (temples buy in famine years too) | incense in the Mediterranean, ghee/sandalwood in India |
| **Prestige** | elites pay a premium (×1.5-3) and display it | jade in China, horses on the steppe, amber in the Baltic, purple dye in Rome |
| **Staple preference** | the culture's own staple; substitutes cost it more | rice vs wheat vs millet cultures |
| **Taboo** | zero demand (the good does not sell there at all) | pork, alcohol, specific meats |
| **Currency preference** | which treasure metal it mints and hoards | silver in China and the Islamic world, gold in Venice and West Africa |

Currency preference is what makes treasure interesting. **The gold:silver ratio is computed
per region** from local supply and preference. When it differs between two regions,
bullion flows from one to the other. That is the real 16th-17th-century silver drain from
Europe to China, emerging from the model with no scripted event.

**Where profiles come from:** auto-derived by default from the culture's existing traits
(Martial → horses/arms prestige, Devout → sacred incense or candles, Mercantile → silver
coinage…) and from **what grows in its homeland** (a culture prizes the local wonder it is
known for). The author can edit any culture's profile in the Goods Builder.

### 9.3 Homeland and reach (decision 4)
Every placed good gets two settings in the builder:
- **Homeland(s):** auto-picked by suitability, OR **clicked on the map** by the author.
  Multiple homelands are allowed (`origins` already exists, `goods_spec.rs:224`).
- **Reach at year 0:** a 5-step slider:

| Reach | Meaning | Example |
|---|---|---|
| Endemic | only the homeland; cannot be grown elsewhere | nutmeg (Banda) |
| Local | homeland + ~300 km of similar land | silk early on |
| Regional | the homeland's region / sub-continent | wine around one sea |
| Continental | every suitable cell on the homeland's continent | maize before Columbus |
| Global | every suitable cell anywhere | wheat, timber, fish |

This replaces today's three hidden flags (`Distribution::Global/Local/Endemic`) with one
honest slider plus a map pin. The engine already has all three modes. Local/Regional need
only a km cap on the existing flood-fill.

**In the campaign, reach can grow:** crops spread along trade routes over decades (cotton,
sugar and citrus all did), and a secret good (silk, porcelain) spreads when its secret is
stolen. Rule: a good spreads only to land that already suits it, and only if a route
carries it there. This is a later slice, flagged so the reach setting is built to change.

### 9.4 Unknown deposits (decision 5)
- At world start a deposit is **known** if it sits within reach of a settlement that could
  plausibly work it (a known placer below a town, a salt pan beside a village). Everything
  else is **unknown**.
- **On the map:** unknown deposits are shown as a faint, desaturated tint over the general
  metallogenic belt ("this mountain country may hold silver"). They never appear as exact
  sites. A known deposit gets its full colour and its exact working icon.
- **Discovery in the campaign:** by expedition (they already exist, with hazards and
  failure), by a mining colony prospecting its province, or by a small yearly chance for a
  settlement next to it. Discovery is chronicled ("Silver found at Kesh, 1243").
- **Author view vs player view:** in the Forge the author can switch "show everything"
  on and off. The player in a campaign sees only what is known.

### 9.5 Goods Builder: the shape (mock-ups come after round 2)
Three levels so it never overwhelms:
1. **Preset** (Minimal ~10 / Classic ~30 / Spice Age ~60 / Fantasy) with a one-line summary
   of each, plus the essential-slot check (6 slots, ✓/⚠ per inhabited region).
2. **Goods list**, grouped by Role. Each row: icon · name · tier · difficulty · reach slider ·
   homeland pin · on/off. Most authors stop here.
3. **Good card** (click a row): the six genes; "where it can grow" (the existing envelope
   editor: temperature, rain, elevation, soil, relief); unit and trade-unit name; failure
   risk; cultural valuations that mention it. A live map preview updates as you edit.
Plus a **Culture valuations** tab (§9.2) and a **World balance** readout (how much gold
exists, which regions lack an essential).

---

## 10. Layer audit

### 10.1 Inventory
- **25 base layers** in the Toolbar (`Toolbar.tsx:59-113`): Land, Elevation, Hillshade,
  Ridges, Plates, Shelf · Currents, SST, Salinity, Fisheries · Temperature, Precipitation,
  Snow, Wind, Wind Speed · Köppen, Biomes, Natural · Soil, Fertility, Habitability · Storm,
  Reef, Disease.
- **~52 overlays**: 5 circulation (`Toolbar.tsx:119`), 11 general (`:126`), 25 "bio/political"
  (`:166`) plus provinces, toponyms (×7 sub-toggles), good belts (×~100), riverBreaks,
  travelRoute, goodFlow, goodScarcity and plate motion, which are switched from panels.
- **18 map plates** (`mapThemes.ts`) + 3 map styles + 10 elevation styles: three separate
  "how should the map look" systems.

### 10.2 Why layers appear by themselves: root cause
Overlays are **sticky**. Steps switch them ON and nothing ever switches them OFF:

| Who | Turns on | Where |
|---|---|---|
| Default state | migrations, colonies, riverBreaks (+ rivers, lakes, settlements) | `uiStore.ts:548-560` |
| Generate World (Run-All) | rivers, lakes, settlements, latLines, tradeRoutes, **then** chokepoints, politicalInfluence | `WorkflowPanel.tsx:165-166, 309-313` |
| Ocean & Atmosphere | wind, currents, latLines | `StepOceanAtmo.tsx:35-37` |
| Planet controls | latLines | `PlanetControls.tsx:72` |
| Goods & Trade | stormZones, reefZones, tradeRoutes, tradeFlows | `StepBiological.tsx:67-70` |
| Settlements | settlements, provinces | `StepSettlements.tsx:100, 135` |
| Political / Economy | politicalInfluence, chokepoints | `StepPolitical.tsx:53`, `StepEconomy.tsx:64-65` |
| Opening a world | toponyms (whenever names exist) | `App.tsx:769, 881`, `MapCanvas.tsx:1720` |

So after one Generate World the map draws **8 overlays** (rivers, lakes, settlements, river
breaks, lat lines, trade routes, chokepoints, trade hubs), plus toponyms when names exist.
A campaign then adds migrations and colonies, which are on by default and appear as soon as
there is data. None of it was the user's choice.

**Fix (one rule, not per-overlay patches):** *a stage's overlays are on loan.* Entering a
stage shows the overlays that stage is about. Leaving it restores the user's own set. Only
overlays the **user** toggled persist. After generation the map shows a clean default:
rivers, lakes, settlements, names. A campaign adds realms. Everything else is one click
away, and every panel's "show on map" button keeps working, because those are user actions.

### 10.3 Delete, merge, keep: recommendation

**Delete: broken or dead (verified)**
| Item | Why |
|---|---|
| Overlay **"Volcanoes"** (`markers`) | The toggle exists (`Toolbar.tsx:164`) but the draw code never reads `visibility.markers`, so it draws nothing. Volcanoes return properly with the Relief stage. |
| Base layer **Wind** | Muted land plus wind magnitude (`render_wind`); Wind Speed shows magnitude better and the Wind *overlay* shows direction. |
| Base layer **Currents** | A speed field. The Currents *overlay* (warm/cold streamlines) and SST tell the story better. |
| Base layers **Storm** and **Reef** | Duplicates of the Hurricanes / Reef Zones overlays, which the Nautical plate already uses. |
| Overlays **Trade Hubs** (`politicalInfluence`) and **Hub Names** | Products of the Political step (9), which §8 replaces with real starting realms. |
| Overlay **Trade Regions** | Internal clustering of the economy preview (≤14 regions), not a map fact. |

**Merge: the same information in two places**
| Merge | Into |
|---|---|
| Ridges + Plates | one **Plates** layer with boundary types drawn on it |
| Shelf | a sub-view of Relief (it is an editing aid, not a map) |
| Fishery Banks overlay | labels on the Fisheries layer |
| Monsoon Areas overlay | the planned summer/winter view of the Climate stage |
| Settlement Names + Hub Names | one **Names** toggle: major only / all |
| Trade Routes + Trade Corridors (worldgen) | one **Routes** overlay |
| Trade Flows (worldgen) + Dynamic Trade Flow (campaign) | one **Trade flow** overlay that shows the preview in the Forge and the live flow in a campaign |
| Lat Lines + Circulation Belts | **Lat Lines** becomes a real graticule: equator, tropics at ±tilt, polar circles at 90°−tilt, so it follows the planet's own tilt. **Circulation Belts** keeps the wind boundaries. Today Lat Lines is hard-coded at 0/±30/±60 (`OverlayManager.ts:2919-2925`), which is wrong on any non-Earth planet. |
| 18 plates + 3 map styles + 10 elevation styles | Plates = *what* to show (thematic). Styles = *how* it looks. Elevation styles become styles. |

**Keep, but off by default and owned by their panel** (campaign): House Control, Merchant
Routes, Futures, Caravan Corridors, Expeditions, Trade Heat, Trade Basins, Speculation,
Coin Dominance, Plague Zones, Guild Cities, Notable Figures, Landmarks, Dynasty Ties. Most
already switch on from their own panel. The Toolbar keeps a short **Campaign map** list:
Realms, Provinces, Trade flow, Migrations, Colonies, Plague.

**Keep as is:** Land, Elevation, Hillshade, SST, Salinity, Fisheries, Temperature,
Precipitation, Snow, Wind Speed, Köppen, Biomes, Natural, Soil, Fertility, Habitability,
Disease; the Rivers, Lakes, Settlements, Peoples, Wind (arrows), Currents (streamlines),
ITCZ and Toponym overlays.

**Result:** 25 → 18 base layers, and ~52 → ~26 overlays in the Toolbar. No information is
lost; everything deleted is broken, duplicated, or a by-product of a step that is being replaced.

**Not measured:** which layers users actually open (there is no usage telemetry). "Not
really used" above means *broken, duplicated or tied to a removed step*, verified in code,
not observed user behaviour.

---

## 11. Questions, round 2 (the last before mock-ups)

1. **Stages (§8):** accept 6 stages + Report, with the planet knobs inside Climate and a
   world-archetype preset in stage 1?
2. **Layers (§10.3):** approve the delete list and the merge list as written? Anything on
   them you use and want kept?
3. **Clean default after generation:** rivers, lakes, settlements and names only (campaign
   adds realms)? Or a different set?
4. **Cultural valuations (§9.2):** auto-derived from culture traits with author edits, or
   authored per culture from scratch?
5. **Reach over time (§9.3):** should crops spread along trade routes during a campaign, and
   secret goods spread by theft? Yes / later / no.
6. **Discovery (§9.4):** who discovers deposits: expeditions and mining colonies only, or
   also a slow chance for any nearby town?
7. **Planet size:** a setting (Earth / half / double / custom km)? From round 1, still open.
8. **Starting realms:** should People generate borders at year 0, or should borders come only
   from a history pre-roll ("simulate 100 years first")? Still open from round 1.
9. **Fantasy goods:** are the six genes enough, or do you want extra ones (e.g. "magical",
   "monster-guarded", "cursed")? Still open from round 1.
10. **Merge the duplicate goods** (gemstones vs individual gems, salt vs bay salt, spices vs
    pepper/cloves, incense vs frankincense, ceramics placed vs crafted)? Still open from round 1.

---

## 12. Owner decisions, round 2 (2026-10-17)

| # | Topic | Decision |
|---|---|---|
| 1 | 6 stages + Report | **Accepted.** |
| 2 | Layers / goods defaults | "By default enable all, but add the ability to create your own roster, good icon and good generator." Clarification pending (§13 Q1). |
| 3 | Clean default / realms in view | **Parked.** Realm logic is to be investigated in depth later; no decision on campaign defaults yet. |
| 4 | Cultural valuations | **Both:** a sensible default is derived, and the author can set any culture by hand. |
| 5 | Reach grows in campaign (crop spread, stolen secrets) | **Yes.** |
| 6 | Deposit discovery | **Nearby towns, and over time as a city explores deeper into its province, finding new and DEEPER deposits as its mining technology allows.** Deposits already carry `depth` and `depth_workability` (`deposits.rs:83-287`), so depth tiers have real data behind them. |
| 7 | Planet size | **A setting; default Earth-like.** |
| 8 | Borders at year 0 | **Yes: People generates starting borders.** |
| 9 | Fantasy genes | **The six properties are enough.** Fantasy goods are parked. |
| 10 | Merge duplicate goods | **Yes, but keep the quality factor.** A merged family (e.g. Gems) keeps each member's own grade/terroir and its own deposits; the family only shares demand and substitution. Merging never flattens quality. |
| 11 | Mock-ups | **HTML mock-ups are allowed for this project** (an exception to CLAUDE.md §2.2). They must look good and be iterated. |

## 13. Owner decisions, round 3 (2026-10-17)
1. **Layers:** delete only the dead toggle (Volcanoes) and true duplicates (Wind/Currents/
   Storm/Reef base layers, Trade Hubs, Hub Names, Trade Regions). Keep everything else
   available, and add **"My layer sets"** (save your own roster of layers). Overlays stop
   switching on by themselves (§10.2 rule).
2. **Mock-up look:** both: one in the current bronze-frame/ledger style, one alternative
   direction for comparison.
3. **Custom good icon:** both: a 24×24 pixel editor (start from any existing icon, recolour)
   **and** image upload (downscaled/pixelised to the house style).
4. **Mock-up order:** Goods Builder first, then the stage wizard + "Why?" inspector, then the
   layers panel.
