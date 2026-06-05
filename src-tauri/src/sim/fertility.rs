use super::world_buffer::WorldBuffer;
use super::rivers::River;
use super::soil::*;

/// Compute fertility score for all land cells.
/// score = 0.30×SOIL_BASE + 0.20×precipScore + 0.15×tempScore
///       + 0.20×riverProx + 0.10×coastScore + 0.05×volcanicBonus
/// Matches WF1 fertility-score.ts algorithm.
pub fn compute_fertility(buf: &mut WorldBuffer, rivers: &[River]) {
    let w = buf.width;
    let h = buf.height;

    // Step 1: Compute river proximity via BFS
    let mut river_prox = vec![0.0f32; buf.total()];
    let max_river_dist = 5u32;

    let mut river_dist = vec![u32::MAX; buf.total()];
    let mut queue = std::collections::VecDeque::new();

    for river in rivers {
        for &(rx, ry) in &river.points {
            let idx = buf.idx(rx, ry);
            if river_dist[idx] > 0 {
                river_dist[idx] = 0;
                queue.push_back((rx, ry));
            }
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        let idx = buf.idx(x, y);
        let d = river_dist[idx];
        if d >= max_river_dist { continue; }

        for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if river_dist[ni] > d + 1 {
                river_dist[ni] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    // Gaussian decay for river proximity
    for i in 0..buf.total() {
        if river_dist[i] < u32::MAX {
            let d = river_dist[i] as f32;
            river_prox[i] = (-d * d / 8.0).exp(); // ~1.0 at river, ~0.04 at dist 5
        }
    }

    // Step 2: Compute fertility for each land cell
    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 {
                buf.fertility[idx] = 0.0;
                continue;
            }

            // Soil base score
            let soil_base = match buf.soil_type[idx] {
                SOIL_VOLCANIC_ASH => 0.97, // young ash — the richest soil of all
                SOIL_ALLUVIAL => 0.92,
                SOIL_ANDISOL => 0.85,
                SOIL_MOLLISOL => 0.90,
                SOIL_ALFISOL => 0.65,
                SOIL_HISTOSOL => 0.50,
                SOIL_ULTISOL => 0.35,
                SOIL_ENTISOL => 0.30,
                SOIL_OXISOL => 0.20,
                SOIL_SPODOSOL => 0.25,
                SOIL_ARIDISOL => 0.15,
                SOIL_GELISOL => 0.05,
                _ => 0.20,
            };

            // Precipitation score (optimal ~1000mm)
            let p = buf.precipitation[idx];
            let precip_score = if p <= 0.0 {
                0.0
            } else if p <= 250.0 {
                p / 250.0 * 0.30
            } else if p <= 500.0 {
                0.30 + (p - 250.0) / 250.0 * 0.50
            } else if p <= 1000.0 {
                0.80 + (p - 500.0) / 500.0 * 0.20
            } else if p <= 1500.0 {
                1.0 - (p - 1000.0) / 500.0 * 0.10
            } else {
                (0.90 - (p - 1500.0) / 1500.0 * 0.40).max(0.50)
            };

            // Temperature score (optimal 10-25°C)
            let t = buf.temperature[idx];
            let temp_score = if t < -10.0 {
                0.0
            } else if t < 0.0 {
                (t + 10.0) / 10.0 * 0.10
            } else if t < 10.0 {
                0.10 + t / 10.0 * 0.50
            } else if t < 17.0 {
                0.60 + (t - 10.0) / 7.0 * 0.40
            } else if t < 25.0 {
                1.0 - (t - 17.0) / 8.0 * 0.40
            } else if t < 35.0 {
                0.60 - (t - 25.0) / 10.0 * 0.40
            } else {
                0.20
            };

            // River proximity
            let rp = river_prox[idx];

            // Coastal score (5-cell search)
            let coast_dist = buf.distance_to_ocean[idx];
            let coast_score = if coast_dist < 0.08 {
                1.0 - coast_dist / 0.08
            } else {
                0.0
            };

            // Volcanic bonus
            let volcanic = if buf.is_volcanic[idx] == 1 { 1.0 } else { 0.0 };

            // Final score
            let score = 0.30 * soil_base
                + 0.20 * precip_score
                + 0.15 * temp_score
                + 0.20 * rp
                + 0.10 * coast_score
                + 0.05 * volcanic;

            // Cold-climate gate. Temperature is only 15% of the weighted sum, so
            // a frozen tundra/ice cell with a coast or a river still scored a
            // moderate fertility — the map showed "very fertile" polar regions.
            // Multiply the whole score down toward zero in genuinely cold
            // climates: nothing grows where the growing season never thaws.
            // Köppen E (tundra/ice) is forced to ~0; the ramp keeps cool-
            // temperate land (subpolar forest, boreal margins) usable.
            let k = buf.koppen[idx];
            let temp_gate = if k == 21 || k == 22 {
                0.0                                   // ET tundra / EF ice cap
            } else if t <= -8.0 {
                0.0
            } else if t < 3.0 {
                (t + 8.0) / 11.0                      // 0 at -8°C → 1 at 3°C
            } else {
                1.0
            };

            buf.fertility[idx] = (score * temp_gate).clamp(0.0, 1.0);
        }
    }
}

/// Compute fishery productivity for sea cells from four real-world drivers:
///   1. Upwelling & cold currents — cold eastern-boundary / subpolar flow over a
///      shelf is the planet's richest fishing ground (anchovy/sardine grounds).
///   2. Shelf depth & sunlight — shallow continental shelves sit in the photic
///      zone; deep open ocean is a biological desert.
///   3. River nutrient outflow — estuaries/plumes fertilise coastal water; the
///      effect scales with the contributing river's size and fades out to sea.
///   4. Latitude / temperature — cold-temperate & subpolar shelves out-produce
///      warm tropical water (with a small warm-shallow reef bonus).
/// The drivers are blended into a continuous 0..1 field (no more 0.2/0.7 steps).
pub fn compute_fisheries(buf: &mut WorldBuffer, rivers: &[River]) {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();

    // ── River-nutrient plume: BFS out from each mouth, decaying with distance,
    //    weighted by the river's width (a proxy for discharge/nutrient load). ──
    let mut nutrient = vec![0.0f32; total];
    let max_plume = 6u32;
    let mut dist = vec![u32::MAX; total];
    let mut load = vec![0.0f32; total];
    let mut queue = std::collections::VecDeque::new();
    for river in rivers {
        if let Some(&(mx, my)) = river.points.last() {
            // Seed the sea cells around the mouth.
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = buf.wrap_x(mx as i32 + dx);
                    let ny = buf.clamp_y(my as i32 + dy);
                    let ni = buf.idx(nx, ny);
                    if buf.terrain[ni] != 0 { continue; }
                    let l = (river.width / 8.0).clamp(0.15, 1.0);
                    if l > load[ni] { load[ni] = l; }
                    if dist[ni] != 0 { dist[ni] = 0; queue.push_back((nx, ny)); }
                }
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let idx = buf.idx(x, y);
        let d = dist[idx];
        if d >= max_plume { continue; }
        for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
            let nx = buf.wrap_x(x as i32 + dx);
            let ny = buf.clamp_y(y as i32 + dy);
            let ni = buf.idx(nx, ny);
            if buf.terrain[ni] != 0 { continue; }
            if dist[ni] > d + 1 {
                dist[ni] = d + 1;
                if load[idx] > load[ni] { load[ni] = load[idx]; }
                queue.push_back((nx, ny));
            }
        }
    }
    for i in 0..total {
        if dist[i] < u32::MAX {
            let d = dist[i] as f32;
            nutrient[i] = load[i] * (-d * d / 18.0).exp();
        }
    }

    for y in 0..h {
        let abs_lat = buf.latitude(y).abs();
        // Latitude productivity curve: peaks in the cold-temperate / subpolar
        // band (~45–60°), tapers toward the warm tropics and the frozen poles.
        let lat_prod = if abs_lat < 40.0 {
            0.35 + 0.45 * (abs_lat / 40.0)            // 0.35 → 0.80
        } else if abs_lat < 62.0 {
            0.80 + 0.20 * (1.0 - (abs_lat - 50.0).abs() / 12.0).max(0.0) // peak ~1.0
        } else if abs_lat < 78.0 {
            0.80 - 0.45 * ((abs_lat - 62.0) / 16.0)   // 0.80 → 0.35
        } else {
            0.30
        };

        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 0 {
                buf.fishery[idx] = 0.0;
                continue;
            }

            let depth = buf.sea_depth[idx].clamp(0.0, 1.0);
            let on_shelf = buf.is_shelf[idx] == 1;

            // Coastal gate: productivity is essentially confined to the
            // continental shelf and its immediate edge. Beyond the shelf the
            // photic zone ends and open-ocean water is a biological desert, so
            // fisheries must hug the coast where upwelling/shelf nutrients are.
            let coastal_gate = if on_shelf {
                1.0
            } else {
                (1.0 - (depth - 0.12) / 0.08).clamp(0.0, 1.0) // 0 by depth ~0.20
            };
            if coastal_gate <= 0.0 {
                buf.fishery[idx] = 0.0;
                continue;
            }

            // 1. Upwelling & cold currents — only meaningful over/near the shelf.
            let mut upwell = 0.0f32;
            if buf.current_type[idx] == 2 {
                upwell += 0.45;
                // Equatorward flow along an eastern boundary = classic upwelling.
                let lat = buf.latitude(y);
                let equatorward = if lat > 0.0 { buf.current_vy[idx] > 0.05 } else { buf.current_vy[idx] < -0.05 };
                let coast_east = (1..=4).any(|dx| buf.terrain[buf.idx(buf.wrap_x(x as i32 + dx), y)] == 1);
                if equatorward && coast_east { upwell += 0.35; }
            }

            // 3. River nutrient plume (already shelf/coast-weighted by BFS).
            let river_nut = nutrient[idx];

            // Blend: a shelf base, lifted by upwelling/nutrients, scaled by the
            // latitude productivity envelope, then confined to the coast. A small
            // warm shallow-water reef bonus keeps the tropics from reading barren.
            let shelf_base = 0.30;
            let reef = if abs_lat < 25.0 && depth < 0.10 { 0.15 } else { 0.0 };
            let biological = (shelf_base + upwell + 0.55 * river_nut + reef) * lat_prod;

            buf.fishery[idx] = (biological * coastal_gate).clamp(0.0, 1.0);
        }
    }
}
