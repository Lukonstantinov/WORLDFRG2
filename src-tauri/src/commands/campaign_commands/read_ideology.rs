//! read_ideology commands — `docs/living_world/06_IDEOLOGY_AND_SCHOLARS.md`.
//!
//! Three plain, read-only queries over the real state `ideology.rs` builds:
//! a city's three ideology meters + resident scholars/schools, the world's
//! named-ideology roster, and the world's school roster. All three are pure
//! derived reads (nothing here writes anything), the same discipline every
//! other `read_*.rs` file in this folder already follows.
use super::*;
use crate::sim::tick::{TickHub, ideo_trait_name, ROLE_SCHOLAR, ROLE_PHILOSOPHER, ROLE_IDEOLOGUE, TICKS_PER_YEAR};

#[derive(Serialize, Clone, Default)]
pub struct ScholarBrief {
    pub individual_id: u32,
    pub name: String,
    pub stage: u8,
    pub fame: f32,
    pub ideology: [f32; 4],
}

#[derive(Serialize, Clone, Default)]
pub struct CityIdeologyBrief {
    pub nobles: [f32; 4],
    pub commons: [f32; 4],
    pub government: [f32; 4],
    /// −1 if no named ideology currently sits close enough to call a hold.
    pub dominant_id: i32,
    pub dominant_name: String,
    pub scholars: Vec<ScholarBrief>,
    pub schools: Vec<u32>,
}

#[tauri::command]
pub fn campaign_get_city_ideology(hub: u32, db: State<'_, WorldDb>) -> Result<CityIdeologyBrief, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(CityIdeologyBrief::default()) };
    let h = hub as usize;
    let Some(hb): Option<&TickHub> = sim.hubs.get(h) else { return Ok(CityIdeologyBrief::default()) };
    let dominant_name = sim.ideologies.iter().find(|i| i.id as i32 == hb.ideology_dominant)
        .map(|i| i.name.clone()).unwrap_or_default();
    let scholars = sim.people.iter()
        .filter(|p| p.is_alive() && p.current_hub == hub as i32
            && p.roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE)))
        .map(|p| ScholarBrief { individual_id: p.id, name: p.name.clone(), stage: p.scholar_stage, fame: p.fame, ideology: p.ideology })
        .collect();
    let schools = sim.schools.iter().filter(|s| s.hub == hub as i32).map(|s| s.id).collect();
    Ok(CityIdeologyBrief {
        nobles: hb.ideology_nobles, commons: hb.ideology_commons, government: hb.ideology_gov,
        dominant_id: hb.ideology_dominant, dominant_name, scholars, schools,
    })
}

#[derive(Serialize, Clone, Default)]
pub struct IdeologyBrief {
    pub id: u32,
    pub name: String,
    pub traits: Vec<String>,
    pub position: [f32; 4],
    pub founder: i32,
    pub home_hub: i32,
    pub adherents: f32,
    pub canonical: bool,
}

/// The world's named-ideology roster — canonical + custom, ranked by
/// adherents (the doc's own "ideologies ranked by adherents" UI ask).
#[tauri::command]
pub fn campaign_get_ideologies(db: State<'_, WorldDb>) -> Result<Vec<IdeologyBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<IdeologyBrief> = sim.ideologies.iter().map(|i| IdeologyBrief {
        id: i.id, name: i.name.clone(),
        traits: i.traits.iter().map(|&t| ideo_trait_name(t).to_string()).collect(),
        position: i.position, founder: i.founder, home_hub: i.home_hub,
        adherents: i.adherents, canonical: i.founder < 0,
    }).collect();
    out.sort_by(|a, b| b.adherents.partial_cmp(&a.adherents).unwrap_or(std::cmp::Ordering::Equal));
    Ok(out)
}

#[derive(Serialize, Clone, Default)]
pub struct SchoolBrief {
    pub id: u32,
    pub hub: i32,
    pub founder_name: String,
    pub doctrine_name: String,
    pub founded_year: u32,
    pub students: u32,
}

/// The world's school roster (the doc's own "Schools & Great Minds" window
/// data — see `ideology.rs`'s own module doc for what the window itself
/// still owes, Q06.5).
#[tauri::command]
pub fn campaign_get_schools(db: State<'_, WorldDb>) -> Result<Vec<SchoolBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let out = sim.schools.iter().map(|s| {
        let founder_name = sim.people.iter().find(|p| p.id == s.founder).map(|p| p.name.clone())
            .or_else(|| sim.hall_of_dead.iter().find(|p| p.id == s.founder).map(|p| p.name.clone()))
            .unwrap_or_else(|| "?".to_string());
        let doctrine_name = sim.ideologies.iter().find(|i| i.id == s.doctrine).map(|i| i.name.clone()).unwrap_or_default();
        SchoolBrief {
            id: s.id, hub: s.hub, founder_name, doctrine_name,
            founded_year: s.founded_tick / TICKS_PER_YEAR, students: s.students,
        }
    }).collect();
    Ok(out)
}
