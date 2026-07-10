pub mod db;
pub mod tile;
pub mod render;
pub mod paint;
pub mod history;
pub mod commands;
pub mod sim;
pub mod import;

use commands::{world_commands, tile_commands, paint_commands, query_commands, template_commands, sim_commands, file_commands, goods_commands, campaign_commands, import_commands};
use db::WorldDb;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    // Create an in-memory database for the initial state.
    // When the user creates or opens a world, this gets populated.
    let world_db = WorldDb::in_memory().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(world_db)
        .invoke_handler(tauri::generate_handler![
            world_commands::new_world,
            world_commands::get_world_meta,
            world_commands::set_latitude_config,
            world_commands::set_culture_count,
            world_commands::get_culture_count,
            tile_commands::get_tiles,
            tile_commands::get_tiles_packed,
            tile_commands::render_world_crop,
            tile_commands::get_tile_range,
            paint_commands::paint_stroke,
            paint_commands::undo,
            paint_commands::redo,
            query_commands::get_cell_info,
            query_commands::get_overlay_vectors,
            query_commands::get_current_streamlines,
            query_commands::compute_trade_routes,
            query_commands::compute_itinerary,
            query_commands::get_river_systems,
            query_commands::get_lake_systems,
            query_commands::compute_fishery_banks,
            query_commands::compute_shark_zones,
            query_commands::compute_shipworm_zones,
            query_commands::compute_storm_zones,
            query_commands::compute_monsoon_zones,
            query_commands::compute_reef_zones,
            query_commands::compute_good_regions,
            query_commands::compute_culture_regions,
            query_commands::compute_overlays,
            query_commands::compute_trade_matrix,
            query_commands::campaign_get_trade_flow,
            query_commands::compute_political,
            query_commands::compute_economy,
            query_commands::get_economy,
            query_commands::persist_overlays,
            query_commands::get_overlays,
            query_commands::compute_settlement_development,
            query_commands::export_trade_data,
            query_commands::get_elevation_distribution,
            goods_commands::default_goods,
            goods_commands::get_goods_spec,
            goods_commands::set_goods_spec,
            goods_commands::preview_good_score,
            goods_commands::get_goods_library,
            goods_commands::save_goods_library,
            template_commands::load_image_template,
            sim_commands::sim_generate_plates,
            sim_commands::sim_invert_terrain,
            sim_commands::sim_generate_terrain,
            sim_commands::sim_generate_terrain_from_template,
            sim_commands::sim_generate_terrain_ridged,
            sim_commands::sim_run_all_from_terrain,
            sim_commands::sim_ocean_atmosphere,
            sim_commands::sim_classify_climate,
            sim_commands::sim_rivers_hydrology,
            sim_commands::sim_soil_fertility,
            sim_commands::sim_generate_shelves,
            sim_commands::sim_scale_elevation,
            sim_commands::sim_generate_settlements,
            sim_commands::sim_biological,
            sim_commands::sim_refresh_hydrology_biology,
            sim_commands::sim_generate_toponyms,
            sim_commands::save_toponyms,
            sim_commands::get_toponyms,
            sim_commands::sim_run_all,
            file_commands::save_world_as,
            campaign_commands::finalize_world,
            campaign_commands::unfreeze_world,
            campaign_commands::new_campaign,
            campaign_commands::save_campaign_as,
            campaign_commands::open_campaign,
            campaign_commands::set_progress,
            campaign_commands::set_appearance,
            campaign_commands::get_appearance,
            campaign_commands::campaign_start_sim,
            campaign_commands::campaign_new_game,
            campaign_commands::campaign_advance,
            campaign_commands::campaign_persist,
            campaign_commands::campaign_get_state,
            campaign_commands::campaign_get_journal,
            campaign_commands::campaign_get_trade_basins,
            campaign_commands::campaign_get_good_heat,
            campaign_commands::campaign_get_era_frame,
            campaign_commands::campaign_get_cultures,
            campaign_commands::campaign_get_culture_presence,
            campaign_commands::campaign_culture_hubs,
            campaign_commands::campaign_get_hub,
            campaign_commands::campaign_get_colony,
            campaign_commands::campaign_get_satellite,
            campaign_commands::campaign_get_migration_routes,
            campaign_commands::campaign_get_provisioning,
            campaign_commands::campaign_get_colonies,
            campaign_commands::campaign_colony_gates,
            campaign_commands::campaign_get_world_economy,
            campaign_commands::campaign_city_price_index,
            campaign_commands::campaign_get_houses,
            campaign_commands::campaign_get_inequality,
            campaign_commands::campaign_get_pops,
            campaign_commands::campaign_get_speculation,
            campaign_commands::campaign_get_poleis,
            campaign_commands::campaign_trade_flows,
            campaign_commands::campaign_get_currencies,
            campaign_commands::campaign_get_mints,
            campaign_commands::campaign_monetary_chronicle,
            campaign_commands::campaign_coin_usage,
            campaign_commands::campaign_get_banks,
            campaign_commands::campaign_get_crashes,
            campaign_commands::campaign_get_wars,
            campaign_commands::campaign_get_epidemics,
            campaign_commands::campaign_get_guilds,
            campaign_commands::campaign_get_figures,
            campaign_commands::campaign_get_landmarks,
            campaign_commands::campaign_get_dynasties,
            campaign_commands::campaign_get_goods,
            campaign_commands::campaign_get_schematics,
            campaign_commands::campaign_house_ledger,
            campaign_commands::campaign_get_house_history,
            campaign_commands::campaign_merchant_routes,
            campaign_commands::campaign_futures_lanes,
            campaign_commands::campaign_warehouses,
            campaign_commands::campaign_city_ranking,
            campaign_commands::campaign_diagnostics,
            import_commands::import_world_layers,
            file_commands::open_world,
            file_commands::export_heightmap,
            file_commands::export_layers,
        ])
        .setup(|app| {
            // Capture panic location + message to a file. The crash-guard in
            // `campaign_advance` turns a panicking tick into a recoverable error,
            // but the dev console swallows stderr and a release build has none —
            // so the only durable record of WHERE a tick blew up is this log.
            if let Ok(dir) = app.path().app_log_dir() {
                let _ = std::fs::create_dir_all(&dir);
                let log_path = dir.join("panic.log");
                let default = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    let loc = info
                        .location()
                        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                        .unwrap_or_else(|| "<unknown location>".into());
                    let msg = info
                        .payload()
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| info.payload().downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "<non-string panic payload>".into());
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path)
                    {
                        use std::io::Write;
                        let _ = writeln!(f, "[unix {ts}] panic at {loc}: {msg}");
                    }
                    default(info);
                }));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
