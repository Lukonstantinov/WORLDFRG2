# C-Batch Plan — Merchant Logistics, Guilds & Offices

Living-Trade DLC follow-up to the A/B batches. All design decisions below were
made with the user (2026-06-12/13). Build order at the bottom. Each piece must
keep the economy stable (reuse the `food_surplus_prevents_famine_collapse`
regression + add new tests) — the tick sim is sensitive (see
`worldforge2_famine_fix` memory).

## Two motivations, one machinery

Houses and Guilds share the same engine (treasury/wealth, fleet of
ships/boats/caravans, offices, the −5% office discount, round-trip voyages) but
pull in opposite directions:

- **👑 Merchant House — private.** Out for its own enrichment: profit-max
  arbitrage, round trips for double profit, aggressive expansion. Existing entity.
- **🏛️ Merchant Guild — civic/collective.** Merchants banded together to act in
  the **interest of their home settlement** (representatives of the population's
  needs). NEW entity.

## C2 — Full two-leg round-trip trade (`sim/tick.rs::dispatch` + `InTransit`)

A house voyage is a round trip, not one-way:
```
load at A  --[out good]-->  B   sell out good
                            B  --[return good]-->  A   sell return good
```
- Vessel slot occupied the WHOLE loop (out + back). Profit booked on both legs.
- `InTransit` gains `home: i32` (round-trip origin, −1 = plain one-way) and
  `phase: u8` (0 outbound, 1 return). On phase-0 arrival at B: deliver the
  outbound cargo, pick the best **return good** (B surplus → A deficit/profit),
  **buy it at B**, spawn the phase-1 return leg B→A; on phase-1 arrival: deliver
  + book the return-leg profit.
- **Discount buying:** the return-leg purchase is usually at B's market price, but
  with a **~1-in-4 chance of a glut bargain (−25%)** when B's stock far exceeds
  its reserve (windfall profit that voyage).
- Must respect granary/reserve + import caps so it never strips B's food.
- Fallback to the existing one-way path when no profitable return good qualifies.

## C3 — Guild entities + inbound offices

### Guild entities
- One Guild per settlement that reaches **≥ 50,000 population** (small cities have
  none; a guild can appear mid-campaign as a city grows past 50k).
- **Distinct per-settlement names** (new generator in `sim/names.rs`, e.g. a
  mercers'/staplers'/hansa-style combination unique to each city).
- **Treasury = civic subsidy** scaling with the home city's size/prosperity (the
  city funds its guild), independent of trade margin.
- **Behavior — fill home city's needs first:** imports what the city lacks
  (essentials/food prioritised) even at thin margins, exports surplus to pay for
  it. Trades broadly/opportunistically (no fixed specialty) but in service of need.
- **Expands rarely**; modelled like a house otherwise (fleet, carries trade).
- Trade ownership is now **House / Guild / local-merchant**: the new Guild
  entities replace the old anonymous "guild" class; truly-independent small trade
  remains "local merchants" (owner −1).

### Offices (inbound — a settlement HOSTS foreign merchants' offices)
- A foreign **House or Guild** plants an office in a host city. Holder pool comes
  from powerful cities (houses in top hubs; guilds in ≥50k cities).
- **Trigger:** an **existing trade tie** — sustained (decaying) trade volume
  through the host above a threshold — AND wealth ≥ `OFFICE_OPEN_COST`, which is
  **scaled to the host city's importance** (a counting-house in a great hub costs
  more). Cost deducted; holder must keep a buffer (wealth gates expansion).
  - House office trigger: where it trades MOST (profit footprint).
  - Guild office trigger: to **secure a needed supply** — an office in a city that
    reliably supplies a good the home population needs. Rare.
- **Unlimited** offices per holder; host panel shows ALL.
- **Perk:** −5% on goods the holder BUYS in that host city. Stacks with the C2
  glut bargain; total discount capped ~30%. (Feeds the round-trip return-leg buy.)
- **Office = second base:** `house_for`/guild lookup also matches a holder with an
  office at that hub, so it can originate trade/round-trips from the office → real
  expansion.
- **Lifecycle:** persistent, but **closed** if the holder's trade through the host
  falls to ~0 for a long time OR the holder is near-bankrupt. Chronicle/journal
  `branch` (opened) / `office_closed` events.

## C5 — Merchant map layer (campaign overlay)
- Backend command aggregates live `in_transit` by `(owner, A↔B)` → `{family/guild,
  color, a, b, out_good, out_vol, return_good, return_vol, profit, sea}`, busiest N.
- Frontend: "Merchant routes" toggle draws major active routes coloured by the
  owning family/guild (width ∝ volume); click a route → details panel (who, goods
  sold/bought each leg, volumes, profit). Campaign-only.

## C6 — Settlement view: "Foreign offices here" panel
- Per host settlement, list every foreign House/Guild with an office: origin city,
  holder name (in its colour), **% of this settlement's trade throughput** it
  handles (from the hub×holder throughput matrix), and the **goods dealt**
  (icons + names, from its shipments touching this hub). Show all. Derived live.

## FUTURE IDEA (documented, not yet implemented) — Great Plague cycle

A recurring civilisation-scale plague distinct from the ordinary small plague
event. For game balance later:
- **Era begins ~year 25.** From then, each MONTH there is a base **30% chance** a
  great plague erupts somewhere.
- **Escalation:** if no great plague fires for 5 consecutive years, the monthly
  chance rises +10% (yr 30 → 40%, yr 35 → 50%, …) until one fires, then resets.
- **Spread:** an outbreak spreads along ROADS/trade routes to neighbouring hubs
  over time, and may also appear **sporadically** at distant secondary points.
- **Severity:** wipes out **5%–90%** of an affected hub's population.
- **Duration:** persists **6 months to 4 years** in a given settlement (varies per
  hub), with **debuffs while active** — reduced trade, suppressed population growth,
  and lowered production for the duration.
- Should interact with the population/recovery model so the world can rebuild
  between pandemics (and with the famine fixes so it is survivable in aggregate).

## Build order
1. **C2** round-trip trade (+ tests) — foundation, riskiest.
2. **C3** guild entities (50k threshold, names, civic behavior, treasury) + offices
   (trigger, cost, −5% discount, lifecycle) for houses AND guilds (+ tests).
3. **C5** merchant map layer.
4. **C6** settlement foreign-offices panel.
5. *(future)* great-plague cycle.
