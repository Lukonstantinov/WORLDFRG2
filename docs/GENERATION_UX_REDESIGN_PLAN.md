# Generation UX Redesign — plates you draw, cities you place, worlds you pick, maps you print

> **STATUS: ANALYSIS + PLAN. NOTHING BUILT YET** except one `#[ignore]`d
> diagnostic (`diag_enclave_rescue_rate`, `sim/step1_plates/plates.rs`) added to
> measure the "small circular plates" report rather than assert it.
>
> Read §1 (measured findings) before §3 (the slices). Four of the six asks turn
> out to be *reaching a mechanism that already exists* rather than building one,
> and two of them are genuine new work. Knowing which is which is most of the
> value here.

---

## 1. Measured findings — what is actually wrong

Each of these was read out of the tree, not assumed. Several are the SAME
failure the codebase has already recorded and fixed elsewhere, which is the
strongest argument for fixing them: the project has a name for this disease.

### F1 · Both run-alls discard six of the user's own settings (the §4 bug, still present)

`CLAUDE.md` §4 records that "Both run-alls used to HARDCODE a generator and
silently discard the user's pick and all four sliders". That was fixed **for the
elevation model only**. The same line of `sim_run_all` still hardcodes everything
else:

| Setting | Step UI that sets it | What the run-all uses |
|---|---|---|
| settlement density / realism | `StepSettlements` slider | hardcoded `0.55` |
| settlement count cap | `StepSettlements` slider (20–1000) | hardcoded `None` |
| province granularity | `StepSettlements` slider | hardcoded `0.5` |
| shelf width/slope/noise | `StepElevation` (`sim_generate_shelves`) | hardcoded `(12.0, 0.4, 0.3, 8.0)` |
| lake fill depth / max fraction | `StepRivers` (`riverParams`) | hardcoded `0.004` / `total/2000` |
| ore-district count, goods spread | `StepBiological` (`bioParams.gemDeposits`) | hardcoded `(6, 0.5)` |

`culture_count` is the ONE thing that survives, because it is read back out of
`metadata` rather than passed as an argument. **That is the pattern to copy** —
it is why it works.

Consequence: the primary button in the app ("Generate Full World") produces a
world that ignores most of the panel the user just filled in. Merging the two
run-all buttons (the user's ask) makes this strictly worse unless it is fixed in
the same change, because the merged button becomes the *only* path most users
take.

### F2 · Ocean fraction is built, wired, tested — and unreachable

`generate_plates_and_landmass_with_target(buf, seed, plate_count, ocean_fraction)`
exists and is gated (`land_fraction_tracks_the_target`). **No caller ever passes
anything but `DEFAULT_OCEAN_FRACTION` (0.70).** `simGeneratePlates(seed, plateCount)`
has no parameter for it and neither run-all does either.

So "how much of my world is ocean" — the single most basic world-generation knob
in any comparable tool — cannot be set at all, while the mechanism to honour it
sits finished one argument away. This is the eleven-mechanisms-at-zero-dose
pattern `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` names, in the worldgen half.

It is also **exactly what the requested presets need**: an ocean world, an
archipelago world and a Pangaea world differ first in this number.

### F3 · "More continents" is hardcoded as an objective, so Pangaea is unreachable

`plates.rs` already counts continental connected components
(`count_continental_components`) and, among ocean-fill trials within
`CONTINENT_AREA_TOLERANCE_FRAC` of the best area match, **always takes the one
with the MOST separate landmasses**:

```rust
.max_by_key(|t| (t.1, -(t.0)))   // t.1 = continent count
```

That is a fine default and a bad constant. A Pangaea preset is the same code with
the comparator reversed, and "about four continents" is the same code scoring
`-(t.1 as i64 - target).abs()`. The continent-count axis the user wants is
**one objective function away**, not a new algorithm.

### F4 · MEASURED: at the default plate count, 43% of your plates are rescue discs

The report ("sometimes plates create small circular ones") turns out to understate
it. `diag_enclave_rescue_rate` + `diag_plate_size_distribution` (both added by this
analysis, both `#[ignore]`d) measure it directly.

**Plate sizes on one real world, 360×180, seed 0, 16 plates requested:**

```
[21, 23, 24, 24, 25, 27, 29,  1472, 3208, 3617, 3659, 4836, 5301, 6399, 9044, 27091]
 └──────── seven discs ────┘  └─────────────── nine real plates ───────────────────┘
```

That is not a continuum with a small tail — it is **bimodal with a 50× cliff**.
Nothing in the power diagram produces a 25-cell cell; the only mechanism in the
file that can is the forced-enclave stamp, whose disc of radius `0.02·min(w,h)`
= 3.6 cells has an area of exactly ~41 cells. The observed 21–29 is precisely that
disc's core-plus-penumbra.

**Across 12 seeds × 2 world sizes:**

| plates requested | disc-sized plates per world | share |
|---|---|---|
| 8 | 3.08 | **38%** |
| 16 (the app's default) | 6.92 | **43%** |
| 24 | 10.9 | **45%** |
| 32 | 15.0 | **47%** |

Identical at 360×180 and 900×450, because the starvation is a property of the
site/weight geometry and not of the raster — so this is not a small-world
artefact and a production 3600×1800 world has it too.

**The cause is arithmetic, not luck.** With `SIZE_CLASS_WEIGHTS`
`[2.2, 1.6, 1.0, 0.55]` and `POWER_DIAGRAM_OFFSET_SCALE = 2.2`, the power
diagram's offset is `(w − 1)·spacing²·2.2`, so a giant beats a small-class site
**at that site's own centre** whenever `d_giant² − d_small² > 3.63·spacing²` —
i.e. out to ~1.9 plate spacings. The jittered grid puts every site within
~1–1.4 spacings of its neighbours, so **any small-class site next to a giant is
annihilated by construction**. With 10% giants and 25% smalls, that happens
constantly.

**And the existing gate cannot see it.**
`every_requested_plate_gets_real_territory` asserts only that each plate "holds
any cells at all" — which a 25-cell rescue disc satisfies. The gate is passing
on exactly the artefact being reported.

(Circularity measures 0.48 for the small plates against 0.43 for normal ones —
weakly confirmatory, and I am not resting the claim on it: a 4-connected raster
perimeter systematically under-scores small shapes, so the *size cliff* is the
proof, not the shape metric.)

The fix is therefore **not** to soften the stamp again (that was the previous
attempt, and it left the cause untouched). It is to make the seeding
class-aware so a small plate is given room rather than rescued — see Slice 1.

### F5 · The map export renders every shaded layer WITHOUT its neighbour ring

`export_layers` (`commands/file_commands.rs`) calls:

```rust
let rgba = render_tile(tile, layer);   // == render_tile_full(.., &TileNeighbors::default(), &RenderCtx::default())
```

while the interactive path (`tile_commands.rs`) builds a real neighbour ring and
a real `RenderCtx { grid_w, grid_h, tx, ty, .. }`. `CLAUDE.md` §8.12 states the
invariant plainly: *"Every shading layer now needs its neighbour ring in
`tile_commands.rs`, or slopes break at tile edges."*

So **every exported hillshade, terrain, climate, biome, soil, fertility and
natural PNG carries a 128-pixel grid of shading seams** — and differs from what
the user is looking at on screen. The export is the artefact people actually
keep; it is the one path that must match the map exactly.

### F6 · The export layer list is a hand-copied table, and it has already drifted

`EXPORTABLE_LAYERS` in `App.tsx` lists **15** layers. `layerGroups` in
`Toolbar.tsx` — the canonical list the app renders from — carries **26**.
Missing from export: `natural` (the reference-atlas view, arguably the single
most exportable layer in the app), `sst`, `salinity`, `snow`, `windspeed`,
`ridges`, `shark`, `shipworm`, `reef`, `storm`, `disease`.

This is §8.18's own rule broken in a new place: *"Never reintroduce a
hand-copied colour table in the frontend"* — here it is a hand-copied *layer*
table, and it drifted for exactly the reason §8.18 predicts. Also unreachable
from export: the seven elevation STYLES (§8.22), class isolation, and the twelve
MAP PLATES (§8.17) — the compositions that make a page look like an atlas.

### F7 · Export cannot draw a single river, city, border or name

Rivers, lakes, settlements, labels, trade routes, provinces, states and every
other overlay are drawn client-side in Canvas 2D by `OverlayManager` (§7, rule 4).
`export_layers` is a pure Rust tile stitcher and cannot see any of them. **An
exported "map" today is a bare raster with no hydrography, no cities and no
type** — which is not a map, it is a data layer.

The good news, and it decides the architecture: `OverlayManager.render(ctx)`
draws in **world-cell coordinates** (`ctx.fillRect(x, y, 1, 1)`) with the caller
supplying the transform. Pointing it at an offscreen canvas sized to the grid
gives full-resolution overlays for free. The export therefore belongs on the
**frontend**, compositing Rust's base-layer PNG under the existing overlay
renderer — not in `file_commands.rs`.

### F8 · Manual settlement placement is ~90% plumbed already

Nothing needs inventing for "place a city by hand and have it be alive":

- **Storage** — settlements live in `worldStore.settlements` and are persisted by
  `persist_overlays` into the `settlements` campaign key. Adding one to the array
  is adding one to the world.
- **Trade routes** — `MapCanvas`'s route effect already lists `settlements` in its
  dependency array, so a hand-placed city gets routed **the moment it is added**,
  with no new code at all.
- **Aliveness** — the campaign seeds from the `EconomySnapshot` that
  `compute_economy(hubs, …)` builds *from the frontend's settlement array*. Re-run
  Political + Economy and the manual city is a real hub with prices, houses and
  trade.
- **Authenticity** — every field a generated settlement carries is derivable at a
  clicked cell from data already on the tile: `habitability` is a persisted
  column, `name` from `names::gen_name_epithet`, `culture`/`region` from
  `names::culture_label`/`region_name`, `site` from `site_label`, and `population`
  from the same `compute_food_capacity` → `pop_agri × access × port_premium ×
  civ_factor × cold_factor × winter_factor` chain `generate_settlements` uses.

What is missing is one backend command (`place_settlement_at(x, y)` returning a
fully-scored `Settlement`), one tool, and **one rule**: re-running "Find
Settlements" must not silently delete hand-placed cities.

### F9 · Nothing about first-run tells you what to do

The wizard is 13 steps; the panel is 220 px wide; the primary button
("Generate Full World") sits *above* the step list and bypasses all of it. A
second button ("Complete from Landmass") appears conditionally, and the two are
distinguished only by colour and a sentence. Presets exist in three unconnected
places — `TERRAIN_PRESETS` (elevation sliders), `ARCHETYPES` (planet knobs),
grid-size presets (New World dialog) — and none of them produces a *world*.

---

## 2. What the user asked for, mapped to the findings

| Ask | Reality | Work |
|---|---|---|
| Plates first, drawable, simulatable | Plates are step 1 but bundled into "Landmass"; no drawing; motion is persisted and drawable already (B2) | **New** (needs a decision — §4 Q1) |
| Fix small circular plates | F4 — cause known, rescue stamp | **Fix** |
| Manual settlement placement, connected + alive | F8 — plumbing exists | **Small** |
| One run-all button | F1 — merging without fixing F1 makes it worse | **Fix + merge** |
| Presets for full worlds (continents / Pangaea / dryness / tilt) | F2 + F3 — both axes already built and unreachable | **Reach + compose** |
| PNG + PDF export with selectable layers | F5, F6, F7 — export is seamed, drifted, and overlay-blind | **Rewrite, frontend-side** |
| User friendly | F9 | **Throughout** |

---

## 3. The plan — nine slices, in dependency order

Each slice names its own gate. Slices 1–3 are backend and independently
verifiable; 4–9 are frontend and verified by `npx tsc --noEmit` plus looking at
the app (this repo has no frontend rendering test beyond `tsc` — stated plainly
rather than dressed up with an assertion that would not exercise the canvas).

### Slice 1 · Plates stop producing discs (F4)

Fix the CAUSE, since F4's arithmetic shows the rescue is not an edge case but a
structural consequence of the seeding:

- **Class-aware seeding.** Draw each plate's size class *before* placing its
  site, then place sites in descending class order with a per-class exclusion
  radius, so a small-class site is never dropped inside a giant's dominance
  region (F4 shows that region reaches ~1.9 plate spacings). Equivalently: make
  the offset derive from an intended per-class RADIUS and space sites to fit it,
  rather than letting an offset tuned for area-spread decide adjacency.
- **Region-grow whatever rescue remains.** A residual guarantee must still exist
  — "a mineral must never silently vanish" (§8.16), the same discipline applied
  to plates — but it claims cells by BFS from the site into whichever neighbour
  has most to spare, so its outline is made of the same warped margins as every
  other plate. **Delete the disc stamp**; do not soften it a second time.

Target, stated as a number because F4 gives us one: **zero disc-sized plates at
8/16/24/32 on the same twelve seeds that today produce 3.1/6.9/10.9/15.0**, with
the size gate still clearing.

Gates: `every_requested_plate_gets_real_territory` (exists — and must be
STRENGTHENED, since F4 shows a 25-cell disc satisfies it today: it should require
a real fraction of the world's mean plate area, not merely non-zero),
`plate_territory_stays_connected` (exists), `plate_sizes_span_an_order_of_
magnitude` (exists), plus the two diagnostics above promoted from `#[ignore]`d
measurement to an asserted floor once the fix lands. Each new assertion is to be
**verified failing on the unfixed code** before it is trusted (§8.23b's rule,
already paid for three times in this repo).

### Slice 2 · The three landmass axes become real parameters (F2, F3)

- `sim_generate_plates` gains `ocean_fraction: Option<f32>` and
  `continent_goal: Option<i32>` (`-1` = as many as possible, today's behaviour;
  `0` = as few as possible / Pangaea; `n` = nearest to n). Both `Option`, both
  defaulting to today's values, so **every existing call is bit-identical**.
- The ocean-fill trial comparator becomes a function of `continent_goal` instead
  of a hardcoded `max`.
- Both are persisted to `metadata` (`ocean_fraction`, `continent_goal`) the way
  `culture_count` already is — that is what lets the run-all honour them without
  growing two more positional arguments.

Gates: `land_fraction_tracks_the_target` (exists) extended over several targets;
NEW `a_pangaea_target_fuses_the_continents_an_archipelago_target_does_not` — the
two extremes must measurably differ in continent count on the same seed, or the
knob is decorative.

### Slice 3 · The run-all stops discarding settings (F1)

Follow `culture_count`'s pattern exactly: every step panel writes its settings to
`metadata` as it always does; **`sim_run_all` reads them back** instead of
hardcoding. No new positional arguments (the signature is already seven wide).

Gate: NEW `the_run_all_honours_every_persisted_setting` — set each of the six
settings to a non-default, run the run-all, assert the output differs from the
default run in the way that setting predicts (settlement count, province count,
shelf cell count, lake count, ore district count). A gate that passes with the
settings still hardcoded is worthless, so **verify it failing first**.

### Slice 4 · One Generate button (the merge)

`Generate Full World` and `Complete from Landmass` become **one primary button**
whose behaviour is decided by `uiStore.landmassSource`, stated in a line of text
under the button rather than by making the user pick:

> *Generating from **plates** — your landmass will be regenerated.*
> *Generating from **your painted landmass** — your coastlines are kept.*

with a small "regenerate the landmass too" checkbox in the second case (the one
genuine choice), rather than two buttons that differ by colour.

### Slice 5 · World presets (the headline user-facing win)

A `WorldPreset` composes what is now scattered across three files — plate count,
ocean fraction, continent goal, elevation model + its four sliders, and the
planetary knobs `ARCHETYPES` already spans. Shipping set, each a real
world-generation idea rather than a colour scheme:

Earthlike · Many Continents · Pangaea · Archipelago · Ocean World · Desert World ·
Ice Age · Hothouse Jungle · Highlands · Tidal-Locked Extreme (high tilt)

Two rules carried over from `planetArchetypes.ts`, because it already learned
them: **a preset is a SPAN, not a point** (an intensity dial between a mild and a
strong endpoint, so the user has somewhere to go), and **a preset only sets the
axes it is about** (picking "Archipelago" must not silently throw away the tilt
set two minutes ago). One more, new: a preset **never touches a finalized world**
and is a generation input, never a view — rule 14's line, on the other side.

Pick a preset → press Generate → a whole world. That is the flow the app does
not currently have.

### Slice 6 · Plates as their own first step, drawable — **DECIDED: seeds + classify**

Split today's "Landmass" into **1 · Plates** and **2 · Landmass**.

**Drawing = plate SEEDS** (your answer). Click the map to drop a plate's centre,
pick its size class (giant / large / medium / small) and its oceanic/continental
type, drag to nudge, click to delete. The partition regenerates from your seeds
through the same power diagram, so every existing plate gate still applies and
Slice 1's class-aware spacing protects a hand-placed small plate exactly as it
protects a generated one. Seeds you do not place are filled in by the generator,
so you can author three plates and let it supply the rest.

**"Simulate" = classify and build** (your answer). One button: derive an Euler
pole for every seed (at a real fraction of world width off its own centroid, the
same rule generation uses), classify every margin into convergent / divergent /
transform, roll volcanism, and rasterize the coastline. Motion arrows and
boundary tinting then appear through machinery that **already exists** —
`get_plate_motion`, `OverlayManager.drawPlateMotion`, and the `plates` render
layer's own boundary tinting — so this slice adds no render work.

Time-stepped tectonic history (drift, collision, accretion over geological time)
stays a separate future project, per your "classify now, history later" framing.
It is named in §5 so nobody later assumes it shipped here.

### Slice 7 · Settlement placement and editing — **DECIDED: full editor, world-derived by default**

- **Backend**: `place_settlement_at(x, y)` → a fully-derived `Settlement` — real
  habitability (a persisted tile column), a culture-appropriate name from
  `names::gen_name_epithet`, `culture`/`region` from the live culture map, `site`
  from `site_label`, and a population from the SAME `compute_food_capacity` →
  `pop_agri × access × port_premium × civ_factor × cold_factor × winter_factor`
  chain `generate_settlements` uses. It must **reuse** that chain, not re-derive
  it, or a hand-placed city is a different kind of object from a generated one
  and every downstream consumer can tell.
- **The default is: you click, the world decides.** Placement alone gives a city
  indistinguishable from a generated one. Everything below is opt-in editing on
  top of that default, never a form you must fill in.
- **Full editor** (your answer): rename, re-tier (village/town/city/capital),
  set population, drag to move, delete — and it applies to **generated cities
  too**, not only hand-placed ones. Each edited field is marked as overridden so
  a regeneration knows to keep it.
- **The honest caveat, surfaced in the UI rather than buried here**: a
  hand-typed population is no longer a consequence of the land, and the economy
  downstream reads it as if it were. The panel shows the world's own figure
  beside yours ("the land supports ~18,400") and a one-click "reset to what the
  world says", so the override is visible and reversible rather than silent.
- **The rule that matters**: re-running "Find Settlements" **keeps** every
  manual and every edited city and generates around them (they take part in the
  minimum-spacing check like any other site). Silently deleting a user's
  hand-placed capital is the single worst thing this feature could do.
- **Alive**: after any edit the step offers "Re-solve trade & economy" — the
  existing `runPoliticalAndEconomy`. Trade routes redraw on their own (F8), with
  no new code.

### Slice 8 · Map export, rewritten frontend-side (F5, F6, F7) — **DECIDED: multi-page atlas, vector text**

The export becomes a **composition**, not a layer dump:

- A base layer (any of the 26, plus elevation STYLE and class isolation), fetched
  through the SAME tile path the screen uses — which fixes F5 by construction
  rather than by duplicating the neighbour-ring logic into `file_commands.rs`.
- Any set of overlays, drawn by the existing `OverlayManager.render(ctx)` onto an
  offscreen canvas at grid resolution. This is why the export moves to the
  frontend: it is the only place rivers, cities, borders and names exist.
- The layer list **derived from `layerGroups`** (F6) — never a second copy.
- Presets from the twelve MAP PLATES (§8.17), so "export the Climate plate"
  is one click and the page is a real atlas plate, graticule and type and all.

**PNG**: full grid resolution, or a chosen multiple.

**PDF — a multi-page atlas with real vector text** (your answer):

- A page per selected plate/layer, plus a **legend page** and a **gazetteer**
  (named settlements, rivers, lakes, provinces, with their coordinates).
- Place names, legend entries, titles, scale bars and the gazetteer are **real
  embedded PDF text** — selectable and searchable in a reader, crisp at any print
  size — with the map raster underneath. `OverlayManager` already computes every
  label's position and style through one registry (§8.11 `drawLabel`/
  `measureLabel`), so the label PASS can be re-run in "emit, don't paint" mode to
  produce a text list for the PDF instead of pixels. That existing single
  registry is what makes vector type affordable here; without it this would mean
  re-deriving 23 call sites.
- Legend colours come from the SERVED palettes (§8.18), so the key on the page is
  provably the same table the pixels came from.
- This lands a **new dependency** and a font-embedding step. Recommendation:
  do it in **Rust** (a PDF writer crate) rather than npm, so the heavy raster
  pages never cross the IPC boundary as base64 and the export can stream to disk
  — the frontend sends composited layer bitmaps plus a text/label manifest, and
  Rust assembles the document. Page size (A4/A3/Letter/custom) and DPI are user
  settings with A3 @ 300 dpi as the default.

### Slice 9 · First-run legibility (F9)

The wizard gets a two-mode head: **Quick** (preset → Generate → done, the 90%
path) and **Detailed** (today's 13 steps, unchanged). Plus the small honesty
fixes: the primary button says what it will do to the landmass; a step that
cannot run says which step it waits on (the Toolbar already does this for
layers — `layerReady` is the precedent); and settings that the run-all now
honours (Slice 3) are visibly the same settings, not a parallel set.

---

## 4. Decisions taken, and the two still open

Four forks were put to the maintainer and answered:

| Question | Decision |
|---|---|
| What does "draw the plates" mean? | **Draw plate SEEDS** — centre + size class + type; the partition regenerates from them. Keeps the power diagram and every existing gate. |
| What does "simulate" do? | **Classify & build the coastline** from the drawn seeds. Time-stepped tectonic history is explicitly a later, separate project. |
| What is the PDF? | **Multi-page atlas with real vector text** — a page per plate, a legend page, a gazetteer, selectable/searchable type. |
| How much control over cities? | **Full editor** (rename / re-tier / set population / move / delete, on generated cities too) — **but placing a city with everything world-derived stays the default path**, never a form to fill in. |

Both remaining questions have since been answered too:

**Q-A · May a preset overwrite settings you have already set?** Neither "strict"
nor "total" — a preset applied to a partly-generated world offers **two explicit
choices**, and never a silent third:

- **Fill in what is missing** — apply the preset only to the axes whose steps
  have not run yet, and **warn** which of the preset's own settings are being
  skipped and why ("Elevation is already generated; this preset's Mountainous
  relief will not be applied. Regenerate Elevation to use it."). Nothing already
  generated is touched.
- **Overwrite fully** — apply every axis and invalidate the downstream steps, so
  the world regenerates from the earliest axis the preset touches.

The dialog must name the actual consequence in both cases (which steps are
skipped, or which will be regenerated), because the whole failure this replaces
is a preset quietly doing one of the two and the user finding out later.

**Q-B · Should export composite overlays by default?** **Yes.** Rivers, cities,
borders and names are layers to the user, so they are layers in the export, on by
default. This is the decision that moves the export from Rust to the frontend —
`OverlayManager.render(ctx)` is the only place those exist.

## 5. Deliberately NOT in this plan

Named so they are not silently assumed done:

- **A time-stepped tectonic history** — plates drifting, colliding and accreting
  over geological time. Decided as a separate later project; Slice 6 ships
  classification and coastline-building from drawn seeds, and nothing more.
- **Per-plate hand-drawn margins or painted plate territory** — the two plate
  drawing models not chosen. Seeds only.
- **Touching the campaign half.** Nothing here changes `sim/campaign/tick/`, so
  §2.5's `econ_` gates cannot move and are not run.
- **The Earth fidelity gate.** `earth_validation.rs` scores a baked DEM and never
  calls a generator (§8.23b), so Slices 1–3 cannot move it by construction —
  which will be verified, not argued, since Slice 1 touches `step1_plates/`.
- **Re-tuning elevation, climate or goods.** This plan reaches existing knobs and
  fixes existing bugs; it does not re-dose anything.
- **Editing a finalized world.** Every new verb here is a generation verb and
  respects `ensure_unfrozen`.
