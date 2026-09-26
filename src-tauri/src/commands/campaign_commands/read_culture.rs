//! read_culture commands — `docs/living_world/05_CULTURE_ACCEPTANCE.md`
//! slice 05.6, the settlement's culture-acceptance table.
//!
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;
use crate::sim::tick::{TickHub, acceptance_tier_name};

/// One (city, culture) relation, browser-ready.
#[derive(Serialize, Clone, Default)]
pub struct CultureAcceptanceBrief {
    pub culture: String,
    pub tier: u8,
    pub tier_name: String,
    pub score: f32,
    pub trend: f32,
    pub reason: String,
    /// Approximate resident share (0..1) — the majority entry reads 1.0
    /// minus every listed minority, a foreign-trade-only entry (none exist
    /// yet in this slice) would read 0.0.
    pub residents_frac: f32,
    /// `-1` while no tier-change proposal is open, else the target tier.
    pub proposed_tier: i8,
}

/// The culture table for one city — every culture this hub has a sparse
/// relation for (05.1's own "read by nothing until 05.2/05.6" data, now
/// surfaced). Empty (not an error) for an estate, an abandoned hub, or a hub
/// the yearly pass has not reached yet.
#[tauri::command]
pub fn campaign_get_culture_acceptance(hub: u32, db: State<'_, WorldDb>) -> Result<Vec<CultureAcceptanceBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let h = hub as usize;
    let Some(hb): Option<&TickHub> = sim.hubs.get(h) else { return Ok(vec![]) };
    if hb.culture_relations.is_empty() { return Ok(vec![]); }
    let majority = sim.hub_culture.get(h).cloned().unwrap_or_default();
    let minorities = sim.hub_minorities.get(h).cloned().unwrap_or_default();
    let maj_minority_sum: f32 = minorities.iter().map(|(_, s)| *s).sum();
    let out = hb.culture_relations.iter().map(|r| {
        let residents_frac = if r.culture == majority {
            (1.0 - maj_minority_sum).max(0.0)
        } else {
            minorities.iter().find(|(c, _)| *c == r.culture).map(|(_, s)| *s).unwrap_or(0.0)
        };
        CultureAcceptanceBrief {
            culture: r.culture.clone(),
            tier: r.tier,
            tier_name: acceptance_tier_name(r.tier).to_string(),
            score: r.score,
            trend: r.trend,
            reason: r.reason.clone(),
            residents_frac,
            proposed_tier: r.proposed_tier,
        }
    }).collect();
    Ok(out)
}
