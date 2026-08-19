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

---

# Part 2 — decisions taken, and the design that follows

§1–6 above were written before the maintainer answered §5's questions. This part
records the answers, what they settle, and the concrete design they imply. Still
**not approved for build** — but the open questions are now closed, so this is a
design rather than a menu.

## 7. The answers

| # | Question | Answer | What it settles |
|---|---|---|---|
| 1 | Simulation or view? | *(clarified below)* | Both, **view + instrumentation first** |
| 2 | Who is the player? | **The polis.** A city controls its own market; houses reach it through government (`council_house`/`captor_house`) and through a realm they hold | Tier 3's verb list is the right one; no house-side cargo verbs |
| 3 | Is grain the right yardstick? | Consider gold, or "the right equivalent" | See §8 — keep grain as numeraire, change what is *measured* |
| 4 | Re-baseline the scoreboard? | Maintainer's discretion, optimise for legibility and flow | Free to append worse-but-truer rows |
| 5 | Save budget for price history? | **Yes** to a sparse per-(hub, good) series | 0b is approved in principle |
| 6 | Institutions or microstructure? | *(elaborated below)* | See §9 |
| 7 | Price risk acceptable? | **Yes** — this is an observation game, instability is fine. Add **AI price regulation in a crisis** | T1a is unblocked; a new polis lever, §10 |

### §1 clarified — what "simulation or view" meant

Two bodies of work that share almost no code:

* **The simulation half** — `production.rs`, `mod.rs`'s day loop, `polis.rs`,
  `money.rs`. Elasticity, arrival settlement, margins, institutions. Every change
  here moves the `econ_` numbers, needs its own gate, and can regress things far
  away (this codebase has a documented history of that — see 4.7/4.9/A6 in
  `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md`).
* **The view half** — `HubPanel.tsx`, a new Markets window, `read_hubs.rs`.
  Making state that already exists legible. Cannot regress the sim by construction.

The recommendation is **view + instrumentation first**, for a specific reason and
not out of caution: the mechanism half is currently **undiagnosable from inside the
app**. F1's flat gradient was invisible for the whole life of the project because no
screen and no persisted series could have shown it. Build the instruments, then the
mechanism work has something to aim at.

## 8. The numeraire — keep grain, change what is measured

Two different things were being conflated, mine as much as anyone's:

* **The NUMERAIRE** — the unit of account. `base_value` is quoted in grain
  (`goods_spec.rs:257`: *"wheat = 1.0"*), and every price in the sim is a multiple
  of it.
* **The YARDSTICK** — the good `economy_validation.rs` measures integration on.

**Do not move the numeraire to gold.** Three reasons, in order of weight:

1. **Gold's own value is a variable this project already simulates.** Debasement,
   seigniorage, bullion-limited minting, `coin_trust`, recoinage and reform all
   exist in `money.rs`. Measuring in gold means the ruler moves whenever the mint
   does — you would lose the ability to distinguish "grain got dear" from "the coin
   got bad", which is the single most important distinction in pre-modern price
   history and one this model is otherwise equipped to make.
2. **It is not what the literature does.** Allen's welfare ratios, Persson's and
   Federico's integration series, Clark's real wages — all grain- or basket-based.
   A gold numeraire makes every band in the scorecard uncomparable to its source.
3. It would touch every `base_value` in the goods spec and every price in every
   save. Enormous blast radius, no gain.

**"Or the right equivalent" — the right answer is a BASKET, and half of it is
already built.** `campaign_city_price_index` computes a need-weighted cost-of-living
basket per city (`types/campaign.ts:1030`, "100 = the world standard"). That is the
honest general-purpose yardstick — a bundle of what people actually consume, not one
commodity.

So the change is in the **oracle**, not the model:

* **8a** — report the price/distance gradient **per good**, top and bottom five,
  not for grain alone (F2).
* **8b** — report a second gradient on the **basket index**, which is the closest
  thing to what Chilosi/Persson actually measure across a whole economy.
* **8c** — keep the grain row, but label it as what it is: the numeraire, and the
  one good with a 45-day export reserve and a subsistence top-up. Report it beside
  a "tradeable goods" aggregate rather than as the headline.

All three are test-only, read-only, and cannot move a simulated number.

## 9. Institutions vs microstructure — elaborated

**Institutions** are the legal and organisational forms trade happened *through*.
They are visible, nameable and chronicle-able:

* a **bill of exchange** drawn at Bruges and accepted at Venice, settled at a fair
  so balances net and no specie crosses the Alps;
* the **commenda / colleganza** — the sedentary partner puts up the capital, the
  travelling partner the voyage, and they split the realised proceeds;
* **marine insurance** — a premium priced off a route's own loss rate;
* the **staple right** — goods passing must be landed and offered here first;
* the **fair** — periodic concentration of exchange AND of credit settlement.

**Microstructure** is the mechanics of how a single price forms *inside* a market:
bid/ask spreads, an order book, market makers, limit orders, tick size. That is a
19th–20th century exchange. A medieval market cleared by bilateral haggling, with a
guild or a fair sometimes publishing a reference price. Modelling an order book
would be both an anachronism and far more expensive per tick, for a level of detail
no observation game would ever surface.

**Recommendation: build institutions, not microstructure** — with one exception,
because one microstructure-shaped idea *is* period-correct:

> **One good, several prices, depending on who you are.** A guild brother, a
> foreigner paying a stranger's toll, and a council exercising right of first buy did
> not pay the same. The sim already has fragments — `COUNCIL_BUY_PRICE` vs
> `COUNCIL_RETAIL_PRICE`, the office discount, `MAX_BUY_DISCOUNT`, the coin discount
> — scattered across three files with no shared concept. Formalising them into one
> "who is buying" modifier is the historically correct version of microstructure, and
> it is mostly consolidation of code that already exists.

## 10. Crisis price regulation (answer to §5 Q7)

Real, well documented, and it fits the existing decide/apply split exactly: a new
field on `PolisChoice`, decided in `decide_polis_policy`, applied in
`apply_polis_policy`. Historical precedents: the Roman *annona*, Venice's
*Provveditori alle Biade*, Florence's *Abbondanza*, the English Assize of Bread.

**Trigger** — state that already exists: `lack_basic` above a threshold, or
`food_balance < 0`, or `starving > 0`, with a treasury floor.

**Four levers, escalating. They are NOT equally worth building today:**

| | Lever | Status |
|---|---|---|
| 1 | **Release the civic granary** into the market | ⭐ **Build this first.** `civic_goods` exists and `council_provision_pass` FILLS it — but nothing ever releases it back to relieve a price. This is the missing second half of an already-built mechanism, not a new one |
| 2 | **Suspend food export** (the *tratta* prohibition) | Cheap — dispatch already checks per-hub conditions (`quarantined`), so an `export_locked` flag is the same shape |
| 3 | **Import bounty** — pay merchants a premium to bring grain in | Meaningful today; costs the treasury, which gives treasury a job in a crisis |
| 4 | **Price ceiling / assize** | ⚠️ **Cosmetic until T1c.** A ceiling's entire historical consequence is that it causes shortage and hoarding. With demand inelastic (F5) the shortage is already unconditional, so a ceiling would change nothing but a number on screen. Build after elasticity, with hoarding (stock leaving the open market into private warehouses) as its documented cost |

**Gate:** with the AI supplying the choice and no crisis firing,
`simulate_decades_reports_dynamics` must be **bit-identical**. With crises firing,
`crisis_year_share` (measured 0.000 against a 0.05–0.20 band) must not fall further —
a regulation that eliminates famine entirely has broken the model, not fixed the city.

## 11. The city market view — schematic

### What is wrong with the current Trade tab

It is three columns (arrivals ⇢ Market ⇢ departures), then Transit, then six
sparklines, then Exports/Imports, then a chain ladder — ~230 lines of JSX. It answers
*"what ships are moving"* well and *"what IS this market"* poorly. A single good's
story is spread across four places in three different units, and there is no sense of
the market as a place with a **balance**.

### The redesign: one row per good, and the row is the story

```
  ⚖  RAVENNA — market                        pop 21,400 · basket 118 · ⚠ salt dear
  ────────────────────────────────────────────────────────────────────────────────
  GOOD          SUPPLY                     DEMAND      HELD   PRICE          VERDICT
  🌾 wheat      ████████░ 820 own +40 imp   740 basic   38 d   0.42× ▁▁▂▂▁▁▂  cheap · we export
  🧂 salt       ░░███ 0 own +180 imp        170 basic    9 d   1.90× ▂▃▅▆▇█▇  DEAR · 1 of 3 sources lost
  🫒 olive oil  █████ 300 own               120 comfort 62 d   0.31× ▁▁▁▁▁▁▁  glut · nobody is buying
  🌶 pepper     — none —                     90 luxury    0 d      —  ·······  ABSENT · unreachable
  🐟 stockfish  ██ 60 own +210 imp          260 basic   14 d   0.88× ▃▃▂▂▃▄▃  supplied
```

Six decisions behind that, each with a reason:

1. **Sorted by what is unusual**, not by production: `|price − world_avg| × need_weight`.
   The goods where this city is odd are the goods worth looking at. Sortable, but that
   is the default.
2. **The supply bar is stacked by ORIGIN** — own production / imports / civic stores.
   Currently split across three columns in three units; it is the single most
   informative thing a market can tell you and it is not assembled anywhere.
3. **Stock in DAYS of need, not units.** "38 days of grain" means something; "820
   units" does not. Free — `stock / need` is already computed every tick.
4. **One price unit everywhere** (× world standard), with the world min/max as a
   range bar so you see where this city sits in the world spread. `HubGoodDetail`
   already carries `world_min`/`world_max`/`world_avg` **with the hub names**, and
   today they are used only as a text aside.
5. **A trend sparkline that survives closing the panel** — this needs 0b, and is the
   one part of the redesign with a backend prerequisite.
6. **A verdict phrase, not a raw number.** Same discipline as the house stability
   gauges: pips and a phrase, and a healthy row stays quiet so a warning still means
   something.

### Expanding a row: the price build-up

This is the part that exists nowhere and is the reason to build the view at all —
every number in it is already computed *inside* `dispatch()` and then discarded.

```
  🧂 salt at Ravenna — 1.90× world standard
  ──────────────────────────────────────────────────────────
    cheapest reachable source   Comacchio    0.38×    2 d by sea
    + freight                                +0.09    bulk 2.4 × 2 d
    + import tariff                          +0.11    6% · council Ottaviani (protectionist)
    − reserve-coin discount                  −0.02    Ravenna ducat trusted at source
    ────────────────────────────────────────────────
    = delivered cost                          0.56×
      local price                             1.90×   ← a 3.4× gap nobody is closing
  ──────────────────────────────────────────────────────────
    why: of 32 reachable markets, 2 hold any surplus; both are barred to our houses
    supplied by: Comacchio 61% · Cervia 39%      moves on to: —
```

That last "why" line is the feature. A market view that says *why a gap is not being
closed* is a diagnostic tool for F1 as much as a screen for the player — it would
have made the flat gradient obvious years ago.

## 12. The Markets window — schematic

Answer to *"a campaign subwindow for markets so I can pick the city"*: yes, and it
slots into machinery that already exists. `CampaignTopBar`'s **📦 Economy** group is
where it belongs; `useFloatingWindow` + `@ui/kit`'s `Panel`/`Tabs`/`Chip` are the
shell; `SettlementSearch`'s prefix-match-by-name is the city picker.

```
  ┌─ 📈 Markets ─────────────────────────────────────────────────── ✕ ─┐
  │  [ 🏙 City ] [ 📦 Good ] [ 🌍 World ]                               │
  │  city  ⌕ rav|                    ▸ Ravenna  21,400  Romagna        │
  │                                    Ravello   3,100  Campania       │
  ├────────────────────────────────────────────────────────────────────┤
  │  … the §11 table for the chosen city …                             │
  └────────────────────────────────────────────────────────────────────┘
```

**Three lenses on the same data**, and the second and third do not exist anywhere
today:

* **🏙 City** — §11's table, for a city chosen by typing its name. Its selection is
  seeded from `uiStore.selectedHub` but **not bound to it**, so you can read one
  city's market while the map is somewhere else, and open two windows to compare.
* **📦 Good** — the good-centric screen the app has never had: one good, its price in
  **every** city ranked cheapest-to-dearest, the spread, who produces it, who eats
  it, where it moves, and its price over time. Clicking a row flies the map there and
  lights the belt overlay. Every field exists except the series (0b).

```
    📦 pepper · 45 markets · world avg 3.1×  · spread 0.4× → 9.8× (24×)
    ────────────────────────────────────────────────────────────────
    Calicut     0.4×  ███░░░░░░░  produces 1,240   →  exports to 6
    Alexandria  1.9×  █████░░░░░  entrepôt         →  exports to 11
    Venezia     3.4×  ███████░░░  —                ←  imports 380
    Bruges      9.8×  ██████████  —                ←  imports 40   ⚠ 24× the source
```

* **🌍 World** — the market's own vital signs, and this is the genuinely new idea:
  **put the fidelity oracle on screen.** The price index over time, the spatial
  spread, and the integration scatter (price gap vs travel days, one dot per city
  pair) — literally `economy_validation.rs`'s own plot, live. A flat cloud says *this
  world's markets are not integrated*; a rising one says they are. It serves the
  player as world-character flavour and the maintainer as a live gauge on the number
  that currently has none.

## 13. Recommended build order

Each step's gate is named, and no step's gate is its own target.

| # | Step | Touches | Gate |
|---|---|---|---|
| 1 | **0b · persist a sparse per-(hub, good) yearly price series** (`TradeHist`'s shape) | `tick/mod.rs` | `simulate_decades_reports_dynamics` **bit-identical** — a write-only observability field cannot move the sim |
| 2 | **The Markets window + the §11 city view** | frontend + `read_hubs.rs` | `npx tsc --noEmit`; no sim code touched |
| 3 | **8a/8b/8c · per-good and basket gradients in the oracle** | `economy_validation.rs` | test-only; `econ_scorecard_is_deterministic` unchanged |
| 4 | **Crisis regulation, lever 1 (granary release)** | `polis.rs` | bit-identical when no crisis fires; `crisis_year_share` must not fall to 0 |
| 5 | *re-read F1 with real instruments*, then Tier 1 | — | — |

Steps 1–3 cannot regress the simulation. Step 4 is a genuine mechanism change and is
deliberately the smallest one available (completing a mechanism that is already half
built). Everything with real risk waits until §5 can be answered from evidence rather
than from reading the code.

---

# Part 3 — the city market view 2.0 (same skeleton, better market)

Amends §11. The maintainer's call: **keep the view on its buy/sell arrivals ⇢
market ⇢ departures basis as it is now**, rather than replacing it with §11's
per-good balance table. §11's table is superseded; the price build-up survives as a
row expansion.

The organising idea: the tab already shows *deals*. Make it show a **merchant's
book** — what we bought a good for, what we sold it for, the spread, and what we are
sitting on — which is what a buy/sell view is for and is exactly what is missing.

## The tab

```
┌─ 🏙 RAVENNA · market ───────────────────────────────────────────── 780px ─┐
│  year 1341   turnover 4,820 ▲6%   bought 2,610  sold 2,210  net −400      │
│  ⚠ salt 1.90× and rising — 1 of 3 sources lost                            │
│  [ Market ] [ Flows ]        sort ▾ unusual   filter ▾ all   ⌕ good…      │
├──────────────── ⇢ WE BUY ────────┬─ THE BOOK ─┬──── WE SELL ⇢ ────────────┤
│ ⏳3d ▲Trevisan  Comacchio        │  (below)   │  ⏳2d ▲Ottaviani  Forlì   │
│      🧂 salt ×110  0.58  −1.32   │            │      🌾 wheat ×240  0.44  │
│      ⛵ 64 gr-eq                  │            │      🐫 106 gr-eq         │
│ ⏳6d ▲local     Cervia           │            │  ⏳9d ▲Trevisan   Venezia │
│      🧂 salt ×70   0.66  −1.24   │            │      🫒 oil  ×180  0.33   │
│ ── landed ──                     │            │  ── departed ──           │
│ 12 Mar ▲Bardi   Ancona           │            │  09 Mar ▲local   Rimini   │
│      🐟 stockfish ×210  0.81     │            │      🌾 wheat ×90   0.45  │
└──────────────────────────────────┴────────────┴───────────────────────────┘
```

## The book — the centre column, rebuilt

```
 GOOD          BOUGHT        SOLD          SPREAD  HELD   LOCAL   40 yr
 🧂 salt       180 @ 0.61    —             —        9 d   1.90×  ▁▂▃▅▆▇█  ▲
 🌾 wheat      40 @ 0.39     330 @ 0.44   +0.05    38 d   0.42×  ▁▁▂▂▁▁▂
 🫒 olive oil  —             180 @ 0.33    —       62 d   0.31×  ▁▁▁▁▁▁▁  glut
 🐟 stockfish  210 @ 0.81    60 @ 0.86    +0.05    14 d   0.88×  ▃▃▂▂▃▄▃
 🌶 pepper     —             —             —        0 d      —   ·······  absent
```

Four things that do not exist today:

* **The SPREAD** — mean sell price less mean buy price. It is *the* number in a
  buy/sell view and it appears nowhere in the app. Both sides are already carried
  per deal (`RecentTrade.price`); nothing aggregates them.
* **HELD in days of need**, not units. "38 days of grain" carries meaning; "820
  units" does not. Free — `stock / need` is computed every tick already.
* **A trend that survives closing the panel** — Part 2 step 1, now built
  (`TradeHist.prices`).
* **Sorted by what is UNUSUAL** (`|price − world_avg| × need_weight`) by default,
  so the row worth reading is at the top rather than buried in production order.

## A deal row

```
now:   ● Trevisan   Comacchio   🧂 salt ×110    1.90×   ⛵
2.0:   ⏳3d  ● Trevisan   ← Comacchio
             🧂 salt ×110  @ 0.58  ▼ −1.32 vs local     ⛵ 64 gr-eq
```

Three fixes, one of which is a real defect:

* **ETA.** The most interesting property of an in-flight cargo is when it lands, and
  it is not shown.
* **The real deal price.** `read_hubs.rs`'s `mk_row` stamps every IN-FLIGHT row with
  `hub.price[g]` — *the viewing city's own local price* — rather than the price the
  deal was struck at. `mk_recent` (completed deals) correctly uses `RecentTrade.price`.
  So an arrival currently reads as though it were bought at the destination's price.
  2.0 shows the deal price and its gap to local, which is the buyer's actual position.
* **Direction and value**, so buy/sell reads without consulting the column header.

## Expanding a book row

This absorbs two sections that currently sit apart and say overlapping things.

```
🧂 salt — bought 180 @ 0.61 · sold none · held 9 d · local 1.90×
──────────────────────────────────────────────────────────────────
  BOUGHT FROM   Comacchio  110 @ 0.58   ● Trevisan        2 d ⛵
                Cervia      70 @ 0.66   ● local merchants 3 d 🐫
  SOLD TO       — nothing leaves —
  PRICE  40 yr  ▁▁▂▂▃▃▄▅▅▆▇█▇▆▇█   0.4 → 1.9×    volume ▂▃▃▂▁▁▁
  WHY DEAR      of 32 reachable markets, 2 hold surplus; both barred to our houses
```

The **WHY** line is the one worth fighting for. Every number in it is computed inside
`dispatch()` on every tick and then discarded. A market view that explains why a
price gap is *not* being closed is a diagnostic for F1 as much as a screen for the
player.

## 2.0 REMOVES more than it adds

* the standalone "Prices — local vs world average" grid → its sparklines move into
  the book rows;
* the Exports / Imports two-column block → becomes the row expansion;
* the supply-chain price ladder → same.

Three scattered sections collapse into one book, and the tab gains the spread, ETAs,
true deal prices, days-held and a persistent trend while getting *shorter*.

**Layout caveat.** The tab is 600px today and three columns are already cramped —
part of why it reads as dense. 2.0 wants ~780px. Below that, stack: the book on top,
one merged deal list beneath tagged buy/sell, rather than squeezing three columns.


---

# Part 4 — what was built

Part 2's build order, executed. Every step's gate is recorded with what it actually
measured, including the one that went wrong.

## Step 1 · the price series — BUILT

`TradeHist.prices`, sampled yearly in `fold_trade_year` beside the volume series it
shares a row, cap and pruning with. Sparse by construction. Tail-aligned (rule 28).
Exposed as `HubGoodDetail.price_hist` / `.vol_hist`.

**Gate met exactly.** `simulate_decades_reports_dynamics` bit-identical against a
baseline built from a clean stash of HEAD — all 50 reported years, every figure in
the digest.

## Step 3 · per-good and basket gradients — BUILT, and F2 IS ANSWERED

The oracle now reports the integration gradient per good and on the model's own
need-weighted basket. **The answer is the worse one:**

* **0 of 6 priced goods show any positive distance gradient.**
* **basket gradient −0.006**, basket spatial CV 1.573.

So the flat grain figure was never an artefact of grain's 45-day export reserve.
Distance costs nothing anywhere. **F2 is closed: this is mechanism work, not
instrument work**, and F3/F4/F5/F8 are where it lives.

**A second finding came free.** Dispersion varies enormously BY GOOD while the
gradient does not:

| good | mean \|ln gap\| | as a ratio |
|---|---|---|
| silk | 0.244 | 1.28× |
| olives | 0.484 | 1.62× |
| fish | 0.901 | 2.46× |
| wheat | 0.991 | 2.69× |
| iron | 1.444 | **4.24×** |

Low-bulk goods are near-uniform in space; bulky ones are wild; **neither responds to
distance.** That is exactly the shape F3 predicts — if outbound profit is the FREIGHT
(which scales with `bulk`) rather than the price gap, then bulk sets the level of the
price field and nothing sets its gradient. Caveat: the reference world prices only 6
goods.

## Step 4 · crisis relief — BUILT (two levers of four)

`decide_crisis_relief` / `apply_crisis_relief`, monthly, on the decide/apply split.
Granary release across every food good; the export bar (`food_export_lock`) in
famine, honoured in `dispatch`. The import bounty and the price ceiling are **not**
built — §10's own reasoning for the ceiling stands unchanged.

**A correction to §10 of this document.** It claimed nothing ever released
`civic_goods`. That was wrong: `update_government`'s step 6 already dumped half the
store of the FIRST food good once `starving > 0.5`. What was missing is the policy
layer — a trigger on the dearth rather than on deaths, and coverage of every food
good. The two now compose; the backstop is untouched.

**Measured.** Dynamics: peak house wealth 487,927 → 300,598, sustained richest
487,927 → 248,725 (both toward the project's own bounded ideal), and towns hold at 30
until year 40 instead of losing one by year 15 — a granary keeping cities alive is
what a granary is for. Scorecard barely moves: gradient −0.029 → −0.026, grain
spatial CV 2.582 → 2.542, Gini 0.642 → 0.662, real wage 145.4 → 146.3, crisis-year
share still 0.000.

### The gate that went wrong, and what it cost

`econ_inheritance_rules_fragment_differently` flipped — 190 houses ever under
partible against 196 under primogeniture, on a 3% margin. This gate has flipped
inside noise three times before in this codebase (4.7's branching-order flip, 4.9's
dose-dependent flip, and the whole reason `suppress_realms` exists).

What was checked before reaching for isolation: **the gate's substantive assertion
held wide open throughout** — mean wealth 141,368 partible against 157,415
primogeniture, which the test's own printed note calls "the measure that actually
moves". Only the house COUNT moved.

Relief keeps struggling towns alive, which changes which houses survive and therefore
how many were ever founded. Path-dependent, and orthogonal to the law of inheritance
the gate measures. Isolated with `suppress_relief`, mirroring `suppress_realms`
exactly, and the gate now passes with margin in BOTH directions (193 vs 172 ever;
149,613 vs 161,790 mean wealth). Relief keeps its own instruments: the dynamics run
and the scorecard.

## Step 2 · the market view — BUILT

`CityMarketView` is the Part 3 schematic, shared between the settlement window's
Trade tab and a new floating **⚖ Markets** window with its own city picker
(`campaign_market_cities` — a LIVE list, so towns founded during the campaign are
reachable, which the frozen worldgen snapshot cannot manage).

Delivered as designed: the spread, days-of-need held, the persisted trend,
unusual-first sorting, real deal prices with their gap to the local quote, ETAs, and
the expandable book row. Three sections were removed as promised — the standalone
price grid, Exports/Imports, and the chain ladder — and the map's supply-road
highlight was **rewired rather than orphaned**: opening a good's book traces its road,
the job the old Imports click used to do.

**Two things the schematic promised that the shipped view does NOT do**, rather than
faking them:

* **The full price BUILD-UP** (source + freight + tariff + coin discount = delivered
  cost). Freight needs the per-pair travel days, which the query layer does not send
  and the frontend cannot derive. The row shows what it truly has — the cheapest
  market, its price, the dearest, the average, and the multiple this city pays over
  the cheapest — and stops there.
* **The "WHY it is not being closed" line.** That needs dispatch's internals
  (reachable markets holding surplus, and which are barred). In its place is a
  verdict phrase derived only from state the panel actually holds. Surfacing the real
  reason means exporting it from `dispatch`, which is its own change.

## What is still open

The Markets window's **📦 Good** and **🌍 World** lenses from §12 are not built —
only the City lens, per the maintainer's call. The World lens (the integration
scatter on screen, live) is one command away and is the one that would make F1
visible in the app rather than only in a test.

Everything in Part 1's Tier 1 remains untouched: settle-at-arrival (F4), margin as a
share of the gap (F3), and own-price elasticity (F5). Step 3's result says plainly
that this is where the work now is.
