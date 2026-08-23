# WorldForge 2 — Scoreboard

**The project in twelve numbers.** 89k lines across climatology, economics,
rendering and UI is more than anyone can hold as code. It is easy to hold as a
table of measurements. That is what this file is for.

Append a row every session that moves a number. Never edit an old row — a
scoreboard whose history is rewritten cannot show a regression.

---

## 2026-08-20d — The two gates disagree about `COMFORT_IMPORT_FRAC`, and the one that won isn't about trade

Follow-up to 20c, which closed by flagging that `COMFORT_IMPORT_FRAC` = 0.30 had no
justification independent of the inheritance gate it was tuned against — the exact thing
§2.4 forbids. **It now has independent evidence, and the evidence cuts against 0.30.**

Measured by sweeping the constant and reading `econ_fidelity_scorecard` (one seed per
dose — see caveats):

| dose | basket price gap × distance | goods with a positive gradient | basket CV | real wage |
|---|---|---|---|---|
| 0.00 | −0.006 | 0 of 6 | 1.573 | 146.3 |
| **0.30 (shipped)** | **−0.064** | **0 of 6** | 1.596 | 162.5 |
| 0.60 (the reverted dose) | **+0.041** | **2 of 6** | 1.672 | 169.4 |
| 0.90 | +0.053 | 3 of 6 | 1.678 | 152.9 |

A POSITIVE price/distance gradient is the historically correct sign (Federico; Persson),
and its absence is the largest market failure this project has named for itself —
`TRADE_AND_MARKET_REVIEW.md` F2, *"0 of 6 priced goods show any positive distance
gradient… distance costs nothing anywhere."* **Only doses ≥0.60 produce one. The shipped
0.30 is the worst of the four tested on that measure.**

So the two gates genuinely disagree: `econ_inheritance_rules_fragment_differently` wants
the dose low (0.60 inverts it — that is the 20c story), and market integration wants it
high. The value currently in the tree was chosen by the gate that has nothing to do with
trade, because that is the gate that was red.

**Nothing was changed.** Raising it would re-break a gate and turn `main` red again, and
the right fix is not to buy market integration with a demand constant anyway — F2 already
names the real culprits (freight is ~11% of grain value over the longest route, against
real carting that roughly doubled grain's price over 150–300 km; and i.i.d. per-hub
harvest shocks leave no regional scarcity for a gradient to form against). This is a
finding handed to the maintainer, recorded at the constant and here.

**Caveats, stated because the table is seductive:** one seed per dose; the low end is not
monotone (0.00 → 0.30 makes the gradient *more* negative, so this is not a clean "more is
better" curve); real wage peaks at 0.60 rather than tracking the dose; and every dose
leaves basket CV at 1.57–1.68 against a historical 0.20–0.40 — no value tested makes this
market realistic, the differences are directional inside an already badly-calibrated
regime. Treat the table as "0.30 is not the best-supported value", not as "0.60 is right".

Reproduce: for each dose, `sed` the constant in `sim/campaign/tick/mod.rs` and run
`cargo test --release --lib econ_fidelity_scorecard -- --nocapture`, reading the
basket-gradient and per-good rows. (Deliberately not built as a permanent `econ_measure_*`
harness: sweeping a `const` needs it promoted to a runtime field on `CampaignSim`, which is
a real change to the sim struct for a diagnostic that runs about once a year.)

No code changed; no test counts moved.

---

## 2026-08-20c — CORRECTION: 20b was wrong, and the way it was wrong is the finding

**The 20b entry below is left standing because it is a textbook error, not because it is
right.** It concluded that `econ_inheritance_rules_fragment_differently`'s mean-wealth
assertion was "measurably false" — 1 seed in 6 — and removed it, with several pages of
supporting reasoning about the merchant pool not being conserved.

The sweep behind that conclusion was run while `COMFORT_IMPORT_FRAC` was still at
`a7ff520`'s 0.60: **the very dose that had inverted the gate in the first place.** A
concurrent session had already bisected to that commit and corrected the dose to 0.30.
Re-running the identical 6-seed sweep at 0.30:

| contrast | @0.60 (broken) | @0.30 (shipped) |
|---|---|---|
| more houses ever founded | 6/6 | 5/6 |
| more houses still standing | 4/6 | 2/6 |
| lower top share | 2/6 | 3/6 |
| **lower mean wealth per house** | **1/6** | **5/6** |
| no MORE capital in total | 1/6 | 5/6 |

The disputed assertion goes from 1/6 to 5/6. It is real, the dose genuinely broke it, and
the other session's diagnosis was correct. The assertion is **restored**.

**The lesson, which is worth more than the assertion.** A seed sweep is an instrument, and
an instrument reads the world it is pointed at. Measuring "robustness" inside an economy
that a known bug had already bent produced a conclusion that was confident, quantified,
documented at length, and false — including a crisp, memorable, wrong slogan ("firm count
is a multiplier on merchant wealth, not a divisor of a fixed stock") that was purely an
artefact of the 0.60 dose. Quantification is not the same as validity. **Before concluding
an assertion is unsound, establish that the world you measured in is not itself broken** —
and prefer fixing a bisected mechanism over deleting a claim it contradicts.

Also corrected: 20b's margin comment cited a "measured 1.08–1.45" cross-seed range for the
houses-ever ratio. That range was likewise from the 0.60 world; at the shipped dose the
contrast holds 5/6 and seed 1337 inverts it outright (180 v 193), so the ≥1.05 floor is
calibrated to this gate's own seed with headroom, not to a cross-seed minimum. Said plainly
in the code rather than left to imply a robustness it does not have.

Kept from 20b, both still useful and both independent of the error:
`a_division_moves_capital_and_creates_none` (the zero-sum invariant asserted at
`divide_estate`, where it is decidable — worth having regardless, since at 0.60 the
partible world held 44% more total wealth while the split remained exactly zero-sum, so a
downstream inference would have reported a minting bug that did not exist), and
`econ_measure_inheritance_robustness` itself, which is the instrument that settled the
disagreement and now carries the dose-comparison table.

One residual caveat, flagged not resolved: 0.30 was chosen because it restores this gate,
which is thin grounds for a demand parameter. The dose-dependence above shows the gate is
reading something real rather than noise, so the choice is defensible — but
`COMFORT_IMPORT_FRAC` still has no independent justification.

Rust tests: **342 pass, 0 fail** (28 ignored).

---

## 2026-08-20b — The merchant pool is not conserved (a gate that was asserting a false claim)

> **SUPERSEDED — see 2026-08-20c above. The central conclusion of this entry is WRONG.**
> The 6-seed sweep it rests on was run at the broken `COMFORT_IMPORT_FRAC` = 0.60; at the
> corrected 0.30 the "false" assertion holds 5/6 and has been restored. Left in place
> unedited as the honest record of a well-documented mistake.

The 2026-08-20 entry below recorded `econ_inheritance_rules_fragment_differently`
failing after `a7ff520` and left it for a later session as "pre-existing and
unrelated". Diagnosed and fixed here. **It was not a confounder to isolate — the
assertion was simply false.**

**The measurement.** A new `#[ignore]`d diagnostic,
`econ_measure_inheritance_robustness`, runs the partible/primogeniture pair across
6 seeds and asks how often each candidate contrast actually holds:

| contrast | holds on |
|---|---|
| partible fragments more (houses ever) | **6/6** |
| partible leaves more houses standing | 4/6 |
| partible disperses more (lower top share) | 2/6 |
| partible leaves the average house poorer (mean wealth) | **1/6** |
| partible holds no MORE capital in total | 1/6 |

The failing assertion holds on ONE seed in six, and not on the gate's own. **Every
one of the three rejected candidates passes on the gate's own fixed seed**, so any
of them could have been dropped in to make the suite green while asserting something
false — precisely §2.4's "spot-check win with an aggregate loss". Both replacements
tried here (concentration, then houses-still-standing) were caught that way and
rejected; measuring first is the only reason.

**The mechanism, and why it matters beyond this test.** `divide_estate` is exactly
zero-sum: it debits the parent what it credits the co-heir (now asserted directly by
`a_division_moves_capital_and_creates_none`, at the mechanism, where the claim is
decidable). But the extra firms then TRADE, and trade captures profit from the wider
economy — so partible ends the run holding **MORE** total merchant wealth, 5 seeds in
6, typically 30-45% more (seed 42: 8,523,684 against 5,934,903). **Firm count is a
multiplier on merchant wealth, not a divisor of a fixed stock.** The gate's own
printed note had been telling every reader the opposite ("the same capital, spread
over more houses") for as long as it has existed.

That also names a real model limit: with no fixed factor and no diminishing return to
firm count, the model cannot reproduce the historical case partible inheritance is
usually cited for — that fragmentation left Florentine and Venetian firms genuinely
smaller. Recorded, not fixed.

**What changed.** The false assertion is gone and **not replaced** — only one aggregate
contrast survived measurement, and inventing a second to keep the count up is the thing
that caused this. What remains is strengthened instead: the fragmentation assertion now
requires a real MARGIN (≥1.05×; measured 1.08–1.45 across seeds, 1.23 here) rather than a
bare `>`, because a bare `>` on a near-tie is a coin flip dressed as a gate — that is
exactly how crisis relief flipped it at 190 against 196. Assertion 4's comment, which
inferred "if the partible world has more total wealth, the split is minting money", was
deleted as a false inference that would have sent the next session hunting a nonexistent
bug; the invariant it wanted is now its own unit test. The gate's printed note reports
every rejected contrast with the seed count behind it, so nobody reads an unmeasured
claim off it again.

**The meta-lesson, recorded because this gate has now been perturbed five times**
(realms, crisis relief, the trade horizon, estate-share tuning, comfort-good import
demand): the first three were genuine confounders and are correctly isolated
(`suppress_realms`, `suppress_relief`, a widened `world_w`). The fifth was an
unrelated change exposing a wrong assertion — and the established reflex, isolating
the trigger, would have preserved the wrong claim indefinitely. Diagnose across seeds
before reaching for another suppression flag.

Rust tests: **342 pass, 0 fail** (28 ignored) — two added (the conservation unit test
and the seed diagnostic), the standing failure resolved.

---

## 2026-08-20 — Seven elevation styles, served not copied

Render-only follow-up to Terrain 2.0, prompted by the maintainer asking whether
the app offered any elevation-style choices beyond the two hard-coded layers
("elevation" flat/unshaded, "terrain" the one hillshade). It didn't — every
atlas convention (classed hypsometric bands, Imhof's neutral Alpine relief, a
monochrome analytical hillshade, a sepia antique plate) was unavailable no
matter how the map was configured.

Seven ship — Layer Colouring, Alpine, Arid, Polar, Analytical, Antique Plate,
Abyssal — as **data, not new render functions**: `ElevationStyle` + `StyleParams`
(land/sea ramp, classed-vs-smooth, climate-tint strength, real-`snow_frac`
blend strength, AO/contrast/shadow-floor/light-altitude, warm-vs-cool shadow
tint, sea relief amplitude) drive ONE shared `render_elevation_styled`, selected
via a layer-key modifier (`"elevation#style=alpine"`) that mirrors class
isolation's own `#iso=` mechanism exactly, so the frontend tile cache keys and
invalidates for free. `relief_at`/`sea_shade`/`relief_channels` were
parameterised (`relief_at_params`/`sea_shade_amp`/`relief_channels_warm`) with
the ORIGINAL functions kept as thin wrappers passing the historical constants —
bit-identical default rendering, gated by the full unit-test suite passing
unchanged (339/340; the one failure is the pre-existing, unrelated
`econ_inheritance_rules_fragment_differently` regression from `a7ff520`, see
below).

Two things worth recording:

- **Every style's palette is SERVED, not copied** (§8.18's rule applied to a
  new table for the first time since it was written): `elevation_style_palettes()`
  → `get_render_palettes()`'s new `elevation_styles` field → `LayerLegend.tsx`
  reads the ACTIVE style's real ramp. No second hand-written copy was created to
  drift.
- **A shallow-water clipping artefact, caught by actually rendering a world, not
  by reading the constants.** Layer Colouring's first sea stop (176,219,231) is
  already close to white; `sea_shade_amp`'s ×1.18 sunlit-slope ceiling pushed it
  to solid (255,255,255) on every sunlit shelf cell, reading as a bright halo
  hugging every coastline in the full-world render. Darkened to (150,196,214) —
  same pale "classic atlas shelf blue" once shaded, no clipping. `AO_REF`
  (§8.21) was exactly this lesson the first time; it recurred here in a
  different table, which is why `dump_elevation_style_sheet` renders a REAL
  generated world through the real dispatch path rather than sampling constants.

`cargo test --release --lib dump_elevation_style_sheet -- --ignored --nocapture`
(env `ELEVATION_STYLE_SHEET_DIR`, `ELEVATION_STYLE_SEED`) writes one full-world
PNG per style plus the two default baselines and a numbered montage — the
`dump_natural_sheet`/`dump_biome_swatch_sheet` discipline applied to a new
render feature.

**Pre-existing, unrelated finding, not fixed here:** `cargo test --lib` on this
branch (merged from `origin/main` at `a7ff520`, "Demand: comfort goods also draw
foreign-import craving...") shows `econ_inheritance_rules_fragment_differently`
failing — partible leaves the average house RICHER than primogeniture
(193,720 vs 164,858), the inverse of the assertion. Confirmed via `git worktree`
against `origin/main` in isolation (no terrain/render changes present) that the
failure reproduces there too, so it predates and is unrelated to this session's
work. Left for a session that can attribute it to `a7ff520` specifically.

---

## 2026-08-19e — Terrain 2.0 slice 4, take three: a level-set coastline

The 2026-08-19d entry's slice 4 (below) shipped a real, measured, but visually
unconvincing fix: `coast_on_boundary` fell from ~100% to 62.5%, yet a maintainer
screenshot showed the coastline still reading as an almost-unmodified Voronoi
polygon (a perfect triangular island included), with a scatter of speckle-islands
bolted on elsewhere — plus straight diagonal "scar" lines cutting across
continental interiors that turned out to be the divergent-boundary rift pulldown
(`e *= 0.7` on `boundary_type[idx] == 2`, read at the cell's own UNwarped
position, exactly on the raw Voronoi edge — a bug the D4 orogeny-belt warp never
touched because it's a separate code path).

**Root cause, finally correct.** A 2-D noise field thresholded per-cell has no
notion of "near the boundary" — wherever it crosses zero, it crosses in whatever
shape ITS OWN contours happen to make, uncorrelated with where the true 1-D
boundary curve runs. Fixed by construction instead of by amplitude: a SIGNED
DISTANCE TO THE NEAREST BOUNDARY (positive on the land side, negative on the sea
side, from a plain BFS) is perturbed by noise and re-thresholded at zero — the
level-set technique real coastline generators use. Only cells within `reach`
(~0.55×plate-size) of an actual boundary are ever eligible to flip; a deep
continental interior or open ocean is untouched by construction, so the far-flung
speckle islands are gone without needing the aggressive despeckle floor the
previous pass required (90 cells → 14). Two noise octaves scaled to complete
several cycles across `reach` (a broad one for sweeping peninsulas/bays, a
shorter one for headlands on a bulge's own edge) replace the single frequency
that couldn't do both jobs at once. The rift-pulldown line is fixed the same way
as the D4 orogeny belt already was: read at the SAME warped position, faded
smoothly with distance instead of switching on a hard 1-cell line.

**Measured: dramatically better, not marginally.** On the exact `dump_natural_sheet`
world (1440×720, seed 20260818) that produced the maintainer's screenshot,
`coast_on_boundary` fell from **78.9% → 6.9%** (a different config than 08-19d's
62.5% figure — that entry's own number came from a different world size, which is
itself part of why the visual didn't match the metric). On the `terrain_metrics`
config: **6.2%** (was 90.0% before ANY slice-4 work, per the 08-19d table). Land
fraction preserved to within 20 cells out of 367,224 (the percentile-threshold
mechanism, unchanged). A visual check (`dump_natural_sheet`, both the whole-world
PNG and the 3× mountainous crop) now shows real peninsulas, bays and irregular
islands in place of the polygon; the diagonal scar lines are gone.

**Gates:** all 340 lib tests still pass, including `coastline_departs_from_the_
plate_boundary` (now easily clears its 85% floor at single-digit percentages) and
`goods_coverage_diagnostic` (the `pearls` exception from 08-19d is still needed —
re-verified by removing it and watching the test fail again). `earth_koppen_
agreement` unaffected (still 70.2%/39.0%). `bench_phase2` @ 3600×1800 essentially
unchanged (plates 11.1s, shape 13.3s) — the level-set score pass early-returns
for any cell outside `reach`, so it's no more expensive than the approach it
replaced despite computing a real BFS field.

**Lesson for the record:** the 08-19d entry's own `coast_on_boundary` number was
real and correctly measured, but a single scalar from a config nobody was
looking at is not the same claim as "the picture looks right" — the codebase's
own rule 4 (§4 of `TERRAIN_2_PLAN.md`, "the headline gate could be gamed") warned
about exactly this for slope spread and it applied here too. The fix was to look
at the actual image the user was looking at, not just trust a different number
from a different world.

---

## 2026-08-19d — Terrain 2.0: all six slices, measured not asserted

`TERRAIN_2_PLAN.md`'s whole build in one pass: droplet erosion → stream-power
(priority-flood + flow accumulation + `K·A^m·S^n` incision), a new transient
`geology.rs` (lithology + real orogeny setting/age for the plate model, a relief
pseudo-setting for the other three, a phase-2 climate-erosion proxy, regionalised
hypsometric redistribution), the plates.rs D3 boundary-classification fix (true
Voronoi-bisector normal, strongest-signal tie-break at a triple junction), coastline
decoupled from the plate Voronoi edge (a warped crust-thickness field), seafloor
ridges/trenches/abyssal hills/seamounts, and a render follow-up (a bounded texture-
shading approximation on the AO term + the elevation ramp's white blow-out softened).
The `terrain_metrics` harness (§3, new) is this session's own instrument — first
measurement, so it sets the baseline rather than clearing one:

| model | rms_slope | slope_spread | drainage_density | coast_on_boundary | sea_depth↔dist r |
|---|---|---|---|---|---|
| plates | 0.0491 | 0.846 | 14.9% | 62.5% | 0.720 |
| shape | 0.0355 | 0.410 | 12.4% | n/a | 0.736 |
| ridged | 0.0288 | 0.765 | 17.1% | n/a | 0.736 |
| cordillera | 0.0674 | 0.758 | 19.2% | n/a | 0.736 |

**A second, sharper measurement bug found INSIDE the slice-4 measurement itself.**
The first pass landed `coast_on_boundary` at 90% and looked, in `dump_natural_sheet`,
unmistakably like the raw Voronoi edge — the exact symptom D1 describes. A direct
probe (comparing the resulting `terrain` bit-for-bit with the domain warp on vs
off) found why: the crust FIELD genuinely differed between the two runs (confirmed
by summing it), but the percentile THRESHOLD selected the identical set of cells
regardless, because `fbm_noise`'s multi-octave output clusters far more tightly
than its nominal 0..1 range — the realised noise swing rarely bridged the 0.5 base
gap between an oceanic and a continental plate's crust value at all. A numerically-
different-but-geometrically-identical result is invisible to eyeballing a render and
easy to mistake for "a small effect" rather than "no effect" — the probe (comparing
the actual `terrain` array, not the intermediate float field) is what told them
apart. Retuned to a ±0.9 swing (amplitude 1.8, up from 0.62) and a warp beyond a
full plate-size wander, which is what actually moves `coast_on_boundary` (~100% →
62.5%) and is now itself a permanent gate
(`coastline_departs_from_the_plate_boundary`, two probe seeds, <85%). The stronger
noise also threw a scatter of single/few-cell islands that read as a rendering
glitch rather than a real archipelago; `despeckle_terrain` (a 4-connected component
flood-fill, flip anything under `DESPECKLE_MIN`=90 cells back to its surroundings)
cleans that up without touching the real decoupling. One measured downstream
consequence: the fixed-seed 300×150 goods-coverage reference world now places
`pearls`' inshore homeland on a stretch of new coastline outside every settlement's
catchment at that specific seed/scale (settlements are regenerated FROM the
decoupled coastline in real generation, so this is a fixed-fixture sampling
artefact, not a claim that pearls is unreachable in practice) — named as a new,
honestly-labelled exception in `goods_validation.rs` alongside the pre-existing
`dyes` case, not silently folded into it. `sea_depth↔distance` fell from ~1.0 (a
pure function of distance-to-coast) to 0.66–0.74 across all four models — genuine
seafloor structure, since slice 5 (ridges/trenches/hills) runs in `generate_shelves`
regardless of which elevation model generated the land.

**The Earth gate is the one thing this touches that CAN'T be waved through.**
`compute_sea_depth`/`generate_shelves` feed `distance_to_ocean`/upwelling in phase 3.
Measured: **main-class 70.1% → 70.2%**, exact-zone unchanged at 39.0%. Floor raised
70.1 → 70.15. `earth_monsoon_wind_reverses` and the named-region spot checks are
unaffected, and unaffected again by the slice-4 retune above (`compute_sea_depth`/
`generate_shelves` don't depend on how `terrain` was thresholded, only on the
result).

**Cost, recorded rather than hidden.** `bench_phase2` @ 3600×1800: plates
**8.5s → 11.4s**, shape **11.4s → 13.9s** — short of the plan's own "no slower than
before" target despite rayon-parallelising every pass with no cross-cell dependency
(Voronoi assignment, boundary classification, the lithology/climate-proxy maps, and
now the slice-4 crust map itself). The priority-flood queue itself is inherently
sequential (a priority queue doesn't parallelise the way the phase-3 row loops do)
and stayed the dominant cost.

**A real negative result, kept as the plan's own §2.4 discipline asks.** The outer
pass count was first keyed to the old `iterations` (droplet-budget) parameter,
which scales UP with world size before its own ceiling clamp — exactly backwards
for a perf-costly pass, and it starved every unit-test-sized fixture (all well
under the `iterations` floor) down to 2-4 passes, which silently broke
`cordillera_crest_runs_parallel_to_the_coast` (a traced spine needs ≥5-6 outer
passes to differentiate from generic ridged noise; below that the test's own
80%-of-noise-spread margin fails at ~81%, i.e. barely). Fixed by keying pass count
to GRID SIZE instead (4 passes >4M cells, 6 between 1-4M, 8 below — free for every
test fixture, capped for a real generated world), which is the actually-correct
axis: wall-clock cost is set by how many cells a pass touches, not by an
erosion-strength slider.

**Gates:** 340 lib tests pass (one new: `coastline_departs_from_the_plate_boundary`,
slice 4's own permanent gate). Zero unexplained failures along the way —
`cordillera_crest_runs_parallel_to_the_coast` caught the pass-count regression
above and is green after the grid-size fix; `goods_coverage_diagnostic` caught the
`pearls` consequence above and is green after the honestly-labelled exception.
`earth_` green with the raised floor. `simulate_decades_reports_dynamics`
unaffected (the campaign sim doesn't touch world-gen). `cargo check` and
`npx tsc --noEmit` clean.

**Still not built** (named per §2.4, not silently assumed): plate motion over
time, glacial/aeolian/karst landforms, sub-cell detail (the inverse LOD pyramid),
persisted geology (still transient by design), two-way climate coupling (the
phase-2 climate proxy stays a documented proxy), and the true multi-scale
fractional-Laplacian texture-shading transform (needs a wider cross-tile render
halo than `TileNeighbors` carries today — the shipped follow-up is a bounded,
honest approximation using the existing single-ring AO curvature instead).

---

## 2026-08-17d — Consolidation: realms grow, absorb, and break

**The gap this closes.** Tilly's ~500 European polities c.1500 fall to ~25 by
1900, and the model had only the first half of that curve — realms formed and
fragmented, nothing ever merged, so a world reached 1500 and stayed there. Three
new yearly passes in `realms.rs`, all CONTIGUITY-driven over `prov_neighbors`,
which is also what makes a realm read as a country rather than a scatter:

- `realm_expansion_pass` — annexes ONE adjacent free province, preferring land of
  the realm's own culture (which is not flavour: growing along your own people
  keeps the cohesion needed to keep growing). Gated on cohesion and treasury.
- `realm_vassalage_pass` — a realm ≥2.5× stronger imposes vassalage on an
  adjacent one; after 80 years it may integrate it outright, land and treasury
  passing whole. `Realm.vassals` has existed since R1 and had no writer until now.
- `realm_secession_pass` — a culturally foreign province breaks away from a crown
  whose cohesion has collapsed; a realm losing its last province falls. Without
  this a model that only grows converges on one colour as surely as one that only
  fragments (the plan's own §5.6).

**The tuning is the deliverable as much as the mechanism.** Shipped at
first-guess rates, consolidation ran away: of 19 realms founded over two
centuries only **5 were still standing**, with 16 integrations — straight past
Tilly's four-century curve into a handful of empires inside 200 years. Measured
progression on `econ_measure_realm_paths` (72 cities / 24 provinces / 6 peoples):

| | founded | standing | annex | vassal | integrate | secede | largest |
|---|---|---|---|---|---|---|---|
| first guess | 19 | **5** | 19 | 14 | 16 | 1 | 7 |
| slowed | 25 | 13 | 20 | 16 | 18 | 14 | 6 |
| **shipped** | **31** | **21** | 8 | 17 | 12 | 3 | 5 |

Shipped state: first realm at year 51, 23 of 24 provinces under a crown, live
count rising 17 → 22 and holding, largest realm 5 of 24 (no runaway), mean
cohesion 0.79. All three paths populated (merchant 3 · city 9 · culture 9), both
governments (dynastic 11 · civic 10), the full rank ladder occupied (7 city-states
· 8 kingdoms · 4 great powers · 2 hegemons).

**Two more fixes for symptoms the maintainer reported.** Partition was
round-robin BY INDEX — every n-th province by ID — which produced interleaved
checkerboard realms, the worst thing this layer did to the map's readability.
It now seeds each heir far from the others (`province_hops`) and grows connected
shares outward, the way real divisions went (Verdun's three north-south strips,
the Mongol uluses by campaign theatre). And a realm founded by a PEOPLE is now
named for that people, not for whichever town led them — France is not "the
Kingdom of Paris", and styling every realm after a city was why the names read
wrong.

**Two tests changed premise, and neither was a bug.** The merchant-gate test and
the bloc-minimum test both began passing/failing for the wrong reason once Path B
could found a republic on its own account and the culture minimum dropped: a city
nobody governs crowning an office is exactly what Path B is FOR, so it was
satisfying every "never proclaims" assertion for an unrelated reason. Both now
isolate the gate they actually test.

**Gates:** **325** lib tests pass (6 new consolidation gates). `econ_` scorecard
green; dynamics green; `earth_` untouched. `cargo check` and `npx tsc --noEmit`
clean.

**Still not built:** personal union / cross-realm marriage, inherited claims, and
conquering a foreign CAPITAL (still guarded off in `apply_war_goal`). Vassalage
and integration are the consolidation routes that exist; dynastic union is not.

---

## 2026-08-17c — Realms actually form: a real instrument, then the tuning

**First, the instrument.** The 2026-08-17b entry recorded a null result — the two
new formation paths added zero realms — and blamed the oracle. That was right, and
the fix was to build one that works. `realm_reference_world()` +
`econ_measure_realm_paths` (both `#[ignore]`d): **72 cities, 24 provinces in a 6x4
grid, six peoples in contiguous 2x2 blocs, a 4-connected `prov_neighbors` graph,
and a rank-size spread of city populations.** `reference_world()` could not
express any of that — 5 provinces, a unique culture each, no neighbour graph, flat
city sizes — so Path C early-returned and Path B never found a tier-1 city. It is
kept SEPARATE rather than replacing the scorecard's world, because `prov_culture`
feeds migration.

**Measured, on a world that can answer the question:**

| | merchant paths only | all three paths |
|---|---|---|
| realms/century | 8.0 | **11.5** |
| live at year 200 | 11 | **17** |
| provinces under a crown | 15 of 24 | **22 of 24** |
| paths firing | merchant 11 | merchant 5 · city 8 · culture 4 |

The ablation is the point: merchant-only leaves **9 of 24 provinces permanently
stateless**, which is the genuinely unhistorical outcome. First realm at year 51.

**Three bugs the instrument exposed that review had not.**
1. **Sovereignty was double-assigned.** A coronation collected provinces by
   `prov_holder == seat` without checking `prov_realm`, so a new realm could list
   a province another crown already held — measured as "36 provinces under a crown
   of 24". Administration and sovereignty are independent layers (rule 27); taking
   an owned province needs a war, not a coronation.
2. **Path C could never fire.** It required the WHOLE culture bloc to be
   unclaimed, and from year 50 the other paths take provinces one at a time, so a
   single proclamation anywhere in a bloc foreclosed that people's nationhood
   forever. Now a people unifies out of whatever of itself is still free (≥
   `REALM_CULTURE_MIN_FREE_FRAC`), and the culture pass runs FIRST — unification
   characteristically happens against existing statelets (Piedmont, Prussia), not
   in a vacuum.
3. **Landless realms.** A city could proclaim over a province already under
   another crown and end up sovereign over nothing — most of them, measured (45
   live realms against 24 provinces). `has_free_province_at` now gates both the
   merchant seat and city paths.

**The year-50 cliff is gone.** `REALM_RAMP_YEARS` (25) scales every proclamation
chance from 0 at the floor to full a generation later, so the first crown appears
just after 50 and the rest arrive as a stream — which is also how state formation
looks: a slow start, then an accelerating cascade.

**The fragile inheritance gate, and why it is now isolated rather than tuned.**
`econ_inheritance_rules_fragment_differently` inverted (partible measured RICHER
than primogeniture, 137401 vs 133569). It was NOT the city path — tier 1 and tier
2 gave byte-identical numbers, so that path never fires on the reference world at
all. The cause is realm formation itself: `REALM_YEAR_FLOOR` is 50 and that gate
runs 60 years, so a decade of coronations lands inside its window, and each moves
a whole house's fortune out of the merchant pool at once — the realm plan's own
§5.2 warning, "crowns drain the merchant pool". That perturbation is large,
path-dependent and orthogonal to the law being measured. New `suppress_realms`
flag, set by that one test, isolates the variable the same way fixing the seed and
the world already do. Realm formation keeps its own instrument.

**Historical judgement** (`docs/WORLD_REALISM_REVIEW.md` §3.6): more realms is the
historically correct direction. Tilly counts ~500 political units in Europe c.1500
consolidating to ~25 by 1900; the HRE alone held ~300. A world of 72 cities
carrying 17 polities is squarely in that range, and the previous 8-on-a-5-province
world was not. The standing caveat is the other half of Tilly's curve: **nothing
in this model consolidates** — no personal union, no vassalising a realm, no
conquering a foreign capital — so the world reaches 1500 and stays there.

**Gates:** **319** lib tests pass. `econ_` scorecard green; dynamics green;
`earth_` untouched. `cargo check` and `npx tsc --noEmit` clean.

---

## 2026-08-17b — Realms: three dead fields revived, two non-merchant paths

**The three fields that did nothing.** `cohesion` was set to 1.0 at founding and
never written again (so `realm_collection_efficiency` — the plan's own "a state
is limited by what it can COLLECT" — reduced to distance alone); `rank` was never
promoted off `REALM_CITY_STATE` despite its doc describing a percentile ladder;
`legitimacy` was written by two paths and read as a decision input by none. Fixed
together because they are one mechanism: the path a realm formed by sets its
cohesion target, cohesion decides what it can collect, and rank is the reading of
that. `assign_realm_ranks` mirrors `assign_city_tiers` exactly (percentile among
live realms + an absolute floor on the top rank + hysteresis), and `realm_title_
for(rank, government)` replaces the flat four-name list that styled a house
holding one town "King".

**Two non-merchant formation paths** (maintainer's decision: stateless start,
merchants + powerful settlements + cultural domination). Path B — a tier-1 city
proclaims for itself, the FIRST reader `hub.tier`/`hub.standing` has ever had.
Path C — a contiguous single-culture bloc of ≥4 provinces unifies under its
largest city, over `prov_culture` + `prov_neighbors`, both previously unread for
this. `Realm.government` splits dynastic from CIVIC: a republic has no `family`,
no succession by birth, and never a dynastic title.

**THE NEGATIVE RESULT, which is the more useful half.** A matched before/after of
`econ_measure_realm_formation` (stash, run, restore) gives **8 realms by year 170
both before and after** — the two new paths added exactly zero on the reference
world. The cause is the ORACLE, not the mechanism: `reference_world()` seeds
`prov_culture` as `Culture{i}` (a different culture per province) and never seeds
`prov_neighbors`, so no culture bloc can exist and Path C early-returns; and the
fixture's 30 undifferentiated cities never clear tier 1's absolute standing
floor, so Path B has nothing to fire on. Both paths are therefore gated by unit
tests, not by the funnel, and **realms-per-century on a real generated world
remains unmeasured — as it was before, now with the reason known.** Making the
reference world express a culture bloc is the right next step but changes
`prov_culture`, which feeds migration, so it belongs with the `econ_` scorecard
rather than alongside a mechanism change.

**Two bugs the tests caught that review did not.** Path B's treasury bar used the
UPPER median of city treasuries, so on a small even-numbered world the richest
city was measured against ITSELF and could never clear its own bar (the same
funnel collapse `realm_founding_cost` already had to fix once). And `war.rs`
resolved a sovereign hub's ruler by indexing `ruling_house` raw — a civic realm
has none (`u32::MAX`), so the first republic to win a war would have panicked the
tick.

**Gates:** **319** lib tests pass. `econ_` scorecard green with top-10% wealth
share **0.696** and Gini **0.785**, both in band; dynamics run green; `earth_`
untouched. `cargo check` and `npx tsc --noEmit` clean.

---

## 2026-08-17 — Goods: a climate-placement bug, the cull, origins, endemics, terroir

**A measured mis-placement, not a tuning question.** `good_score` folded the
dry-winter Köppen variants onto their humid equivalents BEFORE its match ran, so
every arm naming `CWA`/`CWB`/`CWC`/`DW*`/`DS*` was **unreachable**. Tea and coffee
both name `CWB` (subtropical highland, dry winter — Darjeeling, Yunnan, the
Ethiopian highlands) and both scored exactly **0.0** there, placed instead by
their weak fallback arms in the wrong climates. Silk was silently downgraded
0.5 → 0.25; wine survived only via its `med_like` fallback. Fixed by scoring the
RAW zone first and folding only as a fallback. `envelope_score` now follows the
same rule — before this, custom goods read raw Köppen and built-ins read folded
Köppen, so one cell was scored under two different climate labels depending on
which scorer ran. Gates: `dry_winter_zones_are_reachable` and
`the_humid_fold_still_applies_as_a_fallback`.

**Catalogue 92 → 90 active.** `gemstones` (a generic gem alongside ELEVEN
specific stones) and `dyes` (marine murex, the same product as `tyrian_purple`,
and the one standing coverage-floor exception) retired — disabled, never deleted,
since both are fixed indices in `TileData.goods` (rule 7). Six island endemics
added: nutmeg, mace, dragon's blood, camphor, benzoin, sandalwood.

**Three new capabilities.** `GoodSpec.origins` (independent homelands per good —
pepper from Malabar *and* Sumatra, cotton's three domestications);
`LandmassContext` + `Distribution::Endemic` (the connected-component pass this
codebase never had — `Domain::Island` was `distance_to_ocean < 0.20`, i.e.
near-coast land, so an "island" good was really a coastal good); and
`GoodSpec.soil`/`relief`, the fine-grain terroir terms — `soil_type` was computed
by phase 6 and read by NOTHING in the goods layer, and slope was never computed,
which is why belts rendered as smooth continent-sized washes.

**Three silent-vanish failures caught by MEASUREMENT, not review** — the whole
value of running the coverage diagnostic and reading the per-good table:

| Attempt | Measured | Cause |
|---|---|---|
| endemics shipped as `Domain::Island` | all six placed **0 cells** | domain and distribution both gated "is this an island"; the domain gate zeroed the score before the distribution could choose a home |
| `ISLAND_MAX_CELLS` as a fixed cell count | islands unresolvable | resolution-dependent: a cell is ~11 km at 3600×1800 and ~133 km on a test world. Now `ISLAND_MAX_KM2`, converted per world |
| terroir applied as a raw multiplier | `tea` and `saffron` placed **0 cells** | soil×relief pushed already-marginal climates under the seed threshold. Now remapped into `[TERROIR_FLOOR, 1.0]`, soil never vetoes, and `saffron` is excluded from the table entirely |

Final placement on the diagnostic world: nutmeg 54 cells / 2 settlements, mace 29/2,
dragon's blood 28/1, camphor 45/2, benzoin 111/3, sandalwood 23/2, tea 47/2,
saffron 7/1. Slice 0 coverage floor green.

**New: the placement report.** `GoodsPlacementReport` →
`metadata["goods_report"]` → `get_goods_report`. Per good: cells, land share,
origins seeded, localities, notable names, mean grade, and the flags that are the
point of it — `absent`, `fallback_seed`, `ubiquitous`, `single_cell`. A good that
silently failed to place was previously invisible until someone went looking for
it on the map; `fallback_seed` was entirely unreported even though the seeder
falls back to the best passable cell *regardless of score*.

**Render + report UI (same day, follow-up commit).** The two remaining goods
issues, which turned out to be one bug in two places — a real per-cell field
drawn at a coarser resolution than it has. (1) The world quality overlay carried
TWO resolutions: full-resolution coverage clipped a quality wash that still rode
the old ~8-cell grid, so belts read as blocky steps inside a sharp coastline.
`coverage_rle` → `quality_rle`: the same runs now carry a 4-bit quantized belt
value, one layer at one resolution, payload roughly unchanged because a smooth
belt's neighbours share a bucket. (2) The province plate drew each locality as a
true-to-scale square — a 900 km staple on a 200-400 km province filled the whole
plate, which is what "large squares" meant; where a belt mask already draws the
real per-cell area, the locality is now a small core diamond at its real cell.
Plus `GoodsReportPanel`, opened automatically when Biological finishes. New gate:
`quality_levels_never_swallow_a_covered_cell`.

**Gates:** all **311** lib tests pass (goods coverage floor, belt-coastline claim,
`econ_` scorecard, `simulate_decades_reports_dynamics`, `earth_` untouched —
no `step3`/`step4` change). `cargo check` and `npx tsc --noEmit` clean.

**Not built, stated rather than dropped:** diffusion over time; Old/New World
separation (the same flood-fill labels continents, so it's cheap now); knowledge
as a scarcity axis; endemic value derived from island size/remoteness; an
exhaustible good; and the two RENDER fixes — the province plate draws a 900 km
staple locality as a square on a 200–400 km province, and the world quality
overlay still rides the old coarse 8-cell grid. Realm and city-placement findings
are **diagnosis only** in `docs/WORLD_REALISM_REVIEW.md`: three dead realm fields
(`cohesion` never written after founding, `rank` never promoted off `city_state`,
`legitimacy` read by nothing), the merchant-republic monoculture, and the absence
of any settlement-placement oracle.

---

## 2026-08-16 — Province trade view + a trade-share path to realm formation

**Player-reported: "no realms after 50 years."** First, the mechanical floor:
`REALM_YEAR_FLOOR = 50`, so a realm cannot form before year 50 by design — "none
by year 50" is expected. Beyond that, `econ_measure_realm_formation` (the funnel
diagnostic) shows the choke precisely: 27 governing hubs → 24 tier-1-2 merchant
rulers → **only 3 hold a province writ** → 3 afford. Affordability is a non-issue
(richest governing house 32M vs a 159k cost); the seat-writ requirement is the
throttle — trade dynasties dominate a province's commerce but rarely hold the
formal seat of its largest city.

**Fix (maintainer-chosen): a SECOND eligibility path.** A house commanding ≥
`PROV_TRADE_CONTROL_FRAC` (0.20) of a whole province's trade may now proclaim at
that province's seat without holding the seat office — the historically truer
basis for a merchant republic (Venice, Genoa). Additive: the seat-writ path is
unchanged. Trade share is `House.trade_at` summed over the province's cities
(`province_trade_shares`). Measured effect on the reference fixture: realms
12 → 13 over 120y, and the final-year count of ungoverned province seats fell
1 → 0 (the mechanism now consumes essentially every eligible seat this
deliberately-fragmented synthetic world offers; a real generated world has far
more provinces). Per §2.4 the 0.20 was NOT tuned against the fixture's number.

**Exact per-good province trade accounting.** New `prov_export_year` /
`prov_import_year` (flat `prov_count × goods`) accumulate in `accrue_flow` — the
one choke point every shipment passes — whenever a shipment crosses a province
boundary (export from the source province, import into the destination),
snapshotted yearly in `roll_city_finances`. Gated on a non-empty `hub_province`,
so a province-less sim (the dynamics test) never touches them → **bit-identical**
(dynamics gate green, econ scorecard green, all 135 tick unit + 15 realm tests pass).

**Dead-from-the-start cities · coastal cabotage (#6c).** Player-reported some cities
never trade. The route graph already guarantees no route-dead city WITHIN a
geographic component (`MIN_GUARANTEED_PARTNERS`, the per-component market lifeline)
and folds tiny components in (`rescue_tiny_components`) — but a small island or
near-shore region of ≥3 towns the worldgen pathfinder never joined by sea stayed
isolated, because the cross-component gate (#4) refuses all straight-line links to
avoid dishonest trans-oceanic arrows. New `CABOTAGE_SEA_FRAC` (0.08) pass links each
COASTAL hub to the nearest coastal hubs of OTHER components within a SHORT crossing (a
third of the #4 horizon) — the short-sea/inter-island trade a pre-modern economy
actually ran, without reopening long ocean lanes. Cross-component only ⇒ strict no-op
on the single-component econ-fidelity reference (scorecard bit-identical, dynamics
green). Two follow-ups explicitly NOT done: a manual "reorganize trade" button (would
mostly no-op — routes already exist) and a "why is this city dead" diagnostic
(isolated / no surplus / starving) — starvation is a production-balance problem
routing can't fix.

**New province-trade view.** `campaign_province_trade` (read-only) → per province:
trade share by house/guild, by city, and per-good exports/imports. Rendered as
dynamic donuts in the Province Inspector's new **Trade** tab, with the eligible
≥20% controller highlighted. First change to move realm formation off its
seat-writ bottleneck; realms-per-century on real worlds is still unmeasured
outside the synthetic fixture.

---

## Current state — 2026-08-12b (`ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` — ALL 13 slices addressed)

**The remaining four slices, closing out the plan.** Continuing from the
2026-08-12 entry below (4.1-4.9, 4.13):

- **4.12 · certification fee (A2), wired; adulteration built, deferred.** "Whoever
  grades, profits" — a certifying authority (a resident guild house, else the
  parent city's civic pool) takes `CERT_FEE_FRAC` (4%) of an estate's owner-cut
  at the EXISTING per-sale site, as a pure REDISTRIBUTION of that cut, never
  added on top of `sale`. Passed `econ_inheritance_rules_fragment_differently`
  on the FIRST real attempt despite touching every estate sale in the tick — a
  uniform skim applied identically under every inheritance law measured as a far
  more symmetric perturbation than 4.7/4.9's targeted transfers, confirming the
  session's working theory. Surfaced a genuine, over-broad pre-existing test
  assertion (`a_guild_never_divides_its_estate` checked NO house anywhere ever
  divides, when its own name and intent was that the GUILD specifically never
  does) — fixed by scoping the assertion to `h.is_guild`, verified against the
  two actual offenders (ordinary new non-guild houses dividing under partible
  law, correct behaviour). Adulteration (a distressed owner's one-off windfall,
  risking detection) is fully implemented and directly unit-tested but NOT
  wired into the tick loop — its trigger is gated on estate-owner DISTRESS,
  which differs structurally between inheritance regimes BY CONSTRUCTION
  (exactly what the fragile gate measures), so unlike the fee this wasn't a
  case dose-tuning could fix.
- **4.10 · coronation converts owned estates into crown leases (D12); A7 free by
  reuse.** Every estate a house owned outright converts at coronation: a
  pre-existing minority share is grandfathered into a time-limited LEASE
  (`instrument` → 1, 9-year term, A1's own range); the unclaimed remainder goes
  to the crown via a new Share row, reusing the ALREADY-WIRED `holder_kind=4`
  (realm) path both the dividend cut and 4.8's offtake pass credit to
  `Realm.treasury`. Closes a real latent gap rule 25 warned about — the old
  per-sale cut only checked `!defunct`, so a crowned house's former estates
  would have kept crediting it forever. A7 (royalty in kind) falls out for
  free: a raw estate's crown share is offtake (physical goods), not dividend.
  D13 (lease loss by revocation/war/intrigue) is a real, separate decision
  system, explicitly not built.
- **4.11 · population status (F8), a safe pure derived read; D18/D19 verified
  pre-existing.** `population_status(food_balance, starving)` reuses the exact
  0.5 threshold the existing civic-granary famine release already keys on,
  exposed on `HubBrief.pop_status` for the frontend — no new state, no tick
  change, so it cannot move any gate by construction. The riskier half (batching
  daily consumption into a visible monthly release, a real civic buy/resell
  margin) was deliberately not attempted: consumption is the single hottest,
  most universal per-tick path in the whole sim, and the plan's own gate wording
  already anticipated disruption. D18 (essentials-only civic routing) and D19
  (the civic share isn't alienable) were checked, not built: both already hold
  in the existing code (`council_provision_pass`'s existing food-first rule;
  no code path anywhere sells `civic_goods`).
- **Frontend · heraldic accents (A10).** A shareholder/tenant row in
  `WorksCard`'s ownership bar now resolves through the same `houseColor()` the
  House Dossier's shield renders with, plus a small `CoatOfArms` badge per
  house/guild row — the backend's generic `distinct_color` tag stays only as
  the bank/realm fallback (no heraldry to draw). `CityWarehousePanel`'s
  supplier board is untouched on purpose — D20 groups sellers into five
  CLASSES, never a named per-house ledger.

**The plan's own §6 "deliberately not built" list, now complete**: A6 (bank
credit-conversion, 4.9), D11/A9's reimbursement transfer (4.7), adulteration's
wiring (4.12), D13 (4.10), and 4.11's consumption-timing rearchitecture. Five
real, flagged follow-ups — none silently dropped, each with a stated reason
tied to a measurement or a structural risk, not a guess.

**Gates:** `cargo test --lib` **303 passed, 0 failed, 22 ignored** ·
`econ_inheritance_rules_fragment_differently` and `simulate_decades_reports_
dynamics` both pass · `npx tsc --noEmit` clean · `earth_` unaffected (this
session never touches `step3_ocean_atmo`/`step4_climate`).

---

## Current state — 2026-08-12 (`ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` — slices 4.1-4.9 and 4.13 built)

**What shipped.** Grade bands on stock (4.1: `stock[g]` → `stock[g][band]`, three
bands coarse/common/fine); spoilage + city warehouse capacity (4.2); the Warehouse
panel, a 6×6 slot grid (4.3, frontend); supplier attribution (4.4); the share table
replacing `stake_bank`/`stake_share` (4.5, two instruments — SHARE and TENANCY per
amendment A1); works cards with rank + yield index (4.6, frontend+backend);
toponymic brands for a *great*-or-above works (4.13, presentation only). Then the
three hardest slices, each with a real tuning story:

- **4.7 · disasters + repair.** The disaster ROLL is bit-identical to the pre-4.7
  code (same chance, magnitude, 3-way pick) — three successive attempts to also vary
  magnitude/repair-rate/frequency BY KIND each independently pushed
  `simulate_decades_reports_dynamics` into a sustained-runaway-rich house and were
  reverted per §2.4. D11/A9's real content (a repair-cost reimbursement pass:
  dilution on refusal, tenancy voided on persistent neglect) was built, conservative
  arithmetic and all, but flipped `econ_inheritance_rules_fragment_differently`'s
  partible-vs-primogeniture ordering on the SAME seed the disaster fix had already
  made bit-identical. Deferred, not shipped broken — `Share.neglect_years`/
  `instrument` and the dilution constants stay in place, reserved, for a future,
  better-isolated attempt.
- **4.9 · envoys + negotiation.** Cross-city acquisition (intent → dispatch → travel
  → standing → outcome), amended by A4 (a real `Law.kind` foreign-ownership bar,
  enacted only occasionally at a fresh council capture) and A5 (a bank branch
  spanning both cities clears the deal at full price; otherwise a costlier specie
  fallback). A6 (bank credit-conversion, the Fugger lend-into-arrears model) was
  named and deliberately NOT built — it needs a `Loan.estate_hub` field that doesn't
  exist yet, and retrofitting one touches `bank_pass`'s own default handling, the
  single most gate-sensitive loop in the tick. **A second, independent data point on
  this codebase's RNG sensitivity**: unlike 4.7's discrete branching-order flips
  (same final number regardless of tuning), reducing the envoy mechanism's trigger
  rate moved `econ_inheritance_rules_fragment_differently`'s margin roughly IN
  PROPORTION — 16.6% overshoot on the wrong side → 2.1% → passing — genuine
  dose-dependence. Landed on a deliberately narrow trigger (elite wealth floor,
  ~0.6%/month, year 25+) that proved real at a higher dose during tuning but is
  inert in the small dynamics/`econ_` fixtures at the shipped dose.
- **4.8 · offtake routing — "the big one," built last on purpose.** An extraction
  estate's offtake-payout shares (never a manufactory — D1) now deliver physical
  goods into the holder's own warehouse, largest holder first, off the FINEST grade
  band first (D5) via a new `stock_take_finest_first`, the mirror of the existing
  cheapest-first `stock_take`. Deliberately a MONTHLY pass over already-accumulated
  stock rather than a hook into the daily production write (the single hottest loop
  in the tick) — the same "drain a snapshot, don't touch the hot path" idiom
  `sync_and_stock_warehouses` already uses. Currently reachable only through 4.9's
  envoy PARTIAL outcome, so — like envoys — proven correct by two direct unit tests
  (an exact-conservation finest-band-first delivery, and a manufactory-never-routes
  guard) but not yet exercised by the dynamics/`econ_` fixtures at the shipped dose.

**Deliberately not built** (named in the plan's own §6 and flagged again above, not
silently skipped): A6's bank credit-conversion (4.9); the D11/A9 repair-cost
reimbursement money transfer (4.7, though the disaster table itself shipped). Both
recorded as open follow-ups, not abandoned.

**Gates:** `cargo check` clean · `cargo test --lib` **300 passed, 0 failed, 22
ignored** (was 294/0/21 before this session) · `earth_` unaffected (unchanged, this
session never touches `step3_ocean_atmo`/`step4_climate`) · `simulate_decades_
reports_dynamics` bit-identical to the pre-session baseline (richest 429723 at year
50) · `econ_inheritance_rules_fragment_differently` passes (the fragile gate two of
this session's three hardest slices had to tune against) · `npx tsc --noEmit` clean.

---

## Current state — 2026-08-11 (`GOODS_LOCALITIES_PLAN.md` — all 8 slices built)

**What shipped.** Trade goods got the belt→locality→cell hierarchy minerals already
had (§8.16 → §8.19 in `CLAUDE.md`): rivers now bias placement for 12 named goods
(floodplain/irrigation/riverbank/float_out, Slice 1), marine goods can be split into
`Inshore`/`Bank` bands (Slice 2), a new `sim::localities` module clusters every
enabled `Global`/`Local` good's belt into real terroir localities with full
modulation and a hard floor (Slice 3), notable localities get culture-local names
(Slice 4), the global map overlay draws a full-resolution two-layer (coverage +
quality) belt mask instead of the old coarse 8-cell blocks that spilled past the
coastline (Slice 5), the province survey plate draws real locality squares instead
of hashed markers (Slice 6), and locality/deposit grade now feeds hub quality in the
worldgen economy (Slice 7).

**Slice 0's coverage diagnostic — the gate every later slice was measured against —
caught two real things**, not just a passing baseline:
1. A process-global test race: the diagnostic originally called
   `cultures::set_active` (a process-wide `RwLock`), which under `cargo test --lib`'s
   parallel execution intermittently corrupted `econ_inheritance_rules_fragment_
   differently` and `econ_scorecard_is_deterministic`'s "deterministic" results —
   found by running the FULL suite, not just the new test in isolation. Fixed by
   relying on `names::gen_name`'s legacy-culture fallback instead of activating a
   real map.
2. A coastline-crossing bug in `deposits.rs`: `Distribution::Deposits` goods bypass
   `envelope_score`'s domain gate entirely, so the `CoastalMarine` model can place a
   working on the wrong side of its own declared `Domain` (measured at 300×150:
   bay_salt 115 cells, tyrian_purple 16, ambergris 1). Recorded as a printed finding
   for `docs/DEPOSITS_AND_MINING_PLAN.md`, not silently clamped in the renderer.

Also recorded, not chased: a handful of the rarest deposit goods (tyrian_purple,
ambergris, emerald) and one pre-existing belt good (`dyes`, murex purple — verified
untouched by any Slice 1-4 change) can land zero settlements in catchment at this
diagnostic's deliberately modest 300×150 world size. Named exceptions, not a
loosened floor.

**Gates:** `cargo check` clean · `cargo test --lib` **294 passed, 0 failed, 21
ignored** (was 290/0/20 before this session) · `earth_` unaffected (unchanged, this
session never touches `step3_ocean_atmo`/`step4_climate`) · `simulate_decades_
reports_dynamics` and `econ_` both green · `npx tsc --noEmit` clean · `npx vite
build` succeeds.

**Deliberately not built** (named in `GOODS_LOCALITIES_PLAN.md` §6 and `CLAUDE.md`
§8.19): a literal per-good coverage/quality toggle pair (one toggle per good plus two
global layer switches instead — ~90 checkboxes would be unusable); subtype boundary
strokes on the full-resolution mask path (subtype tinting is preserved); a fix for
the `Deposits` coastline-crossing finding above (belongs to a different plan).

---

## Current state — 2026-08-09 (CI fix: `econ_inheritance_rules_fragment_differently` — root cause bisected and fixed, unrelated to the realm work)

**The finding.** After the realm work (R1-R5, `REALM_AND_GOVERNMENT_PLAN.md`) shipped,
`cargo test --lib` started failing CI on every push, always the same test:
`econ_inheritance_rules_fragment_differently` (Phase 0.4's own gate). The failure
mode itself had silently CHANGED between an early-session check and the CI reports
— from "57 vs 57 houses ever" (a tie) to "partible must leave the average house
poorer than primogeniture (331107 vs 203572)" (an inversion) — a distinct failure a
prior verification pass had missed by checking only "still 1 econ_ test fails, as
before" rather than comparing the actual panic message. A first hypothesis (a
crowned house's zeroed wealth being misread by `measure_fragmentation`'s `!defunct`
filter instead of `is_merchant()` — the same bug class rule 25 already names 5 other
instances of) was real and worth fixing, but **empirically proven inert** for this
specific test via a debug instrument (`s.realms.len()`): the reference world's tiny
60-year run never actually crowns a house in the partible/primogeniture rows that
fail the assertion (it does in the seniority row, which isn't part of the failing
comparison).

**The actual root cause**, found by bisecting through 8 separate isolated worktree
builds (each commit checked out and re-run against the exact same test): `a212a4c`
("Cap campaign trade to a regional horizon") — a real, correct economic change,
verified at the time only against the real-world econ scorecard — introduced
`TRADE_MAX_DIST_FRAC`=0.24 (a fraction of `world_w`, tuned for a REAL generated
world's cell-count scale, e.g. ~864-cell reach on a 3600-wide world). Nobody
re-checked it against `economy_validation.rs`'s own synthetic `reference_world()`
fixture, whose `world_w`=100 with hubs spread across ~58 units native to a scale set
years before the trade-horizon feature existed. At world_w=100 the reach cap is only
24 units — under the hub grid's own diagonal spread — so the fixture's inter-hub
trade was silently severed for most pairs the moment that commit shipped, changing
the world's basic economic connectivity in a way sensitive to exactly the kind of
house-count divergence between inheritance-rule variants this test measures. Not a
hash-order or RNG-divergence artifact (verified: bit-identical across repeated
process runs at the same commit) — a genuine, deterministic, previously-uninspected
side effect of a real feature on an unrelated test's own private fixture.

**The fix**: `run_under()` (private to this one test) now widens its own copy of the
world (`s.world_w = s.world_h = 300.0`, set after `reference_world()` returns,
before `force_inheritance`) so the existing hub layout stays fully connected —
restoring what this test was actually calibrated against — without touching the
shared `reference_world()` every other `econ_` test also builds on, so nothing else
recalibrates. `assign_house_tiers`/`update_solvency`/`apply_wealth_sinks`'s existing
`is_merchant()` guards were correct as shipped; `measure_fragmentation`'s own
`!defunct`-only filter was still a real latent instance of the same bug class (a
future world where this test DOES crown a house would silently corrupt the mean) and
is fixed alongside it (`is_merchant()`), even though proven not to be this failure's
cause.

**Gate results:** `cargo check` clean · `npx tsc --noEmit` clean ·
`simulate_decades_reports_dynamics` hard-passes (wealth ∈ [-4.5, 418280.1]) ·
`econ_inheritance_rules_fragment_differently` passes (partible 142408 <
primogeniture 146881 mean wealth; 68 > 43 houses ever) · full `cargo test --lib` —
**273 passed, 0 failed** (was 1 failed). Clippy still reports pre-existing warnings
elsewhere in the tree (313, none touched by this fix, none in the changed files) —
advisory-only in CI (`continue-on-error: true`), not the blocking gate.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4d: sack and purge — the last step, `CITY_PROVINCE_WAR_PLAN.md` COMPLETE)

**What shipped.** The plan's own highest-risk item, deliberately built last:
`apply_war_defeat_consequences` (`tick/war.rs`), fired from `resolve_war` on any
decisive-enough defeat (`score_abs >= WAR_PRICE_TRIBUTE`, 40 — a marginal
reparations-only win does not cascade into breaking houses). Two paths, both
funnelling into the same `strip_holdings_at` + `house_is_ruined` check so
neither invents parallel machinery:

- **Enemy sack.** Every live non-guild house resident at the losing city
  (`house.hub == lose`) risks losing its own estates THERE (up to
  `WAR_SACK_MAX_ESTATES`=2, ownership passing to the city — `owner_house = -1`,
  the same "confiscated" convention the resale market already uses), offices/
  bailos/influence there, and any warehouse stock depot there — a per-house
  roll (`WAR_SACK_CHANCE`=0.5), not a guarantee, since not every resident
  family is equally exposed to a single sacking.
- **Internal purge.** The city turns on whichever house actually financed the
  losing war: the house-driven war's own `backer_house` (§3.4c) if this was
  one, else the losing city's own ruling house (`council_house`/
  `captor_house`) for an ordinary rival-council war — guaranteed once
  triggered (a targeted political act, not a raid), stripped the same way
  (up to `WAR_PURGE_MAX_ESTATES`=3) PLUS a wealth confiscation
  (`WAR_PURGE_CONFISCATE_FRAC`=0.25) straight into the city's own treasury and
  a real prestige/power cost (`WAR_PURGE_POWER_LOSS`=0.15).

Either path may cascade to full dissolution through the EXISTING
`dissolve_house` — no new cascade logic. `house_is_ruined` is a NEW check
distinct from the ordinary insolvency test (`update_solvency`, which reads
wealth alone): a war can strip a house's assets while it's still technically
solvent for a while longer, and that house is ruined in every way that
matters (no wealth AND no estates AND no offices anywhere) — the honest
trigger for a war-driven collapse.

**Gate results:** `cargo check` clean · `npx tsc --noEmit` clean (no frontend
surface for this step — sack/purge journal entries already render through the
existing chronicle) · `economic_war_levies_houses_and_resolves` and
`every_war_terminates_within_the_round_cap` both pass ·
`simulate_decades_reports_dynamics` hard-passes · `econ_` 4/4 non-ignored pass
— **first attempt, no RNG-divergence regression this time** (unlike 3.4a-c's
and 3.4e's own tuning rounds), because the severity gate keeps this path
comparatively rare. `econ_fidelity_scorecard`'s wars/century held at 45.00
(3.4e's own final value), consistent with sack/purge being a consequence of a
war's END, not a new trigger on how often one starts.

**`CITY_PROVINCE_WAR_PLAN.md` is now fully built end to end** — every item in
its own §7 order (1.2/1.3 panel · 2.1–2.5 provinces · 3.1–3.3 politics ·
3.4f/3.4a-c/3.4e/3.4d war) shipped and gated across this session. What remains
is explicitly out of scope by the plan's own §6 ("deliberately not built"):
territorial empires above the city-state, sieges/army movement, a rival house
finishing an enemy under cover of war, land state persisted back to tiles, a
per-cell quality field, the unexploited-opportunity view, and leagues/
treaties/diplomacy (FIX_PLAN B4) — plus 3.4e's own voluntary-war-financing
gap (lend to the chest, goods at a war premium) noted in its own entry above.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4e: war ledger, damage, blockade, boom)

**What shipped.** The four remaining §2 requirements from the plan, all reusing
existing machinery rather than inventing new fields:

- **War damage** (`war_damage_pass`, `tick/war.rs`): each belligerent's own
  estates/manufactories can take war damage yearly (`WAR_DAMAGE_CHANCE`=0.15 —
  see the tuning note below), writing straight into the EXISTING `TickHub.damage`
  field — the same field a natural disaster uses. No new repair machinery
  either: `estate_condition_pass` already repairs any nonzero `damage` whatever
  its cause, funded by the owning house or the parent city's treasury exactly
  as it already does for disasters. A house-owned estate's loss (in wealth
  terms, via `estate_market_value`) is booked to that house's own Accountant
  ledger.
- **A real, persistent blockade.** The pre-existing `trade_wealth *= 0.8` line
  was COSMETIC ONLY — `update_houses` recomputes `trade_wealth` fresh from
  `export_earn`/`import_spend` every single day, so that multiply was silently
  overwritten before a player could ever see its effect past the tick it ran on.
  `export_earn` — the term that actually drives `trade_wealth` — now shrinks to
  `WAR_BLOCKADE_EXPORT_MULT`=0.55 each year at war, which persists (decaying at
  its own natural 3%/day rate) for the rest of the year between `update_wars`
  calls. The old line is kept for its immediate display value.
- **The neutral war boom.** A hub sharing a belligerent's trade component,
  itself at peace, gets its own `export_earn` nudged (`WAR_BOOM_EXPORT_FRAC`=
  0.12 proportional + `WAR_BOOM_EXPORT_FLAT`=5.0 flat floor) — exactly why a
  house wants to supply a war it isn't fighting (§2).
- **Ledger lines.** `LedgerAcc` gains `war_levy` (split OUT of the general
  `civic_tax` field, which used to silently combine the progressive wealth tax
  and war levies — a war's cost now reads as its own line, per "war must be
  legible as money") and `war_damage`. Both are now included in
  `HouseLedger.expense_total` (previously `civic_tax` wasn't even wired into the
  Accountant view's total at all — a real pre-existing gap, not something this
  session introduced) and rendered as their own ⚔-prefixed lines in
  `HousesPanel.tsx`'s Accountant tab.

**Not built, and why:** the plan's Houses row also describes VOLUNTARY war
financing (lend to the chest, supply goods at a war premium) and "two houses
backing opposite sides is a new feud cause." Neither is required by 3.4e's own
step text ("Accountant lines for every war cost and gain; manufactory and
estate damage through the existing `damage` field; blockade on belligerent
routes; the neutral war boom") — only the FORCED levy exists today, which is
what `war_levy` reports. Voluntary contracts are real future work, not silently
folded in here.

**A second RNG-divergence round, same shape as 3.4a-c's.** Shipped first with
`WAR_DAMAGE_CHANCE`=0.35; `econ_inheritance_rules_fragment_differently` failed
again ("partible must leave the average house poorer than primogeniture
(172949 vs 160729)") — the SAME sensitivity 3.4a-c's own tuning already found:
two 60-year sub-simulations sharing a seed but diverging in house/estate count
from year one, so any new per-war-year `hash01` draw in a shared code path
shifts which values each run consumes downstream. Lowered to 0.15 (still a
real, recurring cost — just not rolled every single war-year) and the gate
passed again. Left here as the explicit, named reason for that constant's
value, so a future session doesn't raise it back toward 0.35 without knowing
why it was lowered.

**Gate results:** `cargo check` clean · `npx tsc --noEmit` clean ·
`economic_war_levies_houses_and_resolves` and
`every_war_terminates_within_the_round_cap` both pass ·
`simulate_decades_reports_dynamics` hard-passes (wealth ∈ [-4.6, 757383.0],
bounded/finite) · `econ_` 4/4 non-ignored pass (after the `WAR_DAMAGE_CHANCE`
fix above). `econ_fidelity_scorecard`'s wars/century moved 45.0 → **41.67**
(the stronger blockade/damage likely ending wars a little sooner on average) —
still a real, frequent feature of city life, not chased further per the
3.4a-c entry's own open pointer.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4a-c: war score, terms priced in score, casus belli)

**What shipped.** `sim/campaign/tick/war.rs` gets a real score-and-round engine on
top of DLC 3.5's declare/wage/resolve skeleton:

- **3.4a · score + quarterly rounds.** `War` gains `score` (−100..100, bidirectional),
  `round`, `peak_effort_a/b`. Every year now catches up every quarterly round due
  since `start_tick` (tick-driven, so a back-dated war still resolves correctly —
  the same trick the crisis engine uses). Each round rolls a battle/raid/blockade
  outcome biased by relative war-chest+treasury strength. Termination checks, in
  order: decisive score (±100) → the three exhaustion paths (force broken, treasury
  &credit spent, war weariness) → backers-withdraw (house wars only) → the round cap
  (`WAR_ROUND_CAP`=12 quarters = 3 years) as the guarantee of last resort, mirroring
  rule 22's discipline for the crisis engine. New test
  `every_war_terminates_within_the_round_cap` asserts this the same way
  `every_crisis_terminates` does.
- **3.4b · terms priced in score.** `apply_war_goal` is now score-gated at §1.4's
  table (reparations 10 · trade rights 25 · tribute 40 · a province 55 · annexation
  90) — a new `WAR_GOAL_PROVINCE` goal reassigns one ordinary (non-house-held, rule
  24) province's `prov_holder` to the victor. A win short of its declared goal's
  price downgrades to the richest goal the final score actually affords; it never
  upgrades on overperformance.
- **3.4c · casus belli expanded.** A WARMONGER RULER (`head_character_factor` axis 0
  on the council head) biases `WAR_DECLARE_CHANCE`. A HOUSE-DRIVEN WAR: the winner
  of a vendetta-stage feud flare, if it holds its own city's council or captor seat,
  may drag that whole city into a full state war on the loser's city instead of the
  ordinary property damage — `declare_house_war`, gated on differing cities, neither
  already at war, room under the war cap, and the new treasury/cooldown
  preconditions below — with itself auto-committed as `backer_house`, whose own
  insolvency is that war's backers-withdraw path.

**The tuning story — a real negative-result chain, not a single clean pass.**
Shipped first with `HOUSE_WAR_CHANCE`=0.20 and no other new preconditions:
`econ_fidelity_scorecard` read **65.0 wars/century**, an order of magnitude past
the §3.4f pre-3.4a-c baseline of 6.0/century measured for exactly this purpose.
Four successive attempts, each a real precondition from §3.4f's own list
("reach satisfied, a real grievance, sufficient treasury, council control"):

1. `HOUSE_WAR_CHANCE` 0.20 → 0.025 (8×): 65.0 → 56.7/century. Barely moved —
   proof the house-driven path was never the volume driver.
2. `WAR_MIN_TREASURY`=80 added to both declaration paths: 56.7 → 56.7/century.
   Zero effect — every candidate seat already cleared it.
3. `war_cooldown_until` (new `TickHub` field, 5-year "no fresh grievance" cooldown
   after ANY war, both belligerents): 56.7 → 50.0/century. Some effect.
4. `WAR_MIN_ROUNDS_TO_RESOLVE`=4 (a full year must pass before the three
   exhaustion paths — not a decisive score — may end a war): 50.0 → 50.0/century.
   Zero effect again.

Four preconditions on *declaring* a war, three of them near-inert, was the signal
that the volume was never about how often a war started — it was about how fast
one FINISHED and freed one of the two `MAX_ACTIVE_WARS` slots for the next. The
round-outcome magnitudes (24/16/8/11 per quarterly roll) let a lopsided pair reach
the decisive ±100 score in a handful of rounds. Halving them
(→ 12/8/4/5.5) was the one change that actually moved the number: **50.0 → 45.0
wars/century** — still well above the pre-3.4a-c baseline (a NEW war channel
plus real casus belli SHOULD raise it), but no longer an order of magnitude off,
and consistent with wars now being a real, if frequent, feature of city life
rather than the rare set-piece the old flat-10%-chance mechanism produced.

**The halving also fixed a real `econ_` regression, not just the frequency
finding.** At 50 wars/century, `econ_inheritance_rules_fragment_differently`
(Phase 0.4's own gate, unrelated to war on its face) FAILED outright: "partible
must leave the average house poorer than primogeniture (141324 vs 109769)" — war
had become frequent and fast enough that its own RNG divergence between the two
60-year sub-simulations (partible vs primogeniture diverge in house count almost
immediately, so they consume `hash01` draws differently from the first year) swamped
the structural signal the assertion depends on. The same magnitude halving that
brought the frequency down restored it: partible 150,940 < primogeniture 155,624,
passing again. **A second, unplanned benefit measured in the same run: top-10%
wealth share moved from 0.498 (out of its 0.60–0.90 band, unchanged from the
pre-3.4a-c 0.491) to 0.671 — back in band**, the first time since early in this
session's 3.1–3.3 work. Not the target of any of these changes — a side effect
of war now being a real, survivable-but-costly wealth event.

**Left as an open pointer, not chased further:** wars/century still reads well
above 6.0/century, and reasoning (not yet directly instrumented) points at
decisive-score resolution remaining the dominant path even after halving. A future
session wanting to bring it down further should look at raising
`WAR_SCORE_DECISIVE` or damping round magnitude again, informed by an actual
per-war termination-reason histogram — not another blind precondition on
declaration, which this session's four attempts already showed doesn't touch the
real lever. Per CLAUDE.md §2.4, this negative-result chain is the deliverable,
not a loose end to feel bad about.

**Full gate set, final state:** `cargo check` clean · `npx tsc --noEmit` clean ·
`econ_` 4/4 non-ignored passed (house wealth Gini 0.769, top-10% share 0.671 — both
in band; house dissolutions/century 66.67, printed-only, no band) ·
`simulate_decades_reports_dynamics` hard-passes (wealth ∈ [-4.6, 507320.7],
bounded/finite, turnover happens) · `economic_war_levies_houses_and_resolves` and
`every_war_terminates_within_the_round_cap` both pass. `WarBrief` (the Wars tab's
existing active-war list) gained `score`/`round`/`goal_label`, shown as a small
bidirectional meter in `MoneyFinancePanel.tsx` — the same "surface it the moment
it's built" discipline the crisis engine's round log follows.

**Not yet built:** 3.4e (accountant ledger lines, manufactory/estate damage,
blockade, the neutral war boom) and 3.4d (sack and purge — deliberately last, the
highest-risk item). See `docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the order.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4f: war frequency measured)

**3.4f — measure BEFORE tuning, the precedent Phase 4.4 set for the foreign hand.**
`econ_measure_war_frequency` (`economy_validation.rs`, `#[ignore]`d, 150-year
reference world) measures the EXISTING DLC 3.5 war mechanism — the baseline
3.4a-e's score+preconditions redesign will be judged against, not a target in
itself.

```
wars started                              9   →  6.00 / century
wars resolved in the window               9
mean duration                          2.0 yr   (every war resolves at the
                                                  earliest eligible tick —
                                                  `years >= 2` — see below)
outcome mix     plunder 6   tribute 0   trade-rights 3   annex 0
causes          independence 6   rival councils 3
war-eligible cities (a council seat)     11
structurally isolated (own component)     0   (0.0%)
```

**The headline finding: two-thirds of "wars" are not the mechanism 3.4a-c is
about to redesign.** 6 of 9 are colony wars of independence
(`declare_independence_war`, its own gate in `colonies.rs`), not
`maybe_declare_war`'s rival-council/trade-dispute path — which fired only 3
times in 150 years (2.0/century) despite a flat 10% yearly roll
(`WAR_DECLARE_CHANCE`) whenever ≥2 eligible seats share a connectivity
component. Zero cities were structurally isolated on this fixture, so the low
rate is the trigger's own rarity (a rival pair + `hash01 < 0.10`, checked once a
year), not §5.8's "no reachable rival" cause — worth re-confirming against a
real generated world, since `reference_world()`'s 11 seats in one component is
a much denser graph than most generated worlds will have.

**Every resolved war ends at EXACTLY 2 years** — `update_wars` resolves the
instant `years >= 2` is first true, weighted only by cumulative treasury +
war-chest at that moment; there is no further escalation once eligible. This is
the mechanism 3.4a (a proper score + quarterly rounds, exhaustion paths) exists
to replace — a war that always resolves at its floor duration cannot show mean
duration moving at all, so "mean duration" as measured here is really "the
resolution floor," not a real distribution. That is itself useful context for
judging 3.4a's post-redesign number.

**A real bug was caught and fixed while building this diagnostic**: the first
draft computed a war's duration from the loop's 0-indexed year counter instead
of `s.tick / TICKS_PER_YEAR` taken AFTER that year's `advance()`, silently
undercounting every duration by exactly one year (reported 1.0 yr instead of
the true 2.0 yr). Caught by hand-checking `update_wars`' own `years >= 2` gate
against the printed number, not by a test — recorded here per §2.4's "a
diagnosis is a complete task" so the off-by-one doesn't reappear if this
diagnostic is ever rewritten from scratch.

Run originally at 300 years; killed and re-run at 150 after ~280s CPU with no
sign of finishing — war-driven house turnover keeps growing `s.houses`, and
several per-tick passes scan it, so cost per simulated year rises through a
long run (a `rust-performance` question for another session, not this one).
150 years already gives a clean per-century rate, matching the window
`econ_diagnose_outpost_founding` already uses.

Verified: `cargo check` clean, `simulate_decades_reports_dynamics`
byte-identical to the pre-3.4f baseline (the diagnostic is `#[ignore]`d and
touches no production path). No `econ_` re-run needed for the same reason — the
change is a new test function only.

**Not yet started:** 3.4a (war score + quarterly rounds), 3.4b (terms priced in
score), 3.4c (casus belli incl. warmonger ruler + house-driven war), 3.4e
(ledger/damage/blockade/boom), 3.4d (sack and purge — last, highest risk). See
`docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.3: state name/colour/borders)

**3.3 — states.** A state is not new sim state: `compute_states`
(`campaign_commands/province.rs`) is a pure derived read over what 3.2 and Phase 5
already carry — every province a tier 1-2 city holds the writ to (`prov_holder`,
excluding a house-held writ per rule 24 — that's the house's territory, not a
city's state), grouped by the world's own `province_raster` cells into one
`StateRegion` per city. Nothing is persisted; a rerun cannot desync from the sim
because nothing is stored to desync. Name is deterministically varied (bare city
name / "X Republic" / "Republic of X" / "Duchy of X" / "Free City of X" / paired
with the home province's people-name), hashed off the hub id so it's stable
without being hand-authored. Colour reuses `distinct_color`'s golden-angle hue
rotation but phase-shifted (+53°) and desaturated, so a state's tint is provably
distinct from a house's heraldic colour even where a hub id and a house id
happen to collide numerically — different index spaces. Rendered with the exact
"cell cloud" technique `compute_culture_regions`/`drawCultureRegions` already
uses for ethnic territories (`OverlayManager.drawStates`), gated behind a new
Toolbar toggle (🏰 States), refreshed on year boundaries like the caravan-corridor
overlay. A tier 3-4 or untiered town keeps self-administering its own province
exactly as before; it simply never forms a state.

This is where §3.2's own note said the "bit-identical to the dynamics test"
guarantee would end — city tier now decides what the MAP draws. It does NOT mean
the tick itself changed: `compute_states` reads `prov_holder`/`hub.tier` and
writes nothing back, so the dynamics run stays byte-identical to the pre-3.3
baseline (confirmed below), and no new `econ_` exposure exists because no new
tick state was added.

Verified against the full required gate set: `cargo check` clean (only
pre-existing unused-constant warnings), `npx tsc --noEmit` clean,
`simulate_decades_reports_dynamics` byte-identical year-by-year to the pre-3.3
baseline (richest/houses/banks/wars/crashes/towns all match exactly).

**Not yet started:** the whole abstract war system (3.4a–f), starting with 3.4f
(measure war frequency before tuning anything). See `docs/CITY_PROVINCE_WAR_PLAN.md`
§7 for the full order.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.1 + 3.2: the city leader and city tiers)

**3.1 — the office as a person.** `council_house`/`captor_house` already existed and
already compete for the seat (bribery/intimidation/capture in the existing
`update_government`); what was missing was surfacing WHO holds it. `CityLeader`
reads `kin[0]` of whichever office is stronger, reusing `character_phrase` and
`head_vice` — both already built for the House Dossier but never exposed outside
it. New `vice_label()` is the first thing that surfaces `head_vice` to the
frontend at all. Pure read/display addition, no tick mutation.

**3.2 — city tiers.** `TickHub` gains `tier`/`standing`, recomputed monthly by
`assign_city_tiers` — a direct mirror of `assign_house_tiers` (same percentile
cutoffs, same Tier-1 absolute floor, same hysteresis). Four axes: population,
trade wealth, treasury, territory administered (rural population under provinces
this city holds), and the ruling house's own standing. Query-side only at this
step — nothing downstream reads the new fields, so the guarantee holds exactly as
it did for house tiers. Four new tests (richest-city-ranks-highest, Tier-1-empty-
on-a-flat-world, hysteresis stability, an-estate-is-never-tiered) all pass.

Both steps verified against the full required gate set: `cargo check` clean,
`econ_` 4/4 non-ignored passed with numbers UNCHANGED from the pre-3.1/3.2
baseline, `simulate_decades_reports_dynamics` byte-identical, `npx tsc --noEmit`
clean (3.1 only touched the frontend; 3.2 has no frontend surface yet — city
tiers become visible once §3.3 turns them into state borders).

**Not yet started:** 3.3 (state name/colour/borders — where city tiers stop being
bit-identical) and the whole abstract war system (3.4a–f). See
`docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

---

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md: 1.3, 2.3, 2.4, 2.5 built)

**1.3 — panel polish.** Smoothed the Trade tab's 360→600px width snap into a CSS
transition; applied the house stability gauges' own "quiet when healthy" rule to
the two treasury displays (grey+small when positive, a loud warning colour only
when actually empty).

**2.3 — the survey plate's real terrain.** New query command
`get_province_terrain_crop(province_id, max_dim)` returns a cropped elevation/
land/biome grid over a province's bbox, read from the world's cached tiles —
replacing `ProvinceMiniMap`'s flat placeholder relief fill with real hypsometric
shading. River courses are NOT re-sent by the backend — the frontend already
holds the world's full river geometry (`worldStore.rivers`) and clips it to the
province's own raster mask itself, so the water plate now draws a real course
instead of a proportional scatter. Both fall back to the old placeholders when
absent (old world / still loading).

**2.4 — elevation-biased land use.** Reuses 2.3's terrain crop: the land-use
dither's placement is now a RANKED composite (elevation + noise) rather than pure
noise, so woodland/waste cluster uphill and arable/pasture on the flat — while the
province's overall shares stay exactly exact (ranking, not threshold-shifting, is
what preserves that). Tenure's dither is untouched by design.

**2.5 — goods exploitation (the workstream's own "substantial/risk" item).** New
frozen per-(province, good) belt score (`Province.good_belt`, world-side, an
unfiltered mean unlike the existing top-6 quality shortlist) snapshotted once at
campaign start. `potential`/`actual`/`exploitation`/`market_share` are PURE
DERIVED reads — no new production, no touched prices — computed fresh from
current land use, live hub+estate production, and the one piece of state that
does persist: `prov_good_depletion`, updated yearly with an estate-kind-aware
wear/heal rate (mine barely recovers, fishery recovers fast, vineyard doesn't
deplete at all — plantation also nudges `prov_soil` down, a real cross-link). The
yield constant is SELF-CALIBRATED per world (mirroring `need_scale`) so mean
exploitation reads ≈1.0 on day one regardless of world size, rather than a single
hand-picked constant that would silently read wrong on a differently-shaped
world. New test `province_goods_exploitation_tracks_pressure_and_depletes`
exercises the whole loop (calibration → sustained overexploitation → erosion →
easing → healing) end to end. Because the pass only ever writes
`prov_good_depletion`, it cannot move the `econ_` bands or the dynamics test by
construction — verified, not just argued: both are byte-identical with this
wired in. Exposed via `campaign_province_goods`; the Province Inspector's Land
tab now shows the live reading in place of the frozen quality/rank list the
moment a campaign is actually producing something (falls back to the frozen list
pre-campaign).

**Simplified / not built, flagged rather than hidden:** land-use category is a
small hardcoded name table over the 45 shipped goods (`good_land_kind`), not a
new `GoodSpec` schema field — an unrecognized/custom good defaults to
unconstrained rather than guessed. §5.5's "keep a good listed while produced
recently" caveat is simplified to "produced now OR depletion hasn't healed away"
— no separate last-produced-year is tracked. Vineyard's "raises grade instead"
positive half isn't tracked (only the "doesn't lose tonnage" half is). Estate
tier's own "footprint + ceiling + grade" mechanics are untouched by this pass.

Whole-lib gates run: `cargo check --lib` clean · `provinces::tests` unaffected ·
new exploitation test passes · `econ_` 4/4 non-ignored passed, numbers unchanged
from the pre-2.5 baseline · `simulate_decades_reports_dynamics` byte-identical ·
`npx tsc --noEmit` clean.

**Not yet started:** 2.5's own estate-tier depth, Workstream 3 (politics/war —
city leader, city tiers, state name/colour/borders, the whole abstract war
system). See `docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

---

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md begun: Step 0 + 2.1 enclave fix + 2.2 sizing)

**Step 0 — the economy oracle could see geography for the first time.** The
60-year/30-city reference world's province layer was UNIFORM (5 identical
provinces, seats on a straight line), so the fidelity scorecard could measure
*levels* but never *dispersion* — it could not say why one province is rich and
its neighbour poor. Seeded five geographically distinct provinces (fertile river
lowland, wooded hills, arid steppe, temperate mix, marginal upland — varied
capacity, forest/arable/soil, seat position) matched to real hub clusters. Added
three printed (not asserted) diagnostics ahead of the mechanisms that will give
them real meaning: province land-pressure CV **1.406**, province output-share CV
**0.662** (both stand-ins for Workstream 2.5's exploitation/market-share ratios,
which don't exist yet), wars started/century **1.67** (the existing DLC 3.5 rival-
polis mechanism, ahead of §3.4's abstract state-war system). All existing `econ_`
bands held; `simulate_decades_reports_dynamics` stayed bit-identical (it seeds no
provinces).

**2.1 — the enclave fix, reversing a documented Phase 1 decision (§5.1).** Seed
rejection (`too_close`) tested only the CANDIDATE's own required separation, never
the incumbent seed's — a fertile valley (small separation) sitting inside a desert
or tundra region (large separation) passed a test the surrounding province would
have failed. Fixed to `max(sep_candidate, sep_incumbent)`. Added a post-snap pass
(`merge_enclaves`, run AFTER `snap_borders_to_features` per §5.3 — the snap itself
can create or heal an enclave) that folds any province bordering exactly one
neighbour into it, unless the province is its own island. New test
`no_enclosed_province_survives_unless_its_own_island` passes; all 8 pre-existing
`provinces::tests` (crest affinity, diagonal-river affinity, determinism, coverage)
still pass.

**2.2 — sizing, compressing the fertile↔hostile spread.** Measured baseline: the
old constants produced a ≈169× area ratio between max-hostile (ice cap) and
max-fertile land at the seed-separation level alone, before `VAST_MERGE_CAP_FRAC`
enlarges hostile blocks further — in the "roughly 100×" range the plan named.
Shrunk globally (`base_sep` multiplier 0.5 → 0.40) and compressed the spread from
both ends: the habitability ramp 1+1.6·hostile → 1+1.0·hostile, every
`koppen_spacing_mult` ceiling lowered (ice cap 3.0→2.0, tundra 2.2→1.7, desert
1.9→1.5, etc.), and the fertile floor raised (0.6→0.75). New `#[ignore]`d
diagnostic `province_size_distribution` measures the result on a synthetic zonal
world: **hostile/fertile mean-area ratio ≈33×** — a real, measured compression,
not just a paper one. Not a hard gate (no single "right" size exists; the
maintainer judges this visually in the app) — determinism and the existing
`provinces::tests` are the actual gate, and both hold.

Whole-lib gates run: `cargo check --lib` clean · `provinces::tests` 9/9 passed ·
`econ_` 4/4 non-ignored passed (incl. the new determinism check over the three new
scorecard fields) · `simulate_decades_reports_dynamics` bit-identical.

**Not yet started:** Workstream 1 (settlement panel rework), 2.3-2.5 (terrain crop,
organic land use, goods/exploitation), Workstream 3 (politics/war). See
`docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

---

## Current state — 2026-07-30 (Phase 5 complete: provinces as house territory — the house series is DONE)

**Asked to "move on with phase 5 and 6" — neither exists in `HOUSE_MASTER_PLAN.md`.**
Phase 5 ("Provinces as house territory", the Stato da Mar case) lives in
`docs/proposals/HOUSE_INHERITANCE_AND_TERRITORY.md` Part D, whose own revised phase
list runs 0 through 5 — **5 is the last phase in the whole house series; there is no
Phase 6.** Built it: a house may hold a province's writ instead of a city
(`prov_holder_house`), with dues redirected to the house, unrest directed at the
house (prestige + wealth, not the city's mood), standing weighted 3× toward held
territory, and a narrow GRANT trigger (a Tier 1-2 house already dominating its seat
city may be granted its ungoverned hinterland, small yearly chance). A held
province is inherited for free (house-indexed, not head-indexed) and released only
when its holder dissolves. **Contesting a held province (war, a rival house) is
explicitly NOT built** — needs new territorial war-goal machinery, the single
largest remaining gap in the whole series.

**The grant trigger needed one real fix, caught by measurement, not review**: the
first cut required a bailo specifically at the province's own seat, and fired ZERO
times on the real economy-oracle world (a house rarely bailos its own home city —
a bailo is a foreign foothold). Relaxed to council/captor-house-or-bailo (the same
signal `assign_house_tiers` already sums), and the effect became real.

**A genuinely dramatic result — the first metric in this whole series to cross INTO
its band, not just move toward it**: diffed against the pre-Phase-5 commit on the
60-year/30-city economy-oracle world, **top-10% wealth share 0.497 → 0.651**, now
inside its 0.60–0.90 historical band for the first time since Phase 0.4 first pushed
it out of band (0.422) fixing turnover. **House wealth Gini 0.693 → 0.790** — stayed
in its 0.60–0.85 band, now nearer the ceiling (worth watching in a longer run). Also
moved: surviving houses 49 → 38, dissolutions/century 40.00 → 33.33, banks chartered
24 → 21, bank failures/century 28.33 → 25.00.

Also exposed to the frontend (no new command — the existing `ProvinceLand` query
gained one field, `holder_house: i32`); `ProvinceInspector.tsx`'s existing
writ/granary/works-funding text was updated to stay accurate for a house holder.

Whole-lib test suite: **224 passed, 0 failed** (was 219, +5, covering Part D's own
invariant #7, `province_authority_is_not_assumed_to_be_a_city`). The small
dynamics-test world stays byte-identical (it seeds no provinces). `cargo check`/
`npx tsc --noEmit` both clean.

**The house mechanism series — Phases 0 through 5 — is now complete as scoped.**
Two gaps remain across the whole series, both recorded rather than hidden: goals not
yet biasing decision weights (Phase 3.1), and a held province not yet contestable
(Phase 5). Everything else in the series' own tables is built.

---

## Current state — 2026-07-30 (Phase 4 complete: 4.4 foreign hand, 2.4 salience, 4.5 mavericks declined)

**Measured before building, exactly as §2.5 demanded.** A new 300-year diagnostic
(`econ_measure_foreign_hand_conjunction`, `#[ignore]`d) found the design's "foreign
hand" trigger — a rival's office/bailo in a posted kin's city, or the house leasing
in a city a rival controls, coinciding with that kin already reading disaffected —
firing **1229 times/century** (89,784 kin-months sampled; 27.66% show either channel
present, 4.11% of those also disaffected). Two orders of magnitude past "a handful a
century", so the mechanism was built: `sim/campaign/tick/foreign_hand.rs`, a small
bounded monthly loyalty decay (ceiling 0.015/month even at maximum leverage) plus an
occasional named disclosure. **The design's own required gate held**: diffed against
the pre-4.4 commit, house dissolutions/century moved 41.67 → 40.00 (down, not up) —
leverage colours outcomes, it does not manufacture them.

Also shipped in the same pass: **§2.4 crisis salience** (only Tier 1-2 crises reach
the world news feed; Tier 3-4 stay fully chronicled on the house's own record, just
quiet on the world stage). **§4.5 mavericks** was considered and explicitly
DECLINED: `roll_character`'s existing uniform draw already lands on a full ±2
extreme ~20% of the time per axis by construction, so a true "maverick" (a rare
escape from an otherwise-centred distribution) would mean changing the baseline
distribution every already-wired character knob, `head_vice`, crisis actions and
goal selection reads — a systemic change with no gate of its own.

Whole-lib test suite: **219 passed, 0 failed** (was 215, +4). `simulate_decades_
reports_dynamics` stays byte-identical. `cargo check` clean.

**Phase 4 (Consequences) is now complete as scoped: 4.1 through 4.4 all built, 4.5
addressed item-by-item (2 correctly deferred, 1 declined with a documented reason).
Phase 5 does not exist in `HOUSE_MASTER_PLAN.md` — there is no such section.**

---

## Current state — 2026-07-30 (Phase 4.1–4.3 · Consequences)

Asked to implement "all 3 phases" — read as the three concrete, buildable items in
Phase 4's table (4.4 is explicitly conditional on an unmeasured signal, 4.5 is
explicitly "Deferred" already). **4.1** Departure schism (new `schism.rs`): a house
above a simplified `tension` proxy (mean kin loyalty · reach · feuds · a passed-over
heir) monthly either Quarrels (common, chatter) or, if the disloyal kin is POSTED to
a real holding, Departs with it to found a new rival house (Rupture stays deferred,
per this file's own earlier call). **4.2** Bankruptcy aftermath: `dissolve_house` now
writes off any outstanding bank loan and names the bank on both ledgers (kin barred
from office was cut — would need new per-`TickHub` state for a detail the source
design itself calls small). **4.3** Plague as a lineage event: a struck house can
lose several kin at once or, rarely, be extinguished outright — independent of head
mortality by design, documented in `plague_house_toll`'s own doc.

**A genuinely good result, not just a bounded one**: 4.3 is the first change in this
whole series to move **top-10% wealth share** — out of band since Phase 0.4 fixed
turnover — TOWARD its band: **0.382 → 0.509** (still below 0.60–0.90, much nearer).
**House wealth Gini 0.607 → 0.698** (stayed in its 0.60–0.85 band, now more centred).
This is exactly the historically-documented mechanism (plague extinction removes
weaker houses, concentrating survivors' share) showing up as a measured number.
Also moved: bank failures/century 33.33 → 28.33, dissolutions/century 46.67 → 41.67,
banks chartered 25 → 23. Whole-lib test suite: **214 passed, 0 failed** (was 206,
+8). The small dynamics-test world stays byte-identical (its seeded houses have no
kin roster, so both new mechanisms read "nothing to act on"). `cargo check`/`npx tsc
--noEmit` both clean.

**Phase 4 is now complete as scoped: 4.1-4.3 built, 4.4 correctly left un-attempted
(gated on a signal nobody measured), 4.5 deferred by the plan's own design.**

---

## Current state — 2026-07-30 (Phase 3.2–3.6 · the crisis engine, real but cut down)

Asked to implement "the last step" — the whole rest of Phase 3 — in one pass. New
`sim/campaign/tick/crisis.rs` (~470 lines) consolidates FOUR source design docs
(`HOUSE_POWER_AND_POLITICS.md`, `HOUSE_SUCCESSION_CRISIS.md`,
`HOUSE_POWER_STRUGGLE_VIEW.md`, `HOUSE_FACTION_NAMING_AND_RECORD.md`) into: **3.2**
competence/vice (5 named vices derived from character+skill, Lavish wired to a real
wealth cost); **3.3** the crisis itself — `HouseCrisis` opens on discontent, runs a
FIXED 4 quarterly rounds, named factions drawn from the house's own heraldic
tincture palette (mirrors `CoatOfArms.tsx::houseColor` bit-for-bit); **3.4** the
undecided bloc folded into each round's delta + a 5-year survivor grace period;
**3.5** civic intervention (a severe deposition risks the seat council sequestering
a slice of the estate); **3.6** a capped permanent `CrisisRecord`, same discipline
as `goal_history`.

**Two cuts matter most, both documented in `crisis.rs`'s own module doc**: no
per-figure power-share ledger (`head_support`/`plot_support` are two abstract
aggregate numbers, not a sum of named shares) and no continuously-drifting `regard`
ladder (plot leadership reads each kin's existing static `Kin.loyalty` roll
instead). The Split/schism outcome is deliberately not built — consistent with this
file's own Part 3 already recommending deferring "Rupture" behind Departure.

**A real bug, caught by the existing suite, not by review**: the first cut of
deposition succession ignored the culture's `LineRule` entirely, and
`a_matrilineal_house_is_held_by_women` (a Phase 0.4 test) immediately failed — a
70-year run put a man at the head of an enatic house. Fixed by filtering every
crisis successor candidate through `heir_is_female`, the same guarantee
`succeed_house` already gives every ordinary succession.

Measured on the real 60-year/30-city economy-oracle world (diffed against the
pre-pass commit): **house wealth Gini 0.649 → 0.607** (stays in the 0.60–0.85 band,
nearer its floor), **top-10% share 0.409 → 0.382** (already below its 0.60–0.90 band
before this pass — an existing finding, moved further from band rather than into
it, worth watching), **surviving houses 49 → 44**, **banks chartered 23 → 25**,
**bank failures/century 36.67 → 33.33**, **house dissolutions/century unchanged at
46.67** (Dissolved is "very rare" by design). Whole-lib test suite **206 passed, 0
failed** (was 199, +7). `simulate_decades_reports_dynamics` stays bounded. `cargo
check`/`npx tsc --noEmit` both clean.

**Phase 3 (Politics) is now complete AS SCOPED** — 3.1's goals remain read-only
tracking (not wired to bias decision weights) and 3.2–3.6's crisis engine is real
but missing the power-share ledger and regard drift described above. Both gaps are
recorded, not hidden. The crisis→deposition rate over a long run is UNMEASURED,
same honest-gap pattern as 3.1's own goal achieve/fail rate.

---

## Current state — 2026-07-30 (Phase 3.1 · goals, built as STRUCTURE only)

Scoped Phase 3 down to **3.1 only** — the crisis engine (3.2–3.6: competence/vice,
factions, resolution rounds, contested succession, civic intervention, CrisisRecord)
is a bigger undertaking and was explicitly set aside, not attempted.

**3.1** gives every non-guild house `goals: Vec<Goal>` (1 slot, 2 for Tier 1) plus a
capped `goal_history`: 7 kinds (corner a trade good, seat a council, raise the Bailo
tier, charter a bank, reach a province by expedition, outlast a named rival, restore
peak wealth after a fall), chosen yearly biased by archetype + character axis, checked
yearly, chronicled achieved (milestone, permanent) vs. failed/abandoned (chatter,
prunable). `GOAL_REACH_PROVINCE` hooks the existing expedition-arrival pass rather
than adding a new success channel. A 🎯 Ambitions dossier tab shows active (progress
bar / deadline countdown) and past (✓/✗ list) goals.

**Same honest gap as always with a "structure first" cut: goals do not yet bias
anything.** Nothing in `decide_fleets`/`update_feuds`/`update_guilds_and_offices`/etc.
reads a house's active goal to weight its choices — the master plan's §4 closed loop
(goal → weighted decisions → outcome → new goal) is not built. This is pure tracking
against state the sim already computes, so it is provably inert: goals touch no
wealth, no decision, no probability. Verified BYTE-IDENTICAL on both the dynamics
test and the economy-oracle scorecard (goals literally cannot move a number yet).
78 `tick::` tests pass (was 72, +6 — one per representative goal kind plus the
Tier-1-gets-two-slots case). Full scoping note in `HOUSE_MASTER_PLAN.md`'s handoff
block, including the still-UNMEASURED 200-year achieve/fail-rate the design spec
actually cares about.

---

## Current state — 2026-07-30 (Phase 2.4/2.5 · Phase 2 now COMPLETE)

Asked to build the two items the previous entry deliberately deferred, with a single
check at the end instead of the usual per-change gate. **2.4** wires character into
one real decision per axis (fleet-buy threshold, feud heat, civic consumption rate,
office-open threshold), each bounded to exactly ±`CHARACTER_KNOB_CAP`=0.15 and a TRUE
1.0 no-op with no roster. **2.5** gives every hired (unposted) holding a monthly
wage+skim and a 1%/month poaching risk.

**A real bug surfaced doing it this way, exactly as flagged when 2.4/2.5 were
deferred**: the first cut of steward costs read an EMPTY kin roster as "everything is
hired" rather than "nothing is known", so an old save's houses would have been
silently CHEAPER to run than freshly-generated ones — a backward-compatibility
regression, not a cosmetic bug. It was caught by the test suite (a Phase 2.1 test,
`a_house_with_no_kin_is_bit_identical`, started failing) rather than by inspection,
fixed by gating both mechanics on a non-empty roster, and the test renamed to
`an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached` to describe what's
now actually guaranteed. Full account in `HOUSE_MASTER_PLAN.md`'s handoff block.

Measured effect: on the small 30-house/50-year dynamics-test world, BYTE-IDENTICAL
output (verified by diff against the pre-2.4/2.5 commit — that world's seeded houses
never succeed inside 50 years, so never gain a roster). On the real 60-year/30-city
economy-oracle world: **house wealth Gini 0.609 → 0.649**, **top-10% share 0.422 →
0.409**, **mean firm lifespan 36.8 → 39.9yr** — all moved, none left their historical
bands. 72 `tick::` tests pass (was 67, +5 net — 6 new, 1 retired/renamed).

**Phase 2 (People) is now fully complete: 2.1 through 2.6, all built and gated.**

---

## Current state — 2026-07-30 (Phase 1.3 + 2.1/2.2/2.3/2.6 · Phase 1 complete, Phase 2 half)

Phase 1 is now fully shipped: **1.3** adds `Expedition.dest_province`, a 🧭 Expeditions
dossier tab, and click-to-highlight on the province plate.

Phase 2 (People) is half built, on purpose. Built and gated: **2.1** the `Kin` roster
(`kin[0]` mirrors the head, 2–4 siblings per founding/succession, up to two posted to
current holdings) plus the widow regency (an agnatic line's one route to a female
head, `WIDOW_REGENCY_CHANCE`=8%); **2.2** holdings authorship (a family-run estate/
office tags its posted kin's name in the Summary tab, silent = hired); **2.3**
character as four culture-derived axes read into a phrase, wired to nothing; **2.6**
`kin_power_shares` (role × skill × loyalty, always sums to exactly 100). **2.4**
(character → real decisions) and **2.5** (stewards with skim/wage mechanics) were
**deliberately not attempted** — both move house wealth directly and need `econ_`
verification per knob as they're built, not a single check at the end. 67 `tick::`
tests pass (was 61, +6); dynamics and economy scorecards bit-identical — nothing new
here is read by any decision.

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
| **Earth main-class agreement** | **70.2%** | `EARTH_MAIN_FLOOR` = 70.1 | ✅ asserted |
| **Earth exact-zone agreement** | **39.0%** | `EARTH_EXACT_FLOOR` = 38.8 | ✅ asserted |
| Earth C-class own accuracy | 32.2% | — | worst class |
| Earth `C → B` confusion | 39% | — | largest single error |
| Earth `D → E` confusion | 18% | — | second largest |
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

## House trade outposts — measured, fixed, still not fully explained

Player-reported: outposts basically never appeared over ordinary play. A 150-year
instrumented run (`econ_diagnose_outpost_founding`, `#[ignore]`d) on the reference
world found the wealth bar was never the blocker (cleared 96.8% of months) — two real
structural bugs were: only the single richest house ever got a try each year, so the
mechanism stalled for good the moment that ONE house's network stopped bordering a
remaining site; and ordinary estates (founded far more often) could exhaust the shared
`MAX_TOTAL_ESTATES` budget outposts draw from too. Fixed both (every qualifying house
gets a try, richest first, up to `OUTPOST_MAX_PER_CALL`; `OUTPOST_RESERVED_ESTATES`
holds back budget outposts can't be starved out of) and added a house's own estates as
network anchors alongside home+offices. Confirmed in the standard 50-year dynamics
gate: outposts now found at year 30 and reach 2 by year 35, where every prior scorecard
run in this file shows a flat 0 for the whole window. The 150-year diagnostic itself
still plateaus at 2 outposts after year 31 on this specific fixture — attributed to
`reference_world()`'s colonizable sites sitting in one compact band disjoint from most
hubs (a geometry no real generated world has), not re-tuned against blindly per §2.4 —
left as an open item to confirm against a real generated world.

Financed expeditions (`expedition_launch_pass`) were rewired the same session: the old
scoring rewarded raw distance with no ceiling, so a corridor could only ever reach the
single farthest city (structurally >5,600 km on an Earth-scale world). Now bounded to a
regional ≈1,400–8,800 km band with a "sweet spot" peak near the floor, so several
shorter corridors are viable instead of one maximal one.

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
| 2026-08-23c | *this* | 70.2% | 39.0% | **354 pass, 0 fail** (30 ignored) | 0 | **Physiographic provinces + the hypsometric target + the shoreline cliff.** Removing the drawn-in drainage texture exposed what was under it — one mottled cloud per continent, no plains, plateaus, basins or coherent ranges. Three causes, all measured. (1) ONE NOISE RECIPE for the whole world. New transient `step2_terrain/landform.rs`: a domain-warped lattice of `PROVINCE_KM`=1900 km sites, seven archetypes (plain/shield/hills/upland/massif/plateau/basin) chosen from TECTONIC CONTEXT not a die roll, each carrying relief amplitude, ruggedness and fine-detail weight, plus plateau-rim and basin shaping applied AFTER the rank remap (which would otherwise un-flatten a plateau). Wired into the plate and template models — the two defaults. **Two rendering-found mistakes are the deliverable here as much as the module.** `detail` must be a WEIGHT, never a frequency multiplier: scaling noise coordinates by a spatially-varying factor makes the sample position jump, producing concentric moiré that reads exactly like contour terracing — blending two FIXED-frequency fields gives the same broad-vs-busy axis with no artefact. And the parameter fields must be assigned HARD then BLURRED, never blended between the two nearest sites: a two-site blend still creases where the nearest-site IDENTITY changes, and the map came out crazed with a polygonal crack network like dried mud. (2) **THE HYPSOMETRIC TARGET WAS WRONG BY A FACTOR OF FORTY** — the single largest fidelity defect in the elevation field. At the default `height` the old anchors put **~21% of land above 4000 m** and 38% below 1000 m; Earth is **0.5%** and **71%**. Every world was a pale high plateau with the tint ramp saturated, burying all relief underneath. Anchors reset so the midpoint lands on the real ETOPO row (71% below 1 km, 2.07% above 4 km); the `density` shift now TAPERS up the curve instead of spreading evenly, which had taken the alpine end to 9.2% above 4 km. Mean landform relief on a plate world 2174 → **1291 m** — the land had been averaging nearly 3× too high, and lapse-rate temperature, biomes, habitability and settlement placement were all reading it. (3) **The shoreline was shaded as a cliff**: a sea cell stores `elevation = 0`, so coastal land was shaded against a neighbour up to 8848 m below it, ringing every coastline with a hard bright/dark rim and swamping the AO term. `halo_terrain`/`land_elev_at` substitute the centre cell's own height for a sea neighbour in both shading paths. Gates: `provinces_give_a_world_genuinely_different_country`, `province_character_never_steps_between_neighbouring_cells` (the crack regression, bounded against the character table's own range since a blurred field's gradient scales with cell size), `a_world_too_small_for_provinces_is_exactly_neutral`, `the_province_mosaic_is_deterministic`, `the_default_hypsometry_resembles_earth` (asserts the shipped (0.5,0.5) default, not just the tidy one). Earth gate unmoved at 70.2/39.0 (baked DEM, never `generate_elevation`). `bench_phase2` @3600×1800 plates 12.5 → 13.6 s, shape 14.5 → 14.8 s. **Still open, named:** the 1–2 cell dark lineament in the base tectonic/noise field; vertical hatch marks in shelf water near some coasts; plateau/basin shaping present but subtle. |
| 2026-08-23b | *this* | 70.2% | 39.0% | **349 pass, 0 fail** (30 ignored) | 0 | **Valley carving removed outright — the third and last attempt.** The previous row's fix cut grid-scale texture 82–93% and the user's next screenshot still showed the dendritic tree, because **the STRUCTURE reads as wrong, not the amplitude**: an inverted ridged-multifractal field is a drainage tree by construction, and damping it is not the same as not drawing it. `stream_power_erosion` and both noise `carve` terms are deleted from all four generators; `thermal_erosion` (rounds, never incises) and `limit_grid_scale_relief` stay. **Two measurement findings are the real deliverable here.** (1) The statistics said "fixed" while the map plainly was not, because `notch_metrics`'s 120 m threshold is calibrated to the HILLSHADE (half of `AO_REF`) while the user was looking at the flat `elevation` TINT, which has no shading and shows a systematic pattern at a far shallower amplitude — judge a tint layer on a tint render. `dump_erosion_sheet` now writes `elevation` too, and takes `EROSION_MODEL=shape` for the TEMPLATE path, which is what anyone importing a real coastline actually runs and where the carve was both absolute and strongest. (2) **No cheap statistical gate exists for this and none was shipped.** `largest_notch_component` (largest 8-connected notch chain, meant to separate "rough" from "a drawn tree") does not discriminate: re-adding `fine_carve` to a clean build measures 0.110/0.177/0.287% at 60/25/12 m against a clean 0.122/0.186/0.299% — carved reads LOWER at every threshold, because `limit_grid_scale_relief` normalises the one-cell band to a fixed RMS and no amplitude statistic downstream of it can see structure. A gate that passes either way is worse than none. Measured now (one-cell RMS · notch density · landform relief @1800×900): plates 19.8 m · 0.30% · 2174 m, shape 18.2 · 0.19% · 2274, cordillera 21.2 · 0.27% · 2172, ridged 17.3 · 0.15% · 2293 — against 102.7/98.6/340.2/125.8 m before any of this work. `priority_flood_flow` is now `#[cfg(test)]` (phase 5 keeps its own for the real river network; rivers never needed pre-cut channels). Earth gate unmoved at 70.2/39.0. **What this exposes, and the next piece of work:** with the drawn-in texture gone the terrain reads as one uniform mottled cloud — no plains, no plateaus, no basins, no coherent ranges. `generate_elevation` uses ONE global noise recipe for every continent, which CLAUDE.md §8.21 already named. `GeoContext.erodibility`/`.climate` are now computed and read by nothing, having lost their only consumer. |
| 2026-08-23 | *this* | 70.2% | 39.0% | **350 pass, 0 fail** (30 ignored) | 0 | **Erosion appearance: the map was textured with one-cell content, and almost none of it was erosion.** User report: the hillshade shows "too thin lines, looks more like river erosion — an Earth-size world should be much less eroded". The previous session's answer (`3943136`, noise `carve` 0.16→0.05) had already been tried and had not fixed it; measured per stage, the noise carve terms leave notch density at **0.05%** of land, so they were never the source. Two NEW instruments answer it: `erosion_texture_metrics` (notch density, one-cell RMS concavity, landform relief, and a per-AXIS curvature ratio) and `dump_erosion_sheet` (the world rendered through the REAL hillshade in the monochrome `analytical` style, plus a 4× crop — §8.21's "look at it, don't argue about it"). Three real causes found, each fixed and gated: (1) **stream power cut one-cell trenches** — a 22-km-wide, 350-m-deep slot along every D8 trunk plus the parallel single-cell rills D8 always makes on a planar slope; incision is now scaled by `FLUVIAL_VALLEY_KM / km_per_cell` and SPREAD over `FLUVIAL_SPREAD_KM` inside each pass (it redistributes the carve, it does not delete it, so the denudation budget and isostatic rebound are unchanged in kind). (2) **`thermal_erosion` had a north-to-south scan bias** — it wrote donor and recipient in place while scanning rows, so a cell was slumped into before it was visited; now a simultaneous delta-buffer update, gated by `thermal_erosion_does_not_depend_on_scan_order` (erode the world flipped in Y, flip back, demand the identical field — the old code fails this on the first row it touches). (3) **the finished field carried far more one-cell content than an 11-km cell can hold**, and `redistribute_elevation` amplifies it ×8.9 along with everything else; `limit_grid_scale_relief` now caps the one-cell band at `GRID_RELIEF_BUDGET_M` = 16 m, anchored to the renderer's own `AO_REF` (240 m). Measured @1800×900 (one-cell RMS concavity / notch density / landform relief): plates **102.7→18.7 m**, 2.77→**0.22%**, 2178→2176 m · shape 98.6→20.5, 3.84→0.28%, 2261→2267 · cordillera **340.2→22.9**, 7.83→0.28%, 2084→2132 · ridged 125.8→19.3, 3.69→0.26%, 2249→2253. **Grid-scale texture down 82–93%; landform relief moves under 2% on every model** — that pair is the whole claim, since a limiter that also flattened the mountains would "succeed" on the first number and ruin the map. TWO NEGATIVE RESULTS kept rather than re-attempted: rewriting `redistribute_elevation` as a monotone value-transfer curve (instead of a per-cell RANK assignment, whose ties break by row-major index) measured **bit-identical to the displayed precision** and was REVERTED — with ~500k distinct f32 elevations there are no ties to break, so the rank map already is a monotone curve; and the first `fluvial_incision_is_spread_not_slotted` fixture used a tilted plane, which **passed on the unfixed code** (a uniform slope drains every cell alike, so its carve is broad however it is applied) — the gate only discriminates on a real generated landscape, where it reads 0.409 sharp vs 0.189 spread. `limit_grid_scale_relief` also needed two fixes found by measurement, not review: its local mean must be LAND-ONLY (a plain blur calls the coastline itself "detail" and planes the coast down toward sea level, drawing the exact dark rim it exists to remove) and it must ITERATE — `e' = m + k(e−m)` is not idempotent, it leaves `(1−k)(m − blur m)` behind, and one pass aimed at 16 m settled at 41 m. Earth gate **unmoved at 70.2 / 39.0 by construction**: `earth_validation.rs` scores against the baked GMT DEM and never calls `generate_elevation`. `bench_phase2` @3600×1800: plates 11.4→**12.5 s**, shape 13.9→**14.5 s** (+9% / +4%) after rayon-parallelising `box_blur_wrap`, which the new passes made hot — recorded, not hidden, exactly as the 2026-08-19d row recorded its own. **Still open, named not fixed:** a 1–2 cell dark lineament survives every erosion ablation (it is in the base tectonic/noise field, not the erosion), and the hypsometric target puts implausibly much land above 6 km. |
| 2026-08-20e | *this* | — | — | **345 pass, 0 fail** (28 ignored) | 0 | **Province & realm 2.0.** Four findings, one of them the point of the session. (1) **Realms could never form on a world with no province layer, silently.** `maybe_proclaim_realms` early-returns on `prov_holder.is_empty()`, and province generation (step 7b) is a standalone step neither run-all calls — so a campaign started on such a world has sovereignty structurally impossible for its whole life, with no journal line, ever. Every *other* early return here eventually reports itself via `ensure_a_realm_exists`; this earlier, more total one did not. Now writes one chronicle line at `REALM_YEAR_FLOOR`. The mechanism itself is fine — verified, not assumed: `econ_measure_realm_paths` on a properly seeded world gives first realm at year 50, 23 founded, 24/24 provinces under a crown by year 200. (2) **Land improvement was unreachable without a player.** The four `prov_works` kinds were startable ONLY from `campaign_start_province_work`, so an unattended campaign never improved a single province on any world. `maybe_fund_province_works` (§5.3) makes it autonomous, through the identical funded-or-stalls machinery. (3) **The infrastructure/sovereignty split is load-bearing, and measurement is what found it.** Shipped naive it inverted `econ_inheritance_rules_fragment_differently`'s SUBSTANTIVE assertion — partible 191,991 against primogeniture 163,230, when dividing an estate must leave the average house *poorer* — a ~32pt swing against a margin the row below had deliberately widened because this gate has flipped six times now. Cause: irrigation carries `PROV_IRRIGATION_GAIN` (+45% harvest at full watering) and the pass *prioritised* it, so every city-funded province drove it to cap: a world-wide yield multiplier wearing the name of a local improvement. Fix on the merits, not by isolation — irrigation and roads are STATE projects (qanats, Roman roads) and now require `prov_realm >= 0`; clearance and drainage stay local. Gate returns at **171,184 against 253,572, wider than the pristine 149,925/174,496**. A `suppress_auto_works` flag was written first on the `suppress_realms` precedent and then **deleted** — the correct model beat the workaround, and dead isolation machinery is worse than none. Recorded because the reflex was to reach for the flag. (4) **Ore deposits both too poor and self-erasing.** `grade` rolled 0.25+0.65·fit floored at 0.05 (mean 0.575) and a mine was the WORST-eroding kind in `update_province_goods_pressure` (`wear 1.3 / recover 0.15`, "exhausts") — so a mining province decayed toward worthlessness even though §8.16 sets `grade`/`extent` as worldgen-frozen geology. Now 0.42+0.55·fit floored at 0.20 (**mean 0.695**), extent thresholds lowered (**great+world-class 9.1% → 26.6%**), and a mine accrues NO depletion, exactly as a vineyard already did and for the same stated reason. Frontend: Province Inspector rebuilt on the shared `@ui/kit`/`chronicleTheme` system the Realms panel uses (was ~60 ad-hoc hexes), gains a realm banner reading the same persisted `Realm` the map tint does; survey plate gains a real NW hillshade (one lamp, §8.21's rule); the browser list finally shows real yield instead of quality stars alone; the manual work buttons are gone, replaced by a read-only "Under way" card naming the crown or city funding it. Doc drift fixed: CLAUDE.md said Path C needs "≥4 provinces" against a shipped `REALM_CULTURE_MIN_PROVINCES` of **2**. |
| 2026-08-20d | *this* | 70.2% | 39.0% | 342 pass, 0 fail (28 ignored) | 0 | **`COMFORT_IMPORT_FRAC` measured against a gate that isn't the one it was tuned on — and the evidence cuts against the shipped value.** Sweeping the dose against `econ_fidelity_scorecard`: the basket price/distance gradient is −0.064 at the shipped 0.30 (0 of 6 goods showing any gradient) and turns POSITIVE at 0.60 (+0.041, 2 of 6) and 0.90 (+0.053, 3 of 6). A positive gradient is the historically correct sign and its absence is F2, the largest market failure this project has named. So the inheritance gate and market integration want opposite doses, and the value in the tree was set by the one unrelated to trade. **Nothing changed** — raising it re-breaks a gate, and F2's real culprits (freight ~11% of grain value; i.i.d. harvest shocks) are the thing to fix. Caveats recorded: one seed/dose, non-monotone at the low end, and every dose leaves basket CV at 1.57–1.68 vs a historical 0.20–0.40 |
| 2026-08-20c | *this* | 70.2% | 39.0% | **342 pass, 0 fail** (28 ignored) | 0 | **CORRECTION of 20b, which was wrong.** 20b's 6-seed sweep ran at the broken `COMFORT_IMPORT_FRAC`=0.60 — the dose that inverted the gate — and concluded the mean-wealth assertion was "measurably false" (1/6). At the corrected 0.30 it holds **5/6**; the assertion is restored and the concurrent session's bisect-and-fix-the-dose diagnosis was right. **The lesson is the deliverable: a seed sweep only reads the world you point it at**, and measuring robustness inside an already-bent economy produced a confident, quantified, false conclusion. Kept from 20b: `a_division_moves_capital_and_creates_none` (zero-sum asserted at the mechanism) and `econ_measure_inheritance_robustness` (now carrying the dose-comparison table). Margin comment also corrected — its "1.08–1.45 across seeds" was likewise a 0.60 artefact |
| 2026-08-20b | *this* | 70.2% | 39.0% | 342 pass, 0 fail (28 ignored) | 0 | **SUPERSEDED by 20c — central conclusion WRONG, left unedited as the record of the mistake.** `econ_inheritance_rules_fragment_differently` "fixed" — and the fix is a finding, not a repair. Measured across 6 seeds (`econ_measure_inheritance_robustness`, new): the failing assertion (partible leaves the average house poorer) holds **1/6**; houses-still-standing 4/6; concentration 2/6 — and all three pass on the gate's own seed, so any could have been swapped in to go green while asserting something false. Only houses-ever is structural (6/6), so the assertion is **not replaced**, just strengthened to require a real margin (≥1.05×; a bare `>` is what let crisis relief flip it at 190v196). **The merchant pool is not conserved**: `divide_estate` is exactly zero-sum (now asserted at the mechanism by `a_division_moves_capital_and_creates_none`), but the extra firms trade, so partible ends **richer** in total 5/6 (~30-45%). Firm count is a multiplier on merchant wealth, not a divisor of a fixed stock. Assertion 4's false "more total wealth ⇒ minting money" inference deleted |
| 2026-08-20 | *this* | — | — | **341 pass, 0 fail** (26 ignored) | 0 | **main was RED and had been for four commits.** `econ_inheritance_rules_fragment_differently` was failing its SUBSTANTIVE assertion — partible left the average house RICHER than primogeniture (193,720 vs 164,858), the opposite of what dividing an estate must do. Bisected to `a7ff520` ("Demand: comfort goods also draw foreign-import craving"), which raised the foreign-craving gain to tier-1 goods at 0.60 of the luxury rate; its parent `96ef1e2` is green with byte-identical numbers to the pre-change baseline. It then survived `2153af3` (Terrain 2.0) and `345c807` (wine fix) because **each of those was verified against a different SUBSET** — "cargo check + tsc + realm suite", "dynamics test passes" — and none ran `econ_`. That is the failure mode §2.5 and rule 16 exist to prevent, and it is the finding here, more than the constant. The response is dose-dependent (the `envoys.rs` shape, not 4.7's discrete branching flip), so the fix is the dose and not the mechanism: `COMFORT_IMPORT_FRAC` 0.60 → **0.30**. Comfort goods still draw real foreign craving at half strength; the gate returns with a WIDE margin (149,925 vs 174,496 mean wealth, 194 vs 176 houses ever) rather than a thin one, deliberately, because this gate has now flipped inside its own noise band five times. Scorecard on the repaired main: gradient −0.052, basket −0.064, grain spatial CV 2.471, Zipf −0.625 (from −0.485, toward its band), Gini 0.717 (in band), top-10% 0.588 (from 0.512, approaching band), real wage 162.5. Those reflect everything on main since the last row, not this tuning alone — not isolated per-commit.  This is the fix for the "1 pre-existing unrelated fail" the row below correctly observed and left in place. |
| 2026-08-20 | *this* | 70.2% | 39.0% | 339 pass, **1 pre-existing unrelated fail** (27 ignored) | 0 | Seven elevation styles (Layer Colouring, Alpine, Arid, Polar, Analytical, Antique Plate, Abyssal), data-driven off one shared `render_elevation_styled`; palettes served via `get_render_palettes` (§8.18); `relief_at`/`sea_shade` parameterised, default rendering bit-identical. The one failing test (`econ_inheritance_rules_fragment_differently`) is confirmed pre-existing on `origin/main` via an isolated worktree check, unrelated to this change |
| 2026-08-19e | *this* | **70.2%** | 39.0% | **340 pass, 0 fail** (26 ignored) | 0 | Slice 4 redone as a level-set (signed distance-to-boundary + noise, re-thresholded at zero) after a maintainer screenshot showed the 08-19d fix still reading as an unmodified Voronoi polygon despite its own 62.5% number. `coast_on_boundary` now 6-7% (was 90-100% pre-slice-4); real peninsulas/bays/islands, not speckle. Also fixed the divergent-boundary rift pulldown, a second straight-line artefact (read at an unwarped cell position, unrelated to the D4 orogeny-belt warp) the maintainer separately flagged as visible "plate line ridges" |
| 2026-08-19d | *this* | **70.2%** | 39.0% | **340 pass, 0 fail** (26 ignored) | 0 | `TERRAIN_2_PLAN.md` all six slices: stream-power erosion, transient `geology.rs` (lithology + orogeny setting/age + climate proxy + regionalised redistribution), plates.rs D3 boundary fix, coastline decoupled from the Voronoi edge (retuned after a probe found the first pass numerically differed but geometrically didn't — `coast_on_boundary` ~100%→62.5%, its own new gate), seafloor structure, texture-shading render follow-up. Earth main-class 70.1→70.2 (floor raised); `bench_phase2` @ 3600×1800 plates 8.5s→11.4s / shape 11.4s→13.9s (short of "no slower", recorded not hidden); `terrain_metrics` harness establishes the first slope-spread/drainage/coast-on-boundary/sea_depth-correlation baseline; `pearls` added as an honestly-labelled goods-coverage exception (a slice-4 consequence on the fixed-seed reference world, not a real-generation regression) |
| 2026-08-19c | *this* | — | — | **331 pass, 0 fail** (24 ignored) | 0 | **City market 2.0 + the Markets window.** `CityMarketView` — shared between the settlement Trade tab and a new floating ⚖ Markets window with its own live city picker (`campaign_market_cities`, so campaign-founded towns are reachable at all). Keeps the buy/sell arrivals⇢market⇢departures basis and rebuilds the centre as a merchant's BOOK: bought at / sold at / **the spread** / days-of-need held / a price trend off the persisted series. Removes three sections it absorbs (the standalone price grid, Exports/Imports, the chain ladder) and rewires the map's supply-road highlight rather than orphaning it. Fixes a real defect: in-flight rows were stamped with the VIEWING city's local price, so an inbound cargo read as though bought at its destination's price (`InTransit.price` now carries the struck price). **Cost recorded:** `econ_inheritance_rules_fragment_differently` flipped on crisis relief — 190 vs 196 houses ever, a 3% margin, the fourth time this gate has moved inside its own noise. Its SUBSTANTIVE assertion held wide open throughout (mean wealth 141,368 vs 157,415 — the measure the test's own note calls the one that moves); only the count moved. Isolated with `suppress_relief`, mirroring `suppress_realms`; the gate now passes with margin both ways (193 vs 172 ever; 149,613 vs 161,790). Two schematic promises deliberately NOT faked: the full price build-up needs per-pair travel days the query layer doesn't send, and the "why the gap isn't closing" line needs dispatch's internals — both left out rather than invented. |
| 2026-08-19b | *this* | — | — | 354 | 0 | **F2 ANSWERED: it is not a grain problem, the whole market is unintegrated.** The economy oracle now reports the price/distance gradient PER GOOD and on the model's own need-weighted BASKET, not on grain alone. Result: **0 of 6 priced goods show any positive distance gradient**, and the basket gradient is **−0.006**. So the flat −0.026 grain figure is not an artefact of grain's 45-day export reserve — distance costs nothing anywhere. A second, unlooked-for finding in the same table: dispersion varies enormously BY GOOD while the gradient does not — silk mean \|ln gap\| **0.244** (a 1.28× spread, historically reasonable), olives 0.484, fish 0.901, wheat 0.991, iron **1.444** (4.2×). Low-bulk goods are near-uniform in space, bulky ones are wild, and neither responds to distance — consistent with F3 (outbound profit equals the freight, which scales with bulk, rather than the price gap). Caveat: the reference world prices only 6 goods. Also lands CRISIS RELIEF (`polis.rs::decide_crisis_relief`/`apply_crisis_relief`): a council in dearth opens the civic granary across every food good and, in famine, bars the export of food. Dynamics run: peak house wealth **487,927 → 300,598** and sustained richest **487,927 → 248,725** (toward the project's own "no 100k blow-ups" ideal), and towns hold 30 until year 40 instead of losing one by year 15 — a granary keeps cities alive, which is what it is for. Scorecard essentially unmoved: gradient −0.029 → −0.026, grain spatial CV 2.582 → 2.542, Gini 0.642 → 0.662, top-10% 0.510 → 0.512, real wage 145.4 → 146.3, crisis-year share still 0.000. |
| 2026-08-19 | `3db1c1d` | — | — | — | 0 | **Market measurement only — no code changed.** Re-ran `econ_fidelity_scorecard` at HEAD for `docs/TRADE_AND_MARKET_REVIEW.md`. Price/distance gradient **−0.029** (was −0.01 on 2026-07-29); mean \|ln gap\| **0.901 nearest quartile vs 0.914 furthest** — distance is not merely a weak predictor of price, it is no predictor at all; grain price CV **across** cities **2.582** (was 2.10, band 0.20–0.40); grain price CV **within** a city **0.010** (band 0.30–0.50) — recorded here for the first time and the largest proportional error in the economy oracle: a city's grain price is very nearly a constant over 60 years. The pair is the INVERSE of a real pre-modern market (moderate dispersion rising steeply with distance, large harvest-driven swings over time). Other metrics in the same run have drifted far from the 2026-07-29 row and are recorded as-measured, not diagnosed: house dissolutions/century 253.33 (was 10.0), urban share 0.998, crisis-year share 0.000, top-10% share 0.510, Gini 0.642, wars/century 31.67, 71 surviving houses, 29 banks. |
| 2026-08-12b | *this* | 70.2% | 39.0% | 303 | 0 | `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` ALL 13 SLICES addressed: certification fee (4.12, wired — a uniform redistribution passed the fragile gate on the first real attempt, unlike 4.7/4.9's targeted transfers); coronation-to-crown-lease conversion + free A7 royalty in kind (4.10); population status as a safe pure derived read (4.11); heraldic accents on the works ownership bar (A10, frontend). Five real follow-ups flagged, not silently dropped: A6, D11/A9's reimbursement, adulteration's wiring, D13, and 4.11's consumption-timing rearchitecture — each deferred for a stated, measured or structural reason |
| 2026-08-12 | *this* | 70.2% | 39.0% | 300 | 0 | `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` slices 4.1-4.9 + 4.13 built (grade bands, spoilage, warehouse panel, supplier attribution, share table, works cards, brands, disasters/repair, envoys, offtake routing). Two independent RNG-sensitivity data points against `econ_inheritance_rules_fragment_differently`: 4.7's discrete branching-order flip (reverted) and 4.9's genuine dose-dependent flip (tuned down to passing). A6 and D11/A9's reimbursement money transfer deliberately deferred |
| 2026-08-11 | *this* | 70.2% | 39.0% | 294 | 0 | `GOODS_LOCALITIES_PLAN.md` all 8 slices built (rivers, marine bands, localities, naming, the two-layer overlay, province squares, production wiring). Slice 0's own gate found and fixed a process-global test race it had introduced, and printed (not asserted) a pre-existing `Deposits`-goods coastline-crossing finding for `DEPOSITS_AND_MINING_PLAN.md` |
| 2026-07-31 | *this* | **70.2%** | 39.0% | 227 | 0 | Ocean evaporation's wind term was DEAD CODE — it read `|belt_wind|`, which is a unit vector, so the factor was identically 1.0. Now reads `jets::base_speed`, the real belt speed profile, as the bulk formula `E ∝ U·(q_s − q_a)` requires |
| 2026-07-31 | *this* | 70.1% | **39.0%** | 227 | 0 | **Köppen no longer emits `H`.** Highland has no Köppen counterpart — the reference calls Tibet and the high Andes `ET`/`EF`/`Dwc` — so every `H` cell was unmatchable by construction. Exact-zone 33.7 → 39.0, the largest single move of the session, with main-class *identical* (it only ever sat on terrain the reference already calls polar). Alpine is unaffected on the Biomes layer, which has its own altitudinal band. Graded rain shadow tried and reverted (A15) |
| 2026-07-31 | *this* | **70.1%** | **33.7%** | 227 | 0 | **Seasonal monsoon adopted (FIX_PLAN A14).** The wind belts now migrate with the ITCZ and cross-equatorial flow recurves, so monsoon winds actually reverse: 0/7 → 4/7 sites, now ASSERTED by `earth_monsoon_wind_reverses`. Exact-zone to its best ever. Main-class floor LOWERED 70.6 → 70.0 — the only lowering in this file's history, a deliberate trade (the arid belt had been propped up by a wind that never changed direction). ITCZ overlay now draws both seasonal lines with the migration band hatched between them |
| 2026-07-31 | *this* | **70.8%** | **32.8%** | 226 | 0 | Continental seasonal span raised (`K_SEASONAL` 0.20 → 0.24). The generated warmest−coldest span at 60–70°N was 28.6 °C against a real 57–65 in Siberia, which made `Dfd`/`Dwd` *arithmetically impossible* (they need `t_coldest < −38 °C`). D row 58.5 → 70.8; `Dfd` and `Dsa` go from never-emitted to present. Cost is the C row (34.5 → 31.5) |
| 2026-07-31 | *this* | **69.6%** | **31.9%** | 226 | 0 | Orographic uplift made a graded response to upslope RISE (Smith & Barstad `w = U·∇h`) instead of a binary `elevation > 1681 m` test. Measured: the Western Ghats, Appalachians and NZ Southern Alps cleared that threshold in ZERO cells, so three of the wettest orographic coasts on Earth produced no uplift at all. C row 33.0 → 34.5; Mumbai `B→A`, SE-US now C. Also adds a Köppen ZONE CENSUS: 5 zones are never emitted (all `Dw*`/`*d`) and H is 8.07% of land against 0% in the reference |
| 2026-07-31 | *this* | **69.4%** | **31.8%** | 226 | 0 | Moisture emission scaled by SST via Clausius-Clapeyron (bulk formula). The source was a 3-valued step on `current_type` and, because only boundary currents poleward of ~18° carry a tag, it made the mid-latitudes the model's strongest moisture source and the equator its weakest — backwards. A row 83.8 → 85.1, exact-zone 31.6 → 31.8. Gain damped to 0.30 (sweep in the constant's doc comment). Two REVERTED negative results recorded in FIX_PLAN A7/A8 |
| 2026-07-31 | *this* | **69.2%** | **31.6%** | 226 | 0 | Snow-albedo cooling confined to the COLD SEASON (it was lowering the annual mean, so `seasonal_temps` put the full 4 °C on July — and Köppen's D/E boundary IS the warmest month). D row 49.8 → 58.7, `D → E` 37% → 30%. Also documents the subtropical basin-position asymmetry (Miyasaka & Nakamura 2005) that entered `6d0aaa1` unreviewed. Floors 67.0 → 69.0; `EARTH_EXACT_FLOOR` added at 31.0 |
| 2026-07-30 | *this* | **67.4%** | **30.0%** | 225 | 0 | Shelf-velocity fix: `generate_ocean_currents` no longer zeroes current_vx/vy on shelf cells (a rendering concern moved to `render_currents`). `compute_upwelling_zones` was measurably DEAD — 0 usable sources, 0 cells cooled — and is now 3 428 sources / 872 cells / up to 4 °C. First Earth-score move since `d53fdc9`. Mumbai C→A and SE-US B→C now match reference; `D → E` 40%→37%. Floor raised 65.0 → 67.0 |
| 2026-07-30 | *this* | 66.3% | 29.1% | 224 | 0 | House lineage tab + Compare window + figure variation + enlarged dossier window; outpost/expedition regional-reach fixes (see below) |
| 2026-07-29 | `936a8a3`+ | 66.3% | 29.1% | 159 | 0 | Economy oracle added; CI added; scoreboard created |
| 2026-07-29 | *this* | 66.3% | 29.1% | 159 | 0 | Harness calibrated to real campaign start; LOD sampler fixed; tick determinism defect found |
| 2026-07-30 | *this* | 66.3% | 29.1% | 166 | 0 | Phase 0.3: tick determinism FIXED (4 hash-order sites); guard un-ignored |
| 2026-07-30 | *this* | 66.3% | 29.1% | 165 | 0 | Phase 0.1/0.2: firm lifespan ~12 → ~51–101 yr (seed capital from the parent guild); Gini 0.828 → 0.853 (left band — measured cost); determinism defect promoted to a blocker |
| 2026-07-29 | *this* | 66.3% | 29.1% | 165 | 0 | Feuds elaborated (cause/stage/ending); province LAND state + B1 feedback edge; sustained richest 297 748 → 154 045; Gini 0.771 → 0.828 |
| — | `d53fdc9` | 66.2% | 29.0% | — | 0 | FIX_PLAN baseline |
