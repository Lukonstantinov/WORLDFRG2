//! dev_production — `docs/living_world/03_DEVELOPMENT_TRACKS.md`, slice 03.7,
//! DOSE STEP 1 of the dose walk (`DEV_PRODUCTION_DOSE`).
//!
//! Blends each city's OWN development factor (`TickHub.dev`, 03.1) into
//! production in place of the single global `tech_factor` — the design's own
//! "replacing tech_factor" plan (`FIX_PLAN` Part C: "growth is exogenous",
//! the same finding this whole row exists to fix). At `DEV_PRODUCTION_DOSE =
//! 0.0` (shipped) every hub reads the plain global `tech_factor`, unchanged —
//! a true no-op, proven by `dev_production_dose_zero_is_a_noop`.
//!
//! **Normalisation, per the design doc's own text**: "the city factor fed to
//! production is scaled so that the population-weighted world mean equals
//! the old value" — `dev` is seeded around 1.0 (`development.rs`'s own
//! seeding) while `tech_factor` reads near `TECH_FACTOR_FLOOR` (0.85) once a
//! campaign has run a few years (a PRE-EXISTING, already-measured finding —
//! `roll_events`' setbacks outweigh `tech_factor`'s own 1.5%/yr growth
//! within about six years). Feeding raw `dev` into production unscaled would
//! jump every city's output the instant the dose is raised.
//! `CampaignSim.dev_norm_scale` is calibrated ONCE, lazily (since `dev` isn't
//! seeded until `update_development`'s first yearly call, so it cannot be
//! calibrated at tick 0), as `tech_factor / population_weighted_mean(dev)`
//! at the moment it is first needed — so a freshly-dosed campaign's
//! production is CONTINUOUS with what it was a moment before, and cities
//! diverge from that shared start only as their own `dev` moves from there.
//!
//! **Composing with other production doses** (`PROD_ELASTICITY`,
//! `ORE_CEILING_DOSE`, …, all of which also read/scale the same `tech` term
//! at their own call sites): raising this dose must be walked with the
//! others pinned at their shipped values, per CLAUDE.md's own "two doses
//! moving together cannot be told apart" rule — `dev_blended_tech` touches
//! nothing else, so that discipline is the caller's responsibility, not
//! this function's.
use super::*;

pub(crate) const DEV_PRODUCTION_DOSE: f32 = 0.0;

impl CampaignSim {
    /// One "effective tech" value per hub for today's production pass.
    /// `self.tech_factor` for EVERY hub at dose 0 — the only path taken at
    /// the shipped dose, so `dev_norm_scale` is never even read there — a
    /// per-city blend toward `dev` once the dose is raised.
    pub(crate) fn dev_blended_tech(&mut self) -> Vec<f32> {
        let n = self.hubs.len();
        if DEV_PRODUCTION_DOSE <= 0.0 {
            return vec![self.tech_factor; n];
        }
        if self.dev_norm_scale <= 0.0 {
            let mut wsum = 0.0f32;
            let mut psum = 0.0f32;
            for h in &self.hubs {
                if h.is_estate || h.abandoned { continue; }
                let p = h.population.max(0.0);
                wsum += h.dev * p;
                psum += p;
            }
            let mean = if psum > EPS { wsum / psum } else { 0.0 };
            if mean > EPS {
                self.dev_norm_scale = self.tech_factor / mean;
            } else {
                // `dev` has not been seeded yet this campaign (before
                // `update_development`'s first yearly call) — fall back to
                // the plain global value rather than calibrate on a
                // meaningless all-zero mean.
                return vec![self.tech_factor; n];
            }
        }
        let scale = self.dev_norm_scale;
        let global = self.tech_factor;
        let dose = DEV_PRODUCTION_DOSE.clamp(0.0, 1.0);
        self.hubs.iter()
            .map(|h| {
                let city = h.dev * scale;
                global * (1.0 - dose) + city * dose
            })
            .collect()
    }
}
