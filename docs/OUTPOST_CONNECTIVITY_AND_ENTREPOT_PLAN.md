# Outpost connectivity & the entrepôt — fix plan

**Status: PROPOSED, nothing built.** Read alongside
`TECTONICS_RIVERS_PROVINCES_PLAN.md` (the world half); this is the campaign half.

Reported: outposts are not connected to their founding capital, no
understandable routes are drawn to them, and there is no natural port /
transshipment hub letting goods move from a hard-access interior to a river,
lake or open sea.

The first two are **bugs with a single shared cause**. The third is a real
design gap and is the larger piece of work.

---

## 0 · What an outpost actually is

`try_found_house_outpost` (`tick/houses.rs`) creates a hub via `create_estate`
with **`parent = -1`**, then sets `colony_kind = 2`. The `parent = -1` is
deliberate and documented — it is what keeps the outpost at its own remote site
coordinates instead of being co-located with its founder.

That produces a hub which is `is_estate == true` but **is not co-located with a
parent city**. Every other estate in the model is. The whole codebase branches on
`is_estate` in one of two ways, and an outpost is wrong under both:

- **"It's internal to its parent, collapse it there"** — correct for a farm
  outside a city, wrong for an outpost 2,000 km away with no parent at all.
- **"It's not a real hub, exclude it"** — correct for keeping estate dots off
  city rankings, wrong for a settlement that is genuinely a separate place.

An outpost is a **third category** — a remote, self-standing production site —
and no code path recognises it. That single fact explains both bugs.

*(One lead investigated and ruled out: `create_estate` does set
`self.routes_dirty = true`, so the route matrix IS rebuilt when an outpost is
founded. The rebuild is not the problem.)*

---

## 1 · The measured findings

### G1 — The flow overlay silently drops every outpost's trade

`commands/query_commands/flow.rs`, the Dynamic Trade Flow overlay — the layer
that draws the trade lines on the map:

```rust
for h in &sim.hubs {
    if h.is_estate { continue; }        // ← outposts never get a node
    node_of.insert(h.id, cc.cidx(cx, cy));
}
...
for &(a_id, b_id, vol) in &sim.flow_year {
    let (s, g) = match (node_of.get(&a_id), node_of.get(&b_id)) {
        (Some(&s), Some(&g)) if s != g => (s, g),
        _ => continue,                  // ← outpost endpoint ⇒ flow discarded
    };
```

An outpost never enters `node_of`, so **every `flow_year` entry with an outpost
at either end fails the lookup and is dropped**. This is the direct answer to
"why are there no understandable routes there": the outpost's trade is not
missing from the simulation, it is missing from the *picture*.

It has a second, quieter cost: that volume is discarded rather than reassigned,
so the trunk widths on the rest of the map **under-report** by whatever the
outposts were carrying.

Note the contrast with `read_trade.rs`, which handles the same problem correctly
and independently:

```rust
let city_of = |h: u32| match sim.hubs.get(h as usize) {
    Some(x) if x.is_estate && x.parent >= 0 => x.parent as u32,   // collapse
    _ => h,                                                        // keep
};
```

That guard checks `parent >= 0`, so a co-located estate collapses to its city and
a **parentless outpost correctly keeps its own identity**. The flow overlay never
got the same treatment. Two places solve one problem, one of them right.

### G2 — Outposts are excluded from all three "no dead city" lifelines

`rebuild_routes` (`tick/production.rs`) has three guarantees, added specifically
so no settlement is a dead dot that can never trade. All three open with the
same filter:

```rust
let real: Vec<usize> = (0..n)
    .filter(|&i| !self.hubs[i].is_estate && !self.hubs[i].abandoned)
    .collect();
```

- **#6 `MIN_GUARANTEED_PARTNERS`** (4 nearest same-component partners) — skipped.
- **#6b hub-and-spoke market lifeline** (a route to a real market) — skipped.
- **#6c coastal cabotage** (short-sea link to another component) — skipped.

So the one class of hub that is remote, tiny (`OUTPOST_MAX_POP` = 800) and
newly-founded — i.e. **the most likely to be stranded** — is the only class
denied every anti-stranding guarantee. An outpost gets a route only from the
generic pass, which requires both:

- within the trade horizon, `TRADE_MAX_DIST_FRAC = 0.24` of world width, and
- **`component[a] == component[b]`**, because `base_days` (the real pathfound
  lane matrix) only covers the founding hub set and an outpost's index is always
  `≥ base_n`.

### G3 — An outpost's component can go stale and is never repaired

The outpost copies `component` from its founder at creation. The repair pass
`rescue_tiny_components` skips estates, and its estate fixup is:

```rust
if self.hubs[i].is_estate {
    let p = self.hubs[i].parent;
    if p >= 0 && ... { self.hubs[i].component = self.hubs[p as usize].component; }
}
```

`p >= 0` is **false for every outpost**, so an outpost is never re-synced to its
founder's component. Today `components_rescued` is a one-shot flag that fires at
campaign start, before any outpost exists, so this is currently latent rather
than active — but it is a live trap for any future pass that reassigns
components, and combined with G2 it means a mis-set component silently removes
an outpost from trade with no lifeline to catch it.

### G4 — There is no transshipment anywhere in the model

This is the user's actual design point, and it is not a bug — the capability
does not exist.

`rebuild_routes` produces a **direct point-to-point** `days[a][b]` matrix, and
dispatch ships **origin → destination in one leg**. There is no notion of a
cargo moving inland-site → port → distant market. So a hard-access interior site
either reaches a market *directly* or does not trade at all. There is no way for
a port to earn its living by *handling other people's goods*, which is what an
entrepôt is and what nearly every real pre-modern trade city was.

Related state exists but is unused for this: `TickHub` already carries
`coastal`, and `in_by_sea` / `in_by_land`. Worldgen already finds the right
places — step 7a's `generate_trade_sites` scores straits, isthmuses, passes and
great river mouths precisely because "a great port need not sit on the best
farmland". The campaign never reads that idea.

---

## 1b · Is the outpost logic "explore, then site for movement, then relay"?

Asked directly: does the model explore first, site outposts where goods can
actually move, and chain several posts so cargo gets from cart to river boat to
ship? **Measured answer: no, on all four counts.** Each is a separate finding.

### G5 — There is no exploration. Houses are omniscient from day one.

There is no `explored` / `discovered` / `surveyed` state anywhere in the tick.
`colonizable` is a **whole-world list snapshotted at campaign start**, and
`try_found_house_outpost` scans *all of it* on every call, taking the best-scoring
site within `COLONY_MAX_KM` of the founder's network. A house in year 1 already
knows the trade value of every site on the planet.

Expeditions do exist (`expedition_launch_pass`, `route_prospects`,
`envoys.rs`) — but they are a **parallel, ornamental system**: a prospect feeds
the overlay and a goal check, and gates nothing. Nothing an expedition discovers
is required before an outpost is planted there.

So an outpost is not a venture into the unknown; it is an optimal pick off a
complete map. That is the opposite of the intended fiction, and it is also why
outposts appear in surprising places — nothing stops a house reaching past
everything it knows.

### G6 — Outpost siting ignores the very flags that mark a transshipment point

`ColonizeSite` already carries exactly the right data:

```rust
/// River-mouth / DELTA (fertile coastal alluvium — a natural port + granary).
pub delta: bool,
/// Land→sea CHOKEPOINT (strait / isthmus / portage where cargo transships and
/// tolls can be levied — Venice/Bruges/Constantinople-style prize sites).
pub chokepoint: bool,
```

`maybe_found_settlement_colony` (the **city**-founded path) reads both and prices
them heavily — `delta +0.60`, `chokepoint +0.80`, against `coastal +0.35`.

`try_found_house_outpost` (the **house** path — the outposts in question) scores:

```rust
let trade_score = s.trade_value + if s.coastal { 0.30 } else { 0.0 };
```

**Neither `delta` nor `chokepoint` is read.** A river mouth and a strait — the two
site kinds that exist in the data *precisely because* cargo transships there —
carry no weight at all when a merchant house picks where to trade from. The
outpost is sited on what the ground yields, with almost no weight on whether the
cargo can leave. That is the direct cause of the reported "hard-access" posts.

The city path also refuses to let an inland founder plant a coastal colony ("no
fleet tradition"); the house path has no equivalent rule.

### G7 — No relay. Outposts are picked independently, never as a chain.

`maybe_found_house_outpost` takes the richest houses (`OUTPOST_MAX_PER_CALL` = 3)
and each independently scores its own best site. **No pass anywhere considers two
posts together**, or plants one *because* another needs an outlet. There is no
staging, no relay and no notion of a corridor of posts. The "several outposts, at
greater cost, so cargo can reach the water" pattern is not implemented in any
form.

### G8 — Water is not cheaper than land, and rivers are not a mode at all

This is the deepest of the four, and it reaches well past outposts.

```rust
// Route mode: a sea voyage when both ends are coastal, else overland.
let sea = self.hubs[a].coastal && self.hubs[b].coastal;
```

Mode is decided solely by whether *both* endpoints are coastal, and all it does is
choose which fleet counter to decrement (`cap_sea` vs `cap_land`). The cost is
untouched: `days` comes from `rebuild_routes` as `dist · days_per_cell` with a
single global `days_per_cell` and **no land/sea/river distinction anywhere**.
`good_freight` then multiplies those same days by the good's `bulk`.

So a ton of stone crosses 500 km of mountain for exactly what it costs to sail it
500 km along a calm coast.

And rivers are not a mode at all:

```rust
cap_land[i] = (h.fleet_river + h.fleet_caravan) as i32;
```

**River barges and ox-carts are pooled into one interchangeable "land" capacity.**
A river confers no cost advantage, so nothing in the economy has any reason to
follow one.

That water carriage was roughly an order of magnitude cheaper than overland haulage
is arguably *the* organising economic fact of the pre-modern world — it is why
cities sit on rivers and estuaries at all. Its absence is very likely a major
contributor to the market-integration failure CLAUDE.md already names as the
largest known economy defect (the basket price/distance gradient reading **−0.064**,
with 0 of 6 goods showing any gradient, where the historically correct sign is
positive). If distance costs the same whatever the terrain, no trade concentrates
on water and no economic geography forms.

**This should be tested before it is designed around.** The claim "differentiated
transport cost is a main cause of the flat gradient" is a hypothesis with a clear
instrument — `econ_fidelity_scorecard` already measures the gradient — and
CLAUDE.md §8.15's cautionary tale is precisely about concluding from an untested
mechanism inside an already-distorted economy. Measure first.

### Slice D — Transport modes with real relative costs *(G8; the largest lever)*

Give a route a **mode** with its own per-day cost: sea ≪ river < road < track.
Concretely: keep one `days_per_cell` but attach a per-mode multiplier, and split
`cap_land` back into `fleet_river` and `fleet_caravan` so a river barge is not an
ox-cart. A route is river-borne where both endpoints sit on the same navigable
system — worldgen already knows which rivers are navigable (`River.navigable`,
used by the province partition), so the data exists and only needs carrying into
the campaign snapshot.

**Gate:** the existing `econ_fidelity_scorecard` price/distance gradient. This is
the honest test of whether G8 is the cause: if differentiating transport cost does
not move the gradient off −0.064, the hypothesis is wrong and should be recorded
as a negative result rather than quietly kept.

**Order note:** Slice D plausibly subsumes much of Slice C. If water carriage is
genuinely cheap, an outpost near navigable water becomes valuable *automatically*
through the ordinary route cost, without a bespoke entrepôt rule. **Build D before
C2**, and re-measure whether C2 is still needed.

### Slice E — Site outposts for movement, and explore first *(G5/G6/G7)*

- **E1 (small):** read `delta` and `chokepoint` in `try_found_house_outpost` with
  the same premiums the colony path already uses, and add the inland-founder rule.
  A few lines, and it directly targets the reported problem.
- **E2:** gate outpost founding on a prior expedition or route prospect reaching
  that region, turning the existing ornamental expedition system into the real
  precondition it was written to be. This is also `MERCHANT_VESSELS_AND_
  INFORMATION_PLAN.md` stages 5-6, which already own the survey-agent design —
  **coordinate rather than duplicate.**
- **E3 (only after D):** a relay pass — a house with a productive post whose
  outlet is poor plants an intermediate post toward navigable water, at a premium.
  This is the user's "several outposts at bigger price" and it is deliberately
  last: with Slice D in place the ordinary site scorer may already produce it.

**Gate:** `an_outpost_prefers_a_site_its_cargo_can_leave` — given two sites of
equal `trade_value`, one on a delta and one landlocked, the delta must win. Fails
today (the flags are not read).

---

## 2 · The slices

### Slice A — Draw outpost trade *(fixes G1; smallest, most visible)*

In `flow.rs`, replace the `if h.is_estate { continue; }` skip with the same
`city_of` mapping `read_trade.rs` already uses: an estate **with** a parent maps
to its parent's coarse node (so its flow is *credited to the parent city*, not
discarded); an estate **without** a parent gets its own node.

This makes outpost trade visible AND stops the silent volume loss on the rest of
the map. Extract `city_of` into one shared helper so the two call sites cannot
drift again — they already have.

**Gate:** `outpost_flow_is_never_dropped` — with an outpost trading, assert the
sum of volume reaching the overlay equals the sum in `flow_year` (today it is
strictly less whenever an outpost trades).

### Slice B — A remote site is a real trade node *(fixes G2/G3)*

Introduce one predicate and use it everywhere the three lifelines run:

```rust
/// A production site that stands on its own ground rather than inside a
/// parent city — today exactly the house trade outposts (colony_kind 2).
/// It is an estate for OWNERSHIP purposes and a real place for ROUTING.
fn is_remote_site(&self, i: usize) -> bool {
    self.hubs[i].is_estate && self.hubs[i].parent < 0 && !self.hubs[i].abandoned
}
```

Widen `real` in `rebuild_routes` to `!is_estate || is_remote_site`, so an outpost
gets `MIN_GUARANTEED_PARTNERS`, the market lifeline and cabotage like any other
settlement. Also let `rescue_tiny_components`' estate fixup fall back to the
**founder** (`founder_hub`) when `parent < 0`, closing G3.

Deliberately *not* widened: city rankings, society/pops, government — an outpost
should stay out of those. The predicate is about **routing**, not about promoting
outposts to cities.

**Gate:** `a_remote_outpost_always_has_a_market` — found an outpost at the edge of
its founder's range and assert it has ≥1 finite route to a same-component market.
Fails today.

**Watch:** this adds hubs to the lifeline loops, which are O(n²) over `real`. The
`econ_` bands and the dynamics run must both be re-checked — this changes who
trades, so it is *not* a bit-identical change and should not be claimed as one.

### Slice C — The entrepôt *(G4 — the real feature)*

Two parts, in order. **C1 is worth building alone**; C2 only makes sense after it.

**C1 · Two-leg routing through an outlet.** For a hub with poor direct
connectivity, allow one intermediate leg: `days[a][b]` may be composed as
`days[a][p] + dwell(p) + days[p][b]` where `p` is an **outlet** — a coastal hub,
a navigable-river hub or a lake port, in the same component as `a`. Cap it at
**one** transshipment (a pre-modern cargo was not containerised; two-leg is the
honest ceiling and it keeps the matrix build from becoming an all-pairs shortest
path). `dwell(p)` is a real cost in days — that cost is exactly what makes a good
port valuable and a bad one bypassed.

The handling port should **earn** from it: route a share of the transit value to
`p`'s `treasury` / `trade_wealth`. That is the entrepôt's whole economic
character, and it gives the model something it currently lacks — a city that is
rich because of *where it is*, not what it grows.

**C2 · Founding a port where one is needed.** A new yearly pass, reusing
`maybe_found_house_outpost`'s machinery with a different site scorer: when a
house holds ≥2 remote sites in one region whose best outlet is poor, it founds a
**port** at the best coastal / river-mouth / lakeshore site *between them and
open water*. Score by outlet quality (navigable water, shelter, low approach
cost) rather than by `trade_value` — a port is not a plantation and the existing
scorer would never pick the right cell.

This is the "one more outpost as a trade hub which comes naturally" the user
described, and it is the historical pattern (Phoenician emporia, the Venetian
*fondaco*, Hudson's Bay factories). `maybe_graduate_outpost` already exists to
let such a post mature into a real city, so the lifecycle is already there.

**Gate:** `an_inland_site_trades_through_its_port` — an inland outpost with no
direct market route must reach one via an outlet, and the outlet's trade wealth
must rise as a result. And a companion the plan should not ship without:
`transshipment_does_not_inflate_total_trade` — two-leg routing must **move**
value, never create it (the same zero-sum discipline as
`a_division_moves_capital_and_creates_none`).

---

## 3 · Suggested convention (CLAUDE.md rule 32)

> **`is_estate` is an OWNERSHIP flag, not a geography flag.** An estate with
> `parent >= 0` is co-located inside its parent city and collapses to it; an
> estate with `parent < 0` is a remote place standing on its own ground and must
> be routed, drawn and rescued like any settlement. Code that branches on
> `is_estate` alone will be wrong for one of the two, and the failure is silent
> in both directions — a co-located estate draws a zero-length route, a remote
> outpost is dropped from the map entirely.

---

## 4 · Risks

- **Slice B changes who trades**, so it moves the economy. Run `econ_` and the
  dynamics test; expect small shifts and check they are in the right direction
  rather than asserting nothing moved.
- **Slice C1 is the cost risk.** Composing routes through outlets is a
  shortest-path over the hub graph, and `rebuild_routes` is already O(n²) and
  runs whenever a hub is added. Mitigation: restrict candidate outlets to a small
  precomputed set per component (the top few coastal/river hubs by population),
  which keeps the extra work O(n · |outlets|) with `|outlets|` in the tens.
- **Slice C2 adds hubs**, and `MAX_TOTAL_ESTATES` already bounds the hub list.
  A port should draw from the `OUTPOST_RESERVED_ESTATES` reservation rather than
  competing with ordinary estates — the same starvation problem, and the same
  fix, that outpost founding already needed once.

## 5 · Deliberately NOT in this plan

- **Individual vessels with manifests and locations.** A vessel is still three
  counters on `House` — see `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md`, which
  owns that work. Slice C1 routes *cargo*, not ships, and should not pretend
  otherwise.
- **More than one transshipment leg.** Capped at one, deliberately (§C1).
- **Promoting outposts to cities to fix this.** `maybe_graduate_outpost` already
  handles maturation on its own terms; widening it to paper over a routing bug
  would be the wrong fix.
- **Re-pathfinding real lanes for hubs founded mid-campaign.** A tick has no tile
  access; `terrain_route_mult` is the documented stand-in and stays.
