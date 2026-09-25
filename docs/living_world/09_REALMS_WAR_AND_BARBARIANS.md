# 09 · Realms, war and barbarians

**Status:** NOT STARTED · **Depends on:** 02–05 · **Next:** 10

## Goal

The arc the maintainer described: a house rises from nothing to rule a city, the
city becomes a state, states become empires and war with each other — Rome from
republic to empire, the Greek poleis to Macedon, Phoenicia to Carthage — while
barbarians rise from the provinces, raze cities and sometimes found kingdoms of
their own.

## Historical frame (from the analysis)

Every lasting empire had five parts, and every one broke at the fifth:

| Part | Rome | Greeks → Macedon | Phoenicia → Carthage |
|---|---|---|---|
| Revenue engine | tribute, Spanish silver, the grain annona | Laurion silver, Black Sea grain; Pangaion gold | trading posts, Spanish silver, Libyan farms |
| Money into force | paid legions | the fleet; a professional army | mercenaries |
| Absorbing the conquered | the *socii* (troops, not tribute), a ladder to citizenship, colonies as garrisons | leagues (Delian, Corinth) | treaty zones |
| Glue | roads, the denarius, one market | the owl, then Alexander's coinage | a sea network |
| Succession / legitimacy | generals' client armies → civil war → the principate | partible succession → the Diadochi | the Mercenary War |

The biggest lesson for design: **openness scaled empires** (Rome extended
citizenship; Athens closed its own in 451 BC and capped its manpower) — row 05's
tiers feed realm cohesion here.

## Decisions (maintainer)

- **Observation only**; conflicts are shown on a **map layer**: where wars are
  now, armies, **razed cities with a smoke animation**, destroyed cities.
- **Barbarians appear spontaneously from provinces**: from certain cultures, from
  discontent, from **opposition to a settlement** they dislike, when **trade does
  not benefit them**, and sometimes as a **new culture rising around a leader**
  (as a subculture or creole — the existing mechanism).
- Barbarian leaders are **notable persons with goals** (row 02); barbarian tribes
  get **their own window** with a short story of how they arose, a minimap of
  their location, and their main goals.
- **2–4 major barbarian waves a century.**
- A razed city can be **resettled after 10 years** (the existing threshold, kept for all — decided).
- **Provinces are deepened, not shrunk**, so they carry weight.
- Realm benefits and armies come in the same row as barbarians.

## What exists today (verified)

- Realms (`tick/realms.rs`): three founding paths (house, city, culture bloc),
  dynastic and civic governments, genealogy, taxation with collection efficiency,
  tax farming, expansion/vassalage/secession/partition, ranks and titles. The
  treasury is **spent** only on province works (`cities.rs` ~1094/1230) and
  annexation (`realms.rs` ~850); it is also read for war affordability (`war.rs`
  ~805) and moved by vassal integration and partition. **No member city gets any
  benefit.**
- War (`tick/war.rs`): city vs city; `maybe_declare_war` takes the **first
  eligible pair in index order**; `MAX_ACTIVE_WARS = 2` world-wide; strength =
  war chest + treasury + a small manpower levy; war goals incl. province, annex,
  humiliate, enthrone, vassalize; sack and purge; a realm capital cannot be
  annexed.
- Provinces: `prov_rural`/`prov_cap`, migration to cities, land use, tithe to the
  seat, unrest → revolt; `prov_neighbors`.
- The map already draws dead cities as a † ruin with `died_cause`.

## Part A — Why a realm is worth having

| Benefit | Mechanism (dosed from zero) |
|---|---|
| Internal free trade | intra-realm lanes get the league privilege (`lane_league_privileged`) |
| Roads | crown roads (Civil 5) cut lane days between member cities |
| The annona | the capital gets a grain lifeline from member provinces |
| Realm coin | unblock the realm coinage (MONEY plan M6 queue) |
| Protection | third parties need a much larger advantage to attack a member city |
| Knowledge | members share a floor on the development tracks (row 03, Q03.2) |
| Cohesion from openness | realm-wide acceptance policy (row 05) lowers the cultural cohesion penalty |

Measure first: `econ_measure_realm_benefit` — member vs free cities (wealth,
growth, famine, survival). Expected today: no difference.

## Part B — Armies and war between realms

- `Army { owner (realm or city or horde), province, strength, morale, upkeep,
  commander: Person }`. Raised from rural population and the soldier class, scaled
  by the Military track (row 03). **Paid monthly from the treasury** — upkeep is
  what gives a crown something to spend on. Unpaid armies mutiny and may become a
  **mercenary company** (Carthage's Mercenary War).
- Armies move along `prov_neighbors`; a province is **occupied** when an enemy
  army holds it uncontested; a city is **besieged** (walls from row 03 matter).
- **One war per city today:** `hub.war_with` is a single `i32`. Realm wars need a
  separate `RealmWar` record listing member cities, with a member's `war_with`
  kept for the city-level fight only; the O(n²) seat scan in `maybe_declare_war`
  must not grow with realm count (pair realms first, then cities).
- **Realm wars**: declared by realms; all member cities pool into one war score;
  fought by sieges and occupation; peace cedes occupied provinces; a foreign
  **capital can be taken**. The global two-war cap is lifted for realm wars (a
  per-realm cap instead); the pair choice uses a hash, not index order.
- Commanders are notables with the row-02 decision rule: spare or sack a city,
  keep faith or turn on the crown.

## Part C — Deeper provinces

Each province gains: **control** (whose writ really runs), **occupation**,
**fortification** level, **garrison**, a **levy** from its rural population,
**loyalty** to its realm (separate from culture), **barbarian pressure**, and
**villages** — the rural population drawn as settlement density that a horde can
burn.

## Part D — Barbarians

**A warband** (small) that can grow into **a horde**:

```rust
pub struct Horde {
    pub id: u32,
    pub name: String,            // from the culture's naming kit ("the Horde of Kaan")
    pub culture: u16,            // culture index; may be a NEW culture (subculture/creole via the existing mechanism)
    pub leader: u32,             // Individual (row 02), famous
    pub goal: u8,                // plunder · land to settle · revenge on <city/house> · a crown · tribute
    pub target: i32,             // hub or province
    pub home_province: u32,
    pub strength: f32,
    pub province: u32,           // where it is now
    pub origin_story: String,    // generated from the trigger that raised it
    pub history: Vec<LifeEntry>,
}
```

**Triggers** (from data the sim already has):

| Why they rise | Signal |
|---|---|
| Their culture is warlike | Martial or Nomadic culture, stateless province, rural population above capacity, steppe/arid/mountain land |
| Discontent | high province unrest, famine, an active tax farm, rule by a foreign-culture crown |
| Opposition to a settlement | a colony or outpost founded in a province of another culture (native resistance) |
| Trade does not benefit them | the province's trade controlled by foreign houses (`province_trade_shares`) while its exports are cheap raw goods |
| A new people forms under a leader | a mixed frontier province forms a confederation around a charismatic figure (the Franks, the Alamanni) — a new culture via the existing creole/subculture path |

**What a horde does:** raid (strip stock and treasury), **sack** (reuse
`strip_holdings_at` and war damage: estates stripped, damage, deaths, masterworks
looted — row 07), **raze** (the city becomes a ruin), occupy provinces, demand
tribute.

**How it ends:** it **settles** and founds a realm (the culture-bloc path — the
Visigoths); a crown **pays** or **hires** it (the *foederati*); it is **defeated**;
or it **breaks up** when its leader dies.

**Rate:** 2–4 major waves per century (decided), measured by a 300-year
diagnostic; smaller warbands more often. **Decided: a universal rate** — not
scaled by how much steppe or frontier a world has.

**Termination:** every horde and every mercenary company ends within a bounded
time (settles, is paid off, is defeated, or disbands) — the rule-22 discipline.

**Razing and resettlement:** a razed city is abandoned. The existing path
(`resettle_pass`) today waits `RESETTLE_COOLDOWN_YEARS` = **10** and needs a
patron city of ≥ 5k people within reach. **Decided: keep the 10-year threshold
for every razed city**, large or small — no change to `RESETTLE_COOLDOWN_YEARS`;
razing simply feeds the existing path.

## Part E — Empires

- **Empire** as a realm rank above kingdom: several cultures and many provinces,
  with overextension costs (cohesion).
- **The Greek path**: a league goes hegemonic when the league seat captures its
  purse (the Delian League → the Athenian empire). Leagues exist (`league.rs`).
- **The Carthage path**: a colony wins independence (exists) and builds its own
  realm.
- **Republic → principate**: in a civic realm, a commander whose army is loyal to
  him rather than the state stages a coup and the realm turns dynastic (row 04's
  coup path + row 02 decisions).

## Part F — The conflict map and windows

- **Map layer** (view only): pulsing markers on active wars, revolts and horde
  raids; hatched occupation over provinces; army and horde tokens with an arrow
  toward their target; **smoke plumes** over a recently sacked city for a few
  years; a charred **†** for a razed city ("Razed 412 by the Horde of Kaan");
  scorch marks fading on a sacked but surviving one. One query returns conflict
  sites, army positions and razed cities; nothing is computed for the view.
- Commands: `campaign_get_conflict_map`, `campaign_get_hordes`,
  `campaign_get_horde` (lib.rs + bridge + types). `Realm.events` gets a cap (row 01).
- **Barbarian Tribes window**: each horde — leader portrait and traits, the story
  of its rise, a minimap of where it is and where it has been, its goals, its
  strength, its history.
- **War window** extended for realm wars (fronts, sieges, occupied provinces).

## Slices

| Slice | Content | Gate |
|---|---|---|
| 09.1 | `econ_measure_realm_benefit` (measurement only) | printed numbers |
| 09.2 | Realm benefits, dosed from zero | `realm_benefits_are_noops_at_zero` |
| 09.3 | Province deepening: control, fort, garrison, levy, loyalty, villages | `levies_come_from_rural_population` |
| 09.4 | Armies: raising, upkeep, movement, occupation, mutiny → mercenaries | `unpaid_armies_mutiny` |
| 09.5 | Realm wars: pooled score, sieges, cession, capital conquest, hashed pairing | `every_realm_war_terminates` |
| 09.6 | Hordes: triggers, leaders (famous by design — may force a notable slot), goals, raids, sacks (calling row 07's `loot_masterworks`), razing, endings; the resettlement change | `hordes_arise_from_each_trigger`, `every_horde_ends`, `razed_cities_can_be_resettled` |
| 09.7 | Empire rank; league → hegemony; republic → principate | `leagues_can_turn_hegemonic` |
| 09.8 | Conflict map layer, Barbarian Tribes window, War window | `tsc`, `vite build` |
| 09.9 | End of row: dose walks; `tick::tests`, `econ_`, the 300-year wave-rate diagnostic | SCOREBOARD row |

**Risk:** razing and armies move population and wealth on a large scale; every
effect ships at zero and is dosed only in 09.9 against the full economy gates.

## Queue
- Q09.1 — Naval warfare as its own mechanic (fleets as armies at sea).
- Q09.2 — Personal union and inherited claims between dynasties.
- Q09.3 — Faith-driven movements (no religion system yet — decided).
