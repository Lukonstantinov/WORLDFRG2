# 10 · Settlement overview

**Status:** PARTIAL (2026-09-26) — 10.1/10.2 shipped; 10.3/10.4 not built ·
**Depends on:** 03–08 · **Next:** —

**Shipped.** `ui/campaign/SettlementOverviewTab.tsx`, a new "Overview" tab
(now the FIRST/default tab on selecting a city) in `HubPanel.tsx`, showing
government (form/legitimacy/seats/debate in progress), ideology (dominant
doctrine + scholar count), culture tiers, a venues/masterworks count, and
the last 5 chronicle lines.

**Scope cut, recorded rather than silently made (rule 36):** 10.1 calls for
"one read query assembling the Overview (no new sim state)". This is built
as a CLIENT-SIDE composition of rows 04/06/07/08's own already-shipped read
commands instead — each of those `#[tauri::command]` functions takes its
own `db: State<'_, WorldDb>` and locks the SAME `std::sync::Mutex`
internally, so calling one FROM inside a new aggregator command would
deadlock on that lock. Composing independent `invoke()` calls client-side
avoids inventing a second, parallel read path for data these commands
already serve correctly, and needed zero new Rust code. Development-track
levels, identity (population/culture mix/portrait), war/realm/horde-threat
and the economy-at-a-glance blocks the doc's own table also asks for are
NOT in this cut — most of that data already has its OWN reader elsewhere
in `HubPanel.tsx` (the existing "summary"/"development"/"city" tabs), and
folding it into Overview too is real, low-risk follow-up work, not a
blocker; row 09's war/horde-threat block waits on that row's own Parts B/C
existing to have anything to show.

Gate: `cargo check --lib --tests` (unchanged, no Rust touched), `npx tsc
--noEmit` + `npx vite build` (191 modules) both clean. No `econ_` run
owed — no sim state changed (CLAUDE.md §2.8's own routing-table row for a
frontend-only change).

## Goal

With rows 02–08 in place a settlement holds far more than its current panel was
built for. The first thing the maintainer sees when opening a city should be an
**Overview** that shows everything important at once, with each block linking to
its detailed window (as Market already is).

## Decisions (maintainer)

- The **Overview is the first panel** with the main information; details stay in
  **separate windows** (Market, Government, Society, Venues…).
- Build the mechanics first, **rework the view afterwards** — this row comes last
  on purpose.

## Content of the Overview

| Block | From | Links to |
|---|---|---|
| Identity: name, portrait of the city (buildings visible — walls, aqueduct, library, venue), population, culture mix | existing + row 03 buildings + `buildingArt.ts` | Society |
| Development factor: value, this year's growth, 50-year sparkline, "why" | row 03 | Development |
| Tracks: Military · Trade · Civil · Ideological levels and buildings | row 03 | Development |
| Government: form, ruler or presiding officer, legitimacy, stability, the debate in progress | row 04 | Government window |
| Ideology: nobles · commons · government meters | row 06 | Ideology |
| Culture tiers: the table, with the most recent change | row 05 | Society |
| People: resident notables, scholars, artisans, performers (portraits) | rows 02, 06–08 | Person windows |
| Venues and masterworks | rows 07, 08 | Venue subpanel, Gallery |
| War and realm: at war with whom, sieges, occupation, which realm and its rank, horde threat nearby | row 09 | War window, Realms, Barbarian Tribes |
| Economy at a glance: top exports/imports, treasury | existing | Market, Flows |
| Last 5 chronicle lines | row 01 | the city's history |

The city portrait should change with development — that is the most direct way to
*see* a city grow.

## Slices

| Slice | Content | Gate |
|---|---|---|
| 10.1 | One read query assembling the Overview (no new sim state) | `cargo check --lib --tests` |
| 10.2 | Overview component as the first tab of the settlement panel | `tsc`, `vite build` |
| 10.3 | City portrait reflecting buildings and venues | visual check |
| 10.4 | Tidy the remaining tabs into their detail windows | `tsc`, `vite build` |

No sim change → no `econ_` run needed (CLAUDE.md §2.8, frontend row).

## Queue
- Q10.1 — Venue art and seasonal festivals animated on the portrait.
- Q10.2 — City portrait reflecting buildings/venues (10.3) — needs
  `buildingArt.ts`-style procedural art for a venue, not built this session.
- Q10.3 — Tidy the remaining tabs into pure detail windows (10.4) — the
  Overview now duplicates a little of what "summary"/"development" already
  show; a full pass to make each tab strictly the DETAIL behind an Overview
  block (never a second copy of the summary) is real follow-up work.
- Q10.4 — Fold in development-track levels, city identity (portrait/
  population/culture mix) and an economy-at-a-glance block — the data
  already has its own reader elsewhere in `HubPanel.tsx`; only the Overview
  composition itself is missing them.
- Q10.5 — A war/realm/horde-threat block — waits on row 09's Parts B/C
  (armies, realm wars) actually existing to have something to summarise.
