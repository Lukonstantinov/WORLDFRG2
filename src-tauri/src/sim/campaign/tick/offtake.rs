//! offtake — ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.8 (D1, D5), "the big one",
//! built late on purpose (§4.8's own gate note: "this is the slice that moves
//! goods").
//!
//! D1 splits share payout by works kind: a MANUFACTORY pays its shareholders a
//! DIVIDEND (money, out of sale proceeds — the pre-existing bank-stake path,
//! `production.rs`'s per-sale cut, untouched by this file). An EXTRACTION works
//! (farm/mine/fishery/vineyard/plantation) instead pays OFFTAKE: the holder
//! takes its fraction of PHYSICAL OUTPUT, quality-ranked (D5) — the largest
//! holder draws from the finest grade band first (`stock_take_finest_first`),
//! cascading down only once that band is exhausted, so a controlling share is
//! worth more than its bare fraction the same way a first-pressing right was.
//!
//! Deliberately a MONTHLY pass over the estate's already-accumulated
//! `stock[g][band]`, not a hook into the daily production write — the daily
//! loop (`mod.rs`'s production step) is the single hottest, most gate-sensitive
//! path in the whole tick (see `estate_condition_pass`'s and `envoys.rs`'s own
//! postmortems on this exact codebase's RNG/threshold sensitivity), and
//! draining a SNAPSHOT of current stock once a month is exactly the same
//! idiom `sync_and_stock_warehouses`'s own stocking step already uses for
//! house depots — not a new pattern.
//!
//! Currently only reachable through `envoys.rs`'s PARTIAL outcome (the one
//! place that creates a `payout: 0` row), so — like envoys — this is real and
//! load-bearing at production scale but proven inert in the small dynamics/
//! `econ_` fixtures, where an envoy essentially never fires at the tuned dose.
use super::*;

impl CampaignSim {
    pub(crate) fn offtake_delivery_pass(&mut self) {
        let ng = self.goods.len();
        let n = self.hubs.len();
        for ei in 0..n {
            // Never a manufactory (estate_kind 6) — D1 keeps those on dividend.
            if !self.hubs[ei].is_estate || self.hubs[ei].estate_kind == 6 { continue; }
            if self.hubs[ei].shares.is_empty() { continue; }
            let parent = self.hubs[ei].parent;
            if parent < 0 || parent as usize >= n { continue; }
            let parent = parent as usize;
            let mut offtake: Vec<Share> = self.hubs[ei].shares.iter()
                .filter(|s| s.payout == 0 && s.frac > 0.0).cloned().collect();
            if offtake.is_empty() { continue; }
            // D5 · the largest holder skims first.
            offtake.sort_by(|a, b| b.frac.partial_cmp(&a.frac).unwrap_or(std::cmp::Ordering::Equal));
            for g in 0..ng {
                for sh in &offtake {
                    let total = stock_of(&self.hubs[ei].stock, g);
                    if total <= EPS { continue; }
                    let want = total * sh.frac.clamp(0.0, 1.0);
                    let drawn = stock_take_finest_first(&mut self.hubs[ei].stock, g, want);
                    if drawn <= EPS { continue; }
                    self.credit_offtake(sh, parent, g, drawn);
                }
            }
        }
    }

    /// Deliver `qty` of good `g`, drawn from an estate's stock, to one offtake
    /// holder — into its own warehouse at the estate's parent city (creating
    /// the depot if it doesn't exist yet, same shape `sync_and_stock_
    /// warehouses` already pushes), capped by that depot's free room. A
    /// bank or realm holder has no warehouse to receive goods into, so it
    /// takes the cash equivalent at the estate's own local price instead —
    /// the same fallback D6/A5 already give a holder without local presence.
    fn credit_offtake(&mut self, sh: &Share, parent: usize, g: usize, qty: f32) {
        match sh.holder_kind {
            1 | 2 => { // house or guild — both live in `houses`
                let hi = sh.holder as usize;
                if hi >= self.houses.len() || self.houses[hi].defunct { return; }
                let wi = self.warehouses.iter()
                    .position(|w| w.owner == hi as i32 && w.hub == parent as u32)
                    .unwrap_or_else(|| {
                        let ng = self.goods.len();
                        self.warehouses.push(Warehouse {
                            owner: hi as i32, hub: parent as u32, capacity: WH_START_CAP,
                            stock: vec![0.0; ng], tier: Self::capacity_tier(WH_START_CAP), damage: 0.0,
                        });
                        self.warehouses.len() - 1
                    });
                let used: f32 = self.warehouses[wi].stock.iter().sum();
                let room = (self.warehouses[wi].capacity - used).max(0.0);
                let credit = qty.min(room);
                if credit > EPS { self.warehouses[wi].stock[g] += credit; }
                // Anything beyond the depot's room is lost on the dock — a real,
                // if quiet, consequence of a holding outgrowing its storage
                // (the same "overflow spoils fastest" idea 4.2 already applies
                // to a city's own warehouse).
            }
            3 => { // bank
                let bi = sh.holder as usize;
                if bi >= self.banks.len() || self.banks[bi].defunct { return; }
                let price = self.live_price(self.hub_stock(parent, g), 0.0, self.goods[g].base_value);
                let value = qty * price;
                self.banks[bi].reserves += value;
                self.banks[bi].dividends_earned += value;
            }
            4 => { // realm
                let ri = sh.holder as usize;
                if ri >= self.realms.len() { return; }
                let price = self.live_price(self.hub_stock(parent, g), 0.0, self.goods[g].base_value);
                self.realms[ri].treasury += qty * price;
            }
            _ => {}
        }
    }
}
