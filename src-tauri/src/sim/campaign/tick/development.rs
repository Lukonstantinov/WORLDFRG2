//! development — `docs/living_world/03_DEVELOPMENT_TRACKS.md`, slices
//! 03.1-03.2.
//!
//! A per-city development factor: grows from real trade volume, partner
//! reach and welfare (scaled by STABILITY, 03.2), decays from starvation/
//! isolation (unscaled — a collapsing city loses ground regardless of how
//! calm it otherwise is), and diffuses from trade partners (pulling only UP,
//! never down — a rich neighbour never impoverishes a backwater's own
//! knowledge; also unscaled by stability, per the design's own formula,
//! since a shaken city can still absorb an idea from a calm neighbour).
//! Computed yearly by `update_development`, called from `disease.rs`'s
//! yearly block right before `record_city_annals` so this year's value is
//! what the annal snapshots.
//!
//! **This slice is read by nothing.** `TickHub.dev`/`dev_breakdown` are
//! written and nothing else in the tick consults them — `DEV_PRODUCTION_
//! DOSE` (03.7) is what will eventually blend a city's own factor into its
//! production in place of the global `tech_factor`, and the four tracks
//! (03.3) are what will eventually consume `stability_of` for point gains
//! and losses. Until then this is pure bookkeeping, which is exactly why
//! `sim_fingerprint` (folding only `stock`/`price`/`population`/`treasury`/
//! `export_earn`/`import_spend`) is provably untouched by it — no dose flag
//! is needed for a mechanism nothing reads yet.
//!
//! **Forward hooks read neutral until their row exists** (00_INDEX rule):
//! the "scholars/schools/universities" and "cultural acceptance" sources the
//! design names both need data rows 05/06 don't yet supply, so their terms
//! are simply absent from `DevBreakdown` rather than wired to an invented
//! stand-in — the doc's own table marks them "0 contribution until then".
//! `stability_of`'s own two inputs — legitimacy (04) and deadlock (04) —
//! read the same way: `NEUTRAL_LEGITIMACY`/`NEUTRAL_DEADLOCK` stand in until
//! row 04 gives a city (as opposed to a realm — `Realm.legitimacy` already
//! exists and is a DIFFERENT, realm-scale quantity) its own real reading.
use super::*;
use super::individuals::living_world_salts as salts;

/// Source-order breakdown of one year's `dev` change, always freshly
/// computed (never accumulated) so it reads as "this year's story".
pub(crate) const DEV_TRADE: usize = 0;
pub(crate) const DEV_PARTNERS: usize = 1;
pub(crate) const DEV_WELFARE: usize = 2;
pub(crate) const DEV_DIFFUSION: usize = 3;
pub(crate) const DEV_DECAY: usize = 4;

/// `dev` reads as zero on a fresh hub or an old save; seed it to a small,
/// deliberately unequal starting value (`hash01`-jittered around 1.0) rather
/// than a uniform 1.0 for every city — a dead-flat starting line would make
/// the first few years of diffusion meaningless (nothing to pull toward).
pub(crate) fn dev_needs_seeding(dev: f32) -> bool {
    dev < EPS
}

/// Doubling trade adds a fixed STEP to the trade term, not a doubling of it —
/// `ln(1 + volume/ref)`, so a great entrepôt's advantage over an ordinary
/// market is real but bounded.
const DEV_TRADE_REF: f32 = 20_000.0;
const DEV_TRADE_WEIGHT: f32 = 0.014;
/// Partner reach is `neighbours reachable / NEIGHBOR_K`, i.e. how much of the
/// hub's own trade shortlist is filled — Miletus's advantage was contact, not
/// size, so this rewards a wide network even before any of it is wealthy.
const DEV_PARTNER_WEIGHT: f32 = 0.006;
/// Welfare above/below the "comfortable" reading (Allen ≈ 2.0, already
/// rescaled to 1.0 by `welfare_opportunity_e`'s own convention) nudges
/// development a little either way — a starving city cannot build libraries.
const DEV_WELFARE_WEIGHT: f32 = 0.004;
/// Diffusion pulls a city's factor toward its BEST-off trade partner, scaled
/// by how much of its trade actually reaches that partner's own network
/// (approximated here, at this slice, by `1 / neighbour_count` — a uniform
/// share of attention across a hub's shortlist).
const DEV_DIFFUSION_RATE: f32 = 0.05;
/// Starvation and outright famine erode a city's development a little each
/// year it persists — plague deaths do not, on the reading that knowledge
/// dies with starving people leaving, not with people dying suddenly.
const DEV_DECAY_STARVATION: f32 = 0.02;
/// A city with NO reachable trade partners at all loses ground slowly (post-
/// Roman Britain) — isolation, not any one shock.
const DEV_DECAY_ISOLATION: f32 = 0.01;
/// A soft ceiling so no city runs away forever: growth (before decay) is
/// scaled down as `dev` approaches it, never clamped outright (a hard clamp
/// would make two great cities indistinguishable at the top).
const DEV_SOFT_CEILING: f32 = 4.0;

// ── 03.2 · stability, "one formula, used everywhere" ────────────────────
/// Slice 03.2's own bounds: 0.2 (chaos) to 1.2 (golden age), per the design
/// doc verbatim.
pub(crate) const STABILITY_MIN: f32 = 0.2;
pub(crate) const STABILITY_MAX: f32 = 1.2;
const STABILITY_BASE: f32 = 1.0;
/// A city with no realm reads a comfortable, self-governing default rather
/// than either extreme — `stability_is_bounded`'s own fixture (no realm, no
/// war, no damage, no unrest) should read as an ordinary, unremarkable town.
const NEUTRAL_LEGITIMACY: f32 = 0.75;
/// Row 04 has no deadlock mechanism yet (edicts/votes); a neutral reading is
/// "no deadlock", i.e. zero cost — the forward-hook convention (00_INDEX).
const NEUTRAL_DEADLOCK: f32 = 0.0;
const STABILITY_LEGIT_WEIGHT: f32 = 0.3;
const STABILITY_UNREST_WEIGHT: f32 = 0.5;
const STABILITY_WAR_PENALTY: f32 = 0.15;
const STABILITY_DAMAGE_WEIGHT: f32 = 0.3;
const STABILITY_DEADLOCK_WEIGHT: f32 = 0.2;

/// The one stability formula "used everywhere" (design doc's own words):
/// legitimacy, unrest, war, recent structural damage and deadlock, blended
/// from a base of 1.0 and clamped to `[STABILITY_MIN, STABILITY_MAX]`. A
/// PURE function (never reads `self` directly) so 03.3's track-point math and
/// 03.4's building-cost math can call it identically without duplicating the
/// formula — the whole point of "one formula".
pub(crate) fn stability_of(legitimacy: f32, unrest: f32, at_war: bool, damage: f32, deadlock: f32) -> f32 {
    let mut s = STABILITY_BASE;
    s += (legitimacy - NEUTRAL_LEGITIMACY) * STABILITY_LEGIT_WEIGHT;
    s -= unrest.clamp(0.0, 1.0) * STABILITY_UNREST_WEIGHT;
    if at_war { s -= STABILITY_WAR_PENALTY; }
    s -= damage.clamp(0.0, 1.0) * STABILITY_DAMAGE_WEIGHT;
    s -= deadlock.clamp(0.0, 1.0) * STABILITY_DEADLOCK_WEIGHT;
    s.clamp(STABILITY_MIN, STABILITY_MAX)
}

impl CampaignSim {
    /// `stability_of` fed this hub's real, currently-available inputs.
    /// `legitimacy`/`deadlock` read their neutral constants (00_INDEX forward
    /// hooks) until row 04 gives a CITY (as opposed to a realm) its own real
    /// values for either.
    pub(crate) fn stability_at(&self, h: usize) -> f32 {
        let hub = &self.hubs[h];
        let unrest = hub.society.unrest;
        let at_war = hub.war_with >= 0;
        stability_of(NEUTRAL_LEGITIMACY, unrest, at_war, hub.damage, NEUTRAL_DEADLOCK)
    }

    /// One year of `TickHub.dev` for every settled (non-estate) hub. Reads
    /// `self.neighbors` (already built by `rebuild_neighbors`) for partner
    /// reach and diffusion, and each hub's own `trade_last_year`/
    /// `welfare_ratio`/`starving` for the rest — no new per-hub scan beyond
    /// what a yearly pass already costs (§ the row's own performance budget,
    /// 00_INDEX: "under 1% of the tick").
    pub(crate) fn update_development(&mut self, year: u32) {
        let n = self.hubs.len();
        // Seed pass first, so diffusion below reads real (if fresh) values
        // for every neighbour, not a mix of seeded and un-seeded hubs.
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            if dev_needs_seeding(self.hubs[h].dev) {
                let jitter = hash01(self.seed, self.hubs[h].id as u64, salts::DEV_SEED);
                self.hubs[h].dev = 0.85 + jitter * 0.3; // ≈ the old tech_factor floor, ± a real spread
            }
        }
        // Diffusion needs EVERY hub's current dev before any of them change
        // this year, so read the whole vector once up front.
        let cur_dev: Vec<f32> = self.hubs.iter().map(|h| h.dev).collect();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let neighbours = self.neighbors.get(h).map(|v| v.as_slice()).unwrap_or(&[]);
            // 03.2 · the design's own formula scales the SOURCE terms by
            // stability (`Σ source_i × modifier_i × stability`); diffusion and
            // decay sit OUTSIDE that product and are never stability-scaled.
            let stability = self.stability_at(h);
            let trade_term = (1.0 + self.hubs[h].trade_last_year.max(0.0) / DEV_TRADE_REF).ln() * DEV_TRADE_WEIGHT * stability;
            let partner_frac = (neighbours.len() as f32 / NEIGHBOR_K as f32).min(1.0);
            let partner_term = partner_frac * DEV_PARTNER_WEIGHT * stability;
            let wr = self.hubs[h].welfare_ratio;
            let welfare_term = if wr > 0.0 { (wr / 2.0 - 1.0).clamp(-1.0, 1.0) * DEV_WELFARE_WEIGHT * stability } else { 0.0 };
            // Diffusion — only ever pulls UP, toward the best-connected
            // partner's own dev, spread thin across however many partners
            // this hub has (an isolated hub's one link matters more).
            let diffusion_term = if neighbours.is_empty() {
                0.0
            } else {
                let share = 1.0 / neighbours.len() as f32;
                let mut pulled = 0.0f32;
                for &nb in neighbours {
                    let nb = nb as usize;
                    if nb >= cur_dev.len() { continue; }
                    let gap = (cur_dev[nb] - cur_dev[h]).max(0.0);
                    pulled += gap * share * DEV_DIFFUSION_RATE;
                }
                pulled
            };
            let mut decay_term = 0.0f32;
            if self.hubs[h].starving > 0.3 { decay_term -= DEV_DECAY_STARVATION; }
            if neighbours.is_empty() { decay_term -= DEV_DECAY_ISOLATION; }

            let raw_growth = trade_term + partner_term + welfare_term + diffusion_term + decay_term;
            // Soft ceiling: positive growth is damped as dev nears it; decay
            // (already negative) is never damped, so a great city can still
            // fall.
            let headroom = (1.0 - (self.hubs[h].dev / DEV_SOFT_CEILING).clamp(0.0, 1.0)).max(0.05);
            let growth = if raw_growth > 0.0 { raw_growth * headroom } else { raw_growth };

            self.hubs[h].dev = (self.hubs[h].dev + growth).max(0.1);
            self.hubs[h].dev_breakdown = [trade_term, partner_term, welfare_term, diffusion_term, decay_term];
        }
        let _ = year; // reserved for a future per-year diagnostic (03.7's econ_measure_development_leaders)
    }
}
