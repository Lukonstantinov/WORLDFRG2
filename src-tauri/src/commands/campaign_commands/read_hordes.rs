//! read_hordes commands — `docs/living_world/09_REALMS_WAR_AND_BARBARIANS.md`
//! Part D/F. One plain read-only query: the world's horde roster (the doc's
//! own "Barbarian Tribes window" data). The fuller conflict-map layer
//! (armies, occupation, smoke plumes) is queued — Parts B/C were not built
//! this session, see `hordes.rs`'s own doc comment.
use super::*;
use crate::sim::tick::Horde;

#[derive(Serialize, Clone, Default)]
pub struct HordeBrief {
    pub id: u32,
    pub name: String,
    pub culture: String,
    pub leader_name: String,
    /// "plunder" | "land to settle" | "revenge" | "a crown" | "tribute".
    pub goal_name: String,
    pub province: u32,
    pub strength: f32,
    pub origin_story: String,
    /// "active" | "settled" | "paid off" | "defeated" | "broken up".
    pub stage_name: String,
    pub cities_sacked: u32,
    pub cities_razed: u32,
}

fn goal_name(g: u8) -> &'static str {
    match g {
        1 => "land to settle", 2 => "revenge", 3 => "a crown", 4 => "tribute", _ => "plunder",
    }
}
fn stage_name(s: u8) -> &'static str {
    match s {
        2 => "settled", 3 => "paid off", 4 => "defeated", 5 => "broken up", _ => "active",
    }
}

#[tauri::command]
pub fn campaign_get_hordes(db: State<'_, WorldDb>) -> Result<Vec<HordeBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let out: Vec<HordeBrief> = sim.hordes.iter().map(|h: &Horde| {
        let leader_name = sim.people.iter().find(|p| p.id == h.leader).map(|p| p.name.clone())
            .or_else(|| sim.hall_of_dead.iter().find(|p| p.id == h.leader).map(|p| p.name.clone()))
            .unwrap_or_else(|| "unknown".to_string());
        HordeBrief {
            id: h.id, name: h.name.clone(), culture: h.culture.clone(), leader_name,
            goal_name: goal_name(h.goal).to_string(), province: h.province, strength: h.strength,
            origin_story: h.origin_story.clone(), stage_name: stage_name(h.stage).to_string(),
            cities_sacked: h.cities_sacked, cities_razed: h.cities_razed,
        }
    }).collect();
    Ok(out)
}
