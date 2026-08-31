# Ports, Junctions, Provinces & Trade — fix plan

**Status: all eight slices built (2026-08-31), gated, `cargo test --lib` clean.**
See the end of this file and `docs/SCOREBOARD.md`'s 2026-08-31/2026-08-31b entries.
Eight slices in four groups, ordered so each one's gate can be read before the next
is written.

| Group | Slices | Touches |
|---|---|---|
| **A · Province view** | 1 · 2 | UI + one query command |
| **B · Worldgen placement** | 3 · 8 | `step7_settlements/`, run-alls |
| **C · Campaign trade cost** | 4 · 5 | `economy_validation.rs`, route pricing |
| **D · Colonies** | 6 · 7 | `tick/colonies.rs`, `tick/cities.rs` |

Written from a review of `sim/step7_settlements/`, `sim/shared/provinces.rs`,
`commands/query_commands/`, `sim/campaign/tick/` and the three Paradox trade models
(EU4 · Imperator · Victoria 3) held against the Mediterranean and Indian-Ocean
historical record. The findings are recorded in §0 first so a future session can
tell a diagnosis from a guess.

---

## 0. Measured findings

Read out of the code or measured at `bba19f1`, not inferred from the docs.

### F1 · The province plate draws goods 24× coarser than the relief under them

Two layers, two resolutions, same SVG:

| Plate | How it samples | Across a ~300 km province |
|---|---|---|
| Relief (base) | `get_province_terrain_crop(id, maxDim=130)` — raster for the bbox, then **real world cells** at a stride | up to **130 samples**, ≈ 2.3 km each |
| Goods (on top) | `province_good_belt_masks` — iterates **province-raster** cells, one centre-sample each | **~5 cells**, ≈ 56 km each |

`sim_commands.rs:1141` caps the province raster at 768 on the long axis, so on a
3600-wide world `step = 5` and a raster cell is 5×5 world cells ≈ 56 km — constant
at any world size by design. A province is 200–400 km across, so the goods layer
can only ever draw it **4–7 blocks wide**.

The full-resolution data exists twice over: `compute_good_belt_masks` copies the
belt column verbatim at ~11 km, and `province_raster_rle` stores the pixel-exact
partition. The plate reads neither. The raster is also built by **point-sampling
one corner cell per block** (`sim_commands.rs:1146`), not majority vote, so thin
belts vanish and edges jitter.

**No per-good area is reported anywhere.** `Province.area_km2` covers the whole
province; "how many km² of this province carries wine" appears in no query and no
panel.

### F2 · A port site is not outranked — it is not on the ballot

`compute_habitability`'s trade ladder (`settlements.rs:269–281`) already contains
most of what this plan wants:

```
estuary 1.00 · river mouth 0.92 · river port at sea 0.90 · coastal+river 0.85
confluence 0.80 · head of navigation 0.78 · salt lake 0.76
navigable inland 0.70 · harbour 0.60 · caravan oasis 0.60 · lake 0.50 · river 0.45
```

Two things stop it mattering, and the second is the real one:

1. **Weight.** It enters at `trade_score * 0.10` against climate 0.40 + fertility
   0.20 + water 0.20 + terrain 0.10. A perfect estuary contributes 0.10; farmland
   with no trade value contributes up to 0.90.
2. **The candidate set comes from the wrong field.** `generate_settlements`
   (`settlements.rs:414–428`) takes 8-neighbour **local maxima of `habitability`**.
   A river mouth beside better farmland is not a local maximum, so it is discarded
   *before* the threshold, the weight or the spacing are consulted. Raising the
   weight alone would not fix this.

Missing from the ladder entirely — both junction types, both detectors already
written elsewhere and unused in placement:

- **Strait / isthmus** — `chokepoint()` (`query_commands/mod.rs:1694`: open sea
  within 3 cells on two opposite sides), which already feeds
  `ColonizeSite.chokepoint` at +0.80, the largest colony site bonus in the game.
  Constantinople, Malacca, Hormuz, Copenhagen.
- **Mountain pass / saddle** — the test in `build_coarse_cost`
  (`mod.rs:428–447`: a cell that is a local low along one axis between higher
  flanks, ×0.45 cost discount). The Gotthard's economic history is one bridge at
  the Schöllenen gorge in the 1230s relocating European trade.

### F3 · Route shape is priced by path length, so the mode differential is discarded

`build_coarse_cost` prices coastal/shelf water 0.5, navigable trunk river 0.8,
minor river 1.4, land 4.0 + elevation×14, desert +9, pass ×0.45 — a sea : river :
road ratio of about **1 : 1.6 : 8** against Masschaele's measured **1 : 4 : 8**
(*EcHR* 46, 1993, 266–79). The road end is right.

Dijkstra then correctly finds the least-**cost** path. But
`coarse_path_len_cells` returns the path's **geometric cell length** and
`compute_route_days_matrix` converts that at one blended `days_per_cell`
(~55 km/day). The comment is explicit that this is deliberate — *"detours around
mountains/seas lengthen a leg while open routes stay ≈ the straight-line time —
economic calibration is preserved."*

So a detour is charged and the **mode** is not. Worse: the sea route is usually
the longer path in raw cells, so the correctly-chosen coastal route is billed
*more* than the overland route it beat.

Compounding it, `rebuild_routes` (`production.rs:150–186`) only uses the pathfound
`base_days` for hubs with index `< base_n` — every colony founded during a
campaign falls back to raw Euclidean distance. The hubs whose entire point is
remoteness carry the least honest cost in the model.

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

Cities one day apart and cities eleven days apart differ in price by the same
0.83. That is not a weak gradient; it is none.

But the fixture cannot show one either. `reference_world()` is 30 cities on a 6×5
grid at 9 units spacing with `world_w = 100`; the harness sets
`days_per_cell: 0.2, freight_per_day: 0.01` (`tests.rs:54`). Trade horizon =
0.24 × 100 = **24 units ≈ 4.8 days**; longest rescued route **11.5 days**; maximum
freight = **11.5% of wheat's base value**. At the campaign's blended 55 km/day
that is a **~630 km** plain with no sea and no mountains. Masschaele's ~0.25%/km
would put grain up ~150% over that longest route.

The shipped campaign uses `freight_per_day: 0.018` (80% higher) over a horizon of
0.24 × 3600 = 864 cells ≈ 175 days, where freight would reach ~3× base value — the
right historical band. **The shipped constant may be sound and the instrument may
be blind.** This is §8.15's own recorded lesson: check that the world you measured
in is not itself the thing that is broken.

The `urban share 0.999` row is not this plan's business but should not go
unrecorded: over 60 years the entire countryside moves into the cities. It needs
its own session.

### F5 · Duplication in the province view

- **"Currently worked"** (`ProvinceInspector.tsx:485`) is a strict subset of
  **"Goods of the region"** twenty lines below, which already prints
  `{actual}/yr of ~{potential}/yr · {exploitation}% worked · {market_share}% to
  market · {depletion}% depleted` per good, plus potential and world rank.
- **Three goods plates** (`goods` / `quality` / `deposits`) where the mask already
  carries 4-bit quality per cell, so shading coverage by quality is free.
- **The true-to-scale locality square** (`ProvinceMiniMap.tsx:605–670`). §8.20
  already recorded that it fills the whole plate — a staple locality is 900 km
  against a 300 km province.
- `goods`, `quality` and `deposits` are all **off by default**
  (`DEFAULT_PLATES`, `ProvinceMiniMap.tsx:115`), so the areas feature is invisible
  unless you know to toggle it.
- `ProvinceGood.rank`/`of` — "finest in the world", "#3 of 47" — is the single most
  legible fact about a province's economy and renders as small grey text.
- Stale comment: `get_province_terrain_crop` says the raster is "capped ~384 cells
  across". It is 768.

### F6 · A colony produces a photocopy of its parent, not its site

`create_market_colony` (`colonies.rs:1483`) seeds the new hub's `base_per_capita`
as the founder's basket × 0.6. The site is *chosen* for its resources —
`trade_value` dominates the scoring — and then the colony produces 60% of whatever
the metropolis produced. `ColonizeSite` carries `fertility`, a `kind_hint` and a
scalar `trade_value`; it has **no per-good vector**, so the information needed to
do better is not in the struct.

The house-outpost path gets this right: `try_found_house_outpost`
(`houses.rs:1013`) picks one good — including an unexploited-belt override that can
open a trade the world lacks — and creates a real single-commodity post. That is
the Kontor and it is correct. Only the settlement-colony path photocopies.

### F7 · The province feedback edge hands a new colony a mature countryside

`province_demography_pass` step 1 (`cities.rs:224–232`) grows `prov_rural` toward
`prov_cap` with **no member check** — an unsettled province fills to carrying
capacity and stays there. Step 3, the migration that would drain it, requires a
non-empty `members[p]`, so nothing ever leaves.

`province_land_pass` step 6 then delivers `prov_surplus` to
`province_seat_hub(p)` — defined as the **largest non-estate hub** in the province
(`cities.rs:1081`). A colony founded in an empty province is the only such hub, so
it becomes seat on its first land pass and immediately draws the whole province's
surplus into its granary and `prov_revenue` into its treasury.

With `PROV_YIELD_PER_HEAD = 0.55` against `PROV_SUBSISTENCE = 0.42`, surplus is
`rural × 0.13` at land multiplier 1.0 — roughly 24% of gross, on a rural pool
sitting at capacity. The colony itself is seeded at `COLONY_MIGRATION_FRAC` (6%) of
a ≥5,000-pop founder: a few hundred people inheriting a full province's harvest.

This is the instant-autarky problem, and it is not in the colony code. The colony
half is careful — `update_food_and_starvation`'s lifeline moves real grain out of a
named source's stock above a residents' buffer, the metropolis pays freight from
its treasury, `supply_years` resets on any break and `collapse_colony` fires on
reserve-empty plus starvation. All of that is drowned by a province feedback edge
that feeds the colony regardless.

### F8 · Why routes cannot drive placement, and what can

`compute_trade_routes(settlements, rivers, reach, max_crossing)` takes settlements
**as its input** and returns least-cost paths *between* them. Routes are a
consequence of settlements, so siting settlements at route junctions is circular.

What is settlement-independent is the **cost grid** and every landform test above.
That is what slice 3 keys off. The route-driven version already exists in the
campaign as `maybe_found_caravanserai`, which plants a waystation at the midpoint
of a long inland haul between two cities. Worldgen has no equivalent.

---

# Group A · Province view

## 1. Province goods at real resolution

**What.** Make `province_good_belt_masks` sample the way
`get_province_terrain_crop` already does: use the province raster only to find the
bounding box, then read **world cells** at a `max_dim` stride inside it. Return the
same RLE-friendly payload at the plate's own fidelity (~130 across, matching the
relief layer it is drawn over).

**Also.** Report each good's real extent, which nothing currently shows:
`area_km2` (covered cells × latitude-aware km²/cell — the conversion
`Province.area_km2` already uses) and `land_share` (fraction of the province's land
the belt reaches). Surface both on the Land tab's goods rows.

**Files.** `commands/query_commands/overlays.rs`, `types/campaign.ts`
(`ProvinceGoodMask`), `ui/world/ProvinceMiniMap.tsx`,
`ui/world/ProvinceInspector.tsx`.

**Gate.** `a_province_goods_mask_matches_the_world_belt` — every cell the mask
marks covered must have `belt >= COVERAGE_MIN_U8` in the goods tile column, read
against the shared constant, not a copy (the claim
`goods_validation::a_belt_never_crosses_the_coastline` already makes for the world
mask). Plus: the returned grid's long side must equal the relief crop's, so the two
plates cannot drift apart again.

**Risk.** Payload. A 300 km province at 130 across is ~17k cells before RLE.
Bound `max_dim` the way the terrain crop does so a pathological province cannot
blow up.

## 2. Province view cleanup

**What.**
- Delete "Currently worked" (F5 — strict subset of "Goods of the region").
- Merge the `goods` and `quality` plates into one, shaded by the belt's own
  absolute value. Keep `deposits` separate — different geology, different symbol.
  Two toggles instead of three.
- Drop the true-to-scale locality square; keep the core diamond + name.
- Add the merged goods plate to `DEFAULT_PLATES`.
- Promote `ProvinceGood.rank` ("finest in the world" / "#3 of 47") out of small
  grey text — it is the most legible economic fact the province carries.
- Fix the stale "~384 cells" comment.

**Files.** `ui/world/ProvinceInspector.tsx`, `ui/world/ProvinceMiniMap.tsx`,
`commands/sim_commands.rs` (comment only).

**Gate.** `npx tsc --noEmit`. No Rust gate — this removes UI, it does not change a
number.

**Risk.** None to the sim. The judgement call is defaulting the goods plate on; if
it reads busy in the app, default it off again and record that here.

---

# Group B · Worldgen placement

## 3. Ports & junctions — the trade-site pass

The substantive slice, and the answer to "settlements should appear at junctions,
river mouths and land→sea transitions."

### 3a · Return the trade field

`compute_habitability` computes `trade_score` per cell and throws it away. Add
`compute_habitability_fields(buf, rivers, lakes) -> HabFields { hab, trade }`,
where `trade` is the ladder value **already multiplied by the four viability
gates** (`temp_gate × winter_gate × cryo_gate × disease_gate`). Keep
`compute_habitability` as a thin wrapper so the five existing call sites
(`sim_commands.rs:509, 886, 978`, `world_buffer.rs:867`,
`goods_validation.rs:105`) are untouched.

The gates are the right ones: `temp_gate` is a flat 1.0 from 3 °C to 30 °C,
`cryo_gate` is 0.0 on an ice cap and 0.30 on tundra. So a hot, barren desert island
port survives — Hormuz had no fresh water and no vegetation and was the richest
port on earth — while nothing is ever planted on the ice. Aridity and infertility
enter only through `climate_score`/`fertility_score`, which this pass ignores by
construction. That is the point.

### 3b · Add the two missing junction types

Port both detectors into `compute_habitability`'s loop and extend the ladder:
**strait / isthmus → 0.95**, **mountain pass / saddle → 0.82**. Both are pure local
geometry — no routes, no goods, no settlements.

### 3c · Scale the water terms by the hinterland they drain

A river mouth draining a continent is Rotterdam; one draining 20 km of hillside is
a fishing village. `Hydrology.acc` is **flow accumulation per cell** and is already
computed in the settlement command. Multiply the river-mouth / estuary / river-port
rungs by a `smoothstep` of `acc` at the mouth, normalised against the world's own
maximum so it stays grid-independent. `River.order` (Strahler) is available as a
cross-check. One array lookup separates a great port from a creek.

### 3d · The pass itself — step 7a

Runs **after** `generate_settlements`, **before** `sim_generate_provinces`, so the
province partition seeds from the complete set and nothing is done twice.

```
candidates = 8-neighbour local maxima of `trade` with trade >= TRADE_SITE_MIN
sort by trade desc
greedy, skipping any within `port_min_dist` of an ALREADY-PLACED settlement
cap at TRADE_SITES_MAX
```

Exactly the structure `generate_settlements` uses, over a different field.

- `port_min_dist = min_dist * 0.6`. A port sits close to its market town —
  Ostia/Rome, Piraeus/Athens. Precedent: `river_min_dist = min_dist * 0.5`.
- `TRADE_SITES_MAX = min(24, max_settlements / 20)`. A handful, not a flood. This
  bound is the safety property that makes the slice additive.
- `Settlement.site = "port"`, a new value beside `coast`/`river`/`hills`/`plain`.
- **Size them from the junction, not from a catchment.** These sites exist because
  traffic passes, not because the land feeds them; sizing them through
  `compute_food_capacity` would re-import the farmland assumption the slice exists
  to escape. Seed them modestly — `compute_political` (step 9) already re-ranks by
  0.30 route-centrality and will give them their real standing once routes exist.

**Files.** `sim/step7_settlements/settlements.rs`, `commands/sim_commands.rs` (the
7a call in both run-alls and the standalone step),
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
tiles). The cap holds the change to a handful of towns. The value to watch is
`TRADE_SITE_MIN` — too low and every coastal cell becomes a port. Tune against
`trade_sites_are_bounded` plus a **rendered** world, never a guess (§8.23).

## 8. (Optional) Cost-grid betweenness

**What.** If 3b's local saddle/strait tests prove too blunt in a rendered world,
replace them with a real measure: sample K seed cells spread over the land, run
Dijkstra from each over the coarse cost grid, accumulate a traversal count per
cell. High traversal = a genuine junction — the Gotthard and the Khyber found
without knowing anything about cities. K = 64 on a ~700-wide coarse grid is
seconds, stays settlement-independent, and drops into 3d's candidate scoring with
no reordering.

**Do not build this speculatively.** 3b's tests are cheap and deterministic. Build
this only if a rendered world shows them missing junctions a reader can point at,
and record the finding here either way.

---

# Group C · Campaign trade cost

## 4. A large-world fixture for the economy oracle

**What.** Add a **second** reference world to `economy_validation.rs`: wide enough
that the trade horizon is thousands of kilometres, with a sea, a mountain barrier
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

**Risk.** None to the sim. Test-only, and a complete deliverable on its own even if
slice 5 is never written: it either dissolves F4's severity or sharpens it into a
real number.

## 5. Price route-days by cost, not by path length

**What.** Have `coarse_path_len_cells` return the accumulated Dijkstra **cost**
alongside the geometric length, and price route-days from cost. Recalibrate
`days_per_cell` so an all-sea route of a given physical length keeps roughly its
current travel time — then land routes become dear rather than sea routes becoming
free, which preserves the colony food-lifeline calibration that depends on current
timings. Raise the river rung from 0.8 toward Masschaele's 1 : 4 in the same change
or an immediate follow-up with its own reading.

**Also.** Fix the mid-campaign fallback (F3): pathfind on founding, or
terrain-penalise, so a colony behind a mountain range no longer costs what one down
a river costs.

**Files.** `commands/query_commands/mod.rs`, `sim/campaign/tick/production.rs`.

**Gate.** The `econ_` integration gradient must turn **positive on slice 4's
large-world fixture** — not on the current one.
`simulate_decades_reports_dynamics` stays bounded. No colony may starve that did
not starve before: watch `supply_years` and `collapse_colony` counts across the run.

**Risk.** High. This moves every freight cost in the game, and
`econ_inheritance_rules_fragment_differently` has flipped inside its own noise band
five times. **Slice 4 first** — tuning freight against a 630 km world is exactly how
§8.15's cautionary tale started.

---

# Group D · Colonies

## 6. Colony production seeded from its site

**What.** Add the site's own per-good belt vector to `ColonizeSite`, and seed
`base_per_capita` from it in `create_market_colony` the way
`try_found_house_outpost` already does for a single good. The founding motive
becomes causal — the silver site produces silver — and it is the prerequisite for a
colony behaving like an extraction economy rather than a small copy of home.

**Files.** `sim/campaign/tick/mod.rs` (`ColonizeSite`),
`commands/query_commands/mod.rs` (site construction),
`sim/campaign/tick/colonies.rs` (`create_market_colony`).

**Gate.** `a_colony_produces_what_its_site_carries` — a colony founded on a site
whose dominant belt differs from its founder's must produce that good within a
year. `econ_` bands unmoved; dynamics run bounded.

**Risk.** Low. `ColonizeSite` is serde-defaulted throughout, so an empty belt
vector reproduces today's behaviour exactly and old saves are unaffected.

## 7. Gate the province feedback edge on colony maturity

**What.** A stage-1 colony should not inherit a full province's harvest on its
first land pass (F7). Two options, and the second is better:

- Bar a colony from being `province_seat_hub` until `colony_stage >= 2`; or
- **Scale the delivered share by `colony_stage`** — a quarter at stage 1, rising to
  the full share at stage 4. The colony still gets *something* (it is the only
  authority there), but the countryside's yield accrues to a place that has grown
  into it rather than arriving with the founding fleet.

Prefer the second: it is continuous, it needs no special case in
`province_seat_hub`, and it leaves the mechanism intact for ordinary cities.

The undelivered share should **not** be quietly retained — give it the same
treatment `realm_collection_efficiency` already gives an uncollected tithe: a real
administrative loss, so nothing is created or hidden.

**Files.** `sim/campaign/tick/cities.rs` (`province_land_pass` step 6).

**Gate.** `a_young_colony_does_not_inherit_a_full_province` — a colony founded in a
province whose rural pool is at capacity must receive a fraction, not the whole,
of `prov_surplus` in its first year. Then the one that matters: the colony food
lifeline must **still bind** — `supply_years` and `collapse_colony` counts must
move, showing the supply fleet is doing work it previously did not have to.

**Risk.** Medium. This makes early colonies genuinely harder, which is the point,
but it interacts with `MAX_SETTLEMENT_COLONIES` and the collapse path. Read the
dynamics digest for colony counts before and after; a world that founds colonies
and immediately loses all of them has overshot.

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
- **Regenerating settlements after goods.** As above, plus wasted work, plus a
  province re-seed.
- **A goods-discovery mechanic.** Concealed sources and famous goods with unknown
  homelands — Europe ate cinnamon for a millennium without knowing the continent —
  fall out of a merchant-belief layer
  (`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` stage 4). Building both means
  building the same thing twice.
- **Staple right and transit tolls.** Real, and historically the engine of Cologne,
  Bruges, Hormuz and Malacca — the one thing EU4 gets structurally right that this
  codebase does not. It needs slice 3 first: a toll on a seam is worth nothing
  until towns stand on seams.
- **A seasonal sailing calendar.** The largest gap between what the codebase knows
  and what it uses: the world half computes reversing monsoon winds behind a
  hard-gated physics test (`earth_monsoon_wind_reverses`) and `sim/campaign/tick/`
  contains no notion of season, wind or sailing window. It is the mechanism behind
  segmented voyages, emporia at the wind junctions, the annual rhythm of long-haul
  trade, and Malacca's entire existence. Deferred because it is bigger than this
  plan, not because it is small.
- **The urban-share anomaly** (F4's last row, 0.110 → 0.999). Named, not touched.

---

## Order

| # | Slice | Cost | Needs | Gate reads |
|---|---|---|---|---|
| 1 | Province goods at real resolution | small | — | new mask test + `tsc` |
| 2 | Province view cleanup | small | 1 | `tsc` |
| 3 | Ports & junctions (7a) | medium | — | 4 new settlement tests + a rendered world |
| 4 | Large-world econ fixture | small | — | printed, no assertion |
| 5 | Route-days priced by cost | large | 4 | `econ_` + dynamics |
| 6 | Colony production from its site | small | — | 1 new test + `econ_` unmoved |
| 7 | Province feedback edge by maturity | medium | — | 1 new test + dynamics digest |
| 8 | Cost-grid betweenness | medium | 3 | only if 3 measures short |

Slices 1–2 are one sitting and almost entirely UI. 3, 4 and 6 are independent of
each other and of 1–2. 4 must precede 5. 7 is independent but reads best after 6,
since both change what a young colony is.

Every slice from 3 down touches generation or the tick: run the dynamics test and
`econ_` where it touches `tick/` (5, 6, 7), and **render a world** where it touches
placement (3) — per §8.23's rule that every cause found there was invisible in
review and obvious in a hillshade crop. None of these touch `step3_ocean_atmo/` or
`step4_climate/`, so the Earth gate cannot move; verify it anyway.

---

## Built (2026-08-31)

Slices 1-4 and 6-7. Full account in `docs/SCOREBOARD.md`'s dated entry; the short
version:

- **1** — `province_good_belt_masks` and `get_province_terrain_crop` now share one
  geometry function (`sim::provinces::province_sample_geom`), so the goods plate
  samples at the relief plate's own world-cell fidelity instead of the coarse
  province raster. Reports each good's `area_km2`/`land_share`.
- **2** — merged "goods"/"quality" into one plate, dropped the true-to-scale
  land-locality square (always a core diamond now), deleted "Currently worked",
  promoted a good's world rank to a `Badge`.
- **3** — `compute_habitability_fields` returns the trade ladder (3a), adds
  strait/isthmus and mountain-pass/saddle rungs (3b), scales river-mouth rungs by
  flow accumulation (3c); `generate_trade_sites` (step 7a, 3d) places a bounded,
  spaced set of junction sites the base pass cannot reach. Four gates, run
  `cargo test --lib settlements::` / `trade_site_tests`.
- **4** — `reference_world_large` (a real trade horizon, the shipped
  `freight_per_day`) confirmed F4's own diagnosis: freight over its longest route
  is 5.83× wheat's base value, where the compact fixture could only ever show
  ~11%. Printed only, no assertion changed, per the slice's own instruction.
- **6** — `ColonizeSite.belt` (serde-default empty = old behaviour exactly);
  `create_market_colony` blends 65% site-belt / 35% founder-basket.
- **7** — `colony_delivery_maturity` scales `province_land_pass`'s delivery to a
  colony seat by `colony_stage` (0.25 → 1.0); ordinary cities are maturity 1.0,
  untouched.

## Built (2026-08-31b) — slices 5 and 8, on request

The two slices above were held back deliberately (5 for its own stated risk, 8 for
its own "only if 3 measures short" instruction). Asked to finish the plan anyway,
both are now built, gated as tightly as the available harness allows, and recorded
honestly where that harness falls short of the plan's own stated gate.

- **5 (route-days by cost, F3).** `coarse_dijkstra_prev` → `coarse_dijkstra_dist_prev`
  now returns the accumulated Dijkstra COST alongside the predecessor array;
  `compute_route_days_matrix` prices every route from `dist[goal]` instead of the
  path's geometric cell count, via `cost_to_days = (days_per_cell × f) / (OPEN_SEA_
  COST × 100)` — calibrated so a PURE OPEN-SEA route keeps its old travel time
  exactly (three new unit tests in `query_commands::route_pricing_tests` measure
  this directly: an all-sea route lands within 15% of the old length-only price,
  ~1.0×; a same-distance flat-land route now costs 1.9×+ that; a navigable river
  prices at 3-5× coastal sea, matching Masschaele's ~4× rather than the old 1.6×).
  The navigable-river rung itself moved from 0.8 to 2.0 in `build_coarse_cost`. The
  F3 mid-campaign fallback (a colony/satellite founded during play has no pathfound
  `base_days` row at all) is TERRAIN-PENALISED via a new `terrain_route_mult(koppen)`
  in `rebuild_routes`'s straight-line branch — not a real path (a tick has no
  elevation, only the `koppen` a `ColonizeSite` already copies onto its `TickHub`),
  but no longer blind to climate at all.

  **What the plan's own gate asks for and what actually ran.** The gate reads "the
  `econ_` integration gradient must turn positive on slice 4's large-world fixture."
  That fixture (`reference_world_large`) is a synthetic `CampaignSim` built directly
  from hand-set hub coordinates and a hand-set `base_days` matrix — by design (§5 of
  CLAUDE.md: a tick has no tile access) it never calls `compute_route_days_matrix` or
  `build_coarse_cost` at all, so slice 5's actual code change cannot move that
  fixture's gradient one way or the other, and re-running `econ_fidelity_scorecard_
  large_world` after slice 5 landed shows it byte-for-byte unchanged from slice 4
  alone (grain gradient +0.062, 3/6 goods positive) — which is why the three
  `route_pricing_tests` above exist: they are the closest thing to that gate this
  codebase's test harness can actually run, exercising the real production
  functions directly rather than a synthetic stand-in. Confirming the repricing
  moves a REAL generated world's own economy gradient needs an end-to-end run
  (`campaign_start_sim` → `campaign_advance` → `econ_`-style measurement) that no
  existing test harness performs and this session did not build — named here rather
  than quietly claimed. `simulate_decades_reports_dynamics` and the full `econ_`
  suite (incl. `econ_inheritance_rules_fragment_differently`, which has flipped on
  tuning changes shaped like this one before) all still pass unchanged, because
  none of their fixtures exercise this code path either — a real, not a
  reassuring, silence.
- **8 (cost-grid betweenness, F8).** `compute_betweenness` (in `settlements.rs`) runs
  its own simplified, LAND-ONLY coarse Dijkstra from K=64 seed cells spread over the
  land, accumulating how often a cell lies on the shortest path between two seeds.
  `generate_trade_sites` folds it into the candidate score (`trade + 0.12 ×
  betweenness`, both the gating and the local-maxima test use the combined field,
  per the plan's own "no reordering" instruction) with its own, higher, threshold
  (`BETWEENNESS_SITE_MIN = 0.85`) so it can admit a real geographic pinch point 3b's
  local saddle/strait tests miss without ever letting ordinary land past
  `TRADE_SITE_MIN` on its own. New gate: `betweenness_finds_a_pinch_point_the_ladder_
  missed` — a "dumbbell" world (two large landmasses on one thin corridor, all at
  identical climate/fertility/elevation so the trade ladder scores every cell
  identically) proves the ladder is genuinely blind to the corridor while
  betweenness peaks there at over 3× a blob's own open interior, and a trade site
  lands in it. The one thing NOT done is the plan's own precondition (build this
  only if a rendered world shows 3b coming up short) — there is still no way to
  render a world in this session, so this was built on request rather than on that
  evidence; watch a real generated world for whether it actually adds real
  junctions 3b was already finding, or just costs the extra Dijkstra passes for
  nothing.

`cargo test --lib`: 373 pass, 0 fail (30 ignored). `earth_` gate unmoved (70.2%/
39.0%, untouched by construction). `npx tsc --noEmit` clean.
