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
}

/// Number of trade-good sublayer fields stored per cell. See sim/biological.rs
/// GOOD_NAMES for the ordered list. New goods are appended LAST so older saves
/// still decompress (the trailing reads pad missing goods to 0). 17 -> 21 added
/// wheat(17), iron(18), cotton(19), gemstones(20).
pub const GOODS_COUNT: usize = 21;

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
        }
    }

    /// Serialize to a compact binary format, then zstd compress
    pub fn compress(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(N * 60);

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
        // shipworm_risk serialized AFTER goods so it is the very last column —
        // older saves (which end after their goods) simply pad it to zero, and a
        // save with fewer goods pads the extra goods to zero before reaching here.
        buf.extend_from_slice(&self.shipworm_risk);

        zstd::encode_all(buf.as_slice(), 3).unwrap_or(buf)
    }

    /// Decompress and deserialize from binary format
    pub fn decompress(data: &[u8]) -> Self {
        let buf = zstd::decode_all(data).unwrap_or_else(|_| data.to_vec());
        let mut offset = 0;

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
        let mut goods = Vec::with_capacity(GOODS_COUNT);
        for _ in 0..GOODS_COUNT {
            goods.push(read_u8(&buf, &mut offset));
        }
        let shipworm_risk = read_u8(&buf, &mut offset);

        Self {
            terrain, elevation, sea_depth, is_shelf, is_shelf_edge,
            locked_bits, plate_index, boundary_type, is_volcanic,
            temperature, precipitation, koppen, soil_type,
            fertility, fishery, current_type,
            wind_vx, wind_vy, current_vx, current_vy,
            distance_to_ocean, habitability,
            salinity, shark_risk, goods, shipworm_risk,
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
