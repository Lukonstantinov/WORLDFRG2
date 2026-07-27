//! The planetary general circulation — the belts of prevailing wind and pressure
//! whose latitudes are set by the planet's **rotation rate**, not by fiat.
//!
//! On Earth the three-cell structure (Hadley / Ferrel / Polar) puts the
//! trade-wind → westerly boundary near 30° and the westerly → polar-easterly
//! boundary near 60°. Those latitudes are a consequence of Coriolis: a faster
//! rotator confines the overturning to a narrow, banded circulation (belts nearer
//! the equator, more cells); a slower rotator lets a single wide Hadley cell reach
//! toward the pole (belts pushed poleward, deserts and storm tracks with them).
//! This module derives the belt latitudes from the rotation rate (with a mild
//! thermal-driving dependence on the planet's warmth) and hands them to the wind,
//! current and precipitation code, which used to hard-code 30° / 60°.
//!
//! ## Zero regression at Earth
//!
//! At Earth's parameters `hadley_edge == 30` and `polar_front == 60` *exactly*, so
//! `belt_scale()` / `front_scale()` are 1 and every consumer sees the latitudes it
//! saw before. The Held–Hou-style width scaling `φ_H ∝ Ω^(−0.65)` only bends the
//! belts once the rotation rate leaves 1×.

use crate::sim::world_buffer::WorldBuffer;

/// Rotation exponent for the Hadley-cell edge. Held–Hou gives φ_H ∝ Ω^(−1); the
/// real atmosphere is gentler (eddies broaden the cell), so we use ~0.65.
const HADLEY_ROT_EXP: f32 = 0.65;
/// Rotation exponent for the Ferrel/Polar (storm-track) boundary — flatter still.
const FRONT_ROT_EXP: f32 = 0.40;
/// Earth Hadley-edge latitude (trade → westerly boundary).
const EARTH_HADLEY: f32 = 30.0;
/// Earth polar-front latitude (westerly → polar-easterly boundary / storm track).
const EARTH_FRONT: f32 = 60.0;

/// The general-circulation belt geometry for a world.
#[derive(Debug, Clone, Copy)]
pub struct Circulation {
    /// Latitude of the descending branch of the Hadley cell — the subtropical
    /// high / trade-→-westerly boundary. ~30° on Earth.
    pub hadley_edge: f32,
    /// Latitude of the polar front — the westerly-→-polar-easterly boundary and
    /// the core of the mid-latitude storm track. ~60° on Earth.
    pub polar_front: f32,
    /// Number of meridional circulation cells per hemisphere (3 on Earth).
    pub cells: u32,
    /// Sign of the planet's rotation relative to Earth's: `+1.0` prograde
    /// (Earth-like, the default), `-1.0` retrograde. Belt LATITUDE only depends
    /// on rotation rate magnitude (Held–Hou scaling), so this never touches
    /// `hadley_edge`/`polar_front`/`cells` above — it exists purely for
    /// downstream wind/current code to flip the Coriolis-deflection DIRECTION
    /// (the zonal trade/westerly/polar-easterly components) without disturbing
    /// the unrelated hemisphere-based (N/S) sign used for seasonal logic.
    pub rotation_sign: f32,
}

impl Circulation {
    /// Derive the circulation from a world's planetary state.
    pub fn for_world(buf: &WorldBuffer) -> Self {
        Self::from_params(buf.rotation_rate, buf.solar_lum, buf.greenhouse)
    }

    /// Earth's circulation (belts at exactly 30° / 60°).
    pub fn earth() -> Self {
        Self::from_params(1.0, 1.0, 1.0)
    }

    fn from_params(rotation_rate: f32, solar_lum: f32, greenhouse: f32) -> Self {
        // Belt LATITUDE depends only on rotation MAGNITUDE (Held–Hou scaling);
        // direction is carried separately in `rotation_sign` below.
        let rot = rotation_rate.abs().clamp(0.1, 10.0);
        let rotation_sign = if rotation_rate < 0.0 { -1.0 } else { 1.0 };
        // Warmth widens the Hadley cell slightly (stronger thermal driving). Earth
        // = 1 → no effect, so the belts stay at 30/60 exactly.
        let warmth = (solar_lum * greenhouse).clamp(0.4, 2.5);
        let hadley_edge =
            (EARTH_HADLEY * rot.powf(-HADLEY_ROT_EXP) * warmth.powf(0.10)).clamp(12.0, 70.0);
        let polar_front =
            (EARTH_FRONT * rot.powf(-FRONT_ROT_EXP)).clamp(hadley_edge + 8.0, 87.0);
        // Cells per hemisphere: Earth's 30° Hadley edge → 3 cells (Hadley, Ferrel,
        // Polar). A wide single Hadley cell (slow rotator) collapses toward 1.
        let cells = ((90.0 / hadley_edge).round() as u32).max(1);
        Circulation { hadley_edge, polar_front, cells, rotation_sign }
    }

    /// Subtropical belt scale: how far the Hadley/desert structure has moved
    /// relative to Earth (1.0 at Earth). Latitudes divided by this map a world
    /// latitude back into its Earth-equivalent "belt-space" latitude.
    #[inline]
    pub fn belt_scale(&self) -> f32 {
        self.hadley_edge / EARTH_HADLEY
    }

    /// Storm-track / westerly belt scale (1.0 at Earth).
    #[inline]
    pub fn front_scale(&self) -> f32 {
        self.polar_front / EARTH_FRONT
    }

    /// Map a true latitude to its Earth-equivalent latitude for the subtropical
    /// (Hadley, trade-wind, subtropical-high, monsoon, ITCZ) structure.
    #[inline]
    pub fn belt_lat(&self, lat: f32) -> f32 {
        lat / self.belt_scale()
    }

    /// Map a true latitude to its Earth-equivalent latitude for the mid-latitude
    /// (westerly / storm-track / frontal) structure.
    #[inline]
    pub fn front_lat(&self, lat: f32) -> f32 {
        lat / self.front_scale()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_belts_are_exactly_30_60() {
        let c = Circulation::earth();
        assert!((c.hadley_edge - 30.0).abs() < 1e-3, "hadley {}", c.hadley_edge);
        assert!((c.polar_front - 60.0).abs() < 1e-3, "front {}", c.polar_front);
        assert_eq!(c.cells, 3);
        assert!((c.belt_scale() - 1.0).abs() < 1e-4);
        assert!((c.front_scale() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn slow_rotator_widens_hadley_and_fewer_cells() {
        let slow = Circulation::from_params(0.5, 1.0, 1.0);
        assert!(slow.hadley_edge > 40.0, "slow rotator Hadley {} too narrow", slow.hadley_edge);
        assert!(slow.cells < 3, "slow rotator should have fewer cells, got {}", slow.cells);
        assert!(slow.belt_scale() > 1.0);
    }

    #[test]
    fn fast_rotator_narrows_belts() {
        let fast = Circulation::from_params(2.0, 1.0, 1.0);
        assert!(fast.hadley_edge < 25.0, "fast rotator Hadley {} too wide", fast.hadley_edge);
        assert!(fast.cells >= 4, "fast rotator should have more cells, got {}", fast.cells);
    }

    #[test]
    fn retrograde_matches_prograde_belts_but_flips_sign() {
        let pro = Circulation::from_params(1.0, 1.0, 1.0);
        let retro = Circulation::from_params(-1.0, 1.0, 1.0);
        assert!((pro.hadley_edge - retro.hadley_edge).abs() < 1e-4);
        assert!((pro.polar_front - retro.polar_front).abs() < 1e-4);
        assert_eq!(pro.cells, retro.cells);
        assert_eq!(pro.rotation_sign, 1.0);
        assert_eq!(retro.rotation_sign, -1.0);
    }
}
