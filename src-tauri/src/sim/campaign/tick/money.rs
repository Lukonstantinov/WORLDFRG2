//! money — CampaignSim methods split from the former monolithic tick.rs.
//! `use super::*` brings the struct, its fields, tuning consts and free helpers into scope.
use super::*;

/// One hub's coinage CHOICE for the year — the final field values a mint's
/// council (or, eventually, a player who holds the charter) settles on, plus
/// the journal entries that narrate how it got there. `decide_coinage` computes
/// this read-only, replaying the same per-hub arithmetic the old monolithic
/// function used but against LOCAL shadow copies instead of `self` (several of
/// the terms below — trust, seigniorage — depend on a mutation earlier in the
/// SAME hub's SAME year, e.g. a charter debit changing the treasury that year's
/// trust target reads, so decide has to carry that shadow state through rather
/// than mutate and re-read `self`). `apply_coinage` then just writes the final
/// values — no branching left to do (FIX_PLAN B2).
pub(crate) struct CoinageChoice {
    has_mint: bool,
    coin_name: String,
    coin_trust: f32,
    coin_metal: u8,
    mint_bullion_ratio: f32,
    mint_fineness: f32,
    mint_fineness_prev: f32,
    treasury: f32,
    seigniorage_booked: f32,
    journal: Vec<JournalEntry>,
}

impl CampaignSim {

    /// Deterministic coin denomination for a polis (Venice → ducat, Florence →
    /// florin, …). Stable per seed/hub.
    pub(crate) fn coin_denomination(&self, hub: usize) -> &'static str {
        const DENOMS: [&str; 10] = [
            "Ducat", "Florin", "Mark", "Dinar", "Solidus",
            "Crown", "Sequin", "Thaler", "Bezant", "Stater",
        ];
        let i = (hash01(self.seed, hub as u64 ^ 0xC0114, 0xDEED) * DENOMS.len() as f32) as usize
            % DENOMS.len();
        DENOMS[i]
    }


    /// v2.0 · the monetary METAL a mint strikes, from the bullion its trade region
    /// can reach: gold if a gold province lies in the region, silver if silver hills,
    /// electrum where both are plentiful and balanced, bronze/billon where only base
    /// metal (copper/tin) is available (0 silver · 1 gold · 2 electrum · 3 bronze).
    pub(crate) fn coin_metal_for(&self, hub: usize, gi: Option<usize>, si: Option<usize>,
                      ci: Option<usize>, ti: Option<usize>) -> u8 {
        let region = self.hubs[hub].component;
        let (mut g, mut s, mut base) = (0.0f32, 0.0f32, 0.0f32);
        for h in &self.hubs {
            if h.is_estate || h.component != region { continue; }
            if let Some(i) = gi { g += h.production.get(i).copied().unwrap_or(0.0); }
            if let Some(i) = si { s += h.production.get(i).copied().unwrap_or(0.0); }
            if let Some(i) = ci { base += h.production.get(i).copied().unwrap_or(0.0); }
            if let Some(i) = ti { base += h.production.get(i).copied().unwrap_or(0.0); }
        }
        let (has_g, has_s) = (g > EPS, s > EPS);
        if has_g && has_s {
            // Both metals in reach → a balanced supply is struck as electrum, else
            // the region coins its dominant precious metal.
            if g.min(s) > 0.25 * g.max(s) { 2 } else if g >= s { 1 } else { 0 }
        } else if has_g { 1 }
        else if has_s { 0 }
        else if base > EPS { 3 }
        else { 0 } // no precious metal in the region → assume imported silver specie
    }


    /// v2.0 · the ceiling bullion supply puts on a mint's fineness. A region flush
    /// with gold/silver relative to its coin demand can strike full-bodied money
    /// (cap → 1.0); a bullion-poor region minting beyond its metal is forced to
    /// debase (cap → `MINT_FINENESS_FLOOR`). Returns the cap in [floor, 1.0] plus
    /// the raw capacity ratio (for the panel's "limiting factor" read).
    pub(crate) fn mint_bullion_cap(&self, hub: usize, gi: Option<usize>, si: Option<usize>) -> (f32, f32) {
        let region = self.hubs[hub].component;
        let (mut bull, mut demand) = (0.0f32, 0.0f32);
        for h in &self.hubs {
            if h.is_estate || h.component != region { continue; }
            if let Some(i) = gi { bull += h.production.get(i).copied().unwrap_or(0.0) * MINT_GOLD_WEIGHT; }
            if let Some(i) = si { bull += h.production.get(i).copied().unwrap_or(0.0); }
            demand += h.tw_house + h.tw_local + h.tw_guild;
        }
        let cr = if demand > EPS { bull / (demand * MINT_BULLION_DEMAND) } else { 1.0 };
        let cap = (MINT_FINENESS_FLOOR + (1.0 - MINT_FINENESS_FLOOR) * cr.clamp(0.0, 1.0)).clamp(MINT_FINENESS_FLOOR, 1.0);
        (cap, cr)
    }


    /// DLC 3.5 · Coinage — once a year each council seat mints a NAMED coin and
    /// updates its acceptance ("trust"): sticky reputation built from full-bodied
    /// minting, a deep treasury, trade wealth, civic stability and throughput, and
    /// docked when the council debases. Debasement also skims seigniorage into the
    /// treasury (mint profit now, at trust's expense later). The strongest coins
    /// become reserve currencies (see `coin_discount`).
    ///
    /// Pure AI proposal (reads `&self` only) — see `apply_coinage` for the
    /// mutation and `run_coinage` for the combined call the tick loop uses.
    pub(crate) fn decide_coinage(&self, _year: u32) -> Vec<CoinageChoice> {
        let n = self.hubs.len();
        // Normalizers across all council seats.
        let mut max_treasury = 1.0f32;
        let mut max_through = 1.0f32;
        let mut max_pop = 1.0f32;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            max_treasury = max_treasury.max(self.hubs[h].treasury);
            max_through = max_through.max(self.hub_throughput(h));
            max_pop = max_pop.max(self.hubs[h].population);
        }
        // Good indices for the monetary metals (computed once) → per-mint metal.
        let gi = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("gold"));
        let si = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("silver"));
        let cui = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("copper"));
        let tii = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("tin"));
        let tick = self.tick;
        let mut out = Vec::with_capacity(n);
        for h in 0..n {
            if self.hubs[h].is_estate {
                out.push(CoinageChoice {
                    has_mint: self.hubs[h].has_mint, coin_name: self.hubs[h].coin_name.clone(),
                    coin_trust: self.hubs[h].coin_trust, coin_metal: self.hubs[h].coin_metal,
                    mint_bullion_ratio: self.hubs[h].mint_bullion_ratio,
                    mint_fineness: self.hubs[h].mint_fineness,
                    mint_fineness_prev: self.hubs[h].mint_fineness_prev,
                    treasury: self.hubs[h].treasury, seigniorage_booked: 0.0, journal: Vec::new(),
                });
                continue;
            }
            let fineness = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            let council = self.hubs[h].council_house;
            // Local shadow state — mutated below exactly as the old function
            // mutated `self.hubs[h]`, so later terms in this SAME hub/year see
            // the same values the original sequential mutation produced.
            let mut has_mint = self.hubs[h].has_mint;
            let mut coin_name = self.hubs[h].coin_name.clone();
            let mut coin_trust = self.hubs[h].coin_trust;
            let mut treasury = self.hubs[h].treasury;
            let mut mint_fineness = self.hubs[h].mint_fineness;
            let mut seigniorage_booked = 0.0f32;
            let mut journal = Vec::new();
            // v2.0 · MINT CHARTER — minting is a privilege. Grandfather any city that
            // already struck a coin; otherwise a council seat earns the right of the
            // mint only once it is a substantial commercial centre (busy AND large)
            // and can pay to establish the mint-house.
            if !has_mint {
                if !coin_name.is_empty() {
                    has_mint = true; // pre-existing coin keeps its mint
                } else if council >= 0 {
                    let busy = self.hub_throughput(h) >= MINT_CHARTER_THROUGH_FRAC * max_through;
                    let big = self.hubs[h].population >= MINT_CHARTER_POP_FRAC * max_pop;
                    if busy && big && treasury >= MINT_CHARTER_COST {
                        treasury -= MINT_CHARTER_COST;
                        has_mint = true;
                        let city = self.hubs[h].name.clone();
                        journal.push(JournalEntry {
                            tick, kind: "charter".into(), hub: h as i32, good: -1, value: MINT_CHARTER_COST,
                            text: format!("{} is granted the right of the mint and establishes a mint-house", city),
                        });
                    }
                }
            }
            // A chartered council seat with no coin yet strikes its first.
            if council >= 0 && has_mint && coin_name.is_empty() {
                let denom = self.coin_denomination(h);
                let city = self.hubs[h].name.clone();
                coin_name = format!("{} of {}", denom, city);
                coin_trust = 0.35;
                journal.push(JournalEntry {
                    tick, kind: "coinage".into(), hub: h as i32, good: -1, value: 0.0,
                    text: format!("{} mints the {}", city, coin_name),
                });
            }
            if coin_name.is_empty() {
                // No mint → any residual trust slowly bleeds away.
                out.push(CoinageChoice {
                    has_mint, coin_name, coin_trust: coin_trust * 0.9,
                    coin_metal: self.hubs[h].coin_metal, mint_bullion_ratio: self.hubs[h].mint_bullion_ratio,
                    mint_fineness, mint_fineness_prev: fineness, treasury, seigniorage_booked, journal,
                });
                continue;
            }
            // v2.0 · pick the metal from the region's reachable bullion.
            let coin_metal = self.coin_metal_for(h, gi, si, cui, tii);
            // v2.0 · MINT REGULATION — regional bullion caps how full-bodied the coin
            // can be. A bullion-poor mint striking beyond its metal is forced to debase
            // (fineness capped down); an ample region can strike full-bodied coin.
            let (bcap, bratio) = self.mint_bullion_cap(h, gi, si);
            if mint_fineness > bcap { mint_fineness = bcap; }
            let fineness = if mint_fineness <= 0.0 { 1.0 } else { mint_fineness };
            // Trust target — each term in 0..1.
            let through = self.hub_throughput(h);
            let t_fine = fineness.clamp(0.0, 1.0);
            let t_treas = (treasury / max_treasury).clamp(0.0, 1.0);
            let tw = self.hubs[h].trade_wealth;
            let t_trade = (tw / (tw.abs() + 1.0)).clamp(0.0, 1.0);
            let t_stab = self.hubs[h].sent_stability.clamp(0.0, 1.0);
            let t_through = (through / max_through).clamp(0.0, 1.0);
            let mut target =
                0.34 * t_fine + 0.20 * t_treas + 0.16 * t_trade + 0.14 * t_stab + 0.16 * t_through;
            // A fresh debasement (a cut vs last year) spooks holders extra.
            let prev = if self.hubs[h].mint_fineness_prev <= 0.0 { fineness } else { self.hubs[h].mint_fineness_prev };
            let debase = (prev - fineness).max(0.0);
            target = (target - debase * COIN_DEBASE_PENALTY).clamp(0.0, 1.0);
            coin_trust = (coin_trust + (target - coin_trust) * COIN_TRUST_EASE).clamp(0.0, 1.0);
            // Seigniorage from minting (esp. from debasement), scaled by throughput.
            let seign = through * (1.0 - fineness).max(0.0) * COIN_SEIGNIORAGE;
            treasury += seign;
            seigniorage_booked += seign;
            out.push(CoinageChoice {
                has_mint, coin_name, coin_trust, coin_metal, mint_bullion_ratio: bratio,
                mint_fineness, mint_fineness_prev: fineness, treasury, seigniorage_booked, journal,
            });
        }
        out
    }

    /// Carries out a year's `CoinageChoice`s — the only part of coinage that
    /// mutates hub state. See `decide_coinage`'s doc comment (FIX_PLAN B2).
    pub(crate) fn apply_coinage(&mut self, choices: Vec<CoinageChoice>) {
        for (h, c) in choices.into_iter().enumerate() {
            self.hubs[h].has_mint = c.has_mint;
            self.hubs[h].coin_name = c.coin_name;
            self.hubs[h].coin_trust = c.coin_trust;
            self.hubs[h].coin_metal = c.coin_metal;
            self.hubs[h].mint_bullion_ratio = c.mint_bullion_ratio;
            self.hubs[h].mint_fineness = c.mint_fineness;
            self.hubs[h].mint_fineness_prev = c.mint_fineness_prev;
            self.hubs[h].treasury = c.treasury;
            self.hubs[h].finance.seigniorage += c.seigniorage_booked;
            self.journal.extend(c.journal);
        }
    }

    /// The tick loop's entry point: AI decides, sim applies. A future player-owned
    /// mint would call `apply_coinage` directly with its own `CoinageChoice`
    /// instead of going through `decide_coinage` (FIX_PLAN B2).
    pub(crate) fn run_coinage(&mut self, year: u32) {
        let choices = self.decide_coinage(year);
        self.apply_coinage(choices);
    }


    /// Import-freight multiplier (≤ 1.0) a destination's money earns: a trusted
    /// reserve coin — or a bank-note from a branch of a strong-coin bank — shaves
    /// transaction cost, making strong-money cities natural entrepôts.
    pub(crate) fn coin_discount(&self, dest: usize) -> f32 {
        if dest >= self.hubs.len() { return 1.0; }
        let mut trust = self.hubs[dest].coin_trust;
        if trust < RESERVE_TRUST_MIN {
            // Bank notes: a branch of a bank seated in a strong-coin city brings
            // that coin's credit here.
            for b in &self.banks {
                if b.defunct { continue; }
                if b.seat as usize == dest || b.branches.contains(&(dest as u32)) {
                    let st = self.hubs.get(b.seat as usize).map(|x| x.coin_trust).unwrap_or(0.0);
                    if st > trust { trust = st; }
                }
            }
        }
        if trust < RESERVE_TRUST_MIN { return 1.0; }
        1.0 - COIN_FREIGHT_DISCOUNT * trust.clamp(0.0, 1.0)
    }


    /// "Banco di <City>" name for a bank seated at `seat`.
    pub(crate) fn bank_name_for(&self, seat: usize) -> String {
        format!("Banco di {}", self.hubs[seat].name)
    }


    /// DLC 3.5 · once a year: qualifying banking houses charter banks, and existing
    /// banks open counting-house branches wherever their owner has trade offices
    /// (extending the home coin's reach and booking real estate).
    /// DLC 3.5 · once a year, update every city's CURRENCY BASKET: a small set of
    /// coins it holds (own/main + foreign coins that arrive with trade). Shares ease
    /// toward an adoption target (coin value × trust × issuer weight, with a home-mint
    /// bias); the main coin flips only when a rival durably dominates. The issuing
    /// polis earns a little seigniorage when its coin circulates abroad. Drives the
    /// coin-usage overlay + per-coin breakdown + circulating amount.
    pub(crate) fn update_currency_baskets(&mut self) {
        let n = self.hubs.len();
        // 1) Per-hub trade-partner volumes (by hub INDEX) from last year's realized
        //    flows. A coin can ONLY spread to a city that actually TRADES with a
        //    coin-holder — so coins travel along merchant routes, not teleport across
        //    the world. (Previously any reserve coin reached every hub.)
        let mut partner_vol: Vec<std::collections::HashMap<usize, f32>> =
            vec![std::collections::HashMap::new(); n];
        for f in &self.trade_last {
            let (a, b) = (f.hub as usize, f.partner as usize);
            if a < n && b < n && a != b { *partner_vol[a].entry(b).or_insert(0.0) += f.amount; }
        }
        // v2.1 · attractiveness is trust×value with a MODEST metal reserve-preference:
        // high-value-density gold (and electrum) coins are the natural international
        // reserve money (florin/ducat were gold), so they spread & hold a little better;
        // base-metal billon is shunned as a store of value. The FULL bimetallic ratio
        // lives in the exchange value, not here — so a silver-minting city keeps its own
        // silver as daily money (home bias) while gold coins accrue as its RESERVES.
        let attr = |h: &TickHub| {
            let metal_pref = match h.coin_metal {
                1 => COIN_METAL_GOLD_PREF, 2 => COIN_METAL_ELECTRUM_PREF,
                3 => COIN_METAL_BRONZE_PREF, _ => 1.0,
            };
            (coin_value(h.mint_fineness, h.coin_trust) * h.coin_trust * metal_pref).max(EPS)
        };
        let mints = |h: &TickHub| !h.coin_name.is_empty() && h.coin_trust >= BANK_FOUND_COIN_TRUST;
        let mut new_baskets: Vec<Vec<(u32, f32)>> = vec![Vec::new(); n];
        let mut new_main: Vec<i32> = vec![-1; n];
        for i in 0..n {
            if self.hubs[i].is_estate { continue; }
            // Adoption target: the city's OWN coin (home bias) + coins that ARRIVE via
            // its trade partners — each partner's basket coins, weighted by the trade
            // volume with that partner × the coin's share in the partner × its
            // attractiveness. No trade link to a coin-holder ⇒ that coin never arrives.
            let mut wmap: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
            if mints(&self.hubs[i]) {
                *wmap.entry(i as u32).or_insert(0.0) += COIN_HOME_BIAS * attr(&self.hubs[i]);
            }
            let totv: f32 = partner_vol[i].values().sum::<f32>().max(EPS);
            for (&p, &v) in &partner_vol[i] {
                let frac = v / totv;
                for &(c, share) in &self.hubs[p].coin_basket {
                    let cj = c as usize;
                    if cj < n && !self.hubs[cj].coin_name.is_empty() {
                        *wmap.entry(c).or_insert(0.0) += frac * share * attr(&self.hubs[cj]);
                    }
                }
                if mints(&self.hubs[p]) {
                    *wmap.entry(p as u32).or_insert(0.0) += frac * 0.5 * attr(&self.hubs[p]);
                }
            }
            if wmap.is_empty() { continue; } // barter — no coin physically reaches here
            let mut tw: Vec<(u32, f32)> = wmap.into_iter().collect();
            tw.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            tw.truncate(COIN_BASKET_N);
            let sum: f32 = tw.iter().map(|x| x.1).sum::<f32>().max(EPS);
            // Ease last year's basket toward the target (sticky adoption).
            let prev = &self.hubs[i].coin_basket;
            let mut keys: Vec<u32> = tw.iter().map(|x| x.0).collect();
            for &(k, _) in prev { if !keys.contains(&k) { keys.push(k); } }
            let mut eased: Vec<(u32, f32)> = Vec::new();
            for k in keys {
                let pv = prev.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or(0.0);
                let tv = tw.iter().find(|x| x.0 == k).map(|x| x.1 / sum).unwrap_or(0.0);
                let nv = pv + COIN_ADOPT_EASE * (tv - pv);
                if nv > 0.02 { eased.push((k, nv)); }
            }
            eased.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            eased.truncate(COIN_BASKET_N);
            let s2: f32 = eased.iter().map(|x| x.1).sum::<f32>().max(EPS);
            for e in eased.iter_mut() { e.1 /= s2; }
            // Main coin: keep the current one unless a rival leads it by the flip margin.
            let cur = self.hubs[i].settle_coin;
            let leader = eased.first().map(|x| x.0 as i32).unwrap_or(-1);
            let main = if cur >= 0 && eased.iter().any(|x| x.0 as i32 == cur) {
                let cur_s = eased.iter().find(|x| x.0 as i32 == cur).map(|x| x.1).unwrap_or(0.0);
                let lead_s = eased.first().map(|x| x.1).unwrap_or(0.0);
                if leader != cur && lead_s > cur_s * COIN_FLIP_MARGIN { leader } else { cur }
            } else { leader };
            new_baskets[i] = eased;
            new_main[i] = main;
        }
        for i in 0..n {
            if self.hubs[i].is_estate { self.hubs[i].settle_coin = -1; self.hubs[i].coin_basket.clear(); continue; }
            self.hubs[i].coin_basket = std::mem::take(&mut new_baskets[i]);
            self.hubs[i].settle_coin = new_main[i];
        }
        // 2) Seigniorage: a coin circulating ABROAD earns its issuing polis a little
        //    treasury income + a tiny prestige bump to the council house (routed to
        //    TREASURY, not house wealth, so the wealth bound is unaffected).
        let mut circ_abroad: Vec<f32> = vec![0.0; n];
        for i in 0..n {
            let thru = self.hubs[i].tw_house + self.hubs[i].tw_local + self.hubs[i].tw_guild;
            for &(k, s) in &self.hubs[i].coin_basket {
                let j = k as usize;
                if j != i && j < n { circ_abroad[j] += thru * s; }
            }
        }
        for j in 0..n {
            if circ_abroad[j] <= 0.0 { continue; }
            self.hubs[j].treasury += circ_abroad[j] * COIN_CIRCULATION_SEIGNIORAGE;
            let ch = self.hubs[j].council_house;
            if ch >= 0 && (ch as usize) < self.houses.len() {
                self.houses[ch as usize].prestige =
                    (self.houses[ch as usize].prestige + 0.001 * circ_abroad[j].min(50.0)).min(2.0);
            }
        }
    }


    /// v2.0 · close the monetary loop. Each year, turn every coin's DEBASEMENT and
    /// MONEY-SUPPLY growth into a real inflation rate (quantity-theory-lite:
    /// π ≈ debasement + money growth − real output growth), compound each city's
    /// `price_level` by the inflation of the coin it settles in, and RETURN the
    /// per-hub inflation rate so the caller can levy the matching inflation-tax on
    /// resident fortunes. A debased-coin city now visibly gets dearer AND erodes its
    /// hoards faster — the seigniorage the mint skimmed is paid back as an inflation
    /// tax on the people who hold the coin.
    pub(crate) fn update_price_levels(&mut self) -> Vec<f32> {
        let n = self.hubs.len();
        // 1) Money supply per issuing coin (hub INDEX) = Σ holders' throughput×share.
        let mut m: Vec<f32> = vec![0.0; n];
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let thru = self.hubs[h].tw_house + self.hubs[h].tw_local + self.hubs[h].tw_guild;
            for &(k, share) in &self.hubs[h].coin_basket {
                if (k as usize) < n { m[k as usize] += thru * share; }
            }
        }
        // 2) Per-coin inflation from debasement + money growth − real growth.
        let mut coin_infl: Vec<f32> = vec![0.0; n];
        for c in 0..n {
            if self.hubs[c].coin_name.is_empty() { self.hubs[c].coin_circ_prev = m[c]; continue; }
            let fine = if self.hubs[c].mint_fineness <= 0.0 { 1.0 } else { self.hubs[c].mint_fineness };
            let mprev = if self.hubs[c].coin_circ_prev <= EPS { m[c].max(EPS) } else { self.hubs[c].coin_circ_prev };
            let mgrow = ((m[c] - mprev) / mprev).clamp(-0.5, 0.5);
            coin_infl[c] = (INFL_BASE + INFL_DEBASE_K * (1.0 - fine).max(0.0) + INFL_MONEY_K * mgrow - INFL_REAL_GROWTH)
                .clamp(INFL_MIN, INFL_MAX);
            self.hubs[c].coin_circ_prev = m[c];
        }
        // 3) Per-hub local inflation = its settle-coin's inflation (barter → base),
        //    compounded into the price level (bounded so a long campaign stays finite).
        let mut hub_infl: Vec<f32> = vec![INFL_BASE; n];
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let sc = self.hubs[h].settle_coin;
            let infl = if sc >= 0 && (sc as usize) < n && !self.hubs[sc as usize].coin_name.is_empty() {
                coin_infl[sc as usize]
            } else { INFL_BASE };
            if self.hubs[h].price_level <= 0.0 { self.hubs[h].price_level = 1.0; }
            self.hubs[h].price_level = (self.hubs[h].price_level * (1.0 + infl)).clamp(0.1, 1000.0);
            hub_infl[h] = infl;
        }
        hub_infl
    }


    /// v2.0 · recoinage / reform. A council whose coin has slipped — fineness below
    /// `REFORM_FINENESS_FLOOR` AND trust below `REFORM_TRUST_FLOOR` — CALLS IN the
    /// debased coin and re-mints at full fineness, if its treasury can bear the cost
    /// and it hasn't reformed within the cooldown. Confidence partly recovers at
    /// once, the price level eases, and an HONEST-MONEY mandate then bars further
    /// debasement for a few years (after which cheap-money pressure can creep back).
    pub(crate) fn maybe_reform_coinage(&mut self, _year: u32) {
        let tick = self.tick;
        let year = self.year();
        let n = self.hubs.len();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].coin_name.is_empty() { continue; }
            let fine = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            if fine >= REFORM_FINENESS_FLOOR || self.hubs[h].coin_trust >= REFORM_TRUST_FLOOR { continue; }
            if self.hubs[h].last_reform_tick != 0
                && tick < self.hubs[h].last_reform_tick + REFORM_COOLDOWN_YEARS * TICKS_PER_YEAR { continue; }
            let cost = self.hub_throughput(h) * REFORM_COST_FRAC;
            if self.hubs[h].treasury < cost { continue; }
            self.hubs[h].treasury -= cost;
            self.hubs[h].mint_fineness = 1.0;
            self.hubs[h].mint_fineness_prev = 1.0; // the re-mint is not read as a debasement
            self.hubs[h].coin_trust = (self.hubs[h].coin_trust + REFORM_TRUST_BUMP).clamp(0.0, 1.0);
            self.hubs[h].price_level = (self.hubs[h].price_level * 0.92).max(0.5);
            self.hubs[h].last_reform_tick = tick;
            self.hubs[h].reform_until = tick + REFORM_MANDATE_YEARS * TICKS_PER_YEAR;
            let cn = self.hubs[h].coin_name.clone();
            let city = self.hubs[h].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "reform".into(), hub: h as i32, good: -1, value: cost,
                text: format!("{} reforms the {} — the debased coin is called in and re-struck at full fineness", city, cn),
            });
            let _ = year;
        }
    }


    pub(crate) fn update_banks(&mut self, _year: u32) {
        let tick = self.tick;
        // 1) Charter new banks — only once the age of banking has opened (year 20).
        //    (Existing banks still service loans/deposits below before this gate.)
        if tick >= BANK_START_TICK {
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            if self.banks.iter().any(|b| !b.defunct && b.house == hi as u32) { continue; }
            let seat = self.houses[hi].hub as usize;
            if seat >= self.hubs.len() { continue; }
            if self.houses[hi].wealth < BANK_FOUND_WEALTH { continue; }
            if self.houses[hi].prestige < BANK_FOUND_PRESTIGE { continue; }
            if self.hubs[seat].coin_trust < BANK_FOUND_COIN_TRUST { continue; }
            // The house pays the founding PRICE (50k). Of that, 40k is paid in as the
            // bank's starting specie reserves / liquidity (its treasury); the rest
            // (10k) is the establishment / charter cost paid to the seat city.
            let price = BANK_FOUND_PRICE;
            let capital = BANK_FOUND_RESERVE;
            self.houses[hi].wealth -= price;
            let charter_fee = price - capital; // establishment cost → seat city treasury
            self.hubs[seat].treasury += charter_fee;
            let bname = self.bank_name_for(seat);
            let mut bank = Bank {
                name: bname.clone(), house: hi as u32, seat: seat as u32,
                founded_tick: tick, defunct: false,
                reserves: capital, loans: Vec::new(), real_estate: BANK_BRANCH_VALUE,
                deposits: 0.0, notes_issued: 0.0,
                branches: vec![seat as u32], prestige: self.houses[hi].prestige,
                interest_earned: 0.0, losses: 0.0,
                stakes: Vec::new(), dividends_earned: 0.0, bills_income: 0.0, history: Vec::new(),
                events: Vec::new(),
            };
            bank.events.push(HouseEvent { tick, kind: "founded".into(),
                text: format!("{} chartered in {} with {:.0} in specie (founding price {:.0})",
                    bname, self.hubs[seat].name, capital, price) });
            let house_name = self.houses[hi].name.clone();
            self.journal.push(JournalEntry { tick, kind: "bank".into(), hub: seat as i32, good: -1,
                value: 0.0, text: format!("{} founds the {}", house_name, bname) });
            self.houses[hi].events.push(HouseEvent { tick, kind: "bank".into(),
                text: format!("charters the {}", bname) });
            self.banks.push(bank);
        }
        } // end year-20 charter gate
        // 2) Grow branches to follow the owner's trade offices.
        for bi in 0..self.banks.len() {
            if self.banks[bi].defunct { continue; }
            let owner = self.banks[bi].house as usize;
            if owner >= self.houses.len() { continue; }
            let offices = self.houses[owner].offices.clone();
            for off in offices {
                if (off as usize) >= self.hubs.len() { continue; }
                if !self.banks[bi].branches.contains(&off) {
                    self.banks[bi].branches.push(off);
                    self.banks[bi].real_estate += BANK_BRANCH_VALUE;
                    let cn = self.hubs[off as usize].name.clone();
                    self.banks[bi].events.push(HouseEvent { tick, kind: "branch".into(),
                        text: format!("opens a counting-house in {}", cn) });
                }
            }
        }
        // 3) Snapshot each live bank's balance sheet for the Bank panel history charts.
        let year = self.year();
        for b in self.banks.iter_mut() {
            if b.defunct { continue; }
            b.history.push(BankSnapshot {
                year,
                reserves: b.reserves, loans: b.loans_outstanding(), stakes: b.stake_book(),
                real_estate: b.real_estate, deposits: b.deposits, notes: b.notes_issued,
                equity: b.equity(),
                interest_cum: b.interest_earned, dividends_cum: b.dividends_earned, losses_cum: b.losses,
            });
            if b.history.len() > BANK_HISTORY_CAP {
                let drop = b.history.len() - BANK_HISTORY_CAP; b.history.drain(0..drop);
            }
        }
    }


    /// DLC 3.5 · monthly bank dynamics: service loans (borrowers pay interest +
    /// amortize, or default), pay depositors, dividend the owner, take new deposits,
    /// originate loans (frozen during a panic), and fail when equity turns negative.
    pub(crate) fn bank_pass(&mut self) {
        let tick = self.tick;
        for bi in 0..self.banks.len() {
            if self.banks[bi].defunct { continue; }
            let seat = self.banks[bi].seat as usize;
            let panicked = self.hub_in_panic(seat);
            // 1) Service loans.
            let nloans = self.banks[bi].loans.len();
            let mut interest_income = 0.0f32;
            let mut writeoff = 0.0f32;
            let mut principal_repaid = 0.0f32; // note-funded credit returned → retire notes
            let mut cash_repaid = 0.0f32;      // specie returned on a cash-funded loan
            let mut keep = vec![true; nloans];
            for li in 0..nloans {
                let (bh, bp, outstanding, principal, rate, term, cash_funded) = {
                    let l = &self.banks[bi].loans[li];
                    // Colony ventures are staked with hard specie OUT of reserves; every
                    // other loan is funded by ISSUING NOTES (credit creation).
                    (l.borrower_house, l.borrower_polis, l.outstanding, l.principal, l.rate,
                     l.term_ticks, l.purpose == "colony")
                };
                if outstanding <= EPS { keep[li] = false; continue; }
                let due = outstanding * rate;
                let amort = (principal / (term.max(30) as f32 / 30.0)).min(outstanding);
                let pay = due + amort;
                let paid = if bh >= 0 && (bh as usize) < self.houses.len() && !self.houses[bh as usize].defunct {
                    if self.houses[bh as usize].wealth > pay * 1.2 {
                        self.houses[bh as usize].wealth -= pay; true
                    } else { false }
                } else if bp >= 0 && (bp as usize) < self.hubs.len() {
                    if self.hubs[bp as usize].treasury > pay * 1.2 {
                        self.hubs[bp as usize].treasury -= pay; true
                    } else { false }
                } else { false };
                if paid {
                    interest_income += due;
                    // The borrower returns principal. A note-funded loan RETIRES the
                    // notes it created (liability ↓); a cash loan returns specie to
                    // reserves (asset ↑). Either way equity is conserved and only the
                    // INTEREST is profit. (Previously the principal repayment simply
                    // vanished — neither booked to reserves nor used to retire notes —
                    // so equity bled ~`amort` per loan per month and EVERY bank went
                    // insolvent within a few years, then its failure cascaded a crash.)
                    if cash_funded { cash_repaid += amort; } else { principal_repaid += amort; }
                    let rem = (outstanding - amort).max(0.0);
                    self.banks[bi].loans[li].outstanding = rem;
                    if rem <= EPS { keep[li] = false; }
                } else {
                    // Default: write off the balance; the bank seizes property worth a
                    // fraction of the loan (a foreclosed asset on its books).
                    writeoff += outstanding;
                    self.banks[bi].real_estate += outstanding * 0.4;
                    self.banks[bi].loans[li].outstanding = 0.0;
                    keep[li] = false;
                    self.banks[bi].events.push(HouseEvent { tick, kind: "default".into(),
                        text: format!("writes off a loan of {:.0} in default", outstanding) });
                }
            }
            self.banks[bi].reserves += interest_income + cash_repaid;
            self.banks[bi].notes_issued = (self.banks[bi].notes_issued - principal_repaid).max(0.0);
            self.banks[bi].interest_earned += interest_income;
            self.banks[bi].losses += writeoff;
            let mut idx = 0;
            self.banks[bi].loans.retain(|_| { let k = keep[idx]; idx += 1; k });
            // 2) Depositor interest (a cost paid from reserves).
            let dep_cost = self.banks[bi].deposits * BANK_DEPOSIT_RATE;
            self.banks[bi].reserves -= dep_cost;
            // 2b) v2.0 · IDIOSYNCRATIC BANK RUN. A bank whose reserve ratio has slipped
            //     below the fragility floor faces withdrawals as depositors lose
            //     confidence — even absent a systemic panic. The run drains reserves
            //     (scaled by how far below the floor it is) and can tip a stressed bank
            //     into failure on its own; a sound, well-reserved bank never sees one.
            let rr = self.banks[bi].reserve_ratio();
            if rr < BANK_RUN_RATIO && self.banks[bi].deposits > EPS {
                let severity = ((BANK_RUN_RATIO - rr) / BANK_RUN_RATIO).clamp(0.0, 1.0);
                let withdraw = (self.banks[bi].deposits * BANK_RUN_WITHDRAW * (0.5 + severity))
                    .min(self.banks[bi].deposits);
                self.banks[bi].reserves -= withdraw;
                self.banks[bi].deposits = (self.banks[bi].deposits - withdraw).max(0.0);
                self.banks[bi].events.push(HouseEvent { tick, kind: "run".into(),
                    text: format!("depositors withdraw {:.0} in a run on the bank", withdraw) });
                self.journal.push(JournalEntry { tick, kind: "run".into(), hub: seat as i32,
                    good: -1, value: withdraw,
                    text: format!("Run on {}: depositors pull {:.0} as confidence cracks", self.banks[bi].name, withdraw) });
            }
            // 3) Dividend the bank's net spread to the owning house.
            let owner = self.banks[bi].house as usize;
            let profit = interest_income - dep_cost;
            if profit > 0.0 && owner < self.houses.len() && !self.houses[owner].defunct {
                let dividend = (profit * 0.5).min(self.banks[bi].reserves.max(0.0));
                self.banks[bi].reserves -= dividend;
                self.houses[owner].wealth += dividend;
            }
            // 3b) B4 · BILLS OF EXCHANGE — FX-spread income. The bank profits from
            //     settling trade across its branch cities when they use DIFFERENT coins:
            //     the wider the metal-value gap between the two coins, and the busier the
            //     lighter of the two markets, the larger the fee it captures. This is the
            //     historical core of merchant banking (the Medici/Fugger earned on the
            //     bill, not usury) and rewards a WIDE branch network spanning currencies.
            let branches = self.banks[bi].branches.clone();
            if branches.len() >= 2 {
                let nh = self.hubs.len();
                let mut fx = 0.0f32;
                for i in 0..branches.len() {
                    for j in (i + 1)..branches.len() {
                        let (a, b) = (branches[i] as usize, branches[j] as usize);
                        if a >= nh || b >= nh { continue; }
                        let (ca, cb) = (self.hubs[a].settle_coin, self.hubs[b].settle_coin);
                        if ca < 0 || cb < 0 || ca == cb { continue; }
                        let (ca, cb) = (ca as usize, cb as usize);
                        if ca >= nh || cb >= nh { continue; }
                        let ea = coin_exchange(self.hubs[ca].coin_metal, self.hubs[ca].mint_fineness, self.hubs[ca].coin_trust);
                        let eb = coin_exchange(self.hubs[cb].coin_metal, self.hubs[cb].mint_fineness, self.hubs[cb].coin_trust);
                        let gap = (ea - eb).abs() / ea.max(eb).max(EPS);
                        let vol = self.hub_throughput(a).min(self.hub_throughput(b));
                        fx += gap * vol * BILL_FEE;
                    }
                }
                let fx = fx.min(BILL_INCOME_CAP);
                if fx > 0.0 {
                    self.banks[bi].reserves += fx;
                    self.banks[bi].bills_income += fx;
                }
            }
            // 4) New lending + equity investment (only in calm times).
            if !panicked { self.bank_maybe_lend(bi); self.bank_maybe_invest(bi); }
            // 5) Attract deposits.
            self.bank_maybe_take_deposits(bi);
            // 6) Owner recapitalization: before a bank is allowed to fail, its owning
            //    house — if solvent — injects fresh specie to cover a shortfall (a
            //    family stands behind its bank). This absorbs a transient loss from a
            //    loan default instead of letting one bad year topple a sound bank.
            let owner = self.banks[bi].house as usize;
            let shortfall = (-self.banks[bi].equity()).max(-self.banks[bi].reserves).max(0.0);
            if shortfall > 0.0 && owner < self.houses.len() && !self.houses[owner].defunct {
                let inject = shortfall.min(self.houses[owner].wealth * 0.5);
                if inject > 0.0 {
                    self.houses[owner].wealth -= inject;
                    self.banks[bi].reserves += inject;
                    self.banks[bi].events.push(HouseEvent { tick, kind: "recap".into(),
                        text: format!("{} injects {:.0} to shore up the bank", self.houses[owner].name, inject) });
                }
            }
            // 7) Failure (only if recapitalization couldn't cover it).
            if self.banks[bi].equity() < 0.0 || self.banks[bi].reserves < 0.0 {
                self.fail_bank(bi);
            }
        }
    }


    /// A wealthy house at a bank's seat parks idle capital as an interest-bearing
    /// deposit (a liability that funds the bank's lending). Capped by the
    /// fractional-reserve limit so the bank stays sound.
    pub(crate) fn bank_maybe_take_deposits(&mut self, bi: usize) {
        let tick = self.tick;
        let cap = self.banks[bi].reserves * BANK_RESERVE_MULT;
        if self.banks[bi].liabilities() >= cap { return; }
        if hash01(self.seed, tick as u64 ^ 0xDEED0, bi as u64) > 0.25 { return; }
        let seat = self.banks[bi].seat as usize;
        let owner = self.banks[bi].house;
        let mut best = (usize::MAX, 0.0f32);
        for (hi, h) in self.houses.iter().enumerate() {
            if h.defunct || hi as u32 == owner { continue; }
            if h.hub as usize != seat { continue; }
            if h.wealth > best.1 { best = (hi, h.wealth); }
        }
        if best.0 == usize::MAX || best.1 < 3.0 { return; }
        let amt = (best.1 * 0.1).min(cap - self.banks[bi].liabilities());
        if amt < 0.5 { return; }
        self.houses[best.0].wealth -= amt;
        self.banks[bi].deposits += amt;
        self.banks[bi].reserves += amt;
    }


    /// A sound bank with headroom originates a loan: usually to a promising resident
    /// house (financing its ventures), sometimes to the seat city's treasury (public
    /// works). It issues notes/credit (a liability) and disburses to the borrower.
    pub(crate) fn bank_maybe_lend(&mut self, bi: usize) {
        let tick = self.tick;
        let headroom = self.banks[bi].reserves * BANK_RESERVE_MULT - self.banks[bi].liabilities();
        if headroom < 1.0 { return; }
        if hash01(self.seed, tick as u64 ^ 0x10A40, bi as u64) > 0.3 { return; }
        let seat = self.banks[bi].seat as usize;
        let amt = headroom.min(self.banks[bi].reserves * 0.5).max(1.0);
        let owner = self.banks[bi].house;
        // Richest non-defunct resident borrower of the requested kind (guild or house),
        // homed at the seat and not the bank's own owner.
        let richest_resident = |guild: bool, this: &Self| -> usize {
            let mut best = (usize::MAX, 0.0f32);
            for (hi, h) in this.houses.iter().enumerate() {
                if h.defunct || h.is_guild != guild || hi as u32 == owner { continue; }
                if h.hub as usize != seat { continue; }
                if h.wealth > best.1 { best = (hi, h.wealth); }
            }
            best.0
        };
        // Pick a borrower: most often a resident merchant house (trade venture);
        // sometimes a resident GUILD financing a factory or civic works; sometimes the
        // seat city's treasury (public works).
        let pick = hash01(self.seed, tick as u64 ^ 0x77B10, bi as u64);
        let (bh, bp, purpose) = if pick < 0.55 {
            let h = richest_resident(false, self);
            if h == usize::MAX { return; }
            (h as i32, -1i32, "trade")
        } else if pick < 0.80 {
            let g = richest_resident(true, self);
            if g == usize::MAX {
                (-1i32, seat as i32, "treasury") // no guild here → fund public works instead
            } else {
                let civic = hash01(self.seed, tick as u64 ^ 0x6171C, bi as u64) < 0.4;
                (g as i32, -1i32, if civic { "guild_civic" } else { "guild_factory" })
            }
        } else {
            (-1i32, seat as i32, "treasury")
        };
        // v2.1 · ENDOGENOUS rate — priced per loan instead of a flat house rate:
        //   base × (1 + scarcity·K_s + risk·K_r) + panic premium, capped.
        //   • scarcity = how little lending headroom is left (tight credit → dearer);
        //   • risk = borrower/purpose risk premium (a city treasury is safest, a
        //     speculative works dearest); • panic = a crunch premium at a stressed seat.
        let scarcity = 1.0 - (headroom / (self.banks[bi].reserves * BANK_RESERVE_MULT).max(EPS)).clamp(0.0, 1.0);
        let risk = match purpose {
            "treasury" => 0.0, "guild_civic" => 0.15, "trade" => 0.45, "guild_factory" => 0.60, _ => 0.40,
        };
        let panic_prem = if self.hub_in_panic(seat) { BANK_LOAN_RATE * BANK_RATE_PANIC } else { 0.0 };
        let rate = (BANK_LOAN_RATE * (1.0 + BANK_RATE_SCARCITY * scarcity + BANK_RATE_RISK * risk) + panic_prem)
            .clamp(BANK_LOAN_RATE, BANK_LOAN_RATE_MAX);
        self.banks[bi].loans.push(Loan {
            borrower_house: bh, borrower_polis: bp,
            principal: amt, outstanding: amt, rate,
            start_tick: tick, term_ticks: 1825, purpose: purpose.into(),
        });
        self.banks[bi].notes_issued += amt;
        if bh >= 0 { self.houses[bh as usize].wealth += amt; }
        else if bp >= 0 { self.hubs[bp as usize].treasury += amt; }
    }


    /// A sound bank takes an EQUITY STAKE in a promising manufactory in its branch
    /// network: it injects capital into the works' owning house and, in return, draws
    /// a share of that manufactory's income as a dividend. The stake is carried as a
    /// balance-sheet asset, so a bank can grow its net worth from real PRODUCTION, not
    /// only from lending. (User-requested: "banks should be able to invest in buildings.")
    pub(crate) fn bank_maybe_invest(&mut self, bi: usize) {
        let tick = self.tick;
        // Invest only idle reserves, occasionally, keeping ample liquidity.
        if self.banks[bi].reserves < BANK_FOUND_RESERVE * 0.5 { return; }
        if hash01(self.seed, tick as u64 ^ 0x57A4E, bi as u64) > 0.10 { return; }
        let branches = self.banks[bi].branches.clone();
        // The highest-tier un-staked manufactory in the branch network whose owner is solvent.
        let mut best: (usize, u8) = (usize::MAX, 0);
        for ei in 0..self.hubs.len() {
            let e = &self.hubs[ei];
            if !e.is_estate || e.estate_kind != 6 || e.stake_bank >= 0 { continue; }
            if e.parent < 0 || !branches.contains(&(e.parent as u32)) { continue; }
            let oh = e.owner_house;
            if oh < 0 || (oh as usize) >= self.houses.len() || self.houses[oh as usize].defunct { continue; }
            if e.estate_tier > best.1 { best = (ei, e.estate_tier); }
        }
        let ei = best.0;
        if ei == usize::MAX { return; }
        let tier = self.hubs[ei].estate_tier.max(1);
        let price = (tier as f32 * BANK_STAKE_VALUE_PER_TIER * BANK_STAKE_SHARE)
            .min(self.banks[bi].reserves * 0.4);
        if price < 1.0 || self.banks[bi].reserves < price * 2.0 { return; }
        let good = self.hubs[ei].base_per_capita.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(g, _)| g as u32).unwrap_or(0);
        let oh = self.hubs[ei].owner_house as usize;
        self.banks[bi].reserves -= price;        // specie out
        self.houses[oh].wealth += price;          // capital injected into the works
        self.hubs[ei].stake_bank = bi as i32;
        self.hubs[ei].stake_share = BANK_STAKE_SHARE;
        self.banks[bi].stakes.push(BankStake {
            estate_hub: ei as u32, share: BANK_STAKE_SHARE, basis: price, good });
        let en = self.hubs[ei].name.clone();
        self.banks[bi].events.push(HouseEvent { tick, kind: "stake".into(),
            text: format!("takes a {:.0}% stake in {} for {:.0}", BANK_STAKE_SHARE * 100.0, en, price) });
        let bn = self.banks[bi].name.clone();
        self.journal.push(JournalEntry { tick, kind: "bank".into(), hub: self.hubs[ei].parent,
            good: good as i32, value: price, text: format!("{} buys into {}", bn, en) });
    }


    /// Release a bank's equity stakes (on failure): each works reverts fully to its
    /// owner so it can be re-staked later.
    pub(crate) fn release_bank_stakes(&mut self, bi: usize) {
        let staked: Vec<u32> = self.banks[bi].stakes.iter().map(|s| s.estate_hub).collect();
        for eh in staked {
            if (eh as usize) < self.hubs.len() && self.hubs[eh as usize].stake_bank == bi as i32 {
                self.hubs[eh as usize].stake_bank = -1;
                self.hubs[eh as usize].stake_share = 0.0;
            }
        }
        self.banks[bi].stakes.clear();
    }


    /// A bank fails: depositors are wiped, its notes go worthless, the owning house
    /// is battered, and the failure ignites a regional crash.
    pub(crate) fn fail_bank(&mut self, bi: usize) {
        let tick = self.tick;
        if self.banks[bi].defunct { return; }
        self.banks[bi].defunct = true;
        let seat = self.banks[bi].seat as usize;
        let name = self.banks[bi].name.clone();
        let lost = self.banks[bi].deposits;
        self.banks[bi].events.push(HouseEvent { tick, kind: "failed".into(),
            text: format!("fails — {:.0} in deposits wiped out", lost) });
        self.journal.push(JournalEntry { tick, kind: "bank".into(), hub: seat as i32, good: -1,
            value: lost, text: format!("The {} collapses", name) });
        let owner = self.banks[bi].house as usize;
        if owner < self.houses.len() {
            self.houses[owner].wealth *= 0.6;
            self.houses[owner].prestige *= 0.7;
        }
        self.release_bank_stakes(bi);
        self.trigger_regional_crash(seat, 1, "bank failure");
    }


    /// DLC 3.5 · a regional financial crash. Credit freezes across the origin's
    /// whole trade-connected region (one connectivity `component`): coin trust
    /// collapses, house fortunes are haircut, a region-wide panic event tanks
    /// morale + stability (via the existing sentiment loop), and thinly-reserved
    /// banks in the region are swept away (contagion). Recorded for the panel.
    pub(crate) fn trigger_regional_crash(&mut self, origin: usize, origin_is_bank: u32, cause: &str) {
        if origin >= self.hubs.len() { return; }
        let tick = self.tick;
        let year = self.year();
        let region = self.hubs[origin].component;
        let origin_name = self.hubs[origin].name.clone();
        let until = tick + CRASH_PANIC_TICKS;
        let region_hubs: Vec<usize> = (0..self.hubs.len())
            .filter(|&h| !self.hubs[h].is_estate && self.hubs[h].component == region)
            .collect();
        // 1) Poleis: trust collapse + panic event (sentiment loop does the morale hit).
        for &h in &region_hubs {
            self.hubs[h].coin_trust = (self.hubs[h].coin_trust - CRASH_TRUST_HIT).max(0.0);
            self.hubs[h].trade_wealth *= 0.5;
            self.active_events.push(ActiveEvent {
                kind: "panic".into(), hub: h as i32, good: -1,
                magnitude: 0.5, until_tick: until,
            });
        }
        // 2) Houses homed in the region take a wealth haircut (margin calls).
        for hh in self.houses.iter_mut() {
            if hh.defunct { continue; }
            if region_hubs.contains(&(hh.hub as usize)) {
                hh.wealth *= 1.0 - CRASH_WEALTH_HAIRCUT;
                hh.volume *= 0.7;
            }
        }
        // 3) Contagion: banks in the region face a run; the fragile ones fall.
        let mut banks_failed = origin_is_bank.min(1);
        for bi in 0..self.banks.len() {
            if self.banks[bi].defunct { continue; }
            if !region_hubs.contains(&(self.banks[bi].seat as usize)) { continue; }
            let run = self.banks[bi].deposits * CRASH_CONTAGION_RUN;
            self.banks[bi].reserves -= run;
            self.banks[bi].deposits = (self.banks[bi].deposits - run).max(0.0);
            if self.banks[bi].reserve_ratio() < BANK_RUN_RATIO || self.banks[bi].equity() < 0.0 {
                self.banks[bi].defunct = true;
                self.banks[bi].events.push(HouseEvent { tick, kind: "failed".into(),
                    text: "swept away in the panic".into() });
                self.release_bank_stakes(bi);
                banks_failed += 1;
            }
        }
        let cities_hit = region_hubs.len() as u32;
        let text = format!(
            "The Crash of {}: credit froze across the {} region — {} cities struck, {} banks failed ({}).",
            year, origin_name, cities_hit, banks_failed, cause
        );
        self.journal.push(JournalEntry {
            tick, kind: "crash".into(), hub: origin as i32, good: -1,
            value: cities_hit as f32, text: text.clone(),
        });
        self.crashes.push(CrashRecord {
            year, origin_hub: origin as u32, origin_name, component: region,
            cities_hit, banks_failed, cause: cause.into(), text,
        });
        if self.crashes.len() > CRASH_RECORD_CAP {
            let drop = self.crashes.len() - CRASH_RECORD_CAP;
            self.crashes.drain(0..drop);
        }
    }


    /// DLC 3.5 · after the yearly speculation read, a HIGH-tier (≥4★) bubble may
    /// burst into a regional crash.
    pub(crate) fn maybe_pop_bubbles(&mut self, year: u32) {
        let pops: Vec<usize> = self.spec_centers.iter()
            .filter(|c| c.stars >= 4)
            .filter(|c| hash01(self.seed, c.hub as u64 ^ 0xB0BB1E, year as u64) < CRASH_BUBBLE_POP_CHANCE)
            .map(|c| c.hub as usize)
            .collect();
        for h in pops {
            if h >= self.hubs.len() || self.hub_in_panic(h) { continue; }
            self.trigger_regional_crash(h, 0, "bubble burst");
        }
    }


    /// A3 · push a yearly coin-biography snapshot for every mint, marking the year's
    /// notable monetary event. Called once a year AFTER the basket/reform/crash passes
    /// so circulation, fineness, reform state and any crash are all settled.
    pub(crate) fn snapshot_coins(&mut self, year: u32) {
        let n = self.hubs.len();
        let tick = self.tick;
        // Per-coin circulation (by mint hub INDEX) from this year's baskets.
        let mut circ: Vec<f32> = vec![0.0; n];
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let thru = self.hubs[h].tw_house + self.hubs[h].tw_local + self.hubs[h].tw_guild;
            for &(k, share) in &self.hubs[h].coin_basket {
                if (k as usize) < n { circ[k as usize] += thru * share; }
            }
        }
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].coin_name.is_empty() { continue; }
            let fine = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            let trust = self.hubs[h].coin_trust;
            let prev_fine = self.hubs[h].coin_history.last().map(|s| s.fineness);
            let region = self.hubs[h].component;
            let crashed = self.crashes.iter().any(|c| c.year == year && c.component == region);
            let reformed = self.hubs[h].last_reform_tick != 0
                && tick < self.hubs[h].last_reform_tick + TICKS_PER_YEAR;
            let event = if prev_fine.is_none() { "first".to_string() }
                else if reformed { "reform".to_string() }
                else if fine + 0.005 < prev_fine.unwrap() { "debasement".to_string() }
                else if crashed { "crash".to_string() }
                else { String::new() };
            let snap = CoinSnapshot {
                year, fineness: fine, trust,
                value: coin_value(fine, trust),
                exchange: coin_exchange(self.hubs[h].coin_metal, fine, trust),
                strength: coin_strength(fine, trust),
                price_level: if self.hubs[h].price_level <= 0.0 { 1.0 } else { self.hubs[h].price_level },
                circulating: circ[h],
                metal: self.hubs[h].coin_metal,
                event,
            };
            self.hubs[h].coin_history.push(snap);
            let len = self.hubs[h].coin_history.len();
            if len > COIN_HISTORY_CAP { self.hubs[h].coin_history.drain(0..len - COIN_HISTORY_CAP); }
        }
    }


    /// B3 · civic PUBLIC DEBT (the Monte / Casa di San Giorgio). Once a year each
    /// council seat: (1) SERVICES its bonds — pays the coupon to holders (a stable
    /// return that pulls patrician capital out of risky trade into the public funds);
    /// (2) DEFAULTS with a haircut when the debt has outgrown what its trade can
    /// service (holders' claims are cut and the coin's credit-standing suffers); and
    /// (3) ISSUES fresh bonds when the treasury is short (war / public works), sold to
    /// its richest resident house — turning private wealth into a claim on the city.
    /// Issuance and coupons are pure transfers (house <-> treasury), so the wealth
    /// invariant is untouched; a default only forgoes future coupons + claim value.
    /// Holders are houses only for now (bank-held sovereign debt needs its own asset
    /// class on the balance sheet).
    pub(crate) fn update_public_debt(&mut self, _year: u32) {
        let tick = self.tick;
        if tick < DEBT_START_TICK { return; }
        let year = self.year();
        let n = self.hubs.len();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].population < 1.0 || self.hubs[h].council_house < 0 { continue; }
            let throughput = self.hub_throughput(h).max(EPS);

            // 1) SERVICE the coupon (treasury → bondholders, pro-rata).
            if self.hubs[h].debt_principal > EPS {
                if self.hubs[h].debt_coupon <= 0.0 { self.hubs[h].debt_coupon = DEBT_COUPON; }
                let coupon = self.hubs[h].debt_principal * self.hubs[h].debt_coupon;
                if self.hubs[h].treasury >= coupon {
                    self.hubs[h].treasury -= coupon;
                    let holders = self.hubs[h].debt_holders.clone();
                    let total: f32 = holders.iter().map(|x| x.2).sum::<f32>().max(EPS);
                    for (kind, idx, amt) in holders {
                        if kind != 0 { continue; }
                        if let Some(ho) = self.houses.get_mut(idx as usize) {
                            if !ho.defunct { ho.wealth += coupon * (amt / total); }
                        }
                    }
                }
            }

            // 1b) DELEVERAGE — if the debt has grown heavy relative to trade (usually
            //     because throughput fell), the city retires principal out of a healthy
            //     treasury, RETURNING capital to holders, rather than sliding to default.
            let target = DEBT_TARGET_RATIO * throughput;
            if self.hubs[h].debt_principal > target * DEBT_DELEVERAGE_RATIO && self.hubs[h].treasury > 0.0 {
                let retire = (self.hubs[h].debt_principal - target)
                    .min(self.hubs[h].treasury * 0.3).max(0.0);
                if retire > EPS {
                    let holders = self.hubs[h].debt_holders.clone();
                    let total: f32 = holders.iter().map(|x| x.2).sum::<f32>().max(EPS);
                    for (kind, idx, amt) in &holders {
                        if *kind != 0 { continue; }
                        if let Some(ho) = self.houses.get_mut(*idx as usize) {
                            if !ho.defunct { ho.wealth += retire * (amt / total); } // capital returned
                        }
                    }
                    for hd in self.hubs[h].debt_holders.iter_mut() { hd.2 *= 1.0 - retire / (total.max(EPS)); }
                    self.hubs[h].treasury -= retire;
                    self.hubs[h].debt_principal -= retire;
                }
            }

            // 2) DEFAULT / haircut when debt overwhelms the city's capacity to service.
            if self.hubs[h].debt_principal > EPS
                && self.hubs[h].debt_principal / throughput > DEBT_DEFAULT_RATIO {
                let cut = self.hubs[h].debt_principal * DEBT_HAIRCUT;
                for hd in self.hubs[h].debt_holders.iter_mut() { hd.2 *= 1.0 - DEBT_HAIRCUT; }
                self.hubs[h].debt_principal -= cut;
                self.hubs[h].coin_trust = (self.hubs[h].coin_trust - DEBT_DEFAULT_TRUST_HIT).max(0.0);
                let city = self.hubs[h].name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "debt".into(), hub: h as i32, good: -1, value: cut,
                    text: format!("{} restructures its public debt — bondholders take a {:.0}% haircut ({:.0} written down)",
                        city, DEBT_HAIRCUT * 100.0, cut),
                });
            }

            // 3) ISSUE toward a STANDING public debt (a permanent civic institution,
            //    like Venice's Monte) that funds PUBLIC WORKS — the proceeds flow to the
            //    city's people (civic pool), so the borrowing does real work rather than
            //    piling in the treasury. Gated on serviceability: a council only issues
            //    as far as it can pay the resulting coupon out of its treasury income.
            let target = DEBT_TARGET_RATIO * throughput;
            let cap_headroom = DEBT_MAX_RATIO * throughput - self.hubs[h].debt_principal;
            if self.hubs[h].debt_principal < target && cap_headroom > 1.0 {
                // Richest non-defunct resident (private) house here subscribes.
                let mut buyer = (usize::MAX, 0.0f32);
                for (hi, ho) in self.houses.iter().enumerate() {
                    if ho.defunct || ho.is_guild || ho.hub as usize != h { continue; }
                    if ho.wealth > buyer.1 { buyer = (hi, ho.wealth); }
                }
                if buyer.0 != usize::MAX && buyer.1 > 50.0 {
                    let step = (target - self.hubs[h].debt_principal)
                        .min(throughput * DEBT_ISSUE_STEP).min(buyer.1 * 0.3).min(cap_headroom);
                    let coupon = if self.hubs[h].debt_coupon > 0.0 { self.hubs[h].debt_coupon } else { DEBT_COUPON };
                    let new_coupon = (self.hubs[h].debt_principal + step) * coupon;
                    // Serviceability: the treasury must be able to carry the new coupon.
                    if step > 1.0 && self.hubs[h].treasury > new_coupon * DEBT_SERVICE_COVER {
                        self.houses[buyer.0].wealth -= step;
                        self.hubs[h].civic_pool += step;          // proceeds → public works → the people
                        self.hubs[h].finance.spent_works += step;
                        self.hubs[h].debt_coupon = coupon;
                        let fresh = self.hubs[h].debt_principal <= EPS;
                        self.hubs[h].debt_principal += step;
                        // Merge into the buyer's holding (or add, capped).
                        let bidx = buyer.0 as u32;
                        if let Some(hd) = self.hubs[h].debt_holders.iter_mut().find(|x| x.0 == 0 && x.1 == bidx) {
                            hd.2 += step;
                        } else if self.hubs[h].debt_holders.len() < DEBT_HOLDER_CAP {
                            self.hubs[h].debt_holders.push((0, bidx, step));
                        }
                        if fresh {
                            let (city, house) = (self.hubs[h].name.clone(), self.houses[buyer.0].name.clone());
                            self.journal.push(JournalEntry {
                                tick, kind: "debt".into(), hub: h as i32, good: -1, value: step,
                                text: format!("{} opens a public debt (Monte) at {:.1}% — {} subscribes {:.0}",
                                    city, coupon * 100.0, house, step),
                            });
                        }
                    }
                }
            }
            let _ = year;
        }
    }
}
