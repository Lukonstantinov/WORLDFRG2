//! read_venues commands — `docs/living_world/08_LEISURE_AND_GAMES.md`.
//!
//! One plain read-only query: a city's venues (the doc's own venue
//! subpanel data — full history/attendance detail is queued, Q08.3).
use super::*;
use crate::sim::tick::{Venue, leisure_type_name};

#[derive(Serialize, Clone, Default)]
pub struct VenueBrief {
    pub id: u32,
    pub name: String,
    pub leisure_type_name: String,
    pub tier: u8,
    /// "thriving" | "declining" | "abandoned".
    pub condition_name: String,
    pub games_held: u32,
    pub prestige: f32,
    pub international_host: bool,
    pub sponsor_house: i32,
}

fn condition_name(c: u8) -> &'static str {
    match c { 1 => "declining", 2 => "abandoned", _ => "thriving" }
}

#[tauri::command]
pub fn campaign_get_venues(hub: u32, db: State<'_, WorldDb>) -> Result<Vec<VenueBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let out: Vec<VenueBrief> = sim.venues.iter()
        .filter(|v: &&Venue| v.hub == hub as i32)
        .map(|v| VenueBrief {
            id: v.id, name: v.name.clone(), leisure_type_name: leisure_type_name(v.leisure_type).to_string(),
            tier: v.tier, condition_name: condition_name(v.condition).to_string(),
            games_held: v.games_held, prestige: v.prestige, international_host: v.international_host,
            sponsor_house: v.sponsor_house,
        })
        .collect();
    Ok(out)
}
