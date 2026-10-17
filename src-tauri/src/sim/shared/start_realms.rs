//! STARTING REALMS (owner decisions 2026-10-17, `docs/PROVINCES_REALMS_LOGISTICS_REDESIGN.md` §6/§8).
//!
//! When a world's provinces are generated, its countries are generated with them: realms
//! of RANDOM size (a free city of one province up to a great kingdom), each a contiguous
//! group of provinces with its own distinct colour. They are world data (metadata
//! `start_realms`), so the author can recolour, rename, reassign provinces or found new
//! realms before a campaign; at campaign start they become live realms, are randomised
//! again, or are dropped so history starts from nothing (`realm_start_mode`).

use serde::{Deserialize, Serialize};

use super::provinces::Province;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartRealm {
    pub name: String,
    pub color: [u8; 3],
    /// Province id of the capital.
    pub capital: u32,
    pub provinces: Vec<u32>,
}

/// The palette realms are coloured from (also the editor's colour picker). Muted,
/// map-friendly hues that stay distinct from each other.
pub const REALM_PALETTE: [[u8; 3]; 16] = [
    [176, 71, 58], [63, 111, 168], [201, 154, 58], [79, 138, 90], [138, 79, 160], [47, 143, 138],
    [168, 96, 58], [106, 125, 47], [176, 74, 122], [74, 90, 160], [154, 138, 58], [58, 122, 90],
    [192, 106, 80], [90, 74, 138], [122, 154, 74], [168, 58, 90],
];

fn rng(seed: u64) -> impl FnMut() -> f32 {
    // splitmix64 first: nearby seeds must not give nearby (or equal) streams.
    let mut s = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    s = (s ^ (s >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    s = (s ^ (s >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    s = (s ^ (s >> 31)) | 1;
    move || {
        s ^= s << 13; s ^= s >> 7; s ^= s << 17;
        (s >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Group `provinces` into contiguous realms of random size, deterministically in `seed`.
/// About 30% are single-province free cities; the rest aim for 2…16 provinces (most
/// small, a few large). Names follow size: Free City · County · Duchy · Kingdom.
/// Colours: the first palette colour no neighbouring realm already uses.
pub fn generate_start_realms(provinces: &[Province], seed: u64) -> Vec<StartRealm> {
    let mut r = rng(seed);
    let n = provinces.len();
    let index: std::collections::HashMap<u32, usize> = provinces.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
    let mut owner = vec![usize::MAX; n];
    // Visit provinces in a seeded order so realms do not all start in one corner.
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() { let j = (r() * (i + 1) as f32) as usize % (i + 1); order.swap(i, j); }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for &start in &order {
        if owner[start] != usize::MAX { continue; }
        let target = if r() < 0.3 { 1 } else { 2 + ((r() * r()) * 15.0) as usize };
        let gi = groups.len();
        let mut members = vec![start];
        owner[start] = gi;
        let mut k = 0;
        while k < members.len() && members.len() < target {
            let here = members[k];
            for nb in &provinces[here].neighbors {
                let Some(&j) = index.get(nb) else { continue };
                if owner[j] == usize::MAX && members.len() < target && r() < 0.85 {
                    owner[j] = gi;
                    members.push(j);
                }
            }
            k += 1;
        }
        groups.push(members);
    }
    // Capital: the province with the most rural people. Colours: avoid neighbours'.
    let mut colors: Vec<usize> = vec![usize::MAX; groups.len()];
    let mut out = Vec::with_capacity(groups.len());
    for (gi, members) in groups.iter().enumerate() {
        let cap = *members.iter().max_by_key(|&&i| (provinces[i].rural_pop, std::cmp::Reverse(provinces[i].id))).unwrap();
        let mut used = std::collections::HashSet::new();
        for &m in members {
            for nb in &provinces[m].neighbors {
                if let Some(&j) = index.get(nb) {
                    let og = owner[j];
                    if og != gi && og < colors.len() && colors[og] != usize::MAX { used.insert(colors[og]); }
                }
            }
        }
        let c = (0..REALM_PALETTE.len()).map(|k| (k + gi * 7) % REALM_PALETTE.len()).find(|k| !used.contains(k)).unwrap_or(gi % REALM_PALETTE.len());
        colors[gi] = c;
        let kind = match members.len() { 1 => "Free City of", 2..=3 => "County of", 4..=7 => "Duchy of", _ => "Kingdom of" };
        out.push(StartRealm {
            name: format!("{kind} {}", provinces[cap].name),
            color: REALM_PALETTE[c],
            capital: provinces[cap].id,
            provinces: members.iter().map(|&i| provinces[i].id).collect(),
        });
    }
    out
}

#[cfg(test)]
mod start_realms_tests {
    use super::*;

    fn prov(id: u32, neighbors: Vec<u32>, rural: u32) -> Province {
        let mut p: Province = serde_json::from_value(serde_json::json!({
            "id": id, "name": format!("P{id}"), "seat_x": 0, "seat_y": 0, "cells": 10, "area_km2": 100,
            "island": 0, "neighbors": [], "koppen": 12, "elevation_class": 0, "mean_fertility": 0.5,
            "coastal": false, "goods": [], "culture": "", "rural_pop": 0, "analog": "", "settlements": []
        })).expect("a minimal province");
        p.neighbors = neighbors;
        p.rural_pop = rural;
        p
    }

    #[test]
    fn start_realms_cover_every_province_once_contiguously_with_varied_sizes() {
        // A 10×6 grid of provinces (4-neighbour).
        let (w, h) = (10u32, 6u32);
        let ps: Vec<Province> = (0..w * h).map(|i| {
            let (x, y) = (i % w, i / w);
            let mut nb = Vec::new();
            if x > 0 { nb.push(i - 1); } if x + 1 < w { nb.push(i + 1); }
            if y > 0 { nb.push(i - w); } if y + 1 < h { nb.push(i + w); }
            prov(i, nb, 100 + (i * 37 % 50))
        }).collect();
        let realms = generate_start_realms(&ps, 42);
        let mut seen = std::collections::HashSet::new();
        for r in &realms {
            assert!(r.provinces.contains(&r.capital));
            for &p in &r.provinces { assert!(seen.insert(p), "province {p} in two realms"); }
            // Contiguous: every member after the first touches an earlier member.
            for (k, &p) in r.provinces.iter().enumerate().skip(1) {
                assert!(r.provinces[..k].iter().any(|q| ps[*q as usize].neighbors.contains(&p)), "realm {} not contiguous", r.name);
            }
        }
        assert_eq!(seen.len(), (w * h) as usize, "every province belongs to a realm");
        let sizes: std::collections::HashSet<usize> = realms.iter().map(|r| r.provinces.len()).collect();
        assert!(sizes.contains(&1) && sizes.len() >= 3, "sizes vary, including free cities: {sizes:?}");
        assert_eq!(realms, generate_start_realms(&ps, 42), "deterministic");
        assert_ne!(realms, generate_start_realms(&ps, 43), "randomised by seed");
    }
}
