# Exploration, the known world, and transport modes — design

**Status: DESIGN, nothing built. Open questions at the end need answering before
this is buildable.** Companion to `OUTPOST_CONNECTIVITY_AND_ENTREPOT_PLAN.md`,
which measured the problems this design answers.

The premise, restated from the measurements: houses are **omniscient** (G5 —
`colonizable` is a whole-world list snapshotted at campaign start, scanned in full
on every founding call, with no `explored`/`discovered`/`surveyed` state anywhere
in the tick), outposts are **sited without regard to whether cargo can leave**
(G6 — `delta` and `chokepoint` exist on the site struct and the house path reads
neither), and **water carriage costs the same as overland** (G8 — one global
`days_per_cell`, with `fleet_river` pooled into `cap_land` alongside ox-carts).

---

## 1 · The known world

### 1.1 Two layers of knowledge, not one

The design asks for two distinct things, and they should stay distinct because
they are acquired differently and decay differently:

- **DIRECT knowledge** — a place this settlement's or house's own people have
  physically reached. Acquired by an expedition or by trading there.
- **REPORTED knowledge** — a place known *because someone we trade with has been
  there*. Acquired by contact, second-hand, and **less reliable**.

That distinction is the whole value of the mechanic. A one-layer fog is just a
map filter; two layers give you the thing that actually drove pre-modern
commerce — merchants knowing *of* a market they have never seen, on the word of
someone who has. It is also the natural join with
`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` stage 4, where a house trades on the
price it *believes*, with a spread set by how fresh its knowledge is.

**Proposed encoding.** Knowledge is held **per province**, not per cell:

```rust
/// What one knower (a house, or a city) knows about one province.
struct Known { level: u8, since_tick: u32, source: i32 }
// level 0 unknown · 1 reported (hearsay) · 2 surveyed (an expedition returned)
//       3 established (we trade or hold there)
```

Province granularity is the right unit and it is not a compromise: provinces are
already the world↔campaign join (FIX_PLAN B1), already carry `prov_neighbors` for
contiguity, and are ~200-400 km across — about the resolution at which
pre-modern knowledge actually existed ("the Baltic shore", not a 11 km cell). A
per-cell fog would be a large per-knower raster and would imply a precision
nobody had.

**Storage cost is the first real design constraint.** `houses × provinces` at,
say, 40 houses × 300 provinces is 12,000 entries — trivial. Per *settlement*
(hundreds of hubs) it is larger but still bounded. Per cell it is not. This is
the main argument for provinces beyond realism.

### 1.2 Expeditions become the acquisition mechanism

`expedition_launch_pass` and `route_prospects` already exist and currently gate
nothing (measured). This design gives them their job: **an expedition is how
`Known.level` rises from 0/1 to 2**, and an outpost may only be founded at level
≥ 2.

Routing, per the brief — expeditions should follow known trade routes where they
exist, and otherwise **coasts and rivers**, which is both the historical pattern
(coastal crawl before open-water navigation; the Russian frontier moved along
river systems; the American west along the Missouri and the Platte) and cheap to
implement, since the campaign will already know navigable water once Slice D
lands.

Risk, per the brief — an expedition can **fail and be lost**. This is what makes
distance a real constraint rather than a number: risk should rise with distance
beyond the known frontier, with unfamiliarity, and with the hostility of what is
crossed. A lost expedition should still be worth something (a partial report at
level 1, "reported"), because a total loss teaches the player nothing and makes
the mechanic feel arbitrary.

`EXP_START_TICK` already exists; the brief sets it to **year 25**.

### 1.3 What the map shows

A **known-world layer**: provinces at level 0 drawn as unsurveyed, level 1 as
sketched/uncertain, levels 2-3 in full. Whose knowledge is shown is a UI choice —
the selected house, the selected city, or the union of the player's holdings.

This is a **view**, so CLAUDE.md rule 14 / §8.17 applies: it may never change what
the world *is*, only what is drawn. That matters here because a fog layer is
exactly the kind of feature that tempts a "hide the data" implementation which
then leaks into generation.

---

## 2 · Transport modes (the prerequisite)

Restating from the companion plan because §1 depends on it: give a route a
**mode** with its own per-day cost and its own capacity — sea ≪ river < road <
track — and split `cap_land` back into `fleet_river` and `fleet_caravan`.

Per the brief, three things differ by mode and all three should be modelled:
**days** (speed), **capacity** (how much moves per slot), and **risk** (loss
probability). Water should win on all three, which is what makes it preferable
without a special case. Worldgen already knows which rivers are navigable
(`River.navigable`); that flag needs carrying into the campaign snapshot.

**This is the first thing to build**, ahead of the fog: it is the lever most
likely to move the known market-integration defect (a −0.064 price/distance
gradient), it is testable against an instrument that already exists
(`econ_fidelity_scorecard`), and — importantly — **it may make much of the rest
unnecessary**. If water carriage is genuinely cheap, an outpost near navigable
water becomes valuable through ordinary route cost, with no bespoke entrepôt
rule and no siting special case.

---

## 3 · Province view: manufactured goods, and richer detail

### 3.1 The bug — measured

The Province Inspector shows **Books & Manuscripts** as a province good. It is a
manufactured good (`goods_spec.rs`: `mg("books", … inputs: paper 0.8, dyes 0.1)`,
which sets `Distribution::Manufactured`) and has no belt or deposit, so it should
never appear.

The chip row is built in `ProvinceInspector.tsx`:

```ts
const beltGoods = (potential?.goods ?? [])
  .filter((g) => !g.is_deposit)     // ← the ONLY structural filter
```

and its source, `campaign_province_potential` (`campaign_commands/province.rs`),
filters only on magnitude:

```rust
for g in 0..ng {
    let belt = sim.prov_good_belt.get(idx).copied().unwrap_or(0.0);
    if belt <= PROV_GOOD_ABSENT_BELT { continue; }
    goods.push(ProvinceGoodPotential { … });
}
```

**Nothing anywhere filters `Distribution::Manufactured`.** The query already
computes an `is_deposit` map from the same specs and passes it through as a flag;
there is simply no manufactured equivalent.

Verified as the correct behaviour on the generation side, so the guard is the only
thing missing: `compute_trade_goods` does write `buf.goods[slot] = vec![0u8; n]`
for a `Manufactured` good, and an all-zero belt lands exactly on
`PROV_GOOD_ABSENT_BELT` and is excluded.

**Honest limit of this diagnosis:** that means something has put a *non-zero* belt
in the `books` slot in this particular world, and I could not determine which from
the code alone. Two candidates, both plausible and distinguishable only against
the actual save: (a) the world was generated before `books` was in the spec, or
the spec was edited after generation, leaving a **stale column** at that index —
CLAUDE.md §8.20's "fixed indices in `TileData.goods`" is exactly this hazard; or
(b) a length mismatch between `Province.good_belt` (sized by `buf.goods.len()`)
and the campaign's `goods.len()` misaligning the flat `prov_good_belt` row.

**Either way the fix is the same and should not wait on that answer**: exclude
manufactured goods structurally, the way `is_deposit` already is. That both fixes
the symptom and makes the class of bug impossible, rather than repairing one
world's data. `generate_provinces` should additionally receive a belt-good mask —
today it takes no goods spec at all and therefore *cannot* tell a manufactured
column from a belt column, which is the structural root.

**Gate:** `a_province_never_lists_a_manufactured_good` — assert no
`ProvinceGoodPotential` carries a `Manufactured` distribution, on a world whose
spec includes them.

### 3.2 Richer goods and deposit detail

The requested elaboration, using data that already exists and is not surfaced:

- **Per good:** belt quality *and* its grade word (the served
  `deposits::grade_label` vocabulary — coarse/ordinary/good/fine/exquisite,
  §8.19), area covered, whether a named **locality** sits here and its grade
  (`GoodLocality` already carries `name`, `grade`, `extent`, `river_fed`),
  live `exploitation` vs `potential` (§2.5 already computes both), and
  `market_share`.
- **Per deposit:** `ProvinceDepositDot` already carries `grade`, `extent` and
  `depth` per working — none of which is shown. Depth especially: "flooded" is a
  real economic fact (§8.16's "visible but largely LOCKED"), and nothing reads
  `depth` anywhere yet, which is `DEPOSITS_AND_MINING_PLAN.md` slice 4's own note.

Both are **read-only presentation of existing state** — no new sim, no new
persisted field — which makes this the cheapest item in the whole set.

---

## 4 · Suggested build order

1. **Transport modes** (§2). Biggest lever, existing instrument, may subsume later work.
2. **Manufactured-goods filter + province detail** (§3). Small, self-contained, visible.
3. **Outpost siting reads `delta`/`chokepoint`** (companion plan E1). A few lines.
4. **Knowledge state + expeditions as the gate** (§1). The large one; build last,
   and only after re-measuring whether 1 and 3 already produced sensible siting.

---

## 5 · Open questions — these block §1

These are genuine forks where different answers give materially different games.

1. **Who is the knower — the house, the city, or both?** The brief says
   "settlement/house depending on who is the colony founder", which implies both.
   Both is the most faithful and roughly doubles the state and every lookup. Is a
   house's knowledge inherited on succession? Shared with a house it allies or
   marries into? *Recommendation: start with houses only — they are the actors
   that found outposts — and add cities only if the city-founded colony path
   needs it.*

2. **Does knowledge decay?** A province surveyed in year 30 and never revisited
   by year 200: still known? Decay makes the map live and gives repeat voyages a
   point; no decay is simpler and avoids a player re-surveying the same ground
   forever. *Recommendation: no decay of `level`, but `since_tick` ages so the
   information is stale for price purposes — which is exactly the hook stage 4
   of the vessels plan needs.*

3. **Does the fog gate anything besides outpost founding?** Trading with a city
   you have never heard of, a house's goal targeting an unknown province, the
   route matrix itself? Gating **trade** is the realistic reading and by far the
   most invasive — it would reshape the whole economy and every `econ_` band.
   *Recommendation: gate founding and goals first; treat gating trade as its own
   later, separately-measured change.*

4. **Is this player-visible agency or pure AI?** Does the player *send* an
   expedition (a new mutating verb — there are currently four, §5.1), or watch
   houses send them? *Recommendation: AI-driven first, matching how every other
   campaign system works today; a player verb is a clean addition afterwards.*

5. **What does "taken by natives" mean mechanically?** There is no unlanded/native
   population in the model at all — provinces have `prov_rural` but no notion of
   an unincorporated people. Is this a flat distance-scaled risk, or does it need
   a real frontier-population concept? *Recommendation: flat risk scaled by
   distance beyond the frontier and by province emptiness; a real native polity is
   a much larger design (and arguably `historical-society`'s question, not this
   plan's).*

6. **What happens to a world already in progress?** Every existing campaign has no
   knowledge state. Seed all provinces containing a house's holdings at level 3
   and their neighbours at 1, or start everyone blind? *Recommendation: seed from
   holdings — starting a year-200 campaign blind would strand every existing
   house.*

7. **§2 first — does it change the answer to any of the above?** If cheap water
   transport alone produces sensible outposts, the fog becomes a *legibility and
   pacing* feature rather than a corrective one, and could be scoped down a lot.
   Worth re-asking after step 1.
