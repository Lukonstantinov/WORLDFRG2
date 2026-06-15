# WorldForge 2 — Performance Optimization Plan

*A design document for the performance workstream. No code changes are made by
this document — it is the implementation spec to be executed afterward.*

Scope: backend generation throughput, UI responsiveness during generation, and
frontend overlay/tile churn. **Out of scope:** economic-realism changes (tracked
separately) and any change to simulation *output* — every optimization here must
be **bit-for-bit output-preserving** (the sim is deterministic from `seed`, and
the test suite + saved worlds depend on that).

---

## 0. Guiding constraints

1. **Determinism is non-negotiable.** All sim phases are pure functions of
   `(seed, WorldBuffer)`. Parallelism must not reorder floating-point reductions
   in a way that changes results, and must not introduce position-dependent race
   conditions. Per-cell maps are safe; neighbor-writing stencils and reductions
   need care (§2.3).
2. **No behavior change on save/load.** Blob format (`[0xF2][2][goods u16]…`),
   column masks, LOD invalidation, and undo semantics stay observable-identical.
3. **Measure before and after.** Every workstream lands with a before/after
   timing on a fixed reference world (§7). "Faster" is a number, not a vibe.

---

## 1. Baseline: what is already optimized (do not redo)

So the plan doesn't re-litigate solved problems, the current good state:

- **Binary packed IPC** — `get_tiles_packed` (`commands/tile_commands.rs:60`)
  returns raw little-endian RGBA via `tauri::ipc::Response`, parsed with
  `DataView` (`TileManager.ts:55-77`). No base64 on the hot path.
- **Parallel blob compress/decompress** — `WorldBuffer::load_with`
  (`world_buffer.rs:~297-378`, `into_par_iter` decompress) and `save`
  (`world_buffer.rs:417` `par_iter` compress, batched into 256-row
  transactions). `rayon = "1.10"` is already a dependency.
- **Column-masked sim loads** — per-phase `ColumnSet` masks load only touched
  columns; `save` merges unmodified columns from old blobs.
- **Undo snapshots already-compressed blobs** — `save` no longer
  decompress→recompresses for the undo journal (`world_buffer.rs:383-396`).
- **LOD pyramid** persisted + fingerprint-invalidated; **RAF-gated render loop**
  (`MapCanvas.tsx:269-277`); **version-keyed tile/cost caches** (`db/mod.rs`).
- **Off-lock tile rendering** — `render_full_res` grabs blobs under the lock,
  drops the guard, then decompresses+renders in parallel (`tile_commands.rs:113-126`).
  This is the pattern §3 generalizes to the sim path.

The remaining wins are: (W1) the sim *compute* is single-threaded, (W2) the DB
lock is held across whole generations, (W3) `save` still re-reads the whole world
and recompresses unchanged tiles, (W4) supertiles copy all columns, (W5) frontend
overlay/tile effect churn.

---

## 2. Workstream W1 — Parallelize the simulation compute (largest win)

### 2.1 Problem
`rayon` is used only for blob (de)compression. Every compute phase is a serial
grid sweep. `sim_run_all` (`sim_commands.rs:196-251`) chains ~25 passes; the two
heaviest modules are `ocean.rs` (1364 lines) and `biological.rs` (1611). On a
7200×3600 world that is ~26M cells × ~25 passes single-threaded — most cores idle.

### 2.2 Classification of every pass (drives what we parallelize first)

| Class | Behavior | Passes | Action |
|---|---|---|---|
| **A. Pure per-cell map** | output[i] = f(input[i], lat) only | `temperature::compute_temperature`, `koppen::classify_koppen`, `soil::classify_soil`, `fertility::compute_fertility` (per-cell scoring part), per-cell `good_score` in `biological`, `render/tile_image.rs` per-pixel | **Parallelize first** — `par_chunks_mut` over flat arrays, near-linear speedup, trivially deterministic |
| **B. Read-neighbors stencil** | output[i] = f(input neighborhood); reads neighbors, writes self | orographic/advection steps in `precipitation.rs`, current/upwelling kernels in `ocean.rs` | Parallelizable with **double-buffer** (read prev, write next) — output unchanged; medium effort |
| **C. Sequential / order-dependent** | mutates shared frontier or accumulates in order | D8 flow accumulation (`rivers.rs`), flood-fill `localize_good` / lakes BFS, plate Voronoi growth (`plates.rs`), salinity plume BFS | **Leave serial** for now; parallelizing changes traversal order and risks output drift |

The point of the table: **W1 only touches class A (and later B).** Class C is
explicitly deferred so we never risk a determinism regression for the hardest,
lowest-ratio passes.

### 2.3 Pattern (class A)

The flat arrays on `WorldBuffer` are `Vec<T>` row-major (`width*height`). For a
per-cell map that needs latitude, chunk by row so each chunk knows its `y`:

```rust
use rayon::prelude::*;

// before: for i in 0..n { out[i] = f(temp[i], precip[i], lat_of(i)); }
out.par_chunks_mut(width)
    .enumerate()
    .for_each(|(y, row)| {
        let lat = lat_of_row(y, height, &lat_cfg);
        for x in 0..width {
            row[x] = f(temp[y*width + x], precip[y*width + x], lat);
        }
    });
```

Rules that preserve output exactly:
- **No cross-row writes** in class A (that is what makes it class A). Reads of
  other arrays are by absolute index and unaffected by chunking.
- **No floating-point reduction across the parallel region** — each cell's result
  depends only on its own inputs, so there is no sum/min order to perturb. (Where
  a pass *also* computes a global min/max for normalization, compute that in a
  separate deterministic serial reduction, or use an order-independent
  formulation, e.g. `reduce` with a commutative+associative op on a fixed
  partition — and verify against the serial result in a test.)
- **Cylindrical wrap** (`wrap_x`) only matters for class B/C; class A indexes its
  own cell, no wrap needed.

### 2.4 Deliverables / order within W1
1. `temperature::compute_temperature` — smallest, clearest first conversion;
   establishes the `par_chunks_mut(width)` idiom + a determinism test.
2. `koppen::classify_koppen`, `soil::classify_soil` — pure classifiers.
3. `fertility::compute_fertility` per-cell scoring (the BFS river-proximity field
   stays serial; only the final weighted-sum map parallelizes).
4. The per-cell `good_score` loop in `biological::compute_trade_goods` (the
   `localize_good` flood-fill stays serial).
5. `render/tile_image.rs` per-tile pixel loop (already per-tile; parallelize the
   inner pixel map or the per-tile loop in batch renders).

### 2.5 Validation
- Add `#[test]` per converted pass: run serial vs parallel on a small fixed
  buffer, assert **bitwise-equal** output arrays. Keep a `cfg`-gated serial path
  or a reference vector so the test is self-contained.
- The existing `deterministic_and_finite` campaign test and any worldgen golden
  tests must still pass unchanged.

### 2.6 Expected impact
Class A passes are a large share of `sim_run_all` wall time. With N cores the
per-cell maps approach ~N× on those passes; realistic end-to-end generation
speedup is **substantial but sub-linear** (Amdahl: class C remainder serial).
Quantify on the reference world (§7) — do not promise a multiplier in the doc.

---

## 3. Workstream W2 — Release the DB lock during sim compute

### 3.1 Problem
`sim_run_all` locks the global `Mutex<Connection>` (`db/mod.rs:22`) at
`sim_commands.rs:193` and holds it through **all ~25 phases plus load and save**.
The compute phases (`plates::…`, `ocean::…`, etc.) operate purely on the in-RAM
`WorldBuffer` — they never touch `conn`. But because the guard is held, any
concurrent command (notably `get_tiles_packed` from pan/zoom) blocks for the
entire generation. **The UI tile pipeline freezes for the whole run.**

### 3.2 Target shape (mirror `render_full_res`)

```
let buf = {
    let conn = db.conn.lock()?;          // (a) acquire
    ensure_unfrozen(&conn)?;
    WorldBuffer::load(&conn)?            // (b) read blobs
};                                       // (c) GUARD DROPPED here

// (d) compute — no lock held; cores free; tile fetches unblocked
plates::generate_plates_and_landmass(&mut buf, …);
… all phases …

let modified = {
    let conn = db.conn.lock()?;          // (e) re-acquire only for the write
    cultures::store_and_activate(&conn, cmap)?;  // any mid-run DB writes move here
    buf.save(&conn, "Full world generation")?
};
```

### 3.3 Complications to handle in implementation
- **`buf.save(&conn, …)` takes `&Connection`** — fine, just call it under the
  re-acquired guard (e).
- **Mid-run DB access exists.** `sim_run_all` does two DB things *between* load
  and save today:
  - `cultures::store_and_activate(&conn, cmap)` (`sim_commands.rs:239`)
  - `goods_commands::load_world_goods(&conn)` (`sim_commands.rs:245`)
  Both must be relocated: load goods **before** dropping the guard (b); store the
  culture map under the **re-acquired** guard (e), or in a short-lived re-lock.
  Audit every `&conn` use in each sim command and bucket it into "read up front"
  or "write at end."
- **`ensure_unfrozen` + freeze invariants.** The frozen check must happen under a
  held guard before compute; re-checking at save time is cheap insurance against
  a concurrent finalize (decide policy: last-writer-wins vs error).
- **Scope creep across commands.** Apply the same restructure to every long sim
  command, not just `sim_run_all`: `sim_run_all_from_terrain`, and the individual
  `sim_*` phase commands that currently `lock → load → compute → save` in one
  scope (`sim_commands.rs`). Each is a mechanical "drop the guard around compute."
- **`clear_caches()` ordering** (`sim_commands.rs:192`) stays before load.

### 3.4 Risk
Low logic risk (no algorithm change), but **easy to leave a `&conn` borrow alive**
across the drop and fail to compile (good — the borrow checker enforces
correctness here). The real risk is a *behavioral* one: a concurrent write landing
between (c) and (e). Mitigation: the only writers are other sim/paint commands and
finalize; generation is a user-initiated modal action, so concurrent writes are
unlikely, but the re-acquire at (e) should re-validate the frozen flag and bail
cleanly rather than corrupt.

### 3.5 Impact
The headline UX win: **the map stays interactive during generation.** No compute
speedup by itself, but composes with W1 (free cores during the now-unlocked
compute window).

---

## 4. Workstream W3 — Cut redundant work in `WorldBuffer::save`

### 4.1 Problem (current state, `world_buffer.rs:382-436`)
- `save` **re-fetches every blob from SQLite** (`load_blob_with_version` loop,
  lines 387-395) to snapshot for undo — a full-world DB read on top of the read
  `load_with` already did. For partial-mask phases this doubles read traffic.
- For partial masks (`full == false`), it **decompresses every old blob**
  (`TileData::decompress(&old_states[start+i].2)`, line 424) and **recompresses
  the whole world** (`gather_tile(...).compress()`, line 426) even though only a
  few columns changed.
- `push_undo` (`history/undo.rs`) zstd's the concatenation of every old blob; the
  512MB / 50-entry caps can thrash on large worlds.

### 4.2 Options (pick per-phase, not globally)

**(a) Skip whole-world undo for full sim runs.** A `sim_run_all` result is exactly
reproducible from `(seed, plate_count)`; journaling the entire pre-gen world for
undo buys little and costs a full-world zstd. Add a `save` variant
(`save_no_undo` or a `SaveOpts { undo: bool }`) and use it for the run-all
commands. Single-phase sim steps and paint strokes keep undo.

**(b) Thread original blobs through instead of re-fetching.** `load_with` already
read every blob to build the buffer. Have it optionally retain the
already-compressed source blobs (or their row coords) and hand them to `save` so
the undo snapshot reuses them — eliminating the second full-world DB read. Only
worth it for phases that keep undo.

**(c) Compress only changed tiles for partial masks.** A column mask plus the set
of tiles actually written is known; tiles whose loaded columns are all unchanged
need not be re-gathered/compressed at all. Requires tracking dirty tiles during
compute (a per-tile dirty bit set by the scatter-back), which is a larger change —
defer unless profiling says save dominates.

Recommended first cut: **(a)** for run-alls (smallest, biggest single saving on
the most expensive operation), then **(b)** if the undo re-fetch still shows up.

### 4.3 Validation
- Undo/redo for paint strokes and single sim phases must be byte-identical
  before/after.
- A run-all followed by undo: define and document the new semantics (run-all is
  not undoable to pre-gen state, or is restored from seed) — and assert it in a
  test.

---

## 5. Workstream W4 — Supertile builder copies/compresses only needed columns

### 5.1 Problem
`sample_supertile` (`tile_commands.rs:~265-336`) builds a 128×128 `TileData` by
copying **all ~28 columns + all 45 goods** per output cell, then `compress()`es
the synthetic tile — even though a given layer render needs only 1–4 columns (the
`land` layer needs `terrain/elevation/koppen/sea_depth`). A single paint stroke
invalidates up to 4 pyramid entries, forcing full re-sample on next zoom-out.

### 5.2 Approach
- Pass the **requested layer's required `ColumnSet`** into `sample_supertile` and
  copy only those columns; leave the rest at their `new_sea` defaults. The
  supertile is a *render source*, not a persisted authoritative tile, so omitted
  columns are fine as long as the persisted pyramid entry is keyed by layer (it
  already is: cache keys are `layer|lod|tx,ty`).
- Consider **not persisting** transient supertiles for fast zoom-outs, or
  compressing the derived pyramid at a lower zstd level (e.g. 1) since it is a
  rebuildable cache, not the source of truth.

### 5.3 Risk
Low — derived cache only. Verify each layer declares the exact columns its
renderer reads (a small static map `layer → ColumnSet`), or this silently renders
blank supertiles. Add a test that every render layer's declared column set is a
superset of what its renderer indexes.

---

## 6. Workstream W5 — Frontend overlay & tile churn

### 6.1 Coalesce the trade/matrix/political effects
`MapCanvas.tsx` fires `computeTradeRoutes`, `computeTradeMatrix`, and
`computePolitical` as **three separate effects on overlapping deps**
(`[settlements, rivers, tileVersion, bioParams.*]`), each `JSON.stringify`-ing the
full river set (thousands of points) and re-running least-cost routing. Scrubbing
one `bioParams` slider re-serializes rivers and re-routes up to 3× per change.

Fix options:
- **Debounce** the shared `bioParams` deps (e.g. 150–250ms) so slider scrubs
  collapse to one recompute.
- **Coalesce** into a single backend call that returns routes+matrix+political
  together (they already share the `cost_cache` coarse grid, `db/mod.rs:27`), so
  rivers are serialized and the grid is built once.
- Avoid re-`JSON.stringify(rivers)` per call: memoize the serialized rivers string
  by `tileVersion`/identity.

### 6.2 Throttle `refreshTiles` to RAF on pan/zoom
Pointer-move during pan calls `refreshTiles()` → `loadVisibleTiles` on **every
mousemove** (`MapCanvas.tsx:849-853`); wheel-zoom similarly per tick
(`:219-225`). Coalesce to one call per animation frame (`requestAnimationFrame`
guard) — the single-flight queue already dedupes fetches, this just stops
rebuilding the `needed` set dozens of times per frame.

### 6.3 Replace sort-based LRU eviction
`TileManager.evict()` (`TileManager.ts:268-278`) does
`[...cache.entries()].sort(...)` — O(n log n) over ≤2000 entries on every
overflow, firing repeatedly during a streaming pan. JS `Map` preserves insertion
order: on access, `delete`+`set` to move-to-front; evict by taking the first
`keys().next()` entry — O(1) amortized. Mechanical, low-risk.

### 6.4 Minor
`TileManager.invalidate()` (`:248-257`) splits every key string on each paint
stroke; bounded (≤2000) so low priority. Fold in if touching the file anyway.

---

## 7. Benchmarking & validation harness (build this first)

Before any optimization, stand up a repeatable measurement so each workstream
reports a real number:

- **Reference world:** a fixed `(seed, plate_count, grid size)` — pick one mid
  (e.g. 2048×1024) and one large (e.g. 7200×3600) for memory-bound effects.
- **Backend timing:** wrap each phase in `sim_run_all` with `std::time::Instant`
  behind a `WF_PROFILE` env flag (or a `tracing` span), printing per-phase ms.
  This both guides W1 (which passes dominate) and proves the W1/W2/W3 wins.
- **Determinism gate:** a test that hashes the full post-`sim_run_all`
  `WorldBuffer` (all columns) for the reference seed and asserts it is unchanged
  across the whole workstream. This is the safety net for W1/W3.
- **Frontend:** measure `bioParams`-scrub recompute count and pan FPS before/after
  W5 (manual or a lightweight perf marker).

Document the before/after table in this file as each workstream lands.

---

## 8. Risk register

| Risk | Workstream | Likelihood | Mitigation |
|---|---|---|---|
| FP reduction reorder changes output | W1 | Med | Only parallelize per-cell maps; isolate global reductions; bitwise-equal tests |
| Concurrent write between load & save | W2 | Low | Re-validate frozen flag at re-acquire; user-modal generation |
| Leftover `&conn` borrow across drop | W2 | Low | Compile error catches it; mechanical |
| Undo semantics change surprises users | W3 | Med | Document run-all-not-undoable; keep undo for strokes/single phases |
| Supertile renders blank (missing column) | W4 | Low | Static `layer→ColumnSet` map + superset test |
| Debounce hides a needed recompute | W5 | Low | Trailing-edge debounce; recompute on settle |

---

## 9. Sequencing (recommended landing order)

1. **W7-harness** (§7) — profiling flag + determinism hash test. *No risk, enables
   everything.*
2. **W2** — release DB lock. *Small, high UX payoff, no output change.*
3. **W1 (class A)** — parallelize per-cell passes incrementally, one per PR with
   its determinism test. *Largest compute win.*
4. **W5** — frontend coalesce + RAF throttle + Map-LRU. *Independent, ship anytime.*
5. **W3 (option a)** — skip undo for run-alls. *Cuts the biggest single I/O cost.*
6. **W4** — supertile column trim. *Polish; do when touching the pyramid.*
7. **W1 (class B)** — stencil/advection passes (double-buffered). *Stretch.*

Each item is independently shippable and independently revertible. None depends on
the economic-realism workstream.

---

## 10. Explicitly out of scope (here)

- Economic-model realism (depletion, balance-of-payments, tolls-in-pricing,
  population ceiling) — separate plan.
- Replacing SQLite, changing the blob format, or GPU rendering — large rewrites,
  not warranted by current profiles.
- Parallelizing class-C sequential passes (D8 flow, flood-fills, Voronoi growth)
  — deferred until class A/B are exhausted and profiling justifies the
  determinism risk.
