use super::world_buffer::WorldBuffer;
use super::koppen::*;

/// Soil type codes (1-based, 0=none)
pub const SOIL_OXISOL: u8 = 1;    // Tropical weathered
pub const SOIL_ULTISOL: u8 = 2;   // Subtropical leached
pub const SOIL_MOLLISOL: u8 = 3;  // Prairie/grassland
pub const SOIL_ALFISOL: u8 = 4;   // Temperate forest
pub const SOIL_SPODOSOL: u8 = 5;  // Boreal/podzol
pub const SOIL_ARIDISOL: u8 = 6;  // Desert/dry
pub const SOIL_HISTOSOL: u8 = 7;  // Organic/peat
pub const SOIL_ENTISOL: u8 = 8;   // Young/undeveloped
pub const SOIL_ANDISOL: u8 = 9;   // Volcanic
pub const SOIL_GELISOL: u8 = 10;  // Permafrost
pub const SOIL_ALLUVIAL: u8 = 11; // River-deposited

/// Classify soil types based on Köppen climate.
/// Matches WF1 soil-types.ts algorithm.
pub fn classify_soil(buf: &mut WorldBuffer) {
    for i in 0..buf.total() {
        if buf.terrain[i] != 1 {
            buf.soil_type[i] = 0;
            continue;
        }

        // Volcanic override
        if buf.is_volcanic[i] == 1 {
            buf.soil_type[i] = SOIL_ANDISOL;
            continue;
        }

        // Köppen-based classification
        buf.soil_type[i] = match buf.koppen[i] {
            AF | AM => SOIL_OXISOL,
            AW => SOIL_ULTISOL,
            BWH | BWK => SOIL_ARIDISOL,
            BSH => SOIL_ALFISOL,
            BSK => SOIL_MOLLISOL,
            CSA | CSB | CSC | CFA | CFB | CFC => SOIL_ALFISOL,
            DFA | DFB => SOIL_MOLLISOL,
            DFC | DFD => SOIL_SPODOSOL,
            DSA | DSB | DSC => SOIL_ALFISOL,
            ET | EF => SOIL_GELISOL,
            _ => SOIL_ENTISOL,
        };
    }
}

/// Override soil to alluvial near major rivers.
/// Rivers data provides proximity information.
pub fn apply_alluvial_override(buf: &mut WorldBuffer, rivers: &[super::rivers::River]) {
    for river in rivers {
        if river.width < 2.5 { continue; }

        // Only lower 60% of river
        let start = (river.points.len() as f32 * 0.4) as usize;
        for &(rx, ry) in &river.points[start..] {
            // Mark cells within 2 of river as alluvial
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    if dx * dx + dy * dy > 4 { continue; }
                    let nx = buf.wrap_x(rx as i32 + dx);
                    let ny = buf.clamp_y(ry as i32 + dy);
                    let ni = buf.idx(nx, ny);
                    if buf.terrain[ni] == 1 {
                        buf.soil_type[ni] = SOIL_ALLUVIAL;
                    }
                }
            }
        }
    }
}
