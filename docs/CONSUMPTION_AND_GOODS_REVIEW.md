# Consumption, goods quantity and the warehouse glut — a measured review

**Status: ANALYSIS ONLY. Nothing here is implemented.** One test-only diagnostic
was added (`econ_measure_goods_stock_and_price`) because the central claims are
worth nothing unless measured. No production code was changed.

This answers a set of questions put to it directly: are any goods' quantities
right; why do heavy luxuries like gemstones move in bulk; why are warehouses
full; why are fewer manufactories built; what can the market view honestly show
about buyers; and how do the strata react. It ends with the questions I cannot
answer without a decision from you.

---

## 0 · How to reproduce the numbers

```bash
cd src-tauri
cargo test --lib econ_measure_goods_stock_and_price -- --ignored --nocapture
```

Runs the `economy_validation.rs` reference world (30 cities, 6 goods, 10 houses)
and prints, per good and per year: total stock, structural need per day, **days
of need held**, mean price ÷ `base_value`, and production ÷ need. Every figure in
§3 and §4 below was re-measured at `44f5289` (current `main`); an earlier run on
a base 44 commits older gave the same picture with slightly milder numbers.

The demand-share table in §2 is arithmetic over the shipped constants
(`biological.rs`'s 45-good tables × `tick/mod.rs`'s `TIER_WEIGHT`,
`LUX_IMPORT_DESIRE`, `COMFORT_IMPORT_FRAC`) and can be recomputed by hand.

---

## 1 · What "a city consumes" actually means today

The whole consumer side of the economy is these four lines
(`tick/mod.rs`, the day loop):

```rust
for g in 0..ng {
    let need = needs_struct[h][g];
    let eat  = need.min(stock_of(&self.hubs[h].stock, g));
    stock_take(&mut self.hubs[h].stock, g, eat);
}
```

Read plainly, that says:

- **Consumption is a physical sink, not a transaction.** Nobody pays for what
  they eat. No wealth moves, no `treasury`, no `civic_pool`, no house ledger. The
  population is not an economic agent; it is a hole that goods fall into.
- **Demand is a hard ration, never a budget.** `need` is computed from
  population × tier weight × desire, and consumption is exactly `min(need,
  stock)`. A city cannot want more of a cheap good, cannot be priced out of a
  dear one, and cannot substitute *quantity* (it can substitute *between* goods
  in a category — that part is real — but the category aggregate is fixed).
- **Nothing is ever left over on purpose.** There is no saving, no hoarding, no
  investment by households, no seed corn.

This is the single most important fact in the review, and every other finding is
downstream of it. **There are no buyers in this economy** — which is why the
market view cannot show you any. More on that in §6.

The only money in the model moves between *merchants*, *cities* and *crowns*:
arbitrage margins, tariffs, taxes, wages, seigniorage, bank interest. The
household economy — historically 70–85% of all economic activity — is entirely
outside it.

---

## 2 · Is any good's quantity right? — the demand table, scored

`base_need` (`production.rs:667`) is:

```
need = pop × TIER_WEIGHT[tier] × desire × cadence × foreign_lux
       × society_mult × need_scale × DEMAND_PRESSURE
```

with `TIER_WEIGHT = [1.0, 0.45, 0.22]` and `foreign_lux = 1 + 0.7 ×
clamp(base_value/15, 0.4, 1.6)` for a luxury the city cannot make itself.

Every good's quantity is therefore decided by **two numbers in one table**:
`GOOD_NEED_TIER` and `GOOD_DESIRE`. Nothing else. `GOOD_DESIRE` spans 0.25–0.85
— a 3.4× range — and `TIER_WEIGHT` spans 4.5×. So the widest possible spread in
per-capita demand across all 45 goods is about **15×**, and the `foreign_lux`
term *narrows* it further by boosting exactly the dearest goods.

Computed over the shipped 45-good tables, for a city that imports its luxuries
(the normal case), here is what a head of population demands per day and what
share of its expenditure that is:

| good | tier | base | qty (rel.) | qty % | spend % |
|---|---|---|---|---|---|
| **gold** | 2 | 50.0 | 0.280 | 2.1% | **16.1%** |
| **gemstones** | 2 | 60.0 | 0.187 | 1.4% | **12.9%** |
| cloves | 2 | 25.0 | 0.257 | 1.9% | 7.4% |
| horses | 1 | 12.0 | 0.368 | 2.8% | 5.1% |
| pearls | 2 | 30.0 | 0.140 | 1.1% | 4.8% |
| silk | 2 | 20.0 | 0.149 | 1.1% | 3.4% |
| ivory | 2 | 20.0 | 0.149 | 1.1% | 3.4% |
| … | | | | | |
| salt | 0 | 2.5 | 0.750 | 5.7% | 2.2% |
| stockfish | 0 | 2.5 | 0.700 | 5.3% | 2.0% |
| rice | 0 | 1.1 | 0.800 | 6.0% | 1.0% |
| **wheat** | 0 | 1.0 | 0.850 | 6.4% | **1.0%** |
| barley | 0 | 0.9 | 0.700 | 5.3% | 0.7% |
| millet | 0 | 0.8 | 0.500 | 3.8% | 0.5% |

**The three headline numbers:**

| | model | history |
|---|---|---|
| food & drink as a share of consumption spend | **12.4%** | 60–80% (Allen's respectability basket; bread alone ≈40%) |
| luxury-tier goods as a share of spend | **69.7%** | low single digits, and concentrated in <5% of the population |
| gemstones : wheat, by spend | **13.2 : 1** | perhaps 1 : 300 for a whole city, and zero for most households |
| gemstones : wheat, by **quantity** | **0.22 : 1** | ~1 : 10⁶–10⁸ by mass |

So: **a modelled city buys four and a half kilos of grain for every kilo of
gemstones, and spends thirteen times more on gems than on bread.** Gold alone
outweighs every cereal, fish, salt and oil in the model put together.

**Direct answer to "is any of the goods correctly done in terms of amount?"** —
No, and it is worth being precise about *why* rather than which. There is
exactly **one** calibration constant in the whole demand system:

```rust
let need_scale = total_prod / (total_pop * sum_tw_desire);   // lifecycle.rs:708
```

It balances *total* production against *total* need across all 45 goods at once.
It says nothing about any individual good. A good's level is therefore the
accident of a ratio between two tables that were never compared: how much
geography grows of it (`biological.rs` belts → `base_per_capita`) and its two
entries in the demand table. **At most the average is right; every single good is
right only by coincidence, and the model contains no mechanism that would ever
notice or correct a mismatch.**

The measured spread proves it. In the reference world, `production ÷ need`
settles at roughly:

- **1.0–1.7×** for the staples (wheat, fish, olives) — mild permanent surplus
- **1.9–2.4×** for iron
- **4.7–6.8×** for silk and wine — a permanent five-to-sevenfold overproduction

and none of those ratios ever moves toward 1.0, because nothing in the model is
trying to make it.

**Why gems specifically come out in bulk.** Two independent causes compound:

1. **Production is per-capita for `Deposits` goods too.**
   `production[g] = base_per_capita[g] × population × season × events × tech`
   (`tick/mod.rs:6875`). `base_per_capita` for gemstones is set once at campaign
   start from the static economy's output ÷ founding population, and then a city
   "mines" gems *in proportion to how many people live there*. There is no ore
   body, no `extent`, no `depth`, no working. §8.16 built a real ore geology with
   grade/extent/depth per working, and the campaign reads only `grade` (as a
   quality tier). A metropolis of 200,000 mines gems twenty times faster than the
   town of 10,000 next door standing on the same lode.
2. **The demand table pulls them hard.** `foreign_lux` gives gemstones the
   maximum 1.6 prestige multiplier precisely *because* `base_value` is 60, so the
   dearest goods get the strongest import craving — a positive feedback from price
   to demand, which is the wrong sign.

---

## 3 · Why the warehouses are full — measured

Reference world, `econ_measure_goods_stock_and_price`:

```
  yr     good tier          stock       need/day    days_held  price/base  prod/need
   1    wheat    0        3033355         9667.8        313.8       0.212       1.50
   1     silk    2        1050379          700.5       1499.4       0.176       5.58
   5    wheat    0       11564008        23687.6        488.2       0.194       1.12
   5     silk    2       10138943         1955.1       5185.9       0.168       7.31
  10    wheat    0       31869040        53709.5        593.4       0.337       0.92
  10     silk    2       37031239         4719.2       7846.9       0.168       7.99
  25    wheat    0      284821876       206121.4       1381.8       0.517       1.17
  25     silk    2      250183266        18387.7      13606.0       0.168       5.07
  50    wheat    0     1286956402       308591.3       4170.4       0.445       1.24
  50     silk    2      949063676        27943.3      33963.9       0.163       4.62
 100    wheat    0     4045006625       376044.6      10756.7       0.312       1.45
 100     silk    2     2389137796        33157.5      72054.2       0.138       4.73
```

**By year 25 the world holds 3.8 years of grain and 37 years of silk; by year
100, 29.5 years of grain and 197 years of silk** — and it was already holding 10
months of grain and 4 years of silk in year *one*. This is not a slow drift: it
is the founding condition, and it compounds without limit. Note that this happens
even though `tech_factor` is pinned at its floor of 0.85 for the whole run (a
separate pre-existing finding, documented at `PROD_GROWTH_PER_YEAR` in
`tick/mod.rs`) — so the glut is not productivity growth. It is pure retained
surplus.

Four mechanisms, each individually sufficient:

1. **Consumption is capped at `need`, so 100% of any surplus is retained.**
   `eat = need.min(stock)`. A city with 13,000 days of silk consumes exactly the
   same as a city with none. There is no dumping, no glut discount, no "we made
   too much this year".
2. **Production carries no price signal, anywhere.** Neither the raw pass
   (`production[g] = percap × pop × …`) nor `manufacture_pass`
   (`let made = by_inputs.min(labor_cap);`) reads a price, a cost or a margin.
   Nothing in this economy ever produces less because the thing is worthless.
3. **Durable goods never decay.** `warehouse_and_spoilage_pass`
   (`colonies.rs:798`) opens with `if rate <= 0.0 { continue; }` and `rate` is
   `perishable × SPOIL_PER_PERISHABLE`. **31 of the 45 shipped goods have
   `perishable = 0.0`** — including every metal, every gem, salt, silk, spices,
   timber, cotton, ceramics, glassware. For those goods spoilage is exactly zero
   and `stock` is a strictly non-decreasing function of time.
4. **The city warehouse capacity is decorative for exactly those goods.**
   `wh_capacity` (population × 0.08 + structures) exists and is shown in the UI,
   but its only consequence is `SPOIL_OVERFLOW_MULT` — a multiplier *on the
   spoilage rate*. Multiplying zero by two is zero. **A city's stated capacity
   cannot bind on any non-perishable good.** That is why the warehouse panel
   reads "tons of goods stored": the number is real, the cap beside it is inert.

The price column is the proof that the market has stopped functioning:
`price/base` sits between **0.138 and 0.52** for almost every good at every date,
against a `PRICE_FLOOR_MULT` of **0.15**. The whole world lives within a hair of
the price floor from year one; `live_price = base × (need/stock)^k` cannot go
lower, so the clamp does the work and prices carry almost no information.

The one instructive exception is IRON, which reaches **0.942** at year 100 with
`production ÷ need` finally dipping to **0.96** — the only good in the run that
ever comes close to scarcity, and even it never crosses 1.0× base. That single
excursion is worth more than the rest of the column: it shows the price mechanism
is alive and would respond, if anything ever made a good genuinely scarce.

---

## 4 · Manufactories — why fewer are built, and the icon question

### Why fewer

`maybe_found_guild_workshop` (`houses.rs:967`) founds **at most one workshop in
the entire world per month**, and only where:

```rust
let demand = self.demand_pressure_at(h, g);
if demand < WORKSHOP_MIN_DEMAND { continue; }        // 1.08
```

and

```rust
pub(crate) fn demand_pressure_at(&self, h: usize, g: usize) -> f32 {
    (price / base).clamp(0.6, 3.0)
}
```

So a workshop opens only where a good sells for **≥1.08 × its base value**. §3
just measured that the world sits between **0.138 and 0.942 × base** — the
maximum reached by any good at any date in a 100-year run — so
`demand_pressure_at` returns at most 0.942 and usually its clamped floor of
**0.6**. It never once reaches 1.08, in any city, for any good, from year one.
The gate is not merely rarely satisfied; on this evidence it is unsatisfiable.

**The glut is the cause of the manufactory shortage.** It is one bug, not two:
unbounded stock → prices pinned at the floor → the demand gate can never clear →
workshops are never founded. `maybe_found_estate` uses the same figure as a
*score* rather than a *gate*, which is exactly why raw estates keep being founded
while manufactories dry up — the asymmetry you noticed is a gate-vs-score
difference, and it is directly traceable.

Two secondary limits, both smaller: `MAX_TOTAL_ESTATES` = 220 world-wide shared
between farms, mines, plantations, fisheries, vineyards, manufactories *and*
outposts; and one workshop per (city, good) pair.

### Why the icons don't correspond

There are two different things being drawn and they disagree.

- `ESTATE_EMOJI` (`HubPanel.tsx:46`) is keyed on `estate_kind`:
  `1 🌾 · 2 ⛏️ · 3 🌿 · 4 🎣 · 5 🍇 · 6 🏭`. A manufactory is always 🏭 — one icon
  for a weaving house, a foundry, a glassworks and a paper mill alike.
- Beside it the row draws `iconFor(e.good)` — the *good's* emoji from
  `GOOD_DEFS`. For a manufactured good that is often the raw material's glyph or
  a generic one.

And `estate_kind` itself is assigned by **substring matching on the good's
name** (`estate_kind_for_good`, `mod.rs:1320`) — `n.contains("gem")`,
`n.contains("salt")`, `n.contains("stone")`. Its own doc comment records that
this table had already gone stale once: the entire gem split, tin, lead, marble,
jade, mercury, alum, lapis and turquoise all fell through to "Plantation". The
world side knows the answer exactly (`GoodSpec.distribution`), but `TickGood`
does not carry `distribution`, so the campaign re-derives it from a string.
**That is the icon bug's root: the kind is guessed from a name instead of read
from the spec**, and any good whose name does not contain one of ~26 magic
substrings is silently labelled a plantation and drawn with 🌿.

---

## 5 · How the strata react — and the Vic3-pops question

The strata are a **single scalar tilt on the whole city's demand**:

```rust
// cities.rs:1467
match need_tier {
    2 => (1.0 + 0.45 * (elite - 0.18) / 0.18).clamp(0.4, 1.8),   // luxury
    0 => (1.0 + 0.45 * (mass  - 0.82) / 0.82).clamp(0.6, 1.4),   // basic
    _ => 1.0,                                                     // comfort: nothing
}
```

That is the *entire* interaction between society and the economy. Consequences:

- **A patrician and a labourer buy the same basket**, scaled. There is no
  per-stratum need, no per-stratum income, no budget constraint, and a stratum
  cannot go without while another is comfortable. `lack_basic` is a city-wide
  average, so "the poor are starving while the rich dine" is not expressible.
- **The comfort tier is literally neutral** — the burgher middle, the one
  stratum whose whole historical identity is comfort consumption, has no
  demand signature at all.
- **The clamp bites early.** At 0.4–1.8 the richest imaginable city can want at
  most 1.8× the luxury a beggars' town does. In practice the strata shares move
  slowly and this multiplier spends most of its life within a few percent of 1.0.
- **`Pop` is inert** (already recorded in §5.1 of `CLAUDE.md`): `hubs[h].pops` is
  written yearly and read *only* by `campaign_get_pops` for display.
  `militancy` and `consciousness` are computed and thrown away. The nine
  `POP_PROFESSIONS` exist as a display list and drive nothing.

**On Vic3-style pops:** the honest description is that the model has the *data
shape* of pops (shares, professions, a wealth figure, militancy/consciousness)
and none of the *mechanism* (no pop income, no pop budget, no pop-specific
basket, no promotion/demotion driven by needs met, no political consequence).
Expanding the strata **window** on top of today's model would present four
numbers that do almost nothing as though they ran the city — which is a worse
outcome than the plain view, because a UI that implies causation the sim does not
have is the failure this codebase has already paid for repeatedly (§8.18's
hand-copied palettes, §8.24a3's three render-drops-a-fact bugs). **The window
should follow the mechanism, not lead it.**

---

## 6 · Buyers in the market view — what can and cannot be shown

You asked to see buyers. Here is exactly what exists.

`supply_accum` is a flat `goods × 5` tally per hub with five **seller** classes —
`SUPPLY_CITY` / `SUPPLY_HOUSE` / `SUPPLY_GUILD` / `SUPPLY_LOCAL` /
`SUPPLY_FOREIGN` — decayed 2%/day. `CityMarketView` already draws it as the
"who supplied it" bar.

**There is no buyer-side equivalent, and it is not an oversight — there is
nothing to attribute.** Household consumption (§1) is an anonymous `stock_take`
with no counterparty. The only identifiable purchasers in the whole model are:

| purchaser | exists? | where |
|---|---|---|
| a merchant house buying to re-export | yes | `dispatch`, but **95.7% of shipments carry `owner = -1`** — nobody (`econ_measure_carrier_mix`, `ACTORS_AND_CARRIAGE_PLAN.md` §1) |
| a city council's right-of-first-buy / granary | yes | `council_provision_pass` |
| a manufactory consuming inputs | yes | `manufacture_pass`, per (hub, good) |
| **the population** | **no** | anonymous sink |

So a truthful "buyers" panel today would read: *local merchants ~96%, the
council a few percent, workshops a few percent, and the city's own inhabitants —
unattributed*. That is a real and interesting reading, and it is the same finding
`CITY_TRADERS_PANEL_PLAN.md` already recorded for the carrier list. **It must not
be dressed up.**

The two cheapest honest wins here, both of which need no new sim state:

- **Manufactory input draw is already per (hub, good)** — `manufacture_pass`
  knows exactly how much wool each weaving house pulled. Booking that into a
  `demand_accum` mirror of `supply_accum` would give a genuine buyer class.
- **"Made here, and where it went"** — your request to see the settlement's
  production and how much was sold into the city. The production half already
  exists (`CityMarketView`'s MADE HERE block). The "sold to the city" half does
  not, and cannot, because the sale is not an event. What *can* be shown truthfully
  is the physical disposition: *made X, ate Y (the ration), shipped out Z, added
  W to store* — a four-way flow balance per good. Every one of those numbers
  exists today. It is the right panel, and it says something real.

---

## 7 · The historical critique, stated plainly

Take the model as a claim about a pre-modern economy and score it.

1. **A pre-modern city was a stomach.** 60–80% of all household spending was
   food, most of that bread; the grain trade *was* the trade, and grain politics
   *was* politics. This model spends 12.4% on food and 70% on luxuries. It is not
   a medieval economy with the dial mis-set — it is structurally a modern luxury
   market wearing period clothes.
2. **Scarcity was the normal condition.** Granaries held months, not decades.
   Venice's *Camera del Frumento* existed because the city was two bad harvests
   from famine. Here the world holds 3.8 years of grain in year 25 and prices sit
   at the floor — the model has abolished dearth, and with it the entire
   political economy that dearth generated (the *annona*, the *tratta*, bread
   riots, the granary as an instrument of rule). The crisis-relief machinery
   (`decide_crisis_relief`) is built and correct, and can essentially never fire.
3. **Price signalled to producers.** The whole mechanism of a pre-modern economy
   — why Flanders wove and Poland grew rye — is producers responding to price
   over years. Here production is fixed per-capita and price-blind, so
   specialisation is frozen at campaign start and can never emerge, shift, or die.
4. **Luxuries were carried in tiny quantities at enormous margins.** The
   Portuguese pepper fleet was a few thousand tons a year for a continent. The
   value density is what made the trade worth a year at sea. Here the *quantities*
   are comparable to staples, which drains the romance out of the long-distance
   trade the whole game is about: a gem cargo is not a special event, it is
   Tuesday.
5. **Households were the economy.** Making them a sink rather than an agent
   removes the demand side, the labour market, wages, saving, and any channel by
   which a city getting richer changes what it buys. This is also, mechanically,
   why the top-decile wealth share has been so hard to keep in band — the
   merchant layer is the *whole* monetary economy, so all wealth concentrates
   there by construction.
6. **What the model gets right, and should not lose.** Grain-equivalent
   numeraire; category substitution (real cross-price elasticity, already live);
   freight by bulk and distance; the two-pass staple/luxury need ladder; the ore
   geology of §8.16; localities of §8.19; the seasonal sailing window (N5). The
   bones are good. The demand table and the absence of a price→production loop
   are what break it.

---

## 8 · What I would propose, in the order I would build it

Every item states a **gate that is not the target**, per §2.4.

**P1 · Re-scale the demand table so a city eats.** Set `GOOD_DESIRE` and
`GOOD_NEED_TIER` from a target *expenditure share* rather than by feel: food to
55–70%, luxury tier under 10%. Concretely this needs `TIER_WEIGHT`'s luxury arm
far below 0.22, `foreign_lux`'s prestige term capped or inverted (a dearer good
should be wanted in *smaller quantity*, not larger), and per-good desire figures
that are sanity-checked against real consumption. *Gate:* a new
`econ_expenditure_shares_resemble_a_household` asserting food's share of
consumption spend lands in 0.50–0.80 — and `econ_fidelity_scorecard`'s existing
bands must not regress. Cheapest change here by far, and it alone fixes the "gems
in bulk" symptom.

**P2 · Give durables a real sink.** Three candidates, cheapest first: a small
non-zero `perishable` for everything (leakage, theft, rot, breakage — historically
real for every good including metal); make `wh_capacity` actually bind by
discarding or forcing sale above it; or a per-good *depreciation in use* (a
consumed durable is genuinely gone). *Gate:* `days_held` per good must stay under
a stated ceiling over a 100-year run — the diagnostic in §0 is already the
instrument.

**P3 · One price term in production.** The minimum viable version is a
multiplier on the raw pass and `manufacture_pass`:
`output *= f(price/base)`, monotone, clamped to something like 0.5–1.5, with a
lag. This is the change that makes specialisation emergent and makes P2 less
load-bearing. *Gate:* `econ_inheritance_rules_fragment_differently` and the
dynamics run's bounded-wealth assertion — this is the item most likely to move
wealth, so it is dosed from zero exactly like N1/N2/N6.

**P4 · Fix the manufactory gate at its source, then re-measure.** Do *not* lower
`WORKSHOP_MIN_DEMAND` — that treats the symptom. P1–P3 should bring prices off
the floor; then count workshops again. If they are still scarce, the gate is
worth revisiting on evidence. *Gate:* a new `econ_measure_workshop_founding`
counting workshops founded per century, before and after.

**P5 · Read `estate_kind` from the spec, not from a substring.** Carry
`distribution` (or a precomputed kind) on `TickGood` at campaign start. This
removes the whole class of icon/label/depletion-curve bugs at once, including the
one already documented in `estate_kind_for_good`'s own comment. *Gate:* a test
asserting every shipped `Deposits` good resolves to kind 2 — which fails today.

**P6 · A buyer ledger (`demand_accum`), mirroring `supply_accum`.** Book
manufactory input draw, council provisioning and house re-export purchase. Leave
household consumption explicitly unattributed until §1 changes. *Gate:* class
totals must sum to the goods actually leaving stock.

**P7 · The four-way flow balance in the market view** (made / eaten / shipped /
stored, per good). No new sim state. This is the panel your "production and how
much is sold to the city" question is really asking for, in the strongest form
the model can honestly support.

**P8 · Only then, per-stratum consumption** — a real basket and a real budget per
stratum, which is what makes a Vic3-style pops window mean anything. This is a
large change and it depends on P1 and P3; it should not be started first because
it is the most fun.

---

## 9 · Questions I need answered before any of this is built

1. **How far do you want to go?** P1+P2+P5 is a *tuning and plumbing* job that
   makes the existing model behave — a week's work, low risk, and it fixes every
   symptom you actually reported. P3+P6+P8 is a *new economic model* — a
   price→production loop and households as agents — which is a much larger,
   riskier project that will move every `econ_` band and needs its own dosing
   plan. Which of those two are we doing?
2. **Should the population pay for what it consumes?** This is the fork in the
   road. Making consumption a monetary transaction gives you buyers, budgets,
   real strata, dearth, and bread politics — and it changes the wealth
   distribution the whole finance layer is tuned against. Everything in P6/P8
   depends on the answer, and I do not want to guess it.
3. **Should a mine's output come from the ore body instead of the population?**
   §8.16 already computes `grade`, `extent` and `depth` per working and the
   campaign uses only `grade`. Wiring `extent` to a production *ceiling* would
   stop a big city mining gems in proportion to its citizens — but it would also
   make `Deposits` goods genuinely scarce, which is a real economic change and
   `DEPOSITS_AND_MINING_PLAN.md` slice 4 is where it belongs.
4. **When the glut is gone, is starvation acceptable?** Today no city can starve
   because everyone holds years of grain. Fix that and famine becomes possible —
   which is historically right and is what the relief machinery was built for,
   but it will kill cities. Do you want that, and how hard?
5. **What is the manufactory window for?** A works card already exists
   (`works_monthly_pass` samples output/quality/price into a 12-month ring). If
   the ask is "who buys from this manufactory", that needs P6 first. If it is
   "what does it make, from what, how well, and is it worth keeping", that can be
   built today from existing state. Which one?
6. **How many manufactories is right?** "Fewer are being made" is a comparison to
   an expectation I do not have. A number — workshops per city, or per century —
   would let me put a gate on it instead of a feeling.

---

## 10 · One thing that is not a bug and should not be "fixed"

`econ_measure_carrier_mix` reports 95.7% of shipments carried by nobody
(`owner = -1`), and any buyers/traders panel will show it. That figure is a real
measured finding about the model, recorded in `ACTORS_AND_CARRIAGE_PLAN.md` §1,
and the plan for it (N1's dose walk) is separate work. A market panel must
report it, not flatter it.
