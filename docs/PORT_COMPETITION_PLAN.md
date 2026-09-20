# Port hubs, break-of-bulk legibility, and port competition — plan

> **Status: Slice 1 BUILT AND GATED. Slice 2's diagnostic BUILT and the dose
> walked one step (0.0 → 0.3), gated. Slice 3 not built.** Written after a
> live user report ("I don't see any city port
> where ships can land and unload cargo to caravans… I need to see the exact
> transshipment happening") turned into three rounds of investigation across
> one session. Two of the three things asked for turned out to already exist;
> the session's own earlier analysis (given to the user mid-conversation) was
> partly wrong and is corrected below rather than quietly dropped (CLAUDE.md
> §2.4: negative results are deliverables).

---

## 0. Three corrections to this session's own earlier analysis

Read this before the rest of the doc — it explains why Slice 1 looks the way
it does rather than the way the session first proposed it.

1. **"The map never shows a real staged relay" was WRONG.** The session's
   first message to the user claimed `route_outlet` (the cost-based single
   entrepôt) and `staging_hop` (the range-based multi-hop relay) "don't talk
   to each other" and that the map is blind to real staging. Tracing
   `dispatch`/the arrivals pass (`production.rs`, `mod.rs`) line by line shows
   this is false: **`InTransit.from`/`.to` already ARE the real physical
   current leg's endpoints**, for both mechanisms — `first_stop = if staged
   >= 0 { staged } else { outlet }` decides `to` at dispatch, and the arrivals
   pass pushes a brand-new `InTransit` with `from: to (the hub just reached),
   to: next_stop` on every re-embarkation. So `campaign_merchant_routes()`'s
   own grouping by real `(from, to)` pairs already reflects genuine staged
   hops as separate groups, with no guessing needed.

2. **What WAS real: the map's relay MARKER was wired to the wrong signal.**
   `campaign_merchant_routes()` used to re-derive a relay guess by
   independently re-checking `route_outlet` for the EMITTED `(lo, hi)` pair
   and splitting it into two synthetic legs. This is redundant with what the
   real in-flight shipments already show, and can actively disagree with it:
   `route_outlet` is recomputed only periodically (`rebuild_routes`), so a
   shipment genuinely dispatched under an OLDER outlet table could be marked
   (or not marked) inconsistently with what it is actually doing right now.
   Worse, because the guess only ever finds ONE relay per emitted pair, a
   genuinely multi-hop staged journey (up to `RELAY_MAX_HOPS` = 6 real stops)
   could have its non-adjacent legs correctly present as separate groups but
   silently unmarked, because the guess for THAT leg's own short `(lo, hi)`
   found nothing further to relay through.

   **Fixed in Slice 1**: `campaign_merchant_routes()` now reads
   `InTransit.via` directly off the real shipments making up each group —
   `relay_hi`/`relay_lo` on the aggregator, true whenever an actual in-flight
   shipment on that exact leg has `via >= 0` (i.e. really does continue past
   this hub). No guessing, no redundant `route_outlet` re-derivation, and it
   naturally extends to any number of real hops since each hop is already its
   own group.

3. **"Big ports don't visibly act as hubs" was WRONG — this already ships.**
   `TickHub.hub_class` (0 ordinary · 1 trade hub · 2 entrepôt, from real
   yearly-reranked `transit_year` throughput) and `relay_count`/
   `is_transit_knot` (off `route_outlet`, "how many OTHER pairs' cheapest
   route relays through here") are both real, computed state, and both
   already draw distinct map markers on every settlement dot: a **blue square
   ring** (trade hub), a **red triangle ring** (entrepôt/emporium,
   `OverlayManager.ts` ~3415-3442), and a **teal hourglass** badge (a genuine
   transit knot, ~3463-3490) — see `commands/campaign_commands/mod.rs`
   :291-303 for the field docs. This is a live campaign feature, not
   something this plan needed to build. It is easy to miss because `hub_class`
   starts at 0 for every hub and only differentiates after real trade has run
   for a while (reranked twice a year), and because it rides the ordinary
   settlement dot rather than a big banner. No further work is proposed here
   for this specific ask — if it should be MORE prominent (e.g. surfaced in a
   legend, or a toggle to filter the map to hub-class-2 cities only), that is
   a `cartographer`/`design`-flavoured follow-up, not an economic one, and is
   not scoped here.

---

## 1. What Slice 1 actually changed

`docs/TRADE_STAGING_AND_POSTS_PLAN.md` already shipped the substance (staged
legs, real transit revenue, break-of-bulk resale-or-forward at an outlet).
What this slice adds is entirely about **making what's already happening
legible**, plus one new, deliberately-inert lever:

- **`campaign_merchant_routes()` rewritten** to derive `relay_at` from real
  `InTransit.via`, not a re-guessed `route_outlet` lookup (see §0.2). Also
  gained real per-leg `km`/`days`/`tariff_export`/`tariff_import` (pure
  derived reads off `CampaignSim::hub_km`/`.days`/existing tariff rates — no
  new persisted state, no `econ_` risk).
- **`MerchantRoutePanel.tsx`** (the map's route-click panel) now shows those
  figures, and stitches a relayed route's two legs together to name the real
  break-of-bulk port instead of showing an unexplained line.
- **`renderMerchantRoutes` draws each stretch in its real medium** (dashed
  sea / solid road-river) instead of one dash style for the whole lane — the
  literal cause of "one continuous line with no visible stop", independent of
  whether any economic relay applies at all.
- **A new geometric marker**: a small teal ring at EVERY point along a
  routed corridor (Merchant Routes layer and the Flows-tab route highlight)
  where the drawn path's own medium changes (land/river ↔ sea) —
  `OverlayManager.drawMediumTransitionRings`. This is deliberately a
  DIFFERENT, additional signal from the economic relay ring: cargo cannot
  sail on land, so a land↔sea crossing on the corridor is a real
  transshipment point regardless of whether the sim's own dispatch treats
  the voyage as one leg or several. It directly answers "I need to see the
  exact transshipment happening on those cities" — every conversion, not
  just the ones the sim happens to book as a separate economic stop.
- **`PORT_COMPETITION_DOSE` / `transit_toll_mult`** (below) — new, dosed at
  zero, a true no-op today.

**On "fix those lengthy routes":** a long single-medium stretch is not
automatically a bug. `SHIP_LEG_MAX_KM` (3,500 km) / `CARAVAN_LEG_MAX_KM`
(800 km) already force staging past those distances (`TRADE_STAGING_AND_
POSTS_PLAN.md`, shipped live); a route shorter than its mode's cap runs in
one leg by design, and coastal shipping legitimately covers a long stretch
without a stop, historically (Baltic grain to Amsterdam is this project's
own cited example, `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md`). What was
actually wrong was that this was invisible (§0.2) or badly drawn (the single
dash-style bug, both fixed above). One real, disclosed limitation remains:
`staging_hop`'s candidate stops are the departing hub's own bounded
`NEIGHBOR_K` (32) trade-neighbour shortlist, "a captain makes for a port he
already trades with" by design — a geometrically obvious town not in that
shortlist is never offered as a stop. Not touched here; flagged as a
possible Slice 2 item below rather than assumed clean.

**Gates run** (per §2.8, this touched `sim/campaign/tick/production.rs` and
`polis.rs`): `cargo check --lib --tests` clean · `cargo test --lib
tick::tests` 262/262 · `cargo test --lib econ_` 6/6, including the
hard-asserted multi-seed inheritance gate · `simulate_decades_reports_
dynamics` read directly: sustained richest 486,401 over 50y — identical to
the documented pre-existing baseline (`CLAUDE.md` §5.5), confirming the
rewrite and the new lever are genuinely inert at the shipped dose, not just
plausibly so. `npx tsc --noEmit` clean.

---

## 2. Port competition — the genuinely new mechanic

The user's ask ("cities may compete with each other if nearby for the
trade") has no existing mechanism behind it. What exists today:
`route_outlet`'s entrepôt search picks, for each hub `a`, the single nearest
outlet by raw travel days, then checks whether composing through it beats a
direct route. Two adjacent ports of similar distance split transshipment
traffic by pure geometry — an implicit, invisible, price-blind form of
"competition" with no way for a losing port to respond.

### Slice 1 (built) — the toll lever, dosed at zero

`TickHub.transit_toll_mult` (serde-default 1.0) is a port's own charge on
cargo that merely breaks bulk there — the anchorage/staple due a real port
levied on a passing ship, distinct from `tariff_export`/`tariff_import`
(which tax goods the city itself buys or sells). Historically grounded
exactly where `TRADE_STAGING_AND_POSTS_PLAN.md` §3/§4.1 already cites it:
the Palmyrene Tariff, the Sound Dues, the *cartaz*.

`decide_port_tolls`/`apply_port_tolls` (`polis.rs`, yearly, folded into
`run_polis_policy`) computes a target toll from `relay_counts()` — a hub
well above the world's own median relay traffic leans on that demand and
raises its toll; one below it cuts, trying to draw transshipment away from a
busier rival. The toll biases **which outlet wins the role** in
`rebuild_routes`'s entrepôt search (`production.rs`), not the real travel
time recorded in `self.days` (that stays a genuine cost, never inflated by
policy) — so two similarly-placed rivals can now genuinely out-compete each
other on price for the same relay trade, not just split it by raw distance.

**Dosed at exactly zero** (`PORT_TOLL_COMPETITION_DOSE = 0.0`): every hub's
toll target is 1.0 regardless of its relay traffic, `transit_toll_mult`
never moves off 1.0, and `route_outlet`'s outlet selection is bit-identical
to before this existed — proven by `port_toll_competition_is_a_noop_at_zero_
dose` (`tick::tests`), which builds a real fixture with two coastal outlets
specifically so the outlet-selection branch this dose feeds is actually
exercised, not short-circuited by "no outlets exist".

Bounds for when the dose is ever raised: `PORT_TOLL_MIN = 0.7`, `PORT_TOLL_
MAX = 1.6` — a port may lean on its traffic but never tax a rival's trade to
nothing (the exact "post owner deletes a rival's lane" failure mode
`TRADE_STAGING_AND_POSTS_PLAN.md` §4.1 Brake 2 already warns against), nor
pay cargo to stop.

### Slice 2 — BUILT: the diagnostic, and the first dose step (0.0 → 0.3)

The R2 risk this doc itself named — "the toll mechanism never actually
matters on a real world" — is now measured rather than assumed either way.
`econ_measure_port_competition` (`economy_validation.rs`, `#[ignore]`d)
builds `tests::dense_world()` (60 hubs at real ~445 km spacing, 20 of them
coastal — the established stand-in for "a realistically dense settled
world" this codebase already uses for the N1c/C4 staging dose walks;
`reference_world()`, tried first, has only ONE coastal hub and measures
0% trivially — a fixture problem, not a finding), runs it 60 years, and for
every hub with 2+ same-component coastal outlet candidates measures the
travel-day gap between its best and second-best outlet against
`MAX_TOLL_SWING = ENTREPOT_DWELL_DAYS × (PORT_TOLL_MAX − PORT_TOLL_MIN)`
(2.70 days — the largest bias two rival ports could ever separate by at the
plan's own bounds).

**Measured: 54 of 54 real hubs had 2+ coastal outlet candidates, and 26 of
54 (48.1%) were CONTESTABLE** (top-2 margin ≤ the swing; median margin
3.35 days, mean 4.04 days). R2 is refuted — on a realistically dense world
roughly half of all hubs sit close enough to a second port that a toll
difference could genuinely decide which one wins their relay trade. That
justifies raising the dose off zero; it says nothing about how HIGH is
safe, which is what the gate suite below exists to check.

Dosed to **0.3** — a conservative first step (`PORT_TOLL_MIN` = 0.7 /
`PORT_TOLL_MAX` = 1.6; at 0.3 the raw pre-clamp toll target already spans
0.70..1.30, i.e. the full downward range and over half the upward one, so
this is not a token move). `decide_port_tolls` was split into a thin
`decide_port_tolls()` (reads the shipped constant) over a parametrized
`decide_port_tolls_at(dose)`, so the original zero-dose no-op claim stays
checked against the literal value `0.0` (`port_toll_at_zero_dose_is_a_true_
noop`) independent of what ships; a new
`port_toll_competition_biases_toll_by_relay_traffic_within_bounds` checks
the dosed behaviour (a busier relay targets a higher toll than a
never-relayed one, both bounded, `apply_port_tolls` still eases rather than
snaps).

**Gates run at the 0.3 step** (per §2.8 — this touched
`sim/campaign/tick/{mod,polis,tests,economy_validation}.rs`):
`cargo check --lib --tests` clean · `cargo test --lib tick::tests` 263/263
(incl. `simulate_decades_reports_dynamics` and both dense-world staging
gates) · `cargo test --lib econ_` 6/6, including the hard-asserted
multi-seed `econ_inheritance_rules_fragment_differently` ·
`simulate_decades_reports_dynamics` read directly: sustained richest
**486,401** over 50y — bit-identical to the pre-existing documented baseline
(`CLAUDE.md` §5.5). That exact match is itself a finding worth recording,
not just a clean pass: `simulate_decades_reports_dynamics`'s own fixture
has too few coastal hubs for this mechanism to engage at all (the same
"abstract world, wrong instrument" gap the diagnostic itself exists to
route around) — so this run proves NO REGRESSION on that world, not that
the toll mechanism is exercised there. The 48.1%-contestable measurement on
`dense_world()` is what shows the mechanism matters; this run is what shows
it doesn't break anything. No `npx tsc --noEmit` needed — nothing in
`src/` changed.

### Slice 2, continued — QUEUED. Raising the dose further.

0.3 is one step, not the ceiling. Raising it again needs the identical
recipe: one step, re-run `tick::tests` + `econ_` + the multi-seed
inheritance gate, record the table, raise again only if nothing broke — and
this time also read `econ_measure_port_competition`'s own numbers again
after several dose steps, since a toll mechanism that is actually moving
trade should, in principle, start closing some of those 48.1% margins as
ports adjust (a feedback the diagnostic does not yet measure — it is a
static snapshot, not a before/after). The two numbers most likely to move
on a further raise are top-10% wealth share (a port that starts winning
trade compounds — `hub_class` rises, `hub_pull` rises, it wins even more)
and house turnover, the same two the closure risk register in
`TRADE_STAGING_AND_POSTS_PLAN.md` §4.1 already names for the adjacent
embargo mechanic. Do not raise this dose without re-running the full gate
recipe at each step; a spot-check win with an aggregate loss is a revert,
not a judgement call (CLAUDE.md §2.4).

### Slice 3 — QUEUED, not built. An active response.

Today the toll target is a pure function of relay traffic — a hub cannot
choose to fight for a specific rival's business, only drift its own toll
with its own volume. A real "port A undercuts port B because B is winning
its trade" story needs the decision to read a NAMED rival (the busiest
nearby outlet actually competing for the same (a, b) pairs), which
`route_outlet`'s per-pair table can answer but nothing currently aggregates
into "who is MY rival". Waits on Slice 2 landing first (no point building a
sharper decision on a lever not yet known to matter), and on a chronicle
line so a toll war is a story the player can read, not a silent number
(CLAUDE.md's own "every mechanism must produce a legible story" rule,
`INSTITUTIONS_BUILD_ORDER.md`'s governing rule, applies here too).

### Staging_hop's neighbour-list limitation — QUEUED, not built.

Named in §1 above. Waits on a `#[ignore]`d diagnostic on a real generated
world measuring how often a leg is forced into a longer/costlier stage
because its nearest geometric stop wasn't in the departing hub's own
`NEIGHBOR_K` trade-neighbour list — before touching `staging_hop` itself,
since it is explicitly `O(NEIGHBOR_K)` by design for dispatch's hot loop
(§8.9 rule 1's spirit) and any fix has to keep that bound.

---

## 3. Deliberately NOT built

- **A post closing itself to everyone** (a true blockade of a relay route) —
  out of scope for the same reason `TRADE_STAGING_AND_POSTS_PLAN.md` §6
  already excludes it; only per-house bars exist.
- **Player agency over toll policy** — `decide_port_tolls` is AI-only,
  consistent with every other `decide_*`/`apply_*` split in this codebase
  (FIX_PLAN B2); re-exposing it to a player who holds the seat is a UI change
  once the mechanism is measured, not before.
- **Rewriting `staging_hop`'s candidate set** — see §2 Slice 2's own queued
  item; needs its own measurement before any change, not assumed broken.

---

## 4. Risk register

| # | Risk | Slice | Mitigation |
|---|---|---|---|
| R1 | Raising the toll dose lets one port's winning streak compound into a wealth-concentration spiral (same shape as `TRADE_STAGING_AND_POSTS_PLAN.md`'s embargo risk) | 2 | `PORT_TOLL_MIN`/`_MAX` bound the toll; `econ_` + multi-seed inheritance gate re-run at each dose step — checked clean at 0.3, still open at any dose above it |
| R2 | The toll mechanism never actually matters on a real world (two competing ports never sit close enough in cost) | 2 | **RESOLVED, measured 2026-09-20**: `econ_measure_port_competition` on `dense_world()` found 48.1% of hubs contestable — the mechanism matters |
| R3 | The medium-transition ring (§1) over-fires on a route whose corridor merely hugs the coast in and out of many small bays, cluttering the map with rings that aren't real stops | — | Not yet measured on a real world; if this turns out to be noisy, the fix is a minimum-run-length filter in `drawMediumTransitionRings`, not removing the feature — left as a finding to watch, not fixed pre-emptively |
