# In-App Verification Checklist — population, trade & manufacturing work

Run a **fresh full world → finalize → start a campaign → advance 200–500 years**, then
check each item. Note anything off next to it; I'll use these notes to fix/tune.

## A. Population growth (the 3.2M stall fix — slice 1)
- [ ] **Total world population TRENDS UP** over the centuries (census sparkline), not flat.
      *(Was: stalled ~3.2M by ~year 80–120.)*
- [ ] **A few trade hubs grow much larger than before** — climbing well past the old
      ~9× founding ceiling (a great port should reach hundreds of thousands, not freeze
      at ~25–90k).
- [ ] **Isolated / inland low-trade towns stay small** (they should NOT all balloon).
- [ ] Population stays **sane & bounded** — no city runaway to absurd millions everywhere,
      no craters to zero except real famine/war.
- [ ] Migration still flows **toward prosperous trade centers** (people leave poor towns).
- ⚙️ If growth too slow/fast → tune `TRADE_DEV_CAP` (max earned headroom, currently 15)
      and `BIRTH_RATE` (currently 0.00006) in `sim/tick.rs`.

## B. Manufactured goods & procurement futures (slice 3)
- [ ] **More manufactured goods are actually produced** (cloth, metalware, linen, silk
      brocade, jewelry, etc. show real output in big cities, not ~0).
- [ ] **Futures contracts form** and some are for **raw inputs** delivered to
      manufacturing cities (Coin&Credit / Futures panel) — not only finished goods.
- [ ] **Resource colonies / outposts** appear that produce a raw input the founding
      house's workshops were short of (an outpost tagged to iron/timber/fibre/dye, etc.).
- [ ] Manufacturing concentrates in **larger cities** (expected) but isn't totally absent
      elsewhere.
- ⚙️ If still too few manufactures → the likely remaining cause is inputs not reaching
      workshops (thin trade); note which good and which city.

## C. Trade network & routes
- [ ] **Most settlements participate in trade** (few permanently static dots) — note any
      cluster that never trades.
- [ ] ⚠️ **Routes still draw as straight lines** settlement→settlement (pathfound routes
      = slice 2, NOT yet implemented). Confirm this is still the case so we know it's
      pending, not broken.
- [ ] Trade hubs / entrepôts **change rank over time** (a rising city becomes a hub, a
      declining one loses status) — `classify_hubs` runs twice a year.

## D. Stability / regressions to watch
- [ ] No cities perpetually stuck "starving" that clearly *should* be fed (esp. coastal
      trade cities) — note them (feeds the food-capacity redesign, slice 1b).
- [ ] House / bank / coin / war / crash turnover still healthy (rise & fall).
- [ ] No obvious economic blow-ups or crashes-to-zero across the board.
- [ ] Performance still acceptable advancing many years.

## E. New this round (verify)
- [ ] **Megacity >1M:** over a long run, does the richest coastal capital of a large region
      approach ~1M (only when fed + prosperous + a trade hub)? Others should NOT.
- [ ] **Settlement tier badge** (Provision tab): shows Outpost→Emporium; stable (hysteresis),
      can rise/fall over years.
- [ ] **City Stores** (Provision tab): civic reserve + "riches in goods" value + food reserve
      + richest goods held.
- [ ] **Cold Start** (Chronicle panel, before the clock starts): "❄ Cold Start" zeroes
      houses/banks/coin/wealth and shrinks cities; on unpause the economy + network rebuild
      from nothing (guilds ~yr5, houses ~yr10).
- [ ] **Hinterland breathes:** sub-cap village dots grow/shrink with their market over years
      (not frozen). ⚙️ tune `HINTERLAND_BASE_CAP`/`HINTERLAND_GROWTH`.

## Known NOT-yet-done (so don't file these as bugs)
- Pathfound routes ("never a straight line") — slice 2 (needs in-app perf + geometry check).
- Food-capacity redesign (arid towns settling content) — needs an imported-food tracking
  field; the naive version is degenerate (documented).
- Tier ability-gating (guild/mint/coinage unlocks) — deferred (economy-regression risk blind).
- Hinterland → full-hub PROMOTION (a grown village becoming a live simulated hub).

---
*Fill in notes inline and hand back; I'll turn them into targeted fixes/tuning.*
