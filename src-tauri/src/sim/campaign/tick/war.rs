//! war — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

impl CampaignSim {

    /// Raise a forced WAR LEVY from every house homed at `hub`: a slice of each
    /// fortune into the city's war chest (treasury). The core wealth sink of war.
    /// Returns the total raised.
    pub(crate) fn raise_war_levy(&mut self, hub: usize) -> f32 {
        let mut total = 0.0f32;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            if self.houses[hi].hub as usize != hub { continue; }
            let levy = self.houses[hi].wealth.max(0.0) * WAR_LEVY_RATE;
            if levy <= EPS { continue; }
            self.houses[hi].wealth -= levy;
            total += levy;
            if hi < self.house_ledger.len() { self.house_ledger[hi].war_levy += levy; }
        }
        if hub < self.hubs.len() {
            self.hubs[hub].treasury += total;
            self.hubs[hub].finance.war_levy += total;
        }
        total
    }


    /// Spend a slice of `hub`'s treasury on the war effort (consumed — the cost of
    /// armies & blockade). Returns the amount spent (its contribution to victory).
    pub(crate) fn spend_war(&mut self, hub: usize) -> f32 {
        let spend = (self.hubs[hub].treasury.max(0.0) * WAR_SPEND_RATE).min(self.hubs[hub].treasury.max(0.0));
        self.hubs[hub].treasury -= spend;
        self.hubs[hub].finance.spent_war += spend;
        self.hubs[hub].war_effort += spend;
        spend
    }


    /// §3.4d · houses broken by war, on a defeat severe enough to matter (gated by
    /// the caller). Two independent paths, both funnelling into the SAME
    /// `strip_holdings_at` + ruin check — neither invents new state:
    ///
    /// - ENEMY SACK: every live non-guild house resident (`house.hub == lose`) at
    ///   the losing city risks losing what it holds THERE specifically (its own
    ///   estates in that city, offices/bailos/influence there, warehouse stock
    ///   there) — a roll per house, not a guarantee, since not every resident
    ///   family is equally exposed.
    /// - INTERNAL PURGE: the city turns on whichever house actually financed the
    ///   losing war — the house-driven war's own `backer_house` (§3.4c) if this
    ///   was one, else the losing city's own ruling house (`council_house`/
    ///   `captor_house`) for an ordinary rival-council war — guaranteed (a
    ///   targeted political act, not a raid), stripped the same way plus a
    ///   confiscation of a share of its wealth straight into the city's own
    ///   treasury and a real prestige/power cost.
    ///
    /// Either path may cascade to full dissolution through the EXISTING
    /// `dissolve_house`, which already funnels every dissolution path and writes
    /// off outstanding bank loans — no new cascade logic needed.
    fn apply_war_defeat_consequences(&mut self, lose: usize, _win: usize, backer_house: i32, tick: u32) {
        let ln = self.hubs[lose].name.clone();
        let residents: Vec<usize> = (0..self.houses.len())
            .filter(|&hi| !self.houses[hi].defunct && !self.houses[hi].is_guild
                && self.houses[hi].hub as usize == lose)
            .collect();
        for hi in residents {
            if hash01(self.seed, tick as u64 ^ 0x5AC4, hi as u64) > WAR_SACK_CHANCE { continue; }
            let lost = self.strip_holdings_at(hi, lose, WAR_SACK_MAX_ESTATES);
            if lost > EPS {
                if hi < self.house_ledger.len() { self.house_ledger[hi].war_damage += lost; }
                let hn = self.houses[hi].name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "war".into(), hub: lose as i32, good: -1, value: lost,
                    text: format!("{} is sacked in the fall of {}, losing {:.0} in holdings", hn, ln, lost),
                });
                self.houses[hi].events.push(HouseEvent {
                    tick, kind: "war".into(), text: format!("Sacked in the fall of {}", ln),
                });
            }
            if self.house_is_ruined(hi) { self.dissolve_house(hi); }
        }

        let financier = if backer_house >= 0 { backer_house }
            else { self.hubs[lose].council_house.max(self.hubs[lose].captor_house) };
        if financier < 0 { return; }
        let fi = financier as usize;
        if fi >= self.houses.len() || self.houses[fi].defunct || self.houses[fi].is_guild { return; }
        let stripped = self.strip_holdings_at(fi, lose, WAR_PURGE_MAX_ESTATES);
        let confiscated = (self.houses[fi].wealth.max(0.0) * WAR_PURGE_CONFISCATE_FRAC).max(0.0);
        self.houses[fi].wealth -= confiscated;
        self.hubs[lose].treasury += confiscated;
        self.houses[fi].political_power = (self.houses[fi].political_power - WAR_PURGE_POWER_LOSS).max(0.0);
        let total = stripped + confiscated;
        if total > EPS {
            if fi < self.house_ledger.len() { self.house_ledger[fi].war_damage += total; }
            let hn = self.houses[fi].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "war".into(), hub: lose as i32, good: -1, value: total,
                text: format!("{} is purged and stripped of office in {} for financing the losing war", hn, ln),
            });
            self.houses[fi].events.push(HouseEvent {
                tick, kind: "war".into(), text: format!("Purged in {} for financing the losing war", ln),
            });
        }
        if self.house_is_ruined(fi) { self.dissolve_house(fi); }
    }


    /// Strip house `hi`'s holdings sited specifically at `city` — up to
    /// `max_estates` of its own estates there (ownership passes to the city,
    /// `owner_house = -1`, the same "confiscated" convention the resale market
    /// uses), any office/bailo/influence it holds there, and any warehouse stock
    /// depot at that city (stock is lost, not spilled to the local pool — it is
    /// PLUNDERED, not liquidated). Returns the wealth-equivalent value stripped,
    /// for the Accountant ledger and the journal.
    fn strip_holdings_at(&mut self, hi: usize, city: usize, max_estates: usize) -> f32 {
        let mut lost = 0.0f32;
        let estates: Vec<usize> = (0..self.hubs.len())
            .filter(|&ei| self.hubs[ei].is_estate && !self.hubs[ei].abandoned
                && self.hubs[ei].owner_house == hi as i32 && self.hubs[ei].parent == city as i32)
            .take(max_estates)
            .collect();
        for ei in estates {
            lost += self.estate_market_value(ei);
            self.hubs[ei].owner_house = -1;
        }
        let cu = city as u32;
        self.houses[hi].offices.retain(|&o| o != cu);
        self.houses[hi].bailos.retain(|&o| o != cu);
        self.houses[hi].influence.retain(|&(c, _)| c != cu);
        for w in self.warehouses.iter_mut() {
            if w.owner == hi as i32 && w.hub == cu {
                lost += w.stock.iter().sum::<f32>();
                w.stock.iter_mut().for_each(|s| *s = 0.0);
            }
        }
        lost
    }


    /// A house left with no wealth AND no productive assets/offices anywhere has
    /// nothing left to rebuild from — the honest trigger for §3.4d's cascade into
    /// `dissolve_house`, distinct from the ordinary insolvency check
    /// (`update_solvency`) which reads wealth alone: a war can strip a house's
    /// ASSETS while leaving it technically solvent for a while longer, and that
    /// house is still ruined in every way that matters.
    fn house_is_ruined(&self, hi: usize) -> bool {
        if self.houses[hi].defunct || self.houses[hi].wealth > EPS { return false; }
        let has_estate = self.hubs.iter().any(|h| h.is_estate && !h.abandoned && h.owner_house == hi as i32);
        let has_office = !self.houses[hi].offices.is_empty() || !self.houses[hi].bailos.is_empty();
        !has_estate && !has_office
    }


    /// §3.4e · war damages one of `hub`'s own estates/manufactories this year —
    /// reuses the EXISTING `TickHub.damage` field exactly as a natural disaster
    /// would (see `estate_condition_pass`, which already repairs ANY nonzero
    /// damage whatever its cause, so no new repair machinery is needed here).
    /// Smaller than a single disaster (a siege nibbles, it doesn't level the
    /// works) but recurs every year the war lasts. A house-owned estate's loss
    /// is booked to that house's Accountant ledger (`war_damage`); a civic or
    /// unowned works still takes the damage, just with no ledger line to book it
    /// to — the journal entry is the record either way.
    fn war_damage_pass(&mut self, hub: usize) {
        if hash01(self.seed, self.tick as u64 ^ 0xDA3A9E, hub as u64) > WAR_DAMAGE_CHANCE { return; }
        let candidates: Vec<usize> = (0..self.hubs.len())
            .filter(|&ei| self.hubs[ei].is_estate && !self.hubs[ei].abandoned
                && self.hubs[ei].parent == hub as i32
                && self.hubs[ei].estate_tier > 0 && self.hubs[ei].damage < 0.8)
            .collect();
        if candidates.is_empty() { return; }
        let pick = ((hash01(self.seed, self.tick as u64 ^ 0x9E57, hub as u64) * candidates.len() as f32) as usize)
            .min(candidates.len() - 1);
        let ei = candidates[pick];
        let r = hash01(self.seed, self.tick as u64 ^ 0x0DA3, ei as u64);
        let dmg = WAR_DAMAGE_MIN + r * (WAR_DAMAGE_MAX - WAR_DAMAGE_MIN);
        let before = self.hubs[ei].damage;
        self.hubs[ei].damage = (before + dmg).clamp(0.0, 1.0);
        let inflicted = self.hubs[ei].damage - before;
        if inflicted <= EPS { return; }
        let owner = self.hubs[ei].owner_house;
        if owner >= 0 && (owner as usize) < self.house_ledger.len() {
            self.house_ledger[owner as usize].war_damage += inflicted * self.estate_market_value(ei);
        }
        let en = self.hubs[ei].name.clone();
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "war".into(), hub: hub as i32, good: -1, value: inflicted,
            text: format!("War damages {} ({:.0}% harm)", en, inflicted * 100.0),
        });
    }


    /// Apply a resolved war's GOAL — the victor's lasting spoils beyond the one-off
    /// plunder. Returns a short clause appended to the journal / Wars-log text.
    /// Trade rights & annexation both work through the BAILO primitive (a foreign
    /// governing foothold), so the winner's house is seated on the loser's council
    /// by the ordinary yearly recompute and the control transfer sticks.
    /// R4 · true when `h` is the CAPITAL of a still-standing realm — the one case
    /// every goal below treats specially, since a capital's own sovereignty (and
    /// its dynasty's provinces/family) is a bigger thing to move than a war goal
    /// alone should cascade through. Reused by ANNEX and VASSALIZE to gate the
    /// deliberately-deferred "conquer a whole realm" case (see their own comments).
    fn hub_is_realm_capital(&self, h: usize) -> bool {
        let ri = self.hubs[h].realm;
        ri >= 0 && self.realms.get(ri as usize)
            .map(|r| r.fallen_tick == 0 && r.capital_hub as usize == h).unwrap_or(false)
    }

    pub(crate) fn apply_war_goal(&mut self, win: usize, lose: usize, goal: u8, tick: u32, _yr: u32) -> String {
        let (wn, ln) = (self.hubs[win].name.clone(), self.hubs[lose].name.clone());
        // R4 · the victor's TRUE ruler at a sovereign capital is its crown, not
        // whichever house currently tops the ordinary civic-capture tally —
        // `update_government`'s bribery loop does not yet know to stand aside for
        // a realm's own seat (a real, narrow gap named here rather than silently
        // worked around: a rival house could in principle still out-bribe the
        // crown's own presence at its capital; fixing that is its own change).
        let ruler = if self.hub_is_realm_capital(win) {
            let ri = self.hubs[win].realm as usize;
            // A CIVIC realm has no dynasty (`ruling_house` is `u32::MAX`), so a
            // sovereign hub's "true ruler" may genuinely be nobody. Resolve it
            // through `houses.get` and fall through to the ordinary council/
            // strongest-house path when there is no crowned family to award to —
            // indexing it raw would panic the moment a republic won a war.
            match self.houses.get(self.realms[ri].ruling_house as usize) {
                Some(_) => Some(self.realms[ri].ruling_house as usize),
                None => {
                    let c = self.hubs[win].council_house;
                    if c >= 0 && (c as usize) < self.houses.len() { Some(c as usize) }
                    else { self.strongest_house_at(win) }
                }
            }
        } else {
            let c = self.hubs[win].council_house;
            if c >= 0 && (c as usize) < self.houses.len() { Some(c as usize) }
            else { self.strongest_house_at(win) }
        };
        let grant_bailo = |me: &mut Self, hi: usize| {
            if !me.houses[hi].bailos.contains(&(lose as u32)) {
                me.houses[hi].bailos.push(lose as u32);
            }
        };
        match goal {
            WAR_GOAL_HUMILIATE => {
                // R4 · purely reputational: no land, no coin beyond the ordinary
                // reparations already taken above this call. A realm's own
                // legitimacy is what cracks; an ordinary house-led city's is its
                // ruling house's prestige.
                if self.hub_is_realm_capital(lose) {
                    let ri = self.hubs[lose].realm as usize;
                    self.realms[ri].legitimacy = (self.realms[ri].legitimacy - HUMILIATE_LEGITIMACY_HIT).max(0.0);
                } else if let Some(hi) = self.strongest_house_at(lose) {
                    self.houses[hi].prestige = (self.houses[hi].prestige - HUMILIATE_PRESTIGE_HIT).max(0.0);
                }
                if self.hub_is_realm_capital(win) {
                    let ri = self.hubs[win].realm as usize;
                    self.realms[ri].legitimacy = (self.realms[ri].legitimacy + HUMILIATE_LEGITIMACY_GAIN).min(1.0);
                } else if let Some(hi) = ruler {
                    self.houses[hi].prestige += HUMILIATE_PRESTIGE_GAIN;
                }
                format!("; {} is humiliated before {} — its standing visibly cracks", ln, wn)
            }
            WAR_GOAL_ENTHRONE => {
                // R4 · a puppet, not a conquest: the winner's own kin takes the
                // loser's head seat (`Official.kin`, already unbribable once set —
                // `update_government` step 3 locks it at control 1.0) for
                // `ENTHRONE_TERM_YEARS`, then the ordinary regime-change clock
                // (`reseat_official`) takes back over with no special unwind
                // needed. The loser keeps its coin, market and nominal standing.
                let Some(hi) = ruler else { return String::new(); };
                if self.hubs[lose].officials.is_empty() { self.seed_government(lose); }
                let placed = self.hubs[lose].officials.iter_mut().find(|o| o.role == 0);
                match placed {
                    Some(head) => {
                        head.house = hi as i32;
                        head.control = 1.0;
                        head.kin = true;
                        head.term_end = tick + ENTHRONE_TERM_YEARS * TICKS_PER_YEAR;
                        let hn = self.houses[hi].name.clone();
                        format!("; {} seats a kinsman of {} on the throne of {}", wn, hn, ln)
                    }
                    None => String::new(),
                }
            }
            WAR_GOAL_TRIBUTE => {
                self.hubs[lose].tribute_to = win as i32;
                self.hubs[lose].tribute_until = tick + TRIBUTE_YEARS * TICKS_PER_YEAR;
                format!("; {} is made a tributary of {} for {} years", ln, wn, TRIBUTE_YEARS)
            }
            WAR_GOAL_VASSALIZE => {
                // R4 · stronger than tribute: the FULL relationship (fights in the
                // overlord's wars, may not declare its own — `Realm.vassals` +
                // `REALM_ROLE_TRIBUTARY`) only forms when the winner itself has a
                // realm to be a vassal OF, and the loser is not itself a realm's
                // own capital (conquering a whole foreign crown into vassalage is
                // the same deferred cascade ANNEX defers below — a real design
                // question about what happens to the loser's own dynasty/family,
                // not a guard to improvise here). Otherwise this downgrades
                // quietly to plain tribute, the same idiom `WAR_GOAL_PROVINCE`
                // already uses when there's nothing to cede.
                self.hubs[lose].tribute_to = win as i32;
                self.hubs[lose].tribute_until = tick + TRIBUTE_YEARS * TICKS_PER_YEAR;
                if self.hub_is_realm_capital(win) && !self.hub_is_realm_capital(lose) {
                    let ri = self.hubs[win].realm as usize;
                    if !self.realms[ri].vassals.contains(&(lose as u32)) {
                        self.realms[ri].vassals.push(lose as u32);
                    }
                    self.hubs[lose].realm = ri as i32;
                    self.hubs[lose].realm_role = REALM_ROLE_TRIBUTARY;
                    format!("; {} becomes a vassal of {} — bound to its wars, its own voice silenced", ln, wn)
                } else {
                    format!("; {} has no crown to answer to — {} settles for tribute alone", wn, ln)
                }
            }
            WAR_GOAL_TRADE_RIGHTS => {
                if let Some(hi) = ruler {
                    grant_bailo(self, hi);
                    let hn = self.houses[hi].name.clone();
                    format!("; {} wins trade rights in {} — a bailo for {}", wn, ln, hn)
                } else { String::new() }
            }
            WAR_GOAL_PROVINCE => {
                // §3.4b · take ONE province the loser held — short of the whole city.
                // Only an ordinary city-administered province is up for grabs (rule 24:
                // a house-held writ, `prov_holder_house >= 0`, belongs to that house,
                // not to whichever city loses a war). The richest (most rural population)
                // qualifying province is the prize.
                let Some(hi) = ruler else { return String::new(); };
                let prize = (0..self.prov_holder.len())
                    .filter(|&p| self.prov_holder[p] == lose as i32
                        && self.prov_holder_house.get(p).copied().unwrap_or(-1) < 0)
                    .max_by(|&x, &y| self.prov_rural.get(x).copied().unwrap_or(0.0)
                        .partial_cmp(&self.prov_rural.get(y).copied().unwrap_or(0.0)).unwrap());
                match prize {
                    Some(p) => {
                        self.prov_holder[p] = win as i32;
                        // R4 · a ceded province leaves its OLD sovereignty (if any)
                        // and, if the WINNER has a realm, joins that one instead —
                        // cession is a territorial transfer, and `prov_realm` must
                        // never keep pointing at a realm that no longer wins or
                        // administers this land (rule 25).
                        if p < self.prov_realm.len() {
                            self.prov_realm[p] = self.hubs[win].realm;
                        }
                        let pn = self.province_name(p);
                        format!("; {} cedes {} to {}", ln, pn, wn)
                    }
                    // The loser held no ordinary province to cede — downgrade quietly to
                    // the trade-rights clause rather than award nothing at all.
                    None => {
                        grant_bailo(self, hi);
                        let hn = self.houses[hi].name.clone();
                        format!("; {} had no province to cede — {} takes trade rights instead, a bailo for {}", ln, wn, hn)
                    }
                }
            }
            WAR_GOAL_ANNEX => {
                let Some(hi) = ruler else { return format!("; {} is annexed by {}", ln, wn); };
                grant_bailo(self, hi);
                self.hubs[lose].council_house = hi as i32;
                self.hubs[lose].coin_trust = (self.hubs[lose].coin_trust - 0.15).max(0.0);
                let hn = self.houses[hi].name.clone();
                // R4 · annexing a realm's own CAPITAL — a full conquest of another
                // crown, its dynasty and its family — is deliberately NOT built in
                // this pass (the same deferral `WAR_GOAL_VASSALIZE` names above).
                // An ORDINARY member/subject city changing hands is: it leaves
                // whatever realm held it (if any) and, if the WINNER has a realm,
                // joins that one instead; the provinces IT administered move with
                // it, exactly as a ceded province does above.
                if !self.hub_is_realm_capital(lose) {
                    self.hubs[lose].realm = self.hubs[win].realm;
                    self.hubs[lose].realm_role = if self.hubs[win].realm >= 0 { REALM_ROLE_SUBJECT } else { 0 };
                    for p in 0..self.prov_holder.len() {
                        if self.prov_holder[p] != lose as i32 { continue; }
                        if p < self.prov_realm.len() { self.prov_realm[p] = self.hubs[win].realm; }
                    }
                }
                format!("; {} is annexed by {} — {} installed on its council", ln, wn, hn)
            }
            _ => String::new(),
        }
    }


    /// §3.4a · which named exhaustion path (if any) side `h` has hit this call —
    /// "force broken" (this year's own war effort has collapsed against its own
    /// past peak) or "treasury and credit spent" (state coffers AND resident
    /// private wealth both drained). Neither invents a troop-count field; both
    /// read state the tick already carries.
    fn war_side_exhaustion(&self, h: usize, peak_effort: f32, effort_this_year: f32) -> Option<&'static str> {
        // R4 · a sovereign capital's crown treasury counts too (see `war_
        // affordable_treasury`) — otherwise R3's redirect of dues away from
        // `hub.treasury` would read every realm as permanently "treasury spent".
        let treasury = self.war_affordable_treasury(h);
        // rule 25 · a crowned house's wealth is frozen at 0 (it left the merchant
        // world), so `is_merchant()` rather than `!defunct` alone keeps this a
        // rank-of-live-merchants figure rather than counting a dynasty shell.
        let credit: f32 = self.houses.iter()
            .filter(|house| house.is_merchant() && house.hub as usize == h)
            .map(|house| house.wealth.max(0.0)).sum();
        if treasury < WAR_FINANCIAL_EPS && credit < WAR_FINANCIAL_EPS {
            return Some("treasury and credit spent");
        }
        if peak_effort > WAR_FINANCIAL_EPS && effort_this_year < peak_effort * WAR_FORCE_BROKEN_FRAC {
            return Some("force broken");
        }
        None
    }

    /// §3.4a/b · a war has ended (by any path). Clears belligerency, awards
    /// reparations + the score-priced goal (or a white peace below the reparations
    /// price, §1.4's Outcomes row), logs it, and — for a colony's war of
    /// independence — resolves freedom or a cooldown exactly as before.
    /// `winner = None` is a mutual white peace: both sides are named but neither
    /// gains — used when both belligerents hit the same exhaustion/weariness
    /// condition in the same round, or the round cap closes an unresolved
    /// near-deadlock.
    fn resolve_war(&mut self, wi: usize, winner: Option<usize>, reason: &str, tick: u32, yr: u32) {
        let a = self.wars[wi].a as usize;
        let b = self.wars[wi].b as usize;
        self.hubs[a].war_with = -1;
        self.hubs[b].war_with = -1;
        // §3.4f · "a real grievance" — neither side has one again until the cooldown
        // lapses, so the same two cities cannot cycle straight back into a rematch.
        let cooldown_until = tick + WAR_COOLDOWN_YEARS * TICKS_PER_YEAR;
        self.hubs[a].war_cooldown_until = cooldown_until;
        self.hubs[b].war_cooldown_until = cooldown_until;
        let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
        let score_abs = self.wars[wi].score.abs();
        let declared_goal = self.wars[wi].goal;
        let independence = self.wars[wi].cause == "independence";
        let cause = self.wars[wi].cause.clone();
        let levies_total = self.wars[wi].levies;
        let start_tick = self.wars[wi].start_tick;
        let backer_house = self.wars[wi].backer_house;

        let (win, lose, rep, spoils, awarded_goal) = match winner {
            Some(win) if score_abs >= WAR_PRICE_REPARATIONS => {
                let lose = if win == a { b } else { a };
                let rep = (self.hubs[lose].treasury.max(0.0) * 0.4).max(0.0);
                self.hubs[lose].treasury -= rep;
                self.hubs[win].treasury += rep;
                self.hubs[win].finance.reparations_in += rep;
                self.hubs[lose].finance.reparations_out += rep;
                self.hubs[lose].coin_trust = (self.hubs[lose].coin_trust - 0.15).max(0.0);
                // §3.4b · terms priced in score: the richest goal the FINAL score
                // affords, capped by what was originally declared — overperforming
                // never upgrades the war's own aim, it only guarantees reaching it.
                let richest_affordable = [
                    WAR_GOAL_ANNEX, WAR_GOAL_PROVINCE, WAR_GOAL_VASSALIZE, WAR_GOAL_TRIBUTE,
                    WAR_GOAL_ENTHRONE, WAR_GOAL_TRADE_RIGHTS, WAR_GOAL_HUMILIATE, WAR_GOAL_PLUNDER,
                ].into_iter().find(|&g| war_goal_price(g) <= score_abs).unwrap_or(WAR_GOAL_PLUNDER);
                let awarded = if war_goal_price(declared_goal) <= score_abs { declared_goal } else { richest_affordable };
                let spoils = self.apply_war_goal(win, lose, awarded, tick, yr);
                // §3.4d · a defeat severe enough to have earned tribute (score ≥ 40)
                // is severe enough to break houses over — sack + internal purge,
                // deliberately the plan's own highest-risk item, gated so it does
                // not fire on every white-peace-adjacent skirmish.
                if score_abs >= WAR_PRICE_TRIBUTE {
                    self.apply_war_defeat_consequences(lose, win, backer_house, tick);
                }
                (Some(win), Some(lose), rep, spoils, awarded)
            }
            // A winner was named but the score never reached the reparations floor
            // (10) — a WHITE PEACE: the fight happened, but nobody had anything
            // worth taking.
            _ => (None, None, 0.0, String::new(), WAR_GOAL_PLUNDER),
        };
        let _ = awarded_goal;

        let text = match (win, lose) {
            (Some(win), Some(lose)) => {
                let (wn, ln) = (self.hubs[win].name.clone(), self.hubs[lose].name.clone());
                format!("The war of {} and {} ends in year {} ({}): {} prevails, {} pays {:.0} in reparations{}.",
                    an, bn, yr, reason, wn, ln, rep, spoils)
            }
            _ => format!("The war of {} and {} ends in year {} in a WHITE PEACE ({}) — neither side gains.",
                an, bn, yr, reason),
        };
        self.journal.push(JournalEntry {
            tick, kind: "war".into(), hub: win.map(|w| w as i32).unwrap_or(-1), good: -1,
            value: rep, text: text.clone(),
        });
        self.war_log.push(WarRecord {
            start_year: start_tick / TICKS_PER_YEAR, end_year: yr,
            a_name: an, b_name: bn,
            winner: win.map(|w| self.hubs[w].name.clone()).unwrap_or_else(|| "neither".into()),
            loser: lose.map(|l| self.hubs[l].name.clone()).unwrap_or_else(|| "neither".into()),
            reparations: rep, levies_total, cause, text,
        });
        if self.war_log.len() > WAR_LOG_CAP {
            let drop = self.war_log.len() - WAR_LOG_CAP;
            self.war_log.drain(0..drop);
        }
        // War of independence: the colony either wins free, or is brought to heel
        // for 15 years before it may rebel again. A white peace (no clear winner)
        // reads the same as failing to win outright — the colony stays a colony.
        if independence {
            let colony = if self.hubs[a].colony_kind == 1 { a }
                else if self.hubs[b].colony_kind == 1 { b } else { usize::MAX };
            if colony != usize::MAX {
                if win == Some(colony) {
                    self.make_colony_independent(colony, true);
                } else {
                    self.hubs[colony].indep_cooldown_until = tick + 15 * TICKS_PER_YEAR;
                    let cn = self.hubs[colony].name.clone();
                    self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: colony as i32,
                        good: -1, value: 0.0, text: format!("{}'s bid for independence is crushed; it remains a colony", cn) });
                }
            }
        }
    }


    /// DLC 3.5 · the economic-war engine — §3.4a wraps it in a score + quarterly
    /// rounds. Once a year: wage (levies, war-chest spending, trade blockade —
    /// unchanged) and occasionally declare a new war. Then, since the LAST year
    /// processed, run every quarterly round now due (`WAR_ROUND_TICKS` apart,
    /// tick-driven rather than a fixed 4/call so a back-dated `start_tick` still
    /// catches up correctly — the same trick the crisis engine and its own tests
    /// rely on), each a battle/raid/blockade/occupation outcome that moves the
    /// war's bidirectional score, checked after every round against: a decisive
    /// score (±100), the four independent exhaustion paths (§1.4), and finally
    /// the round cap — the termination guarantee of last resort. Deterministic.
    pub(crate) fn update_wars(&mut self, yr: u32) {
        let tick = self.tick;
        // Tributaries pay their overlords first — a bounded treasury→treasury
        // transfer (never minted), lapsing when the term ends.
        for h in 0..self.hubs.len() {
            let to = self.hubs[h].tribute_to;
            if to < 0 { continue; }
            if tick >= self.hubs[h].tribute_until || to as usize >= self.hubs.len() {
                self.hubs[h].tribute_to = -1;
                continue;
            }
            let to = to as usize;
            let cap = TRIBUTE_CAP * self.city_size_factor(h);
            let pay = (self.hubs[h].treasury.max(0.0) * TRIBUTE_RATE).min(cap);
            if pay <= EPS { continue; }
            self.hubs[h].treasury -= pay;
            self.hubs[to].treasury += pay;
            self.hubs[h].finance.reparations_out += pay;
            self.hubs[to].finance.reparations_in += pay;
        }
        let mut ended: Vec<usize> = Vec::new();
        for wi in 0..self.wars.len() {
            let (a, b) = (self.wars[wi].a as usize, self.wars[wi].b as usize);
            if a >= self.hubs.len() || b >= self.hubs.len() {
                ended.push(wi); continue;
            }
            // Wage: levies on each side's houses, war-chest spending, trade blockade —
            // once a year, unchanged from before §3.4a.
            let raised_a = self.raise_war_levy(a);
            let raised_b = self.raise_war_levy(b);
            self.wars[wi].levies += raised_a + raised_b;
            self.wars[wi].levies_a += raised_a;
            self.wars[wi].levies_b += raised_b;
            let spent_a = self.spend_war(a);
            let spent_b = self.spend_war(b);
            self.wars[wi].chest_a += spent_a;
            self.wars[wi].chest_b += spent_b;
            for &h in &[a, b] {
                // §3.4e · the real, PERSISTENT blockade. `trade_wealth` is recomputed
                // fresh from `export_earn`/`import_spend` every single day
                // (`update_houses`), so a one-off `trade_wealth *= 0.8` here is
                // erased before a player could ever see it — kept for its immediate
                // display value, but `export_earn` is what actually has to shrink for
                // the blockade to bite for the rest of the year.
                self.hubs[h].trade_wealth *= 0.8;
                self.hubs[h].export_earn *= WAR_BLOCKADE_EXPORT_MULT;
                self.active_events.push(ActiveEvent {
                    kind: "war".into(), hub: h as i32, good: -1,
                    magnitude: 0.4, until_tick: tick + TICKS_PER_YEAR,
                });
                self.war_damage_pass(h);
            }
            // §3.4e · the neutral WAR BOOM — a hub sharing a belligerent's trade
            // component, itself at peace, profits from supplying the war. Exactly
            // why a house wants to supply a war it is not fighting (§2).
            for h in 0..self.hubs.len() {
                if self.hubs[h].war_with >= 0 { continue; }
                if self.hubs[h].component != self.hubs[a].component
                    && self.hubs[h].component != self.hubs[b].component { continue; }
                self.hubs[h].export_earn += self.hubs[h].export_earn.max(0.0) * WAR_BOOM_EXPORT_FRAC
                    + WAR_BOOM_EXPORT_FLAT;
            }
            let effort_a = raised_a + spent_a;
            let effort_b = raised_b + spent_b;
            self.wars[wi].peak_effort_a = self.wars[wi].peak_effort_a.max(effort_a);
            self.wars[wi].peak_effort_b = self.wars[wi].peak_effort_b.max(effort_b);

            // §3.4a · run every quarterly round now due.
            let mut end: Option<(Option<usize>, &'static str)> = None;
            loop {
                let due = (self.wars[wi].round as u64 + 1) * WAR_ROUND_TICKS as u64;
                if due > tick.saturating_sub(self.wars[wi].start_tick) as u64 { break; }
                self.wars[wi].round += 1;
                let strength_a = self.wars[wi].chest_a + self.hubs[a].treasury.max(0.0) + 1.0;
                let strength_b = self.wars[wi].chest_b + self.hubs[b].treasury.max(0.0) + 1.0;
                let bias = strength_a / (strength_a + strength_b);
                let salt = ((wi as u64) << 16) ^ self.wars[wi].round as u64;
                let roll_kind = hash01(self.seed, tick as u64 ^ 0x0A2D, salt);
                // Occupation only reads once a side already holds a real advantage —
                // it is the outcome of ALREADY winning, not a random opener.
                let occupying = self.wars[wi].score.abs() > 40.0;
                // §3.4a tuning (see WAR_MIN_ROUNDS_TO_RESOLVE's doc comment): the
                // original 24/16/8/11 magnitudes let a lopsided pair reach the
                // decisive ±100 score in a handful of rounds, which is what actually
                // drove "wars started / century" to 50-65 despite four successive
                // preconditions on DECLARING a war — the volume was never about how
                // often a war started, only about how fast one finished. Halved so a
                // decisive win takes roughly the whole round-cap window on average,
                // matching the old fixed-2-year mechanism's rough pace.
                let mut mag = if occupying && roll_kind < 0.20 { 12.0 }
                    else if roll_kind < 0.45 { 8.0 }
                    else if roll_kind < 0.75 { 4.0 }
                    else { 5.5 };
                // Duration varies with the MATCHUP (§3.4a / WAR_IMBALANCE_ESCALATION).
                // A lopsided war escalates fast and ends decisively in a few rounds;
                // an even one grinds toward the cap. Without this every war ran the
                // full round cap (~3-4 yr) because the halved magnitudes could never
                // reach the ±100 decisive score inside the window.
                let imbalance = ((bias - 0.5).abs() * 2.0).clamp(0.0, 1.0);
                mag *= 1.0 + WAR_IMBALANCE_ESCALATION * imbalance;
                let roll_dir = hash01(self.seed, tick as u64 ^ 0x7EED, salt ^ 0x51DE);
                let delta = if roll_dir < bias { mag } else { -mag };
                self.wars[wi].score = (self.wars[wi].score + delta).clamp(-100.0, 100.0);
                // §3.4a · record the round as a "battle" for the panel's history.
                let (b_round, b_score) = (self.wars[wi].round, self.wars[wi].score);
                self.wars[wi].battles.push(WarBattle {
                    round: b_round,
                    year: tick / TICKS_PER_YEAR,
                    favored: if delta >= 0.0 { 0 } else { 1 },
                    delta,
                    score_after: b_score,
                    decisive: mag >= 10.0,
                });

                if self.wars[wi].score.abs() >= WAR_SCORE_DECISIVE {
                    let w = if self.wars[wi].score >= 0.0 { a } else { b };
                    end = Some((Some(w), "decisive victory"));
                } else if self.wars[wi].round < WAR_MIN_ROUNDS_TO_RESOLVE {
                    // A real war takes at least a year to exhaust either side — see
                    // `WAR_MIN_ROUNDS_TO_RESOLVE`'s own doc comment. Only a genuine
                    // decisive score (above) may end it faster than that.
                } else {
                    let a_exh = self.war_side_exhaustion(a, self.wars[wi].peak_effort_a, effort_a);
                    let b_exh = self.war_side_exhaustion(b, self.wars[wi].peak_effort_b, effort_b);
                    let a_weary = self.hubs[a].mood < WAR_MOOD_WEARY_FLOOR;
                    let b_weary = self.hubs[b].mood < WAR_MOOD_WEARY_FLOOR;
                    let backer = self.wars[wi].backer_house;
                    let backer_withdrew = backer >= 0 && self.houses.get(backer as usize)
                        .map(|h| h.defunct || h.wealth <= WAR_BACKER_INSOLVENT).unwrap_or(true);
                    if a_exh.is_some() && b_exh.is_some() {
                        end = Some((None, a_exh.unwrap()));
                    } else if let Some(r) = a_exh {
                        end = Some((Some(b), r));
                    } else if let Some(r) = b_exh {
                        end = Some((Some(a), r));
                    } else if a_weary && b_weary {
                        end = Some((None, "war weariness"));
                    } else if a_weary {
                        end = Some((Some(b), "war weariness"));
                    } else if b_weary {
                        end = Some((Some(a), "war weariness"));
                    } else if backer_withdrew {
                        let backer_hub = self.houses.get(backer as usize).map(|h| h.hub);
                        let w = if backer_hub == Some(a as u32) { b } else { a };
                        end = Some((Some(w), "its backer's ruin"));
                    } else if self.wars[wi].round >= WAR_ROUND_CAP {
                        // Past the ordinary war length a war of attrition CONTINUES while
                        // BOTH sides can still afford it — a natural long war between two
                        // rich, determined states. It settles here once a side's chest runs
                        // low, and can never grind past the absolute hard cap (rule 22: no
                        // war is permanent). The exhaustion / weariness / decisive checks
                        // above still end most wars well before either cap.
                        let a_flush = self.war_affordable_treasury(a) >= WAR_ATTRITION_MIN_CHEST;
                        let b_flush = self.war_affordable_treasury(b) >= WAR_ATTRITION_MIN_CHEST;
                        if self.wars[wi].round >= WAR_ROUND_HARD_CAP || !(a_flush && b_flush) {
                            let w = if self.wars[wi].score.abs() < WAR_PRICE_REPARATIONS { None }
                                else if self.wars[wi].score >= 0.0 { Some(a) } else { Some(b) };
                            end = Some((w, "the round cap — neither side could finish it"));
                        }
                        // else: both flush and under the hard cap → another round of attrition.
                    }
                }
                if end.is_some() { break; }
            }
            if let Some((winner, reason)) = end {
                self.resolve_war(wi, winner, reason, tick, yr);
                ended.push(wi);
            }
        }
        for &wi in ended.iter().rev() { self.wars.remove(wi); }
        self.maybe_declare_war(yr);
    }


    /// Occasionally ignite a new economic war between two rival poleis in the same
    /// region, both at peace. Rival councils are the spark; prosperity is the prize.
    /// §3.4c · a WARMONGER RULER (a bold council head, `head_character_factor` axis
    /// 0) raises the odds a candidate pair actually comes to blows this year.
    /// True when two seats are close enough (and same trade component) to wage a
    /// real war — see `WAR_MAX_DIST_FRAC`. Cylindrical in X. A pre-colonial city
    /// cannot fight one an ocean away, so this gates every path that starts a war.
    /// R4 · a hub's own war-AFFORDABILITY, including its crown's treasury when the
    /// hub is a sovereign CAPITAL. R3 redirected the tithe/poll/customs away from
    /// a sovereign capital's own `hub.treasury` into `Realm.treasury` — reading
    /// `hub.treasury` alone here would make every realm systematically too poor to
    /// even DECLARE a war, which is exactly backwards for a phase whose whole job
    /// is making realms fight. Scoped to ELIGIBILITY/EXHAUSTION checks only — it
    /// does NOT redirect actual chest-SPENDING (`raise_war_levy`/`spend_war` still
    /// draw only from `hub.treasury`, unchanged). Pooling the crown's money into
    /// the actual war effort, and doing the same for non-capital member cities, is
    /// "one war, one score, many cities" — real, named, deliberately deferred to a
    /// later pass rather than attempted half-verified here (plan §7).
    pub(crate) fn war_affordable_treasury(&self, h: usize) -> f32 {
        let base = self.hubs[h].treasury.max(0.0);
        let ri = self.hubs[h].realm;
        if ri >= 0 {
            if let Some(r) = self.realms.get(ri as usize) {
                if r.fallen_tick == 0 && r.capital_hub as usize == h {
                    return base + r.treasury.max(0.0);
                }
            }
        }
        base
    }

    pub(crate) fn hubs_within_war_reach(&self, a: usize, b: usize) -> bool {
        if a >= self.hubs.len() || b >= self.hubs.len() { return false; }
        if self.hubs[a].component != self.hubs[b].component { return false; }
        let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
        if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
        let dy = self.hubs[a].y - self.hubs[b].y;
        let cap = self.world_w * WAR_MAX_DIST_FRAC;
        dx * dx + dy * dy <= cap * cap
    }

    pub(crate) fn maybe_declare_war(&mut self, yr: u32) {
        if self.wars.len() >= MAX_ACTIVE_WARS { return; }
        let n = self.hubs.len();
        // Candidate seats: real cities with a council, at peace, and — §3.4f's own
        // "sufficient treasury" precondition — enough in the chest to field more
        // than a single quarterly round before financial exhaustion ends it for free.
        let seats: Vec<usize> = (0..n).filter(|&h|
            !self.hubs[h].is_estate && self.hubs[h].war_with < 0
            && self.hubs[h].council_house >= 0 && self.hubs[h].population > 1.0
            && self.war_affordable_treasury(h) >= WAR_MIN_TREASURY
            && self.hubs[h].war_cooldown_until <= self.tick
        ).collect();
        if seats.len() < 2 { return; }
        // Prefer a pair in the same region whose councils are rivals; else any pair.
        let mut best: Option<(usize, usize, &'static str)> = None;
        for (ii, &a) in seats.iter().enumerate() {
            for &b in seats.iter().skip(ii + 1) {
                // Same component AND within marching/blockade reach — a pre-colonial
                // city cannot war one across an ocean or a continent.
                if !self.hubs_within_war_reach(a, b) { continue; }
                let ca = self.hubs[a].council_house as usize;
                let cb = self.hubs[b].council_house as usize;
                let rivals = self.houses.get(ca).map(|h| h.rivals.contains(&cb)).unwrap_or(false)
                    || self.houses.get(cb).map(|h| h.rivals.contains(&ca)).unwrap_or(false);
                if rivals { best = Some((a, b, "rival councils")); break; }
                if best.is_none() { best = Some((a, b, "trade dispute")); }
            }
            if matches!(best, Some((_, _, "rival councils"))) { break; }
        }
        let Some((a, b, cause)) = best else { return };
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        let boldness = |me: &Self, h: usize| -> f32 {
            let c = me.hubs[h].council_house;
            if c >= 0 { me.head_character_factor(c as usize, 0) } else { 1.0 }
        };
        let chance = WAR_DECLARE_CHANCE * ((boldness(self, a) + boldness(self, b)) / 2.0);
        if hash01(self.seed, self.tick as u64 ^ 0xDEC1A6E, yr as u64) > chance { return; }
        // A WAR GOAL — what the aggressor is after. Rival councils fight for political
        // supremacy (annexation / trade rights); a plain trade dispute is fought for
        // tribute or plunder. A lopsided match leans toward annexation (the strong
        // swallow the weak); an even one toward tribute. Deterministic.
        let pow = |me: &Self, h: usize| me.hubs[h].population.max(1.0) * (me.hubs[h].treasury.max(0.0) + 5.0);
        let (pa, pb) = (pow(self, a), pow(self, b));
        let ratio = pa.max(pb) / pa.min(pb).max(1.0);
        let g = hash01(self.seed, self.tick as u64 ^ 0x60A15, (a * 131 + b) as u64);
        let goal = if cause == "rival councils" {
            if ratio >= 2.0 && g < 0.6 { WAR_GOAL_ANNEX } else { WAR_GOAL_TRADE_RIGHTS }
        } else if ratio >= 2.5 && g < 0.4 {
            WAR_GOAL_ANNEX
        } else if g < 0.6 {
            WAR_GOAL_TRIBUTE
        } else {
            WAR_GOAL_PLUNDER
        };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "war".into(), hub: a as i32, good: -1, value: 0.0,
            text: format!("{} declares war on {} ({} · {})", an, bn, cause, war_goal_label(goal)),
        });
        self.wars.push(War {
            a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, levies_a: 0.0, levies_b: 0.0,
            battles: Vec::new(), cargo_lost: 0, cause: cause.into(), goal,
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1,
        });
    }


    /// §3.4c · a house-driven war: the winner of a vendetta-stage feud, holding its
    /// city's council or captor seat, drags that city into a full war against the
    /// loser's city — with itself automatically committed as BACKER. Its own
    /// insolvency is the "backers withdraw" exhaustion path for this particular war.
    fn declare_house_war(&mut self, a: usize, b: usize, backer_house: usize, rival_house: usize) {
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
        let hn = self.houses[backer_house].name.clone();
        let rn = self.houses[rival_house].name.clone();
        let text = format!("{}'s feud with {} drags {} into war against {}", hn, rn, an, bn);
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "war".into(), hub: a as i32, good: -1, value: 0.0, text,
        });
        self.wars.push(War {
            a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, levies_a: 0.0, levies_b: 0.0,
            battles: Vec::new(), cargo_lost: 0,
            cause: "a house's war".into(), goal: WAR_GOAL_TRADE_RIGHTS,
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0,
            backer_house: backer_house as i32,
        });
    }


    /// Monthly warehouse pass (Phase 2): (1) ensure every live house has a home depot
    /// and one in each office city; (2) STOCK — each house draws a slice of its
    /// specialty goods' LOCAL SURPLUS (never food, never below the city's trade
    /// reserve) into its depot, paying the market (the cost circulates into the
    /// city's civic pool) — this is the inventory a futures contract later ships out
    /// of; (3) EXPAND — a profitable house enlarges a depot that stays nearly full.
    /// Stocking only MOVES goods within a hub (pool → house depot), so the aggregate
    /// `hub_stock` — and thus prices, needs and the famine balance — is unchanged.
    pub(crate) fn sync_and_stock_warehouses(&mut self, needs: &[Vec<f32>]) {
        let ng = self.goods.len();
        let nh = self.houses.len();
        // Slowly heal cosmetic damage on standing depots.
        for w in &mut self.warehouses { w.damage *= 0.98; }
        // (1) Ensure home + office depots exist. A single membership set of existing
        //     (owner, hub) pairs replaces the old per-call linear scan, so this is
        //     O(houses + offices) rather than O(houses · warehouses).
        let nhub = self.hubs.len();
        let mut have: std::collections::HashSet<(i32, u32)> =
            self.warehouses.iter().map(|w| (w.owner, w.hub)).collect();
        let mut new_depots: Vec<(i32, u32)> = Vec::new();
        for hi in 0..nh {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub;
            if (home as usize) < nhub && have.insert((hi as i32, home)) {
                new_depots.push((hi as i32, home));
            }
            for off in self.houses[hi].offices.clone() {
                if (off as usize) < nhub && have.insert((hi as i32, off)) {
                    new_depots.push((hi as i32, off));
                }
            }
        }
        for (owner, hub) in new_depots {
            self.warehouses.push(Warehouse {
                owner, hub, capacity: WH_START_CAP,
                stock: vec![0.0; ng], tier: Self::capacity_tier(WH_START_CAP), damage: 0.0,
            });
        }
        // (2) Stocking.
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= nh || self.houses[oi].defunct { continue; }
            let hub = self.warehouses[wi].hub as usize;
            if hub >= self.hubs.len() { continue; }
            // `needs` is sized to the hub count at the consumption pass; a hub added
            // later this tick (a fresh estate/colony) has no needs row yet — skip it
            // this tick rather than index out of bounds.
            if hub >= needs.len() { continue; }
            let used: f32 = self.warehouses[wi].stock.iter().sum();
            let mut room = (self.warehouses[wi].capacity - used).max(0.0);
            if room <= EPS { continue; }
            for g in self.houses[oi].spec.clone() {
                if room <= EPS { break; }
                if g >= ng || self.goods[g].food { continue; }
                let reserve = needs[hub][g] * TRADE_RESERVE_MULT;
                let surplus = (stock_of(&self.hubs[hub].stock, g) - reserve).max(0.0);
                if surplus <= EPS { continue; }
                let price = self.live_price(self.hub_stock(hub, g), needs[hub][g], self.goods[g].base_value);
                let afford = if price > EPS { (self.houses[oi].wealth * 0.25).max(0.0) / price } else { 0.0 };
                let take = (surplus * WH_STOCK_FRAC).min(room).min(afford);
                if take <= EPS { continue; }
                stock_take(&mut self.hubs[hub].stock, g, take);
                self.warehouses[wi].stock[g] += take;
                let cost = take * price;
                self.houses[oi].wealth -= cost;
                self.hubs[hub].civic_pool += cost;
                room -= take;
            }
        }
        // (3) Expansion.
        let tick = self.tick;
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= nh || self.houses[oi].defunct { continue; }
            let cap = self.warehouses[wi].capacity;
            if cap >= WH_MAX_CAP { continue; }
            let used: f32 = self.warehouses[wi].stock.iter().sum();
            if used < cap * WH_FULL_FRAC { continue; }
            let cost = WH_EXPAND_COST * self.warehouses[wi].tier.max(1) as f32;
            if self.houses[oi].wealth < cost * 1.5 { continue; }
            if hash01(self.seed, tick as u64 ^ 0x3A5E, wi as u64) > 0.25 { continue; }
            self.houses[oi].wealth -= cost;
            let newcap = (cap * WH_EXPAND_MULT).min(WH_MAX_CAP);
            self.warehouses[wi].capacity = newcap;
            self.warehouses[wi].tier = Self::capacity_tier(newcap);
        }
    }


    /// A thriving, crowded city sends SWARM_POP_FRAC of its people to break new
    /// ground: an INDEPENDENT town (no charter, no lifeline) on the best farmable
    /// free site within short range. At most one founding a year worldwide so each
    /// stays a legible chronicle event.
    pub(crate) fn maybe_swarm_town(&mut self) {
        if self.colonizable.is_empty() { return; }
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || hub.colony_kind != 0 { continue; }
            if hub.population < SWARM_MIN_POP
                || hub.population < hub.founding_pop * SWARM_PRESSURE { continue; }
            if hub.starving > 0.05 || hub.mood < 0.5 { continue; }
            let score = hub.population / hub.founding_pop.max(1.0) * (0.5 + hub.mood);
            if score > best.1 { best = (h, score); }
        }
        let Some(mother) = (best.0 != usize::MAX).then_some(best.0) else { return };
        let cap = self.world_w * SWARM_REACH_FRAC;
        let (mx, my) = (self.hubs[mother].x, self.hubs[mother].y);
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            if s.fertility < SWARM_MIN_FERTILE { continue; }
            let mut dx = (s.x - mx).abs();
            if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
            let d = (dx * dx + (s.y - my).powi(2)).sqrt();
            // Not beyond a settler's walk, and not on the mother's doorstep either.
            if d > cap || d < cap * 0.15 { continue; }
            let score = (0.3 + s.fertility + 0.5 * s.trade_value) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        let site = self.colonizable.remove(si);
        let seed_pop = (self.hubs[mother].population * SWARM_POP_FRAC).max(600.0);
        self.hubs[mother].population -= seed_pop;
        let mother_name = self.hubs[mother].name.clone();
        let idx = self.create_organic_town(mother, &site, seed_pop);
        let new_name = self.hubs[idx].name.clone();
        self.total_foundings += 1;
        // Naturally-spawned free towns are a MERCHANT venture (related to a house/
        // guild), unlike the city-founded DEPENDENT satellites: name the sponsoring
        // house/guild in the beat so the tie reads in the chronicle.
        let sponsor = self.strongest_house_at(mother).map(|hi| self.houses[hi].name.clone());
        let text = match &sponsor {
            Some(hn) => format!("Merchants of {} lead settlers from {} to found {}", hn, mother_name, new_name),
            None => format!("Settlers from {} found {}", mother_name, new_name),
        };
        self.journal.push(JournalEntry { tick: self.tick, kind: "founding".into(),
            hub: idx as i32, good: -1, value: seed_pop, text });
    }


    /// Open a war of independence between a colony and its metropolis (reuses the
    /// economic-war machinery; resolved in `update_wars`, where chest+treasury+luck
    /// decide it — a colony can still win against the odds).
    pub(crate) fn declare_independence_war(&mut self, colony: usize, metro: usize) {
        let (a, b) = if colony < metro { (colony, metro) } else { (metro, colony) };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        self.wars.push(War { a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, levies_a: 0.0, levies_b: 0.0,
            battles: Vec::new(), cargo_lost: 0, cause: "independence".into(),
            goal: WAR_GOAL_PLUNDER,
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1 });
        let (cn, mn) = (self.hubs[colony].name.clone(), self.hubs[metro].name.clone());
        self.journal.push(JournalEntry { tick: self.tick, kind: "war".into(), hub: colony as i32,
            good: -1, value: 0.0, text: format!("{} rises in a war of independence against {}", cn, mn) });
    }


    // ═══════════════════════════════════════════════════════════════════════════
    //  FEUDS
    //
    //  The old model was one symmetric `rivals` entry plus a 15%-per-half-year roll
    //  in which the weaker house lost 8% of its wealth. That is a tax on being poor:
    //  it had no cause, no memory, no escalation, and no ending short of one side
    //  dying. What follows keeps `rivals` in sync — every existing consumer (war
    //  causes in `pick_war_pair`, marriage eligibility, the Houses panel) reads the
    //  same field it always did — and adds the four things a quarrel between merchant
    //  families actually has:
    //
    //    · a CAUSE      (`FEUD_TRADE` … `FEUD_SUCCESSION`) — feuds are ABOUT something
    //    · an INTENSITY that heats with live overlap and cools without it
    //    · STAGES whose weapons differ: a cold rivalry is words, a vendetta burns ships
    //    · a SETTLEMENT — arbitration by a council, a marriage, ruin, or plain neglect
    //
    //  Historically this is the Italian/Hanseatic pattern: quarrels between trading
    //  families were adjudicated by the commune they both traded in, because an open
    //  feud between two big houses was a threat to the city's own commerce. A feud
    //  that can only end when someone dies produces a world where every old house has
    //  a dozen rivals; one that can be settled produces a world with a HISTORY.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Index of the live feud between `a` and `b`, if any (order-insensitive).
    pub(crate) fn feud_between(&self, a: usize, b: usize) -> Option<usize> {
        let (lo, hi) = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        self.feuds.iter().position(|f| f.a == lo && f.b == hi && f.outcome == FEUD_RUNNING)
    }

    /// How much two houses actually get in each other's way right now, plus the good
    /// and the city the quarrel is most about. Overlap drives heating: two houses that
    /// no longer trade the same goods in the same places stop feuding, which is what
    /// lets a feud cool on its own.
    pub(crate) fn feud_overlap(&self, a: usize, b: usize) -> (f32, i32, i32) {
        if a >= self.houses.len() || b >= self.houses.len() { return (0.0, -1, -1); }
        let (ha, hb) = (&self.houses[a], &self.houses[b]);
        if ha.defunct || hb.defunct { return (0.0, -1, -1); }
        // Shared trade: the goods both live off. The FIRST shared good names the feud.
        let mut good = -1i32;
        let mut shared_goods = 0u32;
        for g in &ha.spec {
            if hb.spec.contains(g) {
                shared_goods += 1;
                if good < 0 { good = *g as i32; }
            }
        }
        // Shared ground: cities both stand in (seat, offices, bailos). The one where
        // their combined influence is highest is the city the feud is about.
        let nodes = |h: &House| -> Vec<u32> {
            let mut v = Vec::with_capacity(1 + h.offices.len() + h.bailos.len());
            v.push(h.hub);
            v.extend(h.offices.iter().copied());
            v.extend(h.bailos.iter().copied());
            v
        };
        let (na, nb) = (nodes(ha), nodes(hb));
        let inf_at = |h: &House, c: u32| h.influence.iter().find(|(x, _)| *x == c)
            .map(|(_, v)| *v).unwrap_or(0.0);
        let mut shared_cities = 0u32;
        let mut hub = -1i32;
        let mut best_inf = -1.0f32;
        for &c in &na {
            if !nb.contains(&c) { continue; }
            shared_cities += 1;
            let combined = inf_at(ha, c) + inf_at(hb, c);
            // Ties break on the LOWER hub index so the choice is order-independent.
            if combined > best_inf { best_inf = combined; hub = c as i32; }
        }
        // A feud needs real CONTACT (#14): the two families must either share a city
        // (seat / office / bailo) OR sit within trading reach of each other. Two
        // houses that merely deal the same GOOD on opposite sides of the world have no
        // quarrel to have — without this a shared commodity alone (0.22 × 2 goods =
        // 0.44, over the 0.30 feud threshold) paired up houses an ocean apart. A feud
        // that LOSES contact returns 0 here and so cools, which is correct.
        if shared_cities == 0
            && !self.hubs_within_war_reach(ha.hub as usize, hb.hub as usize) {
            return (0.0, good, hub);
        }
        // Same trading component is a weak tie on its own — it is what made the old
        // model pair up every house on a continent. It counts, but only a little.
        let same_component = self.hubs.get(ha.hub as usize).map(|h| h.component)
            == self.hubs.get(hb.hub as usize).map(|h| h.component);
        let overlap = (0.22 * shared_goods.min(3) as f32
            + 0.30 * shared_cities.min(3) as f32
            + if same_component { 0.10 } else { 0.0 }).clamp(0.0, 1.0);
        (overlap, good, hub)
    }

    /// Open (or re-heat) a feud between two houses for a named reason. Every path that
    /// creates bad blood — a soured match, a closed market, a contested council — comes
    /// through here, so a feud always knows why it exists.
    pub(crate) fn open_feud(&mut self, a: usize, b: usize, cause: u8, good: i32, hub: i32,
                            heat: f32) {
        if a == b || a >= self.houses.len() || b >= self.houses.len() { return; }
        if self.houses[a].defunct || self.houses[b].defunct { return; }
        let (lo, hi) = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        if let Some(fi) = self.feud_between(a, b) {
            // Already quarrelling — a fresh grievance pours heat on the existing feud
            // rather than starting a second one.
            self.feuds[fi].intensity = (self.feuds[fi].intensity + heat).min(1.0);
            let cur = self.feuds[fi].stage;
            self.feuds[fi].stage = feud_stage(self.feuds[fi].intensity, cur);
            return;
        }
        if self.feuds.iter().filter(|f| f.outcome == FEUD_RUNNING).count() >= FEUDS_CAP { return; }
        self.feuds.push(Feud {
            a: lo, b: hi, cause, good, hub,
            intensity: heat.clamp(0.0, 1.0),
            stage: feud_stage(heat, FEUD_COLD),
            started_tick: self.tick, last_flare_tick: 0, flares: 0,
            damage_a: 0.0, damage_b: 0.0,
            outcome: FEUD_RUNNING, ended_tick: 0, log: Vec::new(),
        });
        if !self.houses[a].rivals.contains(&b) { self.houses[a].rivals.push(b); }
        if !self.houses[b].rivals.contains(&a) { self.houses[b].rivals.push(a); }
        let (na, nb) = (self.houses[a].name.clone(), self.houses[b].name.clone());
        let why = FEUD_CAUSES.get(cause as usize).copied().unwrap_or("an old grievance");
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "feud".into(), hub: self.houses[a].hub as i32,
            good, value: 0.0,
            text: format!("{} and {} fall out over {}", na, nb, why),
        });
        for (h, other) in [(a, nb.clone()), (b, na.clone())] {
            self.houses[h].events.push(HouseEvent {
                tick: self.tick, kind: "feud".into(),
                text: format!("Fell out with {} over {}", other, why),
            });
        }
    }

    /// Close a feud with a stated outcome, clearing the rival entries on both sides so
    /// the rest of the sim sees the quarrel as genuinely over.
    pub(crate) fn close_feud(&mut self, fi: usize, outcome: u8) {
        if fi >= self.feuds.len() || self.feuds[fi].outcome != FEUD_RUNNING { return; }
        let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
        self.feuds[fi].outcome = outcome;
        self.feuds[fi].ended_tick = self.tick;
        if a < self.houses.len() { self.houses[a].rivals.retain(|&r| r != b); }
        if b < self.houses.len() { self.houses[b].rivals.retain(|&r| r != a); }
    }

    /// Every feud a (now defunct) house was part of ends in its ruin. Called from
    /// `dissolve_house` so a dead family does not leave live quarrels behind.
    pub(crate) fn end_feuds_of(&mut self, hi: usize) {
        let idx: Vec<usize> = self.feuds.iter().enumerate()
            .filter(|(_, f)| f.outcome == FEUD_RUNNING
                && (f.a as usize == hi || f.b as usize == hi))
            .map(|(i, _)| i).collect();
        for fi in idx { self.close_feud(fi, FEUD_RUINED); }
    }

    /// Half-yearly · FORMATION. Scan house pairs for a reason to fall out. This is the
    /// only O(n²) part of the feud system and it keeps the old cadence; the per-feud
    /// work below runs monthly over the bounded `feuds` list instead.
    pub(crate) fn update_rivalries(&mut self) {
        let n = self.houses.len();
        // Old saves (and any state written before feuds existed) carry rival pairs with
        // no feud object. Adopt them once, as trade feuds already part-way heated, so a
        // loaded campaign does not silently lose its quarrels.
        if self.feuds.is_empty() {
            let mut adopt: Vec<(usize, usize)> = Vec::new();
            for a in 0..n {
                if self.houses[a].defunct { continue; }
                for &b in &self.houses[a].rivals {
                    if b > a && b < n && !self.houses[b].defunct { adopt.push((a, b)); }
                }
            }
            for (a, b) in adopt {
                let (_, good, hub) = self.feud_overlap(a, b);
                self.open_feud(a, b, FEUD_TRADE, good, hub, 0.35);
            }
        }
        for a in 0..n {
            if self.houses[a].defunct { continue; }
            for b in (a + 1)..n {
                if self.houses[b].defunct { continue; }
                if self.feud_between(a, b).is_some() { continue; }
                // Allied houses do not start quarrels with each other.
                let (lo, hi) = (a.min(b) as u32, a.max(b) as u32);
                if self.alliances.contains(&(lo, hi)) { continue; }
                let (overlap, good, hub) = self.feud_overlap(a, b);
                if overlap < 0.30 { continue; } // brushing past each other is not a feud
                // A contested COUNCIL is a sharper cause than mere competition: both
                // families are courting the same city's seats.
                let contested = hub >= 0 && {
                    let inf = |h: usize| self.houses[h].influence.iter()
                        .find(|(c, _)| *c as i32 == hub).map(|(_, v)| *v).unwrap_or(0.0);
                    inf(a) >= 0.12 && inf(b) >= 0.12
                };
                let cause = if contested { FEUD_SEAT } else { FEUD_TRADE };
                // Not every overlap becomes a quarrel — some houses simply coexist.
                let roll = hash01(self.seed, self.tick as u64 ^ (a as u64) << 12, b as u64);
                if roll > overlap * 0.55 { continue; }
                self.open_feud(a, b, cause, good, hub, 0.10 + 0.20 * overlap);
            }
        }
    }

    /// Monthly · TEMPERATURE + FLARES. Runs over the bounded feud list: heat each live
    /// feud by how much the two houses still get in each other's way, cool it when they
    /// no longer do, re-derive the stage, and occasionally let it flare.
    pub(crate) fn update_feuds(&mut self) {
        if self.feuds.is_empty() { return; }
        let tick = self.tick;
        let mut forget: Vec<usize> = Vec::new();
        for fi in 0..self.feuds.len() {
            if self.feuds[fi].outcome != FEUD_RUNNING { continue; }
            let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
            if a >= self.houses.len() || b >= self.houses.len()
                || self.houses[a].defunct || self.houses[b].defunct {
                self.close_feud(fi, FEUD_RUINED);
                continue;
            }
            let (overlap, good, hub) = self.feud_overlap(a, b);
            // Phase 2.4 · the greedier of the two heads (axis 1) heats the quarrel
            // faster; the more honourable cools it — averaged since a feud has two
            // sides, ±15% capped.
            let heat_mod = (self.head_character_factor(a, 1) + self.head_character_factor(b, 1)) / 2.0;
            {
                let f = &mut self.feuds[fi];
                if overlap > 0.0 {
                    f.intensity = (f.intensity + FEUD_HEAT * heat_mod * overlap).min(1.0);
                    // Keep the feud pointed at what it is currently about — two houses
                    // whose quarrel has moved to a new market should say so.
                    if good >= 0 { f.good = good; }
                    if hub >= 0 { f.hub = hub; }
                } else {
                    f.intensity = (f.intensity - FEUD_COOL).max(0.0);
                }
                f.stage = feud_stage(f.intensity, f.stage);
            }
            if self.feuds[fi].intensity < FEUD_FORGET { forget.push(fi); continue; }
            let stage = self.feuds[fi].stage as usize;
            let roll = hash01(self.seed, tick as u64 ^ ((a as u64) << 20), b as u64);
            if roll < FEUD_FLARE_CHANCE[stage] { self.feud_flare(fi); }
        }
        for fi in forget { self.close_feud(fi, FEUD_COOLED); }
        // A settled feud is kept for the record but must not grow without bound.
        if self.feuds.len() > FEUDS_CAP * 2 {
            let drop = self.feuds.len() - FEUDS_CAP * 2;
            let mut removed = 0usize;
            self.feuds.retain(|f| {
                if removed < drop && f.outcome != FEUD_RUNNING { removed += 1; false } else { true }
            });
        }
    }

    /// One flare. The stage decides the weapon — this is where the elaboration earns
    /// its keep, because "what a feud DOES" now depends on how bad it has become.
    fn feud_flare(&mut self, fi: usize) {
        let tick = self.tick;
        let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
        let stage = self.feuds[fi].stage;
        // The stronger house prevails — by wealth, with prestige and political weight
        // counting for something (a poorer but better-connected family can win).
        let power = |h: &House| h.wealth.max(0.0) + h.prestige * 400.0 + h.political_power * 600.0;
        let (winner, loser) = if power(&self.houses[a]) >= power(&self.houses[b]) { (a, b) } else { (b, a) };
        let (wn, ln) = (self.houses[winner].name.clone(), self.houses[loser].name.clone());
        // Limited liability: a bite is a share of what the loser actually has, so a
        // feud can impoverish a house but never drive it arbitrarily negative.
        let bite = self.houses[loser].wealth.max(0.0) * FEUD_BITE[stage as usize];
        self.houses[loser].wealth -= bite;
        // Prestige is UNBOUNDED and feeds political power → charters → monopolies →
        // wealth, so a per-flare award is a compounding loop, not flavour. The old
        // model paid +0.03 at ~0.3 flares/yr ≈ 0.009/yr; paying +0.008..0.032 at up to
        // 3.4 flares/yr came to ~0.11/yr, and the dynamics run's sustained-richest house
        // went from 298k to 1.9M — a monopoly runaway driven entirely by feud prestige.
        // Kept deliberately close to the old annual rate, and capped, so winning feuds
        // makes a family respected without making it unassailable.
        let gain = (0.002 + 0.002 * stage as f32).min(FEUD_PRESTIGE_CAP - self.houses[winner].prestige);
        self.houses[winner].prestige += gain.max(0.0);
        self.houses[loser].prestige = (self.houses[loser].prestige - 0.004 * stage as f32).max(0.0);
        let good_name = {
            let g = self.feuds[fi].good;
            if g >= 0 { self.goods.get(g as usize).map(|x| x.name.clone()) } else { None }
        };
        let city = self.feuds[fi].hub;
        let city_name = if city >= 0 {
            self.hubs.get(city as usize).map(|h| h.name.clone()).unwrap_or_default()
        } else { String::new() };

        let text = match stage {
            FEUD_COLD => match &good_name {
                Some(g) => format!("{} snubs {} over the {} trade", wn, ln, g),
                None => format!("{} snubs {} at the exchange", wn, ln),
            },
            FEUD_OPEN => {
                // Undercutting: the loser's recent volume — the basis of its market
                // share and monopolies — is cut back.
                self.houses[loser].volume *= 0.94;
                match &good_name {
                    Some(g) => format!("{} undercuts {} in {}, taking the trade", wn, ln, g),
                    None => format!("{} undercuts {} and takes its custom", wn, ln),
                }
            }
            FEUD_TRADEWAR => {
                self.houses[loser].volume *= 0.90;
                // Strip the loser's standing in the contested city, and — where the
                // winner governs it — close the market outright (the pre-existing
                // trade-war path, now driven by a feud that EARNED it).
                let mut barred_here = false;
                if city >= 0 {
                    let c = city as u32;
                    if let Some(e) = self.houses[loser].influence.iter_mut().find(|(x, _)| *x == c) {
                        e.1 = (e.1 - FEUD_INFLUENCE_STRIP).max(0.0);
                    }
                    let governs = self.hubs.get(c as usize)
                        .map(|h| h.captor_house == winner as i32).unwrap_or(false)
                        || (self.houses[winner].dominant_seat && self.houses[winner].hub == c);
                    if governs && !self.houses[loser].is_guild {
                        let already = self.house_barred.get(loser).is_some_and(|v| v.contains(&c));
                        if !already {
                            if let Some(v) = self.house_barred.get_mut(loser) { v.push(c); }
                            barred_here = true;
                        }
                    }
                }
                if barred_here {
                    self.journal.push(JournalEntry {
                        tick, kind: "trade_war".into(), hub: city, good: -1, value: 0.0,
                        text: format!("{} bars {} from the market of {}", wn, ln, city_name),
                    });
                    format!("{} bars {} from the market of {}", wn, ln, city_name)
                } else if !city_name.is_empty() {
                    format!("{} drives {} out of the {} trade", wn, ln, city_name)
                } else {
                    format!("{} shuts {} out of its markets", wn, ln)
                }
            }
            _ => {
                // §3.4c · a HOUSE-DRIVEN WAR: the winner of a vendetta escalation, if it
                // holds its OWN city's council or captor seat, may drag that whole city
                // into a full state war on the loser's — "capturing a government is what
                // lets a family spend a city's blood on its own quarrel" (§5's Tiers
                // note). Gated on the two houses actually sitting in different cities,
                // neither city already at war, and room under the active-war cap; the
                // roll is drawn unconditionally so it stays deterministic regardless of
                // which branch runs.
                let wh = self.houses[winner].hub as usize;
                let lh = self.houses[loser].hub as usize;
                let can_escalate = wh != lh && wh < self.hubs.len() && lh < self.hubs.len()
                    && self.hubs[wh].war_with < 0 && self.hubs[lh].war_with < 0
                    && self.wars.len() < MAX_ACTIVE_WARS
                    // A feud can span the map (houses share goods, not geography),
                    // but a STATE war between their cities cannot — the two seats
                    // must be within marching reach. This is the gate the
                    // house-driven path was missing, and the main source of
                    // cross-continent wars.
                    && self.hubs_within_war_reach(wh, lh)
                    && self.hubs[wh].treasury >= WAR_MIN_TREASURY
                    && self.hubs[wh].war_cooldown_until <= tick && self.hubs[lh].war_cooldown_until <= tick
                    && (self.hubs[wh].council_house == winner as i32
                        || self.hubs[wh].captor_house == winner as i32);
                let war_roll = hash01(self.seed, tick as u64 ^ 0x9A5, ((winner as u64) << 8) ^ loser as u64);
                if can_escalate && war_roll < HOUSE_WAR_CHANCE {
                    self.declare_house_war(wh, lh, winner, loser);
                    format!("{}'s feud with {} boils over into open war between {} and {}",
                        wn, ln, self.hubs[wh].name, self.hubs[lh].name)
                } else {
                    // Vendetta: property. A ship taken, or a foreign office forced shut —
                    // the two things that actually cost a merchant house its reach.
                    self.houses[loser].volume *= 0.88;
                    let pick = hash01(self.seed, tick as u64, (loser as u64) << 8);
                    let fleet = self.houses[loser].fleet_sea + self.houses[loser].fleet_river
                        + self.houses[loser].fleet_caravan;
                    if fleet > 0 && pick < 0.6 {
                        let sea = self.houses[loser].fleet_sea > 0;
                        self.damage_fleet(loser, sea);
                        format!("{} has a {} of {} taken at sea", wn,
                            if sea { "ship" } else { "caravan" }, ln)
                    } else {
                        // Force a foreign counting-house shut. Only a lease-free office can
                        // be taken; a leased one is held open by its term, and one backing a
                        // live contract stays open — the same rules the office system uses.
                        let victim = self.houses[loser].offices.iter().copied().find(|&o| {
                            (city < 0 || o as i32 == city)
                                && !self.office_leased(loser, o)
                                && !self.backs_active_contract(loser, o)
                        });
                        match victim {
                            Some(off) => {
                                let on = self.hubs.get(off as usize).map(|h| h.name.clone())
                                    .unwrap_or_default();
                                self.houses[loser].offices.retain(|&o| o != off);
                                self.houses[loser].bailos.retain(|&o| o != off);
                                self.houses[loser].influence.retain(|&(c, _)| c != off);
                                format!("{} forces {}'s counting-house in {} to close", wn, ln, on)
                            }
                            None => format!("{} sets its bravos on {}'s factors", wn, ln),
                        }
                    }
                }
            }
        };

        {
            let f = &mut self.feuds[fi];
            f.flares += 1;
            f.last_flare_tick = tick;
            if loser as u32 == f.a { f.damage_a += bite; } else { f.damage_b += bite; }
            f.log.push(FeudFlare { tick, stage, loser: loser as u32, cost: bite, text: text.clone() });
            if f.log.len() > FEUD_LOG_CAP { let d = f.log.len() - FEUD_LOG_CAP; f.log.drain(0..d); }
        }
        // Only the loud stages reach the world chronicle; a cold snub belongs to the
        // two families' own records, not to the history of the world.
        if stage >= FEUD_TRADEWAR {
            self.journal.push(JournalEntry {
                tick, kind: "feud".into(), hub: self.houses[winner].hub as i32,
                good: self.feuds[fi].good, value: bite, text: text.clone(),
            });
        }
        self.houses[winner].events.push(HouseEvent { tick, kind: "feud".into(), text: text.clone() });
        self.houses[loser].events.push(HouseEvent { tick, kind: "feud".into(), text });
    }

    /// Yearly · SETTLEMENT. A council both houses trade in has every reason to end a
    /// long feud: two great families at open war in its market is the city's problem,
    /// not just theirs. This is the mechanism the old model lacked entirely, and it is
    /// what stops the world converging on "every old house feuds with every other".
    pub(crate) fn arbitrate_feuds(&mut self, yr: u32) {
        if self.feuds.is_empty() { return; }
        let tick = self.tick;
        let mut settle: Vec<(usize, usize, f32)> = Vec::new(); // feud, city, damages
        for fi in 0..self.feuds.len() {
            let f = &self.feuds[fi];
            if f.outcome != FEUD_RUNNING { continue; }
            if tick.saturating_sub(f.started_tick) < FEUD_ARBITRATE_YEARS * TICKS_PER_YEAR { continue; }
            let city = f.hub;
            if city < 0 { continue; }
            let ci = city as usize;
            // A city at war has other concerns; a captured one favours its captor
            // instead of arbitrating.
            match self.hubs.get(ci) {
                Some(h) if h.war_with < 0 && h.captor_house < 0 && !h.abandoned => {}
                _ => continue,
            }
            let roll = hash01(self.seed, yr as u64 ^ 0xA2B1, fi as u64);
            if roll >= FEUD_ARBITRATE_CHANCE { continue; }
            // The council fines the AGGRESSOR — the house that has taken less damage,
            // i.e. the one that has been winning — and pays part to the injured party.
            let (a, b) = (f.a as usize, f.b as usize);
            let aggressor = if f.damage_a <= f.damage_b { a } else { b };
            let injured = if aggressor == a { b } else { a };
            let owed = (f.damage_a.max(f.damage_b) * 0.25)
                .min(self.houses.get(aggressor).map(|h| h.wealth.max(0.0) * 0.10).unwrap_or(0.0));
            settle.push((fi, ci, owed));
            let _ = (aggressor, injured);
        }
        for (fi, ci, owed) in settle {
            let (a, b) = (self.feuds[fi].a as usize, self.feuds[fi].b as usize);
            let aggressor = if self.feuds[fi].damage_a <= self.feuds[fi].damage_b { a } else { b };
            let injured = if aggressor == a { b } else { a };
            if aggressor >= self.houses.len() || injured >= self.houses.len() { continue; }
            self.houses[aggressor].wealth -= owed;
            self.houses[injured].wealth += owed * 0.7;
            if let Some(h) = self.hubs.get_mut(ci) { h.civic_pool += owed * 0.3; }
            // Both lose a little standing: needing the council to settle your quarrel
            // is not a good look for a great house.
            for h in [a, b] {
                self.houses[h].prestige = (self.houses[h].prestige - FEUD_ARBITRATE_PRESTIGE).max(0.0);
            }
            // The peace also lifts any market closure between them at that city.
            let c = ci as u32;
            for h in [a, b] {
                if let Some(v) = self.house_barred.get_mut(h) { v.retain(|&x| x != c); }
            }
            self.close_feud(fi, FEUD_ARBITRATED);
            let (an, bn) = (self.houses[a].name.clone(), self.houses[b].name.clone());
            let cn = self.hubs.get(ci).map(|h| h.name.clone()).unwrap_or_default();
            self.journal.push(JournalEntry {
                tick, kind: "feud".into(), hub: ci as i32, good: -1, value: owed,
                text: format!("The council of {} imposes a settlement on {} and {}", cn, an, bn),
            });
            for h in [a, b] {
                self.houses[h].events.push(HouseEvent {
                    tick, kind: "feud".into(),
                    text: format!("The council of {} settled our quarrel", cn),
                });
            }
        }
    }
}
