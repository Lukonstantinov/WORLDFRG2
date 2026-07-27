//! #26 Â· Geographic toponyms â€” culture-appropriate names for the world's natural
//! features (rivers, mountains, lakes) and its regions. Names are drawn from the
//! same deterministic culture-name machinery used for settlements (`super::names`
//! â†’ `super::cultures`), so a river takes its name from the people whose land it
//! runs through. Generation is OPTIONAL and GATED: it requires an active culture
//! map (the Settlements step), and the caller refuses to run before then.
//!
//! The result is a flat list persisted as JSON in world metadata; the user may
//! rename any entry afterwards (the edited list is saved back).

use super::names;
use crate::sim::rivers::{Lake, River};
use crate::sim::world_buffer::WorldBuffer;

/// A named geographic feature. `kind`: "river" | "mountain" | "lake" | "region" |
/// "desert" | "forest" | "tundra" (the last three are large biome-region
/// subnames — a contiguous patch of the same broad biome, named separately from
/// the culture-hearth "region" so a player can toggle them independently).
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Toponym {
    pub kind: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

/// Normalized elevation above which a local maximum is considered a named peak.
/// (MOUNTAIN_NORM â‰ˆ 0.339 â‰ˆ 3000 m; a named peak sits well above the snow line.)
const PEAK_MIN_ELEV: f32 = 0.52;
/// Half-window (cells) for the local-maximum / prominence test.
const PEAK_WINDOW: i32 = 6;
/// Cap on named peaks so the map isn't littered (highest first).
const MAX_PEAKS: usize = 40;
/// Minimum lake size (cells) to earn a name.
const MIN_LAKE_CELLS: usize = 6;
/// Minimum contiguous size (cells) for a biome patch to earn a subregion name —
/// scaled by grid area below so a small painted world and a big generated one
/// both get a sane number of named deserts/forests/tundras rather than either
/// none or hundreds of tiny slivers.
const MIN_BIOME_REGION_FRAC: f32 = 0.0015; // ~0.15% of the land grid
/// Cap on named biome subregions (largest first) so the map isn't littered.
const MAX_BIOME_REGIONS: usize = 25;

/// Build the toponym list for a world. `rivers`/`lakes` come from the Rivers step
/// (passed through from the frontend); peaks are scanned from `buf.elevation`;
/// regions come from the active culture map's hearths.
pub fn generate(buf: &WorldBuffer, rivers: &[River], lakes: &[Lake]) -> Vec<Toponym> {
    let (w, h) = (buf.width, buf.height);
    let mut out: Vec<Toponym> = Vec::new();
    // Every drawn feature name must be unique across the whole world â€” no two
    // rivers/lakes/peaks share a label (the "same name in several places" bug).
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();

    // â”€â”€ Regions: one per culture hearth (its homeland name) â”€â”€
    if let Some(map) = super::cultures::active() {
        for hh in &map.hearths {
            if hh.people.is_empty() { continue; }
            used.insert(hh.people.clone());
            out.push(Toponym { kind: "region".into(), name: hh.people.clone(), x: hh.x, y: hh.y });
        }
    }

    // â”€â”€ Rivers: name the larger ones at their midpoint, styled by local culture â”€â”€
    // Prefer major/navigable rivers; fall back to the longest if none are flagged.
    let mut river_idx: Vec<usize> = (0..rivers.len())
        .filter(|&i| rivers[i].major || rivers[i].navigable)
        .collect();
    if river_idx.is_empty() {
        river_idx = (0..rivers.len()).collect();
    }
    river_idx.sort_by_key(|&i| std::cmp::Reverse(rivers[i].points.len()));
    for (n, &i) in river_idx.iter().enumerate() {
        let pts = &rivers[i].points;
        if pts.len() < 4 { continue; }
        let (mx, my) = pts[pts.len() / 2];
        // Culture-styled river name from the local people, made globally unique
        // (re-rolled on a collision). Matches the Hydrology dashboard's naming.
        if let Some(name) = unique_river_name(mx, my, w, h, 0x1111 ^ n as u64, &mut used) {
            out.push(Toponym { kind: "river".into(), name, x: mx as f32, y: my as f32 });
        }
    }

    // â”€â”€ Lakes: name the larger basins at their centroid â”€â”€
    for (n, lake) in lakes.iter().enumerate() {
        if lake.cells.len() < MIN_LAKE_CELLS { continue; }
        let (mut sx, mut sy) = (0u64, 0u64);
        for &(cx, cy) in &lake.cells { sx += cx as u64; sy += cy as u64; }
        let (cx, cy) = ((sx / lake.cells.len() as u64) as u32, (sy / lake.cells.len() as u64) as u32);
        if let Some(name) = unique_feature_name(cx, cy, w, h, 0x2222 ^ n as u32, &mut used) {
            out.push(Toponym { kind: "lake".into(), name, x: cx as f32, y: cy as f32 });
        }
    }

    // â”€â”€ Mountains: prominent local elevation maxima â”€â”€
    let mut peaks: Vec<(u32, u32, f32)> = Vec::new();
    let idx = |x: u32, y: u32| (y * w + x) as usize;
    let mut y = PEAK_WINDOW as u32;
    while y + PEAK_WINDOW as u32 <= h.saturating_sub(1) {
        let mut x = 0u32;
        while x < w {
            let e = buf.elevation[idx(x, y)];
            if e >= PEAK_MIN_ELEV && buf.terrain[idx(x, y)] == 1 {
                // Local maximum within the window (X wraps, Y clamps).
                let mut is_max = true;
                'scan: for dy in -PEAK_WINDOW..=PEAK_WINDOW {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    for dx in -PEAK_WINDOW..=PEAK_WINDOW {
                        let nx = ((x as i32 + dx).rem_euclid(w as i32)) as u32;
                        if buf.elevation[idx(nx, ny as u32)] > e { is_max = false; break 'scan; }
                    }
                }
                if is_max { peaks.push((x, y, e)); }
            }
            x += 1;
        }
        y += 1;
    }
    // Highest first, capped, then named.
    peaks.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    peaks.truncate(MAX_PEAKS);
    for (n, &(px, py, _)) in peaks.iter().enumerate() {
        if let Some(name) = unique_feature_name(px, py, w, h, 0x3333 ^ n as u32, &mut used) {
            out.push(Toponym { kind: "mountain".into(), name, x: px as f32, y: py as f32 });
        }
    }

    // -- Biome subregions: contiguous desert/forest/tundra patches, named
    // separately from the culture-hearth "region" above so they can be toggled
    // independently (large deserts/forests are landmarks in their own right,
    // not tied to any one people's homeland).
    for (n, (cat, cx, cy)) in find_biome_regions(buf).into_iter().enumerate() {
        let suffix = match cat {
            "desert" => "Desert",
            "tundra" => "Tundra",
            _ => "Forest",
        };
        if let Some(base) = unique_feature_name(cx, cy, w, h, 0x4444 ^ n as u32, &mut used) {
            let name = format!("{base} {suffix}");
            out.push(Toponym { kind: cat.into(), name, x: cx as f32, y: cy as f32 });
        }
    }

    out
}

/// Broad biome CATEGORY for a Köppen code + elevation — coarser than the full
/// 32-code classification, grouped into the handful of buckets a namer would
/// actually reach for on a map. `None` (savanna, Mediterranean scrub, dry
/// continental, …) is deliberately left unnamed — those read as transitional,
/// not a landmark region in their own right.
fn biome_category(koppen: u8, elevation: f32) -> Option<&'static str> {
    if elevation > 0.40 { return Some("tundra"); } // montane/alpine reads cold/bare
    match koppen {
        1 | 2 => Some("forest"),                                     // Af/Am rainforest/monsoon
        4 | 5 | 6 | 7 => Some("desert"),                              // BWh/BWk/BSh/BSk
        11 | 12 | 13 | 14 | 15 | 24 | 25 | 26 | 27 | 28 => Some("forest"), // Cf/Cw/Df/Dw forests
        16 | 17 | 29 | 30 => Some("tundra"),                          // Dfc/Dfd/Dwc/Dwd taiga fringe
        21 | 22 => Some("tundra"),                                    // ET/EF
        _ => None,
    }
}

/// Flood-fill contiguous same-category biome patches (desert/forest/tundra),
/// returning `(category, centroid_x, centroid_y)` for each patch at or above
/// `MIN_BIOME_REGION_FRAC` of the land grid, largest first, capped at
/// `MAX_BIOME_REGIONS`. Cheap: one flood-fill pass over the grid with a visited
/// bitmap, mirroring the same technique culture hearths use.
fn find_biome_regions(buf: &WorldBuffer) -> Vec<(&'static str, u32, u32)> {
    let (w, h) = (buf.width, buf.height);
    let n = (w as usize) * (h as usize);
    let idx = |x: u32, y: u32| (y * w + x) as usize;
    let mut visited = vec![false; n];
    let min_size = ((n as f32) * MIN_BIOME_REGION_FRAC).round().max(20.0) as usize;
    let mut regions: Vec<(&'static str, u32, u32, usize)> = Vec::new();

    for sy in 0..h {
        for sx in 0..w {
            let si = idx(sx, sy);
            if visited[si] || buf.terrain[si] != 1 { continue; }
            let Some(cat) = biome_category(buf.koppen[si], buf.elevation[si]) else {
                visited[si] = true;
                continue;
            };
            // BFS the connected component sharing this category.
            let mut stack = vec![(sx, sy)];
            visited[si] = true;
            let (mut sumx, mut sumy, mut count) = (0u64, 0u64, 0usize);
            while let Some((x, y)) = stack.pop() {
                sumx += x as u64;
                sumy += y as u64;
                count += 1;
                for &(dx, dy) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let ny = y as i32 + dy;
                    if ny < 0 || ny >= h as i32 { continue; }
                    let nx = buf.wrap_x(x as i32 + dx);
                    let ni = idx(nx, ny as u32);
                    if visited[ni] || buf.terrain[ni] != 1 { continue; }
                    if biome_category(buf.koppen[ni], buf.elevation[ni]) != Some(cat) { continue; }
                    visited[ni] = true;
                    stack.push((nx, ny as u32));
                }
            }
            if count >= min_size {
                regions.push((cat, (sumx / count as u64) as u32, (sumy / count as u64) as u32, count));
            }
        }
    }

    regions.sort_by(|a, b| b.3.cmp(&a.3));
    regions.truncate(MAX_BIOME_REGIONS);
    regions.into_iter().map(|(cat, x, y, _)| (cat, x, y)).collect()
}

/// A culture-styled RIVER name at `(x,y)`, re-rolled until it's not already in
/// `used` (bounded). Returns None only if every draw collided (vanishingly rare).
fn unique_river_name(x: u32, y: u32, w: u32, h: u32, salt: u64, used: &mut std::collections::HashSet<String>) -> Option<String> {
    let (kit, ms) = names::resolve_kit(x, y, w, h);
    let seed0 = (x as u64).wrapping_mul(73856093) ^ (y as u64).wrapping_mul(19349663) ^ salt.wrapping_mul(0x9E3779B97F4A7C15);
    for t in 0..32u64 {
        let cand = super::cultures::river_name(kit, ms, seed0.wrapping_add(t.wrapping_mul(0x100000001B3)));
        if used.insert(cand.clone()) { return Some(cand); }
    }
    None
}

/// A culture-styled place name for a lake/peak at `(x,y)`, re-rolled off `salt`
/// until it isn't already in `used`. Returns None if all attempts collided.
fn unique_feature_name(x: u32, y: u32, w: u32, h: u32, salt: u32, used: &mut std::collections::HashSet<String>) -> Option<String> {
    for t in 0..24u32 {
        let cand = feature_name(x, y, w, h, salt.wrapping_add(t.wrapping_mul(2654435761)));
        if used.insert(cand.clone()) { return Some(cand); }
    }
    None
}

/// A culture-styled proper name for the feature at `(x,y)`, salted so distinct
/// features (and nearby settlements) never collide. Uses the culture resolved at
/// the feature's own cell, so the name fits the people whose land it lies in.
fn feature_name(x: u32, y: u32, w: u32, h: u32, salt: u32) -> String {
    // Perturb the position passed to the deterministic name draw (the culture/kit
    // is still resolved from the TRUE cell inside names::gen_name via resolve_kit
    // when salt is 0; here we want a different draw, so shift the draw position
    // while keeping it within the same locale).
    let dx = (salt & 0x7) as u32;
    let dy = ((salt >> 3) & 0x7) as u32;
    let sx = x.wrapping_add(dx) % w.max(1);
    let sy = y.wrapping_add(dy) % h.max(1);
    // Resolve kit at the true cell so the style matches local culture, but draw
    // the syllables from the shifted position for variety.
    let (kit, ms) = names::resolve_kit(x, y, w, h);
    super::cultures::place_name(kit, ms, sx, sy)
}


