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
| **Economy: price/distance gradient** | **0.069** | `ECON_INTEGRATION_FLOOR` = 0.05 | ✅ asserted (barely) |
| Economy: grain price CV across cities | 0.372 | `ECON_SPATIAL_CV_FLOOR` = 0.01 | ✅ in historical band |
| Economy: grain price CV within a city | 0.701 | — | ⚠️ above band (0.30–0.50) |
| Economy: rank-size (Zipf) slope | −0.422 | band [−3.0, −0.15] | ⚠️ flatter than −0.8…−1.2 |
| Economy: urban share drift (60 yr) | 0.100 → 0.183 | — | ⚠️ ~10× historical rate |
| Economy: house dissolutions / century | 53.3 | — | ⚠️ vs 1–3 generations |
| Economy: crisis-year share | 1.000 | — | ⚠️ permanent subsistence crisis |
| **Rust tests** | **159 pass, 0 fail** (7 ignored) | CI | ✅ |
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
| 2026-07-29 | *this* | 66.3% | 29.1% | 159 | 0 | Economy oracle added; CI added; scoreboard created |
| — | `d53fdc9` | 66.2% | 29.0% | — | 0 | FIX_PLAN baseline |
