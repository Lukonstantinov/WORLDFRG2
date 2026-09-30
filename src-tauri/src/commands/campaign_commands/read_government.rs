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
    regime_kind_name, govt_form_for, office_title_for, govt_kind_key,
    trait_name, role_name, directed_edict_phrase,
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
    /// 2026-09-30 · the council window's seat card — the seat holder's own
    /// character, read off their `Individual` (empty when a seat carries no
    /// linked person, e.g. an old save's plain name).
    #[serde(default)] pub traits: Vec<String>,
    /// The four ideology axes (Authority · Tradition · Openness · Economy, −5..+5).
    #[serde(default)] pub ideology: [f32; 4],
    #[serde(default)] pub ideology_seeded: bool,
    #[serde(default)] pub age: u32,
    #[serde(default)] pub female: bool,
    #[serde(default)] pub famous: bool,
    #[serde(default)] pub fame: f32,
    #[serde(default)] pub roles: Vec<String>,
    /// −1 against … +1 for the debate in progress, as this seat would lean
    /// this round (`CampaignSim::official_edict_lean`); 0 with no debate.
    #[serde(default)] pub vote_lean: f32,
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
    /// +1 open/enact · −1 restrict/repeal (`GovEdict.dir`).
    #[serde(default)] pub dir: i8,
    /// "open the gates to foreigners" — the measure in words.
    #[serde(default)] pub what: String,
    /// Proposed by the prevailing ideology's agenda.
    #[serde(default)] pub agenda: bool,
}

/// One edict in force.
#[derive(Serialize, Clone, Default)]
pub struct EdictBrief {
    pub family: String,
    pub tag: i8,
    #[serde(default)] pub what: String,
    pub major: bool,
    pub enacted_year: u32,
    pub expires_year: u32,
}

/// One closed debate's outcome, or a change of government — "recent history".
#[derive(Serialize, Clone, Default)]
pub struct GovHistoryBrief {
    pub year: u32,
    pub family: String,
    /// "passed" | "failed" | "deadlocked" | "coup".
    pub outcome: String,
    /// Q04.5b · "coup" | "revolution" | "oligarchic closing" | "emergency
    /// rule" | "succession crisis" | "reform" | "ostracism" | "" (an
    /// ordinary edict/Lustrum entry, `outcome != "coup"`).
    pub regime_kind: String,
}

/// The whole Government window for one city.
#[derive(Serialize, Clone, Default)]
pub struct GovernmentBrief {
    pub hub: u32,
    pub city: String,
    /// "Merchant Council (Doge)" | "Principality (Prince)" | "Free Commune (Mayor)".
    pub form: String,
    /// "council" | "ruler" | "assembly" — the stable badge key, independent
    /// of the culture's own NAME for its form.
    #[serde(default)] pub form_kind: String,
    /// The head's title in this culture ("Doge", "Jarl", "Khan"…).
    #[serde(default)] pub head_title: String,
    /// The prevailing named ideology in this city, "" if none holds.
    #[serde(default)] pub dominant_ideology: String,
    /// The dominant ideology's demands as edict family names, each with
    /// whether a live edict meets it right now.
    #[serde(default)] pub demands: Vec<(String, bool)>,
    pub legitimacy: f32,
    pub gov_points: f32,
    /// −1 conservative .. +1 libertarian.
    pub gov_position: f32,
    pub seats: Vec<SeatBrief>,
    pub blocs: Vec<BlocBrief>,
    pub debate: Option<DebateBrief>,
    pub edicts: Vec<EdictBrief>,
    pub history: Vec<GovHistoryBrief>,
    /// 2026-09-30b · ONE CIVIC AGENDA (Q05.3): every culture-tier question
    /// the council has open, listed beside its ordinary motion. The two
    /// still debate on their own tracks in the sim (§5.10 — a single slot
    /// would starve one or the other); what changed is that the council
    /// window shows everything before the chamber in one place.
    #[serde(default)] pub culture_motions: Vec<CultureMotionBrief>,
}

/// One open culture-tier proposal (row 05's shadow debate), for the agenda.
#[derive(Serialize, Clone)]
pub struct CultureMotionBrief {
    pub culture: String,
    pub from_tier: String,
    pub to_tier: String,
    /// true = toward full citizenship, false = toward exclusion.
    pub warmer: bool,
    pub round: u8,
    pub tally: f32,
    pub reason: String,
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

fn seat_brief(
    sim: &crate::sim::tick::CampaignSim, h: usize, o: &Official,
    house_name: &dyn Fn(i32) -> String, govt_type: u8, pop: f32, kit: Option<usize>,
) -> SeatBrief {
    let person = if o.individual_id >= 0 {
        sim.people.iter().find(|p| p.id as i32 == o.individual_id)
    } else { None };
    let vote_lean = sim.hubs[h].gov_debate.as_ref()
        .map(|d| sim.official_edict_lean(h, o, d.family, d.tag, d.dir))
        .unwrap_or(0.0);
    SeatBrief {
        role: o.role,
        office_title: office_title_for(o.role, govt_type, pop, kit),
        name: o.name.clone(),
        individual_id: o.individual_id,
        path: official_path_name(o.path).to_string(),
        suitability: o.suitability,
        allegiance: official_allegiance(o),
        house: o.house,
        house_name: house_name(o.house),
        control: o.control,
        traits: person.map(|p| p.traits.iter().map(|&(t, _)| trait_name(t).to_string()).collect()).unwrap_or_default(),
        ideology: person.map(|p| p.ideology).unwrap_or([0.0; 4]),
        ideology_seeded: person.map(|p| p.ideology_seeded).unwrap_or(false),
        age: person.map(|p| sim.tick.saturating_sub(p.birth_tick) / TICKS_PER_YEAR).unwrap_or(0),
        female: person.map(|p| p.female).unwrap_or(false),
        famous: person.map(|p| p.famous).unwrap_or(false),
        fame: person.map(|p| p.fame).unwrap_or(0.0),
        roles: person.map(|p| p.roles.iter().map(|&r| role_name(r).to_string()).collect()).unwrap_or_default(),
        vote_lean,
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
    let culture = sim.hub_culture.get(h).cloned().unwrap_or_default();
    let kit = crate::sim::cultures::kit_of_people(&culture);
    let (form, head_title) = govt_form_for(hb.govt_type, hb.population, kit);
    let seats: Vec<SeatBrief> = hb.officials.iter()
        .map(|o| seat_brief(&sim, h, o, &house_name, hb.govt_type, hb.population, kit)).collect();
    let dom = sim.ideologies.iter().find(|i| i.id as i32 == hb.ideology_dominant);
    let dominant_ideology = dom.map(|i| i.name.clone()).unwrap_or_default();
    let demands: Vec<(String, bool)> = dom.map(|i| i.demands.iter().map(|&(fam, sign)| {
        let met = hb.gov_edicts.iter().any(|e| e.family == fam && e.dir == sign);
        (directed_edict_phrase(fam, sign).to_string(), met)
    }).collect()).unwrap_or_default();
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
        dir: d.dir, what: directed_edict_phrase(d.family, d.dir).to_string(), agenda: d.agenda,
    });
    let edicts: Vec<EdictBrief> = hb.gov_edicts.iter().map(|e| EdictBrief {
        family: edict_family_name(e.family).to_string(),
        what: directed_edict_phrase(e.family, e.dir).to_string(),
        tag: e.tag, major: e.major,
        enacted_year: e.enacted_tick / TICKS_PER_YEAR,
        expires_year: e.expires_tick / TICKS_PER_YEAR,
    }).collect();
    let history: Vec<GovHistoryBrief> = hb.gov_history.iter().map(|e| GovHistoryBrief {
        year: e.tick / TICKS_PER_YEAR,
        family: edict_family_name(e.family).to_string(),
        outcome: outcome_name(e.outcome).to_string(),
        regime_kind: regime_kind_name(e.regime_kind).to_string(),
    }).collect();

    Ok(Some(GovernmentBrief {
        hub, city: hb.name.clone(), form,
        form_kind: govt_kind_key(hb.govt_type).to_string(), head_title, dominant_ideology, demands,
        legitimacy: hb.legitimacy, gov_points: hb.gov_points, gov_position: hb.gov_position,
        seats, blocs, debate, edicts, history,
        culture_motions: hb.culture_relations.iter().filter(|r| r.proposed_tier >= 1).map(|r| CultureMotionBrief {
            culture: r.culture.clone(),
            from_tier: crate::sim::tick::acceptance_tier_name(r.tier).to_string(),
            to_tier: crate::sim::tick::acceptance_tier_name(r.proposed_tier as u8).to_string(),
            warmer: (r.proposed_tier as u8) < r.tier,
            round: r.debate_round,
            tally: r.debate_tally,
            reason: r.reason.clone(),
        }).collect(),
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
        what: directed_edict_phrase(e.family, e.dir).to_string(),
        tag: e.tag, major: e.major,
        enacted_year: e.enacted_tick / TICKS_PER_YEAR,
        expires_year: e.expires_tick / TICKS_PER_YEAR,
    }).collect())
}

