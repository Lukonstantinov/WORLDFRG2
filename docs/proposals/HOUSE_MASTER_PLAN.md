# The house mechanism — critique, then the master plan

**Status: Phase 0 COMPLETE; Phase 1 COMPLETE; Phase 2 COMPLETE; Phase 3 COMPLETE as
SCOPED; Phase 4 COMPLETE as scoped — 4.1 through 4.4 all built, 4.5's mavericks item
considered and declined, religion/patronage stays deferred by design. §2.5's own
"measure before building" instruction was honoured for 4.4: the 300-year diagnostic
(`econ_measure_foreign_hand_conjunction`) measured the conjunction firing 1229
times/century — the mechanism was then built, and the deposition/dissolution rate
did NOT rise materially. See the handoff block for the full account.**
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
> ## 4.4 (the foreign hand) is built — the measurement said to, and the gate held
>
> The immediately preceding handoff block below measured `econ_measure_foreign_hand_
> conjunction` in the background and left 4.4 un-attempted pending the result. The
> result: **1229 full-conjunction occurrences per century** (89,784 posted-kin-months
> sampled over 300 years; 27.66% show channel A or B present at all; 4.11% of THOSE
> also coincide with the kin already reading as disaffected, `loyalty < 0.4`) — two
> orders of magnitude past the "a handful a century" bar §2.5 itself set for whether
> this was worth building. So it was built: `sim/campaign/tick/foreign_hand.rs`,
> monthly, called right before the crisis/schism passes so the same month's pressure
> already feeds their discontent/tension reads.
>
> **The mechanism, kept small on purpose.** Two channels (`HOUSE_POWER_STRUGGLE_VIEW.md`
> §2): Channel A — any other live house holds an office or bailo in a posted kin's own
> city; Channel B — our house itself leases in a city a rival `captor_house` CONTROLS
> (the design's own "strong" channel — real dependency, not mere proximity). Leverage
> = `(0.5·A + 0.8·B, capped at 1.0) × rival's political_power × (1 + 0.5·feud)`, and it
> ONLY nudges that kin's `loyalty` down — small and bounded
> (`FOREIGN_HAND_DECAY_RATE`=0.01/month at leverage 1.0, so even the worst case, both
> channels plus an active feud plus a maximally powerful rival, is a 0.015/month
> ceiling, asserted directly by `foreign_hand_decay_is_small_and_bounded_in_a_single_
> month`). This is the literal test of the design's own promise: **"it CANNOT create
> the plot... a loyal, contented member is not turned by a rival's bailo"** — a single
> month can never move a fully-loyal kin far enough to matter; only YEARS of sustained
> exposure compounds into something that changes an outcome, and even then it is only
> ever deepening state (`house_tension`, crisis discontent) that ALSO has to clear its
> own independent, unrelated thresholds.
>
> **Disclosure, scoped down from "always".** The design says the leverage is "always
> disclosed" — a persistent, always-visible annotation. Building that literally would
> need a new PER-KIN state field (another House-adjacent struct patch across every
> construction site, the same blast-radius argument that cut 4.2's "kin barred from
> office"). Shipped instead as an occasional chronicle event
> (`FOREIGN_HAND_DISCLOSE_CHANCE`=0.06/month at leverage 1.0) naming the rival and the
> channel — "eventually and occasionally visible" rather than a persistent dossier
> annotation. The underlying effect (the loyalty decay) is UNCONDITIONAL regardless of
> whether this roll fires, so the mechanism's real behaviour doesn't depend on the
> flavour text firing.
>
> **The required gate held.** `HOUSE_POWER_STRUGGLE_VIEW.md` §7's own gate for this
> item is explicit: "must NOT raise the deposition rate materially... measure the rate
> with the channel off vs on." Diffed against the pre-4.4 commit on the real
> 60-year/30-city economy-oracle world: **house dissolutions/century 41.67 → 40.00**
> (down, not up), **banks chartered 23 → 24**, **house wealth Gini 0.698 → 0.693**,
> **top-10% wealth share 0.509 → 0.497** — every number moved by less than the
> run-to-run noise already visible between adjacent phases in this series, not a
> material shift. The gate is satisfied: leverage colours outcomes, it does not drive
> them.
>
> Whole-lib test suite: **219 passed, 0 failed** (was 215, +4:
> `foreign_hand_never_moves_a_kin_with_no_rival_presence`,
> `channel_a_exposure_lowers_a_posted_kins_loyalty`,
> `channel_b_exposure_via_a_controlled_lease_lowers_loyalty`,
> `foreign_hand_decay_is_small_and_bounded_in_a_single_month`). The small
> dynamics-test world stays byte-identical (its seeded houses have no kin roster, the
> same no-op pattern every Kin-gated feature in this series keeps). `cargo check`
> clean.
>
> **Phase 4 is now complete as scoped**: 4.1, 4.2, 4.3, 4.4 all built (each
> individually scoped down from its source design, every cut documented in its own
> handoff block); 4.5 considered item-by-item (religion/patronage still correctly
> deferred, Rupture deferred behind Departure since 4.1, mavericks declined this pass
> for a documented distribution-risk reason). Nothing in Phase 4's own table remains
> un-addressed.
>
> ## §2.4 salience, §2.5 the foreign-hand measurement, and §4.5's mavericks — the three small items still open after 4.1-4.3
>
> Asked to "move on with 4 and 5" after 4.1-4.3 shipped. Phase 5 does not exist
> anywhere in this document — there is no such section. Read as the three genuinely
> open items left in Phase 4's own table plus its Part 2 prerequisites: 4.4's gate
> (§2.5) was never measured, and §2.4 (crisis salience) was flagged as unbuilt in the
> 3.2-3.6 handoff. Building "4.4 outright" without its own gate would repeat the
> exact failure mode §2.4 of `CLAUDE.md` warns against — a feature shipped on vibes
> whose only measurement is "does it look reasonable in the code".
>
> **§2.4 · crisis salience — DONE.** `HOUSE_MASTER_PLAN.md`'s own §2.4 ("the player
> cannot watch fourteen houses... roughly one crisis a year somewhere") is now real:
> `crisis.rs`'s two `journal.push` calls (crisis opens, crisis resolves) are gated on
> `matches!(self.houses[hi].tier, 1 | 2)`. A Tier 3/4 (or not-yet-tiered) house's
> crisis is still written IN FULL to its own `events` chronicle — nothing about the
> house's own record changes — only the WORLD news feed goes quiet for it, the same
> "a healthy gauge stays quiet" discipline the stability gauges already use. Gated by
> `only_tier_one_and_two_crises_reach_the_news_feed`.
>
> **§2.5 · the foreign-hand measurement — INSTRUMENTED, per its own explicit
> instruction.** §2.5 doesn't ask for the mechanism; it says, verbatim, "Instrument
> it first: count how often the conjunction exists across a 300-year run, before
> writing the mechanism." That is exactly what `econ_measure_foreign_hand_conjunction`
> (new, `economy_validation.rs`, `#[ignore]`d like its sibling
> `econ_diagnose_house_turnover`) does: for every posted kin at every struck house,
> across 300 simulated years, it checks Channel A (a rival house holds an office or
> bailo in that kin's city) or Channel B (the house holds a lease in a city a rival
> `captor_house` controls), and separately tracks how often that conjunction ALSO
> coincides with the kin already reading as disaffected (`loyalty < 0.4`, the same
> rough cut `crisis.rs`'s own plot-leader pick already uses). It prints a verdict —
> fire the diagnostic yourself (`cargo test --release --lib
> econ_measure_foreign_hand_conjunction -- --ignored --nocapture`; it is a 300-year
> run and genuinely slow, run it in release mode and expect several minutes) to read
> the current number; per §2.4 of `CLAUDE.md`, a diagnosis is a complete task on its
> own, and 4.4 stays correctly un-attempted until this number says whether it is
> worth building. **This is a change from the earlier (wrong) handoff note** that
> said "2.5's foreign-hand work in the ORIGINAL numbering was never built... there is
> no measurement to gate 4.4 on" — that was true of a stewards/character-knobs
> reading of "2.5"; the ACTUAL §2.5 (Part 2 of this very file) is exactly the
> foreign-hand measurement instruction, and it has now been acted on directly rather
> than routed around.
>
> **4.5 · mavericks — considered, declined this pass, not silently skipped.** The
> design (`docs/proposals/HOUSE_MASTER_PLAN.md` Part 3) calls mavericks "flavour on
> top of an unproven baseline… keep, but last". The literal ask — an occasional kin
> rolling to a full ±2 character extreme — turns out to already be happening: reading
> `roll_character` (`houses.rs`), the existing uniform roll `((r−0.5)·5.0).round()`
> already lands on a full ±2 extreme roughly **20% of the time per axis** by
> construction (the round-to-nearest-integer buckets at the tails are half-width).
> A "maverick" as the design means it — a normally-centred distribution with a RARE
> escape to the extremes — would require tightening the BASELINE distribution first
> (e.g. summing two rolls for a more triangular shape) and only then adding the rare
> escape, which changes the input distribution to Phase 2.4's already-wired knobs,
> `head_vice`, EVERY crisis-round action-choice, and goal selection all at once — a
> genuinely systemic change with no gate of its own, exactly what §2.4 of
> `CLAUDE.md` calls the standing failure mode here. Declined rather than attempted
> cheaply and wrong; worth its own dedicated pass with an `econ_` check if picked up
> later.
>
> ## Phase 4.1–4.3 (Consequences) are built — Departure/Quarrel, bankruptcy aftermath, plague as a lineage event
>
> Asked to implement "all 3 phases" in one pass — read as 4.1/4.2/4.3, the three
> concrete, buildable items in Phase 4's table (4.4 is explicitly conditional on an
> unmeasured signal from 2.5's foreign-hand work, which was never built, so it has no
> trigger to gate on; 4.5 is explicitly "Deferred" in the table itself). Same
> discipline as every batch in this file: build everything, run the full gate suite
> once at the end.
>
> **4.1 — Departure schism** (`sim/campaign/tick/schism.rs`, new file). The design's
> `tension` formula (`HOUSE_PEOPLE_AND_TIERS.md` §5) reads a `cohesion` gauge that
> only exists as a READ-ONLY dossier computation (`campaign_house_stability`, not
> reachable cheaply from inside the tick) — `house_tension` is a documented
> stand-in built from state the tick already carries: mean kin loyalty, a
> `stretch` term (offices+bailos), a feud-count term, and a passed-over-heir flag.
> Monthly, a house above the threshold (and past its own cooldown) either QUARRELS
> (common — the disloyal kin's own loyalty craters further, chronicled as chatter)
> or, if that kin is POSTED to a real holding, DEPARTS with it to found a new rival
> house (25% of parent wealth, reusing `found_branch`'s pattern with a forced
> identity — the departing kinsman becomes the founder — the same technique
> `crisis.rs::depose_and_succeed` already established). **Rupture (a full split by
> line of descent) is NOT built** — this file's own Part 3 already recommended
> deferring it behind Departure, and a schism-triggered Rupture is the same risk to
> the wealth distribution by another door.
>
> **4.2 — Bankruptcy aftermath** (`dissolve_house`, extended in place). "Named
> creditor losses" is real: any bank still owed money by a dissolving house writes
> the loan down to zero, adds the loss to `Bank.losses` (the balance sheet's own
> existing write-off tally — no new state needed), and BOTH sides of the ledger name
> the other (the house's own "dissolved" event lists its creditor(s) and what they
> lost; the bank gets a `bad_debt` event naming the house). Because every dissolution
> path funnels through this one function — plain insolvency, a crisis's DISSOLVED
> outcome, and 4.3's plague extinction below — this is a single point of coverage
> for all of them. **"Kin barred from office in that city for a period" is NOT
> built.** It would need new PER-CITY state (a `TickHub` field), and unlike the House
> struct (which this series has patched at 7-8 call sites all along) a `TickHub`
> field touches many more construction sites across world-gen, colony-founding and
> satellite-founding code — real risk for a detail the source design itself calls
> "small". Cut and documented rather than attempted cheaply and wrong.
>
> **4.3 — Plague as a lineage event** (`disease.rs::plague_house_toll`, hooked into
> `strike_plague`). A struck house with real presence at the city (its seat, or any
> kin posted there) can lose SEVERAL non-head kin in one visitation (each rolled
> independently) or, rarely, be extinguished outright — a distinct chronicle kind
> (`plague_extinction`, a new milestone) from ordinary bankruptcy. **Deliberately
> INDEPENDENT of head mortality**, which stays governed entirely by
> `head_lifespan`/succession: extinction is its own small, separate roll rather than
> "did the head also happen to die", because reaching into that separate, already-
> tested mechanism for a flavour feature was real regression risk for no real gain —
> the player-visible outcome ("the family did not survive the plague") is identical
> either way. "Wealth concentrates in survivors" (1.6's other historical claim) needed
> NO extra code: fewer surviving kin simply means fewer co-heirs when Partible
> inheritance next divides an estate (Phase 0.4's `divide_estate`), which is the
> actual historical mechanism, not something this function should separately invent.
>
> ## A genuinely good measured result: 4.3 moved the ONE metric this whole series had left out of band
>
> Every earlier phase in this file was careful to say "byte-identical" or "moved but
> stayed in band" — Phase 0.4 left **top-10% wealth share out of band from BELOW**
> (0.809 → 0.422 when turnover was fixed) and flagged it as "Phase 3's job… to make
> the top of the distribution fragile". Diffed against the pre-Phase-4 commit on the
> real 60-year/30-city economy-oracle world: **top-10% wealth share 0.382 → 0.509**
> (still below its 0.60–0.90 band, but now much nearer it) and **house wealth Gini
> 0.607 → 0.698** (was already in its 0.60–0.85 band, now more centred rather than
> hugging the floor). This is exactly the historically-documented mechanism 1.6
> named — plague extinction removes weaker houses outright, concentrating the
> survivors' share — showing up as a real number, not asserted. Also moved: **bank
> failures/century 33.33 → 28.33**, **house dissolutions/century 46.67 → 41.67**,
> **banks chartered 25 → 23**. Whole-lib test suite: **214 passed, 0 failed** (was
> 206, +8: `a_quiet_house_never_schisms`,
> `a_disloyal_unposted_kin_can_only_quarrel_never_depart`,
> `a_posted_disloyal_kin_can_depart_and_found_a_rival_house`,
> `a_dissolved_house_leaves_a_named_creditor_loss`,
> `a_house_with_no_debt_dissolves_with_no_creditor_line`,
> `a_plague_can_kill_several_kin_at_once`,
> `a_plague_can_extinguish_a_house_independent_of_the_head`,
> `a_house_with_no_presence_at_the_struck_city_is_untouched`). The small dynamics-test
> world stays BYTE-IDENTICAL (its seeded houses have no kin roster — see Phase 0.4's
> own finding — so `house_tension` and `plague_house_toll` both read "nothing to act
> on" there, exactly the same no-op pattern every Kin-gated feature in this series
> has kept). `cargo check`/`npx tsc --noEmit` both clean.
>
> **What's left of Phase 4**: 4.4 (foreign hand) stays gated on a signal nobody has
> measured (2.5's foreign-hand work in the ORIGINAL numbering was never built — the
> shipped 2.5 here is stewards, a different item entirely; there is no "2.5
> measurement" to gate 4.4 on, so it remains correctly un-attempted, not overlooked).
> 4.5 stays deferred by the plan's own design.
>
> ## Phase 3.2–3.6 (the crisis engine) is built — real, but cut down hard from the source designs
>
> Asked to implement "the last step" in one pass, same discipline as every prior batch
> here: build everything, check once at the end. This is the biggest single addition
> in the series — a new `sim/campaign/tick/crisis.rs` (~470 lines) implementing
> competence/vice (3.2), the crisis struggle with named factions (3.3), a folded-in
> undecided contest + grace period (3.4), civic intervention (3.5), and the permanent
> capped record (3.6). It consolidates FOUR source documents
> (`HOUSE_POWER_AND_POLITICS.md`, `HOUSE_SUCCESSION_CRISIS.md`,
> `HOUSE_POWER_STRUGGLE_VIEW.md`, `HOUSE_FACTION_NAMING_AND_RECORD.md`) that between
> them describe a system several times the size of what's built here. Read the module
> doc comment at the top of `crisis.rs` first — it states the two biggest cuts up
> front, not buried in a design rationale nobody re-reads.
>
> ## The two cuts that matter most
>
> 1. **No per-figure power-share ledger.** The source design (`HOUSE_POWER_AND_POLITICS.md`
>    §1) makes every prominent kinsman hold a numeric share of a 100% pie, with a
>    `PowerModifier` log explaining every point gained or lost. **Not built.**
>    `head_support`/`plot_support` on `HouseCrisis` are two abstract aggregate numbers
>    derived from discontent at crisis-open time and nudged by each round's action —
>    not a sum of named shares. This is the single biggest simplification: it is what
>    let 3.2–3.6 ship in one pass instead of needing the ledger as a prerequisite.
> 2. **No drifting `regard` ladder.** The source design (`HOUSE_SUCCESSION_CRISIS.md`
>    §1) has every kinsman's relation to the head drift continuously year over year,
>    with a reason attached to every move. **Not built.** Plot leadership instead
>    reads each kin's existing STATIC `Kin.loyalty` roll (set once at
>    `ensure_kin_roster` time) — the least-loyal live, non-head kinsman becomes the
>    plot leader. This is honest but real: a house's plot leader today is whoever
>    randomly rolled the lowest loyalty at the last founding/succession, not someone
>    whose relationship visibly soured over years of being passed over.
>
> ## Smaller, deliberate cuts (each already recommended by this file or the source docs)
>
> - **The Split/schism outcome is not built.** `HOUSE_SUCCESSION_CRISIS.md`'s
>   resolution table has four outcomes; only three ship (Prevailed / Deposed /
>   Dissolved). This isn't a shortcut invented for this pass — Part 3 of THIS file
>   already recommends deferring "Rupture (full house split by descent) behind
>   Departure", and a crisis-triggered Split is the same risk to the wealth
>   distribution by another door.
> - **Only 4 of 6 head actions.** "Concede a holding", "buy off the plot", "launch a
>   venture", "stand firm" are built; "marry a rival's line in" and "press a feud to a
>   win" are cut because both need state this pass didn't add (a marriage-in mechanic,
>   a way to target one SPECIFIC feud from crisis code) — matches the same
>   "build what needs no new subsystem" discipline goals used to cut 17 kinds to 7.
> - **Only 2 of 6 faction-naming patterns.** "Tincture + Charge" and "Leader's men" are
>   built; the four culture-specific patterns (Brotherhood/Legitimist/Place/Grievance)
>   need a culture-keyed word table this pass didn't build. Contrast and
>   distinctness are still guaranteed (opposite-index tincture pick), just with less
>   naming variety than the full design.
> - **No structured `CauseShift`/stake-shift log.** A round's narrative TEXT can note a
>   shift in prose ("the venture failed" implies the crisis is now about the ships,
>   not the funds), but there's no separate `CauseShift` struct tracking it — the
>   design's own decision 3 (`HOUSE_FACTION_NAMING_AND_RECORD.md`) is only partially
>   honoured.
> - **The "salience rule" referenced in the master-plan table (§2.4 of a source doc
>   this pass didn't need to open) was not built at all** — there is no separate
>   visibility gating beyond the dossier tab appearing only when a crisis exists.
> - **No `allegiance_partitions_the_house` invariant.** That test's premise (every
>   prominent figure sits in exactly one of three camps, summing to the pie) doesn't
>   apply to a model with no per-figure ledger. In its place: `every_crisis_terminates`
>   (round cap respected, exactly one crisis per house) and
>   `faction_names_and_tints_are_distinct` (24-seed sweep, names/tints never collide) —
>   both real invariants of what actually shipped, not the design's original two.
> - **The crisis→deposition rate over 300 years is UNMEASURED**, same honest gap
>   pattern as Phase 3.1's goal achieve/fail rate — a long-run diagnostic
>   (`econ_diagnose_house_turnover`'s own pattern) would answer it and wasn't built
>   this pass.
>
> ## A real bug this pass found and fixed: deposition could break the inheritance law
>
> The first cut of `pick_crisis_successor` picked whoever fit the crisis role (plot
> leader, heir, prominent kinsman) with NO regard for the house's culture-mandated
> `LineRule` — and a pre-existing test, `a_matrilineal_house_is_held_by_women`, caught
> it immediately: a 70-year run put a MAN ("Titus") at the head of an Enatic
> (matrilineal) house, because the deposed successor's sex came from whichever kin
> happened to lead the plot, not from the culture's own law (Phase 0.4). Fixed by
> computing the culture's expected sex the same way `succeed_house` already does
> (`crate::sim::inheritance::heir_is_female`) and filtering every candidate against
> it before falling back to a freshly-generated, correctly-sexed synthetic name. This
> is the same class of bug Phase 2.5 found (a new mechanic silently breaking an
> existing backward-compatibility/correctness invariant) and the same lesson: **the
> existing test suite is what caught it, not review** — a reason the "check once at
> the end" approach still needs the full suite run, not a partial one.
>
> ## Measured effect
>
> Diffed against the pre-3.2–3.6 commit on the real 60-year/30-city economy-oracle
> world: **Gini 0.649 → 0.607** (stays inside the 0.60–0.85 band, though now nearer
> its floor), **top-10% share 0.409 → 0.382** (already below its 0.60–0.90 band before
> this pass — an existing, documented finding, not something this phase was expected
> to fix, and it moved further from the band rather than into it — worth watching, not
> yet alarming at this magnitude), **surviving houses 49 → 44**, **banks chartered
> 23 → 25**, **bank failures/century 36.67 → 33.33**, **house dissolutions/century
> unchanged at 46.67** (the Dissolved crisis outcome is "very rare" by design — its
> absence from this number is expected, not a sign it never fires). The whole lib
> test suite is **206 passed, 0 failed** (was 199, +7 new:
> `every_crisis_terminates`, `faction_names_and_tints_are_distinct`,
> `head_vice_is_a_true_noop_with_no_roster_or_flat_character`,
> `head_vice_matches_the_designs_priority_order`,
> `lavish_vice_costs_wealth_a_sober_head_does_not_pay`,
> `a_decisive_plot_deposes_the_head`, `a_decisive_head_prevails_and_earns_a_grace_period`) —
> verified by diffing the pre-pass commit's own `cargo test --lib` total, not just
> eyeballing green. `simulate_decades_reports_dynamics` stays bounded (no blow-ups,
> no craters). `cargo check`/`npx tsc --noEmit` both clean.
>
> A ⚠ Crisis dossier tab was added (`HousesPanel.tsx`): the live struggle (two
> named factions in their own tinctures, a round log, the heir's recorded choice) plus
> a permanent "past risings" list — read-only, matching decision 2 of
> `HOUSE_SUCCESSION_CRISIS.md` ("observation only... the AI supplies every choice").
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
| 3.2 | ~~**Competence + vice**~~ **DONE (scoped — see the handoff finding)** — competence is `kin[0].skill` read directly at each call site; vice is derived from character+skill (5 named vices), one wired economic consequence (Lavish → extra consumption drain). | ✅ `head_vice_*` tests; dynamics bounded (`simulate_decades_reports_dynamics` still healthy) |
| 3.3 | ~~**Crisis**: open · named factions + tints · heir choice · rounds · resolve~~ **DONE (scoped — see the handoff finding)** — `HouseCrisis`, quarterly rounds fixed at `CRISIS_ROUND_CAP`=4, faction names/tints drawn from the house's own heraldic palette, heir choice recorded at opening. **No per-figure power-share ledger and no drifting `regard` ladder** — see the handoff. | ✅ `every_crisis_terminates`; `faction_names_and_tints_are_distinct`; econ/dynamics move but stay in band (see handoff numbers) |
| 3.4 | ~~**Contested undecided** + grace period + salience~~ **DONE (partial — see the handoff finding)** — the undecided bloc is folded into each round's own delta rather than a separate contest step; `crisis_immune_until` (5yr grace) is built and tested. §2.4's salience rule ("the player cannot watch fourteen houses") is now built too: only Tier 1-2 crises reach the world news feed, Tier 3-4 stay fully chronicled on the house's own record but silent on the world stage. **The structured `CauseShift` log is still NOT built** — a backfire's narrative text notes a shift in prose only, no separate data field. | ✅ `a_decisive_head_prevails_and_earns_a_grace_period`; `only_tier_one_and_two_crises_reach_the_news_feed` |
| 3.5 | ~~**Civic intervention**~~ **DONE (scoped — sequestration only, no exile)** — a severe deposition (peak plot ≥0.6) has a 25% chance the seat's council sequesters 3% of the estate into its treasury. | ✅ exercised by the dynamics/econ runs; no dedicated unit test (rare, small, same discipline as other tail events) |
| 3.6 | ~~**`CrisisRecord`** permanent + capped~~ **DONE** — capped at `CRISIS_HISTORY_CAP`=8, same discipline as `goal_history`. | ✅ `every_crisis_terminates` asserts exactly one record per resolved crisis |

## Phase 4 — Consequences

| # | Step | Gate |
|---|---|---|
| 4.1 | ~~**Departure schism** (holdings + wealth; wealth moves)~~ **DONE (Quarrel + Departure; Rupture deferred)** — `sim/campaign/tick/schism.rs`, monthly, gated on a simplified `tension` proxy + a per-house cooldown. | ✅ `econ_` Gini 0.60–0.85 held; 3 new tests |
| 4.2 | ~~**Bankruptcy aftermath** — named creditor losses, kin barred from office (1.4)~~ **DONE (creditor losses only — see the handoff finding)** — `dissolve_house` now writes off any outstanding bank loan and names the bank on both ledgers. | ✅ 2 new tests; dynamics bounded |
| 4.3 | ~~**Plague as a lineage event** — multiple kin, extinction as a distinct death (1.6)~~ **DONE** — `disease.rs::plague_house_toll`, independent of head mortality. | ✅ 3 new tests; measurably moved Gini/top-10% TOWARD their bands — see the handoff finding |
| 4.4 | ~~**Foreign hand** — *only if* 2.5's measurement says it fires~~ **DONE — measurement said build it (1229/century)** — `sim/campaign/tick/foreign_hand.rs`, two channels, bounded monthly loyalty decay, occasional disclosure. | ✅ 4 new tests; deposition/dissolution rate did NOT rise materially (41.67 → 40.00/century) — see the handoff finding |
| 4.5 | Religion/patronage: still deferred (Part 3's own reasoning holds — a third system on an unbuilt second one). Rupture: deferred, see 4.1. **Mavericks: considered and declined this pass** — see the handoff finding. | — |

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
