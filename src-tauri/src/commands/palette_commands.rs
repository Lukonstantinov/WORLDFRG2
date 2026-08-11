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

/// GOODS_LOCALITIES_PLAN.md D10 — ONE ABSOLUTE QUALITY SCALE, SHARED BY EVERY GOOD.
///
/// A goods overlay carries two facts at once: WHICH good (the hue, from the good's
/// own spec colour) and HOW GOOD the land is for it (this scale). D10 settles the
/// second: it is the belt's own absolute 0..1 value, never renormalised per good —
/// so a thin wine fringe and a thin wheat fringe read exactly alike, and a good
/// whose whole belt is mediocre never gets promoted to full colour just because it
/// is that good's own best.
///
/// The stop carries `alpha` and `mix` rather than a colour, because the colour is
/// the good's. `mix` = 0 is the pale ground tint (`GOOD_QUALITY_PALE`), 1 is the
/// good's own full colour. It lives here, served, for the same reason every other
/// table does (§8.18): one copy, so the map and the key cannot disagree.
#[derive(Serialize)]
pub struct QualityStop {
    /// The belt value itself, 0..1. Absolute — never per-good normalised.
    pub at: f32,
    /// Opacity of the wash at this value.
    pub alpha: f32,
    /// How far the good's own hue is mixed in (0 = the pale ground tint, 1 = full).
    pub mix: f32,
}

/// The stop table. Deliberately starts at a visible-but-faint fringe rather than at
/// zero: a cell in the belt at all is a cell that produces, and Slice 3's `FLOOR`
/// exists precisely so it never stops producing — an overlay that faded the fringe
/// to nothing would hide the thing §5.1 asks the coverage layer to make visible.
pub(crate) const GOOD_QUALITY_STOPS: [(f32, f32, f32); 5] = [
    (0.00, 0.13, 0.00),
    (0.25, 0.20, 0.25),
    (0.50, 0.28, 0.55),
    (0.75, 0.37, 0.80),
    (1.00, 0.46, 1.00),
];

/// The pale end the ramp mixes AWAY from — the same near-white the overlay's old
/// `rampToward` faded to, kept so a low-quality belt still reads as paper-and-wash.
pub(crate) const GOOD_QUALITY_PALE: (u8, u8, u8) = (236, 230, 224);

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
    /// D10 · the one absolute belt-quality scale every good's quality layer shades on.
    pub good_quality: Vec<QualityStop>,
    /// The pale end that scale mixes away from, as a hex colour.
    pub good_quality_pale: String,
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
        good_quality: GOOD_QUALITY_STOPS.iter()
            .map(|&(at, alpha, mix)| QualityStop { at, alpha, mix })
            .collect(),
        good_quality_pale: hex(GOOD_QUALITY_PALE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D10's scale must be monotone and cover the whole 0..1 belt range, or a
    /// higher-quality cell could draw fainter than a lower one — which is exactly
    /// the misreport §8.18 exists to prevent, in a new place.
    #[test]
    fn the_good_quality_scale_is_monotone_and_spans_zero_to_one() {
        let s = &GOOD_QUALITY_STOPS;
        assert_eq!(s[0].0, 0.0, "the scale must start at belt value 0");
        assert_eq!(s[s.len() - 1].0, 1.0, "the scale must end at belt value 1");
        for w in s.windows(2) {
            assert!(w[1].0 > w[0].0, "positions must ascend: {:?} then {:?}", w[0], w[1]);
            assert!(w[1].1 >= w[0].1, "alpha must never fall as quality rises");
            assert!(w[1].2 >= w[0].2, "hue mix must never fall as quality rises");
        }
        // The faintest wash still has to be visible — Slice 3's FLOOR keeps a fringe
        // cell producing, so the map must keep it drawn (§5.1).
        assert!(s[0].1 >= 0.10, "the fringe must not fade to invisible");
    }
}
