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

fn render_biomes(tile: &TileData, rgba: &mut [u8]) {
    for i in 0..PIXEL_COUNT {
        let offset = i * 4;
        if tile.terrain[i] == 0 {
            rgba[offset + 3] = 0;
            continue;
        }
        let (r, g, b) = biome_color(tile.koppen[i], tile.elevation[i]);
        rgba[offset] = r;
        rgba[offset + 1] = g;
        rgba[offset + 2] = b;
        rgba[offset + 3] = 255;
    }
}

fn biome_color(koppen: u8, elevation: f32) -> (u8, u8, u8) {
    // High elevation overrides.
    // Elevation is normalized 0-1 over 8848m, so treeline (~3500m) ≈ 0.40 and
    // permanent snow / nival (~5500m) ≈ 0.62. The previous 0.55/0.75 thresholds
    // (≈4900m/6600m) almost never triggered, so mountains never showed up.
    if elevation > 0.62 { return (220, 220, 230); } // alpine / nival (snow)
    if elevation > 0.40 { return (160, 170, 150); } // montane (above treeline)

    match koppen {
        1 => (0, 80, 20),        // Af - tropical rainforest (dark green)
        2 => (20, 100, 40),      // Am - tropical monsoon forest
        3 => (140, 170, 60),     // Aw - tropical savanna (yellow-green)
        4 => (210, 190, 130),    // BWh - hot desert (tan)
        5 => (190, 175, 140),    // BWk - cold desert
        6 => (185, 175, 100),    // BSh - hot steppe (khaki)
        7 => (170, 165, 110),    // BSk - cold steppe
        8 | 9 | 10 => (120, 150, 60),  // Cs - Mediterranean scrubland
        11 => (60, 120, 50),     // Cfa - subtropical forest
        12 => (50, 110, 60),     // Cfb - temperate deciduous forest
        13 => (40, 90, 55),      // Cfc - cool temperate forest
        14 | 15 => (50, 100, 50), // Dfa/Dfb - mixed forest
        16 | 17 => (30, 70, 50), // Dfc/Dfd - taiga / boreal forest (dark teal)
        18 | 19 | 20 => (80, 110, 70), // Ds - dry continental
        21 => (180, 190, 180),   // ET - tundra (grey-green)
        22 => (230, 235, 240),   // EF - ice cap (near white)
        23 => (150, 175, 70),    // As - savanna (dry summer)
        24 => (60, 120, 50),     // Cwa - subtropical forest
        25 => (55, 115, 60),     // Cwb - highland forest
        26 => (45, 95, 58),      // Cwc - cool highland forest
        27 => (55, 100, 50),     // Dwa - mixed forest
        28 => (45, 90, 52),      // Dwb - mixed forest
        29 => (32, 72, 50),      // Dwc - taiga
        30 => (28, 60, 46),      // Dwd - taiga
        31 => (80, 110, 70),     // Dsd - dry continental
        32 => (205, 205, 215),   // H  - alpine
        _ => (100, 120, 80),
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
            // Land: dim so the sea field reads clearly.
            rgba[offset] = 18;
            rgba[offset + 1] = 22;
            rgba[offset + 2] = 26;
            rgba[offset + 3] = 255;
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
