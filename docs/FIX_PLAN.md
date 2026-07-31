# WorldForge 2 — Fix Plan

**Status:** A1 (moisture recycling) implemented; B2 (decide/apply split) started
(polis/coinage/fleets done); B3 (Pops wired into unrest) first pass done — see
status notes below. Rest of the plan not yet started.
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

**Status: partially implemented.** Landed two changes in `precipitation.rs`:

1. **ET recycling** (`ET_RECYCLE_MAX = 0.5`): along each ocean-emitter's advection
   ray, a fraction of what a step rains out re-enters the parcel instead of
   vanishing, gated by local warmth (Tetens ratio) and by how much of the parcel's
   original supply has already fallen upwind (0 at the coast, rising inland). Bounded
   — the parcel's moisture still strictly decreases — so no runaway feedback.
2. **Delta-mouth monsoon-onshore fix**: `monsoon_onshore` was breaking on the FIRST
   suppressed ("enclosed sea") ocean cell it hit, which falsely flagged the Bay of
   Bengal's mouth (locally narrow at the Ganges-Brahmaputra delta) the same as a
   truly enclosed sea (Red Sea/Persian Gulf), zeroing the monsoon onshore gate for
   Bangladesh. Now probes a short additional distance (`SUPPRESSED_PROBE_KM = 220`)
   past a suppressed hit for real open water before giving up — short enough that a
   genuinely enclosed basin (which stays suppressed far past the probe) still blocks
   Arabia/Yemen correctly.

Measured effect (`cargo test --lib earth_ -- --nocapture`):
- Main-class agreement 66.2% → 66.3%, exact-zone ~flat (28.9%). `EARTH_MAIN_FLOOR`
  raised 63.0 → 65.0.
- `C` row (own-class accuracy) 33.8% → 35.3%; `C → B` confusion 39% → 37% — moving,
  not at the 25% gate.
- Deep-tropics moisture measurably increased (Amazon 1663→1723 mm, Congo 1049→1145
  mm, both still asserted `A`), and the required Sahara/Arabia/Amazon/Congo/
  Indonesia/Vietnam spot checks all still pass.
- Bangladesh/China-South/SE-US precip roughly tripled (62→232, 285→425, 342→496 mm)
  from the onshore-gate fix alone (confirmed by isolating `ET_RECYCLE_MAX = 0` —
  identical numbers), but all three are still short of 800 mm and still classify
  `B`, not `C`/`A`.

**Why the gate isn't fully met:** investigation traced Bangladesh's remaining
shortfall to the seasonal wind field itself (`seasonal.rs`), not the moisture model —
the simulated summer wind just offshore of the Bangladesh coast still blows
south-westerly (the trade-wind belt direction) rather than turning onshore; the
monsoon thermal-low perturbation (`MONSOON_WIND_GAIN`) is too weak to reverse it
there. Tried raising `MONSOON_WIND_GAIN` (0.10 → 0.22 → 0.40): it helped Mumbai a lot
(865 → 1233 mm) but left Bangladesh essentially unchanged and *lowered* global
main-class agreement (66.3% → 66.0%, with knock-on wetting/drying elsewhere from the
renormalized wind headings) — a net regression, so it was reverted. That confirms
this is really **A4 territory** (no pressure field ⇒ no real thermal-low object to
anchor the monsoon reversal) rather than an A1 moisture-budget gap; the 800 mm gate
for these three sites should be re-attempted once A4 exists, not by further tuning
`ET_RECYCLE_MAX` or `MONSOON_WIND_GAIN` in isolation (both plateau/regress quickly —
tested up to `ET_RECYCLE_MAX = 0.85` with no further C→B movement).

**Known pre-existing failure (unrelated):** `precipitation::tests::
meridional_ridge_shadows_its_lee` fails on this baseline independent of the A1
changes (confirmed via `git stash`) — windward/lee ratio measures 2.39x against a
2.5x assertion. Not touched here; flagged for separate investigation.

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

### A6. Ocean current fidelity — ACC dead on Earth-shaped worlds ⭐ cheap, isolated

**Files:** `sim/step3_ocean_atmo/ocean.rs` (`generate_ocean_currents`, `gyre_vector`)

Requested analysis: are currents "eligible and moving where they're supposed to,"
and does their speed/direction correctly couple into precipitation/temperature via
warm/cold tagging and salinity? Spot-checked named real currents through the Earth
harness (`run_earth()`), each printed as `(current_type, vx, vy, speed, sst, salinity)`:

- **Confirmed working:** Gulf Stream (35N70W) reads WARM, strongly poleward
  (`v=(+0.37,-2.29)`, speed 2.32 — a vigorous boundary current), matching the real
  current's direction and relative strength. Western-vs-eastern intensification
  (`SPEED_BOUNDARY_WEST=2.2` vs `..._EAST=0.55`), the salinity→current-speed
  feedback (`advect_salinity_and_recouple`'s gradient boost, ±30%) and the
  current→temperature coupling (`temperature.rs`'s `vol.clamp(0.35, 1.3)` reach/
  decay) are all present and directionally sound; the `vol` floor of 0.35 means
  even a current-type cell with a locally near-zero vector (the tag can be smeared
  a few cells wide by `extend_warm_tag`'s perpendicular corridor) still contributes
  a non-zero, bounded thermal anomaly — not a bug, a deliberate guard.
- **Confirmed broken: the Antarctic Circumpolar Current never activates on a
  real Earth-shaped world.** `generate_ocean_currents` special-cases the ACC with
  a single global gate: `circumpolar_active` requires the *entire* latitude row at
  `y = height × (1 − CIRCUMPOLAR_FRAC)` (90% down, ≈72°S) to be 100% ocean at
  *every* longitude before any cell anywhere gets the hardcoded eastward
  `SPEED_ACC = 1.6` flow. On the real Earth fixture that row is 431/720 cells
  (60%) **land** (Antarctica), so `circumpolar_active` is `false` for the whole
  planet and every Southern Ocean cell falls through to the ordinary basin-based
  `gyre_vector` path instead — which, sampled at 60°S/0°E, produces a weak
  (`speed≈0.68`) and even westward (`vx=-0.29`) drift, the opposite direction of
  the real ACC. This isn't a narrow-band placement quibble (moving the row
  equatorward to the ACC's real ~50–60°S location would make the "whole row is
  ocean" check fail even harder, since South America/Antarctica extend well
  north of there in places) — the **gate design itself** silently disables the
  entire mechanic on any world with a pole-covering or pole-adjacent continent,
  which includes Earth.

**Why not fixed here:** this session already spent six reverted attempts on a
different Somalia-region ocean/wind fix (see the print-only Somalia spot checks
in `earth_validation.rs`), each either failing to fix the target or regressing
Amazon/Congo/Indonesia/SE-US. The ACC sits in the same file and the same
current-classification code path, so a fix carries the same regression profile
and deserves the same care — done here as **diagnosis only** (a throwaway
`scratch_current_diagnostic` test was used to confirm this, then reverted; not
committed). A safe fix would replace the whole-row boolean with a **per-cell**
or **per-longitude-band** gate (e.g. "is this cell south of the nearest land at
its own longitude, by some margin" rather than "is the entire ring clear"), then
re-run `earth_koppen_agreement` to confirm the `E`-class row (99.1%, already
near-ceiling) doesn't regress before landing it.

**Gate:** `earth_koppen_agreement` main-class ≥ 65.0 (current floor) unchanged or
improved, particularly the `E` row; a new print-only spot check near 55-60°S
confirming eastward (`vx > 0`) flow once fixed.

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

**Status: land state + the feedback edge LANDED.** `province_land_pass(yr)` runs yearly
right after `province_demography_pass` and carries `prov_forest` / `prov_arable` /
`prov_pasture` / `prov_irrigated` / `prov_soil` / `prov_tenure` / `prov_tax` /
`prov_arrears` / `prov_unrest` / `prov_surplus` / `prov_revenue` / `prov_holder` /
`prov_works` / `prov_history` / `prov_events`, all serde-defaulted, all early-returning
on an empty layer. The **feedback edge is closed**: `prov_surplus` is added to the seat
city's food stock and `prov_revenue` to its treasury each year. Rural fiscality, rural
unrest and rural revolt are new — all three were absent, and unrest in particular was a
city-only property while every major pre-modern revolt was rural. Multi-year works
(clearance / drainage / irrigation / road) reuse the satellite-construction funded-progress
shape; unpaid work stalls rather than failing.

Two calibration findings worth keeping:
- **The land multiplier must be centred on 1.0** for ordinary land. The first cut
  averaged ~0.7, which put gross output below rural subsistence on decent land — so no
  province ever had a surplus and the feedback edge silently delivered nothing. A silent
  zero is the failure mode to watch for in this whole item.
- **Land use is a partition.** The fallback seeder could hand out forest 0.75 + arable
  0.38 = 1.13 of one province.

**Still open on B1:** (a) the climate-anomaly slot for A5 is NOT added; (b) land state is
never persisted back to tiles, so the map itself still does not visibly change over
centuries — the panel's year slider is the only place the change is legible;
(c) **neither fidelity oracle seeds provinces**, so the whole layer is measured only by
its own four tests. A province-seeded variant of `economy_validation.rs` is the obvious
next step — the urban-share drift row (0.100 → 0.997) is exactly what a working supply
shed should move.

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

**Status update — tier 1 has its first real verbs.** The campaign is no longer
one-mutating-verb: `campaign_set_province_tax`, `campaign_start_province_work` and
`campaign_cancel_province_work` mutate a running sim, wired to the Province Inspector's
Holdings tab. They are deliberately the *observer+* tier, and they establish the shape
for the rest: validate → call the same routine the AI would → `set_sim` + persist. Note
these verbs act on a province rather than through a `decide_X`/`apply_X` pair, because
the land pass had no AI decision to split — the tax rate and the works had no AI author
to displace. The `decide_X`/`apply_X` split is still the pattern for anything that DOES
have one.

**Status: started — `decide_polis_policy` split, pattern established.**
`polis.rs::decide_polis_policy` (council seating, tariff stance, mint-fineness
target, health-funding decision) is now `fn decide_polis_policy(&self, year) ->
Vec<PolisChoice>` (pure, no mutation) + `fn apply_polis_policy(&mut self,
choices)` (the only part that touches hub state), wired together by
`run_polis_policy` at the sole call site in `mod.rs`. `PolisChoice` is the
concrete `XChoice` the plan describes — a player who held a seat would call
`apply_polis_policy` directly with their own choice instead of going through
`decide_polis_policy`. Verified bit-identical: `simulate_decades_reports_
dynamics` output diffed byte-for-byte against the pre-refactor baseline.

`money.rs::decide_coinage` (mint charter, first coin, metal choice, bullion-cap
fineness clamp, trust target, seigniorage) is now split the same way:
`fn decide_coinage(&self, year) -> Vec<CoinageChoice>` + `fn apply_coinage(&mut
self, choices)`, wired by `run_coinage`. This one was more involved than the
polis split: several terms (the trust target, seigniorage) depend on a
mutation earlier in the SAME hub's SAME year (e.g. a charter debit changing
the treasury that year's trust-target reads), so `decide_coinage` replays the
exact per-hub arithmetic against LOCAL shadow variables (a local `treasury`,
`coin_trust`, `mint_fineness`, …) instead of mutating `self`, and returns the
hub's final resulting values (plus the journal entries it generated) for
`apply_coinage` to write verbatim — no branching left in `apply_coinage` at
all. Also verified bit-identical against the same pre-refactor baseline.

`houses.rs::manage_fleets` (per-house fleet upkeep/decay + buy-or-sell) is also
split: `fn decide_fleets(&self) -> Vec<FleetChoice>` + `fn apply_fleets(&mut
self, choices)`, wired by `manage_fleets` (kept as the tick-loop entry-point
name since it was already the call site everyone uses). Each house's fleet
choice is independent of every other house (unlike `decide_coinage`, no
cross-hub coupling — only per-house sequential steps, e.g. upkeep debited
before the buy/sell check reads the post-upkeep wealth), so this one used the
same local-shadow-variable technique as `decide_coinage` but per-house rather
than per-hub. Also verified bit-identical.

**Not yet done — and NOT all safely splittable the same way:** looked at
`houses.rs::maybe_house_invests` (estate/manufactory investment) next and
found it does NOT fit this pattern cleanly — it appends new hubs
(`create_estate` grows `self.hubs`), and later houses in the SAME tick's loop
read `estate_count()` / per-city estate caps that include estates JUST BUILT
by earlier houses in that same loop. That is cross-iteration coupling through
a growing `Vec`, not the per-hub/per-house-local coupling `decide_coinage`/
`decide_fleets` had — a naive decide-then-apply split would either have to
replicate the entire sequential mutation inside "decide" (defeating the
purpose) or risk changing which house gets which estate slot. Needs a
purpose-built approach (e.g. decide computes proposals against a snapshot,
apply resolves conflicts/order), not the same copy-paste as the other three.
Bank lending/colonisation/office leasing (`houses.rs`, `colonies.rs`) and war
goals (`war.rs`) are still unexamined for the same "safe to split?" question
and remain monolithic `&mut self` AI+mutation functions — each needs its own
change + bit-identical-diff verification pass rather than one large sweep.

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

**Status: militancy/consciousness wired in (first pass).** Two changes,
both in `cities.rs`:

1. **Broke the circularity.** `derive_pops`'s `militancy` used to be a pure
   rescaling of last year's `so.unrest` (`base_mil = so.unrest * 10.0`) —
   feeding that back into unrest would have told `update_unrest` nothing it
   didn't already know. Now `militancy` derives from THIS year's own hardship
   (`lack_basic`/`starving`/`lack_comfort`), independent of `so.unrest`, so the
   per-profession bias already in the split (labourers/soldiers +1.5, elites
   −2.0) carries genuine new information: population-weighted militancy now
   reflects profession MIX, not just the aggregate stats.
2. **Wired both into `update_unrest`.** Population-weighted militancy (0..1)
   is a new small additive term (`POP_MILITANCY_WEIGHT = 0.10`) in the unrest
   target — a city with the same inequality/wealth but an underclass-heavy
   population now reads more unrest-prone, which the old formula (only
   `ineq`/`welfare`, never the raw class shares) couldn't express. Population-
   weighted consciousness (0..1) scales grievance ACCRUAL (not cooling) between
   0.75×–1.25× (`CONSCIOUSNESS_GRIEVANCE_MIN/MAX`) — a more politically aware
   populace organizes chronic misery into revolt-triggering grievance faster,
   which is the "consciousness gating political events" the plan named.

Per-profession consumption baskets (the plan's third suggested consumer) are
NOT done — `needs_life/everyday/luxury` on `Pop` are still copies of the
hub-aggregate `lack_basic/comfort/luxury`, not profession-differentiated.
Left for a follow-up pass; the militancy/consciousness wiring was judged the
higher-value, lower-risk piece to land first (it's what the plan's gate
actually measures — unrest/revolt dynamics — while consumption baskets would
touch `production.rs`'s demand model, a much larger blast radius).

Verified: `cargo test --lib` — 115/116 pass (same pre-existing unrelated
`meridional_ridge_shadows_its_lee` failure, nothing new); `unrest_topples_
councils` (the dedicated revolt-mechanics test) still passes;
`simulate_decades_reports_dynamics` stays bounded (wealth finite, houses/
towns/coins turn over at a similar cadence to before — NOT bit-identical,
since this is an intentional behavior change, not a refactor).

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

### C1a. Measured: the compounding is unbounded past ~100 years, not just past 60
`simulate_decades_reports_dynamics` (the standing dynamics gate, §2.1) only runs a
handful of decades, so nobody had watched this economy past ~60 years before. A
150-year instrumented run of the reference world (`econ_diagnose_outpost_founding`,
`economy_validation.rs`, `#[ignore]`d — built to diagnose an unrelated outpost-founding
stall, but it happens to track richest-house wealth every month as a side effect) found
the richest house's peak wealth landing in the **billions** by year 150 (four separate
runs across small code changes: 402,665,734,144 · 29,200,267,264 · 4,405,119,488 — the
exact figure moves with the constants, the ORDER OF MAGNITUDE past a century does not),
against the project's own "no 100k blow-ups" ideal (§2.1) and the ~150k–370k peaks seen
in the 60-year scorecard runs above. This is consistent with, not a new mechanism beyond,
C1's diagnosis: `tech_factor` compounds 1.5%/yr forever with nothing to brake it, and at
60 years (1.015^60 ≈ 2.4×) that's invisible; at 150 years (1.015^150 ≈ 9.1×) compounded
through wealth-begets-wealth channels (interest, monopoly rents, feud/tier prestige) it
is not. **Not fixed here** — flagged per §2.4 ("negative results are deliverables") as
evidence for prioritising C1, and as a reason any future long-horizon dynamics run should
watch peak wealth, not just assume the 60-year shape holds.

### C2. Fuel
No coal, charcoal or firewood as an input to anything. Every pre-industrial manufacture
that matters — glass, metalware, ceramics, salt-boiling, brewing, brick — is
fuel-limited, and that is *why* those industries cluster near forests and later migrate
to coal. `timber` exists as construction but never as energy, so manufacturing has no
location logic beyond labour ∝ population.

### C3. Labour market and wages
`labor` is proportional to population and free. There are no wages. `commoner_wealth`
is *derived* from prosperity rather than *earned*, so the social strata cannot respond
to economic conditions through the actual historical mechanism. (It is the missing
link between economy and society,
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

---

### A7. The zero-wind ring at 30°/60° — REAL BUG, REVERTED (negative result)

`ocean::belt_wind` builds its field by blending three belt vectors, but `west` is
the exact negative of `trade` (and `polar` is identical to `trade`), so the blend
collapses to a signed multiple of ONE vector whose magnitude passes through zero at
each belt edge. A trailing `if mag > 0.2 { normalize }` guard then leaves every cell
within ~±0.7° of `hadley_edge` / `polar_front` at its raw near-zero length, and the
edge itself at exactly `(0, 0)`. Measured `|v|`: 1.000 at 28°, 0.186 at 29.5°,
**0.000 at 30°**.

This is not cosmetic. `compute_precipitation` skips an ocean emitter whose wind
length is < 0.01, and `jets.rs` multiplies its base speed by the same length — so
those rows emit no moisture and carry no jet, in both hemispheres. The comment on
the guard claims it exists to prevent "spurious dry/wet bands at 30°/60°". It
creates one.

**Two fixes were tried and both were reverted.** Forcing the whole sub-guard band
to unit length: **69.0 / 31.5**. Flooring only the exactly-zero rows at 0.15 —
touching cells where the blend cancels to within 1e-4 and nothing else: **69.0 /
31.4**. Baseline is 69.2 / 31.6. Both cost the same ~0.2 main-class and both move
the same rows: **B 69.0 → 68.4, C 33.0 → 32.4.**

**The finding is the number, not the fix.** The dead band sits exactly on the
subtropical desert latitude, and restoring *any* wind there wets the deserts. Part
of this model's arid belt is held up by an absence of wind rather than by
`subtropical_penalty` — which is the same conclusion A9 reaches from the other
direction. Fixing the ring honestly therefore requires the subsidence term to be
strong enough to stand without it; it is not a standalone change. Do not re-attempt
it in isolation — this is the second recorded attempt.

### A8. Ekman-based cold-current tagging — REVERTED (negative result)

`ocean.rs` decides "cold current" from basin side and latitude
(`basin_dir > 0.45 && abs_lat > 23.0`). The Somali Current is the world's one cold
WESTERN-boundary current, at 2–11°N, so it can never be tagged — which also makes
`monsoon_onshore`'s own documented guard ("cold Somali current zeros E ray")
unreachable code. Asserted by
`precipitation::tests::the_cold_current_guard_cannot_fire_in_the_deep_tropics`.

Replaced with the general physical criterion — offshore Ekman transport
(`ekman · depth_gradient < 0`, handedness from hemisphere × `rotation_sign`),
appended LAST in the classification chain so it could only add tags. Measured
**68.4 / 31.2** against 69.2 / 31.6.

The arid row *improved* (B 68.7 → 69.9), confirming the mechanism is real. The cost
was the tropics: **A 83.8 → 79.1**, because the annual-mean wind off southwest
India is alongshore, so the criterion fires there and kills the monsoon
(Mumbai 874 → 643 mm, `A → B`, with `current_type = 2` on its coast). Real
Indian-coast upwelling is a transient the seasonal model cannot separate from the
monsoon that overwhelms it.

**And it barely helped the target anyway**: Bosaso 3105 → 2734 mm against a real
~100. The reason is the more important finding — `subtropical_penalty` is **zero
below 13°**, so Pass 1b's sink is 0.00 across the whole Horn regardless of what the
cold tag says, while `monsoon_bonus` fires from 5°. **The Horn is not a tagging
problem. Between 5° and 13° the model has a monsoon source and no subsidence sink
at all**, so nothing there can be dried by the mechanism that dries Arabia. Any real
fix must extend the subsidence term equatorward, and that is a change to the
latitude structure the Earth calibration is built on — not a local patch.

### A9. Where the C→B error actually lives (measurement, no code)

Area-weighted `ref = C` cells by latitude band × east/west position in the
continent, `%B` being the error:

```
  15-25  west  66%    mid  72%    east  48%
  25-35  west  60%    mid  81%    east  69%   <- 1621 wt, the largest single block
  35-45  west   9%    mid  20%    east  27%
  45-55  west   0%                            <- C error here is C->D/E, not C->B
```

**85% of all C→B error is equatorward of 35°; 64% of it is in the 25–35° band
alone.** Poleward of 45° the C error is not aridity at all. Consequence for
planning: the "cyclones funnel moisture into continental interiors" hypothesis is
aimed at the wrong latitude — `US-Kansas 39N98W` is *already* classified C (250 mm
vs ~650 real). And an Eady-growth-rate storm track is **not buildable in this
model**: `earth_base_curve` is piecewise-linear so its meridional derivative is a
three-step function peaking at the pole, and a single-layer model has no static
stability, so both terms of σ ≈ 0.31·(f/N)·|∂u/∂z| are unavailable.

### A10. The Köppen zone census — five zones are never emitted

`earth_diagnose_koppen_zone_census` (ignored) prints every zone's generated area
share against the reference. Zones the model **never produces at all**:

```
  Dfd extreme subarctic     0.00 / 0.97 %      Dwb dry-winter cont.    0.00 / 0.82 %
  Dwa dry-winter cont.      0.00 / 0.47 %      Dwd dry-winter extreme  0.00 / 0.08 %
  Dsa dry-summer cont.      0.00 / 0.10 %      Dwc dry-winter subarc.  0.00 / 1.50 %
```

The pattern is not random. **Every missing zone is either `Dw*` or `*d`:**

- **The whole dry-winter continental family is absent** (~2.9% of land — Manchuria,
  Korea, north China, eastern Siberia). `w` requires `winter_wet < summer_wet / 3.0`
  *and* survival to the C/D branch; the monsoon belt that should produce it is
  classified `B` first (China-South reads B at 497 mm against a real ~1700).
- **Both extreme-cold `d` zones are absent** because `extreme_cold` requires
  `t_coldest < −38 °C` and the seasonal span cannot reach it: measured span at
  60–70°N is 28.6 °C against a real 57–65 °C in Yakutsk/Verkhoyansk. Same root
  cause as the D→E error — the model's continental seasonality is roughly half of
  reality.

Two more census findings worth acting on:

- **`H` (highland) is 8.07% of generated land and 0.00% of the reference.** It is a
  WF2 invention with no Köppen counterpart, scored as `E` by `main_letter`. Every
  `H` cell over reference `C`/`D` terrain is therefore a guaranteed miss, and it is
  a large part of why `ET` is over-emitted (9.91% vs 6.52%).
- **Mediterranean is nearly missing**: `Csa` 0.12% against 1.94% — 6% of what it
  should be. `Csb` 0.23×. Given §8.6 gates `Cs` on a windward coast beside a cold
  current, the gate is very likely too strict.

### A11. Orographic uplift was a step function of ABSOLUTE height — FIXED

`orographic_multiplier` tested `elevation > MOUNTAIN_THRESHOLD` (0.19 ≈ 1681 m) and
returned a flat 2.5. Measured on the real Earth fixture at 0.5°, cells clearing that
threshold in a ~500 km box (of 81):

```
  Western Ghats      0   (max 1200 m)      NZ Southern Alps   0   (max 1663 m)
  Appalachians       0   (max 1407 m)      Norway/Bergen      4   (max 2029 m)
```

Three of the wettest orographic coasts on Earth produced **no uplift whatsoever**,
and an 1800 m range and the Himalaya were treated identically. A cell-mean
elevation also falls as the grid coarsens, so an absolute threshold makes orography
**resolution-dependent** — the Earth gate runs at 720×360 while the app defaults to
3600×1800, so the gate and the shipped product disagreed about where mountains are.

Replaced with a graded response to the upslope RISE along the wind
(`w = U·∇h`, Smith & Barstad 2004), saturating at the old threshold so a
full-height range is unchanged and only previously-invisible terrain gains.
**69.4 → 69.6 main, 31.8 → 31.9 exact, C row 33.0 → 34.5.** Mumbai `B → A`,
SE-US now `C`. Sweep of the saturating rise (main flat at 69.6 throughout):

```
  0.10  exact 31.7  C 35.8  B 67.1        0.19  exact 31.9  C 34.5  B 68.1  <- chosen
  0.14  exact 31.8  C 35.0  B 67.6        0.25  exact 32.0  C 33.9  B 68.7
```

**Still open here:** the LEE/rain-shadow term is still the binary absolute test, and
nothing in the orographic response depends on wind SPEED, though `w = U·∇h` says it
should.

### A12. Continental seasonality was half of reality — FIXED (unblocks two zones)

`K_SEASONAL` 0.20 → 0.24. The generated warmest−coldest span at 60–70°N measured
**28.6 °C** against a real 57 °C at Yakutsk and 65 °C at Verkhoyansk. Two
consequences, and the first is the one that matters most:

- **`Dfd` and `Dwd` were arithmetically impossible.** Both require
  `t_coldest < −38 °C`. With a 28 °C span, `t_coldest = mean − 0.55·span` is at
  most ~15 °C below the annual mean, so no plausible mean could reach −38. The
  zones were not rare in the model; they were unreachable.
- Same root cause as `D → E`: too narrow a span puts the warmest month under
  Köppen's 10 °C polar line.

**70.6 → 70.8 main, 31.9 → 32.8 exact, D row 58.5 → 70.8, `D → E` 30% → 18%.**
`Dfd` 0.00 → 0.17%, `Dsa` 0.00 → 0.11%, `Dsb` 0.07 → 0.21% — three zones go from
never-emitted (or near it) to present, which is the independent target the sweep
was checked against rather than the score it was tuned on.

**The cost is real and is the C row: 34.5% → 31.5%**, mostly `Cfb` (4.57 → 1.66%
against a reference 3.50%) flipping to D as winters deepen. Beyond K = 0.24
main-class keeps creeping to 71.0 while exact-zone collapses to 30.4 — the D row
is bought straight out of C and E — so 0.24 is the joint maximum, not the
main-class maximum.

0.24 is still far short of the doubling the Siberian figures imply, so the
under-swing is reduced, not fixed. Going further needs the C-row cost addressed
first, most likely via `Cfb`'s maritime damping (`TAU_MARITIME`) rather than by
backing off the continental span.

### A13. Still-blocked zones after A12

`Dwa`/`Dwb`/`Dwd` remain never-emitted and `Dwc` is at 0.01% against 1.50%. A12
confirms this is **not** a seasonality problem — the whole `Dw` family is lost
upstream, in the aridity test. The East Asian dry-winter belt is classified `B`
before it can reach the C/D branch (China-South 25N113E reads `B` at 497 mm
against a real ~1700), so the `w` third letter never gets a chance to apply. Fixing
`Dw` means fixing the `C → B` over-aridity in the monsoon subtropics, i.e. it is
downstream of A1/A9, not a separate item.

Also still open from the census: `H` (highland) at 8.07% of generated land against
0.00% of the reference, and `Csa` Mediterranean at 0.17% against 1.94%.

### A14. The two-season wind does not reverse — MEASURED, mechanism built and REVERTED

`earth_diagnose_seasonal_wind_reversal` (ignored) compares the January and July
headings at the real Earth's monsoon sites. On the shipped model:

```
  Arabian Sea 15N65E   Jan 225  Jul 225   Δ  0        N Australia 13S132E  Δ 2
  Bay of Bengal 15N90E Jan 225  Jul 225   Δ  0        W Africa/Sahel 12N0E Δ 3
  Somali coast 5N52E   Jan 225  Jul 225   Δ  0        SE Asia 15N100E      Δ 1
  India west coast     Jan 228  Jul 221   Δ  8
                                   monsoon sites reversing (Δ > 120°): 0/7
```

**The maximum seasonal heading change anywhere on Earth is 8°.** A monsoon IS a
seasonal wind reversal — that is the word's definition — so the model has two
seasons of *rain* running on one season of *wind*. `belt_wind(lat, circ)` takes no
season argument; the only seasonal term is `seasonal.rs`'s thermal-low
perturbation at `MONSOON_WIND_GAIN = 0.10`, which cannot turn a belt.

This is also the wrong MECHANISM, not merely a weak one. The model implements the
historical view of a monsoon as a giant land–sea breeze; the modern view is that a
monsoon is the seasonal migration of the ITCZ, with land–sea contrast selecting the
LONGITUDE the convergence zone reaches furthest poleward rather than supplying the
force (Chao & Chen 2001; Gadgil 2003; Geen et al. 2020, *Rev. Geophys.*).

**Built and measured:** `belt_wind_shifted(lat, circ, shift)` displacing the whole
circulation toward the summer hemisphere, with the meridional direction taken from
the side of the *displaced* ITCZ and the Coriolis handedness from the TRUE latitude
— which reproduces cross-equatorial recurving into a genuine south-westerly. Plus a
per-column land pull, a latitude taper (a uniform shift reversed the 55°S Southern
Ocean westerlies, which is flatly wrong), and a wind-aware `monsoon_onshore`.

```
  configuration                                   main   exact   reversal
  baseline (shipped)                              70.8    32.8      0/7
  migration only, 10°/1.6                         68.7    32.3      7/7
  migration 8°/1.0 + wind-aware onshore gate      70.1    33.7      4/7
  migration 10°/1.6 + wind-aware onshore gate     68.5    32.4      7/7
```

**ADOPTED** at migration 8°/1.0 with the wind-aware onshore gate (align floor 0):
**70.1 main / 33.7 exact / 4-of-7 sites reversing**, with `EARTH_MAIN_FLOOR`
lowered 70.6 → 70.0 as a deliberate, one-off trade and
`earth_monsoon_wind_reverses` promoted to an ASSERTED test so the physics bought
with that point is defended by CI. The record of the decision follows.

**Every configuration costs main-class.** The reason
is now well established across A7, A8 and A14 — *the model's arid belt is held up
by a wind that never changes direction.* Turn the wind honestly and the B row
collapses (68.1% → 61.3%). The wind-aware onshore gate recovers B completely
(back to 68.1%) and gives the best exact-zone agreement ever measured (33.7% vs
32.8%), but then costs the A row (83.5% → 80.0%) by shutting the monsoon off over
genuinely monsoonal tropics too.

**This is the session's clearest open trade**, and it is not a tuning problem:

- **For** adopting it: the physics is unambiguously right, it has an independent
  non-score gate (0/7 → 4/7 sites reversing), and it improves exact-zone — the
  metric CLAUDE.md §2.3 says to track, since main-class is inflated by E scoring
  ~99% for free.
- **Against**: main-class drops 0.7, carried entirely by the largest-weight class.

Whoever picks this up should decide that question FIRST, before touching a
constant. If the answer is to adopt, the follow-on work is a better onshore
criterion — one that distinguishes "the summer wind blows off a warm sea onto this
coast" (India) from "the summer wind has an onshore component somewhere in its
quadrant" (the Sahara) — most likely the moist-static-energy + orographic-insulation
index in the earth-systems report rather than any ray test at all.
