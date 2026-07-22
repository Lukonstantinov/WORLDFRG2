//! Daily-mean top-of-atmosphere insolation and its seasonal cycle.
//!
//! This is the astronomical driver for the **energy-balance seasonality** in
//! `temperature.rs`: instead of a fabricated latitude "seasonal range", the
//! seasonal temperature swing is derived from how much the incoming sunlight at a
//! latitude varies over the year, attenuated by surface heat capacity.
//!
//! Formulas are the standard ones (Hartmann, *Global Physical Climatology*, ch. 2):
//!   * ecliptic longitude of the sun `λ` grows ~linearly through the year, 0 at the
//!     spring (vernal) equinox;
//!   * solar declination `δ = asin(sin ε · sin λ)` for obliquity `ε` — valid for
//!     ANY tilt, so a fantasy world can be re-tilted from 0° (no seasons) to >45°
//!     (the tropics reach the poles);
//!   * sunset hour angle `H0 = acos(−tanφ·tanδ)`, clamped so polar day gives the
//!     whole day of sun and polar night gives none;
//!   * daily-mean insolation `Q = (S0/π)(H0 sinφ sinδ + cosφ cosδ sinH0)`.
//!
//! The one thing the seasonal model needs from here is, per latitude, the annual
//! MEAN insolation and the seasonal AMPLITUDE (half of max−min over the year).

use std::f32::consts::PI;

/// Solar constant (W/m²).
const S0: f32 = 1361.0;

/// Fraction of the year from Jan 1 to the spring equinox (~day 80 of 365.24).
const SPRING_FRAC: f32 = 80.0 / 365.24;

/// Solar declination (radians) at a fraction `year_frac ∈ [0,1)` of the year for a
/// given obliquity (radians). 0 at the spring equinox, +ε at NH summer solstice.
#[inline]
pub fn declination(year_frac: f32, obliquity_rad: f32) -> f32 {
    let lambda = 2.0 * PI * (year_frac - SPRING_FRAC); // ecliptic longitude of the sun
    (obliquity_rad.sin() * lambda.sin()).asin()
}

/// Daily-mean top-of-atmosphere insolation (W/m²) at latitude `lat_rad` for a solar
/// declination `decl_rad`. Handles polar day (sun never sets) and polar night.
#[inline]
pub fn daily_mean_insolation(lat_rad: f32, decl_rad: f32) -> f32 {
    let cos_h0 = -lat_rad.tan() * decl_rad.tan();
    let h0 = if cos_h0 <= -1.0 {
        PI // polar day: sun above the horizon the whole day
    } else if cos_h0 >= 1.0 {
        0.0 // polar night: sun never rises
    } else {
        cos_h0.acos()
    };
    let q = (S0 / PI)
        * (h0 * lat_rad.sin() * decl_rad.sin() + lat_rad.cos() * decl_rad.cos() * h0.sin());
    q.max(0.0)
}

/// Annual-mean insolation and seasonal amplitude `(mean, amp)` in W/m² at a latitude
/// (degrees) for a given obliquity (degrees). `amp = (max − min)/2` over 12 months —
/// the quantity that sets the seasonal temperature swing. Cheap enough to call once
/// per grid row.
pub fn insolation_stats(lat_deg: f32, obliquity_deg: f32) -> (f32, f32) {
    let lat_rad = lat_deg.to_radians();
    let eps = obliquity_deg.to_radians();
    let mut sum = 0.0f32;
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for m in 0..12 {
        let yf = (m as f32 + 0.5) / 12.0;
        let q = daily_mean_insolation(lat_rad, declination(yf, eps));
        sum += q;
        lo = lo.min(q);
        hi = hi.max(q);
    }
    (sum / 12.0, (hi - lo) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equator_gets_more_annual_sun_than_pole() {
        let (eq_mean, _) = insolation_stats(0.0, 23.44);
        let (pole_mean, _) = insolation_stats(88.0, 23.44);
        assert!(eq_mean > pole_mean, "equator {eq_mean} should out-sun pole {pole_mean}");
    }

    #[test]
    fn seasonal_amplitude_grows_poleward() {
        let (_, eq_amp) = insolation_stats(2.0, 23.44);
        let (_, mid_amp) = insolation_stats(45.0, 23.44);
        let (_, hi_amp) = insolation_stats(70.0, 23.44);
        assert!(eq_amp < mid_amp, "equator amp {eq_amp} < mid amp {mid_amp}");
        assert!(mid_amp < hi_amp, "mid amp {mid_amp} < high amp {hi_amp}");
    }

    #[test]
    fn zero_obliquity_removes_seasons() {
        // No tilt → no seasonal cycle anywhere.
        for lat in [0.0, 30.0, 60.0, 85.0] {
            let (_, amp) = insolation_stats(lat, 0.0);
            assert!(amp < 1.0, "no-tilt world should have ~0 seasonal amplitude at {lat}°, got {amp}");
        }
    }

    #[test]
    fn higher_tilt_means_stronger_high_latitude_seasons() {
        let (_, earth) = insolation_stats(60.0, 23.44);
        let (_, extreme) = insolation_stats(60.0, 45.0);
        assert!(extreme > earth, "45° tilt {extreme} should beat 23° tilt {earth} at 60°");
    }
}
