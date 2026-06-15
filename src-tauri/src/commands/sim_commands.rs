use tauri::State;
use crate::db::WorldDb;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{plates, elevation, ocean, temperature, precipitation, koppen, rivers, soil, fertility, settlements, biological};

/// Per-phase wall-clock timer, active only when the `WF_PROFILE` env var is set.
/// Prints `[wf-profile] <phase>: <ms>` to stderr at each `lap`, so a full
/// generation reveals which passes dominate (guides further parallelization and
/// proves the off-lock / rayon wins). Zero overhead when disabled.
struct PhaseTimer {
    on: bool,
    last: std::time::Instant,
}
impl PhaseTimer {
    fn new() -> Self {
        Self { on: std::env::var_os("WF_PROFILE").is_some(), last: std::time::Instant::now() }
    }
    fn lap(&mut self, phase: &str) {
        if self.on {
            eprintln!("[wf-profile] {phase}: {:.1}ms", self.last.elapsed().as_secs_f64() * 1000.0);
        }
        self.last = std::time::Instant::now();
    }
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
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_PLATES)
    })?;
    plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Generate plates & landmass")
    })
}

/// Invert land and sea
#[tauri::command]
pub fn sim_invert_terrain(db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_PLATES)
    })?;
    plates::invert_terrain(&mut buf);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Invert terrain")
    })
}

/// Generate elevation from plate tectonics + sea depth.
/// Phase 2: Terrain → elevation + bathymetry
#[tauri::command]
pub fn sim_generate_terrain(seed: u64, db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_ELEVATION)
    })?;
    elevation::generate_elevation(&mut buf, seed);
    elevation::compute_sea_depth(&mut buf);
    // Proper continental shelf (not just compute_sea_depth's thin ~1px ring) so
    // the Shelf layer is populated after the per-step elevation phase.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Generate terrain & depth")
    })
}

/// Run ocean & atmosphere simulation.
/// Phase 3: Winds → currents → upwelling → distance_to_ocean → temperature → precipitation
#[tauri::command]
pub fn sim_ocean_atmosphere(db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE)
    })?;

    ocean::compute_wind_belts(&mut buf);
    // Salinity must be computed before the currents so the moderate-thermohaline
    // coupling (density-driven speed boost + extended warm conveyor) can run
    // inside generate_ocean_currents. Salinity uses a latitude SST estimate (not
    // the temperature field) to avoid a salinity↔temperature↔current cycle.
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    precipitation::compute_precipitation(&mut buf);

    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Ocean & atmosphere simulation")
    })
}

/// Run climate classification.
/// Phase 4: Köppen zones
#[tauri::command]
pub fn sim_classify_climate(db: State<'_, WorldDb>) -> Result<Vec<(i32, i32)>, String> {
    db.clear_caches(); // drop the (soon-stale) decompressed snapshot before allocating world buffers
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_CLIMATE)
    })?;
    koppen::classify_koppen(&mut buf);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Climate classification")
    })
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
    let buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_RIVERS)
    })?; // lock dropped — the D8 hydrology below runs off-lock

    let max_cells = ((buf.total() as f32) * lake_max_fraction.clamp(0.000002, 0.05)) as usize;
    let max_cells = max_cells.max(4);
    let hydro = rivers::compute_hydrology(&buf);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, river_density, river_width);
    let lakes = rivers::detect_lakes(&buf, &hydro.filled, lake_fill_depth, max_cells);

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
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_SOIL_FERTILITY)
    })?;

    // Deserialize rivers from JSON
    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json)
        .unwrap_or_default();

    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &river_data);
    fertility::compute_fertility(&mut buf, &river_data);
    fertility::compute_fisheries(&mut buf, &river_data);

    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Soil & fertility")
    })
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
    // Load the buffer AND the world goods spec up front under one short lock, so
    // the goods-belt compute below runs off-lock.
    let (mut buf, goods) = db.with_conn(|conn| {
        let buf = WorldBuffer::load_with(conn, ColumnSet::PHASE_BIOLOGICAL)?;
        let goods = crate::commands::goods_commands::load_world_goods(conn);
        Ok((buf, goods))
    })?;

    let river_data: Vec<rivers::River> = serde_json::from_str(&rivers_json)
        .unwrap_or_default();

    biological::compute_shark_risk(&mut buf, &river_data);
    biological::compute_shipworm_risk(&mut buf, &river_data);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    biological::compute_trade_goods(&mut buf, &river_data, seed, gem_deposits, climate_strictness, &goods);

    db.with_conn(|conn| {
        buf.save(conn, "Biological (sharks, shipworms, storms, reefs & trade goods)")
    })
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
    // Load the world buffer + goods spec up front under one short lock; all the
    // multi-second compute below runs OFF-lock so tile fetches stay live, with
    // only brief re-locks for the mid-run culture store and the final save.
    let (mut buf, goods) = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        let buf = WorldBuffer::load(conn)?;
        let goods = crate::commands::goods_commands::load_world_goods(conn);
        Ok((buf, goods))
    })?;
    let mut pt = PhaseTimer::new();
    pt.lap("load");

    // Phase 1: Plates & landmass
    plates::generate_plates_and_landmass(&mut buf, seed, plate_count);
    pt.lap("plates");

    // Phase 2: Elevation & depth
    elevation::generate_elevation(&mut buf, seed);
    elevation::compute_sea_depth(&mut buf);
    // Phase 2b: continental shelves (default params). Without this the only
    // shelf-tagged cells are the thin 1-shelf ring compute_sea_depth marks, so
    // the shelf layer looked empty and upwelling/fisheries had almost nothing to
    // work with.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    pt.lap("elevation");

    // Phase 3: Ocean & atmosphere (salinity before currents for thermohaline coupling)
    ocean::compute_wind_belts(&mut buf);
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    precipitation::compute_precipitation(&mut buf);
    pt.lap("ocean_atmosphere");

    // Phase 4: Climate
    koppen::classify_koppen(&mut buf);
    pt.lap("climate");

    // Phase 5: Rivers (default river/lake parameters)
    let hydro = rivers::compute_hydrology(&buf);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0);
    let lake_max = (buf.total() / 2000).max(20);
    let lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);
    pt.lap("rivers");

    // Phase 6: Soil & fertility
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);
    pt.lap("soil_fertility");

    // Phase 7: Settlements
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Organic culture map first, so settlements are named in their region's
    // culture. `store_and_activate` both persists the map and sets the
    // process-global that settlement naming reads, so it must run BEFORE
    // generate_settlements — a brief re-lock just for that metadata write.
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed);
    db.with_conn(|conn| {
        crate::sim::cultures::store_and_activate(conn, cmap).map_err(|e| e.to_string())
    })?;
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let generated_settlements = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.55);
    settlements::write_habitability(&mut buf, &habitability);
    pt.lap("settlements");

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);
    pt.lap("biological");

    let modified = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Full world generation")
    })?;
    pt.lap("save");

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
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_ELEVATION)
    })?;
    elevation::generate_elevation_from_terrain(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    // Generate a proper continental shelf here too. Previously only the
    // all-in-one "Complete from Landmass" ran generate_shelves; the per-step
    // path left just compute_sea_depth's ~4-cell ring (≈1px at 3600-wide → the
    // Shelf layer looked empty). This makes per-step match Run-All.
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Generate elevation from template")
    })
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
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_ELEVATION)
    })?;
    elevation::generate_elevation_ridged(&mut buf, seed, mountain_density, mountain_height, mountain_spread, noise_roughness);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Generate ridged elevation")
    })
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
    // Load buffer + goods up front; compute phases run off-lock (see sim_run_all).
    let (mut buf, goods) = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        let buf = WorldBuffer::load(conn)?;
        let goods = crate::commands::goods_commands::load_world_goods(conn);
        Ok((buf, goods))
    })?;

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
    ocean::compute_distance_to_ocean(&mut buf);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    precipitation::compute_precipitation(&mut buf);

    // Phase 4: Climate
    koppen::classify_koppen(&mut buf);

    // Phase 5: Rivers (default river/lake parameters)
    let hydro = rivers::compute_hydrology(&buf);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0);
    let lake_max = (buf.total() / 2000).max(20);
    let lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);

    // Phase 6: Soil & fertility
    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    // Phase 7: Settlements
    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Organic culture map first (brief re-lock for the metadata write — see
    // sim_run_all); must precede generate_settlements for in-culture naming.
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed);
    db.with_conn(|conn| {
        crate::sim::cultures::store_and_activate(conn, cmap).map_err(|e| e.to_string())
    })?;
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let generated_settlements = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.55);
    settlements::write_habitability(&mut buf, &habitability);

    // Phase 8: Biological — shark + shipworm waters + trade-good belts.
    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 6, 0.5, &goods);

    let modified = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Full generation from template")
    })?;

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
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_ELEVATION)
    })?;
    elevation::scale_elevation(&mut buf, scale, lock_peaks_above);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Scale elevation")
    })
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
    let mut buf = db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?; // geography is frozen once a campaign starts
        WorldBuffer::load_with(conn, ColumnSet::PHASE_ELEVATION)
    })?;
    elevation::generate_shelves(&mut buf, seed, shelf_width, noise_amount, depth_profile, dropoff_width);
    db.with_conn(|conn| {
        crate::commands::campaign_commands::ensure_unfrozen(conn)?;
        buf.save(conn, "Generate shelves")
    })
}

/// Generate optimal settlement locations based on habitability scoring.
/// Phase 7: Habitability → settlements
#[tauri::command]
pub fn sim_generate_settlements(
    seed: u64,
    rivers_json: String,
    realism: Option<f32>,
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
    let cmap = crate::sim::cultures::compute_culture_map(&buf, seed);
    crate::sim::cultures::store_and_activate(&conn, cmap).map_err(|e| e.to_string())?;
    let habitability = settlements::compute_habitability(&buf, &river_data, &lakes);
    let result = settlements::generate_settlements(
        &buf, &habitability, &river_data, seed, realism.unwrap_or(0.55));

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
