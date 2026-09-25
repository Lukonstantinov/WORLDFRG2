# 06 · Ideology and scholars

**Status:** NOT STARTED · **Depends on:** 02, 04, 05 · **Next:** 07

## Goal

Ideas are real forces. Scholars form views over their lives, found schools, move
between cities, advise rulers and move a city's politics; ideologies whose demands
are met spread along trade routes; a merchant's son who studied abroad comes home
with a new doctrine.

## Decisions (maintainer)

- **Hybrid** model: axes are the underlying numbers; **traits** express named
  ideologies. Scholars can craft **custom ideologies**.
- Axes run **−5 (conservative) … +5 (libertarian)**, fractional values allowed
  (−3.5).
- Every scholar has an ideology that **forms over time** through life events.
- Each city shows an **ideology meter** for its **nobles**, its **commons** and its
  **government**.
- A scholar **in government or teaching at a school** moves the meter.
- Each ideology has **demands** on edicts; met → it prevails and **spreads** to
  other cities via trade; unmet → unrest.
- Spread: trade + scholars travelling + diasporas carrying their home ideology.
- Scholars **spawn individually**, anywhere (great minds come from unexpected
  places), with a much higher chance at centres of learning.
- Notable persons' behaviour ("internal AI") is designed here for scholars.

## Axes

| Axis | −5 | 0 | +5 |
|---|---|---|---|
| Authority | one ruler, absolute | mixed constitution | the assembly of all |
| Tradition | ancestral custom is law | pragmatic | reason and reform |
| Openness | own kind only | guests welcome, not equals | all peoples may be citizens |
| Economy | state and guild set prices and trade | regulated market | free harbour, free trade |

## Ideology traits (the vocabulary)

| Trait | Authority | Tradition | Openness | Economy |
|---|---|---|---|---|
| Rule of the Best Families | −2.0 | −1.5 | | |
| Law Above Kings | +2.5 | +1.0 | | |
| The Strong Hand | −3.5 | | | −1.0 |
| Ancestral Piety | | −3.0 | −1.0 | |
| Inquiry and Reason | | +3.0 | +0.5 | |
| The Stranger Is a Guest | | | +2.5 | +0.5 |
| Blood and Soil | | −1.0 | −3.5 | |
| Free Harbour | | | +1.0 | +3.0 |
| Just Price | | −0.5 | | −2.5 |
| Voice of the Many | +3.5 | +0.5 | | |
| Virtue of Frugality | | −1.0 | | −1.0 |
| Citizenship Earned | +0.5 | | +1.5 | |
| Honour in Arms | −1.0 | −1.0 | −0.5 | |
| Commonwealth of Letters | +1.0 | +2.0 | +1.5 | |
| Harmony and Order (Confucian-like) | −1.5 | −1.5 | | −0.5 |
| Merit Before Birth | +1.0 | +1.5 | +0.5 | |

A **named ideology** = 3–5 traits; its position = the sum of their pushes, clamped
to ±5. A scholar founding a school crystallises a **custom ideology**: the 3–5
traits nearest their personal position, named from their city and culture
("the Kedran Inquiry"). Named ideologies are few and persistent; every one records
its founder and where it holds sway.

## Meters

Per city: `nobles: [f32; 4]`, `commons: [f32; 4]`, government = the ruler's or
the seats' prestige-weighted mean. The gap between nobles and commons is
**tension** (feeds unrest and the revolution path in row 04). Each professional
class (`Pop`) carries its own position; nobles are elites + houses, commons the
rest (Victoria 2-style, over the existing 9 professions).

Drift each year:
- resident scholars (by prestige), schools and the university pull toward their
  doctrine;
- edicts that satisfy an ideology's demands raise its hold;
- events: a sack pulls Authority toward the strong hand; a trade boom pulls
  Economy toward free trade; a plague blamed on foreigners pulls Openness down;
- trade partners' commons pull a little (spread);
- diasporas bring their home city's position.

## Demands

Each named ideology lists 2–4 edicts it wants (row 04): *Voice of the Many* wants
wider seats and ostracism; *Free Harbour* wants free-harbour and fondaco edicts;
*Blood and Soil* wants expulsions and a foreign-ownership bar. Met demands raise
its adherents' contentment and its spread rate; unmet ones raise their unrest.

## Scholars — lives and choices

Stages, each a set of decisions using row 02's 75 % rule:

1. **Birth and youth** — anywhere (a small chance even in a backwater), higher in
   centres of learning. Origin and family from real facts.
2. **Study** — a child of a merchant or noble family may travel to a centre of
   learning. More likely with family wealth/house, trade reach to the centre, the
   centre's fame; less likely if their culture is tier 4–5 there.
3. **Career** — stay and teach, return home, or seek a patron. Weighed:
   patronage (rich houses, a court), freedom (their tier and ideological fit),
   peers (rival schools nearby), home ties. The merchant's son who returns home
   brings a school, a doctrine and development.
4. **Politics** — take a seat, advise a ruler, become an orator or demagogue.
5. **Exile** — persecuted or out of step with a new regime → flee to the most
   welcoming reachable city.
6. **Legacy** — writings and students keep moving the meter after death.

Relationships: teacher, students, rival (a rival school in the same city),
patron. Personal ideology forms by: study (pulled 30–60 % toward the teacher),
travel (Openness up), exile (pushed away from the exiling regime), witnessing a
sack (toward the strong hand), prospering under a patron (toward the patron),
their culture persecuted (Openness up or down).

## Institutions

Tutor (one scholar) → **school** (a scholar with students; has a lineage) →
**library** (Ideological 3) → **academy** (Ideological 4) → **university**
(Ideological 5; produces scholars on its own and outlives founders).
Institutions add Ideological points (row 03) and raise the appearance chance.

## Guard against a permanent winner

Learning must be able to move: sacks burn libraries (row 09), persecutions export
scholars, patrons die, and development diffuses. Diagnostic
`econ_measure_learning_centres` (300 years, `#[ignore]`d) must show the leading
centre change over the centuries (Miletus → Athens → Alexandria).

## UI

City: the three meters on four axes, the ruling ideology, resident scholars and
schools. World window **Schools & Great Minds**: schools and lineages as a tree,
each scholar's travels on a minimap, ideologies ranked by adherents with their
traits and founders.

## Slices

| Slice | Content | Gate |
|---|---|---|
| 06.1 | Axes, trait vocabulary, named ideologies, meters seeded from culture traits; **read by nothing** | `ideology_positions_are_bounded` |
| 06.2 | Personal ideology on `Individual`, formation events | `study_pulls_toward_the_teacher` |
| 06.3 | Scholar lives: spawning, study travel, career, exile, legacy | `scholars_prefer_centres_but_appear_anywhere` |
| 06.4 | Schools, lineages, custom ideologies, institutions | `a_school_names_a_custom_ideology` |
| 06.5 | Meter drift, demands vs row 04 edicts, spread along trade | `met_demands_spread_the_ideology` |
| 06.6 | Government reads its ideology in edict costs (row 04 hook) | `mismatched_edicts_cost_more` (re-run) |
| 06.7 | UI; end of row `tick::tests` + `econ_` + the learning-centres diagnostic | SCOREBOARD row |

## Queue
- Q06.1 — Ideological leagues between like-minded cities (a realm formation path,
  row 09).
- Q06.2 — Written works as objects that travel (a treatise read in another city),
  waits on row 07's masterwork records.
