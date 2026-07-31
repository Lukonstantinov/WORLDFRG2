# WorldForge 2 — Scoreboard

**The project in twelve numbers.** 89k lines across climatology, economics,
rendering and UI is more than anyone can hold as code. It is easy to hold as a
table of measurements. That is what this file is for.

Append a row every session that moves a number. Never edit an old row — a
scoreboard whose history is rewritten cannot show a regression.

---

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4d: sack and purge — the last step, `CITY_PROVINCE_WAR_PLAN.md` COMPLETE)

**What shipped.** The plan's own highest-risk item, deliberately built last:
`apply_war_defeat_consequences` (`tick/war.rs`), fired from `resolve_war` on any
decisive-enough defeat (`score_abs >= WAR_PRICE_TRIBUTE`, 40 — a marginal
reparations-only win does not cascade into breaking houses). Two paths, both
funnelling into the same `strip_holdings_at` + `house_is_ruined` check so
neither invents parallel machinery:

- **Enemy sack.** Every live non-guild house resident at the losing city
  (`house.hub == lose`) risks losing its own estates THERE (up to
  `WAR_SACK_MAX_ESTATES`=2, ownership passing to the city — `owner_house = -1`,
  the same "confiscated" convention the resale market already uses), offices/
  bailos/influence there, and any warehouse stock depot there — a per-house
  roll (`WAR_SACK_CHANCE`=0.5), not a guarantee, since not every resident
  family is equally exposed to a single sacking.
- **Internal purge.** The city turns on whichever house actually financed the
  losing war: the house-driven war's own `backer_house` (§3.4c) if this was
  one, else the losing city's own ruling house (`council_house`/
  `captor_house`) for an ordinary rival-council war — guaranteed once
  triggered (a targeted political act, not a raid), stripped the same way
  (up to `WAR_PURGE_MAX_ESTATES`=3) PLUS a wealth confiscation
  (`WAR_PURGE_CONFISCATE_FRAC`=0.25) straight into the city's own treasury and
  a real prestige/power cost (`WAR_PURGE_POWER_LOSS`=0.15).

Either path may cascade to full dissolution through the EXISTING
`dissolve_house` — no new cascade logic. `house_is_ruined` is a NEW check
distinct from the ordinary insolvency test (`update_solvency`, which reads
wealth alone): a war can strip a house's assets while it's still technically
solvent for a while longer, and that house is ruined in every way that
matters (no wealth AND no estates AND no offices anywhere) — the honest
trigger for a war-driven collapse.

**Gate results:** `cargo check` clean · `npx tsc --noEmit` clean (no frontend
surface for this step — sack/purge journal entries already render through the
existing chronicle) · `economic_war_levies_houses_and_resolves` and
`every_war_terminates_within_the_round_cap` both pass ·
`simulate_decades_reports_dynamics` hard-passes · `econ_` 4/4 non-ignored pass
— **first attempt, no RNG-divergence regression this time** (unlike 3.4a-c's
and 3.4e's own tuning rounds), because the severity gate keeps this path
comparatively rare. `econ_fidelity_scorecard`'s wars/century held at 45.00
(3.4e's own final value), consistent with sack/purge being a consequence of a
war's END, not a new trigger on how often one starts.

**`CITY_PROVINCE_WAR_PLAN.md` is now fully built end to end** — every item in
its own §7 order (1.2/1.3 panel · 2.1–2.5 provinces · 3.1–3.3 politics ·
3.4f/3.4a-c/3.4e/3.4d war) shipped and gated across this session. What remains
is explicitly out of scope by the plan's own §6 ("deliberately not built"):
territorial empires above the city-state, sieges/army movement, a rival house
finishing an enemy under cover of war, land state persisted back to tiles, a
per-cell quality field, the unexploited-opportunity view, and leagues/
treaties/diplomacy (FIX_PLAN B4) — plus 3.4e's own voluntary-war-financing
gap (lend to the chest, goods at a war premium) noted in its own entry above.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4e: war ledger, damage, blockade, boom)

**What shipped.** The four remaining §2 requirements from the plan, all reusing
existing machinery rather than inventing new fields:

- **War damage** (`war_damage_pass`, `tick/war.rs`): each belligerent's own
  estates/manufactories can take war damage yearly (`WAR_DAMAGE_CHANCE`=0.15 —
  see the tuning note below), writing straight into the EXISTING `TickHub.damage`
  field — the same field a natural disaster uses. No new repair machinery
  either: `estate_condition_pass` already repairs any nonzero `damage` whatever
  its cause, funded by the owning house or the parent city's treasury exactly
  as it already does for disasters. A house-owned estate's loss (in wealth
  terms, via `estate_market_value`) is booked to that house's own Accountant
  ledger.
- **A real, persistent blockade.** The pre-existing `trade_wealth *= 0.8` line
  was COSMETIC ONLY — `update_houses` recomputes `trade_wealth` fresh from
  `export_earn`/`import_spend` every single day, so that multiply was silently
  overwritten before a player could ever see its effect past the tick it ran on.
  `export_earn` — the term that actually drives `trade_wealth` — now shrinks to
  `WAR_BLOCKADE_EXPORT_MULT`=0.55 each year at war, which persists (decaying at
  its own natural 3%/day rate) for the rest of the year between `update_wars`
  calls. The old line is kept for its immediate display value.
- **The neutral war boom.** A hub sharing a belligerent's trade component,
  itself at peace, gets its own `export_earn` nudged (`WAR_BOOM_EXPORT_FRAC`=
  0.12 proportional + `WAR_BOOM_EXPORT_FLAT`=5.0 flat floor) — exactly why a
  house wants to supply a war it isn't fighting (§2).
- **Ledger lines.** `LedgerAcc` gains `war_levy` (split OUT of the general
  `civic_tax` field, which used to silently combine the progressive wealth tax
  and war levies — a war's cost now reads as its own line, per "war must be
  legible as money") and `war_damage`. Both are now included in
  `HouseLedger.expense_total` (previously `civic_tax` wasn't even wired into the
  Accountant view's total at all — a real pre-existing gap, not something this
  session introduced) and rendered as their own ⚔-prefixed lines in
  `HousesPanel.tsx`'s Accountant tab.

**Not built, and why:** the plan's Houses row also describes VOLUNTARY war
financing (lend to the chest, supply goods at a war premium) and "two houses
backing opposite sides is a new feud cause." Neither is required by 3.4e's own
step text ("Accountant lines for every war cost and gain; manufactory and
estate damage through the existing `damage` field; blockade on belligerent
routes; the neutral war boom") — only the FORCED levy exists today, which is
what `war_levy` reports. Voluntary contracts are real future work, not silently
folded in here.

**A second RNG-divergence round, same shape as 3.4a-c's.** Shipped first with
`WAR_DAMAGE_CHANCE`=0.35; `econ_inheritance_rules_fragment_differently` failed
again ("partible must leave the average house poorer than primogeniture
(172949 vs 160729)") — the SAME sensitivity 3.4a-c's own tuning already found:
two 60-year sub-simulations sharing a seed but diverging in house/estate count
from year one, so any new per-war-year `hash01` draw in a shared code path
shifts which values each run consumes downstream. Lowered to 0.15 (still a
real, recurring cost — just not rolled every single war-year) and the gate
passed again. Left here as the explicit, named reason for that constant's
value, so a future session doesn't raise it back toward 0.35 without knowing
why it was lowered.

**Gate results:** `cargo check` clean · `npx tsc --noEmit` clean ·
`economic_war_levies_houses_and_resolves` and
`every_war_terminates_within_the_round_cap` both pass ·
`simulate_decades_reports_dynamics` hard-passes (wealth ∈ [-4.6, 757383.0],
bounded/finite) · `econ_` 4/4 non-ignored pass (after the `WAR_DAMAGE_CHANCE`
fix above). `econ_fidelity_scorecard`'s wars/century moved 45.0 → **41.67**
(the stronger blockade/damage likely ending wars a little sooner on average) —
still a real, frequent feature of city life, not chased further per the
3.4a-c entry's own open pointer.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4a-c: war score, terms priced in score, casus belli)

**What shipped.** `sim/campaign/tick/war.rs` gets a real score-and-round engine on
top of DLC 3.5's declare/wage/resolve skeleton:

- **3.4a · score + quarterly rounds.** `War` gains `score` (−100..100, bidirectional),
  `round`, `peak_effort_a/b`. Every year now catches up every quarterly round due
  since `start_tick` (tick-driven, so a back-dated war still resolves correctly —
  the same trick the crisis engine uses). Each round rolls a battle/raid/blockade
  outcome biased by relative war-chest+treasury strength. Termination checks, in
  order: decisive score (±100) → the three exhaustion paths (force broken, treasury
  &credit spent, war weariness) → backers-withdraw (house wars only) → the round cap
  (`WAR_ROUND_CAP`=12 quarters = 3 years) as the guarantee of last resort, mirroring
  rule 22's discipline for the crisis engine. New test
  `every_war_terminates_within_the_round_cap` asserts this the same way
  `every_crisis_terminates` does.
- **3.4b · terms priced in score.** `apply_war_goal` is now score-gated at §1.4's
  table (reparations 10 · trade rights 25 · tribute 40 · a province 55 · annexation
  90) — a new `WAR_GOAL_PROVINCE` goal reassigns one ordinary (non-house-held, rule
  24) province's `prov_holder` to the victor. A win short of its declared goal's
  price downgrades to the richest goal the final score actually affords; it never
  upgrades on overperformance.
- **3.4c · casus belli expanded.** A WARMONGER RULER (`head_character_factor` axis 0
  on the council head) biases `WAR_DECLARE_CHANCE`. A HOUSE-DRIVEN WAR: the winner
  of a vendetta-stage feud flare, if it holds its own city's council or captor seat,
  may drag that whole city into a full state war on the loser's city instead of the
  ordinary property damage — `declare_house_war`, gated on differing cities, neither
  already at war, room under the war cap, and the new treasury/cooldown
  preconditions below — with itself auto-committed as `backer_house`, whose own
  insolvency is that war's backers-withdraw path.

**The tuning story — a real negative-result chain, not a single clean pass.**
Shipped first with `HOUSE_WAR_CHANCE`=0.20 and no other new preconditions:
`econ_fidelity_scorecard` read **65.0 wars/century**, an order of magnitude past
the §3.4f pre-3.4a-c baseline of 6.0/century measured for exactly this purpose.
Four successive attempts, each a real precondition from §3.4f's own list
("reach satisfied, a real grievance, sufficient treasury, council control"):

1. `HOUSE_WAR_CHANCE` 0.20 → 0.025 (8×): 65.0 → 56.7/century. Barely moved —
   proof the house-driven path was never the volume driver.
2. `WAR_MIN_TREASURY`=80 added to both declaration paths: 56.7 → 56.7/century.
   Zero effect — every candidate seat already cleared it.
3. `war_cooldown_until` (new `TickHub` field, 5-year "no fresh grievance" cooldown
   after ANY war, both belligerents): 56.7 → 50.0/century. Some effect.
4. `WAR_MIN_ROUNDS_TO_RESOLVE`=4 (a full year must pass before the three
   exhaustion paths — not a decisive score — may end a war): 50.0 → 50.0/century.
   Zero effect again.

Four preconditions on *declaring* a war, three of them near-inert, was the signal
that the volume was never about how often a war started — it was about how fast
one FINISHED and freed one of the two `MAX_ACTIVE_WARS` slots for the next. The
round-outcome magnitudes (24/16/8/11 per quarterly roll) let a lopsided pair reach
the decisive ±100 score in a handful of rounds. Halving them
(→ 12/8/4/5.5) was the one change that actually moved the number: **50.0 → 45.0
wars/century** — still well above the pre-3.4a-c baseline (a NEW war channel
plus real casus belli SHOULD raise it), but no longer an order of magnitude off,
and consistent with wars now being a real, if frequent, feature of city life
rather than the rare set-piece the old flat-10%-chance mechanism produced.

**The halving also fixed a real `econ_` regression, not just the frequency
finding.** At 50 wars/century, `econ_inheritance_rules_fragment_differently`
(Phase 0.4's own gate, unrelated to war on its face) FAILED outright: "partible
must leave the average house poorer than primogeniture (141324 vs 109769)" — war
had become frequent and fast enough that its own RNG divergence between the two
60-year sub-simulations (partible vs primogeniture diverge in house count almost
immediately, so they consume `hash01` draws differently from the first year) swamped
the structural signal the assertion depends on. The same magnitude halving that
brought the frequency down restored it: partible 150,940 < primogeniture 155,624,
passing again. **A second, unplanned benefit measured in the same run: top-10%
wealth share moved from 0.498 (out of its 0.60–0.90 band, unchanged from the
pre-3.4a-c 0.491) to 0.671 — back in band**, the first time since early in this
session's 3.1–3.3 work. Not the target of any of these changes — a side effect
of war now being a real, survivable-but-costly wealth event.

**Left as an open pointer, not chased further:** wars/century still reads well
above 6.0/century, and reasoning (not yet directly instrumented) points at
decisive-score resolution remaining the dominant path even after halving. A future
session wanting to bring it down further should look at raising
`WAR_SCORE_DECISIVE` or damping round magnitude again, informed by an actual
per-war termination-reason histogram — not another blind precondition on
declaration, which this session's four attempts already showed doesn't touch the
real lever. Per CLAUDE.md §2.4, this negative-result chain is the deliverable,
not a loose end to feel bad about.

**Full gate set, final state:** `cargo check` clean · `npx tsc --noEmit` clean ·
`econ_` 4/4 non-ignored passed (house wealth Gini 0.769, top-10% share 0.671 — both
in band; house dissolutions/century 66.67, printed-only, no band) ·
`simulate_decades_reports_dynamics` hard-passes (wealth ∈ [-4.6, 507320.7],
bounded/finite, turnover happens) · `economic_war_levies_houses_and_resolves` and
`every_war_terminates_within_the_round_cap` both pass. `WarBrief` (the Wars tab's
existing active-war list) gained `score`/`round`/`goal_label`, shown as a small
bidirectional meter in `MoneyFinancePanel.tsx` — the same "surface it the moment
it's built" discipline the crisis engine's round log follows.

**Not yet built:** 3.4e (accountant ledger lines, manufactory/estate damage,
blockade, the neutral war boom) and 3.4d (sack and purge — deliberately last, the
highest-risk item). See `docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the order.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.4f: war frequency measured)

**3.4f — measure BEFORE tuning, the precedent Phase 4.4 set for the foreign hand.**
`econ_measure_war_frequency` (`economy_validation.rs`, `#[ignore]`d, 150-year
reference world) measures the EXISTING DLC 3.5 war mechanism — the baseline
3.4a-e's score+preconditions redesign will be judged against, not a target in
itself.

```
wars started                              9   →  6.00 / century
wars resolved in the window               9
mean duration                          2.0 yr   (every war resolves at the
                                                  earliest eligible tick —
                                                  `years >= 2` — see below)
outcome mix     plunder 6   tribute 0   trade-rights 3   annex 0
causes          independence 6   rival councils 3
war-eligible cities (a council seat)     11
structurally isolated (own component)     0   (0.0%)
```

**The headline finding: two-thirds of "wars" are not the mechanism 3.4a-c is
about to redesign.** 6 of 9 are colony wars of independence
(`declare_independence_war`, its own gate in `colonies.rs`), not
`maybe_declare_war`'s rival-council/trade-dispute path — which fired only 3
times in 150 years (2.0/century) despite a flat 10% yearly roll
(`WAR_DECLARE_CHANCE`) whenever ≥2 eligible seats share a connectivity
component. Zero cities were structurally isolated on this fixture, so the low
rate is the trigger's own rarity (a rival pair + `hash01 < 0.10`, checked once a
year), not §5.8's "no reachable rival" cause — worth re-confirming against a
real generated world, since `reference_world()`'s 11 seats in one component is
a much denser graph than most generated worlds will have.

**Every resolved war ends at EXACTLY 2 years** — `update_wars` resolves the
instant `years >= 2` is first true, weighted only by cumulative treasury +
war-chest at that moment; there is no further escalation once eligible. This is
the mechanism 3.4a (a proper score + quarterly rounds, exhaustion paths) exists
to replace — a war that always resolves at its floor duration cannot show mean
duration moving at all, so "mean duration" as measured here is really "the
resolution floor," not a real distribution. That is itself useful context for
judging 3.4a's post-redesign number.

**A real bug was caught and fixed while building this diagnostic**: the first
draft computed a war's duration from the loop's 0-indexed year counter instead
of `s.tick / TICKS_PER_YEAR` taken AFTER that year's `advance()`, silently
undercounting every duration by exactly one year (reported 1.0 yr instead of
the true 2.0 yr). Caught by hand-checking `update_wars`' own `years >= 2` gate
against the printed number, not by a test — recorded here per §2.4's "a
diagnosis is a complete task" so the off-by-one doesn't reappear if this
diagnostic is ever rewritten from scratch.

Run originally at 300 years; killed and re-run at 150 after ~280s CPU with no
sign of finishing — war-driven house turnover keeps growing `s.houses`, and
several per-tick passes scan it, so cost per simulated year rises through a
long run (a `rust-performance` question for another session, not this one).
150 years already gives a clean per-century rate, matching the window
`econ_diagnose_outpost_founding` already uses.

Verified: `cargo check` clean, `simulate_decades_reports_dynamics`
byte-identical to the pre-3.4f baseline (the diagnostic is `#[ignore]`d and
touches no production path). No `econ_` re-run needed for the same reason — the
change is a new test function only.

**Not yet started:** 3.4a (war score + quarterly rounds), 3.4b (terms priced in
score), 3.4c (casus belli incl. warmonger ruler + house-driven war), 3.4e
(ledger/damage/blockade/boom), 3.4d (sack and purge — last, highest risk). See
`docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.3: state name/colour/borders)

**3.3 — states.** A state is not new sim state: `compute_states`
(`campaign_commands/province.rs`) is a pure derived read over what 3.2 and Phase 5
already carry — every province a tier 1-2 city holds the writ to (`prov_holder`,
excluding a house-held writ per rule 24 — that's the house's territory, not a
city's state), grouped by the world's own `province_raster` cells into one
`StateRegion` per city. Nothing is persisted; a rerun cannot desync from the sim
because nothing is stored to desync. Name is deterministically varied (bare city
name / "X Republic" / "Republic of X" / "Duchy of X" / "Free City of X" / paired
with the home province's people-name), hashed off the hub id so it's stable
without being hand-authored. Colour reuses `distinct_color`'s golden-angle hue
rotation but phase-shifted (+53°) and desaturated, so a state's tint is provably
distinct from a house's heraldic colour even where a hub id and a house id
happen to collide numerically — different index spaces. Rendered with the exact
"cell cloud" technique `compute_culture_regions`/`drawCultureRegions` already
uses for ethnic territories (`OverlayManager.drawStates`), gated behind a new
Toolbar toggle (🏰 States), refreshed on year boundaries like the caravan-corridor
overlay. A tier 3-4 or untiered town keeps self-administering its own province
exactly as before; it simply never forms a state.

This is where §3.2's own note said the "bit-identical to the dynamics test"
guarantee would end — city tier now decides what the MAP draws. It does NOT mean
the tick itself changed: `compute_states` reads `prov_holder`/`hub.tier` and
writes nothing back, so the dynamics run stays byte-identical to the pre-3.3
baseline (confirmed below), and no new `econ_` exposure exists because no new
tick state was added.

Verified against the full required gate set: `cargo check` clean (only
pre-existing unused-constant warnings), `npx tsc --noEmit` clean,
`simulate_decades_reports_dynamics` byte-identical year-by-year to the pre-3.3
baseline (richest/houses/banks/wars/crashes/towns all match exactly).

**Not yet started:** the whole abstract war system (3.4a–f), starting with 3.4f
(measure war frequency before tuning anything). See `docs/CITY_PROVINCE_WAR_PLAN.md`
§7 for the full order.

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md 3.1 + 3.2: the city leader and city tiers)

**3.1 — the office as a person.** `council_house`/`captor_house` already existed and
already compete for the seat (bribery/intimidation/capture in the existing
`update_government`); what was missing was surfacing WHO holds it. `CityLeader`
reads `kin[0]` of whichever office is stronger, reusing `character_phrase` and
`head_vice` — both already built for the House Dossier but never exposed outside
it. New `vice_label()` is the first thing that surfaces `head_vice` to the
frontend at all. Pure read/display addition, no tick mutation.

**3.2 — city tiers.** `TickHub` gains `tier`/`standing`, recomputed monthly by
`assign_city_tiers` — a direct mirror of `assign_house_tiers` (same percentile
cutoffs, same Tier-1 absolute floor, same hysteresis). Four axes: population,
trade wealth, treasury, territory administered (rural population under provinces
this city holds), and the ruling house's own standing. Query-side only at this
step — nothing downstream reads the new fields, so the guarantee holds exactly as
it did for house tiers. Four new tests (richest-city-ranks-highest, Tier-1-empty-
on-a-flat-world, hysteresis stability, an-estate-is-never-tiered) all pass.

Both steps verified against the full required gate set: `cargo check` clean,
`econ_` 4/4 non-ignored passed with numbers UNCHANGED from the pre-3.1/3.2
baseline, `simulate_decades_reports_dynamics` byte-identical, `npx tsc --noEmit`
clean (3.1 only touched the frontend; 3.2 has no frontend surface yet — city
tiers become visible once §3.3 turns them into state borders).

**Not yet started:** 3.3 (state name/colour/borders — where city tiers stop being
bit-identical) and the whole abstract war system (3.4a–f). See
`docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

---

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md: 1.3, 2.3, 2.4, 2.5 built)

**1.3 — panel polish.** Smoothed the Trade tab's 360→600px width snap into a CSS
transition; applied the house stability gauges' own "quiet when healthy" rule to
the two treasury displays (grey+small when positive, a loud warning colour only
when actually empty).

**2.3 — the survey plate's real terrain.** New query command
`get_province_terrain_crop(province_id, max_dim)` returns a cropped elevation/
land/biome grid over a province's bbox, read from the world's cached tiles —
replacing `ProvinceMiniMap`'s flat placeholder relief fill with real hypsometric
shading. River courses are NOT re-sent by the backend — the frontend already
holds the world's full river geometry (`worldStore.rivers`) and clips it to the
province's own raster mask itself, so the water plate now draws a real course
instead of a proportional scatter. Both fall back to the old placeholders when
absent (old world / still loading).

**2.4 — elevation-biased land use.** Reuses 2.3's terrain crop: the land-use
dither's placement is now a RANKED composite (elevation + noise) rather than pure
noise, so woodland/waste cluster uphill and arable/pasture on the flat — while the
province's overall shares stay exactly exact (ranking, not threshold-shifting, is
what preserves that). Tenure's dither is untouched by design.

**2.5 — goods exploitation (the workstream's own "substantial/risk" item).** New
frozen per-(province, good) belt score (`Province.good_belt`, world-side, an
unfiltered mean unlike the existing top-6 quality shortlist) snapshotted once at
campaign start. `potential`/`actual`/`exploitation`/`market_share` are PURE
DERIVED reads — no new production, no touched prices — computed fresh from
current land use, live hub+estate production, and the one piece of state that
does persist: `prov_good_depletion`, updated yearly with an estate-kind-aware
wear/heal rate (mine barely recovers, fishery recovers fast, vineyard doesn't
deplete at all — plantation also nudges `prov_soil` down, a real cross-link). The
yield constant is SELF-CALIBRATED per world (mirroring `need_scale`) so mean
exploitation reads ≈1.0 on day one regardless of world size, rather than a single
hand-picked constant that would silently read wrong on a differently-shaped
world. New test `province_goods_exploitation_tracks_pressure_and_depletes`
exercises the whole loop (calibration → sustained overexploitation → erosion →
easing → healing) end to end. Because the pass only ever writes
`prov_good_depletion`, it cannot move the `econ_` bands or the dynamics test by
construction — verified, not just argued: both are byte-identical with this
wired in. Exposed via `campaign_province_goods`; the Province Inspector's Land
tab now shows the live reading in place of the frozen quality/rank list the
moment a campaign is actually producing something (falls back to the frozen list
pre-campaign).

**Simplified / not built, flagged rather than hidden:** land-use category is a
small hardcoded name table over the 45 shipped goods (`good_land_kind`), not a
new `GoodSpec` schema field — an unrecognized/custom good defaults to
unconstrained rather than guessed. §5.5's "keep a good listed while produced
recently" caveat is simplified to "produced now OR depletion hasn't healed away"
— no separate last-produced-year is tracked. Vineyard's "raises grade instead"
positive half isn't tracked (only the "doesn't lose tonnage" half is). Estate
tier's own "footprint + ceiling + grade" mechanics are untouched by this pass.

Whole-lib gates run: `cargo check --lib` clean · `provinces::tests` unaffected ·
new exploitation test passes · `econ_` 4/4 non-ignored passed, numbers unchanged
from the pre-2.5 baseline · `simulate_decades_reports_dynamics` byte-identical ·
`npx tsc --noEmit` clean.

**Not yet started:** 2.5's own estate-tier depth, Workstream 3 (politics/war —
city leader, city tiers, state name/colour/borders, the whole abstract war
system). See `docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

---

## Current state — 2026-07-31 (CITY_PROVINCE_WAR_PLAN.md begun: Step 0 + 2.1 enclave fix + 2.2 sizing)

**Step 0 — the economy oracle could see geography for the first time.** The
60-year/30-city reference world's province layer was UNIFORM (5 identical
provinces, seats on a straight line), so the fidelity scorecard could measure
*levels* but never *dispersion* — it could not say why one province is rich and
its neighbour poor. Seeded five geographically distinct provinces (fertile river
lowland, wooded hills, arid steppe, temperate mix, marginal upland — varied
capacity, forest/arable/soil, seat position) matched to real hub clusters. Added
three printed (not asserted) diagnostics ahead of the mechanisms that will give
them real meaning: province land-pressure CV **1.406**, province output-share CV
**0.662** (both stand-ins for Workstream 2.5's exploitation/market-share ratios,
which don't exist yet), wars started/century **1.67** (the existing DLC 3.5 rival-
polis mechanism, ahead of §3.4's abstract state-war system). All existing `econ_`
bands held; `simulate_decades_reports_dynamics` stayed bit-identical (it seeds no
provinces).

**2.1 — the enclave fix, reversing a documented Phase 1 decision (§5.1).** Seed
rejection (`too_close`) tested only the CANDIDATE's own required separation, never
the incumbent seed's — a fertile valley (small separation) sitting inside a desert
or tundra region (large separation) passed a test the surrounding province would
have failed. Fixed to `max(sep_candidate, sep_incumbent)`. Added a post-snap pass
(`merge_enclaves`, run AFTER `snap_borders_to_features` per §5.3 — the snap itself
can create or heal an enclave) that folds any province bordering exactly one
neighbour into it, unless the province is its own island. New test
`no_enclosed_province_survives_unless_its_own_island` passes; all 8 pre-existing
`provinces::tests` (crest affinity, diagonal-river affinity, determinism, coverage)
still pass.

**2.2 — sizing, compressing the fertile↔hostile spread.** Measured baseline: the
old constants produced a ≈169× area ratio between max-hostile (ice cap) and
max-fertile land at the seed-separation level alone, before `VAST_MERGE_CAP_FRAC`
enlarges hostile blocks further — in the "roughly 100×" range the plan named.
Shrunk globally (`base_sep` multiplier 0.5 → 0.40) and compressed the spread from
both ends: the habitability ramp 1+1.6·hostile → 1+1.0·hostile, every
`koppen_spacing_mult` ceiling lowered (ice cap 3.0→2.0, tundra 2.2→1.7, desert
1.9→1.5, etc.), and the fertile floor raised (0.6→0.75). New `#[ignore]`d
diagnostic `province_size_distribution` measures the result on a synthetic zonal
world: **hostile/fertile mean-area ratio ≈33×** — a real, measured compression,
not just a paper one. Not a hard gate (no single "right" size exists; the
maintainer judges this visually in the app) — determinism and the existing
`provinces::tests` are the actual gate, and both hold.

Whole-lib gates run: `cargo check --lib` clean · `provinces::tests` 9/9 passed ·
`econ_` 4/4 non-ignored passed (incl. the new determinism check over the three new
scorecard fields) · `simulate_decades_reports_dynamics` bit-identical.

**Not yet started:** Workstream 1 (settlement panel rework), 2.3-2.5 (terrain crop,
organic land use, goods/exploitation), Workstream 3 (politics/war). See
`docs/CITY_PROVINCE_WAR_PLAN.md` §7 for the full order.

---

## Current state — 2026-07-30 (Phase 5 complete: provinces as house territory — the house series is DONE)

**Asked to "move on with phase 5 and 6" — neither exists in `HOUSE_MASTER_PLAN.md`.**
Phase 5 ("Provinces as house territory", the Stato da Mar case) lives in
`docs/proposals/HOUSE_INHERITANCE_AND_TERRITORY.md` Part D, whose own revised phase
list runs 0 through 5 — **5 is the last phase in the whole house series; there is no
Phase 6.** Built it: a house may hold a province's writ instead of a city
(`prov_holder_house`), with dues redirected to the house, unrest directed at the
house (prestige + wealth, not the city's mood), standing weighted 3× toward held
territory, and a narrow GRANT trigger (a Tier 1-2 house already dominating its seat
city may be granted its ungoverned hinterland, small yearly chance). A held
province is inherited for free (house-indexed, not head-indexed) and released only
when its holder dissolves. **Contesting a held province (war, a rival house) is
explicitly NOT built** — needs new territorial war-goal machinery, the single
largest remaining gap in the whole series.

**The grant trigger needed one real fix, caught by measurement, not review**: the
first cut required a bailo specifically at the province's own seat, and fired ZERO
times on the real economy-oracle world (a house rarely bailos its own home city —
a bailo is a foreign foothold). Relaxed to council/captor-house-or-bailo (the same
signal `assign_house_tiers` already sums), and the effect became real.

**A genuinely dramatic result — the first metric in this whole series to cross INTO
its band, not just move toward it**: diffed against the pre-Phase-5 commit on the
60-year/30-city economy-oracle world, **top-10% wealth share 0.497 → 0.651**, now
inside its 0.60–0.90 historical band for the first time since Phase 0.4 first pushed
it out of band (0.422) fixing turnover. **House wealth Gini 0.693 → 0.790** — stayed
in its 0.60–0.85 band, now nearer the ceiling (worth watching in a longer run). Also
moved: surviving houses 49 → 38, dissolutions/century 40.00 → 33.33, banks chartered
24 → 21, bank failures/century 28.33 → 25.00.

Also exposed to the frontend (no new command — the existing `ProvinceLand` query
gained one field, `holder_house: i32`); `ProvinceInspector.tsx`'s existing
writ/granary/works-funding text was updated to stay accurate for a house holder.

Whole-lib test suite: **224 passed, 0 failed** (was 219, +5, covering Part D's own
invariant #7, `province_authority_is_not_assumed_to_be_a_city`). The small
dynamics-test world stays byte-identical (it seeds no provinces). `cargo check`/
`npx tsc --noEmit` both clean.

**The house mechanism series — Phases 0 through 5 — is now complete as scoped.**
Two gaps remain across the whole series, both recorded rather than hidden: goals not
yet biasing decision weights (Phase 3.1), and a held province not yet contestable
(Phase 5). Everything else in the series' own tables is built.

---

## Current state — 2026-07-30 (Phase 4 complete: 4.4 foreign hand, 2.4 salience, 4.5 mavericks declined)

**Measured before building, exactly as §2.5 demanded.** A new 300-year diagnostic
(`econ_measure_foreign_hand_conjunction`, `#[ignore]`d) found the design's "foreign
hand" trigger — a rival's office/bailo in a posted kin's city, or the house leasing
in a city a rival controls, coinciding with that kin already reading disaffected —
firing **1229 times/century** (89,784 kin-months sampled; 27.66% show either channel
present, 4.11% of those also disaffected). Two orders of magnitude past "a handful a
century", so the mechanism was built: `sim/campaign/tick/foreign_hand.rs`, a small
bounded monthly loyalty decay (ceiling 0.015/month even at maximum leverage) plus an
occasional named disclosure. **The design's own required gate held**: diffed against
the pre-4.4 commit, house dissolutions/century moved 41.67 → 40.00 (down, not up) —
leverage colours outcomes, it does not manufacture them.

Also shipped in the same pass: **§2.4 crisis salience** (only Tier 1-2 crises reach
the world news feed; Tier 3-4 stay fully chronicled on the house's own record, just
quiet on the world stage). **§4.5 mavericks** was considered and explicitly
DECLINED: `roll_character`'s existing uniform draw already lands on a full ±2
extreme ~20% of the time per axis by construction, so a true "maverick" (a rare
escape from an otherwise-centred distribution) would mean changing the baseline
distribution every already-wired character knob, `head_vice`, crisis actions and
goal selection reads — a systemic change with no gate of its own.

Whole-lib test suite: **219 passed, 0 failed** (was 215, +4). `simulate_decades_
reports_dynamics` stays byte-identical. `cargo check` clean.

**Phase 4 (Consequences) is now complete as scoped: 4.1 through 4.4 all built, 4.5
addressed item-by-item (2 correctly deferred, 1 declined with a documented reason).
Phase 5 does not exist in `HOUSE_MASTER_PLAN.md` — there is no such section.**

---

## Current state — 2026-07-30 (Phase 4.1–4.3 · Consequences)

Asked to implement "all 3 phases" — read as the three concrete, buildable items in
Phase 4's table (4.4 is explicitly conditional on an unmeasured signal, 4.5 is
explicitly "Deferred" already). **4.1** Departure schism (new `schism.rs`): a house
above a simplified `tension` proxy (mean kin loyalty · reach · feuds · a passed-over
heir) monthly either Quarrels (common, chatter) or, if the disloyal kin is POSTED to
a real holding, Departs with it to found a new rival house (Rupture stays deferred,
per this file's own earlier call). **4.2** Bankruptcy aftermath: `dissolve_house` now
writes off any outstanding bank loan and names the bank on both ledgers (kin barred
from office was cut — would need new per-`TickHub` state for a detail the source
design itself calls small). **4.3** Plague as a lineage event: a struck house can
lose several kin at once or, rarely, be extinguished outright — independent of head
mortality by design, documented in `plague_house_toll`'s own doc.

**A genuinely good result, not just a bounded one**: 4.3 is the first change in this
whole series to move **top-10% wealth share** — out of band since Phase 0.4 fixed
turnover — TOWARD its band: **0.382 → 0.509** (still below 0.60–0.90, much nearer).
**House wealth Gini 0.607 → 0.698** (stayed in its 0.60–0.85 band, now more centred).
This is exactly the historically-documented mechanism (plague extinction removes
weaker houses, concentrating survivors' share) showing up as a measured number.
Also moved: bank failures/century 33.33 → 28.33, dissolutions/century 46.67 → 41.67,
banks chartered 25 → 23. Whole-lib test suite: **214 passed, 0 failed** (was 206,
+8). The small dynamics-test world stays byte-identical (its seeded houses have no
kin roster, so both new mechanisms read "nothing to act on"). `cargo check`/`npx tsc
--noEmit` both clean.

**Phase 4 is now complete as scoped: 4.1-4.3 built, 4.4 correctly left un-attempted
(gated on a signal nobody measured), 4.5 deferred by the plan's own design.**

---

## Current state — 2026-07-30 (Phase 3.2–3.6 · the crisis engine, real but cut down)

Asked to implement "the last step" — the whole rest of Phase 3 — in one pass. New
`sim/campaign/tick/crisis.rs` (~470 lines) consolidates FOUR source design docs
(`HOUSE_POWER_AND_POLITICS.md`, `HOUSE_SUCCESSION_CRISIS.md`,
`HOUSE_POWER_STRUGGLE_VIEW.md`, `HOUSE_FACTION_NAMING_AND_RECORD.md`) into: **3.2**
competence/vice (5 named vices derived from character+skill, Lavish wired to a real
wealth cost); **3.3** the crisis itself — `HouseCrisis` opens on discontent, runs a
FIXED 4 quarterly rounds, named factions drawn from the house's own heraldic
tincture palette (mirrors `CoatOfArms.tsx::houseColor` bit-for-bit); **3.4** the
undecided bloc folded into each round's delta + a 5-year survivor grace period;
**3.5** civic intervention (a severe deposition risks the seat council sequestering
a slice of the estate); **3.6** a capped permanent `CrisisRecord`, same discipline
as `goal_history`.

**Two cuts matter most, both documented in `crisis.rs`'s own module doc**: no
per-figure power-share ledger (`head_support`/`plot_support` are two abstract
aggregate numbers, not a sum of named shares) and no continuously-drifting `regard`
ladder (plot leadership reads each kin's existing static `Kin.loyalty` roll
instead). The Split/schism outcome is deliberately not built — consistent with this
file's own Part 3 already recommending deferring "Rupture" behind Departure.

**A real bug, caught by the existing suite, not by review**: the first cut of
deposition succession ignored the culture's `LineRule` entirely, and
`a_matrilineal_house_is_held_by_women` (a Phase 0.4 test) immediately failed — a
70-year run put a man at the head of an enatic house. Fixed by filtering every
crisis successor candidate through `heir_is_female`, the same guarantee
`succeed_house` already gives every ordinary succession.

Measured on the real 60-year/30-city economy-oracle world (diffed against the
pre-pass commit): **house wealth Gini 0.649 → 0.607** (stays in the 0.60–0.85 band,
nearer its floor), **top-10% share 0.409 → 0.382** (already below its 0.60–0.90 band
before this pass — an existing finding, moved further from band rather than into
it, worth watching), **surviving houses 49 → 44**, **banks chartered 23 → 25**,
**bank failures/century 36.67 → 33.33**, **house dissolutions/century unchanged at
46.67** (Dissolved is "very rare" by design). Whole-lib test suite **206 passed, 0
failed** (was 199, +7). `simulate_decades_reports_dynamics` stays bounded. `cargo
check`/`npx tsc --noEmit` both clean.

**Phase 3 (Politics) is now complete AS SCOPED** — 3.1's goals remain read-only
tracking (not wired to bias decision weights) and 3.2–3.6's crisis engine is real
but missing the power-share ledger and regard drift described above. Both gaps are
recorded, not hidden. The crisis→deposition rate over a long run is UNMEASURED,
same honest-gap pattern as 3.1's own goal achieve/fail rate.

---

## Current state — 2026-07-30 (Phase 3.1 · goals, built as STRUCTURE only)

Scoped Phase 3 down to **3.1 only** — the crisis engine (3.2–3.6: competence/vice,
factions, resolution rounds, contested succession, civic intervention, CrisisRecord)
is a bigger undertaking and was explicitly set aside, not attempted.

**3.1** gives every non-guild house `goals: Vec<Goal>` (1 slot, 2 for Tier 1) plus a
capped `goal_history`: 7 kinds (corner a trade good, seat a council, raise the Bailo
tier, charter a bank, reach a province by expedition, outlast a named rival, restore
peak wealth after a fall), chosen yearly biased by archetype + character axis, checked
yearly, chronicled achieved (milestone, permanent) vs. failed/abandoned (chatter,
prunable). `GOAL_REACH_PROVINCE` hooks the existing expedition-arrival pass rather
than adding a new success channel. A 🎯 Ambitions dossier tab shows active (progress
bar / deadline countdown) and past (✓/✗ list) goals.

**Same honest gap as always with a "structure first" cut: goals do not yet bias
anything.** Nothing in `decide_fleets`/`update_feuds`/`update_guilds_and_offices`/etc.
reads a house's active goal to weight its choices — the master plan's §4 closed loop
(goal → weighted decisions → outcome → new goal) is not built. This is pure tracking
against state the sim already computes, so it is provably inert: goals touch no
wealth, no decision, no probability. Verified BYTE-IDENTICAL on both the dynamics
test and the economy-oracle scorecard (goals literally cannot move a number yet).
78 `tick::` tests pass (was 72, +6 — one per representative goal kind plus the
Tier-1-gets-two-slots case). Full scoping note in `HOUSE_MASTER_PLAN.md`'s handoff
block, including the still-UNMEASURED 200-year achieve/fail-rate the design spec
actually cares about.

---

## Current state — 2026-07-30 (Phase 2.4/2.5 · Phase 2 now COMPLETE)

Asked to build the two items the previous entry deliberately deferred, with a single
check at the end instead of the usual per-change gate. **2.4** wires character into
one real decision per axis (fleet-buy threshold, feud heat, civic consumption rate,
office-open threshold), each bounded to exactly ±`CHARACTER_KNOB_CAP`=0.15 and a TRUE
1.0 no-op with no roster. **2.5** gives every hired (unposted) holding a monthly
wage+skim and a 1%/month poaching risk.

**A real bug surfaced doing it this way, exactly as flagged when 2.4/2.5 were
deferred**: the first cut of steward costs read an EMPTY kin roster as "everything is
hired" rather than "nothing is known", so an old save's houses would have been
silently CHEAPER to run than freshly-generated ones — a backward-compatibility
regression, not a cosmetic bug. It was caught by the test suite (a Phase 2.1 test,
`a_house_with_no_kin_is_bit_identical`, started failing) rather than by inspection,
fixed by gating both mechanics on a non-empty roster, and the test renamed to
`an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached` to describe what's
now actually guaranteed. Full account in `HOUSE_MASTER_PLAN.md`'s handoff block.

Measured effect: on the small 30-house/50-year dynamics-test world, BYTE-IDENTICAL
output (verified by diff against the pre-2.4/2.5 commit — that world's seeded houses
never succeed inside 50 years, so never gain a roster). On the real 60-year/30-city
economy-oracle world: **house wealth Gini 0.609 → 0.649**, **top-10% share 0.422 →
0.409**, **mean firm lifespan 36.8 → 39.9yr** — all moved, none left their historical
bands. 72 `tick::` tests pass (was 67, +5 net — 6 new, 1 retired/renamed).

**Phase 2 (People) is now fully complete: 2.1 through 2.6, all built and gated.**

---

## Current state — 2026-07-30 (Phase 1.3 + 2.1/2.2/2.3/2.6 · Phase 1 complete, Phase 2 half)

Phase 1 is now fully shipped: **1.3** adds `Expedition.dest_province`, a 🧭 Expeditions
dossier tab, and click-to-highlight on the province plate.

Phase 2 (People) is half built, on purpose. Built and gated: **2.1** the `Kin` roster
(`kin[0]` mirrors the head, 2–4 siblings per founding/succession, up to two posted to
current holdings) plus the widow regency (an agnatic line's one route to a female
head, `WIDOW_REGENCY_CHANCE`=8%); **2.2** holdings authorship (a family-run estate/
office tags its posted kin's name in the Summary tab, silent = hired); **2.3**
character as four culture-derived axes read into a phrase, wired to nothing; **2.6**
`kin_power_shares` (role × skill × loyalty, always sums to exactly 100). **2.4**
(character → real decisions) and **2.5** (stewards with skim/wage mechanics) were
**deliberately not attempted** — both move house wealth directly and need `econ_`
verification per knob as they're built, not a single check at the end. 67 `tick::`
tests pass (was 61, +6); dynamics and economy scorecards bit-identical — nothing new
here is read by any decision.

---

## Current state — 2026-07-30 (Phase 1.2/1.4 · figure + chronicle-first dossier)

Also read-only/query-side — no economy number moves. `HouseDetail`'s default tab is now
Chronicle (§2.3), showing the Phase 0.4 succession line inline before the year-grouped
event log. The dossier opens on a `cultureFigureSVG` portrait in the seat culture's kit
and the head's own sex, tier-registered (ceremonial/national/everyday). Three positive
events (§2.2) shipped as markers on `House`: finest hour (peak wealth, never chronicled),
golden age (a decade at Tier 1 with wealth rising), dynasty of merchants (three
consecutive heads who each grew the house, derived from Phase 0.4's `line`). 61 `tick::`
tests pass (was 58); dynamics and economy scorecards bit-identical.

**Finding:** `succeed_house`'s branch-on-succession (30% of wealth spun off at every
gen>=2 succession) can make "three consecutive GROWING heads" genuinely hard to reach
even in a compounding economy — worth knowing before reading the dynasty-fire rate off a
real campaign as a fidelity signal.

---

## Current state — 2026-07-30 (Phase 1.1 · house tiers)

Read-only, query-side classification — no economy number moves. `assign_house_tiers`
bands every live private house into a rank (1 great .. 4 marginal) from state that
already existed, with hysteresis on both the percentile cutoffs and Tier 1's absolute
floor. `HousesPanel.tsx` groups the list by tier (3/4 collapsed by default, per
`HOUSE_PEOPLE_AND_TIERS.md` §1's schematic). 58 `tick::` tests pass (was 55); dynamics
and economy scorecards bit-identical to the Phase 0.4 numbers below — nothing downstream
reads `tier`, by design.

---

## Current state — 2026-07-30 (Phase 0.4 · inheritance)

Only the numbers that MOVED. Everything else still reads as the 2026-07-29 table below.

| Metric | Value | Gate | Status |
|---|---|---|---|
| **Economy: mean firm lifespan** | **36.8 yr** (was 96.9) | `econ_diagnose_house_turnover` | ✅ **inside the 30–90 band for the first time** |
| Economy: lifespan excl. stillbirths | **147.0 yr** (was 193.8) | same | ❌ established firms still almost never fail — Phase 3's job |
| Economy: house wealth Gini | **0.609** (was 0.853) | `ECON_GINI_FLOOR` = 0.15 | ✅ **back inside the 0.60–0.85 band**, at its floor |
| Economy: top-10% wealth share | **0.422** (was 0.809) | — | ❌ **left the 0.60–0.90 band from below** — the merchant elite is now too flat |
| Economy: houses alive at 60 yr | **42** (was 2) | — | ⚠️ the reference world finally HAS a merchant class |
| Economy: house dissolutions / century | 46.7 (was 10.0) | — | ⚠️ stock-dependent — read the lifespan row instead |
| **Inheritance rule is wired** | partible **18 divisions / 22 co-heirs**; primogeniture · ultimogeniture · seniority **0** | `econ_inheritance_rules_fragment_differently` | ✅ asserted |
| Inheritance: houses ever founded | partible **88** · primogeniture **55** · ultimogeniture **49** · seniority **124** | same | ✅ the rule measurably changes fragmentation |
| Inheritance: mean wealth per house | partible **120 325** · primogeniture **195 264** | same | ✅ same capital, spread thinner |
| **Rust tests** | **171 pass, 0 fail** (4 ignored) | CI | ✅ |
| Dynamics: sustained richest house | 154 045 — **unchanged** | `late_max < 1e6` | ✅ bit-identical (that world seeds no successions) |

**Why so much moved at once.** The reference world was not reproducing campaign start:
`tests::sim()`'s placeholder gave every seeded head a **274-year** lifespan, so not one
of the ten houses ever reached a succession inside a 60-year run. Every number that
depends on generational turnover — lifespan, Gini, top-10%, surviving houses — was
measuring a world where merchant families were immortal. `calibrate_like_campaign_start`
now runs the same two steps `campaign_start_sim` does (`ensure_culture_rules` +
`seed_house_lines`). The old numbers were not wrong measurements; they were measurements
of the wrong world.

---

## Current state — 2026-07-29

| Metric | Value | Gate | Status |
|---|---|---|---|
| **Earth main-class agreement** | **70.2%** | `EARTH_MAIN_FLOOR` = 70.1 | ✅ asserted |
| **Earth exact-zone agreement** | **39.0%** | `EARTH_EXACT_FLOOR` = 38.8 | ✅ asserted |
| Earth C-class own accuracy | 32.2% | — | worst class |
| Earth `C → B` confusion | 39% | — | largest single error |
| Earth `D → E` confusion | 18% | — | second largest |
| **Economy: price/distance gradient** | **−0.01** | *none* | ❌ distance does not move prices |
| Economy: grain price CV across cities | 2.10 | `ECON_SPATIAL_CV_FLOOR` = 0.01 | ⚠️ far above band (0.20–0.40) |
| Economy: rank-size (Zipf) slope | −0.41 | band [−3.0, −0.15] | ⚠️ flatter than −0.8…−1.2 |
| Economy: urban share drift (60 yr) | 0.100 → **0.997** | — | ❌ countryside empties completely |
| Economy: house dissolutions / century | **10.0** (was 312) | — | ⚠️ superseded — use lifespan below |
| **Economy: mean firm lifespan** | **96.9 yr** (was ~12) | `econ_diagnose_house_turnover` | ⚠️ slightly ABOVE band (30–90) — now stable and measurable |
| Economy: lifespan excl. stillbirths | **193.8 yr** | same | ❌ established firms now almost never fail — Phase 3's job |
| Economy: house wealth Gini | **0.853** (was 0.828) | `ECON_GINI_FLOOR` = 0.15 | ❌ **just left the 0.60–0.85 band** — the cost of fixing turnover |
| Economy: top-10% wealth share | **0.809** (was 0.712) | — | ⚠️ in band (0.60–0.90), rising |
| Dynamics: sustained richest house | 154 045 | `late_max < 1e6` | ✅ was 297 748 before the feud rework |
| Dynamics: peak house wealth | 370 527 | finite + bounded | ⚠️ still an order above the "no 100k" ideal |
| **Province land layer** | **unmeasured by either oracle** | own tests only | ⚠️ see below |
| **Economy: tick determinism** | **PASSES** | `econ_scorecard_is_deterministic` (no longer ignored) | ✅ **fixed — 4 hash-order sites, see below** |
| **Rust tests** | **166 pass, 0 fail** (8 ignored) | CI | ✅ |
| **Frontend tests** | **0** | *none* | ❌ 33k lines uncovered |
| `cargo check` | clean | CI | ✅ |
| `npx tsc --noEmit` | clean | CI | ✅ |
| Phase 3 wall time @ 3600×1800 | ~16 s (release, 4 cores) | `bench_ocean_atmosphere` | ✅ |
| Rust / TypeScript LOC | 55.9k / 33.2k | — | — |

---

## How to reproduce every number here

```bash
# Climate fidelity — main-class, exact-zone, confusion matrix, spot checks
cd src-tauri && cargo test --lib earth_ -- --nocapture

# Economy fidelity — the full scorecard against pre-modern reference series
cd src-tauri && cargo test --lib econ_ -- --nocapture

# Economy dynamics — bounded wealth, house turnover, determinism
cd src-tauri && cargo test --lib simulate_decades_reports_dynamics -- --nocapture

# Everything
cd src-tauri && cargo test --lib
cd src-tauri && cargo check
npx tsc --noEmit

# Performance (release, slow, ignored by default)
cd src-tauri && cargo test --release --lib bench_ocean_atmosphere -- --ignored --nocapture
cd src-tauri && cargo test --release --lib ocean_atmosphere_field_checksums -- --ignored --nocapture
```

---

## The two oracles

An **oracle** is a test that answers "is this good?" without the maintainer
needing to be a domain expert. The project has two, and they are the reason any
of this is knowable:

1. **`sim/step4_climate/earth_validation.rs`** — scores the generated climate
   against the real Köppen-Geiger map (Kottek & Rubel, 0.5°). Hard-asserts
   `EARTH_MAIN_FLOOR`. **Raise the floor after every improvement** so it always
   guards the current best.

2. **`sim/campaign/tick/economy_validation.rs`** — scores the campaign economy
   against published pre-modern price, wage, urbanisation and inequality series
   (Allen, Federico, Persson, De Vries, Alfani, Van Zanden). Most metrics are
   **printed, not asserted**: a printed metric outside its historical band is a
   *finding*, not a build failure. Promote metrics to assertions as the model
   earns them.

**Track exact-zone, not main-class.** Class E scores 99.1% for free — polar is
just "cold" — which inflates the aggregate. Exact-zone is where the real state of
the climate model lives, and it is currently ungated. Adding an
`EARTH_EXACT_FLOOR` is the cheapest fidelity improvement available.

---

## ⚠️ Open defect: the campaign tick is not deterministic

`CLAUDE.md` §5 states a tick is "pure & deterministic per `(seed, tick)`". **It is
not, once the economy is actually trading.** Two identical reference worlds run in
one process produce different scorecards.

**Cause.** HashMap iteration order feeding **float accumulations**. Float addition
is not associative, and Rust's `RandomState` gives every HashMap instance its own
iteration order, so identical inputs fold to different sums. Two sites are fixed
(`classify_hubs`'s `throughput`, and `flow_year`'s ordering — both `cities.rs`);
the divergence shrank but did not vanish. Roughly a dozen accumulator maps remain
in `houses.rs`, `disease.rs`, `colonies.rs` and `mod.rs`.

**Why it hid for so long.** The existing determinism assertions in `tests.rs` run a
world where `tests::sim()` hard-codes `need_scale = 1.0` — about **84× real
demand**. Every hub sits in permanent famine, `dispatch` never sees a surplus, so
almost nothing is traded and the accumulator maps stay nearly empty. Order cannot
matter when there is nothing to order. Calibrating the reference world to real
campaign-start conditions is what exposed it.

**Consequence for this file.** Every economy number above is a single sample from a
non-reproducible process. Treat them as indicative of magnitude, not as
measurements, until determinism is restored. That is the first economy work to do.

**Fix.** Audit every hash accumulator in `tick/`, sort by key before folding, and
hold `simulate_decades_reports_dynamics` bit-identical at each step. Then remove
the `#[ignore]` from `econ_scorecard_is_deterministic`.

---

## Phase 0.4 · the law of inheritance — built, and two defects it exposed

**What was built.** Two enums on the culture (`sim/shared/inheritance.rs`): a LINE rule
(agnatic · agnatic-cognatic · absolute · enatic) and a DIVISION rule (partible ·
primogeniture · ultimogeniture · seniority · matrilineal), assigned per language kit
where the record is clear and seeded where it is not. They are read at one place —
`succeed_house` — and decide three things: who inherits (the heir's sex, and the name
bank they are drawn from), **how old they are when they do**, and whether the estate
divides.

**The age is the part that mattered most.** An heir was previously handed a fresh 45–75
year "lifespan" as their TENURE, i.e. every head was effectively born on the day they
inherited. They now inherit at an age the rule implies — an eldest son at ~27–45, a
hearth-keeping youngest at ~17–31, an elected elder at ~44–62 — and rule for what
remains of a life. That alone is what makes ultimogeniture and seniority behave
differently from primogeniture without a single extra mechanism.

**The gate.** `econ_inheritance_rules_fragment_differently` runs ONE world four times,
changing only the law:

| rule | houses ever | successions | divisions | co-heirs | mean wealth |
|---|---|---|---|---|---|
| partible | 88 | 61 | 18 | 22 | 120 325 |
| primogeniture | 55 | 57 | 0 | 0 | 195 264 |
| ultimogeniture | 49 | 45 | 0 | 0 | 164 205 |
| seniority | 124 | 147 | 0 | 0 | 103 372 |

Note what partible does **not** do: the top share and Gini do not fall, because a
division adds small firms at the bottom as fast as it trims the top. What moves is mean
wealth per house — the same capital spread over more houses. Seniority fragments by a
different route entirely: short tenures → three times the successions → far more cadet
branches.

### Defect 1 — a house's chronicle was eating its own milestones

`HOUSE_EVENTS_CAP` kept the 60 most recent events and dropped the oldest. In a hot feud
a house generates dozens of flare entries a year, so **a family lost its own founding
and every succession within a couple of years**. A 500-year dynasty's chronicle read as
three weeks of shipping losses — and it silently zeroed the division metric above, which
is how it was found. Milestones (founding, succession, division, monopoly, charter,
ruin) are now never evicted by chatter; only chatter is pruned.

This matters beyond the metric: `HOUSE_MASTER_PLAN` 2.3 concluded the chronicle IS the
product for an observation-only game. It was being deleted.

### Defect 2 — cadet branches were the new stillbirth path

With successions actually firing, the turnover diagnosis was re-run with a breakdown by
**how the dead house was founded** — and 19 of 35 deaths were cadet branches, 74% of
which never traded, dead at a mean age of 8 years. `found_branch` endowed a branch with
30% of the parent's wealth **and** `initial_fleet`'s two or three vessels it had never
paid for. That is precisely the arithmetic Phase 0.2 found behind the original 12-year
house, arriving through a second door. A branch now inherits capital only and buys hulls
from it when its trade justifies them.

Effect: mean firm lifespan **29.4 → 36.8 yr**, real-firm mean age at death 7.7 → 19.2.

### What is still open here

- Co-heir houses are **100% stillborn** when they die (8 of 28 deaths, mean age 7.2 yr)
  and branches are still 86%. They have capital and no fleet, so the endowment is not
  the cause this time — a new house appears to have no way to originate trade at its own
  seat. That is the next turnover question, and it is a *diagnosis* task, not a constant
  to tune.
- **Top-10% wealth share fell out of band from below (0.422 vs 0.60–0.90).** The
  merchant elite is now too flat. This is the mirror image of the Phase 0.2 finding and
  points the same way: at Phase 3, which is supposed to make the top of the distribution
  fragile rather than making the bottom crowded.

---

## Phase 0.1 · house turnover — diagnosed, fixed, and the cost measured

**The finding.** A house was born with `wealth: 1.0` and a two-to-three vessel fleet
costing ~0.70–1.05/month. That is ~1.4 months of runway at birth, so it went negative in
its second month, `update_solvency` ran its twelve-month clock, and it died at ≈13.4
months. Measured median age at death: **1.1 years** — the arithmetic to two significant
figures. **73% of all dissolutions were houses that never traded at all.** The
`dissolutions/century` metric was therefore counting *stillbirths, not failures*.

**My hypothesis was wrong.** I predicted overextension from ambition, i.e. a negative
correlation between age at death and committed upkeep. Measured: **+0.802** — houses that
committed more upkeep lived *longer*. The fatal commitment was the founding endowment, not
accumulated ambition.

**The fix.** Not a bigger constant. `maybe_found_house` already requires a guild at the
hub, so the seed capital is taken **from that guild** — a family separating out with its
share, as it historically did. Three properties: no money is created; a guild too poor to
endow a viable family cannot spawn one (churn stopped at source); and the seed scales with
how rich the local trade actually is.

**Result:** mean firm lifespan **~12 yr → ~51–101 yr** (band 30–90); dissolutions/century
312 → 10.

**Two things this exposed, both worth more than the fix:**

1. **`dissolutions/century` is the wrong metric.** It scales with how many houses are
   standing, so the same mortality reads differently in a 20-house and a 50-house world.
   And a 60-year run cannot observe a 90-year lifespan — the survivors are right-censored.
   The correct estimator is a hazard over exposure: `deaths ÷ house-years lived`, using the
   living houses' time instead of discarding it. That is what the lifespan row above reports.

2. **The determinism defect blocked further tuning — and is now FIXED (Phase 0.3).**
   Three runs of the same test on the same binary gave **11, 11, 6** deaths and lifespans of
   **51.1, 51.1, 101.2 yr** — a 2× swing straddling the band boundary. Four sites were
   folding or ordering by HashMap iteration order:

   | Site | What it broke |
   |---|---|
   | `money.rs::update_currency_baskets` | summed a partner-volume map with `+=` and divided every basket weight by that total; float addition is not associative, so the coin basket flipped |
   | `production.rs::fold_trade_year` | pushed new series onto `trade_hist` in map order; the peak sort is *stable*, so equal peaks kept insertion order and a different set survived truncation |
   | `mod.rs` culture desire | built `hub_desire[h]` as a `Vec` from a map |
   | `colonies.rs::update_lingua_franca` | iterated components in map order **and** resolved the dominant-culture `max_by` tie by hash order |

   Each now iterates in key order with an explicit tie-break. Three identical runs
   confirmed, and `econ_scorecard_is_deterministic` is **no longer ignored** — it is the
   guard that stops the defect returning, and any new hash accumulator in `tick/` trips it.

**Where turnover landed (final, deterministic).** Mean firm lifespan **96.9 yr** against
the 30–90 band — the overshoot is deliberate and *not* being tuned away: the remaining gap
is that **established firms almost never fail** (193.8 yr excluding stillbirths), and the
honest fix for that is a failure mechanism (the Phase 3 crisis layer), not a smaller seed
constant. Shrinking the seed would re-introduce the stillbirths that caused the original bug.

**The cost, measured: `HOUSE_MASTER_PLAN`'s open risk was real.** Wealth Gini rose
0.828 → **0.853**, just outside the 0.60–0.85 band, and the top-10% share rose
0.712 → 0.809. Houses dying young *was* partly load-bearing: it was destroying wealth in an
economy that compounds at 1.5%/yr with no other brake. So the two anomalies were **in
tension, not one bug**, and the phase boundary in that plan is wrong — Phase 0.2 needs the
Phase 3 crisis layer as its replacement brake, and the two must be co-tuned.

---

## The province land layer is unmeasured by both oracles

`province_land_pass` (FIX_PLAN B1) closes the world↔campaign feedback edge — a
province's surplus reaches its seat city's granary and its dues reach that city's
treasury. Neither fidelity oracle sees it:

- **`simulate_decades_reports_dynamics` seeds no provinces**, by design. That is what
  makes the land layer provably free of side effects on the base economy
  (`province_land_pass_is_a_noop_without_provinces` asserts it), but it also means the
  standing dynamics run says nothing about whether the land behaves.
- **`economy_validation.rs` seeds no provinces either**, so urbanisation, grain prices
  and real wages are all still measured on a world whose countryside is only a
  population reservoir.

What covers it today is four of its own tests (feedback edge + bounds, the no-op gate,
works cost money and take years, unfunded work stalls). What would actually measure it
is a province-seeded variant of the economy harness — the urban-share drift row above
(0.100 → 0.997, the countryside emptying completely) is precisely the metric a working
supply shed should move, and it is the obvious next thing to ask of this layer.

---

## House trade outposts — measured, fixed, still not fully explained

Player-reported: outposts basically never appeared over ordinary play. A 150-year
instrumented run (`econ_diagnose_outpost_founding`, `#[ignore]`d) on the reference
world found the wealth bar was never the blocker (cleared 96.8% of months) — two real
structural bugs were: only the single richest house ever got a try each year, so the
mechanism stalled for good the moment that ONE house's network stopped bordering a
remaining site; and ordinary estates (founded far more often) could exhaust the shared
`MAX_TOTAL_ESTATES` budget outposts draw from too. Fixed both (every qualifying house
gets a try, richest first, up to `OUTPOST_MAX_PER_CALL`; `OUTPOST_RESERVED_ESTATES`
holds back budget outposts can't be starved out of) and added a house's own estates as
network anchors alongside home+offices. Confirmed in the standard 50-year dynamics
gate: outposts now found at year 30 and reach 2 by year 35, where every prior scorecard
run in this file shows a flat 0 for the whole window. The 150-year diagnostic itself
still plateaus at 2 outposts after year 31 on this specific fixture — attributed to
`reference_world()`'s colonizable sites sitting in one compact band disjoint from most
hubs (a geometry no real generated world has), not re-tuned against blindly per §2.4 —
left as an open item to confirm against a real generated world.

Financed expeditions (`expedition_launch_pass`) were rewired the same session: the old
scoring rewarded raw distance with no ceiling, so a corridor could only ever reach the
single farthest city (structurally >5,600 km on an Earth-scale world). Now bounded to a
regional ≈1,400–8,800 km band with a "sweet spot" peak near the floor, so several
shorter corridors are viable instead of one maximal one.

---

## What is still unmeasured

Being explicit about this matters as much as the table above — an unmeasured
subsystem is one you cannot have an opinion about.

- **The entire frontend.** 33k lines, zero tests. `tsc --noEmit` proves the types
  agree with each other, not that anything works.
- **Rust ↔ TypeScript type drift.** `types/campaign.ts` hand-mirrors Rust serde
  structs. A field rename produces a silent runtime `undefined`, not an error.
- **Peak memory.** 26M cells × 25+ columns on "Large" worlds. Time is benchmarked;
  memory is not, and memory is the likelier failure on a customer's machine.
- **Frame rate.** No measurement of pan/zoom under load with overlays enabled.
- **Save-format forward compatibility.** The v2 self-describing blob design is
  sound, but a compatibility claim with no old-save fixture behind it is a hope.
- **Anything about the app as a product** — install success, first-run
  completion, time to a finished world.

---

## History

| Date | Commit | Earth main | Earth exact | Rust tests | FE tests | Note |
|---|---|---|---|---|---|---|
| 2026-07-31 | *this* | **70.2%** | 39.0% | 227 | 0 | Ocean evaporation's wind term was DEAD CODE — it read `|belt_wind|`, which is a unit vector, so the factor was identically 1.0. Now reads `jets::base_speed`, the real belt speed profile, as the bulk formula `E ∝ U·(q_s − q_a)` requires |
| 2026-07-31 | *this* | 70.1% | **39.0%** | 227 | 0 | **Köppen no longer emits `H`.** Highland has no Köppen counterpart — the reference calls Tibet and the high Andes `ET`/`EF`/`Dwc` — so every `H` cell was unmatchable by construction. Exact-zone 33.7 → 39.0, the largest single move of the session, with main-class *identical* (it only ever sat on terrain the reference already calls polar). Alpine is unaffected on the Biomes layer, which has its own altitudinal band. Graded rain shadow tried and reverted (A15) |
| 2026-07-31 | *this* | **70.1%** | **33.7%** | 227 | 0 | **Seasonal monsoon adopted (FIX_PLAN A14).** The wind belts now migrate with the ITCZ and cross-equatorial flow recurves, so monsoon winds actually reverse: 0/7 → 4/7 sites, now ASSERTED by `earth_monsoon_wind_reverses`. Exact-zone to its best ever. Main-class floor LOWERED 70.6 → 70.0 — the only lowering in this file's history, a deliberate trade (the arid belt had been propped up by a wind that never changed direction). ITCZ overlay now draws both seasonal lines with the migration band hatched between them |
| 2026-07-31 | *this* | **70.8%** | **32.8%** | 226 | 0 | Continental seasonal span raised (`K_SEASONAL` 0.20 → 0.24). The generated warmest−coldest span at 60–70°N was 28.6 °C against a real 57–65 in Siberia, which made `Dfd`/`Dwd` *arithmetically impossible* (they need `t_coldest < −38 °C`). D row 58.5 → 70.8; `Dfd` and `Dsa` go from never-emitted to present. Cost is the C row (34.5 → 31.5) |
| 2026-07-31 | *this* | **69.6%** | **31.9%** | 226 | 0 | Orographic uplift made a graded response to upslope RISE (Smith & Barstad `w = U·∇h`) instead of a binary `elevation > 1681 m` test. Measured: the Western Ghats, Appalachians and NZ Southern Alps cleared that threshold in ZERO cells, so three of the wettest orographic coasts on Earth produced no uplift at all. C row 33.0 → 34.5; Mumbai `B→A`, SE-US now C. Also adds a Köppen ZONE CENSUS: 5 zones are never emitted (all `Dw*`/`*d`) and H is 8.07% of land against 0% in the reference |
| 2026-07-31 | *this* | **69.4%** | **31.8%** | 226 | 0 | Moisture emission scaled by SST via Clausius-Clapeyron (bulk formula). The source was a 3-valued step on `current_type` and, because only boundary currents poleward of ~18° carry a tag, it made the mid-latitudes the model's strongest moisture source and the equator its weakest — backwards. A row 83.8 → 85.1, exact-zone 31.6 → 31.8. Gain damped to 0.30 (sweep in the constant's doc comment). Two REVERTED negative results recorded in FIX_PLAN A7/A8 |
| 2026-07-31 | *this* | **69.2%** | **31.6%** | 226 | 0 | Snow-albedo cooling confined to the COLD SEASON (it was lowering the annual mean, so `seasonal_temps` put the full 4 °C on July — and Köppen's D/E boundary IS the warmest month). D row 49.8 → 58.7, `D → E` 37% → 30%. Also documents the subtropical basin-position asymmetry (Miyasaka & Nakamura 2005) that entered `6d0aaa1` unreviewed. Floors 67.0 → 69.0; `EARTH_EXACT_FLOOR` added at 31.0 |
| 2026-07-30 | *this* | **67.4%** | **30.0%** | 225 | 0 | Shelf-velocity fix: `generate_ocean_currents` no longer zeroes current_vx/vy on shelf cells (a rendering concern moved to `render_currents`). `compute_upwelling_zones` was measurably DEAD — 0 usable sources, 0 cells cooled — and is now 3 428 sources / 872 cells / up to 4 °C. First Earth-score move since `d53fdc9`. Mumbai C→A and SE-US B→C now match reference; `D → E` 40%→37%. Floor raised 65.0 → 67.0 |
| 2026-07-30 | *this* | 66.3% | 29.1% | 224 | 0 | House lineage tab + Compare window + figure variation + enlarged dossier window; outpost/expedition regional-reach fixes (see below) |
| 2026-07-29 | `936a8a3`+ | 66.3% | 29.1% | 159 | 0 | Economy oracle added; CI added; scoreboard created |
| 2026-07-29 | *this* | 66.3% | 29.1% | 159 | 0 | Harness calibrated to real campaign start; LOD sampler fixed; tick determinism defect found |
| 2026-07-30 | *this* | 66.3% | 29.1% | 166 | 0 | Phase 0.3: tick determinism FIXED (4 hash-order sites); guard un-ignored |
| 2026-07-30 | *this* | 66.3% | 29.1% | 165 | 0 | Phase 0.1/0.2: firm lifespan ~12 → ~51–101 yr (seed capital from the parent guild); Gini 0.828 → 0.853 (left band — measured cost); determinism defect promoted to a blocker |
| 2026-07-29 | *this* | 66.3% | 29.1% | 165 | 0 | Feuds elaborated (cause/stage/ending); province LAND state + B1 feedback edge; sustained richest 297 748 → 154 045; Gini 0.771 → 0.828 |
| — | `d53fdc9` | 66.2% | 29.0% | — | 0 | FIX_PLAN baseline |
