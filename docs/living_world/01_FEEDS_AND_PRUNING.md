# 01 · Feeds and pruning

**Status:** NOT STARTED · **Depends on:** — · **Next:** 02

## Goal

Remove the world News Feed window, and keep every other chronicle small by
dropping ordinary chatter older than 50 years while keeping milestones. A notable
person's life story is **never** pruned — the maintainer reads them as stories.

## Decisions (maintainer)

- The **News Feed window is deleted.**
- **Variant B:** chatter older than 50 years is deleted; **important milestones are
  kept** (still capped in number).
- **Person histories are untouched** — never pruned.
- Ordinary people who die are forgotten (that is row 02's concern; this row only
  guarantees their pruning does not touch notables).

## What exists today (verified 2026-09-25)

| Feed | Where | Current cap |
|---|---|---|
| World journal | `CampaignSim.journal: Vec<JournalEntry>` | **already pruned**: `sample_journal` (`mod.rs` ~11069) keeps a rolling **25-year** window of everything, keeps older entries only where `is_milestone_kind` (`mod.rs` ~11141 — drops only price/world/voyage_loss), then hard-caps at **12,000** (dropping the oldest, milestones included). `JOURNAL_CAP` = 20,000 is a separate cap |
| House chronicle | `House.events: Vec<HouseEvent>` | `HOUSE_EVENTS_CAP` = 60 chatter, `HOUSE_MILESTONE_CAP` = 120 milestones (`is_house_milestone`) |
| Province chronicle | `CampaignSim.prov_events: Vec<Vec<ProvEvent>>` | `PROV_EVENTS_CAP` = 40 per province |
| Realm chronicle | `Realm.events: Vec<RealmEvent>` | uncapped |
| Feud log | `Feud.log: Vec<FeudFlare>` | per feud |
| City annals | `TickHub.annals: Vec<CityYear>` | `ANNALS_CAP` = 300 — **numeric yearly data, not a feed; keep as is** |
| Figures' life events | `read_people.rs::life_events_for` — **built at read time from `sim.journal`** | — |

Journal readers: `HubPanel` chronicle, `HousesPanel` pulse ticker, `FiguresPanel`
(via `life_events_for`), **`AtlasPanel`** — all must still work.

**This row REPLACES `sample_journal`'s rule, it does not add a second one beside
it.** Kinds are too coarse to classify milestones (`"war"` covers both a
declaration and every round; `"disaster"`/`"event"` cover many things), so a
`JournalEntry.milestone: bool` (`#[serde(default)]`) is set **at the write site**
of each entry, and pruning reads that flag.

**The trap:** a figure's life story is not stored on the figure; it is re-derived
from the world journal each time the panel opens. Pruning the journal to 50 years
would therefore silently erase the youth of every long-lived figure. This row must
**materialise** figure life logs before it prunes anything.

## Slices

### 01.1 · Materialise figure life logs (no behaviour change)
- Add `Figure.life_log: Vec<LifeEntry>` (`#[serde(default)]`), `LifeEntry { tick,
  kind, text }`.
- Whenever a journal entry is written that `life_events_for` would pick up for a
  living figure (same role filter, `role_journal_kinds`), also push it to that
  figure's `life_log` — **not pruned** (decided: person stories are never pruned).
  It is naturally bounded (≤ 10 events a year × a life) and stored compactly as
  `(tick, template_id, args)` from row 02 on.
- Backfill on load: if `life_log` is empty and the journal holds entries,
  build it once with the existing `life_events_for` logic.
- `read_people.rs` reads `life_log` first, falls back to the journal scan.
- Row 02 migrates `Figure` into `Individual` and carries `life_log` across.
- **Gate:** `figure_life_survives_journal_pruning` — build a figure, write journal
  entries across 120 years, prune the journal, assert the figure's story is intact.

### 01.2 · Milestone flags at the write sites
- `JournalEntry.milestone` set where each entry is written — founding and death
  of cities, wars declared/ended, sacks, realm proclaimed/fallen, bank collapse,
  plague outbreak, masterwork created, notable born/died.
- Reuse `is_house_milestone` for houses (unchanged).
- Province: revolt, holder change, work completed, realm change.
- Realm: proclaimed, succession, partition, war outcome, capital move, fall.
- Feud: formation and ending.
- **Gate:** `milestone_kinds_cover_every_permanent_event` — a table test listing
  the kinds above.

### 01.3 · The pruning pass
- `CHRONICLE_KEEP_YEARS: u32 = 50`.
- `prune_chronicles(&mut self, yr)` yearly, after the year's other passes:
  - journal: drop entries older than 50 years **unless** flagged milestone;
    replaces `sample_journal`'s 25-year/12,000 rule;
  - **milestone overflow** (a milestone cap is reached — the existing
    `HOUSE_MILESTONE_CAP` = 120, a journal milestone cap): the oldest milestones
    are **folded into one summary line per decade** ("In the 210s: 3 successions,
    a charter, the plague") rather than silently dropped — this keeps CLAUDE.md
    rule 20 (milestones are permanent) true in substance;
  - house events: drop non-milestones older than 50 years (caps unchanged);
  - province and realm events and feud logs: same rule; `Realm.events` gets a cap
    (it is unbounded today);
  - never touch any `life_log`.
- **Gates:** `pruning_keeps_milestones`, `pruning_drops_old_chatter`,
  `pruning_never_touches_person_stories`.

### 01.4 · Remove the News Feed window
- Delete `src/ui/campaign/NewsFeedPanel.tsx` and its menu entries/`uiStore` flag.
- **Keep** `campaign_get_journal` — `HousesPanel`'s pulse ticker, `HubPanel`'s
  chronicle and `FiguresPanel` still read it.
- **Gate:** `npx tsc --noEmit`, `npx vite build`.

### 01.5 · Measure and close
- A small `#[ignore]`d diagnostic printing save size (serialized `CampaignSim`
  bytes) and journal length after 200 years, before/after pruning. Record the
  numbers in `docs/SCOREBOARD.md`.
- End-of-row gates (`tick::tests`, `econ_`) — pruning never feeds back into the
  economy, so both must be bit-identical.

## Queue
- Q01.1 — Venue histories (row 08) and person-less city chronicles must adopt the
  same rule when they are built.
- Q01.2 — A settings option for `CHRONICLE_KEEP_YEARS` (waits on a settings pass).
