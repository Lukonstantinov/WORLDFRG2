//! One-dimensional diffusive energy-balance model (EBM) for the zonal-mean
//! surface temperature — the planet's *energy budget*, which the fabricated
//! latitude curve never had.
//!
//! This is the classic North–Budyko model (North, *J. Atmos. Sci.* 1975):
//! at steady state the absorbed shortwave, the outgoing longwave and the
//! meridional heat transport balance in every latitude band,
//!
//! ```text
//!   d/dx[ D (1−x²) dT/dx ]  =  A + B·T  −  I(x)·a(x,T)          x = sin φ
//!   └─ meridional transport ┘   └ outgoing ┘   └ absorbed shortwave ┘
//! ```
//!
//! * `I(x)` — annual-mean top-of-atmosphere insolation for the world's **axial
//!   tilt** (from `insolation.rs`) scaled by its **stellar luminosity**. Because
//!   `I` comes from the real astronomical integral, raising the obliquity
//!   redistributes annual-mean sunshine toward the poles — the physics the old
//!   "seasonal range only" temperature model dropped, which is why cranking the
//!   tilt slider used to change the seasons but not the annual-mean climate.
//! * `A + B·T` — outgoing longwave (Budyko linearization). The **greenhouse**
//!   knob lowers `A` → the surface must warm to radiate the same energy.
//! * `a(x,T)` — co-albedo, with an ice-albedo step so a cold enough band turns
//!   reflective (the snowball feedback) and a warm world melts its caps.
//! * `D` — meridional diffusivity (heat transport). **Rotation** throttles it:
//!   a fast rotator confines eddies zonally → weaker poleward transport →
//!   steeper equator-pole gradient; a slow rotator mixes heat efficiently → flat.
//!
//! ## Anomaly coupling (why Earth can't regress)
//!
//! We never feed the EBM's *absolute* temperature into the pipeline. We solve it
//! twice — once for the world's parameters, once for Earth's — and hand
//! `temperature.rs` only the **difference** `T_world(φ) − T_earth(φ)`, which it
//! adds onto the existing Earth-calibrated base curve. At Earth settings the
//! difference is exactly zero, so every downstream field (Köppen, biology,
//! settlements) is untouched and the Earth calibration is preserved bit-for-bit;
//! move a knob and the climate shifts by the physically-correct amount.

use super::insolation;

// ── Model constants (tuned so Earth's parameters reproduce a ~15 °C global mean
// and a ~27 °C equator / ~−25 °C pole gradient) ─────────────────────────────
/// Outgoing-longwave intercept A (W/m²) at Earth's greenhouse. `OLR = A + B·T`.
const OLR_A: f32 = 201.0;
/// Outgoing-longwave slope B (W/m²/°C).
const OLR_B: f32 = 2.09;
/// Meridional diffusivity at Earth's rotation (W/m²/°C, in the x = sinφ operator).
const DIFF_D0: f32 = 0.60;
/// Co-albedo (1 − albedo) of an ice-free surface+atmosphere column.
const COALB_WARM: f32 = 0.68;
/// Co-albedo of an ice/snow-covered column (high albedo → low co-albedo).
const COALB_ICE: f32 = 0.38;
/// Temperature (°C) about which the ice-albedo transition is centred.
const T_ICE: f32 = -8.0;
/// Half-width (°C) of the smooth ice-albedo transition.
const ICE_SOFT: f32 = 4.0;
/// W/m² removed from `A` per unit of greenhouse above Earth's (→ warming). Sized
/// so greenhouse 2× ≈ +12 °C global mean and greenhouse 0 collapses to a
/// snowball, without either running away.
const GREENHOUSE_SENS: f32 = 25.0;
/// Exponent coupling rotation rate to transport: `D = D0 · rotation^(−ROT_EXP)`.
const ROT_EXP: f32 = 0.7;

/// Number of latitude bands in the internal solver grid (uniform in x = sinφ).
const N: usize = 180;

/// Solve the steady-state zonal-mean temperature (°C) on the internal grid for a
/// given planet. `rotation` is × Earth; `solar_lum` × Earth's solar constant;
/// `greenhouse` × Earth's; `obliquity` in degrees.
fn solve_profile(obliquity: f32, solar_lum: f32, greenhouse: f32, rotation: f32) -> [f32; N] {
    let dx = 2.0 / N as f32;
    // Band centres in x = sinφ and the annual-mean insolation there.
    let mut x = [0.0f32; N];
    let mut ins = [0.0f32; N];
    for i in 0..N {
        let xi = -1.0 + (i as f32 + 0.5) * dx;
        x[i] = xi;
        let phi_deg = xi.clamp(-1.0, 1.0).asin().to_degrees();
        ins[i] = insolation::insolation_stats(phi_deg, obliquity).0 * solar_lum;
    }
    // Face diffusivities k_{i+1/2} = D·(1 − x_face²); the polar faces (x = ±1)
    // vanish, giving no-flux boundaries automatically.
    let d = DIFF_D0 * rotation.clamp(0.1, 10.0).powf(-ROT_EXP);
    let mut kface = [0.0f32; N + 1]; // kface[i] = flux coeff on the i-1/2 face
    for f in 0..=N {
        let xf = -1.0 + f as f32 * dx;
        kface[f] = d * (1.0 - xf * xf).max(0.0);
    }
    let a_eff = OLR_A - GREENHOUSE_SENS * (greenhouse - 1.0);

    // Outer fixed-point loop over the (temperature-dependent) ice-albedo.
    let mut t = [15.0f32; N];
    for _ in 0..40 {
        // Co-albedo from the current temperature (smooth ice step).
        let mut rhs = [0.0f32; N];
        for i in 0..N {
            let f = 0.5 * (1.0 + ((t[i] - T_ICE) / ICE_SOFT).tanh()); // 0 cold → 1 warm
            let coalb = COALB_ICE + (COALB_WARM - COALB_ICE) * f;
            rhs[i] = ins[i] * coalb - a_eff;
        }
        // Tridiagonal system  sub·T_{i-1} + diag·T_i + sup·T_{i+1} = rhs.
        let inv_dx2 = 1.0 / (dx * dx);
        let mut sub = [0.0f32; N];
        let mut diag = [0.0f32; N];
        let mut sup = [0.0f32; N];
        for i in 0..N {
            let kl = kface[i] * inv_dx2; // i-1/2 face
            let kr = kface[i + 1] * inv_dx2; // i+1/2 face
            sub[i] = -kl;
            sup[i] = -kr;
            diag[i] = OLR_B + kl + kr;
        }
        let new = thomas(&sub, &diag, &sup, &rhs);
        // Relax toward the new solution and check convergence.
        let mut maxd = 0.0f32;
        for i in 0..N {
            maxd = maxd.max((new[i] - t[i]).abs());
            t[i] = new[i];
        }
        if maxd < 1e-3 {
            break;
        }
    }
    t
}

/// Thomas algorithm for a tridiagonal system (sub/diag/sup are the three bands).
fn thomas(sub: &[f32; N], diag: &[f32; N], sup: &[f32; N], rhs: &[f32; N]) -> [f32; N] {
    let mut c = [0.0f32; N];
    let mut d = [0.0f32; N];
    c[0] = sup[0] / diag[0];
    d[0] = rhs[0] / diag[0];
    for i in 1..N {
        let m = diag[i] - sub[i] * c[i - 1];
        c[i] = sup[i] / m;
        d[i] = (rhs[i] - sub[i] * d[i - 1]) / m;
    }
    let mut x = [0.0f32; N];
    x[N - 1] = d[N - 1];
    for i in (0..N - 1).rev() {
        x[i] = d[i] - c[i] * x[i + 1];
    }
    x
}

/// Sample a profile at latitude `lat_deg` by linear interpolation in x = sinφ.
fn sample(profile: &[f32; N], lat_deg: f32) -> f32 {
    let x = lat_deg.to_radians().sin().clamp(-1.0, 1.0);
    let dx = 2.0 / N as f32;
    let pos = ((x + 1.0) / dx - 0.5).clamp(0.0, (N - 1) as f32);
    let i0 = pos.floor() as usize;
    let i1 = (i0 + 1).min(N - 1);
    let frac = pos - i0 as f32;
    profile[i0] * (1.0 - frac) + profile[i1] * frac
}

/// The zonal-mean temperature anomaly `T_world(φ) − T_earth(φ)` (°C) that the
/// world's planetary state imposes on top of the Earth-calibrated base curve.
/// Built once (two EBM solves) and sampled per latitude in `temperature.rs`.
pub struct EbmAnomaly {
    world: [f32; N],
    earth: [f32; N],
}

impl EbmAnomaly {
    /// Solve the EBM for this world and for Earth reference parameters.
    pub fn for_world(obliquity: f32, solar_lum: f32, greenhouse: f32, rotation: f32) -> Self {
        EbmAnomaly {
            world: solve_profile(obliquity, solar_lum, greenhouse, rotation),
            earth: solve_profile(23.44, 1.0, 1.0, 1.0),
        }
    }

    /// Anomaly (°C) to add to the base curve at `lat_deg`. Exactly 0 everywhere
    /// when the world's parameters equal Earth's.
    pub fn sample(&self, lat_deg: f32) -> f32 {
        sample(&self.world, lat_deg) - sample(&self.earth, lat_deg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn global_mean(p: &[f32; N]) -> f32 {
        // Area-weighted mean is just the plain mean of the x-uniform grid.
        p.iter().sum::<f32>() / N as f32
    }

    #[test]
    fn earth_profile_is_earth_like() {
        let t = solve_profile(23.44, 1.0, 1.0, 1.0);
        let mean = global_mean(&t);
        assert!((mean - 15.0).abs() < 6.0, "Earth global mean {mean} not ~15 °C");
        let eq = sample(&t, 0.0);
        let pole = sample(&t, 89.0);
        assert!(eq > 20.0, "equator {eq} too cold");
        assert!(pole < -5.0, "pole {pole} too warm");
        assert!(eq - pole > 25.0, "equator-pole gradient {} too weak", eq - pole);
    }

    #[test]
    fn earth_params_give_zero_anomaly() {
        let a = EbmAnomaly::for_world(23.44, 1.0, 1.0, 1.0);
        for lat in [-80.0, -30.0, 0.0, 25.0, 55.0, 88.0] {
            assert!(a.sample(lat).abs() < 1e-3, "anomaly at {lat}° should be 0");
        }
    }

    #[test]
    fn more_greenhouse_warms_globally() {
        let cold = solve_profile(23.44, 1.0, 1.0, 1.0);
        let warm = solve_profile(23.44, 1.0, 2.0, 1.0);
        assert!(
            global_mean(&warm) > global_mean(&cold) + 6.0,
            "2× greenhouse should warm the planet substantially"
        );
    }

    #[test]
    fn brighter_star_warms_globally() {
        let base = solve_profile(23.44, 1.0, 1.0, 1.0);
        let bright = solve_profile(23.44, 1.15, 1.0, 1.0);
        assert!(global_mean(&bright) > global_mean(&base) + 3.0);
    }

    #[test]
    fn high_obliquity_warms_poles_relative_to_equator() {
        // Raising the tilt moves annual-mean sunshine poleward: the poles warm
        // and the tropics cool RELATIVE to Earth — the tilt-mean physics the old
        // model dropped. (Anomaly method makes this a clean differential test.)
        let a = EbmAnomaly::for_world(45.0, 1.0, 1.0, 1.0);
        let pole = a.sample(85.0);
        let eq = a.sample(0.0);
        assert!(pole > 3.0, "45° tilt should warm the pole (got {pole})");
        assert!(eq < pole, "tilt should warm poles more than the equator");
    }

    #[test]
    fn faster_rotation_steepens_gradient() {
        // Stronger Coriolis → weaker poleward transport → colder poles / warmer
        // tropics (steeper gradient) than a slow rotator.
        let slow = solve_profile(23.44, 1.0, 1.0, 0.5);
        let fast = solve_profile(23.44, 1.0, 1.0, 2.0);
        let grad = |p: &[f32; N]| sample(p, 0.0) - sample(p, 85.0);
        assert!(grad(&fast) > grad(&slow), "fast rotator should have a steeper gradient");
    }
}
