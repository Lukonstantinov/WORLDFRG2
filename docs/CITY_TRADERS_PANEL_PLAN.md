# The city Traders panel — who trades here, and who is established here

**Status: AGREED (the five design questions are answered below), BACKEND
GROUNDWORK BUILT, UI BUILT — see `ui/campaign/TradersView.tsx`.**

A third tab beside `Market | Flows` on a settlement's Trade view, answering two
questions the app cannot currently answer at all:

* **Who moves cargo through this city** — which houses, which guilds, and how much
  is ordinary local merchants on nobody's account.
* **Who is established here** — offices, bailos, the council seat, possession by
  force — whether or not they carried anything.

They are deliberately two lists rather than one, because they routinely disagree:
a house can seat a city's council and move no cargo at all, and that disagreement
is itself the interesting reading.

---

## 0. The finding this panel exists to surface

Before any UI decision, the number that shapes all of them.
`econ_measure_carrier_mix` (on `main`, `#[ignore]`d) measures who actually
finances shipments:

| | reference world | large world |
|---|---|---|
| financed by a house | **4.0 %** | 4.6 % |
| **ownerless residual** (local merchants) | **96.0 %** | 95.4 % |

**Ninety-six per cent of all shipments move on no house's account.** Any honest
"who trades here" panel will therefore read `local merchants 96 %` on nearly every
city, with named houses as slivers.

That is not a defect in the panel and must not be designed around. It is the
model's current state, and the panel is the instrument that makes it visible —
which is exactly the value: the screen's real message is *"almost nobody's houses
trade here"*, and that is worth knowing. The same diagnostic already breaks down
**why** the residual took each shipment (no house at either end · no free vessel ·
could not afford it · barred), and §4 puts that on screen.

**Rule for this panel: never suppress the residual to make the house list look
better.** Hiding it would turn a true and surprising reading into a flattering
and false one.

---

## 1. The five design decisions

Asked and answered, recorded here so the reasoning is not lost:

| # | Question | Decision |
|---|---|---|
| 1 | Third tab, or fold into Flows? | **Third tab** — `Market │ Flows │ Traders`. Flows stays about GOODS; Traders is about PEOPLE. |
| 2 | Ranked by what? | **User-chosen**, not fixed: volume · standing · route length · carriage type, plus an import/export filter. See §3. |
| 3 | List "local merchants"? | **Yes, and first.** It is the real trading capacity of the city; hiding it hides the finding. |
| 4 | Expose the *why*? | **Yes, but FOLDED AWAY by default** — the panel stays clean and the explanation is there when wanted. |
| 5 | Scope? | **This city only** — a balance sheet of the trade that happened here. This is already what the data is (`trade_last` is filtered to `hub == this city`), so no change was needed. |

---

## 2. Schematic

```
┌ Balyurt · traders ───────────────────────────────────────────────────┐
│  96% of trade here moves on no house's account                       │
│  ┌ carried in ─┐ ┌ carried out ┐ ┌ re-exported ┐ ┌ made here ┐       │
│  │   1,248k    │ │    839k     │ │    212k     │ │  3,760k   │       │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────┘       │
│  consumed here 2,910k · 12 traders · 3 established                   │
│                                                                      │
│  rank ▾ volume  standing  route length  carriage                     │
│  show ▾ all     imports   exports       ⛵ sea    🐫 overland         │
├─ WHO TRADES HERE ────────────────────────────────────────────────────┤
│  ·  local merchants     ████████████████░  96%   ⛵🐫  1,140 km      │
│       silk · cloves · grain · pearls           re-exported 198k      │
│  ⚜ House Vinenacos      █░░░░░░░░░░░░░░░░   3%   ⛵     2,880 km     │
│       🏛 BAILO · seats the council             silk · cloves         │
│  🏛 Clothiers' Guild    ░░░░░░░░░░░░░░░░░   1%   🐫       310 km     │
│       office                                   linen                 │
├─ WHO IS ESTABLISHED HERE ────────────────────────────────────────────┤
│  ⚜ House Vinenacos   🏛 BAILO · seats the council      carries 3%    │
│  ⚜ House Tharrasid   office                            carries 0     │
│  🏛 Clothiers' Guild office                            carries 1%    │
├─ ▸ why 96% moves on no house's account  (world-wide) ────────────────┤
└──────────────────────────────────────────────────────────────────────┘
```

Expanded, the folded note reads:

```
  ▾ why 96% moves on no house's account            (world-wide, this year)
      of 41,300 shipments, 1,650 were financed by a house.
      the rest went ownerless because:
        no house at either end   38,400   (93%)
        no free vessel              820    (2%)
        could not afford it         310    (1%)
        barred from the market       120   (0%)
      These counters are WORLD-WIDE, not this city's — the sim keeps them
      globally, and attributing them to one place would be inventing a
      measurement that was never taken.
```

---

## 3. Ranking and filtering (decision 2)

Four rankings, because they order the same list completely differently and each
answers a different question:

| Rank by | Answers | Notes |
|---|---|---|
| **volume** | who moves the most | the residual dominates — that is the point |
| **standing** | who *matters* here | captor > council seat > bailo > office > merely trades; a house with a seat and no cargo rises to the top |
| **route length** | who trades FAR | volume-weighted mean distance to partners, in km. Separates a local carter from a long-haul house |
| **carriage** | who ships vs who carts | by sea share |

Two independent filters: **direction** (all / imports / exports) and **carriage**
(all / sea / overland).

**One rule.** Sorting must never change what the header totals say. The header is
the city's balance sheet and is invariant; the sort reorders rows only. A filter
DOES narrow the list, and when one is active the header says so explicitly rather
than silently reporting a subtotal as if it were the whole.

---

## 4. "Re-exported", and why it is not called "transit" (decision 3)

The user asked to see "who only transits". The honest answer is that **the sim has
no transit**: a shipment goes from A to B in a single hop.
`TRADE_STAGING_AND_POSTS_PLAN.md`'s own central finding says it plainly — *"the sim
ALREADY moves people in legs; cargo is the only thing that teleports"*. There is no
cargo that passes through a third city, so a "transit" column would be reporting a
quantity that does not exist.

What DOES exist and is real: a trader that lands a good here and ships **the same
good** out of here. Per trader, summed over goods, `min(brought in, sent out)`.
That is genuine entrepôt trade — the city as a stop rather than an origin or a
destination — and it is labelled **re-exported**, not transit, so the column never
claims more than it measures.

When staged voyages are built (that plan's own subject), this column can become
true transit. Until then it must not pretend to be.

---

## 5. What is already built

**Backend groundwork — BUILT, and inert: nothing reads it yet.**

`campaign_trade_flows` now also returns:

* `traders: Vec<CityTrader>` — per carrier: volume, in/out split, sea volume,
  share, re-export, volume-weighted mean route km, top goods, and standing here
  (office / bailo / council seat / captor).
* `established: Vec<CityEstablished>` — every holder with standing here, carrying
  or not, ranked by standing then by volume.
* `carrier_why: CarrierWhy` — the world-wide diagnostic counters behind §4's
  folded note.
* `produced_here` / `consumed_here` — the city's own yearly capacity, from
  `production` and the tick's own `base_need`, so trade volume can be read against
  what this place actually makes and eats rather than in isolation.

Carrier attribution rides state the tick always had and used to discard:
`log_trade` receives each shipment's `sea` flag and its `owner`, and the yearly
fold now keeps both (`TradeFlowAgg::sea_amount` / `::carriers`). Sea share is
attributed **pro rata** from the aggregate row rather than per shipment, because
`sea` is a property of the ROUTE (`coastal_a && coastal_b`) and every shipment in
one row shares it — inventing a per-carrier flag the sim never recorded would be a
fabrication.

**UI — BUILT.** `ui/campaign/TradersView.tsx` is the third `Market │ Flows │
Traders` sub-tab on a settlement's Trade view (wired in `HubPanel.tsx`), reading
the same `campaignTradeFlows` query FlowsView already uses. It opens with the
residual finding (never hidden, §0) plus the four capacity tiles (carried in/out,
re-exported, made here); ranks by volume/standing/route length/carriage and
filters by direction/carriage (§3) without fabricating a per-direction sea split
that the backend does not record; lists WHO TRADES HERE and WHO IS ESTABLISHED
HERE as two separate lists (§2); and folds the world-wide "why" diagnostic away
by default (§4/decision 4), labelled world-wide rather than implying a per-city
attribution the sim never measured.

---

## 6. Gates

* `cargo check --lib --tests` — the query compiles and the new structs serialize.
* `cargo test --lib tick::tests` + `simulate_decades_reports_dynamics` — the
  carrier/sea plumbing is DISPLAY-ONLY: nothing in the tick reads it, so the
  dynamics run must be unchanged. (Met: 164 passed, dynamics ok.)
* `cargo test --lib econ_` — §2.5's gate for any `tick/` change. (Met: 5 passed.)
* `npx tsc --noEmit` / `npx vite build` — the types mirror the Rust structs.
* **A gate this panel needs and does not yet have:** an assertion that the
  traders' shares sum to the city's total trade. A carrier breakdown that quietly
  loses volume would be invisible on screen and wrong in exactly the way this
  panel exists to prevent. Still not written — `campaign_trade_flows` needs a
  `WorldDb`/`State` test harness no existing `campaign_commands` test builds yet;
  by construction each `CityTrader.pct` is `a.vol / trade_total * 100.0` where
  `trade_total = Σ a.vol`, so the shares sum to 100% today, but nothing guards a
  future refactor from breaking that. Left for the session that builds that
  harness rather than adding a one-off fixture here.

---

## 7. Deliberately NOT built

* **True transit / multi-leg voyages.** §4's subject; belongs to
  `TRADE_STAGING_AND_POSTS_PLAN.md`, not here.
* **A river carriage mode.** The sim's travel test is `coastal_a && coastal_b`
  alone, so a river or lake city's trade genuinely reads as overland. Offering a
  third chip would be a lie about the model. Sea-vs-overland is the honest split.
* **Per-city "why" attribution.** The `diag_*` counters are global and per-year.
  Making them per-city needs new per-hub state; until then the note is labelled
  world-wide rather than implying this city's houses declined these shipments.
* **Fixing the 96 %.** This panel MEASURES the carrier mix; it does not change it.
  Whether houses should finance more of the world's trade is an economy question
  with its own gate (`econ_measure_carrier_mix`) and its own plan.
