# Trade Staging & Trading Posts — implementation plan

> **Status: APPROVED, NOTHING BUILT.** Design settled with the maintainer 2026-08-31.
> UI schematics for every slice exist as a published artifact ("Break of Bulk", 8 plates).
> Read §1 before §5 — the measured findings are why the slices are in this order.

---

## 0. The one-sentence premise

A 7,000 km trade lane is historically ordinary. **A 7,000 km lane with no stops, no
middleman and no city grown along it is not.** Rome bought from Alexandria, who bought
from Aden, who bought from Muziris — and Palmyra, Berenike and Aden became cities out
of the difference. This plan makes the campaign's cargo travel that way.

---

## 1. Measured findings that motivated this

All read from the code at `670d772`, none from review opinion. Line numbers are that
commit's.

### 1.1 There is a distance horizon, and 7,000 km is inside it

`TRADE_MAX_DIST_FRAC = 0.24` (`tick/mod.rs:1535`) is a **cylindrical straight-line cap
in world-width fractions**, applied in `rebuild_routes` (`tick/production.rs:186-197`).
On a 3600-wide world (1 cell ≈ 11.1 km) that is **864 cells ≈ 9,617 km**. A 7,000 km
direct lane is 73 % of the cap and entirely legal.

**Three passes bypass the horizon entirely**, all in the same function:

| Hatch | Line | Bound |
|---|---|---|
| `#6` NO DEAD CITY — 4 nearest partners guaranteed | `production.rs:229-260` | **no distance bound**, same-component only |
| `#6b` market lifeline — 2 links to a regional market | `production.rs:262-305` | `MARKET_REACH_FRAC = 0.5` → **~20,000 km** |
| `#6c` cabotage across components | `production.rs:307-333` | `CABOTAGE_SEA_FRAC = 0.08` ≈ 3,200 km |

All three price the link as `dist * days_per_cell` with **no terrain multiplier and no
pathfinding** — flat ground at 55 km/day. **A rescue lane is therefore cheaper per km
than any real pathfound route on the map**, which makes the implausibly long lane the
*most attractive* one available. This is the leading suspect for the reported 7,000 km
direct routes.

### 1.2 Nothing resembling distance exhaustion exists

- **No legs, no stops.** `InTransit { from, to, eta_tick }` (`production.rs:1362-1375`).
  Cargo leaves A and materialises at B.
- **Cost is strictly linear in days.** `good_freight = rate·days·bulk + perishable·days`
  (`production.rs:475-480`). No fixed outfitting cost, no super-linear term.
- **Risk is flat and distance-independent.** `SEA_LOSS 0.05` / `CARAVAN_LOSS 0.03` /
  `RIVER_LOSS 0.015` (`mod.rs:110-112`) are per-*shipment* probabilities rolled once
  (`production.rs:1110-1127`). A 9,000 km crossing is exactly as safe as a 200 km one.
- **Capacity does not bind.** `SHIP_CAPACITY`/`BOAT_CAPACITY`/`CARAVAN_CAPACITY`
  (`mod.rs:1172-4`) are read **only** by futures-contract delivery. `dispatch`
  decrements one fleet slot per shipment regardless of quantity
  (`production.rs:983-995`).
- **Nothing is consumed en route** — no crew, no animals, no water.

### 1.3 Travel mode is two booleans, and the campaign route matrix is river-blind

```rust
let sea = self.hubs[a].coastal && self.hubs[b].coastal;   // production.rs:975, :1332
```

That is the entire mode model. `coastal` is set at worldgen as
`distance_to_ocean < 0.06` (`query_commands/economy.rs:1162-1167`) — **pure ocean
proximity**, so a river, lake or estuary city is never `coastal` and every one of its
shipments is classed overland. That flag drives the fleet pool charged
(`cap_land = fleet_river + fleet_caravan` — river and caravan share one undifferentiated
pool), the loss roll, and `hub.in_by_sea`/`in_by_land` (`mod.rs:6733`), which is the
tally the settlement panel reports.

Compounding it, the campaign's own route-days matrix is built with **no rivers**:

```rust
// commands/query_commands/mod.rs:707
let cc = cached_coarse_cost(db, &world, fp, grid_w, grid_h, "", false, true, 0.0, -1, 12)?;
//                                                            ↑ rivers_json = ""
```

`is_river` is all false (`mod.rs:354-365`), so the river rungs of the cost table are
**never reached** in the campaign:

| surface | cost |
|---|---|
| coastal sea | 0.5 |
| minor river | 1.4 |
| navigable river | 2.0 |
| open sea | 2.2 |
| plain land | 4.0 + elev·14 (+ up to +26 biome) |

Meanwhile `OverlayManager.renderFlowHighlight` (`OverlayManager.ts:4019`) snaps the
drawn line onto the **worldgen** trade-route graph, which *does* carry rivers. **The
route the user sees and the route the sim priced are computed by two unrelated
systems.**

> Side finding, not part of this plan: at `query_commands/mod.rs:405-406` a **minor**
> river (1.4) is cheaper than a **navigable** trunk (2.0), which inverts the comment
> above it. It cannot affect the campaign today (neither rung fires) but it does affect
> the drawn overlay routes.

### 1.4 Gravity is applied twice on the same axis

`rebuild_neighbors` ranks partners by `days / hub_pull(b)` (`mod.rs:6083`), and
`dispatch` then scores the shortlist by `gap * hub_pull(b)` (`production.rs:955`) and
keeps only **three** destinations per good per tick. `hub_pull` caps at 3.5
(`mod.rs:393-395`), so a metropolis 9,000 km away ranks as if it were 2,750 km away —
twice over.

### 1.5 Import and export mirror each other by construction

`deploy_return_leg` (`production.rs:1279-1383`) has the same vessel buy at B and carry
home to A, logging `log_trade(b → a)`: **every outbound lane spawns a matching inbound
lane**. `days` is symmetric, so partner lists mirror. And `TRADE_RESERVE_MULT = 1.1`
(`mod.rs:99`) means a non-food good is re-exportable once the city holds **1.1 days of
its own need** — arriving cargo is back on the market almost immediately.

### 1.6 A post is structurally incapable of becoming a city

`try_found_house_outpost` (`houses.rs:1130-1133`) creates an **estate**
(`is_estate = true`, `colony_kind = 2`). `hub_class` zeroes estates
(`cities.rs:1871-1875`) and `hub_pull` returns a flat 1.0 for them, so a post can never
reach trade-hub or entrepôt rank until `maybe_graduate_outpost` promotes it. That gate
(`houses.rs:1160-1180`) reads **age ≥ 30 y + population ≥ 0.9 × cap + the owner's
wealth** — and **never once looks at traffic**. A post on the busiest lane in the world
grows no faster than one on a dead one.

Its siting is also purely **resource-driven**: score is
`trade_value + coast bonus + unexploited belt`, scaled by nearness to the founding
house's own network (`houses.rs:1067-1077`). Nothing can site a post **where a route
needs one** — and Cape Town, St Helena, Socotra and Berenike all score near zero on
`trade_value`.

### 1.7 The good news — three mechanisms already exist

- **`house_barred`** (`mod.rs:4640`) is a per-house list of hub indices the house may not
  trade at. A feud reaching TRADEWAR already writes to it (`war.rs:1313`, gated on the
  winner actually *governing* the city); `dispatch` already refuses a trade where either
  endpoint is barred (`production.rs:990`); `pay_to_regain_markets`
  (`production.rs:684-707`) already lets a barred house buy back in, one market a month,
  at `(pop/5000).clamp(2,40)` into the city's civic pool.
- **The promotion ladder exists in full.** `["", "outpost", "colony", "town", "city"]`
  (`colonies.rs:1732`), and `colony_pass` already grants independence at
  `age >= 50 && colony_stage >= 3` (`colonies.rs:1752-1757`), where stage 3 is
  `pop >= 15_000`. Sibling constants `SATELLITE_INDEP_YEARS = 40` /
  `SATELLITE_INDEP_POP = 8_000` (`mod.rs:528-529`).
- **Cargo can already be routed hop-by-hop.** `neighbor_path` (`mod.rs:7115`) walks the
  trade graph weighted by `days`, documented "people move strictly by routes." **The sim
  already moves people in legs. Cargo is the only thing that teleports.**

---

## 2. Decisions taken (and by whom)

| # | Decision | Taken by |
|---|---|---|
| D1 | A staging stop is a **real settlement**, not a bare facility | maintainer |
| D2 | Cargo at a stop **pauses and may be resold**; the carrier keeps ownership | maintainer |
| D3 | The monsoon / sailing-season gate is **out of scope** | maintainer |
| D4 | Posts are founded by **houses**, may be **usurped by cities**, need distinct map marks and their own window | maintainer |
| D5 | **Closure is a weapon.** A post's owner may bar rival houses; the barred must bypass at real risk; rich houses may found their own posts in answer | maintainer |
| D6 | A post **attracts settlers on its traffic** and may become an **independent city** at ~50 years and a real population | maintainer |
| D7 | "Lanes" is a **third Trade sub-tab** beside Market and Flows | maintainer |
| D8 | One entity — a **house trading post** — with **two founding motives** (route / resource), not a parallel "station" system | proposed, accepted |
| D9 | Independence reuses the **shipped** `age >= 50 && colony_stage >= 3` (pop ≥ 15,000); `10,000` is NOT substituted | proposed, accepted in this doc — see §4.2 |

---

## 3. Historical grounding

Kept short and load-bearing; each claim is here because a slice depends on it.

- **The Rome–India trade was a relay, never a lane.** Alexandria → Nile to Coptos →
  desert road to Berenike → Red Sea coasting → Aden → open-ocean monsoon run → Muziris.
  Five modes, four transhipments (*Periplus Maris Erythraei*, 1st c. AD).
- **Stops existed because range is real.** The Coptos–Berenike road carried
  ***hydreumata*** — fortified, garrisoned cisterns roughly a day's march apart — built
  by the state precisely because the desert leg exceeded a caravan's unprovisioned
  range. This is slice 3's range rule, stated as Roman infrastructure.
- **The route was deliberately broken at nodes.** The *Periplus* (§26) records Aden as
  *Eudaimon* — "fortunate" — because cargoes from Egypt and India were exchanged
  **there**, neither fleet venturing further. Rome and India were structurally never
  counterparties. This is slice 4's break of bulk.
- **The nodes became the cities.** Palmyra taxed transit and built a city on it (the
  Palmyrene Tariff, 137 AD). Petra, Aden, Berenike, Barygaza. This is slice 5's
  transit-driven promotion.
- **Refreshment stations became cities.** Cape Town (1652) was a VOC
  ***verversingspost*** — a vegetable garden and scurvy hospital, under explicit orders
  *not* to become a colony. So were St Helena, Mozambique Island, Hormuz, Malacca.
- **Closure was a real and deliberate weapon.** The Portuguese ***cartaz*** (from 1502)
  required every Indian Ocean ship to buy a pass and call at a Portuguese fortress;
  ships without one were seized. The Danish **Sound Dues** held the Baltic entrance for
  428 years. Venice and Genoa fought over Chios, Caffa and the Bosphorus, not over
  cargo. This is slice 6.
- **…and closure was answered by founding a rival post, not by surrender.** Portuguese
  Hormuz was answered by Bandar Abbas. **Leverage lasts exactly as long as no
  alternative site exists** — which is why the Sound Dues, on a strait with no
  alternative, lasted four centuries. Slice 6's escape valve is not a kindness; it is
  the mechanism.

---

## 4. Design, and where each decision is braked

### 4.1 Closure — accepted, with three brakes

The idea is sound and among the strongest in the design. The risk is not historical, it
is mechanical.

- **Brake 1 — a bar needs a cause, never a whim.** Keep the existing discipline exactly:
  a bar is written only where a feud reaches TRADEWAR **and** the barring party actually
  governs the place (`war.rs:1305-1317`), plus the new case of a declared war. A post
  owner who can ban at will *will* ban on every quarrel, and the map becomes a permanent
  snarl within fifty years.
- **Brake 2 — bypass must be survivable.** Bypass is **running the leg unprovisioned**,
  with loss probability scaling on how far past the mode's range the gap is — ~1.3× a
  gamble worth taking, ~2.1× ruinous. Never a flat die roll. Without this a ban simply
  deletes a rival's lane, which would be a stronger weapon than any existing war goal
  (compare `WAR_GOAL_PROVINCE`, which transfers one province).
- **Brake 3 — the ban feeds the victim's site-search.** A barred house's prospecting
  score is raised for sites that would restore the lanes it lost, so leverage expires by
  construction wherever an alternative site exists.

> **THE RISK THAT MATTERS, stated plainly.** Closure is a plausible **wealth-
> concentration feedback loop**: a rich house closes its posts → poorer houses lose
> lanes → grow poorer → cannot afford `OUTPOST_FOUND_COST` (70,000) to answer → the
> rich house closes more. Top-10 % wealth share is a hard-won number in this project
> (0.497 → 0.651, moved into band by Phase 5 — see `SCOREBOARD.md`). **Slice 6 must be
> measured against `econ_` before and after, and is the single most likely change here
> to break a shipped gate.** If it does, the correct response is to strengthen the
> brakes, not to weaken the gate.

### 4.2 Independence — accepted, and mostly already built

The maintainer's "50 years, more than 10,000 people" landed within a hair of the shipped
constant: `colony_pass` already gates independence on `age >= 50 && colony_stage >= 3`,
and stage 3 is `pop >= 15_000`.

**Keep 15,000; do not substitute 10,000.** Retuning a shipped, tuned constant to fit a
new feature is precisely the trap CLAUDE.md §2.4 names — it would move every colony in
every campaign, not only posts, and the gate that would catch the damage is not the one
we would be watching. If posts specifically prove unable to reach 15,000, that is a
finding about post growth, to be fixed in post growth.

Only two links are missing:

1. `OUTPOST_MAX_POP = 800` is a hard cap applied in `disease.rs:551`. Graduation must
   lift it (it already changes `colony_kind`, so the cap simply stops applying).
2. `maybe_graduate_outpost` must read **traffic**, not only age + population + the
   owner's wealth.

### 4.3 Immigration needs a source, and the obvious one is wrong

A post on an empty coast has **no rural pool**: `province_demography_pass` moves
countryside into cities, and there is no countryside at a cape. The honest source is the
traffic itself — merchants and crews settling along the lanes that call there, i.e.
**route-bound migration over `neighbor_path`**, which already exists and already moves
people strictly by routes. This also makes growth *causally* the traffic, which is the
entire Cape Town claim.

### 4.4 Decline must be as real as growth

A post whose lane dies must die with it, through the existing `decline_years` /
`abandoned` path. **St Helena is the control case**: a post whose site carries no belt
must be able to stay a rock forever. Without the downward path the map paves itself with
posts within a century.

---

## 5. Slices, in build order

Each slice is independently shippable and carries its own gate. Per CLAUDE.md §2.8 the
gate is the routing-table row for the paths the slice touches, **not** the whole suite.

### Slice 1 — the Flows panel redesign *(frontend only; no sim risk)*

Goes first deliberately: it costs no simulation risk, needs no new state, and **it is
the instrument slices 2–7 are judged with**.

- A written **role sentence** per city, from the own/transit/consumed mix.
- A **three-segment goods bar** — own produce · passing through · bought for itself.
  Derived, with no new sim state:
  `transit = max(0, out − own_production)`, `own_export = out − transit`,
  `for_us = in − transit`.
- **Mirrored bought-from / sold-to columns** (the port-book layout `CityMarketView`
  already uses), each row carrying **distance · days · mode · price gap**. All four
  already exist: `hub_cell_dist`, `days[a][b]`, the route's leg mix, and `dispatch`'s
  own `gap`, which is currently discarded.
- A **reach histogram** — volume by distance band (0–500 / 500–1,500 / 1,500–4,000 /
  4,000 km+), flagging the far band and naming the lanes in it.
- **Fix the percentage**: `read_trade.rs:731-737` divides by the good's *combined* in+out.
  Share must be within its own direction.

New backend field: `TradeFlowGood.own_production` (yearly, already computed) plus
per-route `km`/`days`/`mode`/`gap` on `TradeRouteFlow`.

**Gate:** `npx tsc --noEmit`; visual check against a real campaign.

### Slice 2 — correctness *(no new mechanism; these are defects)*

1. Feed real rivers into the campaign route matrix (`query_commands/mod.rs:707`).
2. Apply `hub_pull` **once**, not in both `rebuild_neighbors` and `dispatch`.
3. Price the `#6`/`#6b`/`#6c` rescue lanes with the terrain multiplier
   `terrain_route_mult` the main loop already uses, instead of flat 55 km/day.
4. Derive travel mode from the **route's own leg mix**, not `coastal_a && coastal_b`.

**Gate:** `cargo test --lib tick::tests` + `econ_` (§2.1/§2.5). Expect movement — record
it in `SCOREBOARD.md` rather than tuning it away.

### Slice 3 — provisioning & range *(cost model; the biggest lever on a named failure)*

- **Outfitting cost** — a fixed per-voyage charge, so long hauls need scale.
- **Victualling** — a crew/animal subsistence term ∝ days.
- **Range without a friendly port** — a leg longer than the mode's unprovisioned range
  (sea ≈ 30 d, caravan ≈ 20 d, river none) is refused. *The hydreumata rule.*
- **Distance-scaled loss** — `1 − (1 − p)^(days/leg_days)` replacing the flat
  per-shipment roll.

**Gate:** `econ_` — specifically the price/distance gradient, measured at **−0.064** and
named as the project's largest market failure (`TRADE_AND_MARKET_REVIEW.md` F2). This
slice is the most direct attack on it that has been proposed.

### Slice 4 — legs *(the keystone)*

`InTransit` carries a route (`Vec<hub>` + leg index) instead of a from/to pair; cargo
walks `neighbor_path`. At each intermediate hub:

- **provisioning is charged** (the refresher, and where distance finally costs
  non-linearly);
- the stop **earns transit revenue** and `transit_year` throughput;
- the cargo **may be sold there** if the local price beats the destination net of
  remaining freight — *break of bulk*.

**This is what makes stop cities emerge with no new growth code**: `hub_class` is
already computed from `transit_year` (`cities.rs:1855-1898`, top 20 % → trade hub,
top 5 % → entrepôt) and already feeds `hub_pull`. The Palmyra loop is already wired; it
has simply never had transit to feed on.

It is also what turns slice 1's derived transit into a **literal** figure.

**Gate:** `bench_campaign_tick` (per-shipment route vectors are a real memory/perf
question) + `econ_` + `simulate_decades_reports_dynamics`.

### Slice 5 — posts

- **`colony_kind = 4`** — a route post. One entity with the existing resource post; the
  difference is the **founding motive** and therefore the siting rule.
- **Route siting**: scan the busiest lanes for the longest gap without a friendly port;
  site near the midpoint. This is the thing nothing in the codebase can currently do.
- **A post is a real hub** (`is_estate = false`) so it is eligible for `hub_class`,
  `hub_pull`, city tiers and the partner graph. **This is the change with real blast
  radius.**
- **Transit in the promotion gate** (`maybe_graduate_outpost`), so a post on a busy lane
  graduates in a generation and one on a dead lane never does.
- **Growth from traffic** via route-bound migration (§4.3); the 800 cap lifts on
  graduation; independence then rides the shipped `colony_pass` path unchanged (§4.2).
- **Decline** through `decline_years` / `abandoned` when the traffic goes (§4.4).

**Gate:** `econ_` before/after (mandatory — this changes what a post *is*), dynamics,
plus new unit gates: a post on a dead lane never graduates; a post whose lane dies is
abandoned; a post with no belt never grows past its traffic.

### Slice 6 — embargo & bypass *(the risky one — see §4.1)*

- Extend `house_barred` from "may not **trade** here" to "may not **pass** here" — today
  it is only consulted for the endpoints of a shipment (`production.rs:990`).
- **Bypass** = the leg runs unprovisioned, loss scaling on how far past range.
- **Buy-back scales with the post's traffic**, not its population — the leverage is the
  road, not the town. (`pay_to_regain_markets` currently uses `(pop/5000).clamp(2,40)`,
  which on an 800-person post is the floor of 2.)
- A ban **raises the barred house's prospecting score** for sites that would restore the
  lost lanes.
- The closer's own **forgone transit revenue** is computed and surfaced, so the cost of
  the weapon is visible to the player and weighable by the AI.

**Gate:** `econ_` with **top-10 % wealth share and house turnover reported explicitly,
before and after**. Treat a move out of band as a finding to be braked, not a gate to be
relaxed.

### Slice 7 — usurpation, the map, and the Trading Posts window

- **Usurpation rides the province writ** (rule 24: `prov_holder` vs
  `prov_holder_house`). A city or realm taking the writ takes the **transit revenue**;
  every house still calls there. A war goal, a grant or a realm annexation already flips
  it, so this needs almost no new mechanism.
- **Map** (`OverlayManager`, Canvas 2D): posts as **squares**, never settlement dots —
  fill = the owner house's `houseColor` arms colour; vertical ticks = route motive,
  centre dot = resource motive; gold dashed ring = under a city writ; red bar struck
  through = closed to somebody; hollow dashed = declining. Lanes drawn **by leg** in each
  mode's stroke (solid cyan sea · waved green river · dashed amber caravan · dashed red
  unprovisioned bypass), with a gold diamond at a break of bulk and a cyan ring at a
  station call. A **provisioning-range wash** around every callable port — *the holes in
  it are the sites worth founding*, which turns "sites wanted" from a table into a place
  you can point at.
- **Trading Posts window**: roster (owner, motive, writ, rung, transit, trend), traffic,
  who calls here, control + the ban list, trajectory, and "lanes that die without this
  post" — which is simultaneously the founder's siting score, the seizer's motive and
  the abandonment test.
- **Lanes** as the third Trade sub-tab (D7).

**Gate:** `simulate_decades_reports_dynamics`; `npx tsc --noEmit`.

---

## 6. Deliberately NOT built

Named so a later session does not assume they were forgotten.

- **The monsoon / sailing-season gate** (D3). The world already computes reversing
  seasonal winds and gates them (`earth_monsoon_wind_reverses`), so a departure-window
  rule on long sea lanes would be cheap and historically excellent. Deferred, not
  rejected.
- **Full break of bulk with a change of ownership** — the carrier selling at the
  entrepôt and a *different* house carrying onward (the Palmyrene/Nabataean middleman).
  This is the only version in which Rome and India genuinely never meet, and the only one
  where a city gets rich producing nothing. It moves real wealth between houses and needs
  its own `econ_` measurement; D2 explicitly chose pause-and-resell instead.
- **Vessels as things.** `fleet_sea`/`_river`/`_caravan` remain three counters on
  `House` with no identity, location or manifest. See
  `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md`, which is the correct home for that work.
  This plan does not make a vessel real, so lanes report **cargoes**, never vessels.
- **Information decay / price belief.** A house still trades on the true price
  everywhere. That is stage 4 of the vessels plan and remains the most plausible fix for
  the residual price/distance gradient after slice 3.
- **Player agency over posts.** Founding, closing and seizing are all AI decisions.
  Consistent with §5.1's four mutating campaign verbs; re-exposing them is a UI change
  once the mechanism is measured.
- **A post closing itself to *everyone*** (a true blockade of the route). Only per-house
  bars are modelled.
- **The `is_river` / navigable-river cost inversion** at `query_commands/mod.rs:405-406`
  (§1.3). Real, but it belongs to the overlay's route graph, not to this plan.

---

## 7. Risk register

| # | Risk | Slice | Mitigation |
|---|---|---|---|
| R1 | Embargo drives a wealth-concentration spiral and breaks the top-10 % share | 6 | Three brakes (§4.1); mandatory before/after `econ_`; strengthen brakes, never relax the gate |
| R2 | Per-shipment route vectors cost real memory/time on a long campaign | 4 | `bench_campaign_tick` is the gate; shipments are bounded (3 targets × goods × sellers) but this must be measured, not argued |
| R3 | Posts as real hubs move `econ_` numbers via `hub_class`/`hub_pull`/city tiers | 5 | Measure before/after; this is the change with the widest blast radius in the plan |
| R4 | The route-siting motive fires zero times on a real world | 5 | The `maybe_grant_provinces` precedent — it required a bailo at the province's own seat and fired **never**. Ship a `#[ignore]`d diagnostic that counts firings on a real generated world *before* trusting the mechanism |
| R5 | Slice 3's range rule strands cities that currently trade fine | 3 | Range is checked per **leg**, and slice 4's legs are what make a long lane legal at all — hence the ordering. Until slice 4 lands, range must be checked against the whole hop |
| R6 | A ban with no alternative site is permanent | 6 | Accepted and historically correct (the Sound Dues lasted 428 years). Surfaced in the UI rather than prevented |

---

## 8. Open questions

- **Should a post ever close to *everyone*** (a route blockade rather than a per-house
  bar)? Historically the Portuguese *cartaz* came close. Out of scope above; would need
  its own retaliation rules.
- **Does a seized post's new holder inherit the ban list**, or does seizure clear it?
  Inheriting is simpler; clearing is the better story ("the crown reopens the port").
- **What is the actual population distribution of hubs in a mature campaign?** §4.2
  assumes 15,000 is reachable for a well-sited post. Nobody has measured it. A
  `#[ignore]`d diagnostic should answer this before slice 5 relies on it.

---

## 9. Companion artifact

Eight UI schematics — the current panel and its four faults, the redesigned Flows panel,
a lane opened into its legs, the Posts roster, one post's dossier, embargo & bypass, the
map symbology, and the world map — are published as **"Break of Bulk"**, drawn in the
app's own `chronicleTheme.ts` palette at true UI density. Per CLAUDE.md §2.2 no mockup
file is committed to the repository; `main` remains the source of truth.
