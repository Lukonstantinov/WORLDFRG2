# Seasons, Elasticity & Leagues — the design for N5, N6, N7

*The three proposals `ACTORS_AND_CARRIAGE_PLAN.md` left as sketches, designed out:
the **sailing window** (N5), **price-elastic demand** (N6), and the **League** (N7).*

**Status: DESIGN. Nothing built. Not approved.** Each section carries its own
data structures, hook sites, zero-dose setting, gates by name, and a list of what
it deliberately does not do. Read `ACTORS_AND_CARRIAGE_PLAN.md` §3.5–§3.7 for the
one-paragraph versions these expand, and its §4.1 for the balancing method every
dose below is bound by.

**Three findings from writing this are worth more than the designs.** Each one
made its proposal smaller, and each was found by reading the code rather than by
reasoning about the sketch:

1. **N5 is mostly already built — on the world side.** `build_coarse_cost` takes
   `season: i32, months: u32` and already closes snow-shut passes and stormy
   sailing windows; `cached_coarse_cost` already keys its cache per season. The
   campaign's own route builder calls it with `season = -1` and the comment
   *"no seasonal closure"*. N5 is largely **calling a function that exists, per
   season, at campaign start** — not modelling seasons.
2. **Demand is not "perfectly price-inelastic" — only its AGGREGATE is.**
   Category substitution in `mod.rs` already weights each member by
   `pref / rel` where `rel = price / base_value`. Cross-price elasticity is
   built and live. What is missing is own-price elasticity of the category
   total. That is a much smaller, much safer change than the sketch implies,
   and the existing block is the template to copy.
3. **N7's stated dependency is wrong.** The sketch says the League's boycott is
   "an N2 cargo-ban voted by every member at once". N2 as built is a
   **hub × good** ban (`export_ban_until`) — it cannot express "we will not
   trade with *that city*". A boycott needs a **lane-scoped** ban, which nothing
   in the tree has. N7 therefore depends on an N2 EXTENSION, not on N2 being
   dosed. §4.1 below designs it.

---

## 0. What the last session's evidence changes about all three

Two live regressions were measured on `main` while shipping N1–N4, and both bear
directly on the designs below:

- **N2's export ban, dosed live, broke the hard-asserted wealth bound**
  (a sustained richest house of 1,005,714) and stayed broken after halving.
  Closing a market hands its resident monopolist a rent that *runs away* rather
  than merely concentrating. **Every design here that closes a market inherits
  that risk** — N5's seasonal closure and N7's boycott both do.
- **N4's first cut, weighted by `political_power`, inverted
  `econ_inheritance_rules_fragment_differently`** because `political_power`
  grows with wealth. **Any weight taken from a wealth-correlated field is a
  rich-get-richer channel**, whatever it is named. N7's dues, purse and vote
  weight are all exposed to this.

Both are the reason every slice below names a zero-dose setting and the gate
that proves it inert, rather than shipping "on" and tuning afterwards.

---

# N5 · The sailing window

## 1.1 What exists already (read this before designing anything)

`commands/query_commands/mod.rs`:

```rust
fn build_coarse_cost(world, grid_w, grid_h, rivers_json, block_sea,
                     desert_routes, piracy: f32, season: i32, months: u32)
```

and inside it, already written and already used by the world-side trade-route
overlay:

```rust
// Seasonal closures (#12): for a specific moon (season 1..=months) high
// mountain passes snow shut in the hemisphere's winter and monsoon/cyclone
// seas close their sailing windows, so routes detour or wait.
if season > 0 && months > 0 {
    if is_land[ci] {
        if elev[ci] >= 0.45 {                    // snow-shut passes
            let winter = ((m - shift) * TAU).cos() * 0.5 + 0.5;   // hemisphere-aware
            let latw = smooth(lat.abs(), 22.0, 50.0);
            cost[ci] += winter * latw * 45.0;
        }
    } else if !block_sea {                       // monsoon / cyclone window
        cost[ci] += sea_hazard[ci].clamp(0.0,1.0)
                  * biological::storm_season_phase(season, months, lat) * 16.0;
    }
}
```

`sea_hazard` is sampled per coarse cell as `max(storm_base, reef_risk)` off the
real tiles; `storm_season_phase` is a hemisphere-flipped cosine concentrated into
half the year and damped to a flat 0.5 at the equator. `cached_coarse_cost`
hashes `season` and `months` into its cache key, so twelve grids coexist.

And the campaign's own builder, `compute_route_days_matrix`, calls it with:

```rust
// Allow sea crossings … and desert routes …; no seasonal closure.
let cc = cached_coarse_cost(db, &world, fp, grid_w, grid_h, &rivers_json,
                            false, true, 0.0, -1, 12)?;
```

**`season = -1`.** The campaign has been reading the annual-mean grid since it
was written. That is the whole of what N5 changes.

Two consequences that make this design much better than a from-scratch one:

- **The season applies along the ROUTE, not to the endpoints.** The cost is
  accumulated cell by cell, each with its own latitude and its own hazard, so a
  lane from a northern port to a southern one is priced by the water it actually
  crosses. A per-hub seasonal term (the obvious design) could not do this and
  would have had to invent an answer for opposite-hemisphere pairs.
- **The physics is already the world's own.** `storm_base` comes from
  `compute_storm_base` (cyclogenesis belt × warm SST), `reef_risk` from the reef
  model, elevation from the DEM. Nothing here re-derives climate in the campaign
  half, which is what rule 11 and FIX_PLAN B1 both want.

## 1.2 The data structure, and why it is a `u8`

`base_days: Vec<f32>` (`base_n × base_n`) is serialized into `.campaign` today.
Twelve more f32 matrices would be 12× that — at `base_n = 400`, 7.7 MB in the
save. Instead:

```rust
/// N5 · per-lane seasonal travel-time multiplier, quantised.
/// Flat `SEASON_SLICES × base_n × base_n`; `mult = 1.0 + v as f32 * SEASON_MULT_STEP`.
/// v = 0 ⇒ EXACTLY 1.0 ⇒ bit-identical, which is the zero-dose gate.
/// Empty on an old save ⇒ every slice reads 1.0.
#[serde(default)] pub base_days_season: Vec<u8>,
#[serde(default)] pub season_slices: u8,   // 0 = none; 4 shipped; 12 possible
```

`SEASON_MULT_STEP = 1.0 / 64.0` gives multipliers 1.00 … 4.98 at ~1.6% steps,
finer than a travel time expressed in whole days can resolve. At
`SEASON_SLICES = 4`, `base_n = 400`: **640 KB**, u8, in the save.

**Four slices, not twelve.** The sketch says twelve. Four is recommended, for
reasons that are not thrift:

- the mechanism being modelled is seasonal, not monthly — *mare clausum* ran
  roughly November to March, and the monsoon reverses twice a year;
- `storm_season_phase` is a smooth cosine, so monthly resolution samples the
  same curve more finely and buys no new shape;
- **the build cost is linear in slices** (§1.4), and it is paid at campaign
  start, which is already the slowest step there.

`season_slices` is stored, so a future 12 is a data change, not a format change.

## 1.3 The read path — one accessor, and `days` keeps its meaning

`self.days[a*n+b]` is read in `dispatch`, `deploy_return_leg`, contract delivery
(`houses.rs`), colony lifelines, migration and the itinerary query. Converting
all of them at once is how a subtle bug ships. Instead:

```rust
/// Travel days for this lane RIGHT NOW — the annual mean (`days`) times this
/// lane's seasonal multiplier for the current slice. `days` itself keeps
/// holding the ANNUAL MEAN, so any consumer not yet converted behaves exactly
/// as it does today rather than silently reading a season it did not ask for.
#[inline]
pub(crate) fn lane_days(&self, a: usize, b: usize) -> f32 {
    let d = self.days[a * self.hubs.len() + b];
    if self.season_slices == 0 || !d.is_finite() { return d; }
    d * self.season_mult(a, b, self.season_slice_now())
}

#[inline]
fn season_slice_now(&self) -> usize {
    (self.day_of_year() as usize * self.season_slices as usize) / TICKS_PER_YEAR as usize
}
```

`day_of_year()` already exists (`mod.rs:5881`) and is read at exactly one site in
the whole tick — N5 gives it its second reader, which is the sketch's own note.
`Fair { month: u8 }` is the existing precedent for month arithmetic in the tick.

Conversion order: `dispatch` first (it is the one that matters), then the return
leg, then contracts, then lifelines. Each is its own commit and each is
bit-identical while the multipliers are 1.0.

## 1.4 The cost, stated before it is paid

Campaign start calls `compute_route_days_matrix` once. N5 makes it
`1 + SEASON_SLICES` calls: one annual (the existing `base_days`, unchanged) plus
one per slice. Each call is a coarse-grid build (cached per season) plus `base_n`
Dijkstra sweeps.

**Measure it before dosing it** — `campaign_start_sim` is user-facing latency,
not a background job. If 4 slices cost more than ~2× today's start, the fallbacks
in preference order are: (a) compute the seasonal matrices only for lanes with a
non-trivial hazard or elevation profile and leave the rest at 1.0 — most lanes on
most worlds are seasonally flat, so this should be a large win; (b) drop to 2
slices (open season / closed season), which is still the whole of *mare clausum*.

## 1.5 It must be a delay, never a wall — and why

A hard closure (`INFINITY` for a slice) is the historically literal mechanism and
is **deliberately not built first**. A closed lane stops food reaching a hungry
city, and the model's only response to that is `starving` — deaths. N1's own gate
names this ("food is the one cargo where throttling carriage kills people") and
N2's live trial is the precedent for a market-closing dose running away.

So the dose is a **multiplier with a cap**, `SEASON_MAX_MULT`, walked up from
1.0. A delay raises freight (`good_freight` is per-day) and postpones arrival,
which is most of what a closed season does economically, without the discontinuity.
The hard closure is a later, separately-gated slice — the same "Departure before
Rupture" discipline `schism.rs` already used.

## 1.6 What N5 is actually for

Not the price/distance gradient. The scorecard's **grain price CV *within* a
city reads 0.000 against a historical 0.30–0.50** — recorded in
`SCOREBOARD.md` (2026-08-19) as "the largest proportional error in the economy
oracle: a city's grain price is very nearly a constant over 60 years". A world
with no seasons cannot produce a seasonal price. **N5 is the most plausible fix
for that specific number**, and that is a better justification than the sketch's
own framing (which sold it as stacking with N1).

## 1.7 Gates

Per §4.1 rule 3, none of these is the metric N5 targets.

| Gate | Assertion |
|---|---|
| `n5_season_multipliers_at_unity_are_a_noop` | `season_slices = 0` or all `v = 0` ⇒ `lane_days == days` for every lane, and a season's worth of ticks is bit-identical |
| `n5_a_lane_is_dearer_in_its_stormy_season` | on a fixture with a real hazard profile, the closed-slice multiplier > the open-slice one, and an all-season lane's multipliers are equal |
| volume | total annual trade volume must not fall > 5% — a window redistributes voyages, it does not delete them |
| hunger | `lack_basic` must not rise materially at any hub |
| `simulate_decades_reports_dynamics` | wealth stays bounded (N2's blow-up is the precedent) |
| `econ_inheritance_rules_fragment_differently` | re-run per dose step, per §5.2 |
| `earth_` | asserted UNCHANGED — N5 must not touch the climate pipeline, only read it |

**The metric to watch (not gate on):** within-city grain price CV should RISE
toward 0.30–0.50 at a seasonally-closed port and NOT at an all-season one. Report
both; a CV that rises everywhere equally means the multiplier is a global freight
increase wearing a season's name.

## 1.8 Deliberately not built

- **A hard seasonal closure** (§1.5) — behind the dosed multiplier.
- **Seasonal PRODUCTION.** `seasonal_mult(h, g, doy)` already exists and already
  swings harvests. N5 touches carriage only; conflating the two would make the
  price signal unattributable.
- **A ship that waits for the season.** A vessel is not a thing yet
  (`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` §1) — "wait in port until spring"
  needs a `Vessel` with a location. Delay prices the wait instead.
- **Seasonal river ice as its own term.** `snow_frac`/`sst` are sampled but the
  existing seasonal block keys land closure off elevation. Extending it is a
  world-side change and belongs to whoever owns `build_coarse_cost`.

---

# N6 · Price-elastic demand

## 2.1 What exists already — half of this is built

In `mod.rs`'s needs assembly, category substitution:

```rust
let weights: Vec<f32> = members.iter().map(|&g| {
    let rel = (self.hubs[h].price[g] / self.goods[g].base_value.max(EPS))
        .max(PRICE_FLOOR_MULT);
    let pref = self.base_need(h, g) / total;
    pref / rel
}).collect();
let wsum: f32 = weights.iter().sum::<f32>().max(EPS);
for (mi, &g) in members.iter().enumerate() {
    needs[h][g] = total * weights[mi] / wsum;
}
```

So **cross-price elasticity within a category is live**: a dearer member loses
share to a cheaper one, `rel` is exactly `price / base_value`, and
`PRICE_FLOOR_MULT` already floors it. What is fixed is `total` — the category
aggregate. N6 is one multiplier on that aggregate, in the same vocabulary, at the
same site.

**A second finding, recorded and NOT fixed here:** this substitution is
budget-neutral in *quantity* — `needs = total * weights / wsum` conserves units,
not spend. A household that switches from wine to ale does not buy the same
number of units; it buys more of the cheaper thing for the same money.
Value-neutral substitution is a separate, arguably larger change; naming it keeps
it from being smuggled into N6's dose.

## 2.2 The three decisions that make this safe

**1 · Use the LAGGED price. This is what breaks the loop.**
`price` is a need, `need` would be a price — a simultaneity. But
`hubs[h].price[g]` is an EMA updated *after* needs are assembled
(`price = 0.6·price + 0.4·target`), so reading it is reading yesterday's price,
damped. No solver, no iteration, and the damping is already tuned. The
substitution block above already relies on exactly this property.

**2 · Apply it OUTSIDE `base_need`, never inside.** `base_need` has four other
callers: `need_scale`'s one-time calibration at campaign start, the food-need
sums that drive starvation (`mod.rs:6633`, `:6681`), and
`council_provision_pass`. Putting elasticity inside would (a) silently shift the
start-time calibration and (b) make the granary provision less exactly when
prices spike — backwards, and lethal.

> **The sharpest line in this design: elasticity belongs to the market, not to
> the ration.** `base_need` answers *what these people need*; the elastic value
> answers *what they will buy at today's price*. Those are different questions
> that currently share one function. Welfare signals (`lack_basic`, food
> balance, starvation, civic provisioning, crisis relief triggers) must keep
> reading the STRUCTURAL need; only market clearing reads the elastic one.

**3 · Elasticity is per TIER, and the staple is nearly inelastic.** This is the
historical shape, not a balance choice: grain's own-price elasticity is small
(≈ −0.1…−0.3), which is *precisely why* dearth prices spike so violently in
Persson's and Clark's series; comfort goods are moderate; luxuries are elastic.

```rust
/// N6 · own-price elasticity of the category aggregate, by need tier
/// [basic, comfort, luxury]. Shipped at [0,0,0] — a true no-op.
/// Historical target once dosed: [0.15, 0.50, 1.00].
const DEMAND_ELASTICITY: [f32; 3] = [0.0, 0.0, 0.0];
/// Elasticity may never move the aggregate outside this band, whatever the
/// price does. `PRICE_CEIL_MULT` is 12: at e = 1.0 an unclamped term would
/// collapse demand 12×, emptying a market on a price spike alone.
const ELASTIC_CLAMP: (f32, f32) = (0.55, 1.45);
/// Tier 0 floor. A starving population's grain demand is nearly perfectly
/// inelastic; the model has no "the poor go without and live" pathway, only
/// `starving`, so the floor is the guard the gate ("starvation must not rise")
/// is enforced by, not merely hoped for.
const SUBSISTENCE_FLOOR: f32 = 0.85;
```

## 2.3 The change

At the needs-assembly site, after `total` is computed and before the weights:

```rust
// N6 · own-price elasticity of the AGGREGATE. `rel` is the same
// price/base_value ratio substitution already uses one line below, and the
// same one N2's ban trigger reads — one vocabulary for "how dear is this".
// Uses the EMA price, i.e. YESTERDAY's, which is what makes this a lagged
// response rather than a simultaneous equation.
let e = DEMAND_ELASTICITY[tier.min(2)];
if e > 0.0 {
    let rel = (aggregate_price / aggregate_base).max(PRICE_FLOOR_MULT);
    let mut m = rel.powf(-e).clamp(ELASTIC_CLAMP.0, ELASTIC_CLAMP.1);
    if tier == 0 { m = m.max(SUBSISTENCE_FLOOR); }
    total *= m;
}
```

where `aggregate_price / aggregate_base` is the need-weighted mean `rel` over the
category's members (a singleton category is just that good's own ratio).
Structural need is kept alongside for the welfare readers:

```rust
// needs[h][g]        — what the market clears on (elastic)
// needs_struct[h][g] — what people NEED (inelastic); lack_basic, food balance,
//                      starvation, provisioning and relief all read THIS.
```

`needs_struct` is a second `Vec<Vec<f32>>` built in the same pass — it is exactly
today's `needs`, so at elasticity 0 the two are equal element-wise, which is both
the no-op proof and the migration path.

## 2.4 The couplings nobody would notice until they bit

Named up front, because the N2 blow-up was exactly a coupling nobody predicted:

- **Elasticity shrinks measured scarcity.** `lack_basic` is unmet demand; shrink
  demand and a city in dearth reports itself healthier — which would suppress
  `decide_crisis_relief`, `council_provision_pass` and (once dosed) N2's ban.
  §2.2 decision 3 is the fix: those all read `needs_struct`.
- **Elasticity damps the very price signal N2 triggers on.** With N2 dosed above
  zero, a price spike would now be partly self-correcting, so N2's trigger fires
  less often. That is *correct economics* and a *changed dose* simultaneously —
  so N6 and N2 must never be dosed in the same commit.
- **`live_price` reads `need`.** Price = `base·(need/stock)^k`. Elastic need
  lowers the price, which raises need again next tick — a negative feedback loop
  (stabilising), damped by the EMA. Stabilising loops are safe but they can
  *hide* a broken parameter by absorbing it; watch price CV, not just levels.
- **`need_scale` is calibrated once, at start, from `base_need`.** Untouched by
  design (§2.2 decision 2) — verify it, don't assume it.

## 2.5 Gates

The plan's own framing is right and unusually strict: *"a 'no regression
anywhere' change, not a 'win one metric' change."*

| Gate | Assertion |
|---|---|
| `n6_elasticity_at_zero_is_a_noop` | `DEMAND_ELASTICITY = [0,0,0]` ⇒ `needs == needs_struct` element-wise and a year is bit-identical |
| `n6_the_ration_is_not_elastic` | with elasticity dosed high, `lack_basic` / food balance / provisioning read the STRUCTURAL need — assert a dear-food hub still provisions and still reports its real dearth |
| `n6_a_dearer_good_is_bought_less` | the direction, on a fixture: same stock, doubled price ⇒ strictly lower aggregate need, and a tier-0 good falls by less than a tier-2 one |
| starvation | `starving` and `lack_basic` must not rise at any dose. If elastic demand makes the poor stop eating, the term is in the wrong place |
| every `econ_` band currently passing | must still pass — no exceptions |
| `simulate_decades_reports_dynamics` | bounded wealth, turnover unchanged in kind |
| `econ_inheritance_rules_fragment_differently` | re-run per dose step (§5.2) |

**Dose walk:** 0 → `[0.05, 0.15, 0.30]` → `[0.10, 0.30, 0.60]` → the historical
`[0.15, 0.50, 1.00]`. **Expect to stop early, and write down where.** If the walk
stops at `[0.05, 0.15, 0.30]`, that number is the deliverable (§4.1 rule 5).

## 2.6 Deliberately not built

- **Value-neutral substitution** (§2.1) — a real finding, its own change.
- **Income elasticity / a budget constraint.** `Society` shares exist but no
  household budget does; a real Marshallian demand system is a different
  project and would subsume this one.
- **A price ceiling.** `polis.rs` already argues the case against: a ceiling's
  whole historical consequence is that it *causes* shortage, and the shortage is
  currently unconditional. **N6 is what makes a ceiling modellable** — once
  demand responds to price, a ceiling has something to bind on. State it as the
  sequel, don't build it here.
- **Elasticity of SUPPLY.** Production is exogenous (`tech_factor`); nothing
  produces more because the price rose. Naming this keeps N6 from being mistaken
  for a market-clearing model.

---

# N7 · The League

## 3.1 The negative definition IS the design

A `Realm` is sovereignty: provinces (`prov_realm`), a capital, succession by
`LineRule`, a genealogy, a treasury that *is* the dynasty's money, taxation, and
vassals. A League is none of those. Its members keep their own government, their
own treasury, their own provinces, and **the right to leave**.

That is why it cannot be a `Realm`, and it is also the failure mode: a League
that acquires a capital, then a purse it taxes with, then a member it will not
release, has become a Realm one field at a time. Every field below is chosen to
make that regression visible rather than gradual.

```rust
/// N7 · a VOLUNTARY association of hubs that stay independent. Not a realm:
/// no provinces, no capital, no succession, no writ. The one collective verb
/// is the boycott (§4.1).
pub struct League {
    pub id: u32,
    pub name: String,           // via the seat's culture kit, as `guild_name_for` does
    /// Where the diet MEETS. Deliberately not called a capital: it holds no
    /// writ, no province, and carries no authority over any member.
    pub seat_hub: u32,
    /// Dues, and the only pot. Spent on the collective act; never taxed from a
    /// member's provinces (that would be sovereignty).
    pub purse: f32,
    pub founded_tick: u32,
    #[serde(default)] pub dissolved_tick: u32,
    #[serde(default)] pub boycotts: Vec<Boycott>,
    #[serde(default)] pub events: Vec<RealmEvent>,   // reuse; same shape, same cap
}
```

**Membership lives on the hub, not in a `members` vec** — `TickHub.league: i32`
(−1 = none), authoritative, exactly as `TickHub.realm` is. `Realm`'s own doc
comment records why a second list was *removed*: "a second copy of the same fact
with no mechanism keeping it in sync". One league per hub, and multiple
simultaneous memberships (historically real — a city could sit in the Hanse and
a regional *Landfriede*) is on the not-built list rather than invited in.

Note the layer count: this is a **fourth** authority axis over rule 27's three
(`prov_holder` seat · `prov_holder_house` dues · `prov_realm` sovereignty). It is
admissible only because it is not territorial — a league holds no province and
its writ is a lane rule, not land. That distinction has to hold or rule 27's
"three layers, all independent" becomes four and unfalsifiable.

## 3.2 Formation — from state that already exists

Yearly. A candidate seat is a tier-1/2 hub (`assign_city_tiers` already computes
this and R-phase Path B already reads it) that is **not** a realm capital.

Three signals, all already computed:

- **A shared lane.** `flow_year` carries per-pair realised flow. A member
  candidate needs a flow tie to the seat above a threshold — a league is a
  trading bloc, so shared commerce is the precondition, not proximity.
- **A shared threat.** Any of: an adjacent realm of high `rank`; an active `War`
  in the component; `run_piracy` losses in recent years. Leagues form *against*
  something; without this term they form everywhere and never dissolve.
- **Free to join.** `hub.realm < 0`, or a realm member with high `Realm.autonomy`
  (an autonomous crown city joining a merchant league is the historically normal
  case; a centralised crown's city is not).

> **A determinism trap, named because this codebase has already been bitten by
> it:** `flow_accum` is a `HashMap<(u32,u32), f32>`. Tick determinism was a real
> shipped defect fixed at four hash-order sites, and N4's gate says the pick
> "must be a `hash01` draw, never an RNG or a `HashMap` order". Any pass reading
> `flow_year`/`flow_accum` **must sort by key before iterating**.

## 3.3 Dissolution — the gate, not an afterthought

The plan's own gate: *"leagues must form **and dissolve** — a monotone member
count is a failed build, the same failure `realm_secession_pass` exists to
prevent."* Four exits, all mirroring passes that already exist:

1. **Annexed.** A member taken by a realm (war goal, vassalage) leaves on the
   spot — sovereignty outranks association.
2. **The threat lapses.** No threat signal for `LEAGUE_DRIFT_YEARS` ⇒ members
   drift out one at a time. This is the one that keeps the count non-monotone in
   a quiet world, and it is the mechanism most likely to be tuned away by
   accident; gate it directly.
3. **Dues unpaid.** A member that cannot pay leaves (or is expelled by the diet).
4. **The seat falls.** Seat abandoned/absorbed ⇒ the diet moves to the largest
   remaining member, or the league dissolves below `LEAGUE_MIN_MEMBERS`.

Modelled on `realm_secession_pass` deliberately: it is the proven non-monotone
pass in this tree.

## 3.4 The diet, and the one collective verb

Yearly, at the seat, on the `decide_*` / `apply_*` split (FIX_PLAN B2) so a
player holding the seat can supply the choice later:

```rust
pub(crate) fn decide_league_diet(&self) -> Vec<LeagueChoice>;  // pure &self
pub(crate) fn apply_league_diet(&mut self, choices: Vec<LeagueChoice>);
pub(crate) fn run_league_diet(&mut self);                       // the entry point
```

A diet sets dues, admits/expels, and may vote **one boycott**. Vote weight is the
open question flagged by N4's regression: weighting by wealth or throughput is a
wealth-correlated channel of exactly the kind that inverted the inheritance gate.
**Recommendation: one member, one voice**, and record wealth-weighting as an
explicit not-built. It is also the historically defensible reading of a Hanseatic
*Tagfahrt*.

## 4.1 The boycott — and the dependency the sketch got wrong

The sketch says the boycott is "an N2 cargo-ban voted by every member at once".
**N2 as built cannot express it.** `TickHub.export_ban_until` is indexed by
*good* — "this city bars export of iron", to anyone. A boycott is "we bar trade
with *Ragusa*", which is a **lane**. So:

```rust
/// N7 · a LANE-scoped ban: this hub will not trade with `target`
/// (optionally only in `good`, −1 = all) until `until_tick`.
/// N2 extended: `export_ban_until` bans a GOOD to everyone; this bans a
/// PARTNER. Checked in dispatch's target loop, where `quarantined[b]` already is.
pub struct Boycott { pub target: u32, pub good: i32, pub until_tick: u32 }
```

Enforced exactly where the proven shape already sits — precomputed once per
dispatch, read in the target loop beside `quarantined[b]` and
`export_ban_until`. The list is tiny (members × active boycotts), so the
precompute is a small per-hub bitset, not a scan.

**This changes the build order.** N7 does not depend on "N2 dosed above zero"; it
depends on **N2 extended to lanes**, which is a separate slice of N2 that was
never built. Say so in `ACTORS_AND_CARRIAGE_PLAN.md` §3.7 rather than discovering
it mid-build.

## 4.2 The risk N7 inherits, quantified

N2's single-city export ban produced a **1,005,714** sustained richest house and
broke the hard-asserted wealth bound at two separate doses. **A boycott is that
same market closure, applied by every member at once, against a chosen target.**
The concentration risk is not analogous — it is the same mechanism, multiplied.

Therefore the build order inside N7 is not negotiable:

| Slice | Content | Dose |
|---|---|---|
| N7.1 | `League`, `TickHub.league`, formation, dissolution, dues, the diet — **and no boycott at all** | live; a league that forms, meets, collects and dissolves is a complete, safely gateable thing |
| N7.2 | `Boycott` struct + dispatch enforcement, authored by nobody | zero dose (empty list), gated inert |
| N7.3 | The diet may vote a boycott | walked from `LEAGUE_BOYCOTT_MAX = 0` |

N7.1 alone is worth shipping: it puts a real institution on the map and in the
chronicle without touching what moves.

## 4.3 Gates

| Gate | Assertion |
|---|---|
| `n7_a_world_with_no_leagues_is_bit_identical` | zero leagues ⇒ every pass early-returns, a year is bit-identical (the `province_land_pass_is_a_noop_without_provinces` discipline) |
| `n7_leagues_form_and_dissolve` | on `realm_reference_world`: member count must go **down** as well as up over the run. A monotone count fails |
| `n7_a_league_is_not_a_realm` | a league holds no province, sets no `prov_realm`, has no succession, and a member's `hub.realm` is untouched by joining |
| `n7_boycott_is_inert_at_zero` | empty boycott list ⇒ dispatch bit-identical |
| `n7_a_boycotted_city_reroutes` | the target's share of its component's throughput falls and its partners shift — measured as throughput share, **not** as a wealth delta (§3.2's own gate wording) |
| `simulate_decades_reports_dynamics` | **the load-bearing one.** Wealth bounded. N2's blow-up is the precedent and this is the same mechanism multiplied |
| `econ_measure_realm_formation` | realm formation must not collapse — a league is a rival to a crown, not a replacement |
| `econ_inheritance_rules_fragment_differently` | per dose step |

New instrument: `econ_measure_league_formation` on `realm_reference_world` —
leagues founded/century, mean lifespan, mean size, and the up/down member-count
series. That world already has 72 cities, contiguous cultures and a real
neighbour graph, and was built precisely because the scorecard's world could not
express formation mechanics.

## 4.4 Deliberately not built

- **Multiple simultaneous memberships** (§3.1) — one `hub.league`, per the Realm
  precedent.
- **A league army, navy or war.** "A city cannot march an army" is doctrine here
  and `WAR_MAX_DIST_FRAC` depends on it. A league's weapon is the boycott.
- **League courts / arbitration.** `arbitrate_feuds` already arbitrates between
  houses; extending it to cities is a separate mechanism.
- **A league treasury that lends.** `Bank` exists and is the well-modelled actor;
  a lending league is a bank with a flag.
- **Wealth-weighted voting** (§3.4) — the N4 lesson, held out on purpose.
- **A league becoming a realm.** Tempting and historically attested; it is also
  exactly the regression §3.1 exists to prevent. If it is ever built it must be
  an explicit, chronicled transition, not a drift.

---

## 5. Build order across the three

```
FIRST     N5   sailing window     independent; no economic feedback; fixes a
                                  NAMED scorecard error (within-city CV 0.000)
THEN      N6   elasticity         self-contained, walk from 0, expect to stop early
                                  — never dosed in the same commit as N2
LAST      N7   the League         needs N2 EXTENDED TO LANES (§4.1), and inherits
                                  N2's measured concentration risk × members
```

N5 first is a change from the sketch's "N5 can run in parallel". The reason is
evidence: N5 is the only one of the three with (a) most of its machinery already
written, (b) no feedback into wealth concentration, and (c) a specific, named,
badly-wrong number to move. N6 and N7 both touch the mechanisms that have now
broken a hard gate twice in one session.

## 6. The one-line summary of each design

- **N5** — stop passing `season = -1`. Store a quantised per-lane multiplier,
  read it through one accessor, dose it as a delay and never a wall.
- **N6** — demand's *aggregate* is what is inelastic, not demand. One lagged
  multiplier on the category total, outside `base_need`, with the welfare
  signals kept on structural need. Elasticity belongs to the market, not the
  ration.
- **N7** — a league is a realm's negative: no land, no crown, no succession, one
  verb. Ship the institution first and the weapon last, because the weapon is
  the thing that already broke the economy once.
