# Population, Growth & Trade-Network Dynamics — Analysis & Fix Plan

*Analysis lens: historian · city-builder · social engineer. Scope: why population
stalls, why some settlements never join the trade network, and how to make the
world grow and stay interesting to watch.*

---

## 0. TL;DR — the three root causes

1. **Population is mathematically capped.** Every city's carrying capacity is a
   *fixed multiple of its `founding_pop`*, and that multiple maxes at ~9.24× (realistically
   ~2–3×). `founding_pop` never changes. So total world population asymptotes to
   `Σ(founding_pop) × cap_mult` and then **logistic growth drives the rate to zero**.
   3.2M is not a bug in one city — it is the sum of all the ceilings. (`tick.rs:7829-7847`)

2. **Only the top 250 settlements are "alive."** The rest are frozen `hinterland`
   towns: drawn, clickable, counted in the census — but **never simulated, never grow,
   never trade, never join the network** (`campaign_commands.rs:1787-1828`, census at
   `:920-923`). That is exactly the "some settlements are static" complaint.

3. **Trade routes are straight lines.** The campaign tick's route-days matrix is pure
   **Euclidean distance × days_per_cell** between hubs within a connectivity component
   (`tick.rs:4661-4683`). It ignores terrain, passes, coast-hugging and the coarse cost
   grid. The real pathfinding routes live only in the read-only overlay
   (`query_commands.rs::compute_trade_routes`) and are **never fed to the sim.**

---

## 1. Strong points (what's already good)

- **Deterministic, tile-free tick.** Pure `(seed, tick)` math, no DB/global RNG, fast
  and reproducible. Excellent substrate for a living economy.
- **Rich emergent layers already exist:** logistic growth, food/starvation guards,
  estates, houses/guilds, banks, coinage, wars, crashes, colonies, migration corridors,
  satellites, epidemics.
- **Migration already follows trade ties, not straight lines.** `economic_migration_pass`
  (`tick.rs:9497`) moves people only to *direct trade-partner neighbours* with a
  homophily pull — a genuinely good "social engineering" rule. It chains city→city over
  years. (But it's zero-sum: see §2.)
- **Cultures 2.0 exists:** minority quarters, creoles (`CREOLE_MIN_POP`), lingua franca
  per component, culture history sampling — the machinery for "new cultures appear" is
  present, just under-triggered.
- **Connectivity is already continent-aware.** Components are built by geographic
  K-nearest union (`COMP_K=6`, `max_link≈30% world width`) + worldgen corridors, with a
  tiny-component rescue (`campaign_commands.rs:1974-2050`, `tick.rs:4628`). Wide ocean
  gaps correctly stay separate markets.
- **Limited-liability wealth guards** keep the economy bounded and finite (the standing
  dynamics test hard-asserts this).

## 2. Weak points (what blocks growth & a living map)

### 2.1 The population ceiling (primary)
```
cap_mult = (0.35 + 1.30·food_sec) · (0.60 + 5.0·prosperity²)   // max ≈ 9.24×
capacity = founding_pop · cap_mult                              // founding_pop is FROZEN
new_pop  = pop + rate·pop·(1 − pop/capacity)                    // logistic → 0 at cap
```
- `founding_pop` is set once (500 / 2 000 / 10 000 by rank) and **never rises**. A city
  cannot become a metropolis of 100k+ from a 10k founding — the hard ceiling is ~92k
  even at impossible perfection, ~25-30k realistically.
- `prosperity` and `food_sec` are **eased sentiments clamped to [0,1]** → bounded inputs
  → bounded capacity → bounded world.
- **Historian's verdict:** real premodern cities grew 10–100× over centuries as trade
  hinterlands deepened; here a city's destiny is fixed on day 1.

### 2.2 The 250-hub cap freezes the long tail
- Settlements ranked 251+ are inert. They don't grow, don't trade, don't migrate, can't
  be a migration destination, can't found colonies. On a large map this is *most* of the
  world's places — permanently static dots.

### 2.3 Straight-line routes
- The sim's cost model is "as the crow flies." No mountain detour, no strait, no
  coast-hugging. Migration/trade "lines" therefore cut across terrain. The proper
  pathfinder exists but is overlay-only and never reaches the tick.

### 2.4 Growth is redistributive, not generative
- Migration is **zero-sum** (`hubs[src].pop -= movers; hubs[di].pop += movers`,
  `tick.rs:9537-9538`). It concentrates people (good — centers emerge) but adds nothing
  to the total. With every city near its cap, the world total is flat and migration just
  shuffles a fixed pie.

### 2.5 New cities/cultures are rare and small
- Colonies gate on year ≥ 50, pop/wealth thresholds, and are founded ~yearly, each
  starting at 500 and capped at the same `founding_pop` ceiling. Creoles need
  `CREOLE_MIN_POP` + minority quarters that migration rarely builds up. So "new cultures
  appear / new trade centers rise" happens too weakly to watch.

---

## 3. Why 500 years → 3.2M then flat (the mechanism, step by step)

1. Day 0: `Σ founding_pop` across 250 live hubs (≈0.5–0.7M) + frozen hinterland.
2. Logistic growth lifts each hub toward `founding_pop · cap_mult` (~2–3× realistic).
3. Within ~50–100 years every hub is within a few % of its cap → `(1 − pop/cap) → 0` →
   growth rate → 0.
4. Migration reshuffles the now-fixed pie toward prosperous centers but can't grow it.
5. Colonies add a trickle of new small (capped) hubs; hinterland never moves.
6. **Total asymptotes.** 3.2M is the sum of all ceilings — stable by construction.

### 3.1 Evidence from the standing dynamics test (50y, 30-town synthetic world)
```
yr  5: towns 30  hungry 23  thriving 1
yr 20: towns 28  hungry 24  thriving 0
yr 35: towns 28  hungry 20  thriving 0
yr 50: towns 29  hungry 22  thriving 1     richest 417438
```
- **20–24 of ~30 towns are HUNGRY and 0–1 THRIVING for the entire run.** Both inputs to
  the carrying-capacity formula are pinned low, so `cap_mult ≈ 1.0–1.3×` → cities barely
  grow past founding. This is the stall mechanism made visible: it isn't only the fixed
  ceiling, it's that **the economy keeps cities hungry and non-prosperous, holding
  capacity at ~1× founding.**
- Town count is flat (28–30) — no net new cities; houses do turn over (rise/defunct),
  which is healthy. So the *economic churn* is alive but the *demographic engine* is not.
- Takeaway: fixing growth needs BOTH (a) a rising/earned ceiling (§4.1) AND (b) getting
  food security up so the existing logistic term actually has headroom to climb.

---

## 4. Fix direction (design)

### 4.1 Break the founding-pop ceiling → capacity from *land + trade*, and let it ratchet
- Recompute `capacity` from an **absolute site-carrying-capacity** (hinterland fertility,
  water, coast) **plus a trade multiplier that grows with realized throughput/connectivity**,
  not from a frozen founding number. Let a well-connected entrepôt's cap ratchet upward
  as its trade network deepens (a slow, decaying "developed capacity" term), so a hub can
  climb from 10k → 100k+ over centuries when it earns it.
- Keep logistic form (bounded, stable) but make the ceiling *earned and rising*, not fixed.

### 4.2 Make more (ideally all) settlements live
- Options (pick per performance budget): raise the cap; or promote/demote hinterland ↔
  live dynamically (a growing hinterland town "wakes up" into a real hub when it crosses a
  threshold; a dead hub sleeps). Even a cheap "hinterland grows slowly toward its own small
  cap and can graduate" removes the static-dot feel.

### 4.3 Real routes into the tick (never a straight line)
- **Precompute a pathfound route-days matrix once at campaign start** over the coarse cost
  grid (passes / rivers / coast-hugging / reach-limited sea crossings — the same rules
  `compute_trade_routes` already uses) and **serialize it**. The tick stays tile-free; it
  just reads a real matrix instead of `dist × days_per_cell`. Neighbors, migration and
  dispatch then all lie on real routes automatically.
- This also fixes components: reachability = "finite pathfound days," so islands connect
  only where a real sea route within reach exists.

### 4.4 The "zero'fy & grow" campaign start (user's core request)
- Add a **"Cold Start"** toggle/button at campaign start: set every hub to a small seed
  population, **empty all trade ties, warehouses, houses, coin, wealth to zero**, and clear
  the route network.
- On unpause, cities **discover partners organically**: each tick a hub can open a trade
  tie to a *reachable* partner (via the pathfound matrix) when the price gradient/ surplus
  makes it worthwhile; ties strengthen with use and decay when unused. The route network
  thus *emerges* rather than being handed over pre-built. Migration, houses, guilds, coin
  then bootstrap on top exactly as the existing emergence-order intends (merchants → guilds
  yr5 → houses yr10).

### 4.5 Generative population + emergent culture
- Add a small **net biological growth** term (births−deaths) gated by food security so the
  total pie can actually grow, with famine/plague as the checks (already present). Migration
  then *concentrates* real growth into trade/cultural centers.
- Lower creole/minority thresholds so sustained migration mixing **spawns new cultures** at
  big cosmopolitan hubs; let a dominant in-migrant culture flip a city's lingua franca.

---

## 5. Decisions (locked)

- **Growth model:** rising *earned* carrying capacity (land + trade throughput, ratchets
  up) **AND** a generative net births−deaths term gated by food. *(Q1 = both.)*
- **Static towns:** dynamic **wake/sleep** hinterland (grow slowly, graduate to live hubs).
- **Routes:** **precompute a pathfound route-days matrix** at campaign start; tick reads it.
- **Cold Start:** ship as a **new optional mode** (default start unchanged).
- **Procurement futures:** a **merchant house** (house-first) signs input contracts.
- **Input sourcing:** traders open **new pathfound routes AND found resource colonies**.
- **Manufacturing spread:** keep concentration but **slightly lower the labour gate** so
  mid cities craft 1–2 secondary goods.
- **Sequencing:** **growth + food fix FIRST** (unblocks the stall), test, then routes →
  procurement futures + trader-seeking → hinterland wake/sleep + Cold Start.
- **Target scale (working goal, tunable):** over ~500 y expect a handful of cities at
  200k–500k, total world pop rising into the tens of millions, 2–3 new (creole) cultures,
  and a visibly denser real-route web. *(Confirm/adjust.)*

---

---

## 5b. Implementation status

- **Slice 1 — growth + food (IN PROGRESS).**
  - ✅ *Earned rising ceiling:* carrying capacity now ratchets with realized trade
    (`trade_last_year`) via `trade_dev` — a busy entrepôt reaches ≈30× founding (was a
    fixed ~9×), an isolated hub stays small. `tick.rs` growth block + new consts
    `TRADE_DEV_REF`/`TRADE_DEV_CAP`.
  - ✅ *Generative births−deaths:* net demographic drift (`BIRTH_RATE·food_sec −
    DEATH_RATE_BASE`) added below capacity so the TOTAL pie can grow, not just
    redistribute. Damped by remaining headroom → stays bounded/finite.
  - ✅ Standing dynamics test green; wealth bounded (~423k), turnover intact.
  - ⏳ *Caveat:* the synthetic test world is food-poor by construction (≈28/30 hungry),
    so it can't visually showcase growth; real validation needs a full generated world
    (needs GUI — verify in-app on the user's 500-y campaign). Next: decouple "at food
    capacity (content, small)" from "starving (dying)" so structurally arid sites settle
    at a stable small size instead of perpetual famine pinning their growth signal.
- Slices 2-4 (pathfound routes · procurement futures + trader-seeking · hinterland
  wake/sleep + Cold Start): not started.

---

## 6. Manufactured goods, procurement futures & traders seeking inputs

### 6.1 Finding: the catalog is rich, but little gets MADE
- A new world already ships **~25 manufactured goods** (`default_list()` = 45 builtins +
  `default_custom_goods()`: cloth, metalware, refined_sugar, citrus_liqueur, linen,
  cotton_cloth, silk_brocade, carpets, leather_goods, bronzeware, jewelry, brandy, mead,
  perfume, soap, candles, books, furniture, ivory_carvings, statuary, …). So "not many
  manufactured goods" is a **runtime production** problem, not a missing-catalog problem.
- `manufacture_pass` (`tick.rs:4775`) makes good G at hub H only if **every recipe input
  is physically in H's stock this tick**: `made = min(inputs_available/qty, labor_cap)`,
  `labor_cap = (pop/median_pop)·labor`. Consequences:
  1. **Inputs must already be there.** They arrive only via *opportunistic spot dispatch*
     driven by `add_manufacturing_demand` (`:4873`) — which competes with all other demand
     and fails whenever trade is thin, cities are hungry, or the (straight-line) reach is
     short. No guaranteed input supply → stochastic, often-zero output.
  2. **Multi-input recipes need ALL inputs at once.** Missing one → output 0.
  3. **Big-city gated.** `labor_cap ∝ pop/median_pop`, so only large cities make real
     volume; most cities (small, hungry) make ~nothing.
- **Futures today are consumption-side only** (`form_contracts`, `tick.rs:6464`): a merchant
  HOUSE supplies a *finished* good to a deficit *consumer* office-city. There is **no
  procurement-side contract** guaranteeing a manufactory's INPUT inflow, and manufactories
  themselves never initiate contracts.

### 6.2 Plan: guarantee input flow + traders hunt for goods
**A. Procurement (input-side) futures.** When a hub has manufacturing capacity for G but a
   chronic INPUT shortfall of I (track a smoothed `input_lack[hub][I]`), a house with an
   office there signs a *procurement* futures contract to buy I from the nearest reachable
   surplus source — reusing the `form_contracts` machinery but with buyer-need = derived
   manufacturing demand for I, flagged as an input contract. This is literally "manufactures
   are urged to have futures contracts so there is a stable goods flow." Buyer of record =
   the house (or the city council as fallback), so a manufactory's raws arrive on schedule.

**B. Traders actively seek scarce input goods.** A house whose manufacturing (or whose
   offices' manufactories) is starved of I will, in order of cost:
   1. **prioritise dispatch/arbitrage** of I toward the starved manufacturing hub;
   2. **open a new trade tie / route** to a reachable producer of I (real pathfound route,
      §4.3) — this is where "new trade routes get established" comes from;
   3. if no reachable source exists in-component, **sponsor an OUTPOST or resource COLONY**
      at a site that produces I — tie outpost/colony site-selection to the *scarce input
      good*, not just generic reach (extends the existing colony/outpost system).

**C. Make manufacturing more robust.** Warehouse-buffer inputs so a lean week doesn't zero
   output; optionally lower the big-city gate so mid cities craft secondary goods
   (specialisation vs. spread — a design choice, see questions). Surface a per-hub
   "why isn't X being made?" limiting-factor (missing input vs. labour vs. demand).

### 6.3 Synergy with the population/network plan
Procurement futures + trader-seeking **depend on the pathfound route matrix (§4.3)** so the
new routes are real, not straight lines; resource colonies **feed the growth engine (§4)**
by planting new population centres where a coveted input is produced. The "Cold Start" flow
(§4.4) then lets this whole supply-chain web assemble itself from zero on unpause — exactly
the "watch the world build trade and relations" experience.

---

*File kept as the living design record for this work (per user: "keep info for the fix in
the future"). Update with test trajectory numbers and decisions as they land.*
