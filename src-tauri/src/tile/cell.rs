use crate::tile::coords::TILE_SIZE;

const N: usize = (TILE_SIZE * TILE_SIZE) as usize;

/// Columnar storage for all cells in a single tile.
/// Each field is a contiguous array of TILE_SIZE*TILE_SIZE elements.
/// Index = ly * TILE_SIZE + lx (row-major within the tile).
#[derive(Clone)]
pub struct TileData {
    pub terrain: Vec<u8>,        // 0=sea, 1=land
    pub elevation: Vec<f32>,     // 0.0-1.0 normalized
    pub sea_depth: Vec<f32>,
    pub is_shelf: Vec<u8>,
    pub is_shelf_edge: Vec<u8>,
    pub locked_bits: Vec<u16>,
    pub plate_index: Vec<u16>,
    pub boundary_type: Vec<u8>,
    pub is_volcanic: Vec<u8>,
    pub temperature: Vec<f32>,
    pub precipitation: Vec<f32>,
    pub koppen: Vec<u8>,         // 0=none, 1-22 = zone codes
    pub soil_type: Vec<u8>,
    pub fertility: Vec<f32>,
    pub fishery: Vec<f32>,
    pub current_type: Vec<u8>,   // 0=none, 1=warm, 2=cold
    pub wind_vx: Vec<f32>,
    pub wind_vy: Vec<f32>,
    pub current_vx: Vec<f32>,
    pub current_vy: Vec<f32>,
    pub distance_to_ocean: Vec<f32>,
    pub habitability: Vec<f32>,   // 0.0-1.0 settlement suitability (heatmap layer)
    // ── Salinity + Biological (appended last for save back-compat) ──
    pub salinity: Vec<u8>,        // sea: 0..255 ↔ ~28-42 PSU
    pub shark_risk: Vec<u8>,      // sea: 0..255 shark-habitat danger
    pub goods: Vec<Vec<u8>>,      // [GOODS_COUNT] trade-good intensity fields (0..255)
    pub shipworm_risk: Vec<u8>,   // sea: 0..255 shipworm (Teredo) hull-hazard. Serialized AFTER goods.
    pub storm_base: Vec<u8>,      // sea: 0..255 annual cyclone/storm potential (open ocean).
    pub reef_risk: Vec<u8>,       // sea: 0..255 reef/shoal wreck hazard (warm shallow coast).
}

/// Number of trade-good sublayer fields stored per cell. See sim/biological.rs
/// GOOD_NAMES for the ordered list. 17 -> 21 added wheat/iron/cotton/gemstones;
/// 21 -> 30 added hardwoods, horses, wool_fleece, wool_llama, ivory, cacao,
/// copper, tin, gold.
///
/// Since this count can change between releases, tile blobs are now
/// **self-describing**: a v2 blob carries `goods_count` in its header (see
/// `compress`/`decompress`), so a save always reads back exactly the goods it
/// stored regardless of the compile-time `GOODS_COUNT`.
pub const GOODS_COUNT: usize = 30;

/// First byte of a v2 (self-describing) tile blob. Chosen so it can never collide
/// with a legacy v1 blob, whose first byte is `terrain[0]` ∈ {0, 1}.
const TILE_MAGIC: u8 = 0xF2;
/// Current tile binary format version.
const TILE_VERSION: u8 = 2;
/// Number of goods a legacy (headerless) v1 blob contained.
const V1_GOODS_COUNT: usize = 21;

impl TileData {
    pub fn new_sea() -> Self {
        Self {
            terrain: vec![0; N],
            elevation: vec![0.0; N],
            sea_depth: vec![0.0; N],
            is_shelf: vec![0; N],
            is_shelf_edge: vec![0; N],
            locked_bits: vec![0; N],
            plate_index: vec![0; N],
            boundary_type: vec![0; N],
            is_volcanic: vec![0; N],
            temperature: vec![0.0; N],
            precipitation: vec![0.0; N],
            koppen: vec![0; N],
            soil_type: vec![0; N],
            fertility: vec![0.0; N],
            fishery: vec![0.0; N],
            current_type: vec![0; N],
            wind_vx: vec![0.0; N],
            wind_vy: vec![0.0; N],
            current_vx: vec![0.0; N],
            current_vy: vec![0.0; N],
            distance_to_ocean: vec![1.0; N],
            habitability: vec![0.0; N],
            salinity: vec![0; N],
            shark_risk: vec![0; N],
            goods: vec![vec![0u8; N]; GOODS_COUNT],
            shipworm_risk: vec![0; N],
            storm_base: vec![0; N],
            reef_risk: vec![0; N],
        }
    }

    /// Serialize to a compact binary format, then zstd compress.
    ///
    /// v2 layout (self-describing): `[MAGIC][VERSION][goods_count u16 LE]` then
    /// the columns. `goods_count` lets a reader recover exactly the goods that
    /// were written even if `GOODS_COUNT` changes in a later release.
    pub fn compress(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(N * 64);

        // Self-describing header.
        buf.push(TILE_MAGIC);
        buf.push(TILE_VERSION);
        let gc = self.goods.len() as u16;
        buf.extend_from_slice(&gc.to_le_bytes());

        // Write each column as raw bytes
        buf.extend_from_slice(&self.terrain);
        buf.extend_from_slice(bytemuck_f32(&self.elevation));
        buf.extend_from_slice(bytemuck_f32(&self.sea_depth));
        buf.extend_from_slice(&self.is_shelf);
        buf.extend_from_slice(&self.is_shelf_edge);
        buf.extend_from_slice(bytemuck_u16(&self.locked_bits));
        buf.extend_from_slice(bytemuck_u16(&self.plate_index));
        buf.extend_from_slice(&self.boundary_type);
        buf.extend_from_slice(&self.is_volcanic);
        buf.extend_from_slice(bytemuck_f32(&self.temperature));
        buf.extend_from_slice(bytemuck_f32(&self.precipitation));
        buf.extend_from_slice(&self.koppen);
        buf.extend_from_slice(&self.soil_type);
        buf.extend_from_slice(bytemuck_f32(&self.fertility));
        buf.extend_from_slice(bytemuck_f32(&self.fishery));
        buf.extend_from_slice(&self.current_type);
        buf.extend_from_slice(bytemuck_f32(&self.wind_vx));
        buf.extend_from_slice(bytemuck_f32(&self.wind_vy));
        buf.extend_from_slice(bytemuck_f32(&self.current_vx));
        buf.extend_from_slice(bytemuck_f32(&self.current_vy));
        buf.extend_from_slice(bytemuck_f32(&self.distance_to_ocean));
        // Appended last so older save files (which lack this column) still
        // decompress — the trailing read simply pads with zeros.
        buf.extend_from_slice(bytemuck_f32(&self.habitability));
        // Salinity + biological columns, also appended last (same back-compat
        // reasoning: older saves end here and the trailing reads pad with zeros).
        buf.extend_from_slice(&self.salinity);
        buf.extend_from_slice(&self.shark_risk);
        for g in &self.goods {
            buf.extend_from_slice(g);
        }
        // Hazard columns after goods, in a fixed versioned order. New hazards are
        // appended here (after the previous last column) so older v2 blobs simply
        // pad them to zero on read.
        buf.extend_from_slice(&self.shipworm_risk);
        buf.extend_from_slice(&self.storm_base);
        buf.extend_from_slice(&self.reef_risk);

        zstd::encode_all(buf.as_slice(), 3).unwrap_or(buf)
    }

    /// Decompress and deserialize from binary format.
    ///
    /// Detects v2 (self-describing header) vs legacy v1 (headerless, 21 goods,
    /// `shipworm_risk` as the last column, no storm/reef). Both load into the
    /// current `GOODS_COUNT` layout; goods beyond what was stored pad to zero.
    pub fn decompress(data: &[u8]) -> Self {
        let buf = zstd::decode_all(data).unwrap_or_else(|_| data.to_vec());
        let mut offset = 0;

        // Header detection. A v2 blob starts with MAGIC; a v1 blob starts with
        // terrain[0] ∈ {0,1}, which can never equal MAGIC.
        let stored_goods: usize = if buf.first() == Some(&TILE_MAGIC) {
            let gc = u16::from_le_bytes([
                *buf.get(2).unwrap_or(&0),
                *buf.get(3).unwrap_or(&0),
            ]) as usize;
            offset = 4;
            gc.min(256) // sanity clamp
        } else {
            V1_GOODS_COUNT
        };

        let terrain = read_u8(&buf, &mut offset);
        let elevation = read_f32(&buf, &mut offset);
        let sea_depth = read_f32(&buf, &mut offset);
        let is_shelf = read_u8(&buf, &mut offset);
        let is_shelf_edge = read_u8(&buf, &mut offset);
        let locked_bits = read_u16(&buf, &mut offset);
        let plate_index = read_u16(&buf, &mut offset);
        let boundary_type = read_u8(&buf, &mut offset);
        let is_volcanic = read_u8(&buf, &mut offset);
        let temperature = read_f32(&buf, &mut offset);
        let precipitation = read_f32(&buf, &mut offset);
        let koppen = read_u8(&buf, &mut offset);
        let soil_type = read_u8(&buf, &mut offset);
        let fertility = read_f32(&buf, &mut offset);
        let fishery = read_f32(&buf, &mut offset);
        let current_type = read_u8(&buf, &mut offset);
        let wind_vx = read_f32(&buf, &mut offset);
        let wind_vy = read_f32(&buf, &mut offset);
        let current_vx = read_f32(&buf, &mut offset);
        let current_vy = read_f32(&buf, &mut offset);
        let distance_to_ocean = read_f32(&buf, &mut offset);
        let habitability = read_f32(&buf, &mut offset);
        let salinity = read_u8(&buf, &mut offset);
        let shark_risk = read_u8(&buf, &mut offset);
        // Read exactly the goods that were stored, then normalize to GOODS_COUNT
        // (truncate extras / pad missing with zeros) so the in-memory layout is
        // stable regardless of how many goods the save held.
        let mut goods: Vec<Vec<u8>> = Vec::with_capacity(GOODS_COUNT.max(stored_goods));
        for _ in 0..stored_goods {
            goods.push(read_u8(&buf, &mut offset));
        }
        let shipworm_risk = read_u8(&buf, &mut offset);
        // storm_base / reef_risk only exist in v2 blobs that were written after
        // they were added; older blobs end above and these reads pad to zero.
        let storm_base = read_u8(&buf, &mut offset);
        let reef_risk = read_u8(&buf, &mut offset);

        // Keep every stored good column (the count is variable and may exceed the
        // built-in GOODS_COUNT); pad up to GOODS_COUNT so code that indexes the
        // built-in slots on an old/short tile stays in range.
        goods.resize(stored_goods.max(GOODS_COUNT), vec![0u8; N]);

        Self {
            terrain, elevation, sea_depth, is_shelf, is_shelf_edge,
            locked_bits, plate_index, boundary_type, is_volcanic,
            temperature, precipitation, koppen, soil_type,
            fertility, fishery, current_type,
            wind_vx, wind_vy, current_vx, current_vy,
            distance_to_ocean, habitability,
            salinity, shark_risk, goods, shipworm_risk,
            storm_base, reef_risk,
        }
    }
}

// Helper functions for binary serialization

fn bytemuck_f32(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

fn bytemuck_u16(data: &[u16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) }
}

fn read_u8(buf: &[u8], offset: &mut usize) -> Vec<u8> {
    let end = (*offset + N).min(buf.len());
    let slice = buf[*offset..end].to_vec();
    *offset = end;
    let mut v = slice;
    v.resize(N, 0);
    v
}

fn read_f32(buf: &[u8], offset: &mut usize) -> Vec<f32> {
    let byte_len = N * 4;
    let end = (*offset + byte_len).min(buf.len());
    let bytes = &buf[*offset..end];
    *offset = end;
    let mut v = vec![0.0f32; N];
    let dst = unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, byte_len) };
    dst[..bytes.len()].copy_from_slice(bytes);
    v
}

fn read_u16(buf: &[u8], offset: &mut usize) -> Vec<u16> {
    let byte_len = N * 2;
    let end = (*offset + byte_len).min(buf.len());
    let bytes = &buf[*offset..end];
    *offset = end;
    let mut v = vec![0u16; N];
    let dst = unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, byte_len) };
    dst[..bytes.len()].copy_from_slice(bytes);
    v
}
