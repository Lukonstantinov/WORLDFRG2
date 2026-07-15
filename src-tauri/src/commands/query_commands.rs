use serde::{Serialize, Deserialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tauri::State;
use crate::db::{WorldDb, CostKey, tile_store, metadata};
use crate::db::world_cache::WorldTiles;
use crate::tile::coords::{TileCoord, TILE_SIZE};
use crate::tile::cell::TileData;
use crate::sim::biological::GOOD_NAMES;
use crate::sim::market;

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
    pub disease_risk: f32, // 0..1 (malaria/fever, land)
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
    /// "capital" | "city" | "town" | "village" | "outpost". Used to keep tiny
    /// trade outposts out of the hub ranking and tag them in the class stats.
    #[serde(default)]
    size: String,
}

/// A river, as sent from the frontend store (only the cell path is needed).
#[derive(Deserialize)]
struct RouteRiver {
    points: Vec<(u32, u32)>,
    /// Navigable trunk (barge/boat highway). Old callers that send only `points`
    /// default to false → treated as a minor river (still a cheap corridor).
    #[serde(default)]
    navigable: bool,
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
    elev: Vec<f32>,         // normalized elevation (relief → caravan slowdown)
    sea_hazard: Vec<f32>,   // storm/reef exposure (ship slowdown)
}

const SEA_BLOCK_COST: f32 = 1.0e6;
/// Cost of an open-water (no land neighbour) coarse cell. Higher than the
/// coastal-sea cost (0.5) so least-cost routes/flows still hug the coast and cross
/// open ocean at the narrowest point — but low enough that a sea crossing is
/// genuinely PREFERRED over a long overland detour, and separate landmasses get
/// wired into one trade network whenever a reasonable crossing exists (the user
/// wants sea paths preferred and routes connected where possible). Was 3.5.
const OPEN_SEA_COST: f32 = 2.2;
/// Seawater freezes near −2 °C; a sea cell at or below this carries pack/sea ice
/// for much of the year and is impassable to ships (no route crosses it).
const SEA_FREEZE_C: f32 = -2.0;
/// Land this cold (deep ice-sheet / EF ice-cap interior) is near-impassable to
/// caravans — crossed only if there is no alternative at all.
const ICE_LAND_C: f32 = -8.0;
/// Near-blocking cost for ice-sheet land (huge, but below `SEA_BLOCK_COST` so a
/// route with literally no other option can still thread it).
const ICE_LAND_COST: f32 = 5.0e4;

// ── Physical travel scale ───────────────────────────────────────────────────
// The sim treats the map width as one Earth circumference (mirrors
// `sim/precipitation.rs::EARTH_CIRCUMFERENCE_KM`), so 1 fine cell ≈
// 40075 / grid_width km. Travel time on a routed leg = km ÷ mode speed, with a
// terrain multiplier (relief slows caravans; storm/reef hazard slows ships).
const KM_EQUATOR: f32 = 40075.0;
const SPEED_CARAVAN_KMD: f32 = 30.0;  // overland pack caravan
const SPEED_SHIP_KMD: f32 = 120.0;    // coastal sailing ship
const SPEED_RIVER_KMD: f32 = 40.0;    // river barge / boat
// Per-traveller land speeds for the itinerary calculator (water legs always go by
// boat/ship at SPEED_RIVER/SPEED_SHIP). A lone traveller on foot makes better time
// than a laden pack caravan; a post-horse better still; an ox/mule cart is slowest.
const SPEED_FOOT_KMD: f32 = 25.0;   // traveller on foot
const SPEED_HORSE_KMD: f32 = 55.0;  // mounted / post-horse
const SPEED_CART_KMD: f32 = 18.0;   // ox / mule cart (slow, heavy)

/// Physical length (km), travel time (days) and dominant transport mode
/// (0 land / 1 sea / 2 river) of a coarse routed path. Each coarse step spans `f`
/// fine cells; diagonal steps are √2 longer. Mode + terrain set the per-leg speed.
fn path_metrics(cc: &CoarseCost, path: &[usize], km_per_cell: f32) -> (f32, f32, u8) {
    let mut km = 0.0f32;
    let mut days = 0.0f32;
    let (mut land, mut sea, mut river) = (0u32, 0u32, 0u32);
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ax = (a as i32) % cc.cw; let ay = (a as i32) / cc.cw;
        let bx = (b as i32) % cc.cw; let by = (b as i32) / cc.cw;
        let mut dx = (ax - bx).abs(); if dx > cc.cw / 2 { dx = cc.cw - dx; }
        let dy = (ay - by).abs();
        let diag = dx != 0 && dy != 0;
        let seg_km = (if diag { std::f32::consts::SQRT_2 } else { 1.0 }) * cc.f as f32 * km_per_cell;
        km += seg_km;
        let (a_land, b_land) = (cc.is_land[a], cc.is_land[b]);
        let (a_riv, b_riv) = (cc.is_river[a], cc.is_river[b]);
        let (speed, factor) = if a_land && b_land && (a_riv || b_riv) {
            river += 1;
            (SPEED_RIVER_KMD, 1.0 + 0.4 * ((cc.elev[a] + cc.elev[b]) * 0.5))
        } else if !a_land || !b_land {
            sea += 1;
            let hz = ((cc.sea_hazard[a] + cc.sea_hazard[b]) * 0.5).clamp(0.0, 1.0);
            (SPEED_SHIP_KMD, 1.0 + 0.8 * hz)
        } else {
            land += 1;
            (SPEED_CARAVAN_KMD, 1.0 + 1.6 * ((cc.elev[a] + cc.elev[b]) * 0.5))
        };
        days += seg_km / speed.max(1.0) * factor;
    }
    let mode = if sea >= land && sea >= river { 1u8 } else if river > land { 2 } else { 0 };
    (km, days, mode)
}

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
    desert_routes: bool, piracy: f32, season: i32, months: u32,
) -> Result<CoarseCost, String> {
    let f = (grid_w / 700).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let cn = (cw * ch) as usize;

    // Sample each coarse cell from its centre fine cell (one tile pass).
    let mut is_land = vec![false; cn];
    let mut elev = vec![0.0f32; cn];
    let mut koppen = vec![0u8; cn];
    // Surface temperature per coarse cell — used to freeze impassable sea ice and
    // near-block ice-sheet land so routes can't cross frozen ocean / ice caps.
    let mut temp = vec![0.0f32; cn];
    // Maritime-hazard exposure per coarse cell (annual storm peak + reef wreck
    // risk), used to make stormy/reef-fouled sea more expensive to cross.
    let mut sea_hazard = vec![0.0f32; cn];
    // Continental-shelf mask per coarse cell. Shelf (shallow) water is "reachable
    // sea" — coastal navigation that ancient/medieval shipping hugged — so it is
    // cheap AND is NOT counted as an open-water crossing. Only deep / high sea
    // (non-shelf ocean) counts against the trade-reach crossing limit.
    let mut shelf = vec![false; cn];
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
        let mut f_temp = vec![0.0f32; fn_cells];
        let mut f_shelf = vec![0u8; fn_cells];
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
                        f_temp[gi] = tile.temperature[ti];
                        f_shelf[gi] = tile.is_shelf[ti];
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
                temp[ci] = f_temp[gi];
                shelf[ci] = f_shelf[gi] != 0;
            }
        }
    }

    // Coarse river mask from overlay JSON (rivers aren't stored in tiles).
    let mut is_river = vec![false; cn];
    // Navigable trunks are a distinct, cheaper corridor than minor rivers (a
    // barge highway vs a fordable creek), so route preferentially down them.
    let mut is_nav_river = vec![false; cn];
    {
        let rivers: Vec<RouteRiver> = serde_json::from_str(rivers_json).unwrap_or_default();
        for r in &rivers {
            for &(rx, ry) in &r.points {
                let cx = (rx / f).min(cw as u32 - 1) as i32;
                let cy = (ry / f).min(ch as u32 - 1) as i32;
                let ci = (cy * cw + cx) as usize;
                is_river[ci] = true;
                if r.navigable { is_nav_river[ci] = true; }
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
                    1 => 6.0,               // AF rainforest (+ tsetse: pack animals die)
                    2 | 3 | 23 => 4.0,      // Am/Aw/As savanna-woodland tsetse belt: caravans shun it
                    32 => 7.0,              // H highland
                    _ => 0.0,
                };
                // Navigable trunk = a fast inland highway (cheapest overland
                // corridor); a minor river is still a cheap valley route.
                if is_nav_river[ci] { c = c.min(0.8); }
                else if is_river[ci] { c = c.min(1.4); }
                // Ice-sheet land (EF ice cap or a deeply frozen interior) is
                // near-impassable — caravans don't road across a glacier.
                if koppen[ci] == 22 || temp[ci] <= ICE_LAND_C {
                    c = c.max(ICE_LAND_COST);
                }
                c
            } else if temp[ci] <= SEA_FREEZE_C {
                // Frozen ocean (sea ice) — ships cannot cross, even in an
                // otherwise-open sea. This also covers ice shelves over water.
                SEA_BLOCK_COST
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
                // Coastal-ring AND continental-shelf water is cheap coast-hugging
                // shipping (reachable sea). …but never re-open frozen water (sea ice
                // stays blocked).
                if (coastal || shelf[ci]) && temp[ci] > SEA_FREEZE_C { cost[ci] = 0.5; }
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
        // Lower base weight so that, absent real piracy/seasonal storms, goods take
        // the SHORTEST sea route rather than detouring around mild intrinsic hazard
        // (user: "keep shortest possible route … if there are no hazards"). Piracy
        // and seasonal storm closures (applied below) still bend routes when set.
        let hazard_w: f32 = if desert_routes { 20.0 } else { 4.0 };
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
                // Shelf (shallow) water is reachable coastal sea, NOT open ocean —
                // so it never counts against the open-water crossing limit. Only
                // deep / high sea beyond the shelf is a true crossing.
                is_open_sea[ci] = !coastal && !shelf[ci];
            }
        }
    }

    // Piracy surcharge (#11): raiders haunt the busy coastal narrows / straits
    // worst, the open sea less so. Higher piracy pushes trade to hug safer coasts,
    // detour, or go overland — and makes goods that can only arrive by sea dearer.
    if !block_sea && piracy > 0.0 {
        for ci in 0..cn {
            if is_land[ci] { continue; }
            cost[ci] += piracy * if is_open_sea[ci] { 1.5 } else { 4.0 };
        }
    }

    // Seasonal closures (#12): for a specific moon (season 1..=months) high
    // mountain passes snow shut in the hemisphere's winter and monsoon/cyclone
    // seas close their sailing windows, so routes detour or wait. season <= 0
    // (or months == 0) leaves the annual grid untouched.
    if season > 0 && months > 0 {
        let m = (((season as u32).min(months) - 1) as f32 + 0.5) / months as f32; // 0..1 round the year
        let smooth = |x: f32, a: f32, b: f32| -> f32 { let t = ((x - a) / (b - a)).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) };
        for cy in 0..ch {
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1);
            let lat = 90.0 - (wy as f32 / grid_h as f32) * 180.0; // north positive
            for cx in 0..cw {
                let ci = (cy * cw + cx) as usize;
                if is_land[ci] {
                    // Snow-shut passes: high relief, mid/high latitude, deep winter.
                    if elev[ci] >= 0.45 {
                        let shift = if lat >= 0.0 { 0.0 } else { 0.5 };
                        let winter = ((m - shift) * std::f32::consts::TAU).cos() * 0.5 + 0.5;
                        let latw = smooth(lat.abs(), 22.0, 50.0);
                        cost[ci] += winter * latw * 45.0;
                    }
                } else if !block_sea {
                    // Monsoon / cyclone sailing window: stormy seas dearer in season.
                    let monsoon = sea_hazard[ci].clamp(0.0, 1.0)
                        * crate::sim::biological::storm_season_phase(season, months, lat);
                    cost[ci] += monsoon * 16.0;
                }
            }
        }
    }

    Ok(CoarseCost { f, cw, ch, cost, is_land, is_open_sea, is_river, elev, sea_hazard })
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
    piracy: f32,
    season: i32,
    months: u32,
) -> Result<Arc<CoarseCost>, String> {
    let mut hasher = DefaultHasher::new();
    rivers_json.hash(&mut hasher);
    desert_routes.hash(&mut hasher); // distinct grid when desert-routing is on
    piracy.to_bits().hash(&mut hasher); // distinct grid per piracy level
    season.hash(&mut hasher);
    months.hash(&mut hasher); // distinct grid per seasonal closure month
    let key: CostKey = (fingerprint.0, fingerprint.1, block_sea, hasher.finish());
    if let Some(any) = db.cost_cache_get(&key) {
        if let Ok(cc) = any.downcast::<CoarseCost>() {
            return Ok(cc);
        }
    }
    let cc = Arc::new(build_coarse_cost(world, grid_w, grid_h, rivers_json, block_sea, desert_routes, piracy, season, months)?);
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

/// Single-source Dijkstra over the coarse cost grid returning the predecessor array,
/// so many goals can be reconstructed from ONE run (the campaign route-days matrix).
fn coarse_dijkstra_prev(cc: &CoarseCost, start: usize) -> Vec<usize> {
    let cw = cc.cw;
    let ch = cc.ch;
    let cn = (cw * ch) as usize;
    let mut dist = vec![i64::MAX; cn];
    let mut prev = vec![usize::MAX; cn];
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
    if start >= cn { return prev; }
    dist[start] = 0;
    heap.push(Reverse((0, start)));
    while let Some(Reverse((d, u))) = heap.pop() {
        if d > dist[u] { continue; }
        let ux = (u as i32) % cw;
        let uy = (u as i32) / cw;
        for &(dx, dy, mult) in &COARSE_DIRS {
            let ny = uy + dy;
            if ny < 0 || ny >= ch { continue; }
            let v = cc.cidx(ux + dx, ny);
            let step = ((cc.cost[u] + cc.cost[v]) * 0.5 * mult * 100.0) as i64;
            let nd = d.saturating_add(step.max(1));
            if nd < dist[v] { dist[v] = nd; prev[v] = u; heap.push(Reverse((nd, v))); }
        }
    }
    prev
}

/// Geometric length (in FINE cells) of the reconstructed least-cost path start→goal,
/// or None if goal is unreachable. Used to convert a routed path into travel-days at the
/// sim's own `days_per_cell` scale (so detours around mountains/seas lengthen a leg while
/// open routes stay ≈ the straight-line time — economic calibration is preserved).
fn coarse_path_len_cells(cc: &CoarseCost, prev: &[usize], start: usize, goal: usize) -> Option<f32> {
    if goal == start { return Some(0.0); }
    if goal >= prev.len() || prev[goal] == usize::MAX { return None; }
    let cw = cc.cw;
    let mut len = 0.0f32;
    let mut cur = goal;
    let mut guard = 0usize;
    while cur != start {
        let p = prev[cur];
        if p == usize::MAX { return None; }
        let (cx, cy) = ((cur as i32) % cw, (cur as i32) / cw);
        let (px, py) = ((p as i32) % cw, (p as i32) / cw);
        let mut dx = (cx - px).abs();
        if dx > cw / 2 { dx = cw - dx; } // cylindrical wrap on X
        let dy = cy - py;
        len += ((dx * dx + dy * dy) as f32).sqrt();
        cur = p;
        guard += 1;
        if guard > (cc.cw * cc.ch) as usize { return None; } // safety
    }
    Some(len * cc.f as f32)
}

/// Precompute the campaign's REAL pathfound route-days matrix (n·n) over the coarse cost
/// grid: mountain passes, rivers, coast-hugging and sea crossings are all priced in, so
/// campaign trade/migration follow real lanes and NEVER draw a straight line. Days are the
/// routed path's cell-length × `days_per_cell` (the sim's own scale). Only same-`component`
/// pairs are filled; everything else is `INFINITY` (matches the sim's trade regions). On
/// any failure the caller falls back to the old Euclidean matrix.
pub(crate) fn compute_route_days_matrix(
    db: &WorldDb,
    conn: &rusqlite::Connection,
    hub_xy: &[(f32, f32)],
    components: &[u32],
    days_per_cell: f32,
) -> Result<Vec<f32>, String> {
    let n = hub_xy.len();
    if n == 0 { return Ok(vec![]); }
    let grid_w: u32 = metadata::get_meta(conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Err("no grid".into()); }
    let world = db.cached_tiles_with_conn(conn)?;
    // Allow sea crossings (block_sea=false) and desert routes so islands/continents in one
    // component connect; no seasonal closure. Rivers omitted (campaign has no overlay JSON).
    let cc = cached_coarse_cost(db, &world, world.fingerprint, grid_w, grid_h, "", false, true, 0.0, -1, 12)?;
    let cell = |x: f32, y: f32| -> usize {
        let cx = ((x / cc.f as f32) as i32).clamp(0, cc.cw - 1);
        let cy = ((y / cc.f as f32) as i32).clamp(0, cc.ch - 1);
        (cy * cc.cw + cx) as usize
    };
    let nodes: Vec<usize> = hub_xy.iter().map(|&(x, y)| cell(x, y)).collect();
    let mut days = vec![f32::INFINITY; n * n];
    for a in 0..n {
        days[a * n + a] = 0.0;
        // Only run the (costly) Dijkstra if this hub shares a component with any other.
        let has_partner = (0..n).any(|b| b != a && components[b] == components[a]);
        if !has_partner { continue; }
        let prev = coarse_dijkstra_prev(&cc, nodes[a]);
        for b in 0..n {
            if b == a || components[b] != components[a] { continue; }
            if let Some(len) = coarse_path_len_cells(&cc, &prev, nodes[a], nodes[b]) {
                days[a * n + b] = (len * days_per_cell).max(1.0);
            }
        }
    }
    Ok(days)
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
    piracy: f32,
    season: i32,
    months: u32,
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
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, season, months)?;
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

/// A point-to-point journey over the shared coarse movement-cost grid: the routed
/// polyline plus the physical distance split by medium (land / navigable river /
/// sea) and the travel time for each land-travel mode. Water legs always go by
/// boat (river) or ship (sea); the chosen mode only changes the land speed.
#[derive(Serialize)]
pub struct Itinerary {
    pub points: Vec<[f32; 2]>,
    pub reachable: bool,
    pub km: f32,
    pub land_km: f32,
    pub river_km: f32,
    pub sea_km: f32,
    pub days_foot: f32,
    pub days_horse: f32,
    pub days_cart: f32,
    /// 0 = mostly overland, 1 = mostly sea, 2 = mostly river.
    pub dominant_mode: u8,
}

/// Travel-time / itinerary calculator (#23). Least-cost route between two world
/// cells over the same coarse grid trade uses, then per-mode journey times. Reuses
/// `cached_coarse_cost`, `coarse_dijkstra`, `path_metrics`'s per-segment medium
/// classification and `world_of` (no new pathfinding). `reach == 2` (continental)
/// blocks open-sea crossings so the route stays on one landmass.
#[tauri::command]
pub fn compute_itinerary(
    from_x: u32,
    from_y: u32,
    to_x: u32,
    to_y: u32,
    rivers_json: String,
    reach: u8,
    desert_routes: bool,
    db: State<'_, WorldDb>,
) -> Result<Itinerary, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let empty = Itinerary {
        points: vec![], reachable: false, km: 0.0, land_km: 0.0, river_km: 0.0,
        sea_km: 0.0, days_foot: 0.0, days_horse: 0.0, days_cart: 0.0, dominant_mode: 0,
    };
    if grid_w == 0 || grid_h == 0 { return Ok(empty); }

    let world = db.cached_tiles_with_conn(&conn)?;
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h,
        &rivers_json, reach == 2, desert_routes, 0.0, 0, 0)?;
    let km_per_cell = KM_EQUATOR / grid_w as f32;

    let start = cc.cidx((from_x / cc.f).min(cc.cw as u32 - 1) as i32,
                        (from_y / cc.f).min(cc.ch as u32 - 1) as i32);
    let goal = cc.cidx((to_x / cc.f).min(cc.cw as u32 - 1) as i32,
                       (to_y / cc.f).min(cc.ch as u32 - 1) as i32);

    let path = match coarse_dijkstra(&cc, start, goal) {
        Some(p) => p,
        None => return Ok(Itinerary {
            points: vec![cc.world_of(start), cc.world_of(goal)], ..empty
        }),
    };

    // Per-medium effective distance (km × terrain factor) mirrors `path_metrics`:
    // relief slows land legs; storm/reef hazard slows sea legs.
    let (mut land_km, mut river_km, mut sea_km) = (0.0f32, 0.0f32, 0.0f32);
    let (mut land_eff, mut river_eff, mut sea_eff) = (0.0f32, 0.0f32, 0.0f32);
    let (mut land_n, mut sea_n, mut river_n) = (0u32, 0u32, 0u32);
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ax = (a as i32) % cc.cw; let ay = (a as i32) / cc.cw;
        let bx = (b as i32) % cc.cw; let by = (b as i32) / cc.cw;
        let mut dx = (ax - bx).abs(); if dx > cc.cw / 2 { dx = cc.cw - dx; }
        let dy = (ay - by).abs();
        let diag = dx != 0 && dy != 0;
        let seg_km = (if diag { std::f32::consts::SQRT_2 } else { 1.0 }) * cc.f as f32 * km_per_cell;
        let (a_land, b_land) = (cc.is_land[a], cc.is_land[b]);
        let (a_riv, b_riv) = (cc.is_river[a], cc.is_river[b]);
        if a_land && b_land && (a_riv || b_riv) {
            river_n += 1;
            river_km += seg_km;
            river_eff += seg_km * (1.0 + 0.4 * ((cc.elev[a] + cc.elev[b]) * 0.5));
        } else if !a_land || !b_land {
            sea_n += 1;
            sea_km += seg_km;
            let hz = ((cc.sea_hazard[a] + cc.sea_hazard[b]) * 0.5).clamp(0.0, 1.0);
            sea_eff += seg_km * (1.0 + 0.8 * hz);
        } else {
            land_n += 1;
            land_km += seg_km;
            land_eff += seg_km * (1.0 + 1.6 * ((cc.elev[a] + cc.elev[b]) * 0.5));
        }
    }
    let water_days = river_eff / SPEED_RIVER_KMD + sea_eff / SPEED_SHIP_KMD;
    let dominant_mode = if sea_n >= land_n && sea_n >= river_n { 1 }
        else if river_n > land_n { 2 } else { 0 };

    Ok(Itinerary {
        points: path.iter().map(|&c| cc.world_of(c)).collect(),
        reachable: true,
        km: land_km + river_km + sea_km,
        land_km, river_km, sea_km,
        days_foot: land_eff / SPEED_FOOT_KMD + water_days,
        days_horse: land_eff / SPEED_HORSE_KMD + water_days,
        days_cart: land_eff / SPEED_CART_KMD + water_days,
        dominant_mode,
    })
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

/// Monsoon-climate intensity (0..1) for a Köppen code: the tropical monsoon (Am)
/// highest, then the wet/dry savanna (Aw/As) and the dry-winter "w" subtropical /
/// continental types (Cw*/Dw*) that are driven by a seasonal monsoon reversal.
fn monsoon_climate_intensity(k: u8) -> f32 {
    use crate::sim::koppen::*;
    // A "monsoon zone" is a region of MARKED seasonal rain reversal — the tropical
    // monsoon (Am) and the dry-winter monsoon climates (Cw*/Dw*, driven by the
    // winter continental high + summer onshore monsoon). Plain wet/dry savanna
    // (Aw/As) is ITCZ-migration rainfall, not a true monsoon, and tagging it lit
    // up the whole of sub-Saharan Africa — so it no longer counts here. The
    // genuine West-African / Indian monsoon coasts still register through Am.
    match k {
        AM => 1.0,       // tropical monsoon (heaviest wet-season rains)
        CWA => 0.85,     // monsoon-influenced humid subtropical
        CWB => 0.7,      // monsoon subtropical highland
        CWC => 0.55,
        DWA => 0.8,      // monsoon-influenced continental (E-Asia winter monsoon)
        DWB => 0.65,
        DWC => 0.5,
        DWD => 0.4,
        _ => 0.0,        // Aw/As savanna excluded — not a true monsoon
    }
}

/// Cluster the LAND areas under a monsoon-type climate (tropical monsoon Am,
/// savanna Aw/As, and the dry-winter Cw*/Dw* types) into discrete regions — the
/// seasonal wet-season flood belt. Shown on the Natural Disasters overlay
/// alongside the cyclone (hurricane) zones. Mirrors `compute_storm_zones`.
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
                    let v = monsoon_climate_intensity(tile.koppen[ti]);
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

/// One people's territory for the "Peoples" overlay — the coarse cells the hearth
/// governs, plus its colour and labels. Built straight from the in-memory culture
/// map (no WorldBuffer load).
#[derive(Serialize)]
pub struct CultureRegion {
    pub cells: Vec<[f32; 2]>, // coarse cell top-left world coords (the marked area)
    pub cell_size: f32,
    pub x: f32,               // label centroid
    pub y: f32,
    pub color: [u8; 3],
    pub label: String,        // people / region name (e.g. "Vexillia")
    pub culture: String,      // kit name (e.g. "Norse")
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
        let mut regions = cluster_cells(source_grid, cw, ch, f, 0.30, min_cells);
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
    piracy: f32,
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
    let lux_mult = (0.6 + luxury_bias.clamp(0.0, 1.0) * 1.4).clamp(0.3, 2.2);
    // Full-basket floor, unified with compute_economy: every good is at least
    // modestly desired so regions import what they don't produce (not only the
    // few high-`desire` staples), and the two engines agree on who imports what.
    const BASKET_FLOOR: f32 = 0.35;
    let mut demand = vec![vec![0.0f32; gc]; nr];
    for ri in 0..nr {
        let size = region_weight[ri] / max_rw; // 0..1
        for g in 0..gc {
            let mut d = size * desire[g].max(BASKET_FLOOR);
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
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, 0, 0)?;
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

/// DLC 3.5 · the live campaign's DYNAMIC TRADE FLOW: last year's actual shipped
/// volume per hub-pair, routed over the SAME coarse cost grid as the static trunks
/// (so flows follow real roads/sea-lanes, not straight lines) and bundled per
/// coarse edge — width ∝ the year's volume on that segment, so the busiest arteries
/// read thickest. Recomputed once a year inside the tick; this just routes + bundles
/// the snapshot. Returns the same `TradeTrunk` shape the trunk overlay renders.
#[tauri::command]
pub fn campaign_get_trade_flow(
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TradeTrunk>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let sim = match crate::commands::campaign_commands::get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(vec![]),
    };
    if sim.flow_year.is_empty() { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h,
        &rivers_json, reach == 2, true, 0.0, -1, 12)?;

    // Map each real hub (by stable id) to its coarse node.
    let mut node_of: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for h in &sim.hubs {
        if h.is_estate { continue; }
        let cx = ((h.x.max(0.0) as u32) / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = ((h.y.max(0.0) as u32) / cc.f).min(cc.ch as u32 - 1) as i32;
        node_of.insert(h.id, cc.cidx(cx, cy));
    }
    // Route each pair and bundle its volume onto every coarse edge it traverses.
    let mut edge: std::collections::HashMap<(usize, usize), f32> = std::collections::HashMap::new();
    for &(a_id, b_id, vol) in &sim.flow_year {
        if vol <= 0.0 { continue; }
        let (s, g) = match (node_of.get(&a_id), node_of.get(&b_id)) {
            (Some(&s), Some(&g)) if s != g => (s, g),
            _ => continue,
        };
        let path = match coarse_dijkstra(&cc, s, g) { Some(p) => p, None => continue };
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }
        for w in path.windows(2) {
            let e = (w[0].min(w[1]), w[0].max(w[1]));
            *edge.entry(e).or_insert(0.0) += vol;
        }
    }
    let mut trunks: Vec<TradeTrunk> = edge.into_iter()
        .filter(|(_, v)| *v > 0.0)
        .map(|((a, b), v)| TradeTrunk {
            points: vec![cc.world_of(a), cc.world_of(b)], volume: v, good: -1, road: String::new(),
        })
        .collect();
    trunks.sort_by(|x, y| y.volume.partial_cmp(&x.volume).unwrap_or(std::cmp::Ordering::Equal));
    Ok(trunks)
}

// ─────────────────────────────────────────────────────────────────────────────
// Trade CORRIDORS — campaign-only. A long-haul trade tie between a home city and
// a distant one, ROUTED with the very same river/terrain-prone coarse pathing the
// trade routes use, then dressed with WAYSTATIONS placed by leg terrain
// (river-port / caravanserai / coastal factory / pass hospice) and tinted by the
// OWNING house. Read-only + recomputed from live campaign flow, so a corridor
// network grows and shifts with the campaign. Ocean-crossing is disallowed via the
// same `path_allowed` reach/max-crossing gate — corridors hug rivers, land & coast.
// ─────────────────────────────────────────────────────────────────────────────

/// Golden-angle HSL house tint — matches `campaign_commands::distinct_color` so a
/// corridor's colour agrees with the ward-grid / family-influence colours.
fn house_hex(i: usize) -> String {
    let hue = (i as f32 * 137.508) % 360.0;
    let (s, l) = (0.68f32, 0.55f32);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = hue / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    format!("#{:02x}{:02x}{:02x}", to(r1), to(g1), to(b1))
}

/// A waystation dropped along a corridor: `kind` 1 river-port · 2 caravanserai ·
/// 3 coastal factory · 4 mountain-pass hospice.
#[derive(Serialize)]
pub struct CorridorWaystation { pub x: f32, pub y: f32, pub kind: u8 }

/// One routed trade corridor for the map overlay + Corridors panel.
#[derive(Serialize)]
pub struct TradeCorridor {
    pub origin: String,
    pub dest: String,
    pub owner: String,          // owning house ("civic" when no resident house)
    pub color: String,          // owner hex tint (matches the ward grid)
    pub good: String,           // the origin's chief export the corridor carries
    pub volume: f32,
    pub km: f32,
    pub days: f32,
    pub land_legs: u32,
    pub river_legs: u32,
    pub sea_legs: u32,
    pub points: Vec<[f32; 2]>,  // world-coord polyline (the routed path)
    pub waystations: Vec<CorridorWaystation>,
}

#[tauri::command]
pub fn campaign_get_corridors(
    rivers_json: String,
    reach: u8,
    max_crossing: f32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TradeCorridor>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let sim = match crate::commands::campaign_commands::get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(vec![]),
    };
    // Event-driven: corridors are the ESTABLISHED routes earned by expeditions
    // (docs/EXPEDITIONS_CORRIDORS_PLAN.md), not the top-N yearly flows. There are
    // only a handful, so routing them is cheap — this is what removes the year-tick
    // recompute storm the old flow_year source caused.
    if sim.corridors.is_empty() { return Ok(vec![]); }
    let world = db.cached_tiles_with_conn(&conn)?;
    // Same river/terrain-prone coarse grid the trade routes ride.
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h,
        &rivers_json, reach == 2, true, 0.0, -1, 12)?;
    let km_per_cell = KM_EQUATOR / grid_w as f32;

    // Hub id → (index, coarse node).
    let mut idx_of_id: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut node_of: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (i, h) in sim.hubs.iter().enumerate() {
        if h.is_estate { continue; }
        idx_of_id.insert(h.id, i);
        let cx = ((h.x.max(0.0) as u32) / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = ((h.y.max(0.0) as u32) / cc.f).min(cc.ch as u32 - 1) as i32;
        node_of.insert(h.id, cc.cidx(cx, cy));
    }
    // Strongest resident house per hub INDEX — the corridor's owner-from-home.
    let mut owner_of: std::collections::HashMap<usize, (usize, f32)> = std::collections::HashMap::new();
    for (hi, hh) in sim.houses.iter().enumerate() {
        if hh.defunct { continue; }
        let e = owner_of.entry(hh.hub as usize).or_insert((usize::MAX, f32::MIN));
        if hh.wealth > e.1 { *e = (hi, hh.wealth); }
    }

    // The established corridors (as id-pairs with a volume proxy = successful
    // ventures) — routed geographically below, strongest first.
    let mut flows: Vec<(u32, u32, f32)> = sim.corridors.iter().filter_map(|c| {
        let a = sim.hubs.get(c.a as usize)?;
        let b = sim.hubs.get(c.b as usize)?;
        Some((a.id, b.id, (c.successes.max(1)) as f32))
    }).collect();
    flows.sort_by(|x, y| y.2.partial_cmp(&x.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
    let mut corridors: Vec<TradeCorridor> = Vec::new();
    for (a_id, b_id, vol) in flows {
        if vol <= 0.0 { continue; }
        if corridors.len() >= 24 { break; }
        let key = (a_id.min(b_id), a_id.max(b_id));
        if !seen.insert(key) { continue; }
        let (sa, sb) = match (node_of.get(&a_id), node_of.get(&b_id)) {
            (Some(&s), Some(&g)) if s != g => (s, g), _ => continue,
        };
        let path = match coarse_dijkstra(&cc, sa, sb) { Some(p) => p, None => continue };
        if path.len() < 6 { continue; }                                   // long hauls only
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; } // no ocean crossing
        let (km, days, _mode) = path_metrics(&cc, &path, km_per_cell);
        let (ai, bi) = match (idx_of_id.get(&a_id), idx_of_id.get(&b_id)) {
            (Some(&ai), Some(&bi)) => (ai, bi), _ => continue,
        };
        // Origin = the endpoint whose resident house is stronger (the backer).
        let (oa, ob) = (owner_of.get(&ai).copied(), owner_of.get(&bi).copied());
        let (origin_idx, dest_idx, owner) = match (oa, ob) {
            (Some(x), Some(y)) => if x.1 >= y.1 { (ai, bi, Some(x.0)) } else { (bi, ai, Some(y.0)) },
            (Some(x), None) => (ai, bi, Some(x.0)),
            (None, Some(y)) => (bi, ai, Some(y.0)),
            (None, None) => (ai, bi, None),
        };
        let (owner_name, color) = match owner {
            Some(hi) if hi < sim.houses.len() => (sim.houses[hi].name.clone(), house_hex(hi)),
            _ => ("civic".to_string(), "#7a8aa0".to_string()),
        };
        // The corridor carries the origin's chief export.
        let good = {
            let prod = &sim.hubs[origin_idx].production;
            let mut best = (usize::MAX, 0.0f32);
            for (g, &p) in prod.iter().enumerate() { if p > best.1 { best = (g, p); } }
            if best.0 != usize::MAX && best.0 < sim.goods.len() {
                sim.goods[best.0].name.clone()
            } else { String::new() }
        };
        // Waystations at regular intervals, typed by the cell's terrain; the whole
        // path's cells are tallied for the leg breakdown.
        let spacing = (path.len() / 6).max(2);
        let mut waystations: Vec<CorridorWaystation> = Vec::new();
        let (mut land_legs, mut river_legs, mut sea_legs) = (0u32, 0u32, 0u32);
        let last = path.len() - 1;
        for (k, &c) in path.iter().enumerate() {
            let kind = if !cc.is_land[c] { sea_legs += 1; 3u8 }
                else if cc.is_river[c] { river_legs += 1; 1 }
                else if cc.elev[c] > 0.62 { land_legs += 1; 4 }
                else { land_legs += 1; 2 };
            if k == 0 || k == last || k % spacing != 0 { continue; }
            let w = cc.world_of(c);
            waystations.push(CorridorWaystation { x: w[0], y: w[1], kind });
        }
        let points: Vec<[f32; 2]> = path.iter().map(|&c| cc.world_of(c)).collect();
        corridors.push(TradeCorridor {
            origin: sim.hubs[origin_idx].name.clone(),
            dest: sim.hubs[dest_idx].name.clone(),
            owner: owner_name, color, good, volume: vol,
            km, days, land_legs, river_legs, sea_legs,
            points, waystations,
        });
    }
    Ok(corridors)
}

// ─────────────────────────────────────────────────────────────────────────────
// Expeditions — financed ventures crawling toward distant lands (the way a
// corridor is earned). Read-only projection of the live sim for the map + panel.
// ─────────────────────────────────────────────────────────────────────────────

/// One live venture for the map/panel: current position, progress, fleet, cargo,
/// leader, survival + recent hazards (the struggle story).
#[derive(Serialize)]
pub struct ExpeditionView {
    pub id: u32,
    pub leader: String,
    pub origin: String,
    pub dest: String,
    pub x: f32,
    pub y: f32,
    pub ox: f32, pub oy: f32,   // origin (for drawing the intended track)
    pub dx: f32, pub dy: f32,   // destination
    pub progress: f32,          // 0..1 over the whole round trip
    pub outbound: bool,
    pub status: u8,             // 0 en-route · 1 arrived · 2 returning
    pub caravans: u16,
    pub ships: u16,
    pub good: String,
    pub survived: f32,          // fraction of the fleet still alive
    pub cost: f32,
    pub launched_year: u32,
    pub hazards: Vec<[f32; 3]>, // recent (x, y, kind) struggles
}

/// A recent FAILED venture, for the map ✕ overlay.
#[derive(Serialize)]
pub struct ExpeditionFail {
    pub x: f32,
    pub y: f32,
    pub kind: u8,
}

#[derive(Serialize)]
pub struct ExpeditionsPayload {
    pub active: Vec<ExpeditionView>,
    pub failed: Vec<ExpeditionFail>,
}

#[tauri::command]
pub fn campaign_get_expeditions(db: State<'_, WorldDb>) -> Result<ExpeditionsPayload, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match crate::commands::campaign_commands::get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(ExpeditionsPayload { active: vec![], failed: vec![] }),
    };
    let mut active = Vec::new();
    for e in sim.expeditions.iter() {
        if e.status >= 3 { continue; } // en-route / arrived / returning only
        let t = e.pos.clamp(0.0, 1.0);
        let (sx, sy, gx, gy) = if e.outbound { (e.ox, e.oy, e.dx, e.dy) } else { (e.dx, e.dy, e.ox, e.oy) };
        let (x, y) = (sx + (gx - sx) * t, sy + (gy - sy) * t);
        let progress = if e.outbound { t * 0.5 } else { 0.5 + t * 0.5 };
        let origin = sim.hubs.get(e.origin as usize).map(|h| h.name.clone()).unwrap_or_default();
        let dest = sim.hubs.get(e.dest as usize).map(|h| h.name.clone()).unwrap_or_default();
        let good = sim.goods.get(e.good as usize).map(|g| g.name.clone()).unwrap_or_default();
        let hazards = e.hazards.iter().rev().take(6).map(|h| [h.x, h.y, h.kind as f32]).collect();
        active.push(ExpeditionView {
            id: e.id, leader: e.leader.clone(), origin, dest,
            x, y, ox: e.ox, oy: e.oy, dx: e.dx, dy: e.dy,
            progress, outbound: e.outbound, status: e.status,
            caravans: e.caravans, ships: e.ships, good,
            survived: e.arrived_frac, cost: e.cost, launched_year: e.launched_tick / 365, hazards,
        });
    }
    let failed = sim.failed_expeditions.iter().rev().take(40)
        .map(|h| ExpeditionFail { x: h.x, y: h.y, kind: h.kind }).collect();
    Ok(ExpeditionsPayload { active, failed })
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
    pub emporium: bool,         // one of the few greatest entrepôts — rendered RED
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
    piracy: f32,
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
    // Emit only the MOST IMPORTANT hubs (the user asked for far fewer trade hubs);
    // the contender pool stays large so ranking is accurate, but the rendered set
    // is trimmed hard. Lesser towns still exist in the settlement list for later
    // layers — they just aren't drawn as trade hubs.
    let hub_out = er.clamp(6, 20);
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
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, 0, 0)?;
    let cnode: Vec<usize> = nodes.iter().map(|s| {
        let cx = (s.x / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (s.y / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();
    // Sea access (real ocean port, NOT a closed lake) + outpost flags per node.
    // distance_to_ocean only measures distance to the OPEN OCEAN, so a lakeshore
    // town reads as inland here — exactly what we want: lake hubs can't be emporia.
    let sea_access: Vec<bool> = nodes.iter().map(|s| {
        let t = world.tile((s.x / TILE_SIZE) as i32, (s.y / TILE_SIZE) as i32);
        let ti = ((s.y % TILE_SIZE) * TILE_SIZE + (s.x % TILE_SIZE)) as usize;
        t.distance_to_ocean.get(ti).map(|&d| d < 0.06).unwrap_or(false)
    }).collect();
    let is_outpost: Vec<bool> = nodes.iter().map(|s| s.size == "outpost").collect();
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
    let pop_max = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    for ni in 0..nn {
        // Population-weighted (matches compute_economy) so the top hub — the one
        // marked with the golden square — is a large central city, not a remote
        // island that just monopolises a good.
        let pop_n = (nodes[ni].population as f32 / pop_max).clamp(0.0, 1.0).sqrt();
        let mut p = 0.26 * nodes[ni].score.clamp(0.0, 1.0)
            + 0.30 * pop_n
            + 0.30 * (centrality[ni] / cmax)
            + 0.14 * (monopoly[ni] / mmax);
        // Great trade hubs are SEA PORTS — a sea-accessible node gets a boost, and a
        // lake-locked / inland one is held back, so the largest hubs sit on the coast
        // of the open sea, not a closed lake (the user's request).
        if sea_access[ni] { p *= 1.18; } else { p *= 0.72; }
        // Tiny trade outposts are never hub contenders — floor their power so they
        // can't be ranked, marked, or named as trade hubs (they get black dots).
        if is_outpost[ni] { p *= 0.05; }
        power[ni] = p;
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

    // Emporia: the few greatest entrepôts by throughput (rendered red). They MUST be
    // sea ports — a great pass-through entrepôt is a maritime hub, never a lake town.
    let emporia: std::collections::HashSet<usize> = {
        let mut t: Vec<(f32, usize)> = (0..nn)
            .filter(|&ni| sea_access[ni] && !is_outpost[ni])
            .map(|ni| (throughput[ni], ni)).collect();
        t.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        t.into_iter().take(6).filter(|&(v, _)| v > 0.0).map(|(_, ni)| ni).collect()
    };

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
            emporium: emporia.contains(&ni),
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
    pub flavor: String, // evocative per-good variety name by quality (empty if none)
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
pub struct ExchangeRate {
    /// Counter-good name.
    pub good_name: String,
    /// Units of the counter-good one unit of this good buys here.
    pub ratio: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HubMarketGood {
    pub good: usize,
    pub good_name: String,
    /// Local price in the grain-equivalent numeraire.
    pub price: f32,
    /// World-standard value (the spec's base_value) for comparison.
    pub base_value: f32,
    pub in_flow: f32,
    pub out_flow: f32,
    /// What this good actually trades against here (top counter-goods).
    pub exchanged_for: Vec<ExchangeRate>,
}

/// One emergent currency good with the components that made it money — for the
/// settlement window's explained currency card (liquidity/value/stability bars +
/// grain-equivalent price for exchange ratios).
#[derive(Serialize, Deserialize, Clone)]
pub struct HubCurrency {
    pub good: usize,
    pub name: String,
    /// Distinct trade counterparties (raw count → liquidity).
    pub liquidity: f32,
    /// World-standard value (grain-equivalent base value).
    pub value: f32,
    /// Price stability 0..1 (1 = rock-steady).
    pub stability: f32,
    /// Local price in grain-equivalent (the numeraire) → exchange ratios.
    pub price: f32,
}

/// Per-hub market panel data from the equilibrium solver (Part III).
#[derive(Serialize, Deserialize, Clone)]
pub struct HubMarket {
    /// Food-stock value per capita (food security).
    pub grain_wealth: f32,
    /// Net market earnings per capita (commercial prosperity).
    pub trade_wealth: f32,
    /// Emergent currency goods at this hub (most liquid + stable first).
    pub currency_goods: Vec<String>,
    /// Explained currency goods with liquidity/value/stability + grain price.
    #[serde(default)]
    pub currencies: Vec<HubCurrency>,
    pub prices: Vec<HubMarketGood>,
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
    /// True for the handful of greatest pass-through emporia (most goods routed
    /// THROUGH them). Rendered RED on the Trade Hub layer; these are the "most
    /// important" hubs the rest of the network hangs off.
    #[serde(default)]
    pub emporium: bool,
    // ── Trade statistics (for the hub-click window) ──
    #[serde(default)] pub throughput: f32,   // total goods handled (in + out + transit)
    #[serde(default)] pub exports: f32,      // total goods exported
    #[serde(default)] pub imports: f32,      // total goods imported
    #[serde(default)] pub partners: u32,     // number of trade-graph links (degree)
    #[serde(default)] pub ref_pct: f32,      // throughput as % of the strongest hub
    #[serde(default)] pub nearest_ref: String, // closest real 1450 trade hub
    #[serde(default)] pub monopolies: Vec<String>, // goods this hub dominates
    #[serde(default)] pub koppen: u8,        // climate at the hub cell (for flavour)
    #[serde(default)] pub elevation: f32,    // normalized elevation (terrain flavour)
    #[serde(default)] pub coastal: bool,     // sits on/near the coast
    // ── Social classes (population split) ──
    #[serde(default)] pub nobility: u32,     // wealthy/patrician class
    #[serde(default)] pub merchants: u32,    // trading class
    #[serde(default)] pub commoners: u32,    // everyone else
    #[serde(default)] pub elite_level: f32,  // 0..1 how large the wealthy class is (vs typical)
    #[serde(default)] pub merchant_level: f32, // 0..1 how large the merchant class is
    #[serde(default)] pub top_export: String,  // the good that brings the city the most wealth
    #[serde(default)] pub top_export_share: f32, // its fraction of the hub's total export value
    #[serde(default)] pub luxuries: Vec<HubLuxury>, // luxury demand vs received + price
    /// True when the hub sits on (or beside) the open sea — a real port. A hub on a
    /// closed lake is NOT sea-accessible and can never be an emporium.
    #[serde(default)] pub sea_access: bool,
    /// Where this hub's exports go: per (good → destination hub) with the share %.
    #[serde(default)] pub exports_to: Vec<EconExport>,
    /// Goods the hub demands but can't fully get, each with a plain reason.
    #[serde(default)] pub shortages: Vec<ShortageNote>,
    /// Market panel: equilibrium prices, barter ratios, currency goods,
    /// grain/trade wealth (None for outposts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<HubMarket>,
    pub produces: Vec<EconHubGood>,
    pub receives: Vec<EconReceive>,
}

/// One outbound shipment leg from a hub: which good goes to which hub, how much,
/// and what share of that good's total exports from this hub it represents.
#[derive(Serialize, Deserialize, Clone)]
pub struct EconExport {
    pub good: usize,
    pub good_name: String,
    pub to_hub: u32,
    pub amount: f32,
    pub pct: f32,   // 0..100, share of this good's exports leaving the hub
    pub chain: u32, // index into EconomySnapshot.chains
}

/// A good a hub wants but cannot fully obtain, with the cause classified so the
/// settlement window can explain WHY (the user's "explain why a good can't reach").
#[derive(Serialize, Deserialize, Clone)]
pub struct ShortageNote {
    pub good: usize,
    pub good_name: String,
    pub reason: String,   // "no_supplier" | "unreachable" | "deficit" | "no_port"
    pub severity: f32,    // 0..1 fraction of demand unmet
}

/// World-level facts about one good: who imports/exports the most of it, and which
/// hub + social class craves it most. Drives the route window's header line.
#[derive(Serialize, Deserialize, Clone)]
pub struct GoodStat {
    pub good: usize,
    pub good_name: String,
    pub top_importer: i32,        // hub id (-1 = none)
    pub top_exporter: i32,        // hub id (-1 = none)
    pub biggest_desire_hub: i32,  // hub with the greatest demand
    pub biggest_desire_class: String, // "nobility" | "merchants" | "commoners"
}

/// Aggregate counts/totals for one class of trade node, for the separate-statistics
/// breakdown (trade hubs vs emporiums vs local trade posts).
#[derive(Serialize, Deserialize, Clone)]
pub struct ClassStats {
    pub label: String,     // "hubs" | "emporiums" | "outposts"
    pub count: u32,
    pub population: u64,
    pub throughput: f32,
    pub avg_wealth: f32,
}

/// One luxury good's market state at a hub: how much is demanded, how much actually
/// arrives over the routes, and its delivered price (which climbs when little gets
/// through — scarcity from the difficulty of moving it).
#[derive(Serialize, Deserialize, Clone)]
pub struct HubLuxury {
    pub good: usize,
    pub good_name: String,
    pub demand: f32,
    pub received: f32,
    pub price: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconChainStop {
    pub hub: u32,
    pub price: f32, // price multiplier when the good reaches this stop
    #[serde(default)]
    pub days: f32,  // cumulative travel days from the origin to this stop
    #[serde(default)]
    pub km: f32,    // cumulative distance from the origin to this stop
    // ── Per-stop price breakdown (for the route window's price ladder + story) ──
    #[serde(default)] pub markup: f32,       // merchant resale margin applied at this stop
    #[serde(default)] pub toll: f32,         // toll/tax added crossing into this stop (straits / great hubs)
    #[serde(default)] pub demand_spike: f32, // transient premium from local unmet demand
    #[serde(default)] pub koppen: u8,        // climate at this stop hub (for the narrative)
    #[serde(default)] pub note: String,      // short text reason for the price change here
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconChain {
    pub id: u32,
    pub good: usize,
    pub good_name: String,
    pub stops: Vec<EconChainStop>, // origin → … intermediate hubs … → consumer
    pub points: Vec<[f32; 2]>,     // world coords of each stop (for the overlay)
    #[serde(default)]
    pub days: f32,                 // total travel days origin → consumer
    #[serde(default)]
    pub km: f32,                   // total distance origin → consumer
    #[serde(default)]
    pub value: f32,                // shipment value = amount × delivered price
    #[serde(default)]
    pub mode: u8,                  // dominant transport mode (0 land / 1 sea / 2 river)
}

/// One commodity's value moving across a corridor in one direction (for the
/// cargo-diversity columns in the good-flow panel).
#[derive(Serialize, Deserialize, Clone)]
pub struct CorridorGood {
    pub good: usize,
    pub good_name: String,
    pub value: f32,
}

/// A hub→hub corridor of the trade graph carrying goods in BOTH directions. The
/// good-flow overlay draws one net-direction arrow per corridor (so direction only
/// changes at hubs), and the cargo popup reads `fwd_goods` / `bwd_goods` into two
/// side-by-side columns. `a` < `b` are hub ids; "forward" = a→b.
#[derive(Serialize, Deserialize, Clone)]
pub struct EconCorridor {
    pub a: u32,
    pub b: u32,
    pub points: Vec<[f32; 2]>, // [hub a, hub b] world coords
    pub fwd_value: f32,        // total value flowing a→b
    pub bwd_value: f32,        // total value flowing b→a
    pub fwd_goods: Vec<CorridorGood>, // a→b cargo, ranked by value
    pub bwd_goods: Vec<CorridorGood>, // b→a cargo, ranked by value
    pub days: f32,             // one-way travel days across this corridor
    pub km: f32,
    pub mode: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconChokepoint {
    pub points: Vec<[f32; 2]>, // the coarse edge [from, to]
    pub volume: f32,
    pub share: f32,            // share of total routed volume
    pub name: String,
}

/// A trade region: the square cell-mask territory a hub controls, drawn like the
/// goods overlay so each hub's hinterland is visible as an area.
#[derive(Serialize, Deserialize, Clone)]
pub struct EconRegion {
    pub hub: u32,                // index into `hubs`
    pub name: String,
    pub cells: Vec<[f32; 2]>,    // world coords of each owned coarse cell (top-left)
    pub cell_size: f32,          // coarse cell size in world cells
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EconomySnapshot {
    pub hubs: Vec<EconHub>,
    pub chains: Vec<EconChain>,
    pub chokepoints: Vec<EconChokepoint>,
    pub regions: Vec<EconRegion>,
    #[serde(default)]
    pub corridors: Vec<EconCorridor>,
    #[serde(default)]
    pub good_stats: Vec<GoodStat>,
    #[serde(default)]
    pub class_stats: Vec<ClassStats>,
    pub goods: Vec<String>,
    /// Empty-land sites a wealthy house / large city can colonize with an estate
    /// during the campaign (precomputed here while tile data is available).
    #[serde(default)]
    pub colonizable_sites: Vec<crate::sim::tick::ColonizeSite>,
}

fn grade_name(q: f32) -> &'static str {
    if q >= 0.85 { "Exquisite" }
    else if q >= 0.68 { "Fine" }
    else if q >= 0.50 { "Standard" }
    else if q >= 0.32 { "Common" }
    else { "Coarse" }
}

/// Evocative variety name for a good by quality tier (0..4). Empty for goods
/// without a curated ladder (the generic grade label is shown instead).
fn good_flavor(good_id: &str, q: f32) -> String {
    let tier = if q >= 0.85 { 4 } else if q >= 0.68 { 3 } else if q >= 0.50 { 2 } else if q >= 0.32 { 1 } else { 0 };
    let list: Option<[&str; 5]> = match good_id {
        "wine" => Some(["Vin Ordinaire", "Village", "Estate", "Reserve", "Grand Cru"]),
        "silk" => Some(["Raw Tussah", "Plain Weave", "Damask", "Brocade", "Imperial Sericea"]),
        "spices" => Some(["Market Blend", "Garden", "Highland", "Monsoon", "Royal Stores"]),
        "pepper" => Some(["Common", "Garbled", "Tellicherry", "Malabar Gold", "Royal Long"]),
        "cloves" => Some(["Stem", "Whole Bud", "Hand-Picked", "Mother Clove", "Sultan's Reserve"]),
        "tea" => Some(["Coarse Leaf", "Garden", "Spring Flush", "First Flush", "Imperial Tribute"]),
        "coffee" => Some(["Common", "Plantation", "Highland", "Peaberry", "Heirloom Reserve"]),
        "oliveoil" => Some(["Lampante", "Virgin", "Cold-Pressed", "Extra Virgin", "First Harvest"]),
        "pearls" => Some(["Seed", "Baroque", "Round", "Fine Round", "Orient Lustre"]),
        "gemstones" => Some(["Industrial", "Flawed", "Clean", "Brilliant", "Flawless"]),
        "ruby" => Some(["Opaque", "Included", "Clean", "Pigeon's Blood", "Star Ruby"]),
        "sapphire" => Some(["Pale", "Tinted", "Cornflower", "Royal Blue", "Kashmir"]),
        "emerald" => Some(["Murky", "Jardin", "Clear", "Vivid Green", "Old Mine"]),
        "diamond" => Some(["Bort", "Cape", "Near-Colourless", "Brilliant", "Flawless White"]),
        "amethyst" => Some(["Pale", "Rose de France", "Siberian", "Deep Violet", "Crown"]),
        "topaz" => Some(["Common", "Sherry", "Golden", "Imperial", "Padparadscha"]),
        "gold" => Some(["Placer", "Alloyed", "Refined", "Fine", "Crown Bullion"]),
        "cotton" => Some(["Homespun", "Calico", "Muslin", "Fine Muslin", "Sea-Island"]),
        "wool_fleece" | "wool_llama" => Some(["Coarse", "Carded", "Combed", "Fine Staple", "Vicuña-Soft"]),
        "tobacco" => Some(["Field", "Cured", "Aged", "Prime Leaf", "Vintage Wrapper"]),
        "frankincense" | "incense" => Some(["Common", "Temple", "Pure", "Select", "First Tears"]),
        "cacao" => Some(["Bulk", "Trinitario", "Criollo", "Fine Criollo", "Ancient Cru"]),
        "ceramics" => Some(["Earthenware", "Stoneware", "Glazed", "Porcelain", "Eggshell Porcelain"]),
        "glassware" => Some(["Bottle", "Blown", "Cristallo", "Cut Crystal", "Millefiori"]),
        "furs" => Some(["Pelt", "Winter Coat", "Prime", "Silver", "Imperial Sable"]),
        "amber" => Some(["Cloudy", "Raw", "Clear", "Golden", "Sun-Stone"]),
        "dyes" | "indigo" => Some(["Faded", "Common", "Fast", "Deep", "Royal Tyrian"]),
        _ => None,
    };
    list.map(|l| l[tier].to_string()).unwrap_or_default()
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

/// Precompute empty-land sites a wealthy house / large city can colonize with an
/// estate during the campaign. A coarse single pass over tiles picks productive land
/// a good distance from every settlement, spread out, tagged with a suggested estate
/// kind. Stored in the economy snapshot so the tick sim (which has no WorldBuffer)
/// can plant colonies. See `CampaignSim::maybe_colonize`.
pub(crate) fn compute_colonizable_sites(
    world: &crate::db::world_cache::WorldTiles,
    grid_w: u32, grid_h: u32,
    settlements: &[(f32, f32)],
    base_value: &[f32],
) -> Vec<crate::sim::tick::ColonizeSite> {
    use crate::sim::tick::ColonizeSite;
    if grid_w == 0 || grid_h == 0 { return vec![]; }
    let f = (grid_w / 120).max(2);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let cn = (cw * ch) as usize;
    let (mut is_land, mut fert, mut koppen, mut elev) =
        (vec![false; cn], vec![0.0f32; cn], vec![0u8; cn], vec![0.0f32; cn]);
    // Raw trade-good richness per coarse cell: Σ (belt strength × good base_value).
    let mut traw = vec![0.0f32; cn];
    for cy in 0..ch {
        for cx in 0..cw {
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1);
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1);
            let tile = world.tile((wx / TILE_SIZE) as i32, (wy / TILE_SIZE) as i32);
            let ti = ((wy % TILE_SIZE) * TILE_SIZE + (wx % TILE_SIZE)) as usize;
            let ci = (cy * cw + cx) as usize;
            is_land[ci] = tile.terrain[ti] == 1;
            fert[ci] = tile.fertility[ti];
            koppen[ci] = tile.koppen[ti];
            elev[ci] = tile.elevation[ti];
            let mut tv = 0.0f32;
            for g in 0..base_value.len().min(tile.goods.len()) {
                tv += tile.goods[g][ti] as f32 / 255.0 * base_value[g];
            }
            traw[ci] = tv;
        }
    }
    // Normalize trade richness to 0..1 across all land cells.
    let tmax = traw.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let tval: Vec<f32> = traw.iter().map(|&v| (v / tmax).clamp(0.0, 1.0)).collect();
    let wrap = |x: i32| ((x % cw) + cw) % cw;
    let coastal = |cx: i32, cy: i32| -> bool {
        [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
            let (nx, ny) = (wrap(cx + dx), cy + dy);
            ny >= 0 && ny < ch && !is_land[(ny * cw + nx) as usize]
        })
    };
    // Land→sea CHOKEPOINT: a land cell with open sea on TWO OPPOSITE sides within a
    // few cells — a strait / isthmus / portage where cargo must transship (toll site).
    let sea_within = |cx: i32, cy: i32, dx: i32, dy: i32, r: i32| -> bool {
        (1..=r).any(|k| {
            let (nx, ny) = (wrap(cx + dx * k), cy + dy * k);
            ny >= 0 && ny < ch && !is_land[(ny * cw + nx) as usize]
        })
    };
    let chokepoint = |cx: i32, cy: i32| -> bool {
        let r = 3;
        (sea_within(cx, cy, -1, 0, r) && sea_within(cx, cy, 1, 0, r))
            || (sea_within(cx, cy, 0, -1, r) && sea_within(cx, cy, 0, 1, r))
    };
    let scoords: Vec<(i32, i32)> = settlements.iter()
        .map(|&(sx, sy)| ((sx / f as f32) as i32, (sy / f as f32) as i32)).collect();
    let min_settle_dist = |cx: i32, cy: i32| -> f32 {
        scoords.iter().map(|&(sx, sy)| {
            let mut dx = (cx - sx).abs();
            if dx > cw / 2 { dx = cw - dx; }
            let dy = cy - sy;
            ((dx * dx + dy * dy) as f32).sqrt()
        }).fold(f32::MAX, f32::min)
    };
    // Lowered so sites may sit a little nearer existing cities (more reachable, the
    // "0 in range" fix) while still keeping a buffer so colonies aren't suburbs.
    let min_dist = (cw as f32 * 0.04).max(4.0);
    let mut cands: Vec<(f32, ColonizeSite)> = Vec::new();
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            if !is_land[ci] { continue; }
            // Skip only genuinely worthless flatland. Thresholds lowered so lean but
            // cargo-rich frontier (low fertility, some trade goods) qualifies — the
            // colony/outpost leans on trade, not farmland (user ask).
            if fert[ci] < 0.22 && elev[ci] < 0.45 && tval[ci] < 0.12 { continue; }
            if min_settle_dist(cx, cy) < min_dist { continue; }
            let cst = coastal(cx, cy);
            let kind_hint = if cst && fert[ci] < 0.40 { 4 }
                else if elev[ci] > 0.45 { 2 }
                else if fert[ci] > 0.55 { 1 }
                else { 3 };
            // River-mouth / DELTA proxy: a fertile coastal alluvial cell (natural
            // port + granary). Chokepoint: a land→sea transshipment neck.
            let delta = cst && fert[ci] > 0.55;
            let choke = chokepoint(cx, cy);
            // Rank candidates leaning on the TRADE-GOODS prize over farmland, plus a
            // premium for the historically prized sites: sea shore, river delta, and
            // land→sea chokepoints (transshipment / toll points).
            let score = 0.6 * fert[ci]
                + if elev[ci] > 0.45 { 0.2 } else { 0.0 }
                + if cst { 0.35 } else { 0.0 }
                + if delta { 0.60 } else { 0.0 }
                + if choke { 0.80 } else { 0.0 }
                + 1.1 * tval[ci];
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as f32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as f32;
            cands.push((score, ColonizeSite {
                x: wx, y: wy, koppen: koppen[ci], elevation: elev[ci],
                fertility: fert[ci], coastal: cst, kind_hint, trade_value: tval[ci],
                delta, chokepoint: choke,
            }));
        }
    }
    cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    // Greedy spatial spread so colonies don't all cluster in one fertile belt.
    let spacing = (cw as f32 * 0.06).max(4.0) * f as f32; // world cells
    let mut picked: Vec<ColonizeSite> = Vec::new();
    for (_, s) in cands {
        if picked.len() >= 120 { break; }
        let ok = picked.iter().all(|p| {
            let mut dx = (p.x - s.x).abs();
            if grid_w > 1 { dx = dx.min(grid_w as f32 - dx); }
            let dy = p.y - s.y;
            (dx * dx + dy * dy).sqrt() >= spacing
        });
        if ok { picked.push(s); }
    }
    picked
}

/// Candidate sites for a metropolis SATELLITE (Ostia→Rome, Piraeus→Athens): valid
/// land NEAR an existing city — roughly 30–500 km away, a day's ride rather than an
/// ocean. Unlike `compute_colonizable_sites` (which buffers colony sites ≥~1600 km
/// from every city so colonies aren't suburbs), a satellite is MEANT to hug its
/// metropolis, so this pool deliberately keeps the near-city ring the colony pool
/// excludes. The founder picks the NEAREST, so satellites land close.
pub(crate) fn compute_satellite_sites(
    world: &crate::db::world_cache::WorldTiles,
    grid_w: u32, grid_h: u32,
    settlements: &[(f32, f32)],
    base_value: &[f32],
) -> Vec<crate::sim::tick::ColonizeSite> {
    use crate::sim::tick::ColonizeSite;
    if grid_w == 0 || grid_h == 0 || settlements.is_empty() { return vec![]; }
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let near_min = 30.0 / km_per_cell;   // world cells — not on top of the city
    let near_max = 500.0 / km_per_cell;  // ≤ 500 km from the metropolis (user rule)
    let f = (grid_w / 300).max(2);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let wrap = |x: i32| ((x % cw) + cw) % cw;
    let min_settle_dist = |wx: f32, wy: f32| -> f32 {
        settlements.iter().map(|&(sx, sy)| {
            let mut dx = (wx - sx).abs();
            if dx > grid_w as f32 / 2.0 { dx = grid_w as f32 - dx; }
            let dy = wy - sy;
            (dx * dx + dy * dy).sqrt()
        }).fold(f32::MAX, f32::min)
    };
    let mut cands: Vec<(f32, ColonizeSite)> = Vec::new();
    for cy in 0..ch {
        for cx in 0..cw {
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1);
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1);
            let tile = world.tile((wx / TILE_SIZE) as i32, (wy / TILE_SIZE) as i32);
            let ti = ((wy % TILE_SIZE) * TILE_SIZE + (wx % TILE_SIZE)) as usize;
            if tile.terrain[ti] != 1 { continue; } // land only
            let d = min_settle_dist(wx as f32, wy as f32);
            if d < near_min || d > near_max { continue; }
            let fert = tile.fertility[ti];
            let elev = tile.elevation[ti];
            let mut tv = 0.0f32;
            for g in 0..base_value.len().min(tile.goods.len()) {
                tv += tile.goods[g][ti] as f32 / 255.0 * base_value[g];
            }
            let coastal = [(-1i32, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                let nx = wrap(cx + dx);
                let ny = cy + dy;
                if ny < 0 || ny >= ch { return false; }
                let nwx = (nx as u32 * f + f / 2).min(grid_w - 1);
                let nwy = (ny as u32 * f + f / 2).min(grid_h - 1);
                let nt = world.tile((nwx / TILE_SIZE) as i32, (nwy / TILE_SIZE) as i32);
                let nti = ((nwy % TILE_SIZE) * TILE_SIZE + (nwx % TILE_SIZE)) as usize;
                nt.terrain[nti] == 0
            });
            let kind_hint = if coastal && fert < 0.40 { 4 } else if elev > 0.45 { 2 }
                else if fert > 0.55 { 1 } else { 3 };
            // Prefer CLOSE, fertile, coastal suburb sites; nearer scores higher.
            let score = (0.6 * fert + if coastal { 0.3 } else { 0.0 } + 0.3 * tv)
                * (1.0 - d / near_max);
            cands.push((score, ColonizeSite {
                x: wx as f32, y: wy as f32, koppen: tile.koppen[ti], elevation: elev,
                fertility: fert, coastal, kind_hint, trade_value: tv.min(1.0),
                delta: false, chokepoint: false,
            }));
        }
    }
    cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let spacing = (60.0 / km_per_cell).max(3.0); // ~60 km apart
    let mut picked: Vec<ColonizeSite> = Vec::new();
    for (_, s) in cands {
        if picked.len() >= 240 { break; }
        let ok = picked.iter().all(|p| {
            let mut dx = (p.x - s.x).abs();
            if grid_w > 1 { dx = dx.min(grid_w as f32 - dx); }
            let dy = p.y - s.y;
            (dx * dx + dy * dy).sqrt() >= spacing
        });
        if ok { picked.push(s); }
    }
    picked
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
    piracy: f32,
    season: i32,
    months: u32,
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
    let empty = EconomySnapshot { hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![], corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names.clone(), colonizable_sites: vec![] };
    if grid_w == 0 || grid_h == 0 {
        return Ok(empty);
    }
    let world = db.cached_tiles_with_conn(&conn)?;
    let settlements: Vec<RouteSettlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.len() < 2 {
        let _ = metadata::campaign_set(&conn, "economy", &serde_json::to_string(&empty).unwrap_or_default());
        return Ok(empty);
    }
    // Empty-land colonization candidates for the campaign (uses tile data here).
    let base_value: Vec<f32> = specs.iter().map(|s| s.base_value.max(0.0)).collect();
    let settle_xy: Vec<(f32, f32)> = settlements.iter().map(|s| (s.x as f32, s.y as f32)).collect();
    let colonizable_sites = compute_colonizable_sites(&world, grid_w, grid_h, &settle_xy, &base_value);

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > grid_w as i32 / 2 { d = grid_w as i32 - d; }
        d
    };

    // ── Market nodes: EVERY settlement is an economic node (so any town is
    // clickable and inspectable), sorted by score — strongest first — so the hub
    // topology below mirrors compute_trade_routes exactly. ──
    let er = economic_regions.clamp(2, 40) as usize;
    let mut idx: Vec<usize> = (0..settlements.len()).collect();
    idx.sort_by(|&a, &b| settlements[b].score.partial_cmp(&settlements[a].score)
        .unwrap_or(std::cmp::Ordering::Equal));
    let nodes: Vec<&RouteSettlement> = idx.iter().map(|&i| &settlements[i]).collect();
    let nn = nodes.len();
    if nn < 2 {
        let _ = metadata::campaign_set(&conn, "economy", &serde_json::to_string(&empty).unwrap_or_default());
        return Ok(empty);
    }
    // Major hubs carry the primary network; lesser towns hang off their nearest
    // hub. `territory_n` bounds the (legible) trade-territory overlay.
    let major_n = nn.min((er * 6).clamp(10, 200));
    let territory_n = nn.min((er * 3).clamp(5, 60));

    // Cost grid built WITH the chosen season + piracy so seasonal closures and
    // raiders actually shape trade flows and prices (not just the drawn routes).
    let cc = cached_coarse_cost(&db, &world, world.fingerprint, grid_w, grid_h, &rivers_json, reach == 2, desert_routes, piracy, season, months)?;
    let cnode: Vec<usize> = nodes.iter().map(|s| {
        let cx = (s.x / cc.f).min(cc.cw as u32 - 1) as i32;
        let cy = (s.y / cc.f).min(cc.ch as u32 - 1) as i32;
        cc.cidx(cx, cy)
    }).collect();
    // Sea access (real ocean port vs closed-lake / inland) + outpost flags per node.
    let node_sea: Vec<bool> = nodes.iter().map(|s| {
        let t = world.tile((s.x / TILE_SIZE) as i32, (s.y / TILE_SIZE) as i32);
        let ti = ((s.y % TILE_SIZE) * TILE_SIZE + (s.x % TILE_SIZE)) as usize;
        t.distance_to_ocean.get(ti).map(|&d| d < 0.06).unwrap_or(false)
    }).collect();
    let is_outpost: Vec<bool> = nodes.iter().map(|s| s.size == "outpost").collect();

    // ── Trade-route GRAPH ────────────────────────────────────────────────────
    // Every settlement is wired into the network exactly as compute_trade_routes
    // draws it: each major hub links to its 3 nearest neighbours, and every lesser
    // town links to its nearest major hub. An edge survives only if a real route
    // exists under the chosen reach, so goods can flow ONLY where there is a road /
    // sea-route — never beelining across an ocean that has no route. Each edge
    // carries its routed coarse path (for chokepoints) and least-cost weight.
    let nearest_k = |i: usize, k: usize, pool: usize| -> Vec<usize> {
        let mut d: Vec<(usize, i64)> = Vec::new();
        for j in 0..pool {
            if i == j { continue; }
            let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
            let dy = nodes[i].y as i64 - nodes[j].y as i64;
            d.push((j, dx * dx + dy * dy));
        }
        d.sort_by_key(|&(_, dd)| dd);
        d.iter().take(k).map(|&(j, _)| j).collect()
    };
    // Absolute link ceiling: no trade link may span more than this straight-line
    // distance, so a lone far island never wires itself to a distant continent
    // regardless of reach (the user: "no trade routes to far away islands which do
    // not adhere to trade distance"). The crossing-fraction reach test still applies
    // on top of this via path_allowed.
    let max_link2: i64 = {
        let d = (grid_w as f32 * 0.30).max(60.0);
        (d * d) as i64
    };
    let node_dist2 = |i: usize, j: usize| -> i64 {
        let dx = wrap_dx(nodes[i].x as i32, nodes[j].x as i32) as i64;
        let dy = nodes[i].y as i64 - nodes[j].y as i64;
        dx * dx + dy * dy
    };
    let mut cand: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for i in 0..major_n {
        // 4 nearest neighbours (was 3) so a continent's hub graph is well enough
        // connected to form one trading market — goods can then reach across it.
        for j in nearest_k(i, 4, nn) {
            if node_dist2(i, j) <= max_link2 { cand.insert((i.min(j), i.max(j))); }
        }
    }
    for i in major_n..nn {
        if let Some(&j) = nearest_k(i, 1, major_n).first() {
            if node_dist2(i, j) <= max_link2 { cand.insert((i.min(j), i.max(j))); }
        }
    }
    // Direct maritime bypass legs: a ship can sail between two coastal hubs
    // WITHOUT calling at every intermediate port. Without these the hub graph is
    // only nearest-neighbour, so a long sea haul must chain A→B→C→D and the
    // displayed price ladder stacks a transit toll at every pass-through city —
    // the "price balloons jumping city to city" the user reported. Connect each
    // coastal major hub to its nearest OTHER coastal hubs (even distant ones,
    // within the link ceiling); the routed path between two ports is naturally
    // sea-dominant, and `path_allowed` below still drops crossings the chosen
    // trade reach forbids. The shortest-path tree then prefers the direct leg
    // when it beats the tolled chain, so the rebuilt chain has no pass-through
    // toll hubs to compound.
    {
        let coastal: Vec<usize> = (0..nn).filter(|&i| node_sea[i]).collect();
        for &i in coastal.iter().filter(|&&i| i < major_n) {
            let mut d: Vec<(usize, i64)> = coastal.iter().filter(|&&j| j != i)
                .map(|&j| (j, node_dist2(i, j))).collect();
            d.sort_by_key(|&(_, dd)| dd);
            for &(j, dd) in d.iter().take(5) {
                if dd <= max_link2 { cand.insert((i.min(j), i.max(j))); }
            }
        }
    }
    // Materialise each candidate as a routed edge → adjacency + per-edge coarse path.
    let mut adj: Vec<Vec<(usize, f32, usize)>> = vec![Vec::new(); nn]; // (to, cost, edge_id)
    let mut edge_paths: Vec<Vec<usize>> = Vec::new();
    for &(a, b) in &cand {
        let path = match coarse_dijkstra(&cc, cnode[a], cnode[b]) { Some(p) => p, None => continue };
        if !path_allowed(&cc, &path, reach, max_crossing, grid_w) { continue; }
        let cost = coarse_path_cost(&cc, &path).max(0.01);
        let eid = edge_paths.len();
        edge_paths.push(path);
        adj[a].push((b, cost, eid));
        adj[b].push((a, cost, eid));
    }
    // Centrality = reachable degree in the trade graph.
    let centrality: Vec<f32> = (0..nn).map(|i| adj[i].len() as f32).collect();
    // Connected components: a flow may only run between settlements that share a
    // trade-network component → no unreachable cross-ocean origins, ever.
    let mut comp = vec![usize::MAX; nn];
    {
        let mut c = 0usize;
        for s in 0..nn {
            if comp[s] != usize::MAX { continue; }
            comp[s] = c;
            let mut stack = vec![s];
            while let Some(u) = stack.pop() {
                for &(v, _, _) in &adj[u] {
                    if comp[v] == usize::MAX { comp[v] = c; stack.push(v); }
                }
            }
            c += 1;
        }
    }
    // All-pairs cheapest route over the (small) graph: cost + parent for rebuilding
    // the actual road a shipment travels.
    let mut gdist: Vec<Vec<f32>> = vec![vec![f32::INFINITY; nn]; nn];
    let mut gpar: Vec<Vec<usize>> = vec![vec![usize::MAX; nn]; nn];
    for s in 0..nn {
        gdist[s][s] = 0.0;
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, s)));
        let mut done = vec![false; nn];
        while let Some(Reverse((_, u))) = heap.pop() {
            if done[u] { continue; }
            done[u] = true;
            let du = gdist[s][u];
            for &(v, w, _) in &adj[u] {
                let nd = du + w;
                if nd < gdist[s][v] {
                    gdist[s][v] = nd;
                    gpar[s][v] = u;
                    heap.push(Reverse(((nd * 1000.0) as i64, v)));
                }
            }
        }
    }

    // ── Territory overlay grid: each land coarse cell → nearest MAJOR hub. This is
    // the legible trade-REGION overlay ONLY (kept broad per major hub so regions read
    // cleanly); the actual goods COLLECTION below is bounded by a small per-hub radius.
    let f = (grid_w / 220).max(1);
    let cw = ((grid_w + f - 1) / f) as i32;
    let ch = ((grid_h + f - 1) / f) as i32;
    let mut owner = vec![u16::MAX; (cw * ch) as usize];
    for cy in 0..ch {
        for cx in 0..cw {
            let ci = (cy * cw + cx) as usize;
            let wx = (cx as u32 * f + f / 2).min(grid_w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(grid_h - 1) as i32;
            let mut best_t = 0usize;
            let mut bd_t = i64::MAX;
            for (ni, s) in nodes.iter().enumerate().take(territory_n) {
                let dx = wrap_dx(wx, s.x as i32) as i64;
                let dy = (wy - s.y as i32) as i64;
                let d = dx * dx + dy * dy;
                if d < bd_t { bd_t = d; best_t = ni; }
            }
            let tx = (wx as u32 / TILE_SIZE) as i32;
            let ty = (wy as u32 / TILE_SIZE) as i32;
            let lt = world.tile(tx, ty);
            let ti = ((wy as u32 % TILE_SIZE) * TILE_SIZE + (wx as u32 % TILE_SIZE)) as usize;
            if lt.terrain[ti] != 0 { owner[ci] = best_t as u16; }
        }
    }

    // ── Production: each hub gathers goods from a BOUNDED radius — ≈50 km for the
    // smallest towns / outposts, scaling up to ≈120 km for a great metropolis (user
    // rule). Replaces the old unbounded nearest-hub Voronoi, so a lone city no longer
    // harvests half a continent. Cells beyond EVERY hub's radius stay UNCLAIMED (a
    // colonization prize that feeds the colony/outpost site scoring). Overlapping
    // radii resolve to the nearest hub. Marine goods on nearby sea are collected too
    // (a port works its own fishery), so no land-only filter here.
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let hub_radius_cells: Vec<i32> = (0..nn)
        .map(|hh| {
            let km = if is_outpost[hh] {
                50.0
            } else {
                // log-scaled 50→120 km across ~500 … ~100k population (ln 6.2 … 11.5).
                let t = ((nodes[hh].population as f32).max(1.0).ln() - 6.2) / (11.5 - 6.2);
                50.0 + t.clamp(0.0, 1.0) * (120.0 - 50.0)
            };
            ((km / km_per_cell).round() as i32).max(1)
        })
        .collect();
    // cell key (wy*grid_w + wx) → (best distance², owning hub).
    let mut claim: std::collections::HashMap<u64, (i64, u32)> = std::collections::HashMap::new();
    for hh in 0..nn {
        let r = hub_radius_cells[hh];
        let r2 = (r as i64) * (r as i64);
        let (sx, sy) = (nodes[hh].x as i32, nodes[hh].y as i32);
        for dy in -r..=r {
            let wy = sy + dy;
            if wy < 0 || wy >= grid_h as i32 { continue; }
            let dy2 = (dy as i64) * (dy as i64);
            for dx in -r..=r {
                let d2 = (dx as i64) * (dx as i64) + dy2;
                if d2 > r2 { continue; }
                let wx = (((sx + dx) % grid_w as i32) + grid_w as i32) % grid_w as i32;
                let key = (wy as u64) * (grid_w as u64) + wx as u64;
                let e = claim.entry(key).or_insert((i64::MAX, u32::MAX));
                if d2 < e.0 { *e = (d2, hh as u32); }
            }
        }
    }
    let mut prod = vec![vec![0.0f32; gc]; nn];
    for (&key, &(_, hh)) in claim.iter() {
        if hh == u32::MAX { continue; }
        let wx = (key % grid_w as u64) as u32;
        let wy = (key / grid_w as u64) as u32;
        let tile = world.tile((wx / TILE_SIZE) as i32, (wy / TILE_SIZE) as i32);
        let ti = ((wy % TILE_SIZE) * TILE_SIZE + (wx % TILE_SIZE)) as usize;
        for g in 0..gc.min(tile.goods.len()) {
            prod[hh as usize][g] += tile.goods[g][ti] as f32 / 255.0;
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
    let pop_max = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    let mut power = vec![0.0f32; nn];
    for hh in 0..nn {
        // POPULATION is now a primary driver so the greatest trade hub is a big,
        // central city — not a tiny far-flung island that merely monopolises one
        // good (the user's "largest city on a far island" bug). Population + route
        // centrality dominate; monopoly is a smaller tie-breaker.
        let pop_n = (nodes[hh].population as f32 / pop_max).clamp(0.0, 1.0).sqrt();
        let mut p = 0.26 * nodes[hh].score.clamp(0.0, 1.0)
            + 0.30 * pop_n
            + 0.30 * (centrality[hh] / cmax)
            + 0.14 * (monopoly[hh] / mmax);
        // Sea ports outgrow lake/inland towns as great hubs; outposts never rank.
        if node_sea[hh] { p *= 1.18; } else { p *= 0.72; }
        if is_outpost[hh] { p *= 0.05; }
        power[hh] = p;
    }
    let pmax = power.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // ── Demand: population-scaled per-good desire (+ mercantile/subsistence) ──
    // Unified with compute_trade_matrix: a full-basket floor, a reach factor (how
    // open the network is), a homeland discount (a good is common/cheap where it is
    // produced, so its own region barely imports it) and a wealth feedback (richer
    // markets crave luxuries more). Staples are unaffected by reach/luxury terms.
    let max_pop = nodes.iter().map(|s| s.population).max().unwrap_or(1).max(1) as f32;
    let desire: Vec<f32> = (0..gc)
        .map(|g| specs.get(g).filter(|s| s.enabled).map(|s| s.desire).unwrap_or(0.0))
        .collect();
    let is_luxury: Vec<bool> = (0..gc)
        .map(|g| specs.get(g).map(|s| s.network_luxury).unwrap_or(false))
        .collect();
    let lux_mult = (0.6 + luxury_bias.clamp(0.0, 1.0) * 1.4).clamp(0.3, 2.2);
    let reach_factor = match reach { 0 => 1.0, 1 => 0.7, _ => 0.45 };
    // Income proxy (pre-flow): value of what a hub produces. Richer hubs demand more
    // luxuries. Normalized 0..1.
    let income: Vec<f32> = (0..nn)
        .map(|hh| (0..gc).map(|g| prod[hh][g] * desire[g]).sum::<f32>())
        .collect();
    let inc_max = income.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    const BASKET_FLOOR: f32 = 0.35; // every good is at least modestly desired
    let mut net = vec![vec![0.0f32; gc]; nn];
    let mut demand_v = vec![vec![0.0f32; gc]; nn]; // per-hub per-good demand (for the class/luxury panel)
    for hh in 0..nn {
        let size = (nodes[hh].population as f32 / max_pop).max(0.25);
        // Prosperity: a wealthy city is a hungry market. Demand across the WHOLE
        // basket (not just luxuries) scales with the city's income/production
        // value, so richer hubs import more of everything (user request: "the more
        // wealthy the city is, the more demand"). A poor town gets ~0.55× the base,
        // a rich metropolis ~1.2×.
        let income_n = (income[hh] / inc_max).clamp(0.0, 1.0);
        let prosperity = size * (0.55 + 0.65 * income_n);
        for g in 0..gc {
            let mut d = prosperity * desire[g].max(BASKET_FLOOR);
            if is_luxury[g] {
                // `prod` is already rescaled to 0..abundance (max producer ≈ the
                // good's abundance), so use it directly as the local-share input —
                // identical to compute_trade_matrix's homeland discount.
                let homeland_discount = 1.0 - 0.6 * prod[hh][g].clamp(0.0, 1.0);
                // ELITE-DEMAND SYSTEM: luxuries are conspicuous consumption. A city's
                // appetite for them scales SUPER-LINEARLY with its prosperity — a
                // rich metropolis has a large patrician/court class that craves silk,
                // spices, gems and wine far out of proportion to its size, while a
                // poor town buys almost none. (income_n^1.4 → 0.35 poor … ~2.0 rich.)
                let income_n = (income[hh] / inc_max).clamp(0.0, 1.0);
                let elite = 0.35 + 1.65 * income_n.powf(1.4);
                d *= reach_factor * lux_mult * homeland_discount * elite;
            }
            demand_v[hh][g] = d;
            net[hh][g] = prod[hh][g] - d;
        }
    }

    // ── Market equilibrium (Part III): stock-based local prices in the GRAIN
    // numeraire replace per-hop markup compounding. The solver runs over the
    // SAME trade graph; freight is an additive cost per travel day, so
    // remoteness becomes a decaying price gradient instead of a capped
    // multiplier (the old ×9.6 plateau). ──
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let edge_days: Vec<f32> = edge_paths
        .iter()
        .map(|p| path_metrics(&cc, p, km_per_cell).1)
        .collect();
    // All-pairs travel DAYS over the graph (same Dijkstra as gdist, day weights).
    let mut gdays: Vec<Vec<f32>> = vec![vec![f32::INFINITY; nn]; nn];
    for src in 0..nn {
        gdays[src][src] = 0.0;
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((0, src)));
        let mut done = vec![false; nn];
        while let Some(Reverse((_, u))) = heap.pop() {
            if done[u] { continue; }
            done[u] = true;
            let du = gdays[src][u];
            for &(v, _, eid) in &adj[u] {
                let nd = du + edge_days[eid];
                if nd < gdays[src][v] {
                    gdays[src][v] = nd;
                    heap.push(Reverse(((nd * 1000.0) as i64, v)));
                }
            }
        }
    }
    // Intern good categories. The solver's food convention: ids 0..=3 are
    // cereal/protein/oil/sweetener (they feed grain_wealth).
    fn intern_cat(cat_ids: &mut Vec<String>, c: &str) -> usize {
        if c.is_empty() {
            return usize::MAX;
        }
        if let Some(i) = cat_ids.iter().position(|x| x == c) {
            i
        } else {
            cat_ids.push(c.to_string());
            cat_ids.len() - 1
        }
    }
    let mut cat_ids: Vec<String> =
        ["cereal", "protein", "oil", "sweetener"].iter().map(|s| s.to_string()).collect();
    let mgoods: Vec<market::MarketGood> = (0..gc)
        .map(|g| market::MarketGood {
            category: intern_cat(&mut cat_ids, &specs[g].category),
            need_tier: specs[g].need_tier,
            base_value: specs[g].base_value.max(0.05),
            desire: if specs[g].enabled { specs[g].desire.max(0.0) } else { 0.0 },
            bulk: specs[g].bulk.max(0.0),
            perishable: specs[g].perishable.max(0.0),
        })
        .collect();
    // Production chains: cities transform imported raws into finished exports
    // (wool→cloth, ore→arms). Run the shared resolver on the static per-hub
    // production BEFORE the market solves, so manufactured goods flow as exports
    // from populous hubs that hold the inputs.
    let hub_pop: Vec<f32> = (0..nn).map(|hh| nodes[hh].population.max(1) as f32).collect();
    let _manu_warnings =
        crate::sim::manufacture::apply_manufacturing(&mut prod, &specs, &hub_pop);
    let mhubs: Vec<market::MarketHub> = (0..nn)
        .map(|hh| market::MarketHub {
            population: hub_pop[hh],
            production: prod[hh].clone(),
        })
        .collect();
    let mut routes_m = market::RouteMatrix::new(nn);
    for a in 0..nn {
        for b in (a + 1)..nn {
            if comp[a] == comp[b] && gdays[a][b].is_finite() {
                routes_m.set(a, b, gdays[a][b], 0.0);
            }
        }
    }
    let mparams = market::MarketParams::default();
    let mut mkt = market::solve(&mhubs, &mgoods, &routes_m, &mparams);

    // GUILD manufacture from IMPORTS (the demand→production→raw-demand loop).
    // The first `apply_manufacturing` only consumed each hub's LOCAL production.
    // Now that the market has delivered raws, a hub's post-trade STOCK holds the
    // wool / salt / ore it imported, so its manufacture guilds can turn those into
    // finished exports too — e.g. a herring port that imports SALT now cures
    // SALTED HERRING, and the demand for it pulls more salt + herring in. We
    // manufacture from post-trade stock, credit the extra finished output to
    // production, and re-solve ONCE so the new goods flow out.
    {
        let mut avail: Vec<Vec<f32>> = mkt.stocks.clone();
        let before: Vec<Vec<f32>> = avail.clone();
        crate::sim::manufacture::apply_manufacturing(&mut avail, &specs, &hub_pop);
        let mut changed = false;
        for hh in 0..nn {
            for g in 0..gc {
                if matches!(specs[g].distribution, crate::sim::goods_spec::Distribution::Manufactured) {
                    let extra = avail[hh][g] - before[hh].get(g).copied().unwrap_or(0.0);
                    if extra > 1e-3 {
                        prod[hh][g] += extra;
                        changed = true;
                    }
                }
            }
        }
        if changed {
            let mhubs2: Vec<market::MarketHub> = (0..nn)
                .map(|hh| market::MarketHub { population: hub_pop[hh], production: prod[hh].clone() })
                .collect();
            mkt = market::solve(&mhubs2, &mgoods, &routes_m, &mparams);
        }
    }

    // Climate at each hub cell (for the merchant-story narrative + price notes).
    let node_koppen: Vec<u8> = nodes.iter().map(|s| {
        let t = world.tile((s.x / TILE_SIZE) as i32, (s.y / TILE_SIZE) as i32);
        let ti = ((s.y % TILE_SIZE) * TILE_SIZE + (s.x % TILE_SIZE)) as usize;
        t.koppen.get(ti).copied().unwrap_or(0)
    }).collect();
    // Toll hubs: the great sea markets (top territory_n that are sea ports) levy a
    // transit toll on goods passing THROUGH them, the way Venice / Constantinople did.
    let is_toll_hub: Vec<bool> = (0..nn).map(|h| h < territory_n && node_sea[h]).collect();
    // Narrow-water STRAIT cells (opposing land within `strait_r`) over the coarse cost
    // grid — a route threading one of these pays a strait toll (Sound Dues, etc.).
    let strait_r = 6i32;
    let strait_cell: Vec<bool> = {
        let land_within = |cx: i32, cy: i32, dirx: i32, diry: i32| -> bool {
            for k in 1..=strait_r {
                let nx = cx + dirx * k;
                let ny = cy + diry * k;
                if ny < 0 || ny >= cc.ch { return false; }
                if cc.is_land[cc.cidx(nx, ny)] { return true; }
            }
            false
        };
        let mut sc = vec![false; (cc.cw * cc.ch) as usize];
        for cy in 0..cc.ch {
            for cx in 0..cc.cw {
                let ci = (cy * cc.cw + cx) as usize;
                if cc.is_land[ci] { continue; }
                let ew = land_within(cx, cy, -1, 0) && land_within(cx, cy, 1, 0);
                let ns = land_within(cx, cy, 0, -1) && land_within(cx, cy, 0, 1);
                if ew || ns { sc[ci] = true; }
            }
        }
        sc
    };

    // ── Flows over the trade GRAPH → chains with per-hop prices + chokepoints ──
    // A deficit buys a good from the cheapest-by-ROUTE supplier in the SAME network
    // component (so the origin is always genuinely reachable over real roads). The
    // goods ride the actual route network (settlement → hub → … → consumer); price
    // rises per hop; and import demand is PRICE-ELASTIC — dear, far-hauled goods are
    // bought in smaller volume, so a market only reaches overseas for what it truly
    // cannot get nearer.
    let mut chains: Vec<EconChain> = Vec::new();
    // Per-hub outbound export legs (origin → consumer) for the export %/arrows panel.
    let mut export_acc: Vec<Vec<(usize, usize, f32, u32)>> = vec![Vec::new(); nn]; // (good, to_hub, amt, chain)
    let mut receives: Vec<Vec<EconReceive>> = vec![Vec::new(); nn];
    let mut recv_amt = vec![vec![0.0f32; gc]; nn]; // per-hub per-good received (for the panel)
    let mut exports = vec![0.0f32; nn];
    let mut imports = vec![0.0f32; nn];
    let mut edge_vol: std::collections::HashMap<(usize, usize), (f32, usize)> =
        std::collections::HashMap::new(); // coarse-edge volume + dominant good

    // Travel scale + directional corridor accumulation for the good-flow panel.
    let km_per_cell = KM_EQUATOR / grid_w.max(1) as f32;
    let mut transit = vec![0.0f32; nn]; // goods passing THROUGH a hub (emporium score)
    #[derive(Default)]
    struct CorridorAcc {
        fwd_value: f32,
        bwd_value: f32,
        fwd_good: std::collections::HashMap<usize, f32>,
        bwd_good: std::collections::HashMap<usize, f32>,
        days: f32,
        km: f32,
        mode: u8,
    }
    let mut corridor_acc: std::collections::HashMap<(usize, usize), CorridorAcc> =
        std::collections::HashMap::new();

    // The solver's shipment lanes ARE the chains: each flow rides the real
    // route graph; the per-stop price ladder is the ADDITIVE delivered cost
    // (origin grain-eq price + freight per travel day, × tolls) shown as a
    // multiplier vs the origin — replacing per-hop markup compounding.
    let mut lanes: Vec<&market::MarketFlow> =
        mkt.flows.iter().filter(|fl| fl.amount > 0.01).collect();
    // Largest lanes first per good, capped so the snapshot stays legible.
    lanes.sort_by(|a, b| {
        a.good.cmp(&b.good).then(
            b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut lanes_per_good = vec![0u32; gc];

    for fl in lanes {
        let (si, di, g, amt) = (fl.from, fl.to, fl.good, fl.amount);
        if lanes_per_good[g] >= 220 {
            continue;
        }
        if !gdist[di][si].is_finite() {
            continue;
        }
        // Rebuild the graph path si → di (the real road the goods travel).
        // gpar[di] is the shortest-path tree rooted at di, so walking from
        // si via gpar[di][·] yields si … di directly.
        let mut path: Vec<usize> = Vec::new();
        let mut t = si;
        while t != usize::MAX {
            path.push(t);
            if t == di { break; }
            t = gpar[di][t];
        }
        if path.last() != Some(&di) {
            continue;
        }
        lanes_per_good[g] += 1;
        exports[si] += amt;
        imports[di] += amt;
        recv_amt[di][g] += amt;

        let chain_id = chains.len() as u32;
        // Goods passing THROUGH intermediate hubs make those hubs emporia.
        for k in 1..path.len().saturating_sub(1) { transit[path[k]] += amt; }
        let mut cum_km = 0.0f32;
        let mut cum_days = 0.0f32;
        let mut modes = [0u32; 3];
        let mut chain_stops: Vec<EconChainStop> = Vec::with_capacity(path.len());
        let mut points: Vec<[f32; 2]> = Vec::with_capacity(path.len());
        let origin_price = mkt.prices[si][g].max(1e-3);
        let mut delivered = origin_price; // grain-eq, accrues freight + tolls
        let last_k = path.len().saturating_sub(1);
        for (k, &h) in path.iter().enumerate() {
            let (mut leg_km, mut leg_days, mut leg_mode) = (0.0f32, 0.0f32, 0u8);
            let (mut freight_k, mut toll_k, mut crossed_strait) = (0.0f32, 0.0f32, false);
            if k > 0 {
                let prev = path[k - 1];
                if let Some(&(_, _, eid)) = adj[prev].iter().find(|&&(v, _, _)| v == h) {
                    if let Some(p) = edge_paths.get(eid) {
                        for win in p.windows(2) {
                            let e = (win[0].min(win[1]), win[0].max(win[1]));
                            edge_vol.entry(e).or_insert((0.0, g)).0 += amt;
                        }
                        if p.iter().any(|&c| strait_cell[c]) { crossed_strait = true; }
                        let (lk, ld, lm) = path_metrics(&cc, p, km_per_cell);
                        leg_km = lk; leg_days = ld; leg_mode = lm;
                        cum_km += lk; cum_days += ld; modes[lm as usize] += 1;
                    }
                }
                // Additive freight for this leg, then tolls on the value so far.
                freight_k = mparams.freight_per_day * leg_days;
                delivered += freight_k;
                if crossed_strait { toll_k += 0.30; }                 // strait dues
                if k < last_k && is_toll_hub[h] { toll_k += 0.20; }   // great-hub transit toll
                delivered *= 1.0 + toll_k;
            }
            // Unmet local need still reads as a "demand spike" in the story,
            // but it no longer multiplies the price — cost does.
            let demand_spike = if k > 0 { (mkt.unmet[h][g] * 0.45).clamp(0.0, 0.9) } else { 0.0 };
            let note = if k == 0 {
                "bought at source".to_string()
            } else {
                let mut parts: Vec<&str> = vec!["freight"];
                if crossed_strait { parts.push("strait toll"); }
                if k < last_k && is_toll_hub[h] { parts.push("hub transit toll"); }
                if demand_spike > 0.25 { parts.push("high local demand"); }
                parts.join(" · ")
            };
            chain_stops.push(EconChainStop {
                hub: h as u32,
                price: delivered / origin_price, // multiplier vs origin (UI compat)
                days: cum_days, km: cum_km,
                markup: freight_k / origin_price, // the leg's freight, as a fraction
                toll: toll_k, demand_spike,
                koppen: node_koppen[h], note,
            });
            points.push([nodes[h].x as f32, nodes[h].y as f32]);
            // Directional corridor cargo, valued at the delivered grain-eq cost.
            if k > 0 {
                let prev = path[k - 1];
                let (na, nb) = (prev.min(h), prev.max(h));
                let value_here = amt * delivered;
                let acc = corridor_acc.entry((na, nb)).or_default();
                if prev < h {
                    acc.fwd_value += value_here;
                    *acc.fwd_good.entry(g).or_insert(0.0) += value_here;
                } else {
                    acc.bwd_value += value_here;
                    *acc.bwd_good.entry(g).or_insert(0.0) += value_here;
                }
                acc.days = leg_days; acc.km = leg_km; acc.mode = leg_mode;
            }
        }
        let mult = chain_stops.last().map(|st| st.price).unwrap_or(1.0);
        let total_days = chain_stops.last().map(|st| st.days).unwrap_or(0.0);
        let total_km = chain_stops.last().map(|st| st.km).unwrap_or(0.0);
        let dom_mode = (0..3usize).max_by_key(|&i| modes[i]).unwrap_or(0) as u8;
        receives[di].push(EconReceive {
            good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
            amount: amt, price: mult, chain: chain_id, from_hub: si as u32,
        });
        chains.push(EconChain {
            id: chain_id, good: g,
            good_name: goods_names.get(g).cloned().unwrap_or_default(),
            stops: chain_stops, points,
            days: total_days, km: total_km, value: amt * delivered, mode: dom_mode,
        });
        export_acc[si].push((g, di, amt, chain_id));
    }

    // ── Export destinations + shares per hub ──
    let mut exports_to_v: Vec<Vec<EconExport>> = vec![Vec::new(); nn];
    for si in 0..nn {
        // Total exported per good from this hub (for the percentage).
        let mut good_tot: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for &(g, _, amt, _) in &export_acc[si] { *good_tot.entry(g).or_insert(0.0) += amt; }
        let mut v: Vec<EconExport> = export_acc[si].iter().map(|&(g, di, amt, ch)| {
            let tot = good_tot.get(&g).copied().unwrap_or(amt).max(1e-6);
            EconExport {
                good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                to_hub: di as u32, amount: amt, pct: (amt / tot * 100.0).clamp(0.0, 100.0), chain: ch,
            }
        }).collect();
        v.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        exports_to_v[si] = v;
    }
    // Per-(hub,good) exported amount (for good_stats top-exporter).
    let mut exp_by: Vec<Vec<f32>> = vec![vec![0.0f32; gc]; nn];
    for si in 0..nn { for &(g, _, amt, _) in &export_acc[si] { exp_by[si][g] += amt; } }

    // ── Shortages: goods a hub demands but can't fully obtain, with the reason ──
    // Reuses the component map + graph distances so the cause is accurate: produced
    // nowhere reachable, blocked across water, no port for a sea-borne good, or simply
    // out-produced by demand.
    let mut shortages_v: Vec<Vec<ShortageNote>> = vec![Vec::new(); nn];
    for hh in 0..nn {
        if is_outpost[hh] { continue; }
        let mut notes: Vec<(f32, ShortageNote)> = Vec::new();
        for g in 0..gc {
            let dmd = demand_v[hh][g];
            if dmd < 0.08 { continue; }
            if net[hh][g] >= 0.0 { continue; } // self-sufficient or a net producer
            let got = recv_amt[hh][g];
            let severity = (1.0 - got / dmd).clamp(0.0, 1.0);
            if severity < 0.45 { continue; }
            // Is there any reachable supplier (same component, finite graph distance)?
            let reachable = (0..nn).any(|si| si != hh && net[si][g] > 0.02
                && comp[si] == comp[hh] && gdist[hh][si].is_finite());
            let any_producer = (0..nn).any(|si| prod[si][g] > 0.02);
            let reason = if !any_producer { "no_supplier" }
                else if !reachable { if node_sea[hh] { "unreachable" } else { "no_port" } }
                else { "deficit" };
            notes.push((severity * (0.4 + desire[g]), ShortageNote {
                good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                reason: reason.to_string(), severity,
            }));
        }
        notes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        shortages_v[hh] = notes.into_iter().take(6).map(|(_, n)| n).collect();
    }

    // ── Per-good world statistics: top importer/exporter + which class craves it ──
    let good_stats: Vec<GoodStat> = (0..gc).filter_map(|g| {
        let top_imp = (0..nn).filter(|&h| recv_amt[h][g] > 0.0)
            .max_by(|&a, &b| recv_amt[a][g].partial_cmp(&recv_amt[b][g]).unwrap_or(std::cmp::Ordering::Equal));
        let top_exp = (0..nn).filter(|&h| exp_by[h][g] > 0.0)
            .max_by(|&a, &b| exp_by[a][g].partial_cmp(&exp_by[b][g]).unwrap_or(std::cmp::Ordering::Equal));
        let top_dmd = (0..nn).max_by(|&a, &b| demand_v[a][g].partial_cmp(&demand_v[b][g]).unwrap_or(std::cmp::Ordering::Equal));
        if top_imp.is_none() && top_exp.is_none() { return None; }
        // Which social class most desires this good: luxuries are the nobility's
        // conspicuous consumption; high-value trade goods the merchants'; bulk the
        // commoners'.
        let class = if is_luxury[g] { "nobility" }
            else if desire[g] >= 0.5 { "merchants" } else { "commoners" };
        Some(GoodStat {
            good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
            top_importer: top_imp.map(|h| h as i32).unwrap_or(-1),
            top_exporter: top_exp.map(|h| h as i32).unwrap_or(-1),
            biggest_desire_hub: top_dmd.map(|h| h as i32).unwrap_or(-1),
            biggest_desire_class: class.to_string(),
        })
    }).collect();

    // ── Wealth per hub: grain wealth (food security) + trade wealth (market
    // earnings) from the equilibrium, plus a small centrality term so pure
    // crossroads towns don't read as paupers. Normalized for the UI. ──
    let mut wealth = vec![0.0f32; nn];
    for hh in 0..nn {
        let m = &mkt.hubs[hh];
        wealth[hh] = m.grain_wealth.max(0.0)
            + 1.5 * m.trade_wealth.max(0.0)
            + 0.25 * (centrality[hh] / cmax);
    }
    let wmax = wealth.iter().cloned().fold(0.0f32, f32::max).max(1e-6);

    // ── Directional corridors (hub↔hub) for the good-flow panel ──
    // Each corridor carries goods both ways; the overlay draws ONE net-direction
    // arrow per corridor (so direction only changes at hubs), and the cargo popup
    // reads the two per-direction good lists into side-by-side columns.
    let mut corridors: Vec<EconCorridor> = Vec::new();
    for ((na, nb), acc) in corridor_acc.into_iter() {
        let mut fwd_goods: Vec<CorridorGood> = acc.fwd_good.into_iter()
            .map(|(g, v)| CorridorGood { good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(), value: v })
            .collect();
        fwd_goods.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        let mut bwd_goods: Vec<CorridorGood> = acc.bwd_good.into_iter()
            .map(|(g, v)| CorridorGood { good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(), value: v })
            .collect();
        bwd_goods.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        corridors.push(EconCorridor {
            a: na as u32, b: nb as u32,
            points: vec![[nodes[na].x as f32, nodes[na].y as f32], [nodes[nb].x as f32, nodes[nb].y as f32]],
            fwd_value: acc.fwd_value, bwd_value: acc.bwd_value,
            fwd_goods, bwd_goods, days: acc.days, km: acc.km, mode: acc.mode,
        });
    }

    // ── Emporia: the few hubs the most goods are routed THROUGH (pass-through
    // transit volume). These are flagged so the Trade Hub layer can mark them red
    // as the "most important" hubs. Top ~8, and only ones with real throughput. ──
    let emporium_set: std::collections::HashSet<usize> = {
        let mut e: Vec<(f32, usize)> = (0..nn)
            .filter(|&h| node_sea[h] && !is_outpost[h]) // emporia are sea ports, never lake towns/outposts
            .map(|h| (transit[h], h))
            .filter(|&(v, _)| v > 0.0).collect();
        e.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        e.into_iter().take(8).map(|(_, h)| h).collect()
    };

    // Per-hub trade-statistics for the click window.
    let throughput_v: Vec<f32> = (0..nn).map(|h| exports[h] + imports[h] + transit[h]).collect();
    let tp_max = throughput_v.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let trade_max = (0..nn).map(|h| exports[h] + imports[h]).fold(0.0f32, f32::max).max(1e-6);
    // Monopolies: goods this hub is the dominant producer of (≥45% of world output).
    let monopolies_v: Vec<Vec<String>> = (0..nn).map(|hh| {
        let mut m: Vec<(f32, usize)> = (0..gc)
            .filter(|&g| good_total[g] > 1e-4 && prod[hh][g] > 0.1
                && prod[hh][g] / good_total[g] >= 0.45)
            .map(|g| (prod[hh][g] / good_total[g], g)).collect();
        m.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        m.into_iter().take(4).map(|(_, g)| goods_names.get(g).cloned().unwrap_or_default()).collect()
    }).collect();

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
                let id = goods_names.get(g).cloned().unwrap_or_default();
                let flavor = good_flavor(&id, q);
                EconHubGood {
                    good: g, good_name: id, amount: prod[hh][g], quality: q,
                    grade: grade_name(q).to_string(), flavor,
                    // Local equilibrium price in grain-equivalent (quality
                    // nudges the realized sale price a little).
                    price: mkt.prices[hh][g] * (0.9 + 0.25 * q),
                }
            }).collect();
        produces.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
        let mut recv = receives[hh].clone();
        recv.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));

        // ── Social classes: a population split driven by wealth and trade. The
        // wealthy/patrician class grows with the hub's relative wealth; the merchant
        // class with its trade activity (route centrality + total trade volume).
        let pop = nodes[hh].population as f32;
        let wealth_n = (wealth[hh] / wmax).clamp(0.0, 1.0);
        let cent_n = (centrality[hh] / cmax).clamp(0.0, 1.0);
        let trade_n = ((exports[hh] + imports[hh]) / trade_max).clamp(0.0, 1.0);
        let f_nob = 0.003 + 0.030 * wealth_n;            // 0.3%..3.3% of the populace
        let f_mer = 0.010 + 0.060 * cent_n + 0.040 * trade_n; // ~1%..11%
        let nobility = (pop * f_nob) as u32;
        let merchants = (pop * f_mer) as u32;
        let commoners = nodes[hh].population.saturating_sub(nobility).saturating_sub(merchants);
        // The good that brings the city the most WEALTH = production × local price
        // (volume × value), plus its share of the hub's total export value.
        let mut tv_best: (String, f32) = (String::new(), 0.0);
        let mut tv_total = 0.0f32;
        for g in 0..gc {
            if prod[hh][g] > 0.05 {
                let v = prod[hh][g] * mkt.prices[hh][g].max(0.01);
                tv_total += v;
                if v > tv_best.1 {
                    tv_best = (goods_names.get(g).cloned().unwrap_or_default(), v);
                }
            }
        }
        let top_export = tv_best.0;
        let top_export_share = if tv_total > 0.0 { tv_best.1 / tv_total } else { 0.0 };
        // Luxury market: demand vs what actually arrives + delivered price. Price
        // rises when little gets through (scarcity from hard-to-move routes).
        let mut luxuries: Vec<HubLuxury> = (0..gc)
            .filter(|&g| is_luxury[g] && demand_v[hh][g] > 0.04)
            .map(|g| {
                // Delivered multiplier if it arrives by trade, else the local
                // equilibrium price relative to the world standard (scarce here
                // → well above 1×).
                let price = receives[hh].iter().find(|r| r.good == g).map(|r| r.price)
                    .unwrap_or_else(|| {
                        (mkt.prices[hh][g] / specs[g].base_value.max(0.05)).max(1.0)
                    });
                HubLuxury {
                    good: g, good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    demand: demand_v[hh][g], received: recv_amt[hh][g], price,
                }
            }).collect();
        luxuries.sort_by(|a, b| b.demand.partial_cmp(&a.demand).unwrap_or(std::cmp::Ordering::Equal));
        luxuries.truncate(8);

        // ── Market panel: prices in grain-eq, barter ratios, currency goods ──
        let market_panel = if is_outpost[hh] { None } else {
            let m = &mkt.hubs[hh];
            // This hub's top exports (by outbound value) are what imports are
            // bartered against.
            let mut top_exp: Vec<(usize, f32)> = (0..gc)
                .filter(|&g| exp_by[hh][g] > 0.01 || prod[hh][g] > 0.2)
                .map(|g| (g, exp_by[hh][g].max(prod[hh][g] * 0.2) * mkt.prices[hh][g]))
                .collect();
            top_exp.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_exp.truncate(3);
            // The most market-relevant goods here: traded, produced, or notably
            // mispriced vs the world standard.
            let mut relevant: Vec<(usize, f32)> = (0..gc)
                .filter(|&g| specs[g].enabled)
                .map(|g| {
                    let dev = (mkt.prices[hh][g] / specs[g].base_value.max(0.05) - 1.0).abs();
                    let activity = (recv_amt[hh][g] + exp_by[hh][g]) * mkt.prices[hh][g];
                    (g, activity + dev * specs[g].base_value.sqrt())
                })
                .filter(|&(_, score)| score > 0.02)
                .collect();
            relevant.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            relevant.truncate(16);
            let prices_v: Vec<HubMarketGood> = relevant.iter().map(|&(g, _)| {
                let exchanged_for: Vec<ExchangeRate> = top_exp.iter()
                    .filter(|&&(eg, _)| eg != g)
                    .map(|&(eg, _)| ExchangeRate {
                        good_name: goods_names.get(eg).cloned().unwrap_or_default(),
                        // Units of the counter-good one unit of g buys here.
                        ratio: mkt.prices[hh][g] / mkt.prices[hh][eg].max(1e-3),
                    })
                    .collect();
                HubMarketGood {
                    good: g,
                    good_name: goods_names.get(g).cloned().unwrap_or_default(),
                    price: mkt.prices[hh][g],
                    base_value: specs[g].base_value,
                    in_flow: recv_amt[hh][g],
                    out_flow: exp_by[hh][g],
                    exchanged_for,
                }
            }).collect();
            Some(HubMarket {
                grain_wealth: m.grain_wealth,
                trade_wealth: m.trade_wealth,
                currency_goods: m.currency_goods.iter()
                    .map(|c| goods_names.get(c.good).cloned().unwrap_or_default())
                    .collect(),
                currencies: m.currency_goods.iter()
                    .map(|c| HubCurrency {
                        good: c.good,
                        name: goods_names.get(c.good).cloned().unwrap_or_default(),
                        liquidity: c.liquidity,
                        value: c.value,
                        stability: c.stability,
                        price: mkt.prices[hh][c.good],
                    })
                    .collect(),
                prices: prices_v,
            })
        };

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
            emporium: emporium_set.contains(&hh),
            koppen: {
                let (hx, hy) = (nodes[hh].x, nodes[hh].y);
                let t = world.tile((hx / TILE_SIZE) as i32, (hy / TILE_SIZE) as i32);
                let ti = ((hy % TILE_SIZE) * TILE_SIZE + (hx % TILE_SIZE)) as usize;
                t.koppen.get(ti).copied().unwrap_or(0)
            },
            elevation: {
                let (hx, hy) = (nodes[hh].x, nodes[hh].y);
                let t = world.tile((hx / TILE_SIZE) as i32, (hy / TILE_SIZE) as i32);
                let ti = ((hy % TILE_SIZE) * TILE_SIZE + (hx % TILE_SIZE)) as usize;
                t.elevation.get(ti).copied().unwrap_or(0.0)
            },
            coastal: {
                let (hx, hy) = (nodes[hh].x, nodes[hh].y);
                let t = world.tile((hx / TILE_SIZE) as i32, (hy / TILE_SIZE) as i32);
                let ti = ((hy % TILE_SIZE) * TILE_SIZE + (hx % TILE_SIZE)) as usize;
                t.distance_to_ocean.get(ti).map(|&d| d < 0.06).unwrap_or(false)
            },
            nobility,
            merchants,
            commoners,
            elite_level: wealth_n,
            merchant_level: (cent_n * 0.5 + trade_n * 0.5),
            top_export,
            top_export_share,
            luxuries,
            sea_access: node_sea[hh],
            exports_to: exports_to_v[hh].clone(),
            shortages: shortages_v[hh].clone(),
            throughput: throughput_v[hh],
            exports: exports[hh],
            imports: imports[hh],
            partners: centrality[hh] as u32,
            ref_pct: (throughput_v[hh] / tp_max * 100.0).clamp(0.0, 100.0),
            nearest_ref: nearest_ref_hub((throughput_v[hh] / tp_max * 100.0).clamp(0.0, 100.0)).to_string(),
            monopolies: monopolies_v[hh].clone(),
            market: market_panel,
            produces,
            receives: recv,
        });
    }

    // ── Strategic chokepoints: geographic STRAITS + emporium CITIES ──
    // Two clean categories instead of arbitrary busy edges. A strait is a narrow
    // water cell with land close on two opposing sides that routed volume threads;
    // an emporium is a hub city a large share of routed goods passes THROUGH.
    let mut cell_vol = vec![0.0f32; (cc.cw * cc.ch) as usize];
    for ((a, b), (vol, _)) in &edge_vol {
        cell_vol[*a] += *vol;
        cell_vol[*b] += *vol;
    }
    let strait_r = 6i32;
    let land_within = |cx: i32, cy: i32, dirx: i32, diry: i32| -> bool {
        for k in 1..=strait_r {
            let nx = cx + dirx * k;
            let ny = cy + diry * k;
            if ny < 0 || ny >= cc.ch { return false; }
            if cc.is_land[cc.cidx(nx, ny)] { return true; }
        }
        false
    };
    let mut straits: Vec<(f32, usize)> = Vec::new();
    for cy in 0..cc.ch {
        for cx in 0..cc.cw {
            let ci = (cy * cc.cw + cx) as usize;
            if cc.is_land[ci] { continue; }
            let v = cell_vol[ci];
            if v <= 0.0 { continue; }
            let ew = land_within(cx, cy, -1, 0) && land_within(cx, cy, 1, 0);
            let ns = land_within(cx, cy, 0, -1) && land_within(cx, cy, 0, 1);
            if ew || ns { straits.push((v, ci)); }
        }
    }
    straits.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let sep = (grid_w as f32 * 0.04).max(8.0);
    let sep2 = sep * sep;
    let mut cps: Vec<(f32, [f32; 2], String)> = Vec::new();
    for (v, ci) in straits {
        let wpt = cc.world_of(ci);
        let far = cps.iter().all(|(_, p, _)| {
            let mut dx = (p[0] - wpt[0]).abs();
            if dx > grid_w as f32 / 2.0 { dx = grid_w as f32 - dx; }
            let dy = p[1] - wpt[1];
            dx * dx + dy * dy >= sep2
        });
        if far { cps.push((v, wpt, "Strait".to_string())); }
        if cps.len() >= 8 { break; }
    }
    // Emporium cities: hubs ranked by pass-through (transit) volume.
    let mut emp: Vec<(f32, usize)> = (0..nn)
        .map(|h| (transit[h], h)).filter(|&(v, _)| v > 0.0).collect();
    emp.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (v, h) in emp.into_iter().take(8) {
        cps.push((v, [nodes[h].x as f32, nodes[h].y as f32], hubs[h].name.clone()));
    }
    cps.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    cps.truncate(12);
    let top_vol = cps.first().map(|c| c.0).unwrap_or(1.0).max(1e-6);
    let chokepoints: Vec<EconChokepoint> = cps.into_iter().map(|(v, p, name)| EconChokepoint {
        points: vec![p, p], volume: v, share: v / top_vol, name,
    }).collect();

    // ── Trade-region territories: group owned coarse cells per hub ──
    let mut region_cells: Vec<Vec<[f32; 2]>> = vec![Vec::new(); nn];
    for cy in 0..ch {
        for cx in 0..cw {
            let o = owner[(cy * cw + cx) as usize];
            if o == u16::MAX { continue; }
            region_cells[o as usize].push([(cx as u32 * f) as f32, (cy as u32 * f) as f32]);
        }
    }
    let mut regions: Vec<EconRegion> = Vec::new();
    for hh in 0..nn {
        if region_cells[hh].is_empty() { continue; }
        regions.push(EconRegion {
            hub: hh as u32,
            name: hubs[hh].name.clone(),
            cells: std::mem::take(&mut region_cells[hh]),
            cell_size: f as f32,
        });
    }

    // ── Statistics by node class: trade hubs vs emporiums vs local trade posts ──
    // Computed from the emitted hubs so the figures match what's drawn.
    let class_stats: Vec<ClassStats> = {
        let agg = |pred: &dyn Fn(usize) -> bool, label: &str| -> ClassStats {
            let idxs: Vec<usize> = (0..nn).filter(|&h| pred(h)).collect();
            let count = idxs.len() as u32;
            let population: u64 = idxs.iter().map(|&h| nodes[h].population as u64).sum();
            let throughput: f32 = idxs.iter().map(|&h| throughput_v[h]).sum();
            let avg_wealth = if count > 0 {
                idxs.iter().map(|&h| wealth[h] / wmax).sum::<f32>() / count as f32
            } else { 0.0 };
            ClassStats { label: label.to_string(), count, population, throughput, avg_wealth }
        };
        vec![
            agg(&|h| !is_outpost[h] && emporium_set.contains(&h), "emporiums"),
            agg(&|h| !is_outpost[h] && !emporium_set.contains(&h), "hubs"),
            agg(&|h| is_outpost[h], "outposts"),
        ]
    };

    let snapshot = EconomySnapshot { hubs, chains, chokepoints, regions, corridors, good_stats, class_stats, goods: goods_names, colonizable_sites };
    let _ = metadata::campaign_set(&conn, "economy", &serde_json::to_string(&snapshot).unwrap_or_default());
    Ok(snapshot)
}

/// Export the economy snapshot to `path`. `.json` writes the raw snapshot;
/// anything else writes a flat CSV (one row per hub × good, produce + receive).
#[tauri::command]
pub fn export_trade_data(path: String, db: State<'_, WorldDb>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let json = metadata::campaign_get_or_meta(&conn, "economy").map_err(|e| e.to_string())?
        .ok_or_else(|| "No economy snapshot — run the Economy step (10) first.".to_string())?;
    if path.to_lowercase().ends_with(".json") {
        return std::fs::write(&path, json).map_err(|e| e.to_string());
    }
    let snap: EconomySnapshot = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let esc = |v: &str| -> String {
        if v.contains(',') || v.contains('"') || v.contains('\n') {
            format!("\"{}\"", v.replace('"', "\"\""))
        } else { v.to_string() }
    };
    let mut s = String::from("hub,x,y,stars,wealth,population,good,role,amount,quality,grade,flavor,price,counterparty\n");
    for h in &snap.hubs {
        for p in &h.produces {
            let row = [
                esc(&h.name), h.x.to_string(), h.y.to_string(), h.stars.to_string(),
                format!("{:.3}", h.wealth), h.population.to_string(),
                esc(&p.good_name), "produce".to_string(), format!("{:.3}", p.amount),
                format!("{:.3}", p.quality), esc(&p.grade), esc(&p.flavor),
                format!("{:.3}", p.price), String::new(),
            ];
            s.push_str(&row.join(",")); s.push('\n');
        }
        for r in &h.receives {
            let from = snap.hubs.iter().find(|x| x.id == r.from_hub)
                .map(|x| x.name.clone()).unwrap_or_default();
            let row = [
                esc(&h.name), h.x.to_string(), h.y.to_string(), h.stars.to_string(),
                format!("{:.3}", h.wealth), h.population.to_string(),
                esc(&r.good_name), "receive".to_string(), format!("{:.3}", r.amount),
                String::new(), String::new(), String::new(),
                format!("{:.3}", r.price), esc(&from),
            ];
            s.push_str(&row.join(",")); s.push('\n');
        }
    }
    std::fs::write(&path, s).map_err(|e| e.to_string())
}

/// Read the persisted economy snapshot (empty if none has been generated yet).
#[tauri::command]
pub fn get_economy(db: State<'_, WorldDb>) -> Result<EconomySnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let goods_names: Vec<String> = crate::commands::goods_commands::load_world_goods(&conn)
        .iter().map(|s| s.id.clone()).collect();
    match metadata::campaign_get_or_meta(&conn, "economy").map_err(|e| e.to_string())? {
        Some(json) => serde_json::from_str(&json)
            .map_err(|_| ()).or_else(|_| Ok::<_, String>(EconomySnapshot {
                hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![], corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names.clone(), colonizable_sites: vec![],
            })),
        None => Ok(EconomySnapshot { hubs: vec![], chains: vec![], chokepoints: vec![], regions: vec![], corridors: vec![], good_stats: vec![], class_stats: vec![], goods: goods_names, colonizable_sites: vec![] }),
    }
}

/// All frontend-held overlay state, restored from the DB when a saved world is
/// re-opened (the tiles persist in the DB already; these derived overlays live in
/// the store, so they must be persisted to metadata and re-hydrated on open).
#[derive(Serialize)]
pub struct OverlaysState {
    pub settlements: Vec<crate::sim::settlements::Settlement>,
    pub rivers: Vec<crate::sim::rivers::River>,
    pub lakes: Vec<crate::sim::rivers::Lake>,
    pub economy: EconomySnapshot,
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

/// Trade-development feedback: after the economy is solved, grow each settlement by
/// the trade wealth of its matching economy hub, so real emporia / chokepoint
/// cities swell into metropolises while inland subsistence towns stay put. One-way
/// (does NOT re-run the economy) and bounded (≤ ×3). Returns the updated list.
#[tauri::command]
pub fn compute_settlement_development(
    settlements_json: String,
    db: State<'_, WorldDb>,
) -> Result<Vec<crate::sim::settlements::Settlement>, String> {
    use crate::sim::settlements::Settlement;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut settlements: Vec<Settlement> =
        serde_json::from_str(&settlements_json).unwrap_or_default();
    if settlements.is_empty() { return Ok(settlements); }

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let snap: EconomySnapshot = match metadata::campaign_get_or_meta(&conn, "economy").map_err(|e| e.to_string())? {
        Some(json) => match serde_json::from_str(&json) { Ok(s) => s, Err(_) => return Ok(settlements) },
        None => return Ok(settlements),
    };
    if snap.hubs.is_empty() { return Ok(settlements); }

    let wrap_dx = |a: f32, b: f32| -> f32 {
        let mut d = (a - b).abs();
        if grid_w > 0 && d > grid_w as f32 / 2.0 { d = grid_w as f32 - d; }
        d
    };
    const DEV_GAIN: f32 = 2.0; // hub wealth 0..1 → up to ~×3 growth
    for s in settlements.iter_mut() {
        // Match to the nearest economy hub (settlements ARE the hubs, so this is
        // essentially itself) and read its normalized trade wealth.
        let mut best_w = 0.0f32;
        let mut bd = f32::INFINITY;
        for hub in &snap.hubs {
            let dx = wrap_dx(s.x as f32, hub.x);
            let dy = s.y as f32 - hub.y;
            let d = dx * dx + dy * dy;
            if d < bd { bd = d; best_w = hub.wealth; }
        }
        let grown = (s.population as f32 * (1.0 + DEV_GAIN * best_w.clamp(0.0, 1.0)))
            .min(s.population as f32 * 3.0);
        s.population = grown.round() as u32;
        s.size = if s.population >= 100_000 { "capital" }
            else if s.population >= 30_000 { "city" }
            else if s.population >= 5_000 { "town" }
            else { "village" }.to_string();
    }
    Ok(settlements)
}

// ─────────────────────────────────────────────────────────────────────────────
// River systems — Hydrology dashboard (#hydrology)
// ─────────────────────────────────────────────────────────────────────────────

/// A settlement as sent from the frontend store (only what the river panel needs).
#[derive(Deserialize, Default)]
struct RiverSettleIn {
    x: u32,
    y: u32,
    #[serde(default)] name: String,
    #[serde(default)] size: String,
}

/// A city sitting on (within a few cells of) a river reach.
#[derive(Serialize, Clone)]
pub struct RiverCityInfo {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub size: String,
    pub dist_from_mouth_km: f32,
}

/// One river in the system tree: a trunk (root) or a tributary (nested), with its
/// hydrological stats, longitudinal elevation profile, cities and children.
#[derive(Serialize)]
pub struct RiverNode {
    /// Index into the input rivers array (so the frontend can look up the cell path).
    pub id: usize,
    /// Unique, culture-styled river name (deterministic; dedup'd across the world).
    pub name: String,
    pub order: u8,
    pub navigable: bool,
    pub tributary: bool,
    pub mouth_kind: u8,
    pub length_km: f32,
    pub drop_m: f32,
    pub source_elev_m: f32,
    pub mouth_elev_m: f32,
    pub avg_slope_m_per_km: f32,
    pub discharge_m3s: f32,
    pub max_width_m: f32,
    pub max_depth_m: f32,
    pub navigable_km: f32,
    /// "alpine" | "highland" | "hills" | "lowland" | "bog".
    pub source_kind: String,
    pub source_x: u32,
    pub source_y: u32,
    pub mouth_x: u32,
    pub mouth_y: u32,
    pub mid_x: u32,
    pub mid_y: u32,
    /// Distance downstream along the PARENT (from its source) at which this
    /// tributary joins, in km. 0 for a trunk.
    pub join_km: f32,
    /// Total tributaries and cities in this subtree (this node + descendants).
    pub trib_total: usize,
    pub city_total: usize,
    /// Earth counterpart matched by discharge (roots only; empty for tributaries).
    pub counterpart: String,
    /// Hydrological flow regime phrase (Köppen-driven): "perennial temperate",
    /// "tropical flood-pulse", "nival subarctic", … (`sim/aquatic::river_regime`).
    pub regime: String,
    /// Fish assemblage prose — real Earth taxa by climate band × gradient × size
    /// × estuary (`sim/aquatic::river_fish`).
    pub fish: String,
    /// Riparian (bankside) vegetation phrase (`sim/aquatic::river_riparian`).
    pub riparian: String,
    /// Water character — clarity / sediment / productivity (`aquatic::river_water`).
    pub water: String,
    /// Charismatic riverine wildlife beyond fish (`aquatic::river_wildlife`).
    pub wildlife: String,
    /// A UNIQUE account of this river: for a trunk it is the SUMMARY lede
    /// synthesizing the whole course (`aquatic::river_summary`); for a tributary a
    /// short seeded description (`aquatic::river_story`).
    pub story: String,
    /// The biomes the trunk crosses source→mouth, as a phrase ("temperate forest,
    /// then cold steppe, ending in reed delta"). Empty for tributaries.
    #[serde(default)]
    pub climate_journey: String,
    /// The trunk's course split into upper / middle / lower-delta reaches, each
    /// with its own biome, character, story and confluences. Empty for tributaries.
    #[serde(default)]
    pub zones: Vec<RiverZoneOut>,
    /// Signature fish species for this river (one per zone it spans). Each carries
    /// a `slug` the frontend uses to show an illustration (`public/fish/<slug>.png`).
    pub species: Vec<FishSpeciesOut>,
    /// Downsampled elevation (m) along the reach, source → mouth.
    pub profile: Vec<f32>,
    pub cities: Vec<RiverCityInfo>,
    pub children: Vec<RiverNode>,
}

/// Precomputed per-river metrics (before the tree is assembled).
struct RiverDerived {
    length_km: f32,
    len_cells: f32,
    cum_km: Vec<f32>, // cumulative distance from source (km) at each point
    source_pt: (u32, u32),
    mouth_pt: (u32, u32),
    mid: (u32, u32),
    outlet: usize, // this reach's own downstream-most channel cell
    discharge: f32,
    width_m: f32,
    depth_m: f32,
    drop_m: f32,
    source_elev_m: f32,
    mouth_elev_m: f32,
    avg_slope: f32,
    source_kind: String,
    regime: String,
    fish: String,
    riparian: String,
    water: String,
    wildlife: String,
    band: crate::sim::aquatic::Band,
    arid: bool,
    profile: Vec<f32>,
}

/// Real rivers as (name, mean discharge m³/s, absolute mouth latitude °). The
/// latitude lets the match respect CLIMATE BAND: a big tropical river is likened
/// to the Amazon/Congo, a big far-northern one to the Lena/Ob — not purely by
/// discharge (which would call a great arctic river "the Amazon").
const EARTH_RIVERS: &[(&str, f32, f32)] = &[
    ("the Amazon", 209000.0, 0.0), ("the Congo", 41000.0, 6.0), ("the Ganges", 38000.0, 22.0),
    ("the Orinoco", 37000.0, 9.0), ("the Yangtze", 30000.0, 31.0), ("the Yenisei", 19000.0, 70.0),
    ("the Paraná", 18000.0, 34.0), ("the Lena", 17000.0, 72.0), ("the Mekong", 16000.0, 10.0),
    ("the Mississippi", 16000.0, 29.0), ("the Saint Lawrence", 12000.0, 48.0), ("the Ob", 12000.0, 66.0),
    ("the Amur", 11000.0, 53.0), ("the Volga", 8000.0, 46.0), ("the Indus", 6600.0, 24.0),
    ("the Danube", 6500.0, 45.0), ("the Niger", 5700.0, 5.0), ("the Zambezi", 3400.0, 18.0),
    ("the Nile", 2800.0, 31.0), ("the Rhine", 2300.0, 52.0), ("the Rhône", 1700.0, 43.0),
    ("the Elbe", 870.0, 54.0), ("the Seine", 560.0, 49.0), ("the Thames", 65.0, 51.0),
    // Regional mid/small rivers, so a modest river is likened to a real one of its
    // own climate & size rather than to a great continental trunk.
    ("the Chao Phraya", 3200.0, 13.0), ("the Magdalena", 7000.0, 11.0), ("the Neva", 2500.0, 60.0),
    ("the Northern Dvina", 3300.0, 64.0), ("the Po", 1540.0, 45.0), ("the Vistula", 1080.0, 54.0),
    ("the Loire", 840.0, 47.0), ("the Nemunas", 680.0, 55.0), ("the Daugava", 680.0, 57.0),
    ("the Garonne", 630.0, 45.0), ("the Oder", 570.0, 54.0), ("the Ebro", 430.0, 41.0),
    ("the Kemijoki", 560.0, 66.0), ("the Tiber", 240.0, 42.0), ("the Venta", 100.0, 57.0),
];

/// Closest real-world river by BOTH discharge (log-distance, primary) and mouth
/// latitude (secondary). The latitude term (≈1 penalty per 25° of difference)
/// only tips the choice between rivers of similar size, so climate band is
/// respected without overriding the dominant discharge match.
fn earth_counterpart(q: f32, abs_lat: f32) -> String {
    let lq = q.max(1.0).ln();
    let mut best = EARTH_RIVERS[0];
    let mut best_d = f32::MAX;
    for &(name, eq, elat) in EARTH_RIVERS {
        let d = (lq - eq.ln()).abs() + 0.9 * (abs_lat - elat).abs() / 25.0;
        if d < best_d { best_d = d; best = (name, eq, elat); }
    }
    best.0.to_string()
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

/// Core of `get_river_systems`, factored out so it can be unit-tested with a
/// synthetic world (no DB / Tauri State needed).
fn build_river_systems(
    buf: &crate::sim::world_buffer::WorldBuffer,
    rivers: &[crate::sim::rivers::River],
    settles: &[RiverSettleIn],
) -> Vec<RiverNode> {
    let w = buf.width;
    let h = buf.height;
    let km_per_cell = 40075.0f32 / w as f32;
    let cell_area_km2 = km_per_cell * km_per_cell;
    let hydro = crate::sim::rivers::compute_hydrology(&buf);
    let acc = &hydro.acc;
    let idx = |x: u32, y: u32| (y * w + x) as usize;

    // Cylindrical segment length (cells) between two adjacent river points.
    let seg = |a: (u32, u32), b: (u32, u32)| -> f32 {
        let dxr = (a.0 as i32 - b.0 as i32).abs();
        let dx = dxr.min(w as i32 - dxr) as f32;
        let dy = (a.1 as i32 - b.1 as i32) as f32;
        (dx * dx + dy * dy).sqrt()
    };

    // ── Per-river metrics ────────────────────────────────────────────────────
    let mut der: Vec<RiverDerived> = Vec::with_capacity(rivers.len());
    for r in rivers {
        let pts = &r.points;
        let m = pts.len();
        // Cumulative distance from source.
        let mut cum_km = vec![0.0f32; m];
        for k in 1..m { cum_km[k] = cum_km[k - 1] + seg(pts[k - 1], pts[k]) * km_per_cell; }
        let len_cells = if m > 1 { cum_km[m - 1] / km_per_cell } else { 0.0 };
        let length_km = cum_km[m.saturating_sub(1)];
        let source_pt = pts[0];
        let mouth_pt = pts[m - 1];
        // This reach's own outlet: the mouth (trunk) or the cell before the
        // confluence (tributary, whose last point belongs to the parent).
        let outlet_pt = if r.tributary && m >= 2 { pts[m - 2] } else { mouth_pt };
        let outlet = idx(outlet_pt.0, outlet_pt.1);
        let outlet_acc = acc[outlet] as f32;

        // Discharge from catchment area × runoff (precip-scaled), then hydraulic
        // geometry for width/depth (channel depth isn't modelled — this is an
        // estimate). Q clamped to a sane river range.
        let mut psum = 0.0f32;
        for &(px, py) in pts { psum += buf.precipitation[idx(px, py)]; }
        let mean_p = psum / m.max(1) as f32;
        let runoff_m = (mean_p / 1000.0 * 0.35).clamp(0.01, 1.2);
        let area_km2 = outlet_acc * cell_area_km2;
        let q = (area_km2 * 1.0e6 * runoff_m / 3.15576e7).clamp(0.3, 300000.0);
        let width_m = (6.0 * q.sqrt()).clamp(1.0, 4000.0);
        // Max (thalweg) depth — great rivers run VERY deep: the Amazon reaches
        // ~100 m and the Congo deeper still. This is the DEEPEST point, not the
        // mean, so it uses a stronger exponent and a high cap; small streams stay
        // shallow. Depth isn't modelled per-cell, so it's an estimate from Q.
        let depth_m = (0.62 * q.powf(0.43)).clamp(0.3, 230.0);

        let source_elev_m = buf.elevation[idx(source_pt.0, source_pt.1)] * 8848.0;
        let mouth_elev_m = buf.elevation[outlet] * 8848.0;
        let drop_m = (source_elev_m - mouth_elev_m).max(0.0);
        let avg_slope = if length_km > 1.0 { drop_m / length_km } else { 0.0 };

        // Source classification (mountain spring ↔ lowland bog).
        let src_norm = buf.elevation[idx(source_pt.0, source_pt.1)];
        let src_precip = buf.precipitation[idx(source_pt.0, source_pt.1)];
        let near = pts[m.min(4) - 1];
        let near_km = cum_km[m.min(4) - 1].max(0.001);
        let head_slope = ((source_elev_m - buf.elevation[idx(near.0, near.1)] * 8848.0) / near_km).abs();
        let source_kind = if src_norm > 0.45 { "alpine" }
            else if src_norm > 0.28 { "highland" }
            else if src_norm > 0.15 { "hills" }
            else if src_precip > 1100.0 && head_slope < 3.0 { "bog" }
            else { "lowland" }.to_string();

        // ── Freshwater ecology (regime / fish / riparian) ──
        // Aggregate the reach's climate: mean air temperature, the dominant Köppen
        // class (mode over the reach), and how arid it runs. These feed the shared
        // aquatic-ecology mapper (real Earth taxa + flow regime).
        use crate::sim::aquatic;
        let mut tsum = 0.0f32;
        let mut arid_cells = 0usize;
        let mut khist: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
        let have_temp = !buf.temperature.is_empty();
        let have_kop = !buf.koppen.is_empty();
        for &(px, py) in pts {
            let pi = idx(px, py);
            if have_temp { tsum += buf.temperature[pi]; }
            if have_kop {
                let k = buf.koppen[pi];
                *khist.entry(k).or_insert(0) += 1;
                if matches!(k, crate::sim::koppen::BWH | crate::sim::koppen::BWK
                    | crate::sim::koppen::BSH | crate::sim::koppen::BSK) { arid_cells += 1; }
            }
        }
        let mean_temp = if have_temp { tsum / m as f32 } else { 12.0 };
        let dominant_koppen = khist.iter().max_by_key(|(_, &c)| c).map(|(&k, _)| k).unwrap_or(0);
        let arid = arid_cells as f32 / m as f32 > 0.4;
        let band = aquatic::band_from_temp(mean_temp);
        // Estuary/delta mouth admits diadromous runs; a plain inland/lake end doesn't.
        let estuary = r.mouth_kind != 0;
        let regime = aquatic::river_regime(dominant_koppen, band);
        let fish = aquatic::river_fish(band, avg_slope, q, estuary);
        let riparian = aquatic::river_riparian(band, arid);
        let water = aquatic::river_water(band, avg_slope >= 6.0, arid);
        let wildlife = aquatic::river_wildlife(band, q >= 3000.0);

        // Downsampled long profile (m), source → mouth. A river's water surface can
        // only ever DESCEND, so we sample the depression-FILLED surface (which is
        // strictly monotonic downhill along the flow path) rather than raw terrain
        // (which rises and falls across the flats the channel crosses — the "height
        // jumps up and down in river view" artefact). A running minimum guards
        // against any residual downsampling wobble so the plotted profile never
        // climbs between source and mouth.
        let samples = 48usize.min(m);
        let mut profile = Vec::with_capacity(samples);
        let mut running_min = f32::MAX;
        for s in 0..samples {
            let t = if samples > 1 { s as f32 / (samples - 1) as f32 } else { 0.0 };
            let pi = (t * (m - 1) as f32).round() as usize;
            let (px, py) = pts[pi.min(m - 1)];
            let e = hydro.filled[idx(px, py)] * 8848.0;
            running_min = running_min.min(e);
            profile.push(running_min);
        }

        der.push(RiverDerived {
            length_km, len_cells, cum_km, source_pt, mouth_pt, mid: pts[m / 2],
            outlet, discharge: q, width_m, depth_m, drop_m, source_elev_m, mouth_elev_m,
            avg_slope, source_kind, regime, fish, riparian, water, wildlife, band, arid, profile,
        });
    }

    // ── Cell ownership (for parent linking + city attribution) ───────────────
    // owner_best[cell] = (order, river_id, point_index) preferring the higher-order
    // river where reaches share a cell (confluence cells belong to more than one).
    let mut owners: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut owner_best: HashMap<usize, (u8, usize, usize)> = HashMap::new();
    for (ri, r) in rivers.iter().enumerate() {
        for (pi, &(px, py)) in r.points.iter().enumerate() {
            let c = idx(px, py);
            owners.entry(c).or_default().push(ri);
            let e = owner_best.entry(c).or_insert((0, ri, pi));
            if r.order >= e.0 { *e = (r.order, ri, pi); }
        }
    }

    // ── Parent linking: each tributary joins the largest reach through its
    // confluence cell (its last point). Unmatched tributaries become roots. ──
    let mut parent = vec![usize::MAX; rivers.len()];
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); rivers.len()];
    let mut join_km = vec![0.0f32; rivers.len()];
    for (ri, r) in rivers.iter().enumerate() {
        if !r.tributary { continue; }
        let jc = *r.points.last().unwrap();
        let jci = idx(jc.0, jc.1);
        if let Some(list) = owners.get(&jci) {
            let mut best: Option<usize> = None;
            for &cand in list {
                if cand == ri { continue; }
                match best {
                    None => best = Some(cand),
                    Some(b) => {
                        let better = rivers[cand].order > rivers[b].order
                            || (rivers[cand].order == rivers[b].order
                                && der[cand].length_km > der[b].length_km);
                        if better { best = Some(cand); }
                    }
                }
            }
            if let Some(p) = best {
                parent[ri] = p;
                children[p].push(ri);
                // Distance along the parent (from its source) to the confluence.
                if let Some(pi) = rivers[p].points.iter().position(|&c| idx(c.0, c.1) == jci) {
                    join_km[ri] = der[p].cum_km[pi];
                }
            }
        }
    }

    // ── City attribution: nearest owned river cell within 3 cells ────────────
    let mut river_cities: Vec<Vec<RiverCityInfo>> = vec![Vec::new(); rivers.len()];
    let wrap_x = |x: i32| ((x % w as i32) + w as i32) % w as i32;
    for s in settles {
        let mut best: Option<(f32, usize, usize)> = None; // (dist, river_id, point_idx)
        for dy in -3i32..=3 {
            let ny = s.y as i32 + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            for dx in -3i32..=3 {
                let nx = wrap_x(s.x as i32 + dx) as u32;
                let c = idx(nx, ny as u32);
                if let Some(&(_, rid, pidx)) = owner_best.get(&c) {
                    let d = ((dx * dx + dy * dy) as f32).sqrt();
                    if best.map_or(true, |(bd, _, _)| d < bd) { best = Some((d, rid, pidx)); }
                }
            }
        }
        if let Some((_, rid, pidx)) = best {
            let dist_from_mouth_km = (der[rid].length_km - der[rid].cum_km[pidx]).max(0.0);
            river_cities[rid].push(RiverCityInfo {
                name: if s.name.is_empty() { "(unnamed)".into() } else { s.name.clone() },
                x: s.x, y: s.y, size: s.size.clone(), dist_from_mouth_km,
            });
        }
    }
    for cs in river_cities.iter_mut() {
        cs.sort_by(|a, b| a.dist_from_mouth_km.partial_cmp(&b.dist_from_mouth_km).unwrap_or(std::cmp::Ordering::Equal));
    }

    // ── Unique culture-styled name + latitude-aware counterpart per river ────
    // Names are drawn from the local culture at each river's midpoint and made
    // globally unique (longest rivers pick first, so great trunks keep clean
    // names and only small tributaries fall back to re-rolls). The counterpart
    // matches by discharge AND mouth latitude so climate band is respected.
    let mut order_by_len: Vec<usize> = (0..rivers.len()).collect();
    order_by_len.sort_by(|&a, &b| {
        der[b].length_km.partial_cmp(&der[a].length_km).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut names_by_id: Vec<String> = vec![String::new(); rivers.len()];
    let mut counterpart_by_id: Vec<String> = vec![String::new(); rivers.len()];
    for &i in &order_by_len {
        let mid = der[i].mid;
        let (kit, ms) = crate::sim::names::resolve_kit(mid.0, mid.1, w, h);
        let seed0 = (mid.0 as u64).wrapping_mul(73856093)
            ^ (mid.1 as u64).wrapping_mul(19349663)
            ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut chosen = String::new();
        for t in 0..32u64 {
            let cand = crate::sim::cultures::river_name(kit, ms, seed0.wrapping_add(t.wrapping_mul(0x100000001B3)));
            if used.insert(cand.clone()) { chosen = cand; break; }
        }
        if chosen.is_empty() {
            // Guaranteed-unique fallback (only if 32 draws all collided).
            let base = crate::sim::cultures::river_name(kit, ms, seed0);
            chosen = format!("{} {}", base, uniq_suffix(i));
            used.insert(chosen.clone());
        }
        names_by_id[i] = chosen;
        counterpart_by_id[i] = earth_counterpart(der[i].discharge, buf.latitude(der[i].mouth_pt.1).abs());
    }

    // ── Segmented course: climate journey + upper/middle/delta zones (trunks) ──
    // Precomputed here because it needs the WorldBuffer (Köppen/elevation along the
    // reach); `build` then attaches it. Only the main stem (a sea-reaching trunk)
    // gets zones — tributaries keep a single short description.
    let mut river_journey = vec![String::new(); rivers.len()];
    let mut river_zones: Vec<Vec<RiverZoneOut>> = vec![Vec::new(); rivers.len()];
    let have_koppen = !buf.koppen.is_empty();
    for i in 0..rivers.len() {
        let r = &rivers[i];
        if r.tributary { continue; }
        let d = &der[i];
        let pts = &r.points;
        let np = pts.len();
        if np < 6 || d.length_km < 1.0 || d.cum_km.len() != np { continue; }
        let cum = &d.cum_km;
        let ll = d.length_km;
        let elev_at = |k: usize| buf.elevation[idx(pts[k].0, pts[k].1)];
        let kop_at = |k: usize| if have_koppen { buf.koppen[idx(pts[k].0, pts[k].1)] } else { 0 };

        // Zone boundaries: gradient-primary (a river sheds most height early, so the
        // 60%/92%-of-drop marks split headwater/middle/lower) with distance clamps.
        let src_e = elev_at(0);
        let total_drop = (src_e - elev_at(np - 1)).max(1e-4);
        let (mut u_end, mut d_start) = (0.30 * ll, 0.75 * ll);
        let (mut got_u, mut got_d) = (false, false);
        let mut mine = src_e;
        for k in 0..np {
            let e = elev_at(k);
            if e < mine { mine = e; }
            let frac = (src_e - mine) / total_drop;
            if !got_u && frac >= 0.60 { u_end = cum[k]; got_u = true; }
            if !got_d && frac >= 0.92 { d_start = cum[k]; got_d = true; }
        }
        u_end = u_end.clamp(0.15 * ll, 0.42 * ll);
        d_start = d_start.clamp(0.62 * ll, 0.90 * ll).max(u_end + 0.05 * ll);

        // Climate journey — biomes crossed source→mouth, de-duplicated in order.
        if have_koppen {
            let mut uniq: Vec<&str> = Vec::new();
            for k in 0..np {
                let b = crate::sim::aquatic::koppen_to_biome(kop_at(k));
                if !uniq.contains(&b) { uniq.push(b); }
            }
            uniq.truncate(4);
            river_journey[i] = join_biomes(&uniq);
        }

        // Direct tributaries as (name, km-from-source-of-this-trunk).
        let tribs: Vec<(String, f32)> = children[i].iter()
            .map(|&c| (names_by_id[c].clone(), join_km[c]))
            .collect();

        // A "great" trunk (mega-river) carries sturgeon/giant-catfish down its deep stem.
        let great = d.discharge >= 3000.0;
        let mut zones: Vec<RiverZoneOut> = Vec::new();
        for &(zs, ze, kind) in &[(0.0, u_end, 0u8), (u_end, d_start, 1u8), (d_start, ll, 2u8)] {
            if ze - zs < 0.5 { continue; }
            // Dominant Köppen over this reach.
            let mut counts: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
            for k in 0..np { if cum[k] >= zs && cum[k] <= ze { *counts.entry(kop_at(k)).or_default() += 1; } }
            let dom_k = counts.iter().max_by_key(|(_, c)| **c).map(|(k, _)| *k).unwrap_or(0);
            let biome = crate::sim::aquatic::koppen_to_biome(dom_k);
            let ztribs: Vec<(String, f32)> = tribs.iter().filter(|(_, km)| *km >= zs && *km < ze).cloned().collect();
            // Tidal/brackish mouth → diadromous runs ascend the delta reach.
            let est = kind == 2 && (r.mouth_kind != 0 || d.discharge >= 800.0);
            let zone_seed = (d.source_pt.0 as u64).wrapping_mul(2246822519)
                ^ (d.mouth_pt.1 as u64).wrapping_mul(3266489917)
                ^ ((i as u64) << 3) ^ kind as u64;
            // The FULL reach assemblage (every fish that lives in this reach); the
            // prose sentence is built from the SAME species so plates and text agree.
            let reach_species = crate::sim::aquatic::assign_reach_fish(d.band, kind, great, est, d.arid, zone_seed);
            let fish = crate::sim::aquatic::reach_fish_prose(&reach_species);
            let species_out: Vec<FishSpeciesOut> = reach_species.iter().map(|f| FishSpeciesOut {
                slug: f.slug.into(), name: f.name.into(), binomial: f.binomial.into(),
                zone: f.zone, real: f.real.into(), blurb: f.blurb.into(),
            }).collect();
            let story = crate::sim::aquatic::zone_story(&crate::sim::aquatic::ZoneFacts {
                kind, name: &names_by_id[i], biome, fish: &fish, tribs: &ztribs,
                mouth_kind: if kind == 2 { r.mouth_kind } else { 0 }, band: d.band,
                seed: zone_seed,
            });
            let label = match kind {
                0 => "Upper river", 1 => "Middle river",
                _ => match r.mouth_kind { 1 => "Delta", 2 => "Estuary", _ => "Lower course" },
            };
            zones.push(RiverZoneOut {
                kind: match kind { 0 => "upper", 1 => "middle", _ => "delta" }.into(),
                label: label.into(), start_km: zs, end_km: ze,
                biome: biome.into(), koppen: crate::sim::aquatic::koppen_code(dom_k).into(),
                character: zone_character(kind, r.mouth_kind).into(),
                story,
                tributaries: ztribs.into_iter().map(|(name, km)| TribJoin { name, km }).collect(),
                species: species_out,
            });
        }
        river_zones[i] = zones;
    }

    // ── Assemble the tree (recursive, full depth) ────────────────────────────
    // `seen` guards against a degenerate mutual-confluence cycle: a node already
    // on the current path is emitted as a leaf so the recursion can't run away.
    fn build(
        i: usize, rivers: &[crate::sim::rivers::River], der: &[RiverDerived],
        children: &[Vec<usize>], join_km: &[f32], river_cities: &[Vec<RiverCityInfo>],
        names_by_id: &[String], counterpart_by_id: &[String],
        river_journey: &[String], river_zones: &[Vec<RiverZoneOut>],
        seen: &mut Vec<bool>,
    ) -> RiverNode {
        let cyclic = seen[i];
        seen[i] = true;
        let r = &rivers[i];
        let d = &der[i];
        let mut kids: Vec<usize> = if cyclic { Vec::new() } else { children[i].clone() };
        kids.sort_by(|&a, &b| der[b].discharge.partial_cmp(&der[a].discharge).unwrap_or(std::cmp::Ordering::Equal));
        let child_nodes: Vec<RiverNode> = kids.iter()
            .map(|&c| build(c, rivers, der, children, join_km, river_cities, names_by_id, counterpart_by_id, river_journey, river_zones, seen))
            .collect();
        let trib_total = child_nodes.iter().map(|c| 1 + c.trib_total).sum();
        let city_total = river_cities[i].len() + child_nodes.iter().map(|c| c.city_total).sum::<usize>();
        // Write-up: a TRUNK gets the SUMMARY lede (the whole-course synthesis) plus
        // its upper/middle/delta zones + climate journey; a TRIBUTARY keeps a short
        // seeded description and no zones.
        let seed = (d.source_pt.0 as u64).wrapping_mul(73856093)
            ^ (d.source_pt.1 as u64).wrapping_mul(19349663)
            ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let (story, climate_journey, zones): (String, String, Vec<RiverZoneOut>) = if !r.tributary {
            let summary = crate::sim::aquatic::river_summary(&crate::sim::aquatic::RiverSummaryFacts {
                name: &names_by_id[i], discharge_m3s: d.discharge, source_kind: &d.source_kind,
                mouth_kind: r.mouth_kind, trib_total, journey: &river_journey[i],
                counterpart: &counterpart_by_id[i], length_km: d.length_km, navigable: r.navigable, seed,
            });
            (summary, river_journey[i].clone(), river_zones[i].clone())
        } else {
            let s = crate::sim::aquatic::river_story(&crate::sim::aquatic::RiverStoryFacts {
                name: &names_by_id[i], counterpart: &counterpart_by_id[i], length_km: d.length_km,
                discharge_m3s: d.discharge, source_kind: &d.source_kind, mouth_kind: r.mouth_kind,
                navigable: r.navigable, trib_total, city_total, band: d.band, regime: &d.regime,
                fish: &d.fish, riparian: &d.riparian, water: &d.water, wildlife: &d.wildlife,
                arid: d.arid, seed,
            });
            (s, String::new(), Vec::new())
        };
        // Signature fish species. A TRUNK's flat list is the union of its reach
        // assemblages (deduped, source→mouth); a TRIBUTARY (no zones) keeps the
        // single-species-per-zone roster.
        let species: Vec<FishSpeciesOut> = if !zones.is_empty() {
            let mut seen: Vec<String> = Vec::new();
            let mut out: Vec<FishSpeciesOut> = Vec::new();
            for z in &zones {
                for sp in &z.species {
                    if !seen.contains(&sp.slug) { seen.push(sp.slug.clone()); out.push(sp.clone()); }
                }
            }
            out
        } else {
            let sp_seed = (d.source_pt.0 as u64).wrapping_mul(2654435761)
                ^ (d.mouth_pt.1 as u64).wrapping_mul(40503)
                ^ (i as u64);
            crate::sim::aquatic::assign_river_fish(d.band, &d.source_kind, r.mouth_kind, d.discharge, d.arid, sp_seed)
                .into_iter()
                .map(|f| FishSpeciesOut {
                    slug: f.slug.into(), name: f.name.into(), binomial: f.binomial.into(),
                    zone: f.zone, real: f.real.into(), blurb: f.blurb.into(),
                })
                .collect()
        };
        RiverNode {
            id: i,
            name: names_by_id[i].clone(),
            order: r.order,
            navigable: r.navigable,
            tributary: r.tributary,
            mouth_kind: r.mouth_kind,
            length_km: d.length_km,
            drop_m: d.drop_m,
            source_elev_m: d.source_elev_m,
            mouth_elev_m: d.mouth_elev_m,
            avg_slope_m_per_km: d.avg_slope,
            discharge_m3s: d.discharge,
            max_width_m: d.width_m,
            max_depth_m: d.depth_m,
            navigable_km: if r.navigable { d.length_km } else { 0.0 },
            source_kind: d.source_kind.clone(),
            source_x: d.source_pt.0, source_y: d.source_pt.1,
            mouth_x: d.mouth_pt.0, mouth_y: d.mouth_pt.1,
            mid_x: d.mid.0, mid_y: d.mid.1,
            join_km: join_km[i],
            trib_total,
            city_total,
            // Closest real-world counterpart (by discharge AND mouth latitude).
            counterpart: counterpart_by_id[i].clone(),
            regime: d.regime.clone(),
            fish: d.fish.clone(),
            riparian: d.riparian.clone(),
            water: d.water.clone(),
            wildlife: d.wildlife.clone(),
            story,
            climate_journey,
            zones,
            species,
            profile: d.profile.clone(),
            cities: river_cities[i].clone(),
            children: child_nodes,
        }
    }

    let mut seen = vec![false; rivers.len()];
    let mut roots: Vec<RiverNode> = (0..rivers.len())
        .filter(|&i| parent[i] == usize::MAX)
        .map(|i| build(i, &rivers, &der, &children, &join_km, &river_cities, &names_by_id, &counterpart_by_id, &river_journey, &river_zones, &mut seen))
        .collect();

    // Sort systems by length (longest first); counterpart is set per node in build.
    roots.sort_by(|a, b| b.length_km.partial_cmp(&a.length_km).unwrap_or(std::cmp::Ordering::Equal));
    roots
}

/// A signature fish species assigned to a river reach (mirrors TS `FishSpecies`).
#[derive(serde::Serialize, Clone)]
pub struct FishSpeciesOut {
    pub slug: String,
    pub name: String,
    pub binomial: String,
    pub zone: u8,
    pub real: String,
    pub blurb: String,
}

/// A tributary joining a river at a given distance downstream (mirrors TS).
#[derive(serde::Serialize, Clone)]
pub struct TribJoin {
    pub name: String,
    pub km: f32,
}

/// One reach of a trunk river's course — upper / middle / lower-delta — with the
/// biome it crosses, its character, a seeded story and the tributaries that join
/// along it (mirrors TS `RiverZone`). Only the main stem (a sea-reaching trunk)
/// gets zones; tributaries keep a single short description.
#[derive(serde::Serialize, Clone)]
pub struct RiverZoneOut {
    pub kind: String,     // "upper" | "middle" | "delta"
    pub label: String,    // "Upper river" | "Middle river" | "Delta / estuary"
    pub start_km: f32,
    pub end_km: f32,
    pub biome: String,    // dominant biome of this reach
    pub koppen: String,   // dominant Köppen code (tooltip)
    pub character: String,
    pub story: String,
    pub tributaries: Vec<TribJoin>,
    /// The full fish assemblage that lives in THIS reach (every named species,
    /// with an illustration slug). Empty for tributaries (no zones).
    pub species: Vec<FishSpeciesOut>,
}

/// Format the ordered list of biomes a river crosses into a prose phrase.
fn join_biomes(b: &[&str]) -> String {
    match b.len() {
        0 => String::new(),
        1 => b[0].to_string(),
        2 => format!("{}, then {}", b[0], b[1]),
        _ => {
            let (last, head) = b.split_last().unwrap();
            format!("{}, ending in {}", head.join(", then "), last)
        }
    }
}

/// A short character phrase for a reach (upper / middle / lower-delta).
fn zone_character(kind: u8, mouth_kind: u8) -> &'static str {
    match kind {
        0 => "steep and fast — a rocky, clear mountain stream",
        1 => "broad and steady, beginning to meander over its floodplain",
        _ => match mouth_kind {
            1 => "flat and branching, splitting into distributaries across its delta",
            2 => "wide and tidal, brackish where it meets the sea",
            _ => "slow and wide as it nears its end",
        },
    }
}

/// One classified lake with its limnological + ecological profile, for the
/// Hydrology dashboard's Lakes tab. Mirrors the TS `LakeNode`.
#[derive(serde::Serialize)]
pub struct LakeNode {
    pub id: usize,
    pub name: String,
    /// Machine type slug: rift | crater | salt | glacial | tropical | lowland | tarn.
    pub kind: String,
    pub kind_label: String,
    pub area_km2: f32,
    pub max_depth_m: f32,
    pub mean_depth_m: f32,
    pub elev_m: f32,
    pub volume_km3: f32,
    /// True terminal salt lake (no outflow river, arid basin).
    pub endorheic: bool,
    pub salinity_ppt: f32,
    /// Real-world analog lake (Baikal, Victoria, …).
    pub analog: String,
    pub thermal: String,
    pub water: String,
    pub fish: String,
    pub wildlife: String,
    pub endemism: String,
    pub blurb: String,
    /// Signature fish species for this lake (a variable-length roster by type ×
    /// band — a rift flock, a single tarn char, brine shrimp for a salt lake).
    pub species: Vec<FishSpeciesOut>,
    /// A UNIQUE, seeded NatGeo-style account of the lake (no two read alike).
    pub story: String,
    /// Names of the rivers that feed / drain the lake ("" outflow = terminal).
    pub inflows: Vec<String>,
    pub outflow: String,
    pub cx: u32,
    pub cy: u32,
    pub area_cells: usize,
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

/// Core of `get_lake_systems`, factored out for unit testing without a DB.
fn build_lake_systems(
    buf: &crate::sim::world_buffer::WorldBuffer,
    lakes: &[crate::sim::rivers::Lake],
    rivers: &[crate::sim::rivers::River],
) -> Vec<LakeNode> {
    use crate::sim::aquatic;
    let w = buf.width;
    let h = buf.height;
    let km_per_cell = 40075.0f32 / w as f32;
    let cell_area_km2 = km_per_cell * km_per_cell;
    let idx = |x: u32, y: u32| (y * w + x) as usize;
    let hydro = crate::sim::rivers::compute_hydrology(buf);
    let have_temp = !buf.temperature.is_empty();
    let have_bound = !buf.boundary_type.is_empty();
    let have_volc = !buf.is_volcanic.is_empty();

    // A cell is "near a lake" if it or an 8-neighbour is a lake cell (used to
    // attach inflow/outflow rivers whose channel stops one cell short of water).
    let mut lake_of: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (li, lk) in lakes.iter().enumerate() {
        for &(cx, cy) in &lk.cells { lake_of.insert(idx(cx, cy), li); }
    }
    let near_lake = |x: u32, y: u32| -> Option<usize> {
        if let Some(&l) = lake_of.get(&idx(x, y)) { return Some(l); }
        for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = y as i32 + dy;
            if ny < 0 || ny >= h as i32 { continue; }
            if let Some(&l) = lake_of.get(&idx(nx, ny as u32)) { return Some(l); }
        }
        None
    };

    // River label = culture-styled name at the reach midpoint (matches the
    // toponym/river-panel naming machinery closely enough for a feeder list).
    let river_label = |r: &crate::sim::rivers::River| -> String {
        if r.points.len() < 2 { return "a stream".into(); }
        let (mx, my) = r.points[r.points.len() / 2];
        let (kit, ms) = crate::sim::names::resolve_kit(mx, my, w, h);
        let seed = (mx as u64).wrapping_mul(73856093) ^ (my as u64).wrapping_mul(19349663) ^ 0x1111;
        crate::sim::cultures::river_name(kit, ms, seed)
    };

    // Attach each river to a lake as inflow (its MOUTH dies at the shore) or
    // outflow (its SOURCE resumes at the shore = the lake drains through it).
    let mut inflows: Vec<Vec<String>> = vec![Vec::new(); lakes.len()];
    let mut outflow: Vec<Option<String>> = vec![None; lakes.len()];
    for r in rivers {
        if r.points.len() < 2 { continue; }
        let (sx, sy) = r.points[0];
        let (ex, ey) = r.points[r.points.len() - 1];
        if let Some(l) = near_lake(ex, ey) { inflows[l].push(river_label(r)); }
        if let Some(l) = near_lake(sx, sy) {
            // The largest reach resuming at the shore is the outflow.
            if outflow[l].is_none() { outflow[l] = Some(river_label(r)); }
        }
    }

    let mut out: Vec<LakeNode> = Vec::with_capacity(lakes.len());
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (li, lk) in lakes.iter().enumerate() {
        let n = lk.cells.len();
        if n == 0 { continue; }
        // Centroid + bbox (for elongation) + depth + climate aggregates.
        let (mut sx, mut sy) = (0u64, 0u64);
        let (mut minx, mut maxx, mut miny, mut maxy) = (u32::MAX, 0u32, u32::MAX, 0u32);
        let (mut max_depth, mut depth_sum) = (0.0f32, 0.0f32);
        let (mut tsum, mut arid_cells) = (0.0f32, 0usize);
        let (mut volcanic, mut on_boundary) = (false, false);
        for &(cx, cy) in &lk.cells {
            sx += cx as u64; sy += cy as u64;
            minx = minx.min(cx); maxx = maxx.max(cx); miny = miny.min(cy); maxy = maxy.max(cy);
            let ci = idx(cx, cy);
            let d = ((hydro.filled[ci] - buf.elevation[ci]).max(0.0)) * 8848.0;
            max_depth = max_depth.max(d); depth_sum += d;
            if have_temp { tsum += buf.temperature[ci]; }
            if !buf.koppen.is_empty() && matches!(buf.koppen[ci],
                crate::sim::koppen::BWH | crate::sim::koppen::BWK
                    | crate::sim::koppen::BSH | crate::sim::koppen::BSK) { arid_cells += 1; }
            if have_volc && buf.is_volcanic[ci] == 1 { volcanic = true; }
            if have_bound && buf.boundary_type[ci] != 0 { on_boundary = true; }
        }
        let cx = (sx / n as u64) as u32;
        let cy = (sy / n as u64) as u32;
        let area_km2 = n as f32 * cell_area_km2;
        // Basins are shallow relative to their fill; scale the modelled fill depth
        // to a plausible lake depth (rift/crater basins run genuinely deep). Add a
        // floor so even a shallow pan reads as a few metres.
        let max_depth_m = (max_depth * 3.0 + 2.0).clamp(2.0, 1600.0);
        let mean_depth_m = ((depth_sum / n as f32) * 3.0 + 1.0).clamp(1.0, max_depth_m);
        let elev_m = lk.elevation * 8848.0;
        let volume_km3 = area_km2 * mean_depth_m / 1000.0;
        let abs_lat = buf.latitude(cy).abs();
        let mean_temp = if have_temp { tsum / n as f32 } else { 30.0 - 0.4 * abs_lat };
        let arid = arid_cells as f32 / n as f32 > 0.4;
        let span_x = (maxx - minx) as f32 + 1.0;
        let span_y = (maxy - miny) as f32 + 1.0;
        let elongated = (span_x / span_y).max(span_y / span_x) >= 2.2;
        let has_outflow = outflow[li].is_some();
        let endorheic = !has_outflow && arid;

        let facts = aquatic::LakeFacts {
            area_km2, max_depth_m, elev_m, abs_lat, mean_temp_c: mean_temp,
            volcanic, on_boundary, endorheic, arid, elongated,
        };
        let kind = aquatic::classify_lake(&facts);
        let band = aquatic::band_from_temp(mean_temp);

        // Unique culture-styled name at the centroid.
        let name = {
            let (kit, ms) = crate::sim::names::resolve_kit(cx, cy, w, h);
            let mut nm = crate::sim::cultures::place_name(kit, ms, cx, cy);
            let mut t = 1u32;
            while !used.insert(nm.clone()) && t < 24 {
                nm = crate::sim::cultures::place_name(kit, ms,
                    (cx + t) % w.max(1), (cy + (t / 4)) % h.max(1));
                t += 1;
            }
            nm
        };

        let story = aquatic::lake_story(&aquatic::LakeStoryFacts {
            name: &name,
            kind, band,
            analog: aquatic::lake_analog(kind, band),
            area_km2, max_depth_m, endorheic,
            inflow_count: inflows[li].len(),
            outflow: outflow[li].as_deref().unwrap_or(""),
            thermal: &aquatic::lake_thermal(kind, band, max_depth_m),
            water: &aquatic::lake_water(kind),
            fish: &aquatic::lake_fish(kind, band),
            wildlife: &aquatic::lake_wildlife(kind, band),
            endemism: &aquatic::lake_endemism(kind, band),
            seed: (cx as u64).wrapping_mul(73856093)
                ^ (cy as u64).wrapping_mul(19349663)
                ^ (li as u64).wrapping_mul(0x9E3779B97F4A7C15),
        });

        out.push(LakeNode {
            id: li,
            name,
            kind: kind.as_str().into(),
            kind_label: kind.label().into(),
            area_km2,
            max_depth_m,
            mean_depth_m,
            elev_m,
            volume_km3,
            endorheic,
            salinity_ppt: aquatic::lake_salinity_ppt(kind, max_depth_m),
            analog: aquatic::lake_analog(kind, band).into(),
            thermal: aquatic::lake_thermal(kind, band, max_depth_m),
            water: aquatic::lake_water(kind),
            fish: aquatic::lake_fish(kind, band),
            wildlife: aquatic::lake_wildlife(kind, band),
            endemism: aquatic::lake_endemism(kind, band),
            blurb: aquatic::lake_blurb(kind, band),
            species: aquatic::assign_lake_fish(kind, band, aquatic::lake_salinity_ppt(kind, max_depth_m))
                .into_iter()
                .map(|f| FishSpeciesOut {
                    slug: f.slug.into(), name: f.name.into(), binomial: f.binomial.into(),
                    zone: f.zone, real: f.real.into(), blurb: f.blurb.into(),
                })
                .collect(),
            story,
            inflows: inflows[li].clone(),
            outflow: outflow[li].clone().unwrap_or_default(),
            cx, cy,
            area_cells: n,
        });
    }
    // Largest first.
    out.sort_by(|a, b| b.area_km2.partial_cmp(&a.area_km2).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// A short base-26 letter tag (0→A, 1→B, 26→AA …) — a guaranteed-unique-per-id
/// last-resort suffix for the astronomically rare case where every name draw for
/// one river collides with an already-used name.
fn uniq_suffix(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        n /= 26;
        if n == 0 { break; }
        n -= 1;
    }
    s
}

#[cfg(test)]
mod river_system_tests {
    use super::*;
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};

    fn synth(w: u32, h: u32, terrain: Vec<u8>, elevation: Vec<f32>) -> WorldBuffer {
        let n = (w * h) as usize;
        let sea_depth: Vec<f32> = (0..n).map(|i| if terrain[i] == 0 { 0.05 } else { 0.0 }).collect();
        let is_shelf: Vec<u8> = (0..n).map(|i| (terrain[i] == 0) as u8).collect();
        WorldBuffer {
            cols: ColumnSet::ALL, width: w, height: h, tiles_x: 1, tiles_y: 1,
            equator_offset: 0.5, lat_scale: 1.0, lat_ratio: 1.0,
            terrain, elevation, sea_depth, is_shelf, is_shelf_edge: vec![0u8; n],
            locked_bits: Vec::new(), plate_index: Vec::new(), boundary_type: Vec::new(),
            is_volcanic: Vec::new(), temperature: Vec::new(),
            precipitation: vec![1200.0f32; n], koppen: vec![crate::sim::koppen::CFB; n],
            soil_type: Vec::new(), fertility: Vec::new(), fishery: Vec::new(),
            current_type: Vec::new(), wind_vx: Vec::new(), wind_vy: Vec::new(),
            current_vx: Vec::new(), current_vy: Vec::new(), distance_to_ocean: Vec::new(),
            habitability: Vec::new(), salinity: Vec::new(), shark_risk: Vec::new(),
            goods: Vec::new(), shipworm_risk: Vec::new(), storm_base: Vec::new(),
            reef_risk: Vec::new(), disease_risk: Vec::new(),
        }
    }

    #[test]
    fn river_systems_have_tributaries_and_stats() {
        let (w, h) = (60u32, 30u32);
        let n = (w * h) as usize;
        let mut terrain = vec![1u8; n];
        let mut elev = vec![0.0f32; n];
        let y0 = h as f32 / 2.0;
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if x == 0 { terrain[i] = 0; }
                elev[i] = 0.02 + x as f32 * 0.015 + ((y as f32 - y0).abs()) * 0.02;
            }
        }
        let buf = synth(w, h, terrain, elev);
        let hy = crate::sim::rivers::compute_hydrology(&buf);
        let rivers = crate::sim::rivers::extract_rivers(&buf, &hy.flow_dir, &hy.acc, 1.2, 1.0, &[]);
        assert!(!rivers.is_empty(), "the valley world should produce rivers");

        // Put a settlement right on the main valley floor near the coast.
        let settles = vec![
            RiverSettleIn { x: 6, y: (y0 as u32), name: "Rivermouth".into(), size: "city".into() },
        ];

        let systems = build_river_systems(&buf, &rivers, &settles);
        assert!(!systems.is_empty(), "should return at least one river system");
        let root = &systems[0];
        assert!(root.length_km > 0.0, "system has a positive length");
        assert!(root.discharge_m3s > 0.0, "system has a positive discharge");
        assert!(root.max_depth_m > 0.0 && root.max_width_m > 0.0, "depth/width estimated");
        assert!(!root.counterpart.is_empty(), "an Earth counterpart is assigned");
        assert!(!root.profile.is_empty(), "the elevation profile is sampled");
        // The long profile must never climb (water flows downhill source→mouth).
        for w in root.profile.windows(2) {
            assert!(w[1] <= w[0] + 1e-3, "profile rises {} → {} (must be monotone down)", w[0], w[1]);
        }
        // Ecology + a UNIQUE narrative are populated for every system.
        assert!(!root.regime.is_empty() && !root.fish.is_empty(), "regime + fish assigned");
        assert!(!root.story.is_empty(), "a NatGeo-style story is generated");
        // Distinct river systems must not share the same write-up verbatim.
        if systems.len() >= 2 {
            let a = &systems[0].story;
            let b = &systems[1].story;
            assert_ne!(a, b, "two river systems share an identical story");
        }
        // The largest system should own tributaries (the tree, not a bare trunk).
        let total_tribs: usize = systems.iter().map(|s| s.trib_total).sum();
        assert!(total_tribs >= 1, "expected tributaries in the system tree");
        // The settlement on the trunk should be attributed to some reach.
        let total_cities: usize = systems.iter().map(|s| s.city_total).sum();
        assert!(total_cities >= 1, "the trunk-side city should attach to a reach");
    }

    #[test]
    fn deeper_discharge_gives_deeper_river() {
        // Sanity on the depth model: a big-discharge river reads much deeper.
        let shallow = (0.62f32 * 50.0f32.powf(0.43)).clamp(0.3, 230.0);
        let amazon = (0.62f32 * 209000.0f32.powf(0.43)).clamp(0.3, 230.0);
        assert!(amazon > 90.0, "a great river should be very deep, got {amazon}");
        assert!(amazon > shallow * 15.0, "great rivers dwarf creeks in depth");
    }
}
