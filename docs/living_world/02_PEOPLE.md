# 02 · People

**Status:** PARTIAL — 02.1 shipped (`Individual` + `people`/`hall_of_dead` +
migration + weekly hook + the Guildmaster-renaming fix), 02.2–02.9 queued
(see §Slices and CLAUDE.md §5.8) · **Depends on:** 01 · **Next:** 03 (once this
row is DONE)

## Goal

One system for every named human the world tracks — ruler, senator, commander,
scholar, artisan, gladiator, horde leader — so that each has a face, a character,
a life that responds to what happens around them, decisions that are
characteristic but not predictable, and a story the maintainer can read.

Everything in rows 03–09 hangs off this row: seat holders (04), scholars (06),
artisans (07), performers (08), commanders and barbarian chiefs (09).

## Decisions (maintainer)

| Topic | Decision |
|---|---|
| Two kinds of people | **Ordinary** (seat holders, officials, local figures — many) and **Notable** (world-famous). Ordinary people can become notable; notables can hold seats |
| Notable cap | **40 alive in total**, performers included, **no role quotas**. Later a setting from 20 to 60 |
| The dead | Dead notables go to a **Hall of the Dead** subpanel with their full life. Dead ordinary people are **forgotten** (deleted) |
| Debut | A person enters the record when they become someone, **at 16 or older**; elders can debut too |
| Life span | From debut to death; the backstory before debut (birth, family, childhood) is generated from real facts |
| Death | Age, plague, war, sack, accident, duel, execution — anything the world can do |
| Traits | **Up to 3 when young**, gained through events and decisions (Crusader Kings style), shown as **tags only** — no esteem bar |
| Decisions | Probability from traits + small modifiers; **≥ 75 % → the person always chooses it**, otherwise a roll |
| Modifiers | Small (≈ ±5 %) and temporary: rushed, drunk, threatened by X, under the influence, remembering a friend's help… |
| Events | **0 to 10 a year**, driven by the person's circumstances (war, siege, plague, festival, travel) and role; **a valid event always exists** (in-city fallback); geography-checked |
| Production | Stories draw on the city's **real** goods, mines and estates (variant A — text only, no ownership) |
| Faces | **Every person has a unique face**, with features (beards, scars, a lost eye…) — some acquired through events |
| Life story | Short generated biography: where they came from (this city, a colony, abroad), family, why they hold their position, what people think of them |
| Performance | Must not slow the tick |

## What exists today (verified)

- `Figure` (`mod.rs`, ~7023): name, kind (Admiral · Demagogue · Master
  Craftsman · Great Banker · Explorer), hub, house, good, born/dies tick, dead,
  rallied. `FIGURE_LIVING_CAP` = 6, `FIGURE_CAP` = 60. Raised by
  `raise_notable_figures`, kept acting by `living_figures_pass` (`houses.rs`).
- `Notable` (`mod.rs`, ~7080): per-city Guildmaster / Alderman / Agitator
  (SETTLEMENT_LIFE_PLAN L12), rebuilt yearly.
- `Official` (`mod.rs`, ~4706): 4 per city (Head · Treasurer · Harbormaster ·
  Magistrate), house `control`, `kin`, `term_end`. Bribery/intimidation in
  `update_government`.
- `Kin` roster on houses (character axes, skill, loyalty) and `Person` in realm
  genealogies (`Realm.family`, `person_mortality_hazard` in `realms.rs`).
- Portraits: `src/ui/campaign/cultureDress.ts` (`DressKit`, `drawBust`).
  `cultureFigure.ts` **no longer exists**, and `cultureDress.ts` itself says it has
  **no sex axis** — unique faces for women and men need one added (02.7). `read_people.rs` builds `bio`, `life_events`
  (from the journal — row 01 materialises this), `thought`.

**Unification rule:** `Individual` becomes the one record for public people.
`Figure` migrates into `Individual` on load (it is a persisted person). The L12
`Notable` is **not** a persisted person — it is rebuilt yearly from a guild, a
council house's `kin[1]` and the Demagogue figure — so it is **not** migrated; it
becomes a local role that points at an `Individual` id (created once, stable,
linked via `kin_ref` / guild id / figure id), or the yearly rebuild would mint new
people every year. House `Kin` and realm `Person`
entries stay where they are, but a kinsman or royal who **enters public life**
(takes a seat, becomes notable) gets an `Individual` record linked back by id — no
duplicated state.

## Data

```rust
pub struct Individual {   // NOT `Person` — that name is the realm genealogy struct (mod.rs ~8620)
    pub id: u32,
    pub name: String,
    pub female: bool,
    pub culture: u16,             // index into the culture table, not a String
    pub birth_tick: u32,          // generated; before debut
    pub debut_tick: u32,          // age ≥ 16 at debut
    pub death_tick: u32,          // 0 = alive
    pub death_cause: u8,          // CAUSE_* (reuse the L4 table + duel/execution/accident)
    pub origin_hub: i32,          // born here (−1 = abroad/unknown, see origin_text)
    pub formed_hub: i32,          // where they studied / made their name
    pub current_hub: i32,
    pub house: i32,               // −1 none
    pub kin_ref: i32,             // index into House.kin if a kinsman
    pub roles: Vec<u8>,           // ROLE_* — a person can hold several over a life
    pub famous: bool,             // one of the 40 "notables" (UI word)
    pub fame: f32,                // bounded, decays
    pub traits: Vec<(u8, i8)>,    // (TRAIT_*, strength −2..+2), ≤ 8
    pub modifiers: Vec<Modifier>, // (MOD_*, value, expires_tick), ≤ 6
    pub ideology: [f32; 4],       // row 06 fills; zero until then
    pub face_seed: u32,
    pub features: u32,            // bitflags: ONE_EYED, SCARRED, LAME, BALD, GREY, TATTOOED…
    pub relations: Vec<(u8, u32)>,// (REL_MENTOR/RIVAL/FRIEND/PATRON/SPOUSE/STUDENT, person id), ≤ 8
    pub backstory: Vec<(u16, Vec<u32>)>, // template ids + args; text generated at read time
    pub life_log: Vec<LifeEntry>, // (tick, template_id, args); famous: never pruned; ordinary: ≤ 12
}
```

- Living people: `CampaignSim.people: Vec<Individual>`.
- Dead notables: `CampaignSim.hall_of_dead: Vec<Individual>` (kept forever; their
  `modifiers` cleared, `life_log` kept).
- Dead ordinary people: removed at death (optionally one line in the city
  chronicle if they held a senior office — row 04 decides which offices), except
  a **tombstone** (id, name, culture, years, role) while anything still refers to
  them (00_INDEX "Ids and tombstones"). Ids are never reused.
- **Hall of the Dead size:** famous people are few (≤ 40 alive, turnover of a few
  per decade), so the Hall grows by roughly 10–20 a century — kept in full,
  measured in the row-01 save-size diagnostic.

## Roles

Ruler · Commander · Admiral · Diplomat · Merchant Prince · Banker · Guildmaster ·
Philosopher · Scholar · Ideologue/Orator · Physician · Artisan (kinds in row 07) ·
Performer (kinds in row 08) · Explorer · Demagogue · Senator/Official (row 04) ·
Horde leader (row 09). The existing five `FIGURE_KINDS` map onto these.

## Traits

About 40, in five groups. Each trait has (a) weights per decision kind and (b)
weights per event template (e.g. a Drunkard attracts tavern events).

| Group | Traits (examples) | Gained how |
|---|---|---|
| Personality | Kind / Cruel · Brave / Craven · Honest / Deceitful · Proud / Humble · Ambitious / Content · Curious / Closed · Loyal / Fickle · Temperate / Impulsive · Generous / Greedy | 1–3 at debut from culture + family; some can flip after a strong event (a Brave man broken by a massacre becomes Craven) |
| Education | Orator · Strategist · Scholar · Administrator · Seafarer · Physician-trained | Study (row 06), service |
| Lifestyle | Drunkard · Gambler · Ascetic · Glutton · Hunter · Patron of the Arts | Repeated choices, events |
| Health | One-eyed · Lame · Scarred · Sickly · Robust · Maimed hand | Battles, accidents, plague survived (sets a face feature too) |
| Reputation | Hero · Coward · Oath-breaker · Benefactor · Bought (known to take bribes) · Kin-slayer | Deeds that became public |

Young people (debut before 25) carry **at most 3**; the cap rises to 8 over a
life. A new trait that contradicts an old one replaces it.

## Decisions — the 75 % rule

```
for each option o:  p(o) = base(kind, o) + Σ trait_weight(kind, o, trait) × strength
                          + Σ modifier_values(o) + context(o);  then normalise to sum 1
if max_o p(o) ≥ 0.75 → that option, always
else                 → pick by hash01(seed, tick, individual, kind) over the p(o)
```
Defined per option, so the rule is symmetric: with two options, A at ≥ 75 % is
certain and so is B at ≥ 75 % (A ≤ 25 %) — which option is "first" never matters.

- Clamped to `[0, 1]` before the test. The 75 % threshold is one constant,
  `DECISION_CERTAIN_AT`, per the maintainer; individual decision kinds may add
  their own context terms but never their own threshold.
- **Modifiers** (≈ ±5 %, each with an expiry): Rushed decision · Drunk · Threatened
  by *[house/person]* · Under the influence of wine / opium · Remembered how a
  best friend helped them · Grieving · In love · Feverish · Flattered by *[name]* ·
  Recently humiliated · Sleepless · Emboldened by victory · Owes a debt to
  *[name]* · Blackmailed by *[name]* · Homesick · Newly wealthy.
- Every decision is logged with its dominant reasons so the UI can say *why*
  ("stayed: Kind, remembered his mentor tending the sick").

**Decision kinds (first set):** stay or flee (plague, siege, persecution) · accept
or refuse a bribe · accept or refuse an office · accept an invitation/commission ·
insult or defer to a ruler in a quarrel · spare or sack a city (commander) · betray
or keep faith with a patron · marry for love or alliance · retire or keep
working · take a student · join a conspiracy · speak or stay silent in a debate.

**Worked example.** Plague in Kedra. Theon: base 0.40 + Kind (+0.15) + Brave
(+0.10) = 0.65, plus "remembered his mentor tending the sick" (+0.05) = 0.70 —
under 0.75, so he rolls: stays 70 %, flees 30 %. If he stays, his plague death
risk is ×3 and a survival gives a chance of the Benefactor trait.

## Life events

**Rate.** Each person, each year, draws a count (ordinary individuals use a
lower role base — they are many and mostly quiet; famous ones the full rate):
`λ = role_base × turbulence`, where turbulence multiplies for war ×3, under
siege ×4, plague ×2, famine ×1.5, festival ×1.5, travelling ×2, holding office
×1.5; the count is a hashed Poisson draw, **capped at 10**, and can be 0. Events
are spread across the year's ticks (not all on 1 January).

**Templates.** Each event template has:
- `requires`: tags that must be true of the person's current place and state —
  geography (`coast`, `river`, `lake`, `desert`, `mountain`, `steppe`, `forest`,
  `cold`, `tropical`), city (`large`, `at_war`, `besieged`, `plague`, `famine`,
  `festival`, `sacked_recently`, `has_mine:<good>`, `exports:<good>`), person
  (`role`, `age_band`, trait tags), travel state;
- `weight`, and trait weights;
- `effects`: trait gain/loss, modifier, ideology nudge (row 06), fame, a face
  feature, a relationship, a track point (row 03), death;
- `text`: a template filled from real names and facts.

**Guaranteed firing.** The pool is layered — role + surroundings → surroundings →
**in-city generic** (market, tavern, workshop, home, street quarrel, a debt, a
neighbour's wedding). The generic layer requires nothing, so a due event always
finds a template. A year can still have 0 events — that comes from the count, not
from an empty pool.

**Production-linked (variant A).** Tags `has_mine:<good>`, `exports:<good>`,
`estate:<good>` come from the city's real mines (`mine_deposits`), estates and
top exports, so origin stories and events can say *"son of a ruby-cutter from the
Kora workings"*, *"a new vein struck"*, *"the pepper price collapsed and ruined
him"*. Text only — no ownership.

**The logic reviewer.** A test, `life_event_templates_respect_geography`, holds a
keyword → required-tag table (sea / ship / whale / harbour / shipwreck → `coast`;
river / ferry → `river`; camel / dune → `desert`; snow / ice → `cold`; …) and
fails if any template's text uses a keyword its `requires` does not guarantee.
The ~150 full templates are written **in a separate session**, with a review agent
checking them against this table and against the role list.

**Role-specific examples:** commander — quells a mutiny, spares a city, is
ambushed on the march; admiral — rides out a storm, loses a ship on a reef;
scholar — loses a public debate, burns a manuscript, finds a forgotten scroll;
artisan — a patron refuses to pay; performer — a crowd riots; senator — a speech
turns the vote.

**Inspiration events** (scholars, artisans; they nudge ideology or a track):
watching whales from the mole (`coast`), breaking a wild horse (`steppe`),
counting stars from a pass (`mountain`), haggling over pepper (`exports:pepper`),
watching rams break a gate (`besieged`), tending the sick (`plague`), losing a
debate, the road into exile, clinging to a spar after a shipwreck (`coast` +
travelling by sea), a boxer's defeat at the games (`festival`), a forgotten scroll
in the library (row 03 building), bread riots (`famine`).

## Forward hooks (neutral until the row exists)

Event effects that write development-track points (row 03) or ideology (row 06)
are recorded but applied only once those rows exist (they no-op before).

## Faces and life story

- `cultureDress.ts` gains feature layers: beard styles, grey hair and baldness
  with age, scars, an eyepatch, a broken nose, culture tattoos/paint, jewellery by
  wealth. Seeded by `face_seed`; acquired features from `features` bitflags.
- **Backstory** at debut, assembled only from facts: origin (this city / a named
  colony / abroad), family (house kin or commoner, with the family trade from the
  production tags), how they rose (the role path), and one childhood episode from
  the origin city's real history (a famine, a sack, a festival).
- **What people think**: reputation traits are the answer (Hero, Bought,
  Benefactor…), shown as tags.

## Notable promotion and the Hall of the Dead

- `fame` rises with deeds (events flag how much), decays slowly.
- An ordinary person whose fame crosses `NOTABLE_FAME_THRESHOLD` becomes notable
  **if a slot is free** (40 alive). Roles that are famous **by design** — a horde
  leader (row 09), a talented master artisan (row 07) — may **force** a slot: the
  least-famous living notable is demoted. If full, the least-famous living notable whose
  fame is below the newcomer's by a margin is demoted back to ordinary (keeps
  their story — they stay an `Individual`; if they later die ordinary but were once
  notable, they still go to the Hall).
- Existing figure mechanics (`raise_notable_figures`, `living_figures_pass`)
  continue as the "births" of certain notable roles.

## UI

- **Person window:** portrait, name and epithet, trait tags, role(s), where they
  are, the backstory, the life log (year-grouped), relationships (clickable),
  last decision and why.
- **Notables roster:** the 40 living, filterable by role and city.
- **Hall of the Dead:** its own subpanel; the full lives of dead notables.
- `FiguresPanel.tsx` becomes the roster's front page (its portrait gallery is the
  starting point).

## Performance

- Weekly pass over living people only to expire modifiers and fire due events —
  `O(people)` with small constants; ordinary people ≈ 10k at most (row 04 scales
  seats with city size), notables 40.
- Decisions and events are pure functions over ≤ ~20 numbers + one hash.
- Storage: ~200–400 bytes per ordinary person, a few KB per notable; ≈ 2–4 MB for
  a large world. Measure with the row-01 save-size diagnostic.

## Slices

| Slice | Content | Gate |
|---|---|---|
| 02.1 ✅ | `Individual` + `people`/`hall_of_dead`; load-time migration of `Figure` (with its `life_log`); L12 `Notable` linked to stable `Individual` ids; the **weekly** `tick % 7` hook added to `advance()`; the salt registry; **inert** | `figures_migrate_to_individuals_losslessly`, `local_roles_do_not_mint_new_people_yearly`, `living_world_is_inert_at_zero` |
| 02.2 | Trait catalogue, modifier catalogue, `decide()` with the 75 % rule, reasons | `decision_at_75_percent_is_certain`, `modifiers_can_tip_either_way`, `decisions_are_deterministic` |
| 02.3 | Life cycle: debut ≥ 16, aging, mortality (reuse `person_mortality_hazard`), death causes, fame, promotion/demotion, Hall of the Dead, ordinary forgotten | `notables_never_exceed_the_cap`, `dead_notables_keep_their_story`, `dead_ordinary_people_are_removed` |
| 02.4 | Event engine: rate, tag evaluation, layered pool with fallback, effects, logging | `a_due_event_always_finds_a_template`, `event_rate_follows_turbulence` |
| 02.5 | ~40 starter templates + the geography lint | `life_event_templates_respect_geography` |
| 02.6 | **Separate session:** ~150 more templates, reviewed by an agent against the lint and role list | the same lint |
| 02.7 | Faces: feature layers in `cultureDress.ts`; acquired features | `tsc`, visual check |
| 02.8 | Person window, roster, Hall of the Dead — commands `campaign_get_individual`, `campaign_get_notables`, `campaign_get_hall_of_dead` (lib.rs + bridge + types) | `tsc`, `vite build` |
| 02.9 | End of row: `bench_campaign_tick_large` before/after, `tick::tests`, `econ_` | numbers in SCOREBOARD |

No row-02 mechanism moves money or population except what `living_figures_pass`
already does, so `econ_` should be bit-identical.

## Queue
- Q02.1 — The 20–60 notable-cap setting (waits on a campaign settings panel).
- Q02.2 — Marriages between notables and house kin feeding house alliances (waits
  on row 04's alliance use).
- Q02.3 — Portrait ageing animation across the life log (cosmetic, after 10).
