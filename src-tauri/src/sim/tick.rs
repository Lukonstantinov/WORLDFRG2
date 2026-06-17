//! DLC 1 "Living Trade" — the per-tick campaign simulation engine.
//!
//! A `CampaignSim` is seeded once at campaign start (from the static economy
//! snapshot: hubs, per-good production, goods spec, connectivity) and then
//! advanced one **day** at a time. Each tick runs, in order:
//!
//! 1. production    — stock += base_production · seasonal multiplier
//! 2. consumption   — needs ladder (basic→comfort→luxury) w/ category substitution
//! 3. price         — smoothed `base·(need/stock)^k` in the grain-eq numeraire
//! 4. merchant dispatch — arbitrage surplus→deficit, emitting in-transit cargo
//! 5. arrivals      — cargo whose ETA has passed lands as stock
//! 6. events        — weighted/triggered shocks to stock/production/population
//! 7. estates & starvation — food balance → found estates / population decline
//! 8. houses        — profits, monopoly drift, feuds, periodic succession
//! 9. journal       — sparse price samples + every event (the graph/log source)
//!
//! Pure & deterministic per `(seed, tick)` — no DB, no global RNG, no tile
//! access (the perf rule: a tick is hub-level math only). The route-days matrix
//! is DERIVED from hub positions + connectivity and rebuilt on load, so it is
//! not part of the serialized blob.

use serde::{Deserialize, Serialize};

pub const TICKS_PER_YEAR: u32 = 365;
pub const SEASONS: [&str; 4] = ["Spring", "Summer", "Autumn", "Winter"];

const EPS: f32 = 1e-4;
const TIER_WEIGHT: [f32; 3] = [1.0, 0.45, 0.22];
const PRICE_FLOOR_MULT: f32 = 0.15;
const PRICE_CEIL_MULT: f32 = 12.0;
/// Per-capita appetite scale; multiplied by the seed-time balance factor so total
/// need is comparable to total production (an average good ~ slight shortage).
pub const DEMAND_PRESSURE: f32 = 1.15;
/// A house with wealth below this and no trade dissolves (bankruptcy).
const HOUSE_BANKRUPT: f32 = 0.15;
/// Export reserves: a hub only ships stock ABOVE a kept reserve. Ordinary goods
/// keep a thin day's buffer, but FOOD keeps a granary — roughly this many days of
/// local consumption — so the autumn harvest is held back to feed the city through
/// winter, droughts and lean years instead of being traded away (which otherwise
/// strips the seasonal buffer and triggers a famine death-spiral).
const TRADE_RESERVE_MULT: f32 = 1.1;
const FOOD_RESERVE_DAYS: f32 = 45.0;
/// Lower bound on a hub's production multiplier from STACKED active events. Stops
/// overlapping regional shocks from multiplying output down to near zero.
const EVENT_PROD_FLOOR: f32 = 0.5;
/// A house this wealthy may split a cadet branch into another city on succession.
const HOUSE_BRANCH_WEALTH: f32 = 12.0;

// ── Merchant fleets & voyage risk ────────────────────────────────────────────
/// Per-voyage chance a shipment is lost: storms at sea, ambush on the road,
/// wreck on the river. River boats are the safest, the open sea the riskiest.
const SEA_LOSS: f32 = 0.05;
const CARAVAN_LOSS: f32 = 0.03;
const RIVER_LOSS: f32 = 0.015;
/// Independent trade shorter than this (travel-days) is "local merchants";
/// anything longer is organized "guild" long-haul. Splits the non-house carry.
const LOCAL_HAUL_DAYS: f32 = 8.0;
/// Share of a settlement's population engaged in merchant trade — split across
/// houses / local merchants / guilds by their recent throughput at the hub.
const MERCHANT_POP_FRACTION: f32 = 0.12;
/// Global productivity drift: technology/agronomy improve output ~1.5%/yr,
/// applied as COMPOUND growth — each year multiplies the running index by
/// (1 + rate), so year 1 = ×1.015, year 2 = ×1.015² ≈ ×1.030225, etc. Event
/// setbacks below are also multiplicative (compound decline off the live index).
const PROD_GROWTH_PER_YEAR: f32 = 0.015;
/// One-time global production setbacks applied when adverse events fire. These are
/// a SLIGHT, recoverable nudge to the world production index — the real impact of a
/// drought/plague is the LOCAL, temporary hit in `event_production_mult`. They must
/// stay far below the +1.5%/yr growth drift: with ~30 events/yr a 1% setback each
/// would erode the index ~35%/yr and spiral the whole world into permanent famine
/// (the 8M→1M collapse). Kept ~10× smaller so productivity still trends upward.
const PROD_EVENT_SETBACK: f32 = 0.001;
const PROD_FIRE_SETBACK: f32 = 0.006;
/// The global production index can never fall below this fraction — insurance
/// against any future event storm ratcheting the world into a death-spiral.
const TECH_FACTOR_FLOOR: f32 = 0.85;
/// Share of an estate's gross export sales that flows to its OWNER (a house, or the
/// parent city). The estate's per-capita output is its scale; this is the rent.
const ESTATE_OWNER_CUT: f32 = 0.5;
/// A resident house this wealthy (or richer) takes ownership of a new estate its
/// city founds; below it, the city owns the estate.
const ESTATE_HOUSE_OWNER_WEALTH: f32 = 6.0;
/// A house must be at least this wealthy to lead a colonization of new land.
const COLONIZE_HOUSE_WEALTH: f32 = 18.0;
/// A house with at least this wealth invests its surplus capital into building an
/// estate (raw production) or a manufactory (a luxury good) — the main wealth sink
/// that turns hoarded profit into expansion and more production.
const INVEST_WEALTH: f32 = 12.0;
/// Base capital to found an estate/manufactory, scaled by the host city's size and
/// discounted where the house already holds an office.
const INVEST_COST_BASE: f32 = 5.0;
/// A house won't keep building once it owns this many estates/manufactories.
const MAX_HOUSE_ESTATES: usize = 6;
/// A single settlement won't host more than this many estates/manufactories, so a
/// city's hinterland isn't overrun (houses AND guilds both build there).
const MAX_ESTATES_PER_CITY: usize = 4;
/// Per-capita output rate of a house-built manufactory's luxury good.
const MANUFACTORY_PERCAP: f32 = 0.2;
/// Derived manufacturing demand: per unit of a city's labour capacity, how much
/// raw INPUT stock it wants buffered so its workshops can keep producing. This is
/// what pulls wool/iron/sugar into the weaving/forge/refining cities so the
/// finished goods actually accumulate in their warehouses.
const MANUFACTURE_PULL: f32 = 12.0;
/// Each estate/manufactory upgrade tier multiplies its output by this (5 tiers).
const ESTATE_UPGRADE_MULT: f32 = 1.4;

// ── Merchant fleets & voyage risk ────────────────────────────────────────────
/// A settlement gets a civic Merchant Guild once it reaches this population.
const GUILD_MIN_POP: f32 = 50_000.0;
/// Monthly civic subsidy into a guild's treasury, per 1,000 home-city people
/// (scaled by the city's prosperity). The city funds its guild.
const GUILD_SUBSIDY_PER_1K: f32 = 0.02;
/// Small absolute floor of accumulated trade volume before a holder will open an
/// office in a non-home city. The real trigger is RELATIVE (a top trade partner —
/// see `update_guilds_and_offices`); this floor just stops a barely-trading holder
/// from opening offices. Kept low so offices actually emerge at real trade scales.
const OFFICE_OPEN_VOLUME: f32 = 4.0;
/// Below this tie volume an existing office is abandoned (hysteresis vs OPEN).
const OFFICE_CLOSE_VOLUME: f32 = 0.5;
/// Base wealth cost to open an office, SCALED UP by the host city's importance
/// (population) — a counting-house in a great hub costs more.
const OFFICE_COST_BASE: f32 = 4.0;
/// Standing discount on goods a holder BUYS in a city where it has an office.
const OFFICE_BUY_DISCOUNT: f32 = 0.05;
// ── Office leases (Phase 5: trade network reach). A house signing a futures contract
//    LEASES the cities at both ends for a guaranteed term, so its bases stay put for
//    the life of the contract: a leased office never auto-closes, pays the city a rent
//    each month, and lapses when the term ends. The chain of leased offices is the
//    house's NETWORK — a distant settlement can contract for a good the house sources
//    at ANY of its nodes, and goods moving between the house's OWN cities pay reduced
//    tolls (it passes its own gates). ──
const OFFICE_LEASE_YEARS: u32 = 10;
const OFFICE_LEASE_FEE: f32 = 6.0;   // upfront, scaled by host city size
const OFFICE_LEASE_RENT: f32 = 0.05; // monthly, scaled by host city size
const NETWORK_TOLL_DISCOUNT: f32 = 0.5; // tax multiplier when both ends are own nodes
/// Total source-buy discount is capped here (office + glut bargain).
const MAX_BUY_DISCOUNT: f32 = 0.30;

// ── House archetypes ────────────────────────────────────────────────────────
const ARCH_SPECIALTY: u8 = 0; // cheaper freight + fatter margin on specialty goods
const ARCH_FLEET: u8 = 1;     // safer voyages, cheaper ships, longer reach
const ARCH_BANKING: u8 = 2;   // wealth earns interest; can trade on credit
const ARCH_POLITICAL: u8 = 3; // more political power; wins city charters
/// Pick a deterministic archetype for a new house from the founding context.
pub fn pick_archetype(seed: u64, salt: u64) -> u8 {
    (hash01(seed, salt ^ 0xA3C7, 0x5151) * 4.0) as u8 % 4
}
pub fn archetype_label(a: u8) -> &'static str {
    match a {
        ARCH_SPECIALTY => "Specialist traders",
        ARCH_FLEET => "Shipping dynasty",
        ARCH_BANKING => "Merchant bankers",
        ARCH_POLITICAL => "Political house",
        _ => "Merchant house",
    }
}
/// One-line description of an archetype's standing bonus (for the Houses panel).
pub fn archetype_perk(a: u8) -> &'static str {
    match a {
        ARCH_SPECIALTY => "+25% profit on specialty goods",
        ARCH_FLEET => "safer voyages · cheaper ships",
        ARCH_BANKING => "trades on credit · wealth earns interest",
        ARCH_POLITICAL => "more power · wins city charters",
        _ => "",
    }
}
const SPECIALTY_MARGIN: f32 = 1.25; // extra profit on specialty goods
const CHARTER_RENT: f32 = 1.30;     // extra profit on chartered goods
const BANK_CREDIT_MULT: f32 = 1.6;  // financing reach beyond cash
const BANK_INTEREST: f32 = 0.01;    // monthly interest on wealth

// ── Phase G: wealth sinks (monthly, multiplicative → wealth PLATEAUS instead of
//    compounding forever). Money is hard-won and steadily bleeds, the way a
//    medieval merchant fortune did. Sinks are a fraction of wealth, so a poor
//    house barely pays them and a rich one bleeds a lot (a stabilizing feedback). ──
/// Monthly WAREHOUSE upkeep — a SIZE-based maintenance cost paid EVERY month
/// regardless of wealth or whether the house trades at all. A house keeps a
/// warehouse at home, a counting-house in each foreign office, and a depot at
/// each estate it owns; each costs this much, scaled by the size of the city it
/// sits in (`city_size_factor`). This is the floor that pushes an idle or
/// over-extended house into DEBT — and, if unpaid for a year, into bankruptcy.
/// (Fleet upkeep is still charged separately in `manage_fleets`.)
const UPKEEP_WAREHOUSE_BASE: f32 = 0.30;
/// An estate depot is cheaper to keep than a city warehouse (small rural store).
const UPKEEP_ESTATE_FRAC: f32 = 0.5;
// ── House-warehouse capacity tiers (single TOTAL capacity bands). A warehouse's
//    tier is derived from its capacity by `capacity_tier`: Depot / Storehouse /
//    Warehouse / Entrepôt / Grand Entrepôt. AI expansion raises capacity (promote);
//    damage lowers it below a floor (demote). Used from Phase 2 on; defined now so
//    the scaffolding (struct + accessor) is self-contained. ──
const WH_TIER1_CAP: f32 = 600.0;    // Depot (wood)
const WH_TIER2_CAP: f32 = 1_500.0;  // Storehouse (wood)
const WH_TIER3_CAP: f32 = 3_000.0;  // Warehouse (timber+tile)
const WH_TIER4_CAP: f32 = 6_000.0;  // Entrepôt (stone)
const WH_MAX_CAP: f32 = 12_000.0;   // Tier 5 Grand Entrepôt ceiling
// ── Warehouse economics (Phase 2). Capacity-scaled upkeep makes a big hoard in one
//    city expensive (pushing a house to spread out via offices); a wealth-
//    proportional family overhead caps cash-hoarding and pushes capital into trade /
//    depots / contracts. A new depot starts at Tier 1 and an AI house enlarges it
//    when it stays full. ──
const CAP_UPKEEP: f32 = 0.001;         // monthly upkeep per unit capacity (× city size)
const WEALTH_UPKEEP_RATE: f32 = 0.02;  // monthly overhead on wealth above the allowance
const WEALTH_UPKEEP_FREE: f32 = 30.0;  // wealth free of the family overhead
const WH_START_CAP: f32 = 600.0;       // a fresh depot starts a Tier-1 store
const WH_EXPAND_MULT: f32 = 1.6;       // capacity grows ×this per expansion
const WH_EXPAND_COST: f32 = 6.0;       // base wealth cost to enlarge (× current tier)
const WH_FULL_FRAC: f32 = 0.85;        // enlarge once fill ≥ this fraction
const WH_STOCK_FRAC: f32 = 0.25;       // share of a good's local surplus a house stocks/mo
const WEALTH_HISTORY_CAP: usize = 80;  // years of wealth samples kept per house
const HOUSE_EVENTS_CAP: usize = 60;    // most-recent chronicle entries kept per house
// ── Futures contracts (Phase 3). A contract is a thin, two-sided stability layer
//    ON TOP of the spot market: it covers only a slice of a city's need (so the
//    price signal survives), at a struck price allowed to drift within a band, for
//    a term gated by the seller's record of stable growth. ──
const CONTRACT_COVERAGE_CAP: f32 = 0.25; // max share of a city's need under contract per good
const CONTRACT_PRICE_BAND: f32 = 0.12;   // paid price drifts ≤ ±this around the strike
const CONTRACT_DELIVER_DAYS: u32 = 30;   // a delivery every ~month
const CONTRACT_FORM_CHANCE: f32 = 0.10;  // monthly chance an eligible house offers one
const MAX_CONTRACTS: usize = 400;        // global cap (bounds the per-tick fulfil loop)
// Term → strike factor (longer term = cheaper unit for the buyer) and break penalty
// multiplier (longer = stiffer). Indexed 0:1yr 1:3yr 2:5yr 3:7yr.
const TERM_YEARS: [u8; 4] = [1, 3, 5, 7];
const TERM_STRIKE_FACTOR: [f32; 4] = [1.02, 1.00, 0.97, 0.95];
const TERM_PENALTY_MULT: [f32; 4] = [0.5, 1.0, 1.6, 2.4];
// Per-vessel cargo capacity, by mode. A big contract fans out across MANY vessels
// (e.g. 12 ships); each rolls its own storm/ambush loss, so a delivery can arrive
// PARTIALLY (10 of 12 ships make port). Ships carry the most (the viable bulk
// carrier); river boats less; caravans least. Sea vessels and land vessels (boats +
// caravans, pooled) are tracked separately so a MIXED coast↔inland route must
// reserve BOTH a sea leg and a land leg. `LAND_CAPACITY` blends boat/caravan size.
const SHIP_CAPACITY: f32 = 120.0;   // sea — the viable bulk carrier
const BOAT_CAPACITY: f32 = 70.0;    // river boat — mid
const CARAVAN_CAPACITY: f32 = 40.0; // overland caravan — least
/// Conspicuous consumption (feasts, weddings, charity, building): spent INTO the
/// home city, lifting its people's prosperity — the main "wealth reaches people" lever.
const HOUSE_CONSUMPTION_RATE: f32 = 0.004;
/// Guilds are civic — they spend more of their wealth on their own citizens.
const GUILD_CIVIC_RATE: f32 = 0.008;
/// Soft wealth ceiling for a guild in a baseline (~30k) city, scaled by
/// `city_size_factor`. Beyond it a guild is pressed to endow its city; the drain
/// grows with the overshoot (see `apply_wealth_sinks`), so guild fortunes
/// PLATEAU instead of climbing forever and the surplus flows to the settlement.
const GUILD_WEALTH_SOFTCAP: f32 = 200.0;
/// Maximum fraction of the over-cap wealth a guild endows in one month (reached
/// when it holds roughly twice its cap).
const GUILD_ENDOW_MAX: f32 = 0.5;
/// How fast a hub's accumulated civic spending (the `civic_pool`) is used up.
const CIVIC_DECAY: f32 = 0.97;
/// Monthly fleet upkeep as a fraction of a vessel's value (crew, repairs, berthing)
/// — a steady sink that scales with how big a fleet the house runs.
const FLEET_UPKEEP_FRAC: f32 = 0.035;
/// Per-vessel monthly chance of being lost to wear (rot, storms, breakdown) — the
/// slow decay of ships & caravans, so fleets must be continually replaced.
const FLEET_DECAY_CHANCE: f32 = 0.012;
/// Civic taxes a city levies on a house's trade — export on goods leaving the
/// origin, import on goods arriving at the destination. Paid by the house, funding
/// the city (into its civic_pool → people). Guilds pay HEAVIER taxes (civic duty).
const EXPORT_TAX_RATE: f32 = 0.04;
const IMPORT_TAX_RATE: f32 = 0.05;
const GUILD_TAX_MULT: f32 = 2.0;
/// Guild trade taxes are PROGRESSIVE in trade volume: a dominant guild moving a
/// great deal of cargo pays proportionally more on every shipment than a small
/// one. The extra multiplier ramps with the guild's recent decaying `volume`
/// toward `GUILD_TAX_VOLUME_REF`, capped by `GUILD_TAX_PROGRESSIVE`. This is the
/// "tax proportional to trade amount" lever that bleeds big guilds into cities.
const GUILD_TAX_VOLUME_REF: f32 = 2000.0;
const GUILD_TAX_PROGRESSIVE: f32 = 3.0;
/// Per-city tax BRACKET: a city's trade-tax rate rises with its own prosperity, so
/// rich entrepôts tax harder while poorer hubs stay cheap to trade through — a
/// soft pressure that spreads trade & growth toward the have-nots. A city at full
/// prosperity taxes (1 + CITY_TAX_BRACKET)× the base rate. See `city_tax_factor`.
const CITY_TAX_BRACKET: f32 = 1.0;
/// Tax a city takes on an estate's rent paid to its owning house.
const ESTATE_TAX_RATE: f32 = 0.10;
/// Yearly inflation — coin debasement + rising prices steadily eat the real value
/// of a hoarded fortune (applied once a year to every house's wealth).
const INFLATION_PER_YEAR: f32 = 0.015;
/// PUBLIC WORKS: a city whose civic treasury (`civic_pool`, fed by trade taxes,
/// guild dues and endowments) has grown past a per-capita threshold spends it on
/// the common good — erecting a useful building outright, or, once well-built,
/// throwing a festival that lifts the people's prosperity & stability. This is the
/// visible payoff that turns the guild-endowment sink into something the city
/// (and player) feels. `PUBLIC_WORKS_PC` is the civic-per-capita trigger (same
/// scale as the `civic_pc` used for prosperity); costs scale with city size.
const PUBLIC_WORKS_PC: f32 = 1.5;
const PUBLIC_WORKS_BUILD_COST: f32 = 6.0; // × city_size_factor, to erect a structure
const FESTIVAL_COST: f32 = 1.5;           // × city_size_factor — feasts are cheap relief
const FESTIVAL_PROSPERITY: f32 = 0.18;    // one-off bump to sent_prosperity
const FESTIVAL_STABILITY: f32 = 0.12;     // one-off bump to sent_stability
const FLEET_LOSS_MULT: f32 = 0.6;   // voyage-loss reduction
const FLEET_SHIP_DISCOUNT: f32 = 0.8;
const POLITICAL_POWER_BONUS: f32 = 0.15;

/// Estate-kind id for a produced good, from its name + food flag (0 = unsuited).
/// 1 farm · 2 mine · 3 plantation · 4 fishery · 5 vineyard.
fn estate_kind_for_good(name: &str, food: bool) -> u8 {
    let n = name.to_ascii_lowercase();
    if n.contains("wine") || n.contains("grape") { return 5; }
    if n.contains("fish") || n.contains("pearl") || n.contains("whal")
        || n.contains("herring") || n.contains("stockfish") { return 4; }
    if n.contains("iron") || n.contains("gem") || n.contains("salt") || n.contains("amber")
        || n.contains("ore") || n.contains("stone") || n.contains("copper") || n.contains("silver")
        || n.contains("gold") || n.contains("coal") { return 2; }
    if food { return 1; }
    // Any other cultivated trade good (silk, spices, cotton, sugar, tea, coffee, …).
    3
}

/// Short label for an estate kind (for journal text + the inspector).
fn estate_kind_label(kind: u8) -> &'static str {
    match kind {
        1 => "Farm", 2 => "Mine", 3 => "Plantation", 4 => "Fishery", 5 => "Vineyard",
        6 => "Manufactory", _ => "Estate",
    }
}

// ── Structures (per-settlement buildings) ───────────────────────────────────
const STRUCT_GRANARY: u8 = 1;
const STRUCT_WAREHOUSE: u8 = 2;
const STRUCT_SHIPYARD: u8 = 3;
const STRUCT_GUILDHALL: u8 = 4;
const STRUCT_WORKSHOP: u8 = 5;
/// A hub needs at least this commercial prosperity to fund a new building.
const STRUCT_BUILD_WEALTH: f32 = 0.5;
/// Production multipliers granted by structures.
const WORKSHOP_PROD: f32 = 1.12;   // all goods
const WAREHOUSE_PROD: f32 = 1.05;  // all goods (supply smoothing)
const GRANARY_FOOD_PROD: f32 = 1.12; // food goods only
/// Guildhall lowers freight on trades leaving its hub.
const GUILDHALL_FREIGHT: f32 = 0.85;

pub fn structure_label(id: u8) -> &'static str {
    match id {
        STRUCT_GRANARY => "Granary",
        STRUCT_WAREHOUSE => "Warehouse",
        STRUCT_SHIPYARD => "Shipyard",
        STRUCT_GUILDHALL => "Guildhall",
        STRUCT_WORKSHOP => "Workshop",
        _ => "Building",
    }
}

/// One-line effect description for a structure (for the inspector).
pub fn structure_effect(id: u8) -> &'static str {
    match id {
        STRUCT_GRANARY => "+12% food output",
        STRUCT_WAREHOUSE => "+5% output (storage)",
        STRUCT_SHIPYARD => "+1 sea ship for the resident house",
        STRUCT_GUILDHALL => "−15% freight on exports",
        STRUCT_WORKSHOP => "+12% production",
        _ => "",
    }
}
/// Wealth cost to build a new transport asset (sea ship / river boat / caravan).
/// Dispatch only considers each seller's nearest few markets as targets (it then
/// keeps the 3 hungriest of those). Scanning all `n` hubs per seller was the
/// dominant late-campaign cost as estates inflated `n`; capping to the nearest K
/// keeps a Month step roughly flat regardless of total hub count.
const NEIGHBOR_K: usize = 32;
/// Global ceiling on satellite production sites (estates + colonies). Estates are
/// real hubs in `self.hubs`, so an uncapped count quadratically slows every tick.
const MAX_TOTAL_ESTATES: usize = 220;

const SHIP_COST: f32 = 5.0;
const RIVER_COST: f32 = 3.5;
const CARAVAN_COST: f32 = 3.0;

/// One tradable good in the tick economy (mapped from the world's `GoodSpec`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickGood {
    pub name: String,
    /// Substitution group; alternatives in the same group satisfy one need.
    /// `i32::MAX` = no group.
    pub category: i32,
    /// 0 basic, 1 comfort, 2 luxury.
    pub need_tier: u8,
    /// World-standard value, grain-equivalent (wheat = 1.0).
    pub base_value: f32,
    pub desire: f32,
    /// Counts toward a hub's food balance (cereal/protein/oil/sweetener).
    pub food: bool,
    /// This good's category is a FUNGIBLE recipe input — members of its
    /// `category` group stand in for each other as an ingredient (e.g. bay salt ↔
    /// rock salt curing fish). Narrow: NOT set for metals/fibres. Serde-default
    /// false so old campaign saves load.
    #[serde(default)] pub fungible_input: bool,
    /// Freight weight/volume multiplier (1.0 = silk; 3-4 = bulky staple). Old
    /// saves predate this → `serde(default)` gives 0.0, treated as 1.0 in freight.
    #[serde(default)] pub bulk: f32,
    /// Extra freight per travel-day from spoilage (additive). 0 = durable.
    #[serde(default)] pub perishable: f32,
    /// Recipe inputs (column index, qty) for a Manufactured good — made in cities
    /// from these raws, not produced per-capita. Empty = extracted/raw.
    #[serde(default)] pub inputs: Vec<(usize, f32)>,
    /// Labor output-rate factor for manufacture (∝ population × this).
    #[serde(default)] pub labor: f32,
    /// Demand cadence in days (how often a person consumes a unit). Long cadence →
    /// weak daily local demand → the good sits cheaper and is mostly traded
    /// merchant-to-merchant. 0 on old saves → treated as the neutral 30 in base_need.
    #[serde(default)] pub consumption_interval: f32,
}

/// One settlement participating in the living economy.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickHub {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub population: f32,
    pub founding_pop: f32,
    pub stock: Vec<f32>,
    pub price: Vec<f32>,
    pub production: Vec<f32>,
    pub grain_wealth: f32,
    pub trade_wealth: f32,
    /// Food produced+imported − population need this tick (per capita, smoothed).
    pub food_balance: f32,
    /// 0 = fed, 1 = severe sustained food deficit.
    pub starving: f32,
    pub is_estate: bool,
    /// Parent hub INDEX for an estate (−1 otherwise).
    pub parent: i32,
    pub koppen: u8,
    pub coastal: bool,
    /// Connectivity component — goods move only within a component.
    pub component: u32,
    /// Cumulative export earnings (grain-eq) — for trade wealth & houses.
    pub export_earn: f32,
    pub import_spend: f32,
    // ── Population sentiment (Phase 4) — 0..1, eased toward a target each tick. ──
    /// Overall mood (0 = unrest, 1 = content): a blend of the three drivers below.
    #[serde(default)] pub mood: f32,
    /// Food security (1 = well fed, → 0 as starvation builds).
    #[serde(default)] pub sent_food: f32,
    /// Commercial prosperity (from grain + trade wealth).
    #[serde(default)] pub sent_prosperity: f32,
    /// Stability — freedom from recent disasters and market dearth.
    #[serde(default)] pub sent_stability: f32,
    /// Phase G: accumulated merchant-house spending (conspicuous consumption +
    /// civic tax) circulating in this city. Decays as it is used; feeds the
    /// populace's prosperity — this is how trade wealth REACHES the people.
    #[serde(default)] pub civic_pool: f32,
    /// Sparse per-hub time series for the settlement-window History charts.
    #[serde(default)] pub history: Vec<HubSample>,
    /// Recent goods arriving by SEA (ships) vs by LAND (caravans), a decaying
    /// tally so the settlement view can show how this city is supplied.
    #[serde(default)] pub in_by_sea: f32,
    #[serde(default)] pub in_by_land: f32,
    /// Per-capita production at founding — production scales with LIVE population
    /// (`production[g] = base_per_capita[g] · population · tech · …`). Seeded from
    /// the static economy; back-filled once for pre-existing saves (see `advance`).
    #[serde(default)] pub base_per_capita: Vec<f32>,
    // ── Shortage: smoothed fraction of demand left UNMET this tick, by need tier
    //    (0 = fully supplied, 1 = nothing met). Drives the "% lacking goods" graph.
    #[serde(default)] pub lack_basic: f32,
    #[serde(default)] pub lack_comfort: f32,
    #[serde(default)] pub lack_luxury: f32,
    // ── Trade throughput touching this hub in the last while, split by who carried
    //    it (decaying tallies). Used to estimate the merchant population by class.
    #[serde(default)] pub tw_house: f32,
    #[serde(default)] pub tw_local: f32,
    #[serde(default)] pub tw_guild: f32,
    /// Estate type (0 none / 1 farm / 2 mine / 3 plantation / 4 fishery / 5 vineyard).
    /// Non-zero only when `is_estate`; drives its produced good + the inspector label.
    #[serde(default)] pub estate_kind: u8,
    /// Estate upgrade tier 1..5 — higher tiers produce more (owners invest to
    /// upgrade). 0 on non-estates / old saves (treated as tier 1 for estates).
    #[serde(default)] pub estate_tier: u8,
    /// Owning house index for an estate (−1 = owned by the parent city). Estate
    /// export income flows to this owner — a core engine of house growth.
    #[serde(default = "neg_one_i32")] pub owner_house: i32,
    /// Buildings this settlement has erected (ids: 1 Granary / 2 Warehouse /
    /// 3 Shipyard / 4 Guildhall / 5 Workshop). Each grants a standing bonus; at
    /// most one of each. Auto-built as a city/house prospers.
    #[serde(default)] pub structures: Vec<u8>,
    // ── DLC 3 · the Polis as an actor (set yearly by `decide_polis_policy`) ──
    /// Formal civic TREASURY — a persistent war-chest the council accumulates from
    /// a skim of the city's tax take. Unlike `civic_pool` (which decays back to the
    /// people each tick), the treasury is retained capital the polis can field.
    /// 0 on old saves / non-seat hubs.
    #[serde(default)] pub treasury: f32,
    /// Council-set EXPORT tariff on goods leaving this polis (fraction of value).
    /// 0 = no council policy yet → the global `EXPORT_TAX_RATE` default applies.
    #[serde(default)] pub tariff_export: f32,
    /// Council-set IMPORT tariff on goods arriving here. 0 → global `IMPORT_TAX_RATE`.
    #[serde(default)] pub tariff_import: f32,
    /// Mint FINENESS the council maintains: 1.0 = full-bodied coin, < 1.0 = debased
    /// ("cut the coin fine") which is how a council floods cheap money into its
    /// market. 0 on old saves → read as 1.0 (no debasement). Drives the speculation
    /// engine's "cheap money" bubble driver.
    #[serde(default)] pub mint_fineness: f32,
    /// House index of the family/faction whose council governs this polis (−1 = no
    /// dominant council). The decision-maker behind tariff / mint / charter policy.
    #[serde(default = "neg_one_i32")] pub council_house: i32,
}

/// Serde default for `owner_house` so old saves / non-estate hubs read −1, not 0
/// (which would point at house index 0).
fn neg_one_i32() -> i32 { -1 }

/// One sparse per-hub history sample (weekly) for the settlement-window charts.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HubSample {
    pub tick: u32,
    pub population: f32,
    pub wealth: f32,      // grain + trade wealth
    pub mood: f32,
    pub price_index: f32, // mean local price ÷ world-standard value
    // ── Shortage by tier (fraction of demand unmet) + merchant population by
    //    class, sampled monthly for the settlement-window charts. ──
    #[serde(default)] pub lack_basic: f32,
    #[serde(default)] pub lack_comfort: f32,
    #[serde(default)] pub lack_luxury: f32,
    #[serde(default)] pub pop_house: f32,
    #[serde(default)] pub pop_local: f32,
    #[serde(default)] pub pop_guild: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InTransit {
    pub from: u32,
    pub to: u32,
    pub good: usize,
    pub amount: f32,
    pub eta_tick: u32,
    /// Owning house index (−1 = independent local merchants / guilds).
    pub owner: i32,
    /// True = a sea voyage (occupies a sea-ship slot); false = overland.
    #[serde(default)] pub sea: bool,
    /// Round-trip phase: 0 = OUTBOUND (on arrival it may spawn a return leg that
    /// buys the destination's surplus and carries it home), 1 = RETURN / terminal.
    #[serde(default)] pub phase: u8,
    /// Round-trip origin hub the return leg sells at (−1 = a plain one-way trip
    /// that spawns no return). Only house-owned outbound voyages set this.
    #[serde(default = "neg_one_i32")] pub home: i32,
}

/// One recently completed trade (for the Market tab "recent deals" rows). A small
/// rolling log of dispatched shipments — captures the deal as it leaves, which
/// serves both the source's recent-departures and the destination's recent-arrivals.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentTrade {
    pub from: u32,
    pub to: u32,
    pub good: usize,
    pub amount: f32,
    pub owner: i32,
    pub sea: bool,
    pub price: f32,
    pub tick: u32,
}

/// One milestone in a house's chronicle (its timeline view).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HouseEvent {
    pub tick: u32,
    /// "founded" | "succession" | "monopoly" | "control_gained" | "control_lost"
    /// | "branch" | "loss" | "dissolved"
    pub kind: String,
    pub text: String,
}

/// A merchant family / trading house, with a named head of family who ages, dies
/// and is succeeded by an heir. Houses compete for trade, hold monopolies, feud
/// with rivals, and wield political power in their home city.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct House {
    pub name: String,  // "House Cassii"
    pub hub: u32,      // home hub index
    pub wealth: f32,
    pub prestige: f32,
    /// Goods this house specializes in.
    pub spec: Vec<usize>,
    /// good → monopoly share 0..1 (computed each month).
    pub monopoly: Vec<(usize, f32)>,
    /// House indices this house feuds with.
    pub rivals: Vec<usize>,
    pub generation: u32,
    // ── House chronicle (serde default → old saves still load) ──
    /// Milestone timeline for this family: founding, successions, new monopolies,
    /// control of cities gained/lost, branches, the worst loss. NOT pruned by the
    /// journal's 25-year window — these are the family's permanent record.
    #[serde(default)] pub events: Vec<HouseEvent>,
    /// Cumulative profit earned per good index — for "most profitable resources".
    #[serde(default)] pub good_profit: Vec<f32>,
    /// Goods the house currently HOLDS a monopoly on (hysteresis: entered at
    /// >=50% share, only released when share falls below ~10%). Prevents the
    /// "won a monopoly" spam from share oscillating around the 50% line.
    #[serde(default)] pub mono50: Vec<usize>,
    /// Goods the house has EVER held a monopoly on — so a re-win reads "regained"
    /// rather than "won", and the first win isn't repeated.
    #[serde(default)] pub mono_ever: Vec<usize>,
    /// True when the house holds >=50% of its seat city's trade (control state).
    #[serde(default)] pub dominant_seat: bool,
    /// Wealth at the previous monthly check + the worst single-month loss so far.
    #[serde(default)] pub prev_wealth: f32,
    #[serde(default)] pub worst_loss: f32,
    // ── Fleet: the house's transport capital. Each asset = ONE concurrent
    //    shipment slot on its route type (scale-independent). Sea ships serve
    //    coastal↔coastal voyages; river boats + caravans serve overland routes.
    //    Capital-constrained: houses buy them with wealth and can lose them to
    //    storms / ambush. Bigger fleet → more trade carried → more market share.
    #[serde(default)] pub fleet_sea: u32,
    #[serde(default)] pub fleet_river: u32,
    #[serde(default)] pub fleet_caravan: u32,
    // ── Named head of family (serde default → old saves still load) ──
    #[serde(default)] pub head_name: String,   // "Marcus Cassii"
    #[serde(default)] pub head_since: u32,      // tick the current head took over
    #[serde(default)] pub head_lifespan: u32,   // ticks until succession (≈ a lifetime)
    #[serde(default)] pub founded_tick: u32,
    /// Soft political power 0..1 (wealth + monopoly + prestige) — the great houses
    /// dominate their home city's council.
    #[serde(default)] pub political_power: f32,
    /// Recent trade volume (decaying) — health + monopoly basis.
    #[serde(default)] pub volume: f32,
    /// Defunct (bankrupt / died out): kept for the record, not active.
    #[serde(default)] pub defunct: bool,
    /// House archetype / specialization (0 trade-specialty · 1 fleet & logistics ·
    /// 2 banking & capital · 3 political & charters). Each grants standing bonuses.
    #[serde(default)] pub archetype: u8,
    /// Goods this house holds a CHARTER on at its seat (granted by the city it
    /// dominates) — extra monopoly rent + prestige. Political archetype only.
    #[serde(default)] pub charters: Vec<usize>,
    /// A civic MERCHANT GUILD rather than a private house: it acts in its home
    /// city's interest (fills the city's needs, secures supply), is funded by a
    /// civic subsidy, and never goes bankrupt. Same machinery (fleet, offices,
    /// trade) otherwise. Default false → old saves load as private houses.
    #[serde(default)] pub is_guild: bool,
    /// Host hubs where this holder has opened an OFFICE — a foreign foothold that
    /// gives −5% on goods it BUYS there and lets it originate trade from there.
    #[serde(default)] pub offices: Vec<u32>,
    /// Sparse decaying tally of recent trade VOLUME through each hub this holder
    /// touches `(hub, volume)`. Drives office opening (sustained tie) and closing
    /// (tie withers). Not all hubs — only ones traded through.
    #[serde(default)] pub trade_at: Vec<(u32, f32)>,
    /// Tick at which this house's balance first went NEGATIVE (0 = solvent). A
    /// private house insolvent for a full year is declared bankrupt; reset to 0
    /// the moment it claws back to a non-negative balance. Guilds get a civic
    /// bailout instead, so this stays 0 for them.
    #[serde(default)] pub debt_since: u32,
    /// Yearly wealth samples (most recent last, capped to `WEALTH_HISTORY_CAP`).
    /// Drives `stable_growth_years` → the futures-contract term a house may offer
    /// (only a long, unbroken record of growth unlocks 5- and 7-year contracts).
    #[serde(default)] pub wealth_history: Vec<f32>,
    /// LEASED offices `(hub, lease_end_tick)`: a durable base the house pays the
    /// city a monthly rent for. A leased office is never auto-closed before its term
    /// (offices backing a live contract are also held open) — the foundation of the
    /// house's standing trade NETWORK, which lets distant settlements contract for
    /// goods the house sources from any of its nodes.
    #[serde(default)] pub office_leases: Vec<(u32, u32)>,
}

/// A candidate empty-land site a wealthy house / large city can colonize with an
/// estate. Precomputed from world tiles at the Economy step (the tick sim has no
/// WorldBuffer) and carried into the campaign.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ColonizeSite {
    pub x: f32,
    pub y: f32,
    pub koppen: u8,
    pub elevation: f32,
    pub fertility: f32,
    pub coastal: bool,
    /// Suggested estate kind for the site (1 farm / 2 mine / 3 plantation /
    /// 4 fishery / 5 vineyard), from its climate / elevation / coast / fertility.
    pub kind_hint: u8,
}

/// A persisted shock currently modifying the world.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActiveEvent {
    pub kind: String,
    /// Affected hub index (−1 = world / region by position).
    pub hub: i32,
    pub good: i32,
    pub magnitude: f32,
    pub until_tick: u32,
}

/// One append-only history row (events + sparse price samples).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JournalEntry {
    pub tick: u32,
    pub kind: String, // "price" | "event" | "estate" | "starvation" | "succession"
    pub hub: i32,     // −1 = world
    pub good: i32,    // −1 = none
    pub value: f32,
    pub text: String,
}

/// DLC 3 · one ranked bubble DRIVER in a polis's speculation reason-chain. The UI
/// renders these largest-weight first as the generated causal "why".
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpecDriver {
    /// Stable key ("thin_float" | "cheap_money" | "leverage" | "dividend_surge" |
    /// "price_runup" | "supply_shock" | "hot_capital" | "political_shock" |
    /// "animal_spirits").
    pub key: String,
    /// Human label ("Thin float").
    pub label: String,
    /// Weighted contribution to the risk score (already coefficient-scaled, 0..1).
    pub weight: f32,
    /// Generated clause naming the real entities ("House Verani corners amber (87%)").
    pub detail: String,
}

/// DLC 3 · the once-a-year speculation read for one polis (mirrors `PoliticalCenter`).
/// Computed at the yearly hook, cached on the sim, surfaced as an overlay + panel
/// and (for HIGH tiers) a `JournalEntry{kind:"speculation"}`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpecCenter {
    pub hub: u32,
    pub x: f32,
    pub y: f32,
    pub name: String,
    /// Bubble risk 0..1.
    pub risk: f32,
    /// Tier stars 1..5 (≥4 = a mania watch).
    pub stars: u8,
    /// "LOW" | "MED" | "HIGH".
    pub tier: String,
    /// Narrative classification ("tulip-like" | "company-bubble" | "credit-fueled"
    /// | "speculative froth" | "calm").
    pub pattern_tag: String,
    /// Ranked drivers (largest weight first) — the causal reason-chain.
    pub drivers: Vec<SpecDriver>,
    /// The goods most exposed at this polis ("amber", …).
    pub watch_goods: Vec<String>,
    pub year: u32,
}

/// Phase G — one year of a merchant house's books (the Accountant view). All
/// amounts in grain-equivalent; the per-city Vecs are `(hub_index, amount)` and
/// are shown largest→lowest in the UI. Serde-default so old campaigns load empty.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LedgerAcc {
    pub year: u32,
    // ── Income ──
    pub trade_profit_by_city: Vec<(u32, f32)>,
    pub office_income: f32,
    pub estate_income: f32,
    // ── Expenditure ──
    pub import_tax_by_city: Vec<(u32, f32)>,
    pub export_tax_by_city: Vec<(u32, f32)>,
    pub estate_tax: f32,
    pub upkeep: f32,
    pub fleet_cost: f32,
    pub lost_cargo: f32,
    pub events: f32,
    pub consumption: f32,
    pub inflation: f32,
    /// Monthly wealth samples through the year — drives the Accountant's wealth graph.
    pub wealth_samples: Vec<f32>,
}

impl LedgerAcc {
    /// Add `amt` to the running total for `city` in a per-city accumulator.
    pub fn add_city(v: &mut Vec<(u32, f32)>, city: u32, amt: f32) {
        if let Some(e) = v.iter_mut().find(|e| e.0 == city) {
            e.1 += amt;
        } else {
            v.push((city, amt));
        }
    }
}

/// A merchant warehouse: a finite, OWNED store of goods sited in a city. Owner
/// `−1` is the "local merchants" pool (the city's open market — what used to be
/// the only inventory); a non-negative owner is a house/guild index. The aggregate
/// hub inventory that prices & needs read is `hub.stock` (the local-merchant pool,
/// stored inline on the hub) PLUS the stock of every house warehouse sited here —
/// see `CampaignSim::hub_stock`. In Phase 1 `warehouses` holds only house depots
/// (empty by default → behaviour identical to the pre-warehouse model). Futures
/// contracts (later phase) reserve and ship out of a specific warehouse's `stock`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Warehouse {
    /// House/guild index that owns this depot, or −1 = local merchants (city pool).
    pub owner: i32,
    /// Hub (city) this warehouse sits in.
    pub hub: u32,
    /// Single TOTAL capacity: Σ stock across goods may not exceed this. The −1 pool
    /// is effectively uncapped (capacity ignored for owner −1).
    pub capacity: f32,
    /// Per-good stock OWNED by this warehouse (length = goods count).
    pub stock: Vec<f32>,
    /// Capacity tier 1..5 (Depot/Storehouse/Warehouse/Entrepôt/Grand Entrepôt),
    /// derived from `capacity`. 0 = the uncapped local-merchant pool.
    #[serde(default)] pub tier: u8,
    /// Structural damage 0..1 (storms/fire/riots); repairs over time. 0 = sound.
    #[serde(default)] pub damage: f32,
}

/// A FUTURES CONTRACT: a seated house/guild guarantees a settlement a fixed
/// monthly quantity of one good, at a struck price (allowed to drift within a
/// band), for a 1/3/5/7-year term. The supply is reserved from the seller's
/// warehouse at `source_hub` BEFORE the spot market runs, so the buyer has forward
/// security the reactive market can't give. Longer terms are only offered by houses
/// with a long record of stable growth (`stable_growth_years`) and carry stiffer
/// break penalties. A plague quarantine SUSPENDS a contract (force majeure, no
/// penalty); a seller that simply can't deliver DEFAULTS (penalty to the buyer).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contract {
    pub seller_house: u32,
    pub buyer_hub: u32,
    /// Hub the goods are shipped FROM (the seller's source depot sits here).
    pub source_hub: u32,
    pub good: usize,
    /// Quantity delivered each month (a SMALL, coverage-capped slice of buyer need).
    pub monthly_qty: f32,
    /// Struck reference price at signing (grain-equivalent); paid price drifts within
    /// a band around it.
    pub strike_price: f32,
    pub term_years: u8,
    pub start_tick: u32,
    pub end_tick: u32,
    /// Running total delivered (for the UI / penalty scale).
    #[serde(default)] pub delivered: f32,
    /// Tick the last monthly delivery happened (paces deliveries).
    #[serde(default)] pub last_fulfilled: u32,
    /// While > current tick the contract is force-majeure suspended (plague lockup).
    #[serde(default)] pub suspended_until: u32,
    /// Count of seller defaults; at 3 the contract is voided.
    #[serde(default)] pub defaults: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CampaignSim {
    pub seed: u64,
    pub tick: u32,
    pub goods: Vec<TickGood>,
    pub hubs: Vec<TickHub>,
    pub in_transit: Vec<InTransit>,
    pub houses: Vec<House>,
    pub active_events: Vec<ActiveEvent>,
    pub journal: Vec<JournalEntry>,
    /// Travel-days per cell of straight-line distance (set from grid scale).
    pub days_per_cell: f32,
    pub freight_per_day: f32,
    pub k: f32,
    pub margin: f32,
    /// Seed-time balance factor making need comparable to production.
    pub need_scale: f32,
    /// World wrap width in cells (cylindrical X distance).
    pub world_w: f32,
    /// World height in cells — lets a hub's `y` map to a latitude (and so a
    /// hemisphere) for seasonal harvests. `serde(default)` → 0 on pre-existing
    /// saves, which `seasonal_mult` treats as "no hemisphere data" (a single
    /// global harvest season, the old behaviour).
    #[serde(default)]
    pub world_h: f32,
    pub last_tick_ms: f32,
    /// Total world population at the last monthly chronicle sample (for deltas).
    #[serde(default)]
    pub last_month_pop: f32,
    /// World price index at the last monthly chronicle sample (for deltas).
    #[serde(default)]
    pub last_month_index: f32,
    /// Number of merchant houses seeded at campaign start — the baseline the
    /// founding logic tries to keep the world stocked up to.
    #[serde(default)]
    pub seed_house_count: u32,
    /// One-time migration flag: pre-fleet saves get a starting fleet seeded once.
    #[serde(default)]
    pub fleets_migrated: bool,
    /// Global productivity multiplier — slow technological / agronomic improvement.
    /// Grows ~1.5%/yr (bumper events lift production further, locally + temporarily).
    /// `serde(default)` yields 0.0 on old saves → treated as 1.0 in `advance`.
    #[serde(default)]
    pub tech_factor: f32,
    /// One-time migration flag: derive `base_per_capita` for pre-existing saves whose
    /// hubs were seeded with absolute (population-independent) production.
    #[serde(default)]
    pub percap_migrated: bool,
    /// Phase G — per-house yearly ledgers (Accountant view), indexed to match
    /// `houses`. `house_ledger` = the running current year; `house_ledger_prev` =
    /// the last COMPLETED year (the one the Accountant displays). Resized to the
    /// house count each advance; serde-default → empty on old saves.
    #[serde(default)]
    pub house_ledger: Vec<LedgerAcc>,
    #[serde(default)]
    pub house_ledger_prev: Vec<LedgerAcc>,
    /// Phase G — trade wars: hubs each house is BARRED from trading at (a rival that
    /// dominates a city closes its market to a defeated competitor). The house must
    /// pay the city to regain access. Indexed to match `houses`. Guilds are immune.
    #[serde(default)]
    pub house_barred: Vec<Vec<u32>>,
    /// Empty-land sites a wealthy founder may colonize with an estate (precomputed at
    /// the Economy step). Consumed as colonies are founded; empty on old saves.
    #[serde(default)]
    pub colonizable: Vec<ColonizeSite>,
    // ── Diagnostics (last advance), for the trade analysis log ──
    #[serde(default)] pub diag_shipments: u32,   // shipments dispatched
    #[serde(default)] pub diag_by_house: u32,    // of those, financed by a house
    #[serde(default)] pub diag_by_guild: u32,    // carried by local merchants/guilds
    #[serde(default)] pub diag_lost: u32,        // voyages lost (storm/ambush)
    #[serde(default)] pub diag_volume: f32,      // total goods volume shipped
    /// Rolling log of recently dispatched trades (for the Market "recent deals").
    #[serde(default)] pub recent_trades: Vec<RecentTrade>,
    /// DLC 3 · cached speculation read, recomputed once per year at the yearly
    /// hook (`compute_speculation`). Empty until the first New Year of a campaign.
    #[serde(default)] pub spec_centers: Vec<SpecCenter>,
    /// The year `spec_centers` was computed for (so the UI can label it).
    #[serde(default)] pub spec_year: u32,
    /// Per-hub trade profit booked in the PREVIOUS year, kept so the speculation
    /// engine can read a year-on-year dividend surge. Indexed to match `hubs`.
    #[serde(default)] pub spec_prev_profit: Vec<f32>,
    /// Derived route-days matrix (n·n, f32::INFINITY = unreachable). Not
    /// serialized — rebuilt from positions + components after load.
    #[serde(skip)]
    pub days: Vec<f32>,
    /// Per-hub nearest reachable trade partners (hub indices), sorted nearest
    /// first, capped to `NEIGHBOR_K`. Dispatch only ever ships to the few
    /// hungriest of these, so scanning the full `n` per seller is pure waste —
    /// this keeps dispatch ~O(hubs·K) instead of O(hubs²) as estates accumulate.
    /// Not serialized — rebuilt alongside `days`.
    #[serde(skip)]
    pub neighbors: Vec<Vec<u32>>,
    /// Set when a hub is added/removed (estate/colony) so the route matrix +
    /// neighbour lists are rebuilt at most ONCE per tick instead of once per
    /// estate creation (the old per-estate `rebuild_routes` was O(n²) each).
    #[serde(skip)]
    pub routes_dirty: bool,
    /// House/guild-owned warehouses (the local-merchant `−1` pool stays inline on
    /// each hub's `stock`, so this holds only house depots). Appended LAST →
    /// `#[serde(default)]` makes old `.campaign` saves load with no house depots,
    /// i.e. the pre-warehouse behaviour. Goods owned here are still counted into the
    /// hub aggregate by `hub_stock`, so prices/needs/famine see the full inventory.
    #[serde(default)]
    pub warehouses: Vec<Warehouse>,
    /// Active futures contracts (house → settlement forward supply). Appended LAST,
    /// `#[serde(default)]` → old saves load with none.
    #[serde(default)]
    pub contracts: Vec<Contract>,
}

/// Deterministic 0..1 hash of three mixed inputs (splitmix64).
fn hash01(a: u64, b: u64, c: u64) -> f32 {
    let mut z = a
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add(b.wrapping_mul(0xBF58476D1CE4E5B9))
        .wrapping_add(c.wrapping_mul(0x94D049BB133111EB));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

impl CampaignSim {
    #[inline]
    pub fn n(&self) -> usize {
        self.hubs.len()
    }

    pub fn season(&self) -> &'static str {
        let m = (self.tick % TICKS_PER_YEAR) as f32 / TICKS_PER_YEAR as f32;
        SEASONS[((m * 4.0) as usize).min(3)]
    }

    pub fn year(&self) -> u32 {
        self.tick / TICKS_PER_YEAR
    }

    pub fn day_of_year(&self) -> u32 {
        self.tick % TICKS_PER_YEAR
    }

    /// DLC 3 · Phase 0 — the POLIS as an actor. Once a year each seat city's
    /// council (its dominant house) sets the coming year's tariff schedule and mint
    /// policy, and skims a slice of civic taxes into a retained treasury. These
    /// levers feed both the live sim (tariffs are charged on trade) and the
    /// speculation engine (a debased mint = cheap money). Conservative + additive:
    /// hubs with no dominant house keep the global default rates.
    fn decide_polis_policy(&mut self, _year: u32) {
        let n = self.hubs.len();
        // Dominant council house per hub: the richest non-guild house that holds
        // its seat (`dominant_seat`) and is homed there.
        let mut council: Vec<i32> = vec![-1; n];
        let mut council_wealth: Vec<f32> = vec![0.0; n];
        for (hi, h) in self.houses.iter().enumerate() {
            if h.defunct || h.is_guild { continue; }
            let hub = h.hub as usize;
            if hub >= n { continue; }
            if h.dominant_seat && h.wealth > council_wealth[hub] {
                council[hub] = hi as i32;
                council_wealth[hub] = h.wealth;
            }
        }
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            self.hubs[h].council_house = council[h];
            if self.hubs[h].mint_fineness <= 0.0 { self.hubs[h].mint_fineness = 1.0; }
            let arch = if council[h] >= 0 { self.houses[council[h] as usize].archetype } else { 255 };
            // Tariff stance by the council's character: political houses turn
            // protectionist; bankers/shippers keep trade cheap to move volume.
            let (exp, imp) = match arch {
                ARCH_POLITICAL => (EXPORT_TAX_RATE * 1.6, IMPORT_TAX_RATE * 1.6),
                ARCH_BANKING => (EXPORT_TAX_RATE * 0.8, IMPORT_TAX_RATE * 0.8),
                ARCH_FLEET => (EXPORT_TAX_RATE * 0.7, IMPORT_TAX_RATE * 0.9),
                ARCH_SPECIALTY => (EXPORT_TAX_RATE * 1.1, IMPORT_TAX_RATE * 1.1),
                _ => (EXPORT_TAX_RATE, IMPORT_TAX_RATE),
            };
            self.hubs[h].tariff_export = exp;
            self.hubs[h].tariff_import = imp;
            // Mint: a prosperous, banking-led council "cuts the coin fine" to lend
            // cheap (fineness eases down); others slowly restore full-bodied coin.
            let prosperous = self.hubs[h].trade_wealth > 0.5;
            let target = if arch == ARCH_BANKING && prosperous { 0.88 }
                else if prosperous { 0.96 } else { 1.0 };
            let f = self.hubs[h].mint_fineness;
            self.hubs[h].mint_fineness = f + (target - f) * 0.5;
            // Retained treasury: skim ~8% of the circulating civic pool.
            self.hubs[h].treasury += self.hubs[h].civic_pool * 0.08;
        }
    }

    /// DLC 3 · Phase 3 — the Speculation "Why-Engine". Once a year, score each
    /// polis's bubble risk from drivers that ALL already exist in the sim, build a
    /// ranked causal reason-chain naming the real houses/goods, classify the
    /// pattern, and journal the high-risk poleis. Deterministic; cached on the sim.
    fn compute_speculation(&mut self, year: u32) {
        let n = self.hubs.len();
        let tick = self.tick;
        // This year's trade profit booked at each city (from the just-closed books).
        let mut cur_profit = vec![0.0f32; n];
        for l in &self.house_ledger_prev {
            for (c, amt) in &l.trade_profit_by_city {
                if (*c as usize) < n { cur_profit[*c as usize] += *amt; }
            }
        }
        if self.spec_prev_profit.len() != n { self.spec_prev_profit = vec![0.0; n]; }

        // Weighted blend of normalized drivers (∑ coefficients ≈ 1).
        const W_FLOAT: f32 = 0.22; const W_MONEY: f32 = 0.16; const W_LEV: f32 = 0.12;
        const W_DIV: f32 = 0.14; const W_RUN: f32 = 0.14; const W_SHOCK: f32 = 0.08;
        const W_HOT: f32 = 0.05; const W_POL: f32 = 0.04; const W_SPIRIT: f32 = 0.05;

        let mut centers: Vec<SpecCenter> = Vec::new();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].population < 1.0 { continue; }

            // ── Thin float / corner — the largest monopoly held by a house homed
            //    here (or with an office here). ──
            let mut corner = 0.0f32; let mut corner_good = -1i32; let mut corner_house = String::new();
            for hh in &self.houses {
                if hh.defunct { continue; }
                let here = hh.hub as usize == h || hh.offices.contains(&(h as u32));
                if !here { continue; }
                for (g, share) in &hh.monopoly {
                    if *share > corner { corner = *share; corner_good = *g as i32; corner_house = hh.name.clone(); }
                }
            }

            // ── Cheap money — coin debasement at this polis + banking presence. ──
            let fineness = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            let debase = (1.0 - fineness).clamp(0.0, 1.0);
            let mut bank_seats = 0u32;
            for hh in &self.houses {
                if hh.defunct || hh.archetype != ARCH_BANKING { continue; }
                if hh.hub as usize == h || hh.offices.contains(&(h as u32)) { bank_seats += 1; }
            }
            let cheap_money = (debase / 0.12 * 0.6 + (bank_seats as f32 / 3.0) * 0.4).clamp(0.0, 1.0);
            // ── Leverage — banking credit multiplier scaled by the number of seats. ──
            let leverage = ((bank_seats as f32) * (BANK_CREDIT_MULT - 1.0) / 2.0).clamp(0.0, 1.0);

            // ── Dividend surge — YoY growth of trade profit booked at this city. ──
            let prev = self.spec_prev_profit[h];
            let div_growth = if prev > 1.0 { (cur_profit[h] - prev) / prev } else { 0.0 };
            let dividend = div_growth.clamp(0.0, 1.0);

            // ── Price run-up — dearest recent price sample vs world-standard value. ──
            let mut runup = 0.0f32; let mut runup_good = -1i32;
            for e in self.journal.iter().rev() {
                if e.tick + TICKS_PER_YEAR < tick { break; }
                if e.kind != "price" || e.hub != h as i32 || e.good < 0 { continue; }
                let base = self.goods.get(e.good as usize).map(|x| x.base_value).unwrap_or(1.0).max(1e-3);
                let ratio = (e.value / base - 1.0) / 2.0; // 3× base → 1.0
                if ratio > runup { runup = ratio.clamp(0.0, 1.0); runup_good = e.good; }
            }

            // ── Supply shock — an active embargo / drought / fishery collapse. ──
            let mut shock = 0.0f32; let mut shock_kind = String::new(); let mut shock_good = -1i32;
            for ev in &self.active_events {
                if ev.hub == h as i32 || ev.hub < 0 {
                    let s = (ev.magnitude.abs()).clamp(0.0, 1.0);
                    if s > shock { shock = s; shock_kind = ev.kind.clone(); shock_good = ev.good; }
                }
            }

            // ── Hot capital — foreign offices opened here (imported speculation). ──
            let mut foreign = 0u32;
            for hh in &self.houses {
                if hh.defunct { continue; }
                if hh.hub as usize != h && hh.offices.contains(&(h as u32)) { foreign += 1; }
            }
            let hot = (foreign as f32 / 4.0).clamp(0.0, 1.0);

            // ── Political shock — recent succession / control change at this seat. ──
            let mut pol = 0.0f32;
            for hh in &self.houses {
                for ev in hh.events.iter().rev() {
                    if ev.tick + TICKS_PER_YEAR < tick { break; }
                    let relevant = matches!(ev.kind.as_str(), "succession" | "control_gained" | "control_lost");
                    if relevant && (hh.hub as usize == h) { pol = pol.max(0.7); }
                }
            }

            // ── Animal spirits — the irrational deterministic residual. ──
            let spirits = hash01(self.seed, year as u64, h as u64);

            let drivers_raw = [
                ("thin_float", "Thin float", W_FLOAT * corner,
                    if corner_good >= 0 { format!("{} corners {} ({:.0}% share)", corner_house, self.goods[corner_good as usize].name, corner * 100.0) } else { String::new() }),
                ("cheap_money", "Cheap money", W_MONEY * cheap_money,
                    if debase > 0.01 { format!("council cut the coin fine ({:.0}% debased), {} banking seat(s)", debase * 100.0, bank_seats) } else if bank_seats > 0 { format!("{} banking seat(s) lending freely", bank_seats) } else { String::new() }),
                ("leverage", "Leverage", W_LEV * leverage,
                    if bank_seats > 0 { format!("borrowed money ({:.1}× credit) chasing assets", BANK_CREDIT_MULT) } else { String::new() }),
                ("dividend_surge", "Dividend surge", W_DIV * dividend,
                    if dividend > 0.05 { format!("trade profit up {:.0}% on the year", div_growth * 100.0) } else { String::new() }),
                ("price_runup", "Price run-up", W_RUN * runup,
                    if runup_good >= 0 { format!("{} trading well above its standard value", self.goods[runup_good as usize].name) } else { String::new() }),
                ("supply_shock", "Supply shock", W_SHOCK * shock,
                    if !shock_kind.is_empty() { let g = if shock_good >= 0 { format!(" on {}", self.goods[shock_good as usize].name) } else { String::new() }; format!("a {}{} is spiking prices", shock_kind, g) } else { String::new() }),
                ("hot_capital", "Hot capital", W_HOT * hot,
                    if foreign > 0 { format!("{} foreign house office(s) pouring capital in", foreign) } else { String::new() }),
                ("political_shock", "Political shock", W_POL * pol,
                    if pol > 0.0 { "a recent succession / regime change unsettles the seat".to_string() } else { String::new() }),
                ("animal_spirits", "Animal spirits", W_SPIRIT * spirits, "the irrational froth of the crowd".to_string()),
            ];

            let risk: f32 = drivers_raw.iter().map(|d| d.2).sum::<f32>().clamp(0.0, 1.0);
            // Skip near-silent poleis to keep the overlay legible.
            if risk < 0.15 { continue; }

            let mut drivers: Vec<SpecDriver> = drivers_raw.iter()
                .filter(|d| d.2 > 0.001 && !d.3.is_empty())
                .map(|d| SpecDriver { key: d.0.into(), label: d.1.into(), weight: d.2, detail: d.3.clone() })
                .collect();
            drivers.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

            let stars = if risk >= 0.8 { 5 } else if risk >= 0.65 { 4 } else if risk >= 0.5 { 3 } else if risk >= 0.35 { 2 } else { 1 };
            let tier = if risk >= 0.6 { "HIGH" } else if risk >= 0.4 { "MED" } else { "LOW" };
            // Pattern from the dominant driver.
            let pattern_tag = match drivers.first().map(|d| d.key.as_str()) {
                Some("thin_float") => "tulip-like",
                Some("dividend_surge") | Some("leverage") => "company-bubble",
                Some("cheap_money") => "credit-fueled",
                Some("supply_shock") => "shortage-driven",
                Some("animal_spirits") => "speculative froth",
                _ => "speculative froth",
            }.to_string();

            let mut watch_goods: Vec<String> = Vec::new();
            for g in [corner_good, runup_good, shock_good] {
                if g >= 0 { let nm = self.goods[g as usize].name.clone(); if !watch_goods.contains(&nm) { watch_goods.push(nm); } }
            }

            centers.push(SpecCenter {
                hub: self.hubs[h].id, x: self.hubs[h].x, y: self.hubs[h].y,
                name: self.hubs[h].name.clone(), risk, stars, tier: tier.into(),
                pattern_tag, drivers, watch_goods, year,
            });
        }

        centers.sort_by(|a, b| b.risk.partial_cmp(&a.risk).unwrap_or(std::cmp::Ordering::Equal));

        // Journal the high-risk poleis with the generated causal narrative.
        for c in centers.iter().filter(|c| c.tier == "HIGH") {
            let why = c.drivers.iter().take(3).map(|d| d.detail.clone()).collect::<Vec<_>>().join("; ");
            let watch = if c.watch_goods.is_empty() { String::new() } else { format!(" Watch: {}.", c.watch_goods.join(", ")) };
            let text = format!("{} — speculation {} ({:.2}). {}. Pattern: {}.{}", c.name, c.tier, c.risk, why, c.pattern_tag, watch);
            self.journal.push(JournalEntry { tick, kind: "speculation".into(), hub: c.hub as i32, good: -1, value: c.risk, text });
        }

        self.spec_prev_profit = cur_profit;
        self.spec_centers = centers;
        self.spec_year = year;
    }

    /// Rebuild the route-days matrix from hub positions + components. Same
    /// component → distance-based days; cross-component → unreachable.
    pub fn rebuild_routes(&mut self) {
        let n = self.hubs.len();
        let mut days = vec![f32::INFINITY; n * n];
        for a in 0..n {
            days[a * n + a] = 0.0;
            for b in (a + 1)..n {
                if self.hubs[a].component != self.hubs[b].component {
                    continue;
                }
                let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
                if self.world_w > 1.0 {
                    dx = dx.min(self.world_w - dx); // cylindrical wrap
                }
                let dy = self.hubs[a].y - self.hubs[b].y;
                let dist = (dx * dx + dy * dy).sqrt();
                let d = (dist * self.days_per_cell).max(1.0);
                days[a * n + b] = d;
                days[b * n + a] = d;
            }
        }
        self.days = days;
        self.rebuild_neighbors();
        self.routes_dirty = false;
    }

    /// Build each hub's nearest reachable trade partners (sorted nearest first,
    /// capped to `NEIGHBOR_K`). Estates are kept as candidates (they have a
    /// population that must still import food); the cap simply means dispatch
    /// never scans far-flung hubs, which is where the late-campaign cost went.
    fn rebuild_neighbors(&mut self) {
        let n = self.hubs.len();
        let mut neighbors: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut scratch: Vec<(u32, f32)> = Vec::with_capacity(n);
        for a in 0..n {
            scratch.clear();
            for b in 0..n {
                if b == a { continue; }
                let d = self.days[a * n + b];
                if d.is_finite() { scratch.push((b as u32, d)); }
            }
            // Partial-select the K nearest in O(n) (avoids an O(n log n) full sort of
            // every hub against every other on each estate add), then sort just those K.
            if scratch.len() > NEIGHBOR_K {
                scratch.select_nth_unstable_by(NEIGHBOR_K, |x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
                scratch.truncate(NEIGHBOR_K);
            }
            scratch.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
            neighbors[a] = scratch.iter().map(|&(b, _)| b).collect();
        }
        self.neighbors = neighbors;
    }

    #[inline]
    fn live_price(&self, stock: f32, need: f32, base: f32) -> f32 {
        (base * ((need + EPS) / (stock + EPS)).powf(self.k))
            .clamp(base * PRICE_FLOOR_MULT, base * PRICE_CEIL_MULT)
    }

    /// Aggregate inventory of good `g` available AT hub `h` for pricing & needs:
    /// the local-merchant pool (stored inline on the hub) PLUS every house/guild
    /// warehouse sited here. While `warehouses` is empty this equals the old
    /// `hubs[h].stock[g]`, so behaviour is unchanged until house depots exist.
    #[inline]
    pub fn hub_stock(&self, h: usize, g: usize) -> f32 {
        let mut s = self.hubs[h].stock.get(g).copied().unwrap_or(0.0);
        for w in &self.warehouses {
            if w.hub as usize == h {
                s += w.stock.get(g).copied().unwrap_or(0.0);
            }
        }
        s
    }

    /// Capacity tier (1..5) for a warehouse `capacity`; 0 = the uncapped −1 pool.
    #[inline]
    pub fn capacity_tier(capacity: f32) -> u8 {
        if capacity <= 0.0 { 0 }
        else if capacity <= WH_TIER1_CAP { 1 }
        else if capacity <= WH_TIER2_CAP { 2 }
        else if capacity <= WH_TIER3_CAP { 3 }
        else if capacity <= WH_TIER4_CAP { 4 }
        else { 5 }
    }

    /// Freight to haul one unit of good `g` over `days` at an already-discounted
    /// per-day `rate`: bulky goods cost more, perishable goods accrue spoilage.
    /// A 0 bulk (old saves) is treated as 1.0, so freight is unchanged for them.
    #[inline]
    fn good_freight(&self, g: usize, rate: f32, days: f32) -> f32 {
        let bulk = { let b = self.goods[g].bulk; if b <= 0.0 { 1.0 } else { b } };
        rate * days * bulk + self.goods[g].perishable.max(0.0) * days
    }

    /// Turn each hub's input STOCK into finished `Manufactured` goods, scaled by
    /// labor capacity (∝ population). Mirrors the worldgen `apply_manufacturing`
    /// pass so the living economy and the static trade map agree. Manufactured
    /// goods are ordered raws-first so multi-stage chains resolve; cycles are
    /// skipped (a good that never reaches depth is left unmade).
    fn manufacture_pass(&mut self) {
        let ng = self.goods.len();
        // Manufactured goods = those carrying a recipe.
        let recipe_goods: Vec<usize> = (0..ng).filter(|&g| !self.goods[g].inputs.is_empty()).collect();
        if recipe_goods.is_empty() {
            return;
        }
        // Depth = longest chain of manufactured inputs feeding this good; raws-first
        // order. Iterative relaxation (ng is small); leftover at -1 = a cycle, skip.
        let is_recipe = |g: usize| !self.goods[g].inputs.is_empty();
        let mut depth: Vec<i32> = vec![-1; ng];
        for _pass in 0..recipe_goods.len() + 1 {
            let mut changed = false;
            for &g in &recipe_goods {
                let mut d = 0;
                let mut ready = true;
                for &(idx, _) in &self.goods[g].inputs {
                    if idx < ng && is_recipe(idx) {
                        if depth[idx] < 0 { ready = false; break; }
                        d = d.max(depth[idx] + 1);
                    }
                }
                if ready && depth[g] != d {
                    depth[g] = d;
                    changed = true;
                }
            }
            if !changed { break; }
        }
        let mut order: Vec<usize> = recipe_goods.iter().copied().filter(|&g| depth[g] >= 0).collect();
        order.sort_by_key(|&g| (depth[g], g));

        // Median population → labor scale (big cities out-make villages).
        let mut pops: Vec<f32> = self.hubs.iter().map(|h| h.population.max(0.0)).collect();
        pops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_pop = if pops.is_empty() { 1.0 } else { pops[pops.len() / 2].max(1.0) };

        // Fungible input substitutes (bay salt ↔ rock salt as a preservative cure).
        // Mirrors worldgen `manufacture::apply_manufacturing`; narrow by design so
        // metals/fibres never swap as structural inputs.
        let subs: std::collections::HashMap<usize, Vec<usize>> = {
            let mut m: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for g in 0..ng {
                if !self.goods[g].fungible_input || self.goods[g].category == i32::MAX { continue; }
                let cat = self.goods[g].category;
                let sibs: Vec<usize> = (0..ng)
                    .filter(|&j| j != g && self.goods[j].fungible_input && self.goods[j].category == cat)
                    .collect();
                if !sibs.is_empty() { m.insert(g, sibs); }
            }
            m
        };

        for h in 0..self.hubs.len() {
            let pop = self.hubs[h].population.max(0.0);
            for &g in &order {
                let labor = { let l = self.goods[g].labor; if l <= 0.0 { 1.0 } else { l } };
                let labor_cap = (pop / median_pop) * labor;
                if labor_cap <= 0.0 { continue; }
                let mut by_inputs = f32::INFINITY;
                for &(idx, qty) in &self.goods[g].inputs {
                    if qty <= 0.0 || idx >= ng { continue; }
                    let mut avail = self.hubs[h].stock[idx];
                    if let Some(sl) = subs.get(&idx) { for &s in sl { avail += self.hubs[h].stock[s]; } }
                    by_inputs = by_inputs.min(avail / qty);
                }
                if !by_inputs.is_finite() || by_inputs <= 0.0 { continue; }
                let made = by_inputs.min(labor_cap);
                if made <= 0.0 { continue; }
                // Clone inputs to avoid borrow conflict while mutating stock.
                let inputs = self.goods[g].inputs.clone();
                for (idx, qty) in inputs {
                    if idx >= ng { continue; }
                    let mut need = made * qty;
                    let take = self.hubs[h].stock[idx].min(need);
                    self.hubs[h].stock[idx] -= take;
                    need -= take;
                    if need > 0.0 {
                        if let Some(sl) = subs.get(&idx) {
                            for &s in sl {
                                if need <= 0.0 { break; }
                                let t = self.hubs[h].stock[s].min(need);
                                self.hubs[h].stock[s] -= t;
                                need -= t;
                            }
                        }
                    }
                }
                self.hubs[h].stock[g] += made;
                self.hubs[h].production[g] += made;
            }
        }
    }

    /// Add manufacturing (derived) demand for recipe INPUTS onto the needs table,
    /// so dispatch carries raw wool/iron/sugar into the cities able to work them.
    /// Demand scales with each city's labour capacity (∝ population) — big cities
    /// pull more inputs and so become the manufacturing centres.
    fn add_manufacturing_demand(&mut self, needs: &mut [Vec<f32>]) {
        let ng = self.goods.len();
        let recipe_goods: Vec<usize> = (0..ng).filter(|&g| !self.goods[g].inputs.is_empty()).collect();
        if recipe_goods.is_empty() { return; }
        let mut pops: Vec<f32> = self.hubs.iter().map(|h| h.population.max(0.0)).collect();
        pops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = pops.get(pops.len() / 2).copied().unwrap_or(1.0).max(1.0);
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; } // manufacturing happens in cities
            let cap = (self.hubs[h].population.max(0.0) / median).min(8.0);
            if cap <= 0.0 { continue; }
            for &g in &recipe_goods {
                let labor = { let l = self.goods[g].labor; if l <= 0.0 { 1.0 } else { l } };
                for &(idx, qty) in &self.goods[g].inputs {
                    if idx < ng && qty > 0.0 { needs[h][idx] += cap * labor * qty * MANUFACTURE_PULL; }
                }
            }
        }
    }

    /// Base (pre-substitution) per-capita need for a hub/good this tick.
    #[inline]
    fn base_need(&self, h: usize, g: usize) -> f32 {
        let tg = &self.goods[g];
        // Demand cadence: a good consumed every N days exerts ~30/N of the daily
        // pull of a monthly good. Clamped so it modulates (not dominates) — long
        // cadence goods (furs, luxuries) sit cheaper locally and skew to wholesale.
        let interval = if tg.consumption_interval > 0.0 { tg.consumption_interval } else { 30.0 };
        let cadence = (30.0 / interval).clamp(0.30, 1.8);
        self.hubs[h].population
            * TIER_WEIGHT[tg.need_tier.min(2) as usize]
            * tg.desire.max(0.0)
            * cadence
            * self.need_scale
            * DEMAND_PRESSURE
    }

    /// Hub latitude as a signed fraction: +1 = north pole, 0 = equator, −1 =
    /// south pole. In the equirectangular world `y=0` is the north edge and
    /// `y=world_h` the south edge. Returns 0 when world height is unknown
    /// (old saves) so seasonality degrades to a single global hemisphere.
    #[inline]
    fn hub_lat_frac(&self, h: usize) -> f32 {
        if self.world_h > 1.0 {
            (1.0 - 2.0 * self.hubs[h].y / self.world_h).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    /// Multi-year "fertile vs lean" yield factor for a hemisphere — a slow cycle
    /// of good and bad harvest years (deterministic per seed/year/hemisphere).
    /// Ranges ~0.86 (lean) .. ~1.16 (bountiful); interpolated across the year so
    /// it drifts rather than snapping on New Year's Day.
    fn fertile_year_factor(&self, north: bool, day_of_year: u32) -> f32 {
        let year = self.tick / TICKS_PER_YEAR;
        let hemi = if north { 0u64 } else { 1u64 };
        let yf = |yr: u32| 0.86 + 0.30 * hash01(self.seed, yr as u64 ^ 0x5E450, hemi ^ 0xFE27);
        let f = day_of_year as f32 / TICKS_PER_YEAR as f32;
        yf(year) * (1.0 - f) + yf(year + 1) * f
    }

    /// Seasonal production multiplier for a hub's good this tick. Food crops peak
    /// at harvest; the harvest is offset by HALF A YEAR between the northern and
    /// southern hemisphere (so the world is never short everywhere at once — when
    /// the north troughs the south is at peak, and trade can balance it). Seasonal
    /// amplitude grows with latitude (the tropics crop year-round, high latitudes
    /// have one sharp harvest), and a slow multi-year fertile/lean cycle rides on
    /// top. Non-food goods are aseasonal (1.0).
    fn seasonal_mult(&self, h: usize, g: usize, day_of_year: u32) -> f32 {
        if !self.goods[g].food {
            return 1.0;
        }
        let lat = self.hub_lat_frac(h);
        let north = lat >= 0.0;
        // Tropics (|lat| small) barely swing; temperate/high latitudes swing hard.
        let amp = 0.10 + 0.32 * lat.abs();
        // Northern harvest peaks in late summer (~day 230 ⇒ phase 0.63); the
        // southern hemisphere is shifted half a year so its peak falls opposite.
        let hemi_shift = if north { 0.0 } else { 0.5 };
        let phase = (day_of_year as f32 / TICKS_PER_YEAR as f32 - 0.63 + hemi_shift)
            * std::f32::consts::TAU;
        let seasonal = 1.0 + amp * phase.cos();
        seasonal * self.fertile_year_factor(north, day_of_year)
    }

    /// Advance the simulation `n_ticks` days. Returns nothing; read state after.
    pub fn advance(&mut self, n_ticks: u32) {
        let ng = self.goods.len();
        if self.days.len() != self.n() * self.n() {
            self.rebuild_routes();
        }
        // One-time migration: a campaign started before fleets existed has houses
        // with no ships, so ALL their trade falls through to guilds and no house
        // can control anything. Seed each fleetless house a starting fleet once.
        if !self.fleets_migrated {
            for hi in 0..self.houses.len() {
                if self.houses[hi].defunct { continue; }
                if self.houses[hi].fleet_sea + self.houses[hi].fleet_river + self.houses[hi].fleet_caravan > 0 {
                    continue;
                }
                let coastal = self.hubs.get(self.houses[hi].hub as usize).map(|x| x.coastal).unwrap_or(false);
                let (s, r, c) = Self::initial_fleet(coastal, true);
                self.houses[hi].fleet_sea = s;
                self.houses[hi].fleet_river = r;
                self.houses[hi].fleet_caravan = c;
            }
            self.fleets_migrated = true;
        }
        // Tech factor defaults to 0.0 on pre-existing saves (serde) → treat as 1.0.
        if self.tech_factor <= 0.0 {
            self.tech_factor = 1.0;
        }
        // One-time migration: pre-existing saves seeded hubs with ABSOLUTE production
        // (population-independent). Derive each hub's per-capita rate so production now
        // tracks live population. New campaigns seed base_per_capita directly.
        if !self.percap_migrated {
            for h in 0..self.hubs.len() {
                if self.hubs[h].base_per_capita.len() == ng {
                    continue; // already seeded (new campaign)
                }
                let pop = self.hubs[h].founding_pop.max(1.0);
                self.hubs[h].base_per_capita =
                    self.hubs[h].production.iter().map(|&p| p / pop).collect();
            }
            self.percap_migrated = true;
        }
        // Per-day compounding growth equivalent to the yearly baseline drift.
        let tech_daily = (1.0 + PROD_GROWTH_PER_YEAR).powf(1.0 / TICKS_PER_YEAR as f32);
        // Reset per-advance diagnostics.
        self.diag_shipments = 0;
        self.diag_by_house = 0;
        self.diag_by_guild = 0;
        self.diag_lost = 0;
        self.diag_volume = 0.0;
        // Category membership for substitution.
        let n_cats = self
            .goods
            .iter()
            .filter(|g| g.category != i32::MAX)
            .map(|g| g.category + 1)
            .max()
            .unwrap_or(0)
            .max(0) as usize;
        let mut cat_goods: Vec<Vec<usize>> = vec![Vec::new(); n_cats];
        for (g, spec) in self.goods.iter().enumerate() {
            if spec.category != i32::MAX && spec.category >= 0 {
                cat_goods[spec.category as usize].push(g);
            }
        }

        // Opt-in per-phase profiler (set env WF2_PROFILE): prints a per-in-game-year
        // ms breakdown + the growth counters to the dev console, so late-campaign lag
        // can be attributed to a specific phase instead of guessed. Off → ~zero cost.
        let profile = std::env::var("WF2_PROFILE").is_ok();
        let (mut t_rebuild, mut t_trade, mut t_events, mut t_houses) = (0f32, 0f32, 0f32, 0f32);

        for _ in 0..n_ticks {
            self.tick += 1;
            let tick = self.tick;
            // Rebuild the route matrix + neighbour lists at most once per tick if a
            // hub was added/removed last tick (estate/colony). Batching this keeps
            // the O(n²) rebuild from running once per estate creation, and ensures
            // `self.neighbors` is always sized to `self.hubs` before dispatch reads it.
            if self.routes_dirty {
                let _s = std::time::Instant::now();
                self.rebuild_routes();
                t_rebuild += _s.elapsed().as_secs_f32() * 1000.0;
            }
            let n = self.hubs.len();
            let doy = self.day_of_year();

            // Phase G: keep the per-house ledgers aligned to the house list, and roll
            // the year over on the New Year — the just-finished year becomes the
            // Accountant's displayed `_prev`, and a fresh current year starts.
            self.house_ledger.resize(self.houses.len(), LedgerAcc::default());
            self.house_barred.resize(self.houses.len(), Vec::new());
            if tick % TICKS_PER_YEAR == 0 {
                // Yearly inflation erodes every fortune's real value, recorded in the
                // year that is now closing — then archive it for the Accountant.
                for hi in 0..self.houses.len() {
                    if self.houses[hi].defunct {
                        continue;
                    }
                    let infl = self.houses[hi].wealth.max(0.0) * INFLATION_PER_YEAR;
                    self.houses[hi].wealth -= infl;
                    if hi < self.house_ledger.len() {
                        self.house_ledger[hi].inflation += infl;
                    }
                    // Sample the year's closing wealth — the multi-year record that
                    // `stable_growth_years` reads to gate futures-contract terms.
                    let w = self.houses[hi].wealth;
                    let wh = &mut self.houses[hi].wealth_history;
                    wh.push(w);
                    if wh.len() > WEALTH_HISTORY_CAP { let drop = wh.len() - WEALTH_HISTORY_CAP; wh.drain(0..drop); }
                    // Bound the family chronicle so a long campaign doesn't accumulate
                    // an unbounded event list per house (memory + save size). Keep the
                    // most recent — the timeline UI only shows a recent window anyway.
                    let ev = &mut self.houses[hi].events;
                    if ev.len() > HOUSE_EVENTS_CAP { let drop = ev.len() - HOUSE_EVENTS_CAP; ev.drain(0..drop); }
                }
                self.house_ledger_prev = self.house_ledger.clone();
                let yr = tick / TICKS_PER_YEAR;
                // DLC 3 · the polis council sets the coming year's tariff / mint
                // policy, then the speculation why-engine reads the year that just
                // closed (uses `house_ledger_prev` before the books are reset).
                self.decide_polis_policy(yr);
                self.compute_speculation(yr);
                for l in self.house_ledger.iter_mut() {
                    *l = LedgerAcc { year: yr, ..Default::default() };
                }
            }

            // Expire finished events.
            self.active_events.retain(|e| e.until_tick > tick);
            // Production multipliers from active events (per hub/good, default 1).
            let prod_mult = self.event_production_mult();
            // Slow global productivity growth (~0.5%/yr baseline); bumper events add
            // more locally, adverse events dent the index (see roll_events).
            self.tech_factor *= tech_daily;

            // 1) Production — scales with LIVE population × per-capita output ×
            //    season × event shocks × global tech. A city that grows produces
            //    proportionally more; a shrinking outpost produces proportionally
            //    less (so tiny hubs can no longer flood the world with surplus).
            //    `production[g]` is kept as the realized output for downstream
            //    readers (estates, briefs, "strongest good").
            let tech = self.tech_factor;
            for h in 0..n {
                let pop = self.hubs[h].population.max(0.0);
                // Standing structure bonuses (Workshop/Warehouse = all goods,
                // Granary = food only); `struct_bonus` was the A1 placeholder hook.
                let (struct_all, struct_food) = self.hub_struct_prod(h);
                for g in 0..ng {
                    // Manufactured (recipe) goods aren't extracted per-capita — they're
                    // made from imported raws in the manufacturing pass below.
                    if !self.goods[g].inputs.is_empty() {
                        self.hubs[h].production[g] = 0.0;
                        continue;
                    }
                    let percap = self.hubs[h].base_per_capita.get(g).copied().unwrap_or(0.0);
                    let struct_bonus = struct_all * if self.goods[g].food { struct_food } else { 1.0 };
                    let realized = percap * pop * self.seasonal_mult(h, g, doy)
                        * prod_mult[h][g] * tech * struct_bonus;
                    self.hubs[h].production[g] = realized;
                    self.hubs[h].stock[g] += realized;
                }
            }

            // 1b) Manufacturing — cities transform imported raws into finished goods
            //     (wool→cloth, ore→arms), concentrated in big cities (labor ∝ pop).
            self.manufacture_pass();

            // 2) Consumption with per-category substitution toward cheaper goods.
            let mut needs = vec![vec![0.0f32; ng]; n];
            for h in 0..n {
                for g in 0..ng {
                    needs[h][g] = self.base_need(h, g);
                }
                for members in &cat_goods {
                    if members.len() < 2 {
                        continue;
                    }
                    let total: f32 = members.iter().map(|&g| self.base_need(h, g)).sum();
                    if total <= EPS {
                        continue;
                    }
                    let weights: Vec<f32> = members
                        .iter()
                        .map(|&g| {
                            let rel = (self.hubs[h].price[g] / self.goods[g].base_value.max(EPS))
                                .max(PRICE_FLOOR_MULT);
                            let pref = self.base_need(h, g) / total;
                            pref / rel
                        })
                        .collect();
                    let wsum: f32 = weights.iter().sum::<f32>().max(EPS);
                    for (mi, &g) in members.iter().enumerate() {
                        needs[h][g] = total * weights[mi] / wsum;
                    }
                }
                // Eat down stock; track unmet demand per need-tier for the
                // "% population lacking goods" graph (basic / comfort / luxury).
                let mut tier_need = [0.0f32; 3];
                let mut tier_unmet = [0.0f32; 3];
                for g in 0..ng {
                    let need = needs[h][g];
                    let eat = need.min(self.hubs[h].stock[g]);
                    self.hubs[h].stock[g] -= eat;
                    let t = self.goods[g].need_tier.min(2) as usize;
                    tier_need[t] += need;
                    tier_unmet[t] += (need - eat).max(0.0);
                }
                let frac = |t: usize| if tier_need[t] > EPS { tier_unmet[t] / tier_need[t] } else { 0.0 };
                // Smooth so the graph drifts rather than flickers tick-to-tick.
                self.hubs[h].lack_basic = 0.9 * self.hubs[h].lack_basic + 0.1 * frac(0);
                self.hubs[h].lack_comfort = 0.9 * self.hubs[h].lack_comfort + 0.1 * frac(1);
                self.hubs[h].lack_luxury = 0.9 * self.hubs[h].lack_luxury + 0.1 * frac(2);
            }

            // 2c) DERIVED (manufacturing) demand. A city that can weave/forge wants
            //     the RAW inputs — without this nothing pulls wool to weaving towns or
            //     iron to forges, so finished goods never formed. We add an input
            //     demand ∝ the city's labour capacity so dispatch carries the raws in
            //     (then `manufacture_pass` turns them into cloth/metalware/etc.).
            self.add_manufacturing_demand(&mut needs);

            // 3) Local prices (smoothed scarcity in the grain-eq numeraire).
            for h in 0..n {
                for g in 0..ng {
                    let base = self.goods[g].base_value;
                    let target = self.live_price(self.hubs[h].stock[g], needs[h][g], base);
                    self.hubs[h].price[g] = 0.6 * self.hubs[h].price[g] + 0.4 * target;
                }
            }

            // 3.5) Futures contracts deliver FIRST — the contracted quantity is
            //      reserved from each seller's source depot before the spot market
            //      runs, giving the buyer city forward supply security.
            let _s_trade = std::time::Instant::now();
            self.fulfill_contracts(&needs);
            // 4) Merchant dispatch (arbitrage → in-transit cargo).
            self.dispatch(&needs);
            t_trade += _s_trade.elapsed().as_secs_f32() * 1000.0;

            // 5) Arrivals. Decay each hub's by-sea/by-land supply tally, then add
            //    today's landings tagged by how they travelled (ships vs caravans).
            for hb in &mut self.hubs {
                hb.in_by_sea *= 0.98;
                hb.in_by_land *= 0.98;
            }
            // (to, good, amount, sea, phase, home, owner) — phase/home/owner let an
            // arriving OUTBOUND house cargo spawn its return leg from the dest hub.
            let mut landed: Vec<(usize, usize, f32, bool, u8, i32, i32)> = Vec::new();
            self.in_transit.retain(|c| {
                if c.eta_tick <= tick {
                    landed.push((c.to as usize, c.good, c.amount, c.sea, c.phase, c.home, c.owner));
                    false
                } else {
                    true
                }
            });
            for (to, g, amt, sea, phase, home, owner) in landed {
                if to < self.hubs.len() {
                    self.hubs[to].stock[g] += amt;
                    if sea { self.hubs[to].in_by_sea += amt; } else { self.hubs[to].in_by_land += amt; }
                }
                // Round trip: an OUTBOUND (phase 0) house cargo that just sold at `to`
                // now buys `to`'s surplus and carries it home for a second profit.
                if phase == 0 && owner >= 0 && home >= 0 {
                    self.deploy_return_leg(owner as usize, to, home as usize, &needs);
                }
            }

            // 6) Events.
            let _s_ev = std::time::Instant::now();
            self.roll_events();
            t_events += _s_ev.elapsed().as_secs_f32() * 1000.0;

            // 7) Food balance, estates & starvation.
            self.update_food_and_starvation(&needs);

            // 8) Houses.
            let _s_h = std::time::Instant::now();
            self.update_houses(&needs);
            t_houses += _s_h.elapsed().as_secs_f32() * 1000.0;

            // 8.5) Population sentiment (mood + drivers).
            self.update_sentiment();

            // 9) History — the "main" record is taken once per MONTH: a per-hub
            //    snapshot (charts + growth movers), the world price-index point
            //    (sparkline), and a rich world-summary chronicle row with numbers.
            if tick % 30 == 0 {
                self.sample_hub_history();
                self.sample_journal();
                self.sample_world_chronicle();
            }

            // Per-year profiler dump (opt-in). Counters reveal what is GROWING
            // (hubs/houses/warehouses/contracts) alongside where the ms go.
            if profile && tick % TICKS_PER_YEAR == 0 {
                eprintln!(
                    "[WF2 yr {:>3}] hubs={:>4} houses={:>3} wh={:>4} contracts={:>4} intransit={:>5} | \
                     trade={:>5.0} houses={:>5.0} events={:>4.0} rebuild={:>5.0} ms/yr",
                    tick / TICKS_PER_YEAR, self.hubs.len(), self.houses.len(),
                    self.warehouses.len(), self.contracts.len(), self.in_transit.len(),
                    t_trade, t_houses, t_events, t_rebuild,
                );
                t_rebuild = 0.0; t_trade = 0.0; t_events = 0.0; t_houses = 0.0;
            }
        }
    }

    /// Update each hub's mood and its three drivers (food / prosperity /
    /// stability), easing toward a target so the mood drifts rather than jumps.
    fn update_sentiment(&mut self) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        const EASE: f32 = 0.12;
        for h in 0..n {
            // Food security — the inverse of accumulated starvation pressure.
            let target_food = (1.0 - self.hubs[h].starving).clamp(0.0, 1.0);
            // Prosperity — saturating curve over grain + trade wealth + the civic
            // money the resident merchant houses spend locally (Phase G: trade
            // wealth reaching the populace). Per-capita so a feast lifts a town more
            // than a metropolis. The pool then decays (the money is spent through).
            let civic_pc = self.hubs[h].civic_pool / self.hubs[h].population.max(1.0) * 100.0;
            let w = (self.hubs[h].grain_wealth * 0.4 + self.hubs[h].trade_wealth * 0.8
                + civic_pc * 0.6).max(0.0);
            self.hubs[h].civic_pool *= CIVIC_DECAY;
            let target_prosp = (w / (w + 1.2)).clamp(0.0, 1.0);
            // Stability — lowered by active shocks on this hub (or world-wide) and
            // by widespread dearth (goods priced far above their world value).
            let mut hostility = 0.0f32;
            for e in &self.active_events {
                if e.hub == h as i32 { hostility += e.magnitude.max(0.25) + 0.1; }
                else if e.hub < 0 { hostility += 0.15; }
            }
            let mut dear = 0.0f32;
            for g in 0..ng {
                if self.hubs[h].price[g] > self.goods[g].base_value * 2.2 { dear += 1.0; }
            }
            let dear_frac = if ng > 0 { dear / ng as f32 } else { 0.0 };
            let target_stab = (1.0 - hostility - 0.5 * dear_frac).clamp(0.1, 1.0);

            let hb = &mut self.hubs[h];
            hb.sent_food += (target_food - hb.sent_food) * EASE;
            hb.sent_prosperity += (target_prosp - hb.sent_prosperity) * EASE;
            hb.sent_stability += (target_stab - hb.sent_stability) * EASE;
            let target_mood = 0.45 * hb.sent_food + 0.30 * hb.sent_prosperity + 0.25 * hb.sent_stability;
            hb.mood += (target_mood - hb.mood) * EASE;
        }
    }

    /// Phase G: a house barred from a market PAYS the city to regain its trading
    /// rights (one market a month, when it can afford the fee). The fee scales with
    /// the city's size, flows into the city's civic_pool (reaching the people), and
    /// is recorded on the Accountant's misfortune line.
    fn pay_to_regain_markets(&mut self) {
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct {
                continue;
            }
            let city = match self.house_barred.get(hi).and_then(|v| v.first().copied()) {
                Some(c) => c,
                None => continue,
            };
            let fee = self
                .hubs
                .get(city as usize)
                .map(|h| (h.population / 5000.0).clamp(2.0, 40.0))
                .unwrap_or(5.0);
            if self.houses[hi].wealth > fee * 2.0 {
                self.houses[hi].wealth -= fee;
                if let Some(hb) = self.hubs.get_mut(city as usize) {
                    hb.civic_pool += fee;
                }
                if let Some(v) = self.house_barred.get_mut(hi) {
                    v.retain(|&c| c != city);
                }
                if hi < self.house_ledger.len() {
                    self.house_ledger[hi].events += fee;
                }
            }
        }
    }

    /// Phase G monthly wealth sinks: every house/guild pays UPKEEP (depreciation
    /// that counters BANK_INTEREST) and spends a slice on CONSUMPTION that flows
    /// into its home city's `civic_pool` (reaching the people). Both are a fraction
    /// of wealth, so a fortune bleeds proportionally and wealth PLATEAUS where trade
    /// income balances the sinks instead of compounding without end.
    /// A city-size multiplier on a warehouse's keep — a depot in a great entrepôt
    /// costs far more (rents, wages, guards) than one in a market town.
    fn city_size_factor(&self, hub: usize) -> f32 {
        let pop = self.hubs.get(hub).map(|h| h.population).unwrap_or(30_000.0);
        (pop / 30_000.0).clamp(0.3, 4.0)
    }

    /// Per-city trade-tax bracket: the rate scales up with the city's prosperity, so
    /// a wealthy hub taxes trade harder than a struggling one (`CITY_TAX_BRACKET`).
    fn city_tax_factor(&self, hub: usize) -> f32 {
        let prosp = self.hubs.get(hub).map(|h| h.sent_prosperity).unwrap_or(0.5).clamp(0.0, 1.0);
        1.0 + CITY_TAX_BRACKET * prosp
    }

    fn apply_wealth_sinks(&mut self) {
        // Per-house warehouse upkeep + estate-depot counts, each accumulated in ONE
        // pass (were inner scans per house → O(houses·warehouses) and O(houses·hubs);
        // now O(warehouses)+O(hubs)). Late-campaign this is the bulk of the win.
        let nh0 = self.houses.len();
        let mut wh_upkeep = vec![0.0f32; nh0];
        for w in &self.warehouses {
            if w.owner >= 0 && (w.owner as usize) < nh0 {
                wh_upkeep[w.owner as usize] += CAP_UPKEEP * w.capacity * self.city_size_factor(w.hub as usize);
            }
        }
        let mut est_count = vec![0u32; nh0];
        for h in &self.hubs {
            if h.is_estate && h.owner_house >= 0 && (h.owner_house as usize) < nh0 {
                est_count[h.owner_house as usize] += 1;
            }
        }
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct {
                continue;
            }
            let home = self.houses[hi].hub as usize;
            let is_guild = self.houses[hi].is_guild;
            // ── Warehouse upkeep (CAPACITY-scaled, paid every month, trade or not) ──
            // Each of the house's depots costs `CAP_UPKEEP · capacity · city_size`,
            // so a bigger hoard in one city is dearer — pushing the house to spread
            // its stock across office cities rather than pile it up at home. A depot
            // at each owned estate keeps its cheaper flat rate. Charged even at
            // zero/negative wealth, so an idle or over-extended house slides into DEBT.
            let mut upkeep = wh_upkeep[hi];
            upkeep += est_count[hi] as f32 * UPKEEP_WAREHOUSE_BASE * UPKEEP_ESTATE_FRAC;

            // ── Conspicuous consumption (spent INTO the home city's people) ──
            // Only a slice of POSITIVE wealth — a house in debt buys no feasts.
            let pos = self.houses[hi].wealth.max(0.0);
            let consume_rate = if is_guild { GUILD_CIVIC_RATE } else { HOUSE_CONSUMPTION_RATE };
            let consumption = pos * consume_rate;

            // ── Progressive civic endowment (the GUILD wealth ceiling) ──────────
            // A guild that hoards beyond a soft cap (scaled to its home city's size)
            // is pressed to endow that city — the fraction drained RISES with how
            // far it overshoots, so guild fortunes plateau and the surplus visibly
            // enriches the settlement (the home city's civic_pool → its people).
            let mut endowment = 0.0f32;
            if is_guild && pos > 0.0 {
                let cap = GUILD_WEALTH_SOFTCAP * self.city_size_factor(home);
                if pos > cap {
                    let over = pos - cap;
                    // Drain grows with overshoot: 0 at the cap → up to GUILD_ENDOW_MAX.
                    let frac = (over / cap).clamp(0.0, 1.0) * GUILD_ENDOW_MAX;
                    endowment = over * frac;
                }
            }

            // ── Family overhead: a progressive, WEALTH-proportional drag (retainers,
            // mansions, patronage) on top of warehouse upkeep — it bites harder the
            // richer the family, so hoarding cash is costly and capital is pushed into
            // trade, depots and contracts. Private houses only; guilds are already
            // ceilinged by the progressive endowment above. Spent INTO the home city
            // (patronage → its people), like conspicuous consumption.
            let wealth_overhead = if is_guild { 0.0 }
                else { WEALTH_UPKEEP_RATE * (pos - WEALTH_UPKEEP_FREE).max(0.0) };

            self.houses[hi].wealth -= upkeep + consumption + endowment + wealth_overhead;

            if home < self.hubs.len() {
                // Consumption + the family overhead + the guild's civic dues (upkeep)
                // + the endowment all flow to the home city's people. A PRIVATE house's
                // upkeep leaves the economy (paid to landlords / abroad), so it isn't
                // credited; its wealth overhead IS (it's patronage spent in town).
                self.hubs[home].civic_pool += consumption + endowment + wealth_overhead;
                if is_guild {
                    self.hubs[home].civic_pool += upkeep;
                }
            }
            if hi < self.house_ledger.len() {
                self.house_ledger[hi].upkeep += upkeep;
                self.house_ledger[hi].consumption += consumption + endowment + wealth_overhead;
                // Sample wealth each month for the Accountant's year graph — now
                // signed, so a debt-ridden year shows the balance going negative.
                self.house_ledger[hi].wealth_samples.push(self.houses[hi].wealth);
            }
        }
    }

    /// Monthly warehouse pass (Phase 2): (1) ensure every live house has a home depot
    /// and one in each office city; (2) STOCK — each house draws a slice of its
    /// specialty goods' LOCAL SURPLUS (never food, never below the city's trade
    /// reserve) into its depot, paying the market (the cost circulates into the
    /// city's civic pool) — this is the inventory a futures contract later ships out
    /// of; (3) EXPAND — a profitable house enlarges a depot that stays nearly full.
    /// Stocking only MOVES goods within a hub (pool → house depot), so the aggregate
    /// `hub_stock` — and thus prices, needs and the famine balance — is unchanged.
    fn sync_and_stock_warehouses(&mut self, needs: &[Vec<f32>]) {
        let ng = self.goods.len();
        let nh = self.houses.len();
        // Slowly heal cosmetic damage on standing depots.
        for w in &mut self.warehouses { w.damage *= 0.98; }
        // (1) Ensure home + office depots exist. A single membership set of existing
        //     (owner, hub) pairs replaces the old per-call linear scan, so this is
        //     O(houses + offices) rather than O(houses · warehouses).
        let nhub = self.hubs.len();
        let mut have: std::collections::HashSet<(i32, u32)> =
            self.warehouses.iter().map(|w| (w.owner, w.hub)).collect();
        let mut new_depots: Vec<(i32, u32)> = Vec::new();
        for hi in 0..nh {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub;
            if (home as usize) < nhub && have.insert((hi as i32, home)) {
                new_depots.push((hi as i32, home));
            }
            for off in self.houses[hi].offices.clone() {
                if (off as usize) < nhub && have.insert((hi as i32, off)) {
                    new_depots.push((hi as i32, off));
                }
            }
        }
        for (owner, hub) in new_depots {
            self.warehouses.push(Warehouse {
                owner, hub, capacity: WH_START_CAP,
                stock: vec![0.0; ng], tier: Self::capacity_tier(WH_START_CAP), damage: 0.0,
            });
        }
        // (2) Stocking.
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= nh || self.houses[oi].defunct { continue; }
            let hub = self.warehouses[wi].hub as usize;
            if hub >= self.hubs.len() { continue; }
            let used: f32 = self.warehouses[wi].stock.iter().sum();
            let mut room = (self.warehouses[wi].capacity - used).max(0.0);
            if room <= EPS { continue; }
            for g in self.houses[oi].spec.clone() {
                if room <= EPS { break; }
                if g >= ng || self.goods[g].food { continue; }
                let reserve = needs[hub][g] * TRADE_RESERVE_MULT;
                let surplus = (self.hubs[hub].stock[g] - reserve).max(0.0);
                if surplus <= EPS { continue; }
                let price = self.live_price(self.hub_stock(hub, g), needs[hub][g], self.goods[g].base_value);
                let afford = if price > EPS { (self.houses[oi].wealth * 0.25).max(0.0) / price } else { 0.0 };
                let take = (surplus * WH_STOCK_FRAC).min(room).min(afford);
                if take <= EPS { continue; }
                self.hubs[hub].stock[g] -= take;
                self.warehouses[wi].stock[g] += take;
                let cost = take * price;
                self.houses[oi].wealth -= cost;
                self.hubs[hub].civic_pool += cost;
                room -= take;
            }
        }
        // (3) Expansion.
        let tick = self.tick;
        for wi in 0..self.warehouses.len() {
            let owner = self.warehouses[wi].owner;
            if owner < 0 { continue; }
            let oi = owner as usize;
            if oi >= nh || self.houses[oi].defunct { continue; }
            let cap = self.warehouses[wi].capacity;
            if cap >= WH_MAX_CAP { continue; }
            let used: f32 = self.warehouses[wi].stock.iter().sum();
            if used < cap * WH_FULL_FRAC { continue; }
            let cost = WH_EXPAND_COST * self.warehouses[wi].tier.max(1) as f32;
            if self.houses[oi].wealth < cost * 1.5 { continue; }
            if hash01(self.seed, tick as u64 ^ 0x3A5E, wi as u64) > 0.25 { continue; }
            self.houses[oi].wealth -= cost;
            let newcap = (cap * WH_EXPAND_MULT).min(WH_MAX_CAP);
            self.warehouses[wi].capacity = newcap;
            self.warehouses[wi].tier = Self::capacity_tier(newcap);
        }
    }

    /// Latest tick a plague quarantine at `hub` runs to (0 = none active). The hot
    /// paths (dispatch/fulfill) inline this into a per-tick lookup table; this
    /// single-hub form is kept for queries (UI / network routing).
    #[allow(dead_code)]
    fn quarantine_until(&self, hub: usize) -> u32 {
        self.active_events.iter()
            .filter(|e| e.kind == "plague_lockup" && e.hub == hub as i32 && e.until_tick > self.tick)
            .map(|e| e.until_tick).max().unwrap_or(0)
    }
    /// True while `hub` is locked up by plague (no trade in or out).
    #[allow(dead_code)]
    fn is_quarantined(&self, hub: usize) -> bool { self.quarantine_until(hub) > self.tick }

    /// Map a contract term (years) to its index into the TERM_* tables.
    fn term_index(years: u8) -> usize { TERM_YEARS.iter().position(|&y| y == years).unwrap_or(0) }

    /// Years of unbroken wealth growth on record — the seller's track record that
    /// gates the futures-contract term it may offer. A civic guild is inherently
    /// stable, so its "record" is simply its age in years.
    fn stable_growth_years(&self, hi: usize) -> u32 {
        let h = &self.houses[hi];
        if h.is_guild {
            return self.tick.saturating_sub(h.founded_tick) / TICKS_PER_YEAR;
        }
        let wh = &h.wealth_history;
        if wh.len() < 2 { return 0; }
        let mut run = 0u32;
        for i in (1..wh.len()).rev() {
            if wh[i] >= wh[i - 1] * 0.98 { run += 1; } else { break; }
        }
        run
    }

    /// The highest contract-term INDEX (into TERM_*) a house qualifies to offer:
    /// 1yr always · 3yr ≥4 stable yrs · 5yr ≥7 · 7yr >10.
    fn max_term_index(&self, hi: usize) -> usize {
        let y = self.stable_growth_years(hi);
        if y > 10 { 3 } else if y >= 7 { 2 } else if y >= 4 { 1 } else { 0 }
    }

    /// Deliver every DUE futures contract — runs BEFORE the spot `dispatch`, so the
    /// contracted quantity is reserved from the seller's source depot before the
    /// open market can compete for it (the buyer's forward security). A quarantine
    /// at either end suspends the contract (force majeure, no penalty); a seller that
    /// can't supply DEFAULTS and pays the buyer a term-scaled penalty.
    fn fulfill_contracts(&mut self, needs: &[Vec<f32>]) {
        if self.contracts.is_empty() { return; }
        let n = self.hubs.len();
        let ng = self.goods.len();
        let tick = self.tick;
        // Built ONCE per pass: quarantine end-tick per hub, and a (owner,hub)→depot
        // index — so the per-contract loop avoids re-scanning events and warehouses.
        let mut q_until = vec![0u32; n];
        for e in &self.active_events {
            if e.kind == "plague_lockup" && e.until_tick > tick && e.hub >= 0 && (e.hub as usize) < n {
                let h = e.hub as usize;
                if e.until_tick > q_until[h] { q_until[h] = e.until_tick; }
            }
        }
        let mut whidx: std::collections::HashMap<(i32, u32), usize> =
            std::collections::HashMap::with_capacity(self.warehouses.len());
        for (i, w) in self.warehouses.iter().enumerate() { whidx.insert((w.owner, w.hub), i); }
        // Fleet slots free THIS tick per house (fleet minus cargo already in flight).
        // Contracts run before `dispatch`, so they get first call on the vessels; a
        // house with no free ship/caravan for a due delivery is in logistics breach.
        let nh = self.houses.len();
        let mut cap_sea: Vec<i32> = vec![0; nh];
        let mut cap_land: Vec<i32> = vec![0; nh];
        for (i, h) in self.houses.iter().enumerate() {
            if h.defunct { continue; }
            cap_sea[i] = h.fleet_sea as i32;
            cap_land[i] = (h.fleet_river + h.fleet_caravan) as i32;
        }
        for c in &self.in_transit {
            if c.owner >= 0 { let oi = c.owner as usize;
                if oi < nh { if c.sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; } } }
        }
        let mut remove: Vec<usize> = Vec::new();
        for ci in 0..self.contracts.len() {
            let c = self.contracts[ci].clone();
            if tick >= c.end_tick { remove.push(ci); continue; }
            let (buyer, src, seller, g) =
                (c.buyer_hub as usize, c.source_hub as usize, c.seller_house as usize, c.good);
            if buyer >= n || src >= n || g >= ng
                || seller >= self.houses.len() || self.houses[seller].defunct {
                remove.push(ci); continue;
            }
            // Force majeure: a quarantine at either end suspends deliveries (no penalty).
            if q_until[buyer] > tick || q_until[src] > tick {
                self.contracts[ci].suspended_until = q_until[buyer].max(q_until[src]).max(tick + 1);
                continue;
            }
            if c.suspended_until > tick { continue; }
            // Monthly cadence (and a first delivery no sooner than one period after signing).
            if tick.saturating_sub(c.start_tick) < CONTRACT_DELIVER_DAYS { continue; }
            if c.last_fulfilled != 0 && tick.saturating_sub(c.last_fulfilled) < CONTRACT_DELIVER_DAYS { continue; }
            let days = self.days[src * n + buyer];
            if !days.is_finite() {
                self.contracts[ci].suspended_until = tick + CONTRACT_DELIVER_DAYS; // route gone
                continue;
            }
            let spot = self.live_price(self.hub_stock(buyer, g), needs[buyer][g], self.goods[g].base_value);
            let wi = whidx.get(&(seller as i32, src as u32)).copied();
            let have = wi.map(|i| self.warehouses[i].stock.get(g).copied().unwrap_or(0.0)).unwrap_or(0.0);
            if wi.is_none() || have < c.monthly_qty {
                // SELLER DEFAULT — can't deliver. Compensate the buyer above its spot
                // fallback, scaled by term (longer commitments hurt more to break).
                let ti = Self::term_index(c.term_years);
                let penalty = c.monthly_qty * spot * TERM_PENALTY_MULT[ti];
                self.houses[seller].wealth -= penalty;
                self.hubs[buyer].civic_pool += penalty;
                self.houses[seller].prestige = (self.houses[seller].prestige - 0.02).max(0.0);
                self.contracts[ci].defaults += 1;
                self.contracts[ci].last_fulfilled = tick;
                let (hn, cn, gn) = (self.houses[seller].name.clone(),
                    self.hubs[buyer].name.clone(), self.goods[g].name.clone());
                let txt = format!("{} defaults on its {} supply contract to {} (forfeits {:.0})", hn, gn, cn, penalty);
                self.houses[seller].events.push(HouseEvent { tick, kind: "disaster".into(), text: txt.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: buyer as i32, good: g as i32, value: penalty, text: txt });
                if self.contracts[ci].defaults >= 3 { remove.push(ci); }
                continue;
            }
            // ── Multimodal convoy ────────────────────────────────────────────────
            // The route's legs come from the endpoints: a SEA leg if either end is
            // coastal, a LAND leg if either is inland — so a coast↔inland route is
            // MIXED and must reserve BOTH a ship and a land vessel. A big delivery
            // fans out over MANY vessels (qty ÷ per-vessel capacity); each rolls its
            // own loss, so the cargo can arrive PARTIALLY (e.g. 10 of 12 ships).
            let qty = c.monthly_qty;
            let (src_coastal, buyer_coastal) = (self.hubs[src].coastal, self.hubs[buyer].coastal);
            let need_sea = src_coastal || buyer_coastal;     // ≥1 coastal → a sea leg
            let need_land = !(src_coastal && buyer_coastal);  // ≥1 inland → a land leg
            // Each required leg's monthly carrying capacity (free vessels × per-vessel
            // hold). The journey is limited by its TIGHTEST leg. A land vessel's hold
            // is the house's boat/caravan mix average (river boats carry more than
            // caravans), so a riverine house moves more overland per slot.
            let rv = self.houses[seller].fleet_river as f32;
            let cv = self.houses[seller].fleet_caravan as f32;
            let land_per = if rv + cv > 0.0 {
                (rv * BOAT_CAPACITY + cv * CARAVAN_CAPACITY) / (rv + cv)
            } else { CARAVAN_CAPACITY };
            let sea_cap = if need_sea { cap_sea[seller].max(0) as f32 * SHIP_CAPACITY } else { f32::INFINITY };
            let land_cap = if need_land { cap_land[seller].max(0) as f32 * land_per } else { f32::INFINITY };
            let leg_cap = sea_cap.min(land_cap);
            if leg_cap <= 0.0 {
                // A required leg has NO vessel free → LOGISTICS BREACH (penalty + strike).
                let ti = Self::term_index(c.term_years);
                let penalty = qty * spot * TERM_PENALTY_MULT[ti];
                self.houses[seller].wealth -= penalty;
                self.hubs[buyer].civic_pool += penalty;
                self.houses[seller].prestige = (self.houses[seller].prestige - 0.02).max(0.0);
                self.contracts[ci].defaults += 1;
                self.contracts[ci].last_fulfilled = tick;
                let (hn, cn, gn) = (self.houses[seller].name.clone(),
                    self.hubs[buyer].name.clone(), self.goods[g].name.clone());
                let txt = format!("{} has no vessel free for its {} contract to {} — breach (forfeits {:.0})", hn, gn, cn, penalty);
                self.houses[seller].events.push(HouseEvent { tick, kind: "disaster".into(), text: txt.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: buyer as i32, good: g as i32, value: penalty, text: txt });
                if self.contracts[ci].defaults >= 3 { remove.push(ci); }
                continue;
            }
            let loadable = qty.min(leg_cap); // can't ship more than the fleet can carry
            let ships_used = if need_sea { (loadable / SHIP_CAPACITY).ceil() as i32 } else { 0 };
            let landv_used = if need_land { (loadable / land_per).ceil() as i32 } else { 0 };
            cap_sea[seller] -= ships_used;
            cap_land[seller] -= landv_used;
            // Reserve the loaded goods from the source depot (sunk cargo is lost).
            let wi = wi.unwrap();
            self.warehouses[wi].stock[g] -= loadable;
            // Per-vessel loss. On a mixed route a unit must survive BOTH legs, so the
            // combined risk is 1−(1−p_sea)(1−p_land). The convoy = its binding leg's
            // vessels, each carrying an equal share of the load.
            let sea_p = if need_sea {
                if self.houses[seller].archetype == ARCH_FLEET { SEA_LOSS * FLEET_LOSS_MULT } else { SEA_LOSS }
            } else { 0.0 };
            let land_p = if need_land {
                let cv = self.houses[seller].fleet_caravan as f32;
                let rv = self.houses[seller].fleet_river as f32;
                let tot = (cv + rv).max(1.0);
                let base = CARAVAN_LOSS * (cv / tot) + RIVER_LOSS * (rv / tot);
                if self.houses[seller].archetype == ARCH_FLEET { base * FLEET_LOSS_MULT } else { base }
            } else { 0.0 };
            let route_loss = 1.0 - (1.0 - sea_p) * (1.0 - land_p);
            let vessels = ships_used.max(landv_used).max(1);
            let per = loadable / vessels as f32;
            let mut delivered_qty = 0.0;
            let mut sunk = 0;
            for k in 0..vessels {
                let lost = hash01(self.seed,
                    (tick as u64) ^ 0xC0117 ^ ((src as u64) << 8) ^ (buyer as u64),
                    (g as u64) ^ ((k as u64) << 24)) < route_loss;
                if lost {
                    sunk += 1;
                    if need_sea { self.damage_fleet(seller, true); }
                    if need_land { self.damage_fleet(seller, false); }
                    self.diag_lost += 1;
                } else {
                    delivered_qty += per;
                }
            }
            self.contracts[ci].last_fulfilled = tick;
            let sea = need_sea; // tag the in-transit leg as a sea voyage when one exists
            if sunk > 0 {
                let gn = self.goods[g].name.clone();
                let txt = format!("{} of {} {} convoys carrying {} are lost en route to {}",
                    sunk, vessels, if need_sea { "ship" } else { "caravan" }, gn,
                    self.hubs[buyer].name.clone());
                self.houses[seller].events.push(HouseEvent { tick, kind: "disaster".into(), text: txt.clone() });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: buyer as i32, good: g as i32, value: 0.0, text: txt });
            }
            // A significant shortfall (heavy losses OR too few vessels to carry the
            // contracted amount) counts as a missed delivery; minor storm losses don't.
            // A GOOD delivery CLEARS the strike count, so only 3 CONSECUTIVE failures
            // void the contract — otherwise a multi-year contract inevitably accrues 3
            // scattered misses and voids before it can ever reach term (the "no
            // contracts ever finish" bug).
            if delivered_qty < qty * 0.5 {
                self.contracts[ci].defaults += 1;
                if self.contracts[ci].defaults >= 3 { remove.push(ci); }
            } else {
                self.contracts[ci].defaults = 0;
            }
            if delivered_qty <= EPS { continue; } // total loss — nothing ships/sells
            // Paid price drifts toward spot but stays within the band around the strike.
            let pt = (0.7 * c.strike_price + 0.3 * spot).clamp(
                c.strike_price * (1.0 - CONTRACT_PRICE_BAND),
                c.strike_price * (1.0 + CONTRACT_PRICE_BAND));
            let value = delivered_qty * pt;
            let freight = delivered_qty * self.good_freight(g, self.freight_per_day, days);
            self.houses[seller].wealth += value - freight;
            // Toll-free network transit: when the cargo moves between the house's OWN
            // cities (its gates), it pays reduced civic tolls at both ends.
            let toll = if self.is_house_node(seller, src as u32) && self.is_house_node(seller, buyer as u32) {
                NETWORK_TOLL_DISCOUNT
            } else { 1.0 };
            let export_tax = value * EXPORT_TAX_RATE * self.city_tax_factor(src) * toll;
            let import_tax = value * IMPORT_TAX_RATE * self.city_tax_factor(buyer) * toll;
            self.houses[seller].wealth -= export_tax + import_tax;
            self.hubs[src].civic_pool += export_tax;
            self.hubs[buyer].civic_pool += import_tax;
            self.hubs[src].export_earn += value;
            self.hubs[buyer].import_spend += value;
            self.houses[seller].volume += delivered_qty;
            self.in_transit.push(InTransit {
                from: src as u32, to: buyer as u32, good: g, amount: delivered_qty,
                eta_tick: tick + (days.ceil() as u32).max(1),
                owner: seller as i32, sea, phase: 1, home: -1, // one-way: no return leg
            });
            self.bump_trade_at(seller, src, delivered_qty);
            self.bump_trade_at(seller, buyer, delivered_qty);
            self.log_trade(src as u32, buyer as u32, g, delivered_qty, seller as i32, sea, pt);
            self.contracts[ci].delivered += delivered_qty;
        }
        for &ci in remove.iter().rev() { self.contracts.remove(ci); }
    }

    /// Monthly: a seated house with an office in a city that is a STRUCTURAL importer
    /// of one of its specialty goods — and which the house can source from its home
    /// depot — offers that city a futures contract, for the longest term its record
    /// allows, covering only the spare slice under the per-good coverage cap (so the
    /// spot market keeps the rest and prices still form).
    /// A city the house operates from — its home or any office.
    fn is_house_node(&self, hi: usize, hub: u32) -> bool {
        self.houses[hi].hub == hub || self.houses[hi].offices.contains(&hub)
    }
    /// True while the house holds a live lease on `hub`.
    fn office_leased(&self, hi: usize, hub: u32) -> bool {
        self.houses[hi].office_leases.iter().any(|&(h, until)| h == hub && until > self.tick)
    }
    /// True while an active contract relies on the house's base at `hub` (as buyer or
    /// source) — such an office must stay open for the life of the contract.
    fn backs_active_contract(&self, hi: usize, hub: u32) -> bool {
        self.contracts.iter().any(|c| c.seller_house as usize == hi
            && (c.buyer_hub == hub || c.source_hub == hub) && self.tick < c.end_tick)
    }
    /// Lease `hub` as a durable office for `years`: ensures it's an office, pays the
    /// city an upfront fee (once), and records/extends the lease end-tick.
    fn lease_office(&mut self, hi: usize, hub: u32, years: u32) {
        let until = self.tick + years * TICKS_PER_YEAR;
        if let Some(e) = self.houses[hi].office_leases.iter_mut().find(|(h, _)| *h == hub) {
            if until > e.1 { e.1 = until; }
        } else {
            self.houses[hi].office_leases.push((hub, until));
            let fee = OFFICE_LEASE_FEE * self.city_size_factor(hub as usize);
            self.houses[hi].wealth -= fee;
            if (hub as usize) < self.hubs.len() { self.hubs[hub as usize].civic_pool += fee; }
        }
        if !self.houses[hi].offices.contains(&hub) { self.houses[hi].offices.push(hub); }
    }

    fn form_contracts(&mut self, needs: &[Vec<f32>]) {
        if self.contracts.len() >= MAX_CONTRACTS { return; }
        let n = self.hubs.len();
        let ng = self.goods.len();
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].offices.is_empty() { continue; }
            if hash01(self.seed, tick as u64 ^ 0xC047, hi as u64) > CONTRACT_FORM_CHANCE { continue; }
            let ti = self.max_term_index(hi);
            let term = TERM_YEARS[ti];
            let offices = self.houses[hi].offices.clone();
            let specs = self.houses[hi].spec.clone();
            // The house's NETWORK nodes (home + offices) — any can SOURCE a contract,
            // letting a distant office-city be supplied a good the house makes far away.
            let nodes: Vec<u32> = std::iter::once(self.houses[hi].hub)
                .chain(offices.iter().copied()).collect();
            'outer: for &off in &offices {
                let buyer = off as usize;
                if buyer >= n { continue; }
                for &g in &specs {
                    if g >= ng || self.goods[g].food { continue; }
                    // Structural deficit: the city produces well under its own need.
                    if self.hubs[buyer].production.get(g).copied().unwrap_or(0.0) >= needs[buyer][g] * 0.8 { continue; }
                    let cap = CONTRACT_COVERAGE_CAP * needs[buyer][g] * 30.0;
                    if cap <= EPS { continue; }
                    let existing: f32 = self.contracts.iter()
                        .filter(|c| c.buyer_hub as usize == buyer && c.good == g)
                        .map(|c| c.monthly_qty).sum();
                    let room = (cap - existing).max(0.0);
                    if room <= EPS { continue; }
                    if self.contracts.iter().any(|c|
                        c.seller_house as usize == hi && c.buyer_hub as usize == buyer && c.good == g) { continue; }
                    // Pick the NEAREST reachable network node that can supply g (a depot
                    // holding it, or a city producing it) — the goods can come from
                    // anywhere on the house's network, not just home.
                    let src = nodes.iter().copied()
                        .filter(|&nd| nd as usize != buyer && (nd as usize) < n
                            && self.days[nd as usize * n + buyer].is_finite())
                        .filter(|&nd| {
                            self.warehouses.iter().any(|w| w.owner == hi as i32 && w.hub == nd
                                && w.stock.get(g).copied().unwrap_or(0.0) > 0.0)
                            || self.hubs.get(nd as usize).and_then(|h| h.production.get(g)).copied().unwrap_or(0.0) > 0.0
                        })
                        .min_by(|&a, &b| self.days[a as usize * n + buyer]
                            .partial_cmp(&self.days[b as usize * n + buyer]).unwrap_or(std::cmp::Ordering::Equal));
                    let src = match src { Some(s) => s as usize, None => continue };
                    // Size the monthly quantity to what the seller can REALISTICALLY
                    // SUPPLY and CARRY, not just to the buyer's need — otherwise the
                    // depot/fleet can't meet it and the contract defaults every month
                    // until it voids (the "contracts cancel a month later" bug).
                    //   supply = depot stock already at src + the sustainable rate the
                    //            depot restocks from the source city's monthly surplus.
                    let depot_stock: f32 = self.warehouses.iter()
                        .filter(|w| w.owner == hi as i32 && w.hub == src as u32)
                        .map(|w| w.stock.get(g).copied().unwrap_or(0.0)).sum();
                    let src_surplus = (self.hubs[src].production.get(g).copied().unwrap_or(0.0)
                        - needs[src][g]).max(0.0);
                    let supply_cap = depot_stock + src_surplus * 30.0 * WH_STOCK_FRAC;
                    //   carry = the seller's fleet capacity on this route's binding leg
                    //   (coast↔coast = sea; any inland end needs a land leg).
                    let (sc, bc) = (self.hubs[src].coastal, self.hubs[buyer].coastal);
                    let need_sea = sc || bc;
                    let need_land = !(sc && bc);
                    let rv = self.houses[hi].fleet_river as f32;
                    let cv = self.houses[hi].fleet_caravan as f32;
                    let land_per = if rv + cv > 0.0 {
                        (rv * BOAT_CAPACITY + cv * CARAVAN_CAPACITY) / (rv + cv)
                    } else { CARAVAN_CAPACITY };
                    let sea_carry = if need_sea { self.houses[hi].fleet_sea as f32 * SHIP_CAPACITY } else { f32::INFINITY };
                    let land_carry = if need_land { (rv + cv) * land_per } else { f32::INFINITY };
                    let carry_cap = sea_carry.min(land_carry);
                    let monthly_qty = room.min(supply_cap).min(carry_cap);
                    if monthly_qty <= EPS { continue; } // can't sustain a real delivery → don't sign
                    let strike = self.live_price(self.hub_stock(buyer, g), needs[buyer][g],
                        self.goods[g].base_value) * TERM_STRIKE_FACTOR[ti];
                    self.contracts.push(Contract {
                        seller_house: hi as u32, buyer_hub: buyer as u32, source_hub: src as u32,
                        good: g, monthly_qty, strike_price: strike, term_years: term,
                        start_tick: tick, end_tick: tick + term as u32 * TICKS_PER_YEAR,
                        delivered: 0.0, last_fulfilled: 0, suspended_until: 0, defaults: 0,
                    });
                    // Lease BOTH ends for the contract's life (≥ its term) so the bases
                    // can't lapse under it — the durable spine of the trade network.
                    let lease_years = (term as u32).max(OFFICE_LEASE_YEARS);
                    self.lease_office(hi, buyer as u32, lease_years);
                    if src != self.houses[hi].hub as usize { self.lease_office(hi, src as u32, lease_years); }
                    let (hn, cn, sn, gn) = (self.houses[hi].name.clone(), self.hubs[buyer].name.clone(),
                        self.hubs[src].name.clone(), self.goods[g].name.clone());
                    self.journal.push(JournalEntry {
                        tick, kind: "charter".into(), hub: buyer as i32, good: g as i32, value: term as f32,
                        text: format!("{} signs a {}-year {} supply contract: {} → {}", hn, term, gn, sn, cn) });
                    break 'outer; // at most one new contract per house per pass
                }
            }
        }
    }

    /// Monthly solvency check. A balance is allowed to go NEGATIVE (debt, shown
    /// in the Accountant); a PRIVATE house that stays in the red for a full year
    /// is declared bankrupt and dissolved. A GUILD never dissolves — its home city
    /// bails it out from the civic pool (and, failing that, simply carries the debt
    /// until its subsidy recovers it), because a city won't let its guild fail.
    fn update_solvency(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let w = self.houses[hi].wealth;
            if self.houses[hi].is_guild {
                if w < 0.0 {
                    let home = self.houses[hi].hub as usize;
                    if home < self.hubs.len() {
                        let bail = (-w).min(self.hubs[home].civic_pool.max(0.0));
                        self.hubs[home].civic_pool -= bail;
                        self.houses[hi].wealth += bail;
                    }
                }
                self.houses[hi].debt_since = 0; // guilds never accrue toward bankruptcy
                continue;
            }
            // Private house: stamp when debt begins, clear on recovery.
            if w < 0.0 {
                if self.houses[hi].debt_since == 0 {
                    self.houses[hi].debt_since = tick.max(1);
                }
            } else {
                self.houses[hi].debt_since = 0;
            }
            // Insolvent for a full year → bankrupt.
            let since = self.houses[hi].debt_since;
            if since > 0 && tick.saturating_sub(since) >= TICKS_PER_YEAR {
                self.houses[hi].events.push(HouseEvent {
                    tick, kind: "bankruptcy".into(),
                    text: "Ruined — a year in debt forces the house into bankruptcy".into(),
                });
                self.dissolve_house(hi);
            }
        }
    }

    /// Push one weekly history sample per hub (capped to the last ~5 years) for the
    /// settlement-window charts.
    fn sample_hub_history(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        for hb in &mut self.hubs {
            let mut idx = 0.0f32;
            if ng > 0 {
                let mut s = 0.0f32;
                for g in 0..ng {
                    s += hb.price[g] / self.goods[g].base_value.max(EPS);
                }
                idx = s / ng as f32;
            }
            let (pop_house, pop_local, pop_guild) = merchant_pops(hb);
            hb.history.push(HubSample {
                tick,
                population: hb.population,
                wealth: hb.grain_wealth + hb.trade_wealth,
                mood: hb.mood,
                price_index: idx,
                lack_basic: hb.lack_basic,
                lack_comfort: hb.lack_comfort,
                lack_luxury: hb.lack_luxury,
                pop_house,
                pop_local,
                pop_guild,
            });
            // Monthly samples → keep ~30 years of history.
            if hb.history.len() > 360 {
                let drop = hb.history.len() - 360;
                hb.history.drain(0..drop);
            }
        }
    }

    /// Total world population and the population-weighted world price index
    /// (1.0 = every good at its world-standard value). Single source of truth for
    /// both the sparkline sample and the monthly chronicle.
    fn world_totals(&self) -> (f32, f32) {
        let total_pop: f32 = self.hubs.iter().map(|h| h.population).sum();
        let mut idx = 0.0;
        let mut wsum = 0.0;
        for h in &self.hubs {
            let mut hp = 0.0;
            let mut hw = 0.0;
            for g in 0..self.goods.len() {
                let w = self.goods[g].base_value;
                hp += (h.price[g] / self.goods[g].base_value.max(EPS)) * w;
                hw += w;
            }
            if hw > 0.0 {
                idx += (hp / hw) * h.population;
                wsum += h.population;
            }
        }
        (total_pop, if wsum > 0.0 { idx / wsum } else { 1.0 })
    }

    /// One rich monthly world-summary row (kind="world"): total population and
    /// world price index WITH the month-over-month change, plus the fastest-
    /// growing and fastest-shrinking city. This is the "main history" the player
    /// reads in the Chronicle. Per-hub history must be sampled first (movers read
    /// the last two per-hub samples).
    fn sample_world_chronicle(&mut self) {
        let (total_pop, index) = self.world_totals();
        let dpop_pct = if self.last_month_pop > 0.0 {
            (total_pop - self.last_month_pop) / self.last_month_pop * 100.0
        } else { 0.0 };
        let didx_pct = if self.last_month_index > 0.0 {
            (index - self.last_month_index) / self.last_month_index * 100.0
        } else { 0.0 };
        // Fastest grower / shrinker from the per-hub monthly history.
        let mut up: (&str, f32) = ("", 0.0);
        let mut down: (&str, f32) = ("", 0.0);
        for h in &self.hubs {
            let n = h.history.len();
            if n >= 2 {
                let prev = h.history[n - 2].population;
                let cur = h.history[n - 1].population;
                if prev > 0.0 {
                    let g = (cur - prev) / prev;
                    if g > up.1 { up = (h.name.as_str(), g); }
                    if g < down.1 { down = (h.name.as_str(), g); }
                }
            }
        }
        let mut text = format!(
            "Pop {} ({:+.1}%) · prices {:.2}× standard ({:+.1}%)",
            fmt_pop(total_pop), dpop_pct, index, didx_pct
        );
        if !up.0.is_empty() { text.push_str(&format!(" · ▲ {} {:+.0}%", up.0, up.1 * 100.0)); }
        if !down.0.is_empty() { text.push_str(&format!(" · ▼ {} {:+.0}%", down.0, down.1 * 100.0)); }
        self.journal.push(JournalEntry {
            tick: self.tick,
            kind: "world".into(),
            hub: -1,
            good: -1,
            value: index,
            text,
        });
        self.last_month_pop = total_pop;
        self.last_month_index = index;
    }

    /// Per-hub per-good production multiplier from active events (drought/blight…).
    fn event_production_mult(&self) -> Vec<Vec<f32>> {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let mut m = vec![vec![1.0f32; ng]; n];
        for e in &self.active_events {
            match e.kind.as_str() {
                "drought" | "blight" | "fishery_collapse" => {
                    // Regional: affect hubs within a radius of the event hub.
                    let center = if e.hub >= 0 { e.hub as usize } else { continue };
                    let (cx, cy) = (self.hubs[center].x, self.hubs[center].y);
                    for h in 0..n {
                        let mut dx = (self.hubs[h].x - cx).abs();
                        if self.world_w > 1.0 {
                            dx = dx.min(self.world_w - dx);
                        }
                        let dy = self.hubs[h].y - cy;
                        if (dx * dx + dy * dy).sqrt() < self.world_w * 0.12 {
                            for g in 0..ng {
                                let hit = match e.kind.as_str() {
                                    "drought" | "blight" => self.goods[g].food,
                                    "fishery_collapse" => {
                                        self.goods[g].name.contains("fish")
                                            || self.goods[g].name.contains("herring")
                                            || self.goods[g].name.contains("whal")
                                    }
                                    _ => false,
                                };
                                if hit {
                                    m[h][g] *= 1.0 - e.magnitude;
                                }
                            }
                        }
                    }
                }
                "embargo" => {
                    if e.hub >= 0 && e.good >= 0 {
                        m[e.hub as usize][e.good as usize] *= 1.0 - e.magnitude;
                    }
                }
                "bumper" => {
                    // Exceptional harvest: production surges at the hub (+mag), so
                    // its goods grow plentiful and cheap.
                    if let Some(center) = (e.hub >= 0).then_some(e.hub as usize) {
                        for g in 0..ng {
                            if self.goods[g].food || self.goods[g].name.contains("wine")
                                || self.goods[g].name.contains("oil") {
                                m[center][g] *= 1.0 + e.magnitude;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        // Floor the cumulative penalty: overlapping shocks (e.g. several droughts
        // covering one clustered region) used to stack multiplicatively to near
        // zero, draining even a full granary and triggering a famine death-spiral.
        // A bad season is a bad season — never a total production wipeout.
        for row in m.iter_mut() {
            for v in row.iter_mut() {
                if *v < EVENT_PROD_FLOOR { *v = EVENT_PROD_FLOOR; }
            }
        }
        m
    }

    /// Arbitrage one round: each surplus hub ships toward the best reachable
    /// deficit hubs, creating in-transit cargo with an ETA. Bounded per hub.
    fn dispatch(&mut self, needs: &[Vec<f32>]) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let tick = self.tick;
        // Quarantine lookup, built ONCE per dispatch (O(events)) instead of scanning
        // every active event inside the hot seller×target×good loop. A locked-down
        // city neither ships nor receives.
        let mut quarantined = vec![false; n];
        for e in &self.active_events {
            if e.kind == "plague_lockup" && e.until_tick > tick && e.hub >= 0 && (e.hub as usize) < n {
                quarantined[e.hub as usize] = true;
            }
        }
        // ── Merchant fleet capacity (concurrent shipment slots) for this round ──
        // Each house has fleet_sea sea-slots and (fleet_river + fleet_caravan)
        // land-slots. Slots already busy with in-flight cargo are subtracted, so a
        // house can only finance as many NEW shipments as it has free vessels. A
        // trade it can't carry falls to the independent local merchants/guilds.
        let nh = self.houses.len();
        let mut cap_sea: Vec<i32> = vec![0; nh];
        let mut cap_land: Vec<i32> = vec![0; nh];
        for (i, h) in self.houses.iter().enumerate() {
            if h.defunct { continue; }
            cap_sea[i] = h.fleet_sea as i32;
            cap_land[i] = (h.fleet_river + h.fleet_caravan) as i32;
        }
        for c in &self.in_transit {
            if c.owner >= 0 {
                let oi = c.owner as usize;
                if oi < nh { if c.sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; } }
            }
        }
        // Snapshot stocks so a single round's decisions use consistent prices.
        for g in 0..ng {
            let base = self.goods[g].base_value;
            // Build (hub, surplus) and (hub, price) lists.
            let reserve_mult = if self.goods[g].food { FOOD_RESERVE_DAYS } else { TRADE_RESERVE_MULT };
            let mut sellers: Vec<(usize, f32)> = Vec::new();
            for a in 0..n {
                // Keep a reserve (a granary for food) before exporting the rest.
                let surplus = self.hubs[a].stock[g] - needs[a][g] * reserve_mult;
                if surplus > EPS {
                    sellers.push((a, surplus));
                }
            }
            if sellers.is_empty() {
                continue;
            }
            for a_i in 0..sellers.len() {
                let (a, mut surplus) = sellers[a_i];
                if surplus <= EPS {
                    continue;
                }
                // A city under plague quarantine ships nothing out.
                if quarantined[a] { continue; }
                let pa = self.live_price(self.hubs[a].stock[g], needs[a][g], base);
                // A Guildhall at the SELLER's hub lowers freight on its exports.
                let freight_rate = self.freight_per_day
                    * if self.hub_has_struct(a, STRUCT_GUILDHALL) { GUILDHALL_FREIGHT } else { 1.0 };
                // Find the best deficit hubs among a's NEAREST reachable markets.
                // (Capping to the K nearest keeps this O(K) rather than O(n); the
                // 3 hungriest are kept below, so far-flung hubs never mattered.)
                let mut targets: Vec<(usize, f32, f32)> = Vec::new(); // (b, gap, days)
                for ti in 0..self.neighbors[a].len() {
                    let b = self.neighbors[a][ti] as usize;
                    if b == a {
                        continue;
                    }
                    // A quarantined city takes no imports either.
                    if quarantined[b] { continue; }
                    let days = self.days[a * n + b];
                    if !days.is_finite() {
                        continue;
                    }
                    let pb = self.live_price(self.hubs[b].stock[g], needs[b][g], base);
                    let freight = self.good_freight(g, freight_rate, days);
                    let gap = pb - (pa + freight) - self.margin * base;
                    if gap > 0.0 {
                        targets.push((b, gap, days));
                    }
                }
                if targets.is_empty() {
                    continue;
                }
                targets.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
                targets.truncate(3); // ship to the 3 hungriest reachable markets
                for (b, _gap, days) in targets {
                    if surplus <= EPS {
                        break;
                    }
                    // Don't overfill b past delivered-cost parity.
                    let delivered = pa + self.good_freight(g, freight_rate, days);
                    let max_stock =
                        needs[b][g] * (base / delivered.max(EPS)).powf(1.0 / self.k);
                    let room = (max_stock - self.hubs[b].stock[g]).max(0.0);
                    let mut amount = surplus.min(room * 0.5);
                    if amount <= EPS {
                        continue;
                    }
                    // Route mode: a sea voyage when both ends are coastal, else overland.
                    let sea = self.hubs[a].coastal && self.hubs[b].coastal;
                    // ── Who carries it ──────────────────────────────────────────
                    // Prefer the SELLER's house (the exporter organizes the sale);
                    // if it has no free vessel / no capital, fall back to the
                    // BUYER's house (the importing city sends its own ships to fetch
                    // the goods). This lets houses in big IMPORTER cities earn — the
                    // old code only ever credited the exporter, so houses clustered
                    // in importing capitals never grew. Only if NEITHER can carry it
                    // does it fall to independent local merchants & guilds.
                    let mut owner = -1i32;
                    for cand in [self.house_for(a, g), self.house_for(b, g)] {
                        if cand < 0 { continue; }
                        let oi = cand as usize;
                        // Trade war: a house barred from either market cannot run this
                        // leg — the trade falls to a rival or independent merchants.
                        if self.house_barred.get(oi).is_some_and(|v| v.contains(&(a as u32)) || v.contains(&(b as u32))) {
                            continue;
                        }
                        let slots = if sea { cap_sea[oi] } else { cap_land[oi] };
                        // Merchant-banker houses can finance cargo beyond their cash.
                        let credit = if self.houses[oi].archetype == ARCH_BANKING { BANK_CREDIT_MULT } else { 1.0 };
                        let afford = if pa > EPS { self.houses[oi].wealth * credit / pa } else { f32::MAX };
                        if slots >= 1 && afford > EPS {
                            amount = amount.min(afford);
                            if sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; }
                            owner = cand;
                            break;
                        }
                    }
                    surplus -= amount;
                    self.hubs[a].stock[g] -= amount;
                    let sale = amount * pa;
                    self.hubs[a].export_earn += sale;
                    // An ESTATE's sales pay rent to its OWNER: a share to the owning
                    // house's wealth (the engine of house growth), or to the parent
                    // city's prosperity if the estate is city-owned.
                    if self.hubs[a].is_estate {
                        let cut = sale * ESTATE_OWNER_CUT;
                        let owner = self.hubs[a].owner_house;
                        if owner >= 0 && (owner as usize) < self.houses.len()
                            && !self.houses[owner as usize].defunct {
                            self.houses[owner as usize].wealth += cut;
                            // Phase G: estate income, taxed by the parent city.
                            let etax = cut * ESTATE_TAX_RATE;
                            self.houses[owner as usize].wealth -= etax;
                            let parent = self.hubs[a].parent;
                            if parent >= 0 && (parent as usize) < self.hubs.len() {
                                self.hubs[parent as usize].civic_pool += etax;
                            }
                            if (owner as usize) < self.house_ledger.len() {
                                self.house_ledger[owner as usize].estate_income += cut;
                                self.house_ledger[owner as usize].estate_tax += etax;
                            }
                        } else {
                            let p = self.hubs[a].parent;
                            if p >= 0 && (p as usize) < self.hubs.len() {
                                self.hubs[p as usize].export_earn += cut;
                            }
                        }
                    }
                    // ── Voyage loss: storms at sea, ambush/wreck overland ──
                    let lost = if owner >= 0 {
                        let oi = owner as usize;
                        let mut p = if sea {
                            SEA_LOSS
                        } else {
                            // River boats are safer than caravans — blend by fleet mix.
                            let cv = self.houses[oi].fleet_caravan as f32;
                            let rv = self.houses[oi].fleet_river as f32;
                            let tot = (cv + rv).max(1.0);
                            CARAVAN_LOSS * (cv / tot) + RIVER_LOSS * (rv / tot)
                        };
                        // A shipping dynasty loses fewer cargoes (skilled crews).
                        if self.houses[oi].archetype == ARCH_FLEET { p *= FLEET_LOSS_MULT; }
                        hash01(self.seed,
                            (tick as u64) ^ 0x5EA10 ^ ((a as u64) << 8) ^ (b as u64),
                            g as u64) < p
                    } else { false };
                    if lost {
                        let oi = owner as usize;
                        let invested = amount * pa;
                        self.houses[oi].wealth = (self.houses[oi].wealth - invested).max(0.0);
                        if oi < self.house_ledger.len() {
                            self.house_ledger[oi].lost_cargo += invested;
                        }
                        self.damage_fleet(oi, sea);
                        let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                        let hn = self.houses[oi].name.clone();
                        let (etext, jtext) = if sea {
                            (format!("A storm sank a ship carrying {}", gn),
                             format!("A storm sinks a ship of {} ({})", hn, gn))
                        } else {
                            (format!("A caravan carrying {} was ambushed", gn),
                             format!("A caravan of {} is ambushed ({})", hn, gn))
                        };
                        self.houses[oi].events.push(HouseEvent { tick, kind: "voyage_loss".into(), text: etext });
                        // Tagged "voyage_loss" (not "event") so the settlement chronicle
                        // can hide the shipwreck/ambush spam — it's noise, not history.
                        self.journal.push(JournalEntry {
                            tick, kind: "voyage_loss".into(), hub: a as i32, good: g as i32,
                            value: invested, text: jtext,
                        });
                        self.diag_lost += 1;
                        // Cargo is gone — never delivered (source already debited).
                        continue;
                    }
                    self.diag_shipments += 1;
                    self.diag_volume += amount;
                    if owner >= 0 { self.diag_by_house += 1; } else { self.diag_by_guild += 1; }
                    // Attribute throughput to a merchant CLASS at both endpoints for
                    // the population estimate: a house-owned voyage → houses; an
                    // independent short haul → local merchants; a long haul → guilds.
                    let cls = if owner >= 0 { 0u8 } else if days <= LOCAL_HAUL_DAYS { 1 } else { 2 };
                    for &hh in &[a, b] {
                        match cls {
                            0 => self.hubs[hh].tw_house += amount,
                            1 => self.hubs[hh].tw_local += amount,
                            _ => self.hubs[hh].tw_guild += amount,
                        }
                    }
                    let value = amount * delivered;
                    self.hubs[b].import_spend += value;
                    if owner >= 0 {
                        let oi = owner as usize;
                        let margin = amount * (delivered - pa).max(0.0);
                        // A house that holds a monopoly on this good extracts extra
                        // rent (pricing power) on top of the plain margin.
                        let mono = self.houses[oi].monopoly.iter()
                            .find(|(mg, _)| *mg == g).map(|(_, s)| *s).unwrap_or(0.0);
                        let mut mult = 1.0 + 0.6 * mono;
                        // Specialist houses earn fatter margins on their trade; a city
                        // charter (political houses) adds further rent on that good.
                        if self.houses[oi].archetype == ARCH_SPECIALTY
                            && self.houses[oi].spec.contains(&g) { mult *= SPECIALTY_MARGIN; }
                        if self.houses[oi].charters.contains(&g) { mult *= CHARTER_RENT; }
                        let profit = margin * mult;
                        self.houses[oi].wealth += profit;
                        self.houses[oi].volume += amount;
                        // Phase G: civic taxes on this trade (export at origin a,
                        // import at destination b) — paid by the house, funding the
                        // cities (civic_pool → people). Guilds pay heavier taxes.
                        // Guilds pay heavier, PROGRESSIVE taxes — the more a guild
                        // trades, the higher the rate on each shipment.
                        let tax_mult = if self.houses[oi].is_guild {
                            let vol = self.houses[oi].volume.max(0.0);
                            GUILD_TAX_MULT
                                + GUILD_TAX_PROGRESSIVE * (vol / GUILD_TAX_VOLUME_REF).clamp(0.0, 1.0)
                        } else { 1.0 };
                        // DLC 3 · the origin/destination poleis levy their COUNCIL-set
                        // tariff (0 = no policy yet → the global default rate); a
                        // per-city prosperity bracket then scales it — rich cities tax
                        // harder, poor ones stay cheap to trade through.
                        let exp_rate = if self.hubs[a].tariff_export > 0.0 { self.hubs[a].tariff_export } else { EXPORT_TAX_RATE };
                        let imp_rate = if self.hubs[b].tariff_import > 0.0 { self.hubs[b].tariff_import } else { IMPORT_TAX_RATE };
                        let export_tax = value * exp_rate * tax_mult * self.city_tax_factor(a);
                        let import_tax = value * imp_rate * tax_mult * self.city_tax_factor(b);
                        self.houses[oi].wealth -= export_tax + import_tax;
                        self.hubs[a].civic_pool += export_tax;
                        self.hubs[b].civic_pool += import_tax;
                        if oi < self.house_ledger.len() {
                            LedgerAcc::add_city(&mut self.house_ledger[oi].trade_profit_by_city, b as u32, profit);
                            LedgerAcc::add_city(&mut self.house_ledger[oi].export_tax_by_city, a as u32, export_tax);
                            LedgerAcc::add_city(&mut self.house_ledger[oi].import_tax_by_city, b as u32, import_tax);
                        }
                        // Track cumulative profit per good (for "most profitable resources").
                        let gp = &mut self.houses[oi].good_profit;
                        if gp.len() <= g { gp.resize(g + 1, 0.0); }
                        gp[g] += profit;
                        // Build the holder's trade ties at both ends (for offices).
                        self.bump_trade_at(oi, a, amount);
                        self.bump_trade_at(oi, b, amount);
                    }
                    self.in_transit.push(InTransit {
                        from: a as u32,
                        to: b as u32,
                        good: g,
                        amount,
                        eta_tick: tick + (days.ceil() as u32).max(1),
                        owner,
                        sea,
                        // A house voyage is a ROUND TRIP: on arrival at b it tries to
                        // buy b's surplus and carry it home to a (sold there for a
                        // second profit). Guild/local one-way trips spawn no return.
                        phase: 0,
                        home: if owner >= 0 { a as i32 } else { -1 },
                    });
                    self.log_trade(a as u32, b as u32, g, amount, owner, sea, pa);
                }
            }
        }
    }

    /// The RETURN leg of a house round trip. A house vessel that just sold its
    /// outbound cargo at `b` buys `b`'s most profitable surplus good and carries it
    /// home to `a`, where it sells for a SECOND profit. The buy is usually at `b`'s
    /// market price, but an over-supplied (glutted) source yields an occasional
    /// ~25% bargain (a windfall that voyage). Profit here is true arbitrage
    /// (sell − buy − freight), so source discounts actually raise the take — the
    /// hook the office −5% discount (C3) plugs into. Respects the same food granary
    /// reserve and the buyer-side import cap, so it never strips `b`'s supply.
    fn deploy_return_leg(&mut self, owner: usize, b: usize, a: usize, needs: &[Vec<f32>]) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        if b >= n || a >= n || owner >= self.houses.len() || self.houses[owner].defunct {
            return;
        }
        let days = self.days[b * n + a];
        if !days.is_finite() {
            return;
        }
        // Freight home (a Guildhall at b lowers it); per-good weight/spoilage is
        // folded in per candidate good below via `good_freight`.
        let freight_rate = self.freight_per_day
            * if self.hub_has_struct(b, STRUCT_GUILDHALL) { GUILDHALL_FREIGHT } else { 1.0 };
        // An office at b gives the holder a standing −5% on what it buys there.
        let office_disc = if self.houses[owner].offices.contains(&(b as u32)) { OFFICE_BUY_DISCOUNT } else { 0.0 };
        // Pick b's surplus good that earns the most carried home to a.
        let mut best: Option<(usize, f32, f32, f32)> = None; // (good, amount, buy_price, sell_price)
        let mut best_score = 0.0f32;
        for g in 0..ng {
            let base = self.goods[g].base_value;
            let reserve_mult = if self.goods[g].food { FOOD_RESERVE_DAYS } else { TRADE_RESERVE_MULT };
            let surplus = self.hubs[b].stock[g] - needs[b][g] * reserve_mult;
            if surplus <= EPS { continue; }
            let pb = self.live_price(self.hubs[b].stock[g], needs[b][g], base);
            // Occasional bargain when b is heavily oversupplied in this good.
            let glut = self.hubs[b].stock[g] > (needs[b][g] * reserve_mult * 2.0).max(20.0);
            let bargain = glut && hash01(self.seed,
                (self.tick as u64) ^ 0x0BA46A1 ^ ((b as u64) << 8) ^ a as u64, g as u64) < 0.25;
            // Source-buy discount: an occasional glut bargain + any office −5%, capped.
            let discount = (if bargain { 0.25 } else { 0.0 } + office_disc).min(MAX_BUY_DISCOUNT);
            let pb_buy = pb * (1.0 - discount);
            let pa_sell = self.live_price(self.hubs[a].stock[g], needs[a][g], base);
            let freight = self.good_freight(g, freight_rate, days);
            let gap = pa_sell - pb_buy - freight - self.margin * base;
            if gap <= 0.0 { continue; }
            // Don't overfill a past delivered-cost parity.
            let delivered = pb_buy + freight;
            let max_stock = needs[a][g] * (base / delivered.max(EPS)).powf(1.0 / self.k);
            let room = (max_stock - self.hubs[a].stock[g]).max(0.0);
            let amount = surplus.min(room * 0.5);
            if amount <= EPS { continue; }
            let score = gap * amount;
            if score > best_score {
                best_score = score;
                best = Some((g, amount, pb_buy, pa_sell));
            }
        }
        let Some((g, amount, pb_buy, pa_sell)) = best else { return };
        let freight = self.good_freight(g, freight_rate, days);
        let sea = self.hubs[b].coastal && self.hubs[a].coastal;
        // Buy at b (goods leave b's stock), sell on arrival at a.
        self.hubs[b].stock[g] -= amount;
        self.hubs[b].export_earn += amount * pb_buy;
        self.hubs[a].import_spend += amount * (pb_buy + freight);
        // True-arbitrage profit (so the source discount actually pays).
        let mono = self.houses[owner].monopoly.iter()
            .find(|(mg, _)| *mg == g).map(|(_, s)| *s).unwrap_or(0.0);
        let mut mult = 1.0 + 0.6 * mono;
        if self.houses[owner].archetype == ARCH_SPECIALTY && self.houses[owner].spec.contains(&g) {
            mult *= SPECIALTY_MARGIN;
        }
        if self.houses[owner].charters.contains(&g) { mult *= CHARTER_RENT; }
        let profit = amount * (pa_sell - pb_buy - freight).max(0.0) * mult;
        self.houses[owner].wealth += profit;
        if owner < self.house_ledger.len() {
            // Round-trip arbitrage profit, realised selling at the home hub `a`.
            LedgerAcc::add_city(&mut self.house_ledger[owner].trade_profit_by_city, a as u32, profit);
        }
        self.houses[owner].volume += amount;
        let gp = &mut self.houses[owner].good_profit;
        if gp.len() <= g { gp.resize(g + 1, 0.0); }
        gp[g] += profit;
        // Diagnostics + throughput at both ends (house class).
        self.diag_shipments += 1;
        self.diag_by_house += 1;
        self.diag_volume += amount;
        self.hubs[b].tw_house += amount;
        self.hubs[a].tw_house += amount;
        self.bump_trade_at(owner, a, amount);
        self.bump_trade_at(owner, b, amount);
        // The same vessel carries it home (occupies the owner's slot until it lands).
        // No fresh voyage-loss roll here — the return is the trip's bonus leg.
        self.in_transit.push(InTransit {
            from: b as u32,
            to: a as u32,
            good: g,
            amount,
            eta_tick: self.tick + (days.ceil() as u32).max(1),
            owner: owner as i32,
            sea,
            phase: 1,
            home: -1,
        });
        self.log_trade(b as u32, a as u32, g, amount, owner as i32, sea, pb_buy);
    }

    /// Index helper so the borrow checker is happy reading b's stock in dispatch.
    fn house_for(&self, hub: usize, good: usize) -> i32 {
        // Private merchant houses TAKE OVER a city's trade in their specialty: a
        // seated specialist wins its own good first, then a specialist that holds an
        // OFFICE here (offices project real trading power into the city). Only then
        // does the civic GUILD carry the rest (the city's general/needs trade), then
        // any seated house, then any office-holder. This lets dynamic houses grow
        // dominant instead of the guild monopolising everything at home.
        let off = hub as u32;
        self.houses.iter()
            .position(|h| !h.defunct && !h.is_guild && h.hub as usize == hub && h.spec.contains(&good))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && !h.is_guild && h.offices.contains(&off) && h.spec.contains(&good)))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && h.is_guild && h.hub as usize == hub))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && !h.is_guild && h.hub as usize == hub))
            .or_else(|| self.houses.iter().position(|h| !h.defunct && h.offices.contains(&off)))
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    /// Roll low-probability events for this tick.
    fn roll_events(&mut self) {
        let n = self.hubs.len();
        if n == 0 {
            return;
        }
        let tick = self.tick;
        let r = hash01(self.seed, tick as u64, 0xE7E7);
        // ~ one event every ~12 ticks on average.
        if r > 0.085 {
            return;
        }
        let pick = hash01(self.seed, tick as u64, 0x1234);
        let hub = (hash01(self.seed, tick as u64, 0x5678) * n as f32) as usize % n;
        let (kind, mag, dur, good): (&str, f32, u32, i32) = if pick < 0.26 {
            // A drought trims food for a season. Kept moderate so a hub's granary
            // reserve + the baseline food surplus can ride it out — a bad year, not
            // an automatic famine (deep deficits used to spiral into collapse).
            ("drought", 0.20 + 0.15 * pick, 30 + (pick * 40.0) as u32, -1)
        } else if pick < 0.42 {
            ("plague", 0.18, 30, -1)
        } else if pick < 0.54 {
            ("fire", 0.5, 1, -1)
        } else if pick < 0.66 {
            ("fishery_collapse", 0.5, 120, -1)
        } else if pick < 0.82 {
            // EXCEPTIONAL YEAR — a bumper harvest: production surges for a season,
            // stocks build and prices fall, so this settlement's goods turn cheap
            // and flood out to its trade partners.
            ("bumper", 0.55 + 0.35 * pick, 120 + (pick * 60.0) as u32, -1)
        } else if pick < 0.90 {
            ("festival", 0.0, 1, -1)
        } else {
            // House feud → embargo on a random good at this hub.
            let g = (hash01(self.seed, tick as u64, 0x9999) * self.goods.len() as f32) as i32
                % self.goods.len().max(1) as i32;
            ("embargo", 0.8, 60, g)
        };
        let text = match kind {
            "drought" => format!("Drought grips the lands around {}", self.hubs[hub].name),
            "plague" => format!("Plague strikes {}", self.hubs[hub].name),
            "fire" => format!("Fire ravages the warehouses of {}", self.hubs[hub].name),
            "fishery_collapse" => format!("The fisheries off {} collapse", self.hubs[hub].name),
            "bumper" => format!("An exceptional harvest at {} — goods turn cheap", self.hubs[hub].name),
            "festival" => format!("{} holds a great festival", self.hubs[hub].name),
            _ => format!("A trade feud erupts at {}", self.hubs[hub].name),
        };
        // Immediate one-shot effects.
        match kind {
            "fire" => {
                for g in 0..self.goods.len() {
                    self.hubs[hub].stock[g] *= 1.0 - mag;
                }
                // The warehouses that burn belong to the city's merchant houses:
                // every resident house loses a slice of its wealth (stored stock
                // value), the heavier the richer it is — a stabilizing loss that
                // scales with prosperity. Recorded in the Accountant's misfortune line.
                for hi in 0..self.houses.len() {
                    if self.houses[hi].defunct || self.houses[hi].hub as usize != hub {
                        continue;
                    }
                    let loss = self.houses[hi].wealth * mag * 0.5;
                    self.houses[hi].wealth -= loss;
                    if hi < self.house_ledger.len() {
                        self.house_ledger[hi].events += loss;
                    }
                }
                // Phase 2: the fire also strikes ONE house depot in the city —
                // BURNING it out (all stock lost, gutted to a Tier-1 building) or
                // DAMAGING it (up to 80% of stock AND capacity, which may demote a
                // tier). A burned depot that can't meet a futures contract will later
                // trigger a seller default (Phase 3). Tagged "disaster" so it is kept
                // in the chronicle (unlike routine voyage losses).
                let wis: Vec<usize> = (0..self.warehouses.len())
                    .filter(|&i| self.warehouses[i].hub as usize == hub
                        && self.warehouses[i].owner >= 0
                        && !self.houses.get(self.warehouses[i].owner as usize)
                            .map(|h| h.defunct).unwrap_or(true))
                    .collect();
                if !wis.is_empty() {
                    let wi = wis[(hash01(self.seed, tick as u64 ^ 0xB175, 0) * wis.len() as f32)
                        as usize % wis.len()];
                    let oi = self.warehouses[wi].owner as usize;
                    let hname = self.houses[oi].name.clone();
                    let cname = self.hubs[hub].name.clone();
                    let old_t = self.warehouses[wi].tier;
                    let sev_roll = hash01(self.seed, tick as u64 ^ 0xF13E, wi as u64);
                    let txt = if sev_roll < 0.4 {
                        // BURN: total stock loss, building gutted to a Tier-1 depot.
                        for s in self.warehouses[wi].stock.iter_mut() { *s = 0.0; }
                        self.warehouses[wi].capacity = WH_TIER1_CAP;
                        self.warehouses[wi].tier = 1;
                        self.warehouses[wi].damage = 1.0;
                        format!("Fire guts the {} warehouse of {} — all stock lost", cname, hname)
                    } else {
                        // DAMAGE: up to 80% of stock AND capacity; capacity loss may demote.
                        let sev = 0.2 + 0.6 * sev_roll; // 0.2 .. 0.8
                        for s in self.warehouses[wi].stock.iter_mut() { *s *= 1.0 - sev; }
                        let newcap = (self.warehouses[wi].capacity * (1.0 - sev)).max(WH_TIER1_CAP * 0.5);
                        self.warehouses[wi].capacity = newcap;
                        self.warehouses[wi].tier = Self::capacity_tier(newcap);
                        self.warehouses[wi].damage = (self.warehouses[wi].damage + sev).min(1.0);
                        let note = if self.warehouses[wi].tier < old_t {
                            format!(", dropped to tier {}", self.warehouses[wi].tier)
                        } else { String::new() };
                        format!("Fire damages the {} warehouse of {} (−{:.0}% stock{})",
                            cname, hname, sev * 100.0, note)
                    };
                    self.houses[oi].events.push(HouseEvent {
                        tick, kind: "disaster".into(), text: txt.clone() });
                    self.journal.push(JournalEntry {
                        tick, kind: "disaster".into(), hub: hub as i32, good: -1, value: 0.0, text: txt });
                }
                // Estates around the city are struck too — a fire/blight cripples a
                // farm, mine or manufactory; a SEVERE one ABANDONS it (its people
                // scatter, production stops). The estate hub is kept (no index churn)
                // but goes dormant — its population falls toward zero.
                let ests: Vec<usize> = (0..self.hubs.len())
                    .filter(|&i| self.hubs[i].is_estate && self.hubs[i].parent == hub as i32).collect();
                if !ests.is_empty() {
                    let ei = ests[(hash01(self.seed, tick as u64 ^ 0xE57A, hub as u64) * ests.len() as f32) as usize % ests.len()];
                    let sev = hash01(self.seed, tick as u64 ^ 0xDEAD, ei as u64);
                    let ename = self.hubs[ei].name.clone();
                    let parent = self.hubs[ei].parent;
                    let txt = if sev < 0.18 {
                        self.hubs[ei].population = (self.hubs[ei].population * 0.02).max(1.0);
                        for v in self.hubs[ei].base_per_capita.iter_mut() { *v = 0.0; }
                        self.hubs[ei].estate_tier = 0;
                        format!("{} is abandoned after disaster — its lands fall silent", ename)
                    } else {
                        let s = 0.3 + 0.4 * sev;
                        self.hubs[ei].population *= 1.0 - s * 0.5;
                        for v in self.hubs[ei].base_per_capita.iter_mut() { *v *= 1.0 - s; }
                        self.hubs[ei].estate_tier = self.hubs[ei].estate_tier.saturating_sub(1).max(1);
                        format!("Disaster cripples {} (−{:.0}% output)", ename, s * 100.0)
                    };
                    self.journal.push(JournalEntry {
                        tick, kind: "disaster".into(), hub: parent, good: -1, value: 0.0, text: txt });
                }
            }
            "plague" => {
                self.hubs[hub].population *= 1.0 - mag;
                // Quarantine: the stricken city is LOCKED UP — no trade in or out —
                // for a spell longer than the mortality window. The spot market routes
                // around it; futures contracts touching it are force-majeure suspended
                // (no default). Tagged "disaster" so the lockup shows in the chronicle.
                let lock = 60 + (hash01(self.seed, tick as u64 ^ 0x10CC, hub as u64) * 120.0) as u32;
                self.active_events.push(ActiveEvent {
                    kind: "plague_lockup".into(), hub: hub as i32, good: -1,
                    magnitude: 1.0, until_tick: tick + lock,
                });
                self.journal.push(JournalEntry {
                    tick, kind: "disaster".into(), hub: hub as i32, good: -1, value: lock as f32,
                    text: format!("{} is locked down under quarantine", self.hubs[hub].name),
                });
            }
            "festival" => { /* demand spike handled implicitly by low stock */ }
            _ => {}
        }
        // Adverse events also dent the GLOBAL production index: an ordinary shock
        // trims ~1%, a rare catastrophic fire ~4.5% (on top of the local hit). The
        // slow +0.5%/yr drift recovers it over the following years.
        match kind {
            "fire" => self.tech_factor *= 1.0 - PROD_FIRE_SETBACK,
            "drought" | "plague" | "fishery_collapse" | "embargo" => {
                self.tech_factor *= 1.0 - PROD_EVENT_SETBACK;
            }
            _ => {}
        }
        self.tech_factor = self.tech_factor.max(TECH_FACTOR_FLOOR);
        if dur > 1 {
            self.active_events.push(ActiveEvent {
                kind: kind.to_string(),
                hub: hub as i32,
                good,
                magnitude: mag,
                until_tick: tick + dur,
            });
        }
        self.journal.push(JournalEntry {
            tick,
            kind: "event".into(),
            hub: hub as i32,
            good,
            value: mag,
            text,
        });
    }

    /// Food balance per hub → estates & starvation.
    fn update_food_and_starvation(&mut self, needs: &[Vec<f32>]) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        for h in 0..n {
            let mut food_need = 0.0;
            let mut food_have = 0.0;
            for g in 0..ng {
                if self.goods[g].food {
                    food_need += needs[h][g];
                    food_have += self.hubs[h].stock[g] + self.hubs[h].production[g];
                }
            }
            let bal = if food_need > EPS {
                (food_have - food_need) / food_need
            } else {
                1.0
            };
            // Smooth.
            self.hubs[h].food_balance = 0.85 * self.hubs[h].food_balance + 0.15 * bal;
            let fb = self.hubs[h].food_balance;
            // Starvation pressure builds when food balance is negative.
            if fb < 0.0 {
                self.hubs[h].starving = (self.hubs[h].starving + 0.02 * (-fb).min(1.0)).min(1.0);
            } else {
                self.hubs[h].starving = (self.hubs[h].starving - 0.02).max(0.0);
            }
            // Population: logistic growth toward a CARRYING CAPACITY set by both
            // FOOD security and TRADE prosperity. Well-fed, well-connected trade
            // hubs grow well above their founding size; food-poor or commercially
            // isolated settlements stagnate or shrink back. Uses the eased
            // sentiment drivers (already normalized 0..1) as the food/trade signal,
            // so the capacity tracks the same numbers the settlement window shows.
            let pop = self.hubs[h].population;
            let food_sec = self.hubs[h].sent_food.clamp(0.0, 1.0); // 1 = well fed
            let prosperity = self.hubs[h].sent_prosperity.clamp(0.0, 1.0); // trade+grain wealth
            // Capacity in multiples of the founding population (≈0.25× when poor &
            // hungry, up to ≈2.7× for a rich, well-fed entrepôt).
            let cap_mult = (0.35 + 1.15 * food_sec) * (0.70 + 1.10 * prosperity);
            let capacity = (self.hubs[h].founding_pop * cap_mult)
                .max(self.hubs[h].founding_pop * 0.15);
            // Logistic step: approach capacity from below, decline when above it.
            let rate = if pop < capacity { 0.0006 } else { 0.0012 };
            let mut new_pop = pop + rate * pop * (1.0 - pop / capacity);
            // Famine empties a city faster than trade decline alone.
            if self.hubs[h].starving > 0.5 {
                new_pop *= 1.0 - 0.0016 * (self.hubs[h].starving - 0.5);
                if self.tick % 90 == 0 {
                    self.journal.push(JournalEntry {
                        tick: self.tick,
                        kind: "starvation".into(),
                        hub: h as i32,
                        good: -1,
                        value: self.hubs[h].starving,
                        text: format!("{} suffers famine; people leave", self.hubs[h].name),
                    });
                }
            }
            self.hubs[h].population = new_pop.max(self.hubs[h].founding_pop * 0.10);
        }
        // Estate founding: a big, rich, food-secure hub with a hungry neighbour
        // founds a food estate. At most one per advance batch (cheap, rare).
        if self.tick % 120 == 0 {
            self.maybe_found_estate();
        }
        // Colonization of new land: rarer (yearly) — the settled map fills in.
        if self.tick % 365 == 0 {
            self.maybe_colonize();
        }
    }

    fn hub_has_struct(&self, h: usize, id: u8) -> bool {
        self.hubs[h].structures.contains(&id)
    }

    /// Standing production multipliers from a hub's structures: `(all_goods, food_only)`.
    fn hub_struct_prod(&self, h: usize) -> (f32, f32) {
        let (mut all, mut food) = (1.0f32, 1.0f32);
        for &s in &self.hubs[h].structures {
            match s {
                STRUCT_WORKSHOP => all *= WORKSHOP_PROD,
                STRUCT_WAREHOUSE => all *= WAREHOUSE_PROD,
                STRUCT_GRANARY => food *= GRANARY_FOOD_PROD,
                _ => {}
            }
        }
        (all, food)
    }

    /// Monthly: a prosperous settlement erects the most useful building it lacks.
    /// A Shipyard grants the resident house an extra sea ship on completion.
    fn update_structures(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].trade_wealth < STRUCT_BUILD_WEALTH { continue; }
            // Gradual: ~8%/month for an eligible hub → about one building a year.
            if hash01(self.seed, tick as u64 ^ 0x57D0C7, h as u64) > 0.08 { continue; }
            let has = |id: u8| self.hubs[h].structures.contains(&id);
            let coastal = self.hubs[h].coastal;
            let resident = self.strongest_house_at(h);
            let pick = if !has(STRUCT_WORKSHOP) { STRUCT_WORKSHOP }
                else if !has(STRUCT_GRANARY) { STRUCT_GRANARY }
                else if coastal && resident.is_some() && !has(STRUCT_SHIPYARD) { STRUCT_SHIPYARD }
                else if !has(STRUCT_GUILDHALL) { STRUCT_GUILDHALL }
                else if !has(STRUCT_WAREHOUSE) { STRUCT_WAREHOUSE }
                else { continue; };
            self.hubs[h].structures.push(pick);
            if pick == STRUCT_SHIPYARD {
                if let Some(hi) = resident { self.houses[hi].fleet_sea += 1; }
            }
            let hn = self.hubs[h].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "structure".into(), hub: h as i32, good: -1, value: 0.0,
                text: format!("{} builds a {}", hn, structure_label(pick)),
            });
        }
    }

    /// Monthly: a city spends its civic treasury (`civic_pool` — fed by trade
    /// taxes, guild dues and endowments) on PUBLIC WORKS. While it still lacks a
    /// useful civic building it erects one outright; once well-built it instead
    /// throws an occasional festival that lifts the people's prosperity and
    /// stability. This is the visible return of the guild-endowment sink to the
    /// settlement, distinct from the slower trade-wealth-funded `update_structures`.
    fn fund_public_works(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let pop = self.hubs[h].population.max(1.0);
            let civic_pc = self.hubs[h].civic_pool / pop * 100.0;
            if civic_pc < PUBLIC_WORKS_PC { continue; }
            let size = self.city_size_factor(h);
            let has = |id: u8| self.hubs[h].structures.contains(&id);
            // 1) Erect the next civic building it lacks (workshop → granary →
            //    guildhall → warehouse), if the treasury covers the cost.
            let pick = if !has(STRUCT_WORKSHOP) { Some(STRUCT_WORKSHOP) }
                else if !has(STRUCT_GRANARY) { Some(STRUCT_GRANARY) }
                else if !has(STRUCT_GUILDHALL) { Some(STRUCT_GUILDHALL) }
                else if !has(STRUCT_WAREHOUSE) { Some(STRUCT_WAREHOUSE) }
                else { None };
            let build_cost = PUBLIC_WORKS_BUILD_COST * size;
            if let Some(pick) = pick {
                if self.hubs[h].civic_pool >= build_cost {
                    self.hubs[h].civic_pool -= build_cost;
                    self.hubs[h].structures.push(pick);
                    let hn = self.hubs[h].name.clone();
                    self.journal.push(JournalEntry {
                        tick, kind: "structure".into(), hub: h as i32, good: -1, value: 0.0,
                        text: format!("{} funds public works — a {}", hn, structure_label(pick)),
                    });
                    continue;
                }
            }
            // 2) Well-built already → an occasional festival (a one-off lift to the
            //    populace), if the treasury can spare it.
            let fest_cost = FESTIVAL_COST * size;
            if self.hubs[h].civic_pool >= fest_cost
                && hash01(self.seed, tick as u64 ^ 0xFE57, h as u64) < 0.35
            {
                self.hubs[h].civic_pool -= fest_cost;
                self.hubs[h].sent_prosperity = (self.hubs[h].sent_prosperity + FESTIVAL_PROSPERITY).min(1.0);
                self.hubs[h].sent_stability = (self.hubs[h].sent_stability + FESTIVAL_STABILITY).min(1.0);
                let hn = self.hubs[h].name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "festival".into(), hub: h as i32, good: -1, value: 0.0,
                    text: format!("{} holds a public festival", hn),
                });
            }
        }
    }

    /// The strongest resident house at `hub` (richest, non-defunct), if any.
    fn strongest_house_at(&self, hub: usize) -> Option<usize> {
        let mut best = (usize::MAX, 0.0f32);
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct || hh.hub as usize != hub { continue; }
            if hh.wealth >= best.1 { best = (hi, hh.wealth); }
        }
        (best.0 != usize::MAX).then_some(best.0)
    }

    /// Push a new estate hub working good `g0` (kind `kind`) at `(x,y)`, owned by
    /// `owner_house` (−1 = the parent city). `percap` is the estate's dedicated
    /// per-capita output rate. Shared by neighbour-estates and new-land colonies.
    #[allow(clippy::too_many_arguments)]
    fn create_estate(&mut self, parent: i32, x: f32, y: f32, g0: usize, kind: u8,
                     owner_house: i32, koppen: u8, coastal: bool, component: u32,
                     base_pop: f32, percap: f32) {
        let ng = self.goods.len();
        let est_pop = base_pop.max(1.0);
        let mut base_per_capita = vec![0.0f32; ng];
        base_per_capita[g0] = percap.max(0.05);
        let mut production = vec![0.0f32; ng];
        production[g0] = base_per_capita[g0] * est_pop;
        let id = 100_000 + self.hubs.len() as u32;
        let owner_label = if owner_house >= 0 && (owner_house as usize) < self.houses.len() {
            self.houses[owner_house as usize].name.clone()
        } else if parent >= 0 && (parent as usize) < self.hubs.len() {
            self.hubs[parent as usize].name.clone()
        } else { "New".into() };
        let name = format!("{} {}", owner_label, estate_kind_label(kind));
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "estate".into(), hub: parent, good: g0 as i32, value: 0.0,
            text: format!("{} establishes {} ({})", owner_label, name, self.goods[g0].name),
        });
        self.hubs.push(TickHub {
            id, x, y, name, population: est_pop, founding_pop: est_pop,
            stock: vec![0.0; ng], price: self.goods.iter().map(|g| g.base_value).collect(),
            production, grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: true, parent, koppen, coastal, component,
            export_earn: 0.0, import_spend: 0.0, mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5,
            sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(), in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0,
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: kind, estate_tier: 1, owner_house, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
        });
        // Defer the O(n²) route/neighbour rebuild to the next tick (batched).
        self.routes_dirty = true;
    }

    /// Count satellite production sites (estates + colonies). Used to keep the hub
    /// list — and therefore every per-tick loop — bounded over a long campaign.
    fn estate_count(&self) -> usize {
        self.hubs.iter().filter(|h| h.is_estate).count()
    }

    fn maybe_found_estate(&mut self) {
        if self.estate_count() >= MAX_TOTAL_ESTATES { return; }
        let n = self.hubs.len();
        let ng = self.goods.len();
        // Founder: a LARGE, commercially successful, non-estate city (rank by
        // population × trade wealth) — big entrepôts plant estates, not tiny hubs.
        let mut best: Option<usize> = None;
        let mut best_score = 0.0f32;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].trade_wealth <= 0.15 { continue; }
            if self.hubs[h].food_balance < -0.1 { continue; } // a hungry city doesn't expand
            let score = self.hubs[h].population * self.hubs[h].trade_wealth.max(0.0);
            if score > best_score { best_score = score; best = Some(h); }
        }
        let Some(parent) = best else { return };
        // The estate works the parent's strongest export (highest per-capita output);
        // its kind follows that good (farm / mine / plantation / vineyard / fishery).
        let mut bestg = (usize::MAX, 0.0f32);
        for g in 0..ng {
            let pc = self.hubs[parent].base_per_capita.get(g).copied().unwrap_or(0.0);
            if pc > bestg.1 { bestg = (g, pc); }
        }
        let Some(mut g0) = (bestg.0 != usize::MAX).then_some(bestg.0) else { return };
        let mut kind = estate_kind_for_good(&self.goods[g0].name, self.goods[g0].food);
        // A fishery needs a coast; inland, fall back to the strongest food good (a farm).
        if kind == 4 && !self.hubs[parent].coastal {
            let mut bf = (g0, 0.0f32);
            for g in 0..ng {
                if self.goods[g].food {
                    let pc = self.hubs[parent].base_per_capita.get(g).copied().unwrap_or(0.0);
                    if pc > bf.1 { bf = (g, pc); }
                }
            }
            g0 = bf.0;
            kind = 1;
        }
        let owner_house = self.strongest_house_at(parent)
            .filter(|&hi| self.houses[hi].wealth >= ESTATE_HOUSE_OWNER_WEALTH)
            .map(|hi| hi as i32).unwrap_or(-1);
        // Place near the parent (deterministic small offset).
        let off = hash01(self.seed, self.tick as u64, parent as u64);
        let ex = self.hubs[parent].x + (off - 0.5) * self.world_w * 0.03;
        let ey = self.hubs[parent].y + (hash01(self.seed, parent as u64, self.tick as u64) - 0.5)
            * self.world_w * 0.02;
        let est_pop = self.hubs[parent].founding_pop * 0.15;
        let percap = self.hubs[parent].base_per_capita.get(g0).copied().unwrap_or(0.05).max(0.05) * 1.5;
        let (koppen, coastal, component) =
            (self.hubs[parent].koppen, self.hubs[parent].coastal, self.hubs[parent].component);
        self.create_estate(parent as i32, ex, ey, g0, kind, owner_house, koppen, coastal,
            component, est_pop, percap);
    }

    /// Monthly: a wealthy house invests its surplus capital into a new estate (raw
    /// production) or a manufactory (a luxury good), in a city it trades with —
    /// cheaper where it already holds an office. This is the wealth sink that turns
    /// hoarded profit into expansion and more production (estate income flows back
    /// to the owning house, so it compounds).
    fn maybe_house_invests(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let ng = self.goods.len();
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            if self.houses[hi].wealth < INVEST_WEALTH { continue; }
            // Soft cap so one rich house doesn't blanket the map with estates.
            let owned = self.hubs.iter().filter(|h| h.is_estate && h.owner_house == hi as i32).count();
            if owned >= MAX_HOUSE_ESTATES { continue; }
            // ~4%/month for an eligible house → invests roughly every couple of years.
            if hash01(self.seed, tick as u64 ^ 0xE57A7E, hi as u64) > 0.04 { continue; }
            // Prefer UPGRADING an existing estate/manufactory it owns (tier < 5) ~half
            // the time — cheaper than building new and compounds its output.
            if let Some(ei) = self.hubs.iter().enumerate()
                .filter(|(_, e)| e.is_estate && e.owner_house == hi as i32
                    && e.estate_tier > 0 && e.estate_tier < 5)
                .min_by_key(|(_, e)| e.estate_tier)
                .map(|(idx, _)| idx)
            {
                let tier = self.hubs[ei].estate_tier.max(1);
                let cost = INVEST_COST_BASE * tier as f32 * 0.8; // rises with tier
                if self.houses[hi].wealth >= cost * 1.5
                    && hash01(self.seed, tick as u64 ^ 0x09A7, hi as u64) < 0.5
                {
                    self.houses[hi].wealth -= cost;
                    self.hubs[ei].estate_tier = tier + 1;
                    for v in self.hubs[ei].base_per_capita.iter_mut() { *v *= ESTATE_UPGRADE_MULT; }
                    let (en, ep) = (self.hubs[ei].name.clone(), self.hubs[ei].parent);
                    self.journal.push(JournalEntry {
                        tick, kind: "estate".into(), hub: ep, good: -1, value: (tier + 1) as f32,
                        text: format!("{} upgrades to tier {}", en, tier + 1),
                    });
                    continue;
                }
            }
            // Global cap: upgrades (above) are always allowed (no new hub), but
            // building a NEW estate is blocked once the world is saturated — keeps
            // the hub list (and every per-tick loop) bounded late-campaign.
            if self.estate_count() >= MAX_TOTAL_ESTATES { continue; }
            // Build in the house's strongest trade partner (a city it actually works),
            // else at home. Skip estates themselves.
            let home = self.houses[hi].hub as usize;
            let target = self.houses[hi].trade_at.iter()
                .filter(|(hb, _)| (*hb as usize) < n && !self.hubs[*hb as usize].is_estate)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(hb, _)| *hb as usize)
                .unwrap_or(home);
            if target >= n || self.hubs[target].is_estate { continue; }
            // Per-settlement cap: don't overrun one city's hinterland with estates.
            let on_city = self.hubs.iter()
                .filter(|h| h.is_estate && h.parent == target as i32).count();
            if on_city >= MAX_ESTATES_PER_CITY { continue; }
            // Cost scales with the host city's size; an office there makes it cheaper.
            let has_office = self.houses[hi].offices.contains(&(target as u32));
            let cost = INVEST_COST_BASE
                * (self.hubs[target].population / 30_000.0).clamp(0.5, 3.0)
                * if has_office { 0.6 } else { 1.0 };
            if self.houses[hi].wealth < cost * 1.5 { continue; }
            // A manufactory for a LUXURY (one the house specializes in, or — for a
            // guild / spec-less holder — the target city's strongest-produced luxury),
            // else a raw estate of the city's strongest good. Mix the two so cities
            // get both raw output and value-added luxuries.
            let house_lux = self.houses[hi].spec.iter().cloned()
                .find(|&g| g < ng && !self.goods[g].food && self.goods[g].base_value >= 4.0);
            let city_lux = (0..ng)
                .filter(|&g| !self.goods[g].food && self.goods[g].base_value >= 4.0)
                .map(|g| (g, self.hubs[target].base_per_capita.get(g).copied().unwrap_or(0.0)))
                .filter(|(_, pc)| *pc > 0.0)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(g, _)| g);
            let want_manu = house_lux.is_some()
                || (city_lux.is_some() && hash01(self.seed, tick as u64 ^ 0xFAC7, hi as u64) < 0.5);
            let manu_good = house_lux.or(city_lux);
            let (g0, kind, percap) = if want_manu && manu_good.is_some() {
                (manu_good.unwrap(), 6u8, MANUFACTORY_PERCAP)
            } else {
                let mut bg = (usize::MAX, 0.0f32);
                for g in 0..ng {
                    let pc = self.hubs[target].base_per_capita.get(g).copied().unwrap_or(0.0);
                    if pc > bg.1 { bg = (g, pc); }
                }
                if bg.0 == usize::MAX { continue; }
                let k = estate_kind_for_good(&self.goods[bg.0].name, self.goods[bg.0].food);
                (bg.0, k, self.hubs[target].base_per_capita.get(bg.0).copied().unwrap_or(0.05).max(0.05) * 1.5)
            };
            // A fishery needs a coast; inland fall back to a farm of the city's good.
            let (kind, g0, percap) = if kind == 4 && !self.hubs[target].coastal {
                let mut bf = (usize::MAX, 0.0f32);
                for g in 0..ng {
                    if self.goods[g].food {
                        let pc = self.hubs[target].base_per_capita.get(g).copied().unwrap_or(0.0);
                        if pc > bf.1 { bf = (g, pc); }
                    }
                }
                if bf.0 == usize::MAX { continue; }
                (1u8, bf.0, (bf.1 * 1.5).max(0.05))
            } else { (kind, g0, percap) };
            self.houses[hi].wealth -= cost;
            let off = hash01(self.seed, tick as u64 ^ 0x12E5, target as u64);
            let ex = self.hubs[target].x + (off - 0.5) * self.world_w * 0.03;
            let ey = self.hubs[target].y
                + (hash01(self.seed, target as u64, tick as u64 ^ 0x77) - 0.5) * self.world_w * 0.02;
            let est_pop = self.hubs[target].founding_pop * 0.12;
            let (koppen, coastal, component) =
                (self.hubs[target].koppen, self.hubs[target].coastal, self.hubs[target].component);
            self.create_estate(target as i32, ex, ey, g0, kind, hi as i32, koppen, coastal,
                component, est_pop, percap);
        }
    }

    /// A very wealthy house (or, failing that, the largest city) plants an estate on
    /// the best reachable empty-land site — the world's settled map fills in over the
    /// campaign. Sites are precomputed at the Economy step and consumed as used.
    fn maybe_colonize(&mut self) {
        if self.colonizable.is_empty() || self.hubs.is_empty() { return; }
        if self.estate_count() >= MAX_TOTAL_ESTATES { return; }
        // Founder: the richest house (if wealthy enough) else the largest city.
        let mut founder_house = -1i32;
        let mut hw = COLONIZE_HOUSE_WEALTH;
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct { continue; }
            if hh.wealth > hw { hw = hh.wealth; founder_house = hi as i32; }
        }
        let founder_hub = if founder_house >= 0 {
            self.houses[founder_house as usize].hub as usize
        } else {
            let mut best = (usize::MAX, 0.0f32);
            for h in 0..self.hubs.len() {
                if self.hubs[h].is_estate { continue; }
                let s = self.hubs[h].population * self.hubs[h].trade_wealth.max(0.0);
                if s > best.1 { best = (h, s); }
            }
            match (best.0 != usize::MAX).then_some(best.0) { Some(h) => h, None => return }
        };
        if founder_hub >= self.hubs.len() { return; }
        // Only a commercially strong city colonizes when no rich house leads.
        if founder_house < 0 && self.hubs[founder_hub].trade_wealth < 0.25 { return; }
        // Best reachable unused site (fertility-weighted, nearer is better).
        let (fx, fy) = (self.hubs[founder_hub].x, self.hubs[founder_hub].y);
        let max_reach = self.world_w * 0.30;
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            let mut dx = (s.x - fx).abs();
            if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
            let dy = s.y - fy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > max_reach { continue; }
            let score = (0.4 + s.fertility) * (1.0 - dist / max_reach);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        let site = self.colonizable.swap_remove(si);
        // Pick a good matching the site's hint (prefer one the founder already makes).
        let ng = self.goods.len();
        let (mut g0, mut gbest) = (usize::MAX, 0.0f32);
        for g in 0..ng {
            if estate_kind_for_good(&self.goods[g].name, self.goods[g].food) == site.kind_hint {
                let s = self.hubs[founder_hub].base_per_capita.get(g).copied().unwrap_or(0.0) + 0.001;
                if s > gbest { gbest = s; g0 = g; }
            }
        }
        if g0 == usize::MAX {
            for g in 0..ng {
                let pc = self.hubs[founder_hub].base_per_capita.get(g).copied().unwrap_or(0.0);
                if pc > gbest { gbest = pc; g0 = g; }
            }
        }
        if g0 == usize::MAX { return; }
        let kind = estate_kind_for_good(&self.goods[g0].name, self.goods[g0].food);
        let founder_max_pc = self.hubs[founder_hub].base_per_capita.iter().cloned()
            .fold(0.0f32, f32::max).max(0.1);
        let percap = founder_max_pc * (0.5 + site.fertility);
        let est_pop = self.hubs[founder_hub].founding_pop * 0.12;
        let component = self.hubs[founder_hub].component;
        self.create_estate(founder_hub as i32, site.x, site.y, g0, kind, founder_house,
            site.koppen, site.coastal, component, est_pop, percap);
    }

    fn update_houses(&mut self, needs: &[Vec<f32>]) {
        for h in 0..self.hubs.len() {
            // Per-capita denominator floored at half the FOUNDING size so a hub that
            // loses population can't have its per-capita wealth spike to absurd
            // values (the old "millionaire outpost" bug). Estates inherit a small
            // founding size, so their wealth stays bounded too.
            let pop = self.hubs[h].population.max(self.hubs[h].founding_pop * 0.5).max(1.0);
            // Food security = current food-stock value per capita.
            self.hubs[h].grain_wealth = food_value(&self.hubs[h], &self.goods);
            // Commercial prosperity = recent net trade earnings per capita. The
            // accumulators decay so this tracks the last ~weeks, not all history.
            self.hubs[h].trade_wealth =
                (self.hubs[h].export_earn - self.hubs[h].import_spend) / pop;
            self.hubs[h].export_earn *= 0.97;
            self.hubs[h].import_spend *= 0.97;
            // Decay the per-class throughput tallies so the merchant-population
            // estimate tracks the last while, not all history.
            self.hubs[h].tw_house *= 0.97;
            self.hubs[h].tw_local *= 0.97;
            self.hubs[h].tw_guild *= 0.97;
        }
        self.update_house_dynamics(needs);
    }

    fn world_h(&self) -> u32 { (self.world_w * 0.5).max(1.0) as u32 }

    /// "House Cassii"-style family name for the home `hub`, varied by `salt`.
    fn family_name_for(&self, hub: usize, salt: u64) -> String {
        let (x, y) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        format!(
            "House {}",
            crate::sim::names::gen_family_name(x, y, self.world_w as u32, self.world_h(), salt)
        )
    }

    /// A GLOBALLY-UNIQUE "House X" name for `hub`. The culture surname pools are
    /// finite, so naive generation repeats names — and because a house's coat of
    /// arms is derived from its name, repeated names also mean repeated heraldry.
    /// Retry with re-salted surnames; if the pool collides, distinguish the family
    /// with its home city ("House Cassii of Aquentia") so every house is unique in
    /// both name and arms. Checks against ALL houses (incl. defunct) so a fallen
    /// family's name isn't silently reused.
    fn unique_family_name_for(&self, hub: usize, salt: u64) -> String {
        let taken = |name: &str, houses: &[House]| houses.iter().any(|h| h.name == name);
        for k in 0..32u64 {
            let cand = self.family_name_for(hub, salt ^ k.wrapping_mul(0x9E3779B1));
            if !taken(&cand, &self.houses) { return cand; }
        }
        let city = self.hubs[hub].name.clone();
        for k in 0..32u64 {
            let base = self.family_name_for(hub, salt ^ k.wrapping_mul(0x85EBCA77));
            let cand = format!("{} of {}", base, city);
            if !taken(&cand, &self.houses) { return cand; }
        }
        // Last resort (vanishingly rare): tick-tag guarantees uniqueness.
        format!("{} of {} [{}]", self.family_name_for(hub, salt), city, self.tick)
    }

    /// "Marcus Cassii"-style head name for `house_name` at `hub`, varied by `salt`.
    fn head_name_for(&self, hub: usize, house_name: &str, salt: u64) -> String {
        let surname = house_name.strip_prefix("House ").unwrap_or(house_name);
        let (x, y) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        crate::sim::names::gen_head_name(x, y, self.world_w as u32, self.world_h(), surname, salt)
    }

    /// A lifetime in ticks (≈45–75 years) from a hashed roll.
    fn roll_lifespan(&self, salt: u64) -> u32 {
        let r = hash01(self.seed, self.tick as u64 ^ 0x11FE, salt);
        ((45.0 + r * 30.0) * TICKS_PER_YEAR as f32) as u32
    }

    /// The living merchant families: ageing heads, monopolies, feuds, founding,
    /// extinction and political power.
    /// Log a dispatched trade for the Market "recent deals" rows (rolling, capped).
    fn log_trade(&mut self, from: u32, to: u32, good: usize, amount: f32, owner: i32, sea: bool, price: f32) {
        self.recent_trades.push(RecentTrade { from, to, good, amount, owner, sea, price, tick: self.tick });
        let n = self.recent_trades.len();
        if n > 400 { self.recent_trades.drain(0..n - 400); }
    }

    /// Record trade VOLUME a holder moved through a hub (for office ties).
    fn bump_trade_at(&mut self, holder: usize, hub: usize, amount: f32) {
        if holder >= self.houses.len() { return; }
        let t = &mut self.houses[holder].trade_at;
        if let Some(e) = t.iter_mut().find(|(hb, _)| *hb == hub as u32) {
            e.1 += amount;
        } else {
            t.push((hub as u32, amount));
        }
    }

    /// Monthly: found guilds for cities that have grown past the threshold, pay the
    /// civic subsidy into guild treasuries, and open/close offices for every holder.
    fn update_guilds_and_offices(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        // 1) A city that has grown to GUILD_MIN_POP and has no guild founds one.
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].population < GUILD_MIN_POP { continue; }
            let has_guild = self.houses.iter()
                .any(|g| !g.defunct && g.is_guild && g.hub as usize == h);
            if !has_guild {
                self.found_guild(h);
            }
        }
        // 2) Civic subsidy: the home city funds its guild (scaled by size + prosperity).
        for gi in 0..self.houses.len() {
            if self.houses[gi].defunct || !self.houses[gi].is_guild { continue; }
            let hub = self.houses[gi].hub as usize;
            if hub >= n { continue; }
            let pop = self.hubs[hub].population.max(0.0);
            let prosp = self.hubs[hub].sent_prosperity.clamp(0.1, 1.0);
            self.houses[gi].wealth += (pop / 1000.0) * GUILD_SUBSIDY_PER_1K * prosp;
        }
        // 3) Open / close offices for every active holder.
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub as usize;
            // Expired leases lapse; surviving leases each cost a monthly rent paid to
            // the host city (the durable-base running cost).
            self.houses[hi].office_leases.retain(|&(_, until)| until > tick);
            let leases = self.houses[hi].office_leases.clone();
            for &(lh, _) in &leases {
                let rent = OFFICE_LEASE_RENT * self.city_size_factor(lh as usize);
                self.houses[hi].wealth -= rent;
                if (lh as usize) < n { self.hubs[lh as usize].civic_pool += rent; }
            }
            // Strongest partner volume (scale-invariant trigger).
            let max_vol = self.houses[hi].trade_at.iter().map(|(_, v)| *v).fold(0.0f32, f32::max);
            // CLOSE: an office whose tie has withered, or a (private house) gone broke.
            let close_floor = (max_vol * 0.1).max(OFFICE_CLOSE_VOLUME);
            let broke = !self.houses[hi].is_guild && self.houses[hi].wealth < HOUSE_BANKRUPT;
            let offices = self.houses[hi].offices.clone();
            for &ohub in &offices {
                // A LEASED office, or one a live contract relies on, never auto-closes —
                // the network base is guaranteed for the contract's life.
                if self.office_leased(hi, ohub) || self.backs_active_contract(hi, ohub) { continue; }
                let vol = self.houses[hi].trade_at.iter()
                    .find(|(hb, _)| *hb == ohub).map(|(_, v)| *v).unwrap_or(0.0);
                if broke || vol < close_floor {
                    self.houses[hi].offices.retain(|&x| x != ohub);
                    let cn = self.houses[hi].name.clone();
                    let city = self.hubs.get(ohub as usize).map(|x| x.name.clone()).unwrap_or_default();
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "office_closed".into(),
                        text: format!("{} abandons its office in {}", cn, city),
                    });
                    self.journal.push(JournalEntry {
                        tick, kind: "office_closed".into(), hub: ohub as i32, good: -1, value: 0.0,
                        text: format!("{} closes its office in {}", cn, city),
                    });
                }
            }
            // OPEN: the strongest non-home partner with a real tie the holder can afford.
            if max_vol <= 0.0 { continue; }
            let mut cand: Option<(usize, f32)> = None;
            for &(hb, v) in &self.houses[hi].trade_at {
                let hb = hb as usize;
                if hb == home || hb >= n { continue; }
                if self.houses[hi].offices.contains(&(hb as u32)) { continue; }
                if v < OFFICE_OPEN_VOLUME || v < max_vol * 0.5 { continue; }
                if cand.map_or(true, |(_, bv)| v > bv) { cand = Some((hb, v)); }
            }
            if let Some((hb, _)) = cand {
                // Cost scales with the host city's importance (population).
                let cost = OFFICE_COST_BASE * (1.0 + self.hubs[hb].population / 50_000.0);
                if self.houses[hi].wealth >= cost * 1.5 {
                    self.houses[hi].wealth -= cost;
                    self.houses[hi].offices.push(hb as u32);
                    let cn = self.houses[hi].name.clone();
                    let city = self.hubs[hb].name.clone();
                    let verb = if self.houses[hi].is_guild { "establishes a factory" } else { "opens a counting-house" };
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "branch".into(),
                        text: format!("{} {} in {}", cn, verb, city),
                    });
                    self.journal.push(JournalEntry {
                        tick, kind: "office".into(), hub: hb as i32, good: -1, value: 0.0,
                        text: format!("{} {} in {}", cn, verb, city),
                    });
                }
            }
        }
    }

    /// Seed civic guilds for every city already at/above the population threshold
    /// when the campaign begins (more emerge later as cities grow — see
    /// `update_guilds_and_offices`).
    pub fn seed_initial_guilds(&mut self) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            if self.hubs[h].population >= GUILD_MIN_POP {
                self.found_guild(h);
            }
        }
    }

    /// Found a civic Merchant Guild for city `h` (≥ GUILD_MIN_POP). Distinct name,
    /// a starting treasury and fleet sized to the city; acts in the city's interest.
    fn found_guild(&mut self, h: usize) {
        let tick = self.tick;
        let coastal = self.hubs[h].coastal;
        let pop = self.hubs[h].population.max(1.0);
        let name = self.guild_name_for(h);
        let (fleet_sea, fleet_river, fleet_caravan) = Self::initial_fleet(coastal, true);
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("Chartered by the merchants of {}", self.hubs[h].name),
        };
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: h as i32, good: -1, value: 0.0,
            text: format!("{} is chartered in {}", name, self.hubs[h].name),
        });
        self.houses.push(House {
            name, hub: h as u32, wealth: (pop / 1000.0).max(1.0), prestige: 0.2,
            spec: vec![], monopoly: vec![], rivals: vec![], generation: 1,
            events: vec![founded], good_profit: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: 0.0, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: format!("Guildmaster of {}", self.hubs[h].name),
            head_since: tick, head_lifespan: self.roll_lifespan(h as u64 ^ 0x6111),
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: ARCH_SPECIALTY, charters: Vec::new(),
            is_guild: true, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
        });
    }

    /// A distinct guild name for a city, styled by its CULTURE (e.g. "Collegium of
    /// Aquentia (wine)", "Suq of Madinah", "Hang of Linzhou") — so a guild reads in
    /// its home people's idiom and tags the city's chief trade.
    fn guild_name_for(&self, h: usize) -> String {
        let city = self.hubs[h].name.clone();
        let (x, y) = (self.hubs[h].x.max(0.0) as u32, self.hubs[h].y.max(0.0) as u32);
        // The city's strongest-produced good flavours the guild ("of wine").
        let specialty = {
            let mut bg = (usize::MAX, 0.0f32);
            for g in 0..self.goods.len() {
                let p = self.hubs[h].production.get(g).copied().unwrap_or(0.0);
                if p > bg.1 { bg = (g, p); }
            }
            if bg.0 != usize::MAX { self.goods.get(bg.0).map(|x| x.name.clone()) } else { None }
        };
        crate::sim::names::gen_guild_name(
            x, y, self.world_w as u32, self.world_h(), &city, specialty.as_deref(), h as u64 ^ 0x6111)
    }

    fn update_house_dynamics(&mut self, needs: &[Vec<f32>]) {
        let tick = self.tick;
        // Decay recent-volume so monopoly tracks the last while, not all history.
        // Per-hub trade ties decay more slowly (an office relationship is built and
        // lost over months, not days).
        for hh in &mut self.houses {
            if !hh.defunct {
                hh.volume *= 0.98;
                for e in &mut hh.trade_at { e.1 *= 0.997; }
                hh.trade_at.retain(|(_, v)| *v > 0.01);
            }
        }
        // Succession: the head dies at the end of their lifespan; an heir takes over.
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let h = &self.houses[hi];
            if h.head_lifespan > 0 && tick.saturating_sub(h.head_since) >= h.head_lifespan {
                self.succeed_house(hi);
            }
        }
        // Per tick: a new house may appear (probabilistic — see maybe_found_house).
        self.maybe_found_house();
        // Monthly: monopolies, political power, branching, extinction, feuds.
        if tick % 30 == 0 {
            // Merchant bankers' capital earns interest each month.
            for hh in &mut self.houses {
                if !hh.defunct && hh.archetype == ARCH_BANKING && hh.wealth > 0.0 {
                    hh.wealth *= 1.0 + BANK_INTEREST;
                }
            }
            // Phase 2: ensure/stock/expand house warehouses BEFORE upkeep, so the
            // capacity-scaled upkeep below sees each house's current depots.
            self.sync_and_stock_warehouses(needs);
            // Phase G: wealth bleeds (upkeep + consumption) so it plateaus and some
            // flows to the people — runs right after interest so it offsets it.
            self.apply_wealth_sinks();
            self.pay_to_regain_markets();
            self.recompute_monopolies_and_power();
            self.manage_fleets();
            self.update_structures();
            self.fund_public_works();
            self.maybe_branch_houses();
            self.maybe_house_invests();
            self.update_guilds_and_offices();
            // Offices are (re)settled above → now offer futures contracts from them.
            self.form_contracts(needs);
            self.update_solvency();
        }
        if tick % 180 == 0 { self.update_rivalries(); }
    }

    /// Heir succession: new generation, a freshly-named head, a new lifespan, and
    /// occasionally the house splits a branch off into another city.
    fn succeed_house(&mut self, hi: usize) {
        let tick = self.tick;
        let gen = self.houses[hi].generation + 1;
        let hub = self.houses[hi].hub as usize;
        let name = self.houses[hi].name.clone();
        let heir = self.head_name_for(hub, &name, gen as u64 ^ 0x5151);
        let lifespan = self.roll_lifespan(hi as u64 ^ gen as u64);
        {
            let h = &mut self.houses[hi];
            h.generation = gen;
            h.head_name = heir.clone();
            h.head_since = tick;
            h.head_lifespan = lifespan;
            h.prestige += 0.05;
            h.events.push(HouseEvent {
                tick, kind: "succession".into(),
                text: format!("{} succeeds as head (generation {})", heir, gen),
            });
        }
        self.journal.push(JournalEntry {
            tick, kind: "succession".into(), hub: hub as i32, good: -1,
            value: self.houses[hi].generation as f32,
            text: format!("{} succeeds as head of {}", heir, name),
        });
        // A wealthy house founds a cadet BRANCH in a city it trades with. Lowered
        // to gen>=2 (was gen>=3 ≈ 150+ yrs, which essentially never happened in a
        // normal playthrough). Periodic branching also runs monthly (see
        // maybe_branch_houses), so expansion no longer depends solely on a death.
        if self.houses[hi].wealth > HOUSE_BRANCH_WEALTH && gen >= 2 {
            if let Some(dest) = self.pick_branch_hub(hub) {
                let parent = name.clone();
                self.found_branch(hi, dest, parent);
            }
        }
    }

    /// A branch name that KEEPS the family identity: "House Cassii of <City>", so
    /// the same family visibly spreads across cities (instead of inventing an
    /// unrelated surname). Unique per city.
    fn branch_name_for(&self, parent_name: &str, dest: usize) -> String {
        let surname = parent_name.strip_prefix("House ").unwrap_or(parent_name);
        let base = surname.split(" of ").next().unwrap_or(surname).trim();
        let city = self.hubs[dest].name.clone();
        let cand = format!("House {} of {}", base, city);
        if !self.houses.iter().any(|h| h.name == cand) { return cand; }
        format!("House {} of {} [{}]", base, city, self.tick)
    }

    /// Split a cadet branch of house `hi` into hub `dest`, carrying ~30% of the
    /// parent's wealth and its specialties, named to keep the family identity.
    fn found_branch(&mut self, hi: usize, dest: usize, parent: String) {
        let tick = self.tick;
        let bname = self.branch_name_for(&parent, dest);
        // Don't stack two branches of the same family in one city.
        if self.houses.iter().any(|h| !h.defunct && h.hub as usize == dest && h.name == bname) {
            return;
        }
        let split = self.houses[hi].wealth * 0.30;
        self.houses[hi].wealth -= split;
        let spec = self.houses[hi].spec.clone();
        let bhead = self.head_name_for(dest, &bname, tick as u64 ^ hi as u64 ^ 0x9001);
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("Founded by {} as a branch of {} in {}", bhead, parent, self.hubs[dest].name),
        };
        let (fleet_sea, fleet_river, fleet_caravan) =
            Self::initial_fleet(self.hubs[dest].coastal, false);
        self.houses.push(House {
            name: bname.clone(), hub: dest as u32, wealth: split, prestige: 0.1,
            spec, monopoly: vec![], rivals: vec![hi], generation: 1,
            events: vec![founded], good_profit: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: split, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: bhead.clone(), head_since: tick,
            head_lifespan: self.roll_lifespan(dest as u64 ^ 0xCC),
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: self.houses[hi].archetype, // a cadet branch keeps the family trade
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
        });
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: dest as i32, good: -1, value: 0.0,
            text: format!("{} founds a branch of {} in {}", bhead, parent, self.hubs[dest].name),
        });
    }

    /// Monthly: wealthy, established houses occasionally branch into a city they
    /// trade with — so a family network spreads across the map within a normal
    /// playthrough, not only on the rare succession that meets the old gen-3 bar.
    fn maybe_branch_houses(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; } // guilds expand via offices, not cadet branches
            if self.houses[hi].generation < 2 { continue; }
            if self.houses[hi].wealth <= HOUSE_BRANCH_WEALTH { continue; }
            // ~2.5%/month for an eligible house → a branch every few years.
            if hash01(self.seed, tick as u64 ^ 0xBA11, hi as u64) > 0.025 { continue; }
            let hub = self.houses[hi].hub as usize;
            if let Some(dest) = self.pick_branch_hub(hub) {
                let parent = self.houses[hi].name.clone();
                self.found_branch(hi, dest, parent);
            }
        }
    }

    /// A destination hub for a new branch: the house's strongest trade partner-ish
    /// — here, the nearest reachable hub in the same component that isn't home.
    fn pick_branch_hub(&self, home: usize) -> Option<usize> {
        let n = self.hubs.len();
        let comp = self.hubs[home].component;
        let mut best = (usize::MAX, f32::INFINITY);
        for b in 0..n {
            if b == home || self.hubs[b].component != comp { continue; }
            let d = self.days.get(home * n + b).copied().unwrap_or(f32::INFINITY);
            if d.is_finite() && d > 1.0 && d < best.1 { best = (b, d); }
        }
        if best.0 == usize::MAX { None } else { Some(best.0) }
    }

    /// A starting fleet for a new house, sized to its home geography: coastal
    /// seats are seafaring (ships + a caravan), inland ones overland (caravans +
    /// a river boat). `big` gives the seeded great houses a slightly larger fleet.
    fn initial_fleet(coastal: bool, big: bool) -> (u32, u32, u32) {
        match (coastal, big) {
            (true, true) => (2, 0, 1),
            (true, false) => (1, 0, 1),
            (false, true) => (0, 1, 2),
            (false, false) => (0, 1, 1),
        }
    }

    /// A lost voyage sometimes takes the vessel/caravan with it (~30%).
    fn damage_fleet(&mut self, hi: usize, sea: bool) {
        if hash01(self.seed, self.tick as u64 ^ 0xDEAD, hi as u64) > 0.30 { return; }
        let h = &mut self.houses[hi];
        if sea {
            if h.fleet_sea > 0 { h.fleet_sea -= 1; }
        } else if h.fleet_caravan > 0 {
            h.fleet_caravan -= 1;
        } else if h.fleet_river > 0 {
            h.fleet_river -= 1;
        }
    }

    /// Monthly fleet management: a profitable house whose vessels are all busy
    /// buys another (more capacity → more trade carried → more market share); a
    /// failing house with idle ships scraps one for a little cash. This capital
    /// churn — build-up, over-extension, loss, recovery — keeps the trade network
    /// perpetually shifting instead of settling into a static equilibrium.
    fn manage_fleets(&mut self) {
        let tick = self.tick;
        let nh = self.houses.len();
        let mut used_sea = vec![0i32; nh];
        let mut used_land = vec![0i32; nh];
        for c in &self.in_transit {
            if c.owner >= 0 {
                let oi = c.owner as usize;
                if oi < nh { if c.sea { used_sea[oi] += 1; } else { used_land[oi] += 1; } }
            }
        }
        for hi in 0..nh {
            if self.houses[hi].defunct { continue; }
            // Phase G: fleet upkeep (a steady sink scaling with fleet size) + slow
            // decay (an occasional vessel lost to wear), so a big fleet costs money
            // to keep and must be continually rebuilt.
            let fleet_total =
                self.houses[hi].fleet_sea + self.houses[hi].fleet_river + self.houses[hi].fleet_caravan;
            if fleet_total > 0 {
                let fleet_cost = fleet_total as f32 * SHIP_COST * FLEET_UPKEEP_FRAC;
                self.houses[hi].wealth -= fleet_cost;
                if hi < self.house_ledger.len() {
                    self.house_ledger[hi].fleet_cost += fleet_cost;
                }
                if hash01(self.seed, tick as u64 ^ 0x5EA1, hi as u64)
                    < FLEET_DECAY_CHANCE * fleet_total as f32
                {
                    if self.houses[hi].fleet_sea > 0 {
                        self.houses[hi].fleet_sea -= 1;
                    } else if self.houses[hi].fleet_caravan > 0 {
                        self.houses[hi].fleet_caravan -= 1;
                    } else if self.houses[hi].fleet_river > 0 {
                        self.houses[hi].fleet_river -= 1;
                    }
                }
            }
            let coastal = self.hubs.get(self.houses[hi].hub as usize).map(|x| x.coastal).unwrap_or(false);
            let w = self.houses[hi].wealth;
            let sea_slots = self.houses[hi].fleet_sea as i32;
            let land_slots = (self.houses[hi].fleet_river + self.houses[hi].fleet_caravan) as i32;
            let sea_busy = used_sea[hi] >= sea_slots;
            let land_busy = used_land[hi] >= land_slots;
            // Shipping dynasties build vessels at a discount.
            let disc = if self.houses[hi].archetype == ARCH_FLEET { FLEET_SHIP_DISCOUNT } else { 1.0 };
            // BUY: capital to spare and every vessel of the favoured kind is busy.
            if coastal && sea_busy && w > SHIP_COST * 2.5 {
                self.houses[hi].wealth -= SHIP_COST * disc;
                self.houses[hi].fleet_sea += 1;
            } else if !coastal && land_busy && w > CARAVAN_COST * 2.5 {
                if hash01(self.seed, tick as u64 ^ 0x21B0, hi as u64) < 0.30 {
                    self.houses[hi].wealth -= RIVER_COST * disc;
                    self.houses[hi].fleet_river += 1;
                } else {
                    self.houses[hi].wealth -= CARAVAN_COST * disc;
                    self.houses[hi].fleet_caravan += 1;
                }
            } else if w < HOUSE_BRANCH_WEALTH * 0.15 {
                // SELL: a struggling house with an idle vessel scraps it for cash.
                if used_sea[hi] < sea_slots && self.houses[hi].fleet_sea > 0 {
                    self.houses[hi].fleet_sea -= 1;
                    self.houses[hi].wealth += SHIP_COST * 0.4;
                } else if used_land[hi] < land_slots {
                    if self.houses[hi].fleet_caravan > 0 {
                        self.houses[hi].fleet_caravan -= 1;
                        self.houses[hi].wealth += CARAVAN_COST * 0.4;
                    } else if self.houses[hi].fleet_river > 0 {
                        self.houses[hi].fleet_river -= 1;
                        self.houses[hi].wealth += RIVER_COST * 0.4;
                    }
                }
            }
        }
    }

    /// Recompute each house's per-good monopoly shares (volume among houses that
    /// specialize in the good) and its political power (wealth + monopoly +
    /// prestige). The dominant house concentrates a little wealth into its home
    /// city's commercial prosperity.
    fn recompute_monopolies_and_power(&mut self) {
        let ng = self.goods.len();
        let tick = self.tick;
        let nhubs = self.hubs.len();
        // Volume per good (across speccing houses) + per-hub resident-house volume
        // and the strongest resident (for >=50% city control).
        let mut good_vol = vec![0.0f32; ng];
        let mut hub_vol = vec![0.0f32; nhubs];
        let mut hub_top = vec![(usize::MAX, 0.0f32); nhubs];
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct { continue; }
            for &g in &hh.spec {
                if g < ng { good_vol[g] += hh.volume; }
            }
            let hub = hh.hub as usize;
            if hub < nhubs {
                let v = hh.volume.max(0.0001);
                hub_vol[hub] += v;
                if v > hub_top[hub].1 { hub_top[hub] = (hi, v); }
            }
        }
        let wmax = self.houses.iter().filter(|h| !h.defunct)
            .map(|h| h.wealth).fold(1.0f32, f32::max);
        let pmax = self.houses.iter().filter(|h| !h.defunct)
            .map(|h| h.prestige).fold(0.5f32, f32::max);
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let mut mono: Vec<(usize, f32)> = Vec::new();
            let mut top_share = 0.0f32;
            let mut shares: Vec<(usize, f32)> = Vec::new();
            let (spec, vol) = (self.houses[hi].spec.clone(), self.houses[hi].volume);
            for &g in &spec {
                if g >= ng { continue; }
                let share = if good_vol[g] > 1e-3 { (vol / good_vol[g]).clamp(0.0, 1.0) } else { 0.0 };
                if share > 0.25 { mono.push((g, share)); }
                top_share = top_share.max(share);
                shares.push((g, share));
            }
            // ── Monopoly milestones with HYSTERESIS ──────────────────────────
            // A monopoly is WON when share first reaches >=50% (recorded once);
            // it's only LOST when share falls below 10% (a genuine collapse, not
            // noise around the 50% line); a later re-win reads "regained". This
            // kills the per-month "won a monopoly" spam.
            let mut held = std::mem::take(&mut self.houses[hi].mono50);
            let mut ever = std::mem::take(&mut self.houses[hi].mono_ever);
            for &(g, share) in &shares {
                let is_held = held.contains(&g);
                if share >= 0.5 && !is_held {
                    let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                    let regained = ever.contains(&g);
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "monopoly".into(),
                        text: if regained { format!("Regained the monopoly on {}", gn) }
                              else { format!("Won a monopoly on {}", gn) },
                    });
                    held.push(g);
                    if !regained { ever.push(g); }
                } else if share < 0.10 && is_held {
                    let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                    self.houses[hi].events.push(HouseEvent {
                        tick, kind: "monopoly_lost".into(),
                        text: format!("Lost the monopoly on {}", gn),
                    });
                    held.retain(|&x| x != g);
                }
            }
            self.houses[hi].mono50 = held;
            self.houses[hi].mono_ever = ever;
            let wn = (self.houses[hi].wealth.max(0.0) / wmax).clamp(0.0, 1.0);
            let pn = (self.houses[hi].prestige / pmax).clamp(0.0, 1.0);
            // Political houses wield extra influence in their city's council.
            let arch_bonus = if self.houses[hi].archetype == ARCH_POLITICAL { POLITICAL_POWER_BONUS } else { 0.0 };
            let power = (0.45 * wn + 0.35 * top_share + 0.20 * pn + arch_bonus).clamp(0.0, 1.0);
            self.houses[hi].monopoly = mono;
            self.houses[hi].political_power = power;

            // Control of the seat city (>=50% of its resident-house trade).
            let hub = self.houses[hi].hub as usize;
            let now_dom = hub < nhubs && hub_top[hub].0 == hi
                && (hub_top[hub].1 / hub_vol[hub].max(1e-6)) >= 0.5;
            if now_dom != self.houses[hi].dominant_seat {
                let cn = self.hubs.get(hub).map(|x| x.name.clone()).unwrap_or_default();
                let (kind, text) = if now_dom {
                    ("control_gained", format!("Gained control of {}", cn))
                } else {
                    ("control_lost", format!("Lost control of {}", cn))
                };
                self.houses[hi].events.push(HouseEvent { tick, kind: kind.into(), text });
                self.houses[hi].dominant_seat = now_dom;
            }
            // Settlement grant: a POLITICAL house that controls its seat is granted a
            // city CHARTER on its specialty goods — a standing rent monopoly.
            if now_dom && self.houses[hi].archetype == ARCH_POLITICAL {
                let spec = self.houses[hi].spec.clone();
                let cn = self.hubs.get(hub).map(|x| x.name.clone()).unwrap_or_default();
                for g in spec {
                    if !self.houses[hi].charters.contains(&g) {
                        self.houses[hi].charters.push(g);
                        let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                        self.houses[hi].events.push(HouseEvent {
                            tick, kind: "charter".into(),
                            text: format!("{} grants a charter on {}", cn, gn),
                        });
                    }
                }
            }

            // Worst single-month loss (a sharp wealth fall — rivals, embargo, crash).
            let prev = self.houses[hi].prev_wealth;
            let drop = prev - self.houses[hi].wealth;
            if drop > 2.0 && drop > self.houses[hi].worst_loss {
                self.houses[hi].worst_loss = drop;
                self.houses[hi].events.push(HouseEvent {
                    tick, kind: "loss".into(),
                    text: format!("Its most devastating loss — {:.0} wealth lost in a month", drop),
                });
            }
            self.houses[hi].prev_wealth = self.houses[hi].wealth;
        }
    }

    /// A booming merchant city with no strong resident house occasionally spawns a
    /// new trading family.
    fn maybe_found_house(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        // Candidate: the richest-trade hub with ROOM for a new family (no strong
        // resident house, and fewer than 2 nascent ones so we don't stack). Also
        // track the world's max trade wealth to flag "large" hubs.
        let mut best = (usize::MAX, 0.0f32);
        let mut max_tw = 1e-6f32;
        for h in 0..self.hubs.len() {
            max_tw = max_tw.max(self.hubs[h].trade_wealth);
            let tw = self.hubs[h].trade_wealth;
            if tw <= 0.05 { continue; } // any hub with a little trade can seed a family
            let mut strongest = 0.0f32;
            let mut count = 0u32;
            for hs in self.houses.iter().filter(|hs| !hs.defunct && hs.hub as usize == h) {
                strongest = strongest.max(hs.wealth);
                count += 1;
            }
            // Room for a new family: no overwhelmingly dominant house yet, and a
            // big hub can host a couple of rivals (small ones just one).
            let cap = if tw >= 0.5 * max_tw { 3 } else { 2 };
            if strongest > 8.0 || count >= cap { continue; }
            if tw > best.1 { best = (h, tw); }
        }
        let Some(hub) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // ── Probabilistic founding (per tick) ────────────────────────────────
        // A house does NOT auto-appear just because a city is guild-run. Per tick:
        //   • below the seeded baseline → 10% (the world repopulates its houses)
        //   • a large trade hub (>=50% of the richest hub's trade) → 5%
        //   • otherwise → 2%
        let active = self.houses.iter().filter(|h| !h.defunct).count() as u32;
        let baseline = if self.seed_house_count > 0 { self.seed_house_count } else { 24 };
        let large = best.1 >= 0.5 * max_tw;
        let prob = if active < baseline { 0.10 } else if large { 0.05 } else { 0.02 };
        if hash01(self.seed, tick as u64 ^ 0xF0F0, hub as u64) > prob { return; }
        // Specialty = the hub's top-2 produced goods.
        let mut gi: Vec<usize> = (0..ng).collect();
        gi.sort_by(|&a, &b| self.hubs[hub].production[b]
            .partial_cmp(&self.hubs[hub].production[a]).unwrap_or(std::cmp::Ordering::Equal));
        let spec: Vec<usize> = gi.into_iter().filter(|&g| self.hubs[hub].production[g] > 0.0)
            .take(2).collect();
        if spec.is_empty() { return; }
        let name = self.unique_family_name_for(hub, tick as u64 ^ 0xF00D);
        let head = self.head_name_for(hub, &name, tick as u64 ^ 0x1234);
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: hub as i32, good: spec[0] as i32, value: 0.0,
            text: format!("{} establishes {} on the {} trade", head, name,
                self.goods.get(spec[0]).map(|g| g.name.as_str()).unwrap_or("local")),
        });
        let founded = HouseEvent {
            tick, kind: "founded".into(),
            text: format!("Founded by {} in {} on the {} trade", head, self.hubs[hub].name,
                self.goods.get(spec[0]).map(|g| g.name.as_str()).unwrap_or("local")),
        };
        let (fleet_sea, fleet_river, fleet_caravan) =
            Self::initial_fleet(self.hubs[hub].coastal, false);
        self.houses.push(House {
            name, hub: hub as u32, wealth: 1.0, prestige: 0.0, spec,
            monopoly: vec![], rivals: vec![], generation: 1,
            events: vec![founded], good_profit: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: false, prev_wealth: 1.0, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: head, head_since: tick,
            head_lifespan: self.roll_lifespan(hub as u64 ^ 0x7E),
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: pick_archetype(self.seed, tick as u64 ^ hub as u64),
            charters: Vec::new(),
            is_guild: false, offices: Vec::new(), trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
        });
    }

    fn dissolve_house(&mut self, hi: usize) {
        let tick = self.tick;
        let (name, hub) = (self.houses[hi].name.clone(), self.houses[hi].hub as i32);
        self.houses[hi].defunct = true;
        self.houses[hi].political_power = 0.0;
        self.houses[hi].monopoly.clear();
        // A ruined house's depots are wound up: their stock spills back onto the
        // local market (the −1 pool, stored on the hub) and the buildings are dropped.
        for w in &self.warehouses {
            if w.owner == hi as i32 && (w.hub as usize) < self.hubs.len() {
                let h = w.hub as usize;
                for g in 0..self.hubs[h].stock.len().min(w.stock.len()) {
                    self.hubs[h].stock[g] += w.stock[g];
                }
            }
        }
        self.warehouses.retain(|w| w.owner != hi as i32);
        self.houses[hi].events.push(HouseEvent {
            tick, kind: "dissolved".into(),
            text: "Fell into ruin and was dissolved".into(),
        });
        self.journal.push(JournalEntry {
            tick, kind: "extinction".into(), hub, good: -1, value: 0.0,
            text: format!("{} falls into ruin and is dissolved", name),
        });
    }

    /// Houses that specialize in the same good and sit in the same component become
    /// rivals (competing for the same trade). A feud occasionally flares into a
    /// Chronicle event with a mutual prestige/wealth cost.
    fn update_rivalries(&mut self) {
        let n = self.houses.len();
        for a in 0..n {
            if self.houses[a].defunct { continue; }
            for b in (a + 1)..n {
                if self.houses[b].defunct { continue; }
                let shared = self.houses[a].spec.iter().any(|g| self.houses[b].spec.contains(g));
                let same_region = self.houses[a].hub == self.houses[b].hub
                    || self.hubs.get(self.houses[a].hub as usize).map(|h| h.component)
                        == self.hubs.get(self.houses[b].hub as usize).map(|h| h.component);
                if shared && same_region {
                    if !self.houses[a].rivals.contains(&b) { self.houses[a].rivals.push(b); }
                    if !self.houses[b].rivals.contains(&a) { self.houses[b].rivals.push(a); }
                    // Feud flare: the weaker pays, occasionally logged.
                    let roll = hash01(self.seed, self.tick as u64 ^ a as u64, b as u64);
                    if roll < 0.15 {
                        let (loser, winner) = if self.houses[a].wealth < self.houses[b].wealth {
                            (a, b)
                        } else { (b, a) };
                        self.houses[loser].wealth *= 0.92;
                        self.houses[winner].prestige += 0.03;
                        if roll < 0.05 {
                            let (ln, wn) = (self.houses[loser].name.clone(),
                                self.houses[winner].name.clone());
                            self.journal.push(JournalEntry {
                                tick: self.tick, kind: "feud".into(),
                                hub: self.houses[winner].hub as i32, good: -1, value: 0.0,
                                text: format!("{} outmaneuvers {} in a bitter trade feud", wn, ln),
                            });
                            // Trade war: if the winner dominates its seat city and the
                            // loser is not an embargo-immune guild, CLOSE that market to
                            // the loser until it pays to regain its rights.
                            if self.houses[winner].dominant_seat && !self.houses[loser].is_guild {
                                let city = self.houses[winner].hub;
                                let already = self.house_barred.get(loser).is_some_and(|v| v.contains(&city));
                                if !already {
                                    let cn = self.hubs.get(city as usize).map(|h| h.name.clone()).unwrap_or_default();
                                    if let Some(v) = self.house_barred.get_mut(loser) { v.push(city); }
                                    self.journal.push(JournalEntry {
                                        tick: self.tick, kind: "trade_war".into(),
                                        hub: city as i32, good: -1, value: 0.0,
                                        text: format!("{} bars {} from the market of {}", wn, ln, cn),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn sample_journal(&mut self) {
        // World price index = population-weighted mean price / base_value.
        let (_total_pop, index) = self.world_totals();
        self.journal.push(JournalEntry {
            tick: self.tick,
            kind: "price".into(),
            hub: -1,
            good: -1,
            value: index,
            text: String::new(),
        });
        // ROLLING 25-YEAR WINDOW: drop journal entries older than 25 years. The
        // journal is append-only and the WHOLE thing is (de)serialized on every
        // state fetch — past ~25 years that accumulation is the source of the lag.
        // Older ticks are simply discarded (overwritten by newer history). A hard
        // count cap stays as a safety net for event-dense ticks.
        const JOURNAL_WINDOW_TICKS: u32 = 25 * TICKS_PER_YEAR; // 25 years
        let cutoff = self.tick.saturating_sub(JOURNAL_WINDOW_TICKS);
        if self.journal.first().map_or(false, |e| e.tick < cutoff) {
            self.journal.retain(|e| e.tick >= cutoff);
        }
        if self.journal.len() > 12_000 {
            let drop = self.journal.len() - 12_000;
            self.journal.drain(0..drop);
        }
    }
}

/// Compact human-readable population (12,400 / 1.2M) for chronicle text.
fn fmt_pop(p: f32) -> String {
    let p = p.max(0.0);
    if p >= 1_000_000.0 {
        format!("{:.2}M", p / 1_000_000.0)
    } else if p >= 10_000.0 {
        format!("{:.0}k", p / 1_000.0)
    } else {
        // thousands separator for the small range
        let n = p as u64;
        let s = n.to_string();
        let bytes = s.as_bytes();
        let mut out = String::new();
        for (i, b) in bytes.iter().enumerate() {
            if i > 0 && (bytes.len() - i) % 3 == 0 { out.push(','); }
            out.push(*b as char);
        }
        out
    }
}

fn food_value(h: &TickHub, goods: &[TickGood]) -> f32 {
    let mut v = 0.0;
    for g in 0..goods.len() {
        if goods[g].food {
            v += h.stock[g] * goods[g].base_value;
        }
    }
    v / h.population.max(1.0)
}

/// Estimated merchant population at a hub, split by class (houses / local
/// merchants / guilds). A fixed fraction of the population trades for a living;
/// it is divided by each class's recent throughput share at the hub.
pub fn merchant_pops(h: &TickHub) -> (f32, f32, f32) {
    let total = h.tw_house + h.tw_local + h.tw_guild;
    let pop = h.population.max(0.0) * MERCHANT_POP_FRACTION;
    if total <= EPS {
        return (0.0, 0.0, 0.0);
    }
    (pop * h.tw_house / total, pop * h.tw_local / total, pop * h.tw_guild / total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good(name: &str, cat: i32, tier: u8, val: f32, desire: f32, food: bool) -> TickGood {
        TickGood { name: name.into(), category: cat, need_tier: tier, base_value: val, desire, food,
            fungible_input: false,
            bulk: 1.0, perishable: 0.0, inputs: vec![], labor: 1.0, consumption_interval: 30.0 }
    }

    fn hub(id: u32, x: f32, y: f32, pop: f32, prod: Vec<f32>, comp: u32) -> TickHub {
        let ng = prod.len();
        let base_per_capita: Vec<f32> = prod.iter().map(|&p| p / pop.max(1.0)).collect();
        TickHub {
            id, x, y, name: format!("H{id}"), population: pop, founding_pop: pop,
            stock: vec![0.0; ng], price: vec![1.0; ng], production: prod,
            grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: 0, coastal: false, component: comp,
            export_earn: 0.0, import_spend: 0.0,
            mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5, sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(),
            in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0,
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, owner_house: -1, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
        }
    }

    fn house_at(hub: u32, spec: Vec<usize>, fleet_sea: u32) -> House {
        House {
            name: format!("House{hub}"), hub, wealth: 50.0, prestige: 0.0, spec,
            monopoly: vec![], rivals: vec![], generation: 1, events: vec![],
            good_profit: vec![], mono50: vec![], mono_ever: vec![], dominant_seat: false,
            prev_wealth: 50.0, worst_loss: 0.0, fleet_sea, fleet_river: 0, fleet_caravan: 0,
            head_name: "Head".into(), head_since: 0, head_lifespan: 100_000, founded_tick: 0,
            political_power: 0.0, volume: 0.0, defunct: false, archetype: 1, charters: vec![],
            is_guild: false, offices: vec![], trade_at: vec![], debt_since: 0,
            wealth_history: vec![], office_leases: vec![],
        }
    }

    fn sim(hubs: Vec<TickHub>, goods: Vec<TickGood>) -> CampaignSim {
        let mut s = CampaignSim {
            seed: 42, tick: 0, goods, hubs, in_transit: vec![], houses: vec![],
            active_events: vec![], journal: vec![], days_per_cell: 0.2, freight_per_day: 0.01,
            k: 0.6, margin: 0.05, need_scale: 1.0, world_w: 100.0, world_h: 100.0, last_tick_ms: 0.0,
            last_month_pop: 0.0, last_month_index: 0.0, seed_house_count: 0,
            fleets_migrated: true, tech_factor: 1.0, percap_migrated: true,
            house_ledger: Vec::new(), house_ledger_prev: Vec::new(), house_barred: Vec::new(),
            colonizable: vec![],
            diag_shipments: 0, diag_by_house: 0, diag_by_guild: 0, diag_lost: 0, diag_volume: 0.0,
            recent_trades: vec![],
            spec_centers: vec![], spec_year: 0, spec_prev_profit: vec![],
            days: vec![],
            neighbors: vec![],
            routes_dirty: false,
            warehouses: vec![],
            contracts: vec![],
        };
        s.rebuild_routes();
        s
    }

    #[test]
    fn deterministic_and_finite() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![50.0, 5.0], 0),
            hub(1, 40.0, 12.0, 8000.0, vec![40.0, 0.0], 0),
        ];
        let mut a = sim(hubs.clone(), goods.clone());
        let mut b = sim(hubs, goods);
        a.advance(365);
        b.advance(365);
        for h in 0..a.hubs.len() {
            for g in 0..a.goods.len() {
                assert!(a.hubs[h].price[g].is_finite() && a.hubs[h].price[g] > 0.0);
                assert!((a.hubs[h].price[g] - b.hubs[h].price[g]).abs() < 1e-3, "determinism");
            }
        }
    }

    #[test]
    fn speculation_runs_yearly_and_is_deterministic() {
        // DLC 3 · the yearly polis-policy + speculation passes must run inside
        // `advance`, stay finite/in-range, and be reproducible across two runs.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("amber", 1, 2, 14.0, 0.30, false),
        ];
        let mk = || {
            let hubs = vec![
                hub(0, 10.0, 10.0, 12000.0, vec![60.0, 6.0, 4.0], 0),
                hub(1, 40.0, 12.0, 9000.0, vec![45.0, 1.0, 0.5], 0),
                hub(2, 18.0, 38.0, 7000.0, vec![30.0, 0.5, 3.0], 0),
            ];
            let mut s = sim(hubs, goods.clone());
            for i in 0..3u32 { s.houses.push(house_at(i, vec![(i as usize) % 3], 2)); }
            s.rebuild_routes();
            s
        };
        let mut a = mk();
        let mut b = mk();
        a.advance(800); // > 2 years → at least two yearly speculation passes
        b.advance(800);
        assert_eq!(a.spec_year, b.spec_year, "speculation year reproducible");
        assert!(a.spec_year >= 2, "at least two yearly passes ran");
        assert_eq!(a.spec_centers.len(), b.spec_centers.len(), "centers reproducible");
        for c in &a.spec_centers {
            assert!(c.risk.is_finite() && (0.0..=1.0).contains(&c.risk), "risk in range");
            assert!((1..=5).contains(&c.stars));
            assert!(!c.drivers.is_empty(), "a scored polis has a reason-chain");
            // drivers are ranked largest-weight first
            for w in c.drivers.windows(2) { assert!(w[0].weight >= w[1].weight - 1e-6); }
        }
        // The polis agent set per-city tariffs (council policy ran).
        assert!(a.hubs.iter().any(|h| h.tariff_export > 0.0), "a council set a tariff");
    }

    #[test]
    fn cutting_food_starves_a_dependent_hub() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Hub 1 grows no food and is in a SEPARATE component (no route in).
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![200.0], 0),
            hub(1, 80.0, 50.0, 10000.0, vec![0.0], 1),
        ];
        let mut s = sim(hubs, goods);
        s.advance(400);
        assert!(s.hubs[1].starving > 0.5, "isolated foodless hub starves: {}", s.hubs[1].starving);
        assert!(s.hubs[1].population < s.hubs[1].founding_pop, "population declines");
    }

    #[test]
    fn idle_house_pays_upkeep_and_goes_bankrupt() {
        // Hub 1 is isolated (its own component) so a house there can NEVER trade or
        // earn — it must still pay warehouse upkeep every month, slide into debt,
        // and after a year in the red be dissolved.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 80.0, 50.0, 10000.0, vec![100.0], 1),
        ];
        let mut s = sim(hubs, goods);
        let mut h = house_at(1, vec![0], 0); // fleetless, isolated
        h.wealth = 0.5;
        h.prev_wealth = 0.5;
        s.houses.push(h);
        // One month: upkeep is charged even though no trade happened.
        s.advance(30);
        assert!(s.houses[0].wealth < 0.5,
            "an idle house still pays upkeep (wealth {})", s.houses[0].wealth);
        // Years on: it falls into debt and a full year in the red bankrupts it.
        s.advance(30 * 40);
        assert!(s.houses[0].defunct,
            "a house a year in debt is dissolved (wealth {}, debt_since {})",
            s.houses[0].wealth, s.houses[0].debt_since);
    }

    #[test]
    fn production_scales_with_population() {
        // Two hubs with the SAME per-capita rate but 2× population: the bigger one
        // must produce ~2× as much (the core fix — output tracks live population).
        let goods = vec![good("iron", i32::MAX, 1, 5.0, 0.4, false)]; // non-food → no season
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 1000.0, vec![10.0], 0), // percap 0.01
            hub(1, 12.0, 10.0, 2000.0, vec![20.0], 0), // percap 0.01
        ], goods);
        s.advance(1);
        let (p0, p1) = (s.hubs[0].production[0], s.hubs[1].production[0]);
        assert!(p0 > 0.0 && (p1 / p0 - 2.0).abs() < 0.05, "double pop ⇒ ~double output: {p0} {p1}");
    }

    #[test]
    fn big_city_is_a_net_importer() {
        // A populous, food-poor city wired to a small food-rich one must IMPORT food
        // (regression for "large cities show 0 trade"): production no longer keeps
        // pace with a grown population, so the metropolis pulls in food.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 20000.0, vec![100.0], 0),  // huge pop, tiny per-capita food
            hub(1, 14.0, 10.0, 2000.0, vec![4000.0], 0),  // small pop, big surplus
        ], goods);
        s.advance(120);
        assert!(s.hubs[0].import_spend > 0.0, "big city imports food: {}", s.hubs[0].import_spend);
    }

    #[test]
    fn tiny_hub_wealth_stays_bounded() {
        // Regression for the "millionaire outpost": a tiny-population luxury exporter
        // can no longer accumulate absurd per-capita trade wealth, because its output
        // scales with its small population and the wealth denominator is floored.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 8000.0, vec![60.0, 0.0], 0),
            hub(1, 13.0, 10.0, 60.0, vec![0.0, 30.0], 0), // tiny pop, makes luxury
        ], goods);
        s.advance(365 * 3);
        assert!(s.hubs[1].trade_wealth < 1000.0, "tiny hub wealth bounded: {}", s.hubs[1].trade_wealth);
    }

    #[test]
    fn hemispheres_harvest_opposite_seasons() {
        // North and south hubs harvest half a year apart, so the world is never
        // short everywhere at once. At the northern harvest peak (~day 230) the
        // north out-produces the (then-troughing) south, and the seasonal swing is
        // strong at high latitude. (Seasonal ratio ~2× dominates the ±15% fertile-
        // year noise, so the inequality is robust.)
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let s = sim(vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0), // y=10 of 100 → far north
            hub(1, 14.0, 90.0, 10000.0, vec![100.0], 0), // y=90 of 100 → far south
        ], goods);
        let north_peak = s.seasonal_mult(0, 0, 230);
        let south_then = s.seasonal_mult(1, 0, 230);
        assert!(north_peak > south_then,
            "north harvests while south troughs: {north_peak} vs {south_then}");
        // Same hub, half a year apart: a strong seasonal swing at high latitude.
        let peak = s.seasonal_mult(0, 0, 230);
        let trough = s.seasonal_mult(0, 0, 230 - 182);
        assert!(peak > trough * 1.25, "high-latitude harvest swings hard: {peak} vs {trough}");
    }

    #[test]
    fn food_surplus_prevents_famine_collapse() {
        // Regression for the 8M→1M famine collapse. A connected world whose hubs are
        // each seeded with ~1.5× their food need (the seed-time food-surplus
        // guarantee) must NOT slide into world-wide famine over 5 years, even with
        // seasonal harvest troughs. Many hubs spread over latitudes both dilute
        // plague (which strikes one random hub) and let opposite hemispheres trade
        // across each other's lean seasons. We assert on the famine signals (no
        // sustained world-wide starvation) plus a soft population floor — plague
        // attrition is allowed, a food death-spiral is not.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let pop = 10000.0f32;
        let need = pop * 0.85 * DEMAND_PRESSURE; // tier_w[0]=1, need_scale=1 in test sim
        let prod = need * 1.5; // mirror the seed-time food surplus
        // 10 hubs in ONE component, spread across the whole map (x 5..95) and fanned
        // north→south (y 8..84, both hemispheres) — like a real world where a
        // regional drought (radius = 12% of width) hits a neighbour or two, not the
        // entire civilisation at once.
        let mut hubs = Vec::new();
        for i in 0..10u32 {
            let x = 5.0 + i as f32 * 10.0;
            let y = 8.0 + i as f32 * 8.4;
            hubs.push(hub(i, x, y, pop, vec![prod, 2.0], 0));
        }
        let mut s = sim(hubs, goods);
        let start: f32 = s.hubs.iter().map(|h| h.population).sum();
        s.advance(365 * 5);
        let end: f32 = s.hubs.iter().map(|h| h.population).sum();
        let mean_starving: f32 =
            s.hubs.iter().map(|h| h.starving).sum::<f32>() / s.hubs.len() as f32;
        assert!(mean_starving < 0.25, "world is not in famine: mean starving {mean_starving}");
        assert!(end > start * 0.6, "no famine collapse: {start:.0} → {end:.0}");
    }

    #[test]
    fn round_trip_earns_on_both_legs() {
        // A house at coastal hub A exports silk to coastal hub B, then carries B's
        // wine home and sells it — profit on BOTH goods proves the round trip:
        // outbound silk (A→B) AND a return cargo of wine (B→A).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5; // food at a healthy surplus
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); // silk surplus
        ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); // wine surplus
        hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.advance(400);
        let prof = &s.houses[0].good_profit;
        assert!(prof.get(1).copied().unwrap_or(0.0) > 0.0,
            "house earns on the outbound silk leg: {prof:?}");
        assert!(prof.get(2).copied().unwrap_or(0.0) > 0.0,
            "house earns on the return wine leg (round trip): {prof:?}");
    }

    #[test]
    fn guild_appears_only_in_large_cities() {
        // A city ≥ 50k people charters a civic Merchant Guild; a small town doesn't.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let big = hub(0, 10.0, 10.0, 60_000.0, vec![60_000.0 * 0.85 * DEMAND_PRESSURE * 1.5], 0);
        let small = hub(1, 40.0, 12.0, 9_000.0, vec![9_000.0 * 0.85 * DEMAND_PRESSURE * 1.5], 0);
        let mut s = sim(vec![big, small], goods);
        s.seed_initial_guilds();
        assert!(s.houses.iter().any(|h| h.is_guild && h.hub == 0), "big city charters a guild");
        assert!(!s.houses.iter().any(|h| h.is_guild && h.hub == 1), "small town has no guild");
    }

    #[test]
    fn house_opens_office_at_a_trade_partner() {
        // A house that trades steadily between its home A and partner B eventually
        // opens an office in B (its expansion mechanism).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.advance(800);
        assert!(s.houses[0].offices.contains(&1),
            "house opens an office in its trade partner B: {:?}", s.houses[0].offices);
    }

    #[test]
    fn rich_house_invests_in_estates() {
        // A profitable house should spend its hoarded capital building estates /
        // manufactories instead of letting wealth pile up forever.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.advance(365 * 4);
        let owned = s.hubs.iter().filter(|h| h.is_estate && h.owner_house == 0).count();
        assert!(owned >= 1, "a profitable house builds at least one estate/manufactory (owned={owned})");
    }

    #[test]
    fn warehouses_aggregate_into_hub_stock() {
        // Phase 1 scaffolding: with no house warehouses, hub_stock equals the
        // inline local-merchant pool (behaviour-preserving). A house depot's stock
        // then adds into the aggregate that prices & needs read.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let mut s = sim(vec![hub(0, 10.0, 10.0, 10000.0, vec![50.0, 5.0], 0)], goods);
        s.hubs[0].stock = vec![100.0, 0.0];
        // Empty warehouses → aggregate == the pool.
        assert_eq!(s.hub_stock(0, 0), 100.0);
        assert_eq!(s.hub_stock(0, 1), 0.0);
        // A house depot sited here adds its owned stock into the aggregate.
        s.warehouses.push(Warehouse {
            owner: 0, hub: 0, capacity: 1_000.0,
            stock: vec![50.0, 20.0], tier: CampaignSim::capacity_tier(1_000.0), damage: 0.0,
        });
        assert_eq!(s.hub_stock(0, 0), 150.0);
        assert_eq!(s.hub_stock(0, 1), 20.0);
        // Tier bands.
        assert_eq!(CampaignSim::capacity_tier(0.0), 0);   // uncapped −1 pool
        assert_eq!(CampaignSim::capacity_tier(500.0), 1); // Depot
        assert_eq!(CampaignSim::capacity_tier(1_000.0), 2); // Storehouse
        assert_eq!(CampaignSim::capacity_tier(6_000.0), 4); // Entrepôt
        assert_eq!(CampaignSim::capacity_tier(12_000.0), 5); // Grand Entrepôt
    }

    #[test]
    fn house_auto_builds_and_stocks_a_home_depot() {
        // Phase 2: a live house auto-builds a home warehouse and draws a slice of its
        // specialty good's local surplus into it (inventory it can later contract out).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)]; // specializes in silk
        s.advance(365 * 2);
        assert!(s.warehouses.iter().any(|w| w.owner == 0 && w.hub == 0 && w.tier >= 1),
            "house auto-builds a home depot: {:?}",
            s.warehouses.iter().map(|w| (w.owner, w.hub, w.tier, w.capacity)).collect::<Vec<_>>());
        let owned_silk: f32 = s.warehouses.iter().filter(|w| w.owner == 0).map(|w| w.stock[1]).sum();
        assert!(owned_silk > 0.0, "house stocks its specialty silk into the depot: {owned_silk}");
    }

    #[test]
    fn contract_term_gate_scales_with_record() {
        // Phase 3: the term a house may offer is gated by its unbroken growth record:
        // 1yr always · 3yr ≥4 stable yrs · 5yr ≥7 · 7yr >10.
        let mut s = sim(vec![hub(0, 10.0, 10.0, 5000.0, vec![1.0], 0)],
            vec![good("wheat", 0, 0, 1.0, 0.85, true)]);
        s.houses = vec![house_at(0, vec![0], 0)];
        s.houses[0].wealth_history = vec![]; // young → 1yr
        assert_eq!(s.max_term_index(0), 0);
        s.houses[0].wealth_history = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 4 growth yrs → 3yr
        assert_eq!(s.max_term_index(0), 1);
        s.houses[0].wealth_history = (1..=9).map(|i| i as f32).collect(); // 8 → 5yr
        assert_eq!(s.max_term_index(0), 2);
        s.houses[0].wealth_history = (1..=12).map(|i| i as f32).collect(); // 11 → 7yr
        assert_eq!(s.max_term_index(0), 3);
        // A decline breaks the run.
        s.houses[0].wealth_history = vec![1.0, 2.0, 3.0, 0.5, 1.0]; // only 1 trailing growth yr
        assert_eq!(s.max_term_index(0), 0);
    }

    #[test]
    fn seated_house_forms_a_supply_contract() {
        // Phase 3: a house with an office in a city that STRUCTURALLY imports its
        // specialty good offers that city a futures contract (sourced from its home
        // depot), covering a capped slice of the city's need.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 3000.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![100.0, 0.0], 0); hb.coastal = true; // no silk
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 4); // specializes in silk
        h.offices = vec![1];                 // seated in the importer city
        h.wealth = 1000.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 3000.0,
            stock: vec![0.0, 1000.0], tier: 3, damage: 0.0 }); // home silk depot
        // hub 1 needs silk; hub 0 doesn't (it produces it).
        let needs = vec![vec![0.0, 0.0], vec![0.0, 5.0]];
        // Step past the 10%/month formation throttle.
        let mut formed = false;
        for t in 1..400u32 { s.tick = t; s.form_contracts(&needs);
            if s.contracts.iter().any(|c| c.seller_house == 0 && c.buyer_hub == 1 && c.good == 1) { formed = true; break; } }
        assert!(formed, "seated house forms a silk supply contract to the importer city");
        let c = s.contracts.iter().find(|c| c.buyer_hub == 1).unwrap();
        assert!(c.monthly_qty > 0.0 && c.monthly_qty <= CONTRACT_COVERAGE_CAP * 5.0 * 30.0 + 1.0,
            "contract volume is within the coverage cap: {}", c.monthly_qty);
    }

    #[test]
    fn a_contract_delivers_from_the_source_depot() {
        // Phase 3: an active contract reserves its monthly quantity from the seller's
        // source depot and ships it to the buyer — over months `delivered` grows and
        // the depot drains.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 4);
        h.wealth = 10_000.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 6000.0,
            stock: vec![0.0, 5000.0], tier: 4, damage: 0.0 });
        let term = 3u8;
        s.contracts.push(Contract {
            seller_house: 0, buyer_hub: 1, source_hub: 0, good: 1, monthly_qty: 50.0,
            strike_price: 25.0, term_years: term, start_tick: 0,
            end_tick: term as u32 * TICKS_PER_YEAR, delivered: 0.0,
            last_fulfilled: 0, suspended_until: 0, defaults: 0,
        });
        s.advance(150);
        let c = &s.contracts[0];
        assert!(c.delivered > 0.0, "the contract delivers silk over the months: {}", c.delivered);
        // A well-stocked depot with a fleet meets most deliveries; the rare storm
        // loss is allowed (≤ a couple over the run), it just isn't the norm.
        assert!(c.defaults <= 2, "deliveries mostly succeed (defaults={})", c.defaults);
    }

    #[test]
    fn a_contract_without_a_ship_breaches() {
        // A house with no free vessel for a due contract delivery is in logistics
        // breach — it delivers nothing and the contract takes a default strike.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 3000.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![100.0, 0.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 0); // NO sea ships
        h.fleet_river = 0; h.fleet_caravan = 0; h.wealth = 100.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 6000.0,
            stock: vec![0.0, 5000.0], tier: 4, damage: 0.0 });
        s.contracts.push(Contract {
            seller_house: 0, buyer_hub: 1, source_hub: 0, good: 1, monthly_qty: 50.0,
            strike_price: 25.0, term_years: 3, start_tick: 0,
            end_tick: 3 * TICKS_PER_YEAR, delivered: 0.0,
            last_fulfilled: 0, suspended_until: 0, defaults: 0,
        });
        // Drive one DUE delivery directly (no advance → no random plague quarantine,
        // which would force-majeure-suspend the contract instead of breaching it).
        s.tick = CONTRACT_DELIVER_DAYS;
        let needs = vec![vec![0.0, 0.0], vec![0.0, 5.0]];
        s.fulfill_contracts(&needs);
        assert_eq!(s.contracts[0].delivered, 0.0, "a shipless house delivers nothing");
        assert!(s.contracts[0].defaults >= 1, "a shipless house breaches the contract");
    }

    #[test]
    fn network_sources_a_contract_from_a_distant_node() {
        // Phase 5: a house with offices in several cities supplies a deficit office
        // from the NEAREST network node that produces the good — not just its home.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let h0 = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 0.0], 0); // home, no silk
        let h1 = hub(1, 16.0, 10.0, 5000.0, vec![100.0, 3000.0], 0); // office, makes silk
        let h2 = hub(2, 22.0, 10.0, 5000.0, vec![100.0, 0.0], 0); // office, imports silk
        let mut s = sim(vec![h0, h1, h2], goods);
        let mut h = house_at(0, vec![1], 4); // specializes in silk, home = hub 0
        h.offices = vec![1, 2];
        h.wealth = 1000.0;
        h.fleet_caravan = 4; // overland carry capacity (contracts are now sized to the fleet)
        s.houses = vec![h];
        let needs = vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 5.0]]; // hub 2 needs silk
        let mut formed = false;
        for t in 1..400u32 {
            s.tick = t;
            s.form_contracts(&needs);
            if s.contracts.iter().any(|c| c.seller_house == 0 && c.buyer_hub == 2
                && c.source_hub == 1 && c.good == 1) { formed = true; break; }
        }
        assert!(formed, "contract sourced from the silk-making node (1) to the importer office (2)");
        // Signing leased the buyer office, so it can't auto-close under the contract.
        assert!(s.office_leased(0, 2), "the contract leases the buyer office");
    }

    #[test]
    fn a_supplied_contract_survives_to_term_and_is_retired() {
        // The fix for "no contracts ever finish": a seller that meets its monthly
        // deliveries has its strike count reset each time, so it NEVER accrues the 3
        // strikes that void a contract — and at term end it is RETIRED (finished), not
        // voided. Deliveries are driven directly so the test isolates the contract
        // lifecycle from the campaign's random fire / plague events (which can burn a
        // depot or quarantine a city in a full `advance`).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 50.0], 0); ha.coastal = true;
        let mut hb = hub(1, 14.0, 10.0, 5000.0, vec![100.0, 0.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 20); // 20 ships → ample carry for a small qty
        h.wealth = 100_000.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 12000.0,
            stock: vec![0.0, 12000.0], tier: 5, damage: 0.0 });
        let term = 1u8; // a 1-year contract → end_tick = 365
        s.contracts.push(Contract {
            seller_house: 0, buyer_hub: 1, source_hub: 0, good: 1, monthly_qty: 30.0,
            strike_price: 25.0, term_years: term, start_tick: 0,
            end_tick: term as u32 * TICKS_PER_YEAR, delivered: 0.0,
            last_fulfilled: 0, suspended_until: 0, defaults: 0,
        });
        s.rebuild_routes();
        let needs = vec![vec![0.0, 0.0], vec![0.0, 5.0]]; // hub 1 needs silk
        // Step one delivery per month across the whole term, restocking the depot each
        // month as the source city would. The contract must stay alive (not void) right
        // up to the final month, then be retired when the tick crosses its end.
        let mut last_delivered = 0.0;
        for month in 1..=13u32 {
            s.tick = month * CONTRACT_DELIVER_DAYS; // 30, 60, … 390
            s.warehouses[0].stock[1] = 12000.0;     // source keeps the depot supplied
            s.fulfill_contracts(&needs);
            if s.tick < term as u32 * TICKS_PER_YEAR {
                assert_eq!(s.contracts.len(), 1, "still alive in month {month} (not voided)");
                assert!(s.contracts[0].defaults < 3, "strikes keep clearing (month {month})");
                last_delivered = s.contracts[0].delivered;
            }
        }
        assert!(last_delivered > 30.0 * 8.0, "delivered most months: {last_delivered}");
        assert!(s.contracts.is_empty(), "the contract reached term end and was retired (finished)");
    }
}
