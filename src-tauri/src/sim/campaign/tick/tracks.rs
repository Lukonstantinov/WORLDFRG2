//! tracks — `docs/living_world/03_DEVELOPMENT_TRACKS.md`, slice 03.3.
//!
//! Four independent per-city tracks — Military · Trade · Civil · Ideological
//! (`TRACK_*` indices) — each a running point total and a level (0-5) that
//! rises AUTOMATICALLY once the total crosses a threshold. Points arrive
//! yearly from real sources (trade volume, resident houses/banks/guilds,
//! welfare, laws, officials, soldier-class share, war/levy activity, partner
//! reach) scaled by `stability_of` (03.2), plus a small bonus per resident
//! `Individual` (row 02) whose role suits the track — a commander or admiral
//! for Military, a merchant prince/banker/guildmaster for Trade, a physician
//! or official for Civil, a philosopher/scholar/ideologue/artisan for
//! Ideological — the design doc's own "notable roles that add" column, now a
//! real cross-row reader rather than a name in a table.
//!
//! **A sack costs points, and at worst a level.** `TickHub.damage` is the
//! REAL, already-live signal (war sack, fire, flood — `war_damage_pass` and
//! `estate_condition_pass`'s own disaster roll both already set it) — no new
//! "was this hub just sacked" state is needed. While damage stays above
//! `TRACK_SACK_DAMAGE_FLOOR` every track's points are shaved by a fixed
//! fraction each year, tapering off naturally as the existing funded-repair
//! pass lowers `damage` back down. Because level is RECOMPUTED from points
//! every year (never incremented independently), a sack that drops points
//! below the current level's own threshold silently costs the level too —
//! exactly the design doc's own "at worst a level" phrasing, with no extra
//! branch needed to express it.
//!
//! **Forward hooks read as absent, not neutral, until their row exists**
//! (00_INDEX rule): "victories"/"defended sieges" (no war-outcome log at
//! hub granularity yet), "Martial culture" (03.5's own culture-ideal axis),
//! resident scholars/schools/masterworks (rows 06/07) all contribute
//! `0.0` today rather than an invented stand-in. Ideological's own points
//! are real even so — "ideas arriving along trade routes" is exactly what
//! partner reach already measures (the same term `development.rs` uses).
//!
//! **Read by nothing downstream this slice** — 03.4's buildings are the
//! first real consumer of `track_level`. `sim_fingerprint` (folding only
//! `stock`/`price`/`population`/`treasury`/`export_earn`/`import_spend`) is
//! therefore provably untouched, proven the same way slice 03.1 proved it.
use super::*;

pub(crate) const TRACK_MILITARY: usize = 0;
pub(crate) const TRACK_TRADE: usize = 1;
pub(crate) const TRACK_CIVIL: usize = 2;
pub(crate) const TRACK_IDEOLOGICAL: usize = 3;
pub(crate) const TRACK_COUNT: usize = 4;

/// Cumulative points needed to REACH level 1..5 (level 0 is the floor, no
/// threshold to clear). Deliberately the SAME ladder for every track — the
/// design doc gives no reason for one track to level faster than another.
pub(crate) const TRACK_THRESHOLDS: [f32; 5] = [8.0, 20.0, 40.0, 70.0, 110.0];
pub(crate) const TRACK_LEVEL_MAX: u8 = 5;

/// A pure function: which level a running point total has reached. Used both
/// by the yearly pass and (so the same rule can be reasoned about in one
/// place) by `stability_is_bounded`'s sibling test below.
pub(crate) fn level_for_points(points: f32) -> u8 {
    let mut lvl = 0u8;
    for &t in &TRACK_THRESHOLDS {
        if points >= t { lvl += 1; } else { break; }
    }
    lvl.min(TRACK_LEVEL_MAX)
}

const MIL_SOLDIER_WEIGHT: f32 = 6.0;
const MIL_WAR_WEIGHT: f32 = 1.2;
const MIL_LEVY_WEIGHT: f32 = 0.8;
const TRADE_VOLUME_REF: f32 = 20_000.0;
const TRADE_VOLUME_WEIGHT: f32 = 2.0;
/// Per resident house, capped — a great entrepôt hosting fifty houses is not
/// ten times the trade-town a five-house city is; contact matters, not count.
const TRADE_HOUSE_WEIGHT: f32 = 0.5;
const TRADE_HOUSE_CAP: f32 = 4.0;
const TRADE_BANK_WEIGHT: f32 = 0.6;
const TRADE_GUILD_WEIGHT: f32 = 0.8;
const CIVIL_WELFARE_WEIGHT: f32 = 1.0;
const CIVIL_LAW_WEIGHT: f32 = 0.3;
const CIVIL_LAW_CAP: f32 = 2.0;
const CIVIL_OFFICIAL_WEIGHT: f32 = 0.25;
/// A small ambient trickle so Ideological is never a hard, permanent zero
/// while rows 06/07 don't exist — ideas move a little even without a named
/// scholar, the same way a market has SOME price discovery with no merchant
/// house present (96% of shipments move on no house's account at all).
const IDEO_BASE: f32 = 0.1;
const IDEO_PARTNER_WEIGHT: f32 = 1.0;
/// A resident `Individual` (row 02) whose role suits the track — the design
/// doc's own "notable roles that add" column.
const NOTABLE_ROLE_BONUS: f32 = 0.5;

/// While a hub's real, already-live `damage` (war sack, fire, flood) sits
/// above this floor, every track bleeds `TRACK_SACK_LOSS_FRAC` of its points
/// each year — tapering off on its own as the existing funded-repair pass
/// lowers `damage`, so no separate "recovery" logic is needed here.
const TRACK_SACK_DAMAGE_FLOOR: f32 = 0.3;
const TRACK_SACK_LOSS_FRAC: f32 = 0.25;

// ── 03.4 · buildings ─────────────────────────────────────────────────────
// A level makes a building POSSIBLE; building it costs real goods (via
// `pick_build_supply_good`, the same L6 housing convention) and city
// treasury, gradually (`TRACK_BUILD_RATE` of the remaining cost a year, when
// affordable — the same shape `housing_build_persons_e` already uses).
// `TickHub.track_buildings` is the highest level ACTUALLY BUILT per track
// (0 = none), always `<= track_level` (the ability/points level) by
// construction — `track_building_allowed` is the one place that invariant is
// enforced, and it holds regardless of dose.
//
// **Construction is dosed, and ships at 0.0 — a true no-op** (this slice's
// own gate name: "at dose 0 construction is off"). Effects of an actually
// built building (army strength, warehouse capacity, population ceiling,
// idea spread, …) are 03.7's job, each behind its own separate dose
// constant — this slice only makes a building buildable and buildable-from,
// never wires an effect.
pub(crate) const TRACK_CONSTRUCTION_DOSE: f32 = 0.0;
const TRACK_BUILD_RATE: f32 = 0.35;
const TRACK_BUILD_GOOD_BASE: f32 = 40.0;
const TRACK_BUILD_MONEY_BASE: f32 = 30.0;

/// A building at `next_level` may be started only once the track's own
/// ability level has reached at least that rung — dose-independent, always
/// true even at full construction dose.
pub(crate) fn track_building_allowed(next_level: u8, track_level: u8) -> bool {
    next_level >= 1 && next_level <= TRACK_LEVEL_MAX && next_level <= track_level
}

/// One year's construction progress on one (hub, track)'s next building —
/// pure, so the dose can be exercised directly in tests without touching the
/// shipped-zero constant. `dose <= 0.0` returns `progress` unchanged and
/// spends nothing, which is the whole "construction is off" claim.
pub(crate) fn track_build_progress_e(
    progress: f32, avail_good: f32, avail_money: f32,
    cost_good_total: f32, cost_money_total: f32, dose: f32,
) -> (f32, f32, f32) {
    if dose <= 0.0 || progress >= 1.0 {
        return (progress, 0.0, 0.0);
    }
    let remaining_frac = 1.0 - progress;
    let want_frac = (TRACK_BUILD_RATE * dose.clamp(0.0, 1.0)).min(remaining_frac);
    if want_frac <= 0.0 {
        return (progress, 0.0, 0.0);
    }
    let need_good = cost_good_total * want_frac;
    let need_money = cost_money_total * want_frac;
    let good_afford = if need_good > EPS { (avail_good / need_good).min(1.0) } else { 1.0 };
    let money_afford = if need_money > EPS { (avail_money / need_money).min(1.0) } else { 1.0 };
    let afford = good_afford.min(money_afford).max(0.0);
    let actual_frac = want_frac * afford;
    let used_good = cost_good_total * actual_frac;
    let used_money = cost_money_total * actual_frac;
    ((progress + actual_frac).min(1.0), used_good, used_money)
}

/// The design doc's own building names, by track and level — used only for
/// the chronicle line today; the settlement panel's own reading (03.6) will
/// call this too rather than duplicate the table.
pub(crate) fn track_building_name(track: usize, level: u8) -> &'static str {
    match (track, level) {
        (TRACK_MILITARY, 1) => "a palisade",
        (TRACK_MILITARY, 2) => "stone walls",
        (TRACK_MILITARY, 3) => "a barracks",
        (TRACK_MILITARY, 4) => "an arsenal",
        (TRACK_MILITARY, 5) => "a fortress",
        (TRACK_TRADE, 1) => "a quay",
        (TRACK_TRADE, 2) => "a harbour mole and warehouse district",
        (TRACK_TRADE, 3) => "a mint and exchange",
        (TRACK_TRADE, 4) => "a lighthouse and fondaco quarter",
        (TRACK_TRADE, 5) => "a bourse",
        (TRACK_CIVIL, 1) => "a well and granary",
        (TRACK_CIVIL, 2) => "a forum and council hall",
        (TRACK_CIVIL, 3) => "a public granary",
        (TRACK_CIVIL, 4) => "an aqueduct, sewer and baths",
        (TRACK_CIVIL, 5) => "roads and a courier service",
        (TRACK_IDEOLOGICAL, 1) => "a shrine",
        (TRACK_IDEOLOGICAL, 2) => "a school",
        (TRACK_IDEOLOGICAL, 3) => "a library",
        (TRACK_IDEOLOGICAL, 4) => "an academy and embassy",
        (TRACK_IDEOLOGICAL, 5) => "a university",
        _ => "a building",
    }
}

impl CampaignSim {
    /// One year of construction toward each (hub, track)'s next possible
    /// building. `dose` is passed explicitly (never read from the constant
    /// internally) so a test can exercise the mechanism at full dose while
    /// the real yearly call always passes the shipped `TRACK_CONSTRUCTION_
    /// DOSE` (0.0).
    pub(crate) fn update_track_buildings(&mut self, dose: f32) {
        if dose <= 0.0 { return; }
        let ng = self.goods.len();
        if ng == 0 { return; }
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let g = self.pick_build_supply_good(h, 2) as usize;
            if g >= ng { continue; }
            for k in 0..TRACK_COUNT {
                let next = self.hubs[h].track_buildings[k] + 1;
                if !track_building_allowed(next, self.hubs[h].track_level[k]) { continue; }
                let cost_good_total = TRACK_BUILD_GOOD_BASE * next as f32;
                let cost_money_total = TRACK_BUILD_MONEY_BASE * next as f32;
                let avail_good = stock_of(&self.hubs[h].stock, g).max(0.0);
                let avail_money = self.hubs[h].treasury.max(0.0);
                let progress = self.hubs[h].track_build_progress[k];
                let (new_progress, used_good, used_money) = track_build_progress_e(
                    progress, avail_good, avail_money, cost_good_total, cost_money_total, dose,
                );
                if used_good > EPS { stock_take(&mut self.hubs[h].stock, g, used_good); }
                if used_money > EPS { self.hubs[h].treasury -= used_money; }
                self.hubs[h].track_build_progress[k] = new_progress;
                if new_progress >= 1.0 - 1e-6 {
                    self.hubs[h].track_buildings[k] = next;
                    self.hubs[h].track_build_progress[k] = 0.0;
                    let hn = self.hubs[h].name.clone();
                    let label = track_building_name(k, next);
                    self.journal.push(JournalEntry {
                        tick: self.tick, kind: "structure".into(), hub: h as i32, good: -1, value: 0.0,
                        text: format!("{hn} raises {label}"),
                    });
                }
            }
        }
    }

    /// One year of the four tracks for every settled (non-estate) hub.
    pub(crate) fn update_tracks(&mut self, _year: u32) {
        let n = self.hubs.len();
        // Per-hub resident-house counts and guild strength, gathered ONCE —
        // an O(houses) + O(guilds) pass, not O(hubs × houses).
        let mut houses_at = vec![0u32; n];
        for hh in &self.houses {
            if !hh.defunct && (hh.hub as usize) < n { houses_at[hh.hub as usize] += 1; }
        }
        let mut guild_strength_at = vec![0.0f32; n];
        for g in &self.guilds {
            if (g.hub as usize) < n {
                guild_strength_at[g.hub as usize] = guild_strength_at[g.hub as usize].max(g.strength);
            }
        }
        // Notable presence per hub per track, from the real Individual
        // roster (row 02) — one O(people) pass, not one per hub.
        let mut notable_at = [vec![0u32; n], vec![0u32; n], vec![0u32; n], vec![0u32; n]];
        for p in &self.people {
            if p.current_hub < 0 || p.current_hub as usize >= n { continue; }
            let h = p.current_hub as usize;
            for &r in &p.roles {
                let track = match r {
                    ROLE_COMMANDER | ROLE_ADMIRAL => Some(TRACK_MILITARY),
                    ROLE_MERCHANT_PRINCE | ROLE_BANKER | ROLE_GUILDMASTER => Some(TRACK_TRADE),
                    ROLE_PHYSICIAN | ROLE_OFFICIAL | ROLE_ALDERMAN => Some(TRACK_CIVIL),
                    ROLE_PHILOSOPHER | ROLE_SCHOLAR | ROLE_IDEOLOGUE | ROLE_ARTISAN => Some(TRACK_IDEOLOGICAL),
                    _ => None,
                };
                if let Some(t) = track { notable_at[t][h] += 1; }
            }
        }

        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let stability = self.stability_at(h);
            let neighbours = self.neighbors.get(h).map(|v| v.as_slice()).unwrap_or(&[]);

            // ── Military ──
            let soldier_share = self.hubs[h].pops.iter()
                .find(|p| p.profession == POP_SOLDIER)
                .map(|p| (p.size / self.hubs[h].population.max(1.0)).clamp(0.0, 1.0))
                .unwrap_or(0.0);
            let at_war = self.hubs[h].war_with >= 0;
            let levying = self.hubs[h].war_manpower > 0.0;
            let mut mil = soldier_share * MIL_SOLDIER_WEIGHT
                + if at_war { MIL_WAR_WEIGHT } else { 0.0 }
                + if levying { MIL_LEVY_WEIGHT } else { 0.0 };

            // ── Trade ──
            let volume_term = (1.0 + self.hubs[h].trade_last_year.max(0.0) / TRADE_VOLUME_REF).ln() * TRADE_VOLUME_WEIGHT;
            let house_term = (houses_at[h] as f32).min(TRADE_HOUSE_CAP) * TRADE_HOUSE_WEIGHT;
            let bank_term = if self.hubs[h].main_bank >= 0 { TRADE_BANK_WEIGHT } else { 0.0 };
            let guild_term = guild_strength_at[h] * TRADE_GUILD_WEIGHT;
            let mut trade = volume_term + house_term + bank_term + guild_term;

            // ── Civil ──
            let wr = self.hubs[h].welfare_ratio;
            let welfare_term = if wr > 0.0 { (wr / 2.0).clamp(0.0, 1.5) * CIVIL_WELFARE_WEIGHT } else { 0.0 };
            let law_term = (self.hubs[h].laws.len() as f32).min(CIVIL_LAW_CAP) * CIVIL_LAW_WEIGHT;
            let official_term = self.hubs[h].officials.len() as f32 * CIVIL_OFFICIAL_WEIGHT;
            let mut civil = welfare_term + law_term + official_term;

            // ── Ideological ── scholars/schools/masterworks are rows 05-07,
            // absent (forward hook); trade-route contact is real today.
            let partner_frac = (neighbours.len() as f32 / NEIGHBOR_K as f32).min(1.0);
            let mut ideo = IDEO_BASE + partner_frac * IDEO_PARTNER_WEIGHT;

            mil += notable_at[TRACK_MILITARY][h] as f32 * NOTABLE_ROLE_BONUS;
            trade += notable_at[TRACK_TRADE][h] as f32 * NOTABLE_ROLE_BONUS;
            civil += notable_at[TRACK_CIVIL][h] as f32 * NOTABLE_ROLE_BONUS;
            ideo += notable_at[TRACK_IDEOLOGICAL][h] as f32 * NOTABLE_ROLE_BONUS;

            let gains = [mil, trade, civil, ideo];
            let sacked = self.hubs[h].damage > TRACK_SACK_DAMAGE_FLOOR;
            for k in 0..TRACK_COUNT {
                let mut pts = self.hubs[h].track_points[k] + gains[k].max(0.0) * stability;
                if sacked { pts *= 1.0 - TRACK_SACK_LOSS_FRAC; }
                pts = pts.max(0.0);
                self.hubs[h].track_points[k] = pts;
                self.hubs[h].track_level[k] = level_for_points(pts);
            }
        }
    }
}
