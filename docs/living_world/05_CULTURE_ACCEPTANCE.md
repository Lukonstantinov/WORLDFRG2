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

**Relation score** per (city, culture), −100…+100, drifting yearly:
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

**The fondaco switched on:** the existing `Fondaco` struct becomes the tier 2–3
foreign quarter: founded by edict, it holds a foreign culture's merchants, can be
closed by the host (a real chronicle event).

## UI

A culture table in the settlement: each culture present or trading here — tier,
score and its trend, residents, and why ("trade +12, the war of 214 −20").

## Slices

| Slice | Content | Gate |
|---|---|---|
| 05.1 | Tiers + relation scores + stance, derived at campaign start; **read by nothing** | `default_tiers_follow_stance_and_relation` |
| 05.2 | Drift from trade/war/feuds/events; tier-change proposals into row 04's agenda | `trade_raises_relations`, `war_lowers_them` |
| 05.3 | Persecution, expulsion, massacre, diaspora with tradition transfer | `expulsion_moves_residents_and_tradition` |
| 05.4 | Bondage attitude; fondaco activation | `bondage_follows_culture_until_edict` |
| 05.5 | Effects dosed from zero (taxes, office, migration, scholars) | `acceptance_effects_are_noops_at_zero` |
| 05.6 | UI table; end of row `tick::tests` + `econ_` | SCOREBOARD row |

## Queue
- Q05.1 — Realm-wide tier policy (row 09).
- Q05.2 — Intermarriage raising relation scores (waits on row 02 marriages, Q02.2).
