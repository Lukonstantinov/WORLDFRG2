# Houses, Guilds & the Settlement Market — one-session build plan

**Status: S2/S3/S4/S5/S7/S8/S9/S10/S12a BUILT AND GATED; S1 BLOCKED
(pre-existing negative result); S6 DOSE-WALKED TO 0.3 AND REVERTED (a real
negative result, see below); S11/S12b/S12c QUEUED.** Written 2026-09-22 from
a measured brainstorm over `sim/campaign/tick/`, `render/`, `src/ui/campaign/`.
See `CLAUDE.md` §5.6 for what shipped, and for the discovery that
`N1B_OWNERLESS_LOSS_RATE`'s own doc comment already records a dose walk
attempted before this plan existed — 0.01 made `dense_world`'s uncapped
trade volume rise 6.4× rather than fall, a structural feedback in the
target-room calculation, not a tunable collapse. S2 (the *annona* carrier
class) is what the plan asked S1 to be protected by; it shipped, verified
additive (not a subtraction from `tw_local`, which the plan's own text implied
but which checking showed would NOT have been inert), and does not by itself
unblock S1. S3 (the craft guild roster unfreeze) shipped, but its own dose
walk (`GUILD_MAX_PER_CITY` 1 → 3) turned out UNTESTABLE: every standing gate
fixture (`reference_world`/`reference_world_large`/`dense_world`/
`simulate_decades_reports_dynamics`) carries zero manufactured goods, so
guild founding is structurally inert on all of them regardless of the cap —
recorded at the constant's own doc comment rather than silently shipped as a
validated dose. S5 (transit demand) shipped exactly as the plan specifies —
inert at `TRANSIT_DEMAND_DOSE = 0.0`, its own dose walk explicitly deferred
to queue item Q2. S6 (the routed wartime blockade) was walked to 0.3 (this
plan's own §3 instruction) and REVERTED: `simulate_decades_reports_dynamics`
failed its bounded-wealth assertion and `the_relay_carries_long_lanes_in_
stages_on_a_realistically_dense_world` failed its own "the relay is inert
with the caps off" assertion, because `BLOCKADE_STAGING_DOSE` gives the
shared `staging_hop` relay a THIRD trigger (`war_with`) that test's "loose"
fixture never accounted for. Recorded at `BLOCKADE_STAGING_DOSE`'s own doc
comment (`mod.rs`) rather than silently reverted with no trace — new queue
item Q16. This plan's own §9 risk register names three doses as the session
ceiling; S1 (investigated, found already blocked), S3 (walked, found
untestable) and S6 (walked, reverted) are the three spent here — S8/S9 (the
house/craft atlas queries, `campaign_house_atlas`/`campaign_guild_atlas`)
shipped afterward in the same session since neither is a dose: both are
pure derived reads touching no tile/sim state, gated by `cargo check`/`tsc`
alone per this plan's own §4 note. S10 (the four-window split) shipped in
the same pass — asked for explicitly by the maintainer despite this
environment having no display to open the app in (the "attempt it blind,
carefully" choice, over stopping or a narrower slice): `HousesPanel.tsx`
went 1,455 → 338 lines (browse-only), `HouseDetail` and its ten subtabs
moved verbatim into `HouseDossier.tsx`, a new `FeudsAlliancesPanel.tsx`
wraps the existing `FeudsView` as its own window, and `House.is_guild`
became a filter chip instead of a tab. Verified by `npx tsc --noEmit` and a
full `vite build` (both clean) — type-correctness and bundling, never a
human looking at the running window, which is owed before trusting the
four windows visually. S12a (the Houses bump chart) shipped in a follow-up
session on the same branch: a new `campaign_house_bump_chart` query
reconstructs each of the top ~12 houses' wealth RANK per year from its own
`wealth_history` (checked first — 80 years capped, past the 50-year window
this needs, per this plan's own §9 risk-register note), rendered as an SVG
line chart above the house list with four quiet gauges beside it. Of those
four, only TWO ship as real sparklines (`families`/`top-10% share`, sourced
from `campaign_get_inequality`'s existing yearly `series`) — `founded`/
`fallen` ship as plain totals rather than a fabricated history, because no
per-year series for either exists anywhere in the sim (new queue item Q18).
A bottom "pulse" ticker reuses the existing world journal query
(`campaign_get_journal(-1,-1)`, `NewsFeedPanel`'s own data source) filtered
to house-ish kinds — the plan's own §7 build-order table already named this
"a sixth caller of an existing mechanism, not a new system" for S11's map
lanes; the same discipline applies here to an existing query. Pure derived
read, gated by `cargo check --lib --tests` + `npx tsc --noEmit` + a clean
`vite build` alone (§4's own note: neither can move a tile/sim gate) — same
no-display caveat as S10, folded into queue item Q17.

This plan is scoped to **one working session**. It is ordered so that value
lands early and the elastic work is at the end: if the session runs short,
stop at the S-marker in §7 and everything before it is coherent and shippable
on its own. Everything this plan does not reach is a **numbered queue** in §8,
each item naming what it waits for (rule 36 — "deliberately not built" is not
a way to close scope).

---

## 0. Four decisions, taken

1. **Era is a Roman/medieval MIX**, not a fork. The medieval layer already in
   the tree (dynastic houses, banks, the Monte, leagues) stays. The Roman
   layer (state carriage, compulsory corpora, monopolies, customs frontiers)
   is added *beside* it. No `EraProfile` switch is built in this session —
   every Roman-flavoured slice below ships era-NEUTRAL so the profile can be
   added later without rework. That is queue item Q1, not a refusal.
2. **The *annona* reframe applies to BIG CITIES ONLY.** State grain carriage
   is a metropolitan phenomenon; a market town has no state fleet calling on
   it. Concretely: the ownerless residual keeps its free ride only where the
   DESTINATION clears `ANNONA_MIN_POP`; everywhere else it pays S1's real
   voyage risk like anybody else.
3. **The player stays OBSERVATION-ONLY.** No new verbs. This makes every item
   here a SPECTACLE problem — which is why half the plan is atlas and UI, and
   why §6 is not decoration.
4. **Every behavioural change ships DOSED FROM ZERO** unless it is provably
   inert by construction. The record (`COMFORT_IMPORT_FRAC`, N1c, N2, S7
   household monetization) is five separate occasions where an un-walked dose
   inverted a gate.

---

## 1. What is already built and unused — read before writing code

Four mechanisms in this plan are **already fully wired and dead at a zero
dose**. Finding them is most of why this plan fits in a session.

| Mechanism | Where | State |
|---|---|---|
| Ownerless cargo loss | `production.rs:2160`, `N1B_OWNERLESS_LOSS_RATE` | roll + consequence + `diag_lost` all built; constant is `0.0` |
| Routed wartime blockade | `BLOCKADE_STAGING_DOSE` | diverts through `staging_hop`; constant is `0.0` |
| Carriage capacity bind | `CAPACITY_BIND_DOSE` | built; `0.0` |
| House trading web on the map | `OverlayManager.ts:1602` | draws seat→city lanes through the real corridor resolver; undirected, unlabelled, no goods |

And three data sources the UI never reads: `House.trade_at` (`mod.rs:4867`),
`TickHub.supply_accum`/`demand_accum`, `CraftGuild.signature`/`brand_name()`.

**Two findings that shape the design:**

- **`house_for` returns `-1` for 95.7% of shipments.** "Local merchants" is
  not an actor — it is the `else` branch. It takes no vessel slot, is not
  clamped by capital, never sinks, and cannot be barred. `stock_take` sits
  OUTSIDE carrier resolution, so the cargo moves either way.
- **The word "guild" names two unrelated things.** `House { is_guild }` is a
  civic merchant company (same struct, a flag, FEWER organs). `CraftGuild` is
  the producers' body — capped at **12 for the whole world**, seeded ONCE at
  tick 0, never founded or dissolved again, against 23 manufactured goods.
  The UI shows them in two panels under one word.

---

## 2. Gates this plan runs

Per §2.8's routing table. `sim/campaign/tick/**` changes owe:

```
cargo test --lib tick::tests
cargo test --lib econ_ -- --nocapture
cargo test --lib simulate_decades_reports_dynamics -- --nocapture
```

`sim/step8_biological_goods/**` changes owe `cargo test --lib goods_ -- --nocapture`.
Frontend owes `npx tsc --noEmit`. No `earth_` run is owed anywhere in this
plan — nothing here touches `step3_ocean_atmo/` or `step4_climate/`.

**The multi-seed `econ_inheritance_rules_fragment_differently` is the
instrument that blocks every dose below.** It is ~6 minutes in debug and has
flipped inside its own noise band five times. Run it **per dose step**, never
per slice.

---

## 3. Backend slices — the economy

### S1 · Dose ownerless voyage risk (`N1B_OWNERLESS_LOSS_RATE`)

**One constant.** The roll, the `else if lost { … continue; }` consequence and
the `diag_lost` counter are all already there and unreachable.

Walk `0.0 → 0.006 → 0.012`, stopping at the last value where all three gates
read healthy. Do NOT jump straight to a target: N1c's own record shows a
routing rule that is safe at one dose collapsing a world at another, and the
response here is expected to be monotone but has never been measured.

- Gate: `tick::tests` + `econ_` + the dynamics run's bounded-wealth assertion.
- Gate that is not the target: `the_dosed_economy_stays_healthy_on_a_realistically_dense_world`.
- Expected direction: ownerless share DOWN, house share UP, total volume
  slightly down. A volume collapse past ~15% is a STOP, not a tuning step.

### S2 · The *annona* — state carriage into great cities

`ANNONA_MIN_POP` (start at 60,000 — the same order the old `size_bonus`
saturation used, so it names a genuine metropolis on a real world).

An ownerless shipment whose DESTINATION hub clears `ANNONA_MIN_POP` is
**state carriage**: exempt from S1's loss roll, and exempt from
`house_barred`-style exclusion when that ever binds. Everywhere else the
residual takes the full S1 rate.

This is a relabel plus one branch. Its value is that it turns the project's
single worst measured anomaly into a named feature instead of a defect, and
it is what makes S1 safe to dose harder: the lanes that genuinely must not
be disrupted (grain into the great cities) are explicitly protected.

- Also: the carrier CLASS written at `production.rs:2203` gains a fourth
  value, `tw_state`, split out of `tw_local`. `supply_accum` gains
  `SUPPLY_STATE` the same way N8 added `SUPPLY_LOCAL`. Nothing in the tick
  reads either, so this cannot move a number by construction — verify that,
  do not assume it.
- Gate: a new `annona_carriage_is_exempt_and_only_for_great_cities` (a small
  city's ownerless cargo CAN sink; a metropolis's cannot) plus `econ_`
  bit-identical at `N1B = 0.0`.

### S3 · Unfreeze the craft guild roster

The single most binding constraint in the campaign: `GUILD_MAX = 12`
world-wide, `guilds_seeded` at tick 0, no founding or dissolution pass ever.

- Replace the world cap with `GUILD_MAX_PER_CITY` (3) and a much larger world
  ceiling used only as a sanity bound.
- `maybe_found_craft_guild` (yearly): a hub with real output and accumulated
  `tradition` in a manufactured good, and no guild in it yet, founds one.
- `maybe_dissolve_craft_guild` (yearly): a guild whose hub is dead, or whose
  good has not been made here for N years, dissolves. Chronicled both ways —
  per `INSTITUTIONS_BUILD_ORDER`'s governing rule, a slice names what it
  writes to the chronicle or it is not done.
- Keep `maybe_poach_master` and `maybe_steal_quality` untouched; they finally
  have a population to act on.
- Gate: `a_craft_guild_is_founded_and_dissolved_over_a_century`,
  `guild_count_per_city_is_bounded`, and the dynamics run (guild count is an
  input to civic stability, so this CAN move wealth — treat it as a dose:
  ship `GUILD_MAX_PER_CITY` at 1 first, then 3).

### S4 · Signatures for every craft (near-free)

`brand_name(place, good)` and `CraftGuild.signature` already exist and are set
for exactly the goods that clear `SIGNATURE_TRADITION_YEARS` +
`SIGNATURE_QUALITY_FLOOR`. Nothing lowers the bar for cloth, and nothing in
the market view ever shows the name.

- Serve `signature` on `HubGoodDetail` and on the guild brief.
- The City Market and the goods tooltip read *"Ypres broadcloth (fine)"* in
  place of *"Woolen Cloth 0.94"* wherever a signature exists, and the plain
  name where it does not — quiet when ordinary.
- No sim change at all. This is the cheapest item in the plan and the one a
  player notices first.

### S5 · Transit demand — the entrepôt (dosed at zero)

`base_need` is population × budget share × cadence × `foreign_lux`. Every term
is about RESIDENTS. Nothing anywhere says "a merchant came here because this
is where the pepper is" — yet Delos, Puteoli and Palmyra produced almost
nothing and were the busiest markets in the world.

- `transit_need_mult(h, g)` reads the hub's own recent throughput of `g`
  (`supply_accum` / `trade_last`, both already tracked and decayed) against
  its resident need, and returns a multiplier bounded by
  `TRANSIT_DEMAND_CAP` (a hard fraction of resident need, so the feedback
  loop cannot run away).
- Applied to the MARKET-FACING `needs[h][g]` at the wiring site in `mod.rs`,
  beside `local_satiety_mult` and `foreign_prestige_mult_e` — **never inside
  `base_need`, never to `needs_struct`.** Merchants passing through are not
  mouths; if transit demand reaches the structural ration then `lack_basic`,
  starvation and crisis relief all start lying. That is exactly the bug the
  S7 household-monetization dose hit and had to revert.
- `TRANSIT_DEMAND_DOSE = 0.0` ships as a true no-op.
- Gates: `transit_demand_is_a_noop_at_zero`,
  `an_entrepot_wants_more_than_its_residents_do`,
  `transit_demand_never_touches_the_structural_ration`.
- **Do not dose it in the same session as S1.** Three demand terms
  (`LOCAL_SATIETY`, `FOREIGN_PRESTIGE`, this) now multiply the same
  expression and all three are at zero. Walk ONE at a time with the others
  pinned — `ACTORS_AND_CARRIAGE_PLAN` §5.2's lesson.

### S6 · War actually interferes with trade

`INSTITUTIONS_BRAINSTORM` §0's finding: `dispatch` never reads `self.wars`.
Two cities at war trade at full volume every day. The blockade only scales
`export_earn`.

The mechanism is built — `BLOCKADE_STAGING_DOSE` diverts a belligerent-to-
belligerent lane through a neutral `staging_hop`. Walk it `0.0 → 0.3 → 0.6`.

- It ROUTES, never refuses. A bare refusal at this shape of dose has twice
  collapsed the inheritance gate's world (N1c, N2). Keep it a routing rule.
- Gate: `econ_measure_war_frequency` is NOT re-run here (300-year,
  `#[ignore]`d, outside a session's budget) — named as queue item Q4 rather
  than silently assumed clean.

### S7 · Wire the orphan raws — more manufactured goods

Five shipped raws have **zero downstream consumers**: `clay`, `coal`, `alum`,
`hemp`, `pitch`. They are placed, mined, shipped and never used. Wiring those
is worth more than inventing new goods.

New `mg(...)` entries in `goods_spec.rs`'s recipe library:

| Good | Recipe | Why |
|---|---|---|
| `ceramics` | clay 1.0 + timber 0.2 | clay's first consumer. Roman *terra sigillata* — a craft that MIGRATED between provinces, the perfect showcase for S3 + S4. |
| `glassware` | bay_salt 0.6 + timber 0.5 | the Murano case the design docs already name |
| `fixed_dye` | alum 0.3 + dyes 1.0 | the mordant trade; `DEPOSITS_AND_MINING_PLAN` explicitly deferred it |
| `sailcloth` | hemp 1.0 | the yards plan needs it |
| `cordage` | hemp 0.8 + pitch 0.1 | ditto |
| `armour` | iron 1.0 + coal 0.4 | coal's first consumer |
| `garum` | herring 1.0 + bay_salt 0.4 | the most distinctive Roman manufactured good |
| `parchment` | hides 1.0 + alum 0.1 | feeds the existing `books` chain |

Rules this slice must hold:
- Goods are **appended**, never reordered — a good's index is a fixed
  position in `TileData.goods` forever (rule 7).
- Every one is `Distribution::Manufactured`, so it has no belt and is
  filtered out of the province list by `strip_manufactured_from_province_goods`
  (rule 33). Verify, don't assume.
- Gate: `cargo test --lib goods_ -- --nocapture` and READ the per-good table
  (rule 26), not just pass/fail. A recipe whose input never reaches a city
  makes a good that is permanently absent.
- **Cloth GRADES (broadcloth / kersey / says) are NOT in this slice** — they
  are queue item Q5. S4's signatures give cloth its origin variety at zero
  index cost first; promote to real graded goods only for the two or three
  cloths where the grades were genuinely different commodities.

---

## 4. Backend slices — the queries the atlas needs

### S8 · `campaign_house_atlas(house_idx)`

A pure derived read, no new persisted state, in `read_houses.rs`:

```
HouseAtlas {
  partners: Vec<AtlasPartner>,   // hub, x, y, volume_in, volume_out, goods[]
  goods:    Vec<AtlasGood>,      // good, bought_at[], sold_at[], spread, spread_trend
  holdings: Vec<AtlasHolding>,   // seat | office | bailo | estate | province, x, y
  seasons:  [f32; 12],           // when this house's lanes actually move
}
```

Sources that already exist: `House.trade_at` (per-hub weight),
`InTransit` (live cargo, with `via`/`hops`/`origin_km`), `House.offices`,
`prov_holder_house`, `season_slices`/`base_days_season`.

### S9 · `campaign_guild_atlas(guild_idx)`

Same shape, different question — a guild has a craft and a reach, not
partners:

```
GuildAtlas {
  inputs:  Vec<AtlasPartner>,   // where the raws come from
  outputs: Vec<AtlasPartner>,   // where the finished good goes
  reach:   Vec<u32>,            // hubs buying this guild's marked good
  signature: Option<String>,
  tradition_by_year: Vec<f32>,
}
```

Both are read-only and touch no tile and no sim state, so neither can move a
gate. State that plainly in the commit rather than running the suite for it.

---

## 5. Frontend — the window split

### S10 · Four windows, not two

`HousesPanel.tsx` is **1,455 lines** carrying a list, tier grouping, a feuds
board, a compare launcher and an 11-subtab dossier. `GuildsPanel.tsx` is
**125 lines** — a sortable table. That asymmetry is the problem.

| Window | Holds | From |
|---|---|---|
| 🏛 **Houses** | browse / rank / filter only | HousesPanel's list half |
| 📜 **House Dossier** | one house, big, floating, the tabs | HousesPanel's detail half |
| ⚔ **Feuds & Alliances** | world relations board | HousesPanel's feuds TAB (a feud belongs to two houses, not one — it was never a house's tab) |
| 🔨 **Crafts & Guilds** | craft guilds by city and good | GuildsPanel, expanded |

**Merchant Companies** (`House { is_guild }`) stop being a tab and become a
FILTER CHIP in Houses. They are firms, not a different kind of thing — and
keeping them in a tab beside "Guilds" is what made the naming collision
visible to users in the first place.

### S11 · The House Atlas and the Craft Atlas

A trade-flow window with real on-map labelling, in the lineage of
`FlowsView` / `drawGoodFlows`.

**Three panes plus the map.**
- **Ledger** — partner cities, each with goods, an in/out two-tone bar.
  Sorted by what is UNUSUAL (imbalance x volume), never by size: the biggest
  balanced staple always tops a volume sort and is never the interesting row.
  `FlowsView` already adopted this discipline; match it.
- **Goods** — the portfolio: bought where, sold where, the SPREAD, and
  whether the spread is widening or closing.
- **Seasons** — a month band showing when this house's lanes actually move.
  `season_slices` is live and nobody can see it.

**On the map:** thickness = volume · colour = the dominant good's own
`GOOD_DEFS` hue · arrows = NET direction (outbound and inbound drawn
differently) · dashed-vs-solid = the existing `mediumRuns` medium split ·
teal ring = break of bulk (exists) · a `drawGoodIcon` medallion at the lane
midpoint · a label at the far end (*"Pepper → Ostia · 340/yr"*) drawn through
`drawLabel` so it obeys the map-label registry (§8.11 — never set `ctx.font`
for a place name).

Toggles: outbound / inbound / both · one good or all · offices & bailos as
distinct markers · **rivals' lanes ghosted behind yours**, which is how a
feud becomes something you SEE.

The Craft Atlas is the same window asking about a craft: where the inputs
come from, where the finished good goes, and the signature's REACH as a soft
tint over every city buying the marked good.

**The rule that governs every lane here:** it goes through `laneBetween` →
cache → `compute_coarse_route`, falls back to dashed-direct, and is honest
about which it is. Never a straight slash presented as a road; never a
silently dropped lane the panel is simultaneously listing (rule 35). This is
a sixth caller of an existing mechanism, not a new system.

---

## 6. Frontend — the Houses redesign

The organising idea: **stop showing state, start showing change.** Every
figure in the current panel is a level, and almost all of them have a series
behind them already (`price_hist`, `vol_hist`, `world_series`, `House.line`,
`goal_history`, `crisis_history`).

### S12a — the three bands (in scope this session)

- **Top · the world of houses as one picture.** A **bump chart**: the top ~12
  houses' wealth RANK over the last 50 years, one line each in its own
  heraldic colour. Lines cross; lines end. A line climbing three ranks in a
  decade is a story with no words in it, and a line that stops is a house
  that died. Beside it four QUIET gauges (standing · founded · died · top-10%
  share), each a sparkline rather than a number.
- **Middle · the list as self-ranking strips.** Arms · name · tier · a
  30-year wealth sparkline · one phrase for what it is DOING ("cornering the
  pepper trade", "at feud with the Aurelii", "heirless"). One line for a
  tier-4 house, three for a tier-1. Sort chips that are QUESTIONS, not
  fields: Rising · Falling · Richest · Oldest · At war · In crisis · Heirless.
- **Bottom · the pulse.** A thin always-on ticker of the last handful of
  house events world-wide, colour-coded by kind. Not the chronicle — a
  heartbeat, so the window feels alive while time advances.

### S12b — the Dossier plate (in scope if time allows)

Chronicle-first stays. The header becomes a **plate**: the portrait larger
with arms at the shoulder, and under it **the life of the house as one
horizontal timeline** — every head a segment, milestones as marks above,
feuds as brackets below, the wealth curve running through as a filled area.
Scrub it and the tabs below re-read to that year.

### S12c — one graph per tab (QUEUED, Q8)

Accountant waterfall · Kin as a real tree · Lineage as a branching diagram ·
Standing as a radar against the tier-1 median · Feuds as a temperature line
with stage transitions. Each is small; together they are more than a session.

### Three principles

1. **A number alone is never satisfying; a number with a history is.**
2. **Quiet when ordinary.** Colour is a currency and the current panel spends
   it on everything. A tier-4 house that is fine gets one line and no colour.
3. **Everything is a doorway.** Feud row → both dossiers. Supplier row →
   the house. Guild → city → province. Most of these are dead ends today, and
   following a thread is most of what makes a world feel alive.

---

## 7. Build order — and where to stop

```
S4  signatures                      (no sim change — do it first, it is free)
S7  orphan raws + manufactured      (goods_ gate; independent of everything)
S2  annona class split              (inert at N1B = 0)
S1  dose ownerless loss             ← FIRST REAL DOSE WALK
S3  unfreeze guild roster           ← SECOND DOSE (per-city cap 1 → 3)
S6  dose the routed blockade        ← THIRD DOSE
S5  transit demand, at zero         (no-op; the walk is Q2, not today)
S8  house atlas query
S9  guild atlas query
S10 window split                    ── ■ STOP HERE IF SHORT ──
S12a Houses three bands             ✓ shipped (bump chart + gauges + pulse)
S11 the two atlases + map labelling
S12b Dossier plate
```

Everything above the marker is coherent on its own: the economy is fairer,
guilds are alive, there are more goods, and the panels are separated. The
atlases and the redesign are the elastic tail.

**Three doses in one session is the ceiling**, and each owes a full gate run
per STEP. If S1 needs three steps and S3 needs two, S6 does not happen today
— say so in the commit rather than dosing it unverified.

---

## 8. The queue — what this plan does not reach

Each item names what it waits for and the gate it will need.

1. **Q1 · The `EraProfile` axis.** Classical vs Medieval as a
   world-generation setting (firm form, guild form, credit instruments, bulk
   carriage, exclusion vocabulary). Waits on: S2/S3 shipping era-neutral, so
   the profile has two real behaviours to switch between. Gate: both profiles
   pass `econ_` independently.
2. **Q2 · Dose `TRANSIT_DEMAND_DOSE` above zero.** Waits on: S5 shipped
   inert, and on `LOCAL_SATIETY`/`FOREIGN_PRESTIGE` being walked first or
   confirmed pinned — all three multiply one expression. Gate:
   `econ_expenditure_shares_resemble_a_household` per step.
3. **Q3 · Give the residual an OWNER.** The merchants' guild as residual
   carrier: capital-clamped, vessel-slot-consuming, barrable. Closes all four
   asymmetries at once and makes `house_barred` a real weapon instead of a
   0.1% mechanism. Waits on: S1's dose walk landing, so the loss asymmetry is
   already closed and this only has to close the other three. Gate: the
   dense-world staging gates, which are what caught N1's 30-day collapse.
4. **Q4 · Re-run `econ_measure_war_frequency`** after S6's blockade dose.
   300-year `#[ignore]`d diagnostic, outside a session's budget. Baseline to
   beat: 6.0 wars/century pre-3.4a-c, ~45/century after.
5. **Q5 · Cloth grades as real goods** (broadcloth / kersey / says). Waits
   on: S4's signatures being live, so the cheap origin-variety answer is
   measured before spending permanent good indices on the expensive one.
6. **Q6 · Untie the quality ceiling.** Measured: the finest maker is the
   biggest city **0 times in 14 goods** — everyone converges on 0.960, so
   there is no finest anything, only a tie. Waits on: S3, which gives
   tradition and secrecy a live population to differentiate.
7. **Q7 · Guild powers** — entry restriction, price floor, foreigner
   exclusion (medieval); compulsory hereditary corpus paid in *immunitas*
   (Roman). Waits on: Q1, because these are the two profiles' divergent
   answers to the same question and building one without the axis bakes in an
   era.
8. **Q8 · S12c** — one graph per dossier tab.
9. **Q9 · Goals bias decisions.** Phase 3.1 shipped goals as read-only
   tracking; nothing in `decide_fleets`/`update_feuds`/
   `update_guilds_and_offices` reads a house's active goal. Waits on: a
   `GOAL_BIAS_DOSE` walk of its own — it moves wealth and needs `econ_` per
   step, which is a session on its own.
10. **Q10 · Alliances and joint ventures.** The missing positive counterpart
    to the fully-built `Feud`. Waits on: nothing technical; it is simply
    larger than the tail of this session.
11. **Q11 · Contesting a house-held province.** Rule 24's own named gap — a
    house's territory is unassailable except by its own dissolution. Waits
    on: territorial war-goal machinery.
12. **Q12 · State monopolies and customs frontiers** (*portoria*, the Red Sea
    *tetarte*). Waits on: Q1.
13. **Q13 · Information decay** — a house trades on the price it BELIEVES.
    `MERCHANT_VESSELS_AND_INFORMATION_PLAN` stage 4; the most plausible
    remaining fix for the price/distance gradient, and the prerequisite for
    the staple right.
14. **Q14 · Fix `dispatch`'s room/deficit reopening before S1 can be dosed at
    all.** `N1B_OWNERLESS_LOSS_RATE`'s own doc comment records the finding
    (pre-dating this plan, re-confirmed rather than re-run this session): a
    lost ownerless shipment does not reduce recorded trade, it reopens the
    buyer's deficit and invites MORE dispatch, so `dense_world`'s volume rose
    6.4× at a dose of 0.01 instead of falling. S2 (this session, shipped)
    protects the metropolitan lanes but cannot touch this — it fires on every
    non-metropolitan buyer regardless. The room/deficit calculation
    (`max_stock`/`room` in `production.rs`) needs to account for cargo already
    lost this cycle (or an equivalent brake) before S1 is safe to dose at any
    rate. Gate: `n1_bind_stays_healthy_on_a_realistically_dense_world`'s own
    `dense_world` fixture, re-dosed at the same 0.01 token rate used to find
    this, must show volume falling rather than rising before raising it
    further.
15. **Q15 · Give the standing gates a manufactured-goods fixture.**
    `reference_world`/`reference_world_large`/`dense_world`/`simulate_decades_
    reports_dynamics` all build goods through the plain `good()` helper, whose
    `inputs` is always empty — every craft-guild mechanism (founding,
    dissolution, quality/tradition, secrecy, signatures) is structurally a
    no-op on the entire standing `tick::tests`/`econ_` suite, discovered while
    trying to dose `GUILD_MAX_PER_CITY`. A `manufacturing_world()` fixture (a
    couple of raws + one manufactured good with real recipe `inputs`, run
    through `advance` for decades) would let a future session actually dose
    guild-related constants — and every OTHER manufactured-goods behaviour
    this codebase has ever shipped dosed-from-zero — against real evidence
    instead of shipping a plan's target value unvalidated, as S3 had to here.
    Waits on: nothing technical: it's a fixture-building session.
16. **Q16 · Give `the_relay_carries_long_lanes_in_stages_on_a_realistically_
    dense_world`'s "loose" fixture a war-free variant before re-attempting
    S6.** The dose walk to 0.3 failed that test's own "the relay is provably
    inert with the range caps off" assertion (`diag_relay_staged` read 2, not
    0) because `BLOCKADE_STAGING_DOSE` triggers the same shared `staging_hop`
    relay via `war_with`, independent of the N1/N1c range caps the fixture
    disables — a trigger that test was never built to account for. It ALSO
    failed `simulate_decades_reports_dynamics`'s bounded-wealth assertion (a
    house past the limited-liability floor), and the two failures were not
    disentangled before reverting: it is not yet known whether the wealth
    failure is a genuine economic effect of the blockade or a downstream
    consequence of the same test-assumption gap. Waits on: updating that
    fixture to either disable war or accept a nonzero staged count when one
    is live, so the wealth-bound failure can be isolated and re-measured on
    its own. Gate: both tests, re-dosed at 0.3, one change at a time.
17. **Q17 · Visually verify S10 (and now S12a) in a real browser.** The
    four-window split (`HousesPanel.tsx`/`HouseDossier.tsx`/
    `FeudsAlliancesPanel.tsx`/`GuildsPanel.tsx`) and the bump-chart top band
    + pulse ticker added on top of it were both built and verified by
    `tsc`/`vite build` alone — this session's environment has no display to
    launch the Tauri app in. Type-correctness and a clean bundle are not the
    same claim as "the four windows open, position, and read correctly
    together" (§8.11-adjacent — layout, z-index stacking against the other
    ~30 windows, the new filter chip's interaction, the Feuds button) OR "the
    bump chart's SVG scales sensibly at 300px wide with 1 house vs. 12, the
    gauges don't overflow their cells, the pulse ticker's horizontal scroll
    doesn't fight the panel's own drag handle". Waits on: a session with a
    browser/dev server. Gate: `npm run tauri dev`, open Houses, toggle the
    Companies chip, open a house dossier from a card, open Feuds from both
    the new button and the Society menu, click a bump-chart line and confirm
    it opens that house, confirm nothing regressed for an existing player
    save.
18. **Q18 · Give `founded`/`fallen` a real per-year series.** S12a's two
    "plain total" gauges (`campaign_get_inequality`'s `founded_total`/
    `defunct_houses`) have no yearly series to sparkline because neither is
    sampled per year anywhere in the sim — only a running cumulative count.
    A `founded_by_year`/`died_by_year` pair (derivable from each house's own
    founding tick and, for a dead house, the tick it was marked `defunct` —
    neither currently retained once a house's `wealth_history` stops being
    sampled) would let both gauges become real sparklines like their two
    siblings. Waits on: nothing technical, just deciding where that small
    piece of state belongs (`InequalitySnapshot.series` is the natural home,
    since `InequalityPoint` already carries `active`/`defunct` counts per
    year — it would need `founded_this_year`/`died_this_year` fields added
    there instead of only cumulative `active`). Gate: `cargo check` alone —
    it is a pure read/derivation, not a dose.

---

## 9. Risk register

- **S1 + S3 + S6 in one session is three doses against one fragile
  instrument.** `econ_inheritance_rules_fragment_differently` has flipped
  inside its own noise band five times and takes ~6 minutes. If two doses
  disagree, STOP and record the pair — a disagreement is a finding (§2.4),
  not something to average out.
- **S3 can move wealth.** Guild count feeds civic stability and the quality
  ceiling. Treat `GUILD_MAX_PER_CITY` as a dose, not a constant.
- **S5's cap is load-bearing.** Transit demand raises price → raises the
  arbitrage gap → attracts transit → raises demand. `TRANSIT_DEMAND_CAP` is
  the only thing between that loop and a runaway, which is why the dose ships
  at zero even though the mechanism is inert without it.
- **S7 adds goods, permanently.** Indices are fixed positions in
  `TileData.goods` (rule 7). An id shipped wrong cannot be renumbered later.
- **S12's bump chart needs a series that may not exist at the right
  granularity.** Check `world_series` and per-house history depth BEFORE
  building the chart; if the data is yearly and capped at ~400 rows, the
  chart's window is bounded by that, and it should say so rather than
  interpolating a history it does not have.

---

## 10. Keeping the record true

Per §2.6 and §2.7, in the same commits as the work:

- **`docs/SCOREBOARD.md`** — a dated row for every measured number that moves:
  the ownerless carrier share before/after S1, guild count before/after S3,
  wars/century if S6 is dosed. Append; never edit an old row.
- **`CLAUDE.md`** — §5 gains the annona split and the guild lifecycle, §7
  gains the four windows and the two atlases, §9 gains this plan's status
  line, §10 gains any new rule this work earns.
- **Negative results are deliverables.** A dose that is walked and reverted
  gets its table written down here, in §3 beside its own slice. An
  undocumented reverted attempt will simply be attempted again.
