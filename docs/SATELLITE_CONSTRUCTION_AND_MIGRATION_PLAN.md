# Satellite Construction + Route-Bound Migration — Implementation Plan

Design locked with the user (2026-07-05). Mockup:
`docs/mockups/satellite-construction-and-migration.html`.

## Locked decisions

**Satellite construction**
- **Window:** Blend V1+V3 — Ledger top (5-step stage bar · monthly-cost sparkline ·
  council fund + runway) with the full convoy **manifest** living inside the supply tabs.
- **Duration:** ~10 years, with **decay** — a funding/supply gap slowly loses % on the
  current stage; a *long* gap drops a whole stage back.
- **Trade binding:** **Permanent** — a finished satellite always routes its trade through
  the metropolis first; only true independence (the existing age/prosperity path) frees it.
- **Cost model:** **Real goods + treasury** — convoys physically pull food / preservables /
  construction goods OUT of the metropolis stock; the council treasury pays convoy upkeep.
  A shortage of either (goods or gold) slows/decays the build.
- **Supply picking:** **Auto by locale** — the sim picks the cheapest available good in each
  of the 3 categories from {metropolis stock, local site belts}; shown in the tabs; it
  changes with what's tradeable there (coast→salt fish, forest→timber, delta→grain…).
- **Events:** **Both** — existing hazards (storm/piracy/plague/war on a supply leg) delay
  convoys, PLUS new construction events (masons hired → faster; collapse/flood → setback;
  patron endowment → funds).
- Council-funded only (no house/guild needed) — already true of `maybe_found_satellite`.

**Migration rework**
- **Routing:** **Strict multi-hop** — a flow exists only if an unbroken trade-route chain
  connects origin→destination; drawn along the **actual routed polyline** (coarse-grid path,
  same geometry as trade trunks). No route ⇒ no migration.
- **Visual:** **All three, toggle** under the Migration overlay: `dots | ribbon | focus`
  (flowing dots · culture ribbons width∝volume · click-a-city → isolate its inbound routes
  + origins/culture list).

## Data model (backend, `sim/tick.rs`)

New `TickHub` fields (all `#[serde(default)]`; add to the 4 literals + campaign builder):
```
build_stage: u8            // 0 = finished/not-a-project; 1..=5 = current stage
build_progress: f32        // 0..1 within the current stage
build_supply: [f32;3]      // this-month delivered vs quota, per category (food/presv/constr)
build_supply_good: [u16;3] // chosen good id per category (auto by locale)
build_idle_months: u8      // consecutive under-funded months (drives decay / stage drop)
build_convoys: u8          // dedicated caravans+ships count
build_start_tick: u32
```
Category enum index: `0 food · 1 preservables · 2 construction`.
`SatelliteProject` summary struct for the snapshot (stage, %, ETA, cost/mo, fund, runway,
per-tab {good, source, rate, met%}, convoy manifest rows, future-exploit good ids, event).

Constants: `SAT_BUILD_YEARS=10`, `SAT_STAGE_QUOTA`, `SAT_CONVOY_UPKEEP`,
`SAT_DECAY_PER_IDLE_MONTH`, `SAT_STAGE_DROP_IDLE_MONTHS`, event odds.

## Backend phases

1. **State + founding.** Add fields. `maybe_found_satellite` now creates a *project*
   (build_stage=1, pick 3 supply goods by locale, allocate convoys) instead of an instant
   town. The hub exists (small seed pop) but is flagged "under construction".
2. **`construction_pass` (monthly hook in `advance`).** For each in-build satellite:
   pull the 3 supply goods from metropolis stock (capped by availability), pay convoy upkeep
   from council treasury, compute `met%` = min across categories, advance `build_progress`
   by `met%/(stage_len)`; on shortfall raise `build_idle_months` and **decay** progress; on
   long idle **drop a stage**. Advance stage on 100%. Roll construction events (both kinds).
3. **Completion + binding.** At stage 5 done → `build_stage=0`, becomes a functional bound
   city; set a permanent `trade_bound_to = metropolis` flag; activate the site's exploit goods.
4. **Trade binding in dispatch.** A bound satellite's exports are forced through the
   metropolis (route its surplus to the mother hub first; mother market absorbs it). Reuse
   the existing colony-supply / office machinery where possible.
5. **Migration rework.** Replace the arc emitter: `economic_migration_pass` +
   `diaspora_pass` + `poleis_sponsor_migration` route strictly over `neighbors`/coarse graph
   (multi-hop, no-route→skip). Emit the routed polyline (list of hub hops) not just endpoints.
   New `migration_routes` accumulator: `{from,to, culture, volume, path:[hub ids]}`.

## Frontend phases

6. **Types + bridge.** `SatelliteProject`, migration-route payload; `campaign_get_satellite(id)`.
7. **`SatelliteConstructionPanel.tsx`** (Blend V1+V3) — opened when a clicked hub has
   `build_stage>0`; auto-swaps to the normal HubPanel on completion.
8. **Migration overlay rework** in `OverlayManager` — 3 toggles; draw ribbons/dots along the
   routed polylines; click-city focus isolates inbound routes + origins list.

## Standing-rule checkpoints
After every `tick.rs` change: `cargo test --lib simulate_decades_reports_dynamics -- --nocapture`
and read the digest (bounded wealth, turnover). HTML mockup already produced.
