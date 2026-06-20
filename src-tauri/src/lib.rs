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
            tile_commands::get_tiles,
            tile_commands::get_tiles_packed,
            tile_commands::get_tile_range,
            paint_commands::paint_stroke,
            paint_commands::undo,
            paint_commands::redo,
            query_commands::get_cell_info,
            query_commands::get_overlay_vectors,
            query_commands::get_current_streamlines,
            query_commands::compute_trade_routes,
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
            campaign_commands::campaign_advance,
            campaign_commands::campaign_persist,
            campaign_commands::campaign_get_state,
            campaign_commands::campaign_get_journal,
            campaign_commands::campaign_get_hub,
            campaign_commands::campaign_get_colony,
            campaign_commands::campaign_get_world_economy,
            campaign_commands::campaign_get_houses,
            campaign_commands::campaign_get_speculation,
            campaign_commands::campaign_get_poleis,
            campaign_commands::campaign_trade_flows,
            campaign_commands::campaign_get_currencies,
            campaign_commands::campaign_coin_usage,
            campaign_commands::campaign_get_banks,
            campaign_commands::campaign_get_crashes,
            campaign_commands::campaign_get_wars,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
