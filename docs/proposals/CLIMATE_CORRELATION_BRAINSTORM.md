# WorldForge 2 — Real-Climate Correlation & Biology: 20 Proposals

> **See also `FIX_PLAN.md`** — the climate model has since gained an energy-balance
> model, astronomical insolation, rotation-derived circulation belts and an **Earth
> validation harness**. Fidelity is now a measured number (main-class 66.2%,
> exact-zone 29.0%), and the prioritised climate fixes live there. This document
> remains a proposal menu; the fix plan is what's actually scheduled.

**Theme:** help world *creators* see how their generated map correlates to real
Earth climate, and enrich the world with real climate/biology information.

**Grounding (already in the codebase):**
- Full **Köppen** classification — 31 zone codes + highland (`sim/step4_climate/koppen.rs`).
- Per-cell **temperature** + **seasonal range** (`seasonal_range_base`, `seasonal_temps`).
- **Precipitation** (ITCZ / orographic / frontal model).
- **Ocean currents**, **salinity** (incl. thermohaline + advection).
- A **biomes** render layer + hazard columns (`storm`, `reef`, `disease`, shark, shipworm).
- **Trade-good belts** tied to climate suitability.

So "real-climate correlation" is mostly **bundled reference data + new overlays/panels**,
not new physics. Everything below can stay **offline / self-contained**, matching the
current architecture (bundle Köppen tables / biome rules / species envelopes into the app).

Ranked by **feasibility** (1 = easiest to ship). ⭐ = highest creator value.

---

## Tier A — Low-hanging fruit (data already present; UI + small bundled table)

### 1. Köppen distribution histogram vs. Earth
% of world land per climate zone, overlaid on Earth's real distribution; flag
over/under-represented zones.
- **Value:** instant realism gut-check.
- **Critique:** Earth is one reference; a fantasy world may differ on purpose — frame as comparison, not error.
- **Feasibility: very high** — aggregate `koppen[i]` + a constant Earth table.

### 2. Latitude climate ribbon
Hover a row → strip of what climate/biome bands Earth has at that latitude.
- **Value:** teaches the latitude→climate intuition live.
- **Critique:** ignores continentality/longitude.
- **Feasibility: very high.**

### 3. Köppen teaching legend with real examples
Click a zone → real cities/regions + plain-language description.
- **Value:** demystifies the codes.
- **Critique:** needs curated text.
- **Feasibility: very high** (static dataset).

### 4. Earth-Analog Finder ⭐
For any cell/region, find the closest real place by climate vector (Köppen + mean
temp + annual precip + seasonal range): "this coast ≈ Pacific NW; interior ≈ Kazakh steppe."
- **Value:** the single most on-brief feature.
- **Critique:** match quality scales with reference dataset; needs a good distance metric.
- **Feasibility: high** — inputs exist; bundle ~200–500 reference points.

### 5. Biome → real flora & fauna suggestions
Per biome/Köppen zone, curated analog animals & plants.
- **Value:** the biological angle; inspiration for GMs/authors.
- **Critique:** static lookup; curation effort; label as "real-world analogs for inspiration."
- **Feasibility: high.**

### 6. Region dossier / climate reference card
Select a region → one-pager: Köppen, Earth analogs (#4), weather, flora/fauna (#5),
plausible crops, native hazards.
- **Value:** turns raw sim into usable lore.
- **Critique:** aggregator — only as good as #4/#5.
- **Feasibility: high** (after 4 & 5).

### 7. Day-length & solar insolation overlay
Astronomy from latitude config: daylight hours + insolation by latitude & season.
- **Value:** polar-night/midnight-sun realism, growing-season grounding.
- **Critique:** needs an axial-tilt setting.
- **Feasibility: high** (no sim data needed).

### 8. Hardiness & growing-season zones
USDA-style hardiness + frost-free season from temp + seasonal range; plausible crops.
- **Value:** agriculture realism; ties to economy/goods.
- **Critique:** needs winter-min estimate; approximate mapping.
- **Feasibility: high.**

---

## Tier B — Moderate (new derived computation or richer dataset)

### 9. Whittaker biome layer upgrade
Promote `biomes` layer to a proper Whittaker classification (temp × precip) + 2D legend.
- **Value:** ecology-first lens on climate.
- **Critique:** may overlap current biome logic — verify before rebuilding.
- **Feasibility: high–medium.**

### 10. Animal habitat-envelope overlay ⭐
Pick a real animal → highlight cells matching its real range envelope ("where could
polar bears / camels / tigers live?").
- **Value:** most fun/shareable biological feature.
- **Critique:** needs species envelope dataset; ignores prey/barriers.
- **Feasibility: medium.**

### 11. Köppen realism validator (climate linter)
Rule-based checker flags implausible placements (lowland tundra at equator, rainforest
abutting hot desert) with explanations + jump-to-cell.
- **Value:** catches invisible mistakes.
- **Critique:** "implausible" is subtle; make every rule dismissible/overridable.
- **Feasibility: medium.**

### 12. Walter-Lieth climate diagram per cell
Classic 12-month temp+precip atlas graph for any clicked cell.
- **Value:** professional way to read climate; pairs with #4.
- **Critique:** only annual + seasonal range stored → months synthesized (approximation).
- **Feasibility: medium.**

### 13. Realism report card
Whole-world score with breakdown (band realism, biome diversity, ocean sanity, vs Earth).
- **Value:** one-glance health + onboarding.
- **Critique:** scoring is subjective — show components, not just a grade.
- **Feasibility: medium** (aggregates #1, #11, #15).

### 14. Earth Köppen comparison overlay
Bundle a real Earth Köppen raster; side-by-side / latitude-aligned view.
- **Value:** direct visual benchmark.
- **Critique:** world isn't Earth-shaped → honest comparison is by latitude band only.
- **Feasibility: medium.**

### 15. Ecoregion clustering & naming
Cluster similar climate+biome cells into named ecoregions (à la WWF).
- **Value:** ready-made named regions.
- **Critique:** boundary tuning; reuse existing trade-region clustering.
- **Feasibility: medium.**

### 16. Storm / hurricane-formation realism
Use storm column + SST + latitude to mark plausible cyclone genesis belts.
- **Value:** coastal-hazard realism + lore.
- **Critique:** simplified genesis model.
- **Feasibility: medium.**

---

## Tier C — Ambitious (real new simulation / heavier scope)

### 17. Climate-change / axial-tilt scenario slider
Offset global temp or tilt, re-run pipeline, watch biomes migrate.
- **Value:** ice-age vs greenhouse worldbuilding + education.
- **Critique:** re-sim cost; compare-states UX; caching.
- **Feasibility: medium–low.**

### 18. Ocean-current circulation check
Verify gyres rotate correctly (CW N / CCW S), W-boundary warm, E cold; flag anomalies.
- **Value:** catches subtle ocean errors most generators get wrong.
- **Critique:** robust detection is genuinely hard.
- **Feasibility: medium–low.**

### 19. Seasonal climate playback (Jan→Dec)
Animate temp/precip + ITCZ/monsoon shift across the year.
- **Value:** makes the world feel alive; shows wet/dry seasons.
- **Critique:** needs month-synthesis (#12) + animation plumbing in tile renderer.
- **Feasibility: low–medium.**

### 20. Migratory corridors & trait-based species generation
Model seasonal migration over biomes (coarse cost grid) and/or generate creature
ecologies from climate niches.
- **Value:** deepest biological layer; unique GM hook.
- **Critique:** big scope, fuzzy to validate, easy to over-engineer.
- **Feasibility: low.**

---

## Recommendation

Build the **spine** first — **#4 Earth-Analog Finder + #5 flora/fauna + #6 dossier**,
fed by one bundled reference dataset, with **#1 histogram** and **#3 legend** as quick
wins. That trio delivers the exact "see how my map correlates to real climate, with
biology" pitch, reuses existing data, and stays fully offline. **#10 animal-envelope
overlay** is the standout shareable follow-up.

---

# Part 2 — Human, Settlement, Economy & Goods (18 more)

The campaign side is the engine's deepest layer (tick sim: houses, banks, coin, wars,
markets; 45 goods + recipe DAGs; trade trunks; political/centrality ranking). Most of
these are **reuse + new panels**. ⭐ = standout value.

## Human & settlement

### 21. City rank-size (Zipf) realism check
Plot settlement wealth/size vs Zipf's law; flag a missing primate city or a too-flat hierarchy.
- **Critique:** fantasy worlds may differ — a lens, not a verdict. **Feasibility: high** (`CityRankingPanel`).

### 22. "Why is this city here?" site dossier ⭐
Break down the habitability score + tag the real site archetype (river-ford, harbor, oasis, hill).
- **Value:** explainable placement lore; pairs with #6. **Feasibility: high** (Phase-7 components).

### 23. Travel-time / itinerary calculator ⭐
Two settlements → journey time by foot/horse/cart/ship/river over the coarse cost grid.
- **Value:** top GM tool. **Critique:** needs per-mode speeds. **Feasibility: high** (cost grid + routes exist).

### 24. Settlement tiers + real analogs
hamlet→metropolis bands + a real historical analog.
- **Critique:** curated mapping. **Feasibility: high.**

### 25. Population & carrying-capacity estimate + density map
Plausible populations from fertility/area; density overlay.
- **Critique:** tech-level assumption; rough. **Feasibility: high–medium.**

### 26. Geographic toponym generator
Extend `names.rs` + cultures to name rivers/mountains/regions with meanings.
- **Critique:** linguistic rabbit hole. **Feasibility: high.**

### 27. Persistent roads & waystations
Promote trade trunks into named roads with inns spaced a day apart.
- **Critique:** persistence + placement is new. **Feasibility: medium.**

### 28. Defensibility / fortification layer
Terrain + chokepoints + rivers → where castles/walls make sense.
- **Critique:** heuristic-heavy. **Feasibility: medium.**

## Economy

### 29. Wealth inequality (Gini) & social mobility over time ⭐
Chart Gini, concentration, and house turnover from the tick sim; optional historical compare.
- **Value:** makes the living economy legible; on-theme. **Feasibility: high** (tick tracks wealth + turnover).

### 30. City price index / cost-of-living
Basket-of-goods CPI per hub to compare cities.
- **Feasibility: high** (market prices exist).

### 31. Trade balance & comparative-advantage map
Per-region net exporter/importer + Ricardian specialization.
- **Feasibility: high** (trade-matrix net flows).

### 32. Economic shock timeline
Replay crashes/wars/bank failures with cause→effect chains.
- **Critique:** mostly UI over existing logs/why-chains. **Feasibility: medium (UI).**

### 33. Historical economy-era analog
Tag the current economy (barter/coinage/banking) to a real era.
- **Critique:** interpretive, fuzzy. **Feasibility: medium.**

### 34. Wages & labor layer
Wages by city from labor demand vs cost of living.
- **Critique:** new variable to balance. **Feasibility: medium.**

## Goods

### 35. Goods provenance / supply-chain tracer ⭐
Click a finished good → trace back through recipe DAG + actual routes/belts.
- **Value:** spectacular; reuses DAG + `GoodFlowPanel`. **Feasibility: high.**

### 36. Real-world commodity history cards
Each good gets a short real-history card (silk road, spice trade, salt-as-money).
- **Value:** cheap lore depth; on-theme. **Feasibility: very high** (static content).

### 37. Scarcity ↔ luxury heatmap per good
Overlay cheap-commodity vs prized-luxury from market price gradients.
- **Feasibility: high** (gradients already computed).

### 38. Seasonal harvest calendar
Perishables/harvests vary by season per region (shares season model with #12/#19).
- **Critique:** needs a season model. **Feasibility: medium.**

## Best bets (Part 2)
- **Settlement layer:** #22 + #23 + #24 (mostly reuse).
- **Economy legibility:** #29 + #30 + #31 (dashboards over tick data).
- **Goods:** #35 + #36 (high wow, low cost).

Biggest "wow per hour": **#23 travel-time** and **#35 provenance tracer**; nearly free
wins: **#36 commodity cards** and **#22 site dossier**.
