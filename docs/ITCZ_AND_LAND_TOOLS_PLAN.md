# ITCZ physics + stage-1 land drawing + four new terrain generators

> **STATUS: APPROVED, NOTHING BUILT.** This is a plan, not a record of work.
> Read `docs/FIX_PLAN.md` for what is prioritised across the whole project and
> `docs/SCOREBOARD.md` for what is actually measured. The measured baseline in
> §Context below was taken on `7786da8` and is real; everything after it is
> intent.

## Context

Two separate problems, raised together.

**1 · The climate is too dry where monsoons should be.** I measured the current
baseline (`cargo test --lib earth_`, this branch, `7786da8`):

```
main-class 70.2%   exact-zone 39.0%   C row 32.2%   C→B confusion 36%
India-Mumbai 19N73E   gen=B ref=A  precip=  161mm  (real ~2200)  summer 76%
Bangladesh   24N90E   gen=B ref=A  precip=   84mm  (real ~2500)  summer 25%
China-South  25N113E  gen=B ref=C  precip=  286mm  (real ~1700)  summer 30%
SE-US        34N84W   gen=C ref=C  precip=  375mm  (real  1300)  summer 18%
monsoon sites reversing (Δ>120°): 4/7
```

Bangladesh's **summer fraction is 25%** — it rains more in winter there than in
summer. That is a monsoon running backwards, not merely a weak one.

To the direct question: **the ITCZ does run before climate** (it is built inside
phase 3, Köppen is phase 4) **and it does affect climate**, through precipitation.
Ordering is fine. The defect is that there is **not one ITCZ — there are two**,
computed by different formulas that never see each other:

| | file | formula | drives |
|---|---|---|---|
| Wind ITCZ | `seasonal.rs::itcz_latitude` | `8° · sun_sign · (1 + land_pull)`, land pull sampled over the **summer hemisphere only, 5–35°** | displaces the wind belts (`belt_wind_shifted`) |
| Rain ITCZ | `precipitation.rs::compute_itcz_shift_zonal` + `ITCZ_SEASONAL_MIGRATE` | `(NH−SH land frac)·20` clamped ±12°, **both hemispheres, 0–30°**, plus ±10° seasonal | the additive `itcz_bonus_shifted` rain band |

Different amplitudes (8° vs 10°), different land measures, different sign
conventions. So the convergence zone the *wind* converges into is not the
convergence zone where the *rain* falls, and the overlay draws both.

The root cause of the dryness itself is already diagnosed in `FIX_PLAN.md` A4:
**there is no pressure field.** Winds are prescribed from latitude plus a thermal
perturbation at `MONSOON_WIND_GAIN = 0.10`, which cannot turn a belt — so there
is no monsoon low as an *object* to anchor a reversal. A1 records that raising
`MONSOON_WIND_GAIN` (0.10 → 0.22 → 0.40) and `ET_RECYCLE_MAX` (to 0.85) were both
tried and reverted: they move a spot check and regress the aggregate. Per §2.4 I
will not repeat that.

**2 · Stage 1 has almost no drawing tools.** The Landmass step is three buttons
and a circle brush: no area marking, no coastline shaping, no islands, and
pressing "Generate from Plates" twice gives the *identical* world (one fixed seed
field). The Elevation step has four generators but "Generate" repeats a stored
seed, and mountain chains stop rather than dying into their surroundings.

## Decisions already taken

- Build the **A4 pressure field**. Adopt it if **exact-zone rises**, lowering
  `EARTH_MAIN_FLOOR` deliberately and recording the trade in `SCOREBOARD.md` —
  exactly the precedent A14 set (70.6 → 70.0 to buy the seasonal monsoon).
- Area marking is a **freehand lasso**.
- "More terrain styles" means **more generators**, not more render styles.
- Variants are **generated for real, then undone**.
- "Randomise" reaches **landmass shape only**.
- StepElevation: **group models by family, collapse the rest.**

## Order

Land tools → generators → climate. Three commits on
`claude/itcz-climate-generation-3mknjx`. The visible, low-risk work lands first;
if anything is lost to time it is the speculative physics, not the tools.

---

## Commit 1 — Lasso area tools, randomise, variants

### Backend · `src-tauri/src/sim/step1_plates/landmass_ops.rs` (new)

A `Lasso` type plus four operations. Each loads `ColumnSet::PHASE_PLATES`,
mutates `terrain`/`elevation`/`is_volcanic`, and calls `buf.save(&conn, label)` —
which already pushes exactly one `undo_journal` entry, so **every op is undoable
and re-rollable for free**, no new history code.

- **`smooth_roughen(lasso, amount, seed)`** — one bipolar control. Negative
  smooths (majority filter over a disc, repeated); positive roughens.
- **`fjords(lasso, count, length_km, width, seed)`** — walked from a sea cell
  *inland* up the land gradient, sinuous, optionally branching, tapering to the
  head.
- **`island_chain(lasso, count, kind, size, seed)`** — `Arc` | `Scatter` |
  `Single`. Arc islands are marked `is_volcanic`, which is real data:
  `deposits.rs`'s `VolcanicArc` model scores off that exact column (§8.16), so a
  planted arc can carry a genuine ore province later.
- **`fill(lasso, land)`** — bulk set.

Four rules this module must hold, each with a test:

1. **The lasso is UNWRAPPED, not clamped** (rule 6). A polygon drawn across the
   antimeridian arrives with points on both edges; a naive point-in-polygon test
   selects the *complement* of what the user circled. `Lasso::new` re-expresses
   every vertex in one continuous frame anchored on the first, and the hit test
   tries `x`, `x−w`, `x+w`.
2. **Every op FEATHERS to the lasso edge.** A hard clip prints the user's
   selection gesture onto the map as a straight coastline — the same class of
   tell §8.24 spent a session removing from the province mosaic.
3. **Roughening is a LEVEL SET, never a per-cell dice roll**: signed
   distance-to-coast, perturbed by fbm, re-thresholded at zero, bounded by a
   reach. This is the same mechanism and the same reasoning as `TERRAIN_2_PLAN`'s
   D1/T1 coastline decoupling — the attempt that worked did so because it was
   structural, not because it was tuned. A per-cell threshold scatters speckle
   islands across deep ocean by construction.
4. **Ops iterate the selection, never the world.** A 26 M-cell world must not pay
   a full-grid sweep to reshape one bay (§8.9 rule 1's spirit).

### Backend · commands

`sim_commands.rs`: `land_op_smooth_roughen`, `land_op_fjords`,
`land_op_islands`, `land_op_fill` — each takes the polygon as JSON (the shape
`sim_generate_ridges` already uses for `linesJson`), calls `ensure_unfrozen`, and
returns modified tile coords. Registered in `lib.rs` (rule 8).

`preview_commands.rs`: `render_world_thumbnail(layer, max_px)` — read-only,
samples the `WorldBuffer` into base64 RGBA exactly as `CoarsePreview` does. Used
for the variant minimaps. Deliberately *not* read back through the tile/LOD cache,
whose invalidation timing after a generate would make a thumbnail silently stale.

### Frontend

- `uiStore`: `activeTool` gains `"lasso"`; `lassoPolygon` + setters, mirroring the
  existing `ridgeLines`/`setRidgeSketch` plumbing exactly.
- `MapCanvas.tsx` / `OverlayManager.ts`: capture and draw the lasso, following the
  `ridgeDraftRef` → `setRidgeSketch` pattern already there.
- `StepLandmass.tsx`: an **Area tools** group (the four ops, each with its own
  params and a **Re-roll** button that re-runs with a new seed after an `undo`), a
  **Randomise landmass** button (new seed → `simGeneratePlates`), and a
  **2 variants** compare.
- Variant flow, using only existing commands plus the thumbnail:
  generate A → thumbnail → `undo` → generate B → thumbnail → show both →
  keep B, or `undo` and re-generate A from its seed (deterministic, so identical).

**Verify:** `cd src-tauri && cargo test --lib landmass_ops -- --nocapture` ·
`cargo check` · `npx tsc --noEmit`.

---

## Commit 2 — Four new generators + tidy StepElevation

### Backend · `src-tauri/src/sim/step2_terrain/`

Four new models, all registered in **`apply_elevation_model`**
(`sim_commands.rs:391`) — the single selector, so both run-alls honour them
automatically and none is reachable only from its own button:

- **`rift`** — parallel fault blocks with **flat-floored grabens** and asymmetric
  half-graben tilt (one steep scarp, one gentle back-slope). Strike follows
  `boundary_type` divergent segments where plate data exists, a regional noise
  strike otherwise.
- **`glaciated`** — the shape model, then glacial modification gated by an ice
  mask: U-valley broadening, cirque hollows below crests, over-deepened troughs
  that breach the coast. This is the *honest* way to get fjords, as opposed to
  notching a coastline. Phase 2 has no climate, so the ice mask is a
  latitude+altitude **proxy**, documented as one — the same convention
  `geology.rs` already uses for its phase-2 climate proxy.
- **`plateau`** — quantised levels with sharp escarpment rims and outlying buttes.
  `landform.rs` already has a `plateau` archetype and a `terrace` parameter that
  §8.24 calls "present but subtle"; this generator makes it the subject.
- **`volcanic`** — shield cones on `is_volcanic` cells, summit calderas on the
  largest, and hotspot trails of decreasing height. Composes with commit 1's
  volcanic arcs, which write the same column.

**Chains that die into their surroundings.** `walk_spine`'s along-strike taper is
lengthened and given a noise-modulated falloff so a range ends in foothills
rather than stopping; the same treatment is applied to the drawn-ridge tool
(`generate_ridges`), which is where a hard end is most visible.

**Randomisation.** Both steps roll a fresh seed on every Generate press, with an
explicit **lock seed** control for when the user wants to iterate a slider against
a fixed world. Today's behaviour (repeat the stored seed, randomise only via a
separate button) is the wrong default for brainstorming.

### Gate — and a named risk

`elevation_model_tests::every_elevation_model_builds_a_different_world` currently
asserts all pairs of **four** models disagree on >25% of land. It extends to
**eight**, i.e. 28 pairs. **`glaciated` vs `shape` is the pair most likely to
fail**, because glaciated *starts* from shape. If it does, the answer is to make
the glacial modification actually substantial — not to weaken the assertion. If
it cannot be made substantial without wrecking the landform, I will say so and
record it rather than lowering the bar.

### Frontend · `StepElevation.tsx`

Eight models grouped by family (Tectonic · Shape · Chains · Landform types), with
the sliders, ridge tool, shelf and elevation-adjust blocks each collapsible — the
pattern `StepWorldCharacteristics` already uses. Island/volcanic-chain creation
stays in stage 1 only, per your instruction.

**Verify:** `cargo test --lib elevation_model_tests -- --nocapture` ·
`cargo test --lib step2_terrain` · `cargo check` · `npx tsc --noEmit`.
Look at it, don't argue about it: `EROSION_SHEET_DIR=… cargo test --release --lib
dump_erosion_sheet -- --ignored --nocapture` per rule 30, which requires a render
rather than a statistic.

---

## Commit 3 — One ITCZ, then the pressure field

### 3a · Unify the two ITCZs (small, lands first)

One shared convergence-line function, consumed by `seasonal.rs`,
`precipitation.rs` **and** the `compute_climate_bands` overlay, so the drawn line
is the modelled line for rain and wind alike. Expected to be close to
score-neutral; the point is that A4 needs a single ITCZ to be *about*.

### 3b · `sim/step3_ocean_atmo/pressure.rs` (new)

A single-layer surface pressure anomaly per season:

- **Base**: the zonal structure `Circulation` already implies — subtropical
  highs, the equatorial trough, the polar-front low, the polar high.
- **Thermal**: `season_temp_anomaly` (which exists) diffused into a smooth
  pressure response — warm continent → thermal low. This is what finally makes
  the monsoon low an *object* rather than a boolean hunting for "a big landmass
  with ocean to its east."
- **Wind**: geostrophic plus friction — `v = (1/f)·k̂×∇p` rotated cross-isobarically
  toward low pressure, blending to pure down-gradient flow as `f → 0` near the
  equator. `seasonal.rs` already approximates exactly this with its `theta` term,
  so this generalises the existing mechanism rather than replacing it.

### 3c · The ITCZ becomes derived, not prescribed

With a wind field from pressure, the ITCZ is the **convergence line** of that
field, and precipitation reads convergence directly instead of adding
`itcz_bonus_shifted` at a hand-placed latitude. That is the actual unification
3a only approximates.

### Constraints this must not break

- **Rule 10 — Earth parameters stay a no-op.** The pressure field must be applied
  as an anomaly that is exactly zero at Earth settings, the way the EBM already is.
- **Rule 11 — the phase-3 order is duplicated in THREE files**: `sim_commands.rs`,
  `earth_validation.rs`, `step3_ocean_atmo/preview.rs`. All three move together or
  the fidelity gate and the settings preview stop testing the real pipeline.
- **§8.9 — no per-cell outward scans; keep the row loops rayon-parallel.** An
  iterative pressure relaxation is a new full-grid pass and is exactly the shape
  that has cost this project 30 s before.

### Adoption rule

Adopt if **exact-zone rises**, per your decision — lowering `EARTH_MAIN_FLOOR`
deliberately if main-class falls, and defending the change with the physics gates
(`earth_monsoon_wind_reverses`, the monsoon spot checks) rather than the aggregate.
I will report every configuration measured, including the ones I do not ship: A14's
own table is the model. **If it comes out a negative result, that is the
deliverable** and it goes into `FIX_PLAN.md` as one — per §2.4, a documented
reverted attempt is worth more than the code around it.

**Verify:** `cargo test --lib earth_ -- --nocapture` (main/exact, the confusion
matrix, the monsoon spot checks, the 4/7 reversal gate) ·
`cargo test --lib step3_ocean_atmo` ·
`cargo test --release --lib bench_ocean_atmosphere -- --ignored --nocapture`
for the §8.9 budget.

---

## Testing scope

Per your instruction, each commit runs only what it touches. **Nothing in this
plan goes near `sim/campaign/tick/`**, so the economy oracle (`econ_`) and the
campaign dynamics run are not applicable and will not be run — §2.5 requires them
after `tick/` changes, which these are not. `cargo check` and `npx tsc --noEmit`
run on every commit.

## Docs (§2.6, §2.7 — same commit as the code, not after)

- `CLAUDE.md`: new §8.25 for the stage-1 land ops; §8.13/§4 updated for eight
  elevation models; §8.2 updated once the ITCZ is unified.
- `docs/SCOREBOARD.md`: an appended row per commit that moves a measured number —
  never an edit to an old row.
- `docs/FIX_PLAN.md`: A4 updated with whatever is measured, adopted or reverted.
