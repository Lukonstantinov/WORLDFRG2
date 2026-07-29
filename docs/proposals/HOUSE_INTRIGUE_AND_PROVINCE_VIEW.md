# Houses & Provinces — audit and design

**Status: DESIGN. Nothing here is built.** Per `CLAUDE.md` §9 this lives in
`docs/proposals/` because it is a menu, not a commitment. Every item carries a
regression gate (§2.4) so it can be commissioned one piece at a time.

Two commissions, answered in order:

1. Audit the house mechanic; design a subwindow surfacing **intrigue** and the
   **stability** of a house, plus whatever else the audit says is missing.
2. Redesign the province view — more information, a **layered** plate for a small
   province, and **control** over it — with historical and campaign-side additions.

---

# Part 1 — The house mechanic as it stands

## 1.1 What is actually built

A `House` (`tick/mod.rs:1730`) is already one of the richest objects in the codebase.
2 787 lines of `tick/houses.rs` drive it. Held state:

| Group | Fields |
|---|---|
| Identity | `name` · `head_name` · `head_since` · `head_lifespan` · `generation` · `founded_tick` · `archetype` (4) · `is_guild` |
| Capital | `wealth` · `prev_wealth` · `worst_loss` · `wealth_history` · `debt_since` |
| Commerce | `spec` · `volume` · `good_profit` · `monopoly` · `mono50` · `mono_ever` · `trade_at` |
| Transport | `fleet_sea` · `fleet_river` · `fleet_caravan` |
| Network | `offices` · `office_leases` · `influence` (per-city 0..1) · `bailos` · `dominant_seat` |
| Politics | `political_power` · `charters` · `rivals` |
| Record | `events` (unpruned family chronicle) |

Around it, systems that already exist and already run:

- **Succession** (`succeed_house`, houses.rs:2250) — the head dies on a lifespan
  timer, an heir is named, the archetype may pivot to reflect what the family became.
- **Capture of a city government** (`update_government`, mod.rs:3137) — per-seat
  bribery or intimidation against `Official{role, name, house, control, kin, term_end}`;
  a majority of control-weighted seats makes the house `captor_house`, which buys a
  favoured-house charter, a tariff tilt, and `CAPTOR_INFLUENCE_BOOST`.
- **Rivalry and feud** (`update_rivalries`, war.rs:413) — shared specialty + shared
  trade component ⇒ rivals; a 15%-per-half-year flare where the weaker house pays.
- **Marriage alliances** (`arrange_marriages`, houses.rs:1777) — a dowry transfer,
  feud cancelled, `alliances` recorded, and a `MARRIAGE_BREAK_CHANCE` collapse back
  into feud. Gated on `houses_in_contact` — grounded in commerce, not geography.
- **Solvency and death** (`update_solvency`, mod.rs:4264) — a private house in the
  red for a full year is dissolved; a guild is bailed out from `civic_pool`.
- **Banks** owned by houses, with a full balance sheet, runs, and failure contagion.
- **Trade wars** — the `barred` list (cities a house is shut out of).
- **Notable figures**, piracy, craft guilds, holy sites, fairs.

This is a lot. The gaps are not "no systems" — they are three specific shapes.

## 1.2 Three findings

**Finding 1 — the family is one atom.**
A house is `head_name: String` plus a wealth scalar. There are no other people in it.
Succession is a formality: a new name, `prestige += 0.05`, an archetype pivot, and
possibly a branch. Nothing is at stake. Historically the succession is precisely the
moment a merchant family dies — the Medici bank survived Cosimo and Piero and was
destroyed under Lorenzo, who was a statesman and not a banker; the Fugger firm's
fortunes tracked Jakob → Anton → the drift after Anton; Italian *fraterna* partnerships
had to be reconstituted at every generation and frequently split the capital.
Cadet branches exist in code but are disabled by standing rule (`ENABLE_CADET_BRANCHES`),
which is fine — the missing thing is not more houses, it is *internal* structure.

**Finding 2 — intrigue exists but has no risk, and no window.**
`update_government` is the intrigue engine and it cannot fail. A house with money and
`influence ≥ GOVT_MIN_INFLUENCE` buys `budget / (weight·BRIBE_COST)` control every year,
deterministically. There is no detection, no scandal, no counter-intelligence, no
consequence for the briber. A bribe is therefore just a slower cash transfer with a
policy coupon attached. The same is true of every other intriguing act: a marriage
never fails, a feud flare is a fixed 15% coin flip, a charter is granted or not.

And almost none of it is visible. Grepping the frontend for `officials` finds exactly
one site — a list inside `HubPanel.tsx:483`. The Houses panel shows wealth, monopolies,
rivals and cities; it never shows *what the family is doing to anyone*.

**Finding 3 — stability is fully determined and completely unreadable.**
The sim knows exactly how close a house is to death — `debt_since` is a literal
12-month countdown — and the UI shows none of it. A player watches a house vanish
between two advances. Every input for a proper stability readout is already held.

## 1.3 What the house mechanic still lacks (beyond the window)

Ranked by leverage:

1. **Failure modes with an agent attached.** Houses die of arithmetic (a year in the
   red). Real ones died of *people*: the Medici's Bruges and London branch managers
   (Portinari, Tani) made unsecured sovereign loans to Charles the Bold and Edward IV
   against Florence's instructions and took the bank down in 1494. The codebase already
   has the right noun — `offices` and `bailos` are branch establishments — they just
   have no staff.
2. **Sovereign lending as a distinct, dangerous asset.** `Loan{borrower_polis}` already
   exists. Bardi and Peruzzi were destroyed by Edward III's 1345 default. A loan to a
   polis at war should be visibly different from a trade loan on the dossier.
3. **Reputation as a separate currency from prestige.** `prestige` is a prize; repute
   is a licence. A house caught bribing, defaulting, or dumping adulterated goods should
   find credit dearer and councils closed. The Hanse had a formal instrument for this —
   *Verhansung*, expulsion from the league — and `barred` is already the mechanism.
4. **Secrets and leverage.** Blackmail is the cheapest intrigue verb and needs almost no
   new machinery: a fact about a house, held by another house, spendable once.
5. **A house that is not a merchant.** Every house trades. Historically the interesting
   ones were the *asymmetric* ones — the Fuggers were a mining-and-lending concern that
   bought an imperial election (543 000 fl. for Charles V in 1519) and took Tyrolean
   copper as security. The `archetype` field is the hook; the political archetype's perk
   is currently just "more political power".

---

# Part 2 — Design · the House Dossier ("The Family Book")

One subwindow, opened from the Houses panel row or from a house's seat on the map.
Follows the existing floating-window convention (`useFloatingWindow`, `PANEL_TINTS`)
and the house's own `color` / `CoatOfArms` for identity.

```
┌─ ⚜️ House Cassii ──────────────────────── Vethra · est. 71 · 3rd gen ── ✕ ─┐
│ [arms]  Marcus Cassii, head since 149 (aged 34 in office)                  │
│         Merchant bankers · 🏦 Banco Cassii · governs Vethra                │
│                                                                            │
│  STANDING           ▁▂▃▅▆▇█  wealth 4 210 gr-eq   ▲ +6%/yr                │
│  ┌────────────┬────────────┬────────────┬────────────┬────────────┐        │
│  │ SOLVENCY   │ LIQUIDITY  │ EXPOSURE   │ SUCCESSION │ COHESION   │        │
│  │  ●●●●○     │  ●●○○○     │  ●●●○○     │  ●○○○○  ⚠ │  ●●●●○     │        │
│  │  secure    │ 4 mo runway│ 2 feuds    │ heir weak  │  loyal     │        │
│  └────────────┴────────────┴────────────┴────────────┴────────────┘        │
│                                                                            │
│ [ Ledger ] [ Intrigues ] [ Network ] [ Family ] [ Chronicle ]              │
└────────────────────────────────────────────────────────────────────────────┘
```

## 2.1 The five stability gauges

The header is the answer to "is this house about to die, and of what?". Four of the
five are **pure derivations of state that already exists** — no new sim, no new save
format, no risk to determinism. Only Cohesion needs new state.

| Gauge | Computed from (all existing) | Reads |
|---|---|---|
| **Solvency** | `wealth` vs. committed liabilities: contract penalties at risk, `Loan.outstanding` where the house borrows, `debt_since` clock | `debt_since > 0` ⇒ a literal countdown: *"in the red 7 of 12 months — bankrupt in 5"*. This is the single highest-value number in the panel and it is currently invisible. |
| **Liquidity** | `wealth` ÷ monthly burn (fleet upkeep + warehouse capacity upkeep + office rents + estate upkeep, all already charged in `apply_wealth_sinks` / `manage_fleets`) | *"4 months of runway"* |
| **Exposure** | Herfindahl over `good_profit` **and** over `active[].influence`; plus `rivals.len()`, `barred.len()`, active wars touching its cities | *"78% of income from one good in one city"* — the concentration that turns one blockade into ruin |
| **Succession** | `tick - head_since` vs. `head_lifespan`, `generation`, **`heir_quality`** (new) | *"the head is late in life; the heir is untested"* |
| **Cohesion** | **new** `cohesion` + factor loyalty (below) | *"the Bruges factor has not remitted in two years"* |

Design rules for the gauges:

- **Never show a raw 0..1.** Each gauge shows five pips and a phrase. The phrase is the
  product; the pips are the comparison across houses.
- **A gauge that is fine is quiet.** Only a gauge below 2 pips takes a warning colour.
  Five permanently-amber dials teach the player to ignore all five.
- **Every gauge is clickable and explains itself** — clicking Solvency opens the Ledger
  tab scrolled to the liabilities block. A dial the player cannot drill into is decoration.

## 2.2 The Intrigues tab

The centre of the commission. Two lists and a board.

```
┌ Intrigues ─────────────────────────────────────────────────────────────┐
│ RUNNING (3)                                                            │
│  🕯 Court the Harbourmaster of Ostrahn        ██████░░░░ 61%   secret  │
│     stake 340 · resolves ~year 156 · opposed by House Verrin           │
│  📜 Petition for the salt charter at Vethra   ███░░░░░░░ 28%   open    │
│  🗡 Undercut House Verrin in cloth            ████████░░ 82%   RUMOURED│
│                                                                        │
│ AGAINST US (1)                                                         │
│  🕯 House Verrin courts our Treasurer         ████░░░░░░ 40%   exposed │
│     ↳ [counter: outbid 220] [denounce to the council]                  │
│                                                                        │
│ LEVERAGE                                                               │
│  📄 House Verrin defaulted on Ostrahn, year 143   (spendable once)     │
│                                                                        │
│ SEATS WE HOLD                                                          │
│  Vethra   Doge ●●●●● kin · Treasurer ●●●●○ · Magistrate ○○○○○ (Verrin) │
│  Ostrahn  Harbourmaster ●●○○○                                          │
└────────────────────────────────────────────────────────────────────────┘
```

Three things this makes true that are not true today:

1. **Bribery becomes visible as a process with a duration**, rather than a yearly
   invisible delta on `Official.control`. `control` is already a 0..1 progress bar;
   the panel just never drew it.
2. **Intrigue becomes contestable.** The "against us" list with counter-verbs is the
   B2 player surface (FIX_PLAN tier 2, "play a house") in its cheapest possible form.
3. **Secrecy becomes a resource.** See the mechanic below.

### The one mechanic that has to be added: exposure

Without it, intrigue is a cash transfer. Proposal — a scheme carries `secrecy`, which
decays monthly against the target's counter-intelligence:

```
detect_pressure = target.political_power
                + 0.5·(city govt stiffness by govt_type)
                + 0.3·(rival influence in the same city)
secrecy -= EXPOSE_RATE · detect_pressure          // monthly
```

At `secrecy ≤ 0` the scheme is **exposed**, and exposure — not failure — is the
interesting branch:

| On exposure | Effect |
|---|---|
| Scandal | a chronicle + `HouseEvent{kind:"scandal"}`; the family record is permanent and unpruned, so it stays on the dossier forever |
| Repute | `repute` falls; higher borrowing rate from banks, councils resist the next suit |
| Feud | the target becomes a rival immediately (reuses `rivals`) |
| Civic penalty | a fine to `civic_pool`, and at severity, expulsion — push the city onto `barred` and drop the office/bailo |
| Counter-opportunity | the exposing house gains `dirt` on the actor — leverage for one future scheme |

This is the Venetian model exactly: the Council of Ten and the *bocche di leone*
denunciation boxes existed because the Republic's entire threat model was patrician
families capturing the state, and it is what made Venetian intrigue *cautious* rather
than merely expensive. It is also what makes the mechanic a *game*: a house must choose
between a fast loud push and a slow quiet one.

### Scheme kinds

Eight, each reusing machinery that already exists:

| Kind | Reuses | Success does |
|---|---|---|
| 0 Court a seat | `update_government` bribery | raises `Official.control` |
| 1 Blackmail | new `dirt` | flips a seat outright, or forces a rival to drop a charter |
| 2 Embargo a rival | `barred`, tariffs | a rival's goods barred from a city the actor governs |
| 3 Marriage suit | `arrange_marriages` | a proposed rather than random alliance |
| 4 Charter suit | `charters` | a monopoly grant at a governed city |
| 5 Smear | `repute` | costs a rival repute and its cheapest seat |
| 6 Run on a rival's bank | `fail_bank`, `trigger_regional_crash` | forces a reserve crisis — already fully modelled |
| 7 Poach a factor | new factors (below) | a rival's branch agent defects with the branch's book |

Only kinds 1 and 7 need genuinely new state; 0, 2, 3, 4, 6 are wrappers over live code.

## 2.3 The Family tab — giving the house people

The minimum internal structure that changes outcomes, not just flavour:

```rust
/// A branch agent — the person who runs an office or bailo in a distant city. The
/// Medici failure mode: an agent with local autonomy, a personal book, and his own
/// judgement about who is good for the money.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Factor {
    pub name: String,
    pub hub: u32,
    /// 0..1 — falls with distance, unpaid share, and rival courting; rises with
    /// kinship and a fair cut of the branch's profit.
    pub loyalty: f32,
    /// 0..1 — how good he actually is. Multiplies the office's trade margin.
    pub skill: f32,
    /// Cumulative amount he has quietly not remitted.
    pub skimmed: f32,
    /// True once he is a family member (kin) — loyal, often less able.
    pub kin: bool,
}
```

At low loyalty a factor **skims** (a slow wealth leak the dossier can name), **defects**
(the office transfers to a rival — scheme kind 7), or **overextends** (books an
unsecured loan to a polis; if that polis defaults, the loss lands on the house). This
is one struct and one monthly pass, and it converts `offices`/`bailos` from a static
list into the thing that actually kills merchant houses.

Two more small additions on the same tab:

- **`heir_quality: f32`**, rolled when the current head takes office, revealed at
  succession. A weak heir means a wealth/volume shock and a cohesion drop; a strong one
  a prestige and margin bonus. This makes the succession timer *suspenseful* rather
  than cosmetic, and it is historically the correct place to put family risk.
- **`cohesion: f32`**, falling with number of distant nodes, a weak head, and unpaid
  factors; rising with marriage alliances and kin appointments. Below a floor the house
  suffers a partition event: capital split, a branch leaves as a new house. That is the
  *fraterna* dissolving, and it is the one honest reason to re-enable branching.

## 2.4 Ledger, Network, Chronicle

- **Ledger** — the missing balance sheet. `HouseLedger` already exists as a command;
  give it liabilities: fleet/warehouse/office/estate upkeep, contract penalty exposure,
  bank loans as borrower, and the sovereign-loan block called out separately with the
  borrower polis's war state beside it (the Bardi/Peruzzi line).
- **Network** — `active[]` is already influence-ranked with roles and a contested flag;
  it wants a map, not a list. Reuse `ProvinceMiniMap`'s SVG approach at world scale:
  seat, offices, bailos, estates, lanes, with rivals' seats greyed behind.
- **Chronicle** — `events` already unpruned; just render it through `YearChronicle`,
  which exists, filtered by kind chips (succession · monopoly · scandal · war · bank).

## 2.5 Cost and gates

| Item | New sim state | Gate |
|---|---|---|
| Five gauges + Ledger/Network/Chronicle tabs | **none** | `npx tsc --noEmit`; dynamics test untouched |
| Scheme board (read-only, over existing `Official.control`) | none | as above |
| `Scheme` + exposure | `schemes`, `repute`, `dirt` — all `#[serde(default)]`, appended last (rule 7) | dynamics test bounded + turnover; `econ_` printed metrics unmoved outside their bands; empty-`schemes` run bit-identical |
| `Factor` | `factors` | same, plus: no factor ⇒ bit-identical |
| `heir_quality` / `cohesion` | 2 f32 | dynamics test: house turnover must not spike — **houses dying is expected and good** (§2.1) but the death *rate* is the regression signal |

Sequence: the whole window first with zero sim change (it is genuinely all already
computable), then exposure, then factors. If only one item is ever built, build the
**Solvency countdown** — the sim already knows, and the player currently cannot.

---

# Part 3 — The province view

## 3.1 What exists

- `ProvincePanel.tsx` (333 lines) — the browser: sort, filter, compare, generate.
- `ProvinceInspector.tsx` (293) — the dossier for one province, opened by a map click.
- `ProvinceMiniMap.tsx` — an SVG footprint from `province_raster` with minimalist
  building glyphs (estate/manufactory/depot/bank/mint) and hover stats.
- `provinceStory.ts` — shared prose helpers, used by both views.
- Data: the frozen `Province` struct (area, Köppen mix, elevation stats, temp, precip,
  aridity, fertility, disease, coast/river/lake cells, goods with world rank, borders
  with feature kind, culture, analog) + a thin live join `ProvinceLive`
  (`rural_pop`, `urban_pop`, `hub_count`, `net_migration`) and `ProvinceDetail`
  (settlements, buildings).

## 3.2 Two problems

**Problem 1 — it is a flat list of twenty-five rows.** The inspector is `<Row k v />`
twenty-five times under five headings. There is no hierarchy (fertility and coastline
length have identical visual weight), no time (nothing shows a trend), no comparison
(is 0.42 fertility good?), and no agency.

**Problem 2 — a province is identical in year 1 and year 500.** Everything except four
live numbers is frozen at worldgen. This is `FIX_PLAN` B1 stated from the UI side: the
richest object the world half produces is a read-only fact sheet. A five-century
campaign leaves it pixel-identical.

Both are worth fixing, and they are the same fix: **give the province mutable state,
then show that state layered and over time.**

## 3.3 Design · the layered plate

The mini-map becomes a **survey plate** — a stack of toggleable layers in the tradition
of an estate map or a geological sheet, which is exactly the idiom §8.12 already
established for biome pattern fills.

```
┌ 🏞 Vaskeld ─────────────────────────────────────────────── province ── ✕ ┐
│ Ashkar people · upland · Dfb · coastal · 41 200 km²                       │
│                                                                           │
│  ┌───────────────────────────────────┐   LAYERS                           │
│  │                                   │   ☑ relief                         │
│  │        [ layered plate ]          │   ☑ water                          │
│  │                                   │   ☑ land use        ← new (B1)     │
│  │                                   │   ☐ tenure          ← new          │
│  │                                   │   ☑ holdings                       │
│  │                                   │   ☑ borders                        │
│  └───────────────────────────────────┘   ☐ routes                         │
│   year 149 ◀━━━━━━━━━━━━━━━●━━▶ 500      [◀ 100y] [today]                │
│                                                                           │
│ [ Land ] [ People ] [ Holdings ] [ Chronicle ]                            │
└───────────────────────────────────────────────────────────────────────────┘
```

Six plates, drawn bottom-up:

| # | Plate | Source | Notes |
|---|---|---|---|
| 1 | **Relief** | elevation via the existing raster | hillshade ground; everything else reads against it |
| 2 | **Water** | rivers by order, lakes, coast, marsh biome | the navigable trunk drawn heavier — it is the province's economic spine |
| 3 | **Land use** | **new** `prov_*` land state | arable · pasture · woodland · waste · irrigated, as §8.12-style pattern fills; **this is the plate that changes over 500 years** |
| 4 | **Tenure** | **new** `prov_tenure` | who holds the land: civic/crown · house (in the house's own `color`) · temple · common |
| 5 | **Holdings** | existing `PBuilding` glyphs | keep the current minimalist SVG vocabulary exactly — it is good |
| 6 | **Borders** | existing `ProvinceBorder.kind` | draw ridge / river / lake frontiers with distinct cartographic symbols instead of the current list-only presentation |

Two rules carried over from the map proper: pattern periods must tile (§8.12), and
labels must go through `drawLabel` (§8.11) — the plate is a small map and should obey
the same registry, so a theme change moves it too.

**The time slider is the point.** A province plate at year 1 and year 500 that differ —
woodland cleared, arable expanded, a marsh drained, a house's tenure block grown — is
the single most legible proof that the campaign and the world are one simulation. It is
also cheap: sample `prov_*` yearly into a small history vector, exactly as
`Bank.history` and `HubSample` already do.

## 3.4 The four tabs

**Land** — the current geography rows, but ranked and contextualised: each figure gets
a percentile against the world (`ProvinceGood.rank`/`of` already does this for goods —
extend the idea to fertility, rainfall, relief). Plus the new land-state block with
trend arrows.

**People** — rural/urban/capacity/migration as they are now, plus culture shares, plus
the strata split, plus **rural unrest** (below). The saturation meter is already the
best element in the current panel; keep it and give it a history sparkline.

**Holdings** — the existing settlement + building list, plus tenure shares, plus which
houses hold estates here and which polis's writ runs here.

**Chronicle** — the province's own year-grouped history through `YearChronicle`
(founded, cleared, famine, revolt, changed hands, plague passed through). Provinces
have no chronicle today and are the natural unit for one — a city chronicle is a
biography, a province chronicle is a history.

## 3.5 What to add, from a historical perspective

The current province is a *statistics bucket*. A pre-modern province is four things it
is not yet:

**(a) A tenure structure.** Who holds the land is the most consequential single
variable in pre-modern economic history — it is most of the difference between the
Low Countries and Neapolitan latifundia, between the Rhineland and the second serfdom
east of the Elbe. Proposal: `prov_tenure: [f32; 4]` — civic/crown · house-noble ·
temple · common. It is four floats, it drives surplus extraction, and it gives the
tenure plate something to draw.

**(b) A fiscal object.** Cities have `treasury` and `civic_pool`; the countryside
contributes nothing. Historically rural extraction — tithe, seigneurial dues, tax
farming, the Ottoman *timar*, the French *taille* — was the fiscal base of every
pre-modern state. Proposal: `prov_tax_rate` + `prov_arrears`, routed to the seat hub's
treasury. This also repairs a real modelling gap flagged by the economy oracle's
framing: city treasuries currently come from tariffs and seigniorage alone, which is
not how any pre-modern polity was financed.

**(c) A supply shed, not merely a migration source.** `province_demography_pass` moves
*people* to cities; nothing moves *grain*. Cities are fed only by inter-hub trade. But
the defining relationship of a pre-modern city is with its own hinterland — Florence's
*contado*, the Roman *annona*, London's grain counties. Proposal: `prov_surplus` lands
in the seat hub's food stock each year, scaled by `prov_rural`, land use, tenure and
the climate anomaly slot A5 wants. **This is FIX_PLAN B1's missing "feedback edge" in
the direction that matters most** — and it makes a bad climate year a food crisis
rather than a dice roll.

**(d) Land that degrades and improves.** Forest cover, soil depletion, irrigation,
cleared land — B1's own list. Deforestation is among the best-documented pre-modern
environmental processes (the Venetian Arsenal's timber reserves, England's coppice
crisis, the Mediterranean's long clearance) and it is what makes five centuries *look*
like five centuries.

**(e) Unrest is rural.** Every major pre-modern revolt was: the Jacquerie (1358), the
English Rising (1381), the German Peasants' War (1525), the Croquants, the Pugachev
rebellion. In this codebase unrest is a city property only. Proposal: `prov_unrest`
driven by saturation (`rural/cap` — already computed and already displayed), tax rate,
tenure concentration and a bad harvest; it suppresses surplus, drives migration, and at
threshold produces a revolt event that a polis must garrison or concede. This is also
the natural consumer for FIX_PLAN B3's now-live `militancy`.

**(f) Provinces should change hands.** Nothing today moves a province between polities
or drifts its culture. `update_wars` does blockade and reparations only — a war changes
a ledger, never a map. Proposal: `prov_holder` (whose writ runs here), contested where
two seats' influence overlaps, transferable at a war's settlement, with slow culture
drift toward the holder's. A war whose outcome is visible on the map is worth an order
of magnitude more than a war whose outcome is a number.

## 3.6 What to add, campaign-wise (control)

Per FIX_PLAN B2 tier 2, the province is where a player's decisions become legible,
because a province is where a decision has a *place*. Six verbs, all decide/apply split
(`fn decide_X(&self) -> XChoice` + `fn apply_X(&mut self, c: XChoice)`), so the AI keeps
supplying the choice until a player takes it:

| Verb | Actor | Reuses |
|---|---|---|
| Set tax / grant relief | the polis whose writ runs here, or its captor house | `PolisChoice` pattern |
| Charter a market town / found a village | polis | `cities.rs` organic founding |
| Clear woodland · drain marsh · build irrigation | polis or house | **the satellite-construction machinery** — `build_stage`/`build_progress`/`build_supply`/`build_convoys` is already a multi-year, supply-fed, decaying project system. Do not rebuild it. |
| Grant land to a house | polis | `create_estate` |
| Garrison / suppress | polis | war levies |
| Build a road to the seat | polis | shortens the derived route-days matrix |

The land-improvement verb is the strongest of the six: it is multi-year, it costs goods
that must be *shipped* (so it feeds the trade sim), it visibly changes the land-use
plate, and its entire implementation already exists for suburbs.

## 3.7 Cost and gates

| Item | Cost | Gate |
|---|---|---|
| Tabs + hierarchy + percentile context | frontend only | `npx tsc --noEmit`; no Rust change |
| Layered plate (relief/water/holdings/borders) | frontend only, extends `ProvinceMiniMap` | as above |
| `prov_*` land state + yearly pass | ~1 struct block, 1 pass, all serde-defaulted | dynamics test **bit-identical** (it seeds no provinces); `province_demography_feeds_cities_and_stays_bounded` passes; `bench_campaign_tick` unchanged within noise |
| `prov_surplus` → seat food stock | the B1 feedback edge | `econ_` scorecard: urbanisation and grain-price bands must not leave their historical ranges — this is exactly what the economy oracle exists to catch |
| Land-use plate + time slider | needs the yearly history vector | visual; describe in prose per §2.2 |
| Control verbs | decide/apply split each | **dynamics test bit-identical with the AI supplying every choice** — that is what proves the refactor was pure (B2's own gate) |

---

# Part 4 — Recommended order

1. **House Dossier with zero sim change.** Five gauges, Ledger, Network, Chronicle, and
   the read-only scheme board over `Official.control`. Everything is already computable;
   this is the largest legibility gain per line in either half of the app.
2. **Province tabs + layered plate, frontend only.** Same argument.
3. **`prov_*` land state + `prov_surplus`.** Closes FIX_PLAN B1's feedback edge, unlocks
   the land-use plate and the time slider, and is an extension of a working pattern
   (`prov_rural`) rather than new architecture.
4. **Exposure + `Scheme`.** Turns intrigue from a cash transfer into a decision.
5. **`Factor`.** Gives houses a human failure mode.
6. **Province control verbs.** The B2 payoff, once there is something in a province
   worth controlling.

Steps 1 and 2 are pure frontend and cannot regress either fidelity oracle. Steps 3–6
each need their gate run before the next begins.

**A note on scope, per §2.4:** three of these items are stability work with no gate of
their own beyond "does not regress" — cohesion, heir quality, repute. Those are the
ones most likely to be tuned toward a spot check and against the aggregate. If they are
commissioned, commission them with the death-rate distribution from
`simulate_decades_reports_dynamics` as the gate, not with a screenshot.
