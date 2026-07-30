# The house mechanism — critique, then the master plan

**Status: Phase 0 COMPLETE; Phase 1 COMPLETE; Phase 2 COMPLETE; Phase 3.1 (goals)
built as structure, not yet wired to bias decisions — see the handoff block; 3.2–5
are plan.**
Consolidates the design documents
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
> ## Phase 3.1 (goals) is built — as STRUCTURE, not yet the closed loop §4 describes
>
> Phase 0, 1 and 2 are complete. Phase 3 (Politics) is the big one — goals, competence/
> vice, and a full multi-round crisis engine with named factions, civic intervention,
> and a permanent record (3.2 through 3.6). Only **3.1 (goals)** is built so far; 3.2–3.6
> are genuinely a different scale of work (a crisis engine, not a tracked ambition) and
> deserve their own dedicated pass rather than being compressed into this one.
>
> **What 3.1 actually is.** Seven goal kinds (cut from the design's 17 to the ones that
> reference systems already in this codebase — every one of monopoly tracking,
> `council_house`/`captor_house`, `bailos`, bank solvency, expeditions with
> `dest_province`, feuds, and `peak_wealth` already existed): corner the trade, seat
> the council, raise a bailo, charter a bank, reach a province, outlast a rival, restore
> the house. Chosen yearly, biased by archetype and the head's character axes; checked
> yearly (or, for `GOAL_REACH_PROVINCE`, by a hook in `expedition_travel_pass` the
> moment a backed expedition completes its round trip); achieved is a milestone, failed
> is chatter, both chronicled and shown in a new 🎯 Ambitions dossier tab.
>
> **What 3.1 is NOT, and this matters**: §4 says "a goal biases the WEIGHTS of decisions
> the AI already makes — it never adds a new action." As built, goals are read-only —
> they TRACK toward success/failure against state the sim already produces, but nothing
> in `decide_fleets`/`update_feuds`/`update_guilds_and_offices`/etc. reads a house's
> active goal to weight its choices. That closed loop (a house pursuing "corner the silk
> trade" actually trading more aggressively in silk BECAUSE of the goal) is not built.
> This is why the gate came back byte-identical against the pre-3.1 economy scorecard —
> not a bug, but a real gap between what's shipped and what §4 describes. Wiring the
> bias in is the natural 3.1b, and it's exactly the kind of change 2.4/2.5's own finding
> warns about: it moves wealth, so it needs its own `econ_` check as it's built, not
> folded into a single end-of-session run.
>
> **The achieve/fail rate gate ("sane over 200 yrs") is UNMEASURED**, not passing —
> building a 200-year long-run diagnostic (mirroring `econ_diagnose_house_turnover`'s
> pattern) is the next honest step before trusting the 7 kinds' balance against each
> other, and it was not built this pass. Recorded here so it isn't silently assumed
> fine.
>
> ## Phase 1 and Phase 2 are BOTH COMPLETE
>
> The previous handoff deferred 2.4 (character → decisions) and 2.5 (stewards) because
> both move house wealth directly and the earlier passes here had all been checked
> incrementally. Asked explicitly to build them anyway with a single check at the end,
> they're now done, gated, and the bands held — but building them exposed one real
> compatibility bug that the "check only at the end" approach could easily have missed
> entirely, which is the whole reason that deferral existed in the first place. Recorded
> in full below because it is the most important finding of this pass.
>
> ### The bug: "no roster" quietly became "assume it's all hired"
>
> The first cut of 2.5 read an EMPTY kin roster (every save from before Phase 2.1, or any
> house that hasn't succeeded since) as "every holding is hired" — because an empty
> roster trivially has no kin POSTED anywhere, so the wage/skim/poaching logic charged
> the WORST case rather than recognising it had no information at all. This directly
> contradicts the master plan's own invariant #2, `a_house_with_no_kin_is_bit_identical`,
> and would have made an old save's houses suddenly and silently cheaper to run than a
> freshly-generated one purely because their roster hadn't been (re)generated yet — a
> real backward-compatibility regression, not a cosmetic one.
>
> **Caught by the test suite, not by inspection**: `hired_offices_cost_more_than_
> family_run_ones` passed on the first run (a genuinely hired-vs-family comparison), but
> the ORIGINAL `a_house_with_no_kin_is_bit_identical` — retained from Phase 2.1 — started
> failing the moment 2.5 landed, because clearing a house's kin roster mid-run (its whole
> test) now measurably cut its costs instead of leaving them untouched. That failure is
> what surfaced the bug. Fixed by gating the ENTIRE steward mechanic (wage, skim,
> poaching) on a non-empty roster: `!self.houses[hi].kin.is_empty()`. An absent roster
> now means "nothing is known", never "assume the worst" — which also matches how every
> other Kin-adjacent feature in this plan already treats an empty roster.
>
> The old invariant's name no longer describes what's true (a house WITH a roster is not
> bit-identical to one without — that was the entire point of building 2.4/2.5), so it's
> renamed `an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached`, scoped to what
> is actually still guaranteed.
>
> ### What else landed
>
> - **2.4** — one touchpoint per character axis (not all three the design lists per
>   axis): boldness moves the fleet-buy affordability threshold, greed moves feud
>   heating speed, civic-mindedness moves the consumption rate that funds
>   `fund_public_works`, expansiveness moves the office-opening affordability threshold.
>   Each capped at exactly ±15% (`CHARACTER_KNOB_CAP`) and a TRUE 1.0 no-op with no
>   roster or an all-zero roll — not an approximation, gated by a dedicated test.
> - **2.5** — a hired holding costs `STEWARD_WAGE` (fixed) + `STEWARD_SKIM_RATE`
>   (proportional, capped to a few holdings' worth) per month, and can be
>   `STEWARD_POACH_CHANCE`=1%/month POACHED — reusing the existing office-close
>   machinery with a distinct event kind. A poached office may be immediately
>   restaffed by the same pass's OPEN logic if the trade tie is still strong — realistic
>   resilience, not a missing event, so the gate counts events directly rather than
>   watching the office list for a hole that may not stay open.
> - **Measured effect**: on the small 30-house/50-year dynamics-test world the change is
>   BYTE-IDENTICAL (verified by diffing the printed output against the pre-2.4/2.5
>   commit) — that world's seeded houses never succeed inside 50 years, so they never
>   gain a roster, and newly-founded houses rarely accumulate offices fast enough to
>   matter there. On the real 60-year/30-city economy-oracle world the effect is real:
>   Gini 0.609→0.649, top-10% share 0.422→0.409, mean firm lifespan 36.8→39.9yr — all
>   moved, none left their bands.
>
> **Phase 2 (People):** 2.1 (`Kin` roster + widow regency), 2.2 (holdings authorship),
> 2.3 (character as a phrase, no effects) and 2.6 (power shares) are built. **2.4
> (character wired to real decisions) and 2.5 (stewards with skim/wage mechanics) were
> NOT attempted this pass — deliberately, not by running out of time.** Both move house
> wealth directly, which means each needs its own `econ_` verification as it's built,
> the same discipline every other economically-live change in this file has followed.
> Building four knobs blind and finding out at the end whether the Gini/lifespan bands
> survived would be exactly the failure mode §2.4 of `CLAUDE.md` warns against — a spot
> win that risks an aggregate loss, discovered too late to cheaply revert.
>
> Four findings from this pass:
>
> 1. **`Kin.posted` is a snapshot, not a live mirror.** It's set when the roster is
>    (re)generated — at founding and every succession — from whatever holdings the
>    house owns AT THAT MOMENT. A holding gained between successions has no posted
>    kin until the next one. Documented on the field itself and in the master-plan
>    table rather than hidden; the alternative (keeping `posted` continuously
>    synchronised with `hubs[].owner_house`/`offices`) is real additional machinery
>    for a cosmetic display, and this project's own rule 4 (no half-finished
>    abstractions) argues against building it speculatively.
> 2. **The widow regency needed no roster dependency at all.** The design frames it as
>    "if the head dies with no eligible heir, the widow becomes head" — but the kin
>    roster doesn't yet track marriages, so "is there a widow" isn't answerable from
>    state. Implemented instead as an independent small roll
>    (`WIDOW_REGENCY_CHANCE`=8%) on purely agnatic successions, which is the one line
>    rule that otherwise has zero route to a female head (agnatic-cognatic, absolute
>    and enatic already produce one via `heir_is_female`). Gated by
>    `widow_regency_occasionally_holds_an_agnatic_house` — it must fire, and it must
>    stay rare.
> 3. **Relations/modifiers (the rest of 2.6) has no state to derive from yet.** The
>    design's "power shares + relations + modifiers" bundles three things; only power
>    shares are pure functions of the roster alone. A kin's RELATION to another kin
>    (rival, ally, married-in) needs the marriage/schism state Phase 2's own later
>    items would add — building it now would mean inventing state Phase 4/5 might
>    duplicate or contradict.
> 4. **Holdings authorship folds naturally into the existing Summary tab** rather than
>    needing a new view: an estate/office row now reads "Kelmar (Tanmo)" when family-run
>    and stays unmarked when hired — the same "quiet unless it matters" rule the
>    stability gauges and the character phrase both already follow.
>
> ## Phase 1.1/1.2/1.4 are DONE — 1.3 (expeditions tab) is what's left of Phase 1 (superseded — see above)
>
> Three findings from building the tier list, the dossier figure, and the
> chronicle-first reorder:
>
> 1. **The "three house marks" plan needed one simplification.** Recolouring the
>    garment's accent band inside `cultureFigure.ts` would mean threading a house
>    colour through the shared SVG renderer also used by `PeoplesPanel` — real risk
>    for a cosmetic win. Shipped instead as a coloured FRAME around the portrait,
>    which reads the same ("of its culture, but distinct") without touching shared
>    code. The coat-of-arms badge and tier-register occasion are exactly as designed.
> 2. **The positive-event set (§2.2) had to be cut to what's honestly measurable.**
>    **Finest hour** (a marker, never chronicled — spamming a peak-wealth event most
>    months would be worse than the obituary problem it fixes) and **golden age**
>    (Tier 1 held + wealth rising, a decade) are built and gated by tests. **Dynasty
>    of merchants** is built too, but from `line` (Phase 0.4) rather than needing new
>    state — three consecutive heads who each grew the house. **Great partnership**
>    needs alliance-linked tier rises (a bigger join than this pass did) and
>    **legendary head** needs goals (Phase 3, unbuilt) — both deferred, not built
>    silently short.
> 3. **`succeed_house`'s branch-on-succession can eat three generations of growth.**
>    A rich house spinning off a cadet branch at every gen>=2 succession (30% of
>    wealth) can make "three consecutive GROWING heads" genuinely hard to reach even
>    in an economy that's compounding — worth knowing before reading the dynasty rate
>    off a real campaign as a fidelity signal.
>
> Phase 1 is now Tiers · figure · chronicle-first — all shipped. **1.3 (expeditions
> tab)** is the smallest of what's left: `Expedition.house` already exists; it needs
> one new field (`dest_province`) for the province highlight and goal-checkability.
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
| 1.1 | ~~**Tiers** + list grouping (rank-banded, hysteresis, Tier 1 may be empty)~~ **DONE** — `assign_house_tiers` (`sim/campaign/tick/houses.rs`), `HousesPanel.tsx` groups by tier (3/4 collapsed by default). | ✅ `tsc` clean; dynamics bit-identical (`simulate_decades_reports_dynamics`); 3 new tests |
| 1.2 | ~~**Culture dress figure** on the dossier — reuse `cultureFigure.ts`, 3 house marks, register by tier~~ **DONE** — mark 1 (garment recolour) simplified to a coloured frame around the portrait rather than touching the shared SVG renderer; marks 2 (CoatOfArms badge) and 3 (tier-register occasion) built as specified. | ✅ `tsc` clean |
| 1.3 | ~~**Expeditions tab** + province highlight (`Expedition.house` already exists)~~ **DONE** — `Expedition.dest_province`, an 🧭 Expeditions dossier tab, click-to-highlight on the province plate. | ✅ `tsc`; `dest_province` unread by the tick ⇒ dynamics bit-identical |
| 1.4 | ~~**Chronicle-first dossier** (2.3) + **positive-event markers** (2.2)~~ **DONE** — Chronicle is now the FIRST/default dossier tab, showing the Phase 0.4 succession line inline plus the year-grouped event log. Finest hour + golden age + dynasty of merchants built; great partnership + legendary head deferred (see finding above). | ✅ `tsc`; 3 new `tick::` tests; dynamics bit-identical |

Nothing here can regress either oracle. This is the phase you can look at soonest.

## Phase 2 — People

| # | Step | Gate |
|---|---|---|
| 2.1 | ~~**`Kin` roster** with gender; widows may inherit (1.2)~~ **DONE** — `kin[0]` mirrors the head, 2–4 siblings generated per founding/succession; an agnatic line's one route to a female head, `WIDOW_REGENCY_CHANCE`=8%. | ✅ `an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached`; `widow_regency_occasionally_holds_an_agnatic_house` |
| 2.2 | ~~**Holdings authorship** — kin vs hired steward~~ **DONE (simplified)** — `Kin.posted` is a SNAPSHOT taken at roster generation, not live-synced to holdings gained since (documented, not hidden); the Summary tab tags a family-run estate/office with the posted kin's name, silent (= hired) otherwise. | ✅ `tsc` |
| 2.3 | ~~**`Character`** on Kin/Official/Figure, culture-derived, phrase only — no effects~~ **DONE (Kin only, not Official/Figure)** — four axes rolled per kin, `character_phrase` reads notable axes only (§3's own discipline), never wired to a decision. | ✅ `character_phrase_is_quiet_unless_notable`; trivially bit-identical (nothing reads `Kin.character`) |
| 2.4 | ~~**Character → knobs**, ±15% cap~~ **DONE** — one touchpoint per axis (not all three the design lists per axis): boldness → fleet-buy threshold, greed → feud heat, civic → consumption/civic-pool rate, expansive → office-open threshold. `head_character_factor` is a true 1.0 no-op with no roster or an all-zero character. | ✅ `character_factor_is_a_true_noop_at_zero`; `character_factor_is_bounded_and_directional`; `econ_` bands hold (Gini 0.649, lifespan 39.9yr) |
| 2.5 | ~~**Stewards** — skill, wage, skim, poaching~~ **DONE** — a hired (unposted) holding costs a small wage + skim (`apply_wealth_sinks`) and can be POACHED (`update_guilds_and_offices`); gated on a NON-EMPTY roster so an old save with no roster pays nothing — see the finding below. | ✅ `hired_offices_cost_more_than_family_run_ones`; `poaching_occasionally_takes_a_hired_office_never_a_family_one`; `a_guild_has_no_steward_cost_or_poaching`; `econ_` bands hold |
| 2.6 | ~~**Power shares** + relations + modifiers, read-only~~ **DONE (power shares only)** — `kin_power_shares` (role × skill × loyalty, normalised); relations/modifiers between kin not built (no marriage/feud-at-the-kin-level state to derive them from yet). | ✅ `power_shares_always_sum_to_100` |

## Phase 3 — Politics

| # | Step | Gate |
|---|---|---|
| 3.1 | ~~**~8 goals**, head-chosen (cut from 17, per Part 3)~~ **DONE (structure only — see the handoff finding)** — 7 kinds, chosen yearly by archetype/character bias, checked yearly, chronicled achieved (milestone) vs failed (chatter), a 🎯 Ambitions dossier tab. **Goals do NOT yet bias any decision's weights** — they are read-only tracking against state that already exists, not the closed loop §4 describes. | ✅ 6 new tests (one per representative kind + slot cap); `econ_`/dynamics BYTE-IDENTICAL (goals touch no wealth) |
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
2. `an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached` (renamed from
   `a_house_with_no_kin_is_bit_identical` when Phase 2.4/2.5 made a roster's PRESENCE,
   not just its content, a real behavioural difference — see the handoff block)
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
