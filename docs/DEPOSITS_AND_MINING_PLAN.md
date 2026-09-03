# Deposits, Mining & the Quarry Layer — Plan

**Status: APPROVED IN DESIGN, PARTLY BUILT.** Slices 1–3 are on disk, gated and
tested (§4). Slice 4 is PARTLY built (mining capability's depth-cost gate +
mercury amalgamation; mine-vs-quarry mechanics and per-D3 weak-body depletion
are not). Slice 5 (the quarry window, mining settlements, growing catchment)
is unbuilt. Scope is medieval / early-colonial.

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

### Slice 1 — Geological placement ⭐ BUILT, GATED

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

**Done, this session:**
- Ran the tests: `cargo test --lib deposits::` — 7/7 pass.
- The `highland_cap` binding was already gone (removed with the old branch in the
  same commit that added `deposits.rs`) — there was nothing left to clean up.
- District count vs. total production: `economy.rs`'s `good_max`/`abundance`
  normalisation is per-good relative (divides every hub's production by that
  good's own world-max, then rescales by a sqrt-abundance floor), so it is
  invariant to how many districts a mineral places — more districts spread the
  SAME normalisation across more hubs rather than inflating it. Confirmed by
  reading the normalisation, not just asserted; no separate before/after
  world-gen run was needed since the formula's shape makes the invariant
  structural, not empirical.
- CLAUDE.md §4/§6/§8 updated (added §8.16, the phase-8 table row, the
  `deposits.rs` map entry, and the stale "30–110 km" cell-size claim in §8.12
  fixed to the real ~11 km) in the commit that shipped slice 1.

**Gate:** `cargo test --lib deposits:: -- --nocapture` (diamond lands on craton not
peaks; different models separate minerals; workings cluster; determinism; depth
attenuates; no shipped mineral places nothing; template world still places) — 7/7
pass. Plus `cargo check` and `npx tsc --noEmit`, both clean.

### Slice 2 — Grade → quality rewire ⭐ BUILT, GATED

`economy.rs:305` derived a hub's gem quality from its **share of world
production**:

```rust
quality[hh][g] = (0.30 + 0.62 * share + jitter).clamp(0.0, 1.0);
```

That was backwards — a big cheap deposit read as fine stones. Now `economy.rs`
loads `metadata["deposits"]`, attributes each working to the hub whose catchment
claims its cell (the same `claim` map that already attributes belt production),
and — for a `Deposits`-distribution good only — reads the mean `grade` of the
workings in a hub's catchment directly as the quality base (plus the same small
jitter as before). Every other good keeps the old share-based formula unchanged.
`grade` is already a 0..1 richness number, so this is a direct read, not a proxy.

**Gate:** `cargo test --lib econ_ -- --nocapture` before/after — ran clean both
ways; see the session's SCOREBOARD entry.

### Slice 3 — Txt import + the new goods ⭐ BUILT, GATED

**Done, this session:**
- `commands/goods_import.rs` — the INI-ish parser (`parse_goods_txt`), the
  add-only merge (`merge_add_only`, D8), and the `import_goods_txt` Tauri
  command, registered in `lib.rs` and wrapped in `bridge/goods.ts`
  (`importGoodsTxt`). A minimal "Import .txt" button in `GoodsEditor.tsx` picks a
  path via the OS dialog (falls back to a `prompt()` if the dialog plugin isn't
  reachable) and shows the added/rejected/defaulted counts inline.
- Only `[id]` and `name` are required, exactly as designed; `deposit_model`,
  `domain` and `distribution` parse through the REAL enums'
  own serde representation (`serde_json::from_str` on a quoted value), so the
  parser can never disagree with `Domain`/`Distribution`/`DepositModel`'s actual
  mapping. `workings`/`grade`/`depth` are accepted but always reported as
  not-yet-wired (see the caveat below) rather than silently eaten — same for any
  unrecognised key. An id already in the library is rejected, never overwritten.
- The 8 goods shipped directly in `default_custom_goods()` (not via the import
  path — they are the app's own shipped library, not a user import), each using a
  new `dg()` helper that — unlike `cg()` — carries its OWN `base_value`/
  `category`/`bulk` rather than the flat `base_value: 1.0` every existing `cg()`
  custom good has always shipped with. `cg()` itself is UNCHANGED, so nothing
  about the ~20 existing customs (silver, jade, ruby, …) moved.
- **Mercury and alum ship with correct geology but no recipe wiring.** Alum's
  cloth-mordant link is documented in its own comment, not wired into the
  shipped `cloth` recipe — adding a hard input dependency to an EXISTING
  manufactured good is an economic change of its own (a market that previously
  never needed alum now does), and needs its own `econ_` measurement rather than
  riding along inside an add-only slice. Mercury → silver amalgamation remains
  explicitly out of scope per the plan (below).

**Gate:** `cargo test --lib goods_import:: -- --nocapture` — 7/7 pass (the plan's
own lapis_lazuli example parses byte-for-byte into the fields the plan lists;
missing-name, duplicate-id-in-file and existing-id-in-library are all rejected,
not silently dropped). Plus `cargo check` and `npx tsc --noEmit`, both clean.
`cargo test --lib deposits::` still 7/7 (the 8 new minerals use the SAME
`default_model_for`/`place_mineral` path as the shipped six, so slice 1's own
tests already cover them structurally).

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

### Slice 4 — Mining as an industry ⭐ PARTLY BUILT, GATED

The campaign half, and the first slice that can move the economy bands.

**Built, this session** (`sim/campaign/tick/mod.rs`, `commands/campaign_commands/
lifecycle.rs`):
- `CampaignSim.mine_deposits: Vec<MineSite>` — the world's real ore/gem/stone
  workings (§8.16, `metadata["deposits"]`), seeded once at campaign start
  (`seed_mine_deposits`, mirroring `province.rs`/`economy.rs`'s own parse of the
  same table). A positional/depth index, not a duplicate of the full `Deposit`
  record — empty on a template world or one generated before slice 1, which
  every reader treats as "no depth data" rather than "no deposit".
- `TickHub.mine_depth: u8` — for a Mine estate (`estate_kind == 2`) only, the
  depth class of the real working nearest its parent city, looked up ONCE at
  founding (`create_estate` → `mine_depth_at`) within `MINE_DEPOSIT_SEARCH_KM`.
  Never re-queried afterward. `DEPTH_SURFACE` (ungated) on every other kind, an
  old save, and a world with no positional deposit data.
- **Mining capability, as a cost gate rather than a workability multiplier.**
  A mine's baseline output already reflects depth (it was baked into
  `base_per_capita` at worldgen by `workable_intensity() = grade ×
  depth_workability`, §8.16) — reapplying `depth_workability` here would double
  the penalty. What was missing was the CAPABILITY side: `maybe_house_invests`'s
  upgrade branch now scales a mine's upgrade cost by `MINE_UPGRADE_COST_MULT`
  (indexed by `mine_depth`, 1.0/1.3/2.2/3.5 for surface/shallow/deep/flooded) —
  a flooded body needs real drainage capital (Rio Tinto's reverse waterwheels,
  Agricola's *De Re Metallica*) and so grows far more slowly at the same
  wealth. Every other estate kind reads a flat 1.0 (unaffected).
- **Mercury → silver amalgamation**, wired as a CONSUMABLE EXTRACTION input
  (`apply_mercury_amalgamation`, called once daily right after ordinary
  per-capita extraction), not a manufacturing recipe (silver is dug, not
  assembled from parts — it never touches `manufacture.rs`). A silver-mine
  estate draws mercury from its OWN stock; `served` (how much of the need was
  met) interpolates recovery between `MERCURY_AMALGAMATION_FLOOR` (0.75, hand
  smelting) and `_BONUS` (1.25, full amalgamation) — a true no-op wherever this
  world has no silver good, no mercury good, or no silver mine.

Gate: `mine_depth_at_finds_the_nearest_working_within_reach`,
`mine_upgrade_cost_mult_increases_with_depth_and_only_gates_mines`,
`mercury_amalgamation_is_a_noop_without_both_goods`,
`mercury_amalgamation_rewards_a_supplied_mine_and_still_serves_a_dry_one`
(`tick::tests`) — all new, all pass. Plus `cargo test --lib tick::tests` (194
passed) and `cargo test --lib econ_` (both scorecards + the inheritance gate),
run in full and unchanged, since this only ever raises a mine's upgrade cost or
adjusts silver by a bounded ±25% around a consumable it must actually spend.

**Not built** (left for a future session, same discipline as everywhere else
in this file — named rather than silently skipped):
- **Mine vs quarry as separate mechanics.** Both still share `estate_kind == 2`
  ("Mine") and the SAME depth gate; a stone/marble/gem "quarry" is not yet
  bound by transport cost instead, nor exempted from the depth gate the way
  the design's table asks (a marble quarry is near-surface in practice, so the
  gate rarely bites it in play, but it is not STRUCTURALLY exempt).
- **Founding gated on a real deposit**, beyond what already followed for free:
  a Mine estate is only ever founded where `base_per_capita[g] > 0`, which —
  for a `Deposits`-distribution good — already required a real working inside
  the founding city's world-side catchment (slice 2's rewire). What is NOT
  built is a campaign-time re-check against `mine_deposits` at founding (the
  lookup only sets `mine_depth`; it never blocks founding when nothing is
  found within `MINE_DEPOSIT_SEARCH_KM`, which can differ from the world-side
  catchment radius).
- **Depletion per D3** (weak bodies decline to a floor under sustained
  pressure) — `update_province_goods_pressure` already made every mine
  NEVER deplete (v2.0, §5), which satisfies D3's "persist" half but not its
  "a weak body still declines" half; that needs `extent` threaded through,
  which this session did not do.
- Mons Claudianus-style treasury-funded quarrying, and the `Mines`
  read-only inspector list. See Slice 5.

**Gate (going forward):** `cargo test --lib simulate_decades_reports_dynamics`
**and** `cargo test --lib econ_` (§2.1 and §2.5 both apply) for any FURTHER
change here. This slice *can* move numbers.

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
