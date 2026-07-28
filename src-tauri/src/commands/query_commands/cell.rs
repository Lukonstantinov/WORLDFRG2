//! cell commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


#[tauri::command]
pub fn get_cell_info(
    wx: u32,
    wy: u32,
    db: State<'_, WorldDb>,
) -> Result<CellInfo, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);

    let tc = TileCoord::from_world(wx, wy);
    let tile = tile_store::load_tile(&conn, tc.tx, tc.ty, 0)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(TileData::new_sea);

    let (lx, ly) = TileCoord::local(wx, wy);
    let idx = (ly * TILE_SIZE + lx) as usize;

    let koppen = tile.koppen[idx];
    let elevation = tile.elevation[idx];
    let is_land = tile.terrain[idx] == 1;

    // Prefer the classified biome column (phase 6b). Fall back to the old
    // Köppen-derived name only for sea cells and worlds saved before that phase.
    let biome_code = tile.biome[idx];
    let biome = if is_land && biome_code != 0 {
        crate::sim::biome::biome_name(biome_code).to_string()
    } else {
        koppen_to_biome(koppen, elevation, is_land)
    };
    let biome_group = if is_land && biome_code != 0 {
        crate::sim::biome::biome_group(biome_code).to_string()
    } else {
        String::new()
    };

    Ok(CellInfo {
        wx,
        wy,
        grid_width: grid_w,
        grid_height: grid_h,
        terrain: if is_land { "land".into() } else { "sea".into() },
        elevation,
        sea_depth: tile.sea_depth[idx],
        temperature: tile.temperature[idx],
        precipitation: tile.precipitation[idx],
        koppen,
        biome,
        biome_code,
        biome_group,
        soil_type: tile.soil_type[idx],
        fertility: tile.fertility[idx],
        fishery: tile.fishery[idx],
        plate_index: tile.plate_index[idx],
        is_volcanic: tile.is_volcanic[idx] != 0,
        is_shelf: tile.is_shelf[idx] != 0,
        wind_vx: tile.wind_vx[idx],
        wind_vy: tile.wind_vy[idx],
        current_vx: tile.current_vx[idx],
        current_vy: tile.current_vy[idx],
        current_type: tile.current_type[idx],
        distance_to_ocean: tile.distance_to_ocean[idx],
        salinity: SAL_MIN_PSU + (tile.salinity[idx] as f32 / 255.0) * (SAL_MAX_PSU - SAL_MIN_PSU),
        shark_risk: tile.shark_risk[idx] as f32 / 255.0,
        shipworm_risk: tile.shipworm_risk[idx] as f32 / 255.0,
        storm_risk: tile.storm_base[idx] as f32 / 255.0,
        reef_risk: tile.reef_risk[idx] as f32 / 255.0,
        disease_risk: tile.disease_risk[idx] as f32 / 255.0,
        goods: {
            let specs = crate::commands::goods_commands::load_world_goods(&conn);
            (0..tile.goods.len())
                .filter_map(|g| {
                    let a = tile.goods[g][idx];
                    if a == 0 { return None; }
                    let name = specs.get(g).map(|s| s.id.clone())
                        .or_else(|| GOOD_NAMES.get(g).map(|s| s.to_string()))
                        .unwrap_or_else(|| format!("good_{g}"));
                    Some(GoodAmount { name, amount: a })
                })
                .collect()
        },
    })
}
