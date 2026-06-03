use std::collections::HashMap;
use rusqlite::Connection;
use crate::tile::coords::{TileCoord, TILE_SIZE};
use crate::tile::cell::TileData;
use crate::db::tile_store;
use crate::history::undo;

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
pub enum PaintValue {
    #[serde(rename = "terrain")]
    Terrain { value: u8 }, // 0=sea, 1=land
    #[serde(rename = "elevation")]
    Elevation { value: f32 },
    #[serde(rename = "shelf")]
    Shelf { value: u8 }, // 0=remove, 1=set shelf
    #[serde(rename = "volcanic")]
    Volcanic { value: u8 }, // 0=remove, 1=set volcanic
}

/// Apply a paint stroke to the database, returns list of modified tile coords
pub fn apply_stroke(
    conn: &Connection,
    cells: &[(u32, u32)],
    paint_value: &PaintValue,
) -> Result<Vec<(i32, i32)>, String> {
    // Group cells by tile
    let mut tile_groups: HashMap<(i32, i32), Vec<(u32, u32)>> = HashMap::new();
    for &(wx, wy) in cells {
        let tc = TileCoord::from_world(wx, wy);
        tile_groups
            .entry((tc.tx, tc.ty))
            .or_default()
            .push((wx, wy));
    }

    let modified: Vec<(i32, i32)> = tile_groups.keys().cloned().collect();

    // Save old tile states for undo BEFORE modifying
    let mut old_states: Vec<(i32, i32, Vec<u8>)> = Vec::new();
    for &(tx, ty) in &modified {
        let old_tile = tile_store::load_tile(conn, tx, ty, 0)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(TileData::new_sea);
        old_states.push((tx, ty, old_tile.compress()));
    }

    let label = match paint_value {
        PaintValue::Terrain { value } => {
            if *value == 1 { "Paint land" } else { "Paint sea" }
        }
        PaintValue::Elevation { .. } => "Paint elevation",
        PaintValue::Shelf { value } => {
            if *value == 1 { "Paint shelf" } else { "Remove shelf" }
        }
        PaintValue::Volcanic { value } => {
            if *value == 1 { "Place volcano" } else { "Remove volcano" }
        }
    };
    undo::push_undo(conn, label, &old_states)?;

    for (&(tx, ty), tile_cells) in &tile_groups {
        // Load tile
        let mut tile = tile_store::load_tile(conn, tx, ty, 0)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(TileData::new_sea);

        // Apply changes
        for &(wx, wy) in tile_cells {
            let (lx, ly) = TileCoord::local(wx, wy);
            let idx = (ly * TILE_SIZE + lx) as usize;

            match paint_value {
                PaintValue::Terrain { value } => {
                    tile.terrain[idx] = *value;
                    if *value == 0 {
                        tile.elevation[idx] = 0.0;
                    }
                }
                PaintValue::Elevation { value } => {
                    if tile.terrain[idx] == 1 {
                        tile.elevation[idx] = *value;
                    }
                }
                PaintValue::Shelf { value } => {
                    tile.is_shelf[idx] = *value;
                }
                PaintValue::Volcanic { value } => {
                    tile.is_volcanic[idx] = *value;
                }
            }
        }

        // Save tile back
        tile_store::save_tile(conn, tx, ty, 0, &tile).map_err(|e| e.to_string())?;
    }

    Ok(modified)
}
