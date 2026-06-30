# Trade-Base Mechanic — houses develop small cities as bases

## Problem
Remote, low-surplus settlements (e.g. Beyakent: 27,705 souls, 0 throughput, 1
partner, 0% wealth) never enter trade. The dispatch is realistic but leaves a third
of the map inert: a subsistence town has no exportable surplus and is freight-isolated
(`tick.rs` arbitrage requires `surplus > 0` AND a buyer whose price gap covers
`freight + margin`). Both conditions fail for a poor, distant town.

## Concept
A wealthy merchant house **invests influence + capital** into an existing
under-traded city to make it a **base of operations**: it opens an office, builds a
guildhall (and a warehouse), seeds working capital, and subsidizes the town while it
finds its feet. The freight discount + primed capital let the town's modest surplus
finally clear to nearby markets, so it bootstraps into a live node and grows — and the
patron house profits from dominating its trade. This is the *existing-settlement*
cousin of `maybe_found_house_outpost` (which plants on empty land).

## Eligibility (yearly, rate-limited — one per advance batch)
- **Founder house:** active, not a guild, wealth ≥ `BASE_INVEST_WEALTH` (~40k — well
  below the 100k outpost bar; developing a base is more accessible than colonizing),
  and does **not** already patronize / hold an office in the target.
- **Target city:** a real hub (`colony_kind == 0`, not estate, not already patronized),
  on the **founder's continent** (`component`), population in
  `[BASE_MIN_POP, BASE_MAX_POP]` (~5k–60k: big enough to matter, small enough to be
  undeveloped), and **under-traded** — `export_earn + import_spend` below a small
  fraction of population (the "untapped market" signal). Prefer the candidate nearest
  the house's existing network (seat + offices) so the base extends real reach.
- **Gate:** opens from `BASE_START_TICK` (~year 10) once a few houses have capital.

## The investment (committed atomically; affordability checked first)
- Debit `BASE_INVEST_COST` (scaled to city size) from the house's wealth.
- House **opens an office** in the city (`house.offices.push`) — a permanent foothold.
- City gains a **Guildhall** (`STRUCT_GUILDHALL`, −15% export freight) and a
  **Warehouse** (`STRUCT_WAREHOUSE`) if absent — the physical base.
- Seed **working capital** into the city treasury (`hubs[c].treasury += BASE_SEED`) to
  prime local commerce.
- Record patronage: `hub_patron[c] = house` (a new `#[serde(default)]
  Vec<i32>` on `CampaignSim`, indexed by hub, default −1 — avoids touching every
  `TickHub` literal and loads clean on old saves).
- Journal + house chronicle: *"House X establishes a trade base in Beyakent."*

## Bootstrapping & growth (ongoing, while patronized)
- The guildhall freight cut + seeded capital let exports begin; the office gives the
  patron market access and a share of the city's trade (`tw_house`).
- **Development bonus:** while patronized and still small, the town gets a modest pop
  growth bonus (`BASE_POP_GROWTH_BONUS`, gentler than `POP_GROWTH_COLONY_MULT`) and a
  small standing trade nudge, so it visibly grows into a node.
- **Graduation / payoff:** once the city clears a "developed" bar (population +
  structures + real throughput), patronage **concludes** — the base has become a
  self-standing market the house still trades from. The slot frees so other small
  cities can be developed (keeps the mechanic spreading across the periphery).

## Risk / bounded fortunes
The capital is at risk: the cost is real and the payoff uncertain (a town that never
develops is a sunk investment; a house may **withdraw** after a cooldown, dropping the
office and patronage). This keeps fortunes bounded — consistent with the dynamics
test's hard wealth-bound + turnover assertions. Rivals can still contest the city via
the existing dominance mechanics.

## Surfacing (UI)
- **HubPanel:** "Patron: House X" + a *Trade base (developing)* badge; structures list
  shows the new Guildhall/Warehouse. (`HubDetail` reads `hub_patron[h]`.)
- **HousesPanel:** the house's bases listed beside offices/outposts; chronicle event.
- **Map:** reads as an office node on the existing house-control / office overlay.

## Code touch-points
- `sim/tick.rs`
  - New consts: `BASE_INVEST_WEALTH`, `BASE_INVEST_COST`, `BASE_SEED`,
    `BASE_START_TICK`, `BASE_MIN_POP`, `BASE_MAX_POP`, `BASE_POP_GROWTH_BONUS`,
    `BASE_UNDERTRADE_FRAC`, `BASE_DEVELOPED_POP`.
  - New `#[serde(default)] pub hub_patron: Vec<i32>` on `CampaignSim`; resized to
    `hubs.len()` each tick (like `house_ledger`).
  - `fn maybe_establish_trade_base(&mut self)` — called at the yearly hook beside
    `maybe_found_house_outpost`, gated on `tick >= BASE_START_TICK`.
  - Patronage upkeep folded into the yearly hub pass (growth bonus + graduation).
- `commands/campaign_commands.rs` — expose `patron` (house name) + a `trade_base`
  flag on `HubDetail`.
- `src/types.ts` + `ui/HubPanel.tsx` / `ui/HousesPanel.tsx` — show patron + badge.

## Tuning / verification
- Run `simulate_decades_reports_dynamics`: wealth must stay bounded, houses must still
  turn over, and some previously-inert small cities should now show throughput.
- New unit test: a rich house + a qualifying small city ⇒ a base is established
  (office opened, guildhall built, patron set), and the city later shows trade.
