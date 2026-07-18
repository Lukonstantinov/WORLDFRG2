//! Place- and family-name generation.
//!
//! Names are a deterministic function of cell position + world size, so a
//! settlement and the trade hub on the same cell always resolve to the SAME name
//! across the settlement, political and economy layers.
//!
//! The CULTURE of a cell comes from the organic hearth map in [`super::cultures`]
//! when one is active (see `cultures::ensure_active`). When none is active (old
//! saves, or before the Settlements step has run), we fall back to a legacy coarse
//! 4-culture province grid so naming still works and old worlds are unchanged.

use super::cultures;

/// Legacy province culture → kit index (0 Roman · 1 Hellene · 2 Punic · 3 Persian),
/// used only as a fallback when no organic culture map is active.
fn legacy_kit(x: u32, y: u32, w: u32, h: u32) -> usize {
    let pw = (w / 5).max(1);
    let ph = (h / 4).max(1);
    let px = (x / pw) as u64;
    let py = (y / ph) as u64;
    let v = cultures::hash64(0x501E_C0DE_u64 ^ px.wrapping_mul(0x9E3779B1) ^ py.wrapping_mul(0x85EBCA77));
    (v % 4) as usize
}

/// Resolve the (kit index, per-world mutation seed) governing cell `(x,y)`.
pub fn resolve_kit(x: u32, y: u32, w: u32, h: u32) -> (usize, u64) {
    if let Some(map) = cultures::active() {
        if let Some(k) = map.kit_at(x, y) {
            return k;
        }
    }
    (legacy_kit(x, y, w, h), 0)
}

/// The culture/people label governing a cell ("Norse", "Sinitic", …).
pub fn culture_label(x: u32, y: u32, w: u32, h: u32) -> &'static str {
    let (kit, _) = resolve_kit(x, y, w, h);
    cultures::KITS[kit.min(cultures::KITS.len() - 1)].name
}

/// The region / people homeland name governing a cell (e.g. "Vexillia"). Falls
/// back to the culture label when no organic map is active.
pub fn region_name(x: u32, y: u32, w: u32, h: u32) -> String {
    if let Some(map) = cultures::active() {
        if let Some(hh) = map.hearth_at(x, y) {
            return hh.people.clone();
        }
    }
    culture_label(x, y, w, h).to_string()
}

/// Generate an antique place name for the settlement / hub at `(x, y)`.
pub fn gen_name(x: u32, y: u32, w: u32, h: u32) -> String {
    let (kit, ms) = resolve_kit(x, y, w, h);
    cultures::place_name(kit, ms, x, y)
}

/// A trading-house family surname for the culture at `(x,y)`, varied by `salt`.
pub fn gen_family_name(x: u32, y: u32, w: u32, h: u32, salt: u64) -> String {
    let (kit, ms) = resolve_kit(x, y, w, h);
    cultures::family_name(kit, ms, salt, x, y)
}

/// A head-of-family personal name ("Marcus Cassii") for the local culture.
pub fn gen_head_name(x: u32, y: u32, w: u32, h: u32, family: &str, salt: u64) -> String {
    let (kit, ms) = resolve_kit(x, y, w, h);
    cultures::head_name(kit, ms, family, salt, x, y)
}

/// A culture-styled guild name for the city at `(x,y)` (e.g. "Collegium of
/// Aquentia", "Suq of Madinah"), optionally tagged with its chief trade.
pub fn gen_guild_name(x: u32, y: u32, w: u32, h: u32, city: &str, specialty: Option<&str>, salt: u64) -> String {
    let (kit, ms) = resolve_kit(x, y, w, h);
    cultures::guild_name(kit, ms, city, specialty, salt)
}

/// Generate a name plus an epithet for a major place. `tier` 0 = none, higher = grander.
pub fn gen_name_epithet(x: u32, y: u32, w: u32, h: u32, tier: u8) -> String {
    let (kit, ms) = resolve_kit(x, y, w, h);
    cultures::place_epithet(kit, ms, x, y, tier)
}
