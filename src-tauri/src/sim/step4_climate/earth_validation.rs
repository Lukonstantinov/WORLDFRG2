//! Earth-accuracy validation harness — the objective regression gate.
//!
//! This is what turns "the map looks believable" into a number. It loads a real
//! Earth landmass (a 1° elevation grid baked from the GMT `earth_relief` DEM) and
//! the canonical **Köppen-Geiger** reference map (Kottek & Rubel, 0.5°, majority-
//! downsampled to the same grid), runs the *actual* Ocean & Atmosphere → Climate
//! pipeline on it, and scores how well the generated Köppen classification agrees
//! with the real one.
//!
//! The score is **area-weighted** (cos φ) so the over-represented polar rows don't
//! dominate, and reported at two grains: the **main class** (A/B/C/D/E — the axis a
//! player actually perceives: tropical / arid / temperate / continental / polar)
//! and the **exact zone** (Cfa vs Cfb …). The test asserts a floor on the main-
//! class agreement so a future change that breaks the global pattern fails CI.
//!
//! Regenerate the fixtures with the baker in the commit that introduced this file
//! (Köppen ASCII from koeppen-geiger.vu-wien.ac.at + GMT earth_relief_01d).

#![cfg(test)]

use crate::db::schema;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{ocean, temperature, jets, precipitation, koppen, elevation};
use rusqlite::Connection;

// 0.5° grid — the Köppen-Geiger reference's native resolution, so no reference
// downsampling, and the cell-based reaches in the ocean/continentality model sit
// closer to their real (km) footprint than a coarse 1° grid would.
const W: usize = 720;
const H: usize = 360;

static ELEV_I16: &[u8] = include_bytes!("fixtures/earth_elev_720x360.i16");
static KOPPEN_U8: &[u8] = include_bytes!("fixtures/earth_koppen_720x360.u8");

/// Köppen main class (perceptual axis) for a WF2 köppen code. Highland (H) is
/// mapped to E for scoring — WF2 emits it on cold high terrain the reference (which
/// has no highland class) labels mostly ET/EF or Dfc.
fn main_letter(code: u8) -> u8 {
    match code {
        koppen::AF | koppen::AM | koppen::AW | koppen::AS => b'A',
        koppen::BWH | koppen::BWK | koppen::BSH | koppen::BSK => b'B',
        koppen::CSA | koppen::CSB | koppen::CSC | koppen::CFA | koppen::CFB | koppen::CFC
        | koppen::CWA | koppen::CWB | koppen::CWC => b'C',
        koppen::DFA | koppen::DFB | koppen::DFC | koppen::DFD | koppen::DSA | koppen::DSB
        | koppen::DSC | koppen::DSD | koppen::DWA | koppen::DWB | koppen::DWC | koppen::DWD => b'D',
        koppen::ET | koppen::EF => b'E',
        koppen::H => b'E',
        _ => 0,
    }
}

/// Build a WorldBuffer from the Earth fixtures and run the full climate pipeline.
fn run_earth() -> (WorldBuffer, Vec<u8>) {
    let conn = Connection::open_in_memory().unwrap();
    schema::create_tables(&conn).unwrap();
    for (k, v) in [("grid_width", W.to_string()), ("grid_height", H.to_string())] {
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        ).unwrap();
    }
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();

    // Reference Köppen (u8 per cell, 0 = ocean) and elevation (i16 metres).
    let reference: Vec<u8> = KOPPEN_U8.to_vec();
    assert_eq!(reference.len(), W * H, "koppen fixture size");
    assert_eq!(ELEV_I16.len(), W * H * 2, "elev fixture size");

    for i in 0..W * H {
        let land = reference[i] != 0;
        buf.terrain[i] = if land { 1 } else { 0 };
        let e = i16::from_le_bytes([ELEV_I16[i * 2], ELEV_I16[i * 2 + 1]]) as f32;
        // Land elevation normalised to 0..1 (× 8848 m); ocean stays 0 (the pipeline
        // derives sea depth itself, exactly as it does for a generated world).
        buf.elevation[i] = if land { (e.max(0.0) / 8848.0).clamp(0.0, 1.0) } else { 0.0 };
    }

    // Sea depth + a proper continental shelf, mirroring the tail of the terrain
    // phase, so the ocean model sees the same inputs it would in the app.
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, 42, 12.0, 0.4, 0.3, 8.0);

    // ── The exact Ocean & Atmosphere → Climate sequence (see sim_commands.rs) ──
    ocean::compute_wind_belts(&mut buf);
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    ocean::compute_sst(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    let sea_freeze = ocean::compute_shelf_freeze(&buf);
    ocean::reinforce_cold_shelf_currents(&mut buf, &sea_freeze);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    ocean::apply_cold_shelf_cooling(&mut buf, &sea_freeze);
    temperature::compute_seasonal_amplitude(&mut buf);
    temperature::apply_ice_albedo_feedback(&mut buf);
    jets::compute_low_level_jets(&mut buf);
    precipitation::compute_precipitation(&mut buf);
    koppen::classify_koppen(&mut buf);

    (buf, reference)
}

/// Area-weighted main-class + exact-zone agreement of the generated Köppen map
/// against the real Earth reference. Prints a scorecard and asserts a floor so a
/// change that breaks the global climate pattern fails.
#[test]
fn earth_koppen_agreement() {
    let (buf, reference) = run_earth();

    let mut w_total = 0.0f64;
    let mut w_main = 0.0f64; // area-weighted main-class matches
    let mut w_exact = 0.0f64; // area-weighted exact-zone matches
    // Per-reference-main-class agreement, to see WHERE it agrees/disagrees.
    let mut per_ref: std::collections::BTreeMap<u8, (f64, f64)> = std::collections::BTreeMap::new();
    // Confusion: reference main letter → generated main letter (area weight).
    let letters = [b'A', b'B', b'C', b'D', b'E'];
    let li = |l: u8| letters.iter().position(|&c| c == l).unwrap_or(5);
    let mut confusion = [[0.0f64; 6]; 5]; // [ref][gen], col 5 = H/other

    for y in 0..H {
        let lat = buf.latitude(y as u32);
        let wt = (lat.to_radians().cos() as f64).max(0.0); // area weight
        for x in 0..W {
            let i = y * W + x;
            let rc = reference[i];
            if rc == 0 { continue; } // ocean / no reference
            let gc = buf.koppen[i];
            let rl = main_letter(rc);
            let gl = main_letter(gc);
            w_total += wt;
            let m = if rl == gl { 1.0 } else { 0.0 };
            let e = if rc == gc { 1.0 } else { 0.0 };
            w_main += wt * m;
            w_exact += wt * e;
            let ent = per_ref.entry(rl).or_insert((0.0, 0.0));
            ent.0 += wt * m;
            ent.1 += wt;
            if li(rl) < 5 { confusion[li(rl)][li(gl).min(5)] += wt; }
        }
    }

    let main_pct = 100.0 * w_main / w_total;
    let exact_pct = 100.0 * w_exact / w_total;
    println!("\n═══ Earth Köppen validation ({W}×{H}, area-weighted) ═══");
    println!("  main-class agreement : {main_pct:.1}%  (A/B/C/D/E)");
    println!("  exact-zone agreement : {exact_pct:.1}%");
    println!("  by reference main class:");
    for (letter, (hit, tot)) in &per_ref {
        println!("    {}: {:.1}%  (weight {:.0})", *letter as char, 100.0 * hit / tot, tot);
    }
    println!("  confusion  ref↓ gen→   A     B     C     D     E     H/·");
    for (r, row) in confusion.iter().enumerate() {
        let tot: f64 = row.iter().sum();
        if tot <= 0.0 { continue; }
        print!("    {}          ", letters[r] as char);
        for v in row.iter() {
            print!("{:5.0}% ", 100.0 * v / tot);
        }
        println!();
    }
    println!("════════════════════════════════════════════════════════\n");

    // Regression floor — set just under the measured baseline. A change that drops
    // the global agreement below this breaks the build. Raise it as accuracy improves.
    assert!(
        main_pct >= EARTH_MAIN_FLOOR,
        "Earth main-class Köppen agreement {main_pct:.1}% fell below the {EARTH_MAIN_FLOOR:.1}% \
         regression floor — a change has degraded the climate model's fidelity to Earth."
    );
}

/// The regression floor for area-weighted main-class agreement. Calibrated just
/// under the measured baseline (66.3% at 0.5° after FIX_PLAN A1 — conserved
/// moisture-recycling + the delta-mouth monsoon-onshore fix); bump it up as the
/// model improves so it always guards the current fidelity.
const EARTH_MAIN_FLOOR: f64 = 65.0;

/// Why a subtropical cell came out wet or dry — the decision chain, not the total.
///
/// The aridity of a subtropical land cell is decided by Pass 1b of
/// `precipitation.rs`: `sink_frac = 0.75 · subtropical_penalty(lat) ·
/// (1 − monsoon_onshore)^1.5`. Two large opposing effects hang off the single
/// `monsoon_onshore` ray test — it both switches the subsidence sink off AND
/// switches the additive `monsoon_bonus` (≤1150 mm) on. A cell that reads wet
/// when it should read desert is almost always this one number.
///
/// Prints `onshore` / `sub` / `sink` beside the resulting precipitation and the
/// `current_type` of the nearest water, so the Horn-of-Africa and Arabian cases
/// can be read directly rather than inferred. `#[ignore]`d — a diagnostic, in the
/// same spirit as `econ_diagnose_house_turnover`.
#[test]
#[ignore]
fn earth_diagnose_subtropical_aridity_chain() {
    use crate::sim::precipitation;
    let (buf, reference) = run_earth();
    let cell = |lat: f32, lon: f32| -> (u32, u32) {
        let y = ((90.0 - lat) * 2.0).round() as u32;
        let x = ((lon + 180.0) * 2.0).round() as u32;
        (x.min(W as u32 - 1), y.min(H as u32 - 1))
    };
    // Nearest water cell's current_type, searched outward on the 4 axes.
    let near_water_type = |x: u32, y: u32| -> i32 {
        for r in 1..=20i32 {
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nx = buf.wrap_x(x as i32 + dx * r);
                let ny = y as i32 + dy * r;
                if ny < 0 || ny >= H as i32 { continue; }
                let ni = buf.idx(nx, ny as u32);
                if buf.terrain[ni] == 0 { return buf.current_type[ni] as i32; }
            }
        }
        -1
    };
    let sites: [(&str, f32, f32, f32); 12] = [
        // (name, lat, lon, real annual precip mm — for the error column)
        ("Somalia-Bosaso 11N49E", 11.0, 49.0, 100.0),
        ("Somalia-Hargeisa 9N44E", 9.0, 44.0, 400.0),
        ("Somalia-Mogadishu 2N45E", 2.0, 45.0, 430.0),
        ("Yemen-Aden 13N45E", 13.0, 45.0, 40.0),
        ("Oman-Salalah 17N54E", 17.0, 54.0, 100.0),
        ("Arabia-Riyadh 25N47E", 25.0, 47.0, 110.0),
        ("Arabia-interior 22N47E", 22.0, 47.0, 60.0),
        // Controls: the monsoon that SHOULD fire, and deserts that should not.
        ("India-Mumbai 19N73E", 19.0, 73.0, 2200.0),
        ("Sahara 23N13E", 23.0, 13.0, 20.0),
        ("Namib 23S15E", -23.0, 15.0, 20.0),
        // Mid-latitude interiors (the C→B thread).
        ("US-Kansas 39N98W", 39.0, -98.0, 650.0),
        ("SE-US 34N84W", 34.0, -84.0, 1300.0),
    ];
    println!("\n─── Subtropical aridity decision chain ───");
    println!(
        "  {:24} {:>3} {:>3} {:>6} {:>6} {:>6} {:>7} {:>7} {:>5}",
        "site", "gen", "ref", "onshor", "sub", "sink", "precip", "real", "cur"
    );
    for (nm, la, lo, real) in sites {
        let (x, y) = cell(la, lo);
        let i = buf.idx(x, y);
        let (onshore, sub, sink) = precipitation::diagnose_subtropical_chain(&buf, x, y);
        println!(
            "  {:24} {:>3} {:>3} {:6.2} {:6.2} {:6.2} {:7.0} {:7.0} {:5}",
            nm,
            main_letter(buf.koppen[i]) as char,
            main_letter(reference[i]) as char,
            onshore, sub, sink,
            buf.precipitation[i], real,
            near_water_type(x, y),
        );
    }
    println!("  (cur: 0=neutral 1=warm 2=cold, -1=no water within 20 cells)");
    println!("──────────────────────────────────────────\n");
}

/// Named-region spot checks — regional regression protection for the archetypal
/// climates a player would immediately judge (the deserts, the equatorial
/// rainforests, the monsoon belt). Prints a per-site table (gen vs real Köppen +
/// summer-precip fraction) and asserts the UNAMBIGUOUS facts so a change that turns
/// the Sahara green or the Amazon into steppe fails the build.
///
/// Known-soft regions NOT yet asserted (the current tuning frontier — see the
/// module report): the wet monsoon subtropics (Bangladesh, S China, SE-US) still
/// come out too dry because the onshore-monsoon *detection* under-fires there, so
/// they read arid. Fixing that is the next accuracy step; until then they are
/// printed but not gated.
#[test]
fn earth_named_region_spot_checks() {
    let (buf, reference) = run_earth();
    let cell = |lat: f32, lon: f32| -> usize {
        let y = ((90.0 - lat) * 2.0).round() as usize;
        let x = ((lon + 180.0) * 2.0).round() as usize;
        (y.min(H - 1)) * W + x.min(W - 1)
    };
    // (name, lat, lon, expected main class or 0 = print-only)
    let sites: [(&str, f32, f32, u8); 15] = [
        ("Sahara 23N13E", 23.0, 13.0, b'B'),
        ("Arabia 22N47E", 22.0, 47.0, b'B'),
        ("Amazon 3S60W", -3.0, -60.0, b'A'),
        ("Congo 0N20E", 0.0, 20.0, b'A'),
        ("Indonesia 0N114E", 0.0, 114.0, b'A'),
        ("SEAsia-Vietnam 11N106E", 11.0, 106.0, b'A'),
        // Printed but not yet gated (too-dry monsoon subtropics — see doc comment):
        ("India-Mumbai 19N73E", 19.0, 73.0, 0),
        ("Bangladesh 24N90E", 24.0, 90.0, 0),
        ("China-South 25N113E", 25.0, 113.0, 0),
        ("SE-US 34N84W", 34.0, -84.0, 0),
        ("NWEurope 52N5E", 52.0, 5.0, 0),
        ("Med-Rome 42N12E", 42.0, 12.0, 0),
        ("Somalia-Mogadishu 2N45E", 2.0, 45.0, 0),
        ("Somalia-Hargeisa 9N44E", 9.0, 44.0, 0),
        ("Somalia-Bosaso 11N49E", 11.0, 49.0, 0),
    ];
    println!("\n─── Earth named-region spot checks ───");
    for (nm, la, lo, want) in sites {
        let i = cell(la, lo);
        let gl = main_letter(buf.koppen[i]);
        let rl = main_letter(reference[i]);
        let sf = if buf.precip_summer_frac.is_empty() { 0 } else { buf.precip_summer_frac[i] };
        println!(
            "  {:24} gen={} ref={} precip={:5.0}mm summer={:3.0}% T={:5.1}°",
            nm, gl as char, rl as char, buf.precipitation[i], sf as f32 / 2.55, buf.temperature[i]
        );
        if want != 0 {
            assert_eq!(gl, want, "{nm}: expected main class {}, got {}", want as char, gl as char);
        }
    }
    println!("──────────────────────────────────────\n");
}

// ─────────────────────────────────────────────────────────────────────────────
// SCRATCH DIAGNOSTICS (advisory session; not intended to be committed)
// ─────────────────────────────────────────────────────────────────────────────

/// Where in geography do the two dominant confusions live?
#[test]
#[ignore]
fn scratch_error_geography() {
    let (buf, reference) = run_earth();
    // Land "basin position": scan W and E along the row to the first ocean.
    let cap = 200i32;
    let side = |x: u32, y: u32| -> f32 {
        let mut dw = cap; let mut de = cap;
        for d in 1..=cap {
            let i = buf.idx(buf.wrap_x(x as i32 - d), y);
            if buf.terrain[i] == 0 { dw = d; break; }
        }
        for d in 1..=cap {
            let i = buf.idx(buf.wrap_x(x as i32 + d), y);
            if buf.terrain[i] == 0 { de = d; break; }
        }
        (dw - de) as f32 / (dw + de).max(1) as f32
    };

    // ── C-row: split by latitude band × coast side ──
    println!("\n─── ref=C cells: how the model classifies them, by lat band × coast side ───");
    println!("  {:10} {:10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7} {:>7}",
        "latband", "side", "n", "%A", "%B", "%C", "%D/E", "precip", "bthresh");
    let bands: [(f32, f32); 5] = [(15.0,25.0),(25.0,35.0),(35.0,45.0),(45.0,55.0),(55.0,70.0)];
    let sides: [(&str, f32, f32); 3] = [("west", -1.01, -0.34), ("mid", -0.34, 0.34), ("east", 0.34, 1.01)];
    for (lo, hi) in bands {
        for (sname, slo, shi) in sides {
            let (mut n, mut a, mut b, mut c, mut de_) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
            let (mut psum, mut tsum) = (0.0f64, 0.0f64);
            for y in 0..H { for x in 0..W {
                let i = y*W+x;
                if reference[i] == 0 { continue; }
                if main_letter(reference[i]) != b'C' { continue; }
                let al = buf.latitude(y as u32).abs();
                if al < lo || al >= hi { continue; }
                let s = side(x as u32, y as u32);
                if s < slo || s >= shi { continue; }
                let wt = (buf.latitude(y as u32).to_radians().cos() as f64).max(0.0);
                n += wt;
                match main_letter(buf.koppen[i]) {
                    b'A' => a += wt, b'B' => b += wt, b'C' => c += wt, _ => de_ += wt,
                }
                psum += wt * buf.precipitation[i] as f64;
                // Köppen aridity threshold with the neutral (f) constant, for scale.
                tsum += wt * (20.0 * buf.temperature[i] as f64 + 140.0);
            }}
            if n < 30.0 { continue; }
            println!("  {:>4.0}-{:<5.0} {:10} {:6.0} {:6.0} {:6.0} {:6.0} {:6.0} {:7.0} {:7.0}",
                lo, hi, sname, n, 100.0*a/n, 100.0*b/n, 100.0*c/n, 100.0*de_/n, psum/n, tsum/n);
        }
    }

    // ── B-row: same split, to see whether real deserts are under-dried ──
    println!("\n─── ref=B cells, by lat band × coast side ───");
    println!("  {:10} {:10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>7}",
        "latband", "side", "n", "%A", "%B", "%C", "%D/E", "precip");
    for (lo, hi) in bands {
        for (sname, slo, shi) in sides {
            let (mut n, mut a, mut b, mut c, mut de_) = (0.0f64, 0.0, 0.0, 0.0, 0.0);
            let mut psum = 0.0f64;
            for y in 0..H { for x in 0..W {
                let i = y*W+x;
                if reference[i] == 0 || main_letter(reference[i]) != b'B' { continue; }
                let al = buf.latitude(y as u32).abs();
                if al < lo || al >= hi { continue; }
                let s = side(x as u32, y as u32);
                if s < slo || s >= shi { continue; }
                let wt = (buf.latitude(y as u32).to_radians().cos() as f64).max(0.0);
                n += wt;
                match main_letter(buf.koppen[i]) {
                    b'A' => a += wt, b'B' => b += wt, b'C' => c += wt, _ => de_ += wt,
                }
                psum += wt * buf.precipitation[i] as f64;
            }}
            if n < 30.0 { continue; }
            println!("  {:>4.0}-{:<5.0} {:10} {:6.0} {:6.0} {:6.0} {:6.0} {:6.0} {:7.0}",
                lo, hi, sname, n, 100.0*a/n, 100.0*b/n, 100.0*c/n, 100.0*de_/n, psum/n);
        }
    }

    // ── D→E: is it temperature or is it the warmest-month reconstruction? ──
    println!("\n─── ref=D cells: annual T, seasonal span, warmest/coldest month ───");
    for (lo, hi) in [(40.0f32,50.0f32),(50.0,60.0),(60.0,70.0)] {
        let (mut n, mut e, mut tt, mut sp, mut wm, mut cm) = (0.0f64,0.0f64,0.0,0.0,0.0,0.0);
        for y in 0..H { for x in 0..W {
            let i = y*W+x;
            if reference[i] == 0 || main_letter(reference[i]) != b'D' { continue; }
            let al = buf.latitude(y as u32).abs();
            if al < lo || al >= hi { continue; }
            let wt = (buf.latitude(y as u32).to_radians().cos() as f64).max(0.0);
            n += wt;
            if main_letter(buf.koppen[i]) == b'E' { e += wt; }
            let (cold, warm) = crate::sim::koppen::seasonal_temps(&buf, x as u32, y as u32);
            tt += wt * buf.temperature[i] as f64;
            sp += wt * (warm - cold) as f64;
            wm += wt * warm as f64;
            cm += wt * cold as f64;
        }}
        if n < 30.0 { continue; }
        println!("  {:>3.0}-{:<3.0}  n={:6.0}  ->E {:4.0}%   T={:5.1}  span={:5.1}  warmest={:5.1}  coldest={:6.1}",
            lo, hi, n, 100.0*e/n, tt/n, sp/n, wm/n, cm/n);
    }

    // ── Ocean: NECC and ACC ──
    let (mut ne, mut nn) = (0.0f64, 0.0f64);
    for y in 0..H { for x in 0..W {
        let i = y*W+x;
        if buf.terrain[i] != 0 { continue; }
        let al = buf.latitude(y as u32).abs();
        if !(3.0..=10.0).contains(&al) { continue; }
        nn += 1.0; if buf.current_vx[i] > 0.05 { ne += 1.0; }
    }}
    println!("\n  NECC/SECC band 3-10 deg: {:.0}% of ocean cells flow EASTWARD ({:.0} cells)", 100.0*ne/nn, nn);
    let (mut acc_vx, mut acc_n) = (0.0f64, 0.0f64);
    for y in 0..H { for x in 0..W {
        let i = y*W+x;
        if buf.terrain[i] != 0 { continue; }
        let l = buf.latitude(y as u32);
        if !(-65.0..=-50.0).contains(&l) { continue; }
        acc_vx += buf.current_vx[i] as f64; acc_n += 1.0;
    }}
    println!("  ACC band 50-65S: mean vx = {:+.3} (positive = eastward, correct)", acc_vx/acc_n);

    // ── Belt-wind magnitude near the belt boundaries ──
    println!("\n  belt_wind |v| by latitude (unit expected):");
    let circ = crate::sim::circulation::Circulation::earth();
    for l in [28.0f32, 29.0, 29.5, 30.0, 30.5, 31.0, 59.5, 60.0, 60.5] {
        let (vx, vy) = crate::sim::ocean::belt_wind(l, &circ);
        println!("    {:5.1} deg  |v| = {:.3}", l, (vx*vx+vy*vy).sqrt());
    }

    // ── Ocean moisture emission is blind to SST: show the SST range being ignored ──
    let mut by_lat: Vec<(f32, f32, f32)> = Vec::new();
    for tgt in [0.0f32, 15.0, 25.0, 35.0, 45.0, 55.0] {
        let (mut s, mut n2) = (0.0f64, 0.0f64);
        for y in 0..H { for x in 0..W {
            let i = y*W+x;
            if buf.terrain[i] != 0 { continue; }
            if (buf.latitude(y as u32).abs() - tgt).abs() > 1.0 { continue; }
            s += buf.sst[i] as f64; n2 += 1.0;
        }}
        if n2 > 0.0 {
            let sst = (s/n2) as f32;
            // Saturation specific humidity ratio vs 15 C (Tetens), the bulk-formula driver.
            let es = |t: f32| 6.112 * (17.67*t/(t+243.5)).exp();
            by_lat.push((tgt, sst, es(sst)/es(15.0)));
        }
    }
    println!("\n  mean SST vs the moisture the model actually emits (init_moisture is 1.0 everywhere neutral):");
    for (l, sst, r) in by_lat {
        println!("    {:4.0} deg  SST {:5.1} C   e_sat ratio vs 15C = {:4.2}x", l, sst, r);
    }
    println!();
}
