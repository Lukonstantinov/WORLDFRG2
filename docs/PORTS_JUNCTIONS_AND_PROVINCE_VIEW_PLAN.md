# Ports, Junctions & the Province View — fix plan

**Status: proposed, nothing built.** Six slices, ordered so each one's gate can be
read before the next is written. Slices 1–2 are UI-side and cheap; 3 is the
substantive worldgen change (ports and junctions); 4–5 are the trade-cost half;
6 is optional.

Written from a review of `sim/step7_settlements/`, `sim/shared/provinces.rs`,
`commands/query_commands/`, `sim/campaign/tick/` and the three Paradox trade
models (EU4 · Imperator · Victoria 3) held against the Mediterranean/Indian-Ocean
historical record. The findings that motivate each slice are recorded in §0 so a
future session can tell a diagnosis from a guess.

---

## 0. Measured findings

Each of these was read out of the code or measured, not inferred from the docs.

### F1 · The province plate draws goods 24× coarser than the relief under them

Two layers, two resolutions, same SVG:

| Plate | How it samples | Across a ~300 km province |
|---|---|---|
| Relief (base) | `get_province_terrain_crop(id, maxDim=130)` — raster only for the bbox, then **real world cells** at a stride | up to **130 samples**, ≈ 2.3 km each |
| Goods (on top) | `province_good_belt_masks` — iterates **province-raster** cells, one centre-sample per cell | **~5 cells**, ≈ 56 km each |

`sim_commands.rs:1141` caps the province raster at 768 on the long axis, so on a
3600-wide world `step = 5` and a raster cell is 5×5 world cells ≈ 56 km — and that
holds at any world size by design. A province is 200–400 km across, so the goods
layer can only ever draw it **4–7 blocks wide**.

The full-resolution data exists twice over: the world map's own
`compute_good_belt_masks` copies the belt column verbatim at ~11 km, and
`province_raster_rle` stores the pixel-exact partition. The plate reads neither.

The raster is also built by **point-sampling one corner cell per block**
(`sim_commands.rs:1146`), not by majority vote, so thin belts vanish and edges jitter.

### F2 · A port site is not outranked — it is not on the ballot

`compute_habitability`'s trade ladder (`settlements.rs:269–281`) is genuinely good
and already contains most of what this plan wants:

```
estuary 1.00 · river mouth 0.92 · river port at sea 0.90 · coastal+river 0.85
confluence 0.80 · head of navigation 0.78 · salt lake 0.76
navigable inland 0.70 · harbour 0.60 · caravan oasis 0.60 · lake 0.50 · river 0.45
```

Two things stop it mattering, and the second is the real one:

1. **Weight.** It enters at `trade_score * 0.10` against climate 0.40 + fertility
   0.20 + water 0.20 + terrain 0.10. A perfect estuary contributes 0.10; farmland
   with no trade value contributes up to 0.90.
2. **The candidate set is generated from the wrong field.** `generate_settlements`
   (`settlements.rs:414–428`) takes 8-neighbour **local maxima of `habitability`**.
   A river mouth beside better farmland is not a local maximum of habitability, so
   it is discarded *before* the threshold or the spacing are ever consulted.
   Raising the weight alone would not fix this.

Missing from the ladder entirely, and both are junction types this plan needs:

- **Strait / isthmus** — the Hormuz, Malacca, Constantinople case. The detector
  already exists as `chokepoint()` (`query_commands/mod.rs:1694`: open sea within
  3 cells on two opposite sides) and feeds `ColonizeSite.chokepoint` at +0.80, the
  largest colony site bonus in the game. Not used in settlement placement.
- **Mountain pass / saddle** — the Gotthard, Brenner, Khyber case. The detector
  already exists in `build_coarse_cost` (`mod.rs:428–447`: a cell that is a local
  low along one axis between higher flanks, ×0.45 cost discount). Not used in
  settlement placement.

### F3 · Route shape is priced by path length, so the mode differential is discarded

`build_coarse_cost` prices coastal/shelf water 0.5, navigable trunk river 0.8,
minor river 1.4, land 4.0 + elevation×14, desert +9, pass ×0.45 — a sea : river :
road ratio of about **1 : 1.6 : 8** against Masschaele's measured **1 : 4 : 8**
(*EcHR* 46, 1993). The road end is right.

Dijkstra then correctly finds the least-**cost** path. But
`coarse_path_len_cells` returns the path's **geometric cell length**, and
`compute_route_days_matrix` converts that at one blended `days_per_cell`
(~55 km/day). The comment is explicit that this is deliberate — *"detours around
mountains/seas lengthen a leg while open routes stay ≈ the straight-line time —
economic calibration is preserved."*

So a detour is charged and the **mode** is not. Worse: the sea route is usually the
longer path in raw cells, so the correctly-chosen coastal route is billed *more*
than the overland route it beat.

Compounding it, `rebuild_routes` (`production.rs:150–186`) only uses the pathfound
`base_days` for hubs with index < `base_n` — every colony founded during a campaign
falls back to raw Euclidean distance.

### F4 · The economy oracle measures a barrier-free world ~630 km wide

Run at `bba19f1`:

| Scorecard row | Measured | Band |
|---|---|---|
| price gap × distance (grain, r) | −0.038 | positive, steep |
| mean \|ln gap\|, **nearest** quartile | 0.829 | — |
| mean \|ln gap\|, **furthest** quartile | 0.827 | — |
| basket gap × distance (r) | −0.037 | positive, steep |
| goods with any positive gradient | 0 of 6 | most |
| urban population share (seeded 0.110) | **0.999** | 0.08 – 0.15 |

Cities one day apart and cities eleven days apart differ in price by the same 0.83.
That is not a weak gradient; it is none.

But the fixture cannot show one either. `reference_world()` is 30 cities on a 6×5
grid at 9 units spacing with `world_w = 100`; the shared harness sets
`days_per_cell: 0.2, freight_per_day: 0.01` (`tests.rs:54`). Trade horizon =
0.24 × 100 = **24 units ≈ 4.8 days**; longest rescued route **11.5 days**; maximum
freight = **11.5% of wheat's base value**. At the campaign's blended 55 km/day that
is a **~630 km** plain with no sea and no mountains. Masschaele's ~0.25%/km would
put grain up ~150% over that longest route.

The shipped campaign uses `freight_per_day: 0.018` (80% higher) over a horizon of
0.24 × 3600 = 864 cells ≈ 175 days, where freight would reach ~3× base value —
in the right historical band. **The shipped constant may be sound and the
instrument may be blind.** This is §8.15's own recorded lesson: check that the
world you measured in is not itself the thing that is broken.

The `urban share 0.999` row is not about trade and is not this plan's business,
but it should not go unrecorded: over 60 years the entire countryside moves into
the cities. It needs its own session.

### F5 · Duplication in the province view

- **"Currently worked"** (`ProvinceInspector.tsx:485`) is a strict subset of
  **"Goods of the region"** twenty lines below it, which already prints
  `{actual}/yr of ~{potential}/yr · {exploitation}% worked · {market_share}% to
  market · {depletion}% depleted` per good, plus potential and rank.
- **Three goods plates** (`goods` / `quality` / `deposits`) where the mask already
  carries 4-bit quality per cell, so shading coverage by quality is free.
- **The true-to-scale locality square** (`ProvinceMiniMap.tsx:605–670`). §8.20
  already recorded that it fills the whole plate — a staple locality is 900 km
  against a 300 km province.
- `goods`, `quality` and `deposits` are all **off by default**
  (`DEFAULT_PLATES`, `ProvinceMiniMap.tsx:115`), so the areas feature is invisible
  unless you know to toggle it.
- Stale comment: `get_province_terrain_crop` says the raster is "capped ~384 cells
  across". It is 768.

### F6 · Why routes cannot drive placement, and what can

`compute_trade_routes(settlements, rivers, reach, max_crossing)` takes settlements
**as its input** and returns least-cost paths *between* them. Routes are a
consequence of settlements, so siting settlements at route junctions is circular.

What is settlement-independent is the **cost grid** and every landform test above.
That is what slice 3 keys off. The route-driven version of this idea already
exists — in the campaign, as `maybe_found_caravanserai`, which plants a waystation
at the midpoint of a long inland haul between two cities. Worldgen has no
equivalent.

---

## 1. Province goods at real resolution

**What.** Make `province_good_belt_masks` sample the way
`get_province_terrain_crop` already does: use the province raster only to find the
bounding box, then read **world cells** at a `max_dim` stride inside it. Return the
same RLE-friendly payload at the plate's own fidelity (~130 across, matching the
relief layer it is drawn over).

**Also.** Report each good's real extent, which nothing currently shows:
`area_km2` (covered cells × latitude-aware km²/cell — the same conversion
`Province.area_km2` already uses) and `land_share` (fraction of the province's land
the belt reaches). Surface both on the Land tab's goods rows.

**Files.** `commands/query_commands/overlays.rs` (`province_good_belt_masks`),
`types/campaign.ts` (`ProvinceGoodMask`), `ui/world/ProvinceMiniMap.tsx`,
`ui/world/ProvinceInspector.tsx`.

**Gate.** `a_province_goods_mask_matches_the_world_belt` — for a sampled province,
every cell the mask marks covered must have `belt >= COVERAGE_MIN_U8` in the goods
tile column, read against the shared constant, not a copy (the same claim
`goods_validation::a_belt_never_crosses_the_coastline` already makes for the world
mask). Plus: the returned grid's long side must equal the relief crop's, so the two
plates cannot drift apart again.

**Risk.** Payload. A 300 km province at 130 across is ~17k cells before RLE —
trivial. Bound `max_dim` the same way the terrain crop does so a pathological
province cannot blow up.

---

## 2. Province view cleanup

**What.**
- Delete the "Currently worked" section (F5 — strict subset of "Goods of the
  region").
- Merge the `goods` and `quality` plates into one, shaded by the belt's own
  absolute value. Keep `deposits` separate (different geology, different symbol).
  Two toggles instead of three.
- Drop the true-to-scale locality square; keep the core diamond + name.
- Add the merged goods plate to `DEFAULT_PLATES`.
- Fix the stale "~384 cells" comment.

**Files.** `ui/world/ProvinceInspector.tsx`, `ui/world/ProvinceMiniMap.tsx`,
`commands/sim_commands.rs` (comment only).

**Gate.** `npx tsc --noEmit`. No Rust gate — this removes UI, it does not change a
number.

**Risk.** None to the sim. The one judgement call is defaulting the goods plate on;
if the plate reads busy at a glance, default it off again and say so here.

---

## 3. Ports & junctions — the trade-site pass

The substantive slice, and the answer to "settlements should appear at junctions,
river mouths and land→sea transitions."

### 3a · Return the trade field

`compute_habitability` computes `trade_score` per cell and throws it away. Add
`compute_habitability_fields(buf, rivers, lakes) -> HabFields { hab, trade }`,
where `trade` is the ladder value **already multiplied by the four viability
gates** (`temp_gate × winter_gate × cryo_gate × disease_gate`). Keep
`compute_habitability` as a thin wrapper so the four existing call sites
(`sim_commands.rs:509, 886, 978`, `world_buffer.rs:867`,
`goods_validation.rs:105`) are untouched.

Gating matters and the gates are the right ones: `temp_gate` is a flat 1.0 from
3 °C to 30 °C, `cryo_gate` is 0.0 on an ice cap and 0.30 on tundra. So a hot,
barren desert island port survives (Hormuz had no fresh water and no vegetation and
was the richest port on earth) while nothing is ever planted on the ice. Aridity
and infertility enter only through `climate_score`/`fertility_score`, which this
pass ignores by construction — which is exactly the point.

### 3b · Add the two missing junction types

Port both detectors into `compute_habitability`'s loop and extend the ladder:

- **Strait / isthmus → 0.95.** Copy `chokepoint()` from
  `query_commands/mod.rs:1694`: open sea within `r` cells on two *opposite* sides.
  Constantinople, Malacca, Hormuz, Copenhagen.
- **Mountain pass / saddle → 0.82.** Copy the saddle test from `build_coarse_cost`
  (`mod.rs:428–447`): a land cell above a relief floor that is a local low along
  one axis between higher flanks. The Gotthard's whole economic history is one
  bridge at the Schöllenen gorge in the 1230s relocating European trade.

Both are pure local geometry — no routes, no goods, no settlements.

### 3c · Scale the water terms by the hinterland they drain

A river mouth draining a continent is Rotterdam; one draining 20 km of hillside is
a fishing village. `Hydrology.acc` is **flow accumulation per cell** and is already
computed in the settlement command. Multiply the river-mouth / estuary / river-port
rungs by `smoothstep` of `acc` at the mouth, normalised against the world's own
maximum so it is grid-independent. `River.order` (Strahler) is available as a
cross-check.

This is the single cheapest realism win in the slice: one array lookup separates a
great port from a creek.

### 3d · The pass itself — step 7a

Runs **after** `generate_settlements`, **before** `sim_generate_provinces` (so the
province partition seeds from the complete set and nothing is done twice).

```
candidates = 8-neighbour local maxima of `trade` with trade >= TRADE_SITE_MIN
sort by trade desc
greedy, skipping any within `port_min_dist` of an ALREADY-PLACED settlement
cap at TRADE_SITES_MAX
```

Exactly the structure `generate_settlements` already uses, over a different field.

- `port_min_dist = min_dist * 0.6`. A port and its market town sit close —
  Ostia/Rome, Piraeus/Athens. Precedent: `river_min_dist = min_dist * 0.5`.
- `TRADE_SITES_MAX = min(24, max_settlements / 20)`. A handful, not a flood. This
  bound is the safety property that makes the slice additive.
- `Settlement.site = "port"`, a new value beside `coast`/`river`/`hills`/`plain`.
- **Size them from the junction, not from their catchment.** These sites exist
  because traffic passes, not because the land feeds them — sizing them through
  `compute_food_capacity` would re-import the farmland assumption the slice exists
  to escape. Seed them modestly; `compute_political` (step 9) already re-ranks by
  0.30 route-centrality and will give them their real standing once routes exist.

**Files.** `sim/step7_settlements/settlements.rs`, `commands/sim_commands.rs`
(the 7a call in both run-alls and the standalone step),
`ui/workflow/StepSettlements.tsx` (report the count).

**Gates.** Four, and the last is the important one:

- `a_strait_town_appears_where_no_farm_would` — a synthetic world with a narrow
  desert isthmus between two seas. The base pass must place nothing on it; the
  trade pass must place exactly one town there.
- `trade_sites_respect_the_cryosphere` — no trade site in Köppen EF, on any world.
- `trade_sites_are_bounded` — count ≤ `TRADE_SITES_MAX` on a real generated world.
- `the_base_settlement_set_is_unchanged` — `generate_settlements`' own output must
  be **bit-identical** with the trade pass present. The slice is additive; if the
  base set moves, something has leaked.

**Risk.** Real but bounded. Provinces seed from settlements, so 7a changes the
province partition on newly generated worlds (saved worlds keep their stored
tiles). The cap keeps the change to a handful of towns. The judgement call to watch
is `TRADE_SITE_MIN` — set it too low and every coastal cell becomes a port; the
gate to tune against is `trade_sites_are_bounded` plus a rendered world, not a
guess.

---

## 4. Price route-days by cost, not by path length

**What.** Have `coarse_path_len_cells` return the accumulated Dijkstra **cost**
alongside the geometric length, and price route-days from cost. Recalibrate
`days_per_cell` so an all-sea route of a given physical length keeps roughly its
current travel time — then land routes become dear rather than sea routes becoming
free, which preserves the colony food-lifeline calibration that depends on current
timings.

Raise the river rung from 0.8 toward Masschaele's 1 : 4 ratio in the same change,
or in an immediate follow-up with its own reading.

**Also.** Fix the mid-campaign fallback: hubs with index ≥ `base_n` get raw
Euclidean distance, so the hubs whose entire point is remoteness have the least
honest cost in the model. Either pathfind on founding or terrain-penalise.

**Files.** `commands/query_commands/mod.rs`, `sim/campaign/tick/production.rs`.

**Gate.** The `econ_` integration gradient must turn **positive on the large-world
fixture from slice 5** (not on the current one — see F4).
`simulate_decades_reports_dynamics` stays bounded. No colony may starve that did
not starve before: watch `supply_years` and `collapse_colony` counts across the
run.

**Risk.** High. This moves every freight cost in the game, and
`econ_inheritance_rules_fragment_differently` has flipped inside its own noise band
five times. **Do slice 5 first** — tuning freight against a 630 km world is exactly
how §8.15's cautionary tale started.

---

## 5. A large-world fixture for the economy oracle

**What.** Add a **second** reference world to `economy_validation.rs`: wide enough
that the trade horizon is thousands of kilometres, with a sea, a mountain barrier,
and the shipped `freight_per_day: 0.018` rather than the harness's 0.01. Report the
integration gradient on both.

**Keep the existing compact world.** The dynamics gate is calibrated against it and
`econ_scorecard_is_deterministic` reads it; this adds an instrument, it does not
replace one.

**Files.** `sim/campaign/tick/economy_validation.rs` only.

**Gate.** Both worlds print `integration_gradient` and `basket_gradient`. The large
world's longest route must carry freight ≥ 100% of wheat's base value, matching
Masschaele's ~0.25%/km. **No assertion changes in this slice** — it is an
instrument, and per §2.5 a printed metric outside its band is a finding.

**Risk.** None to the sim. Test-only. This is the slice that makes slice 4
decidable, and it is a complete deliverable on its own even if slice 4 is never
written: it either dissolves F4's severity or sharpens it into a real number.

---

## 6. (Optional) Cost-grid betweenness

**What.** If slice 3's local saddle/strait tests prove too blunt in a rendered
world, replace them with a real measure: sample K seed cells spread over the land,
run Dijkstra from each over the coarse cost grid, accumulate a traversal count per
cell. High traversal = a genuine junction — the Gotthard and the Khyber found
without knowing anything about cities.

K = 64 on a ~700-wide coarse grid is seconds, and it stays settlement-independent,
so it drops into slice 3d's candidate scoring with no reordering.

**Do not build this speculatively.** Slice 3's local tests are cheap and
deterministic; build this only if a rendered world shows them missing junctions a
reader can point at. Record the finding here either way.

---

## Deliberately NOT built

- **Re-weighting `compute_habitability`.** F2's weight problem is real, but
  changing the five coefficients relocates every city on every world to fix a
  dozen. Slice 3 is additive for exactly this reason. Hormuz is an addition to the
  map, not a re-weighting of it.
- **Moving settlements after goods.** Technically possible — the only real
  dependency is `buf.habitability`, read by paper/ceramics/glassware
  (`biological.rs:1756, 1766, 1774`), and it is a pure function of the buffer;
  cultures do **not** depend on settlements. But it buys the wrong fact. Calicut is
  not in the pepper gardens and Hormuz grew nothing. A break-of-bulk point is
  defined by transport geometry, not by cargo.
- **Regenerating settlements after goods.** Same as above, plus wasted work, plus a
  province re-seed.
- **A goods-discovery mechanic.** Concealed sources and famous goods with unknown
  homelands (the cinnamon-bird case) fall out of a merchant-belief layer
  (`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` stage 4). Building both means
  building the same thing twice.
- **Staple right and transit tolls.** Real, historically the engine of Cologne,
  Bruges, Hormuz and Malacca, and the one thing EU4 gets structurally right that
  this codebase does not. It needs slice 3 first — a toll on a seam is worth
  nothing until towns stand on seams.
- **A seasonal sailing calendar.** The largest gap between what the codebase knows
  and what it uses: the world half computes reversing monsoon winds behind a
  hard-gated physics test (`earth_monsoon_wind_reverses`) and `sim/campaign/tick/`
  never reads any of it. Deferred because it is a bigger change than this plan, not
  because it is small — it is the mechanism behind segmented voyages, emporia at
  the wind junctions, and Malacca's entire existence.

---

## Order

| # | Slice | Cost | Needs | Gate reads |
|---|---|---|---|---|
| 1 | Province goods at real resolution | small | — | new mask test + `tsc` |
| 2 | Province view cleanup | small | 1 | `tsc` |
| 3 | Ports & junctions (7a) | medium | — | 4 new settlement tests |
| 5 | Large-world econ fixture | small | — | printed, no assertion |
| 4 | Route-days priced by cost | large | 5 | `econ_` + dynamics |
| 6 | Cost-grid betweenness | medium | 3 | only if 3 measures short |

1–2 are one sitting and are almost entirely UI. 3 is independent of them and can
run in parallel. 5 must precede 4. 6 is conditional on 3's rendered result.

Every slice from 3 down touches generation or the tick, so each needs
`cargo test --lib earth_` where it touches step 3/4 (none do), the dynamics run and
`econ_` where it touches `tick/` (4 does), and a **rendered world** where it
touches placement (3 does) — per §8.23's rule that every cause found there was
invisible in review and obvious in a hillshade crop.
