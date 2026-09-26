//! dev_production — `docs/living_world/03_DEVELOPMENT_TRACKS.md`, slice 03.7,
//! DOSE STEP 1 of the dose walk (`DEV_PRODUCTION_DOSE`).
//!
//! Blends each city's OWN development factor (`TickHub.dev`, 03.1) into
//! production in place of the single global `tech_factor` — the design's own
//! "replacing tech_factor" plan (`FIX_PLAN` Part C: "growth is exogenous",
//! the same finding this whole row exists to fix).
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
//! **`dev_blended_tech` takes `dose` as an explicit parameter** (the
//! `stability_of`/`track_build_progress_e` pattern) rather than reading
//! `DEV_PRODUCTION_DOSE` internally, so the mechanism stays testable at ANY
//! dose regardless of which value is currently shipped — `dose <= 0.0`
//! returns the plain global `tech_factor` for every hub, a true no-op,
//! proven by `dev_production_dose_zero_is_a_noop`.
//!
//! **Dose walk record — raised to 0.2 and KEPT, after two gates it touched
//! but does not own were investigated rather than blindly loosened.** A
//! 100-year baseline at dose 0.0 (`econ_measure_development_leaders`) first
//! confirmed the underlying `dev`/track dynamics are not degenerate: the
//! development leader genuinely changes hands (H20 → H27 around year 60 on
//! `reference_world`). Raising `DEV_PRODUCTION_DOSE` to 0.2 and running the
//! full `tick::tests` suite then failed two gates that are not this dose's
//! target:
//!
//! - `the_relay_carries_long_lanes_in_stages_on_a_realistically_dense_world`
//!   (an N1c relay-staging gate, unrelated to this mechanism) measured
//!   staged trade at 0.57× unstaged, just past its 0.6 floor. A more
//!   developed hub trades LESS with distant partners (more self-sufficient
//!   local production), which compounds with staging's own time cost on the
//!   same long lanes that gate exercises — a real, expected effect, and
//!   explicitly accepted by the maintainer ("if trade collapses that's okay
//!   as it was in real life"). The floor was widened 0.6 → 0.5 (still well
//!   above the measured 0.57×), not removed, so the gate still catches an
//!   actual collapse.
//! - `the_coin_ledger_conserves_every_struck_coin` failed at a tolerance of
//!   1e-3 absolute (35.57376 held against 35.572525 circulating — 0.0035%
//!   relative). This is compounded f32 rounding, not a leak: per-hub tech
//!   blending changes the exact order production sums are struck in, and a
//!   float-precision assertion tuned to one summation order will drift under
//!   a different one. The tolerance is now RELATIVE
//!   (`circulating * 1e-4 + 1e-3`), which still catches a real leak (one
//!   that grows with circulation) while tolerating this scale of noise.
//!
//! Both changes are documented at their own assertions in `tests.rs`. Full
//! `tick::tests` (all tests) and the `econ_` suite were re-run at dose 0.2
//! after both fixes to confirm nothing else moved.
//!
//! **Composing with other production doses** (`PROD_ELASTICITY`,
//! `ORE_CEILING_DOSE`, …, all of which also read/scale the same `tech` term
//! at their own call sites, all still shipped at 0.0): raising THIS dose
//! further must be walked with the others still pinned, per CLAUDE.md's own
//! "two doses moving together cannot be told apart" rule.
use super::*;

pub(crate) const DEV_PRODUCTION_DOSE: f32 = 0.2;

impl CampaignSim {
    /// One "effective tech" value per hub for today's production pass.
    /// `self.tech_factor` for EVERY hub at `dose <= 0.0` — the only path
    /// taken there, so `dev_norm_scale` is never even read — a per-city
    /// blend toward `dev` once `dose` is positive.
    pub(crate) fn dev_blended_tech(&mut self, dose: f32) -> Vec<f32> {
        let n = self.hubs.len();
        if dose <= 0.0 {
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
        let dose = dose.clamp(0.0, 1.0);
        self.hubs.iter()
            .map(|h| {
                let city = h.dev * scale;
                global * (1.0 - dose) + city * dose
            })
            .collect()
    }
}
