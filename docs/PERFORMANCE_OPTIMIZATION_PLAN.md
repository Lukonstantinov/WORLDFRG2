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

## Tier 0 — Stop the hard crash (small, ship first)  ✅ DONE

Goal: a bad tick or a failed allocation becomes a **recoverable error with a
message**, never a silent process abort.

> **Implemented** (cargo check + 30 tick tests green): `catch_unwind` crash-guard
> in `campaign_advance`; startup panic hook → `app_log_dir/panic.log`;
> `CRASH_RECORD_CAP`/`WAR_LOG_CAP` caps. The "shrink the autosave" item (4) is left
> for later — frequent autosave is what the crash-guard rolls back to, so reducing
> its cadence would just lose more progress; revisit only if OOM is confirmed.

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

## Tier 1 — Fast & non-blocking (medium)  ◑ PARTIALLY DONE

Goal: the UI never freezes during a long batch, and late-campaign cost stops
growing quadratically.

### 1a. Get the sim off the UI thread  ✅ DONE

`campaign_advance` was a **synchronous** command — in Tauri 2 those run on the
**main thread**, so the whole advance blocked the UI event loop (the "frozen →
force-close" path that also yields WER hang-dumps).

> **Implemented:** `campaign_advance` is now `async` (Tauri runs it on a worker
> thread, not the main thread), and it is restructured into three tight lock
> phases — (1) ensure-loaded under conn+campaign, (2) **compute holding ONLY the
> `campaign` lock** so conn-dependent tile rendering stays responsive, (3) persist
> under conn+campaign. The documented conn-before-campaign lock order is preserved
> (phase 2 takes a single lock). No `.await` inside → the future stays `Send`.

### 1b. Kill the O(n²)-on-growing-`n` cost  ⏳ NOT DONE (revised — see caveat)

- **Incremental route updates — blocked by storage layout.** `days` is a flat
  `Vec<f32>` with stride `n` (`days[a*n+b]`), so growing `n` re-lays-out the whole
  matrix; a true O(n) append needs the storage changed to jagged `Vec<Vec<f32>>`
  (or a fixed-capacity arena), which touches every `days[…]` access site. Deferred
  as a focused refactor — and lower priority than it looks, because the rebuild is
  already batched to ≤1/tick and only fires on ticks that add a hub. The dominant
  cost is per-tick dispatch across all ticks, not the occasional rebuild.
- **Treat estates as satellites, not routing nodes.** An estate imports food from
  exactly one parent hub; it does not need a global routing row. Keep estates out
  of `days`/`neighbors` and resolve their supply against the parent directly →
  caps routing `n` near the (roughly constant) real-settlement count. This is the
  highest-leverage structural win but it **changes economic semantics** (how
  estates are supplied), so it must be done WITH in-app verification + the dynamics
  digest, not blind. Deferred deliberately.
- **Cap/pool runaway hub growth** — per-parent estate cap and/or merge co-located
  estates so `n` can't climb without bound.

### 1c. Cheap constant-factor wins (guided by `WF2_PROFILE`)  ◑ STARTED

> **Implemented:** the per-tick `needs` matrix (`n×ng` floats) is no longer
> reallocated every tick — a reusable buffer is hoisted above the tick loop and
> resized/cleared in place (behavior-identical; dynamics digest unchanged).
> Remaining: `prod_mult` (`event_production_mult`) still allocates per tick — cheap
> to pool when it shows up in the profiler.

**Verify (for the remaining 1b/1c work):** `WF2_PROFILE=1` before/after, capture the
per-year ms breakdown into an HTML report (Tier rule: quantified change →
`docs/mockups/*.html`, before/after). Run ~300 in-game years and confirm per-year ms
stays roughly flat. Dynamics digest unchanged. UI stays responsive (tile pan works)
while "Play" runs.

**Risk:** the satellite/estate change touches dispatch + supply — guard it behind
the dynamics test (estates must still import food, houses still turn over).

---

## Tier 2 — Dedicate the engine (background engine, same app)  ⏳ NOT DONE (needs in-app verification)

> Note: Tier 1a already delivers Tier 2's **primary** user-facing benefit — the sim
> runs off the UI thread, so the window no longer freezes during a long batch. What
> remains below (the channel-based actor + event-driven UI) is a refinement that
> rewrites the `campaignStore` / `StepCampaign` data flow, and that CANNOT be
> validated headlessly — it needs the app running. It is deliberately left for a
> session where the change can be exercised in-app, rather than shipped blind.

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

| Tier | Effort | Ship when | Status |
|---|---|---|---|
| 0 | small | a panicking tick shows a recoverable error + writes a panic log; caps added | ✅ done |
| 1a | small | advance off the main thread; conn free during compute | ✅ done |
| 1c | small | per-tick `needs` matrix no longer reallocated | ◑ partial (`prod_mult` left) |
| 1b | medium | estates off the routing grid; per-year ms flat over 300 years | ⏳ deferred (semantics + GUI verify) |
| 2 | larger | actor engine + event-driven UI; engine crash-recoverable | ⏳ deferred (needs in-app verify) |

Across all tiers: **save-format back-compat** (new fields serde-default), and the
**dynamics digest must stay healthy** (bounded finite wealth, houses turn over,
banks/coins/wars/crashes still occur) — performance work must not flatten the living
economy.
