//! polis — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

/// One seat's polis-policy CHOICE for the coming year — the levers a council (or,
/// eventually, a player who holds the seat) sets: who sits the council, the tariff
/// schedule, the mint's fineness target, and whether treasury funds public health.
/// `decide_polis_policy` computes these read-only from the AI's rules;
/// `apply_polis_policy` is the only part that mutates hub state. Splitting the two
/// (FIX_PLAN B2) is what lets a future player supply a `PolisChoice` directly in
/// place of the AI's — the sim doesn't care which one produced it.
pub(crate) struct PolisChoice {
    council_house: i32,
    tariff_export: f32,
    tariff_import: f32,
    /// Target mint fineness (0..1) the treasury eases toward this year.
    mint_target: f32,
    /// Whether treasury headroom funds public health this year (else it decays).
    fund_health: bool,
}

/// One council's CRISIS RELIEF decision for this month — the dearth counterpart to
/// `PolisChoice`, produced by `decide_crisis_relief` and consumed by
/// `apply_crisis_relief` (FIX_PLAN B2's decide/apply split, so a player holding the
/// seat can supply one directly).
pub(crate) struct ReliefChoice {
    pub hub: usize,
    /// 0 = no crisis · 1 = dearth · 2 = famine.
    pub severity: u8,
    /// (good, units) drawn from `civic_goods` into the open market.
    pub release: Vec<(usize, f32)>,
    /// Bar the export of food until this tick (0 = don't touch the lock).
    pub lock_until: u32,
    /// True when the lock is being imposed FRESH (a lapsed or absent lock) — the one
    /// condition under which the episode is chronicled, so a months-long dearth
    /// produces one beat rather than one per month.
    pub announce: bool,
}

impl CampaignSim {

    /// DLC 3 · Phase 0 — the POLIS as an actor. Once a year each seat city's
    /// council (its dominant house) sets the coming year's tariff schedule and mint
    /// policy, and skims a slice of civic taxes into a retained treasury. These
    /// levers feed both the live sim (tariffs are charged on trade) and the
    /// speculation engine (a debased mint = cheap money). Conservative + additive:
    /// hubs with no dominant house keep the global default rates.
    ///
    /// Pure AI proposal (reads `&self` only) — see `apply_polis_policy` for the
    /// mutation and `run_polis_policy` for the combined call the tick loop uses.
    pub(crate) fn decide_polis_policy(&self, _year: u32) -> Vec<PolisChoice> {
        let n = self.hubs.len();
        // Dominant council house per hub: the richest non-guild house that holds
        // its seat (`dominant_seat`) and is homed there.
        let mut council: Vec<i32> = vec![-1; n];
        let mut council_wealth: Vec<f32> = vec![0.0; n];
        for (hi, h) in self.houses.iter().enumerate() {
            if h.defunct || h.is_guild { continue; }
            let hub = h.hub as usize;
            if hub >= n { continue; }
            // A family driven from a seat by revolt is barred from it for a generation.
            let banned = |this: &Self, hub: usize| {
                let so = &this.hubs[hub].society;
                so.ousted_house == hi as i32 && this.tick < so.ousted_until
            };
            if h.dominant_seat && h.wealth > council_wealth[hub] && !banned(self, hub) {
                council[hub] = hi as i32;
                council_wealth[hub] = h.wealth;
            }
            // A BAILO is a governing headquarters: the house also sits the council of
            // each city where it has raised one (the only way besides the home seat).
            for &c in &h.bailos {
                let c = c as usize;
                if c < n && h.wealth > council_wealth[c] && !banned(self, c) {
                    council[c] = hi as i32;
                    council_wealth[c] = h.wealth;
                }
            }
        }
        let mut choices = Vec::with_capacity(n);
        for h in 0..n {
            if self.hubs[h].is_estate {
                choices.push(PolisChoice {
                    council_house: -1, tariff_export: 0.0, tariff_import: 0.0,
                    mint_target: 1.0, fund_health: false,
                });
                continue;
            }
            // A house that has CAPTURED this government (its key figures) sets the stance —
            // otherwise the council house does. Capture = policy in the captor's interest.
            let ruler = if self.hubs[h].captor_house >= 0 { self.hubs[h].captor_house } else { council[h] };
            let arch = if ruler >= 0 && (ruler as usize) < self.houses.len() {
                self.houses[ruler as usize].archetype } else { 255 };
            // Tariff stance by the ruler's character: political houses turn
            // protectionist; bankers/shippers keep trade cheap to move volume.
            let (exp, imp) = match arch {
                ARCH_POLITICAL => (EXPORT_TAX_RATE * 1.6, IMPORT_TAX_RATE * 1.6),
                ARCH_BANKING => (EXPORT_TAX_RATE * 0.8, IMPORT_TAX_RATE * 0.8),
                ARCH_FLEET => (EXPORT_TAX_RATE * 0.7, IMPORT_TAX_RATE * 0.9),
                ARCH_SPECIALTY => (EXPORT_TAX_RATE * 1.1, IMPORT_TAX_RATE * 1.1),
                _ => (EXPORT_TAX_RATE, IMPORT_TAX_RATE),
            };
            // Mint: a prosperous, banking-led council "cuts the coin fine" to lend
            // cheap (fineness eases down); others slowly restore full-bodied coin.
            let prosperous = self.hubs[h].trade_wealth > 0.5;
            // v2.0 · a post-reform HONEST-MONEY mandate bars debasement until it lapses.
            let under_mandate = self.hubs[h].reform_until > self.tick;
            let mint_target = if under_mandate { 1.0 }
                else if arch == ARCH_BANKING && prosperous { 0.88 }
                else if prosperous { 0.96 } else { 1.0 };
            // Hospices & quarantine (public health): a council with treasury headroom funds
            // public health; one that can't afford it lets the provision lapse. This is
            // the lever by which a WEALTHY city buys down its plague mortality — coin
            // spent so fewer of its people die.
            let fund_health = self.hubs[h].treasury > HOSPICE_MIN_TREASURY;
            choices.push(PolisChoice {
                council_house: council[h], tariff_export: exp, tariff_import: imp,
                mint_target, fund_health,
            });
        }
        choices
    }

    /// Carries out a year's `PolisChoice`es — the only part of polis policy that
    /// mutates hub state. See `decide_polis_policy`'s doc comment (FIX_PLAN B2).
    pub(crate) fn apply_polis_policy(&mut self, choices: &[PolisChoice]) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let c = &choices[h];
            self.hubs[h].council_house = c.council_house;
            if self.hubs[h].mint_fineness <= 0.0 { self.hubs[h].mint_fineness = 1.0; }
            self.hubs[h].tariff_export = c.tariff_export;
            self.hubs[h].tariff_import = c.tariff_import;
            let f = self.hubs[h].mint_fineness;
            self.hubs[h].mint_fineness = f + (c.mint_target - f) * 0.5;
            // Retained treasury: skim ~8% of the circulating civic pool.
            self.hubs[h].treasury += self.hubs[h].civic_pool * 0.08;
            let ph = self.hubs[h].public_health;
            if c.fund_health {
                let target = self.hubs[h].trade_wealth.clamp(0.0, 1.0) * HOSPICE_MAX_LEVEL;
                self.hubs[h].public_health = (ph + (target - ph) * HOSPICE_EASE).clamp(0.0, HOSPICE_MAX_LEVEL);
                let cost = self.hubs[h].treasury * HOSPICE_TREASURY_SKIM;
                self.hubs[h].treasury -= cost;
                self.hubs[h].finance.spent_health += cost;
            } else {
                self.hubs[h].public_health = (ph - HOSPICE_DECAY).max(0.0);
            }
        }
    }

    /// The tick loop's entry point: AI decides, sim applies. A future player-owned
    /// seat would call `apply_polis_policy` directly with its own `PolisChoice`
    /// instead of going through `decide_polis_policy` (FIX_PLAN B2).
    pub(crate) fn run_polis_policy(&mut self, year: u32) {
        let choices = self.decide_polis_policy(year);
        self.apply_polis_policy(&choices);
    }


    /// DLC 3 · Phase 3 — the Speculation "Why-Engine". Once a year, score each
    /// polis's bubble risk from drivers that ALL already exist in the sim, build a
    /// ranked causal reason-chain naming the real houses/goods, classify the
    /// pattern, and journal the high-risk poleis. Deterministic; cached on the sim.
    pub(crate) fn compute_speculation(&mut self, year: u32) {
        let n = self.hubs.len();
        let tick = self.tick;
        // This year's trade profit booked at each city (from the just-closed books).
        let mut cur_profit = vec![0.0f32; n];
        for l in &self.house_ledger_prev {
            for (c, amt) in &l.trade_profit_by_city {
                if (*c as usize) < n { cur_profit[*c as usize] += *amt; }
            }
        }
        if self.spec_prev_profit.len() != n { self.spec_prev_profit = vec![0.0; n]; }

        // Weighted blend of normalized drivers (∑ coefficients ≈ 1).
        const W_FLOAT: f32 = 0.22; const W_MONEY: f32 = 0.16; const W_LEV: f32 = 0.12;
        const W_DIV: f32 = 0.14; const W_RUN: f32 = 0.14; const W_SHOCK: f32 = 0.08;
        const W_HOT: f32 = 0.05; const W_POL: f32 = 0.04; const W_SPIRIT: f32 = 0.05;

        let mut centers: Vec<SpecCenter> = Vec::new();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].population < 1.0 { continue; }

            // ── Thin float / corner — the largest monopoly held by a house homed
            //    here (or with an office here). ──
            let mut corner = 0.0f32; let mut corner_good = -1i32; let mut corner_house = String::new();
            for hh in &self.houses {
                if hh.defunct { continue; }
                let here = hh.hub as usize == h || hh.offices.contains(&(h as u32));
                if !here { continue; }
                for (g, share) in &hh.monopoly {
                    if *share > corner { corner = *share; corner_good = *g as i32; corner_house = hh.name.clone(); }
                }
            }

            // ── Cheap money — coin debasement at this polis + banking presence. ──
            let fineness = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            let debase = (1.0 - fineness).clamp(0.0, 1.0);
            let mut bank_seats = 0u32;
            for hh in &self.houses {
                if hh.defunct || hh.archetype != ARCH_BANKING { continue; }
                if hh.hub as usize == h || hh.offices.contains(&(h as u32)) { bank_seats += 1; }
            }
            let cheap_money = (debase / 0.12 * 0.6 + (bank_seats as f32 / 3.0) * 0.4).clamp(0.0, 1.0);
            // ── Leverage — banking credit multiplier scaled by the number of seats. ──
            let leverage = ((bank_seats as f32) * (BANK_CREDIT_MULT - 1.0) / 2.0).clamp(0.0, 1.0);

            // ── Dividend surge — YoY growth of trade profit booked at this city. ──
            let prev = self.spec_prev_profit[h];
            let div_growth = if prev > 1.0 { (cur_profit[h] - prev) / prev } else { 0.0 };
            let dividend = div_growth.clamp(0.0, 1.0);

            // ── Price run-up — dearest recent price sample vs world-standard value. ──
            let mut runup = 0.0f32; let mut runup_good = -1i32;
            for e in self.journal.iter().rev() {
                if e.tick + TICKS_PER_YEAR < tick { break; }
                if e.kind != "price" || e.hub != h as i32 || e.good < 0 { continue; }
                let base = self.goods.get(e.good as usize).map(|x| x.base_value).unwrap_or(1.0).max(1e-3);
                let ratio = (e.value / base - 1.0) / 2.0; // 3× base → 1.0
                if ratio > runup { runup = ratio.clamp(0.0, 1.0); runup_good = e.good; }
            }

            // ── Supply shock — an active embargo / drought / fishery collapse. ──
            let mut shock = 0.0f32; let mut shock_kind = String::new(); let mut shock_good = -1i32;
            for ev in &self.active_events {
                if ev.hub == h as i32 || ev.hub < 0 {
                    let s = (ev.magnitude.abs()).clamp(0.0, 1.0);
                    if s > shock { shock = s; shock_kind = ev.kind.clone(); shock_good = ev.good; }
                }
            }

            // ── Hot capital — foreign offices opened here (imported speculation). ──
            let mut foreign = 0u32;
            for hh in &self.houses {
                if hh.defunct { continue; }
                if hh.hub as usize != h && hh.offices.contains(&(h as u32)) { foreign += 1; }
            }
            let hot = (foreign as f32 / 4.0).clamp(0.0, 1.0);

            // ── Political shock — recent succession / control change at this seat. ──
            let mut pol = 0.0f32;
            for hh in &self.houses {
                for ev in hh.events.iter().rev() {
                    if ev.tick + TICKS_PER_YEAR < tick { break; }
                    let relevant = matches!(ev.kind.as_str(), "succession" | "control_gained" | "control_lost");
                    if relevant && (hh.hub as usize == h) { pol = pol.max(0.7); }
                }
            }

            // ── Animal spirits — the irrational deterministic residual. ──
            let spirits = hash01(self.seed, year as u64, h as u64);

            let drivers_raw = [
                ("thin_float", "Thin float", W_FLOAT * corner,
                    if corner_good >= 0 { format!("{} corners {} ({:.0}% share)", corner_house, self.goods[corner_good as usize].name, corner * 100.0) } else { String::new() }),
                ("cheap_money", "Cheap money", W_MONEY * cheap_money,
                    if debase > 0.01 { format!("council cut the coin fine ({:.0}% debased), {} banking seat(s)", debase * 100.0, bank_seats) } else if bank_seats > 0 { format!("{} banking seat(s) lending freely", bank_seats) } else { String::new() }),
                ("leverage", "Leverage", W_LEV * leverage,
                    if bank_seats > 0 { format!("borrowed money ({:.1}× credit) chasing assets", BANK_CREDIT_MULT) } else { String::new() }),
                ("dividend_surge", "Dividend surge", W_DIV * dividend,
                    if dividend > 0.05 { format!("trade profit up {:.0}% on the year", div_growth * 100.0) } else { String::new() }),
                ("price_runup", "Price run-up", W_RUN * runup,
                    if runup_good >= 0 { format!("{} trading well above its standard value", self.goods[runup_good as usize].name) } else { String::new() }),
                ("supply_shock", "Supply shock", W_SHOCK * shock,
                    if !shock_kind.is_empty() { let g = if shock_good >= 0 { format!(" on {}", self.goods[shock_good as usize].name) } else { String::new() }; format!("a {}{} is spiking prices", shock_kind, g) } else { String::new() }),
                ("hot_capital", "Hot capital", W_HOT * hot,
                    if foreign > 0 { format!("{} foreign house office(s) pouring capital in", foreign) } else { String::new() }),
                ("political_shock", "Political shock", W_POL * pol,
                    if pol > 0.0 { "a recent succession / regime change unsettles the seat".to_string() } else { String::new() }),
                ("animal_spirits", "Animal spirits", W_SPIRIT * spirits, "the irrational froth of the crowd".to_string()),
            ];

            let risk: f32 = drivers_raw.iter().map(|d| d.2).sum::<f32>().clamp(0.0, 1.0);
            // Skip near-silent poleis to keep the overlay legible.
            if risk < 0.15 { continue; }

            let mut drivers: Vec<SpecDriver> = drivers_raw.iter()
                .filter(|d| d.2 > 0.001 && !d.3.is_empty())
                .map(|d| SpecDriver { key: d.0.into(), label: d.1.into(), weight: d.2, detail: d.3.clone() })
                .collect();
            drivers.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

            let stars = if risk >= 0.8 { 5 } else if risk >= 0.65 { 4 } else if risk >= 0.5 { 3 } else if risk >= 0.35 { 2 } else { 1 };
            let tier = if risk >= 0.6 { "HIGH" } else if risk >= 0.4 { "MED" } else { "LOW" };
            // Pattern from the dominant driver.
            let pattern_tag = match drivers.first().map(|d| d.key.as_str()) {
                Some("thin_float") => "tulip-like",
                Some("dividend_surge") | Some("leverage") => "company-bubble",
                Some("cheap_money") => "credit-fueled",
                Some("supply_shock") => "shortage-driven",
                Some("animal_spirits") => "speculative froth",
                _ => "speculative froth",
            }.to_string();

            let mut watch_goods: Vec<String> = Vec::new();
            for g in [corner_good, runup_good, shock_good] {
                if g >= 0 { let nm = self.goods[g as usize].name.clone(); if !watch_goods.contains(&nm) { watch_goods.push(nm); } }
            }

            centers.push(SpecCenter {
                hub: self.hubs[h].id, x: self.hubs[h].x, y: self.hubs[h].y,
                name: self.hubs[h].name.clone(), risk, stars, tier: tier.into(),
                pattern_tag, drivers, watch_goods, year,
            });
        }

        centers.sort_by(|a, b| b.risk.partial_cmp(&a.risk).unwrap_or(std::cmp::Ordering::Equal));

        // Journal the high-risk poleis with the generated causal narrative.
        for c in centers.iter().filter(|c| c.tier == "HIGH") {
            let why = c.drivers.iter().take(3).map(|d| d.detail.clone()).collect::<Vec<_>>().join("; ");
            let watch = if c.watch_goods.is_empty() { String::new() } else { format!(" Watch: {}.", c.watch_goods.join(", ")) };
            let text = format!("{} — speculation {} ({:.2}). {}. Pattern: {}.{}", c.name, c.tier, c.risk, why, c.pattern_tag, watch);
            self.journal.push(JournalEntry { tick, kind: "speculation".into(), hub: c.hub as i32, good: -1, value: c.risk, text });
        }

        self.spec_prev_profit = cur_profit;
        self.spec_centers = centers;
        self.spec_year = year;
    }


    /// The council's target civic reserve per needed good — scales with the city's size
    /// and how many colonies/satellites it must feed.
    pub(crate) fn council_reserve_target(&self, h: usize, deps: usize) -> f32 {
        COUNCIL_RESERVE_BASE * (1.0 + deps as f32)
            * (self.hubs[h].population / 5_000.0).clamp(0.3, 4.0)
    }


    // ── CRISIS PRICE REGULATION ────────────────────────────────────────────────
    // The council's response to a DEARTH, on the same decide/apply split as
    // `decide_polis_policy` (FIX_PLAN B2), so a player holding the seat can supply
    // a `ReliefChoice` in place of the AI's without the sim knowing the difference.
    //
    // Two of the four levers `docs/TRADE_AND_MARKET_REVIEW.md` §10 lists are built
    // here — releasing the civic store, and barring the export of food (the *tratta*
    // prohibition). The import bounty is not, and the price ceiling deliberately is
    // not: a ceiling's entire historical consequence is that it CAUSES shortage, and
    // with demand still price-inelastic (F5) the shortage is already unconditional,
    // so a ceiling would move a number on screen and nothing else.
    //
    // A NARROWER RELEASE ALREADY EXISTED and is deliberately left alone:
    // `update_government`'s step 6 dumps half the civic store of the FIRST food good
    // once `starving > 0.5`. That is a famine backstop — it fires when people are
    // already dying, on one good. This pass is the POLICY layer above it: it triggers
    // on the dearth (unmet basic demand, a negative food balance) rather than on
    // deaths, and it covers EVERY food good the council actually holds. The two
    // compose rather than duplicate — the backstop simply finds less left to dump.

    /// Pure AI proposal — reads `&self` only.
    pub(crate) fn decide_crisis_relief(&self) -> Vec<ReliefChoice> {
        let ng = self.goods.len();
        let tick = self.tick;
        let mut out = Vec::new();
        if ng == 0 || self.suppress_relief { return out; }
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || hub.population < 1.0 { continue; }
            if hub.civic_goods.len() < ng { continue; }
            // FAMINE is the harder of the two signals; DEARTH is the earlier one.
            let famine = hub.starving > RELIEF_STARVE_TRIGGER;
            let dearth = famine
                || hub.lack_basic > RELIEF_LACK_TRIGGER
                || hub.food_balance < RELIEF_BALANCE_TRIGGER;
            if !dearth { continue; }
            let frac = if famine { RELIEF_RELEASE_FAMINE } else { RELIEF_RELEASE_DEARTH };
            let mut release = Vec::new();
            for g in 0..ng {
                if !self.goods[g].food { continue; }
                let held = hub.civic_goods[g];
                if !(held > 0.0) { continue; }
                let amt = held * frac;
                if amt >= RELIEF_MIN_RELEASE { release.push((g, amt)); }
            }
            // A council with an empty granary and no famine has nothing to say.
            if release.is_empty() && !famine { continue; }
            // The export bar is the FAMINE lever: releasing grain into a market that
            // ships it straight back out again is futile, which is exactly why the
            // prohibition and the release are historically the same policy.
            let (lock_until, announce) = if famine {
                (tick + RELIEF_EXPORT_LOCK_TICKS, hub.food_export_lock <= tick)
            } else {
                (0, false)
            };
            out.push(ReliefChoice {
                hub: h,
                severity: if famine { 2 } else { 1 },
                release,
                lock_until,
                announce,
            });
        }
        out
    }

    /// The only part that mutates.
    pub(crate) fn apply_crisis_relief(&mut self, choices: Vec<ReliefChoice>) {
        let tick = self.tick;
        for c in choices {
            let h = c.hub;
            for (g, amt) in c.release {
                if g >= self.hubs[h].civic_goods.len() { continue; }
                let take = amt.min(self.hubs[h].civic_goods[g]).max(0.0);
                if take <= 0.0 { continue; }
                self.hubs[h].civic_goods[g] -= take;
                // Into the OPEN market, ungraded — the same convention every other
                // civic release uses. Price relief follows from `live_price` reading
                // a larger stock on the next tick; nothing sets a price directly.
                stock_add_ungraded(&mut self.hubs[h].stock, g, take);
            }
            if c.lock_until > 0 { self.hubs[h].food_export_lock = c.lock_until; }
            if c.announce {
                let city = self.hubs[h].name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "relief".into(), hub: h as i32, good: -1, value: 0.0,
                    text: format!(
                        "Dearth at {}: the council opens the public granary and forbids the export of grain.",
                        city),
                });
            }
            let _ = c.severity;
        }
    }

    /// Decide + apply, the entry point the monthly block calls.
    pub(crate) fn run_crisis_relief(&mut self) {
        let choices = self.decide_crisis_relief();
        self.apply_crisis_relief(choices);
    }
}
