# Living World — build index (START HERE)

This folder is the agreed design for turning the campaign from an economy with
houses into a **living world**: people with lives, cities that develop, govern
themselves, argue, build, celebrate, persecute, invent, and — in the end — form
states that become empires, war with each other and fall to barbarians.

It was designed in one long conversation with the maintainer (2026-09-25) and is
split into numbered documents so each can be **built in its own chat**. Every
decision recorded here was made by the maintainer; where a document says
"decided", do not re-open it without asking.

## How an AI session picks its work

1. Read this index, then `CLAUDE.md`.
2. Take the **first row below whose status is `NOT STARTED` and whose
   dependencies are all `DONE`**. Do not skip ahead, do not start two rows, and
   **do not start any row while an earlier row is `PARTIAL`** — finish or
   explicitly re-scope it first (a PARTIAL row can silently satisfy a later row's
   dependency list otherwise).
3. Build that document's slices **in order** (`02.1`, `02.2`, …). Each slice
   lists its own narrow gate.
4. Run the **full economy gates only at the end of the row** (see "Testing"
   below), then push to `main`.
5. In the same commit, set the row's status here to `DONE` (or
   `PARTIAL — slices x.y–x.z done, see doc §Queue`) and update `CLAUDE.md`
   maps per its §2.7.

## The rows

| # | Document | Status | Depends on | What it delivers |
|---|---|---|---|---|
| 01 | [`01_FEEDS_AND_PRUNING.md`](01_FEEDS_AND_PRUNING.md) | DONE | — | News Feed window removed; chronicles keep milestones, drop chatter older than 50 years; notable persons' stories are never pruned |
| 02 | [`02_PEOPLE.md`](02_PEOPLE.md) | DONE — all slices 02.1-02.9 shipped (02.7's faces verified by `tsc`/`vite build` only, no display in this environment — see doc §Queue Q02.4) | 01 | ONE Person system: life from debut (≥16) to death, traits, modifiers, the 75 % decision rule, faces, world-linked life events, 40 notables + Hall of the Dead |
| 03 | [`03_DEVELOPMENT_TRACKS.md`](03_DEVELOPMENT_TRACKS.md) | PARTIAL — slices 03.1-03.6 done, 03.7's production-blend mechanism built and shipped at dose 0.0 (a true no-op) but its own dose walk not yet attempted (a deliberately faster, lower-risk session — see doc §Slices/§Queue) | 02 | Per-city development factor (replaces the dead global `tech_factor`), four tracks (Military · Trade · Civil · Ideological), buildings, culture "ideals" and the barbarian judgement |
| 04 | [`04_GOVERNMENT_AND_EDICTS.md`](04_GOVERNMENT_AND_EDICTS.md) | PARTIAL — slice 04.1 done, see doc §Queue | 02, 03 | Government forms, offices (cultural and custom), named seat holders, political points, edicts debated in weekly rounds, the Lustrum, bribery, coups, ostracism, the Government window |
| 05 | [`05_CULTURE_ACCEPTANCE.md`](05_CULTURE_ACCEPTANCE.md) | NOT STARTED | 03, 04 | Five acceptance tiers per city per culture, city stance, persecution and diaspora, bondage attitude, the fondaco switched on |
| 06 | [`06_IDEOLOGY_AND_SCHOLARS.md`](06_IDEOLOGY_AND_SCHOLARS.md) | NOT STARTED | 02, 04, 05 | Four ideology axes (−5…+5), trait-built named ideologies, city meters (nobles · commons · government), scholars, schools, universities, the spread of ideas |
| 07 | [`07_ARTISANS_AND_MASTERWORKS.md`](07_ARTISANS_AND_MASTERWORKS.md) | NOT STARTED | 02, 03, 05 | Artisan kinds, culturally capped guild quality, named masterworks, galleries, the masterwork market, theft and looting, invitations |
| 08 | [`08_LEISURE_AND_GAMES.md`](08_LEISURE_AND_GAMES.md) | NOT STARTED | 02, 03, 04, 05 | Leisure families (every culture, creole and mix picks 3 preferred types), venues by tier, financing, games and festivals, performers, the venue subpanel |
| 09 | [`09_REALMS_WAR_AND_BARBARIANS.md`](09_REALMS_WAR_AND_BARBARIANS.md) | NOT STARTED | 02–05, 07 | Why a realm is worth having, armies, realm-vs-realm war, the conflict map (smoke, ruins, resettlement), deeper provinces, barbarians and their leaders, empires |
| 10 | [`10_SETTLEMENT_OVERVIEW.md`](10_SETTLEMENT_OVERVIEW.md) | NOT STARTED | 03–09 | The settlement Overview as the first panel, linking out to the detailed windows |

Already shipped as part of this conversation (not a row): **ore districts scale
with land area + minor showings** (`8288b3a`, CLAUDE.md §8.16).

**Row 02 was started on `claude/00-index-second-part-tkp497` on explicit
maintainer instruction, while row 01 was NOT YET `DONE`** — a deliberate,
named exception to rule 2's own "do not start a row before its dependency is
DONE", made because a sibling branch/session was assigned row 01 concurrently.
Nothing row 02 shipped so far reads row 01's own output (pruning/the journal
window), so the two do not conflict; if row 01 lands with a different
chronicle shape than row 02 assumed, re-check `individuals.rs`/`life_events.rs`
before building further on top.

**2026-09-25, out-of-order start:** the maintainer separately, explicitly chose
to begin row 04 before rows 02/03 were built (normally forbidden by this
file's own ordering rule above). Slice 04.1 ships what is buildable without
them — seat-count scaling and the office-title scaffolding — with
`Official.name` staying a plain generated name rather than an `Individual`;
see `04_GOVERNMENT_AND_EDICTS.md`'s own Queue for what the rest of the row
still needs. Row 02 (the `Individual` system) has SINCE landed on `main`
(above) — 04.2 onward can now draw on it; 04.1's own scaffolding was not
revisited to use it and still stands as shipped.

## Rules that apply to every row

**Observation only.** The player watches; every choice in these documents is the
simulation's. No player verbs are added.

**Determinism.** Every roll uses the sim's seeded `hash01(seed, tick, key)`, never
a thread RNG and never iteration order of a `HashMap`. Text generation included —
the same save must tell the same story.

**Dose from zero.** Any mechanism that moves wealth, population, prices or
production ships with its effect constant at `0.0` (a true no-op, proven by a
`*_is_a_noop_at_zero` test), and is dosed up only in the row's last slice. This is
the project's existing N1/N6/L-series discipline (CLAUDE.md §5).

**Save compatibility.** Every new field is `#[serde(default)]`; an old save must
load and behave as before until the new pass runs.

**Performance budget.** Nothing here may be a per-day scan over all people or all
cities. Everything is event-driven or runs on a weekly/monthly/yearly cadence over
bounded lists. Target: the whole Living World layer costs **under 1 %** of the
tick (the trade `dispatch` is ~8–10 s per simulated year on a large world; this
layer should be under ~100 ms per year). Check `bench_campaign_tick_large` before
and after each row and write the two numbers in the commit.

**Caps.** Everything that grows has a cap: notables (40 alive), life logs,
chronicles (01), buildings per city, venues, masterworks per city, offices per
city. Prestige-like numbers carry a ceiling and a decay (CLAUDE.md rule 18).

**Chronicle salience.** With ~1,200 cities, only tier 1–2 cities and events above
a salience bar reach anything world-wide; everything else goes to the city's own
history. Person stories always record in full.

**Forward hooks read neutral until their row exists.** Several rows read values
a later row produces (stability reads legitimacy from 04; edict costs read
ideology from 06; events write track points from 03). Until the producing row is
built, the reader uses a documented NEUTRAL constant, and the gate that needs the
real value lives in the producing row. Every doc names its forward hooks.

**Naming — avoid existing symbols.** `Person` is already the realm genealogy
struct (`mod.rs` ~8620) and `Notable` is already the L12 per-city local-role
entry (`mod.rs` ~7080). The new record is **`Individual`**; the 40 famous people
are **"notables"** in the UI but `Individual { famous: true }` in code. The L12
`Notable` stays and links to an `Individual` id.

**Ids and tombstones.** `Individual` ids are never reused. When an ordinary
individual dies they are forgotten (decided) — except that anything still pointing
at them (a masterwork's maker, a school lineage, a relation, a horde) keeps a
**tombstone**: id, name, culture, years, one-word role. No story.

**Cadence hooks.** `advance()` today has daily work, `tick % 30` (monthly) and the
365-tick year — **no weekly step**. Row 02 adds one (`tick % 7`) in a documented
place in the day loop; every weekly pass in later rows uses it.

**Lazy seeding.** Every new per-hub or per-culture state seeds itself on first
read (the existing `*_needs_seeding` convention), so old saves and hubs founded
mid-campaign get it without a migration step.

**Hash salts.** Each new roll uses its own named salt constant from one registry
(`living_world_salts` in `mod.rs`), so dozens of new `hash01` calls cannot collide.

**Storage.** Culture as an index, not a `String`; life-log entries as
`(tick, template_id, args)` with text generated lazily at read time; sparse maps
(e.g. culture relations only for cultures present or trading). Measure the
serialized size in row 01 and after each row.

**Every new command is wired four ways:** `#[tauri::command]` → registered in
`lib.rs` → a `bridge/` wrapper → a `types/` mirror (CLAUDE.md rules 8–9). Each
doc's UI slice lists its commands.

**Queue, don't refuse (CLAUDE.md rule 36).** Anything a row does not build goes in
that document's `Queue` section with what it waits for.

## Testing (decided 2026-09-25)

The maintainer was waiting over an hour for gates on minor changes. For Living
World rows:

- **Per slice:** `cargo check --lib --tests` + the slice's own named tests
  (`cargo test --lib <name>`) + **`living_world_is_inert_at_zero`** (a short
  30-year run of a small fixture with the new layer's passes on vs off; the
  `sim_fingerprint` must match while every dose is zero — seconds, and it is what
  makes skipping `econ_` safe), + `npx tsc --noEmit` for frontend slices. Never the
  whole suite.
- **Once, at the end of the row (before the final push):**
  `cargo test --lib tick::tests` and `cargo test --lib econ_ -- --nocapture`
  (and `simulate_decades_reports_dynamics`, which `tick::tests` includes). If a
  dose was raised in the last slice, these are what judge it.
- Intermediate slices may be committed and pushed without `econ_` **only because
  every behavioural constant is still at zero** — a slice that raises a dose is by
  definition the last slice.
- Inside that last slice, if several doses are walked, run `econ_` **once per dose
  step** (not once for all of them), so a regression can be traced to one dose.
- `CLAUDE.md` §2.9 carries this rule for the rest of the project.

## Independent review (2026-09-25)

A separate review agent read every row against the code and CLAUDE.md. Its
findings were folded in: the new record is `Individual` (both `Person` and
`Notable` already exist in the code); row 01 now replaces the journal's existing
25-year / 12,000-entry trimming (`sample_journal`) instead of adding a second rule
beside it, and flags milestones where entries are written; milestone overflow
becomes per-decade summaries (keeps rule 20); famous people's life logs are never
capped; tombstones keep references valid; a weekly hook, lazy seeding, a hash-salt
registry and compact storage are shared rules above; row 03 absorbs the existing
per-hub `dev_tier` and `hub.structures` and normalises the new factor to today's
production level; every trait maps to a culture ideal; row 04 is dosed
(`GOV_POWER_DOSE`) because capture feeds wars and realms, seeds ideology from
culture traits until row 06, and obeys rules 22/23/27; the 75 % rule is symmetric
per option; the 18 shipped language kits are mapped to leisure families and the
missing Southeast Asian kit is flagged; real dependencies were added (07 → 05,
08 → 03/05, 09 → 07, 10 → 09); a per-slice `living_world_is_inert_at_zero`
fingerprint gate and `econ_` per dose step keep regressions traceable.

**Resolved by the maintainer:** razed cities keep the existing 10-year
resettlement threshold, large or small; barbarian waves use a universal rate
(2–4 a century), not scaled by steppe/frontier area.

## Decisions log (for quick reference — details in each doc)

| Topic | Decision |
|---|---|
| Development factor | Per city + diffusion from trade partners (variant B); visible yearly value, growth and "why" |
| Tracks | Military · Trade · Civil · **Ideological** (diplomatic + cultural merged); leisure venues are a building ladder, not a track |
| Level-ups | Automatic at thresholds; points lost to destruction, war, internal conflict; stability multiplies |
| Culture ideals | Each culture admires something (conquest, wealth, learning, stability, lineage); prestige and "barbarian" judgement follow |
| Ideology | 4 axes, −5…+5, fractional; hybrid axes + traits; custom named ideologies from scholars; spread via trade + scholars + diasporas |
| Government | Tyrant / council / senate / assembly; up to 16 seats; named seat holders; weekly debate rounds; houses and guilds influence; bribes can be exposed |
| Edict pacing | Minor ≈ yearly, major every 25–50 years, the Lustrum (direction edict) every 5 years |
| Notables | 40 alive in total (performers included), no role quotas, later a 20–60 setting; dead notables in a Hall of the Dead |
| Ordinary people | Seat holders etc. are ordinary; forgotten at death unless they became notable |
| Traits | Up to 3 when young, gained over life; shown as tags only (no esteem bar) |
| Decisions | Probability from traits + ±5 % modifiers; ≥ 75 % → always chosen, else a roll |
| Life events | 0–10 a year by circumstance; always a valid template (in-city generic fallback); geography-checked by a lint; ~150 more authored in a separate session |
| Production in stories | Variant A — text from the city's real goods/mines, no ownership |
| Artisans | Invitations variant C (commissions + relocation on push); masterworks buyable, lootable, stealable |
| Leisure | Every culture/creole/mix has 3 preferred types; venues open to all; built by edict; financed by treasury/houses/banks/guilds/liturgy; 2–6 games a year, a major festival every 3–5 years; **no betting money**; unviable venues decline and are abandoned, never converted |
| Slavery | Per-culture bondage attitude, changeable by edict; gladiators from captives only where it is allowed |
| Feeds | News Feed window removed; chatter > 50 years pruned; milestones kept; person stories never pruned |
| Barbarians | Universal rate, 2–4 waves a century; arise from provinces (culture, discontent, opposition to a settlement, unfair trade, ethnogenesis); leaders are notables with goals; 2–4 waves a century; own window |
| Razed cities | Resettled after the existing 10-year threshold (`RESETTLE_COOLDOWN_YEARS`), for all cities |
| Ore | Scales with land area automatically (done) |
