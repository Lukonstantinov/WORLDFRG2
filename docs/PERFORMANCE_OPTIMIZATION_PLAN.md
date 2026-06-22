# Campaign Performance & Stability Plan

The campaign tick (`src-tauri/src/sim/tick.rs`) is the heart of WF2 and its main
cost centre. This plan covers (Tier 0) stopping the hard crashes, (Tier 1) making
the sim fast and non-blocking, and (Tier 2) promoting it to a **dedicated
background engine** that runs isolated from the UI and the DB — same app, separate
thread/crate.

> Standing rule reminder: after ANY change to `tick.rs` run the dynamics digest and
> read it (`cargo test --lib simulate_decades_reports_dynamics -- --nocapture`).
> Each tier below lists its own verification on top of that.

---

## 0. Evidence — what we actually know

### The crash (dump `worldforge2.exe.26308.dmp`, 2026-06-22 23:53)

Parsed from the Windows minidump exception + misc streams:

| Field | Value | Reading |
|---|---|---|
| Exception code | `0xC0000409`, param[0] = `7` | `__fastfail(FAST_FAIL_FATAL_APP_EXIT)` — **Rust `abort()`** |
| Faulting module | `worldforge2.exe` (`base+0x12D6446`) | abort came from **our own code**, not GPU/WebView2 |
| User-mode CPU at death | **1745 s (~29 min)** | the sim was grinding very hard before it died |
| Kernel CPU | 61 s | not I/O bound — it's compute |
| Recovered "panicked at" text | none (partial dump) | the panic message, if any, went to stderr, not the dump |

`Cargo.toml` does **not** set `panic = "abort"`, so an ordinary panic should unwind.
The observed `__fastfail` therefore means one of:

1. **OOM** — an allocation failed and Rust called `abort()` (no panic message). The
   prime suspects are large allocations on a *growing* campaign: the full-sim →
   JSON autosave string, and the per-tick `needs`/`prod_mult` matrices.
2. **A debug-build panic crossing the FFI boundary** — `tauri dev` builds keep
   `debug-assertions` + `overflow-checks` **on** (not overridden in `[profile.dev]`),
   so an index-out-of-bounds or arithmetic overflow in a tick panics, and when that
   unwinds across Tauri's C/IPC frame it is converted to the same `__fastfail` abort.

Both root causes are **inside the campaign sim**, and both are invisible to the user
today (the app just vanishes). Tier 0 makes either one a recoverable, *named* error.

### Where the CPU goes

- **Route rebuilds are O(n²) on a *growing* `n`.** `rebuild_routes` (tick.rs:2806)
  fills an `n×n` `days` matrix and `rebuild_neighbors` (tick.rs:2835) scans all
  `n` candidates per hub. `n` is **not fixed**: every estate and colony does
  `self.hubs.push(...)` (tick.rs:5587, 6218), so a long campaign with many houses
  founding estates/offices/colonies pushes `n` from ~150 into the thousands. O(n²)
  on a climbing `n` is exactly the "smooth early, grinds to a halt late" profile
  that burned 29 min of CPU. Rebuilds are already batched to ≤1/tick via
  `routes_dirty`, which helps but doesn't change the asymptotics.
- **Dispatch is already capped.** `NEIGHBOR_K = 32` (tick.rs:577) means `dispatch`
  (tick.rs:4610) ships only to the K nearest partners → ~O(n·K·ng), not O(n²·ng).
  This part is good; don't regress it.
- **My 2026-06-22 trade fix raised the floor (correctly).** Unifying interior
  cities into one component means more lanes now genuinely trade, so dispatch does
  more *real* work per tick. Right behaviour, higher cost — accounted for here.

### What is already done well (do not "fix")

- `days` and `neighbors` are `#[serde(skip)]` (tick.rs:1612, 1619) and rebuilt on
  load (`advance` checks `self.days.len() != n*n`) — the n² matrix is **not** in the
  save. `flow_accum` is `#[serde(skip)]` too.
- Per-house `wealth_history` (cap 80), `events` (cap 60), `journal`
  (milestone-retain + drain), `bank.history` (cap 60), `trade_hist` (cap 40),
  `active_events` (retain) are all bounded.
- There is an opt-in profiler: set `WF2_PROFILE=1` to get a per-in-game-year ms
  breakdown of rebuild/trade/events/houses (tick.rs:3206+).

### Remaining unbounded / unscoped growth

- `crashes` (Vec<CrashRecord>) and `war_log` (Vec<WarRecord>) have **no cap**
  (low frequency, so a slow leak rather than the crash driver — but cap them).
- `n` itself (estate/colony hubs) grows without an upper bound → the real scaling
  problem, addressed in Tier 1.
- The autosave serializes the whole resident sim to one JSON string every ~5 s /
  on year-rollover (`campaign_advance`, campaign_commands.rs:1238). Cost grows with
  state; it's a plausible OOM allocation site on huge late campaigns.

---

## Tier 0 — Stop the hard crash (small, ship first)

Goal: a bad tick or a failed allocation becomes a **recoverable error with a
message**, never a silent process abort.

1. **Crash-guard `advance`.** In `campaign_advance` (campaign_commands.rs:1233) wrap
   the `sim.advance(...)` call in `std::panic::catch_unwind(AssertUnwindSafe(...))`.
   On `Err`, do **not** persist (the sim may be half-mutated), surface
   `Err("campaign tick panicked: <payload>")` to the frontend, and let the UI show a
   banner + offer to reload the last autosave. This alone converts last night's
   abort into a named, survivable error and tells us the panicking line.
   - Pair with a one-time `std::panic::set_hook` at startup that logs
     `location + message` to a file under the app data dir, so the panic text is
     captured even though `tauri dev` swallows stderr.
2. **Bound the batch.** `advance` already clamps ticks to `[1, 3650]`. Keep "year"
   speed (365) as the max single call; never let the UI request a decade in one
   blocking call.
3. **Cap the stragglers.** Add `CRASH_RECORD_CAP` / `WAR_LOG_CAP` drains mirroring
   the journal pattern.
4. **(If OOM is confirmed) shrink the autosave.** Serialize on year-rollover only
   (drop the 5 s wall-clock trigger) and/or persist with a compact binary
   (`bincode`/`postcard`) instead of `serde_json::to_string`. Keeps a giant
   transient `String` from being allocated every few seconds.

**Verify:** force a panic in a tick (temporary `panic!`), confirm the UI shows the
error and the app stays alive; confirm the panic log file is written. Run the
dynamics digest (unchanged numbers). 28+ tick tests stay green.

**Risk:** very low. No format change except the new caps (serde-default; old saves
load). `catch_unwind` requires the closure be `UnwindSafe` — `AssertUnwindSafe`
around the `&mut sim` is fine because on panic we discard the sim without reading it.

---

## Tier 1 — Fast & non-blocking (medium)

Goal: the UI never freezes during a long batch, and late-campaign cost stops
growing quadratically.

### 1a. Get the sim off the UI thread

Today `campaign_advance` is a **blocking** command holding **both** DB mutexes
(`conn` + `campaign`) for the entire compute, so every tile fetch / panel query
stalls behind it — the "frozen → force-close" path that also yields WER hang-dumps.

- Make `campaign_advance` `async` and run the compute on `tauri::async_runtime::
  spawn_blocking` (or a dedicated worker thread). Compute on an owned snapshot;
  take the `campaign` lock only to swap the result back in. **Do not hold `conn`
  during compute** — only re-acquire it for the (now less frequent) persist.
- This is the stepping-stone to Tier 2: the sim already wants to own its state and
  hand back snapshots.

### 1b. Kill the O(n²)-on-growing-`n` cost

- **Incremental route updates.** When a single estate/colony hub is appended, don't
  rebuild the whole `n×n` matrix — append one row/column (its distances to existing
  hubs, O(n)) and insert it into affected neighbour lists. Full rebuild only on
  structural resets (load, or a rare compaction).
- **Treat estates as satellites, not routing nodes.** An estate imports food from
  exactly one parent hub; it does not need a global routing row. Keep estates out
  of `days`/`neighbors` and resolve their supply against their parent directly.
  This caps the routing `n` near the number of *real settlements*, which is roughly
  constant, and removes the dominant growth term.
- **Cap or pool runaway hub growth.** If satellites are still modelled as hubs,
  enforce a per-parent estate cap and/or merge co-located estates so `n` can't
  climb without bound.

### 1c. Cheap constant-factor wins (guided by `WF2_PROFILE`)

- Hoist per-tick allocations out of the loop: `needs` (`vec![vec![...]; n]`,
  tick.rs:3325) and `prod_mult` reallocate every tick — keep reusable scratch
  buffers on the sim and clear them instead.
- Profile first; only optimize the phases the profiler shows are hot. Do not
  hand-optimize blind.

**Verify:** `WF2_PROFILE=1` before/after, capture the per-year ms breakdown into the
HTML report (Tier rule: visual/quantified change → `docs/mockups/*.html`,
before/after). Run a long campaign (e.g. 300 in-game years) and confirm per-year ms
stays roughly flat instead of climbing. Dynamics digest unchanged. UI stays
responsive (tile pan works) while "Play" runs.

**Risk:** medium. The satellite/estate change touches dispatch + supply — guard it
behind the dynamics test (estates must still import food, houses still turn over).

---

## Tier 2 — Dedicate the engine (background engine, same app)

Chosen architecture: **the sim runs on its own dedicated thread inside WF2**,
isolated from the UI and the DB mutex, talking over channels. This delivers the
"dedicated campaign" benefits — UI never freezes, the engine is crash-isolated and
recoverable, and it can be driven faster/headless for testing — **without** forking
the codebase into a second app (which would duplicate the world-load + IPC + render
pipeline and keep all the same internal perf issues).

### Shape

```
            commands (Cmd: Advance{ticks}, SetSpeed, Pause, Save, Load)
  UI  ──────────────────────────────────────────────►  CampaignEngine thread
 (Tauri          ◄──────────────────────────────────────  (owns CampaignSim)
  commands)        events (Snapshot, YearRolled, Error{panic}, Persisted)
```

- A `CampaignEngine` owns the `CampaignSim` and runs its own loop on a dedicated
  `std::thread` (or `tauri::async_runtime` task). Commands arrive on an `mpsc`/
  channel; snapshots + events are pushed back (Tauri `emit` to the frontend, or a
  watch channel the existing `campaign_get_state` reads).
- Tauri commands become thin: `campaign_advance` sends `Cmd::Advance` and returns
  immediately; the frontend re-renders from emitted `Snapshot` events instead of the
  command's return value. The `StepCampaign` auto-play loop becomes
  "set speed = month, subscribe" rather than an `await advance()` ping-pong.
- **Crash isolation:** the engine loop wraps each batch in `catch_unwind` (Tier 0
  logic, now living in the engine). A panic emits `Error{...}`, parks the engine,
  and the UI offers reload — the *engine* is restartable without taking the window
  down.
- **Persistence stays where it is:** the engine, not the UI, owns the autosave
  cadence and writes through the existing `metadata::campaign_set` path. The DB
  mutex is touched only at persist time, never during compute.

### Optional internal split (not a second app)

Extract the sim into its own crate `wf2-campaign` (pure compute: `CampaignSim` +
tick logic + tests, no Tauri/DB deps). `src-tauri` depends on it and provides the
engine thread + IPC glue. Benefits: the dynamics test + a future headless
`cargo run -p wf2-campaign --bin replay` can run the sim with **zero** GUI/DB build
cost (much faster iteration), and the boundary forces the sim to stay free of UI
concerns. This is the "dedicated" win done as a module boundary, not a product fork.

**Verify:** UI fully interactive during a multi-decade Play; killing/parking the
engine on a forced panic leaves the window alive and reloadable; snapshots arrive as
events and the panels update without per-tick `await`. Full dynamics digest + tick
tests green from the (possibly extracted) crate.

**Risk:** higher (touches the command/IPC contract + `campaignStore`/`StepCampaign`
data flow). Do it only after Tier 0+1 land, and keep the synchronous
`campaign_get_state` path working for back-compat during the transition.

---

## Sequencing & exit criteria

| Tier | Effort | Ship when |
|---|---|---|
| 0 | small | a panicking/OOM tick shows a recoverable error + writes a panic log; caps added |
| 1 | medium | UI stays responsive during Play; per-year ms flat over 300 years; estates off the routing grid |
| 2 | larger | sim runs on a dedicated engine thread/crate; UI is event-driven; engine crash-recoverable |

Across all tiers: **save-format back-compat** (new fields serde-default), and the
**dynamics digest must stay healthy** (bounded finite wealth, houses turn over,
banks/coins/wars/crashes still occur) — performance work must not flatten the living
economy.
