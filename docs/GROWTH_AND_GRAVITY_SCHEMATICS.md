# Schematics — 150k top cities + trade gravity for large hubs

## 1. Population ceiling → large cities reach ≥150k

**Before (absolute, mis-calibrated):** `trade_dev` depended on an unknown absolute trade
scale, so when trade was thin it was ~0 and every city froze near ~28k.

```
                         trade_last_year
        trade_dev = ───────────────────────────   (0 when trade thin → STALL)
                     founding_pop · TRADE_DEV_REF
```

**After (relative to the world's busiest hub):** the top entrepôt always earns the FULL
headroom; lesser hubs earn their *share* of it.

```
   world_max_trade = max over hubs of trade_last_year        (this year)

                 ┌  TRADE_DEV_CAP · (trade_last_year / world_max_trade)   if any trade
   trade_dev  =  ┤
                 └  0                                                     otherwise
                                                     (TRADE_DEV_CAP = 20)

   cap_mult  = (0.35 + 1.30·food_sec) · (0.60 + 3.0·prosperity² + trade_dev)
   capacity  = founding_pop · cap_mult          (min 0.15·founding)
   pop_{t+1} = logistic(pop → capacity) + (BIRTH_RATE·food_sec − DEATH_RATE)·…
```

**Why the top cities now clear 150k** (large-founding hub = 10,000):

| food_sec | prosperity | trade_dev | cap_mult | capacity |
|---------:|-----------:|----------:|---------:|---------:|
| 0.7 | 0.6 | 20 (busiest) | ≈ 1.65·(0.6+1.08+20)=**35.8** | **≈ 358k** |
| 0.5 | 0.4 | 20 (busiest) | ≈ 1.00·(0.6+0.48+20)=**21.1** | **≈ 211k** |
| 0.3 | 0.4 | 20 (busiest) | ≈ 0.74·(21.08)=**15.6** | **≈ 156k** |

So even a moderately-fed busiest hub clears 150k; a thriving one approaches ~350k. Smaller
hubs (low `trade_last_year/world_max`) get little `trade_dev` and stay small → concentration
preserved. *Knob: `TRADE_DEV_CAP`.*

> Note: capacity is still anchored to `founding_pop` (large cities start at 10k). Letting a
> small-founding town that becomes a nexus also balloon needs a founding-independent base —
> tracked separately (megacity engine §8 / food redesign 1b).

## 2. Trade gravity — big hubs attract trade from afar & draw merchants

`hub_pull(b)` ≥ 1 grows with a hub's class and population:

```
   hub_pull(b) = clamp( 1 + HUB_PULL_CLASS·hub_class(b)
                          + min(pop(b)/HUB_PULL_POP_REF, 1),
                        1 .. HUB_PULL_MAX )
   e.g. entrepôt (class 2, 60k) → 1 + 0.7·2 + 1 = 3.4  (capped 3.5)
        plain town (class 0, 3k) → ≈ 1.06
```

**(a) Reaches farther — partner selection uses EFFECTIVE distance:**
```
   rebuild_neighbors:  rank candidates by   days(a→b) / hub_pull(b)
                                            └─ big hub looks ~3× nearer ─┘
   → a great entrepôt enters the K-nearest partner list of cities up to
     ~3× farther away than an ordinary town would.
   (Freight/sale still use the REAL days — only WHO trades changes, not cost.)
```

**(b) Preferred by merchants — arbitrage shortlist weighted by pull:**
```
   for each surplus seller a, each reachable buyer b:
       gap = price_b − (price_a + freight) − margin
       score = gap · hub_pull(b)          ◄── gravity
   keep top-3 by score, ship there
   → among profitable destinations, the great markets win the cargo.
```

**Net effect (schematic):**
```
        small town ─┐         ┌─ small town
        small town ──┼──▶  ★ ENTREPÔT  ◀──┼── distant town   (pulled in from afar)
        distant town ┘     (high pull)    └─ small town
              trade converges on the great hubs; they grow (§1) → higher pull → more trade
              (a positive feedback, bounded by HUB_PULL_MAX and the capacity ceiling)
```

## Files touched
- `sim/tick.rs`: `TRADE_DEV_CAP`/`HUB_PULL_*` consts · `update_food_and_starvation`
  (`world_max_trade` + relative `trade_dev`) · `hub_pull()` · `rebuild_neighbors`
  (effective distance) · arbitrage target scoring (pull-weighted shortlist).

## What to check in-app
- [ ] The largest 2–3 trade cities reach **≥150k** over a long campaign.
- [ ] Trade **converges on big hubs** — distant cities ship to the great entrepôts.
- [ ] Merchants/houses cluster in and around the great markets.
- [ ] Economy stays bounded (no runaway), house/bank/war turnover still healthy.
- ⚙️ Too few reach 150k → raise `TRADE_DEV_CAP`. Gravity too strong (everything funnels to
  one hub) → lower `HUB_PULL_MAX` / `HUB_PULL_CLASS`.
