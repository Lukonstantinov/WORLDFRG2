use rusqlite::Connection;
use rayon::prelude::*;
use crate::db::{tile_store, metadata};
use crate::tile::coords::TILE_SIZE;
use crate::tile::cell::{TileData, GOODS_COUNT};
use crate::history::undo;

/// Equator sits at the vertical centre of the canvas by default.
pub const DEFAULT_EQUATOR_OFFSET: f32 = 0.5;
/// No stretch by default: the full ±90° span maps exactly onto the canvas
/// height when the equator is centred.
pub const DEFAULT_LAT_SCALE: f32 = 1.0;
/// Even latitude-line spacing by default (gap 30→60 ÷ gap 0→30 = 1). >1 fans the
/// poleward bands apart (Mercator-like, ~2.4); the simulation uses the SAME ratio
/// as the drawn latitude lines so every latitude-dependent layer lands exactly on
/// the lines (see `lat_from_y`, frontend `latLineY`).
pub const DEFAULT_LAT_RATIO: f32 = 1.0;

/// Map a cell row `y` to latitude in degrees given a configurable equator
/// position and latitude scale.
///
/// - `equator_offset`: where the 0° line sits, as a fraction of `height` from
///   the top (0 = top edge, 0.5 = centre, 1 = bottom edge). Moving it shifts
///   every latitude band with it.
/// - `lat_scale`: expansion factor, like dragging Fibonacci levels apart.
///   `1.0` keeps the default mapping; `>1` stretches the 0/30/60 bands further
///   from the equator so the poles fall *off-canvas* (those rows simply don't
///   exist and so are cropped); `<1` compresses them inward.
///
/// The result is clamped to `[-90, 90]`, so rows beyond a pole that happens to
/// sit inside the canvas (only possible when `lat_scale < 1`) read as the pole
/// value rather than an unphysical >90° latitude.
/// - `lat_ratio`: spacing ratio between consecutive 30° latitude bands (gap
///   30→60 ÷ gap 0→30). `1.0` is the even (linear) mapping; `>1` fans the
///   poleward bands apart so a given canvas row reads a *lower* latitude
///   (Mercator-like) — the exact inverse of the frontend `latLineY` geometric
///   model, so the simulation and the drawn lines agree everywhere.
#[inline]
pub fn lat_from_y(y: f32, height: f32, equator_offset: f32, lat_scale: f32, lat_ratio: f32) -> f32 {
    let scale = if lat_scale <= 1e-4 { 1.0 } else { lat_scale };
    let equator_row = equator_offset * height;
    let dy = equator_row - y; // >0 = north of the equator
    // Pixels for the first 30° band (equator→30°) — identical to the linear
    // spacing so `lat_ratio == 1` reproduces the original mapping exactly.
    let band = (height * scale / 6.0).max(1e-6);
    let mag = dy.abs() / band; // bands away from the equator (continuous)
    let r = lat_ratio;
    let abs_lat = if (r - 1.0).abs() < 1e-4 {
        30.0 * mag
    } else {
        // Invert the geometric sum cum = band·(rⁿ−1)/(r−1): n = ln(1+mag·(r−1))/ln r.
        let inner = 1.0 + mag * (r - 1.0);
        if inner <= 1e-6 { 90.0 } else { 30.0 * inner.ln() / r.ln() }
    };
    (dy.signum() * abs_lat).clamp(-90.0, 90.0)
}

/// Flat world-sized arrays for simulation.
/// Loads all tiles into contiguous buffers, runs simulation, writes back.
/// Index = wy * width + wx (row-major).
pub struct WorldBuffer {
    pub width: u32,
    pub height: u32,
    pub tiles_x: u32,
    pub tiles_y: u32,
    /// Latitude framing: equator position (fraction of height from top) and
    /// expansion scale. See `lat_from_y`. Loaded from world metadata so every
    /// simulation phase generates against the same configurable latitudes.
    pub equator_offset: f32,
    pub lat_scale: f32,
    /// Latitude-line spacing ratio shared with the overlay (see `lat_from_y`).
    pub lat_ratio: f32,
    // Per-cell data
    pub terrain: Vec<u8>,
    pub elevation: Vec<f32>,
    pub sea_depth: Vec<f32>,
    pub is_shelf: Vec<u8>,
    pub is_shelf_edge: Vec<u8>,
    pub locked_bits: Vec<u16>,
    pub plate_index: Vec<u16>,
    pub boundary_type: Vec<u8>,
    pub is_volcanic: Vec<u8>,
    pub temperature: Vec<f32>,
    pub precipitation: Vec<f32>,
    pub koppen: Vec<u8>,
    pub soil_type: Vec<u8>,
    pub fertility: Vec<f32>,
    pub fishery: Vec<f32>,
    pub current_type: Vec<u8>,
    pub wind_vx: Vec<f32>,
    pub wind_vy: Vec<f32>,
    pub current_vx: Vec<f32>,
    pub current_vy: Vec<f32>,
    pub distance_to_ocean: Vec<f32>,
    pub habitability: Vec<f32>,
    // ── Salinity + Biological ──
    pub salinity: Vec<u8>,         // sea: 0..255 ↔ ~28-42 PSU
    pub shark_risk: Vec<u8>,       // sea: 0..255 shark-habitat danger
    pub goods: Vec<Vec<u8>>,       // [GOODS_COUNT] trade-good intensity fields
    pub shipworm_risk: Vec<u8>,    // sea: 0..255 shipworm hull-hazard
    pub storm_base: Vec<u8>,       // sea: 0..255 annual storm/cyclone potential
    pub reef_risk: Vec<u8>,        // sea: 0..255 reef/shoal wreck hazard
    pub disease_risk: Vec<u8>,     // land: 0..255 malaria/fever risk
}

impl WorldBuffer {
    /// Load all tiles from the database into flat world arrays.
    pub fn load(conn: &Connection) -> Result<Self, String> {
        let width: u32 = metadata::get_meta_required(conn, "grid_width")
            .map_err(|e| e.to_string())?
            .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        let height: u32 = metadata::get_meta_required(conn, "grid_height")
            .map_err(|e| e.to_string())?
            .parse().map_err(|e: std::num::ParseIntError| e.to_string())?;

        // Latitude framing (optional metadata; older saves default to a
        // centred, unstretched equator so they load unchanged).
        let equator_offset: f32 = metadata::get_meta(conn, "equator_offset")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_EQUATOR_OFFSET);
        let lat_scale: f32 = metadata::get_meta(conn, "lat_scale")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_LAT_SCALE);
        let lat_ratio: f32 = metadata::get_meta(conn, "lat_ratio")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_LAT_RATIO);

        let total = (width * height) as usize;
        let tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;

        let mut buf = Self {
            width,
            height,
            tiles_x,
            tiles_y,
            equator_offset,
            lat_scale,
            lat_ratio,
            terrain: vec![0; total],
            elevation: vec![0.0; total],
            sea_depth: vec![0.0; total],
            is_shelf: vec![0; total],
            is_shelf_edge: vec![0; total],
            locked_bits: vec![0; total],
            plate_index: vec![0; total],
            boundary_type: vec![0; total],
            is_volcanic: vec![0; total],
            temperature: vec![0.0; total],
            precipitation: vec![0.0; total],
            koppen: vec![0; total],
            soil_type: vec![0; total],
            fertility: vec![0.0; total],
            fishery: vec![0.0; total],
            current_type: vec![0; total],
            wind_vx: vec![0.0; total],
            wind_vy: vec![0.0; total],
            current_vx: vec![0.0; total],
            current_vy: vec![0.0; total],
            distance_to_ocean: vec![1.0; total],
            habitability: vec![0.0; total],
            salinity: vec![0; total],
            shark_risk: vec![0; total],
            goods: vec![vec![0u8; total]; GOODS_COUNT],
            shipworm_risk: vec![0; total],
            storm_base: vec![0; total],
            reef_risk: vec![0; total],
            disease_risk: vec![0; total],
        };

        // Fetch every tile's compressed blob serially (cheap memcpy under the
        // lock), decompress them in parallel with rayon (the actual cost), then
        // scatter into the flat arrays serially.
        let mut blobs: Vec<Option<Vec<u8>>> = Vec::with_capacity((tiles_x * tiles_y) as usize);
        for ty in 0..tiles_y as i32 {
            for tx in 0..tiles_x as i32 {
                let bv = tile_store::load_blob_with_version(conn, tx, ty, 0)
                    .map_err(|e| e.to_string())?;
                blobs.push(bv.map(|(_, b)| b));
            }
        }
        // `into_par_iter` consumes each compressed blob as it's decompressed, so
        // the compressed copy of the world isn't held alongside the decompressed one.
        let tiles: Vec<TileData> = blobs
            .into_par_iter()
            .map(|b| match b {
                Some(blob) => TileData::decompress(&blob),
                None => TileData::new_sea(),
            })
            .collect();

        let mut tile_idx = 0usize;
        for ty in 0..tiles_y as i32 {
            for tx in 0..tiles_x as i32 {
                let tile = &tiles[tile_idx];
                tile_idx += 1;

                // Grow the buffer's good count to fit this tile (a world may carry
                // more than the built-in GOODS_COUNT goods).
                if tile.goods.len() > buf.goods.len() {
                    buf.goods.resize(tile.goods.len(), vec![0u8; total]);
                }

                let tile_w = TILE_SIZE.min(width - tx as u32 * TILE_SIZE) as usize;
                let tile_h = TILE_SIZE.min(height - ty as u32 * TILE_SIZE);

                // Whole-row slice copies (one memcpy per column per row) instead
                // of per-cell assignments — the goods inner loop alone made the
                // old form ~40 scattered writes per cell on large worlds.
                for ly in 0..tile_h {
                    let wi0 = ((ty as u32 * TILE_SIZE + ly) * width + tx as u32 * TILE_SIZE) as usize;
                    let ti0 = (ly * TILE_SIZE) as usize;
                    macro_rules! copy_rows {
                        ($($f:ident),* $(,)?) => { $(
                            buf.$f[wi0..wi0 + tile_w].copy_from_slice(&tile.$f[ti0..ti0 + tile_w]);
                        )* };
                    }
                    copy_rows!(
                        terrain, elevation, sea_depth, is_shelf, is_shelf_edge,
                        locked_bits, plate_index, boundary_type, is_volcanic,
                        temperature, precipitation, koppen, soil_type, fertility,
                        fishery, current_type, wind_vx, wind_vy, current_vx,
                        current_vy, distance_to_ocean, habitability, salinity,
                        shark_risk, shipworm_risk, storm_base, reef_risk, disease_risk,
                    );
                    for g in 0..tile.goods.len() {
                        buf.goods[g][wi0..wi0 + tile_w].copy_from_slice(&tile.goods[g][ti0..ti0 + tile_w]);
                    }
                }
            }
        }

        Ok(buf)
    }

    /// Save all tiles back to the database, with undo support.
    pub fn save(&self, conn: &Connection, label: &str) -> Result<Vec<(i32, i32)>, String> {
        // Save old states for undo. Snapshot the *already-compressed* blobs
        // straight from the DB (undo stores compressed bytes anyway), avoiding a
        // needless decompress→recompress of every tile on each sim step / stroke.
        let mut old_states: Vec<(i32, i32, Vec<u8>)> = Vec::new();
        for ty in 0..self.tiles_y as i32 {
            for tx in 0..self.tiles_x as i32 {
                let blob = tile_store::load_blob_with_version(conn, tx, ty, 0)
                    .map_err(|e| e.to_string())?
                    .map(|(_, b)| b)
                    .unwrap_or_else(|| TileData::new_sea().compress());
                old_states.push((tx, ty, blob));
            }
        }
        undo::push_undo(conn, label, &old_states)?;

        // Gather + compress every tile in parallel (reads `&self` only), in
        // batches so peak memory stays bounded on the largest worlds, then write
        // each batch of precompressed blobs inside one transaction instead of
        // one implicit commit per tile.
        let coords: Vec<(i32, i32)> = (0..self.tiles_y as i32)
            .flat_map(|ty| (0..self.tiles_x as i32).map(move |tx| (tx, ty)))
            .collect();

        const SAVE_BATCH: usize = 256;
        let txn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for batch in coords.chunks(SAVE_BATCH) {
            let blobs: Vec<(i32, i32, Vec<u8>)> = batch
                .par_iter()
                .map(|&(tx, ty)| (tx, ty, self.gather_tile(tx, ty).compress()))
                .collect();
            for (tx, ty, blob) in &blobs {
                tile_store::save_tile_blob(&txn, *tx, *ty, 0, blob).map_err(|e| e.to_string())?;
            }
        }
        txn.commit().map_err(|e| e.to_string())?;

        Ok(coords)
    }

    /// Rebuild one tile's columnar data from the flat world arrays.
    fn gather_tile(&self, tx: i32, ty: i32) -> TileData {
        let mut tile = TileData::new_sea();
        // Match the tile's good column count to the buffer (may exceed the
        // built-in GOODS_COUNT when the world defines custom goods).
        if self.goods.len() != tile.goods.len() {
            tile.goods.resize(self.goods.len(), vec![0u8; (TILE_SIZE * TILE_SIZE) as usize]);
        }
        let tile_w = TILE_SIZE.min(self.width - tx as u32 * TILE_SIZE) as usize;
        let tile_h = TILE_SIZE.min(self.height - ty as u32 * TILE_SIZE);

        for ly in 0..tile_h {
            let wi0 = ((ty as u32 * TILE_SIZE + ly) * self.width + tx as u32 * TILE_SIZE) as usize;
            let ti0 = (ly * TILE_SIZE) as usize;
            macro_rules! copy_rows {
                ($($f:ident),* $(,)?) => { $(
                    tile.$f[ti0..ti0 + tile_w].copy_from_slice(&self.$f[wi0..wi0 + tile_w]);
                )* };
            }
            copy_rows!(
                terrain, elevation, sea_depth, is_shelf, is_shelf_edge,
                locked_bits, plate_index, boundary_type, is_volcanic,
                temperature, precipitation, koppen, soil_type, fertility,
                fishery, current_type, wind_vx, wind_vy, current_vx,
                current_vy, distance_to_ocean, habitability, salinity,
                shark_risk, shipworm_risk, storm_base, reef_risk, disease_risk,
            );
            for g in 0..self.goods.len() {
                tile.goods[g][ti0..ti0 + tile_w].copy_from_slice(&self.goods[g][wi0..wi0 + tile_w]);
            }
        }
        tile
    }

    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Wrap x coordinate for cylindrical topology
    #[inline]
    pub fn wrap_x(&self, x: i32) -> u32 {
        ((x % self.width as i32 + self.width as i32) % self.width as i32) as u32
    }

    /// Clamp y coordinate
    #[inline]
    pub fn clamp_y(&self, y: i32) -> u32 {
        y.clamp(0, self.height as i32 - 1) as u32
    }

    /// Get index with wrapping x, clamping y
    #[inline]
    pub fn widx(&self, x: i32, y: i32) -> usize {
        let wx = self.wrap_x(x);
        let wy = self.clamp_y(y);
        self.idx(wx, wy)
    }

    /// Total number of cells
    #[inline]
    pub fn total(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Get latitude in degrees (-90 to 90) from y coordinate, honouring the
    /// configurable equator position and expansion scale.
    #[inline]
    pub fn latitude(&self, y: u32) -> f32 {
        lat_from_y(y as f32, self.height as f32, self.equator_offset, self.lat_scale, self.lat_ratio)
    }

    /// Row of the equator (0° line), clamped to the canvas.
    #[inline]
    pub fn equator_row(&self) -> u32 {
        ((self.equator_offset * self.height as f32).round() as i32)
            .clamp(0, self.height as i32 - 1) as u32
    }

    /// Get absolute latitude (0 to 90)
    #[inline]
    pub fn abs_latitude(&self, y: u32) -> f32 {
        self.latitude(y).abs()
    }
}
