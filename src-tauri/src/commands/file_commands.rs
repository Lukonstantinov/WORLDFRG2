use tauri::State;
use crate::db::{WorldDb, metadata, tile_store};
use crate::tile::coords::TILE_SIZE;
use crate::tile::cell::TileData;
use crate::render::tile_image::render_tile;
use rusqlite::Connection;

/// Save the WORLD file: full SQLite backup, then strip campaign data — the
/// `.worldforge` file carries geography only; campaigns save separately.
#[tauri::command]
pub fn save_world_as(
    path: String,
    db: State<'_, WorldDb>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Use SQLite backup API to copy in-memory DB to file
    let mut dest = Connection::open(&path).map_err(|e| e.to_string())?;
    {
        let backup = rusqlite::backup::Backup::new(&conn, &mut dest).map_err(|e| e.to_string())?;
        backup.run_to_completion(100, std::time::Duration::from_millis(10), None)
            .map_err(|e| e.to_string())?;
    }
    // The backup copies every table; campaign rows don't belong in a world file.
    dest.execute("DELETE FROM campaign", []).map_err(|e| e.to_string())?;
    Ok(())
}

/// What `open_world` hands the frontend: the world meta, the campaign (if the
/// file carried one — legacy single-file saves do, in-memory-migrated), and
/// persisted wizard progress.
#[derive(serde::Serialize)]
pub struct OpenWorldResult {
    pub meta: crate::commands::world_commands::WorldMeta,
    /// True when the file was a pre-split save (its settlements/economy lived
    /// in world metadata) — the frontend offers to split it into two files.
    pub legacy: bool,
    pub campaign_name: Option<String>,
    /// JSON-encoded step-completion maps (empty when never persisted).
    pub world_progress: Option<String>,
    pub campaign_progress: Option<String>,
}

#[tauri::command]
pub fn open_world(
    path: String,
    db: State<'_, WorldDb>,
) -> Result<OpenWorldResult, String> {
    // Copy file DB into the managed in-memory DB
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Attach the source DB and copy all data
    let src_path = path.replace('\'', "''");
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS src;
         DELETE FROM tiles; DELETE FROM metadata; DELETE FROM objects;
         DELETE FROM sim_state; DELETE FROM undo_journal; DELETE FROM campaign;
         INSERT INTO metadata SELECT * FROM src.metadata;
         INSERT INTO tiles SELECT * FROM src.tiles;
         INSERT OR IGNORE INTO objects SELECT * FROM src.objects;
         INSERT OR IGNORE INTO sim_state SELECT * FROM src.sim_state;",
        src_path
    )).map_err(|e| e.to_string())?;
    // Older files have no campaign table; copy it only when present.
    let has_campaign: i64 = conn.query_row(
        "SELECT COUNT(*) FROM src.sqlite_master WHERE type = 'table' AND name = 'campaign'",
        [], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    if has_campaign > 0 {
        conn.execute_batch("INSERT INTO campaign SELECT * FROM src.campaign;")
            .map_err(|e| e.to_string())?;
    }
    conn.execute_batch("DETACH DATABASE src;").map_err(|e| e.to_string())?;

    // Pre-split saves keep settlements/economy in metadata → move them into the
    // campaign table so the app always runs on the split model in memory.
    let legacy = crate::commands::campaign_commands::migrate_legacy_campaign_keys(&conn)?;

    // Read meta from the loaded DB
    let name = metadata::get_meta(&conn, "name")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Untitled".to_string());
    let grid_width: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(360);
    let grid_height: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    let (equator_offset, lat_scale, lat_ratio) =
        crate::commands::world_commands::read_lat_config(&conn);

    Ok(OpenWorldResult {
        meta: crate::commands::world_commands::WorldMeta {
            name,
            grid_width,
            grid_height,
            tile_size: 128,
            equator_offset,
            lat_scale,
            lat_ratio,
            frozen: crate::commands::campaign_commands::is_frozen(&conn),
        },
        legacy,
        campaign_name: metadata::campaign_get(&conn, "name").ok().flatten(),
        world_progress: metadata::get_meta(&conn, "world_progress").ok().flatten(),
        campaign_progress: metadata::campaign_get(&conn, "campaign_progress").ok().flatten(),
    })
}

#[tauri::command]
pub fn export_heightmap(
    path: String,
    db: State<'_, WorldDb>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if grid_w == 0 || grid_h == 0 {
        return Err("No world loaded".into());
    }

    // Ensure a .png extension so the image crate selects the PNG encoder.
    let mut out_path = std::path::PathBuf::from(&path);
    let is_png = out_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("png"))
        .unwrap_or(false);
    if !is_png {
        out_path.set_extension("png");
    }

    // Create 16-bit grayscale image. Sea cells get a small non-zero floor so the
    // coastline is visible (pure black = sea, faint grey = shore) and land uses
    // the upper range.
    let mut img_buf: Vec<u16> = vec![0u16; (grid_w * grid_h) as usize];
    let mut land_cells = 0u64;

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;

    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = tile_store::load_tile(&conn, tx, ty, 0)
                .map_err(|e| e.to_string())?
                .unwrap_or_else(TileData::new_sea);

            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;

            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);

            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let idx = (ly * TILE_SIZE + lx) as usize;
                    let wx = base_x + lx;
                    let wy = base_y + ly;
                    let pixel_idx = (wy * grid_w + wx) as usize;

                    if tile.terrain[idx] == 1 {
                        land_cells += 1;
                        // Map land into the 0.10..1.0 grey range so even low
                        // coastal land is clearly above the black sea.
                        let elev = tile.elevation[idx].clamp(0.0, 1.0);
                        let v = 0.10 + 0.90 * elev;
                        img_buf[pixel_idx] = (v * 65535.0) as u16;
                    }
                    // Sea cells stay 0 (black)
                }
            }
        }
    }

    if land_cells == 0 {
        return Err("No land elevation to export — run elevation generation (step 2) first".into());
    }

    // Write as 16-bit grayscale PNG using the image crate
    let img = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::from_raw(
        grid_w, grid_h, img_buf,
    ).ok_or("Failed to create image buffer")?;

    img.save(&out_path).map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;

    Ok(())
}

/// Export one or more render layers as full-resolution RGBA PNGs.
/// Each layer is stitched from its tiles and written to
/// `<dir>/<base_name>_<layer>.png`. Returns the list of written file paths.
#[tauri::command]
pub fn export_layers(
    dir: String,
    base_name: String,
    layers: Vec<String>,
    db: State<'_, WorldDb>,
) -> Result<Vec<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if grid_w == 0 || grid_h == 0 {
        return Err("No world loaded".into());
    }

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;

    // Load every tile once, reuse across all requested layers.
    let mut loaded: Vec<(i32, i32, TileData)> = Vec::new();
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = tile_store::load_tile(&conn, tx, ty, 0)
                .map_err(|e| e.to_string())?
                .unwrap_or_else(TileData::new_sea);
            loaded.push((tx, ty, tile));
        }
    }

    let safe_base = sanitize(&base_name);
    let dir_path = std::path::PathBuf::from(&dir);
    let mut written = Vec::new();

    for layer in &layers {
        let mut img_buf: Vec<u8> = vec![0u8; (grid_w * grid_h * 4) as usize];

        for (tx, ty, tile) in &loaded {
            let rgba = render_tile(tile, layer); // TILE_SIZE*TILE_SIZE*4
            let base_x = *tx as u32 * TILE_SIZE;
            let base_y = *ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let src = ((ly * TILE_SIZE + lx) * 4) as usize;
                    let dst = (((base_y + ly) * grid_w + (base_x + lx)) * 4) as usize;
                    img_buf[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
                }
            }
        }

        let img = image::RgbaImage::from_raw(grid_w, grid_h, img_buf)
            .ok_or("Failed to create image buffer")?;
        let out_path = dir_path.join(format!("{}_{}.png", safe_base, sanitize(layer)));
        img.save(&out_path)
            .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;
        written.push(out_path.to_string_lossy().to_string());
    }

    Ok(written)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
