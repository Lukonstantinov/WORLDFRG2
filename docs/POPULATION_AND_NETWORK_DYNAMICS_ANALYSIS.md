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
- **Slice 3 — procurement futures (IN PROGRESS).**
  - ✅ `form_contracts` now considers, beyond a house's speciality goods, any
    **manufacturing INPUT the office-city is structurally short of** (`is_input` set ×
    `production < 0.8·need`, with manufacturing demand already folded into `needs`). A
    house therefore signs PROCUREMENT futures to keep a manufactory's raw supply steady.
    Source selection / sizing / liability are unchanged, so a contract only forms when a
    house network node can actually bridge input-source → workshop (safe by construction).
  - ✅ *Resource colonies (slice 3b, part 1):* `maybe_found_house_outpost` now biases the
    outpost's produced good toward a SCARCE manufacturing input the founder's own network
    barely makes (`OUTPOST_INPUT_BIAS`), so a rich house plants a raw-materials resource
    colony to feed its workshops — closing the loop with procurement futures. Tests green.
  - ⏳ Remaining 3b (dispatch-priority + office-expansion toward scarce inputs) and
    manufacturing-gate softening (3c): planned below.

---

## 7. Thorough implementation plan (remaining slices)

### Slice 1b — decouple "at food capacity" from "starving"  *(BLOCKED on redesign)*
**Problem:** `sent_food = 1 − starving`; a structurally food-poor site keeps `food_balance
< 0` at any size (local food = per-capita × pop, so the deficit ratio is constant), so it
racks up permanent famine that pins its growth signal low forever.
**⚠️ Attempt 1 (reverted — commit history):** gating capacity by
`food_cap = pop·(1 + food_balance)` is **degenerate**. Because local food production scales
∝ pop, `food_balance` is *scale-invariant* (`(per_capita_food − per_capita_need)/…`,
independent of pop), so `food_cap` sits just under pop every tick and drives food-poor
cities all the way to the floor instead of to an equilibrium — the same bug, relocated.
It also silently relieved dearth (shrinking cities), which broke `unrest_topples_councils`
(chronic-dearth revolt no longer fired). **Lesson: any food ceiling MUST be absolute, not
∝ current pop.**
**Correct approach (do this):**
1. Give each hub an **absolute local food capacity** `food_hinterland` (fertility/climate
   of its site, seeded at campaign start; a proxy is `founding_pop · LOCAL_FOOD_MULT`).
   Cap LOCAL food *production* at that absolute number (not per-capita × pop).
2. Then `food_cap_pop = (food_hinterland + food_imported) / per_capita_food_need` is a
   real, pop-independent ceiling; `capacity = capacity.min(food_cap_pop)`. Now imports
   genuinely extend a city past its fields, and a food-poor town settles at a stable small
   size with `food_balance ≈ 0` (content), while still able to revolt on inequality.
3. Only a deep, sustained deficit accrues `starving` (famine = shock, not equilibrium).
**Test:** dynamics green AND `unrest_topples_councils` still fires (verify both).

### Slice 2 — pathfound route-days matrix (never a straight line)  *(cross-file)*
**Problem:** `rebuild_routes` uses Euclidean `dist × days_per_cell`.
**Steps:**
1. At campaign start (`campaign_commands.rs::start_campaign`, which HAS db/tile access),
   build a real cost grid (reuse `query_commands::compute_trade_routes` machinery:
   passes / rivers / coast-hugging / reach-limited sea) and pathfind route-days between
   every pair of the 250 live hubs. Serialize as `base_days: Vec<f32>` + a stable
   `hub_id → row` map on `CampaignSim`.
2. `rebuild_routes` reads `base_days` for founding hubs; for hubs added mid-sim (colonies)
   route **through the parent** (`days[new][x] = dist(new,parent) + base_days[parent][x]`)
   — always a real chained path, never an arbitrary straight jump.
3. Keep the component/reachability gate; "reachable" ⇔ finite pathfound days.
4. Feed the same routed polylines to the campaign trade-route overlay so the map shows
   real lanes.
**Test:** dynamics green; spot-check a known mountain pair routes around, not through.

### Slice 3b — trader input-seeking + resource colonies  *(tick.rs)*
**Steps:**
1. Per house, compute its **scarce manufacturing inputs** (offices where `is_input[g]` &
   `production < need`). Cache yearly.
2. **Dispatch priority:** bias arbitrage so a house ships a scarce input toward its own
   starved workshop first.
3. **New routes:** when the input's nearest producer is reachable but un-officed, bias
   office expansion (`update_guilds_and_offices`) to plant an office there → a new lane.
4. **Resource colony:** in `maybe_found_house_outpost`, when a scarce input matches a
   colonizable site's `kind_hint`, prefer founding the outpost to produce THAT input
   (resource colony), shipping it back to the workshop.
**Test:** dynamics green; watch "contracts" and finished-good output rise; new outposts
tagged to inputs.

### Slice 3c — manufacturing robustness  *(tick.rs)*
1. Warehouse-buffer inputs so a lean week doesn't zero output.
2. Gently soften the labour gate (`(pop/median).max(FLOOR)`) so mid cities craft 1–2
   secondary goods — keep big-city concentration for the high-labour luxuries.
3. Emit a per-hub "limiting factor" (missing input / labour / demand) for the UI.

### Slice 4 — hinterland wake/sleep + Cold Start  *(cross-file + frontend)*
1. **Wake/sleep:** give `hinterland` towns a cheap yearly growth toward a small local cap;
   when one crosses a threshold, promote it to a live `TickHub` (and demote a long-dead
   hub to sleep) to keep the live set bounded.
2. **Cold Start:** a new campaign-start flag that seeds tiny populations and **zeros** all
   trade ties, warehouses, houses, coin, wealth and the route network; on unpause the
   emergence order (merchants → guilds yr5 → houses yr10) rebuilds everything from nothing.
   Backend command + a WorkflowPanel toggle; default start unchanged.
**Test:** dynamics green; in-app cold-start run shows the network self-assembling.

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

## 8. The megacity engine (>1M) — historical analysis → sim design

**Why it must be RARE.** Almost all premodern cities capped at 10k–100k; even great trade
republics (Venice, Bruges) topped ~150–200k. Only a handful ever passed a million (Rome
~1M; Han/Tang Chang'an, Song Kaifeng, Ming/Qing Beijing ~1M; Baghdad, Constantinople). A
million-person city needs the **conjunction** of five engines — miss one and it falls back
to ~150k. That conjunction being rare is exactly why >1M should be rare in the sim.

| # | Historical engine | Current sim analogue | What to add |
|---|---|---|---|
| 1 | **Political primacy** — the capital *commands* tax/tribute grain, doesn't merely buy it (Rome's *annona*; Chinese tax-grain) | poleis treasury, `govt_type`, influence, wars/levies | **Capital/primacy multiplier** on carrying capacity for the dominant polis of a region (top treasury+influence): it can command tribute-grain, so its ceiling is far higher than a pure trade city's |
| 2 | **Secured bulk grain by WATER from *multiple* breadbaskets** (sea lanes / Grand Canal; ~20× cheaper than land) | colony supply-ship lifeline (`designate_colony_supply`), bulk/perishable freight | **State grain fleet ("annona")**: a capital runs standing, secured supply from several water-connected surplus provinces; this is what LIFTS the food ceiling. Land-locked hubs can't do it → cap lower |
| 3 | **Granary storage** against convoy breaks (*horrea*, state granaries) | `reserve_food` / `reserve_cap`, granary starvation guards | Scale reserve capacity with primacy/infrastructure so a great capital buffers months, not days |
| 4 | **Constant in-migration** overcoming the urban "graveyard" (deaths>births in dense cities) | `economic_migration_pass` (homophily, trade-tie chains) | **Urban death-sink**: give large/dense cities a mild NEGATIVE natural rate, offset only by in-migration — makes migration the true growth engine of big cities (as in history) and keeps >1M dependent on a live catchment |
| 5 | **Metropolitan system** — core + port/granary/workshop satellites as one unit (Rome+Ostia; Chang'an wards) | satellite construction (`SATELLITE_METRO_POP`=25k), absorption, hinterland | **Metro accounting**: a metropolis's satellites SHARE its food supply and their pop counts toward the metro's effective scale; a capital spins up more satellites as it grows |

**The rule to encode:** capacity for a hub can exceed ~1M **only** when ALL of: it is a
regional political CAPITAL (primacy) · it is WATER-connected (coastal/river/canal) · it runs
a SECURED multi-source grain supply (annona) with granary buffers · it has a live in-migration
CATCHMENT · it anchors a satellite SYSTEM. Encode each as a gate/multiplier so the product is
almost never all-on — a handful of cities per world, exactly like history. This layers on top
of the trade-earned ceiling already shipped (§5b slice 1): trade builds the great city; only
imperial primacy + secured water-grain pushes the rare one past a million.

**Dependencies:** needs the **absolute food-capacity redesign** (slice 1b) so the food ceiling
is real and liftable by imports; needs the **pathfound route matrix** (slice 2) so "water-
connected" and "secured lane" are meaningful; benefits from **dynamic hub roles** (§9) so
"which city is the capital/entrepôt" is known each year.

## 9. Dynamic yearly recalculation of hub roles (trade hubs · entrepôts · largest cities) — *FOR LATER*

Today the hub roles (trade power, entrepôt status, political primacy, largest-city ranking)
are computed **once** by the query-only worldgen passes (`compute_political`, `compute_economy`)
and are effectively static in campaign mode. **Plan:** add a cheap **yearly** pass in `advance`
(at New Year, alongside `sample_culture_history` / `flow_year`) that recomputes, from LIVE state:
- **Largest cities** — rank by live `population`.
- **Trade hubs** — rank by route-centrality × realized throughput (`trade_last_year`).
- **Entrepôts** — hubs whose transit/re-export volume dominates their own consumption.
- **Political primacy / capital** — top treasury + influence per region (component), feeding the
  megacity primacy multiplier (§8).
These feed: the growth ceiling (capital & entrepôt bonuses), coinage/mint eligibility, office
expansion targets, and the map's hub styling — so the network's "important places" **shift over
the centuries** as cities rise and fall, instead of being frozen at worldgen. Recompute cadence:
once per year (bounded cost). *Not started — scheduled after slices 1b/2.*

---

## 10. Settlement Development Ladder — 5-tier civilization progress bar *(DESIGN — pending approval)*

A per-settlement **development tier (0–5)** shown as a 5-segment progress bar with
milestones, expressing how *organised / civilised* a place is (distinct from `hub_class`,
which is only its **commercial rank**; this ladder blends many pillars). A settlement
advances by hitting milestones across pillars; the bar shows current tier + progress to the
next, with a tooltip listing each milestone ✓/✗ and current value vs threshold.

### Proposed tiers & milestones (illustrative thresholds, tunable)
| Tier | Name | Population | Government / Stability | Trade | Warehouse | Civic buildings | Extra pillar |
|------|------|-----------|------------------------|-------|-----------|-----------------|--------------|
| 1 | **Hamlet** | founding | — | subsistence | — | — | — |
| 2 | **Village** | ≥ 1,500 | not starving | ≥1 trade tie | Depot | — | — |
| 3 | **Town** | ≥ 6,000 | council seated, unrest low | ≥2 partners / active market | Storehouse | ≥1 (granary/well) | a **guild** forms |
| 4 | **City** | ≥ 25,000 | laws + officials, stable | trade hub (`hub_class`≥1) | Entrepôt | ≥3 | a **bank or mint** (finance) |
| 5 | **Metropolis** | ≥ 100,000 | dominant/capital, very stable | entrepôt (`hub_class`=2) | Grand Entrepôt | many + high public health | own **coinage + satellites** (the §8 megacity conjunction) |

### Pillars (user's list + "something else")
Population · Government stability (`govt_type`, `officials`, `laws`, `society.unrest`,
`sent_stability`) · Trade (`trade_last_year`, `hub_class`, partner count) · Warehouse
(`capacity_tier` of the biggest depot) · Civic buildings (`structures` count, `civic_goods`
granary, `public_health`) · **Extra pillar → propose Finance (bank/mint/coin) + Culture
(guild / books / lingua-franca seat)**.

### Mechanics
- Recompute the tier **yearly** (alongside `classify_hubs`) with **hysteresis** (sustained
  ~1 yr before promotion/demotion) so it's earned, not flickery. A tier can be **lost**
  (decline, war, plague) — the bar can go down.
- New state on `TickHub`: `dev_tier: u8` (+ optional `dev_progress: f32`), serde-defaulted.
- Frontend: a `TierBar` in `HubPanel` (segments + milestone tooltip) and a small tier badge
  on the map/settlement list.
- **Open choice:** is a tier purely *descriptive*, or does it **gate abilities** (e.g. mint
  unlocks at City, satellites at Metropolis)? — see approval questions.

### Ties to the rest
Tier 5 deliberately requires the megacity engine (§8), so the ladder and the >1M mechanic
reinforce each other; the tier badge also gives the map an at-a-glance "how developed is
this place" read that shifts over the centuries.

---

*File kept as the living design record for this work (per user: "keep info for the fix in
the future"). Update with test trajectory numbers and decisions as they land.*
