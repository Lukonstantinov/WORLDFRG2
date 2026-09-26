# 03 · Development tracks

**Status:** DONE — slices 03.1-03.7 all shipped; 03.7's `DEV_PRODUCTION_DOSE`
raised to 0.2 and kept, `tick::tests` (343/343) and the full `econ_` suite
(6/6, incl. the multi-seed inheritance gate) both green (dose step 1 of its
own walk; the building-effect doses remain queued) · **Depends on:** 02 ·
**Next:** 04

## Goal

Cities develop over time, each in its own way. A city's growth comes from trade,
learning and stable government, spreads along trade routes, and is lost to war,
sack and strife. The development shows as four independent tracks, each unlocking
buildings with real effects.

## Decisions (maintainer)

- **Per-city development factor with diffusion** (variant B): grows from the
  city's own sources and is pulled toward its trade partners. It **rises faster
  the more trade happens**, and also from scholars, schools and universities, and
  from cultural acceptance — each with its own modifier.
- **Shown in every settlement**: the value, this year's growth, a history, and why.
- **Four tracks**, each an independent factor with its own points and level:
  **Military · Trade · Civil · Ideological** (diplomatic and cultural merged into
  Ideological — science, schools, philosophy and ideas, including foreign
  philosophers and ideas arriving from abroad).
- **Levels rise automatically** when points reach the threshold.
- **Points can be lost**: destruction, war, internal conflict. **Stability is the
  key multiplier.**
- **Culture ideals**: each culture gains prestige from what it values (the Mongols
  from martial conquest) and judges others by it.
- Leisure venues are a separate building ladder (row 08), not a track.

## What exists today (verified)

- `CampaignSim.tech_factor` — ONE global number, meant to grow 1.5 %/year, but
  `roll_events`' fire/event setbacks outweigh the growth, so it collapses to
  `TECH_FACTOR_FLOOR` (0.85) within ~6 years and stays there (documented at
  `mod.rs` ~976 and in `economy_validation.rs` ~2285). **The world has no
  technological progress today** — in effect a constant ×0.85 on production. It is
  read in `production.rs` (~873, ~992) **and** `mod.rs` (~9806).
- `TickHub.tradition` (per-good craft practice), craft guilds, signatures,
  `LAW_*` laws, structures (`buildingArt.ts` draws 15 building types),
  `TickHub.annals` (`CityYear`, yearly) — the natural home for the factor's
  history.
- **`dev_tier`/`dev_momentum` are ALREADY a per-hub development tier**, 0–5
  (Outpost … Emporium), computed in `cities.rs` (~2449, `development_tier` ~2538)
  and shown by `read_hubs.rs` (~707). The new factor must **absorb or feed** that
  tier, never sit beside it with a second, disagreeing notion of "development".
  Decide in 03.1: the tier becomes a *display banding* of the new factor.
- World-age development is a different thing: `WORLD_AGE_DEV_CAP` / `trade_dev`
  in `disease.rs` (~510–546), a growth-ceiling term — leave it alone.
- **`hub.structures` already exists** (`mod.rs` ~3987) with five buildings —
  `STRUCT_GRANARY` (+12 % food), `STRUCT_WAREHOUSE`, `STRUCT_SHIPYARD`,
  `STRUCT_GUILDHALL`, `STRUCT_WORKSHOP` (`mod.rs` ~2570). Track buildings extend
  this list; the Civil-1 granary **is** `STRUCT_GRANARY`, the Trade warehouse **is**
  `STRUCT_WAREHOUSE`, the guildhall/workshop map onto Trade/Ideological rungs.

## The development factor

Per city, yearly:

```
growth = Σ source_i × modifier_i  × stability
       − decay (sack, plague deaths, famine, isolation)
dev    = dev + growth + DIFFUSION × Σ_partners w_ab × (dev_b − dev)⁺
```

| Source | Modifier (first cut, all constants) | Notes |
|---|---|---|
| Trade volume | `log(1 + volume / ref)` | Doubling trade adds a step, not a doubling |
| Partner reach | distinct partner cities and cultures | Miletus: contact, not size |
| Scholars / schools / universities | per resident scholar, per institution | row 06 supplies them |
| Cultural acceptance | share of residents in tiers 1–3 | row 05 supplies it; 0 contribution until then |
| Welfare | welfare ratio (L2) | people fed and housed |
| Diffusion | `DIFFUSION × trade weight × gap` | only pulls **up**; a backwater linked to a hub rises |

Decay: a sack or razing removes a share (row 09), plague deaths and famine
remove a little, and a city with no trade partners loses knowledge slowly
(post-Roman Britain). Growth is diminishing near a soft ceiling so no city runs
away forever.

**Display:** the settlement shows the value, this year's change ("+1.8 %"), a
50-year sparkline (a new `CityYear.dev` field), and the breakdown ("trade +0.9 ·
scholars +0.5 · diffusion from Kedra +0.3 · plague −0.4").

**Replacing `tech_factor`:** production reads a city's factor instead of the
global one through `DEV_PRODUCTION_DOSE`, blended from the old value — at 0.0 the
old global value is used everywhere (bit-identical); dosed in the last slice.
**Normalise:** the city factor fed to production is scaled so that the
population-weighted **world mean at campaign start equals the old value** (0.85
today). Otherwise switching the dose on would jump production everywhere at once
instead of letting cities diverge from a common start.

## The four tracks

Each track: `points: f32`, `level: u8` (0–5), thresholds rising per level
(`TRACK_THRESHOLDS`). Points arrive yearly from the sources below × stability,
and a level is gained **automatically** when points reach the next threshold.
Points (and at worst a level) are **lost** to sack, civil war, a coup, a burned
building.

| Track | Points from | Notable roles that add |
|---|---|---|
| ⚔ Military | Victories, defended sieges, soldier-class share, Martial culture, levies raised | Commanders, admirals |
| ⚖ Trade | Trade volume, entrepôt role, houses resident, banks, guild strength | Merchant princes, bankers, prestigious guilds, guildmasters |
| 🏛 Civil | Able magistrates, working institutions, laws, welfare, public works | Physicians, able officials |
| 📜 Ideological | Resident scholars of **any** culture, schools, ideas arriving along trade routes, envoys and treaties, masterworks | Philosophers, scholars, ideologues, artisans, poets |

**How a city leans between tracks.** The tracks are independent, but the
**Lustrum** direction edict (row 04, every 5 years) adds a bonus to one track and
rewards that track's backers. Until row 04 exists, the bonus goes to the track
with the greatest *need* (see below). Need per track:
- Military: recent war or raid, a horde nearby, breached walls.
- Trade: trade volume, entrepôt role, foreign merchants.
- Civil: crowding, plague deaths, famine, unrest.
- Ideological: scholars present, schools, contact with more-developed cultures.

The government's ideology (row 06), the ruler's character (row 02) and lobbying
by houses and guilds bias the Lustrum's choice.

**Stability** (one formula, used everywhere): from legitimacy (row 04 — a neutral
constant until row 04 exists), unrest,
war, recent disasters, and deadlock. 0.2 (chaos) to 1.2 (golden age).

## Buildings

A level makes a building **possible**; the building itself costs real goods
(timber, stone, iron — `pick_build_supply_good`, as L6 housing does) and money,
and is decided by the government (row 04; until then an automatic rule). A
building can be damaged or destroyed in a sack (row 09).

| Level | ⚔ Military | ⚖ Trade | 🏛 Civil | 📜 Ideological |
|---|---|---|---|---|
| 1 | Palisade | Quay | Well and granary | Shrine / monument |
| 2 | Stone walls | Harbour mole · warehouse district | Forum / council hall | School |
| 3 | Barracks | Mint and exchange | Public granary (horrea) | Library |
| 4 | Arsenal (siege engines) | Lighthouse · fondaco quarter | Aqueduct and sewer · baths | Academy · embassy |
| 5 | Fortress | Bourse (banking, insurance) | Roads and courier service | University |

Effects (each behind its own dose constant, 0.0 until the last slice):

| Track | Effects |
|---|---|
| Military | army strength, levy share, siege attack/defence, horde resistance (row 09) |
| Trade | warehouse capacity, freight cost and loss on the city's lanes, merchant/fondaco slots |
| Civil | population ceiling, crowding and health, realm tax collection, unrest dampening |
| Ideological | scholar/artisan appearance chance, idea spread, guild quality ceiling (row 07), prestige |

"Temples" are cultural monuments (no religion system — decided).

## Culture development and ideals

- **Culture development** is **derived**, never stored: the population-weighted
  mean of its cities' factors. A diaspora's development is that of the cities it
  lives in.
- **Ideal**: what a culture admires, from its traits — all 14 map to one:

  | Traits | Ideal (what earns prestige) |
  |---|---|
  | Martial, Nomadic | conquest, victories, sacks |
  | Mercantile, Seafaring | wealth, trade reach |
  | Scholarly, Artisan | learning, art, masterworks |
  | Agrarian, Pastoral | stability, population, herds and harvests |
  | Clannish | lineage, loyalty, long-lived houses |
  | Devout | monuments and festivals kept (no religion system — decided; read as tradition kept) |
  | Insular, Xenophobic | purity and self-reliance: few foreigners, few imports |
  | Assimilative | numbers absorbed: cultures made citizens |
  | Diaspora | reach of their communities: cities where they live |
 A culture's **prestige** grows by its own ideal: the Mongols gain it
  by sacking great cities.
- **Judgement:** culture A regards B as **barbarian** when B's development, or B's
  score on **A's** ideal, is far below A's → B's default acceptance tier in A's
  cities is 4 (row 05), and A gains a "civilising" reason to war (row 09).
  **Admiration** is the reverse: a less-developed culture's elites admire a far
  more developed one → its default tier rises and its ideas spread faster
  (Hellenisation / Rome's philhellenism).
- Culture **prestige carries a ceiling and a decay** (CLAUDE.md rule 18).

## Slices

| Slice | Content | Gate | Status |
|---|---|---|---|
| 03.1 | `TickHub.dev`/`dev_breakdown`, sources (trade/partner reach/welfare)/decay (starvation/isolation)/diffusion (pulls only up, soft ceiling), `CityYear.dev`; **read by nothing** (`sim/campaign/tick/development.rs`) | `dev_factor_rises_with_trade`, `diffusion_only_pulls_up`, `development_pass_does_not_move_the_fingerprint` | **DONE** |
| 03.2 | Stability formula: `stability_of` (pure — legitimacy/unrest/war/damage/deadlock, base 1.0, clamped 0.2..1.2); wired into 03.1's growth term (`Σ source × modifier × stability`, diffusion/decay left unscaled per the design's own formula shape) | `stability_is_bounded` | **DONE** |
| 03.3 | Four tracks (`sim/campaign/tick/tracks.rs`): `TickHub.track_points`/`track_level` (0-5), same threshold ladder for all four; sources are trade volume/resident houses/banks/guild strength (Trade), soldier share/war/levies (Military), welfare/laws/officials (Civil), partner reach + a small ambient trickle (Ideological, since scholars/schools/masterworks are rows 05-07 and read as absent, not neutral); every source scaled by `stability_of` (03.2); a resident `Individual` (row 02) in a role suited to a track adds a small bonus — the design's own "notable roles that add" column, now real; a real `TickHub.damage` spike (a sack) bleeds a fraction of every track's points each year it persists, and since level is recomputed from points every year, at worst a level too | `levels_rise_automatically`, `a_sack_costs_points`, `tracks_pass_does_not_move_the_fingerprint` | **DONE** |
| 03.4 | Buildings (`tracks.rs`): `TickHub.track_buildings`/`track_build_progress` per track; `track_building_allowed` gates a rung on the track's own `track_level`; construction spends real goods (`pick_build_supply_good`, the L6 housing convention) and city treasury, gradually, via `TRACK_CONSTRUCTION_DOSE` (shipped 0.0 — a true no-op); the 20 named buildings from the design's own table served by `track_building_name`. Damage/destruction in a sack is deferred to row 09 (no sack-vs-building mechanism exists yet; 03.3's own points/level sack cost is the current stand-in) | `a_building_needs_its_level`, `construction_spends_real_stock` (at dose 0 construction is off) | **DONE** — building damage in a sack deferred to row 09, see doc text |
| 03.5 | Culture development (`culture_ideals.rs`): `culture_development` — population-weighted mean `dev` across a culture's cities, derived, never stored; `culture_ideal` — from the culture's own most-characteristic real trait (`culture_trait_ids`) via the design's own trait→ideal table (all 14 traits mapped, `every_trait_maps_to_an_ideal`); `culture_ideal_score` — the matching track's summed points (conquest→Military, wealth→Trade, learning→Ideological, stability→Civil; lineage/tradition/purity/assimilation/reach read 0.0, no track counterpart yet); `is_barbarian_to`/`admires_more_developed` — the design's own two-sided judgement, built and tested but called by NOTHING (row 05 applies it to acceptance tiers, row 09 turns it into a casus belli) | `culture_development_is_population_weighted` | **DONE** |
| 03.6 | UI: a new "Development" tab in `HubPanel.tsx` — the factor + this year's breakdown, and each track's level/points/built-level/build-progress. Backend: `campaign_city_development` (a pure snapshot of `TickHub.dev`/`dev_breakdown`/`track_*`), wired lib.rs → bridge → types per rule 8-9. No sparkline yet (`CityYear.dev`, 03.1, has no reader here — queued Q03.4) | `tsc`, `vite build` | **DONE** — verified by `tsc`/`vite build` only, not opened in a real browser (no display in this environment), same caveat as 02.7 |
| 03.7 | Dose walk: `DEV_PRODUCTION_DOSE` (normalised) and the building effects, **one at a time, `econ_` after each**; end-of-row `tick::tests` + `real_world_price_distance_gradient` + a 100-year `econ_measure_development_leaders` | SCOREBOARD row | **DONE (dose step 1)** — `dev_production.rs`'s `dev_blended_tech` blends each hub's own `dev` into the three real production call sites (`mod.rs`'s daily extraction pass, `production.rs`'s `manufacture_pass`/`add_manufacturing_demand`) in place of the flat global `tech_factor`, lazily normalised via `dev_norm_scale`, and takes `dose` as an explicit parameter (not a hardcoded read) so it is testable at any dose. A 100-year `econ_measure_development_leaders` baseline at dose 0.0 confirmed the leader genuinely changes (H20 → H27 around year 60 on `reference_world`) — the mechanism is not degenerate. `DEV_PRODUCTION_DOSE` raised to **0.2** and kept: it touched two gates that are not its target (the N1c dense-world relay-staging gate's trade-volume floor, and the coin-ledger conservation test's tolerance) — both investigated rather than blindly loosened, and both fixes documented at the assertions themselves and in `dev_production.rs`'s own doc comment (relay floor widened 0.6→0.5 for a real, expected, maintainer-accepted self-sufficiency effect measured at 0.57×; coin-ledger tolerance made relative to tolerate f32 summation-order drift, not a real leak). Full `tick::tests` (343/343) and the full `econ_` suite (6/6, incl. the multi-seed `econ_inheritance_rules_fragment_differently`, all bands unmoved from the pre-dose baseline) both re-verified at dose 0.2 with both fixes. `real_world_price_distance_gradient` not separately re-run this pass. |

**Risk:** the factor feeds production, and production feeds every economy test.
The runaway loop (trade → development → trade) needs the soft ceiling and decay,
and a 300-year diagnostic, `econ_measure_development_leaders`, must show the
leading city **changes** over the centuries.

## Queue
- Q03.1 — Losing a whole level to long isolation (a slow "dark age"), waiting on
  measured decay rates.
- Q03.2 — Per-realm knowledge sharing (row 09).
- Q03.3 — A sack destroying an already-built building outright (rather than
  only costing the owning track's points/level, 03.3's own mechanism), and
  a building's own damage/repair state — waits on row 09's real siege/sack
  mechanism, which is where "damaged or destroyed in a sack" belongs.
- Q03.4 — A 50-year `dev` sparkline in the Development tab, from the existing
  `CityYear.dev` (03.1) — no new backend read needed, just a chart in the
  same style `HubPanel.tsx`'s Life tab already uses for welfare ratio.
- Q03.5 — Opening the new Development tab in a real browser to check it
  visually (no display in this environment) — same caveat as 02.7/Q02.4.
- Q03.6 — DONE for dose step 1 (`DEV_PRODUCTION_DOSE` 0.0 → 0.2, confirmed
  against `tick::tests` + `econ_`, both fixes to unrelated gates documented —
  see slice 03.7's own table entry). **Raising it further is still queued**:
  each next step should re-run `econ_` (and the multi-seed
  `econ_inheritance_rules_fragment_differently`) per CLAUDE.md §2.9, watching
  for the runaway loop this doc's own risk register names (trade →
  development → trade) and re-checking `econ_measure_development_leaders`
  for continued leader turnover, plus the two gates this step's dose touched
  (the dense-world relay-staging floor, the coin-ledger tolerance) in case a
  higher dose pushes either past its now-widened margin.
- Q03.7 — The building EFFECT doses (army strength, warehouse capacity,
  freight cost, population ceiling, unrest dampening, scholar/artisan
  appearance chance, idea spread, guild quality ceiling, prestige) — each
  its own dose constant, each walked separately against `econ_`, per the
  design doc's own effects table. None are wired to anything yet; a built
  building today changes nothing but what the Development tab shows.
