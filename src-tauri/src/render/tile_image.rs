use crate::tile::cell::TileData;
use crate::tile::coords::TILE_SIZE;

const SIZE: usize = TILE_SIZE as usize;
const PIXEL_COUNT: usize = SIZE * SIZE;

/// Render a tile to RGBA pixel buffer for a given layer
pub fn render_tile(tile: &TileData, layer: &str) -> Vec<u8> {
    render_tile_ctx(tile, layer, &RenderCtx::default())
}

/// As `render_tile`, but with the world geometry the hillshade needs (see
/// `RenderCtx`). Every other layer ignores it.
pub fn render_tile_ctx(tile: &TileData, layer: &str, ctx: &RenderCtx) -> Vec<u8> {
    render_tile_full(tile, layer, &TileNeighbors::default(), ctx)
}

/// Split a layer key into its base layer and any isolated class code.
/// `"biomes#iso=12"` → `("biomes", Some(12))`; an unparsable code is ignored rather
/// than failing the tile, since a bad key must never blank the map.
pub fn split_isolate(layer: &str) -> (&str, Option<u8>) {
    match layer.split_once("#iso=") {
        Some((base, code)) => (base, code.parse::<u8>().ok()),
        None => (layer, None),
    }
}

/// The single dispatch. `n` is used by every layer that shades (the terrain
/// hillshade and the four thematic plates); the rest ignore it.
fn render_tile_full(tile: &TileData, layer: &str, n: &TileNeighbors, ctx: &RenderCtx) -> Vec<u8> {
    // The isolated class rides in the LAYER KEY ("biomes#iso=12") rather than in a
    // separate request field. The frontend's tile cache is keyed by layer string, so
    // an isolated view caches and invalidates correctly with no cache-key change —
    // and a client that knows nothing about isolation still asks for plain "biomes".
    let (layer, ctx) = match split_isolate(layer) {
        (base, Some(code)) => (base, RenderCtx { isolate: Some(code), ..*ctx }),
        (base, None) => (base, *ctx),
    };
    let ctx = &ctx;
    let mut rgba = vec![0u8; PIXEL_COUNT * 4];

    match layer {
        "land" => render_land(tile, &mut rgba),
        "elevation" => render_elevation(tile, &mut rgba),
        "climate" => render_climate(tile, n, ctx, &mut rgba),
        "temperature" => render_temperature(tile, &mut rgba),
        "sst" => render_sst(tile, &mut rgba),
        "snow" => render_snow(tile, &mut rgba),
        "precipitation" => render_precipitation(tile, &mut rgba),
        "soil" => render_soil(tile, n, ctx, &mut rgba),
        "fertility" => render_fertility(tile, n, ctx, &mut rgba),
        "plates" => render_plates(tile, &mut rgba),
        "biomes" => render_biomes(tile, n, ctx, &mut rgba),
        "fisheries" => render_fisheries(tile, &mut rgba),
        "terrain" => render_terrain_hillshade_halo(tile, n, ctx, &mut rgba),
        "shelf" => render_shelf(tile, &mut rgba),
        "ridges" => render_ridges(tile, &mut rgba),
        "wind" => render_wind(tile, &mut rgba),
        "windspeed" => render_windspeed(tile, &mut rgba),
        "currents" => render_currents(tile, &mut rgba),
        "habitability" => render_habitability(tile, &mut rgba),
        "salinity" => render_salinity(tile, &mut rgba),
        "shark" => render_shark(tile, &mut rgba),
        "shipworm" => render_shipworm(tile, &mut rgba),
        "storm" => render_storm(tile, &mut rgba),
        "reef" => render_reef(tile, &mut rgba),
        "disease" => render_disease(tile, &mut rgba),
        _ => render_land(tile, &mut rgba),
    }

    rgba
}

fn render_land(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            let e = tile.elevation[i].clamp(0.0, 1.0);
            let (r, g, b) = match tile.koppen[i] {
                // Ice cap (EF): permanent ice sheet, near-white with faint relief.
                22 => {
                    let s = (220.0 + e * 35.0).min(255.0) as u8;
                    (s, s, (235.0 + e * 20.0).min(255.0) as u8)
                }
                // Tundra (ET): frosted, pale grey-green.
                21 => (150, 168, 158),
                // Land: earthy green with subtle elevation shading.
                _ => ((60.0 + e * 40.0) as u8, (100.0 + e * 30.0) as u8, (45.0 + e * 20.0) as u8),
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = 255;
        } else {
            // Sea: the shared bathymetry ramp (see BATHYMETRY_STOPS).
            let (r, g, b) = bathymetry_color(tile.sea_depth[i]);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = 255;
        }
    }
}

fn render_elevation(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            let e = tile.elevation[i].clamp(0.0, 1.0);
            let (r, g, b) = elevation_color_climate(e, tile.temperature[i], tile.precipitation[i]);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = 255;
        } else {
            // Sea: the SHARED bathymetry ramp. This layer used to invert it —
            // dark shelf brightening to abyss — which is what made its own legend
            // wrong. See BATHYMETRY_STOPS.
            let (r, g, b) = bathymetry_color(tile.sea_depth[i]);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = 255;
        }
    }
}

/// Highest elevation the normalised `elevation` column maps to (1.0 = Everest).
pub const ELEV_MAX_M: f32 = 8848.0;

/// Piecewise-linear lookup over `(x, colour)` stops, clamped at both ends. Stops
/// must be sorted ascending on x. Every continuous ramp in this file goes through
/// here, so a ramp is DATA (a stop table) rather than a chain of hand-written
/// branches — which is what lets `palette_commands.rs` hand the exact same tables
/// to the frontend legend instead of the legend keeping its own copy.
pub fn ramp_lookup(x: f32, stops: &[(f32, (u8, u8, u8))]) -> (u8, u8, u8) {
    if stops.is_empty() {
        return (128, 128, 128);
    }
    if x <= stops[0].0 {
        return stops[0].1;
    }
    let last = stops[stops.len() - 1];
    if x >= last.0 {
        return last.1;
    }
    for w in stops.windows(2) {
        let (x0, c0) = w[0];
        let (x1, c1) = w[1];
        if x <= x1 {
            let span = x1 - x0;
            let t = if span.abs() < f32::EPSILON { 0.0 } else { (x - x0) / span };
            return lerp_rgb(c0, c1, t);
        }
    }
    last.1
}

/// HYPSOMETRIC TINT on the conventional atlas breaks (0 · 200 · 500 · 1000 · 2000 ·
/// 3000 · 5000 m), in METRES rather than in fractions of Everest.
///
/// Two properties matter here and both were wrong before:
///
/// 1. **The breaks are where land actually is.** The old anchors were linear in
///    normalised elevation — 0.15/0.35/0.60/0.85, i.e. 1327/3097/5309/7521 m — so
///    roughly the top two-thirds of the ramp served the few percent of land above
///    3000 m while almost every inhabited lowland fell inside a single band.
/// 2. **It is MONOTONE IN LIGHTNESS.** The old ramp ran L* 44 → 65 → back down to
///    45 through its dark brown midband, so a shaded 5000 m ridge and a sunlit
///    lowland resolved to the same value and height fought the hillshade for the
///    same channel. Imhof's rule is the one applied here: higher is brighter.
pub const ELEVATION_STOPS: [(f32, (u8, u8, u8)); 8] = [
    (0.0, (76, 116, 80)),
    (200.0, (124, 154, 94)),
    (500.0, (172, 182, 112)),
    (1000.0, (203, 194, 136)),
    (2000.0, (219, 200, 155)),
    (3000.0, (231, 210, 186)),
    (5000.0, (241, 231, 222)),
    (8848.0, (255, 255, 255)),
];

fn elevation_color(e: f32) -> (u8, u8, u8) {
    ramp_lookup(e.clamp(0.0, 1.0) * ELEV_MAX_M, &ELEVATION_STOPS)
}

/// THE ONE BATHYMETRY RAMP (normalised depth 0 = shore, 1 = abyss).
///
/// `land`, `elevation` and `terrain` each used to carry their own sea colouring,
/// and `elevation`'s ran the OPPOSITE WAY — `(10+d·10, 25+d·30, 70+d·100)`, i.e.
/// dark on the shelf and BRIGHTENING with depth. The legend (correctly) described
/// the shared bright-shelf-to-dark-abyss ramp, so reading a deep-ocean colour off
/// the Elevation layer and looking it up in the key landed you on "Shelf".
///
/// Unifying on this one table fixes the key for all three layers at once, and
/// keeps the physically sensible direction: deeper is darker.
pub const BATHYMETRY_STOPS: [(f32, (u8, u8, u8)); 5] = [
    (0.00, (29, 120, 196)),
    (0.10, (26, 100, 180)),
    (0.25, (20, 74, 140)),
    (0.65, (5, 15, 46)),
    (1.00, (2, 5, 20)),
];

fn bathymetry_color(depth: f32) -> (u8, u8, u8) {
    ramp_lookup(depth.clamp(0.0, 1.0), &BATHYMETRY_STOPS)
}

/// How far the lowland climate tint may pull the base ramp. A BLEND, never a
/// replacement — the hypsometric sequence still has to read as height first.
const LOWLAND_TINT_STRENGTH: f32 = 0.55;

/// Above this height the climate tint is fully gone and every climate shares the
/// same ramp: the eye reads rock and snow up there, not biome, and two ranges in
/// different climates should stay directly comparable.
const LOWLAND_TINT_CEILING_M: f32 = 1200.0;

/// CROSS-BLENDED HYPSOMETRIC TINTS (Patterson & Jenny, *Cartographic Perspectives*
/// 69, 2011).
///
/// A single green-to-brown ramp invites the reader to take LOWLAND GREEN FOR
/// VEGETATION, which is simply wrong wherever the lowland is desert — the Sahara
/// and the Congo basin are both "low", and one flat green says they look alike.
/// Cross-blending makes the low end of the ramp depend on the local climate and
/// converge on the shared yellow/brown/white as it climbs.
///
/// The climate axes here are TEMPERATURE AND PRECIPITATION — both continuous —
/// rather than the categorical Köppen code. That is the whole trick: keying on
/// Köppen would draw every class boundary as a hard colour edge, and a reader would
/// take those edges for terrain, which is exactly the artefact this technique
/// exists to avoid. Köppen is itself a discretisation of these same two fields, so
/// this is the smoother form of the same information.
///
/// WorldForge is unusually well placed for this: the paper's authors had to
/// hand-mask climate regions in Photoshop, and we have the fields per cell already.
fn lowland_tint(temp_c: f32, precip_mm: f32) -> (u8, u8, u8) {
    // sqrt keeps the resolution in the DRY half, where the arid/semi-arid
    // distinction is what actually decides how the land looks.
    let humid = (precip_mm / 1200.0).clamp(0.0, 1.0).sqrt();
    let warm = ((temp_c + 5.0) / 30.0).clamp(0.0, 1.0);

    let mix = |a: (f32, f32, f32), b: (f32, f32, f32), t: f32| {
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
    };

    let cold_dry = (150.0, 150.0, 132.0); // steppe / tundra grey-green
    let cold_wet = (86.0, 122.0, 100.0);  // boreal blue-green
    let hot_dry = (198.0, 178.0, 126.0);  // desert khaki
    let hot_wet = (74.0, 124.0, 62.0);    // tropical yellow-green

    let dry = mix(cold_dry, hot_dry, warm);
    let wet = mix(cold_wet, hot_wet, warm);
    let c = mix(dry, wet, humid);
    (c.0 as u8, c.1 as u8, c.2 as u8)
}

/// The hypsometric colour a LAND cell actually renders: the shared ramp, pulled
/// toward its local lowland tint by a weight that fades out with height.
fn elevation_color_climate(e: f32, temp_c: f32, precip_mm: f32) -> (u8, u8, u8) {
    let base = elevation_color(e);
    let m = e.clamp(0.0, 1.0) * ELEV_MAX_M;
    let w = (1.0 - m / LOWLAND_TINT_CEILING_M).clamp(0.0, 1.0) * LOWLAND_TINT_STRENGTH;
    if w <= 0.0 {
        return base;
    }
    lerp_rgb(base, lowland_tint(temp_c, precip_mm), w)
}

fn render_climate(tile: &TileData, n: &TileNeighbors, ctx: &RenderCtx, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            // Polar SEA ICE: frozen high-latitude ocean reads as a white ice cap
            // at the climate stage (Köppen itself is land-only, so without this the
            // poles looked open-water). Pack ice forms below roughly -1.8°C; we fade
            // a pale ice tint in over a couple of degrees so the cap edge is soft.
            let t = tile.temperature[i];
            if t < 1.0 {
                let ice = ((1.0 - t) / 4.0).clamp(0.0, 1.0); // 0 at +1°C → 1 by -3°C
                let (r, g, b) = lerp_rgb((150, 180, 205), (238, 244, 250), ice);
                rgba[offset] = r;
                rgba[offset + 1] = g;
                rgba[offset + 2] = b;
                rgba[offset + 3] = (180.0 * ice) as u8; // transparent open water → opaque ice
            } else {
                rgba[offset + 3] = 0; // transparent for open sea
            }
            continue;
        }
        let k = tile.koppen[i];
        let (r, g, b) = match ctx.isolate {
            Some(sel) if sel != k => dim_unselected(koppen_color(k)),
            _ => koppen_color(k),
        };
        let m = thematic_relief(tile, n, ctx, i % SIZE, i / SIZE);
        rgba[offset] = (r as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 1] = (g as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 2] = (b as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 3] = 255;
    }
}

pub fn koppen_color(code: u8) -> (u8, u8, u8) {
    match code {
        1 => (0, 0, 255),       // Af - tropical rainforest
        2 => (0, 120, 255),     // Am - tropical monsoon
        3 => (70, 170, 250),    // Aw - tropical savanna
        4 => (255, 0, 0),       // BWh - hot desert
        5 => (255, 150, 150),   // BWk - cold desert
        6 => (245, 165, 0),     // BSh - hot steppe
        7 => (255, 220, 100),   // BSk - cold steppe
        8 => (255, 255, 0),     // Csa - Mediterranean hot
        9 => (200, 200, 0),     // Csb - Mediterranean warm
        10 => (150, 150, 0),    // Csc - Mediterranean cold
        11 => (200, 255, 80),   // Cfa - humid subtropical
        12 => (100, 200, 100),  // Cfb - oceanic
        13 => (50, 150, 50),    // Cfc - subpolar oceanic
        14 => (0, 255, 255),    // Dfa - hot-summer continental
        15 => (55, 200, 255),   // Dfb - warm-summer continental
        16 => (0, 125, 125),    // Dfc - subarctic
        17 => (0, 70, 95),      // Dfd - extreme subarctic
        18 => (255, 0, 255),    // Dsa - Med continental hot
        19 => (200, 0, 200),    // Dsb - Med continental warm
        20 => (150, 50, 150),   // Dsc - Med continental cold
        21 => (200, 205, 210),  // ET - tundra (pale grey, edges the ice)
        22 => (240, 246, 252),  // EF - ice cap (bright white permanent ice)
        23 => (110, 200, 230),  // As - savanna, dry summer
        24 => (170, 235, 120),  // Cwa - monsoon humid subtropical
        25 => (120, 205, 120),  // Cwb - subtropical highland
        26 => (80, 165, 95),    // Cwc - cold subtropical highland
        27 => (150, 130, 225),  // Dwa - dry-winter hot continental
        28 => (120, 100, 205),  // Dwb - dry-winter warm continental
        29 => (95, 85, 170),    // Dwc - dry-winter subarctic
        30 => (70, 60, 130),    // Dwd - dry-winter extreme subarctic
        // Dsd MUST differ from Dsc (20) — the two shipped identical at (150,50,150),
        // so two distinct Köppen zones rendered as one colour. (150,100,150) is the
        // published Kottek/Rubel value; codes 1-10 and 15-20 already follow that
        // table exactly, so this was a transcription gap, not a design choice.
        31 => (150, 100, 150),  // Dsd - dry-summer extreme subarctic
        32 => (185, 165, 185),  // H  - highland / alpine
        _ => (128, 128, 128),
    }
}

/// DIVERGING TEMPERATURE RAMP, PIVOTED EXACTLY AT 0 °C.
///
/// The freezing isotherm is the only semantically real break on a temperature map —
/// it decides snow, ice, the Köppen C/D boundary and whether water moves — so it
/// gets the palette's pale neutral. The old ramp put its light pivot at −5 °C and
/// left 0 °C at an undistinguished orange-yellow, so the one line a reader looks
/// for was invisible.
///
/// Taken in DEGREES, and shared by the land `temperature` layer and the ocean `sst`
/// layer, so ONE COLOUR MEANS ONE TEMPERATURE across both plates. They previously
/// shared a ramp over different domains (−40..30 vs −5..35), which meant identical
/// colours silently denoted different temperatures depending on which layer you
/// were looking at.
pub const TEMPERATURE_STOPS: [(f32, (u8, u8, u8)); 7] = [
    (-40.0, (5, 48, 97)),
    (-25.0, (43, 110, 175)),
    (-12.0, (110, 175, 215)),
    (0.0, (247, 240, 225)),
    (12.0, (250, 190, 140)),
    (25.0, (222, 110, 70)),
    (40.0, (140, 20, 35)),
];

fn temperature_color(celsius: f32) -> (u8, u8, u8) {
    ramp_lookup(celsius, &TEMPERATURE_STOPS)
}

fn render_temperature(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        let (r, g, b) = temperature_color(tile.temperature[i]);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

/// Sea-surface temperature (ocean only). Uses `temperature_color` on the SAME
/// absolute-degree scale as the land temperature layer, so a colour read off this
/// plate means the same temperature it would on that one — the two used to share a
/// ramp stretched over different domains, which made them quietly incomparable.
/// Land is transparent.
fn render_sst(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] != 0 || tile.sst.is_empty() {
            rgba[offset + 3] = 0;
            continue;
        }
        let (r, g, b) = temperature_color(tile.sst[i]);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

/// Annual snow-cover fraction (land only): transparent where snow-free, ramping to
/// opaque white at perennial cover. Highlights tundra / ice-cap / cold-continental
/// margins produced by the ice-albedo feedback.
fn render_snow(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] != 1 || tile.snow_frac.is_empty() {
            rgba[offset + 3] = 0;
            continue;
        }
        let s = tile.snow_frac[i] as f32 / 255.0;
        // Pale-blue-white snow; alpha scales with cover so thin snow reads faint.
        rgba[offset] = (200.0 + 55.0 * s) as u8;
        rgba[offset + 1] = (210.0 + 45.0 * s) as u8;
        rgba[offset + 2] = 255;
        rgba[offset + 3] = (s * 235.0) as u8;
    }
}

/// PRECIPITATION IN CLASSED ATLAS BANDS (upper bound in mm/yr, ascending).
///
/// Rainfall is log-distributed and every published atlas CLASSES it rather than
/// running a continuous ramp. The old ramp was worse than merely unclassed: it
/// interpolated tan → blue straight through a neutral grey at ~1000 mm — a
/// DIVERGING structure used for SEQUENTIAL data. Two consequences, both bad:
/// 100→500 mm (the arid/semi-humid distinction that decides whether anyone can
/// farm) separated by roughly a fifth of the colour distance that agronomically
/// identical 2000→3000 mm received; and the value most temperate land takes
/// rendered as the neutral grey that normally reads as "no data".
///
/// The scheme is sequential and monotone in lightness (a YlGnBu progression), so it
/// stays readable in greyscale and under deuteranopia.
pub const PRECIP_BANDS: [(f32, (u8, u8, u8)); 7] = [
    (125.0, (255, 255, 217)),
    (250.0, (237, 248, 177)),
    (500.0, (199, 233, 180)),
    (1000.0, (127, 205, 187)),
    (2000.0, (65, 182, 196)),
    (4000.0, (34, 94, 168)),
    // Sentinel upper bound — finite so it survives JSON round-tripping to the legend.
    (99_999.0, (12, 44, 132)),
];

fn precipitation_color(mm: f32) -> (u8, u8, u8) {
    for &(upper, c) in PRECIP_BANDS.iter() {
        if mm < upper {
            return c;
        }
    }
    PRECIP_BANDS[PRECIP_BANDS.len() - 1].1
}

fn render_precipitation(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let (r, g, b) = precipitation_color(tile.precipitation[i]);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn render_soil(tile: &TileData, n: &TileNeighbors, ctx: &RenderCtx, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let st = tile.soil_type[i];
        let (r, g, b) = match ctx.isolate {
            Some(sel) if sel != st => dim_unselected(soil_color(st)),
            _ => soil_color(st),
        };
        let m = thematic_relief(tile, n, ctx, i % SIZE, i / SIZE);
        rgba[offset] = (r as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 1] = (g as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 2] = (b as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 3] = 255;
    }
}

pub fn soil_color(code: u8) -> (u8, u8, u8) {
    match code {
        1 => (180, 50, 50),    // oxisol
        2 => (200, 80, 60),    // ultisol
        3 => (60, 60, 60),     // mollisol
        4 => (150, 120, 60),   // alfisol
        5 => (100, 100, 140),  // spodosol
        6 => (220, 200, 160),  // aridisol
        7 => (80, 60, 40),     // histosol
        8 => (180, 180, 160),  // entisol
        9 => (100, 80, 80),    // andisol
        10 => (200, 220, 240), // gelisol
        11 => (120, 100, 60),  // alluvial
        12 => (40, 30, 35),     // young volcanic ash (near-black, very dark)
        _ => (160, 160, 160),
    }
}

fn render_fertility(tile: &TileData, n: &TileNeighbors, ctx: &RenderCtx, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let f = tile.fertility[i].clamp(0.0, 1.0);
        let (r, g, b) = lerp_rgb((180, 150, 100), (0, 120, 0), f);
        let m = thematic_relief(tile, n, ctx, i % SIZE, i / SIZE);
        rgba[offset] = (r as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 1] = (g as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 2] = (b as f32 * m).clamp(0.0, 255.0) as u8;
        rgba[offset + 3] = 255;
    }
}

fn render_plates(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        let plate = tile.plate_index[i];
        let (r, g, b) = plate_color(plate);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

/// The Biomes layer: a per-biome base colour carrying a **procedural pattern
/// fill**, in the tradition of a geological/vegetation survey sheet — canopy
/// stipple for forest, tussock ticks for grassland, the standard horizontal
/// dashes for marsh, ripples for a sand sea, crevasse lines for glacier ice.
///
/// Patterns are functions of the cell's position WITHIN the tile with periods
/// that divide `TILE_SIZE` (128), so adjacent tiles line up without needing the
/// renderer to know its own world coordinates. They are symbols, not surface
/// texture: staying at a fixed pixel scale across the LOD pyramid is correct —
/// it is exactly how a printed map's hatching behaves.
/// Fade a non-selected cell toward a neutral ground, keeping a trace of its own
/// value so context survives. Desaturation rather than an outline: an outline
/// answers "where is the boundary", desaturation answers "where is this class",
/// and the second is the question being asked.
fn dim_unselected(c: (u8, u8, u8)) -> (u8, u8, u8) {
    let lum = (0.2126 * c.0 as f32 + 0.7152 * c.1 as f32 + 0.0722 * c.2 as f32) * 0.55 + 40.0;
    let g = lum.clamp(0.0, 255.0) as u8;
    (g, g, g)
}

/// How far an attenuated hillshade may modulate a THEMATIC plate (climate, biomes,
/// soil, fertility). Far below the terrain layer's own strength: the job here is to
/// hint at form so a climate zone sits on the mountains that cause it, not to
/// present relief as the subject.
const THEMATIC_RELIEF_AMP: f32 = 0.09;

/// A gentle relief multiplier for the thematic plates.
///
/// Bartholomew's and the Times' physical/vegetation plates all print their tint
/// OVER relief — it is what makes a thematic plate read as a place rather than as a
/// chart. Before this the climate plate was flat opaque fill with no terrain cue at
/// all, so mountain climate zones floated free of the mountains producing them.
///
/// It REPLACES the cruder `1.0 + (e - 0.2) * 0.18` elevation lift the biome layer
/// used to carry. That matters for §8.12's contrast ceiling: the old lift reached
/// +0.144 on its own and stacked multiplicatively with a pattern already swinging
/// ±0.19, so the combined excursion was larger than what this shading produces.
/// Replacing rather than stacking is why the combined swing goes DOWN.
fn thematic_relief(tile: &TileData, n: &TileNeighbors, ctx: &RenderCtx, x: usize, y: usize) -> f32 {
    let az = 315.0_f32.to_radians();
    let alt = 45.0_f32.to_radians();
    let (lx, ly, lz) = (az.sin() * alt.cos(), -az.cos() * alt.cos(), alt.sin());

    let (xi, yi) = (x as isize, y as isize);
    let left = halo_elev(tile, n, xi - 1, yi);
    let right = halo_elev(tile, n, xi + 1, yi);
    let up = halo_elev(tile, n, xi, yi - 1);
    let down = halo_elev(tile, n, xi, yi + 1);

    let z = ctx.z_factor();
    let dzdx = (right - left) * z * ctx.lat_stretch(y);
    let dzdy = (down - up) * z;
    let len = (dzdx * dzdx + dzdy * dzdy + 1.0).sqrt();
    let lambert = ((-dzdx / len) * lx + (-dzdy / len) * ly + (1.0 / len) * lz) / lz;

    (1.0 + (lambert - 1.0) * THEMATIC_RELIEF_AMP)
        .clamp(1.0 - THEMATIC_RELIEF_AMP, 1.0 + THEMATIC_RELIEF_AMP)
}

fn render_biomes(tile: &TileData, n: &TileNeighbors, ctx: &RenderCtx, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        // Fall back to a Köppen-derived biome on a world generated before phase
        // 6b existed (the column pads to zero), so the layer is never blank.
        let b = if tile.biome[i] != 0 {
            tile.biome[i]
        } else {
            koppen_fallback_biome(tile.koppen[i], tile.elevation[i])
        };
        let (r, g, b_) = match ctx.isolate {
            Some(sel) if sel != b => dim_unselected(biome_color(b)),
            _ => biome_color(b),
        };

        let lx = (i % SIZE) as u32;
        let ly = (i / SIZE) as u32;
        // Pattern fill, plus a gentle hypsometric lift so relief still reads
        // through the flat ecological colours.
        let pattern = biome_pattern(b, lx, ly);
        // Shading is a SEPARATE multiply applied after the pattern, so texture and
        // form stay independently readable (and independently testable).
        let k = (1.0 + pattern) * thematic_relief(tile, n, ctx, lx as usize, ly as usize);

        rgba[offset] = (r as f32 * k).clamp(0.0, 255.0) as u8;
        rgba[offset + 1] = (g as f32 * k).clamp(0.0, 255.0) as u8;
        rgba[offset + 2] = (b_ as f32 * k).clamp(0.0, 255.0) as u8;
        rgba[offset + 3] = 255;
    }
}

/// Base colour per biome code. Greens darken with canopy closure, open country
/// runs olive→straw→tan with aridity, wetlands hold a blue-green cast and the
/// cryosphere is near-white, so the map reads correctly even in greyscale.
pub fn biome_color(b: u8) -> (u8, u8, u8) {
    use crate::sim::biome::*;
    match b {
        // Tropical forest
        B_TROPICAL_RAINFOREST => (13, 74, 31),
        B_TROPICAL_MOIST_FOREST => (28, 99, 44),
        B_TROPICAL_SEASONAL_FOREST => (108, 142, 54),
        B_CLOUD_FOREST => (46, 110, 92),
        B_MANGROVE => (44, 88, 74),
        // Tropical open
        B_SAVANNA => (168, 168, 84),
        B_THORN_SCRUB => (170, 140, 76),
        // Temperate forest
        B_TEMPERATE_RAINFOREST => (32, 92, 66),
        B_TEMPERATE_DECIDUOUS => (74, 122, 52),
        B_TEMPERATE_MIXED_FOREST => (62, 108, 60),
        B_TEMPERATE_CONIFER => (44, 86, 62),
        B_SUBTROPICAL_MOIST_FOREST => (58, 116, 54),
        // Mediterranean
        B_MEDITERRANEAN_WOODLAND => (124, 134, 70),
        B_CHAPARRAL => (140, 142, 104),
        // Boreal
        B_TAIGA => (38, 76, 58),
        B_FOREST_TUNDRA => (86, 106, 82),
        // Grassland
        B_TALLGRASS_PRAIRIE => (158, 164, 88),
        B_SHORTGRASS_STEPPE => (176, 170, 108),
        B_MONTANE_GRASSLAND => (140, 148, 96),
        // Alpine
        B_ALPINE_MEADOW => (126, 154, 110),
        B_ALPINE_TUNDRA => (150, 156, 140),
        B_ALPINE_DESERT => (166, 158, 146),
        B_NIVAL => (206, 210, 216),
        // Desert
        B_HOT_DESERT => (216, 188, 130),
        B_COASTAL_FOG_DESERT => (196, 186, 168),
        B_COLD_DESERT => (186, 176, 148),
        B_SEMI_DESERT => (192, 178, 126),
        B_SALT_FLAT => (232, 228, 222),
        B_DUNE_SEA => (228, 198, 138),
        B_ROCKY_DESERT => (176, 152, 120),
        // Polar
        B_TUNDRA => (154, 166, 152),
        B_POLAR_DESERT => (188, 192, 190),
        B_ICE_SHEET => (238, 244, 250),
        B_GLACIER => (214, 230, 244),
        // Wetland & riparian
        B_PEAT_BOG => (94, 100, 74),
        B_GALLERY_FOREST => (52, 108, 58),
        B_FLOODPLAIN_GRASSLAND => (120, 148, 78),
        B_FRESHWATER_MARSH => (98, 130, 96),
        B_SWAMP_FOREST => (48, 90, 62),
        B_SALT_MARSH => (128, 140, 106),
        B_OASIS => (60, 130, 70),
        _ => (110, 118, 96),
    }
}

/// Pattern classes. Several biomes share one — a taiga and a temperate conifer
/// forest are both drawn with spires; their colours are what tell them apart.
#[derive(Clone, Copy, PartialEq)]
enum Pattern {
    /// Flat wash — used where a pattern would only add noise (ice, salt flat
    /// interior, bare polar desert).
    None,
    /// Clustered round blobs: closed broadleaf canopy.
    Canopy,
    /// Sparser canopy blobs: open woodland and seasonal forest.
    OpenCanopy,
    /// Small upward chevrons: needleleaf spires.
    Spires,
    /// Short vertical ticks in offset rows: grass sward.
    Grass,
    /// Sparse ticks with gaps: bunchgrass steppe.
    Tussock,
    /// Scattered small dots: scrub and thorn.
    Scrub,
    /// Fine sparse specks: desert gravel/sand grain.
    Stipple,
    /// Sinusoidal ripple bands: dune crests of an erg.
    Dunes,
    /// Angular broken speckle: hamada, scree and frost-shattered rock.
    Rubble,
    /// Horizontal broken dashes in rows — the standard cartographic wetland
    /// symbol, and the reason a marsh is unmistakable on sight.
    Marsh,
    /// Marsh dashes plus stipple: peat and muskeg.
    Bog,
    /// Fine diagonal lines: glacier crevasses.
    Crevasse,
    /// Low-frequency mottle: tundra's patterned ground.
    Mottle,
    /// Polygonal cracking: a dried salt pan.
    SaltPolygon,
}

fn biome_pattern_kind(b: u8) -> Pattern {
    use crate::sim::biome::*;
    match b {
        B_TROPICAL_RAINFOREST | B_TROPICAL_MOIST_FOREST | B_CLOUD_FOREST
        | B_TEMPERATE_RAINFOREST | B_SWAMP_FOREST | B_GALLERY_FOREST | B_OASIS => Pattern::Canopy,
        B_TROPICAL_SEASONAL_FOREST | B_TEMPERATE_DECIDUOUS | B_TEMPERATE_MIXED_FOREST
        | B_SUBTROPICAL_MOIST_FOREST | B_MEDITERRANEAN_WOODLAND => Pattern::OpenCanopy,
        B_TEMPERATE_CONIFER | B_TAIGA | B_FOREST_TUNDRA => Pattern::Spires,
        B_TALLGRASS_PRAIRIE | B_FLOODPLAIN_GRASSLAND | B_ALPINE_MEADOW
        | B_MONTANE_GRASSLAND => Pattern::Grass,
        B_SHORTGRASS_STEPPE | B_SAVANNA | B_SALT_MARSH => Pattern::Tussock,
        B_THORN_SCRUB | B_CHAPARRAL | B_SEMI_DESERT => Pattern::Scrub,
        B_HOT_DESERT | B_COLD_DESERT | B_COASTAL_FOG_DESERT | B_ALPINE_DESERT => Pattern::Stipple,
        B_DUNE_SEA => Pattern::Dunes,
        B_ROCKY_DESERT | B_ALPINE_TUNDRA | B_NIVAL => Pattern::Rubble,
        B_MANGROVE | B_FRESHWATER_MARSH => Pattern::Marsh,
        B_PEAT_BOG => Pattern::Bog,
        B_GLACIER => Pattern::Crevasse,
        B_TUNDRA => Pattern::Mottle,
        B_SALT_FLAT => Pattern::SaltPolygon,
        _ => Pattern::None,
    }
}

/// Brightness delta in roughly [-0.16, +0.10] for the cell at tile-local
/// `(lx, ly)`. Every period divides 128, so the fill is seamless tile-to-tile.
fn biome_pattern(b: u8, lx: u32, ly: u32) -> f32 {
    // Fold into one tile. The renderer only ever passes values already inside
    // [0, TILE_SIZE), so this changes no pixel — but it makes the whole function
    // exactly TILE_SIZE-periodic, which is the property
    // `every_biome_pattern_tiles_seamlessly` asserts, and it is what keeps the
    // hash-driven components (canopy jitter, stipple, scree) from stepping to a
    // fresh lattice row across a tile edge. The consequence, stated plainly: a
    // fill's pseudo-random component REPEATS every 128 cells. That is correct for
    // a cartographic hatch — a printed map's stipple repeats too — and is
    // invisible in practice, since the biome colour changes long before the eye
    // can find the repeat.
    let (lx, ly) = (lx % TILE_SIZE, ly % TILE_SIZE);
    match biome_pattern_kind(b) {
        Pattern::None => 0.0,

        // Canopy: blobs on a staggered 8×8 lattice. Rows offset by 4 so the
        // crowns interlock instead of forming visible columns.
        Pattern::Canopy => blob(lx, ly, 8, 2.2, -0.15, 0.06),
        Pattern::OpenCanopy => blob(lx, ly, 8, 1.5, -0.11, 0.05),

        // Spires: a 2-px-wide upward wedge in each 8×8 cell — narrow at the top,
        // wide at the base, which is what makes it read as a conifer and not a dot.
        Pattern::Spires => {
            let cy = ly % 8;
            let cx = (lx + if (ly / 8) % 2 == 1 { 4 } else { 0 }) % 8;
            let half_width = cy / 3; // 0 at the tip, 2 at the base
            let on = cy >= 1 && cy <= 6 && (cx as i32 - 4).abs() <= half_width as i32;
            if on { -0.14 } else { 0.04 }
        }

        // Grass: 2-px vertical ticks every 4 columns, rows offset every 8.
        Pattern::Grass => {
            let cx = (lx + if (ly / 8) % 2 == 1 { 2 } else { 0 }) % 4;
            let cy = ly % 8;
            if cx == 0 && cy >= 4 { -0.11 } else { 0.03 }
        }
        // Tussock: the same ticks, but only where the hash says a clump grew —
        // bunchgrass with bare ground between.
        Pattern::Tussock => {
            let cx = (lx + if (ly / 8) % 2 == 1 { 4 } else { 0 }) % 8;
            let cy = ly % 8;
            if cx == 0 && cy >= 5 && hash01(lx / 8, ly / 8, 11) > 0.35 { -0.12 } else { 0.03 }
        }

        // Scrub: isolated bushes on a sparse 8×8 lattice. The dot is 2×2, not
        // 1×1 — a single pixel per 64 is below the threshold of visibility once
        // the swatch is on screen, and the biome read as a flat wash.
        Pattern::Scrub => {
            let cx = lx % 8;
            let cy = ly % 8;
            let on = (cx == 3 || cx == 4) && (cy == 3 || cy == 4);
            if on && hash01(lx / 8, ly / 8, 23) > 0.30 { -0.15 } else { 0.02 }
        }

        // Stipple: fine sand/gravel grain — a low-density speck field.
        Pattern::Stipple => {
            let h = hash01(lx, ly, 37);
            if h > 0.93 { -0.10 } else if h < 0.06 { 0.06 } else { 0.0 }
        }

        // Dunes: sinusoidal crests running obliquely, as a real erg's do, with a
        // sharp lee slope and a soft stoss slope.
        //
        // The coefficients are NOT free: crest period is 32/1 = 32 cells across
        // and 32/2 = 16 along, and both must divide TILE_SIZE or every sand sea
        // grows a seam line at each tile edge. (The original 0.55/0.28 gave a
        // 29.09-cell period — 128/29.09 = 4.4 — and did exactly that.)
        Pattern::Dunes => {
            let t = ((lx as f32 + ly as f32 * 2.0) * std::f32::consts::TAU / 32.0).sin();
            if t > 0.55 { 0.08 } else { t * 0.10 - 0.02 }
        }

        // Rubble: angular clasts — a hash field thresholded hard so the specks
        // have edges instead of fading.
        Pattern::Rubble => {
            let h = hash01(lx / 2, ly / 2, 53);
            if h > 0.80 { -0.14 } else if h < 0.18 { 0.07 } else { 0.0 }
        }

        // Marsh: broken horizontal dashes in alternating offset rows — the
        // conventional wetland hatch.
        Pattern::Marsh => marsh_dash(lx, ly),
        // Bog: the marsh hatch, with a stipple of peat hummocks on the ground
        // BETWEEN the dashes. The two must not stack — dash and hummock together
        // pushed the cell past the readable band and made bog read as a different
        // colour entirely rather than as marsh-with-texture.
        Pattern::Bog => {
            let d = marsh_dash(lx, ly);
            if d < 0.0 {
                d // on a dash: leave it alone
            } else if hash01(lx, ly, 71) > 0.88 {
                d - 0.10
            } else {
                d
            }
        }

        // Crevasse: thin diagonal fractures across an otherwise clean ice field.
        Pattern::Crevasse => {
            let d = (lx + ly * 2) % 16;
            if d == 0 && hash01(lx / 16, ly / 8, 89) > 0.4 { -0.12 } else { 0.02 }
        }

        // Mottle: broad, soft patches — frost-heave patterned ground.
        Pattern::Mottle => (hash01(lx / 4, ly / 4, 97) - 0.5) * 0.13,

        // Salt polygon: a coarse cracked lattice with slightly irregular cells.
        Pattern::SaltPolygon => {
            let on_edge = lx % 16 == 0 || ly % 16 == 0
                || (lx % 16 == 8 && hash01(lx / 16, ly / 16, 103) > 0.5)
                || (ly % 16 == 8 && hash01(lx / 16, ly / 16, 107) > 0.5);
            if on_edge { -0.10 } else { 0.03 }
        }
    }
}

/// A round blob of radius `r` centred in each `period`×`period` lattice cell,
/// with alternate rows offset by half a period so crowns interlock. Returns
/// `inside` within the blob and `outside` between them.
fn blob(lx: u32, ly: u32, period: u32, r: f32, inside: f32, outside: f32) -> f32 {
    let half = period / 2;
    let cx = (lx + if (ly / period) % 2 == 1 { half } else { 0 }) % period;
    let cy = ly % period;
    let dx = cx as f32 - half as f32;
    let dy = cy as f32 - half as f32;
    // Jitter the radius per lattice cell so the canopy is not a perfect grid.
    let jitter = 0.75 + hash01(lx / period, ly / period, 17) * 0.5;
    if dx * dx + dy * dy <= (r * jitter) * (r * jitter) { inside } else { outside }
}

/// Broken horizontal dashes in offset rows — the wetland hatch shared by marsh,
/// mangrove and bog.
///
/// This is the most recognisable symbol on the sheet, so it carries the widest
/// contrast of any pattern here (0.24 between dash and ground, against ~0.15 for
/// the others). At the original ±0.13/0.02 the dashes were technically present
/// and visually absent, which is the worst of both.
fn marsh_dash(lx: u32, ly: u32) -> f32 {
    if ly % 4 != 1 {
        return 0.05;
    }
    let seg = (lx + if (ly / 4) % 2 == 1 { 4 } else { 0 }) % 8;
    if seg < 5 { -0.19 } else { 0.05 }
}

/// Deterministic hash in [0, 1) from two lattice coordinates and a salt. Pure
/// integer mixing — no allocation, no float noise field, and identical on every
/// platform.
///
fn hash01(x: u32, y: u32, salt: u32) -> f32 {
    let mut h = x
        .wrapping_mul(0x9E37_79B9)
        ^ y.wrapping_mul(0x85EB_CA6B)
        ^ salt.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h & 0x00FF_FFFF) as f32 / 16_777_216.0
}

/// Best-effort biome for a world saved before phase 6b existed, so the Biomes
/// layer still shows something sensible until the user re-runs the phase. This
/// is the OLD Köppen-and-elevation mapping, kept only as a fallback.
fn koppen_fallback_biome(koppen: u8, elevation: f32) -> u8 {
    use crate::sim::biome::*;
    if elevation > 0.62 { return B_NIVAL; }
    if elevation > 0.40 { return B_ALPINE_TUNDRA; }
    match koppen {
        1 => B_TROPICAL_RAINFOREST,
        2 => B_TROPICAL_MOIST_FOREST,
        3 | 23 => B_SAVANNA,
        4 => B_HOT_DESERT,
        5 => B_COLD_DESERT,
        6 => B_THORN_SCRUB,
        7 => B_SHORTGRASS_STEPPE,
        8 | 9 | 10 => B_MEDITERRANEAN_WOODLAND,
        11 | 24 => B_SUBTROPICAL_MOIST_FOREST,
        12 | 25 => B_TEMPERATE_DECIDUOUS,
        13 | 26 => B_TEMPERATE_MIXED_FOREST,
        14 | 15 | 27 | 28 => B_TEMPERATE_MIXED_FOREST,
        16 | 17 | 29 | 30 => B_TAIGA,
        18 | 19 | 20 | 31 => B_SHORTGRASS_STEPPE,
        21 => B_TUNDRA,
        22 => B_ICE_SHEET,
        32 => B_ALPINE_TUNDRA,
        _ => B_TUNDRA,
    }
}

fn render_fisheries(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            rgba[offset + 3] = 0; // land = transparent
            continue;
        }
        let f = tile.fishery[i].clamp(0.0, 1.0);
        if f < 0.01 {
            // Low fishery — dark ocean
            rgba[offset] = 8;
            rgba[offset + 1] = 20;
            rgba[offset + 2] = 50;
        } else {
            // Fishery gradient: dark blue → cyan → yellow
            let (r, g, b) = if f < 0.5 {
                lerp_rgb((10, 30, 80), (20, 160, 200), f * 2.0)
            } else {
                lerp_rgb((20, 160, 200), (220, 220, 60), (f - 0.5) * 2.0)
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

fn render_habitability(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            // Sea: dim so the land heatmap stands out.
            rgba[offset] = 8;
            rgba[offset + 1] = 14;
            rgba[offset + 2] = 28;
            rgba[offset + 3] = 255;
            continue;
        }
        let v = tile.habitability[i].clamp(0.0, 1.0);
        // Cold (low) → blue/teal, mid → green/yellow, hot (high) → orange/red.
        let (r, g, b) = if v < 0.25 {
            lerp_rgb((20, 30, 70), (20, 110, 130), v / 0.25)
        } else if v < 0.5 {
            lerp_rgb((20, 110, 130), (60, 170, 70), (v - 0.25) / 0.25)
        } else if v < 0.75 {
            lerp_rgb((60, 170, 70), (220, 200, 40), (v - 0.5) / 0.25)
        } else {
            lerp_rgb((220, 200, 40), (230, 40, 30), (v - 0.75) / 0.25)
        };
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn render_salinity(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            // Land is dim so the sea field reads clearly — EXCEPT terminal salt
            // lakes, whose brine was written into the column (compute salt pans):
            // show them on a pink→white hypersaline ramp so the salinity index
            // surfaces the inland playas too.
            let sl = tile.salinity[i] as f32 / 255.0; // 0 ↔ 28 PSU … 1 ↔ 42 PSU
            if sl > 0.45 { // ≳ 34 PSU: an evaporite salt lake, not ordinary land
                let (r, g, b) = if sl < 0.85 {
                    lerp_rgb((198, 110, 150), (232, 150, 120), (sl - 0.45) / 0.40) // rose → salmon
                } else {
                    lerp_rgb((232, 150, 120), (245, 238, 232), (sl - 0.85) / 0.15) // → salt-crust white
                };
                rgba[offset] = r; rgba[offset + 1] = g; rgba[offset + 2] = b; rgba[offset + 3] = 255;
            } else {
                rgba[offset] = 18; rgba[offset + 1] = 22; rgba[offset + 2] = 26; rgba[offset + 3] = 255;
            }
            continue;
        }
        // u8 0..255 ↔ ~28-42 PSU. Fresh → teal-green, ~normal → blue,
        // hypersaline → violet/white.
        let s = tile.salinity[i] as f32 / 255.0;
        let (r, g, b) = if s < 0.4 {
            lerp_rgb((40, 170, 150), (30, 90, 200), s / 0.4)          // fresh → blue
        } else if s < 0.75 {
            lerp_rgb((30, 90, 200), (140, 70, 200), (s - 0.4) / 0.35) // blue → violet
        } else {
            lerp_rgb((140, 70, 200), (240, 230, 250), (s - 0.75) / 0.25) // → near-white
        };
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn render_shark(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            rgba[offset + 3] = 0; // land transparent
            continue;
        }
        let v = tile.shark_risk[i] as f32 / 255.0;
        if v < 0.01 {
            rgba[offset] = 8;
            rgba[offset + 1] = 20;
            rgba[offset + 2] = 50;
        } else {
            // Calm water → menacing red as risk climbs.
            let (r, g, b) = if v < 0.5 {
                lerp_rgb((10, 40, 80), (210, 160, 40), v * 2.0)
            } else {
                lerp_rgb((210, 160, 40), (200, 30, 30), (v - 0.5) * 2.0)
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

fn render_shipworm(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            rgba[offset + 3] = 0; // land transparent
            continue;
        }
        let v = tile.shipworm_risk[i] as f32 / 255.0;
        if v < 0.01 {
            rgba[offset] = 8;
            rgba[offset + 1] = 20;
            rgba[offset + 2] = 50;
        } else {
            // Calm water → rotting-timber brown/orange as hazard climbs.
            let (r, g, b) = if v < 0.5 {
                lerp_rgb((10, 40, 80), (150, 110, 50), v * 2.0)
            } else {
                lerp_rgb((150, 110, 50), (140, 70, 30), (v - 0.5) * 2.0)
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

fn render_storm(tile: &TileData, rgba: &mut [u8]) {
    // Annual (combined) storm danger: calm blue → violent purple/magenta.
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            rgba[offset + 3] = 0; // land transparent
            continue;
        }
        let v = tile.storm_base[i] as f32 / 255.0;
        if v < 0.01 {
            rgba[offset] = 8;
            rgba[offset + 1] = 20;
            rgba[offset + 2] = 50;
        } else {
            let (r, g, b) = if v < 0.5 {
                lerp_rgb((10, 40, 80), (120, 90, 180), v * 2.0)
            } else {
                lerp_rgb((120, 90, 180), (210, 50, 160), (v - 0.5) * 2.0)
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

fn render_reef(tile: &TileData, rgba: &mut [u8]) {
    // Reef/shoal wreck hazard: deep blue → turquoise → warning yellow.
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            rgba[offset + 3] = 0; // land transparent
            continue;
        }
        let v = tile.reef_risk[i] as f32 / 255.0;
        if v < 0.01 {
            rgba[offset] = 8;
            rgba[offset + 1] = 20;
            rgba[offset + 2] = 50;
        } else {
            let (r, g, b) = if v < 0.5 {
                lerp_rgb((10, 40, 80), (40, 180, 170), v * 2.0)
            } else {
                lerp_rgb((40, 180, 170), (230, 210, 70), (v - 0.5) * 2.0)
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

fn render_disease(tile: &TileData, rgba: &mut [u8]) {
    // Malaria/fever risk on LAND: clear green → sickly yellow → fever red/purple.
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0; // sea transparent
            continue;
        }
        let v = tile.disease_risk[i] as f32 / 255.0;
        if v < 0.01 {
            rgba[offset] = 40;
            rgba[offset + 1] = 70;
            rgba[offset + 2] = 45;
        } else {
            let (r, g, b) = if v < 0.5 {
                lerp_rgb((50, 150, 50), (200, 190, 60), v * 2.0)
            } else {
                lerp_rgb((200, 190, 60), (150, 30, 120), (v - 0.5) * 2.0)
            };
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

/// Cardinal neighbour tiles for a seam-free hillshade slope stencil. Missing
/// neighbours (world edge / not loaded) fall back to edge replication.
#[derive(Default)]
pub struct TileNeighbors<'a> {
    pub north: Option<&'a TileData>, // ty - 1 (up, y decreases)
    pub south: Option<&'a TileData>, // ty + 1 (down)
    pub west: Option<&'a TileData>,  // tx - 1 (left)
    pub east: Option<&'a TileData>,  // tx + 1 (right)
}

/// Elevation at a tile-local coordinate that may step ONE cell outside the tile
/// (x or y == -1 or SIZE). Out-of-range reads pull the adjacent neighbour tile's
/// edge line so the slope is continuous across tile boundaries; if the neighbour
/// is absent the value replicates the tile's own edge (old behaviour).
#[inline]
fn halo_elev(tile: &TileData, n: &TileNeighbors, x: isize, y: isize) -> f32 {
    let sz = SIZE as isize;
    if x < 0 {
        let yy = y.clamp(0, sz - 1) as usize;
        return n
            .west
            .map(|t| t.elevation[yy * SIZE + (SIZE - 1)])
            .unwrap_or(tile.elevation[yy * SIZE]);
    }
    if x >= sz {
        let yy = y.clamp(0, sz - 1) as usize;
        return n
            .east
            .map(|t| t.elevation[yy * SIZE])
            .unwrap_or(tile.elevation[yy * SIZE + (SIZE - 1)]);
    }
    if y < 0 {
        let xx = x as usize;
        return n
            .north
            .map(|t| t.elevation[(SIZE - 1) * SIZE + xx])
            .unwrap_or(tile.elevation[xx]);
    }
    if y >= sz {
        let xx = x as usize;
        return n
            .south
            .map(|t| t.elevation[xx])
            .unwrap_or(tile.elevation[(SIZE - 1) * SIZE + xx]);
    }
    tile.elevation[y as usize * SIZE + x as usize]
}

/// How dark a fully-shadowed slope may get, as a fraction of its own tint. Imhof's
/// rule is that shadows stay LIGHT and keep their colour — the old 0.15 floor was a
/// near-black multiply that destroyed hypsometric tint precisely on the mountains.
const SHADOW_FLOOR: f32 = 0.55;

/// Vertical exaggeration at the REFERENCE grid. Shaded relief is always
/// exaggerated; what matters is that the exaggeration means the same thing on
/// every world.
const RELIEF_Z: f32 = 50.0;

/// The grid this z-factor was originally tuned against (the default 3600×1800
/// world). Keeping it explicit is what makes the new scaling BIT-IDENTICAL there
/// while fixing every other size.
const RELIEF_REF_GRID_W: f32 = 3600.0;

/// Geometry the hillshade needs and a `TileData` cannot carry: how large a cell is
/// on the ground, and where the tile sits in latitude.
///
/// Two defects come from not having it:
///
/// 1. **Relief strength tracked grid size.** The z-factor was the hard-coded
///    constant 50, applied to a per-cell elevation difference. On a "Large" world
///    (2× resolution) the difference between adjacent cells is half as big, so the
///    same mountains rendered half as boldly — the map changed because the raster
///    changed, which is exactly what a z-factor exists to prevent.
/// 2. **Polar relief was under-shaded.** On a cylindrical grid east-west ground
///    distance per cell shrinks with `cos(latitude)`, so at 60° a cell is half as
///    wide as at the equator. Treating it as square made identical real slopes
///    shade progressively flatter toward the poles.
///
/// `Default` means "geometry unknown" and falls back to the legacy fixed z-factor,
/// so any call site that has not been converted keeps its exact current output.
#[derive(Clone, Copy, Default)]
pub struct RenderCtx {
    /// World grid width in cells. 0 = unknown → legacy behaviour.
    pub grid_w: u32,
    /// World grid height in cells. 0 = unknown → no latitude correction.
    pub grid_h: u32,
    /// Tile row of the tile being rendered (for latitude).
    pub ty: i32,
    /// Base cells per rendered pixel: 1 at LOD 0, 2^lod on a supertile. Relief must
    /// not strengthen just because the pyramid samples more coarsely.
    pub step: u32,
    /// CLASS ISOLATION: when set, cells of this class keep their colour and every
    /// other cell is desaturated toward the ground. On a 41-class biome map where
    /// adjacent greens are near-indistinguishable, this answers "where is THIS one"
    /// without needing a palette that separates 41 classes — which may not exist.
    ///
    /// Isolation is done HERE rather than in the frontend because only the renderer
    /// knows each cell's class; matching colours back in canvas would be both slow
    /// and, now that the thematic plates carry relief shading, wrong.
    pub isolate: Option<u8>,
}

impl RenderCtx {
    /// Vertical exaggeration for this world and zoom level. Exactly `RELIEF_Z` on
    /// the reference grid at LOD 0, which is what keeps the default world's
    /// hillshade unchanged.
    fn z_factor(&self) -> f32 {
        if self.grid_w == 0 {
            return RELIEF_Z;
        }
        let step = self.step.max(1) as f32;
        RELIEF_Z * (self.grid_w as f32 / RELIEF_REF_GRID_W) / step
    }

    /// `1 / cos(latitude)` for a tile-local row — how much narrower a cell is here
    /// than at the equator. Clamped, or the correction diverges at the pole.
    fn lat_stretch(&self, y_in_tile: usize) -> f32 {
        if self.grid_h == 0 {
            return 1.0;
        }
        let step = self.step.max(1) as i64;
        let gy = step * (self.ty as i64 * SIZE as i64 + y_in_tile as i64);
        let frac = gy as f32 / self.grid_h as f32;
        let lat_rad = ((0.5 - frac) * std::f32::consts::PI).clamp(
            -std::f32::consts::FRAC_PI_2,
            std::f32::consts::FRAC_PI_2,
        );
        1.0 / lat_rad.cos().max(0.15)
    }
}

fn render_terrain_hillshade_halo(tile: &TileData, n: &TileNeighbors, ctx: &RenderCtx, rgba: &mut [u8]) {
    // Directional light from NW (azimuth 315, altitude 45 degrees)
    let az = 315.0_f32.to_radians();
    let alt = 45.0_f32.to_radians();
    let light_x = az.sin() * alt.cos();
    let light_y = -az.cos() * alt.cos(); // negative because y increases downward
    let light_z = alt.sin();
    let z = ctx.z_factor();

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let offset = i * 4;

            if tile.terrain[i] == 0 {
                // Sea: the shared bathymetry ramp (see BATHYMETRY_STOPS).
                let (r, g, b) = bathymetry_color(tile.sea_depth[i]);
                rgba[offset] = r;
                rgba[offset + 1] = g;
                rgba[offset + 2] = b;
                rgba[offset + 3] = 255;
                continue;
            }

            let e = tile.elevation[i];
            // Slope from neighbours, reaching into adjacent tiles at the edges so
            // there is no derivative discontinuity at 128-cell tile boundaries.
            let (xi, yi) = (x as isize, y as isize);
            let left = halo_elev(tile, n, xi - 1, yi);
            let right = halo_elev(tile, n, xi + 1, yi);
            let up = halo_elev(tile, n, xi, yi - 1);
            let down = halo_elev(tile, n, xi, yi + 1);

            // Z-factor derived from real ground distance, so the same mountains
            // shade the same on any grid size and at any LOD; the x-gradient is
            // additionally stretched by 1/cos(lat) because an equirectangular
            // cell narrows toward the poles.
            let dzdx = (right - left) * z * ctx.lat_stretch(y);
            let dzdy = (down - up) * z;

            // Surface normal
            let len = (dzdx * dzdx + dzdy * dzdy + 1.0).sqrt();
            let nx = -dzdx / len;
            let ny = -dzdy / len;
            let nz = 1.0 / len;

            // Lambertian, NORMALISED so flat ground shades to exactly 1.0. Raw
            // Lambertian gives flat land `sin(45°)` = 0.707, so the whole terrain
            // layer rendered ~29% darker than the elevation layer showing the very
            // same hypsometric colours.
            let lambert = (nx * light_x + ny * light_y + nz * light_z) / light_z;

            // AERIAL PERSPECTIVE (Imhof): shading contrast is LOW in the lowlands
            // and rises with elevation, so a summit reads crisp while a plain stays
            // soft and keeps its tint. The old code did the reverse — a flat 0.15
            // shadow floor crushed mountain colour to 21% of its value, exactly
            // where hypsometric tint carries the most information.
            let contrast = 0.42 + 0.58 * e.clamp(0.0, 1.0);
            let shade = (1.0 + (lambert - 1.0) * contrast).clamp(SHADOW_FLOOR, 1.30);

            // Shadow is SKYLIGHT, and skylight is blue: a shaded slope should lose
            // more red than blue rather than darkening to neutral grey.
            let shadow = (1.0 - shade).max(0.0);
            let (kr, kg, kb) = (
                shade * (1.0 - 0.10 * shadow),
                shade * (1.0 - 0.03 * shadow),
                shade * (1.0 + 0.14 * shadow),
            );

            let (br, bg, bb) = elevation_color_climate(e, tile.temperature[i], tile.precipitation[i]);
            rgba[offset] = (br as f32 * kr).clamp(0.0, 255.0) as u8;
            rgba[offset + 1] = (bg as f32 * kg).clamp(0.0, 255.0) as u8;
            rgba[offset + 2] = (bb as f32 * kb).clamp(0.0, 255.0) as u8;
            rgba[offset + 3] = 255;
        }
    }
}

/// Like `render_tile`, but for the "terrain" hillshade layer it uses the given
/// cardinal neighbour tiles to compute seam-free slopes. All other layers ignore
/// the neighbours and render identically to `render_tile`.
pub fn render_tile_with_neighbors(tile: &TileData, layer: &str, n: &TileNeighbors) -> Vec<u8> {
    render_tile_with_neighbors_ctx(tile, layer, n, &RenderCtx::default())
}

/// As `render_tile_with_neighbors`, plus the world geometry (see `RenderCtx`).
pub fn render_tile_with_neighbors_ctx(
    tile: &TileData,
    layer: &str,
    n: &TileNeighbors,
    ctx: &RenderCtx,
) -> Vec<u8> {
    render_tile_full(tile, layer, n, ctx)
}

fn render_shelf(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            // Land: muted green
            rgba[offset] = 80;
            rgba[offset + 1] = 100;
            rgba[offset + 2] = 70;
            rgba[offset + 3] = 255;
        } else if tile.is_shelf_edge[i] != 0 {
            // Shelf edge: distinct orange
            rgba[offset] = 180;
            rgba[offset + 1] = 120;
            rgba[offset + 2] = 50;
            rgba[offset + 3] = 255;
        } else if tile.is_shelf[i] != 0 {
            // Shelf: light blue
            rgba[offset] = 60;
            rgba[offset + 1] = 150;
            rgba[offset + 2] = 200;
            rgba[offset + 3] = 255;
        } else {
            // Deep sea
            let d = tile.sea_depth[i].clamp(0.0, 1.0);
            let (r, g, b) = lerp_rgb((20, 50, 120), (5, 10, 30), d);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = 255;
        }
    }
}

fn render_ridges(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let e = tile.elevation[i].clamp(0.0, 1.0);
        let (br, bg, bb) = elevation_color(e);

        // Highlight convergent boundaries (mountain ridges)
        if tile.boundary_type[i] == 1 {
            // Convergent: warm brown-orange highlight
            rgba[offset] = ((br as f32 * 0.5) + 128.0).min(255.0) as u8;
            rgba[offset + 1] = ((bg as f32 * 0.4) + 60.0).min(255.0) as u8;
            rgba[offset + 2] = (bb as f32 * 0.3) as u8;
        } else if tile.is_volcanic[i] != 0 {
            // Volcanic: red tint
            rgba[offset] = ((br as f32 * 0.4) + 150.0).min(255.0) as u8;
            rgba[offset + 1] = (bg as f32 * 0.4) as u8;
            rgba[offset + 2] = (bb as f32 * 0.3) as u8;
        } else {
            rgba[offset] = br;
            rgba[offset + 1] = bg;
            rgba[offset + 2] = bb;
        }
        rgba[offset + 3] = 255;
    }
}

fn render_wind(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        let vx = tile.wind_vx[i];
        let vy = tile.wind_vy[i];
        let mag = (vx * vx + vy * vy).sqrt().clamp(0.0, 2.0) / 2.0;

        if tile.terrain[i] == 1 {
            // Land: muted base with wind magnitude overlay
            let (r, g, b) = lerp_rgb((60, 70, 60), (180, 220, 255), mag);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        } else {
            let (r, g, b) = lerp_rgb((10, 20, 50), (100, 180, 240), mag);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
        }
        rgba[offset + 3] = 255;
    }
}

/// Wind Speed layer: a filled low-level wind-intensity field (incl. jets like the
/// Somali Jet) over the whole map, using the classic monsoon-map ramp
/// (tan → yellow → orange → red) keyed on `wind_speed` (m/s), 0 → 30.
fn render_windspeed(tile: &TileData, rgba: &mut [u8]) {
    // Colour stops at 0,4,8,12,16,20,24,30 m/s — pale tan through deep red.
    const STOPS: [(f32, (u8, u8, u8)); 8] = [
        (0.0, (245, 240, 225)),  // near-calm: off-white
        (4.0, (232, 224, 186)),  // pale tan
        (8.0, (224, 205, 130)),  // tan
        (12.0, (240, 220, 90)),  // yellow
        (16.0, (240, 170, 55)),  // orange
        (20.0, (226, 110, 40)),  // deep orange
        (24.0, (200, 55, 40)),   // red
        (30.0, (140, 20, 30)),   // deep red
    ];
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        let s = tile.wind_speed[i].max(0.0);
        // Find the bracketing stops and lerp.
        let (mut r, mut g, mut b) = STOPS[STOPS.len() - 1].1;
        for w in STOPS.windows(2) {
            let (s0, c0) = w[0];
            let (s1, c1) = w[1];
            if s <= s1 {
                let t = if s1 > s0 { ((s - s0) / (s1 - s0)).clamp(0.0, 1.0) } else { 0.0 };
                let (rr, gg, bb) = lerp_rgb(c0, c1, t);
                r = rr; g = gg; b = bb;
                break;
            }
        }
        // Darken land slightly so coastlines stay readable under the fill.
        if tile.terrain[i] == 1 {
            r = (r as f32 * 0.88) as u8;
            g = (g as f32 * 0.88) as u8;
            b = (b as f32 * 0.88) as u8;
        }
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn render_currents(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 1 {
            rgba[offset + 3] = 0; // land = transparent
            continue;
        }
        // Boundary currents run along the continental slope, not over the shallow
        // apron, so the drawn flow stops at the shelf break. This is a RENDERING
        // choice and lives here deliberately: the simulation keeps a real velocity
        // on shelf cells because upwelling, the current→coast temperature coupling
        // and the SST anomaly all read that speed (see the note at the end of
        // `ocean::generate_ocean_currents`).
        let mag = if tile.is_shelf[i] != 0 {
            0.0
        } else {
            let vx = tile.current_vx[i];
            let vy = tile.current_vy[i];
            (vx * vx + vy * vy).sqrt().clamp(0.0, 3.0) / 3.0
        };
        let ct = tile.current_type[i];

        let (r, g, b) = match ct {
            1 => lerp_rgb((20, 10, 30), (220, 80, 40), mag),   // warm: dark → red
            2 => lerp_rgb((10, 15, 40), (40, 120, 220), mag),  // cold: dark → blue
            _ => lerp_rgb((10, 15, 30), (100, 140, 160), mag), // neutral
        };
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn plate_color(index: u16) -> (u8, u8, u8) {
    // Deterministic distinct colors from plate index
    let hue = ((index as f32 * 137.508) % 360.0) / 360.0;
    hsv_to_rgb(hue, 0.6, 0.8)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t) as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t) as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::biome::*;

    /// Relative luminance (sRGB → linear → Y). Monotone with CIE L*, which is all
    /// the ramp-ordering tests below need, and far cheaper than a full Lab convert.
    fn luminance(c: (u8, u8, u8)) -> f32 {
        let f = |v: u8| {
            let s = v as f32 / 255.0;
            if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
    }

    /// CIE L*a*b* (D65), for perceptual colour distance. Two colours a reader
    /// cannot separate are the same colour as far as a map is concerned, and RGB
    /// distance does not model that — hence the conversion rather than a cheap
    /// component compare.
    fn to_lab(c: (u8, u8, u8)) -> (f32, f32, f32) {
        let f = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        let (r, g, b) = (f(c.0), f(c.1), f(c.2));
        let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
        let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
        let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;
        let h = |t: f32| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
        let (fx, fy, fz) = (h(x), h(y), h(z));
        (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
    }

    fn delta_e(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
        let (l1, a1, b1) = to_lab(a);
        let (l2, a2, b2) = to_lab(b);
        ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
    }

    /// No two Köppen zones may share a colour.
    ///
    /// Dsc (20) and Dsd (31) shipped IDENTICAL at (150,50,150), so two distinct
    /// zones rendered as one and no test noticed — the map simply lied about how
    /// many climates it had. Equality is the right check here (unlike biomes, where
    /// §8.12's pattern fills carry extra separation); Köppen is flat colour only.
    #[test]
    fn koppen_colors_are_distinct() {
        let live: Vec<u8> = (1..=31).collect();
        for (ai, &a) in live.iter().enumerate() {
            for &b in live.iter().skip(ai + 1) {
                assert_ne!(
                    koppen_color(a),
                    koppen_color(b),
                    "Köppen codes {a} and {b} render as the same colour",
                );
            }
        }
    }

    /// The hypsometric ramp must brighten monotonically with height.
    ///
    /// Imhof's rule: higher is brighter. The old ramp ran light → dark → light
    /// through a brown midband, so a shaded high ridge and a sunlit lowland
    /// resolved to the same value and elevation fought the hillshade for the same
    /// channel. This is the gate on that never coming back.
    #[test]
    fn elevation_ramp_is_monotone_in_lightness() {
        for w in ELEVATION_STOPS.windows(2) {
            let (m0, c0) = w[0];
            let (m1, c1) = w[1];
            assert!(m1 > m0, "elevation stops must ascend in metres ({m0} → {m1})");
            assert!(
                luminance(c1) > luminance(c0),
                "elevation ramp darkens from {m0}m to {m1}m ({:?} → {:?})",
                c0,
                c1,
            );
        }
    }

    /// Deeper water must render darker, on every layer that draws sea.
    ///
    /// `render_elevation` used to invert this — dark shelf brightening to abyss —
    /// while the legend described the shared bright-shelf ramp, so the key was
    /// backwards on exactly one layer.
    #[test]
    fn bathymetry_darkens_with_depth() {
        for w in BATHYMETRY_STOPS.windows(2) {
            let (d0, c0) = w[0];
            let (d1, c1) = w[1];
            assert!(d1 > d0, "bathymetry stops must ascend in depth");
            assert!(
                luminance(c1) < luminance(c0),
                "bathymetry brightens from depth {d0} to {d1} ({:?} → {:?})",
                c0,
                c1,
            );
        }
        // The three layers that draw sea must agree, or a single key cannot
        // describe all of them — which is the defect this ramp was extracted to fix.
        for d in [0.0f32, 0.2, 0.5, 0.9, 1.0] {
            assert_eq!(bathymetry_color(d), ramp_lookup(d, &BATHYMETRY_STOPS));
        }
    }

    /// A sequential ramp must never pass through a neutral grey — that is the
    /// colour a reader takes for "no data", and it fell at ~1000 mm, the value most
    /// temperate land actually has. Classed bands also have to stay ordered.
    #[test]
    fn precipitation_bands_are_sequential_and_never_neutral() {
        for w in PRECIP_BANDS.windows(2) {
            assert!(w[1].0 > w[0].0, "precipitation bands must ascend");
            assert!(
                luminance(w[1].1) < luminance(w[0].1),
                "precipitation bands must darken as rainfall rises",
            );
        }
        for &(upper, (r, g, b)) in PRECIP_BANDS.iter() {
            let (mx, mn) = (r.max(g).max(b) as i32, r.min(g).min(b) as i32);
            assert!(
                mx - mn > 20,
                "band below {upper}mm is near-neutral ({r},{g},{b}) — reads as 'no data'",
            );
        }
    }

    /// The relief z-factor must mean the same thing on every world.
    ///
    /// Two properties, and the first is what makes the change safe to ship: on the
    /// REFERENCE grid at LOD 0 the z-factor is still exactly the old hard-coded 50,
    /// so the default world's hillshade is bit-identical. Everything else is the fix
    /// — a 2× grid has half the per-cell elevation difference and so needs twice the
    /// exaggeration to render the same mountains, and a supertile stepping over 2^L
    /// cells needs correspondingly less.
    #[test]
    fn relief_z_factor_is_grid_and_lod_independent() {
        let at = |w: u32, step: u32| {
            RenderCtx { grid_w: w, grid_h: w / 2, ty: 0, step, isolate: None }.z_factor()
        };
        assert_eq!(at(3600, 1), RELIEF_Z, "the reference grid must be unchanged");
        assert_eq!(RenderCtx::default().z_factor(), RELIEF_Z, "unknown geometry = legacy");

        // Twice the resolution halves the per-cell delta, so it needs twice the z.
        assert!((at(7200, 1) - 2.0 * RELIEF_Z).abs() < 1e-3);
        // A supertile spans `step` base cells per pixel, so its delta is already
        // larger by that factor and the exaggeration must come back down.
        assert!((at(3600, 4) - RELIEF_Z / 4.0).abs() < 1e-3);
    }

    /// An equirectangular cell narrows toward the poles, so identical real slopes
    /// used to shade progressively flatter with latitude. The correction must grow
    /// away from the equator and stay bounded at the pole.
    #[test]
    fn polar_cells_are_stretched_not_squared() {
        let ctx = RenderCtx { grid_w: 3600, grid_h: 1800, ty: 0, step: 1, isolate: None };
        // Row 0 of tile row 0 is the north pole; the equator is grid_h / 2.
        let equator = RenderCtx { grid_w: 3600, grid_h: 1800, ty: 7, step: 1, isolate: None }.lat_stretch(4);
        let pole = ctx.lat_stretch(0);
        assert!((equator - 1.0).abs() < 0.02, "equator must be unstretched, got {equator}");
        assert!(pole > equator, "polar cells must stretch more than equatorial");
        assert!(pole <= 1.0 / 0.15 + 1e-3, "stretch must stay bounded at the pole");
        // Unknown geometry never stretches — legacy behaviour intact.
        assert_eq!(RenderCtx::default().lat_stretch(0), 1.0);
    }

    /// Cross-blended tints must VANISH with height, or two mountain ranges in
    /// different climates stop being comparable and the reader mistakes a climate
    /// boundary for terrain. Above the ceiling every climate shares one ramp.
    #[test]
    fn cross_blended_tints_converge_with_height() {
        let desert = (35.0f32, 40.0f32);
        let jungle = (27.0f32, 3200.0f32);
        let tundra = (-15.0f32, 250.0f32);

        // At sea level the three climates must be visibly different lowlands —
        // that is the entire point of the technique.
        let d0 = elevation_color_climate(0.0, desert.0, desert.1);
        let j0 = elevation_color_climate(0.0, jungle.0, jungle.1);
        let t0 = elevation_color_climate(0.0, tundra.0, tundra.1);
        let spread = |a: (u8, u8, u8), b: (u8, u8, u8)| {
            (a.0 as i32 - b.0 as i32).abs()
                + (a.1 as i32 - b.1 as i32).abs()
                + (a.2 as i32 - b.2 as i32).abs()
        };
        assert!(spread(d0, j0) > 40, "desert and jungle lowland look alike: {d0:?} vs {j0:?}");
        assert!(spread(d0, t0) > 20, "desert and tundra lowland look alike: {d0:?} vs {t0:?}");

        // Above the ceiling they must be IDENTICAL, and equal to the base ramp.
        let high = (LOWLAND_TINT_CEILING_M + 1.0) / ELEV_MAX_M;
        let base = elevation_color(high);
        for (t, p) in [desert, jungle, tundra] {
            assert_eq!(
                elevation_color_climate(high, t, p),
                base,
                "climate still tints the plate above {LOWLAND_TINT_CEILING_M} m",
            );
        }
    }

    /// 0 °C is the only semantically real break on a temperature map (snow, ice, the
    /// Köppen C/D boundary), so it must be the diverging ramp's own pivot — the
    /// lightest stop — rather than landing mid-hue as it used to at −5 °C.
    #[test]
    fn temperature_ramp_pivots_on_freezing() {
        let freezing = temperature_color(0.0);
        let lum_zero = luminance(freezing);
        for &(deg, c) in TEMPERATURE_STOPS.iter() {
            if deg != 0.0 {
                assert!(
                    luminance(c) < lum_zero,
                    "{deg}°C ({c:?}) is lighter than the 0°C pivot — the freezing line must be the pale band",
                );
            }
        }
        // Cold reads blue, warm reads red — the direction a reader assumes.
        let cold = temperature_color(-30.0);
        let warm = temperature_color(30.0);
        assert!(cold.2 > cold.0, "sub-zero temperatures must read blue");
        assert!(warm.0 > warm.2, "above-zero temperatures must read warm");
    }

    /// Every biome's pattern fill must be SEAMLESS across a tile boundary.
    ///
    /// Patterns are functions of the cell's position WITHIN its tile, and the
    /// renderer never learns its own world coordinates — so the only thing making
    /// two neighbouring tiles line up is that every pattern period divides
    /// `TILE_SIZE`. This test states that directly: continuing the pattern
    /// function past the tile edge must reproduce it exactly, for every biome and
    /// every row. A new pattern with a period of, say, 6 or 10 px would leave a
    /// visible grid of seams across the whole map, and this is what catches it.
    #[test]
    fn every_biome_pattern_tiles_seamlessly() {
        for b in 1..BIOME_COUNT as u8 {
            for y in 0..SIZE as u32 {
                for x in 0..SIZE as u32 {
                    let here = biome_pattern(b, x, y);
                    let wrapped_x = biome_pattern(b, x + TILE_SIZE, y);
                    let wrapped_y = biome_pattern(b, x, y + TILE_SIZE);
                    assert_eq!(here, wrapped_x,
                        "biome {} ({}) breaks across a VERTICAL tile seam at x={x}, y={y}",
                        b, biome_name(b));
                    assert_eq!(here, wrapped_y,
                        "biome {} ({}) breaks across a HORIZONTAL tile seam at x={x}, y={y}",
                        b, biome_name(b));
                }
            }
        }
    }

    /// The pattern must stay a MODULATION, never a repaint: a swatch has to read
    /// as its own colour, so no cell may be pushed more than ~20% off the base.
    /// Two biomes whose base colours are close would otherwise overlap once their
    /// patterns swing, and the legend stops matching the map.
    #[test]
    fn pattern_amplitude_stays_within_a_readable_band() {
        for b in 1..BIOME_COUNT as u8 {
            for y in 0..SIZE as u32 {
                for x in 0..SIZE as u32 {
                    let p = biome_pattern(b, x, y);
                    assert!(p > -0.20 && p < 0.20,
                        "biome {} ({}) pattern swings {p} at ({x},{y}) — too far off base",
                        b, biome_name(b));
                }
            }
        }
    }

    /// The pattern ceiling above is NOT enough on its own once the biome plate also
    /// carries relief shading: what a reader actually sees is pattern × shading, and
    /// a test that measures only the pattern can pass while two biomes blur together.
    ///
    /// The COMBINED excursion is therefore held here, and the bound is deliberately
    /// tighter than the code it replaced. The old plate stacked a crude
    /// `1.0 + (e − 0.2) · 0.18` elevation lift (reaching +0.144 alone) on the same
    /// ±0.19 pattern, for a worst case near ×1.36. Replacing that lift with the
    /// attenuated hillshade — rather than adding shading on top of it — is what
    /// makes the combined swing go DOWN while the plate gains real form.
    #[test]
    fn pattern_and_relief_together_stay_readable() {
        const COMBINED_CEILING: f32 = 0.30;
        // Worst case for the shading term, independent of terrain: the clamp bound.
        let shade_hi = 1.0 + THEMATIC_RELIEF_AMP;
        let shade_lo = 1.0 - THEMATIC_RELIEF_AMP;
        for b in 1..BIOME_COUNT as u8 {
            let mut worst = 0.0f32;
            for y in 0..SIZE as u32 {
                for x in 0..SIZE as u32 {
                    let p = biome_pattern(b, x, y);
                    let hi = ((1.0 + p) * shade_hi - 1.0).abs();
                    let lo = ((1.0 + p) * shade_lo - 1.0).abs();
                    worst = worst.max(hi).max(lo);
                }
            }
            assert!(
                worst <= COMBINED_CEILING,
                "biome {} ({}) swings {worst:.3} once relief shading is applied — \
                 texture and form together stop reading as one surface",
                b, biome_name(b),
            );
        }
    }

    /// Class isolation rides in the layer key, so the parser is what stands between
    /// a typo and a blank map. An unparsable code must degrade to the plain layer,
    /// never to an error or to isolating class 0.
    #[test]
    fn isolate_layer_keys_parse_or_degrade() {
        assert_eq!(split_isolate("biomes"), ("biomes", None));
        assert_eq!(split_isolate("biomes#iso=12"), ("biomes", Some(12)));
        assert_eq!(split_isolate("climate#iso=0"), ("climate", Some(0)));
        // Garbage keeps the base layer and drops the isolation.
        assert_eq!(split_isolate("soil#iso="), ("soil", None));
        assert_eq!(split_isolate("soil#iso=abc"), ("soil", None));
        assert_eq!(split_isolate("soil#iso=999"), ("soil", None));
    }

    /// An unselected class must be DIMMED, not erased: a reader still needs the rest
    /// of the map for context, which is why this desaturates rather than blanking.
    #[test]
    fn isolation_dims_without_erasing() {
        for code in 1..BIOME_COUNT as u8 {
            let c = biome_color(code);
            let d = dim_unselected(c);
            assert_eq!(d.0, d.1, "dimmed colour must be neutral");
            assert_eq!(d.1, d.2, "dimmed colour must be neutral");
            assert!(d.0 > 20, "biome {code} dims to {d:?} — too dark to give context");
            assert!(d.0 < 235, "biome {code} dims to {d:?} — too bright to recede");
        }
    }

    /// Thematic relief must stay a HINT. Above roughly a tenth it stops suggesting
    /// form and starts competing with the class colour it is modulating, which is
    /// the whole reason the terrain layer's own shading is not reused here.
    #[test]
    fn thematic_relief_is_gentle() {
        assert!(
            THEMATIC_RELIEF_AMP <= 0.10,
            "thematic relief {THEMATIC_RELIEF_AMP} would compete with the class colour",
        );
        // Flat ground must be EXACTLY unshaded, or every plate shifts tone wholesale
        // — the same normalisation bug the terrain layer carried for years.
        let flat = TileData::new_sea();
        let ctx = RenderCtx { grid_w: 3600, grid_h: 1800, ty: 4, step: 1, isolate: None };
        let k = thematic_relief(&flat, &TileNeighbors::default(), &ctx, 40, 40);
        assert!((k - 1.0).abs() < 1e-4, "flat ground shades to {k}, not 1.0");
    }

    /// Every biome the classifier can emit must have a distinct base colour, or
    /// two different ecologies render identically and the map lies.
    #[test]
    fn biome_colors_are_distinct() {
        // An EQUALITY check passes on colours a reader cannot tell apart: tropical
        // seasonal forest and temperate deciduous shipped at dE 3.0, thorn scrub and
        // chaparral at 3.8, and both pairs share a pattern kind, so texture did not
        // rescue them either. A perceptual floor is the check that has meaning.
        //
        // The floor guards the palette the app ALREADY satisfies rather than
        // encoding an aspiration — same discipline §2.5 uses for the economy bands.
        // Raise it as the palette earns it; the goal for two biomes sharing a
        // `Pattern` kind is dE >= 8.
        const MIN_DELTA_E: f32 = 2.5;
        let all: Vec<(u8, (u8, u8, u8))> =
            (1..BIOME_COUNT as u8).map(|b| (b, biome_color(b))).collect();
        for (i, &(a, ca)) in all.iter().enumerate() {
            for &(b, cb) in all.iter().skip(i + 1) {
                let d = delta_e(ca, cb);
                assert!(
                    d >= MIN_DELTA_E,
                    "biome {} ({}) and {} ({}) are indistinguishable: dE {:.2} ({:?} vs {:?})",
                    a, biome_name(a), b, biome_name(b), d, ca, cb,
                );
            }
        }
    }

    // ── Swatch-sheet generator (ignored: writes a PNG, run on demand) ──
    //
    //   cargo test --lib render::tile_image::tests::dump_biome_swatch_sheet \
    //       -- --ignored --nocapture
    //
    // Renders every biome through the REAL `render_tile` path — these are the
    // exact pixels the app draws, not an illustration. Output dir comes from
    // $BIOME_SHEET_DIR (default: the system temp dir).

    /// 3×5 bitmap digits, so each swatch can be labelled with its biome code
    /// without pulling in a font stack.
    const DIGITS: [[u8; 5]; 10] = [
        [0b111, 0b101, 0b101, 0b101, 0b111], // 0
        [0b010, 0b110, 0b010, 0b010, 0b111], // 1
        [0b111, 0b001, 0b111, 0b100, 0b111], // 2
        [0b111, 0b001, 0b111, 0b001, 0b111], // 3
        [0b101, 0b101, 0b111, 0b001, 0b001], // 4
        [0b111, 0b100, 0b111, 0b001, 0b111], // 5
        [0b111, 0b100, 0b111, 0b101, 0b111], // 6
        [0b111, 0b001, 0b010, 0b010, 0b010], // 7
        [0b111, 0b101, 0b111, 0b101, 0b111], // 8
        [0b111, 0b101, 0b111, 0b001, 0b111], // 9
    ];

    fn draw_code(sheet: &mut [u8], sw: usize, ox: usize, oy: usize, code: u8) {
        let text = code.to_string();
        const S: usize = 2; // pixel scale
        // Dark plate behind the digits so they read on pale biomes too.
        let plate_w = text.len() * 4 * S + 2 * S;
        for py in 0..(5 * S + 2 * S) {
            for px in 0..plate_w {
                let (x, y) = (ox + px, oy + py);
                let o = (y * sw + x) * 3;
                for c in 0..3 { sheet[o + c] = (sheet[o + c] as u16 * 25 / 100) as u8; }
            }
        }
        for (di, ch) in text.chars().enumerate() {
            let g = DIGITS[ch.to_digit(10).unwrap() as usize];
            for (ry, rowbits) in g.iter().enumerate() {
                for cx in 0..3 {
                    if rowbits & (1 << (2 - cx)) == 0 { continue; }
                    for sy in 0..S { for sx in 0..S {
                        let x = ox + S + di * 4 * S + cx * S + sx;
                        let y = oy + S + ry * S + sy;
                        let o = (y * sw + x) * 3;
                        sheet[o] = 255; sheet[o + 1] = 255; sheet[o + 2] = 255;
                    }}
                }
            }
        }
    }

    /// A tile whose every land cell carries one biome, at a plausible elevation.
    fn uniform_tile(biome: u8) -> TileData {
        let mut t = TileData::new_sea();
        for i in 0..PIXEL_COUNT {
            t.terrain[i] = 1;
            t.elevation[i] = 0.20; // neutral: the hypsometric lift is ~1.0 here
            t.biome[i] = biome;
        }
        t
    }

    #[test]
    #[ignore]
    fn dump_biome_swatch_sheet() {
        let dir = std::env::var("BIOME_SHEET_DIR")
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());

        // ── Sheet 1: every biome, grouped, one 128×128 tile each ──
        // Listed in classifier order so related ecologies sit together.
        let order: Vec<u8> = (1..BIOME_COUNT as u8).collect();
        const COLS: usize = 7;
        const CELL: usize = SIZE;   // one whole tile per swatch
        const GAP: usize = 6;
        let rows = order.len().div_ceil(COLS);
        let sw = COLS * CELL + (COLS + 1) * GAP;
        let sh = rows * CELL + (rows + 1) * GAP;
        let mut sheet = vec![18u8; sw * sh * 3]; // dark backing

        for (k, &b) in order.iter().enumerate() {
            let rgba = render_tile(&uniform_tile(b), "biomes");
            let cx = GAP + (k % COLS) * (CELL + GAP);
            let cy = GAP + (k / COLS) * (CELL + GAP);
            for y in 0..SIZE {
                for x in 0..SIZE {
                    let s = (y * SIZE + x) * 4;
                    let d = ((cy + y) * sw + cx + x) * 3;
                    sheet[d] = rgba[s];
                    sheet[d + 1] = rgba[s + 1];
                    sheet[d + 2] = rgba[s + 2];
                }
            }
            draw_code(&mut sheet, sw, cx + 3, cy + 3, b);
        }
        let p1 = format!("{dir}/biome_swatches.png");
        image::save_buffer(&p1, &sheet, sw as u32, sh as u32, image::ColorType::Rgb8).unwrap();
        println!("wrote {p1}  ({sw}×{sh})");

        // ── Sheet 2: seam proof — 2×2 tiles butted together, no gaps ──
        // Each block is FOUR separately-rendered tiles. If any pattern period did
        // not divide TILE_SIZE, a seam would appear down the middle of each block.
        let picks: [u8; 6] = [
            B_TROPICAL_RAINFOREST, // canopy blobs
            B_TAIGA,               // conifer spires
            B_FRESHWATER_MARSH,    // marsh dashes
            B_DUNE_SEA,            // dune ripples
            B_SALT_FLAT,           // cracked lattice
            B_GLACIER,             // crevasse lines
        ];
        let bw = SIZE * 2;
        let sw2 = picks.len() * bw + (picks.len() + 1) * GAP;
        let sh2 = bw + 2 * GAP;
        let mut seam = vec![18u8; sw2 * sh2 * 3];
        for (k, &b) in picks.iter().enumerate() {
            let rgba = render_tile(&uniform_tile(b), "biomes");
            let ox = GAP + k * (bw + GAP);
            for ty in 0..2 { for tx in 0..2 {
                for y in 0..SIZE { for x in 0..SIZE {
                    let s = (y * SIZE + x) * 4;
                    let dx = ox + tx * SIZE + x;
                    let dy = GAP + ty * SIZE + y;
                    let d = (dy * sw2 + dx) * 3;
                    seam[d] = rgba[s];
                    seam[d + 1] = rgba[s + 1];
                    seam[d + 2] = rgba[s + 2];
                }}
            }}
            draw_code(&mut seam, sw2, ox + 3, GAP + 3, b);
        }
        let p2 = format!("{dir}/biome_seams.png");
        image::save_buffer(&p2, &seam, sw2 as u32, sh2 as u32, image::ColorType::Rgb8).unwrap();
        println!("wrote {p2}  ({sw2}×{sh2})");
    }
}
