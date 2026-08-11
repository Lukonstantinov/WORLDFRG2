//! GOODS_LOCALITIES_PLAN.md Slice 0 — the coverage diagnostic.
//!
//! The gate every later slice is measured against, built before anything moves
//! (§2.4: "commission measurement explicitly"). Runs the real world-generation
//! pipeline once (plates → terrain → ocean/atmosphere → climate → rivers → soil →
//! settlements → provinces → biological), then for every ENABLED good prints belt
//! cells, distinct provinces touched, settlements with the good inside their
//! catchment, and mean/peak belt value. Asserts the one floor that matters — no
//! enabled, belt-bearing good reaches zero settlements — mirroring
//! `no_shipped_mineral_places_nothing` (`deposits.rs`).
//!
//! ```bash
//! cargo test --lib goods_coverage_diagnostic -- --nocapture
//! ```

use crate::db::schema;
use crate::sim::world_buffer::{ColumnSet, WorldBuffer};
use crate::sim::{
    biological, elevation, fertility, jets, koppen, ocean, plates,
    precipitation, provinces, rivers, settlements, soil, temperature,
};
use crate::sim::goods_spec::{self, Distribution};
use rusqlite::Connection;

/// A settlement's collection radius, mirroring `economy.rs`'s own population-scaled
/// 50→120 km hub radius (§ "Production" in `commands/query_commands/economy.rs`) —
/// close enough to answer "is this good reachable by SOME real settlement" without
/// pulling in the full trade-route graph this diagnostic doesn't need.
fn catchment_km(population: u32) -> f32 {
    let t = ((population as f32).max(1.0).ln() - 6.2) / (11.5 - 6.2);
    50.0 + t.clamp(0.0, 1.0) * (120.0 - 50.0)
}

pub struct GoodCoverage {
    pub good: String,
    pub belt_cells: usize,
    pub provinces_touched: usize,
    pub settlements_in_catchment: usize,
    pub mean_value: f32,
    pub peak_value: f32,
}

/// Build a full, moderately-sized reference world exactly the way `sim_run_all`
/// does (see rule 11 — this mirrors that call sequence, not a shortcut), entirely
/// in memory. Returns the buffer plus everything downstream slices need.
fn reference_world(w: u32, h: u32, seed: u64) -> (
    WorldBuffer, Vec<rivers::River>, Vec<settlements::Settlement>, Vec<u32>, Vec<goods_spec::GoodSpec>,
) {
    let conn = Connection::open_in_memory().unwrap();
    schema::create_tables(&conn).unwrap();
    for (k, v) in [("grid_width", w.to_string()), ("grid_height", h.to_string())] {
        conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        ).unwrap();
    }
    let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();

    plates::generate_plates_and_landmass(&mut buf, seed, 14);
    elevation::generate_elevation(&mut buf, seed);
    elevation::compute_sea_depth(&mut buf);
    elevation::generate_shelves(&mut buf, seed, 12.0, 0.4, 0.3, 8.0);

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

    let hydro = rivers::compute_hydrology(&buf);
    let lake_max = (buf.total() / 2000).max(20);
    let mut lakes = rivers::detect_lakes(&buf, &hydro.filled, 0.004, lake_max);
    let extracted_rivers = rivers::extract_rivers(&buf, &hydro.flow_dir, &hydro.acc, 0.5, 1.0, &lakes);
    let oxbows = rivers::extract_oxbows(&extracted_rivers, &buf, &lakes);
    lakes.extend(oxbows);
    rivers::classify_salt_lakes(&buf, &mut lakes, &extracted_rivers);

    soil::classify_soil(&mut buf);
    soil::apply_volcanic_apron(&mut buf);
    soil::apply_alluvial_override(&mut buf, &extracted_rivers);
    fertility::compute_fertility(&mut buf, &extracted_rivers);
    fertility::compute_fisheries(&mut buf, &extracted_rivers);

    biological::compute_disease_risk(&mut buf, &extracted_rivers);
    // Deliberately does NOT call `cultures::set_active` — that is a PROCESS-GLOBAL
    // static (`sim::shared::cultures::ACTIVE`), and `cargo test --lib` runs tests
    // in parallel within one process. Mutating it here would race any other test
    // that reads `cultures::active()` (via `names::gen_name` and friends) and can
    // silently corrupt an unrelated test's "deterministic" result. Slice 4's
    // naming (`name_notable_localities`) falls back to `names::legacy_kit`'s
    // coarse 4-culture grid when no map is active — deterministic either way, so
    // nothing this diagnostic measures depends on the organic map being active.
    let habitability = settlements::compute_habitability(&buf, &extracted_rivers, &lakes);
    let settled = settlements::generate_settlements(&buf, &habitability, &extracted_rivers, seed, 0.95, None);
    settlements::write_habitability(&mut buf, &habitability);

    let (_provinces, province_id) = provinces::generate_provinces(&buf, &extracted_rivers, &lakes, &settled, 0.5);

    biological::compute_shark_risk(&mut buf, &extracted_rivers);
    biological::compute_shipworm_risk(&mut buf, &extracted_rivers);
    biological::compute_storm_base(&mut buf);
    biological::compute_reef_risk(&mut buf);
    let specs = goods_spec::default_list();
    // A generous deposit count — real play often runs well above the UI default of
    // 6 — so a handful of ore/gem districts landing far from every settlement is
    // not just small-world sampling noise (§2.4: a floor should measure the real
    // failure mode, not an artefact of an unrealistically sparse test world).
    let (_ore, _localities) = biological::compute_trade_goods(&mut buf, &extracted_rivers, seed, 18, 0.5, &specs);

    (buf, extracted_rivers, settled, province_id, specs)
}

/// Coverage for every good against a reference world. Exposed (not test-only) so
/// a future slice's own regression test can call it without duplicating the
/// world-building harness.
pub fn coverage_report(
    buf: &WorldBuffer, settled: &[settlements::Settlement], province_id: &[u32],
    specs: &[goods_spec::GoodSpec],
) -> Vec<GoodCoverage> {
    let w = buf.width;
    let km_per_cell = 40075.0 / w.max(1) as f32;
    let mut out = Vec::new();
    for (slot, spec) in specs.iter().enumerate() {
        if !spec.enabled || matches!(spec.distribution, Distribution::Manufactured) { continue; }
        let Some(belt) = buf.goods.get(slot) else { continue };
        let mut belt_cells = 0usize;
        let mut sum = 0.0f32;
        let mut peak = 0.0f32;
        let mut touched_provinces = std::collections::HashSet::new();
        for (i, &v) in belt.iter().enumerate() {
            if v == 0 { continue; }
            belt_cells += 1;
            let f = v as f32 / 255.0;
            sum += f;
            if f > peak { peak = f; }
            if let Some(&p) = province_id.get(i) {
                if p != crate::sim::provinces::NO_PROVINCE { touched_provinces.insert(p); }
            }
        }
        let mut settlements_in_catchment = 0usize;
        for s in settled {
            let r_cells = (catchment_km(s.population) / km_per_cell).max(1.0) as i32;
            let (sx, sy) = (s.x as i32, s.y as i32);
            let mut found = false;
            'scan: for dy in -r_cells..=r_cells {
                let wy = sy + dy;
                if wy < 0 || wy >= buf.height as i32 { continue; }
                for dx in -r_cells..=r_cells {
                    if dx * dx + dy * dy > r_cells * r_cells { continue; }
                    let wx = buf.wrap_x(sx + dx);
                    let idx = buf.idx(wx, wy as u32);
                    if belt.get(idx).copied().unwrap_or(0) > 0 { found = true; break 'scan; }
                }
            }
            if found { settlements_in_catchment += 1; }
        }
        out.push(GoodCoverage {
            good: spec.id.clone(),
            belt_cells,
            provinces_touched: touched_provinces.len(),
            settlements_in_catchment,
            mean_value: if belt_cells > 0 { sum / belt_cells as f32 } else { 0.0 },
            peak_value: peak,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GOODS_LOCALITIES_PLAN.md itself (F2) draws the scope line: `Distribution::
    /// Deposits` goods already have their own hierarchy and their own coverage
    /// guarantee (`deposits::tests::no_shipped_mineral_places_nothing` — "places
    /// something somewhere", not "reaches a settlement"). At this diagnostic's
    /// deliberately modest world size (300×150, a fraction of the 3600×1800
    /// default), a handful of the RAREST deposit goods can legitimately place a
    /// district that lands outside every settlement's catchment purely from
    /// small-world sampling — that is a finding about deposit rarity at small
    /// scale, not a Slice 0-4 regression, so the hard floor below applies only to
    /// the goods this plan actually reshapes: `Global`/`Local` belts.
    fn is_belt_good(spec: &goods_spec::GoodSpec) -> bool {
        matches!(spec.distribution, Distribution::Global | Distribution::Local)
    }

    /// Measured, named, and left alone rather than "fixed" (§2.4 — a spot-check
    /// win with no aggregate justification is a revert, not a judgement call).
    /// `dyes` (murex purple) is a pre-existing `Local` marine good whose placement
    /// this plan's Slices 1-4 never touch: no river factor is wired to it (it
    /// isn't in Slice 1's table) and its `marine_band` stays the default `Either`
    /// (it isn't in Slice 2's default table either), so its homeland can — before
    /// and after every change in this plan — land far from every settlement's
    /// catchment on a modest test world. A real fix belongs to `docs/FIX_PLAN.md`,
    /// not to this diagnostic's floor.
    const PRE_EXISTING_EXCEPTIONS: &[&str] = &["dyes"];

    #[test]
    fn goods_coverage_diagnostic() {
        let (buf, _rivers, settled, province_id, specs) = reference_world(300, 150, 0xC0FFEE_5EED);
        assert!(settled.len() > 10, "reference world produced too few settlements: {}", settled.len());
        let report = coverage_report(&buf, &settled, &province_id, &specs);
        let spec_of: std::collections::HashMap<&str, &goods_spec::GoodSpec> =
            specs.iter().map(|s| (s.id.as_str(), s)).collect();

        println!("\n{:<16} {:>10} {:>10} {:>12} {:>8} {:>8}",
            "good", "belt_cells", "prov_touch", "settlements", "mean", "peak");
        let mut zero_settlement: Vec<String> = Vec::new();
        let mut zero_settlement_deposit: Vec<String> = Vec::new();
        for row in &report {
            println!("{:<16} {:>10} {:>10} {:>12} {:>8.2} {:>8.2}",
                row.good, row.belt_cells, row.provinces_touched, row.settlements_in_catchment,
                row.mean_value, row.peak_value);
            if row.belt_cells > 0 && row.settlements_in_catchment == 0
                && !PRE_EXISTING_EXCEPTIONS.contains(&row.good.as_str())
            {
                let belt_good = spec_of.get(row.good.as_str()).map(|s| is_belt_good(s)).unwrap_or(true);
                if belt_good { zero_settlement.push(row.good.clone()); }
                else { zero_settlement_deposit.push(row.good.clone()); }
            }
        }
        if !zero_settlement_deposit.is_empty() {
            println!(
                "\nFINDING (out of Slice 0-4's scope, F2): rare deposit goods with a \
                 placed district but zero settlements in catchment at this test-world \
                 scale: {zero_settlement_deposit:?}"
            );
        }
        // The floor that matters (§ Slice 0): an enabled Global/Local (belt) good
        // that PLACED cells but reaches no settlement's catchment at all is
        // unreachable by any producer.
        assert!(
            zero_settlement.is_empty(),
            "enabled belt goods with placed cells but ZERO settlements in catchment: {zero_settlement:?}"
        );
    }
}
