# City Stores — settlement panel view (schematic)

City-owned goods reserve (civic warehouse) + total goods held at the city, with valuations,
so the council/player can see how well-PROVISIONED and how RICH-IN-GOODS a city is — the
basis for shortage-buffering and population planning. Lives in the **Provision** tab of the
settlement window; a **development-tier badge** rides in its header.

```
┌─ SETTLEMENT: Ravenmoor ───────────────────── [Summary][City][Government][Provision][Trade] ┐
│                                                                                            │
│  ┌── 🏛 City Stores ───────────────────────────────────  [ Tier 4 · Free City ] ──┐        │
│  │  💰 Riches in goods: 84,200 gr-eq     📦 12,940 units held                       │        │
│  │  🍞 Food reserve: 3,180               🏛 Civic reserve: 9,050 gr-eq              │        │
│  │                                                                                  │        │
│  │  Richest goods held here                                                         │        │
│  │    🧂 Salt .................... 2,400            ⚔ Metalware ............. 1,120  │        │
│  │    🧵 Woolen Cloth ............ 1,850            🍷 Wine ...................  760  │        │
│  │    🌾 Wheat ................... 1,540            💎 Sapphire ..............   40  │        │
│  └──────────────────────────────────────────────────────────────────────────────┘        │
│                                                                                            │
│  ── Secured in the civic warehouse ──  (existing council reserve targets / bars) ──        │
│    🌾 Wheat 🍞   ████████░░  1,540 / 2,000                                                  │
│    🧂 Salt       ██████░░░░    900 / 1,500                                                  │
│    ...                                                                                      │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

## Data source (all live from the campaign sim)
`HubDetail.city_stores : CityStores` + `HubDetail.dev_tier : u8`, built in
`campaign_commands.rs` from:

```
 civic reserve   = hub.civic_goods            (city-owned stockpile)
 food_reserve    = Σ civic_goods[food] + hub.reserve_food
 held-at-city    = hub.stock (local pool) + Σ warehouses at this hub (house depots)
 goods_value     = Σ held[g]·base_value(g)    ← "riches in goods" (grain-eq)
 goods_units     = Σ held[g]
 top_goods       = held ranked by value (top 8)
 reserve/reserve_value = civic stockpile + its value
 dev_tier        = development_tier(hub)      (Outpost..Emporium)
```

```
        ┌ hub.stock (local merchant pool) ┐
held  = ┤ + house/guild depots at this hub ├──►  goods_value / units / top_goods
        └                                  ┘
civic = hub.civic_goods  ──►  reserve / reserve_value / food_reserve   (council-owned)
```

## Why it helps the council (population planning)
- **Food reserve** → how many lean seasons the city can weather → safe to keep growing, or hold.
- **Riches in goods** → the city's stored wealth beyond its coin treasury → capacity to fund
  works, weather war levies, or invest.
- **Top goods** → what the city is long/short on → informs tariffs, first-buy, and which trades
  to court (feeds the trade-gravity + procurement systems).

## Status
- ✅ Backend `CityStores` + `dev_tier` computed & exposed on `HubDetail` (cargo check clean).
- ✅ Frontend types (`CityStores`) + Provision-tab "City Stores" block with tier badge.
- ⏳ Later: make the reserve an explicit tiered civic warehouse building (capacity like house
  depots), council actively stocking non-food staples, and the yearly-hysteresis dev-tier
  persistence + ability gating.
```
