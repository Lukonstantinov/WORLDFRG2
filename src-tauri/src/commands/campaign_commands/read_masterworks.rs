//! read_masterworks commands — `docs/living_world/07_ARTISANS_AND_MASTERWORKS.md`.
//!
//! Two plain read-only queries: a city's masterworks (the doc's own "City
//! Gallery"), and one house's gallery. Both pure derived reads.
use super::*;
use crate::sim::tick::{Masterwork, OWNER_CITY, OWNER_HOUSE};

#[derive(Serialize, Clone, Default)]
pub struct MasterworkBrief {
    pub id: u32,
    pub title: String,
    pub kind_name: String,
    pub maker_name: String,
    pub year: u32,
    pub material: String,
    pub condition_name: String,
    pub prestige: f32,
    pub provenance: Vec<(u32, String)>,
}

fn kind_name(k: u8) -> &'static str {
    ["statue", "fresco", "mosaic", "epic", "temple frieze", "treatise", "jewelled crown", "bronze doors"]
        .get(k as usize).copied().unwrap_or("work")
}
fn condition_name(c: u8) -> &'static str {
    match c { 1 => "damaged", 2 => "looted", 3 => "destroyed", _ => "intact" }
}

fn brief_of(sim: &crate::sim::tick::CampaignSim, m: &Masterwork) -> MasterworkBrief {
    let maker_name = sim.people.iter().find(|p| p.id as i32 == m.maker).map(|p| p.name.clone())
        .or_else(|| sim.hall_of_dead.iter().find(|p| p.id as i32 == m.maker).map(|p| p.name.clone()))
        .unwrap_or_else(|| "unknown".to_string());
    MasterworkBrief {
        id: m.id, title: m.title.clone(), kind_name: kind_name(m.kind).to_string(),
        maker_name, year: m.year, material: m.material.clone(),
        condition_name: condition_name(m.condition).to_string(), prestige: m.prestige,
        provenance: m.provenance.clone(),
    }
}

/// A city's own gallery — every masterwork currently located and owned
/// there (a looted work now sitting in a foreign gallery is not this
/// city's to show).
#[tauri::command]
pub fn campaign_get_city_gallery(hub: u32, db: State<'_, WorldDb>) -> Result<Vec<MasterworkBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let out = sim.masterworks.iter()
        .filter(|m| m.location_hub == hub as i32 && m.owner_kind == OWNER_CITY)
        .map(|m| brief_of(&sim, m))
        .collect();
    Ok(out)
}

/// One house's own gallery — works it owns, wherever they currently sit.
#[tauri::command]
pub fn campaign_get_house_gallery(idx: u32, db: State<'_, WorldDb>) -> Result<Vec<MasterworkBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let out = sim.masterworks.iter()
        .filter(|m| m.owner_kind == OWNER_HOUSE && m.owner_idx == idx as i32)
        .map(|m| brief_of(&sim, m))
        .collect();
    Ok(out)
}
