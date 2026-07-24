use tauri::State;
use crate::db::WorldDb;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{plates, elevation, ocean, temperature, jets, precipitation, koppen, rivers, soil, fertility, settlements, biological, toponyms};
use crate::db::metadata;

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
    plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
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
    let hydro = rivers::compute_hydrology(&buf);
    // Lakes first: rivers must terminate at lake shores (not draw across them),
    // so channel extraction needs to know which cells are open lake water.
    let mut lakes = rivers::detect_lakes(&buf, &hydro.filled, lake_fill_depth, max_cells);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, river_density, river_width, &lakes);
    // Oxbow backwaters cut off from the meandering lowland reaches (real lakes).
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    // Tag terminal salt lakes (endorheic + arid) so the overlay tints them and the
    // Hydrology panel, goods and settlements agree on the brine.
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);

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
    biological::compute_trade_goods(&mut buf, &river_data, seed, gem_deposits, climate_strictness, &goods);

    // Terminal salt lakes → brine into the salinity column + inland salt-pan
    // production. Lakes are re-derived here (this phase does not receive them).
    let hydro = rivers::compute_hydrology(&buf);
    let lake_max = (buf.total() / 2000).max(20);
    let mut salt_lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);
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
    let hydro = rivers::compute_hydrology(&buf);
    let max_cells = (((buf.total() as f32) * lake_max_fraction.clamp(0.000002, 0.05)) as usize).max(4);
    let mut lakes = rivers::detect_lakes(&buf, &hydro.filled, lake_fill_depth, max_cells);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, river_density, river_width, &lakes);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);

    // Phase 6: soil & fertility (incl. delta floodplain abundance + delta fisheries).
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 8: biological — hazards, trade goods, and inland salt pans.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, gem_deposits, climate_strictness, &goods);
    biological::apply_salt_pans(&mut buf, &lakes, &goods);

    let modified = buf.save(&conn, "Refresh hydrology & biology")?;
    Ok(SimRiversResult { modified, rivers: extracted_rivers, lakes })
}

/// Run all simulations in sequence (full world generation pipeline).
/// This is a convenience command that runs all phases.
#[tauri::command]
pub fn sim_run_all(
    seed: u64,
    plate_count: u32,
    db: State<'_, WorldDb>,
) -> Result<SimRunAllResult, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::commands::campaign_commands::ensure_unfrozen(&conn)?; // geography is frozen once a campaign starts
    let mut buf = WorldBuffer::load(&conn)?;

    // Phase 1: Plates & landmass
    plates::generate_plates_and_landmass(&mut buf, seed, plate_count);

    // Phase 2: Elevation & depth
    elevation::generate_elevation(&mut buf, seed);
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
    let hydro = rivers::compute_hydrology(&buf);
    let lake_max = (buf.total() / 2000).max(20);
    let mut lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0, &lakes);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);

    // Phase 6: Soil & fertility
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 7: Settlements
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Organic culture map first, so settlements are named in their region's culture.
    let desired_cultures = crate::db::metadata::get_meta(&conn, "culture_count").ok().flatten()
        .and_then(|s| s.parse::<usize>().ok()).filter(|&n| n >= 1);
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed, desired_cultures);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let generated_settlements = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.55, None);
    settlements::write_habitability(&mut buf, &habitability);

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
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

/// Run full simulation pipeline while preserving existing terrain.
/// For template-based worlds: elevation → shelves → ocean/atmo → climate → rivers → soil → settlements.
#[tauri::command]
pub fn sim_run_all_from_terrain(
    seed: u64,
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

    // Phase 2: Elevation from terrain (no plates)
    elevation::generate_elevation_from_terrain(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
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
    let hydro = rivers::compute_hydrology(&buf);
    let lake_max = (buf.total() / 2000).max(20);
    let mut lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0, &lakes);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);

    // Phase 6: Soil & fertility
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 7: Settlements
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Organic culture map first, so settlements are named in their region's culture.
    let desired_cultures = crate::db::metadata::get_meta(&conn, "culture_count").ok().flatten()
        .and_then(|s| s.parse::<usize>().ok()).filter(|&n| n >= 1);
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed, desired_cultures);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let generated_settlements = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.55, None);
    settlements::write_habitability(&mut buf, &habitability);

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
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

    // Recompute lakes from the depression-filled surface (lakes are overlay data,
    // not persisted in tiles) so lakeshore sites count in habitability.
    let hydro = rivers::compute_hydrology(&buf);
    let lake_max = (buf.total() / 2000).max(20);
    let lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);

    // Malaria/fever (needed before habitability so disease suppresses settlement).
    biological::compute_disease_risk(&mut buf, &river_data);
    // Organic culture map — compute + store + activate BEFORE naming settlements so
    // each town is named in its region's (mutated) culture.
    let desired_cultures = crate::db::metadata::get_meta(&conn, "culture_count").ok().flatten()
        .and_then(|s| s.parse::<usize>().ok()).filter(|&n| n >= 1);
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed, desired_cultures);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let habitability = settlements::compute_habitability(&buf, &river_data, &lakes);
    let result = settlements::generate_settlements(
        &buf, &habitability, &river_data, seed, realism.unwrap_or(0.55),
        max_settlements.map(|c| c as usize));

    // Persist the habitability field so the Habitability heatmap layer can render.
    settlements::write_habitability(&mut buf, &habitability);
    let modified = buf.save(&conn, "Settlements & habitability")?;

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
    let buf = WorldBuffer::load_with(&conn, ColumnSet::TERRAIN | ColumnSet::ELEVATION)?;
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
    /// Downsampled per-cell province id for the map overlay (row-major, `NO_PROVINCE`
    /// = sea/no-data). `raster_w`×`raster_h`, capped so the payload stays small.
    pub raster: Vec<u16>,
    pub raster_w: u32,
    pub raster_h: u32,
    pub grid_w: u32,
    pub grid_h: u32,
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
    let buf = WorldBuffer::load(&conn)?; // all columns: terrain/elev/koppen/fertility/goods/…
    let w = buf.width; let h = buf.height;
    if w == 0 || h == 0 { return Err("world grid not initialised".into()); }

    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json).unwrap_or_default();
    let settle: Vec<settlements::Settlement> = serde_json::from_str(&settlements_json).unwrap_or_default();

    // Lakes (overlay data, recomputed) — used as impassable divides in the flood.
    let hydro = rivers::compute_hydrology(&buf);
    let lake_max = (buf.total() / 2000).max(20);
    let lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);

    let (provinces, province_id) = crate::sim::provinces::generate_provinces(
        &buf, &river_data, &lakes, &settle, granularity.unwrap_or(0.5));

    // Persist the province list (frozen partition; campaign state layers on top later).
    metadata::set_meta(&conn, "provinces",
        &serde_json::to_string(&provinces).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // Downsample the id map for transport / overlay (cap the long side ≈ 384 cells).
    let cap = 384u32;
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

    Ok(SimProvincesResult { provinces, raster, raster_w: rw, raster_h: rh, grid_w: w, grid_h: h })
}

/// Read back the stored province list (for reopening a world / panel refresh).
#[tauri::command]
pub fn get_provinces(db: State<'_, WorldDb>) -> Result<Vec<crate::sim::provinces::Province>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(metadata::get_meta(&conn, "provinces").map_err(|e| e.to_string())?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}
