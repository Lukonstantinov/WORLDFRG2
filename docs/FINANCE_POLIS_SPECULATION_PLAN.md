# DLC 3 — Finance, the Polis & Speculation

> **STATUS (June 2026): PLAN / not implemented.** Builds on Parts I–V of
> `REDESIGN_AND_DLC_PLAN.md`. This plan was reconsidered against the *actual*
> `tick.rs` campaign sim, which already simulates merchant houses, charters,
> civic taxes, a banking archetype and succession — so most steps **extend
> existing systems** rather than build from scratch.

The campaign already plays like a world of **independent city-states (poleis)**:
`compute_political` ranks each settlement on its own trade power and draws
influence discs; the dominant **House** (`dominant_seat`, `ARCH_POLITICAL`)
governs its seat city, wins `charters`, and the city levies civic taxes into a
`civic_pool`. There is no nation layer. This DLC leans into that: the polis is
the financial and political unit, and the headline feature is an **emergent,
yearly speculation engine** that explains *why* a given city tips into a bubble.

Decisions locked with the owner (June 2026):

| Decision | Choice |
|---|---|
| Era ceiling | **Early-modern ~1600** — joint-stock, bourses, marine insurance, manias are period-correct |
| Player model | **Open/undecided** — every layer is autonomous AI sim first; interaction is a thin optional layer later |
| Build order | **Foundation first** — cheap reuse before new mechanics |
| Speculation engine | **Dynamic**, activates after campaign start, recomputed **once per year**, with a generated causal "why" |

---

## 1. What already exists (the foundation we extend)

All in `src-tauri/src/sim/tick.rs` unless noted.

- **Merchant houses** (`House`, `tick.rs:438`): `wealth`, `prestige`,
  `monopoly: Vec<(good,share)>`, `political_power` (wealth+monopoly+prestige),
  `good_profit`, `archetype`, `charters`, `offices` (kontors), fleets,
  `is_guild`, `dominant_seat`, succession (`head_name/head_since/head_lifespan`),
  cadet **branches** (`HOUSE_BRANCH_WEALTH`), and a permanent `events` chronicle.
- **Archetypes** (`tick.rs:123`): `ARCH_SPECIALTY`, `ARCH_FLEET`,
  `ARCH_BANKING` (`BANK_CREDIT_MULT=1.6`, `BANK_INTEREST=0.01/mo`),
  `ARCH_POLITICAL` (wins city charters, `CHARTER_RENT=1.30`).
- **Civic fiscal seeds**: `EXPORT_TAX_RATE=0.02`, `IMPORT_TAX_RATE=0.03`,
  `GUILD_TAX_MULT`, `ESTATE_TAX_RATE`, all flowing into a city `civic_pool`
  that spends back onto its people; `INFLATION_PER_YEAR=0.015` is a flat
  "coin debasement" applied once a year to every fortune (`tick.rs:974`).
- **Events** (`ActiveEvent`): `embargo` (house feuds), `drought`, `plague`,
  `fishery_collapse`, plus monopoly-rent extraction (`tick.rs:1636`).
- **Market solver** (`market.rs::solve`): stock-based prices in the grain
  numeraire, `RouteMatrix.toll` **already consumed** (`market.rs:292`) but
  stubbed `0`; `HubMetrics{grain_wealth, trade_wealth, currency_goods, …}`.
- **Politics** (`query_commands.rs:2415`): `PoliticalCenter{power, rank,
  radius, stars, monopolies, emporium, …}`; power = 0.26·habitability +
  0.30·pop + 0.30·centrality + 0.14·monopoly, ×sea-access.
- **Yearly cadence**: `TICKS_PER_YEAR=365`; the yearly block at `tick.rs:974`
  is the natural host for the once-a-year speculation pass.

**Gaps to design around:** cultures are cosmetic (`cultures.rs`); there is no
resource depletion; no tradable shares/bourse; no diaspora/migration; dynasties
are single-head succession, not family trees (that is DLC 2 / Part V).

---

## 2. The Polis as a politico-economic agent

Promote the city from a *ranking* (`PoliticalCenter`) to an *actor* by
formalizing what is already implicit — the seat city governed by its dominant
house(s):

| Lever | Extends | New behavior |
|---|---|---|
| **Treasury** | `civic_pool` | accumulates tax; funds fleets/walls/debt |
| **Tariffs / tolls** | `EXPORT/IMPORT_TAX`, `RouteMatrix.toll` | transit tolls on goods passing *through* (Venice effect, #10) |
| **Mint** | `INFLATION_PER_YEAR` | debasement becomes a *council decision* (#4) → money supply |
| **Charters** | `House.charters`, `CHARTER_RENT` | grant a lane/good monopoly to a company (L3) |
| **Council** | `dominant_seat`, `political_power` | the dominant family/faction sets tariff/mint/charter policy |
| **Foreign policy** | influence discs, `offices` | leagues, rivalries, embargoes, tribute |

No nation-state is introduced; politics stays **inter-polis** (leagues,
rivalry, trade war), with the dominant house's council as the decision-maker.

---

## 3. The financial stack (under the polis frame)

- **L0 · Valuation** *(EXISTS, surface)* — company net worth = `wealth` +
  monopoly goodwill (from `monopoly`/`good_profit`) + discounted forward
  earnings. Add a ranked "Companies/Houses" valuation view.
- **L1 · Marine insurance / bottomry** *(extend, M)* — premiums priced off the
  existing voyage-loss model (`SEA_LOSS/CARAVAN_LOSS/RIVER_LOSS`) + world hazard
  fields; underwriting is the natural extension of `ARCH_BANKING`. A bad storm
  year bankrupts over-exposed underwriters → systemic risk.
- **L2 · Banking + bills of exchange** *(extend ARCH_BANKING, M)* — deposit/credit
  network + FX between a city's `currency_goods`; bank failures propagate.
- **L3 · Joint-stock chartered companies** *(build on House, L)* — a `Company`
  pools capital from several houses/cities, holds a polis **charter** over a
  lane, pays **dividends** from trade profit. Reuses charter + office + fleet
  machinery; adds shareholding + dividend distribution.
- **L4 · Polis bourse + tradable shares** *(NEW, L)* — each major polis hosts an
  exchange; share price = f(expected dividend, monopoly value, recent profit,
  sentiment). Cross-polis listing → **capital flight** between rival bourses.
- **L5 · Manias & crashes** *(NEW, grounded, M)* — driven by the speculation
  engine (§4). On trigger, the cornered good's / company's price detaches
  upward, then crashes, damaging over-leveraged houses by their credit exposure
  and triggering capital flight to a rival.

---

## 4. The Speculation "Why-Engine" (the centerpiece)

**Dynamic, once per year, with a generated causal narrative + heatmap.** A new
query/overlay `compute_speculation_risk(year)` (mirrors `compute_political`),
computed at the yearly hook (`tick.rs:974`) and cached between years.

For each polis `P`, score `SpecRisk(P) ∈ [0,1]` as a weighted blend of
normalized drivers — **every input already exists**:

| Driver | Source in code | Bubble logic |
|---|---|---|
| Thin float / corner | `House.monopoly`, `mono50`, `dominant_seat` | one house owns the float |
| Cheap money | mint debasement (#4) + `ARCH_BANKING` presence at `P` | easy credit inflates |
| Leverage | count/wealth of `ARCH_BANKING` seats × `BANK_CREDIT_MULT` | borrowed money chases assets |
| Dividend surge | `good_profit` YoY / L3 dividends | "this can't lose" |
| Price run-up | journal price samples vs `base_value` | self-reinforcing rise |
| Supply shock | active `embargo`/`drought`/`fishery_collapse` on `P`'s goods | price spike |
| Hot capital | diaspora inflow (#18) / office openings | imported speculation |
| Political shock | `House.events` succession/`control_gained` at `P` | regime uncertainty |
| Animal spirits | deterministic `hash01(seed, year, hub)` | the irrational residual |

**Output per polis** (a `SpecCenter`): risk score, `stars` tier, heatmap
intensity (rendered as discs like `PoliticalCenter` + good cell-masks), the
**top at-risk goods**, and a **ranked reason chain** naming the real entities,
plus a `pattern_tag` ("tulip-like", "company-bubble"). Emitted as a
`JournalEntry{kind:"speculation"}` and an overlay payload (toggle under Toolbar).

Sample generated line:
> **Aurelia — HIGH (0.81).** House Verani cornered **amber** (thin float); the
> council cut coin fine (mint), so bankers cheap-lent against **Northern Galley
> Co.** shares whose lane just doubled dividends; Genoese exiles' capital poured
> into the bourse. *Pattern: tulip-like. Watch: amber, Northern Co. shares.*

**As an L5 trigger:** when `SpecRisk(P)` crosses a threshold and animal spirits
spike, fire a `mania` `ActiveEvent` (price detaches for N ticks) followed by a
`crash` that reverts price, hits over-leveraged houses, and pushes capital
flight (#18) toward a rival bourse — all journaled with the same causal text.
The engine works as a **leading indicator before L4/L5 exist**, then becomes
their trigger.

---

## 5. Human-drama layers (wire back into speculation)

- **#15 Dynasties** *(= DLC 2 / Part V, L)* — extend single-head succession into
  family trees; oligarch families control the council; a **succession crisis
  becomes a crash trigger** the why-engine names. Heaviest item; already
  roadmapped.
- **#18 Migration & diasporas** *(NEW, M)* — merchant networks (Genoese/
  Armenian/Hanseatic) and war/famine refugees; diasporas **carry capital
  between bourses**, so a crash in one polis feeds a rival's rise (the L4
  capital-flight loop). Offices/kontors are the existing seed.

---

## 6. Roadmap (foundation first)

| Phase | Content | Effort | Status of inputs |
|---|---|---|---|
| **0** | Formalize **Polis agent** (treasury/tariff/mint/council) on `civic_pool`+`dominant_seat` | M | ~60% implicit |
| **1** | **#1** valuation view · **#10** author tolls · **#9** import cap in `solve` | S | mostly reuse |
| **2** | **#4** mint/debasement as council decision · **#3** bills-of-exchange on `ARCH_BANKING` | M | partial |
| **3** | **Speculation why-engine + yearly heatmap** (predictor) | M | inputs exist |
| **4** | **L3** companies → **L4** polis bourses → **L5** manias (consume Phase 3) | L | new + grounded |
| **5** | **#18** diaspora capital flight · **#15** dynasty trees (DLC 2) | L | new / roadmapped |

Phases 0–3 reuse existing data and already produce the dynamic speculation
drama without new core mechanics; 4–5 are the greenfield builds.

---

## 7. Open questions to settle before implementation

1. **Bourse scope** — one shared world exchange, or a per-polis bourse with
   cross-listing + capital flight (this plan assumes the latter)?
2. **Mint/debasement agency** — autonomous council heuristic only, or exposed as
   a future player lever?
3. **Insurance (L1) placement** — fold into Phase 2 with banking, or its own
   phase? (It has the best ROI: priced entirely off existing hazard fields.)
4. **Dynasty depth** — do full family trees (#15) ship here, or stay in DLC 2
   with only the *succession-crisis → crash* hook surfaced earlier?
5. **Compat** — all new `Company`/share/treasury state rides the append-only
   campaign schema (serde-default), per Part II.4.
