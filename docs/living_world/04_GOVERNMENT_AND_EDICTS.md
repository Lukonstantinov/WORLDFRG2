# 04 · Government and edicts

**Status:** PARTIAL — slice 04.1 done, see §Queue · **Depends on:** 02, 03 · **Next:** 05

**2026-09-25:** started ahead of rows 02/03 at the maintainer's explicit
request. Slice 04.1 (seat-count-by-size + office-title scaffolding) shipped as
pure, dosed-zero scaffolding — `GOV_POWER_DOSE = 0.0` — because a seat is still
an `Official` with a generated name, not yet an `Individual` with a face,
traits and an ideology position (row 02). Slices 04.2 onward (paths,
suitability, allegiance, blocs, debate, the Lustrum, the Government window)
need row 02's `Individual` and are QUEUED below, not built.

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
| 04.1 | **DONE (2026-09-25, scaffolding only).** Seat counts by size (`seat_count_for`, `GOVT_SEAT_CAP`); extra seats beyond the 4 named offices seed as generic role-4 "Councillor" seats; gated behind `GOV_POWER_DOSE = 0.0` (a true no-op — `seed_government` still builds the old fixed 3-4 roles at dose 0). **NOT done**: offices held by `Individual`s (needs row 02) and per-culture title sets (needs a culture-kit index threaded into `TickHub`, which the campaign tick does not carry today — `hub.culture` is a plain generated name) — both QUEUED (Q04.3, Q04.4) | `officials_migrate_to_seats`, `seat_count_scales_with_city` |
| 04.2 | Paths, suitability, allegiance, blocs; houses' kin seats + clients via existing bribery | `bought_members_follow_their_patron` |
| 04.3 | Political points, edict catalogue, costs by ideological distance, expiry | `mismatched_edicts_cost_more`, `edicts_expire` |
| 04.4 | Weekly debate rounds, amendments, filibuster, votes, deadlock | `every_debate_terminates`, `deadlock_costs_legitimacy` |
| 04.5 | Tyrant path, legitimacy, overthrow risk, changes of government, ostracism, exposure | `unpopular_tyrants_fall_more_often`, `ostracism_exiles_one_person` |
| 04.6 | The Lustrum (feeds row 03's track bonus). **Owner in every form:** the Censor (senate), the First Councillor (council), the ruler alone (tyranny), the magistrates proposing to the assembly (assembly); debated like a minor edict except under a tyrant. Before row 04, row 03 picks the bonus track by need | `lustrum_every_five_years` |
| 04.7 | Government window — commands `campaign_get_government`, `campaign_get_edicts` (lib.rs + bridge + types) | `tsc`, `vite build` |
| 04.8 | End of row: edict effects dosed from zero; `tick::tests`, `econ_` | SCOREBOARD row |

`every_debate_terminates` is the analogue of `every_crisis_terminates`
(CLAUDE.md rule 22): no edict may sit in debate forever.

## Queue
- Q04.1 — Realm-level government and realm-wide edicts (row 09).
- Q04.2 — Elections with campaigns (candidates spending, speeches) — waits on
  measured seat turnover.
- Q04.3 — Seat holders as real `Individual`s (face, traits, suitability,
  ideology position) — waits on row 02.
- Q04.4 — Per-culture office title sets (Roman/Hellene/Norse/… from the
  "Forms, sizes and offices" table) — waits on a culture-kit index being
  threaded into `TickHub` (today `hub.culture` is a plain generated name with
  no back-reference to `cultures::KITS`); until then `office_title` serves the
  Roman-flavoured default set for every culture.
- Q04.5 — Slices 04.2-04.8 (paths/suitability/allegiance/blocs, political
  points + edict catalogue, weekly debate, tyrant/legitimacy/coups, the
  Lustrum, the Government window, end-of-row dosing) — waits on rows 02 and 03
  per this row's own stated dependency; do not build them against the
  `Official`-only stand-in above, or the eventual `Individual` migration would
  have to redo this row's own vote/bribery/suitability wiring.
