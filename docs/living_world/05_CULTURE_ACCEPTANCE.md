# 05 · Culture acceptance

**Status:** NOT STARTED · **Depends on:** 03, 04 · **Next:** 06

## Goal

Each city treats each culture differently — from full citizens to a hated people
with no rights — and that treatment shifts with trade, war, events, people and
edicts. Open cities grow rich and learned; closed ones stay safe and small; and a
persecution sends skills and scholars to somebody else.

## Decisions (maintainer)

- **Five tiers** per city per culture:

| Tier | Name | Rights |
|---|---|---|
| 1 | Citizens | Full rights, may hold any office, highest chance of producing scholars |
| 2 | Enfranchised | Trade freely, limited political rights (lesser offices), may live in fondacos |
| 3 | Resident foreigners (metics) | Live and trade freely, **taxed more**, may run a fondaco, no office |
| 4 | Unwelcome | Heavily taxed, rarely settle; can visit to trade |
| 5 | Hated | No rights; expulsion, and in the worst case massacre |

- **Start from a default tier** and let random and not-so-random events, **notable
  people** (a scholar leaning the council) and edicts move it — the watcher should
  see surprising outcomes.
- **City stance**: remote cities are conservative and like their own; trading cities
  are open because foreigners bring income.
- **Who sets tiers:** a city for itself and the colonies/posts it directly
  controls; a realm for all its cities (row 09).
- **Massacres and expulsions are allowed** as chronicled events.
- **Slavery is per culture**: some accept it, some don't (the bondage attitude).

## What exists today (verified)

Cities hold `hub_culture`, `hub_minorities` and culture shares; creoles form from
long-resident minorities (`CREOLE_*`, ethnogenesis); cultures carry 14 traits
(Mercantile, Insular, Xenophobic, Assimilative, Diaspora…); migration moves
cultures between cities; `Fondaco` exists but is never founded (W5 at zero);
`LAW_FOREIGN_BAR` exists.

## Mechanics

**Relation score** per (city, culture), −100…+100, drifting yearly — stored
**sparsely**, only for cultures resident in or trading with the city (not 1,200 ×
every culture):
- up: shared trade volume, that culture's merchants resident, related culture
  family, the same language kit, its scholars and artisans living here,
  **admiration** of a more-developed culture (row 03);
- down: war with that culture's cities, feuds with its houses, its share of local
  trade (resentment of dominant foreigners), a famine or plague blamed on
  outsiders, **barbarian judgement** (row 03), a massacre remembered.
- **Stance** shifts the baseline: remoteness (few partners), Insular/Xenophobic
  traits pull closed; Mercantile/Assimilative traits and trade dependence pull open.

**Tier changes** (score proposes, edict disposes): when the score crosses a
threshold, an edict to change the tier enters the government's agenda (row 04);
whether it passes depends on the government's Openness axis (row 06), bribes,
events and scholars. A notable of that culture sitting in the city makes a raise
easier; a demagogue makes a lowering easier.

**Effects** (each dosed from zero):

| Effect | Tiers |
|---|---|
| May hold office / seats | 1 (full), 2 (lesser offices) |
| Scholar and artisan appearance chance | 1 full, 2–3 reduced, 4–5 none (history: Aristotle was a metic in Athens — tiers 2–3 do produce thinkers) |
| Tax surcharge | 3 small, 4 large |
| Settlement (migration in) | 1–3 normal, 4 rare, 5 none |
| Fondaco | 2–3 may found and run one |
| Development contribution (row 03) | share in tiers 1–3 |
| Realm cohesion (row 09) | open realms absorb conquests; closed ones hold only their own |

**Persecution events** (tier 5, or a sudden drop): expulsion (the culture's
residents leave), confiscation, riots, and rarely a massacre (e.g. 88 BC, the
"Asiatic Vespers"). Always chronicled. A **diaspora** leaves: residents migrate
toward the most welcoming reachable cities, carrying part of the city's craft
`tradition` and any resident scholars/artisans — the Huguenot effect.

**Bondage attitude** per culture (from traits; an edict can permit or abolish it
in a city). Used by row 08 (gladiators from captives only where permitted) and
row 09 (captives taken in war).

**The fondaco switched on:** the existing `Fondaco` struct's `occupant` is a
**house** (`mod.rs` ~6464). So: a fondaco is chartered by edict **for a house
whose culture holds tier 2–3** in the host city; it is the quarter where that
house's (and its culture's) merchants live and trade; the host can close it (a
real chronicle event).

## UI

A culture table in the settlement: each culture present or trading here — tier,
score and its trend, residents, and why ("trade +12, the war of 214 −20").

## Slices

| Slice | Content | Gate |
|---|---|---|
| 05.1 | Tiers + relation scores + stance, derived at campaign start; **read by nothing** | `default_tiers_follow_stance_and_relation` |
| 05.2 | Drift from trade/war/feuds/events; tier-change proposals into row 04's agenda | `trade_raises_relations`, `war_lowers_them` |
| 05.3 | Persecution, expulsion, massacre, diaspora with tradition transfer — **population movement behind `PERSECUTION_DOSE` = 0** (recorded, not applied, until 05.5) | `expulsion_moves_residents_and_tradition` (at a test dose) |
| 05.4 | Bondage attitude; fondaco activation | `bondage_follows_culture_until_edict` |
| 05.5 | Effects dosed from zero (taxes, office, migration, scholars) | `acceptance_effects_are_noops_at_zero` |
| 05.6 | UI table; end of row `tick::tests` + `econ_` | SCOREBOARD row |

## Queue
- Q05.1 — Realm-wide tier policy (row 09).
- Q05.2 — Intermarriage raising relation scores (waits on row 02 marriages, Q02.2).
- Q05.3 — Tier-change proposals currently run their own self-contained "shadow
  debate" (`CultureRelation.proposed_tier`/`debate_round`/`debate_tally`),
  deliberately kept separate from row 04's single per-city `GovDebate` slot (a
  city can only debate one ordinary edict at a time — routing every culture-tier
  crossing through that scarce slot would starve either ordinary government
  edicts or culture policy). Merging the two into ONE shared agenda list (so the
  Government window shows both kinds of proposal together) is real future work,
  waiting on a design decision about how the two compete for debate time, not on
  any missing mechanism — both halves already exist and are gated.
- Q05.4 — The five 05.5 effects (`acceptance_tax_mult_e`, `acceptance_settle_
  mult_e`, `acceptance_office_allowed_e`, `acceptance_dev_share_e`,
  `acceptance_scholar_mult_e`) are pure, tested and dosed from zero, but NONE is
  wired into a real tax/migration/office/development pass — the same "built and
  tested, called by nothing" shape row 03's `admires_more_developed`/
  `is_barbarian_to` already carry in this tree. Wiring a real effect needs a
  per-culture population/wealth attribution that `hub_minorities`'s plain
  0..1 share does not yet give (the pass can say "this culture is 20% of this
  city" but not "here is that 20%'s own treasury/office/migration weight" at
  the resolution these effects need) — new per-culture accounting, not a small
  diff, and its own `econ_` dose walk once raised above zero.
- Q05.5 — The diaspora destination (`maybe_persecute`) is picked from the SAME
  trade component only, an O(component) scan with no reach bound. At the
  shipped `PERSECUTION_DOSE = 0.0` this never moves anyone so the cost is
  paid for nothing measurable; once the dose is raised (Q05.6), a reach-bounded
  search (mirroring the trade-route `MAX_OPEN_SEA_CROSSING_KM`/component-horizon
  discipline, CLAUDE.md §8.5) should replace the unbounded component scan
  before it runs on a real, large world.
- Q05.6 — Raising any of `PERSECUTION_DOSE`/`ACCEPT_TAX_DOSE`/`ACCEPT_
  SETTLE_DOSE`/`ACCEPT_SCHOLAR_DOSE`/`ACCEPT_OFFICE_DOSE`/`ACCEPT_DEV_DOSE`/
  `FONDACO_CHARTER_DOSE` above zero is unstarted, separate work — each needs
  its own `econ_`-per-dose-step walk (00_INDEX's own testing rule: "if several
  doses are walked, run `econ_` once per dose step, not once for all of
  them"), and `ACCEPT_SCHOLAR_DOSE`/`ACCEPT_COHESION_DOSE` cannot be
  meaningfully raised at all until rows 06/07/09 exist to give them something
  real to scale.
