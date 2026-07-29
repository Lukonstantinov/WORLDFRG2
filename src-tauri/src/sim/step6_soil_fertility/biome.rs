//! Phase 6b — ecological biome classification.
//!
//! Köppen answers "what is the *climate* here"; it does not answer "what
//! *grows* here". Two cells can share `BSk` and be shortgrass prairie in one
//! place and a stony cold desert in another; an `Aw` cell is savanna on the
//! interfluve and closed gallery forest 400 m away on the riverbank. This phase
//! turns the climate stack into an actual vegetation/ecosystem map.
//!
//! ## The three axes
//!
//! 1. **Biotemperature** (Holdridge): the mean of the monthly temperatures with
//!    each month clamped to 0–30 °C, i.e. the heat that is actually available to
//!    plants. Frozen months contribute nothing, and heat above 30 °C buys no
//!    extra growth. Derived here from the annual mean plus `seasonal_amp`
//!    (the energy-balance seasonal half-range written by `temperature.rs`).
//! 2. **PET ratio** (Holdridge): `58.93 · biotemperature / annual precip` — the
//!    ratio of potential evapotranspiration to rainfall, which is the honest
//!    moisture axis. 1.0 is the humid/dry hinge; > 2 is semi-arid, > 4 arid.
//!    This is why a cold 300 mm cell is a *steppe* while a hot 300 mm cell is a
//!    *desert* without any special-casing.
//! 3. **Seasonality**, taken from Köppen's own letters (`s`/`w`/`f` via the zone
//!    code) plus `precip_summer_frac` — what separates a Mediterranean
//!    sclerophyll woodland from an evergreen temperate forest at identical
//!    totals.
//!
//! ## Order of precedence (each overrides the ones below it)
//!
//! ```text
//!   1. Cryospheric  — permanent ice / nival rock (nothing grows, full stop)
//!   2. Azonal water — mangrove, marsh, swamp, floodplain, gallery forest, oasis,
//!                     bog, salt flat. LOCAL water beats regional climate: this is
//!                     the layer that makes rivers and lakes visible in the biomes.
//!   3. Altitudinal  — above the *climatic* treeline (warmest month < ~6.5 °C),
//!                     which moves with latitude instead of a fixed elevation.
//!   4. Zonal        — the Holdridge/Köppen life zone for the cell's own climate.
//! ```
//!
//! Everything here is **descriptive**: no later phase scores off `biome`. Goods,
//! fertility, habitability and settlement placement are untouched, so adding
//! this phase cannot move a single city or trade belt.

use crate::sim::koppen::*;
use crate::sim::rivers::{Lake, River};
use crate::sim::soil::*;
use crate::sim::world_buffer::WorldBuffer;
use rayon::prelude::*;
use std::collections::VecDeque;

// ─────────────────────────── Biome codes ───────────────────────────
//
// Stored in the `biome` u8 tile column. 0 = unclassified (sea, or a world whose
// save predates this phase). Codes are STABLE — append new biomes at the end,
// never renumber, or existing saves change meaning.

pub const B_NONE: u8 = 0;

// ── Tropical forest ──
pub const B_TROPICAL_RAINFOREST: u8 = 1;
pub const B_TROPICAL_MOIST_FOREST: u8 = 2;
pub const B_TROPICAL_SEASONAL_FOREST: u8 = 3;
pub const B_CLOUD_FOREST: u8 = 4;
pub const B_MANGROVE: u8 = 5;
// ── Tropical open ──
pub const B_SAVANNA: u8 = 6;
pub const B_THORN_SCRUB: u8 = 7;
// ── Temperate forest ──
pub const B_TEMPERATE_RAINFOREST: u8 = 8;
pub const B_TEMPERATE_DECIDUOUS: u8 = 9;
pub const B_TEMPERATE_MIXED_FOREST: u8 = 10;
pub const B_TEMPERATE_CONIFER: u8 = 11;
pub const B_SUBTROPICAL_MOIST_FOREST: u8 = 12;
// ── Mediterranean ──
pub const B_MEDITERRANEAN_WOODLAND: u8 = 13;
pub const B_CHAPARRAL: u8 = 14;
// ── Boreal ──
pub const B_TAIGA: u8 = 15;
pub const B_FOREST_TUNDRA: u8 = 16;
// ── Grassland ──
pub const B_TALLGRASS_PRAIRIE: u8 = 17;
pub const B_SHORTGRASS_STEPPE: u8 = 18;
pub const B_MONTANE_GRASSLAND: u8 = 19;
// ── Alpine ──
pub const B_ALPINE_MEADOW: u8 = 20;
pub const B_ALPINE_TUNDRA: u8 = 21;
pub const B_ALPINE_DESERT: u8 = 22;
pub const B_NIVAL: u8 = 23;
// ── Desert ──
pub const B_HOT_DESERT: u8 = 24;
pub const B_COASTAL_FOG_DESERT: u8 = 25;
pub const B_COLD_DESERT: u8 = 26;
pub const B_SEMI_DESERT: u8 = 27;
pub const B_SALT_FLAT: u8 = 28;
pub const B_DUNE_SEA: u8 = 29;
pub const B_ROCKY_DESERT: u8 = 30;
// ── Polar ──
pub const B_TUNDRA: u8 = 31;
pub const B_POLAR_DESERT: u8 = 32;
pub const B_ICE_SHEET: u8 = 33;
pub const B_GLACIER: u8 = 34;
// ── Azonal / wetland (river-, lake- and coast-driven) ──
pub const B_PEAT_BOG: u8 = 35;
pub const B_GALLERY_FOREST: u8 = 36;
pub const B_FLOODPLAIN_GRASSLAND: u8 = 37;
pub const B_FRESHWATER_MARSH: u8 = 38;
pub const B_SWAMP_FOREST: u8 = 39;
pub const B_SALT_MARSH: u8 = 40;
pub const B_OASIS: u8 = 41;

/// Total number of biome codes including `B_NONE`, for LUT sizing.
pub const BIOME_COUNT: usize = 42;

/// Human-readable name for a biome code.
pub fn biome_name(b: u8) -> &'static str {
    match b {
        B_TROPICAL_RAINFOREST => "Tropical Rainforest",
        B_TROPICAL_MOIST_FOREST => "Tropical Moist Forest",
        B_TROPICAL_SEASONAL_FOREST => "Tropical Dry Forest",
        B_CLOUD_FOREST => "Montane Cloud Forest",
        B_MANGROVE => "Mangrove Swamp",
        B_SAVANNA => "Savanna",
        B_THORN_SCRUB => "Thorn Scrub",
        B_TEMPERATE_RAINFOREST => "Temperate Rainforest",
        B_TEMPERATE_DECIDUOUS => "Deciduous Forest",
        B_TEMPERATE_MIXED_FOREST => "Mixed Forest",
        B_TEMPERATE_CONIFER => "Coniferous Forest",
        B_SUBTROPICAL_MOIST_FOREST => "Subtropical Forest",
        B_MEDITERRANEAN_WOODLAND => "Mediterranean Woodland",
        B_CHAPARRAL => "Chaparral",
        B_TAIGA => "Taiga",
        B_FOREST_TUNDRA => "Forest-Tundra",
        B_TALLGRASS_PRAIRIE => "Tallgrass Prairie",
        B_SHORTGRASS_STEPPE => "Shortgrass Steppe",
        B_MONTANE_GRASSLAND => "Montane Grassland",
        B_ALPINE_MEADOW => "Alpine Meadow",
        B_ALPINE_TUNDRA => "Alpine Tundra",
        B_ALPINE_DESERT => "Alpine Desert",
        B_NIVAL => "Nival Zone",
        B_HOT_DESERT => "Hot Desert",
        B_COASTAL_FOG_DESERT => "Coastal Fog Desert",
        B_COLD_DESERT => "Cold Desert",
        B_SEMI_DESERT => "Semi-Desert Scrub",
        B_SALT_FLAT => "Salt Flat",
        B_DUNE_SEA => "Dune Sea",
        B_ROCKY_DESERT => "Rocky Desert",
        B_TUNDRA => "Tundra",
        B_POLAR_DESERT => "Polar Desert",
        B_ICE_SHEET => "Ice Sheet",
        B_GLACIER => "Glacier",
        B_PEAT_BOG => "Peat Bog",
        B_GALLERY_FOREST => "Gallery Forest",
        B_FLOODPLAIN_GRASSLAND => "Floodplain Grassland",
        B_FRESHWATER_MARSH => "Freshwater Marsh",
        B_SWAMP_FOREST => "Swamp Forest",
        B_SALT_MARSH => "Salt Marsh",
        B_OASIS => "Oasis",
        _ => "Unclassified",
    }
}

/// Coarse group a biome belongs to — used by the legend and the cell inspector.
pub fn biome_group(b: u8) -> &'static str {
    match b {
        B_TROPICAL_RAINFOREST | B_TROPICAL_MOIST_FOREST | B_TROPICAL_SEASONAL_FOREST
        | B_CLOUD_FOREST => "Tropical forest",
        B_SAVANNA | B_THORN_SCRUB => "Tropical grassland",
        B_TEMPERATE_RAINFOREST | B_TEMPERATE_DECIDUOUS | B_TEMPERATE_MIXED_FOREST
        | B_TEMPERATE_CONIFER | B_SUBTROPICAL_MOIST_FOREST => "Temperate forest",
        B_MEDITERRANEAN_WOODLAND | B_CHAPARRAL => "Mediterranean",
        B_TAIGA | B_FOREST_TUNDRA => "Boreal",
        B_TALLGRASS_PRAIRIE | B_SHORTGRASS_STEPPE | B_MONTANE_GRASSLAND => "Grassland",
        B_ALPINE_MEADOW | B_ALPINE_TUNDRA | B_ALPINE_DESERT | B_NIVAL => "Alpine",
        B_HOT_DESERT | B_COASTAL_FOG_DESERT | B_COLD_DESERT | B_SEMI_DESERT | B_SALT_FLAT
        | B_DUNE_SEA | B_ROCKY_DESERT => "Desert",
        B_TUNDRA | B_POLAR_DESERT | B_ICE_SHEET | B_GLACIER => "Polar",
        B_MANGROVE | B_PEAT_BOG | B_GALLERY_FOREST | B_FLOODPLAIN_GRASSLAND
        | B_FRESHWATER_MARSH | B_SWAMP_FOREST | B_SALT_MARSH | B_OASIS => "Wetland & riparian",
        _ => "Unclassified",
    }
}

// ─────────────────────────── Thresholds ───────────────────────────

/// Warmest-month mean temperature (°C) at the climatic treeline. The classic
/// Köppen/Brockmann-Jerosch value; modern measurement puts the growing-season
/// soil isotherm at 6.4 °C, which is what actually stops trees. Using a
/// TEMPERATURE treeline instead of a fixed elevation is what makes the treeline
/// descend from ~4000 m in the tropics to sea level at the Arctic circle.
const TREELINE_T_WARM: f32 = 6.5;
/// Warmest-month mean below which no vascular plant cover persists: permanent
/// snow / nival rock.
const NIVAL_T_WARM: f32 = 0.0;

/// Holdridge PET-ratio class boundaries (PET / precipitation).
const PET_PERHUMID: f32 = 0.5;
const PET_HUMID: f32 = 1.0;
const PET_SUBHUMID: f32 = 2.0;
const PET_SEMIARID: f32 = 4.0;
const PET_ARID: f32 = 8.0;

/// Elevation is normalized over 8848 m.
const M_PER_UNIT: f32 = 8848.0;

/// How far (in cells) a river's influence reaches for the azonal overrides. The
/// riparian corridor is genuinely narrow — a few hundred metres to a couple of
/// km — but it widens hugely in arid country, where the gallery forest / oasis
/// strip is the only green for a hundred km. `RIVER_REACH` is the arid maximum;
/// humid cells use only the innermost ring, so a rainforest is not repainted as
/// gallery forest (there is no contrast to draw).
const RIVER_REACH: u16 = 6;
const LAKE_REACH: u16 = 4;
/// Unreached sentinel for the BFS distance fields.
const FAR: u16 = u16::MAX;

/// Elevation ceiling (m) for the "tidal flat" test behind mangrove and salt
/// marsh. A real mangrove belt is a few km wide and within metres of sea level,
/// but a cell here is 30–110 km across, so the honest question is not "is this
/// cell intertidal" — it is "is this shore a low-lying depositional coast rather
/// than a cliffed one". 80 m is that line: it admits deltas, lagoon shores and
/// coastal plains while excluding anywhere the land climbs straight out of the
/// sea. Tightening it toward true tidal elevations would empty the biome, since
/// the elevation field's coastal taper rarely lands a shore cell below ~40 m.
const TIDAL_MAX_M: f32 = 80.0;

// ─────────────────────────── Entry point ───────────────────────────

/// Classify every land cell into a biome, writing `buf.biome`.
///
/// `rivers` / `lakes` come from phase 5 (they are objects, not tile columns), and
/// drive the azonal wetland/riparian biomes. Passing empty slices is valid — the
/// map then contains only zonal + altitudinal biomes.
pub fn classify_biomes(buf: &mut WorldBuffer, rivers: &[River], lakes: &[Lake]) {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = buf.total();

    // ── Precomputed neighbourhood fields (all linear or bounded-radius) ──
    let slope = compute_slope(buf);
    let coast = compute_coast_distance(buf);
    let (river_dist, big_river_dist) = compute_river_distance(buf, rivers);
    let (lake_dist, salt_lake_dist) = compute_lake_distance(buf, lakes);

    // Classify in parallel over rows; every cell writes only its own slot.
    let mut out = vec![B_NONE; n];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let i = y * w + x;
            if buf.terrain[i] != 1 {
                row[x] = B_NONE;
                continue;
            }
            row[x] = classify_cell(
                buf, i,
                Neighbourhood {
                    slope: slope[i],
                    coast_dist: coast[i],
                    river_dist: river_dist[i],
                    big_river_dist: big_river_dist[i],
                    lake_dist: lake_dist[i],
                    salt_lake_dist: salt_lake_dist[i],
                },
            );
        }
    });
    let _ = h;

    buf.biome = out;
}

/// The per-cell neighbourhood facts the classifier needs, all precomputed.
#[derive(Clone, Copy)]
struct Neighbourhood {
    /// Local relief in metres per cell (max drop to an 8-neighbour).
    slope: f32,
    /// Cells to the nearest sea cell (0 = touching the sea).
    coast_dist: u16,
    /// Cells to the nearest river cell of any size.
    river_dist: u16,
    /// Cells to the nearest navigable/major river or delta cell.
    big_river_dist: u16,
    /// Cells to the nearest freshwater lake cell.
    lake_dist: u16,
    /// Cells to the nearest terminal SALT lake cell.
    salt_lake_dist: u16,
}

/// The climate facts derived once per cell.
#[derive(Clone, Copy)]
struct Climate {
    /// Annual mean temperature (°C).
    t_mean: f32,
    /// Warmest-month mean (°C).
    t_warm: f32,
    /// Coldest-month mean (°C).
    t_cold: f32,
    /// Holdridge biotemperature (°C): monthly means clamped to 0–30, averaged.
    biotemp: f32,
    /// Annual precipitation (mm).
    precip: f32,
    /// Holdridge PET ratio = 58.93·biotemp / precip. Higher = drier.
    pet: f32,
    /// Summer share of annual precipitation (0..1).
    summer_frac: f32,
    /// Annual snow-cover fraction (0..1).
    snow: f32,
    /// Elevation in metres.
    elev_m: f32,
}

fn derive_climate(buf: &WorldBuffer, i: usize) -> Climate {
    let t_mean = buf.temperature[i];
    // Warmest/coldest month come from `koppen::seasonal_temps` — the documented
    // single source of truth for the seasonal split (it decodes `seasonal_amp`,
    // applies the asymmetric winter-dips-more skew, and falls back to the
    // latitude-parametric range on an old save whose column is still zero). Using
    // it here keeps the biome treeline consistent with Köppen's own C↔D boundary
    // and with the settlement winter gate.
    let w = buf.width as usize;
    let (t_cold, t_warm) = crate::sim::koppen::seasonal_temps(buf, (i % w) as u32, (i / w) as u32);
    // Sinusoid parameters implied by that (skewed) split.
    let t_mid = (t_warm + t_cold) * 0.5;
    let amp = (t_warm - t_cold) * 0.5;

    // Holdridge biotemperature: average of the 12 monthly means, each clamped to
    // [0, 30]. Months colder than freezing contribute nothing to growth, and heat
    // beyond 30 °C adds none either. A 12-step sinusoid is exact enough here and
    // costs a dozen flops.
    let mut acc = 0.0f32;
    for m in 0..12 {
        let phase = (m as f32) * std::f32::consts::TAU / 12.0;
        let t = t_mid + amp * phase.cos();
        acc += t.clamp(0.0, 30.0);
    }
    let biotemp = acc / 12.0;

    let precip = buf.precipitation[i].max(1.0);
    let pet = (58.93 * biotemp) / precip;

    Climate {
        t_mean,
        t_warm,
        t_cold,
        biotemp,
        precip,
        pet,
        summer_frac: buf.precip_summer_frac[i] as f32 / 255.0,
        snow: buf.snow_frac[i] as f32 / 255.0,
        elev_m: buf.elevation[i] * M_PER_UNIT,
    }
}

fn classify_cell(buf: &WorldBuffer, i: usize, nb: Neighbourhood) -> u8 {
    let c = derive_climate(buf, i);
    let k = buf.koppen[i];
    let soil = buf.soil_type[i];

    // ── 1. Cryosphere: nothing grows, and this beats everything ──────────
    if let Some(b) = cryosphere(&c, k, nb) {
        return b;
    }
    // ── 2. Azonal: local water beats regional climate ────────────────────
    if let Some(b) = azonal(&c, soil, nb) {
        return b;
    }
    // ── 3. Altitudinal: above the climatic treeline ──────────────────────
    if c.t_warm < TREELINE_T_WARM {
        return above_treeline(&c, nb);
    }
    // ── 4. Zonal life zone ───────────────────────────────────────────────
    zonal(&c, k, soil, nb)
}

/// Permanent ice, glaciers and bare nival rock.
fn cryosphere(c: &Climate, k: u8, nb: Neighbourhood) -> Option<u8> {
    // Ice cap: Köppen EF, or a cell that never thaws and holds snow year-round.
    if k == EF || (c.t_warm < NIVAL_T_WARM && c.snow > 0.9) {
        // On a low-relief lowland this is a continental ICE SHEET; on steep
        // mountain ground it is a valley/cirque GLACIER. Both are "permanent
        // ice", but they read completely differently on a map.
        return Some(if nb.slope > 60.0 && c.elev_m > 1500.0 {
            B_GLACIER
        } else {
            B_ICE_SHEET
        });
    }
    // Nival: thaws just barely, but too cold and too brief for any plant cover —
    // bare frost-shattered rock and permanent snowfields between the summits.
    if c.t_warm < NIVAL_T_WARM {
        return Some(B_NIVAL);
    }
    None
}

/// Wetland, riparian and coastal-saline biomes. These are the cells where LOCAL
/// water — a river, a lake, the tide — overrides what the regional climate would
/// otherwise dictate, and they are what makes hydrology legible on a biome map.
fn azonal(c: &Climate, soil: u8, nb: Neighbourhood) -> Option<u8> {
    let flat = nb.slope < 12.0;
    let very_flat = nb.slope < 6.0;
    let tidal = nb.coast_dist <= 1 && c.elev_m < TIDAL_MAX_M && very_flat;

    // ── Coastal saline wetlands ──
    if tidal {
        // Mangrove: the tropical/subtropical tidal forest. Bounded by frost —
        // mangroves die below about 5 °C in the coldest month, which is why they
        // stop dead around 30° latitude on the real Earth.
        if c.t_cold >= 5.0 && c.precip > 800.0 {
            return Some(B_MANGROVE);
        }
        // Salt marsh: the temperate equivalent, on the same tidal flats.
        if c.t_warm >= TREELINE_T_WARM && c.pet < PET_SEMIARID {
            return Some(B_SALT_MARSH);
        }
    }

    // ── Terminal salt lake shore: brine flats and playa ──
    if nb.salt_lake_dist <= 2 && very_flat {
        return Some(B_SALT_FLAT);
    }

    // ── Standing fresh water: marsh, swamp, bog ──
    let near_water = nb.lake_dist <= 2 || nb.big_river_dist <= 1;
    if near_water && very_flat && c.pet < PET_SUBHUMID {
        // Cold + waterlogged + organic soil = peat. This is the muskeg/mire that
        // covers a staggering share of the real boreal zone and is normally
        // missing from generated maps entirely.
        if c.biotemp < 6.0 || soil == SOIL_HISTOSOL {
            return Some(B_PEAT_BOG);
        }
        // Warm + wet enough to hold closed canopy on saturated ground.
        if c.t_warm >= 14.0 && c.pet < PET_HUMID {
            return Some(B_SWAMP_FOREST);
        }
        return Some(B_FRESHWATER_MARSH);
    }

    // ── Peat bog away from open water: cold, wet, flat, organic ──
    if soil == SOIL_HISTOSOL && very_flat && c.pet < PET_HUMID && c.biotemp < 9.0 {
        return Some(B_PEAT_BOG);
    }

    // ── Riparian corridors ────────────────────────────────────────────────
    // The corridor's WIDTH scales with aridity: in a rainforest a riverbank looks
    // like the rest of the rainforest (reach 1, and the humid guard below drops
    // it entirely), while in a desert the gallery strip is the only vegetation
    // for a very long way, and it is the single most important feature on the
    // map for anyone reading it.
    let reach = if c.pet >= PET_ARID {
        RIVER_REACH
    } else if c.pet >= PET_SEMIARID {
        4
    } else if c.pet >= PET_SUBHUMID {
        2
    } else {
        1
    };
    let on_river = nb.river_dist <= reach || nb.big_river_dist <= reach + 1;
    if on_river && c.t_warm >= TREELINE_T_WARM {
        // Desert oasis: a true arid-zone watered pocket — date palms and irrigated
        // green against bare ground.
        if c.pet >= PET_ARID {
            return Some(B_OASIS);
        }
        // Gallery forest: the ribbon of closed woodland that follows a river
        // through savanna, steppe or semi-desert. Only meaningful where the
        // SURROUNDING land is open — hence the dryness gate.
        if c.pet >= PET_SUBHUMID {
            return Some(B_GALLERY_FOREST);
        }
        // Humid country: a big river still makes a distinct floodplain, but only
        // where the ground is flat alluvium — otherwise it is just forest.
        if soil == SOIL_ALLUVIAL && flat && nb.big_river_dist <= 2 {
            return Some(B_FLOODPLAIN_GRASSLAND);
        }
    }

    None
}

/// Everything above the climatic treeline but below permanent ice.
fn above_treeline(c: &Climate, nb: Neighbourhood) -> u8 {
    // Distinguish genuinely ALPINE ground (a mountain, with relief) from flat
    // polar lowland: both are treeless and cold, but they are different places.
    let alpine = c.elev_m > 1200.0 && nb.slope > 15.0;

    if alpine {
        return if c.pet >= PET_SEMIARID {
            // The cold high desert of a rain-shadowed plateau (Tibet, the Altiplano).
            B_ALPINE_DESERT
        } else if c.pet >= PET_HUMID {
            // Dry short turf and cushion plants on frost-shattered ground.
            B_ALPINE_TUNDRA
        } else {
            // Wet, herb-rich summer pasture above the trees.
            B_ALPINE_MEADOW
        };
    }

    // Polar lowland.
    if c.pet >= PET_SEMIARID {
        // Cold *and* dry: the Arctic desert / dry valleys. Almost no cover.
        B_POLAR_DESERT
    } else {
        B_TUNDRA
    }
}

/// The zonal life zone — the vegetation the cell's own climate supports.
fn zonal(c: &Climate, k: u8, soil: u8, nb: Neighbourhood) -> u8 {
    // Köppen's dry-season letter is the cleanest available seasonality signal.
    let mediterranean = matches!(k, CSA | CSB | CSC | DSA | DSB | DSC | DSD);
    let dry_winter = matches!(k, AS | CWA | CWB | CWC | DWA | DWB | DWC | DWD);

    // ── Deserts and semi-deserts (PET ratio ≥ 4) ──────────────────────────
    if c.pet >= PET_SEMIARID {
        return desert(c, nb);
    }

    // ── Semi-arid: grassland, steppe and scrub ────────────────────────────
    if c.pet >= PET_SUBHUMID {
        if c.biotemp >= 18.0 {
            // Hot semi-arid: thorn scrub if it is genuinely dry, savanna if the
            // rain merely arrives all at once.
            return if c.pet >= 3.0 { B_THORN_SCRUB } else { B_SAVANNA };
        }
        if mediterranean {
            return B_CHAPARRAL;
        }
        if c.elev_m > 1800.0 {
            return B_MONTANE_GRASSLAND;
        }
        return B_SHORTGRASS_STEPPE;
    }

    // ── Sub-humid: the tallgrass / forest-steppe transition ───────────────
    if c.pet >= PET_HUMID {
        if c.biotemp >= 18.0 {
            return B_SAVANNA;
        }
        if mediterranean {
            return B_MEDITERRANEAN_WOODLAND;
        }
        // The prairie belt: enough rain for trees by total, but the summer
        // moisture deficit and fire keep it in grass. Mollisol — the deep black
        // prairie soil — is the fingerprint of exactly this.
        if soil == SOIL_MOLLISOL || c.summer_frac < 0.35 {
            return B_TALLGRASS_PRAIRIE;
        }
        if c.biotemp < 6.0 {
            return B_FOREST_TUNDRA;
        }
        if c.biotemp < 9.0 {
            return B_TAIGA;
        }
        return B_TEMPERATE_DECIDUOUS;
    }

    // ── Humid and wetter: closed forest. Split by biotemperature. ─────────
    // Tropical (biotemp ≥ 24 °C — Holdridge's tropical belt).
    if c.biotemp >= 24.0 {
        // Cloud forest: a persistently wet tropical mountain in the condensation
        // band. Needs the elevation AND the moisture — a dry tropical mountain is
        // montane grassland, not cloud forest.
        if c.elev_m > 1200.0 && c.pet < PET_PERHUMID {
            return B_CLOUD_FOREST;
        }
        if dry_winter || c.summer_frac > 0.75 {
            return if c.pet < PET_PERHUMID {
                B_TROPICAL_MOIST_FOREST
            } else {
                B_TROPICAL_SEASONAL_FOREST
            };
        }
        return if c.pet < PET_PERHUMID {
            B_TROPICAL_RAINFOREST
        } else {
            B_TROPICAL_MOIST_FOREST
        };
    }

    // Subtropical (biotemp 17–24 °C).
    if c.biotemp >= 17.0 {
        if mediterranean {
            return B_MEDITERRANEAN_WOODLAND;
        }
        if c.elev_m > 1500.0 && c.pet < PET_PERHUMID {
            return B_CLOUD_FOREST;
        }
        return B_SUBTROPICAL_MOIST_FOREST;
    }

    // Warm temperate (biotemp 12–17 °C).
    if c.biotemp >= 12.0 {
        if mediterranean {
            return B_MEDITERRANEAN_WOODLAND;
        }
        // Temperate rainforest: the very wet, mild, maritime west coast
        // (Pacific Northwest, Chile, Fiordland, western Norway).
        if c.pet < 0.35 && nb.coast_dist < 30 {
            return B_TEMPERATE_RAINFOREST;
        }
        return B_TEMPERATE_DECIDUOUS;
    }

    // Cool temperate (biotemp 9–12 °C).
    if c.biotemp >= 9.0 {
        if c.pet < 0.35 && nb.coast_dist < 30 {
            return B_TEMPERATE_RAINFOREST;
        }
        // Conifers take over where the growing season shortens or the soil is a
        // leached podzol that broadleaves cannot hold.
        if soil == SOIL_SPODOSOL || c.t_cold < -8.0 {
            return B_TEMPERATE_CONIFER;
        }
        return B_TEMPERATE_MIXED_FOREST;
    }

    // Boreal (biotemp 3–9 °C).
    if c.biotemp >= 6.0 {
        return B_TAIGA;
    }
    if c.biotemp >= 3.0 {
        return B_FOREST_TUNDRA;
    }

    // Below the boreal threshold but the warmest month still clears the treeline —
    // scattered krummholz over tundra.
    B_FOREST_TUNDRA
}

/// The desert family. Which desert a cell is depends far more on its GROUND and
/// its coast than on how dry it is.
fn desert(c: &Climate, nb: Neighbourhood) -> u8 {
    // Coastal fog desert: hyper-arid, but cooled and fogged by an offshore cold
    // current — Atacama, Namib, Baja. Cool for its latitude and right on the sea.
    if nb.coast_dist <= 6 && c.pet >= PET_ARID && c.t_mean < 22.0 && c.biotemp >= 12.0 {
        return B_COASTAL_FOG_DESERT;
    }
    // Merely semi-arid: scrub, not bare ground.
    if c.pet < PET_ARID {
        return B_SEMI_DESERT;
    }
    if c.biotemp < 12.0 {
        return B_COLD_DESERT;
    }
    // True hot desert — now split by LANDFORM, which is what a traveller
    // actually cares about.
    if nb.slope > 45.0 {
        // Broken, stony, deeply dissected ground: hamada and wadi country.
        return B_ROCKY_DESERT;
    }
    if nb.slope < 4.0 && c.pet >= 12.0 {
        // Flat and hyper-arid: the sand sea. Ergs form in the flattest, driest
        // basins where there is nothing to trap the sand.
        return B_DUNE_SEA;
    }
    B_HOT_DESERT
}

// ─────────────────────── Precomputed fields ───────────────────────

/// Local relief in metres per cell: the largest elevation drop to an 8-neighbour.
/// Row-parallel; each row writes only its own slice.
fn compute_slope(buf: &WorldBuffer) -> Vec<f32> {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let mut out = vec![0.0f32; buf.total()];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let i = y * w + x;
            if buf.terrain[i] != 1 {
                continue;
            }
            let e = buf.elevation[i];
            let mut max_d = 0.0f32;
            for dy in -1i32..=1 {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    // X wraps (cylindrical topology), Y clamps at the poles.
                    let nx = ((x as i32 + dx) % w as i32 + w as i32) % w as i32;
                    let ni = ny as usize * w + nx as usize;
                    if buf.terrain[ni] != 1 {
                        continue;
                    }
                    max_d = max_d.max((e - buf.elevation[ni]).abs());
                }
            }
            row[x] = max_d * M_PER_UNIT;
        }
    });
    out
}

/// Cells to the nearest sea cell, as a bounded multi-source BFS from the coast.
fn compute_coast_distance(buf: &WorldBuffer) -> Vec<u16> {
    let w = buf.width as usize;
    let h = buf.height as usize;
    let n = buf.total();
    let mut dist = vec![FAR; n];
    let mut queue = VecDeque::new();
    // Seed: every LAND cell touching the sea is distance 0.
    for i in 0..n {
        if buf.terrain[i] != 1 {
            continue;
        }
        let x = i % w;
        let y = i / w;
        let mut touches = false;
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = y as i32 + dy;
            if ny < 0 || ny >= h as i32 {
                continue;
            }
            let nx = ((x as i32 + dx) % w as i32 + w as i32) % w as i32;
            if buf.terrain[ny as usize * w + nx as usize] != 1 {
                touches = true;
                break;
            }
        }
        if touches {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    // Cap the flood: nothing in the classifier looks past 30 cells inland, so a
    // full continental flood would be wasted work on a 26 M-cell world.
    bfs_land(buf, &mut dist, &mut queue, 30);
    dist
}

/// Distance fields for rivers: `(any river, major/navigable river or delta)`.
fn compute_river_distance(buf: &WorldBuffer, rivers: &[River]) -> (Vec<u16>, Vec<u16>) {
    let w = buf.width as usize;
    let n = buf.total();
    let mut any = vec![FAR; n];
    let mut big = vec![FAR; n];
    let mut q_any = VecDeque::new();
    let mut q_big = VecDeque::new();

    for r in rivers {
        let is_big = r.major || r.navigable;
        for &(x, y) in r.points.iter().chain(r.delta.iter()) {
            let i = y as usize * w + x as usize;
            if i >= n || buf.terrain[i] != 1 {
                continue;
            }
            if any[i] != 0 {
                any[i] = 0;
                q_any.push_back(i);
            }
            // A delta is by definition a big-river landform, whatever the
            // trunk's own flags say.
            if is_big && big[i] != 0 {
                big[i] = 0;
                q_big.push_back(i);
            }
        }
    }
    bfs_land(buf, &mut any, &mut q_any, RIVER_REACH);
    bfs_land(buf, &mut big, &mut q_big, RIVER_REACH + 1);
    (any, big)
}

/// Distance fields for lakes: `(freshwater, terminal salt)`.
fn compute_lake_distance(buf: &WorldBuffer, lakes: &[Lake]) -> (Vec<u16>, Vec<u16>) {
    let w = buf.width as usize;
    let n = buf.total();
    let mut fresh = vec![FAR; n];
    let mut salt = vec![FAR; n];
    let mut q_fresh = VecDeque::new();
    let mut q_salt = VecDeque::new();

    for l in lakes {
        for &(x, y) in &l.cells {
            let i = y as usize * w + x as usize;
            if i >= n {
                continue;
            }
            // Lake cells themselves are land in the terrain mask (they are a
            // filled depression), so seed them and let the BFS spread onto the
            // surrounding shore.
            let (field, queue) = if l.endorheic {
                (&mut salt, &mut q_salt)
            } else {
                (&mut fresh, &mut q_fresh)
            };
            if field[i] != 0 {
                field[i] = 0;
                queue.push_back(i);
            }
        }
    }
    bfs_land(buf, &mut fresh, &mut q_fresh, LAKE_REACH);
    bfs_land(buf, &mut salt, &mut q_salt, LAKE_REACH);
    (fresh, salt)
}

/// Bounded 4-connected multi-source BFS across LAND, respecting the cylindrical
/// topology (X wraps, Y clamps). Stops at `max_dist`, so the cost is O(seeded
/// area × max_dist) rather than O(world).
fn bfs_land(buf: &WorldBuffer, dist: &mut [u16], queue: &mut VecDeque<usize>, max_dist: u16) {
    let w = buf.width as usize;
    let h = buf.height as usize;
    while let Some(ci) = queue.pop_front() {
        let d = dist[ci];
        if d >= max_dist {
            continue;
        }
        let x = ci % w;
        let y = ci / w;
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let ny = y as i32 + dy;
            if ny < 0 || ny >= h as i32 {
                continue;
            }
            let nx = ((x as i32 + dx) % w as i32 + w as i32) % w as i32;
            let ni = ny as usize * w + nx as usize;
            if buf.terrain[ni] != 1 || dist[ni] <= d + 1 {
                continue;
            }
            dist[ni] = d + 1;
            queue.push_back(ni);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::db::schema;
    use crate::sim::world_buffer::{ColumnSet, SEASON_AMP_SCALE};
    use rusqlite::Connection;

    /// Encode a seasonal SPAN (warmest − coldest month, °C) into the column's units.
    fn span(deg: f32) -> u8 {
        (deg * SEASON_AMP_SCALE).round().clamp(1.0, 255.0) as u8
    }

    /// A uniform all-land test world: mild oceanic, no relief, no hydrology.
    fn world(gw: u32, gh: u32) -> WorldBuffer {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        for (k, v) in [("grid_width", gw.to_string()), ("grid_height", gh.to_string())] {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
                rusqlite::params![k, v],
            ).unwrap();
        }
        let mut buf = WorldBuffer::load_with(&conn, ColumnSet::ALL).unwrap();
        for i in 0..buf.total() {
            buf.terrain[i] = 1;
            buf.elevation[i] = 0.02;
            buf.temperature[i] = 20.0;
            buf.precipitation[i] = 1200.0;
            buf.seasonal_amp[i] = span(10.0);
            buf.precip_summer_frac[i] = 128;
            buf.distance_to_ocean[i] = 0.5;
            buf.koppen[i] = CFB;
            buf.soil_type[i] = SOIL_ALFISOL;
        }
        buf
    }

    /// Biotemperature must clamp: a place that spends half the year frozen has a
    /// far lower biotemperature than its annual mean, and a place that bakes at
    /// 40 °C gains nothing above 30.
    #[test]
    fn biotemperature_clamps_both_ends() {
        let mut buf = world(8, 8);
        // Annual mean 0 °C with a huge swing: the warm half still grows.
        buf.temperature[0] = 0.0;
        buf.seasonal_amp[0] = span(40.0);
        let cold = derive_climate(&buf, 0);
        assert!(cold.biotemp > 0.0, "warm months must contribute");
        assert!(cold.biotemp < 10.0, "frozen months must contribute nothing");

        // Uniformly 40 °C: biotemperature saturates at 30.
        buf.temperature[1] = 40.0;
        buf.seasonal_amp[1] = span(0.5);
        let hot = derive_climate(&buf, 1);
        assert!((hot.biotemp - 30.0).abs() < 0.01, "heat above 30 °C is not growth");
    }

    /// The treeline is a TEMPERATURE, not an elevation: a tropical mountain must
    /// keep forest far higher than a subpolar one.
    #[test]
    fn treeline_follows_temperature_not_elevation() {
        let mut buf = world(8, 8);
        // Tropical highland at 3000 m, still warm.
        buf.elevation[0] = 3000.0 / M_PER_UNIT;
        buf.temperature[0] = 14.0;
        buf.seasonal_amp[0] = span(4.0);
        // Subpolar lowland at 100 m, cold: warmest month ≈ -6 + 20·0.45 = 3 °C.
        buf.elevation[1] = 100.0 / M_PER_UNIT;
        buf.temperature[1] = -6.0;
        buf.seasonal_amp[1] = span(20.0);
        buf.precipitation[1] = 400.0;

        let nb = Neighbourhood {
            slope: 5.0, coast_dist: 100, river_dist: FAR,
            big_river_dist: FAR, lake_dist: FAR, salt_lake_dist: FAR,
        };
        let high = classify_cell(&buf, 0, nb);
        let low = classify_cell(&buf, 1, nb);
        assert_ne!(biome_group(high), "Polar", "warm tropical highland keeps trees");
        assert!(
            matches!(low, B_TUNDRA | B_POLAR_DESERT | B_FOREST_TUNDRA),
            "cold subpolar lowland is treeless, got {}", biome_name(low)
        );
    }

    /// A river through dry country must produce a visibly different biome from
    /// the land beside it — that is the whole point of the azonal layer. The same
    /// river through rainforest must NOT, because there is no contrast to draw.
    #[test]
    fn rivers_only_repaint_dry_country() {
        let mut buf = world(8, 8);
        // Arid cell.
        buf.temperature[0] = 26.0;
        buf.precipitation[0] = 120.0;
        buf.koppen[0] = BWH;
        // Wet tropical cell.
        buf.temperature[1] = 26.0;
        buf.precipitation[1] = 3000.0;
        buf.koppen[1] = AF;

        let dry_nb = Neighbourhood {
            slope: 3.0, coast_dist: 100, river_dist: FAR,
            big_river_dist: FAR, lake_dist: FAR, salt_lake_dist: FAR,
        };
        let on_river = Neighbourhood { river_dist: 1, ..dry_nb };

        let desert_away = classify_cell(&buf, 0, dry_nb);
        let desert_on_river = classify_cell(&buf, 0, on_river);
        assert_ne!(desert_away, desert_on_river, "a river must show in the desert");
        assert_eq!(desert_on_river, B_OASIS);

        let jungle_away = classify_cell(&buf, 1, dry_nb);
        let jungle_on_river = classify_cell(&buf, 1, on_river);
        assert_eq!(jungle_away, jungle_on_river, "a river adds nothing in rainforest");
    }

    /// Mangroves are frost-bounded: the same tidal flat carries mangrove in the
    /// tropics and salt marsh where winters freeze.
    #[test]
    fn mangrove_stops_at_frost() {
        let mut buf = world(8, 8);
        buf.precipitation[0] = 1600.0;
        buf.precipitation[1] = 1600.0;
        // A low-lying depositional shore (see TIDAL_MAX_M).
        buf.elevation[0] = 30.0 / M_PER_UNIT;
        buf.elevation[1] = 30.0 / M_PER_UNIT;
        // Tropical coast: coldest month well above 5 °C.
        buf.temperature[0] = 26.0;
        buf.seasonal_amp[0] = span(4.0);
        // Temperate coast: coldest month below freezing (10 − 24·0.55 ≈ −3 °C).
        buf.temperature[1] = 10.0;
        buf.seasonal_amp[1] = span(24.0);

        let tidal = Neighbourhood {
            slope: 1.0, coast_dist: 0, river_dist: FAR,
            big_river_dist: FAR, lake_dist: FAR, salt_lake_dist: FAR,
        };
        assert_eq!(classify_cell(&buf, 0, tidal), B_MANGROVE);
        assert_eq!(classify_cell(&buf, 1, tidal), B_SALT_MARSH);
    }

    /// Every code the classifier can emit must have a name and a group — a code
    /// that falls through to "Unclassified" is a silent hole in the legend.
    #[test]
    fn every_biome_code_is_named() {
        for b in 1..BIOME_COUNT as u8 {
            assert_ne!(biome_name(b), "Unclassified", "biome {b} has no name");
            assert_ne!(biome_group(b), "Unclassified", "biome {b} has no group");
        }
    }

    /// BACK-COMPAT: a world saved before phase 6b existed has `seasonal_amp`,
    /// `precip_summer_frac`, `snow_frac` and `biome` all padded to zero on load.
    /// Classifying such a world must not panic and must not produce garbage — it
    /// is exactly what happens the first time an existing `.worldforge` is opened
    /// and the user presses Reclassify Biomes.
    ///
    /// The zero `seasonal_amp` is the interesting part: it sends
    /// `koppen::seasonal_temps` down its latitude-parametric FALLBACK, which calls
    /// `continentality` → `upwind_is_open_ocean` → the shelf column. That path
    /// must survive here too, which is why PHASE_BIOME carries SHELF.
    #[test]
    fn an_old_save_with_no_seasonality_columns_still_classifies() {
        let mut buf = world(48, 32);
        // Exactly what an old blob decodes to.
        for i in 0..buf.total() {
            buf.seasonal_amp[i] = 0;
            buf.precip_summer_frac[i] = 0;
            buf.snow_frac[i] = 0;
            buf.biome[i] = 0;
        }
        // A little sea so the coast/continentality paths are exercised.
        for i in 0..(48 * 4) {
            buf.terrain[i] = 0;
        }
        classify_biomes(&mut buf, &[], &[]);

        let land: Vec<u8> = (0..buf.total())
            .filter(|&i| buf.terrain[i] == 1)
            .map(|i| buf.biome[i])
            .collect();
        assert!(!land.is_empty());
        assert!(land.iter().all(|&b| b != B_NONE),
            "every land cell must get a biome even with no seasonality columns");
        assert!(land.iter().all(|&b| (b as usize) < BIOME_COUNT),
            "no out-of-range code");
    }

    /// The same, with the shelf column absent entirely — the defensive path in
    /// `upwind_is_open_ocean`. Guards against a future phase mask dropping SHELF.
    #[test]
    fn classification_survives_a_missing_shelf_column() {
        let mut buf = world(32, 24);
        for i in 0..buf.total() {
            buf.seasonal_amp[i] = 0;
        }
        buf.is_shelf = Vec::new();
        buf.is_shelf_edge = Vec::new();
        classify_biomes(&mut buf, &[], &[]);
        assert!(buf.biome.iter().any(|&b| b != B_NONE));
    }

    /// Sea cells stay unclassified, and a river/lake-free world still classifies.
    #[test]
    fn sea_stays_unclassified_and_empty_hydrology_is_valid() {
        let mut buf = world(16, 16);
        for i in 0..32 {
            buf.terrain[i] = 0;
        }
        classify_biomes(&mut buf, &[], &[]);
        for i in 0..32 {
            assert_eq!(buf.biome[i], B_NONE);
        }
        assert!(buf.biome[buf.total() - 1] != B_NONE, "land must be classified");
    }
}
