# WorldForge 2 — Scoreboard

**The project in twelve numbers.** 89k lines across climatology, economics,
rendering and UI is more than anyone can hold as code. It is easy to hold as a
table of measurements. That is what this file is for.

Append a row every session that moves a number. Never edit an old row — a
scoreboard whose history is rewritten cannot show a regression.

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
| Economy: house wealth Gini | 0.828 | `ECON_GINI_FLOOR` = 0.15 | ✅ in historical band (0.60–0.85) |
| Economy: house dissolutions / century | 312 | — | ⚠️ needs a denominator to interpret |
| Dynamics: sustained richest house | 154 045 | `late_max < 1e6` | ✅ was 297 748 before the feud rework |
| Dynamics: peak house wealth | 370 527 | finite + bounded | ⚠️ still an order above the "no 100k" ideal |
| **Province land layer** | **unmeasured by either oracle** | own tests only | ⚠️ see below |
| **Economy: tick determinism** | **FAILS** | `econ_scorecard_is_deterministic` | ❌ **open defect — see below** |
| **Rust tests** | **165 pass, 0 fail** (8 ignored) | CI | ✅ |
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
| 2026-07-29 | *this* | 66.3% | 29.1% | 165 | 0 | Feuds elaborated (cause/stage/ending); province LAND state + B1 feedback edge; sustained richest 297 748 → 154 045; Gini 0.771 → 0.828 |
| — | `d53fdc9` | 66.2% | 29.0% | — | 0 | FIX_PLAN baseline |
