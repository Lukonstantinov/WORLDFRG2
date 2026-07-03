# Systems 2.1 — Proposals: Performance · Manufactories · Banks · Houses

Follow-up proposals after Trade 2.0 (Living Map / Trade Heat / World Atlas).
Each item names the code it touches and a rough cost. Nothing here is built yet.

---

## 1 · Performance

### P1 — Stop cloning the whole sim per query  ★ the big one, cheap to fix
`campaign_commands::get_sim` returns `cache.sim.clone()` — **every read-only
panel query deep-clones the entire `CampaignSim`**: ~150 hubs × 45-good
`stock/price/production/quality` vectors, per-hub `history`, the whole journal
(capped at 20 000 entries), houses, banks, contracts. A single HUD refresh fires
several such queries (state + houses + diagnostics + world economy + any open
panel), so fast Play clones megabytes many times per second.

**Fix:** replace `get_sim(...) -> Option<CampaignSim>` with a borrow-based
helper — `with_sim<R>(db, conn, f: impl FnOnce(&CampaignSim) -> R) -> Result<Option<R>>`
— and convert the ~30 read-only queries mechanically (they only read). Writers
(`campaign_advance`, start/new-game) already work on the resident copy.
*Cost: half a day, mostly mechanical. Expected: biggest single latency win for
every panel while Playing.*

### P2 — Split the journal out of the persisted blob
`persist_campaign` serializes the whole sim (including the 20k-entry journal)
to ONE JSON metadata row on every flush cadence. Move `journal` to its own
row (append-mostly), persist it less often, and serialize the hot state with
`bincode + zstd` instead of JSON (the tile store already depends on zstd).
*Cost: 1 day incl. legacy-load path (JSON fallback). Smaller saves, faster
pause/flush.*

### P3 — Snapshot cost gating
`build_snapshot` now computes `pop_spark` (downsampled history) for every hub on
every advance tick. Gate the spark (and `recent_events` slicing) behind the
`heavy` refresh flag the store already passes, so fast Play snapshots stay
minimal.
*Cost: hours.*

### P4 — Parallel per-hub passes with rayon (already a dependency)
The daily passes that are embarrassingly per-hub (production, consumption,
manufacture, sentiment easing) can run under `par_iter_mut` without changing
results (no cross-hub reads). Keep cross-hub passes (dispatch, arrivals,
contagion) serial for determinism. Measure with the existing
`bench_campaign_tick` (`--ignored`) before/after.
*Cost: 1–2 days incl. determinism audit; benefit grows with hub count.*

### P5 — Query result caching keyed by tick
Read-only queries (basins, speculation, inequality, trade flow) recompute per
call but can only change when `sim.tick` advances (most only at New Year).
A tiny `(tick, result)` memo in the campaign cache makes repeated panel opens
free.
*Cost: hours per query, adopt opportunistically.*

---

## 2 · Manufactory logic

### M1 — Quality flows through recipes
DLC 4 grades goods (`TickHub.quality`) but a manufactory's OUTPUT quality
ignores its INPUT quality. Make finished quality
`f(mean input quality, workshop tier, resident guild skill)` — so Venetian
glass is only as good as its ash and sand, and quality espionage matters
upstream. Surfaces in the Goods window with a "why is this fine" breakdown.
*Cost: ~1 day in `manufacture_pass` + Goods UI line.*

### M2 — Category-fungible recipe inputs
`TickGood.fungible_input` exists but recipes bind to exact goods. Let a recipe
row accept any good of the input's *category* at a small efficiency penalty
(mirroring the market's category substitution): cloth takes cotton OR wool,
beer takes any cereal. Kills the "chain dead because one belt is far away"
failure and makes manufactories siege-resilient (feeds the war/blockade story).
*Cost: ~1 day across `manufacture.rs` + `tick.rs::manufacture_pass` + chain
review DAG display.*

### M3 — Industrial districts (agglomeration)
Co-located producers of a recipe's inputs give the manufactory a small
throughput bonus (supply lines are short). Over decades this concentrates
industries into recognisable districts — "the cloth towns" — which the Trade
Basins overlay then names for free. Feed the bonus from the existing
`neighbors` matrix; no new pathfinding.
*Cost: ~1 day + a HubPanel "district" badge.*

### M4 — Labor market coupling
Manufacture labor is `∝ population` only. Couple it to the SOCIETY strata that
already exist: burghers/commoners supply workshop labor, wages drift with labor
scarcity, and a manufactory closure dumps workers into `underclass` (feeding
the existing unrest loop). Industry booms then visibly reshape a city's social
pyramid in the Society block.
*Cost: 2 days; touches `update_society` + `manufacture_pass`.*

---

## 3 · Bank logic

### B1 — Interbank lending → network contagion
Banks currently fail alone and crashes flood a whole region uniformly. Let
thin banks borrow reserves from fat ones (an exposure edge). On failure, losses
propagate along REAL exposure edges before the regional panic — contagion
becomes a story you can trace in the Bank panel ("Bank of X fell because it
lent to Y"). The Crashes tab draws the failure chain.
*Cost: 2 days; `Bank` gains an `interbank: Vec<(bank, amount)>` ledger.*

### B2 — Endogenous interest rates
Loan/deposit rates are effectively capped constants. Derive each bank's loan
rate from its reserve ratio (scarce reserves → dearer credit → cooling booms) —
an organic credit cycle instead of tuned constants, and the SpecCenter risk
read gets a real signal (cheap credit near a 4★ bubble = the classic setup).
*Cost: ~1 day + digest check (crash frequency must stay in band).*

### B3 — Lender of last resort (a polis policy)
A council whose seat bank wobbles may inject treasury (existing
`decide_polis_policy` hook) — saving the bank but draining the war chest and
nudging `mint_fineness` (inflationary bailout). Player-visible tradeoff in the
Poleis tab; bailout vs crash becomes a chronicled decision.
*Cost: ~1 day.*

### B4 — Collateral & fire sales
Banks already take equity stakes in estates. Add loans collateralized on
estates: in a crash, margin calls force estate sales at haircuts (bounded by
limited liability), transferring works between houses — crashes then RESHUFFLE
the house league table instead of just haircutting everyone.
*Cost: 2 days; reuse `BankStake` machinery.*

---

## 4 · House logic / features

### H1 — Head traits (personality-driven strategy)
Houses have 4 archetypes; heads have names and lifespans but no character. Give
each head 1–2 traits (bold/cautious/pious/ruthless/builder) that bias real
levers that already exist: risk appetite (contract terms, outpost distance),
levy resistance, bank founding, festival/wonder spending. Succession then
visibly changes course — "the cautious years ended with old Maro". Traits show
on the house Summary + chronicle beats.
*Cost: 2 days; deterministic trait roll on succession.*

### H2 — Cadet branches
A house above a wealth/offices threshold splits on succession: the younger
line takes the remotest offices and a wealth slice, becoming a NEW house with
a kinship tie (no feuds for a generation, marriage-like alliance). Keeps house
count self-balancing upward (deaths already prune) and creates dynastic
sprawl the Dynasties panel can draw as a family tree.
*Cost: 2 days; reuses house-creation + marriage-alliance plumbing.*

### H3 — Vendettas (private wars)
`rivals` exist but only shade trading. Escalate: a rivalry past a threshold
opens a vendetta — office raids, cargo seizures (bounded losses), resolvable by
a dynastic marriage or one side's ruin. Chronicled with its own journal kind;
the Atlas timeline gets a "feud" filter for free.
*Cost: 2–3 days; must re-run the dynamics digest (house turnover stays healthy).*

### H4 — House projects (long-term goals with progress)
Each house periodically adopts a visible multi-year PROJECT: corner a good
(≥50% monopoly), charter a bank, build a wonder/guildhall, plant a colony.
Progress bar on the house card; completion grants prestige + a chronicle beat;
failure (rival got there first) feeds the vendetta meter. This is pure
narrative surface over mechanics that all already exist — the cheapest
"the world has protagonists" win.
*Cost: ~2 days, mostly UI + goal-picker.*

### H5 — Creditworthiness ties houses to banks
`stable_growth_years` already gates futures terms. Extend it into a credit
score (growth record + defaults + prestige) that sets each house's loan
ceiling/rate at each bank — connecting H-features to B2/B4 so finance and
dynasty stories interlock.
*Cost: ~1 day.*

---

## Suggested order

1. **P1** (clone removal — everything else gets faster free)
2. **M2 + M1** (recipes get resilient, then quality-deep)
3. **B2 → B1** (organic rates, then network contagion)
4. **H1 + H4** (character + goals = visible protagonists)
5. P2/P4 when tick cost or save size actually bites; H2/H3/B3/B4 as the next
   flavour drop.

Every sim change re-runs `simulate_decades_reports_dynamics` and reads the
digest per the standing rule; B1/B2/H3 additionally watch crash frequency and
house-turnover lines.
