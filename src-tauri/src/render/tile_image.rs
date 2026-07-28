use crate::tile::cell::TileData;
use crate::tile::coords::TILE_SIZE;

const SIZE: usize = TILE_SIZE as usize;
const PIXEL_COUNT: usize = SIZE * SIZE;

/// Render a tile to RGBA pixel buffer for a given layer
pub fn render_tile(tile: &TileData, layer: &str) -> Vec<u8> {
    let mut rgba = vec![0u8; PIXEL_COUNT * 4];

    match layer {
        "land" => render_land(tile, &mut rgba),
        "elevation" => render_elevation(tile, &mut rgba),
        "climate" => render_climate(tile, &mut rgba),
        "temperature" => render_temperature(tile, &mut rgba),
        "sst" => render_sst(tile, &mut rgba),
        "snow" => render_snow(tile, &mut rgba),
        "precipitation" => render_precipitation(tile, &mut rgba),
        "soil" => render_soil(tile, &mut rgba),
        "fertility" => render_fertility(tile, &mut rgba),
        "plates" => render_plates(tile, &mut rgba),
        "biomes" => render_biomes(tile, &mut rgba),
        "fisheries" => render_fisheries(tile, &mut rgba),
        "terrain" => render_terrain_hillshade(tile, &mut rgba),
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
            // Sea: bathymetry coloring matching WF1
            let d = tile.sea_depth[i].clamp(0.0, 1.0);
            let (r, g, b) = if d < 0.10 {
                lerp_rgb((29, 120, 196), (26, 100, 180), d / 0.10)
            } else if d < 0.25 {
                lerp_rgb((26, 100, 180), (20, 74, 140), (d - 0.10) / 0.15)
            } else if d < 0.65 {
                lerp_rgb((20, 74, 140), (5, 15, 46), (d - 0.25) / 0.40)
            } else {
                lerp_rgb((5, 15, 46), (2, 5, 20), (d - 0.65) / 0.35)
            };
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
            let (r, g, b) = elevation_color(e);
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = 255;
        } else {
            // Sea depth coloring
            let d = tile.sea_depth[i].clamp(0.0, 1.0);
            rgba[offset] = (10.0 + d * 10.0) as u8;
            rgba[offset + 1] = (25.0 + d * 30.0) as u8;
            rgba[offset + 2] = (70.0 + d * 100.0) as u8;
            rgba[offset + 3] = 255;
        }
    }
}

fn elevation_color(e: f32) -> (u8, u8, u8) {
    // Green -> yellow -> brown -> white gradient
    if e < 0.15 {
        let t = e / 0.15;
        lerp_rgb((56, 118, 50), (86, 148, 60), t)
    } else if e < 0.35 {
        let t = (e - 0.15) / 0.2;
        lerp_rgb((86, 148, 60), (170, 160, 60), t)
    } else if e < 0.6 {
        let t = (e - 0.35) / 0.25;
        lerp_rgb((170, 160, 60), (140, 100, 50), t)
    } else if e < 0.85 {
        let t = (e - 0.6) / 0.25;
        lerp_rgb((140, 100, 50), (180, 170, 160), t)
    } else {
        let t = (e - 0.85) / 0.15;
        lerp_rgb((180, 170, 160), (255, 255, 255), t)
    }
}

fn render_climate(tile: &TileData, rgba: &mut [u8]) {
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
        let (r, g, b) = koppen_color(tile.koppen[i]);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn koppen_color(code: u8) -> (u8, u8, u8) {
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
        14 => (0, 255, 150),    // Dfa - hot-summer continental
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
        31 => (150, 50, 150),   // Dsd - dry-summer extreme subarctic
        32 => (185, 165, 185),  // H  - highland / alpine
        _ => (128, 128, 128),
    }
}

fn render_temperature(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        let t = ((tile.temperature[i] + 40.0) / 70.0).clamp(0.0, 1.0);
        let (r, g, b) = if t < 0.5 {
            lerp_rgb((0, 0, 200), (200, 200, 50), t * 2.0)
        } else {
            lerp_rgb((200, 200, 50), (200, 0, 0), (t - 0.5) * 2.0)
        };
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

/// Sea-surface temperature (ocean only): same blue→yellow→red ramp as the land
/// temperature layer, so warm/cold currents read at a glance. Land is transparent.
fn render_sst(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] != 0 || tile.sst.is_empty() {
            rgba[offset + 3] = 0;
            continue;
        }
        let t = ((tile.sst[i] + 5.0) / 40.0).clamp(0.0, 1.0);
        let (r, g, b) = if t < 0.5 {
            lerp_rgb((0, 0, 200), (200, 200, 50), t * 2.0)
        } else {
            lerp_rgb((200, 200, 50), (200, 0, 0), (t - 0.5) * 2.0)
        };
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

fn render_precipitation(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let p = (tile.precipitation[i] / 3000.0).clamp(0.0, 1.0);
        let (r, g, b) = lerp_rgb((200, 180, 100), (0, 50, 200), p);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn render_soil(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let (r, g, b) = soil_color(tile.soil_type[i]);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn soil_color(code: u8) -> (u8, u8, u8) {
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

fn render_fertility(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let f = tile.fertility[i].clamp(0.0, 1.0);
        let (r, g, b) = lerp_rgb((180, 150, 100), (0, 120, 0), f);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
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
fn render_biomes(tile: &TileData, rgba: &mut [u8]) {
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
        let (r, g, b_) = biome_color(b);

        let lx = (i % SIZE) as u32;
        let ly = (i / SIZE) as u32;
        // Pattern fill, plus a gentle hypsometric lift so relief still reads
        // through the flat ecological colours.
        let pattern = biome_pattern(b, lx, ly);
        let relief = 1.0 + (tile.elevation[i].clamp(0.0, 1.0) - 0.2) * 0.18;
        let k = (1.0 + pattern) * relief;

        rgba[offset] = (r as f32 * k).clamp(0.0, 255.0) as u8;
        rgba[offset + 1] = (g as f32 * k).clamp(0.0, 255.0) as u8;
        rgba[offset + 2] = (b_ as f32 * k).clamp(0.0, 255.0) as u8;
        rgba[offset + 3] = 255;
    }
}

/// Base colour per biome code. Greens darken with canopy closure, open country
/// runs olive→straw→tan with aridity, wetlands hold a blue-green cast and the
/// cryosphere is near-white, so the map reads correctly even in greyscale.
fn biome_color(b: u8) -> (u8, u8, u8) {
    use crate::sim::biome::*;
    match b {
        // Tropical forest
        B_TROPICAL_RAINFOREST => (13, 74, 31),
        B_TROPICAL_MOIST_FOREST => (28, 99, 44),
        B_TROPICAL_SEASONAL_FOREST => (76, 124, 48),
        B_CLOUD_FOREST => (46, 110, 92),
        B_MANGROVE => (44, 88, 74),
        // Tropical open
        B_SAVANNA => (168, 168, 84),
        B_THORN_SCRUB => (150, 138, 82),
        // Temperate forest
        B_TEMPERATE_RAINFOREST => (32, 92, 66),
        B_TEMPERATE_DECIDUOUS => (74, 122, 52),
        B_TEMPERATE_MIXED_FOREST => (62, 108, 60),
        B_TEMPERATE_CONIFER => (44, 86, 62),
        B_SUBTROPICAL_MOIST_FOREST => (58, 116, 54),
        // Mediterranean
        B_MEDITERRANEAN_WOODLAND => (124, 134, 70),
        B_CHAPARRAL => (146, 138, 88),
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

        // Scrub: isolated 1-px dots on a sparse 8×8 lattice.
        Pattern::Scrub => {
            if lx % 8 == 3 && ly % 8 == 3 && hash01(lx / 8, ly / 8, 23) > 0.45 { -0.13 } else { 0.02 }
        }

        // Stipple: fine sand/gravel grain — a low-density speck field.
        Pattern::Stipple => {
            let h = hash01(lx, ly, 37);
            if h > 0.93 { -0.10 } else if h < 0.06 { 0.06 } else { 0.0 }
        }

        // Dunes: sinusoidal crests running obliquely, as a real erg's do, with a
        // sharp lee slope and a soft stoss slope.
        Pattern::Dunes => {
            let t = ((lx as f32 * 0.55 + ly as f32 * 0.28) * std::f32::consts::TAU / 16.0).sin();
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
        // Bog: the marsh hatch over a stipple of peat hummocks.
        Pattern::Bog => {
            let d = marsh_dash(lx, ly);
            let h = if hash01(lx, ly, 71) > 0.88 { -0.06 } else { 0.0 };
            d + h
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

/// Broken horizontal dashes in offset rows — the wetland hatch shared by marsh
/// and bog.
fn marsh_dash(lx: u32, ly: u32) -> f32 {
    if ly % 4 != 1 {
        return 0.02;
    }
    let seg = (lx + if (ly / 4) % 2 == 1 { 4 } else { 0 }) % 8;
    if seg < 5 { -0.13 } else { 0.02 }
}

/// Deterministic hash in [0, 1) from two lattice coordinates and a salt. Pure
/// integer mixing — no allocation, no float noise field, and identical on every
/// platform.
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

fn render_terrain_hillshade(tile: &TileData, rgba: &mut [u8]) {
    // No neighbour tiles available (supertile / crop paths): fall back to
    // within-tile slopes. Edge pixels replicate, which is what produced the
    // faint tile-seam grid at LOD 0 — the neighbour-aware path below removes it.
    render_terrain_hillshade_halo(tile, &TileNeighbors::default(), rgba);
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

fn render_terrain_hillshade_halo(tile: &TileData, n: &TileNeighbors, rgba: &mut [u8]) {
    // Directional light from NW (azimuth 315, altitude 45 degrees)
    let az = 315.0_f32.to_radians();
    let alt = 45.0_f32.to_radians();
    let light_x = az.sin() * alt.cos();
    let light_y = -az.cos() * alt.cos(); // negative because y increases downward
    let light_z = alt.sin();

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = y * SIZE + x;
            let offset = i * 4;

            if tile.terrain[i] == 0 {
                // Sea: simple depth color
                let d = tile.sea_depth[i].clamp(0.0, 1.0);
                let (r, g, b) = lerp_rgb((29, 120, 196), (5, 15, 46), d);
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

            let dzdx = (right - left) * 50.0; // scale factor for visible relief
            let dzdy = (down - up) * 50.0;

            // Surface normal
            let len = (dzdx * dzdx + dzdy * dzdy + 1.0).sqrt();
            let nx = -dzdx / len;
            let ny = -dzdy / len;
            let nz = 1.0 / len;

            // Lambertian shading
            let shade = (nx * light_x + ny * light_y + nz * light_z).clamp(0.15, 1.0);

            // Base elevation color modulated by shade
            let (br, bg, bb) = elevation_color(e);
            rgba[offset] = (br as f32 * shade) as u8;
            rgba[offset + 1] = (bg as f32 * shade) as u8;
            rgba[offset + 2] = (bb as f32 * shade) as u8;
            rgba[offset + 3] = 255;
        }
    }
}

/// Like `render_tile`, but for the "terrain" hillshade layer it uses the given
/// cardinal neighbour tiles to compute seam-free slopes. All other layers ignore
/// the neighbours and render identically to `render_tile`.
pub fn render_tile_with_neighbors(tile: &TileData, layer: &str, n: &TileNeighbors) -> Vec<u8> {
    if layer == "terrain" {
        let mut rgba = vec![0u8; PIXEL_COUNT * 4];
        render_terrain_hillshade_halo(tile, n, &mut rgba);
        rgba
    } else {
        render_tile(tile, layer)
    }
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
        let vx = tile.current_vx[i];
        let vy = tile.current_vy[i];
        let mag = (vx * vx + vy * vy).sqrt().clamp(0.0, 3.0) / 3.0;
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
