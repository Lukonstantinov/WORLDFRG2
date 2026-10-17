//! THE CROWN'S GOVERNMENT (owner decisions 2026-10-17,
//! `docs/PROVINCES_REALMS_LOGISTICS_REDESIGN.md` §8).
//!
//! - **Laws** per realm: crown tax, levies, succession, trade policy, town privileges
//!   (cities always keep their own council and pay the crown a fee), tolerance.
//! - **Governors** per province of a realm (not the capital's own): loyalty and
//!   honesty; a dishonest one SKIMS the crown's dues, the crown REPLACES one that
//!   fails it, and a disloyal one over restless people far from the capital may BREAK
//!   AWAY and found his own one-province realm.
//! - The **royal council** is derived where it is read (`campaign_realm_overview`): it
//!   holds no state of its own.
//! Every economic effect is scaled by `CROWN_DOSE`; at 0 nothing here touches money or
//! territory (laws and governors are still seated, so the window can show them).

use super::*;

/// Crown laws. Levels 1 light · 2 moderate · 3 heavy; 0 = not yet set (derived on first
/// pass). Succession/trade/tolerance index the names below.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct RealmLaws {
    pub tax: u8,
    pub levy: u8,
    pub succession: u8,
    pub trade: u8,
    pub town_fee: u8,
    pub tolerance: u8,
}
pub const SUCCESSION_NAMES: [&str; 4] = ["Primogeniture", "Elective", "Partible", "Seniority"];
pub const TRADE_NAMES: [&str; 4] = ["Open markets", "River tolls", "Staple right", "Embargo on rivals"];
pub const TOLERANCE_NAMES: [&str; 3] = ["Welcomed", "Tolerated", "Restricted"];

/// A province's governor, appointed by the crown that holds it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Governor {
    pub name: String,
    pub loyalty: f32,
    pub honesty: f32,
    pub since_tick: u32,
    /// The realm that appointed him (-1 = no governor).
    pub realm: i32,
}

/// How strongly laws and governors act on money and territory (see module doc).
/// Walked 0 → 1.0: `tick::tests` 421/421 and `econ_` 6/6 at both, with IDENTICAL `econ_`
/// numbers, because no economy fixture has a realm holding provinces under a governor.
/// The gate cannot see this mechanism; `a_disloyal_governor_breaks_away_with_his_province`
/// is what tests it.
pub(crate) const CROWN_DOSE: f32 = 1.0;
/// Town fee a member city pays the crown each year, share of its treasury, by level.
const TOWN_FEE: [f32; 4] = [0.0, 0.01, 0.02, 0.04];
/// Crown tax law multiplies what the crown collects, by level.
pub(crate) const CROWN_TAX_MULT: [f32; 4] = [1.0, 0.8, 1.0, 1.25];
/// Yearly chance a disloyal governor over a restless, distant province breaks away.
const BREAKAWAY_CHANCE: f32 = 0.10;

impl Governor {
    /// Share of the crown's dues a governor keeps: nothing above honesty 0.5, up to
    /// a quarter for the most dishonest.
    pub fn skim(&self) -> f32 { ((0.5 - self.honesty).max(0.0) * 0.5).clamp(0.0, 0.25) }
}

impl CampaignSim {
    /// The province a hub sits in, or -1.
    fn crown_hub_province(&self, h: usize) -> i32 { self.hub_province.get(h).copied().unwrap_or(-1) }

    /// Risk (0..1) that province `p`'s governor breaks away this year: disloyalty ×
    /// unrest × distance from the capital.
    pub(crate) fn breakaway_risk(&self, p: usize) -> f32 {
        let Some(g) = self.prov_governor.get(p) else { return 0.0 };
        if g.realm < 0 { return 0.0; }
        let unrest = self.prov_unrest.get(p).copied().unwrap_or(0.0);
        let far = self.realms.get(g.realm as usize).and_then(|r| {
            let cap = self.crown_hub_province(r.capital_hub as usize);
            let (a, b) = (self.prov_seat.get(p)?, self.prov_seat.get(cap.max(0) as usize)?);
            let mut dx = (a[0] - b[0]).abs(); if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
            Some(((dx * dx + (a[1] - b[1]).powi(2)).sqrt() * EARTH_EQUATOR_KM / self.world_w.max(1.0) / 800.0).min(1.0))
        }).unwrap_or(0.0);
        ((1.0 - g.loyalty).max(0.0) * 0.6 + unrest * 0.3 + far * 0.3).clamp(0.0, 1.0) * if g.loyalty < 0.35 { 1.0 } else { 0.3 }
    }

    /// Yearly: seat laws on new realms, appoint/replace governors, drift loyalty, and
    /// (scaled by `CROWN_DOSE`) collect town fees and let disloyal governors break away.
    pub(crate) fn crown_yearly_pass(&mut self) {
        if self.realms.is_empty() || self.prov_realm.is_empty() { return; }
        let tick = self.tick;
        let np = self.prov_realm.len();
        if self.prov_governor.len() != np { self.prov_governor.resize(np, Governor { realm: -1, ..Default::default() }); }
        // Laws: derived once per realm from its id and seat, then kept.
        for (ri, r) in self.realms.iter_mut().enumerate() {
            if r.laws.tax != 0 { continue; }
            let h = |k: u64| hash01(self.seed ^ 0xC20E, ri as u64, k);
            r.laws = RealmLaws {
                tax: 1 + (h(1) * 3.0) as u8 % 3, levy: 1 + (h(2) * 3.0) as u8 % 3,
                succession: (h(3) * 4.0) as u8 % 4, trade: (h(4) * 4.0) as u8 % 4,
                town_fee: 1 + (h(5) * 3.0) as u8 % 3, tolerance: (h(6) * 3.0) as u8 % 3,
            };
        }
        let (ww, wh) = (self.world_w.max(1.0) as u32, self.world_h.max(1.0) as u32);
        for p in 0..np {
            let ri = self.prov_realm[p];
            let live = ri >= 0 && (ri as usize) < self.realms.len() && self.realms[ri as usize].fallen_tick == 0;
            let capital_prov = if live { self.crown_hub_province(self.realms[ri as usize].capital_hub as usize) } else { -1 };
            if !live || capital_prov == p as i32 {
                if self.prov_governor[p].realm >= 0 { self.prov_governor[p] = Governor { realm: -1, ..Default::default() }; }
                continue;
            }
            let cohesion = self.realms[ri as usize].cohesion;
            let unrest = self.prov_unrest.get(p).copied().unwrap_or(0.0);
            let g = &self.prov_governor[p];
            let needs_new = g.realm != ri
                || (g.loyalty < 0.2 && hash01(self.seed ^ 0x6E0, tick as u64, p as u64) < 0.5)
                || (g.skim() > 0.15 && hash01(self.seed ^ 0x6E1, tick as u64, p as u64) < 0.3);
            if needs_new {
                let seat = self.prov_seat.get(p).copied().unwrap_or([0.0, 0.0]);
                let (sx, sy) = (seat[0].max(0.0) as u32 % ww, (seat[1].max(0.0) as u32).min(wh - 1));
                let salt = (tick as u64) << 20 ^ p as u64;
                let family = crate::sim::names::gen_family_name(sx, sy, ww, wh, salt);
                let replaced = g.realm == ri && !g.name.is_empty();
                let old = g.name.clone();
                let name = crate::sim::names::gen_head_name(sx, sy, ww, wh, &family, salt);
                self.prov_governor[p] = Governor {
                    name: name.clone(),
                    loyalty: 0.45 + 0.5 * hash01(self.seed ^ 0x6E2, tick as u64, p as u64),
                    honesty: 0.3 + 0.65 * hash01(self.seed ^ 0x6E3, tick as u64, p as u64),
                    since_tick: tick, realm: ri,
                };
                if replaced {
                    self.realms[ri as usize].events.push(RealmEvent { tick, kind: "governor".into(),
                        text: format!("The crown replaces {old} with {name} as governor") });
                }
                continue;
            }
            // Loyalty drifts with the realm's cohesion and the province's mood.
            let gm = &mut self.prov_governor[p];
            gm.loyalty = (gm.loyalty + 0.04 * (cohesion - 0.5) - 0.05 * (unrest - 0.3)).clamp(0.0, 1.0);
        }
        if CROWN_DOSE <= 0.0 { return; }
        // Town fees: member cities (not the seat) pay the crown a share of their treasury.
        for h in 0..self.hubs.len() {
            let ri = self.hubs[h].realm;
            if ri < 0 || self.hubs[h].is_estate || self.hubs[h].realm_role == REALM_ROLE_SEAT { continue; }
            let Some(r) = self.realms.get(ri as usize) else { continue };
            if r.fallen_tick > 0 { continue; }
            let fee = self.hubs[h].treasury.max(0.0) * TOWN_FEE[r.laws.town_fee.min(3) as usize] * CROWN_DOSE;
            self.hubs[h].treasury -= fee;
            self.realms[ri as usize].treasury += fee;
        }
        // Breakaways: at most one a year world-wide, the riskiest province first.
        let mut worst: Option<(usize, f32)> = None;
        for p in 0..np {
            let risk = self.breakaway_risk(p);
            if risk > 0.6 && worst.map(|w| risk > w.1).unwrap_or(true) { worst = Some((p, risk)); }
        }
        if let Some((p, risk)) = worst {
            if hash01(self.seed ^ 0xB2EA, tick as u64, p as u64) < BREAKAWAY_CHANCE * risk * CROWN_DOSE {
                self.governor_breaks_away(p);
            }
        }
    }

    /// Province `p` leaves its realm under its governor, as a new one-province realm.
    pub(crate) fn governor_breaks_away(&mut self, p: usize) {
        let old = self.prov_realm[p];
        if old < 0 { return; }
        let Some(seat) = self.province_seat_hub(p) else { return };
        let gov = self.prov_governor[p].name.clone();
        self.prov_realm[p] = -1;
        self.realms[old as usize].provinces.retain(|&q| q != p as u32);
        if self.hubs[seat].realm == old { self.hubs[seat].realm = -1; }
        let yr = self.tick / TICKS_PER_YEAR;
        let id = self.found_civic_realm(seat, yr, 2);
        if (id as usize) < self.realms.len() {
            if !self.realms[id as usize].provinces.contains(&(p as u32)) { self.realms[id as usize].provinces.push(p as u32); }
            self.prov_realm[p] = id as i32;
        }
        let realm_name = self.realms[old as usize].name.clone();
        self.prov_governor[p] = Governor { realm: -1, ..Default::default() };
        self.journal.push(JournalEntry { tick: self.tick, kind: "breakaway".into(), hub: seat as i32, good: -1, value: 0.0,
            text: format!("Governor {gov} breaks with {realm_name} and rules {} as his own", self.hubs[seat].name) });
    }
}
