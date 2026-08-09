//! crisis — Phase 3.2–3.6 of `docs/proposals/HOUSE_MASTER_PLAN.md`: competence/vice,
//! the succession-crisis struggle (named factions, quarterly rounds, resolution),
//! civic intervention, and the permanent capped record it leaves behind.
//!
//! Scoped down hard from the source designs (`HOUSE_POWER_AND_POLITICS.md`,
//! `HOUSE_SUCCESSION_CRISIS.md`, `HOUSE_POWER_STRUGGLE_VIEW.md`,
//! `HOUSE_FACTION_NAMING_AND_RECORD.md`) — see the Phase 3 handoff block in
//! `HOUSE_MASTER_PLAN.md` for the full list of what was cut and why. The two biggest
//! cuts: there is no per-figure power-share ledger (`head_support`/`plot_support` are
//! two abstract aggregate numbers, not a sum of named shares) and there is no
//! continuously-drifting `regard` ladder (plot leadership reads each kin's existing
//! static `Kin.loyalty` roll instead). Both are documented, not hidden.
use super::*;

/// The heraldic tincture palette, mirrored EXACTLY from `CoatOfArms.tsx`'s
/// `TINCTURES` (same order, same hex) so a crisis's loyalist tint is the same
/// colour the house's own arms already render in.
const CRISIS_TINCTURES: [(&str, &str); 8] = [
    ("gules", "#b32d2d"),
    ("azure", "#2a5fa0"),
    ("vert", "#2f7d44"),
    ("purpure", "#6a3d9a"),
    ("sable", "#1d2733"),
    ("or", "#c9a227"),
    ("argent", "#c8ced6"),
    ("tenné", "#b5651d"),
];
/// The same 16 charges `CoatOfArms.tsx` draws, for the "Tincture + Charge" faction
/// name pattern (`HOUSE_FACTION_NAMING_AND_RECORD.md` §1, pattern 1).
const CRISIS_CHARGES: [&str; 16] = [
    "Lion", "Eagle", "Tower", "Fleur-de-Lis", "Crown", "Mullet", "Sun", "Crescent",
    "Rose", "Garb", "Escallop", "Sword", "Key", "Anchor", "Boar", "Trefoil",
];

/// FNV-1a over a string, matching `CoatOfArms.tsx::hash` bit for bit, so a house's
/// crisis loyalist tint is provably the SAME tincture its arms already render in
/// rather than a coincidentally-similar one.
fn fnv1a(s: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

impl CampaignSim {
    // ── Phase 3.2 · competence + vice ────────────────────────────────────────────
    // Both DERIVED from state Phase 2.1/2.3 already rolled (`kin[0]`'s character +
    // skill) — no third random layer. Competence is just `Kin.skill` read directly
    // at each call site (the crisis roll below, `head_character_factor`'s callers);
    // vice needs a name, so it gets this one function.

    /// The head's vice, if any (`HOUSE_POWER_AND_POLITICS.md` §4) — `VICE_NONE` with
    /// no roster, matching every other Kin-derived query in this file. Checked in the
    /// table's own priority order; a head whose character satisfies more than one
    /// vice's condition (rare — the axes are independent) gets the first that fires.
    pub(crate) fn head_vice(&self, hi: usize) -> u8 {
        let Some(head) = self.houses.get(hi).and_then(|h| h.kin.first()) else { return VICE_NONE; };
        let c = head.character;
        let (bold, greed, civic, rooted) = (c[0], c[1], c[2], c[3]);
        if civic >= 1 && head.skill <= 0.4 { VICE_LAVISH }
        else if bold >= 2 { VICE_RECKLESS }
        else if greed >= 2 { VICE_RAPACIOUS }
        else if bold <= -2 && civic <= -1 { VICE_MISERLY }
        else if rooted <= -2 { VICE_PAROCHIAL }
        else { VICE_NONE }
    }

    /// Magnitude of the vice's contribution to discontent and the crisis roll — a
    /// vice is either present (flat cost) or absent, not gradated, so the number
    /// carries no separate tuning surface beyond `head_vice`'s own thresholds.
    fn vice_severity(&self, hi: usize) -> f32 {
        if self.head_vice(hi) == VICE_NONE { 0.0 } else { 0.55 }
    }

    // ── Phase 3.3-3.6 · the crisis pass ──────────────────────────────────────────

    /// Called monthly. For a house already mid-crisis, progresses it on quarter
    /// boundaries and resolves it at the round cap. For a house that is clear (no
    /// active crisis, past its grace period), checks whether discontent has crossed
    /// the threshold and opens one. A house holds at most one crisis at a time —
    /// `every_crisis_terminates` in `tests.rs` is the invariant this whole function
    /// exists to keep true.
    pub(crate) fn update_house_crises(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            // R2 · a crowned house is never eligible for a MERCHANT succession
            // crisis — its `wealth`/`kin`/discontent inputs are frozen at the
            // coronation, and a DEPOSED outcome would rewrite `head_name` out from
            // under the realm's OWN genealogy (`Realm.family`/`ruler`), corrupting
            // the very identity §5.1 exists to protect. A realm gets its own
            // succession mechanic (`realm_family_pass`), not this one.
            if !self.houses[hi].is_merchant() { continue; }
            if self.houses[hi].crisis.is_some() {
                let opened = self.houses[hi].crisis.as_ref().unwrap().opened_tick;
                if tick > opened && (tick - opened) % CRISIS_ROUND_TICKS == 0 {
                    self.progress_crisis_round(hi);
                }
            } else if tick >= self.houses[hi].crisis_immune_until {
                self.maybe_open_crisis(hi);
            }
        }
    }

    /// `HOUSE_POWER_AND_POLITICS.md` §5's discontent formula, simplified to the state
    /// this codebase actually carries: falling funds (from `wealth_history`), recent
    /// failed goals, the head's own vice, and the most hostile live kinsman's
    /// disloyalty (a proxy for "mean hostility" — see the module doc for why there is
    /// no full `regard` ladder to average over). Returns `(discontent, cause)`.
    fn house_discontent(&self, hi: usize) -> (f32, u8) {
        let h = &self.houses[hi];
        let wh = &h.wealth_history;
        let falling = if wh.len() >= 2 {
            let (prev, cur) = (wh[wh.len() - 2], wh[wh.len() - 1]);
            ((prev - cur) / prev.abs().max(1.0)).clamp(0.0, 1.0)
        } else { 0.0 };
        let recent_goals = &h.goal_history[h.goal_history.len().saturating_sub(3)..];
        let failed = if recent_goals.is_empty() { 0.0 } else {
            recent_goals.iter().filter(|g| g.state == GOAL_FAILED).count() as f32 / recent_goals.len() as f32
        };
        let vice = self.vice_severity(hi);
        let hostile = h.kin.iter()
            .filter(|k| k.role != 0 && k.role != 4 && k.role != 5)
            .map(|k| 1.0 - k.loyalty)
            .fold(0.0f32, f32::max);
        let discontent = 0.45 * falling + 0.25 * failed + 0.20 * hostile + 0.10 * vice;
        // The largest term names the cause — matches whichever grievance the
        // chronicle text and the crisis window should lead with.
        let terms = [(falling, CRISIS_CAUSE_FUNDS), (failed, CRISIS_CAUSE_GOALS),
                     (hostile, CRISIS_CAUSE_HOSTILE), (vice, CRISIS_CAUSE_VICE)];
        let cause = terms.iter().cloned()
            .fold((f32::MIN, CRISIS_CAUSE_FUNDS), |acc, x| if x.0 > acc.0 { x } else { acc }).1;
        (discontent.clamp(0.0, 1.0), cause)
    }

    fn maybe_open_crisis(&mut self, hi: usize) {
        let (discontent, cause) = self.house_discontent(hi);
        if discontent < CRISIS_DISCONTENT_THRESHOLD { return; }
        let tick = self.tick;
        let seed = self.seed;

        // Plot leadership: the least-loyal live, non-head, non-departed kinsman —
        // the module doc's documented stand-in for the design's `regard` ladder.
        // Leaderless discontent (no such kin, or the roster is empty) is real and
        // per the design "much easier to survive" — it just narrows the roll below.
        let plot_leader: i32 = self.houses[hi].kin.iter().enumerate()
            .filter(|(_, k)| k.role != 0 && k.role != 4 && k.role != 5)
            .min_by(|a, b| a.1.loyalty.partial_cmp(&b.1.loyalty).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i as i32).unwrap_or(-1);

        let hostile_norm = if plot_leader >= 0 {
            1.0 - self.houses[hi].kin[plot_leader as usize].loyalty
        } else { 0.0 };
        let vice = self.vice_severity(hi);
        let undecided = 0.12 + hash01(seed, hi as u64, tick as u64 ^ 0x9DEC) * 0.10;
        let plot_support = (0.20 + 0.30 * hostile_norm + 0.20 * vice)
            .clamp(0.10, 1.0 - undecided - 0.15);
        let head_support = 1.0 - plot_support - undecided;

        // Heir choice (`HOUSE_POWER_STRUGGLE_VIEW.md` §3), evaluated once at opening.
        let heir_loyalty = self.houses[hi].kin.iter().find(|k| k.role == 1).map(|k| k.loyalty);
        let heir_choice: u8 = match heir_loyalty {
            Some(l) if l >= 0.60 => 0,
            Some(l) if l < 0.45 => 1,
            Some(_) => if hash01(seed, hi as u64, tick as u64 ^ 0x4E1A) < 0.5 { 0 } else { 1 },
            None => 2,
        };

        let house_name = self.houses[hi].name.clone();
        let head_name = self.houses[hi].head_name.clone();
        let leader_name = if plot_leader >= 0 {
            self.houses[hi].kin[plot_leader as usize].name.clone()
        } else { head_name.clone() };
        let (loyalist_tint_name, loyalist_tint) = house_tincture(&house_name);
        let plot_tint_idx = (CRISIS_TINCTURES.iter().position(|&(n, _)| n == loyalist_tint_name).unwrap_or(0) + 4) % 8;
        let (_, plot_tint) = CRISIS_TINCTURES[plot_tint_idx];
        let loyalist_name = faction_name(&head_name, loyalist_tint_name, seed, hi as u64, tick as u64, 0);
        let plot_name = faction_name(&leader_name, CRISIS_TINCTURES[plot_tint_idx].0, seed, hi as u64, tick as u64, 1);

        self.houses[hi].crisis = Some(HouseCrisis {
            opened_tick: tick, cause, plot_leader, round: 0,
            head_support, plot_support, peak_plot: plot_support,
            rounds: Vec::new(),
            loyalist_name: loyalist_name.clone(), loyalist_tint: loyalist_tint.into(),
            plot_name: plot_name.clone(), plot_tint: plot_tint.into(),
            heir_choice,
        });
        let cause_text = crisis_cause_text(cause);
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "crisis_opened".into(),
            text: format!("A succession crisis opens in {} — {}. {} against {}.",
                house_name, cause_text, loyalist_name, plot_name),
        });
        // Phase 2.4 · crisis SALIENCE (`HOUSE_MASTER_PLAN.md`: "the player cannot
        // watch fourteen houses"). At ~14 houses and a crisis every ~15 years each,
        // that's roughly one crisis a year somewhere — everything cannot surface.
        // Only Tier 1-2 reaches the world news feed; Tier 3-4 (and an as-yet-
        // untiered house, tier 0) is still fully recorded on the HOUSE's own
        // chronicle above, just silent on the world stage, discoverable in the
        // dossier — the same "a healthy gauge stays quiet" discipline everywhere
        // else in this file.
        if matches!(self.houses[hi].tier, 1 | 2) {
            self.journal.push(JournalEntry {
                tick, kind: "crisis".into(), hub: self.houses[hi].hub as i32, good: -1,
                value: plot_support,
                text: format!("{} rises against {} of {}", plot_name, head_name, house_name),
            });
        }
    }

    /// One quarterly round: the head picks an action in character, the roll
    /// resolves it, the undecided bloc is pulled toward whichever side the round
    /// favoured (`HOUSE_FACTION_NAMING_AND_RECORD.md` §2, folded into the same step
    /// rather than a separate pass). At the round cap, resolves the crisis outright.
    fn progress_crisis_round(&mut self, hi: usize) {
        let tick = self.tick;
        let seed = self.seed;
        let mut crisis = self.houses[hi].crisis.clone().unwrap();
        let round = crisis.round as u64;

        let skill = self.houses.get(hi).and_then(|h| h.kin.first()).map(|k| k.skill).unwrap_or(0.5);
        let bold = self.head_axis_pub(hi, 0);
        let greed = self.head_axis_pub(hi, 1);
        let expansive = self.head_axis_pub(hi, 3);
        let wealth = self.houses[hi].wealth;

        // Eligible actions, "chosen when" per `HOUSE_SUCCESSION_CRISIS.md`'s table
        // (Concede/Marry/Press-a-feud cut to the four that need no new subsystem —
        // see the module doc). A hash tiebreak among eligible actions gives round to
        // round variety without pretending the choice is unrelated to character.
        let mut eligible: Vec<u8> = Vec::new();
        if bold <= -1 { eligible.push(CRISIS_ACT_CONCEDE); }
        if greed >= 1 && wealth > CRISIS_BUYOFF_CAP * 2.0 { eligible.push(CRISIS_ACT_BUY_OFF); }
        if bold >= 1 && expansive >= 1 { eligible.push(CRISIS_ACT_VENTURE); }
        let in_character = !eligible.is_empty();
        if eligible.is_empty() { eligible.push(CRISIS_ACT_STAND_FIRM); }
        let pick = (hash01(seed, hi as u64 ^ round, tick as u64 ^ 0x1CE) * eligible.len() as f32) as usize;
        let action = eligible[pick.min(eligible.len() - 1)];

        if action == CRISIS_ACT_BUY_OFF {
            let cost = (wealth * CRISIS_BUYOFF_FRAC).min(CRISIS_BUYOFF_CAP).max(0.0);
            self.houses[hi].wealth -= cost;
        }

        let fit_bonus = if in_character { 0.08 } else { 0.0 };
        let p_work = (0.25 + 0.45 * skill
            + 0.10 * (crisis.head_support - crisis.plot_support)
            + fit_bonus - 0.15 * self.vice_severity(hi)).clamp(0.02, 0.95);
        let roll = hash01(seed, hi as u64 ^ round, tick as u64 ^ 0xC215);
        let worked = roll < p_work;
        let result: i8 = if worked { 1 } else { -1 };

        let (delta, text) = match (action, worked) {
            (CRISIS_ACT_CONCEDE, true) => (0.12, "conceded a holding to the plot — the gesture is taken well".to_string()),
            (CRISIS_ACT_CONCEDE, false) => (-0.06, "conceded a holding — it reads as weakness, not generosity".to_string()),
            (CRISIS_ACT_BUY_OFF, true) => (0.10, "bought off the plot's hardest voices".to_string()),
            (CRISIS_ACT_BUY_OFF, false) => (-0.08, "the money was seen — the plot took it and stayed hostile".to_string()),
            (CRISIS_ACT_VENTURE, true) => (0.18, "launched a venture that restored the family's standing".to_string()),
            (CRISIS_ACT_VENTURE, false) => (-0.15, "launched a venture that failed — the classic overreach".to_string()),
            (CRISIS_ACT_STAND_FIRM, true) => (0.0, "stood firm and named no concession — discontent stalls".to_string()),
            (_, _) => (-0.05, "stood firm — hostility only hardens".to_string()),
        };
        if action == CRISIS_ACT_VENTURE && !worked && self.houses[hi].fleet_sea > 0 {
            self.houses[hi].fleet_sea -= 1;
        }

        crisis.head_support = (crisis.head_support + delta).clamp(0.0, 1.0);
        crisis.plot_support = (crisis.plot_support - delta * 0.6).clamp(0.0, (1.0 - crisis.head_support).max(0.0));
        crisis.peak_plot = crisis.peak_plot.max(crisis.plot_support);
        crisis.rounds.push(CrisisRound { tick, action, result, head_delta: delta, text });
        crisis.round += 1;

        if crisis.round >= CRISIS_ROUND_CAP {
            self.houses[hi].crisis = Some(crisis);
            self.resolve_crisis(hi);
        } else {
            self.houses[hi].crisis = Some(crisis);
        }
    }

    /// Read-only wrapper so `crisis.rs` can read a head's character axis without
    /// duplicating `houses.rs`'s private `head_axis` — kept separate (not just made
    /// `pub(crate)` there) because that one is scoped to goal-picking's own doc
    /// comment and this call site has a different reason to read the same field.
    fn head_axis_pub(&self, hi: usize, axis: usize) -> i8 {
        self.houses.get(hi).and_then(|h| h.kin.first())
            .and_then(|k| k.character.get(axis).copied()).unwrap_or(0)
    }

    /// After the final round: PREVAILED / DEPOSED / DISSOLVED
    /// (`HOUSE_SUCCESSION_CRISIS.md` §2's resolution table, minus the Split outcome —
    /// deferred exactly as `HOUSE_MASTER_PLAN.md` Part 3 already recommends deferring
    /// "Rupture" behind Departure). Always clears the active crisis and pushes a
    /// capped `CrisisRecord`.
    fn resolve_crisis(&mut self, hi: usize) {
        let tick = self.tick;
        let crisis = self.houses[hi].crisis.clone().unwrap();
        let house_name = self.houses[hi].name.clone();

        let (outcome, successor_name) = if crisis.head_support > crisis.plot_support {
            self.houses[hi].crisis_immune_until = tick + CRISIS_GRACE_YEARS * TICKS_PER_YEAR;
            if crisis.plot_leader >= 0 {
                if let Some(k) = self.houses[hi].kin.get_mut(crisis.plot_leader as usize) {
                    k.loyalty = (k.loyalty * 0.3).max(0.0);
                }
            }
            let head_name = self.houses[hi].head_name.clone();
            self.houses[hi].events.push(HouseEvent {
                tick, kind: "crisis_survived".into(),
                text: format!("{} weathers the {} — {} holds the seat, secure until {}",
                    house_name, crisis.plot_name, head_name,
                    (tick + CRISIS_GRACE_YEARS * TICKS_PER_YEAR) / TICKS_PER_YEAR),
            });
            (CRISIS_PREVAILED, head_name)
        } else {
            let kin_empty = self.houses[hi].kin.is_empty();
            let insolvent = self.houses[hi].debt_since != 0 || self.houses[hi].wealth < 0.0;
            if kin_empty && insolvent {
                let old_head = self.houses[hi].head_name.clone();
                self.houses[hi].events.push(HouseEvent {
                    tick, kind: "dissolved".into(),
                    text: format!("{} tears itself apart in the {} — no heir, no funds left to save it", house_name, crisis.plot_name),
                });
                self.dissolve_house(hi);
                (CRISIS_DISSOLVED, old_head)
            } else {
                let (succ_name, succ_female, accession) = self.pick_crisis_successor(hi, crisis.plot_leader);
                let old_head = self.houses[hi].head_name.clone();
                self.depose_and_succeed(hi, succ_name.clone(), succ_female, accession);
                let phrase = match accession {
                    "seized the seat" => format!("{} seized the seat", succ_name),
                    "the succession held" => format!("the succession held — {} takes the seat", succ_name),
                    _ => format!("the house settled on {}", succ_name),
                };
                self.houses[hi].events.push(HouseEvent {
                    tick, kind: "deposed".into(),
                    text: format!("{} is deposed after {} rounds of struggle. {}. {} did not survive the year.",
                        old_head, crisis.rounds.len(), phrase, old_head),
                });
                self.maybe_civic_intervention(hi, &crisis);
                (CRISIS_DEPOSED, succ_name)
            }
        };

        // Phase 2.4 · salience, same rule as the opening — see that call site's
        // comment. A DEPOSED/DISSOLVED outcome always still closes out fully on the
        // house's own permanent record (`events` above, `crisis_history` below);
        // only the world news feed is gated by tier.
        if matches!(self.houses[hi].tier, 1 | 2) {
            self.journal.push(JournalEntry {
                tick, kind: "crisis".into(), hub: self.houses[hi].hub as i32, good: -1,
                value: outcome as f32,
                text: format!("the rising in {} is over — {}", house_name,
                    match outcome { CRISIS_PREVAILED => "the ruler prevails",
                        CRISIS_DEPOSED => "the ruler is deposed", _ => "the house is dissolved" }),
            });
        }

        let record = CrisisRecord {
            opened_tick: crisis.opened_tick, closed_tick: tick, cause: crisis.cause,
            loyalist_name: crisis.loyalist_name, loyalist_tint: crisis.loyalist_tint,
            plot_name: crisis.plot_name, plot_tint: crisis.plot_tint,
            rounds: crisis.round, peak_plot: crisis.peak_plot,
            outcome, successor: successor_name,
        };
        let hist = &mut self.houses[hi].crisis_history;
        hist.push(record);
        if hist.len() > CRISIS_HISTORY_CAP {
            let drop = hist.len() - CRISIS_HISTORY_CAP;
            hist.drain(0..drop);
        }
        self.houses[hi].crisis = None;
    }

    /// Who takes the seat, in the design's own order of likelihood
    /// (`HOUSE_SUCCESSION_CRISIS.md` §2): the plot leader, else the heir (if not
    /// hostile and not the least skilled), else the most able prominent kinsman.
    ///
    /// A deposition names WHO takes over, but it must NOT be able to override WHICH
    /// SEX may hold the house — that is the culture's own `LineRule`
    /// (`sim::inheritance`), and `succeed_house` already guarantees it (a Phase 0.4
    /// invariant: an enatic/matrilineal house is held by women, full stop). A kin
    /// candidate whose sex disagrees with the rule is passed over for a fresh,
    /// correctly-sexed name from the same generator `succeed_house` itself uses,
    /// rather than letting a crisis quietly hand a matrilineal house to a man.
    fn pick_crisis_successor(&self, hi: usize, plot_leader: i32) -> (String, bool, &'static str) {
        let hub = self.houses[hi].hub as usize;
        let (line_rule, _) = self.rules_for_hub(hub);
        let expect_female = crate::sim::inheritance::heir_is_female(
            line_rule, hi as u64 ^ self.tick as u64 ^ 0xDEC0DE, self.seed);
        let sex_ok = |female: bool| line_rule == LineRule::AgnaticCognatic || female == expect_female;

        let kin = &self.houses[hi].kin;
        if plot_leader >= 0 {
            if let Some(k) = kin.get(plot_leader as usize) {
                if sex_ok(k.female) { return (k.name.clone(), k.female, "seized the seat"); }
            }
        }
        let min_skill = kin.iter().map(|k| k.skill).fold(f32::MAX, f32::min);
        if let Some(heir) = kin.iter().find(|k| k.role == 1 && k.loyalty >= 0.25 && k.skill > min_skill && sex_ok(k.female)) {
            return (heir.name.clone(), heir.female, "the succession held");
        }
        if let Some(best) = kin.iter().filter(|k| (k.role == 2 || k.role == 3) && sex_ok(k.female))
            .max_by(|a, b| a.skill.partial_cmp(&b.skill).unwrap_or(std::cmp::Ordering::Equal)) {
            return (best.name.clone(), best.female, "a compromise candidate");
        }
        // No sex-eligible kin to draw from — a synthetic compromise name, same
        // generator every founder/heir name already comes from, correctly sexed.
        let name = self.houses[hi].name.clone();
        (self.head_name_sexed_for(hub, &name, hi as u64 ^ self.tick as u64 ^ 0xC0DE, expect_female), expect_female, "a compromise candidate")
    }

    /// Installs `successor_name` as the new head via the SAME line-closing +
    /// line-opening machinery `succeed_house` uses (`close_head_record` /
    /// `found_head_record`), just with a forced identity instead of a culture-rule
    /// heir pick — a deposition names WHO takes over, not which rule applies.
    fn depose_and_succeed(&mut self, hi: usize, successor_name: String, successor_female: bool, accession: &'static str) {
        let tick = self.tick;
        let hub = self.houses[hi].hub as usize;
        self.close_head_record(hi);
        let (_, inh) = self.rules_for_hub(hub);
        let (age, tenure) = self.roll_tenure(inh, hi as u64 ^ tick as u64 ^ 0xDE90);
        self.houses[hi].generation += 1;
        self.houses[hi].head_name = successor_name;
        self.houses[hi].head_female = successor_female;
        self.houses[hi].head_age = age;
        self.houses[hi].head_since = tick;
        self.houses[hi].head_lifespan = tenure;
        self.found_head_record(hi, accession);
    }

    /// Phase 3.5 · civic intervention. A SEVERE deposition (the plot peaked at ≥0.6)
    /// occasionally draws the seat city's council into sequestering a slice of the
    /// estate amid the disorder — small and rare by construction (§3.5's own gate is
    /// that dynamics stay bounded; this is colour on an already-resolved outcome,
    /// not a second wealth sink).
    fn maybe_civic_intervention(&mut self, hi: usize, crisis: &HouseCrisis) {
        if crisis.peak_plot < 0.6 { return; }
        let tick = self.tick;
        let hub = self.houses[hi].hub as usize;
        if hash01(self.seed, hi as u64, tick as u64 ^ 0x51E2) >= CIVIC_INTERVENTION_CHANCE { return; }
        if hub >= self.hubs.len() { return; }
        let take = (self.houses[hi].wealth.max(0.0) * CIVIC_SEQUESTER_FRAC).max(0.0);
        if take <= 0.0 { return; }
        self.houses[hi].wealth -= take;
        self.hubs[hub].treasury += take;
        let hub_name = self.hubs[hub].name.clone();
        let house_name = self.houses[hi].name.clone();
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "sequestered".into(),
            text: format!("the council of {} sequesters a share of {}'s estate amid the turmoil", hub_name, house_name),
        });
    }
}

fn crisis_cause_text(cause: u8) -> &'static str {
    match cause {
        CRISIS_CAUSE_FUNDS => "years of falling funds",
        CRISIS_CAUSE_GOALS => "a run of failed ambitions",
        CRISIS_CAUSE_VICE => "the head's own vice",
        _ => "a hostile kinsman's discontent",
    }
}

const CRISIS_CAUSE_FUNDS: u8 = 0;
const CRISIS_CAUSE_GOALS: u8 = 1;
const CRISIS_CAUSE_VICE: u8 = 2;
const CRISIS_CAUSE_HOSTILE: u8 = 3;

const CRISIS_ACT_CONCEDE: u8 = 0;
const CRISIS_ACT_BUY_OFF: u8 = 1;
const CRISIS_ACT_VENTURE: u8 = 2;
const CRISIS_ACT_STAND_FIRM: u8 = 3;

/// The house's own tincture (name, hex), FNV-hashed off its name exactly like
/// `CoatOfArms.tsx::houseColor` — this is what lets "the loyalists default to the
/// house's own colour" be literally true rather than a re-roll that happens to
/// usually agree.
fn house_tincture(house_name: &str) -> (&'static str, &'static str) {
    let h = fnv1a(house_name) as usize % CRISIS_TINCTURES.len();
    CRISIS_TINCTURES[h]
}

/// A faction name, drawn from two of the design's six patterns
/// (`HOUSE_FACTION_NAMING_AND_RECORD.md` §1) — "Tincture + Charge" and "Leader's
/// men", the two that need no culture-word table. Deterministic on
/// `(seed, house, opened_tick, camp)`, so a replayed campaign names the same
/// faction the same way a century later.
fn faction_name(leader_name: &str, tint_name: &str, seed: u64, house: u64, opened_tick: u64, camp: u64) -> String {
    let r = hash01(seed, house ^ opened_tick, camp ^ 0xFAC7);
    if r < 0.5 {
        let charge_idx = ((hash01(seed, house, camp ^ 0xC4A6) * CRISIS_CHARGES.len() as f32) as usize)
            .min(CRISIS_CHARGES.len() - 1);
        let charge = CRISIS_CHARGES[charge_idx];
        let tint_cap = {
            let mut c = tint_name.chars();
            match c.next() { Some(f) => f.to_uppercase().collect::<String>() + c.as_str(), None => tint_name.into() }
        };
        format!("the {} {}", tint_cap, charge)
    } else {
        let first = leader_name.split_whitespace().next().unwrap_or(leader_name);
        format!("{}'s men", first)
    }
}
