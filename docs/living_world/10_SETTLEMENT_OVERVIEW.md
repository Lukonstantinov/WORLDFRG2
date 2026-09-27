# 10 · Settlement overview

**Status:** PARTIAL — 10.1/10.2 (2026-09-26) plus Q10.4's fold-in and 10.3's
portrait (2026-09-27) shipped; a full tab-tidy pass (the remainder of 10.4)
and a war/realm/horde-threat block (10.5, waits on row 09) are still
queued · **Depends on:** 03–08 · **Next:** —

**Shipped (2026-09-26).** `ui/campaign/SettlementOverviewTab.tsx`, a new
"Overview" tab (now the FIRST/default tab on selecting a city) in
`HubPanel.tsx`, showing government (form/legitimacy/seats/debate in
progress), ideology (dominant doctrine + scholar count), culture tiers, a
venues/masterworks count, and the last 5 chronicle lines.

**Scope cut, recorded rather than silently made (rule 36):** 10.1 calls for
"one read query assembling the Overview (no new sim state)". This is built
as a CLIENT-SIDE composition of rows 04/06/07/08's own already-shipped read
commands instead — each of those `#[tauri::command]` functions takes its
own `db: State<'_, WorldDb>` and locks the SAME `std::sync::Mutex`
internally, so calling one FROM inside a new aggregator command would
deadlock on that lock. Composing independent `invoke()` calls client-side
avoids inventing a second, parallel read path for data these commands
already serve correctly, and needed zero new Rust code.

**Shipped (2026-09-27) — Q10.4's fold-in and 10.3's portrait.**
`campaignGetHub`/`campaignCityDevelopment` (both already read elsewhere in
`HubPanel.tsx`) are now ALSO called from `SettlementOverviewTab` itself,
adding three blocks the doc's own table asked for: **Identity**
(population, culture, minority count), **Development** (the dev value, this
year's net breakdown, and the four track levels), and **Economy at a
glance** (treasury, trade wealth, food balance) — every field a plain read
off `HubDetail`/`CityDevelopment`, no derived guesswork. The **city
portrait** (10.3) reuses the EXISTING, already-shipped `cityScene`/
`IsoThumb` isometric renderer VERBATIM — the same one `CityView.tsx`,
`ColonyWindow.tsx` and `HouseWindow.tsx` already draw a city with, wired
through the same `useWorldStore`/`useCultureKits` lookups those three
already use for sea-access/elevation/river/culture-kit — rather than new
procedural art. This was a deliberate choice, not a shortcut: this
environment has no display to visually verify a NEW canvas layer against,
which is exactly the risk §8.21's own fill-light regression and row 02's
02.7 (face art) both name and both held back for. Reusing an
ALREADY-VERIFIED renderer sidesteps that risk entirely while still
satisfying "reflects buildings and venues" (`deriveSlots`/`deriveDistricts`
already read the hub's real structures/venues state). A genuinely new,
venue-specific sprite/animation layer (the doc's own Q10.1/Q10.2) is real,
separate follow-up work.

Gate (2026-09-26): `cargo check --lib --tests` (unchanged, no Rust touched),
`npx tsc --noEmit` + `npx vite build` (191 modules) both clean. Gate
(2026-09-27): same — no Rust touched by either the fold-in or the reused
portrait renderer, so `npx tsc --noEmit` + `npx vite build` (191 modules,
unchanged) both clean. No `econ_` run owed either time — no sim state
changed (CLAUDE.md §2.8's own routing-table row for a frontend-only
change).

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
| 10.3 | **DONE (2026-09-27).** City portrait — reuses the existing `cityScene`/`IsoThumb` renderer verbatim rather than new procedural art (no display to verify one against); reflects buildings/districts/ships via the same `deriveSlots`/`deriveDistricts` every other city window already reads | `tsc`, `vite build` (both clean, no visual check possible in this environment — stated plainly) |
| 10.4 | **PARTIAL (2026-09-27).** Development-track levels, identity (population/culture mix) and an economy-at-a-glance block folded into Overview. **NOT done**: tidying the OTHER tabs into pure detail windows behind each Overview block (the remainder of 10.4, tracked as Q10.3) | `tsc`, `vite build` |

No sim change → no `econ_` run needed (CLAUDE.md §2.8, frontend row).

## Queue
- Q10.1 — Venue art and seasonal festivals animated on the portrait — real,
  separate procedural-art work, still queued (10.3 reused an EXISTING
  renderer rather than building this).
- Q10.2 — **DONE 2026-09-27** (10.3 shipped, via the existing `cityScene`/
  `IsoThumb` renderer rather than new `buildingArt.ts`-style venue art —
  see the doc's own 2026-09-27 entry for why that was the safer call in an
  environment with no display).
- Q10.3 — Tidy the remaining tabs into pure detail windows (the rest of
  10.4) — the Overview now duplicates a little of what "summary"/
  "development" already show; a full pass to make each tab strictly the
  DETAIL behind an Overview block (never a second copy of the summary) is
  real follow-up work, not attempted this session either.
- Q10.4 — **DONE 2026-09-27** (development-track levels, city identity —
  portrait + population/culture mix — and an economy-at-a-glance block are
  all folded into Overview now; see the doc's own 2026-09-27 entry).
- Q10.5 — A war/realm/horde-threat block — waits on row 09's Parts B/C
  (armies, realm wars) actually existing to have something to summarise.
