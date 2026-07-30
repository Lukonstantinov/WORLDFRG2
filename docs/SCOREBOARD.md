# WorldForge 2 — Scoreboard

**The project in twelve numbers.** 89k lines across climatology, economics,
rendering and UI is more than anyone can hold as code. It is easy to hold as a
table of measurements. That is what this file is for.

Append a row every session that moves a number. Never edit an old row — a
scoreboard whose history is rewritten cannot show a regression.

---

## Current state — 2026-07-30 (Phase 1.2/1.4 · figure + chronicle-first dossier)

Also read-only/query-side — no economy number moves. `HouseDetail`'s default tab is now
Chronicle (§2.3), showing the Phase 0.4 succession line inline before the year-grouped
event log. The dossier opens on a `cultureFigureSVG` portrait in the seat culture's kit
and the head's own sex, tier-registered (ceremonial/national/everyday). Three positive
events (§2.2) shipped as markers on `House`: finest hour (peak wealth, never chronicled),
golden age (a decade at Tier 1 with wealth rising), dynasty of merchants (three
consecutive heads who each grew the house, derived from Phase 0.4's `line`). 61 `tick::`
tests pass (was 58); dynamics and economy scorecards bit-identical.

**Finding:** `succeed_house`'s branch-on-succession (30% of wealth spun off at every
gen>=2 succession) can make "three consecutive GROWING heads" genuinely hard to reach
even in a compounding economy — worth knowing before reading the dynasty-fire rate off a
real campaign as a fidelity signal.

---

## Current state — 2026-07-30 (Phase 1.1 · house tiers)

Read-only, query-side classification — no economy number moves. `assign_house_tiers`
bands every live private house into a rank (1 great .. 4 marginal) from state that
already existed, with hysteresis on both the percentile cutoffs and Tier 1's absolute
floor. `HousesPanel.tsx` groups the list by tier (3/4 collapsed by default, per
`HOUSE_PEOPLE_AND_TIERS.md` §1's schematic). 58 `tick::` tests pass (was 55); dynamics
and economy scorecards bit-identical to the Phase 0.4 numbers below — nothing downstream
reads `tier`, by design.

---

## Current state — 2026-07-30 (Phase 0.4 · inheritance)

Only the numbers that MOVED. Everything else still reads as the 2026-07-29 table below.

| Metric | Value | Gate | Status |
|---|---|---|---|
| **Economy: mean firm lifespan** | **36.8 yr** (was 96.9) | `econ_diagnose_house_turnover` | ✅ **inside the 30–90 band for the first time** |
| Economy: lifespan excl. stillbirths | **147.0 yr** (was 193.8) | same | ❌ established firms still almost never fail — Phase 3's job |
| Economy: house wealth Gini | **0.609** (was 0.853) | `ECON_GINI_FLOOR` = 0.15 | ✅ **back inside the 0.60–0.85 band**, at its floor |
| Economy: top-10% wealth share | **0.422** (was 0.809) | — | ❌ **left the 0.60–0.90 band from below** — the merchant elite is now too flat |
| Economy: houses alive at 60 yr | **42** (was 2) | — | ⚠️ the reference world finally HAS a merchant class |
| Economy: house dissolutions / century | 46.7 (was 10.0) | — | ⚠️ stock-dependent — read the lifespan row instead |
| **Inheritance rule is wired** | partible **18 divisions / 22 co-heirs**; primogeniture · ultimogeniture · seniority **0** | `econ_inheritance_rules_fragment_differently` | ✅ asserted |
| Inheritance: houses ever founded | partible **88** · primogeniture **55** · ultimogeniture **49** · seniority **124** | same | ✅ the rule measurably changes fragmentation |
| Inheritance: mean wealth per house | partible **120 325** · primogeniture **195 264** | same | ✅ same capital, spread thinner |
| **Rust tests** | **171 pass, 0 fail** (4 ignored) | CI | ✅ |
| Dynamics: sustained richest house | 154 045 — **unchanged** | `late_max < 1e6` | ✅ bit-identical (that world seeds no successions) |

**Why so much moved at once.** The reference world was not reproducing campaign start:
`tests::sim()`'s placeholder gave every seeded head a **274-year** lifespan, so not one
of the ten houses ever reached a succession inside a 60-year run. Every number that
depends on generational turnover — lifespan, Gini, top-10%, surviving houses — was
measuring a world where merchant families were immortal. `calibrate_like_campaign_start`
now runs the same two steps `campaign_start_sim` does (`ensure_culture_rules` +
`seed_house_lines`). The old numbers were not wrong measurements; they were measurements
of the wrong world.

---

## Current state — 2026-07-29

| Metric | Value | Gate | Status |
|---|---|---|---|
| **Earth main-class agreement** | **66.3%** | `EARTH_MAIN_FLOOR` = 65.0 | ✅ asserted |
| **Earth exact-zone agreement** | **29.1%** | *none* | ⚠️ ungated |
| Earth C-class own accuracy | 32.8% | — | worst class |
| Earth `C → B` confusion | 40% | — | largest single error |
| Earth `D → E` confusion | 40% | — | second largest |
| **Economy: price/distance gradient** | **−0.01** | *none* | ❌ distance does not move prices |
| Economy: grain price CV across cities | 2.10 | `ECON_SPATIAL_CV_FLOOR` = 0.01 | ⚠️ far above band (0.20–0.40) |
| Economy: rank-size (Zipf) slope | −0.41 | band [−3.0, −0.15] | ⚠️ flatter than −0.8…−1.2 |
| Economy: urban share drift (60 yr) | 0.100 → **0.997** | — | ❌ countryside empties completely |
| Economy: house dissolutions / century | **10.0** (was 312) | — | ⚠️ superseded — use lifespan below |
| **Economy: mean firm lifespan** | **96.9 yr** (was ~12) | `econ_diagnose_house_turnover` | ⚠️ slightly ABOVE band (30–90) — now stable and measurable |
| Economy: lifespan excl. stillbirths | **193.8 yr** | same | ❌ established firms now almost never fail — Phase 3's job |
| Economy: house wealth Gini | **0.853** (was 0.828) | `ECON_GINI_FLOOR` = 0.15 | ❌ **just left the 0.60–0.85 band** — the cost of fixing turnover |
| Economy: top-10% wealth share | **0.809** (was 0.712) | — | ⚠️ in band (0.60–0.90), rising |
| Dynamics: sustained richest house | 154 045 | `late_max < 1e6` | ✅ was 297 748 before the feud rework |
| Dynamics: peak house wealth | 370 527 | finite + bounded | ⚠️ still an order above the "no 100k" ideal |
| **Province land layer** | **unmeasured by either oracle** | own tests only | ⚠️ see below |
| **Economy: tick determinism** | **PASSES** | `econ_scorecard_is_deterministic` (no longer ignored) | ✅ **fixed — 4 hash-order sites, see below** |
| **Rust tests** | **166 pass, 0 fail** (8 ignored) | CI | ✅ |
| **Frontend tests** | **0** | *none* | ❌ 33k lines uncovered |
| `cargo check` | clean | CI | ✅ |
| `npx tsc --noEmit` | clean | CI | ✅ |
| Phase 3 wall time @ 3600×1800 | ~16 s (release, 4 cores) | `bench_ocean_atmosphere` | ✅ |
| Rust / TypeScript LOC | 55.9k / 33.2k | — | — |

---

## How to reproduce every number here

```bash
# Climate fidelity — main-class, exact-zone, confusion matrix, spot checks
cd src-tauri && cargo test --lib earth_ -- --nocapture

# Economy fidelity — the full scorecard against pre-modern reference series
cd src-tauri && cargo test --lib econ_ -- --nocapture

# Economy dynamics — bounded wealth, house turnover, determinism
cd src-tauri && cargo test --lib simulate_decades_reports_dynamics -- --nocapture

# Everything
cd src-tauri && cargo test --lib
cd src-tauri && cargo check
npx tsc --noEmit

# Performance (release, slow, ignored by default)
cd src-tauri && cargo test --release --lib bench_ocean_atmosphere -- --ignored --nocapture
cd src-tauri && cargo test --release --lib ocean_atmosphere_field_checksums -- --ignored --nocapture
```

---

## The two oracles

An **oracle** is a test that answers "is this good?" without the maintainer
needing to be a domain expert. The project has two, and they are the reason any
of this is knowable:

1. **`sim/step4_climate/earth_validation.rs`** — scores the generated climate
   against the real Köppen-Geiger map (Kottek & Rubel, 0.5°). Hard-asserts
   `EARTH_MAIN_FLOOR`. **Raise the floor after every improvement** so it always
   guards the current best.

2. **`sim/campaign/tick/economy_validation.rs`** — scores the campaign economy
   against published pre-modern price, wage, urbanisation and inequality series
   (Allen, Federico, Persson, De Vries, Alfani, Van Zanden). Most metrics are
   **printed, not asserted**: a printed metric outside its historical band is a
   *finding*, not a build failure. Promote metrics to assertions as the model
   earns them.

**Track exact-zone, not main-class.** Class E scores 99.1% for free — polar is
just "cold" — which inflates the aggregate. Exact-zone is where the real state of
the climate model lives, and it is currently ungated. Adding an
`EARTH_EXACT_FLOOR` is the cheapest fidelity improvement available.

---

## ⚠️ Open defect: the campaign tick is not deterministic

`CLAUDE.md` §5 states a tick is "pure & deterministic per `(seed, tick)`". **It is
not, once the economy is actually trading.** Two identical reference worlds run in
one process produce different scorecards.

**Cause.** HashMap iteration order feeding **float accumulations**. Float addition
is not associative, and Rust's `RandomState` gives every HashMap instance its own
iteration order, so identical inputs fold to different sums. Two sites are fixed
(`classify_hubs`'s `throughput`, and `flow_year`'s ordering — both `cities.rs`);
the divergence shrank but did not vanish. Roughly a dozen accumulator maps remain
in `houses.rs`, `disease.rs`, `colonies.rs` and `mod.rs`.

**Why it hid for so long.** The existing determinism assertions in `tests.rs` run a
world where `tests::sim()` hard-codes `need_scale = 1.0` — about **84× real
demand**. Every hub sits in permanent famine, `dispatch` never sees a surplus, so
almost nothing is traded and the accumulator maps stay nearly empty. Order cannot
matter when there is nothing to order. Calibrating the reference world to real
campaign-start conditions is what exposed it.

**Consequence for this file.** Every economy number above is a single sample from a
non-reproducible process. Treat them as indicative of magnitude, not as
measurements, until determinism is restored. That is the first economy work to do.

**Fix.** Audit every hash accumulator in `tick/`, sort by key before folding, and
hold `simulate_decades_reports_dynamics` bit-identical at each step. Then remove
the `#[ignore]` from `econ_scorecard_is_deterministic`.

---

## Phase 0.4 · the law of inheritance — built, and two defects it exposed

**What was built.** Two enums on the culture (`sim/shared/inheritance.rs`): a LINE rule
(agnatic · agnatic-cognatic · absolute · enatic) and a DIVISION rule (partible ·
primogeniture · ultimogeniture · seniority · matrilineal), assigned per language kit
where the record is clear and seeded where it is not. They are read at one place —
`succeed_house` — and decide three things: who inherits (the heir's sex, and the name
bank they are drawn from), **how old they are when they do**, and whether the estate
divides.

**The age is the part that mattered most.** An heir was previously handed a fresh 45–75
year "lifespan" as their TENURE, i.e. every head was effectively born on the day they
inherited. They now inherit at an age the rule implies — an eldest son at ~27–45, a
hearth-keeping youngest at ~17–31, an elected elder at ~44–62 — and rule for what
remains of a life. That alone is what makes ultimogeniture and seniority behave
differently from primogeniture without a single extra mechanism.

**The gate.** `econ_inheritance_rules_fragment_differently` runs ONE world four times,
changing only the law:

| rule | houses ever | successions | divisions | co-heirs | mean wealth |
|---|---|---|---|---|---|
| partible | 88 | 61 | 18 | 22 | 120 325 |
| primogeniture | 55 | 57 | 0 | 0 | 195 264 |
| ultimogeniture | 49 | 45 | 0 | 0 | 164 205 |
| seniority | 124 | 147 | 0 | 0 | 103 372 |

Note what partible does **not** do: the top share and Gini do not fall, because a
division adds small firms at the bottom as fast as it trims the top. What moves is mean
wealth per house — the same capital spread over more houses. Seniority fragments by a
different route entirely: short tenures → three times the successions → far more cadet
branches.

### Defect 1 — a house's chronicle was eating its own milestones

`HOUSE_EVENTS_CAP` kept the 60 most recent events and dropped the oldest. In a hot feud
a house generates dozens of flare entries a year, so **a family lost its own founding
and every succession within a couple of years**. A 500-year dynasty's chronicle read as
three weeks of shipping losses — and it silently zeroed the division metric above, which
is how it was found. Milestones (founding, succession, division, monopoly, charter,
ruin) are now never evicted by chatter; only chatter is pruned.

This matters beyond the metric: `HOUSE_MASTER_PLAN` 2.3 concluded the chronicle IS the
product for an observation-only game. It was being deleted.

### Defect 2 — cadet branches were the new stillbirth path

With successions actually firing, the turnover diagnosis was re-run with a breakdown by
**how the dead house was founded** — and 19 of 35 deaths were cadet branches, 74% of
which never traded, dead at a mean age of 8 years. `found_branch` endowed a branch with
30% of the parent's wealth **and** `initial_fleet`'s two or three vessels it had never
paid for. That is precisely the arithmetic Phase 0.2 found behind the original 12-year
house, arriving through a second door. A branch now inherits capital only and buys hulls
from it when its trade justifies them.

Effect: mean firm lifespan **29.4 → 36.8 yr**, real-firm mean age at death 7.7 → 19.2.

### What is still open here

- Co-heir houses are **100% stillborn** when they die (8 of 28 deaths, mean age 7.2 yr)
  and branches are still 86%. They have capital and no fleet, so the endowment is not
  the cause this time — a new house appears to have no way to originate trade at its own
  seat. That is the next turnover question, and it is a *diagnosis* task, not a constant
  to tune.
- **Top-10% wealth share fell out of band from below (0.422 vs 0.60–0.90).** The
  merchant elite is now too flat. This is the mirror image of the Phase 0.2 finding and
  points the same way: at Phase 3, which is supposed to make the top of the distribution
  fragile rather than making the bottom crowded.

---

## Phase 0.1 · house turnover — diagnosed, fixed, and the cost measured

**The finding.** A house was born with `wealth: 1.0` and a two-to-three vessel fleet
costing ~0.70–1.05/month. That is ~1.4 months of runway at birth, so it went negative in
its second month, `update_solvency` ran its twelve-month clock, and it died at ≈13.4
months. Measured median age at death: **1.1 years** — the arithmetic to two significant
figures. **73% of all dissolutions were houses that never traded at all.** The
`dissolutions/century` metric was therefore counting *stillbirths, not failures*.

**My hypothesis was wrong.** I predicted overextension from ambition, i.e. a negative
correlation between age at death and committed upkeep. Measured: **+0.802** — houses that
committed more upkeep lived *longer*. The fatal commitment was the founding endowment, not
accumulated ambition.

**The fix.** Not a bigger constant. `maybe_found_house` already requires a guild at the
hub, so the seed capital is taken **from that guild** — a family separating out with its
share, as it historically did. Three properties: no money is created; a guild too poor to
endow a viable family cannot spawn one (churn stopped at source); and the seed scales with
how rich the local trade actually is.

**Result:** mean firm lifespan **~12 yr → ~51–101 yr** (band 30–90); dissolutions/century
312 → 10.

**Two things this exposed, both worth more than the fix:**

1. **`dissolutions/century` is the wrong metric.** It scales with how many houses are
   standing, so the same mortality reads differently in a 20-house and a 50-house world.
   And a 60-year run cannot observe a 90-year lifespan — the survivors are right-censored.
   The correct estimator is a hazard over exposure: `deaths ÷ house-years lived`, using the
   living houses' time instead of discarding it. That is what the lifespan row above reports.

2. **The determinism defect blocked further tuning — and is now FIXED (Phase 0.3).**
   Three runs of the same test on the same binary gave **11, 11, 6** deaths and lifespans of
   **51.1, 51.1, 101.2 yr** — a 2× swing straddling the band boundary. Four sites were
   folding or ordering by HashMap iteration order:

   | Site | What it broke |
   |---|---|
   | `money.rs::update_currency_baskets` | summed a partner-volume map with `+=` and divided every basket weight by that total; float addition is not associative, so the coin basket flipped |
   | `production.rs::fold_trade_year` | pushed new series onto `trade_hist` in map order; the peak sort is *stable*, so equal peaks kept insertion order and a different set survived truncation |
   | `mod.rs` culture desire | built `hub_desire[h]` as a `Vec` from a map |
   | `colonies.rs::update_lingua_franca` | iterated components in map order **and** resolved the dominant-culture `max_by` tie by hash order |

   Each now iterates in key order with an explicit tie-break. Three identical runs
   confirmed, and `econ_scorecard_is_deterministic` is **no longer ignored** — it is the
   guard that stops the defect returning, and any new hash accumulator in `tick/` trips it.

**Where turnover landed (final, deterministic).** Mean firm lifespan **96.9 yr** against
the 30–90 band — the overshoot is deliberate and *not* being tuned away: the remaining gap
is that **established firms almost never fail** (193.8 yr excluding stillbirths), and the
honest fix for that is a failure mechanism (the Phase 3 crisis layer), not a smaller seed
constant. Shrinking the seed would re-introduce the stillbirths that caused the original bug.

**The cost, measured: `HOUSE_MASTER_PLAN`'s open risk was real.** Wealth Gini rose
0.828 → **0.853**, just outside the 0.60–0.85 band, and the top-10% share rose
0.712 → 0.809. Houses dying young *was* partly load-bearing: it was destroying wealth in an
economy that compounds at 1.5%/yr with no other brake. So the two anomalies were **in
tension, not one bug**, and the phase boundary in that plan is wrong — Phase 0.2 needs the
Phase 3 crisis layer as its replacement brake, and the two must be co-tuned.

---

## The province land layer is unmeasured by both oracles

`province_land_pass` (FIX_PLAN B1) closes the world↔campaign feedback edge — a
province's surplus reaches its seat city's granary and its dues reach that city's
treasury. Neither fidelity oracle sees it:

- **`simulate_decades_reports_dynamics` seeds no provinces**, by design. That is what
  makes the land layer provably free of side effects on the base economy
  (`province_land_pass_is_a_noop_without_provinces` asserts it), but it also means the
  standing dynamics run says nothing about whether the land behaves.
- **`economy_validation.rs` seeds no provinces either**, so urbanisation, grain prices
  and real wages are all still measured on a world whose countryside is only a
  population reservoir.

What covers it today is four of its own tests (feedback edge + bounds, the no-op gate,
works cost money and take years, unfunded work stalls). What would actually measure it
is a province-seeded variant of the economy harness — the urban-share drift row above
(0.100 → 0.997, the countryside emptying completely) is precisely the metric a working
supply shed should move, and it is the obvious next thing to ask of this layer.

---

## What is still unmeasured

Being explicit about this matters as much as the table above — an unmeasured
subsystem is one you cannot have an opinion about.

- **The entire frontend.** 33k lines, zero tests. `tsc --noEmit` proves the types
  agree with each other, not that anything works.
- **Rust ↔ TypeScript type drift.** `types/campaign.ts` hand-mirrors Rust serde
  structs. A field rename produces a silent runtime `undefined`, not an error.
- **Peak memory.** 26M cells × 25+ columns on "Large" worlds. Time is benchmarked;
  memory is not, and memory is the likelier failure on a customer's machine.
- **Frame rate.** No measurement of pan/zoom under load with overlays enabled.
- **Save-format forward compatibility.** The v2 self-describing blob design is
  sound, but a compatibility claim with no old-save fixture behind it is a hope.
- **Anything about the app as a product** — install success, first-run
  completion, time to a finished world.

---

## History

| Date | Commit | Earth main | Earth exact | Rust tests | FE tests | Note |
|---|---|---|---|---|---|---|
| 2026-07-29 | `936a8a3`+ | 66.3% | 29.1% | 159 | 0 | Economy oracle added; CI added; scoreboard created |
| 2026-07-29 | *this* | 66.3% | 29.1% | 159 | 0 | Harness calibrated to real campaign start; LOD sampler fixed; tick determinism defect found |
| 2026-07-30 | *this* | 66.3% | 29.1% | 166 | 0 | Phase 0.3: tick determinism FIXED (4 hash-order sites); guard un-ignored |
| 2026-07-30 | *this* | 66.3% | 29.1% | 165 | 0 | Phase 0.1/0.2: firm lifespan ~12 → ~51–101 yr (seed capital from the parent guild); Gini 0.828 → 0.853 (left band — measured cost); determinism defect promoted to a blocker |
| 2026-07-29 | *this* | 66.3% | 29.1% | 165 | 0 | Feuds elaborated (cause/stage/ending); province LAND state + B1 feedback edge; sustained richest 297 748 → 154 045; Gini 0.771 → 0.828 |
| — | `d53fdc9` | 66.2% | 29.0% | — | 0 | FIX_PLAN baseline |
