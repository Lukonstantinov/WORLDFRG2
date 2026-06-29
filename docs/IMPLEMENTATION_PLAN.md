# Redesign — Implementation Plan & Status

Living tracker for the redesign work. Tracks what is **done in the app**,
what is **design-only**, and the ordered plan for what remains.

## ✅ Implemented (merged to `main`)

| Item | File(s) | PR |
|---|---|---|
| Flat goods-icon medallions (parchment chip + coloured ring/pictogram) | `src/canvas/goodIcons.ts` | #19 |
| Settlement tier markers (gold-star capital / ring city / disc town / square outpost) | `src/canvas/OverlayManager.ts` | #19 |
| Vector coat-of-arms (16 charges, 5 shapes, ermine/vair furs, 11 divisions, 10 ordinaries) | `src/ui/CoatOfArms.tsx` | #19 |
| Expanded house surnames + guild words + guild name pattern | `src-tauri/src/sim/cultures.rs` | #19 |
| Trade-route line **styles** by kind (sea rhumb dashes · river even dash · land dotted caravan) | `src/canvas/OverlayManager.ts` | #20 |
| Icon **coverage**: all 85 goods now draw a symbol (was ~35 blank dots) | `src/canvas/goodIcons.ts` | #21 |
| **DLC 4.1 — POP data model** (typed `Pop` derived from `Society` each year, read-only) + `campaign_get_pops` IPC | `sim/tick.rs`, `commands/campaign_commands.rs`, `lib.rs`, `types.ts`, `bridge/tauri.ts` | this PR |

Verification gates for every change: `npx tsc --noEmit` and (for Rust) `cargo check`.
For `tick.rs` economy changes additionally run
`cargo test --lib simulate_decades_reports_dynamics -- --nocapture`.

## 🔜 Phase A — finish the visual layer (small, contained)

1. **Per-good pictograms** (`goodIcons.ts`).
   - ✅ *Batch 1 (done):* every good now maps to a thematic symbol — the ~35
     goods that fell back to a plain dot (gems, rice/barley/herring, manufactured
     goods) now render an icon; +`book`/`candle` symbols added. See
     `docs/mockups/goods-icon-coverage.svg`.
   - ⏳ *Follow-up:* give the highest-traffic goods **bespoke distinct** art (the
     2-tone pictograms in `docs/mockups/goods-iconography-redesign.svg`) instead
     of reusing shared symbols. Ship in batches by category.
2. **Parchment / aged base map** (`src-tauri/src/render/tile_image.rs`). Tint the
   land/sea ramps toward the aged palette + a subtle paper grain. *Server-side
   render change; needs the dynamics/visual report. Medium risk — gate behind a
   layer toggle first.*
3. **Compass rose + graticule overlay** (`OverlayManager.ts`) as an opt-in
   decoration toggle (already prototyped in the SVG mockups). *Small.*

## 🧭 Phase B — Victoria 2 layer (large, staged — see VICTORIA2_REDESIGN_PROPOSAL.md)

Build on the existing campaign sim (`sim/tick.rs`), append-only, one DLC at a time.
Each milestone ends with the standing dynamics test green + an HTML report.

| DLC | Scope | Key files |
|---|---|---|
| **4 — Nations & POPs** | Province→State→Country layer; convert `Society` shares → `Pop` objects buying needs via `market.rs`; read-only Country dashboard | `sim/tick.rs`, `sim/market.rs`, new `ui/CountryPanel.tsx` |
| **5 — Politics & Reform** | Government types, parties/ideologies, upper house, militancy/consciousness, reforms, rebellions | `sim/tick.rs`, `ui/CountryPanel.tsx` |
| **6 — Player Agency** | Pick a country; budget sliders, tax/tariff, build, research; AI runs the rest | commands + UI |
| **7 — Technology & Industrialisation** | Real tech tree + inventions; factory/capitalist depth | `sim/tick.rs` |
| **8 — Diplomacy, Great Powers & War** | GP ranking, spheres, CBs/war goals, optional military | `sim/tick.rs` |

### Recommended order
Phase A.1 (per-good art) and A.3 (compass toggle) are quick visual wins and
safe. A.2 (parchment base) next. Then start **DLC 4** as the spike that proves
the Victoria layer on top of the working economy.

## Constraints / how work is verified here
- The live Tauri window can't launch on a headless box — only `tsc` and
  `cargo check`/tests run. Rendered results are shown via `docs/mockups/*`.
- Visual changes ship with a self-contained HTML/SVG report in `docs/mockups/`
  (project standing rule). Economy changes ship with the dynamics test output.
