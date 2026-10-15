## The owner's stated goal
The owner (a solo developer) builds WorldForge 2 as a **tool for fantasy worldbuilding** that they **eventually want to sell**. No price, target platform, release date or marketing plan was given.

## What the project is (from its README and repository, github.com/Lukonstantinov/WORLDFRG2)
WorldForge 2 is a Tauri 2 desktop application (Rust backend, React 18 + PixiJS 8 + Zustand frontend, SQLite + zstd tile storage) that procedurally generates "plausible, science-grounded" fantasy world maps and then runs a living economy on top of them.

- It is a ground-up rewrite of a browser-based WorldForge 1, which hit a ~1.4 GB memory ceiling and crashed on large grids. WF2 simulates worlds at 3600x1800 cells and beyond.
- **World generation pipeline:** plates -> elevation -> ocean & atmosphere (wind belts, salinity, currents, temperature, upwelling, precipitation) -> Köppen climate -> rivers/lakes -> soil & fertility -> settlements -> biology & trade goods -> politics -> market economy. Users can start from procedural tectonics or import/paint their own landmass.
- Climate uses a 1-D North-Budyko energy-balance model with astronomical insolation; it is scored against the real Earth's Köppen-Geiger map by an in-repo validation harness with a CI regression floor.
- **Economy/campaign:** a stock-based market equilibrium solver over 45 trade goods (incl. ~21 manufactured goods from recipes); a day-by-day "Living Trade" campaign with merchant houses, banks, coinage/debasement, poleis, tariffs, wars, blockades, credit crashes and a chronicle. Recent work (Sept-Oct 2026) adds a "Living World" people layer: notable individuals with face genomes, councils, governments & edicts, culture acceptance, ideology & scholars, artisans & masterworks, leisure & games, realms/war/barbarians, character windows.
- **Creator tools:** itinerary/travel time between cities, Goods Codex (provenance, real-world commodity history, scarcity overlay), economy dashboard (price index, Gini), culture-styled toponyms for rivers/mountains/regions.
- **Rendering & export:** 18+ server-side-rendered tile layers with an LOD pyramid; 16-bit heightmap PNG and per-layer PNG export.
- **Architecture:** world split into 128x128-cell tiles (X wraps, Y clamps); each sim phase loads tiles into a flat WorldBuffer, computes, writes back. Rust renders RGBA tiles, frontend displays them. World is "finalized" into a `.worldforge` file; the campaign lives in a separate `.campaign` file.
- README status line: "Active development, broadly feature-complete... Ongoing work focuses on tuning plausibility (climate, currents) and the depth of the economic dynamics."

## Measurable facts about the repository (as of 2026-10-15)
- First commit 2026-06-03, last commit 2026-10-01: about 4 months; 988 commits. Of those, 728 are authored by "Claude" (an AI coding agent) and 260 by the owner, i.e. the code is largely AI-written under the owner's direction.
- ~129,500 lines of Rust in 137 files; ~59,800 lines of TypeScript/TSX. 748 Rust `#[test]` functions; the latest scoreboard entry records `tick::tests` 411/411 passing and an economy-fidelity suite taking 573 s.
- 116 git branches. 36+ design/plan markdown documents in docs/ (e.g. BANKS_MONEY_AND_CRAFT_PLAN, REALM_AND_GOVERNMENT_PLAN, PROVINCE_SYSTEM_PLAN, MERCHANT_VESSELS_AND_INFORMATION_PLAN, a 10-row "living_world" plan) plus proposals.
- No LICENSE file. No release tags; only dev-mode run instructions (`npm run tauri dev`, `run.bat`) and a `tauri build` command. No mention of users, testers, sales page or pricing anywhere in the README.
- The project keeps a SCOREBOARD.md of measured economy metrics (e.g. house-wealth Gini, top-10% wealth share). The latest entry notes a metric that "fell out of its band" with cause "not yet explained".
- A docs/TWO_APPS_AND_FILE_UPLOAD_PLAN.md audit asked whether WorldForge should become two apps (world generator + campaign player); it found opening a saved `.worldforge` file and pressing Begin Campaign failed on every world file previously written (save deleted rows the campaign needs). Slices 1-2 of the fix were built; slices 3-5 (bundle file, sibling-file pairing, drag-and-drop) were not.

## History of problems recorded in the owner's development notes (all reported fixed unless stated)
- Campaigns crashed around simulated year 30: a Rust abort from an unbounded undo/history journal causing out-of-memory (journal capped); a separate crash from a preview loading every data column (fixed).
- Performance: O(n^2) algorithms on growing entity counts in the campaign; a tiered performance plan and a background-engine architecture were written.
- Economy bugs found and fixed over time: population collapse/famine from a tech-factor erosion, banks failing within 2-3 years from vanishing loan principal, trade reporting 0.0/yr, realms never forming, single-house dominance from a founding gate, pepper appearing in the arctic.
- Climate bugs found and fixed: tropical deserts from a scaling bug, zebra-striped Köppen bands, river seams.
- Notes record some pre-existing failing tests on main at various points (e.g. an inheritance gate and an `unrest_topples_councils` test).
