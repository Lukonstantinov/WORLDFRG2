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

                // Coastal damping: only a coast exposed to OPEN ocean on the
                // prevailing-wind upwind side is moderated toward 15°C. A lee /
                // east coast — OR a coast fronted by a wide continental shelf
                // (shelf water doesn't count as open ocean) — keeps its continental
                // mean so it reads cold-winter Df/Dw instead of mild oceanic Cfb.
                let ocean_dist = buf.distance_to_ocean[idx];
                if ocean_dist < 0.1 && super::koppen::upwind_is_open_ocean(buf, x, y, 6) {
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
    //
    // The thermal push now scales with the current's SPEED — a "volume" proxy.
    // A strong warm boundary current (Gulf Stream / Kuroshio) carries a large,
    // warm body of water and pushes its signal far inland; a slow drift carries
    // less and reaches a shorter distance. Cold currents are modelled with a
    // somewhat smaller thermal magnitude ("less water"); their characteristic
    // DRYNESS is handled separately in precipitation. This keeps the
    // Gulf-Stream-to-Europe conveyor while stopping weak high-latitude drift
    // from over-warming the far north (which was letting warm-climate crops like
    // wine creep too far poleward).
    let current_types = buf.current_type.clone();
    let current_vx = buf.current_vx.clone();
    let current_vy = buf.current_vy.clone();
    let mut temp_delta = vec![0.0f32; buf.total()];

    // Reference speed: a vigorous boundary current. `vol` 0.35..1.3 scales both
    // the anomaly strength and how far it carries.
    const REF_SPEED: f32 = 1.4;

    for y in 0..h {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if current_types[idx] == 0 { continue; }
            if buf.terrain[idx] != 0 { continue; } // only from ocean cells

            let speed = (current_vx[idx] * current_vx[idx]
                + current_vy[idx] * current_vy[idx]).sqrt();
            let vol = (speed / REF_SPEED).clamp(0.35, 1.3);

            let base_delta = match current_types[idx] {
                1 => 3.4,  // warm — a warm current carries a larger heat anomaly
                2 => -2.4, // cold — less thermal mass; aridity dominates its effect
                _ => continue,
            };
            let delta = base_delta * vol;

            // Spread influence into land cells upwind. A strong current reaches
            // ~78 cells; a weak drift only ~30.
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

            let reach = (24.0 + 42.0 * vol) as i32; // ~30 (weak) .. ~78 (strong)
            let efold = 18.0 + 12.0 * vol;           // strong currents decay slower
            for step in 1..=reach {
                let nx = buf.wrap_x(x as i32 + wind_dir_x * step);
                let ni = buf.idx(nx, y);
                if buf.terrain[ni] != 1 { continue; }

                let decay = (-step as f32 / efold).exp();
                temp_delta[ni] += delta * decay;
            }
        }
    }

    // Apply clamped anomaly. Warm moderation can run a little higher than cold
    // cooling: a strong warm current keeps a coast mild (NW-Europe style), while
    // the cold-current floor is tighter so upwelling can't freeze a subtropical
    // coast into a polar/subarctic zone.
    for i in 0..buf.total() {
        if buf.terrain[i] == 1 {
            buf.temperature[i] += temp_delta[i].clamp(-5.0, 6.5);
        }
    }
}
