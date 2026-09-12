# Institutions — the sequenced build order

> **DECIDED IN SCOPE, NOTHING BUILT.** The design and the variants are in
> `docs/INSTITUTIONS_BRAINSTORM.md`; this file is the order, the gates, and what
> each slice owes the chronicle. Four decisions were taken by the maintainer and
> are recorded in §0 — they are the reason this order is what it is.

---

## 0. The four decisions this plan is built on

1. **The player stays OBSERVATION-ONLY.** The four mutating verbs (§5.1) stand.
   → **B-D "Play the Bank" is dropped**, and recorded as considered-and-rejected
   rather than deferred, so it is not re-proposed.
2. **All three chains, sequenced**, one plan, a gate per slice.
3. **Measure the craft spread before choosing its number.** Phase 0 owes a
   diagnostic; the dose decision comes after it, not now.
4. **Murano as such** — the named craft city with a protected secret that can be
   stolen or carried away — scoped as ONE feature, not as a general guild-powers
   rework.

### The governing rule this plan adds

**Observation-only means every mechanism must produce a LEGIBLE STORY, not a
decision.** That is not a softening; it is a harder constraint than a player
verb, and it disqualifies otherwise-good mechanics.

A slice earns its place only if a reader can, after the fact, answer *what
happened and why* from the chronicle and the panels alone. Rule 20 already says
the chronicle is the product for an observation-only game; this plan makes that a
per-slice acceptance criterion. So **every slice below names what it writes to
the chronicle**, and a slice that moves numbers and writes nothing is not done.

The corollary bites in a useful direction: a mechanism whose whole effect is a
percentage on a hidden accumulator (the current war blockade, `export_earn` ×
0.55) fails this test even though it is real. A mechanism that is smaller but
*nameable* — "the Doge bars pitch and hemp to Genoa" — passes.

---

## Phase 0 · Instruments. Nothing changes.

Three diagnostics, all `#[ignore]`d, no production code touched. This phase
exists because two of the three chains below are dose-walks, and
`ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` §10 Q1 already established that a dose
walked against a blind instrument is how this codebase wastes a session.

- **0.1 · `econ_measure_finance`.** Banks chartered / failed / surviving and mean
  lifespan; bank income by class (interest · dividends · bills · colony
  dividends); the share of house wealth that is borrowed; loans by purpose;
  `diag_why_cash` refusals; and the live maximum of `demand_pressure_at` (which
  settles `CONSUMPTION_AND_GOODS_REVIEW`'s unsatisfiable-workshop-gate finding,
  measured pre-S1 and stale). *Baseline for phases 1 and 3.*
- **0.2 · `econ_measure_craft_spread`.** Per manufactured good: how many hubs make
  it, the min/median/max `quality`, the realised `quality_value_mult` spread, and
  **whether the leader is simply the largest city.** *This is decision 3's
  instrument and it gates phase 2's dose.*
- **0.3 · `econ_measure_war_trade`.** What a war costs a belligerent today —
  volume on lanes touching a belligerent, before and during, against a peaceful
  control. Expected result: **no difference**, since `dispatch` never reads
  `self.wars`. *Baseline for phase 4; a zero is exactly the point.*

*Gate:* `cargo check --lib --tests`, then each by name (§2.8's diagnostic row).

---

## Phase 1 · Stop the bleeding. Two dead things, both cheap.

Both fix a measured defect rather than adding a mechanism, and both unblock later
phases.

- **1.1 · Bank failure splits from contagion.** Three outcomes — **wound down**
  (reserves cover deposits: depositors paid, bank closes, no crash), **absorbed**
  (a solvent rival in the same component buys the book at a discount; `Loan` and
  `BankStake` are already transferable records), **collapse** (today's path, only
  when deposits are actually wiped). Only collapse ignites contagion.
  *Why first:* 12 crashes in 50 years hammer coin trust to 43%, below
  `RESERVE_TRUST_MIN` 0.55, which closes `coin_discount` — money's only channel
  into trade — for the back half of every campaign. This is a live trade bug, not
  just a finance one.
  *Chronicle:* "The Banco di X is wound up; its depositors are paid in full" /
  "The Banco di Y takes over its books". A bank that dies quietly must still be
  legible, or the mechanism reads as banks silently vanishing.
  *Gate:* coin trust must stop collapsing below 0.55 and stay up; wealth bounded
  (fewer crashes = one fewer `CRASH_WEALTH_HAIRCUT`, which is currently doing real
  work holding fortunes down — that is this slice's actual risk).

- **1.2 · A bank may hold the Monte.** Drop the `if kind != 0 { continue; }`
  filter from the coupon, deleverage and haircut loops, and let
  `update_public_debt` subscribe a bank. 157,677 of public debt exists across 19
  cities and no bank may touch a penny of it.
  *Why here:* it is the prerequisite for phase 3's whole point, and it gives banks
  a safe asset that is not a loan to a merchant.
  *Chronicle:* the Monte's subscribers by name; a civic default names the banks it
  hurt.
  *Gate:* a fixture where a city defaults must haircut a bank holder by exactly
  the same fraction as a house holder; dynamics bounded.

- **1.3 · `League.purse` buys a convoy.** The pot has one writer and zero readers.
  Dues fund escort: reduced voyage loss on member-to-member lanes.
  *Why here:* it is nearly free, it fixes a dead field, and it is the first thing
  that makes a league visible in a number.
  *Chronicle:* "The League fits out a convoy for the season" — and, when it
  lapses, that it could not afford one.
  *Gate:* a league with no purse is bit-identical; dynamics unchanged.

---

## Phase 2 · Why a place is itself. (Chain 3 — the Murano feature.)

Weighted early because the player is observation-only: this is the chain that
makes the world worth watching. Lowest risk to the economy gates of the three.

- **2.1 · Give the quality ceiling a term that still discriminates among mature
  cities.** *Rewritten after Phase 0.2 ran; the prediction it was first written on
  was falsified — see the correction below.* Today
  `cap = (0.62 + size_bonus + struct_bonus).clamp(0, 0.97)`, where `size_bonus` is
  `(pop / 60_000).min(0.20)` — a term that **saturates at 60,000 population**. So
  it separates a village from a town and then stops discriminating entirely: every
  real city with a workshop and a guildhall lands on exactly 0.960. Add a term that
  keeps working above that — **accumulated tradition** (years of continuous
  production of that good at that hub) plus the master's arrival from 2.2.
  *Dose decided by 0.2, not now* (decision 3). Nothing else in this phase reads
  until this lands.

  > **CORRECTION — the prediction was wrong, and what replaced it is better.**
  > This slice first claimed "the largest city is automatically the finest maker of
  > everything it makes." `real_world_craft_spread` measured the opposite: across
  > 14 manufactured goods on a real world, the finest maker was the biggest city
  > **0 times**. Not because tradition already works — because **there is no finest
  > maker at all.** Quality SATURATES: for most goods the median maker is already at
  > the maximum, so "the leader" is an arbitrary tie-break among cities sitting on a
  > shared ceiling. The one thing that lifts a city above it is `LAW_GUILD_MONOPOLY`
  > (`GUILD_MONOPOLY_QUALITY_CAP` 0.97 against the natural 0.96) — so the single
  > mechanism already in the tree that produces a distinctive maker is a guild
  > charter, which is a good omen for 2.2 and the reason this slice's target moved
  > from "untie it from size" to "add a term that still discriminates". Recorded per
  > §2.4: a prediction from reading the code, falsified by running it.
  *Chronicle:* nothing yet — this is the enabling change. Say so rather than
  inventing an event for it.
  *Gate:* 0.2 re-run (the leader must stop being merely the biggest city); the
  multi-seed inheritance gate per dose step; `econ_expenditure_shares_resemble_a_
  household` (a wider price spread on manufactures moves the luxury share).

- **2.2 · The secret, and the master who leaves.** `maybe_steal_quality` already
  implements industrial espionage — the *theft* half of Murano is built. Add its
  counterparty: a guild with a hall and a monopoly charter accrues **secrecy**
  that resists theft; and a rival city can **poach a master**, who carries the
  technique with him (`STEWARD_POACH_CHANCE` already implements poaching for
  offices — reuse that shape, not a new one). Secrecy resists; a rich rival's
  treasury beats it.
  *Why it matters:* this is what gives a trade good a HISTORY — invented
  somewhere, guarded, eventually diffused. No good in the world currently has one.
  *Chronicle:* the invention, the charter, each attempted theft that fails, and
  the defection — named on both sides, in both cities' chronicles. This is the
  slice with the richest story per line of code in the whole plan.
  *Gate:* a world with no guilds is bit-identical; a guarded craft must measurably
  diffuse SLOWER than an unguarded one (or secrecy is decoration).

- **2.3 · The signature.** A city that has made a good long enough and well enough
  earns a named signature — "Muranese glass" — that travels with the cargo and
  prices anywhere. `names::gen_name` and `brand_name`/`brand_place` already exist
  (the latter already brands colony output); `quality_value_mult` already prices
  quality.
  *Chronicle:* the naming, once, as a milestone (`is_house_milestone`'s discipline
  applied to a city).
  *Gate:* mostly wiring; dynamics bit-identical except through 2.1's dose.

- **2.4 · The refining entrepôt.** A city with high throughput and no local raw
  gets a manufacturing bonus for goods whose **inputs it imports** — Amsterdam
  refining Caribbean sugar, and `refined_sugar` is already a shipped recipe of
  exactly this shape. Makes a port structurally different from a producer, which
  is the largest missing distinction between settlements.
  *Chronicle:* the city's own description changes — "lives by what passes through".
  *Gate:* trade volume must not collapse; the price/distance gradient must not
  regress (`real_world_price_distance_gradient`).

---

## Phase 3 · War is paid for by borrowing. (Chain 1b.)

The historically central linkage and the cheapest big win in the plan, because
every piece already exists and none of them are connected.

- **3.1 · War spending issues debt.** Today a war is paid from treasury plus
  forced house levies. Route it through `update_public_debt` instead: a war
  spikes issuance against throughput, `DEBT_DEFAULT_RATIO` already haircuts
  holders, `fail_bank` already cascades. With 1.2 in place the chain closes:
  **war → debt spike → a bank holds the bond → the war is lost → haircut → the
  bank fails → the crash.** That is 1345, assembled from four systems that each
  already work.
  *Chronicle:* the loan that financed the war, named; the haircut, named; the bank
  that fell because of it, named. The whole point of this slice is that the crash
  finally has a *cause a reader can follow*, where today `trigger_regional_crash`
  fires from an anonymous insolvency.
  *Gate:* dynamics bounded; debt/throughput stays inside `DEBT_MAX_RATIO`; crash
  frequency must not climb back toward phase 1's baseline (1.1 bought that
  headroom and this slice spends some of it — measure, don't assume).

---

## Phase 4 · Exclusion, dosed. (Chain 2 — riskiest, therefore last.)

**One rule governs the whole phase: every prohibition ROUTES, it never refuses.**
An excluded lane goes through a neutral port at extra cost via `staging_hop`,
which is already built and already dosed live. This is both the historical shape
(the neutral entrepôt in wartime is the mechanism — Baltic grain kept reaching
Amsterdam) and the only shape this codebase has measured as safe: at identical
doses, refusing collapsed the inheritance gate's world to 3 live houses while
routing left it healthy, and N2's market closure broke the hard wealth bound
twice because **an export-locked market's rent concentrates harder than the plan
anticipated.**

Ordered by blast radius, smallest first.

- **4.1 · Contraband.** A war bans a short list of goods to the enemy — metalware,
  iron, timber, and the naval stores `pitch`/`hemp` that
  `YARDS_VESSELS_AND_DEPOTS_PLAN` already shipped — and leaves everything else
  flowing. This is `export_ban_until`'s exact shape, already wired into
  `dispatch`, currently at `INFINITY`. Safest possible first dose of exclusion: a
  handful of goods, not a market.
  *Chronicle:* the proclamation, by name, listing the goods.
- **4.2 · The routed blockade.** War enters `dispatch`'s prohibition list as a
  staging rule. Dose from zero.
  *Chronicle:* "cargo for X now goes by way of Y" — the detour must be visible or
  the mechanism is invisible.
- **4.3 · League privileges.** Members grant each other a tariff exemption and a
  freight discount; outsiders pay full. First thing that makes membership worth
  anything, and it makes blocs legible with no new overlay.
- **4.4 · The Kontor.** A shared factory at a NON-member city with negotiated
  privileges — Bruges, Bergen, Novgorod, the Steelyard. Members route through it;
  the host can expel it, which is a real political event.
  `YARDS_VESSELS_AND_DEPOTS_PLAN` already names the missing ownership class as the
  **fondaco**; a Kontor is that with a collective owner. This is what puts a
  league on the map instead of in a list.
- **4.5 · The boycott, dosed.** Built, at `LEAGUE_BOYCOTT_MAX = 0`. Denmark
  1361–70. **Last, and expect trouble** for N2's measured reason.

*Gate, every slice, per dose step:* the multi-seed
`econ_inheritance_rules_fragment_differently`, long-haul trade volume on
`tests::dense_world`, and `simulate_decades_reports_dynamics`' sustained-richest
bound. A slice that cannot hold all three at any non-zero dose ships at zero with
its negative result written down (§2.4), exactly as N2 did.

---

## Phase 5 · The apparatus. (Orthogonal — may land beside any phase above.)

- **5.1 · The cadastre.** A fifth `ProvWork` kind: expensive once, per province,
  permanently raising that province's collection efficiency. Domesday, the
  Ottoman *defter*, the Milanese *catasto*. Reuses `ProvWork` entirely — multi-year,
  funded-or-stalls, crown-funded, cost already scaled by real area and relief —
  and shows on the province plate. *A state that knows what it owns collects more*
  is the whole thesis of pre-modern fiscality, and today the entire administrative
  model is `cohesion × distance_decay`.
  *Chronicle:* the survey ordered, and completed a decade later.
- **5.2 · Farm or collect.** Tax farming already exists. Make the choice
  consequential: a farmed province yields cash now, but unrest rises and its
  efficiency decays, because the farmer squeezes. This is what makes 5.1 *matter*
  rather than merely exist.
  *Chronicle:* the farm sold, to whom, for how many years; the unrest that follows.

*Gate:* `province_land_pass_is_a_noop_without_provinces` must still hold; a save
with no cadastre keeps today's efficiency exactly.

---

## 6. Deliberately not in this plan

- **B-D · Play the Bank.** Rejected by decision 1, not deferred.
- **A wage bill and a labour market.** `FIX_PLAN` Part C ("growth is exogenous"),
  not a finance or institutions change. Named so it is not assumed delivered.
- **Interbank lending and contagion graphs** (`SYSTEMS_21_PROPOSALS` B1). The
  crash mechanism is already too active; adding exposure edges before phase 1.1
  would make it worse.
- **G-C · The craft quarter** (guilds as sited populations with apprenticeships).
  Only worth it if guilds become a live population, which is itself not in this
  plan — decision 4 scoped Murano as one feature, not a guild rework.
- **A-C · Officials as people.** The richest bureaucracy variant and the one with
  no forcing reason yet.
- **W-B/W-D · Commerce raiding and the prize.** Both want a `Vessel` to be a real
  thing (`MERCHANT_VESSELS_AND_INFORMATION_PLAN` stage 1), which is a different
  plan's prerequisite.
- **S-D · The staple right.** Belongs to phase 4's exclusion family but is a
  city-level power rather than a war or league one; it needs its own design pass
  and would land after 4.4.
