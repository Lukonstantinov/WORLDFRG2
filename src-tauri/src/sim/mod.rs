// ── Shared foundation (referenced by every step) ─────────────────────────────
pub mod world_buffer;

// ── Step 1 · Tectonic Plates ─────────────────────────────────────────────────
pub mod step1_plates;
pub use step1_plates::plates;

// ── Step 2 · Terrain / Elevation ─────────────────────────────────────────────
pub mod step2_terrain;
pub use step2_terrain::elevation;

// ── Step 3 · Ocean & Atmosphere ──────────────────────────────────────────────
//    ocean → temperature → jets → seasonal → precipitation
pub mod step3_ocean_atmo;
pub use step3_ocean_atmo::ocean;
pub use step3_ocean_atmo::insolation;
pub use step3_ocean_atmo::ebm;
pub use step3_ocean_atmo::circulation;
pub use step3_ocean_atmo::temperature;
pub use step3_ocean_atmo::jets;
pub use step3_ocean_atmo::seasonal;
pub use step3_ocean_atmo::precipitation;
pub use step3_ocean_atmo::preview;

// ── Step 4 · Climate Classification (Köppen) ─────────────────────────────────
pub mod step4_climate;
pub use step4_climate::koppen;

// ── Step 5 · Rivers & Hydrology ──────────────────────────────────────────────
pub mod step5_rivers;
pub use step5_rivers::rivers;
pub use step5_rivers::aquatic;

// ── Step 6 · Soil & Fertility ────────────────────────────────────────────────
pub mod step6_soil_fertility;
pub use step6_soil_fertility::soil;
pub use step6_soil_fertility::fertility;
pub use step6_soil_fertility::biome;

// ── Step 7 · Settlements ─────────────────────────────────────────────────────
pub mod step7_settlements;
pub use step7_settlements::settlements;

// ── Step 8 · Biological / Trade Goods ────────────────────────────────────────
pub mod step8_biological_goods;
pub use step8_biological_goods::biological;
pub use step8_biological_goods::deposits;
pub use step8_biological_goods::goods_spec;
pub use step8_biological_goods::localities;

// ── Campaign Simulation ──────────────────────────────────────────────────────
pub mod campaign;
pub use campaign::tick;
pub use campaign::market;
pub use campaign::manufacture;

// ── Shared Utilities (names, cultures, toponyms) ─────────────────────────────
pub mod shared;
pub use shared::names;
pub use shared::cultures;
pub use shared::inheritance;
pub use shared::toponyms;
pub use shared::provinces;
