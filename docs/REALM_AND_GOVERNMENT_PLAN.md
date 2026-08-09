# Realms — the first countries

*A merchant family takes a city, proclaims sovereignty, and becomes a dynasty. The
realm it founds holds provinces, taxes them, mints its own coin, conquers its
neighbours, and eventually breaks apart along its own family tree.*

**Status: R1 and R2 built** (entity, proclamation, house→crown transfer,
`compute_states` reading real realms, genealogy/succession/regency — see §7's order
table for exactly what shipped in each). **R3-R5 not yet built.** Every decision in §1
was made explicitly by the maintainer over the design conversation that produced this
document; nothing here is a suggestion looking for approval. §7 lists what is
deliberately NOT in the first build, and that list is as binding as the rest.

This document extends `CITY_PROVINCE_WAR_PLAN.md`, and **reverses one of its
decisions**: §6 of that plan deferred "territorial empires — a `Realm` entity above
cities" behind the city-state, on the grounds that *the city IS the state for now*.
That deferral ends here. `compute_states` — a pure derived read that groups provinces
by their holder city's tier — becomes a read over a real, persisted `Realm`.

Read `FIX_PLAN.md` for the wider prioritisation and `SCOREBOARD.md` for what is
actually measured.

---

## 1. Decisions

### 1.1 What a realm is

| | |
|---|---|
| Founding | ONE city plus the provinces it holds. Never a multi-city realm at birth |
| Trigger | a house holds `captor_house` at its seat and **proclaims**; a mere `council_house` is not enough |
| Floor | **hard floor year 50.** After that, any eligible house may proclaim, at any time |
| Rate | no target count. Growth is organic; more realms over time is the expected shape |
| Identity | **one house = one realm**, permanently. A house that gains a second sovereignty merges it into the realm it already has |
| Territory | the union of provinces whose sovereignty it holds. A realm's border is a province border |

### 1.2 The house becomes the crown

| | |
|---|---|
| Wealth | the house's wealth **becomes the realm treasury**. One pot, not two |
| Trade | the crown inherits estates, fleets, banks, offices, bailos and monopolies, and operates them. A realm is a **trading crown** |
| The house | ceases to be a merchant house — removed from the merchant tier ladder, from `HousesPanel`, and from every merchant-house pass |
| What survives | name · arms · `line` · `origin_house` lineage · kin. The house record becomes the **dynasty shell** |
| Rank | realms rank on their own ladder: city-state → kingdom → great power → hegemon |

### 1.3 Government and succession

| | |
|---|---|
| Succession | a real **genealogy**: persons with birth and death dates, parents, spouses, children |
| Heir | the eldest eligible child, by the culture's own `LineRule` (§5.3) |
| Minority | an heir under 16 reigns under a **regency** — legitimacy penalty, elevated plot risk |
| Autonomy | a realm-level policy: **centralized · core-and-periphery · autonomous** (§3.4). Decides what an annexed city keeps |
| Taxes | several historical kinds, each with its own base, unrest slope, and **collection efficiency** (§3.3) |
| Coin | a realm mints its own coin. Member coins fold or coexist per the autonomy policy |
| Capital | movable, and **need not be the largest city** — the Karakorum rule |

### 1.4 Expansion and war

| | |
|---|---|
| Targets | free cities, and other realms |
| Realm war | declaring on a realm brings **every member city into a defensive war** — one war, one score, many cities |
| Separate peace | a member city **may quit** a realm's war (§3.6) |
| Free cities | may be hired as allies or coerced into a realm's war |
| Two wars | allowed only if force and treasury reserves permit; while in two, exhaustion accrues at a steep convex multiplier |
| Vassals | fight in their overlord's wars automatically |
| Overseas | non-contiguous territory is legal **only where the realm's own merchants are present** (§3.5) |

### 1.5 War goals

Extends `CITY_PROVINCE_WAR_PLAN.md` §1.4's priced ladder. **Below annexation, the
losing city always keeps its council, its market and its ability to recover** — a war
that erases a player-legible entity erases the chronicle.

| Goal | Price | Winner takes | Loser keeps |
|---|---|---|---|
| Plunder / reparations | 10 | goods paid over years | everything else |
| **Humiliate** *(new)* | 15 | prestige + legitimacy transfer | all land, all money |
| Trade rights | 25 | a bailo | sovereignty, council |
| **Enthrone** *(new)* | 35 | winner's kin seated in the loser's government (`Official.kin`) | independence on paper, own coin, own trade |
| Tribute | 40 | yearly payment, fixed term | council, walls, autonomy |
| **Vassalize** *(new)* | 50 | tribute + follows in war + may not declare its own | council, market, coin, internal law |
| Province | 55 | one ordinary province | the city itself |
| Annex | 90 | full sovereignty over city + its provinces | the dynasty survives, dispossessed |

---

## 2. Data model

```rust
struct Realm {
    id: u32,
    name: String, title: String,          // "Ashuran", "Lugalate"
    capital_hub: u32,
    ruling_house: u32,                    // 1:1 with House, permanently
    rank: u8,                             // 0 city-state · 1 kingdom · 2 great power · 3 hegemon
    autonomy: u8,                         // 0 centralized · 1 core-and-periphery · 2 autonomous
    provinces: Vec<u32>, cities: Vec<u32>, vassals: Vec<u32>,
    treasury: f32, debts: f32,
    legitimacy: f32, cohesion: f32,
    tax_rates: [f32; TAX_KINDS],
    coin: i32,                            // index into the existing coinage layer, -1 = none
    family: Vec<Person>, ruler: u32, regent: i32,
    founded_tick: u64,
    events: Vec<RealmEvent>,
}

struct Person {
    id: u32, name: String, sex: u8,
    born: u64, died: u64,                 // ticks; died == 0 → alive
    father: i32, mother: i32, spouse: i32,
    legitimate: bool,
    axes: [i8; 4], skill: f32,            // reuse Kin's character axes
    epithet: String,                      // set at death, as House.line already does
    reigned: Option<(u64, u64)>,
}

// on CampaignSim — every field serde-defaulted; empty ⇒ bit-identical
realms: Vec<Realm>,
prov_realm: Vec<i32>,                     // -1 = free land

// on House
crowned: bool,   realm: i32,              // NOT `defunct` — see §5.1

// on TickHub
realm: i32, realm_role: u8, integrated_at: u64,
// realm_role: 0 seat · 1 subject · 2 tributary · 3 occupied
```

### 2.1 Three layers of authority

Sovereignty is added **above** the existing stack, so nothing existing is rewritten:

```
 SOVEREIGN   prov_realm / hub.realm        who is obeyed — levies, war, taxes
 ADMIN       prov_holder                   who administers — council, market, granary
 DUES        prov_holder_house             who is paid — the Stato da Mar (rule 24)
```

A house-held province inside a realm's borders stays legal (`CITY_PROVINCE_WAR_PLAN.md`
§5.9). This needs a **rule 25** in CLAUDE.md: *sovereignty is never assumed to exist* —
`prov_realm == -1` is the pre-state default and must remain legal forever, because that
is what every world looks like in year 1 and what most land looks like at year 500.

---

## 3. The systems

### 3.1 Proclamation

`update_government` already runs the full capture loop — officials with terms and
roles, houses bribing (cash) or intimidating (prestige + ships), and `captor_house` set
when a house holds >50% of control-weighted seats. **The takeover is built. Only the
proclamation is missing.**

```
year >= 50                                  (hard floor)
AND captor_house held continuously >= 10 years
AND house tier <= 2  AND city tier <= 2
AND the city holds >= 1 province writ (prov_holder)
AND treasury and prestige above a floor
AND not a tributary of another realm
AND a per-year roll biased by the head's boldness/expansiveness axes
```

The tier gates apply **only at proclamation**. After founding, a realm may keep a small
capital indefinitely (§1.3, the Karakorum rule).

The **first realm in a world is a world event** — an unmissable journal entry. It is the
moment the campaign changes genre and the chronicle should say so.

**Indirect rule stays a live alternative.** A house may seat kin as officials
(`Official.kin`, already locked at control 1.0 and unbribable) and rule without a crown:
cheap, reversible, no legitimacy cost, **and the city stays grey on the map**. That is
the Medici case against the Sforza case, and it is why some cities never become realms
even when a house could take one.

### 3.2 House → crown transfer

Not a dissolution. See §5.1 for why this must never route through `dissolve_house`.

```
promote_house_to_realm(hi):
    realm.treasury   <- house.wealth              // the pot moves whole
    realm.debts      <- outstanding bank loans    // sovereign default becomes possible
    realm.estates/fleets/banks/offices/bailos/monopolies <- the house's
    house.crowned = true                          // NOT house.defunct
    house keeps  name · arms · line · lineage · kin      -> the dynasty shell
    house leaves merchant tiers · merchant goals · HousesPanel
```

Every merchant-house iteration gains `&& !h.crowned`. Everything that reads *identity*
(arms, `origin_house` chains, the chronicle, the Lineage tab) is untouched.

Kin who do not inherit become realm governors; a share of them found **cadet merchant
houses**, which is the designed counter-pressure to §5.2's hollowing-out risk.

The crowned house's feuds carry over as realm rivalries — free casus belli, and it
means a coronation does not erase a century of grudges.

### 3.3 Taxation — collection, not rates, is the constraint

The historical fact this system exists to model: **pre-modern states were not limited by
what they charged, but by what they could collect.**

```
collected = base × rate × efficiency(cohesion, distance, integration) − evasion(unrest)
```

| Tax | Base | Falls on | Unrest | Anchor |
|---|---|---|---|---|
| **Harvest tithe** | `prov_surplus` (exists) | countryside | med | the universal land tax |
| **Hearth / poll tax** | heads, rural + urban | everyone, flat | **high** | English poll tax → 1381 |
| **Customs / tariff** | trade at member ports (exists) | merchants | low | every port city |
| **Vassal tribute** | vassal treasury (`tribute_to` exists) | vassal city | vassal's own | hegemony before empire |
| **Tax farming** | sell N years of collection to a HOUSE for cash now | the crown's future | med | *publicani*, *iltizam* |

**Tax farming is in the first pass on purpose.** Crowns drain the merchant pool; tax
farming pours crown money back into it and keeps houses politically entangled with
realms instead of replaced by them.

The **fiscal mix becomes the realm's character** with no separate "government form"
system: tithe + corvée reads as a temple state, customs + staple as a merchant realm,
tribute + poll tax as a conquest kingdom. The form emerges from the choices.

Rates are **AI-set** (`decide_realm_taxes`, in the existing `decide_*` family). The
province tax slider remains the player's verb.

**A crown that cannot tax debases.** Realm coinage (§1.3) closes the loop into the built
monetary layer: tax capacity → debasement → `coin_trust` → prices. Every piece of that
chain already exists.

### 3.4 The autonomy axis

One policy that ties annexation, cohesion, separate peace and revenue together.

| Policy | Annexed city keeps | Revenue | Cohesion | Separate-peace risk |
|---|---|---|---|---|
| **Centralized** | nothing — crown coin, crown market, governor for council | high | low at distance | low |
| **Core & periphery** | core folds; distant cities keep council + coin | med | med | med |
| **Autonomous** | coin, market, council, own tariffs; crown takes tribute + a customs share | low | **high, distance-insensitive** | high |

Sticky and costly to change — centralisation is a reform with an unrest spike, not a
toggle. **AI realms default to core-and-periphery**, because a fully centralised world
by year 400 is a world in which the settlement panel means nothing.

### 3.5 Overseas territory is merchant-gated

A realm may hold a non-contiguous province only where its own trade presence is real:

```
requires ALL of:  a crown office / bailo / estate in a city of that province
                  a sustained trade route from the realm to it
                  naval reach

link severed (blockade · rival monopoly · fleet lost · route collapse)
    → cohesion decays yearly → tribute stops → the province breaks away
```

This makes the trade network the literal skeleton of overseas empire (the Stato da Mar,
the *Estado da Índia*), gives the crown's inherited fleet an ongoing job, and provides a
**non-military way to lose territory** — a naval defeat can cost an empire without a
single province changing hands at the peace table.

### 3.6 War at realm scale

One war, one bidirectional score, many cities. Force pools aggregate levies
(`prov_rural` — levy dead are people dead), fleets, and crown + member-city chests.
Reach still gates everything (`hubs_within_war_reach`); distant members contribute money
rather than levies.

**Separate peace** — a member city quits when autonomy is high, cohesion low, its own
damage high, and it was not the aggressor. The crown loses legitimacy; the city may
defect outright. This is what gives the autonomy axis teeth on both sides.

**Two wars** are permitted but punishing: allowed only above a force/treasury reserve
threshold, with exhaustion and weariness accruing at a steep convex multiplier — the
same convex-cost principle `CITY_PROVINCE_WAR_PLAN.md` §1.4 already sets for force.

A major war deserves a **name** in the chronicle and an entry in both realms' histories.
For an observation-only game the naming is the feature.

Rule 22 applies unchanged: **it must always terminate.**

### 3.7 Genealogy and succession

Today `Kin` is a snapshot regenerated at each succession — no birth dates, no parents,
no persistence. Realms need persistent people.

Yearly life pass, deterministic on `hash(seed, tick, person_id)`:

```
marriage     ruler and adult heirs marry at ~18-25
births       fertility by the wife's age; roughly one birth per two fertile years
child death  ~25% before age 5   <- the engine of contested succession
aging        childhood -> adult at 16
adult death  age-dependent hazard + war + plague (plague_house_toll exists)
succession   on the ruler's death, by the culture's LineRule
```

```
ruler dies
  ├─ living legitimate issue? ─ yes ─► eldest ELIGIBLE by LineRule
  │                                      ├─ age >= 16 ─► crowned
  │                                      └─ age <  16 ─► REGENCY
  │                                            legitimacy −, plot risk +
  ├─ no issue → siblings → nephews → a cadet branch
  └─ nothing → dynasty ends → the crown passes to the strongest house at the
                              capital, else the realm dissolves to free cities
```

### 3.8 Fragmentation — entirely dynastic

No proclamations are possible inside a realm (§1.1), so a realm can only break along its
own family. Two paths:

**Path A — Partible division.** A culture whose `InheritanceRule` is `Partible` divides
the realm among eligible sons at **every** succession. Merovingian and Carolingian
fragmentation, for free.

**Path B — Contested succession.** Two claimants with real backing (a minor heir under
regency against an adult brother governing a second city) fight a civil war, reusing
`crisis.rs`'s quarterly rounds at realm scale. Each holds the cities where their support
is real — governed by their own kin, near their seat, backed by houses. One wins and
reunites, or neither does and there are two realms sharing one dynasty.

**This is the most important structural claim in the plan**: the culture's inheritance
law, already shipped and already gated by
`econ_inheritance_rules_fragment_differently`, now decides whether a people can build a
lasting empire at all. An Agnatic-Primogeniture people accumulates; a Partible people
fragments every generation and never holds one. Emergent, not scripted, and nearly free.

---

## 4. Presentation

### 4.1 Map

```
○   grey/black            free city, no house grip
○ᶜ  grey + heraldic pip   free city, captured internally (indirect rule)
●   realm colour          subject city inside a realm
★   realm colour + crown  realm capital
◍   own fill + ring       vassal / tributary
▨   hatched realm colour  taken within N years, not yet integrated
```

The area layer (province-raster fill, border traced on province edges — already built as
of `dacffec`) and the settlement dots share **one palette**. Heraldry never tints the
dot; it stays in panels and on the crown glyph.

### 4.2 Realms panel

A list plus a large detail window — larger than the House dossier, because a realm holds
many cities.

```
[Realm] [Cities] [Provinces] [Dynasty] [Genealogy] [Taxes] [Trade] [Wars] [Economy] [Chronicle]
```

Left column: a **province minimap** tinted in the realm's colour with the capital marked
and unintegrated provinces hatched; legitimacy and cohesion as pips-and-a-phrase in the
`HouseDossier` idiom (quiet when healthy); crown treasury, debts, levies available.

`Dynasty` continues `House.line` unbroken through the coronation. `Genealogy` renders the
family tree as SVG (the approach `GoodsChainReview`'s recipe DAG already uses), clickable
to a person card: birth, death, character phrase, epithet, whether they reigned.

### 4.3 Settlement Government tab

Two panes above the existing content:

* **Sovereignty** — for a free city, *"{City} answers to no one"* plus a tension meter
  naming the house closest to capture (the officials tally already computes it) and,
  when conditions approach, *"{House} could proclaim a realm here."* Foreshadowing is
  most of the drama. For a member city: realm arms, role, *since year N*, integration
  bar, tax remitted upward, levies owed.
* **Controller** — the sovereign house and its head (reuse `CityLeader`), the dynasty
  line, and who actually runs the city day to day: a posted kinsman, a crown governor,
  or a local council left in place.

The officials list gains one column: **kin** (locked) versus bought (control bar), which
is what makes indirect rule visible.

---

## 5. Caveats — read before building

### 5.1 A coronation must never route through `dissolve_house`

"The house defuncts into the realm" is the right design and the wrong implementation if
taken literally. `dissolve_house` is a **liquidation**: it writes off outstanding bank
loans as `Bank.losses`, releases held provinces, strips holdings and chronicles ruin. Run
on a coronation, a family would celebrate its crowning by defaulting on its debts and
losing its territory.

Worse, `defunct` is read by roughly forty paths, one of which is `GOAL_OUTLAST_RIVAL` —
*"a named rival goes defunct while this house lives."* Setting `defunct` on a coronation
makes **every rival holding that goal instantly win**: a family becomes a king and its
enemies celebrate having outlived it.

Hence `House.crowned`, a distinct flag, and `promote_house_to_realm` (§3.2), a transfer.

### 5.2 Crowns drain the merchant pool

Houses *are* the economy — estates, fleets, banks, contracts, monopolies, warehouses.
Every successful house that becomes a crown leaves the merchant pool, so the top of the
wealth distribution keeps emigrating. `top-10% wealth share` was moved **into** its
historical band (0.497 → 0.651) by the Phase 5 province work and is the metric most
likely to break here.

Two designed counter-pressures, both in this plan: cadet merchant houses founded by
non-inheriting kin (§3.2), and tax farming returning crown money to houses (§3.3).
Whether they suffice is a **measurement**, not an argument. R3's gate exists for this.

### 5.3 `LineRule` must survive contact with "the eldest son"

The decision is that the eldest male child inherits. The codebase carries `LineRule`
(Agnatic · AgnaticCognatic · Absolute · Enatic) per culture, a matrilineal minority of
peoples, **rule 23** (*any forced succession must obey the culture's LineRule*), and a
gate that already caught a man taking a matrilineal house's seat.

Resolution: **route the pick through `LineRule`.** Agnatic is the majority law, so
eldest-son is what will in fact be observed nearly everywhere; an Enatic people crowns an
eldest daughter. This costs nothing, preserves rule 23, and is required anyway by §3.8,
which makes `InheritanceRule` load-bearing at realm scale.

### 5.4 Bit-identity ends at R3, not before

R1 adds a `Realm` entity nothing in the tick reads (only `compute_states` changes its
source), so the dynamics test stays bit-identical exactly as house tiers and city tiers
did. R2 adds genealogy, which changes succession — the first real divergence. R3 rewrites
tax flow and adds a currency. **Do not claim bit-identity after R1.**

### 5.5 Three failure sinks now compound

`CITY_PROVINCE_WAR_PLAN.md` §5.7 already warned that urban-graveyard mortality, plague
and levy casualties compound. Realms add a fourth pressure — poll taxes and corvée on the
same rural pool that supplies levies. Check the combined effect on population and
urbanisation, never each in isolation.

### 5.6 Realms must be able to fall

The gate that matters most is not "do realms form" — it is **"do realms end."** A realm
count that only ever rises means §3.8 is not firing and the world converges on one
colour. R5's gate asserts the count is non-monotonic over a long run.

### 5.7 Rule 24 grows a third case

`prov_holder` (seat) · `prov_holder_house` (dues) · `prov_realm` (sovereignty). Every
existing reader must tolerate all three, and a house-held province **inside** a realm's
borders is legal. Add rule 25 (§2.1) in the same commit as R1.

---

## 6. Deliberately not built (first pass)

Stated rather than silently dropped:

- **Subject-city revolt.** Set aside by explicit decision — fragmentation is dynastic
  (§3.8). A subject city does not rise on its own.
- **Proclamation inside a realm.** No secession by a house holding a non-capital city.
- **Cross-realm marriage**, personal unions, inherited claims. The genealogy is built to
  support it; the diplomacy is not in this pass.
- **Bastards and pretenders.** Legitimate issue only.
- **The *liberate* war goal.** Designed in §1.5's spirit, deferred — it needs third-party
  war participation that does not exist yet.
- **Corvée · staple right · requisition · regalian monopoly · sale of offices.** Designed,
  deferred behind the five first-pass taxes.
- **Sieges, fronts, army movement.** War stays abstract, per `CITY_PROVINCE_WAR_PLAN.md`
  §6. Unchanged here.
- **Leagues, treaties, diplomacy** (FIX_PLAN B4). Still the rung after this one.
- **Land state persisted back to tiles.** Still FIX_PLAN B1's open item (b).

---

## 7. Order

```
R1 ✅ Realm entity · prov_realm · proclamation · house→crown transfer
     compute_states reads realms · colour layers (StatesPanel relabelled → Realms)
     rule 25 added (CLAUDE.md §10). Landed as R1a (schema, dormant) then R1b
     (the proclamation trigger + transfer) then the compute_states rewire.
R2 ✅ Genealogy · births · child mortality · aging · succession · regency
     LineRule honoured (rule 23) · a flat read-only family list in the Realms
     panel (campaign_get_realm_family) — NOT yet the SVG tree §4.2 sketches
R3   Taxes + collection efficiency + tax farming + realm coin
R4   Annex · vassalize · enthrone · realm-vs-realm war · separate peace
     · free-city participation · the two-war penalty
R5   Autonomy policy · overseas merchant-gated holdings · capital moves
     · fragmentation, both paths
```

**Two guard fixes R2 needed that weren't anticipated in R1's own caveats:** building
the realm's own succession surfaced that a crowned house could still be pulled back
into the MERCHANT succession/crisis machinery through two further paths —
`succeed_house` via a stale `head_lifespan` countdown left over from before the
coronation, and `update_house_crises` opening an ordinary discontent crisis on a
house whose wealth is now permanently zero. Both would rewrite `head_name`/`kin` out
from under the realm's own genealogy — the same identity-corruption trap §5.1 names
for `dissolve_house` and `GOAL_OUTLAST_RIVAL`, found and closed via the same
`House::is_merchant()` guard, a third and fourth path into a risk the plan had only
named two instances of. Recorded here because it is exactly the shape §5.1 warns
about repeating, and because whoever builds R3-R5 should expect more of the same
each time a new realm-facing pass is added — audit every house-iteration loop it
touches for the guard, not just the one it's adding.

Gates on every step: `cargo test --lib econ_` · `simulate_decades_reports_dynamics` ·
`cargo check` · `npx tsc --noEmit`. The Earth gate is not needed — nothing here touches
`step3_ocean_atmo/` or `step4_climate/`.

Additional per-phase gates:

| Phase | Gate beyond the standard four |
|---|---|
| R1 | dynamics test **bit-identical** — nothing in the tick reads a realm yet. Confirmed by grep, not assumed: the only non-comment reader of any new field in R1a was a `resize()` nothing downstream consumed |
| R2 | direct unit tests on `resolve_realm_succession` (eldest-eligible-by-`LineRule`, regency, dynasty extinction) rather than `every_realm_succeeds_or_ends` as a single named gate — the mechanism is deterministic enough to test directly; dynamics run stays bit-identical (that sim carries no province layer, so no realm can ever be founded there) |
| R3 | `top-10% wealth share` and urbanisation measured against §5.2; a printed finding is a finding, not a build failure |
| R4 | war frequency against the 45/century baseline; every realm war terminates within the round cap |
| R5 | realm count is **non-monotonic** over 500 years (§5.6) |

**R5 is deliberately last.** Fragmentation is the item most likely to disturb every
measured number at once, and placing it after everything else means that when a number
moves, it is clear what moved it — the same reasoning that put sack-and-purge last in
`CITY_PROVINCE_WAR_PLAN.md`.

Append a `SCOREBOARD.md` row whenever a measured number moves. Never edit an old row.
