# WorldForge 2 — Trade & Economy System Review

*A multidisciplinary review (economics · sociology · history · biology) of the
implemented trade simulation, with statistics and recommendations.*

Scope: `sim/biological.rs` (goods), `commands/query_commands.rs` (routes, matrix,
political, economy), `StepBiological.tsx` + `uiStore.bioParams` (controls).

---

## 1. Snapshot statistics

| Metric | Value |
|---|---|
| Trade goods modelled | **38** (`GOODS_COUNT`), v1 saves = 21 (back-compat) |
| Marine goods | 5 (stockfish, amber, dyes, pearls, whaling) |
| Deposit goods (point-scatter) | 5 (gemstones, copper, tin, gold, salt, iron) |
| Distribution models | 3 — **Global** (unlimited), **Local** (one seeded homeland), **Deposits** |
| Per-good attributes | label, icon, colour, desire (0.25–0.85), rarity, marine, luxury flag, distribution, envelope |
| Goods editor | declarative `GoodSpec` envelopes — custom goods, fully editable |
| Köppen zones driving goods | 22 (used as climate gates) |
| Quality grades | 5 tiers (Coarse→Exquisite) + curated flavour ladders for ~22 goods |
| Reference hubs (1450 CE) | 14 (Venice=100 … Kilwa=30) for throughput comparison |
| Coarse routing grid | ~700 cells wide, 8-neighbour Dijkstra |
| Trade reaches | 3 (Global / Coastal+short / Continental) |
| User controls | gem deposits, climate strictness, economic regions (4–40), demand bias, piracy, season/months, desert-routes, max crossing |
| Maritime hazards priced into routing | storms (seasonal), reefs, piracy, shipworms, seasonal pass closures |

---

## 2. What is implemented (and is genuinely good)

### Economics ✅
- **Supply/demand matching.** Production summed per region from belt fields; demand
  = economic size × per-good desire; net surplus matched to deficit → flows.
- **Price formation.** Delivered price = origin (quality) × **scarcity** (1/abundance,
  1.0–3.3×) × **transport** (route cost) × local-shortage premium. Prices accrue
  **per hop** along the real route — a working spatial price gradient.
- **Price-elastic demand** (`ELAST = 0.6`): dear, far-hauled goods move in smaller
  volume — markets only reach overseas for what they cannot get nearer. This is
  textbook gravity/iceberg-cost trade behaviour.
- **Homeland discount** + **full-basket floor** (0.35): a good is cheap where it is
  made; everywhere imports a little of everything. Sound.
- **Comparative advantage emerges** from one-homeland seeded goods → clean monopolies
  → trade. Deposit abundance floor (0.6) keeps tiny-but-precious goods trade-relevant.
- **Wealth & feedback loop:** hub wealth → `compute_settlement_development` grows
  emporia up to ×3. Trade reshapes the settlement hierarchy.

### History ✅
- **Chokepoints** (Strait/Passage/Pass) emerge from edge-volume clustering — Malacca/
  Bosphorus/Hormuz analogues, named & ranked.
- **Silk-Road mode** (`desert_routes`): cuts the steppe penalty so overland caravan
  corridors out-compete dangerous seas.
- **Mountain passes** (saddle discount ×0.45), **navigable rivers** as cheap inland
  highways, **coast-hugging** shipping, **open-ocean** penalty so trade crosses at the
  narrowest strait. All historically faithful.
- **Seasonality:** snow-shut passes (winter, lat-weighted) + monsoon/cyclone sailing
  windows by hemisphere. Reference 1450 hub comparison is a nice touch.

### Sociology ✅
- **Network-luxury demand:** distant luxuries only prized in large/open networks;
  closed/continental worlds care about staples. Mercantile↔subsistence slider.
- **Political power** = 0.45 habitability + 0.30 route centrality + 0.25 good
  monopoly → 5-tier hubs with influence discs. Trade, not just food, makes capitals.

### Biology ✅
- Goods keyed to **Köppen + temperature/precip/elevation/latitude bells**, fertility,
  coast. Genuinely biogeographic (sericulture warm-temperate, cacao wet-tropical
  lowland, frankincense arid, stockfish cold N. banks tied to the fishery field).
- **tsetse belt** (savanna caravan penalty), **shipworm** (Teredo, warm low-salinity
  hulls), **shark/reef/disease/storm** hazard layers all biologically grounded.
- Island-jump flood-fill so straits/archipelagos don't fragment a homeland.

---

## 3. Gaps & realism issues (by discipline)

### Economics ⚠️
1. **No money/price equilibrium, only one-pass matching.** Flows are greedy nearest-
   first; there is no market-clearing iteration, so prices don't feed back into
   *who produces what*. Real economies re-allocate land/labour toward high-price
   goods. → Consider 2–3 relaxation passes, or a simple Walrasian price update.
2. **Demand is production-and-population-scaled, not income-closed.** A region's
   imports aren't budget-constrained by its export earnings — a hub can "buy"
   beyond its means. Add a balance-of-payments cap (imports ≤ export value × credit).
3. **No transport capacity / congestion.** A trunk's width is volume, but cost
   doesn't rise with traffic, so chokepoints never bottleneck or price-spike.
4. **Production is static intensity, not output × labour.** Belt u8 = suitability,
   not quantity produced by a workforce. Population doesn't *work* the land.
5. **No middleman/entrepôt margin.** Venice's wealth was re-export arbitrage;
   chains accrue transport cost but hubs don't take a markup/tariff.

### History ⚠️
6. **No tariffs, tolls, guilds, or staple rights** — the institutional layer that
   actually shaped medieval trade (Sound Dues, Hanseatic Kontors, Italian quarters).
7. **No temporal dynamics.** It's a single 1450-style snapshot; no trade growth,
   route-shift, or shock (a closed Silk Road, a new sea route) over time.
8. **Caravan/ship range & relay structure** is implicit. No oases/caravanserai or
   port-of-call spacing constraining how far a single leg can run.

### Sociology ⚠️
9. **No demand heterogeneity by culture/class.** Every region wants the same basket
   scaled by size; no elite-vs-commoner demand, no cultural taste (no wine-avoiding
   regions, no prestige goods restricted to capitals).
10. **No conflict/competition over trade.** Monopoly is measured but never contested;
    no trade wars, embargoes, or piracy *response* (piracy is an exogenous cost).
11. **Influence discs are geometric, not political.** No borders, no rival exclusion,
    discs simply overlap.

### Biology ⚠️
12. **No carrying capacity / overexploitation.** Whaling grounds and fisheries never
    deplete; furs never trap out. Real staple trades collapsed their resource base.
13. **Seasonality of *production* is absent** (only of *transport*). Harvests, fish
    runs, and caravan seasons are tied to the same calendar but goods are annual.
14. **Pack-animal biogeography is partial.** tsetse is modelled, but camels (desert),
    horses (steppe), and llamas exist as *goods* yet don't change *transport cost*
    by region — a desert should be cheap *only if* camels are locally available.
15. **Disease suppresses settlement but not trade routes.** Malaria coasts should
    raise the cost/risk of a port, not only its habitability.

---

## 4. Quick-win recommendations (ranked)

| # | Change | Discipline | Effort |
|---|---|---|---|
| 1 | **Resource depletion** on fisheries/whaling/furs (stock that draws down under sustained flow) | Biology/Econ | M |
| 2 | **Balance-of-payments cap** on imports (close the income loop) | Economics | S |
| 3 | **Entrepôt markup / tolls at chokepoints** (re-export wealth, the Venice effect) | History/Econ | S |
| 4 | **Congestion pricing** on trunks (volume raises edge cost) → real bottlenecks | Economics | M |
| 5 | **Pack-animal-gated land cost** (camel/horse/llama goods unlock cheap desert/steppe/highland transit) | Biology/Hist | M |
| 6 | **Production seasonality** reusing the existing month slider (harvest/fish-run phase) | Biology | M |
| 7 | **Class/cultural demand modifiers** (capital-only luxury demand, regional tastes) | Sociology | M |
| 8 | **Iterative market clearing** (2–3 passes so prices reallocate supply) | Economics | L |
| 9 | **Temporal trade shocks** (route closure events, new-route discovery) | History | L |

---

## 5. Verdict

The system is **unusually deep for a worldgen tool** — it already does spatial price
formation, elastic demand, scarcity, monopoly-driven politics, biogeographic goods,
maritime hazards, seasonal route closures, and emergent chokepoints, with a fully
editable goods library and CSV/JSON export. The bones of a real economic geography
are here.

The main realism gaps are **closure and dynamics**: it is a static, open-budget,
non-depleting snapshot. The highest-value next steps are (1) renewable-resource
depletion, (2) an income/balance-of-payments constraint, and (3) tolls/entrepôt
margins — together these would turn a convincing *map* of trade into a convincing
*economy*.
