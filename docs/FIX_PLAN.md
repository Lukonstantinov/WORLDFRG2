# WorldForge 2 — Fix Plan

**Status:** analysis complete, nothing implemented yet.
**Baseline measured:** `cargo test --lib earth_ -- --nocapture` on commit `d53fdc9`.
**Revision 2:** B1 rescoped after verifying the province layer is already wired
end-to-end (see B1); sequencing updated accordingly.

This plan is grounded in the Earth-validation harness, not in impressions. Every
climate item below names the confusion-matrix cell or spot-check it is meant to
move, and the regression gate that proves it moved.

---

## 0. The measured baseline

```
═══ Earth Köppen validation (720×360, area-weighted) ═══
  main-class agreement : 66.2%   (A/B/C/D/E)     [floor: 63.0%]
  exact-zone agreement : 29.0%
  by reference main class:
    A: 80.8%   B: 65.8%   C: 33.8%   D: 49.9%   E: 99.1%

  confusion  ref↓ gen→    A     B     C     D     E
    A                    81%   10%    9%    0%    0%
    B                    10%   66%   11%    6%    7%
    C                     5%   39%   34%   13%   10%
    D                     0%    1%    9%   50%   40%
    E                     0%    0%    0%    1%   99%
```

Named-region spot checks (generated vs. real):

| Site | Gen | Ref | Precip (gen) | Precip (real) | Summer frac |
|---|---|---|---|---|---|
| Bangladesh 24N90E | B | A | **62 mm** | ~2000 mm | 52% |
| China-South 25N113E | B | C | 285 mm | ~1700 mm | 84% |
| SE-US 34N84W | B | C | 342 mm | ~1300 mm | 40% |
| India-Mumbai 19N73E | C | A | 865 mm | ~2200 mm | **95%** |
| NWEurope 52N5E | D | C | 955 mm | ~800 mm | 49% (T = **7.8 °C**, real ~10) |
| Med-Rome 42N12E | C | C | 1179 mm | ~800 mm | 47% |

### What the numbers say

Three findings drive the whole climate section of this plan:

1. **The failure is moisture *amount*, not seasonal *phase*.** Mumbai resolves 95%
   summer precipitation — the monsoon is detected essentially perfectly — and still
   delivers a third of the real water. Bangladesh at 62 mm against ~2000 mm is not a
   seasonality bug. Moisture dies in transit and nothing puts it back.

2. **`C → B` at 39% is the single largest error in the model.** Temperate land is
   *more likely to be classified arid than temperate* (39% B vs. 34% C). C is the
   worst-scoring class at 33.8%, and it is the class most of human history happens in.

3. **`D → E` at 40%** — continental reads as polar; the high-mid latitudes run cold.
   NW Europe confirms it directly: `gen=D ref=C` at 7.8 °C where reality is ~10 °C.

Also note: **track exact-zone (29%), not main-class (66.2%).** E scores 99.1% for
free on 8 671 weight units — polar is just "cold" — which inflates the aggregate.
Exact-zone agreement is where the real state of the model lives.

> **Correction to an earlier read.** Monthly resolution was initially assumed to be
> the fix for the monsoon subtropics. The spot checks disprove that: phase is already
> right, magnitude is wrong. Monthly resolution stays on the plan (item A3) but it is
> what converts *amount*-accuracy into *zone*-accuracy — it is not the first move.

---

## Part A — Climate fidelity

### A1. Conserved moisture budget with evapotranspiration recycling ⭐ start here

**Targets:** `C → B` 39%, the four too-dry spot checks.
**Files:** `sim/step3_ocean_atmo/precipitation.rs`

Precipitation is currently advection-decay plus additive bonuses (ITCZ, orographic,
frontal, monsoon, jet). Moisture is emitted at the coast and decays along the wind
with `EFOLD_TROP_KM = 1300` / `EFOLD_MID_KM = 1700`, but it is never tracked as a
conserved mass, and **nothing replenishes it inland**. On Earth roughly half the rain
over the Amazon and the Congo is recycled from upwind evapotranspiration — that is the
mechanism keeping continental interiors wet, and it is absent here.

Do:
- Track moisture as a conserved quantity along each advection path: what precipitates
  out leaves the parcel; what does not, continues.
- Add a recycling term — a fraction of land precipitation re-enters the parcel as
  evapotranspiration, scaled by temperature and vegetation/soil-moisture proxy.
- With recycling supplying interiors, `MOISTURE_FLOOR = 0.09` should no longer be
  load-bearing. It is currently propping up deep interiors that no packet reaches
  (see the comment at `precipitation.rs:62`); treat its removal as the success signal.

Do **not** simply raise the e-folding distances. That trades one hand-tuned constant
for another and will over-wet the genuine deserts (the `B` row is already only 66%).

**Gate:** `C → B` below 25%; Bangladesh/China-South/SE-US above 800 mm; Sahara and
Arabia remain `B` (the existing asserted spot checks must not regress).

---

### A2. Maritime temperature damping

**Targets:** `D → E` 40%, NW Europe `gen=D ref=C`.
**Files:** `sim/step3_ocean_atmo/temperature.rs`, `sim/step4_climate/koppen.rs`

Coastal damping is gated on `ocean_dist < 0.1 && upwind_is_open_ocean(buf, x, y, 6)`
and pulls only 45% of the way toward 15 °C. On the maritime coasts it was written for
it is under-firing: NW Europe lands at 7.8 °C and classifies continental.

Two suspects, both worth measuring separately before changing anything:
- The 6-cell upwind probe is short at 0.5° resolution (~330 km) — a westerly coast
  should feel open ocean much further upwind.
- The continental-shelf exclusion (shelf water not counting as open ocean) may be
  disqualifying exactly the broad-shelf NW-European case it was meant to protect.

Also here: the lapse rate is a flat 5 °C/km everywhere. Real lapse runs ~4 in the
saturated tropics and ~9 in dry air. Coupling it to humidity is cheap and shows up on
every mountain range.

**Gate:** `D → E` below 25%; NW Europe reads `C`; Med-Rome stays `C`.

---

### A3. Seasonal resolution — 2 states → 4 or 12

**Targets:** exact-zone 29%.
**Files:** `sim/step3_ocean_atmo/seasonal.rs`, `sim/step4_climate/koppen.rs`

Everything seasonal collapses to `sun_sign = ±1` (≈July/≈January), stored as an annual
mean plus a `precip_summer_frac` byte. Köppen's third letter (`s`/`w`/`f`) is defined
on *monthly* extremes, so it currently comes from hand-coded detectors rather than
from the definition: `winter_dry_monsoon()`, `cold_override()`, `is_windward_ocean()`,
`monsoon_onshore()`.

Once A1 has the amounts right, moving to 4 (or 12) states lets the third letter fall
out of the Köppen definition directly and lets several hundred lines of detector
heuristics be deleted. Sequenced **after** A1 — running it first would tune the
detectors against wrong magnitudes.

**Gate:** exact-zone above 40% with main-class not regressing.

---

### A4. Pressure field (larger, optional)

**Files:** new module under `sim/step3_ocean_atmo/`

Winds are prescribed from latitude (`belt_wind`) plus a monsoon perturbation. There is
no pressure field and no geostrophic balance anywhere in the model. This is the root
cause of the detector sprawl in A3 — with no pressure you cannot have the Siberian
High as an *object*, so it becomes a boolean that hunts for "a giant landmass with
open ocean to its east."

A single-layer pressure solve would make the winter monsoon, the Mediterranean regime
and the subtropical highs emerge from one mechanism. Large; only worth it if A1–A3
leave the remaining error concentrated in monsoon/continental regimes.

---

### A5. Interannual variability ⭐ high value, low cost

**Files:** `sim/campaign/tick/production.rs`, new per-year climate state

The climate is a single deterministic steady state. There is no ENSO/NAO analogue, no
volcanic year, no multidecadal drift. Meanwhile the campaign's `drought` and `bumper`
events (`production.rs:1043`) are **uncorrelated per-hub dice rolls** — not dry years
in actual places.

That is backwards for this project. Harvest variance is the engine of pre-modern
history, and you have already built the entire apparatus that should respond to
correlated harvest shocks — granaries, food reserves, grain speculation, futures,
banks, unrest, migration — and are feeding it white noise.

Minimal version: one or two global oscillation indices with multi-year persistence,
plus a per-province anomaly derived from them. Replace the `drought` dice roll with a
lookup against that state. Small change, disproportionate effect on how alive the
world feels.

**Depends on:** B1 for the per-province carrier — but note the province state
(`prov_*` on `CampaignSim`) already exists and is already advanced yearly, so the
anomaly can be added as one more field on a working structure. Can also ship
standalone at hub level if B1 slips.

---

## Part B — One simulator

### B1. Two-way world ↔ campaign coupling via provinces ⭐ highest leverage

**Files:** `sim/shared/provinces.rs`, `commands/campaign_commands/lifecycle.rs`,
`sim/campaign/tick/production.rs`

`campaign_start_sim` reads an `EconomySnapshot` out of `metadata`, builds a
`CampaignSim`, and **from that moment the campaign never touches a tile again** — a
tick is hub-level math only. That was the right performance decision and it is why
500-year runs work. The consequences:

- Climate can never affect history (drought is a dice roll, not a dry year somewhere).
- History can never affect the land — no deforestation, no soil exhaustion, no
  irrigation, no siltation. Five centuries pass and the world is pixel-identical.
- Colonies and expeditions cannot discover anything that was not in the snapshot.

The naive fix (campaign reads tiles per tick) would destroy the performance. The right
fix is **provinces**, and far more of it is already built than it first appears.

**What already exists (verified — do not rebuild):**
- `sim_generate_provinces` partitions land on watersheds/coasts and persists both the
  province list and a downsampled `province_raster` to `metadata`.
- `campaign_start_sim` already seeds per-province campaign state:
  `prov_rural`, `prov_cap`, `prov_culture`, `prov_seat`, `prov_net_mig`,
  `prov_neighbors`, and `hub_province` (exact via the raster, nearest-seat fallback).
- `province_demography_pass()` already runs **yearly** — rural pools grow toward
  carrying capacity and migrate into cities. Cross-province plague hop already uses
  `prov_neighbors`.
- The whole layer is serde-defaulted and every routine early-returns on empty, so the
  dynamics test (which never seeds provinces) is untouched. There is already a bounded
  regression test: `province_demography_feeds_cities_and_stays_bounded`.

So the carrier, the hub→province mapping, the yearly cadence and the
safe-on-empty pattern are all **already in place**. `prov_rural` is proof the pattern
works: a mutable per-province quantity the campaign advances every year.

**What is actually missing** is narrower than it looks:
- **Land state.** Add forest cover, soil depletion, irrigation, cleared land, water
  table alongside `prov_rural`, following the exact same serde-default + early-return
  convention.
- **The feedback edge.** Nothing currently reads province state back into *production* —
  `prov_rural` feeds population, not yield. Hub production must read the land state so
  land use changes output.
- **A climate anomaly slot** on the same struct, for A5 to write into.
- **(Optional, later)** persisting land state back to tiles on save, so the map itself
  visibly changes over centuries. Not required for the simulation loop to close.

Cost is `O(provinces)` annually — negligible against a 365-tick year. Framed correctly
this is an **extension of a working pattern**, not new architecture, which makes it
considerably cheaper than its position in this plan suggests.

**Gate:** `simulate_decades_reports_dynamics` still bounded and finite (it seeds no
provinces, so it must be bit-identical); `province_demography_feeds_cities_and_stays_bounded`
still passes; per-tick cost unchanged within noise (`bench_campaign_tick`).

---

### B2. Player agency — the decide/apply split ⭐ cheapest big win

**Files:** `sim/campaign/tick/polis.rs`, `money.rs`, `houses.rs`, `colonies.rs`

There are 60+ campaign commands and **exactly one mutates the running simulation**:
`campaign_advance(ticks)`. The UI is play/pause and week/month/year. The current model
is: build a world, freeze it, press play, read panels.

The useful observation is that **every AI decision function is a latent player verb,
and they are all already written** — `decide_polis_policy` (council/tariff/mint/
treasury), `decide_coinage`, house dispatch, bank lending, colonisation, office
leasing, war goals. Refactor each into:

```rust
fn decide_X(&self, ...) -> XChoice     // AI proposes
fn apply_X(&mut self, c: XChoice)      // sim disposes
```

A player then supplies the `XChoice` instead of the AI. Near-zero new simulation, the
entire interaction surface unlocked at once, and the AI becomes unit-testable as a
side effect.

Three tiers — **recommend tier 2**:

1. **Observer+** — keep it autonomous, add what-if nudges: pause, set a tariff,
   embargo a lane, found a city, trigger a drought, resume.
2. **Play a house** ← *this one.* A `House` is already a complete agent: wealth,
   fleets, offices, warehouses, contracts, monopolies, per-city influence, bailos,
   council seats, rivals, archetype, named head with a lifespan, heraldry, chronicle.
   The gap between what exists and a merchant-republic game is smaller than the gap
   already closed.
3. **Play a polis** — same trick on `decide_polis_policy`; wants B4 (diplomacy) to be
   interesting.

**Gate:** with the AI supplying every choice, `simulate_decades_reports_dynamics` must
produce bit-identical output to today. That is what proves the refactor was pure.

---

### B3. Wire the Pops layer in ⭐ built and inert

**Files:** `sim/campaign/tick/cities.rs`, `production.rs`

```rust
pub consciousness: f32,  // 0..10 political awareness
pub militancy: f32,      // 0..10 willingness to revolt
```

`hubs[h].pops` is written once a year (`cities.rs:309–347`) and read **only** by
`campaign_get_pops` for display. Nothing in the simulation consumes it. The entire
Victoria-style layer is a rendering of the abstract `Society` shares, not a driver;
`militancy` and `consciousness` are computed and discarded. The source says so plainly:

> *"NOT yet wired into consumption/politics (that is DLC 4 step 2); kept read-only."*

The data model is already built and already correct. It needs consumers: per-profession
consumption baskets, `militancy` feeding unrest instead of the abstract shares,
`consciousness` gating political events.

**Gate:** dynamics test stays bounded; unrest/revolt frequency does not spike.

---

### B4. Diplomacy, treaties, leagues

Wars flare between rival poleis with no treaties, alliances, or leagues (no Hanse).
`update_wars` has war goals and reparations but no negotiated state. Medium cost;
mostly valuable as the thing that makes B2 tier 3 worth playing.

---

## Part C — Closing the economy

The economy currently **cannot generate its own growth**. Three holes, and they
compound:

### C1. Capital goods
Ships, looms, mills, kilns, tools are not goods. `fleet_sea` is a `u32` a house buys
with abstract wealth. With no investment good there is no capital accumulation, so
growth is:

```rust
const PROD_GROWTH_PER_YEAR: f32 = 0.015;
self.tech_factor *= tech_daily;   // 1.5%/yr, compounded, forever
```

A single exogenous scalar is the entire technology and growth model. Banks, credit,
futures, warehouses and monopolies all redistribute an output whose growth rate nobody
can influence. A house that corners a market or a polis that invests its treasury
cannot move it, because it is not computed from anything.

### C2. Fuel
No coal, charcoal or firewood as an input to anything. Every pre-industrial manufacture
that matters — glass, metalware, ceramics, salt-boiling, brewing, brick — is
fuel-limited, and that is *why* those industries cluster near forests and later migrate
to coal. `timber` exists as construction but never as energy, so manufacturing has no
location logic beyond labour ∝ population.

### C3. Labour market and wages
`labor` is proportional to population and free. There are no wages. `commoner_wealth`
is *derived* from prosperity rather than *earned*, so the social strata cannot respond
to economic conditions through the actual historical mechanism. (Already ranked #1 in
`FUTURE_SYSTEMS_PLAN.md` — concur; it is the missing link between economy and society,
and the natural partner to B3.)

### C4. Chain depth
Chains are at most 2 deep and mostly 1 (`jewelry ← gold + silver + gemstones` is a
single hop). The DAG resolver in `manufacture.rs` supports arbitrary depth; the shipped
library does not use it. No ore → ingot → tool, no flax → thread → linen → sailcloth.
Depth is what creates intermediate cities that add value without owning any raw
resource — precisely the Flanders/Lombardy dynamic the project is reaching for.

### C5. A frozen good list
The 45 belts plus ~21 manufactures are fixed for all time. No discovery, no new-world
crops arriving, no good going obsolete. The colonisation system cannot bring home
anything that was not already on the map.

**Sequencing note:** C1–C3 are one coherent change, not three. Doing any one alone
leaves the loop open. This is the largest item in the plan; schedule it as a block or
not at all.

---

## Part D — The social layer

Strong already: culture hearths spreading by least-cost flood-fill so borders fall on
real geography, 14 traits, assimilation modulated by language-family kinship + lingua
franca + trait resistance + prestige, creole genesis, homophily in migration, minority
quarters that stir unrest only in already-stressed cities, and grievance accumulating
across years so chronic misery boils over without an acute spike.

Gaps, in cost order:

- **D1. Kinship graph.** Houses have `head_name`, `generation` and `rivals` — no
  marriages, no children, no alliances (`rivals` has no `allies` counterpart).
  Merchant-republic politics *is* marriage politics; the Medici/Fugger/Hanse networks
  were kinship networks with ledgers attached. Houses can currently only compete,
  never combine.
- **D2. Religion as a system.** Holy sites are pilgrimage flavour; "Devout" is a
  culture trait. No faith identity separable from ethnicity, no schism, no confessional
  minority. Expensive omission *for this project specifically*, because religion is the
  missing input to systems already built: usury prohibitions are why banking
  concentrated in particular communities, confessional diasporas are why trust networks
  spanned hostile borders, pilgrimage was a trade route.
- **D3. Figures who decide.** `Figure` is a name, a `kind`, a hub and a death date.
  Nobody in this world makes a choice a person made.
- **D4. Bound labour** (slavery/serfdom) — large historical omission, and downstream
  of C3.

---

## Sequencing

**Do first — highest value per unit of work:**

| Order | Item | Why here |
|---|---|---|
| 1 | **A1** moisture budget | Largest measured error (`C→B` 39%); self-contained; hard gate |
| 2 | **B2** decide/apply split | Unlocks all interaction; provable by bit-identical dynamics |
| 3 | **B3** wire Pops in | Data model already built and correct; needs consumers only |
| 4 | **B1** land state on provinces | Cheaper than it looks — carrier + yearly cadence already exist |
| 5 | **A5** interannual climate | Rides B1's carrier; turns climate into a driver of history |
| 6 | **A2** maritime damping | Second measured error (`D→E` 40%); small, targeted |

B1 and A5 moved ahead of A2 on a second pass: the province layer turned out to be
**already wired end-to-end** (seeded at campaign start, advanced yearly, regression-
tested), so B1 is an extension of a working pattern rather than new architecture — and
A5 then costs little on top. A2 is unchanged in scope, just less urgent than two items
that together close the world↔campaign loop.

**Then:** A3 (seasonal resolution) → D1/D2 (kinship, religion) → C1–C3 as a block.

**Deliberately deferred:** A4 (pressure field), B4 (diplomacy), C4/C5, D3/D4.

### A note on scope

There are 32 planning documents in `docs/`. The bottleneck on this project is not
ideas — there are far more good ideas written down than anyone can build. It is scope
discipline. The recommendation is to land **items 1–3 above** and nothing else before
re-evaluating. Each is independently shippable, each has a hard regression gate, and
together they move fidelity, interaction and the social layer at once.

---

## Regression gates — run these on every change

```bash
cd src-tauri
cargo test --lib earth_ -- --nocapture                             # climate fidelity
cargo test --lib simulate_decades_reports_dynamics -- --nocapture  # economy dynamics
cargo check
cd .. && npx tsc --noEmit
```

Raise `EARTH_MAIN_FLOOR` (`sim/step4_climate/earth_validation.rs`) after every
improvement so it always guards the current best. Consider adding an
`EARTH_EXACT_FLOOR` — exact-zone is the number that actually tracks model quality, and
it is currently ungated.

The four too-dry spot checks (Mumbai, Bangladesh, China-South, SE-US) are printed but
**not asserted**. Promote them to assertions as A1 fixes them — that converts the
current tuning frontier into permanent protection.
