use crate::sim::world_buffer::WorldBuffer;
use crate::sim::rivers::{River, Lake};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Settlement {
    pub id: String,
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub size: String,       // "capital" | "city" | "town" | "village"
    pub population: u32,
    pub score: f32,
    // â”€â”€ Culture / geography labels (serde default â†’ old saves still load) â”€â”€
    /// The people/culture governing this site ("Norse", "Sinitic", â€¦).
    #[serde(default)] pub culture: String,
    /// The region / homeland name (e.g. "Vexillia").
    #[serde(default)] pub region: String,
    /// The site type: "coast" | "river" | "hills" | "plain".
    #[serde(default)] pub site: String,
}

/// Classify a cell's site type from terrain (for settlement labels / search rows).
fn site_label(buf: &WorldBuffer, idx: usize, is_river: bool) -> &'static str {
    if buf.distance_to_ocean[idx] < 0.05 { "coast" }
    else if is_river { "river" }
    else if buf.elevation[idx] > 0.40 { "hills" }
    else { "plain" }
}

/// The habitability field PLUS the trade field it already computes internally and
/// used to throw away (PORTS_JUNCTIONS_AND_PROVINCE_VIEW_PLAN.md F2 / slice 3a).
/// `trade` is the ladder value already multiplied by the SAME four viability gates
/// `hab` uses (`temp_gate × winter_gate × cryo_gate × disease_gate`) — the gates are
/// deliberately not the climate/fertility terms, so a hot barren desert port (Hormuz
/// had no fresh water and no vegetation and was the richest port on earth) survives
/// while nothing is ever planted on the ice.
pub struct HabFields {
    pub hab: Vec<f32>,
    pub trade: Vec<f32>,
}

/// Compute habitability score for every land cell.
/// score = climate(0.40) + fertility(0.20) + water(0.20) + terrain(0.10) + trade(0.10)
///
/// Thin wrapper over `compute_habitability_fields` (slice 3a) — kept so the pre-
/// existing call sites that only ever wanted `hab` stay untouched. It passes no flow-
/// accumulation array, so slice 3c's mouth-scaling degrades to its own no-scaling
/// default (`mouth_mult = 1.0`) here; the pipeline's own settlement-generation call
/// sites call `compute_habitability_fields` directly with the real accumulation field.
pub fn compute_habitability(buf: &WorldBuffer, rivers: &[River], lakes: &[Lake]) -> Vec<f32> {
    compute_habitability_fields(buf, rivers, lakes, None).hab
}

/// See `compute_habitability`. `acc` is `Hydrology.acc` (flow accumulation per cell,
/// already computed by phase 5) — `None` when unavailable (e.g. a caller with no
/// hydrology in scope), in which case slice 3c's river-mouth scaling is a no-op.
pub fn compute_habitability_fields(
    buf: &WorldBuffer, rivers: &[River], lakes: &[Lake], acc: Option<&[u32]>,
) -> HabFields {
    let total = buf.total();
    let mut hab = vec![0.0f32; total];
    let mut trade = vec![0.0f32; total];
    let max_acc = acc.map(|a| a.iter().copied().max().unwrap_or(1).max(1)).unwrap_or(1);

    // 3b · strait/isthmus and mountain-pass/saddle radii, in WORLD cells, scaled to
    // match the physical granularity `build_coarse_cost`'s own chokepoint/pass
    // detectors use on the coarse route-cost grid (`f = grid_w / 700`) — the same
    // real-world scale, just read straight off the fine buffer instead of a
    // resampled grid, since worldgen has no settlements yet to drive a route grid.
    let choke_r = (3 * (buf.width / 700).max(1)) as i32;
    let pass_r = (buf.width / 700).max(3) as i32;
    let smoothstep = |t: f32| { let t = t.clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) };

    // Pre-compute river cell set and coast proximity
    let mut is_river_cell = vec![false; total];
    let mut is_river_mouth = vec![false; total];
    // Navigable rivers are inland highways â€” a town on one behaves like a port.
    let mut is_navigable_cell = vec![false; total];
    // Confluences (where a tributary joins a larger stream) and the head of
    // navigation (upstream limit of a navigable trunk = the fall line where
    // rapids stop boats) are the classic river-city magnets â€” St. Louis /
    // Khartoum sit at confluences, Richmond / fall-line cities at the head of nav.
    let mut is_confluence_cell = vec![false; total];
    let mut is_head_of_nav = vec![false; total];
    // Estuary/delta mouths (drowned tidal mouth or depositional fan) are prime
    // deep-water port + fishery sites, distinct from an ordinary river mouth.
    let mut is_estuary_mouth = vec![false; total];
    for river in rivers {
        for &(rx, ry) in &river.points {
            let idx = buf.idx(rx, ry);
            is_river_cell[idx] = true;
            if river.navigable { is_navigable_cell[idx] = true; }
        }
        if river.tributary {
            // Tributary segment ends at its confluence with a larger stream.
            if let Some(&(cx, cy)) = river.points.last() {
                is_confluence_cell[buf.idx(cx, cy)] = true;
            }
        } else if let Some(&(mx, my)) = river.points.last() {
            // Trunk: last point is the sea mouth.
            is_river_mouth[buf.idx(mx, my)] = true;
            if river.mouth_kind == 1 || river.mouth_kind == 2 {
                is_estuary_mouth[buf.idx(mx, my)] = true;
            }
        }
        // Head of navigation: the upstream-most point of a navigable trunk.
        if river.navigable {
            if let Some(&(hx, hy)) = river.points.first() {
                is_head_of_nav[buf.idx(hx, hy)] = true;
            }
        }
    }
    // Lake cells â€” FRESH inland water is a first-class settlement draw (lakeshore
    // towns). Terminal SALT lakes are held apart: their brine is undrinkable (no
    // fresh-water bonus) but their shores are a strong TRADE magnet â€” salt was a
    // prime historical commodity (Salzburg, Timbuktu, the salt roads).
    let mut is_lake_cell = vec![false; total];
    let mut is_salt_lake_cell = vec![false; total];
    for lake in lakes {
        let salt = lake.endorheic && lake.salinity_ppt >= crate::sim::rivers::SALT_PRODUCTION_PPT;
        for &(lx, ly) in &lake.cells {
            if salt { is_salt_lake_cell[buf.idx(lx, ly)] = true; }
            else { is_lake_cell[buf.idx(lx, ly)] = true; }
        }
    }

    for y in 0..buf.height {
        for x in 0..buf.width {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }

            // --- Climate score (40%) ---
            let temp = buf.temperature[idx];
            // Coldest-month temperature (continentality-aware) for the winter gate â€”
            // so a brutal-winter lee/east coast (Vladivostok/Kamchatka/Hudson Bay)
            // can't host a metropolis just because its annual MEAN looks mild.
            let winter_temp = crate::sim::koppen::seasonal_temps(buf, x, y).0;
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

            // KÃ¶ppen modifier â€” biases settlement toward the climates that
            // actually cradled early civilisation (Mediterranean, fertile
            // subtropics/savanna river valleys) and away from polar, desert
            // and dense-rainforest zones.
            let koppen_mod = match buf.koppen[idx] {
                8 | 9 => 0.42,        // Csa/Csb Mediterranean â€” ideal (Sumer, Indus, Greece); the
                                      // cradle climate, biased the strongest so towns cluster here.
                10 => 0.20,           // Csc Mediterranean cold-summer
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
                21 => -0.85,          // ET tundra â€” frozen ground, no farming
                22 => -0.92,          // EF ice cap â€” uninhabitable
                _ => 0.0,
            };

            let climate_score = (base_climate + koppen_mod).clamp(0.0, 1.0);

            // Temperature viability gate: nobody founds a capital on the ice.
            // Zero below ~+2Â°C annual mean, full by ~13Â°C, easing off in extreme
            // heat. Applied multiplicatively so a frozen but coastal/river cell
            // can't sneak past the threshold on water+trade bonuses alone. The
            // threshold was raised (from -2Â°C/8Â°C) because settlements were
            // creeping too far into the cold subpolar north; a 2-13Â°C ramp keeps
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

            // Winter-severity gate: brutal coldest-month winters (continental east
            // coasts, deep interiors) suppress large permanent cities â€” full above
            // -10Â°C, tapering to a 0.30 floor by -32Â°C (Harbin/Moscow stay possible,
            // megalopolises do not).
            let winter_gate = if winter_temp >= -10.0 {
                1.0
            } else {
                (1.0 + (winter_temp + 10.0) / 22.0).clamp(0.30, 1.0)
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

            // River nearby (within 2 cells). Rivers are the dominant pre-modern
            // settlement magnet (water, transport, defence, fertile floodplain), so
            // the draw is strong â€” a cell ON or right beside a river gets the full
            // bonus, making river valleys line with towns.
            let on_river = is_river_cell[idx];
            let mut on_navigable = false;
            let mut near_confluence = false;
            let has_river = on_river || (-2i32..=2).any(|dy| {
                (-2i32..=2).any(|dx| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    if is_navigable_cell[ni] { on_navigable = true; }
                    if is_confluence_cell[ni] { near_confluence = true; }
                    is_river_cell[ni]
                })
            });
            if has_river { water_score += if on_river { 0.7 } else { 0.55 }; }
            // A navigable river (an inland trade artery) is a stronger draw than an
            // unnavigable creek, and a confluence doubly so (two valleys + a route
            // node meet). Bounded by the overall water_score.min(1.0) below.
            if on_navigable { water_score += 0.20; }
            if near_confluence { water_score += 0.15; }

            // Coast nearby â€” a stronger draw now, so genuine PORTS form on rivers'
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
            // desert, a lake, or a fertile pocket) is precious â€” caravan oasis towns.
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
            // River mouth / estuary / head-of-navigation nearby (the river-trade
            // nodes). Estuaries & deltas are deep-water ports; the head of
            // navigation is a natural entrepÃ´t where river and overland trade meet.
            let mut near_river_mouth = false;
            let mut near_estuary = false;
            // 3c · the hinterland a river mouth drains, read straight off the flow-
            // accumulation field (no distance scan needed — `acc[idx]` at the mouth
            // cell already integrates its whole upstream basin). The MAX over every
            // mouth found in the window, so a candidate a couple of cells off the
            // exact mouth still reads that mouth's real size, not its own (near-zero,
            // off-channel) accumulation.
            let mut mouth_acc = 0u32;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    if is_river_mouth[ni] {
                        near_river_mouth = true;
                        if let Some(a) = acc { mouth_acc = mouth_acc.max(a[ni]); }
                    }
                    if is_estuary_mouth[ni] {
                        near_estuary = true;
                        if let Some(a) = acc { mouth_acc = mouth_acc.max(a[ni]); }
                    }
                }
            }
            let near_head_nav = (-2i32..=2).any(|dy| {
                (-2i32..=2).any(|dx| is_head_of_nav[buf.widx(x as i32 + dx, y as i32 + dy)])
            });
            // Salt-lake shore within a few cells â†’ a salt-trading town site.
            let near_salt = (-3i32..=3).any(|dy| {
                (-3i32..=3).any(|dx| is_salt_lake_cell[buf.widx(x as i32 + dx, y as i32 + dy)])
            });

            // 3b Â· strait / isthmus â€” open sea on TWO OPPOSITE sides within
            // `choke_r` cells (Constantinople, Malacca, Hormuz, Copenhagen). Pure
            // local geometry, no routes or settlements needed, so it is available
            // this early in the pipeline.
            let sea_within = |dx: i32, dy: i32, r: i32| -> bool {
                (1..=r).any(|k| {
                    let ny = y as i32 + dy * k;
                    if ny < 0 || ny >= buf.height as i32 { return false; }
                    buf.terrain[buf.widx(x as i32 + dx * k, ny)] == 0
                })
            };
            let is_strait = (sea_within(-1, 0, choke_r) && sea_within(1, 0, choke_r))
                || (sea_within(0, -1, choke_r) && sea_within(0, 1, choke_r));

            // 3b Â· mountain pass / saddle â€” a moderately high cell that is a local
            // LOW along one axis between higher flanks (the Gotthard's economic
            // history is one bridge relocating European trade). Sampled at `pass_r`
            // world cells so the gap reads at the physical scale of a real pass
            // rather than the ±14 m micro-relief dither every land cell carries.
            let is_pass = if elev < 0.33 { false } else {
                let elev_at = |wx: i32, wy: i32| -> f32 {
                    let wy = wy.clamp(0, buf.height as i32 - 1);
                    buf.elevation[buf.widx(wx, wy)]
                };
                let l = elev_at(x as i32 - pass_r, y as i32);
                let r = elev_at(x as i32 + pass_r, y as i32);
                let up = elev_at(x as i32, y as i32 - pass_r);
                let dn = elev_at(x as i32, y as i32 + pass_r);
                (elev < l && elev < r && (l.min(r) - elev) > 0.04)
                    || (elev < up && elev < dn && (up.min(dn) - elev) > 0.04)
            };

            // 3c Â· scale the water-mouth rungs by the hinterland they drain: a river
            // mouth draining a continent is Rotterdam, one draining 20 km of hillside
            // is a fishing village. Floored at 0.5 so a genuinely small mouth still
            // reads as SOME port rather than vanishing â€” only the top end (a world's
            // largest basins) reaches the full rung.
            let mouth_mult = 0.5 + 0.5 * smoothstep(mouth_acc as f32 / max_acc as f32);

            let trade_score = if near_estuary { 1.0 * mouth_mult }   // drowned tidal port / delta entrepÃ´t
                else if is_strait { 0.95 }                            // strait / isthmus chokepoint
                else if near_river_mouth { 0.92 * mouth_mult }
                else if near_coast && on_navigable { 0.90 * mouth_mult } // river port at the sea
                else if near_coast && has_river { 0.85 }
                else if is_pass { 0.82 }                               // mountain pass / saddle
                else if near_confluence { 0.80 }                       // confluence trade node
                else if near_head_nav { 0.78 }                         // fall-line entrepÃ´t
                else if near_salt { 0.76 }   // salt-lake shore: salt-trade town
                else if on_navigable { 0.70 }                // navigable inland highway
                else if near_coast { 0.6 }   // natural harbour / port
                else if oasis { 0.6 }        // caravan oasis on a desert route
                else if has_lake { 0.5 }     // lake port
                else if has_river { 0.45 }
                else { 0.1 };

            // Disease suppression: malaria/fever lowlands are settled, but more
            // sparsely and in smaller numbers (a multiplicative drag, not a wall).
            let disease_gate = 1.0 - 0.55 * (buf.disease_risk[idx] as f32 / 255.0);

            let gates = temp_gate * winter_gate * cryo_gate * disease_gate;

            // --- Final score (gated by temperature viability) ---
            hab[idx] = ((climate_score * 0.40
                + fertility_score * 0.20
                + water_score * 0.20
                + terrain_score * 0.10
                + trade_score * 0.10) * gates).clamp(0.0, 1.0);
            trade[idx] = (trade_score * gates).clamp(0.0, 1.0);
        }
    }

    HabFields { hab, trade }
}

/// Copy a habitability score field into the world buffer (for the heatmap layer).
pub fn write_habitability(buf: &mut WorldBuffer, hab: &[f32]) {
    for i in 0..buf.total() {
        buf.habitability[i] = if buf.terrain[i] == 1 { hab[i].clamp(0.0, 1.0) } else { 0.0 };
    }
}

/// Per-cell food potential â€” the basis of agricultural carrying capacity. Land
/// only (0 on sea). Farmland = fertility Ã— growing-season length Ã— irrigation
/// (arid land beside a wide river is a breadbasket) Ã— disease drag; a coastal cell
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
            // Growing season (0..12 months above 10Â°C): long seasons double-crop.
            let gs = crate::sim::koppen::growing_season_months(buf, x, y);
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
/// `realism` (0..1) is the single "settlement density / realism" lever. LOW =
/// sparse and strict: only genuinely viable sites survive (high habitability
/// threshold, wide spacing, low cap) so there are no marginal polar/desert
/// specks. HIGH = dense and permissive (low threshold, tight spacing, high cap).
/// ~0.55 reproduces the historical default.
pub fn generate_settlements(
    buf: &WorldBuffer,
    habitability: &[f32],
    rivers: &[River],
    _seed: u64,
    realism: f32,
    cap: Option<usize>,
) -> Vec<Settlement> {
    let w = buf.width;
    let h = buf.height;
    let total = buf.total();
    let d = realism.clamp(0.0, 1.0);
    // Spacing widens, the habitability threshold rises, and the count cap drops as
    // realism is lowered â€” pruning marginal sites and thinning the map together.
    let min_dist = ((w as f32 / (95.0 + 90.0 * (1.0 - d))) as u32).max(3) as i32;
    let threshold = 0.22 + 0.18 * (1.0 - d); // d=1 â†’ 0.22 (permissive) Â· d=0 â†’ 0.40 (strict)
    // An explicit user cap (20..1000) HARD-limits the total settlement count; otherwise
    // the realism slider sets it (d=1 â†’ 1000 Â· d=0 â†’ 180).
    let explicit_cap = cap.map(|c| c.clamp(20, 1000));
    let max_settlements = explicit_cap.unwrap_or((180.0 + 820.0 * d) as usize);

    let food = compute_food_capacity(buf, rivers);

    // River-cell mask: towns are allowed to pack MUCH closer along a river (a
    // string of small river towns) than out in open country, so river valleys
    // fill with settlements instead of one town per wide spacing radius.
    let mut is_river_cell = vec![false; total];
    for river in rivers {
        for &(rx, ry) in &river.points {
            is_river_cell[buf.idx(rx, ry)] = true;
        }
    }

    // â”€â”€ Site selection: greedy local-maxima of habitability with spacing â”€â”€
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
    let river_min_dist = (min_dist as f32 * 0.5).max(2.0) as i32; // pack river towns tighter
    'outer: for (idx, score) in &candidates {
        if sites.len() >= max_settlements { break; }
        let sx = (*idx % w as usize) as u32;
        let sy = (*idx / w as usize) as u32;
        // A river-side candidate only needs the (much smaller) river spacing from
        // existing sites, so many small towns line a valley.
        let req = if is_river_cell[*idx] { river_min_dist } else { min_dist };
        let req2 = req * req;
        for &(_, ex, ey, _) in &sites {
            let mut dx = (sx as i32 - ex as i32).abs();
            if dx > w as i32 / 2 { dx = w as i32 - dx; }
            let dy = (sy as i32 - ey as i32).abs();
            if dx * dx + dy * dy < req2 { continue 'outer; }
        }
        sites.push((*idx, sx, sy, *score));
    }
    if sites.is_empty() { return Vec::new(); }

    // â”€â”€ Carrying capacity: coarse-Voronoi catchment, capped to a real hinterland
    // (a lone town can't claim a whole continent) so no double-counting of food. â”€â”€
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

    // â”€â”€ Trade-access masks (ports / navigable rivers / river mouths & deltas) â”€â”€
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

    // â”€â”€ Population & tier from carrying capacity + trade access â”€â”€
    // Calibration (resolution-dependent; tune per world size in verification). Kept
    // modest so the agricultural baseline skews to villages/towns â€” the metropolises
    // emerge afterward from the trade-development pass (compute_settlement_development).
    const FOOD_TO_POP: f32 = 25.0;
    // Trade access is the dominant driver of the LARGEST cities (history's great
    // metropolises are nearly all ports / river-mouths / crossroads, not the most
    // fertile inland valley). Raised so coastal & estuary sites outgrow a purely
    // agricultural inland breadbasket.
    const TRADE_ALPHA: f32 = 2.6;
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
        let access = ((if near_coast { 0.55 } else { 0.0 })
            + (if mouth { 0.45 } else if nav { 0.3 } else { 0.0 })
            + 0.3 * crossroads)
            .clamp(0.0, 1.0);

        let pop_agri = k_food[si] * FOOD_TO_POP;
        // Extra multiplicative port premium so a great natural harbour (coast +
        // river mouth) can become a metropolis even on modest farmland, while a
        // landlocked town stays capped by its hinterland's food.
        let port_premium = 1.0
            + (if near_coast { 0.5 } else { 0.0 })
            + (if mouth { 0.3 } else { 0.0 });

        // Latitude concentration of population: history's great metropolises cluster
        // in the warm subtropics / Mediterranean belt (Sumer, Egypt, Greece, the
        // Indus, China) â€” NOT the cold north. `civ_factor` gives a population bonus
        // peaking ~32Â°, and `cold_factor` taxes high latitudes hard so there are far
        // fewer huge cities above 45â€“50Â° (the user's main complaint).
        let abs_lat = buf.latitude(sy).abs();
        let civ_factor = 1.0 + 0.30 * (-((abs_lat - 30.0).powi(2)) / (2.0 * 12.0 * 12.0)).exp();
        let cold_factor = if abs_lat <= 45.0 {
            1.0
        } else if abs_lat <= 62.0 {
            1.0 - 0.55 * (abs_lat - 45.0) / 17.0  // 1.0 â†’ 0.45 across 45â€“62Â°
        } else {
            (0.45 - 0.18 * (abs_lat - 62.0) / 13.0).max(0.22) // 0.45 â†’ 0.27 across 62â€“75Â°+
        };
        // Continental winter severity caps city size even at MID latitude â€” a
        // brutal-winter east coast (Vladivostok â‰ˆ43Â°, Harbin) escapes the
        // latitude-only `cold_factor` but is real, not a megacity. Full above
        // -8Â°C coldest month, down to a 0.30 floor by -30Â°C.
        let winter_t = crate::sim::koppen::seasonal_temps(buf, sx, sy).0;
        let winter_factor = if winter_t >= -8.0 {
            1.0
        } else {
            (1.0 + (winter_t + 8.0) / 22.0).clamp(0.30, 1.0)
        };
        let population = (pop_agri * (1.0 + TRADE_ALPHA * access) * port_premium
            * civ_factor * cold_factor * winter_factor).max(40.0) as u32;

        let size = if population >= 100_000 { "capital" }
            else if population >= 30_000 { "city" }
            else if population >= 5_000 { "town" }
            else { "village" };
        let tier = if size == "capital" { 2 } else if size == "city" { 1 } else { 0 };
        let name = crate::sim::names::gen_name_epithet(sx, sy, w, h, tier);

        settlements.push(Settlement {
            id: format!("s-{}", si),
            x: sx,
            y: sy,
            name,
            size: size.to_string(),
            population,
            score,
            culture: crate::sim::names::culture_label(sx, sy, w, h).to_string(),
            region: crate::sim::names::region_name(sx, sy, w, h),
            site: site_label(buf, idx, is_river_cell[idx]).to_string(),
        });
    }

    // â”€â”€ Trading outposts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // Small supply-settlements (same Settlement type, tiny population) in the
    // HARSH zones where ordinary towns won't form â€” hot deserts and cold
    // subarctic/tundra â€” but where a resource worth shipping downstream exists (a
    // caravan oasis, a coastal whaling/fishing post, a mountain ore lode, a
    // volcanic field). Far fewer than settlements, and NEVER on EF ice caps. They
    // become low-power economy nodes that funnel their good to the nearest hubs.
    {
        let mut river_near = vec![false; total];
        for r in rivers {
            for &(rx, ry) in &r.points {
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        river_near[buf.widx(rx as i32 + dx, ry as i32 + dy)] = true;
                    }
                }
            }
        }
        // Harsh climates only. EF (22) ice caps are excluded; the bitter ice-bound
        // subarctic/tundra (DFd 17, DWd 30) are dropped too â€” the user forbids trade
        // posts on ice sheets / ice caps, so we keep only the hot/cold deserts,
        // steppe, mild tundra and the *milder* subarctic (DFc 16, DWc 29).
        let harsh = |k: u8| matches!(k, 4 | 5 | 6 | 7 | 21 | 16 | 29);
        let out_min_dist = (min_dist * 2).max(4);
        // Under an explicit cap the harsh-zone outposts must fit within the remaining
        // budget so the TOTAL never exceeds the user's limit.
        let budget = if explicit_cap.is_some() { max_settlements.saturating_sub(settlements.len()) } else { usize::MAX };
        let max_outposts = (sites.len() / 5).min(50).min(budget);
        let mut cand: Vec<(usize, f32)> = Vec::new();
        for y in 1..h - 1 {
            for x in 0..w {
                let idx = buf.idx(x, y);
                if buf.terrain[idx] != 1 { continue; }
                if buf.koppen[idx] == 22 { continue; } // never on ice caps
                // Hard temperature gate: nothing on perennially frozen ground,
                // regardless of KÃ¶ppen label (catches glaciated highland / ice shelf).
                if buf.temperature[idx] < -8.0 { continue; }
                if !harsh(buf.koppen[idx]) { continue; }
                // A coastal post needs an UNFROZEN adjacent sea cell â€” no posts on a
                // frozen, ice-locked shore (sea ice is rendered for ocean < 1Â°C).
                let coast = buf.distance_to_ocean[idx] < 0.05
                    && [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| {
                        let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                        buf.terrain[ni] == 0 && buf.temperature[ni] >= 1.0
                    });   // whaling / fishing / port
                let oasis = river_near[idx];                      // desert caravan oasis
                let ore = buf.elevation[idx] > 0.40;             // mountain mining lode
                let volc = buf.is_volcanic[idx] != 0;            // volcanic minerals
                let draw = (if coast { 0.6 } else { 0.0 })
                    + (if oasis { 0.7 } else { 0.0 })
                    + (if ore { 0.5 } else { 0.0 })
                    + (if volc { 0.4 } else { 0.0 });
                if draw <= 0.0 { continue; }
                cand.push((idx, draw));
            }
        }
        cand.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut placed: Vec<(u32, u32)> = settlements.iter().map(|s| (s.x, s.y)).collect();
        let start_n = settlements.len();
        for (idx, draw) in cand {
            if settlements.len() - start_n >= max_outposts { break; }
            let sx = (idx % w as usize) as u32;
            let sy = (idx / w as usize) as u32;
            let far = placed.iter().all(|&(ex, ey)| {
                let mut dx = (sx as i32 - ex as i32).abs();
                if dx > w as i32 / 2 { dx = w as i32 - dx; }
                let dy = sy as i32 - ey as i32;
                dx * dx + dy * dy >= out_min_dist * out_min_dist
            });
            if !far { continue; }
            placed.push((sx, sy));
            let population = (60.0 + 340.0 * draw.min(1.0)) as u32; // tiny: 60..400
            let name = crate::sim::names::gen_name_epithet(sx, sy, w, h, 0);
            let oi = settlements.len() - start_n;
            settlements.push(Settlement {
                id: format!("o-{}", oi),
                x: sx,
                y: sy,
                name,
                size: "outpost".to_string(),
                population,
                score: (draw.min(1.0) * 0.3).max(0.05),
                culture: crate::sim::names::culture_label(sx, sy, w, h).to_string(),
                region: crate::sim::names::region_name(sx, sy, w, h),
                site: site_label(buf, idx, river_near[idx]).to_string(),
            });
        }
    }

    // Hard cap: guarantee the total never exceeds the user's explicit limit (the primary
    // sites come first / highest-priority, so truncation drops only marginal tail sites).
    if let Some(c) = explicit_cap { settlements.truncate(c); }

    settlements
}

/// PORTS_JUNCTIONS_AND_PROVINCE_VIEW_PLAN.md slice 8 â€” cost-grid BETWEENNESS: a
/// real, settlement-independent measure of which land actually sits on the paths
/// people would take, found the way the Gotthard and the Khyber are found â€” by
/// where routes squeeze through, not by knowing anything about cities or goods.
/// Sample K seed cells spread over the land, run Dijkstra from each over a coarse
/// cost grid, and accumulate how often a cell lies on the shortest path between two
/// seeds. High traversal = a genuine junction, independent of 3b's local saddle/
/// strait geometry tests â€” it catches a pinch point those miss (a chain of modest
/// saddles that is nonetheless the only way through) and confirms the ones they do.
///
/// Deliberately a SEPARATE, simpler cost grid from the campaign's own
/// `build_coarse_cost` (`query_commands/mod.rs`) â€” worldgen has no settlements yet
/// to route between and no rivers-as-JSON, and this only needs to rank land by
/// "does traffic squeeze through here", not price a real trade lane. Coarsened to
/// the same ~700-cell-wide grid the campaign's own route cost grid targets, which is
/// what keeps K=64 seeds a few seconds' work rather than a per-cell full-resolution
/// scan (Â§8.9 rule 1's spirit â€” this runs once per world, but a world can be 26M
/// cells on "Large").
///
/// Deterministic (no RNG): the K seeds are a stratified scatter over the land cells
/// in INDEX order, not a random sample, so the same world always yields the same
/// betweenness field.
pub fn compute_betweenness(buf: &WorldBuffer) -> Vec<f32> {
    let w = buf.width;
    let h = buf.height;
    let f = (w / 700).max(1);
    let cw = ((w + f - 1) / f) as i32;
    let ch = ((h + f - 1) / f) as i32;
    let cn = (cw * ch) as usize;
    let total = buf.total();
    if cn == 0 { return vec![0.0; total]; }

    let mut is_land = vec![false; cn];
    let mut cost = vec![1.0f32; cn];
    for cy in 0..ch {
        for cx in 0..cw {
            let wx = (cx as u32 * f + f / 2).min(w - 1);
            let wy = (cy as u32 * f + f / 2).min(h - 1);
            let idx = buf.idx(wx, wy);
            let ci = (cy * cw + cx) as usize;
            let land = buf.terrain[idx] == 1;
            is_land[ci] = land;
            // Only LAND cost matters — the Dijkstra below never expands into a sea
            // cell at all (see `DIRS` relaxation), so this value is never read for
            // a sea cell. "Where does traffic squeeze through" is a question about
            // a continuous landmass (the Gotthard, the Khyber); letting paths cut
            // across open water between two blobs of the same coastline would
            // route straight past the land corridor connecting them, which is
            // exactly backwards for what this measure exists to find.
            cost[ci] = 4.0 + buf.elevation[idx] * 14.0;
        }
    }
    let wrap_cx = |x: i32| -> i32 { ((x % cw) + cw) % cw };
    let cidx = |cx: i32, cy: i32| -> usize { (cy * cw + wrap_cx(cx)) as usize };

    const K: usize = 64;
    let land_cells: Vec<usize> = (0..cn).filter(|&i| is_land[i]).collect();
    if land_cells.len() < 4 { return vec![0.0; total]; } // too little land to mean anything
    let k = K.min(land_cells.len());
    let seeds: Vec<usize> = (0..k).map(|i| land_cells[(i * land_cells.len()) / k]).collect();

    const DIRS: [(i32, i32, f32); 8] = [
        (-1, 0, 1.0), (1, 0, 1.0), (0, -1, 1.0), (0, 1, 1.0),
        (-1, -1, 1.4142), (1, -1, 1.4142), (-1, 1, 1.4142), (1, 1, 1.4142),
    ];

    let mut traversal = vec![0u32; cn];
    for &s in &seeds {
        // Single-source Dijkstra â†’ predecessor array (one run serves every OTHER
        // seed as a destination), mirroring `coarse_dijkstra_prev`'s own shape in
        // `query_commands/mod.rs` â€” fixed-point (Ă—100) integer costs on a binary
        // heap, not floats, for the same determinism reason that code uses them.
        let mut dist = vec![i64::MAX; cn];
        let mut prev = vec![usize::MAX; cn];
        let mut heap: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        dist[s] = 0;
        heap.push(Reverse((0, s)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if d > dist[u] { continue; }
            let ux = (u as i32) % cw;
            let uy = (u as i32) / cw;
            for &(dx, dy, mult) in &DIRS {
                let ny = uy + dy;
                if ny < 0 || ny >= ch { continue; }
                let v = cidx(ux + dx, ny);
                if !is_land[v] { continue; } // land-only graph — see the cost comment above
                let step = ((cost[u] + cost[v]) * 0.5 * mult * 100.0) as i64;
                let nd = d.saturating_add(step.max(1));
                if nd < dist[v] { dist[v] = nd; prev[v] = u; heap.push(Reverse((nd, v))); }
            }
        }
        // Trace the shortest path from every OTHER seed back to this one, marking
        // every coarse cell it crosses â€” the traversal count IS the betweenness.
        for &t in &seeds {
            if t == s || prev[t] == usize::MAX { continue; }
            let mut cur = t;
            let mut guard = 0usize;
            while cur != s && guard < cn {
                traversal[cur] += 1;
                cur = prev[cur];
                guard += 1;
            }
        }
    }

    let tmax = (traversal.iter().copied().max().unwrap_or(0)).max(1) as f32;
    let coarse_norm: Vec<f32> = traversal.iter().map(|&t| t as f32 / tmax).collect();

    // Upsample coarse â†’ world resolution (nearest coarse cell), land only.
    let mut out = vec![0.0f32; total];
    for y in 0..h {
        let cy = ((y / f) as i32).min(ch - 1);
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            let cx = ((x / f) as i32).min(cw - 1);
            out[idx] = coarse_norm[cidx(cx, cy)];
        }
    }
    out
}

/// PORTS_JUNCTIONS_AND_PROVINCE_VIEW_PLAN.md slice 3d â€” step 7a: the settlement-
/// independent JUNCTION sites `compute_habitability_fields`'s trade ladder can now
/// name (straits, isthmuses, mountain passes, great river mouths) but which the BASE
/// pass (`generate_settlements`, local maxima of the full `hab` blend) can miss
/// entirely â€” a river mouth beside better farmland is not a local maximum of `hab`,
/// so it is discarded before the trade weight or the spacing rule are ever consulted
/// (F2). Hormuz had no fresh water and no farmland at all; nothing in the base pass
/// could ever place it.
///
/// Runs AFTER `generate_settlements` (so `existing` can be skipped) and BEFORE
/// `sim_generate_provinces` (so the province partition seeds from the complete set,
/// not twice). Additive and BOUNDED (`TRADE_SITES_MAX`) â€” the safety property that
/// keeps this a handful of towns, not a flood along every coastline. Every site is
/// sized modestly from the junction alone, never a food catchment â€” these sites
/// exist because traffic passes, not because the land feeds them;
/// `compute_political` (step 9) re-ranks by 0.30 route-centrality once real routes
/// exist and gives them their real standing.
///
/// Slice 8 folds `compute_betweenness` into the candidate SCORE, with no reordering
/// of the algorithm above: a cell still needs to be a local maximum of the combined
/// score and clear the same kind of threshold, but the combined score can now also
/// admit a real geographic pinch point 3b's local saddle/strait tests miss â€” a chain
/// of only-moderate saddles that is nonetheless the sole way through a range.
pub fn generate_trade_sites(
    buf: &WorldBuffer,
    trade: &[f32],
    existing: &[Settlement],
    realism: f32,
) -> Vec<Settlement> {
    let w = buf.width;
    let h = buf.height;
    let d = realism.clamp(0.0, 1.0);
    // Mirrors `generate_settlements`' own spacing/cap formula exactly, so a trade
    // site's "close to its market town" distance is stated relative to the SAME base
    // spacing the ordinary pass used, not a second, independent number.
    let min_dist = ((w as f32 / (95.0 + 90.0 * (1.0 - d))) as u32).max(3) as i32;
    let max_settlements = (180.0 + 820.0 * d) as usize;

    // A port sits close to its market town â€” Ostia/Rome, Piraeus/Athens â€” precedent:
    // `generate_settlements`' own `river_min_dist = min_dist * 0.5`.
    let port_min_dist = ((min_dist as f32 * 0.6) as i32).max(2);
    // A handful, not a flood â€” the bound that makes this slice additive.
    let trade_sites_max = 24usize.min((max_settlements / 20).max(1));

    const TRADE_SITE_MIN: f32 = 0.75;
    // Slice 8 â€” a pure geographic pinch point (no coastal/river trade value of its
    // own) still needs to stand well clear of ordinary land to qualify on
    // betweenness alone; the weight then nudges ranking among cells that already
    // clear `TRADE_SITE_MIN` on the ladder, without ever being able to promote
    // ordinary land past it by itself (weight â‰¤ the gap between an ordinary
    // harbour's 0.60 rung and the 0.75 floor).
    const BETWEENNESS_SITE_MIN: f32 = 0.85;
    const BETWEENNESS_WEIGHT: f32 = 0.12;

    let betweenness = compute_betweenness(buf);
    let combined: Vec<f32> = (0..trade.len())
        .map(|i| trade[i] + BETWEENNESS_WEIGHT * betweenness.get(i).copied().unwrap_or(0.0))
        .collect();

    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for y in 1..h - 1 {
        for x in 0..w {
            let idx = buf.idx(x, y);
            if buf.terrain[idx] != 1 { continue; }
            // Belt-and-braces: `compute_habitability_fields`'s own `cryo_gate` already
            // zeroes `trade` on an EF ice cap, but state the rule explicitly so it
            // can never regress silently if the gate weighting ever changes.
            if buf.koppen[idx] == 22 { continue; }
            if trade[idx] < TRADE_SITE_MIN && betweenness[idx] < BETWEENNESS_SITE_MIN { continue; }
            let score = combined[idx];
            let is_max = [(-1i32, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
                .iter()
                .all(|&(dx, dy)| {
                    let ni = buf.widx(x as i32 + dx, y as i32 + dy);
                    score >= combined[ni]
                });
            if is_max { candidates.push((idx, score)); }
        }
    }
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let port_min_dist2 = port_min_dist * port_min_dist;
    let mut placed: Vec<(u32, u32)> = existing.iter().map(|s| (s.x, s.y)).collect();
    let mut out: Vec<Settlement> = Vec::new();
    'outer: for (idx, score) in &candidates {
        if out.len() >= trade_sites_max { break; }
        let sx = (*idx % w as usize) as u32;
        let sy = (*idx / w as usize) as u32;
        for &(ex, ey) in &placed {
            let mut dx = (sx as i32 - ex as i32).abs();
            if dx > w as i32 / 2 { dx = w as i32 - dx; }
            let dy = sy as i32 - ey as i32;
            if dx * dx + dy * dy < port_min_dist2 { continue 'outer; }
        }
        placed.push((sx, sy));
        // Seeded modestly from the junction alone â€” size comes later, from routes.
        let population = (300.0 + 4200.0 * score.clamp(0.0, 1.0)) as u32;
        let size = if population >= 5_000 { "town" } else { "village" };
        let name = crate::sim::names::gen_name_epithet(sx, sy, w, h, 0);
        out.push(Settlement {
            id: format!("t-{}", out.len()),
            x: sx, y: sy, name,
            size: size.to_string(),
            population,
            score: *score,
            culture: crate::sim::names::culture_label(sx, sy, w, h).to_string(),
            region: crate::sim::names::region_name(sx, sy, w, h),
            site: "port".to_string(),
        });
    }
    out
}

#[cfg(test)]
mod trade_site_tests {
    use super::*;
    use crate::sim::world_buffer::ColumnSet;
    use crate::db::schema;
    use rusqlite::Connection;

    fn tiny_buf(w: u32, h: u32) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", &w.to_string()), ("grid_height", &h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap()
    }

    /// A synthetic world: a narrow desert isthmus of land between two open seas,
    /// far from any farmland. The BASE pass must place nothing on it (no climate,
    /// fertility or water score to speak of); the TRADE pass must place exactly one
    /// town there, because it is a strait/isthmus chokepoint by pure geometry.
    #[test]
    fn a_strait_town_appears_where_no_farm_would() {
        // A wide world so `generate_trade_sites`' own spacing (derived from world
        // width) comfortably exceeds the isthmus's own footprint below — otherwise
        // the spacing rule (correctly) tiles a long plateau of tied maxima into
        // several sites instead of collapsing it to one, which is a property of
        // the greedy spacing, not of the strait detector this test targets.
        let w = 1200u32;
        let h = 200u32;
        let mut buf = tiny_buf(w, h);
        // All sea, except a single-row land isthmus at x in [598,602), y = 100 —
        // narrow enough that sea sits within `choke_r` on both north AND south —
        // and a chunk of ordinary land far to the west (so the base pass has SOME
        // farmland to place towns on, ruling out "no land generated at all").
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                buf.terrain[idx] = 0;
                buf.elevation[idx] = 0.05;
                buf.koppen[idx] = 4; // BWh hot desert everywhere (no farmland draw)
                buf.temperature[idx] = 25.0;
                buf.precipitation[idx] = 30.0;
                buf.fertility[idx] = 0.02;
                buf.distance_to_ocean[idx] = 0.5;
                buf.disease_risk[idx] = 0;
            }
        }
        let isthmus_y = h / 2;
        for x in 598..602u32 {
            let idx = buf.idx(x, isthmus_y);
            buf.terrain[idx] = 1;
            buf.distance_to_ocean[idx] = 0.0;
        }
        let rivers: Vec<River> = Vec::new();
        let lakes: Vec<Lake> = Vec::new();
        let base_hab = compute_habitability(&buf, &rivers, &lakes);
        let base = generate_settlements(&buf, &base_hab, &rivers, 1, 0.55, None);
        assert!(
            base.iter().all(|s| !(598..602).contains(&s.x)),
            "the base pass placed a settlement on bare desert isthmus land"
        );

        let fields = compute_habitability_fields(&buf, &rivers, &lakes, None);
        let trade_sites = generate_trade_sites(&buf, &fields.trade, &base, 0.55);
        let on_isthmus: Vec<_> = trade_sites.iter().filter(|s| (598..602).contains(&s.x)).collect();
        assert_eq!(on_isthmus.len(), 1, "expected exactly one strait town on the isthmus, got {}", on_isthmus.len());
        assert_eq!(on_isthmus[0].site, "port");
    }

    /// No trade site may ever land on Köppen EF (ice cap), on any world â€” the same
    /// discipline `cryo_gate` already enforces for ordinary settlements.
    #[test]
    fn trade_sites_respect_the_cryosphere() {
        let w = 200u32;
        let h = 100u32;
        let mut buf = tiny_buf(w, h);
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                // A narrow polar isthmus (would otherwise register as a strait) sitting
                // entirely on an EF ice cap.
                let on_isthmus = (98..102).contains(&x);
                buf.terrain[idx] = if on_isthmus { 1 } else { 0 };
                buf.koppen[idx] = 22; // EF ice cap
                buf.temperature[idx] = -35.0;
                buf.elevation[idx] = 0.4;
                buf.distance_to_ocean[idx] = if on_isthmus { 0.0 } else { 0.5 };
                buf.fertility[idx] = 0.0;
            }
        }
        let rivers: Vec<River> = Vec::new();
        let lakes: Vec<Lake> = Vec::new();
        let fields = compute_habitability_fields(&buf, &rivers, &lakes, None);
        assert!(fields.trade.iter().all(|&t| t == 0.0), "trade must be zero everywhere on an EF world");
        let trade_sites = generate_trade_sites(&buf, &fields.trade, &[], 0.55);
        assert!(trade_sites.is_empty(), "a trade site was placed on Köppen EF ice cap");
    }

    /// The count must stay bounded even on a world RICH in candidates — a handful of
    /// towns, never a flood along every coastline. A comb of dozens of narrow land
    /// segments (each independently a strait/isthmus by the same geometry test the
    /// first gate exercises) manufactures far more raw candidates than
    /// `TRADE_SITES_MAX` allows, so the cap is exercised, not merely unreached.
    #[test]
    fn trade_sites_are_bounded() {
        let w = 900u32;
        let h = 60u32;
        let mut buf = tiny_buf(w, h);
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                // 4-wide land segments separated by 4-wide sea gaps, the whole way
                // around the world — a comb of ~110 independent strait candidates.
                let land = (x % 8) < 4;
                buf.terrain[idx] = if land { 1 } else { 0 };
                buf.koppen[idx] = 6; // BSh steppe: some trade draw, little farmland pull
                buf.temperature[idx] = 20.0;
                buf.precipitation[idx] = 200.0;
                buf.fertility[idx] = 0.05;
                buf.elevation[idx] = 0.05;
                buf.distance_to_ocean[idx] = if land { 0.02 } else { 0.5 };
                buf.disease_risk[idx] = 0;
            }
        }
        let rivers: Vec<River> = Vec::new();
        let lakes: Vec<Lake> = Vec::new();
        let fields = compute_habitability_fields(&buf, &rivers, &lakes, None);
        let base = generate_settlements(&buf, &fields.hab, &rivers, 1, 0.55, None);
        let trade_sites = generate_trade_sites(&buf, &fields.trade, &base, 0.55);
        let max_settlements = (180.0 + 820.0 * 0.55) as usize;
        let cap = 24usize.min((max_settlements / 20).max(1));
        assert!(trade_sites.len() <= cap, "{} trade sites exceeds the bound {cap}", trade_sites.len());
        assert_eq!(trade_sites.len(), cap, "a candidate-rich world should actually bind the cap, not merely satisfy it");
    }

    /// The slice is additive: `generate_settlements`' own output must be bit-
    /// identical whether or not the trade pass runs afterward.
    #[test]
    fn the_base_settlement_set_is_unchanged() {
        let w = 200u32;
        let h = 100u32;
        let mut buf = tiny_buf(w, h);
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                buf.terrain[idx] = if (20..180).contains(&x) { 1 } else { 0 };
                buf.koppen[idx] = 8; // Csa Mediterranean, the cradle climate
                buf.temperature[idx] = 18.0;
                buf.precipitation[idx] = 700.0;
                buf.fertility[idx] = 0.6;
                buf.elevation[idx] = 0.15;
                buf.distance_to_ocean[idx] = if (20..180).contains(&x) { 0.2 } else { 0.0 };
            }
        }
        let rivers: Vec<River> = Vec::new();
        let lakes: Vec<Lake> = Vec::new();
        let hab_before = compute_habitability(&buf, &rivers, &lakes);
        let base_before = generate_settlements(&buf, &hab_before, &rivers, 42, 0.55, None);

        // Run the trade pass — must not read back into `buf` or mutate it.
        let fields = compute_habitability_fields(&buf, &rivers, &lakes, None);
        let _ = generate_trade_sites(&buf, &fields.trade, &base_before, 0.55);

        let hab_after = compute_habitability(&buf, &rivers, &lakes);
        let base_after = generate_settlements(&buf, &hab_after, &rivers, 42, 0.55, None);
        assert_eq!(base_before.len(), base_after.len());
        for (a, b) in base_before.iter().zip(base_after.iter()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
            assert_eq!(a.population, b.population);
        }
    }

    /// Slice 8 — a "dumbbell": two large landmasses joined by one thin corridor, all
    /// at IDENTICAL flat climate/fertility/elevation and far from any coast, river or
    /// lake, so `compute_habitability_fields`'s trade ladder scores every cell the
    /// same (the default 0.1 rung — no water feature anywhere to distinguish them)
    /// and 3b's local strait/pass tests never fire (the corridor is flat, and its
    /// middle sits well outside `choke_r` of the open sea at its short ends). The
    /// corridor is nonetheless the ONLY way between the two landmasses — exactly
    /// the "chain of only-moderate saddles" case the ladder cannot see and
    /// betweenness is built to catch.
    #[test]
    fn betweenness_finds_a_pinch_point_the_ladder_missed() {
        let w = 1500u32;
        let h = 150u32;
        let mut buf = tiny_buf(w, h);
        for y in 0..h {
            for x in 0..w {
                let idx = buf.idx(x, y);
                buf.terrain[idx] = 0;
                buf.elevation[idx] = 0.10; // flat — never clears the pass threshold (0.33)
                buf.koppen[idx] = 8; // Csa — a comfortable, unremarkable climate
                buf.temperature[idx] = 18.0;
                buf.precipitation[idx] = 700.0;
                buf.fertility[idx] = 0.30;
                // Uniformly "not near a coast" so `near_coast`/`is_strait` can't
                // distinguish a blob cell that happens to touch real sea from a
                // corridor cell that doesn't — the ladder must be genuinely blind.
                buf.distance_to_ocean[idx] = 0.30;
                buf.disease_risk[idx] = 0;
            }
        }
        let land = |buf: &mut WorldBuffer, x0: u32, x1: u32, y0: u32, y1: u32| {
            for y in y0..y1 {
                for x in x0..x1 {
                    let idx = buf.idx(x, y);
                    buf.terrain[idx] = 1;
                }
            }
        };
        land(&mut buf, 0, 600, 0, h);         // blob A
        land(&mut buf, 600, 800, 65, 85);     // the one corridor (20 cells tall)
        land(&mut buf, 800, 1400, 0, h);      // blob B
        // x in [1400,1500) stays sea — a buffer so the cylindrical wrap doesn't
        // silently rejoin blob B to blob A around the back of the world.

        let rivers: Vec<River> = Vec::new();
        let lakes: Vec<Lake> = Vec::new();
        let fields = compute_habitability_fields(&buf, &rivers, &lakes, None);

        // The ladder really is blind: no cell in the corridor scores higher than a
        // cell deep in either blob.
        let corridor_trade = fields.trade[buf.idx(700, 74)];
        let blob_trade = fields.trade[buf.idx(300, 74)];
        assert!(
            (corridor_trade - blob_trade).abs() < 1e-6,
            "the trade ladder should score the corridor and the open blob identically \
             (corridor {corridor_trade}, blob {blob_trade}) — otherwise this isn't testing \
             what betweenness adds"
        );

        let betweenness = compute_betweenness(&buf);
        // The PEAK anywhere in the corridor (traffic may spread across its several
        // parallel rows) against a point deep in a blob's own far corner — clear of
        // any natural convergence toward the corridor mouth, so it reads as ordinary
        // open interior rather than as a second bottleneck of the blob's own making.
        let corridor_btw = (600..800).flat_map(|x| (65..85).map(move |y| (x, y)))
            .map(|(x, y)| betweenness[buf.idx(x, y)])
            .fold(0.0f32, f32::max);
        let blob_btw = betweenness[buf.idx(50, 10)];
        assert!(
            corridor_btw > blob_btw * 3.0 && corridor_btw > 0.05,
            "the corridor should read as far higher betweenness than a blob's own open \
             interior (corridor peak {corridor_btw}, blob corner {blob_btw})"
        );

        let base = generate_settlements(&buf, &fields.hab, &rivers, 1, 0.55, None);
        let trade_sites = generate_trade_sites(&buf, &fields.trade, &base, 0.55);
        assert!(
            trade_sites.iter().any(|s| (600..800).contains(&s.x)),
            "betweenness should have placed a trade site in the corridor — none of \
             {} sites did: {:?}",
            trade_sites.len(),
            trade_sites.iter().map(|s| (s.x, s.y)).collect::<Vec<_>>()
        );
    }
}



