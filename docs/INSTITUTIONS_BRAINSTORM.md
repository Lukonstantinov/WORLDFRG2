# Institutions: banks, war, guilds, bureaucracy, signature cities, leagues

> **BRAINSTORM. NOTHING HERE IS APPROVED OR BUILT.** Every "measured" claim was
> read out of the tree or off the standing dynamics run. Every variant is costed
> against machinery that already exists, because in this codebase the cheap
> changes are the ones that reuse a shape (`decide_*`/`apply_*`, a dosed constant,
> a `dispatch` prohibition, a `ProvWork` kind) rather than invent one. §8 is the
> open questions. Companion: `docs/BANKS_MONEY_AND_CRAFT_PLAN.md`, which measures
> the finance layer this file builds on.

---

## 0. The one thing all six have in common

**Nothing in this world can forbid anything.**

`ACTORS_AND_CARRIAGE_PLAN.md` §2.1 already found the exclusion primitive —
`house_barred`, read by `dispatch`, with a buy-back — and found it **authored at
exactly two sites, both in `colonies.rs`, both about a colony.** This review adds
the rest of the picture. `dispatch` honours exactly five prohibitions:

| prohibition | author | live? |
|---|---|---|
| plague quarantine | `disease.rs` | yes |
| famine food-export lock | `polis.rs` crisis relief | yes |
| `export_ban_until` (a hub × good) | `decide_trade_bans` | **`N2_BAN_PRICE_RATIO = INFINITY`** — never fires |
| league boycott (a lane) | `run_league_diet` | **`LEAGUE_BOYCOTT_MAX = 0`** — never fires |
| `house_barred` | a colony charter, a colonial war | yes, ~0.1% of trade |

**War is not on that list.** Neither is a guild, a league, a crown, a city
council, or a bank. So: two cities at war trade with each other at full volume
every day; a craft guild cannot stop an outsider making its good; a league
cannot make membership worth anything; a crown cannot close a port.

Each of the six topics below is a different answer to *who may exclude whom from
what*, and that is why they share machinery and should be sequenced together
rather than as six projects.

---

## 1. Banks — can a bank be a legitimate player?

Two readings of "player", both worth answering.

### 1a. A legitimate ACTOR

`BANKS_MONEY_AND_CRAFT_PLAN.md` measures this in full. Short version: the balance
sheet is genuinely good, and it is connected to almost nothing. Nobody borrows
(`bank_maybe_lend` rolls a die and pushes cash at the **richest** resident, with no
project attached); no bank may hold public debt though 157,677 of it exists across
19 cities; bills-of-exchange income accrues at 5,047 and **no merchant pays it**;
and every `fail_bank` calls `trigger_regional_crash`, so 12 crashes in 50 years
hammer coin trust to 43% — below `RESERVE_TRUST_MIN` (0.55), which closes
`coin_discount`, which is money's *only* channel into trade.

### 1b. A legitimate PLAYABLE role — and this is the interesting answer

**A bank is the best-fitting player role this codebase could offer**, better than
a house or a crown, for four reasons that are properties of the code and not of
taste:

1. **Its verbs are few and consequential.** Lend or refuse · at what rate · to
   whom · take a stake · hold a bond · open a branch · call it in. That is about
   seven decisions a year. `campaign_advance` plus three province verbs is the
   entire current player surface (§5.1); a bank fits that scale exactly, where a
   house (fleets, offices, feuds, kin, goals, crises, provinces) does not.
2. **The decide/apply split already exists for it.** `decide_coinage`/
   `apply_coinage` is the shipped convention and FIX_PLAN B2's whole thesis is
   that every `decide_*` is a latent player verb. `bank_maybe_lend` is one
   refactor from being a queue of applications a human approves.
3. **A bank reads the whole world and moves none of it directly.** That suits an
   observation-first game: you never command a fleet, you decide who gets to.
4. **It is the Medici fantasy**, and it is the one role where losing is
   interesting rather than merely bad — a bank that lends to the wrong prince
   fails, and the failure is already modelled.

**Variants.**

- **B-A · The Sound Bank** *(smallest, and a prerequisite for all the rest)*
  Split failure from contagion (wound down / absorbed by a rival / collapse), and
  let a bank hold the Monte. A bank stops dying every eight years, which is what
  every later idea needs; coin trust recovers, which reopens `coin_discount`.
  *Cheap. Gate: coin trust must stop collapsing below 0.55; wealth stays bounded.*

- **B-B · The Merchant Bank** *(medium)*
  Borrower-side demand (a house that fails an affordability check it wanted to
  pass applies for credit — `diag_why_cash` already counts these), risk priced off
  the borrower's own record, and A6: a loan secured on a works converts to an
  offtake share on arrears. That is the Fugger model, and `offtake.rs` and the
  `Share` table are already built and currently reachable only through an inert
  envoy path.
  *Gate: `econ_inheritance_rules_fragment_differently` per dose step.*

- **B-C · The Civic Bank** *(medium, and a genuinely different actor)*
  The Taula de Canvi (Barcelona 1401), the Banco di San Giorgio (1407), the Bank
  of Amsterdam (1609) were **public** banks: chartered by the council, holding the
  city's deposits and administering its debt. In this codebase that is a `Bank`
  whose `house` is the council rather than a family — which changes who the
  profits go to, gives a city a reason to want one, and makes a civic default a
  political event rather than a private bankruptcy. Cheap because `Bank` already
  has an owner index; it needs a second owner KIND.

- **B-D · Play the Bank** *(the player role)*
  You hold one bank. A loan queue you approve or refuse; a house rate you set; a
  stake you take or decline; branches you open. Everything else in the world stays
  AI. Build B-A and B-B first — a player role on top of a bank that dies every
  eight years and lends to nobody in particular is not a game, it is a form.

**Recommendation:** B-A → B-B → B-C, and only then decide about B-D.

---

## 2. War — where is the real impact on trade?

**Measured: there isn't one.** `dispatch` never reads `self.wars`. What a war
actually does economically is `war_damage_pass` (writes into the existing
`TickHub.damage`, which the funded-repair pass already handles) and
`WAR_BLOCKADE_EXPORT_MULT = 0.55` applied to `export_earn` — an accounting
accumulator that drives `trade_wealth`. `trade_wealth` has real consumers (city
growth, structures, colony siting, city tiers, the crown's customs levy), so the
blockade is a real hit to a city's **prosperity**. It does not stop, delay,
reroute or endanger a single shipment. The enemy's grain arrives on schedule.

**Variants, cheapest first.**

- **W-A · The blockade routes, it does not refuse** *(the one I would build)*
  Add war to `dispatch`'s prohibition list — but as a **staging rule, not a
  refusal**. A lane between two belligerents is not cancelled; it is routed
  through a NEUTRAL port at extra cost, using `staging_hop`, which already exists
  and is already dosed live for N1/N1c.
  This is right on two independent grounds. Historically, the neutral entrepôt in
  wartime *is* the mechanism — Baltic grain kept reaching Amsterdam through
  neutral bottoms, and the whole business of a neutral flag exists because trade
  reroutes rather than stops. Mechanically, this codebase has already proved twice
  (N1c, N2) that a bare refusal collapses the economy while routing at the
  identical dose leaves it healthy — and N2's own trial broke the hard wealth bound
  because **an export-locked market's rent concentrates harder than anyone
  anticipated.** A war blockade is N2 with a bigger trigger; it must not be built
  as a refusal.
  *Dose from zero. Gate: the multi-seed inheritance gate + long-haul volume, plus
  the sustained-richest bound, per dose step.*

- **W-B · Commerce raiding — the lane becomes dangerous, not closed** *(medium)*
  `SEA_LOSS`/`CARAVAN_LOSS` already roll per shipment. A war raises loss on lanes
  touching a belligerent. This is the first mechanism that would give a house's
  **fleet** a reason to exist beyond a slot counter, and it pairs with the *muda*:
  a house or a league pays for CONVOY and buys the risk back down. Note it is a
  softer version of W-A and can be dosed independently — losing 8% of cargo is
  economically similar to a detour but reads completely differently in the
  chronicle.

- **W-C · Contraband** *(cheap, and very characteristic)*
  Not all trade stops in a war — *specific goods* do. Ban the war list to the
  enemy (metalware, iron, timber, and the naval stores `pitch`/`hemp` that
  `YARDS_VESSELS_AND_DEPOTS_PLAN` already shipped) and leave everything else
  flowing. A per-good war ban is `export_ban_until`'s exact shape, already wired
  into `dispatch`, currently at `INFINITY`. It is also a much safer first dose
  than a blanket block, because it touches a handful of goods rather than a
  market.

- **W-D · The prize** *(medium)*
  Captured cargo becomes the captor's, rather than vanishing. This also fixes
  `run_piracy`, which today deletes one `fleet_sea` from a random house with **no
  pirate, no patron and nobody to pay off** (§5's own words).

- **W-E · War is paid for by borrowing** *(the historically central one)*
  Today a war is paid from treasury plus forced house levies. Historically wars
  were paid by **debt**, and that is precisely what created the Monte and made
  banks systemically important. Every piece exists: `update_public_debt` issues
  against throughput, `DEBT_DEFAULT_RATIO` haircuts holders, `fail_bank` cascades.
  Wire war spending to debt issuance and let a bank hold the bond (bank slice B-A),
  and you get the real chain for free:
  **war → debt spike → a bank holds it → the war is lost → haircut → the bank fails
  → the crash.** That is the Bardi and Peruzzi in 1345, and it is currently four
  disconnected systems that each work.

**Recommendation:** W-C (contraband) as the safe first dose, W-A (routed blockade)
as the real one, W-E as the highest-value linkage. W-B/W-D once vessels are real.

---

## 3. Glass guilds — the craft as a defensible secret

**Measured.** `glassware` and `ceramics` both exist and are real manufactured
goods (converted from belts to recipes: glassware ← `bay_salt` + `timber`,
ceramics ← `clay` + `timber`), so this is not a missing-good problem. A
`CraftGuild` exists, and **industrial espionage already exists**
(`maybe_steal_quality`: a house running a manufactory steals the world leader's
technique, recorded as `stolen_good`/`stolen_from` and journaled).

So the *theft* half of the Murano story is built and the *secrecy* half is not.
There is nothing a guild can do to protect a craft, and nothing that makes losing
one a story.

**The historical case, precisely.** Venice moved the furnaces to Murano in 1291;
glassmakers were privileged and forbidden to leave; *cristallo* was a state
secret. The monopoly broke exactly one way — **masters left**, and *façon de
Venise* glass appeared in Antwerp, Nevers and London.

**Variants.**

- **G-A · The Secret** *(smallest)*
  A guild with a hall and a monopoly charter accrues `secrecy`, which resists
  `maybe_steal_quality`. Espionage gains its counterparty. Two fields, one roll.

- **G-B · The master leaves** *(medium — and this is the mechanism that matters)*
  A rival city bribes a master away, and the technique goes with him: the target
  city's quality jumps toward the source's, the source's guild loses standing, and
  it is chronicled on both sides. `STEWARD_POACH_CHANCE` already implements
  poaching for offices — reuse the shape. Secrecy resists; a rich rival's treasury
  beats it.
  This is the one that produces **a history of a craft** rather than a static
  attribute: invented somewhere, guarded, eventually diffused. Right now no trade
  good in the world has a history.

- **G-C · The craft quarter** *(full)*
  A guild becomes a sited thing with masters, apprentices and a rebuild time, so a
  sack or a plague can destroy a craft for a generation. Expensive; only worth it
  if guilds become a live population (§6 / topic 7 below).

**The blocker under all three, and it is not the guild.** Quality *does* reach
price — `quality_value_mult = 0.6 + 0.9·q`, applied to a hub's own production, so
a 0.6×–1.5× spread is expressible. But the **ceiling** is
`0.62 + size_bonus(≤0.20) + struct_bonus(≤0.14)`, and `QUALITY_LEARN_RATE` (0.04/mo)
drives every producer to its own cap within a few years. So in practice the whole
world sits between 0.62 and 0.96 — a ~26% price spread — **and the ceiling is set
by CITY SIZE.** The largest city is automatically the finest maker of everything
it makes.

**MEASURED — and the prediction above is FALSE.** `real_world_craft_spread`
(Phase 0.2) ran it on a real world: across 14 manufactured goods the finest maker
was the biggest city **0 times**. Not because tradition already works, but because
`size_bonus` is `(pop / 60_000).min(0.20)` and therefore **saturates at 60,000
population** — so every real city with a workshop and a guildhall converges on the
same ceiling (0.960) and there is no leader at all, only a tie. The sole mechanism
that lifts anyone above it is `LAW_GUILD_MONOPOLY` (`GUILD_MONOPOLY_QUALITY_CAP`
0.97), which is a good omen for G-A/G-B: a guild charter is already the one thing
in the tree that makes a maker distinctive.

So the target is not "untie quality from size" but **"give the ceiling a term that
still discriminates among mature cities"** — and it is the same problem as topic 5,
not a separate one. Murano is an island, Solingen a town, Meissen a town; until a
small place can out-make a metropolis at one thing, no guild mechanic will read as
a signature craft.

---

## 4. A bureaucratic apparatus

**Measured.** The entire administrative model is one line:
`realm_collection_efficiency = cohesion × 1/(1 + decay·distance_from_capital)`,
with `autonomy` tilting the decay. There are no officials, no salaries, no
records, no corruption, and no way for a crown to *invest* in its own reach.
Tax farming already exists (a house buys N years of tithe collection for cash
now — `publicani`/*iltizam*), which is the alternative to a bureaucracy without
the bureaucracy existing to be the alternative to.

Cities are better served: `KeyFigure` already gives each city a
Head/Treasurer/Harbormaster/Magistrate, and **houses already bribe them.** So a
personnel-based administration is half-built at city level and absent at crown
level.

**Variants.**

- **A-A · The Chancery** *(smallest)*
  A realm buys `admin` levels; each raises collection efficiency and costs yearly
  upkeep. One field, one cost, one term in a formula that already exists. Gives a
  crown its first thing to spend on besides war. Weakness: it is a slider, not an
  institution.

- **A-B · The Survey** *(cheap, historically exact, and my pick)*
  A **cadastre** — Domesday, the Ottoman *defter*, the Milanese *catasto* — as a
  fifth `ProvWork` kind. Expensive once, per province; permanently raises that
  province's collection efficiency. It reuses `ProvWork` entirely (multi-year,
  funded-or-stalls, crown-funded, cost already scaled by real area and relief), it
  is visible on the province plate, and it makes "what a state can collect" a
  thing built over decades rather than a constant. **A state that knows what it
  owns collects more** is the whole thesis of pre-modern fiscality.

- **A-C · Officials as people** *(medium)*
  Extend `KeyFigure` to the crown: appointed persons with competence and loyalty
  who **skim**. This connects bureaucracy to the house system — a house places a
  kinsman in the chancery and buys real influence over collection — and reuses
  the bribery machinery already shipped at city level. It also gives corruption a
  home, which A-A cannot.

- **A-D · Farm or collect** *(cheap, uses what exists)*
  Make the existing tax-farming choice consequential: a farmed province yields
  cash now but its unrest rises and its efficiency decays, because the farmer
  squeezes. Direct collection needs A-A/A-B/A-C to be worth anything. This is the
  decision that makes a bureaucracy *matter* rather than merely exist.

**Recommendation:** A-B then A-D. A-A is the fallback if `ProvWork` turns out to
be the wrong host. A-C is the richest and should wait for a reason.

---

## 5. Settlements with unique production

The one I think has the highest ratio of world-feel to code.

**Measured.** A hub's basket is `base_per_capita`, derived at campaign start from
`compute_economy`'s per-hub production, which is belt score × catchment. So a
city's output is **entirely the ground under it**, and two cities in the same belt
are interchangeable. Nothing a city does or has done changes what it is known for.

Historically a signature product comes from five causes, and only two are modelled:

| cause | example | modelled? |
|---|---|---|
| terroir | Champagne, Falernian | **yes** — localities, §8.19 |
| a deposit | Potosí, Falun, Kutná Hora | **yes** — deposits, §8.16 |
| a craft tradition | Murano, Damascus, Toledo, Solingen, Delft, Meissen | **no** |
| an entrepôt that finishes what passes through | Amsterdam sugar & diamonds, Venetian re-export | **no** |
| a privilege | the staple right, Tolfa alum, a royal monopoly | **no** |

**Variants.**

- **S-A · Give the ceiling a term that discriminates among mature cities** *(the
  enabling change — do this first or nothing else reads)*
  Today `cap = 0.62 + size + structures` with the size term saturating at 60,000
  population, so — measured, not predicted — every real city converges on the same
  ceiling and no city leads at anything. Add an **accumulated tradition** term:
  years of continuous production of that good at that hub, guild presence, and a
  master's arrival (G-B), and widen the spread so a specialist can genuinely
  out-make a bigger neighbour. Nothing else in this section works without it.

- **S-B · The Signature** *(small, once S-A is in)*
  A city that has made a good long enough and well enough earns a **named**
  signature — "Muranese glass", "Toledo steel" — that travels with the cargo and
  commands a premium anywhere. The naming machinery (`names::gen_name`, and
  `brand_name`/`brand_place`, already used for colony toponymic brands) exists;
  `quality_value_mult` already prices quality. Mostly wiring plus a threshold.

- **S-C · The Refining Entrepôt** *(medium, and the biggest structural gain)*
  A city with high throughput and no local raw gets a manufacturing bonus for
  goods whose **inputs it imports**. That is Amsterdam refining Caribbean sugar and
  cutting Indian diamonds; `refined_sugar` is already a shipped recipe doing
  exactly this shape. It makes a port city structurally different from a producer
  city, which is the single largest missing distinction between settlements — and
  it gives the trade network a reason to have a *centre* rather than just lanes.

- **S-D · The staple right** *(needs exclusion, §0)*
  A city compels cargo passing through to be unloaded and offered for sale before
  it may continue — Bruges, Dordrecht, the Rhine towns. This is the purest form of
  "a city that produces nothing and is rich anyway," and it is `dispatch`'s
  prohibition list again, in a new mood. It is also the thing a league negotiates
  away for its members (§6), so build it before or with the Kontor.

**Recommendation:** S-A is a prerequisite for the whole topic and for topic 3.
Then S-B (cheap, high flavour) and S-C (structural).

---

## 6. Trade leagues — making "unified" mean something

**Measured — and better than expected.** `league.rs` is real and shipped:
formation on a genuine trade tie plus a shared threat, a yearly diet, dues,
drift-out when the threat lapses, dissolution below three members, a proper
`decide_*`/`apply_*` split. It is well built.

It also **does nothing.** Dues accumulate into `League.purse` and `purse` is
**never spent** — one writer, zero readers. The single collective verb is the
boycott, at `LEAGUE_BOYCOTT_MAX = 0`. So a league is currently a name, a member
list, and a growing pot of money nobody uses.

The Hansa's real instruments, ordered by how cheap each is here:

- **L-A · The purse buys a convoy** *(smallest, and it fixes a dead field today)*
  Dues fund escort: reduced voyage loss on member-to-member lanes. One reader for
  a pot that already fills, and it makes the league visible in a number the player
  already watches. Also the natural pair to W-B.

- **L-B · Privileged trade** *(medium, and what makes joining worth anything)*
  Members grant each other a tariff exemption and a freight discount; outsiders
  pay full. Tariffs already exist per polis. This is the first thing that would
  make league membership a *decision* rather than a label, and it makes blocs
  legible on the map without a single new overlay.

- **L-C · The Kontor** *(the characteristic one)*
  A shared factory at a **non-member** city with negotiated privileges — Bruges,
  Bergen, Novgorod, the London Steelyard. Members route through it and get its
  terms; the host city can expel it, which is a real political event.
  `YARDS_VESSELS_AND_DEPOTS_PLAN` already names the missing ownership class as the
  **fondaco** — state-owned, foreigner-occupied, compulsory — and a Kontor is that
  plus a collective owner. Warehouses, offices and bailos all exist to build it
  from. This is what makes a league a thing *on the map* rather than a list in a
  panel.

- **L-D · The boycott, dosed** *(built, at zero)*
  Denmark 1361–70 is the case: a league closes its members' markets to one city
  and wins. The code is there. **Dose it last**, and expect it to be dangerous for
  the same measured reason N2 was: market closure concentrates rent, and N2 broke
  the hard wealth bound twice.

- **L-E · The league fights** *(needs the war system)*
  The Hansa beat Denmark. A league at war is the natural top of this ladder and
  should follow W-A/W-C, not precede them.

**Note on cohesion.** `component_threatened` already reads real wars and powerful
realms, so a league's own reason to exist is already coupled to topic 2 — a world
with more consequential wars would produce more durable leagues without touching
`league.rs` at all.

**Recommendation:** L-A now (it is nearly free and fixes a dead field), L-B next,
L-C as the flagship. L-D last and carefully.

---

## 7. How they interlock — a suggested order

These are not six projects. Three chains:

**Chain 1 · Money and consequence.**
B-A (banks stop dying, coin trust recovers) → W-E (war is paid by borrowing) →
banks hold the debt → a lost war breaks a bank → the crash means something.
*This is the single most connected chain and every piece of it already exists
separately.*

**Chain 2 · Exclusion.**
W-C (contraband — a per-good ban, the safest possible first dose of exclusion) →
W-A (the routed blockade) → L-B/L-D (a league's privileges and its boycott) →
S-D (the staple right). Each is `dispatch`'s prohibition list with a different
author, and each must route rather than refuse.

**Chain 3 · Why a place is itself.**
S-A (untie quality from city size) → G-A/G-B (a craft can be kept and lost) →
S-B (the signature) → S-C (the entrepôt). Topic 3 and topic 5 are one problem.

Bureaucracy (topic 4) is orthogonal and can go any time; A-B is small enough to
land alongside any of the above.

---

## 8. Questions I need answered

**Q1 · What is the player, eventually?** Everything above is shaped by this. If
the answer is "a merchant bank" (B-D) I would build chain 1 first and design the
loan queue as I go. If it is "still observation-only", chain 3 is the better
investment because it makes the world worth watching rather than worth steering.

**Q2 · How much economic pain is a war allowed to cause?** The measured baseline
is ~45 wars/century. If war starts actually stopping trade, that frequency is a
problem — either wars must become rarer and bigger, or the per-war effect must
stay small. Which lever do you want moved?

**Q3 · Should exclusion ever be absolute?** My strong recommendation is no —
every prohibition routes through a neutral or a longer road at extra cost, never
refuses. That is both historically right and the only shape this codebase has
measured as safe. But it does mean a blockade never *starves* a city, and if you
want sieges to bite that is a deliberate exception to argue for.

**Q4 · For signature crafts, how wide should the spread be?** Currently the best
maker in the world sells at about 1.26× the worst. Murano glass against ordinary
glass was not 26%. Am I allowed to make a signature good 2–3× the price of the
generic — which is a real change to the wealth distribution and will need its own
dose walk against the inheritance gate?

**Q5 · Do leagues get to hold territory?** `league.rs`'s own doc calls a league
"a realm's negative — no provinces, no capital, no writ." A Kontor (L-C) is a
building on someone else's soil, which I think respects that. But if a league
should be able to *own* its Kontor city outright, that is a different entity and
the doc's premise needs revisiting.

**Q6 · Bureaucracy at which level?** Crown only (A-A/A-B, simplest), or city too?
Cities already have `KeyFigure`s and laws, so a civic administration is nearly as
cheap — but two administrative layers is real complexity for the player to read.

**Q7 · Is glass specifically interesting to you, or is it the example?** If you
want Murano *as such* — a named craft city with a state-protected secret — that
is S-A + G-A + G-B and I would scope it as one feature called "the signature
craft". If glass was shorthand for "guilds should matter", the answer is different
and probably starts at L-B/S-D, because what guilds actually did was *exclude*.
