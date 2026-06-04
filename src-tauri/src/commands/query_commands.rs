use serde::{Serialize, Deserialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tauri::State;
use crate::db::{WorldDb, CostKey, tile_store, metadata};
use crate::db::world_cache::WorldTiles;
use crate::tile::coords::{TileCoord, TILE_SIZE};
use crate::tile::cell::TileData;
use crate::sim::biological::GOOD_NAMES;

// Salinity u8 ↔ PSU mapping (mirror of sim/ocean.rs).
const SAL_MIN_PSU: f32 = 28.0;
const SAL_MAX_PSU: f32 = 42.0;

#[derive(Serialize)]
pub struct CellInfo {
    pub wx: u32,
    pub wy: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub terrain: String,
    pub elevation: f32,
    pub sea_depth: f32,
    pub temperature: f32,
    pub precipitation: f32,
    pub koppen: u8,
    pub biome: String,
    pub soil_type: u8,
    pub fertility: f32,
    pub fishery: f32,
    pub plate_index: u16,
    pub is_volcanic: bool,
    pub is_shelf: bool,
    pub wind_vx: f32,
    pub wind_vy: f32,
    pub current_vx: f32,
    pub current_vy: f32,
    pub current_type: u8,
    pub distance_to_ocean: f32,
    pub salinity: f32,    // PSU
    pub shark_risk: f32,  // 0..1
    pub shipworm_risk: f32, // 0..1
    pub storm_risk: f32,  // 0..1 (annual storm potential)
    pub reef_risk: f32,   // 0..1
    pub goods: Vec<GoodAmount>,
}

#[derive(Serialize)]
pub struct GoodAmount {
    pub name: String,
    pub amount: u8, // 0..255 belt intensity
}

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

    let biome = koppen_to_biome(koppen, elevation, is_land);

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

/// Derive biome name from Köppen code + elevation (matches tile_image.rs biome layer).
fn koppen_to_biome(koppen: u8, elevation: f32, is_land: bool) -> String {
    if !is_land { return "Ocean".into(); }
    // High elevation overrides (match render/tile_image.rs biome_color)
    if elevation > 0.62 { return "Alpine".into(); }
    if elevation > 0.40 { return "Montane".into(); }
    match koppen {
        1 => "Tropical Rainforest",
        2 => "Tropical Monsoon Forest",
        3 => "Tropical Savanna",
        4 => "Hot Desert",
        5 => "Cold Desert",
        6 => "Hot Steppe",
        7 => "Cold Steppe",
        8 | 9 | 10 => "Mediterranean Scrubland",
        11 => "Subtropical Forest",
        12 => "Temperate Broadleaf Forest",
        13 => "Subpolar Forest",
        14 | 15 => "Temperate Mixed Forest",
        16 | 17 => "Boreal/Taiga",
        18 | 19 | 20 => "Continental Scrubland",
        21 => "Tundra",
        22 => "Ice Cap",
        23 => "Tropical Savanna",
        24 => "Humid Subtropical (monsoon)",
        25 => "Subtropical Highland",
        26 => "Cold Subtropical Highland",
        27 | 28 => "Continental Forest (dry winter)",
        29 | 30 => "Boreal/Taiga (dry winter)",
        31 => "Dry-summer Subarctic",
        32 => "Highland / Alpine",
        _ => "Unknown",
    }.into()
}

#[derive(Serialize)]
pub struct VectorSample {
    pub x: u32,
    pub y: u32,
    pub vx: f32,
    pub vy: f32,
    #[serde(rename = "type")]
    pub vec_type: u8, // 0=none, 1=warm, 2=cold (for currents)
}

#[derive(Serialize)]
pub struct OverlayVectors {
    pub wind: Vec<VectorSample>,
    pub currents: Vec<VectorSample>,
    /// World-cell spacing between sampled arrows (lets the frontend size them).
    pub current_step: u32,
}

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

/// A traced ocean-current streamline: an ordered list of [x, y] world points
/// forming one continuous arrow from where the current starts to where it ends.
/// `ctype`: 0 = neutral (equatorial / counter-current / gyre return / drift),
/// 1 = warm boundary current, 2 = cold boundary current / ACC.
#[derive(Serialize)]
pub struct Streamline {
    pub points: Vec<[f32; 2]>,
    pub ctype: u8,
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

    // Load the full-resolution current field into flat arrays.
    let mut vx = vec![0.0f32; n];
    let mut vy = vec![0.0f32; n];
    let mut ctype = vec![0u8; n];
    let mut terrain = vec![0u8; n];

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
                }
            }
        }
    }

    let wrap_x = |x: i32| -> i32 { ((x % w) + w) % w };
    let idx = |x: i32, y: i32| -> usize { (y as usize) * grid_w as usize + (wrap_x(x) as usize) };

    // Nearest sample of the current vector at a float position.
    let sample = |fx: f32, fy: f32| -> (f32, f32, u8, u8) {
        let xi = fx.floor() as i32;
        let yi = fy.floor() as i32;
        if yi < 0 || yi >= h { return (0.0, 0.0, 0, 1); }
        let i = idx(xi, yi);
        (vx[i], vy[i], ctype[i], terrain[i])
    };

    // Does the current family `fam` accept a cell of `cell_type`?
    //   fam 1 (warm)    → warm only
    //   fam 2 (cold)    → cold only
    //   fam 0 (neutral) → neutral only (equatorial / counter-current / gyre / drift)
    let accepts = |fam: u8, cell_type: u8| -> bool { fam == cell_type };

    // Seed density: spaced grid so we don't start a line in every cell.
    let seed_step = (grid_w / 90).clamp(6, 30) as i32;
    let visited_cell = (grid_w / 28).clamp(3, 12) as i32; // corridor half-spacing
    let max_steps = 700usize;
    let step_len = 1.5f32;
    let min_points = 6usize;

    let mut out: Vec<Streamline> = Vec::new();

    // Integrate a streamline of family `fam` from (sx,sy) in `dir` (+1 fwd, -1 back).
    // Stops at land (checking the *next* cell before stepping so the line never
    // draws onto a continent) and when the current weakens or changes family.
    let integrate = |sx: i32, sy: i32, dir: f32, fam: u8| -> Vec<[f32; 2]> {
        let mut pts: Vec<[f32; 2]> = Vec::new();
        let mut px = sx as f32 + 0.5;
        let mut py = sy as f32 + 0.5;
        for _ in 0..max_steps {
            let (svx, svy, sct, sterr) = sample(px, py);
            if sterr == 1 { break; }            // current's own cell is land
            let mag = (svx * svx + svy * svy).sqrt();
            if mag < 0.05 { break; }            // current died out
            if !accepts(fam, sct) { break; }    // changed family → end this line
            pts.push([wrap_x(px.floor() as i32) as f32, py]);
            // Look ahead: if the next position is land or off-grid, stop here so
            // the polyline terminates at the coast instead of crossing it.
            let nx = px + dir * (svx / mag) * step_len;
            let ny = py + dir * (svy / mag) * step_len;
            if ny < 0.0 || ny >= h as f32 { break; }
            let (_, _, _, nterr) = sample(nx, ny);
            if nterr == 1 { break; }
            px = nx;
            py = ny;
        }
        pts
    };

    // Per-family seeding. Neutral lines need a lower magnitude floor so the
    // equatorial currents and gyre return limbs (which are slower) still show.
    let families: [(u8, f32); 3] = [(1, 0.30), (2, 0.30), (0, 0.22)];
    for &(fam, mag_floor) in &families {
        let mut visited = vec![false; n];
        for sy in (0..h).step_by(seed_step as usize) {
            for sx in (0..w).step_by(seed_step as usize) {
                let i = idx(sx, sy);
                if terrain[i] == 1 || ctype[i] != fam { continue; }
                if visited[i] { continue; }
                let mag = (vx[i] * vx[i] + vy[i] * vy[i]).sqrt();
                if mag < mag_floor { continue; }

                let mut back = integrate(sx, sy, -1.0, fam);
                back.reverse();
                let fwd = integrate(sx, sy, 1.0, fam);
                let mut line = back;
                if !fwd.is_empty() { line.extend_from_slice(&fwd[1.min(fwd.len())..]); }

                if line.len() < min_points { continue; }

                for p in &line {
                    let cx = p[0] as i32;
                    let cy = p[1] as i32;
                    for dy in -visited_cell..=visited_cell {
                        let ny = cy + dy;
                        if ny < 0 || ny >= h { continue; }
                        for dx in -visited_cell..=visited_cell {
                            visited[idx(cx + dx, ny)] = true;
                        }
                    }
                }
                out.push(Streamline { points: line, ctype: fam });
            }
        }
    }

    Ok(out)
}

#[derive(Serialize)]
pub struct ElevationBand {
    pub label: String,
    pub count: u32,
    pub percentage: f32,
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

// ─────────────────────────────────────────────────────────────────────────────
// Trade routes
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal settlement shape we need for routing (sent from the frontend store).
#[derive(Deserialize)]
struct RouteSettlement {
    x: u32,
    y: u32,
    #[serde(default)]
    score: f32,
    #[serde(default)]
    population: u32,
}

/// A river, as sent from the frontend store (only the cell path is needed).
#[derive(Deserialize)]
struct RouteRiver {
    points: Vec<(u32, u32)>,
}

/// A computed trade route between two settlements.
/// `kind`: 0 = overland caravan, 1 = maritime (sea-dominant), 2 = river route
/// (inland, following navigable rivers).
#[derive(Serialize)]
pub struct TradeRoute {
    pub points: Vec<[f32; 2]>,
    pub kind: u8,
    /// True for a minor connector road (a lesser town's single link to the
    /// network) so the overlay can draw it thinner than a major trunk route.
    pub minor: bool,
}

// ── Shared coarse movement-cost grid ─────────────────────────────────────────

/// A coarse (~700-wide) movement-cost grid shared by trade routes, the routed
/// trade-flow trunks, and the political layer. Land cost rises with relief and
/// climate hostility, but mountain **passes** (saddles) are discounted so
/// caravans thread the gaps instead of detouring around whole ranges; navigable
/// rivers are cheap inland highways; coastal sea is cheap coast-hugging
/// shipping; open sea is moderate — or fully blocked when `block_sea` (used for
/// continental-only trade so paths stay on one landmass).
struct CoarseCost {
    f: u32,
    cw: i32,
    ch: i32,
    cost: Vec<f32>,
    is_land: Vec<bool>,
    is_open_sea: Vec<bool>, // sea cell not adjacent to land (true open water)
    is_river: Vec<bool>,
}

const SEA_BLOCK_COST: f32 = 1.0e6;
/// Cost of an open-water (no land neighbour) coarse cell. Much higher than the
/// coastal-sea cost (0.5) so least-cost routes/flows hug the coast and cross open
/// ocean only at the narrowest point, rather than cutting straight across a basin.
const OPEN_SEA_COST: f32 = 3.5;

impl CoarseCost {
    #[inline]
    fn wrap_cx(&self, x: i32) -> i32 { ((x % self.cw) + self.cw) % self.cw }
    #[inline]
    fn cidx(&self, x: i32, y: i32) -> usize { (y * self.cw + self.wrap_cx(x)) as usize }
    #[inline]
    fn world_of(&self, c: usize) -> [f32; 2] {
        let cx = (c as i32) % self.cw;
        let cy = (c as i32) / self.cw;
        [(cx as u32 * self.f + self.f / 2) as f32, (cy as u32 * self.f + self.f / 2) as f32]
    }
}

fn build_coarse_cost(
    world: &WorldTiles, grid_w: u32, grid_h: u32, rivers_json: &str, block_sea: bool,
    desert_routes: bool,
) -> Result<CoarseCost, String> {
    let f = (grid_w / 700).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let cn = (cw * ch) as usize;

    // Sample each coarse cell from its centre fine cell (one tile pass).
    let mut is_land = vec![false; cn];
    let mut elev = vec![0.0f32; cn];
    let mut koppen = vec![0u8; cn];
    // Maritime-hazard exposure per coarse cell (annual storm peak + reef wreck
    // risk), used to make stormy/reef-fouled sea more expensive to cross.
    let mut sea_hazard = vec![0.0f32; cn];
    {
        let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
        let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
        // Load fine fields, then resample. (We only need the centre cell of each
        // coarse block, but a full load keeps the index math simple.)
        let fn_cells = (grid_w * grid_h) as usize;
        let mut f_terrain = vec![0u8; fn_cells];
        let mut f_elev = vec![0.0f32; fn_cells];
        let mut f_koppen = vec![0u8; fn_cells];
        let mut f_hazard = vec![0.0f32; fn_cells];
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
                        f_terrain[gi] = tile.terrain[ti];
                        f_elev[gi] = tile.elevation[ti];
                        f_koppen[gi] = tile.koppen[ti];
                        f_hazard[gi] = (tile.storm_base[ti].max(tile.reef_risk[ti])) as f32 / 255.0;
                    }
                }
            }
        }
        for cy in 0..ch {
            for cx in 0..cw {
                let wx = (cx as u32 * f + f / 2).min(grid_w - 1);
                let wy = (cy as u32 * f + f / 2).min(grid_h - 1);
                let gi = (wy * grid_w + wx) as usize;
                let ci = (cy * cw + cx) as usize;
                is_land[ci] = f_terrain[gi] == 1;
                elev[ci] = f_elev[gi];
                koppen[ci] = f_koppen[gi];
                sea_hazard[ci] = f_hazard[gi];
            }
        }
    }

    // Coarse river mask from overlay JSON (rivers aren't stored in tiles).
    let mut is_river = vec![false; cn];
    {
        let rivers: Vec<RouteRiver> = serde_json::from_str(rivers_json).unwrap_or_default();
        for r in &rivers {
            for &(rx, ry) in &r.points {
                let cx = (rx / f).min(cw as u32 - 1) as i32;
                let cy = (ry / f).min(ch as u32 - 1) as i32;
                is_river[(cy * cw + cx) as usize] = true;
            }
        }
    }

    let wrap_cx = |x: i32| -> i32 { ((x % cw) + cw) % cw };

    // Base cost.
    let mut cost = vec![1.0f32; cn];
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            cost[ci] = if is_land[ci] {
                // Lower relief multiplier than before (22 → 14) so interiors are
                // traversable and inland trade actually happens.
                let mut c = 4.0 + elev[ci] * 14.0;
                // Desert/steppe surcharge. In "Silk Road" mode (overland caravans
                // preferred over dangerous seas) the arid-land penalty is cut so
                // STEPPE corridors (the Silk Road's main highway) and deserts
                // become competitive overland routes. Steppe stays cheaper than
                // desert, so caravans favour the grass corridors.
                let (desert, steppe) = if desert_routes { (2.0, 0.5) } else { (9.0, 2.5) };
                c += match koppen[ci] {
                    4 | 5 => desert,        // BWh / BWk hot & cold desert
                    6 | 7 => steppe,        // BSh / BSk steppe
                    21 => 12.0,             // ET tundra
                    22 => 26.0,             // EF ice cap
                    16 | 17 | 29 | 30 => 5.0,
                    1 => 5.0,               // AF rainforest
                    32 => 7.0,              // H highland
                    _ => 0.0,
                };
                if is_river[ci] { c = c.min(1.2); }
                c
            } else if block_sea {
                SEA_BLOCK_COST
            } else {
                // Open water is now markedly dearer than coastal sea (set to 0.5
                // below). Routes and the trade-flow trunks both least-cost over
                // this grid, so a high open-sea base makes them hug the coast and
                // take the SHORTEST crossing through straits/chokepoints instead
                // of beelining straight across deep ocean between continents.
                OPEN_SEA_COST
            };
        }
    }

    // Mountain-pass (saddle) discount: a moderately high land cell that is a
    // local low along one axis (a gap between higher flanks) is a pass, so cut
    // its cost — caravans thread passes rather than going over/around ranges.
    {
        let base = elev.clone();
        for cy in 0..ch {
            for cx in 0..cw {
                let ci = (cy * cw + cx) as usize;
                if !is_land[ci] || base[ci] < 0.33 { continue; }
                let e = base[ci];
                let l = base[(cy * cw + wrap_cx(cx - 1)) as usize];
                let r = base[(cy * cw + wrap_cx(cx + 1)) as usize];
                let up = if cy > 0 { base[((cy - 1) * cw + cx) as usize] } else { e };
                let dn = if cy < ch - 1 { base[((cy + 1) * cw + cx) as usize] } else { e };
                let gap_ew = e < l && e < r && (l.min(r) - e) > 0.04;
                let gap_ns = e < up && e < dn && (up.min(dn) - e) > 0.04;
                if (gap_ew || gap_ns) && is_land[(cy * cw + wrap_cx(cx - 1)) as usize] {
                    cost[ci] *= 0.45;
                }
            }
        }
    }

    // Coastal sea is cheap shipping (unless sea is blocked entirely).
    if !block_sea {
        let base = is_land.clone();
        for cy in 0..ch {
            for cx in 0..cw {
                let ci = (cy * cw + cx) as usize;
                if base[ci] { continue; }
                let coastal = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                    let ny = cy + dy;
                    if ny < 0 || ny >= ch { return false; }
                    base[(ny * cw + wrap_cx(cx + dx)) as usize]
                });
                if coastal { cost[ci] = 0.5; }
            }
        }
    }

    // Maritime-hazard surcharge: stormy / reef-fouled sea is dearer to cross, so
    // routes detour around cyclone belts and reefs, and goods reachable only over
    // hazardous water grow scarcer downstream. Applied only when sea is passable.
    if !block_sea {
        // Stormy / reef-fouled sea is dearer to cross. In "desert routes" mode the
        // surcharge is large so caravans take the safe overland (desert) path when
        // the waters are dangerous, rather than risking the crossing.
        let hazard_w: f32 = if desert_routes { 20.0 } else { 7.0 };
        for ci in 0..cn {
            if !is_land[ci] {
                cost[ci] += sea_hazard[ci].clamp(0.0, 1.0) * hazard_w;
            }
        }
    }

    // Open-water mask (sea cell with no land neighbour) for crossing-length checks.
    let mut is_open_sea = vec![false; cn];
    {
        let base = is_land.clone();
        for cy in 0..ch {
            for cx in 0..cw {
                let ci = (cy * cw + cx) as usize;
                if base[ci] { continue; }
                let coastal = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                    let ny = cy + dy;
                    if ny < 0 || ny >= ch { return false; }
                    base[(ny * cw + wrap_cx(cx + dx)) as usize]
                });
                is_open_sea[ci] = !coastal;
            }
        }
    }

    Ok(CoarseCost { f, cw, ch, cost, is_land, is_open_sea, is_river })
}

const COARSE_DIRS: [(i32, i32, f32); 8] = [
    (-1, 0, 1.0), (1, 0, 1.0), (0, -1, 1.0), (0, 1, 1.0),
    (-1, -1, 1.4142), (1, -1, 1.4142), (-1, 1, 1.4142), (1, 1, 1.4142),
];

/// Memoized `build_coarse_cost`. The trade-route, trade-matrix and political
/// commands all build the *same* coarse cost grid (same world + same rivers), so
/// cache the last one on `WorldDb`, keyed by (tile fingerprint, block_sea, rivers
/// hash). Within one overlay fan-out only the first call pays the build; a tile
/// edit (fingerprint change) or a rivers change invalidates it.
fn cached_coarse_cost(
    db: &WorldDb,
    world: &WorldTiles,
    fingerprint: (i64, i64),
    grid_w: u32,
    grid_h: u32,
    rivers_json: &str,
    block_sea: bool,
    desert_routes: bool,
) -> Result<Arc<CoarseCost>, String> {
    let mut hasher = DefaultHasher::new();
    rivers_json.hash(&mut hasher);
    desert_routes.hash(&mut hasher); // distinct grid when desert-routing is on
    let key: CostKey = (fingerprint.0, fingerprint.1, block_sea, hasher.finish());
    if let Some(any) = db.cost_cache_get(&key) {
        if let Ok(cc) = any.downcast::<CoarseCost>() {
            return Ok(cc);
        }
    }
    let cc = Arc::new(build_coarse_cost(world, grid_w, grid_h, rivers_json, block_sea, desert_routes)?);
    db.cost_cache_put(key, cc.clone());
    Ok(cc)
}

/// Least-cost path (coarse indices, start→goal) over a CoarseCost grid, or None.
fn coarse_dijkstra(cc: &CoarseCost, start: usize, goal: usize) -> Option<Vec<usize>> {
    let cw = cc.cw;
    let ch = cc.ch;
    let cn = (cw * ch) as usize;
    let mut dist = vec![i64::MAX; cn];
    let mut prev = vec![usize::MAX; cn];
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    dist[start] = 0;
    heap.push(Reverse((0, start)));
    while let Some(Reverse((d, u))) = heap.pop() {
        if u == goal { break; }
        if d > dist[u] { continue; }
        let ux = (u as i32) % cw;
        let uy = (u as i32) / cw;
        for &(dx, dy, mult) in &COARSE_DIRS {
            let ny = uy + dy;
            if ny < 0 || ny >= ch { continue; }
            let v = cc.cidx(ux + dx, ny);
            let step = ((cc.cost[u] + cc.cost[v]) * 0.5 * mult * 100.0) as i64;
            let nd = d.saturating_add(step.max(1));
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = u;
                heap.push(Reverse((nd, v)));
            }
        }
    }
    if dist[goal] == i64::MAX { return None; }
    let mut path = Vec::new();
    let mut cur = goal;
    while cur != usize::MAX {
        path.push(cur);
        if cur == start { break; }
        cur = prev[cur];
    }
    path.reverse();
    if path.len() < 2 { None } else { Some(path) }
}

/// Is a path acceptable under the chosen trade reach?
///   reach 0 = global (any crossing) · 1 = coastal+short crossings (open-water
///   run capped at `max_crossing_frac` of the width) · 2 = continental (no sea).
fn path_allowed(cc: &CoarseCost, path: &[usize], reach: u8, max_crossing_frac: f32, grid_w: u32) -> bool {
    match reach {
        2 => path.iter().all(|&c| cc.is_land[c]),
        1 => {
            let mut run = 0u32;
            let mut best = 0u32;
            for &c in path {
                if cc.is_open_sea[c] { run += 1; best = best.max(run); } else { run = 0; }
            }
            (best * cc.f) as f32 <= max_crossing_frac.max(0.0) * grid_w as f32
        }
        _ => true,
    }
}

/// Compute plausible trade routes between the major settlements over the shared
/// coarse cost grid (mountain passes, rivers, coast-hugging all priced in), with
/// the chosen trade reach limiting how far trade may cross open water.
#[tauri::command]
pub fn compute_trade_routes(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TradeRoute>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }

    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.len() < 2 { return Ok(vec![]); }

    let world = db.cached_tiles_with_conn(&conn)?;
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes)?;
    let (cw, f) = (cc.cw, cc.f);

    // Map EVERY settlement to a coarse node (sorted by score, strongest first).
    // The major network is built among the top hubs (3 nearest neighbours each),
    // but every remaining settlement still gets at least one minor road to its
    // nearest neighbour, so no town is left unconnected.
    let mut nodes: Vec<(i32, i32, f32)> = settlements.iter()
        .map(|s| ((s.x / f).min(cw as u32 - 1) as i32, (s.y / f).min(cc.ch as u32 - 1) as i32, s.score))
        .collect();
    nodes.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let nn = nodes.len();
    if nn < 2 { return Ok(vec![]); }
    // Primary-network node count scales with the chosen economic granularity
    // (default 14 regions ≈ 84 hubs, matching the legacy fixed 80).
    let hubs = nn.min(((economic_regions.clamp(2, 40) as usize) * 6).clamp(10, 200));

    // Candidate links: top hubs link to their 3 nearest neighbours; every other
    // settlement links to its single nearest neighbour (a minor road). Tracked
    // separately so minor roads can be drawn thinner.
    let mut major_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut minor_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let nearest_k = |i: usize, k: usize| -> Vec<usize> {
        let mut dists: Vec<(usize, i64)> = Vec::with_capacity(nn - 1);
        for j in 0..nn {
            if i == j { continue; }
            let mut dx = (nodes[i].0 - nodes[j].0).abs();
            if dx > cw / 2 { dx = cw - dx; }
            let dy = nodes[i].1 - nodes[j].1;
            dists.push((j, (dx * dx + dy * dy) as i64));
        }
        dists.sort_by_key(|&(_, d)| d);
        dists.iter().take(k).map(|&(j, _)| j).collect()
    };
    for i in 0..hubs {
        for j in nearest_k(i, 3) { major_edges.insert((i.min(j), i.max(j))); }
    }
    // Each lesser town links to its nearest HUB (a major-network node), not merely
    // its nearest neighbour — otherwise small towns chain only to each other and
    // never reach the trunk network. This guarantees every settlement is attached
    // to the major trade routes.
    let nearest_hub = |i: usize| -> Option<usize> {
        let mut best = None;
        let mut bd = i64::MAX;
        for j in 0..hubs {
            if i == j { continue; }
            let mut dx = (nodes[i].0 - nodes[j].0).abs();
            if dx > cw / 2 { dx = cw - dx; }
            let dy = nodes[i].1 - nodes[j].1;
            let d = (dx * dx + dy * dy) as i64;
            if d < bd { bd = d; best = Some(j); }
        }
        best
    };
    for i in hubs..nn {
        if let Some(j) = nearest_hub(i) {
            let e = (i.min(j), i.max(j));
            if !major_edges.contains(&e) { minor_edges.insert(e); }
        }
    }

    let mut routes: Vec<TradeRoute> = Vec::new();
    let all_edges: Vec<((usize, usize), bool)> = major_edges.iter().map(|&e| (e, false))
        .chain(minor_edges.iter().map(|&e| (e, true)))
        .collect();
    for &((a, b), minor) in &all_edges {
        let start = cc.cidx(nodes[a].0, nodes[a].1);
        let goal = cc.cidx(nodes[b].0, nodes[b].1);
        let path = match coarse_dijkstra(&cc, start, goal) { Some(p) => p, None => continue };
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }

        let mut sea_cells = 0u32;
        let mut river_cells = 0u32;
        let pts: Vec<[f32; 2]> = path.iter().map(|&c| {
            if !cc.is_land[c] { sea_cells += 1; }
            if cc.is_land[c] && cc.is_river[c] { river_cells += 1; }
            cc.world_of(c)
        }).collect();

        let kind = if sea_cells as usize * 3 >= path.len() {
            1
        } else if river_cells as usize * 4 >= path.len() {
            2
        } else {
            0
        };
        routes.push(TradeRoute { points: pts, kind, minor });
    }

    Ok(routes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Fishery grand banks
// ─────────────────────────────────────────────────────────────────────────────

/// A "grand bank": a circular region marking a rich fishing ground. `x`,`y` are
/// the world-cell centre, `radius` is in world cells, `score` the mean fishery
/// productivity (0..1).
#[derive(Serialize)]
pub struct FisheryBank {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub score: f32,
}

/// The *static* (non-seasonal) map overlays after a sim step, gathered in ONE
/// command so the frontend makes a single IPC round-trip (and shares one
/// decompressed-tile cache read across all of them) instead of separate calls
/// each re-locking the connection. Storm zones are intentionally NOT here — they
/// depend on the month slider, so bundling them made every month tick recompute
/// all of these (the scrubbing lag). Storm has its own light command.
#[derive(Serialize)]
pub struct OverlaysResult {
    pub vectors: OverlayVectors,
    pub streamlines: Vec<Streamline>,
    pub fishery_banks: Vec<FisheryBank>,
    pub shark_zones: Vec<SharkZone>,
    pub shipworm_zones: Vec<SharkZone>,
    pub reef_zones: Vec<SharkZone>,
    pub good_regions: Vec<GoodRegion>,
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

// ─────────────────────────────────────────────────────────────────────────────
// Biological overlays: shark zones, trade-good regions, trade matrix
// ─────────────────────────────────────────────────────────────────────────────

/// One connected coarse cluster: the member coarse cells (as world top-left
/// coords), a label centroid, and the mean score. Used to draw a marked AREA
/// (filled cell mask) rather than an abstract circle.
struct CoarseCluster {
    cells: Vec<[f32; 2]>,
    values: Vec<f32>, // per-cell field value (parallel to `cells`), for abundance shading
    cx: f32,
    cy: f32,
    score: f32,
}

/// Flood-fill connected coarse cells above `thresh` into clusters, returning each
/// cluster's member cells + centroid + mean score. X wraps.
fn cluster_cells(
    field: &[f32], cw: i32, ch: i32, f: u32, thresh: f32, min_cells: usize,
) -> Vec<CoarseCluster> {
    let cn = (cw * ch) as usize;
    let wrap_cx = |x: i32| -> i32 { ((x % cw) + cw) % cw };
    let mut visited = vec![false; cn];
    let mut out: Vec<CoarseCluster> = Vec::new();

    for start in 0..cn {
        if visited[start] || field[start] < thresh { continue; }
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
                if !visited[ni] && field[ni] >= thresh {
                    visited[ni] = true;
                    stack.push(ni);
                }
            }
        }
        if cells.len() < min_cells { continue; }

        let cx0 = (cells[0] as i32) % cw;
        let (mut sx, mut sy, mut ssc) = (0.0f64, 0.0f64, 0.0f32);
        let mut world: Vec<[f32; 2]> = Vec::with_capacity(cells.len());
        let mut vals: Vec<f32> = Vec::with_capacity(cells.len());
        for &ci in &cells {
            let mut cx = (ci as i32) % cw;
            let cy = (ci as i32) / cw;
            world.push([(cx as u32 * f) as f32, (cy as u32 * f) as f32]);
            vals.push(field[ci]);
            if cx - cx0 > cw / 2 { cx -= cw; } else if cx0 - cx > cw / 2 { cx += cw; }
            sx += cx as f64;
            sy += cy as f64;
            ssc += field[ci];
        }
        let nn = cells.len() as f64;
        let mean_cx = wrap_cx((sx / nn).round() as i32);
        let mean_cy = (sy / nn).round() as i32;
        out.push(CoarseCluster {
            cells: world,
            values: vals,
            cx: (mean_cx as u32 * f + f / 2) as f32,
            cy: (mean_cy as u32 * f + f / 2) as f32,
            score: ssc / cells.len() as f32,
        });
    }
    out
}

/// splitmix64-style u64 → [0,1) hash (for deterministic value noise).
#[inline]
fn hash01u(mut x: u64) -> f32 {
    x ^= x >> 33; x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33; x = x.wrapping_mul(0xC4CEB9FE1A85EC53); x ^= x >> 33;
    ((x >> 40) as f32) / 16_777_216.0
}

/// Smooth deterministic value noise in [0,1] over coarse-cell coords, with the X
/// lattice wrapped to `cw` so it is seamless across the date line. Used to break a
/// smooth hazard band (e.g. the cyclone belt) into several discrete blobs.
fn value_noise(cx: i32, cy: i32, cw: i32, period: f32, seed: u64) -> f32 {
    let lc = ((cw as f32 / period).round() as i32).max(1); // lattice columns (wrap period)
    let fx = cx as f32 / period;
    let fy = cy as f32 / period;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let latt = |ix: i32, iy: i32| -> f32 {
        let wx = ix.rem_euclid(lc);
        hash01u((wx as u64).wrapping_mul(0x9E3779B97F4A7C15)
            ^ (iy as u64).wrapping_mul(0xC2B2AE3D27D4EB4F)
            ^ seed)
    };
    let n00 = latt(x0, y0);
    let n10 = latt(x0 + 1, y0);
    let n01 = latt(x0, y0 + 1);
    let n11 = latt(x0 + 1, y0 + 1);
    let a = n00 + (n10 - n00) * sx;
    let b = n01 + (n11 - n01) * sx;
    a + (b - a) * sy
}

#[derive(Serialize)]
pub struct SharkZone {
    pub cells: Vec<[f32; 2]>, // coarse cell top-left world coords (the marked area)
    pub cell_size: f32,       // coarse cell size in world cells
    pub x: f32,               // label centroid
    pub y: f32,
    pub score: f32,
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

#[derive(Serialize)]
pub struct GoodRegion {
    pub good: String,
    pub cells: Vec<[f32; 2]>, // coarse cell top-left world coords (the marked area)
    pub values: Vec<u8>,      // per-cell abundance 0..255 (parallel to `cells`) for shading
    pub subtypes: Vec<u8>,    // per-cell subtype id (grain/paper); empty if the good has none
    pub cell_size: f32,
    pub x: f32,               // label centroid
    pub y: f32,
    pub score: f32,
    pub sublabel: String,     // e.g. the specific gemstone (Ruby/Sapphire/…); else ""
}

/// Grain species at a cell from its climate (0 wheat, 1 rice, 2 maize, 3 millet,
/// 4 barley). The "wheat" belt is really the world's staple-grain land; which
/// cereal grows depends on warmth/moisture/altitude, so neighbouring species meet
/// along a visible boundary (the "split").
fn grain_subtype(temp: f32, precip: f32, elev: f32) -> u8 {
    // 8 staple cereals. Each keys off a distinct temperature/moisture/altitude
    // niche, so neighbouring species meet along a visible "split" and — because
    // the niches sit in different climate bands — they naturally land on
    // different continents (cold north → rye/barley; hot wet equator → rice;
    // hot dry → sorghum/millet; temperate cores → wheat/maize/oats).
    if temp < 1.0 { 5 }                                // rye — cold humid-continental
    else if temp < 7.0 || elev > 0.50 { 4 }            // barley — cool / high altitude
    else if temp >= 22.0 && precip < 500.0 { 7 }       // sorghum — very hot & arid
    else if temp >= 20.0 && precip >= 1100.0 { 1 }     // rice — hot & wet (paddy)
    else if precip < 550.0 && temp >= 15.0 { 3 }       // millet — hot semi-arid
    else if temp < 14.0 && precip >= 900.0 { 6 }       // oats — cool & wet (oceanic)
    else if temp >= 16.0 && precip >= 700.0 { 2 }      // maize — warm & moist
    else { 0 }                                          // wheat — temperate / Mediterranean
}

/// Paper source at a cell = whichever production driver is strongest there.
/// 0 papyrus, 1 bamboo, 2 mill (manufactured), 3 parchment, 4 rice paper.
fn paper_subtype(temp: f32, precip: f32, elev: f32, hab: f32) -> u8 {
    let papyrus = if temp >= 15.0 && temp <= 30.0 && elev < 0.30 && precip >= 800.0 {
        0.40 + 0.55 * (precip / 2000.0).min(1.0)         // warm freshwater delta reed
    } else { 0.0 };
    let bamboo = if temp >= 10.0 && temp <= 24.0 && (700.0..=2400.0).contains(&precip) {
        0.58                                              // subtropical bamboo/mulberry pulp
    } else { 0.0 };
    let ricepaper = if temp >= 20.0 && precip >= 1200.0 && elev < 0.40 {
        0.55                                              // hot wet east-monsoon rice fibre
    } else { 0.0 };
    let parchment = if temp < 9.0 || elev > 0.45 {
        0.50                                              // cold/high pastoral animal hide
    } else { 0.0 };
    let mill = hab;                                       // dense-settlement rag/wood paper
    // argmax → subtype id.
    let cands = [papyrus, bamboo, mill, parchment, ricepaper];
    let mut best = 0usize;
    for i in 1..cands.len() { if cands[i] > cands[best] { best = i; } }
    best as u8
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
    let mut out: Vec<GoodRegion> = Vec::new();
    for g in 0..gc {
        let spec = match specs.get(g) {
            Some(s) if s.enabled => s,
            _ => continue, // disabled goods produce nothing to map
        };
        // Deposit goods are scattered (keep every deposit). Global goods blanket
        // the world (many patches). Local goods are one homeland (top few).
        let (max_keep, min_cells) = match spec.distribution {
            Distribution::Deposits => (32usize, 1usize),
            Distribution::Global => (14, 1),
            Distribution::Local => (4, 1),
        };
        // Multi-type goods: wheat splits into grain species, paper into its three
        // sources — classified per cell from the coarse climate fields.
        let subtype_kind = match spec.id.as_str() { "wheat" => 1u8, "paper" => 2u8, _ => 0u8 };
        let mut regions = cluster_cells(&grids[g], cw, ch, f, 0.30, min_cells);
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

// ── Trade matrix (settlement-cluster regions) ────────────────────────────────

#[derive(Serialize)]
pub struct TradeRegion {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub production: Vec<f32>, // per good, normalized 0..1 across regions
    pub demand: Vec<f32>,     // per good, 0..1
    pub net: Vec<f32>,        // production − demand
}

#[derive(Serialize)]
pub struct TradeFlow {
    pub from: u32,
    pub to: u32,
    pub good: usize,
    pub good_name: String,
    pub weight: f32,
    pub points: Vec<[f32; 2]>, // [from_center, to_center]
}

/// A bundled trunk segment of the routed trade network: one coarse edge carrying
/// the summed volume of every commodity flow that travels along it. `points` are
/// ordered **in the dominant flow direction** (source → consuming hub), so the
/// overlay can draw an arrowhead showing which way the goods are pulled. Width ∝
/// volume; `good`/`road` name the corridor by its dominant commodity ("Spice
/// Road", "Silk Road", …) for the major trunks.
#[derive(Serialize)]
pub struct TradeTrunk {
    pub points: Vec<[f32; 2]>, // [from, to] in flow direction (toward the consumer)
    pub volume: f32,
    pub good: i32,             // dominant good index, or -1
    pub road: String,          // corridor name for major trunks; "" otherwise
}

/// Evocative name for a trade corridor from its dominant commodity (the famous
/// historical routes for the iconic goods, "<Good> Road" otherwise).
fn road_name(good_id: &str) -> String {
    match good_id {
        "silk" => "Silk Road",
        "spices" => "Spice Road",
        "cloves" => "Clove Route",
        "pepper" => "Pepper Way",
        "frankincense" => "Incense Route",
        "incense" => "Incense Road",
        "salt" => "Salt Road",
        "amber" => "Amber Road",
        "tea" => "Tea Horse Road",
        "coffee" => "Coffee Trail",
        "cacao" => "Cacao Trail",
        "sugar" => "Sugar Route",
        "gold" => "Golden Road",
        "copper" => "Copper Way",
        "tin" => "Tin Route",
        "iron" => "Iron Road",
        "gemstones" => "Jewel Road",
        "pearls" => "Pearl Route",
        "furs" => "Fur Road",
        "wine" => "Wine Road",
        "oliveoil" => "Oil Route",
        "wheat" => "Grain Road",
        "timber" => "Timber Way",
        "hardwoods" => "Ebony Route",
        "horses" => "Horse Road",
        "ivory" => "Ivory Road",
        "dyes" => "Tyrian Route",
        "indigo" => "Indigo Road",
        "cotton" => "Cotton Road",
        "wool_fleece" | "wool_llama" => "Wool Way",
        "stockfish" | "whaling" => "Cod Route",
        "ceramics" => "Porcelain Road",
        "glassware" => "Glass Way",
        "tobacco" => "Tobacco Trail",
        "dates" => "Caravan Road",
        "paper" => "Paper Road",
        other => {
            let mut ch = other.chars();
            let titled = ch.next()
                .map(|f| f.to_uppercase().collect::<String>() + ch.as_str())
                .unwrap_or_default();
            return format!("{} Road", titled);
        }
    }
    .to_string()
}

#[derive(Serialize)]
pub struct TradeMatrix {
    pub regions: Vec<TradeRegion>,
    pub flows: Vec<TradeFlow>,
    pub trunks: Vec<TradeTrunk>,
    pub goods: Vec<String>,
}

/// Build a region↔region commodity-flow matrix. Settlements are clustered into
/// economic regions; each region's production of every good is summed from the
/// belt fields in its territory, demand is scaled by economic size, and net
/// surpluses are matched to deficits as flows along straight inter-region links.
#[tauri::command]
pub fn compute_trade_matrix(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    luxury_bias: f32,
    db: State<'_, WorldDb>,
) -> Result<TradeMatrix, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    // Active (editable) good specs drive names/desire/luxury so the matrix tracks
    // the world's authored goods.
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let goods_names: Vec<String> = specs.iter().map(|s| s.id.clone()).collect();
    if grid_w == 0 || grid_h == 0 {
        return Ok(TradeMatrix { regions: vec![], flows: vec![], trunks: vec![], goods: goods_names });
    }
    let world = db.cached_tiles_with_conn(&conn)?;

    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.len() < 2 {
        return Ok(TradeMatrix { regions: vec![], flows: vec![], trunks: vec![], goods: goods_names });
    }

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // ── 1. Cluster settlements into regions (greedy seeds by score, spaced) ──
    let mut sorted: Vec<&RouteSettlement> = settlements.iter().collect();
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    // Region granularity is user-controlled (default 14). More regions → tighter
    // spacing so they can all fit; fewer → wider, more spaced economic blocs.
    let er = economic_regions.clamp(2, 40) as usize;
    let min_sep = (grid_w / (er as u32 / 2 + 2)).max(1) as i32;
    let mut seeds: Vec<(i32, i32)> = Vec::new();
    for s in &sorted {
        let (sx, sy) = (s.x as i32, s.y as i32);
        let far = seeds.iter().all(|&(qx, qy)| {
            let dx = wrap_dx(sx, qx);
            let dy = sy - qy;
            ((dx * dx + dy * dy) as f32).sqrt() >= min_sep as f32
        });
        if far { seeds.push((sx, sy)); }
        if seeds.len() >= er { break; }
    }
    if seeds.is_empty() {
        return Ok(TradeMatrix { regions: vec![], flows: vec![], trunks: vec![], goods: goods_names });
    }

    // Assign every settlement to its nearest seed; accumulate weighted centers.
    let nr = seeds.len();
    let mut acc_x = vec![0.0f64; nr];
    let mut acc_y = vec![0.0f64; nr];
    let mut acc_w = vec![0.0f64; nr];
    let assign = |sx: i32, sy: i32| -> usize {
        let mut best = 0usize;
        let mut bd = i64::MAX;
        for (ri, &(qx, qy)) in seeds.iter().enumerate() {
            let dx = wrap_dx(sx, qx) as i64;
            let dy = (sy - qy) as i64;
            let d = dx * dx + dy * dy;
            if d < bd { bd = d; best = ri; }
        }
        best
    };
    for s in &settlements {
        let ri = assign(s.x as i32, s.y as i32);
        let wgt = (s.score.max(0.05)) as f64;
        // unwrap x toward the seed for a correct mean across the seam
        let mut sx = s.x as i32;
        let seedx = seeds[ri].0;
        if sx - seedx > grid_w as i32 / 2 { sx -= grid_w as i32; }
        else if seedx - sx > grid_w as i32 / 2 { sx += grid_w as i32; }
        acc_x[ri] += sx as f64 * wgt;
        acc_y[ri] += s.y as f64 * wgt;
        acc_w[ri] += wgt;
    }
    let wrap_x = |x: i32| -> i32 { ((x % grid_w as i32) + grid_w as i32) % grid_w as i32 };
    let mut centers: Vec<(i32, i32)> = Vec::with_capacity(nr);
    let mut region_weight = vec![0.0f32; nr];
    for ri in 0..nr {
        let cw_ = acc_w[ri].max(1e-6);
        let cx = wrap_x((acc_x[ri] / cw_).round() as i32);
        let cy = (acc_y[ri] / cw_).round() as i32;
        centers.push((cx, cy));
        region_weight[ri] = acc_w[ri] as f32;
    }

    // ── 2. Production: sum each good over the region's territory ──────────────
    // Coarse pass; assign each coarse cell to its nearest region center.
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut production = vec![vec![0.0f32; gc]; nr];

    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    // Sum goods into a coarse grid first (cheap), then assign coarse cells.
    let mut coarse = vec![vec![0.0f32; gc]; (cw * ch) as usize];
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
                        coarse[ci][g] += tile.goods[g][ti] as f32 / 255.0;
                    }
                }
            }
        }
    }
    let max_reach = (grid_w / 4) as i64;
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            // world coords of this coarse cell centre
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            // nearest region
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (ri, &(qx, qy)) in centers.iter().enumerate() {
                let dx = wrap_dx(wx, qx) as i64;
                let dy = (wy - qy) as i64;
                let d = dx * dx + dy * dy;
                if d < bd { bd = d; best = ri; }
            }
            if bd > max_reach * max_reach { continue; }
            for g in 0..gc {
                production[best][g] += coarse[ci][g];
            }
        }
    }

    // Per-good raw totals (absolute footprint) BEFORE normalization, so true
    // scarcity survives: a good present in only a sliver of one region must not
    // read as "plentiful" merely because it normalizes to 1.0 (#18). Deposit
    // goods (gems/metals) are spatially tiny but high-value, so they earn an
    // abundance floor to ensure they still drive trade (#19).
    let raw_total: Vec<f32> = (0..gc)
        .map(|g| production.iter().map(|p| p[g]).sum::<f32>())
        .collect();
    let raw_total_max = raw_total.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let is_deposit: Vec<bool> = (0..gc)
        .map(|g| specs.get(g)
            .map(|s| matches!(s.distribution, crate::sim::goods_spec::Distribution::Deposits))
            .unwrap_or(false))
        .collect();
    let abundance: Vec<f32> = (0..gc).map(|g| {
        let a = (raw_total[g] / raw_total_max).sqrt(); // soft scarcity curve
        if is_deposit[g] { a.max(0.6) } else { a }
    }).collect();

    // Normalize production per good across regions to 0..1 (relative pattern),
    // then rescale by global abundance so absolute scarcity carries through to net.
    for g in 0..gc {
        let mx = production.iter().map(|p| p[g]).fold(0.0f32, f32::max);
        if mx > 0.0 {
            for ri in 0..nr { production[ri][g] = production[ri][g] / mx * abundance[g]; }
        }
    }

    // ── 3. Demand: economic size × per-good base demand (from the active spec) ──
    let max_rw = region_weight.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    // Per-good desire / luxury flag come from the world's editable good spec, so
    // disabled goods demand nothing and custom desires take effect.
    let desire: Vec<f32> = (0..gc)
        .map(|g| specs.get(g).filter(|s| s.enabled).map(|s| s.desire).unwrap_or(0.0))
        .collect();
    let is_luxury: Vec<bool> = (0..gc)
        .map(|g| specs.get(g).map(|s| s.network_luxury).unwrap_or(false))
        .collect();
    // Luxuries are only prized across a large/open trade network; tighter reaches
    // (more closed networks) realize less of that demand.
    let reach_factor = match reach { 0 => 1.0, 1 => 0.7, _ => 0.45 };
    // Mercantile ↔ subsistence bias (#4): >0.5 prizes distant luxuries
    // (silk/spices), <0.5 makes the world care mostly about staples. Neutral 0.5
    // leaves the legacy behaviour (lux_mult == 1.0). Staples are unaffected.
    let lux_mult = (0.4 + luxury_bias.clamp(0.0, 1.0) * 1.2).clamp(0.2, 1.8);
    let mut demand = vec![vec![0.0f32; gc]; nr];
    for ri in 0..nr {
        let size = region_weight[ri] / max_rw; // 0..1
        for g in 0..gc {
            let mut d = size * desire[g];
            if is_luxury[g] {
                // Discount in the good's own producing homeland (it's local/common
                // there) and scale by how open the trade network is + the world's
                // taste for luxuries.
                let homeland_discount = 1.0 - 0.6 * production[ri][g].clamp(0.0, 1.0);
                d *= reach_factor * homeland_discount * lux_mult;
            }
            demand[ri][g] = d;
        }
    }

    // Net = production − demand.
    let mut regions: Vec<TradeRegion> = Vec::with_capacity(nr);
    for ri in 0..nr {
        let net: Vec<f32> = (0..gc).map(|g| production[ri][g] - demand[ri][g]).collect();
        regions.push(TradeRegion {
            id: ri as u32,
            name: format!("Region {}", ri + 1),
            x: centers[ri].0 as f32,
            y: centers[ri].1 as f32,
            production: production[ri].clone(),
            demand: demand[ri].clone(),
            net,
        });
    }

    // ── 4. Flows: match surpluses to deficits per good, routed over the trade
    // network and bundled into trunks. A supplier can only serve an importer if
    // a route between their regions exists under the chosen trade reach (so when
    // continents are too far / the ocean too wide, trade stays within reach and
    // no flow is drawn). Flows that share a corridor stack onto the same trunk.
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes)?;
    let region_node: Vec<usize> = centers.iter().map(|&(x, y)| {
        let cx = (x.max(0) as u32 / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (y.max(0) as u32 / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();

    // Lazily-computed least-cost path per region pair (None = unreachable).
    let mut pair_path: std::collections::HashMap<(usize, usize), Option<Vec<usize>>> =
        std::collections::HashMap::new();
    // Per coarse-edge accumulation for the bundled trunks: total volume, volume
    // by good (to name the corridor by its dominant commodity), and directional
    // volume (which way the goods are pulled — toward the consuming hub).
    #[derive(Default)]
    struct EdgeAcc {
        total: f32,
        fwd: f32, // volume flowing min→max node
        bwd: f32, // volume flowing max→min node
        by_good: std::collections::HashMap<usize, f32>,
    }
    let mut edge_acc: std::collections::HashMap<(usize, usize), EdgeAcc> =
        std::collections::HashMap::new();

    let mut flows: Vec<TradeFlow> = Vec::new();
    for g in 0..gc {
        let mut supply: Vec<(usize, f32)> = (0..nr)
            .filter_map(|ri| { let n = regions[ri].net[g]; if n > 0.05 { Some((ri, n)) } else { None } })
            .collect();
        let mut deficit: Vec<(usize, f32)> = (0..nr)
            .filter_map(|ri| { let n = regions[ri].net[g]; if n < -0.05 { Some((ri, -n)) } else { None } })
            .collect();
        if supply.is_empty() || deficit.is_empty() { continue; }
        deficit.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for &mut (di, mut need) in deficit.iter_mut() {
            let (dx0, dy0) = centers[di];
            supply.sort_by_key(|&(si, _)| {
                let (sx, sy) = centers[si];
                let dx = wrap_dx(sx, dx0) as i64;
                let dy = (sy - dy0) as i64;
                dx * dx + dy * dy
            });
            for s in supply.iter_mut() {
                if need <= 0.05 { break; }
                if s.1 <= 0.05 { continue; }
                if s.0 == di { continue; }
                let key = (s.0.min(di), s.0.max(di));
                if !pair_path.contains_key(&key) {
                    // Trade flows must NEVER cross open ocean: reject any routed
                    // path that touches an open-water (no-land-neighbour) cell.
                    // Coastal-sea hugging is still allowed (is_open_sea is false
                    // there), so island/coastal trade via short coastal hops works,
                    // but no trunk beelines across a basin between continents.
                    let p = coarse_dijkstra(&cc, region_node[s.0], region_node[di])
                        .filter(|p| path_allowed(&cc, p, reach, max_crossing, grid_w))
                        .filter(|p| !p.iter().any(|&c| cc.is_open_sea[c]));
                    pair_path.insert(key, p);
                }
                if pair_path[&key].is_none() { continue; } // unreachable under this reach
                let amt = need.min(s.1);
                s.1 -= amt;
                need -= amt;
                // Accumulate this flow onto every coarse edge of its routed path,
                // in the consumer-ward direction (supply s.0 → deficit di).
                if let Some(Some(path)) = pair_path.get(&key) {
                    let forward = path.first() == Some(&region_node[s.0]);
                    for w in path.windows(2) {
                        let (from, to) = if forward { (w[0], w[1]) } else { (w[1], w[0]) };
                        let e = (from.min(to), from.max(to));
                        let acc = edge_acc.entry(e).or_default();
                        acc.total += amt;
                        if from < to { acc.fwd += amt; } else { acc.bwd += amt; }
                        *acc.by_good.entry(g).or_insert(0.0) += amt;
                    }
                }
                let (sx, sy) = centers[s.0];
                flows.push(TradeFlow {
                    from: s.0 as u32,
                    to: di as u32,
                    good: g,
                    good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    weight: amt,
                    points: vec![[sx as f32, sy as f32], [dx0 as f32, dy0 as f32]],
                });
            }
        }
    }

    // Build bundled trunks from the per-edge accumulation: width ∝ volume, points
    // ordered in the dominant flow direction (source→consumer), dominant good, and
    // a corridor name for the major arteries.
    let mut edges_v: Vec<((usize, usize), EdgeAcc)> = edge_acc.into_iter().collect();
    edges_v.sort_by(|a, b| b.1.total.partial_cmp(&a.1.total).unwrap_or(std::cmp::Ordering::Equal));
    let max_total = edges_v.first().map(|e| e.1.total).unwrap_or(1.0).max(1e-6);
    let mut trunks: Vec<TradeTrunk> = Vec::new();
    // Name ONE corridor per dominant commodity (its single highest-volume edge,
    // since edges are volume-sorted), so many goods get a named road without the
    // same name repeating along every segment of a corridor.
    let mut named_goods: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for ((a, b), acc) in edges_v.into_iter() {
        if acc.total < 0.02 { continue; }
        let good = acc.by_good.iter()
            .max_by(|x, y| x.1.partial_cmp(y.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(g, _)| *g);
        let (p_from, p_to) = if acc.fwd >= acc.bwd { (a, b) } else { (b, a) };
        let road = match good {
            Some(g) if acc.total >= 0.06 * max_total && named_goods.insert(g) => {
                goods_names.get(g).map(|id| road_name(id)).unwrap_or_default()
            }
            _ => String::new(),
        };
        trunks.push(TradeTrunk {
            points: vec![cc.world_of(p_from), cc.world_of(p_to)],
            volume: acc.total,
            good: good.map(|g| g as i32).unwrap_or(-1),
            road,
        });
    }

    Ok(TradeMatrix { regions, flows, trunks, goods: goods_names })
}

// ─────────────────────────────────────────────────────────────────────────────
// Political layer — settlement trade power + influence
// ─────────────────────────────────────────────────────────────────────────────

/// One politically significant settlement: re-ranked by trade power (route
/// centrality + good monopoly, on top of base habitability), with an influence
/// radius scaled by that power and the goods it monopolizes.
#[derive(Serialize)]
pub struct PoliticalCenter {
    pub x: f32,
    pub y: f32,
    pub power: f32,              // 0..1 combined trade power
    pub rank: u32,              // 0 = most powerful
    pub radius: f32,            // (legacy) influence radius in world cells
    pub stars: u8,              // power tier 1..5 — major trade hubs get 5 (Venice/Genoa)
    pub population: u32,
    pub monopolies: Vec<String>, // goods this settlement dominates production of
    pub name: String,           // generated hub name
    pub throughput: f32,        // total goods throughput handled (relative units)
    pub ref_pct: f32,           // throughput as % of the strongest hub (≈ a 1450 Venice)
    pub nearest_hub: String,    // closest-matching real 1450 trade hub
    pub top_goods: Vec<HubGood>, // dominant traded goods + amounts (for the click panel)
}

#[derive(Serialize)]
pub struct HubGood {
    pub name: String,
    pub amount: f32,
}

/// Real major trade hubs of ~1450 CE with relative throughput weights (Venice =
/// 100), used to give each generated hub a "≈ X% of Venice / closest to <hub>"
/// comparison.
const REF_HUBS_1450: [(&str, f32); 14] = [
    ("Venice", 100.0), ("Guangzhou", 95.0), ("Constantinople", 88.0), ("Calicut", 82.0),
    ("Malacca", 78.0), ("Alexandria", 72.0), ("Cairo", 68.0), ("Bruges", 64.0),
    ("Hangzhou", 60.0), ("Hormuz", 52.0), ("Samarkand", 46.0), ("Novgorod", 40.0),
    ("Timbuktu", 36.0), ("Kilwa", 30.0),
];

/// Closest 1450 hub to a throughput percentage (of the world's top hub).
fn nearest_ref_hub(pct: f32) -> &'static str {
    REF_HUBS_1450.iter()
        .min_by(|a, b| (a.1 - pct).abs().partial_cmp(&(b.1 - pct).abs()).unwrap_or(std::cmp::Ordering::Equal))
        .map(|h| h.0)
        .unwrap_or("Venice")
}

/// Re-rank settlements by trade power and emit influence centers. Power blends
/// base habitability (settlement score), route centrality (how many trade links
/// a settlement anchors under the chosen reach) and trade monopoly (being the
/// dominant producer of goods — strongest for the seeded one-homeland goods).
/// The frontend draws translucent influence discs sized by power.
#[tauri::command]
pub fn compute_political(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    db: State<'_, WorldDb>,
) -> Result<Vec<PoliticalCenter>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;

    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.is_empty() { return Ok(vec![]); }

    // Top settlements by base score (the political contenders). Both the
    // contender pool and the emitted-hub count scale with economic granularity
    // (default 14 → 84 contenders / 42 hubs, matching the legacy 80 / 40).
    let er = economic_regions.clamp(2, 40) as usize;
    let contenders = (er * 6).clamp(10, 200);
    let hub_out = (er * 3).clamp(5, 120);
    let mut idx: Vec<usize> = (0..settlements.len()).collect();
    idx.sort_by(|&a, &b| settlements[b].score.partial_cmp(&settlements[a].score)
        .unwrap_or(std::cmp::Ordering::Equal));
    idx.truncate(contenders);
    let nn = idx.len();
    let nodes: Vec<&RouteSettlement> = idx.iter().map(|&i| &settlements[i]).collect();

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // Route centrality: count reachable nearest-neighbour links per node.
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes)?;
    let cnode: Vec<usize> = nodes.iter().map(|s| {
        let cx = (s.x / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (s.y / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();
    let mut edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for i in 0..nn {
        let mut dists: Vec<(usize, i64)> = Vec::new();
        for j in 0..nn {
            if i == j { continue; }
            let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
            let dy = nodes[i].y as i64 - nodes[j].y as i64;
            dists.push((j, dx * dx + dy * dy));
        }
        dists.sort_by_key(|&(_, d)| d);
        for &(j, _) in dists.iter().take(3) {
            edges.insert((i.min(j), i.max(j)));
        }
    }
    let mut centrality = vec![0.0f32; nn];
    for &(a, b) in &edges {
        if let Some(path) = coarse_dijkstra(&cc, cnode[a], cnode[b]) {
            if path_allowed(&cc, &path, reach, max_crossing, grid_w) {
                centrality[a] += 1.0;
                centrality[b] += 1.0;
            }
        }
    }

    // Monopoly: assign each good-bearing coarse cell to its nearest node, then
    // measure how large a share of each goods total production a node holds.
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let mut prod = vec![vec![0.0f32; gc]; nn];
    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    let mut coarse = vec![vec![0.0f32; gc]; (cw * ch) as usize];
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
                        coarse[ci][g] += tile.goods[g][ti] as f32 / 255.0;
                    }
                }
            }
        }
    }
    let max_reach = (grid_w / 4) as i64;
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (ni, s) in nodes.iter().enumerate() {
                let dx = wrap_dx(wx, s.x as i32) as i64;
                let dy = (wy - s.y as i32) as i64;
                let d = dx * dx + dy * dy;
                if d < bd { bd = d; best = ni; }
            }
            if bd > max_reach * max_reach { continue; }
            for g in 0..gc {
                prod[best][g] += coarse[ci][g];
            }
        }
    }
    let mut good_total = vec![0.0f32; gc];
    for g in 0..gc {
        for ni in 0..nn { good_total[g] += prod[ni][g]; }
    }
    let mut monopoly = vec![0.0f32; nn];
    let mut monopolies: Vec<Vec<String>> = vec![Vec::new(); nn];
    for ni in 0..nn {
        for g in 0..gc {
            if good_total[g] < 1e-4 || prod[ni][g] < 0.05 { continue; }
            let share = prod[ni][g] / good_total[g];
            monopoly[ni] += share * share; // squared → rewards true dominance
            if share > 0.55 {
                monopolies[ni].push(specs[g].id.clone());
            }
        }
    }

    // Combine into trade power and normalize.
    let cmax = centrality.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mmax = monopoly.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mut power = vec![0.0f32; nn];
    for ni in 0..nn {
        power[ni] = 0.45 * nodes[ni].score.clamp(0.0, 1.0)
            + 0.30 * (centrality[ni] / cmax)
            + 0.25 * (monopoly[ni] / mmax);
    }
    let pmax = power.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    let mut order: Vec<usize> = (0..nn).collect();
    order.sort_by(|&a, &b| power[b].partial_cmp(&power[a]).unwrap_or(std::cmp::Ordering::Equal));

    // Throughput: total goods handled in the hub's hinterland, lifted by how many
    // trade links it anchors (a central node moves more than its own production).
    let mut throughput = vec![0.0f32; nn];
    for ni in 0..nn {
        let prod_sum: f32 = prod[ni].iter().sum();
        throughput[ni] = prod_sum * (1.0 + 0.5 * centrality[ni] / cmax);
    }
    let tp_max = throughput.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    let base_r = grid_w as f32 * 0.018;
    let span_r = grid_w as f32 * 0.075;
    let mut out: Vec<PoliticalCenter> = Vec::new();
    for (rank, &ni) in order.iter().enumerate().take(hub_out) {
        let p = power[ni] / pmax;
        // Power tier → 1..5 stars; only the dominant hubs reach 5 (Venice/Genoa).
        let stars = if p >= 0.80 { 5 } else if p >= 0.60 { 4 }
            else if p >= 0.42 { 3 } else if p >= 0.25 { 2 } else { 1 };
        let ref_pct = (throughput[ni] / tp_max * 100.0).clamp(0.0, 100.0);
        let mut gv: Vec<(usize, f32)> = (0..gc)
            .map(|g| (g, prod[ni][g]))
            .filter(|&(_, a)| a > 0.01)
            .collect();
        gv.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        gv.truncate(6);
        let top_goods: Vec<HubGood> = gv.into_iter()
            .map(|(g, a)| HubGood { name: specs[g].id.clone(), amount: a })
            .collect();
        out.push(PoliticalCenter {
            x: nodes[ni].x as f32,
            y: nodes[ni].y as f32,
            power: p,
            rank: rank as u32,
            radius: base_r + span_r * p,
            stars,
            population: nodes[ni].population,
            monopolies: monopolies[ni].clone(),
            name: crate::sim::names::gen_name_epithet(
                nodes[ni].x, nodes[ni].y, grid_w, grid_h,
                if stars >= 5 { 2 } else if stars >= 4 { 1 } else { 0 },
            ),
            throughput: throughput[ni],
            ref_pct,
            nearest_hub: nearest_ref_hub(ref_pct).to_string(),
            top_goods,
        });
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// Economy snapshot (Phase 2) — hub-anchored production, quality grades,
// cost-aware flows with per-hop prices, wealth, and strategic chokepoints.
// Built once and persisted as JSON in metadata (key "economy"); read instantly
// when a hub is clicked. Hubs ARE the political/economic centers (one shared set,
// unifying the matrix + political views).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct EconHubGood {
    pub good: usize,
    pub good_name: String,
    pub amount: f32,
    pub quality: f32,   // 0..1 grade
    pub grade: String,  // grade label (Exquisite/Fine/Standard/Common/Coarse)
    pub price: f32,     // local origin price multiplier (× base)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconReceive {
    pub good: usize,
    pub good_name: String,
    pub amount: f32,
    pub price: f32,     // delivered price multiplier at this hub
    pub chain: u32,     // index into EconomySnapshot.chains
    pub from_hub: u32,  // origin hub id
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconHub {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub power: f32,
    pub stars: u8,
    pub wealth: f32,        // 0..1 relative trade wealth
    pub population: u32,
    pub produces: Vec<EconHubGood>,
    pub receives: Vec<EconReceive>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconChainStop {
    pub hub: u32,
    pub price: f32, // price multiplier when the good reaches this stop
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconChain {
    pub id: u32,
    pub good: usize,
    pub good_name: String,
    pub stops: Vec<EconChainStop>, // origin → … intermediate hubs … → consumer
    pub points: Vec<[f32; 2]>,     // world coords of each stop (for the overlay)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconChokepoint {
    pub points: Vec<[f32; 2]>, // the coarse edge [from, to]
    pub volume: f32,
    pub share: f32,            // share of total routed volume
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconomySnapshot {
    pub hubs: Vec<EconHub>,
    pub chains: Vec<EconChain>,
    pub chokepoints: Vec<EconChokepoint>,
    pub goods: Vec<String>,
}

fn grade_name(q: f32) -> &'static str {
    if q >= 0.85 { "Exquisite" }
    else if q >= 0.68 { "Fine" }
    else if q >= 0.50 { "Standard" }
    else if q >= 0.32 { "Common" }
    else { "Coarse" }
}

/// splitmix-style hash → [0,1).
fn hash01q(mut x: u64) -> f32 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    ((x >> 40) as f32) / 16_777_216.0
}

/// Transport cost of a coarse path (sum of midpoint cell costs along its edges).
fn coarse_path_cost(cc: &CoarseCost, path: &[usize]) -> f32 {
    let mut c = 0.0;
    for w in path.windows(2) { c += (cc.cost[w[0]] + cc.cost[w[1]]) * 0.5; }
    c
}

/// Build the economy snapshot, persist it to metadata, and return it. Same
/// inputs as the matrix/political commands so the hubs stay consistent.
#[tauri::command]
pub fn compute_economy(
    settlements_json: String,
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    desert_routes: bool,
    economic_regions: u32,
    luxury_bias: f32,
    db: State<'_, WorldDb>,
) -> Result<EconomySnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let specs = crate::commands::goods_commands::load_world_goods(&conn);
    let gc = specs.len();
    let goods_names: Vec<String> = specs.iter().map(|s| s.id.clone()).collect();
    let empty = EconomySnapshot { hubs: vec![], chains: vec![], chokepoints: vec![], goods: goods_names.clone() };
    if grid_w == 0 || grid_h == 0 {
        return Ok(empty);
    }
    let world = db.cached_tiles_with_conn(&conn)?;
    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.len() < 2 {
        let _ = metadata::set_meta(&conn, "economy", &serde_json::to_string(&empty).unwrap_or_default());
        return Ok(empty);
    }

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // ── Hub set: top settlements by score (the clickable economic centers) ──
    let er = economic_regions.clamp(2, 40) as usize;
    let hub_out = (er * 3).clamp(5, 60);
    let mut idx: Vec<usize> = (0..settlements.len()).collect();
    idx.sort_by(|&a, &b| settlements[b].score.partial_cmp(&settlements[a].score)
        .unwrap_or(std::cmp::Ordering::Equal));
    idx.truncate(hub_out);
    let nodes: Vec<&RouteSettlement> = idx.iter().map(|&i| &settlements[i]).collect();
    let nn = nodes.len();
    if nn < 2 {
        let _ = metadata::set_meta(&conn, "economy", &serde_json::to_string(&empty).unwrap_or_default());
        return Ok(empty);
    }

    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes)?;
    let cnode: Vec<usize> = nodes.iter().map(|s| {
        let cx = (s.x / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (s.y / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();

    // ── Route centrality: reachable nearest-neighbour links per hub ──
    let mut edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for i in 0..nn {
        let mut dists: Vec<(usize, i64)> = Vec::new();
        for j in 0..nn {
            if i == j { continue; }
            let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
            let dy = nodes[i].y as i64 - nodes[j].y as i64;
            dists.push((j, dx * dx + dy * dy));
        }
        dists.sort_by_key(|&(_, d)| d);
        for &(j, _) in dists.iter().take(3) { edges.insert((i.min(j), i.max(j))); }
    }
    let mut centrality = vec![0.0f32; nn];
    for &(a, b) in &edges {
        if let Some(path) = coarse_dijkstra(&cc, cnode[a], cnode[b]) {
            if path_allowed(&cc, &path, reach, max_crossing, grid_w) {
                centrality[a] += 1.0;
                centrality[b] += 1.0;
            }
        }
    }

    // ── Production: assign each good-bearing coarse cell to its nearest hub ──
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let tiles_x = (grid_w + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_h + TILE_SIZE - 1) / TILE_SIZE;
    let mut coarse = vec![vec![0.0f32; gc]; (cw * ch) as usize];
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
                        coarse[ci][g] += tile.goods[g][ti] as f32 / 255.0;
                    }
                }
            }
        }
    }
    let max_reach = (grid_w / 4) as i64;
    let mut prod = vec![vec![0.0f32; gc]; nn];
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (ni, s) in nodes.iter().enumerate() {
                let dx = wrap_dx(wx, s.x as i32) as i64;
                let dy = (wy - s.y as i32) as i64;
                let d = dx * dx + dy * dy;
                if d < bd { bd = d; best = ni; }
            }
            if bd > max_reach * max_reach { continue; }
            for g in 0..gc { prod[best][g] += coarse[ci][g]; }
        }
    }

    // Absolute scarcity (#18) + deposit floor (#19): rescale by global abundance.
    let raw_total: Vec<f32> = (0..gc).map(|g| prod.iter().map(|p| p[g]).sum::<f32>()).collect();
    let raw_total_max = raw_total.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let is_deposit: Vec<bool> = (0..gc)
        .map(|g| specs.get(g)
            .map(|s| matches!(s.distribution, crate::sim::goods_spec::Distribution::Deposits))
            .unwrap_or(false))
        .collect();
    let abundance: Vec<f32> = (0..gc).map(|g| {
        let a = (raw_total[g] / raw_total_max).sqrt();
        if is_deposit[g] { a.max(0.6) } else { a }
    }).collect();
    let good_max: Vec<f32> = (0..gc).map(|g| prod.iter().map(|p| p[g]).fold(0.0f32, f32::max).max(1e-6)).collect();
    // quality grade per hub/good: richer territory share → higher grade, with a
    // small deterministic jitter so equal shares still differ between hubs/goods.
    let mut quality = vec![vec![0.0f32; gc]; nn];
    for hh in 0..nn {
        for g in 0..gc {
            let share = prod[hh][g] / good_max[g];
            let jitter = hash01q((hh as u64).wrapping_mul(0x9E3779B1) ^ (g as u64).wrapping_mul(0x85EBCA77)) * 0.24 - 0.12;
            quality[hh][g] = (0.30 + 0.62 * share + jitter).clamp(0.0, 1.0);
            // apply abundance scaling to production AFTER quality is read off share
            prod[hh][g] = prod[hh][g] / good_max[g] * abundance[g];
        }
    }

    // ── Monopoly + trade power + stars ──
    let mut good_total = vec![0.0f32; gc];
    for g in 0..gc { for hh in 0..nn { good_total[g] += prod[hh][g]; } }
    let mut monopoly = vec![0.0f32; nn];
    for hh in 0..nn {
        for g in 0..gc {
            if good_total[g] < 1e-4 || prod[hh][g] < 0.05 { continue; }
            let share = prod[hh][g] / good_total[g];
            monopoly[hh] += share * share;
        }
    }
    let cmax = centrality.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mmax = monopoly.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mut power = vec![0.0f32; nn];
    for hh in 0..nn {
        power[hh] = 0.45 * nodes[hh].score.clamp(0.0, 1.0)
            + 0.30 * (centrality[hh] / cmax)
            + 0.25 * (monopoly[hh] / mmax);
    }
    let pmax = power.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // ── Demand: population-scaled per-good desire (+ mercantile/subsistence) ──
    let max_pop = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    let desire: Vec<f32> = (0..gc)
        .map(|g| specs.get(g).filter(|s| s.enabled).map(|s| s.desire).unwrap_or(0.0))
        .collect();
    let is_luxury: Vec<bool> = (0..gc)
        .map(|g| specs.get(g).map(|s| s.network_luxury).unwrap_or(false))
        .collect();
    let reach_factor = match reach { 0 => 1.0, 1 => 0.7, _ => 0.45 };
    let lux_mult = (0.4 + luxury_bias.clamp(0.0, 1.0) * 1.2).clamp(0.2, 1.8);
    let mut net = vec![vec![0.0f32; gc]; nn];
    for hh in 0..nn {
        let size = (nodes[hh].population as f32 / max_pop).max(0.15);
        for g in 0..gc {
            let mut d = size * desire[g];
            if is_luxury[g] {
                let homeland_discount = 1.0 - 0.6 * prod[hh][g].clamp(0.0, 1.0);
                d *= reach_factor * homeland_discount * lux_mult;
            }
            net[hh][g] = prod[hh][g] - d;
        }
    }

    // ── Flows (cost-aware) → chains with per-hop prices + chokepoint volumes ──
    let hubnode: std::collections::HashMap<usize, usize> =
        (0..nn).map(|h| (cnode[h], h)).collect();
    let mut pair: std::collections::HashMap<(usize, usize), Option<(Vec<usize>, f32)>> =
        std::collections::HashMap::new();
    let mut chains: Vec<EconChain> = Vec::new();
    let mut receives: Vec<Vec<EconReceive>> = vec![Vec::new(); nn];
    let mut exports = vec![0.0f32; nn];
    let mut imports = vec![0.0f32; nn];
    let mut edge_vol: std::collections::HashMap<(usize, usize), (f32, usize)> =
        std::collections::HashMap::new(); // volume + dominant good

    for g in 0..gc {
        let mut supply: Vec<(usize, f32)> = (0..nn)
            .filter_map(|h| { let v = net[h][g]; if v > 0.05 { Some((h, v)) } else { None } }).collect();
        let mut deficit: Vec<(usize, f32)> = (0..nn)
            .filter_map(|h| { let v = net[h][g]; if v < -0.05 { Some((h, -v)) } else { None } }).collect();
        if supply.is_empty() || deficit.is_empty() { continue; }
        deficit.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for &mut (di, mut need) in deficit.iter_mut() {
            let need0 = need;
            // Order suppliers by ROUTE COST (cheapest corridor wins) — cost-aware.
            let mut ord: Vec<(usize, f32, f32)> = Vec::new(); // (supplier, avail, cost)
            for &(si, avail) in supply.iter() {
                if si == di || avail <= 0.05 { continue; }
                let key = (si.min(di), si.max(di));
                if !pair.contains_key(&key) {
                    let p = coarse_dijkstra(&cc, cnode[si], cnode[di])
                        .filter(|p| path_allowed(&cc, p, reach, max_crossing, grid_w))
                        .filter(|p| !p.iter().any(|&c| cc.is_open_sea[c]))
                        .map(|p| { let c = coarse_path_cost(&cc, &p); (p, c) });
                    pair.insert(key, p);
                }
                if let Some(Some((_, cost))) = pair.get(&key) { ord.push((si, avail, *cost)); }
            }
            ord.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

            for (si, _avail, _cost) in ord {
                if need <= 0.05 { break; }
                let pos = match supply.iter().position(|&(s, _)| s == si) { Some(p) => p, None => continue };
                let savail = supply[pos].1;
                if savail <= 0.05 { continue; }
                let key = (si.min(di), si.max(di));
                let (path, _cost) = match pair.get(&key) { Some(Some(pc)) => pc.clone(), _ => continue };
                let amt = need.min(savail);
                need -= amt;
                supply[pos].1 -= amt;
                exports[si] += amt;
                imports[di] += amt;

                // Orient origin(si) → consumer(di) and find intermediate hub stops.
                let path_o = if path.first() == Some(&cnode[si]) { path.clone() }
                    else { let mut p = path.clone(); p.reverse(); p };
                let mut stops: Vec<(usize, f32)> = vec![(si, 0.0)];
                let mut cum = 0.0;
                for win in path_o.windows(2) {
                    cum += (cc.cost[win[0]] + cc.cost[win[1]]) * 0.5;
                    if let Some(&h) = hubnode.get(&win[1]) {
                        if h != si && h != di && stops.last().map(|l| l.0) != Some(h) {
                            stops.push((h, cum));
                        }
                    }
                    // chokepoint volume per coarse edge
                    let e = (win[0].min(win[1]), win[0].max(win[1]));
                    let entry = edge_vol.entry(e).or_insert((0.0, g));
                    entry.0 += amt;
                }
                stops.push((di, cum));

                let q0 = quality[si][g];
                let base_q = 0.7 + 0.6 * q0;
                let scarcity = need0.clamp(0.0, 1.5);
                let ns = stops.len();
                let chain_id = chains.len() as u32;
                let chain_stops: Vec<EconChainStop> = stops.iter().enumerate().map(|(k, &(h, cu))| {
                    let progress = if ns > 1 { k as f32 / (ns - 1) as f32 } else { 0.0 };
                    let transport = (cu / cc.cw as f32) * 1.5;
                    let price = base_q * (1.0 + transport) * (1.0 + scarcity * 0.8 * progress);
                    EconChainStop { hub: h as u32, price }
                }).collect();
                let delivered = chain_stops.last().map(|s| s.price).unwrap_or(base_q);
                let points: Vec<[f32; 2]> = stops.iter()
                    .map(|&(h, _)| [nodes[h].x as f32, nodes[h].y as f32]).collect();
                receives[di].push(EconReceive {
                    good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    amount: amt, price: delivered, chain: chain_id, from_hub: si as u32,
                });
                chains.push(EconChain {
                    id: chain_id, good: g,
                    good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    stops: chain_stops, points,
                });
            }
        }
    }

    // ── Wealth per hub (production value + trade throughput balance) ──
    let mut wealth = vec![0.0f32; nn];
    for hh in 0..nn {
        let prod_val: f32 = (0..gc).map(|g| prod[hh][g] * desire[g]).sum();
        wealth[hh] = prod_val + 0.6 * exports[hh] + 0.3 * imports[hh] + 0.4 * (centrality[hh] / cmax);
    }
    let wmax = wealth.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // ── Strategic chokepoints: highest-volume coarse edges ──
    let total_vol: f32 = edge_vol.values().map(|v| v.0).sum::<f32>().max(1e-6);
    let mut ev: Vec<((usize, usize), (f32, usize))> = edge_vol.into_iter().collect();
    ev.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));
    let max_edge = ev.first().map(|e| e.1.0).unwrap_or(1.0).max(1e-6);
    let mut chokepoints: Vec<EconChokepoint> = Vec::new();
    for ((a, b), (vol, gdom)) in ev.into_iter().take(10) {
        if vol < 0.25 * max_edge { break; }
        let oa = cc.is_open_sea.get(a).copied().unwrap_or(false);
        let ob = cc.is_open_sea.get(b).copied().unwrap_or(false);
        let kind = if oa || ob { "Strait" }
            else if !cc.is_land.get(a).copied().unwrap_or(true) || !cc.is_land.get(b).copied().unwrap_or(true) { "Passage" }
            else { "Pass" };
        let road = goods_names.get(gdom).map(|id| road_name(id)).unwrap_or_default();
        let nm = if road.is_empty() { kind.to_string() } else { format!("{} {}", road.replace(" Road", "").replace(" Route", "").replace(" Way", ""), kind) };
        chokepoints.push(EconChokepoint {
            points: vec![cc.world_of(a), cc.world_of(b)],
            volume: vol,
            share: vol / total_vol,
            name: nm,
        });
    }

    // ── Emit hubs ──
    let mut hubs: Vec<EconHub> = Vec::with_capacity(nn);
    for hh in 0..nn {
        let p = power[hh] / pmax;
        let stars = if p >= 0.80 { 5 } else if p >= 0.60 { 4 }
            else if p >= 0.42 { 3 } else if p >= 0.25 { 2 } else { 1 };
        let mut produces: Vec<EconHubGood> = (0..gc)
            .filter(|&g| prod[hh][g] > 0.05)
            .map(|g| {
                let q = quality[hh][g];
                EconHubGood {
                    good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    amount: prod[hh][g], quality: q, grade: grade_name(q).to_string(),
                    price: 0.7 + 0.6 * q,
                }
            }).collect();
        produces.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        let mut recv = receives[hh].clone();
        recv.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        hubs.push(EconHub {
            id: hh as u32,
            x: nodes[hh].x as f32,
            y: nodes[hh].y as f32,
            name: crate::sim::names::gen_name_epithet(
                nodes[hh].x, nodes[hh].y, grid_w, grid_h,
                if stars >= 5 { 2 } else if stars >= 4 { 1 } else { 0 }),
            power: p,
            stars,
            wealth: wealth[hh] / wmax,
            population: nodes[hh].population,
            produces,
            receives: recv,
        });
    }

    let snapshot = EconomySnapshot { hubs, chains, chokepoints, goods: goods_names };
    let _ = metadata::set_meta(&conn, "economy", &serde_json::to_string(&snapshot).unwrap_or_default());
    Ok(snapshot)
}

/// Read the persisted economy snapshot (empty if none has been generated yet).
#[tauri::command]
pub fn get_economy(db: State<'_, WorldDb>) -> Result<EconomySnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let goods_names: Vec<String> = crate::commands::goods_commands::load_world_goods(&conn)
        .iter().map(|s| s.id.clone()).collect();
    match metadata::get_meta(&conn, "economy").map_err(|e| e.to_string())? {
        Some(json) => serde_json::from_str(&json)
            .map_err(|_| ()).or_else(|_| Ok::<_, String>(EconomySnapshot {
                hubs: vec![], chains: vec![], chokepoints: vec![], goods: goods_names.clone(),
            })),
        None => Ok(EconomySnapshot { hubs: vec![], chains: vec![], chokepoints: vec![], goods: goods_names }),
    }
}
