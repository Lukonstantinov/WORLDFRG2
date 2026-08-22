//! Ore deposits — the geological placer for `Distribution::Deposits` goods.
//!
//! # Why this module exists
//!
//! The previous placer decided where a metal or gem was found from exactly two
//! inputs: an absolute elevation floor and a per-good noise field. That encodes no
//! geology. Meanwhile phase 1 already computes plate boundaries, boundary TYPE and
//! volcanism, and phase 5 already computes rivers — and the goods placer read none
//! of it.
//!
//! Ore geology is almost entirely a function of TECTONIC SETTING. That is not
//! flavour; it is the organising principle of the discipline. Silver is found
//! above subduction zones, tin in collisional granites, diamond only on ancient
//! cratons, lead-zinc in carbonate platforms, rock salt in arid restricted basins,
//! and a great deal of the gold and tin ever won by pre-modern people came out of
//! river gravel BELOW one of those. So a deposit good now carries a
//! [`DepositModel`], and the model — not an elevation number — decides where it
//! goes.
//!
//! # The three-level hierarchy
//!
//! Real ore geology is described at three scales, and the old placer collapsed all
//! three into one cell:
//!
//! | Level | Real scale | Here | Example |
//! |---|---|---|---|
//! | Metallogenic belt | 100–1000 km | the model's setting field × belt noise | Iberian Pyrite Belt (~250 × 30 km) |
//! | Ore district / camp | 10–60 km | a cluster centre, `MIN_DISTRICT_SEP_KM` apart | Freiberg, the Mendips, Laurion |
//! | Working / deposit | 1–10 km | ONE cell (11 km at the standard grid) | Cerro Rico, Rammelsberg, one quarry face |
//!
//! A cell is already ~11 km wide at 3600×1800 (5.6 km on Large), i.e. FINER than
//! an ore district — so clustering is a matter of placing several workings per
//! district, not of shrinking cells. The old code placed single cells a full
//! `width * 0.025` ≈ 1000 km apart, which is why deposits read as isolated dots.
//!
//! # Per-working state
//!
//! Each working carries three numbers that the old u8 intensity could not express:
//!
//! * `grade` — ore richness / stone fineness. Drives the good's QUALITY tier, which
//!   is what makes "tiers of gems" possible. (The old quality formula derived a
//!   hub's gem quality from its SHARE OF WORLD PRODUCTION, so a big cheap deposit
//!   read as fine stones — exactly backwards.)
//! * `extent` — how much is there. Only a WEAK body can be meaningfully drawn down.
//! * `depth` — outcrop → shallow → deep → below the water table. This is *the*
//!   pre-modern mining constraint: the whole of Agricola's *De Re Metallica* is in
//!   large part a drainage manual, and Rio Tinto's Roman reverse waterwheels exist
//!   for one reason. Only surface and shallow workings are meaningfully available
//!   to a settlement with no mining capability.
//!
//! # Template worlds
//!
//! `boundary_type` is EMPTY on template/painted worlds (see `elevation.rs`, which
//! already guards the same way). Every tectonic model therefore degrades to a
//! relief-and-continentality proxy rather than vanishing — the failure mode this
//! codebase has hit before is a good that silently places nothing.

use serde::{Deserialize, Serialize};

use crate::sim::plates::{BOUNDARY_CONVERGENT, BOUNDARY_DIVERGENT};
use crate::sim::rivers::River;
use crate::sim::world_buffer::WorldBuffer;

// ── Depth classes ───────────────────────────────────────────────────────────
/// Outcrop, gossan or river gravel — workable by anyone with hand tools.
pub const DEPTH_SURFACE: u8 = 0;
/// Open cut or a short adit — modest organisation, no drainage.
pub const DEPTH_SHALLOW: u8 = 1;
/// Galleries needing timbering and ventilation — a real mining operation.
pub const DEPTH_DEEP: u8 = 2;
/// Below the water table — worthless without drainage engineering.
pub const DEPTH_FLOODED: u8 = 3;

// ── Extent classes ──────────────────────────────────────────────────────────
/// A pocket. This is the ONLY class that a large city working it hard can draw
/// down appreciably, and even then only over centuries and only to a floor.
pub const EXTENT_WEAK: u8 = 0;
pub const EXTENT_MODERATE: u8 = 1;
pub const EXTENT_GREAT: u8 = 2;
/// A Rammelsberg (worked ~1000 years) or a Cerro Rico. Effectively inexhaustible.
pub const EXTENT_WORLD_CLASS: u8 = 3;

/// How much of a working's grade is actually reachable at each depth, before any
/// mining capability is applied. A deep or flooded body is VISIBLE but largely
/// locked — it is inventory for a future mining industry, not present output.
pub fn depth_workability(depth: u8) -> f32 {
    match depth {
        DEPTH_SURFACE => 1.00,
        DEPTH_SHALLOW => 0.80,
        DEPTH_DEEP => 0.35,
        _ => 0.15,
    }
}

pub fn depth_label(depth: u8) -> &'static str {
    match depth {
        DEPTH_SURFACE => "surface",
        DEPTH_SHALLOW => "shallow",
        DEPTH_DEEP => "deep",
        _ => "flooded",
    }
}

pub fn extent_label(extent: u8) -> &'static str {
    match extent {
        EXTENT_WEAK => "weak",
        EXTENT_MODERATE => "moderate",
        EXTENT_GREAT => "great",
        _ => "world-class",
    }
}

/// Grade → the quality word a merchant would use. This is the "tier" ladder: for a
/// gem it reads as clarity, for marble as statuary vs building stone, for an ore as
/// richness.
pub fn grade_label(grade: f32) -> &'static str {
    match grade {
        g if g >= 0.86 => "exquisite",
        g if g >= 0.70 => "fine",
        g if g >= 0.52 => "good",
        g if g >= 0.34 => "ordinary",
        _ => "coarse",
    }
}

// ── The deposit models ──────────────────────────────────────────────────────

/// The ore-forming environment a mineral belongs to. Each is a real, named class
/// of deposit with real type localities; the doc comment on each is the reason it
/// scores where it scores, so a future session can check the geology rather than
/// re-deriving it.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DepositModel {
    /// Volcanic arc above a subduction zone: epithermal silver-gold veins, volcanic
    /// massive sulphide copper, native sulphur, alum from hydrothermally altered
    /// tuff. Potosí, Guanajuato, Cyprus, Rio Tinto, Tolfa.
    VolcanicArc,
    /// Collisional orogen and its granites: orogenic lode gold, tin-tungsten
    /// greisen, cobalt, slate, granite. Cornwall, the Erzgebirge, the Alps.
    CollisionalOrogen,
    /// Ancient stable plate interior. Banded iron formation, and — by Clifford's
    /// Rule, the most rule-like fact in ore geology — economic diamondiferous
    /// kimberlite occurs ONLY over thick Archean lithosphere. Never in young
    /// mountains, which is where the old placer put diamond.
    Craton,
    /// Continental rift and flood basalt: sediment-hosted stratiform copper,
    /// agate/carnelian in basalt amygdules. Kupferschiefer, the Deccan.
    Rift,
    /// Shallow-sea carbonate platform and its basins: Mississippi-Valley-type
    /// lead-zinc, calamine, coal measures, building limestone. Mendips, Silesia.
    CarbonatePlatform,
    /// Platform carbonate cooked by a nearby intrusion. Marble is metamorphosed
    /// limestone (Carrara, Pentelikon); Mogok's rubies are marble-hosted; lapis
    /// lazuli at Sar-i-Sang is contact-metamorphosed limestone.
    ContactMetamorphic,
    /// Arid restricted basin: rock salt, gypsum/alabaster, natron, nitre.
    /// Wieliczka, Hallstatt, Cheshire.
    EvaporiteBasin,
    /// River gravel below a lode. NOT scored from a field — derived by walking
    /// downstream from a parent working (see `placer_frac`). Most pre-modern gold
    /// and tin came this way: Cornwall worked stream tin before lode tin, and
    /// Golconda's diamonds were entirely alluvial.
    Placer,
    /// Wetland iron: bog ore, precipitated in marshes and reworked every
    /// generation. The iron source of early medieval northern Europe, and
    /// structurally impossible under an elevation-floor model because it is a
    /// LOWLAND deposit.
    Bog,
    /// Shelf, beach and warm shallows: pearl beds, murex, coral, amber, solar salt.
    CoastalMarine,
    /// Supergene alteration of a parent orebody under an arid climate — a DAUGHTER
    /// deposit. Turquoise at Nishapur and Sinai forms where copper-bearing rock
    /// weathers in a desert.
    Weathering,
}

impl DepositModel {
    /// Derived models are placed relative to a PARENT deposit rather than scored
    /// from a setting field.
    pub fn is_derived(self) -> bool {
        matches!(self, DepositModel::Placer | DepositModel::Weathering)
    }

    pub fn label(self) -> &'static str {
        match self {
            DepositModel::VolcanicArc => "volcanic arc",
            DepositModel::CollisionalOrogen => "collisional orogen",
            DepositModel::Craton => "craton",
            DepositModel::Rift => "rift",
            DepositModel::CarbonatePlatform => "carbonate platform",
            DepositModel::ContactMetamorphic => "contact metamorphic",
            DepositModel::EvaporiteBasin => "evaporite basin",
            DepositModel::Placer => "placer",
            DepositModel::Bog => "bog",
            DepositModel::CoastalMarine => "coastal / marine",
            DepositModel::Weathering => "weathering",
        }
    }
}

/// One WORKING — a single mine, quarry or gravel field. The discrete record the
/// u8 belt column cannot carry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Deposit {
    /// Spec id of the good this working produces.
    pub good: String,
    pub x: u32,
    pub y: u32,
    /// Which ore district this working belongs to (index within this good).
    pub district: u32,
    /// Ore richness / stone fineness, 0..1 → the good's quality tier.
    pub grade: f32,
    pub extent: u8,
    pub depth: u8,
    pub model: DepositModel,
}

impl Deposit {
    /// How much this working contributes to the belt column BEFORE any mining
    /// capability: grade attenuated by how reachable it is.
    pub fn workable_intensity(&self) -> f32 {
        (self.grade * depth_workability(self.depth)).clamp(0.0, 1.0)
    }
}

// ── The default model per shipped mineral ───────────────────────────────────

/// The geologically correct default model for a shipped mineral, plus the share of
/// its districts that are ALLUVIAL rather than in-situ.
///
/// This is the table the user edits from in the Goods Designer — it exists so that
/// nobody has to author geology to get a plausible world, while anybody can
/// override a single mineral without touching the rest.
///
/// `placer_frac` is the fraction of a good's districts placed as river gravel
/// downstream of a lode district, and it is high for exactly the minerals that
/// were historically won from gravel first.
pub fn default_model_for(id: &str) -> (DepositModel, f32) {
    use DepositModel::*;
    match id {
        // ── Metals ──
        // Orogenic lode gold + the alluvial gold that financed antiquity (the
        // Pactolus, Bambuk, the Klondike).
        "gold" => (CollisionalOrogen, 0.45),
        // Epithermal silver above an arc: Potosí, Guanajuato, and Joachimsthal —
        // whose coin gave us the word "dollar".
        "silver" => (VolcanicArc, 0.05),
        // Volcanic massive sulphide. Cyprus gave copper its name.
        "copper" => (VolcanicArc, 0.0),
        // Greisen tin over collisional granite; Cornwall streamed it before it
        // mined it.
        "tin" => (CollisionalOrogen, 0.35),
        // Banded iron formation on old shields.
        "iron" => (Craton, 0.0),
        // Mississippi-Valley-type lead in platform carbonate: the Mendips, Silesia.
        "lead" => (CarbonatePlatform, 0.0),
        // Cinnabar above an arc — Almadén and Idrija were effectively the world's
        // only sources, and mercury amalgamation is how Potosí's silver was refined
        // from 1554.
        "mercury" => (VolcanicArc, 0.0),
        // Zinc carbonate for brass, in platform carbonate. Moresnet, Altenberg.
        "calamine" => (CarbonatePlatform, 0.0),
        // Erzgebirge cobalt — smalt, the blue of medieval glass.
        "cobalt" => (CollisionalOrogen, 0.0),

        // ── Gems ──
        // Mogok's rubies sit in MARBLE, not in a volcano, and were won from the
        // gravels below it.
        "ruby" => (ContactMetamorphic, 0.60),
        // Basalt-hosted, and famously worked from the illam gravels of Ratnapura.
        "sapphire" => (VolcanicArc, 0.70),
        // Muzo and Habachtal: hydrothermal, in black shale and schist along an
        // orogen — not a peak deposit.
        "emerald" => (CollisionalOrogen, 0.10),
        // Clifford's Rule: cratons only. And every diamond traded before 1725 came
        // from the alluvial workings of Golconda.
        "diamond" => (Craton, 0.80),
        // Agate and amethyst fill gas cavities in flood basalt.
        "amethyst" => (Rift, 0.20),
        "carnelian" => (Rift, 0.50),
        // Granite pegmatite and greisen.
        "topaz" => (CollisionalOrogen, 0.30),
        "garnet" => (CollisionalOrogen, 0.30),
        // Khotan jade came out of a riverbed; Burmese jade from contact rock.
        "jade" => (ContactMetamorphic, 0.50),
        // Sar-i-Sang in Badakhshan — one district, for four thousand years.
        "lapis_lazuli" => (ContactMetamorphic, 0.0),
        // Supergene copper alteration in a desert: Nishapur, Serabit el-Khadim.
        "turquoise" => (Weathering, 0.0),
        // The generic pre-split gem belt, kept so old worlds still load.
        "gemstones" => (CollisionalOrogen, 0.40),

        // ── Stone & industrial ──
        // Carrara: platform limestone metamorphosed by the Apuan orogeny.
        "marble" => (ContactMetamorphic, 0.0),
        "granite" => (CollisionalOrogen, 0.0),
        "slate" => (CollisionalOrogen, 0.0),
        // Gypsum in an evaporite sequence: Volterra, Nottingham.
        "alabaster" => (EvaporiteBasin, 0.0),
        // Freshwater burrstone for millstones — La Ferté-sous-Jouarre.
        "millstone" => (CarbonatePlatform, 0.0),
        // Alum from hydrothermally altered volcanic rock. Tolfa, 1462.
        "alum" => (VolcanicArc, 0.0),
        // Coal measures are a sedimentary basin sequence.
        "coal" => (CarbonatePlatform, 0.0),
        // Nitre earth — arid, sheltered, Bengal.
        "saltpetre" => (EvaporiteBasin, 0.0),

        // ── Evaporite & coastal ──
        "salt" => (EvaporiteBasin, 0.0),
        "bay_salt" => (CoastalMarine, 0.0),
        "tyrian_purple" => (CoastalMarine, 0.0),
        "ambergris" => (CoastalMarine, 0.0),
        "amber" => (CoastalMarine, 0.0),
        // Bog ore: the lowland iron an elevation floor can never produce.
        "bog_iron" => (Bog, 0.0),

        // Unknown / user-defined without an explicit model: the orogen is the
        // single most common host for metals, so it is the least-wrong default.
        _ => (CollisionalOrogen, 0.0),
    }
}

/// The parent good a DERIVED model hangs off. A placer's parent is its own lode,
/// so only `Weathering` needs a table.
pub fn default_parent_for(id: &str) -> Option<&'static str> {
    match id {
        "turquoise" => Some("copper"),
        _ => None,
    }
}

// ── Geological context ──────────────────────────────────────────────────────

const UNREACHED: u16 = u16::MAX;

/// Distance fields shared by every deposit model, computed ONCE per world rather
/// than per good (45 goods × a full-grid BFS would be a real cost).
///
/// Every field is a multi-source BFS — a linear sweep from all seeds at once, NOT
/// an outward scan per cell (§8.9 rule 1). Distances are 8-connected, so they are
/// Chebyshev rather than Euclidean; these feed soft proximity ramps tens of cells
/// wide, where that anisotropy is invisible.
pub struct GeoContext {
    /// Cells to the nearest convergent boundary.
    pub conv_dist: Vec<u16>,
    /// Cells to the nearest divergent boundary.
    pub div_dist: Vec<u16>,
    /// Cells to the nearest boundary of ANY type — the craton discriminator.
    pub bdy_dist: Vec<u16>,
    /// Cells to the nearest volcanic cell.
    pub volc_dist: Vec<u16>,
    /// Cells to the nearest orogen cell — drives contact metamorphism.
    pub orogen_dist: Vec<u16>,
    /// Local relief (elevation range in a 5×5 window), 0..1.
    pub relief: Vec<f32>,
    /// Land-elevation percentiles for this world, so a model's "high" is relative
    /// to the world it is on rather than an absolute number that empties on a flat
    /// map (the bug `highland_cap` was added to fix).
    pub elev_p40: f32,
    pub elev_p70: f32,
    pub elev_p90: f32,
    /// False on template/painted worlds, where every tectonic model must fall back
    /// to a relief proxy instead of scoring zero everywhere.
    pub have_plates: bool,
    /// Per-cell "is this a river cell", for placers and bog ore.
    pub river: Vec<u8>,
}

/// Multi-source BFS over land+sea with cylindrical X wrap and clamped Y (rule 6).
/// `pub(crate)` so `localities.rs` (GOODS_LOCALITIES_PLAN.md) reuses the same
/// linear-sweep primitive rather than a second per-cell outward scan (§8.9 rule 1).
pub(crate) fn bfs_dist(seeds: &[usize], w: u32, h: u32) -> Vec<u16> {
    let n = (w as usize) * (h as usize);
    let mut d = vec![UNREACHED; n];
    if seeds.is_empty() {
        return d;
    }
    let mut queue: Vec<usize> = Vec::with_capacity(seeds.len() * 4);
    for &s in seeds {
        if s < n && d[s] != 0 {
            d[s] = 0;
            queue.push(s);
        }
    }
    let mut head = 0usize;
    while head < queue.len() {
        let i = queue[head];
        head += 1;
        let di = d[i];
        if di == UNREACHED - 1 {
            continue;
        }
        let x = (i % w as usize) as i32;
        let y = (i / w as usize) as i32;
        for dy in -1i32..=1 {
            let ny = y + dy;
            if ny < 0 || ny >= h as i32 {
                continue;
            }
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = ((x + dx) % w as i32 + w as i32) % w as i32;
                let ni = (ny as usize) * (w as usize) + nx as usize;
                if d[ni] == UNREACHED {
                    d[ni] = di + 1;
                    queue.push(ni);
                }
            }
        }
    }
    d
}

/// A proximity ramp: 1.0 at the seed, falling to 0 at `reach` cells.
#[inline]
pub(crate) fn near(dist: u16, reach: f32) -> f32 {
    if dist == UNREACHED {
        return 0.0;
    }
    (1.0 - dist as f32 / reach.max(1.0)).clamp(0.0, 1.0)
}

/// A far-ness ramp: 0 at the seed, 1.0 at or beyond `reach` cells. An unreached
/// cell is maximally far, which is the correct reading for a craton on a world
/// with no boundaries at all.
#[inline]
fn far(dist: u16, reach: f32) -> f32 {
    if dist == UNREACHED {
        return 1.0;
    }
    (dist as f32 / reach.max(1.0)).clamp(0.0, 1.0)
}

fn percentile(sorted_src: &[f32], p: f32) -> f32 {
    if sorted_src.is_empty() {
        return 0.0;
    }
    let k = (((sorted_src.len() - 1) as f32) * p.clamp(0.0, 1.0)) as usize;
    sorted_src[k]
}

impl GeoContext {
    /// Build every shared field. `rivers` comes from phase 5, which runs BEFORE
    /// phase 8 — that ordering is what makes placer deposits possible at all.
    pub fn build(buf: &WorldBuffer, rivers: &[River]) -> GeoContext {
        let w = buf.width;
        let h = buf.height;
        let n = buf.total();
        let have_plates = buf.boundary_type.len() == n;

        // ── Elevation percentiles of THIS world's land ──
        let mut land_elev: Vec<f32> = (0..n)
            .filter(|&i| buf.terrain[i] == 1)
            .map(|i| buf.elevation[i])
            .collect();
        land_elev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let elev_p40 = percentile(&land_elev, 0.40);
        let elev_p70 = percentile(&land_elev, 0.70);
        let elev_p90 = percentile(&land_elev, 0.90);
        drop(land_elev);

        // ── Local relief: elevation range in a 5×5 window, separable min/max so it
        //    stays two linear passes rather than a 25-tap gather per cell.
        let mut relief = vec![0.0f32; n];
        {
            let r = 2i32;
            let mut row_min = vec![0.0f32; n];
            let mut row_max = vec![0.0f32; n];
            for y in 0..h {
                for x in 0..w {
                    let i = buf.idx(x, y);
                    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                    for dx in -r..=r {
                        let nx = buf.wrap_x(x as i32 + dx);
                        let e = buf.elevation[buf.idx(nx, y)];
                        lo = lo.min(e);
                        hi = hi.max(e);
                    }
                    row_min[i] = lo;
                    row_max[i] = hi;
                }
            }
            for y in 0..h {
                for x in 0..w {
                    let i = buf.idx(x, y);
                    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                    for dy in -r..=r {
                        let ny = buf.clamp_y(y as i32 + dy);
                        let j = buf.idx(x, ny);
                        lo = lo.min(row_min[j]);
                        hi = hi.max(row_max[j]);
                    }
                    relief[i] = (hi - lo).max(0.0);
                }
            }
        }

        // ── Boundary / volcano seeds ──
        let mut conv_seeds = Vec::new();
        let mut div_seeds = Vec::new();
        let mut bdy_seeds = Vec::new();
        let mut volc_seeds = Vec::new();
        if have_plates {
            for i in 0..n {
                match buf.boundary_type[i] {
                    BOUNDARY_CONVERGENT => {
                        conv_seeds.push(i);
                        bdy_seeds.push(i);
                    }
                    BOUNDARY_DIVERGENT => {
                        div_seeds.push(i);
                        bdy_seeds.push(i);
                    }
                    0 => {}
                    _ => bdy_seeds.push(i),
                }
            }
        }
        if buf.is_volcanic.len() == n {
            for i in 0..n {
                if buf.is_volcanic[i] == 1 {
                    volc_seeds.push(i);
                }
            }
        }

        // TEMPLATE-WORLD FALLBACK. With no plate data, a convergent margin is
        // approximated by the high-relief spine of the land (that is, after all,
        // what a collisional orogen leaves behind) and a divergent one by the
        // low-relief interior lowlands. Nothing vanishes; it is just less sharp.
        if !have_plates {
            for i in 0..n {
                if buf.terrain[i] != 1 {
                    continue;
                }
                if relief[i] > 0.055 && buf.elevation[i] >= elev_p70 {
                    conv_seeds.push(i);
                    bdy_seeds.push(i);
                }
            }
        }

        // ── Orogen cells: high, rugged land. Contact metamorphism hangs off these.
        let orogen_seeds: Vec<usize> = (0..n)
            .filter(|&i| buf.terrain[i] == 1 && buf.elevation[i] >= elev_p70 && relief[i] > 0.03)
            .collect();

        // ── River mask ──
        let mut river = vec![0u8; n];
        for r in rivers {
            for &(x, y) in &r.points {
                if x < w && y < h {
                    river[(y as usize) * (w as usize) + x as usize] = 1;
                }
            }
        }

        GeoContext {
            conv_dist: bfs_dist(&conv_seeds, w, h),
            div_dist: bfs_dist(&div_seeds, w, h),
            bdy_dist: bfs_dist(&bdy_seeds, w, h),
            volc_dist: bfs_dist(&volc_seeds, w, h),
            orogen_dist: bfs_dist(&orogen_seeds, w, h),
            relief,
            elev_p40,
            elev_p70,
            elev_p90,
            have_plates,
            river,
        }
    }

    #[inline]
    fn d(field: &[u16], i: usize) -> u16 {
        field.get(i).copied().unwrap_or(UNREACHED)
    }

    /// How well cell `i` fits `model`, 0..1.
    pub fn setting_score(&self, buf: &WorldBuffer, model: DepositModel, i: usize) -> f32 {
        self.setting_score_ex(buf, model, i, self.have_plates)
    }

    /// How well cell `i` fits `model`, 0..1. This is the geological heart of the
    /// module: everything else is placement bookkeeping.
    ///
    /// Reach numbers are in CELLS and are stated as real distances in the comments;
    /// at the standard grid a cell is ~11 km.
    ///
    /// `use_plates` is normally `self.have_plates`. The placer forces it to `false`
    /// when a world's plate geometry yields no ground at all for a model — a real
    /// case, not a hypothetical: most divergent boundaries on any Earth-like world
    /// are OCEANIC, so a continental-rift mineral keyed strictly on the boundary
    /// would silently vanish. Degrading to the relief proxy is the same discipline
    /// `highland_cap` already applies to the old elevation floor.
    pub fn setting_score_ex(
        &self,
        buf: &WorldBuffer,
        model: DepositModel,
        i: usize,
        use_plates: bool,
    ) -> f32 {
        if buf.terrain.get(i).copied().unwrap_or(0) != 1 {
            // Only CoastalMarine lives off land.
            if model != DepositModel::CoastalMarine {
                return 0.0;
            }
        }
        let elev = buf.elevation.get(i).copied().unwrap_or(0.0);
        let relief = self.relief.get(i).copied().unwrap_or(0.0);

        match model {
            // Arc magmatism sits 100–300 km behind the trench, so ~25 cells, and is
            // marked by volcanism above all else.
            DepositModel::VolcanicArc => {
                let volc = near(Self::d(&self.volc_dist, i), 12.0);
                let conv = near(Self::d(&self.conv_dist, i), 25.0);
                // A RELIEF floor applies on BOTH paths now. On a plate world whose
                // continents happen to have no on-land subduction arc (common — most
                // convergent margins are offshore, and volcanism is painted at only
                // ~8% of convergent cells), the pure volc+conv score is ≈0 everywhere,
                // so copper/silver/mercury/alum/sapphire silently vanished. Giving arc
                // minerals a rugged-upland floor (geology stays PRIMARY — the floor is
                // lower than the arc signal) means they always find plausible ground.
                let relief_floor = relief.min(0.08) / 0.08;
                let base = if use_plates {
                    (volc * 0.65 + conv * 0.35).min(1.0).max(relief_floor * 0.30)
                } else {
                    (volc * 0.7 + conv * 0.3).max(relief_floor * 0.45)
                };
                base * (0.55 + 0.45 * ((elev - self.elev_p40) / 0.3).clamp(0.0, 1.0))
            }

            // Collision: high, rugged, convergent — and explicitly NOT volcanic,
            // which is what separates a Cornwall from a Potosí.
            DepositModel::CollisionalOrogen => {
                let high = ((elev - self.elev_p40) / (self.elev_p90 - self.elev_p40).max(0.02))
                    .clamp(0.0, 1.0);
                let rugged = (relief / 0.06).clamp(0.0, 1.0);
                let conv = if use_plates {
                    0.35 + 0.65 * near(Self::d(&self.conv_dist, i), 40.0)
                } else {
                    1.0
                };
                let not_volcanic = 1.0 - 0.45 * near(Self::d(&self.volc_dist, i), 8.0);
                (high * 0.55 + rugged * 0.45) * conv * not_volcanic
            }

            // A craton is OLD, STABLE and FLAT: far from every boundary, low relief,
            // continental interior. This is the exact inverse of the elevation floor
            // that used to place diamond on the highest peaks.
            DepositModel::Craton => {
                let stable = if use_plates {
                    far(Self::d(&self.bdy_dist, i), 55.0)
                } else {
                    1.0 - (relief / 0.05).clamp(0.0, 1.0)
                };
                let flat = 1.0 - (relief / 0.045).clamp(0.0, 1.0);
                let interior = buf
                    .distance_to_ocean
                    .get(i)
                    .map(|&d| (d / 0.25).clamp(0.0, 1.0))
                    .unwrap_or(0.5);
                stable * (0.45 + 0.35 * flat + 0.20 * interior)
            }

            // Continental rift: divergent, moderate elevation, flood basalt country.
            // TWO extensional settings, and a given world may have either or
            // neither. An ACTIVE rift sits on a divergent boundary — the Rhine
            // graben, the East African rift, and the Kupferschiefer's Permian
            // basin. A FLOOD BASALT province — the Deccan, the Paraná, the
            // Siberian Traps, which between them host most of the world's agate and
            // amethyst — is a broad low-relief volcanic plateau with no present-day
            // boundary under it at all.
            //
            // Scoring this on the divergent boundary ALONE is a trap: on any
            // Earth-like world most divergent boundaries are OCEANIC, so a
            // continental-rift mineral would place nothing and silently vanish.
            // That is not hypothetical — it is what the first cut of this module
            // did, and `no_shipped_mineral_places_nothing` caught it.
            DepositModel::Rift => {
                let active = if use_plates {
                    near(Self::d(&self.div_dist, i), 22.0)
                } else {
                    0.0
                };
                let flat = 1.0 - (relief / 0.05).clamp(0.0, 1.0);
                // Traps: volcanic, flat, and away from any collision front.
                let away = if use_plates {
                    1.0 - near(Self::d(&self.conv_dist, i), 15.0)
                } else {
                    1.0
                };
                let traps = near(Self::d(&self.volc_dist, i), 20.0) * flat * away;
                // A basalt plateau that has lost its volcanic signature still reads
                // as a broad flat upland — a fair description of the Deccan today.
                let plateau = flat * ((elev - self.elev_p40) / 0.22).clamp(0.0, 1.0) * away;
                active.max(traps).max(plateau * 0.6)
            }

            // Old shallow sea: low, flat, and not deep in a continental interior.
            DepositModel::CarbonatePlatform => {
                let low = 1.0 - ((elev - self.elev_p40) / 0.25).clamp(0.0, 1.0);
                let flat = 1.0 - (relief / 0.04).clamp(0.0, 1.0);
                let shelfy = buf
                    .distance_to_ocean
                    .get(i)
                    .map(|&d| 1.0 - (d / 0.55).clamp(0.0, 1.0))
                    .unwrap_or(0.5);
                low * (0.45 + 0.35 * flat + 0.20 * shelfy)
            }

            // Platform carbonate NEXT TO an orogen — limestone that got cooked.
            // Carrara is exactly this: Apuan marble is metamorphosed platform
            // limestone.
            DepositModel::ContactMetamorphic => {
                let platform = self.setting_score_ex(buf, DepositModel::CarbonatePlatform, i, use_plates);
                let contact = near(Self::d(&self.orogen_dist, i), 14.0);
                // A pure platform far from any heat source yields nothing.
                (platform * 0.45 + 0.55) * contact * (0.35 + 0.65 * platform.max(0.25))
            }

            // Arid, low, and poorly drained. Köppen B is the whole signal; a
            // restricted basin is approximated by low relief and distance inland.
            DepositModel::EvaporiteBasin => {
                let kp = buf.koppen.get(i).copied().unwrap_or(0);
                // Köppen group B (arid) codes; anything else is a weak candidate.
                let arid = if is_arid_koppen(kp) {
                    1.0
                } else {
                    let p = buf.precipitation.get(i).copied().unwrap_or(800.0);
                    (1.0 - (p / 700.0).clamp(0.0, 1.0)) * 0.55
                };
                let low = 1.0 - ((elev - self.elev_p40) / 0.30).clamp(0.0, 1.0);
                let flat = 1.0 - (relief / 0.04).clamp(0.0, 1.0);
                arid * (0.40 + 0.35 * low + 0.25 * flat)
            }

            // Waterlogged lowland: wet, flat, low, and near standing or running
            // water. Bog ore forms in marshes, not on hills.
            DepositModel::Bog => {
                let p = buf.precipitation.get(i).copied().unwrap_or(0.0);
                let wet = ((p - 500.0) / 700.0).clamp(0.0, 1.0);
                let low = 1.0 - ((elev - self.elev_p40) / 0.20).clamp(0.0, 1.0);
                let flat = 1.0 - (relief / 0.025).clamp(0.0, 1.0);
                let watered = if self.river.get(i).copied().unwrap_or(0) == 1 {
                    1.0
                } else {
                    0.45
                };
                wet * low * (0.35 + 0.65 * flat) * watered
            }

            // Shore and shelf.
            DepositModel::CoastalMarine => {
                let coastal = buf
                    .distance_to_ocean
                    .get(i)
                    .map(|&d| 1.0 - (d / 0.05).clamp(0.0, 1.0))
                    .unwrap_or(0.0);
                let shelf = buf.is_shelf.get(i).copied().unwrap_or(0) == 1;
                if buf.terrain.get(i).copied().unwrap_or(0) == 0 {
                    if shelf {
                        0.85
                    } else {
                        0.15
                    }
                } else {
                    coastal
                }
            }

            // Derived models are never scored from a field.
            DepositModel::Placer | DepositModel::Weathering => 0.0,
        }
    }
}

/// Köppen group B (arid/semi-arid) zone codes, per `step4_climate/koppen.rs`.
/// Kept as a small local helper so this module does not depend on the classifier's
/// internal numbering beyond the group test.
fn is_arid_koppen(code: u8) -> bool {
    // BWh BWk BSh BSk occupy codes 4..=7 in the shipped 31-zone table.
    (4..=7).contains(&code)
}

// ── Placement ───────────────────────────────────────────────────────────────

/// Minimum spacing between two ORE DISTRICTS of the same mineral. The old code
/// used `width * 0.025` ≈ 1000 km between single cells, which is why deposits read
/// as isolated dots rather than as camps. Real belts hold districts a few hundred
/// km apart: the Erzgebirge is ~150 km end to end and holds many.
const MIN_DISTRICT_SEP_KM: f32 = 320.0;
/// Radius, in km, over which one district scatters its workings. A real camp is
/// 10–60 km across.
const DISTRICT_RADIUS_KM: f32 = 45.0;
const KM_EQUATOR: f32 = 40075.0;

/// How many workings a district holds, by how strong the district is.
const WORKINGS_MIN: u32 = 2;
const WORKINGS_MAX: u32 = 8;

pub(crate) fn hash01(mut x: u64) -> f32 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CEB9FE1A85EC53);
    x ^= x >> 33;
    ((x >> 40) as f32) / 16_777_216.0
}

/// Everything the placer needs about one mineral. Built by the caller from the
/// good's `GoodSpec` so this module never has to know about spec shape.
pub struct MineralPlan<'a> {
    pub id: &'a str,
    pub model: DepositModel,
    /// Fraction of districts placed as river placers below a lode district.
    pub placer_frac: f32,
    /// Parent good id for a `Weathering` mineral.
    pub parent: Option<&'a str>,
    /// Number of ORE DISTRICTS to try to place.
    pub districts: u32,
    /// Deterministic per-mineral salt.
    pub salt: u64,
    /// 0..1 — rarer minerals demand a stronger setting and grade poorer on average
    /// at the margins.
    pub rarity: f32,
}

/// Result of placing one mineral: the u8 belt column (unchanged in shape, so every
/// existing reader and the v2 blob format are untouched) plus the discrete
/// workings.
pub struct Placement {
    pub belt: Vec<u8>,
    pub deposits: Vec<Deposit>,
}

/// Place one mineral as districts of workings.
///
/// `parent_deposits` supplies the already-placed workings of a `Weathering`
/// mineral's parent; pass an empty slice otherwise.
pub fn place_mineral(
    buf: &WorldBuffer,
    ctx: &GeoContext,
    rivers: &[River],
    plan: &MineralPlan,
    parent_deposits: &[Deposit],
    seed: u64,
) -> Placement {
    let w = buf.width;
    let h = buf.height;
    let n = buf.total();
    let mut belt = vec![0u8; n];
    let mut out: Vec<Deposit> = Vec::new();
    if plan.districts == 0 || n == 0 {
        return Placement { belt, deposits: out };
    }
    let gs = seed ^ plan.salt;
    let km_per_cell = KM_EQUATOR / w.max(1) as f32;
    let sep_cells = (MIN_DISTRICT_SEP_KM / km_per_cell).max(3.0);
    let radius_cells = ((DISTRICT_RADIUS_KM / km_per_cell).round() as i32).max(1);

    // ── A WEATHERING mineral is a daughter of its parent: it forms where the
    //    parent orebody was exposed to an arid climate. No independent search.
    if plan.model == DepositModel::Weathering {
        for (k, p) in parent_deposits.iter().enumerate() {
            let i = (p.y as usize) * (w as usize) + p.x as usize;
            if i >= n {
                continue;
            }
            let kp = buf.koppen.get(i).copied().unwrap_or(0);
            if !is_arid_koppen(kp) {
                continue;
            }
            // Only a fraction of exposed parent bodies alter to anything worth
            // digging, and only the ones near enough to the surface.
            if p.depth > DEPTH_SHALLOW {
                continue;
            }
            if hash01(gs ^ (k as u64).wrapping_mul(0x9E3779B97F4A7C15)) > 0.35 {
                continue;
            }
            // Supergene enrichment sits ABOVE the parent, so it is by definition a
            // surface deposit and it is small.
            let grade = (0.45 + 0.45 * hash01(gs ^ 0xA17E ^ (k as u64) << 7)).clamp(0.0, 1.0);
            let dep = Deposit {
                good: plan.id.to_string(),
                x: p.x,
                y: p.y,
                district: p.district,
                grade,
                extent: if hash01(gs ^ 0xE5E5 ^ (k as u64) << 3) < 0.7 {
                    EXTENT_WEAK
                } else {
                    EXTENT_MODERATE
                },
                depth: DEPTH_SURFACE,
                model: DepositModel::Weathering,
            };
            write_belt(&mut belt, i, &dep);
            out.push(dep);
        }
        return Placement { belt, deposits: out };
    }

    // ── Score every land cell for this mineral's setting, modulated by the
    //    mineral's own metallogenic-belt noise so two minerals sharing a model do
    //    not stack on the identical ground. The BELT is the 100–1000 km scale; the
    //    setting decides which belts are even possible.
    let belt_scale = 0.045;
    let mut cand = vec![0.0f32; n];
    let build = |cand: &mut Vec<f32>, use_plates: bool| -> usize {
        let mut live = 0usize;
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                let s = ctx.setting_score_ex(buf, plan.model, i, use_plates);
                if s <= 0.02 {
                    cand[i] = 0.0;
                    continue;
                }
                let p = crate::sim::elevation::fbm_noise(
                    x as f32 * belt_scale,
                    y as f32 * belt_scale,
                    plan.salt,
                    4,
                    2.0,
                    0.5,
                );
                // The setting gates; the belt noise selects WHICH of the eligible
                // ground actually hosts a province of this mineral.
                cand[i] = s * (0.35 + 0.65 * p.clamp(0.0, 1.0));
                live += 1;
            }
        }
        live
    };
    // NEVER LET A MINERAL SILENTLY VANISH. If this world's plate geometry offers
    // no ground at all for the model — a real case, since most divergent
    // boundaries are oceanic and a world may have no continental rift whatsoever —
    // fall back to the relief proxy, which is the same path a template world with
    // no plate data takes and which is covered by its own test. Geology stays the
    // PRIMARY signal; it degrades only when the world genuinely lacks the setting.
    let want_cells = (plan.districts as usize * 6).max(12);
    if build(&mut cand, ctx.have_plates) < want_cells && ctx.have_plates {
        build(&mut cand, false);
    }

    // Rarer minerals demand a stronger candidate, so prized stones stay scarce.
    let mut thresh = 0.30 + (plan.rarity - 0.5).max(0.0) * 0.35;
    let want = want_cells;
    while thresh > 0.06 && cand.iter().filter(|&&v| v >= thresh).count() < want {
        thresh -= 0.03;
    }

    // Rank candidates deterministically (weighted random key on the score).
    let mut ranked: Vec<(usize, f32)> = Vec::new();
    for i in 0..n {
        if cand[i] >= thresh {
            let key = (cand[i] + 0.05) * hash01(gs ^ (i as u64).wrapping_mul(0x100000001B3));
            ranked.push((i, key));
        }
    }
    if ranked.is_empty() {
        // GUARANTEE placement (#10): this world's geology offered no cell above even
        // the lowest threshold for this mineral's model, so a shipped stone would
        // vanish from the ENTIRE world. Rather than return empty, place a few
        // districts on the most model-plausible LAND available — the raw setting
        // score with NO floor (best of the plate and relief-proxy readings), keeping
        // only the top band so the fallback still lands on the right KIND of ground
        // (diamond keeps preferring low-relief interior, so the craton test holds) and
        // stays scarce. Only reached when normal placement finds nothing.
        let mut relaxed = vec![0.0f32; n];
        let mut maxs = 0.0f32;
        for i in 0..n {
            if buf.terrain[i] != 1 { continue; }
            let s = ctx.setting_score_ex(buf, plan.model, i, ctx.have_plates)
                .max(ctx.setting_score_ex(buf, plan.model, i, false));
            relaxed[i] = s;
            if s > maxs { maxs = s; }
        }
        let floor = (maxs * 0.5).max(0.001);
        for i in 0..n {
            if buf.terrain[i] == 1 && relaxed[i] >= floor {
                cand[i] = relaxed[i].max(0.05);
                let key = (cand[i] + 0.05)
                    * hash01(gs ^ 0xFA11 ^ (i as u64).wrapping_mul(0x100000001B3));
                ranked.push((i, key));
            }
        }
        if ranked.is_empty() {
            return Placement { belt, deposits: out };
        }
        // The per-working local gate is relative to `thresh`; drop it so a forced
        // district's workings actually get written on this indifferent ground.
        thresh = 0.0;
    }
    ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    let wrap_dx = |a: i32, b: i32| -> i32 {
        let mut d = (a - b).abs();
        if d > w as i32 / 2 {
            d = w as i32 - d;
        }
        d
    };

    // ── District centres, spaced. ──
    let mut centers: Vec<(i32, i32, f32)> = Vec::new();
    for (i, _) in &ranked {
        if centers.len() as u32 >= plan.districts {
            break;
        }
        let cx = (*i as i32) % w as i32;
        let cy = (*i as i32) / w as i32;
        let far_enough = centers.iter().all(|&(qx, qy, _)| {
            let dx = wrap_dx(cx, qx) as f32;
            let dy = (cy - qy) as f32;
            (dx * dx + dy * dy).sqrt() >= sep_cells
        });
        if !far_enough {
            continue;
        }
        centers.push((cx, cy, cand[*i]));
    }

    // How many of these districts are ALLUVIAL rather than in situ.
    let placer_n = ((centers.len() as f32) * plan.placer_frac.clamp(0.0, 1.0)).round() as usize;

    // ── Workings within each district. ──
    for (di, &(cx, cy, cscore)) in centers.iter().enumerate() {
        let dsalt = gs ^ (di as u64).wrapping_mul(0x2545F4914F6CDD1D);
        // A richer district supports more workings — a real camp's size tracks how
        // much there was to find.
        let span = (WORKINGS_MAX - WORKINGS_MIN) as f32;
        let count = WORKINGS_MIN
            + ((cscore.clamp(0.0, 1.0) * 0.6 + hash01(dsalt ^ 0x11) * 0.4) * span).round() as u32;

        // Alluvial districts are handled after the lode ones, so their parent
        // exists to walk down from.
        if di >= centers.len() - placer_n {
            continue;
        }

        for k in 0..count {
            let ks = dsalt ^ (k as u64).wrapping_mul(0x9E3779B97F4A7C15);
            // Scatter within the camp. First working sits on the centre.
            let (ox, oy) = if k == 0 {
                (0i32, 0i32)
            } else {
                let ang = hash01(ks ^ 0xA1) * std::f32::consts::TAU;
                let rad = hash01(ks ^ 0xB2).sqrt() * radius_cells as f32;
                ((ang.cos() * rad).round() as i32, (ang.sin() * rad).round() as i32)
            };
            let wy = cy + oy;
            if wy < 0 || wy >= h as i32 {
                continue;
            }
            let wx = buf.wrap_x(cx + ox);
            let i = buf.idx(wx, wy as u32);
            // A working must still sit on ground the model accepts — a camp does
            // not spill into the sea or onto terrain of the wrong kind.
            let local = ctx.setting_score(buf, plan.model, i);
            if local < thresh * 0.45 {
                continue;
            }
            if belt[i] != 0 {
                continue;
            }
            let dep = make_working(plan, di as u32, wx, wy as u32, local, cscore, ks);
            write_belt(&mut belt, i, &dep);
            out.push(dep);
        }
    }

    // ── Alluvial districts: walk downstream from a lode working. ──
    if placer_n > 0 && !out.is_empty() {
        let lode: Vec<Deposit> = out.clone();
        for pi in 0..placer_n {
            let ps = gs ^ 0x9AC1_0000 ^ (pi as u64).wrapping_mul(0x100000001B3);
            let parent = &lode[(hash01(ps) * lode.len() as f32) as usize % lode.len()];
            let trail = downstream_cells(buf, ctx, rivers, parent.x, parent.y, 60);
            if trail.is_empty() {
                continue;
            }
            let count = WORKINGS_MIN + (hash01(ps ^ 0x77) * 4.0).round() as u32;
            for k in 0..count {
                let ks = ps ^ (k as u64).wrapping_mul(0x9E3779B97F4A7C15);
                let t = trail[(hash01(ks ^ 0xC3) * trail.len() as f32) as usize % trail.len()];
                if belt[t] != 0 {
                    continue;
                }
                let x = (t % w as usize) as u32;
                let y = (t / w as usize) as u32;
                // Gravel is concentrated by the river, so grade can be HIGHER than
                // the lode it came from — which is precisely why placers were
                // worked first.
                let grade = (parent.grade * (0.85 + 0.35 * hash01(ks ^ 0xD4))).clamp(0.0, 1.0);
                let dep = Deposit {
                    good: plan.id.to_string(),
                    x,
                    y,
                    district: parent.district,
                    grade,
                    extent: if hash01(ks ^ 0xE5) < 0.75 {
                        EXTENT_WEAK
                    } else {
                        EXTENT_MODERATE
                    },
                    // A placer is by definition at the surface. This is the whole
                    // reason it is the deposit a world discovers first.
                    depth: DEPTH_SURFACE,
                    model: DepositModel::Placer,
                };
                write_belt(&mut belt, t, &dep);
                out.push(dep);
            }
        }
    }

    Placement { belt, deposits: out }
}

/// Roll one working's grade, extent and depth.
fn make_working(
    plan: &MineralPlan,
    district: u32,
    x: u32,
    y: u32,
    local: f32,
    cscore: f32,
    ks: u64,
) -> Deposit {
    // Grade follows the local setting fit — a camp is richest at its centre and
    // poorer at the margins, which is what zoning around an intrusion looks like.
    // v2.0 · baseline and floor both raised (was 0.25 base / 0.05 floor): a working
    // that clears every placement gate (real tectonic setting, real proximity, real
    // grade fit) is meant to be worth digging, not a coin flip against being a scrap
    // deposit — the gates already do the "does this exist at all" job (§8.16's five
    // rules), so grade no longer needs to re-litigate that with a low floor too.
    let fit = (local * 0.6 + cscore * 0.4).clamp(0.0, 1.0);
    let grade = (0.42 + 0.55 * fit + 0.14 * (hash01(ks ^ 0x51) - 0.5)).clamp(0.20, 1.0);

    // Extent: most bodies are small. A world-class one is rare and is the reason a
    // Potosí or a Rammelsberg exists at all. v2.0 · thresholds lowered so a well-fit
    // working is more often a real body worth working, not a pocket — still weighted
    // toward WEAK/MODERATE at a poor-fit margin (the `fit` term still narrows the
    // random draw there), so a camp's own centre-to-edge zoning is unchanged.
    let r = hash01(ks ^ 0x62) * (0.70 + 0.30 * fit);
    let extent = if r > 0.85 {
        EXTENT_WORLD_CLASS
    } else if r > 0.62 {
        EXTENT_GREAT
    } else if r > 0.32 {
        EXTENT_MODERATE
    } else {
        EXTENT_WEAK
    };

    // Depth: most of what a pre-modern world could find was what broke surface.
    // Rarer minerals skew deeper, because the shallow ones were found first.
    let dr = hash01(ks ^ 0x73) + (plan.rarity - 0.5) * 0.22;
    let depth = if dr < 0.34 {
        DEPTH_SURFACE
    } else if dr < 0.66 {
        DEPTH_SHALLOW
    } else if dr < 0.88 {
        DEPTH_DEEP
    } else {
        DEPTH_FLOODED
    };

    Deposit {
        good: plan.id.to_string(),
        x,
        y,
        district,
        grade,
        extent,
        depth,
        model: plan.model,
    }
}

#[inline]
fn write_belt(belt: &mut [u8], i: usize, dep: &Deposit) {
    // Floor at 1: a working that is genuinely PLACED here must register a nonzero
    // belt byte, or a deep/low-grade deposit rounds to 0 and the placement report
    // reads the good as "absent" even though the geology put it on the map.
    let v = ((dep.workable_intensity() * 255.0) as u8).max(1);
    if v > belt[i] {
        belt[i] = v;
    }
}

/// Cells along a river below `(sx, sy)`, up to `max_steps`.
///
/// River point lists carry no guaranteed direction, so rather than trusting the
/// storage order this walks the polyline OUTWARD from the nearest point in both
/// directions and keeps whichever way loses elevation. That is robust to how
/// `extract_rivers` happens to store a course.
fn downstream_cells(
    buf: &WorldBuffer,
    ctx: &GeoContext,
    rivers: &[River],
    sx: u32,
    sy: u32,
    max_steps: usize,
) -> Vec<usize> {
    let w = buf.width as i32;
    let _ = ctx;
    // Nearest river point to the lode, within a plausible catchment.
    let mut best: Option<(usize, usize, i64)> = None; // (river, point index, d2)
    for (ri, r) in rivers.iter().enumerate() {
        for (pi, &(px, py)) in r.points.iter().enumerate() {
            let mut dx = (px as i32 - sx as i32).abs();
            if dx > w / 2 {
                dx = w - dx;
            }
            let dy = py as i32 - sy as i32;
            let d2 = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
            if best.map(|(_, _, bd)| d2 < bd).unwrap_or(true) {
                best = Some((ri, pi, d2));
            }
        }
    }
    // A lode with no river within ~30 cells sheds no placer.
    let Some((ri, pi, d2)) = best else {
        return Vec::new();
    };
    if d2 > 900 {
        return Vec::new();
    }
    let pts = &rivers[ri].points;
    let at = |k: usize| -> usize {
        let (x, y) = pts[k];
        (y as usize) * (buf.width as usize) + x as usize
    };
    let elev_at = |k: usize| buf.elevation.get(at(k)).copied().unwrap_or(0.0);
    let here = elev_at(pi);
    // Which end of the polyline is downhill?
    let fwd_lower = pts.len() > pi + 1 && elev_at(pts.len() - 1) < here;
    let bwd_lower = pi > 0 && elev_at(0) < here;
    let mut out = Vec::new();
    if fwd_lower {
        for k in (pi + 1)..pts.len().min(pi + 1 + max_steps) {
            out.push(at(k));
        }
    } else if bwd_lower {
        for k in (pi.saturating_sub(max_steps)..pi).rev() {
            out.push(at(k));
        }
    } else {
        // Flat or single-point course: the reach around the lode is still gravel.
        let lo = pi.saturating_sub(max_steps / 2);
        let hi = pts.len().min(pi + max_steps / 2);
        for k in lo..hi {
            out.push(at(k));
        }
    }
    out.retain(|&i| buf.terrain.get(i).copied().unwrap_or(0) == 1);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world_buffer::{ColumnSet, WorldBuffer};

    /// A synthetic world with a convergent margin down one side, a volcanic arc
    /// behind it, and a wide flat interior — enough relief structure for the models
    /// to separate on.
    fn test_world(w: u32, h: u32) -> WorldBuffer {
        let n = (w * h) as usize;
        let mut buf = WorldBuffer {
            cols: ColumnSet::ALL,
            width: w,
            height: h,
            tiles_x: (w + 127) / 128,
            tiles_y: (h + 127) / 128,
            equator_offset: 0.0,
            lat_scale: 1.0,
            lat_ratio: 0.5,
            obliquity: 23.44,
            rotation_rate: 1.0,
            solar_lum: 1.0,
            greenhouse: 1.0,
            eccentricity: 0.0167,
            dryness: 1.0,
            terrain: vec![1; n],
            elevation: vec![0.05; n],
            sea_depth: vec![0.0; n],
            is_shelf: vec![0; n],
            is_shelf_edge: vec![0; n],
            locked_bits: vec![0; n],
            plate_index: vec![0; n],
            boundary_type: vec![0; n],
            is_volcanic: vec![0; n],
            temperature: vec![15.0; n],
            precipitation: vec![900.0; n],
            koppen: vec![12; n],
            soil_type: vec![0; n],
            fertility: vec![0.5; n],
            fishery: vec![0.0; n],
            current_type: vec![0; n],
            wind_vx: vec![0.0; n],
            wind_vy: vec![0.0; n],
            wind_speed: vec![0.0; n],
            current_vx: vec![0.0; n],
            current_vy: vec![0.0; n],
            distance_to_ocean: vec![0.5; n],
            habitability: vec![0.5; n],
            salinity: vec![0; n],
            shark_risk: vec![0; n],
            goods: vec![vec![0u8; n]; 1],
            shipworm_risk: vec![0; n],
            storm_base: vec![0; n],
            reef_risk: vec![0; n],
            disease_risk: vec![0; n],
            precip_summer_frac: vec![0; n],
            seasonal_amp: vec![0; n],
            sst: vec![0.0; n],
            snow_frac: vec![0; n],
            biome: vec![0; n],
        };
        for y in 0..h {
            for x in 0..w {
                let i = buf.idx(x, y);
                // Ocean on the far west, a convergent margin + mountain belt just
                // inland of it, then a wide flat craton to the east.
                if x < 6 {
                    buf.terrain[i] = 0;
                    buf.elevation[i] = 0.0;
                    buf.distance_to_ocean[i] = 0.0;
                    continue;
                }
                let d = (x - 6) as f32;
                buf.distance_to_ocean[i] = (d / 40.0).min(1.0);
                if (10..=16).contains(&x) {
                    buf.boundary_type[i] = BOUNDARY_CONVERGENT;
                }
                if (12..=14).contains(&x) && y % 3 != 0 {
                    buf.is_volcanic[i] = 1;
                }
                // A rugged belt over the margin, tapering into a flat interior.
                let ridge = (-(((x as f32) - 14.0) / 5.0).powi(2)).exp();
                let jag = if (x + y) % 2 == 0 { 0.03 } else { 0.0 };
                buf.elevation[i] = 0.04 + ridge * (0.55 + jag);
            }
        }
        buf
    }

    fn plan_for<'a>(id: &'a str, districts: u32) -> MineralPlan<'a> {
        let (model, placer_frac) = default_model_for(id);
        MineralPlan {
            id,
            model,
            placer_frac,
            parent: default_parent_for(id),
            districts,
            salt: id
                .bytes()
                .fold(0xcbf29ce484222325u64, |a, b| (a ^ b as u64).wrapping_mul(0x100000001b3)),
            rarity: 0.6,
        }
    }

    /// The headline geological claim: diamond belongs on stable cratons, not on the
    /// highest peaks. The old placer used `min_elev: 0.55`, i.e. exactly the
    /// opposite. Clifford's Rule is the most rule-like fact in ore geology and it
    /// is worth a test.
    #[test]
    fn diamond_lands_on_craton_not_on_peaks() {
        let buf = test_world(96, 48);
        let ctx = GeoContext::build(&buf, &[]);
        let p = plan_for("diamond", 6);
        let out = place_mineral(&buf, &ctx, &[], &p, &[], 1234);
        assert!(!out.deposits.is_empty(), "diamond placed nothing");
        let lode: Vec<&Deposit> = out
            .deposits
            .iter()
            .filter(|d| d.model == DepositModel::Craton)
            .collect();
        assert!(!lode.is_empty(), "no cratonic diamond workings");
        let mean_elev: f32 = lode
            .iter()
            .map(|d| buf.elevation[buf.idx(d.x, d.y)])
            .sum::<f32>()
            / lode.len() as f32;
        let peak = buf
            .elevation
            .iter()
            .cloned()
            .fold(0.0f32, f32::max);
        assert!(
            mean_elev < peak * 0.45,
            "diamond averaged {mean_elev:.3} against a peak of {peak:.3} — it is on the mountains again"
        );
    }

    /// Tin (collisional orogen) and diamond (craton) must not sit on the same
    /// ground. Under the old model every deposit good ranked by elevation, so they
    /// all piled onto the single tallest range.
    #[test]
    fn different_models_separate_minerals() {
        let buf = test_world(96, 48);
        let ctx = GeoContext::build(&buf, &[]);
        let tin = place_mineral(&buf, &ctx, &[], &plan_for("tin", 6), &[], 99);
        let dia = place_mineral(&buf, &ctx, &[], &plan_for("diamond", 6), &[], 99);
        assert!(!tin.deposits.is_empty() && !dia.deposits.is_empty());
        let mean = |ds: &[Deposit]| -> f32 {
            ds.iter().map(|d| buf.elevation[buf.idx(d.x, d.y)]).sum::<f32>() / ds.len() as f32
        };
        let (mt, md) = (mean(&tin.deposits), mean(&dia.deposits));
        assert!(
            mt > md * 1.8,
            "tin ({mt:.3}) and diamond ({md:.3}) landed on the same kind of ground"
        );
    }

    /// Workings must CLUSTER into camps. The old placer forced ~1000 km between
    /// single cells, so a district was never a thing.
    #[test]
    fn workings_cluster_into_districts() {
        let buf = test_world(128, 64);
        let ctx = GeoContext::build(&buf, &[]);
        let out = place_mineral(&buf, &ctx, &[], &plan_for("copper", 5), &[], 7);
        assert!(out.deposits.len() >= 6, "expected several workings, got {}", out.deposits.len());
        // At least one district must hold more than one working.
        let mut per_district = std::collections::BTreeMap::new();
        for d in &out.deposits {
            *per_district.entry(d.district).or_insert(0u32) += 1;
        }
        let biggest = per_district.values().cloned().max().unwrap_or(0);
        assert!(biggest >= 2, "no district held more than one working");
    }

    /// Same seed, same map — the placer uses hashing and a sort, both of which have
    /// bitten this codebase before (`provinces` ties must break on the key).
    #[test]
    fn placement_is_deterministic() {
        let buf = test_world(96, 48);
        let ctx = GeoContext::build(&buf, &[]);
        let a = place_mineral(&buf, &ctx, &[], &plan_for("silver", 5), &[], 42);
        let b = place_mineral(&buf, &ctx, &[], &plan_for("silver", 5), &[], 42);
        assert_eq!(a.belt, b.belt);
        assert_eq!(a.deposits.len(), b.deposits.len());
        for (x, y) in a.deposits.iter().zip(b.deposits.iter()) {
            assert_eq!((x.x, x.y, x.depth, x.extent), (y.x, y.y, y.depth, y.extent));
        }
    }

    /// A deep or flooded body must read WEAKER in the belt column than a surface
    /// one of the same grade — it is inventory for a mining industry that does not
    /// exist yet, not present output.
    #[test]
    fn depth_attenuates_workable_intensity() {
        let mk = |depth: u8| Deposit {
            good: "gold".into(),
            x: 0,
            y: 0,
            district: 0,
            grade: 0.9,
            extent: EXTENT_MODERATE,
            depth,
            model: DepositModel::CollisionalOrogen,
        };
        let s = mk(DEPTH_SURFACE).workable_intensity();
        let d = mk(DEPTH_DEEP).workable_intensity();
        let f = mk(DEPTH_FLOODED).workable_intensity();
        assert!(s > d && d > f, "depth must attenuate: {s} {d} {f}");
    }

    /// Every shipped deposit mineral must actually place something on a plain
    /// world. A good that silently vanishes is the failure mode this codebase has
    /// hit repeatedly (the metals disappearing under an absolute elevation floor).
    #[test]
    fn no_shipped_mineral_places_nothing() {
        let buf = test_world(128, 64);
        let ctx = GeoContext::build(&buf, &[]);
        for id in [
            "gold", "silver", "copper", "tin", "iron", "lead", "mercury", "ruby", "sapphire",
            "emerald", "diamond", "amethyst", "topaz", "marble", "jade", "gemstones",
        ] {
            let out = place_mineral(&buf, &ctx, &[], &plan_for(id, 5), &[], 5150);
            assert!(
                !out.deposits.is_empty(),
                "{id} placed no workings — it would silently vanish from the map"
            );
        }
    }

    /// Template worlds carry no plate data at all. Every model must degrade to a
    /// relief proxy rather than scoring zero everywhere.
    #[test]
    fn template_world_without_plates_still_places() {
        let mut buf = test_world(96, 48);
        buf.boundary_type.clear();
        buf.is_volcanic = vec![0; (buf.width * buf.height) as usize];
        let ctx = GeoContext::build(&buf, &[]);
        assert!(!ctx.have_plates);
        for id in ["gold", "copper", "diamond", "tin", "iron"] {
            let out = place_mineral(&buf, &ctx, &[], &plan_for(id, 4), &[], 21);
            assert!(!out.deposits.is_empty(), "{id} vanished on a template world");
        }
    }
}
