# Expeditions & Corridors — design plan

Rethinks how long-haul trade corridors come into being. Today a corridor is drawn
instantly from `flow_year` with fixed-spacing marker "waystations", ignores
geography (dips over water), can't be inspected, and recomputes 24 coarse-Dijkstra
routes **every year** (the year-start lag). This plan replaces that with corridors
that are **earned by expeditions**, founded as **real port / caravanserai
villages**, and told as a **story**.

Merges with the existing colony logic → one **"Expeditions & Colonies"** panel.

---

## 1. Core idea: a corridor is EARNED, not drawn

A permanent trade corridor between two distant cities only exists after merchants
have **repeatedly** made the journey and survived. The pipeline:

```
opportunity  →  house finances an EXPEDITION  →  it travels (visible on map)
   →  hazards cull it (illness/climate/natives/wreck)  →  arrives (partial) or FAILS
   →  repeated successful attempts accumulate  →  CORRIDOR established
   →  ports (coast) + caravanserais (land) founded as real villages
```

## 2. Expedition — data model (`sim/tick.rs`)

```rust
struct Expedition {
    id: u32,
    house: usize,              // backer (pays, profits)
    leader_name: String,       // generated (names.rs) — "led by Doran of House X"
    origin_hub: usize,
    dest_hub: usize,
    path: Vec<usize>,          // coarse-grid nodes (geographic least-cost route)
    launched_tick: u32,
    pos: f32,                  // 0..1 progress along path (advances per tick)
    outbound: bool,            // true = heading to dest, false = returning
    caravans: u16,             // land transport units
    ships: u16,                // sea transport units
    cargo: Vec<(u16, f32)>,    // (good, qty) bought at origin
    cost: f32,                 // capital committed at launch (registered)
    revenue: f32,              // accrues on arrival (sold at dest / back home)
    status: ExpeditionStatus,  // EnRoute | Arrived | Returning | Succeeded | Failed
    arrived_frac: f32,         // fraction of units still alive (1.0 → 0.0)
    hazards: Vec<HazardEvent>, // struggle log → narrative
}
enum ExpeditionStatus { EnRoute, Arrived, Returning, Succeeded, Failed }
struct HazardEvent { tick: u32, x: u32, y: u32, kind: HazardKind, losses: f32 }
enum HazardKind { Illness, Climate, NativeRaid, Storm, Wreck, Starvation, Bandits }
```

Per **city-pair** we keep an attempt ledger so establishment takes time:

```rust
struct RouteProspect {
    a: usize, b: usize,
    attempts: u16, successes: u16,
    cum_profit: f32,           // toward the establishment threshold
    last_tick: u32,
    established: bool,
}
```

All appended to the `CampaignSim` blob (serde-defaulted → old saves load).

## 3. Launch — clear, expensive, registered

At the yearly hook a wealthy house may **propose** an expedition toward the best
untapped opportunity: a distant hub pair with high `market`-implied gain, no
established corridor, and a viable geographic path.

- **Route & mode** from the shared coarse cost grid (`cached_coarse_cost` +
  `coarse_dijkstra`) — rivers, mountain passes, coast-hugging sea lanes. Sea legs
  ⇒ ships, land legs ⇒ caravans. Geography is authoritative: the path never cuts
  across open water except where a sea lane genuinely shortcuts (reach-gated).
- **Fleet size (randomised, formula-driven — not fixed):**
  ```
  units = round( base · (distance_km / REF_KM) · (cargo_value / REF_VALUE)
                 · house_wealth_factor · rand(0.7..1.3) )
  caravans = land-leg share · units ; ships = sea-leg share · units
  ```
- **Cost** = fleet size · per-unit outfitting · (1 + terrain hostility + sea risk),
  **debited from house wealth** and recorded in the house ledger + city finances as
  an "expedition venture" line (so it shows in the Accountant view). Expensive
  enough that only rich houses attempt it and a failure hurts (limited-liability
  capped, like contracts).
- **Cargo**: bought at origin from its chief exports (draws down origin stock,
  pays the origin) — real goods, valued for the profit calc.

## 4. Travel — visible on the map, over real time

`pos` advances each tick by `speed / path_len` (ships faster on open water,
caravans slower over mountains/desert). While `EnRoute`/`Returning` the expedition
is **drawn on the map** as a small caravan/ship icon with a progress bead along its
actual path. This is the "attempts on the map" the design calls for — you watch
ventures crawl toward new lands years before any corridor line appears.

## 5. Hazards — expeditions fail, and you see why

Each tick, for the cell the expedition currently occupies, roll hazards weighted by
what it is crossing (all data already on the world/campaign):

| Hazard      | Driven by                                                       |
|-------------|-----------------------------------------------------------------|
| Illness     | `disease` field + duration en route + crossing dense regions    |
| Climate     | Köppen hostility (BWh desert, ET/EF polar, high elevation lapse) |
| NativeRaid  | low political control / wilderness (no nearby friendly hub)      |
| Bandits     | long land legs far from any settlement or waystation             |
| Storm/Wreck | sea legs · storm band latitude · shipworm/reef risk              |
| Starvation  | long stretch with no waystation/port to resupply                |

Each hit removes a fraction of units (`arrived_frac` decays) and pushes a
`HazardEvent`. If `arrived_frac` hits 0 → **Failed**: a faded ✕ marker is dropped
at the failure site (cause on hover), the attempt is logged, and the house eats the
loss. Otherwise it **Arrives** (partial), sells cargo, and **Returns** (a second
hazard gauntlet) to bank the profit.

## 6. Establishment — repeated success

A `RouteProspect` becomes an **established corridor** only when
`successes ≥ MIN_SUCCESSES` **and** `cum_profit ≥ EST_THRESHOLD` (a few good round
trips). Until then, only the moving expeditions + failed-attempt ✕'s are visible —
the corridor line itself appears when the route is proven. On establishment:

1. A persistent `Corridor { a, b, path, owner_house, good, founded_tick,
   founding_expedition, attempts, successes }` is cached on the sim.
2. **Waystation villages are founded as real hubs** (see §7).
3. The pair now feeds `flow_year` normally (steady trade), owner = the backer.

## 7. Ports & caravanserais — real geographic villages

The route's markers stop being cosmetic dots. Along an established corridor we
found actual small hubs (reusing the satellite/caravanserai hub machinery already
in tick.rs), **placed by geography**:

- **Port** (`kind = port`) — on a genuine **coast cell** where the path meets/leaves
  the sea (a sea leg's landfall). Never in open water (fixes the "ports over water"
  bug: today markers land on the cheapest offshore cell; ports must snap to the
  adjacent land/coast cell).
- **Caravanserai** (`kind = caravanserai`) — on a defensible **land** cell near a
  water source, at a day's-march interval on long land legs.
- **Count is formula-driven + randomised** (not the current fixed `path.len()/6`):
  ```
  n_ports        = number of distinct sea↔land transitions on the path
  n_caravanserai = round( land_km / DAY_MARCH_KM · rand(0.75..1.25) )
  ```
  So a short river hop gets none, a long Silk-Road land haul gets several, a sea
  route gets ports at each landfall. Each village seeds small (like
  `CARAVAN_SEED_POP`) and grows with the corridor's traffic; it inherits the
  nearest hub's culture at founding (fixes blank-culture outposts).

## 8. Panel — merged "Expeditions & Colonies"

The colony panel gains an **Expeditions** section (and corridors move here from the
overlay-only state). Per active/established route it shows the story:

- Backer house + **leader**, why (the trade opportunity: good + expected gain).
- Fleet: **caravans / ships sent vs arrived**, over how many attempts.
- Goods **bought at origin / sold at destination**, cost vs profit.
- A generated **narrative of struggles** from the `HazardEvent` log
  ("Half the caravans lost to fever in the Saltmarsh; the third camp was raided by
  hill-folk; two ships foundered off the cape…").
- Ports & caravanserais founded (with links to those hubs).

Clicking a corridor / expedition on the map opens this panel (adds the missing
hit-test: nearest path segment within N px → select).

## 9. Lag fix (folded in)

- **Corridors become event-driven.** They're computed **once, when established**,
  and cached on the sim (`sim.corridors`). The overlay just reads the cache — the
  per-year 24-Dijkstra recompute (`campaign_get_corridors`) is **deleted**. This
  removes the main new year-boundary cost.
- Expedition routing (one Dijkstra per *newly launched* expedition, a few per year)
  replaces it — far cheaper and spread out.
- Frontend: the corridor/flow overlay effects read cached data, so they no longer
  fan out heavy work on the year tick.

## 10. Outpost graduation / culture (issue 3 tie-in)

Superseded by the expedition model for *why* remote settlements appear, but two
standalone fixes still apply and are cheap:
- **Culture at founding** — every newly founded hub (outpost, satellite,
  caravanserai, colony, port) is assigned a culture immediately
  (`ensure_hub_cultures` currently runs before they're founded, leaving them blank
  for a year). Assign from nearest cultured hub / hearth at creation.
- Keep graduation earned (do **not** blanket-promote); the corridor establishment
  path is now the "clear rule" for a post growing into a town.

---

## Implementation order

1. **Data model + serialization** — `Expedition`, `RouteProspect`, `Corridor`,
   `HazardEvent` on `CampaignSim` (appended, defaulted).
2. **Launch + routing + cost/registration** at the yearly hook (opportunity pick,
   fleet formula, ledger debit).
3. **Per-tick travel + hazard rolls** (advance `pos`, decay `arrived_frac`, log).
4. **Establishment + waystation-village founding** (geographic port/caravanserai
   placement, culture-at-founding).
5. **Delete** `campaign_get_corridors` per-year recompute → serve `sim.corridors`;
   add `campaign_get_expeditions` (moving + failed attempts).
6. **Frontend**: draw en-route expeditions + failed ✕'s + established corridors
   from cache; hit-test to open the panel.
7. **Panel**: Expeditions section on the colony panel (story + fleet + goods +
   narrative + founded villages).
8. **Sim-test** (`simulate_decades_reports_dynamics`) — expeditions launch, some
   fail, corridors establish over years, wealth stays bounded; then push to `main`.

## Tunables (all in `tick.rs`, sim-tested)
`EXP_START_YEAR`, `EXP_MIN_HOUSE_WEALTH`, `EXP_REF_KM`, `EXP_REF_VALUE`,
`EXP_UNIT_COST`, hazard base rates per `HazardKind`, `MIN_SUCCESSES`,
`EST_THRESHOLD`, `DAY_MARCH_KM`, port/caravanserai seed pop.
