use tauri::State;
use crate::db::WorldDb;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{plates, elevation, ocean, temperature, jets, precipitation, koppen, rivers, soil, fertility, settlements, biological, toponyms};
use crate::db::metadata;
use crate::tile::coords::{TileCoord, TILE_SIZE};

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
    let ore = biological::compute_trade_goods(&mut buf, &river_data, seed, gem_deposits, climate_strictness, &goods);
    metadata::set_meta(&conn, "deposits",
        &serde_json::to_string(&ore).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

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
    let ore = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, gem_deposits, climate_strictness, &goods);
    metadata::set_meta(&conn, "deposits",
        &serde_json::to_string(&ore).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
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
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let generated_settlements = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.55, None);
    settlements::write_habitability(&mut buf, &habitability);

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    let ore = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
    metadata::set_meta(&conn, "deposits",
        &serde_json::to_string(&ore).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
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
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let generated_settlements = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.55, None);
    settlements::write_habitability(&mut buf, &habitability);

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    let goods = crate::commands::goods_commands::load_world_goods(&conn);
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    let ore = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
    metadata::set_meta(&conn, "deposits",
        &serde_json::to_string(&ore).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
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
    let buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PROVINCES)?;
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
    metadata::set_meta(&conn, "province_raster", &raster_blob).map_err(|e| e.to_string())?;

    // Full-resolution RLE for the pixel-exact map overlay (survives reload).
    let raster_rle = rle_encode_provinces(&province_id);
    let rle_blob = serde_json::to_string(&(w, h, &raster_rle)).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "province_raster_rle", &rle_blob).map_err(|e| e.to_string())?;

    Ok(SimProvincesResult { provinces, raster, raster_w: rw, raster_h: rh, grid_w: w, grid_h: h, raster_rle })
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
    Ok(SimProvincesResult { provinces, raster, raster_w, raster_h, grid_w, grid_h, raster_rle })
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

    // Bounding box in RASTER cells. The raster is already a small downsample
    // (capped ~384 cells across in `sim_generate_provinces`), so a full scan for
    // one province's footprint is cheap — no need for the two-pass coarse/fine
    // search the frontend uses on the full-resolution province-ID mask.
    let (mut minx, mut miny, mut maxx, mut maxy) = (rw as i64, rh as i64, -1i64, -1i64);
    for ry in 0..rh {
        for rx in 0..rw {
            if raster[(ry * rw + rx) as usize] != province_id { continue; }
            let (rxi, ryi) = (rx as i64, ry as i64);
            if rxi < minx { minx = rxi; }
            if ryi < miny { miny = ryi; }
            if rxi > maxx { maxx = rxi; }
            if ryi > maxy { maxy = ryi; }
        }
    }
    if maxx < 0 { return Ok(None); }

    // Raster bbox → world-cell bbox, padded a raster cell on each side so the crop
    // doesn't clip the province's own edge.
    let to_world_x = |rx: i64| -> i64 { (rx * gw as i64) / rw as i64 };
    let to_world_y = |ry: i64| -> i64 { (ry * gh as i64) / rh as i64 };
    let ox = to_world_x((minx - 1).max(0));
    let oy = to_world_y((miny - 1).max(0));
    let ex = (to_world_x((maxx + 2).min(rw as i64)) - 1).clamp(ox, gw as i64 - 1);
    let ey = (to_world_y((maxy + 2).min(rh as i64)) - 1).clamp(oy, gh as i64 - 1);
    let bw = (ex - ox + 1).max(1);
    let bh = (ey - oy + 1).max(1);

    let stride = (bw.max(bh) / (max_dim.max(1) as i64)).max(1) as i32;
    let cols = (((bw - 1) / stride as i64) + 1).max(1) as u32;
    let rows = (((bh - 1) / stride as i64) + 1).max(1) as u32;

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
        let step = ((gw.max(gh) + 383) / 384).max(1);
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
