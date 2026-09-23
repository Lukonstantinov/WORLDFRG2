//! coinage.rs — `docs/MONEY_AND_COINAGE_PLAN.md` M1 (the coin CATALOGUE) + M3
//! (the PARALLEL LEDGER's mint-side half).
//!
//! **M1** records what `decide_coinage`/`apply_coinage` (money.rs) already
//! decide every year — a mint's fineness, trust and metal — as real countable
//! objects (a `Currency` with 1-3 named `Denom`inations, each carrying a
//! timeline of `Issue`s) instead of the bare `TickHub.coin_name` string F2
//! named. Nothing outside this file reads `currencies`/`issues` for economic
//! effect (the catalogue WINDOW is M2, `commands/campaign_commands/read_
//! money.rs::campaign_get_coin_catalogue`); `sim_fingerprint` (tests.rs) folds
//! neither, so M1 itself is bit-identical to the sim it observes.
//!
//! **M3** adds `Purse`s (§3.1) and the mint-striking transaction (§3.2): every
//! time a new `Issue` is recorded, a real STRUCK quantity is computed and
//! split into seigniorage (→ the city treasury's OWN purse), brassage (→ a
//! household purse at the mint — the mint workers' wages) and circulation (→
//! a local-merchant purse at the mint — the sim has no tracked "who brought
//! the bullion" yet, so this is the honest placeholder: `docs/ACTORS_AND_
//! CARRIAGE_PLAN.md`'s own measured finding is that ~96% of trade already
//! moves on no one's account). **This is additive, not a mirror of the
//! existing `+=` sites** — `TickHub.treasury`/`House.wealth` etc. are
//! completely untouched by this file; the purses are a SEPARATE ledger
//! computed alongside them, exactly D10's "parallel ledger first" calls for.
//! `sim_fingerprint` still folds none of `purses`/`currencies`/`issues`, so
//! this remains bit-identical to every existing gate.
//!
//! **Scoped down from §3.2's own full design**, stated plainly: real bullion
//! cargo (silver/gold mined, shipped to a mint, sometimes lost at sea) is NOT
//! wired here — that needs new dispatch/production integration, real risk to
//! measure, and is queued, not attempted. The STRUCK quantity below is sized
//! from the mint's own already-computed regional throughput/bullion-ratio
//! proxy (`decide_coinage`'s own inputs), not from a real cargo delivery.
//! Melting, loss, hoarding and wear (§3.2's SINKS) are also not implemented —
//! every issue's `circulating` equals its `struck` until a sink exists to
//! move the difference, which is exactly what
//! `the_coin_ledger_conserves_every_struck_coin` (tests.rs) asserts.
//!
//! **M6** adds mint CLOSURE (§3.7), CATALOGUE-ONLY: `mark_mint_closures`
//! reads the mint's own city's EXISTING `coin_basket` share (a live,
//! already-computed signal — no new mechanism to measure acceptance) and
//! marks `Currency.open = false` once it has sat below `MINT_CLOSE_SHARE`
//! for `MINT_CLOSE_YEARS` running. It deliberately does NOT touch `TickHub.
//! has_mint`/`coin_name` — closing the REAL, live coinage mechanism
//! (`decide_coinage`/`apply_coinage`, money.rs) is a further, separate
//! change with real economic consequences (trust, seigniorage, freight
//! discount all key off those fields) and is queued, not attempted here.
//! Because it only ever writes fields nothing else reads, this is safe to
//! ship at its real dose rather than gated inert — see the constants' own
//! doc comments.
//!
//! **M8** adds the household purse (§3.9), scoped DOWN from its real design:
//! `household_ledger_pass` mirrors a hub's existing wage (`household_income_
//! pass`'s own formula) as a coin deposit, then immediately spends the
//! identical amount on the ration via `take_coin_issue` (M3's missing spend
//! side, ISSUE-TARGETED — see that function's own doc comment for why a
//! plain `take_coin` was wrong here) — a closed loop, so this needs no dose
//! gate at all, the same reason M3 shipped without one. What it does NOT
//! build: a household that saves, borrows, or is priced out of its ration
//! when the wage falls short (§3.9's own R6-flagged risk) — that is queued
//! behind `FOOD_AFFORDABILITY_DOSE` (M7) actually being walked, per the
//! plan's own R6.
//!
//! Culture-rooted currency naming beyond today's flat 10-name
//! `coin_denomination` list (§3.3), the real three-stage diffusion of §3.6,
//! and the REAL mint-closure economic effect all remain unbuilt, queued work.
use super::*;

/// §3.1 · who a purse belongs to.
pub const HOLDER_CITY_TREASURY: u8 = 0;
pub const HOLDER_HOUSE: u8 = 1;
pub const HOLDER_BANK: u8 = 2;
pub const HOLDER_HOUSEHOLD: u8 = 3;
pub const HOLDER_LOCAL_MERCHANT: u8 = 4;
#[allow(dead_code)]
pub const HOLDER_MINT: u8 = 5;

/// §3.1 · `(holder, hub) → coins/bullion`. Sparse: a purse is only created the
/// first time something is added to it. `holder_id` is a house/bank index for
/// `HOLDER_HOUSE`/`HOLDER_BANK`, `-1` for every per-hub-only holder (treasury,
/// households, local merchants, the mint itself).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Purse {
    pub holder_kind: u8,
    pub holder_id: i32,
    pub hub: u32,
    /// `(issue id, count)`, one entry per issue actually held.
    pub coins: Vec<(u32, f32)>,
    /// Raw metal awaiting striking: `[gold, silver, copper]`.
    pub bullion: [f32; 3],
}

/// §3.2 · what share of a mint's yearly throughput becomes a fresh striking
/// the year a new issue is recorded. A proxy for real bullion delivery
/// (this file's own header comment) — small, so the parallel ledger's coin
/// stock grows on a plausible order against `hub_throughput`, not a
/// guessed absolute.
const STRUCK_FRAC_OF_THROUGHPUT: f32 = 0.05;
/// §3.2 · brassage — the minting cost, paid to the mint's own household purse
/// as wages. Small and constant; debasement's extra seigniorage is what
/// varies (mirrors `COIN_SEIGNIORAGE`'s own `(1 - fineness)` term).
const BRASSAGE_FRAC: f32 = 0.03;

/// M5 (§3.5) · the PROBABILITY a given shipment settles by barter instead of
/// coin, per day. This is a placeholder for the real trigger M6 will add ("a
/// trade chooses coin when both sides can use it and falls back to barter
/// otherwise") — until mint reach/coin acceptance is modelled, there is no
/// live signal to decide it on, so a flat dose is the honest stand-in. Ships
/// at 0.0 — a true no-op, per this file's own header comment.
const BARTER_DOSE: f32 = 0.0;
/// §3.5 · the valuation loss a barter settlement takes vs. coin (goods
/// carried and resold are worth less to the merchant than cash in hand).
/// Inert while `BARTER_DOSE` is 0.0; walking it is queued alongside the dose.
const BARTER_SPREAD: f32 = 0.25;

/// M6 (§3.7) · a mint's own coin must hold at least this share of its own
/// city's `coin_basket` or it starts counting toward closure. Unlike M5's
/// dose, this constant carries NO economic-concentration risk to walk
/// carefully — `mark_mint_closures` only ever writes `Currency.open`/
/// `closed_year`/`below_share_years`, never a hub or a house, so it is
/// shipped LIVE at its real value rather than gated inert.
pub(crate) const MINT_CLOSE_SHARE: f32 = 0.05;
/// M6 (§3.7) · consecutive years below `MINT_CLOSE_SHARE` before a mint
/// closes — long enough that an ordinary bad year or two doesn't shutter a
/// real mint, short enough that a genuinely abandoned coin closes inside a
/// human generation.
pub(crate) const MINT_CLOSE_YEARS: u32 = 15;

/// §3.4/D6 · a people's unit of account — resolved once per culture, exactly
/// like `CultureRule`, and never re-rolled. `ladder` is the ratio to the
/// smallest coin, LARGEST unit first (e.g. `[240, 20, 1]` is a pound of 20
/// shillings of 12 pence — 240 pence to the pound); `names` is the matching
/// tier label per entry. Always 2 or 3 entries, `ladder.len() == names.len()`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnitOfAccount {
    pub culture: String,
    pub ladder: Vec<u32>,
    pub names: Vec<String>,
}

/// The three stylised historical ladders §3.4 names, picked deterministically
/// per culture (never a specific real-world claim — a categorical variety, the
/// same discipline `culture_rules`' law-of-inheritance draw already uses).
const UOA_LADDERS: [(&[u32], &[&str]); 3] = [
    (&[240, 20, 1], &["pound", "shilling", "penny"]),
    (&[96, 8, 1], &["mina", "stater", "obol"]),
    (&[60, 1], &["talent", "drachm"]),
];

/// 0 Gold · 1 Silver · 2 Petty (copper/billon) — a `Denom`'s tier.
pub const DENOM_GOLD: u8 = 0;
pub const DENOM_SILVER: u8 = 1;
pub const DENOM_PETTY: u8 = 2;

const GOLD_STANDARD_GRAMS: f32 = 4.5;
const SILVER_STANDARD_GRAMS: f32 = 3.0;
const PETTY_STANDARD_GRAMS: f32 = 1.0;

fn denom_tier_label(tier: u8) -> &'static str {
    match tier { DENOM_GOLD => "Gold", DENOM_PETTY => "Petty", _ => "Silver" }
}
fn denom_standard_grams(tier: u8) -> f32 {
    match tier { DENOM_GOLD => GOLD_STANDARD_GRAMS, DENOM_PETTY => PETTY_STANDARD_GRAMS, _ => SILVER_STANDARD_GRAMS }
}

/// §3.3 · issue cause. Only First/Debasement/Reform are produced by M1 — a
/// realm-struck NewRuler issue and a WarIssue both wait on mechanisms this
/// slice does not touch (a realm dynasty minting its own coin, M6's queue
/// item 1; a war-financed strike, M9's queue item 6).
pub const ISSUE_FIRST: u8 = 0;
pub const ISSUE_DEBASEMENT: u8 = 1;
pub const ISSUE_REFORM: u8 = 2;
#[allow(dead_code)]
pub const ISSUE_NEW_RULER: u8 = 3;
#[allow(dead_code)]
pub const ISSUE_WAR_ISSUE: u8 = 4;

/// §3.3 · one dated striking of a `Denom`. Permanent — an issue is a historical
/// fact once recorded and is never edited or removed (the same discipline
/// `House.line`/`CrisisRecord` already use for a house's own permanent record).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Issue {
    pub id: u32,
    /// Index into `CampaignSim::currencies`.
    pub currency: u32,
    /// Index into that currency's OWN `denoms` (not a `DENOM_*` tier code —
    /// a currency carries at most one denom per tier, so the two agree, but a
    /// reader indexes `Currency::denoms` with this field directly).
    pub denom: u32,
    pub year: u32,
    /// The council house / authority that struck it, BY NAME at record time —
    /// a copy, not a live index, since the authority can dissolve after its
    /// issue stands (the same "record the name, not the index" rule
    /// `DeadHouse`'s diagnostics already use).
    pub authority: String,
    pub grams: f32,
    pub fineness: f32,
    /// M3 (the parallel ledger) populates these from real purses; M1 ships
    /// every issue at 0.0 — see this file's own header comment.
    pub struck: f32,
    pub circulating: f32,
    pub hoarded: f32,
    pub melted: f32,
    pub lost: f32,
    pub cause: u8,
    pub cognomen: String,
}

/// §3.3 · one denomination of a `Currency` (its gold trade coin, its everyday
/// silver, or its petty billon) — a name, a nominal weight, and the timeline of
/// `Issue`s struck under it.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Denom {
    pub tier: u8,
    pub name: String,
    pub standard_grams: f32,
    /// Indices into `CampaignSim::issues`, oldest first.
    pub issues: Vec<u32>,
}

/// §3.3 · one mint's currency — one per `mint_hub`, created the year that hub
/// first strikes a coin (mirrors `decide_coinage`'s own charter/first-mint
/// decision) and never removed (a closed mint's currency stays in the
/// catalogue, `open = false` — see §3.7, not yet implemented here).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Currency {
    pub mint_hub: u32,
    pub name: String,
    /// Index into `CampaignSim::units_of_account`, or `u32::MAX` if the mint's
    /// culture has none resolved (should not happen once `ensure_unit_of_
    /// account` has run at least once, but a defensive default all the same —
    /// a world with no culture map, e.g. every unit-test fixture, never
    /// resolves one).
    pub unit_of_account: u32,
    pub denoms: Vec<Denom>,
    pub open: bool,
    pub closed_year: u32,
    /// Internal bookkeeping only, not part of §3.3's design — the fineness
    /// this currency's issues were last recorded at, so M1 can tell a real
    /// debasement/reform from a year where nothing changed. Mirrors
    /// `TickHub.mint_fineness_prev`'s own role in `decide_coinage`.
    pub last_fineness: f32,
    /// M6 (§3.7) · consecutive years this currency's own `coin_basket` share
    /// at its own mint city has sat below `MINT_CLOSE_SHARE`. Internal
    /// bookkeeping, resets to 0 the moment the share recovers.
    #[serde(default)]
    pub below_share_years: u32,
}

fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

impl CampaignSim {
    /// §3.4/D6 · resolve each live people's unit of account ONCE and keep it —
    /// the direct mirror of `ensure_culture_rules` (houses.rs), same call sites
    /// (campaign start, then yearly alongside it so a creole or resettled
    /// culture picks one up too).
    pub(crate) fn ensure_unit_of_account(&mut self) {
        let mut names: Vec<String> = Vec::new();
        for c in self.hub_culture.iter() {
            if c.is_empty() || c == "—" { continue; }
            if !names.iter().any(|n| n == c) { names.push(c.clone()); }
        }
        for c in self.creoles.iter() {
            if !names.iter().any(|n| *n == c.name) { names.push(c.name.clone()); }
        }
        for n in names {
            if self.units_of_account.iter().any(|u| u.culture == n) { continue; }
            let i = (hash01(self.seed, fnv1a64(&n), 0xC01A) * UOA_LADDERS.len() as f32) as usize
                % UOA_LADDERS.len();
            let (ladder, tier_names) = UOA_LADDERS[i];
            self.units_of_account.push(UnitOfAccount {
                culture: n,
                ladder: ladder.to_vec(),
                names: tier_names.iter().map(|s| s.to_string()).collect(),
            });
        }
    }

    /// The 2 or 3 denomination TIERS a mint at `hub` strikes, from the same
    /// `coin_metal` the region's reachable bullion already picked
    /// (`coin_metal_for`, money.rs) — 0 silver · 1 gold · 2 electrum ·
    /// 3 bronze/billon. A small bullion-poor mint strikes petty and/or silver
    /// only; a gold- or electrum-rich region strikes gold alongside silver —
    /// D2's "2-3 denominations", read off data the sim already computed rather
    /// than invented fresh for the catalogue.
    fn denom_tiers_for_metal(metal: u8) -> &'static [u8] {
        match metal {
            1 | 2 => &[DENOM_GOLD, DENOM_SILVER],
            3 => &[DENOM_PETTY],
            _ => &[DENOM_SILVER, DENOM_PETTY],
        }
    }

    fn unique_currency_name(&self, base: &str) -> String {
        if !self.currencies.iter().any(|c| c.name == base) { return base.to_string(); }
        let mut i = 2u32;
        loop {
            let candidate = format!("{base} ({i})");
            if !self.currencies.iter().any(|c| c.name == candidate) { return candidate; }
            i += 1;
        }
    }

    /// M3 · find or create the purse for `(kind, id, hub)`.
    fn purse_idx(&mut self, kind: u8, id: i32, hub: u32) -> usize {
        if let Some(i) = self.purses.iter().position(|p| p.holder_kind == kind && p.holder_id == id && p.hub == hub) {
            return i;
        }
        self.purses.push(Purse { holder_kind: kind, holder_id: id, hub, coins: Vec::new(), bullion: [0.0; 3] });
        self.purses.len() - 1
    }

    /// M3 · add `amount` of `issue_id` coin to a purse. The ONE place this
    /// file's parallel ledger creates money — a negative or zero amount is a
    /// no-op, never a withdrawal. `take_coin` (M8, below) is the matching
    /// spend side, added once a caller (the household ledger) actually
    /// needed one.
    pub(crate) fn add_coin(&mut self, kind: u8, id: i32, hub: u32, issue_id: u32, amount: f32) {
        if !(amount > 0.0) { return; }
        let pi = self.purse_idx(kind, id, hub);
        match self.purses[pi].coins.iter_mut().find(|(iid, _)| *iid == issue_id) {
            Some(e) => e.1 += amount,
            None => self.purses[pi].coins.push((issue_id, amount)),
        }
    }

    /// M8 · total coin a purse holds, across every issue — a read-only
    /// convenience for callers (and tests) that don't care which issue a
    /// coin belongs to, only how much is there.
    #[cfg(test)]
    pub(crate) fn purse_total_for_test(&self, kind: u8, id: i32, hub: u32) -> f32 {
        self.purses.iter()
            .find(|p| p.holder_kind == kind && p.holder_id == id && p.hub == hub)
            .map(|p| p.coins.iter().map(|(_, c)| *c).sum())
            .unwrap_or(0.0)
    }

    /// M8 · remove up to `amount` of `issue_id` coin from a purse — the
    /// ISSUE-TARGETED spend side `add_coin` was missing, for a caller that
    /// must spend the SAME issue it just deposited (`household_ledger_
    /// pass`, below) rather than "whichever coin the purse happens to hold
    /// first".
    ///
    /// **A first cut drained a purse's `coins` list in plain VECTOR order,
    /// with no regard for `issue_id`, and was wrong** — found by measurement,
    /// not review (2026-09-23d). A household purse can already hold an OLDER
    /// issue's coin (M3's `strike_issue` credits brassage into the SAME
    /// `HOLDER_HOUSEHOLD` purse at the mint's own hub) by the time
    /// `household_ledger_pass` deposits this month's wage under the CURRENT
    /// issue and immediately withdraws the identical amount — an
    /// issue-blind take could drain the OLDER issue's entry instead, while
    /// the fresh deposit sits untouched, and the withdrawn amount was still
    /// re-credited to the merchant purse under the CURRENT issue (`add_
    /// coin`'s own `issue_id` argument) — silently relabelling coin from one
    /// issue to another, conserved in TOTAL but not PER ISSUE, which is
    /// exactly what `the_coin_ledger_conserves_every_struck_coin` checks
    /// (measured drift: issue 6 short by ~0.003 out of ~34). Targeting the
    /// SAME issue that was just deposited is what keeps a closed add/take
    /// loop closed under its own tag; clamped to what that ONE entry holds,
    /// never negative, the same discipline `stock_take` uses for goods.
    fn take_coin_issue(&mut self, kind: u8, id: i32, hub: u32, issue_id: u32, amount: f32) -> f32 {
        if !(amount > 0.0) { return 0.0; }
        let pi = match self.purses.iter().position(|p| p.holder_kind == kind && p.holder_id == id && p.hub == hub) {
            Some(i) => i,
            None => return 0.0,
        };
        let Some(entry) = self.purses[pi].coins.iter_mut().find(|(iid, _)| *iid == issue_id) else {
            return 0.0;
        };
        let take = entry.1.min(amount);
        entry.1 -= take;
        if entry.1 <= 1e-9 {
            self.purses[pi].coins.retain(|(iid, c)| *iid != issue_id || *c > 1e-9);
        }
        take
    }

    /// M8 (§3.9) · the household purse — a PARALLEL LEDGER pass on exactly
    /// M3's own terms: it only ever writes `purses`, which nothing outside
    /// this file reads, so it is observe-only and bit-identical to every
    /// existing gate (`tick::tests`, `econ_`) whatever it computes — no dose
    /// constant needed for this half, the same reason M3 shipped without one.
    ///
    /// Monthly, at each hub with an open mint currency: mirrors `household_
    /// income_pass`'s own wage formula (`trade_wealth * HOUSEHOLD_WAGE_
    /// SHARE`, independent of that function's own `HOUSEHOLD_MONETIZATION_
    /// DOSE` gate, which stays 0.0 and is irrelevant to this purely
    /// observational ledger) as a coin deposit into the hub's household
    /// purse, then immediately debits the SAME amount back out as a
    /// consumption purchase into the local-merchant purse — the scoped-down
    /// half of §3.9's "paid consumption": a household's wage is spent on its
    /// ration in the same month it is earned, a closed loop that conserves
    /// the coin ledger by construction (deposit and debit are the identical
    /// float, taken through `take_coin` rather than assumed).
    ///
    /// **What this does NOT build**, named per rule 36: a household SAVING,
    /// falling into debt, or being priced out of its ration when the wage
    /// can't cover it (§3.9's own R6-flagged risk — the real "paid
    /// consumption" that would feed back into `update_food_and_starvation`).
    /// That is the actual M8 dose walk, and it waits on `FOOD_AFFORDABILITY_
    /// DOSE` (M7) being raised first, per the plan's own R6 ("M7 before M8,
    /// no exceptions").
    pub(crate) fn household_ledger_pass(&mut self) {
        let n = self.hubs.len();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
            let ci = match self.currencies.iter().position(|c| c.mint_hub == h as u32 && c.open) {
                Some(i) => i,
                None => continue,
            };
            let issue_id = match self.currencies[ci].denoms.first().and_then(|d| d.issues.last()) {
                Some(&id) => id,
                None => continue,
            };
            let wage = self.hubs[h].trade_wealth.max(0.0) * HOUSEHOLD_WAGE_SHARE;
            if wage <= 0.0 { continue; }
            let h32 = h as u32;
            self.add_coin(HOLDER_HOUSEHOLD, -1, h32, issue_id, wage);
            let taken = self.take_coin_issue(HOLDER_HOUSEHOLD, -1, h32, issue_id, wage);
            if taken > 0.0 {
                self.add_coin(HOLDER_LOCAL_MERCHANT, -1, h32, issue_id, taken);
            }
        }
    }

    /// §3.2 · strike a fresh issue: size it from the mint's own throughput,
    /// split into seigniorage/brassage/circulation, credit the three purses,
    /// and return `(struck, circulating)` for the caller to stamp onto the
    /// `Issue` it is about to push. `fine` is this issue's OWN fineness (a
    /// more debased strike skims more seigniorage, mirroring `COIN_SEIGNIORAGE`
    /// in money.rs).
    fn strike_issue(&mut self, hub: usize, issue_id: u32, fine: f32) -> (f32, f32) {
        let through = self.hub_throughput(hub);
        let struck = through * STRUCK_FRAC_OF_THROUGHPUT;
        if struck <= 0.0 { return (0.0, 0.0); }
        let seign = struck * (1.0 - fine).max(0.0);
        let brassage = struck * BRASSAGE_FRAC;
        let circulation = (struck - seign - brassage).max(0.0);
        let h = hub as u32;
        self.add_coin(HOLDER_CITY_TREASURY, -1, h, issue_id, seign);
        self.add_coin(HOLDER_HOUSEHOLD, -1, h, issue_id, brassage);
        self.add_coin(HOLDER_LOCAL_MERCHANT, -1, h, issue_id, circulation);
        (struck, seign + brassage + circulation)
    }

    /// M1 · record this year's coinage decisions into the catalogue. Called
    /// once a year AFTER `apply_coinage`/`maybe_reform_coinage` have settled
    /// (mirrors `snapshot_coins`'s own timing — see mod.rs's yearly hook), so
    /// `TickHub.coin_name`/`mint_fineness`/`coin_metal` all read the year's
    /// FINAL values. Read-only over every OTHER field: this never writes back
    /// to a hub, a house or the journal beyond the catalogue's own vectors.
    pub(crate) fn record_currencies(&mut self, year: u32) {
        let n = self.hubs.len();
        for h in 0..n {
            if self.hubs[h].is_estate || !self.hubs[h].has_mint || self.hubs[h].coin_name.is_empty() {
                continue;
            }
            let fine = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            let metal = self.hubs[h].coin_metal;
            let culture = self.hub_culture.get(h).cloned().unwrap_or_default();
            let uoa_idx = self.units_of_account.iter().position(|u| u.culture == culture)
                .map(|i| i as u32).unwrap_or(u32::MAX);
            let authority = {
                let ch = self.hubs[h].council_house;
                if ch >= 0 { self.houses.get(ch as usize).map(|hh| hh.name.clone()) } else { None }
            }.unwrap_or_else(|| format!("the council of {}", self.hubs[h].name));

            let cur_idx = self.currencies.iter().position(|c| c.mint_hub == h as u32);
            match cur_idx {
                None => {
                    // First mint — found the currency and strike its opening issues.
                    let tiers = Self::denom_tiers_for_metal(metal);
                    let cur_name = self.unique_currency_name(&self.hubs[h].coin_name.clone());
                    let ci = self.currencies.len() as u32;
                    let mut denoms = Vec::with_capacity(tiers.len());
                    for &tier in tiers {
                        let id = self.next_issue_id;
                        self.next_issue_id += 1;
                        let (struck, circulating) = self.strike_issue(h, id, fine);
                        self.issues.push(Issue {
                            id, currency: ci, denom: denoms.len() as u32, year,
                            authority: authority.clone(),
                            grams: denom_standard_grams(tier), fineness: fine,
                            struck, circulating, hoarded: 0.0, melted: 0.0, lost: 0.0,
                            cause: ISSUE_FIRST,
                            cognomen: format!("the First {}", denom_tier_label(tier)),
                        });
                        denoms.push(Denom {
                            tier, name: format!("{} {}", denom_tier_label(tier), self.hubs[h].name),
                            standard_grams: denom_standard_grams(tier), issues: vec![id],
                        });
                    }
                    self.currencies.push(Currency {
                        mint_hub: h as u32, name: cur_name, unit_of_account: uoa_idx,
                        denoms, open: true, closed_year: 0, last_fineness: fine,
                        below_share_years: 0,
                    });
                }
                Some(ci) => {
                    // A currency already open — issue a fresh striking only on a real
                    // fineness move (mirrors `snapshot_coins`'s own debasement/reform
                    // read, money.rs), never on an ordinary steady year.
                    let prev_fine = self.currencies[ci].last_fineness;
                    let reformed = self.hubs[h].last_reform_tick != 0
                        && self.tick < self.hubs[h].last_reform_tick + TICKS_PER_YEAR;
                    let debased = fine + 0.005 < prev_fine;
                    if !reformed && !debased { continue; }
                    let cause = if reformed { ISSUE_REFORM } else { ISSUE_DEBASEMENT };
                    let cognomen = if reformed {
                        format!("the Reform of {year}")
                    } else {
                        format!("the Debased Money of {year}")
                    };
                    let ndenoms = self.currencies[ci].denoms.len();
                    for di in 0..ndenoms {
                        let tier = self.currencies[ci].denoms[di].tier;
                        let id = self.next_issue_id;
                        self.next_issue_id += 1;
                        let (struck, circulating) = self.strike_issue(h, id, fine);
                        self.issues.push(Issue {
                            id, currency: ci as u32, denom: di as u32, year,
                            authority: authority.clone(),
                            grams: denom_standard_grams(tier), fineness: fine,
                            struck, circulating, hoarded: 0.0, melted: 0.0, lost: 0.0,
                            cause, cognomen: cognomen.clone(),
                        });
                        self.currencies[ci].denoms[di].issues.push(id);
                    }
                    self.currencies[ci].last_fineness = fine;
                }
            }
        }
    }

    /// M5 (§3.5) · barter as a real settlement. Ships at `BARTER_DOSE = 0.0` —
    /// a true no-op (the early return below), following this codebase's own
    /// established pattern for a behavioural change too risky to dose blind
    /// (`N1B_OWNERLESS_LOSS_RATE`/`CAPACITY_BIND_DOSE`/`LOCAL_SATIETY`/
    /// `FOREIGN_PRESTIGE` all shipped the identical way — CLAUDE.md §5).
    ///
    /// **Deliberately NOT woven into `dispatch`'s own carrier cascade.** That
    /// cascade is the single most fragile piece of this codebase — every one
    /// of N1/N1c/N2/N4's dose walks broke the hard wealth bound or the
    /// multi-seed inheritance gate on a much smaller change than a coin/
    /// barter branch inside it would be (CLAUDE.md §8.5/§8.15). This is
    /// instead a wholly SEPARATE, additive pass over the round's own
    /// `recent_trades` — at any dose it can only ever ADD a counter-trade
    /// alongside a shipment `dispatch` already made; it can never resize,
    /// redirect or undo one.
    ///
    /// **Scoped down from §3.5's own design**: the payment good moves as an
    /// IMMEDIATE stock transfer (both sides settle the same day), not a
    /// separate `InTransit` return leg with its own travel time — modelling
    /// a real return voyage, and the commodity-money naming §3.5 also asks
    /// for, are real, separate work for when this is actually dosed.
    /// `BARTER_SPREAD` still prices the valuation loss the real mechanism
    /// will keep.
    pub(crate) fn barter_settlement_pass(&mut self) {
        self.barter_settlement_pass_e(BARTER_DOSE, BARTER_SPREAD);
    }

    /// The pure-parameter twin of `barter_settlement_pass`, in the same shape
    /// `local_satiety_mult_e`/`transit_need_mult_e` already use — lets a test
    /// exercise the real mechanism at a NONZERO dose while the shipped
    /// constant stays 0.0.
    pub(crate) fn barter_settlement_pass_e(&mut self, dose: f32, spread: f32) {
        if dose <= 0.0 { return; }
        let tick = self.tick;
        let ng = self.goods.len();
        let n = self.hubs.len();
        // Only today's trades — `recent_trades` is a rolling 400-entry window
        // that is never cleared, so an unfiltered walk would re-process the
        // same historical trade every subsequent day.
        let todays: Vec<RecentTrade> = self.recent_trades.iter()
            .filter(|t| t.tick == tick).cloned().collect();
        for t in todays {
            let roll = hash01(self.seed, tick as u64 ^ 0xBA47E5,
                ((t.from as u64) << 16) ^ (t.to as u64) ^ (t.good as u64));
            if roll >= dose { continue; }
            let (a, b) = (t.from as usize, t.to as usize);
            if a >= n || b >= n || a == b { continue; }
            // The staple B is most willing to pay away: highest stock relative
            // to its own need, excluding the good just delivered (never pay a
            // buyer back in the very good it just bought).
            let mut best: Option<(usize, f32)> = None;
            for g2 in 0..ng {
                if g2 == t.good { continue; }
                let st = stock_of(&self.hubs[b].stock, g2);
                if st <= EPS { continue; }
                let need = self.hubs[b].base_per_capita.get(g2).copied().unwrap_or(0.0)
                    * self.hubs[b].population;
                let ratio = st / need.max(1.0);
                let better = match best { Some((_, r)) => ratio > r, None => true };
                if better { best = Some((g2, ratio)); }
            }
            let Some((pg, _)) = best else { continue };
            let base = self.goods[pg].base_value.max(EPS);
            let need_pg = self.hubs[b].base_per_capita.get(pg).copied().unwrap_or(0.0)
                * self.hubs[b].population;
            let price_b = self.live_price(stock_of(&self.hubs[b].stock, pg), need_pg, base);
            if price_b <= EPS { continue; }
            let sale_value = t.amount * t.price * (1.0 - spread);
            let cap = stock_of(&self.hubs[b].stock, pg) * 0.5;
            let pay_qty = (sale_value / price_b).min(cap);
            if pay_qty <= EPS { continue; }
            stock_take(&mut self.hubs[b].stock, pg, pay_qty);
            stock_add_ungraded(&mut self.hubs[a].stock, pg, pay_qty);
            self.diag_barter_trades += 1;
            self.diag_barter_volume += pay_qty * price_b;
        }
    }

    /// M6 (§3.7) · mint closure — CATALOGUE-ONLY, see this file's own header.
    /// Called once a year. Reads `hub.coin_basket` (already computed by
    /// `update_currency_baskets`, money.rs) for the mint's own share of its
    /// own city's coin use; writes only `Currency.open`/`closed_year`/
    /// `below_share_years`.
    pub(crate) fn mark_mint_closures(&mut self, year: u32) {
        for i in 0..self.currencies.len() {
            if !self.currencies[i].open { continue; }
            let hub = self.currencies[i].mint_hub as usize;
            let share = match self.hubs.get(hub) {
                Some(h) => h.coin_basket.iter()
                    .find(|&&(k, _)| k as usize == hub)
                    .map(|&(_, s)| s)
                    .unwrap_or(0.0),
                None => 0.0,
            };
            if share < MINT_CLOSE_SHARE {
                self.currencies[i].below_share_years += 1;
            } else {
                self.currencies[i].below_share_years = 0;
            }
            if self.currencies[i].below_share_years >= MINT_CLOSE_YEARS {
                self.currencies[i].open = false;
                self.currencies[i].closed_year = year;
            }
        }
    }
}
