# Merchant vessels, cargo, and the price a house *believes*

**Status: DESIGN, NOT APPROVED.** Six stages, each independently useful, each with
its own gate. Nothing here is committed to.

Companion to `docs/TRADE_AND_MARKET_REVIEW.md`, which measured the problem. That
document ends by saying the work is now in the mechanism (F3/F4/F5/F8, and the
per-good gradient result that closed F2). This document is that mechanism.

---

## 0. The finding this is all built on

**A vessel is not a thing.**

```rust
#[serde(default)] pub fleet_sea: u32,
#[serde(default)] pub fleet_river: u32,
#[serde(default)] pub fleet_caravan: u32,
```

Three counters on `House`. No identity, no location, no cargo. An `InTransit` leg
does not reference a vessel either — `dispatch` simply does `cap_sea[oi] -= 1`, one
slot per shipment **regardless of quantity**. `campaign_merchant_routes` aggregates
in-flight legs into city-pair lanes, so the map draws *trade*, never *ships*.

Three consequences, all of which look like missing UI and are actually missing state:

* "Venezia: 10 boats, 7 ships · Istria: 5 caravans, 2 ships" cannot be answered.
* "Which vessels are in port loading, and whose are they?" cannot be answered.
* A cargo of pepper **and** silk cannot be represented: `InTransit.good` is one index.

And the capacities that *do* exist are fiction for ordinary trade:

```rust
const SHIP_CAPACITY: f32 = 120.0;   // sea
const BOAT_CAPACITY: f32 =  70.0;   // river
const CARAVAN_CAPACITY: f32 = 40.0; // overland
```

They are read **only** by futures-contract delivery (`houses.rs:155-180`), where a
large contract correctly fans out across N vessels each rolling its own loss. Spot
trade ignores them entirely.

Historically the single-commodity voyage is a *modern* bulk-shipping idea. A
Venetian *muda* galley, a Hansa cog and a Saharan caravan all carried assortments.

---

## 1. Stage 1 — the vessel entity

```rust
pub struct Vessel {
    pub id: u32,
    pub owner: i32,          // house/guild index (−1 never: local trade stays abstract)
    pub kind: u8,            // 0 caravan · 1 river boat · 2 ship
    pub at_hub: u32,         // where it sits, or the leg's ORIGIN while in transit
    pub status: u8,          // 0 idle · 1 loading · 2 in transit · 3 unloading
    pub ready_tick: u32,     // when loading/unloading completes
    pub cargo: Vec<(u16, f32)>,   // (good, qty) — the MANIFEST, sparse
    pub leg: i32,            // index into in_transit while sailing, else −1
}
```

`cargo` is the change that matters. Capacity by kind binds: a voyage is N vessels and
the manifest fills them. Bulk (`GoodSpec.bulk`) consumes capacity, so a hold of
timber carries far fewer units than a hold of silk — which is the physical fact the
freight formula currently only approximates as a price.

**Port dwell.** A vessel does not turn around in a tick. `status = loading` for
`LOAD_TICKS_*` by kind, which is what makes "how many vessels are in port" a real,
watchable number, and is historically right (a galley lay in Alexandria for weeks;
the *muda* had fixed loading windows).

**Multi-leg circuits.** Today a house's outbound leg spawns a return leg that buys
the destination's single best surplus good and sails straight home. Real tramping
traders sold and bought at each stop along a coast. A vessel with a manifest and a
dwell state can express a circuit; whether stage 1 builds circuits or only keeps the
existing out-and-back is a scope decision (recommend: out-and-back first).

**Decision — vessel granularity: INDIVIDUAL, lightweight.** Counts would serve the
"10 boats, 7 ships" display, but not "where is each vessel". Individual records are
also what let a *specific* ship sink rather than a counter decrement.

**Decision — who gets vessels: houses and guilds only.** Guilds already receive a
fleet at founding (`found_guild` → `initial_fleet`). Local merchants stay abstract:
they are the city's own short-haul trade, and giving them entities multiplies the
count for nothing anyone would look at.

**Gate.** Two, and the second is the real one:
1. `simulate_decades_reports_dynamics` still passes its bounded/finite wealth and
   turnover assertions.
2. **Long-haul trade volume must not collapse.** Capacity binding is a new constraint
   and the obvious failure is that distant trade simply stops. Measure trade volume
   by distance quartile before and after; the far quartile may fall, it may not
   vanish.

**Expect every `econ_` number to move**, and expect
`econ_inheritance_rules_fragment_differently` to flip — it has flipped on four
consecutive changes now. Diagnose before isolating, exactly as crisis relief did.

---

## 2. Stage 2 — the market view, variant C

Chosen over the by-voyage and by-good variants because cities are the unit a port
book is actually organised by, and because it is the only one that makes the office
and exploration mechanics legible: a city you hold an office in should not read like
a city you touched once.

```
⇢ ARRIVING FROM                │ MARKET                │ SOLD TO ⇢
▾ Comacchio     2d ⛵×2  64    │ ── MADE HERE ──       │ ▾ Venezia   9d ⛵×3 210
    🧂 salt ×110 @0.58         │ 🌾 wheat 820 · city   │    🫒 oil ×180 @0.33
    🐟 fish  ×40  @0.79        │ 🫒 oil   300 · estate │    🧵 cloth ×30 @1.20
▸ Cervia        3d 🐫×1  46    │ 🧵 cloth  90 · manuf. │  ▸ Forlì    2d 🐫×2 106
                               │ ── ON THE MARKET ──   │
── IN PORT, LOADING ─────────  │ good  held price  who │  ── READY TO SAIL ──
⚜ Trevisan  ⛵×2  ▓▓▓░ 68%     │ 🧂salt 9d 1.90× 🏛🏠  │  ⚜ Ottaviani 🐫×2 ⏳1d
🏛 Guild     🐫×1  ▓░░░ 20%    │ 🌾wheat 38d 0.42× 👥  │  🏛 Guild    ⛵×1 ⏳2d
```

A city block collapses; its cargo lines nest under it; a mixed manifest is one block
with one ETA and several goods. The two new strips are the vessels in port.

`supplied by` already exists — `supply_accum` is a flat `good × 5 classes` array
(city / house / guild / local / foreign), served by `read_trade.rs`. **`taken by`
does not**: arriving cargo lands in one undifferentiated pool and is drawn down by
three separate passes (`council_provision_pass`, `sync_and_stock_warehouses`, and
daily consumption). Per-arrival buyer attribution needs new state and is deliberately
left out of this stage.

**Gate:** `npx tsc --noEmit`; no simulation code touched.

---

## 3. Stage 3 — the house fleet & ventures window

```
⚜ Trevisan — fleet & ventures            22 vessels · 6 at sea
──────────────────────────────────────────────────────────────
IN PORT                          AT SEA / ON THE ROAD
Venezia (home)                   → Alexandria ⛵×3 ⏳12d  pepper 240 · silk 40
  ⛵ 7 ships    2 idle · 4 loading    believed 4.2×  spread ±38%  ⚠ never traded
  🛶 10 boats   9 idle              → Ragusa    🐫×2 ⏳3d   wool 90
Istria (office)                       believed 1.8×  spread ±8%   office
  🐫 5 caravans  all idle           ← Corfu     ⛵×1 ⏳5d   oil 180 (homeward)
  ⛵ 2 ships     1 loading
```

Plus a map layer drawing each vessel at its interpolated position, tinted by owner.
The `believed / spread` line per venture is stage 4 surfacing here; without stage 4
the window still works, it just shows no spread.

---

## 4. Stage 4 — what a house BELIEVES a price to be

The core idea, and the one with the strongest historical claim. Persson and Federico
attribute pre-modern market integration to **information** — postal networks, price
currents, resident factors — as much as to transport cost. Information decayed with
distance. Our measured price/distance gradient is −0.026 (`per_good`: 0 of 6 goods
positive), and this is the most plausible mechanism we are missing.

### 4.1 The state

One row per (house, hub, good) is 60 × 500 × 66 ≈ 2M rows — far too many. Instead,
mirror the sparse assoc-vec idiom `House.trade_at` and `House.influence` already use:

```rust
pub struct MarketKnowledge {
    pub hub: u32,
    pub seen_tick: u32,              // when this house last had real news
    pub prices: Vec<(u16, f32)>,     // only goods actually observed, sparse
}
// on House:
#[serde(default)] pub knows: Vec<MarketKnowledge>,
```

Sparse in both directions: a house holds rows only for cities it has touched, and
prices only for goods it has actually seen. A house that has never left home carries
an empty vec and is bit-identical to today.

### 4.2 How belief is formed

| event | effect |
|---|---|
| a survey agent arrives (stage 5) | writes true prices for observed goods, `seen_tick = now` |
| a cargo of the house's arrives/sells | writes the price of the goods actually traded |
| an **office** at the hub | refreshed continuously — a resident factor writes home |
| a **bailo / council / captor** seat | exact, no error at all |
| nothing | `seen_tick` ages and confidence decays |

### 4.3 Confidence, and the spread

```
conf   = exp(-(now - seen_tick) / KNOWLEDGE_HALFLIFE)      // 0 with no row at all
spread = SPREAD_MAX * (1 - conf), floored by presence tier
```

| presence | spread floor |
|---|---|
| never been | — (spread is `SPREAD_MAX`) |
| surveyed / traded once, ageing | decays toward `SPREAD_MAX` |
| office | 0.08 |
| bailo · council · captor | 0.00 |

### 4.4 The believed price

Not merely a stale price — the house *guesses*, and the guess is wrong in a
particular direction for a particular while:

```
anchor   = last-seen price, or the good's world base value (the "rumour" prior)
believed = anchor * (1 + spread * (hash01(seed, house, hub, good, MONTH) * 2 - 1))
```

Two properties that matter:

* **Deterministic** — `hash01`, no global RNG (the project's standing rule).
* **Stable within a decision window.** The hash is bucketed by MONTH, not by tick.
  A belief that re-rolls every tick makes a house flip-flop; a belief that holds for
  a month is both stable and historically right — you receive a letter and act on it
  for a while.

### 4.5 Where it enters the tick

In `dispatch`, the destination price:

```rust
let pb = self.live_price(stock_of(&self.hubs[b].stock, g), needs[b][g], base);
```
becomes the carrying house's *believed* price. The ORIGIN price is always true —
you are standing in that market.

**Belief applies only to legs a HOUSE carries beyond its knowledge.** Local merchants
and a city's own guild trade near and know near, so they keep the true price. This is
both a scoping decision that keeps the blast radius small and the historically
correct one.

A wrinkle to handle carefully: the carrier is currently chosen *after* the target
shortlist is built (`house_for(a,g)`, then `house_for(b,g)` as fallback). The
shortlist should be built from the SELLER-side house's belief, since the existing
code's own comment says the exporter organises the sale.

### 4.6 Risk appetite — an existing knob

```
required_margin = margin * base * (1 + RISK_AVERSION * spread / bold)
```

where `bold = head_character_factor(hi, 0)` — the boldness axis, already built,
already capped at ±15%, already a true no-op for a house with no kin roster. A bold
head ventures into a wide spread; a cautious one demands the margin exceed it.

### 4.7 The outcome, and the chronicle

The voyage arrives and the truth is discovered: profit, break-even, or loss. **The
visit updates the belief** — which is the exploration loop closing, and it works
*even before* settle-at-arrival (`TRADE_AND_MARKET_REVIEW.md` T1a), because the
arrival can write the observed price into `knows` regardless of when money moved.

When belief and truth diverge badly on a losing venture, that is a chronicle beat
worth writing:

> *Trevisan's factor had reported pepper at 4.2× in Ragusa; the galley found it at
> 1.1×, and the voyage was a loss.*

### 4.8 Privilege where a house controls a city

**Decision: privilege changes that house's TERMS, never the city's actual price.**
Extend the ladder that already exists — `OFFICE_BUY_DISCOUNT` (−5%),
`BAILO_CONCESSION_TOLL`, and the council's own `COUNCIL_BUY_PRICE` right of first
refusal — rather than bending `live_price`. That keeps the market price honest and
keeps the mechanic contained.

The alternative — a controlling house actually setting the local price on its
chartered goods — is what a real staple right did, and is a much larger change. Left
as an explicit open option, not folded in silently.

### 4.9 The gate — and the failure mode it must catch

**The claim:** a house facing a wide spread to a distant city demands a larger
expected margin, so distant trades happen only when the gap is genuinely large, so
the observed price gap should RISE with distance.

**The gate:** the price/distance gradient (`integration_gradient`, plus the per-good
table and the basket gradient added in `d4d3a8a`) must rise measurably, **without
`spatial_cv` worsening.**

**The companion gate, which is the one that isn't the target:** long-haul trade
VOLUME must not collapse. If the spread is set too wide, no distant trade ever
happens, cities starve, and the gradient goes flat again from the other side — a
"success" on the headline metric that is really the market shutting down. Measure
volume by distance quartile.

`SPREAD_MAX` is therefore **tuned against the oracle, not by feel** — it is the one
constant here for which we have a real instrument.

---

## 5. Stage 5 — agents, and their log

`envoys.rs` already exists and is most of this: dispatch, real travel time, arrival
resolution, a nearer rival able to close first, `LAW_FOREIGN_BAR` blocking a house
with no local presence, payment clearing through a bank whose branches span both
cities. It is currently single-purpose (acquire a distressed estate abroad) and
deliberately rare — `ENVOY_MIN_WEALTH = 40_000`, at most 2 in flight world-wide.

Generalise rather than rebuild: `Envoy` gains a **purpose** — `Acquire` (existing) or
`Survey` (new). A survey agent is cheap, frequent, sent by houses **and guilds**,
costs its trip, and on arrival writes into `knows`. The log you want — who was sent,
where, by what route, what they found — is a natural record on the envoy plus a
capped `agent_log`.

**The warning from that file's own postmortem:** envoy dispatch rate is
*dose-dependent* on `econ_inheritance_rules_fragment_differently` — cutting its
trigger ~3× cut that gate's margin ~3×. Survey agents must be tuned against that
gate, or isolated with a `suppress_*` flag the way crisis relief now is.

---

## 6. Stage 6 — corridors must pay, and the loop closes

`RouteProspect { a, b, attempts, successes, cum_profit, established }` and
`establish_corridor` already exist. A corridor is established after repeated proven
success and even founds ports and caravanserais along the route.

**It feeds only the map overlay and those villages.** It does not add the destination
to the house's trade set, does not cheapen the lane, and does not open an office. So
establishing a corridor currently means nothing to the trade system.

Three wires close it:

1. An established corridor **reduces effective distance or freight** on that lane —
   known ports, waystations, a known passage.
2. Ordinary houses **prospect**: a house with spare fleet and cash occasionally sends
   a speculative voyage OUTSIDE its usual partner set. Success feeds
   `route_prospects` exactly as an expedition's does.
3. A proven corridor **justifies an office**, inverting today's causality. Right now
   `update_guilds_and_offices` opens an office where a house *already trades most*
   (`trade_at` volume above a floor and ≥30% of its best partner) — the office is a
   reward for existing trade, never a cause of new trade.

Note `hub_pull` already biases dispatch toward large distant markets over small near
ones — a crude version of "pass by the small ones to reach the good one". The two
compose rather than conflict.

---

## 7. Order, and what each stage costs

| # | Stage | Moves the numbers? | Gate |
|---|---|---|---|
| 1 | Vessel entity · capacity · manifests · dwell | **yes, everything** | dynamics passes; long-haul volume does not collapse |
| 2 | Market view C | no | `tsc` clean, no sim code |
| 3 | House fleet window + vessels on the map | no | `tsc` clean |
| 4 | Belief & spread | **yes, everything** | gradient rises, `spatial_cv` does not worsen, long-haul volume holds |
| 5 | Survey agents + log | some | tuned against the inheritance gate, or isolated |
| 6 | Corridors pay · prospecting · offices | some | corridor count and office count both rise; volume holds |

**Stages 1 and 4 are done separately, never together.** Each will move every `econ_`
number and each needs its own diagnosis. This codebase has punished combined changes
four times on record (4.7, 4.9, the realm work, and crisis relief).

---

## 8. Deliberately not built

* **In-transit spoilage.** Raised, and it was a misreading of "rates" as "rats".
  `perishable` stays a freight-price term. (Static spoilage in a city already exists:
  `SPOIL_PER_PERISHABLE`, granary and warehouse multipliers, `SPOIL_OVERFLOW_MULT`.)
* **Per-arrival buyer attribution** ("who bought THIS cargo"). Needs new state; the
  existing seller-class attribution plus the council/warehouse purchase totals cover
  most of what the view needs.
* **A controlling house setting the actual local price.** §4.8 — real staple-right
  behaviour, much larger change, left as an open option.
* **Named vessels.** Lightweight records; a ship is a hull and a hold, not a
  character.
* **Multi-leg trading circuits** in stage 1 (out-and-back first).
