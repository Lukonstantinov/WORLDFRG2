use std::collections::{HashMap, HashSet};
use serde::Serialize;
use tauri::State;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rayon::prelude::*;
use crate::db::{WorldDb, tile_store};
use crate::render::tile_image;
use crate::tile::cell::TileData;
use crate::tile::coords::TILE_SIZE;

/// Highest supported level-of-detail. At LOD L, one response image covers
/// 2^L × 2^L base tiles (so LOD 4 → a 16×16-tile region in one 128×128 image).
const MAX_LOD: i32 = 4;

#[derive(Serialize)]
pub struct TileResponse {
    pub tx: i32,
    pub ty: i32,
    pub layer: String,
    pub version: i64,
    pub rgba: String, // base64-encoded RGBA pixels
}

#[tauri::command]
pub fn get_tiles(
    tiles: Vec<(i32, i32)>,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    let lod = lod.clamp(0, MAX_LOD);
    if lod == 0 {
        return get_tiles_full_res(tiles, layers, db);
    }
    get_supertiles(tiles, layers, lod, db)
}

/// LOD 0 fast path: each (tx, ty) is a base tile, rendered as-is.
fn get_tiles_full_res(
    tiles: Vec<(i32, i32)>,
    layers: Vec<String>,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    // Fetch the compressed blobs + versions under the lock (cheap memcpy), then
    // release it so the CPU-bound decompress → render → base64 runs in parallel
    // off-lock instead of serializing every tile behind the DB mutex.
    let raw: Vec<(i32, i32, i64, Option<Vec<u8>>)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        tiles
            .iter()
            .map(|&(tx, ty)| {
                let bv = tile_store::load_blob_with_version(&conn, tx, ty, 0)
                    .map_err(|e| e.to_string())?;
                Ok(match bv {
                    Some((version, blob)) => (tx, ty, version, Some(blob)),
                    None => (tx, ty, 0, None),
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    let results = raw
        .par_iter()
        .flat_map_iter(|(tx, ty, version, blob)| {
            let tile = match blob {
                Some(b) => TileData::decompress(b),
                None => TileData::new_sea(),
            };
            layers.iter().map(move |layer| {
                let rgba_bytes = tile_image::render_tile(&tile, layer);
                TileResponse {
                    tx: *tx,
                    ty: *ty,
                    layer: layer.clone(),
                    version: *version,
                    rgba: BASE64.encode(&rgba_bytes),
                }
            }).collect::<Vec<_>>()
        })
        .collect();

    Ok(results)
}

/// LOD > 0 path: each requested (tx, ty) is a SUPERTILE covering base tiles
/// [tx·S, tx·S+S) × [ty·S, ty·S+S) with S = 2^lod. The response is still one
/// 128×128 image per supertile per layer, built by sampling every S-th cell of
/// the underlying region — so a fully zoomed-out world costs a handful of
/// images instead of thousands.
fn get_supertiles(
    tiles: Vec<(i32, i32)>,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    let s = 1i32 << lod;

    // Deduped set of base tiles needed across all requested supertiles.
    let mut base_set: HashSet<(i32, i32)> = HashSet::new();
    for &(tx, ty) in &tiles {
        for by in (ty * s)..(ty * s + s) {
            for bx in (tx * s)..(tx * s + s) {
                base_set.insert((bx, by));
            }
        }
    }
    let base_coords: Vec<(i32, i32)> = base_set.into_iter().collect();

    // Fetch ALL required base-tile blobs under the DB lock once, then release it.
    let raw: Vec<((i32, i32), i64, Option<Vec<u8>>)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        base_coords
            .iter()
            .map(|&(bx, by)| {
                let bv = tile_store::load_blob_with_version(&conn, bx, by, 0)
                    .map_err(|e| e.to_string())?;
                Ok(match bv {
                    Some((version, blob)) => ((bx, by), version, Some(blob)),
                    None => ((bx, by), 0, None),
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    // Decompress unique base tiles in parallel off-lock. Missing tiles (outside
    // the world's tile grid) fall back to default sea, matching the LOD 0 path.
    let base: HashMap<(i32, i32), (i64, TileData)> = raw
        .into_par_iter()
        .map(|(coord, version, blob)| {
            let tile = match blob {
                Some(b) => TileData::decompress(&b),
                None => TileData::new_sea(),
            };
            (coord, (version, tile))
        })
        .collect();

    // Build the synthetic downsampled tiles and render them in parallel.
    let results = tiles
        .par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let (tile, version) = sample_supertile(tx, ty, s, &base);
            layers.iter().map(move |layer| {
                let rgba_bytes = tile_image::render_tile(&tile, layer);
                TileResponse {
                    tx,
                    ty,
                    layer: layer.clone(),
                    version,
                    rgba: BASE64.encode(&rgba_bytes),
                }
            }).collect::<Vec<_>>()
        })
        .collect();

    Ok(results)
}

/// Build a synthetic 128×128 `TileData` for supertile (tx, ty) at scale factor
/// `s` (= 2^lod) by sampling every s-th cell of the covered base-tile region:
/// output cell (lx, ly) reads global cell (tx·s·128 + lx·s, ty·s·128 + ly·s).
/// Every columnar field is copied so `render_tile` works unchanged for any
/// layer. Returns the tile plus the max version of the covered base tiles.
fn sample_supertile(
    tx: i32,
    ty: i32,
    s: i32,
    base: &HashMap<(i32, i32), (i64, TileData)>,
) -> (TileData, i64) {
    let t = TILE_SIZE as i32;
    let n = (TILE_SIZE * TILE_SIZE) as usize;
    let mut out = TileData::new_sea();

    // Match the source tiles' goods count (a loaded save may carry more goods
    // than the built-in GOODS_COUNT; decompress keeps the larger layout).
    let goods_count = base
        .values()
        .map(|(_, tile)| tile.goods.len())
        .max()
        .unwrap_or(0)
        .max(out.goods.len());
    out.goods.resize(goods_count, vec![0u8; n]);

    let mut version = 0i64;
    for by in (ty * s)..(ty * s + s) {
        for bx in (tx * s)..(tx * s + s) {
            if let Some((v, _)) = base.get(&(bx, by)) {
                version = version.max(*v);
            }
        }
    }

    for ly in 0..t {
        let gy = ty * s * t + ly * s;
        let bty = gy / t;
        let sy = gy % t;
        for lx in 0..t {
            let gx = tx * s * t + lx * s;
            let btx = gx / t;
            let sx = gx % t;
            let Some((_, src)) = base.get(&(btx, bty)) else { continue };
            let si = (sy * t + sx) as usize;
            let di = (ly * t + lx) as usize;

            out.terrain[di] = src.terrain[si];
            out.elevation[di] = src.elevation[si];
            out.sea_depth[di] = src.sea_depth[si];
            out.is_shelf[di] = src.is_shelf[si];
            out.is_shelf_edge[di] = src.is_shelf_edge[si];
            out.locked_bits[di] = src.locked_bits[si];
            out.plate_index[di] = src.plate_index[si];
            out.boundary_type[di] = src.boundary_type[si];
            out.is_volcanic[di] = src.is_volcanic[si];
            out.temperature[di] = src.temperature[si];
            out.precipitation[di] = src.precipitation[si];
            out.koppen[di] = src.koppen[si];
            out.soil_type[di] = src.soil_type[si];
            out.fertility[di] = src.fertility[si];
            out.fishery[di] = src.fishery[si];
            out.current_type[di] = src.current_type[si];
            out.wind_vx[di] = src.wind_vx[si];
            out.wind_vy[di] = src.wind_vy[si];
            out.current_vx[di] = src.current_vx[si];
            out.current_vy[di] = src.current_vy[si];
            out.distance_to_ocean[di] = src.distance_to_ocean[si];
            out.habitability[di] = src.habitability[si];
            out.salinity[di] = src.salinity[si];
            out.shark_risk[di] = src.shark_risk[si];
            for g in 0..src.goods.len().min(out.goods.len()) {
                out.goods[g][di] = src.goods[g][si];
            }
            out.shipworm_risk[di] = src.shipworm_risk[si];
            out.storm_base[di] = src.storm_base[si];
            out.reef_risk[di] = src.reef_risk[si];
            out.disease_risk[di] = src.disease_risk[si];
        }
    }

    (out, version)
}

#[tauri::command]
pub fn get_tile_range(
    tx_min: i32,
    tx_max: i32,
    ty_min: i32,
    ty_max: i32,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    // With lod > 0 the range is interpreted in supertile coordinates.
    let mut tile_coords = Vec::new();
    for ty in ty_min..=ty_max {
        for tx in tx_min..=tx_max {
            tile_coords.push((tx, ty));
        }
    }
    get_tiles(tile_coords, layers, lod, db)
}
