# Banks, money, and the crafts

> **REVIEW + PLAN. NOTHING IN §4 IS BUILT.** Every claim in §1–§2 was read out of
> the tree at the time of writing and is cited to a file and a symbol. §3 is the
> historical reading. §4 is a gated build order. §5 records what is deliberately
> out of scope, and §6 what earlier documents already proposed here so this one
> does not silently re-propose it.

The campaign has a bank with a real balance sheet, a coin with real fineness and
real trust, and a craft guild with a real quality ceiling. What it does not have
is a **channel from any of the three into the price of anything**. This document
measures that gap, checks it against what pre-modern finance and craft actually
were, and proposes the smallest set of changes that would make a bank a firm
that finances ventures rather than a ledger that accrues.

---

## 1. Measured findings — banks

Each read out of the code, not assumed.

**B-F1 · The balance sheet is genuinely good.** `Bank` (`tick/mod.rs`) carries
`reserves` + `loans` + `real_estate` + `stakes` against `deposits` + `notes_issued`,
with `equity()`, `reserve_ratio()`, per-year `BankSnapshot` history, and cumulative
`interest_earned` / `losses` / `dividends_earned` / `bills_income`. `bank_pass`
(`money.rs`) services loans, amortises principal correctly (a note-funded loan
RETIRES notes, a cash loan returns specie — the comment there records the bug where
principal simply vanished and every bank went insolvent within a few years),
pays depositors, runs an idiosyncratic run below `BANK_RUN_RATIO`, lets the owning
house recapitalise before failure, and fails on negative equity.
`ACTORS_AND_CARRIAGE_PLAN.md` §2 calls the bank "the one unambiguously
well-modelled actor" and on this axis that is right. **Nothing below is a
complaint about the accounting.**

**B-F2 · Nobody borrows. The bank decides to lend.** `bank_maybe_lend` fires on a
`hash01(...) > 0.3` roll, then picks the **richest** resident house (or the seat
treasury) and pushes money at it. There is no borrower-side demand anywhere: no
house ever reaches a venture it cannot afford and seeks credit. The loan is
credited straight to `houses[bh].wealth` with **no project attached** — it is
fungible cash, not project finance. Two consequences:
- The direction is backwards. Credit flows to whoever needs it least, which is
  a wealth-concentration pump pointed at the top of the distribution.
- Nothing can be *denied* credit, because nothing asks.

**B-F3 · The only demand-driven credit in the tree is the colony.** `colonies.rs`
(two sites: settlement colony, mining colony) raises `COLONY_FOUND_COST` as a real
joint-stock syndicate — the city puts in what its treasury allows, the strongest
resident house tops up, and **a bank on the same trade component is REQUIRED** or
the colony is not founded. Backers are recorded as `(kind, idx, share)` and paid a
proportional dividend out of the colony's trade surplus. This is the correct
pattern and it exists exactly twice. Its return is also nominal: the dividend is
`(trade_wealth · pop · COLONY_DIVIDEND_RATE).min(2.0)` — a hard cap of **2.0 per
period** against a bank founded with `BANK_FOUND_RESERVE = 40,000`.

And it almost never fires. The 50-year dynamics run (§1d) founds **one colony and
zero outposts**. So the only genuine venture finance in the campaign — the only place
a bank is asked for money by someone who wants to build something — happens about
once a century.

**B-F4 · A bank cannot hold public debt, though the docs say it can.**
`CLAUDE.md` §5 states the Monte "pays holders (houses **and banks**) pro-rata".
`TickHub.debt_holders` is `Vec<(u8, u32, f32)>` and the kind byte allows for it —
but the single issuance site in `update_public_debt` pushes only `(0, house, step)`,
and every payout loop opens with `if kind != 0 { continue; }`. **No bank has ever
held a bond.** This is the single largest missing asset class, and historically the
central one: the Casa di San Giorgio *was* the Genoese public debt.

**B-F5 · "Trading on credit" is not connected to any bank.** `BANK_CREDIT_MULT =
1.6` in `production.rs`'s dispatch loop multiplies a house's affordability if its
`archetype == ARCH_BANKING`. It applies whether or not that house owns a bank,
whether or not any bank would lend to it, is never repaid, and accrues no interest.
The archetype also grants `BANK_INTEREST` on wealth up to `BANK_INTEREST_CAP` with
no counterparty. Both are flat perks wearing the word "credit".
**The affordability term itself is real** — `amount = amount.min(afford)` binds
cargo size, and `diag_why_cash` counts the refusals — so this is a live channel
being driven by a fake number.

**B-F6 · Banks fail at roughly the rate they are chartered, and every failure is
systemic.** Measured on the standing dynamics run at `b578640`
(`simulate_decades_reports_dynamics`, 50 years — full digest in §1d): the LIVE bank
count after the year-20 charter gate opens goes **7 · 6 · 8 · 5 · 5 · 9 · 4** at
five-year marks. It never accumulates; banks are founded and destroyed at about the
same rate for thirty years. `fail_bank` unconditionally calls
`trigger_regional_crash`, so **every** bank death is systemic — and the run records
**12 regional crashes in 50 years**, one roughly every four years. There is no quiet
wind-down, no sale, no absorption by a rival, and no bank that simply endures. The
Medici bank ran 97 years.

**B-F7 · There is no instrument for finance.** `economy_validation.rs` carries 18
`econ_*` diagnostics — carrier mix, house turnover, war frequency, realm paths,
outpost founding, expenditure shares — and **not one measures a bank**. §2.5 of
`CLAUDE.md` exists because 16.7k lines of economy were covered only by mechanism
tests; the finance layer is in exactly that pre-oracle state today.

## 1b. Measured findings — money

**M-F1 · The monetary loop writes a number nobody spends.**
`update_price_levels` computes a per-hub inflation from coin circulation and
debasement and writes `TickHub.price_level`. Its readers, in full: the Money panel
(`read_money.rs`), the coin-biography snapshot, and the CPI line in
`econ_fidelity_scorecard`. **The market never reads it.** `live_price` is
`base_value · (need/stock)^k`, clamped — no money term, no price level, no coin.
So a council can debase to the floor and the price of bread does not move.

**M-F2 · Bimetallism is a display fact and a bank income line.** `coin_specie` /
`coin_exchange` implement a real `GOLD_SILVER_RATIO = 13.0`. Their only non-display
reader is the bills-of-exchange fee in `bank_pass`. **A merchant shipping between
two currency zones pays no exchange cost.** `Contract.coin` is explicitly documented
as a "tag only (no FX revaluation)".

**M-F3 · Coin reaches trade through exactly one channel, it is freight, and the
crashes switch it off.** `coin_discount(dest)` returns
`1.0 - COIN_FREIGHT_DISCOUNT · trust` (max 10%) once `trust >= RESERVE_TRUST_MIN`
(**0.55**), and a bank BRANCH can import its seat's trust to a weak-coin city. That
is the whole of money's effect on commerce, and it is a good mechanism — it is why a
strong-coin city becomes an entrepôt, and the one place the bank network already
touches the price of goods.

**But the measured world average coin trust runs 56% → 64% → and then falls to
43%** and stays there from year 25 on (§1d), because each of B-F6's twelve crashes
applies `CRASH_TRUST_HIT` across a whole trade component and trust never recovers
between them. So for most of the campaign most cities sit **below the 0.55
threshold**, `coin_discount` returns exactly 1.0, and money's single channel into
trade is closed. This is not a tuning observation about one constant: it is two
subsystems in a loop that neither was designed against — bank mortality suppressing
coin trust suppressing the entrepôt effect — and it is invisible without running the
digest, since both halves read healthy on their own.

## 1c. Measured findings — manufacture and the crafts

**C-F1 · Twelve craft guilds. In the world. For ever.** `GUILD_MAX = 12`.
`seed_craft_guilds` runs **once**, at first tick (`guilds_seeded`), assigning each
manufactured good to the single city that produced most of it in year zero. There
are **23 manufactured goods** in the shipped library, so eleven crafts have no guild
anywhere, every craft that has one has exactly one, and no guild is ever founded or
dissolved for the remaining 500 years. Florence alone had 21 *arti*.
*(Corrected: an earlier count of 21 missed `ceramics` and `glassware` — both are
declared as belt goods and then converted to `Distribution::Manufactured` with real
recipes further down `default_goods_spec`, so a grep for the `mg(` helper cannot see
them. Relevant beyond the count: glass DOES exist as a craft, so the Murano question
is not a missing-good problem — see `docs/INSTITUTIONS_BRAINSTORM.md` §3.)*

**C-F2 · A guild's whole economic power is a quality ceiling.** `run_craft_guilds`
lifts one good's local quality by `GUILD_QUALITY_STEP` toward `GUILD_QUALITY_CAP`
(0.92, or `GUILD_MONOPOLY_QUALITY_CAP` 0.97 under `LAW_GUILD_MONOPOLY`), rolls a
`GUILD_STRIKE_CHANCE` strike, and raises one hall worth +0.05 civic stability. It
cannot restrict entry, set a price or a wage, cap output, bar an outsider, or
require an apprenticeship. `ACTORS_AND_CARRIAGE_PLAN.md` §2 already names this.

**C-F3 · The manufactory has no capital, no wages and no fuel.** `manufacture_pass`
is `made = min(inputs/qty, craftsmen_ratio · labor · tech) · price_mult`. There is
no wage bill, no fuel input, no plant to build or maintain, and — with
`PROD_ELASTICITY = 0.0` — no price response. Output is bounded by inputs and by a
craftsmen headcount, and by nothing that costs money. So a manufactory cannot be
under-capitalised, which is precisely the condition that would make it want a bank.

**C-F4 · The workshop-founding gate is one dose from unsatisfiable.**
`maybe_found_guild_workshop` requires `demand_pressure_at(h,g) >= WORKSHOP_MIN_DEMAND`
(1.08), where `demand_pressure_at` is `(price/base).clamp(0.6, 3.0)`.
`docs/CONSUMPTION_AND_GOODS_REVIEW.md` measured that ratio's **maximum across every
good and every date at 0.94** — the gate was never once satisfied anywhere. S1's
budget-share rework has since moved prices, so this needs re-measuring rather than
re-asserting; it is listed as slice 0 below for exactly that reason.

**C-F5 · A bank may take equity in a manufactory, and only a manufactory.**
`bank_maybe_invest` buys `BANK_STAKE_SHARE` (25%) of the highest-tier un-staked
`estate_kind == 6` works in its branch network, on a 10%/month roll, and books it
as a `BankStake` plus two `Share` rows. Mines, fisheries, vineyards and plantations
are excluded by construction (D1 keeps extraction works on offtake, `offtake.rs`),
and the only writer of an offtake share row is the envoy path — which is inert at
its shipped dose. **The Fugger case — finance converting into mining output — is
therefore not reachable at all.**

## 1d. The measured run this document is written against

`cargo test --lib simulate_decades_reports_dynamics -- --nocapture` at `b578640`,
the standing dynamics gate (§2.1). Passing; wealth bounded and finite; houses turn
over. The finance columns:

| yr | banks (live) | crashes (cum.) | coin trust | public debt | bills income (cum.) | contracts | colonies |
|---:|---:|---:|---:|---|---:|---:|---:|
|  5 | 0 |  0 | 56% | — | 0 | 0 | 0 |
| 10 | 0 |  0 | 58% | — | 0 | 0 | 0 |
| 15 | 0 |  0 | 64% | 11c / 23,260 | 0 | 1 | 0 |
| 20 | 7 |  0 | 54% | 11c / 40,708 | 0 | 0 | 0 |
| 25 | 6 |  1 | 54% | 11c / 65,971 | 1,094 | 2 | 0 |
| 30 | 8 |  2 | 40% | 12c / 42,437 | 2,572 | 0 | 0 |
| 35 | 5 |  5 | 39% | 13c / 60,959 | 3,716 | 0 | 0 |
| 40 | 5 |  7 | 43% | 16c / 63,223 | 4,076 | 0 | 0 |
| 45 | 9 |  7 | 49% | 16c / 199,291 | 4,173 | 0 | 0 |
| 50 | 4 | 12 | 43% | 19c / 157,677 | 5,047 | 0 | 1 |

Five things are legible here that are not legible in the code:

1. **Crashes compound; banks do not.** Twelve crashes in fifty years — ~24/century —
   against a live bank count that ends *lower* than it peaked. Compare B-F6.
2. **Coin trust breaks the 0.55 reserve threshold at year 30 and never returns.**
   M-F3's channel closes for the second half of the run.
3. **The public debt is the biggest pool of capital in the finance layer** —
   157,677 across 19 cities, roughly four times a bank's founding reserve each — and
   B-F4 means no bank may touch it.
4. **Bills income is real and material** (5,047 cumulative) and no merchant pays it.
5. **Contracts and colonies are ~inert.** The two demand-side instruments the model
   has both round to zero.

---

---

## 2. What the three systems add up to

Money, credit and craft are each modelled to a decent standard *internally* and are
each **terminal**: they consume state and produce a number that is displayed.

| System | Real internal mechanism | What it changes in the economy |
|---|---|---|
| Bank balance sheet | full, correct, gated | a house's wealth (a loan), a works' owner-cut (a stake) |
| Coin fineness / trust | full, sticky, reformable | ≤10% freight at the destination — **and 0% for most of a run**, M-F3 |
| Price level / inflation | quantity-theory-lite | **nothing** |
| Bimetallic exchange | correct ratio | a bank's bill income |
| Craft guild | quality + strike + hall | one good's quality ceiling at one city |
| Manufactory | recipe + labor cap | output |

The line that matters is the third one. `TRADE_AND_MARKET_REVIEW.md`'s F1 measures
the market as too flat and `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` counts **eleven
built, wired mechanisms shipped at a dose of exactly zero**. The monetary loop is a
twelfth of the same shape, except that it has no dose knob at all — it is not
dosed to zero, it is simply not wired.

---

## 3. The historical reading

**A pre-modern bank did four things, and the model does one and a half.**

1. **Exchange** — the *campsor* at the bench, and above him the bill of exchange
   (*cambium per litteras*). This was the core business, not a sideline: the Medici
   earned on the bill, and the exchange rate carried the interest that canon law
   forbade to be named. `bills_income` models the fee correctly and then charges it
   to nobody — no merchant pays a spread, so the bank's income has no payer.
2. **Deposit and transfer** — giro banking. A Venetian merchant settled by book
   transfer at the Rialto without coin moving. The model has deposits (one house,
   at the seat, on a 25% roll) and **no transfer at all**.
3. **Credit** — and crucially, credit to **princes and cities**, not to the richest
   merchant in town. The Bardi and Peruzzi lent Edward III roughly 900,000 and
   600,000 florins and were destroyed by his 1345 default; the Fuggers financed
   Charles V's imperial election. The model lends to a treasury 20–45% of the time,
   which is right in kind, but the sovereign borrower carries no war, no election
   and no crown — and B-F4 means it cannot hold that debt as a bond.
4. **Conversion of debt into real assets** — the strongest omission. Jakob Fugger
   lent to the Habsburgs and took **Schwaz silver and Hungarian copper output** as
   security and repayment; the Medici held the Tolfa alum monopoly the same way.
   Finance *became* offtake. `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` A6 names exactly
   this and records it as deliberately not built.

**Three further mismatches worth naming.**

- **Rates.** `BANK_LOAN_RATE` 1.2%–2.8% and `BANK_DEPOSIT_RATE` 0.6% are *monthly*,
  i.e. ~15–39% and ~7.4% annualised, which is defensible for the period — Italian
  *discrezione* on deposits ran near 7–10% and commercial credit well above that.
  But they are undifferentiated by BORROWER. The historical spread between a
  well-secured commercial loan and a princely one was the whole story of who went
  bankrupt. The model prices risk by *purpose string* (`"treasury" => 0.0`,
  `"guild_factory" => 0.60`) and never by the borrower's record.
- **Failure.** Bank failure was real and clustered — 1345 wiped out the Florentine
  super-companies and spread across Europe. But B-F6's near-total mortality with
  *every* failure igniting a regional crash is the wrong shape: it makes the crash
  routine, and a routine crash carries no information. The Medici bank ran 97 years.
- **Guilds.** The Ogilvie/Epstein argument turns on whether guilds were rent-seeking
  cartels or quality-and-training institutions; the model has taken the second side
  by building only the quality ceiling. But the *Verlagssystem* — the merchant
  supplying raw wool to rural weavers and taking the cloth, precisely to escape the
  guild town — is the mechanism that made the argument matter, and it needs a guild
  that can actually exclude someone before it means anything.
  `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` §9.3 already records putting-out as
  "truer than D1's dividend", adopted against, for a stated engineering reason.

---

## 4. Proposal — a build order, each slice with a gate

Ordered by ratio of economic truth gained to risk taken. Every slice states a gate
that is **not** the thing it is trying to move, per `CLAUDE.md` §2.4. Three of the
gate-sensitive ones are dosed from zero on the established N1/S3/S5 pattern.

### Slice 0 · The instrument (build first; changes nothing)
`econ_measure_finance` — an `#[ignore]`d 300-year diagnostic beside
`econ_measure_carrier_mix`, printing: banks chartered / failed / surviving and mean
lifespan; the composition of bank income (interest · dividends · bills · colony
dividends); the share of house wealth that is borrowed; loan count and mean size by
purpose; how often `diag_why_cash` refuses a shipment; and the live maximum of
`demand_pressure_at` (which settles C-F4 by measurement rather than by citing a
pre-S1 number). **No production code changes.** Without this, every slice below is
tuned blind — the same argument §2.5 makes for the economy as a whole.

### Slice 1 · Not every bank death is a crash (cheapest, unblocks M-F3)
Split failure from contagion. `fail_bank` currently *always* calls
`trigger_regional_crash`; give it three outcomes instead — **wound down** (equity
negative but reserves cover deposits: depositors are paid, the bank closes, no
crash), **absorbed** (a solvent rival bank in the same component buys the book at a
discount — a `Loan` and a `BankStake` are already transferable records), and
**collapse** (the present path, reserved for a bank whose deposits are actually
wiped). Only collapse ignites contagion.
*Why first:* it is the smallest change with the largest measured effect. It cuts
crash frequency toward a defensible few-per-century, which is what lets coin trust
climb back over `RESERVE_TRUST_MIN`, which re-opens the ONE channel money has into
trade (M-F3). It also gives bank lifespan a distribution instead of a cliff, which
every later slice needs in order to mean anything — a bank that dies every eight
years cannot hold a bond, finance a colony, or convert a loan into a mine.
*Gate — deliberately not the crash count:* the dynamics digest's **coin trust** must
stop collapsing below 0.55 and stay there, and wealth must remain bounded (a world
with fewer crashes has one fewer wealth haircut, and `CRASH_WEALTH_HAIRCUT` is
currently doing real work holding fortunes down — that is the risk this slice
carries and the reason the gate is trust plus the bound, not "are there fewer
crashes").

### Slice 2 · The Monte accepts a bank (small, high value, fixes a doc lie)
Let `update_public_debt` subscribe a **bank** as well as the richest house, and drop
the `if kind != 0 { continue; }` filter from the coupon, deleverage and haircut
loops. A bond is booked to the bank's own balance sheet as a distinct asset.
*Why second:* B-F4 is a real inconsistency with the documentation, the field already
allows it, it gives banks a **safe** asset that is not a loan to a merchant, and it
gives the coupon a systemic bite — a civic default now hits a bank's equity, which
is the historical transmission channel.
*Gate:* `simulate_decades_reports_dynamics` bounded; a fixture where a city defaults
must haircut a bank holder by exactly the same fraction as a house holder.

### Slice 3 · Credit is demanded, not pushed
Add a borrower side. A house that fails an affordability check it *wants* to pass —
`diag_why_cash` already counts these, and `decide_fleets` / `maybe_found_estate` /
`maybe_found_guild_workshop` all have a wealth gate they silently return from —
records a **credit need**. `bank_maybe_lend` then ranks applicants by need × record
instead of picking the richest resident. Rate keys off the borrower's own history
(prior defaults, `worst_loss`, `debt_since`) instead of the purpose string alone.
*Why:* this is the single change that turns the bank from an accrual into an
allocator, and it inverts the wealth-concentration direction of B-F2.
*Gate:* `econ_inheritance_rules_fragment_differently` — the fragile one, and the
right one here, because credit reallocated toward smaller firms is exactly the axis
that gate measures. Dose the reallocation weight from 0.

### Slice 4 · The price level is spendable
Multiply `live_price`'s `base` by the hub's own `price_level`, dosed from zero
(`MONETARY_PRICE_DOSE = 0.0` ships as a true no-op, proven by a bit-identical
dynamics run). At full dose, debasement raises nominal prices at the debasing city
and the Köppen-stable real economy underneath is unchanged.
*Why:* M-F1 is the largest single "computed and discarded" in the finance half.
*Gate — deliberately not the CPI:* `econ_fidelity_scorecard`'s **grain price/distance
gradient** and within-city price CV, since a monetary shock that only moves the CPI
line it is measured by proves nothing. Walk the dose per step, re-running `econ_`
each time, exactly as S1's `TIER_WEIGHT` walk did.
*Risk, stated:* this touches `live_price`, the hottest and most gate-sensitive
function in the tick. It may not survive at any non-zero dose. That result is worth
recording either way.

### Slice 5 · The bill is paid by someone
Charge a cross-currency settlement spread on a shipment whose two ends settle in
different coins, and route it to a bank with a branch at both ends — otherwise the
merchant eats a **larger** specie cost. This is A5's own "banks are structurally
necessary to distant trade", applied to cargo rather than to an envoy's deal.
*Why:* it gives `bills_income` a payer, makes `coin_exchange` load-bearing, and
makes a wide branch network a real competitive asset.
*Gate:* long-haul trade volume must not collapse — the same companion gate
`MERCHANT_VESSELS_AND_INFORMATION_PLAN` names, and the failure mode N1c hit twice.
Dose from zero.

### Slice 6 · A6 — finance becomes offtake (the Fugger slice)
Add `Loan.works_hub: i32`. A loan may be secured on a works; sustained arrears
convert the security into a `Share` row on that works — an **offtake** row for an
extraction works (`offtake.rs` already delivers physical goods, largest holder
first, finest band first) and a dividend row for a manufactory.
*Why:* it is the historically central bank mechanism, it makes B-F3's syndicate
pattern general, and `offtake.rs` and the `Share` table are already built and
currently reachable only through an inert envoy path.
*Why sixth:* `SCOREBOARD.md` records A6 as deferred specifically because it needs
this field and because retrofitting it touches `bank_pass`'s default handling —
"the single most gate-sensitive loop in the tick". Slice 0's instrument and
slice 3's borrower side both reduce that risk.
*Gate:* `econ_inheritance_rules_fragment_differently` per dose step, plus a unit
test that a converted loan moves value and creates none (the same zero-sum assertion
`a_division_moves_capital_and_creates_none` makes at `divide_estate`).

### Slice 7 · Crafts as a live population
`seed_craft_guilds` becomes a yearly `update_craft_guilds`: a guild is **founded**
where a craft is genuinely concentrated and **dissolves** when its craft leaves
town, with `GUILD_MAX` raised from a world cap to a per-city cap. Then give a
chartered guild one real exclusion — a `Law`-backed bar on an outsider's manufacture
of its good at its own city — which is the mechanism the *Verlagssystem* has to
route around to mean anything.
*Why last:* it is the largest behaviour change and the one with no existing
instrument. C-F1 is nonetheless the most obviously wrong number in this document.
*Gate:* guild count must track city count rather than a constant; a bar must be
escapable — measure that manufacture of a barred good rises in the barred city's
*hinterland*, or the mechanism is just a tax.

---

## 5. Deliberately NOT proposed

- **A wage bill and a labour market.** C-F3's real absence, and it is Part C of
  `FIX_PLAN.md` ("growth is exogenous"), not a finance change. Naming it here so it
  is not mistaken for something these slices deliver.
  `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` §9.4 already names labour as its largest
  historical hole, with the moral weight that carries.
- **Interbank lending and contagion graphs** (`SYSTEMS_21_PROPOSALS.md` B1). The
  crash mechanism is already *too* active (B-F6); adding exposure edges before
  fixing the failure rate would make it worse.
- **Fractional-reserve money creation as a macro aggregate.** `notes_issued` is
  already credit creation at the firm level. A world money supply needs slice 3 to
  land first or it has nothing to act on.
- **A player-facing bank verb.** `CLAUDE.md` §5.1 — there are four mutating verbs
  and B2 in `FIX_PLAN.md` owns that question for the whole campaign.
- **Share fragmentation by inheritance** (*Kuxen*). Named in
  `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` §9.3 as "the strongest single candidate for
  a later addition" and it still is — but it is a property change, not a finance one.

## 6. What earlier documents already say here

So this file does not re-propose them as new:

- `SYSTEMS_21_PROPOSALS.md` §3 — B1 interbank contagion · B2 endogenous rates
  (**since built**, `bank_maybe_lend`'s scarcity/risk/panic pricing) · B3 civic
  bailout · B4 collateralised loans (≈ slice 6) · B5 creditworthiness (≈ slice 3).
- `SOCIAL_ECONOMIC_WEALTH_PROPOSAL.md` §3.1 — "banking & financial crises are
  dormant", measured at 2 banks / 0 crashes in 50 years and recommending a **lower**
  founding bar. **That reading is now stale in both directions**: the run in §1d
  shows 4–9 live banks from year 20 and 12 crashes in 50 years, so banks are no
  longer rare and crashes are no longer absent — they are routine, and the
  recommendation to lower the founding bar would now make things worse. Do not act
  on that section without re-measuring (slice 0).
- `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` — A5 (a bank branch spanning both ends
  clears a deal; **built**, for envoys) and A6 (credit-conversion; **named and
  deliberately not built**, = slice 6). §9.3 records putting-out as historically
  truer than the dividend model and rejected on compatibility grounds.
- `CONSUMPTION_AND_GOODS_REVIEW.md` — the `demand_pressure_at` measurement behind
  C-F4, taken before S1 and needing a re-run.
