# City · Province · War — the political layer

*The plan for the next three workstreams: a settlement panel that can be read, a
province layer that shows real land and real goods, and a political layer in which a
merchant family rules a city, a city becomes a state, and states go to war.*

**Status: planned, nothing built.** Every decision below was made explicitly by the
maintainer; nothing here is a suggestion looking for approval. What is *not* being
built is listed in §6 — that list is as binding as the rest.

Read `FIX_PLAN.md` first for the wider prioritisation and `SCOREBOARD.md` for what is
actually measured. This document extends `PROVINCE_SYSTEM_PLAN.md` (which describes
the province layer as shipped) and reverses one of its decisions — see §5.1.

---

## 1. Decisions

### 1.1 Provinces

| | |
|---|---|
| Size | Enclave fix **and** shrink globally **and** compress the fertile↔hostile spread |
| Enclaves | Illegal — **unless the province is its own island** (§5.1) |
| View base | Real terrain crop: relief / biome / elevation, with real river courses |
| Land use | Organic patches from a stable noise field biased by real elevation; never a grid |
| Estates | Pinned on the plate |
| Naming | Both names kept — *"the province of Kethvar, in the realm of Ashuran"* — in the controlling city's colour, controller named |

### 1.2 Goods and exploitation

| | |
|---|---|
| Listed | **Only goods actually produced here.** No "unexploited opportunity" view |
| Per good | capacity/yr · taken/yr · worked % · grade · **market ↔ local consumption split** |
| Potential | **Soft cap** — exceedable at rising cost, with depletion (never a hard stop) |
| Baseline | Rural population works the land at a negligible-but-nonzero rate |
| Quality | Lives on the **producer**, never per cell (§5.2) |
| Estate kinds | mine exhausts · fishery collapses and recovers · plantation wears soil · vineyard raises grade not tonnage · manufactory extracts nothing |
| Estate tier | **footprint + ceiling + grade** — explicitly *not* a higher rate on the same cells |

The tier choice is the design statement: an estate does not squeeze the same land
harder, it **improves the land, spreads across more of it, and makes finer goods**.
Overexploitation therefore arises from population pressure and many competing
estates, never from one well-run estate.

### 1.3 Politics

| | |
|---|---|
| City leader | An **office held by a house head** — reuses kin, character, vice, succession, crisis |
| City tiers | population+trade · treasury and fiscal reach · territory held · the leader's own standing |
| The state | **The city IS the state** (Sumer / Greek polis). A tier-2+ city holding provinces gains a state name, colour and border. A territorial empire above cities is a later, higher tier — not this plan |

### 1.4 War

| | |
|---|---|
| Shape | Abstract · quarterly rounds · hard round cap · its own window |
| Force | Levies + mercenaries + treasury, at **convex cost** — each increment dearer, so a large treasury has sharply diminishing returns and a small well-run city can win (the Venice/Genoa case) |
| Domains | **Sea and land are separate force pools** |
| Reach | Land adjacency through provinces, or naval range; force projected far costs more and attrits |
| Casualties | **Levy dead are people dead** (permanent population loss); mercenary losses cost only money |
| Score | One **bidirectional** bar, 100 = over. Runs **alongside** exhaustion, not instead of it |
| Exhaustion | Four independent paths: force broken · treasury and credit spent · war weariness · backers withdraw |
| Terms | **Priced in war score** — reparations 10 · trade rights 25 · tribute 40 · province 55 · annexation 90 |
| Outcomes | Province conquest · white peace · tribute / trade rights / annexation · reparations **in goods** paid over years |
| Houses | Forced levy **and** voluntary contracts (lend to the chest, supply goods at a war premium). Two houses backing opposite sides is a new feud cause |
| House fates | **Enemy sack** and **internal purge**; either may cascade to dissolution |
| Frequency | **Gated by conditions, not by a rate cap.** Rarity must be earned by preconditions and then *measured* — see §3.4f |

Both score and exhaustion are live because "winning but unable to continue" is the
outcome worth building for: it is how a victorious side is still forced to stop, and
how a losing side reaches a white peace.

---

## 2. New requirements folded in

**War must be legible as money.** Every war cost and gain becomes a line in the house
dossier's 📒 Accountant tab: forced levy paid · loan to the war chest · goods supplied
at the war premium · trade lost to blockade · manufactory damage · holdings lost to
sack or purge · profit or loss at the peace. For a merchant house a war *is* a
balance-sheet event, and it should read as one.

**Damage to trade and manufactories** reuses machinery that already exists.
`TickHub.damage` already suppresses estate output until repaired, and a manufactory is
an estate (`estate_kind` 6), so no new field is needed. `trade_wealth *= 0.8` already
models blockade; war extends it to block routes between belligerents specifically.
Neutral suppliers get the corresponding **war boom** — which is precisely why a house
wants to supply a war it is not fighting.

---

## 3. The steps

### Step 0 — Make the economy oracle see geography *(blocks everything)*

`economy_validation.rs:254` seeds a **uniform** province layer: every province
identical, seats on a straight line. The land pass runs, so the layer is bounded and
finite, but the oracle can measure *levels* and never *dispersion* — it cannot say why
one province is rich and its neighbour poor. Everything in workstreams 2 and 3 lands
unmeasured until this changes.

Seed a heterogeneous layer (varied soil, forest, capacity, seat spacing). Add
exploitation, market-share and war-frequency rows as **printed** metrics — §2.5's rule
is printed first, promoted to an assertion as the model earns it.

**Gate:** existing `econ_` bands unchanged · `simulate_decades_reports_dynamics`
bit-identical.

### Workstream 1 — The settlement panel

`src/ui/campaign/HubPanel.tsx` is 2,082 lines, 9 tabs, 24 sub-components, 360 px wide
(600 on Trade, so it jumps when you switch). The Summary tab stacks ten sections; the
People tab is rendered by two separate JSX blocks (1174–1463 and 1464–1520).

The structural diagnosis is that it is **not a settlement panel** — it is the whole
campaign UI filtered to one hub, because there was nowhere else to put per-hub views:

| Tab | Duplicates |
|---|---|
| Depots | `WarehousesPanel` |
| Supply | `ColonialPanel` |
| City finances | `MoneyFinancePanel` |
| Society / cultures | `PeoplesPanel` |
| People → migration | opens `ImmigrationPanel` outright |

**1.1 · Cut the redundancy.** Remove those views; replace with hand-off links that open
the real panel filtered to this hub. **9 tabs → 3** (Land & People · Trade · Power),
roughly 500 lines deleted. This is a genuine cut, not a hide.

**1.2 · Hierarchy.** A hero card above the tabs: population as one large number with a
sparkline, wealth, mood as a word and a colour, who rules, war state. Merge the two
duplicate `tab === "people"` blocks.

**1.3 · Visual weight.** Apply the *quiet when healthy* rule the house stability gauges
already follow — a sound treasury small and grey, a failing one the second thing you
see. Fix the width jump.

The panel shell is deliberately left at variant A (hero card + 3 tabs) rather than the
full dossier: the content cuts are identical either way, and the dossier shell arrives
with the city-tier and leader work that actually needs it (§3.1–3.2). No double work.

**Gate:** `npx tsc --noEmit` · every removed view reachable in ≤1 click · no data lost.

### Workstream 2 — Provinces

**2.1 · Enclave fix.** In `sim/shared/provinces.rs`, seed rejection currently reads
`too_close(&seed_cells, bx, by, local_sep2(i))` — it tests only the **candidate's own**
required separation, never the incumbent seed's. `local_sep2` is floored at 10 cells in
fertile land and up to ~2.6× larger in hostile land, so a fertile river valley inside a
desert or tundra region passes a test the surrounding province would have failed. That
asymmetry is the mechanism behind small provinces embedded in large ones.

Make the test symmetric — reject on `max(sep_candidate, sep_incumbent)` — then add a
post-pass that merges any province whose boundary touches exactly one neighbour,
**skipped when the province is on its own island**.

**The merge must run after `snap_borders_to_features`, not before** (§5.3).

**Gate:** `provinces::tests` must hold — crest affinity ≥3.1× · diagonal-river affinity
≥3.3× · `partition_is_deterministic` · `partition_covers_all_land_and_stats_are_sane`.
New test: no non-island province is enclosed by a single neighbour.

**2.2 · Sizing.** Shrink globally and compress the fertile↔hostile spread (today roughly
100× in area, plus `VAST_MERGE_CAP_FRAC` at 8% of the world per merged block). Retune
`base_sep`, the `1.0 + 1.6·hostile` ramp and `koppen_spacing_mult`. Report province
count and size distribution before and after at fixed granularity; determinism holds.

**2.3 · Terrain crop.** A new query command returning cropped **biome / elevation /
relief** for a province's bounding box — reusing `render/tile_image.rs` and the existing
LOD pyramid — plus real river polylines. This becomes the base layer of the survey
plate, masked to the province outline. Today `ProvinceMiniMap` receives only the
downsampled province-ID raster, which is why rivers are an honest scatter with no
course and there is no elevation or biome at all.

This is a **display-time read and does not violate the one-way snapshot** (§5.4).

**2.4 · Organic land use.** Replace the per-cell hash with a stable **noise field**
thresholded at the cumulative shares, biased by the real elevation underneath —
woodland uphill, arable on the flat. Proportions stay exactly the model's and a cell
keeps its class between years, so rule 17 holds and the checkerboard disappears.
Estates pinned on the plate.

**Gate:** rendered shares match `ProvinceLand` shares to within 1% · a cell's class is
stable across the year slider.

**2.5 · Goods and exploitation** *(the substantial item)*

```
potential[prov][good] = belt_score/255 · area · yield_const · land_use_share(good)
actual[prov][good]    = production of hubs + estates here, attributed
                        in proportion to belt score
exploitation          = actual ÷ potential
```

- **Potential tracks live land use**, not only the frozen belt (§5.2) — timber scales
  with `prov_forest`, grain with `prov_arable`, wool with `prov_pasture`. Without this,
  clearing forest does not reduce timber capacity and the entire feedback story fails.
- **Soft cap**: beyond 1.0 the marginal cost rises and a per-province-good `depletion`
  multiplier erodes potential, recovering when the pressure eases. This reuses the
  `prov_soil` wear/heal shape already shipped and proven, serde-defaulted, early-return
  on an empty layer.
- **Kind-specific consequences** as in §1.2.
- **Tier as footprint + ceiling + grade.**
- **Market ↔ local split** derived from the needs ladder and the flow accumulator. Most
  output never enters trade, which is the true pre-modern picture and worth showing.
- The Goods tab lists **only produced goods** — with the depletion caveat in §5.5.

**Gate:** `econ_` price and output bands hold · dynamics test bit-identical (it seeds
no provinces).

### Workstream 3 — Politics

**3.1 · City leader.** The office is the head of `council_house` / `captor_house`.
Surface their character, kin and vice; houses compete for the office; favours flow
through it. No new person entity — the whole house-person stack is reused.

**3.2 · City tiers.** `assign_city_tiers`, monthly, percentile-ranked among live cities,
mirroring `assign_house_tiers`: hysteresis on the cutoffs and an absolute floor on
Tier 1 so a young world has an empty Tier 1. Four axes as in §1.3. **Query-side only at
this step**, so it is provably bit-identical — house tiers shipped exactly this way.

**3.3 · The state.** A tier-2+ city holding provinces gains a state name (varied:
sometimes derived from the city, sometimes from the province), a colour from a new
palette — colour identity is house-only today (`houseColor` / `CoatOfArms`) — and a
border drawn from the union of held provinces. The province keeps its own name; the
state names the authority over it.

**This is where city tier stops being query-side** (§5.6).

**3.4a · War score.** A single bidirectional accumulator fed by round outcomes
(battles, blockades, raids, occupation). 100 ends the war outright; the four exhaustion
paths end it independently; the round cap is the termination guarantee of last resort.

**3.4b · Terms priced in score.** `apply_war_goal` becomes score-gated at the prices in
§1.4. This is the mechanism that stops province conquest happening on every marginal
victory, and it is the main protection for the top-10% wealth-share band.

**3.4c · Casus belli, expanded.** Beyond trade rivalry, contested monopoly, colony
independence and refused tribute:
- **A warmonger ruler** — read from the leader's existing boldness axis
  (`head_character_factor` axis 0). No new persisted state.
- **A house's war** — a feud that would have flared escalates into a full state war
  **only when that house holds the council or captor seat**, with the instigator
  automatically committed as a backer. This is the payoff of the whole leader design:
  capturing a government is what lets a family spend a city's blood on its own quarrel.

**3.4d · Houses broken by war** *(highest risk)*
- **Enemy sack** — a defeated or occupied city's resident houses lose estates, offices
  and warehouses there.
- **Internal purge** — a city turns on a house that financed a losing war; expulsion
  and confiscation.
- Either may cascade to full dissolution through the existing `dissolve_house`, which
  already funnels every dissolution path and writes off outstanding bank loans.

**3.4e · Ledger and damage.** Accountant lines for every war cost and gain; manufactory
and estate damage through the existing `damage` field; blockade on belligerent routes;
the neutral war boom.

**3.4f · Frequency.** Wars are gated by **conditions, not by a rate cap**. Preconditions:
reach satisfied, a real grievance, sufficient treasury, and — for a house-driven war —
council control. Rarity must then be *measured*, not assumed: add
`econ_measure_war_frequency` (`#[ignore]`d, beside `econ_diagnose_house_turnover` and
`econ_measure_foreign_hand_conjunction`) and report wars per century, mean duration and
outcome mix. Phase 4.4 set this precedent — the foreign hand was built only after its
own trigger rate was measured and justified. If the conditions produce implausible
numbers, tighten the *conditions*; do not add an artificial rate limiter.

---

## 4. Risks and the numbers to report

Three items can move measured behaviour. Each gets a gate that is **not its own
target** — §2.4's rule, and the one every reverted attempt on record violated.

| Item | Risk | Number to watch |
|---|---|---|
| 2.5 soft cap | A real constraint on production could move prices and output | `econ_` price + output bands |
| 3.4d house fates | **Two new dissolution paths, and no home-city-only safety valve** | Dissolutions/century — currently **33.33** |
| 3.4b + province conquest | Territory becomes losable for the first time | Top-10% wealth share — currently **0.651**, in its 0.60–0.90 band since Phase 5 |

Plus levy casualties as a **third mortality sink** alongside the urban graveyard and
plague (§5.7) → `econ_` population and urbanisation bands.

**On every politics step, report dissolutions/century and top-10% wealth share whether
they moved or not.** If the purge mechanic runs away, the fix is a higher threshold,
not a removed gate. A mechanism that measures badly is a negative result to write
down — §2.4 — not a failure to hide.

---

## 5. Caveats — read before building

### 5.1 Surviving enclaves are a documented DESIGN DECISION being reversed

`provinces.rs`'s module header states: *"provinces are NOT forced simply-connected, so
genuine enclaves/exclaves survive."* `PROVINCE_SYSTEM_PLAN.md` lists surviving enclaves
as a shipped Phase 1 feature. This plan **reverses that decision** on the maintainer's
explicit judgement: an enclave that is not an island reads as a generation artefact, not
as history.

Recorded here so a future session does not silently restore it as a bug fix. Note the
island exception **narrows** the original intent rather than contradicting it — a
genuinely separate landmass may still be its own province. Update the module header and
`PROVINCE_SYSTEM_PLAN.md` in the same commit as 2.1, per §2.7.

### 5.2 Potential must track live land use, not only the frozen belt

The phase-8 goods belts are frozen at finalize. If `potential` reads only from them,
then clearing forest never reduces timber capacity, and the exploitation layer becomes
scenery. Potential must be the belt **scaled by the province's current land-use share**
for that good. This is the single point on which the whole feature turns.

### 5.3 The enclave merge must run after the border snap

`snap_borders_to_features` (the marker-controlled watershed, §8.10) re-places border
*lines* after the cost-flood sets topology, and can itself create or heal an enclave.
Merging before it measures pre-snap topology and will both miss enclaves and merge
provinces that the snap would have separated.

### 5.4 Reading tiles for the province view is not a snapshot violation

CLAUDE.md §3.4 states the campaign never touches a tile after `campaign_start_sim`.
That rule governs the **tick**, and exists so 500-year runs stay fast. A read-only
*query command* fetching terrain for display is not in the tick and costs nothing per
year. Documented here so a future session does not "fix" it.

### 5.5 A depleted good vanishes from the list with no explanation

"Only produced goods" plus depletion means an exhausted mine's good simply disappears.
Keep a good listed while its `depletion` is non-zero or it was produced within the last
several years, so the record of exhaustion survives the exhaustion.

### 5.6 City tiers stop being bit-identical at 3.3

3.2 is query-side and provably bit-identical. 3.3 requires tier ≥ 2 to form a state, at
which point tier becomes load-bearing and the bit-identity guarantee ends. That is the
correct place for it to end; it must not be claimed after 3.3.

### 5.7 Three mortality sinks now compound

Urban-graveyard crowding mortality, plague (including Phase 4.3's kin toll), and now
levy casualties. Check the combined effect on population and urbanisation, not each in
isolation — three individually-bounded sinks can still crater a population together.

### 5.8 Reach means isolated cities never fight

A city with no reachable rival is permanently at peace. This is realistic and expected,
not a bug — but it means whole regions may see no war at all, and the war-frequency
diagnostic (3.4f) should report the share of cities that are structurally unable to
fight, so a low global war count is not misread as a broken trigger.

### 5.9 A house may still hold a province inside another city's state

Rule 24 already says province authority is never assumed to be a city. Adding
city-states adds a third case, and `prov_holder_house` must remain legal inside a
state's borders — that is exactly the Stato da Mar the Phase 5 work built. A state's
border is the union of provinces it holds; a province held by a *house* within that
area is not the state's, and the border must reflect that.

---

## 6. Deliberately not built

Stated rather than silently dropped:

- **Territorial empires — a `Realm` entity above cities.** Deferred behind the
  city-state by explicit decision. The city IS the state for now.
- **Sieges, fronts, army movement.** War stays abstract.
- **A rival house finishing an enemy under cover of war.** Sack and internal purge
  only.
- **Land state persisted back to tiles.** The world map still will not visibly change
  over 500 years; the province view remains the only place that change is legible.
  This is FIX_PLAN B1's remaining open item (b).
- **A per-cell quality field.** Quality stays on the producer, where it actually lives.
- **The unexploited-opportunity view.** Follows from "only produced goods".
- **Leagues, treaties, diplomacy** (FIX_PLAN B4). The natural rung after city-states,
  and the historically correct next one — Hanse, Lombard League, Delian League — but
  not this plan.

---

## 7. Order

```
0     Heterogeneous provinces in the econ oracle            ← blocks everything
1.1   Panel: cut redundancy (9 tabs → 3)
1.2   Panel: hero card, merge duplicate People blocks
1.3   Panel: visual weight, fix the width jump
2.1   Enclave fix (symmetric separation + post-snap merge, island exception)
2.2   Province sizing + compress the fertile↔hostile spread
2.3   Terrain crop command + view (relief / biome / elevation + real rivers)
2.4   Organic land use + estates on the plate
2.5   Goods, exploitation, depletion, market↔local split     ← risk
3.1   City leader (office on a house head)
3.2   City tiers (query-side only, bit-identical)
3.3   State name / colour / borders
3.4f  MEASURE war frequency before tuning anything
3.4a  War score + quarterly rounds
3.4b  Terms priced in score
3.4c  Casus belli incl. warmonger ruler + house-driven war
3.4e  Accountant lines, manufactory damage, blockade, war boom
3.4d  Sack and purge                                         ← highest risk, last
```

Gates on every step: `cargo test --lib econ_` · `simulate_decades_reports_dynamics` ·
`cargo check` · `npx tsc --noEmit`; plus `provinces::tests` on 2.1–2.2. The Earth gate
(`earth_`) is **not** needed — nothing in this plan touches `step3_ocean_atmo/` or
`step4_climate/`.

**3.4d is deliberately last.** It is the item most likely to disturb measured
behaviour, and placing it after everything else means that when a number moves, it is
clear what moved it.
