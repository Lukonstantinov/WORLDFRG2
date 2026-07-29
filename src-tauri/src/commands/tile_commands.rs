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

/// One rendered tile image before response encoding (shared by the JSON and
/// packed-binary commands).
struct RawTileImage {
    tx: i32,
    ty: i32,
    layer_idx: u8, // index into the request's `layers`
    version: i64,
    rgba: Vec<u8>,
}

/// JSON/base64 response (kept for compatibility and debugging; the frontend's
/// hot path uses `get_tiles_packed`).
#[tauri::command]
pub fn get_tiles(
    tiles: Vec<(i32, i32)>,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<Vec<TileResponse>, String> {
    let raw = render_tiles_raw(tiles, &layers, lod, &db)?;
    Ok(raw
        .into_iter()
        .map(|r| TileResponse {
            tx: r.tx,
            ty: r.ty,
            layer: layers[r.layer_idx as usize].clone(),
            version: r.version,
            rgba: BASE64.encode(&r.rgba),
        })
        .collect())
}

/// Raw-bytes response: no base64 (+33 %), no `atob`, no multi-MB JSON parse on
/// the frontend. Format (little-endian): `[u32 count]` then per record
/// `[i32 tx][i32 ty][i64 version][u8 layer_idx][u8 lod][u16 size_px]
///  [u32 byte_len][byte_len RGBA bytes]`.
#[tauri::command]
pub fn get_tiles_packed(
    tiles: Vec<(i32, i32)>,
    layers: Vec<String>,
    lod: i32,
    db: State<'_, WorldDb>,
) -> Result<tauri::ipc::Response, String> {
    let lod = lod.clamp(0, MAX_LOD);
    let raw = render_tiles_raw(tiles, &layers, lod, &db)?;
    Ok(tauri::ipc::Response::new(pack_tiles(&raw, lod)))
}

fn pack_tiles(raw: &[RawTileImage], lod: i32) -> Vec<u8> {
    let total: usize = 4 + raw.iter().map(|r| 24 + r.rgba.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    for r in raw {
        out.extend_from_slice(&r.tx.to_le_bytes());
        out.extend_from_slice(&r.ty.to_le_bytes());
        out.extend_from_slice(&r.version.to_le_bytes());
        out.push(r.layer_idx);
        out.push(lod as u8);
        out.extend_from_slice(&(TILE_SIZE as u16).to_le_bytes());
        out.extend_from_slice(&(r.rgba.len() as u32).to_le_bytes());
        out.extend_from_slice(&r.rgba);
    }
    out
}

/// Render the requested tiles for every layer at the given LOD, as raw RGBA.
fn render_tiles_raw(
    tiles: Vec<(i32, i32)>,
    layers: &[String],
    lod: i32,
    db: &State<'_, WorldDb>,
) -> Result<Vec<RawTileImage>, String> {
    let lod = lod.clamp(0, MAX_LOD);
    if lod == 0 {
        render_full_res(tiles, layers, db)
    } else {
        render_supertiles(tiles, layers, lod, db)
    }
}

/// LOD 0 fast path: each (tx, ty) is a base tile, rendered as-is.
fn render_full_res(
    tiles: Vec<(i32, i32)>,
    layers: &[String],
    db: &State<'_, WorldDb>,
) -> Result<Vec<RawTileImage>, String> {
    // The "terrain" hillshade layer needs each tile's cardinal neighbours to
    // shade slopes continuously across tile boundaries (otherwise a faint grid
    // of tile seams appears). Pull the neighbour ring only when that layer is
    // requested; other layers render purely from their own tile.
    let needs_neighbors = layers.iter().any(|l| l == "terrain");

    // Deduped set of tiles to fetch: the requested tiles, plus (for terrain) the
    // 4 cardinal neighbours of each so the halo has real edge data.
    let mut want: HashSet<(i32, i32)> = tiles.iter().copied().collect();
    if needs_neighbors {
        for &(tx, ty) in &tiles {
            want.insert((tx - 1, ty));
            want.insert((tx + 1, ty));
            want.insert((tx, ty - 1));
            want.insert((tx, ty + 1));
        }
    }

    // Fetch the compressed blobs + versions under the lock (cheap memcpy), then
    // release it so the CPU-bound decompress → render runs in parallel off-lock
    // instead of serializing every tile behind the DB mutex.
    let raw: Vec<((i32, i32), i64, Option<Vec<u8>>)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        want
            .iter()
            .map(|&(tx, ty)| {
                let bv = tile_store::load_blob_with_version(&conn, tx, ty, 0)
                    .map_err(|e| e.to_string())?;
                Ok(match bv {
                    Some((version, blob)) => ((tx, ty), version, Some(blob)),
                    None => ((tx, ty), 0, None),
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    // Decompress everything (requested + neighbours) in parallel, off-lock.
    let decoded: HashMap<(i32, i32), (i64, TileData)> = raw
        .into_par_iter()
        .map(|(coord, version, blob)| {
            let tile = match blob {
                Some(b) => TileData::decompress(&b),
                None => TileData::new_sea(),
            };
            (coord, (version, tile))
        })
        .collect();

    let results = tiles
        .par_iter()
        .flat_map_iter(|&(tx, ty)| {
            let (version, tile) = decoded
                .get(&(tx, ty))
                .expect("requested tile always decoded");
            let neighbors = if needs_neighbors {
                tile_image::TileNeighbors {
                    north: decoded.get(&(tx, ty - 1)).map(|(_, t)| t),
                    south: decoded.get(&(tx, ty + 1)).map(|(_, t)| t),
                    west: decoded.get(&(tx - 1, ty)).map(|(_, t)| t),
                    east: decoded.get(&(tx + 1, ty)).map(|(_, t)| t),
                }
            } else {
                tile_image::TileNeighbors::default()
            };
            layers.iter().enumerate().map(move |(li, layer)| {
                RawTileImage {
                    tx,
                    ty,
                    layer_idx: li as u8,
                    version: *version,
                    rgba: tile_image::render_tile_with_neighbors(tile, layer, &neighbors),
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
fn render_supertiles(
    tiles: Vec<(i32, i32)>,
    layers: &[String],
    lod: i32,
    db: &State<'_, WorldDb>,
) -> Result<Vec<RawTileImage>, String> {
    let s = 1i32 << lod;

    // Probe the persisted LOD pyramid first: supertiles already downsampled are
    // one small blob each. Only misses need their base tiles fetched. (Pyramid
    // entries are dropped whenever a covered base tile is written — see
    // tile_store::save_tile_blob — so a hit is always current.)
    let mut cached: Vec<((i32, i32), i64, Vec<u8>)> = Vec::new();
    let mut misses: Vec<(i32, i32)> = Vec::new();
    let base_raw: Vec<((i32, i32), i64, Option<Vec<u8>>)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        for &(tx, ty) in &tiles {
            match tile_store::load_blob_with_version(&conn, tx, ty, lod).map_err(|e| e.to_string())? {
                Some((version, blob)) => cached.push(((tx, ty), version, blob)),
                None => misses.push((tx, ty)),
            }
        }

        // Deduped set of base tiles needed across the missing supertiles.
        let mut base_set: HashSet<(i32, i32)> = HashSet::new();
        for &(tx, ty) in &misses {
            for by in (ty * s)..(ty * s + s) {
                for bx in (tx * s)..(tx * s + s) {
                    base_set.insert((bx, by));
                }
            }
        }
        base_set
            .into_iter()
            .map(|(bx, by)| {
                let bv = tile_store::load_blob_with_version(&conn, bx, by, 0)
                    .map_err(|e| e.to_string())?;
                Ok(match bv {
                    Some((version, blob)) => ((bx, by), version, Some(blob)),
                    None => ((bx, by), 0, None),
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };

    // Decompress everything in parallel off-lock. Missing base tiles (outside
    // the world's tile grid) fall back to default sea, matching the LOD 0 path.
    let cached_tiles: Vec<((i32, i32), i64, TileData)> = cached
        .into_par_iter()
        .map(|(coord, version, blob)| (coord, version, TileData::decompress(&blob)))
        .collect();
    let base: HashMap<(i32, i32), (i64, TileData)> = base_raw
        .into_par_iter()
        .map(|(coord, version, blob)| {
            let tile = match blob {
                Some(b) => TileData::decompress(&b),
                None => TileData::new_sea(),
            };
            (coord, (version, tile))
        })
        .collect();

    // Build the missing downsampled tiles (and their compressed form for the
    // pyramid) in parallel.
    let built: Vec<((i32, i32), i64, TileData, Vec<u8>)> = misses
        .par_iter()
        .map(|&(tx, ty)| {
            let (tile, version) = sample_supertile(tx, ty, s, &base);
            let blob = tile.compress();
            ((tx, ty), version, tile, blob)
        })
        .collect();
    drop(base);

    // Persist the freshly built supertiles (best-effort cache write).
    if !built.is_empty() {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        for ((tx, ty), version, _, blob) in &built {
            let _ = tile_store::save_lod_blob(&conn, *tx, *ty, lod, *version, blob);
        }
    }

    // Render cached + built supertiles for every layer in parallel.
    let all: Vec<((i32, i32), i64, TileData)> = cached_tiles
        .into_iter()
        .chain(built.into_iter().map(|(c, v, t, _)| (c, v, t)))
        .collect();
    let results = all
        .par_iter()
        .flat_map_iter(|((tx, ty), version, tile)| {
            layers.iter().enumerate().map(move |(li, layer)| {
                RawTileImage {
                    tx: *tx,
                    ty: *ty,
                    layer_idx: li as u8,
                    version: *version,
                    rgba: tile_image::render_tile(tile, layer),
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
            out.wind_speed[di] = src.wind_speed[si];
            // Everything appended AFTER wind_speed was missing here, so the LOD
            // pyramid served zeros for it. `biome` was the visible casualty:
            // `TileData::new_sea` allocates the column non-empty and zeroed, so
            // `render_biomes` saw valid-but-zero data and fell through to
            // `koppen_fallback_biome` — the legacy 16-code mapping with the fixed
            // elevation treeline that §8.12 exists to replace. Since `lodForScale`
            // puts any scale < 0.5 at LOD ≥ 1, the 41-biome layer was invisible at
            // the default world view. `sst` rendered a uniform 0 °C and `snow_frac`
            // rendered fully transparent for the same reason.
            //
            // §3.3's rule is "new tile fields append LAST". This sampler is the
            // second place that has to know — see the completeness test below.
            out.precip_summer_frac[di] = src.precip_summer_frac[si];
            out.seasonal_amp[di] = src.seasonal_amp[si];
            out.sst[di] = src.sst[si];
            out.snow_frac[di] = src.snow_frac[si];
            out.biome[di] = src.biome[si];
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `sample_supertile` must carry EVERY columnar field into the LOD pyramid.
    ///
    /// This is the second place that has to learn about a new tile column (§3.3's
    /// "new fields append LAST" covers serialization; this covers the LOD sampler),
    /// and it was silently missed for the five fields added after `wind_speed`:
    /// `precip_summer_frac`, `seasonal_amp`, `sst`, `snow_frac` and `biome`.
    ///
    /// The failure was invisible rather than loud, which is what makes it worth a
    /// test: `TileData::new_sea` allocates those columns non-empty and zeroed, so
    /// the renderer received well-formed zeros and drew a plausible wrong map —
    /// the 41-biome layer silently degraded to the legacy Köppen fallback at every
    /// zoom below scale 0.5, which is the default world view.
    ///
    /// Method: fill a source tile with a DISTINCT non-zero value per column, sample
    /// it at lod 1, and assert nothing arrives as zero. A newly appended column that
    /// the sampler forgets fails here immediately.
    #[test]
    fn lod_sampler_carries_every_column() {
        let n = (TILE_SIZE * TILE_SIZE) as usize;
        let mut src = TileData::new_sea();

        // Distinct sentinels so a mis-wired assignment (copying the wrong column)
        // is caught as well as an omitted one.
        src.terrain = vec![1u8; n];
        src.elevation = vec![0.11f32; n];
        src.sea_depth = vec![0.12f32; n];
        src.is_shelf = vec![2u8; n];
        src.is_shelf_edge = vec![3u8; n];
        src.locked_bits = vec![4u16; n];
        src.plate_index = vec![5u16; n];
        src.boundary_type = vec![6u8; n];
        src.is_volcanic = vec![7u8; n];
        src.temperature = vec![0.13f32; n];
        src.precipitation = vec![0.14f32; n];
        src.koppen = vec![8u8; n];
        src.soil_type = vec![9u8; n];
        src.fertility = vec![0.15f32; n];
        src.fishery = vec![0.16f32; n];
        src.current_type = vec![10u8; n];
        src.wind_vx = vec![0.17f32; n];
        src.wind_vy = vec![0.18f32; n];
        src.current_vx = vec![0.19f32; n];
        src.current_vy = vec![0.20f32; n];
        src.distance_to_ocean = vec![0.21f32; n];
        src.habitability = vec![0.22f32; n];
        src.salinity = vec![11u8; n];
        src.shark_risk = vec![12u8; n];
        for g in src.goods.iter_mut() {
            *g = vec![13u8; n];
        }
        src.shipworm_risk = vec![14u8; n];
        src.storm_base = vec![15u8; n];
        src.reef_risk = vec![16u8; n];
        src.disease_risk = vec![17u8; n];
        src.wind_speed = vec![0.23f32; n];
        src.precip_summer_frac = vec![18u8; n];
        src.seasonal_amp = vec![19u8; n];
        src.sst = vec![0.24f32; n];
        src.snow_frac = vec![20u8; n];
        src.biome = vec![21u8; n];

        // A lod-1 supertile covers a 2×2 block of base tiles; fill all four so
        // every sampled cell has a source.
        let mut base: HashMap<(i32, i32), (i64, TileData)> = HashMap::new();
        for by in 0..2 {
            for bx in 0..2 {
                base.insert((bx, by), (1i64, src.clone()));
            }
        }

        let (out, _version) = sample_supertile(0, 0, 2, &base);

        macro_rules! carried {
            ($field:ident, $zero:expr) => {
                assert!(
                    out.$field.iter().all(|&v| v != $zero),
                    concat!(
                        "sample_supertile dropped `", stringify!($field),
                        "` — the LOD pyramid will serve zeros for it at every zoom \
                         below scale 0.5. Add the assignment in sample_supertile."
                    )
                );
            };
        }

        carried!(terrain, 0);
        carried!(elevation, 0.0);
        carried!(sea_depth, 0.0);
        carried!(is_shelf, 0);
        carried!(is_shelf_edge, 0);
        carried!(locked_bits, 0);
        carried!(plate_index, 0);
        carried!(boundary_type, 0);
        carried!(is_volcanic, 0);
        carried!(temperature, 0.0);
        carried!(precipitation, 0.0);
        carried!(koppen, 0);
        carried!(soil_type, 0);
        carried!(fertility, 0.0);
        carried!(fishery, 0.0);
        carried!(current_type, 0);
        carried!(wind_vx, 0.0);
        carried!(wind_vy, 0.0);
        carried!(current_vx, 0.0);
        carried!(current_vy, 0.0);
        carried!(distance_to_ocean, 0.0);
        carried!(habitability, 0.0);
        carried!(salinity, 0);
        carried!(shark_risk, 0);
        carried!(shipworm_risk, 0);
        carried!(storm_base, 0);
        carried!(reef_risk, 0);
        carried!(disease_risk, 0);
        carried!(wind_speed, 0.0);
        // The five that were actually missing.
        carried!(precip_summer_frac, 0);
        carried!(seasonal_amp, 0);
        carried!(sst, 0.0);
        carried!(snow_frac, 0);
        carried!(biome, 0);

        for (g, col) in out.goods.iter().enumerate() {
            assert!(col.iter().all(|&v| v != 0), "sample_supertile dropped goods belt {g}");
        }
    }

    /// The packed format must parse back exactly (mirrors the TS parser in
    /// TileManager.ts — keep the two in sync).
    #[test]
    fn packed_format_round_trips() {
        let raw = vec![
            RawTileImage { tx: -3, ty: 7, layer_idx: 0, version: 42, rgba: vec![1, 2, 3, 4] },
            RawTileImage { tx: 11, ty: 0, layer_idx: 2, version: i64::MAX, rgba: vec![9; 65536] },
        ];
        let buf = pack_tiles(&raw, 3);

        let count = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        assert_eq!(count, 2);
        let mut off = 4;
        for r in &raw {
            let tx = i32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
            let ty = i32::from_le_bytes(buf[off + 4..off + 8].try_into().unwrap());
            let version = i64::from_le_bytes(buf[off + 8..off + 16].try_into().unwrap());
            let layer_idx = buf[off + 16];
            let lod = buf[off + 17];
            let size_px = u16::from_le_bytes(buf[off + 18..off + 20].try_into().unwrap());
            let len = u32::from_le_bytes(buf[off + 20..off + 24].try_into().unwrap()) as usize;
            off += 24;
            assert_eq!((tx, ty, version, layer_idx, lod), (r.tx, r.ty, r.version, r.layer_idx, 3));
            assert_eq!(size_px as u32, TILE_SIZE);
            assert_eq!(&buf[off..off + len], &r.rgba[..]);
            off += len;
        }
        assert_eq!(off, buf.len());
    }
}

/// A downscaled RGBA raster of an arbitrary world rectangle (used as the terrain
/// backdrop behind the Hydrology river snip).
#[derive(Serialize)]
pub struct CropImage {
    pub data: String, // base64-encoded RGBA
    pub w: u32,
    pub h: u32,
}

/// Render an arbitrary world-cell rectangle [x0..=x1] × [y0..=y1] for one layer,
/// downscaled so the longer side is at most `max_dim`. Reuses the per-tile
/// renderer, rendering only the tiles the output actually samples (cached), so a
/// small crop is cheap. Coordinates are clamped to the world; seam-crossing
/// rectangles aren't wrapped (the river snip's bbox is in-range in practice).
#[tauri::command]
pub fn render_world_crop(
    x0: u32, y0: u32, x1: u32, y1: u32,
    layer: String, max_dim: u32,
    db: State<'_, WorldDb>,
) -> Result<CropImage, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let gw: u32 = crate::db::metadata::get_meta_required(&conn, "grid_width")
        .map_err(|e| e.to_string())?.parse().map_err(|_| "bad grid_width".to_string())?;
    let gh: u32 = crate::db::metadata::get_meta_required(&conn, "grid_height")
        .map_err(|e| e.to_string())?.parse().map_err(|_| "bad grid_height".to_string())?;

    let x0 = x0.min(gw.saturating_sub(1));
    let y0 = y0.min(gh.saturating_sub(1));
    let x1 = x1.min(gw.saturating_sub(1)).max(x0);
    let y1 = y1.min(gh.saturating_sub(1)).max(y0);
    let cw = x1 - x0 + 1;
    let ch = y1 - y0 + 1;

    let md = max_dim.clamp(16, 512) as f32;
    let scale = (md / cw.max(ch) as f32).min(1.0);
    let out_w = ((cw as f32 * scale).round() as u32).max(1);
    let out_h = ((ch as f32 * scale).round() as u32).max(1);

    let ts = TILE_SIZE;
    let tsz = TILE_SIZE as usize;
    let mut cache: HashMap<(i32, i32), Vec<u8>> = HashMap::new();
    let mut out = vec![0u8; (out_w * out_h * 4) as usize];

    for oy in 0..out_h {
        let wy = (y0 + (oy * ch) / out_h).min(y1);
        let ty = (wy / ts) as i32;
        let ly = (wy % ts) as usize;
        for ox in 0..out_w {
            let wx = (x0 + (ox * cw) / out_w).min(x1);
            let tx = (wx / ts) as i32;
            let lx = (wx % ts) as usize;
            if !cache.contains_key(&(tx, ty)) {
                let tile = match tile_store::load_blob_with_version(&conn, tx, ty, 0)
                    .map_err(|e| e.to_string())?
                {
                    Some((_, blob)) => TileData::decompress(&blob),
                    None => TileData::new_sea(),
                };
                cache.insert((tx, ty), tile_image::render_tile(&tile, &layer));
            }
            let buf = &cache[&(tx, ty)];
            let si = (ly * tsz + lx) * 4;
            let di = ((oy * out_w + ox) * 4) as usize;
            out[di..di + 4].copy_from_slice(&buf[si..si + 4]);
        }
    }

    Ok(CropImage { data: BASE64.encode(&out), w: out_w, h: out_h })
}
