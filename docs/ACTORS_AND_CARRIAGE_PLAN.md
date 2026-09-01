# Actors & Carriage — who moves the cargo, and who may refuse

*An audit of every actor the campaign has, a measurement of who actually carries
the world's trade, and eight proposals with gates. The organising finding is that
the campaign is a very good model of **accumulation** and a thin one of
**authority**: its actors compete by out-earning each other, while the verbs that
made every real trading institution rich — refusing, excluding, compelling — are
either absent or wired to a rounding error.*

**Status: measured and planned. One diagnostic built (`econ_measure_carrier_mix`);
no proposal implemented.** The measurement is the load-bearing part of this
document — it invalidated the first version of its own keystone proposal, which is
recorded in §5.1 rather than quietly corrected.

Read `FIX_PLAN.md` for the wider prioritisation and `SCOREBOARD.md` for what is
measured. This document is the campaign-side counterpart to
`TRADE_AND_MARKET_REVIEW.md` and supersedes nothing; it explains *why* that
review's F2 (no price/distance gradient) has resisted every fix aimed at freight.

---

## 1. What was measured

`dispatch` decides a shipment from the arbitrage gap alone, then **attaches** a
carrier: the seller's house, else the buyer's house, else `owner = -1` —
"independent local merchants & guilds". `diag_by_house` / `diag_by_guild` have
existed for a long time and nothing had ever aggregated them over a run.

`econ_measure_carrier_mix` (`economy_validation.rs`, `#[ignore]`d, ~90 s) does.
60 years, two worlds:

| | reference | large |
|---|---|---|
| shipments | 15,504,097 | 17,018,623 |
| financed by a house | 668,212 · **4.3 %** | 685,535 · **4.0 %** |
| **ownerless residual** | 14,835,885 · **95.7 %** | 16,333,088 · **96.0 %** |
| live houses / with a vessel | 71 / 67 | 81 / 73 |
| total fleet slots | 178 | 972 |

Why the residual took it:

| | reference | large |
|---|---|---|
| no house at either end | 54.6 % | 36.6 % |
| house had **no free vessel** | 41.0 % | 59.2 % |
| house **could not afford** it | **0.1 %** | **0.1 %** |
| house was **barred** | **0.1 %** | **0.0 %** |

### 1.1 The three asymmetries that produce this

The house branch is constrained three ways and the ownerless branch by none:

* a free vessel slot is required and consumed (`cap_sea` / `cap_land`);
* the quantity is clamped by capital — `amount.min(afford)`;
* the cargo can be lost — and the guard is literal:
  `let lost = if owner >= 0 { …roll… } else { false }`. **Ownerless cargo never
  sinks.**

And the transfer itself — `surplus -= amount; stock_take(&mut hubs[a].stock, …)`
— sits **outside** the carrier resolution. The goods move either way. A house's
fleet, capital and exposure to storms therefore govern *who profits*, never *what
moves*.

### 1.2 Two consequences

**House carriage cannot scale.** The large world carries 5.5× the fleet and a
*lower* house share, because arbitrage opportunities grow with hubs × goods while
fleets grow with house wealth. "Give houses more ships" is a treadmill, not a fix.

**Capital is never the constraint (0.1 %), and carrier-level bans touch a rounding
error (0.1 %).** Every embargo, blockade and exclusion built on `house_barred` —
including the ones sketched in `TRADE_STAGING_AND_POSTS_PLAN.md` — redirects
profit, not cargo.

### 1.3 A named hypothesis

The scorecard's grain price/distance gradient reads **−0.064** against an
`ECON_INTEGRATION_FLOOR` of +0.05, and is asserted only `is_finite()` because the
model has never earned it. If 96 % of cargo moves with no capital cost, no
capacity limit and no risk, distance costs only freight. **That is the most
plausible single cause of F2, and it is now testable** — §3.1 is the test.

---

## 2. The roster, and what each actor may do

Eleven things can act. `CLAUDE.md` §5 documents six of them; `seed_craft_guilds`,
`seed_holy_sites`, `seed_trade_fairs`, `run_piracy`, `run_diaspora` and
`update_public_debt` are undocumented there (fixed in the same commit as this
file, per rule 2.7).

| Actor | Type | The short version |
|---|---|---|
| City / polis | `TickHub` | Richest by field count. Taxes, mints, wars, holds provinces. **Cannot forbid anything** except a famine export lock |
| Merchant house | `House` | Wealth, fleet, offices → leases → bailo, kin, goals, crises, provinces |
| Merchant guild | `House{is_guild}` | The *same struct* with a flag. Strictly **fewer** organs than a house |
| Craft guild | `CraftGuild` | Four fields, cap 12, doc-labelled "flavour" |
| Bank | `Bank` | A real balance sheet; the one unambiguously well-modelled actor |
| Realm / crown | `Realm` | Sovereignty, genealogy, taxation, tax farming, vassals |
| Province | `prov_*` | The world↔campaign join; writ held by a city *or* house *or* crown |
| Estate / works | `TickHub{is_estate}` | Production sites with owners, shares, grades |
| Colony / outpost | `TickHub` + backers | **The only actor that can bar anyone from a market** |
| Fair · holy site | `Fair` · `HolySite` | Doc-labelled "flavour": sentiment bumps, one price bump |
| "Local merchants" | `tw_local: f32` | Not an entity. §2.2 |
| The populace | `Society` · `Pop` | Shares and unrest. `Pop` is computed and discarded. One verb: revolt |

### 2.1 The exclusion primitive is built, and has one author

`house_barred: Vec<Vec<u32>>` is read by `dispatch` at `production.rs`, and
`pay_to_regain_markets` even provides a buy-back. It is **authored at exactly two
sites, both in `colonies.rs`, both about a colony** — the colony charter, and a
colonial war of independence. A colony can shut a house out of a market. A city
cannot. A guild cannot. A league does not exist.

### 2.2 "Local merchants" is a name given to distance

`cls = if owner >= 0 { 0 } else if days <= LOCAL_HAUL_DAYS { 1 } else { 2 }`, with
`LOCAL_HAUL_DAYS = 8.0`. An ownerless cargo travelling seven days is "local
merchants"; the identical cargo travelling nine days is "guilds". Two of the three
merchant classes the UI shows are one unconstrained residual, split by a threshold
that governs nothing else. **`SUPPLY_LOCAL` is never written at all** — one of the
five seller classes the City Market view shows is structurally always zero — and
every arrival books as `SUPPLY_FOREIGN` regardless of who carried it.

### 2.3 The market does not consult its actors

* **Demand is perfectly price-inelastic.** `base_need` = population × tier weight ×
  desire × cadence × foreign-craving × society multiplier × scale. No price term.
* **Price is an inventory ratio**, `base·(need/stock)^k`, clamped — not a clearing
  price.
* **Trade is a global scan**: each seller's 3 best reachable neighbours, gravity-
  ordered. The scan is the market maker.
* **`house_for` picks with `.position()`** — the first match by array index — so the
  oldest house at a hub holds permanent first refusal on its city's trade. A silent
  incumbency bias that compounds for 500 years and appears in no design document.

Steps 1–7 of the tick are decided entirely by the algorithm; every actor in the
game acts in step 8. **The market clears without consulting anyone, then the actors
settle up.**

### 2.4 Guilds specialise in nothing and are preferred for everything

`House.spec` is the "goods this house trades" list. A private house is founded with
the top 2 goods its city produces (`houses.rs`). **A guild is founded with
`spec: vec![]` and nothing ever fills it.** And `house_for`'s guild arm is
`h.is_guild && h.hub as usize == hub` — no specialisation check — sitting *above*
the unspecialised private-house arm, so it shadows the two rungs below it at every
city with a guild.

This is not a guild that was designed too strong. The narrowing mechanism was
written and the guild path routes around it.

---

## 3. Proposals

Each carries a gate that is **not** the metric it targets (§2.4 of `CLAUDE.md`).

### 3.1 N1 · Make the local haul bind — *the keystone*

`LOCAL_HAUL_DAYS` already exists and currently only *labels*. Make it bind: the
ownerless residual may carry a haul shorter than the threshold and nothing longer.
A long haul needs a carrier with a real vessel, or it does not sail.

This is the historically correct division and the reason merchant capitalism
existed: local marketing was done out of a basket; long-distance carriage required
capital, vessels and organisation. It hands the house layer an exclusive niche
instead of a 4 % share of a job anyone can do.

The dose knob is the threshold. Start at ∞ (today, bit-identical) and walk down.

*N1b, separately dosed:* let ownerless cargo sink — the `owner >= 0` guard on the
loss roll becomes an unconditional roll at a lower rate.

**Gate.** Threshold ∞ must be bit-identical, proven before any dose. Total annual
trade volume must not fall more than ~15 % — this is the gate that will fail first
and the one that matters. `lack_basic` must not rise materially: food is the one
cargo where throttling carriage kills people. `econ_inheritance_rules_fragment_
differently` holds at ≥1.05×; top-10 % share stays in 0.60–0.90. The win condition
is promoting `integration_gradient` from `is_finite()` to a real assertion against
`ECON_INTEGRATION_FLOOR`, per §2.5's "promote a printed metric as the model earns
it".

### 3.2 N2 · Ban the cargo, not the carrier

Exclusion must bind the **lane and the good**, before the arbitrage decision, not
the carrier after it. The shape exists and is proven: `food_export_lock` is
precomputed once per dispatch and consulted in the seller loop. Generalise it into
a hub-level rule — export ban, import ban, staple right, navigation act — authored
by a council, a company within its charter, or a craft guild within its craft.

Keep `house_barred`, and label it honestly in the UI as what it is: a weapon
against a rival's *profit*, not against a city's supply.

**Gate.** Zero bans ⇒ bit-identical. A banned good's price at the banning city must
move in the predicted direction and its trade must reroute — measured as a change
in that city's share of its component's throughput, not as a wealth delta. Top-10 %
share in band (a staple right is a rent, and rents concentrate). A food ban must
stay bounded by the existing relief machinery.

### 3.3 N3 · The Company: chartered staple, opportunistic venture

Four changes, three of which make it **weaker**:

* **Give it a charter.** Populate `spec` with 2–4 goods from its city's real
  strengths, as a private house's is built.
* **Stop it carrying everything.** Add the missing `spec.contains(&good)` check to
  `house_for`'s guild arm and drop that arm *below* the specialised private-house
  arms. This alone is the "not so strong" fix.
* **Venture trade.** A non-chartered good may be taken only when it is *inbound to
  the company's home hub*, there is vessel capacity left after chartered
  obligations, and it accepts a worse margin. `deploy_return_leg` is already
  buy-abroad-sell-at-home.
* **Give it a monopoly, and a price for it.** Within its chartered goods at its home
  city it may author an N2 ban — but the council revokes the charter if it fails to
  supply the city's need, and its bankruptcy immunity becomes a visible subsidy line
  in `CityFinance`. Today a company is immortal *and* free; it should be immortal
  *because* the city pays for it.

**The rename.** Two institutions share the word "guild". Historically they are
opposites: a *guild* is an association of **producers**; a chartered body of
**traders** is a **company**. Reserve "Guild" for the craftsmen and rename
`House{is_guild}` → **Company** ("the Company of Aquentia") — historically right
(Merchant Adventurers, Casa di San Giorgio) and it sets up a chartered joint-stock
body later without a third word. Alternatives considered: *Consulate*, *Staple*,
*Commune's House*. `guild_name_for` already generates culture-flavoured names, so
this is a type name, a display string and a field rename; no save-format change if
`is_guild` keeps its serde key.

**Gate.** Company share of shipments must **fall** versus today (the point is to
weaken it) while its share *of its chartered goods* rises above a private house's.
Venture shipments must be a minority of company traffic and overwhelmingly
home-inbound — assert the direction, or "opportunistic" quietly becomes
"everything". No city loses its company to the new subsidy cost in the first 50
years; a company failing later is correct and should be chronicled.

### 3.4 N4 · Carrier assignment by competition, not by array index

Replace `house_for`'s `.position()` with a weighted pick over eligible houses
(influence at the hub × free capacity × specialisation match). Matters much more
*after* N1, when carriage becomes scarce and therefore worth competing for; land it
in the same phase.

**Gate.** The correlation between house *founding order* and terminal wealth must
fall — if it doesn't, the bias was not real and the change should be reverted. The
pick must be a `hash01` draw, never an RNG or a `HashMap` order.

### 3.5 N5 · The sailing window

Make `days[]` seasonal from fields phase 3 already computes — `storm_base`,
`snow_frac`, `sst`, and the monsoon reversal `earth_monsoon_wind_reverses` already
asserts. Twelve multipliers per lane in the `EconomySnapshot`; `dispatch` reads the
current month. `day_of_year()` is currently read at exactly one site in the whole
tick.

The only proposal that makes the frozen world↔campaign interface earn its keep
(FIX_PLAN B1). It stacks with N1 rather than competing: N1 makes carriage scarce in
*space*, N5 in *time*.

**Gate.** Total annual volume must not fall > 5 % — a window redistributes voyages,
it does not delete them. Intra-year price volatility must rise at a seasonally-
closed port and not at an all-season one. All-1.0 multipliers ⇒ bit-identical;
`earth_` asserted unchanged.

### 3.6 N6 · Price-elastic demand — *high risk, own track*

Add a price term to `base_need`. Today a dearth is unconditional, substitution
cannot respond to cost, and a price ceiling is unmodellable because the shortage it
would cause is already permanent (`polis.rs` says exactly this).

The most dangerous change here. `COMFORT_IMPORT_FRAC` broke a gate for four commits
at twice its correct dose; this touches the same function more fundamentally. Ship
at elasticity 0, walk up in small steps, expect to stop early.

**Gate.** Elasticity 0 ⇒ bit-identical. Every `econ_` band that currently passes
must still pass — a "no regression anywhere" change, not a "win one metric" change.
Starvation must not rise: if elastic demand makes the poor stop eating, the term is
in the wrong place.

### 3.7 N7 · The League — *expensive*

A voluntary association of hubs that stay independent: a member list, a diet on a
cadence, a common purse, and one collective verb — the boycott, which is an N2
cargo-ban voted by every member at once. Explicitly **not** sovereign: no
provinces, no capital, no succession. That negative definition is why it cannot be
a `Realm`.

Its dependency moved with the measurement: a league whose boycott bans *carriers*
would be theatre; one that closes *lanes* is the Hanse.

**Gate.** New `econ_measure_league_formation` on `realm_reference_world`: leagues
must form **and dissolve** — a monotone member count is a failed build, the same
failure `realm_secession_pass` exists to prevent. A boycotted city's trade must
visibly reroute. Realm formation must not collapse: a league is a rival to a crown,
not a replacement.

### 3.8 N8 · Make the market book honest — *free, do first*

Write `SUPPLY_LOCAL`; attribute arrivals by actual carrier instead of booking every
arrival `SUPPLY_FOREIGN`; and stop splitting the residual into two invented classes
until N1 makes those words mean something — until then it is one class and its
honest name is *the open market*.

**Gate.** `cargo check --lib --tests` + `npx tsc --noEmit`. No sim gate — nothing in
the tick reads these. The five class shares must sum to the hub's actual
throughput; they currently do not.

---

## 4. Build order

```
PHASE 0  N8  market book            free, no sim exposure
PHASE 1  N1  local haul binds       keystone; ship at zero dose, then walk
PHASE 2  N2  cargo bans             needs N1 to bite
         N4  carrier competition    cheap; matters once carriage is scarce
PHASE 3  N3  the Company            needs N2 for the monopoly half
         N7  the League             needs N2 for the boycott
INDEPENDENT   N5 sailing window     no dependency; can run in parallel
LAST, ALONE   N6 elasticity         expect to stop early
```

N1 is both the keystone and the largest blast radius. That tension is resolved by
shipping the mechanism at zero dose — provably bit-identical — and treating the
dose walk as its own multi-commit exercise with `econ_measure_carrier_mix` already
in place.

**One target for the whole programme, stated so it can be checked:** houses should
carry a **minority of shipments and a majority of shipment *value***. That is what
a merchant elite is — few cargoes, high value, long distance — and it is falsifiable
against the instrument that now exists.

### 4.1 How to balance them

1. **Ship at zero dose and prove bit-identity first.** Every change above has a null
   setting. The `suppress_realms` / `suppress_relief` precedent exists for this. A
   change that cannot be turned off cannot be bisected.
2. **Dose, don't redesign.** This codebase's failures are overwhelmingly dose
   failures: `COMFORT_IMPORT_FRAC` at 0.60 inverted a gate and 0.30 restored it with
   a wide margin; halving the war score swing fixed war frequency *and* an unrelated
   inheritance gate. When a gate goes red, halve the dose before touching the design.
3. **Never gate on the metric you are targeting.** N1 targets the gradient, so its
   gates are volume, hunger, inheritance and top-10 % share. A gradient that improves
   while cities starve is a failure a gradient-shaped gate reports as success.
4. **One instrument per change, written before the change.**
5. **Write down the reverts.** If N6's elasticity walk stops at 0.05, that number is
   the deliverable.

---

## 5. Caveats

### 5.1 This document's own wrong turn, kept

The first version of this analysis proposed, as its keystone, giving more actors the
right to write into `house_barred`. The measurement then showed carrier bans touch
**0.1 %** of shipments, because the residual absorbs anything a barred house drops —
which `dispatch`'s own comment already admitted ("the trade falls to a rival or
independent merchants"). The proposal was not too small; it was aimed at the wrong
layer. N2 is the correction. Recorded because a reverted approach that isn't written
down will simply be attempted again (§2.4).

### 5.2 The gate this must be measured against is currently red

`econ_inheritance_rules_fragment_differently` **fails on `main`** at `fe9db2b`,
verified on a clean tree by stashing all local changes: *"partible must leave the
average house poorer than primogeniture (177581 vs 168513)"*. Partible is coming out
**richer** — the same inversion `CLAUDE.md` records from the `COMFORT_IMPORT_FRAC`
episode (193,720 vs 164,858 at the bad dose); last recorded healthy figures were
149,925 vs 174,496. This is the sixth perturbation of a gate that file already
describes as having flipped inside its own noise band five times.

Four of the five outcome columns for these proposals are wealth-sensitive. **No dose
above can be tuned until that gate is green.** The mechanisms hold regardless — they
are structure, not calibration — but the tuning cannot start. Needs its own bisect.

### 5.3 The 96 % may be load-bearing in ways not yet found

15.5 M shipments over 60 years is a great deal of machinery resting on that path.
What it carries and why is measured; not everything downstream that quietly assumes
it is.

---

## 6. Deliberately not proposed

* **Religion / the church.** No substrate — `HolySite` is a pilgrimage-season
  sentiment hook, not a faith. A church as landholder, lender and ban-issuer needs a
  religion system first, which is larger than everything above combined.
* **Armies and navies as units.** The "a city cannot march an army" doctrine is
  coherent and `WAR_MAX_DIST_FRAC` depends on it. Instead: give `run_piracy` an
  **author** — a corsair patronised by a house or city, who can be paid off or
  hunted. Today it is a 35 %/yr world roll that deletes one `fleet_sea` from a random
  house, with no pirate behind it.
* **A chartered joint-stock company as a new type.** `Share` already exists on works,
  N2 supplies the monopoly, N7 the collective body. A company should *emerge* from
  those three. Stated so nobody builds a fourth thing.
* **A landed nobility distinct from a crown.** Rule 24 already lets a house hold a
  province's writ. A fourth authority layer over a model that documents three as its
  limit.
* **Player verbs.** Every `decide_*` remains a latent player verb (FIX_PLAN B2). N2's
  ban would be an unusually good first one — a different document.

---

## 7. Order

| Step | What | Status |
|---|---|---|
| 0 | Audit the roster and capability matrix | **done** (§2) |
| 1 | Measure the carrier mix | **done** — `econ_measure_carrier_mix` |
| 2 | Bisect and fix `econ_inheritance_rules_fragment_differently` | **blocking, not started** (§5.2) |
| 3 | N8 market book | not started |
| 4 | N1 at zero dose, then the dose walk | not started |
| 5 | N2 · N4 | not started |
| 6 | N3 · N7 | not started |
| — | N5 (parallel) · N6 (last, alone) | not started |
