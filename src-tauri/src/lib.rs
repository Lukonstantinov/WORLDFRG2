pub mod db;
pub mod tile;
pub mod render;
pub mod paint;
pub mod history;
pub mod commands;
pub mod sim;
pub mod import;

use commands::{world_commands, tile_commands, paint_commands, query_commands, template_commands, sim_commands, file_commands, goods_commands};
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
            query_commands::compute_reef_zones,
            query_commands::compute_good_regions,
            query_commands::compute_overlays,
            query_commands::compute_trade_matrix,
            query_commands::compute_political,
            query_commands::compute_economy,
            query_commands::get_economy,
            query_commands::get_elevation_distribution,
            goods_commands::default_goods,
            goods_commands::get_goods_spec,
            goods_commands::set_goods_spec,
            goods_commands::get_goods_library,
            goods_commands::save_goods_library,
            template_commands::load_image_template,
            sim_commands::sim_generate_plates,
            sim_commands::sim_invert_terrain,
            sim_commands::sim_generate_terrain,
            sim_commands::sim_generate_terrain_from_template,
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
            file_commands::open_world,
            file_commands::export_heightmap,
            file_commands::export_layers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
