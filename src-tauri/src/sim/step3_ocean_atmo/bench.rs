//! Phase-3 (Ocean & Atmosphere) timing harness — the perf counterpart to
//! `earth_validation.rs`'s fidelity gate.
//!
//! The step runs on the *world* grid (3600×1800 by default, 7200×3600 on "Large"),
//! not the 720×360 validation fixture, so a cost that is invisible in the fidelity
//! test can dominate the real run. This harness upsamples the Earth landmass to a
//! chosen grid and prints a per-sub-step millisecond breakdown of the exact
//! sequence `sim_commands::sim_ocean_atmosphere` runs.
//!
//! ```bash
//! cargo test --lib --release bench_ocean_atmosphere -- --ignored --nocapture
//! ```
#![cfg(test)]

use crate::db::schema;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{elevation, jets, ocean, precipitation, temperature};
use rusqlite::Connection;
use std::time::Instant;

const SRC_W: usize = 720;
const SRC_H: usize = 360;

static ELEV_I16: &[u8] = include_bytes!("../step4_climate/fixtures/earth_elev_720x360.i16");
static KOPPEN_U8: &[u8] = include_bytes!("../step4_climate/fixtures/earth_koppen_720x360.u8");

/// Build a WorldBuffer of `w`×`h` carrying the Earth landmass (nearest-neighbour
/// upsampled from the 0.5° fixture) with sea depth + shelves already computed —
/// i.e. exactly the state phase 3 is handed by the terrain phase.
fn earth_world(w: u32, h: u32) -> WorldBuffer {
    let conn = Connection::open_in_memory().unwrap();
    schema::create_tables(&conn).unwrap();
    for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        )
        .unwrap();
    }
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();

    for y in 0..h as usize {
        let sy = y * SRC_H / h as usize;
        for x in 0..w as usize {
            let sx = x * SRC_W / w as usize;
            let s = sy * SRC_W + sx;
            let i = y * w as usize + x;
            let land = KOPPEN_U8[s] != 0;
            buf.terrain[i] = if land { 1 } else { 0 };
            let e = i16::from_le_bytes([ELEV_I16[s * 2], ELEV_I16[s * 2 + 1]]) as f32;
            buf.elevation[i] = if land { (e.max(0.0) / 8848.0).clamp(0.0, 1.0) } else { 0.0 };
        }
    }
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, 42, 12.0, 0.4, 0.3, 8.0);
    buf
}

/// Order-independent checksum of an f32 field (bit pattern, so NaN/-0.0 differ too).
fn fsum(v: &[f32]) -> u64 {
    v.iter().enumerate().fold(0u64, |a, (i, x)| {
        a.wrapping_add((x.to_bits() as u64).wrapping_mul(i as u64 + 1))
    })
}
fn usum(v: &[u8]) -> u64 {
    v.iter().enumerate().fold(0u64, |a, (i, x)| a.wrapping_add((*x as u64) * (i as u64 + 1)))
}

/// Print a checksum of every field phase 3 writes, so a refactor that is meant to
/// be behaviour-preserving can be proved so: run this before and after and diff
/// the output. (The Earth fidelity gate in `earth_validation.rs` scores the same
/// pipeline, but only to two decimal places of agreement — it cannot tell a
/// bit-exact change from a merely close one.)
#[test]
#[ignore = "perf harness — run explicitly with --release"]
fn ocean_atmosphere_field_checksums() {
    let mut buf = earth_world(720, 360);
    ocean::compute_wind_belts(&mut buf);
    ocean::compute_salinity(&mut buf);
    ocean::generate_ocean_currents(&mut buf);
    ocean::advect_salinity_and_recouple(&mut buf);
    ocean::compute_sst(&mut buf);
    ocean::compute_distance_to_ocean(&mut buf);
    let freeze = ocean::compute_shelf_freeze(&buf);
    ocean::reinforce_cold_shelf_currents(&mut buf, &freeze);
    temperature::compute_temperature(&mut buf);
    ocean::compute_upwelling_zones(&mut buf);
    ocean::apply_cold_shelf_cooling(&mut buf, &freeze);
    temperature::compute_seasonal_amplitude(&mut buf);
    temperature::apply_ice_albedo_feedback(&mut buf);
    jets::compute_low_level_jets(&mut buf);
    precipitation::compute_precipitation(&mut buf);

    println!("\n=== phase-3 field checksums (720×360) ===");
    println!("  wind_vx           {:020}", fsum(&buf.wind_vx));
    println!("  wind_vy           {:020}", fsum(&buf.wind_vy));
    println!("  wind_speed        {:020}", fsum(&buf.wind_speed));
    println!("  current_vx        {:020}", fsum(&buf.current_vx));
    println!("  current_vy        {:020}", fsum(&buf.current_vy));
    println!("  current_type      {:020}", usum(&buf.current_type));
    println!("  salinity          {:020}", usum(&buf.salinity));
    println!("  sst               {:020}", fsum(&buf.sst));
    println!("  temperature       {:020}", fsum(&buf.temperature));
    println!("  precipitation     {:020}", fsum(&buf.precipitation));
    println!("  precip_summer     {:020}", usum(&buf.precip_summer_frac));
    println!("  seasonal_amp      {:020}", usum(&buf.seasonal_amp));
    println!("  distance_to_ocean {:020}", fsum(&buf.distance_to_ocean));
    println!("  shelf_freeze      {:020}", fsum(&freeze));
}

/// Time every sub-step of the phase-3 sequence and print a sorted breakdown.
#[test]
#[ignore = "perf harness — run explicitly with --release"]
fn bench_ocean_atmosphere() {
    for (w, h) in [(1800u32, 900u32), (3600, 1800)] {
        let mut buf = earth_world(w, h);
        let mut rows: Vec<(&str, f64)> = Vec::new();
        let mut time = |name: &'static str, rows: &mut Vec<(&str, f64)>, f: &mut dyn FnMut()| {
            let t = Instant::now();
            f();
            rows.push((name, t.elapsed().as_secs_f64() * 1000.0));
        };

        let total = Instant::now();
        time("wind_belts", &mut rows, &mut || ocean::compute_wind_belts(&mut buf));
        time("salinity", &mut rows, &mut || ocean::compute_salinity(&mut buf));
        time("currents", &mut rows, &mut || ocean::generate_ocean_currents(&mut buf));
        time("advect_salinity", &mut rows, &mut || ocean::advect_salinity_and_recouple(&mut buf));
        time("sst", &mut rows, &mut || ocean::compute_sst(&mut buf));
        time("distance_to_ocean", &mut rows, &mut || ocean::compute_distance_to_ocean(&mut buf));
        let mut freeze = Vec::new();
        time("shelf_freeze", &mut rows, &mut || freeze = ocean::compute_shelf_freeze(&buf));
        time("cold_shelf_currents", &mut rows, &mut || {
            ocean::reinforce_cold_shelf_currents(&mut buf, &freeze)
        });
        time("temperature", &mut rows, &mut || temperature::compute_temperature(&mut buf));
        time("upwelling", &mut rows, &mut || ocean::compute_upwelling_zones(&mut buf));
        time("cold_shelf_cooling", &mut rows, &mut || {
            ocean::apply_cold_shelf_cooling(&mut buf, &freeze)
        });
        time("seasonal_amplitude", &mut rows, &mut || {
            temperature::compute_seasonal_amplitude(&mut buf)
        });
        time("ice_albedo", &mut rows, &mut || temperature::apply_ice_albedo_feedback(&mut buf));
        time("low_level_jets", &mut rows, &mut || jets::compute_low_level_jets(&mut buf));
        time("precipitation", &mut rows, &mut || precipitation::compute_precipitation(&mut buf));
        let total = total.elapsed().as_secs_f64() * 1000.0;

        println!("\n=== Ocean & Atmosphere @ {}×{} — {:.0} ms total ===", w, h, total);
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (name, ms) in &rows {
            println!("  {:>9.1} ms  {:>5.1}%  {}", ms, ms / total * 100.0, name);
        }
    }
}
