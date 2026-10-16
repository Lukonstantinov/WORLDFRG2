# Generation Realism Plan (2026-10-16)

Source: 6 read-only specialist reviews (geomorphology, oceanography, atmosphere, hydrology,
cryosphere, pipeline/UX). Feature freeze lifted for generation realism (owner, 2026-10-16).
Owner rules: physics follows planet settings honestly; animations = cross-fade, flow tracers,
summer/winter toggle. Every batch gated by `cargo test --lib earth_` + full test suite.

## Batch 0 - correctness bugs (small, do first)
- [x] **Noise hash bug** `step2_terrain/elevation.rs:14-16` - signed `>>` clears top bit, all
      noise in [0,0.5). Ridged noise never ridges, warps biased up-left, seamounts never spawn.
      Fix `((h as u32) >> 16) as i32`, then re-measure `FBM_LO/HI`, `FBM_MEASURED_*`, `-0.5`
      warp constants and thresholds in rivers/biological/deposits. Every seed changes.
- [x] **Per-step != Run-All** (`world_buffer.rs` column masks): rivers step misses
      temperature+snow_frac (D1); biological misses soil (D2); provinces built before goods (D3);
      shelf params hard-coded in elevation cmds + `sim_run_all_from_terrain` (D4); lake-floor 4 vs 20 (D5);
      StepRivers Refresh hard-codes goods params (D6).
- [x] **Elevation sliders ignored in plates mode** (`elevation.rs:738-739` hard 0.5).
- [x] **Salinity reads stale precipitation** (`ocean.rs:1131`) - step 3 not idempotent.
- [x] **SST vs ocean temperature mismatch** - `compute_sst` lacks EBM anomaly; shelf-ice cooling after SST.
- [x] **Lakes**: query depth uses spill level not trimmed surface; panel salinity ignores
      `lk.salinity_ppt`; 4 lake types return empty fish lists; >60 ppt must hold no fish.
- [x] **Cell area ignores cos(lat)** (rivers.rs:603,765,1497) - boreal rivers/lakes ~2x too big.
- [x] **Staleness**: re-running step N must invalidate dependants (uiStore `markStepCompleted`).
- [ ] **Found 2026-10-17 (pre-existing, not the noise fix):** analytical relief shows stepped,
  diamond-shaped blocks across shallow shelves around every coast (dump_elevation_style_sheet).

## Batch 1 - terrain randomness (owner priority)
- Noise wavelengths in km not cells; add ~3000 km continent octave; ridge layer 9 octaves.
- Per-region random hypsometry around the slider (continents differ); REGION_CONTRAST ~0.6.
- Orogen belt reach 250-400 km, amplitude ~ convergence speed; drop transform boundaries from uplift.
- Province-dependent relief budget (massif ~60 m/cell, plains 16).
- Later: passive margins (crust mask separate from plates), age-depth ocean floor, hotspot chains,
  stream-power erosion using unused `geo.erodibility`.

## Batch 2 - ocean currents / SST (owner priority)
- Retrograde fully mirrored: zonal table, ACC, coast-turn, upwelling side (`rotation_sign`).
- ACC/circumpolar check both hemispheres, any open band 45-75 deg.
- Upwelling bands scale with `circ.belt_lat`, no shelf requirement, cools SST; equatorial upwelling.
- Sea ice from SST vs freezing point (drop fixed 45-62 deg gate), open-ocean ice allowed.
- Coastal current warmth spread along wind, reach in km.
- Option: coarse Stommel barotropic solve (psi) replacing hand-shaped boundary currents.

## Batch 3 - winds / monsoons / Koppen (owner priority)
- ITCZ migration + monsoon amplitude scale with tilt (insolation ratio); 0 deg tilt -> no monsoon.
- Upwind/maritime helpers use circulation belts + rotation_sign (koppen.rs:69/104/262).
- Seasonal grad/blur scales in km (fixes 3600-wide maps getting 45 km monsoon reach).
- Wind ITCZ x belt_scale so wind and rain ITCZ agree.
- Koppen s/w via monthly sinusoid + Psmin<40 mm (Kottek) -> real Cs/Cw coverage.
- Rodwell-Hoskins west-coast summer drying for Mediterranean.

## Batch 4 - rivers / lakes / fish
- Runoff-weighted accumulation q = area(lat) * P * (1 - Fu(PET/P)); discharge = q[outlet].
- True Strahler order; test Hack exponent 0.5-0.7, bifurcation 3-5.
- Lake water balance uses real catchment; salinity from evap/inflow; rift depth boost.
- Fish band from summer water temp (Mohseni) + oxygen gate.

## Batch 5 - cryosphere
- Stop deleting EF/snow lakes (rivers.rs:1400-1418); draw them frozen.
- New `ice_state` u8 at end of step 4: none / seasonal water ice / perennial lake ice /
  glacier (Ohmura ELA: precip vs summer T) / ice sheet / seasonal sea ice / perennial sea ice + permafrost bit.
- `biome::cryosphere` reads it; frozen water rendered pale flat (no crevasses); sea ice in Natural view.

## Batch 6 - UI + animation
- Step order: Landmass, Planet, Elevation, Ocean&Atmo, Climate zones, Rivers&Lakes,
  Soil&Biomes, Settlements, Goods(+auto provinces), Toponyms, Political, Economy.
- Per step: "Uses / Makes" line, 3-5 main controls, Advanced section (reuse `<Section>`),
  auto layer, amber "re-run needed" chip. Move trade params to Political/Economy.
- Animations: cross-fade snapshot on Generate (frontend); wind/current particle tracers
  (frontend, only while overlay on); summer/winter view variant (small Rust render change).

## Owner decisions (2026-10-16)
- Physics follows planet settings: retrograde fully mirrors gyres; 0 deg tilt = no monsoon;
  eccentricity must actually act (or the slider goes); fast rotators get multiple storm tracks;
  rift lakes forced deep.
- Noise hash fix approved - every seed changes (saves unaffected), no legacy toggle.
- Erosion: Earth-like valley networks (stream power), tuned to what an 11 km cell can show (rule 30).
- Passive margins: YES (continents may sit inside plates).
- Currents: real coarse barotropic (Stommel) gyre solver, not patched rules.
- Koppen s/w: real Kottek definition; if the Earth gate regresses, revert and record (rule 2.4).
- Seasons: keep 2 seasons + sinusoid months.
- River discharge: runoff-based (Budyko/Fu); re-baseline settlement/goods tests.
- Fish: regional (basin/continent ecoregions) on top of water temp/oxygen/salinity.
- Ice look: seasonal frost tint vs perennial ice vs glacier crevasses; sea ice in Natural view.
- Permafrost: visual + lowers farming/habitability; campaign untouched.
- Staleness: amber "re-run needed" chip, Continue still allowed.
- Trade settings move from step 8 to Political/Economy.
- Animations: cross-fade, wind/current tracers, summer/winter toggle (no process replay).
