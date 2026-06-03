# Trade Goods & Seasonal Hazards — Design Document

Status: **DESIGN ONLY (no code yet)** · Branch: `claude/trade-goods-review-Ju0R1`

This document specifies three connected features for the Biological-Trade phase:

1. **A data-driven trade-good system** — every good (built-in + user-added) is a
   declarative *spec* with editable parameters (name, icon, domain, distribution,
   rarity, desire, climate envelope). Replaces the hardcoded `match g` scorer.
2. **New goods** — tropical hardwoods, horses, two wool subtypes, ivory, cacao
   (and a metals batch as an optional follow-on), all expressed as specs.
3. **Seasonal maritime hazards** — open-ocean storm/cyclone belts with a
   per-month slider + a combined static danger layer, plus reef/shoal wreck
   zones. Storm danger feeds back into trade routing and good desirability.

It also covers the **save-format versioning** required to make a variable number
of goods safe.

---

## 0. Current state (baseline)

- `sim/biological.rs::good_score` is a hardcoded `match g { GOOD_SILK => … }` over
  **21** goods (`GOODS_COUNT`). Three distribution models: UNLIMITED (every
  suitable cell), SEEDED (one flood-filled homeland), GEMSTONES (discrete highland
  deposits).
- Goods stored as 21 `u8` columns per tile; `shark_risk` then `goods` then
  `shipworm_risk` serialized **last**, relying on "append-last + zero-pad" for
  back-compat. **This breaks if `GOODS_COUNT` changes** (the goods loop eats the
  shipworm bytes of older saves). Fixed in §4.
- Two hazards: `shark_risk`, `shipworm_risk` (`u8` columns) → `compute_*_zones`
  overlays. Both warm-shallow-coastal and overlapping.
- Trade matrix demand = `region_size × demand_weight[g]` (a fixed 21-length array).

---

## 1. Data-driven good system

### 1.1 The `GoodSpec` schema

A good becomes a serializable spec (Rust struct + TS interface, 1:1):

```rust
struct GoodSpec {
    id: String,            // stable key, e.g. "silk", "custom_jade"
    name: String,          // display label (editable)
    icon: String,          // emoji / glyph (editable)
    color: String,         // hex region tint (editable)
    enabled: bool,         // generate this good at all?

    domain: Domain,        // where it may sit
    distribution: Distribution,
    rarity: f32,           // 0..1  (scarcity → fewer/smaller sources)
    desire: f32,           // 0..1  (base demand weight in the trade matrix)
    network_luxury: bool,  // desire only realized in large trade networks (§3.4)

    // Scoring envelope (all optional; absent term = neutral 1.0):
    climate: Vec<(KoppenZone, f32)>,  // sparse per-zone suitability 0..1
    temp:      Option<Bell>,          // { center, width } on °C
    precip:    Option<Band>,          // { lo, hi, edge } on mm/yr
    elevation: Option<Band>,          // { lo, hi, edge } on normalized 0..1
    abs_lat:   Option<Band>,          // { lo, hi, edge } on |latitude|
    fertility: f32,                   // weight 0..1 → (1-w) + w*fert
    coast_bonus: f32,                 // extra for coast-near land cells
    marine: Option<MarineEnv>,        // shelf/depth/fishery/current/salinity terms
}

enum Domain { Marine, Coastal, Continental, Island }
enum Distribution { Global, Local, Deposits }
```

`Bell { center, width }`, `Band { lo, hi, edge }`, and `MarineEnv` (shelf weight,
max depth, fishery weight, cold-current bonus, low-salinity bonus) cover every
term the current `good_score` arms use. **All 21 built-ins are re-expressible**
with no behavior change — this is a refactor, not a rebalance.

### 1.2 Generic scorer

`good_score(spec, buf, x, y) -> f32` replaces the match:

```
domain gate:
  Marine      → sea cell on/near shelf or within 3 of land
  Coastal     → land cell with distance_to_ocean < coastal_thresh
  Continental → any land cell
  Island      → land cell whose landmass component size < island_thresh (§1.3)
score = climate_lookup(koppen)            // 0 if zone not listed
      * temp.map(bell).unwrap_or(1)
      * precip.map(band).unwrap_or(1)
      * elevation.map(band).unwrap_or(1)
      * abs_lat.map(band).unwrap_or(1)
      * ((1-fertility) + fertility*fert)
      * (1 + coast_bonus * coast_near)
      * marine.map(marine_env_eval).unwrap_or(1)
```

### 1.3 Domains

- **Marine / Coastal / Continental** map directly to existing checks
  (`GOOD_MARINE`, `distance_to_ocean`, land).
- **Island** is new: a one-time connected-component pass over land (cylindrical
  wrap, Y-clamp) labels each landmass with its cell count. A cell is "island" if
  its component size ≤ `island_max_cells`. **Default ≈ 0.5 % of total land, and
  exposed as an editable world/biological parameter** (`island_max_frac`). Cached
  per generation. Enables goods like spices/pearls/island-spice-isles to be
  island-locked.

### 1.4 Distribution × rarity

- **Global** (≈ old UNLIMITED): every passable, in-envelope cell produces.
  `rarity` raises the spread threshold (rarer → only the strongest cells).
- **Local** (≈ old SEEDED): one suitability-weighted seed, flood-fill homeland
  with the existing ~4 % island-jump. `rarity` lowers the seed/spread thresholds'
  generosity and caps belt size (rarer → smaller homeland).
- **Deposits** (generalized GEMSTONES): `place_deposits(spec)` scatters discrete
  blobs. Parameterized by `min_elev` (highland-lock for gems/metals; 0 = lowland
  ok), `count = base * (1 - rarity)` and blob `radius`. Gemstones, gold, copper,
  tin all use this with different `min_elev`/`count`.

### 1.5 Built-in defaults (examples)

```
silk:  Continental, Local, rarity .5, desire .35,
       climate {CFA:1, CWA:1, CFB:.6, CSA:.6, DFA:.4, DFB:.4},
       temp{18,7}, precip{600,1600,500}, fertility .6, elevation cap≈band(_, .4,.7 falloff)
iron:  Continental, Global, rarity .3, desire .65,
       elevation band{.30,.68,.16}, temp not-frozen, volcanic bonus
gemstones: Deposits(min_elev .40), rarity from gem_deposits count, desire .40
```

(The full default table for all 26 lives in `goods_defaults.rs` / `goods.ts`.)

### 1.6 Storage: per-world + global library

- **Global library** — `goods_library.json` in the Tauri app-config dir. Holds the
  default built-ins + any user custom goods. The editing template for *new*
  worlds; survives across worlds. CRUD from the editor (§5).
- **Per-world snapshot** — when Biological-Trade runs, the active good-set (with
  per-world tweaks) is written to the world DB `metadata` key `goods_spec` (JSON).
  Generation and rendering read this; a saved `.worldforge` reproduces its exact
  goods regardless of later library edits.
- Opening a world loads `goods_spec` from its DB; the library is untouched.

### 1.7 Frontend impact

`goods.ts`'s static `GOOD_DEFS` array is replaced by a spec list fetched from the
backend (`get_goods_spec` command) for the active world, defaulting to the library
for new worlds. Toolbar good-toggles, region overlays, the trade matrix, and the
InfoPanel all read names/icons/colors from this list — they already iterate by
index, so they become count-agnostic for free.

---

## 2. New goods (as specs)

All land goods unless noted. Formulas mirror the house scoring style.

| id | name / icon | domain | dist. | rarity | desire | envelope |
|----|-------------|--------|-------|-------:|-------:|----------|
| `hardwoods` | 🌳 Tropical Hardwoods | Coastal | Local | .55 | .55 | AF/AM=1, AW=.5, CWA=.3; precip≥800↑1800; fert .6; elev cap .40–.65 |
| `horses` | 🐎 Horses | Continental | Local | .5 | .70 | BSK/BSH=1, CFB/DFB/DSB/CWB=.5, BWK=.3; open (elev<.45); precip band 250–700; temp bell{12,12} |
| `wool_fleece` | 🐑 Fleece Wool | Continental | Local | .45 | .50 | CFB/CFC=1, CSB/DFB/ET=.5, CWB=.4; elev band .10–.50; precip 600–1600; temp band 4–14 |
| `wool_llama` | 🦙 Highland Wool | Continental | Local | .55 | .40 | CWB/CWC=1, BSK=.5, ET=.4; **high** elev band .35–.70; abs_lat band; temp band 2–12 |
| `ivory` | 🐘 Ivory | Continental | Local | .6 | .35 | AW/AS=1, BSH=.5, AM=.4; abs_lat 0–20; elev<.4; precip 400–1200 |
| `cacao` | 🍫 Cacao | Continental | Local | .55 | .40 | AF/AM=1, AW=.5; temp≥22↑27; precip 1500–3000; elev<.20–.45; fert .5 |

**Wool subtypes** (your lamb-vs-llama point): two *distinct* Local goods that seed
on different continents (different climate centers — lowland oceanic vs highland
dry-winter), with different `desire` and `network_luxury = true`, so each is a
separate trade monopoly and they only become valuable across a large network (§3.4).

**Metals batch — included in Round 1** (your "bronze-age" set): `copper`
(Deposits, min_elev .30, desire .55), `tin` (Deposits, min_elev .35, volcanic
bonus, rarity .7, desire .55), `gold` (Deposits, min_elev .45, very low count,
desire .60). All fall out of the generalized Deposits distribution with zero new
code. This brings the built-in count to **29** goods.

---

## 3. Seasonal hazards

### 3.1 New persisted columns (annual base only)

Add two `u8` tile columns (see §4 for the safe serialization):

- `storm_base` — annual cyclone potential of a **sea** cell.
- `reef_risk` — static reef/shoal wreck hazard of a **sea** cell.

Storm seasonality is **not** stored per month — it is derived analytically at
query time from `storm_base` + the cell's latitude/hemisphere (§3.3).

### 3.2 Storm base (open ocean, not coast-limited)

```
warm     = smoothstep(24, 27, T)          // tropical SST fuels cyclogenesis
lat_band = band(|lat|, 8, 30, 8)          // ~0 on equator, peak ~15-25°
storm_base = warm * lat_band              // open-ocean field, any sea cell
```

This is deliberately distinct from the coastal shark/shipworm hazards: storms
roam open water.

### 3.3 Analytic monthly curve

**Configurable calendar length** `M` ("moons"), an editable world parameter,
**default 12** (slider 1–`M`). A hemisphere-offset seasonal phase concentrates
danger into roughly half the year, with N and S hemispheres ~`M/2` apart. The
curve is written in terms of `M` so any calendar length works unchanged:

```
peak(lat)   = if lat >= 0 { 0.70·M } else { 0.20·M } // late-summer/autumn peak moon
phase(m,lat)= clamp(cos(2π·(m - peak(lat))/M), 0, 1)^p    // p≈1.5 → sharper season
              · equator_blend(lat)                   // smear toward year-round near 0°
storm_month(cell, m) = storm_base[cell] · phase(m, lat(cell))
```

(At `M = 12` the N/S peaks land on ~moon 8.5 / 2.5, ~6 months apart, as before.)

- **Per-month zone overlay:** `compute_storm_zones(month)` clusters
  `storm_month(·, month)` ≥ threshold; the overlay tints each zone by intensity
  and renders **near-transparent** where that month's value is below threshold
  (your "safe season" behavior).
- **Combined static layer:** `compute_storm_zones(None)` (and a `"storm"` render
  layer) use the **annual aggregate** `agg = mean_m storm_month` (≈ fraction of the
  year the cell is dangerous) → the "general static danger %" map.

No per-tile re-render per month: seasonality lives in the **overlay** (cheap to
recompute on slider move); the static tile layer shows the annual aggregate.

### 3.4 Feedback into trade (desire ↑, routes ↓)

The combined annual storm danger raises the cost of open-water trade edges:

- In `build_coarse_cost`, add `+ storm_penalty · agg_storm(edge)` to sea-edge
  cost. Routes bend around storm belts; under limited reach, distant pairs become
  **unreachable** → no flow.
- Emergent result: goods that can only arrive across stormy water show larger net
  deficits in the matrix → effectively "more desired / pricier" exactly as you
  described, with no separate price system.
- **Network-luxury desire** (`network_luxury` goods incl. the wool subtypes):
  effective demand is scaled by the size of the importer's reachable trade network
  (count of partners reachable under the chosen reach, from the path graph already
  built). Large networks crave the luxury; small/closed networks and the good's
  own homeland discount it. Formula:
  `demand = region_size · desire · (small_floor + (1-small_floor)·network_frac)`
  for `network_luxury` goods; staples keep the current flat `region_size · desire`.

### 3.5 Reef / shoal wrecks (static)

```
warm        = smoothstep(20, 25, T)
very_shallow= shelf ? 1 : (1 - (depth-0.06)/0.06)
coast       = coast-BFS proximity (like shark)
reef_risk   = warm · very_shallow · coast
```

Static layer + `compute_reef_zones` overlay (no slider). Also contributes a small
sea-edge cost near reefs (wreck risk on coastal hugging routes).

---

## 4. Save-format versioning (prerequisite)

Variable `GOODS_COUNT` makes the current "append-last + pad" scheme unsafe. Fix by
making tile blobs **self-describing** and versioning the world.

### 4.1 Tile blob header

Prepend each (pre-zstd) tile blob with:

```
[ magic u8 = 0xWF ][ version u8 = 2 ][ goods_count u16 ]
```

`decompress` reads `goods_count` from the header and loops that many good columns
(instead of the compile-time `GOODS_COUNT`), then reads the fixed hazard columns
in a versioned order:

```
v2 column order:  …existing… , salinity, shark_risk,
                  goods[0..goods_count], shipworm_risk, storm_base, reef_risk
```

A blob with **no/!=magic** header → treat as **v1**: 21 goods, `shark` before
goods, `shipworm` after the 21 goods, no storm/reef. Read with the v1 layout, then
up-convert in memory (storm/reef = 0 until Phase 8 re-run).

### 4.2 World metadata

- `metadata.format_version = "2"`.
- `metadata.goods_spec = <JSON GoodSpec[]>` (§1.6).
- On open: missing `format_version` ⇒ v1 → run migration (set goods_spec to the
  v1 default 21, mark hazards stale). Migration is read-time and lossless for
  everything that existed in v1.

### 4.3 Why not keep append-last

Append-last only works while the appended field is genuinely last and fixed-size.
Goods are now variable-length, so the count must be explicit. Self-describing
blobs also future-proof every later goods change (no more silent shipworm-eating).

---

## 5. Pre-generation editor (UI)

A panel surfaced in **StepBiological** (and/or the New World dialog), before
running Biological-Trade:

- **Good list** with per-good: enable checkbox, name, icon, color, domain dropdown
  (Marine/Coastal/Continental/Island), distribution dropdown (Global/Local/
  Deposits), rarity slider, desire slider, `network_luxury` toggle, and an
  "advanced" disclosure for the climate/temp/precip/elevation/lat envelope.
- **Add custom good** → new `GoodSpec` with sane defaults; **Duplicate**; **Delete**.
- **Save to library** (global) vs **apply to this world only** (per-world snapshot).
- Validation: warn if a good's climate envelope doesn't exist in the current world
  (so the user knows it won't generate).

**Editing reach: fully custom.** Both built-in and custom goods are fully
editable — every field, including the climate/temp/precip/elevation/lat envelope,
can be changed. "Reset to default" restores a built-in's shipped spec from the
defaults table; custom goods can be deleted outright. This is the last round
because it depends on §1 (the engine) and §3 (params) being in place.

---

## 6. File-by-file impact

### Round 1 — engine + new goods (incl. copper/tin/gold) + hazard bases + versioning
- `tile/cell.rs` — header magic/version/goods_count in `compress`/`decompress`;
  add `storm_base`, `reef_risk` columns; variable goods loop; v1 read path.
- `sim/world_buffer.rs` — new fields + load/save copy lines.
- `sim/biological.rs` — replace `good_score` match with the generic scorer over
  `GoodSpec`; `goods_defaults.rs` table; island-component pass; generalized
  `place_deposits`; `compute_storm_base`, `compute_reef_risk`.
- `db/schema.rs` / `db/metadata` — `format_version`, `goods_spec` keys; v1 migration.
- `commands/sim_commands.rs` — load spec, call new computes in all 3 phase-8 sites.
- `commands/query_commands.rs` — `compute_good_regions` & trade matrix read spec
  (count-agnostic); `compute_storm_zones(month: Option<u32>)`, `compute_reef_zones`.
- `render/tile_image.rs` — `"storm"` (annual) + `"reef"` layers.
- `lib.rs` — register new commands.
- `bridge/tauri.ts`, `goods.ts`, `types.ts` — fetch spec; `computeStormZones`,
  `computeReefZones`.

### Round 2 — seasonality
- `sim/biological.rs` / `query_commands.rs` — analytic `phase()`, monthly zones,
  annual aggregate; storm/reef sea-edge cost in `build_coarse_cost`;
  network-luxury demand scaling in the matrix.
- `state/uiStore.ts` — `stormMonth` (1–12) + overlay flags.
- `ui/Toolbar.tsx` — `🌀 Storm Belts` / `🪨 Reef Hazards` layers + zone toggles +
  **month slider**.
- `canvas/OverlayManager.ts`, `ui/MapCanvas.tsx` — `drawStormZones`/`drawReefZones`,
  per-month tint + transparency, recompute on slider move.
- `ui/workflow/StepBiological.tsx` — auto-enable overlays; `ui/InfoPanel.tsx` — risk bars.

### Round 3 — editor
- `ui/workflow/StepBiological.tsx` + new `ui/GoodsEditor.tsx` — the panel (§5);
  `bridge/tauri.ts` + commands for library CRUD and per-world snapshot.

---

## 7. Verification per round

- `cargo check` (from `src-tauri/`) + `npx tsc --noEmit` green.
- **Round 1:** load an old `.worldforge` (v1) → opens, existing goods intact,
  hazards stale-but-zero; re-run Phase 8 → 26 goods render with correct icons,
  storm/reef static layers populate. New save reloads identically.
- **Round 2:** slider scrubs months; N/S storm zones peak ~6 months apart; zones
  fade in safe months; static layer = annual aggregate; routes avoid/are severed
  by storm water; network-luxury goods show higher deficits in large networks.
- **Round 3:** edit a good's icon/params, add a custom good, generate → appears;
  library persists to a new world; per-world snapshot survives save/open.

---

## 8. Sign-off decisions (locked)

1. **Calendar length** — *configurable* "moons" count, **default 12**; analytic
   curve rescales to any `M` (§3.3).
2. **Island threshold** — default ≈0.5 % of total land, **exposed as an editable
   parameter** `island_max_frac` (§1.3).
3. **Metals batch** — copper/tin/gold **included in Round 1** → 29 built-in goods
   (§2).
4. **Seasonality scope** — **storms only** for now; reef/sea-ice stay static.
5. **Editor reach** — **fully custom**: built-in *and* custom envelopes editable,
   with per-good "reset to default" (§5).
```
