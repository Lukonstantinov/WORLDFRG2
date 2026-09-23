# Money and Coinage Plan — real money, mints, banks and barter

> **Status: M0-M4 SHIPPED (M4 partial — see its own row), gated,
> bit-identical — see `docs/SCOREBOARD.md` 2026-09-23. The plan's own "Stop
> marker" landing is complete. M5-M11 (barter, the monetary stages, paid
> consumption, the wealth/purse switch-over, banks on real reserves,
> price-level feedback) are each explicitly dosed-from-zero work per §5's own
> build rule — rushing a dose walk without its own gate sweep is the exact
> mistake §8.15 (CLAUDE.md) already recorded this project making five times,
> so each is shipped MECHANISM-FIRST at an inert dose and walked up with its
> own gate run, never blind.**
> Written from a brainstorm with the maintainer (2026-09-23). Supersedes
> nothing — it extends `BANKS_MONEY_AND_CRAFT_PLAN.md` (whose findings §1
> relies on) and `MONEY_MINES_AND_GOODS_PLAN.md` (whose slices 1-3 — notes
> retired on default, arrears, book diversification — are prerequisites here,
> already shipped).

The premise, in one sentence: **a coin should be a thing that is struck from
metal someone mined, carried by someone to somewhere, and spent by someone to
someone — never a number that appears in a vault.**

---

## 1. What is true today (measured by reading the code)

These are the findings this plan exists to answer. Each is stated with where it
lives so it can be re-checked rather than trusted.

- **F1 · Money is created from nothing on every sale.** `production.rs:2297`
  (`self.houses[oi].wealth += profit`) and `production.rs:2516` (the round-trip
  return leg) credit the carrying house. Nobody's purse goes DOWN:
  `hubs[b].export_earn` / `hubs[a].import_spend` are counters, the destination's
  population consumes without paying (`eat = need.min(stock)` has no
  counterparty — `CONSUMPTION_AND_GOODS_REVIEW.md`), and treasuries, bank
  reserves and seigniorage are all bare `+=`. There are ~150 `wealth/treasury
  +=/-=` sites across 18 tick files. There is no *quantity* of money in the
  world, only scores.
- **F2 · A coin is not an object.** It is `TickHub.coin_name` on the minting
  city, identified by that hub's index. One coin per mint, forever; a debased
  coin and its reformed successor are the same coin; there are no
  denominations. Names come from a 10-entry list (`money.rs::coin_denomination`),
  so a large world has several unrelated "Ducats".
- **F3 · Money never forms a price.** `live_price = base·(need/stock)^k`
  (`production.rs:639`) is in grain-equivalents. `price_level` is computed by a
  real quantity-theory loop (`update_price_levels`) and read by nothing except
  the inflation tax on fortunes and the panels.
- **F4 · Money's only effect on trade is usually off.** `coin_discount` shaves
  ≤10% freight when trust ≥ `RESERVE_TRUST_MIN` (0.55); the standing run holds
  average trust near 0.43 from year 30.
- **F5 · Crossing currencies is free.** `coin_exchange` prices only a bank's own
  bill income; a merchant pays no spread.
- **F6 · Barter exists only as a label.** `settle_coin = -1` means "barter — no
  coin physically reaches here" (`update_currency_baskets`), and it costs
  nothing and does nothing.
- **F7 · The coin basket is real and good.** `coin_basket` already spreads coins
  along real trade partners with sticky adoption — the one piece of this system
  that already obeys "coins travel". It is shown only as an overlay; the city
  market never mentions money.
- **F8 · Households have a purse field and nothing else.** `household_wealth`
  exists; `household_income_pass` is gated by `HOUSEHOLD_MONETIZATION_DOSE = 0.0`,
  whose dose walk was REVERTED because `update_food_and_starvation` reads raw
  stock, not what households could afford (CLAUDE.md §5, S7). That fix is a
  hard prerequisite for paid consumption here.

---

## 2. Decisions (maintainer, 2026-09-23)

| # | Decision |
|---|---|
| D1 | **Display first, then dose.** Every behavioural change ships at a dose of zero and is walked up one step at a time against the gates (§6). |
| D2 | **Issues + denominations.** A mint's currency has 2-3 denominations (gold trade coin / silver everyday coin / copper-billon petty); every debasement or reform is a new dated ISSUE. |
| D3 | **All four surfaces**: the coin catalogue, the market money view, money statistics, the bank redesign. |
| D4 | **Cities keep their own coins; only the successful mints survive.** A mint may CLOSE when demand for its coin falls. No realm-imposed royal coin in this plan (queued, §9). |
| D5 | **No counterfeiting or clipping crimes** for now. Wear is allowed as a plain physical rate. |
| D6 | **Units of account vary by culture/region**, and the gold:silver ratio in a region follows the metal actually present there. |
| D7 | **Victorian engraved art** for the catalogue (the `goodArt.ts` ledger treatment, not the flat heraldic `CoinIcon`). |
| D8 | **Follow a coin** from the catalogue — its events reach the news feed. |
| D9 | **Money is real and conserved.** Coins are struck, travel, and are spent; nothing appears in a vault. |
| D10 | **Parallel ledger first.** The conserved ledger runs beside today's wealth numbers until it is measured, then replaces them in dosed steps. |
| D11 | **Purses per holder per city.** A house's coin is somewhere; moving it needs a shipment or a bill of exchange. |
| D12 | **The full household loop.** Households are paid wages and pay for what they consume. |
| D13 | **Barter is always available — money is simply better.** A world starts in barter; credit houses and mints arrive later and coin spreads by trade. Barter never disappears: a trade with no acceptable coin still happens as a goods-for-goods exchange, with REAL stock moving both ways. |

On D13's order ("banks before coins"): it has a real precedent. Mesopotamian
temples and palaces kept accounts in weighed silver and grain for two millennia
before the first coin (Lydia, 7th c. BC), and Ptolemaic Egypt ran state grain
banks. So the sequence **barter → weighed metal and ledger houses → coin** is
modelled on history, not invented.

---

## 3. The model

### 3.1 Holders and purses

A **Purse** is `(holder, hub) → { coins: Vec<(issue_id, count)>, bullion: [f32; 3] }`
— sparse, only non-empty entries stored. Holders:

| Holder | Where its purses sit |
|---|---|
| City treasury | its own hub |
| House | seat + every office/bailo/warehouse hub |
| Bank | seat + every branch (these are its VAULTS; deposits are claims on them) |
| Households | one purse per hub (`household_wealth` becomes its reading) |
| Local merchants | one purse per hub — the carriers of today's ~96% ownerless trade (`econ_measure_carrier_mix`) |
| Mint | its hub — holds bullion awaiting striking |
| In transit | a coin chest riding an `InTransit` leg |

Memory: ~1,200 hubs × a few holders × `COIN_BASKET_N` issues, sparse — well
inside the budget `trade_last` already uses. Ordering of any fold over purses
must be by key, never by HashMap iteration (the basket determinism lesson,
`docs/SCOREBOARD.md`).

### 3.2 Bullion → mint → coin (the ONLY source of money)

1. Silver/gold (and copper for petty coin) are mined as goods (§8.16 deposits,
   `Mine` estates). Bullion is ordinary cargo: it travels to a mint by the same
   dispatch/relay machinery as any good, and can be lost at sea.
2. A mint strikes what bullion it holds, at the fineness its council chooses
   (`decide_coinage`'s existing decision, bounded by `mint_bullion_cap`). The
   striking splits the metal into: **seigniorage** (the council's cut → treasury,
   in coin), **brassage** (the cost of striking → mint workers' wages, i.e.
   household purse), and **circulation** (to whoever brought the bullion).
3. **Sinks** (the only ways money leaves): melting (a coin worth more as metal
   than face — the D5-compatible form of Gresham), loss at sea with its chest,
   hoards buried in sack or plague (§4.4), and a small wear rate per year.
4. **Conservation invariant**: `Σ coins everywhere = Σ struck − Σ melted − Σ lost
   − Σ buried`, per issue, checked every year in debug builds and by a gate.

### 3.3 Denominations and issues

```
Currency   { mint_hub, name_root, unit_of_account, denominations: [Denom; 1..=3], open: bool, closed_year }
Denom      { tier: Gold|Silver|Petty, name, standard_grams, issues: Vec<IssueId> }
Issue      { id, denom, year, authority (council house / realm dynasty head),
             grams, fineness, struck, circulating, hoarded, melted, lost,
             cause: First|Debasement|Reform|NewRuler|WarIssue, cognomen }
```

- A small polis typically strikes only silver + petty and uses foreign gold for
  large payments (historically correct — small cities rarely struck gold). The
  tiers a mint can strike follow the metals its region's bullion supplies.
- **Naming** replaces the 10-name list: a root from the culture's language kit
  (`names.rs`, the same machinery settlements use) + a tier suffix, plus a
  cognomen per issue ("the Lion Grosso", "Ducat of Doge Vitale", "the Black
  Money of 1261"). No two currencies share a name within a world.

### 3.4 Units of account and regional ratios (D6)

- Each culture carries a unit-of-account ladder (e.g. 1 : 20 : 12 pound/shilling/
  penny; 1 : 12 : 8; 1 : 60), resolved once per culture like `culture_rules`
  (§8.15) and never re-rolled.
- Prices are WRITTEN in the everyday silver coin's unit of account. Gold floats
  against it: a region's gold:silver ratio is derived from the gold and silver
  actually present in that region's purses (not a world constant), so a new
  silver strike visibly moves the local price of gold, and money changers profit
  from the gap between regions.

### 3.5 Barter — always available, never free (D13)

Barter is the fallback settlement of EVERY trade, not an era that ends. What
changes over the campaign is how often money is available to beat it.

**Mechanics of a barter trade.** A merchant delivering good X to hub B is paid
not in coin but in goods FROM B's own stock — the goods B values least relative
to what the merchant can sell at home (B's surplus, ranked by the same live price
gap dispatch already reads). So a barter settlement **moves stock both ways**:

- X is ADDED to B's stockpile (as today);
- the payment goods LEAVE B's stock and ride home with the merchant as a return
  cargo (an `InTransit` leg, `phase = 1`), where they are sold or bartered again.

This is the requirement that barter must visibly feed some goods INTO a city's
stockpile and send others OUT — a city in barter shows both columns in its
market book, rather than trade silently happening with nothing handed back.

**Why money is better** — barter carries three frictions that coin removes:

1. **Double coincidence**: if B holds nothing the merchant can resell at a
   margin, the trade is smaller (or does not happen). Measured by how much of the
   delivered value found an acceptable payment good.
2. **Valuation loss** (`BARTER_SPREAD`): goods-for-goods settles at a worse rate
   than coin — the merchant discounts goods he must carry and resell.
3. **Carriage of the payment**: the return cargo needs a hull/caravan slot; coin
   in a purse does not.

**Commodity money** emerges per market: the staple most often accepted as
payment there (grain, cloth, salt, cattle — whichever has the widest acceptance
in B's recent barter settlements) becomes the local unit, and a barter market's
prices read "reckoned in measures of barley". This costs one argmax over the
hub's recent settlements; nothing new is persisted beyond its name.

A trade chooses coin when both sides can use it (the seller's hub accepts an
issue the buyer holds) and falls back to barter otherwise. Nothing forbids
barter in a monetised city; it just loses.

### 3.6 The three monetary stages, spreading by trade

| Stage | Where it exists | What it does |
|---|---|---|
| **Barter** | everywhere, always (§3.5) | goods-for-goods; commodity money emerges |
| **Weighed metal + ledger houses** | where bullion reaches and a house/temple opens a credit house | settlement by ledger entry between ACCOUNT HOLDERS in that city and its branches; bullion changes hands by weight (with a weighing cost) |
| **Coin** | a city holding a ledger house, reachable bullion, a solvent council and enough trade charters a mint (extends today's `has_mint` charter gate) | stamped coin removes the weighing cost; coin reaches other cities only in purses and chests |

Coin use therefore diffuses outward from mints along real routes over decades;
isolated regions stay in barter longer. The existing `coin_basket` (F7) becomes
a READING of what the purses in a city actually hold, rather than an eased
target.

### 3.7 Mints close on demand (D4)

A mint strikes only when bullion reaches it AND its coin is accepted. When its
own coin's share of the purses in its own city stays below
`MINT_CLOSE_SHARE` for `MINT_CLOSE_YEARS`, the mint closes: the council stops
striking, the city settles in the foreign coin it actually holds, and the
currency is marked closed in the catalogue (its surviving coins still circulate
and wear out). A closed mint may re-charter later through the ordinary gate.
Over centuries this concentrates the world onto a few reserve coins (bezant →
dinar → florin → ducat) — the league table of §5.3.

### 3.8 Every trade is a payment

- **Buying at A**: the merchant's purse (at A — D11) pays A's sellers: local
  merchants, estate owners, or the city (tariff). Coin moves between purses.
- **Selling at B**: B's buyers pay the merchant from THEIR purses. Buyers with
  no coin → B cannot absorb the cargo at that price → the price falls or the
  trade falls back to barter.
- **Returning home**: profit in coin rides home in a CHEST on the return leg, or
  stays at B in the house's purse there (if it has an office), or is sent by a
  bill of exchange (§3.10).
- **Trade deficits drain coin.** A city that keeps importing more than it sells
  sends its silver away: a real bullion famine, coin shortage, falling prices,
  more barter.

### 3.9 The household loop (D12)

- **Wages**: estates, manufactories, mints, yards and fleets pay households for
  labour, from the owner's purse at that hub.
- **Consumption is a purchase**: households buy their ration from the city's
  sellers out of the household purse.
- **Prerequisite (S7's lesson)**: `update_food_and_starvation` must read what
  households could AFFORD (the spending shortfall, `lack_basic`), not raw stock —
  otherwise grain the poor cannot buy reads as the city being fed. This fix
  lands BEFORE paid consumption is dosed.
- The STRUCTURAL ration (`needs_struct`) stays untouched, as N6 requires.

### 3.10 Banks on real reserves

- **Deposits** are coins actually in the vault; **notes** and deposits are claims
  on it. A loan hands out real coins, so reserves really fall; a run is a real
  drain; a failure means depositors lose real coin.
- **Loans are requested, not pushed**: a house applies for a named purpose
  (fleet, estate, mine, war levy, ransom); the bank prices it by the borrower's
  track record, collateral and the local rate, and may REFUSE — and a refusal is
  chronicled. (Builds on `MONEY_MINES_AND_GOODS_PLAN.md` slices 2-3: arrears and
  borrower-share caps, already shipped.)
- **Local interest rates** emerge from vault depth, loan demand and coin trust,
  per city. Historical check: rates falling from ~20% toward ~5% where banking
  matured (Italy 13th-15th c., Amsterdam 17th c.).
- **Bills of exchange**: a merchant pays coin into branch A and is paid at
  branch B — no coin moves. Branches net their balances a few times a year and
  settle the imbalance with a REAL specie shipment (a chest, which can be lost).
  This is why banks exist, and the bills/branches network is the thing to watch.
- **Money changers**: cross-currency payments pay a spread, to a bank where one
  is present, else to local merchants. Fixes F5.

---

## 4. Surfaces (the UI)

### 4.1 The coin catalogue (a new floating window)

Modelled on a numismatic reference book, Victorian engraved treatment (D7).

- **Browse**: a grid of coin cards grouped by currency; filters for metal,
  region, realm, era and status (circulating / closed / debased / reserve coin).
- **Coin card**: obverse and reverse drawn procedurally in the `goodArt.ts`
  ledger style — obverse the city's arms (`CoatOfArms`) or the issuing ruler's
  where a realm dynasty struck it, reverse a denomination motif, metal tint,
  visible wear on old issues, the rim legend in the local language kit.
- **Denomination page**: issue timeline (fineness steps down on debasement, back
  up on reform), weight chart, struck-per-year, circulation map (the basket
  overlay filtered to this coin), exchange value against the strongest reserve
  coin, a one-line biography from `coin_history`.
- **Issue page**: real figures — struck / circulating / hoarded / melted / lost —
  and its problems: debased X% below the previous issue, worn to Y% of standard,
  called in, demonetised, disappearing into hoards.
- **Hoards**: a city sacked (`apply_war_defeat_consequences`) or struck by plague
  buries part of the coin in its purses; the hoard is recorded ("Hoard of Vethra,
  1312: 400 grossi, 12 foreign ducats") and those coins leave circulation.
- **Follow a coin** (D8): a pin on any currency; its mintings, debasements,
  large payments, drains from a city, hoards and mint closure go to the news feed.

### 4.2 The market money band (`CityMarketView`)

- **Money-changer's table**: coins in use here by denomination, their share of
  payments, rate in the local unit of account, the changer's spread. The home
  coin is marked; a dominant foreign coin reads "foreign money rules here".
- **Prices in local money** ("Grain · 3 s 4 d / measure"), or in the commodity
  money in a barter market, with a toggle back to grain-equivalents.
- **Price breakdown** for any good: base value → scarcity (need/stock) → freight
  → tariff → local price level → currency agio, each as a multiplier plus a
  phrase ("scarce: ×1.8 · stock covers 40 days").
- **How today's trade was paid**: gold / silver / petty / bills / BARTER — the
  barter share shows which goods came in and which went out as payment (§3.5).
- **Money health strip**, quiet unless wrong: coin shortage, debased home coin
  (−18%), barter-dominant market, bank branch present.

### 4.3 Money statistics (dashboard; extends `MoneyFinancePanel`)

- **Exchange-rate matrix**: reserve coins as columns, each currency's rate and
  year-on-year change as rows (a Rialto rate sheet).
- **Per currency**: money stock, inflation, price level, seigniorage, gold:silver
  ratio, all over time.
- **Money stock ledger**: struck vs melted vs lost vs hoarded, world and per
  currency — the conservation invariant as a chart.
- **Bullion flows**: trade-balance arrows on the map, silver draining from
  deficit to surplus cities; bullion famines as events.
- **Monetisation map**: the share of each city's trade settled in coin vs barter
  vs bills — the three stages spreading over the centuries.
- **Reserve-coin league table** over centuries (§3.7).

### 4.4 The bank redesign surfaces

Loan book by purpose and borrower, refused applications, per-city interest rate
chart, a live run panel (deposits draining day by day, reserve coverage, the
outcome — rescued / wound down / collapsed — from the existing
`resolve_bank_failure`), branch settlement map (net imbalances and the specie
chests that settle them).

---

## 5. Slices

Each slice: what it builds, its dose, and the gate that is NOT its own target
(§2.4). Routing per §2.8: every slice touching `tick/` runs `cargo test --lib
tick::tests` + `econ_`; every frontend slice runs `npx tsc --noEmit`.

| # | Slice | Dose / effect | Gate |
|---|---|---|---|
| **M0** | ✅ SHIPPED 2026-09-23. **Instrument**: `econ_measure_money_creation` (`#[ignore]`d). Scoped down from a literal per-SITE breakdown (~150 call sites — queued) to per HOLDER CLASS: house wealth / hub treasury / bank reserves. | none (diagnostic) | ran; number on `SCOREBOARD.md` 2026-09-23 |
| **M1** | ✅ SHIPPED 2026-09-23. **Coin data model** (`coinage.rs`): `Currency`/`Denom`/`Issue` recorded from the decisions `decide_coinage` already makes; units of account per culture (§3.4). Currency naming still rides the existing `coin_name` string (deduplicated) — full culture-rooted naming (§3.3's own ask) is real M2 UI-adjacent polish, not done here. | observe only — bit-identical | `sim_fingerprint` unchanged (verified: no folded field touched); `every_currency_name_is_unique_in_a_world` (`tick::tests`) |
| **M2** | ✅ SHIPPED 2026-09-23. **Catalogue surface**: `campaign_get_coin_catalogue` (a pure read of `currencies`/`issues`/`units_of_account`) + a "📜 Catalogue" tab in `MoneyFinancePanel.tsx` — every currency, its denominations, and each denomination's dated issue timeline. **Scoped down**: a functional data listing, not the full Victorian-engraved obverse/reverse card art (D7) or a standalone floating window (§4.1's own design) — both real, separate, unbuilt illustration/layout work. | UI only | `tsc` clean; `cargo check --lib` clean |
| **M3** | ✅ SHIPPED 2026-09-23. **Parallel ledger, mint side**: `Purse`s (§3.1) + the mint-striking transaction (§3.2) — every new `Issue` sizes a real STRUCK quantity from the mint's own throughput and splits it into seigniorage (city treasury purse) / brassage (household purse) / circulation (local-merchant purse). Additive — no existing `wealth`/`treasury` `+=` site is touched or mirrored; this is a genuinely separate ledger computed alongside them, per D10. **Scoped down**: real bullion CARGO (mined, shipped, sometimes lost at sea) is not wired — the struck quantity is sized from the mint's existing regional throughput/bullion-ratio proxy, not a real delivery; melting/loss/hoarding/wear (the sinks) are not implemented, so every issue's `circulating` still equals its `struck` exactly. | observe only — bit-identical (purses are read by nothing else) | `sim_fingerprint` unchanged; `the_coin_ledger_conserves_every_struck_coin` (tests.rs) — Σ purses == Σ struck == Σ circulating per issue, to the float ulp |
| **M4** | ✅ SHIPPED 2026-09-23 (partial). **Money stock ledger** (§4.3's own bullet): `CoinLedgerSummary` — Σ purses by holder class, a snapshot of M3's ledger — served on `CoinCatalogue` and shown as a stat strip atop the Catalogue tab. **Not done**: the market money band in `CityMarketView` (§4.2 — coins-in-use table, prices in local money, the barter/coin split), the exchange-rate matrix and bullion-flow map (§4.3's other bullets), and a TIME SERIES for the ledger (today's snapshot only — no yearly sample is persisted). All real, unbuilt, queued. | UI only | `tsc` clean; `cargo check --lib` clean |
| **M5** | **Barter as a real settlement** (§3.5): goods-for-goods with the payment goods leaving the seller's stock on a return leg; commodity money; `BARTER_SPREAD`. | dosed from 0 | `econ_` per dose step; multi-seed `econ_inheritance_rules_fragment_differently`; new `barter_moves_stock_both_ways`, `barter_is_never_refused`, `money_beats_barter_when_available` |
| **M6** | **Three stages**: ledger houses, weighed bullion, the extended mint charter; coin reaches a city only by purse/chest; mint closure on demand (§3.7). | the charter/closure logic live in the parallel ledger only | new `a_coin_never_reaches_a_city_nothing_trades_with`, `an_unused_mint_closes` |
| **M7** | **Affordability fix**: `update_food_and_starvation` reads the spending shortfall, not raw stock (S7's prerequisite). | dosed from 0 | `unrest_topples_councils` and the famine tests still fire; `econ_expenditure_shares_resemble_a_household` |
| **M8** | **Wages + paid consumption** (§3.9) in the ledger. | parallel, then dosed | household purses stay non-negative; no city starves with grain it could afford |
| **M9** | **The switch-over**: house wealth / treasuries / bank reserves BECOME the ledger (coins + goods + claims), blended from the old numbers by a dose 0 → 1. | the big recalibration | `simulate_decades_reports_dynamics` (bounded, finite, turnover), full `econ_`, multi-seed inheritance gate, re-tuned at each step; SCOREBOARD row per step |
| **M10** | **Banks on real reserves** (§3.10): requested loans, local rates, bills of exchange, branch specie settlement, money-changer spreads. | dosed from 0 | `econ_measure_finance` lifespan and failure rate; interest-rate trend against the historical band |
| **M11** | **Price level into prices**: money scarcity and debasement move `live_price`; regional gold:silver ratios live. | last and riskiest, dosed from 0 | `econ_fidelity_scorecard` (grain CV, price/distance gradient) must not regress; may not ship |

**Stop marker**: M0-M4 is a coherent landing on its own — the diagnosis measured,
coins as real catalogued objects, and a market and dashboard that show money —
with the simulation bit-identical. Everything from M5 changes the economy.

**Build rule for M5-M11**: one dose at a time, the others pinned at zero
(`ACTORS_AND_CARRIAGE_PLAN.md` §5.2). Three doses per session is the ceiling
(`HOUSES_GUILDS_AND_MARKET_PLAN.md` §9).

---

## 6. Risks

- **R1 · The switch-over (M9) moves every number.** House wealth, the wealth
  bound, tiers, realms, wars and banks all rest on the created-from-nothing
  profit (F1). A conserved economy is poorer until the money stock grows, so
  expect deflation and fewer houses at first. Mitigation: the parallel ledger
  runs long enough to show the gap before anything switches, and M9 is dosed.
- **R2 · Deflation spiral.** If bullion supply lags trade growth, prices fall
  and debts grow in real terms. This happened historically (the 15th-century
  bullion famine) and is a feature to show, not to suppress — but the dynamics
  test must not see houses wiped out en masse.
- **R3 · Determinism.** Purses and baskets are maps; every fold iterates in key
  order (the coin-basket bug on `SCOREBOARD.md` already paid for this lesson).
- **R4 · Performance.** Every trade gains a payment. Keep purses sparse and
  flat, resolve the payer/payee indices once per dispatch round (the
  `house_for_indexed` memo pattern, §5.5), and confirm with
  `bench_campaign_tick_large` and `sim_fingerprint`.
- **R5 · The inheritance gate** has been perturbed five times; a coronation
  moves a fortune at once, and now so will a bank failure. Isolate with a
  `suppress_*` flag only if the effect is a confounder, never to hide a real
  break (§8.15).
- **R6 · Households priced out of food** — the reverted S7 walk. M7 before M8,
  no exceptions.

---

## 7. Queue (rule 36 — owed, not waived)

1. **Royal coinage** — a realm crown taking over or closing its cities' mints,
   one royal coin with the ruler's portrait. Waits for M6 (mint closure) and a
   decision on whether it overrides D4.
2. **Counterfeiting and clipping as crimes** with named culprits (a feud or a
   penalty). Waits for M9; excluded now by D5.
3. **Bank notes as money** circulating beyond the issuing bank's branches.
   Waits for M10.
4. **Public debt in coin** — the Monte's coupons paid from real treasury coin,
   bonds as tradeable claims. Waits for M9.
5. **Coin-denominated contracts** — futures struck in a named issue, gaining or
   losing with its value. Waits for M11.
6. **Ransom, tribute and reparations as real specie shipments** (war.rs) rather
   than wealth transfers. Waits for M9.
7. **Temple and palace economies** — non-merchant holders of grain and bullion
   in the ledger-house stage. Waits for M6.

---

## 8. What this plan does not change

The world pipeline, the tile layers, the route matrix and dispatch's choice of
WHAT to ship are untouched. This plan changes how a trade is PAID and what
money IS; which goods move where is still decided by the arbitrage gap until M11
lets money feed back into prices.
