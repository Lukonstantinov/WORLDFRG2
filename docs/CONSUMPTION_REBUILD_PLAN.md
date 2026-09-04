# Consumption rebuild — the plan

**Status: S1/S2/S4/S6/S8(partial) BUILT. S3/S5/S7 BUILT AND WIRED, DOSED TO A
VERIFIED-ZERO NO-OP** — the mechanism is real (a pure/parametrized function
split, e.g. `production_price_mult()` calling `production_price_mult_e(..,
dose)`), gated bit-identical at the shipped zero dose
(`production_price_mult_is_a_noop_at_zero_and_correctly_signed`,
`s5_ore_ceiling_at_zero_is_a_noop`, `s7_household_monetization_at_zero_is_a_
noop`), and the actual dose walk for each (measured against `econ_` per step,
per §0's rule) is separate, un-started future work — see the N1/N6 pattern
this follows. S8's flow-balance and buyers-beside-sellers PANELS are not
built; only the backend `demand_shares`/`supply_shares` data they would read
is wired (`HubGoodDetail`, `commands/campaign_commands/read_hubs.rs`).

The measured case for this work is `docs/CONSUMPTION_AND_GOODS_REVIEW.md`; read
it first. This document is only the build order, the gates and the risks.

**Three decisions taken, all on the maximal path:**

1. Scope — **add a price→production loop**, not just tuning.
2. Buyers — **yes, the population pays for what it consumes.**
3. Mines — **yes, wire a deposit's `extent` to a production ceiling.**

Those three together are a new economic model, not a tuning pass. The whole risk
of this plan is that it will move every `econ_` band and the dynamics test's
wealth bound at once, so the ordering below exists to make each move
attributable to one change.

---

## 0 · The three rules this plan runs under

- **One slice, one gate, one push.** Every slice below lands as its own commit
  with its own named gate and a re-run of `econ_` + the dynamics test. Bundling
  two of these is how the four-commit red main of 2026-08-20 happened.
- **Every behavioural slice is dosed from zero.** S3, S5 and S7 each ship with a
  constant that makes them provably inert, a gate asserting the inert case is
  bit-identical, and a separate dose walk. This is the N1/N2/N6 pattern and it is
  not optional here — two of the three are wealth-concentration risks.
- **A gate that is not the target.** Each slice names a gate it must not
  regress, distinct from the number it is trying to move (§2.4).

---

## S1 · The demand table becomes a BUDGET SHARE

**The single highest-value change in this plan, and the cheapest.**

`base_need` treats `TIER_WEIGHT[tier] × desire` as the QUANTITY a head consumes
per day, then prices it with `base_value` afterwards. That makes a good's share
of household spending *rise* with its price — backwards, and the direct cause of
food at 12.4% and gems at 13.2× bread.

The fix is one division. The same two table entries now name a **share of
spend**, and quantity is derived by dividing by `base_value` — constant-share
(Cobb-Douglas) demand, the standard model and the one Allen's basket work
assumes. No table is rewritten; only what the tables *mean* changes.

Measured over the shipped 45 goods:

| | food & drink | luxury tier | gem:wheat spend | gem:wheat qty |
|---|---|---|---|---|
| today | 12.4% | 69.7% | 13.17 | 0.220 |
| ÷ `base_value` alone, no other edit | **46.5%** | 23.8% | 0.22 | 0.0037 |
| + `TIER_WEIGHT` `[1.0, 0.28, 0.07]` | 56.7% | 13.9% | 0.10 | 0.0017 |
| + `LUX_IMPORT_DESIRE` 0.7 → 0.4 | **61.0%** | **8.8%** | **0.05** | **0.0009** |
| historical target | 60–80% | <10% | ~0.003 | — |

Divide by `base_value`, the good's *intrinsic* worth — **never the live price**.
Dividing by a live price would make aggregate demand perfectly price-elastic and
duplicate N6's `elastic_aggregate_mult`, which is deliberately applied *outside*
`base_need` and to the category aggregate only.

**Also required, or the whole world is mis-scaled:** `need_scale`
(`lifecycle.rs:708`) is calibrated as `total_prod / (total_pop × sum_tw_desire)`,
where `sum_tw_desire` is `Σ TW[tier]×desire`. That sum must gain the same
`/base_value` and `foreign_lux` terms, or day one starts at the wrong absolute
level. This is the one part of S1 that is easy to miss and impossible to see.

- **Gate (new):** `econ_expenditure_shares_resemble_a_household` — food & drink's
  share of consumption spend must land in **0.50–0.80** and the luxury tier under
  **0.15**, computed the way §2 of the review computes it. Verify it FAILS on
  unfixed code (it reads 0.124 / 0.697 today).
- **Gate (not the target):** `econ_inheritance_rules_fragment_differently`. This
  is a demand-constant change and that gate has flipped on exactly this kind of
  change five times. Also the dynamics run's bounded-wealth assertion.
- **Expect to move:** everything. Food becomes the bulk of trade, which is
  historically right and will change every route, every house's earnings and the
  basket price gradient. Record the scorecard before and after in `SCOREBOARD.md`
  as one attributable step.

**Risk:** food demand rising ~5× against unchanged food production could starve
the world on day one. `lifecycle.rs` already carries a founding food-viability
constraint for exactly this reason; check it still binds, and expect to re-run
the S1 numbers with it in the loop.

---

## S2 · Durables get a real sink

31 of the 45 shipped goods have `perishable = 0.0`, so
`warehouse_and_spoilage_pass` early-returns for them and their stock is a
strictly non-decreasing function of time. `wh_capacity` is shown in the UI and
its only mechanical effect is a multiplier *on the spoil rate* — multiplying zero
by two is zero, so **a city's stated capacity cannot bind on any durable good.**

Three candidate mechanisms, cheapest first. Build one, measure, stop.

1. **A non-zero floor on `perishable` for everything.** Leakage, theft,
   breakage, rust, rot — historically real for every good including metal.
   One constant, no new state.
2. **Make `wh_capacity` bind.** Above capacity, stock is discarded or forced onto
   the market at a discount. This is the mechanism that makes the *warehouse
   panel* honest, which is what was actually reported.
3. **Depreciation in use** — a durable consumed is genuinely gone rather than
   returning to stock. Cleanest economically, largest change.

- **Gate (new):** `econ_stock_never_exceeds_a_years_supply` (name provisional) —
  `days_held` per good stays under a stated ceiling over a 100-year run.
  `econ_measure_goods_stock_and_price` is already the instrument.
- **Gate (not the target):** the dynamics run's town survival — a sink that is
  too strong starves cities, and "no towns lost by year 40" is the existing
  observation to hold.

---

## S3 · One price term in production (**dosed from zero**)

The substantive change of the whole plan. Neither the raw pass
(`production[g] = percap × pop × …`) nor `manufacture_pass`
(`made = by_inputs.min(labor_cap)`) reads a price, a cost or a margin, so nothing
in this economy ever makes less of a worthless thing and specialisation is frozen
at campaign start.

Minimum viable form: a monotone multiplier on both passes,
`output *= f(price / base_value)`, clamped (something like 0.5–1.5) and **lagged**
— a farmer responds to last year's price, not today's, and an unlagged loop will
oscillate. `PROD_ELASTICITY = 0.0` makes it exactly 1.0 and provably inert.

- **Gate (new):** `s3_price_elasticity_at_zero_is_a_noop` — bit-identical at zero
  dose, the N1/N6 pattern. Then `s3_a_glutted_good_is_made_less_of`.
- **Gate (not the target):** `econ_inheritance_rules_fragment_differently` and
  the dynamics run's bounded-wealth assertion, **re-run per dose step, not per
  slice** — that is `ACTORS_AND_CARRIAGE_PLAN.md` §5.2's own recorded lesson.
- **Watch for:** the loop interacting with S2. If S2 lands first, S3 has less
  work to do and can be dosed gentler; that is why it is ordered second.

---

## S4 · `estate_kind` from the spec, not from a substring

`estate_kind_for_good` (`mod.rs:1320`) matches ~26 magic substrings against the
good's *name*. Its own doc comment records the table having gone stale once: the
entire gem split, tin, lead, marble, jade, mercury, alum, lapis and turquoise all
fell through to "Plantation" — the wrong icon, the wrong label and the wrong
depletion curve. The world side knows the answer exactly (`GoodSpec.distribution`)
but `TickGood` does not carry it.

Carry `distribution` (or a precomputed kind) on `TickGood` at campaign start.
Removes the whole class of bug at once. Small, self-contained, and a prerequisite
for S5.

- **Gate (new):** every shipped `Deposits` good resolves to kind 2 (Mine) — which
  fails today.
- **Also:** one icon per manufactory kind rather than 🏭 for a weaving house, a
  foundry, a glassworks and a paper mill alike.

---

## S5 · A mine's output comes from the ore body (**dosed from zero**)

Today `production[gems] = base_per_capita × population`, so a metropolis mines
gems twenty times faster than the town on the same lode, and `base_per_capita`
was fixed once at campaign start. §8.16 already computes `grade`, `extent` and
`depth` per working and the campaign reads only `grade`.

Wire `extent` to a production **ceiling** (not a multiplier — a lode has a
maximum rate, and population below that still limits output, which keeps a small
town from out-mining a city on the same body). `depth` should gate workability,
which is `DEPOSITS_AND_MINING_PLAN.md` slice 4's own subject — coordinate, do not
duplicate.

- **Gate (new):** `s5_a_lode_caps_output_regardless_of_population` — two hubs of
  very different size on the same working produce within a stated ratio.
  Inert-at-zero-dose gate as above.
- **Gate (not the target):** `goods_` coverage (rule 26) — a ceiling that bites
  too hard makes a mineral effectively absent, which is the silent-vanish failure
  §8.16 has already been caught by twice.
- **Note:** this makes `Deposits` goods genuinely scarce for the first time.
  Expect prices for them to rise off the floor, which is the point, and expect it
  to interact with S3's elasticity.

---

## S6 · A buyer ledger (`demand_accum`)

Mirror `supply_accum`'s shape and decay with the buyers that already exist:
manufactory input draw (`manufacture_pass` already knows the exact per-(hub,
good) figure), council provisioning, and house re-export purchase. Household
consumption stays **explicitly unattributed** until S7.

- **Gate (new):** class totals sum to the goods actually leaving stock.
- **Must report, not flatter:** `econ_measure_carrier_mix` finds 95.7% of
  shipments carried by nobody (`owner = -1`). Any buyer or trader panel will show
  that. It is a real recorded finding (`ACTORS_AND_CARRIAGE_PLAN.md` §1) with its
  own separate plan; do not suppress it to make the panel look better.

---

## S7 · The population pays (**the fork in the road, dosed from zero**)

Consumption becomes a transaction: the household stratum pays for what it eats,
money leaves the population and reaches whoever sold it. This is what gives you
real buyers, real budgets, a stratum that can go without while another does not,
dearth, and bread politics.

Build in this order, and stop at each step to measure:

1. **Household income.** Nothing currently pays wages to the population.
   `commoner_wealth` exists as an eased per-capita figure with no real source.
2. **A budget constraint.** Consumption becomes `min(ration, what can be
   afforded)`, so a price spike genuinely prices people out — which is what makes
   S3's price loop close.
3. **Per-stratum baskets.** Patrician / burgher / commoner / underclass each get
   their own tier weights and their own income. Today the strata are a single
   clamped 0.4–1.8 tilt with the **comfort tier literally neutral**.
4. **Only then** the Vic3-style pops window. `Pop` is inert today
   (`militancy`/`consciousness` computed and discarded); a window built before
   the mechanism would imply causation the sim does not have, which is the exact
   failure §8.18 and §8.24a3 already record.

- **Gate (new):** `s7_at_zero_dose_consumption_is_unchanged`; then
  `s7_the_poor_go_short_before_the_rich_do`.
- **Gate (not the target):** the top-10% wealth share band. This is the big one —
  the merchant layer currently holds *all* the money in the model by construction,
  and giving households a monetary existence redistributes away from it. That
  band has been hard-won (Phase 4.3, Phase 5) and this slice is the most likely
  thing in the plan to break it.

---

## S8 · The market view tells the truth

Two panels, both buildable from state that exists once S6 lands.

- **The four-way flow balance per good** — made / eaten / shipped out / added to
  store. This is what "show me the settlement's production and how much was sold
  to the city" is really asking for, in the strongest form the model can honestly
  support.
- **Buyers beside sellers** — `demand_accum`'s classes next to `supply_accum`'s,
  with the unattributed household share shown as unattributed.
- **The estate/manufactory window** — a works card already exists
  (`works_monthly_pass` samples output/quality/price into a 12-month ring). What
  it makes, from what, how well, and whether it is worth keeping is buildable
  today; *who buys from it* needs S6 first.

---

## Ordering, and why

```
S1 demand table ──┬─> S2 durable sink ──> S3 price loop ──┬─> S7 households ──> S8 panels
                  │                                        │
S4 estate kind ───┴─> S5 ore ceiling ────────────────────┘        S6 buyer ledger ──┘
```

S1 first because it is cheap, self-contained, and every later measurement is
meaningless while a city spends 70% of its money on jewellery. S2 before S3
because a sink makes the price loop's job smaller and its dose gentler. S4 before
S5 because the ore ceiling needs to know what a mine is. S7 last because it
depends on prices meaning something (S3) and is the only slice that can break the
wealth distribution the entire finance layer is tuned against.

**Two things that are explicitly NOT in this plan**, so they are not assumed
done: the ownerless-carriage residual (N1's dose walk, its own plan) and
`tech_factor` being pinned at its 0.85 floor for every campaign (a real
pre-existing bug, already documented at `PROD_GROWTH_PER_YEAR`, and its own
`econ_`-gated change).

---

## Open questions this plan does not answer

- **When the glut is gone, how hard should famine bite?** Today no city can
  starve because everyone holds years of grain. S2 and S3 make famine possible,
  which is historically right and is what the relief machinery was built for —
  but it will kill cities, and how many is a design call, not a measurement.
- **How many manufactories is right?** "Fewer are being made" is a comparison to
  an expectation I do not have. A number — workshops per city, or per century —
  turns it into a gate instead of a feeling.
- **Does household money come from wages, or from a share of city output?** S7.1
  is a genuine design fork and the rest of S7 sits on top of whichever is chosen.
