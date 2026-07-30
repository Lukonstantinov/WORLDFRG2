# The house mechanism — critique, then the master plan

**Status: Phase 0 COMPLETE (0.1–0.4); Phases 1–5 are plan.** Consolidates the design documents
(`HOUSE_PEOPLE_AND_TIERS` · `HOUSE_PEOPLE_PLAN` · `HOUSE_POWER_AND_POLITICS` ·
`HOUSE_SUCCESSION_CRISIS` · `HOUSE_POWER_STRUGGLE_VIEW` ·
`HOUSE_FACTION_NAMING_AND_RECORD`) and reviews them cold before committing to build.

---

# Part 0 — The blocking finding

**A house currently lives about twelve years. The politics layer needs it to live sixty.**

From the last economy scorecard (60-year run): **311.67 dissolutions per century**, ending
with **37 surviving houses**. That is 187 deaths in 60 years against a standing stock of
~37 — a death rate of ~3.1/yr, so a mean firm lifespan of roughly

```
37 houses ÷ 3.1 deaths/yr ≈ 12 years
```

The oracle's reference (Greif; Mueller & Lane on Venetian houses) is **1–3 generations,
30–90 years**. Houses are dying **2.5–7× too fast**, and the scorecard already prints this
as a finding.

Now hold that against what the designs assume:

| Design element | Needs |
|---|---|
| A head with a lifespan and a succession | ~40 years |
| A crisis with 3–5 quarterly rounds | ~1 year |
| A five-year grace after surviving one | 5 years |
| Two crises in one head's tenure | ~15 years |
| A goal with a deadline (e.g. monopoly held 5 yrs) | 5–10 years |
| A schism splitting a line of descent | ≥2 generations |
| Tier 1 earned and then lost | decades |

**A twelve-year house reaches none of these.** It dies before its founder does. Built on
today's turnover, the entire politics layer would fire rarely, read as noise, and be
impossible to tune — and every gate on it ("deposition rate sane", "crisis rate sane")
would be measuring a population that barely exists.

> **This is the one thing that must be fixed before anything else in the series is built.**
> Not because the politics is wrong, but because it has no substrate. It is also the
> cheapest item in the plan to *measure*, and it is already instrumented.

I do not yet know *why* turnover is this high — that is a diagnosis task, and per §2.4 a
diagnosis is a complete task. Candidates worth measuring, in order:
`update_solvency`'s one-year debt window (is a year long enough to claw back?), the
warehouse/fleet upkeep charged at zero wealth, `apply_wealth_sinks`' early-years
surcharge, and whether `maybe_found_house` is spawning marginal houses that were never
viable (which would inflate the death count without any established house failing).

---

# Part 1 — The historian's critique

Six things the design gets wrong or leaves out, with a verdict on each.

### 1.1 There is no inheritance *rule* — and it is the biggest lever there is ⭐ ADD

The design has succession (an heir is named) but no **inheritance system**. Historically
this is the single most consequential rule for whether a house accumulates or fragments:

- **Partible** inheritance (Italian *fraterna*, Islamic law, much of Germany) splits capital
  among heirs at every generation. Firms had to be *reconstituted* each time, and many
  simply didn't survive it.
- **Primogeniture / impartible** (English gentry, some patrician republics) concentrates.
  The famous consequence is a surplus of able, landless younger sons — exactly the
  personnel who staffed offices abroad, joined the church, or led ventures.

This is one enum on the culture, it drives schism probability, the size of the wealth split,
the number of idle able kin, and even how many prominent figures a house has. It is cheaper
than almost anything else in the series and it explains more. **Its absence is the biggest
historical hole in the design.**

### 1.2 Women are absent as agents ⭐ ADD

`cultureFigure` draws male and female. `arrange_marriages` moves dowries. But the `Kin`
roster as designed has no gender, and that quietly loses two very well-documented things:

- **The widow as a capable merchant.** In Italian and Hanseatic practice a widow could hold
  and run the firm; many did, sometimes for decades. This is a *positive* mechanic the
  design badly needs (see 2.2) and it costs almost nothing: a widow becomes head with her
  husband's holdings and her own character.
- **Daughters as the instrument of alliance, and dowry as capital.** The design treats
  marriage as an edge in an alliance list. Historically the dowry was a major capital
  transfer *and* a major liability — dowry inflation was a real fiscal problem for patrician
  families, and cities legislated against it. `arrange_marriages` already moves a dowry;
  it just has no person attached.

### 1.3 The crisis has no civic actor — and that is the most documented part of the subject ⭐ CHANGE

Feuds get arbitration by a council (already shipped). The succession crisis, as designed,
resolves entirely *inside* the family. That inverts the historical record: the commune,
the guild and the council were the usual resolvers. The canonical case is Florence —
Albizzi exile the Medici in 1433, the Medici return in 1434 and exile the Albizzi.
**Exile was the standard instrument**, and the city was the one wielding it.

Change: give the crisis a **civic outcome** alongside the four internal ones — the council
intervenes, sequesters the disputed holdings, or exiles the losing faction from the city.
This also fixes a designer problem: it puts an outside force into a loop that is otherwise
entirely self-referential.

*(Note this makes exile arrive by the back door, after the earlier decision that a deposed
ruler simply dies. Those are compatible: the family kills its own head; the city exiles a
faction. But it needs confirming — see the questions.)*

### 1.4 Bankruptcy has no aftermath ⭐ ADD (small)

A house in the red for a year is "dissolved". Historically a failed merchant faced a
sequence: asset seizure, *cessio bonorum*, being struck from the guild, debtors' prison,
exile — and crucially **his creditors took losses that propagated**. The design already has
banks with loans and contagion, so the propagation half exists. What's missing is that a
failure should be an *event with a tail*, not a deletion. Cheap version: a dissolved house
leaves named creditors who take a recorded loss, and its former kin are barred from office
in that city for a period.

### 1.5 No religion, no patronage, no usury problem ⚠ DEFER (but note it)

`Devout` is a culture trait and nothing reads it. Missing: the confraternity as social glue
and credit network; the family chapel and funded church as the standard prestige purchase
(the Medici and San Lorenzo); and the **usury problem** — a house lending at interest in a
Christian-analogue culture faces a legitimacy cost that an Islamic-analogue house resolves
differently (*mudaraba*, profit-sharing). The design has interest-bearing banks and zero
religious friction.

Verdict: real, and a good later layer, but it is a *third* system on top of an unbuilt
second one. Defer — and note it so it isn't rediscovered as novel.

### 1.6 Plague hits figures, not lineages ⚠ CHANGE (small)

A named figure can die of plague — good, and precisely nameable. But the demographic fact
is bigger: the Black Death ended many patrician lineages outright, and it **concentrated**
wealth in survivors (well documented for post-1348 inheritance). A great plague should be
able to take *several* kin at once and occasionally end a house by extinction rather than
by bankruptcy — a different death, worth distinguishing in the record.

---

# Part 2 — The designer's critique

### 2.1 Twenty numbers for one house ⭐ CHANGE

Count what the player must hold to read one family: tier, standing, five stability gauges,
power % per figure, regard per figure, relation label, four character axes, skill, loyalty,
goal progress, discontent, tension, crisis round, two camp shares, undecided share. That is
~20 quantities, and it grows with the roster.

The shipped stability gauges got this right — **pips and a phrase, quiet when healthy** —
and the politics designs quietly abandoned that discipline. Required correction: **one
headline number per view, everything else a phrase or a bar.**

| View | The ONE number | Everything else |
|---|---|---|
| Houses list | tier | phrases |
| Standing | the solvency countdown | pips + phrases (already correct) |
| Figures | the head's share | relation labels, motives |
| Struggle | the two camp shares | round log in prose |
| Ambitions | goal progress | outcomes as ✓/✗ |

### 2.2 The mechanism only produces decline ⭐ ADD

Inventory of what the design can generate: vices, feuds, skimming, discontent, crises,
deposition, schism, ruin, exile. Inventory of what it can generate that is *good*: a goal
achieved, a tier gained. That is a machine for watching things rot.

The Fugger arc — nothing to a European power in two generations — is more compelling than
any decay, and the design cannot currently tell it. **Add positive events with equal
weight:**

| Event | Trigger |
|---|---|
| **A golden age** | Tier 1 held + funds rising + no crisis, for a decade |
| **A legendary head** | high skill + 3 goals achieved + died in office |
| **A great partnership** | two houses allied and both rise a tier |
| **A dynasty of merchants** | three consecutive heads with rising funds |
| **The house's finest hour** | a peak wealth / peak standing marker kept forever |

These use no new state beyond a marker, and they give the chronicle something other than
obituaries.

### 2.3 The player has nothing to do, so the chronicle *is* the product ⭐ CHANGE

Observation-only was a deliberate call and I think it's right for now — but it changes what
the UI is for. A dashboard exists to support decisions; there are none. So the primary
artefact should be **the chronicle**, with the numbers as annotation, not the reverse.

Concretely: the house dossier's default tab should be its history, not its balance sheet.
The struggle window's round log should read as narrative prose with figures attached, which
the current schematic nearly does already.

### 2.4 Crisis salience — the player cannot watch fourteen houses ⭐ ADD

At ~14 houses and a crisis every ~15 years each, that is roughly **one crisis per year
somewhere**. Everything cannot surface. Salience rule: only **Tier 1–2** crises reach the
news feed; Tier 3–4 crises are recorded but silent, discoverable in the dossier. Same
principle as a healthy gauge staying quiet.

### 2.5 The foreign hand may never fire ⚠ MEASURE BEFORE BUILDING

It requires a conjunction: a rival holds an office/bailo in *our* city, **or** our member
holds a lease in a city that rival *controls*. Both are plausible individually; together
with "and that member is already disaffected" the joint probability could be near zero.

This is the prettiest piece of design in the series and it is the most likely to ship as
dead code. **Instrument it first**: count how often the conjunction exists across a 300-year
run, before writing the mechanism. If it fires less than a handful of times a century,
loosen the channel or cut it.

---

# Part 3 — What I would cut or defer

| Item | Verdict | Why |
|---|---|---|
| Religion / patronage / usury | **Defer** | A third system on an unbuilt second one (1.5) |
| Foreign hand | **Gate on measurement** | May never fire (2.5) |
| Maverick characters (±2 drift) | **Keep, but last** | Flavour on top of an unproven baseline |
| 17 goal kinds | **Cut to ~8 for v1** | Each needs a success test and a chronicle line; ship the ones that reference systems that exist |
| Rupture (full house split by descent) | **Defer behind Departure** | Departure gives 80% of the effect with a fraction of the risk to the wealth distribution |

---

# Part 4 — The master plan

Five phases. **Phase 0 is not optional** — it is the foundation the rest stands on, and it
contains the two open defects already on the scoreboard.

## Phase 0 — Make the foundation sound (no new features)

| # | Step | Gate |
|---|---|---|
> # ▶ HANDOFF · READ THIS FIRST
>
> **This file is the entry point for the house work.** Phase 0 is complete except 0.4.
> Everything below Part 4 is the plan; this block is the live state.
>
> ## What is DONE (on `main`)
>
> | Step | Result |
> |---|---|
> | **0.1 diagnose turnover** | ✅ cause found exactly — the *founding endowment*, not ambition. `wealth: 1.0` + a 2–3 vessel fleet = ~1.4 months of runway at birth → dead at ≈13.4 months. Measured median age at death **1.1 yr**. **73% of dissolutions never traded.** My overextension hypothesis was **refuted**: corr(age, upkeep) = **+0.802**. |
> | **0.2 fix turnover** | ✅ seed capital taken **from the parent guild** (which `maybe_found_house` already requires). Lifespan **~12 yr → 96.9 yr**. |
> | **0.3 fix determinism** | ✅ four hash-order sites fixed (`money.rs`, `production.rs`, `mod.rs`, `colonies.rs`). `econ_scorecard_is_deterministic` **un-ignored** and passing. 166 tests. |
> | **0.4 inheritance rule** | ✅ **DONE.** `sim/shared/inheritance.rs` — a LINE rule and a DIVISION rule per culture, read at `succeed_house`. Gate `econ_inheritance_rules_fragment_differently` passes: partible **18 divisions / 22 co-heirs / 88 houses ever**, the three concentrating rules **0 divisions**, mean wealth per house 120k vs 195k. It also carries the **succession LINE** — a permanent per-head record (name · sex · age at accession and death · wealth at each end · how they came in · an epithet earned at death). |
>
> ## What Phase 0.4 changed, beyond the rule itself
>
> Three findings, all in `docs/SCOREBOARD.md`:
>
> 1. **An heir is not a newborn.** Every head used to be handed a fresh 45–75-year
>    *lifespan* as their tenure. They now inherit at an age their culture's rule implies
>    and rule for what remains of a life — which is the whole reason ultimogeniture (a
>    young heir, long weak-opening tenures) and seniority (an elected elder, short ones)
>    behave differently without any extra mechanism.
> 2. **The reference world was not reproducing campaign start.** `tests::sim()`'s
>    placeholder gave seeded heads a 274-year lifespan, so no house in the 60-year
>    fidelity run ever reached a succession. Every turnover-dependent number was
>    measuring a world of immortal merchants. Fixed in `calibrate_like_campaign_start`.
> 3. **Two defects surfaced and were fixed**: a house's chronicle cap was evicting its
>    own founding and successions under feud spam (milestones are now never pruned by
>    chatter — and per 2.3 the chronicle IS the product); and cadet branches were being
>    founded with a fleet they never paid for, the same arithmetic Phase 0.2 found,
>    through a second door. Mean firm lifespan **96.9 → 36.8 yr, now inside the 30–90
>    band for the first time**, and house wealth Gini **0.853 → 0.609, back in band**.
>
> **The one number that moved the wrong way:** top-10% wealth share **0.809 → 0.422**,
> now out of band from BELOW. The merchant elite is too flat. This is the mirror of the
> Phase 0.2 finding and points at the same place: Phase 3, whose job is to make the top
> of the distribution fragile, not to make the bottom crowded.
>
> ## Two numbers that are OUT OF BAND, deliberately
>
> | Metric | Now | Band | Why it is left alone |
> |---|---|---|---|
> | Mean firm lifespan | **96.9 yr** | 30–90 | Established firms almost never fail (**193.8 yr** excluding stillbirths). The honest fix is a failure *mechanism* — the Phase 3 crisis layer — not a smaller seed constant. Shrinking the seed re-introduces the stillbirths that were the original bug. |
> | House wealth Gini | **0.853** | 0.60–0.85 | Houses dying young was partly load-bearing: it destroyed wealth in an economy compounding at 1.5%/yr with no other brake. |
>
> **Consequence for the plan: Part E.2's open risk was real.** The too-rich and
> too-short-lived anomalies are **in tension, not one bug**. So Phase 0.2 is *not finished
> in isolation* — it must be co-tuned with Phase 3, whose revised gate is: move sustained
> richest wealth toward 100 000 while holding Gini in 0.60–0.85 and lifespan in 30–90.
> **The phase boundary drawn in Part 4 below is wrong as written.**
>
> ## How to measure anything here
>
> ```bash
> cd src-tauri
> cargo test --lib econ_diagnose_house_turnover -- --ignored --nocapture  # lifespan + causes
> cargo test --lib econ_ -- --nocapture                                   # the economy oracle
> cargo test --lib simulate_decades_reports_dynamics -- --nocapture       # the dynamics digest
> ```
>
> Use **mean firm lifespan**, never `dissolutions/century` — the latter scales with how many
> houses are standing and is right-censored on a 60-year run. The correct estimator is a
> hazard over exposure (`deaths ÷ house-years`), which `econ_diagnose_house_turnover` reports.
>
> ## Recommended next step
>
> **Phase 1** — and it is now the obvious one rather than a toss-up. Phase 0.4 wrote a
> succession LINE for every house (who held it, at what age, how they came in, how the
> family fared under them, and the by-name their tenure earned) and **nothing in the app
> shows it**. Phase 1.4's chronicle-first dossier has a subject now. It is read-only,
> touches no simulation and cannot regress either oracle.
>
> The two open questions Phase 0.4 leaves are both *diagnoses*, not code:
> why a newly-founded house (co-heir or branch) so often never trades at all, and why the
> top-10% wealth share fell out of band from below.
>
> ---
>
> ## ⚠ PHASE 0.1 AND 0.2 ARE DONE, AND THEY CHANGED THIS PLAN
>
> Measured (see `docs/SCOREBOARD.md` for the full write-up):
> - The cause was **the founding endowment**, not ambition: `wealth: 1.0` plus a
>   2–3 vessel fleet = ~1.4 months of runway at birth → dead at ≈13.4 months. Measured
>   median age at death 1.1 yr. **73% of dissolutions were houses that never traded.**
> - My overextension hypothesis was **refuted**: corr(age, committed upkeep) = **+0.802**.
> - Fixed by taking seed capital **from the parent guild** (which `maybe_found_house`
>   already requires). Lifespan **~12 yr → ~51–101 yr**.
> - **`dissolutions/century` was the wrong metric** — stock-dependent and censored. Use
>   the hazard estimator `deaths ÷ house-years`.
> - **The open risk in Part E.2 was real.** Gini 0.828 → **0.853**, out of band. Houses
>   dying young was partly load-bearing. The anomalies are **in tension, not one bug**, so
>   **0.2 must be co-tuned with the Phase 3 crisis layer** — the phase boundary below is
>   wrong as drawn.
> - **0.3 (determinism) is now a BLOCKER, not a backlog item**: three runs of the same
>   test gave lifespans of 51.1, 51.1 and 101.2 yr. Nothing here can be tuned to a band
>   until that is fixed.
>
> **Revised order: 0.3 (determinism) → finish 0.2 with Phase 3 → 0.4.**

| 0.1 | ~~**Diagnose house turnover.**~~ **DONE** Instrument dissolutions by cause (debt window / upkeep at zero wealth / early surcharge / never-viable spawns). Write the finding down even if no code changes. | A documented cause breakdown. Per §2.4 a diagnosis is a complete task. |
| 0.2 | **Fix turnover** to land inside 1–3 generations. | `econ_` house dissolutions/century moves from ~312 toward **33–100**; Gini stays in 0.60–0.85; dynamics run still shows turnover (houses dying is good — 12-year houses are not) |
| 0.3 | **Fix tick determinism.** Audit every hash accumulator in `tick/`, sort by key before folding. | `econ_scorecard_is_deterministic` un-ignored and passing; `simulate_decades_reports_dynamics` bit-identical at each step |
| 0.4 | ~~**Inheritance rule per culture** (partible / impartible).~~ **DONE** — two axes (line + division), five division rules, the matrilineal minority, and the succession line record. | ✅ `econ_inheritance_rules_fragment_differently`; dynamics bit-identical; `econ_` bands held or improved |

Phase 0 delivers no new UI. It is also the only phase that can make everything after it
tunable.

## Phase 1 — Read-only legibility (no simulation change at all)

| # | Step | Gate |
|---|---|---|
| 1.1 | **Tiers** + list grouping (rank-banded, hysteresis, Tier 1 may be empty) | `tsc`; dynamics untouched |
| 1.2 | **Culture dress figure** on the dossier — reuse `cultureFigure.ts`, 3 house marks, register by tier | `tsc`; schematic renders clean |
| 1.3 | **Expeditions tab** + province highlight (`Expedition.house` already exists) | `Expedition.dest_province` unread by the tick ⇒ dynamics bit-identical |
| 1.4 | **Chronicle-first dossier** (2.3) + **positive-event markers** (2.2) | `tsc` |

Nothing here can regress either oracle. This is the phase you can look at soonest.

## Phase 2 — People

| # | Step | Gate |
|---|---|---|
| 2.1 | **`Kin` roster** with gender; widows may inherit (1.2) | no roster ⇒ bit-identical |
| 2.2 | **Holdings authorship** — kin vs hired steward | as above |
| 2.3 | **`Character`** on Kin/Official/Figure, culture-derived, phrase only — no effects | **all-zero character ⇒ bit-identical** |
| 2.4 | **Character → knobs**, ±15% cap | dynamics bounded; `econ_` bands hold; all-zero still bit-identical |
| 2.5 | **Stewards** — skill, wage, skim, poaching | dynamics bounded; `econ_` bands hold |
| 2.6 | **Power shares + relations + modifiers**, read-only | `power_shares_always_sum_to_100` |

## Phase 3 — Politics

| # | Step | Gate |
|---|---|---|
| 3.1 | **~8 goals**, head-chosen (cut from 17, per Part 3) | achieve/fail rate sane over 200 yrs |
| 3.2 | **Competence + vice** | dynamics bounded; house death-rate must not spike |
| 3.3 | **Crisis**: open · named factions + tints · heir choice · rounds · resolve | `every_crisis_terminates`; `faction_names_and_tints_are_distinct`; `allegiance_partitions_the_house`; deposition rate sane over 300 yrs |
| 3.4 | **Contested undecided** + cause/stake shifts + grace period + **salience rule** (2.4) | courting spend must not move the econ scorecard |
| 3.5 | **Civic intervention** in crises — sequestration / exile of a faction (1.3) | dynamics bounded |
| 3.6 | **`CrisisRecord`** permanent + capped | save-size growth bounded over 500 yrs |

## Phase 4 — Consequences

| # | Step | Gate |
|---|---|---|
| 4.1 | **Departure schism** (holdings + wealth; wealth moves) | `econ_` Gini in 0.60–0.85; dissolutions must not spike |
| 4.2 | **Bankruptcy aftermath** — named creditor losses, kin barred from office (1.4) | dynamics bounded |
| 4.3 | **Plague as a lineage event** — multiple kin, extinction as a distinct death (1.6) | dynamics bounded; extinction rate small |
| 4.4 | **Foreign hand** — *only if* 2.5's measurement says it fires | must NOT materially raise the deposition rate |
| 4.5 | Deferred: religion/patronage · rupture · mavericks | — |

---

## The six invariants

Carried forward, to be written as tests as their phase lands:

1. `power_shares_always_sum_to_100`
2. `a_house_with_no_kin_is_bit_identical`
3. `every_crisis_terminates`
4. `allegiance_partitions_the_house`
5. `faction_names_and_tints_are_distinct`
6. **`a_house_lives_a_historical_span`** — new, and the most important: mean firm lifespan
   stays inside 30–90 years. This is Phase 0's gate promoted to a permanent guard, because
   every later phase can silently break it, and if it breaks the whole politics layer
   quietly stops meaning anything.

---

## What this plan says "no" to

Per §2.4, the failure mode on record for this project is tuning a constant until a spot
check passes while the aggregate regresses. The version of that failure available here is:
**build the politics, watch a crisis fire, declare it good, and never notice that houses
live twelve years.** Phase 0 exists to make that impossible.
