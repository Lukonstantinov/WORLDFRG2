# Goods localities, rivers, and the two-layer goods map

> **APPROVED, NOT YET BUILT.** Decisions in §2 are settled with the maintainer.
> Read §1 first — the measured findings are the reason any of this exists.

Trade goods are the one half of the world pipeline that never got the treatment
minerals got. `sim/step8_biological_goods/deposits.rs` (§8.16) placed ore by real
geology with a three-level hierarchy and per-working state; the biological and
agricultural goods beside it are still one smooth blob per good. This plan closes
that asymmetry, adds the river factor that placement has never had, and splits the
map overlay into the two questions it currently conflates.

---

## 1. Measured findings (why)

Each of these was read out of the code, not assumed.

**F1 · A belt has no internal structure.** `localize_good` (`biological.rs:1361`)
either blankets every suitable cell (`Global`: wheat, iron, timber, salt, furs, and
the six staples) or flood-fills ONE contiguous homeland from a single seed
(`Local`), then `dilate_belt` adds 2 decay rings. Quality inside a belt is nothing
but the climate score, which varies over hundreds of km. There is no locality, no
terroir, no cluster — a wine belt is a single smooth wash.

**F2 · Minerals already have what goods lack.** `deposits.rs` runs belt (100–1000
km) → district (10–60 km, `MIN_DISTRICT_SEP_KM` 320) → working (one cell), with
per-working `grade`/`extent`/`depth` persisted to `metadata["deposits"]`. The
province view can draw real ore dots because that data exists. It draws hashed
emoji for everything else because it does not.

**F3 · The province plate cannot honestly draw areas today.** `prov_good_belt` is
ONE number per (province, good). `ProvinceMiniMap.tsx` plate 6a says so in a
comment and places symbolic markers instead — correctly, under rule 17. Any
"squares of goods" feature is blocked on generation producing real sub-province
positions first. This is the load-bearing constraint of the whole plan.

**F4 · The world overlay draws coarse axis-aligned blocks.** `compute_good_regions`
rasterises belts at `f = grid_w / 450` (≈8×8 world cells at 3600 wide) and the
frontend `fillRect`s each block, so a belt's edge is a staircase that ignores the
coastline and spills into the sea.

**F5 · Marine goods use one undifferentiated gate.** `sea_coastal = is_shelf == 1
|| has_land_within(3)` serves pearls and stockfish alike. There is no inshore /
offshore-bank distinction, so a fishing bank sits on the beach and a pearl bed sits
on the open shelf. In the province view marine markers land on the province's
*land* edge cells, because the province raster is land-only (sea is `NO_PROVINCE`).

**F6 · Placement never reads rivers.** `good_score` takes no river input at all;
`compute_trade_goods` receives `rivers: &[River]` and passes it only to
shark/shipworm/disease/salt-pan. Rivers reach goods solely through `fertility`'s
0.20 river-proximity term, which is a single scalar with no notion of floodplain,
delta, navigability or irrigation. So rice has no paddy preference, cotton no
irrigation preference, timber no float-to-market preference. (Placer gold and bog
iron are unaffected — `deposits.rs` already walks rivers for those.)

**F7 · A correction to an earlier claim.** `Domain::Coastal` is NOT mishandled.
`envelope_score` gates it to land within `distance_to_ocean < 0.12`, which is right
for citrus and cinnamon; the two Coastal *deposit* goods route through the geology
placer. There is no Coastal bug. The marine work is the inshore/bank split only.

---

## 2. Decisions taken

| # | Decision | Chosen |
|---|---|---|
| D1 | Cluster realism | **Real locality layer** — belt → locality → cell, persisted to `metadata["good_localities"]` exactly as `deposits` is |
| D2 | Economy coupling | **Wire into production, iterating** — locality grade feeds quality, measured with `econ_` before and after |
| D3 | Overlay shape | **Full-res land clip, smooth outline** — belts stay physical, not snapped to province polygons |
| D4 | Marine | **Inshore band vs offshore bank**, per good; provinces gain NO maritime territory |
| D5 | Belt modulation | **Full modulation** — belt genuinely thinned between cores, with a hard floor (§5.1) |
| D6 | Global goods | **Yes, with much larger clusters** — the chernozem case |
| D7 | Old saves | **Require re-running Biological (8)**; no lazy derivation |
| D8 | Naming | **Notable localities only**, in the province culture's language |
| D9 | Map layers | **Two per good** — coverage and quality, independent toggles |
| D10 | Quality ramp | **One absolute 0–1 scale shared by every good** — never per-good normalisation |

### 2.1 The size ladder

A cell is 11.1 km at 3600×1800 (5.6 km on Large), so sizes are stated in km and
converted per world, never in cells.

| Tier | Span | Cells @3600 | Goods |
|---|---|---|---|
| Ore district *(shipped)* | 45 km | 4 | minerals |
| Luxury locality | 175 km | 16 | wine, silk, spices, cacao, cloves, pepper |
| Pastoral / secondary | 400 km | 36 | wool, hides, horses, timber, tobacco |
| **Staple region** | **900 km** | **81** | grain, rice, iron, salt, furs, barley, millet |

The staple tier is the point of D6: ~20× the linear span of a mining camp, so a
grain region covers a large piece of a continent and routinely spans several
provinces — the Ukrainian chernozem, not a farm.

---

## 3. Data model

```rust
/// One locality: a terroir patch inside a good's belt. The agricultural counterpart
/// of `deposits::Deposit`, and deliberately the same shape so the province query's
/// raster-attribution code is reused rather than rewritten.
pub struct GoodLocality {
    pub good: String,      // spec id
    pub x: u32, pub y: u32,// centre cell
    pub radius_km: f32,    // §2.1 tier, resolved per world
    pub grade: f32,        // 0..1 mean quality of the patch
    pub extent: u8,        // cells the patch covers (binned)
    pub name: String,      // empty unless notable (D8)
    pub river_fed: bool,   // §4.2 — this patch owes its grade to a river
}
```

Persisted to `metadata["good_localities"]` as JSON, exactly as `deposits` is. **No
tile-column change**, so rule 7 and every `.worldforge` save format stay untouched.

The two map layers (D9) need NO new data: coverage is `belt_value > threshold`,
quality is the belt value itself. Both read the existing u8 column.

---

## 4. Slices

Ordered so that **measurement comes first** and **the maintainer can see clusters
before the economy changes** — the render slices deliberately precede the
production wiring, so cluster sizes can be tuned by eye against a map that is not
yet feeding wealth.

### Slice 0 · The coverage diagnostic *(no behaviour change)*

The gate every later slice is measured against, built before anything can move.
A test in `biological.rs` (or a new `goods_validation.rs`) that, for one reference
world, prints per good: belt cells, distinct provinces touched, **settlements with
the good inside their catchment**, and mean/peak belt value. Asserts the floor that
matters — **no enabled good reaches zero settlements** — mirroring
`no_shipped_mineral_places_nothing`.

*Gate:* new test passes; baseline table appended to `docs/SCOREBOARD.md`.

### Slice 1 · Rivers as a placement factor (F6)

Build a river field once per world in `compute_trade_goods` (multi-source BFS from
all river cells at once — never an outward scan per cell, §8.9 rule 1), carrying
distance-to-river, whether the nearest river is `navigable`/`major`, and
delta/floodplain membership from `River.delta` + `mouth_kind`.

Per-good use, added to `good_score` and as new `Envelope` fields for custom goods:

| Factor | Meaning | Goods |
|---|---|---|
| `floodplain` | delta / low flat ground near a major river | rice, cotton, indigo, sugar, wheat |
| `irrigation` | river water in an arid climate (B zones) | cotton, dates, sugar, rice |
| `riverbank` | within a few cells of any river | paper (papyrus/bamboo), honey, hides |
| `float_out` | near a **navigable** river — the good can reach market | timber, hardwoods, furs |

`float_out` is the one that also serves the settlements constraint: a timber
locality on a navigable river is worth more than one stranded inland, which is
historically exactly right (the Baltic and Canadian timber trades were river
trades).

*Gate:* Slice 0's table re-run — coverage per good must not fall; rice/cotton
should visibly move toward river valleys. Deposits untouched (Placer/Bog already
walk rivers). Any good that LOSES settlements is a tuning failure, not a result.

### Slice 2 · Marine bands (F5)

Split the marine gate into **inshore** (shelf cell adjacent to land) and **bank**
(shelf cell ≥2 cells from land, still inside the shelf) and give each marine good a
band, via a serde-defaulted `marine_band` field on `GoodSpec` with sensible
built-in defaults: inshore = pearls, coral, bay salt, tyrian purple, amber; bank =
stockfish, herring, whaling.

*Gate:* Slice 0 table; a new assertion that no bank good places a cell adjacent to
land and no inshore good places one beyond the shelf edge.

### Slice 3 · The locality generator + full modulation (D1, D5, D6)

Cluster within each placed belt only (cost proportional to belt area, not world
area). Locality count scales with belt area against the §2.1 tier; centres are
picked by suitability × a minimum separation of one tier radius, exactly the
`MIN_DISTRICT_SEP_KM` pattern. Then modulate:

```
belt[i] = max(FLOOR, belt[i] * (FRINGE + (1 - FRINGE) * locality_influence[i]))
```

**`FLOOR` is the entire safety mechanism for D5.** A cell already in the belt never
falls to zero — it thins toward a fringe that still produces. `FRINGE` and `FLOOR`
are the two constants to tune against Slice 0's table.

*Gate:* Slice 0 table, **per-good coverage diff printed before/after**. A good that
loses settlements means `FLOOR` goes up. Dynamics test + `econ_` scorecard read to
confirm the belt change alone has not moved the economy in an unintended way.

### Slice 4 · Notable naming (D8)

Localities above a quality threshold draw a deterministic name from
`sim/shared/toponyms.rs` in the province culture's language. Threshold chosen so a
world yields tens, not thousands.

*Gate:* deterministic — same seed, same names; no unnamed locality carries a name.

### Slice 5 · Global map: two layers + full-res clip (D3, D9, D10)

Replace the coarse-block path with a full-resolution belt mask clipped to the land
mask, bordered along cell EDGES (the technique `drawStates` already uses on the
province raster). Emit coverage and quality as separately toggleable overlays per
good; quality shades on one absolute 0–1 ramp shared by all goods (D10).

*Gate:* `npx tsc --noEmit`; visual check that a belt meeting the sea ends on the
coastline; no per-good normalisation anywhere in the render path.

### Slice 6 · Province view: locality squares (F3, D4)

Replace plate 6a's hashed markers with real locality squares clipped to the
province footprint, opacity carrying grade, hue from the existing `GOOD_DEFS`
table. Marine goods draw in the adjacent sea using the Slice 2 bands — the province
gains no maritime territory (D4). Ore workings stay on their own Deposits plate,
unchanged.

*Gate:* `npx tsc --noEmit`; a landlocked province shows no marine patch; a province
with no locality of a good shows no square for it.

### Slice 7 · Production wiring (D2)

Locality grade feeds `province_good_potential` and per-producer quality, so a house
working a fine locality genuinely produces finer goods.

*Gate:* **`cargo test --lib econ_ -- --nocapture` read before and after**, plus
`simulate_decades_reports_dynamics`. This slice can move wealth and quality
distributions; the reading is the deliverable, not an afterthought. If a band moves
out, tune here rather than in the generator, which Slices 0–3 have already proven.

---

## 5. Risks

**5.1 · Full modulation versus "goods must reach settlements".** D5 and the
maintainer's own constraint pull against each other: thinning marginal cells is
exactly what can drop a good's last producer near some port. Mitigated by the
`FLOOR`, by Slice 0 existing before any change, and by the coverage layer (D9)
making a shrinking belt visible on the map rather than buried in a diagnostic.
**A coverage loss is a revert or a retune, never a judgement call** (§2.4).

**5.2 · D7 strands frozen worlds.** Requiring a re-run of Biological (8) means a
world already finalised with a live campaign can never gain localities —
`ensure_unfrozen` blocks phase 8. Accepted knowingly; the escape hatch, if it is
ever wanted, is the lazy derivation rejected in D7.

**5.3 · Metadata size.** ~45 goods × tens of localities is far smaller than the
existing `deposits` list, so JSON in `metadata` is fine. Worth one measurement in
Slice 3 rather than an assumption.

**5.4 · Rivers may move goods off their historical homelands.** Slice 1 adds a term
to a score that is already tuned; a floodplain bonus large enough to matter can
pull rice out of its climate band. The bonus is a MULTIPLIER on an existing score,
never a replacement for the climate gate.

---

## 6. Deliberately NOT built

- **Province polygons as belt boundaries** — rejected in D3. A wine belt crossing
  three provinces is one belt; snapping it to political lines would make a physical
  fact look administrative.
- **Maritime province territory** — rejected in D4. Marine goods draw in the sea
  but no province owns water, because land use, tenure and the revenue pass would
  all then need an answer for it.
- **Lazy locality derivation for old saves** — rejected in D7 (see 5.2).
- **A per-locality depletion/exhaustion state.** `prov_good_depletion` already
  exists at province granularity; a second per-locality one is duplicate state
  until something reads it.
- **Sub-cell terroir.** A cell is 11 km; there is nothing below it to model.

---

## 7. Order

`0 → 1 → 2 → 3 → 4 → 5 → 6 → 7`. Slices 0–4 are Rust and gated by the coverage
table; 5–6 are frontend and gated by `tsc` plus the eye; 7 is the only slice that
may move the economy, and it goes last on purpose so every earlier change has
already been proven neutral.
