//! overlays commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


#[tauri::command]
pub fn get_overlay_vectors(
    db: State<'_, WorldDb>,
) -> Result<OverlayVectors, String> {
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
        return Ok(OverlayVectors { wind: vec![], currents: vec![], current_step: 1 });
    }
    let world = db.cached_tiles_with_conn(&conn)?;

    // Sparse sampling so currents read as a few large arrows / clear gyres
    // rather than a dense field of tiny ones. ~70 arrows across the map width.
    let step = (grid_w / 70).clamp(10, 50);
    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;

    let mut wind = Vec::new();
    let mut currents = Vec::new();

    // Load tiles and sample
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);

            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;

            // Sample on a GLOBAL grid aligned to `step` rather than restarting at
            // each tile's local origin. `step` does not divide TILE_SIZE (128),
            // so iterating local 0,step,2step… made the sample positions bunch up
            // at every tile boundary — the arrow lattice showed a visible
            // "shift"/seam line every 128 cells. Offsetting to the next global
            // multiple of `step` keeps arrows evenly spaced across tile seams.
            let start_lx = (step - base_x % step) % step;
            let start_ly = (step - base_y % step) % step;

            for ly in (start_ly..TILE_SIZE).step_by(step as usize) {
                for lx in (start_lx..TILE_SIZE).step_by(step as usize) {
                    let wx = base_x + lx;
                    let wy = base_y + ly;
                    if wx >= grid_w || wy >= grid_h { continue; }

                    let idx = (ly * TILE_SIZE + lx) as usize;

                    let wvx = tile.wind_vx[idx];
                    let wvy = tile.wind_vy[idx];
                    if wvx != 0.0 || wvy != 0.0 {
                        wind.push(VectorSample { x: wx, y: wy, vx: wvx, vy: wvy, vec_type: 0 });
                    }

                    let cvx = tile.current_vx[idx];
                    let cvy = tile.current_vy[idx];
                    // Skip dead/negligible cells so arrows only mark real flow.
                    if cvx * cvx + cvy * cvy > 0.0025 {
                        currents.push(VectorSample {
                            x: wx, y: wy, vx: cvx, vy: cvy,
                            vec_type: tile.current_type[idx],
                        });
                    }
                }
            }
        }
    }

    Ok(OverlayVectors { wind, currents, current_step: step })
}


/// Trace ocean currents into a small number of long, continuous polylines
/// (instead of a dense field of per-cell arrows). We integrate the current
/// vector field forward and backward from spaced seed cells, joining each
/// current into a single sweeping arrow. Three families are seeded so the map
/// shows warm boundary currents (red), cold currents / ACC (blue), and the
/// strong neutral flows — equatorial currents, counter-currents and gyre
/// limbs (grey). Lines stop at the coastline rather than running onto land.
#[tauri::command]
pub fn get_current_streamlines(
    db: State<'_, WorldDb>,
) -> Result<Vec<Streamline>, String> {
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
        return Ok(vec![]);
    }
    let world = db.cached_tiles_with_conn(&conn)?;

    let w = grid_w as i32;
    let h = grid_h as i32;
    let n = (grid_w * grid_h) as usize;

    // Load the full-resolution current field into flat arrays. `is_shelf` is
    // loaded too so the tracer can stop a streamline at the shelf break — the
    // shallow apron carries a real velocity for the physics, but boundary
    // currents are DRAWN running along the continental slope, not over it (the
    // same choice `render_currents` makes with `mag = 0` on shelf cells).
    let mut vx = vec![0.0f32; n];
    let mut vy = vec![0.0f32; n];
    let mut ctype = vec![0u8; n];
    let mut terrain = vec![0u8; n];
    let mut shelf = vec![0u8; n];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let gi = ((base_y + ly) * grid_w + (base_x + lx)) as usize;
                    vx[gi] = tile.current_vx[ti];
                    vy[gi] = tile.current_vy[ti];
                    ctype[gi] = tile.current_type[ti];
                    terrain[gi] = tile.terrain[ti];
                    if !tile.is_shelf.is_empty() {
                        shelf[gi] = tile.is_shelf[ti];
                    }
                }
            }
        }
    }
    let _ = (w, h); // kept for readability; the tracer takes grid_w/grid_h

    let out = crate::sim::ocean::trace_current_streamlines(
        &vx, &vy, &ctype, &terrain, &shelf, grid_w, grid_h,
    )
    .into_iter()
    .map(|(points, ctype)| Streamline { points, ctype })
    .collect();

    Ok(out)
}


#[tauri::command]
pub fn get_elevation_distribution(
    db: State<'_, WorldDb>,
) -> Result<Vec<ElevationBand>, String> {
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
        return Ok(vec![]);
    }
    let world = db.cached_tiles_with_conn(&conn)?;

    let band_labels = [
        "0-1000m", "1000-2000m", "2000-3000m", "3000-4000m", "4000-5000m",
        "5000-6000m", "6000-7000m", "7000-8000m", "8000m+",
    ];
    let mut counts = [0u32; 9];
    let mut total_land = 0u32;

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;

    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);

            let max_lx = TILE_SIZE.min(grid_w - tx as u32 * TILE_SIZE);
            let max_ly = TILE_SIZE.min(grid_h - ty as u32 * TILE_SIZE);

            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let idx = (ly * TILE_SIZE + lx) as usize;
                    if tile.terrain[idx] != 1 { continue; }
                    total_land += 1;
                    let meters = tile.elevation[idx] * 8848.0;
                    let band = ((meters / 1000.0) as usize).min(8);
                    counts[band] += 1;
                }
            }
        }
    }

    let result = band_labels.iter().enumerate().map(|(i, label)| {
        ElevationBand {
            label: label.to_string(),
            count: counts[i],
            percentage: if total_land > 0 { counts[i] as f32 / total_land as f32 * 100.0 } else { 0.0 },
        }
    }).collect();

    Ok(result)
}


#[tauri::command]
pub fn compute_overlays(
    db: State<'_, WorldDb>,
) -> Result<OverlaysResult, String> {
    // `State` clones are cheap (just a wrapped reference); each query hits the
    // same shared tile cache (first builds it, the rest reuse).
    Ok(OverlaysResult {
        vectors: get_overlay_vectors(db.clone())?,
        streamlines: get_current_streamlines(db.clone())?,
        fishery_banks: compute_fishery_banks(db.clone())?,
        shark_zones: compute_shark_zones(db.clone())?,
        shipworm_zones: compute_shipworm_zones(db.clone())?,
        reef_zones: compute_reef_zones(db.clone())?,
        good_regions: compute_good_regions(db)?,
    })
}


/// Cluster the richest fishing grounds into a handful of circular "grand banks".
///
/// We down-sample the fishery field to a coarse grid, threshold the productive
/// cells, flood-fill connected clusters, and return each sizeable cluster as a
/// circle (centroid + area-equivalent radius). The frontend draws these as large
/// translucent discs labelled like real fishing banks (Grand Banks, Dogger Bank…).
#[tauri::command]
pub fn compute_fishery_banks(
    db: State<'_, WorldDb>,
) -> Result<Vec<FisheryBank>, String> {
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
        return Ok(vec![]);
    }
    let world = db.cached_tiles_with_conn(&conn)?;

    // Coarse grid (~400 cells across) holding the *max* fishery in each block so
    // a strong but small ground isn't averaged away.
    let f = (grid_w / 400).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let cn = (cw * ch) as usize;
    let mut fish = vec![0.0f32; cn];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    if tile.terrain[ti] != 0 { continue; }
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if tile.fishery[ti] > fish[ci] { fish[ci] = tile.fishery[ti]; }
                }
            }
        }
    }

    // Flood-fill connected clusters of productive coarse cells (X wraps).
    const THRESH: f32 = 0.42;
    let wrap_cx = |x: i32| -> i32 { ((x % cw) + cw) % cw };
    let mut visited = vec![false; cn];
    let mut banks: Vec<FisheryBank> = Vec::new();

    for start in 0..cn {
        if visited[start] || fish[start] < THRESH { continue; }
        let mut stack = vec![start];
        visited[start] = true;
        let mut cells: Vec<usize> = Vec::new();
        while let Some(ci) = stack.pop() {
            cells.push(ci);
            let cx = (ci as i32) % cw;
            let cy = (ci as i32) / cw;
            for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let ny = cy + dy;
                if ny < 0 || ny >= ch { continue; }
                let ni = (ny * cw + wrap_cx(cx + dx)) as usize;
                if !visited[ni] && fish[ni] >= THRESH {
                    visited[ni] = true;
                    stack.push(ni);
                }
            }
        }
        if cells.len() < 4 { continue; } // ignore tiny specks

        // Centroid (handle X wrap by averaging around the first cell) + radius.
        let cx0 = (cells[0] as i32) % cw;
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        let mut ssc = 0.0f32;
        for &ci in &cells {
            let mut cx = (ci as i32) % cw;
            let cy = (ci as i32) / cw;
            // unwrap X relative to the cluster's first cell
            if cx - cx0 > cw / 2 { cx -= cw; } else if cx0 - cx > cw / 2 { cx += cw; }
            sx += cx as f64;
            sy += cy as f64;
            ssc += fish[ci];
        }
        let n = cells.len() as f64;
        let mean_cx = wrap_cx((sx / n).round() as i32);
        let mean_cy = (sy / n).round() as i32;
        let world_x = (mean_cx as u32 * f + f / 2) as f32;
        let world_y = (mean_cy as u32 * f + f / 2) as f32;
        // Area-equivalent radius in world cells, enlarged a touch so the disc
        // reads as a broad "bank" rather than tracing the exact cluster.
        let radius = ((n / std::f64::consts::PI).sqrt() as f32) * f as f32 * 1.35;
        banks.push(FisheryBank {
            x: world_x,
            y: world_y,
            radius,
            score: ssc / cells.len() as f32,
        });
    }

    // Keep the most significant banks (by area × productivity).
    banks.sort_by(|a, b| {
        let av = a.radius * a.radius * a.score;
        let bv = b.radius * b.radius * b.score;
        bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
    });
    banks.truncate(24);

    Ok(banks)
}


/// Cluster shark-infested water into circular danger zones for the overlay.
#[tauri::command]
pub fn compute_shark_zones(db: State<'_, WorldDb>) -> Result<Vec<SharkZone>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;

    // Finer coarse grid than the circle version so the marked area follows the
    // actual physics-driven shark distribution shape.
    let f = (grid_w / 300).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut risk = vec![0.0f32; (cw * ch) as usize];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let v = tile.shark_risk[ti] as f32 / 255.0;
                    if v <= 0.0 { continue; }
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if v > risk[ci] { risk[ci] = v; }
                }
            }
        }
    }

    // Only the highest-probability water reads as a "shark zone" — a high
    // threshold + a higher min-cluster size so sharks aren't tagged everywhere.
    let mut zones: Vec<SharkZone> = cluster_cells(&risk, cw, ch, f, 0.62, 3)
        .into_iter()
        .map(|c| SharkZone { cells: c.cells, cell_size: f as f32, x: c.cx, y: c.cy, score: c.score })
        .collect();
    // Largest, most dangerous areas first.
    zones.sort_by(|a, b| {
        (b.cells.len() as f32 * b.score)
            .partial_cmp(&(a.cells.len() as f32 * a.score))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    zones.truncate(14);
    Ok(zones)
}


/// Cluster shipworm hull-hazard water into the highest-risk danger zones (same
/// shape as shark zones — a Biological hazard sublayer for wooden shipping).
#[tauri::command]
pub fn compute_shipworm_zones(db: State<'_, WorldDb>) -> Result<Vec<SharkZone>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;

    let f = (grid_w / 300).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut risk = vec![0.0f32; (cw * ch) as usize];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let v = tile.shipworm_risk[ti] as f32 / 255.0;
                    if v <= 0.0 { continue; }
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if v > risk[ci] { risk[ci] = v; }
                }
            }
        }
    }

    let mut zones: Vec<SharkZone> = cluster_cells(&risk, cw, ch, f, 0.58, 3)
        .into_iter()
        .map(|c| SharkZone { cells: c.cells, cell_size: f as f32, x: c.cx, y: c.cy, score: c.score })
        .collect();
    zones.sort_by(|a, b| {
        (b.cells.len() as f32 * b.score)
            .partial_cmp(&(a.cells.len() as f32 * a.score))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    zones.truncate(14);
    Ok(zones)
}


/// Cluster the highest-risk storm water into danger zones (open-ocean cyclone
/// belts; same overlay shape as shark/shipworm zones). `month <= 0` returns the
/// combined annual extent; `month` in 1..=`months` applies the analytic seasonal
/// phase (`biological::storm_season_phase`) so zones fade out in their calm
/// season and the hemispheres peak ~half a year apart.
#[tauri::command]
pub fn compute_storm_zones(month: i32, months: u32, db: State<'_, WorldDb>) -> Result<Vec<SharkZone>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;
    let months = months.max(1);

    let f = (grid_w / 300).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut risk = vec![0.0f32; (cw * ch) as usize];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let base = tile.storm_base[ti] as f32 / 255.0;
                    if base <= 0.0 { continue; }
                    let wy = base_y + ly;
                    let lat = 90.0 - (wy as f32 / grid_h as f32) * 180.0;
                    let v = base * crate::sim::biological::storm_season_phase(month, months, lat);
                    if v <= 0.0 { continue; }
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if v > risk[ci] { risk[ci] = v; }
                }
            }
        }
    }

    // Real cyclone risk isn't a single smooth band — distinct basins (NW Pacific,
    // N Atlantic, S Indian…) spawn storms semi-independently. Modulate the smooth
    // cyclogenesis field with low-frequency value noise so it breaks into several
    // discrete danger areas instead of one map-spanning blob.
    let period = (cw as f32 / 7.0).max(4.0); // ~7 lumps across the map width
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            if risk[ci] <= 0.0 { continue; }
            let nz = value_noise(cx, cy, cw, period, 0x57014_C9C10_u64);
            let lump = 0.30 + 1.05 * nz; // troughs fall under threshold → gaps
            risk[ci] = (risk[ci] * lump).min(1.0);
        }
    }

    let mut zones: Vec<SharkZone> = cluster_cells(&risk, cw, ch, f, 0.50, 3)
        .into_iter()
        .map(|c| SharkZone { cells: c.cells, cell_size: f as f32, x: c.cx, y: c.cy, score: c.score })
        .collect();
    zones.sort_by(|a, b| {
        (b.cells.len() as f32 * b.score)
            .partial_cmp(&(a.cells.len() as f32 * a.score))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    zones.truncate(30);
    Ok(zones)
}


/// Cluster the LAND areas under the seasonal monsoon into discrete regions — the
/// wet-season flood belt. Derived from the low-level-jet dynamics: heaviest where
/// a decelerating onshore JET converges over the coast (jet exit), gated by
/// onshore warm-sea flow, with the Köppen monsoon climates (Am/Cw*/Dw*) kept as a
/// supporting signal. Shown on the Natural Disasters overlay alongside the cyclone
/// (hurricane) zones. Mirrors `compute_storm_zones`.
#[tauri::command]
pub fn compute_monsoon_zones(db: State<'_, WorldDb>) -> Result<Vec<SharkZone>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;

    let f = (grid_w / 300).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut risk = vec![0.0f32; (cw * ch) as usize];

    // Monsoon intensity is now derived from the low-level-jet dynamics — the
    // heavy wet-season rains land where a decelerating onshore JET converges over
    // the coast (jet EXIT), so the overlay is built from: (a) jet-exit convergence
    // gated by (b) onshore warm-sea flow, with the Köppen monsoon climates
    // (Am/Cw*/Dw*) kept as a supporting signal so established monsoon regions still
    // register even on worlds without a strong resolved jet.
    const MONSOON_JET_MIN: f32 = 8.0;    // wind speed (m/s) below which a cell isn't a jet
    const MONSOON_ACCEL_SCALE: f32 = 6.0; // along-flow Δspeed that saturates the exit signal
    let gw = grid_w as i32;
    let gh = grid_h as i32;
    let km_per_cell = 40075.0f32 / grid_w as f32;
    let onshore_range = ((800.0 / km_per_cell).round() as i32).max(10);

    // Sample (terrain, current_type, wind_speed) at any global cell (x wraps).
    let at = |wx: i32, wy: i32| -> Option<(u8, u8, f32)> {
        if wy < 0 || wy >= gh { return None; }
        let x = (((wx % gw) + gw) % gw) as u32;
        let y = wy as u32;
        let t = world.tile((x / TILE_SIZE) as i32, (y / TILE_SIZE) as i32);
        let li = ((y % TILE_SIZE) * TILE_SIZE + (x % TILE_SIZE)) as usize;
        Some((t.terrain[li], t.current_type[li], t.wind_speed[li]))
    };

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    if tile.terrain[ti] != 1 { continue; }
                    let wx = (base_x + lx) as i32;
                    let wy = (base_y + ly) as i32;
                    let lat = world.latitude(wy as u32);

                    // Köppen support (established monsoon climates).
                    let kopp = monsoon_climate_intensity(tile.koppen[ti]);

                    // Jet-exit convergence: the flow DECELERATES downwind (s_up > s_down).
                    let s = tile.wind_speed[ti];
                    let wvx = tile.wind_vx[ti];
                    let wvy = tile.wind_vy[ti];
                    let wl = (wvx * wvx + wvy * wvy).sqrt();
                    let mut jet_exit = 0.0f32;
                    if s >= MONSOON_JET_MIN && wl > 0.01 {
                        let dx = wvx / wl;
                        let dy = wvy / wl;
                        let s_down = at((wx as f32 + dx * 2.0).round() as i32,
                                        (wy as f32 + dy * 2.0).round() as i32)
                            .map(|c| c.2).unwrap_or(0.0);
                        let s_up = at((wx as f32 - dx * 2.0).round() as i32,
                                      (wy as f32 - dy * 2.0).round() as i32)
                            .map(|c| c.2).unwrap_or(s);
                        let decel = ((s_up - s_down) / MONSOON_ACCEL_SCALE).clamp(0.0, 1.0);
                        let strength = ((s - MONSOON_JET_MIN) / 12.0).clamp(0.0, 1.0);
                        jet_exit = decel * strength;
                    }

                    // Onshore warm-sea flow on the equatorward / eastern fetch.
                    let eqdir = if lat >= 0.0 { 1i32 } else { -1 };
                    let mut onshore = 0.0f32;
                    for &(dx, dy) in &[(0i32, eqdir), (1, eqdir), (1, 0)] {
                        for step in 1..=onshore_range {
                            match at(wx + dx * step, wy + dy * step) {
                                None => break,
                                Some((terr, ct, _)) => {
                                    if terr == 0 {
                                        let warm = if ct == 2 { 0.3 } else { 1.0 };
                                        onshore = onshore
                                            .max((1.0 - step as f32 / onshore_range as f32) * warm);
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    let v = (0.55 * kopp + 0.9 * jet_exit * onshore + 0.25 * onshore).min(1.0);
                    if v <= 0.0 { continue; }
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if v > risk[ci] { risk[ci] = v; }
                }
            }
        }
    }

    let mut zones: Vec<SharkZone> = cluster_cells(&risk, cw, ch, f, 0.45, 4)
        .into_iter()
        .map(|c| SharkZone { cells: c.cells, cell_size: f as f32, x: c.cx, y: c.cy, score: c.score })
        .collect();
    zones.sort_by(|a, b| {
        (b.cells.len() as f32 * b.score)
            .partial_cmp(&(a.cells.len() as f32 * a.score))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    zones.truncate(24);
    Ok(zones)
}


/// Cluster the highest-risk reef/shoal wreck water into danger zones.
#[tauri::command]
pub fn compute_reef_zones(db: State<'_, WorldDb>) -> Result<Vec<SharkZone>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;

    let f = (grid_w / 300).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut risk = vec![0.0f32; (cw * ch) as usize];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let v = tile.reef_risk[ti] as f32 / 255.0;
                    if v <= 0.0 { continue; }
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if v > risk[ci] { risk[ci] = v; }
                }
            }
        }
    }

    let mut zones: Vec<SharkZone> = cluster_cells(&risk, cw, ch, f, 0.55, 3)
        .into_iter()
        .map(|c| SharkZone { cells: c.cells, cell_size: f as f32, x: c.cx, y: c.cy, score: c.score })
        .collect();
    zones.sort_by(|a, b| {
        (b.cells.len() as f32 * b.score)
            .partial_cmp(&(a.cells.len() as f32 * a.score))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    zones.truncate(14);
    Ok(zones)
}


/// The organic culture territories of the active world, one region per hearth.
#[tauri::command]
pub fn compute_culture_regions(db: State<'_, WorldDb>) -> Result<Vec<CultureRegion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::sim::cultures::ensure_active(&conn);
    let Some(map) = crate::sim::cultures::active() else { return Ok(vec![]); };
    let (cw, ch) = (map.cw, map.ch);
    if cw == 0 || ch == 0 || map.hearths.is_empty() { return Ok(vec![]); }
    let csx = map.world_w as f32 / cw as f32;
    let csy = map.world_h as f32 / ch as f32;
    let mut by_hearth: Vec<Vec<[f32; 2]>> = vec![Vec::new(); map.hearths.len()];
    for cy in 0..ch {
        for cx in 0..cw {
            let id = map.ids[(cy * cw + cx) as usize];
            if (id as usize) < map.hearths.len() {
                by_hearth[id as usize].push([cx as f32 * csx, cy as f32 * csy]);
            }
        }
    }
    let mut out: Vec<CultureRegion> = Vec::new();
    for (i, hh) in map.hearths.iter().enumerate() {
        let cells = std::mem::take(&mut by_hearth[i]);
        if cells.is_empty() { continue; }
        let n = cells.len() as f32;
        let (sx, sy) = cells.iter().fold((0.0f32, 0.0f32), |(ax, ay), c| (ax + c[0], ay + c[1]));
        let kit = crate::sim::cultures::KITS[(hh.kit as usize).min(crate::sim::cultures::KITS.len() - 1)].name;
        out.push(CultureRegion {
            cell_size: csx.max(csy),
            x: sx / n + csx * 0.5,
            y: sy / n + csy * 0.5,
            color: hh.color,
            label: hh.people.clone(),
            culture: kit.to_string(),
            cells,
        });
    }
    Ok(out)
}


/// Mark every trade-good belt as a filled AREA (the actual physics-driven cells,
/// coarse-downsampled) plus a label centroid for the emoji. Each good is already
/// localized to one homeland region in `compute_trade_goods`.
#[tauri::command]
pub fn compute_good_regions(db: State<'_, WorldDb>) -> Result<Vec<GoodRegion>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }

    // Finer coarse grid (smaller marked cells) so good belts read as detailed
    // patches rather than coarse blocks.
    let f = (grid_w / 450).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let cn = (cw * ch) as usize;
    let world = db.cached_tiles_with_conn(&conn)?;
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let mut grids: Vec<Vec<f32>> = vec![vec![0.0f32; cn]; gc];
    // Coarse climate fields for per-cell subtype classification (grain / paper).
    let mut c_temp = vec![0.0f32; cn];
    let mut c_precip = vec![0.0f32; cn];
    let mut c_elev = vec![0.0f32; cn];
    let mut c_hab = vec![0.0f32; cn];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    for ty in 0..tiles_y as i32 {
        for tx in 0..tiles_x as i32 {
            let tile = world.tile(tx, ty);
            let base_x = tx as u32 * TILE_SIZE;
            let base_y = ty as u32 * TILE_SIZE;
            let max_lx = TILE_SIZE.min(grid_w - base_x);
            let max_ly = TILE_SIZE.min(grid_h - base_y);
            for ly in 0..max_ly {
                for lx in 0..max_lx {
                    let ti = (ly * TILE_SIZE + lx) as usize;
                    let cx = ((base_x + lx) / f) as i32;
                    let cy = ((base_y + ly) / f) as i32;
                    let ci = (cy * cw + cx) as usize;
                    for g in 0..gc.min(tile.goods.len()) {
                        let v = tile.goods[g][ti] as f32 / 255.0;
                        if v > grids[g][ci] { grids[g][ci] = v; }
                    }
                    if tile.terrain[ti] == 1 {
                        c_temp[ci] = tile.temperature[ti];
                        c_precip[ci] = tile.precipitation[ti];
                        c_elev[ci] = tile.elevation[ti];
                        c_hab[ci] = tile.habitability[ti];
                    }
                }
            }
        }
    }

    use crate::sim::biological::GEM_STONES;
    use crate::sim::goods_spec::Distribution;
    // id → column index, for resolving a manufactured good's recipe inputs to belts.
    let id_index: std::collections::HashMap<&str, usize> =
        specs.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
    let mut out: Vec<GoodRegion> = Vec::new();
    for g in 0..gc {
        let spec = match specs.get(g) {
            Some(s) if s.enabled => s,
            _ => continue, // disabled goods produce nothing to map
        };
        // Manufactured goods are made in CITIES from imported raws — they have no
        // natural homeland, so they get NO map overlay (drawing the input supply zone
        // misleadingly implied a belt). The frontend hides their toggle to match.
        if matches!(spec.distribution, Distribution::Manufactured) {
            continue;
        }
        // Deposit goods are scattered (keep every deposit). Global goods blanket
        // the world (many patches). Local goods are one homeland (top few).
        // Manufactured goods have NO belt of their own — they're made in cities from
        // imported raws; their overlay is the SUPPLY ZONE where all recipe inputs
        // co-occur (the natural manufacturing hinterland).
        let (max_keep, min_cells) = match spec.distribution {
            Distribution::Deposits => (32usize, 1usize),
            Distribution::Global => (14, 1),
            Distribution::Local => (4, 1),
            Distribution::Manufactured => (8, 1),
        };
        // For a manufactured good, synthesize a candidate grid = the co-occurrence
        // (min) of its input belts; inputs that are themselves manufactured (or
        // unknown) are treated as ubiquitous (1.0). Empty recipe → nothing to map.
        let synth: Vec<f32>;
        let source_grid: &Vec<f32> = if matches!(spec.distribution, Distribution::Manufactured) {
            if spec.inputs.is_empty() {
                continue;
            }
            let in_cols: Vec<Option<usize>> =
                spec.inputs.iter().map(|inp| id_index.get(inp.good.as_str()).copied()).collect();
            synth = (0..cn)
                .map(|ci| {
                    in_cols.iter().fold(1.0f32, |acc, col| {
                        let v = match col {
                            Some(c) if matches!(
                                specs[*c].distribution,
                                Distribution::Global | Distribution::Local | Distribution::Deposits
                            ) => grids[*c][ci],
                            _ => 1.0, // manufactured/unknown input assumed available
                        };
                        acc.min(v)
                    })
                })
                .collect();
            &synth
        } else {
            &grids[g]
        };
        // Multi-type goods: wheat splits into grain species, paper into its three
        // sources — classified per cell from the coarse climate fields.
        let subtype_kind = match spec.id.as_str() { "wheat" => 1u8, "paper" => 2u8, _ => 0u8 };
        // Deposits (§8.16) carry `workable_intensity = grade · depth_workability`,
        // and `depth_workability` alone is 0.35 (deep) or 0.15 (flooded) — so a
        // deep/flooded working's belt byte routinely sits well under the 0.30 cut
        // tuned for climate-belt goods that span close to 1.0. That silently broke
        // this function's own "Deposit goods are scattered (keep every deposit)"
        // comment above: a real, placed working just never clustered into a region,
        // so "not all mining goods appear on the map" even though the working was
        // genuinely placed and correctly counted by `campaign_province_goods`/
        // `compute_economy`. Use a near-zero floor for Deposits so a working's mere
        // presence — not its depth-discounted intensity — decides whether it draws.
        let thresh = if matches!(spec.distribution, Distribution::Deposits) { 0.02 } else { 0.30 };
        let mut regions = cluster_cells(source_grid, cw, ch, f, thresh, min_cells);
        regions.sort_by(|a, b| b.cells.len().cmp(&a.cells.len()));
        regions.truncate(max_keep);
        for c in regions {
            // Name each gemstone deposit for a specific stone (gemstones only).
            let sublabel = if spec.id == "gemstones" {
                let h = (c.cx as i64).wrapping_mul(73856093) ^ (c.cy as i64).wrapping_mul(19349663);
                GEM_STONES[(h.unsigned_abs() as usize) % GEM_STONES.len()].to_string()
            } else {
                String::new()
            };
            let values: Vec<u8> = c.values.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8).collect();
            let subtypes: Vec<u8> = if subtype_kind == 0 {
                Vec::new()
            } else {
                c.cells.iter().map(|&[wx, wy]| {
                    let cx = (wx as u32 / f).min(cw as u32 - 1) as i32;
                    let cy = (wy as u32 / f).min(ch as u32 - 1) as i32;
                    let ci = (cy * cw + cx) as usize;
                    if subtype_kind == 1 {
                        grain_subtype(c_temp[ci], c_precip[ci], c_elev[ci])
                    } else {
                        paper_subtype(c_temp[ci], c_precip[ci], c_elev[ci], c_hab[ci])
                    }
                }).collect()
            };
            out.push(GoodRegion {
                good: spec.id.clone(),
                cells: c.cells,
                values,
                subtypes,
                cell_size: f as f32,
                x: c.cx,
                y: c.cy,
                score: c.score,
                sublabel,
            });
        }
    }
    Ok(out)
}


// ═════════════════════════════════════════════════════════════════════════════════
//  GOODS_LOCALITIES_PLAN.md Slice 5 — THE TWO-LAYER, FULL-RESOLUTION BELT MASK
//  (D3 · D9 · D10)
//
//  `compute_good_regions` above rasterises a belt onto a COARSE grid (`f =
//  grid_w / 450`, ~8×8 world cells at the default size) and the frontend fills each
//  block. That is F4: a block containing one land cell of belt paints all 64 of its
//  cells, so the belt's edge is a staircase that ignores the coastline and spills
//  into the sea. It cannot be fixed by shading — the information simply is not in
//  the payload.
//
//  So the mask below is FULL RESOLUTION, and that alone is the coastline clip: a
//  land good's belt byte is already exactly zero on every sea cell (`envelope_score`
//  gates `Domain::Continental`/`Coastal` on `terrain == 1`) and a marine good's is
//  zero on land. Nothing has to know where the coast is; drawing the belt at its own
//  resolution ends it on the coast by construction. D3's "never snapped to province
//  polygons" is satisfied for the same reason — the belt is a physical fact and this
//  draws the physical fact.
//
//  TWO LAYERS (D9), because they are two different questions and they compress
//  differently:
//
//  * COVERAGE — "can it grow here". A boolean, at FULL resolution, run-length
//    encoded. A belt is contiguous, so a row crosses its boundary a handful of times
//    and the RLE is small even for a world-spanning staple region.
//  * QUALITY — "is it fine here". The belt value itself, on a COARSE grid, because
//    a wash does not need per-cell precision and the full-resolution field is noisy
//    enough that RLE would not compress it. The frontend paints quality only where
//    coverage says so, so the coarse wash still ends exactly on the coastline.
//
//  D10 lives on the frontend's side of this: the value emitted here is the belt's
//  own absolute 0..255, never rescaled against the good's own maximum, and the scale
//  it shades on is served once by `get_render_palettes`.
// ═════════════════════════════════════════════════════════════════════════════════

/// A belt cell counts as COVERED at a hair above zero, not at the 0.30 the region
/// clusterer uses. The two thresholds answer different questions: the clusterer is
/// picking where to put a label, while coverage is the honest extent of the land
/// that can yield the good — which is exactly the reading §5.1 asks to be visible on
/// the map rather than buried in a diagnostic, now that Slice 3 thins the fringe.
pub(crate) const COVERAGE_MIN_U8: u8 = 5;

/// Slice 5 · one good's belt as a full-resolution mask (see the block comment above).
#[derive(Debug, Serialize)]
pub struct GoodBeltMask {
    pub good: String,
    /// Bounding box of the belt, in world cells. The mask covers exactly this box.
    pub x0: u32,
    pub y0: u32,
    pub w: u32,
    pub h: u32,
    /// COVERAGE — full-resolution 0/1 over the box, row-major, run-length encoded as
    /// flat `(value, run)` pairs (the same shape `province_raster_rle` already uses).
    pub coverage_rle: Vec<u32>,
    /// QUALITY — the belt's own absolute value 0..255 on a coarse grid over the same
    /// box (`qw × qh`, block size `coarse`). Never per-good normalised (D10).
    pub quality: Vec<u8>,
    pub qw: u32,
    pub qh: u32,
    /// World cells per quality block.
    pub coarse: u32,
    /// Per-quality-block subtype id (grain species / paper source); empty if the good
    /// has no subtypes.
    pub subtypes: Vec<u8>,
    /// Covered cells, so the frontend can size a medallion / report an extent.
    pub cells: u32,
}

/// Run-length encode a row-major boolean field into flat `(value, run)` pairs.
fn rle_encode_mask(mask: &[u8]) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i < mask.len() {
        let v = mask[i];
        let start = i;
        while i < mask.len() && mask[i] == v { i += 1; }
        out.push(v as u32);
        out.push((i - start) as u32);
    }
    out
}

/// Build one good's mask from its FULL-RESOLUTION belt column.
///
/// Kept as a plain function over a flat column — no `State`, no tiles — so the
/// `dump_good_belt_mask_sheet` proof can drive the real code path rather than a
/// re-implementation of it (the same reason `dump_biome_swatch_sheet` renders
/// through the real `render_tile`, §8.12). Returns `None` for a good the world never
/// placed. `subtype_at` maps a world cell to a subtype id and is only consulted for
/// a multi-type good (grain species / paper source).
pub(crate) fn build_belt_mask(
    good: &str, belt: &[u8], grid_w: u32, grid_h: u32, coarse: u32,
    subtype_at: Option<&dyn Fn(u32, u32) -> u8>,
) -> Option<GoodBeltMask> {
    let mut x0 = grid_w; let mut y0 = grid_h;
    let mut x1 = 0u32;   let mut y1 = 0u32;
    let mut covered = 0u32;
    for (i, &v) in belt.iter().enumerate() {
        if v < COVERAGE_MIN_U8 { continue; }
        let wx = (i as u32) % grid_w;
        let wy = (i as u32) / grid_w;
        if wx < x0 { x0 = wx; }
        if wx > x1 { x1 = wx; }
        if wy < y0 { y0 = wy; }
        if wy > y1 { y1 = wy; }
        covered += 1;
    }
    if covered == 0 { return None; }

    let bw = x1 - x0 + 1;
    let bh = y1 - y0 + 1;
    // Coverage: full-resolution 0/1 over the box.
    let mut cov = vec![0u8; (bw as usize) * (bh as usize)];
    for by in 0..bh {
        let src = ((y0 + by) as usize) * (grid_w as usize) + x0 as usize;
        let dst = (by as usize) * (bw as usize);
        for bx in 0..bw as usize {
            if belt[src + bx] >= COVERAGE_MIN_U8 { cov[dst + bx] = 1; }
        }
    }

    // Quality: the block MAX, so a small but excellent patch is not averaged away
    // (the same choice `compute_fishery_banks` makes for the same reason).
    let qw = (bw + coarse - 1) / coarse;
    let qh = (bh + coarse - 1) / coarse;
    let mut quality = vec![0u8; (qw * qh) as usize];
    // A representative land cell per block for the subtype classification, so a grain
    // block is typed from real climate rather than from whatever cell the scan ended on.
    let mut sub_src = vec![u32::MAX; (qw * qh) as usize];
    for by in 0..bh {
        let wy = y0 + by;
        let qy = by / coarse;
        for bx in 0..bw {
            let v = belt[(wy as usize) * (grid_w as usize) + (x0 + bx) as usize];
            if v == 0 { continue; }
            let qi = (qy * qw + bx / coarse) as usize;
            if v > quality[qi] {
                quality[qi] = v;
                if subtype_at.is_some() { sub_src[qi] = wy * grid_w + (x0 + bx); }
            }
        }
    }
    let subtypes: Vec<u8> = match subtype_at {
        None => Vec::new(),
        Some(f) => sub_src.iter()
            .map(|&idx| if idx == u32::MAX { 0 } else { f(idx % grid_w, idx / grid_w) })
            .collect(),
    };

    Some(GoodBeltMask {
        good: good.to_string(),
        x0, y0, w: bw, h: bh,
        coverage_rle: rle_encode_mask(&cov),
        quality, qw, qh, coarse,
        subtypes,
        cells: covered,
    })
}

/// Slice 5 · the FULL-RESOLUTION belt mask for each named good — the payload that
/// replaces the coarse blocks of `compute_good_regions` for the FILL. Regions are
/// still computed there and still carry the label centroid / medallion / sublabel;
/// this command only answers "which cells, and how good".
///
/// Takes an explicit good list rather than returning every good, because a mask is
/// a real payload and only the toggled goods are ever drawn. `Manufactured` goods
/// are skipped exactly as the region path skips them (they are made in cities and
/// have no belt); `Deposits` goods are NOT skipped — a working's belt byte is a real
/// cell and drawing it at full resolution is strictly better than a block.
#[tauri::command]
pub fn compute_good_belt_masks(
    goods: Vec<String>,
    db: State<'_, WorldDb>,
) -> Result<Vec<GoodBeltMask>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 || goods.is_empty() { return Ok(vec![]); }

    let world = db.cached_tiles_with_conn(&conn)?;
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    use crate::sim::goods_spec::Distribution;

    // The quality grid's block size — the SAME coarse factor the region path uses, so
    // the wash carries exactly the detail it always did while the coverage layer
    // beneath it supplies the edge.
    let coarse = (grid_w / 450).max(1);

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    let n = (grid_w as usize) * (grid_h as usize);

    let mut out: Vec<GoodBeltMask> = Vec::new();
    // One full-resolution scratch buffer, reused across goods (a per-good allocation
    // of the world grid is 6.5 MB at the default size and 26 MB on Large).
    let mut belt = vec![0u8; n];

    for name in &goods {
        let Some((g, spec)) = specs.iter().enumerate().find(|(_, s)| &s.id == name) else { continue };
        if !spec.enabled || matches!(spec.distribution, Distribution::Manufactured) { continue; }

        belt.fill(0);
        for ty in 0..tiles_y as i32 {
            for tx in 0..tiles_x as i32 {
                let tile = world.tile(tx, ty);
                if g >= tile.goods.len() { continue; }
                let base_x = tx as u32 * TILE_SIZE;
                let base_y = ty as u32 * TILE_SIZE;
                let max_lx = TILE_SIZE.min(grid_w - base_x);
                let max_ly = TILE_SIZE.min(grid_h - base_y);
                for ly in 0..max_ly {
                    for lx in 0..max_lx {
                        let v = tile.goods[g][(ly * TILE_SIZE + lx) as usize];
                        if v < COVERAGE_MIN_U8 { continue; }
                        belt[((base_y + ly) as usize) * (grid_w as usize) + (base_x + lx) as usize] = v;
                    }
                }
            }
        }

        let subtype_kind = match spec.id.as_str() { "wheat" => 1u8, "paper" => 2u8, _ => 0u8 };
        let classify = |wx: u32, wy: u32| -> u8 {
            let tile = world.tile((wx / TILE_SIZE) as i32, (wy / TILE_SIZE) as i32);
            let ti = ((wy % TILE_SIZE) * TILE_SIZE + (wx % TILE_SIZE)) as usize;
            let (t, p, e, hb) = (
                tile.temperature[ti], tile.precipitation[ti],
                tile.elevation[ti], tile.habitability[ti],
            );
            if subtype_kind == 1 { grain_subtype(t, p, e) } else { paper_subtype(t, p, e, hb) }
        };
        let sub: Option<&dyn Fn(u32, u32) -> u8> = if subtype_kind == 0 { None } else { Some(&classify) };
        if let Some(m) = build_belt_mask(&spec.id, &belt, grid_w, grid_h, coarse, sub) {
            out.push(m);
        }
    }
    Ok(out)
}


#[cfg(test)]
mod belt_mask_tests {
    use super::rle_encode_mask;

    /// The RLE is the whole reason a FULL-RESOLUTION coverage layer is affordable,
    /// so it has to round-trip exactly — a decoder disagreeing with the encoder would
    /// draw a belt shifted by however many cells the first bad run was long.
    #[test]
    fn the_coverage_rle_round_trips_exactly() {
        let cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![1],
            vec![0, 0, 0, 1, 1, 0, 1],
            vec![1; 1000],
            (0..500).map(|i| (i % 3 == 0) as u8).collect(),
        ];
        for src in cases {
            let rle = rle_encode_mask(&src);
            assert_eq!(rle.len() % 2, 0, "the RLE is flat (value, run) pairs");
            let mut back: Vec<u8> = Vec::with_capacity(src.len());
            for pair in rle.chunks(2) {
                for _ in 0..pair[1] { back.push(pair[0] as u8); }
            }
            assert_eq!(back, src, "coverage RLE did not round-trip");
        }
    }

    /// A contiguous belt is what makes the payload small; a run per cell would put a
    /// world-spanning staple region past any sane IPC size. This is the property, not
    /// a byte count: one solid block of coverage must cost a handful of runs, not one
    /// per cell.
    #[test]
    fn a_contiguous_belt_costs_few_runs() {
        let (w, h) = (200usize, 100usize);
        let mut mask = vec![0u8; w * h];
        for y in 20..80 {
            for x in 40..160 { mask[y * w + x] = 1; }
        }
        let rle = rle_encode_mask(&mask);
        // At most ~3 runs per row (lead-in zeros, the belt, trailing zeros), merged
        // across rows where they abut — far below one run per cell.
        assert!(rle.len() / 2 <= h * 3, "expected ≲3 runs/row, got {}", rle.len() / 2);
    }
}


/// Persist the current overlay state (settlements / rivers / lakes) to metadata so it
/// is included in the SQLite backup (save). The economy snapshot is already persisted
/// by `compute_economy`. Called by the frontend right before saving a world.
#[tauri::command]
pub fn persist_overlays(
    settlements_json: String,
    rivers_json: String,
    lakes_json: String,
    db: State<'_, WorldDb>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Settlements are campaign data; rivers/lakes are world geography.
    metadata::campaign_set(&conn, "settlements", &settlements_json).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "rivers", &rivers_json).map_err(|e| e.to_string())?;
    metadata::set_meta(&conn, "lakes", &lakes_json).map_err(|e| e.to_string())?;
    Ok(())
}


/// Read all persisted overlay state when re-opening a saved world (empty vectors when
/// a key is missing — e.g. an older save, or steps not yet run).
#[tauri::command]
pub fn get_overlays(db: State<'_, WorldDb>) -> Result<OverlaysState, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let read = |key: &str| -> String {
        metadata::get_meta(&conn, key).ok().flatten().unwrap_or_default()
    };
    let read_camp = |key: &str| -> String {
        metadata::campaign_get_or_meta(&conn, key).ok().flatten().unwrap_or_default()
    };
    let settlements = serde_json::from_str(&read_camp("settlements")).unwrap_or_default();
    let rivers = serde_json::from_str(&read("rivers")).unwrap_or_default();
    let lakes = serde_json::from_str(&read("lakes")).unwrap_or_default();
    let goods_names: Vec<String> = crate::commands::goods_commands::load_world_goods(&conn)
        .iter().map(|s| s.id.clone()).collect();
    let economy = serde_json::from_str(&read_camp("economy")).unwrap_or(EconomySnapshot {
        hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![],
        corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names,
        colonizable_sites: vec![],
    });
    Ok(OverlaysState { settlements, rivers, lakes, economy })
}


/// Build the river-system tree with full hydrological stats for the Hydrology
/// dashboard. Takes the world's rivers + settlements (from the frontend store),
/// loads the elevation/precip columns and recomputes flow accumulation to size
/// discharge, then assembles trunks with their tributaries nested (full depth).
#[tauri::command]
pub fn get_river_systems(
    rivers_json: String,
    settlements_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<RiverNode>, String> {
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Rivers columns + TEMPERATURE (thermal band for the fish/ecology layer).
    let cols = ColumnSet(ColumnSet::PHASE_RIVERS.0 | ColumnSet::TEMPERATURE.0);
    let buf = WorldBuffer::load_with(&conn, cols)?;

    let rivers: Vec<crate::sim::rivers::River> =
        serde_json::from_str(&rivers_json).map_err(|e| e.to_string())?;
    if rivers.is_empty() { return Ok(vec![]); }
    let settles: Vec<RiverSettleIn> = serde_json::from_str(&settlements_json).unwrap_or_default();
    Ok(build_river_systems(&buf, &rivers, &settles))
}


/// Classify the world's lakes and build their limnological + ecological profiles
/// (type, depth, thermal/trophic regime, fish, wildlife, endemism, real-world
/// analog, inflow/outflow rivers). Read-only; mirrors `get_river_systems`.
#[tauri::command]
pub fn get_lake_systems(
    lakes_json: String,
    rivers_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<LakeNode>, String> {
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    // Rivers columns + temperature (thermal band) + plates/volcanic (rift/crater)
    // + salinity (for completeness). All needed to classify a lake.
    let cols = ColumnSet(
        ColumnSet::PHASE_RIVERS.0 | ColumnSet::TEMPERATURE.0
            | ColumnSet::PLATES.0 | ColumnSet::VOLCANIC.0 | ColumnSet::SALINITY.0,
    );
    let buf = WorldBuffer::load_with(&conn, cols)?;
    let lakes: Vec<crate::sim::rivers::Lake> =
        serde_json::from_str(&lakes_json).map_err(|e| e.to_string())?;
    if lakes.is_empty() { return Ok(vec![]); }
    let rivers: Vec<crate::sim::rivers::River> =
        serde_json::from_str(&rivers_json).unwrap_or_default();
    Ok(build_lake_systems(&buf, &lakes, &rivers))
}

/// Latitude bands of the general circulation — the ITCZ rain line plus the
/// subtropical-high (desert) and polar-front (storm-track) belts — for the Climate
/// Bands overlay. The belt latitudes come from the rotation-driven `Circulation`
/// model, so they move when the Planet panel changes rotation/greenhouse/sunlight;
/// the ITCZ line is the per-column land-asymmetry position (it migrates poleward
/// over the continents, equatorward over the oceans, exactly as on Earth).
#[derive(Debug, Serialize)]
pub struct ClimateBands {
    /// Grid width (the ITCZ polyline has one latitude per column).
    pub width: u32,
    /// Per-column ITCZ latitude (degrees, +N), annual mean. Length = `width`.
    pub itcz: Vec<f32>,
    /// Per-column ITCZ latitude at the two SEASONAL EXTREMES (degrees, +N).
    ///
    /// These are not decorative curves: they are the exact line
    /// `seasonal::belt_wind_shifted` displaces the wind belts about in each
    /// insolation state, so the band between them is the belt of land that changes
    /// circulation regime between seasons — which is what a monsoon climate IS.
    /// Both bulge poleward over continents, because `itcz_land_pull` measures the
    /// summer hemisphere's subtropical land fraction per meridian: land–sea
    /// contrast selects the LONGITUDE the convergence zone reaches furthest
    /// poleward, which is why July runs far north over Asia and stays near 10°N
    /// over the open Pacific.
    pub itcz_july: Vec<f32>,
    pub itcz_january: Vec<f32>,
    /// Subtropical-high / Hadley-edge latitude (degrees). Dry desert belt. ~30° on Earth.
    pub hadley_edge: f32,
    /// Polar-front / storm-track latitude (degrees). ~60° on Earth.
    pub polar_front: f32,
    /// Circulation cells per hemisphere (3 on Earth).
    pub cells: u32,
}

/// Compute the climate-band overlay data (ITCZ line + circulation belts).
#[tauri::command]
pub fn compute_climate_bands(db: State<'_, WorldDb>) -> Result<ClimateBands, String> {
    use crate::sim::world_buffer::{WorldBuffer, ColumnSet};
    use crate::sim::circulation::Circulation;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 {
        return Ok(ClimateBands {
            width: 0, itcz: vec![], itcz_july: vec![], itcz_january: vec![],
            hadley_edge: 30.0, polar_front: 60.0, cells: 3,
        });
    }
    // Terrain drives the per-column ITCZ shift (land-fraction asymmetry); the planet
    // state (loaded into the buffer) drives the circulation belts.
    let buf = WorldBuffer::load_with(&conn, ColumnSet::TERRAIN)?;
    let itcz = crate::sim::precipitation::compute_itcz_shift_zonal(&buf);
    let circ = Circulation::for_world(&buf);
    // The two seasonal extremes, from the SAME helpers the seasonal wind field is
    // built from — so the drawn convergence zone is provably the one the winds
    // were displaced about, not a second, cosmetic model of it.
    let pull_jul = crate::sim::seasonal::itcz_land_pull(&buf, 1.0);
    let pull_jan = crate::sim::seasonal::itcz_land_pull(&buf, -1.0);
    let itcz_july: Vec<f32> = (0..grid_w as usize)
        .map(|x| crate::sim::seasonal::itcz_latitude(pull_jul[x], 1.0))
        .collect();
    let itcz_january: Vec<f32> = (0..grid_w as usize)
        .map(|x| crate::sim::seasonal::itcz_latitude(pull_jan[x], -1.0))
        .collect();
    Ok(ClimateBands {
        width: grid_w,
        itcz,
        itcz_july,
        itcz_january,
        hadley_edge: circ.hadley_edge,
        polar_front: circ.polar_front,
        cells: circ.cells,
    })
}
