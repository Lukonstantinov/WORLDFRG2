use super::world_buffer::WorldBuffer;

/// Compute temperature for all cells.
/// Base from latitude bands + altitude lapse + current influence + coastal damping.
/// Matches WF1 temperature.ts algorithm.
pub fn compute_temperature(buf: &mut WorldBuffer) {
    let w = buf.width;
    let h = buf.height;

    for y in 0..h {
        let lat = buf.latitude(y);
        let abs_lat = lat.abs();

        // Base annual-mean temperature from latitude bands. Tuned to real Earth
        // anchors so the mid-latitudes are warm enough to host temperate climates
        // (the old curve put 50° at +4°C, ~7°C too cold, which forced oceanic /
        // Mediterranean / temperate zones into continental, polar or arid types):
        //   0° → 30,  30° → 20,  45° → 12.5,  60° → 5,  75° → -12,  90° → -29.
        let base_temp = if abs_lat < 30.0 {
            30.0 - 0.333 * abs_lat
        } else if abs_lat < 60.0 {
            20.0 - 0.5 * (abs_lat - 30.0)
        } else {
            5.0 - 1.15 * (abs_lat - 60.0)
        };

        for x in 0..w {
            let idx = buf.idx(x, y);
            let mut temp = base_temp;

            if buf.terrain[idx] == 1 {
                // Altitude lapse rate: -5°C per 1000m (elevation is 0-1, max 8848m)
                let altitude_m = buf.elevation[idx] * 8848.0;
                temp -= 5.0 * altitude_m / 1000.0;

                // Coastal damping: reduce temperature extremes near ocean
                let ocean_dist = buf.distance_to_ocean[idx];
                if ocean_dist < 0.1 {
                    // Very close to ocean: moderate toward 15°C
                    let coastal_factor = 1.0 - ocean_dist / 0.1;
                    temp = temp + (15.0 - temp) * 0.45 * coastal_factor;
                }
            }

            buf.temperature[idx] = temp;
        }
    }

    // Current temperature influence: warm/cold currents affect nearby land.
    // Accumulate into a separate delta buffer so a coastal cell that sits
    // downwind of many ocean source cells doesn't stack a runaway anomaly,
    // then clamp the total per cell to a realistic maritime range before
    // applying it. (Earlier this added directly to temperature, letting
    // dozens of ocean sources in the same row sum into +20°C-plus coasts.)
    let current_types = buf.current_type.clone();
    let current_vx = buf.current_vx.clone();
    let mut temp_delta = vec![0.0f32; buf.total()];

    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if current_types[idx] == 0 { continue; }
            if buf.terrain[idx] != 0 { continue; } // only from ocean cells

            let delta = match current_types[idx] {
                1 => 3.0,  // warm
                2 => -3.0, // cold
                _ => continue,
            };

            // Spread influence into land cells upwind (up to 45 cells)
            let wind_dir_x = if current_vx[idx].abs() > 0.1 {
                if current_vx[idx] > 0.0 { 1i32 } else { -1 }
            } else {
                // Default: spread from ocean toward land
                let east_land = (1..=3).any(|d| {
                    let ni = buf.idx(buf.wrap_x(x as i32 + d), y);
                    buf.terrain[ni] == 1
                });
                if east_land { 1 } else { -1 }
            };

            // Large ocean currents push their thermal signal well inland; reach
            // ~70 cells with a slower decay so a big warm/cold current "covers"
            // more of the continent (the requested behaviour) rather than just
            // tinting the immediate shore.
            for step in 1..=70i32 {
                let nx = buf.wrap_x(x as i32 + wind_dir_x * step);
                let ni = buf.idx(nx, y);
                if buf.terrain[ni] != 1 { continue; }

                let decay = (-step as f32 / 25.0).exp();
                temp_delta[ni] += delta * decay;
            }
        }
    }

    // Apply clamped anomaly. ±6°C bounds maritime moderation: enough for a warm
    // current to keep a coast mild (NW-Europe style) but not so much that a cold
    // current + upwelling freezes a subtropical coast into a polar/subarctic zone.
    for i in 0..buf.total() {
        if buf.terrain[i] == 1 {
            buf.temperature[i] += temp_delta[i].clamp(-6.0, 6.0);
        }
    }
}
