//! World/campaign split: the WORLD (tiles + metadata: geography, climate,
//! rivers, goods belts…) is frozen by `finalize_world`; everything human —
//! settlements, economy, step 7-10 progress — lives in the `campaign` table
//! and is saved/opened as a separate `.campaign` file referencing the world by
//! its frozen fingerprint.
//!
//! Campaign steps (settlements, biological-trade) still write DERIVED tile
//! columns (habitability, hazard, goods belts) on a frozen world — that's
//! allowed: the freeze protects user-authored geography (paint + phases 1-6),
//! and the campaign's `world_ref` is the fingerprint RECORDED AT FINALIZE,
//! which later derived-column writes don't touch.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::db::{metadata, WorldDb};

/// Extract a human-readable message from a `catch_unwind` panic payload (the
/// payload is `&str` for `panic!("…")` and `String` for `format!`-style panics;
/// anything else is opaque). The full file:line is captured separately by the
/// startup panic hook in `lib.rs`.
fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic (see the panic log in the app data dir)".to_string()
    }
}

/// Campaign keys carried by `.campaign` files (and stripped from world saves
/// via the whole-table strip — this list is what save/open copies explicitly).
const CAMPAIGN_KEYS: [&str; 7] = [
    "name",
    "settlements",
    "economy",
    "bio_params",
    "campaign_progress",
    "world_ref",
    // DLC 1 "Living Trade" tick-simulation state (serialized CampaignSim incl.
    // the append-only journal). Travels with the .campaign file.
    "campaign_sim",
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldRef {
    pub fingerprint: (i64, i64),
    pub grid_width: u32,
    pub grid_height: u32,
    pub world_name: String,
}

#[derive(Serialize)]
pub struct CampaignInfo {
    pub name: String,
    /// False when the campaign was made on a different (or re-finalized) world.
    pub world_match: bool,
    /// JSON step-completion map for the campaign wizard (steps 7-10).
    pub campaign_progress: Option<String>,
}

/// Fingerprint of the world's base tiles (lod 0 only — the LOD pyramid is a
/// derived cache). Stable across save/open round-trips because the SQLite
/// backup copies tile versions verbatim.
pub fn world_fingerprint(conn: &Connection) -> Result<(i64, i64), String> {
    conn.query_row(
        "SELECT COALESCE(SUM(version), 0), COUNT(*) FROM tiles WHERE lod = 0",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| e.to_string())
}

pub fn is_frozen(conn: &Connection) -> bool {
    metadata::get_meta(conn, "frozen").ok().flatten().as_deref() == Some("1")
}

/// Guard for commands that edit world geography (paint, template import,
/// sim phases 1-6 and the run-alls).
pub fn ensure_unfrozen(conn: &Connection) -> Result<(), String> {
    if is_frozen(conn) {
        return Err(
            "World is finalized (frozen). Unfreeze it to edit geography — note that \
             existing campaigns will no longer match the changed world."
                .into(),
        );
    }
    Ok(())
}

fn finalized_fp(conn: &Connection) -> Option<(i64, i64)> {
    let s = metadata::get_meta(conn, "finalized_fp").ok().flatten()?;
    let (a, b) = s.split_once(',')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

fn current_world_ref(conn: &Connection) -> Result<WorldRef, String> {
    let fingerprint = finalized_fp(conn)
        .ok_or_else(|| "Finalize the world before starting a campaign.".to_string())?;
    let grid_width: u32 = metadata::get_meta_required(conn, "grid_width")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let grid_height: u32 = metadata::get_meta_required(conn, "grid_height")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let world_name = metadata::get_meta(conn, "name")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Untitled".to_string());
    Ok(WorldRef { fingerprint, grid_width, grid_height, world_name })
}

// ═══════════════════════════════════════════════════════════════════════════
// DLC 1 "Living Trade" — tick simulation commands.
// ═══════════════════════════════════════════════════════════════════════════

use crate::sim::tick::{CampaignSim, House, JournalEntry, ORIGIN_NONE, SpecCenter, TickGood, TickHub};
use crate::commands::query_commands::EconomySnapshot;

/// Compact clock for the Campaign Clock UI.
#[derive(Serialize)]
pub struct CampaignClock {
    pub tick: u32,
    pub year: u32,
    pub day: u32,
    pub season: String,
    pub last_tick_ms: f32,
}

/// Per-hub summary for the campaign map/list (no per-good vectors).
#[derive(Serialize)]
pub struct HubBrief {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub population: u32,
    pub grain_wealth: f32,
    pub trade_wealth: f32,
    pub starving: f32,
    pub is_estate: bool,
    /// Overall population mood 0..1 (for an optional map tint).
    pub mood: f32,
    /// Month-over-month population growth fraction (+0.05 = +5%); 0 until two
    /// monthly history samples exist. Drives the "which cities grow/shrink" list.
    pub growth: f32,
    /// Colony state for map markers: 0 none · 1 settlement colony · 2 house outpost.
    pub colony_kind: u8,
    pub colony_stage: u8,
    /// Owner house index (house outposts) — frontend maps it to the house colour.
    pub owner_house: i32,
    /// Founder/owner-home hub INDEX (lane endpoint); −1 if none.
    pub founder_hub: i32,
    pub autonomous: bool,
    /// Atlas 2.0 · the settlement is a DEAD ruin († marker, skipped by the sim).
    pub abandoned: bool,
    /// Tick founded mid-campaign (0 = primordial) — drives the "new town" badge.
    pub founded_tick: u32,
    /// Last full year's trade throughput (grain-eq, in+out) — Trade Heat overlay.
    pub trade_volume: f32,
    /// Dynamically-earned commercial class (re-ranked twice a year): 0 ordinary ·
    /// 1 trade hub · 2 entrepôt. Drives the distinct map marker (blue diamond / red
    /// triangle). 0 until a campaign has run.
    #[serde(default)] pub hub_class: u8,
    /// Satellite CONSTRUCTION stage: 0 = finished/not a build site · 1..=5 = building.
    /// Drives the map's under-construction marker + opens the construction window.
    #[serde(default)] pub build_stage: u8,
    /// Why the settlement died ("famine"/"plague"/"war"/"disaster"; "" = alive).
    pub died_cause: String,
    /// Downsampled population history (≤30 points, oldest first) — the census
    /// sparkline. Empty until the hub has history samples.
    pub pop_spark: Vec<f32>,
    /// CITY_PROVINCE_WAR_PLAN.md §3.2 · 1 great · 2 major · 3 lesser · 4 marginal ·
    /// 0 = not yet assigned. §3.3 reads this to decide state eligibility (tier 1-2).
    #[serde(default)] pub tier: u8,
}

/// What `campaign_start_sim` / `campaign_advance` / `campaign_get_state` return.
#[derive(Serialize)]
pub struct CampaignSnapshot {
    pub active: bool,
    pub clock: CampaignClock,
    pub hubs: Vec<HubBrief>,
    /// Most recent journal events (newest last), capped.
    pub recent_events: Vec<JournalEntry>,
    /// Population-weighted world price index (1.0 = baseline).
    pub price_index: f32,
    pub in_transit: u32,
    /// Total population across all hubs (shown as a number in the world pulse).
    pub total_population: u32,
    /// Population change since the last monthly chronicle sample.
    pub population_delta: i32,
    /// World price-index change since the last monthly chronicle sample.
    pub price_index_delta: f32,
    /// Atlas 2.0 · recent refugee roads `[from_x, from_y, to_x, to_y, tick]` —
    /// the map draws them as fading migration arrows for ~4 years.
    pub migrations: Vec<[f32; 5]>,
}

/// M6 world-economy panel datum: one good's world price + who moves it.
#[derive(Serialize)]
pub struct WorldGoodPrice {
    pub good: usize,
    pub name: String,
    pub world_price: f32, // mean local price / base_value across hubs
    pub producers: u32,
    pub top_hub: String,
}

#[derive(Serialize)]
pub struct WorldEconomy {
    pub goods: Vec<WorldGoodPrice>,
    /// World price-index time series (tick, value) from the journal.
    pub index_series: Vec<[f32; 2]>,
    /// Current population-weighted fraction of demand unmet, by need tier.
    #[serde(default)] pub lack_basic: f32,
    #[serde(default)] pub lack_comfort: f32,
    #[serde(default)] pub lack_luxury: f32,
    /// Current world merchant population totals by class.
    #[serde(default)] pub pop_house: f32,
    #[serde(default)] pub pop_local: f32,
    #[serde(default)] pub pop_guild: f32,
    /// World time series `[tick, basic, comfort, luxury]` (population-weighted unmet).
    #[serde(default)] pub lack_series: Vec<[f32; 4]>,
    /// World time series `[tick, houses, local, guild]` merchant population totals.
    #[serde(default)] pub merchant_series: Vec<[f32; 4]>,
    /// Atlas 2.0 — yearly world samples `[year, population, trade volume, live
    /// hubs, cumulative foundings, cumulative abandonments]` for the Atlas graphs.
    #[serde(default)] pub world_series: Vec<[f32; 6]>,
    /// Batch 1 — the Hall of Records (all-time world records).
    #[serde(default)] pub records: crate::sim::tick::WorldRecords,
}

/// One good's live state at a hub, for the settlement-window Market tab.
#[derive(Serialize)]
pub struct HubGoodDetail {
    pub good: usize,
    pub name: String,
    pub price: f32,        // local price in the grain-eq numeraire
    pub base_value: f32,   // world-standard value (price/base = ×-world)
    pub stock: f32,
    pub need: f32,         // approx per-tick demand (desire × population)
    pub production: f32,
    /// Cheapest / dearest world price (×-world) and the hub names there.
    pub world_min: f32,
    pub world_min_hub: String,
    pub world_max: f32,
    pub world_max_hub: String,
    /// Mean ×-world price across all settlements right now (the "world average"
    /// reference line on the Market price graphs).
    #[serde(default)] pub world_avg: f32,
    /// DLC 4 · this hub's production quality 0..1 for the good + its grade label.
    #[serde(default)] pub quality: f32,
    #[serde(default)] pub grade: String,
}

/// One shipment touching a settlement (Market tab arrivals/departures), with its
/// owner so houses/guilds and round-trip return legs are identifiable.
#[derive(Serialize, Clone)]
pub struct ShipmentRow {
    pub owner: String,          // house / guild name, or "Local merchants"
    pub color: String,
    pub is_guild: bool,
    pub other: String,          // origin city (arrivals) or destination (departures)
    pub good: String,
    pub amount: f32,
    pub price: f32,             // ×-world price
    pub value: f32,             // amount × local price (the ranking key)
    pub sea: bool,
    pub returning_home: bool,   // a round-trip RETURN leg (goods bought abroad, coming home)
}

/// DLC 3 · a polis seat's government, for the settlement Government subtab: the
/// council house and the fiscal policy it sets, plus this seat's speculation read.
#[derive(Serialize, Clone, Default)]
pub struct Government {
    /// Governing (dominant, non-guild) house name, or "—" if none holds the seat.
    pub council: String,
    pub council_color: String,
    pub council_archetype: String,
    pub council_is_guild: bool,
    /// The council house's soft political power 0..1.
    pub council_power: f32,
    /// Effective export / import tariff fractions in force (council policy, or the
    /// global default until a council sets one).
    pub tariff_export: f32,
    pub tariff_import: f32,
    /// True when these are the global defaults (no council policy set yet).
    pub tariff_default: bool,
    /// Mint fineness (1.0 = full-bodied coin, < 1 = debased "cheap money").
    pub mint_fineness: f32,
    pub treasury: f32,
    pub civic_pool: f32,
    /// This seat's speculation read (empty tier = none / below the noise floor).
    pub spec_risk: f32,
    pub spec_tier: String,
    pub spec_stars: u8,
    pub spec_pattern: String,
    /// The ranked causal "why" clauses (largest driver first).
    pub spec_drivers: Vec<String>,
    pub spec_watch: Vec<String>,
    // ── Government layer (key figures, capture, laws, stores) ──
    /// Regime type label ("Merchant Council" / "Principality" / "Free Commune").
    pub govt_type: String,
    /// Years until the next seat turns over (regime change).
    pub next_election_years: i32,
    /// The house that CONTROLS this government (captured a majority of its figures), or "".
    pub captor: String,
    pub captor_color: String,
    /// The city's key figures (mayor/treasurer/harbormaster/magistrate).
    pub officials: Vec<OfficialRow>,
    /// Each family's influence over the government, as a normalised %.
    pub family_influence: Vec<InfluenceRow>,
    /// Recent enacted laws (newest first).
    pub laws: Vec<LawRow>,
    /// The goods the government itself holds (civic granary/stockpile).
    pub civic_goods: Vec<CivicGoodRow>,
    /// CITY_PROVINCE_WAR_PLAN.md §3.1 · the office as a PERSON — the head of
    /// whichever house actually runs this seat. `None` when no house holds either
    /// office (the ordinary early-campaign case). No new person entity: this is
    /// `kin[0]` of `captor` (the stronger office — a majority-captured government)
    /// or `council` (the softer, dominant-but-not-captured case), whichever is set.
    #[serde(default)]
    pub leader: Option<CityLeader>,
}

/// CITY_PROVINCE_WAR_PLAN.md §3.1 · the city leader — the head of the house that
/// runs this seat's government, reusing the existing house-person stack (kin,
/// character, vice) rather than a new entity.
#[derive(Serialize, Clone, Default)]
pub struct CityLeader {
    pub house: String,
    pub house_color: String,
    pub is_guild: bool,
    /// True when the house holds this seat by CAPTURE (a majority of its
    /// control-weighted offices) rather than merely being its dominant council.
    pub is_captor: bool,
    pub head_name: String,
    pub female: bool,
    /// A phrase from the head's character axes ("Bold, civic-minded." or "" for a
    /// middling character) — the same discipline the stability gauges use.
    pub character_phrase: String,
    /// "" when the head has none.
    pub vice: String,
}

/// One government key figure for the Government subtab.
#[derive(Serialize, Clone)]
pub struct OfficialRow {
    pub role: String,
    pub name: String,
    /// The house it serves ("" = neutral) + colour + a status word.
    pub allegiance: String,
    pub allegiance_color: String,
    pub control: f32,
    /// "neutral" | "leaning" | "controlled" | "kin".
    pub status: String,
}

/// One family's share of government influence.
#[derive(Serialize, Clone)]
pub struct InfluenceRow { pub name: String, pub color: String, pub pct: f32 }

/// A single building in a settlement, resolved with WHO owns/controls it so the
/// ward-grid panel can tint each tile by its owning faction. `owner_kind` is
/// "house" (a resident merchant family), "civic" (the city/polis itself) or
/// "fondaco" (a foreign diaspora quarter's own trade house). `color` is a hex
/// tint — the house's heraldic colour (matching `family_influence`), the civic
/// slate, or the diaspora people's hearth colour. Resolved every query, so the
/// grid recolours live as houses rise/fall and control shifts.
#[derive(Serialize, Clone)]
pub struct BuildingInfo {
    pub label: String,
    pub effect: String,
    pub emoji: String,
    pub owner: String,
    pub owner_kind: String,
    pub color: String,
}

/// Civic (city-owned) building tint — a neutral slate that reads as "the commons".
const CIVIC_TINT: &str = "#7a8aa0";

/// `[r,g,b]` → `#rrggbb`.
fn rgb_hex(c: [u8; 3]) -> String { format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]) }

/// Emoji for a building label (mirrors the frontend fallback so the payload is
/// self-describing).
fn building_emoji(label: &str) -> &'static str {
    match label {
        "Granary" => "🌾", "Warehouse" => "📦", "Shipyard" => "⚓",
        "Guildhall" => "🏛️", "Workshop" => "🔨", "Fondaco" => "🏯",
        _ => "🏗️",
    }
}

/// One enacted-law log row (rendered text).
#[derive(Serialize, Clone)]
pub struct LawRow { pub year: u32, pub text: String }

/// One good the government holds in its stores.
#[derive(Serialize, Clone)]
pub struct CivicGoodRow { pub name: String, pub amount: f32 }

/// City STORES — the settlement's own goods reserve (the civic warehouse) plus the total
/// goods physically held at the city across every pool, with grain-equivalent valuations.
/// Lets the council/panel see at a glance how well-PROVISIONED (supplies, food buffer) and
/// how RICH-IN-GOODS the city is — the basis for shortage-buffering and population planning.
#[derive(Serialize, Clone, Default)]
pub struct CityStores {
    /// Civic (city-owned) warehouse contents — goods the government stockpiles, top by amount.
    pub reserve: Vec<CivicGoodRow>,
    /// Grain-eq value of the civic reserve.
    pub reserve_value: f32,
    /// Food held in the city's own stores (civic granary + dedicated food reserve), in units.
    pub food_reserve: f32,
    /// ALL goods physically held at the city (local merchant pool + house depots), top by value.
    pub top_goods: Vec<CivicGoodRow>,
    /// Grain-eq value of ALL goods held at the city — the city's "riches in goods".
    pub goods_value: f32,
    /// Total units of all goods held at the city.
    pub goods_units: f32,
}

/// One coin in a city's currency basket (for the settlement-view pie).
#[derive(Serialize)]
pub struct CoinShare {
    pub coin_name: String,
    pub share: f32,    // 0..1 of the city's circulation
    pub main: bool,    // the city's main settling coin
    pub reserve: bool, // a foreign reserve coin circulating here
}

/// One sub-cap hinterland village that markets through this town (satellite trade).
#[derive(Serialize, Clone)]
pub struct HinterlandVillage {
    pub name: String,
    pub population: u32,
    pub x: f32,
    pub y: f32,
}

/// Cultures 2.0 · one resident people's contentment in a city — how well the goods it
/// prizes are supplied here (met vs unmet), so the panel can show happy/unhappy peoples.
#[derive(Serialize, Clone)]
pub struct CultureMood {
    pub name: String,
    pub share: f32,          // 0..1 of the city's population
    pub satisfaction: f32,   // 0..1 — mean availability of its prized goods
    pub color: [u8; 3],
    pub met: Vec<String>,    // prized goods well-supplied here
    pub unmet: Vec<String>,  // prized goods scarce/dear here
}

/// Full per-settlement detail for the redesigned settlement window (live campaign
/// state): sentiment, market, and history.
#[derive(Serialize)]
pub struct HubDetail {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub koppen: u8,
    pub coastal: bool,
    pub is_estate: bool,
    // sentiment
    pub mood: f32,
    pub sent_food: f32,
    pub sent_prosperity: f32,
    pub sent_stability: f32,
    // wealth / food
    pub grain_wealth: f32,
    pub trade_wealth: f32,
    pub food_balance: f32,
    pub starving: f32,
    pub goods: Vec<HubGoodDetail>,
    pub history: Vec<crate::sim::tick::HubSample>,
    pub events: Vec<JournalEntry>,
    /// Merchant families resident in this city (richest first).
    #[serde(default)]
    pub houses: Vec<HouseBrief>,
    /// Recent supply arriving by SEA (ships) vs LAND (caravans) — decaying tally.
    #[serde(default)] pub in_by_sea: f32,
    #[serde(default)] pub in_by_land: f32,
    /// Current fraction of demand UNMET, by need tier (0 = supplied, 1 = none met).
    #[serde(default)] pub lack_basic: f32,
    #[serde(default)] pub lack_comfort: f32,
    #[serde(default)] pub lack_luxury: f32,
    /// Estimated merchant population in this city by class.
    #[serde(default)] pub pop_house: f32,
    #[serde(default)] pub pop_local: f32,
    #[serde(default)] pub pop_guild: f32,
    /// Abstract social strata of this settlement (shares + inequality + welfare).
    #[serde(default)] pub society: Option<SocietyBrief>,
    /// For an estate: its kind (0 none / 1 farm / 2 mine / 3 plantation / 4 fishery /
    /// 5 vineyard), who owns it, and the good it works — for the inspector.
    #[serde(default)] pub estate_kind: u8,
    #[serde(default)] pub estate_owner: String,
    #[serde(default)] pub estate_good: String,
    /// Buildings erected here: (name, one-line effect) — legacy flat list (kept
    /// for compatibility; the panel now renders `buildings`).
    #[serde(default)] pub structures: Vec<(String, String)>,
    /// Ward-grid buildings, each resolved with its owning faction + tint colour so
    /// the settlement panel can show WHO controls what (houses / civic / diaspora
    /// fondaco). Recomputed every query → recolours live with the campaign.
    #[serde(default)] pub buildings: Vec<BuildingInfo>,
    /// Trade-base patron: the merchant house developing this city as a base of
    /// operations (empty = none).
    #[serde(default)] pub patron: String,
    /// #23 · the majority PEOPLE of this settlement + its minority quarters
    /// `(people, population share)` — grown by in-migration of a different culture
    /// and slowly eroded by assimilation. Display-only.
    #[serde(default)] pub culture: String,
    #[serde(default)] pub minorities: Vec<(String, f32)>,
    /// Cultures 2.0 · per-people SATISFACTION in this city — is each resident culture's
    /// prized goods well-supplied here? Drives the "happy / unhappy peoples" readout.
    #[serde(default)] pub culture_moods: Vec<CultureMood>,
    /// DLC 3 · the polis government of this seat (council + fiscal policy + this
    /// city's speculation read). None for estates / unsettled hubs.
    #[serde(default)] pub government: Option<Government>,
    /// FOREIGN merchant offices hosted in this settlement (houses/guilds based
    /// elsewhere who have opened a counting-house here).
    #[serde(default)] pub offices_here: Vec<OfficeHere>,
    /// Market flow: in-flight shipments arriving / departing (ranked by value).
    #[serde(default)] pub arrivals: Vec<ShipmentRow>,
    #[serde(default)] pub departures: Vec<ShipmentRow>,
    /// Recently COMPLETED deals (most recent first), by direction.
    #[serde(default)] pub recent_arrivals: Vec<ShipmentRow>,
    #[serde(default)] pub recent_departures: Vec<ShipmentRow>,
    /// Recent trade value bought (imports) vs sold (exports), decaying tallies.
    #[serde(default)] pub bought: f32,
    #[serde(default)] pub sold: f32,
    /// Estates & manufactories in this city's hinterland.
    #[serde(default)] pub estates_here: Vec<EstateRow>,
    // ── DLC 3.5 · treasury, finances, war, and the carrying trade ──
    /// Retained civic treasury (grain-eq).
    #[serde(default)] pub treasury: f32,
    /// The city's treasury books (current running year + last completed in `prev`).
    #[serde(default)] pub finance: Option<crate::sim::tick::CityFinance>,
    /// PUBLIC HEALTH level (hospices/quarantine), 0..0.6 — how far the council funds
    /// disease mitigation. Higher = far fewer die in a plague, longer immunity.
    #[serde(default)] pub public_health: f32,
    /// Sub-cap HINTERLAND villages that market through this town (satellite trade). These
    /// are the small settlements below the sim cap — connected to the economy via their
    /// nearest hub rather than simulated individually.
    #[serde(default)] pub satellites: Vec<HinterlandVillage>,
    /// Name of the polis this city is at war with ("" = at peace).
    #[serde(default)] pub war_with: String,
    /// Its coin, if it mints one.
    #[serde(default)] pub coin_name: String,
    #[serde(default)] pub coin_trust: f32,
    #[serde(default)] pub coin_value: f32,
    /// The city's CURRENCY BASKET — which coins circulate here and their share
    /// (main coin first). For the settlement-view currency-basket pie.
    #[serde(default)] pub coin_basket: Vec<CoinShare>,
    /// Carrying trade: in-flight shipments THIS city's merchants run between OTHER
    /// cities (the entrepôt "transit" — goods that pass through its houses' hands).
    #[serde(default)] pub transit: Vec<TransitRow>,
    /// DLC 4 · espionage: if this estate/manufactory stole a quality technique, the
    /// good's name + the city it was stolen from ("" = none).
    #[serde(default)] pub stolen_good: String,
    #[serde(default)] pub stolen_from: String,
    /// Colonies & outposts this city FOUNDED (its metropolis roster).
    #[serde(default)] pub related_colonies: Vec<ColonySummary>,
    /// City STORES — civic warehouse + total goods held at the city, with valuations.
    #[serde(default)] pub city_stores: CityStores,
    /// Settlement DEVELOPMENT tier (0..5): 1 Outpost · 2 Market · 3 Guild Town ·
    /// 4 Free City · 5 Emporium. Advancement (institutions), not raw size.
    #[serde(default)] pub dev_tier: u8,
}

/// Abstract social strata of a settlement for the HubPanel "Society" block:
/// the four population shares (Σ=1), an inequality index, and a derived welfare.
#[derive(Serialize, Clone)]
pub struct SocietyBrief {
    pub patrician: f32,
    pub burgher: f32,
    pub commoner: f32,
    pub underclass: f32,
    pub commoner_wealth: f32,
    pub inequality: f32,
    /// 0 = destitute, 1 = comfortable — commoner welfare for the meter (derived).
    pub welfare: f32,
    /// 0 = content … 1 = boiling — civil unrest (It. 3).
    #[serde(default)] pub unrest: f32,
}

/// DLC 3.5 · one leg of a city's carrying trade (a merchant of this city hauling
/// goods between two OTHER cities), for the settlement "Transit" section.
#[derive(Serialize, Clone)]
pub struct TransitRow {
    pub merchant: String,
    pub is_guild: bool,
    pub color: String,
    pub good: String,
    pub amount: f32,
    pub value: f32,
    pub from_name: String,
    pub to_name: String,
    pub sea: bool,
    /// Coin the deal settles in ("" → barter).
    pub coin: String,
    /// Barter ratio when no reserve coin applies (e.g. "~3.2 wheat/unit").
    pub barter: String,
}

/// One estate / manufactory in a settlement's hinterland (host-side view).
#[derive(Serialize)]
pub struct EstateRow {
    pub name: String,
    pub kind: u8,            // 1 farm/2 mine/3 plantation/4 fishery/5 vineyard/6 manufactory
    pub good: String,
    pub output: f32,         // current production/day of its good
    pub owner: String,       // owning house/guild, or "City of …"
    pub owner_is_guild: bool,
    pub owner_is_civic: bool, // city-financed (locally owned), no private house/guild
    pub tier: u8,            // upgrade tier 1..5
    pub damage: f32,         // disaster damage 0 (intact) .. 1 (ruined); suppresses output
}

/// One foreign merchant's office hosted in a settlement (host-side view).
#[derive(Serialize)]
pub struct OfficeHere {
    pub holder: String,        // house / guild name
    pub color: String,         // its stable map colour
    pub is_guild: bool,
    pub origin: String,        // the city the holder is based in
    pub throughput_pct: f32,   // % of THIS settlement's live trade it handles
    pub goods: Vec<String>,    // goods it currently moves through here
}

/// Parse the campaign sim from the DB into the resident cache ONCE. After this the
/// query commands read the live object instead of re-parsing JSON every call.
fn ensure_campaign_loaded(cache: &mut crate::db::CampaignCache, conn: &Connection) -> Result<(), String> {
    if cache.loaded {
        return Ok(());
    }
    // Make the organic culture map active so houses/guilds founded this campaign are
    // named in their home city's culture (no-op when none is stored).
    crate::sim::cultures::ensure_active(conn);
    let raw = metadata::campaign_get(conn, "campaign_sim").map_err(|e| e.to_string())?;
    cache.sim = match raw {
        Some(s) if !s.is_empty() => {
            let mut sim = decode_campaign_blob(&s)?;
            sim.rebuild_routes(); // `days` / `neighbors` are not serialized
            Some(std::sync::Arc::new(sim))
        }
        _ => None,
    };
    cache.loaded = true;
    cache.dirty = false;
    Ok(())
}

/// Batch 1 · campaign blob codec: `"Z1:" + base64(zstd(json))` — ~5-10× smaller
/// DB rows and `.campaign` files. JSON stays the wire format underneath
/// (self-describing, so the append-`serde(default)` save-compat strategy is
/// untouched); legacy plain-JSON rows load via the prefix check.
fn compress_blob(json: &str) -> Result<String, String> {
    use base64::Engine as _;
    let z = zstd::encode_all(json.as_bytes(), 3).map_err(|e| e.to_string())?;
    Ok(format!("Z1:{}", base64::engine::general_purpose::STANDARD.encode(z)))
}

/// Returns the JSON inside `raw` — decompressing a `Z1:` blob, passing a legacy
/// plain-JSON row through untouched.
fn decompress_blob(raw: &str) -> Result<std::borrow::Cow<'_, str>, String> {
    use base64::Engine as _;
    if let Some(b64) = raw.strip_prefix("Z1:") {
        let z = base64::engine::general_purpose::STANDARD.decode(b64)
            .map_err(|e| format!("campaign_sim base64: {e}"))?;
        let bytes = zstd::decode_all(&z[..]).map_err(|e| format!("campaign_sim zstd: {e}"))?;
        Ok(std::borrow::Cow::Owned(
            String::from_utf8(bytes).map_err(|e| format!("campaign_sim utf8: {e}"))?))
    } else {
        Ok(std::borrow::Cow::Borrowed(raw))
    }
}

fn encode_campaign_blob(sim: &CampaignSim) -> Result<String, String> {
    let json = serde_json::to_string(sim).map_err(|e| e.to_string())?;
    compress_blob(&json)
}

fn decode_campaign_blob(raw: &str) -> Result<CampaignSim, String> {
    let json = decompress_blob(raw)?;
    serde_json::from_str(&json).map_err(|e| format!("campaign_sim parse: {e}"))
}

#[cfg(test)]
mod blob_tests {
    use super::{compress_blob, decompress_blob};

    /// Batch 1 · the save-blob codec round-trips and shrinks; legacy plain-JSON
    /// rows pass through untouched.
    #[test]
    fn campaign_blob_codec_roundtrip_and_legacy() {
        // A repetitive JSON-ish payload (like real sim state) compresses well.
        let json = format!("{{\"hubs\":[{}]}}",
            (0..500).map(|i| format!("{{\"id\":{i},\"stock\":[0.0,1.5,2.25]}}"))
                .collect::<Vec<_>>().join(","));
        let blob = compress_blob(&json).expect("compress");
        assert!(blob.starts_with("Z1:"));
        assert!(blob.len() < json.len() / 3, "blob {} vs json {}", blob.len(), json.len());
        assert_eq!(decompress_blob(&blob).expect("decompress").as_ref(), json);
        // Legacy row (no prefix) passes through untouched.
        assert_eq!(decompress_blob(&json).expect("legacy").as_ref(), json);
    }
}

/// A HANDLE to the resident sim (loading it from the DB once if needed) — an Arc
/// pointer bump, NOT a deep copy. The old version returned `CampaignSim.clone()`,
/// which copied every hub's per-good vectors, the per-hub histories and the
/// 20k-entry journal on EVERY read-only panel query — megabytes per HUD refresh.
/// The campaign lock is released before the caller computes, so heavy queries
/// never stall the Play loop.
pub(crate) fn get_sim(db: &WorldDb, conn: &Connection)
    -> Result<Option<std::sync::Arc<CampaignSim>>, String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    ensure_campaign_loaded(&mut cache, conn)?;
    Ok(cache.sim.clone())
}

/// Replace the resident sim and mark it dirty (the DB row is flushed later by
/// `persist_campaign` / the advance cadence). Used by commands that build or rewrite
/// the whole sim.
fn set_sim(db: &WorldDb, sim: &CampaignSim) -> Result<(), String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    cache.sim = Some(std::sync::Arc::new(sim.clone()));
    cache.loaded = true;
    cache.dirty = true;
    Ok(())
}

/// Flush the resident sim to the DB metadata row if it has unsaved changes. Called on
/// pause/save/close and periodically from `campaign_advance`. Caller holds `conn`.
fn persist_campaign(db: &WorldDb, conn: &Connection) -> Result<(), String> {
    let mut cache = db.campaign.lock().map_err(|e| e.to_string())?;
    if cache.dirty {
        if let Some(sim) = &cache.sim {
            let blob = encode_campaign_blob(sim.as_ref())?;
            metadata::campaign_set(conn, "campaign_sim", &blob).map_err(|e| e.to_string())?;
        }
        cache.dirty = false;
        cache.last_persist = Some(std::time::Instant::now());
    }
    Ok(())
}

fn build_snapshot(sim: &CampaignSim) -> CampaignSnapshot {
    let hubs = sim
        .hubs
        .iter()
        .map(|h| {
            // Month-over-month growth from the last two per-hub history samples.
            let n = h.history.len();
            let growth = if n >= 2 && h.history[n - 2].population > 0.0 {
                (h.history[n - 1].population - h.history[n - 2].population)
                    / h.history[n - 2].population
            } else { 0.0 };
            HubBrief {
                id: h.id,
                x: h.x,
                y: h.y,
                name: h.name.clone(),
                population: h.population.max(0.0) as u32,
                grain_wealth: h.grain_wealth,
                trade_wealth: h.trade_wealth,
                starving: h.starving,
                is_estate: h.is_estate,
                mood: h.mood,
                growth,
                colony_kind: h.colony_kind,
                colony_stage: h.colony_stage,
                owner_house: h.owner_house,
                founder_hub: h.founder_hub,
                autonomous: h.autonomous,
                abandoned: h.abandoned,
                founded_tick: h.founded_tick,
                trade_volume: h.trade_last_year,
                hub_class: h.hub_class,
                build_stage: h.build_stage,
                died_cause: h.died_cause.clone(),
                pop_spark: {
                    // ≤30 evenly-spaced population samples for the census sparkline.
                    let step = (h.history.len() / 30).max(1);
                    h.history.iter().step_by(step).map(|s| s.population).collect()
                },
                tier: h.tier,
            }
        })
        .collect();
    // Census = live simulated hubs + the inert hinterland towns below the sim cap
    // (decouple: counted even though they aren't ticked, so Atlas isn't undercounting).
    let total_population: f32 = sim.hubs.iter().map(|h| h.population.max(0.0)).sum::<f32>()
        + sim.hinterland.iter().map(|t| t.population.max(0.0)).sum::<f32>();
    let population_delta = if sim.last_month_pop > 0.0 {
        (total_population - sim.last_month_pop) as i32
    } else { 0 };
    let recent_events: Vec<JournalEntry> = sim
        .journal
        .iter()
        .filter(|e| e.kind != "price")
        .rev()
        .take(40)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let price_index = sim
        .journal
        .iter()
        .rev()
        .find(|e| e.kind == "price")
        .map(|e| e.value)
        .unwrap_or(1.0);
    let price_index_delta = if sim.last_month_index > 0.0 {
        price_index - sim.last_month_index
    } else { 0.0 };
    CampaignSnapshot {
        active: true,
        clock: CampaignClock {
            tick: sim.tick,
            year: sim.year(),
            day: sim.day_of_year(),
            season: sim.season().to_string(),
            last_tick_ms: sim.last_tick_ms,
        },
        hubs,
        recent_events,
        price_index,
        in_transit: sim.in_transit.len() as u32,
        total_population: total_population.max(0.0) as u32,
        population_delta,
        price_index_delta,
        // Refugee roads still fresh enough to draw (fade over ~4 years).
        migrations: sim.migrations.iter()
            .filter(|m| sim.tick as f32 - m[4] < 4.0 * 365.0)
            .cloned()
            .collect(),
    }
}

fn inactive_snapshot() -> CampaignSnapshot {
    CampaignSnapshot {
        active: false,
        clock: CampaignClock { tick: 0, year: 0, day: 0, season: "Spring".into(), last_tick_ms: 0.0 },
        hubs: vec![],
        recent_events: vec![],
        price_index: 1.0,
        in_transit: 0,
        total_population: 0,
        population_delta: 0,
        price_index_delta: 0.0,
        migrations: vec![],
    }
}

/// Atlas 2.0 · one NAMED TRADE BASIN — a cluster of living market towns whose
/// strongest trade ties bind them together: "the regions where trade happens".
#[derive(Serialize, Clone)]
pub struct TradeBasin {
    /// Culture-styled region name from the basin's heart (e.g. "Vexillia").
    pub name: String,
    /// Total yearly flow volume INTERNAL to the basin (grain-eq).
    pub volume: f32,
    pub hub_ids: Vec<u32>,
    /// Member positions (world cells) — the frontend hulls + labels them.
    pub pts: Vec<[f32; 2]>,
    pub cx: f32,
    pub cy: f32,
    /// Busiest member (highest yearly throughput).
    pub top_city: String,
    /// Batch 1 · the basin's top traded goods (≤2, by yearly volume).
    pub top_goods: Vec<String>,
}

/// Cluster the yearly flow ledger into named trade basins: each town keeps only
/// its TOP-2 flow edges, and the connected components of that sparse
/// "strongest-partner" graph are the basins — weak long-range ties don't merge
/// the whole world into one blob. Volume counts ALL flow internal to a basin.
/// Basin memo: the clustering is deterministic in the sim state and can only
/// change when time advances, so repeated panel opens / overlay refreshes at the
/// same (seed, tick) are answered from this cache.
static BASINS_MEMO: std::sync::Mutex<Option<(u64, u32, Vec<TradeBasin>)>> =
    std::sync::Mutex::new(None);

/// Batch 1 · era scrubber: the world as it stood at the end of `year` — marker +
/// heat data reconstructed from the yearly frame ring. None if that year isn't
/// in the (bounded) ring.
#[derive(Serialize)]
pub struct EraFrame {
    pub year: u32,
    pub hubs: Vec<EraHub>,
}
#[derive(Serialize)]
pub struct EraHub {
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub population: f32,
    pub trade: f32,
    pub dead: bool,
    pub is_new: bool,
}

// ── #1/#23 · Peoples (cultures) ─────────────────────────────────────────────────
#[derive(Serialize)]
pub struct CultureBrief {
    pub name: String,
    pub color: [u8; 3],
    pub population: u32,
    pub towns: u32,      // settlements where this culture is the MAJORITY
    pub presence: u32,   // settlements where it is present (≥ 5%)
    pub mobility: f32,   // 0..1 travel-proneness (≥0.7 = merchant diaspora)
    pub top_cities: Vec<(String, u32)>, // by this culture's population, top 4
    pub houses: Vec<String>,            // merchant houses of this people
    /// Language family (Italic, Semitic, Germanic, …); "" for a creole with mixed roots.
    #[serde(default)] pub family: String,
    /// Static origin card — where this people arose + temperament (Cultures 2.0).
    #[serde(default)] pub origin: String,
    /// Costume/appearance kit index (0..11) for the figure art; -1 unknown. For a creole,
    /// `kit` and `kit2` are its two parent kits so the figures blend both dresses.
    #[serde(default = "neg_one_i32c")] pub kit: i32,
    #[serde(default = "neg_one_i32c")] pub kit2: i32,
    /// Goods this people PRIZES (cultural taste) — good-spec ids, for the panel.
    #[serde(default)] pub desired_goods: Vec<String>,
    /// Character traits (name/emoji/blurb), 2–3 per people (Cultures 3.0).
    #[serde(default)] pub traits: Vec<CultureTraitBrief>,
    /// Inter-culture relations: kin/friendly, rivals, hostile neighbours.
    #[serde(default)] pub relations: Vec<CultureRelation>,
    /// Total wealth attributed to this people (city grain+trade wealth by share +
    /// its merchant houses) — powers the "richest cultures" ranking.
    #[serde(default)] pub wealth: f64,
    /// False for an EXTINCT people (no living settlement holds it) — the panel files
    /// these under a separate "Vanished peoples" section.
    #[serde(default = "true_bool")] pub alive: bool,
    /// Obituary shown for an extinct people (how/where they faded).
    #[serde(default)] pub obituary: String,
    /// Cultures 2.0 · number of trade regions where THIS people's tongue is the
    /// lingua franca (the regional trade language). >0 → a badge on the panel; can be
    /// nonzero even for a vanished people (a legacy tongue that outlived them).
    #[serde(default)] pub lingua_regions: u32,
}
fn neg_one_i32c() -> i32 { -1 }
fn true_bool() -> bool { true }

#[derive(serde::Serialize, Clone)]
pub struct CultureTraitBrief { pub name: String, pub emoji: String, pub blurb: String }

#[derive(serde::Serialize, Clone)]
pub struct CultureRelation {
    pub name: String,
    /// "kin" (same family), "friendly", "rival" (economic), or "hostile".
    pub kind: String,
}

/// A culture's (family, origin card, kit, kit2) from the active worldgen hearths (or a
/// spawned creole). "" / -1 when unknown (e.g. a legacy save without cards).
fn culture_lore(sim: &crate::sim::tick::CampaignSim, name: &str) -> (String, String, i32, i32) {
    if let Some(cr) = sim.creoles.iter().find(|c| c.name == name) {
        return (cr.family.clone(), cr.origin.clone(), cr.kit_a as i32, cr.kit_b as i32);
    }
    if let Some(m) = crate::sim::cultures::active() {
        if let Some(h) = m.hearths.iter().find(|h| h.people == name) {
            return (h.family.clone(), h.origin.clone(), h.kit as i32, -1);
        }
    }
    (String::new(), String::new(), -1, -1)
}

/// Deterministic per-culture seed (a hearth's mut_seed, else a hash of the name) —
/// so trait assignment is stable across queries.
fn culture_seed(name: &str) -> u64 {
    if let Some(m) = crate::sim::cultures::active() {
        if let Some(h) = m.hearths.iter().find(|h| h.people == name) { return h.mut_seed; }
    }
    let mut x = 0xcbf29ce484222325u64;
    for b in name.bytes() { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); }
    x
}

/// Trait indices for a culture (into `cultures::TRAITS`), from its kit + mobility +
/// seed. Empty when the kit is unknown (legacy save).
fn culture_trait_ids(sim: &crate::sim::tick::CampaignSim, name: &str) -> Vec<usize> {
    let (_, _, kit, _) = culture_lore(sim, name);
    if kit < 0 { return Vec::new(); }
    let mob = crate::sim::tick::CampaignSim::culture_mobility(name);
    crate::sim::cultures::kit_traits(kit as usize, mob, culture_seed(name))
}

/// Culture colour: the worldgen hearth colour if known, else a deterministic tint.
fn culture_color(name: &str) -> [u8; 3] {
    if let Some(m) = crate::sim::cultures::active() {
        if let Some(h) = m.hearths.iter().find(|h| h.people == name) {
            return h.color;
        }
    }
    let mut x = 0xcbf29ce484222325u64;
    for b in name.bytes() { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); }
    [(80 + (x % 150)) as u8, (80 + ((x >> 8) % 150)) as u8, (80 + ((x >> 16) % 150)) as u8]
}

/// A coarse "where this people lives" raster for the Peoples-panel mini-map — the same
/// shape as the Goods Editor's suitability preview (`data` 0..255 + a `land` mask), so
/// the UI draws it identically. Homeland territory (from the worldgen culture raster)
/// is brightest; live cities where the people is present are lit too (majority brighter
/// than a minority quarter), so migration/diaspora shows.
#[derive(serde::Serialize)]
pub struct CulturePresenceGrid {
    pub width: u32, pub height: u32, pub data: Vec<u8>, pub land: Vec<u8>,
    /// Per coarse cell: 1 = this people is DOMINANT here (its homeland, or the local
    /// majority of a city); 0 = merely present as a minority quarter (or other land).
    /// The panel draws dominant areas at full colour and non-dominant at ~30%.
    #[serde(default)] pub dominant: Vec<u8>,
}

#[derive(serde::Serialize)]
pub struct NotableCity { pub name: String, pub x: f32, pub y: f32, pub role: String }

/// One notable figure of a people — a merchant magnate / dynastic head, grounded in
/// the live house registry (real names, cities, wealth and deeds).
#[derive(serde::Serialize)]
pub struct NotablePerson {
    pub name: String,       // the head of family (individual)
    pub house: String,      // their house
    pub era: String,        // when they lived / flourished
    pub known_for: String,  // what they are remembered for
    pub city: String,       // their seat city
    pub wealth: f64,
    pub alive: bool,        // their house still endures
    pub cities: Vec<NotableCity>, // seat + the cities they reached (offices) — clickable
}

fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let ra = uf_find(&mut *parent, a);
    let rb = uf_find(&mut *parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

/// Autosave cadence: the campaign is flushed to disk every this many SIM-years
/// (crossing an even-year boundary), with a wall-clock safety flush so an unattended
/// fast-play still checkpoints between boundaries.
const AUTOSAVE_EVERY_YEARS: u32 = 2;
const AUTOSAVE_WALLCLOCK_SECS: f32 = 120.0;

/// Below this many remaining sites, `campaign_advance` recomputes the colonization
/// pool from live tiles. The tick sim has no WorldBuffer, so it can never refill the
/// pool itself; gating on a low floor keeps the (cheap) recompute rare.
const COLONIZE_POOL_FLOOR: usize = 8;

/// Recompute the empty-land colonization pool from current tile data, excluding land
/// near any EXISTING hub (cities + already-founded colonies/outposts, so consumed
/// sites are never re-added). Mirrors the Economy step's `compute_colonizable_sites`
/// inputs. This is what keeps colonization alive across a long campaign and unsticks
/// old saves whose one-time pool was empty or drained.
fn recompute_colonizable(
    db: &WorldDb,
    conn: &Connection,
    hub_xy: &[(f32, f32)],
) -> Result<Vec<crate::sim::tick::ColonizeSite>, String> {
    let grid_w: u32 = metadata::get_meta(conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let specs = crate::commands::goods_commands::load_world_goods(conn);
    let base_value: Vec<f32> = specs.iter().map(|s| s.base_value.max(0.0)).collect();
    let world = db.cached_tiles_with_conn(conn)?;
    Ok(crate::commands::query_commands::compute_colonizable_sites(
        &world, grid_w, grid_h, hub_xy, &base_value))
}

/// Recompute the NEAR-city satellite site pool from current tiles (≤500 km from a
/// city). Mirrors `recompute_colonizable` but calls the near-city variant. Used to
/// (re)fill `CampaignSim::satellite_sites` at build and when it drains.
fn recompute_satellite_sites(
    db: &WorldDb,
    conn: &Connection,
    hub_xy: &[(f32, f32)],
) -> Result<Vec<crate::sim::tick::ColonizeSite>, String> {
    let grid_w: u32 = metadata::get_meta(conn, "grid_width")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    let grid_h: u32 = metadata::get_meta(conn, "grid_height")
        .map_err(|e| e.to_string())?.and_then(|s| s.parse().ok()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 { return Ok(vec![]); }
    let specs = crate::commands::goods_commands::load_world_goods(conn);
    let base_value: Vec<f32> = specs.iter().map(|s| s.base_value.max(0.0)).collect();
    let world = db.cached_tiles_with_conn(conn)?;
    Ok(crate::commands::query_commands::compute_satellite_sites(
        &world, grid_w, grid_h, hub_xy, &base_value))
}

/// Advance the sim resiliently — the campaign must ALWAYS move forward and NEVER crash
/// out. The batch runs in year-sized chunks under `catch_unwind`. On a tick fault:
///   1. restore the pre-chunk checkpoint (a cheap in-memory clone — NaN-safe, unlike a
///      JSON round-trip) and RETRY the chunk with territorial expansion frozen (the
///      year-30 founding paths are the usual suspects);
///   2. if it STILL faults, step the chunk tick-by-tick, skipping ONLY the single
///      poisoned tick (the clock still advances) and simulating every good tick.
/// State is preserved throughout (never discarded), so any batch — even one that hits a
/// deterministic bug — completes and the run stays continuable.
fn advance_resilient(sim: &mut crate::sim::tick::CampaignSim, ticks: u32) {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use crate::sim::tick::TICKS_PER_YEAR;
    if ticks == 0 { return; }
    let chunk_len = TICKS_PER_YEAR.max(1);
    let mut checkpoint = sim.clone();
    let mut done = 0u32;
    while done < ticks {
        let chunk = (ticks - done).min(chunk_len);
        // Fast path — the chunk simulates cleanly.
        if catch_unwind(AssertUnwindSafe(|| sim.advance(chunk))).is_ok() {
            checkpoint = sim.clone();
            done += chunk;
            continue;
        }
        // Recovery step 1 — restore, freeze expansion, retry the whole chunk once.
        *sim = checkpoint.clone();
        sim.expansion_frozen_until = sim.tick + chunk_len + chunk;
        if catch_unwind(AssertUnwindSafe(|| sim.advance(chunk))).is_ok() {
            checkpoint = sim.clone();
            done += chunk;
            continue;
        }
        // Recovery step 2 — restore and step tick-by-tick, skipping only bad ticks so
        // the run can never stall on a persistent, deterministic fault.
        *sim = checkpoint.clone();
        let mut tick_cp = checkpoint.clone();
        for _ in 0..chunk {
            match catch_unwind(AssertUnwindSafe(|| sim.advance(1))) {
                Ok(()) => tick_cp = sim.clone(),
                Err(payload) => {
                    // Log the fault so a recurring deterministic bug is diagnosable, then
                    // roll back to the pre-tick checkpoint and skip just this tick.
                    eprintln!("[campaign] tick fault skipped: {}", panic_payload_message(&payload));
                    *sim = tick_cp.clone();
                    sim.skip_poisoned_tick();
                    tick_cp = sim.clone();
                }
            }
        }
        checkpoint = sim.clone();
        done += chunk;
    }
}

/// DLC 4 · one typed population unit for the (future) Population panel.
#[derive(Serialize)]
pub struct PopBrief {
    pub profession: String,
    pub size: f32,
    pub money: f32,
    pub needs_life: f32,
    pub needs_everyday: f32,
    pub needs_luxury: f32,
    pub consciousness: f32,
    pub militancy: f32,
}

/// One backer of a colony venture (city / house / bank), for the Supply subtab.
#[derive(Serialize)]
pub struct ColonyBackerRow { pub kind: u8, pub name: String, pub color: String, pub share: f32 }

/// One civic supply contract row feeding a colony (the supplier roster).
#[derive(Serialize)]
pub struct ColonySupplyRow { pub category: u8, pub supplier: String, pub good: String, pub qty: f32 }

/// Full colony detail for the HubPanel "Supply" subtab.
#[derive(Serialize)]
pub struct ColonyDetail {
    pub stage: u8,
    pub autonomous: bool,
    pub founder_name: String,
    pub main_bank_name: String,
    pub coin_name: String,
    pub charter_open: bool,
    pub supply_years: f32,
    pub reserve_food: f32,
    pub reserve_cap: f32,
    pub age_years: u32,
    /// Years until the colony may seek independence (≤0 = eligible now).
    pub indep_in_years: i32,
    pub backers: Vec<ColonyBackerRow>,
    pub supply: Vec<ColonySupplyRow>,
    /// Dedicated grain-run supply ships the metropolis keeps on this colony.
    pub supply_ships: u32,
    /// Total monthly carriage of that fleet (ships × ship capacity).
    pub supply_capacity: f32,
    /// Food actually delivered last month by the fleet.
    pub supply_delivered: f32,
    /// The city currently designated as the colony's food source (empty = none found).
    pub supply_source: String,
}

/// One construction-supply tab for the satellite window (food / preservables / construction).
#[derive(Serialize)]
pub struct SatSupplyRow {
    pub category: String,
    pub good: String,
    pub source: String,
    pub rate: f32,
    pub met: f32,
}

/// Live state of a satellite still UNDER CONSTRUCTION (build_stage>0) — everything the
/// construction window needs. `None` once finished (it becomes a normal bound city).
#[derive(Serialize)]
pub struct SatelliteBrief {
    pub id: u32,
    pub name: String,
    pub metropolis: String,
    pub metropolis_id: i32,
    pub role: String,
    pub stage: u8,
    pub progress: f32,   // within the current stage
    pub overall: f32,    // 0..1 across all 5 stages
    pub eta_years: f32,
    pub monthly_cost: f32,
    pub fund: f32,
    pub runway_months: f32,
    pub convoys: u8,
    pub idle_months: u8,
    pub founded_year: f32,
    pub supply: Vec<SatSupplyRow>,
    pub exploits: Vec<String>,
}

/// One secured good in a council's civic warehouse (Provisioning tab).
#[derive(Serialize)]
pub struct ProvGoodRow {
    pub good: String,
    pub secured: f32,
    pub target: f32,
    pub food: bool,
}

/// Council RIGHT-OF-FIRST-BUY / provisioning state for a city (Provisioning tab).
#[derive(Serialize)]
pub struct ProvisioningBrief {
    pub first_buy: bool,
    pub dominant_house: String,
    /// Fraction of the city's trade carried by merchant houses (0..1).
    pub dominant_share: f32,
    pub dependents: u32,
    pub reserve_target: f32,
    /// Grain-eq secured into the civic warehouse last month.
    pub bought_month: f32,
    pub goods: Vec<ProvGoodRow>,
}

/// A route-bound migration flow for the reworked Migration overlay (polyline along the
/// trade network + culture + volume + age for fade).
#[derive(Serialize)]
pub struct MigrationRouteBrief {
    pub path: Vec<[f32; 2]>,
    pub culture: String,
    pub volume: f32,
    pub from_hub: i32,
    pub to_hub: i32,
    pub age_years: f32,
}

/// One roster row for the Colonial Office subwindow — a light projection over a
/// colony hub, covering BOTH settlement colonies (`colony_kind==1`) and house
/// trade outposts (`colony_kind==2`). The heavy per-colony detail (joint-stock
/// backers + supply contracts) stays in `ColonyDetail`/`campaign_get_colony`.
#[derive(Serialize)]
pub struct ColonySummary {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub colony_kind: u8,
    pub colony_stage: u8,
    pub autonomous: bool,
    pub population: f32,
    pub founder_hub: i32,
    pub founder_name: String,
    /// The metropolis's map coordinates (−1 if none) — for the click-highlight line.
    #[serde(default)] pub founder_x: f32,
    #[serde(default)] pub founder_y: f32,
    pub main_bank_name: String,
    pub coin_name: String,
    pub charter_open: bool,
    pub reserve_food: f32,
    pub reserve_cap: f32,
    pub supply_years: f32,
    pub age_years: u32,
    pub indep_in_years: i32,
    /// Owning house (house outposts, kind 2) — empty for settlement colonies.
    pub owner_house_name: String,
    pub owner_color: String,
    /// Dedicated grain-run supply ships + what they delivered last month.
    pub supply_ships: u32,
    pub supply_delivered: f32,
}

/// Assemble a `ColonySummary` for colony hub `hi` (caller guarantees it IS a colony).
fn colony_summary(sim: &CampaignSim, hi: usize) -> ColonySummary {
    let hub = &sim.hubs[hi];
    let founder_hub_ref = (hub.founder_hub >= 0)
        .then(|| sim.hubs.get(hub.founder_hub as usize)).flatten();
    let founder_name = founder_hub_ref.map(|h| h.name.clone()).unwrap_or_default();
    let (founder_x, founder_y) = founder_hub_ref.map(|h| (h.x, h.y)).unwrap_or((-1.0, -1.0));
    let main_bank_name = (hub.main_bank >= 0)
        .then(|| sim.banks.get(hub.main_bank as usize))
        .flatten().map(|b| b.name.clone()).unwrap_or_default();
    // Age / independence only meaningful for a settlement colony (outposts don't
    // record a founding tick and never become independent).
    let (age_years, indep_in_years) = match hub.colony_kind {
        1 => { let a = (sim.tick.saturating_sub(hub.colony_founded_tick) / 365) as u32; (a, 70i32 - a as i32) }
        3 => { let a = (sim.tick.saturating_sub(hub.colony_founded_tick) / 365) as u32; (a, 40i32 - a as i32) }
        _ => (0, 0),
    };
    // House outpost owner (kind 2): the owning house, coloured like the Houses panel.
    let (owner_house_name, owner_color) = if hub.colony_kind == 2 && hub.owner_house >= 0 {
        let oh = hub.owner_house as usize;
        (sim.houses.get(oh).map(|h| h.name.clone()).unwrap_or_default(), distinct_color(oh))
    } else {
        (String::new(), String::new())
    };
    ColonySummary {
        id: hub.id, name: hub.name.clone(), x: hub.x, y: hub.y,
        colony_kind: hub.colony_kind, colony_stage: hub.colony_stage, autonomous: hub.autonomous,
        population: hub.population,
        founder_hub: hub.founder_hub, founder_name, founder_x, founder_y, main_bank_name,
        coin_name: hub.coin_name.clone(),
        charter_open: hub.colony_kind == 1 && !hub.autonomous,
        reserve_food: hub.reserve_food, reserve_cap: hub.reserve_cap, supply_years: hub.supply_years,
        age_years, indep_in_years,
        owner_house_name, owner_color,
        supply_ships: hub.supply_ships, supply_delivered: hub.supply_delivered,
    }
}

// Colony-founding gate thresholds — MIRRORED (read-only) from `sim/tick.rs`. The
// Colonial Office REPORTS these to explain why no colony exists yet; it does NOT
// change them. Keep in sync with the consts in tick.rs (COLONY_START_TICK = 30y,
// COLONY_PARENT_MIN_POP, COLONY_MIN_FERTILE/COLONY_MIN_TRADE, COLONY_HOP_REACH_FRAC,
// MAX_SETTLEMENT_COLONIES, and the founder filter in maybe_found_settlement_colony).
const GATE_START_YEAR: u32 = 30;
const GATE_MIN_POP: f32 = 5_000.0;
const GATE_PROSPERITY_MIN: f32 = 0.25;
const GATE_STARVING_MAX: f32 = 0.7;
// A site counts as colonisable when it can part-feed itself OR is rich in trade
// goods (matches maybe_found_settlement_colony's relaxed viability floor).
// Mirror of COLONY_MIN_FERTILE / COLONY_MIN_TRADE / COLONY_HOP_REACH_FRAC in
// sim/tick.rs — keep these in lockstep or the gate will lie about "0 in range".
const GATE_FERTILE_SITE: f32 = 0.12;
const GATE_TRADE_SITE: f32 = 0.18;
const GATE_HOP_REACH_FRAC: f32 = 0.42;
const GATE_MAX_SETTLEMENT_COLONIES: u32 = 24;

/// Read-only snapshot of the settlement-colony founding gates, for the Colonial
/// Office "why no colonies yet?" empty state. Mirrors the conditions in
/// `maybe_found_settlement_colony` WITHOUT changing them.
#[derive(Serialize)]
pub struct ColonyGateStatus {
    pub year: u32,
    pub start_year: u32,
    pub year_ok: bool,
    pub qualifying_founder: String,
    pub founder_ok: bool,
    pub bank_on_continent: bool,
    pub colonizable_sites_in_range: u32,
    pub site_ok: bool,
    pub settlement_colonies: u32,
    pub max_settlement_colonies: u32,
    pub at_colony_cap: bool,
    pub min_pop: f32,
    /// First failing gate: "cap" | "year" | "founder" | "bank" | "site" | "none".
    pub blocking_gate: String,
}

/// DLC 4 · one good's world-wide quality + trade picture, for the floating Goods
/// window. Aggregated across all producing hubs + in-flight cargo.
#[derive(Serialize, Clone)]
pub struct GoodMarketRow {
    pub good: String,
    pub best_quality: f32,
    pub best_grade: String,
    pub best_city: String,
    pub avg_quality: f32,
    pub produced: f32,
    pub traded: f32,
    pub n_producers: u32,
    pub manufactured: bool,
    /// Per quality-grade breakdown (Exquisite→Coarse), only non-empty tiers — so the
    /// Goods window can show e.g. Wheat · Fine / Standard / Coarse separately.
    pub grades: Vec<GradeBucket>,
}

/// DLC 4 · one quality tier of a good: how much is produced & in trade at that grade.
#[derive(Serialize, Clone)]
pub struct GradeBucket {
    pub grade: String,   // "Exquisite" | "Fine" | "Standard" | "Common" | "Coarse"
    pub produced: f32,
    pub traded: f32,
    pub n_producers: u32,
}

/// Grade tier index 0..4 (Coarse..Exquisite) for a quality 0..1.
fn grade_tier(q: f32) -> usize {
    if q >= 0.85 { 4 } else if q >= 0.68 { 3 } else if q >= 0.50 { 2 } else if q >= 0.32 { 1 } else { 0 }
}
const GRADE_NAMES: [&str; 5] = ["Coarse", "Common", "Standard", "Fine", "Exquisite"];

/// One merchant family for the Houses panel / settlement window.
#[derive(Serialize)]
pub struct HouseBrief {
    /// Index into `sim.houses` — the key for per-house detail queries (the ledger).
    #[serde(default)] pub idx: u32,
    /// Phase G — city names this house is currently BARRED from (active trade wars).
    #[serde(default)] pub barred: Vec<String>,
    pub name: String,        // "House Cassii"
    pub head_name: String,   // "Marcus Cassii"
    pub home_hub: u32,       // home hub id
    pub home_name: String,
    pub wealth: f32,
    pub prestige: f32,
    pub political_power: f32,
    #[serde(default)] pub volume: f32, // recent trade volume — "trade amount" the house moves
    pub generation: u32,
    pub head_age: u32,       // years the current head has led
    pub specialties: Vec<String>,       // good names
    pub top_goods: Vec<String>,         // top exported/traded goods by cumulative profit
    pub monopolies: Vec<(String, f32)>, // good name + share 0..1
    pub rivals: Vec<String>,            // rival house names
    pub defunct: bool,
    /// Stable, maximally-distinct colour for this house (hex, golden-angle hue).
    /// Same colour used everywhere: map overlay, houses panel, settlement pie.
    #[serde(default)] pub color: String,
    /// Home-seat position (world cell coords) — where the family is based.
    #[serde(default)] pub seat: [f32; 2],
    /// True when this house holds >=50% of its seat city's merchant trade — i.e.
    /// it controls that settlement. Only controlled seats are tinted on the map.
    #[serde(default)] pub dominant: bool,
    /// Settlements this house CONTROLS (world cell coords): any city — its seat OR
    /// a distant outpost — where it handles >=50% of the trade throughput. A
    /// big-city family can completely control a remote outpost it supplies and
    /// funnel its wealth home.
    #[serde(default)] pub controls: Vec<[f32; 2]>,
    /// Trade-partner / handled settlements (world cell coords) used to colour the
    /// routes the house runs.
    #[serde(default)] pub partners: Vec<[f32; 2]>,
    /// Names of the cities this house trades with / controls (seat first) — shown
    /// in the Houses menu.
    #[serde(default)] pub cities: Vec<String>,
    /// Archetype + its label/perk and any goods it holds city charters on.
    #[serde(default)] pub archetype: u8,
    #[serde(default)] pub archetype_label: String,
    #[serde(default)] pub archetype_perk: String,
    #[serde(default)] pub charters: Vec<String>,
    /// The house's transport capital — each vessel is one concurrent shipment slot.
    #[serde(default)] pub fleet_sea: u32,
    #[serde(default)] pub fleet_river: u32,
    #[serde(default)] pub fleet_caravan: u32,
    /// True = a civic Merchant Guild (acts for its home city), not a private house.
    #[serde(default)] pub is_guild: bool,
    /// Foreign cities where this holder has opened an OFFICE: `(name, [x,y])`.
    #[serde(default)] pub offices: Vec<(String, [f32; 2])>,
    /// Estates/manufactories this holder owns: `(good, host-city)`.
    #[serde(default)] pub estates: Vec<(String, String)>,
    /// Cities this house is active in, ranked MOST → LEAST influential, each tagged
    /// with its role (seat / bailo / dominant / office / trade) + contested flag.
    #[serde(default)] pub active: Vec<HouseCity>,
    // ── DLC 3.5 · richer individual stats + bank/coin links ──
    /// This family owns a chartered bank (→ 🏦 badge + underline + Bank subtab).
    #[serde(default)] pub owns_bank: bool,
    /// Year the house was founded (→ "est. 71 · 88y old").
    #[serde(default)] pub founded_year: u32,
    /// The worst single-month loss the family ever took (grain-eq).
    #[serde(default)] pub worst_loss: f32,
    /// Count of goods the house has EVER monopolised (all-time).
    #[serde(default)] pub mono_ever_count: u32,
    /// If this family's council governs a minting city, the coin it issues (else "").
    #[serde(default)] pub coin_name: String,
    #[serde(default)] pub coin_value: f32,
    #[serde(default)] pub coin_trust: f32,
    /// Phase 1.1 · rank band among live private houses: 1 great .. 4 marginal, 0 if
    /// never computed (a guild, or a house founded this tick before the monthly pass).
    #[serde(default)] pub tier: u8,
    #[serde(default)] pub standing: f32,
    /// Phase 1.2 · the seat culture's language-kit index (see `sim::cultures::KITS`),
    /// −1 if unresolvable — drives the dossier's culture-dress figure.
    #[serde(default = "neg_one_i32c")] pub kit: i32,
    #[serde(default)] pub head_female: bool,
    /// Phase 1.4 · positive events (§2.2) — the house's all-time peak wealth and when
    /// it was reached ("the finest hour"), shown as a fact rather than an event.
    #[serde(default)] pub peak_wealth: f32,
    #[serde(default)] pub peak_wealth_tick: u32,
    /// The house's per-good trade ledger — the goods it has moved the most, richest
    /// first by cumulative VOLUME, each carrying the amount shipped and the profit it
    /// earned. Drives the Compare window's "goods traded most / most profitable trade"
    /// with real numbers instead of just a name list. Up to 6 rows.
    #[serde(default)] pub goods_ledger: Vec<GoodTradeRow>,
}

/// One good in a house's trade ledger: how much it has shipped and what it earned.
#[derive(Serialize, Clone)]
pub struct GoodTradeRow {
    pub good: String,
    pub volume: f32,  // cumulative amount shipped (grain-eq units)
    pub profit: f32,  // cumulative profit earned on that good
}

/// One city a house operates in, for the influence-ranked "Active in" list.
#[derive(Serialize, Clone)]
pub struct HouseCity {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub influence: f32,
    /// "seat" | "bailo" | "dominant" | "office" | "trade".
    pub role: String,
    /// A rival also holds significant influence here.
    pub contested: bool,
}

/// Golden-angle hue → a distinct, saturated hex colour. `i` is a stable index so
/// each house keeps its colour across refreshes.
fn distinct_color(i: usize) -> String {
    let hue = (i as f32 * 137.508) % 360.0;
    // Fixed S/L for vivid but readable colours on the green map.
    let (s, l) = (0.68f32, 0.55f32);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = hue / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u32;
    format!("#{:02x}{:02x}{:02x}", to(r1), to(g1), to(b1))
}

fn build_house_briefs(sim: &CampaignSim) -> Vec<HouseBrief> {
    let gname = |g: usize| sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
    let hub_name = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let seat_pos = |hub: usize| -> [f32; 2] {
        sim.hubs.get(hub).map(|x| [x.x, x.y]).unwrap_or([0.0, 0.0])
    };

    // ── Throughput-based control ─────────────────────────────────────────────
    // Every in-flight shipment touches two cities (its source and destination). A
    // house that OWNS the shipment "handles" that trade at both ends. A house
    // controls a settlement when it handles >=50% of that settlement's total trade
    // throughput — its own seat OR a remote outpost it supplies. Guild/independent
    // trade (owner -1) counts toward the total but controls nothing, so a city
    // mostly served by guilds has no controlling house.
    let nhubs = sim.hubs.len();
    let nh = sim.houses.len();
    let mut hub_total: Vec<f32> = vec![0.0; nhubs];        // all throughput per hub
    let mut hub_house: Vec<f32> = vec![0.0; nhubs * nh.max(1)]; // [hub*nh + house]
    for s in &sim.in_transit {
        let amt = s.amount.max(0.0);
        for &h in &[s.from as usize, s.to as usize] {
            if h < nhubs {
                hub_total[h] += amt;
                if s.owner >= 0 {
                    let oi = s.owner as usize;
                    if oi < nh { hub_house[h * nh + oi] += amt; }
                }
            }
        }
    }
    // Per house: the hubs it controls (>=50% throughput) and the hubs it trades at.
    let mut controlled: Vec<Vec<usize>> = vec![Vec::new(); nh];
    let mut handled: Vec<Vec<usize>> = vec![Vec::new(); nh];
    for h in 0..nhubs {
        if hub_total[h] <= 1e-6 { continue; }
        for oi in 0..nh {
            let v = hub_house[h * nh + oi];
            if v <= 0.0 { continue; }
            handled[oi].push(h);
            // A house controls a city when it handles >=40% of the city's trade
            // throughput (lowered from 50% so dominance actually emerges now that
            // importer-side houses also carry trade).
            if v / hub_total[h] >= 0.4 { controlled[oi].push(h); }
        }
    }

    // Per-hub second-highest influence → the "contested" flag for the Active-in list.
    let mut hub_top2 = vec![(0.0f32, 0.0f32); nhubs]; // (top, second)
    for h in &sim.houses {
        if h.defunct { continue; }
        for &(hb, x) in &h.influence {
            let c = hb as usize; if c >= nhubs { continue; }
            if x > hub_top2[c].0 { hub_top2[c].1 = hub_top2[c].0; hub_top2[c].0 = x; }
            else if x > hub_top2[c].1 { hub_top2[c].1 = x; }
        }
    }

    let mut out: Vec<HouseBrief> = sim.houses.iter().enumerate().map(|(hi, h)| {
        let hub = h.hub as usize;
        // Influence-ranked "Active in" list. Estates/manufactories are NOT independent
        // places — they belong to a settlement — so fold their influence into the
        // PARENT settlement (an office there already links the house to that
        // manufactory). Keep the strongest influence seen at the settlement.
        let display_hub = |c: usize| -> usize {
            if c < nhubs && sim.hubs[c].is_estate && sim.hubs[c].parent >= 0 {
                sim.hubs[c].parent as usize
            } else { c }
        };
        let mut agg: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for &(hb, infl) in &h.influence {
            let d = display_hub(hb as usize);
            let e = agg.entry(d).or_insert(0.0);
            if infl > *e { *e = infl; }
        }
        let mut active: Vec<HouseCity> = agg.into_iter().map(|(c, infl)| {
            let hb = c as u32;
            // Does this house OWN an estate/manufactory sited at this settlement? Then
            // it's the OWNER (it just transports its own goods) rather than a buyer.
            let owns_here = sim.hubs.iter().any(|e|
                e.is_estate && e.owner_house == hi as i32 && e.parent == c as i32);
            let role = if hb == h.hub { "seat" }
                else if h.bailos.contains(&hb) { "bailo" }
                else if sim.city_dominator.get(c).copied().unwrap_or(-1) == hi as i32 { "dominant" }
                else if owns_here { "owner" }
                else if h.offices.contains(&hb) { "office" }
                else { "trade" };
            let contested = c < nhubs && hub_top2[c].1 >= 0.30;
            let p = seat_pos(c);
            HouseCity { name: hub_name(hb), x: p[0], y: p[1], influence: infl, role: role.into(), contested }
        }).collect();
        active.sort_by(|a, b| b.influence.partial_cmp(&a.influence).unwrap_or(std::cmp::Ordering::Equal));
        // A house always counts its seat among the cities listed.
        let mut ctrl_hubs = std::mem::take(&mut controlled[hi]);
        // A house that DOMINATES its seat by trade volume (>=50% of resident-house
        // trade, the `dominant_seat` signal) also colours its home city — even when
        // in-flight throughput is momentarily sparse, so the map isn't all grey.
        if h.dominant_seat && hub < nhubs && !ctrl_hubs.contains(&hub) {
            ctrl_hubs.push(hub);
        }
        let trade_hubs = std::mem::take(&mut handled[hi]);
        let dominant = !ctrl_hubs.is_empty(); // controls at least one settlement
        let controls: Vec<[f32; 2]> = ctrl_hubs.iter().map(|&p| seat_pos(p)).collect();
        let partners: Vec<[f32; 2]> = trade_hubs.iter().filter(|&&p| p != hub)
            .map(|&p| seat_pos(p)).collect();
        let mut cities: Vec<String> = Vec::new();
        if hub < nhubs { cities.push(hub_name(hub as u32)); }
        for &p in &ctrl_hubs { if p != hub { cities.push(format!("{} (controlled)", hub_name(p as u32))); } }
        for &p in &trade_hubs { if p != hub && !ctrl_hubs.contains(&p) { cities.push(hub_name(p as u32)); } }
        // The coin this family mints: if it is the council of a city with a coin.
        let minting = sim.hubs.iter()
            .find(|c| !c.is_estate && c.council_house == hi as i32 && !c.coin_name.is_empty());
        let (coin_for_house, coin_val_house, coin_trust_house) = match minting {
            Some(c) => (c.coin_name.clone(), crate::sim::tick::coin_value(c.mint_fineness, c.coin_trust), c.coin_trust),
            None => (String::new(), 0.0, 0.0),
        };
        HouseBrief {
            idx: hi as u32,
            name: h.name.clone(),
            head_name: h.head_name.clone(),
            home_hub: sim.hubs.get(hub).map(|x| x.id).unwrap_or(0),
            home_name: hub_name(h.hub),
            wealth: h.wealth,
            prestige: h.prestige,
            political_power: h.political_power,
            volume: h.volume,
            generation: h.generation,
            head_age: sim.tick.saturating_sub(h.head_since) / 365,
            specialties: h.spec.iter().map(|&g| gname(g)).collect(),
            top_goods: {
                // Top goods this family is KNOWN FOR — most profitable goods it has
                // traded (up to 3), so the list shows its trade identity (#14).
                let mut tg: Vec<(usize, f32)> = h.good_profit.iter().enumerate()
                    .filter(|(_, &p)| p > 0.0).map(|(g, &p)| (g, p)).collect();
                tg.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                tg.into_iter().take(3).map(|(g, _)| gname(g)).collect()
            },
            monopolies: h.monopoly.iter().map(|&(g, s)| (gname(g), s)).collect(),
            rivals: h.rivals.iter().filter_map(|&r| sim.houses.get(r).map(|x| x.name.clone())).collect(),
            defunct: h.defunct,
            color: distinct_color(hi), // stable per-house index → stable colour
            seat: seat_pos(hub),
            dominant,
            controls,
            partners,
            cities,
            barred: sim.house_barred.get(hi).map(|v| v.iter().map(|&c| hub_name(c)).collect()).unwrap_or_default(),
            archetype: h.archetype,
            archetype_label: crate::sim::tick::archetype_label(h.archetype).to_string(),
            archetype_perk: crate::sim::tick::archetype_perk(h.archetype).to_string(),
            charters: h.charters.iter().map(|&g| gname(g)).collect(),
            fleet_sea: h.fleet_sea,
            fleet_river: h.fleet_river,
            fleet_caravan: h.fleet_caravan,
            is_guild: h.is_guild,
            offices: h.offices.iter()
                .map(|&oh| (hub_name(oh), seat_pos(oh as usize)))
                .collect(),
            estates: sim.hubs.iter()
                .filter(|e| e.is_estate && e.owner_house == hi as i32)
                .map(|e| {
                    let g = e.base_per_capita.iter().enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i).unwrap_or(0);
                    let city = if e.parent >= 0 {
                        sim.hubs.get(e.parent as usize).map(|p| p.name.clone()).unwrap_or_default()
                    } else { String::new() };
                    (gname(g), city)
                })
                .collect(),
            active,
            owns_bank: sim.banks.iter().any(|b| !b.defunct && b.house as usize == hi),
            founded_year: h.founded_tick / crate::sim::tick::TICKS_PER_YEAR,
            worst_loss: h.worst_loss,
            mono_ever_count: h.mono_ever.len() as u32,
            // The coin this family mints, if its council governs a minting city.
            coin_name: coin_for_house.clone(),
            coin_value: coin_val_house,
            coin_trust: coin_trust_house,
            tier: h.tier,
            standing: h.standing,
            kit: crate::sim::cultures::kit_of_people(&sim.hub_culture.get(hub).cloned().unwrap_or_default())
                .map(|k| k as i32).unwrap_or(-1),
            head_female: h.head_female,
            peak_wealth: h.peak_wealth,
            peak_wealth_tick: h.peak_wealth_tick,
            goods_ledger: {
                // The goods this house has moved the most (by cumulative volume),
                // each paired with the amount shipped and the profit it earned. The
                // frontend reads "most traded" off the order and "most profitable"
                // off the max profit — both with real numbers, not just a name.
                let mut rows: Vec<GoodTradeRow> = h.good_volume.iter().enumerate()
                    .filter(|(_, &v)| v > 0.0)
                    .map(|(g, &v)| GoodTradeRow {
                        good: gname(g),
                        volume: v,
                        profit: h.good_profit.get(g).copied().unwrap_or(0.0),
                    })
                    .collect();
                rows.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap_or(std::cmp::Ordering::Equal));
                rows.truncate(6);
                rows
            },
        }
    }).collect();
    // Active first, then richest first.
    out.sort_by(|a, b| (a.defunct, -a.wealth).partial_cmp(&(b.defunct, -b.wealth))
        .unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// One ACTIVE merchant route for the campaign merchant overlay: a holder's live
/// shipments between two cities, aggregated, with the goods carried each way.
#[derive(Serialize)]
pub struct MerchantRoute {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub a_name: String,
    pub b_name: String,
    pub holder: String,
    pub color: String,
    pub is_guild: bool,
    pub sea: bool,
    pub volume: f32,
    /// Goods flowing a→b and b→a (name, volume), each sorted by volume.
    pub out_goods: Vec<(String, f32)>,
    pub ret_goods: Vec<(String, f32)>,
}

/// One active FUTURES CONTRACT as a directional supply lane for the Futures map
/// layer: the seller's source city → the buyer city, with the good, monthly volume,
/// term and end year. Distinct from `MerchantRoute` (live spot voyages) — these are
/// standing contractual obligations.
#[derive(Serialize)]
pub struct FuturesLane {
    pub a: [f32; 2],      // source (producer/warehouse) city
    pub b: [f32; 2],      // buyer (receiver) city
    pub a_name: String,
    pub b_name: String,
    pub holder: String,   // seller house/guild
    pub color: String,
    pub is_guild: bool,
    pub good: String,
    pub qty: f32,         // monthly delivered quantity
    pub term: u8,         // 1 / 3 / 5 / 7 years
    pub end_year: u32,    // campaign year the contract expires
    pub suspended: bool,  // force-majeure (plague lockup) right now
    #[serde(default)] pub delivered: f32,      // running total delivered to date
    #[serde(default)] pub fulfilled_pct: f32,  // delivered vs what was due by now (0-100)
    #[serde(default)] pub value: f32,          // grain-eq value moved so far
    #[serde(default)] pub sealed_at: String,   // city where the deal was struck (buyer)
}

/// One house/guild warehouse for the Warehouses infographic: where it sits, its
/// tier/capacity/fill, the goods it holds, and how many futures contracts it supplies.
#[derive(Serialize)]
pub struct WarehouseInfo {
    /// "warehouse" for a depot, else the estate kind (farm/mine/plantation/fishery/
    /// vineyard/manufactory).
    pub kind: String,
    pub owner: String,
    pub color: String,
    pub is_guild: bool,
    pub city: String,
    pub x: f32,
    pub y: f32,
    pub tier: u8,
    pub capacity: f32,
    pub used: f32,
    /// (good name, amount): a warehouse's stock, or an estate's production — largest first.
    pub goods: Vec<(String, f32)>,
    /// Active futures contracts this depot is the SOURCE of.
    pub contracts: u32,
    pub damage: f32,
}

/// One city in the live "richest cities" ranking.
#[derive(Serialize)]
pub struct CityRank {
    pub id: u32,
    pub name: String,
    pub population: u32,
    pub wealth: f32,      // grain + trade wealth
    pub trade: f32,       // throughput (value bought + sold)
    pub pct_world: f32,   // share of all world trade (%)
    // C1 · a single honest PROSPERITY composite: flow (trade) + public stock
    // (treasury) + broad-based prosperity (commoner wealth) + equity (1 − inequality),
    // each normalized across cities. Lets the panel rank "richest" honestly instead of
    // conflating it with trade VOLUME alone.
    #[serde(default)] pub prosperity: f32,
    #[serde(default)] pub treasury: f32,
    #[serde(default)] pub commoner_wealth: f32,
    #[serde(default)] pub inequality: f32,
}

/// Trade diagnostics — a snapshot to answer "is trade actually moving?".
#[derive(Serialize)]
pub struct CampaignDiagnostics {
    pub tick: u32,
    pub year: u32,
    pub in_transit: u32,          // shipments currently in flight
    pub shipments_last: u32,      // shipments dispatched over the last advance
    pub by_house: u32,            // ...financed by a merchant house
    pub by_guild: u32,            // ...carried by local merchants/guilds
    pub lost_last: u32,           // voyages lost (storm/ambush) last advance
    pub volume_last: f32,         // goods volume shipped last advance
    pub houses_active: u32,
    pub houses_defunct: u32,
    pub fleet_sea: u32,
    pub fleet_river: u32,
    pub fleet_caravan: u32,
    pub controlled_settlements: u32, // cities a house controls (>=50% throughput)
    pub total_house_wealth: f32,
}

/// #29 · one year's wealth-inequality reading among the merchant houses.
#[derive(Serialize, Clone, Default)]
pub struct InequalityPoint {
    pub year: u32,
    pub gini: f32,        // 0 = perfectly equal, →1 = one house holds it all
    pub active: u32,      // houses with positive wealth that year
    pub mean_wealth: f32,
    pub top10_share: f32, // wealth share of the richest 10%
}

/// #29 · the wealth-inequality & social-mobility snapshot for the Economy
/// Dashboard. Pure read over the campaign sim — Gini now, a yearly Gini/top-share
/// trend reconstructed from each house's `wealth_history`, plus turnover stats.
#[derive(Serialize, Clone, Default)]
pub struct InequalitySnapshot {
    pub active: bool,
    pub year: u32,
    pub gini_now: f32,
    pub top10_share_now: f32,
    pub active_houses: u32,
    pub defunct_houses: u32,
    pub founded_total: u32,
    /// 0 = a frozen pecking order, →1 = ranks reshuffle wildly year to year.
    pub rank_churn: f32,
    pub series: Vec<InequalityPoint>, // oldest → newest
}

/// Gini coefficient of a list of (positive) wealths. 0 = equal, →1 = concentrated.
fn gini_of(mut v: Vec<f32>) -> f32 {
    v.retain(|&x| x > 0.0);
    let n = v.len();
    if n < 2 { return 0.0; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sum: f32 = v.iter().sum();
    if sum <= 0.0 { return 0.0; }
    let mut cum = 0.0f32;
    for (i, &x) in v.iter().enumerate() { cum += (i as f32 + 1.0) * x; }
    (((2.0 * cum) / (n as f32 * sum)) - (n as f32 + 1.0) / n as f32).clamp(0.0, 1.0)
}

/// Wealth share held by the richest 10% (at least one house).
fn top10_share_of(mut v: Vec<f32>) -> f32 {
    v.retain(|&x| x > 0.0);
    if v.is_empty() { return 0.0; }
    v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let total: f32 = v.iter().sum();
    if total <= 0.0 { return 0.0; }
    let k = ((v.len() as f32 * 0.1).ceil() as usize).max(1);
    v.iter().take(k).sum::<f32>() / total
}

/// DLC 3 · the POLEIS as actors — each seat city's treasury, council-set tariff /
/// mint policy, and the house that governs it. For the Polis panel.
#[derive(Serialize, Clone)]
pub struct PolisBrief {
    pub hub: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub treasury: f32,
    /// Effective export / import tariff fractions in force.
    pub tariff_export: f32,
    pub tariff_import: f32,
    /// Mint fineness (1.0 = full coin, < 1 = debased "cheap money").
    pub mint_fineness: f32,
    /// Governing house ("—" if none) + its archetype label.
    pub council: String,
    pub council_archetype: String,
    pub council_color: String,
    /// DLC 3.5 · the polis's named coin ("" = none) + its acceptance/trust 0..1.
    pub coin_name: String,
    pub coin_trust: f32,
    /// Coin value index (≈1.2 = strong agio, <1 = debased/distrusted).
    pub coin_value: f32,
    /// Issuing house (the council) whose arms ride the coin; "" → use the city.
    pub coin_issuer: String,
    /// The polis this city is at war with ("" = at peace).
    pub war_with: String,
}

/// DLC 3.5 · one currency in the world reserve-currency ranking. Surfaced in the
/// Coin & Credit panel's "Currencies" tab.
#[derive(Serialize, Clone)]
pub struct CurrencyBrief {
    pub hub: u32,
    pub city: String,
    pub coin_name: String,
    /// Acceptance / trust 0..1.
    pub trust: f32,
    /// Mint fineness (1 = full coin, < 1 = debased).
    pub fineness: f32,
    /// Recent trade throughput at the issuing city (the reserve weight).
    pub throughput: f32,
    /// True once trust clears the reserve-currency floor (accepted abroad).
    pub is_reserve: bool,
    pub color: String,
    /// Value index (≈1.2 strong agio, <1 debased). Display metric.
    pub value: f32,
    /// Issuing house (council) whose arms ride the coin; "" → use the city.
    pub issuer: String,
    /// Circulating amount = Σ over all holding cities of (throughput × share) — the
    /// coin's effective money supply across the world (display-only for now).
    pub circulating: f32,
    /// How many settlements hold this coin in their basket.
    pub held_in: u32,
}

/// v2.0 · one MINT/polis in the unified "Coin & Mints" view — the polis (treasury,
/// tariffs, council, war) AND its coin (strength, drivers, reach, price level)
/// fused into a single card, replacing the old split Poleis + Currencies tabs.
#[derive(Serialize, Clone)]
pub struct MintBrief {
    pub hub: u32,
    pub city: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    // ── civic (the polis behind the mint) ──
    pub treasury: f32,
    pub tariff_export: f32,
    pub tariff_import: f32,
    pub council: String,
    pub council_archetype: String,
    pub council_color: String,
    pub war_with: String,
    // ── the coin ("" coin_name = this polis mints none) ──
    pub coin_name: String,
    pub issuer: String,
    /// The metal it is struck in: "gold" | "silver" | "electrum" | "bronze".
    pub metal: String,
    pub trust: f32,
    pub fineness: f32,
    pub value: f32,
    /// v2.1 · metal-aware INTRINSIC exchange value (silver-coin numeraire = 1): a gold
    /// coin ≈ GOLD_SILVER_RATIO× a silver one of equal fineness. Cross-coin rates read
    /// from this, so a Ducat and a Florin no longer mis-exchange 1:1.
    #[serde(default)] pub exchange: f32,
    /// Single headline 0..100 (fineness × acceptance) — the number the card leads with.
    pub strength: f32,
    pub throughput: f32,
    pub is_reserve: bool,
    pub circulating: f32,
    pub held_in: u32,
    pub abroad: u32,
    // ── v2.0 monetary loop + reform ──
    /// Local price-level index (1.0 = par at start). Rises with debasement/money growth.
    pub price_level: f32,
    /// Bullion capacity: "ample" | "tight" | "scarce" — how the region's gold/silver
    /// supply constrains minting (the coin-supply limiting factor).
    pub bullion: String,
    /// v2.0 · holds the RIGHT OF THE MINT (a charter). A coinless council seat that
    /// lacks this simply isn't a big enough commercial centre to coin yet.
    pub has_mint: bool,
    /// Honest-money mandate currently in force (no debasement allowed).
    pub under_mandate: bool,
    /// This mint has reformed its coinage at least once.
    pub reformed: bool,
    // ── B3 · civic public debt (the Monte) ──
    /// Principal the city owes its bondholders (0 = no public debt).
    #[serde(default)] pub debt_principal: f32,
    /// The annual coupon rate the city pays on its bonds.
    #[serde(default)] pub debt_coupon: f32,
    /// Debt as a multiple of yearly throughput — the fiscal-burden read (≥3 = crisis).
    #[serde(default)] pub debt_ratio: f32,
    /// Number of distinct bondholders (patrician houses funding the debt).
    #[serde(default)] pub debt_holders: u32,
}

/// v2.0 · one entry in the MONETARY CHRONICLE — the dated story of money (mints,
/// debasements, reforms, bank foundings, runs, crashes) for the Shocks timeline.
#[derive(Serialize, Clone)]
pub struct MonetaryEvent {
    pub year: u32,
    pub tick: u32,
    pub kind: String,   // coinage | reform | run | bank | crash
    pub city: String,   // resolved hub name ("" = world)
    pub value: f32,
    pub text: String,
}

/// One city's USE of a coin (for the coin-usage overlay + per-coin breakdown chart).
#[derive(Serialize)]
pub struct CoinUseCity {
    pub coin: u32,           // issuing-mint hub id — which coin this city settles in
    pub coin_name: String,
    pub city: u32,           // the city hub id using it
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub volume: f32,         // trade throughput settled in this coin at this city
    pub share: f32,          // this coin's share of the city's currency basket 0..1
    pub mint: bool,          // this city is the coin's own mint
    pub primary: bool,       // this coin is the city's MAIN settlement currency (settle_coin)
    pub reserve_reach: bool, // a foreign reserve coin circulating here (held, not primary)
    pub color: String,       // stable per-coin colour (its council's arms) for the dominance map
}

/// v2.0 · one coin in a holder's currency reserves (a donut slice).
#[derive(Serialize, Clone)]
pub struct ReserveSlice {
    pub coin_name: String,
    pub color: String,
    pub metal: String,
    pub share: f32,     // 0..1 of the holder's reserves
    pub primary: bool,  // the holder's main/settlement coin
    pub mint: bool,     // the holder's own city mints this coin
}

/// v2.0 · one holder (city / bank / house) and the currency composition of its
/// reserves — the inward view (what coins it HOLDS), for the Reserves donuts.
#[derive(Serialize, Clone)]
pub struct ReserveHolder {
    pub kind: String,   // "city" | "bank" | "house"
    pub name: String,
    pub seat: String,   // home/seat city ("" for a city holder)
    pub total: f32,     // total reserves/wealth (grain-equivalent)
    pub slices: Vec<ReserveSlice>,
}

#[derive(Serialize, Clone)]
pub struct ReservesPayload {
    pub cities: Vec<ReserveHolder>,
    pub banks: Vec<ReserveHolder>,
    pub houses: Vec<ReserveHolder>,
}

/// DLC 3.5 · one bank's balance sheet + reach, for the Coin & Credit panel.
#[derive(Serialize, Clone)]
pub struct BankBrief {
    pub name: String,
    pub seat: String,
    /// The coin the bank banks in (its seat city's coin) + its value (×grain), so the
    /// balance sheet can be shown denominated in real money.
    pub coin_name: String,
    pub coin_value: f32,
    pub owner: String,
    /// Owning house index — lets the Houses panel match a bank to the selected house.
    pub owner_idx: u32,
    pub color: String,
    pub founded_year: u32,
    pub defunct: bool,
    // ── Balance sheet (grain-eq) ──
    pub reserves: f32,
    pub loans_out: f32,
    pub real_estate: f32,
    pub deposits: f32,
    pub notes_issued: f32,
    pub equity: f32,
    pub reserve_ratio: f32,
    pub n_loans: u32,
    pub interest_earned: f32,
    pub losses: f32,
    /// Book value of equity stakes + cumulative stake dividends collected.
    pub stake_book: f32,
    pub dividends_earned: f32,
    /// B4 · cumulative bills-of-exchange (FX-spread) income.
    #[serde(default)] pub bills_income: f32,
    /// Seat coordinates (cell space) — for the bank icon on the map.
    pub seat_x: f32,
    pub seat_y: f32,
    /// Cities hosting a counting-house branch.
    pub branches: Vec<String>,
    /// Recent chronicle lines (founding, branches, defaults, failure).
    pub events: Vec<String>,
    /// Yearly balance-sheet history (charts).
    pub history: Vec<crate::sim::tick::BankSnapshot>,
    /// Every live loan/deal on the books, with its agreement terms.
    pub loans: Vec<BankLoanRow>,
    /// Equity stakes the bank holds in manufactories.
    pub stakes: Vec<BankStakeRow>,
}

/// One loan/deal on a bank's books, with the agreement terms (for the deals list).
#[derive(Serialize, Clone)]
pub struct BankLoanRow {
    /// Borrower name (house, guild, or city).
    pub borrower: String,
    /// "house" | "guild" | "polis".
    pub borrower_kind: String,
    /// "trade" | "guild_factory" | "guild_civic" | "treasury" | "colony".
    pub purpose: String,
    pub principal: f32,
    pub outstanding: f32,
    /// Monthly interest rate.
    pub rate: f32,
    pub start_year: u32,
    pub term_years: f32,
}

/// One equity stake a bank holds in a manufactory.
#[derive(Serialize, Clone)]
pub struct BankStakeRow {
    pub works: String,
    pub good: String,
    pub share: f32,
    pub basis: f32,
}

/// DLC 3.5 · the economic-war picture: active wars + the concluded-war log.
#[derive(Serialize, Clone)]
pub struct WarBrief {
    pub a: String,
    pub b: String,
    pub start_year: u32,
    pub years: u32,
    pub chest_a: f32,
    pub chest_b: f32,
    pub levies: f32,
    pub cause: String,
    /// CITY_PROVINCE_WAR_PLAN.md §3.4a · bidirectional war score, −100..100.
    /// Positive favours `a`. ±100 ends the war outright.
    #[serde(default)] pub score: f32,
    /// §3.4a · quarterly rounds fought so far (of `WAR_ROUND_CAP`'s backstop).
    #[serde(default)] pub round: u16,
    /// §3.4b · what the aggressor is fighting for, as a phrase — the same label
    /// `war_goal_label` already uses in the journal.
    #[serde(default)] pub goal_label: String,
}
#[derive(Serialize, Clone)]
pub struct WarsPayload {
    pub active: Vec<WarBrief>,
    pub log: Vec<crate::sim::tick::WarRecord>,
}

/// Phase 6 · one plague-struck city inside an epidemic (for the Plagues panel + map).
#[derive(Serialize, Clone)]
pub struct PlagueCityBrief {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub deaths: u32,
    /// SIR · people who fell ill in this strike (`ill` ≥ `deaths`).
    #[serde(default)] pub ill: u32,
    /// SIR · people who fell ill and survived (`ill − deaths`).
    #[serde(default)] pub recovered: u32,
    /// Population that survived the strike (immediate aftermath).
    pub pop: u32,
    /// Still under quarantine right now.
    pub active: bool,
    /// The city the pestilence was carried from ("" = spontaneous origin).
    pub from_name: String,
    /// Year this city was struck.
    pub year: u32,
    /// Spread step: 0 = the origin, 1,2,… as the pestilence travelled the lanes.
    pub order: u32,
}

/// Phase 6 · an EPIDEMIC = a contagion chain (all strikes sharing an outbreak id).
#[derive(Serialize, Clone)]
pub struct EpidemicBrief {
    pub id: u32,
    pub name: String,
    /// The city the outbreak began in.
    pub origin_name: String,
    pub start_year: u32,
    pub end_year: u32,
    pub active: bool,
    pub total_dead: u32,
    /// SIR · total people who fell ill / recovered across the whole outbreak.
    #[serde(default)] pub total_ill: u32,
    #[serde(default)] pub total_recovered: u32,
    /// Plague category: 1 = Great Plague (rare, reaches ~4000 km along the lanes),
    /// 2 = Regional (reaches one further city), 3 = Local outbreak (stays put).
    pub category: u8,
    /// The named DISEASE (Bubonic Plague, Cholera, Malaria, …) + its transmission mode.
    #[serde(default)] pub disease: String,
    #[serde(default)] pub transmission: String,
    /// Cities hit, in SPREAD ORDER (origin first). Each `from_name`→`name` is a
    /// contagion route; the panel can re-sort by deaths.
    pub cities: Vec<PlagueCityBrief>,
}

/// Phase 6 · one craft guild (for the Guilds & Crafts panel + map).
#[derive(Serialize, Clone)]
pub struct GuildBrief {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub city: String,
    pub good: u32,
    pub good_name: String,
    pub quality: f32,
    pub output: f32,
    pub strength: f32,
    pub hall: bool,
    pub luxury: bool,
    /// True when the guild's craft is EXCEPTIONAL (renowned enough to be branded).
    pub exceptional: bool,
    /// Place-brand for an exceptional craft ("Veyra cloth" — like Murano glass /
    /// Damascus steel), else "". `culture` names the people whose style it is.
    pub brand: String,
    pub culture: String,
}

/// Quality at/above which a guild's craft earns a place-brand (renowned).
const GUILD_EXCEPTIONAL: f32 = 0.80;

/// Phase 6 · one notable figure (Great Lives roster).
#[derive(Serialize, Clone)]
pub struct FigureBrief {
    pub name: String,
    pub role: String,
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub city: String,
    /// The craft a master craftsman is renowned for (else "").
    pub good_name: String,
    pub born_year: u32,
    pub died_year: u32,
    pub alive: bool,
}

/// Phase 6 · one landmark / place of note (wonders, holy cities, fair towns, guildhalls).
#[derive(Serialize, Clone)]
pub struct LandmarkBrief {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub city: String,
    /// "wonder" | "temple" | "fair" | "guildhall".
    pub kind: String,
    pub label: String,
    pub detail: String,
}

/// Phase 7 · a link between two houses (a marriage alliance or a feud).
#[derive(Serialize, Clone)]
pub struct HouseLink {
    pub a_name: String,
    pub b_name: String,
    pub a_hub: u32,
    pub b_hub: u32,
    pub ax: f32,
    pub ay: f32,
    pub bx: f32,
    pub by: f32,
    pub a_city: String,
    pub b_city: String,
}

/// Phase 7 · the Dynasties & Alliances panel: marriage alliances + feuds.
#[derive(Serialize, Clone)]
pub struct DynastiesPayload {
    pub alliances: Vec<HouseLink>,
    pub feuds: Vec<HouseLink>,
}

/// DLC 3.5 · one city's "schematic" — its standing buildings, estates, bank
/// presence and coin, for the Schematics (blueprint) view.
#[derive(Serialize, Clone)]
pub struct SchematicBuilding { pub label: String, pub effect: String }
#[derive(Serialize, Clone)]
pub struct SchematicEstate { pub label: String, pub tier: u8, pub owner: String, pub good: String }
#[derive(Serialize, Clone)]
pub struct CitySchematic {
    pub hub: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub population: u32,
    pub coin_name: String,
    pub coin_trust: f32,
    /// Metal the city's coin is struck in ("gold" | "silver" | "electrum" | "bronze").
    pub coin_metal: String,
    pub council: String,
    pub buildings: Vec<SchematicBuilding>,
    pub estates: Vec<SchematicEstate>,
    /// Banks seated in this city.
    pub banks_seated: Vec<String>,
    /// Banks with a counting-house branch here.
    pub bank_branches: Vec<String>,
}

// ── Trade Flows subtab (per-settlement realized-trade breakdown) ──────────────
/// One traded good at a settlement: its average + last-year volume, how many
/// partner routes carried it, and its yearly volume series (the trend graph).
#[derive(Serialize, Clone)]
pub struct TradeFlowGood {
    pub good: u32,
    pub name: String,
    pub avg_volume: f32,
    pub last_volume: f32,
    pub in_volume: f32,
    pub out_volume: f32,
    pub route_count: u32,
    pub history: Vec<f32>,
}
/// One good's flow along one partner route (for the per-good route list + map).
#[derive(Serialize, Clone)]
pub struct TradeRouteFlow {
    pub good: u32,
    pub partner: u32,
    pub partner_name: String,
    pub px: f32,
    pub py: f32,
    pub dir: u8,        // 0 = inbound to this city, 1 = outbound
    pub amount: f32,
    pub pct: f32,       // share of this good's flow at this city
}
/// A top partner city: its share of ALL this city's trade + the goods exchanged.
#[derive(Serialize, Clone)]
pub struct TradePartner {
    pub hub: u32,
    pub name: String,
    pub px: f32,
    pub py: f32,
    pub volume: f32,
    pub pct: f32,
    pub goods: Vec<String>,
}
/// The settlement Flows subtab payload (last completed year + history).
#[derive(Serialize, Clone, Default)]
pub struct TradeFlows {
    pub hub: u32,
    pub hub_x: f32,
    pub hub_y: f32,
    pub goods: Vec<TradeFlowGood>,
    pub routes: Vec<TradeRouteFlow>,
    pub partners: Vec<TradePartner>,
}

/// One labelled money line in the Accountant view (a city's tax/profit, or a
/// warehouse good). Per-city lists are sorted largest → lowest.
#[derive(Serialize)]
pub struct LedgerLine {
    pub label: String,
    pub amount: f32,
}

/// A house/guild's yearly books for the Accountant tab (the last COMPLETED year).
#[derive(Serialize)]
pub struct HouseLedger {
    pub name: String,
    pub is_guild: bool,
    pub year: u32,
    // ── Income ──
    pub trade_profit: Vec<LedgerLine>, // per city, largest first
    pub office_income: f32,
    pub estate_income: f32,
    pub income_total: f32,
    // ── Expenditure ──
    pub import_tax: Vec<LedgerLine>,
    pub export_tax: Vec<LedgerLine>,
    pub estate_tax: f32,
    pub upkeep: f32,
    pub fleet_cost: f32,
    pub lost_cargo: f32,
    pub events: f32,
    pub consumption: f32,
    pub inflation: f32,
    /// §3.4e · forced war levy paid this year — split out of ordinary civic tax so
    /// a war's cost reads as its own line, per the plan's own "war must be legible
    /// as money" rule.
    #[serde(default)] pub war_levy: f32,
    /// §3.4e · wealth-equivalent loss when war damages one of this house's own
    /// estates/manufactories (the existing `damage` field, no new mechanism).
    #[serde(default)] pub war_damage: f32,
    pub expense_total: f32,
    pub net: f32,
    /// Monthly wealth samples through the year (for the Accountant's wealth graph).
    pub wealth_graph: Vec<f32>,
    /// YEARLY wealth, oldest→newest, last ~10 years (the multi-year growth graph).
    pub wealth_years: Vec<f32>,
    /// Campaign year of the FIRST `wealth_years` sample (for the graph's X axis).
    pub wealth_start_year: u32,
    // Warehouse stock held at the home city (what a fire/spoilage destroys).
    pub warehouse_city: String,
    pub warehouse: Vec<LedgerLine>,
}

#[derive(Serialize)]
pub struct HouseTimelineEvent {
    pub year: u32,        // campaign year (tick / 365)
    pub kind: String,     // founded | succession | monopoly | control_gained | control_lost | branch | loss | dissolved
    pub text: String,
}

/// A house's full chronicle for the timeline view.
#[derive(Serialize)]
pub struct HouseHistory {
    pub name: String,
    pub color: String,
    pub founder: String,       // founding head + circumstances
    pub founded_year: u32,
    pub events: Vec<HouseTimelineEvent>,
    pub top_goods: Vec<(String, f32)>, // most profitable resources (name + cumulative profit)
    pub defunct: bool,
    /// Colonies/outposts this house OWNS (outposts) or BACKED (joint-stock share).
    #[serde(default)] pub colonies: Vec<ColonySummary>,
    /// Phase 0.4/1.4 · the succession LINE — every head this house has had, oldest
    /// first, so the chronicle-first dossier can open on "who held it, and how did
    /// they do" rather than a balance sheet.
    #[serde(default)] pub line: Vec<HeadBrief>,
}

/// One head of a house, for the chronicle view. Mirrors `tick::HouseHead`.
#[derive(Serialize)]
pub struct HeadBrief {
    pub name: String,
    pub female: bool,
    pub generation: u32,
    pub since_year: u32,
    /// 0 while this head still lives — the dossier reads that as "present".
    pub until_year: u32,
    pub age_at_accession: u32,
    pub age_at_death: u32,
    pub wealth_start: f32,
    pub wealth_end: f32,
    pub accession: String,
    pub epithet: String,
}

/// One city's live cost-of-living basket index (#30 Economy Dashboard · Prices).
#[derive(serde::Serialize)]
pub struct CityPriceIndex {
    pub name: String,
    /// Need-tier-weighted mean of price ÷ base_value across goods, ×100 (100 = world
    /// standard). Leans on staples (tier 0) over luxuries (tier 2).
    pub index: f32,
}

/// Migrate pre-split keys living in `metadata` into the campaign table (runs on
/// every world open; a no-op for already-split worlds). Returns true when
/// anything was migrated (i.e. the file was a legacy single-file save).
pub fn migrate_legacy_campaign_keys(conn: &Connection) -> Result<bool, String> {
    let mut migrated = false;
    for key in ["settlements", "economy"] {
        if let Some(v) = metadata::get_meta(conn, key).map_err(|e| e.to_string())? {
            // Don't clobber campaign data that's already present.
            if metadata::campaign_get(conn, key).map_err(|e| e.to_string())?.is_none() && !v.is_empty() {
                metadata::campaign_set(conn, key, &v).map_err(|e| e.to_string())?;
                migrated = true;
            }
            conn.execute("DELETE FROM metadata WHERE key = ?1", rusqlite::params![key])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(migrated)
}

// ── Command submodules (split out; re-exported so external paths are unchanged) ──
mod lifecycle;
mod read_people;
mod read_colonies;
mod read_money;
mod read_trade;
mod read_hubs;
mod read_houses;
mod province;
pub use lifecycle::*;
pub use read_people::*;
pub use read_colonies::*;
pub use read_money::*;
pub use read_trade::*;
pub use read_hubs::*;
pub use read_houses::*;
pub use province::*;
