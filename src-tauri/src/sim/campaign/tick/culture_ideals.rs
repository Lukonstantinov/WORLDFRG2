//! culture_ideals — `docs/living_world/03_DEVELOPMENT_TRACKS.md`, slice 03.5.
//!
//! Three PURE, DERIVED reads, never stored (the design doc's own words for
//! culture development: "derived, never stored" — extended here to ideal and
//! ideal-score too, since nothing yet needs either to persist or decay):
//!
//! - `culture_development` — the population-weighted mean of `TickHub.dev`
//!   (03.1) across a culture's own cities.
//! - `culture_ideal` — what a culture admires, read straight off its REAL,
//!   already-existing trait list (`culture_trait_ids`, `sim::cultures::
//!   TRAITS`) via the design doc's own trait→ideal table — the culture's
//!   most characteristic trait (index 0 of `culture_trait_ids`, which is
//!   already ordered "most characteristic first") decides its one ideal.
//! - `culture_ideal_score` — how well a culture is doing on its OWN ideal,
//!   read from the matching development TRACK's (03.3) points across its
//!   cities — conquest reads Military, wealth reads Trade, learning reads
//!   Ideological, stability reads Civil. A culture whose ideal has no track
//!   counterpart (lineage, tradition, purity, assimilation, reach — none of
//!   which rows 02-03 measure yet) reads `0.0`, an honest absence rather
//!   than an invented number.
//!
//! `is_barbarian_to`/`admires_more_developed` are the JUDGEMENT the design
//! doc names — "B's development, or B's score on A's ideal, is far below
//! A's" / the reverse — built and tested here, but **called by nothing**:
//! applying a barbarian judgement to a city's default acceptance tier is row
//! 05's job, and turning it into a casus belli is row 09's. Exposed now so
//! neither row has to duplicate the reasoning once it exists.
//!
//! Being pure reads over existing state, none of this can move
//! `sim_fingerprint` — there is nothing here to gate on that basis, unlike
//! 03.1-03.4's own yearly WRITES.
use super::*;

/// The design doc's own nine ideal categories. `IDEAL_COUNT` many; a trait
/// maps to exactly one via `ideal_for_trait`.
pub(crate) const IDEAL_CONQUEST: usize = 0;
pub(crate) const IDEAL_WEALTH: usize = 1;
pub(crate) const IDEAL_LEARNING: usize = 2;
pub(crate) const IDEAL_STABILITY: usize = 3;
pub(crate) const IDEAL_LINEAGE: usize = 4;
pub(crate) const IDEAL_TRADITION: usize = 5;
pub(crate) const IDEAL_PURITY: usize = 6;
pub(crate) const IDEAL_ASSIMILATION: usize = 7;
pub(crate) const IDEAL_REACH: usize = 8;
pub(crate) const IDEAL_NAMES: [&str; 9] = [
    "conquest", "wealth", "learning", "stability", "lineage", "tradition", "purity", "assimilation", "reach",
];

/// `sim::cultures::TRAITS` index → ideal, per the design doc's own table.
/// Trait indices are the FIXED catalogue order (`cultures.rs`'s own "append
/// only" discipline) — Mercantile 0 · Seafaring 1 · Insular 2 · Martial 3 ·
/// Devout 4 · Nomadic 5 · Diaspora 6 · Assimilative 7 · Clannish 8 ·
/// Scholarly 9 · Agrarian 10 · Pastoral 11 · Artisan 12 · Xenophobic 13.
pub(crate) fn ideal_for_trait(trait_idx: usize) -> Option<usize> {
    match trait_idx {
        3 | 5 => Some(IDEAL_CONQUEST),        // Martial, Nomadic
        0 | 1 => Some(IDEAL_WEALTH),          // Mercantile, Seafaring
        9 | 12 => Some(IDEAL_LEARNING),       // Scholarly, Artisan
        10 | 11 => Some(IDEAL_STABILITY),     // Agrarian, Pastoral
        8 => Some(IDEAL_LINEAGE),             // Clannish
        4 => Some(IDEAL_TRADITION),           // Devout
        2 | 13 => Some(IDEAL_PURITY),         // Insular, Xenophobic
        7 => Some(IDEAL_ASSIMILATION),        // Assimilative
        6 => Some(IDEAL_REACH),               // Diaspora
        _ => None,
    }
}

/// Which development track (if any) measures a given ideal — the bridge
/// between 03.5's culture-scale reasoning and 03.3's real per-city points.
pub(crate) fn track_for_ideal(ideal: usize) -> Option<usize> {
    match ideal {
        IDEAL_CONQUEST => Some(TRACK_MILITARY),
        IDEAL_WEALTH => Some(TRACK_TRADE),
        IDEAL_LEARNING => Some(TRACK_IDEOLOGICAL),
        IDEAL_STABILITY => Some(TRACK_CIVIL),
        _ => None, // lineage/tradition/purity/assimilation/reach: no track counterpart yet
    }
}

/// How far below must trigger the barbarian judgement / admiration (the
/// design doc gives no number; a full track level's worth of points, or a
/// visible fraction of the dev scale, is the smallest gap worth naming a
/// judgement over).
const IDEAL_JUDGEMENT_GAP: f32 = TRACK_THRESHOLDS[1]; // one full level's worth of points
const DEV_JUDGEMENT_GAP: f32 = 1.0; // dev is centred near the 0.85-1.15 seed band

impl CampaignSim {
    /// Population-weighted mean `TickHub.dev` across every settled hub whose
    /// majority culture is `culture` — 0.0 if the culture holds no city.
    pub(crate) fn culture_development(&self, culture: &str) -> f32 {
        let mut wsum = 0.0f32;
        let mut psum = 0.0f32;
        for (h, hub) in self.hubs.iter().enumerate() {
            if hub.is_estate || hub.abandoned { continue; }
            if self.hub_culture.get(h).map(|c| c.as_str()) != Some(culture) { continue; }
            let pop = hub.population.max(0.0);
            wsum += hub.dev * pop;
            psum += pop;
        }
        if psum > EPS { wsum / psum } else { 0.0 }
    }

    /// A culture's one ideal, from its most characteristic real trait.
    /// `None` for a culture with no known trait list (a legacy/unknown name).
    pub(crate) fn culture_ideal(&self, culture: &str) -> Option<usize> {
        self.culture_trait_ids(culture).into_iter().find_map(ideal_for_trait)
    }

    /// How well `culture` is doing on ITS OWN ideal — the summed points of
    /// the matching track across its cities, or 0.0 if its ideal has no
    /// track counterpart yet (lineage/tradition/purity/assimilation/reach).
    pub(crate) fn culture_ideal_score(&self, culture: &str) -> f32 {
        let Some(ideal) = self.culture_ideal(culture) else { return 0.0 };
        let Some(track) = track_for_ideal(ideal) else { return 0.0 };
        self.hubs.iter().enumerate()
            .filter(|(h, hub)| !hub.is_estate && !hub.abandoned
                && self.hub_culture.get(*h).map(|c| c.as_str()) == Some(culture))
            .map(|(_, hub)| hub.track_points[track])
            .sum()
    }

    /// The design doc's JUDGEMENT: `a` regards `b` as barbarian when `b`'s
    /// overall development, or `b`'s score on `a`'s OWN ideal, sits far below
    /// `a`'s. Symmetric inputs, asymmetric result — a judgement is always
    /// FROM one culture's own standard.
    pub(crate) fn is_barbarian_to(&self, a: &str, b: &str) -> bool {
        let dev_gap = self.culture_development(a) - self.culture_development(b) >= DEV_JUDGEMENT_GAP;
        let ideal_gap = match self.culture_ideal(a) {
            Some(ideal) => match track_for_ideal(ideal) {
                Some(track) => {
                    let a_score: f32 = self.hubs.iter().enumerate()
                        .filter(|(h, hub)| !hub.is_estate && !hub.abandoned
                            && self.hub_culture.get(*h).map(|c| c.as_str()) == Some(a))
                        .map(|(_, hub)| hub.track_points[track]).sum();
                    let b_score: f32 = self.hubs.iter().enumerate()
                        .filter(|(h, hub)| !hub.is_estate && !hub.abandoned
                            && self.hub_culture.get(*h).map(|c| c.as_str()) == Some(b))
                        .map(|(_, hub)| hub.track_points[track]).sum();
                    a_score - b_score >= IDEAL_JUDGEMENT_GAP
                }
                None => false,
            },
            None => false,
        };
        dev_gap || ideal_gap
    }

    /// The reverse reading: `a`'s elites admire `b` when `b`'s development
    /// sits far above `a`'s own (Hellenisation, Rome's philhellenism).
    pub(crate) fn admires_more_developed(&self, a: &str, b: &str) -> bool {
        self.culture_development(b) - self.culture_development(a) >= DEV_JUDGEMENT_GAP
    }
}
