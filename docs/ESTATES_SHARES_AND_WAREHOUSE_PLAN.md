# Estates as shared property, and the city warehouse

> **APPROVED, NOT YET BUILT.** Nothing in this document is in the code. Every
> claim in §1 was read out of the tree at the time of writing; every design
> decision in §2 was taken by the maintainer in the session that produced this
> file. §6 records what was deliberately left out, and §7 the order.

The campaign has an economy of *quantities* and no economy of *property*. A works
produces, a house grows rich, and at no point is there a thing that several
parties own a piece of and therefore fight over. This plan gives every estate,
mine and manufactory a share table, a condition, a world rank and a chronicle,
and gives the city a warehouse with capacity, spoilage and grades — so that goods
stop being an undifferentiated income stream and become property worth taking.

It is the answer to a measured question: *why does the player never want to
control anything?* Because nothing in the campaign converts a good into a
capability or a holding into a contest. Ownership is the cheapest fix, because
the codebase already half-implements it (F2, F3).

---

## 1. Measured findings (why)

Each was read out of the code, not assumed.

**F1 · The city is already the default owner.** `TickHub.owner_house: i32`
(`tick/mod.rs:1853`) documents `−1` as *"owned by the parent city"*. Estates are
founded by houses (`houses.rs:544`) or by the city, and `EstateRow.owner_is_civic`
already distinguishes them in the query layer. The "settlements own works, houses
acquire them" model is not new — it is the current model, with only one owner.

**F2 · Fractional ownership already exists, once.** `TickHub.stake_bank: i32` +
`stake_share: f32` (`mod.rs:1854-1856`) let a BANK hold an equity stake in a
manufactory and draw a dividend at `RESALE_BANK_STAKE`; the counterpart record is
`BankStake { estate_hub, … }` on the bank. It is single-holder, bank-only and
manufactory-only. A share TABLE is that pair grown a dimension, not a new concept.

**F3 · The works is already a real object.** `estate_kind` u8 (0 none · 1 farm ·
2 mine · 3 plantation · 4 fishery · 5 vineyard · 6 manufactory, `mod.rs:1842`),
`estate_tier` 1..5 (`mod.rs:1845`), `damage: f32` with a funded-repair pass
(`estate_condition_pass`), `estate_effectiveness` scaling realized output, and a
per-hub-per-good `quality` drifting under `update_good_quality`. What it lacks is
plural ownership, a rank, and a panel.

**F4 · Quality is a property of the PRODUCER, not of the STOCK.**
`hubs[h].quality[g]` is one float per hub per good. `hubs[h].stock[g]` is one
float with no grade at all. A warehouse holding 200 fine and 400 coarse is
therefore indistinguishable from 600 mediocre. This blocks two separate features
at once: grades in the warehouse, and any offtake rule that lets a holder take the
better part of a works' output. It is the load-bearing constraint of this plan.

**F5 · Nothing ever spoils in storage.** `GoodSpec.perishable` (`mod.rs:1761`) is
documented as *"extra freight per travel-day from spoilage (additive)"* — it is a
transport surcharge only. City `stock[g]` is uncapped and immortal. A city can
hold forty years of grain at no cost, which removes the entire historical reason
for salting, smoking, granaries and the seasonal grain trade.

**F6 · The city has no warehouse capacity; houses do.** `Warehouse { owner, hub,
capacity, stock, tier, damage }` (`mod.rs:3271`) has five real tiers
(`WH_TIER1_CAP` 600 → `WH_MAX_CAP` 12 000), capacity-scaled upkeep (`CAP_UPKEEP`
× city size), damage and AI expansion at `WH_FULL_FRAC`. None of it applies to a
city. The city's only civic store is `civic_goods` (`mod.rs:2051`), an uncapped
strategic reserve stocked monthly by `council_provision_pass` (`colonies.rs:734`)
— food, plus other goods only when provisioning a colony.

**F7 · The reserve problem is already solved once, for exports.**
`TRADE_RESERVE_MULT` 1.1 and `FOOD_RESERVE_DAYS` 45 (`mod.rs:59-64`) hold back a
food buffer with an explicit comment that trading it away *"strips the seasonal
buffer and triggers a famine death-spiral"*. The pattern exists; it is a flat
constant rather than a policy, and it is invisible to the player.

**F8 · Consumption is daily and invisible.** The populace eats out of the pool
every tick; the only readable output is `lack_basic`/`lack_comfort`/`lack_luxury`,
three floats smoothed `0.9·old + 0.1·new`. There is no monthly release, no
population status, and the populace pays nothing — households are not a money
flow.

**F9 · The market has no sellers.** One pooled `stock[g]` per city, price
`base_value·(need/stock)^0.6`. Who delivered a unit is not recorded anywhere, so
"which houses supply this city" is unanswerable from the data.

**F10 · A crown already inherits its dynasty's assets.** `Realm.treasury` is
documented as *"the house's whole wealth at the coronation. There is no second
pot"*, alongside `debts`, `tax_rates`, and `TaxFarm { house, started_tick, years }`
— a house buying N years of a crown revenue stream for cash now. A works LEASE is
that record pointing at an estate instead of a tithe.

**F11 · Deposits carry `depth` and provinces carry `prov_good_depletion`, and
nothing reads either.** Both were shipped as deliberately non-invasive slices.
Under D9 below they stay that way — this plan does not wire them.

---

## 2. Decisions taken

**D1 · Shares are S3: offtake for extraction, dividend for manufacture.** A share
in a farm, mine, fishery, vineyard or plantation pays in GOODS — the holder
receives its fraction of physical output and takes the better grade first. A share
in a manufactory pays a DIVIDEND in money, which preserves F2's existing bank-stake
behaviour unchanged. Rejected: dividends everywhere (leaves a share an income
stream, so goods stay money and nothing is worth controlling); offtake everywhere
(would rewrite the bank-stake path for no gain).

**D2 · A manufactory still shows everything.** Dividend payment changes only *who
is paid*. Its card carries full production, inputs, buyers, world rank and the
twelve-month curves exactly as an offtake works does.

**D3 · Three grade bands, not five.** `coarse · common · fine`. Stock becomes
`stock[g][band]`. Five bands read more richly and cost 5× the stock storage for
resolution a three-segment bar cannot show anyway.

**D4 · Spoilage does not vary by grade.** One rate per good, modified by climate
and storage quality. A per-grade rate is a second table for a distinction no
reader would notice.

**D5 · The largest holder skims first.** Offtake is quality-ranked: a 40 % holder
takes the top 40 % of output, which is finer than the works' mean. This makes a
controlling share worth more than its fraction, gives "who takes the first
pressing" a real answer, and falls out of the existing per-good `quality` with no
new state.

**D6 · Foreigners buy only through presence.** No presence → no sale. Trading
here → a token share at a premium, essentials excluded. An OFFICE held → ordinary
shares at a foreigner premium. RIGHTS granted (years of conduct, a bailo) → near
local terms, may bid for control. Captor of the council → may take title. A
`foreign_reluctance` term rises with how much of the city's works are already
foreign-held, so a city defends itself and a house pushing past it is doing
something visible and resented.

**D7 · A share purchase is a journey and a negotiation, not a transaction.** An
envoy travels the real route network at `route_days`, may be delayed or lost
(`SEA_LOSS`/`CARAVAN_LOSS`), is checked for standing on arrival, then negotiates
across a few rounds against a seller reluctance. A rival who is closer can
PRE-EMPT the sale while the envoy is at sea — which is the entire argument for
holding offices, expressed as a mechanic rather than a rule.

**D8 · Envoys are chronicled, never drawn.** No map markers. The same events are
written to the city chronicle and to the acting bank's or house's own history.
Markers would be lovely and would clutter a map this plan is not otherwise
touching.

**D9 · Mines do not deplete.** Rejected: wiring `prov_good_depletion` or
`Deposit.depth` into output. A works that wastes is a prize that expires, which
*weakens* the desire to control it; a permanent rich vein takeable only by
purchase, war or intrigue is a stronger object to fight over, and it keeps the
economy stationary over 500 years, which protects the `econ_` bands. Variance
comes from DISASTERS instead (D10).

**D10 · Works are damaged, never destroyed.** Every disaster writes the EXISTING
`damage` field, which `estate_condition_pass` already repairs when funded. No
works is ever removed. This also creates a coordination problem worth watching:
who among the shareholders pays for the repair, and does a holder who refuses get
diluted (D11).

**D11 · Refusing to fund a repair dilutes.** A shareholder who will not pay its
share of a repair has its fraction reduced in favour of those who did. This is the
one mechanism that lets a disaster CHANGE ownership rather than merely dent output.

**D12 · Coronation converts shares to leases.** When a house proclaims a realm
(`House.crowned`, rule 25) title passes to the crown and existing holders are
grandfathered as time-limited operating rights: term, rent to `Realm.treasury`,
offtake or dividend to the lessee. Mirrors `TaxFarm`'s shape exactly. Rejected:
outright expropriation — it would make every house hostile to every realm at the
moment realms are the newest and most fragile system in the campaign.

**D13 · A lease is lost three ways.** Crown REVOCATION (a decision with a stated,
chronicled reason — disloyalty, arrears, a rival's favour bought), WAR (a ceded
province carries its works; a sack damages them), or INTRIGUE (`foreign_hand.rs`
and the crisis machinery turning the crown against a lessee). No silent
revocation, ever.

**D14 · Villages are never share-ownable.** A village is province-level output:
many and small, band 0–1 only (coarse/common), unnamed, aggregated into the
province line, not upgradeable. This is what makes an ESTATE mean something — the
village floor is always there and always plain; quality, ownership and contest
live only in works.

**D15 · Rank is per good; the label leads.** `yield_index = this works' output ÷
world mean output for that good` over WORKS ONLY (villages excluded from the
mean), with a five-step label — marginal · ordinary · notable · great ·
world-class — reusing `Deposit.extent`'s own vocabulary. World rank is
`"4th of 31 copper works"`. A cross-good global rank is meaningless and is not
offered. The label is shown before the number, per the house dossier's own
"pips and a phrase, never a raw 0..1" convention.

**D16 · The works icon is the GOOD's icon, including for manufactories.** Every
works card leads with the icon of the good it PRODUCES, drawn from `GOOD_DEFS`
(`src/goods.ts`) — not a generic kind glyph. `WarehousesPanel`'s current
`KIND_ICON` map (🏬 warehouse, 🌾 farm, ⛏️ mine …) is retained only as a small
secondary marker for the works TYPE. A manufactory therefore shows its output
good's icon (cloth, metalware, liqueur) rather than a shared 🏭, which is the
whole point: two manufactories in one city must be distinguishable at a glance.

**D17 · The warehouse is a SLOT GRID, not a list.** A 6 × 6 grid of cells, one
good per slot, with its icon, amount, grade strip and month delta. One grid per
band (Life · Daily · Luxury). An EMPTY slot is information — a city filling eight
of thirty-six slots is visibly poorer than one filling thirty. Rejected: the
scrolling list — a warehouse is a room with bins, and a grid reads as one at a
glance where a list reads as a spreadsheet.

**D18 · Essentials only for the civic margin.** The city buys and resells FOOD and
preservables and books a margin (the *annona* model, extending
`council_provision_pass`); luxuries and construction goods clear on the open
market with no civic middleman. Rejected: the city as a general merchant — it
routes the whole economy through a new actor and would move every `econ_` number
at once.

**D19 · The civic share in essentials is not alienable.** A captured council must
not be able to sell the city's stake in its own bread supply. Either the share
cannot be sold, or selling it is a deliberate, chronicled scandal with unrest
attached. Left as an explicit choice for the slice that builds it.

**D20 · Sellers are named over ONE pool.** Each delivery is tagged with who
brought it (city · house · guild · local · foreign) and the tags drive the
supplier board. The single pool and the price formula are unchanged. Rejected:
separate books per seller and a full order book — both move every measured number
and neither is needed for the supplier stats this plan actually wants.

---

## 3. Data model

All fields serde-defaulted; every empty case reads as today's behaviour, so old
saves load unchanged and the dynamics test stays bit-identical until the offtake
slice (§4.8) switches on.

```rust
// ── stock gains a grade dimension (D3, F4) ──────────────────────────────
// Replaces `stock: Vec<f32>` with a flat ng × 3 layout. Σ over bands equals
// today's single value, so every existing reader can be given a summing
// accessor and left alone.
pub const GRADE_BANDS: usize = 3;   // 0 coarse · 1 common · 2 fine
stock:      Vec<f32>,               // ng × GRADE_BANDS, flat
stock_age:  Vec<u16>,               // ng × GRADE_BANDS — mean age in days, for spoilage

// ── the share table (D1, F2) ────────────────────────────────────────────
// Supersedes stake_bank/stake_share, which become the migration source:
// an old save with stake_bank ≥ 0 seeds two rows (owner + bank).
pub struct Share {
    pub holder_kind: u8,    // 0 city · 1 house · 2 guild · 3 bank · 4 realm
    pub holder: u32,
    pub frac: f32,          // Σ frac == 1.0, enforced on every mutation
    pub payout: u8,         // 0 offtake · 1 dividend
    pub acquired_tick: u32,
    pub paid: f32,          // what it last traded at — the share-price anchor
}
shares: Vec<Share>,         // empty ⇒ 100 % to whoever `owner_house` names

// ── crown domain (D12, F10) ─────────────────────────────────────────────
pub struct Lease {
    pub lessee_kind: u8, pub lessee: u32,
    pub started_tick: u32, pub years: u32,
    pub rent: f32,
}
lease: Option<Lease>,       // None ⇒ not crown domain

// ── the twelve-month ring behind the cards (§4.6) ────────────────────────
pub struct MonthSample { pub output: f32, pub quality: f32, pub price: f32 }
monthly: [MonthSample; 12],
month_cursor: u8,

// ── city warehouse (D17, F6) ────────────────────────────────────────────
wh_capacity: f32,           // 0 ⇒ uncapped, i.e. today's behaviour
wh_spoiled_month: Vec<f32>, // ng — what rotted, for the panel's top line

// ── supplier attribution (D20, F9) ──────────────────────────────────────
// Accumulated in the tick like `good_flow_accum` already is, never
// recomputed per render.
supply_accum: Vec<f32>,     // ng × 5 seller classes, decaying
```

Derived, never stored: `yield_index`, world rank, cover-in-months, fill fraction.

---

## 4. Slices

Each is independently shippable and states its own gate.

**4.1 · Grade bands.** `stock[g]` → `stock[g][band]`. A summing accessor keeps
every existing reader working. Production writes to the band its producer's
quality falls in; village output is capped to bands 0–1 (D14).
*Gate:* `simulate_decades_reports_dynamics` bit-identical (bands sum to today's
value and nothing yet reads them separately).

**4.2 · Spoilage and city warehouse capacity.** A per-good base rate modified by
climate and storage quality, applied monthly to `stock` and `civic_goods`, tallied
into `wh_spoiled_month`. `wh_capacity` from city size and structures; overflow
spoils fastest. Generalise F7's flat reserve to a per-good cover target in MONTHS,
scaled by connectivity (a well-connected city rationally keeps less).
*Gate:* `cargo test --lib econ_ -- --nocapture` — this moves numbers and is
expected to; record the move in `docs/SCOREBOARD.md`.

**4.3 · The Warehouse panel (D17).** The 6 × 6 slot grid, three band tabs, fill
and spoilage headline, cover in months, grade strips, month deltas, and the
supplier board. Frontend only.
*Gate:* `npx tsc --noEmit` and the eye.

**4.4 · Supplier attribution (D20).** Tag deliveries, accumulate `supply_accum`,
serve it to the panel.
*Gate:* dynamics bit-identical — this only records what already happens.

**4.5 · The share table (D1, D11).** Replace `stake_bank`/`stake_share` with
`shares`, migrating old saves. Payout stays DIVIDEND-only for now, so behaviour is
unchanged; offtake waits for 4.8.
*Gate:* dynamics bit-identical; an old save with a bank stake must produce the
same dividends through the new table.

**4.6 · Works cards, rank and yield index (D15, D16, D2).** The expandable card in
three contexts — settlement view, province view, house dossier — with the good's
own icon leading, the twelve-month output/quality/price curves, buyers, condition,
and the rank line. Each kind surfaces its own true constraint: a farm shows soil, a
vineyard shows grade rising, a manufactory shows input shortfall.
*Gate:* `tsc`; the rank/index computed in Rust as a derived read, no new state.

**4.7 · Disasters and repair (D10, D11).** The table in §4.7a below, all writing
`damage`. Shareholder repair funding, and dilution on refusal.
*Gate:* `econ_` — small but real; disasters remove output.

**4.7a · The disaster table.**

| kind | works | effect | repair |
|---|---|---|---|
| flooding | mine | −60 % output, condition ▼▼ | costly |
| collapse | mine, quarry | −40 %, deaths | moderate |
| firedamp | coal mine | −50 %, deaths, fear | moderate |
| fire | manufactory | −70 % output | moderate |
| blight | vineyard | −50 % output AND grade ▼ | slow, years |
| murrain | pasture estate | −40 % output | slow |
| storm wreck | fishery | −45 %, boats lost | quick |
| drought | farm | −35 % for the season | none — it waits |
| sack, raid | any | output and holdings lost | costly |

**4.8 · Offtake routing (D1, D5).** Physical output splits by share, quality-ranked,
into holders' depots instead of wholly into the city pool.
*Gate:* the big one. `econ_` plus dynamics, with its own `docs/SCOREBOARD.md` row.
This is the slice that moves goods, and it goes late on purpose.

**4.9 · Envoys and negotiation (D7, D8).** Intent → dispatch → travel → standing
check → rounds → outcome (agreed · partial · refused · pre-empted). Chronicled in
the city AND in the acting party's own history.
*Gate:* `econ_`; expected small, since it gates transactions rather than flows.

**4.10 · Coronation, leases, and losing one (D12, D13).** Shares → leases at
`House.crowned`; revocation with a reason; war and intrigue paths reusing
`apply_war_goal`, `strip_holdings_at` and `foreign_hand.rs`.
*Gate:* `econ_`; realms are the youngest system here, so measure before and after.

**4.11 · Monthly release and population status (F8, D18).** Batch daily
consumption into a visible monthly release by population, culture and
wartime/construction demand → content · short · starving. The civic margin on
essentials.
*Gate:* `econ_` — moves consumption timing, expected to move numbers.

**4.12 · Certification and adulteration (A2).** A certifying authority per (city,
good) — guild, city or staple — taking a fee and setting the standard; a
CERTIFIED grade distinct from the true grade; adulteration as a choice with
detection, loss of the mark, and a collapse of the premium.
*Gate:* `econ_` — it moves prices through the grade premium; expected small, but
the fee is a new income stream for guilds and must be checked against rule 18's
compounding lesson.

**4.13 · Toponymic brands (A3).** A works at *great* or above renames its good
after its place; the brand travels with the cargo and carries a premium in distant
markets. Rides §4.6's rank, no new state beyond the name.
*Gate:* `econ_` if the premium is real; `tsc` and the eye if it is presentation
only. Decide which at build time and say so.

> **Amendments to the slices above.** 4.5 now builds TWO instruments, share and
> tenancy (A1), with renewal for the term-limited half. 4.7 takes the recalibrated
> disaster table (A8) and the repair-liability rule (A9). 4.9 gains bills of
> exchange (A5), bank credit-conversion as the primary acquisition route (A6), and
> foreign ownership expressed as a city LAW rather than a scalar (A4). 4.10's
> lease pays a royalty in kind (A7). 4.3 and 4.6 take the heraldic accents (A10).

---

## 5. Risks

**5.1 · Shares compound.** Rule 18's lesson exactly: an uncapped prestige award
took the sustained-richest house from 298k to 1.9M. Share income compounds the
same way and buys more shares. Cap what fraction of a city's works one house may
hold, and check the sustained-richest figure in the dynamics gate after 4.8.

**5.2 · A captured council selling the city's bread.** D19 exists because of this.
If the civic share in essentials is alienable and the AI is allowed to optimise
cash, a captured city will sell its own food supply and starve. Build the guard in
the same slice as the share table, not after.

**5.3 · Grade bands triple stock storage.** ng × 3 floats per hub, plus the age
array. At the campaign's hub counts this is small, but it is on the serialized
autosave path, and `JOURNAL_CAP` exists because that path has OOM'd before.
Measure the blob size before and after 4.1.

**5.4 · Panel cost.** A twelve-month ring per works is cheap; recomputing buyer
shares or world ranks per render is not. Both accumulate in the tick, like
`good_flow_accum` already does.

**5.5 · Offtake starving a city.** Once holders take physical output, a city whose
works are majority foreign-held can be drained of its own production. That is a
GOOD outcome dramatically and a dangerous one numerically — it is the most likely
single cause of a famine cascade in 4.8. Keep the essentials reserve senior to
offtake: the city's cover target is filled before any holder's share ships out.

**5.6 · The rank is only as stable as the mean.** `yield_index` divides by a world
mean that moves as works are founded and damaged. A works can change label without
changing output. Either smooth the mean over years or state plainly in the UI that
the rank is relative.

---

## 6. Deliberately NOT built

- **Mine depletion, and any wiring of `Deposit.depth` or `prov_good_depletion`.**
  Rejected in D9, with reasons. Both stay computed and unread.
- **Permanent destruction of a works.** Rejected in D10 — damage and repair only.
- **Villages as share-ownable property.** Rejected in D14. A manorial model would
  make every province a share table and drown the thing that makes an estate
  special.
- **Five grade bands.** Rejected in D3.
- **Per-grade spoilage rates.** Rejected in D4.
- **Envoys drawn on the map.** Rejected in D8.
- **Outright expropriation at coronation.** Rejected in D12.
- **The city as a general merchant.** Rejected in D18 — essentials only.
- **Separate seller books, and an order book.** Rejected in D20.
- **A traded share MARKET with a live price.** The `paid` field records what a
  share last changed hands at, which is enough for a valuation line. An open
  exchange with bids, asks and a bubble surface is a genuinely interesting
  follow-on and belongs with the speculation machinery, not here.
- **Household spending as a real money flow.** F8 notes the populace pays nothing.
  Making households a money flow would let the model carry real wages and
  cost-of-living, and it is far too large to fold into 4.11 silently.
- **Village → settlement flow for all land goods.** The largest remaining piece of
  the production chain, and the natural sequel to this plan. Not started here.
- **Strategic goods gating capabilities** (timber → fleets, iron → war). Named
  because it is the other half of the desire-to-control problem and this plan
  deliberately answers only the ownership half.
- **Share fragmentation by inheritance.** Considered in 9.3 and not selected;
  recorded there as the strongest candidate for a later addition, ahead of
  anything else in this list.
- **Putting-out (*Verlagssystem*) for manufactories.** See 9.3 — truer than D1's
  dividend, rejected for compatibility with the existing `stake_bank` path.
- **Mine grade drift and rising working cost.** See 9.3 — the available synthesis
  if D9's permanence ever makes the world read as too static.
- **A labour system for works.** See 9.4. The largest historical absence here.
- **A trade calendar** (convoy seasons, fair cycles, Baltic ice) and **women as
  named holders.** See 9.4; both cheap, neither in scope.

---

## 7. Order

`4.1 → 4.2 → 4.3 → 4.4 → 4.5 → 4.6 → 4.13 → 4.7 → 4.9 → 4.8 → 4.12 → 4.10 → 4.11`

Slices 4.1, 4.4, 4.5 and 4.6 should be bit-identical or near it and exist to put
the structure and the views in place before anything moves. 4.2 and 4.7 move
numbers modestly. **4.8 is the slice that moves goods** and is sequenced after the
envoy work so that by the time offtake switches on, every part of the ownership
chain around it has already been proven. 4.10 and 4.11 follow because both depend
on 4.8 having settled.

Note that 4.9 runs BEFORE 4.8 despite being the smaller change: envoys gate who
can acquire a holding at all, and it is better to have acquisition throttled
before acquisition starts changing where goods physically go.

The two amendment slices slot by risk, not by theme. **4.13 (toponymic brands)
runs early** — it rides 4.6's rank, adds no mechanism, and is the cheapest thing
in the document that makes the world feel inhabited. **4.12 (certification and
adulteration) runs late**, after offtake has settled, because it introduces both a
new guild income stream and a new price premium, and stacking either on top of an
unsettled goods flow would make the `econ_` reading unattributable.

---

## 8. Schematics

Not mockups to copy pixel-for-pixel — they fix the INFORMATION each view must
carry and the order it reads in.

### 8.1 · The city warehouse (D17 · slot grid)

Six by six. One good per slot. An empty slot is information.

```
╔═ WAREHOUSE · Thessaly ══════════════════ month 7, 1447 ═══════════╗
║  fill 8,400 / 12,000  ███████████████░░░░░  70% ▲+410             ║
║  spoiled this month  −212   grain −180 · fish −32            ⚠    ║
╠═══════════════════════════════════════════════════════════════════╣
║  [ LIFE ]  DAILY   LUXURY                    cover 6.2 mo  ●●●●○  ║
╠═══════════════════════════════════════════════════════════════════╣
║ ┌──────┬──────┬──────┬──────┬──────┬──────┐                       ║
║ │ 🌾   │ 🧂   │ 🐟   │ 🫒   │ 🧀   │ 🍯   │   each slot:          ║
║ │3,120 │1,840 │  610 │  520 │  340 │  180 │   icon                ║
║ │▲+340 │ ▲+60 │ ▼−90 │ ▲+12 │  ——  │ ▼−20 │   amount              ║
║ │░▓▓▓█ │░▓▓▓█ │░░▓▓█ │░▓▓▓█ │░▓▓██ │░░▓██ │   delta               ║
║ ├──────┼──────┼──────┼──────┼──────┼──────┤   grade strip         ║
║ │ 🍺   │ 🥩   │ 🌰   │ 🫒   │ ⋯    │      │                       ║
║ │  160 │  140 │   95 │   60 │      │      │                       ║
║ │ ▲+8  │ ▼−15 │  ——  │ ▲+4  │      │      │   ← empty slots are   ║
║ │░▓▓▓█ │░░▓▓█ │░▓▓▓█ │░░▓██ │      │      │     the city's poverty║
║ ├──────┼──────┼──────┼──────┼──────┼──────┤     made visible      ║
║ │      │      │      │      │      │      │                       ║
║ └──────┴──────┴──────┴──────┴──────┴──────┘                       ║
║  ⚠ 🐟 stockfish  610 · cover 2.1 mo · BELOW the 3-month floor      ║
╠═══════════════════════════════════════════════════════════════════╣
║  SELECTED · 🌾 barley                                              ║
║  3,120 units · cover 7.1 mo · spoil 0.9 %/mo                      ║
║  ░░░░▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓████    coarse 22 │ common 58 │ fine 20      ║
║  SUPPLIERS   villages 41% ████████░░░░  coarse & common           ║
║              Vetrani  28% █████░░░░░░░  fine        ▲ new         ║
║              Grain gd 19% ███░░░░░░░░░  common                    ║
║              Aegos    12% ██░░░░░░░░░░  common      ▼ −6%         ║
╚═══════════════════════════════════════════════════════════════════╝
```

The grade strip is the panel's most legible element over time — the *annona* year
drawn, with fine draining first and the harvest resetting it:

```
  Jan ░░░▓▓▓▓▓▓▓▓▓▓▓▓▓▓████   Jul ░░░░░░░░░▓▓▓▓▓▓▓▓▓█
  Apr ░░░░░▓▓▓▓▓▓▓▓▓▓▓▓███    Oct ░░▓▓▓▓▓▓▓▓▓▓▓▓▓▓█████
```

### 8.2 · Works cards (D16 · the GOOD's icon leads, manufactories included)

```
 🟠 COPPER WORKING, UPPER VEIN        ⛏ mine · tier 3 · OFFTAKE
    ⭐ 4th of 31 · yield 2.1× · GREAT
    ●●●●○ 0.71 good ore      88/mo ▲+6
    condition ████████░░ sound       upkeep 6.1/mo · net 22.4/mo
    ■Vetrani 42% ■City 38% ■Smiths 12% ■Banco 8%

 🟡 BARLEY FARM OF VALE               🌾 farm · tier 2 · OFFTAKE
    ⭐ 17th of 88 · yield 1.2× · NOTABLE
    ●●●○○ 0.54 sound        412/mo ▼−28
    condition ██████░░░░ worn   ⚠ drought — down this season
    ■City 74% ■Guild of Grain 26%

 🟣 VINEYARD OF KALOS                 🍇 vineyard · tier 4 · OFFTAKE
    ⭐ 2nd of 24 · yield 2.8× · GREAT
    ●●●●● 0.88 first water   61/mo ▲+3
    condition ██████████ sound
    ■Vetrani 55% ■City 30% ■Aegos house 15% (foreign · office held)

 🔵 DYEWORKS OF THE LOWER QUAY        🏭 manufactory · tier 3 · DIVIDEND
    ⭐ 6th of 41 · yield 1.6× · NOTABLE          ← cloth's own icon,
    ●●●○○ 0.66              140/mo ▲+12            never a shared 🏭
    condition ███░░░░░░░ FIRE-DAMAGED ⚠ repair 84 · −70% until mended
    inputs ▸ 🐑 wool 90/mo (Guild) · 🟪 dyes 12/mo (foreign) ⚠ short
    ■City 50% ■Banco di Mare 35% ■Vetrani 15%
    dividend 18.2/mo by share · net margin 13%
```

Two manufactories in one city must be distinguishable at a glance, which a shared
🏭 makes impossible — hence D16. The kind glyph survives as the small secondary
marker on the right.

### 8.3 · The lease ladder (D6, D12)

```
 STAGE 0 · FREE CITY          owner_house = −1
   city holds title · houses may buy in · cash to treasury
        │
 STAGE 1 · PRIVATE CONTROL    a house holds the majority
   city keeps title + ground rent · house directs upgrades, takes grade
        │
 STAGE 2 · CAPTURED           captor_house ≥ 0
   the captor may take title · reversible by sack or purge
        │
 STAGE 3 · CROWN DOMAIN       House.crowned → prov_realm ≥ 0
   title to the realm · shares become LEASES (term · rent · offtake)
   lost by REVOCATION · WAR · INTRIGUE — never silently
```

### 8.4 · The envoy (D7, D8)

```
 INTENT ─► DISPATCH ─► TRAVEL ─► STANDING ─► ROUNDS ─► OUTCOME
             │          route_days     │                 ✓ agreed
             │          delay · LOSS   │                 ~ partial
             │                    office? rights?        ✗ refused
             │                    feud? war?             ⚠ PRE-EMPTED
             └───────────────────────────────────────────  by a nearer rival
```

Travel time is the mechanic, not the flavour: because an envoy takes weeks, a
rival with an office in the city can close first. That is the argument for
presence, expressed as something you watch happen.

### 8.5 · A decade of one works — the system with no scripted content

```
 COPPER WORKING, UPPER VEIN · 1438–1447    ⭐ 4th of 31 · GREAT

 output    ▄▄▅▅▆▆▇▇██▇▇▄▄▂▂▅▅▆▆   ▲ tier 3 1444 · ▼ flooded 1445
 grade     ▅▅▅▅▅▅▆▆▆▆▆▆▆▆▆▆▆▆▆▆   steady — no depletion (D9)
 condition ██████████▉▉▍▍▎▎▋▋██   ▼ flood · ▲ drained 1446
 yield idx 1.7 1.8 1.9 2.3 0.9 2.1    rank 9th→4th→18th→4th
 price     ▃▃▃▄▄▄▄▅▅▅▅▆▆▇▇███▆▆   ▲ spiked while the vein was drowned

 1438  ■■■■■■■■■■  City 100
 1441  ■■■■■■■□□□  City 70 · Vetrani 30            envoy, 2 rounds
 1444  ■■■■■□□□□▨  City 50 · Vetrani 42 · Banco 8
 1445  ■■■■■□□□□▨  flood — Banco refuses the drainage
 1446  ■■■■□□□□□▨  City 44 · Vetrani 48 · Banco 4   ← diluted (D11)
 1447  ■■■■□□□□□▨  City 38 · Vetrani 42 · Smiths 12 · Banco 8

 1445 · The Upper Vein floods. Forty men drowned; the works stands idle.
 1445 · Banco di Mare declines its share of the drainage.
 1446 · House Vetrani drains the vein alone and is enlarged for it.
 1447 · The Guild of Smiths buys in at twelve parts — twice the 1441 price.
```

A disaster, a funding refusal, a dilution, a rank collapse and recovery, and a
rival buying at the top — all emergent from condition, shares, envoys and a yield
index. No scripted events anywhere in it.

---

## 9. Historical critique and amendments

§§1–8 were written as a design. This section is the same design read against the
economic history it claims to model. **The original decisions above are left
standing and unedited**; each amendment names the decision it changes, so the
reasoning stays legible and a future session can see what was traded and why.

### 9.1 · What survived the reading

- **The office ladder (D6) is the Hanseatic and Venetian model.** The *Kontor*
  (Bryggen, the Steelyard, the Novgorod Peterhof), the Venetian *fondaco*, the
  Genoese colonies at Pera and Caffa: foreigners traded from designated compounds
  under restriction. At Novgorod the Hansa could not trade retail, overwinter, or
  partner with Russians. The plan's own vocabulary already borrows *bailo*.
- **Cover-in-months (4.2) is how the offices actually thought.** Rome's *cura
  annonae*, Venice's *Provveditori alle Biade*, Florence's *Abbondanza*, the
  Ottoman *iaşe*. Venice targeted roughly a year's grain.
- **The spoilage figures in §8.1 are period-correct** — grain near 1 %/month,
  salt fish near 3 %. Well-kept granary grain lasted one to three years; the real
  enemies were moisture, pests and heat, which is why the climate modifier earns
  its place. F5 slightly overstates its case: the driver of the grain trade was
  harvest VARIANCE, not storage decay. Soften F5 rather than the mechanic.
- **City ownership of works is real** — communal mills and ovens (*banalités*),
  municipal granaries, the Venetian Arsenal, civic salt monopolies.

### 9.2 · Amendments adopted

**A1 · Two instruments, split by works kind.** *Amends D1 and D14.*
Nobody bought 38 % of a barley farm. Land moved by tenure — freehold, leasehold,
*emphyteusis*, and above all sharecropping (*mezzadria*, *métayage*), where the
landlord took a half or a third of the crop IN KIND. That is the offtake maths
arriving under a different name, with different acquisition and transfer rules.

```
  SHARE   (perpetual, freely traded, divisible)   mine · quarry · fishery
          Kuxen · carati · Bazacle mill shares    · manufactory
  TENANCY (fixed term, renewed or lapsed)         farm · vineyard
          mezzadria · métayage · emphyteusis      · plantation · pasture
```
The maths of payout is shared. What differs: a SHARE is bought and sold; a
TENANCY is GRANTED, runs 5–9 years, and must be RENEWED. Renewal is the point —
a Florentine *compagnia* was re-signed every few years, and re-signing is exactly
when partners quarrelled, refused and split off. **A term that expires generates a
recurring political event that a perpetual holding never can.**

**A2 · Grade is certified by an authority, and can be faked.** *Amends D3;
extends D20.*
Grade was never measured, it was DECLARED: the *aulnage* seal on English cloth,
wool sorted into marks, stockfish graded by cure, and the London **Garbler** — a
public office existing solely to sift and certify spices, because adulteration was
endemic (wine watered, pepper sanded, wool packed with dirt to make weight).
Two mechanics follow:
- **Whoever grades, profits.** A certifying authority per (city, good) — guild,
  city, or staple — takes a fee and sets the standard. This is the historically
  correct form of guild power, and it is far more interesting than the 12 % equity
  stake D1 currently gives a guild.
- **Certified grade may differ from true grade.** A holder may adulterate for
  profit; detection costs it the MARK, which is worth more than the cargo.

**A3 · A great works confers a toponymic brand.** *New; rides D15.*
Nobody in 1400 could compute a world rank. What existed was reputation attached to
a PLACE: Muzo emeralds, Cordovan leather, Damascus steel, Tyrian purple, Falernian
wine, Sarmatian furs. A works reaching *great* or *world-class* therefore renames
its good after its own place — "Kalos wine", "Upper Vein copper" — and the name
travels with the cargo, commands a premium in distant markets, and enters the
chronicle. Pure flavour off a statistic §4.6 already computes.

**A4 · Foreign ownership is a LAW, not a mood.** *Amends D6.*
Cities barred foreign ownership by statute; the bar lifted by treaty, by conquest,
or by capitulation — not by a merchant being sufficiently well-regarded. Replace
the `foreign_reluctance` scalar with a real law on `TickHub.laws` (the field
exists): repealable, purchasable, extractable at swordpoint. This routes foreign
acquisition through POLITICS, which is where the contest was wanted.

**A5 · The envoy carries consent, not coin.** *Amends D7.*
By the fourteenth century money moved by BILL OF EXCHANGE while agents moved
physically — a Florentine house paid for a Bruges holding with a bill drawn on
Florence and settled at the Lyon fairs. So travel gates the DEAL, and payment
clears through a bank. Where no banking relationship spans both ends, payment
falls back to specie: slower, costlier, and range-limited. This makes banks
structurally necessary to distant acquisition rather than merely useful.

**A6 · A bank acquires by converting credit, not by paying cash.** *Amends D7.*
The model case is Jakob Fugger: he lent to Maximilian and the Habsburgs and took
MINING OUTPUT as security and repayment — Schwaz silver and copper, then Hungarian
copper. The Medici did the same with Tolfa alum. Finance BECAME offtake. So a
bank's primary acquisition route is lend → arrears → conversion to a holding, with
cash purchase secondary. This also wires the plan into the bank system already in
the tree instead of running beside it.

**A7 · The crown takes a royalty in kind.** *Amends D12.*
The crown owned the subsoil (*Bergregal* across the Empire) and took a FRACTION OF
PRODUCTION: the *Bergzehnt*, and in Spanish America the *quinto real*, the royal
fifth of Potosí. D12's cash rent stands as an option, but the default lease
payment is a royalty in kind, which ties a crown's wealth to what its land
actually yields.

**A8 · Disaster recalibration.** *Amends 4.7a.*
- **Flooding should dominate and recur.** Water was THE constraint on pre-modern
  mining — Newcomen's engine exists in 1712 because of it. It is the common case,
  not a rare shock.
- **"Blight" on vineyards is anachronistic** — phylloxera is the 1860s, downy
  mildew 1878. The period killers are FROST (the winter of 1709 destroyed French
  vineyards outright), HAIL, and WAR, since armies cut vines deliberately. Rename
  accordingly.
- **Murrain is very well chosen** — the cattle panzootic of 1315–22 killed around
  60 % of cattle in parts of Europe alongside the Great Famine.
- **Harvest failures came in RUNS.** 1315–17 was three consecutive catastrophic
  years. Independent per-year rolls systematically under-produce clustered
  catastrophe, which is where the real demographic damage lives: a bad year must
  raise the odds of the next.

**A9 · A holding can generate losses, not only fail to pay.** *Amends D11.*
A *compagnia* partner was liable with his whole estate; when Edward III defaulted
in the 1340s the Bardi and Peruzzi did not merely fail, the families were ruined.
D11's dilution is half the mechanism. The other half: a disaster creates a REPAIR
LIABILITY, refusing it costs standing as well as fraction, and for a TENANCY (A1)
persistent failure to maintain voids the tenancy outright. The codebase's existing
"limited liability, capped to owner wealth" convention sets the ceiling.

**A10 · Panels are chronicle-themed with heraldic accents.** *Amends §8.*
The existing `chronicleTheme` tokens are the base — no second visual language
inside the campaign shell. But every shareholder or tenant row carries its
holder's ACTUAL arms colour and shield badge (`houseColor`, `CoatOfArms`), so an
ownership bar reads as heraldry rather than as a chart legend:

```
  ■ House Vetrani     42%   ████████░░░░░░░░░░░░
  ▧ City of Thessaly  38%   ███████░░░░░░░░░░░░░   civic hatch, not a tincture
  ■ Guild of Smiths   12%   ██░░░░░░░░░░░░░░░░░░
  ■ Banco di Mare      8%   █░░░░░░░░░░░░░░░░░░░
     each ■ is the holder's own shield colour, per houseColor()
```
A city is drawn as a HATCH rather than a tincture, so civic ownership is never
mistaken for a family's arms — the same discipline that keeps realm tints from
colliding with house colours.

### 9.3 · Corrections considered and NOT adopted

- **Putting-out (*Verlagssystem*) for manufactories.** The characteristic
  pre-modern manufacturing relationship was the merchant supplying raw wool and
  taking finished cloth — the *Verleger* owned the materials and the output, not
  equity in a workshop. That is offtake, and it is truer than D1's dividend.
  D1 stands anyway, for a stated engineering reason: dividends preserve the
  existing `stake_bank` path unchanged. **Recorded here so the compatibility
  choice is not mistaken for a historical one.**
- **Share fragmentation by inheritance.** *Kuxen* were standardised at 128 per
  mine precisely because generations of partible inheritance split holdings into
  ever-smaller fractions. The machinery exists in the tree (`LineRule`,
  `InheritanceRule`, `divide_estate`, and the documented fact that only `Partible`
  divides), so this would connect the inheritance system to industrial property
  and let a culture's law decide whether its people can hold a works together.
  Considered and not selected this round. **It is the strongest single candidate
  for a later addition** and should be reached for before anything in §6.
- **Mine grade drift under D9.** Potosí kept producing for centuries; what changed
  was that the ore got poorer and working it got costlier. Letting grade drift
  down and cost drift up with cumulative extraction would preserve D9's permanence
  (the prize never expires) while capturing the real arc. D9 stands as written;
  this is recorded as the available synthesis if the world ever reads as too
  static.

### 9.4 · Historical holes named but not filled

- **Labour.** The works in this plan have output, condition and ownership, and no
  workers. Potosí ran on the *mita*, a rotational forced-labour draft; European
  mines on wage labour that was often bound; plantations on slavery. This is the
  largest historical absence in the document, and it carries moral weight that
  should not be skipped silently. Named, not built.
- **The trade calendar.** The Venetian *muda* convoys sailed on fixed dates, the
  Champagne fairs cycled six a year, the Baltic closed with ice, the Levant had a
  sailing season. `run_trade_fairs` already exists to hang this on.
- **Women as holders.** Widows ran firms and Genoese women invested heavily in
  *commenda*. The Kin roster already carries sex and the model already rolls an
  8 % widow regency, so this is nearly free and historically well supported.
