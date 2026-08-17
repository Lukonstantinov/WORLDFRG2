# World Realism Review — realms, goods, city placement

**Status: goods findings BUILT (§2). Realm findings §3.1-§3.3 and §3.5 BUILT;
§3.4's smaller points and ALL of §4 (city placement) remain DIAGNOSIS ONLY.**

A review of three areas against the historical record: how realms come into
being, how trade goods appear on the map, and how cities are placed. Written
because "is this realistic?" was unanswerable for two of the three — there is a
fidelity oracle for climate (§2.3 of `CLAUDE.md`) and one for the economy (§2.5),
and nothing at all for settlement placement.

Findings are recorded whether or not they were acted on, including the three
places where my own first reading of the code was **wrong** (§5) — a review that
quietly drops its errors can't be checked by the next reader.

---

## 1. Method

Read end to end: `sim/campaign/tick/realms.rs`, `sim/step7_settlements/
settlements.rs`, the goods chain (`good_score` → `localize_good` →
`localities.rs`), `goods_spec.rs`, and the two existing validation oracles.
Claims about dead state were verified by grepping for readers, not inferred from
the surrounding prose.

---

## 2. Goods — BUILT

### 2.1 The `clim_base` bug (a real, measurable mis-placement)

`good_score` folded the dry-winter/dry-summer Köppen variants onto their humid
equivalents **before** its match ran:

```rust
let k = clim_base(buf.koppen[i]);   // CWA→CFA, CWB→CFB, CWC→CFC, DW*→DF*, DS*→DF*
```

so `k` could never hold `CWA`/`CWB`/`CWC`, and **every match arm naming one was
unreachable**. The damage was not cosmetic:

| Good | Arm | Effect |
|---|---|---|
| tea | `CWB \| CWA => 1.0` | dead — scored **0.0** in Cwb, its home climate |
| coffee | `AW \| CWB => 1.0` | dead — scored **0.0** in Cwb |
| silk | `CSA \| CWB => 0.5` | dead — silently downgraded to 0.25 |
| wine | `CFB \| DSA \| DSB => 0.40` | dead — rescued only by its `med_like` fallback |

`CWB` is *subtropical highland, dry winter* — Darjeeling, Yunnan, the Ethiopian
and Kenyan highlands. Tea and coffee were being excluded from exactly where they
come from and placed by their weak fallback arms in the wrong climates instead.

**Fix:** score the RAW zone first, fall back to the folded zone only if the raw
zone yields nothing. This preserves what `clim_base` was for (a good that doesn't
care about winter dryness still scores in a Cw/Dw/Ds zone) while never zeroing a
good that named the dry-winter zone explicitly. The same rule now applies in
`envelope_score`, which previously read **raw** Köppen while built-ins read
**folded** Köppen — the same cell was scored under two different climate labels
depending on which scorer ran.

**Gate:** `dry_winter_zones_are_reachable` asserts the claim (a good naming a
dry-winter zone must score in it), and `the_humid_fold_still_applies_as_a_
fallback` asserts the fix didn't cost the fold its original purpose.

### 2.2 The cull

The catalogue had grown additively across four labelled rounds to **92 goods**
(45 built-in belts, 26 custom belt/deposit, 21 manufactured), and duplicates
accumulated:

- **`gemstones`** — a generic gem alongside **eleven** specific stones (jade,
  ruby, sapphire, emerald, diamond, amethyst, topaz, garnet, carnelian,
  lapis_lazuli, turquoise). §8.16 had already repurposed `gem_deposits` to mean
  ore *districts*, so the generic's original job no longer existed.
- **`dyes`** — marine murex purple, i.e. the same product as `tyrian_purple`. It
  was also the one good `goods_validation` carried as a standing coverage-floor
  **exception**, so retiring it removes that exception too.

Both are **retired, not deleted** (`enabled: false` by default): they are fixed
indices in `TileData.goods`, so removing a slot would break every saved
`.worldforge` (rule 7). Old worlds keep whatever their snapshot says.

### 2.3 Multi-origin goods (`GoodSpec.origins`)

`localize_good` picked one seed and flood-filled one homeland, so every seeded
good was a world monopoly. That is the *rarest* case in the record, not the usual
one. `origins` (serde-default 1, so every existing good and save is unchanged)
seeds N independent homelands, each repelling the others through the dispersion
penalty already used between different goods.

Defaults are historically grounded, not a variety knob: pepper (Malabar **and**
Sumatra), cinnamon (Ceylon **and** Chinese cassia), cotton (three independent
domestications — India, Peru, the Levant), incense (Hadhramaut **and** the Horn),
silk, sugar, coffee, spices, and a handful of broad domesticates at two apiece.

Two rules fell out of building it:
- The extreme-rarity homeland cap is a budget for the good **as a whole**, split
  between origins — two origins of a rare good are two small patches, not two
  full-size ones, which would double the world's supply of it.
- Only the FIRST origin may fall back to the least-bad cell. A second origin that
  can't clear the threshold simply doesn't exist on this world, which is the
  honest answer rather than a duplicate homeland shoved onto marginal ground.

### 2.4 Island endemics — the missing landmass pass

**There was no connected-component labelling of land anywhere in worldgen.**
`Domain::Island` was approximated as `distance_to_ocean < 0.20` — *near-coast
land*, which matched the entire coastal fringe of every continent. So an "island"
good was really a coastal good, and a true island endemic could not be expressed.

`LandmassContext` (one 8-connected BFS, wrap-aware, built once per world like
`RiverContext`/`GeoContext`) plus `Distribution::Endemic` fixes that. An endemic
is confined to ONE landmass and the flood-fill's island-jump is disabled, so its
belt physically cannot hop a strait.

This is why nutmeg was worth its weight in gold: not because the climate was rare
(nutmeg grows across the wet tropics) but because the *tree* only grew on ten
islands totalling 60 km². `rarity` cannot express that — it makes a good scarce
**everywhere** rather than abundant in one place and absent from the rest of the
world, which is the shape that actually produced the spice trade.

Six shipped: **nutmeg**, **mace** (deliberately sharing an envelope — two products
of one tree, the Banda relationship), **dragon's blood** (Socotra, an *arid*
island, hence a completely different envelope), **camphor** (Barus), **benzoin**
(Sumatra/Java), **sandalwood** (Timor/Sumba).

Two bugs found by measurement, not review — both silent-vanish failures:
1. `ISLAND_MAX_CELLS` as a fixed cell count is **resolution-dependent**: at
   3600×1800 a cell is ~11 km, on a test world ~133 km, so the same constant
   meant "Great Britain" on one world and "most of Eurasia" on another. Now
   `ISLAND_MAX_KM2`, converted per world — the same discipline the locality size
   ladder already uses.
2. `Domain::Island` and `Distribution::Endemic` were **fighting each other**: the
   domain gate zeroed the score on every continental cell *before* the
   distribution could choose a home, so all six endemics measured **zero cells**.
   Resolved by separating the two questions — DOMAIN says where the plant can
   grow (a wet tropical coast), DISTRIBUTION says how it is confined (the
   smallest landmass that scores, preferring a true island). The endemics are
   `Domain::Coastal`.

### 2.5 Terroir — why belts looked like smooth washes

Every scoring term varied over **hundreds of km**: Köppen zone, temperature,
precipitation, |latitude|, normalized elevation, and fertility (itself a smoothed
0.30-weighted blend). Nothing in the model varied at the 2–10 km scale at which
real crop distributions are actually mottled. `soil_type` (12 classes) was
computed by phase 6 and read by **nothing** in the goods layer; slope was never
computed at all.

`GoodSpec.soil` and `GoodSpec.relief` add those two channels. They live on the
**spec**, not on `Envelope`, deliberately — they must apply to built-in goods
(scored by the hardcoded matcher, which has no Envelope) and custom goods alike;
putting them in `Envelope` silently *replaced* a built-in's whole climate scorer
with an envelope holding only these two terms, which was my first attempt.

The table separates goods that **share a climate**, which is the point: vines
want stony sharply-drained ground on a slope and fail on heavy clay; rice wants
that clay precisely because it holds water; olives want thin calcareous soil that
would starve a cereal — all three are Mediterranean.

Two safety rules, both found by measurement:
- **Soil is a preference, never a veto.** An unclassified cell scores 1.0 (no
  information is not bad ground); a classified-but-unlisted soil keeps a floor.
- **`TERROIR_FLOOR`** remaps the whole multiplier into `[0.45, 1.0]`. Applied raw,
  it pushed `tea` and `saffron` under the seeding threshold and both placed
  literally nothing. Terroir shapes a belt's texture; it must not decide whether
  the belt exists. Same discipline as the locality pass's own FRINGE/FLOOR.
- **`saffron` is deliberately excluded** from the table: its belt is already
  tightly bounded by climate, elevation AND latitude, and one more gate moved it
  off every settlement's catchment and tripped the coverage floor.

### 2.6 The placement report

`GoodsPlacementReport`, persisted to `metadata["goods_report"]` (JSON, no tile
column, rule 7) and served by `get_goods_report`. Per good: cells, land share,
origins actually seeded, localities, notable names, mean grade, category — and,
the reason it exists, **flags**:

| Flag | Meaning |
|---|---|
| `absent` | placed nothing — this world has no suitable climate |
| `fallback_seed` | placed only because the seeder fell back to the least-bad cell |
| `ubiquitous` | a non-staple covering >25% of the world, almost always a scoring mistake |
| `single_cell` | a belt of ≤2 cells |

Before this, a good that silently failed to place was invisible until someone
went looking for it on the map. `fallback_seed` in particular was completely
unreported: the seeder falls back to the best passable cell *regardless of score*
(the "never silently vanish" rule applied to belts), so a good with no suitable
climate anywhere still appeared, somewhere implausible, with no indication why.

---

## 3. Realms — §3.1-§3.3 now BUILT (see §3.5); §3.4 still diagnosis

### 3.1 Three pieces of dead state

| Field | Status |
|---|---|
| `Realm.cohesion` | Set to `1.0` at founding and **never written again** outside tests. Its only reader is `realm_collection_efficiency`, so the plan's headline mechanism — "states were limited by what they could collect" — currently reduces to *distance alone*. |
| `Realm.rank` | Always `REALM_CITY_STATE`. `REALM_KINGDOM`/`GREAT_POWER`/`HEGEMON` are defined and **never assigned**, though `rank`'s own doc-comment describes a percentile ladder with hysteresis. A twenty-province realm titled "King" has rank `city_state`. |
| `Realm.legitimacy` | Written by regency and the Humiliate war goal. **Read by nothing** as a decision input — no revolt, no succession contest, no cohesion coupling. |

### 3.2 Every realm in this world is a merchant republic

Both eligibility paths run through a merchant house: a captured council/captor
seat, or ≥20% of a province's trade. That's Venice, Genoa, the Hansa — real, but
a genuinely *minority* route to statehood. The four dominant historical paths are
all absent:

- **Military entrepreneurship** — a warlord with a retinue takes a city (Norman
  Sicily, the Seljuks, the *condottieri*). No military class exists, and war is
  decided by `war_chest + treasury`, i.e. **cash**. Pre-17th-century war was
  limited by manpower and logistics, so a rich small realm beats a huge poor one
  — backwards for the era.
- **Tribal/chiefly aggregation** — Merovingians, Rus', Zulu.
- **Fission from an existing state** — a frontier governor going independent
  (Umayyad Córdoba, the diadochi). §6 of the realm plan explicitly forbids
  proclamation inside a realm, so this cannot happen.
- **Sacral legitimation** — a temple state or prophet-founder.

Concretely: a house with money crowns; a lord commanding 40,000 peasants in
`prov_rural` with no cash cannot. The causation runs the other way historically —
land → men → coercion → the ability to tax trade. The province layer already
holds `prov_rural`, `prov_tenure`, `prov_arable`, `prov_unrest`: the actual
substrate of pre-modern state formation. `maybe_proclaim_realms` reads **none of
it**.

### 3.3 Proposed: three paths, which also revive `cohesion`

Per the maintainer's decision (stateless start; merchants **+ powerful
settlements + cultural domination** unite provinces):

| Path | Trigger | Reads | Cohesion |
|---|---|---|---|
| **A · Merchant** (exists) | trade dominance or a captured seat | `trade_at`, `captor_house` | LOW — rich, fragile, borders follow trade networks |
| **B · Powerful settlement** | a tier-1 city proclaims for itself | `hub.tier`/`hub.standing` — computed by `assign_city_tiers` and documented as having **no readers** | MEDIUM — compact borders (Rome, Assur, Axum) |
| **C · Cultural domination** | a contiguous single-culture bloc of ≥N provinces | `prov_culture` + `prov_neighbors`, both already present | HIGH — culturally-bounded borders (Franks, Poles, Rus') |

Path B's state was built for exactly this and is currently unread. Path C is the
one that produces borders a player can *read*, and the one that most needs
`cohesion` to be live — its whole point is that it holds together better than a
conquest. Together they give `cohesion` its reason to exist: **high for cultural,
medium for civic/territorial, low for mercantile, decaying with each conquest of
a culturally foreign province.**

Path B also raises a real branch: a city proclaiming through its *council* has no
family. That argues for `Realm.government` (dynastic | civic), where a civic realm
has an empty `family`, succeeds by election, and never gets the title "King" —
the honest way to have Venice and Castile in one model.

With Paths B and C in place, `REALM_YEAR_FLOOR = 50` can go: the tier-1 absolute
floor is an emergent condition rather than a calendar date.

### 3.5 What was built — and the measurement that did NOT move

All three dead fields are now live, and the two non-merchant paths exist:

| Change | What it does |
|---|---|
| `Realm.founding_path` | `MERCHANT` / `CITY` / `CULTURE`, set at the coronation |
| `update_realm_cohesion` (yearly) | drifts cohesion toward the path's target, dragged down per culturally-foreign province held and nudged by legitimacy — so `realm_collection_efficiency` is no longer distance alone, and `legitimacy` finally has a reader |
| `assign_realm_ranks` (yearly) | the percentile ladder + top-rank absolute floor + hysteresis that `Realm.rank`'s own doc already described. Four axes: provinces, population, treasury, **cohesion** |
| `realm_title_for(rank, government)` | replaces the flat four-name list that styled a one-town house "King" |
| `Realm.government` + `found_civic_realm` | a republic: no `family`, no succession by birth, never a dynastic title |
| Path B (`maybe_proclaim_city_realms`) | a tier-1 city proclaims for itself — the first reader `hub.tier`/`hub.standing` has ever had |
| Path C (`maybe_proclaim_culture_realms`) | a contiguous single-culture bloc of ≥4 provinces unifies under its largest city |

**The negative result, recorded because it is the more useful half.** A matched
before/after of `econ_measure_realm_formation` (stash, run, restore) gives
**8 realms by year 170 both before and after**. The two new paths added exactly
zero on the reference world, and the reason is the *oracle*, not the mechanism:

- **Path C cannot run there at all.** `reference_world()` seeds `prov_culture` as
  `Culture{i}` — a different culture for every province — and never seeds
  `prov_neighbors`. So no contiguous same-culture bloc of any size can exist, and
  `maybe_proclaim_culture_realms` early-returns on the empty neighbour graph.
- **Path B has no tier-1 city to work with.** Tier 1 carries an absolute standing
  floor by design ("a tier that is always occupied carries no information"), and
  the fixture's 30 cities are too undifferentiated to clear it.

So both paths are gated by unit tests
(`a_powerful_city_can_proclaim_without_a_house`,
`a_culture_bloc_unifies_into_one_realm`, `a_bloc_below_the_minimum_does_not_unify`)
rather than by the funnel diagnostic, and **realms-per-century on a real
generated world remains unmeasured** — exactly as it was before this change, now
with the reason known. Making the reference world able to express a culture bloc
would be the right next step, but it changes `prov_culture`, which feeds
migration, so it must be done against the `econ_` scorecard rather than alongside
a mechanism change.

One bug found by the unit tests rather than by review: Path B's treasury bar used
the UPPER median of city treasuries, so on a small even-numbered world the richest
city was measured against itself and could never clear its own bar — the same
funnel collapse `realm_founding_cost` already had to fix once. It uses the lower
median now.

And one latent panic fixed on the way: `war.rs` resolved a sovereign hub's "true
ruler" by indexing `realms[ri].ruling_house` raw. A civic realm has no dynasty
(`ruling_house` is `u32::MAX`), so the first republic to win a war would have
crashed the tick. It resolves through `houses.get` and falls through to the
ordinary council path now.

### 3.6 Is "more realms" historically right? — yes, with one caveat

The question is worth answering directly rather than assuming, because the
instinct "more states = more realistic" could easily be wrong.

**The anchor is Charles Tilly's count of political units in Europe: roughly 500
around 1500, consolidating to about 25 by 1900.** The Holy Roman Empire alone
held some 300 polities — imperial free cities, prince-bishoprics, abbeys,
counties. Classical Greece carried on the order of 1,000 poleis in the Aegean.
Northern Italy ran 200-300 effectively sovereign communes between 1100 and 1300.
Mesopotamia had dozens of city-states by 2900 BC. At the urbanisation this model
simulates, **many small polities is the norm and a handful of large ones is the
anomaly.**

So: 8 realms on a world of 30 cities was too few by a wide margin, and 17 live
realms on 72 cities (≈4 cities per polity) sits comfortably inside the historical
range. The direction the maintainer asked for is the direction the evidence
supports.

Three further points where the model is now *more* right, not merely more
numerous:

- **A stateless frontier is the unhistorical outcome.** Merchant-only left 9 of 24
  provinces under no crown at all after two centuries. Unclaimed land existed at
  the margins of the pre-modern world, but not a third of a settled, urbanised
  region.
- **Mixed constitutions are the norm.** Venice and Genoa were republics while
  their neighbours were monarchies, and the Hansa was neither. A model producing
  only dynasties was making a claim about the era that the era does not support.
  The split now measures 7 dynastic to 10 civic.
- **A ladder of ranks is how the era actually looked.** Not 17 equal states but a
  few great powers over many small ones — the measured 5 city-states, 7 kingdoms,
  3 great powers, 2 hegemons is that shape.

**The caveat, and it is a real one.** Tilly's number is a *curve*: 500 units
collapsing to 25 over four centuries. This model has the fragmentation half and
**none of the consolidation half** — no personal union, no vassalising another
realm, no conquering a foreign capital (explicitly guarded off), no inherited
claims. So a world here reaches something like Europe in 1500 and then stays
there indefinitely. Making realms frequent is right for the *starting* condition
and, without consolidation, permanently wrong for the *trajectory*. That is the
single largest remaining gap in the realm layer, and it is now the binding one:
before this change the world had too few states to notice that they never merge.

### 3.4 Smaller points

- **The trade path is a very low bar.** No cost floor, tier gate waived, chance
  `base + share` → ~0.86/yr at 51% share. Realms-per-century on a real generated
  world is **unmeasured** — that's the first number to get.
- **Realms can only fragment, never consolidate.** Partition is built; annexing a
  foreign capital is guarded off; vassalising a realm and cross-realm marriage
  aren't built. The long-run attractor is many small realms — the inverse of
  history's consolidation trend. Personal union is the single biggest
  late-medieval state-formation mechanism in Europe and the genealogy already
  supports it.
- **`partition_realm` divides provinces round-robin by index**, producing
  checkerboard realms. Real divisions took coherent blocks (Verdun's three
  north–south strips; the Mongol uluses by campaign theatre).
- **Dynastic demography is thin:** no maternal mortality (a major driver of
  remarriage and instability), no concubinage or bastards, and
  `PERSON_CHILD_MORTALITY` gives ~26% cumulative under-5 against a real 30–50%.
- **Only two crown levies** (poll, customs) plus the tithe. The regalian
  monopolies — **salt, mint, mines** — were the backbone of pre-modern royal
  finance and are deferred. Salt in particular exists as a good; the monopoly
  doesn't.

---

## 4. City placement — DIAGNOSIS ONLY, NOT BUILT

The site-quality model is unusually rich (confluences, head-of-navigation,
estuary vs. ordinary river mouth, salt-lake shores, winter-severity gates,
disease drag) and better than most generators. Four issues:

1. **No oracle exists.** Climate has one, the economy has one; settlement
   placement has nothing. `zipf_slope` is computed in `economy_validation.rs` but
   only `assert!(...is_finite())`, and it measures the *campaign's* hubs on the
   synthetic fixture, not worldgen output. "Are these cities plausibly
   distributed?" is currently unanswerable.
2. **`civ_factor` fits the answer instead of modelling the cause.**
   `1.0 + 0.30·exp(−(|lat|−30)²/288)` — an explicit "civilisation peaks at 30°"
   bonus, applied on **any** world regardless of obliquity, so a 0°- or 45°-tilt
   world still gets a bonus at a latitude that isn't its habitable band. It also
   double-counts causes already in the model (growing season, irrigation,
   fertility). This is the one place in city placement that ignores the world's
   own parameters by construction.
3. **`FOOD_TO_POP = 25.0` is admittedly uncalibrated** and resolution-dependent,
   with tier thresholds at 100k/30k/5k. >100k was a top-20-in-the-world city
   before 1500; if the generator stamps dozens of "capitals", the whole
   population scale is inflated. Measurable, unmeasured.
4. **Spacing is ~295 km** (`min_dist = w/(95+90(1−d))` → ~26 cells at 3600 wide,
   `realism=0.55`). Defensible if the bottom tier is read as "regional market
   centre" and the real countryside lives in `prov_rural` — which is what the
   architecture does. But it means **all trade is long-distance**, when ~90% of
   pre-modern exchange happened inside a 30 km market radius; and one constant
   sets settlement count → province size → realm granularity for the whole game.

### 4.1 Proposed oracle — and its own failure modes

Two scorecards, because they fail independently:

- **SITE score** — of the top 50 settlements, what share sit on a coast,
  navigable river, confluence, estuary or head-of-navigation? On Earth the answer
  is overwhelming. This needs **no** assumptions about absolute population, so it
  should ship first.
- **SIZE score** — rank-size slope, counts above 100k/30k/5k, urban share,
  nearest-neighbour spacing.

Five ways the oracle could be wrong, and what to do:

1. **Zipf is a within-system law, not a global one.** Earth's *global* rank-size
   isn't a clean power law; it's the sum of several regional systems. A single
   global slope measures the wrong thing and could be "improved" by making the
   world more uniform — the opposite of realism. Compute it **per trade component
   / per continent** and score the distribution of slopes.
2. **The reference data is contested.** Chandler's pre-1500 city populations are
   widely criticised as over-precise, so absolute counts get **printed, not
   asserted** (§2.5's discipline).
3. **It can be gamed by its own inputs** — every settlement metric is a function
   of the `realism` slider and the explicit cap, which the *user* sets. The
   oracle must pin both, or it measures the slider.
4. **It conflates two failure modes** — wrong placement vs. a miscalibrated
   population formula. Hence the two-scorecard split.
5. **The latitude metric is circular while `civ_factor` exists** — the oracle
   would simply confirm the thumb on the scale. By §2.4, `civ_factor` has to be
   removed or independently justified **before** the latitude metric means
   anything.

---

## 5. Corrections to this review's own first reading

Recorded because a review whose errors are quietly dropped can't be checked.
Three claims in the first pass were **wrong**, caught by reading the code again
before editing it:

1. **"`salt` is labelled Rock Salt but scores solar evaporation."** Wrong. `salt`
   is already a `Deposits` good — `deposit_params(GOOD_SALT)` returns `Some`, and
   `default_model_for("salt")` is `EvaporiteBasin`, i.e. genuinely mined halite.
   The `good_score` arm I read is the suitability input, which the distribution
   overrides; `localize_good` never runs for it.
2. **"`iron` is an UNLIMITED belt."** Wrong, same reason — also a `Deposits` good.
3. **"`ceramics`/`glassware` are half-manufactured belt goods."** Wrong. Both are
   already converted to `Distribution::Manufactured` with real recipes (clay +
   timber; bay_salt + timber) in `default_list`, and a `clay` good exists.

The lesson generalises: `GoodSpec.distribution` is decided in `default_list` and
can override what a good's `good_score` arm appears to say. Read the spec, not
just the scorer.

---

## 6. Deliberately not built

Stated rather than silently dropped:

- **§3.4's smaller points and all of §4.** Realm consolidation (cross-realm
  marriage, personal union, vassalising a realm, conquering a foreign capital),
  contiguous partition instead of round-robin, dynastic demography (maternal
  mortality, bastards), the regalian monopolies, and the whole settlement-placement
  oracle are diagnosis only. Paths B and C, the rank ladder and live `cohesion`
  ARE built — see §3.5.
- **Diffusion over time.** A good's range is climate ∩ how far it had spread by a
  date. `origins` expresses several hearths but cannot animate spread — silk
  reaching Byzantium in 552 is unrepresentable without a time axis.
- **Old World / New World separation.** `LandmassContext` makes it cheap now (the
  same flood-fill labels continents), but nothing keys a good to a continent yet.
- **Knowledge as a scarcity axis.** Porcelain, Damascus steel and Venetian glass
  were scarce because of *process*, not climate, and a good's scarcity should be
  able to end. Needs a technology layer the project deliberately lacks (§5.1 —
  growth is exogenous).
- **Endemic value derived from island size/remoteness.** Argued for in review;
  the endemics ship with hand-set `rarity`/`base_value` instead.
- **An exhaustible good** (silphium, harvested to extinction).
  `prov_good_depletion` exists and extinction is one more rate, but it wasn't
  asked for.
- **The province plate's oversized locality squares.** A staple locality is
  900 km and a province is 200–400 km across, so one grain locality draws a
  square larger than the whole province. The fix is to draw the full-resolution
  belt mask clipped to the province footprint (a real per-cell layout, so rule 17
  permits it) rather than a radius square. Not done — it's a frontend change and
  wasn't confirmed as the view the report was about.
- **The world quality overlay's 8-cell blocks.** Coverage is full-resolution but
  quality still rides the old coarse grid; carrying a quantized value on the
  coverage runs would remove the second resolution entirely.
