# Settlement Generation — Believability Analysis

*A scientist-and-historian review of how WorldForge 2 places, sizes, populates and
evolves its settlements, judged against the real demographic and urban history of
the medieval and colonial ages. Goal: **believable and unique** generations.*

Scope of the code reviewed:
- **Worldgen placement & sizing** — `sim/step7_settlements/settlements.rs`
  (`compute_habitability`, `compute_food_capacity`, `generate_settlements`)
- **Trade-development pass** — `commands/query_commands/political.rs`
  (`compute_settlement_development`)
- **Living campaign** — `sim/campaign/tick/{disease,cities,colonies,houses,war}.rs`
  (demographics, lifecycle, colonies, satellites, outposts, strata & pops)
- **Naming / culture** — `sim/shared/{names,cultures}.rs`

---

## 0. Executive verdict

The **site-selection** model is genuinely excellent — among the most historically
literate I have seen in a procedural generator. Confluences, fall-line / head-of-
navigation towns, estuaries vs ordinary river mouths, salt-lake trade shores,
desert oases, continentality-aware winter gates: these are real first-order
determinants of where pre-modern cities actually stood, and they are modelled with
care.

The **weaknesses are demographic and representational**, and they cluster in three
places:

1. **Population semantics.** A hub's `population` silently conflates *city* +
   *rural catchment*, but it is displayed and tier-labelled as **city population**.
   This makes "cities" read 5–10× too large for the medieval world.
2. **Demographic direction is backwards.** Pre-modern cities were population
   *sinks* (the "urban graveyard"); the model gives them an intrinsic **birth
   surplus** (`BIRTH_RATE > DEATH_RATE_BASE`), so they grow from within rather than
   by rural in-migration.
3. **Climate/latitude over-determinism.** A strong Mediterranean (Cs) climate bonus
   **and** a latitude bonus peaking at 30° **stack**, so every world grows the same
   "civilisation belt" at ~30° west-coasts. This is the single biggest threat to
   **unique** generations — worlds rhyme too closely.

Colonies, trade outposts and satellites are, by contrast, **well represented** and
historically grounded (chartered joint-stock colonies, grain colonies, feitoria
outposts, the Italian *contado* absorption). Details and fixes below.

---

## 1. Site selection & placement — mostly excellent ✅

`compute_habitability` scores every land cell as
`climate·0.40 + fertility·0.20 + water·0.20 + terrain·0.10 + trade·0.10`, gated
multiplicatively by temperature/winter/cryosphere/disease. `generate_settlements`
then takes local maxima with spacing, and sizes each site by an **emergent carrying
capacity** (food catchment + trade-access premium).

**What is historically right and should be preserved:**

- **River-node magnetism.** Confluences (St. Louis, Khartoum), the head of
  navigation / fall line (Richmond, the whole US fall-line row), estuaries and
  deltas as deep-water ports distinct from ordinary mouths — all explicitly
  scored. This is textbook historical geography and it is *rare* to see it done.
- **Salt as a trade magnet** (Salzburg, Timbuktu, the salt roads) via salt-lake
  shores; **oasis caravan towns** in arid Köppen zones. Correct.
- **Carrying capacity from a food catchment** (coarse-Voronoi, capped hinterland)
  plus **trade access** as the dominant driver of the *largest* cities — this is the
  right causal story: history's great cities are ports/mouths/crossroads, not the
  most fertile inland valley. Correct.
- **Continentality.** The winter-severity gate that lets Vladivostok/Harbin exist
  but denies them megacity scale, separate from a latitude-only cold tax, is
  sophisticated and correct.
- **Defensible-hill premium** (acropolis / hill-fort) alongside the farming-flatland
  score. Correct — many capitals sit on commanding relief, not the best soil.

**Where it over-fits and hurts *uniqueness* (fix priority: HIGH):**

- **Double Mediterranean bias.** `koppen_mod` gives Csa/Csb **+0.42** (the strongest
  of any climate) *and* `civ_factor` gives a Gaussian population bonus peaking at
  **|lat| ≈ 30°**. These reinforce each other, so every world clusters its cities in
  the same subtropical west-coast belt. Real civilisation also rose in **monsoon
  Asia (wet-rice: Aw/Am/Cwa), tropical uplands (Maya, Khmer, Ethiopian highlands),
  and cool-temperate river valleys**. The `Af` rainforest penalty (−0.10) and `Am`
  monsoon (0.0) actively suppress the very climates that fed the densest historical
  populations (the Ganges, the Mekong, Java). **Consequence:** worlds look
  Eurocentric and *interchangeable*.
- **Recommendation.** (a) Add a **wet-rice pathway**: in `Aw/Am/Cwa/Cfa` with a
  large river or ample precipitation, grant a *food-capacity* bonus (paddy
  double-cropping) rather than a habitability penalty. (b) **Decouple** the two
  Mediterranean signals — keep one strong and soften the other, or make which
  climate band is "the cradle" a **per-seed world trait** (a die-roll that this
  world's civilisation favours temperate valleys, or monsoon deltas, or the
  Mediterranean). That single change buys the most *uniqueness* per line of code.

---

## 2. Population & city size — the core believability gap ⚠️

### 2.1 A hub's population is a *region*, labelled as a *city*

`compute_food_capacity` builds per-cell food, and `generate_settlements` assigns
each site the food of its **whole coarse-Voronoi catchment** × `FOOD_TO_POP`. That
is a **regional carrying capacity** — city *plus* its rural hinterland. But the same
number is then bucketed by the size ladder:

```
population ≥ 100_000 → "capital"
population ≥  30_000 → "city"
population ≥   5_000 → "town"
else                → "village"
```

and shown to the user as the settlement's population. **Historically this is a
category error.** In 1300, a European town of **5,000 was a real town**, **10–20k a
major town**, and only a handful of places exceeded 40k (Paris ~200k, Venice/Milan/
Florence ~100k, London ~50–80k). If the displayed number is really *urban + rural*,
then a "30,000 city" corresponds to a town of maybe 3–6k in a region of 30k — a very
different mental image.

**Two coherent ways to fix it (pick one, don't half-do both):**

- **(A) Split urban from rural.** Carry an `urban_frac` per hub (historically ~8–15%
  of the catchment in most of Europe, up to ~30% in the Low Countries / N. Italy,
  rising with trade class). Display the **urban** number as "population", keep the
  catchment total as "region". Tier thresholds then use the urban number and can be
  lowered to medieval-real values. This is the *believable* choice and it composes
  well with the existing `hub_class` / `development_tier`.
- **(B) Relabel only.** If population is meant to be regional, rename the field in
  the UI ("catchment population") and lower the tier labels so a 5k *urban* place
  isn't called a "village". Cheaper, less satisfying.

**Recommendation:** (A). It is the highest-leverage believability fix in the whole
system, and it makes the existing rich social-strata / pops layer read correctly
(the underclass and commoner professions are *urban* strata but are currently
diluted by an implicit rural mass folded into the same number).

### 2.2 The urban-graveyard effect is inverted

In `disease.rs`, yearly demographic drift is `net = BIRTH_RATE·food_sec −
DEATH_RATE_BASE` with `BIRTH_RATE = 0.00006` (~+2.2%/yr) and `DEATH_RATE_BASE =
0.00002` (~−0.7%/yr) — a **structural birth surplus** for any fed city. Real
pre-modern cities had the **opposite**: crowding, contaminated water and endemic
disease made them *net mortality sinks* (London, Rome, Edo all needed a constant
stream of rural migrants just to hold steady). Growth came from **migration**, not
internal increase.

- **Consequence:** cities grow "from the inside" and the map's migration arrows are
  cosmetic rather than demographically load-bearing. It also makes a purely-urban
  world self-sustain, which shouldn't happen.
- **Recommendation:** make `DEATH_RATE_BASE` **scale with crowding / size** (an
  urban-mortality term rising with population density and falling with
  `public_health`, which already exists), so that above some size a city's natural
  balance goes negative and it must **pull rural migrants** to grow. The
  `economic_migration_pass` already draws people toward opportunity — this makes it
  the *engine* of urban growth instead of a garnish, which is both more realistic
  and more dynamic (a plague or a war that cuts the migrant supply now actually
  shrinks the metropolis).

### 2.3 Growth rates are a touch fast, but bounded — minor

`POP_GROWTH_RATE` peaks ~5%/yr and `SMALL_CITY_GROWTH_MULT` allows small towns up to
~5× that. Sustained 5% doubles a population in ~14 years, far above the ~0.1–0.3%/yr
pre-modern trend. It is *gated* (only well-fed, below-capacity, small) so it mostly
represents frontier/immigration booms, which did happen. **Low priority**, but if
§2.2 is adopted, re-tune these downward for the organic (non-migration) component.

### 2.4 The rare megacity is handled well ✅

`PRIMACY_DEV` restricting ~1M scale to a *single* coastal, trade-hub, top-treasury
regional capital that is actually provisioned (`food_sec` still gates it) is exactly
the right model — Rome, Chang'an, Constantinople, Baghdad were tribute-fed water-
connected imperial capitals, and they were *rare*. Keep this.

---

## 3. Colonial cities — well represented ✅ (with gaps)

`colonies.rs` distinguishes:

- **Settlement colony** (`colony_kind 1`) — a crowded, prosperous coastal
  metropolis founds a full daughter market via **joint-stock financing** (city
  treasury + a resident house + a same-continent **bank**, with proportional
  `backers` shares), seeds it with emigrants (relieving the parent), imposes a
  **trade-monopoly charter** (`apply_colony_charter`), and runs a **grain lifeline**
  of supply ships until it is self-feeding. It graduates outpost→city and may go
  autonomous.
- **Grain colony** — the **Greek Black-Sea / Crimea pattern**: a city under
  *sustained famine* self-funds (no bank needed) a farming colony on the most fertile
  reachable site; surplus flows back through the market.
- **Caravanserai** — road-side foundings between paired hubs.

**Historically this is strong** and correctly causal: chartered companies (VOC,
Hudson's Bay), Greek *apoikiai*, and the grain-colony survival move are all real and
distinct drivers, and the **rules are right** — only coastal metropoles colonise the
sea; inland cities can only plant inland; a bank on the same landmass is required to
underwrite and mint. The food-lifeline dependency (a colony that loses its supply
ships starves) is a genuinely good stability mechanic.

**Gaps to close (fix priority: MEDIUM):**

- **No return extraction beyond grain/monopoly.** Colonies exist largely to relieve
  crowding and secure food. The historical *point* of a colony was **extraction** —
  a cash-crop / bullion / fur flow back to the metropolis and its backers. Consider a
  **remittance** term: a share of the colony's trade-good surplus (esp. its unique
  frontier goods) routed to `backers` in proportion to their stakes. This turns
  colonies into an economic strategy, not just a demographic pressure valve.
- **Colonial cultural identity.** Culture-mixing on founding is handled
  (`record_migration_culture`), but a colony should **carry the parent's culture as
  its majority** and only slowly creolise, so a colonial network reads as one
  people's diaspora (the New Spain / New England pattern). Verify
  `create_market_colony` seeds parent culture, not the local hearth.
- **Autonomy / revolt.** Colonies can go `autonomous`, but a **war of independence**
  (a wealthy, populous colony throwing off the monopoly charter under high unrest)
  would be both historical and dramatic. The unrest/revolt machinery already exists
  in `cities.rs` — wiring colony autonomy to it is a small step.

---

## 4. Trade outposts — well represented ✅

Two flavours, both good:

- **Worldgen harsh-zone outposts** (`settlements.rs`, tail) — tiny supply posts in
  deserts / mild subarctic where towns won't form but a *shippable resource* exists:
  a whaling/fishing coast, a caravan oasis, a mountain ore lode, a volcanic mineral
  field. Correctly **excluded from ice caps and frozen shores** (no post on a
  sea-ice coast). Population 60–400. This is the **feitoria / factory** pattern
  (Portuguese trading posts, Arctic whaling stations, Saharan salt posts) and it is
  well judged.
- **Campaign house outposts** (`colony_kind 2`) — a great house plants a remote
  strategic post for reach / new goods, **hard-capped at `OUTPOST_MAX_POP`**. Correct
  — a factory is a warehouse-with-a-garrison, not a city, and the population cap
  enforces that it never balloons.

**Minor gap:** outposts are economic nodes but have little *risk texture* — a
frontier factory should be exposed (raids, being cut off, going dark). The event
system could target them specifically. Low priority.

---

## 5. Satellite cities — well represented ✅

Two mechanisms:

- **Absorption** (`maybe_absorb_dying_city`) — a tiny, failing, *free* town within
  `SATELLITE_MAX_KM` of a big healthy city is taken under its wing as a
  **satellite** (`colony_kind 3`): the metropolis relocates settlers, ships a
  founding grain grant, binds its trade, and mixes cultures. Bounded by
  `SATELLITE_MAX_PER_METRO`.
- **Construction** (`build_stage`) — a metropolis builds a suburb over ~10 years.

This is the **Italian *contado*** pattern (Florence, Venice, Milan absorbing the
towns of their surrounding district) and it is a legitimate, well-bounded model. The
absorption-instead-of-death path is a nice touch: it turns what would be a bare
"town dies" event into a believable metropolitan integration.

**Refinements (LOW):**

- Absorption is currently *rescue-only* (the target must be **struggling**). A
  powerful, ambitious metropolis historically also absorbed **healthy** nearby towns
  by conquest or purchase. A second, rarer path — a dominant city annexing a small
  *prosperous* neighbour — would round this out.
- A satellite should show its dependency in the UI story ("a satellite of {metro}")
  — confirm `settlementStory.ts` surfaces `colony_kind 3` / `founder_hub`.

---

## 6. Citizen life & the social fabric — rich, with era & specificity gaps

The campaign models a genuinely deep social layer: **strata** (patrician / burgher /
commoner / underclass) with prosperity-driven **mobility**; derived **Pops** (9
professions with `consciousness` / `militancy`, Victoria-style); **sentiment**
(food / prosperity / stability → mood); **unrest → riots → revolts** that seize
house wealth and topple councils; **cultural minorities**, discontent and
ethnogenesis. This is well above the bar for the genre.

**Gaps, as a historian would flag them (fix priority: MEDIUM):**

1. **Occupational structure is decoupled from the actual city.** `derive_pops`
   splits professions by **fixed fractions of the strata shares**, so a **fishing
   outpost, a mining post and an inland guild town get the same profession mix**
   (scaled only by size). Historically occupation was intensely place-specific — a
   port is sailors, chandlers and shipwrights; a mining town is miners and smiths; a
   cathedral city is clergy, masons and pilgrims' innkeepers. **Recommendation:** tilt
   the profession split by the hub's **dominant production goods and `hub_class`**
   (the data already exists in `production` / `estate_kind`). This is the single
   change that would most make each settlement feel *individual* rather than a
   scaled-up copy.
2. **Anachronism across the era span.** The Pop set includes **"capitalists" and
   "clerks"** — early-modern/colonial categories — applied uniformly even in a
   "medieval" world. Consider an **era-gated** profession set (guildmasters /
   journeymen / clergy / retainers in the medieval frame; capitalists / clerks /
   factory hands once the world reaches an early-modern stage), keyed off the
   existing era/`development_tier` machinery.
3. **The Church is missing as an urban force.** There is a "clergy" profession and a
   "Devout" culture, but **no ecclesiastical driver of settlement**: many medieval
   cities *were* bishoprics, cathedral towns, pilgrimage sites and monastic
   foundations (Canterbury, Santiago, Cologne). A modest **religious/institutional
   site premium** (a chance for a habitable site to become a bishopric/pilgrimage
   town, boosting habitability, stability and a "clergy" pop weight) would add a
   whole believable class of town and more *unique* identities.
4. **No craft-guild social structure at the citizen level.** Guilds exist as
   *economic houses*, but the medieval urban social reality — masters, journeymen,
   apprentices, guild membership gating who may practise a craft — isn't modelled.
   This is optional depth, but it is *the* defining institution of the medieval town.
5. **Universities / seats of learning** (Bologna, Paris, Oxford, Salamanca) as a
   rare town-identity driver — optional flavour, high uniqueness-per-town.

---

## 7. Obsolete / inconsistent info to clean up

- **Doc drift on good count.** `CLAUDE.md` §8.4 says **"21 belts"** while §3.3, the
  goods-spec section and `market.rs` all say **`GOODS_COUNT` = 45 builtins**. The 21
  refers to the pre-expansion belt count. **Fix the §8.4 line** (belts are now the 45
  goods, minus the manufactured/deposit distributions that have no per-cell belt).
- **Two parallel classification axes are undocumented as such.** `Settlement.size`
  (village/town/city/capital, demographic) and `development_tier`
  (Outpost/Market/Guild Town/Free City/Emporium, institutional) plus `hub_class`
  (ordinary/trade hub/entrepôt, commercial) are **three different things** and that
  is *good design* (a small institutionally-deep Venice should out-rank a large
  backwater). But it is easy to misread as redundant — **document the three axes**
  and their intent in `CLAUDE.md` so future work doesn't "reconcile" them into one.
- **`compute_settlement_development` is a static one-shot** that scales worldgen
  population by nearest-hub wealth (`DEV_GAIN`, cap ×3). Once a **campaign** is
  running, the far richer `disease.rs` carrying-capacity model owns population, so
  this pass is only a *pre-campaign* cosmetic uplift. Worth a comment noting it is
  superseded in-campaign, to avoid confusion about which model is authoritative.

---

## 8. Prioritised recommendation summary

| # | Change | Axis | Effort | Priority |
|---|--------|------|--------|----------|
| 1 | Split **urban vs. catchment** population; tier on the urban number | Believability | M | **HIGH** |
| 2 | Invert demographics to **migration-driven** urban growth (crowding mortality) | Believability | M | **HIGH** |
| 3 | **Decouple / per-seed** the Mediterranean+30° double bias; add a **wet-rice** food path | Uniqueness | S–M | **HIGH** |
| 4 | Tilt **Pop professions by the city's actual economy** (port/mine/cathedral) | Believability+Uniqueness | S | MED |
| 5 | **Era-gate** the profession set (no medieval "capitalists") | Believability | S | MED |
| 6 | Colony **remittance / extraction** flow back to backers; colonial **culture inheritance**; independence revolts | Believability | M | MED |
| 7 | **Religious/institutional** site driver (bishopric / pilgrimage / university towns) | Uniqueness | M | MED |
| 8 | Doc fixes: 21→45 belts; document the three classification axes; note the superseded dev pass | Hygiene | S | LOW |
| 9 | Satellite **annexation of healthy** neighbours; outpost **frontier-risk** events | Depth | S | LOW |

**The three HIGH items are the ones that deliver the stated goal.** #1 and #2 make
the *numbers* and *dynamics* of citizen life read like a real medieval/colonial
city; #3 is what stops every generated world from rhyming, giving genuinely **unique**
civilisational geographies. Colonies, outposts and satellites are already believable
and need only the medium-priority polish in §3, §4 and §5.

*This document is analysis only — no simulation code was changed. Implementing any
item above touches `sim/campaign/tick/` and must be followed by the standing
dynamics test (`cargo test --lib simulate_decades_reports_dynamics -- --nocapture`)
per CLAUDE.md §2.1.*
