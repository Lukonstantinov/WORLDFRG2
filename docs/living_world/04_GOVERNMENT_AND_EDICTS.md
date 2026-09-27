# 04 · Government and edicts

**Status:** DONE — slices 04.1-04.7 (2026-09-26) plus the remainder of
Q04.9 (all 8 edict families now wired, each behind its own dose, still
shipped at `0.0`) and Q04.5b (5 of 6 "changes of government" kinds +
ostracism, **now walked and LIVE at `GOV_POWER_DOSE = 0.75`** — see this
row's own Q04.5b entry below for the walk) · **Depends on:** 02, 03 ·
**Next:** 09

**2026-09-25:** started ahead of rows 02/03 at the maintainer's explicit
request. Slice 04.1 (seat-count-by-size + office-title scaffolding) shipped as
pure, dosed-zero scaffolding — `GOV_POWER_DOSE = 0.0` — because a seat is still
an `Official` with a generated name, not yet an `Individual` with a face,
traits and an ideology position (row 02).

**2026-09-26:** row 02 (`Individual`) has since landed on `main`, so slice
04.2 (paths, suitability, allegiance, blocs) is now built. Every `Official`
carries `path` (why it sits — kin/military/wealth/guild/scholar/elected/
bribed/appointed), a static `suitability` roll, and an `individual_id`
linking it to a real `Individual` (`ROLE_OFFICIAL`, minted or reused exactly
like `individual_id_for_notable`). `official_allegiance`/`government_blocs`
are pure derived reads over the existing `house`/`kin`/`control` fields — no
new stored duplicate. `seed_government`/`reseat_official` set a fresh seat's
path from its government form (appointed/elected/wealth, or kin when a
family is installed); the existing bribery loop in `update_government` now
also records the path a NEWLY captured seat was actually taken by (muscle
for a fleet/political house, plain coin otherwise) — a purely descriptive
addition, since nothing yet reads `path`/`suitability` to change a vote or a
wealth number, so no dose gate was needed (the same "unconditional
bookkeeping" precedent L4's age pyramid set). Gate:
`bought_members_follow_their_patron`, `government_blocs_group_by_allegiance`.

**Same session, 2026-09-26 — slices 04.3-04.7 also built.** All of the
weekly point-accrual → propose → debate → resolve → expire loop, the
tyrant's own no-vote path, the five-yearly Lustrum, and the Government
window's two commands now exist in `government.rs` and are wired into the
real weekly (`tick % 7`) and yearly cadence hooks. Every part of it is
**mechanism-complete and gated, but deliberately EFFECT-INERT**: an edict's
`family` records what it was proposed as and nothing yet reads that family
to move a tariff, a wall, a wage, or any other real economic/military
number (`Q04.9`, queued rather than built — see below). Because nothing
here touches wealth, population, price or production, none of it needed a
`GOV_POWER_DOSE`-style gate of its own (proven directly by
`government_mechanism_moves_no_wealth_or_production`, which runs the real
weekly pass for three years and diffs the untouched economic snapshot) —
the one place a real coup/regime-change mutation WOULD live
(`maybe_tyrant_decide`'s overthrow risk) is explicitly commented as sitting
behind `GOV_POWER_DOSE` per 04.1's own promise, and does nothing at the
shipped `0.0`.

**Same session, continued — Q04.13 and the first slice of Q04.9.** The
Lustrum now credits its chosen track with real points
(`LUSTRUM_TRACK_BONUS × LUSTRUM_TRACK_BONUS_DOSE`, dose-walked 0.0→1.0), and
a passed Welfare or Foreigners edict now enacts the matching ALREADY-LIVE
standing `Law` (`LAW_GRAIN`/`LAW_FOREIGN_BAR`) behind `EDICT_EFFECT_DOSE`
(also walked 0.0→1.0 in this session). Both walks verified safe by
construction before raising (the Lustrum bonus reaches no live reader since
row 03's own `TRACK_CONSTRUCTION_DOSE` is still 0.0; the two laws are
idempotent and only widen an EXISTING trigger a government can now reach
proactively rather than only after living through the event) and confirmed
by re-running the full `tick::tests` (354/354) and `econ_` (6/6, multi-seed
inheritance gate included) after each raise. See §Slices/§Queue below for
exactly which of 04.5/04.9/04.14's remaining pieces are still open.

**Same session, continued — the rest of Q04.9 and Q04.5b, closing out the
row.** `apply_edict_effect` (renamed from `maybe_enact_edict_law`, which it
now wraps) dispatches all 6 remaining edict families to their own small,
BOUNDED, one-time nudge applied the moment an edict of that family PASSES —
each behind its own dose constant (`EDICT_ECONOMY_DOSE`/`_MILITARY_DOSE`/
`_LEARNING_DOSE`/`_CITIZENSHIP_DOSE`/`_CONSTITUTION_DOSE`/`_BUILDINGS_DOSE`),
each shipped at `0.0`. Economy nudges `mint_fineness` (already eased back
toward `decide_polis_policy`'s own yearly target regardless, so it cannot
accumulate); Military tops up `war_manpower` no further than the same
soldier-pool ceiling `raise_manpower_levy` already enforces; Learning and
Buildings credit `track_points` exactly like Q04.13's own Lustrum bonus;
Citizenship nudges one existing `CultureRelation.score` (row 05); Constitution
adds or removes one generic role-4 seat, bounded by `GOVT_SEAT_CAP`. All 8 of
8 edict families now have a real (if undosed) effect — `edict_family_
effects_are_noops_at_zero_dose` proves every one is exactly inert at the
shipped dose, `edict_family_effects_do_something_at_a_test_dose` proves each
one is a real, bounded mechanism at a nonzero test dose passed explicitly
(never read from the constant internally, the same `dose: f32` parameter
shape `lustrum_bonus_e`/`housing_build_persons_e` already use).

Q04.5b's remaining five "changes of government" kinds (coup was already a
stub; revolution, oligarchic closing, emergency ruler, succession crisis,
reform) plus ostracism are now real, in `government.rs`'s own
`government_change_pass(h, dose)`, called yearly from `update_government`
right after its existing capture/bribery bookkeeping, all behind
`GOV_POWER_DOSE` (still `0.0`, exactly 04.1's own promise for "the NEW
capture/coup rules"). At most one LARGE regime change fires per city per
year, tried in the doc's own rough Polybius order, and a
`GOV_CHANGE_COOLDOWN_YEARS` (15) floor stops a shaky government flipping
form annually; ostracism is independent and small. Every forced installation
(coup, succession crisis) filters the heir through `heir_is_female` for the
culture's `LineRule` (rule 23); nothing here ever touches `Realm`/
`prov_realm`, so a coup in a realm capital replaces the CITY's government
only, by construction (rule 27) rather than a special case. Each mechanism
is tested at a nonzero dose (`a_wealthy_house_can_topple_a_weak_tyrant`,
`a_tyrant_dying_without_an_heir_destabilises_the_city`,
`ostracism_exiles_one_person`) and proven a true no-op in combination at
`0.0` (`government_change_kinds_are_noops_at_zero_dose`, 50 years of the
yearly check on a maximally-triggerable fixture). `GovHistoryEntry` gained
`regime_kind` (`#[serde(default = "neg_one_i8_regime_kind")]`, `-1` for an
ordinary edict/Lustrum entry) so the Government window's history can name a
regime change by kind rather than lumping every one under "coup" — surfaced
in both `GovernmentPanel.tsx` and `HubPanel.tsx`'s Government tab.

Two tests in this batch initially failed for the same reason and are worth
recording: `seed_government` unconditionally resets BOTH `govt_type` (from
population) and `legitimacy` (to `NEUTRAL_LEGITIMACY_SEED`) when called, so
a fixture that set either field on the input `TickHub` *before* calling
`sim()`/`seed_government` had it silently overwritten — the field has to be
set on `s.hubs[h]` *after* `seed_government` runs. Caught immediately by the
new tests actually failing (never firing a coup that should have fired at
overwhelming odds), not by review.

Gates run for this closing pass: `cargo check --lib --tests` (clean, only
pre-existing warnings), the 6 new named tests plus the 14 pre-existing
row-04 tests (20/20), the full `cargo test --lib tick::tests` (396 passed, 0
failed, 5 ignored — `simulate_decades_reports_dynamics` included and
unchanged), and the full `cargo test --lib econ_ -- --nocapture` (see
`docs/SCOREBOARD.md` for the printed numbers) — both required by 00_INDEX.md
since this closes the row. Every new dose constant ships at `0.0`, so none
of this should — and, per the gate results, does not — move a single printed
economy figure.

- **04.3 (political points + edict catalogue).** `TickHub.gov_points`
  accrues weekly at `1/52` per year of `stability_at(h)` (still reading row
  03's own NEUTRAL_LEGITIMACY constant, not the new real `legitimacy` field
  — see Q04.9). Eight edict families (`EDICT_FAM_*`), each with a fixed
  ideology tag; `edict_cost(base, tag, gov_position) = base × (1 +
  |tag − gov_position|)` is the doc's own formula VERBATIM, tested directly
  (`mismatched_edicts_cost_more`) rather than through the whole propose
  path. `gov_position` is seeded ONCE per city from its culture's own most
  characteristic real trait (`culture_ideal`, row 03's existing read) via
  `gov_position_for_ideal` — the forward-hook this row's intro names.
  Family choice: war → Military, famine → Welfare, else a hashed pick
  favouring whatever is cheapest for this government's own lean.
- **04.4 (weekly debate).** `GovDebate` — one active debate per city, never
  under a tyranny. Each week's round leans every seated official toward the
  edict by how close its tag sits to their own allegiance (a house seat
  reads half its patron's tilt, half the government's own; everyone else
  reads the government's), noised by `1 − suitability`; the running tally
  is smoothed round to round. Resolves PASSED/FAILED once `|tally|` clears
  `DEBATE_DECISIVE`, or DEADLOCKED at the form's own `round_cap` (04.4's own
  table — council 1/4, assembly 1/2; "Senate" reuses council's numbers,
  since the code has no fourth `govt_type` yet, Q04.10). Gate:
  `every_debate_terminates` (never more rounds than the cap, whatever the
  tally does).
- **Costs, expiry, legitimacy.** A pass spends the full cost; a fail spends
  `0.4×`; a deadlock spends `0.15×` and costs a small legitimacy hit. A
  passed edict is held until `enacted_tick + 25 or 50 years` (minor/major)
  then dropped (`expire_edicts`, gate `edicts_expire`) — CLAUDE.md's own
  "edicts expire" rule.
- **04.5 (tyrant path, partial).** `maybe_tyrant_decide` — no vote, no
  debate: once points allow, the ruler simply enacts, biased by the SAME
  `pick_edict_family`/`edict_cost` the debate path uses. Opposition is read
  off the hub's existing `mood` sentiment (a real field, not invented) and
  costs legitimacy; the actual OVERTHROW roll and regime-change mutation is
  written as a stub behind `if GOV_POWER_DOSE > 0.0 { … }` and does nothing
  while the dose is zero, per 04.1's own promise. **NOT built**: five of
  the doc's six "changes of government" kinds (revolution, oligarchic
  closing, emergency ruler, succession crisis, imposed, reform — only a
  coup stub exists) and ostracism — queued as Q04.5's own remainder below.
- **04.6 (the Lustrum), Q04.13 done.** `maybe_run_lustrum`, called yearly
  per city, fires exactly once every `LUSTRUM_YEARS` (5), reschedules the
  next (gate `lustrum_every_five_years`), and now CREDITS the trailing
  development track with `LUSTRUM_TRACK_BONUS` (2.0, a quarter of a level)
  × `LUSTRUM_TRACK_BONUS_DOSE` real points — a genuine "benefits to its
  backers" edge, not just a chronicle line. `update_government` runs
  BEFORE `update_food_and_starvation`'s own `update_tracks` call in the
  yearly sequence, so a Lustrum-earned level shows the same year, never a
  year late. Shipped at dose 0.0 first (`lustrum_bonus_is_a_noop_at_zero_
  dose`, a pure `_e`-split test independent of whichever dose is currently
  compiled — the N6/S1 pattern), then walked to 1.0 in the same session
  after `tick::tests` (352/352, incl. `simulate_decades_reports_dynamics`)
  and the full `econ_` (multi-seed inheritance gate included) both stayed
  green. **Why this dose carried essentially no risk, verified rather than
  assumed:** `track_points`/`track_level` are read by nothing live yet —
  `track_building_allowed` (03.4) is itself still dosed at
  `TRACK_CONSTRUCTION_DOSE = 0.0` — so nudging `track_points` cannot reach
  a wealth/production number by any path that exists today; the full gate
  run confirms exactly that rather than trusting the read of the code.
- **04.7 (the Government window).** `campaign_get_government(hub)` (seats
  + blocs + the debate in progress + edicts + history) and
  `campaign_get_edicts(hub)`, wired lib.rs → `bridge/campaign.ts` →
  `types/campaign.ts` (rules 8-9). `ui/campaign/GovernmentPanel.tsx` — a
  floating window (Society menu → "🏛 Government", same seed-from-
  `selectedHub`-then-independent pattern as Markets) — a deliberately
  PLAIN first cut: header stats, a seat table, edicts in force, recent
  history. **NOT built** (Q04.14): seat portraits, a live round-by-round
  debate timeline (the tally shows as one number, not animated), and the
  doc's own richer "portraits grouped by bloc" layout. Verified by
  `npx tsc --noEmit` (clean) and `npx vite build` (clean, 189 modules).

**Same session, continued — the Government tab inside the settlement window
itself.** `HubPanel.tsx`'s existing "Government" tab (DLC 3 polis, driven by
`HubDetail.government`) predates row 04 and never carried any of it —
seats read only a plain name/status, with no path/suitability/allegiance,
no debate, no edict catalogue. Its leader/council/regime/officials block is
now REPLACED by a fetch of `campaignGetGovernment(hub)` (the same
`GovernmentBrief` the floating window reads): the real form/legitimacy/
points/lean header, the debate in progress if any, every seat with its
real path/suitability/allegiance, edicts actually in force, and recent
history. The tab's other sections — family influence, charters, fiscal
policy (tariff/mint), laws & decrees, treasury & stores, speculation,
culture acceptance — are UNCHANGED: they read a genuinely different,
still-live mechanism (`decide_polis_policy`'s yearly tariff/mint decision,
the charter/speculation engines) that row 04 does not supersede, so
deleting them would have thrown away real working information for no
reason. Frontend-only (no Rust touched): `npx tsc --noEmit` and `npx vite
build` (189 modules) both clean.

Gates run for 04.3-04.7: `mismatched_edicts_cost_more`,
`every_debate_terminates`, `edicts_expire`, `lustrum_every_five_years`,
`government_mechanism_moves_no_wealth_or_production`, plus the pre-existing
`officials_migrate_to_seats`/`seat_count_scales_with_city`/
`bought_members_follow_their_patron`/`government_blocs_group_by_allegiance`/
`living_world_is_inert_at_zero` re-verified clean. Per CLAUDE.md §2.9, the
end-of-batch `tick::tests`/`econ_` run is recorded in this session's commit.

## Goal

Every city is governed by named people whose ideology, allegiances, competence and
character decide what the city does. Decisions come as edicts that build up over
time, are debated over weeks and are voted on, or are imposed by a single ruler at
a risk. Houses and guilds buy influence. Governments rise and fall.

## Decisions (maintainer)

- **Forms:** one decider (tyrant, prince) **or** a body (council, senate,
  assembly). Houses and guilds influence all of them, and can **overthrow** any.
- **Seats scale with city size, up to 16** for the largest senates.
- **Seat holders are named people** (row 02) — faces, a short life line saying
  **why** they sit (war hero, kin of a house, bought, elected), political
  prestige, ideology, **suitability** (static) and character traits that affect
  their votes. Low suitability → unwise, noisy, bribable decisions.
- **Blocs** are shown as well (grouping by allegiance).
- **Guilds** hold representative seats; a large city has one person speaking for
  all guilds. **Houses** act independently: kin in seats plus bribed clients.
- **Edicts** are tagged **neutral / conservative / libertarian**; matching the
  government's ideology is cheaper, opposing it is dearer but still possible
  (bribes, events). **Edicts expire.**
- **Pacing:** points accrue **weekly** but slowly — a minor edict about **once a
  year**, a major one every **25–50 years**, and every **5 years** a
  **direction edict** (the Lustrum) that adds points to one development track
  with benefits to its backers.
- **Debate:** a body debates an edict over **several weekly rounds** until it
  decides; failure costs some of the points.
- **Tyrant:** unpopular decisions (the nobles are against) raise the chance of
  overthrow — lowered by supportive kin advisers, high prestige, a loyal guard.
- **Bribes can be exposed**, harming the house.
- **Ostracism** is included. **Liturgies** (the rich obliged to fund public
  things) are possible under **any** government.
- **Scope: cities only** for now; realms later (row 09).
- The Government view gets **its own panel, like the war panel**.

## What exists today (verified)

- `Official { role, name, house, control, kin, term_end }`, 4 per city: Head ·
  Treasurer · Harbormaster · Magistrate (`office_title`).
- `govt_type_name`: 0 Merchant Council (Doge) · 1 Principality (Prince) · 2 Free
  Commune (Mayor).
- `update_government` (`mod.rs` ~8985): seeding, term-end reseating, yearly
  bribery (cash) / intimidation (prestige + ships) by the strongest patron house,
  capture by a control-weighted majority, a favoured-house charter on capture, the
  civic granary.
- `Law` with 8 kinds (`LAW_*`), `enact_standing_laws`, `decide_polis_policy`
  (tariff/mint/treasury).
- The house crisis engine (`crisis.rs`), realm succession, the Enthrone war goal.

Row 04 **extends** these: `Official` → an office held by an `Individual`; the
bribery/control code becomes the influence engine for votes; `Law` becomes the
edict record.

**Migration maps:** old `govt_type` 0 Merchant Council → Council; 1 Principality
→ Tyranny/principality; 2 Free Commune → Assembly (a large free commune may become
a Senate by a reform edict). `hub.officials` stays readable through a shim —
`development_tier` and house capture read it.

**This row is NOT inert by construction:** capture (`council_house` /
`captor_house`) feeds war declarations (`maybe_declare_war`), charters and realm
formation (path A). So the NEW capture/coup rules (04.2, 04.5) sit behind
`GOV_POWER_DOSE`; at 0.0 the old `update_government` capture logic runs unchanged
and the new machinery only records what it *would* do.

**Forward hook:** edict costs and votes read ideology (row 06). Until row 06 the
government's position is **seeded from its culture's traits** (Insular/Xenophobic
→ Openness −, Mercantile → Economy +, Clannish → Authority −…) and seat holders'
positions from their traits, so costs genuinely differ from day one; the full
meters arrive in row 06. The commons-meter terms (revolution, assembly votes) read
a neutral constant until then.

## Forms, sizes and offices

| Form | Decider | Seats (by city tier) |
|---|---|---|
| Tyranny / principality | One ruler, advised | 1 + 2–4 advisers |
| Council (oligarchy) | Majority of the council | 5–9 |
| Senate (republic) | Majority, with vetoes | 9–16 |
| Assembly (democracy) | The commons, led by magistrates | magistrates 3–10; the assembly itself is the commons' meter (row 06), not seats |

Villages have 1–3 seats (an elder or a headman). **Custom offices count inside
the same cap** (≤ 16 including them): creating one either uses a free slot or
replaces the least important office — itself a debate.

**Offices** (Roman-flavoured examples; every culture names them in its own kit):

| Tyranny | Council | Senate | Assembly |
|---|---|---|---|
| Tyrant / Prince | Doge / First Councillor | Two Consuls (each can veto the other) | Chair for a day (chosen by lot) |
| Chancellor / Vizier | Councillors | Princeps Senatus (speaks first, sets the agenda) | Ten Generals (elected, re-electable) |
| Master of the Guard | Treasurer | Censor (runs the Lustrum, can expel senators) | Archons (by lot) |
| Treasurer | Harbormaster | Tribune of the People (veto) | Council of 500 (the agenda) |
| Spymaster | Magistrate | Aedile (games, markets, streets) | — |
| Favourite / court poet | Guild Prior | Praetor (courts) · Quaestor (treasury) | — |
| Heir | Advocate of the commons | Dictator (emergency, a term) | — |

**Cultural title sets** from the culture's naming kit: Greek-like (*archon,
strategos*), Latin-like (*consul, tribune*), Italian maritime (*doge, podestà*),
steppe (*khan, tarkhan, noyan*), Norse-like (*jarl, lawspeaker*), Chinese-like
(*prefect, censor, grand secretary*), Persian-like (*shah, vizier, satrap*),
Egyptian-like (*vizier, nomarch*), Slavic-like (*knyaz, voivode, posadnik*),
Southeast-Asian-like (*raja, mandarin, laksamana*), and so on.

**Custom offices** are created by edicts or unlocked by buildings: aqueduct →
Curator of Waters; library → Keeper of the Library; fondaco → Warden of the
Fondaco; grain law → Prefect of the Grain; arena → Master of the Games; walls →
Warden of the Walls. Each is a seat with powers over its own area — and one more
seat for houses to capture.

## Seat holders

Each seat holds an `Individual` (ordinary unless notable), with:
- **Path** — why they sit: kin of a house · military success · wealth · guild
  representative · scholar/orator · elected by the commons · bribed in · appointed
  by the ruler. It sets the life line and the typical suitability.
- **Suitability** (0–1, **static**): noise in their votes, how easily they are
  bribed, chance of a foolish act ("Senator Druso, drunk, voted for the wrong
  motion").
- **Political prestige** — how many others follow their lead.
- **Allegiance** — house / guild / commons / ruler / none.
- **Ideology** (row 06) and traits (row 02).

Dead ordinary seat holders are forgotten; senior office holders get one line in
the city's chronicle.

## Political points and edicts

- `gov_points` accrue **weekly**: base × stability × the government's prestige.
- Costs: minor edict ≈ one year of points; major ≈ 25–50 years; **the Lustrum** is
  paid by a separate 5-yearly allowance.
- **Cost of a specific edict** = base × (1 + distance between the edict's
  ideology position and the government's position). So in a libertarian senate, a
  libertarian edict is cheap and a conservative one dear.

**Edict families** (each with an ideology position, a tag and an expiry):

| Family | Examples |
|---|---|
| Citizenship and culture | grant/revoke a tier (row 05), expel a culture, protect a culture, permit/abolish bondage |
| Foreigners | charter a fondaco, foreigner surtax, bar foreign ownership (exists) |
| Learning | fund a school, found a university, exile a philosopher |
| Welfare | grain dole / grain law (exists), public games (row 08), liturgy |
| Economy | tariff, free harbour, guild monopoly (exists), a mint reform |
| Military | raise walls, levy, hire mercenaries |
| Constitution | create or abolish an office, widen or close the senate, emergency dictator |
| Buildings | build a track building (row 03) or a venue (row 08) with its financing |
| The Lustrum | which track gets the 5-year bonus |

## The flow, week by week

1. **Week 0 — proposal.** When points allow, the **agenda setter** (the ruler,
   the Princeps/First Councillor, or the strongest bloc) proposes. The pick is
   weighted by open **issues** (plague → health/foreigners; war → levy; famine →
   grain), the setter's ideology, house lobbying, and a resident scholar's doctrine.
2. **Weeks 1…N — rounds.** One round per week. In each round:
   - orators persuade (moves other members' positions toward theirs, by prestige);
   - houses bribe (raises a member's lean, adds exposure risk);
   - the proposer may **amend** (weakens the edict, lowers its cost, wins votes);
   - a member may **filibuster** (a round passes with no progress);
   - members may abstain.
3. **Vote.** Each member votes by alignment + allegiance + bribes + suitability
   noise + a hashed roll. **Pass** → points spent, edict in force until expiry.
   **Fail** → part of the points lost. **Deadlock** at the round limit → shelved,
   legitimacy falls.

Round limits by form:

| Form | Minor | Contested | Major |
|---|---|---|---|
| Tyranny | 1–2 weeks of consultation, then the ruler decides | — | — |
| Council | 1 | 2–3 | up to 4 |
| Senate | 1 | 2–4 | up to 6–8 |
| Assembly | 1 (can **reverse** itself the next week — the Mytilene effect) | 1–2 | 2 |

Historical grounding: the Athenian assembly decided in a day (and in 427 BC
reversed a massacre order the next day); the Roman Senate usually decided in one
sitting, but Cato could talk until sunset to block a vote; electing a doge of
Venice took days of alternating lot and ballot; the 1268–71 conclave at Viterbo
took nearly three years.

## Tyrants, legitimacy and change of government

- **Tyrant decisions** skip the vote. An edict the nobles or commons oppose adds
  **overthrow risk** in proportion to the opposition, reduced by kin advisers in
  office, prestige, a loyal Master of the Guard, Military level, recent victories
  and games.
- **Legitimacy** per government: rises with time in power, edicts that match the
  commons, victories, masterworks and games; falls with famine, defeat, exposed
  bribes, deadlock and mismatch.
- **Changes of government** (Polybius' cycle as a pressure, never a script):

| Change | Trigger |
|---|---|
| Coup | a house or commander with wealth/fleet/army vs a low-legitimacy government |
| Revolution | commons' meter far from the government + unrest + a demagogue |
| Oligarchic closing | a long-ruling council with rising rich houses (Venice's *Serrata*, 1297) |
| Emergency ruler | war or plague; the body grants one person power for a term; they may keep it |
| Succession crisis | a tyrant dies without an accepted heir (reuse `crisis.rs`) |
| Imposed | a war's Enthrone goal (exists) or a realm (row 09) |
| Reform | a body votes to change the constitution, pushed by scholars (Solon, Cleisthenes) |

- **Ostracism** (assemblies): once a year the assembly may vote to exile one
  person for 10 years.
- **Rules that must hold:** a forced installation (coup, emergency ruler, tyrant
  succession) filters candidates through `heir_is_female` for the culture's
  `LineRule` (CLAUDE.md rule 23); a coup in a **realm capital** replaces the
  city's government only, never the crown (rule 27); an emergency ruler's term has
  a hard maximum and every coup/revolution resolves within a bounded number of
  weeks (rule 22's discipline).
- **Bribery exposure**: chance rises with bribe size, number involved, rival houses
  watching; exposure costs the house prestige and the government legitimacy, and
  may expel the member.

## UI — the Government window (like the war panel)

Header (form, ruler or presiding officer, legitimacy, stability) · seats as
portraits grouped by bloc, each with path, suitability, allegiance, ideology dot ·
**the debate in progress** as a round timeline with the moving tally · edicts in
force with expiry · recent history (passed, failed, deadlocked, coups).

## Slices

| Slice | Content | Gate |
|---|---|---|
| 04.1 | **DONE (2026-09-25, scaffolding); dose walked LIVE 2026-09-27 (Q04.5b).** Seat counts by size (`seat_count_for`, `GOVT_SEAT_CAP`); extra seats beyond the 4 named offices seed as generic role-4 "Councillor" seats, now scaling CONTINUOUSLY with `GOV_POWER_DOSE` (shipped `0.75`) rather than as an on/off latch. **NOT done**: per-culture title sets (needs a culture-kit index threaded into `TickHub`, which the campaign tick does not carry today — `hub.culture` is a plain generated name) — QUEUED (Q04.4); offices held by `Individual`s landed this same pass for the extra role-4 seats specifically (a resident scholar/philosopher/ideologue, `PATH_SCHOLAR`), the named offices' own generic-name minting is unchanged | `officials_migrate_to_seats`, `seat_count_scales_with_city` |
| 04.2 | **DONE (2026-09-26).** Paths (`PATH_*`), a static `suitability` roll, an `individual_id` linking each seat to a real `Individual` (`ROLE_OFFICIAL`); `official_allegiance`/`government_blocs` as pure derived reads over the existing house/kin/control fields; the existing bribery loop now records a newly-captured seat's path (military vs bribed). Undosed — purely descriptive, moves no wealth/production | `bought_members_follow_their_patron`, `government_blocs_group_by_allegiance` |
| 04.3 | **DONE.** Weekly `gov_points` accrual, 8 edict families (`EDICT_FAM_*`), `edict_cost = base × (1 + \|tag − gov_position\|)` (the doc's own formula), family choice by open issue (war/famine) or hashed lean-affinity pick. `gov_position` seeded once per city from `culture_ideal`. **All 8 of 8 families now have a real effect** (`apply_edict_effect`), each behind its own dose, each shipped `0.0` — Welfare/Foreigners → `LAW_GRAIN`/`LAW_FOREIGN_BAR` (`EDICT_EFFECT_DOSE`); Economy → `mint_fineness` nudge; Military → `war_manpower` top-up bounded by the existing levy ceiling; Learning/Buildings → `track_points`; Citizenship → a `CultureRelation.score` nudge (row 05); Constitution → create/abolish one role-4 seat, bounded by `GOVT_SEAT_CAP`. Raising any of the 6 new doses is queued (Q04.9) | `mismatched_edicts_cost_more`, `edict_effects_are_a_noop_at_zero_dose`, `passed_edicts_enact_their_matching_law`, `edict_family_effects_are_noops_at_zero_dose`, `edict_family_effects_do_something_at_a_test_dose` |
| 04.4 | **DONE.** Weekly debate rounds — allegiance+suitability-noised lean, smoothed tally, resolves PASS/FAIL/DEADLOCK within the form's own `round_cap`. Amendments/filibuster/vote-exposure folded into one persuasion-noise term rather than three separate mechanics (documented scope cut, Q04.11) | `every_debate_terminates` |
| — | Costs (full/0.4×/0.15× pass/fail/deadlock) + expiry (25/50-yr minor/major) + a small legitimacy swing, all shipped alongside 04.3-04.4 | `edicts_expire` |
| 04.5 | **DONE; dose walked LIVE 2026-09-27 (Q04.5b).** Tyrant path (`maybe_tyrant_decide`, no vote) + opposition/legitimacy bookkeeping off the real `mood` field. All 5 remaining "changes of government" kinds — revolution, oligarchic closing, emergency ruler, succession crisis, reform — **and ostracism**, all in `government_change_pass(h, dose)`, called yearly, at the real shipped `GOV_POWER_DOSE = 0.75` (walked `0.0 -> 1.0` — broke the wealth-bound gate — `-> 0.75`, shipped; see Q04.5b's own entry for the full walk). At most one LARGE kind fires per city per year (Polybius order, first match wins), bounded by `GOV_CHANGE_COOLDOWN_YEARS` (15); a forced installation always filters through `heir_is_female` (rule 23); nothing here ever touches `Realm`/`prov_realm` (rule 27) | `government_change_kinds_are_noops_at_zero_dose`, `a_wealthy_house_can_topple_a_weak_tyrant`, `a_tyrant_dying_without_an_heir_destabilises_the_city`, `ostracism_exiles_one_person`, `government_dose_one_over_a_century` |
| 04.6 | **DONE.** `maybe_run_lustrum` fires every `LUSTRUM_YEARS`, picks the trailing track, records it to history/chronicle, and CREDITS it `LUSTRUM_TRACK_BONUS × LUSTRUM_TRACK_BONUS_DOSE` real points (Q04.13, dose-walked 0.0→1.0, `tick::tests`+`econ_` both green before/after) | `lustrum_every_five_years`, `lustrum_bonus_is_a_noop_at_zero_dose`, `lustrum_bonus_credits_exactly_the_trailing_track` |
| 04.7 | **DONE, plain first cut; seat portraits added 2026-09-27.** Government window — `campaign_get_government`, `campaign_get_edicts` (lib.rs + bridge + types), `ui/campaign/GovernmentPanel.tsx`; recent history now names a regime change by kind (`regime_kind`) instead of lumping it under "coup". A seated `Individual`'s bust + click-to-expand life story now shows in the Government TAB (`HubPanel.tsx`, merged with the old Development tab this same pass) and the Ideology section's scholar list (`SettlementOverviewTab.tsx`), via a shared `PersonChip` (`ui/campaign/personShared.tsx`) — a Scholar/Philosopher/Ideologue (or a `PATH_SCHOLAR` seat) tints blue. A government-FORM badge (icon + colour per regime, `ui/campaign/govFormBadge.tsx`) reads at all three "Form" call sites (`HubPanel`, `GovernmentPanel`, `SettlementOverviewTab`). **NOT built** (Q04.14): portraits in the floating `GovernmentPanel.tsx`'s own seat table, a live round timeline, bloc-grouped layout | `tsc`, `vite build` (clean) |
| 04.8 | **DONE — end of row.** All 8 edict families' effects wired (dosed at 0.0, Q04.9's dose walk queued); all 6 "changes of government" kinds + ostracism built and **now LIVE at `GOV_POWER_DOSE = 0.75`** (Q04.5b, 2026-09-27); `tick::tests` (734+/735, `simulate_decades_reports_dynamics` re-verified bounded at the shipped dose), `econ_` (see SCOREBOARD row) | SCOREBOARD row |

`every_debate_terminates` is the analogue of `every_crisis_terminates`
(CLAUDE.md rule 22): no edict may sit in debate forever.

## Queue
- Q04.1 — Realm-level government and realm-wide edicts (row 09).
- Q04.2 — Elections with campaigns (candidates spending, speeches) — waits on
  measured seat turnover.
- Q04.3 — **DONE 2026-09-26** (seat holders now carry a real `Individual` via
  `individual_id` — face/traits/ideology-position reads follow from that link
  once row 06 gives ideology a real meter; traits are already on `Individual`
  from row 02 but nothing here reads them yet, since suitability is currently
  its own static roll rather than trait-derived — a future slice may fold the
  two together).
- Q04.4 — Per-culture office title sets (Roman/Hellene/Norse/… from the
  "Forms, sizes and offices" table) — waits on a culture-kit index being
  threaded into `TickHub` (today `hub.culture` is a plain generated name with
  no back-reference to `cultures::KITS`); until then `office_title` serves the
  Roman-flavoured default set for every culture.
- Q04.5 — **DONE 2026-09-26** (04.3-04.7's mechanism, and 04.6's Lustrum
  bookkeeping, were built in the same window row 03 itself reached `DONE`
  on `main` — a deliberate, named exception mirroring row 04's own original
  out-of-order start, because everything built is EFFECT-INERT: no edict
  yet pays out anything, so there is no live number to get wrong regardless
  of row 03's own state). What remains queued is 04.8's real dosing pass —
  see Q04.9/Q04.13 below — both now unblocked by row 03's dependency, left
  for session budget alone.
- Q04.5b — **DONE, dose walked and shipped LIVE at `GOV_POWER_DOSE = 0.75`
  (2026-09-27, in the same session that merged the Government/Development
  tabs and added the government-form badge and seat figurines).** All 5 of
  6 "changes of government" kinds (revolution, oligarchic closing,
  emergency ruler, succession crisis, reform — "imposed" already existed)
  plus ostracism are built in `government_change_pass`, gated by
  `government_change_kinds_are_noops_at_zero_dose`,
  `a_wealthy_house_can_topple_a_weak_tyrant`,
  `a_tyrant_dying_without_an_heir_destabilises_the_city`,
  `ostracism_exiles_one_person`, plus a new integration-level gate,
  `government_dose_one_over_a_century`, that drives the REAL yearly call
  site (`government_change_pass(h, GOV_POWER_DOSE)` inside `advance`'s own
  loop, never the sub-functions called directly) for 150 years across 16
  mixed-size hubs and asserts a real regime change lands somewhere, with
  `legitimacy`/`gov_position` staying finite and in bounds.
  **The walk: `0.0 -> 1.0 -> 0.75`.** `1.0` was tried first (every
  sub-mechanism already carried its own probability cap and was already
  tested at dose `1.0`) and BROKE `simulate_decades_reports_dynamics`'s
  bounded-wealth floor — sustained minimum wealth read **−29,364.65**
  against the −500.0 floor, a genuine runaway, not a marginal miss.
  Bisected by isolating the two mechanisms this one constant gates: with
  regime-change forced to an explicit `0.0` and only 04.1's seat-count
  scaling live, the floor held (min −99.0); with seat-count scaling forced
  off and only regime-change live at `1.0`, the floor also held (min
  −192.9). **Neither mechanism alone reproduces the failure — only the two
  TOGETHER do**, almost certainly because `update_government`'s per-seat
  yearly bribery spend (step 3, which runs over every seat regardless of
  dose) compounds with the extra churn `government_change_pass`'s
  `reseat_official` calls introduce across a now-larger council. Since
  04.1's seat-count scaling was a hard on/off LATCH (`if GOV_POWER_DOSE >
  0.0`, not a fraction of it), no value between `0.0` and `1.0` could have
  been tested by "backing off the dose" until that was fixed: it was
  rewired to scale CONTINUOUSLY with the dose fraction
  (`((full_extra_seats as f32) * GOV_POWER_DOSE.min(1.0)).round()`),
  matching how every other dose in this codebase already behaves. Re-walked
  at `0.5` (min −166.4, sustained-late-richest 528,805 — held) and `0.75`
  (min −144.2, sustained-late-richest 381,736 — held, comfortable margin on
  both bounds, no worse than `0.5`'s). **Shipped at `0.75`**: real
  government churn and a genuinely size-scaled senate, without the
  runaway. `officials_migrate_to_seats` was rewritten to assert the
  DOSE-SCALED seat count (via the same rounding formula) rather than a
  fixed prediction, since the shipped dose is no longer exactly `1.0`. Full
  numbers (all four intermediate runs) at `GOV_POWER_DOSE`'s own doc
  comment in `tick/mod.rs` and in `docs/SCOREBOARD.md`'s 2026-09-27 entry.
  **Also wired in the same pass**: an extra (role-4) seat now prefers a
  resident scholar/philosopher/ideologue `Individual` over a freshly minted
  generic name (`seed_government`, reusing `maybe_reform`'s own "a resident
  scholar" lookup), installed with `path = PATH_SCHOLAR` — the first real
  assignment site for `official_path_name`'s long-dead-code
  "scholar/orator" string.
- Q04.9 — **DONE (mechanism), dose walks still queued.** All 8 of 8 edict
  families now have a real effect via `apply_edict_effect`: Welfare/
  Foreigners → `LAW_GRAIN`/`LAW_FOREIGN_BAR` (`EDICT_EFFECT_DOSE`, already
  walked 0.0→1.0 — see the entry above); Economy → a bounded `mint_
  fineness` nudge (`EDICT_ECONOMY_DOSE`); Military → a `war_manpower`
  top-up capped at the SAME ceiling `raise_manpower_levy` already enforces
  (`EDICT_MILITARY_DOSE`); Learning/Buildings → `track_points`, the same
  shape as Q04.13's Lustrum bonus (`EDICT_LEARNING_DOSE`/`_BUILDINGS_
  DOSE` — Learning now has somewhere real to land, rows 06/07 having
  since shipped); Citizenship → one `CultureRelation.score` nudge, row 05
  (`EDICT_CITIZENSHIP_DOSE`); Constitution → create/abolish one role-4
  seat bounded by `GOVT_SEAT_CAP` (`EDICT_CONSTITUTION_DOSE`). All 6 new
  doses ship at `0.0` and are gated by `edict_family_effects_are_noops_
  at_zero_dose`/`edict_family_effects_do_something_at_a_test_dose`.
  **Walking any of them is unstarted, separate work** — 00_INDEX's own
  rule is one dose step at a time, each with its own `econ_` run, and six
  at once in one sitting is exactly the "three doses is the ceiling" risk
  this codebase's plans keep naming.
- Q04.10 — A genuine fourth `govt_type` (Senate, distinct from Council) —
  waits on deciding what actually distinguishes it mechanically (the doc
  names vetoes and a widened round cap; today "Senate" is Council's own
  numbers, a documented scope cut).
- Q04.11 — Split persuasion/bribery/filibuster/amendment into distinct
  debate-round actions (folded into one noise term this session) — waits on
  a reason to need the distinction (e.g. bribery EXPOSURE, which needs its
  own actor and its own house-prestige cost).
- Q04.12 — Read the HEAD seat's own character (once `Individual.traits`
  informs `suitability`/decisions, mirroring `head_character_factor`'s
  pattern) for the tyrant's decisions, instead of the government's flat
  `gov_position` — waits on Q04.3's own "fold suitability into traits"
  follow-up.
- Q04.13 — **DONE 2026-09-26.** `LUSTRUM_TRACK_BONUS_DOSE` walked 0.0→1.0,
  `tick::tests`/`econ_` both green before and after. Verified low-risk by
  construction, not just by the gate: `track_points`/`track_level` have no
  live downstream reader today (`TRACK_CONSTRUCTION_DOSE` is itself still
  0.0), so the walk could only ever move things this row itself tracks.
- Q04.14 — The Government window's richer layout: seat portraits, a live
  round-by-round debate timeline, bloc-grouped seats — waits on session
  budget alone, no structural blocker.
