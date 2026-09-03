# Yards, Vessels and Depots

**Status: S1-S4, the guild axis, and W2/W3/W4/W5 are BUILT — S1-S3 additive/inert
(nothing downstream reads them yet, per S1's own "output is a hull-ready event
only"), S4/guild-axis/W2/W3/W4 real but DOSE-WALKED at their zero/no-op setting
(`CAPACITY_BIND_DOSE`, `GUILD_CHARTER_RANGE_DAYS = INFINITY`,
`LANDED_CARGO_TO_DEPOT_DOSE`, `WH_RELEASE_DOSE`, `DEPOT_TO_DEPOT_TRANSFER_ENABLED`
all `0`/no-op), W5 a structural stub only (`FONDACO_FORM_CHANCE = 0.0`, never
fires) — per D5's own order and this plan's own §6 caveat that only the last
two slices can move the fidelity numbers. Code: `sim/campaign/tick/yards.rs`.
Naval-stores goods shipped earlier at `d3bf2da`. S0's `econ_measure_carriage_
ceiling` diagnostic ships alongside. W1 ships as a real query-layer fold
(`HubGoodDetail.depot_stock`/`depot_holders`) plus a minimal `CityMarketView`
inline indicator — the fuller "beside the stall" book view is still frontend
follow-up work.**

Where carrying capacity comes from, who stores what, and why houses feel weak. Extends
`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` stage 1 (which named the problem — "a vessel
is not a thing" — and stopped there) with the *supply* side it never had: hulls have to
be built out of something, by someone, somewhere.

Read `ACTORS_AND_CARRIAGE_PLAN.md` §1 first. Its measured finding — 96% of shipments
move on no house's account — is the thing this plan is downstream of.

---

## 0 · The measured findings this plan rests on

**F1 · Cargo moves whether or not anyone carries it.** `dispatch` decides a shipment
from the arbitrage gap alone and then *attaches* a carrier: the seller's house, else the
buyer's, else `owner = -1`. The stock transfer (`surplus -= amount; stock_take(..)`) sits
**outside** the carrier resolution. So a house's fleet, capital and voyage risk govern
**who profits, never what moves**.

**F2 · The ownerless branch is strictly the best carrier in the model.** It needs no
vessel slot, is not clamped by capital, and the loss roll is literally
`let lost = if owner >= 0 { .. } else { false }` — it cannot sink. Measured share: 96.0%
(reference world) / 95.4% (large), over 60 years, `econ_measure_carrier_mix`.

**F3 · Ship PRICE is not the constraint; the BUILD RATE is.** `SHIP_COST` is 7.0
grain-equivalent and `decide_fleets` buys when wealth exceeds ~17.5; a successful house
holds 100,000–300,000. The pass runs on `tick % 30` — **monthly** — and buys **at most
one hull**, and only when every vessel of that kind is already busy. Ceiling: **12 hulls
a year** however rich the house is. Meanwhile `FLEET_DECAY_CHANCE * fleet_total` = 1.2%
per hull per month reaches certainty at **83 hulls**, a hard ceiling nothing can pass.

Measured fleet: **2.4 hulls per live house** (reference, 64 houses / 152 slots), **10.9**
(large, 83 / 905). And the large world carries **5.5× the fleet with a LOWER house share
of trade** — arbitrage opportunities grow with hubs × goods while fleets grow with house
wealth. *Giving houses more money or more ships is a treadmill.*

> **F3 is an inference from reading `decide_fleets`, not yet a measurement.** Slice S0
> exists to make it one before anything is built on it. If it is wrong, this plan is
> wrong.

**F4 · One shipment consumes one vessel slot regardless of quantity.** `SHIP_CAPACITY`
(120) / `BOAT_CAPACITY` (70) / `CARAVAN_CAPACITY` (40) exist and are read **only** by
futures-contract delivery. Spot trade never consults them.

**F5 · Guilds and houses are the same object.** `House { is_guild: bool }`.
`decide_fleets` never checks the flag — a guild buys hulls identically. What *does*
differ: a guild cannot go bankrupt (`!is_guild && wealth < HOUSE_BANKRUPT`), draws a
civic subsidy, has no cadet branches, and has no tier/kin/goals/crisis/succession.
**Nothing makes a guild LOCAL**, which is the one axis that would distinguish a Zunft
from a Fugger.

**F6 · The house depot is a one-way sink.** Goods enter by monthly stocking
(`sync_and_stock_warehouses`). They leave only by futures delivery (`houses.rs`), a war
sack (`war.rs`), or the house dissolving. **There is no ordinary sale out of a depot.**

**F7 · Depot stock is off-market but still priced in.** `hub_stock` sums pool + every
depot at that hub, so prices see stored goods — but `dispatch` reads the raw **pool**
when it looks for a seller's surplus. Goods in a depot are therefore held **off the spot
market while still depressing the price**. That is very nearly merchant speculation with
half the mechanism missing.

**F8 · Landed cargo has no owner.** Every arrival lands in the undifferentiated city
pool. A house sails a cargo across the world and hands it to the city: it cannot hold it
for a better price, re-export it, or be blockaded out of selling it. Every entrepôt
behaviour is downstream of this one missing edge.

---

## 1 · Decisions taken (and why)

**D1 · A hull is built from a MATERIAL POOL, never a recipe.** The yard consumes whatever
suitable construction material reaches the city — grown in its hinterland or landed on its
quay — and the **mix sets what kind of vessel comes out**, never whether one can be built.

A fixed recipe fails in both directions and this is not hypothetical:

* Built on `timber` + `iron`, it binds **nowhere** — both are `GOOD_UNLIMITED`, so every
  coastal city on every world satisfies it and the yard is decoration.
* Built on scarce naval stores, it locks the **tropics and the desert out of seafaring
  permanently** — the inverse of the history. Arabia is desert and was a great maritime
  culture *precisely because* it imported Malabar teak; a teak hull needs no paying at all
  because the wood's own oils do the job.

**D2 · A ship is not a good.** Its *inputs* are goods, drawn from the city's stock like
any other recipe. The hull is owned, never traded, so it is a `Vessel` and not a
`Distribution::Manufactured` good — which also keeps rule 33 clean. A separate
non-tradeable "resource" class was rejected: it would be a second mechanism beside
`RecipeInput`/`apply_manufacturing`, and it would delete the interesting part, which is
that yards **compete for the same timber the market wants**.

**D3 · Fractional ownership, not whole hulls.** One firm buying one hull outright was the
exception. Venice's Arsenal was state-owned and auctioned galleys voyage by voyage (the
*incanto*); private ships were held in fractional shares (*carati*, 24ths). Genoa's
*commenda* split capital from carriage entirely. The Hanse used *Partenreederei*; the
Dutch standardised at 1/64 *paerten*. **This is also the fix for F3**: shares lift the
build ceiling, which is the binding constraint, where price never was.

**D4 · Overland capacity is HIRED, not owned.** A merchant did not own a caravan — camels
were hired from brokers and you bought *space*. So the land equivalent of a yard is a
**stable/caravanserai**, a different building with a different feel, and the honest model
of the 96% residual is *hired carriage that is currently free*.

**D5 · The order is measurement → structure → identity → ownership → economics.** Only
the last two slices can move a number anyone cares about, and they ship at zero dose.

---

## 2 · Part I — the yard, in six slices

### S0 · Measure the ceiling *(no behaviour change)*

`econ_measure_carriage_ceiling`, `#[ignore]`d, beside `econ_measure_carrier_mix`. Reports:

* the **distribution** of fleet size across live houses, not just the mean;
* how many house-months are spent **fully saturated with capital to spare** — the state
  `decide_fleets` can only relieve one hull at a time;
* how much of the ownerless residual was declined purely for want of a slot at a house
  that could have afforded it (`diag_why_slot` already counts this globally; this splits
  it by house wealth).

**Why first:** F3 is currently an inference. Five slices rest on it.

### S1 · The yard

A new estate kind (7). A city or a house builds it; it holds a berth queue and draws down
the material pool (D1). Reuses `create_estate`, the ownership field, the damage/repair
pass and the tier ladder — no new lifecycle. Output is a *hull ready* event only; nothing
consumes it yet.

**Gates**
* `a_world_with_no_yards_is_bit_identical` — the same guarantee
  `province_land_pass_is_a_noop_without_provinces` holds for provinces.
* `a_yard_with_no_material_builds_nothing` — and says so, rather than stalling silently.
* `every_climate_can_build_a_hull` — assert **directly** that a desert-coast and a
  tropical city both reach a buildable mix. This is D1's whole claim and must not be left
  to inference.

### S2 · The vessel becomes a thing

`Vessel { id, name, kind, home_hub, at_hub, capacity, quality, condition }`.

**The migration is the entire risk.** `fleet_sea`/`_river`/`_caravan` are read in a dozen
places — capacity in `dispatch`, war spoils, colony gates, the crisis venture, the ledger,
`decide_fleets` itself. They stay as **derived accessors** over the vessel list rather
than being deleted, so every existing reader is untouched.

**Gate:** `seeding_one_whole_hull_per_counter_is_bit_identical`. If `cap_sea` comes out
the same on a converted save, the representation changed and nothing else did.

### S3 · Shares

A berth is subscribed by several houses; a hull carries `parts: Vec<(house, u8)>` summing
to 64. A house's carrying capacity becomes the sum of its parts.

**This is the slice that answers "houses are too weak."** It lifts the *build ceiling*
(F3), not the price (which was never binding).

**Gates**
* `vessel_parts_always_sum_to_64`, at every mutation — the shape of
  `power_shares_always_sum_to_100`.
* `a_lost_hull_debits_every_part_owner_and_ruins_none` — which is what fractional
  ownership was *for*.

### S4 · Capacity binds — **DOSE-WALKED**

A shipment consumes cargo space proportional to its quantity instead of one slot
regardless of size (F4). `SHIP_CAPACITY` already exists; making it bind for spot trade is
a real change to who can carry what.

Ships behind a dose constant at the no-op setting, proven bit-identical, then walked one
step at a time against `econ_` **and** `simulate_decades_reports_dynamics`.

### S5 · Charter — **DOSE-WALKED**

The ownerless residual becomes hired carriage at a price. This is `N1`
(`ACTORS_AND_CARRIAGE_PLAN.md` §3.1), already wired at `N1_LOCAL_HAUL_BIND_DAYS =
INFINITY`.

**Last, and separable.** Everything above changes *who profits*; this changes *what
moves*, and it is the only lever measured to plausibly fix F2's −0.064 price/distance
gradient — the largest market failure this project has named.

### The guild axis, free

Bound a **guild's** charter/trade reach and leave a **house's** unbounded (F5). One
number, and the two institutions stop being the same object: guilds take dense regional
traffic, houses take the long, risky, profitable hauls.

---

## 3 · Part II — depots

Most of this system already exists. What is missing is narrower than the whole.

| Idea | State |
|---|---|
| Offices grant warehouse space in each city | **built** — a depot per home + per office, monthly |
| A house holds different amounts in different cities | **built** — per-(owner, hub) capacity, expands independently |
| Excess bought locally, held for transport | **built** — draws its specialty goods' local surplus, pays the market |
| Storage has a size, a limit and upkeep | **built** — 5 tiers, Depot → Grand Entrepôt |
| Storage can be damaged or sacked | **built** — `damage`, and war sacks it wholesale |
| A city's own public store | **built** — `civic_goods` + `reserve_food` |
| Cargo a merchant lands is stored by that merchant | **gap** (F8) |
| Sell out of the store when the price is right | **gap** (F6) |
| Offices ship between themselves | **gap** |
| The market view shows the store | **gap** — it is a separate tab |

### Who owned warehouses — three classes, the model has two

* **Civic.** The public granary and salt store — Venice's grain fondaco, Florence's
  Orsanmichele (built as a grain loggia before it was a church). A hedge against dearth
  and a political instrument. → `civic_goods`.
* **Private.** The *magazzino* on the ground floor of the family palazzo. → `Warehouse`.
* **The *fondaco*.** **State-owned, foreigner-occupied, compulsory.** Venice's Fondaco dei
  Tedeschi: every German merchant had to lodge, store and trade *there*, supervised, and
  the Republic took its cut. Same shape as the Islamic *funduq*/*khan* (often a *waqf*
  endowment) and the Hanseatic *Kontor* — the Steelyard in London, Bryggen in Bergen — a
  walled compound held corporately and allotted among members. → **missing.**

The fondaco is what makes an **office** and a **bailo** a *building the host city can
close* rather than a flag. It is also the same mechanism as the guild ordinance
(`ACTORS_AND_CARRIAGE_PLAN.md` N3 and the ordinance sketch) wearing a different face:
a city taxing and supervising trade it does not itself carry.

### Slices

| | Slice | Risk | Gate |
|---|---|---|---|
| **W1** | The market view shows the store beside the stall, by owner | none | `tsc` — query-layer fold, no tick code |
| **W2** | Landed cargo goes to the carrier's depot when there is room, else the pool as today | real | `econ_` — it moves stock OFF the spot market (F7), changing what can be sold, not merely who holds it |
| **W3** | The release verb: sell out of the depot when the price beats the hold | real | `econ_` + dynamics — this is speculation, and speculation concentrates wealth |
| **W4** | Depot → depot transfer on the house's own account | real | **needs S2** |
| **W5** | The fondaco: a city-owned foreign quarter with a cut and a door that closes | high | zero dose first — a rent mechanism, and `N2` broke the hard wealth bound twice |

**The one dependency worth naming: W4 needs S2.** "The office sends a ship to another
city" is meaningless while a vessel has no location — it would be a teleport with extra
steps. Every other warehouse slice is independent of the yard work.

---

## 4 · Already shipped for this plan

`d3bf2da` — **naval stores, and tropical wood as available as temperate.**

* `hardwoods` is now `GOOD_UNLIMITED`, like `timber`. The two are ONE ROLE split across
  climates and had opposite distributions: every suitable boreal/temperate cell grew
  timber, while the entire tropics shared a SINGLE seeded hardwood homeland. Measured
  311 cells / 3 provinces / 11 settlements → **540 / 5 / 17**.
* Two new `Global` goods whose scarcity is **geographic** rather than a seeded homeland:
  `pitch` (boreal pine tar, DFC/DFB, 45–70° — 3,056 cells, 37 settlements) and `hemp`
  (temperate cordage, CFB/DFB/CFA — 1,793 cells, 44 settlements).

They are **materials, not requirements** (D1). Gated by `goods_` (16 pass), per-good
table read rather than just the result (rule 26).

---

## 5 · Deliberately NOT in this plan

* **Naval stores as a strategic embargo.** Denying an enemy pitch is historically real
  (and is what the Baltic trade was about) but it is a war mechanism, and `N2` already
  demonstrated that market closure concentrates rent harder than expected.
* **A ship's crew.** No labour market exists (`FIX_PLAN` Part C); a crew would be a
  fourth abstraction with nothing to draw on.
* **Insurance.** The historically correct partner to fractional ownership, and a real
  answer to voyage risk — but it needs S3 shipped and measured first, or it is insuring
  a risk nobody bears.
* **Convoys / the *muda*.** Venice sailed its galleys in scheduled state convoys. This
  needs S2 *and* N5's seasonal windows to mean anything.
* **A player verb.** Everything here is AI-decided, like the rest of the campaign
  (`FIX_PLAN` B2). `decide_*`/`apply_*` splits are kept so a player-run house can be
  wired later without restructuring.

## 6 · Caveats

* **S4 and S5 are the only slices that can move the fidelity numbers**, and both ship at
  zero dose. Everything before them is representation.
* **`econ_inheritance_rules_fragment_differently` has flipped six times** and is the
  gate most likely to move here — S3 changes how capital is tied up across houses, which
  is exactly what it measures. Re-run it **per dose step**, not per slice
  (`ACTORS_AND_CARRIAGE_PLAN.md` §5.2).
* **F3 is unmeasured** until S0. Say so anywhere it is cited.
