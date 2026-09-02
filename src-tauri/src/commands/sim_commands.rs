use tauri::State;
use crate::db::WorldDb;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{plates, elevation, ocean, temperature, jets, precipitation, koppen, rivers, soil, fertility, settlements, biological, toponyms, landmass_ops};
use crate::db::metadata;
use crate::tile::coords::{TileCoord, TILE_SIZE};

/// Persist the two placement records `compute_trade_goods` returns — the ORE
/// WORKINGS (`sim::deposits`, §8.16) and, since CLAUDE.md §8.19 (goods localities, shipped), every
/// non-mineral good's LOCALITIES (`sim::localities`, Slice 3) — to world metadata
/// exactly as before, just from one shared helper instead of four duplicated
/// call sites.
fn persist_goods_placement(
    conn: &rusqlite::Connection,
    ore: &[crate::sim::deposits::Deposit],
    localities: &[crate::sim::localities::GoodLocality],
    report: &crate::sim::biological::GoodsPlacementReport,
) -> Result<(), String> {
    metadata::set_meta(conn, "deposits",
        &serde_json::to_string(ore).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    metadata::set_meta(conn, "good_localities",
        &serde_json::to_string(localities).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    // The post-generation placement report — which goods actually made it onto
    // this world, and which quietly did not. Persisted like the two above (JSON in
    // `metadata`, no tile column, rule 7) so it survives a save and can be
    // re-opened later rather than being a one-shot toast the user can miss.
    metadata::set_meta(conn, "goods_report",
        &serde_json::to_string(report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Generate tectonic plates and derive landmass.
/// Phase 1: Plate tectonics → terrain
#[tauri::command]
pub fn sim_generate_plates(
    seed: u64,
    plate_count: u32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES)?;
    let motion = plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
    persist_plate_motion(&conn, &motion);
    buf.save(&conn, "Generate plates & landmass")
}

/// Invert land and sea
#[tauri::command]
pub fn sim_invert_terrain(db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES)?;
    plates::invert_terrain(&mut buf);
    buf.save(&conn, "Invert terrain")
}

/// Generate elevation from plate tectonics + sea depth.
/// Phase 2: Terrain → elevation + bathymetry
#[tauri::command]
pub fn sim_generate_terrain(seed: u64, db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation(&mut buf, seed);
    elevation::compute_sea_depth(&mut buf);
    // Proper continental shelf (not just compute_sea_depth's thin ~1px ring) so
    // the Shelf layer is populated after the per-step elevation phase.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate terrain & depth")
}

/// Run ocean & atmosphere simulation.
/// Phase 3: Winds → currents → upwelling → distance_to_ocean → temperature → precipitation
#[tauri::command]
pub fn sim_ocean_atmosphere(db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE)?;

    ocean::compute_wind_belts(&mut buf);
    // Salinity must be computed before the currents so the moderate-thermohaline
    // coupling (density-driven speed boost + extended warm conveyor) can run
    // inside generate_ocean_currents. Salinity uses a latitude SST estimate (not
    // the temperature field) to avoid a salinity↔temperature↔current cycle.
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    // Close the SST loop: the currents just generated now feed back into the stored
    // sea-surface-temperature field (latitude + current anomaly) used by rendering.
    ocean::compute_sst(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    // Seasonal sea ice on shallow high-latitude shelf seas (Hudson Bay / Okhotsk):
    // compute the freeze index once, let it reinforce cold currents (brine
    // rejection) BEFORE temperature so the current-influence pass sees the new
    // cold tags, then apply the "refrigerator" cooling AFTER temperature (which
    // rewrites the field from scratch, so an earlier cooling would be lost).
    let sea_freeze = ocean::compute_shelf_freeze(&buf);
    ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
    // Energy-balance seasonality: derive the seasonal temperature span from insolation
    // × heat-capacity attenuation (replaces the fabricated latitude range Köppen used),
    // then apply a bounded ice/snow-albedo cooling feedback. Runs after all mean-
    // temperature adjustments so it reads the final annual mean.
    temperature::compute_seasonal_amplitude(&mut buf);
    temperature::apply_ice_albedo_feedback(&mut buf);
    // Low-level jets (Somali jet et al.) must precede precipitation: their
    // entrance/exit acceleration reshapes where rain falls, and the Wind Speed
    // layer reads the speed field they write.
    jets::compute_low_level_jets(&mut buf);
    precipitation::compute_precipitation(&mut buf);

    buf.save(&conn, "Ocean & atmosphere simulation")
}

/// Run climate classification.
/// Phase 4: Köppen zones
#[tauri::command]
pub fn sim_classify_climate(db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_CLIMATE)?;
    koppen::classify_koppen(&mut buf);
    buf.save(&conn, "Climate classification")
}

/// Run river extraction and hydrology.
/// Phase 5: D8 flow → rivers → lakes
#[tauri::command]
pub fn sim_rivers_hydrology(
    river_density: f32,
    river_width: f32,
    lake_fill_depth: f32,
    lake_max_fraction: f32,
    db: State<'_, WorldDb>,
) -> Result<SimRiversResult, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_RIVERS)?;

    let max_cells = ((buf.total() as f32) * lake_max_fraction.clamp(0.000002, 0.05)) as usize;
    let max_cells = max_cells.max(4);
    // ONE call for flow + lakes: the two must agree, or a river ends at a shoreline
    // the renderer never draws (see `compute_world_hydrology`).
    let wh = rivers::compute_world_hydrology(&buf, lake_fill_depth, max_cells);
    let hydro = wh.hydro;
    let mut lakes = wh.lakes;
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, &hydro.filled, river_density, river_width, &lakes);
    persist_rivers(&conn, &extracted_rivers);
    // Oxbow backwaters cut off from the meandering lowland reaches (real lakes).
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    // Tag terminal salt lakes (endorheic + arid) so the overlay tints them and the
    // Hydrology panel, goods and settlements agree on the brine.
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);
    persist_lakes(&conn, &lakes);

    // Store rivers as serialized state for rendering
    // (Rivers are overlays, not per-cell data stored in tiles)
    let tiles_x = buf.tiles_x;
    let tiles_y = buf.tiles_y;
    let mut modified = Vec::new();
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            modified.push((tx, ty));
        }
    }

    Ok(SimRiversResult {
        modified,
        rivers: extracted_rivers,
        lakes,
    })
}

#[derive(serde::Serialize)]
pub struct SimRiversResult {
    pub modified: Vec<(i32, i32)>,
    pub rivers: Vec<rivers::River>,
    pub lakes: Vec<rivers::Lake>,
}

/// Run soil, fertility, and fishery simulation.
/// Phase 6: Soil → fertility → fisheries
#[tauri::command]
pub fn sim_soil_fertility(
    rivers_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_SOIL_FERTILITY)?;

    // Deserialize rivers from JSON
    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json)
        .unwrap_or_default();

    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &river_data);
    fertility::compute_fertility(&mut buf, &river_data);
    fertility::compute_fisheries(&mut buf, &river_data);

    buf.save(&conn, "Soil & fertility")
}

/// Classify ecological biomes.
/// Phase 6b: Köppen + the climate/soil/relief stack + rivers & lakes → `biome`.
///
/// Runs AFTER soil (it reads `soil_type` for the prairie/podzol/peat fingerprints)
/// and needs rivers + lakes for the azonal wetland/riparian biomes. Purely
/// descriptive — no later phase scores off the result.
#[tauri::command]
pub fn sim_classify_biomes(
    rivers_json: String,
    lakes_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_BIOME)?;

    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json).unwrap_or_default();
    let lake_data: Vec<rivers::Lake> = serde_json::from_str(&lakes_json).unwrap_or_default();

    crate::sim::biome::classify_biomes(&mut buf, &river_data, &lake_data);

    buf.save(&conn, "Biome classification")
}

/// Per-biome land-cell counts for the legend, as `(code, name, group, cells)`.
/// Read-only — no tile write-back, so it never dirties the world.
#[tauri::command]
pub fn get_biome_stats(db: State<'_, WorldDb>) -> Result<Vec<BiomeStat>, String> {
    use crate::sim::biome::{biome_group, biome_name, BIOME_COUNT};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let buf = WorldBuffer::load_with(&conn, ColumnSet::TERRAIN | ColumnSet::BIOME)?;

    let mut counts = vec![0u32; BIOME_COUNT];
    for i in 0..buf.total() {
        if buf.terrain[i] != 1 {
            continue;
        }
        let b = buf.biome[i] as usize;
        if b < BIOME_COUNT {
            counts[b] += 1;
        }
    }
    Ok((1..BIOME_COUNT)
        .filter(|&b| counts[b] > 0)
        .map(|b| BiomeStat {
            code: b as u8,
            name: biome_name(b as u8).to_string(),
            group: biome_group(b as u8).to_string(),
            cells: counts[b],
        })
        .collect())
}

/// One biome's share of the world, for the Biomes legend.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BiomeStat {
    pub code: u8,
    pub name: String,
    pub group: String,
    pub cells: u32,
}

/// Run the biological phase: shark-habitat danger + trade-good belts.
/// Phase 8: Sharks + trade goods (persisted u8 fields).
#[tauri::command]
pub fn sim_biological(
    seed: u64,
    rivers_json: String,
    gem_deposits: u32,
    climate_strictness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_BIOLOGICAL)?;

    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json)
        .unwrap_or_default();

    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &river_data);
    biological::compute_shipworm_risk(&mut buf, &river_data);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    // The discrete ORE WORKINGS (grade / extent / depth per deposit) that the u8
    // belt column cannot carry. Persisted alongside the tiles exactly as the
    // province list is — the belt stays the source of truth for production and
    // overlays; this is the record a mining industry and the quarry view read.
    let (ore, localities, goods_report) = biological::compute_trade_goods(&mut buf, &river_data, seed, gem_deposits, climate_strictness, &goods);
    persist_goods_placement(&conn, &ore, &localities, &goods_report)?;

    // Terminal salt lakes → brine into the salinity column + inland salt-pan
    // production. Lakes are re-derived here (this phase does not receive them).
    let hydro = rivers::compute_hydrology(&buf);
    let mut salt_lakes = load_lakes(&conn, &buf, &hydro.filled);
    rivers::classify_salt_lakes(&buf, &mut salt_lakes, &river_data);
    biological::apply_salt_pans(&mut buf, &salt_lakes, &goods);

    buf.save(&conn, "Biological (sharks, shipworms, storms, reefs & trade goods)")
}

/// One-click REFRESH of hydrology → biology on an existing world, WITHOUT
/// re-rolling elevation/climate or relocating settlements. Re-runs the rivers &
/// lakes pass (so a world made before the meander/oxbow/salt work gains true
/// meanders, oxbow backwaters and classified salt lakes), the soil/fertility pass
/// (delta floodplain abundance + delta fisheries) and the biological pass (trade
/// goods + inland salt-pan production). The human map (settlements) is left
/// untouched — use "Complete from Landmass" if you want cities re-placed too.
#[tauri::command]
pub fn sim_refresh_hydrology_biology(
    seed: u64,
    river_density: f32,
    river_width: f32,
    lake_fill_depth: f32,
    lake_max_fraction: f32,
    gem_deposits: u32,
    climate_strictness: f32,
    db: State<'_, WorldDb>,
) -> Result<SimRiversResult, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load(&conn)?; // ALL columns (writes soil/fert/goods/salinity/…)

    // Phase 5: rivers → lakes → oxbows → salt classification.
    // ONE call for flow + lakes: the two must agree or a river ends at a shoreline
    // the renderer never draws (`compute_world_hydrology`). This site was missed in
    // the first pass of that fix, so every REFRESH regenerated the truncated rivers
    // the generate path had just stopped producing.
    let max_cells = (((buf.total() as f32) * lake_max_fraction.clamp(0.000002, 0.05)) as usize).max(4);
    let wh = rivers::compute_world_hydrology(&buf, lake_fill_depth, max_cells);
    let hydro = wh.hydro;
    let mut lakes = wh.lakes;
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, &hydro.filled, river_density, river_width, &lakes);
    persist_rivers(&conn, &extracted_rivers);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);
    // Keep the STORED lake set in step with the refreshed rivers — otherwise a
    // refresh leaves settlement placement and the map reading a stale set.
    persist_lakes(&conn, &lakes);

    // Phase 6: soil & fertility (incl. delta floodplain abundance + delta fisheries).
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 6b: Ecological biomes. After soil (it reads the soil fingerprints) and
    // after rivers/lakes (they drive the azonal wetland & riparian biomes).
    crate::sim::biome::classify_biomes(&mut buf, &extracted_rivers, &lakes);

    // Phase 8: biological — hazards, trade goods, and inland salt pans.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    let (ore, localities, goods_report) = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, gem_deposits, climate_strictness, &goods);
    persist_goods_placement(&conn, &ore, &localities, &goods_report)?;
    biological::apply_salt_pans(&mut buf, &lakes, &goods);

    let modified = buf.save(&conn, "Refresh hydrology & biology")?;
    Ok(SimRiversResult { modified, rivers: extracted_rivers, lakes })
}

/// Run all simulations in sequence (full world generation pipeline).
/// This is a convenience command that runs all phases.

/// THE ELEVATION MODEL SELECTOR — the one place a mode string picks a generator.
///
/// Four models ship and until now only the wizard's step 2 could reach three of
/// them: both run-alls hardcoded a generator and silently discarded the user's
/// choice AND all four sliders, so "Generate Full World" always produced the
/// plate model however the picker was set. That is the bug this exists to close.
///
/// `"plates"` is the only model that reads `boundary_type`, so it is the natural
/// default for a plate-derived world and is NOT reachable on a painted or
/// imported one (there is no tectonic data to read) — a caller without plates
/// passes `allow_plates = false` and falls back to the shape model.
fn apply_elevation_model(
    buf: &mut WorldBuffer,
    seed: u64,
    mode: &str,
    density: f32,
    height: f32,
    spread: f32,
    roughness: f32,
    allow_plates: bool,
) {
    match mode {
        "cordillera" => elevation::generate_elevation_cordillera(buf, seed, density, height, spread, roughness),
        "ridged" => elevation::generate_elevation_ridged(buf, seed, density, height, spread, roughness),
        "shape" => elevation::generate_elevation_from_terrain(buf, seed, density, height, spread, roughness),
        "rift" => elevation::generate_elevation_rift(buf, seed, density, height, spread, roughness),
        "glaciated" => elevation::generate_elevation_glaciated(buf, seed, density, height, spread, roughness),
        "plateau" => elevation::generate_elevation_plateau(buf, seed, density, height, spread, roughness),
        "volcanic" => elevation::generate_elevation_volcanic(buf, seed, density, height, spread, roughness),
        // "plates" and anything unrecognised: the tectonic model where it can be
        // used, the shape model where it cannot. An unknown string must never
        // leave a world with no elevation at all.
        _ if allow_plates => elevation::generate_elevation(buf, seed),
        _ => elevation::generate_elevation_from_terrain(buf, seed, density, height, spread, roughness),
    }
}

#[tauri::command]
pub fn sim_run_all(
    seed: u64,
    plate_count: u32,
    elev_mode: String,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<SimRunAllResult, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load(&conn)?;

    // Phase 1: Plates & landmass
    let motion = plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
    persist_plate_motion(&conn, &motion);

    // Phase 2: Elevation & depth. Plates exist on this path, so every model is
    // available and "plates" is the default.
    apply_elevation_model(&mut buf, seed, &elev_mode,
        mountain_density, mountain_height, mountain_spread, noise_roughness, true);
    elevation::compute_sea_depth(&mut buf);
    // Phase 2b: continental shelves (default params). Without this the only
    // shelf-tagged cells are the thin 1-shelf ring compute_sea_depth marks, so
    // the shelf layer looked empty and upwelling/fisheries had almost nothing to
    // work with.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);

    // Phase 3: Ocean & atmosphere (salinity before currents for thermohaline coupling)
    ocean::compute_wind_belts(&mut buf);
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    // Close the SST loop: the currents just generated now feed back into the stored
    // sea-surface-temperature field (latitude + current anomaly) used by rendering.
    ocean::compute_sst(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    // Seasonal sea ice on shallow high-latitude shelf seas (Hudson Bay / Okhotsk):
    // compute the freeze index once, let it reinforce cold currents (brine
    // rejection) BEFORE temperature so the current-influence pass sees the new
    // cold tags, then apply the "refrigerator" cooling AFTER temperature (which
    // rewrites the field from scratch, so an earlier cooling would be lost).
    let sea_freeze = ocean::compute_shelf_freeze(&buf);
    ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
    // Energy-balance seasonality: derive the seasonal temperature span from insolation
    // × heat-capacity attenuation (replaces the fabricated latitude range Köppen used),
    // then apply a bounded ice/snow-albedo cooling feedback. Runs after all mean-
    // temperature adjustments so it reads the final annual mean.
    temperature::compute_seasonal_amplitude(&mut buf);
    temperature::apply_ice_albedo_feedback(&mut buf);
    // Low-level jets (Somali jet et al.) must precede precipitation: their
    // entrance/exit acceleration reshapes where rain falls, and the Wind Speed
    // layer reads the speed field they write.
    jets::compute_low_level_jets(&mut buf);
    precipitation::compute_precipitation(&mut buf);

    // Phase 4: Climate
    koppen::classify_koppen(&mut buf);

    // Phase 5: Rivers (default river/lake parameters). Lakes first so channel
    // extraction can stop rivers at lake shores instead of crossing the water.
    let lake_max = (buf.total() / 2000).max(20);
    let wh = rivers::compute_world_hydrology(&buf, 0.004, lake_max);
    let hydro = wh.hydro;
    let mut lakes = wh.lakes;
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, &hydro.filled, 0.5, 1.0, &lakes);
    persist_rivers(&conn, &extracted_rivers);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);
    persist_lakes(&conn, &lakes);

    // Phase 6: Soil & fertility
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 6b: Ecological biomes. After soil (it reads the soil fingerprints) and
    // after rivers/lakes (they drive the azonal wetland & riparian biomes).
    crate::sim::biome::classify_biomes(&mut buf, &extracted_rivers, &lakes);

    // Phase 7: Settlements
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Organic culture map first, so settlements are named in their region's culture.
    let desired_cultures = crate::db::metadata::get_meta(&conn, "culture_count").ok().flatten()
        .and_then(|s| s.parse::<usize>().ok()).filter(|&n| n >= 1);
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed, desired_cultures);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let hab_fields = settlements::compute_habitability_fields(&buf, &extracted_rivers, &lakes, Some(&hydro.acc));
    let mut generated_settlements = settlements::generate_settlements(&buf, &hab_fields.hab, &extracted_rivers, seed, 0.55, None);
    // Step 7a (CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 3) — junction sites
    // (straits, isthmuses, mountain passes, great river mouths) the base pass' local-
    // maxima-of-habitability search structurally cannot find, since a great port need
    // not sit on the best farmland. Runs after the base pass (so it can respect
    // spacing from it) and before province generation.
    generated_settlements.extend(settlements::generate_trade_sites(
        &buf, &hab_fields.trade, &generated_settlements, 0.55,
    ));
    settlements::write_habitability(&mut buf, &hab_fields.hab);

    // Phase 7b (WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 3): partition into
    // provinces, incl. step 7a's junction sites, and auto-merge slivers. Neither
    // run-all called this before — "Generate Full World" ended at phase 8 and left
    // the province layer unreachable except from the standalone Settlements/
    // Provinces step panels.
    generate_and_persist_provinces(&conn, &buf, &extracted_rivers, &generated_settlements, 0.5)?;

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    let (ore, localities, goods_report) = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
    persist_goods_placement(&conn, &ore, &localities, &goods_report)?;
    // Terminal salt lakes → brine into the salinity column + inland salt-pan goods.
    biological::apply_salt_pans(&mut buf, &lakes, &goods);

    let modified = buf.save(&conn, "Full world generation")?;

    Ok(SimRunAllResult {
        modified,
        rivers: extracted_rivers,
        lakes,
        settlements: generated_settlements,
    })
}

/// Generate elevation from existing terrain (for template-based worlds).
/// Uses distance-from-coast + noise ridges instead of plate boundaries.
#[tauri::command]
pub fn sim_generate_terrain_from_template(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_from_terrain(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    // Generate a proper continental shelf here too. Previously only the
    // all-in-one "Complete from Landmass" ran generate_shelves; the per-step
    // path left just compute_sea_depth's ~4-cell ring (≈1px at 3600-wide → the
    // Shelf layer looked empty). This makes per-step match Run-All.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate elevation from template")
}

/// Alternative elevation model: plate-free, world-size-aware ridged cordillera
/// (mountain count scales with the map) + erosion. Keeps the existing landmass.
#[tauri::command]
pub fn sim_generate_terrain_ridged(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_ridged(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate ridged elevation")
}

/// Elevation model: a **cordillera** — long continuous chains traced along the
/// continental margin, with a continental divide, asymmetric flanks (steep
/// seaward, broad inland piedmont) and parallel sub-ranges. Keeps the existing
/// landmass. See `elevation::generate_elevation_cordillera`.
#[tauri::command]
pub fn sim_generate_terrain_cordillera(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_cordillera(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate cordillera elevation")
}

/// Elevation model: parallel fault blocks — a tilted, asymmetric horst-and-graben
/// rift system. Strike follows the world's own divergent-boundary trend where
/// plate data exists. See `elevation::generate_elevation_rift`.
#[tauri::command]
pub fn sim_generate_terrain_rift(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_rift(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate rift elevation")
}

/// Elevation model: the shape model, then glacial modification — U-valley
/// broadening, cirque hollows, over-deepened troughs that breach the coast (real
/// fjords, carved rather than notched). May turn some land cells to sea near the
/// coast, so this is the one non-cordillera/ridged/rift model that can change the
/// coastline. See `elevation::generate_elevation_glaciated`.
#[tauri::command]
pub fn sim_generate_terrain_glaciated(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_glaciated(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate glaciated elevation")
}

/// Elevation model: quantised levels with sharp escarpment rims + outlying
/// buttes. See `elevation::generate_elevation_plateau`.
#[tauri::command]
pub fn sim_generate_terrain_plateau(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_plateau(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate plateau elevation")
}

/// Elevation model: shield cones on `is_volcanic` cells, summit calderas on the
/// densest clusters, hotspot trails from isolated seeds. See
/// `elevation::generate_elevation_volcanic`.
#[tauri::command]
pub fn sim_generate_terrain_volcanic(
    seed: u64,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_elevation_volcanic(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    buf.save(&conn, "Generate volcanic elevation")
}

/// Generate mountain ridges from hand-drawn ridge lines.
/// Each line carries its polyline spine, footprint width, peak height and
/// ruggedness; the backend widens them into naturally eroded ranges, SCREEN-
/// blended onto the existing elevation (works on a flat world too), LAND ONLY,
/// with erosion confined to the new ridge footprints so existing terrain is kept.
#[tauri::command]
pub fn sim_generate_ridges(
    lines_json: String,
    seed: u64,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let lines: Vec<elevation::RidgeLine> = serde_json::from_str(&lines_json)
        .map_err(|e| format!("Invalid ridge lines: {e}"))?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_ridges(&mut buf, seed, &lines);
    buf.save(&conn, "Generate ridges")
}

/// Freehand area tools for the Landmass step (`ITCZ_AND_LAND_TOOLS_PLAN.md`
/// Commit 1). Each takes the lasso polygon as JSON — the same shape
/// `sim_generate_ridges` already uses for `linesJson` — mutates only the cells
/// inside it (feathered at the edge), and returns the modified tile coords for
/// invalidation. Every op loads `PHASE_PLATES`, so they run at the Landmass
/// step, before elevation/ocean/climate exist.
fn parse_lasso(lasso_json: &str, world_w: u32) -> Result<landmass_ops::Lasso, String> {
    let points: Vec<(f32, f32)> = serde_json::from_str(lasso_json)
        .map_err(|e| format!("Invalid lasso polygon: {e}"))?;
    Ok(landmass_ops::Lasso::new(points, world_w))
}

#[tauri::command]
pub fn land_op_smooth_roughen(
    lasso_json: String,
    amount: f32,
    seed: u64,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES)?;
    let lasso = parse_lasso(&lasso_json, buf.width)?;
    landmass_ops::smooth_roughen(&mut buf, &lasso, amount, seed);
    buf.save(&conn, if amount < 0.0 { "Smooth coastline" } else { "Roughen coastline" })
}

#[tauri::command]
pub fn land_op_fjords(
    lasso_json: String,
    count: u32,
    length_km: f32,
    width: f32,
    seed: u64,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES)?;
    let lasso = parse_lasso(&lasso_json, buf.width)?;
    landmass_ops::fjords(&mut buf, &lasso, count, length_km, width, seed);
    buf.save(&conn, "Carve fjords")
}

#[tauri::command]
pub fn land_op_islands(
    lasso_json: String,
    count: u32,
    kind: String,
    size: f32,
    seed: u64,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES)?;
    let lasso = parse_lasso(&lasso_json, buf.width)?;
    let kind: landmass_ops::IslandKind = serde_json::from_str(&format!("\"{kind}\""))
        .map_err(|e| format!("Invalid island kind: {e}"))?;
    landmass_ops::island_chain(&mut buf, &lasso, count, kind, size, seed);
    buf.save(&conn, "Place islands")
}

#[tauri::command]
pub fn land_op_fill(
    lasso_json: String,
    land: bool,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES)?;
    let lasso = parse_lasso(&lasso_json, buf.width)?;
    landmass_ops::fill(&mut buf, &lasso, land);
    buf.save(&conn, if land { "Fill land" } else { "Fill sea" })
}

/// Run full simulation pipeline while preserving existing terrain.
/// For template-based worlds: elevation → shelves → ocean/atmo → climate → rivers → soil → settlements.
#[tauri::command]
pub fn sim_run_all_from_terrain(
    seed: u64,
    elev_mode: String,
    mountain_density: f32,
    mountain_height: f32,
    mountain_spread: f32,
    noise_roughness: f32,
    db: State<'_, WorldDb>,
) -> Result<SimRunAllResult, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load(&conn)?;

    // Phase 2: Elevation from the existing land mask. No plate data here, so the
    // tectonic model is unavailable and "plates" degrades to the shape model.
    apply_elevation_model(&mut buf, seed, &elev_mode,
        mountain_density, mountain_height, mountain_spread, noise_roughness, false);
    elevation::compute_sea_depth(&mut buf);
    // Phase 2b: continental shelves (default params) — previously omitted here
    // despite the doc-comment claiming it ran, so template worlds had no visible
    // shelf and broken upwelling/fisheries.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);

    // Phase 3: Ocean & atmosphere (salinity before currents for thermohaline coupling)
    ocean::compute_wind_belts(&mut buf);
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    // Close the SST loop: the currents just generated now feed back into the stored
    // sea-surface-temperature field (latitude + current anomaly) used by rendering.
    ocean::compute_sst(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    // Seasonal sea ice on shallow high-latitude shelf seas (Hudson Bay / Okhotsk):
    // compute the freeze index once, let it reinforce cold currents (brine
    // rejection) BEFORE temperature so the current-influence pass sees the new
    // cold tags, then apply the "refrigerator" cooling AFTER temperature (which
    // rewrites the field from scratch, so an earlier cooling would be lost).
    let sea_freeze = ocean::compute_shelf_freeze(&buf);
    ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
    // Energy-balance seasonality: derive the seasonal temperature span from insolation
    // × heat-capacity attenuation (replaces the fabricated latitude range Köppen used),
    // then apply a bounded ice/snow-albedo cooling feedback. Runs after all mean-
    // temperature adjustments so it reads the final annual mean.
    temperature::compute_seasonal_amplitude(&mut buf);
    temperature::apply_ice_albedo_feedback(&mut buf);
    // Low-level jets (Somali jet et al.) must precede precipitation: their
    // entrance/exit acceleration reshapes where rain falls, and the Wind Speed
    // layer reads the speed field they write.
    jets::compute_low_level_jets(&mut buf);
    precipitation::compute_precipitation(&mut buf);

    // Phase 4: Climate
    koppen::classify_koppen(&mut buf);

    // Phase 5: Rivers (default river/lake parameters). Lakes first so channel
    // extraction can stop rivers at lake shores instead of crossing the water.
    let lake_max = (buf.total() / 2000).max(20);
    let wh = rivers::compute_world_hydrology(&buf, 0.004, lake_max);
    let hydro = wh.hydro;
    let mut lakes = wh.lakes;
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, &hydro.filled, 0.5, 1.0, &lakes);
    persist_rivers(&conn, &extracted_rivers);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);
    persist_lakes(&conn, &lakes);

    // Phase 6: Soil & fertility
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 6b: Ecological biomes. After soil (it reads the soil fingerprints) and
    // after rivers/lakes (they drive the azonal wetland & riparian biomes).
    crate::sim::biome::classify_biomes(&mut buf, &extracted_rivers, &lakes);

    // Phase 7: Settlements
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Organic culture map first, so settlements are named in their region's culture.
    let desired_cultures = crate::db::metadata::get_meta(&conn, "culture_count").ok().flatten()
        .and_then(|s| s.parse::<usize>().ok()).filter(|&n| n >= 1);
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed, desired_cultures);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let hab_fields = settlements::compute_habitability_fields(&buf, &extracted_rivers, &lakes, Some(&hydro.acc));
    let mut generated_settlements = settlements::generate_settlements(&buf, &hab_fields.hab, &extracted_rivers, seed, 0.55, None);
    // Step 7a (CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 3) — junction sites
    // (straits, isthmuses, mountain passes, great river mouths) the base pass' local-
    // maxima-of-habitability search structurally cannot find, since a great port need
    // not sit on the best farmland. Runs after the base pass (so it can respect
    // spacing from it) and before province generation.
    generated_settlements.extend(settlements::generate_trade_sites(
        &buf, &hab_fields.trade, &generated_settlements, 0.55,
    ));
    settlements::write_habitability(&mut buf, &hab_fields.hab);

    // Phase 7b (WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 3) — see the identical
    // call in `sim_run_all`; this path was missing it too.
    generate_and_persist_provinces(&conn, &buf, &extracted_rivers, &generated_settlements, 0.5)?;

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    let (ore, localities, goods_report) = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
    persist_goods_placement(&conn, &ore, &localities, &goods_report)?;
    // Terminal salt lakes → brine into the salinity column + inland salt-pan goods.
    biological::apply_salt_pans(&mut buf, &lakes, &goods);

    let modified = buf.save(&conn, "Full generation from template")?;

    Ok(SimRunAllResult {
        modified,
        rivers: extracted_rivers,
        lakes,
        settlements: generated_settlements,
    })
}

/// Scale land elevation by a factor, optionally locking the highest peaks.
/// `scale` multiplies every land cell's normalized elevation; if
/// `lock_peaks_above` < 1.0, cells at or above that normalized height keep their
/// value (raise/lower the lowlands without flattening or inflating the peaks).
#[tauri::command]
pub fn sim_scale_elevation(
    scale: f32,
    lock_peaks_above: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::scale_elevation(&mut buf, scale, lock_peaks_above);
    buf.save(&conn, "Scale elevation")
}

/// Generate continental shelves with configurable parameters.
#[tauri::command]
pub fn sim_generate_shelves(
    seed: u64,
    shelf_width: f32,
    noise_amount: f32,
    depth_profile: f32,
    dropoff_width: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION)?;
    elevation::generate_shelves(&mut buf, seed, shelf_width, noise_amount, depth_profile, dropoff_width);
    buf.save(&conn, "Generate shelves")
}

/// Generate optimal settlement locations based on habitability scoring.
/// Phase 7: Habitability → settlements
#[tauri::command]
pub fn sim_generate_settlements(
    seed: u64,
    rivers_json: String,
    realism: Option<f32>,
    max_settlements: Option<u32>,
    db: State<'_, WorldDb>,
) -> Result<SimSettlementsResult, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_SETTLEMENTS)?;

    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json)
        .unwrap_or_default();

    // The world's OWN stored lake set (persist_lakes/load_lakes) — never a fresh
    // detect_lakes at a hard-coded depth. Settlement placement must avoid exactly
    // the lakes that get drawn, or towns end up under water (see `persist_lakes`).
    let hydro = rivers::compute_hydrology(&buf);
    let lakes = load_lakes(&conn, &buf, &hydro.filled);

    // Malaria/fever (needed before habitability so disease suppresses settlement).
    biological::compute_disease_risk(&mut buf, &river_data);
    // Organic culture map — compute + store + activate BEFORE naming settlements so
    // each town is named in its region's (mutated) culture.
    let desired_cultures = crate::db::metadata::get_meta(&conn, "culture_count").ok().flatten()
        .and_then(|s| s.parse::<usize>().ok()).filter(|&n| n >= 1);
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed, desired_cultures);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let hab_fields = settlements::compute_habitability_fields(&buf, &river_data, &lakes, Some(&hydro.acc));
    let mut result = settlements::generate_settlements(
        &buf, &hab_fields.hab, &river_data, seed, realism.unwrap_or(0.55),
        max_settlements.map(|c| c as usize));
    // Step 7a (CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 3) — see the run-all
    // call sites for the full rationale.
    result.extend(settlements::generate_trade_sites(
        &buf, &hab_fields.trade, &result, realism.unwrap_or(0.55),
    ));

    // Persist the habitability field so the Habitability heatmap layer can render.
    settlements::write_habitability(&mut buf, &hab_fields.hab);
    let modified = buf.save(&conn, "Settlements & habitability")?;

    // Record the exact inputs this settlement set was generated from, in WORLD
    // metadata (so they survive a `.worldforge` save). Placement is deterministic in
    // (tiles, rivers, seed, realism, cap), so storing them is what lets a world whose
    // human layer was lost regenerate the SAME towns — with the same ids — instead of
    // a different set that the frozen province layer no longer references.
    let _ = metadata::set_meta(&conn, "settlements_seed", &seed.to_string());
    let _ = metadata::set_meta(&conn, "settlements_realism", &realism.unwrap_or(0.55).to_string());
    let _ = metadata::set_meta(
        &conn,
        "settlements_max",
        &max_settlements.map(|c| c.to_string()).unwrap_or_default(),
    );

    Ok(SimSettlementsResult { modified, settlements: result })
}

#[derive(serde::Serialize)]
pub struct SimSettlementsResult {
    pub modified: Vec<(i32, i32)>,
    pub settlements: Vec<settlements::Settlement>,
}

#[derive(serde::Serialize)]
pub struct SimRunAllResult {
    pub modified: Vec<(i32, i32)>,
    pub rivers: Vec<rivers::River>,
    pub lakes: Vec<rivers::Lake>,
    pub settlements: Vec<settlements::Settlement>,
}

// ── #26 · Geographic toponyms (optional, gated, editable) ────────────────────

/// Generate culture-appropriate names for rivers, mountains, lakes and regions.
/// GATED: requires the culture map (Settlements step) and rivers (Rivers step);
/// errors clearly otherwise. The result is persisted in world metadata and may be
/// edited later via `save_toponyms`. Naming a world's geography does not change
/// its geography, so this is allowed whether or not the world is finalized.
#[tauri::command]
pub fn sim_generate_toponyms(
    rivers_json: String,
    lakes_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<toponyms::Toponym>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::sim::cultures::ensure_active(&conn);
    if crate::sim::cultures::active().is_none() {
        return Err("Generate settlements first — toponyms need the culture map (run the Settlements step).".into());
    }
    let rivers: Vec<rivers::River> = serde_json::from_str(&rivers_json).unwrap_or_default();
    let lakes: Vec<rivers::Lake> = serde_json::from_str(&lakes_json).unwrap_or_default();
    // Toponyms also name MOUNTAINS and REGIONS, so rivers aren't strictly required —
    // a river-poor world (or an old save whose river overlay didn't reload) can still
    // be named. Only the culture map (above) is mandatory.
    // KOPPEN is required by the biome-subregion namer (desert/forest/tundra); without
    // it `buf.koppen` is empty and indexing it panics → app crash on this step.
    let buf = WorldBuffer::load_with(&conn, ColumnSet::TERRAIN | ColumnSet::ELEVATION | ColumnSet::KOPPEN)?;
    let list = toponyms::generate(&buf, &rivers, &lakes);
    metadata::set_meta(&conn, "toponyms", &serde_json::to_string(&list).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(list)
}

/// Persist a user-edited toponym list (renames). Validates it parses first.
#[tauri::command]
pub fn save_toponyms(toponyms_json: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let _: Vec<toponyms::Toponym> = serde_json::from_str(&toponyms_json).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "toponyms", &toponyms_json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load the persisted toponym list (empty if none generated yet).
#[tauri::command]
pub fn get_toponyms(db: State<'_, WorldDb>) -> Result<Vec<toponyms::Toponym>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(metadata::get_meta(&conn, "toponyms").map_err(|e| e.to_string())?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

// ── Province partition (runs AFTER settlements; a separate layer) ──────────────

#[derive(serde::Serialize)]
pub struct SimProvincesResult {
    pub provinces: Vec<crate::sim::provinces::Province>,
    /// Downsampled per-cell province id for the campaign hub→province mapping + the
    /// mini-map (row-major, `NO_PROVINCE` = sea/no-data). Capped so the payload is small.
    pub raster: Vec<u32>,
    pub raster_w: u32,
    pub raster_h: u32,
    pub grid_w: u32,
    pub grid_h: u32,
    /// FULL-RESOLUTION per-cell province id, run-length encoded for transport as a flat
    /// `[value, count, value, count, …]` list (provinces are contiguous so this stays
    /// tiny). Decoded on the frontend to a full grid_w×grid_h map → pixel-exact borders
    /// that follow the coastline, with no cell grid.
    pub raster_rle: Vec<u32>,
}

/// Run-length encode a full-resolution province-id map into a flat `[val, count, …]`
/// list (val is the province id; count is the run length).
fn rle_encode_provinces(ids: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i < ids.len() {
        let v = ids[i];
        let mut n = 1u32;
        while i + (n as usize) < ids.len() && ids[i + n as usize] == v { n += 1; }
        out.push(v);
        out.push(n);
        i += n as usize;
    }
    out
}

/// WORLD_AND_TRADE_MASTER_PLAN.md Part I Slice 3: any province below this km² floor
/// is folded into its largest-shared-border neighbour by the automatic merge every
/// province-generating path now runs — "this is a generation artefact, not a
/// province" (~8,000 km²). Stated in km², not cells (CLAUDE.md rule 25): a cell is
/// `KM_EQUATOR / w` km wide, so a fixed cell count means a different real area on
/// every world size.
/// WORLD_AND_TRADE_MASTER_PLAN.md Part III §4 (transport modes) — persist the
/// just-extracted river geometry to metadata the moment it is computed, rather
/// than only when the frontend happens to save the world. Before this, the
/// `rivers` metadata key was written ONLY by `persist_overlays` (called right
/// before a manual save), so `commands::query_commands::compute_route_days_
/// matrix` — which builds the campaign's real pathfound `base_days` at
/// `campaign_start_sim` — had NO river data to work with on the single most
/// common flow (generate a world, start the campaign, never having saved and
/// reloaded first): `is_river`/`is_nav_river` were always empty there, so a
/// navigable river's `build_coarse_cost` discount (sea:river:road ≈ 1:4:8,
/// §7's own Masschaele-calibrated ratio) never fired for a single founding-hub
/// route, however navigable the river actually was. Called at every one of the
/// four sites that computes `extracted_rivers`, so the metadata key is always
/// in sync with the world's current hydrology; a later manual save's own
/// `persist_overlays` call just overwrites it with the identical JSON.
fn persist_rivers(conn: &rusqlite::Connection, rivers: &[rivers::River]) {
    if let Ok(json) = serde_json::to_string(rivers) {
        let _ = metadata::set_meta(conn, "rivers", &json);
    }
}

/// Drop `Distribution::Manufactured` goods from every province's goods
/// shortlist before it reaches the frontend.
///
/// A manufactured good is MADE IN A CITY from a recipe (`GoodSpec.inputs`) — it
/// has no belt, no deposit and no ground it grows on, so listing it among a
/// province's land goods is a category error: the Province Inspector was showing
/// "Books & Manuscripts" and "Incense" beside grain and timber, with quality
/// stars, as though the countryside produced them.
///
/// `Province.goods` is built in `generate_provinces` from the tile `goods`
/// columns, which has no access to the goods SPEC and so cannot tell a belt good
/// from a manufactured one — it only sees bytes. A manufactured good's column
/// should be all zeros and drop out at the quality floor, but does not always:
/// good indices are FIXED positions in `TileData.goods` (rule 7), so a spec
/// edited after generation, or a retired good's slot, leaves stray non-zero
/// bytes in a column whose good is now manufactured.
///
/// Filtering HERE rather than at generation is deliberate: it fixes worlds that
/// already exist, with no regeneration, and it uses the spec — the only thing
/// that actually knows a good's distribution.
fn strip_manufactured_from_province_goods(
    conn: &rusqlite::Connection,
    mut provinces: Vec<crate::sim::provinces::Province>,
) -> Vec<crate::sim::provinces::Province> {
    let specs = crate::commands::goods_commands::load_world_goods(conn);
    let manufactured: Vec<bool> = specs.iter()
        .map(|s| matches!(s.distribution, crate::sim::goods_spec::Distribution::Manufactured))
        .collect();
    if manufactured.iter().all(|&m| !m) { return provinces; }
    for p in &mut provinces {
        p.goods.retain(|g| !manufactured.get(g.good as usize).copied().unwrap_or(false));
        // `rank`/`of` describe a good's standing among provinces and are set in a
        // later pass over the whole list; leaving them is correct here because
        // removing an entry cannot change another entry's rank.
    }
    provinces
}

/// Persist the lake set beside the rivers, under `metadata["lakes"]`.
///
/// Lakes used to be the ONE hydrology product with no stored copy: every
/// consumer that needed them called `detect_lakes` again with its own
/// hard-coded `fill_depth` of 0.004, while the set the user actually sees was
/// built in `sim_rivers_hydrology` from the `lakeFillDepth` SLIDER. Those two
/// disagree the moment anyone touches the slider — and settlement placement was
/// one of the consumers, so towns were sited to avoid one set of lakes and then
/// drawn under a different, larger one. Same class of bug as the hand-copied
/// colour tables in §8.18, and the same fix: keep one copy, so there is nothing
/// to drift.
fn persist_lakes(conn: &rusqlite::Connection, lakes: &[rivers::Lake]) {
    if let Ok(json) = serde_json::to_string(lakes) {
        let _ = metadata::set_meta(conn, "lakes", &json);
    }
}

/// TECTONICS_AND_ISOLATION_PLAN.md Part B2 — persist each plate's Euler-pole
/// motion under `metadata["plate_motion"]`, the same one-shot-generator-output
/// pattern `deposits`/`good_localities`/`lakes` already use. Called from every
/// site that runs phase 1, so a re-generated world always has a fresh motion
/// layer rather than one left over from a previous plate count or seed.
fn persist_plate_motion(conn: &rusqlite::Connection, motion: &[plates::PlateMotion]) {
    if let Ok(json) = serde_json::to_string(motion) {
        let _ = metadata::set_meta(conn, "plate_motion", &json);
    }
}

/// The world's stored lake set. Falls back to recomputing at the old hard-coded
/// depth ONLY for a world generated before lakes were persisted — a fallback,
/// never the normal path, so an old save still places settlements sensibly
/// instead of behaving as though the world had no lakes at all.
fn load_lakes(
    conn: &rusqlite::Connection, buf: &WorldBuffer, filled: &[f32],
) -> Vec<rivers::Lake> {
    if let Ok(Some(json)) = metadata::get_meta(conn, "lakes") {
        if let Ok(lakes) = serde_json::from_str::<Vec<rivers::Lake>>(&json) {
            if !lakes.is_empty() { return lakes; }
        }
    }
    let lake_max = (buf.total() / 2000).max(20);
    rivers::detect_lakes(buf, filled, 0.004, lake_max)
}

const AUTO_MERGE_FLOOR_KM2: f32 = 8000.0;

/// Partition all land into provinces (watershed / cost-flood), then auto-merge
/// slivers below `AUTO_MERGE_FLOOR_KM2`, and persist the three metadata keys
/// (`provinces` / `province_raster` / `province_raster_rle`). Shared by the
/// standalone `sim_generate_provinces` command and both run-alls (Slice 3) so the
/// two paths cannot drift. `granularity` 0..1: coarse (few large) → fine (many).
fn generate_and_persist_provinces(
    conn: &rusqlite::Connection,
    buf: &WorldBuffer,
    river_data: &[rivers::River],
    settle: &[settlements::Settlement],
    granularity: f32,
) -> Result<SimProvincesResult, String> {
    let w = buf.width; let h = buf.height;
    if w == 0 || h == 0 { return Err("world grid not initialised".into()); }

    // Lakes (overlay data, recomputed) — used as impassable divides in the flood.
    let hydro = rivers::compute_hydrology(buf);

    let lakes = load_lakes(conn, buf, &hydro.filled);

    let (provinces, province_id) = crate::sim::provinces::generate_provinces(
        buf, river_data, &lakes, settle, granularity);

    const KM_EQUATOR: f32 = 40075.0;
    let km_per_cell = KM_EQUATOR / w.max(1) as f32;
    let min_cells = ((AUTO_MERGE_FLOOR_KM2 / (km_per_cell * km_per_cell)).round() as u32).max(4);
    let (provinces, province_id) = crate::sim::provinces::merge_small_provinces_wh(
        &province_id, &provinces, min_cells, w, h, None);

    // Persist the province list (frozen partition; campaign state layers on top later).
    metadata::set_meta(conn, "provinces",
        &serde_json::to_string(&provinces).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // Downsample the id map for transport / overlay. A finer cap gives crisper
    // province borders on the map (fewer stair-steps) at a modest payload cost.
    let cap = 768u32;
    let step = ((w.max(h) + cap - 1) / cap).max(1);
    let rw = (w + step - 1) / step;
    let rh = (h + step - 1) / step;
    let mut raster = vec![crate::sim::provinces::NO_PROVINCE; (rw * rh) as usize];
    for ry in 0..rh {
        for rx in 0..rw {
            let sx = (rx * step).min(w - 1);
            let sy = (ry * step).min(h - 1);
            raster[(ry * rw + rx) as usize] = province_id[(sy * w + sx) as usize];
        }
    }

    // Persist the downsampled raster too, so the layer survives a reload and the
    // campaign can map hubs → provinces (read-only foundation).
    let raster_blob = serde_json::to_string(&(rw, rh, w, h, &raster)).map_err(|e| e.to_string())?;
    metadata::set_meta(conn, "province_raster", &raster_blob).map_err(|e| e.to_string())?;

    // Full-resolution RLE for the pixel-exact map overlay (survives reload).
    let raster_rle = rle_encode_provinces(&province_id);
    let rle_blob = serde_json::to_string(&(w, h, &raster_rle)).map_err(|e| e.to_string())?;
    metadata::set_meta(conn, "province_raster_rle", &rle_blob).map_err(|e| e.to_string())?;

    Ok(SimProvincesResult { provinces, raster, raster_w: rw, raster_h: rh, grid_w: w, grid_h: h, raster_rle })
}

/// Partition all land into provinces (watershed / cost-flood). Seeds from the
/// settlements passed in, so this MUST run after the settlement step. Persists the
/// province list to `metadata["provinces"]` and returns it plus a downsampled id
/// raster for rendering. `granularity` 0..1: coarse (few large) → fine (many).
#[tauri::command]
pub fn sim_generate_provinces(
    settlements_json: String,
    rivers_json: String,
    granularity: Option<f32>,
    db: State<'_, WorldDb>,
) -> Result<SimProvincesResult, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Provinces are a FROZEN world layer (phase 7b). Regenerating them mid-campaign
    // recompacts every province id and rewrites the raster the campaign's hub_province /
    // prov_* / realm state was seeded from — so it is blocked once a campaign is running.
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?;
    let buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PROVINCES)?;

    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json).unwrap_or_default();
    let settle: Vec<settlements::Settlement> = serde_json::from_str(&settlements_json).unwrap_or_default();

    generate_and_persist_provinces(&conn, &buf, &river_data, &settle, granularity.unwrap_or(0.5))
}

/// Read back the stored province list (for reopening a world / panel refresh).
#[tauri::command]
pub fn get_provinces(db: State<'_, WorldDb>) -> Result<Vec<crate::sim::provinces::Province>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

/// Read back the FULL province layer (list + downsampled id raster) so reopening a
/// world restores both the panel and the map overlay without recomputing.
#[tauri::command]
pub fn get_province_layer(db: State<'_, WorldDb>) -> Result<SimProvincesResult, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let provinces: Vec<crate::sim::provinces::Province> =
        metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let (raster_w, raster_h, grid_w, grid_h, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or((0, 0, 0, 0, Vec::new()));
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    let (_rw, _rh, mut raster_rle): (u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster_rle").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or((0, 0, Vec::new()));
    crate::sim::provinces::migrate_rle_sentinel(&mut raster_rle);
    let provinces = strip_manufactured_from_province_goods(&conn, provinces);
    Ok(SimProvincesResult { provinces, raster, raster_w, raster_h, grid_w, grid_h, raster_rle })
}

/// What `repair_province_settlements` changed.
#[derive(serde::Serialize)]
pub struct ProvinceRepairReport {
    pub provinces: usize,
    /// Provinces whose town list or seat actually moved.
    pub provinces_changed: usize,
    /// Settlements successfully placed inside a province.
    pub settlements_attached: usize,
    /// Settlements whose cell falls on no province (islands the partition skipped).
    pub settlements_orphaned: usize,
}

/// Re-attach settlements to the EXISTING province partition.
///
/// `Province.settlements` holds settlement IDs, and provinces live in world metadata
/// while settlements lived (until this change) only in the campaign table — so any
/// world whose human layer was rebuilt ends up with provinces naming towns that no
/// longer exist. `sim_generate_provinces` cannot fix it: it is freeze-gated, because
/// regenerating recompacts every province id and rewrites the raster the campaign's
/// `hub_province` / `prov_*` / realm state was seeded from.
///
/// This repairs MEMBERSHIP ONLY — the same "settlements per province, seat = largest
/// population" pass `generate_provinces` runs, replayed against the stored
/// full-resolution raster. No province id changes, no cell changes to the raster, no
/// geometry, so it is safe on a frozen world and is NOT freeze-gated.
#[tauri::command]
pub fn repair_province_settlements(
    settlements_json: String,
    db: State<'_, WorldDb>,
) -> Result<ProvinceRepairReport, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut provinces: Vec<crate::sim::provinces::Province> =
        metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if provinces.is_empty() {
        return Err("No province layer to repair — this world has none.".into());
    }
    let settle: Vec<settlements::Settlement> =
        serde_json::from_str(&settlements_json).map_err(|e| format!("settlements parse: {e}"))?;

    // Full-resolution id map (the downsample would misplace a town by up to `step`
    // cells, which near a border is a different province).
    let (w, h, mut rle): (u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster_rle").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or("province raster not found — this world's province layer cannot be repaired")?;
    crate::sim::provinces::migrate_rle_sentinel(&mut rle);
    let total = (w as usize) * (h as usize);
    if total == 0 { return Err("world grid not initialised".into()); }
    let mut province_id = vec![crate::sim::provinces::NO_PROVINCE; total];
    {
        let mut idx = 0usize;
        let mut i = 0usize;
        while i + 1 < rle.len() && idx < total {
            let v = rle[i];
            let n = rle[i + 1] as usize;
            let end = (idx + n).min(total);
            for c in province_id.iter_mut().take(end).skip(idx) { *c = v; }
            idx = end;
            i += 2;
        }
    }

    // Bucket the towns by province, exactly as `generate_provinces` does.
    let mut by_province: std::collections::HashMap<u32, Vec<(String, u32)>> =
        std::collections::HashMap::new();
    let mut orphaned = 0usize;
    let mut attached = 0usize;
    for st in &settle {
        let x = st.x.min(w.saturating_sub(1));
        let y = st.y.min(h.saturating_sub(1));
        let pid = province_id[(y as usize) * (w as usize) + x as usize];
        if pid == crate::sim::provinces::NO_PROVINCE {
            orphaned += 1;
        } else {
            by_province.entry(pid).or_default().push((st.id.clone(), st.population));
            attached += 1;
        }
    }

    let mut changed = 0usize;
    for pr in provinces.iter_mut() {
        let mut towns = by_province.remove(&pr.id).unwrap_or_default();
        // seat = largest population; ties broken on id so the result is deterministic.
        towns.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let ids: Vec<String> = towns.iter().map(|t| t.0.clone()).collect();
        let mut moved = ids != pr.settlements;
        // A province with towns seats on its largest; one with none keeps the seed
        // cell `generate_provinces` gave it (there is nothing better to point at).
        if let Some(top) = towns.first() {
            if let Some(st) = settle.iter().find(|s| s.id == top.0) {
                let (sx, sy) = (st.x.min(w.saturating_sub(1)), st.y.min(h.saturating_sub(1)));
                if pr.seat_x != sx || pr.seat_y != sy {
                    pr.seat_x = sx;
                    pr.seat_y = sy;
                    moved = true;
                }
            }
        }
        pr.settlements = ids;
        if moved { changed += 1; }
    }

    metadata::set_meta(&conn, "provinces",
        &serde_json::to_string(&provinces).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(ProvinceRepairReport {
        provinces: provinces.len(),
        provinces_changed: changed,
        settlements_attached: attached,
        settlements_orphaned: orphaned,
    })
}

/// POST-GENERATION cleanup: fold every province smaller than a cell threshold into
/// the neighbour it shares the most border with (never an island province) and
/// re-persist the partition. `min_cells` overrides the default, which is 20% of the
/// median province size (floored at 30 cells) — small enough to spare the
/// intentionally-compact fertile provinces, large enough to swallow the sliver
/// artefacts. Returns the full updated layer so the frontend reloads it in place.
#[tauri::command]
pub fn sim_merge_small_provinces(
    min_cells: Option<u32>,
    selected: Option<Vec<u32>>,
    db: State<'_, WorldDb>,
) -> Result<SimProvincesResult, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // frozen once a campaign starts (see sim_generate_provinces)
    let provinces: Vec<crate::sim::provinces::Province> =
        metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if provinces.is_empty() {
        return Err("No provinces to merge — generate the province layer first.".into());
    }
    // Full-resolution id map from the stored RLE (pixel-exact; the downsample would
    // lose slivers, which are exactly what we are trying to measure and remove).
    let (w, h, mut rle): (u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster_rle").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or("province raster not found — regenerate the province layer")?;
    crate::sim::provinces::migrate_rle_sentinel(&mut rle);
    let total = (w as usize) * (h as usize);
    if total == 0 { return Err("world grid not initialised".into()); }
    let mut province_id = vec![crate::sim::provinces::NO_PROVINCE; total];
    {
        let mut idx = 0usize;
        let mut i = 0usize;
        while i + 1 < rle.len() {
            let v = rle[i];
            let cnt = rle[i + 1] as usize;
            let end = (idx + cnt).min(total);
            for slot in &mut province_id[idx..end] { *slot = v; }
            idx += cnt;
            i += 2;
        }
    }

    // Default threshold: 20% of the median province size, floored at 30 cells.
    let min_cells = min_cells.unwrap_or_else(|| {
        let mut sizes: Vec<u32> = provinces.iter().map(|p| p.cells).filter(|&c| c > 0).collect();
        if sizes.is_empty() { return 30; }
        sizes.sort_unstable();
        let median = sizes[sizes.len() / 2];
        ((median as f32 * 0.20) as u32).max(30)
    });

    let only: Option<std::collections::HashSet<u32>> =
        selected.filter(|v| !v.is_empty()).map(|v| v.into_iter().collect());
    let (new_provinces, new_pid) = crate::sim::provinces::merge_small_provinces_wh(
        &province_id, &provinces, min_cells, w, h, only.as_ref());

    // Persist the merged partition (list + downsampled raster + full-res RLE), exactly
    // as `sim_generate_provinces` does, so a reload restores the cleaned layer.
    metadata::set_meta(&conn, "provinces",
        &serde_json::to_string(&new_provinces).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    let cap = 768u32;
    let step = ((w.max(h) + cap - 1) / cap).max(1);
    let rw = (w + step - 1) / step;
    let rh = (h + step - 1) / step;
    let mut raster = vec![crate::sim::provinces::NO_PROVINCE; (rw * rh) as usize];
    for ry in 0..rh {
        for rx in 0..rw {
            let sx = (rx * step).min(w - 1);
            let sy = (ry * step).min(h - 1);
            raster[(ry * rw + rx) as usize] = new_pid[(sy * w + sx) as usize];
        }
    }
    let raster_blob = serde_json::to_string(&(rw, rh, w, h, &raster)).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "province_raster", &raster_blob).map_err(|e| e.to_string())?;
    let raster_rle = rle_encode_provinces(&new_pid);
    let rle_blob = serde_json::to_string(&(w, h, &raster_rle)).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "province_raster_rle", &rle_blob).map_err(|e| e.to_string())?;

    Ok(SimProvincesResult {
        provinces: new_provinces, raster, raster_w: rw, raster_h: rh,
        grid_w: w, grid_h: h, raster_rle,
    })
}

/// POST-GENERATION "split large" (mirror of `sim_merge_small_provinces`): every
/// NON-POLAR province larger than a cell threshold is cut into compact
/// sub-provinces. `max_cells` overrides the default, which is 2.5× the median
/// province size. Arctic/Antarctic (Köppen ET/EF) provinces are never split.
#[tauri::command]
pub fn sim_split_large_provinces(
    max_cells: Option<u32>,
    rivers_json: Option<String>,
    selected: Option<Vec<u32>>,
    db: State<'_, WorldDb>,
) -> Result<SimProvincesResult, String> {
    db.clear_caches();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // frozen once a campaign starts (see sim_generate_provinces)
    let provinces: Vec<crate::sim::provinces::Province> =
        metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if provinces.is_empty() {
        return Err("No provinces to split — generate the province layer first.".into());
    }
    // The organic split floods over the crest/river feature fields, so it needs the
    // world buffer + rivers/lakes (rivers passed from the frontend overlay state; lakes
    // recomputed cheaply, exactly as `sim_generate_provinces` does).
    let buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PROVINCES)?;
    let river_data: Vec<rivers::River> =
        rivers_json.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let hydro = rivers::compute_hydrology(&buf);

    let lakes = load_lakes(&conn, &buf, &hydro.filled);
    let (w, h, mut rle): (u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster_rle").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or("province raster not found — regenerate the province layer")?;
    crate::sim::provinces::migrate_rle_sentinel(&mut rle);
    let total = (w as usize) * (h as usize);
    if total == 0 { return Err("world grid not initialised".into()); }
    let mut province_id = vec![crate::sim::provinces::NO_PROVINCE; total];
    {
        let mut idx = 0usize;
        let mut i = 0usize;
        while i + 1 < rle.len() {
            let v = rle[i];
            let cnt = rle[i + 1] as usize;
            let end = (idx + cnt).min(total);
            for slot in &mut province_id[idx..end] { *slot = v; }
            idx += cnt;
            i += 2;
        }
    }

    // Default threshold: 2.5× the median province size (only clearly-oversized ones).
    let max_cells = max_cells.unwrap_or_else(|| {
        let mut sizes: Vec<u32> = provinces.iter().map(|p| p.cells).filter(|&c| c > 0).collect();
        if sizes.is_empty() { return u32::MAX; }
        sizes.sort_unstable();
        let median = sizes[sizes.len() / 2];
        ((median as f32 * 2.5) as u32).max(median + 1)
    });

    let only: Option<std::collections::HashSet<u32>> =
        selected.filter(|v| !v.is_empty()).map(|v| v.into_iter().collect());
    let (new_provinces, new_pid) = crate::sim::provinces::split_large_provinces_wh(
        &buf, &river_data, &lakes, &province_id, &provinces, max_cells, only.as_ref());

    metadata::set_meta(&conn, "provinces",
        &serde_json::to_string(&new_provinces).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let cap = 768u32;
    let step = ((w.max(h) + cap - 1) / cap).max(1);
    let rw = (w + step - 1) / step;
    let rh = (h + step - 1) / step;
    let mut raster = vec![crate::sim::provinces::NO_PROVINCE; (rw * rh) as usize];
    for ry in 0..rh {
        for rx in 0..rw {
            let sx = (rx * step).min(w - 1);
            let sy = (ry * step).min(h - 1);
            raster[(ry * rw + rx) as usize] = new_pid[(sy * w + sx) as usize];
        }
    }
    let raster_blob = serde_json::to_string(&(rw, rh, w, h, &raster)).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "province_raster", &raster_blob).map_err(|e| e.to_string())?;
    let raster_rle = rle_encode_provinces(&new_pid);
    let rle_blob = serde_json::to_string(&(w, h, &raster_rle)).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "province_raster_rle", &rle_blob).map_err(|e| e.to_string())?;

    Ok(SimProvincesResult {
        provinces: new_provinces, raster, raster_w: rw, raster_h: rh,
        grid_w: w, grid_h: h, raster_rle,
    })
}

/// A single stat row for a building's hover card (label + preformatted value).
#[derive(serde::Serialize)]
pub struct PStat { pub label: String, pub value: String }

/// A building standing in a province, with its position (world cells) and full stats
/// for the hover tooltip. `kind`: 0 estate · 1 manufactory · 2 warehouse · 3 bank · 4 mint.
#[derive(serde::Serialize)]
pub struct PBuilding {
    pub kind: u8,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub stats: Vec<PStat>,
}

/// A live settlement standing in a province (for the mini-map + list).
#[derive(serde::Serialize)]
pub struct PSettlement {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub seat: bool,
    pub hub_class: u8,
    pub dev_tier: u8,
}

/// The full detail of ONE province for the subwindow: live settlements + all the
/// buildings mapped into it (estates, manufactories, warehouses, banks, mints).
/// Read-only join over the stored partition + the live sim — does NOT touch the tick.
#[derive(serde::Serialize)]
pub struct ProvinceDetail {
    pub id: u32,
    pub rural_pop: u32,
    pub urban_pop: u32,
    pub net_migration: i32,
    pub settlements: Vec<PSettlement>,
    pub buildings: Vec<PBuilding>,
}

#[tauri::command]
pub fn campaign_province_detail(id: u32, db: State<'_, WorldDb>) -> Result<Option<ProvinceDetail>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (rw, rh, gw, gh, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        match metadata::get_meta(&conn, "province_raster").map_err(|e| e.to_string())? {
            Some(s) => serde_json::from_str(&s).unwrap_or((0, 0, 0, 0, Vec::new())),
            None => return Ok(None),
        };
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    if raster.is_empty() || gw == 0 || gh == 0 || rw == 0 || rh == 0 { return Ok(None); }
    let Some(sim) = crate::commands::campaign_commands::get_sim(&db, &conn)? else { return Ok(None); };
    // Map a world cell → raster cell by ratio (resolution-independent — works whatever
    // downsample cap the raster was built at, so this never desyncs from generation).
    let prov_of = |x: f32, y: f32| -> i32 {
        let hx = (x.max(0.0) as u32).min(gw - 1);
        let hy = (y.max(0.0) as u32).min(gh - 1);
        let rx = (hx as u64 * rw as u64 / gw as u64) as u32;
        let ry = (hy as u64 * rh as u64 / gh as u64) as u32;
        raster.get((ry * rw + rx) as usize)
            .map(|&p| if p == crate::sim::provinces::NO_PROVINCE { -1 } else { p as i32 })
            .unwrap_or(-1)
    };
    let house_name = |hi: i32| -> String {
        if hi >= 0 { sim.houses.get(hi as usize).map(|h| h.name.clone()).unwrap_or_default() }
        else { "—".into() }
    };
    let main_good = |prod: &[f32]| -> Option<usize> {
        prod.iter().enumerate().filter(|(_, &v)| v > 0.0)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i)
    };
    let stars = |q: f32| -> String {
        let n = (q * 5.0).round().clamp(0.0, 5.0) as usize;
        format!("{}{}", "★".repeat(n), "☆".repeat(5 - n))
    };

    // Settlements (live non-estate hubs mapped to this province) — seat = largest.
    let mut settlements: Vec<PSettlement> = Vec::new();
    let mut seat_pop = -1.0f32; let mut seat_i = usize::MAX;
    for (i, h) in sim.hubs.iter().enumerate() {
        if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
        if prov_of(h.x, h.y) != id as i32 { continue; }
        if h.population > seat_pop { seat_pop = h.population; seat_i = i; }
        settlements.push(PSettlement {
            name: h.name.clone(), x: h.x, y: h.y, population: h.population.max(0.0) as u32,
            seat: false, hub_class: h.hub_class,
            dev_tier: sim.dev_tier.get(i).copied().unwrap_or(0),
        });
    }
    if seat_i != usize::MAX {
        let sname = sim.hubs[seat_i].name.clone();
        if let Some(s) = settlements.iter_mut().find(|s| s.name == sname) { s.seat = true; }
    }

    // Buildings.
    let mut buildings: Vec<PBuilding> = Vec::new();
    for (i, h) in sim.hubs.iter().enumerate() {
        if !h.is_estate || h.abandoned { continue; }
        if prov_of(h.x, h.y) != id as i32 { continue; }
        // Manufactory vs raw estate: a hub with recipe-driven output reads as a works.
        let mg = main_good(&h.production);
        let is_works = mg.map(|g| !sim.goods[g].inputs.is_empty()).unwrap_or(false);
        let kind = if is_works { 1 } else { 0 };
        let goodn = mg.map(|g| sim.goods[g].name.clone()).unwrap_or_else(|| "—".into());
        let q = mg.map(|g| h.quality.get(g).copied().unwrap_or(0.0)).unwrap_or(0.0);
        let mut stats = vec![
            PStat { label: "Type".into(), value: crate::sim::tick::estate_kind_label(h.estate_kind).into() },
            PStat { label: "Produces".into(), value: goodn },
            PStat { label: "Quality".into(), value: stars(q) },
            PStat { label: "Tier".into(), value: format!("{}", h.estate_tier) },
            PStat { label: "Owner".into(), value: house_name(h.owner_house) },
            PStat { label: "Workers".into(), value: format!("{}", h.population.max(0.0) as u32) },
        ];
        if h.damage > 0.01 { stats.push(PStat { label: "Damage".into(), value: format!("{:.0}%", h.damage * 100.0) }); }
        buildings.push(PBuilding {
            kind, x: h.x, y: h.y,
            name: if h.name.is_empty() { crate::sim::tick::estate_kind_label(h.estate_kind).into() } else { h.name.clone() },
            stats,
        });
    }
    for w in &sim.warehouses {
        let Some(hub) = sim.hubs.get(w.hub as usize) else { continue };
        if prov_of(hub.x, hub.y) != id as i32 { continue; }
        let total: f32 = w.stock.iter().sum();
        buildings.push(PBuilding {
            kind: 2, x: hub.x, y: hub.y, name: format!("Depot at {}", hub.name),
            stats: vec![
                PStat { label: "Owner".into(), value: house_name(w.owner) },
                PStat { label: "Tier".into(), value: format!("{}", w.tier) },
                PStat { label: "Capacity".into(), value: format!("{:.0}", w.capacity) },
                PStat { label: "Stored".into(), value: format!("{:.0}", total) },
            ],
        });
    }
    for b in &sim.banks {
        if b.defunct { continue; }
        let Some(hub) = sim.hubs.get(b.seat as usize) else { continue };
        if prov_of(hub.x, hub.y) != id as i32 { continue; }
        buildings.push(PBuilding {
            kind: 3, x: hub.x, y: hub.y, name: b.name.clone(),
            stats: vec![
                PStat { label: "Reserves".into(), value: format!("{:.0}", b.reserves) },
                PStat { label: "Deposits".into(), value: format!("{:.0}", b.deposits) },
                PStat { label: "Loans".into(), value: format!("{}", b.loans.len()) },
                PStat { label: "House".into(), value: house_name(b.house as i32) },
            ],
        });
    }
    for h in sim.hubs.iter() {
        if !h.has_mint || h.abandoned { continue; }
        if prov_of(h.x, h.y) != id as i32 { continue; }
        buildings.push(PBuilding {
            kind: 4, x: h.x, y: h.y,
            name: if h.coin_name.is_empty() { format!("Mint at {}", h.name) } else { h.coin_name.clone() },
            stats: vec![
                PStat { label: "Coin".into(), value: if h.coin_name.is_empty() { "—".into() } else { h.coin_name.clone() } },
                PStat { label: "At".into(), value: h.name.clone() },
            ],
        });
    }

    let urban: u32 = settlements.iter().map(|s| s.population).sum();
    let rural = sim.prov_rural.get(id as usize).map(|&r| r.max(0.0) as u32)
        .unwrap_or(0);
    let net = sim.prov_net_mig.get(id as usize).map(|&m| m as i32).unwrap_or(0);
    Ok(Some(ProvinceDetail { id, rural_pop: rural, urban_pop: urban, net_migration: net, settlements, buildings }))
}

/// A cropped terrain sample grid over ONE province's bounding box: real elevation,
/// land/sea and biome codes read straight from the world's own tiles — the base
/// layer for the province survey plate (`ProvinceMiniMap`'s "relief" plate), which
/// used to be a flat placeholder fill. CITY_PROVINCE_WAR_PLAN.md §2.3.
///
/// A read-only display-time query, not a tick concern — §5.4 of that plan: this is
/// not a snapshot violation, it costs nothing per campaign year, and the campaign
/// itself never touches a tile. River COURSES are deliberately not returned here:
/// the frontend already holds the world's full river geometry (`worldStore.rivers`,
/// loaded once at open) and can clip it to this province's own raster mask itself,
/// so duplicating that data over IPC on every crop request would be pure waste.
#[derive(serde::Serialize)]
pub struct ProvinceTerrainCrop {
    pub ox: i32,           // world-cell X origin of the sample grid
    pub oy: i32,           // world-cell Y origin
    pub stride: i32,       // world cells between samples
    pub cols: u32,
    pub rows: u32,
    pub elevation: Vec<f32>, // cols*rows, row-major, 0..1 normalized elevation
    pub land: Vec<u8>,       // cols*rows, 1 = land, 0 = sea/lake
    pub biome: Vec<u8>,      // cols*rows, raw `sim::biome` code (0 = unclassified/sea)
}

#[tauri::command]
pub fn get_province_terrain_crop(
    province_id: u32,
    max_dim: u32,
    db: State<'_, WorldDb>,
) -> Result<Option<ProvinceTerrainCrop>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (rw, rh, gw, gh, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        match metadata::get_meta(&conn, "province_raster").map_err(|e| e.to_string())? {
            Some(s) => serde_json::from_str(&s).unwrap_or((0, 0, 0, 0, Vec::new())),
            None => return Ok(None),
        };
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    if raster.is_empty() || gw == 0 || gh == 0 || rw == 0 || rh == 0 { return Ok(None); }

    // Bounding box + sample stride — shared with `province_good_belt_masks` (§F1 /
    // slice 1) so the relief and goods plates can never independently drift apart.
    let Some(geom) = crate::sim::provinces::province_sample_geom(
        province_id, rw, rh, gw, gh, &raster, max_dim,
    ) else { return Ok(None); };
    let (ox, oy, stride, cols, rows) = (geom.ox as i64, geom.oy as i64, geom.stride, geom.cols, geom.rows);

    let world = db.cached_tiles_with_conn(&conn)?;
    let n = (cols * rows) as usize;
    let mut elevation = vec![0.0f32; n];
    let mut land = vec![0u8; n];
    let mut biome = vec![0u8; n];
    for r in 0..rows {
        let wy = (oy + r as i64 * stride as i64).clamp(0, gh as i64 - 1) as u32;
        for c in 0..cols {
            let wx = (ox + c as i64 * stride as i64).clamp(0, gw as i64 - 1) as u32;
            let tc = TileCoord::from_world(wx, wy);
            let tile = world.tile(tc.tx, tc.ty);
            let (lx, ly) = TileCoord::local(wx, wy);
            let idx = (ly * TILE_SIZE + lx) as usize;
            let i = (r * cols + c) as usize;
            elevation[i] = tile.elevation[idx];
            land[i] = tile.terrain[idx];
            biome[i] = tile.biome[idx];
        }
    }

    Ok(Some(ProvinceTerrainCrop { ox: ox as i32, oy: oy as i32, stride, cols, rows, elevation, land, biome }))
}

/// One province's LIVE campaign state (read-only): baseline rural population plus the
/// urban population currently standing in it (Σ of live hubs mapped through the
/// province raster). Pure join over the stored partition + the live sim — it does NOT
/// touch the tick, so the economy dynamics are unchanged.
#[derive(serde::Serialize)]
pub struct ProvinceLive {
    pub id: u32,
    pub rural_pop: u32,
    pub urban_pop: u32,
    pub hub_count: u32,
    /// Net migration this year: negative = the countryside is a SOURCE (people
    /// leaving for the cities), ~0 = settled. (Phase 2b.)
    pub net_migration: i32,
}

#[tauri::command]
pub fn campaign_province_state(db: State<'_, WorldDb>) -> Result<Vec<ProvinceLive>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let provinces: Vec<crate::sim::provinces::Province> =
        metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if provinces.is_empty() { return Ok(vec![]); }
    let (rw, _rh, gw, gh, mut raster): (u32, u32, u32, u32, Vec<u32>) =
        metadata::get_meta(&conn, "province_raster").map_err(|e| e.to_string())?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or((0, 0, 0, 0, Vec::new()));
    crate::sim::provinces::migrate_raster_sentinel(&mut raster);
    let mut urban = std::collections::HashMap::<u32, u32>::new();
    let mut hubs = std::collections::HashMap::<u32, u32>::new();
    // Live rural pool (Phase 2b): read the sim's per-province reservoir when present,
    // else fall back to each province's static baseline.
    let mut live_rural: Vec<f32> = Vec::new();
    let mut net_mig: Vec<f32> = Vec::new();
    if !raster.is_empty() && gw > 0 && gh > 0 {
        // Must match the downsample cap `sim_generate_provinces` used to build the
        // raster (`cap = 768u32` above) — see the matching comment in
        // campaign_commands/lifecycle.rs::seed_campaign_provinces.
        let step = ((gw.max(gh) + 767) / 768).max(1);
        if let Some(sim) = crate::commands::campaign_commands::get_sim(&db, &conn)? {
            if !sim.prov_rural.is_empty() { live_rural = sim.prov_rural.clone(); }
            net_mig = sim.prov_net_mig.clone();
            for hub in sim.hubs.iter() {
                if hub.is_estate || hub.abandoned || hub.population < 1.0 { continue; }
                let hx = (hub.x.max(0.0) as u32).min(gw - 1);
                let hy = (hub.y.max(0.0) as u32).min(gh - 1);
                let ri = ((hy / step) * rw + (hx / step)) as usize;
                if let Some(&pid) = raster.get(ri) {
                    if pid != crate::sim::provinces::NO_PROVINCE {
                        *urban.entry(pid).or_insert(0) += hub.population.max(0.0) as u32;
                        *hubs.entry(pid).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    Ok(provinces.iter().map(|p| ProvinceLive {
        id: p.id,
        rural_pop: live_rural.get(p.id as usize).map(|&r| r.max(0.0) as u32).unwrap_or(p.rural_pop),
        urban_pop: urban.get(&p.id).copied().unwrap_or(0),
        hub_count: hubs.get(&p.id).copied().unwrap_or(0),
        net_migration: net_mig.get(p.id as usize).map(|&m| m as i32).unwrap_or(0),
    }).collect())
}

#[cfg(test)]
mod elevation_model_tests {
    use super::*;
    use crate::db::schema;
    use rusqlite::Connection;

    /// A small world with a real plate map, so every model has something to read.
    fn plate_world(seed: u64) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", "180"), ("grid_height", "90")] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        plates::generate_plates_and_landmass(&mut buf, seed, 10);
        buf
    }

    fn land_elevations(buf: &WorldBuffer) -> Vec<f32> {
        (0..buf.total()).filter(|&i| buf.terrain[i] == 1).map(|i| buf.elevation[i]).collect()
    }

    /// THE POINT OF THE SELECTOR. Both run-alls used to hardcode a generator and
    /// silently discard the user's choice, so "Generate Full World" produced the
    /// same relief however the picker was set. Every mode must now yield a
    /// materially DIFFERENT elevation field from every other.
    #[test]
    fn every_elevation_model_builds_a_different_world() {
        let seed = 4242u64;
        let modes = ["plates", "shape", "cordillera", "ridged", "rift", "glaciated", "plateau", "volcanic"];
        let fields: Vec<(&str, Vec<f32>)> = modes.iter().map(|&m| {
            let mut buf = plate_world(seed);
            apply_elevation_model(&mut buf, seed, m, 0.5, 0.5, 0.5, 0.4, true);
            (m, land_elevations(&buf))
        }).collect();

        for (m, f) in &fields {
            assert!(!f.is_empty(), "model {m} produced no land at all");
            assert!(f.iter().any(|&e| e > 0.05), "model {m} produced a flat world");
        }
        for (i, (ma, fa)) in fields.iter().enumerate() {
            for (mb, fb) in fields.iter().skip(i + 1) {
                let diff = fa.iter().zip(fb.iter()).filter(|(a, b)| (*a - *b).abs() > 1e-4).count();
                let frac = diff as f32 / fa.len() as f32;
                assert!(
                    frac > 0.25,
                    "models {ma} and {mb} agree on {:.1}% of land — the picker is not \
                     reaching the generator", 100.0 * (1.0 - frac)
                );
            }
        }
    }

    /// An unrecognised mode string (an older save, a typo, a future model) must
    /// never leave a world with NO elevation — the silent-blank-world failure this
    /// codebase keeps hitting. It degrades to a real model instead.
    #[test]
    fn an_unknown_model_still_builds_terrain() {
        let seed = 99u64;
        for allow_plates in [true, false] {
            let mut buf = plate_world(seed);
            apply_elevation_model(&mut buf, seed, "no-such-model", 0.5, 0.5, 0.5, 0.4, allow_plates);
            let land = land_elevations(&buf);
            assert!(!land.is_empty(), "no land (allow_plates={allow_plates})");
            assert!(
                land.iter().any(|&e| e > 0.05),
                "unknown model produced a flat world (allow_plates={allow_plates})"
            );
        }
    }

    /// Without plate data the tectonic model is not available, so "plates" must
    /// fall back rather than read an empty `boundary_type` and produce nothing.
    #[test]
    fn the_tectonic_model_falls_back_where_there_are_no_plates() {
        let seed = 7u64;
        let mut buf = plate_world(seed);
        apply_elevation_model(&mut buf, seed, "plates", 0.5, 0.5, 0.5, 0.4, false);
        let land = land_elevations(&buf);
        assert!(land.iter().any(|&e| e > 0.05), "fallback produced a flat world");

        let mut shape = plate_world(seed);
        apply_elevation_model(&mut shape, seed, "shape", 0.5, 0.5, 0.5, 0.4, false);
        assert_eq!(land, land_elevations(&shape), "the fallback must BE the shape model");
    }
}
