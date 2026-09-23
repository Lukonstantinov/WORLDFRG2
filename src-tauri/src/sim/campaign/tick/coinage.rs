//! coinage.rs — `docs/MONEY_AND_COINAGE_PLAN.md` M1: the coin CATALOGUE data
//! model. Purely OBSERVATIONAL: it records what `decide_coinage`/`apply_coinage`
//! (money.rs) already decide every year — a mint's fineness, trust and metal —
//! as real countable objects (a `Currency` with 1-3 named `Denom`inations, each
//! carrying a timeline of `Issue`s) instead of the bare `TickHub.coin_name`
//! string F2 named. Nothing outside this file reads `currencies`/`issues` yet
//! (the catalogue window is M2, unbuilt); `sim_fingerprint` (tests.rs) folds
//! neither, so this whole slice is bit-identical to the sim it observes.
//!
//! What is NOT yet true, so a reader does not mistake this for more than it is:
//! `Issue.struck/circulating/hoarded/melted/lost` are quantities that only mean
//! something once real bullion moves through real purses (M3's parallel
//! ledger); M1 ships them at `0.0` on every issue. Mint CLOSURE (D4/§3.7) and
//! culture-rooted currency naming beyond today's flat 10-name `coin_denomination`
//! list (§3.3) are both explicitly M6/M2 work, not this file's.
use super::*;

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
                        self.issues.push(Issue {
                            id, currency: ci, denom: denoms.len() as u32, year,
                            authority: authority.clone(),
                            grams: denom_standard_grams(tier), fineness: fine,
                            struck: 0.0, circulating: 0.0, hoarded: 0.0, melted: 0.0, lost: 0.0,
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
                        self.issues.push(Issue {
                            id, currency: ci as u32, denom: di as u32, year,
                            authority: authority.clone(),
                            grams: denom_standard_grams(tier), fineness: fine,
                            struck: 0.0, circulating: 0.0, hoarded: 0.0, melted: 0.0, lost: 0.0,
                            cause, cognomen: cognomen.clone(),
                        });
                        self.currencies[ci].denoms[di].issues.push(id);
                    }
                    self.currencies[ci].last_fineness = fine;
                }
            }
        }
    }
}
