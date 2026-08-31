use serde::Serialize;

use crate::render::tile_image::{
    biome_color, elevation_style_palettes, koppen_color, soil_color, BATHYMETRY_STOPS,
    ELEVATION_STOPS, ELEV_MAX_M, PRECIP_BANDS, TEMPERATURE_STOPS,
};
use crate::sim::deposits::grade_label;

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

/// An ALTERNATE reading of the same D10 scale: a heat ramp (dark blue = poorest
/// land, red = finest) that ignores the good's own hue entirely. Added because the
/// hue-mix scale above is genuinely subtle for a muted or dark good — mixing toward
/// a colour that is itself close to the pale ground tint reads as almost no change
/// at all. A heat ramp has no such blind spot: every good's best land is red and
/// worst is blue, however that good happens to be coloured on the map. Same
/// absolute 0..1 belt value as `GOOD_QUALITY_STOPS`, so switching modes never
/// changes what a cell's number MEANS, only how it is painted.
pub(crate) const GOOD_QUALITY_HEATMAP_STOPS: [(f32, (u8, u8, u8)); 5] = [
    (0.00, (32, 42, 122)),
    (0.25, (44, 130, 216)),
    (0.50, (240, 214, 64)),
    (0.75, (232, 122, 40)),
    (1.00, (196, 32, 32)),
];

/// Linear interpolation over `GOOD_QUALITY_HEATMAP_STOPS` at an arbitrary belt
/// value — used both to build the served ramp and to colour the served grade
/// swatches at their exact breakpoints, so the two can never disagree.
fn heatmap_at(t: f32) -> (u8, u8, u8) {
    let s = &GOOD_QUALITY_HEATMAP_STOPS;
    let t = t.clamp(0.0, 1.0);
    for w in s.windows(2) {
        let (a, b) = (w[0], w[1]);
        if t <= b.0 {
            let k = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 0.0 };
            let mix = |lo: u8, hi: u8| (lo as f32 + (hi as f32 - lo as f32) * k).round() as u8;
            return (mix(a.1 .0, b.1 .0), mix(a.1 .1, b.1 .1), mix(a.1 .2, b.1 .2));
        }
    }
    s[s.len() - 1].1
}

/// One grade breakpoint, labelled in the SAME vocabulary ore workings already use
/// (`deposits::grade_label` — coarse/ordinary/good/fine/exquisite), so "how good is
/// this land for wine" and "how rich is this silver working" read as one shared
/// quality language rather than two invented scales.
#[derive(Serialize)]
pub struct GradeStop {
    pub at: f32,
    pub label: String,
    pub color: String,
}

/// One elevation STYLE's served palette (§ "Elevation styles" in
/// `render/tile_image.rs`) — a named alternative land+sea ramp for the
/// "elevation"/"terrain" layer keys (`"elevation#style=alpine"`), each a real
/// cartographic convention (classed layer tinting, Imhof neutral relief,
/// desert/polar presets, a monochrome analytical hillshade, a sepia antique
/// plate, a bathymetry-showcase "Abyssal"). Served exactly like every other
/// ramp here so a style's own legend can never disagree with what the map
/// actually paints.
#[derive(Serialize)]
pub struct StylePaletteOut {
    /// The `#style=` key, e.g. "alpine".
    pub key: String,
    /// Human-readable name for a style picker, e.g. "Alpine".
    pub label: String,
    /// Land ramp, x in METRES (same units as `elevation`).
    pub land: Vec<RampStop>,
    /// Sea ramp, x = normalised depth 0..1 (same units as `bathymetry`).
    pub sea: Vec<RampStop>,
    /// If true, `land`/`sea` are STEPPED classed bands (no interpolation
    /// between stops) rather than a smooth ramp — read them as discrete tiles,
    /// not a gradient.
    pub classed: bool,
}

/// The breakpoints `grade_label` itself switches on (kept in sync by
/// `grade_labels_match_the_served_breakpoints` below).
const GRADE_BREAKS: [f32; 5] = [0.0, 0.34, 0.52, 0.70, 0.86];

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
    /// The alternate heat-ramp reading of the SAME scale (dark blue → red).
    pub good_quality_heatmap: Vec<RampStop>,
    /// The heatmap ramp's colour at each `grade_label` breakpoint, so the legend's
    /// swatches are guaranteed to match what the map actually paints there.
    pub good_quality_grades: Vec<GradeStop>,
    /// Every named elevation style's own land+sea palette (§ elevation styles).
    pub elevation_styles: Vec<StylePaletteOut>,
    /// The Plates layer's boundary key: how two plates MEET (convergent /
    /// divergent / transform), served from the renderer's own table so the
    /// legend cannot drift from the map.
    pub plate_boundaries: Vec<LabelledColor>,
}

/// A named class colour (a legend row that carries its own words).
#[derive(serde::Serialize)]
pub struct LabelledColor {
    pub label: String,
    pub color: String,
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
        good_quality_heatmap: GOOD_QUALITY_HEATMAP_STOPS.iter()
            .map(|&(at, c)| RampStop { at, color: hex(c) })
            .collect(),
        good_quality_grades: GRADE_BREAKS.iter()
            .map(|&at| GradeStop {
                at,
                label: grade_label(at).to_string(),
                color: hex(heatmap_at(at)),
            })
            .collect(),
        plate_boundaries: crate::render::tile_image::PLATE_BOUNDARY_COLORS
            .iter()
            .map(|&(label, c)| LabelledColor { label: label.to_string(), color: hex(c) })
            .collect(),
        elevation_styles: elevation_style_palettes()
            .into_iter()
            .map(|sp| StylePaletteOut {
                key: sp.key.to_string(),
                label: sp.label.to_string(),
                land: stops(sp.land),
                sea: stops(sp.sea),
                classed: sp.classed,
            })
            .collect(),
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

    /// `GRADE_BREAKS` must land each stop in its OWN `grade_label` word — otherwise
    /// two adjacent legend swatches would carry the same label, which is the exact
    /// misreport a served-not-copied table exists to prevent.
    #[test]
    fn grade_labels_match_the_served_breakpoints() {
        let labels: Vec<&str> = GRADE_BREAKS.iter().map(|&t| grade_label(t)).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), labels.len(), "breakpoints must map to distinct words: {labels:?}");
        assert_eq!(labels, vec!["coarse", "ordinary", "good", "fine", "exquisite"]);
    }

    /// The heat ramp must actually run from a distinct dark tone to a distinct
    /// bright/warm tone — a heatmap that doesn't visibly change defeats the whole
    /// point of adding it (the user-reported problem: the hue-mix scale was too
    /// subtle to read).
    #[test]
    fn the_heatmap_ramp_spans_a_visible_range() {
        let s = &GOOD_QUALITY_HEATMAP_STOPS;
        assert_eq!(s[0].0, 0.0);
        assert_eq!(s[s.len() - 1].0, 1.0);
        for w in s.windows(2) {
            assert!(w[1].0 > w[0].0, "positions must ascend");
        }
        let lo = s[0].1;
        let hi = s[s.len() - 1].1;
        let dist2 = (lo.0 as i32 - hi.0 as i32).pow(2)
            + (lo.1 as i32 - hi.1 as i32).pow(2)
            + (lo.2 as i32 - hi.2 as i32).pow(2);
        assert!(dist2 > 100 * 100, "the low and high ends of the heatmap look too similar: {lo:?} vs {hi:?}");
        // Interpolation must actually be monotone-ish in perceived warmth: sampling
        // the midpoint should not silently equal an endpoint.
        let mid = heatmap_at(0.5);
        assert_ne!(mid, lo);
        assert_ne!(mid, hi);
    }
}
