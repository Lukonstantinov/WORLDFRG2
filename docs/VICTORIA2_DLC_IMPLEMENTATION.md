# Victoria 2 Layer — DLC-by-DLC Implementation Plan (Phase B)

Engineering-concrete plan for turning the campaign sim into a Victoria 2-style
grand-strategy game. Builds strictly on the existing economy (`sim/tick.rs`,
`sim/market.rs`), append-only (`#[serde(default)]`) so old campaigns keep loading.

**Gate for every DLC milestone (standing rule):** after any `tick.rs` change, run
`cargo test --lib simulate_decades_reports_dynamics -- --nocapture`, read the
5-yearly digest, confirm wealth stays bounded + turnover happens, and tune
constants until the dynamics read healthy. Visual additions ship an HTML/SVG
report in `docs/mockups/`.

---

## DLC 4 — Nations & POPs

**Goal:** abstract `Society` shares → typed `Pop` objects; group hubs into
Provinces→States→Countries; read-only Country/Population dashboard.

| Step | What | Files | Status |
|---|---|---|---|
| 4.1 | `Pop` data model (`profession/size/money/needs/consciousness/militancy`) + per-hub derivation from `Society` each year (read-only) + `campaign_get_pops` IPC | `sim/tick.rs`, `commands/campaign_commands.rs`, `lib.rs`, `types.ts`, `bridge/tauri.ts` | ✅ **done** |
| 4.2 | **Population panel UI** — pop list (proportional bars) + a pop card (needs meters, militancy) reading `campaign_get_pops` | `ui/PopulationPanel.tsx` (new), `App.tsx`, `uiStore` | ⏳ |
| 4.3 | **Country grouping** — derive `Country { id, name, hubs[], capital, gov_type, treasury }` from hub `component` + dominant culture; `campaign_get_countries` IPC; political map-mode tint | `sim/tick.rs`, `commands/*`, `OverlayManager.ts` | ⏳ |
| 4.4 | **Pop-driven demand (wire-in)** — pops' `money` budgets their needs purchases through `market.rs`; replace the per-capita extractor's implicit demand with summed pop demand. *This is the risky economy change — do last, dynamics-test hard.* | `sim/tick.rs`, `sim/market.rs` | ⏳ |

Exit: a world of nations you can inspect, pops visibly promoting/demoting with
prosperity, demand emerging from pop budgets.

## DLC 5 — Politics & Reform

**Goal:** governments, parties, reforms, militancy/consciousness feedback, rebellions.

| Step | What | Files |
|---|---|---|
| 5.1 | `Party { ideology, issues }` + `Country.gov_type`, upper-house support computed from pop ideology distribution | `sim/tick.rs` |
| 5.2 | Pop **consciousness/militancy** dynamics (needs-unmet + reforms + war exhaustion drive them); reuse existing `unrest`/revolt machinery for rebellions | `sim/tick.rs` |
| 5.3 | **Reforms** (political + social) gated by upper-house weight + consciousness; enacting shifts needs/promotion/militancy | `sim/tick.rs` |
| 5.4 | **Politics panel** — upper-house bars + reform rows (mockup exists: `docs/mockups/victoria2-redesign.html`) | `ui/CountryPanel.tsx` |

## DLC 6 — Player Agency

**Goal:** pick a country; the player sets policy, AI runs the rest. *This is the
moment it becomes a game.*

| Step | What | Files |
|---|---|---|
| 6.1 | `player_country` on the campaign; AI uses `decide_polis_policy` brain, player reads sliders from UI | `sim/tick.rs`, `commands/*` |
| 6.2 | **Budget sliders** (poor/mid/rich tax, tariff, mil/admin/edu/social spend) → wired to `CityFinance` | `ui/CountryPanel.tsx`, `commands/*` |
| 6.3 | Build orders (found estate/manufactory), research queue stubs, enact reforms from UI | UI + commands |
| 6.4 | New-game flow: choose country + start | `App.tsx` |

## DLC 7 — Technology & Industrialisation

| Step | What | Files |
|---|---|---|
| 7.1 | Tech tree (army/navy/commerce/culture/industry) replacing the single `tech_factor`; research points from clerks/clergy + edu spend | `sim/tick.rs` |
| 7.2 | Inventions (random unlocks) shifting production/literacy | `sim/tick.rs` |
| 7.3 | Factory economy depth: employment, wages → pop money, capitalists build/expand factories, subsidies | `sim/tick.rs`, `sim/manufacture.rs` |
| 7.4 | Technology + Production panels | UI |

## DLC 8 — Diplomacy, Great Powers & War

| Step | What | Files |
|---|---|---|
| 8.1 | Country scores (prestige + industry + military) → **Great Power ranking** | `sim/tick.rs` |
| 8.2 | Spheres of influence, alliances, relations (extend existing economic `wars`) | `sim/tick.rs` |
| 8.3 | Casus belli + war goals; reparations/annexation resolve via the existing war machinery | `sim/tick.rs` |
| 8.4 | (Optional) army/navy units mobilised from soldier pops; battles | `sim/tick.rs` |
| 8.5 | Diplomacy + Military panels, GP scoreboard | UI |

---

## Sequencing & risk
- **Safe-first within each DLC:** land data models + read-only derivations + UI
  before any change to the economy loop. The one genuinely risky step per DLC is
  the loop wire-in (4.4 demand, 7.3 wages) — do those last, behind the dynamics test.
- **One PR per step** (or small group), each green on `tsc` + `cargo check`
  (+ dynamics test for `tick.rs`), each with a report.
- Estimated scope: DLC 4 ≈ 5–8 PRs, DLC 5–8 each ≈ 6–12 PRs. This is a months-long
  build; it cannot be a single change.
