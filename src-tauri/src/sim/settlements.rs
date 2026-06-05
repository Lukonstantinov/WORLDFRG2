use super::world_buffer::WorldBuffer;
use super::rivers::{River, Lake};

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
pub fn compute_habitability(buf: &WorldBuffer, rivers: &[River], lakes: &[Lake]) -> Vec<f32> {
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
    // Lake cells — fresh inland water is a first-class settlement draw (lakeshore
    // towns), previously computed but unused for habitability.
    let mut is_lake_cell = vec![false; total];
    for lake in lakes {
        for &(lx, ly) in &lake.cells {
            is_lake_cell[buf.idx(lx, ly)] = true;
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
                21 => -0.85,          // ET tundra — frozen ground, no farming
                22 => -0.92,          // EF ice cap — uninhabitable
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
            let temp_gate = if temp <= 3.0 {
                0.0
            } else if temp < 14.0 {
                (temp - 3.0) / 11.0
            } else if temp <= 30.0 {
                1.0
            } else {
                (1.0 - (temp - 30.0) / 15.0).max(0.0)
            };

            // Explicit cryosphere penalty: tundra is barely habitable, ice caps
            // never. Multiplicative so no bonus (coast/trade) can rescue them.
            let cryo_gate = match buf.koppen[idx] {
                21 => 0.30, // ET tundra
                22 => 0.0,  // EF ice cap
                _ => 1.0,
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

            // Coast nearby — a stronger draw now, so genuine PORTS form on rivers'
            // absence (harbours, fishing towns), not only at river mouths.
            let near_coast = buf.distance_to_ocean[idx] < 0.05;
            if near_coast { water_score += 0.45; }

            // Lakeshore (fresh inland water within 2 cells).
            let has_lake = (-2i32..=2).any(|dy| {
                (-2i32..=2).any(|dx| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    is_lake_cell[ni]
                })
            });
            if has_lake { water_score += 0.40; }

            // Desert oasis: in arid land any reliable water (a river crossing the
            // desert, a lake, or a fertile pocket) is precious — caravan oasis towns.
            let arid = matches!(buf.koppen[idx], 4 | 5 | 6 | 7);
            let oasis = arid && (has_river || has_lake || fertility_score > 0.40);
            if oasis { water_score += 0.25; }

            water_score = water_score.min(1.0);

            // --- Terrain score (10%) ---
            // Flat lowland is best for farming, but a commanding hill is a prized,
            // defensible site (acropolis / hill-fort), so moderate relief gets a
            // defensive premium instead of a pure penalty.
            let elev = buf.elevation[idx];
            let farm: f32 = if elev < 0.15 { 0.90 }
                else if elev < 0.30 { 0.70 }
                else if elev < 0.50 { 0.40 }
                else if elev < 0.70 { 0.15 }
                else { 0.05 };
            let defensive: f32 = if (0.22..0.50).contains(&elev) { 0.20 } else { 0.0 };
            let terrain_score = (farm + defensive).min(1.0);

            // --- Trade score (10%) ---
            // River mouth nearby
            let near_river_mouth = (-3i32..=3).any(|dy| {
                (-3i32..=3).any(|dx| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    is_river_mouth[ni]
                })
            });

            let trade_score = if near_river_mouth { 1.0 }
                else if near_coast && has_river { 0.85 }
                else if near_coast { 0.6 }   // natural harbour / port
                else if oasis { 0.6 }        // caravan oasis on a desert route
                else if has_lake { 0.5 }     // lake port
                else if has_river { 0.45 }
                else { 0.1 };

            // Disease suppression: malaria/fever lowlands are settled, but more
            // sparsely and in smaller numbers (a multiplicative drag, not a wall).
            let disease_gate = 1.0 - 0.55 * (buf.disease_risk[idx] as f32 / 255.0);

            // --- Final score (gated by temperature viability) ---
            hab[idx] = ((climate_score * 0.40
                + fertility_score * 0.20
                + water_score * 0.20
                + terrain_score * 0.10
                + trade_score * 0.10) * temp_gate * cryo_gate * disease_gate).clamp(0.0, 1.0);
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

/// Per-cell food potential — the basis of agricultural carrying capacity. Land
/// only (0 on sea). Farmland = fertility × growing-season length × irrigation
/// (arid land beside a wide river is a breadbasket) × disease drag; a coastal cell
/// also eats from the neighbouring sea (the fishery field).
pub fn compute_food_capacity(buf: &WorldBuffer, rivers: &[River]) -> Vec<f32> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();

    // River-cell mask + local river width (irrigation strength).
    let mut river_w = vec![0.0f32; total];
    for r in rivers {
        for &(rx, ry) in &r.points {
            let i = buf.idx(rx, ry);
            if r.width > river_w[i] { river_w[i] = r.width; }
        }
    }

    let mut food = vec![0.0f32; total];
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if buf.terrain[i] != 1 { continue; }

            let fert = buf.fertility[i];
            // Growing season (0..12 months above 10°C): long seasons double-crop.
            let gs = super::koppen::growing_season_months(buf, x, y);
            let season = 0.45 + 0.55 * (gs / 12.0);

            // Irrigation: arid land next to a (wide) river is a breadbasket; dry
            // farming on arid land with no water is poor.
            let arid = matches!(buf.koppen[i], 4 | 5 | 6 | 7);
            let irrig = if arid {
                let mut rw = 0.0f32;
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                        if river_w[ni] > rw { rw = river_w[ni]; }
                    }
                }
                if rw > 0.0 { 2.5 + (rw / 4.0).min(1.5) } else { 0.5 }
            } else {
                1.0
            };

            let disease = 1.0 - 0.4 * (buf.disease_risk[i] as f32 / 255.0);
            let mut f = fert * season * irrig * disease;

            // Coastal fishery: a port feeds itself from the sea.
            if buf.distance_to_ocean[i] < 0.05 {
                let mut fish = 0.0f32;
                let mut cnt = 0.0f32;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2)] {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    if buf.terrain[ni] == 0 { fish += buf.fishery[ni]; cnt += 1.0; }
                }
                if cnt > 0.0 { f += 0.6 * (fish / cnt); }
            }

            food[i] = f.max(0.0);
        }
    }
    food
}

/// Generate settlements at local maxima of habitability, then size each by
/// EMERGENT carrying capacity: the food its catchment (farmland + fisheries) can
/// feed, plus a trade-access premium (ports / navigable rivers / crossroads).
/// Placement is unchanged; only population & tier are carrying-capacity-driven.
pub fn generate_settlements(
    buf: &WorldBuffer,
    habitability: &[f32],
    rivers: &[River],
    _seed: u64,
) -> Vec<Settlement> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();
    let min_dist = (w / 110).max(3) as i32;
    let threshold = 0.28f32;
    let max_settlements = 600;

    let food = compute_food_capacity(buf, rivers);

    // ── Site selection: greedy local-maxima of habitability with spacing ──
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
            if is_max { candidates.push((idx, score)); }
        }
    }
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // (idx, x, y, score)
    let mut sites: Vec<(usize, u32, u32, f32)> = Vec::new();
    'outer: for (idx, score) in &candidates {
        if sites.len() >= max_settlements { break; }
        let sx = (*idx % w as usize) as u32;
        let sy = (*idx / w as usize) as u32;
        for &(_, ex, ey, _) in &sites {
            let mut dx = (sx as i32 - ex as i32).abs();
            if dx > w as i32 / 2 { dx = w as i32 - dx; }
            let dy = (sy as i32 - ey as i32).abs();
            if dx * dx + dy * dy < min_dist * min_dist { continue 'outer; }
        }
        sites.push((*idx, sx, sy, *score));
    }
    if sites.is_empty() { return Vec::new(); }

    // ── Carrying capacity: coarse-Voronoi catchment, capped to a real hinterland
    // (a lone town can't claim a whole continent) so no double-counting of food. ──
    let f = (w / 220).max(1);
    let cw = ((w + f - 1) / f) as i32;
    let ch = ((h + f - 1) / f) as i32;
    let mut coarse_food = vec![0.0f32; (cw * ch) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = buf.idx(x, y);
            if food[i] <= 0.0 { continue; }
            let cx = (x / f) as i32;
            let cy = (y / f) as i32;
            coarse_food[(cy * cw + cx) as usize] += food[i];
        }
    }
    let max_catch = min_dist as f32 * 2.5;
    let max_catch2 = (max_catch * max_catch) as i64;
    let mut k_food = vec![0.0f32; sites.len()];
    for cy in 0..ch {
        for cx in 0..cw {
            let cf = coarse_food[(cy * cw + cx) as usize];
            if cf <= 0.0 { continue; }
            let wx = (cx as u32 * f + f / 2).min(w - 1) as i32;
            let wy = (cy as u32 * f + f / 2).min(h - 1) as i32;
            let mut best = usize::MAX;
            let mut bd = i64::MAX;
            for (si, &(_, sxx, syy, _)) in sites.iter().enumerate() {
                let mut dx = (wx - sxx as i32).abs();
                if dx > w as i32 / 2 { dx = w as i32 - dx; }
                let dy = wy - syy as i32;
                let d = (dx * dx + dy * dy) as i64;
                if d < bd { bd = d; best = si; }
            }
            if best != usize::MAX && bd <= max_catch2 { k_food[best] += cf; }
        }
    }

    // ── Trade-access masks (ports / navigable rivers / river mouths & deltas) ──
    let mut nav_mask = vec![false; total];
    let mut mouth_mask = vec![false; total];
    let mark = |mask: &mut Vec<bool>, cx: i32, cy: i32, r: i32| {
        for dy in -r..=r {
            for dx in -r..=r {
                let nx = ((cx + dx) % w as i32 + w as i32) % w as i32;
                let ny = cy + dy;
                if ny < 0 || ny >= h as i32 { continue; }
                mask[(ny as u32 * w + nx as u32) as usize] = true;
            }
        }
    };
    for r in rivers {
        if let Some(&(mx, my)) = r.points.last() {
            mark(&mut mouth_mask, mx as i32, my as i32, 3);
        }
        for &(dx, dy) in &r.delta { mark(&mut mouth_mask, dx as i32, dy as i32, 1); }
        if r.navigable {
            for &(rx, ry) in &r.points { mark(&mut nav_mask, rx as i32, ry as i32, 1); }
        }
    }

    // ── Population & tier from carrying capacity + trade access ──
    // Calibration (resolution-dependent; tune per world size in verification). Kept
    // modest so the agricultural baseline skews to villages/towns — the metropolises
    // emerge afterward from the trade-development pass (compute_settlement_development).
    const FOOD_TO_POP: f32 = 25.0;
    const TRADE_ALPHA: f32 = 1.6;
    let mid = (min_dist * 4) as i64;
    let mid2 = mid * mid;
    let mut settlements: Vec<Settlement> = Vec::with_capacity(sites.len());
    for (si, &(idx, sx, sy, score)) in sites.iter().enumerate() {
        let near_coast = buf.distance_to_ocean[idx] < 0.05;
        let mouth = mouth_mask[idx];
        let nav = nav_mask[idx];
        // Crossroads: how many other towns lie within a mid-range radius.
        let mut neigh = 0i32;
        for (sj, &(_, ox, oy, _)) in sites.iter().enumerate() {
            if sj == si { continue; }
            let mut dx = (sx as i32 - ox as i32).abs();
            if dx > w as i32 / 2 { dx = w as i32 - dx; }
            let dy = sy as i32 - oy as i32;
            if (dx * dx + dy * dy) as i64 <= mid2 { neigh += 1; }
        }
        let crossroads = (neigh as f32 / 6.0).min(1.0);
        let access = ((if near_coast { 0.4 } else { 0.0 })
            + (if mouth { 0.3 } else if nav { 0.2 } else { 0.0 })
            + 0.3 * crossroads)
            .clamp(0.0, 1.0);

        let pop_agri = k_food[si] * FOOD_TO_POP;
        let population = (pop_agri * (1.0 + TRADE_ALPHA * access)).max(40.0) as u32;

        let size = if population >= 100_000 { "capital" }
            else if population >= 30_000 { "city" }
            else if population >= 5_000 { "town" }
            else { "village" };
        let tier = if size == "capital" { 2 } else if size == "city" { 1 } else { 0 };
        let name = super::names::gen_name_epithet(sx, sy, w, h, tier);

        settlements.push(Settlement {
            id: format!("s-{}", si),
            x: sx,
            y: sy,
            name,
            size: size.to_string(),
            population,
            score,
        });
    }

    settlements
}
