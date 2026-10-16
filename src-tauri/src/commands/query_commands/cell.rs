//! cell commands — split from the former monolithic query_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;

/// One cause in the "Why?" inspector's chain (GENERATOR_AND_GOODS_REDESIGN.md §2.0).
/// `effect` is the direction it pushes the cell: −2 strongly drier/colder/worse …
/// +2 strongly wetter/milder/better; 0 = context only.
#[derive(Debug, Clone, Serialize)]
pub struct ExplainFactor {
    pub topic: String,   // "climate" | "water" | "land" | "sea" | "people"
    pub label: String,
    pub effect: i8,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CellExplanation {
    pub lat: f32,
    pub is_land: bool,
    pub factors: Vec<ExplainFactor>,
}

/// Which circulation belt a latitude sits in, on THIS planet (belt edges come
/// from `Circulation`, so a fast rotator's narrower belts read correctly).
pub(crate) fn belt_reading(alat: f32, he: f32, pf: f32) -> (&'static str, i8, String) {
    if alat < he * 0.35 {
        ("Equatorial rain belt", 2, "near the equator, where the trade winds meet and rise (the ITCZ). Rain most of the year.".into())
    } else if alat < he - 7.0 {
        ("Trade-wind belt", 0, "steady easterly trade winds. Rainy where they blow in from the sea, dry behind mountains.".into())
    } else if alat < he + 7.0 {
        ("Subtropical high", -2, format!("air sinks here (around {he:.0}° on this planet), which stops clouds forming. This is the planet's desert belt."))
    } else if alat < pf - 4.0 {
        ("Westerlies and storm track", 1, "westerly winds carry storms in from the ocean, so west-facing coasts are wet.".into())
    } else if alat < pf + 8.0 {
        ("Polar front", 1, format!("cold polar air meets the westerlies (around {pf:.0}° here). Cloudy and stormy."))
    } else {
        ("Polar belt", -1, "cold, dry polar air. Little snow falls, but little melts.".into())
    }
}

/// "Why is this cell like this?" — a short causal chain read from data the
/// pipeline already stored. Read-only; never recomputes a phase. The upwind walk
/// follows the cell's OWN wind direction in a straight line.
/// shortcut: straight-line upwind walk, not a trajectory through the wind field;
/// upgrade if the inspector is asked about curved monsoon/jet paths.
#[tauri::command]
pub fn explain_cell(wx: u32, wy: u32, db: State<'_, WorldDb>) -> Result<CellExplanation, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let meta_u32 = |k: &str, d: u32| metadata::get_meta(&conn, k).ok().flatten()
        .and_then(|s| s.parse().ok()).unwrap_or(d);
    let grid_w = meta_u32("grid_width", 3600).max(1);
    let grid_h = meta_u32("grid_height", 1800).max(1);
    let (eq_off, lat_scale, lat_ratio, _obl) = crate::commands::world_commands::read_lat_config(&conn);
    let planet = crate::commands::world_commands::read_planet_config(&conn);
    let circ = crate::sim::circulation::Circulation::from_params(planet.rotation_rate, planet.solar_lum, planet.greenhouse);
    let km_cell = 40075.0 / grid_w as f32;

    let load = |x: u32, y: u32| -> Result<(TileCoord, TileData), String> {
        let tc = TileCoord::from_world(x, y);
        let t = tile_store::load_tile(&conn, tc.tx, tc.ty, 0).map_err(|e| e.to_string())?
            .unwrap_or_else(TileData::new_sea);
        Ok((tc, t))
    };
    // Upwind walker: (is_land, elevation, current_type, sst) per cell, tiles cached.
    let mut tiles: std::collections::HashMap<(i32, i32), TileData> = std::collections::HashMap::new();
    let mut cell = |x: u32, y: u32| -> Result<(bool, f32, u8, f32), String> {
        let tc = TileCoord::from_world(x, y);
        if !tiles.contains_key(&(tc.tx, tc.ty)) {
            let (_, t) = load(x, y)?;
            tiles.insert((tc.tx, tc.ty), t);
        }
        let (lx, ly) = TileCoord::local(x, y);
        let t = &tiles[&(tc.tx, tc.ty)];
        let k = (ly * TILE_SIZE + lx) as usize;
        Ok((t.terrain[k] == 1, t.elevation[k], t.current_type[k], t.sst[k]))
    };

    let (_, t) = load(wx, wy)?;
    let i = { let (lx, ly) = TileCoord::local(wx, wy); (ly * TILE_SIZE + lx) as usize };
    let lat = crate::sim::world_buffer::lat_from_y(wy as f32 + 0.5, grid_h as f32, eq_off, lat_scale, lat_ratio);
    let alat = lat.abs();
    let is_land = t.terrain[i] == 1;
    let mut f: Vec<ExplainFactor> = Vec::new();
    let mut add = |topic: &str, label: &str, effect: i8, text: String| f.push(ExplainFactor {
        topic: topic.into(), label: label.into(), effect, text });

    // ── Where on the planet: the circulation belt ──
    let (he, pf) = (circ.hadley_edge, circ.polar_front);
    let hemi = if lat >= 0.0 { "N" } else { "S" };
    let (label, effect, text) = belt_reading(alat, he, pf);
    add("climate", label, effect, format!("{alat:.0}°{hemi}: {text}"));

    if is_land {
        let elev_m = t.elevation[i] * 8848.0;
        let temp = t.temperature[i];
        let amp = t.seasonal_amp[i] as f32 / 4.0;
        let precip = t.precipitation[i];
        add("climate", "Temperature", 0, if amp > 0.5 {
            format!("Averages {temp:.0} °C: about {:.0} °C in summer and {:.0} °C in winter.", temp + amp, temp - amp)
        } else { format!("Averages {temp:.0} °C over the year.") });
        if elev_m > 1200.0 {
            add("land", "Altitude", -1, format!("{elev_m:.0} m up: the air is about {:.0} °C colder than at sea level.", elev_m * 0.0065));
        }
        let sf = t.precip_summer_frac[i] as f32 / 255.0;
        let when = if sf > 0.68 { "mostly in summer (a monsoon pattern)" } else if sf < 0.32 { "mostly in winter (a Mediterranean pattern)" } else { "through the whole year" };
        add("climate", "Rainfall", if precip < 250.0 { -2 } else if precip < 500.0 { -1 } else if precip > 1500.0 { 2 } else { 1 },
            format!("{precip:.0} mm a year, falling {when}."));

        let d_cells = t.distance_to_ocean[i] * 60.0;
        if d_cells >= 59.5 {
            add("climate", "Deep inland", -1, format!("More than {:.0} km from the sea: hotter summers, colder winters, less rain.", 60.0 * km_cell));
        } else if d_cells > 12.0 {
            add("climate", "Inland", 0, format!("About {:.0} km from the sea.", d_cells * km_cell));
        } else {
            add("climate", "Near the coast", 1, "The sea keeps summers and winters mild.".to_string());
        }

        // ── Upwind: rain shadow and what lies offshore ──
        let (vx, vy) = (t.wind_vx[i], t.wind_vy[i]);
        let m = (vx * vx + vy * vy).sqrt();
        if m > 1e-3 {
            let (ux, uy) = (-vx / m, -vy / m);
            let max_steps = (1800.0 / km_cell).ceil() as i32;
            let (mut px, mut py) = (wx as f32 + 0.5, wy as f32 + 0.5);
            let mut peak = (t.elevation[i], 0i32);
            let mut coast: Option<(i32, u8, f32)> = None;
            for s in 1..=max_steps {
                px += ux; py += uy;
                if py < 0.0 || py >= grid_h as f32 { break; }
                let cx = (px.floor() as i64).rem_euclid(grid_w as i64) as u32;
                let (land, el, ctype, sst) = cell(cx, py.floor() as u32)?;
                if !land { coast = Some((s, ctype, sst)); break; }
                if el > peak.0 { peak = (el, s); }
            }
            let rise_m = (peak.0 - t.elevation[i]) * 8848.0;
            if rise_m > 1200.0 && peak.1 > 0 {
                add("climate", "Rain shadow", -2, format!("Mountains about {:.0} m high stand {:.0} km upwind. The wind drops its rain on the far side before it gets here.",
                    peak.0 * 8848.0, peak.1 as f32 * km_cell));
            }
            match coast {
                Some((s, ctype, sst)) => {
                    let km = s as f32 * km_cell;
                    match ctype {
                        2 => add("climate", "Cold current offshore", -2, format!("The wind blows in over a cold current {km:.0} km away (sea {sst:.0} °C). Cold water chills the air, so it brings fog but little rain.")),
                        1 => add("climate", "Warm current offshore", 1, format!("The wind blows in over a warm current {km:.0} km away (sea {sst:.0} °C), bringing mild, moist air.")),
                        _ => add("climate", "Wind from the sea", 1, format!("The prevailing wind reaches here from the sea {km:.0} km away, carrying moisture.")),
                    }
                }
                None => add("climate", "Wind from inland", -1, format!("The prevailing wind crosses more than {:.0} km of land before it arrives, so it has lost most of its moisture.", max_steps as f32 * km_cell)),
            }
        }
        if t.snow_frac[i] as f32 / 255.0 > 0.3 {
            add("climate", "Snow", -1, format!("Snow covers the ground about {:.0}% of the year.", t.snow_frac[i] as f32 / 2.55));
        }

        // ── Life and people ──
        add("land", "Fertility", if t.fertility[i] > 0.6 { 2 } else if t.fertility[i] > 0.35 { 1 } else if t.fertility[i] < 0.15 { -2 } else { -1 },
            format!("Farmland quality {:.0}%.", t.fertility[i] * 100.0));
        let dis = t.disease_risk[i] as f32 / 255.0;
        if dis > 0.35 {
            add("people", "Fever country", -2, format!("Warm, wet lowland with fever (malaria) risk {:.0}%. People settle here, but cities stay small and deadly.", dis * 100.0));
        }
        let hab = t.habitability[i];
        if hab > 0.0 {
            add("people", "Habitability", if hab > 0.6 { 2 } else if hab > 0.35 { 1 } else { -1 },
                format!("{:.0}% as a place to live, combining water, soil, climate and health.", hab * 100.0));
        }
    } else {
        let sst = t.sst[i];
        add("sea", "Sea temperature", 0, format!("The surface water averages {sst:.0} °C."));
        match t.current_type[i] {
            1 => add("sea", "Warm current", 1, "A warm current flows here from the tropics and warms the coasts downwind.".to_string()),
            2 => add("sea", "Cold current", -1, "A cold current flows here from the poles or wells up from the deep. Rich fishing, but dry, foggy coasts.".to_string()),
            _ => {}
        }
        if t.is_shelf[i] != 0 {
            add("sea", "Continental shelf", 1, format!("Shallow shelf: sunlight reaches the food chain, so fishing is good (fishery {:.0}%).", t.fishery[i] * 100.0));
        }
        let storm = t.storm_base[i] as f32 / 255.0;
        if storm > 0.35 {
            add("sea", "Storm waters", -2, format!("Hurricanes or severe storms form here ({:.0}% risk). Dangerous for shipping in season.", storm * 100.0));
        }
    }
    Ok(CellExplanation { lat, is_land, factors: f })
}


#[tauri::command]
pub fn get_cell_info(
    wx: u32,
    wy: u32,
    db: State<'_, WorldDb>,
) -> Result<CellInfo, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let grid_w: u32 = metadata::get_meta(&conn, "grid_width")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let grid_h: u32 = metadata::get_meta(&conn, "grid_height")
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800);

    let tc = TileCoord::from_world(wx, wy);
    let tile = tile_store::load_tile(&conn, tc.tx, tc.ty, 0)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(TileData::new_sea);

    let (lx, ly) = TileCoord::local(wx, wy);
    let idx = (ly * TILE_SIZE + lx) as usize;

    // Plate inspector: this cell's plate's oceanic/continental flag, read from
    // the persisted motion list rather than recomputed — `is_oceanic` is a
    // per-PLATE fact decided once at generation (or overridden by a click-to-
    // flip), never a per-cell one.
    let plate_is_oceanic: Option<bool> = metadata::get_meta(&conn, "plate_motion")
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<Vec<crate::sim::plates::PlateMotion>>(&json).ok())
        .and_then(|motion| motion.get(tile.plate_index[idx] as usize).map(|p| p.is_oceanic));

    let koppen = tile.koppen[idx];
    let elevation = tile.elevation[idx];
    let is_land = tile.terrain[idx] == 1;

    // Prefer the classified biome column (phase 6b). Fall back to the old
    // Köppen-derived name only for sea cells and worlds saved before that phase.
    let biome_code = tile.biome[idx];
    let biome = if is_land && biome_code != 0 {
        crate::sim::biome::biome_name(biome_code).to_string()
    } else {
        koppen_to_biome(koppen, elevation, is_land)
    };
    let biome_group = if is_land && biome_code != 0 {
        crate::sim::biome::biome_group(biome_code).to_string()
    } else {
        String::new()
    };

    Ok(CellInfo {
        wx,
        wy,
        grid_width: grid_w,
        grid_height: grid_h,
        terrain: if is_land { "land".into() } else { "sea".into() },
        elevation,
        sea_depth: tile.sea_depth[idx],
        temperature: tile.temperature[idx],
        precipitation: tile.precipitation[idx],
        koppen,
        biome,
        biome_code,
        biome_group,
        soil_type: tile.soil_type[idx],
        fertility: tile.fertility[idx],
        fishery: tile.fishery[idx],
        plate_index: tile.plate_index[idx],
        plate_is_oceanic,
        is_volcanic: tile.is_volcanic[idx] != 0,
        is_shelf: tile.is_shelf[idx] != 0,
        wind_vx: tile.wind_vx[idx],
        wind_vy: tile.wind_vy[idx],
        current_vx: tile.current_vx[idx],
        current_vy: tile.current_vy[idx],
        current_type: tile.current_type[idx],
        distance_to_ocean: tile.distance_to_ocean[idx],
        salinity: SAL_MIN_PSU + (tile.salinity[idx] as f32 / 255.0) * (SAL_MAX_PSU - SAL_MIN_PSU),
        storm_risk: tile.storm_base[idx] as f32 / 255.0,
        reef_risk: tile.reef_risk[idx] as f32 / 255.0,
        disease_risk: tile.disease_risk[idx] as f32 / 255.0,
        goods: {
            let specs = crate::commands::goods_commands::load_world_goods(&conn);
            (0..tile.goods.len())
                .filter_map(|g| {
                    let a = tile.goods[g][idx];
                    if a == 0 { return None; }
                    let name = specs.get(g).map(|s| s.id.clone())
                        .or_else(|| GOOD_NAMES.get(g).map(|s| s.to_string()))
                        .unwrap_or_else(|| format!("good_{g}"));
                    Some(GoodAmount { name, amount: a })
                })
                .collect()
        },
    })
}

#[cfg(test)]
mod explain_tests {
    use super::belt_reading;
    use crate::sim::circulation::Circulation;

    #[test]
    fn belts_match_earth_and_follow_the_planet() {
        let c = Circulation::earth();
        let at = |lat: f32| belt_reading(lat, c.hadley_edge, c.polar_front).0;
        assert_eq!(at(3.0), "Equatorial rain belt");
        assert_eq!(at(16.0), "Trade-wind belt");
        assert_eq!(at(28.0), "Subtropical high");
        assert_eq!(at(45.0), "Westerlies and storm track");
        assert_eq!(at(62.0), "Polar front");
        assert_eq!(at(80.0), "Polar belt");
        // A faster rotator pulls the desert belt toward the equator, and the
        // reading follows it rather than a fixed 30°.
        let fast = Circulation::from_params(3.0, 1.0, 1.0);
        assert!(fast.hadley_edge < c.hadley_edge);
        assert_eq!(belt_reading(fast.hadley_edge, fast.hadley_edge, fast.polar_front).0, "Subtropical high");
    }
}
