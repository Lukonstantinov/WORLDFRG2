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
/// Axial tilt (obliquity, degrees) driving the seasonal insolation cycle. Earth's
/// 23.44° by default; higher values give more extreme seasons (and push the tropic/
/// polar circles toward the poles/equator), 0° gives no seasons at all. Read from
/// world metadata so the energy-balance seasonality (`temperature.rs`) responds to it.
pub const DEFAULT_OBLIQUITY: f32 = 23.44;

// ── Planetary state (energy budget + general circulation) ───────────────────
// These four drive the *emergent* climate through the anomaly-coupled energy-
// balance model (`ebm.rs`) and the rotation-driven general circulation
// (`circulation.rs`). At their Earth defaults every downstream field is
// bit-for-bit what it was before these existed (the anomaly is zero), so old
// saves and the Earth calibration are untouched; changing one shifts the climate
// by the physically-correct amount. All read from world metadata.
//
/// Planetary rotation rate as a multiple of Earth's. Sets the Coriolis strength
/// and therefore the width of the Hadley cell / the latitudes of the wind &
/// pressure belts and the number of circulation cells. <1 = slow rotator (wide
/// Hadley, deserts & subtropical highs pushed poleward, fewer/broader belts);
/// >1 = fast rotator (narrow banded circulation).
pub const DEFAULT_ROTATION_RATE: f32 = 1.0;
/// Stellar irradiance at the planet as a multiple of Earth's solar constant
/// (1361 W/m²). Scales total insolation → global-mean temperature. <1 = a fainter
/// star / wider orbit (colder); >1 = a brighter star / closer orbit (hotter).
pub const DEFAULT_SOLAR_LUM: f32 = 1.0;
/// Greenhouse factor as a multiple of Earth's. Scales the long-wave trapping
/// (lowers the effective outgoing-radiation intercept), i.e. the atmosphere's
/// CO₂/H₂O warming. 1.0 = Earth; >1 warms the whole planet and flattens the
/// equator-pole gradient; <1 cools it (snowball at the low end).
pub const DEFAULT_GREENHOUSE: f32 = 1.0;
/// Orbital eccentricity (0 = circular). With `perihelion` it sets the asymmetry
/// between the hemispheres' season lengths/intensities. Earth ≈ 0.0167.
pub const DEFAULT_ECCENTRICITY: f32 = 0.0167;
/// Global aridity multiplier applied to the final annual precipitation total
/// (`precipitation.rs`). 1.0 = Earth (no-op); >1 = a drier world (deserts
/// expand), <1 = a wetter world (rainforest/monsoon belts expand). This is a
/// coarse global knob, not a physical mechanism — it does not feed back into
/// the moisture-advection/recycling budget, only the final total.
pub const DEFAULT_DRYNESS: f32 = 1.0;

/// Encoding scale for the `seasonal_amp` column: stored `u8 = span_°C × 2` (so the
/// full seasonal span, warmest−coldest month, is representable up to ~127 °C at
/// 0.5 °C resolution — enough headroom for extreme high-obliquity worlds). Shared by
/// `temperature.rs` (writer) and `koppen.rs` (reader) so the encoding stays in sync.
pub const SEASON_AMP_SCALE: f32 = 2.0;

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

/// Which WorldBuffer columns a simulation phase needs. A fully loaded buffer is
/// ~2.7 GB at 7200×3600 (the 38+-column goods block alone is ~1 GB), but most
/// phases touch a dozen columns — loading only those keeps peak RAM in check.
/// Columns NOT in the set stay as empty `Vec`s; `save()` merges them back from
/// each tile's previous blob so tiles are always written complete.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColumnSet(pub u64);

impl ColumnSet {
    pub const TERRAIN: ColumnSet = ColumnSet(1 << 0);
    pub const ELEVATION: ColumnSet = ColumnSet(1 << 1);
    pub const SEA_DEPTH: ColumnSet = ColumnSet(1 << 2);
    /// is_shelf + is_shelf_edge
    pub const SHELF: ColumnSet = ColumnSet(1 << 3);
    pub const LOCKED: ColumnSet = ColumnSet(1 << 4);
    /// plate_index + boundary_type
    pub const PLATES: ColumnSet = ColumnSet(1 << 5);
    pub const VOLCANIC: ColumnSet = ColumnSet(1 << 6);
    pub const TEMPERATURE: ColumnSet = ColumnSet(1 << 7);
    pub const PRECIPITATION: ColumnSet = ColumnSet(1 << 8);
    pub const KOPPEN: ColumnSet = ColumnSet(1 << 9);
    pub const SOIL: ColumnSet = ColumnSet(1 << 10);
    pub const FERTILITY: ColumnSet = ColumnSet(1 << 11);
    pub const FISHERY: ColumnSet = ColumnSet(1 << 12);
    /// wind_vx + wind_vy + wind_speed
    pub const WIND: ColumnSet = ColumnSet(1 << 13);
    /// current_type + current_vx + current_vy
    pub const CURRENTS: ColumnSet = ColumnSet(1 << 14);
    pub const DIST_OCEAN: ColumnSet = ColumnSet(1 << 15);
    pub const HABITABILITY: ColumnSet = ColumnSet(1 << 16);
    pub const SALINITY: ColumnSet = ColumnSet(1 << 17);
    pub const SHARK: ColumnSet = ColumnSet(1 << 18);
    /// All trade-good belt columns (the big one: ~1 GB at max world size).
    pub const GOODS: ColumnSet = ColumnSet(1 << 19);
    pub const SHIPWORM: ColumnSet = ColumnSet(1 << 20);
    pub const STORM: ColumnSet = ColumnSet(1 << 21);
    pub const REEF: ColumnSet = ColumnSet(1 << 22);
    pub const DISEASE: ColumnSet = ColumnSet(1 << 23);
    /// precip_summer_frac: per-land-cell summer share of annual precip (0..255).
    /// Written by precipitation.rs (two-season advection), read by koppen.rs.
    pub const SEASON: ColumnSet = ColumnSet(1 << 24);
    /// seasonal_amp: per-land-cell seasonal half-range of monthly temperature
    /// (°C ×4). Written by temperature.rs (energy-balance seasonality), read by
    /// koppen.rs::seasonal_temps for the C/D/E split & growing-season proxy.
    pub const SEASON_AMP: ColumnSet = ColumnSet(1 << 25);
    /// sst: per-sea-cell annual-mean sea-surface temperature (°C). Written by
    /// ocean.rs (latitude estimate + current anomaly), used by the thermohaline
    /// density coupling and an optional render layer.
    pub const SST: ColumnSet = ColumnSet(1 << 26);
    /// snow_frac: per-land-cell annual snow-cover fraction (×255). Written by
    /// temperature.rs (ice-albedo feedback), used for the optional snow layer.
    pub const SNOW: ColumnSet = ColumnSet(1 << 27);

    pub const NONE: ColumnSet = ColumnSet(0);
    pub const ALL: ColumnSet = ColumnSet((1 << 28) - 1);

    // ── Per-phase masks (derived from a mechanical grep of `buf.<column>` in
    // each sim module — see the redesign plan). Each is the union of the columns
    // the phase reads OR writes; commands chaining several modules union them. ──
    /// plates.rs: generate_plates_and_landmass / invert_terrain
    pub const PHASE_PLATES: ColumnSet =
        ColumnSet(Self::TERRAIN.0 | Self::ELEVATION.0 | Self::PLATES.0 | Self::VOLCANIC.0);
    /// elevation.rs: all elevation/sea-depth/shelf functions
    pub const PHASE_ELEVATION: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::ELEVATION.0 | Self::PLATES.0 | Self::SEA_DEPTH.0 | Self::SHELF.0,
    );
    /// ocean.rs + temperature.rs + precipitation.rs (phase 3 chain)
    pub const PHASE_OCEAN_ATMOSPHERE: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::ELEVATION.0 | Self::SHELF.0 | Self::WIND.0 | Self::CURRENTS.0
            | Self::SALINITY.0 | Self::DIST_OCEAN.0 | Self::TEMPERATURE.0 | Self::PRECIPITATION.0
            | Self::SEASON.0 // precipitation.rs writes the summer-fraction column
            | Self::SEASON_AMP.0 | Self::SST.0 | Self::SNOW.0, // energy-balance seasonality
    );
    /// koppen.rs
    pub const PHASE_CLIMATE: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::ELEVATION.0 | Self::TEMPERATURE.0 | Self::PRECIPITATION.0
            | Self::DIST_OCEAN.0 | Self::CURRENTS.0 | Self::WIND.0 | Self::KOPPEN.0
            | Self::SHELF.0 // shelf seas don't count as open ocean for continentality
            | Self::SEASON.0 // koppen.rs reads the summer-fraction split
            | Self::SEASON_AMP.0, // koppen.rs reads the energy-balance seasonal amplitude
    );
    /// rivers.rs (hydrology/lakes — read-only, no tile write-back).
    /// PRECIPITATION is needed by `extract_rivers` to size channels by discharge.
    pub const PHASE_RIVERS: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::ELEVATION.0 | Self::SEA_DEPTH.0 | Self::SHELF.0 | Self::KOPPEN.0
            | Self::PRECIPITATION.0,
    );
    /// soil.rs + fertility.rs (phase 6 chain)
    pub const PHASE_SOIL_FERTILITY: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::KOPPEN.0 | Self::VOLCANIC.0 | Self::SOIL.0 | Self::SHELF.0
            | Self::SEA_DEPTH.0 | Self::CURRENTS.0 | Self::DIST_OCEAN.0 | Self::PRECIPITATION.0
            | Self::TEMPERATURE.0 | Self::FERTILITY.0 | Self::FISHERY.0,
    );
    /// settlements.rs + rivers re-derivation + biological::compute_disease_risk
    pub const PHASE_SETTLEMENTS: ColumnSet = ColumnSet(
        Self::PHASE_RIVERS.0 | Self::VOLCANIC.0 | Self::DIST_OCEAN.0 | Self::PRECIPITATION.0
            | Self::TEMPERATURE.0 | Self::FERTILITY.0 | Self::FISHERY.0 | Self::HABITABILITY.0
            | Self::DISEASE.0
            | Self::SEASON_AMP.0, // growing_season_months() -> seasonal_temps() reads it
    );
    /// provinces.rs (watershed partition — phase 7b). Needs terrain/elevation for the
    /// flood, koppen/fertility/fishery/disease/dist_ocean for food capacity + province
    /// stats, temperature/season_amp for growing_season_months, goods for good quality,
    /// shelf for upwind_is_open_ocean (optional — safe if empty).
    pub const PHASE_PROVINCES: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::ELEVATION.0 | Self::KOPPEN.0 | Self::SHELF.0
            | Self::FERTILITY.0 | Self::FISHERY.0 | Self::DIST_OCEAN.0
            | Self::TEMPERATURE.0 | Self::SEASON_AMP.0 | Self::DISEASE.0 | Self::GOODS.0,
    );
    /// biological.rs (sharks/shipworms/storms/reefs/goods — phase 8)
    pub const PHASE_BIOLOGICAL: ColumnSet = ColumnSet(
        Self::TERRAIN.0 | Self::ELEVATION.0 | Self::SEA_DEPTH.0 | Self::SHELF.0 | Self::CURRENTS.0
            | Self::DIST_OCEAN.0 | Self::TEMPERATURE.0 | Self::PRECIPITATION.0 | Self::KOPPEN.0
            | Self::VOLCANIC.0 | Self::FERTILITY.0 | Self::FISHERY.0 | Self::SALINITY.0
            | Self::HABITABILITY.0 | Self::SHARK.0 | Self::SHIPWORM.0 | Self::STORM.0
            | Self::REEF.0 | Self::DISEASE.0 | Self::GOODS.0,
    );

    #[inline]
    pub fn has(self, other: ColumnSet) -> bool {
        self.0 & other.0 != 0
    }
}

impl std::ops::BitOr for ColumnSet {
    type Output = ColumnSet;
    #[inline]
    fn bitor(self, rhs: ColumnSet) -> ColumnSet {
        ColumnSet(self.0 | rhs.0)
    }
}

/// Flat world-sized arrays for simulation.
/// Loads all tiles into contiguous buffers, runs simulation, writes back.
/// Index = wy * width + wx (row-major).
pub struct WorldBuffer {
    /// Which columns were loaded (and so which `save()` writes back; the rest
    /// are merged unchanged from each tile's previous blob).
    pub cols: ColumnSet,
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
    /// Axial tilt (obliquity, degrees) driving the seasonal insolation cycle. See
    /// `DEFAULT_OBLIQUITY`. Loaded from world metadata.
    pub obliquity: f32,
    /// Planetary rotation rate (× Earth). Drives the general circulation belts. See
    /// `DEFAULT_ROTATION_RATE`.
    pub rotation_rate: f32,
    /// Stellar irradiance (× Earth solar constant). See `DEFAULT_SOLAR_LUM`.
    pub solar_lum: f32,
    /// Greenhouse factor (× Earth). See `DEFAULT_GREENHOUSE`.
    pub greenhouse: f32,
    /// Orbital eccentricity. See `DEFAULT_ECCENTRICITY`.
    pub eccentricity: f32,
    /// Global aridity multiplier. See `DEFAULT_DRYNESS`.
    pub dryness: f32,
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
    /// Low-level wind speed (m/s, ~0-35) including jets. `wind_vx/vy` stay unit
    /// direction vectors; this carries the intensity used by the Wind Speed layer
    /// and the jet entrance-dry / exit-wet precipitation coupling.
    pub wind_speed: Vec<f32>,
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
    pub precip_summer_frac: Vec<u8>, // land: summer share of annual precip ×255 (seasonality)
    // ── Thermal realism (energy-balance seasonality) ──
    pub seasonal_amp: Vec<u8>,     // land: seasonal half-range of monthly temp (°C ×4)
    pub sst: Vec<f32>,             // sea: annual-mean sea-surface temperature (°C)
    pub snow_frac: Vec<u8>,        // land: annual snow-cover fraction ×255
}

impl WorldBuffer {
    /// Load ALL columns (the safe default for callers that don't know better,
    /// and for the run-all pipelines that genuinely touch everything).
    pub fn load(conn: &Connection) -> Result<Self, String> {
        Self::load_with(conn, ColumnSet::ALL)
    }

    /// Load only the columns in `cols` from the database into flat world arrays.
    /// Unloaded columns stay as empty `Vec`s — touching one is a programming
    /// error (the per-phase masks are derived from actual column usage).
    pub fn load_with(conn: &Connection, cols: ColumnSet) -> Result<Self, String> {
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
        let obliquity: f32 = metadata::get_meta(conn, "obliquity_deg")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_OBLIQUITY);
        // Planetary state (all optional; old saves default to Earth so their
        // anomaly is zero and every field loads exactly as before).
        let rotation_rate: f32 = metadata::get_meta(conn, "rotation_rate")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_ROTATION_RATE);
        let solar_lum: f32 = metadata::get_meta(conn, "solar_lum")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_SOLAR_LUM);
        let greenhouse: f32 = metadata::get_meta(conn, "greenhouse")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_GREENHOUSE);
        let eccentricity: f32 = metadata::get_meta(conn, "eccentricity")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_ECCENTRICITY);
        let dryness: f32 = metadata::get_meta(conn, "dryness")
            .ok().flatten().and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_DRYNESS);

        let total = (width * height) as usize;
        let tiles_x = (width + TILE_SIZE - 1) / TILE_SIZE;
        let tiles_y = (height + TILE_SIZE - 1) / TILE_SIZE;

        // Allocate only the requested columns; the rest stay empty.
        let u8s = |c: ColumnSet| if cols.has(c) { vec![0u8; total] } else { Vec::new() };
        let u16s = |c: ColumnSet| if cols.has(c) { vec![0u16; total] } else { Vec::new() };
        let f32s = |c: ColumnSet, fill: f32| if cols.has(c) { vec![fill; total] } else { Vec::new() };

        let mut buf = Self {
            cols,
            width,
            height,
            tiles_x,
            tiles_y,
            equator_offset,
            lat_scale,
            lat_ratio,
            obliquity,
            rotation_rate,
            solar_lum,
            greenhouse,
            eccentricity,
            dryness,
            terrain: u8s(ColumnSet::TERRAIN),
            elevation: f32s(ColumnSet::ELEVATION, 0.0),
            sea_depth: f32s(ColumnSet::SEA_DEPTH, 0.0),
            is_shelf: u8s(ColumnSet::SHELF),
            is_shelf_edge: u8s(ColumnSet::SHELF),
            locked_bits: u16s(ColumnSet::LOCKED),
            plate_index: u16s(ColumnSet::PLATES),
            boundary_type: u8s(ColumnSet::PLATES),
            is_volcanic: u8s(ColumnSet::VOLCANIC),
            temperature: f32s(ColumnSet::TEMPERATURE, 0.0),
            precipitation: f32s(ColumnSet::PRECIPITATION, 0.0),
            koppen: u8s(ColumnSet::KOPPEN),
            soil_type: u8s(ColumnSet::SOIL),
            fertility: f32s(ColumnSet::FERTILITY, 0.0),
            fishery: f32s(ColumnSet::FISHERY, 0.0),
            current_type: u8s(ColumnSet::CURRENTS),
            wind_vx: f32s(ColumnSet::WIND, 0.0),
            wind_vy: f32s(ColumnSet::WIND, 0.0),
            wind_speed: f32s(ColumnSet::WIND, 0.0),
            current_vx: f32s(ColumnSet::CURRENTS, 0.0),
            current_vy: f32s(ColumnSet::CURRENTS, 0.0),
            distance_to_ocean: f32s(ColumnSet::DIST_OCEAN, 1.0),
            habitability: f32s(ColumnSet::HABITABILITY, 0.0),
            salinity: u8s(ColumnSet::SALINITY),
            shark_risk: u8s(ColumnSet::SHARK),
            goods: if cols.has(ColumnSet::GOODS) {
                vec![vec![0u8; total]; GOODS_COUNT]
            } else {
                Vec::new()
            },
            shipworm_risk: u8s(ColumnSet::SHIPWORM),
            storm_base: u8s(ColumnSet::STORM),
            reef_risk: u8s(ColumnSet::REEF),
            disease_risk: u8s(ColumnSet::DISEASE),
            precip_summer_frac: u8s(ColumnSet::SEASON),
            seasonal_amp: u8s(ColumnSet::SEASON_AMP),
            sst: f32s(ColumnSet::SST, 0.0),
            snow_frac: u8s(ColumnSet::SNOW),
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
                if cols.has(ColumnSet::GOODS) && tile.goods.len() > buf.goods.len() {
                    buf.goods.resize(tile.goods.len(), vec![0u8; total]);
                }

                let tile_w = TILE_SIZE.min(width - tx as u32 * TILE_SIZE) as usize;
                let tile_h = TILE_SIZE.min(height - ty as u32 * TILE_SIZE);

                // Whole-row slice copies (one memcpy per column per row) instead
                // of per-cell assignments — the goods inner loop alone made the
                // old form ~40 scattered writes per cell on large worlds. Each
                // column group is copied only when it was loaded.
                for ly in 0..tile_h {
                    let wi0 = ((ty as u32 * TILE_SIZE + ly) * width + tx as u32 * TILE_SIZE) as usize;
                    let ti0 = (ly * TILE_SIZE) as usize;
                    macro_rules! copy_rows {
                        ($c:expr => $($f:ident),* $(,)?) => {
                            if cols.has($c) { $(
                                buf.$f[wi0..wi0 + tile_w].copy_from_slice(&tile.$f[ti0..ti0 + tile_w]);
                            )* }
                        };
                    }
                    copy_rows!(ColumnSet::TERRAIN => terrain);
                    copy_rows!(ColumnSet::ELEVATION => elevation);
                    copy_rows!(ColumnSet::SEA_DEPTH => sea_depth);
                    copy_rows!(ColumnSet::SHELF => is_shelf, is_shelf_edge);
                    copy_rows!(ColumnSet::LOCKED => locked_bits);
                    copy_rows!(ColumnSet::PLATES => plate_index, boundary_type);
                    copy_rows!(ColumnSet::VOLCANIC => is_volcanic);
                    copy_rows!(ColumnSet::TEMPERATURE => temperature);
                    copy_rows!(ColumnSet::PRECIPITATION => precipitation);
                    copy_rows!(ColumnSet::KOPPEN => koppen);
                    copy_rows!(ColumnSet::SOIL => soil_type);
                    copy_rows!(ColumnSet::FERTILITY => fertility);
                    copy_rows!(ColumnSet::FISHERY => fishery);
                    copy_rows!(ColumnSet::WIND => wind_vx, wind_vy, wind_speed);
                    copy_rows!(ColumnSet::CURRENTS => current_type, current_vx, current_vy);
                    copy_rows!(ColumnSet::DIST_OCEAN => distance_to_ocean);
                    copy_rows!(ColumnSet::HABITABILITY => habitability);
                    copy_rows!(ColumnSet::SALINITY => salinity);
                    copy_rows!(ColumnSet::SHARK => shark_risk);
                    copy_rows!(ColumnSet::SHIPWORM => shipworm_risk);
                    copy_rows!(ColumnSet::STORM => storm_base);
                    copy_rows!(ColumnSet::REEF => reef_risk);
                    copy_rows!(ColumnSet::DISEASE => disease_risk);
                    copy_rows!(ColumnSet::SEASON => precip_summer_frac);
                    copy_rows!(ColumnSet::SEASON_AMP => seasonal_amp);
                    copy_rows!(ColumnSet::SST => sst);
                    copy_rows!(ColumnSet::SNOW => snow_frac);
                    if cols.has(ColumnSet::GOODS) {
                        for g in 0..tile.goods.len() {
                            buf.goods[g][wi0..wi0 + tile_w].copy_from_slice(&tile.goods[g][ti0..ti0 + tile_w]);
                        }
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
        //
        // Partially loaded buffers must still write COMPLETE tiles: each tile
        // starts from its previous blob (already fetched above for the undo
        // snapshot — `old_states` is in the same row-major order) and only the
        // loaded columns are overwritten. A fully loaded buffer skips the
        // decompress and rebuilds from scratch.
        let coords: Vec<(i32, i32)> = (0..self.tiles_y as i32)
            .flat_map(|ty| (0..self.tiles_x as i32).map(move |tx| (tx, ty)))
            .collect();
        let full = self.cols == ColumnSet::ALL;

        const SAVE_BATCH: usize = 256;
        let txn = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        for (chunk_idx, batch) in coords.chunks(SAVE_BATCH).enumerate() {
            let start = chunk_idx * SAVE_BATCH;
            let blobs: Vec<(i32, i32, Vec<u8>)> = batch
                .par_iter()
                .enumerate()
                .map(|(i, &(tx, ty))| {
                    let base = if full {
                        TileData::new_sea()
                    } else {
                        TileData::decompress(&old_states[start + i].2)
                    };
                    (tx, ty, self.gather_tile(tx, ty, base).compress())
                })
                .collect();
            for (tx, ty, blob) in &blobs {
                tile_store::save_tile_blob(&txn, *tx, *ty, 0, blob).map_err(|e| e.to_string())?;
            }
        }
        txn.commit().map_err(|e| e.to_string())?;

        Ok(coords)
    }

    /// Overwrite the loaded columns of `tile` from the flat world arrays
    /// (columns outside `self.cols` are left exactly as they were).
    fn gather_tile(&self, tx: i32, ty: i32, mut tile: TileData) -> TileData {
        // Match the tile's good column count to the buffer (may exceed the
        // built-in GOODS_COUNT when the world defines custom goods).
        if self.cols.has(ColumnSet::GOODS) && self.goods.len() != tile.goods.len() {
            tile.goods.resize(self.goods.len(), vec![0u8; (TILE_SIZE * TILE_SIZE) as usize]);
        }
        let tile_w = TILE_SIZE.min(self.width - tx as u32 * TILE_SIZE) as usize;
        let tile_h = TILE_SIZE.min(self.height - ty as u32 * TILE_SIZE);

        for ly in 0..tile_h {
            let wi0 = ((ty as u32 * TILE_SIZE + ly) * self.width + tx as u32 * TILE_SIZE) as usize;
            let ti0 = (ly * TILE_SIZE) as usize;
            macro_rules! copy_rows {
                ($c:expr => $($f:ident),* $(,)?) => {
                    if self.cols.has($c) { $(
                        tile.$f[ti0..ti0 + tile_w].copy_from_slice(&self.$f[wi0..wi0 + tile_w]);
                    )* }
                };
            }
            copy_rows!(ColumnSet::TERRAIN => terrain);
            copy_rows!(ColumnSet::ELEVATION => elevation);
            copy_rows!(ColumnSet::SEA_DEPTH => sea_depth);
            copy_rows!(ColumnSet::SHELF => is_shelf, is_shelf_edge);
            copy_rows!(ColumnSet::LOCKED => locked_bits);
            copy_rows!(ColumnSet::PLATES => plate_index, boundary_type);
            copy_rows!(ColumnSet::VOLCANIC => is_volcanic);
            copy_rows!(ColumnSet::TEMPERATURE => temperature);
            copy_rows!(ColumnSet::PRECIPITATION => precipitation);
            copy_rows!(ColumnSet::KOPPEN => koppen);
            copy_rows!(ColumnSet::SOIL => soil_type);
            copy_rows!(ColumnSet::FERTILITY => fertility);
            copy_rows!(ColumnSet::FISHERY => fishery);
            copy_rows!(ColumnSet::WIND => wind_vx, wind_vy, wind_speed);
            copy_rows!(ColumnSet::CURRENTS => current_type, current_vx, current_vy);
            copy_rows!(ColumnSet::DIST_OCEAN => distance_to_ocean);
            copy_rows!(ColumnSet::HABITABILITY => habitability);
            copy_rows!(ColumnSet::SALINITY => salinity);
            copy_rows!(ColumnSet::SHARK => shark_risk);
            copy_rows!(ColumnSet::SHIPWORM => shipworm_risk);
            copy_rows!(ColumnSet::STORM => storm_base);
            copy_rows!(ColumnSet::REEF => reef_risk);
            copy_rows!(ColumnSet::DISEASE => disease_risk);
            copy_rows!(ColumnSet::SEASON => precip_summer_frac);
            copy_rows!(ColumnSet::SEASON_AMP => seasonal_amp);
            copy_rows!(ColumnSet::SST => sst);
            copy_rows!(ColumnSet::SNOW => snow_frac);
            if self.cols.has(ColumnSet::GOODS) {
                for g in 0..self.goods.len() {
                    tile.goods[g][ti0..ti0 + tile_w].copy_from_slice(&self.goods[g][wi0..wi0 + tile_w]);
                }
            }
        }
        tile
    }

    #[inline]
    pub fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Wrap x coordinate for cylindrical topology.
    ///
    /// `width` is a runtime value, so the naive double-`%` form compiles to two
    /// hardware integer divisions (~20-40 cycles each) — and this is called
    /// millions of times per pass in the ocean/precipitation streamline loops,
    /// where the argument is almost always already in range or at most one world
    /// away. Handle those cases with a compare and a subtract; only a genuinely
    /// far-out coordinate pays for the division. The result is unchanged.
    #[inline]
    pub fn wrap_x(&self, x: i32) -> u32 {
        let w = self.width as i32;
        if x >= 0 {
            if x < w { return x as u32; }
            if x < 2 * w { return (x - w) as u32; }
        } else if x >= -w {
            return (x + w) as u32;
        }
        ((x % w + w) % w) as u32
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn test_world(w: u32, h: u32) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            )
            .unwrap();
        }
        conn
    }

    /// A partial-mask save must overwrite ONLY the loaded columns and preserve
    /// every other column from the previous tile blobs.
    #[test]
    fn masked_save_merges_unloaded_columns() {
        // 200×150 spans 2×2 tiles incl. partial edge tiles.
        let conn = test_world(200, 150);

        // Seed every cell with distinguishable values through a FULL buffer.
        let mut full = WorldBuffer::load(&conn).unwrap();
        let total = full.total();
        for i in 0..total {
            full.terrain[i] = (i % 2) as u8;
            full.temperature[i] = i as f32 * 0.5;
            full.fertility[i] = i as f32 * 0.25;
            full.koppen[i] = (i % 30) as u8;
            full.goods[3][i] = (i % 255) as u8;
            full.locked_bits[i] = (i % 7) as u16;
        }
        full.save(&conn, "seed").unwrap();

        // Load a narrow mask, change what it covers.
        let mask = ColumnSet::TERRAIN | ColumnSet::TEMPERATURE;
        let mut part = WorldBuffer::load_with(&conn, mask).unwrap();
        assert_eq!(part.terrain[5], 1);
        assert_eq!(part.temperature[8], 4.0);
        assert!(part.fertility.is_empty(), "unloaded column must stay empty");
        assert!(part.goods.is_empty(), "unloaded goods must stay empty");
        for i in 0..total {
            part.terrain[i] = 1;
            part.temperature[i] = -7.0;
        }
        part.save(&conn, "partial").unwrap();

        // Everything outside the mask must have survived untouched.
        let back = WorldBuffer::load(&conn).unwrap();
        for i in 0..total {
            assert_eq!(back.terrain[i], 1);
            assert_eq!(back.temperature[i], -7.0);
            assert_eq!(back.fertility[i], i as f32 * 0.25, "fertility lost at {i}");
            assert_eq!(back.koppen[i], (i % 30) as u8, "koppen lost at {i}");
            assert_eq!(back.goods[3][i], (i % 255) as u8, "goods lost at {i}");
            assert_eq!(back.locked_bits[i], (i % 7) as u16, "locked lost at {i}");
        }
    }

    /// Phase masks must cover every column their phase actually touches —
    /// regression guard for the mask table (an unloaded-but-touched column
    /// panics on the empty Vec, so running a phase on a tiny world catches it).
    #[test]
    fn phase_masks_run_clean() {
        let conn = test_world(160, 140);
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_PLATES).unwrap();
        crate::sim::plates::generate_plates_and_landmass(&mut buf, 7, 5);
        buf.save(&conn, "plates").unwrap();

        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_ELEVATION).unwrap();
        crate::sim::elevation::generate_elevation(&mut buf, 7);
        crate::sim::elevation::compute_sea_depth(&mut buf);
        crate::sim::elevation::generate_shelves(&mut buf, 7, 12.0, 0.4, 0.3, 8.0);
        buf.save(&conn, "elev").unwrap();

        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap();
        crate::sim::ocean::compute_wind_belts(&mut buf);
        crate::sim::ocean::compute_salinity(&mut buf);
        crate::sim::ocean::generate_ocean_currents(&mut buf);
        crate::sim::ocean::advect_salinity_and_recouple(&mut buf);
        crate::sim::ocean::compute_distance_to_ocean(&mut buf);
        let sea_freeze = crate::sim::ocean::compute_shelf_freeze(&buf);
        crate::sim::ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
        crate::sim::temperature::compute_temperature(&mut buf);
        crate::sim::ocean::compute_upwelling_zones(&mut buf);
        crate::sim::ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
        crate::sim::jets::compute_low_level_jets(&mut buf);
        crate::sim::precipitation::compute_precipitation(&mut buf);
        buf.save(&conn, "ocean").unwrap();

        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_CLIMATE).unwrap();
        crate::sim::koppen::classify_koppen(&mut buf);
        buf.save(&conn, "climate").unwrap();

        let buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_RIVERS).unwrap();
        let hydro = crate::sim::rivers::compute_hydrology(&buf);
        let lakes = crate::sim::rivers::detect_lakes(&buf, &hydro.filled, 0.004, 20);
        let rivers = crate::sim::rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0, &lakes);

        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_SOIL_FERTILITY).unwrap();
        crate::sim::soil::classify_soil(&mut buf);
        crate::sim::soil::apply_volcanic_apron(&mut buf);
        crate::sim::soil::apply_alluvial_override(&mut buf, &rivers);
        crate::sim::fertility::compute_fertility(&mut buf, &rivers);
        crate::sim::fertility::compute_fisheries(&mut buf, &rivers);
        buf.save(&conn, "soil").unwrap();

        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_SETTLEMENTS).unwrap();
        crate::sim::biological::compute_disease_risk(&mut buf, &rivers);
        let hab = crate::sim::settlements::compute_habitability(&buf, &rivers, &lakes);
        let _ = crate::sim::settlements::generate_settlements(&buf, &hab, &rivers, 7, 0.55, None);
        crate::sim::settlements::write_habitability(&mut buf, &hab);
        buf.save(&conn, "settlements").unwrap();

        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_BIOLOGICAL).unwrap();
        let goods = crate::sim::goods_spec::default_list();
        crate::sim::biological::compute_shark_risk(&mut buf, &rivers);
        crate::sim::biological::compute_shipworm_risk(&mut buf, &rivers);
        crate::sim::biological::compute_storm_base(&mut buf);
        crate::sim::biological::compute_reef_risk(&mut buf);
        crate::sim::biological::compute_trade_goods(&mut buf, &rivers, 7, 2, 0.5, &goods);
        buf.save(&conn, "bio").unwrap();
    }
}
