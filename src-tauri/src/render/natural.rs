//! NATURAL COLOUR — the land-cover palette, in the Blue Marble / Natural Earth II
//! tradition: land is coloured by WHAT GROWS ON IT, not by how high it is.
//!
//! This is a second, independent table and deliberately **not**
//! `tile_image::biome_color`. That one is a THEMATIC palette: `biome_colors_are_
//! distinct` holds every pair of its 41 classes apart at a CIELAB floor so a reader
//! can tell savanna from thorn scrub in a legend. Natural colour wants the exact
//! opposite — real vegetation grades continuously, and a palette engineered for
//! class separation renders a satellite-style map as a discretised poster with a
//! visible edge at every biome boundary.
//!
//! Three rules follow from that and are what the table is built to satisfy:
//!
//! 1. **Neighbouring classes are deliberately CLOSE.** Temperate deciduous, mixed
//!    and subtropical moist forest sit within a few ΔE of each other here. Do not
//!    "fix" that by separating them — the separated version already exists next
//!    door, and reaching for it is how this layer stops looking like a photograph.
//! 2. **Class edges are SOFTENED by continuous fields.** `natural_color` blends
//!    every class colour toward a continuous temperature/precipitation tint, the
//!    same trick (and the same reasoning) as `lowland_tint`: Köppen and biome are
//!    both discretisations of continuous fields, so re-mixing a little of the
//!    continuous form back in dissolves the staircase without losing the identity.
//! 3. **Height overrides vegetation at the top, never at the bottom.** Above the
//!    rock line a summit reads as rock and snow whatever biome it is nominally in
//!    — which is what the eye expects and what makes two ranges comparable.
//!
//! Kept in its own module (rather than beside `biome_color`) so the two tables
//! cannot be casually edited as if they were one thing.

use crate::sim::biome::*;

/// Land-cover colour per biome code. Muted and overlapping BY DESIGN — see the
/// module header before separating any pair of these.
pub fn natural_land(b: u8) -> (u8, u8, u8) {
    match b {
        // ── Tropical forest: the darkest, most saturated greens on the map.
        B_TROPICAL_RAINFOREST => (26, 62, 24),
        B_TROPICAL_MOIST_FOREST => (36, 72, 30),
        B_TROPICAL_SEASONAL_FOREST => (62, 88, 38),
        B_CLOUD_FOREST => (40, 74, 52),
        B_MANGROVE => (44, 74, 48),

        // ── Tropical open: the savanna/scrub band that reads gold from orbit.
        B_SAVANNA => (140, 130, 70),
        B_THORN_SCRUB => (152, 128, 78),

        // ── Temperate forest: mid greens, close together on purpose.
        B_TEMPERATE_RAINFOREST => (42, 80, 48),
        B_TEMPERATE_DECIDUOUS => (72, 94, 48),
        B_TEMPERATE_MIXED_FOREST => (64, 88, 48),
        B_SUBTROPICAL_MOIST_FOREST => (58, 88, 42),
        B_TEMPERATE_CONIFER => (50, 78, 50),

        // ── Mediterranean: the dry-summer olive band.
        B_MEDITERRANEAN_WOODLAND => (108, 112, 64),
        B_CHAPARRAL => (124, 120, 76),

        // ── Boreal: dark, slightly blue-green, and very large areas — this is the
        // band that gives a northern continent its colour at world zoom.
        B_TAIGA => (54, 72, 50),
        B_FOREST_TUNDRA => (82, 92, 68),

        // ── Grassland.
        B_TALLGRASS_PRAIRIE => (134, 136, 74),
        B_SHORTGRASS_STEPPE => (158, 148, 92),
        B_MONTANE_GRASSLAND => (124, 126, 80),

        // ── Alpine: vegetation thins to rock; these are already half mineral.
        B_ALPINE_MEADOW => (110, 126, 86),
        B_ALPINE_TUNDRA => (136, 134, 116),
        B_ALPINE_DESERT => (154, 144, 128),
        B_NIVAL => (198, 200, 202),

        // ── Desert: the widest colour spread on the map, because real deserts
        // differ far more from each other than real forests do. A dune sea is
        // iron-stained orange (the Simpson, the Rub' al Khali), a rocky hamada is
        // grey-brown, and a salt pan is blinding.
        B_HOT_DESERT => (206, 180, 132),
        B_DUNE_SEA => (198, 140, 84),
        B_ROCKY_DESERT => (166, 142, 112),
        B_SEMI_DESERT => (184, 166, 118),
        B_COLD_DESERT => (172, 160, 134),
        B_COASTAL_FOG_DESERT => (176, 168, 152),
        B_SALT_FLAT => (226, 222, 214),

        // ── Polar.
        B_TUNDRA => (132, 132, 112),
        B_POLAR_DESERT => (176, 178, 176),
        B_ICE_SHEET => (242, 246, 250),
        B_GLACIER => (222, 232, 242),

        // ── Wetland & riparian: wetter than their surroundings, so DARKER and
        // greener than whatever they cut through. That contrast is the whole
        // reason a river valley is visible from orbit in an arid belt.
        B_PEAT_BOG => (84, 88, 64),
        B_SWAMP_FOREST => (46, 78, 52),
        B_GALLERY_FOREST => (54, 88, 46),
        B_FRESHWATER_MARSH => (82, 104, 74),
        B_FLOODPLAIN_GRASSLAND => (100, 118, 64),
        B_SALT_MARSH => (110, 118, 88),
        B_OASIS => (56, 100, 52),

        _ => (110, 114, 86),
    }
}

/// Continuous climate tint, used to SOFTEN class edges (rule 2 in the module
/// header) and as the whole answer for a world generated before phase 6b, where
/// the `biome` column pads to zero.
///
/// Deliberately the same two axes as `tile_image::lowland_tint` — temperature and
/// precipitation, both continuous, never the categorical Köppen code.
pub fn climate_tint(temp_c: f32, precip_mm: f32) -> (u8, u8, u8) {
    let humid = (precip_mm / 1200.0).clamp(0.0, 1.0).sqrt();
    let warm = ((temp_c + 5.0) / 30.0).clamp(0.0, 1.0);

    let mix = |a: (f32, f32, f32), b: (f32, f32, f32), t: f32| {
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
    };
    let cold_dry = (146.0, 144.0, 124.0);
    let cold_wet = (66.0, 84.0, 60.0);
    let hot_dry = (204.0, 180.0, 132.0);
    let hot_wet = (44.0, 78.0, 32.0);

    let dry = mix(cold_dry, hot_dry, warm);
    let wet = mix(cold_wet, hot_wet, warm);
    let c = mix(dry, wet, humid);
    (c.0 as u8, c.1 as u8, c.2 as u8)
}

/// How much continuous climate tint is mixed back into a class colour. Enough to
/// dissolve the staircase at a biome boundary, not enough to lose the class.
const EDGE_SOFTEN: f32 = 0.28;

/// Elevation (m) at which bare rock starts showing through the vegetation, and the
/// span over which it takes over completely. Above this the map reads MINERAL —
/// the same reasoning as `LOWLAND_TINT_CEILING_M`, at the other end of the ramp.
const ROCK_START_M: f32 = 2200.0;
const ROCK_FULL_M: f32 = 4200.0;

/// Bare high-mountain rock. Warm grey, not neutral — a neutral grey up here reads
/// as missing data against a map with this much colour in it.
const ROCK: (u8, u8, u8) = (150, 140, 128);

/// Permanent snow and ice. Very slightly blue, because snow in shadow is blue and a
/// pure white here leaves nothing for the hillshade to darken.
const SNOW: (u8, u8, u8) = (246, 249, 252);

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t) as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t) as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t) as u8,
    )
}

/// Highest elevation the normalised `elevation` column maps to. Mirrors
/// `tile_image::ELEV_MAX_M`; kept local so this module is a pure palette with no
/// dependency on the renderer.
const ELEV_MAX_M: f32 = 8848.0;

/// The colour a land cell renders in natural-colour view.
///
/// `biome` 0 (a world generated before phase 6b, or one where biomes were never
/// run) falls back to the continuous climate tint alone rather than to grey — the
/// same discipline as `koppen_fallback_biome`: this layer must never be blank.
pub fn natural_color(
    biome: u8,
    elev: f32,
    temp_c: f32,
    precip_mm: f32,
    snow_frac: u8,
) -> (u8, u8, u8) {
    let tint = climate_tint(temp_c, precip_mm);
    let base = if biome == B_NONE {
        tint
    } else {
        lerp(natural_land(biome), tint, EDGE_SOFTEN)
    };

    let m = elev.clamp(0.0, 1.0) * ELEV_MAX_M;

    // Rock shows through with height. Ice and rock biomes are already mineral, so
    // they are exempt — pulling an ice sheet toward brown rock would be nonsense.
    let mineral = matches!(biome, B_ICE_SHEET | B_GLACIER | B_NIVAL | B_SALT_FLAT);
    let c = if mineral {
        base
    } else {
        let t = ((m - ROCK_START_M) / (ROCK_FULL_M - ROCK_START_M)).clamp(0.0, 1.0);
        lerp(base, ROCK, t)
    };

    // Snow is the LAST word, and it comes from the modelled snow-cover fraction
    // rather than from an elevation cutoff — which is what puts the snowline at sea
    // level in the Arctic and near 5000 m in the tropics without a latitude term.
    let s = snow_frac as f32 / 255.0;
    if s <= 0.02 {
        return c;
    }
    // Partial cover is patchy, not a uniform wash: square-root it so a half-covered
    // slope already reads clearly white, the way it does from orbit.
    lerp(c, SNOW, s.sqrt() * 0.92)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole premise of this module: it must NOT be the thematic palette. If
    /// these two tables ever converge, one of them is doing the other's job.
    #[test]
    fn natural_is_a_different_palette_from_the_thematic_one() {
        let mut differ = 0;
        for b in 1..BIOME_COUNT as u8 {
            if natural_land(b) != crate::render::tile_image::biome_color(b) {
                differ += 1;
            }
        }
        assert!(
            differ > BIOME_COUNT / 2,
            "natural colour has collapsed onto the thematic palette ({differ} of \
             {BIOME_COUNT} differ) — see the module header"
        );
    }

    /// Rule 1, asserted rather than merely documented. Forest classes that a
    /// THEMATIC palette must hold apart are close here on purpose, so a
    /// well-meaning future edit that "improves contrast" fails loudly.
    #[test]
    fn neighbouring_vegetation_classes_stay_close() {
        // Crude but sufficient: max channel distance, in 0..255.
        let near = |a: (u8, u8, u8), b: (u8, u8, u8)| {
            let d = (a.0 as i32 - b.0 as i32)
                .abs()
                .max((a.1 as i32 - b.1 as i32).abs())
                .max((a.2 as i32 - b.2 as i32).abs());
            d
        };
        for &(x, y) in &[
            (B_TEMPERATE_DECIDUOUS, B_TEMPERATE_MIXED_FOREST),
            (B_TEMPERATE_MIXED_FOREST, B_SUBTROPICAL_MOIST_FOREST),
            (B_TROPICAL_RAINFOREST, B_TROPICAL_MOIST_FOREST),
            (B_SAVANNA, B_THORN_SCRUB),
        ] {
            let d = near(natural_land(x), natural_land(y));
            assert!(d <= 24, "biomes {x} and {y} are {d} apart — too separated for a \
                              natural-colour palette (see rule 1)");
        }
    }

    /// A world with no biome column must still render as a place, never as grey.
    #[test]
    fn a_world_without_biomes_still_gets_a_colour() {
        let hot_wet = natural_color(B_NONE, 0.02, 27.0, 2400.0, 0);
        let hot_dry = natural_color(B_NONE, 0.02, 27.0, 40.0, 0);
        assert!(hot_wet.1 > hot_wet.0, "wet tropics must read green");
        assert!(hot_dry.0 > hot_dry.2, "hot desert must read warm/tan");
        assert_ne!(hot_wet, hot_dry);
    }

    /// Snow comes from the modelled fraction, so the snowline is latitude-aware
    /// for free. A low ARCTIC cell must be able to read whiter than a high
    /// TROPICAL one — the exact case an elevation cutoff gets backwards.
    #[test]
    fn snow_follows_the_modelled_fraction_not_elevation() {
        let arctic_lowland = natural_color(B_TUNDRA, 0.02, -18.0, 200.0, 250);
        let tropical_highland = natural_color(B_MONTANE_GRASSLAND, 0.45, 12.0, 900.0, 0);
        assert!(
            arctic_lowland.0 > tropical_highland.0 + 40,
            "a snow-covered arctic lowland must read whiter than a bare tropical \
             highland ({arctic_lowland:?} vs {tropical_highland:?})"
        );
    }

    /// Height must take over at the TOP of the ramp only. Two different biomes at
    /// 4200 m are both rock; the same two at sea level are not.
    #[test]
    fn height_overrides_vegetation_only_at_the_top() {
        let hi_a = natural_color(B_TEMPERATE_DECIDUOUS, ROCK_FULL_M / ELEV_MAX_M, 4.0, 800.0, 0);
        let hi_b = natural_color(B_SHORTGRASS_STEPPE, ROCK_FULL_M / ELEV_MAX_M, 4.0, 800.0, 0);
        assert_eq!(hi_a, hi_b, "at the rock line every biome must read as rock");

        let lo_a = natural_color(B_TEMPERATE_DECIDUOUS, 0.0, 4.0, 800.0, 0);
        let lo_b = natural_color(B_SHORTGRASS_STEPPE, 0.0, 4.0, 800.0, 0);
        assert_ne!(lo_a, lo_b, "at sea level vegetation must still decide the colour");
    }

    // ── VISUAL PROOF ───────────────────────────────────────────────────────────
    //
    //   NATURAL_SHEET_DIR=/tmp/n cargo test --lib dump_natural_sheet \
    //       -- --ignored --nocapture
    //
    // Builds a REAL procedural world (plates through biomes, the same call order
    // `sim_run_all` uses — rule 11) and renders it through the REAL `render_tile`
    // path, so these are the exact pixels the app draws rather than an
    // illustration. Same discipline as `dump_biome_swatch_sheet`: look at a
    // palette change instead of arguing about it.
    #[test]
    #[ignore]
    fn dump_natural_sheet() {
        use crate::db::schema;
        use crate::render::tile_image::{render_tile_with_neighbors_ctx, RenderCtx, TileNeighbors};
        use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
        use crate::sim::*;
        use crate::tile::cell::TileData;
        use crate::tile::coords::TILE_SIZE;
        use rusqlite::Connection;

        const W: u32 = 1440;
        const H: u32 = 720;
        let seed = 20260818u64;

        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", W.to_string()), ("grid_height", H.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();

        plates::generate_plates_and_landmass(&mut buf, seed, 14);
        elevation::generate_elevation(&mut buf, seed);
        elevation::compute_sea_depth(&mut buf);
        elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);

        ocean::compute_wind_belts(&mut buf);
        ocean::compute_salinity(&mut buf);
        ocean::generate_ocean_currents(&mut buf);
        ocean::advect_salinity_and_recouple(&mut buf);
        ocean::compute_sst(&mut buf);
        ocean::compute_distance_to_ocean(&mut buf);
        let sea_freeze = ocean::compute_shelf_freeze(&buf);
        ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
        temperature::compute_temperature(&mut buf);
        ocean::compute_upwelling_zones(&mut buf);
        ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
        temperature::compute_seasonal_amplitude(&mut buf);
        temperature::apply_ice_albedo_feedback(&mut buf);
        jets::compute_low_level_jets(&mut buf);
        precipitation::compute_precipitation(&mut buf);
        koppen::classify_koppen(&mut buf);

        let hydro = rivers::compute_hydrology(&buf);
        let lake_max = (buf.total() / 2000).max(20);
        let mut lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);
        let rv = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0, &lakes);
        rivers::classify_salt_lakes(&buf, &mut lakes, &rv);
        soil::classify_soil(&mut buf);
        fertility::compute_fertility(&mut buf, &rv);
        biome::classify_biomes(&mut buf, &rv, &lakes);

        // ── Cut the buffer into tiles, exactly as the tile store would ──
        let ts = TILE_SIZE as usize;
        let tw = (W as usize).div_ceil(ts);
        let th = (H as usize).div_ceil(ts);
        let mut tiles: Vec<TileData> = Vec::with_capacity(tw * th);
        for ty in 0..th {
            for tx in 0..tw {
                let mut t = TileData::new_sea();
                for ly in 0..ts {
                    for lx in 0..ts {
                        let gx = tx * ts + lx;
                        let gy = ty * ts + ly;
                        if gx >= W as usize || gy >= H as usize { continue; }
                        let g = gy * W as usize + gx;
                        let l = ly * ts + lx;
                        t.terrain[l] = buf.terrain[g];
                        t.elevation[l] = buf.elevation[g];
                        t.sea_depth[l] = buf.sea_depth[g];
                        t.temperature[l] = buf.temperature[g];
                        t.precipitation[l] = buf.precipitation[g];
                        t.koppen[l] = buf.koppen[g];
                        t.biome[l] = buf.biome[g];
                        t.snow_frac[l] = buf.snow_frac[g];
                    }
                }
                tiles.push(t);
            }
        }

        let dir = std::env::var("NATURAL_SHEET_DIR")
            .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
        std::fs::create_dir_all(&dir).ok();

        for layer in ["natural", "terrain"] {
            let mut img = vec![0u8; W as usize * H as usize * 3];
            for ty in 0..th {
                for tx in 0..tw {
                    let at = |x: isize, y: isize| -> Option<&TileData> {
                        if x < 0 || y < 0 || x >= tw as isize || y >= th as isize { return None; }
                        tiles.get(y as usize * tw + x as usize)
                    };
                    let n = TileNeighbors {
                        west: at(tx as isize - 1, ty as isize),
                        east: at(tx as isize + 1, ty as isize),
                        north: at(tx as isize, ty as isize - 1),
                        south: at(tx as isize, ty as isize + 1),
                    };
                    let ctx = RenderCtx {
                        grid_w: W, grid_h: H,
                        tx: tx as i32, ty: ty as i32,
                        step: 1, isolate: None,
                    };
                    let rgba = render_tile_with_neighbors_ctx(&tiles[ty * tw + tx], layer, &n, &ctx);
                    for ly in 0..ts {
                        for lx in 0..ts {
                            let gx = tx * ts + lx;
                            let gy = ty * ts + ly;
                            if gx >= W as usize || gy >= H as usize { continue; }
                            let src = (ly * ts + lx) * 4;
                            let dst = (gy * W as usize + gx) * 3;
                            img[dst] = rgba[src];
                            img[dst + 1] = rgba[src + 1];
                            img[dst + 2] = rgba[src + 2];
                        }
                    }
                }
            }
            let tag = std::env::var("SHEET_TAG").unwrap_or_default();
            let path = format!("{dir}/world_{layer}{tag}.png");
            image::save_buffer(&path, &img, W, H, image::ColorType::Rgb8).unwrap();
            println!("wrote {path}");

            // A 3x-magnified crop of the most mountainous window, because a whole
            // world at 1440px cannot show what a hillshade is doing.
            let (cw, ch) = (240usize, 160usize);
            let mut best = (0usize, 0usize, -1.0f32);
            for oy in (0..H as usize - ch).step_by(40) {
                for ox in (0..W as usize - cw).step_by(40) {
                    let mut score = 0.0f32;
                    for y in (oy..oy + ch).step_by(3) {
                        for x in (ox..ox + cw).step_by(3) {
                            let g = y * W as usize + x;
                            if buf.terrain[g] == 1 { score += buf.elevation[g]; }
                        }
                    }
                    if score > best.2 { best = (ox, oy, score); }
                }
            }
            const M: usize = 3;
            let mut crop = vec![0u8; cw * M * ch * M * 3];
            for y in 0..ch * M {
                for x in 0..cw * M {
                    let src = ((best.1 + y / M) * W as usize + (best.0 + x / M)) * 3;
                    let dst = (y * cw * M + x) * 3;
                    crop[dst..dst + 3].copy_from_slice(&img[src..src + 3]);
                }
            }
            let cp = format!("{dir}/crop_{layer}{tag}.png");
            image::save_buffer(&cp, &crop, (cw * M) as u32, (ch * M) as u32, image::ColorType::Rgb8).unwrap();
            println!("wrote {cp} (from {},{})", best.0, best.1);
        }

        // A quick census, so the run says something even without opening the PNGs.
        let mut counts = std::collections::BTreeMap::new();
        let mut land = 0usize;
        for i in 0..buf.total() {
            if buf.terrain[i] == 1 {
                land += 1;
                *counts.entry(buf.biome[i]).or_insert(0usize) += 1;
            }
        }
        println!("land cells: {land}");
        for (b, c) in counts.iter() {
            println!("  biome {:>3} {:>7} ({:>5.2}%)  rgb{:?}",
                     b, c, 100.0 * *c as f32 / land as f32, natural_land(*b));
        }
    }
}
