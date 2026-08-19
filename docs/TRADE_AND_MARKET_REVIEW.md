# Trade & Market — a review, a diagnosis, and a menu

**Status: NOT APPROVED. Nothing here is a commitment.** This is a read of the trade
and market half of the campaign as it actually stands, the measured findings that
came out of reading it, and a menu of what a "level 2" market could be. It follows
§2.4's rules for how work is commissioned here: every proposal below carries a
**gate that is not its own target**, and the findings are written down whether or
not anyone acts on them.

Read `docs/FIX_PLAN.md` Part B2/Part C first — this document does not replace them,
it goes underneath them at the level of the market mechanism itself.

---

## 1. What the market actually is today

Two different markets, and it is worth being precise about which is which:

* **The worldgen market** (`sim/campaign/market.rs`, `compute_economy`) — a static
  equilibrium solver, run once, read by the Economy step. Stocks → needs ladder with
  category substitution → local price `base_value·(need/stock)^0.6` in the grain
  numeraire → arbitrage against freight. Pure, deterministic, unit-tested. It is the
  snapshot the campaign is seeded from and is never consulted again.
* **The campaign market** (`sim/campaign/tick/production.rs` + `mod.rs`'s day loop) —
  the living one. Same price formula, but embedded in a daily loop:
  produce → consume → price → dispatch → arrivals.

Everything below is about the campaign market. The pieces that exist around it —
warehouses and spoilage, futures contracts, tariffs, coinage and coin discount,
banks, monopolies and charters, offtake and share tables, certification, envoys,
trade fairs, right of first buy — are all real and wired. The gaps are not in the
*periphery*; they are in the **core price/exchange mechanism**, which is why they
have gone unnoticed while the periphery grew.

---

## 2. Measured findings

### F1. The market has the *inverse* of a real market's price structure

Measured fresh at `3db1c1d` (`cargo test --lib econ_fidelity_scorecard -- --nocapture`,
60 years, the 30-city reference world), alongside the last recorded scoreboard row:

| Metric | 2026-07-29 | **measured now** | Band | |
|---|---|---|---|---|
| price gap × distance (Pearson r) | −0.01 | **−0.029** | positive, steep (Federico/Persson) | ❌ |
| mean \|ln gap\|, nearest distance quartile | — | **0.901** | | |
| mean \|ln gap\|, furthest distance quartile | — | **0.914** | | |
| grain price CV **across** cities | 2.10 | **2.582** | 0.20 – 0.40 (Chilosi et al.) | ❌ |
| grain price CV **within** a city | — | **0.010** | 0.30 – 0.50 (Persson; Clark) | ❌ |

Those five lines together are a diagnosis, not just a bad report card.

* **Distance does nothing at all.** Two cities a day apart differ in grain price by a
  factor of e^0.901 ≈ **2.46×**; two cities at the far end of the world differ by
  e^0.914 ≈ **2.49×**. The nearest quartile is as badly integrated as the furthest.
  It is not that the gradient is too shallow — there is no gradient.
* **Cross-sectional dispersion is ~7× too high** (CV 2.58 against a 0.20–0.40 band).
* **Time-series variation is ~30–50× too LOW** (CV 0.010 against 0.30–0.50). A city's
  grain price is essentially a constant.

A real pre-modern grain market is the mirror image of this: *moderate* spatial
dispersion that rises steeply with distance, and *large* year-to-year swings driven
by the harvest. This model has enormous spatial dispersion unrelated to distance, and
prices that barely move at all over sixty years.

The two halves have the same root and it is not a tuning problem: a city's price is
set almost entirely by its own production/needs ratio, which is stable, and trade
never moves it enough to matter. F3, F4, F5 and F8 below are four independent
mechanisms all pushing in exactly that direction.

The gradient and the temporal CV have **no gate**. They are the two most central
numbers to "trade and market" in the project and nothing defends either.

> Note the other numbers in the same run have drifted a long way from the 2026-07-29
> scoreboard row (house dissolutions/century 253 vs 10; urban share 0.998; crisis-year
> share 0.000; top-10% share back down to 0.510). Those are outside this review's
> scope, but a scoreboard row that stale is itself a finding — see §2.6.

### F2. The oracle may be measuring the least-traded good in the world

`economy_validation.rs` measures integration on `const GRAIN: usize = 0` — wheat,
the numeraire. Wheat is also the good the tick treats most specially:

* `FOOD_RESERVE_DAYS = 45` — a city must hold **45 days** of need before it exports
  a single unit of food. Every other good reserves `TRADE_RESERVE_MULT = 1.1` days.
* `SUBSISTENCE_FOOD_FRAC = 0.9` (`mod.rs:6287`) tops a remote settlement's own
  cereal production up to 90% of its own need.

So the instrument is pointed at the one good structurally discouraged from moving.
This does **not** excuse F1 — a real grain market did integrate, and Persson's whole
literature is about grain — but it means "fix the gradient" and "fix grain" may be
two different tasks, and nobody currently knows which.

**Cheapest possible next step, and the one I would take first:** report the gradient
**per good**, not only for grain, and print the top and bottom five. That is a
read-only change to `economy_validation.rs`, cannot move a single simulated number,
and converts an unexplained −0.01 into either "the whole market is unintegrated" or
"grain specifically is". Those two findings lead to completely different work.

### F3. On the outbound leg the merchant earns the CARRIAGE, not the arbitrage

`production.rs:899` and `:1104`:

```rust
let delivered = pa + self.good_freight(g, freight_rate, days);
...
let margin = amount * (delivered - pa).max(0.0);   // == amount × freight
let profit = margin * mult;                         // monopoly/specialty/charter
```

The outbound profit is *exactly the freight cost*, times the rent multipliers. The
gap that motivated the voyage — `pb − (pa + freight) − margin·base`, computed a
hundred lines earlier to decide the trade was worth making — accrues to nobody.

The return leg is different and is labelled so in the code itself
(`deploy_return_leg`, `production.rs:1268`: *"True-arbitrage profit (so the source
discount actually pays)"*), where `profit = amount·(pa_sell − pb_buy − freight)·mult`.

The consequence is an inverted incentive on the leg that carries most of the volume:
**the profitable voyage is the LONG one, not the SCARCE one.** Hauling a bulky cheap
good ten days pays better than relieving a famine two days away. This is a plausible
contributor to F1 — if the reward for closing a price gap is independent of the gap,
gaps do not get closed preferentially, and the spatial price field never flattens.

### F4. There is no price risk anywhere in the trade system

Both legs book profit **at dispatch, at departure-time prices**. Arrival
(`mod.rs:6478-6499`) only adds stock to the destination and possibly spawns a return
leg. A voyage can be *lost* (`SEA_LOSS`/`CARAVAN_LOSS`) but it can never *arrive into
a market that moved*.

What this forecloses, all at once:

* **Gluts.** Three houses reading the same shortage all ship, all get paid the
  departure price, and the destination just receives three cargoes. Nobody is ruined
  by good news travelling fast.
* **The cobweb cycle** — the single most characteristic dynamic of pre-modern
  commodity markets.
* **Speculation on storage.** Warehouses exist, are sized, and spoil (§4.2), but
  holding stock can never be a bet, because there is no price to bet against.
* **Information as an asset.** Offices, bailos, envoys and factors all exist; none of
  them can be worth anything for *knowing a price first*, because knowing it first
  changes nothing.

This is the biggest single realism gap in the trade half, and the one whose fix would
unlock the most other mechanics at once.

**F1's temporal CV of 0.010 is this finding, measured.** With every sale settled at a
departure price against a population-driven need, and the price itself smoothed
`0.6·old + 0.4·target` every tick, there is no channel through which a city's price
can move. The model does not have quiet markets because the world is calm; it has
quiet markets because nothing is wired to disturb them.

### F5. Demand has no own-price elasticity

`base_need` (`production.rs:574`) is:

```
population × TIER_WEIGHT × desire × cadence × foreign_lux × society_demand_mult
          × need_scale × DEMAND_PRESSURE
```

Price does not appear. Consumption then eats `min(need, stock)` (`mod.rs:6423`).

Two real price responses *do* exist and should be credited:

* **Cross-price substitution within a category** (`mod.rs:6394`): weights ∝
  `preference / relative_price`, so a dear good inside a category loses share to its
  substitutes. This is a genuine Marshallian substitution effect.
* **An import demand curve on the buyer's side**: `max_stock = need·(base/delivered)^(1/k)`
  caps how much a market will absorb at a given delivered cost.

But **total demand for a category never falls when everything in it gets dearer**.
Nobody eats less bread in a dearth; they simply go short and it is recorded as
`lack_basic`. A price spike therefore has no demand-side relief valve, which makes
famine a pure quantity event and makes any future price-control mechanic (an assize,
a grain magistracy) meaningless — a price ceiling with inelastic demand has no
shortage to cause, because the shortage is already unconditional.

### F6. Credit exists but does not finance trade

`money.rs:766` — loan origination picks a purpose by die roll; `"trade"` (55% of
rolls) means a **five-year term loan of abstract wealth to the richest resident
house**. It is not tied to a cargo, a voyage, a route, or a counterparty. The trade
system's only awareness of credit is one line in dispatch
(`production.rs:928`: `BANK_CREDIT_MULT` widens a banking house's affordability).

Absent entirely: **bills of exchange**, **marine insurance**, **the commenda /
colleganza** (the per-voyage profit-sharing partnership that is the actual reason
Venice worked). The institutions are modelled; the *instruments* are not.

Worth noting because it is cheap: the `Share` table added by
`ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` §4.5 (`holder_kind` / `holder` / `frac` /
`payout`) is already exactly the right shape for a voyage syndicate. It was built for
estates; it would fit a commenda without modification.

### F7. Fairs are cosmetic

`run_trade_fairs` (`houses.rs:2031`) grants sentiment, an overlay-only flow burst on
four lanes, and a chronicle line. No extra clearing, no contracts signed, no
settlement, no price effect. The Champagne-fair mechanism — periodic concentration of
trade *and* the netting of credit balances so specie need not move — is named in the
code and not modelled.

### F8. Dispatch is myopic, and hard-capped in ways that bound integration

One arbitrage round per tick. Per (seller, good): the **3 best** destinations out of
`NEIGHBOR_K = 32` nearest, shipping at most `room × 0.5`, where `room` is measured
against delivered-cost parity. Prices are clamped to `[0.15, 12.0] × base_value`.

The shortlist is ordered by `gap × hub_pull(b)` — the destination's trade gravity.
That is a defensible modelling choice (great entrepôts do pull trade), but it means a
**large rich market outbids a small starving one for the same cargo**, by
construction, at equal gap. If F1's flat gradient turns out to be a dispatch problem
rather than a pricing problem, this is where it lives.

### F9. There is no price history — anywhere

This is the finding I would most want fixed regardless of everything else, because it
is what makes the other eight arguable rather than measurable.

| What exists | Where | Shape |
|---|---|---|
| World price index | `sample_journal`, `mod.rs:7225` | **one scalar**, monthly, 25-year rolling window |
| Per-hub basket index | `HubSample.price_index`, `production.rs:672` | one scalar per hub, monthly, ~30 yr |
| Per-(hub,good) **volume** | `TradeHist.vols`, `mod.rs:2444` | yearly, capped |
| Per-estate output/quality/price | `works_monthly_pass` | 12-month ring, **estates only** |
| Per-(hub,good) **price** | — | **does not exist** |

The Trade tab's price sparklines (`HubPanel.tsx:259`) are accumulated **in the React
component**, into a `useRef` that resets whenever you open a different city, capped at
80 samples, and only filling while the panel is open. Advance fifty years with the
panel closed and there is no price history at all.

So the app cannot answer *"what happened to the price of pepper in Venice"* — the
most basic question anyone asks of a trade game — and neither can a future session
trying to diagnose F1.

**Precedent for the fix already exists.** `TradeHist` is a per-(hub, good) yearly
ring, already serde-defaulted, already capped, already sparse (`TRADE_HIST_CAP`
drops dead trades first). A per-(hub, good) yearly **price** series is the same shape
at the same cost.

---

## 3. The market VIEW

Held separately from the mechanism, because they are different work and it is worth
knowing which one is being asked for.

**What exists and is good:** the settlement Trade tab's arrivals ⇢ market ⇢
departures three-column layout, with per-good made/in/out and a ×-world price; the
Transit (carrying-trade) list; the supply-chain price ladder along a road
(`EconChain` stops with price, toll, markup, demand spike); the Goods window's
quality/grade breakdown; the `goodScarcity` price-premium discs on the map; the
Economy Dashboard's cost-of-living bars; Flows, Futures lanes, Warehouses, the trade
matrix and corridors.

**What is missing, in rough order of how often you would want it:**

1. **A good-centric market screen.** Every view is city-first or house-first. There is
   no "one good, whole world" page: its price in every city ranked, the spread and who
   holds the extremes, who produces it, who eats it, where it moves, and what its
   price has done. Every field for this exists already (`HubGoodDetail` carries
   `world_min`/`world_max`/`world_avg` with the hub names) **except the time series**.
2. **Price over time, at all** (F9). One chart, per good, per city or world.
3. **The Goods window shows no prices.** "Goods of the World" ranks by produced /
   traded / quality — the panel that ought to be the market board has no price, no
   spread, no trend column.
4. **A live price map.** `goodScarcity` reads the frozen worldgen `economy` snapshot;
   during a campaign the map cannot show live prices.
5. **The merchant's arithmetic for a lane.** Buy here, freight, tolls, tariffs at both
   ends, margin, sell there — the numbers dispatch actually computes, shown once. The
   worldgen `EconChain` view has exactly this shape and is not available live.
6. **A shortage board.** `ShortageNote` already carries a reason
   (`no_supplier`/`unreachable`/`deficit`/`no_port`) per hub; there is no world view
   of who is short of what and why.

---

## 4. "Level 2" — the menu

Tiered by cost and by blast radius. **Nothing here is recommended as a block**; the
sequencing note at the end is the actual recommendation.

### Tier 0 — instrument first (no simulated number moves)

| # | Change | Gate |
|---|---|---|
| 0a | Per-good integration gradient in the econ scorecard, not grain alone | read-only; `econ_scorecard_is_deterministic` unchanged |
| 0b | Persist a per-(hub, good) yearly price series, `TradeHist`'s exact shape | `simulate_decades_reports_dynamics` **bit-identical** (a write-only observability field cannot move the sim) |
| 0c | Add "share of consumption that is imported" and a shock half-life to the scorecard | read-only |

0a and 0b together are what make everything below arguable from evidence instead of
from reading. They are also the cheapest items on this page.

### Tier 1 — the core mechanism (each moves every econ number; each needs its own gate)

* **T1a · Settle the sale at arrival, at arrival-time prices** (F4). The single
  highest-value change in this document. Turns storage into a bet, information into an
  asset, and convergent shipping into a glut. It is aimed squarely at the measured
  **temporal CV of 0.010 against a 0.30–0.50 band** — the metric with the largest
  proportional error anywhere in the economy oracle. **Gate:** temporal CV must rise
  by at least an order of magnitude **without** `spatial_cv` (2.582) worsening and
  without the dynamics test's bounded-wealth assertions failing. That second clause
  is the gate-that-isn't-the-target §2.4 requires: it is trivially easy to make prices
  jump around, and doing so while dispersion grows would be a regression dressed as a
  fix.
* **T1b · Make the outbound margin a share of the realised gap** (F3), not the
  freight. Small diff, large incentive change. **Gate:** the price/distance gradient
  (0a) must rise; wealth must stay bounded.
* **T1c · Own-price elasticity in `base_need`** (F5), tier-dependent and hard-capped —
  staples near-inelastic, luxuries elastic, with a floor so a dearth cannot zero
  demand and hide a famine. **Gate:** `crisis_year_share` must stay in its 0.05–0.20
  band; `lack_basic` must not collapse.

These three are related but, unlike Part C's C1–C3, they are **not** one indivisible
block: each is separately testable and separately revertable.

### Tier 2 — institutions (the actual "level 2 market")

Historical, not anachronistic. An order book at a medieval fair would be a mistake;
these would not be.

* **Bills of exchange + fair settlement** (F6, F7). A bill is drawn at one city,
  accepted at another, settled at the fair; balances net so specie need not move.
  Gives fairs a job, gives banks a job connected to trade, and makes distance cost
  *credit* as well as freight.
* **The commenda / voyage syndicate** (F6). Reuse the existing `Share` table. A house
  short of capital sells shares in a voyage; the sedentary partner takes the agreed
  cut of the *realised* proceeds — which only means anything once T1a exists.
* **Marine insurance.** A premium priced off the route's own loss rate (`SEA_LOSS`,
  `CARAVAN_LOSS` already vary by leg) — the natural second product for a bank, and
  the natural counterweight to T1a's new risk.
* **Staple right / entrepôt privilege.** A city may compel goods passing to be landed
  and offered in its market first. This is the actual historical mechanism behind
  Venice, Bruges and Amsterdam; `hub_pull` is currently a cheap statistical stand-in
  for it, and `council_provision_pass`'s right of first buy is half of it already.
* **The assize / grain magistracy.** A price ceiling on bread, with the historically
  correct consequence: shortage, hoarding, and a grey market. **Only meaningful after
  T1c** — with inelastic demand a ceiling has nothing to cause.
* **A published fair price.** Futures exist as bilateral contracts; a fair that
  publishes a settlement price gives the world a forward curve and gives contracts a
  reference that isn't private.

### Tier 3 — local market control (player verbs)

`FIX_PLAN.md` B2 already establishes both the pattern and the gate, and
`decide_polis_policy` / `apply_polis_policy` is **already split**. The trade-relevant
verbs are one command wrapper each:

| Verb | Machinery that already exists |
|---|---|
| `campaign_set_tariff(hub, import, export)` | `PolisChoice.tariff_import/_export`, `apply_polis_policy` |
| `campaign_set_reserve_target(hub, good, days)` | `council_reserve_target`, `council_provision_pass` |
| `campaign_set_embargo(house, hub)` | `house_barred`, already read in dispatch |
| `campaign_grant_charter(house, good)` | `House.charters`, `CHARTER_RENT` |
| `campaign_fund_public_works / health` | `PolisChoice.fund_health` |

The province verbs (`campaign_set_province_tax` and friends) are the shipped template:
validate → call the same routine the AI would → `set_sim` + persist. **Gate, from B2:**
with the AI supplying every choice, `simulate_decades_reports_dynamics` must be
bit-identical to today. That is what proves the refactor was pure.

---

## 5. Open questions for the maintainer

These are the questions whose answers change what the work *is*, not just how it is
done.

1. **Simulation or view?** "Improve trade and market" splits cleanly into a mechanism
   half (§2, §4 tiers 0–2) and a presentation half (§3), and they share almost no
   code. Which one is being asked for — or which first?
2. **Who is the player?** B2 offers observer+ / play-a-house / play-a-polis. Local
   market control means a *tariff and staple right* if the answer is polis, and a
   *cargo, a syndicate and a warehouse* if it is house. The verb list in Tier 3 is
   almost entirely polis-side; the house-side list would be a different page.
3. **Is grain the right yardstick?** (F2.) If the answer is "measure the trade goods
   that actually trade", 0a is the whole task and it is small.
4. **Is a re-baselined scoreboard acceptable?** T1a moves every economy number by
   construction. §2.6 forbids editing old rows, which is exactly right — but someone
   has to be willing to append a row that is *worse* on some metric while being more
   truthful.
5. **What is the save-size budget for history?** (F9.) The journal's 25-year rolling
   window exists precisely because unbounded history caused real lag. A per-(hub, good)
   yearly price ring at 500 hubs × 66 goods × 100 years is ~13M floats if dense — so
   it has to be sparse the way `TradeHist` already is. Is "the goods a hub actually
   trades, yearly, capped at N years" the right cut?
6. **Institutions or microstructure?** (§4 Tier 2.) Bills, insurance, commenda,
   staple rights and fairs are historically real for this period. Bid/ask spreads,
   order books and market makers are not. I would build only the former; is that the
   right call?
7. **Does the campaign want price risk at all?** T1a makes the economy less
   predictable and less controllable. For an observation-first game that is the point;
   if the long-run aim is a playable merchant republic, it is also the main source of
   difficulty. Worth deciding *before* building it, not after.

---

## 6. If I had to pick

**0a + 0b, then re-read F1.** Two read-only-ish changes, one of which cannot move a
simulated number at all and the other of which is gated to be bit-identical. Together
they turn an unexplained −0.029 into a diagnosis, and they give every later item on
this page something to be measured against.

If the appetite is for one *mechanism* change rather than instrumentation, it is
**T1a (settle at arrival)** — it is the one aimed at the largest measured error
(temporal CV 0.010 vs 0.30–0.50), and unlike the others it unlocks four further
mechanics (storage as speculation, information as an asset, insurance, the commenda)
that are all currently unbuildable for want of a price that can move.

Everything in Tier 1 and Tier 2 is more interesting. None of it should be built
first — §2.4's own rule: *never tune a constant without a gate that isn't the target*,
and right now the market's central claim has no gate at all.
