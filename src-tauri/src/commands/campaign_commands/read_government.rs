//! read_government commands — `docs/living_world/04_GOVERNMENT_AND_EDICTS.md`
//! slice 04.7, the Government window's own reads. Split out as its own file
//! (rather than folded into `read_hubs.rs`) for the same reason `read_houses.rs`
//! was: a government seat/edict/debate is a different question about a hub than
//! its market or its trade — "who governs it and what have they decided" — and
//! the row's own slice list names these two commands explicitly.
//!
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;
use crate::sim::tick::{
    Official, TickHub, TICKS_PER_YEAR,
    official_allegiance, official_path_name, government_blocs, edict_family_name,
    GOV_OUTCOME_PASSED, GOV_OUTCOME_FAILED, GOV_OUTCOME_DEADLOCKED, GOV_OUTCOME_COUP,
    govt_type_name, office_title,
};

/// One seat, browser-ready — a plain house name resolved rather than a bare
/// index, since the window has no reason to look houses up itself.
#[derive(Serialize, Clone, Default)]
pub struct SeatBrief {
    pub role: u8,
    pub office_title: String,
    pub name: String,
    pub individual_id: i32,
    /// "kin of a house" | "military success" | … — `official_path_name`.
    pub path: String,
    pub suitability: f32,
    /// 0 house · 1 ruler(kin) · 2 commons/none — `official_allegiance`.
    pub allegiance: u8,
    pub house: i32,
    pub house_name: String,
    pub control: f32,
}

/// A bloc — every seat sharing one allegiance target, grouped for the window's
/// "seats grouped by bloc" header (04's own UI spec).
#[derive(Serialize, Clone, Default)]
pub struct BlocBrief {
    /// -1 for the commons/no-patron bloc, else a house index.
    pub house: i32,
    pub house_name: String,
    pub seat_indices: Vec<u32>,
}

/// The debate in progress, if any — the window's "round timeline with the
/// moving tally".
#[derive(Serialize, Clone, Default)]
pub struct DebateBrief {
    pub family: String,
    pub tag: i8,
    pub major: bool,
    pub cost: f32,
    pub round: u8,
    pub round_cap: u8,
    pub tally: f32,
}

/// One edict in force.
#[derive(Serialize, Clone, Default)]
pub struct EdictBrief {
    pub family: String,
    pub tag: i8,
    pub major: bool,
    pub enacted_year: u32,
    pub expires_year: u32,
}

/// One closed debate's outcome, or a coup — "recent history".
#[derive(Serialize, Clone, Default)]
pub struct GovHistoryBrief {
    pub year: u32,
    pub family: String,
    /// "passed" | "failed" | "deadlocked" | "coup".
    pub outcome: String,
}

/// The whole Government window for one city.
#[derive(Serialize, Clone, Default)]
pub struct GovernmentBrief {
    pub hub: u32,
    pub city: String,
    /// "Merchant Council (Doge)" | "Principality (Prince)" | "Free Commune (Mayor)".
    pub form: String,
    pub legitimacy: f32,
    pub gov_points: f32,
    /// −1 conservative .. +1 libertarian.
    pub gov_position: f32,
    pub seats: Vec<SeatBrief>,
    pub blocs: Vec<BlocBrief>,
    pub debate: Option<DebateBrief>,
    pub edicts: Vec<EdictBrief>,
    pub history: Vec<GovHistoryBrief>,
}

fn outcome_name(o: u8) -> &'static str {
    match o {
        GOV_OUTCOME_PASSED => "passed",
        GOV_OUTCOME_FAILED => "failed",
        GOV_OUTCOME_DEADLOCKED => "deadlocked",
        GOV_OUTCOME_COUP => "coup",
        _ => "passed",
    }
}

fn seat_brief(o: &Official, house_name: &dyn Fn(i32) -> String) -> SeatBrief {
    SeatBrief {
        role: o.role,
        office_title: office_title(o.role).to_string(),
        name: o.name.clone(),
        individual_id: o.individual_id,
        path: official_path_name(o.path).to_string(),
        suitability: o.suitability,
        allegiance: official_allegiance(o),
        house: o.house,
        house_name: house_name(o.house),
        control: o.control,
    }
}

/// The Government window — one city's seats, blocs, the debate in progress
/// (if any), and recent history. `None` on a city with no government seeded
/// yet (an estate, or a hub the yearly pass hasn't reached in its first
/// partial year).
#[tauri::command]
pub fn campaign_get_government(hub: u32, db: State<'_, WorldDb>) -> Result<Option<GovernmentBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let h = hub as usize;
    let hb: &TickHub = match sim.hubs.get(h) { Some(hb) => hb, None => return Ok(None) };
    if hb.officials.is_empty() { return Ok(None); }

    let house_name = |hi: i32| -> String {
        if hi < 0 { return String::new(); }
        sim.houses.get(hi as usize).map(|hh| hh.name.clone()).unwrap_or_default()
    };
    let seats: Vec<SeatBrief> = hb.officials.iter().map(|o| seat_brief(o, &house_name)).collect();
    let blocs: Vec<BlocBrief> = government_blocs(&hb.officials).into_iter()
        .map(|(house, idxs)| BlocBrief {
            house,
            house_name: house_name(house),
            seat_indices: idxs.into_iter().map(|i| i as u32).collect(),
        })
        .collect();
    let debate = hb.gov_debate.as_ref().map(|d| DebateBrief {
        family: edict_family_name(d.family).to_string(),
        tag: d.tag, major: d.major, cost: d.cost, round: d.round, round_cap: d.round_cap, tally: d.tally,
    });
    let edicts: Vec<EdictBrief> = hb.gov_edicts.iter().map(|e| EdictBrief {
        family: edict_family_name(e.family).to_string(),
        tag: e.tag, major: e.major,
        enacted_year: e.enacted_tick / TICKS_PER_YEAR,
        expires_year: e.expires_tick / TICKS_PER_YEAR,
    }).collect();
    let history: Vec<GovHistoryBrief> = hb.gov_history.iter().map(|e| GovHistoryBrief {
        year: e.tick / TICKS_PER_YEAR,
        family: edict_family_name(e.family).to_string(),
        outcome: outcome_name(e.outcome).to_string(),
    }).collect();

    Ok(Some(GovernmentBrief {
        hub, city: hb.name.clone(), form: govt_type_name(hb.govt_type).to_string(),
        legitimacy: hb.legitimacy, gov_points: hb.gov_points, gov_position: hb.gov_position,
        seats, blocs, debate, edicts, history,
    }))
}

/// The edicts in force at one city — a thinner read than the full Government
/// window, for a panel that only wants the "edicts in force with expiry" list
/// (the row's own doc names this as a second, separate command).
#[tauri::command]
pub fn campaign_get_edicts(hub: u32, db: State<'_, WorldDb>) -> Result<Vec<EdictBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let Some(hb) = sim.hubs.get(hub as usize) else { return Ok(vec![]) };
    Ok(hb.gov_edicts.iter().map(|e| EdictBrief {
        family: edict_family_name(e.family).to_string(),
        tag: e.tag, major: e.major,
        enacted_year: e.enacted_tick / TICKS_PER_YEAR,
        expires_year: e.expires_tick / TICKS_PER_YEAR,
    }).collect())
}

