use rusqlite::Connection;
use rayon::prelude::*;
use crate::db::{tile_store, metadata};
use crate::tile::coords::TILE_SIZE;
use crate::tile::cell::TileData;
use crate::sim::world_buffer::{lat_from_y, DEFAULT_EQUATOR_OFFSET, DEFAULT_LAT_SCALE, DEFAULT_LAT_RATIO};

/// All of a world's tiles, decompressed once and shared (via `Arc`) by every
/// read-only overlay/query command. Rebuilt only when the tile data changes (see
/// `WorldDb::cached_tiles_with_conn`), so the common case — many overlay queries
/// after one sim step — decompresses the world a single time instead of once per
/// command.
pub struct WorldTiles {
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub equator_offset: f32,
    pub lat_scale: f32,
    pub lat_ratio: f32,
    pub tiles: Vec<TileData>, // row-major, len = tiles_x * tiles_y
    /// Tile-table fingerprint this snapshot was built from (set by
    /// `WorldDb::cached_tiles_with_conn`); used to key the coarse cost cache.
    pub fingerprint: (i64, i64),
}

impl WorldTiles {
    /// Decompress every tile into a shared snapshot. Blobs are fetched serially
    /// (cheap memcpy) and decompressed in parallel with rayon.
    pub fn load(conn: &Connection) -> Result<Self, String> {
        let width: u32 = metadata::get_meta_required(conn, "grid_width")
            .map_err(|e| e.to_string())?
            .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        let height: u32 = metadata::get_meta_required(conn, "grid_height")
            .map_err(|e| e.to_string())?
            .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        let equator_offset: f32 = metadata::get_meta(conn, "equator_offset")
            .ok().flatten().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_EQUATOR_OFFSET);
        let lat_scale: f32 = metadata::get_meta(conn, "lat_scale")
            .ok().flatten().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_LAT_SCALE);
        let lat_ratio: f32 = metadata::get_meta(conn, "lat_ratio")
            .ok().flatten().and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_LAT_RATIO);

        let tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;

        let mut blobs: Vec<Option<Vec<u8>>> = Vec::with_capacity((tiles_x * tiles_y) as usize);
        for ty in 0..tiles_y as i32 {
            for tx in 0..tiles_x as i32 {
                let bv = tile_store::load_blob_with_version(conn, tx, ty, 0)
                    .map_err(|e| e.to_string())?;
                blobs.push(bv.map(|(_, b)| b));
            }
        }

        let tiles: Vec<TileData> = blobs
            .par_iter()
            .map(|b| match b {
                Some(blob) => TileData::decompress(blob),
                None => TileData::new_sea(),
            })
            .collect();

        Ok(Self { width, height, tiles_x, tiles_y, equator_offset, lat_scale, lat_ratio, tiles, fingerprint: (0, 0) })
    }

    /// Borrow the tile at coarse tile coords (tx, ty).
    #[inline]
    pub fn tile(&self, tx: i32, ty: i32) -> &TileData {
        &self.tiles[(ty as u32 * self.tiles_x + tx as u32) as usize]
    }

    /// Latitude (degrees) of a world row, honouring the world's equator framing.
    #[inline]
    pub fn latitude(&self, y: u32) -> f32 {
        lat_from_y(y as f32, self.height as f32, self.equator_offset, self.lat_scale, self.lat_ratio)
    }
}
