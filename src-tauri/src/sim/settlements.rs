use super::world_buffer::WorldBuffer;
use super::rivers::River;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Settlement {
    pub id: String,
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub size: String,       // "capital" | "city" | "town" | "village"
    pub population: u32,
    pub score: f32,
}

/// Compute habitability score for every land cell.
/// score = climate(0.40) + fertility(0.20) + water(0.20) + terrain(0.10) + trade(0.10)
pub fn compute_habitability(buf: &WorldBuffer, rivers: &[River]) -> Vec<f32> {
    let total = buf.total();
    let mut hab = vec![0.0f32; total];

    // Pre-compute river cell set and coast proximity
    let mut is_river_cell = vec![false; total];
    let mut is_river_mouth = vec![false; total];
    for river in rivers {
        for &(rx, ry) in &river.points {
            let idx = buf.idx(rx, ry);
            is_river_cell[idx] = true;
        }
        // River mouth = last point
        if let Some(&(mx, my)) = river.points.last() {
            let idx = buf.idx(mx, my);
            is_river_mouth[idx] = true;
        }
    }

    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }

            // --- Climate score (40%) ---
            let temp = buf.temperature[idx];
            let ts = if temp < -15.0 {
                0.0
            } else if temp < -5.0 {
                (temp + 15.0) / 10.0 * 0.1
            } else if temp < 5.0 {
                0.1 + (temp + 5.0) / 10.0 * 0.25
            } else if temp < 28.0 {
                0.35 + (1.0 - (temp - 16.0).abs() / 12.0) * 0.65
            } else {
                (0.5 - (temp - 28.0) / 15.0).max(0.0)
            };

            let precip = buf.precipitation[idx];
            let ps = if precip < 100.0 {
                precip / 100.0 * 0.15
            } else if precip < 250.0 {
                0.15 + (precip - 100.0) / 150.0 * 0.35
            } else if precip < 2000.0 {
                0.5 + (1.0 - (precip - 700.0).abs() / 1300.0) * 0.5
            } else {
                (1.0 - (precip - 2000.0) / 3000.0).max(0.3)
            };

            let base_climate = ts * 0.6 + ps * 0.4;

            // Köppen modifier — biases settlement toward the climates that
            // actually cradled early civilisation (Mediterranean, fertile
            // subtropics/savanna river valleys) and away from polar, desert
            // and dense-rainforest zones.
            let koppen_mod = match buf.koppen[idx] {
                8 | 9 => 0.28,        // Csa/Csb Mediterranean — ideal (Sumer, Indus, Greece)
                10 => 0.12,           // Csc Mediterranean cold-summer
                11 | 12 => 0.15,      // Cfa humid subtropical / Cfb oceanic
                13 => 0.0,            // Cfc subpolar oceanic
                3 => 0.08,            // Aw savanna (Nile / Indus-type floodplains)
                2 => 0.0,             // Am monsoon
                1 => -0.10,           // Af tropical rainforest (hard to clear/farm early)
                14 | 15 => 0.05,      // Dfa/Dfb warm-ish continental
                6 | 7 => -0.12,       // BSh/BSk steppe (marginal)
                18 | 19 | 20 => -0.05,// Ds continental Mediterranean
                4 | 5 => -0.40,       // BWh/BWk desert
                16 | 17 => -0.30,     // Dfc/Dfd cold continental
                21 => -0.60,          // ET tundra
                22 => -0.80,          // EF ice cap
                _ => 0.0,
            };

            let climate_score = (base_climate + koppen_mod).clamp(0.0, 1.0);

            // Temperature viability gate: nobody founds a capital on the ice.
            // Zero below ~+2°C annual mean, full by ~13°C, easing off in extreme
            // heat. Applied multiplicatively so a frozen but coastal/river cell
            // can't sneak past the threshold on water+trade bonuses alone. The
            // threshold was raised (from -2°C/8°C) because settlements were
            // creeping too far into the cold subpolar north; a 2-13°C ramp keeps
            // the dense settlement frontier in genuinely temperate latitudes.
            let temp_gate = if temp <= 2.0 {
                0.0
            } else if temp < 13.0 {
                (temp - 2.0) / 11.0
            } else if temp <= 30.0 {
                1.0
            } else {
                (1.0 - (temp - 30.0) / 15.0).max(0.0)
            };

            // --- Fertility score (20%) ---
            let fertility_score = buf.fertility[idx];

            // --- Water access score (20%) ---
            let mut water_score = 0.0f32;

            // River nearby (within 2 cells)
            let has_river = (-2i32..=2).any(|dy| {
                (-2i32..=2).any(|dx| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    is_river_cell[ni]
                })
            });
            if has_river { water_score += 0.5; }

            // Coast nearby (within 3 cells)
            let near_coast = buf.distance_to_ocean[idx] < 0.05;
            if near_coast { water_score += 0.3; }

            water_score = water_score.min(1.0);

            // --- Terrain score (10%) ---
            let elev = buf.elevation[idx];
            let terrain_score = if elev < 0.05 { 0.90 }
                else if elev < 0.15 { 0.90 }
                else if elev < 0.30 { 0.70 }
                else if elev < 0.50 { 0.40 }
                else if elev < 0.70 { 0.15 }
                else { 0.05 };

            // --- Trade score (10%) ---
            // River mouth nearby
            let near_river_mouth = (-3i32..=3).any(|dy| {
                (-3i32..=3).any(|dx| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    is_river_mouth[ni]
                })
            });

            let trade_score = if near_river_mouth { 1.0 }
                else if near_coast && has_river { 0.8 }
                else if near_coast { 0.5 }
                else if has_river { 0.4 }
                else { 0.1 };

            // --- Final score (gated by temperature viability) ---
            hab[idx] = ((climate_score * 0.40
                + fertility_score * 0.20
                + water_score * 0.20
                + terrain_score * 0.10
                + trade_score * 0.10) * temp_gate).clamp(0.0, 1.0);
        }
    }

    hab
}

/// Copy a habitability score field into the world buffer (for the heatmap layer).
pub fn write_habitability(buf: &mut WorldBuffer, hab: &[f32]) {
    for i in 0..buf.total() {
        buf.habitability[i] = if buf.terrain[i] == 1 { hab[i].clamp(0.0, 1.0) } else { 0.0 };
    }
}

/// Generate settlements at local maxima of habitability.
/// Cities are placed in descending probability order (greedy with a minimum
/// spacing), so the highest-habitability zones get the first/largest cities.
/// Names are intentionally left blank — the map marks ranked dots only.
pub fn generate_settlements(
    buf: &WorldBuffer,
    habitability: &[f32],
    _seed: u64,
) -> Vec<Settlement> {
    let w = buf.width;
    let h = buf.height;
    // Tighter spacing + higher cap so good regions can hold several settlements
    // (a fertile river valley realistically supports more than one town).
    let min_dist = (w / 110).max(3) as i32;
    let threshold = 0.28f32;
    let max_settlements = 600;

    // Find local maxima
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for y in 1..h - 1 {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            if habitability[idx] < threshold { continue; }

            let score = habitability[idx];
            let is_max = [(-1i32, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
                .iter()
                .all(|&(dx, dy)| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    score >= habitability[ni]
                });

            if is_max {
                candidates.push((idx, score));
            }
        }
    }

    // Sort by score descending
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Greedy selection with minimum distance
    let mut settlements: Vec<Settlement> = Vec::new();

    'outer: for (idx, score) in &candidates {
        if settlements.len() >= max_settlements { break; }

        let sx = (*idx % w as usize) as u32;
        let sy = (*idx / w as usize) as u32;

        // Check minimum distance from existing settlements
        for existing in &settlements {
            let mut dx = (sx as i32 - existing.x as i32).abs();
            if dx > w as i32 / 2 { dx = w as i32 - dx; } // wrap
            let dy = (sy as i32 - existing.y as i32).abs();
            if dx * dx + dy * dy < min_dist * min_dist {
                continue 'outer;
            }
        }

        let (size, base_pop) = if *score >= 0.80 {
            ("capital", 50000u32)
        } else if *score >= 0.65 {
            ("city", 10000)
        } else if *score >= 0.45 {
            ("town", 2000)
        } else {
            ("village", 200)
        };

        let population = (base_pop as f32 * (0.5 + score)) as u32;

        // Antique place name (Roman/Greek/Phoenician/Persian by region). Capitals
        // and cities earn a grand epithet ("Aquentia Magna").
        let tier = if size == "capital" { 2 } else if size == "city" { 1 } else { 0 };
        let name = super::names::gen_name_epithet(sx, sy, w, buf.height, tier);

        settlements.push(Settlement {
            id: format!("s-{}", settlements.len()),
            x: sx,
            y: sy,
            name,
            size: size.to_string(),
            population,
            score: *score,
        });
    }

    settlements
}
