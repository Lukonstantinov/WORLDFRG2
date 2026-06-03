# Fifteenth batch — trade-goods rules, routed flows, gems, shipworms, Political step 9

User answers (rounds 1-3):
- Unlimited goods (fill every suitable area, no single homeland): stockfish, furs, timber, salt, whaling, **wheat**, **iron**. Rest seeded (single homeland).
- Seeded belts get an **island-jump** of ~4% map width so thin seas don't chop a belt into separate islands.
- Trade reach is a **generation-time choice**: Global / Coastal+short crossings / Continental, plus a max open-water-crossing slider (% width). Continents too far ⇒ trade stays within continent.
- **Wheat**: new unlimited food good, Mediterranean/temperate grain.
- **Iron**: unlimited (hills/mountain margins). **Cotton**: seeded (warm river valleys).
- **Gemstones**: ONE good, multi-deposit, **highland-locked**, count preset (Few/Some/Many). Global (not climate-bound). InfoPanel/region names the stone (ruby/sapphire/emerald) per deposit.
- Trade **flows routed along the route network + bundled** (volume = width). Sea-impassable pairs get no flow.
- Trade **routes add inland travel through mountain passes** (saddle discount).
- **Shark zones**: only highest-probability; move under a Biological overlay group. Add **shipworm** zones (warm, brackish/low-salinity, shallow coastal — wooden-hull hazard).
- **Step 9 Political** (influence circles only): re-rank settlements by trade power (route centrality + good monopoly); translucent influence discs, no territory fill.

## Good indices (appended LAST for save back-compat). GOODS_COUNT 17 -> 21
17 wheat (unlimited, land), 18 iron (unlimited, land), 19 cotton (seeded, land), 20 gemstones (special placement, land).

## New persisted field: shipworm_risk u8 (serialized AFTER goods => truly last => old saves zero-pad).

## Status: IN PROGRESS.
