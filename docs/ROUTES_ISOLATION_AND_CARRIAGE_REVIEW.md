# Routes, Isolation & Carriage — a review, five measured bugs, and a plan

**Status: STAGE A + B BUILT AND GATED; STAGE C's OWN PREREQUISITE BUILT; STAGE C
STARTED (C1 BUILT AND MEASURED).** This began as analysis only; §10's questions have
since been answered (A → B → C order, isolated markets may starve, a ~3,000 km regional
trade horizon, `econ_inheritance_rules_fragment_differently` to go multi-seed before any
Stage C dose walk) and Stages A and B are now implemented — see each item's own status
line below. `econ_inheritance_rules_fragment_differently` is now multi-seed
(`INHERITANCE_GATE_SEEDS = [42, 1337, 7]`, both of the gate's own assertions checked per
seed plus a seed-averaged margin, ~6 min in debug) — see §8.15 of `CLAUDE.md` for the
gate's own description and a fresh 6-seed robustness re-measurement (both contrasts now
6/6, up from the stale 5/6 this doc's own §10 Q1 answer was written against). **C1 is
now built**: `COASTAL_SEA_COST` 0.5 → 1.6 (see its own status line under §9), measured
on a real generated world (`real_world_price_distance_gradient`,
`WORLD_AND_TRADE_MASTER_PLAN.md` §4's "UPDATE 2") to nearly DOUBLE the grain price/
distance gradient on the SAME seed (r = 0.092 → 0.185, both positive, the historically
correct sign) — a genuine, paired, attributable result on the metric this whole review
exists to move. **C1b is also now wired**, at zero dose (`LAND_BULK_PENALTY`,
`good_freight`'s new `sea` parameter) — a true no-op, proven by `cargo test --lib
tick::tests`/`econ_` staying bit-identical, and NOT yet dosed (see its own status line
under §9). C2–C4 and Stage D remain unstarted. It follows §2.4's rules: every
proposal carries a **gate that is not its own target**, and the findings are written
down whether or not anyone acts.

It goes *underneath* `docs/TRADE_AND_MARKET_REVIEW.md` (which reviews the price
mechanism) and beside `docs/TECTONICS_AND_ISOLATION_PLAN.md` Part A (which
diagnosed one half of the isolation bug and fixed the wrong door).

Baseline: `cargo check --lib` clean at `d8cd026`.

---

## 0. The headline, before any of the detail

**The economy's problem is not missing mechanism. Eleven built, wired, gated
mechanisms ship at a dose of exactly zero.**

| Constant | Ships at | What it would do |
|---|---|---|
| `N1_LOCAL_HAUL_BIND_DAYS` | `INFINITY` | make the ownerless 96% of cargo need a carrier |
| `N1B_OWNERLESS_LOSS_RATE` | `0.0` | let ownerless cargo sink |
| `N2_BAN_PRICE_RATIO` | `INFINITY` | export bans |
| `CAPACITY_BIND_DOSE` | `0.0` | make a large shipment need more than one hull |
| `DEMAND_ELASTICITY` | `[0,0,0]` | price-elastic demand (F5) |
| `PROD_ELASTICITY` | `0.0` | the price → production loop |
| `ORE_CEILING_DOSE` | `0.0` | mine output from the ore body, not from population |
| `HOUSEHOLD_MONETIZATION_DOSE` | `0.0` | households pay for what they eat |
| `WH_RELEASE_DOSE` | `0.0` | warehouse speculation has a payoff |
| `LEAGUE_BOYCOTT_MAX` | `0` | league boycotts |
| `GUILD_CHARTER_RANGE_DAYS` | `INFINITY` | a guild charter is regional |

Every feedback loop that would make this a market rather than a set of accounting
identities is present in the tree and switched off. Each was individually correct
to ship at zero — each is documented as gated bit-identical so it could not break
`econ_inheritance_rules_fragment_differently` or the hard wealth bound. But the
cumulative result is that **production ignores price, demand ignores price,
carriage ignores capacity, and storage ignores price**. That is precisely
`TRADE_AND_MARKET_REVIEW.md` F1's measured diagnosis — grain price CV within a
city **0.010** against a 0.30–0.50 band, and a price/distance gradient of
**−0.064** where history says positive — restated as a cause rather than a symptom.

**So the first economy question is not "what should we build next". It is "why has
the dose walk stalled, and what is the instrument that keeps blocking it".** That
instrument is `econ_inheritance_rules_fragment_differently`: a single-seed,
60-year, wealth-comparison gate, documented in CLAUDE.md §8.15 as having been
perturbed five times, which now stands between the project and eleven separate
mechanisms. It is load-bearing far beyond what it was designed to measure.

---

## 1. Do trade routes include mountain passes?

**Yes — and at a resolution that cannot see one.**

The mechanism is real (`query_commands/mod.rs`, the saddle block): a land cell
above `0.33` normalised elevation that is a local low along either axis, with a
flank difference over `0.04`, gets `cost *= 0.45`. Caravans do thread saddles
rather than climbing.

Two things defeat it:

- **The coarse cell is 55.7 km wide** (`f = grid_w / 700` → `f = 5` at the default
  3600×1800, cell 11.13 km). The Brenner is ~1.4 km across at the col, the Khyber
  ~1 km, St Gotthard likewise. **A real pass is between one twentieth and one
  fiftieth of one coarse cell.** The saddle test is comparing 55-km blocks, so what
  it finds is a broad regional sag, not a pass.
- **Elevation is POINT-SAMPLED from each block's centre fine cell**, not averaged
  or minimised:
  ```rust
  let wx = (cx as u32 * f + f / 2).min(grid_w - 1);
  elev[ci] = f_elev[gi];
  ```
  So whether a 55-km block reads as passable is decided by whichever single cell
  happens to sit at its centre. A block containing a perfectly good pass reads as
  a wall if its centre cell is a peak, and vice versa. **Route quality through
  mountains is currently luck.**

**The fix is one line of intent, not a new mechanism: a pass is a MINIMUM, so
sample the block's minimum land elevation for traversal cost.** Keep the mean (or
the centre sample) for everything the block is genuinely an average of — climate,
hazard, temperature. A block that contains a pass then *is* cheap, because it
contains a low cell, and the existing saddle discount stops being the only thing
carrying the idea. This also makes the discount's thresholds meaningful for the
first time, since they would be reading a real low rather than a sample.

Historically this matters more than it sounds: passes do not merely let trade
through, they **concentrate it into a few named corridors**, and the towns at
their feet — Chur, Susa, Innsbruck, Como — grew rich purely on transit. The code
already has the machinery to express that (`maybe_found_route_post` with
`ROUTE_POST_JUNCTION_KM` prefers a real chokepoint), so the payoff is already
built and waiting on a cost grid that can find a pass.

---

## 2. Rivers — the ratio is right, and it reaches almost nothing

**Credit first: the sea:river:road ratio is correctly implemented**, and cited to
the right source. In the cost grid: coastal sea `0.5`, navigable trunk `2.0`, minor
river `1.4`, base land `4.0 + elev*14.0`. That is **1 : 4 : 8**, which is
Masschaele's measured English figure (*EcHR* 46, 1993, 266–79). The code says so
and means it. `rivers_json` is also now genuinely wired into the campaign matrix
via `metadata["rivers"]` — the old hardcoded `""` is gone.

Three things then throw most of it away:

- **It reaches only the founding hubs.** The ratio lives in `base_days`, built once
  by `compute_route_days_matrix`. In `rebuild_routes`, any hub with index `>= base_n`
  — *every colony, outpost, route post and satellite founded during the campaign* —
  falls to `dist * days_per_cell * terrain_route_mult(koppen)`. A straight line with
  a climate multiplier. **It has no elevation and no rivers**, and the code says so
  plainly: *"Not a real path — it has no elevation to read a mountain range from."*
  So the longer a campaign runs, the larger the share of its own trade network that
  is routed by straight line.

- **The ratio is applied to DAYS, and days then does double duty as freight.**
  `good_freight(g, rate, days) = rate*days*bulk + perishable*days + VICTUAL*days`.
  There is **no mode term anywhere in freight** — literally true, and the
  conclusion first drawn from it here was WRONG. See the correction block below,
  which supersedes it: the mode ratio DOES reach freight, through `days`.

  What survives is narrower and still real: the mode penalty is
  **proportional, never differential.** Wheat (`bulk 3.0`) pays 3× silk's freight
  on either mode, and road costs 8× sea for both alike. Historically road
  carriage is *disqualifying* for grain specifically and merely expensive for
  silk — a pack train over the Alps carrying silk was ordinary commerce, the
  same train carrying wheat was not. That differential is what the model lacks;
  the ratio itself is present and correct.

> ### CORRECTION (same session, before anything was built)
>
> The paragraph above originally claimed 300 km of grain carriage costs **30%**
> of its value, that overland bulk freight is **~3× too cheap**, and that it is
> **"exactly as cheap by sea."** All three were wrong, and the cause was using
> the NOMINAL `days_per_cell` (55 km/day) instead of the ROUTED speed the cost
> grid actually produces. Re-derived from `cost_to_days = days_per_cell · f /
> (OPEN_SEA_COST · 100)`, one coarse cell at cost `c` takes `c·100·cost_to_days`
> days, so the real effective speeds at the default 3600-wide grid are:
>
> | medium | cost | km/day | real pre-modern effective average |
> |---|---|---|---|
> | calm coastal sea | 0.50 | **242.0** | ~50–100 (fast passage ~150) |
> | open sea | 2.20 | 55.0 | ″ |
> | navigable river | 2.00 | 60.5 | ~40–60 downstream |
> | flat temperate land | 4.00 | 30.2 | cart ~15–20, pack ~25–40 |
> | steppe (campaign matrix) | 4.50 | 26.9 | ″ |
> | desert | 6.00 | 20.2 | ″ |
> | hills (e = 0.3) | 8.20 | 14.8 | ″ |
>
> Wheat freight is `0.055/day`, so the corrected figures are **54.5% of grain's
> value over 300 km overland** (not 30%), **6.8% over 300 km of coastal sea**, and
> **181.8% over 1,000 km overland**. Against a real cart that roughly doubled
> grain's price over 150–300 km, flat-land carriage is about **2× too cheap**, and
> on ordinary hilly ground (112% at 300 km) it is **already about right**. The
> road:sea ratio measures **8:1**, exactly Masschaele.
>
> **So the diagnosis moves, and so does the fix.** The problem is NOT that road
> freight is cheap; it is that **calm coastal sea runs at 242 km/day**, 2.4–4.8×
> the real effective average, which makes every long sea lane far too cheap in
> ABSOLUTE terms while the ratio stays right. The lever is therefore the
> coastal-sea cost rung (`0.5`) or the 55 km/day reference speed — **not**
> `freight_per_day`, and not a new mode term in `good_freight`. C1 below is
> rewritten accordingly.
>
> One thing this correction *removes* from the charge sheet: a 4,000 km sea
> crossing costs 91% of grain's value against 1,000 km overland at 182%, so the
> model does prefer the long sea haul — and **that is historically correct**
> (Baltic grain reached Amsterdam while inland Polish grain could not). The
> trans-oceanic problem is §5's component bug making two continents one market
> at all; it is not a freight mispricing.
>
> **The lesson, which is the reusable part:** the campaign's `days` are cost
> units divided by `OPEN_SEA_COST`, so `days_per_cell` is a REFERENCE speed
> attached to cost 2.2, never the speed anything actually travels at. Any freight
> or travel-time arithmetic that reads `days_per_cell` as "the speed" is wrong by
> the ratio of that medium's cost to 2.2 — 8× for coastal sea, 1.8× for flat land.

- **`TickHub.river` buys a city nothing.** Every reader of it (`production.rs`,
  `houses.rs`, `read_hubs.rs`) uses it either as a **display label** — which of two
  land modes to name — or for loss-rate blending, plus one real use in yard siting
  (`coastal || river`). It never touches freight and never touches capacity. A
  river city has no cheaper carriage than an inland one.

---

## 3. Caravans carry less than boats — declared, not wired

The constants are right and the comment is right: `SHIP_CAPACITY 120` (bulk
carrier) / `BOAT_CAPACITY 70` / `CARAVAN_CAPACITY 40` (least). Three separate
things stop any of it binding:

1. **`CAPACITY_BIND_DOSE = 0.0`.** `capacity_bind_extra_slots` returns 0, so
   `need = 1 + 0`. **A shipment of any size reserves exactly one slot.** Quantity
   and capacity are unrelated.
2. **96% of shipments have no carrier at all** (`econ_measure_carrier_mix`: houses
   4.3%, ownerless 95.7%). The capacity check sits inside the carrier-resolution
   loop; the ownerless branch never reaches it, and `surplus -= amount` sits
   *outside* that loop, so the cargo moves regardless.
3. **`cap_land` pools boats and caravans**, so even where a slot binds, the
   boat/caravan distinction — the one this question is about — is averaged away.

So `SHIP/BOAT/CARAVAN_CAPACITY` are read only by futures-contract delivery and
yard valuation. **Vessel capacity has no influence on what moves.**

**And the ratio itself is historically too flat.** A 15th-c. cog carried 150–200
tons; a hundred-camel caravan carried 15–20 tons. That is **~10–13 : 1**, and
against a great carrack far more. The model's 120 : 40 is **3 : 1**. Braudel's
point — that water transport is not *better* than land but *categorically
different in kind* — is the thing the flat ratio erases.

---

## 4. Why some settlements are never connected to a route

**Found precisely, and it is a missing fallback, not a scoring problem**
(`routing.rs`, `compute_trade_routes`).

Every non-hub settlement gets exactly **one** candidate link, to its nearest hub —
and `nearest_hub` picks by **straight-line distance**, with no knowledge of whether
a route exists. That single candidate is then dropped silently on any of three
conditions, with no retry:

```rust
let path = match coarse_dijkstra(&cc, start, goal) { Some(p) => p, None => continue };
if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }
```

- Dijkstra finds nothing → `continue`, town unconnected.
- The path violates the trade reach (its open-water run exceeds `max_crossing`) →
  `continue`, town unconnected. **This is the common case**: the nearest hub by
  straight line is very often across a strait or a bay, and picking it by air
  distance is what walks into the reach limit.
- **`start == goal`** — two settlements inside one 55-km coarse cell — produces
  `path.len() < 2` → `None` → `continue`. Clustered towns drop silently.

The comment above the loop claims the opposite guarantee: *"every remaining
settlement still gets at least one minor road … so no town is left unconnected"*
and *"This guarantees every settlement is attached to the major trade routes."*
**Neither is true as written.** The intent is right; there is simply no code that
enforces it.

The fix is small and has an obvious shape: pick the nearest hub **by routed cost,
not by air distance** (which one single-source Dijkstra per settlement gives for
free — see §8), and if the best candidate is rejected, **fall through to the next**
until one is accepted or the list is exhausted. A town with genuinely no legal
route should then be *reported*, not silently omitted — rule 35's discipline
("never silently drop a datum the UI is simultaneously listing") applied here.

---

## 5. Distant continents — the bug, and why the existing guard misses it

This is the most consequential finding in the document.

**Components are built from pure Euclidean distance. Land and sea are never
consulted.** `campaign_commands/lifecycle.rs`, the component build:

```rust
let max_link = (world_w * 0.30).max(1.0);
...
for &(j, dd) in scratch.iter().take(COMP_K) {
    if dd <= max_link2 { uf_union(&mut parent, i, j); }
}
```

`d2` is straight-line cylindrical distance. At the default 3600-wide world:

| | cells | km |
|---|---|---|
| `max_link` = 0.30 × world_w | 1080 | **12,022** |
| trade horizon = 0.24 × world_w | 864 | 9,618 |
| `ISOLATION_RESCUE_MAX_KM` (tick-time only) | — | 1,800 |

| Ocean | width | unioned? |
|---|---|---|
| Atlantic, Brazil–Africa | 2,900 km | **yes** |
| Atlantic, Iberia–Newfoundland | 4,000 km | **yes** |
| Indian, Africa–India | 4,500 km | **yes** |
| Pacific, Asia–Americas | 12,000 km | **yes** |

**Every ocean on Earth is narrower than the union radius**, and union-find is
transitive, so the whole world collapses into **one component** on any Earth-like
map. Consequences, all of which are the user's reported symptoms:

- Every `hubs[a].component == hubs[b].component` guard — in `rebuild_routes` #6
  (guaranteed partners), #6b (market lifeline), `maybe_found_route_post`,
  `urban_exodus_pass` — **is a no-op**. They read as careful geography and enforce
  nothing.
- #6 and #6b then draw **straight-line trans-oceanic lanes**, which is exactly
  what their own comments say they exist to prevent: *"Crossing the component by
  straight line was drawing dishonest trans-oceanic arrows between two separate
  continents."*
- A remote continent can never form its own closed market, because it is not a
  distinct market.

**And the start-time tiny-component rescue is still unbounded** — the very bug
`TECTONICS_AND_ISOLATION_PLAN.md` Part A was written to fix:

```rust
// Fuse each into the nearest SUBSTANTIAL market (any distance) so every city
// is on a trading network.
let mut bj = None; let mut bd = f32::INFINITY;
```

Part A **was** implemented — but only on the *tick-time* twin,
`CampaignSim::rescue_tiny_components`, which now correctly caps at
`ISOLATION_RESCUE_MAX_KM`. The **start-time** function in `lifecycle.rs` was never
touched. Two functions, same job, one fixed.

**The guard test cannot catch this.**
`rescue_tiny_components_never_crosses_an_ocean` hands the tick-time function a
world with components **already assigned** (`0, 1, 2, 3`) and asserts it does not
merge them wrongly. It never runs the component *build*. So the fix is guarded at
the second door while the first door stands open — and the 30% Euclidean union
means the first door is what decides everything.

**A component is a claim about reachability and must be built from reachability.**
The right source already exists and is already computed: `coarse_dijkstra` over the
coarse cost grid, with `SEA_BLOCK_COST`/`OPEN_SEA_COST` and the `is_open_sea`
crossing rule that `path_allowed` already implements. Two hubs are in one component
if a *legal* route joins them under the world's own trade reach — not if a ruler
laid on the map says they are close.

---

## 6. Outposts and colonies land too far away

Three separate causes, all in the same direction:

- **Distance is measured from the house's whole NETWORK, not from the parent city.**
  `try_found_house_outpost` builds `nodes` from home + every office + every estate
  the house owns, then caps at `COLONY_MAX_KM = 2500` from *the nearest of them*.
  A house with scattered holdings can therefore plant a post 2,500 km from an
  estate that is itself 2,500 km from home. The comment acknowledges this is
  deliberate ("the same region the user wants outposts to cluster in IS the region
  a house's estates already work") — but the effect compounds, and 2,500 km from
  anything is already further than most real colonial hinterlands.
- **There is no MINIMUM distance and no routability requirement.** Nothing checks
  that the new post can actually be reached from the founder's network by a legal
  route in its own mode; siting is scored on trade value, coast, and air distance.
- **`create_market_colony` hardcodes `river: false`** and inherits `component` from
  the founder. So **no founded settlement is ever riverine**, and every one is
  instantly same-component with its founder — which, given §5, means it
  immediately qualifies for straight-line rescue lanes.

The user's requirement — "the trade route connecting the city should also be
connected to the local trade network" — is not expressible today, because the
colony's lane *is* the straight-line fallback of §2, not a route over the network.

---

## 7. Cities built on rivers

**Worldgen placement is genuinely good and should be left alone.**
`compute_habitability_fields` reads river cells, navigable trunks, river mouths,
delta/estuary kind, **head of navigation**, and scales the mouth rungs by the
hinterland the river actually drains (from real flow accumulation). The rung ladder
puts a navigable river port at the sea above an ordinary river mouth above a plain
river cell. That is the correct historical model, and it is the mechanism behind
London (tidal limit + first bridge point), Mainz (Rhine–Main confluence), Cologne,
Paris and St Louis.

**The campaign then throws most of the distinction away.**

- `river_hubs` at campaign start counts **navigable rivers only**, within
  `max_d2 = (grid_w * 0.01)^2` — **36 cells, ~400 km** at the default world. Nearly
  every hub on a continent with one navigable river qualifies. A flag that almost
  everything sets carries no information.
- It counts *proximity to a navigable river*, not the far more important thing
  worldgen already computed: whether the site is a **mouth, a confluence, or a head
  of navigation**. Those are the sites that made river cities rich, and the campaign
  cannot tell them from a town 400 km up a tributary.
- Colonies are never riverine at all (§6).
- And the flag buys nothing anyway (§2).

---

## 8. Performance

The **tick is in good shape** — one nested-`Vec` allocation in the whole
production half, `NEIGHBOR_K = 32` bounding dispatch, precomputed per-dispatch
lookups. The cost is all in the **routing/query layer**, which never received the
treatment §8.9 gave the phase-3 physics.

**P1 · Zero rayon in `commands/query_commands/` — all seven files.**
Measured: `grep -c "par_iter\|par_chunks\|into_par_iter"` returns 0 for every one.
Every route search, every overlay, every matrix build is single-threaded, while
§8.9 rule 2 requires the physics passes to be row-parallel. The Dijkstras are
independent per source and are the easy win.

**P2 · One Dijkstra per PAIR at four call sites.** Each `coarse_dijkstra` allocates
two full-grid vectors — `dist` (i64) + `prev` (usize) = **4.1 MB per call** at
259,200 coarse cells — and explores the grid from scratch.

- `routing.rs:103` (`compute_trade_routes`) — one per **edge**. At default settings
  (84 hubs × 3 neighbours ≈ 200 major edges, plus one per remaining settlement)
  a 400-settlement world runs **~500 whole-grid searches, ~2 GB of allocation
  churn**, single-threaded, on every route refresh.
- `economy.rs:158`, `political.rs:89`, `routing.rs:515` — same pattern.

**This exact fix is already done, documented, and precedented in this repo.**
`flow.rs:53–68` records it: *"ONE DIJKSTRA PER SOURCE, NOT PER PAIR … Grouping by
SOURCE and running the single-source variant that already exists
(`coarse_dijkstra_dist_prev`, added for the campaign route-days matrix and never
adopted here) collapses it to one search per distinct origin NODE … The routes are
identical."* Four call sites still need it. It also hands §4 its routed-nearest-hub
ordering for free.

**P3 · `build_coarse_cost` loads 97 MB of fine fields to read 259k centre cells.**
Six full-grid arrays (`f_terrain`, `f_elev`, `f_koppen`, `f_hazard`, `f_temp`,
`f_shelf`) at 6.48 M cells, by the function's own admission: *"We only need the
centre cell of each coarse block, but a full load keeps the index math simple."*
On "Large" (26 M cells) that is ~390 MB. Sampling per tile as it is loaded removes
the whole allocation — **and is the same edit as the §1 min-elevation fix**, since
both change how a block is reduced. Do them together.

**P4 · Campaign start pays for five cost grids and ~1,000 Dijkstras.**
`SEASON_SLICES = 4`, so N5 builds the annual matrix plus one per season: five
distinct `cached_coarse_cost` keys (five × P3) and five × *n* single-source
Dijkstras. At 200 hubs that is 1,000 whole-grid searches, sequential. P1 alone
makes this ~*cores*× faster with no behaviour change.

**P5 · `charter_owner: Vec<Vec<i32>> = vec![vec![-1; ng]; n]` — per day.**
`production.rs:1122`, the only nested-`Vec` allocation in the production path:
*n* separate heap allocations every simulated day. At 200 hubs × 365 days × 500
years that is **36.5 M allocations**. One flat `vec![-1; n*ng]` removes it, matching
the flat-array convention the codebase already states for `prov_good_belt`.

**P6 · Frontend: no viewport culling on per-cell fills.** `OverlayManager.render`
is called **once per visible world-copy**, and layers like lakes iterate every cell
of every lake (`for (const [x,y] of lake.cells) ctx.fillRect(x,y,1,1)`) with no
bounds check against the visible range. Provinces are correctly cached to an
offscreen canvas and blitted — that is the pattern the other per-cell layers want.

---

## 9. The plan, in dependency order

Ordered so each step's instrument exists before the step that needs it. Every item
names **a gate that is not its own target**, per §2.4.

### Stage A — free speed, zero behaviour change (do first, unblocks measurement)

**✅ BUILT.** All four items shipped together (`d7caa11`, `dcfa266`).

**A1 · One Dijkstra per source at the four remaining call sites.** ✅ Shipped as a
shared `coarse_dijkstra_batch`/`coarse_path_from_dist_prev` pair in `query_commands/
mod.rs`, adopted at all four sites (`compute_trade_routes`, the candidate-edge build
in `compute_trade_matrix`'s region graph, route centrality, and the lazily-matched
greedy loop in `compute_trade_matrix`'s flow assignment — kept lazy there, per-source
cached rather than pre-batched, since which pairs are queried depends on the greedy
match itself).
*Gate that is not the target:* ✅ done, as a direct equality test rather than a
manual dump-and-diff — `dijkstra_batch_tests::batched_dijkstra_matches_per_pair_
dijkstra` asserts the batched output equals `coarse_dijkstra`'s own per-pair result,
cell-for-cell, on a mixed land/sea/relief world.

**A2 · Rayon the independent Dijkstras** (per source in the matrix build, per
source in the route build). ✅ Shipped in both `coarse_dijkstra_batch` and
`compute_route_days_matrix_for_season` (`par_chunks_mut` over rows/sources).
*Gate:* satisfied by construction — each parallel task writes only the output
slots its own source owns (disjoint by construction), so the merge is order-
independent; no dedicated determinism test was added beyond that argument.

**A3 · Sample `build_coarse_cost` per tile instead of loading six full grids.**
✅ Shipped, folded together with B1 per this plan's own note.
*Gate:* the "bit-identical" form of this gate no longer applies once B1 changes
what a block's cost actually is — see B1's own gate below, which is what actually
covers this pair.

**A4 · Flatten `charter_owner`.** ✅ Shipped — one `Vec<i32>` sized `n·ng` in place
of `n` separate heap allocations.
*Gate:* `cargo test --lib tick::tests` — 212/216 at the time (the same 4 pre-
existing failures Stage A's own commit diagnosed as unrelated to it, confirmed by
reverting this branch's changes and reproducing them on baseline).

Stage A changes no simulated number. That is the point: it makes every measurement
below cheap enough to run per dose step, which is what §2.4 requires and what the
stalled dose walk has been paying for.

### Stage B — make the cost grid able to see terrain (correctness, small)

**✅ BUILT.** B1 shipped with A3 (`dcfa266`); B2 and B3 shipped together (`6a24322`).

**B1 · Sample per-block MIN land elevation for traversal cost** (mean/centre for
climate, hazard, temperature). ✅ Shipped — `min_land_elev` prices the base relief
term and the saddle discount; `elev` (centre-sampled) is untouched for every other
reader (`path_metrics`, `flow.rs`'s land-leg kind).
*Gate that is not the target:* ✅ done as planned — `a_narrow_pass_off_centre_is_
still_found` builds a ridge with one real gap placed off its block's own centre
sample, and asserts crossing costs distinctly less than the same ridge with no gap
at all (a same-column "did it cross here" check doesn't discriminate: a single
blocking column has nowhere else to cross regardless of whether the gap is seen).
Verified failing on the unfixed centre-only code first, per §8.24a3's rule.
*Caveat that materialised:* re-read, the Earth gate indeed cannot move (§8.23b — it
scores a baked DEM, calls no generator) — verified, not just assumed.

**B2 · Build components from ROUTED reachability, not Euclidean distance.** ✅
Shipped as `compute_routed_components` (at most *k* single-source searches, *k* =
number of components), using `path_allowed`'s reach-1 rule bounded by a NEW
`TRADE_COMPONENT_HORIZON_KM` (3,000 km, per §10 Q3's confirmed reading) rather than
the pre-existing `ISOLATION_RESCUE_MAX_KM` this item originally proposed reusing —
that constant governs a different, already-tested tick-time mechanism and was left
alone (see the commit's own scoping note). **The tiny-component rescue was
RETIRED, not capped**: with real reachability deciding components, a hub
reachable to a bigger market is already unioned with it, and one that isn't is
genuinely isolated — per §10 Q2's confirmed "let it starve" reading, forcing it
onto an unreachable market was exactly the dishonest union this item names.
*Gate:* ✅ done, as two NEW dedicated fixtures (`component_tests`) rather than an
extension of `rescue_tiny_components_never_crosses_an_ocean` (that test exercises a
different function — the tick-time twin — not the component build this item
touches): two landmasses split by open water wider than the horizon come out as
two components; the same landmasses with a narrower gap come out as one. Both
directions asserted on purpose.
*Gate that is not the target:* `econ_fidelity_scorecard` — run (green) as part of
the broader `econ_` suite; the reference/scorecard world's own components did not
visibly change at this world's scale, so no gradient movement was measured either
way. A larger, more geographically dispersed world would be a better instrument for
this specific claim — not yet built.

**B3 · Guarantee every settlement a route, by routed cost with fall-through.** ✅
Shipped — a ranked shortlist of the nearest 5 hubs (not just the nearest 1) per
lesser town, folded into the same A1 batch so extra candidates cost nothing beyond
what testing the first one already cost per source.
*Gate:* ✅ partially — `a_lesser_town_falls_through_to_its_second_hub_when_the_
first_is_unreachable` (via a new testable `compute_trade_routes_impl`) proves the
fall-through mechanism directly rather than end-to-end on a full generated world.
**Reporting the genuinely unroutable is partial**: a town whose whole shortlist
fails is logged (`log::warn!`), not surfaced through the IPC/UI layer — that is a
separate frontend/bridge change, not attempted this pass.

### Stage C — carriage that sorts cargo by mode (the historical core)

**C1 · Slow calm coastal sea toward a real effective average** (REWRITTEN by the
correction in §2 — the original called for raising road freight ~3× and adding a
mode term to `good_freight`, and both were based on a bad number). The road:sea
ratio is already 8:1 and correct; the ABSOLUTE level of sea speed is not. Raise the
coastal-sea cost rung above `0.5` (or lower the 55 km/day reference) until calm
coastal sea lands in the ~50–100 km/day band, and leave `freight_per_day` alone.
Land speeds already sit where history puts them, so this must not touch them.
*Gate that is not the target:* `econ_expenditure_shares_resemble_a_household` — food
is ~60% of spend and dearer sea freight pushes on it hard — plus
`the_dosed_economy_stays_healthy_on_a_realistically_dense_world`, since slowing sea
lengthens every lane's `days` and `SHIP_LEG_MAX_KM`'s staging relay reads distance,
not days. Dose in steps, re-running `econ_` per step. Expect the hard wealth bound
to bind: slower sea concentrates margin in the ports that can still reach a market.
**BUILT.** `COASTAL_SEA_COST` (`query_commands/mod.rs`) `0.5` → `1.6`, resolving to
~75.6 km/day (was 242) — mid-band, `freight_per_day` and every land/river cost
untouched. **A real, load-bearing finding surfaced building this**: NEITHER named
gate above can actually observe a `build_coarse_cost` change — every `tick::tests`/
`econ_` fixture is a synthetic in-memory `CampaignSim` (hand-built `days`, zero DB/
tile access by the module's own design, §5), so this dose is provably a no-op
against both. The correct instrument already existed for exactly this gap:
`real_world_price_distance_gradient` (`commands/real_world_diagnostics.rs`,
`#[ignore]`d, builds a real world end-to-end through the actual Tauri commands). A
same-seed paired run (424242) measured grain price/distance gradient **r = 0.092 →
0.185** — nearly doubled, both positive, the historically correct sign — see
`WORLD_AND_TRADE_MASTER_PLAN.md` §4's "UPDATE 2" for the full measurement and its own
caveats (one seed, one world size, one 20-year run). Mechanically gated by the new
`coastal_sea_speed_matches_the_historical_effective_average` (asserts the derived
km/day figure itself, not a placeholder ratio) and a rewritten `coastal_sea_river_
and_open_sea_price_in_a_sane_order` (the old `navigable_river_prices_near_the_
masschaele_ratio` asserted a fixed sea:river ratio that C1 necessarily retires — see
`COASTAL_SEA_COST`'s own doc comment for why that ratio was a freight-cost citation
conflated with travel speed, not a claim this cost grid can keep literally). Verified:
`cargo test --lib query_commands` 16/16; `tick::tests`/`econ_` deliberately NOT
re-run for this dose — the finding above is why they cannot move.

**C1b · Only then, the DIFFERENTIAL penalty** — make road cost scale harder with
`bulk` than sea does, so grain is priced off the road while silk is not. This is
the part of the original C1 that survives, and it is a genuine addition rather
than a recalibration. *Gate:* long-haul trade VOLUME must not collapse, and
low-bulk luxury lanes must survive unchanged.
**WIRED AT ZERO DOSE, NOT YET DOSED.** `good_freight` (`tick/production.rs`)
gained a `sea: bool` parameter (every one of its 6 call sites already computed
this `hubs[a].coastal && hubs[b].coastal` test nearby, for the display-only
river/sea labelling C1's own §2 records — reused, not duplicated); a LAND leg's
`bulk` term now scales by `1.0 + LAND_BULK_PENALTY * (bulk - 1.0).max(0.0)`,
sea untouched, and a good at or below `bulk = 1.0` untouched on either mode at
any dose — silk (`bulk` 0.35) can never be penalised by this term. `LAND_BULK_
PENALTY = 0.0` ships as the TRUE no-op — the same wired-ahead-of-dosing pattern
`CAPACITY_BIND_DOSE`/`ORE_CEILING_DOSE`/`HOUSEHOLD_MONETIZATION_DOSE` already
established, not a new one — proven by `c1b_land_bulk_penalty_is_a_noop_at_zero`. Verified:
`cargo check --lib --tests` clean; `tick::tests` 216/217 (the one failure is
the pre-existing, unrelated matrilineal regression); `econ_` 6/6, all bit-
identical to before this change (expected — the dose is 0.0, so every number
in every gate is untouched by construction). **Dosing it is separate, unstarted
work**: needs the same `econ_`-per-step walk `LAND_BULK_PENALTY`'s own doc
comment describes, re-verified against this section's own gate each step —
not silently assumed done just because the mechanism is wired.

**C2 · Separate travel TIME from freight COST.** Right now one `days` is both, so
the 1:4:8 cost ratio also claims a ship is 8× faster than a cart, which it is not
(~4×, and it waits for wind). Give the matrix a cost channel and a time channel
over the same path.
*Gate:* arrival latencies must stay inside plausible historical bands per mode
while the cost ratio moves independently.

**C3 · Split `cap_land` into boat and caravan pools; widen SHIP:CARAVAN toward
~10:1**; then begin the `CAPACITY_BIND_DOSE` walk from zero.
*Gate that is not the target:* the hard-asserted wealth bound in
`simulate_decades_reports_dynamics`. Capacity that binds is a scarcity rent, and
§5's N2 record shows an export-locked market's rent concentrating harder than
anticipated (a sustained richest house of 1,005,714). Dose in small steps and read
top-10% share every step.

**C4 · Only then, `N1_LOCAL_HAUL_BIND_DAYS` down from infinity.** This is the
keystone and belongs last, because until C1–C3 land there is no carriage economy
for the 96% to be forced into — binding first would just delete trade.
*Gate:* long-haul trade **volume must not collapse** — the companion gate
`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` already names as the one that matters.

### Stage D — siting

**D1 · Measure `COLONY_MAX_KM` from the parent city, not the whole network**; add a
minimum gap; require the site to be **routable to the founder's network in its own
mode** before founding.
*Gate:* `econ_diagnose_outpost_founding` — outposts must still get founded. The
recorded failure mode here is a tightened gate silently stalling founding
altogether, which is exactly what `OUTPOST_MAX_PER_CALL` was introduced to fix.

**D2 · Give a founded colony a real `river` flag and a real component**, from the
site's own geography rather than `false` and the founder's id.

**D3 · Carry worldgen's river CLASS into the campaign** — mouth / confluence /
head-of-navigation, which `compute_habitability_fields` already computes — instead
of a 400-km "near a navigable river" boolean, and tighten that radius to something
that means "on a river".
*Gate:* the share of hubs flagged riverine must **fall** substantially; a flag
almost everything sets is the thing being fixed.

---

## 10. Questions that need a decision before any of this starts

1. **Is `econ_inheritance_rules_fragment_differently` still the right brake?** It
   blocks most of §0's eleven doses. It is single-seed, 60-year, and documented as
   perturbed five times. Options: keep it and dose slower; make it multi-seed so it
   stops flipping inside its own noise; or narrow its claim to what a division
   provably does (`a_division_moves_capital_and_creates_none` already asserts that
   at the point where it is decidable). **I would make it multi-seed before touching
   a single dose**, but this is a call about the project's own instrument and yours
   to make.
   **BUILT.** Three seeds (`INHERITANCE_GATE_SEEDS = [42, 1337, 7]`, a prefix of the
   6-seed robustness diagnostic's own list chosen by position, not by outcome), each
   run under all four laws; the wiring checks and both statistical claims (houses
   ever founded, mean wealth per house) are asserted per seed, with the houses-ever
   MARGIN floor (1.05×) checked on the seed-average rather than per seed, since one
   seed alone can measure as low as a 1.02× ratio despite the contrast being real
   in aggregate. A fresh 6-seed re-measurement, done to calibrate this change, found
   the model itself had moved since the stale numbers this answer was written
   against — both contrasts now hold 6/6 (were 5/6), most likely from Stage A/B's
   routing fixes though that was not isolated by bisection. See `CLAUDE.md` §8.15
   for the gate's full description and the corrected robustness table.

2. **Should a truly isolated continent be allowed to starve?** B2 will create real
   isolated markets for the first time. `TECTONICS_AND_ISOLATION_PLAN.md` Part A
   already committed to the honest answer ("if there are not enough goods to sustain
   the civilisation, the city becomes dead"), but that has never actually been
   *exercised*, because there have never been isolated components. Expect abandoned
   cities on first run. Is that the intended outcome, or does an isolated continent
   get a grace mechanism?

3. **What is the trade reach *supposed* to mean?** Three different numbers claim to
   bound it: `max_link` 12,022 km (components), `TRADE_MAX_DIST_FRAC` 9,618 km
   (horizon), `ISOLATION_RESCUE_MAX_KM` 1,800 km (rescue) — and `SHIP_LEG_MAX_KM`
   3,500 km bounds a single leg. They cannot all be right. A pre-modern regional
   economy is nearer 1,500–3,000 km; the Indian Ocean monsoon system reached
   ~6,000 km in staged legs. **My reading: keep one horizon at ~3,000 km and let
   staging compose longer routes** — which is what `staging_hop` already exists to
   do. Confirm before B2, because B2 hard-codes the answer into what a component is.

4. **Do you want mountain passes to be NAMED?** B1 makes the route find a real gap.
   Naming it (and letting a town at its foot grow on transit) is a small further
   step given `maybe_found_route_post` and `names::gen_name` already exist, and it
   is most of the visible payoff. Separate step, or same one?

5. **Is a re-baselined scoreboard acceptable?** C1 moves every price and freight
   number in the model by construction. §2.6 rightly forbids editing old rows — so
   someone has to be willing to append a row that is **worse** on some metric while
   being more truthful. `TRADE_AND_MARKET_REVIEW.md` asks this same question and it
   has not been answered.

6. **How much of the world's trade should be house-carried?** The 96% ownerless
   residual is either a placeholder to be dosed away (C4) or an honest model of
   local carriage that nobody records. Historically both are defensible — most
   pre-modern exchange was local and unrecorded, and the famous merchant houses
   really did carry a small share of tonnage and a large share of *value*.
   **If it is the latter, then the residual should be capped by DISTANCE, not
   removed** — which is what N1 does — and the target is not 0% but something like
   local-only. What is the intended split?

7. **Which is the priority: a correct map or a correct market?** Stage B fixes what
   the user *sees* (unconnected towns, ocean-crossing straight lines). Stage C fixes
   what the user *reports as the main problem* (economies). B is much smaller and
   its gates are cheap; C is where the realism is. Stage A serves both. I would run
   **A → B → C**, but if the economy is the real pain, C1 alone is arguably the
   single highest-value change in this document.

---

## 11. Negative results and non-findings, recorded so they are not re-derived

- **The tick is not the performance problem.** One nested-`Vec` allocation in the
  whole production path, bounded dispatch, precomputed lookups. Do not go looking
  for wins there; they are in the query layer.
- **`days_per_cell` is NOT a speed — it is a reference speed pinned to cost 2.2.**
  This document's own worst error (§2's correction block) came from reading it as
  one. `cost_to_days = days_per_cell · f / (OPEN_SEA_COST · 100)`, so a medium's real
  speed is `55 · 2.2 / cost` km/day — 242 for calm coastal sea, 30.2 for flat land.
  Any freight or travel-time figure derived from the nominal 55 km/day is wrong by
  that medium's cost ratio. Recorded here because the mistake survived writing the
  finding, the plan and the commit message, and was only caught by re-deriving the
  arithmetic when asked to explain it out loud.
- **Overland freight is NOT badly mispriced, and road:sea is NOT flat.** Both were
  claimed in this document's first cut and both are false: 54.5% of grain's value
  over 300 km of flat land, 112% over hilly, against a real cart that roughly
  doubled it — so land is about 2× cheap at worst and right on ordinary ground —
  and the road:sea ratio measures 8:1. Do not "fix" `freight_per_day`.
- **A long sea haul beating a shorter overland one is CORRECT, not a bug.** 4,000 km
  by sea at 91% of grain's value against 1,000 km overland at 182% is the Baltic-
  grain-to-Amsterdam fact. Trans-oceanic trade is wrong here for one reason only —
  §5's component build makes two continents one market — and fixing freight would
  not touch it.
- **The 1:4:8 ratio is not missing and does not need re-deriving — STALE AS OF C1.**
  This was true when written: the ratio was correct, cited, and in the cost grid
  already, and the problem was its reach (§2), not its value. C1 necessarily retired
  it as a literal static cost-grid ratio (see `COASTAL_SEA_COST`'s own doc comment):
  the ratio is a citation of Masschaele's FREIGHT-cost figure, and the shared
  `cost_to_days` conversion this cost grid uses conflates freight cost with travel
  SPEED — so fixing coastal sea's absolute speed (a real bug) necessarily changes
  its cost relative to river/land (not a bug, a consequence). C1b's job is to
  recover a Masschaele-shaped differential at the FREIGHT level instead, scaled by
  `bulk`, not baked back into this grid.
- **`rivers_json = ""` is fixed.** CLAUDE.md's §5.1 note and
  `TRADE_STAGING_AND_POSTS_PLAN.md`'s finding that the campaign matrix is built with
  no river geometry are **both now stale** — `metadata["rivers"]` is wired and read.
- **`TradeHist.prices` exists**, so `TRADE_AND_MARKET_REVIEW.md` F9 ("there is no
  price history — anywhere") is also stale. Both stale entries should be corrected
  in place; a review that overstates what is missing costs a future session real time.
- **Worldgen settlement placement on rivers is not the bug.** It is one of the
  better-modelled things in the tree. The loss happens at the campaign boundary.
