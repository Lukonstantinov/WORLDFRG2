# Tectonic character & trade isolation

Two independent subjects that arrived from the same session of feedback, kept in
one file because they share nothing technically and everything in intent: making
the world **believable** rather than merely generated.

**Status: AGREED, PART A/B1/B2/B4 BUILT, B3/A3 NOT BUILT.**

| Part | Subject | Status |
|---|---|---|
| **A** | Trade isolation — an ocean is a real barrier | **built** — `ISOLATION_RESCUE_MAX_KM` caps `rescue_tiny_components`; §A3 (unify Flows highlight onto the coarse grid) not yet built |
| **B1** | Plate sizes | **built** — power-diagram weighted partition, see CLAUDE.md §8.24a2 |
| **B2** | A motion layer you can read | **built** — persisted `PlateMotion` + `get_plate_motion` + arrow render on the `plates` layer, see CLAUDE.md §8.24a2 |
| **B3** | Collision style (multi-ridge belts) | not built |
| **B4** | Relict sutures | **built** — see CLAUDE.md §8.24a2 |

---

# Part A · An ocean is a barrier

## A0. The measured cause

The campaign route matrix already has the right discipline. `#6 NO DEAD CITY`
guarantees partners **only within a hub's own geographic COMPONENT**, and its own
comment says why: *"Crossing the component by straight line was drawing dishonest
trans-oceanic arrows between two separate continents."* Short sea hops are allowed
separately and are bounded — `#6c COASTAL CABOTAGE`, capped at
`CABOTAGE_SEA_FRAC = 0.08` of world width.

So why do trans-oceanic lanes still appear? **`rescue_tiny_components` defeats the
component guard before it runs.** It reassigns every component with fewer than
three hubs to the nearest "big" component — and the search has **no maximum
distance at all**:

```rust
let mut bj = None; let mut bd = f32::INFINITY;
for &j in &big { let d = d2(i, j, &self.hubs); if d < bd { bd = d; bj = Some(j); } }
```

A two-hub island in mid-ocean is therefore relabelled as part of a continent
thousands of km away. Once they share a component id, `#6` cheerfully draws
straight-line lanes between them — the exact thing it exists to refuse. The guard
is not wrong; it is being handed a lie about which cells are connected.

## A1. The decision

> *"A city not reachable for trade should trade with the other cities on the island
> or available ones. If there are not enough goods to sustain the civilisation, the
> city becomes dead."*

So: **cap the rescue by distance.** A tiny component within a plausible sea
crossing is folded into its neighbour as today; beyond that, it stays its own
component and trades internally. A settlement that cannot obtain what it needs
starves and is abandoned.

**That outcome is already modelled and does not need inventing.** `abandon_hub`
sets `abandoned` / `died_tick` / `died_cause`, and starvation already drives it.
A city dying of isolation will therefore appear in the chronicle with a cause,
not vanish silently — which is the difference between a modelled consequence and
a bug.

## A2. The one real risk, and the guard

A **single-hub** component has no internal partner at all: one city cannot trade
with itself. Under a hard cap it would freeze from the first tick — not "a
struggling frontier port" but a town that never had a chance.

Two things keep that honest rather than arbitrary:

* The cap is stated in **km, converted per world** (rule 25), never in cells. A
  cell is ~11 km at 3600×1800 and ~130 km on a test grid; a cell-count cap means
  "the next island" on one world and "the next continent" on another.
* A one-hub component gets **one lifeline** to its nearest coastal neighbour
  regardless of distance, and is **flagged** as such. It is on the edge of the
  known world, not amputated from it. Whether it then survives is the economy's
  answer, not the router's.

This is the discussion the user left open. The alternative — let a lone island
city die outright — is defensible and more austere, but it makes the ROUTER decide
the outcome, and the router should decide reachability while the ECONOMY decides
survival. Keeping those two separate is what makes the death meaningful.

## A3. Also in scope: one routing system, not two

The Flows highlight and the Dynamic Trade Flow layer currently route through
**different graphs**:

* Dynamic Trade Flow → the **coarse cost grid**, with `path_allowed(reach,
  max_crossing)` — it already refuses illegal crossings.
* Flows highlight → the **worldgen trade-route graph**, which is bounded by that
  layer's own reach and cannot express a campaign sea lane. When it finds no
  path, the lane currently falls back to a dashed direct line.

That fallback was added to stop flows vanishing (rule 35) and is the lesser of two
evils, not the right answer. The right answer, and what the user asked for — *"make
the trade lines dynamic trade on the same layer"* — is to route the highlight
through the **same coarse grid with the same crossing rule**. Then every drawn lane
is a legal route, the dashed fallback becomes rare rather than routine, and the two
layers stop disagreeing about where trade can go.

## A4. Gates

* `econ_measure_carrier_mix` and `econ_fidelity_scorecard` **before and after** —
  this reduces long-haul trade by construction, and the size of that reduction is
  the deliverable, not a side effect.
* A new diagnostic: **how many hubs end up isolated, and how many later die of
  it.** If a routine world strands dozens of cities the cap is too tight; if it
  strands none, it is not doing anything.
* `simulate_decades_reports_dynamics` — the economy must stay bounded.
* An assertion that no lane in `days` crosses more open water than the cap allows,
  which is the claim itself and is not currently testable anywhere.

---

# Part B · Tectonic character

**Decision: appearance-level, but DERIVED FROM TECTONIC MOTION — not decorative
noise.** The Euler-pole velocity field (shipped, `plate_velocity_at`) is the
input; everything below reads from it rather than inventing a parallel story.

## B1. Plates of genuinely different size

Today plate seeds are placed on a **jittered grid**, so every plate comes out
roughly the same size. Earth's are not remotely uniform: the Pacific plate is
~103 million km², the Nazca ~15 M, the Juan de Fuca ~0.25 M — nearly three orders
of magnitude.

Draw plate areas from a **power law** instead: a few very large plates, several
medium, many small. Implemented as weighted seeding (large plates get a wider
capture radius in the warped-Voronoi assignment) rather than by moving seeds
about, so `warped_voronoi` and its two gates are untouched.

**Gate:** the largest plate must be at least ~5× the median, across seeds — a
claim the current jittered grid fails by construction.

## B2. A motion layer you can read

`Plate` is **transient** — recomputed from seed each phase-1 run, never persisted,
never reachable from the frontend (§8.24b). So the velocity field that already
drives boundary classification cannot be drawn.

Persist the poles and rates, and render on the existing `plates` layer:

* a velocity **arrow per plate**, length ∝ speed, direction from `v = ω × r`;
* each boundary tinted by what it IS — convergent / divergent / transform — which
  `boundary_type` already knows;
* the **convergence rate** along a boundary, which is what B3 keys off, so the
  layer explains the mountains rather than merely labelling the margin.

This is the first time plate data leaves the generator. Persisting it is also the
prerequisite the master plan names for a per-plate UI override (Part I Slice 5).

## B3. Collision STYLE — why the Himalaya and the Andes look different

Today orogeny is a single distance field from `boundary_type`, so every mountain
belt has the same cross-section: one ridge, tapering. Real belts differ by what is
colliding, and `geology.rs` already computes the setting (`SETTING_COLLISION`,
`SETTING_ISLAND_ARC`, `SETTING_ACTIVE_MARGIN`, `SETTING_SUBDUCTING_SIDE`) — it is
simply not used to shape the belt's PROFILE.

| Setting | Real analogue | Profile to draw |
|---|---|---|
| continent–continent | Himalaya / Alps | **broad, multi-ridge**: a high main range, parallel foothill ranges, an elevated plateau behind (Tibet) |
| ocean–continent | Andes | narrower, **volcanic**, a trench offshore, one dominant crest |
| ocean–ocean | Japan / the Marianas | **island arc** + back-arc basin |
| transform | San Andreas | little uplift; offset and shear, not a range |

The multi-ridge profile is the visible one the user asked for, and it should be
**driven by convergence rate** from B2 — a fast collision builds a wider, higher
belt than a slow one, which is what makes the layer and the mountains agree.

## B4. Relict ranges — the Urals, the Appalachians, the Highlands

**This is the item that makes a world believable, and the one thing in this plan
that does not exist in any form today.**

The measured state: `age` in `geology.rs` is `fbm_noise(...)` — **pure noise**,
uncorrelated with whether a boundary is active. Its only consumer is
`age_amp = 1.25 - oage * 0.5`, a ±25 % amplitude wobble. And decisively, age is
only ever assigned **on a belt that exists today**. There is no mechanism anywhere
for a worn-down range far from any current margin.

That is precisely what the Urals are: a Permian suture sitting *inside* the
Eurasian plate, with no active boundary anywhere near it. Same for the
Appalachians, the Scottish Highlands, the Scandinavian Caledonides — all of them
former collisions the map still remembers.

### The user's question: simulate time, or not?

> *"Maybe create small simulation so drawn landmass can be affected by those
> changes? Or just leave it as it is?"*

**Recommendation: neither. Generate a PAST, not a simulation.**

A time-stepped tectonic model — move the plates over N steps, accumulate uplift,
erode — is Part I Slice 6 of the master plan, deliberately deferred once already.
It needs a coarse deformation solver, it is slow, and it has stability problems
that show up as artefacts rather than as history. And its *output*, for our
purposes, would be almost exactly the thing we can state directly: a handful of
former sutures with ages.

So: **bake 2–4 RELICT SUTURES into the world at generation** — former plate
boundaries, each with an age, placed inside today's plates rather than on their
margins. Each raises a range whose height and sharpness fall with age:

| Age | Height | Character | Analogue |
|---|---|---|---|
| active | full | sharp crests, high | Himalaya, Andes |
| old | ~55 % | rounded, broad, deeply dissected | Urals, Appalachians |
| ancient | ~25 % | rolling uplands, isolated massifs | Scottish Highlands, Scandinavian Caledonides |

Cheap, deterministic, fast, and it produces the reading the user wants. It is
**faking the history rather than the physics**, and that is the honest description
— the doc says so rather than implying a simulation ran.

It also fixes the noise problem at its root: a belt's age stops being random and
becomes a property of **which suture it belongs to**, so a whole range shares one
age instead of dithering between young and old along its length.

### B4b. Ranges away from the margin

Also asked for, and real: the Tien Shan is ~1,500 km from the India–Asia suture
and was raised by that collision. Model it as **intraplate deformation propagating
inland from a strong collision** — a secondary, lower belt at a distance
proportional to convergence rate, along the collision's own strike. Causally
linked, per the user's answer to Q3, rather than scattered noise: that link is what
makes a map read as tectonic instead of merely bumpy.

## B5. Gates

* `plates::tests` — the two existing gates (margin straightness, territory
  connectedness) must keep passing; plus a new one for the size distribution (B1).
* A new gate for B3: a continent–continent belt must be measurably **wider and
  multi-crested** than an ocean–continent belt on the same world. A single-ridge
  model fails it by construction, which is the property a real gate needs.
* A new gate for B4: a relict range must exist **far from every active boundary**,
  and must be measurably lower than an active one.
* `the_default_hypsometry_resembles_earth` — B3/B4 add uplift and must not push
  the world back up the hypsometric curve (§8.24's own regression, which cost a
  factor of forty once already).
* **NOT `earth_`.** `earth_validation.rs` scores a baked DEM and never calls a
  generator (§8.23b), so nothing here can move it. Verify, don't run it blind.
* **Look at it.** `dump_erosion_sheet` / `dump_elevation_style_sheet` render a real
  world through the real pipeline. Every finding in §8.23–§8.24 came from a render
  and none from reading code; a multi-ridge belt and a worn relict range are
  judgements about a picture.

## B6. Deliberately NOT in scope

* **Time-stepped tectonics** (Part I Slice 6) — see B4's reasoning.
* **Erosion of drawn/painted landmass over time.** The user asked; the answer is
  no. Phase 2 does not carve (§8.23 records three failed attempts), and a
  time-varying landmass would invalidate every downstream phase — climate, rivers,
  biomes, settlements are all computed once against one surface.
* **Plate motion during a campaign.** The world is frozen at finalize (§3.4). Plates
  move over tens of millions of years; a campaign is centuries.
