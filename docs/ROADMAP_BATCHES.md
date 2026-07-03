# Roadmap — the 24 picked features, re-evaluated and batched

User's picks: 1, 3, 4, 6, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 23 (important),
26, 27, 28, 30, 33, 37, 40, 43, 44. Numbers refer to the 45-feature slate
(chat, 2026-07-03). Re-evaluated here against the actual code; each batch ends
with the standing gates: `simulate_decades_reports_dynamics` digest read,
`cargo test --lib tick::tests`, `tsc`, an HTML report for anything visual,
push to branch + main.

## Status corrections from the re-evaluation

- **#3 Fast Play — ALREADY SHIPPED** (commit c742688): Arc resident sim
  (0.956 ms → 13 ns per query, `bench_sim_clone`) + basins memo. Rayon (P4)
  stays parked until `bench_campaign_tick` says tick cost bites.
- **#4 Compact saves — RE-SCOPED**: not bincode (it isn't self-describing and
  would break the append-`serde(default)` save-compat strategy every DLC has
  relied on). Instead **zstd-compress the JSON blob** (zstd is already a dep):
  ~5–10× smaller `campaign_sim` row and `.campaign` files, JSON stays the
  format, legacy rows load via a prefix check.
- **#19 Per-good heat — CHEAPER THAN PLANNED**: no per-pair-per-good ledger
  needed. A per-hub `[good] → yearly volume` table (150×45 f32 ≈ 27 KB) filled
  where `flow_accum` is fed gives per-good heat AND per-basin top goods.
- **#18 Era scrubber — PARTLY FREE**: markers can already time-travel
  (`founded_tick`/`died_tick` + history sparks); only per-year per-hub
  `[pop, trade]` samples must be stored (bounded ring, ~500 KB) for heat +
  census scrubbing.
- **#17 Credit scores** moved INTO the banking batch (it is the coupling that
  makes endogenous rates matter for houses).
- **#1 Cultures** and **#23 migration** are one batch: assimilation/minority
  dynamics FEED on the migration flows, so migration lands first.
- **#40 + #44 exporters** share one HTML-snapshot generator.

## Batch 1 — Data plumbing & quick wins  ✅ SHIPPED (see docs/mockups/atlas-batch1.html)
| # | Feature | Notes |
|---|---------|-------|
| 4 | zstd-compressed campaign saves | prefix-detected, legacy JSON fallback |
| 19 | Per-good trade heat | new `hub_good_trade` yearly table; Goods Codex "heat" toggle; basin top-goods |
| 18 | Era scrubber | yearly per-hub `[pop, trade]` ring; Atlas year slider scrubbing map markers + heat + census |
| 43 | Hall of Records | all-time records struct updated at New Year; Atlas "Records" tab |

## Batch 2 — The Living Map II: peoples  (#23 marked IMPORTANT; ~4–5 days)
| # | Feature | Notes |
|---|---------|-------|
| 23 | Economic migration | yearly wage/mood-driven population drift toward thriving cities (same component), reusing migration-arrow language; digest-gated so populations stay bounded |
| 1 | Living Cultures Map | phase A: per-hub culture from the worldgen culture map, inherited by colonies/swarms; campaign overlay (soft borders + labels) + Atlas Cultures stats. Phase B: minority quarters fed by #23 flows + slow assimilation |
| 30 | Caravanserais | waystations sprout on busy long land corridors, trim route cost, can grow into towns (second organic-founding path) |
| 20 | Basin history beats | yearly basin snapshot in the sim → "X eclipses Y" chronicle beats + basin race chart in Atlas Regions |

## Batch 3 — Banking & credit  (one digest-gated block; ~4 days)
Order matters: 10 → 17 → 9 → 11 → 12.
| # | Feature | Notes |
|---|---------|-------|
| 10 | Endogenous interest rates | loan rate from reserve ratio within a band; crash frequency must stay in band |
| 17 | House credit scores | growth record + defaults + prestige → per-house loan ceiling/rate |
| 9 | Interbank contagion | exposure ledger; failures propagate along real edges; Crashes tab draws the chain |
| 11 | Lender of last resort | council bailout: treasury drain + fineness nudge, chronicled |
| 12 | Collateral & fire sales | estate-collateralized loans; crash margin calls transfer works between houses |

## Batch 4 — Dynasty character  (~4 days)
| # | Feature | Notes |
|---|---------|-------|
| 13 | Head traits | deterministic roll at succession; biases existing levers; UI chip + beats |
| 14 | Cadet branches | great houses split on succession; kinship tie; Dynasties panel |
| 16 | House projects | multi-year goals with progress bars; prestige on completion |
| 26 | Dynasty tree viewer | SVG family tree from succession/marriage records |
| 27 | Great Lives codex | auto-written biographies from figures' event trails |
| 37 | Guild masterworks | rare named treasures; auctioned for prestige; feeds Hall of Records |

## Batch 5 — Craft, intrigue & the city  (~3 days)
| # | Feature | Notes |
|---|---------|-------|
| 6 | Fungible recipe inputs | category substitution at an efficiency penalty; chain-review shows "or any <category>" |
| 33 | Spy networks 2.0 | sabotage + market intelligence on top of recipe theft; digest-gated |
| 28 | Living city view | SettlementScene grows structures/districts as the hub builds them |

## Batch 6 — Exports & sharing  (~2 days)
| # | Feature | Notes |
|---|---------|-------|
| 40 | Chronicle book export | self-contained illustrated HTML history of the campaign |
| 44 | Shared atlas snapshot | read-only HTML Atlas export (same generator) |

## Explicitly NOT picked (parked)
2 Polis Wars 2.0 · 5 quality-through-recipes · 7 districts · 8 labor market ·
15 vendettas · 21 roads · 22 pirates · 24 faiths · 25 diplomacy screen ·
29 shipping eras · 31 fair calendar · 32 sumptuary laws · 34 insurance ·
35 climate anomalies · 36 information lag · 38 hand of fate · 39 scenarios ·
41 poster export · 42 message settings · 45 mod packs.
