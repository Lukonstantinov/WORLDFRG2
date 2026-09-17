# Places, Demand and Growth — a build plan

> **Status: AGREED IN SCOPE, NOTHING BUILT.** Written from a measurement pass over
> `sim/step7_settlements/settlements.rs`, `sim/shared/provinces.rs`,
> `sim/campaign/tick/disease.rs` (the growth pass) and
> `sim/campaign/tick/production.rs` (the demand pass). Companion plan:
> `MONEY_MINES_AND_GOODS_PLAN.md` (banking, mining, deposit goods). The two are
> independent; that one should land first, because it is the smaller change.

Three subjects that turn out to be one question — **what makes a place worth
anything?**

```
where towns are        →  one main hub per province, secondaries beneath it
how big they get       →  capacity from the LAND, not from what they were founded as
why anyone trades      →  a good is worth more the further it came, and worth
                          less where it is made
```

The third is the one with a name: **Venice sold salt it did not eat, and Rome paid
for Baltic amber that Balts wore without ceremony.** The model currently cannot
express either sentence.

---

## 1 · Measured findings

### F1 · Settlements are one flat pass; there is no "main" settlement

`generate_settlements` (`settlements.rs:509`) is a **single greedy pass**:

1. every land cell above a habitability threshold that is a local maximum becomes
   a candidate (`:542-554`);
2. candidates sort by score and are taken greedily under ONE spacing radius
   (`min_dist`, halved on river cells) (`:560-580`);
3. population and tier are assigned **afterwards** from carrying capacity
   (`:642-713`).

So a "capital" is simply a town that happened to score well. There is no primary
pass, no secondary pass, and no structural notion of a regional centre.

### F2 · Provinces are seeded from settlements, so nothing guarantees a seat

Provinces run at phase **7b**, after settlements, and `generate_provinces`
(`provinces.rs:1013`) seeds them in two stages (`:1179-1215`):

- **settlement seeds** — sorted biggest-population first, each becomes a seed if it
  clears the local separation from every existing seed;
- **filler seeds** — a jittered fine grid over the remaining land.

A filler-seeded province contains **no settlement at all**. A settlement too close
to a bigger one is absorbed into that province as a secondary town. So a province
may hold zero, one, or five towns of comparable rank, and `Province.settlements`
("seat first") can be empty.

`repair_province_settlements` exists precisely because this order produces
provinces whose seat is wrong.

**This is closer to the requested design than it looks.** The machinery for
"big places seed provinces, small places fall inside them" is already there and
already sorts by importance. What is missing is (a) a real primary/secondary split
in settlement generation, and (b) a guarantee that a filler province gets a seat.

### F3 · Manual settlements already survive regeneration

`StepSettlements.tsx:88-90` keeps every `manual || edited` settlement and generates
the fresh batch around them; `Settlement.manual`/`.edited` are serde-defaulted
(`settlements.rs:22-33`) and `place_settlement_at` (`settlements.rs:1080`) already
scores a hand-placed site through the same food/trade model.

So "hand-place the province capitals, then let the generator fill in" needs **no
new persistence** — only a flag and a change of order.

### F4 · A city's ceiling is a multiple of what it was FOUNDED as

The growth model is a real logistic (`disease.rs:500-556`) with genuine terms —
food security, prosperity, trade development, public health, world age, a
small-city boost, a colony boom, a mining boom. And then:

```rust
let capacity = (self.hubs[h].founding_pop * cap_mult)
    .max(self.hubs[h].founding_pop * 0.15);
```

**Capacity is anchored on `founding_pop`.** A town founded at 500 has its ceiling
pinned to 500 forever, however good its site or its trade become. A perfectly-sited
small town cannot overtake a badly-sited large one — the founding roll decides the
ranking for all time, and every other term only scales it.

`prov_cap` — the province's real rural carrying capacity, computed yearly from
land use, soil and works (`cities.rs:224-229`) — already exists and is *exactly*
the right denominator. Nothing in the city growth pass reads it.

### F5 · Eight founding paths, none province-aware

`maybe_found_satellite` · `maybe_found_caravanserai` · `maybe_found_settlement_colony`
· `maybe_found_mining_colony` · `maybe_found_food_colony` · `maybe_found_estate` ·
`maybe_found_house_outpost` · `maybe_found_route_post`. Not one asks *"does this
province have a hub yet?"* A province can stay empty for 500 years while three
colonies are founded around one crowded coast.

### F6 · Foreign craving is a BINARY SWITCH, and it is the wrong way round

`base_need` (`production.rs:895-903`):

```rust
let local = self.hubs[h].base_per_capita.get(g).copied().unwrap_or(0.0);
if local < 1e-4 {
    foreign_lux = 1.0 + tier_gain * prestige;
}
```

A city that produces **any** amount of a comfort/luxury good gets **zero** import
craving for it. A city that produces none gets the full bonus. There is no
gradient, no distance term, no provenance and no culture axis.

`prestige` is `(base_value / 15.0).clamp(0.4, 1.6)` — a global constant per good.
So every city on the map values amber identically, which is the reported problem
stated exactly: *"amber in the Baltic is not that valued; exported amber in Rome,
Romans value it way more."*

### F7 · Nothing anywhere knows where a good CAME FROM

`TickHub.stock` is a flat pool per good. Once Cypriot salt lands in Venice it is
indistinguishable from Venetian salt: `stock_of` sums it, `stock_take` spends it,
`live_price` prices it. **"Imported goods are more desired" is currently
inexpressible at the market level** — `foreign_lux` can only approximate it at the
need level, and does so with a binary switch.

> **The hook already exists.** `stock` is not flat — it carries a **GRADE**
> dimension (`GRADE_BANDS = 3`: coarse / common / fine, `mod.rs:5197-5200`)
> threaded through `stock_of` / `stock_add` / `stock_take` /
> `stock_take_finest_first` and surfaced in the warehouse panel. A provenance axis
> has a proven shape to copy and a UI that already reads a banded stock.

### F8 · Distance costs freight but confers no value

`good_freight` prices a leg (and since C1b carries a `sea: bool` mode term), so a
distant good is **dearer**. But nothing makes it **more wanted**. Historically for
a pre-modern luxury these are the same fact seen twice: pepper was precious
*because* it came from the Indies, and the price the Rialto could charge was a
function of how hard the voyage was, not only of what it cost.

The requested model — *"if it's hard logistically to import a good then it is
expensive"* — is half-built: the cost half works, the desire half does not exist.

### F9 · A city keeps far too much of its own luxury output

Reported: *a city produces a luxury and consumes it, leaving ~30% for market.*

Confirmed as an outcome, not a single line. `TRADE_RESERVE_MULT` is only **1.1
days** of need (`mod.rs:159`), so the export reserve is not the cause. The cause is
that local need is simply large relative to production:

- `need_scale` is **ONE aggregate scalar over all 45 goods** (self-calibrated at
  campaign start), so at most the AVERAGE is right and each individual good's level
  is an accident of two tables nobody compared — already recorded in
  `CONSUMPTION_AND_GOODS_REVIEW.md`;
- the producing city gets `foreign_lux = 1.0` (F6) but keeps its full base demand,
  so producing a good suppresses nothing.

**Historically a luxury-producing region consumed a small fraction of its own
output.** Moluccan clove growers did not eat cloves; Malabar did not consume its
pepper; Venice — which produced essentially no salt — controlled the salt trade
outright. 30% retained by the producer is far too high for a luxury, and the user's
reading is right.

---

## 2 · Decisions taken

| # | Decision | Note |
|---|---|---|
| D1 | Invert the order: **primary sites → provinces → secondary settlements** | with trade routes kept correct through the change |
| D2 | **Old `.worldforge` files keep their existing provinces** | new order applies only on regeneration; no campaign's `prov_*` state is invalidated |
| D3 | **An explicit UI flag** marks a hand-placed settlement as a province capital | not an implicit "manual ⇒ capital" rule |
| D4 | A capital sits **wherever it actually is** in its province, never recentred | "their position in the province may not be in the center at all" |
| D5 | City capacity re-anchors on the province's real carrying capacity | founding population stops being destiny |
| D6 | **Founding estates and constructing buildings add capacity**, slightly | growth should be earned by what a city builds |
| D7 | Foreign/imported is genuinely more desired; **distance drives both cost and desire** | the amber-in-Rome case |

---

## 3 · Slices

Routing per CLAUDE.md §2.8:
- slices 1-3 touch `sim/step7_settlements/**` + `sim/shared/provinces.rs` →
  `provinces::tests`, the settlement tests, and `goods_` if belts move relative to
  catchments;
- slices 4-7 touch `sim/campaign/tick/**` → `tick::tests` + `econ_` + the dynamics
  run (§2.1, §2.5);
- every frontend change → `npx tsc --noEmit`.

**Depends on `MONEY_MINES_AND_GOODS_PLAN.md` slice 0** (the `tick/mod.rs` constant
split) only in the sense that slices 4-7 add constants to that file. Land the move
first if both plans are in flight.

---

### Slice 1 · Split settlement generation into PRIMARY and SECONDARY passes

Restructure `generate_settlements` into two passes over the same candidate list and
the same habitability field. The candidate scan, the local-maxima test and the
carrying-capacity model are all reused unchanged; only the selection is staged.

**Primary pass** — regional centres:
- wide spacing (`PRIMARY_MIN_DIST`, roughly today's `min_dist` × a factor so
  primaries land at province scale), a higher habitability threshold, and
  crucially a **trade-weighted score** rather than pure habitability. The existing
  `compute_habitability_fields` already computes and returns a separate `trade`
  field (`settlements.rs:44-47`) that plain habitability discards. A regional
  centre is a place goods pass through; a fertile valley is not automatically one.
- **The river-spacing halving does not apply.** That rule exists to string small
  towns along a valley, which is a SECONDARY behaviour; applying it to primaries
  puts three capitals in one valley.

**Secondary pass** — everything else:
- runs **after** provinces exist (slice 2), so it can be told which province each
  candidate falls in;
- keeps today's tighter spacing and the river halving;
- **respects a per-province cap** scaled by the province's own carrying capacity, so
  a rich province fills with towns and a tundra province gets one.

**Gate:** `primary_settlements_are_regional_not_merely_fertile` — assert the
primary set's mean pairwise spacing exceeds the secondary set's by a real margin,
and that primaries score higher on `trade` than a same-sized sample of secondaries.
A pass that merely takes "the top N by habitability" fails the second half, which is
the whole point.

---

### Slice 2 · Provinces from primaries, with a guaranteed seat

Change the pipeline order to:

```
7   settlements (PRIMARY only)
7b  provinces      ← seeded by primaries + fillers, as today
7c  settlements (SECONDARY), placed INSIDE provinces
```

Two changes inside `generate_provinces`:

1. **Seed from primaries only.** The sorted-biggest-first settlement seeding
   (`provinces.rs:1181-1192`) stays exactly as it is — it just receives a list that
   contains only primaries, so a secondary town can never steal a seat from the
   capital two valleys over.
2. **Every filler-seeded province gets a founded seat.** After the flood, any
   province with no settlement has one founded at its **best-scoring cell by the
   same `trade`-weighted primary score** — not at its geometric centre (D4), and
   not at the filler seed cell, which was chosen by a jittered grid and means
   nothing.

Then in slice 1's secondary pass, each town is assigned to the province containing
its cell, and the per-province cap applies.

**Result: exactly one main hub per province, by construction.** No repair pass
needed for newly generated worlds.

**Old worlds (D2):** none of this runs unless settlements/provinces are
regenerated. `get_province_layer` keeps loading the stored partition untouched, and
a running campaign's `prov_*`, `prov_holder`, `prov_realm` state is never
re-indexed. `repair_province_settlements` stays, as the tool for an old world whose
seat is wrong.

**Gates:**
- `every_province_has_exactly_one_seat` — on a freshly generated world, assert
  every province has a non-empty `settlements` list and exactly one settlement
  marked primary.
- `a_province_seat_is_not_its_centroid` — assert seats are distributed off-centre
  (D4), so a future "tidy up the seats" edit that recentres them fails.
- `an_old_world_partition_is_untouched` — load a stored province layer, assert ids
  and raster are byte-identical after the change.

---

### Slice 3 · The capital flag, manual placement, and trade routes

**3a — `Settlement.primary: bool`** (serde-defaulted false, rule 7's discipline).
Written by slice 1's primary pass and by the UI flag.

**3b — The UI flag (D3).** `StepSettlements.tsx`'s placement control gains an
explicit choice: **"province capital"** vs **"town"**. A hand-placed capital is
carried into the primary list and is guaranteed a province; a hand-placed town
joins the secondary pass. The existing `manual || edited` keep-list
(`StepSettlements.tsx:90`) already preserves both across regeneration — only the
flag is new.

This is the workflow requested: place the province centres by hand, press generate,
and provinces form around them with secondaries filling in.

**3c — Trade routes must follow.** Routes are computed from the settlement list
(`compute_trade_routes`), so a changed settlement set changes them. Three things to
check rather than assume, because `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` records
each of them breaking silently before:

- **Every settlement gets a route.** Stage A's fall-through shortlist (a town tries
  its 5 nearest hubs, not just the nearest) must still cover secondaries — they are
  smaller and more numerous than the set it was tuned on.
- **Trade components are built from ROUTED reachability**, not Euclidean distance
  (the fixed B2 bug). More, smaller towns means more component members; verify the
  `TRADE_COMPONENT_HORIZON_KM` behaviour still splits oceans and merges straits.
- **A province seat should be a route node.** A capital founded on a filler province
  (slice 2's guarantee) is new ground — confirm it is routable at all before
  trusting it as a seat.

**Gate:** `every_settlement_is_reachable_by_some_route` on a freshly generated
world, plus the existing `component_tests` fixtures re-run unchanged.

---

### Slice 4 · Capacity from the land (D5), and building adds to it (D6)

The highest-leverage single change in this plan, and the one with the widest blast
radius. **Dose it.**

Replace the `founding_pop` anchor in `disease.rs`'s growth pass with a blend:

```
capacity = lerp(founding_pop, land_capacity, CAPACITY_LAND_WEIGHT) * cap_mult
```

where `land_capacity` derives from the hub's own province: `prov_cap` (already
computed yearly), shared among the hubs in that province by their trade weight, so
a province's capital carries more of its hinterland than its secondary towns — which
is what a province seat *means*.

**`CAPACITY_LAND_WEIGHT` ships at 0.0** — a true no-op, bit-identical, the
N1/N6/S3 pattern this codebase uses for exactly this kind of change — and is
walked up with `econ_` re-run **per dose step**. At 1.0 the founding roll stops
mattering entirely; the right value is almost certainly in between and must be
measured, not chosen.

**A province-less campaign keeps `founding_pop` exactly**, so the dynamics gate
stays bit-identical by construction (the same guarantee
`province_land_pass_is_a_noop_without_provinces` already gives).

**D6 — building adds capacity.** `cap_mult` already carries `trade_dev`,
`primacy_dev`, `health_dev` and `world_age_dev`. Add a bounded
`works_dev` term reading the hub's own estates and structures — an estate founded
in a city's hinterland and a structure raised in it each lift the ceiling slightly.
Keep it **small and capped**: it must read as "a city that builds, grows", not as a
second exponential. A city can already found many estates, and an uncapped term
would turn estate-spam into unbounded population.

**Gates:**
- `capacity_land_weight_is_a_noop_at_zero` (the dose gate)
- `a_well_sited_small_town_can_overtake_a_badly_sited_large_one` — at a live dose,
  two hubs with reversed `founding_pop` and reversed `prov_cap` must cross over.
  This is the claim; without it the slice is just a constant.
- `works_capacity_is_bounded` — assert a hub with many estates does not exceed a
  stated ceiling multiple.

---

### Slice 5 · Province-aware founding (F5)

A small, bounded addition to the founding paths rather than a new system: when
scoring a colonizable site, **prefer a province that currently has no live hub**.

`compute_colonizable_sites` already stamps each candidate's real attributes
(`river`, `kind_hint`, …). Add the site's province id and whether that province is
currently hub-less, and let the existing scorers weight it.

This is deliberately a **preference, not a rule** — the same discipline the endemic
chooser uses (§8.20: *still a preference, never a filter*). A province that is
genuinely uninhabitable must stay empty; a colony must never be forced onto ice
because the partition drew a province there.

**Gate:** `colonisation_prefers_an_empty_province_but_is_not_forced_into_one` —
both directions.

---

### Slice 6 · Demand: local abundance suppresses desire, foreign raises it (D7, F6/F9)

The cheap keystone, and the one that produces the Venice/salt and amber/Rome
patterns without any new entity.

Replace the binary `local < 1e-4` switch with a **continuous** reading of how well
supplied a city is from its own ground:

```
self_supply = local_production / local_need          (0 = imports everything, ≥1 = self-sufficient)
foreign_lux = 1 + tier_gain * prestige * (1 - saturate(self_supply))
local_jade  = 1 - LOCAL_SATIETY * saturate(self_supply)     ← NEW: the producer is jaded
need       *= foreign_lux * local_jade
```

Two distinct effects, and they must be kept distinct:

- **`foreign_lux`** (existing, now continuous) — a city partly supplied from home
  still craves some imports, instead of the craving switching off at the first unit
  produced locally.
- **`LOCAL_SATIETY`** (new) — a city **awash** in a luxury it makes itself wants
  *less* of it per head, which is what frees output for export and is F9's direct
  answer. This is the Moluccan clove grower and the Baltic amber gatherer.

**Three rules this must obey**, all of which this codebase has already paid for:

1. **It applies to COMFORT and LUXURY tiers only.** Nobody is jaded with bread. A
   basic good's need is not a matter of taste.
2. **It must not touch `needs_struct`.** N6's own discipline (CLAUDE.md §5): the
   STRUCTURAL need that `lack_basic` / starvation / civic provisioning read must
   stay untouched, or a council stops provisioning exactly when it matters. Only
   the market-facing `needs` changes.
3. **`LOCAL_SATIETY` ships at 0.0 and is dose-walked** against `econ_` — in
   particular `econ_expenditure_shares_resemble_a_household`, because this directly
   moves the luxury share the S1 rework spent a whole slice calibrating.

**Gates:**
- `a_producing_city_wants_less_of_its_own_luxury` (the claim)
- `local_satiety_is_a_noop_at_zero` (the dose)
- `a_basic_good_is_never_subject_to_satiety` (rule 1)
- `the_structural_ration_is_not_affected` (rule 2 — the same shape as
  `n6_the_ration_is_not_elastic`, which already exists and can be copied)

**Expected reading afterwards:** a luxury producer should retain well under 30% of
its own output. That number goes on the scoreboard as the before/after.

---

### Slice 7 · Provenance and distance — the amber-in-Rome mechanism (D7, F7/F8)

The substantial slice, and the one that needs `GRADE_BANDS`' shape (F7).

**7a — Stamp origin distance on cargo.** `InTransit` already carries `local: bool`
(N8's carrier attribution). Add `origin_km: f32` — the real routed distance the
cargo travelled, which `lane_days` / the route matrix already know. This is one
field on an existing struct, serde-defaulted to 0 so in-flight cargo on an old save
reads as local (the same convention `InTransit.local` itself uses).

**7b — A provenance axis on stock.** The honest way to make imported salt a
different thing on the shelf from local salt. Two options, and the plan recommends
the first:

- **(i) A parallel `stock_origin` accumulator** — for each (hub, good), a decaying
  weighted mean of the `origin_km` of what has arrived. Cheap (one f32 per hub-good,
  no change to the stock layout, no serialization risk), gives a *continuous*
  "how foreign is this market's supply" reading, and cannot break any existing
  `stock_of`/`stock_take` caller. **Recommended.**
- **(ii) Widen `GRADE_BANDS` into a grade × provenance matrix** — truer (a specific
  parcel keeps its origin) but multiplies every stock vector, touches every
  accessor and the warehouse UI, and risks the exact serialization hazard rule 7
  exists for. Not worth it for the effect.

**7c — Distance confers prestige.** With `stock_origin` available:

```
import_prestige = 1 + FOREIGN_PRESTIGE * saturate(stock_origin_km / PRESTIGE_REF_KM)
```

folded into the same luxury/comfort term as slice 6. A far-travelled good is wanted
more **and** (already, via `good_freight`) costs more — which is the requested model
stated precisely: *if it is hard logistically to import a good then it is
expensive*, and the market bears it because the distance is itself the status.

**Scope rules:**
- **Luxury and comfort tiers only.** Distance does not make grain more desirable; it
  only makes it dearer, which is already true and is correct.
- `FOREIGN_PRESTIGE` ships at **0.0** and is dose-walked. It compounds with slice
  6, so walk them **one at a time** — `ACTORS_AND_CARRIAGE_PLAN.md` §5.2's lesson
  (re-run the gate per DOSE STEP, not per phase) applies doubly to two terms in the
  same expression.
- `PRESTIGE_REF_KM` is stated in **kilometres** (rule 25), never in cells.

**Gates:**
- `a_far_travelled_luxury_is_wanted_more_than_a_near_one` (the claim)
- `foreign_prestige_is_a_noop_at_zero` (the dose)
- `distance_prestige_never_touches_a_basic_good`
- `an_old_save_with_no_origin_data_is_bit_identical`

**Deliberately NOT in this slice:** a per-culture demand profile. A vector over
categories per people, resolved once like `culture_rules`, is a real and attractive
idea — and it would move every price in the world at once, on top of two other
demand doses in the same plan. It belongs in its own plan with its own measurement.
Named here so it is not assumed done. See §5.

---

## 4 · Risk register

| Risk | Why it bites | Mitigation |
|---|---|---|
| **Slice 2 re-indexes provinces** | province ids shift, and a campaign's `prov_*`, `prov_holder`, `prov_realm` and realm sovereignty are indexed by them | D2: old worlds keep their partition. The new order runs only on regeneration, and `ensure_unfrozen` already blocks regenerating provinces mid-campaign |
| **Fewer primaries ⇒ fewer, larger provinces** | slice 1's wider primary spacing changes province COUNT and SIZE, which moves `prov_cap`, the tithe, migration and realm eligibility | measure province count and mean area before/after on a fixed seed and put both on the scoreboard; tune `PRIMARY_MIN_DIST` against province size, not against "looks right" |
| **Slice 4 at a live dose moves every population number** | population feeds production, demand, manufacturing labour, unrest and levies — nearly every `econ_` metric | dose from 0, `econ_` per step, and watch total world population specifically: `diag_exodus_population_concentration` records that `dense_world`'s population already collapses for pre-existing reasons, so do not read a collapse there as this slice's fault |
| **Slices 6 and 7 compound** | both multiply the same luxury/comfort need term | walk them one at a time, with the other pinned at zero; never both in one step |
| **`econ_expenditure_shares_resemble_a_household`** | slices 6 and 7 both move the luxury share S1 calibrated | it is the primary gate for both. If food/luxury shares leave their measured band, that is a STOP, not a tuning opportunity |
| **`econ_inheritance_rules_fragment_differently`** | perturbed five times by unrelated work; multi-seed now but still the most fragile gate | check it on the parent commit first. `INSTITUTIONS_BUILD_ORDER.md` records it failing independently of any of this |
| **More towns ⇒ slower routing** | secondary towns multiply the settlement count, and routing is per-source Dijkstra | Stage A's batched `coarse_dijkstra_batch` already dedupes to one search per SOURCE; verify the route-refresh time on a large world before and after, and cap secondaries per province (slice 1) |

---

## 5 · Deliberately NOT built

- **Per-culture demand profiles.** Real, wanted, and out of scope here — it would
  move every price in the world on top of two other demand doses. Its own plan.
- **A provenance × grade stock matrix** (7b option ii). The parallel accumulator
  buys the effect at a fraction of the risk.
- **Founding a settlement as a player verb.** The campaign stays observation-only
  plus the province tax verb. The *worldgen* placement tool is a different thing and
  is what slice 3b extends.
- **Re-siting an existing capital.** Slice 2 guarantees a seat on generation; it does
  not move a seat that already exists. `repair_province_settlements` remains the tool
  for an old world.
- **Making `Pop` the demographic model.** `Society` shares remain the live model;
  `Pop` keeps its three real consumers (militancy, the levy, craftsmen labour).
- **Price-elastic aggregate demand.** `DEMAND_ELASTICITY` is built and shipped at
  zero (N6). Dosing it is separate, unstarted work and must not be folded in here —
  it multiplies the same term slices 6 and 7 touch.
- **Seasonal demand.** N5 gives lanes a season; demand has none. Not opened.

---

## 6 · Build order

```
1  primary/secondary split       worldgen only, no campaign exposure
2  provinces from primaries      + guaranteed seat            ← the structural change
3  capital flag + UI + routes    ← verify routes BEFORE trusting the new seats
   ── measure province count, mean area, settlements/province. Scoreboard. ──
6  local satiety (dose 0 → live) ← the cheap keystone; do this before 4
   ── re-measure retained luxury share; this is the reported symptom ──
4  capacity from the land        (dose 0 → live)              ← widest blast radius
5  province-aware founding       small, bounded
7  provenance + distance prestige (dose 0 → live)             ← last; compounds with 6
```

**Slice 6 is deliberately pulled ahead of slice 4.** It is the cheapest change in
the plan, it answers the most concrete complaint (a luxury producer keeping 30% of
its own output), and it does not depend on anything above it. If a session has
little time, **do slice 6 alone**.

Slices 1-3 are one unit — do not land the primary/secondary split without the
province change, or you get wide-spaced capitals and no secondaries.

---

## 7 · What to write to the scoreboard

- province count and mean area on a fixed seed — before and after slices 1-2
- settlements per province: min / median / max, and the count of hub-less
  provinces — before and after slice 2 (the target for hub-less is **0**)
- share of a luxury producer's own output retained locally — before and after
  slice 6 (the reported ~30%, and where it lands)
- `econ_expenditure_shares_resemble_a_household`'s food and luxury shares — after
  each dose step of slices 6 and 7
- total world population and the largest city's population at year 200 — before
  and after each dose step of slice 4
- grain price/distance gradient (`real_world_price_distance_gradient`) — after
  slice 7, since prestige is the first mechanism that makes distance pay

Never edit an old row.
