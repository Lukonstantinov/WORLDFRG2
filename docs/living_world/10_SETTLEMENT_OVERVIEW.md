# 10 · Settlement overview

**Status:** NOT STARTED · **Depends on:** 03–08 · **Next:** —

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
