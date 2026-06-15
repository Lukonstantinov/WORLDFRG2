# Futures Contracts + House Warehouses — Design Plan

Campaign-economy batch. Adds **house-owned warehouses with finite capacity** and a
**futures-contract layer** on top of the existing spot market (`sim/tick.rs`
`dispatch`). Goal: give settlements *forward supply security* (they can no longer
only trade reactively when a surplus happens to sit nearby this tick), smooth
prices, and deepen house strategy (own the source → seat an office → lock the city).

See [[worldforge-2-project-state]], [[worldforge2-c-batch]], [[worldforge2-famine-fix]].

---

## Locked design decisions (from the design conversation)

| Topic | Decision |
|---|---|
| Inventory model | **Real house inventory** — houses own per-good stock, not just the city pool |
| Capacity | **Single total cap** per warehouse (Σ goods ≤ capacity) |
| Expansion | **Automatic** — AI houses auto-invest (extend `maybe_house_invests`) |
| Upkeep (warehouse) | **Capacity-scaled**: `CAP_UPKEEP · capacity · city_size_factor(hub)` |
| Upkeep (family) | **+ wealth-proportional progressive overhead** (new) → home `civic_pool` |
| Ownership scope | **End-to-end** — every good lives in SOME owner's warehouse; the old city pool becomes the **local-merchants (`owner −1`) warehouse** |
| Famine guardrail | **Subsistence floor** — residents draw food before any house may export/warehouse it (moves `FOOD_RESERVE_DAYS` to hub level) |
| Contract terms | **1 / 3 / 5 / 7 yr**, gated by the SELLER's stable-growth track record |
| Term pricing | longer term → cheaper unit strike, **larger** break penalty |
| Coverage cap | contracts ≤ **~25 %** of a city's structural need per good (tunable) |
| Price band | strike `P0` at signing, paid `Pt` drifts toward spot within **±10–15 %** |
| Tiers | 1 Depot · 2 Storehouse · 3 Warehouse · 4 Entrepôt · 5 Grand Entrepôt |
| Disasters | **burn** (100 % stock, →Tier 1), **damage** (≤80 % stock OR capacity, may demote), **plague lockup** (city quarantine, contracts force-majeure-suspended) |
| Futures overlay | **separate layer** from merchant routes (C5) |
| House-network highlight | on house click: colour squares at traded settlements + **red glowing** edges |

---

## Data model (all `#[serde(default)]`, appended LAST → old `.campaign` saves load)

```rust
struct Warehouse {
    owner: i32,            // house index, or −1 = local merchants (the old city pool)
    hub: u32,
    capacity: f32,         // single TOTAL cap; −1 pool effectively uncapped
    stock: Vec<f32>,       // per-good, owned by this warehouse
    tier: u8,              // 1..5, derived from capacity band
    damage: f32,           // 0..1 structural damage (repairs over time)
}

struct Contract {
    seller_house: u32,
    buyer_hub:    u32,
    source_wh:    usize,   // index into warehouses (the supplying depot)
    good:         usize,
    monthly_qty:  f32,     // SMALL — coverage-capped slice of buyer need
    strike_price: f32,     // P0 at signing (grain-equivalent numeraire)
    term_years:   u8,      // 1 | 3 | 5 | 7
    start_tick:   u32,
    end_tick:     u32,
    delivered:    f32,
    suspended_until: u32,  // force-majeure (plague lockup); 0 = active
    defaults:     u8,
}

// CampaignSim gains:
//   warehouses: Vec<Warehouse>
//   contracts:  Vec<Contract>
```

`hub_stock(hub, good)` for `live_price` / needs = **Σ over warehouses at that hub**.
This preserves all current price-formation and famine logic on the AGGREGATE while
goods are individually owned.

---

## Tiers (capacity bands)

| Tier | Name | Capacity ≤ | Construction | Burn risk |
|---|---|---|---|---|
| 1 | Depot | 600 | wood | high |
| 2 | Storehouse | 1,500 | wood | high |
| 3 | Warehouse | 3,000 | timber + tile | med |
| 4 | Entrepôt | 6,000 | stone | low |
| 5 | Grand Entrepôt | 12,000 | stone | low |

Tier is derived from `capacity`. AI expansion raises capacity → promotes; damage
lowers capacity below a band floor → demotes.

---

## Contract term eligibility (seller's track record)

`stable_growth_years(house)` = trailing run of years where
`wealth[y] ≥ wealth[y-1]·(1−tol)` (from `House.wealth_history`, E-batch #7), broken
by any bankruptcy / default / feud-loss in the window.

| Term | Requires stable-growth yrs | Strike vs spot | Break penalty |
|---|---|---|---|
| 1 yr | eligible (office at buyer + solvent) | +2 % | low |
| 3 yr | ≥ 4 | 0 % | medium |
| 5 yr | ≥ 7 | −3 % | high |
| 7 yr | > 10 | −5 % | very high |

Checked **at signing only**. Term length self-selects for reliability (only proven
houses can offer long lanes → long contracts are inherently low-default-risk).
Guilds (civic, never bankrupt) naturally accumulate stable years.

---

## Disasters (extend `roll_events`, `tick.rs:2048`)

- **🔥 Burn** — targets a specific house warehouse: **all stock destroyed**, building
  gutted → **drops to Tier 1**; capacity regrows via AI reinvestment. Higher tiers
  (stone) have lower burn probability.
- **🪓 Damage** — severity `s ∈ (0, 0.80]` applied to **stock OR capacity**. Capacity
  loss below a tier floor → **demote**. `damage` field repairs over time.
- **☣ Plague lockup** — quarantines a hub for ~60–180 days: every trade leg touching
  it (spot + contract) is skipped. Contracts to/from it are **force-majeure
  SUSPENDED** (`suspended_until`) — **no default penalty**, missed deliveries waived.
  Couples to famine via the subsistence floor (no imports → local draw-down).
  Tagged `kind:"disaster"` (kept in chronicles, like E-batch disasters).

A burned/damaged warehouse that then **misses a contract `monthly_qty` → SELLER
DEFAULT** (penalty + reputation hit). Plague suspension does NOT default — that's
the distinction.

---

## Upkeep changes (`apply_wealth_sinks`, `tick.rs:1370`)

1. **Warehouse upkeep** → `CAP_UPKEEP · capacity · city_size_factor(hub)` (replaces
   the flat per-warehouse `UPKEEP_WAREHOUSE_BASE`). Big hoards in one city get
   expensive → pushes offices + contracts elsewhere.
2. **Family wealth overhead** (new) → `WEALTH_UPKEEP_RATE · max(0, wealth −
   WEALTH_UPKEEP_FREE)`, progressive. Routed to home `civic_pool` (patronage →
   funds public works/festivals). Caps cash-hoarding; modest rate so it limits
   hoarding, not growth. Tunable.

---

## Contract lifecycle in the tick

New `fulfill_contracts(needs)` runs **before** `dispatch`:
```
for each ACTIVE (not suspended) contract:
    if buyer_hub or source quarantined → set suspended_until, skip (no penalty)
    qty = monthly_qty
    Pt  = clamp(0.7·P0 + 0.3·spot_now, P0·(1−band), P0·(1+band))   // term-strike P0
    if source_wh lacks qty → SELLER DEFAULT (penalty, rep hit, journal)
    else reserve qty from source_wh.stock  (before spot sees it)
         ship via existing fleet/freight → buyer warehouse; pay/charge Pt
spot dispatch() then arbitrages only what's LEFT  (coverage cap keeps it alive)
```

Contract **formation** (monthly, deterministic hash-seeded): a seated house/guild at
a city with a CHRONIC deficit in good `g` AND an owned source offers a contract for
the longest term it qualifies for, capped at `coverage_cap · need`.

---

## UI

### Warehouses subtab (BOTH House/Guild detail AND Settlement market view)
- Per warehouse: iso building glyph (reuse `settlementArt.ts`) sized/tinted by tier,
  single total fill bar, per-good chips, tier name, damage/burn state.
- House/Guild: "supplies → City (good · ends Yr N)" + click → Futures layer filtered
  to that warehouse.
- Settlement: lists every warehouse sited there (houses, guilds, local `−1` pool) +
  per-warehouse **import ← / export →** city links + GUILD tag.

### Futures overlay (SEPARATE from merchantRoutes C5)
- `campaign_futures_lanes` query → directional contract lanes
  `{from_hub, to_hub, good, term, end_tick, qty}`.
- `OverlayManager.drawFutures`: **dashed + arrowhead**, width ∝ qty, colour/weight ∝
  term (1yr faint → 7yr gold/bold), label `🍷 Yr08→Yr15`.
- `uiStore.overlays.futures` (default off), Toolbar "📜 Futures" toggle, bridge
  wrapper, MapCanvas fetch on contract change.

### House-network highlight (on house click only)
- `campaign_house_network(house_id)` → settlements (home + offices + warehouses +
  estates + `trade_at` ties) as points + edges among them.
- Frontend: while a house is selected in HouseDetail, draw small **house-colour
  squares** at those settlements + **red glowing** edges. Cleared on deselect.
  Independent of the Futures + merchant-route overlays.

---

## Guardrails that keep the economy intact

1. **Coverage cap** (~25 %): contracts never absorb the whole market → spot price
   signal, house competition, and the wealth/monopoly engine stay live.
2. **Price band**: no decade-long mispricing as `tech_factor`/prices drift.
3. **Two-sided risk**: seller reserves stock (opportunity cost + default penalty),
   buyer locked in (pays even if cheaper spot appears); freight still paid.
4. **Subsistence floor**: residents eat before houses export/warehouse food — the
   non-negotiable protection for the famine balance ([[worldforge2-famine-fix]]).
5. **Force-majeure**: plague lockup suspends, never defaults.

---

## Phasing (de-risked rollout)

1. **Warehouse object + ownership refactor.** Reframe city pool as the `−1`
   warehouse; `hub_stock` = Σ warehouses; subsistence food floor. **Ship + re-verify
   famine tests (`food_surplus_prevents_famine_collapse`, `cutting_food_starves…`)
   BEFORE going further.**
2. **House inventory + tiers + capacity-scaled & wealth-proportional upkeep + AI
   expansion** (`maybe_house_invests`). Burn/damage events.
3. **Contracts**: term ladder 1/3/5/7 gated by stable-growth, banded price,
   term-scaled penalties, office-gated formation, `fulfill_contracts` before
   `dispatch`. Plague-lockup force-majeure suspension.
4. **UI**: Warehouses subtab (both views), Futures overlay, house-network click
   highlight.

Each phase: `cargo check` + `cargo test --lib` + `npx tsc --noEmit` clean before
moving on. Per house convention: compile-verified, NOT visually verified.

---

## Save compatibility

`warehouses` + `contracts` + new `House`/`Warehouse` fields all `#[serde(default)]`,
appended last. Old campaigns load with empty warehouses → a migration on open seeds
one `−1` local-merchant warehouse per hub from existing `hub.stock` (the pool), so
nothing breaks. New `tick.rs` consts (CAP_UPKEEP, WEALTH_UPKEEP_RATE, coverage cap,
price band, term thresholds) are tunable.
