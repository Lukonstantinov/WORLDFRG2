# Money, Mines and Goods — a build plan

> **Status: AGREED IN SCOPE, NOTHING BUILT.** Written from a measurement pass over
> `sim/campaign/tick/money.rs`, `houses.rs`, `colonies.rs`,
> `step8_biological_goods/deposits.rs` and the province query layer. Every claim in
> §1 cites the line it was read off. Companion plan:
> `PLACES_DEMAND_AND_GROWTH_PLAN.md` (settlements, growth, demand). The two are
> independent — this one can land first and should.

This plan is ONE causal chain, which is why four apparently separate complaints are
in one document:

```
banks survive  →  mines get financed  →  ore becomes a real industry
                                      →  deposit goods show up in the province view
```

A mine is the most capital-intensive thing a pre-modern economy builds. It is the
natural first customer for credit, and credit is the thing this model has that
does not work. Fixing mining without fixing banking would mean financing mines
through institutions that fail every five years — measuring a broken instrument
(CLAUDE.md §8.15's own lesson: *before concluding a mechanism is unsound, check
that the world you measured in is not itself the thing that is broken*).

---

## 1 · Measured findings

### F1 · Banks are structurally doomed, not unlucky — four causes

Reported symptom: *"banks fail after 5 years"*. Confirmed, with four independent
causes stacking. The first is a real bug; the other three are calibration.

**F1a — A DEFAULTED NOTE-FUNDED LOAN NEVER RETIRES ITS NOTES.** This is the bug.

`bank_pass` (`money.rs:612-626`) on default:

```rust
writeoff += outstanding;
self.banks[bi].real_estate += outstanding * 0.4;
self.banks[bi].loans[li].outstanding = 0.0;
```

Assets fall by `outstanding` and recover `0.4 * outstanding` as foreclosed
property. But every non-colony loan is funded by ISSUING NOTES (`money.rs:582-584`),
and `notes_issued` is reduced ONLY on the repayment path (`money.rs:624`,
`principal_repaid`). So the liability that funded the loan stays on the balance
sheet **forever**. Net permanent equity destruction: **0.6 × principal per
default**, with no path back.

This is the same class of bug the code's own comment at `money.rs:604` records
finding and fixing for the REPAYMENT path — *"the principal repayment simply
vanished … so equity bled ~amort per loan per month and EVERY bank went insolvent
within a few years"*. The identical hole on the DEFAULT path was never closed.

**F1b — One missed payment destroys the whole loan.** `money.rs:590-598`:

```rust
if self.houses[bh].wealth > pay * 1.2 { ... true } else { false }
```

A borrower whose wealth dips below 1.2× this month's payment defaults on the
**entire outstanding balance**, that month. There is no arrears state, no partial
payment, no grace period and no rescheduling. A merchant house's wealth swings
violently month to month (a lost caravan, a feud flare at stage 3, the progressive
wealth tax, a war levy) — so a perfectly sound borrower is destroyed by a transient
dip. Note the asymmetry: a house gets a **one-year bankruptcy grace period** before
`dissolve_house`; its creditor gets none.

**F1c — Every loan goes to the same borrower.** `bank_maybe_lend`'s
`richest_resident` (`money.rs:754-762`) always returns the single wealthiest
non-owner house homed at the seat. A bank therefore stacks several loans on ONE
counterparty, and F1b means they all default in the same month. There is no
concentration limit anywhere on the book.

**F1d — Loan size and spread cannot survive F1a-c.** `amt = headroom.min(reserves *
0.5)` (`money.rs:750`) — up to **half of reserves in a single loan**, at
`BANK_RESERVE_MULT = 3.0` leverage. Income is `BANK_LOAN_RATE` 1.2%/month against
`BANK_DEPOSIT_RATE` 0.6%/month — a 0.6% net spread. **One default costs roughly 100
months of a performing loan of the same size.**

**F1e — Credit is PUSHED, not demanded, and finances nothing.** `bank_maybe_lend`
rolls a die, picks the richest house, and hands over cash with a `purpose` string —
`"trade"` / `"guild_factory"` / `"treasury"` (`money.rs:766-779`). The purpose is a
**label with no project behind it**: nothing is built with the money, no asset is
created, and repayment is not tied to any venture's income. Already recorded as a
gap in `BANKS_MONEY_AND_CRAFT_PLAN.md` slice 3 ("credit demanded not pushed").

**F1f — Banks may only invest in manufactories.** `bank_maybe_invest`
(`money.rs:820`) filters `e.estate_kind != 6`. A bank can never take a stake in a
mine, quarry, fishery or plantation — which removes the one asset class that
historically defined merchant banking's industrial arm.

> **Encouraging:** `Share.payout` already defines kind **`0 = offtake (extraction
> works, D1/D5 — waits for §4.8)`** (`mod.rs:4679`). The instrument for financing a
> mine against its output is already typed and documented; nothing grants one yet.

---

### F2 · Every gemstone is a QUARRY, so the mining mechanic is unreachable

`estate_kind_for_good` (`mod.rs:2112-2128`) decides mine-vs-quarry by **substring
match on the good's name**, even inside the post-S4 `DIST_DEPOSITS` branch:

```rust
if n.contains("iron") || n.contains("ore") || n.contains("copper") || n.contains("silver")
    || n.contains("gold") || n.contains("coal") || n.contains("tin") || n.contains("lead")
    || n.contains("mercury") { return 2; }   // Mine
return 8;                                     // Quarry — EVERYTHING else
```

So **diamond, ruby, sapphire, emerald, jade, lapis lazuli, turquoise, amethyst,
topaz, garnet, carnelian, amber, marble, salt, bay salt, alum, bog iron** are all
Quarries. A quarry is gated by transport and **never reads `mine_depth`**, so the
entire mining mechanic — `mine_depth`, `MINE_UPGRADE_COST_MULT`, drainage capital,
flooded-body unlocking — is structurally unreachable for every one of them.

A diamond pipe is the archetypal deep mine. The code calls it a stone quarry.

Worse, it is **self-sealing**:
- `campaign_province_potential` marks a flooded body `workable` only if an
  `estate_kind == 2` sits within `MINE_DEPOSIT_SEARCH_KM`
  (`campaign_commands/province.rs:767-775`);
- `maybe_found_mining_colony` counts a site "already served" only for
  `estate_kind == 2` or `is_mining_settlement` (`colonies.rs:1524`).

A flooded gem body can therefore **never** become workable by any code path that
exists.

This substring table's own doc comment already records it going stale once ("the
whole gem split, tin, lead, mercury, alum, lapis, turquoise all silently fell
through to Plantation"). It is stale again.

---

### F3 · An estate is never sited on an ore body, and ore is never chosen

Two independent reasons the ordinary estate path produces no mines.

**F3a — the scorer cannot pick a deposit good.** `maybe_found_estate`
(`houses.rs:764-774`) maximises `base_per_capita × demand_pressure_at`. A deposit
good's per-capita rate is minute by construction (it is placed on a handful of
cells). Wheat, wine and timber win every single time. Note the asymmetry the
existing review already found: `demand_pressure_at` is a SCORE here but a GATE in
`maybe_found_guild_workshop`, whose threshold is measurably unsatisfiable.

**F3b — placement ignores geology entirely.** `houses.rs:792-795`:

```rust
let off = hash01(self.seed, self.tick as u64, parent as u64);
let ex = self.hubs[parent].x + (off - 0.5) * self.world_w * 0.03;
```

A random offset from the parent city. `mine_deposits` is not consulted. Depth and
extent are looked up AFTERWARDS by proximity (`mine_geology_at`, 60 km reach) and
will usually find nothing, so even an accidental mine stands on no ore.

**The only path that sites on real geology is `maybe_found_mining_colony`**
(`colonies.rs:1490`) — and it requires `extent >= EXTENT_GREAT`, caps the world at
`MAX_MINING_SETTLEMENTS = 8`, needs a founder within 2,500 km, and founds one per
call.

**Net: the entire §8.16 ore-geology layer — eleven deposit models, the
belt→district→working hierarchy, per-working grade/extent/depth — is generated at
worldgen and then read by almost nothing in the campaign.** A beautifully modelled
inventory with no industry attached.

---

### F4 · Deposit goods are filtered OUT of the province goods list

`Province.good_belt` is an **unfiltered mean over every cell of the province**
(`provinces.rs:1638-1646`). A good absent from the whole province reads exactly
`8/255 ≈ 0.0314` (bin-0 centre), and both province queries drop anything at or
below `PROV_GOOD_ABSENT_BELT`:

- `campaign_province_goods` — `campaign_commands/province.rs:403`
- `campaign_province_potential` — `campaign_commands/province.rs:877` and `:896`

That floor is **correct for a belt good** (coverage-diluted, area-distributed). It
is **wrong for a deposit**, which is a point, not an area. A diamond district of
1-3 cells inside a 3,000-cell province produces a mean around 0.0318 — it scrapes
past or falls under the floor depending on the PROVINCE'S SIZE. *A larger province
hides a richer mine*, which is backwards.

Meanwhile `ProvincePotential.deposits` (the per-working dot list) is populated from
`metadata["deposits"]` completely independently and correctly. **So the survey plate
can be drawing a diamond working in a province whose goods list says there is no
diamond.** That is the reported symptom.

A second error rides on the same line: `province_good_potential_base`
(`cities.rs:721-728`) is

```rust
belt * prov_cap * province_good_land_share(p, g)
```

`prov_cap` is a **rural carrying capacity in people** and `land_share` is a
forest/arable/pasture split. A mine's output has nothing to do with how many
farmers the province feeds or how much of it is ploughed.

---

### F5 · The manufactured-goods filter is applied in one of four places

`strip_manufactured_from_province_goods` (`sim_commands.rs:1422`) is called ONLY
from `get_province_layer` (`sim_commands.rs:1612`). It is **not** called from:

| path | stripped? |
|---|---|
| `get_province_layer` | ✅ yes |
| `get_provinces` | ❌ no |
| `sim_generate_provinces` | ❌ no |
| `generate_and_persist_provinces` (both run-all paths) | ❌ no |

So immediately after generating a world the Province Panel's fallback list shows
manufactured goods; after closing and reopening it does not. **The symptom is
intermittent by save/reload state, not random** — which is exactly how it was
reported.

The campaign-side reads (`campaign_province_goods`,
`campaign_province_potential`) DO filter correctly, by good id against the spec's
`Distribution`. The leak is strictly the frozen worldgen shortlist.

---

### F6 · Deposit counts make several minerals invisible

`districts = max(gem_deposits * count_num / count_den, 1)`, against a default
`gemDeposits` slider of **6** (`uiStore.ts:511`). Measured over the shipped roster:

| districts on a default world | minerals |
|---|---|
| **1** | ambergris, lapis lazuli |
| **2** | **diamond**, jade, tyrian purple, turquoise |
| **3** | ruby, sapphire, emerald, mercury |
| **4** | tin |
| **6** | **copper**, marble, alum, (gemstones — retired) |
| **9** | gold |
| **12** | amethyst, topaz, silver, lead, bog iron, coal, garnet, carnelian, bay salt |
| **30-36** | iron, salt |

Two districts on a whole world is why diamond reads as absent: it touches ~2 of
~900 provinces, and if neither falls in a settled catchment it never enters the
economy at all. Compounded by F4, it is invisible even where it exists.

The table is also **wrong in the other direction**: amethyst, topaz, garnet and
carnelian each get **twice as many districts as copper**, and copper was mined
across Cyprus, Iberia, Anatolia, Oman, the Tyrol, Slovakia and Falun. Gold at 9
outnumbers tin at 4 — defensible for placer gold, but silver at 12 against gold 9
understates how concentrated Europe's great silver camps actually were relative to
alluvial gold's ubiquity.

**A dead field to clean up:** `DepositSpec.min_elev` is **never read** by
`deposits.rs` (grep: one mention, in a comment). Diamond still ships
`min_elev: 0.55` — the exact value CLAUDE.md §8.16 calls *"exactly backwards"*.
Harmless today, a live trap for anyone who re-wires the field.

---

## 2 · Decisions taken

Recorded so a future session does not relitigate them.

| # | Decision | Rationale given |
|---|---|---|
| D1 | Gems and stone must **both** appear as estates, classified by how each was really worked | historical extraction method decides mine-vs-quarry, not the substring table |
| D2 | **Every mine is bank-financed**; houses AND settlements are eager to exploit reachable bodies | the Roman *societas publicanorum* / Venetian *colleganza* pattern |
| D3 | With no bank yet: **self-fund a shallow working, borrow to go deep** | Laurion's early workings were self-funded; deep drainage needed capital |
| D4 | Deposit counts: **historical per-mineral scarcity AND scaling with world area** | unevenness is what makes a trade route worth anything; a big world should still get proportionally more |
| D5 | Banking fix = **notes bug + arrears + diversification** | banks should be able to fail, but only when they have earned it |

---

## 3 · Slices

Each slice names its gate. A slice is not done until its gate exists and has been
verified to FAIL on the unfixed code (CLAUDE.md §8.23b's rule — a gate that passes
either way is worse than none).

Routing per CLAUDE.md §2.8: every slice here touches `sim/campaign/tick/**`, so the
union is `cargo test --lib tick::tests` + `econ_` + the dynamics run (§2.1, §2.5).
Slices 6-7 touch `step8_biological_goods/**` and additionally owe `goods_` (rule 26).

---

### Slice 0 · Split `tick/mod.rs` — a pure move, no behaviour

`sim/campaign/tick/mod.rs` is **9,852 lines holding 959 constants** plus the
residual `impl CampaignSim`. Every slice below adds constants to it. Do the move
first or the file gets worse.

- `tick/consts/` split by theme (`trade.rs`, `demand.rs`, `finance.rs`,
  `mining.rs`, `war.rs`, `realm.rs`, `population.rs`, …), re-exported from
  `consts/mod.rs` so every `use super::*` path is unchanged.
- **`tick/consts/doses.rs`** — every shipped-at-zero dose constant in one file,
  each with its measured walk in its own doc comment. There are at least eleven
  scattered through the tree today (`N1B_OWNERLESS_LOSS_RATE`,
  `PROFESSION_BASKET_DOSE`, `DEMAND_ELASTICITY`, `LEAGUE_BOYCOTT_MAX`,
  `LAND_BULK_PENALTY`, `ORE_CEILING_DOSE`, `HOUSEHOLD_MONETIZATION_DOSE`,
  `N2_BAN_PRICE_RATIO`, `BLOCKADE_STAGING_DOSE`, `PROD_ELASTICITY`,
  `CAPACITY_BIND_DOSE`). A session cannot currently see the dose surface without
  grepping for it, which is how a dose gets walked twice or forgotten.
- Struct definitions → `tick/state.rs`. The day loop `advance()` → `tick/advance.rs`.

**Gate:** `cargo test --lib tick::tests` + `econ_` **bit-identical**. A pure move
that changes a number is not a pure move.

**Do NOT** fold this into a behavioural slice. Mixing a 9k-line move with a
mechanism change makes the diff unreviewable and makes bisecting the next
regression impossible.

---

### Slice 1 · Retire notes on default (the bug)

One-line class of fix, probably the largest single effect in this plan.

On the default branch of `bank_pass`, a note-funded loan must retire the notes it
created, exactly as the repayment branch does. Model the write-off honestly:

- notes-funded loan defaults → `notes_issued -= outstanding` (the credit is
  extinguished, not owed forever), assets fall by `outstanding`, `real_estate +=
  outstanding * BANK_FORECLOSURE_RECOVERY`;
- cash-funded (`purpose == "colony"`) loan defaults → reserves were already spent;
  only the asset write-down applies, which is correct as it stands.

Net equity hit becomes `(1 − recovery) × outstanding` on the note side rather than
`outstanding` + a permanent phantom liability.

**Gate:** `a_defaulted_note_funded_loan_retires_its_notes` — construct a bank with
one note-funded loan, force the borrower insolvent, assert `notes_issued` falls by
the written-off principal and that `equity()` after default equals
`equity_before − (1 − recovery) × outstanding` within EPS. **Verify it fails on
the unfixed code.**

**Companion measurement:** re-run `econ_measure_finance` (the `#[ignore]`d
diagnostic that already exists, `economy_validation.rs:2840`) before and after, and
record mean bank lifespan + failures/century in `docs/SCOREBOARD.md`. This slice's
whole claim is a lifespan number; it must be on the board.

---

### Slice 2 · Arrears — a missed payment is late, not fatal

Add `Loan.arrears_months: u32` (serde-defaulted 0 — old saves load unchanged, rule
7's discipline applied to a campaign struct).

- Borrower can pay in full → as today.
- Borrower can pay **part** → take what it can afford, capitalise the shortfall
  into `outstanding`, `arrears_months += 1`, emit a quiet `"arrears"` event (the
  bank's own chronicle, NOT the world journal — a late payment is not news, the
  same quiet-unless-it-matters discipline the stability gauges use).
- `arrears_months > LOAN_ARREARS_LIMIT` → default as today.
- Any full payment resets `arrears_months` to 0.

`LOAN_ARREARS_LIMIT` should be months, not years — the historical Florentine
practice was a matter of a few payment cycles, not indefinite forbearance. Start at
6 and measure.

**Also fix the affordability test.** `wealth > pay * 1.2` asks whether the borrower
has 1.2× ONE payment in liquid wealth, which for a large house is noise. It should
ask whether the borrower can pay without being pushed under its own bankruptcy
floor (`HOUSE_BANKRUPT`) — a solvency test, not a wealth-level test.

**Gate:** `a_borrower_that_misses_one_payment_is_not_ruined` — a borrower dipping
below affordability for one month keeps its loan and accrues arrears; a borrower
dipping for `LOAN_ARREARS_LIMIT + 1` consecutive months defaults. Both directions,
because a forbearance with no end is as wrong as no forbearance.

---

### Slice 3 · Diversify the book — concentration limit and a wider borrower pool

Three bounded changes to `bank_maybe_lend`:

1. **Concentration limit.** No single borrower may hold more than
   `BANK_MAX_BORROWER_SHARE` of a bank's outstanding book. Reject the loan rather
   than shrinking it — a bank that cannot lend safely should not lend.
2. **Wider pool.** Replace `richest_resident` (always the single richest) with a
   deterministic draw from the top *k* eligible residents, weighted by
   creditworthiness — `stable_growth_years` already exists (`cities.rs:1649`) and
   is exactly a track record. **Do not weight by wealth**: `house_for`'s N4 fix
   records that weighting a draw by `political_power` measurably inverted
   `econ_inheritance_rules_fragment_differently`, because wealth grows the weight
   and the weight grows wealth. Weight by track record, or draw uniformly.
3. **Smaller loans.** `reserves * 0.5` in one loan is reckless at 3× leverage.
   `BANK_MAX_LOAN_FRAC` should be well under the concentration limit so a bank
   naturally builds a book of several loans.

**Gate:** `a_banks_book_is_never_concentrated_in_one_borrower` — run a bank through
many lending opportunities with one dominant resident and assert no borrower
exceeds the share cap.

---

### Slice 4 · Let a bank hold a mine — extend `bank_maybe_invest`, activate offtake

`bank_maybe_invest` currently filters `estate_kind != 6`. Widen it to extraction
works (mine 2, quarry 8) and use the **already-typed** `Share.payout == 0` OFFTAKE
kind (`mod.rs:4679`) rather than the dividend kind.

The distinction matters and is the historical instrument: a dividend is a share of
profit; an **offtake** is a claim on a fraction of the physical OUTPUT at an agreed
price. That is how the Fuggers financed Schwaz silver and Neusohl copper, and how
the *publicani* were paid. It also behaves differently under stress — an offtake
holder is paid in goods before profit exists, so a bank's mining book does not
evaporate the moment the mine has a bad year.

**Gate:** `a_bank_may_take_an_offtake_stake_in_a_mine` plus
`an_offtake_share_pays_in_output_not_profit`.

---

### Slice 5 · Mine and quarry as real, financed, ore-sited estates

The substantive slice. Four parts.

**5a — Classify by extraction method, from the SPEC, not a substring.**
Add `DepositSpec.working: Option<WorkingKind>` (`Shaft` | `Open` | `Placer` |
`Pan`), serde-defaulted to `None` → `deposits::default_working_for(id)`, mirroring
how `DepositSpec.model` already defaults to `default_model_for(id)`. The substring
cascade stays ONLY as the `DIST_UNKNOWN` fallback for a pre-S4 save.

Historical assignment (a working note per mineral, so a future editor can argue
with the reasoning rather than the number):

| Working | Minerals | Why |
|---|---|---|
| **Shaft (Mine, kind 2)** | diamond, ruby, sapphire, emerald, lapis lazuli, jade, silver, copper, tin, lead, mercury, iron, coal, rock salt | kimberlite pipes, Mogok ruby, Sar-i-Sang, Rammelsberg, Kutná Hora, Almadén, Wieliczka — all worked underground, all drainage-limited |
| **Open (Quarry, kind 8)** | marble, building stone, alum, bay salt, amethyst, topaz, garnet, carnelian | Carrara, Tolfa, salt pans, agate/amygdule gravels — worked from surface, transport-limited |
| **Placer (Mine, kind 2, depth always surface)** | gold, stream tin, Golconda diamond gravel, Ratnapura sapphire gravel, turquoise | alluvial working; cheap to start, no drainage, exhausts fast |
| **Pan (Quarry, kind 8)** | bay salt, bog iron | solar/wetland harvest; neither a shaft nor a pit |

Note the two goods appearing twice: `placer_frac` already splits a mineral's
districts between lode and alluvial (`DepositSpec.placer_frac`), so a good's
WORKING should be read per-district, not per-good. That is the honest model — the
same diamond field had Golconda's gravels and Kimberley's pipes.

**5b — A deposit-estate founding path that starts from the ORE.**
Do **not** try to make `maybe_found_estate`'s per-capita scorer pick ore; any
multiplier large enough to win would fight every other good (and CLAUDE.md §2.4:
*never tune a constant without a gate that isn't the target*).

New `maybe_found_extraction_estate`, iterating `mine_deposits` rather than goods:

1. Find unworked bodies within a founder's reach — reusing `catchment_radius_km`
   and `leg_exceeds_range`, the same mode-legal range ordinary trade uses, so a
   mine is never founded somewhere its ore cannot leave.
2. Score by `grade × extent × depth_workability × (price/base_value at the nearest
   market) ÷ haulage cost`. That last term is what makes a rich remote body lose
   to a mediocre coastal one — the correct pre-modern answer, and it reuses
   `good_freight`.
3. The founder is a **house** OR the **seat city** (D2: "houses and settlements are
   eager to exploit goods they can reach"). A city-founded working is held by the
   city, which is the *publicani* case and needs no new ownership concept —
   `owner_house = -1` already means "the city holds it".
4. **Place the estate AT the deposit cell**, not at a hash offset. This is the
   whole point: `mine_depth`/`mine_extent` then resolve correctly through the
   existing `mine_geology_at`.

**5c — Financing (D2/D3).**
- Surface / shallow body → self-funded from the founder's own purse, at **reduced
  output and capped tier**. Always available, so mines exist from year one.
- Deep / flooded body → requires real capital. The founder **demands** a loan
  (`purpose: "mine"`), and this is the first genuinely demanded loan in the
  model — the fix `BANKS_MONEY_AND_CRAFT_PLAN.md` slice 3 asks for, with a real
  project, a real asset and repayment tied to the working's output.
- No bank reachable → the working stays shallow and **can never reach the deep
  body**, exactly per D3. When a bank later exists it can lend and the mine
  deepens through the existing `MINE_UPGRADE_COST_MULT` path.

**Do not gate the shallow working on credit.** Requiring a bank for every mine
would leave the ore layer inert for the first several decades of every campaign
(banks are chartered after guilds at yr 5 and houses at yr 10) and dead forever on
worlds where banking never takes hold.

**5d — Fix the self-sealing checks.** `campaign_province_potential`'s `workable`
test and `maybe_found_mining_colony`'s `served` test must both accept a
**Quarry (8)** as well as a Mine (2) where the body's own working kind is an open
pit. Today both hard-code `estate_kind == 2`.

**Gates:**
- `a_gem_body_is_worked_as_a_mine_not_a_quarry` (5a — assert diamond/ruby classify
  Shaft and marble/alum classify Open, off the SPEC, not the name)
- `an_extraction_estate_is_founded_on_its_ore_body` (5b — assert the founded
  estate's cell is within one cell of the deposit, and that `mine_depth`/
  `mine_extent` resolved to the body's real values, not defaults)
- `a_deep_body_needs_credit_and_a_shallow_one_does_not` (5c — both directions; a
  world with no banks must still produce shallow workings, and must NOT produce
  deep ones)
- `a_rich_remote_body_loses_to_a_poorer_reachable_one` (5b's haulage term — the
  gate that is not the target, per §2.4)

---

### Slice 6 · Historical deposit counts, scaled by world area (D4)

Two changes, composed.

**6a — Re-author per-mineral counts** against the real pattern. Proposed table
(`count_num`/`count_den` against the `gemDeposits = 6` default, with the resulting
district count and a one-line justification each):

| Mineral | now | proposed | districts @6 | why |
|---|---|---|---|---|
| iron | 5/1 | 5/1 | 30 | genuinely ubiquitous; unchanged |
| rock salt | 6/1 | 5/1 | 30 | ubiquitous but slightly under iron |
| bay_salt | 2/1 | 3/1 | 18 | every warm shallow coast makes it |
| copper | 1/1 | 3/1 | 18 | **raised 3×** — Cyprus, Rio Tinto, Anatolia, Oman, Tyrol, Falun; the most under-placed mineral in the roster |
| marble/stone | 1/1 | 3/1 | 18 | building stone is quarried locally everywhere |
| coal | 2/1 | 2/1 | 12 | regionally concentrated but not rare |
| bog_iron | 2/1 | 2/1 | 12 | any N-European wetland |
| lead | 2/1 | 2/1 | 12 | usually argentiferous, follows silver |
| gold | 3/2 | 3/2 | 9 | alluvial gold is widespread; unchanged |
| silver | 2/1 | 3/2 | 9 | **lowered** — great silver camps were fewer than the current 12 implies |
| garnet | 2/1 | 3/2 | 9 | common orogenic gem gravel, but not commoner than copper |
| carnelian | 2/1 | 1/1 | 6 | Khambhat-style basalt amygdules; regional |
| amethyst | 2/1 | 1/1 | 6 | **lowered from 12** — a semi-precious gem outnumbering copper 2:1 is backwards |
| topaz | 2/1 | 1/1 | 6 | same |
| alum | 1/1 | 1/2 | 3 | Tolfa was near-monopoly after 1462 |
| tin | 2/3 | 2/3 | 4 | famously two regions (Cornwall, the Erzgebirge); unchanged and correct |
| **diamond** | 1/3 | **1/1** | **6** | **raised 3×** — Golconda AND Borneo AND later Brazil/Kimberley; 2 was invisible |
| ruby | 1/2 | 1/1 | 6 | Mogok, Siam, Ceylon |
| sapphire | 1/2 | 1/1 | 6 | Kashmir, Ceylon, Siam, Montana |
| emerald | 1/2 | 2/3 | 4 | Muzo and Cleopatra's mines — genuinely rarer than ruby |
| jade | 1/3 | 2/3 | 4 | Khotan and Burma |
| mercury | 1/2 | 1/2 | 3 | Almadén + Idrija — correctly a near-monopoly |
| turquoise | 1/3 | 1/2 | 3 | derived from copper; rises with copper anyway |
| tyrian_purple | 1/3 | 1/2 | 3 | a handful of Phoenician dye coasts |
| lapis_lazuli | 1/6 | 1/6 | 1 | **keep at 1** — ONE source for four thousand years is the point of this mineral |
| ambergris | 1/4 | 1/4 | 1 | found, not mined; keep vanishing |

The shape to preserve: **the spread must stay wide.** Cornwall mattered *because*
tin was nowhere else. A table where everything lands between 6 and 12 would be
"visible" and worthless.

**6b — Scale with world area.** District counts should scale with the world's real
LAND area (in km², rule 25) against a reference world, so a Large world gets
proportionally more of everything and the relative pattern is untouched. Today a
Large world has 4× the land and the same 2 diamond districts.

A floor of 1 stays — a mineral must never silently vanish (§8.16's first rule).

**6c — Delete or wire `DepositSpec.min_elev`.** It is read nowhere. Deleting it is
cleaner but is a spec-schema change; leaving a documented `#[deprecated]`-style
comment is acceptable. What is NOT acceptable is leaving diamond carrying the value
§8.16 explicitly calls backwards.

**Gates:** `goods_` (rule 26 — read the per-good table, not just pass/fail),
`no_shipped_mineral_places_nothing` (exists), plus
`deposit_counts_span_an_order_of_magnitude` — assert the most-placed mineral has at
least ~20× the districts of the least, so a future "let's make everything visible"
edit fails loudly. Same shape as `plate_sizes_span_an_order_of_magnitude`.

---

### Slice 7 · The province view tells the truth about goods

Three fixes, all in the query layer, all fixing worlds that already exist.

**7a — A deposit good bypasses the belt-mean floor.** In BOTH
`campaign_province_goods` and `campaign_province_potential`: for a good whose
`Distribution` is `Deposits`, presence is decided by **`workings > 0` in this
province** (which both functions already aggregate, into `agg`), never by
`belt > PROV_GOOD_ABSENT_BELT`. A belt good keeps the floor exactly as it is — the
floor is right for a belt and only wrong for a point.

**7b — A deposit good's potential comes from its workings.** Add a deposit branch
to `province_good_potential_base` that reads `Σ(grade × extent × depth_workability)`
over the province's own workings, instead of `belt × prov_cap × land_share`. Ore
output does not scale with how many farmers the province feeds.

Keep `prov_good_yield_scale` self-calibration applied on top — the same
self-calibrating discipline `need_scale` uses (§8.23b), so the change does not need
a hand-picked constant that would read wrong on a differently-sized world.

**7c — Apply the manufactured filter at every exit.** Call
`strip_manufactured_from_province_goods` in `get_provinces`,
`sim_generate_provinces` and `generate_and_persist_provinces` as well as
`get_province_layer`. Better: apply it ONCE inside
`generate_and_persist_provinces` before persisting, and keep the read-path call as
the fix for worlds already on disk. One writer, one reader, no fourth place to
forget.

**Gates:**
- `a_deposit_good_is_listed_wherever_it_has_a_working` — a province with one
  diamond working must list diamond regardless of its cell count. **Verify it
  fails on the unfixed code** — this is the reported bug and the gate is the
  proof.
- `a_province_never_lists_a_manufactured_good` — assert across every exit path,
  parameterised over the four call sites, not just the one that is fixed today.
- `a_mines_potential_does_not_scale_with_farmland` — two provinces with identical
  workings and very different `prov_cap` must report the same deposit potential.

---

## 4 · Risk register

| Risk | Why it bites | Mitigation |
|---|---|---|
| **Slice 1 makes banks immortal** | removing a permanent equity leak could swing too far; a bank that never fails removes `trigger_regional_crash`'s only trigger and the crash layer goes quiet | measure failures/century with `econ_measure_finance` before/after; the target is *fewer, earned* failures, not zero. If crashes go to 0/century, that is a FINDING to record, not a success |
| **Slice 5 routes large new capital through banks** | D2 puts all mining capital through credit; this is a big new flow into a tuned system | dose the deep-mine financing from zero on the N1/N6 pattern, re-running `econ_` **per dose step**, never per slice (`ACTORS_AND_CARRIAGE_PLAN.md` §5.2's own lesson) |
| **Slice 6 moves the goods layer** | more districts = more production = more trade volume | `goods_` per rule 26 AND `econ_` — the basket price and expenditure shares both read production |
| **`econ_inheritance_rules_fragment_differently`** | this gate has been perturbed five times by unrelated changes and has flipped inside its own noise band | it is now multi-seed (§8.15). Run it per dose step. **Check whether it is already red on the parent commit before blaming this work** — `INSTITUTIONS_BUILD_ORDER.md` records it failing independently |
| **Slice 5b's O(deposits × hubs) scan** | a world can carry thousands of workings | bucket the deposits spatially, as `generate_elevation_volcanic` already does for volcanic density (§4's own rule); never an O(n²) pairwise scan |
| **Slice 0 changes a number** | a "pure move" that isn't | bit-identical assertion is the gate; if it is not bit-identical, the move is wrong, not the gate |

---

## 5 · Deliberately NOT built

Named so a future session does not assume they were silently done.

- **A mining labour market.** Mines will scale with capital and geology, not with a
  contested workforce. Labour is FIX_PLAN Part C and is not a finance change.
- **Mercury → silver amalgamation beyond what slice 4-5 of
  `DEPOSITS_AND_MINING_PLAN.md` already shipped.** Already real; not re-opened.
- **A live share exchange.** `Share.paid` is a price anchor; nothing trades shares
  on a market, and §6 of `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` already says so.
- **Realm coin / a state mint financing mines.** Deferred in
  `REALM_AND_GOVERNMENT_PLAN.md` §7 and left deferred.
- **Ore exhaustion.** v2.0 deliberately made a mine NOT accrue depletion — an ore
  body's grade and extent are a worldgen-frozen geological fact (§5). This plan
  does not reopen that; the `EXTENT_WEAK` decline path (D3) already covers the one
  case where it is right.
- **Player verbs for any of this.** The campaign stays observation-only plus the
  existing province tax verb. `INSTITUTIONS_BUILD_ORDER.md`'s own decision.
- **A second financier class beyond banks and city treasuries.** Temple banking,
  bottomry loans and the *commenda* are all real and all out of scope.

---

## 6 · Build order

```
0  refactor        pure move, bit-identical            ← do first, or it never happens
1  notes bug       one class of fix, largest effect    ← measure with econ_measure_finance
2  arrears         banks stop dying transiently
3  diversify       banks stop dying from concentration
   ── re-measure bank lifespan here. If banks are durable, continue. If not, STOP
      and find the fourth cause before building anything on top of them. ──
4  offtake stake   a bank may hold an extraction work
5  mines           classification → siting → financing  ← dose 5c from zero
6  deposit counts  historical + area-scaled            ← goods_ per rule 26
7  province view   the three display fixes             ← cheapest, most visible
```

Slice 7 is last in dependency order but is the cheapest and fixes the most visible
reported symptom. **If a session has little time, do 7 alone** — it is
self-contained, touches only the query layer, needs no dose walk, and makes the
existing ore layer visible for the first time.

---

## 7 · What to write to the scoreboard

Per CLAUDE.md §2.6, append a row whenever a measured number moves. This plan should
produce at least:

- mean bank lifespan (years) and bank failures/century — before slice 1, after
  slice 1, after slice 3
- financial crashes/century — same three points (see the risk register: a drop to
  zero is a finding)
- mines and quarries founded per century, and the share of them standing on a real
  ore body — before and after slice 5
- districts placed per mineral on a reference world — before and after slice 6
- `econ_` expenditure shares and the basket price/distance gradient — after slices
  5c and 6, the two that move real production

Never edit an old row.
