//! #26 · Geographic toponyms — culture-appropriate names for the world's natural
//! features (rivers, mountains, lakes) and its regions. Names are drawn from the
//! same deterministic culture-name machinery used for settlements (`super::names`
//! → `super::cultures`), so a river takes its name from the people whose land it
//! runs through. Generation is OPTIONAL and GATED: it requires an active culture
//! map (the Settlements step), and the caller refuses to run before then.
//!
//! The result is a flat list persisted as JSON in world metadata; the user may
//! rename any entry afterwards (the edited list is saved back).

use super::names;
use super::rivers::{Lake, River};
use super::world_buffer::WorldBuffer;

/// A named geographic feature. `kind`: "river" | "mountain" | "lake" | "region".
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Toponym {
    pub kind: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

/// Normalized elevation above which a local maximum is considered a named peak.
/// (MOUNTAIN_NORM ≈ 0.339 ≈ 3000 m; a named peak sits well above the snow line.)
const PEAK_MIN_ELEV: f32 = 0.52;
/// Half-window (cells) for the local-maximum / prominence test.
const PEAK_WINDOW: i32 = 6;
/// Cap on named peaks so the map isn't littered (highest first).
const MAX_PEAKS: usize = 40;
/// Minimum lake size (cells) to earn a name.
const MIN_LAKE_CELLS: usize = 6;

/// Build the toponym list for a world. `rivers`/`lakes` come from the Rivers step
/// (passed through from the frontend); peaks are scanned from `buf.elevation`;
/// regions come from the active culture map's hearths.
pub fn generate(buf: &WorldBuffer, rivers: &[River], lakes: &[Lake]) -> Vec<Toponym> {
    let (w, h) = (buf.width, buf.height);
    let mut out: Vec<Toponym> = Vec::new();

    // ── Regions: one per culture hearth (its homeland name) ──
    if let Some(map) = super::cultures::active() {
        for hh in &map.hearths {
            if hh.people.is_empty() { continue; }
            out.push(Toponym { kind: "region".into(), name: hh.people.clone(), x: hh.x, y: hh.y });
        }
    }

    // ── Rivers: name the larger ones at their midpoint, styled by local culture ──
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
        // Salt the name draw so a river never collides with a settlement on the
        // same cell, and each river reads distinctly.
        let name = feature_name(mx, my, w, h, 0x1111 ^ n as u32);
        out.push(Toponym { kind: "river".into(), name, x: mx as f32, y: my as f32 });
    }

    // ── Lakes: name the larger basins at their centroid ──
    for (n, lake) in lakes.iter().enumerate() {
        if lake.cells.len() < MIN_LAKE_CELLS { continue; }
        let (mut sx, mut sy) = (0u64, 0u64);
        for &(cx, cy) in &lake.cells { sx += cx as u64; sy += cy as u64; }
        let (cx, cy) = ((sx / lake.cells.len() as u64) as u32, (sy / lake.cells.len() as u64) as u32);
        let name = feature_name(cx, cy, w, h, 0x2222 ^ n as u32);
        out.push(Toponym { kind: "lake".into(), name, x: cx as f32, y: cy as f32 });
    }

    // ── Mountains: prominent local elevation maxima ──
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
        let name = feature_name(px, py, w, h, 0x3333 ^ n as u32);
        out.push(Toponym { kind: "mountain".into(), name, x: px as f32, y: py as f32 });
    }

    out
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
