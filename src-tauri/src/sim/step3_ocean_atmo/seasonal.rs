use crate::sim::koppen::seasonal_range_base;
use super::ocean::belt_wind_shifted;
use super::circulation::Circulation;
use crate::sim::world_buffer::WorldBuffer;

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Two-season winds via a thermal landâ€“sea low/high (the surface consequence of
// the thermal-wind relation)
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// A single annual-mean wind field cannot reverse between seasons, so real monsoon
// behaviour (wet summer coast, dry winter, the reason a warm-sea desert coast like
// Somalia/Arabia stays arid) cannot emerge. We model two insolation states â€”
// boreal summer (`sun_sign = +1`, â‰ˆ July: NH warms, SH cools) and boreal winter
// (`sun_sign = -1`, â‰ˆ January) â€” and in each derive the surface wind as the belt
// wind PLUS a monsoon perturbation that blows down the seasonal pressure gradient
// toward the thermal low (hot land in summer â†’ onshore inflow; cold land in winter
// â†’ offshore outflow).
//
// The raw thermal-wind equation âˆ‚v_g/âˆ‚ln p = âˆ’(R_d/f)Â·kÃ—âˆ‡T gives vertical shear and
// is singular at the equator (fâ†’0), exactly the monsoon belt, so it can't be used
// directly in a single-layer model. Its SURFACE consequence â€” a thermal low over
// hot land driving cross-isobaric inflow â€” is what we implement, from fields we
// already have (temperature seasonal amplitude, distance_to_ocean, land geometry).

/// Monsoon-perturbation gain: converts the smoothed seasonal thermal anomaly's
/// spatial gradient (Â°C per sampling step) into a wind-vector contribution. Tuned so
/// a strong landâ€“sea contrast (a coast in high summer) can fully reverse the local
/// coastal wind (onshore monsoon) while a flat interior/open-ocean field leaves the
/// prevailing belt untouched.
const MONSOON_WIND_GAIN: f32 = 0.10;
/// Cap on the perturbation magnitude (in belt-wind units) so the monsoon can turn
/// the wind but the planetary belts still dominate the global circulation.
const MONSOON_WIND_CAP: f32 = 1.4;
/// Half-separation (cells) used to sample the thermal gradient â€” a couple of cells
/// so a one-cell-wide coast still yields a stable gradient after smoothing.
const GRAD_STEP: i32 = 2;
/// Seasonal ITCZ migration amplitude (degrees) toward the summer hemisphere. The
/// annual-mean land-asymmetry shift is added on top of this per season.
/// 10° is the working compromise: 8° left the SH-summer ITCZ too close to the
/// equator to reach southern tropical Africa (Zimbabwe/Zambia rainy season);
/// 12° pushed the SH-summer ITCZ past the equatorial Amazon (making that column's
/// SH-summer drier than its NH-summer → spurious AS dry-savanna classification).
pub const ITCZ_SEASONAL_MIGRATE: f32 = 10.0;

/// Seasonal displacement (degrees) of the WIND belts toward the summer hemisphere —
/// the migration that actually reverses a monsoon. See `ocean::belt_wind_shifted`.
pub const MONSOON_BELT_MIGRATE: f32 = 8.0;

/// How much further a summer-hemisphere CONTINENT pulls the ITCZ poleward, as a
/// multiple of the base migration at a fully-land column.
///
/// The modern view's division of labour: the migration is the driver, and land-sea
/// contrast selects the LONGITUDE at which the convergence zone reaches furthest
/// poleward (Chao & Chen 2001; Geen et al. 2020). Over the oceans Earth's ITCZ
/// reaches ~10°N in July; over Asia the monsoon trough reaches ~25-30°N, which is
/// why the Bay of Bengal reverses and the central Pacific at the same latitude
/// does not.
pub const MONSOON_LAND_PULL: f32 = 1.0;

/// Latitude (°) up to which the belts migrate at full amplitude, and the latitude
/// by which the migration has died away entirely.
///
/// The seasonal excursion is a TROPICAL phenomenon: the ITCZ and the Hadley cells
/// swing far, the polar front barely moves. Without this taper a uniform shift
/// pushed the 55°S Southern Ocean across `polar_front` and reversed the westerlies
/// there between seasons — measured, and flatly wrong.
pub const MIGRATE_FULL_LAT: f32 = 30.0;
pub const MIGRATE_ZERO_LAT: f32 = 50.0;

/// Per-column land pull (0..1): the summer hemisphere's subtropical land fraction
/// on each meridian. One O(w·h) sweep, no per-cell outward scan. Shared by the wind
/// field and by the ITCZ overlay query so the drawn convergence zone is exactly the
/// one the winds were built from.
pub fn itcz_land_pull(buf: &WorldBuffer, sun_sign: f32) -> Vec<f32> {
    let (w, h) = (buf.width, buf.height);
    let mut pull = vec![0.0f32; w as usize];
    let mut cnt = vec![0.0f32; w as usize];
    for y in 0..h {
        let lat = buf.latitude(y);
        // Summer hemisphere only, across the belt the monsoon trough occupies.
        if lat * sun_sign < 5.0 || lat.abs() > 35.0 { continue; }
        for x in 0..w {
            pull[x as usize] += (buf.terrain[buf.idx(x, y)] == 1) as i32 as f32;
            cnt[x as usize] += 1.0;
        }
    }
    for x in 0..w as usize {
        if cnt[x] > 0.0 { pull[x] /= cnt[x]; }
    }
    pull
}

/// Latitude of the ITCZ on column `x` for insolation state `sun_sign`, in degrees.
/// This IS the convergence zone the seasonal wind field is displaced about — the
/// overlay draws this, not a decorative curve.
pub fn itcz_latitude(land_pull_x: f32, sun_sign: f32) -> f32 {
    MONSOON_BELT_MIGRATE * sun_sign * (1.0 + MONSOON_LAND_PULL * land_pull_x)
}

/// A per-cell seasonal wind field for one insolation state.
pub struct SeasonalWind {
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
}

/// Seasonal near-surface temperature anomaly (Â°C, relative to the annual mean) at a
/// cell for insolation state `sun_sign` (+1 boreal summer, âˆ’1 boreal winter). Land
/// carries most of the swing (amplified in continental interiors); the ocean barely
/// moves. The landâ€“sea DIFFERENCE in this field is the monsoon's pressure engine:
/// warm anomaly â†’ thermal low, cool anomaly â†’ thermal high.
pub fn season_temp_anomaly(buf: &WorldBuffer, x: u32, y: u32, sun_sign: f32) -> f32 {
    let idx = buf.idx(x, y);
    let lat = buf.latitude(y);
    // Local-summer sign: in boreal summer the NH is in summer (+), the SH in winter.
    let local_summer = sun_sign * if lat >= 0.0 { 1.0 } else { -1.0 };
    let amp = 0.5 * seasonal_range_base(lat.abs());
    if buf.terrain[idx] == 1 {
        // Continentality: interiors swing far more than maritime coasts.
        let cont = if buf.distance_to_ocean.is_empty() {
            0.8
        } else {
            0.55 + 0.45 * buf.distance_to_ocean[idx].clamp(0.0, 1.0)
        };
        local_summer * amp * cont
    } else {
        // Ocean thermal inertia damps the swing to a small fraction.
        local_summer * amp * 0.12
    }
}

/// Build the two-dimensional seasonal wind field for one insolation state.
///
/// 1. Build the thermal-anomaly field and smooth it (pressure fields are smooth).
/// 2. Per cell, take the belt wind and add a perturbation pointing down the pressure
///    gradient (toward the warm/low-pressure side), Coriolis-rotated so mid-latitude
///    inflow spirals into the low while equatorial inflow is straight cross-isobaric.
pub fn compute_seasonal_wind(buf: &WorldBuffer, sun_sign: f32) -> SeasonalWind {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();

    // â”€â”€ Thermal-anomaly (âˆ âˆ’pressure) field, then smooth â”€â”€
    let mut anom = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            anom[buf.idx(x, y)] = season_temp_anomaly(buf, x, y, sun_sign);
        }
    }
    // A few isotropic passes: the pressure response to heating is broad and smooth,
    // and smoothing keeps the gradient stable across a one-cell coastline.
    let mut a = anom;
    let mut b = vec![0.0f32; n];
    for _ in 0..3 {
        for y in 0..h {
            for x in 0..w {
                let mut sum = a[buf.idx(x, y)] * 2.0;
                let mut cnt = 2.0f32;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    sum += a[buf.idx(buf.wrap_x(x as i32 + dx), ny as u32)];
                    cnt += 1.0;
                }
                b[buf.idx(x, y)] = sum / cnt;
            }
        }
        std::mem::swap(&mut a, &mut b);
    }
    let anom = a;

    // â”€â”€ Belt wind + monsoon perturbation â”€â”€
    let circ = Circulation::for_world(buf);
    // Displace the whole circulation toward the summer hemisphere. This — not the
    // thermal-low perturbation below — is what reverses a monsoon wind. Before it
    // existed, all seven of the real Earth's monsoon sites changed heading by at
    // most 8° between January and July, so the model ran two seasons of rain on one
    // season of wind (`earth_diagnose_seasonal_wind_reversal`).
    let land_pull = itcz_land_pull(buf, sun_sign);
    let mut vx = vec![0.0f32; n];
    let mut vy = vec![0.0f32; n];
    for y in 0..h {
        let lat = buf.latitude(y);
        // The excursion is tropical: full amplitude to MIGRATE_FULL_LAT, gone by
        // MIGRATE_ZERO_LAT, so the mid-latitude westerlies keep their heading.
        let al = lat.abs();
        let taper = if al <= MIGRATE_FULL_LAT { 1.0 } else {
            ((MIGRATE_ZERO_LAT - al) / (MIGRATE_ZERO_LAT - MIGRATE_FULL_LAT)).max(0.0)
        };
        // Coriolis rotation of the cross-isobaric inflow: 0 at the equator (pure
        // down-gradient), growing to a partial deflection by mid-latitudes. Sign is
        // hemisphere-dependent (surface air spirals into a low counter-clockwise in
        // the NH, clockwise in the SH).
        let f = (lat.abs() / 45.0).clamp(0.0, 1.0);
        let theta = f * 0.8 * if lat >= 0.0 { 1.0 } else { -1.0 };
        let (st, ct) = theta.sin_cos();
        for x in 0..w {
            let i = buf.idx(x, y);
            let (bvx, bvy) = belt_wind_shifted(
                lat, &circ, itcz_latitude(land_pull[x as usize], sun_sign) * taper,
            );
            // âˆ‡anom via a fixed-step central difference (points toward warmer / lower
            // pressure â€” the direction surface air is drawn).
            let gx = sample_anom(&anom, buf, x as i32 + GRAD_STEP, y as i32)
                - sample_anom(&anom, buf, x as i32 - GRAD_STEP, y as i32);
            let gy = sample_anom(&anom, buf, x as i32, y as i32 + GRAD_STEP)
                - sample_anom(&anom, buf, x as i32, y as i32 - GRAD_STEP);
            // Rotate the down-gradient inflow by the Coriolis angle.
            let px = (gx * ct - gy * st) * MONSOON_WIND_GAIN;
            let py = (gx * st + gy * ct) * MONSOON_WIND_GAIN;
            let plen = (px * px + py * py).sqrt();
            let (px, py) = if plen > MONSOON_WIND_CAP {
                (px / plen * MONSOON_WIND_CAP, py / plen * MONSOON_WIND_CAP)
            } else {
                (px, py)
            };
            let mut rvx = bvx + px;
            let mut rvy = bvy + py;
            // Renormalize to a unit heading (advection reads direction, speed lives in
            // the jets field), keeping the belt where the perturbation is negligible.
            let m = (rvx * rvx + rvy * rvy).sqrt();
            if m > 0.05 {
                rvx /= m;
                rvy /= m;
            } else {
                rvx = bvx;
                rvy = bvy;
            }
            vx[i] = rvx;
            vy[i] = rvy;
        }
    }
    SeasonalWind { vx, vy }
}

#[inline]
fn sample_anom(anom: &[f32], buf: &WorldBuffer, x: i32, y: i32) -> f32 {
    let yy = buf.clamp_y(y);
    anom[buf.idx(buf.wrap_x(x), yy)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;
    use crate::sim::world_buffer::ColumnSet;
    use rusqlite::Connection;

    /// A hot summer continent beside a cool ocean must bend the coastal wind ONSHORE
    /// in summer and OFFSHORE in winter â€” the monsoon reversal.
    #[test]
    fn thermal_low_reverses_coastal_wind() {
        let (w, h) = (16u32, 160u32);
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::PHASE_OCEAN_ATMOSPHERE).unwrap();
        // NH land band (lat > 8), SH open ocean; a coast around lat 8-10.
        for y in 0..h {
            let lat = buf.latitude(y);
            for x in 0..w {
                let i = buf.idx(x, y);
                let land = lat > 8.0;
                buf.terrain[i] = if land { 1 } else { 0 };
                buf.distance_to_ocean[i] = if land {
                    ((lat - 8.0) / 60.0).clamp(0.0, 1.0)
                } else { 0.0 };
            }
        }
        // A subtropical coastal row (just inside the land, warm interior to its north).
        let row = (0..h).min_by(|&a, &b| {
            (buf.latitude(a) - 14.0).abs().partial_cmp(&(buf.latitude(b) - 14.0).abs()).unwrap()
        }).unwrap();
        let x = w / 2;

        let summer = compute_seasonal_wind(&buf, 1.0); // NH summer
        let winter = compute_seasonal_wind(&buf, -1.0); // NH winter
        let i = buf.idx(x, row);
        // North is âˆ’y here (latitude increases as y decreases). Summer flow should
        // gain a northward (onshore, toward the hot interior) component vs winter.
        assert!(
            summer.vy[i] < winter.vy[i],
            "summer wind should turn more onshore (âˆ’y) than winter: summer {} winter {}",
            summer.vy[i], winter.vy[i]
        );
    }
}


