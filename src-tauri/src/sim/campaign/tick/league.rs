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
/// INSTITUTIONS_BUILD_ORDER.md 1.3 · `League.purse` had one writer (dues) and
/// zero readers. Each year, if the purse can afford it, it buys a season's
/// convoy escort: member-to-member lanes lose cargo at a reduced rate for the
/// year. A league with no purse (or that never accrues enough) never affords
/// one — bit-identical to before this slice, since `escort_until_tick` starts
/// and stays at 0 (`n7_a_world_with_no_leagues_is_bit_identical` still holds:
/// zero members ⇒ zero dues ⇒ zero purse ⇒ never funds an escort).
pub(crate) const LEAGUE_ESCORT_COST: f32 = 20.0;
/// Multiplier on voyage loss probability for a lane between two hubs of the
/// SAME league while its escort is active this year.
pub(crate) const LEAGUE_ESCORT_LOSS_MULT: f32 = 0.5;
/// INSTITUTIONS_BUILD_ORDER.md 4.3 · League privileges — the first thing that
/// makes membership worth anything beyond the convoy escort (1.3). A
/// member-to-member lane's FREIGHT is cheaper than an ordinary lane's, the
/// same shape `GUILDHALL_FREIGHT` (0.85) already uses at a hub's own
/// warehouse. Applied only to the house-carried leg (the tariff below is the
/// same restriction) — 96% of shipments move ownerless and never reach
/// either check (`ACTORS_AND_CARRIAGE_PLAN.md` §5.1), so this is a real but
/// narrow privilege, not a market-wide rewrite. Shipped LIVE rather than
/// dose-walked from a no-op: unlike 4.2's routed blockade (a routing
/// decision that could, at a large enough dose, concentrate trade toward
/// whichever member happens to be richest — the N2/N4 lesson) or 4.5's
/// boycott (an EXCLUSION), a discount on a narrow, already-taxed lane class
/// carries none of that shape; it is measured against the same gates below
/// rather than assumed safe.
pub(crate) const LEAGUE_FREIGHT_DISCOUNT: f32 = 0.85;
/// The tariff half of 4.3 — a multiplier on BOTH the export and import
/// tariff rate for a member-to-member sale (1.0 = full rate = no privilege).
/// A real exemption (0.0) would let two members trade entirely tax-free,
/// which is a stronger claim than any attested medieval Hanse actually won
/// (partial toll relief, not universal free trade) — `0.5` is a real,
/// nameable privilege without inventing a historically ungrounded absolute.
pub(crate) const LEAGUE_TARIFF_MULT: f32 = 0.5;

/// INSTITUTIONS_BUILD_ORDER.md 4.4 · THE KONTOR — one per league at a time,
/// established at the non-member hub the league trades with most, once that
/// tie clears the same `LEAGUE_FLOW_MIN` threshold formation itself uses (a
/// Kontor is a trading-post decision, not a diplomatic one — the flow tie is
/// the right signal for both). Costs the purse a one-off outlay; a league
/// that never accrues enough dues never establishes one — bit-identical to
/// before this slice (`n7_a_world_with_no_leagues_is_bit_identical` still
/// holds: zero members ⇒ zero purse ⇒ never affords one).
pub(crate) const KONTOR_COST: f32 = 40.0;
/// A host city already fighting a member, or one whose own unrest is high,
/// may expel a standing Kontor — a small yearly roll, scaled up by either
/// condition. Kept small: a Kontor that vanished as often as it formed
/// would never accumulate the trade history that makes it worth building.
pub(crate) const KONTOR_EXPEL_CHANCE: f32 = 0.02;
pub(crate) const KONTOR_EXPEL_CHANCE_AT_WAR: f32 = 0.25;
pub(crate) const KONTOR_EXPEL_UNREST_MULT: f32 = 0.5;

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

    /// INSTITUTIONS_BUILD_ORDER.md 4.5 — the deterministic target-selection
    /// rule N7.3 left open: which rival does the diet name? Prefers a real
    /// war a member is fighting (the Denmark 1361-70 case — a war IS the
    /// concrete threat `component_threatened` reads); falls back to the
    /// seat hub of the strongest threatening great power (rank ≥ 2) when no
    /// member is at war. Neither `self.wars` nor `self.realms` is a
    /// `HashMap`, so scanning in index order is already deterministic.
    pub(crate) fn choose_boycott_target(&self, li: usize, seat: usize) -> Option<usize> {
        let n = self.hubs.len();
        for w in &self.wars {
            let (a, b) = (w.a as usize, w.b as usize);
            if a >= n || b >= n { continue; }
            let a_member = self.hubs[a].league == li as i32;
            let b_member = self.hubs[b].league == li as i32;
            if a_member && !b_member { return Some(b); }
            if b_member && !a_member { return Some(a); }
        }
        self.realms.iter()
            .filter(|r| r.fallen_tick == 0 && r.rank >= 2)
            .map(|r| r.capital_hub as usize)
            .filter(|&h| h < n && h != seat)
            .min() // deterministic: lowest capital hub index among qualifying realms
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
                boycotts: vec![], escort_until_tick: 0,
                events: vec![RealmEvent {
                    tick: self.tick, kind: "league_founded".into(),
                    // 4.3 · names the privilege in the founding line itself, since a
                    // per-shipment chronicle entry for a discount that applies to
                    // every member-to-member sale would flood the journal rather
                    // than inform it — the founding is the one legible moment.
                    text: format!(
                        "{} founded with {} members, trading between themselves at reduced tariff and freight",
                        name, member_count),
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
        // N7.3 (§4.1, closed by INSTITUTIONS_BUILD_ORDER.md 4.5) — the diet
        // MAY vote ONE boycott, walked from `LEAGUE_BOYCOTT_MAX`: at 0 the
        // branch is real code that never fires (`n7_boycott_is_inert_at_
        // zero`); above zero, the diet names a target and votes it, up to
        // `LEAGUE_BOYCOTT_MAX` STANDING boycotts at once (never re-voting a
        // target it already boycotts). Deterministic target selection,
        // mirroring §3.2's own "the trigger is about the actual threat, not
        // an abstract set": the enemy hub of a war touching this league's
        // seat's component, if one exists, else the seat hub of the
        // strongest threatening rank-≥2 realm.
        let active_boycotts = league.boycotts.iter().filter(|b| b.until_tick > self.tick).count();
        let boycott = if LEAGUE_BOYCOTT_MAX > 0 && (active_boycotts as u32) < LEAGUE_BOYCOTT_MAX {
            self.choose_boycott_target(li, seat)
                .filter(|&t| !league.boycotts.iter().any(|b| b.until_tick > self.tick && b.target == t as u32))
                .map(|target| Boycott { target: target as u32, good: -1, until_tick: self.tick + LEAGUE_BOYCOTT_TICKS })
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
            // 4.5 · a real, nameable vote — the whole point of dosing this
            // above zero rather than leaving it structurally present and
            // silent. Chronicled once, at the vote itself, never per
            // shipment refused (`dispatch`'s own `hub_boycotts` check is
            // silent by design — see that check's own doc comment).
            let target = b.target as usize;
            if target < self.hubs.len() {
                let (ln, tn) = (self.leagues[li].name.clone(), self.hubs[target].name.clone());
                self.leagues[li].events.push(RealmEvent {
                    tick: self.tick, kind: "boycott_voted".into(),
                    text: format!("{ln} votes to boycott {tn}"),
                });
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "boycott_voted".into(), hub: target as i32, good: -1, value: 0.0,
                    text: format!("{ln} votes to boycott {tn}"),
                });
            }
            self.leagues[li].boycotts.push(b);
        }
        self.leagues[li].boycotts.retain(|b| b.until_tick > self.tick);
        // 1.3 · the purse buys a season's convoy escort if it can afford one —
        // the first real reader `League.purse` has ever had.
        let name = self.leagues[li].name.clone();
        let had_escort = self.leagues[li].escort_until_tick >= self.tick;
        if self.leagues[li].purse >= LEAGUE_ESCORT_COST {
            self.leagues[li].purse -= LEAGUE_ESCORT_COST;
            self.leagues[li].escort_until_tick = self.tick + TICKS_PER_YEAR;
            self.leagues[li].events.push(RealmEvent {
                tick: self.tick, kind: "escort".into(),
                text: format!("{name} fits out a convoy for the season"),
            });
        } else if had_escort {
            // The escort it had is about to lapse and there's no purse to
            // renew it — legible on the way out, not a silent lapse. Fires
            // once (this branch requires `had_escort`, which a lapsed escort
            // no longer satisfies next year).
            self.leagues[li].events.push(RealmEvent {
                tick: self.tick, kind: "no_escort".into(),
                text: format!("{name} cannot afford a convoy this season"),
            });
        }
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

    /// Is hub `h` currently hosting an ACTIVE Kontor for league `li` — used
    /// both to skip a league that already has one (one Kontor at a time,
    /// §4.4's own minimal scope) and to skip a hub already hosting one for
    /// ANOTHER league (a host may not double-book).
    fn hosts_active_kontor(&self, h: usize) -> Option<u32> {
        self.kontors.iter()
            .find(|k| k.expelled_tick == 0 && k.host_hub as usize == h)
            .map(|k| k.league)
    }

    /// Yearly · INSTITUTIONS_BUILD_ORDER.md 4.4 — a league with no standing
    /// Kontor and a purse that can afford one establishes it at the
    /// non-member hub it trades with most, once that tie clears the same
    /// `LEAGUE_FLOW_MIN` bar formation itself uses. Deterministic (`self.
    /// leagues`/`self.hubs` are plain `Vec`s, `flow_year` sorted by hub
    /// index before ranking, the same discipline `maybe_form_leagues` uses).
    pub(crate) fn maybe_establish_kontors(&mut self) {
        for li in 0..self.leagues.len() {
            if self.leagues[li].dissolved_tick != 0 { continue; }
            if self.leagues[li].purse < KONTOR_COST { continue; }
            if self.kontors.iter().any(|k| k.league == li as u32 && k.expelled_tick == 0) { continue; }
            let members: Vec<usize> = (0..self.hubs.len()).filter(|&h| self.hubs[h].league == li as i32).collect();
            let mut ties: Vec<(usize, f32)> = self.flow_year.iter().filter_map(|&(a, b, v)| {
                let (a, b) = (a as usize, b as usize);
                if members.contains(&a) && !members.contains(&b) { Some((b, v)) }
                else if members.contains(&b) && !members.contains(&a) { Some((a, v)) }
                else { None }
            }).collect();
            ties.sort_by_key(|&(h, _)| h); // determinism (flow_year order is not guaranteed)
            let mut best: Option<(usize, f32)> = None;
            for (h, v) in ties {
                if v < LEAGUE_FLOW_MIN || h >= self.hubs.len() { continue; }
                if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
                if self.hosts_active_kontor(h).is_some() { continue; }
                if best.map_or(true, |(_, bv)| v > bv) { best = Some((h, v)); }
            }
            let Some((host, _)) = best else { continue };
            self.leagues[li].purse -= KONTOR_COST;
            self.kontors.push(Kontor {
                league: li as u32, host_hub: host as u32,
                established_tick: self.tick, expelled_tick: 0,
            });
            let (ln, hn) = (self.leagues[li].name.clone(), self.hubs[host].name.clone());
            self.leagues[li].events.push(RealmEvent {
                tick: self.tick, kind: "kontor_established".into(),
                text: format!("{ln} establishes a Kontor at {hn}"),
            });
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "kontor_established".into(), hub: host as i32, good: -1, value: 0.0,
                text: format!("{ln} establishes a Kontor at {hn}"),
            });
        }
    }

    /// Yearly · a host may expel a standing Kontor — a real political event,
    /// not a mechanical lapse. More likely while at war with a member of the
    /// league that holds it, and scaled up by the host's own unrest (an
    /// unstable council is readier to turn on a foreign trading post).
    pub(crate) fn maybe_expel_kontors(&mut self) {
        for ki in 0..self.kontors.len() {
            if self.kontors[ki].expelled_tick != 0 { continue; }
            let host = self.kontors[ki].host_hub as usize;
            let li = self.kontors[ki].league as usize;
            if host >= self.hubs.len() || li >= self.leagues.len() { continue; }
            let at_war_with_member = (0..self.hubs.len())
                .any(|m| self.hubs[m].league == li as i32 && self.hubs[host].war_with == m as i32);
            let chance = if at_war_with_member { KONTOR_EXPEL_CHANCE_AT_WAR } else { KONTOR_EXPEL_CHANCE }
                + self.hubs[host].society.unrest.max(0.0) * KONTOR_EXPEL_UNREST_MULT;
            let roll = hash01(self.seed, (self.tick as u64) ^ 0x40470E ^ (ki as u64), host as u64);
            if roll >= chance { continue; }
            self.kontors[ki].expelled_tick = self.tick;
            let (ln, hn) = (self.leagues[li].name.clone(), self.hubs[host].name.clone());
            self.leagues[li].events.push(RealmEvent {
                tick: self.tick, kind: "kontor_expelled".into(),
                text: format!("{hn} expels {ln}'s Kontor"),
            });
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "kontor_expelled".into(), hub: host as i32, good: -1, value: 0.0,
                text: format!("{hn} expels {ln}'s Kontor"),
            });
        }
    }

    /// Yearly entry point for 4.4, called alongside `run_league_diet`.
    pub(crate) fn run_kontors(&mut self) {
        self.maybe_establish_kontors();
        self.maybe_expel_kontors();
    }

    /// Which league (if any) holds a standing Kontor at hub `h` — the read
    /// `dispatch` uses to extend the member-to-member privilege (4.3) to a
    /// member trading through a Kontor's host. −1 if none.
    pub(crate) fn kontor_league_at(&self, h: usize) -> i32 {
        self.kontors.iter()
            .find(|k| k.expelled_tick == 0 && k.host_hub as usize == h)
            .map(|k| k.league as i32)
            .unwrap_or(-1)
    }
}
