//! THE ARRIVAL CALENDAR — cargo in flight, filed by the day it lands.
//!
//! A full-size campaign carries ~800k cargoes in flight, and three daily passes
//! used to walk all of them: the arrivals sweep (`retain` on `eta_tick <= tick`)
//! and two busy-vessel recounts (`fulfill_contracts`, `dispatch`). The cost was
//! the size of the STOCK of cargo, paid every day, not the day's FLOW.
//!
//! Here cargo is bucketed by `eta_tick`, so a day touches only what lands that
//! day, and per-house busy-vessel counts are kept as running counters updated on
//! every push and landing. Bit-identical by construction: within one day,
//! landings come out in insertion order — exactly the order the old `retain`
//! produced, since every cargo due earlier had already landed — and the counts
//! are integers, order-free.
//!
//! Serialised as the same flat list `Vec<InTransit>` was, so saves are unchanged.

use super::InTransit;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Busy vessels per house: `[sea, land]` for ALL owned cargo (what
/// `fulfill_contracts` and `decide_fleets` count) and for owned NON-contract
/// cargo (what `dispatch` counts — contract vessels are covered by the standing
/// reservation there).
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Busy { pub sea: i32, pub land: i32, pub sea_spot: i32, pub land_spot: i32 }

#[derive(Clone, Debug, Default)]
pub struct TransitBook {
    by_day: BTreeMap<u32, Vec<InTransit>>,
    len: usize,
    busy: Vec<Busy>,
}

impl TransitBook {
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn push(&mut self, c: InTransit) {
        self.count(&c, 1);
        self.by_day.entry(c.eta_tick).or_default().push(c);
        self.len += 1;
    }

    pub fn clear(&mut self) {
        self.by_day.clear();
        self.len = 0;
        self.busy.clear();
    }

    /// Every cargo due on or before `tick`, in landing order, removed from the book.
    pub fn take_due(&mut self, tick: u32) -> Vec<InTransit> {
        let mut out = Vec::new();
        while let Some(e) = self.by_day.first_entry() {
            if *e.key() > tick { break; }
            out.extend(e.remove());
        }
        for c in &out { self.count(c, -1); }
        self.len -= out.len();
        out
    }

    /// Busy vessels for house `oi` (zero for a house with nothing in flight).
    pub fn busy(&self, oi: usize) -> Busy { self.busy.get(oi).copied().unwrap_or_default() }

    pub fn iter(&self) -> impl Iterator<Item = &InTransit> { self.by_day.values().flatten() }

    fn count(&mut self, c: &InTransit, d: i32) {
        if c.owner < 0 { return; }
        let oi = c.owner as usize;
        if self.busy.len() <= oi { self.busy.resize(oi + 1, Busy::default()); }
        let b = &mut self.busy[oi];
        if c.sea { b.sea += d; } else { b.land += d; }
        if !c.contract { if c.sea { b.sea_spot += d; } else { b.land_spot += d; } }
    }
}

impl<'a> IntoIterator for &'a TransitBook {
    type Item = &'a InTransit;
    type IntoIter = std::iter::Flatten<std::collections::btree_map::Values<'a, u32, Vec<InTransit>>>;
    fn into_iter(self) -> Self::IntoIter { self.by_day.values().flatten() }
}

impl FromIterator<InTransit> for TransitBook {
    fn from_iter<I: IntoIterator<Item = InTransit>>(it: I) -> Self {
        let mut b = TransitBook::default();
        for c in it { b.push(c); }
        b
    }
}

impl Serialize for TransitBook {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.collect_seq(self.iter()) }
}

impl<'de> Deserialize<'de> for TransitBook {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Vec::<InTransit>::deserialize(d)?.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cargo(i: u32, eta: u32, owner: i32, sea: bool, contract: bool) -> InTransit {
        InTransit {
            from: i, to: 0, good: 0, amount: i as f32, eta_tick: eta, owner, sea, river: false,
            phase: 0, home: -1, contract, price: 0.0, local: false, via: -1, hops: 0, origin_km: 0.0,
        }
    }

    fn recount(cs: &[InTransit], oi: usize) -> Busy {
        let mut b = Busy::default();
        for c in cs.iter().filter(|c| c.owner == oi as i32) {
            if c.sea { b.sea += 1 } else { b.land += 1 }
            if !c.contract { if c.sea { b.sea_spot += 1 } else { b.land_spot += 1 } }
        }
        b
    }

    /// Lands exactly what the old `retain` sweep landed, in the same order, and
    /// the running busy counters always equal a full recount.
    #[test]
    fn transit_book_matches_a_flat_list() {
        let mut flat: Vec<InTransit> = Vec::new();
        let mut book = TransitBook::default();
        let mut i = 0u32;
        for tick in 0..60u32 {
            for k in 0..5u32 {
                i += 1;
                let c = cargo(i, tick + 1 + (i * 7 + k) % 9, (i % 4) as i32 - 1, i % 3 == 0, i % 5 == 0);
                flat.push(c.clone());
                book.push(c);
            }
            let mut want = Vec::new();
            flat.retain(|c| if c.eta_tick <= tick { want.push(c.from); false } else { true });
            let got: Vec<u32> = book.take_due(tick).iter().map(|c| c.from).collect();
            assert_eq!(got, want, "landing order differs at tick {tick}");
            assert_eq!(book.len(), flat.len());
            for oi in 0..4 { assert_eq!(book.busy(oi), recount(&flat, oi), "busy counts for house {oi}"); }
        }
        // Saved as the flat list it replaces, and read back to the same book.
        let json = serde_json::to_string(&book).unwrap();
        let back: TransitBook = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), book.len());
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
        for oi in 0..4 { assert_eq!(back.busy(oi), book.busy(oi)); }
    }
}
