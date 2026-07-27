//! Province partition — a watershed/cost-flood administrative layer between tiles
//! and settlements (the EU4-style political/economic map). Runs AFTER the
//! settlement step (settlements seed the partition) and is a SEPARATE layer.
//!
//! The land is split into provinces whose borders follow VISIBLE natural features:
//! - **coasts / islands** — each land connected-component is partitioned on its own,
//!   so no province spans open sea;
//! - **mountain ranges** — genuinely high ground is expensive to traverse, so borders
//!   settle along mountain spines. This is NOT a watershed partition: a minor uphill
//!   step on lowlands is free, so provinces grow organically across plains until they
//!   actually meet a range (only real mountains divide, not every drainage divide);
//! - **rivers & lakes** — a wide / navigable river is expensive to cross and lakes are
//!   impassable, so both act as frontiers between neighbouring provinces;
//! - **organic noise** — a small per-edge noise term wobbles borders off any clean
//!   Voronoi/gradient line, and provinces are NOT forced simply-connected, so genuine
//!   enclaves/exclaves survive.
//!
//! Pure, deterministic, cylindrical (X wraps, Y clamps). No DB, no tile writes — the
//! caller persists the result. See `docs/PROVINCE_SYSTEM_PLAN.md`.

use crate::sim::world_buffer::WorldBuffer;
use crate::sim::rivers::{River, Lake};
use crate::sim::settlements::Settlement;
use crate::sim::{names, cultures};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// One good a province's land can yield, with an environmental-suitability QUALITY
/// (0..1) — never an amount/tonnage. Best-first.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ProvinceGood {
    pub good: u8,
    pub quality: f32,
}

/// A province: a contiguous (mostly) patch of one island's land.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Province {
    pub id: u16,
    pub name: String,              // its OWN generated name (variable length), not the seat's
    pub seat_x: u32,
    pub seat_y: u32,
    pub cells: u32,                // area in cells
    pub area_km2: u32,             // latitude-aware real area
    pub island: u32,               // land connected-component id (coast = hard border)
    pub neighbors: Vec<u16>,
    // ── geography ──
    pub koppen: u8,                // plurality climate
    pub elevation_class: u8,       // 0 lowland · 1 hill · 2 upland
    pub mean_fertility: f32,
    pub coastal: bool,
    // ── economy — WHICH goods + WHAT QUALITY (no amount) ──
    pub goods: Vec<ProvinceGood>,
    // ── people ──
    pub culture: String,           // founding plurality (campaign may shift it via migration)
    pub rural_pop: u32,            // baseline countryside population
    // ── flavour ──
    pub analog: String,            // "looks most like…" real-world regions
    // ── membership ──
    pub settlements: Vec<String>,  // settlement ids whose cell falls inside (seat first)
}

/// Sea sentinel in the per-cell province-id map.
pub const NO_PROVINCE: u16 = u16::MAX;

struct HeapItem { cost: f64, cell: u32, owner: u32 }
impl PartialEq for HeapItem { fn eq(&self, o: &Self) -> bool { self.cost == o.cost } }
impl Eq for HeapItem {}
impl Ord for HeapItem {
    // Min-heap on cost (BinaryHeap is a max-heap → invert).
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
            .then_with(|| self.cell.cmp(&o.cell))
    }
}
impl PartialOrd for HeapItem { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

#[inline]
fn hash2(a: u64, b: u64) -> u64 {
    cultures::hash64(a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
}

/// Partition all land into provinces. `granularity` 0..1: 0 = few large provinces,
/// 1 = many small ones. Returns the province list and a per-cell province-id map
/// (`NO_PROVINCE` on sea), row-major over the full world grid.
pub fn generate_provinces(
    buf: &WorldBuffer,
    rivers: &[River],
    lakes: &[Lake],
    settlements: &[Settlement],
    granularity: f32,
) -> (Vec<Province>, Vec<u16>) {
    let w = buf.width;
    let h = buf.height;
    let wi = w as i32;
    let hi = h as i32;
    let total = buf.total();
    let g = granularity.clamp(0.0, 1.0);

    // Per-cell food (rural carrying capacity) reused from the settlement model.
    let food = crate::sim::settlements::compute_food_capacity(buf, rivers);

    // ── Wide/navigable trunk-river crossing penalty per cell (a river is expensive
    //    to cross → it can become a border). ──
    let mut river_cross = vec![0.0f32; total];
    for r in rivers {
        let pen = if r.navigable { 6.0 } else { (r.width * 0.6).min(4.0) };
        if pen <= 0.0 { continue; }
        for &(rx, ry) in &r.points {
            let i = buf.idx(rx, ry);
            if pen > river_cross[i] { river_cross[i] = pen; }
        }
    }
    // Lake cells are impassable to the flood (a lake is a natural divide).
    let mut is_lake = vec![false; total];
    for lk in lakes {
        for &(lx, ly) in &lk.cells { is_lake[buf.idx(lx, ly)] = true; }
    }

    // ── Island labelling (land connected-components, 8-neighbour, X-wrap). ──
    let mut island = vec![u32::MAX; total];
    let mut n_islands = 0u32;
    let mut stack: Vec<u32> = Vec::new();
    for start in 0..total {
        if buf.terrain[start] != 1 || island[start] != u32::MAX { continue; }
        let id = n_islands; n_islands += 1;
        island[start] = id;
        stack.push(start as u32);
        while let Some(c) = stack.pop() {
            let cx = (c % w) as i32;
            let cy = (c / w) as i32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let ny = cy + dy;
                    if ny < 0 || ny >= hi { continue; }
                    let ni = buf.widx(cx + dx, ny);
                    if buf.terrain[ni] == 1 && island[ni] == u32::MAX {
                        island[ni] = id;
                        stack.push(ni as u32);
                    }
                }
            }
        }
    }

    // ── Seeds: every settlement cell + a filler grid so empty land is covered and
    //    the province count is controlled by granularity. ──
    let mut seed_cells: Vec<u32> = Vec::new();
    let mut is_seed = vec![false; total];
    for s in settlements {
        let i = buf.idx(s.x.min(w - 1), s.y.min(h - 1));
        if buf.terrain[i] == 1 && !is_seed[i] { is_seed[i] = true; seed_cells.push(i as u32); }
    }
    // Filler seeds: a JITTERED, density-varied scatter so provinces come out organic
    // (varied size + off-lattice) instead of a regular Voronoi grid of squares. Coarse
    // (g→0) gives few large provinces, fine (g→1) many.
    let cols = 12.0 + 46.0 * g;
    let spacing = ((w as f32 / cols).round() as i32).max(4);
    let half2 = (spacing / 2) * (spacing / 2);
    let jit = ((spacing as f32) * 0.42) as i64;        // seed jitter off the block centre
    let win = (spacing / 3).max(2);                     // fertile-cell search window
    let mut gy = (spacing / 2) as i32;
    while gy < hi {
        let mut gx = 0i32;
        while gx < wi {
            // Hash the block → jitter the sample centre and vary density (skip ~1 in 6
            // blocks so neighbours fuse into larger, irregular provinces — breaks the
            // uniform lattice that made everything a checkerboard of equal squares).
            let hb = hash2(gx as u64, (gy as u64) ^ 0x51ED_A5A5);
            let span = (2 * jit + 1) as u64;
            let jx = (hb % span) as i64 - jit;
            let jy = ((hb >> 21) % span) as i64 - jit;
            if (hb >> 42) % 6 == 0 { gx += spacing; continue; }   // density variation
            let cxb = gx + jx as i32;
            let cyb = gy + jy as i32;
            // Pick the most fertile land cell in a SMALL window around the jittered
            // centre (keeps the seed near the jittered point → off the lattice).
            let mut best = (u32::MAX, -1.0f32);
            for oy in -win..=win {
                let cy = cyb + oy;
                if cy < 0 || cy >= hi { continue; }
                for ox in -win..=win {
                    let ci = buf.widx(cxb + ox, cy);
                    if buf.terrain[ci] != 1 { continue; }
                    let sc = buf.fertility[ci] + food[ci] * 0.01;
                    if sc > best.1 { best = (ci as u32, sc); }
                }
            }
            if best.0 != u32::MAX {
                let bi = best.0 as usize;
                // Skip if a settlement seed is already very close (keep towns as seats).
                let bx = (bi as u32 % w) as i32;
                let by = (bi as u32 / w) as i32;
                let mut near = false;
                for &sc in &seed_cells {
                    let sx = (sc % w) as i32; let sy = (sc / w) as i32;
                    let mut ddx = (sx - bx).abs(); if ddx > wi / 2 { ddx = wi - ddx; }
                    let dd = ddx * ddx + (sy - by) * (sy - by);
                    if dd < half2 { near = true; break; }
                }
                if !near && !is_seed[bi] { is_seed[bi] = true; seed_cells.push(bi as u32); }
            }
            gx += spacing;
        }
        gy += spacing;
    }
    if seed_cells.is_empty() { return (Vec::new(), vec![NO_PROVINCE; total]); }

    // ── Multi-source cost-flood (Dijkstra) over land. ──
    let mut owner = vec![u32::MAX; total];
    let mut dist = vec![f64::INFINITY; total];
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
    for (oi, &sc) in seed_cells.iter().enumerate() {
        owner[sc as usize] = oi as u32;
        dist[sc as usize] = 0.0;
        heap.push(HeapItem { cost: 0.0, cell: sc, owner: oi as u32 });
    }
    // Mountains are the divider, NOT watersheds: crossing genuinely high ground costs
    // extra (borders settle on ranges), but a minor uphill step on lowlands is free (so
    // provinces grow organically across plains until they hit a river, lake, or range).
    const K_MOUNTAIN: f64 = 18.0;    // cost per unit elevation above the range threshold
    const MOUNT_THRESH: f32 = 0.26;  // ≈2300 m — foothills begin to divide, high ranges wall
    while let Some(HeapItem { cost, cell, owner: ow }) = heap.pop() {
        let ci = cell as usize;
        if cost > dist[ci] { continue; }
        if owner[ci] != ow { continue; }
        let cx = (cell % w) as i32;
        let cy = (cell / w) as i32;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let ny = cy + dy;
                if ny < 0 || ny >= hi { continue; }
                let ni = buf.widx(cx + dx, ny);
                if buf.terrain[ni] != 1 || is_lake[ni] { continue; }
                let diag = if dx != 0 && dy != 0 { 1.4142 } else { 1.0 };
                // Absolute high-elevation cost — a mountain wall, not a drainage divide.
                // Lowlands (elev ≤ threshold) add nothing, so plains stay Voronoi-organic.
                let em = buf.elevation[ni];
                let mountain = if em > MOUNT_THRESH {
                    ((em - MOUNT_THRESH) as f64) * K_MOUNTAIN
                } else { 0.0 };
                let rivp = river_cross[ni] as f64;   // wide/navigable rivers = frontiers
                let noise = (hash2(cell as u64, ni as u64) % 1000) as f64 / 1000.0 * 0.35;
                let nc = cost + diag + mountain + rivp + noise;
                if nc < dist[ni] {
                    dist[ni] = nc;
                    owner[ni] = ow;
                    heap.push(HeapItem { cost: nc, cell: ni as u32, owner: ow });
                }
            }
        }
    }

    // ── Any unowned land (tiny islands with no seed): give each unowned land
    //    connected-component its own province. ──
    let mut extra_seed_cell: Vec<u32> = Vec::new();
    for start in 0..total {
        if buf.terrain[start] != 1 || owner[start] != u32::MAX { continue; }
        let oi = (seed_cells.len() + extra_seed_cell.len()) as u32;
        extra_seed_cell.push(start as u32);
        owner[start] = oi;
        stack.push(start as u32);
        while let Some(c) = stack.pop() {
            let cx = (c % w) as i32; let cy = (c / w) as i32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let ny = cy + dy;
                    if ny < 0 || ny >= hi { continue; }
                    let ni = buf.widx(cx + dx, ny);
                    if buf.terrain[ni] == 1 && !is_lake[ni] && owner[ni] == u32::MAX {
                        owner[ni] = oi;
                        stack.push(ni as u32);
                    }
                }
            }
        }
    }
    let n_seeds = seed_cells.len() + extra_seed_cell.len();
    let seed_of = |oi: u32| -> u32 {
        let oi = oi as usize;
        if oi < seed_cells.len() { seed_cells[oi] } else { extra_seed_cell[oi - seed_cells.len()] }
    };

    // ── Merge slivers: a province below the area floor is folded into the neighbour
    //    it shares the most border with. One pass (deterministic). ──
    let mut cell_count = vec![0u32; n_seeds];
    for &o in owner.iter() { if o != u32::MAX { cell_count[o as usize] += 1; } }
    let min_cells = ((spacing * spacing) / 6).max(6) as u32;
    let mut remap: Vec<u32> = (0..n_seeds as u32).collect();
    // Provinces below the area floor get folded into the neighbour they share the most
    // border with. PERF: this used to rescan ALL cells once per small province — an
    // O(n_small × total) trap that dominated generation on large worlds (hundreds of
    // slivers × 6.5 M cells). Now it's a SINGLE O(total) pass: for every cell owned by a
    // small province, tally its cross-border neighbours into that province's map.
    let is_small: Vec<bool> = cell_count.iter().map(|&c| c > 0 && c < min_cells).collect();
    let mut shared: Vec<std::collections::HashMap<u32, u32>> =
        (0..n_seeds).map(|_| std::collections::HashMap::new()).collect();
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX || !is_small[o as usize] { continue; }
        let cx = (c as u32 % w) as i32; let cy = (c as u32 / w) as i32;
        for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1)] {
            let ny = cy + dy; if ny < 0 || ny >= hi { continue; }
            let no = owner[buf.widx(cx + dx, ny)];
            if no != u32::MAX && no != o { *shared[o as usize].entry(no).or_insert(0) += 1; }
        }
    }
    for p in 0..n_seeds {
        if !is_small[p] { continue; }
        if let Some((&best, _)) = shared[p].iter().max_by_key(|(_, &v)| v) {
            remap[p] = best;
        }
    }
    // Resolve remap chains, then relabel.
    for c in 0..total {
        if owner[c] == u32::MAX { continue; }
        let mut o = owner[c];
        let mut guard = 0;
        while remap[o as usize] != o && guard < 8 { o = remap[o as usize]; guard += 1; }
        owner[c] = o;
    }

    // ── Compact province ids to 0..n and build per-cell u16 map. ──
    let mut old_to_new = std::collections::HashMap::<u32, u16>::new();
    let mut province_id = vec![NO_PROVINCE; total];
    for c in 0..total {
        let o = owner[c];
        if o == u32::MAX { continue; }
        let next = old_to_new.len() as u16;
        let nid = *old_to_new.entry(o).or_insert(next);
        province_id[c] = nid;
    }
    let n = old_to_new.len();
    if n == 0 { return (Vec::new(), province_id); }

    // ── Aggregate per-province stats. ──
    struct Acc {
        cells: u32, fert: f64, elev: f64, food: f64, coastal: bool,
        koppen: std::collections::HashMap<u8, u32>,
        goods_max: Vec<u8>, island: u32, area: f64,
        neighbors: std::collections::HashSet<u16>,
    }
    let ng = buf.goods.len();
    let mut accs: Vec<Acc> = (0..n).map(|_| Acc {
        cells: 0, fert: 0.0, elev: 0.0, food: 0.0, coastal: false,
        koppen: std::collections::HashMap::new(), goods_max: vec![0u8; ng],
        island: 0, area: 0.0, neighbors: std::collections::HashSet::new(),
    }).collect();
    // Map new id → an old owner value (to recover the seed cell for naming).
    let mut new_to_old = vec![0u32; n];
    for (&old, &nid) in old_to_new.iter() { new_to_old[nid as usize] = old; }

    for c in 0..total {
        let pid = province_id[c];
        if pid == NO_PROVINCE { continue; }
        let a = &mut accs[pid as usize];
        a.cells += 1;
        a.fert += buf.fertility[c] as f64;
        a.elev += buf.elevation[c] as f64;
        a.food += food[c] as f64;
        a.island = island[c];
        // Latitude-aware cell area (cos(lat)); base cell ≈ (earth circ / width) km wide.
        let latr = (buf.latitude(c as u32 / w) as f64).to_radians();
        let cell_km = 40075.0 / w as f64;
        a.area += cell_km * cell_km * latr.cos().max(0.05);
        if buf.distance_to_ocean[c] < 0.05 { a.coastal = true; }
        *a.koppen.entry(buf.koppen[c]).or_insert(0) += 1;
        for gd in 0..ng {
            let v = buf.goods[gd][c];
            if v > a.goods_max[gd] { a.goods_max[gd] = v; }
        }
        // neighbour scan (4-dir)
        let cx = (c as u32 % w) as i32; let cy = (c as u32 / w) as i32;
        for &(dx, dy) in &[(-1i32,0i32),(1,0),(0,-1),(0,1)] {
            let nyy = cy + dy; if nyy < 0 || nyy >= hi { continue; }
            let np = province_id[buf.widx(cx + dx, nyy)];
            if np != NO_PROVINCE && np != pid { a.neighbors.insert(np); }
        }
    }

    // Settlements per province (seat = largest population).
    let mut prov_settlements: Vec<Vec<(String, u32)>> = vec![Vec::new(); n];
    for s in settlements {
        let pid = province_id[buf.idx(s.x.min(w - 1), s.y.min(h - 1))];
        if pid != NO_PROVINCE {
            prov_settlements[pid as usize].push((s.id.clone(), s.population));
        }
    }

    let mut provinces: Vec<Province> = Vec::with_capacity(n);
    for pid in 0..n {
        let a = &accs[pid];
        if a.cells == 0 { continue; }
        let cellsf = a.cells as f64;
        let mean_fert = (a.fert / cellsf) as f32;
        let mean_elev = (a.elev / cellsf) as f32;
        let elevation_class: u8 = if mean_elev < 0.30 { 0 } else if mean_elev < 0.55 { 1 } else { 2 };
        let koppen = a.koppen.iter().max_by_key(|(_, &v)| v).map(|(&k, _)| k).unwrap_or(0);
        // Seat: largest settlement, else the seed cell.
        let mut towns = prov_settlements[pid].clone();
        towns.sort_by(|x, y| y.1.cmp(&x.1));
        let (seat_cell, settlement_ids): (u32, Vec<String>) = if let Some((_, _)) = towns.first() {
            // seat = cell of the largest settlement
            let seat = settlements.iter()
                .find(|s| s.id == towns[0].0)
                .map(|s| buf.idx(s.x.min(w - 1), s.y.min(h - 1)) as u32)
                .unwrap_or_else(|| seed_of(new_to_old[pid]));
            (seat, towns.iter().map(|t| t.0.clone()).collect())
        } else {
            (seed_of(new_to_old[pid]), Vec::new())
        };
        let sx = seat_cell % w;
        let sy = seat_cell / w;
        // Goods: quality = suitability (max belt intensity / 255), best-first, top 6.
        let mut goods: Vec<ProvinceGood> = (0..ng)
            .filter(|&gd| a.goods_max[gd] >= 40)
            .map(|gd| ProvinceGood { good: gd as u8, quality: a.goods_max[gd] as f32 / 255.0 })
            .collect();
        goods.sort_by(|x, y| y.quality.partial_cmp(&x.quality).unwrap_or(Ordering::Equal));
        goods.truncate(6);
        // Name (its own) + culture + analog.
        let (kit, ms) = names::resolve_kit(sx, sy, w, h);
        let bucket = name_length_bucket(sx, sy);
        let name = cultures::province_name(kit, ms, sx, sy, bucket);
        let culture = names::culture_label(sx, sy, w, h).to_string();
        let analog = real_world_analog(koppen, elevation_class, a.coastal).to_string();
        let rural_pop = (a.food * 18.0).round().max(0.0) as u32;
        let mut neighbors: Vec<u16> = a.neighbors.iter().copied().collect();
        neighbors.sort_unstable();

        provinces.push(Province {
            id: pid as u16,
            name,
            seat_x: sx, seat_y: sy,
            cells: a.cells,
            area_km2: a.area.round().max(0.0) as u32,
            island: a.island,
            neighbors,
            koppen,
            elevation_class,
            mean_fertility: mean_fert,
            coastal: a.coastal,
            goods,
            culture,
            rural_pop,
            analog,
            settlements: settlement_ids,
        });
    }

    (provinces, province_id)
}

/// Deterministic name-length bucket from the seat cell: 0 very short · 1 short ·
/// 2 medium · 3 long/compound (≈15/35/35/15 split).
fn name_length_bucket(x: u32, y: u32) -> u8 {
    let r = hash2(x as u64, y as u64 ^ 0xA5A5) % 100;
    if r < 15 { 0 } else if r < 50 { 1 } else if r < 85 { 2 } else { 3 }
}

/// Match a province's climate + terrain + coast to a curated list of the real-world
/// regions it most resembles. Deterministic; extend the arms freely.
pub fn real_world_analog(koppen: u8, elev_class: u8, coastal: bool) -> &'static str {
    let upland = elev_class == 2;
    match koppen {
        // Mediterranean (Csa/Csb/Csc)
        8 | 9 | 10 => if coastal {
            "the Provençal coast, Tuscany, the Levant shore, coastal Anatolia and Catalonia"
        } else {
            "the Spanish meseta, inland Anatolia and the hills of Greece"
        },
        // Humid subtropical / oceanic (Cfa/Cfb/Cfc)
        11 | 12 | 13 => if upland {
            "the Appalachians, the Cévennes and the hills of Honshū"
        } else if coastal {
            "the Carolinas, the Rías of Galicia, southern Brazil and coastal Japan"
        } else {
            "the Po valley, the Île-de-France, the English Midlands and the North German plain"
        },
        // Savanna / monsoon (Aw/Am)
        2 | 3 => if coastal {
            "the Coromandel coast, the Guinea shore and coastal Yucatán"
        } else {
            "the Sahel, the Deccan, the East African highlands and the Llanos"
        },
        // Tropical rainforest (Af)
        1 => "the Congo basin, Amazonia, Borneo and the Kerala coast",
        // Steppe (BSh/BSk)
        6 | 7 => if upland {
            "the Iranian plateau, the Anatolian steppe and the Altiplano"
        } else {
            "the Kazakh steppe, the Maghreb high plains, the Great Plains and the Pontic steppe"
        },
        // Desert (BWh/BWk)
        4 | 5 => "the Saharan oases, the Arabian Nejd, the Taklamakan rim and the Atacama",
        // Warm-ish continental (Dfa/Dfb + Ds continental Med)
        14 | 15 | 18 | 19 | 20 => if upland {
            "the Carpathians, the Caucasus foothills and the Appalachians"
        } else {
            "the Ukrainian black-earth, the American Midwest and the Manchurian plain"
        },
        // Cold continental / subarctic (Dfc/Dfd + Dw)
        16 | 17 | 29 | 30 => if coastal {
            "Norway, Newfoundland, the Baltic shore, Hokkaidō and Kamchatka"
        } else {
            "the taiga of Sweden, interior Canada and Siberia"
        },
        // Tundra (ET)
        21 => "Iceland, coastal Greenland, Lapland and the Aleutians",
        // Ice cap (EF)
        22 => "the ice sheets of Greenland and Antarctica",
        _ => if upland {
            "the Alps, the Andes, the Ethiopian highlands and the Harz"
        } else {
            "a temperate, river-fed heartland"
        },
    }
}
