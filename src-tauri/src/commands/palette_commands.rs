use serde::Serialize;

use crate::render::tile_image::{
    biome_color, koppen_color, soil_color, BATHYMETRY_STOPS, ELEVATION_STOPS, ELEV_MAX_M,
    PRECIP_BANDS, TEMPERATURE_STOPS,
};

/// THE PALETTE, STRAIGHT OUT OF THE RENDERER.
///
/// The map legend used to keep its own hand-maintained copies of these tables —
/// four of them, in three files, none checked against the Rust that actually paints
/// the pixels. They drifted, exactly as §8.12 warns: the elevation legend's sea key
/// was copied from the wrong renderer and ran BACKWARDS, and the histogram
/// disagreed with the map by ΔE 19–24 in the high bands.
///
/// A test comparing two copies would only have caught drift after someone wrote it.
/// Serving the renderer's own constants over IPC removes the second copy entirely,
/// so the legend cannot be wrong about the map without the map being wrong about
/// itself. That is the difference between guarding a rule and making it unbreakable.
#[derive(Serialize)]
pub struct RampStop {
    /// Position in the ramp's own units — metres, °C, normalised depth, or the
    /// upper bound in mm for the classed precipitation bands.
    pub at: f32,
    pub color: String,
}

#[derive(Serialize)]
pub struct ClassColor {
    pub code: u8,
    pub color: String,
}

#[derive(Serialize)]
pub struct RenderPalettes {
    /// Continuous, x in METRES above sea level.
    pub elevation: Vec<RampStop>,
    /// Continuous, x = normalised depth (0 = shore, 1 = abyss).
    pub bathymetry: Vec<RampStop>,
    /// Continuous, x in DEGREES CELSIUS. Shared by the temperature and sst layers.
    pub temperature: Vec<RampStop>,
    /// CLASSED, x = the band's upper bound in mm/yr.
    pub precipitation: Vec<RampStop>,
    pub koppen: Vec<ClassColor>,
    pub biome: Vec<ClassColor>,
    pub soil: Vec<ClassColor>,
    /// 1.0 of the normalised elevation column, in metres.
    pub elev_max_m: f32,
}

fn hex(c: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

fn stops(src: &[(f32, (u8, u8, u8))]) -> Vec<RampStop> {
    src.iter().map(|&(at, c)| RampStop { at, color: hex(c) }).collect()
}

fn classes(range: std::ops::RangeInclusive<u8>, f: fn(u8) -> (u8, u8, u8)) -> Vec<ClassColor> {
    range.map(|code| ClassColor { code, color: hex(f(code)) }).collect()
}

/// Read-only: builds no world state and touches no tile.
#[tauri::command]
pub fn get_render_palettes() -> RenderPalettes {
    RenderPalettes {
        elevation: stops(&ELEVATION_STOPS),
        bathymetry: stops(&BATHYMETRY_STOPS),
        temperature: stops(&TEMPERATURE_STOPS),
        precipitation: stops(&PRECIP_BANDS),
        koppen: classes(1..=31, koppen_color),
        biome: classes(1..=41, biome_color),
        soil: classes(1..=12, soil_color),
        elev_max_m: ELEV_MAX_M,
    }
}
