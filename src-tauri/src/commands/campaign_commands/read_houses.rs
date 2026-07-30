//! read_houses commands — the House Dossier's own reads: the five STABILITY gauges
//! and the FEUD board. Split out from `read_hubs.rs` (which already carries the house
//! briefs) because both of these answer a different question about a house: not "what
//! does it own" but "how close is it to dying, and who is it fighting".
//!
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;
use crate::sim::tick::{
    FEUD_CAUSES, FEUD_ENDINGS, FEUD_RUNNING, FEUD_STAGES, House, TICKS_PER_YEAR,
};

// ═════════════════════════════════════════════════════════════════════════════════
//  STABILITY
//
//  The sim already knows exactly how close a house is to death — `debt_since` is a
//  literal 12-month countdown in `update_solvency` — and none of it reached the UI, so
//  a house vanished between two advances. Four of the five gauges below are pure
//  derivations of state that already existed; only cohesion needed anything new, and
//  it is derived here too rather than stored, so nothing about the simulation changes.
// ═════════════════════════════════════════════════════════════════════════════════

/// One stability gauge: a 0..1 score, a five-pip level, and the PHRASE that is the
/// actual product (a raw 0..1 tells a player nothing).
#[derive(Serialize, Clone)]
pub struct Gauge {
    /// "solvency" | "liquidity" | "exposure" | "succession" | "cohesion".
    pub key: String,
    pub label: String,
    /// 0..1, higher is healthier.
    pub score: f32,
    /// 1..5 pips, for comparing houses at a glance.
    pub pips: u8,
    /// The human reading — "in the red 7 of 12 months", "4 months of runway".
    pub phrase: String,
    /// True when this gauge is the one to worry about (drives the warning colour).
    pub warn: bool,
}

/// One liability line on the dossier's Ledger tab.
#[derive(Serialize, Clone)]
pub struct Liability {
    pub label: String,
    pub amount: f32,
    /// Extra context — the borrower polis's war state, the contract's buyer, etc.
    pub note: String,
}

/// The House Dossier header: five gauges plus the liability breakdown behind them.
#[derive(Serialize)]
pub struct HouseStability {
    pub idx: u32,
    pub name: String,
    pub gauges: Vec<Gauge>,
    /// Monthly burn: what the family pays out before it earns anything.
    pub monthly_burn: f32,
    pub liquid: f32,
    pub liabilities: Vec<Liability>,
    pub liabilities_total: f32,
    /// Months already spent in the red (0 = solvent), and the limit before bankruptcy.
    pub debt_months: u32,
    pub debt_limit: u32,
    /// Share of cumulative profit from the single largest good, and its name.
    pub top_good_share: f32,
    pub top_good: String,
    /// Share of commercial influence held in the single most important city.
    pub top_city_share: f32,
    pub top_city: String,
    /// Years the current head has served, and the expected span.
    pub head_years: u32,
    pub head_span_years: u32,
    /// Live feud count and how many are at trade-war or worse.
    pub feuds_live: u32,
    pub feuds_hot: u32,
}

fn pips(score: f32) -> u8 { (1.0 + score.clamp(0.0, 1.0) * 4.0).round().clamp(1.0, 5.0) as u8 }

/// Herfindahl concentration over a set of weights: 0 = perfectly spread, 1 = all in one.
fn concentration(weights: &[f32]) -> (f32, usize) {
    let total: f32 = weights.iter().map(|w| w.max(0.0)).sum();
    if total <= 0.0 { return (0.0, usize::MAX); }
    let mut top = (0.0f32, usize::MAX);
    for (i, w) in weights.iter().enumerate() {
        let s = w.max(0.0) / total;
        if s > top.0 { top = (s, i); }
    }
    (top.0, top.1)
}

/// Everything the dossier header needs for one house. Read-only.
#[tauri::command]
pub fn campaign_house_stability(idx: u32, db: State<'_, WorldDb>) -> Result<Option<HouseStability>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let hi = idx as usize;
    let h: &House = match sim.houses.get(hi) { Some(h) => h, None => return Ok(None) };

    // ── Monthly burn. Rebuilt from the same terms `apply_wealth_sinks` /
    //    `manage_fleets` actually charge, so the runway figure is the real one.
    let mut burn = 0.0f32;
    let mut liabilities: Vec<Liability> = Vec::new();
    let wh: f32 = sim.warehouses.iter()
        .filter(|w| w.owner == hi as i32)
        .map(|w| crate::sim::tick::CAP_UPKEEP * w.capacity * sim.city_size_factor(w.hub as usize))
        .sum();
    if wh > 0.0 {
        let n = sim.warehouses.iter().filter(|w| w.owner == hi as i32).count();
        liabilities.push(Liability { label: "Warehouse upkeep".into(), amount: wh,
            note: format!("{} depot{}", n, if n == 1 { "" } else { "s" }) });
    }
    burn += wh;
    let estates = sim.hubs.iter().filter(|x| x.is_estate && x.owner_house == hi as i32).count();
    if estates > 0 {
        let e = estates as f32 * crate::sim::tick::UPKEEP_WAREHOUSE_BASE
            * crate::sim::tick::UPKEEP_ESTATE_FRAC;
        liabilities.push(Liability { label: "Estate depots".into(), amount: e,
            note: format!("{} holding{}", estates, if estates == 1 { "" } else { "s" }) });
        burn += e;
    }
    let fleet = h.fleet_sea + h.fleet_river + h.fleet_caravan;
    if fleet > 0 {
        // Fleet upkeep is charged as a fraction of the fleet's replacement value.
        let f = fleet as f32 * crate::sim::tick::SHIP_COST * crate::sim::tick::FLEET_UPKEEP_FRAC;
        liabilities.push(Liability { label: "Fleet upkeep".into(), amount: f,
            note: format!("{} sea · {} river · {} caravan", h.fleet_sea, h.fleet_river, h.fleet_caravan) });
        burn += f;
    }
    let leases = h.office_leases.len();
    if leases > 0 {
        let r = leases as f32 * crate::sim::tick::OFFICE_LEASE_RENT;
        liabilities.push(Liability { label: "Office rents".into(), amount: r,
            note: format!("{} leased", leases) });
        burn += r;
    }
    // ── Debts owed as a BORROWER, with sovereign loans called out separately. The
    //    Bardi/Peruzzi line: a loan to a polis at war is not the same asset class as a
    //    trade loan, and the dossier should not pretend otherwise.
    let mut owed = 0.0f32;
    for b in &sim.banks {
        for l in &b.loans {
            if l.borrower_house != hi as i32 || l.outstanding <= 0.0 { continue; }
            owed += l.outstanding;
        }
    }
    if owed > 0.0 {
        liabilities.push(Liability { label: "Bank debt outstanding".into(), amount: owed,
            note: "principal still to repay".into() });
    }
    let mut sovereign = 0.0f32;
    let mut sovereign_at_war = 0u32;
    for b in sim.banks.iter().filter(|b| b.house == hi as u32 && !b.defunct) {
        for l in &b.loans {
            if l.borrower_polis < 0 || l.outstanding <= 0.0 { continue; }
            sovereign += l.outstanding;
            if sim.hubs.get(l.borrower_polis as usize).map(|x| x.war_with >= 0).unwrap_or(false) {
                sovereign_at_war += 1;
            }
        }
    }
    if sovereign > 0.0 {
        liabilities.push(Liability {
            label: "Lent to city treasuries".into(), amount: sovereign,
            note: if sovereign_at_war > 0 {
                format!("{} borrower{} at war — a default would land here",
                    sovereign_at_war, if sovereign_at_war == 1 { "" } else { "s" })
            } else { "sovereign exposure".into() },
        });
    }
    // Contract penalty exposure (capped to owner wealth by limited liability).
    let live_contracts = sim.contracts.iter()
        .filter(|c| c.seller_house == hi as u32 && c.end_tick > sim.tick).count();
    if live_contracts > 0 {
        let pen = (h.wealth.max(0.0) * 0.15).min(live_contracts as f32 * 60.0);
        liabilities.push(Liability { label: "Contract penalties at risk".into(), amount: pen,
            note: format!("{} live contract{}", live_contracts,
                if live_contracts == 1 { "" } else { "s" }) });
    }
    let liabilities_total: f32 = liabilities.iter().map(|l| l.amount).sum();

    // ── 1. SOLVENCY — the countdown that already decides whether this house lives.
    let debt_months = if h.debt_since > 0 {
        (sim.tick.saturating_sub(h.debt_since) / 30).min(12)
    } else { 0 };
    let solvency_score = if h.wealth < 0.0 {
        (1.0 - debt_months as f32 / 12.0).clamp(0.0, 0.35)
    } else {
        // Solvent: how much cover the family has over what it owes.
        let cover = if liabilities_total > 0.0 { h.wealth / (liabilities_total * 4.0) } else { 2.0 };
        (0.35 + cover.clamp(0.0, 1.0) * 0.65).clamp(0.35, 1.0)
    };
    let solvency_phrase = if h.is_guild && h.wealth < 0.0 {
        "in deficit — the city covers a guild".to_string()
    } else if h.wealth < 0.0 {
        format!("in the red {} of 12 months — ruin in {}", debt_months, 12 - debt_months)
    } else if liabilities_total > h.wealth.max(0.0) {
        "owes more than it holds".to_string()
    } else { "secure".to_string() };

    // ── 2. LIQUIDITY — months of runway at the current burn.
    let liquid = h.wealth;
    let runway = if burn > 0.0 { (liquid.max(0.0) / burn).min(99.0) } else { 99.0 };
    let liquidity_score = (runway / 18.0).clamp(0.0, 1.0);
    let liquidity_phrase = if burn <= 0.0 { "nothing to pay for".to_string() }
        else if runway >= 99.0 { "no pressure".to_string() }
        else { format!("{:.0} month{} of runway", runway, if runway < 1.5 { "" } else { "s" }) };

    // ── 3. EXPOSURE — concentration, not size. A house earning most of its living
    //    from one good in one city is one blockade away from ruin.
    let (good_share, good_i) = concentration(&h.good_profit);
    let city_weights: Vec<f32> = h.influence.iter().map(|(_, v)| *v).collect();
    let (city_share, city_i) = concentration(&city_weights);
    let feuds_live = sim.feuds.iter()
        .filter(|f| f.outcome == FEUD_RUNNING && (f.a as usize == hi || f.b as usize == hi))
        .count() as u32;
    let feuds_hot = sim.feuds.iter()
        .filter(|f| f.outcome == FEUD_RUNNING && f.stage >= 2
            && (f.a as usize == hi || f.b as usize == hi))
        .count() as u32;
    let barred = sim.house_barred.get(hi).map(|v| v.len()).unwrap_or(0) as u32;
    let exposure_bad = (0.55 * good_share.max(city_share)
        + 0.10 * feuds_hot as f32 + 0.06 * barred as f32).clamp(0.0, 1.0);
    let exposure_score = 1.0 - exposure_bad;
    let top_good = good_i.and_then_name(&sim, true);
    let top_city = h.influence.get(city_i.min(h.influence.len().saturating_sub(1)))
        .and_then(|(c, _)| sim.hubs.get(*c as usize)).map(|x| x.name.clone()).unwrap_or_default();
    let exposure_phrase = if feuds_hot > 0 {
        format!("{} feud{} at trade war or worse", feuds_hot, if feuds_hot == 1 { "" } else { "s" })
    } else if good_share > 0.6 && !top_good.is_empty() {
        format!("{:.0}% of its living from {}", good_share * 100.0, top_good)
    } else if city_share > 0.7 && !top_city.is_empty() {
        format!("{:.0}% of its standing in {}", city_share * 100.0, top_city)
    } else if barred > 0 {
        format!("shut out of {} market{}", barred, if barred == 1 { "" } else { "s" })
    } else { "spread across trades and cities".to_string() };

    // ── 4. SUCCESSION — how late in life the head is. A house whose head is near the
    //    end of his span with no proven heir is a house about to be tested.
    let head_years = sim.tick.saturating_sub(h.head_since) / TICKS_PER_YEAR;
    let head_span_years = h.head_lifespan / TICKS_PER_YEAR;
    let through = if h.head_lifespan > 0 {
        (sim.tick.saturating_sub(h.head_since) as f32 / h.head_lifespan as f32).clamp(0.0, 1.0)
    } else { 0.0 };
    let succession_score = 1.0 - through * 0.85;
    let succession_phrase = if through > 0.9 { "the head is very old".to_string() }
        else if through > 0.7 { format!("head {} years in, late in life", head_years) }
        else { format!("head {} years in of about {}", head_years, head_span_years.max(1)) };

    // ── 5. COHESION — can the family still act as one? Reach without standing pulls a
    //    house apart: many distant nodes, contested cities, a weak head, open feuds.
    let nodes = 1 + h.offices.len() + h.bailos.len();
    let contested = h.influence.iter().filter(|(c, v)| {
        *v > 0.05 && sim.houses.iter().enumerate().any(|(j, o)| {
            j != hi && !o.defunct && o.influence.iter().any(|(c2, v2)| c2 == c && *v2 > *v * 0.7)
        })
    }).count();
    let stretch = ((nodes as f32 - 3.0).max(0.0) / 10.0).clamp(0.0, 1.0);
    let cohesion_score = (1.0
        - 0.35 * stretch
        - 0.08 * contested as f32
        - 0.10 * feuds_live as f32
        + 0.15 * (h.prestige.min(1.0))).clamp(0.0, 1.0);
    let cohesion_phrase = if stretch > 0.6 {
        format!("stretched across {} cities", nodes)
    } else if contested > 1 {
        format!("{} cities contested by rivals", contested)
    } else if feuds_live > 2 {
        format!("{} quarrels running at once", feuds_live)
    } else { "holds together".to_string() };

    let mk = |key: &str, label: &str, score: f32, phrase: String| Gauge {
        key: key.into(), label: label.into(), score, pips: pips(score), phrase,
        // A gauge that is fine stays quiet — five permanently-amber dials teach a
        // player to ignore all five.
        warn: pips(score) <= 2,
    };
    Ok(Some(HouseStability {
        idx, name: h.name.clone(),
        gauges: vec![
            mk("solvency", "Solvency", solvency_score, solvency_phrase),
            mk("liquidity", "Liquidity", liquidity_score, liquidity_phrase),
            mk("exposure", "Exposure", exposure_score, exposure_phrase),
            mk("succession", "Succession", succession_score, succession_phrase),
            mk("cohesion", "Cohesion", cohesion_score, cohesion_phrase),
        ],
        monthly_burn: burn, liquid, liabilities, liabilities_total,
        debt_months, debt_limit: 12,
        top_good_share: good_share, top_good,
        top_city_share: city_share, top_city,
        head_years, head_span_years,
        feuds_live, feuds_hot,
    }))
}

/// Tiny helper so the concentration index can be turned into a good name without
/// repeating the bounds dance at each call site.
trait GoodName { fn and_then_name(self, sim: &CampaignSim, _good: bool) -> String; }
impl GoodName for usize {
    fn and_then_name(self, sim: &CampaignSim, _good: bool) -> String {
        sim.goods.get(self).map(|g| g.name.clone()).unwrap_or_default()
    }
}

// ═════════════════════════════════════════════════════════════════════════════════
//  FEUDS
// ═════════════════════════════════════════════════════════════════════════════════

/// One flare in a feud's history, as the panel shows it.
#[derive(Serialize, Clone)]
pub struct FeudFlareRow {
    pub year: u32,
    pub stage: String,
    pub loser: String,
    pub cost: f32,
    pub text: String,
}

/// A feud as the Houses panel / dossier shows it: who, why, how hot, how it ended.
#[derive(Serialize, Clone)]
pub struct FeudRow {
    pub a: u32,
    pub b: u32,
    pub a_name: String,
    pub b_name: String,
    pub a_color: String,
    pub b_color: String,
    /// Why it began, already in prose ("the same trade").
    pub cause: String,
    /// "cold rivalry" | "open feud" | "trade war" | "vendetta".
    pub stage: String,
    pub stage_idx: u8,
    /// 0..1 temperature.
    pub intensity: f32,
    pub good: String,
    pub city: String,
    pub started_year: u32,
    pub years: u32,
    pub flares: u32,
    /// Cumulative wealth each side has lost to the other.
    pub damage_a: f32,
    pub damage_b: f32,
    /// "running" | "arbitrated" | "sealed by marriage" | "ended in ruin" | "cooled".
    pub outcome: String,
    pub running: bool,
    pub ended_year: u32,
    pub log: Vec<FeudFlareRow>,
}

/// Every feud, live first then settled, most intense first within each group. When
/// `house` is ≥ 0 only that family's quarrels are returned.
#[tauri::command]
pub fn campaign_get_feuds(house: i32, db: State<'_, WorldDb>) -> Result<Vec<FeudRow>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let name = |i: u32| sim.houses.get(i as usize).map(|h| h.name.clone()).unwrap_or_default();
    let mut rows: Vec<FeudRow> = sim.feuds.iter()
        .filter(|f| house < 0 || f.a as i32 == house || f.b as i32 == house)
        .map(|f| {
            let running = f.outcome == FEUD_RUNNING;
            let end_tick = if running { sim.tick } else { f.ended_tick };
            FeudRow {
                a: f.a, b: f.b, a_name: name(f.a), b_name: name(f.b),
                a_color: distinct_color(f.a as usize), b_color: distinct_color(f.b as usize),
                cause: FEUD_CAUSES.get(f.cause as usize).copied().unwrap_or("an old grievance").into(),
                stage: FEUD_STAGES.get(f.stage as usize).copied().unwrap_or("rivalry").into(),
                stage_idx: f.stage,
                intensity: f.intensity,
                good: if f.good >= 0 {
                    sim.goods.get(f.good as usize).map(|g| g.name.clone()).unwrap_or_default()
                } else { String::new() },
                city: if f.hub >= 0 {
                    sim.hubs.get(f.hub as usize).map(|h| h.name.clone()).unwrap_or_default()
                } else { String::new() },
                started_year: f.started_tick / TICKS_PER_YEAR,
                years: end_tick.saturating_sub(f.started_tick) / TICKS_PER_YEAR,
                flares: f.flares,
                damage_a: f.damage_a, damage_b: f.damage_b,
                outcome: FEUD_ENDINGS.get(f.outcome as usize).copied().unwrap_or("running").into(),
                running,
                ended_year: if running { 0 } else { f.ended_tick / TICKS_PER_YEAR },
                log: f.log.iter().map(|l| FeudFlareRow {
                    year: l.tick / TICKS_PER_YEAR,
                    stage: FEUD_STAGES.get(l.stage as usize).copied().unwrap_or("rivalry").into(),
                    loser: name(l.loser), cost: l.cost, text: l.text.clone(),
                }).collect(),
            }
        }).collect();
    rows.sort_by(|x, y| {
        y.running.cmp(&x.running)
            .then(y.intensity.partial_cmp(&x.intensity).unwrap_or(std::cmp::Ordering::Equal))
            .then(x.a.cmp(&y.a))
    });
    Ok(rows)
}

// ── Phase 2.1/2.2/2.3 · the Kin roster ──────────────────────────────────────────

/// One kin, for the dossier's 👪 Kin tab.
#[derive(Serialize)]
pub struct KinBrief {
    pub name: String,
    pub female: bool,
    pub age: u32,
    /// "head" | "heir" | "factor" | "idle" | "married out" | "dead".
    pub role: String,
    /// The holding they run, if `role == "factor"` (empty if unposted).
    pub posted_name: String,
    /// True if `posted_name` is family-run (this kin), else it's a hired factor —
    /// resolved per-holding by `campaign_holding_authorship` below, not stored here.
    pub loyalty: f32,
    pub skill: f32,
    /// A phrase, never four raw numbers (§3's own discipline) — e.g. "Bold, grasping,
    /// and rooted at home." Empty for a middling, unremarkable character.
    pub character_phrase: String,
    /// Share of the house's internal "power" this kin holds, 0..100, summing to 100
    /// across the roster (Phase 2.6) — head weighted highest, then role and skill.
    pub power_share: f32,
}

/// This house's kin roster, for the dossier's 👪 Kin tab. Empty for a guild, or any
/// house whose roster was never generated (both read as "no roster", not an error).
#[tauri::command]
pub fn campaign_get_house_kin(idx: u32, db: State<'_, WorldDb>) -> Result<Vec<KinBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let Some(h) = sim.houses.get(idx as usize) else { return Ok(vec![]) };
    let tick = sim.tick;
    let shares = crate::sim::tick::kin_power_shares(&h.kin);
    const ROLE_NAMES: [&str; 6] = ["head", "heir", "factor", "idle", "married out", "dead"];
    Ok(h.kin.iter().zip(shares.iter()).map(|(k, &share)| {
        let age = tick.saturating_sub(k.born_tick) / TICKS_PER_YEAR;
        let posted_name = if k.posted >= 0 {
            sim.hubs.get(k.posted as usize).map(|x| x.name.clone()).unwrap_or_default()
        } else { String::new() };
        KinBrief {
            name: k.name.clone(), female: k.female, age,
            role: ROLE_NAMES.get(k.role.min(5) as usize).copied().unwrap_or("idle").into(),
            posted_name, loyalty: k.loyalty, skill: k.skill,
            character_phrase: crate::sim::tick::character_phrase(k.character),
            power_share: share,
        }
    }).collect())
}

// ── Phase 3.1 · goals ────────────────────────────────────────────────────────────

/// One ambition, active or historical, for the dossier's 🎯 Ambitions tab.
#[derive(Serialize)]
pub struct GoalBrief {
    /// A phrase — "cornering the silk trade", never a raw kind code.
    pub what: String,
    /// 0 pursuing · 1 achieved · 2 failed · 3 abandoned.
    pub state: u8,
    pub set_year: u32,
    pub deadline_year: u32,
    /// 0..1 where the kind has an honest fraction (corner-trade/charter-bank hold
    /// years); -1 where it doesn't (a binary goal, or the target-wealth kinds) — the
    /// dossier shows a phrase there instead of a bar, never a fabricated percentage.
    pub progress_frac: f32,
}

#[derive(Serialize)]
pub struct GoalsBrief {
    pub active: Vec<GoalBrief>,
    /// Most recent first.
    pub history: Vec<GoalBrief>,
}

fn goal_brief(sim: &CampaignSim, hi: usize, g: &crate::sim::tick::Goal) -> GoalBrief {
    use crate::sim::tick::{GOAL_CORNER_TRADE, GOAL_CHARTER_BANK, GOAL_RESTORE_HOUSE,
        GOAL_SEAT_COUNCIL, GOAL_RAISE_BAILO, GOAL_REACH_PROVINCE, GOAL_OUTLAST_RIVAL,
        GOAL_HOLD_YEARS_TRADE, GOAL_HOLD_YEARS_BANK};
    let gname = |g: usize| sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
    let hname = |h: usize| sim.hubs.get(h).map(|x| x.name.clone()).unwrap_or_default();
    let what = match g.kind {
        GOAL_CORNER_TRADE => format!("cornering the {} trade", gname(g.target_good.max(0) as usize)),
        GOAL_SEAT_COUNCIL => format!("seating the council of {}", hname(g.target_hub.max(0) as usize)),
        GOAL_RAISE_BAILO => format!("raising a bailo at {}", hname(g.target_hub.max(0) as usize)),
        GOAL_CHARTER_BANK => "chartering a bank".into(),
        GOAL_REACH_PROVINCE => "reaching the far province".into(),
        GOAL_OUTLAST_RIVAL => format!("outlasting {}",
            sim.houses.get(g.target_house.max(0) as usize).map(|h| h.name.clone()).unwrap_or_default()),
        GOAL_RESTORE_HOUSE => "restoring the house's former wealth".into(),
        _ => "an ambition".into(),
    };
    let progress_frac = match g.kind {
        GOAL_CORNER_TRADE => (g.progress / GOAL_HOLD_YEARS_TRADE).clamp(0.0, 1.0),
        GOAL_CHARTER_BANK => (g.progress / GOAL_HOLD_YEARS_BANK).clamp(0.0, 1.0),
        GOAL_RESTORE_HOUSE => {
            let target = g.progress.max(1e-6);
            (sim.houses.get(hi).map(|h| h.wealth).unwrap_or(0.0) / target).clamp(0.0, 1.0)
        }
        _ => -1.0,
    };
    GoalBrief {
        what, state: g.state,
        set_year: g.set_tick / TICKS_PER_YEAR, deadline_year: g.deadline_tick / TICKS_PER_YEAR,
        progress_frac,
    }
}

/// This house's ambitions, active and historical, for the dossier's 🎯 Ambitions tab.
#[tauri::command]
pub fn campaign_get_house_goals(idx: u32, db: State<'_, WorldDb>) -> Result<GoalsBrief, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(GoalsBrief { active: vec![], history: vec![] }) };
    let hi = idx as usize;
    let Some(h) = sim.houses.get(hi) else { return Ok(GoalsBrief { active: vec![], history: vec![] }) };
    let active = h.goals.iter().map(|g| goal_brief(&sim, hi, g)).collect();
    let history = h.goal_history.iter().rev().take(12).map(|g| goal_brief(&sim, hi, g)).collect();
    Ok(GoalsBrief { active, history })
}
