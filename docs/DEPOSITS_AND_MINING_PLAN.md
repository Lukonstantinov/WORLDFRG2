# Deposits, Mining & the Quarry Layer — Plan

**Status: APPROVED IN DESIGN, PARTLY BUILT.** Slice 1 exists on disk and compiles;
slices 2–5 are unbuilt. Scope is medieval / early-colonial.

This plan covers the natural-resource half of the world: how minerals are placed
(geology), how they are worked (mining as an industry), and how a settlement whose
existence *is* its deposit gets represented (the Potosí case).

---

## 0. The measured findings this plan answers

Four facts about the current code, each verified by reading it, not inferred.

### 0.1 The goods placer ignores the tectonics the world already computes

`biological.rs` placed a deposit good from exactly two inputs:

```rust
if buf.terrain[i] == 1 && buf.elevation[i] >= eff_min {
    let p = fbm_noise(x * province_scale, y * province_scale, salt, 4, 2.0, 0.5);
    cand[i] = (p * 0.85 + 0.15 * (buf.elevation[i] - eff_min)).clamp(0.0, 1.0);
}
```

An elevation floor and a per-good noise field. Meanwhile the `WorldBuffer` already
carries `boundary_type` (convergent / divergent / transform), `plate_index` and
`is_volcanic` from phase 1, and rivers from phase 5 — **none of which the placer
read**. Ore geology is almost entirely a function of tectonic setting; that is the
organising principle of the discipline, not flavour.

### 0.2 The split gems never used the ore-province noise at all — REAL BUG

`biological.rs:797`:

```rust
let suitability = dp.map(|d| d.suitability).unwrap_or(spec.scoring.is_some());
```

`builtin_index_of("ruby")` returns `None` (the split gems are *custom* goods, not in
`GOOD_NAMES`), so `dp` is `None`, so `suitability` falls back to
`spec.scoring.is_some()` — which is **true** for every one of them. They therefore
took the suitability branch, and their envelopes are bare elevation ramps:

| Good | Elevation band | Other terms |
|---|---|---|
| lead | 0.26–0.85 | — |
| marble | 0.28–0.90 | — |
| silver | 0.30–1.00 | — |
| topaz | 0.30–0.80 | — |
| amethyst | 0.34–0.85 | — |
| emerald | 0.38–0.90 | temp bell |
| jade | 0.40–1.00 | — |
| ruby | 0.42–1.00 | temp bell |
| sapphire | 0.46–1.00 | — |
| diamond | 0.55–1.00 | — |

Ten near-identical overlapping ramps with nothing to separate them, so all ten
stacked on the same tallest ranges — **precisely the bug the ore-province noise was
written to fix**. The comment claiming "ruby ranges ≠ sapphire ranges (real gem
geology)" described an intention, not the code. The noise only ever applied to the
six *builtin* deposit goods.

### 0.3 Cells are not the problem; district structure is

Measured: `KM_EQUATOR = 40075`, Standard grid 3600×1800 → **11.1 km/cell**; Large
7200×3600 → **5.6 km/cell**. A cell is already *finer* than an ore district, so
clustering is not a cell-size problem. The causes were two constants:

- `min_sep = w * 0.025` ≈ **1000 km** between two deposits of the same mineral.
- A deposit was **one cell**, with a 25% chance of r=1–2.

> Note: CLAUDE.md §8.12 states "a cell is 30–110 km across". That is stale by
> 3–10×. Fix under §2.7 when slice 1 lands.

### 0.4 A mine is a substring match

`tick/mod.rs:1011`:

```rust
if n.contains("iron") || n.contains("gem") || n.contains("salt") … { return 2; }  // "Mine"
```

`estate_kind == 2` is decided by the good's *name*, and mechanically a mine is
identical to a farm — same `estate_effectiveness`, same upkeep, same production
formula. The only place kind 2 means anything today is `dominant_estate_kind` in the
depletion pass. There is no mining industry, no depth, no ore grade.

---

## 1. Design decisions taken (do not relitigate)

| # | Decision | Rationale |
|---|---|---|
| D1 | Deposits are a **discrete list in metadata**, alongside the u8 belt column | The belt stays the source of truth for production and overlays (rule 7, save compat); the list carries grade/extent/depth, which a u8 cannot |
| D2 | A mineral picks a **deposit MODEL**, it does not author rules | A free-form DSL repeats the planet-knob problem and permits geologically impossible minerals. Defaults are correct per mineral; override one without touching the rest |
| D3 | **Deposits persist.** Only a *weak* body under sustained pressure from a large city or high-tier mine declines, over 100–250 years, to a floor — never to zero | User's call. Also the accurate default: Potosí still exists; Rammelsberg ran ~1000 years |
| D4 | Placer parent resolution is **strict with a fallback** — try the parent lode, else a standalone river placement, and report which | Strict is geologically right; the fallback prevents the silent-vanish failure this codebase has hit repeatedly |
| D5 | Gems are **relocated to their real host geology**, not preserved | Existing saved worlds keep their tiles; only newly generated worlds change |
| D6 | The quarry window lists **districts**, expanding to workings on click | ~400–1200 workings worldwide is too long a flat list |
| D7 | Every working carries a **quality index**, and availability is gated on being at/near the **surface** | User's answer 4. Depth is the pre-modern mining constraint |
| D8 | Txt import **adds** to the library, never replaces | User's answer 1 — they prune afterwards |

### Consequence of D3, stated plainly

D3 kills the "mining settlement dies with the ore" outcome. A mining town will
*decline* but survive. This is the historically more accurate default and it is the
user's explicit call; abandonment becomes an exceptional path, not the norm.

---

## 2. The geology — deposit models

Each model is a real, named class of ore deposit, scored from columns that already
exist. Every one is exploitable with pre-1700 technology.

| Model | Scored from | Yields | Type localities |
|---|---|---|---|
| **VolcanicArc** | `is_volcanic` + convergent proximity (~25 cells ≈ 275 km, the real arc-trench gap) | silver (epithermal), copper (VMS), mercury, sulphur, alum, sapphire | Potosí, Guanajuato, Cyprus, Rio Tinto, Almadén, Tolfa |
| **CollisionalOrogen** | high + rugged + convergent + **not** volcanic | lode gold, tin, tungsten, cobalt, emerald, topaz, garnet, slate, granite | Cornwall, the Erzgebirge (Joachimsthal → *thaler* → dollar), Muzo |
| **Craton** | far from **any** boundary + low relief + interior | banded iron, **diamond** | Clifford's Rule: economic kimberlite occurs *only* over thick Archean lithosphere. Never in young mountains |
| **Rift** | divergent proximity + flood basalt | stratiform copper, agate/carnelian, amethyst | Kupferschiefer/Mansfeld, the Deccan |
| **CarbonatePlatform** | low + flat + old shallow sea | MVT lead-zinc, calamine, coal measures, limestone, millstone | Mendips, Silesia, La Ferté-sous-Jouarre |
| **ContactMetamorphic** | platform carbonate **next to** an orogen | marble, ruby, lapis lazuli, jade | Carrara, Pentelikon, Mogok, Sar-i-Sang |
| **EvaporiteBasin** | Köppen B + low + flat + restricted | rock salt, alabaster/gypsum, natron, saltpetre | Wieliczka, Hallstatt, Cheshire |
| **Placer** | **derived** — walked downstream from a parent lode | alluvial gold, stream tin, ruby, sapphire, diamond, jade | Pactolus (→ the first coinage), Klondike, Ratnapura, Golconda |
| **Bog** | wet + low + flat + near water | bog iron | Medieval Scandinavia and Slavic Europe — **structurally impossible under an elevation floor** |
| **CoastalMarine** | shelf / beach / warm shallows | pearls, murex, coral, amber, bay salt | Baltic amber, Gulf of Mannar |
| **Weathering** | **derived** — supergene alteration of a parent in an arid climate | turquoise | Nishapur, Serabit el-Khadim |

### Two structural notes

**Placer is derived, not scored.** It is placed by walking downstream from a lode
working along the river network. Rivers are phase 5, goods are phase 8, so the data
is available. This is historically the correct order: Cornwall worked stream tin
before lode tin; every diamond traded before 1725 came from Golconda's gravels.
`placer_frac` per mineral is the share of its districts placed this way.

**Diamond inverts the old model completely.** It currently ships `min_elev: 0.55` —
the highest mountains. Geologically that is exactly wrong. Diamonds come from the
flattest, oldest, most boring land on the map.

### Template worlds

`boundary_type` is **empty** on template/painted worlds (`elevation.rs:1955`). Every
tectonic model degrades to a relief-and-continentality proxy rather than scoring
zero. The failure mode to guard is a good that silently places nothing — that has
happened here before (the metals vanishing under an absolute elevation floor, which
is why `highland_cap` exists).

---

## 3. The three-level hierarchy

Real ore geology is described at three scales; the old placer collapsed all three
into one cell.

| Level | Real scale | Here | Example |
|---|---|---|---|
| Metallogenic belt | 100–1000 km | model setting field × per-mineral belt noise | Iberian Pyrite Belt (~250 × 30 km, ~90 bodies) |
| Ore district / camp | 10–60 km | cluster centre, `MIN_DISTRICT_SEP_KM` = 320 apart | Freiberg, the Mendips, Laurion |
| Working / deposit | 1–10 km | **one cell** | Cerro Rico, Rammelsberg, one quarry face |

`DISTRICT_RADIUS_KM` = 45; 2–8 workings per district, scaled by how strong the
district is (a real camp's size tracks how much there was to find).

**The UI's "gem deposits" slider now means ORE DISTRICTS.** Relabel it in slice 3.

### Per-working state

| Field | Meaning | Drives |
|---|---|---|
| `grade` 0..1 | ore richness / stone fineness | the good's **quality tier** — this is what makes "tiers of gems" possible |
| `extent` | weak / moderate / great / world-class | how much can be drawn down (D3: only *weak* meaningfully) |
| `depth` | surface / shallow / deep / **flooded** | gated by the city's mining capability |

Depth is *the* pre-modern mining constraint. The progression — outcrop, open cut,
adit, below the water table — is a real four-tier ladder: Rio Tinto's Roman reverse
waterwheels and much of Agricola's *De Re Metallica* exist for drainage alone.

`depth_workability`: surface 1.00 · shallow 0.80 · deep 0.35 · flooded 0.15. The
belt column is written as `grade × workability`, so a deep rich body is **visible
but largely locked** — inventory for a future mining industry, not present output.

---

## 4. Slices

### Slice 1 — Geological placement ⭐ PARTLY BUILT, COMPILES

Pure worldgen (phase 8, frozen at finalize). Touches **neither fidelity oracle**:
`earth_validation` is climate-only; `economy_validation` builds a synthetic world
with no real goods map. Lowest-risk item in the plan, and everything else reads
from it.

**On disk now:**
- `sim/step8_biological_goods/deposits.rs` — `DepositModel` (11 models), `Deposit`,
  `GeoContext` (shared BFS distance fields, built once per world), the geological
  default table `default_model_for`, `place_mineral` with the district/working
  hierarchy, placer downstream-walk, weathering derivation, 7 tests.
- `plates.rs` — boundary constants made `pub`.
- `goods_spec.rs` — `DepositSpec` gains `model` / `placer_frac` / `parent`, all
  serde-defaulted so old spec JSON loads unchanged.
- `world_buffer.rs` — `PLATES` added to `PHASE_BIOLOGICAL` (read-only).
- `biological.rs` — the old branch removed; `compute_trade_goods` returns the
  working list; second pass for derived minerals.
- `sim_commands.rs` — deposits persisted to `metadata["deposits"]` on all four
  pipeline paths.

**Still to do in slice 1:**
- Run the tests (`cargo test --lib deposits::`) — written, not yet executed.
- Remove the now-unused `highland_cap` binding (a warning, not an error).
- Verify the district count change does not inflate total production
  (`economy.rs:305` normalises by `good_max`, so the effect should be bounded — but
  **measure, do not assume**).
- Update CLAUDE.md §4/§6/§8 and fix the stale cell-size claim in §8.12.

**Gate:** `cargo test --lib deposits:: -- --nocapture` (diamond lands on craton not
peaks; different models separate minerals; workings cluster; determinism; depth
attenuates; no shipped mineral places nothing; template world still places).
Plus `cargo check` and `npx tsc --noEmit`.

### Slice 2 — Grade → quality rewire

`economy.rs:305` currently derives a hub's gem quality from its **share of world
production**:

```rust
quality[hh][g] = (0.30 + 0.62 * share + jitter).clamp(0.0, 1.0);
```

That is backwards — a big cheap deposit reads as fine stones. Replace `share` with
the mean `grade` of the workings inside the hub's catchment. Small change, fixes an
existing wrong formula, and is what makes the gem tier ladder mean anything.

**Gate:** `cargo test --lib econ_ -- --nocapture` before/after; the change should
move quality distributions, not price bands.

### Slice 3 — Txt import + the new goods

Format (INI-ish; CSV cannot carry optional/nested fields, JSON punishes a trailing
comma, and this must be hand-editable):

```ini
[lapis_lazuli]
name          = Lapis Lazuli
icon          = 🔷
color         = #26619c
domain        = continental
distribution  = deposits
deposit_model = contact_metamorphic
districts     = 1              # famously ONE source, for four thousand years
workings      = 3..6
grade         = 0.55..0.95
depth         = shallow
rarity        = 0.93
base_value    = 40
category      = gem
need_tier     = 2
```

Rules:
- **Only `[id]` and `name` are required**; `deposit_model` supplies everything else.
  A three-line block must produce a working mineral.
- **Always emit an import report** — parsed / defaulted / rejected *and why*. Never
  a silent drop; that is the same failure shape as FIX_PLAN B1's silent zero.
- Reuses the existing `get_goods_library` / `save_goods_library` file path, so an
  imported mineral survives across worlds (D8: add, never replace).

**Goods to add in this slice** (8 — each buys a mechanic; the long tail is exactly
what the import exists for):

| Good | Model | Why |
|---|---|---|
| mercury | VolcanicArc | Almadén and Idrija were effectively the world's only sources, and amalgamation is how Potosí's silver was refined from 1554 |
| alum | VolcanicArc | Cloth mordant → feeds the existing `cloth` recipe chain. Tolfa, 1462 |
| lapis_lazuli | ContactMetamorphic | The extreme single-district case |
| turquoise | Weathering (parent = copper) | Demonstrates the derived-parent rule |
| bog_iron | Bog | Lowland iron — impossible under an elevation floor |
| coal | CarbonatePlatform | Already referenced in `estate_kind_for_good`, missing as a good |
| garnet | CollisionalOrogen | The bottom rung of the gem ladder |
| carnelian | Rift | Demonstrates the rift model; the great Khambhat bead trade |

> **Mercury → silver is NOT a recipe.** The recipe system turns inputs into a
> *manufactured* output; silver is extracted. A consumable input to *extraction* does
> not exist yet. Slice 3 ships mercury with correct geology and high value; the
> amalgamation dependency is slice 4 work. Do not claim the chain is built.

The long tail for the import file: opal, peridot, jet, emery, natron, whetstone,
ochre, flint, antimony, bismuth, millstone, slate, granite, alabaster, saltpetre,
cobalt, calamine.

### Slice 4 — Mining as an industry

The campaign half, and the first slice that can move the economy bands.

- **Mining capability per city** — a four-tier ladder matching the depth classes.
  Raises `depth_workability` for workings in its reach.
- **Mine vs quarry as different mechanics:**

  | | Mine | Quarry |
  |---|---|---|
  | Constraint | depth, drainage, capital, timber | **transport** — stone is `bulk` 4+ |
  | Founded by | a house or wealthy city (capital-intensive) | a settlement (cheap, useless far from water) |
  | Fails when | water wins | haul cost exceeds the stone's value |

  The exception that proves the quarry rule: **Mons Claudianus**, the Roman
  granodiorite quarry 120 km into the Egyptian Eastern Desert with no water and no
  food, which existed only because the state paid. Worth supporting as a
  treasury-drain edge case.
- **Founding gated on a real deposit** — today an estate becomes a "mine" if the hub
  happens to have a mineral in its basket, at any richness, with no reference to
  whether a deposit exists nearby.
- **Depletion per D3** — only weak bodies, only under sustained large-scale
  extraction, only to a floor.
- **Mercury → silver amalgamation** as a consumable extraction input.

**Gate:** `cargo test --lib simulate_decades_reports_dynamics` **and**
`cargo test --lib econ_` (§2.1 and §2.5 both apply). This slice *will* move numbers.

> **Sequencing hazard.** Capping the settlement catchment (the separate supply-shed
> work) and adding depth gating both *reduce* supply. Landing both before measuring
> risks tuning two new constraints against each other blind — exactly the failure
> §2.4 warns about. **The exploitation-distribution measurement must exist first.**

### Slice 5 — The quarry window, mining settlements, growing catchment

- **Quarry / mine window** — districts, expanding to workings (D6). Each row: good,
  model, grade tier, extent, depth, whether it is currently workable.
- **Mining settlements (the Potosí class)** — a settlement whose existence is the
  deposit: terrible habitability (Cerro Rico sits at 4,090 m with no agriculture),
  all food imported via the existing colony food-lifeline path, explosive growth
  (Potosí reached ~160,000 by 1600, among the largest cities on Earth), and — per D3
  — decline rather than death. Marked as a distinct settlement class.
- **Growing catchment** — +10–20 km on the existing 50–120 km base as population
  rises. Store as **one float per hub** (`radius_km`), grow slowly, rasterise only
  in the view. No per-cell campaign state, no conflict with §3.4's snapshot rule,
  and the province view gets a disc that visibly grows across the year slider.

The honest pre-modern number: a cart hauls grain economically ~30–50 km; a city
extends past that only via water. So +10–20 km on growth is right, and the shed
should not scale freely with population.

---

## 6. Deliberately NOT built

Recorded so a future session does not read absence as oversight.

- **Rock type / stratigraphy.** No lithology column. Every model infers its host
  from relief, tectonics and climate. Adding real stratigraphy is a much larger
  change than this plan.
- **Deposit discovery over time.** Every working is placed at worldgen and known
  from day one. A prospecting mechanic (a placer leading to the lode above it) is a
  natural follow-on and is *not* in this plan, though the placer/parent link is the
  data it would need.
- **Ore beneficiation and smelting chains.** Ore → metal is not modelled as a
  manufacturing step; a mine produces the finished metal.
- **Mercury as an extraction input** until slice 4 (see the warning above).
- **Settlement death by exhaustion**, per D3.
- **Contesting a deposit by war.** Same gap as rule 24's held provinces.

---

## 7. Order

1. **Slice 1** — geological placement. *Partly built; finish and gate.*
2. **Slice 2** — grade → quality rewire.
3. **Slice 3** — txt import + 8 new goods.
4. *(Prerequisite)* exploitation-distribution measurement.
5. **Slice 4** — mining industry, depth gating, mine/quarry split.
6. **Slice 5** — quarry window, mining settlements, growing catchment.
