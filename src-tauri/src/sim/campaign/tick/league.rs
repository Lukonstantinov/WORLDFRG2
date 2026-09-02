//! The League — `docs/SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §3-4 (N7). A
//! REALM'S NEGATIVE (§3.1): members keep their own government, their own
//! treasury, their own provinces, and the right to leave. No provinces, no
//! capital, no succession, no writ — the one collective verb is the boycott,
//! and it ships at zero dose (§4.1's own build order, N7.1 → N7.2 → N7.3).
//!
//! Formation, the diet and dissolution are modelled deliberately close to
//! `realms.rs`'s own passes (`maybe_proclaim_realms`, `realm_secession_pass`)
//! — the proven non-monotone shape in this tree — rather than invented fresh.
use super::*;

/// A league needs at least this many hubs to exist at all (below this it
/// dissolves — §3.3 exit 4/the min-members floor).
pub(crate) const LEAGUE_MIN_MEMBERS: usize = 3;
/// A founding round stops adding members here, so one seat's flow ties can't
/// swallow half a trade component into a single league.
pub(crate) const LEAGUE_MAX_FOUNDING_MEMBERS: usize = 8;
/// Same era as realm formation (`REALM_YEAR_FLOOR`) — leagues are a state-
/// formation-era institution, not an early-game one.
pub(crate) const LEAGUE_YEAR_FLOOR: u32 = REALM_YEAR_FLOOR;
/// A candidate member needs a realised trade tie to the seat (`flow_year`,
/// one full year's volume) above this before it counts as "a shared lane" —
/// a league is a trading bloc, so commerce is the precondition, not proximity.
pub(crate) const LEAGUE_FLOW_MIN: f32 = 50.0;
/// §3.3 exit 2 — no shared-threat signal for this many years and members
/// drift out one at a time. Mirrors `realm_secession_pass`'s own non-monotone
/// discipline: a league that only ever grows is a failed build (§4.3 gate).
pub(crate) const LEAGUE_DRIFT_YEARS: u32 = 20;
/// Annual dues as a fraction of a member's treasury — small; the purse is a
/// real number to gate on, not a wealth-concentration channel (§0's N2/N4
/// lesson: never weight a collective mechanism by a wealth-correlated field).
pub(crate) const LEAGUE_DUES_FRAC: f32 = 0.01;
/// A member below this treasury cannot pay dues and leaves (§3.3 exit 3).
pub(crate) const LEAGUE_DUES_MIN_TREASURY: f32 = 50.0;
/// N7.3 (§4.1) — how many boycotts a diet may vote per year, walked from
/// zero. At 0 the vote is structurally present and never exercised: a real
/// no-op, not a feature flag wrapped around dead code.
pub(crate) const LEAGUE_BOYCOTT_MAX: u32 = 0;
/// How long a voted boycott stands, mirroring `N2_BAN_TICKS`/
/// `RELIEF_EXPORT_LOCK_TICKS` — re-votable, lapses on its own.
pub(crate) const LEAGUE_BOYCOTT_TICKS: u32 = TICKS_PER_YEAR;
/// Chronicle cap, mirroring `HOUSE_EVENTS_CAP`'s discipline at League scale
/// (a league's whole life is a handful of events, not thousands).
pub(crate) const LEAGUE_EVENTS_CAP: usize = 60;

/// A pure yearly decision for one league — dues to collect, who leaves, who
/// the seat becomes if it fell, and (once dosed) a boycott to open. Split
/// from `apply_league_diet` on the `decide_*`/`apply_*` convention
/// (FIX_PLAN B2) so a player holding the seat can supply this later without
/// the sim knowing the difference.
pub(crate) struct LeagueChoice {
    pub league: usize,
    pub dissolve: bool,
    pub new_seat: Option<u32>,
    pub expel: Vec<usize>,
    pub dues: Vec<(usize, f32)>,
    pub refresh_threat: bool,
    pub boycott: Option<Boycott>,
}

impl CampaignSim {
    /// Is hub `h`'s trade component under a shared threat right now — an
    /// active war touching the component, or an adjacent great power (a
    /// realm of rank ≥ 2 anywhere; §3.2's own coarse reading, since nothing
    /// here yet tracks "adjacent" at sub-component granularity). Leagues form
    /// AGAINST something; without this term they would form everywhere and
    /// never dissolve (§3.2).
    fn component_threatened(&self, comp: u32) -> bool {
        let n = self.hubs.len();
        let war_here = self.wars.iter().any(|w| {
            (w.a as usize) < n && (w.b as usize) < n
                && (self.hubs[w.a as usize].component == comp || self.hubs[w.b as usize].component == comp)
        });
        war_here || self.realms.iter().any(|r| r.fallen_tick == 0 && r.rank >= 2)
    }

    /// Is hub `h` free to join a league right now — no league already, and
    /// either no realm or a highly autonomous one (§3.2's "free to join": an
    /// autonomous crown city joining a merchant league is the historically
    /// normal case; a centralised crown's city is not).
    fn league_eligible(&self, h: usize) -> bool {
        let hub = &self.hubs[h];
        if hub.is_estate || hub.abandoned || hub.league >= 0 { return false; }
        hub.realm < 0
            || self.realms.get(hub.realm as usize)
                .is_some_and(|r| r.fallen_tick == 0 && r.autonomy == AUTONOMY_AUTONOMOUS)
    }

    /// A small, deterministic naming set — placeholder exactly like
    /// `REALM_NAME_STYLES` (getting the ENTITY right is N7.1's job; a richer,
    /// culture-derived namer is later follow-up that touches no game state).
    fn league_name_for(&self, seat: usize, salt: u32) -> String {
        const STYLES: [&str; 5] = [
            "the {c} League", "the {c} Hanse", "the {c} Compact",
            "the Confederation of {c}", "the {c} Union",
        ];
        let city = self.hubs[seat].name.clone();
        let idx = (hash01(seat as u64, salt as u64, 0x4C454147) * STYLES.len() as f32) as usize;
        STYLES[idx.min(STYLES.len() - 1)].replace("{c}", &city)
    }

    /// Yearly · §3.2 formation. Iterates candidate SEATS (tier 1/2, not
    /// already a realm's seat and not already leagued) rather than picking
    /// members first, mirroring `maybe_proclaim_realms`'s own "the trigger is
    /// about the city, not the abstract set" shape.
    pub(crate) fn maybe_form_leagues(&mut self, yr: u32) {
        if yr <= LEAGUE_YEAR_FLOOR { return; }
        let n = self.hubs.len();
        if n == 0 { return; }
        // DETERMINISM (§3.2's own named trap): `flow_year` is built off a
        // HashMap elsewhere (`flow_accum`), so every candidate list here is
        // sorted by hub index before this ever branches on it.
        let mut seats: Vec<usize> = (0..n).filter(|&h| {
            let hub = &self.hubs[h];
            !hub.is_estate && !hub.abandoned && hub.league < 0
                && (hub.tier == 1 || hub.tier == 2)
                && (hub.realm < 0 || hub.realm_role != REALM_ROLE_SEAT)
        }).collect();
        seats.sort_unstable();
        for seat in seats {
            if self.hubs[seat].league >= 0 { continue; } // joined an earlier seat's league this pass
            let comp = self.hubs[seat].component;
            if !self.component_threatened(comp) { continue; }
            let mut ties: Vec<(usize, f32)> = self.flow_year.iter().filter_map(|&(a, b, v)| {
                if a == seat as u32 { Some((b as usize, v)) }
                else if b == seat as u32 { Some((a as usize, v)) }
                else { None }
            }).collect();
            ties.sort_by_key(|&(h, _)| h); // determinism (flow_year order is not guaranteed)
            let mut members = vec![seat];
            for (h, v) in ties {
                if v < LEAGUE_FLOW_MIN || h >= n || members.contains(&h) { continue; }
                if !self.league_eligible(h) { continue; }
                members.push(h);
                if members.len() >= LEAGUE_MAX_FOUNDING_MEMBERS { break; }
            }
            if members.len() < LEAGUE_MIN_MEMBERS { continue; }
            let id = self.leagues.len() as u32;
            let name = self.league_name_for(seat, id);
            let member_count = members.len();
            for &h in &members { self.hubs[h].league = id as i32; }
            self.leagues.push(League {
                id, name: name.clone(), seat_hub: seat as u32, purse: 0.0,
                founded_tick: self.tick, dissolved_tick: 0, last_threat_tick: self.tick,
                boycotts: vec![],
                events: vec![RealmEvent {
                    tick: self.tick, kind: "league_founded".into(),
                    text: format!("{} founded with {} members", name, member_count),
                }],
            });
        }
    }

    /// Pure proposal for one league's yearly diet — reads `&self` only.
    fn decide_league_diet_one(&self, li: usize) -> LeagueChoice {
        let n = self.hubs.len();
        let mut members: Vec<usize> = (0..n).filter(|&h| self.hubs[h].league == li as i32).collect();
        members.sort_unstable();
        let league = &self.leagues[li];
        if members.len() < LEAGUE_MIN_MEMBERS {
            return LeagueChoice { league: li, dissolve: true, new_seat: None, expel: vec![], dues: vec![], refresh_threat: false, boycott: None };
        }
        let seat = league.seat_hub as usize;
        let comp = self.hubs.get(seat).map(|h| h.component).unwrap_or(0);
        let threatened = self.component_threatened(comp);
        // §3.3 exit 1 — annexed / no longer free (a realm took the member, or
        // it lost its high-autonomy standing) leaves on the spot.
        let mut expel: Vec<usize> = members.iter().copied()
            .filter(|&h| h != seat && !self.league_eligible_member(h))
            .collect();
        // §3.3 exit 2 — the threat lapsed: drift out ONE member (never the
        // seat) per year past `LEAGUE_DRIFT_YEARS`, deterministically (lowest
        // hub index — never an RNG or HashMap-order pick, N4's own lesson).
        if !threatened && self.tick.saturating_sub(league.last_threat_tick) > LEAGUE_DRIFT_YEARS * TICKS_PER_YEAR {
            if let Some(&drift) = members.iter().find(|&&h| h != seat && !expel.contains(&h)) {
                expel.push(drift);
            }
        }
        // §3.3 exit 3 — dues unpaid.
        let mut dues = Vec::new();
        for &h in &members {
            if expel.contains(&h) { continue; }
            let treas = self.hubs[h].treasury;
            let amt = treas * LEAGUE_DUES_FRAC;
            if treas < LEAGUE_DUES_MIN_TREASURY { expel.push(h); continue; }
            dues.push((h, amt));
        }
        // §3.3 exit 4 — the seat fell: the diet moves to the largest
        // remaining member, or the league dissolves below the floor.
        let remaining: Vec<usize> = members.iter().copied().filter(|h| !expel.contains(h)).collect();
        let (new_seat, dissolve) = if self.hubs.get(seat).is_some_and(|h| h.abandoned) {
            match remaining.iter().filter(|&&h| h != seat)
                .max_by(|&&a, &&b| self.hubs[a].population.partial_cmp(&self.hubs[b].population).unwrap_or(std::cmp::Ordering::Equal)) {
                Some(&biggest) => (Some(biggest as u32), false),
                None => (None, true),
            }
        } else if remaining.len() < LEAGUE_MIN_MEMBERS {
            (None, true)
        } else {
            (None, false)
        };
        // N7.3 (§4.1) — the diet MAY vote ONE boycott, walked from
        // `LEAGUE_BOYCOTT_MAX = 0`: the branch is real code, but at zero it
        // never fires, so this is a true no-op rather than a disabled
        // feature (`n7_boycott_is_inert_at_zero`).
        let boycott = if LEAGUE_BOYCOTT_MAX > 0 {
            // Not yet chosen: the target-selection rule (which rival to name,
            // vote weight, cause) is deliberately left for the walk above
            // zero dose — see §4.1/§4.2 of the plan.
            None::<Boycott>
        } else {
            None
        };
        LeagueChoice { league: li, dissolve, new_seat, expel, dues, refresh_threat: threatened, boycott }
    }

    /// §3.2's "free to join" re-checked for a STANDING member (annexation
    /// exit, §3.3 exit 1) — identical test to `league_eligible` minus the
    /// "not already leagued" clause, which is meaningless for a member.
    fn league_eligible_member(&self, h: usize) -> bool {
        let hub = &self.hubs[h];
        if hub.is_estate || hub.abandoned { return false; }
        hub.realm < 0
            || self.realms.get(hub.realm as usize)
                .is_some_and(|r| r.fallen_tick == 0 && r.autonomy == AUTONOMY_AUTONOMOUS)
    }

    /// The only part that mutates.
    fn apply_league_diet_one(&mut self, c: LeagueChoice) {
        let li = c.league;
        if c.dissolve {
            self.dissolve_league(li);
            return;
        }
        for h in &c.expel {
            if self.hubs[*h].league == li as i32 { self.hubs[*h].league = -1; }
        }
        for (h, amt) in &c.dues {
            let amt = amt.max(0.0).min(self.hubs[*h].treasury);
            self.hubs[*h].treasury -= amt;
            self.leagues[li].purse += amt;
        }
        if let Some(seat) = c.new_seat {
            self.leagues[li].seat_hub = seat;
        }
        if c.refresh_threat {
            self.leagues[li].last_threat_tick = self.tick;
        }
        if let Some(b) = c.boycott {
            self.leagues[li].boycotts.push(b);
        }
        self.leagues[li].boycotts.retain(|b| b.until_tick > self.tick);
        let ev = &mut self.leagues[li].events;
        if ev.len() > LEAGUE_EVENTS_CAP {
            let drop = ev.len() - LEAGUE_EVENTS_CAP;
            ev.drain(0..drop);
        }
    }

    fn dissolve_league(&mut self, li: usize) {
        if self.leagues[li].dissolved_tick != 0 { return; }
        self.leagues[li].dissolved_tick = self.tick;
        let id = li as i32;
        for h in self.hubs.iter_mut() { if h.league == id { h.league = -1; } }
        let name = self.leagues[li].name.clone();
        self.leagues[li].events.push(RealmEvent {
            tick: self.tick, kind: "league_dissolved".into(),
            text: format!("{name} dissolved"),
        });
    }

    /// Yearly entry point: `decide_*` then `apply_*`, per league, in id order
    /// (deterministic — `self.leagues` is a plain `Vec`, never a `HashMap`).
    pub(crate) fn run_league_diet(&mut self) {
        for li in 0..self.leagues.len() {
            if self.leagues[li].dissolved_tick != 0 { continue; }
            let choice = self.decide_league_diet_one(li);
            self.apply_league_diet_one(choice);
        }
    }
}
