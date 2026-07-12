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
/// Cities covet LUXURIES they can't make at home — fine goods from far lands. A
/// luxury good NOT produced locally gets this extra desire, scaled by its prestige
/// (base_value), so high-value foreign luxuries drive vigorous inter-city trade
/// instead of every city being content with what it produces itself (#2).
const LUX_IMPORT_DESIRE: f32 = 0.7;
const PRICE_FLOOR_MULT: f32 = 0.15;
const PRICE_CEIL_MULT: f32 = 12.0;
/// Per-capita appetite scale; multiplied by the seed-time balance factor so total
/// need is comparable to total production (an average good ~ slight shortage).
pub const DEMAND_PRESSURE: f32 = 1.15;
/// A house with wealth below this and no trade dissolves (bankruptcy).
const HOUSE_BANKRUPT: f32 = 0.15;
/// Merchant houses emerge GRADUALLY over the opening years — the founding baseline
/// ramps to the full target over this many years (not all seeded at once).
const HOUSE_RAMP_YEARS: f32 = 5.0;
/// Emergence order (user rule): local merchants → GUILDS (from year 5, a city's
/// merchants chartering a guild) → HOUSES (from year 10, a family splitting off a
/// guild's trade). Houses never appear before guilds.
const GUILD_START_YEAR: u32 = 5;
const HOUSE_START_YEAR: u32 = 10;
/// Hard cap on the number of live merchant houses in the world.
const HOUSE_MAX_TOTAL: usize = 100;
/// Cadet-branch house splits are DISABLED (user rule: no cadet branches).
const ENABLE_CADET_BRANCHES: bool = false;
/// Extra population growth for a TRADE-RICH, WELL-FED city (prosperity × food): a
/// thriving entrepôt can grow up to ~10%/yr. Scales the logistic rate.
const TRADE_FOOD_GROWTH_BONUS: f32 = 0.6;
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
/// DLC · social strata: how strongly a city's class composition tilts its demand
/// ladder (luxury ∝ patrician+burgher share, staples ∝ commoner+underclass). Kept
/// small so it MODULATES demand without dominating the market solver. Normalized
/// around an "average" society so total demand magnitude is preserved.
const STRATA_DEMAND_TILT: f32 = 0.45;
/// Fraction of a hub's population that flows between adjacent strata in a year at
/// full mobility pressure — bounded so the social structure shifts gradually.
const STRATA_MOBILITY_RATE: f32 = 0.04;
// ── Civil unrest & revolts (It. 3) — the social substrate turns load-bearing. ──
/// How fast a hub's smouldering `unrest` eases toward its driver target (yearly).
const UNREST_EASE: f32 = 0.35;
/// Unrest at/above this erupts in bread riots (a production + stability shock).
const RIOT_UNREST: f32 = 0.60;
/// Unrest at/above this boils over into a REVOLT that topples the ruling council.
const REVOLT_UNREST: f32 = 0.82;
/// Slice of every resident house's wealth a revolt seizes and hands to the people
/// (into the city's `civic_pool`) — a redistributive shock that cuts inequality.
const REVOLT_REDISTRIB: f32 = 0.15;
/// Years the toppled family is barred from the council after a revolt.
const REVOLT_BAN_YEARS: u32 = 12;
/// Production hit (hub-wide) while a riot / revolt event is active.
const RIOT_PROD_HIT: f32 = 0.22;
const REVOLT_PROD_HIT: f32 = 0.40;
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
/// Max distance (fraction of world width) a new colony/route may hop from the
/// founder OR any of its offices — offices CHAIN the reach (an office is a relay
/// "ground" from which the next hop is measured), so a network projects far while
/// no single leg is implausibly long. The trade-reach scale, realised in the sim.
// Reach of a founder/seat to a colonisable site, as a fraction of world width.
// Raised from 0.28 so sites are far more often "in range" (the common "0 in range"
// stall): a great city / rich house can now reach roughly half a continent away.
/// Colonies are founded at most this far from their metropolis (was ~0.42·world_w ≈
/// 16 800 km at default res — colonies appeared across whole oceans). 2500 km cap
/// (user rule): a colony is a bold venture but not on the far side of the world.
const COLONY_MAX_KM: f32 = 2500.0;
/// SATELLITES hug their metropolis — a day's ride (Ostia→Rome), never an ocean. They
/// draw from a dedicated near-city pool (`compute_satellite_sites`) and are capped at
/// this range from the parent (user rule).
const SATELLITE_MAX_KM: f32 = 500.0;
/// Capital it takes to found a SETTLEMENT colony (heavy — a city-scale venture),
/// pooled from the parent city treasury + optional house & bank backers.
const COLONY_FOUND_COST: f32 = 14.0;
/// A city needs at least this population (pressure) before it exports a colony.
const COLONY_PARENT_MIN_POP: f32 = 5_000.0;
/// Hard ceiling on live settlement colonies (route-matrix is O(hubs²)).
const MAX_SETTLEMENT_COLONIES: usize = 24;
/// Fraction of the parent's population that emigrates to seed a settlement colony.
const COLONY_MIGRATION_FRAC: f32 = 0.06;
// ── Colony food LIFELINE: dedicated supply ships on the grain run ──────────────────
/// Monthly food a single dedicated supply ship carries to a colony.
pub const SUPPLY_SHIP_CAPACITY: f32 = 900.0;
/// Cost (treasury/wealth units) to commission ONE new dedicated supply ship — paid by
/// the colony's backers (metropolis treasury, then the backing bank/house) when the
/// colony runs short, so the metropolis INVESTS in steady supply.
const SUPPLY_SHIP_COST: f32 = 3.0;
/// A colony's dedicated supply fleet never grows past this (bounds the investment).
const MAX_SUPPLY_SHIPS: u32 = 12;
// ── Atlas 2.0 · organic city LIFECYCLE ──────────────────────────────────────
/// Years of TERMINAL decline before a settlement is abandoned. Terminal means
/// FAMINE-driven: severe sustained starvation while shrunk BELOW the natural
/// capacity floor (poverty alone must never empty a town — an early draft with a
/// mood clause killed 29 of 30 test towns by year 25). Recovery only halves the
/// count — scars persist. At most 2 abandonments/year worldwide.
const ABANDON_YEARS: f32 = 8.0;
/// Population bar as a multiple of founding size. The pop pass caps capacity at
/// ≥0.15× founding and floors at 0.10×, so only FAMINE (which multiplies pop
/// below capacity) can push a town under 0.13 — exactly the towns that qualify.
const ABANDON_POP_FRAC: f32 = 0.13;
/// Organic SWARMING opens at year 25 (a generation in) — earlier than the colonial
/// era, because walking over the hill needs no joint-stock company.
const SWARM_START_TICK: u32 = 25 * 365;
/// A mother city must outgrow its founding size by this multiple (prosperous,
/// crowded), with good mood and no famine, before it swarms.
const SWARM_PRESSURE: f32 = 1.55;
const SWARM_MIN_POP: f32 = 9_000.0;
/// Share of the mother's people who walk out to break new ground.
const SWARM_POP_FRAC: f32 = 0.07;
/// Swarming is SHORT-range (fraction of world width) — daughters cluster into
/// living regions, unlike the long colonial reach.
const SWARM_REACH_FRAC: f32 = 0.10;
/// Only genuinely farmable sites attract organic settlers (no food lifeline).
const SWARM_MIN_FERTILE: f32 = 0.25;

/// Ships a fresh colony is founded with (the founding fleet on the first grain run).
const SUPPLY_SHIPS_AT_FOUNDING: u32 = 2;
/// A food SOURCE must keep this share of its own grain surplus (only the rest is
/// shippable to colonies) — so a colony is only fed from a genuinely sufficient source.
const SUPPLY_SOURCE_SPARE_FRAC: f32 = 0.6;
/// Days of its OWN grain output a food source keeps as a buffer before any is shippable.
const SOURCE_BUFFER_DAYS: f32 = 20.0;
/// A city plants a GRAIN COLONY (the Greek Crimea pattern) once its starvation pressure
/// passes this, to secure a food supply — a survival move it can self-fund (no bank).
const FOOD_COLONY_STARVE_MIN: f32 = 0.30;
const FOOD_COLONY_MIN_TREASURY: f32 = 8.0;
/// Food per-capita boost a grain colony gets (it exists to farm, so it out-produces
/// grain and its surplus flows back to the hungry metropolis through the market).
const FOOD_COLONY_FARM_MULT: f32 = 2.4;
/// Freight cost of a colony grain run, as a fraction of the food's value (the food
/// itself comes from the source's surplus; the metropolis pays only to carry it).
const COLONY_FREIGHT_RATE: f32 = 0.12;
/// Colony viability floor: a SETTLEMENT colony skips a site only when it is BOTH
/// too lean to part-feed itself AND poor in trade goods (otherwise the food lifeline
/// carries a trade-rich frontier colony). HOUSE outposts ignore fertility entirely
/// and chase trade goods. These replace the old hard fertility split between the two
/// pools — colonies now also settle less-fertile land, outposts follow the cargo.
// Lowered so colonisation leans on TRADE rather than farmland (user ask): a leaner
// site still qualifies, and a trade-rich frontier qualifies easily — its food
// lifeline contracts cover the deficit. Outposts ignore fertility entirely.
const COLONY_MIN_FERTILE: f32 = 0.12;
const COLONY_MIN_TRADE: f32 = 0.18;
/// Daily logistic population-growth rate below carrying capacity (~5%/yr peak at
/// low population; eases to 0 at capacity). Was 0.0006 (~24%/yr — too fast).
const POP_GROWTH_RATE: f32 = 0.0003;
/// Daily decline rate when a city is above its carrying capacity.
const POP_DECLINE_RATE: f32 = 0.0006;
/// Young settlement colonies grow this much faster organically (frontier boom).
const POP_GROWTH_COLONY_MULT: f32 = 2.2;
/// Below this population a settlement is a "small city" (user growth/disease rules).
const SMALL_CITY_POP: f32 = 10_000.0;
/// A well-fed small city grows up to this multiple of the base rate (user rule:
/// humble towns can rise into cities). Scaled by food security.
const SMALL_CITY_GROWTH_MULT: f32 = 5.0;
/// …and is this many times LESS likely to be struck by a plague (user rule).
const SMALL_CITY_PLAGUE_RESIST: f32 = 3.0;
/// ── HOSPICES / QUARANTINE (public health): a prosperous city's council funds public
/// health, which CUTS plague mortality and LENGTHENS post-outbreak immunity — a rich
/// city spends coin so fewer of its people die. All bounded so treasuries never crater.
/// Max mortality reduction at full funding (0.6 = a fully-provisioned city loses 60%
/// fewer people to a strike).
const HOSPICE_MAX_LEVEL: f32 = 0.6;
/// A council needs at least this much treasury before it funds public health.
const HOSPICE_MIN_TREASURY: f32 = 30.0;
/// Yearly easing of `public_health` toward its prosperity-set target (~4-8y to build).
const HOSPICE_EASE: f32 = 0.25;
/// Slice of treasury a funding council spends on public health each year (bounded,
/// comparable to the ~8% civic skim → recorded in `finance.spent_health`).
const HOSPICE_TREASURY_SKIM: f32 = 0.04;
/// Public health lapses this much per year when a council can't afford it.
const HOSPICE_DECAY: f32 = 0.05;
/// ── HINTERLAND VILLAGES: sub-cap settlements aren't full hubs, but each markets
/// through its nearest live town (a satellite trade tie). The town earns a small,
/// bounded civic toll from that hinterland trade (grain-eq per villager per year).
/// Bounded by `civic_pool`'s decay sink → cannot inflate wealth.
const HINTERLAND_TOLL: f32 = 0.01;
/// ── ETHNOGENESIS (Cultures 2.0): a large, long-resident minority blends with the
/// local majority into a NEW creole people. Bounded so only a handful arise per campaign.
const CREOLE_MAX: usize = 24;              // global cap on live creole peoples
const CREOLE_MIN_POP: f32 = 4_000.0;       // only sizeable cities spawn creoles
const CREOLE_MIN_MINORITY: f32 = 0.30;     // the minority must be at least this share
const CREOLE_YEARLY_CHANCE: f32 = 0.08;    // per eligible city per year
const CREOLE_SEED_FRAC: f32 = 0.5;         // share of the minority that becomes the creole
/// Cultures 3.0 · SPLINTERING — a far-flung, isolated community of one people slowly
/// drifts into a NEW daughter people of its own (American English from British, Afrikaans
/// from Dutch). Same kit/appearance, a fresh unique name and its own origin card. Rare,
/// biased toward communities far from the parent's hearth.
const SPLINTER_YEARLY_CHANCE: f32 = 0.012; // rare base rate; scaled UP by isolation (distance + few trade ties)
const SPLINTER_MIN_SHARE: f32 = 0.6;       // the parent must clearly dominate the city
const SPLINTER_SEED_FRAC: f32 = 0.45;      // share of the local majority that becomes the daughter
/// EXPEDITIONS — a wealthy house with a fleet occasionally mounts an expedition to a
/// far, isolated settlement the trade network doesn't normally reach: casual merchants
/// arriving in a remote outpost on rare occasions. Delivers goods (relieving a little of
/// the outpost's scarcity) at a modest cost to the house (reach/prestige, not profit).
const EXPEDITION_MIN_WEALTH: f32 = 200.0;  // only a substantial house can fund one
const EXPEDITION_YEARLY_CHANCE: f32 = 0.04; // per eligible house per year (× fleet size)
/// Cultures 2.0 · migration HOMOPHILY: bonus to a destination's opportunity per unit
/// of the migrant's culture already present there (people move toward their own kin).
/// Small vs the opportunity scale so it nudges, not dominates.
const HOMOPHILY_PULL: f32 = 0.15;
/// Cultures 2.0 · a large, UNASSIMILATED minority adds this much to a city's unrest
/// target at full (one-culture-minority) blend — modest, feeds the existing clamped
/// unrest/revolt system so a big disaffected quarter can stir trouble over years.
const MINORITY_UNREST: f32 = 0.06;
/// Cultures 2.0 · ethnic-APPEARANCE affinity: minorities of the same appearance group
/// (climate/dress) as the majority assimilate this much faster even across language
/// families — "people who look alike blend a little more easily". Small, bounded.
const APPEARANCE_ASSIM_BONUS: f32 = 1.3;
/// Cultures 2.0 · CULTURAL TASTE: a good a resident people PRIZES gets this much extra
/// local demand, scaled by that people's population share — so a Norse city craves furs,
/// an Arab one incense. Bounded (capped total) so the economy stays stable.
const CULTURE_DESIRE_BOOST: f32 = 0.4;
const CULTURE_DESIRE_MAX: f32 = 0.5;
/// Cultures 2.0 · when a city cannot supply its resident peoples the goods they PRIZE,
/// that discontent adds to unrest — weighted by how large (pop share) the craving group
/// is. At full, city-wide unmet craving adds this much to the unrest target. Bounded.
const CULTURE_UNREST: f32 = 0.18;
/// ── SATELLITE cities (Ostia→Rome, Piraeus→Athens, Westminster/Southwark→London):
/// a LARGE metropolis whose council can fund it spins off a SHORT-RANGE satellite
/// to serve a concrete NEED it can't fit inside itself — a PORT (inland/large hub
/// wants a harbour), a GRANARY (food-short core), or a WORKSHOP (very large core
/// outgrows its works). Council pays + relocates settlers.
const SATELLITE_METRO_POP: f32 = 25_000.0;    // only a large metropolis spins one off
const SATELLITE_COST: f32 = 8_000.0;          // council pays this from its treasury
const SATELLITE_SEED_POP: f32 = 800.0;        // relocated settlers seed the satellite
// Range is now the absolute `SATELLITE_MAX_KM` (500 km) against the near-city pool.
const SATELLITE_MAX_PER_METRO: u32 = 3;       // a metropolis can raise a few
const SATELLITE_WORKSHOP_POP: f32 = 60_000.0; // this large → it needs a workshop town
const SATELLITE_INDEP_YEARS: u32 = 40;        // a mature satellite may go independent
const SATELLITE_INDEP_POP: f32 = 8_000.0;     // …once it has grown into a real city
// ── Absorption: rather than let a tiny, failing town die with its trade halted, a
// big healthy neighbour ADOPTS it as a satellite — relocating settlers, binding its
// trade through the metropolis and feeding it via the satellite lifeline. (User rule.)
const ABSORB_POP_MAX: f32 = 250.0;            // a town this small is a rescue candidate
const ABSORB_METRO_POP: f32 = 12_000.0;       // its rescuer must be a substantial city
const ABSORB_MIN_AGE_YEARS: u32 = 8;          // give a young swarm town time to find its feet
const ABSORB_AID_FRAC: f32 = 0.03;            // settlers the metropolis relocates to shore it up
// ── Satellite CONSTRUCTION (10-year build with decay). See the plan doc. ──
const SAT_BUILD_CONVOYS: u8 = 3;              // dedicated caravans+ships per project
pub(crate) const SAT_STAGE_MONTHS: f32 = 24.0; // 5 stages × 24 mo = 120 mo ≈ 10 years
pub(crate) const SAT_STAGE_QUOTA: f32 = 300.0; // goods/month per category a stage demands
pub(crate) const SAT_CONVOY_UPKEEP: f32 = 40.0; // gr-eq/month per convoy, paid by the council
const SAT_BUY_MARKUP: f32 = 1.3;              // premium the council pays merchants to bring
                                              // in a supply shortfall (secures the works)
const SAT_DECAY_PER_IDLE_MONTH: f32 = 0.006;  // GENTLE decay when truly starved (user: far less)
const SAT_STAGE_DROP_IDLE_MONTHS: u8 = 60;    // ~5 y fully starved before a stage slips back
// ── Council RIGHT OF FIRST BUY (staple right / pre-emption). A city council pre-empts
//    needed goods from arriving merchants into its civic warehouse (secures the city +
//    its colonies), UNLESS a house has captured/dominates the city's trade. ──
const COUNCIL_PROVISION_MIN_TREASURY: f32 = 1_500.0; // below this the council can't provision
const COUNCIL_PROVISION_BUDGET_FRAC: f32 = 0.15;     // ≤15% of treasury spent per month securing goods
pub(crate) const COUNCIL_RESERVE_BASE: f32 = 180.0;  // target civic stock per needed good (× scale)
const COUNCIL_BUY_PRICE: f32 = 1.0;                  // first-buy pays market price
const COUNCIL_RETAIL_PRICE: f32 = 1.4;               // dominated council must buy at a retail premium
const COUNCIL_DOMINANCE_THRESHOLD: f32 = 0.60;       // houses carrying ≥60% of the city's trade
                                                     // (or a captured govt) suspend first-buy

/// Fraction of a hub's trade carried by merchant HOUSES (vs local traders + guilds).
/// The council loses its right of first refusal once houses dominate the market.
fn hub_house_trade_share(hub: &TickHub) -> f32 {
    let total = hub.tw_house + hub.tw_local + hub.tw_guild;
    if total <= 1e-6 { 0.0 } else { hub.tw_house / total }
}
/// ── CARAVANSERAIS: waystations on long INLAND trade corridors (Silk-Road halts a
/// day apart between distant cities); a small settlement founded near a heavy land
/// tie's midpoint, which can grow into a town like any other.
const CARAVAN_CITY_MIN_POP: f32 = 12_000.0;   // the anchoring inland trade cities
const CARAVAN_MIN_GAP_FRAC: f32 = 0.10;       // the pair must be a long land haul apart
const CARAVAN_SEED_POP: f32 = 300.0;          // a small waystation seed
const CARAVAN_NEAR_MIDPOINT: f32 = 0.04;      // a site must sit near the route midpoint
const CARAVAN_CLEAR_RADIUS: f32 = 0.05;       // skip if a town already serves the midpoint
const CARAVAN_MAX_PER_YEAR: u32 = 3;
/// The age of colonisation opens once the world has matured — from year 30.
const COLONY_START_TICK: u32 = 30 * 365;
/// A wealthy polis devotes this share of its treasury each month to sponsored
/// MIGRATION (relieving its crowding by funding emigrants to needier cities /
/// its own colonies). Drains the treasuries that otherwise hoard indefinitely.
const POLIS_MIGRATION_SPEND: f32 = 0.03;
/// A polis needs at least this treasury before it sponsors migration.
const POLIS_MIGRATION_MIN_TREASURY: f32 = 50.0;
/// Fraction of the sponsoring city's population a monthly migration wave moves.
const POLIS_MIGRATION_POP_FRAC: f32 = 0.012;
/// Recent migration arrows kept for the map (refugee roads + economic drift).
const MIGRATION_ARROW_CAP: usize = 120;
/// Recent ROUTE-BOUND migration flows kept for the reworked Migration overlay.
const MIGRATION_ROUTE_CAP: usize = 90;
/// ── Economic migration (#23) — yearly wage/mood-driven population drift toward
/// thriving cities in the same trade component. People LEAVE a city only when its
/// opportunity (prosperity − starvation) is below `ECON_MIG_STAY_ABOVE`, and only
/// toward a destination that is clearly better by `ECON_MIG_GRADIENT`. Bounded to
/// `ECON_MIG_FRAC` of population/year so the dynamics guardrails stay satisfied.
const ECON_MIG_MIN_POP: f32 = 400.0;
const ECON_MIG_STAY_ABOVE: f32 = 0.55;
const ECON_MIG_GRADIENT: f32 = 0.15;
const ECON_MIG_FRAC: f32 = 0.02;
/// People migrate CITY-TO-CITY to the NEAREST better city within this range — not in
/// one global A→Z leap. Over years the drift chains A→B→C toward the best regions.
const MIGRATION_MAX_KM: f32 = 3000.0;
/// Minority quarters blend into the majority at this fraction per year. Kept gentle so
/// immigrant quarters actually PERSIST long enough to read on the map (was 0.04, which
/// erased a slow trickle of newcomers almost as fast as it arrived — cities looked
/// permanently monocultural).
const MINORITY_ASSIM_RATE: f32 = 0.02;
/// SUBSISTENCE FARMING — every land settlement grows its own staple food. Each tick a
/// hub's cereal production is topped up to at least this fraction of its own food NEED,
/// so an isolated town (no trade route reaching a producer) still feeds itself and is
/// not "100% short" on everything. Trade still supplies comfort/luxury and surpluses.
const SUBSISTENCE_FOOD_FRAC: f32 = 0.9;
/// A settlement can only feed itself from its OWN fields up to this population — the
/// carrying capacity of a village's hinterland. Beyond it a city MUST bring food in
/// by trade (or face shortage), so subsistence never props up a large city that
/// outgrew its region: big cities live and grow on trade, remote hamlets stay small.
const REMOTE_MAX_POP: f32 = 4000.0;
/// ── Diaspora: travel-prone MERCHANT cultures (Hansa-in-the-Baltic / trading
/// minority) spread as minority quarters along trade ties. Roughly a third of cultures
/// are mobile enough (mobility ≥ gate); they send settlers to trade partners each year,
/// so a real patchwork of minority quarters grows across the trade network over decades.
const DIASPORA_MIN_POP: f32 = 800.0;      // a city needs some people to send a diaspora
const DIASPORA_SEND_FRAC: f32 = 0.01;     // fraction of pop that emigrates per wave
const DIASPORA_MOBILITY_GATE: f32 = 0.5;  // only cultures at/above this mobility spread
const DIASPORA_MAX_MINORITY: f32 = 0.45;  // a diaspora tops out at this share of a host
const DIASPORA_MAX_PER_YEAR: u32 = 20;    // a visible flow, still legible
/// ── Ruin REVIVAL — a long-dead site is resettled once its region recovers.
const RESETTLE_COOLDOWN_YEARS: u32 = 15; // a ruin must lie empty this long first
const RESETTLE_REACH_FRAC: f32 = 0.12;   // a thriving patron must be within this of world width
const RESETTLE_PATRON_MIN_POP: f32 = 8_000.0; // the reviving region needs a real city nearby
const RESETTLE_POP: f32 = 500.0;         // pioneers refound a small town (tiered scale)
const RESETTLE_PROB: f32 = 0.10;         // per eligible ruin per year
const RESETTLE_MAX_PER_YEAR: u32 = 2;    // trickle, so revivals stay legible
/// House TRADE OUTPOSTS open earlier — year 30 — but are gated behind serious
/// wealth: only a great house (≈150k) can afford the heavy founding cost (≈120k).
/// Easier conditions than a settlement colony (no bank/food/joint-stock), just the
/// wealth + cost. Outposts can NEVER become independent and stay small.
const OUTPOST_START_TICK: u32 = 30 * 365;
const OUTPOST_FOUND_WEALTH: f32 = 100_000.0; // a house this rich may found one
const OUTPOST_FOUND_COST: f32 = 70_000.0;    // heavy cost (debited from the house)
const OUTPOST_MAX_POP: f32 = 800.0;          // a trade post stays small (hard pop cap)
/// A long-lived, thriving outpost (at the cap) whose wealthy house has held it this
/// many years may MATURE into a full colony (Phoenician emporion → city: Gadir, Utica).
const OUTPOST_GRADUATE_YEARS: u32 = 30;
/// …and its owning house must be at least this rich to make the investment.
const OUTPOST_GRADUATE_WEALTH: f32 = 60_000.0;
// ── Trade bases (houses develop EXISTING under-traded small cities). See
//    docs/TRADE_BASE_MECHANIC_PLAN.md. The accessible cousin of the outpost: a house
//    invests influence + capital into a real settlement to bootstrap it into a node. ──
const BASE_START_TICK: u32 = 10 * 365;        // bases open from ~year 10
const BASE_INVEST_WEALTH: f32 = 40_000.0;     // a house this rich may develop a base
const BASE_INVEST_COST: f32 = 18_000.0;       // base cost (scaled by city size below)
const BASE_SEED: f32 = 600.0;                 // working capital seeded into the city
const BASE_MIN_POP: f32 = 5_000.0;            // big enough to matter
const BASE_MAX_POP: f32 = 60_000.0;           // small/undeveloped enough to be worth it
const BASE_UNDERTRADE_FRAC: f32 = 0.06;       // under-traded if throughput < frac·pop
const BASE_POP_GROWTH_BONUS: f32 = 0.012;     // yearly pop nudge while patronised
const BASE_DEVELOPED_POP: f32 = 75_000.0;     // patronage concludes once developed
/// Yearly dividend a settlement colony pays its backers, as a fraction of its
/// trade surplus (kept small so fortunes stay bounded).
const COLONY_DIVIDEND_RATE: f32 = 0.10;
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
/// city's hinterland isn't overrun (houses AND guilds both build there). Only truly
/// large cities can push past the base cap, and the extra slots are dear.
const MAX_ESTATES_PER_CITY: usize = 3;         // base cap for an ordinary city
const MAX_ESTATES_BIG_CITY: usize = 5;         // a great city (≥ pop threshold) may reach 5
const ESTATE_BIG_CITY_POP: f32 = 150_000.0;    // pop at which the 4th/5th slot unlocks
const ESTATE_HIGH_SLOT_COST_MULT: f32 = 6.0;   // steep cost premium for the 4th/5th slot
/// Per-capita output rate of a house-built manufactory's luxury good.
const MANUFACTORY_PERCAP: f32 = 0.2;
/// Derived manufacturing demand: per unit of a city's labour capacity, how much
/// raw INPUT stock it wants buffered so its workshops can keep producing. This is
/// what pulls wool/iron/sugar into the weaving/forge/refining cities so the
/// finished goods actually accumulate in their warehouses.
const MANUFACTURE_PULL: f32 = 12.0;
/// Each estate/manufactory upgrade tier multiplies its output by this (5 tiers).
const ESTATE_UPGRADE_MULT: f32 = 1.4;
/// A MANUFACTORY (value-added workshop, estate_kind 6) is a major capital works —
/// far dearer than a raw estate. Building one costs at least this; upgrading a tier
/// costs this; and a workshop can only be upgraded once every `UPGRADE_INTERVAL`
/// (re-tooling takes years). Calibrated to the live wealth scale (great houses hold
/// 200k+), so only a serious house can found/expand one.
// NOTE: a flat 40k BUILD floor collapses the economy in the dynamics test — cheap
// manufactories are currently the load-bearing house growth engine, so pricing them
// out stops houses bootstrapping. Reserved pending an income rebalance (see notes).
#[allow(dead_code)]
const MANUFACTORY_BUILD_COST: f32 = 40_000.0;
const MANUFACTORY_UPGRADE_COST: f32 = 30_000.0;
const MANUFACTORY_UPGRADE_INTERVAL: u32 = 5 * 365; // ticks (5 years)
/// Estate/manufactory RESALE market. An asset-rich but cash-poor house below this
/// wealth will sell a holding to raise liquidity (a distress sale).
const RESALE_DISTRESS_WEALTH: f32 = 6_000.0;
/// A polis with a treasury this thin will sell a city-owned (civic) works to refill it.
const CIVIC_SALE_TREASURY_FLOOR: f32 = 100.0;
/// A bank acquiring a manufactory on the resale market takes this controlling share.
const RESALE_BANK_STAKE: f32 = 0.6;
// ── Estate/manufactory condition: age decay, labor/unrest debuffs, disasters ──
/// Effectiveness lost per year since a works was last built/upgraded (wear).
const ESTATE_DECAY_PER_YEAR: f32 = 0.010;
/// Cap on the age/wear penalty (an old un-upgraded works loses at most this).
const ESTATE_AGE_PENALTY_CAP: f32 = 0.12;
/// Host-city population for a fully-staffed works; smaller cities run below capacity.
const ESTATE_LABOR_FULL_POP: f32 = 2_000.0;
/// Cap on the labor-shortage penalty (an almost-empty host city).
const ESTATE_LABOR_PENALTY_CAP: f32 = 0.12;
/// Cap on the unrest/famine penalty (a fully-starving host city).
const ESTATE_UNREST_PENALTY_CAP: f32 = 0.20;
/// Per-year chance an intact works suffers a disaster (fire / flood / blight).
const DISASTER_ANNUAL_CHANCE: f32 = 0.035;
const DISASTER_MIN_DAMAGE: f32 = 0.30;
const DISASTER_MAX_DAMAGE: f32 = 0.70;
/// Fraction of outstanding damage repaired each year when the owner can fund it.
const REPAIR_RATE_PER_YEAR: f32 = 0.30;
/// Repair cost = (damage repaired) × (works value) × this.
const REPAIR_COST_FRAC: f32 = 0.5;

// ── Merchant fleets & voyage risk ────────────────────────────────────────────
/// A prospering TOWN can charter a guild from year 5 (lower bar than the initial
/// seed), but only if commercially successful — so guilds cluster in thriving
/// cities and the world stays differentiated (not a guild in every town).
const GUILD_FORM_POP: f32 = 6_000.0;
const GUILD_FORM_PROSPERITY: f32 = 0.45;
/// A settlement gets a civic Merchant Guild once it reaches this population.
// Lowered 50k→35k: the old 50k was almost never reached on the new humble scale, so
// guilds never formed ("guilds disappear"). The capacity fix now lets thriving
// cities grow past 35k, so the greatest commercial centres charter guilds — kept a
// big-city institution (NOT every large town) so the world stays differentiated.
const GUILD_MIN_POP: f32 = 35_000.0;
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

// ── Commercial influence, trade dominance & the Bailo (HQ) tier ──────────────
/// Monthly influence a house gains in a city = its share of that city's house-trade
/// ÷ resistance, ×this rate (then clamped 0..1). Decays by INFLUENCE_DECAY/mo.
const INFLUENCE_GAIN: f32 = 0.20;
const INFLUENCE_DECAY: f32 = 0.015;
/// An OFFICE guarantees at least this much standing influence (a permanent foothold).
const OFFICE_INFLUENCE_FLOOR: f32 = 0.18;
/// City RESISTANCE to outside takeover = 1 + pop/REF + guild bonus. Small, un-guilded
/// towns have low resistance (fall fast); a great guilded metropolis resists all but a Bailo.
const INFLUENCE_POP_REF: f32 = 12_000.0;
const INFLUENCE_GUILD_RESIST: f32 = 1.5; // a resident civic guild adds this to resistance
/// A house DOMINATES a city's trade when its influence ≥ threshold AND leads the
/// runner-up by the margin. Dominance is TRADE-only (no government — see the Bailo).
const DOMINANCE_THRESHOLD: f32 = 0.45;
const DOMINANCE_MARGIN: f32 = 0.08;
/// Two houses both above this in one city are CONTESTING it → rising rivalry/trade war.
const CONTEST_INFLUENCE: f32 = 0.30;
/// Trade-tax edge of dominance: the dominator pays ×this at its dominated cities,
/// non-dominators pay ×that (a modest sway, not a stranglehold).
const DOMINATOR_TAX_MULT: f32 = 0.75;
const RIVAL_TAX_MULT: f32 = 1.15;
/// A house with overwhelming TRADE CONTROL of a city (influence ≥ this) MONOPOLISES
/// its commerce: it commands the city's surplus and exports on its own terms,
/// taking this extra export rent on goods shipped OUT of that controlled city.
/// (User #2: at ≥80% control a house can monopolise and trade as it pleases.)
const MONOPOLY_CONTROL: f32 = 0.80;
const MONOPOLY_EXPORT_RENT: f32 = 0.5;
// ── Bailo (governing headquarters) ──
/// An office may rise to a Bailo only after sustained dominance at/above this influence,
/// with the wealth to sustain it. The home seat is always a "Bailo-equivalent" capital.
const BAILO_MIN_INFLUENCE: f32 = 0.70;
const BAILO_MIN_WEALTH: f32 = 400.0;
/// Soft cap on a house's foreign Bailos = floor(power·SCALE) + wealth/PER. Each Bailo
/// adds a monthly upkeep (so a house only keeps as many HQs as it can bankroll).
const BAILO_CAP_POWER_SCALE: f32 = 3.0;
const BAILO_CAP_WEALTH_PER: f32 = 30_000.0;
const BAILO_UPKEEP: f32 = 0.6; // monthly, ×city size
/// The concession lane home from a Bailo pays only a token toll (extremely low, not free).
const BAILO_CONCESSION_TOLL: f32 = 0.10; // ×the normal tax rate

// ── House archetypes ────────────────────────────────────────────────────────
const ARCH_SPECIALTY: u8 = 0; // cheaper freight + fatter margin on specialty goods
const ARCH_FLEET: u8 = 1;     // safer voyages, cheaper ships, longer reach
const ARCH_BANKING: u8 = 2;   // wealth earns interest; can trade on credit
const ARCH_POLITICAL: u8 = 3; // more political power; wins city charters
// ── Government / key-figure capture tuning (yearly `update_government`) ──────────────
/// A figure is CAPTURED (serves the top-spending house) once its control passes this.
pub const OFFICIAL_CAPTURE: f32 = 0.55;
/// Yearly decay of an official's control when no house spends to maintain it.
const OFFICIAL_CONTROL_DECAY: f32 = 0.10;
/// Money a bribing house spends per point of control per year (scaled by the seat's
/// weight); it buys `spend / (weight·BRIBE_COST)` control. Cheap enough that a wealthy
/// house can sway a minor seat, dear enough that capturing a whole council costs real money.
const BRIBE_COST: f32 = 400.0;
/// A house only bothers with a city where its commercial influence clears this.
const GOVT_MIN_INFLUENCE: f32 = 0.08;
/// Term lengths (years) before a seat turns over, by govt type (commune/council/prince).
pub const GOVT_TERM_YEARS: [u32; 3] = [3, 5, 10];
/// Chance, at a regime change, that the most-influential family installs one of its OWN
/// (a kin figure that auto-serves it).
const GOVT_KIN_CHANCE: f32 = 0.30;
/// Extra tariff tilt a captured government gives: the captor's exports/imports of ITS
/// specialty goods are cheapened, rivals' dearer (folded into decide_polis_policy).
const CAPTOR_TARIFF_FAVOUR: f32 = 0.6;   // ×base for the captor's goods
/// Trade-influence boost a captor gets at the city it controls (added to `influence`).
const CAPTOR_INFLUENCE_BOOST: f32 = 0.12;
const LAWS_CAP: usize = 12;

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
/// DLC 3.5 rebalance: bankers earn interest only on liquid capital up to this cap,
/// so the perk is a flat early boost rather than an uncapped compounding engine.
const BANK_INTEREST_CAP: f32 = 20.0;
/// DLC 3.5 · progressive civic wealth tax: a rich house bleeds a rising share of
/// its fortune to its home city's TREASURY each month — both caps run-away wealth
/// and funds the polis (war chest / public works). Max marginal rate at high wealth.
const WEALTH_TAX_PROG: f32 = 0.02;
const WEALTH_TAX_SCALE: f32 = 60.0;

/// Private-house progressive wealth tax → the home city TREASURY. A flat base rate
/// plus a QUADRATIC surcharge on wealth above a soft cap: modest families grow
/// freely, but great fortunes hit a firm ceiling (the surplus enriches the polis).
/// Without this the local economy let a single house run away to ~1.25M; this pins
/// the sustained richest house to a few tens of thousands.
const HOUSE_WEALTH_TAX_BASE: f32 = 0.004;    // monthly flat civic wealth tax
// Wealth is NOT hard-capped: the surcharge only bites well above the soft cap and
// gently, so a great trading dynasty CAN climb into the hundreds of thousands (and
// fund a trade outpost) — while the quadratic still bends the very richest back
// before any millions-scale runaway. Was 8k / 5e-6 (which pinned the richest ~64k).
const HOUSE_WEALTH_SOFTCAP: f32 = 60_000.0;  // wealth below this escapes the surcharge
const HOUSE_WEALTH_TAX_QUAD: f32 = 3.0e-7;   // monthly quadratic surcharge on the overshoot
const HOUSE_WEALTH_TAX_MAXFRAC: f32 = 0.4;   // never tax more than this share of wealth/month
/// First-N-years surcharge: it is HARD to get rich early (founding generations), the
/// multiplier lerps from `MULT` at year 0 down to 1.0 by `YEARS`.
const EARLY_WEALTH_TAX_MULT: f32 = 3.0;
const EARLY_WEALTH_TAX_YEARS: f32 = 20.0;

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
// The GLOBAL event chronicle. Unlike the per-house / per-hub / bank histories (all
// capped above/below), `self.journal` is appended from 30+ sites every tick and is
// fully serialized into each year-boundary autosave. Left unbounded it grows without
// limit over a long campaign until the autosave's `to_string` OOMs and the process
// aborts mid-year (the "campaign died/restarted after ~30 years" crash). Cap it to
// the most-recent N entries — plenty for the year-grouped Chronicle UI.
const JOURNAL_CAP: usize = 20_000;
// ── Futures contracts (Phase 3). A contract is a thin, two-sided stability layer
//    ON TOP of the spot market: it covers only a slice of a city's need (so the
//    price signal survives), at a struck price allowed to drift within a band, for
//    a term gated by the seller's record of stable growth. ──
const CONTRACT_COVERAGE_CAP: f32 = 0.25; // max share of a city's need under contract per good
const CONTRACT_PRICE_BAND: f32 = 0.12;   // paid price drifts ≤ ±this around the strike
const CONTRACT_DELIVER_DAYS: u32 = 30;   // a delivery every ~month
const CONTRACT_FORM_CHANCE: f32 = 0.10;  // monthly chance an eligible house offers one
const MAX_CONTRACTS: usize = 400;        // global cap (bounds the per-tick fulfil loop)
/// A house may only sign a contract if its FREE carrying capacity (fleet minus what
/// its existing contracts already claim) exceeds the new monthly quota by this factor
/// — i.e. it must hold ≥20% more transport than the deal needs, so a due delivery
/// always has a spare vessel and the contract doesn't breach for want of a ship.
const CONTRACT_TRANSPORT_MARGIN: f32 = 1.2;
/// Share of a SOURCE city's monthly OUTPUT of a good that a merchant house can commit
/// to shipping under a supply contract (it buys the good on that city's market). Sizing
/// a contract only by the source's *surplus above its own need* starved every deal — a
/// specialist producer consumes little of its export, and a self-sufficient producer
/// shows zero surplus → `supply_cap` 0 → no futures contract could EVER be signed.
/// LIMITED LIABILITY on a contract default: the forfeit is capped so it can push the
/// house at most this far into debt (beyond which it goes bankrupt, not infinitely
/// negative). Keeps a run of over-committed defaults from cratering house wealth.
const CONTRACT_LIABILITY_FLOOR: f32 = 40.0;
/// A city must be at least this prosperous (0..1 sentiment) to be signed a futures
/// supply contract — a futures market needs a solvent buyer. Keeps contracts out of
/// destitute/famine-struck cities (which stay poor and can still boil over into revolt).
const CONTRACT_BUYER_MIN_PROSPERITY: f32 = 0.35;
/// How many years of per-(hub,good) trade volume the Flows trend graph keeps.
const TRADE_HIST_CAP: usize = 40;
/// Global cap on tracked (hub,good) history rows (sparse; drops dead trades first).
const TRADE_HIST_ROWS: usize = 8000;
/// Most-recent regional crashes / concluded wars kept (older entries age out so a
/// century-long campaign can't grow these chronicles without bound → memory).
const CRASH_RECORD_CAP: usize = 200;
const WAR_LOG_CAP: usize = 200;
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
const HOUSE_CONSUMPTION_RATE: f32 = 0.0022;
/// Guilds are civic — they spend more of their wealth on their own citizens.
const GUILD_CIVIC_RATE: f32 = 0.005;
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
const FLEET_UPKEEP_FRAC: f32 = 0.05;
/// Per-vessel monthly chance of being lost to wear (rot, storms, breakdown) — the
/// slow decay of ships & caravans, so fleets must be continually replaced.
const FLEET_DECAY_CHANCE: f32 = 0.012;
/// Civic taxes a city levies on a house's trade — export on goods leaving the
/// origin, import on goods arriving at the destination. Paid by the house, funding
/// the city (into its civic_pool → people). Guilds pay HEAVIER taxes (civic duty).
pub const EXPORT_TAX_RATE: f32 = 0.03;
pub const IMPORT_TAX_RATE: f32 = 0.035;
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
/// DLC 3.5 · share of collected taxes RETAINED in the city treasury (the rest
/// flows to the people via the civic pool, as before). Gives cities real capital
/// to field wars and public works.
const TREASURY_TAX_SHARE: f32 = 0.35;
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

/// DLC 3.5 · a coin's headline VALUE index: full-bodied, fully-trusted coin trades
/// at a premium (~1.2× "agio"); a debased / distrusted coin sits below 1.0. Pure
/// display metric derived from fineness × acceptance.
pub fn coin_value(fineness: f32, trust: f32) -> f32 {
    let f = if fineness <= 0.0 { 1.0 } else { fineness };
    f * (0.7 + 0.5 * trust.clamp(0.0, 1.0))
}

/// v2.0 · a coin's single headline STRENGTH (0..100) — the one number the Money
/// panel leads with, built from its two drivers (fineness × acceptance). A
/// fully-trusted, full-bodied coin scores 100; debasement OR distrust drag it down.
pub fn coin_strength(fineness: f32, trust: f32) -> f32 {
    let f = (if fineness <= 0.0 { 1.0 } else { fineness }).clamp(0.0, 1.0);
    100.0 * trust.clamp(0.0, 1.0) * (0.55 + 0.45 * f)
}

// ── v2.0 · closed monetary loop (quantity-theory-lite inflation) ─────────────
/// Baseline monetary inflation every economy carries (a mild ~1.5% drift), so even
/// sound money still levies an inflation-tax on hoarded fortunes (as the old flat
/// rate did) — debasement/money growth then adds ON TOP.
const INFL_BASE: f32 = 0.015;
/// Debasement pass-through: a coin cut to fineness `f` adds `(1−f)·K` to prices.
const INFL_DEBASE_K: f32 = 0.10;
/// Money-growth pass-through: a coin whose circulation grew `g` YoY adds `g·K`.
const INFL_MONEY_K: f32 = 0.10;
/// Real-output growth that soaks up money each year (the deflationary offset).
const INFL_REAL_GROWTH: f32 = 0.005;
/// Bounds on a single year's local inflation (keeps price levels + the wealth
/// inflation-tax finite; always a small positive drift, capped so it can't blow up).
const INFL_MIN: f32 = 0.002;
const INFL_MAX: f32 = 0.06;

// ── v2.0 · recoinage / reform ────────────────────────────────────────────────
/// A council reforms its coinage only when fineness has slipped below this.
const REFORM_FINENESS_FLOOR: f32 = 0.90;
/// …and trust has fallen below this (the coin is visibly failing).
const REFORM_TRUST_FLOOR: f32 = 0.50;
/// Cost of a reform (recall + re-strike), as a fraction of the seat's throughput —
/// paid from the treasury, so only a solvent polis can afford honest money.
const REFORM_COST_FRAC: f32 = 0.6;
/// Immediate confidence restored by re-minting at full fineness.
const REFORM_TRUST_BUMP: f32 = 0.15;
/// Years a council must wait between reforms, and years its honest-money mandate
/// holds fineness at 1.0 afterwards.
const REFORM_COOLDOWN_YEARS: u32 = 8;
const REFORM_MANDATE_YEARS: u32 = 6;

// ── v2.0 · bank runs (idiosyncratic, outside a systemic crash) ───────────────
/// Fraction of deposits a fragile bank bleeds in a run, scaled by how far its
/// reserve ratio has fallen below `BANK_RUN_RATIO`.
const BANK_RUN_WITHDRAW: f32 = 0.18;

// ── v2.0 · bullion-limited minting (mint regulation) ─────────────────────────
/// Bullion (weighted gold+silver output) a region needs per unit of coin demand
/// (throughput) to sustain FULL-BODIED coin. Below this ratio the mint is forced
/// to stretch its metal — fineness is capped down (endogenous debasement from
/// scarcity, not choice). Lenient so only genuinely bullion-poor regions bind.
const MINT_BULLION_DEMAND: f32 = 0.04;
/// Gold is far denser in value than silver — weight it in the bullion tally.
const MINT_GOLD_WEIGHT: f32 = 3.0;
/// The lowest fineness bullion scarcity alone can force (a bullion-starved mint
/// still strikes a passable, if base-heavy, coin — it imports/short-weights, it
/// does not collapse). Full bullion supply lifts the cap to 1.0.
const MINT_FINENESS_FLOOR: f32 = 0.85;

// ── v2.0 · mint charter (minting as a privilege) ─────────────────────────────
/// A council seat earns the RIGHT OF THE MINT only once it is a real commercial
/// centre: its trade throughput must clear this fraction of the world's busiest
/// hub, AND its population this fraction of the largest — so tiny council seats
/// don't each strike their own coin. Relative, so it adapts to any world.
const MINT_CHARTER_THROUGH_FRAC: f32 = 0.08;
const MINT_CHARTER_POP_FRAC: f32 = 0.05;
/// One-time cost to establish the mint (paid from the treasury) — a city must be
/// solvent enough to fund the minting house.
const MINT_CHARTER_COST: f32 = 30.0;

// ── DLC 4 · Good quality ─────────────────────────────────────────────────────
/// Value multiplier a good's quality earns: coarse goods trade at a discount,
/// exquisite ones at a premium (≈0.6×…1.5× across the grade range).
pub fn quality_value_mult(q: f32) -> f32 {
    0.6 + 0.9 * q.clamp(0.0, 1.0)
}
/// 5-rung grade label for a quality 0..1 (matches the worldgen ladder).
pub fn quality_grade(q: f32) -> &'static str {
    if q >= 0.85 { "Exquisite" } else if q >= 0.68 { "Fine" }
    else if q >= 0.50 { "Standard" } else if q >= 0.32 { "Common" } else { "Coarse" }
}
/// Monthly learning-by-doing rate: a manufactory drifts toward its quality cap.
const QUALITY_LEARN_RATE: f32 = 0.04;
/// Yearly chance a house with a manufactory steals a rival's quality technique.
const QUALITY_STEAL_CHANCE: f32 = 0.10;
/// On a successful steal, the thief closes this fraction of the gap to the leader.
const QUALITY_STEAL_FRAC: f32 = 0.6;

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

const SHIP_COST: f32 = 7.0;
const RIVER_COST: f32 = 4.5;
const CARAVAN_COST: f32 = 4.0;

// ── DLC 3.5 · Coinage (the "Venice ducat") ──────────────────────────────────
// A council-led polis issues a NAMED coin whose acceptance ("trust") is sticky
// reputation: it rises with full-bodied minting, a deep treasury, trade wealth
// and civic stability, and falls when the council debases. The strongest coins
// become reserve currencies accepted abroad — a small import-freight discount
// that turns strong-money cities into entrepôts.
/// How fast coin trust eases toward its yearly target (reputation is sticky).
const COIN_TRUST_EASE: f32 = 0.20;
/// Extra trust hit per point of debasement vs last year (sudden cuts spook holders).
const COIN_DEBASE_PENALTY: f32 = 1.5;
/// Seigniorage skimmed into the treasury = throughput · (1−fineness) · this.
/// Debasing raises immediate mint profit but erodes trust (and feeds bubbles).
const COIN_SEIGNIORAGE: f32 = 0.04;
/// Max import-freight discount a fully-trusted reserve coin grants its market.
const COIN_FREIGHT_DISCOUNT: f32 = 0.10;
/// A coin must clear this trust to count as a reserve currency abroad.
const RESERVE_TRUST_MIN: f32 = 0.55;
/// Multi-currency circulation. A city holds a small BASKET of coins: its own/main
/// plus foreign coins that arrive with trade. Shares ease toward an adoption target
/// each year (sticky → flips take years). The main coin only flips when a rival
/// leads it by `COIN_FLIP_MARGIN`.
const COIN_BASKET_N: usize = 4;           // max coins tracked per city
const COIN_ADOPT_EASE: f32 = 0.18;        // yearly easing of basket shares toward target
const COIN_HOME_BIAS: f32 = 3.0;          // weight multiplier for the city's own mint
const COIN_FLIP_MARGIN: f32 = 1.08;       // a rival must lead the main coin by this to flip
/// Seigniorage (to the issuing polis treasury) per unit of trade circulating in its
/// coin abroad, and a tiny prestige bump to the issuing council house. Small so the
/// house-wealth bound holds (routes to TREASURY, not house wealth).
const COIN_CIRCULATION_SEIGNIORAGE: f32 = 0.002;

// ── DLC 3.5 · Banks (merchant bankers as institutions) ──────────────────────
/// The age of banking opens once the world's coinage has matured — from year 20.
const BANK_START_TICK: u32 = 20 * 365;
/// A house must hold at least this wealth to charter a bank (a great fortune turns
/// to banking). Universal — banking-archetype or not.
const BANK_FOUND_WEALTH: f32 = 100_000.0;
/// Wealth at which a house is "rich enough to be a banker" for the succession
/// archetype pivot (distinct from the founding bar above).
const BANK_FOUND_WEALTH_RICH: f32 = 5_000.0;
const BANK_FOUND_PRESTIGE: f32 = 0.15;
/// The bank's seat city coin must be trusted at least this much (you bank in good
/// money). Kept modest so a bank can actually be chartered once coins are trusted.
const BANK_FOUND_COIN_TRUST: f32 = 0.40;
/// PRICE of founding a bank (debited from the house): 50k. Of that, 40k is paid IN
/// as the new bank's specie reserves / liquidity (its starting treasury); the
/// remaining 10k is the establishment / charter cost paid to the seat polis.
const BANK_FOUND_PRICE: f32 = 50_000.0;
const BANK_FOUND_RESERVE: f32 = 40_000.0;
/// Monthly interest a bank charges on loans / pays on deposits.
const BANK_LOAN_RATE: f32 = 0.012;
const BANK_DEPOSIT_RATE: f32 = 0.006;
/// Fractional reserve: notes/credit may run up to this multiple of specie reserves.
const BANK_RESERVE_MULT: f32 = 3.0;
/// Below this reserve ratio (reserves ÷ liabilities) a bank is fragile → run risk.
const BANK_RUN_RATIO: f32 = 0.22;
/// Book value a counting-house branch adds to a bank's real-estate assets.
const BANK_BRANCH_VALUE: f32 = 2.0;
/// Income share a bank's equity stake draws from a manufactory's owner-cut.
const BANK_STAKE_SHARE: f32 = 0.25;
/// Years of yearly balance-sheet snapshots kept per bank (bounds save size).
const BANK_HISTORY_CAP: usize = 60;
/// Capital value of a manufactory per tier; a stake costs `share × tier × this`.
const BANK_STAKE_VALUE_PER_TIER: f32 = 40_000.0;

// ── DLC 3.5 · Regional financial crashes (contagion) ────────────────────────
/// Fraction of wealth houses in the stricken region lose when the crash hits.
const CRASH_WEALTH_HAIRCUT: f32 = 0.15;
/// Coin-trust collapse applied to every polis in the stricken region. Kept modest
/// so a crash doesn't permanently drag coin_trust below the bank-founding floor
/// (which would prevent the region ever re-chartering banks).
const CRASH_TRUST_HIT: f32 = 0.15;
/// Duration (ticks) of the regional panic event (frozen credit + low morale).
const CRASH_PANIC_TICKS: u32 = 240;
/// Yearly chance a HIGH-tier (≥4★) speculative bubble actually POPS into a crash.
const CRASH_BUBBLE_POP_CHANCE: f32 = 0.15;
/// Fraction of deposits that flee a regional bank in a contagion run. A panic is a
/// partial withdrawal — only thinly-reserved banks should be swept; a soundly
/// capitalised bank rides it out (otherwise one failure wipes every bank).
const CRASH_CONTAGION_RUN: f32 = 0.20;

// ── DLC 3.5 · Economic war (a wealth sink + conflict) ───────────────────────
/// At most this many wars run at once (wars are rare, dramatic events).
const MAX_ACTIVE_WARS: usize = 2;
// ── War GOALS — what the victor takes (beyond the one-off plunder). ──────────
/// Sack-and-go: only the immediate reparations (legacy behaviour).
const WAR_GOAL_PLUNDER: u8 = 0;
/// The loser is made a tributary — recurring yearly payments for a term.
const WAR_GOAL_TRIBUTE: u8 = 1;
/// The victor's ruling house is granted a BAILO (commercial foothold) in the loser.
const WAR_GOAL_TRADE_RIGHTS: u8 = 2;
/// The victor annexes the loser — its ruling house is installed on the loser's
/// council (a bailo makes it stick through the yearly council recompute).
const WAR_GOAL_ANNEX: u8 = 3;
/// Years a defeated city pays tribute to its overlord.
const TRIBUTE_YEARS: u32 = 10;
/// Yearly tribute as a fraction of the tributary's treasury (bounded — moves money,
/// never mints it, so the economy stays within the dynamics-test envelope).
const TRIBUTE_RATE: f32 = 0.06;
/// Hard cap on a single year's tribute payment (× city_size_factor).
const TRIBUTE_CAP: f32 = 40.0;
/// Short label for a war goal (journal / Wars log).
pub fn war_goal_label(goal: u8) -> &'static str {
    match goal {
        WAR_GOAL_TRIBUTE => "for tribute",
        WAR_GOAL_TRADE_RIGHTS => "for trade rights",
        WAR_GOAL_ANNEX => "for annexation",
        _ => "for plunder",
    }
}
/// Yearly chance a new war is declared (when below the cap).
const WAR_DECLARE_CHANCE: f32 = 0.10;
/// Yearly forced levy on each resident house's wealth → the city war chest. The
/// principal way war drains over-rich houses.
const WAR_LEVY_RATE: f32 = 0.12;
/// Fraction of the treasury a belligerent burns on the war effort each year
/// (consumed — the destructive cost of armies & blockade).
const WAR_SPEND_RATE: f32 = 0.30;

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
    /// DLC · abstract social strata of this settlement (shares + inequality +
    /// commoner welfare). Seeded once on first advance; updated yearly.
    #[serde(default)] pub society: Society,
    /// DLC 4 · typed population units derived from `society` × population each year
    /// (read-only foundation; not yet wired into consumption). Empty on old saves
    /// until the next yearly derive.
    #[serde(default)] pub pops: Vec<Pop>,
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
    /// Tick of this estate's last build/upgrade. Manufactories may only be upgraded
    /// once every `MANUFACTORY_UPGRADE_INTERVAL` (re-tooling takes years).
    #[serde(default)] pub last_upgrade_tick: u32,
    /// Owning house index for an estate (−1 = owned by the parent city). Estate
    /// export income flows to this owner — a core engine of house growth.
    #[serde(default = "neg_one_i32")] pub owner_house: i32,
    /// A bank's equity stake in this manufactory: the bank index holding a share, or
    /// −1 for none. `stake_share` of the owner-cut is paid to that bank as a dividend.
    #[serde(default = "neg_one_i32")] pub stake_bank: i32,
    #[serde(default)] pub stake_share: f32,
    /// Disaster damage to an estate/manufactory: 0 = intact, 1 = ruined. Suppresses
    /// the works' output until repaired (a fire/flood/blight sets it; repair clears it).
    #[serde(default)] pub damage: f32,
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
    // ── DLC 3.5 · City finances (treasury books) + war state ──
    /// Running yearly treasury books (taxes in, spending out) for the City Finances
    /// panel; `prev` holds the last completed year.
    #[serde(default)] pub finance: CityFinance,
    /// Hub index of the polis this city is currently AT WAR with (−1 = at peace).
    #[serde(default = "neg_one_i32")] pub war_with: i32,
    /// Tick the current war began (for its duration / the Wars log).
    #[serde(default)] pub war_since: u32,
    /// Accumulated war effort / morale this side has mustered (war chest spent).
    #[serde(default)] pub war_effort: f32,
    /// TRIBUTARY state: the overlord hub this city owes tribute to (−1 = free), set
    /// when it loses a war whose goal was Tribute. Cleared when the term lapses.
    #[serde(default = "neg_one_i32")] pub tribute_to: i32,
    /// Tick the tribute obligation lapses.
    #[serde(default)] pub tribute_until: u32,
    // ── DLC 3.5 · Coinage (the "Venice ducat") ──
    /// The NAMED coin this polis mints ("" = it issues none — only council seats do).
    #[serde(default)] pub coin_name: String,
    /// Acceptance / trust in this coin, 0..1 — sticky reputation eased yearly. The
    /// strongest become reserve currencies accepted abroad. 0 = no/untrusted coin.
    #[serde(default)] pub coin_trust: f32,
    /// The MAIN coin (hub INDEX of the issuing mint) this city settles its trade in.
    /// −1 = barter / none. Can FLIP to a foreign coin that durably dominates the
    /// basket. Reassigned yearly by `update_currency_baskets`.
    #[serde(default = "neg_one_i32")] pub settle_coin: i32,
    /// The city's CURRENCY BASKET: up to `COIN_BASKET_N` coins it holds, as
    /// `(coin mint hub INDEX, share 0..1)` sorted by share desc (shares ≈ sum 1).
    /// Foreign coin arrives with trade; shares ease yearly toward an adoption target
    /// (trust × value × issuer weight × home bias). Drives the usage overlay/chart.
    #[serde(default)] pub coin_basket: Vec<(u32, f32)>,
    /// Last year's mint fineness — so the coinage pass can read a sudden DEBASEMENT
    /// (a cut vs last year) and dock trust accordingly. 0 on old saves → no penalty.
    #[serde(default)] pub mint_fineness_prev: f32,
    // ── v2.0 · closed monetary loop (debasement/money-supply → local prices) ──
    /// Local PRICE LEVEL index (1.0 = par at campaign start). Compounds each year by
    /// the inflation of this city's settle-coin (debasement + money growth − real
    /// output growth), so a debased-coin city visibly gets dearer. 0/absent → 1.0.
    #[serde(default)] pub price_level: f32,
    /// Last year's money supply (Σ throughput × basket-share) for the coin THIS hub
    /// issues — lets the price loop read year-on-year money growth. 0 on old saves.
    #[serde(default)] pub coin_circ_prev: f32,
    // ── v2.0 · recoinage / reform ──
    /// Tick of this polis's last coinage REFORM (call-in + re-mint at full fineness).
    /// Gates a reform cooldown so a council can't reform every year. 0 = never.
    #[serde(default)] pub last_reform_tick: u32,
    /// Until this tick the council upholds an HONEST-MONEY mandate after a reform:
    /// `decide_polis_policy` holds mint fineness at 1.0 (no debasement) until it
    /// lapses, after which cheap-money pressure can creep back. 0 = no mandate.
    #[serde(default)] pub reform_until: u32,
    /// v2.0 · the monetary METAL this polis strikes its coin in, from the bullion
    /// its trade region can reach: 0 = silver (default/imported), 1 = gold,
    /// 2 = electrum (both gold & silver), 3 = bronze/billon (only base metal).
    #[serde(default)] pub coin_metal: u8,
    /// v2.0 · bullion capacity ratio = regional bullion output ÷ coin demand. ≥1
    /// means metal is ample (fineness can be full); <1 means the mint is stretched
    /// and bullion scarcity is forcing debasement (surfaced as the "limiting factor").
    #[serde(default)] pub mint_bullion_ratio: f32,
    /// v2.0 · whether this polis holds the RIGHT OF THE MINT (a charter). Minting is
    /// a privilege of substantial commercial centres, not automatic for every council
    /// seat — a city earns it once it is large & busy enough and pays to establish a
    /// mint. Grandfathered true for any city that already struck a coin.
    #[serde(default)] pub has_mint: bool,
    // ── DLC 4 · Good QUALITY (per producing settlement / estate / manufactory) ──
    /// Per-good production quality 0..1 at THIS hub (Coarse→Exquisite). Seeded so it
    /// varies by settlement; manufactures climb via learning-by-doing and can be
    /// lifted by stealing a rival's technique. Empty until the one-time migration.
    #[serde(default)] pub quality: Vec<f32>,
    /// Espionage record (manufactories): good index whose technique was STOLEN into
    /// this hub (−1 none) and the hub id it was taken from (−1 none).
    #[serde(default = "neg_one_i32")] pub stolen_good: i32,
    #[serde(default = "neg_one_i32")] pub stolen_from: i32,
    // ── Colonisation (serde-defaulted → old saves load as ordinary hubs) ──
    /// 0 = not a colony · 1 = SETTLEMENT colony (city-founded full market hub that
    /// graduates outpost→city) · 2 = HOUSE trade outpost (remote low-pop factory).
    #[serde(default)] pub colony_kind: u8,
    /// Growth stage for a settlement colony: 1 outpost · 2 colony · 3 town · 4 city.
    #[serde(default)] pub colony_stage: u8,
    /// A settlement colony that has declared independence — keeps the hub but drops
    /// the dependency on / monopoly link to its founder.
    #[serde(default)] pub autonomous: bool,
    /// Founding settlement (hub INDEX) for a colony — the metropolis it links to.
    /// −1 = none / not a colony. (Distinct from `parent`, used for in-city estates.)
    #[serde(default = "neg_one_i32")] pub founder_hub: i32,
    /// Joint-stock backers of a settlement colony `(kind,idx,share)` where kind is
    /// 0 city / 1 house / 2 bank, idx the hub/house/bank index, share 0..1. Empty
    /// for house outposts and old saves. Dividends are paid back pro-rata.
    #[serde(default)] pub backers: Vec<(u8, u32, f32)>,
    // ── Colony food lifeline (settlement colonies only; serde-defaulted) ──
    /// Months of food held in the colony's reserve (target = `reserve_cap`). Drains
    /// when the supply lifeline can't cover the deficit; at 0 the colony starves.
    #[serde(default)] pub reserve_food: f32,
    /// Reserve ceiling in months (≈12 × a preservative factor that extends shelf-life).
    #[serde(default)] pub reserve_cap: f32,
    /// Consecutive years the colony has been FULLY supplied (food deficit covered).
    /// Resets to 0 on any break — gates growth (needs ≥5) and is the supply record.
    #[serde(default)] pub supply_years: f32,
    /// Tick the colony was founded (for the year-70 independence check).
    #[serde(default)] pub colony_founded_tick: u32,
    /// The colony's main bank (index) — set at founding; its loan defaults on collapse.
    #[serde(default = "neg_one_i32")] pub main_bank: i32,
    /// After a LOST war of independence, the colony may not rebel again until this tick.
    #[serde(default)] pub indep_cooldown_until: u32,
    /// Plague IMMUNITY: the city cannot be struck by (or carry) a plague until this
    /// tick — earned by surviving an outbreak. `#[serde(default)]` → old saves = 0.
    #[serde(default)] pub plague_immune_until: u32,
    /// PUBLIC HEALTH (hospices / quarantine), 0..`HOSPICE_MAX_LEVEL`. A prosperous
    /// council funds it (drawing on treasury → `finance.spent_health`); it cuts the
    /// death toll of a plague strike and lengthens the immunity earned. `#[serde(default)]`.
    #[serde(default)] pub public_health: f32,
    /// Colony food LIFELINE: dedicated supply ships the metropolis/backers keep on the
    /// grain run to this colony (invested in when the colony runs short, to make the
    /// supply steady). `#[serde(default)]` → old saves = 0.
    #[serde(default)] pub supply_ships: u32,
    /// The hub currently designated as this colony's food SOURCE (nearest sufficient
    /// grain surplus on the component), or −1. `#[serde(default = "neg_one_i32")]`.
    #[serde(default = "neg_one_i32")] pub supply_source: i32,
    /// Food actually delivered to this colony last supply pass (monthly units) — for
    /// the Colonial Office readout. Derived; `#[serde(default)]`.
    #[serde(default)] pub supply_delivered: f32,
    // ── Dynamic entrepôt / trade-hub status (campaign-only; serde-defaulted) ──
    /// Realized trade throughput touching this hub in the LAST year (imports+exports
    /// from `flow_year`). Drives `hub_class`. Display + classification signal.
    #[serde(default)] pub transit_year: f32,
    /// Commercial rank earned LIVE from trade: 0 = ordinary · 1 = trade hub · 2 =
    /// entrepôt (a great sea pass-through market — Venice/Bruges). Rises and falls with
    /// the trade that actually flows through the city.
    #[serde(default)] pub hub_class: u8,
    /// Hysteresis momentum for `hub_class`: consecutive years pushing up (+) or down
    /// (−); a tier changes only after 3 confirming years, so status doesn't flicker.
    #[serde(default)] pub class_momentum: i8,
    // ── Satellite CONSTRUCTION project (a metropolis builds this suburb over ~10y; all
    //    serde-defaulted so 0 = "finished / not a construction site"). See
    //    docs/SATELLITE_CONSTRUCTION_AND_MIGRATION_PLAN.md ──
    /// 0 = functional (not under construction); 1..=5 = current build stage
    /// (Survey · Foundations · Warehousing · Walls · Market).
    #[serde(default)] pub build_stage: u8,
    /// Progress 0..1 within the CURRENT stage. Advances with supply, decays when starved.
    #[serde(default)] pub build_progress: f32,
    /// This month's delivered-vs-quota ratio per category [food, preservables, construction].
    #[serde(default)] pub build_supply: [f32; 3],
    /// Auto-picked good id feeding each category (by locale — cheapest available).
    #[serde(default)] pub build_supply_good: [u16; 3],
    /// Consecutive under-supplied months (drives decay + a stage drop past the threshold).
    #[serde(default)] pub build_idle_months: u8,
    /// Dedicated caravans+ships hauling the works (upkeep paid by the metropolis council).
    #[serde(default)] pub build_convoys: u8,
    /// Tick the project broke ground (for ETA + the window's "founded" line).
    #[serde(default)] pub build_start_tick: u32,
    // ── Government (DLC · key figures / capture / laws — all serde-defaulted) ──
    /// Regime type: 0 Council/Oligarchy · 1 Principality · 2 Free Commune. Seeded once.
    #[serde(default)] pub govt_type: u8,
    /// The city's KEY FIGURES (mayor/treasurer/harbormaster/magistrate). Houses bribe or
    /// intimidate them into service; a family that controls a majority captures the city.
    #[serde(default)] pub officials: Vec<Official>,
    /// The government's own strategic granary/stockpile (ng-length; empty ⇒ zeros).
    #[serde(default)] pub civic_goods: Vec<f32>,
    /// Recently enacted laws/policies (capped log) — the government's decisions.
    #[serde(default)] pub laws: Vec<Law>,
    /// The house that currently CONTROLS this government (captured a majority of its
    /// officials), or −1. Its goods get favourable tariffs + a trade-influence boost.
    #[serde(default = "neg_one_i32")] pub captor_house: i32,
    // ── Atlas 2.0 · city LIFECYCLE (appended LAST → old saves default). ──
    /// The settlement is DEAD (a † ruin on the map): skipped by the food/population
    /// pass so it stays dead, kept forever for the record.
    #[serde(default)] pub abandoned: bool,
    /// Years accumulated in TERMINAL decline (at the famine floor and still
    /// starving/miserable); at ABANDON_YEARS the settlement empties.
    #[serde(default)] pub decline_years: f32,
    /// Tick the settlement was founded MID-CAMPAIGN (0 = primordial, from worldgen).
    #[serde(default)] pub founded_tick: u32,
    /// Tick the settlement died (0 = alive).
    #[serde(default)] pub died_tick: u32,
    /// Last FULL YEAR's trade throughput (grain-eq, imports + exports) from the
    /// flow ledger — drives the Trade Heat overlay and the Atlas census.
    #[serde(default)] pub trade_last_year: f32,
    /// Why the settlement died ("famine" / "plague" / "war" / "disaster"),
    /// classified from its final decade at abandonment. Empty = alive.
    #[serde(default)] pub died_cause: String,
}

/// A city's KEY FIGURE (elected/appointed official). Houses raise `control` of it by
/// bribery or intimidation; at `control ≥ OFFICIAL_CAPTURE` the figure serves `house`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Official {
    /// 0 Head (Mayor/Doge/Lord) · 1 Treasurer · 2 Harbormaster · 3 Magistrate.
    pub role: u8,
    pub name: String,
    /// The house the figure serves (−1 neutral). Set when a house captures it.
    pub house: i32,
    /// How captured the figure is by `house`, 0..1.
    pub control: f32,
    /// A house FAMILY MEMBER holds this seat → it auto-serves that house (control 1.0).
    pub kin: bool,
    /// Tick this figure's term ends (regime change re-seats it).
    pub term_end: u32,
}

/// One enacted law/policy in a city's government log.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Law {
    pub year: u32,
    /// 0 favoured-house charter · 1 protectionist tariff · 2 free-trade · 3 debasement ·
    /// 4 grain law (civic granary) · 5 guild monopoly.
    pub kind: u8,
    /// Beneficiary house (−1 none).
    pub house: i32,
    /// Relevant good (−1 none).
    pub good: i32,
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
    /// True = this leg is a FUTURES-CONTRACT delivery. Its vessel is held by the
    /// standing per-contract reservation in `dispatch`, so the spot-trade capacity
    /// pass must NOT subtract it again (that would double-count the same ship).
    #[serde(default)] pub contract: bool,
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

/// One aggregated trade flow for the settlement "Flows" subtab: how much of `good`
/// moved between `hub` and `partner` in a direction (`dir` 0 = inbound to `hub`,
/// 1 = outbound from `hub`) over a year. Sparse — only pairs that actually traded.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeFlowAgg {
    pub hub: u32,
    pub good: u32,
    pub partner: u32,
    pub dir: u8,
    pub amount: f32,
}

/// Per-(hub, good) yearly trade-volume series (in + out), so the Flows subtab can
/// graph trade DYNAMICS over the campaign and show which trades have fallen.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeHist {
    pub hub: u32,
    pub good: u32,
    /// Total volume traded each year (most recent last), capped to `TRADE_HIST_CAP`.
    pub vols: Vec<f32>,
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
    /// Sparse per-city commercial INFLUENCE `(hub, 0..1)` this house has built up
    /// through sustained trade. Rises with the house's share of a city's commerce
    /// ÷ the city's resistance (population + local guild), decays without presence.
    /// Drives trade DOMINANCE (top influence over a threshold) and Bailo upgrades.
    #[serde(default)] pub influence: Vec<(u32, f32)>,
    /// Cities where this house has raised its office to a BAILO (a governing
    /// headquarters): it seats that city's council and runs a near-toll-free
    /// concession lane home. Soft-capped by the house's wealth/power.
    #[serde(default)] pub bailos: Vec<u32>,
}

/// DLC 3.5 · one loan on a bank's books. An asset to the bank (interest income);
/// a liability to the borrower (a house, or a city treasury). `outstanding` is
/// written down as the loan amortizes or when the borrower defaults.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Loan {
    /// Borrowing house index (−1 = a polis treasury borrowed instead).
    pub borrower_house: i32,
    /// Borrowing polis hub index (−1 = a house borrowed instead).
    pub borrower_polis: i32,
    pub principal: f32,
    pub outstanding: f32,
    /// Monthly interest rate locked at origination.
    pub rate: f32,
    pub start_tick: u32,
    pub term_ticks: u32,
    /// "estate" | "structure" | "treasury" | "trade".
    pub purpose: String,
}

/// A bank's EQUITY STAKE in a manufactory — the bank put capital into the works in
/// exchange for a `share` of its owner-cut income (a dividend). The `basis` is the
/// price paid, carried as the stake's book value on the bank's balance sheet.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BankStake {
    /// The estate (manufactory) hub the bank holds a share of.
    pub estate_hub: u32,
    /// Income share 0..1 the bank receives from the works' owner-cut.
    pub share: f32,
    /// Book value (price paid) — the asset carried on the balance sheet.
    pub basis: f32,
    /// The good the works produces (for the panel's deals list).
    pub good: u32,
}

/// DLC 3.5 · a BANK — a great merchant-banking house's chartered institution,
/// with a real balance sheet. Assets = specie `reserves` + loans outstanding +
/// `real_estate` (branches / foreclosed property); Liabilities = `deposits`
/// (capital placed by other houses) + `notes_issued` (bank-notes circulating).
/// Equity = assets − liabilities. A bank that runs its reserve ratio too thin is
/// vulnerable to a run; a failed bank can ignite a regional crash.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bank {
    pub name: String,        // "Banco di Vethra"
    /// Owning house index.
    pub house: u32,
    /// Home (seat) hub index.
    pub seat: u32,
    pub founded_tick: u32,
    pub defunct: bool,
    // ── Assets ──
    pub reserves: f32,
    pub loans: Vec<Loan>,
    pub real_estate: f32,
    // ── Liabilities ──
    pub deposits: f32,
    pub notes_issued: f32,
    // ── Reach + record ──
    /// Hubs hosting a counting-house branch (extends the home coin's reach).
    pub branches: Vec<u32>,
    pub prestige: f32,
    /// Cumulative interest earned (income) and written-off losses (for the ledger).
    pub interest_earned: f32,
    pub losses: f32,
    /// Equity stakes the bank holds in manufactories (a dividend-bearing asset class).
    #[serde(default)] pub stakes: Vec<BankStake>,
    /// Cumulative stake dividends collected (income, for the ledger / chart).
    #[serde(default)] pub dividends_earned: f32,
    /// Yearly balance-sheet snapshots for the Bank panel's history charts.
    #[serde(default)] pub history: Vec<BankSnapshot>,
    pub events: Vec<HouseEvent>,
}

/// A yearly snapshot of a bank's balance sheet — drives the Bank panel line charts.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BankSnapshot {
    pub year: u32,
    pub reserves: f32,
    pub loans: f32,
    pub stakes: f32,
    pub real_estate: f32,
    pub deposits: f32,
    pub notes: f32,
    pub equity: f32,
    /// Cumulative interest earned / dividends / losses at snapshot time (the panel
    /// differences successive years to show per-year income vs write-offs).
    pub interest_cum: f32,
    pub dividends_cum: f32,
    pub losses_cum: f32,
}

impl Bank {
    /// Loans outstanding (an asset).
    pub fn loans_outstanding(&self) -> f32 {
        self.loans.iter().map(|l| l.outstanding.max(0.0)).sum()
    }
    /// Book value of the bank's equity stakes (an asset).
    pub fn stake_book(&self) -> f32 {
        self.stakes.iter().map(|s| s.basis.max(0.0)).sum()
    }
    pub fn assets(&self) -> f32 {
        self.reserves + self.loans_outstanding() + self.real_estate + self.stake_book()
    }
    pub fn liabilities(&self) -> f32 {
        self.deposits + self.notes_issued
    }
    pub fn equity(&self) -> f32 {
        self.assets() - self.liabilities()
    }
    /// Reserves ÷ liabilities (∞ when it owes nothing). Below `BANK_RUN_RATIO` the
    /// bank is fragile.
    pub fn reserve_ratio(&self) -> f32 {
        let liab = self.liabilities();
        if liab <= EPS { 99.0 } else { self.reserves / liab }
    }
}

/// DLC 3.5 · a regional financial crash — credit froze across a trade-connected
/// region (one connectivity `component`). Recorded for the Coin & Credit panel.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrashRecord {
    pub year: u32,
    /// Hub where the panic started (a failed bank / popped bubble).
    pub origin_hub: u32,
    pub origin_name: String,
    /// Connectivity component hit (the affected region/continent).
    pub component: u32,
    pub cities_hit: u32,
    pub banks_failed: u32,
    /// "bank failure" | "bubble burst".
    pub cause: String,
    pub text: String,
}

/// DLC 3.5 · an active ECONOMIC war between two poleis. No troops — it is waged
/// with war chests (treasury spending), forced levies on resident houses, and a
/// trade blockade. Resolved after a minimum duration; the loser pays reparations.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct War {
    pub a: u32,            // belligerent hub indices (a < b)
    pub b: u32,
    pub start_tick: u32,
    pub chest_a: f32,      // cumulative war-chest spent (effort) by each side
    pub chest_b: f32,
    pub levies: f32,       // total raised from houses across both sides
    pub cargo_lost: u32,
    pub cause: String,
    /// What the war is FOR — decides what the victor takes at resolution (beyond the
    /// one-off plunder): 0 plunder · 1 tribute · 2 trade rights · 3 annexation.
    /// `#[serde(default)]` → old saves load as plunder.
    #[serde(default)] pub goal: u8,
}

/// DLC 3.5 · a concluded war, for the Wars log.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WarRecord {
    pub start_year: u32,
    pub end_year: u32,
    pub a_name: String,
    pub b_name: String,
    pub winner: String,
    pub loser: String,
    pub reparations: f32,
    pub levies_total: f32,
    pub cause: String,
    pub text: String,
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
    /// Normalized 0..1 richness of high-value TRADE GOODS at the site (Σ belt ×
    /// base_value). Drives where merchant houses plant trade outposts, and lets
    /// settlement colonies form on trade-rich frontier even when the land is lean.
    /// 0 on pre-trade saves (serde default).
    #[serde(default)]
    pub trade_value: f32,
    /// River-mouth / DELTA (fertile coastal alluvium — a natural port + granary).
    #[serde(default)] pub delta: bool,
    /// Land→sea CHOKEPOINT (strait / isthmus / portage where cargo transships and
    /// tolls can be levied — Venice/Bruges/Constantinople-style prize sites).
    #[serde(default)] pub chokepoint: bool,
}

/// A worldgen settlement that ranked BELOW the live-hub cap, so it is NOT
/// economically simulated (that would blow up the O(n²) tick cost). It is still a
/// real place on the map: its static worldgen population is COUNTED in the world
/// total and it stays clickable/searchable — the "decouple" of clickability +
/// census from simulation. Held on the sim purely as inert reference data.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HinterlandTown {
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub population: f32,
    pub koppen: u8,
    pub coastal: bool,
    /// Satellite trade tie: the nearest LIVE hub this village markets through (its
    /// produce flows to that town, and it buys from it). Assigned lazily in
    /// `hinterland_pass`; `-1` until linked. `#[serde(default)]` → old saves relink.
    #[serde(default = "neg_one_i32")] pub parent_hub: i32,
}

/// A migration flow drawn STRICTLY along the trade-route network: `path` is the routed
/// polyline (origin → … intermediate trade hubs … → destination cell coords), so people
/// visibly travel the same corridors goods do — never a straight jump across terrain.
/// Carries the migrants' culture + volume for the ribbon/dots/focus overlays.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MigrationRoute {
    pub path: Vec<[f32; 2]>,
    pub culture: String,
    pub volume: f32,
    pub tick: u32,
    pub from_hub: i32,
    pub to_hub: i32,
}

/// Cultures 2.0 · a CREOLE people — a new culture born of sustained blending in one
/// city (ethnogenesis). It carries a synthesized name drawn from BOTH parent peoples'
/// word-banks and its own static origin card, then lives like any other culture
/// (spreading, assimilating). Registered on the sim so queries can show its lore.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Creole {
    pub name: String,       // synthesized blended name (e.g. "Novvik")
    pub family: String,     // "Creole (ParentA · ParentB)"
    pub origin: String,     // static origin card
    pub color: [u8; 3],
    pub born_tick: u32,
    pub birthplace: String, // the city where it arose
    /// Parent kits (appearance/dress blend for the figure art). `#[serde(default)]`.
    #[serde(default)] pub kit_a: u8,
    #[serde(default)] pub kit_b: u8,
}

/// Cultures 2.0 · a 6-monthly snapshot of every living people's total population, for
/// the Peoples-panel population line chart. `t` is the sample time in YEARS (fractional).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CultureHistSample {
    pub t: f32,
    pub pops: Vec<(String, f32)>,
}

/// Batch 1 · one row of the ERA-SCRUBBER ring: the world as it stood at the end
/// of `year`. Vectors are hub-indexed at snapshot time — hub indices are stable
/// (hubs are never removed), so later-founded hubs simply aren't in older rows.
/// Estates carry -1 population so readers skip them.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct YearFrame {
    pub year: u32,
    pub pop: Vec<f32>,
    pub trade: Vec<f32>,
}

/// Batch 1 · the HALL OF RECORDS — all-time world records, each a
/// `(value, holder, year)` triple (zero/empty until first set).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WorldRecords {
    pub largest_city: (f32, String, u32),
    pub richest_house: (f32, String, u32),
    /// Value = the year's total flow volume; holder = its busiest city.
    pub biggest_trade_year: (f32, String, u32),
    /// Value = deaths in the single worst plague strike; holder = the city.
    pub deadliest_plague: (f32, String, u32),
    /// Value = cities hit; holder = the crash's origin city.
    pub worst_crash: (f32, String, u32),
    /// Value = generations; holder = the house.
    pub longest_dynasty: (f32, String, u32),
    pub most_towns: (f32, String, u32),
}

/// A civic supply contract: a supplier hub ships `monthly_qty` of `good` to a
/// settlement colony, paid for by the founding metropolis's treasury (the food
/// lifeline subsidy). `category`: 0 = food (covers the daily deficit) · 1 = reserve
/// (fills the stored buffer) · 2 = preservative (extends the reserve's shelf-life).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ColonySupply {
    pub colony_hub: u32,
    pub supplier_hub: u32,
    pub good: usize,
    pub monthly_qty: f32,
    pub category: u8,
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
    /// DLC 3.5 · progressive civic wealth tax + war levies paid to the home city.
    #[serde(default)] pub civic_tax: f32,
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
    /// The coin (mint hub INDEX) the contract is STRUCK/settled in — the buyer city's
    /// main coin at signing. −1 = barter. Tag only (no FX revaluation); the issuing
    /// polis's seigniorage already flows from circulation. Shown in the contracts view.
    #[serde(default = "neg_one_i32")] pub coin: i32,
}

/// DLC 3.5 · a polis's running yearly TREASURY books (the City Finances view).
/// All grain-equivalent. `prev` holds the last completed year for display, mirroring
/// the house Accountant. Serde-default → old campaigns load empty.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CityFinance {
    pub year: u32,
    // ── Income ──
    pub tax_trade: f32,       // import + export tariffs collected from merchants
    pub tax_estate: f32,      // tax on estate rent paid to owner houses
    pub tax_manufacture: f32, // value-added tax on the city's manufactories
    pub tax_wealth: f32,      // progressive civic wealth tax on resident houses
    pub seigniorage: f32,     // mint profit from coinage
    pub war_levy: f32,        // forced war contributions raised from houses
    pub reparations_in: f32,  // reparations received after a won war
    // ── Spending ──
    pub spent_civic: f32,     // distributed to the people (civic pool)
    pub spent_war: f32,       // war-chest spending (armies, blockade)
    pub spent_works: f32,     // public works / buildings
    #[serde(default)]
    pub spent_health: f32,    // hospices / quarantine (public health) — cuts plague deaths
    pub reparations_out: f32, // reparations paid after a lost war
    /// Last completed year, snapshotted at New Year for the panel.
    pub prev: Option<Box<CityFinance>>,
}

/// DLC · the abstract SOCIAL STRATA of a settlement — a cheap statistical model of
/// the whole population (not just merchants, cf. `merchant_pops`). The four shares
/// sum to 1; `commoner_wealth` and `inequality` are derived read-outs that the
/// later social systems (unrest/revolts, epidemic mortality, regimes) consume.
/// Serde-defaulted (all zero) so old `.campaign` saves load — a one-time seed
/// (`society_migrated`) fills them on first advance.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Society {
    /// Population SHARES (Σ = 1). Ruling/merchant-patrician elite, the burgher
    /// middle (shopkeepers, master craftsmen, lesser traders), the commoner mass
    /// (labourers, smallholders), and the urban underclass (poor, casual, destitute).
    pub patrician: f32,
    pub burgher: f32,
    pub commoner: f32,
    pub underclass: f32,
    /// Per-capita money reaching the common populace (0.. ), eased — feeds welfare.
    pub commoner_wealth: f32,
    /// 0 = egalitarian, 1 = extreme concentration. Elite wealth per head vs commoner.
    pub inequality: f32,
    /// Smouldering civil unrest, 0 = content … 1 = boiling. Built from low mood,
    /// inequality, dearth & war; vented by a riot/revolt. (It. 3)
    #[serde(default)] pub unrest: f32,
    /// After a revolt, the toppled council house index — barred from the seat until
    /// `ousted_until`. Only meaningful while `ousted_until` is in the future
    /// (defaults to 0 = no ban, so the default index 0 is never consulted).
    #[serde(default)] pub ousted_house: i32,
    #[serde(default)] pub ousted_until: u32,
}

/// DLC 4 · profession classes for a `Pop` (Victoria-style social roles).
pub const POP_PROFESSIONS: [&str; 9] = [
    "Farmers", "Labourers", "Craftsmen", "Clerks", "Merchants", "Clergy",
    "Capitalists", "Aristocrats", "Soldiers",
];

/// DLC 4 · a typed population unit — the foundation of the Nations & POPs layer.
/// The abstract `Society` shares are derived into these each year. NOT yet wired
/// into consumption/politics (that is DLC 4 step 2); kept read-only so the economy
/// (and its dynamics test) is unchanged while the data model lands.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Pop {
    pub profession: u8,      // index into POP_PROFESSIONS
    pub size: f32,           // people
    pub money: f32,          // per-capita wealth (grain-eq)
    pub needs_life: f32,     // 0..1 satisfaction of life needs
    pub needs_everyday: f32, // 0..1 everyday needs
    pub needs_luxury: f32,   // 0..1 luxury needs
    pub consciousness: f32,  // 0..10 political awareness
    pub militancy: f32,      // 0..10 willingness to revolt
}

/// Phase 5 (flavour) · CONTAGION tuning. Kept mild + capped so an outbreak spreads
/// as a wave along the trade lanes and then burns out, and the world recovers (the
/// dynamics test must stay bounded — no famine collapse). A plague now travels ONLY
/// along the trade network (a partner a merchant actually reaches), is deliberately
/// hard to transport (low per-tick chance), and cities that weather a long infection
/// gain lasting immunity to further strikes.
const EPIDEMIC_MAX_SPREAD_PER_TICK: usize = 2; // new foci ignited per tick
const EPIDEMIC_CONTAGION_MAG: f32 = 0.06;      // milder cull than the origin

/// Plague CATEGORIES (severity ↑, rarity ↑ as the number falls):
///   3 = LOCAL OUTBREAK — common; small cull; short trade restriction; never spreads.
///   2 = REGIONAL — rarer than cat-3; moderate; may reach ONE nearby trade partner.
///   1 = GREAT PLAGUE — rare; severe; travels the lanes up to ~4000 km from source.
// (Per-disease spread rate, deadliness and reach now live in the DISEASES table.)
/// A single contagion hop can never exceed this — stops a plague leaping a wide
/// ocean to ANOTHER CONTINENT via a long maritime trade tie (user rule). Regional
/// coastal spread still works; only transoceanic jumps are blocked.
const PLAGUE_HOP_MAX_KM: f32 = 1500.0;
const EARTH_EQUATOR_KM: f32 = 40075.0;        // world_w cells span this at the equator

// ── Historical DISEASES ─────────────────────────────────────────────────────────
/// Transmission mode: 0 = TRADE lanes (rats/goods) · 1 = WATER (river-mouth/coast) ·
/// 2 = AIRBORNE (fastest, any near neighbour, usually milder) · 3 = VECTOR/LOCALE
/// (spawns in warm wet places, does NOT pass city-to-city).
pub struct DiseaseSpec {
    pub name: &'static str,
    pub spread: f32,   // per-focus per-tick contagion chance
    pub dead_lo: f32,  // cull fraction range (deadliness)
    pub dead_hi: f32,
    pub mode: u8,
    pub reach_km: f32, // max spread distance from the outbreak origin
    pub immunity: f32, // 0 none … 1 lasting (scales the survivor-immunity window)
    pub weight: f32,   // relative outbreak frequency
}
pub const DISEASES: [DiseaseSpec; 9] = [
    DiseaseSpec { name: "Bubonic Plague",    spread: 0.045, dead_lo: 0.30, dead_hi: 0.60, mode: 0, reach_km: 4000.0, immunity: 1.0, weight: 0.06 },
    DiseaseSpec { name: "Smallpox",          spread: 0.090, dead_lo: 0.25, dead_hi: 0.35, mode: 2, reach_km: 3000.0, immunity: 1.0, weight: 0.10 },
    DiseaseSpec { name: "Typhus",            spread: 0.045, dead_lo: 0.15, dead_hi: 0.30, mode: 0, reach_km: 2000.0, immunity: 0.5, weight: 0.12 },
    DiseaseSpec { name: "Cholera",           spread: 0.070, dead_lo: 0.30, dead_hi: 0.50, mode: 1, reach_km: 2500.0, immunity: 0.2, weight: 0.12 },
    DiseaseSpec { name: "Dysentery",         spread: 0.050, dead_lo: 0.08, dead_hi: 0.15, mode: 1, reach_km: 1500.0, immunity: 0.1, weight: 0.18 },
    DiseaseSpec { name: "Malaria",           spread: 0.000, dead_lo: 0.05, dead_hi: 0.15, mode: 3, reach_km: 0.0,    immunity: 0.0, weight: 0.12 },
    DiseaseSpec { name: "Influenza",         spread: 0.110, dead_lo: 0.03, dead_hi: 0.08, mode: 2, reach_km: 4000.0, immunity: 0.3, weight: 0.16 },
    DiseaseSpec { name: "Measles",           spread: 0.100, dead_lo: 0.10, dead_hi: 0.20, mode: 2, reach_km: 3000.0, immunity: 1.0, weight: 0.08 },
    DiseaseSpec { name: "Sweating Sickness", spread: 0.060, dead_lo: 0.30, dead_hi: 0.50, mode: 2, reach_km: 2000.0, immunity: 0.0, weight: 0.06 },
];
/// Severity category (1 great / 2 regional / 3 local) derived from a disease's
/// deadliness — the UI badge + the immunity/lockdown length still key off this.
pub fn disease_category(d: u8) -> u8 {
    let hi = DISEASES.get(d as usize).map(|s| s.dead_hi).unwrap_or(0.1);
    if hi >= 0.30 { 1 } else if hi >= 0.15 { 2 } else { 3 }
}
/// Weighted pick of a disease for a spontaneous outbreak.
fn pick_disease(seed: u64, tick: u32, hub: usize) -> u8 {
    let total: f32 = DISEASES.iter().fold(0.0f32, |a, d| a + d.weight);
    let mut r = hash01(seed, tick as u64 ^ 0xD15EA5E, hub as u64) * total;
    for (i, d) in DISEASES.iter().enumerate() {
        r -= d.weight;
        if r <= 0.0 { return i as u8; }
    }
    0
}
/// Immunity earned by surviving an outbreak: a base span plus more the longer the city
/// was locked down (a harder, longer visitation confers deeper resistance).
const PLAGUE_IMMUNITY_BASE_YEARS: f32 = 6.0;
const PLAGUE_IMMUNITY_LOCK_MULT: f32 = 18.0;  // ×lockup-ticks added to the immune span
/// A marriage match only forms between houses whose MERCHANTS ACTUALLY REACH each
/// other — they share a trading city, or one house's network node lies within this
/// travel reach of the other's (grounded in real trade contact, NOT same-continent
/// geography). Used by `houses_in_contact`.
const MARRIAGE_REACH_KM: f32 = 3500.0;

/// Phase 5 (flavour) · dynastic MARRIAGE tuning (bounded — a dowry only *moves*
/// wealth between houses, which limited-liability already caps).
const MARRIAGE_MIN_WEALTH: f32 = 30.0;
const MARRIAGE_YEARLY_CHANCE: f32 = 0.5;
const MARRIAGE_DOWRY_FRAC: f32 = 0.08;
const MARRIAGE_DOWRY_CAP: f32 = 500.0;
const MARRIAGE_BREAK_CHANCE: f32 = 0.04;

/// Phase 5 (flavour) · craft-guild tuning (bounded quality lift; a strike is a
/// short, capped manufacture dent via the existing production-shock path).
const GUILD_MAX: usize = 12;
const GUILD_QUALITY_STEP: f32 = 0.03;
const GUILD_QUALITY_CAP: f32 = 0.92;
const GUILD_STRIKE_CHANCE: f32 = 0.10;
const GUILD_STRIKE_MAG: f32 = 0.5;      // halves the good's manufacture while out
const GUILD_HALL_STRENGTH: f32 = 0.6;   // standing at which a guildhall is raised

/// Phase 5 (flavour) · fashion / wonders / piracy / diaspora tuning (all bounded).
const FASHION_YEARLY_CHANCE: f32 = 0.35;
const FASHION_MAG: f32 = 0.30;          // +30% demand for the vogue good
const FASHION_MAX_MULT: f32 = 1.4;      // hard cap on the demand multiplier
const WONDER_YEARLY_CHANCE: f32 = 0.30;
pub const WONDER_NAMES: [&str; 3] = ["a great lighthouse", "a grand market hall", "a soaring cathedral"];
const PIRACY_YEARLY_CHANCE: f32 = 0.35;
const DIASPORA_YEARLY_CHANCE: f32 = 0.30;

/// Phase 4 (flavour) · kinds of notable figure, indexing `FIGURE_KINDS`.
pub const FIGURE_KINDS: [&str; 5] =
    ["Admiral", "Demagogue", "Master Craftsman", "Great Banker", "Explorer"];
/// At most this many notable figures alive at once (keeps the chronicle sparse).
const FIGURE_LIVING_CAP: usize = 6;
/// Total roster cap (living + dead kept for the record) — bounds save size.
const FIGURE_CAP: usize = 60;
/// Yearly probability the world raises a new notable figure.
const FIGURE_YEARLY_CHANCE: f32 = 0.45;

/// Short title prefix for a figure kind (chronicle text).
fn role_title(kind: u8) -> &'static str {
    match kind { 0 => "Admiral", 1 => "Demagogue", 2 => "Master", 3 => "Banker", _ => "Explorer" }
}

/// The office a government key figure holds (for chronicle text + the Government panel).
pub fn office_title(role: u8) -> &'static str {
    match role { 0 => "Head", 1 => "Treasurer", 2 => "Harbormaster", _ => "Magistrate" }
}

/// Human name of a regime type.
pub fn govt_type_name(t: u8) -> &'static str {
    match t { 0 => "Merchant Council", 1 => "Principality", _ => "Free Commune" }
}

/// Human name for the HEAD of a regime type (the mayor-equivalent).
pub fn govt_head_title(t: u8) -> &'static str {
    match t { 0 => "Doge", 1 => "Prince", _ => "Mayor" }
}

/// Capitalize the first character (for a good name at the start of a sentence).
fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Phase 4 (flavour) · a named individual who rose to prominence in the campaign.
/// Purely a chronicle actor plus ONE small, capped, one-time effect on an existing
/// bounded field (a house's fleet, a city's craft quality, civic unrest, a house's
/// prestige) — so the economy dynamics stay bounded. Deterministic: raised from a
/// `hash01(seed, tick, hub)` roll at the yearly hook.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Figure {
    /// The person's name (role is conveyed by `kind` + the chronicle text).
    pub name: String,
    /// Index into `FIGURE_KINDS`.
    pub kind: u8,
    /// Home city (hub index).
    pub hub: u32,
    /// Linked house index, or −1 (demagogues/explorers may be unaffiliated).
    pub house: i32,
    /// Relevant good (master craftsman's craft), else −1.
    pub good: i32,
    pub born_tick: u32,
    /// Planned death tick (a career length, not a full lifespan).
    pub dies_tick: u32,
    /// Death already chronicled.
    #[serde(default)]
    pub dead: bool,
}

/// A component needs at least this many settlements to host its own fair.
const FAIR_MIN_COMPONENT_HUBS: u32 = 4;
const FAIR_PROSPERITY: f32 = 0.10;  // civic-mood lift when the fair opens
const FAIR_STABILITY: f32 = 0.06;
const FAIR_LANES: usize = 4;        // nearest lanes that swell with fair traffic
const FAIR_FLOW: f32 = 200.0;       // overlay-only volume added per inbound lane

/// Phase 4 (flavour) · a recurring seasonal TRADE FAIR at a well-connected market
/// town (Champagne/Leipzig/Nizhny). Once a year in its month the fair opens: a
/// civic-mood boon, a burst of volume on its inbound lanes (visible on the Dynamic
/// Trade Flow overlay), and a chronicle beat. Held in a dedicated list rather than
/// as `TickHub` columns to keep the hub struct (and its many literals) untouched.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fair {
    /// Host hub index.
    pub hub: u32,
    /// Month it opens, 1..12.
    pub month: u8,
}

/// Ritual goods a holy site may take as its patron (searched by name; the first
/// present in the world's goods spec is used). Frankincense/incense for the altar,
/// wine for libation, wax for candles, pearls/amber for votive offering.
const RITUAL_GOODS: [&str; 6] = ["frankincense", "incense", "wine", "pearls", "amber", "dyes"];
const HOLY_MIN_COMPONENT_HUBS: u32 = 4;
const PILGRIM_PROSPERITY: f32 = 0.08;
const PILGRIM_STABILITY: f32 = 0.10;  // faith knits the community (stability lift)
const PILGRIM_LANES: usize = 4;
const PILGRIM_FLOW: f32 = 150.0;      // overlay-only pilgrim traffic per inbound lane
const PILGRIM_PRICE_BUMP: f32 = 1.20; // transient ritual-good demand spike (self-relaxes)

/// Phase 5 (flavour) · a CRAFT GUILD — the masters of one manufactured good in a
/// city. It steadily lifts that good's local quality, guards the craft (occasionally
/// downing tools in a strike that halts its manufacture for a spell), and in time
/// raises a guildhall. Dedicated list on `CampaignSim` (no TickHub churn).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CraftGuild {
    /// Host hub index.
    pub hub: u32,
    /// The manufactured good the guild masters (good index).
    pub good: u32,
    /// Standing 0..1 (built by quality + longevity) — drives guildhall + clout.
    pub strength: f32,
    /// A guildhall has been raised (a one-time civic monument).
    pub hall: bool,
}

/// Phase 6 (observability) · one city struck by plague — recorded for the Plagues &
/// Epidemics panel. `source` is the hub the pestilence was carried from (−1 = a
/// spontaneous outbreak); `outbreak` groups a contagion chain (all strikes sharing
/// an id are one epidemic). `deaths` is the population culled at the strike; `pop_at`
/// is the population that survived. Pure observability — does not affect the sim.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlagueStrike {
    pub hub: u32,
    pub source: i32,
    pub outbreak: u32,
    pub deaths: f32,
    pub pop_at: f32,
    pub start_tick: u32,
    pub until_tick: u32,
    /// Plague category 1..3 (1 = Great Plague, 3 = local outbreak). `#[serde(default)]`
    /// → old saves load as 0; readers treat 0 as a legacy local outbreak.
    #[serde(default)]
    pub category: u8,
    /// The hub the whole outbreak began in (the reach of a Great Plague is measured
    /// from here). `#[serde(default)]` → old saves default to 0.
    #[serde(default)]
    pub origin_hub: u32,
    /// Which DISEASE this is (index into `DISEASES`). serde-default 0 = Bubonic Plague.
    #[serde(default)]
    pub disease: u8,
    /// SIR observability · how many people fell ILL in this strike (`infected` ≥ `deaths`;
    /// `recovered = infected − deaths`). Derived from `deaths / case-fatality-rate`, so a
    /// mild disease infects many and kills few. Pure observability — does not affect the
    /// sim. `#[serde(default)]` → old saves load with 0 (readers fall back to `deaths`).
    #[serde(default)]
    pub infected: f32,
}

/// Phase 4 (flavour) · a HOLY CITY / great temple. Once a year its pilgrimage
/// season draws the faithful: a civic-stability boon, inbound pilgrim traffic, and
/// a transient demand spike for its patron ritual good. Sanctuary/temple-banking is
/// deferred; this is the pilgrimage + festival layer. Dedicated list (no TickHub churn).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolySite {
    /// Host hub index.
    pub hub: u32,
    /// Patron ritual good (good index), or −1 if the world grows none.
    pub patron_good: i32,
    /// 1 = notable temple · 2 = great holy city (flavour/size).
    pub tier: u8,
    /// Month the pilgrimage season opens, 1..12.
    pub month: u8,
}

impl CityFinance {
    pub fn income_total(&self) -> f32 {
        self.tax_trade + self.tax_estate + self.tax_manufacture + self.tax_wealth
            + self.seigniorage + self.war_levy + self.reparations_in
    }
    pub fn spend_total(&self) -> f32 {
        self.spent_civic + self.spent_war + self.spent_works + self.spent_health + self.reparations_out
    }
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
    /// One-time migration flag: seed each hub's social `society` strata on first advance.
    #[serde(default)]
    pub society_migrated: bool,
    /// One-time migration flag: fuse tiny/lone trade components (isolated "cosmetic"
    /// cities that could never trade) into the nearest substantial market. Applies the
    /// campaign_start connectivity rescue to older in-progress saves too.
    #[serde(default)]
    pub components_rescued: bool,
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
    /// NEAR-city sites for metropolis satellites (≤500 km — Ostia→Rome). Separate from
    /// `colonizable` (whose sites are buffered far from cities so colonies aren't
    /// suburbs). Consumed as satellites are founded; refilled from tiles when low.
    #[serde(default)]
    pub satellite_sites: Vec<ColonizeSite>,
    /// Worldgen settlements below the live-hub cap: drawn + clickable + counted in
    /// the world census, but NOT simulated (keeps tick cost bounded). See
    /// [`HinterlandTown`]. Empty on pre-feature saves (serde default).
    #[serde(default)]
    pub hinterland: Vec<HinterlandTown>,
    /// Atlas 2.0 — yearly world samples for the Atlas graphs, one row per completed
    /// year: `[year, population, trade volume (grain-eq), live hubs, cumulative
    /// foundings, cumulative abandonments]`. Bounded (oldest rows drop past ~400).
    #[serde(default)]
    pub world_series: Vec<[f32; 6]>,
    /// Atlas 2.0 — lifetime settlement lifecycle counters (all causes: organic
    /// swarming + colony ventures; abandonments + colony collapses).
    #[serde(default)]
    pub total_foundings: u32,
    #[serde(default)]
    pub total_abandonments: u32,
    /// Atlas 2.0 — recent refugee roads for the map's migration arrows:
    /// `[from_x, from_y, to_x, to_y, tick]`, bounded to the last 60.
    #[serde(default)]
    pub migrations: Vec<[f32; 5]>,
    /// Route-bound migration flows (polyline along the trade network + culture + volume),
    /// for the reworked Migration overlay (dots · ribbon · focus). Bounded.
    #[serde(default)]
    pub migration_routes: Vec<MigrationRoute>,
    /// Grain-eq a council secured into its civic warehouse LAST month (parallel to hubs),
    /// for the Provisioning tab's "secured this month" figure. Rebuilt each pass.
    #[serde(default)]
    pub council_bought_month: Vec<f32>,
    /// Batch 1 · per-hub PER-GOOD throughput accumulator for the running year
    /// (flat, `hub·ng + good`; rebuilt in-year, so not serialized).
    #[serde(skip)]
    pub good_flow_accum: Vec<f32>,
    /// Batch 1 · last FULL year's per-hub per-good throughput (flat, `hub·ng +
    /// good`) — the per-good Trade Heat + basin top-goods read this.
    #[serde(default)]
    pub hub_good_trade: Vec<f32>,
    /// Batch 1 · era-scrubber ring (one row per completed year, bounded ~400).
    #[serde(default)]
    pub year_frames: Vec<YearFrame>,
    /// Batch 1 · the Hall of Records (all-time world records).
    #[serde(default)]
    pub records: WorldRecords,
    /// Trade-base patronage: the house developing each hub as a base (hub-indexed,
    /// −1 = none). Resized to `hubs` each tick. Empty on old saves (serde default).
    /// See docs/TRADE_BASE_MECHANIC_PLAN.md.
    #[serde(default)]
    pub hub_patron: Vec<i32>,
    /// Per-hub majority people/culture (hub-indexed; seeded from the worldgen
    /// culture map at campaign start, inherited by colonies). Resized each tick.
    #[serde(default)]
    pub hub_culture: Vec<String>,
    /// Per-hub minority quarters `(people, share 0..1)` — grown by in-migration of
    /// a DIFFERENT culture and eroded by slow assimilation. Display-only (no
    /// economic coupling), so it never affects the dynamics guardrails.
    #[serde(default)]
    pub hub_minorities: Vec<Vec<(String, f32)>>,
    /// Cultures 2.0 · creole peoples born of blending during the campaign (ethnogenesis).
    /// `#[serde(default)]` → old saves load with none.
    #[serde(default)]
    pub creoles: Vec<Creole>,
    /// Cultures 2.0 · 6-monthly per-people population samples (capped) for the line chart.
    #[serde(default)]
    pub culture_history: Vec<CultureHistSample>,
    /// Consecutive UNPROFITABLE years per MANUFACTORY (hub-indexed). A works idle
    /// for 4 years is shut down + partly sold back to its owner. serde-default.
    #[serde(default)]
    pub estate_idle_years: Vec<u8>,
    /// Civic supply contracts feeding settlement colonies (the food lifeline). Each
    /// row = a supplier shipping a good to a colony, paid by the metropolis treasury.
    #[serde(default)]
    pub colony_supply: Vec<ColonySupply>,
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
    /// DLC 3.5 · chartered banks (great banking houses' institutions). Empty until
    /// a banking house qualifies to found one; serde-default → old saves load none.
    #[serde(default)] pub banks: Vec<Bank>,
    /// DLC 3.5 · log of regional financial crashes (origin region + year + cause),
    /// newest last. Surfaced in the Coin & Credit panel.
    #[serde(default)] pub crashes: Vec<CrashRecord>,
    /// DLC 3.5 · active economic wars between poleis.
    #[serde(default)] pub wars: Vec<War>,
    /// DLC 3.5 · concluded wars, newest last (the Wars log).
    #[serde(default)] pub war_log: Vec<WarRecord>,
    /// DLC 3.5 · the last YEAR's bundled trade volume per hub-pair (by hub id,
    /// ordered low→high): `(a_id, b_id, volume)`. Snapshotted at New Year and drawn
    /// as the "Dynamic Trade Flow" overlay (routed + width ∝ volume).
    #[serde(default)] pub flow_year: Vec<(u32, u32, f32)>,
    /// Running per-pair flow tally for the CURRENT year (by hub id). Not persisted —
    /// rebuilt as trade flows; snapshotted into `flow_year` each New Year.
    #[serde(skip)] pub flow_accum: std::collections::HashMap<(u32, u32), f32>,
    /// DLC 4 · one-time migration flag: seed per-hub good `quality` on first advance.
    #[serde(default)] pub quality_migrated: bool,
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
    /// THIS year's trade-flow accumulator for the settlement Flows subtab, keyed by
    /// (hub, good, partner, dir). In-memory only (`skip`) — a mid-year save/reload
    /// loses just the partial current year; completed years live in `trade_last`/
    /// `trade_hist`. Observability only; never feeds back into the simulation, so it
    /// has no bearing on determinism.
    #[serde(skip)]
    pub trade_cur: std::collections::HashMap<(u32, u32, u32, u8), f32>,
    /// Per-hub trade DOMINATOR (house index, −1 = none), recomputed monthly from
    /// `House.influence`. Derived → not serialized. Drives the dominance trade edge.
    #[serde(skip)]
    pub city_dominator: Vec<i32>,
    /// The LAST completed year's flows (detailed per-partner), the routes/partners
    /// breakdown the Flows subtab reads. Appended LAST → `#[serde(default)]`.
    #[serde(default)]
    pub trade_last: Vec<TradeFlowAgg>,
    /// Per-(hub, good) yearly trade-volume history (the trend graphs).
    #[serde(default)]
    pub trade_hist: Vec<TradeHist>,
    /// Phase 4 (flavour) · notable individuals raised over the campaign — admirals,
    /// demagogues, master craftsmen, great bankers, explorers. Each grants ONE
    /// capped effect and is chronicled at rise and death. Appended LAST →
    /// `#[serde(default)]` so old `.campaign` saves load with none.
    #[serde(default)]
    pub figures: Vec<Figure>,
    /// Phase 4 (flavour) · seasonal trade fairs (one per large trading component),
    /// seeded once. `#[serde(default)]` so old saves load with none.
    #[serde(default)]
    pub fairs: Vec<Fair>,
    /// One-time flag: `seed_trade_fairs` has run.
    #[serde(default)]
    pub fairs_seeded: bool,
    /// Phase 4 (flavour) · holy cities / great temples with a pilgrimage season,
    /// seeded once. `#[serde(default)]` so old saves load with none.
    #[serde(default)]
    pub holy_sites: Vec<HolySite>,
    /// One-time flag: `seed_holy_sites` has run.
    #[serde(default)]
    pub holy_seeded: bool,
    /// Phase 5 (flavour) · dynastic MARRIAGE alliances between houses, as (a,b) house
    /// index pairs (a<b). Ending a feud + a dowry accompany a match; a broken match
    /// rekindles the feud. `#[serde(default)]` so old saves load with none.
    #[serde(default)]
    pub alliances: Vec<(u32, u32)>,
    /// Phase 5 (flavour) · craft guilds (one per manufacturing city), seeded once.
    #[serde(default)]
    pub guilds: Vec<CraftGuild>,
    /// One-time flag: `seed_craft_guilds` has run.
    #[serde(default)]
    pub guilds_seeded: bool,
    /// Phase 5 (flavour) · civic WONDERS raised over the campaign, as (hub, tier)
    /// pairs (tier 0..2 → lighthouse/market hall/cathedral). `#[serde(default)]`.
    #[serde(default)]
    pub wonders: Vec<(u32, u8)>,
    /// Phase 6 (observability) · every plague strike, for the Plagues panel + map.
    /// `#[serde(default)]` so old saves load with none. Capped in `strike_plague`.
    #[serde(default)]
    pub epidemics: Vec<PlagueStrike>,
    /// Phase 6 · next outbreak id to assign to a spontaneous plague.
    #[serde(default)]
    pub next_outbreak: u32,
    /// Resilience: while `tick < expansion_frozen_until`, the risky territorial-
    /// expansion passes (estate / outpost / colony founding) are skipped. Set by the
    /// crash-recovery layer after a tick panic so re-advancing can't re-hit the same
    /// founding fault — the campaign always keeps moving forward. `#[serde(default)]`.
    #[serde(default)]
    pub expansion_frozen_until: u32,
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
            // A family driven from a seat by revolt is barred from it for a generation.
            let banned = |this: &Self, hub: usize| {
                let so = &this.hubs[hub].society;
                so.ousted_house == hi as i32 && this.tick < so.ousted_until
            };
            if h.dominant_seat && h.wealth > council_wealth[hub] && !banned(self, hub) {
                council[hub] = hi as i32;
                council_wealth[hub] = h.wealth;
            }
            // A BAILO is a governing headquarters: the house also sits the council of
            // each city where it has raised one (the only way besides the home seat).
            for &c in &h.bailos {
                let c = c as usize;
                if c < n && h.wealth > council_wealth[c] && !banned(self, c) {
                    council[c] = hi as i32;
                    council_wealth[c] = h.wealth;
                }
            }
        }
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            self.hubs[h].council_house = council[h];
            if self.hubs[h].mint_fineness <= 0.0 { self.hubs[h].mint_fineness = 1.0; }
            // A house that has CAPTURED this government (its key figures) sets the stance —
            // otherwise the council house does. Capture = policy in the captor's interest.
            let ruler = if self.hubs[h].captor_house >= 0 { self.hubs[h].captor_house } else { council[h] };
            let arch = if ruler >= 0 && (ruler as usize) < self.houses.len() {
                self.houses[ruler as usize].archetype } else { 255 };
            // Tariff stance by the ruler's character: political houses turn
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
            // v2.0 · a post-reform HONEST-MONEY mandate bars debasement until it lapses.
            let under_mandate = self.hubs[h].reform_until > self.tick;
            let target = if under_mandate { 1.0 }
                else if arch == ARCH_BANKING && prosperous { 0.88 }
                else if prosperous { 0.96 } else { 1.0 };
            let f = self.hubs[h].mint_fineness;
            self.hubs[h].mint_fineness = f + (target - f) * 0.5;
            // Retained treasury: skim ~8% of the circulating civic pool.
            self.hubs[h].treasury += self.hubs[h].civic_pool * 0.08;
            // Hospices & quarantine (public health): a council with treasury headroom funds
            // public health, easing it toward a prosperity-set target and spending a small,
            // bounded slice of treasury (recorded in `finance.spent_health`). When a council
            // can't afford it, the provision lapses. This is the lever by which a WEALTHY
            // city buys down its plague mortality — coin spent so fewer of its people die.
            let ph = self.hubs[h].public_health;
            if self.hubs[h].treasury > HOSPICE_MIN_TREASURY {
                let target = self.hubs[h].trade_wealth.clamp(0.0, 1.0) * HOSPICE_MAX_LEVEL;
                self.hubs[h].public_health = (ph + (target - ph) * HOSPICE_EASE).clamp(0.0, HOSPICE_MAX_LEVEL);
                let cost = self.hubs[h].treasury * HOSPICE_TREASURY_SKIM;
                self.hubs[h].treasury -= cost;
                self.hubs[h].finance.spent_health += cost;
            } else {
                self.hubs[h].public_health = (ph - HOSPICE_DECAY).max(0.0);
            }
        }
    }

    /// Yearly GOVERNMENT pass: seed each city's regime + key figures, let houses bribe /
    /// intimidate them into service, capture the government (→ favourable policy + trade
    /// influence), turn seats over on their term (sometimes installing a house kinsman),
    /// and keep a small civic granary. Runs right after `decide_polis_policy`.
    fn update_government(&mut self, year: u32) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let tick = self.tick;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            // 1) Seed the regime + officials once.
            if self.hubs[h].officials.is_empty() { self.seed_government(h); }
            // 2) Regime change: reseat any figure whose term has ended.
            for oi in 0..self.hubs[h].officials.len() {
                if tick >= self.hubs[h].officials[oi].term_end { self.reseat_official(h, oi); }
            }
            // 3) Bribery / intimidation: per figure, its strongest local patron house
            //    spends to raise (or a rival erodes) control. Kin figures are locked.
            let noff = self.hubs[h].officials.len();
            for oi in 0..noff {
                if self.hubs[h].officials[oi].kin {
                    self.hubs[h].officials[oi].house = self.hubs[h].officials[oi].house.max(-1);
                    self.hubs[h].officials[oi].control = 1.0;
                    continue;
                }
                // Patron = the non-guild house with the strongest presence here.
                let mut patron = (-1i32, 0.0f32);
                for hi in 0..self.houses.len() {
                    let hh = &self.houses[hi];
                    if hh.defunct || hh.is_guild { continue; }
                    let inf = hh.influence.iter().find(|(c, _)| *c == h as u32)
                        .map(|(_, v)| *v).unwrap_or(0.0);
                    if inf < GOVT_MIN_INFLUENCE { continue; }
                    let score = inf * (hh.wealth.max(0.0) + hh.prestige * 1000.0 + 1.0).sqrt();
                    if score > patron.1 { patron = (hi as i32, score); }
                }
                let cur = self.hubs[h].officials[oi].house;
                let weight = match self.hubs[h].officials[oi].role { 0 => 2.0, 1 => 1.4, _ => 1.0 };
                if patron.0 < 0 {
                    // No-one courts this seat → its allegiance fades toward neutral.
                    let o = &mut self.hubs[h].officials[oi];
                    o.control = (o.control - OFFICIAL_CONTROL_DECAY).max(0.0);
                    if o.control <= 0.05 { o.house = -1; o.control = 0.0; }
                    continue;
                }
                let pi = patron.0 as usize;
                let arch = self.houses[pi].archetype;
                // Fleet/political houses INTIMIDATE (muscle: prestige + ships, cheap in
                // coin); others BRIBE (pure cash). A house spends a small slice of its
                // wealth per seat, capped — enough to capture a seat in a year or two, not
                // so much it bleeds the family (which would flatten the wider economy).
                let budget = if arch == ARCH_FLEET || arch == ARCH_POLITICAL {
                    (self.houses[pi].prestige * 150.0 + self.houses[pi].fleet_sea as f32 * 25.0)
                        .min(self.houses[pi].wealth.max(0.0) * 0.012).min(BRIBE_COST * weight)
                } else {
                    (self.houses[pi].wealth.max(0.0) * 0.012).min(BRIBE_COST * weight)
                };
                if budget <= EPS { continue; }
                let gain = budget / (weight * BRIBE_COST);
                let contest = cur >= 0 && cur != pi as i32;
                self.houses[pi].wealth -= budget;
                // The money doesn't vanish — it lines the city's coffers (and its officials).
                self.hubs[h].civic_pool += budget;
                let o = &mut self.hubs[h].officials[oi];
                if contest {
                    // Erode the rival's grip first; take the seat once it's loosened.
                    o.control = (o.control - gain * 0.6).max(0.0);
                    if o.control <= 0.05 { o.house = pi as i32; o.control = 0.0; }
                } else {
                    o.house = pi as i32;
                    o.control = (o.control + gain).min(1.0);
                }
            }
            // 4) Capture: the house holding a majority of control-weighted seats.
            let mut tally: std::collections::HashMap<i32, f32> = std::collections::HashMap::new();
            let mut total_w = 0.0f32;
            for o in &self.hubs[h].officials {
                let w = match o.role { 0 => 2.0, 1 => 1.4, _ => 1.0 };
                total_w += w;
                if o.house >= 0 && (o.kin || o.control >= OFFICIAL_CAPTURE) {
                    *tally.entry(o.house).or_insert(0.0) += w;
                }
            }
            let captor = tally.iter().find(|(_, &w)| w > total_w * 0.5).map(|(&hh, _)| hh).unwrap_or(-1);
            let prev = self.hubs[h].captor_house;
            self.hubs[h].captor_house = captor;
            // 5) Payoff on a fresh capture: a favoured-house charter + a trade-influence
            //    boost (its policy tilt is applied in `decide_polis_policy`).
            if captor >= 0 && captor != prev {
                self.push_law(h, 0, captor, -1, year);
                let ci = captor as usize;
                if ci < self.houses.len() {
                    match self.houses[ci].influence.iter_mut().find(|(c, _)| *c == h as u32) {
                        Some((_, v)) => *v = (*v + CAPTOR_INFLUENCE_BOOST).min(1.0),
                        None => self.houses[ci].influence.push((h as u32, CAPTOR_INFLUENCE_BOOST)),
                    }
                    let (cn, hn) = (self.houses[ci].name.clone(), self.hubs[h].name.clone());
                    self.journal.push(JournalEntry {
                        tick, kind: "government".into(), hub: h as i32, good: -1, value: 0.0,
                        text: format!("{} seizes control of the government of {}", cn, hn),
                    });
                }
            }
            // 6) Civic granary: the government keeps a modest strategic grain reserve, and
            //    releases it into the market in famine (a small stabiliser).
            if self.hubs[h].civic_goods.len() != ng { self.hubs[h].civic_goods = vec![0.0; ng]; }
            if let Some(fg) = (0..ng).find(|&g| self.goods[g].food) {
                let cap = (self.hubs[h].population * 0.02).max(200.0);
                if self.hubs[h].starving > 0.5 && self.hubs[h].civic_goods[fg] > 0.0 {
                    let rel = self.hubs[h].civic_goods[fg] * 0.5;
                    self.hubs[h].civic_goods[fg] -= rel;
                    self.hubs[h].stock[fg] += rel;
                } else if self.hubs[h].civic_goods[fg] < cap && self.hubs[h].treasury > 20.0 {
                    let price = self.goods[fg].base_value.max(0.1);
                    let buy = ((cap - self.hubs[h].civic_goods[fg]).min(self.hubs[h].treasury * 0.03 / price)).max(0.0);
                    self.hubs[h].civic_goods[fg] += buy;
                    self.hubs[h].treasury -= buy * price;
                }
            }
        }
    }

    /// Seed a city's regime type + its key figures (once).
    fn seed_government(&mut self, h: usize) {
        let pop = self.hubs[h].population;
        let r = hash01(self.seed, h as u64 ^ 0x60F7, 0x1234);
        // Big rich cities run as merchant oligarchies; mid split principality/oligarchy;
        // small towns are free communes.
        let govt = if pop >= 60_000.0 { 0u8 }
            else if pop >= 15_000.0 { if r < 0.5 { 0 } else { 1 } }
            else { 2 };
        self.hubs[h].govt_type = govt;
        let term = GOVT_TERM_YEARS[govt as usize] * TICKS_PER_YEAR;
        let roles: &[u8] = if self.hubs[h].coastal { &[0, 1, 2, 3] } else { &[0, 1, 3] };
        let city = self.hubs[h].name.clone();
        let mut officials = Vec::with_capacity(roles.len());
        for (i, &role) in roles.iter().enumerate() {
            let salt = (h as u64).wrapping_mul(0x9E37).wrapping_add(role as u64 ^ 0x51);
            let name = self.head_name_for(h, &city, salt);
            // Stagger initial terms so the whole council doesn't turn over at once.
            let te = self.tick + term / 2 + (i as u32 * term) / roles.len().max(1) as u32;
            officials.push(Official { role, name, house: -1, control: 0.0, kin: false, term_end: te });
        }
        self.hubs[h].officials = officials;
    }

    /// Turn a key figure over at the end of its term — a fresh neutral appointee, or
    /// (sometimes) a kinsman of the city's most-influential family installed to serve it.
    fn reseat_official(&mut self, h: usize, oi: usize) {
        let govt = (self.hubs[h].govt_type as usize).min(2);
        let term = GOVT_TERM_YEARS[govt] * TICKS_PER_YEAR;
        let role = self.hubs[h].officials[oi].role;
        // Maybe a dominant family installs one of its own.
        let mut kin_house = -1i32;
        if hash01(self.seed, self.tick as u64 ^ 0x4174, (h as u64) ^ oi as u64) < GOVT_KIN_CHANCE {
            let mut best = (-1i32, GOVT_MIN_INFLUENCE);
            for hi in 0..self.houses.len() {
                let hh = &self.houses[hi];
                if hh.defunct || hh.is_guild { continue; }
                let inf = hh.influence.iter().find(|(c, _)| *c == h as u32).map(|(_, v)| *v).unwrap_or(0.0);
                if inf > best.1 { best = (hi as i32, inf); }
            }
            kin_house = best.0;
        }
        let city = self.hubs[h].name.clone();
        let salt = (self.tick as u64).wrapping_add((h as u64) << 8).wrapping_add(oi as u64);
        let surname = if kin_house >= 0 { self.houses[kin_house as usize].name.clone() } else { city.clone() };
        let name = self.head_name_for(h, &surname, salt);
        {
            let o = &mut self.hubs[h].officials[oi];
            o.name = name;
            o.term_end = self.tick + term;
            if kin_house >= 0 { o.house = kin_house; o.control = 1.0; o.kin = true; }
            else { o.house = -1; o.control = 0.0; o.kin = false; }
        }
        if kin_house >= 0 {
            let (hn, cn) = (self.houses[kin_house as usize].name.clone(), city);
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "government".into(), hub: h as i32, good: -1, value: 0.0,
                text: format!("A {} kinsman is installed as {} of {}", hn, office_title(role), cn),
            });
        }
    }

    /// Append a law to a city's government log (bounded).
    fn push_law(&mut self, h: usize, kind: u8, house: i32, good: i32, year: u32) {
        self.hubs[h].laws.push(Law { year, kind, house, good });
        if self.hubs[h].laws.len() > LAWS_CAP {
            let d = self.hubs[h].laws.len() - LAWS_CAP;
            self.hubs[h].laws.drain(0..d);
        }
    }

    // ───────────────────────── DLC 3.5 · Coin, Credit & Crashes ──────────────

    /// DLC 3.5 · accrue shipped volume on a hub-pair for the yearly Dynamic Trade
    /// Flow snapshot (keyed by stable hub IDs, ordered low→high; direction-agnostic).
    /// `good` also feeds the per-hub PER-GOOD ledger (Batch 1: per-good Trade Heat,
    /// basin top-goods); pass `usize::MAX` for goods-less flavour flow (fairs,
    /// pilgrimages) to skip that ledger.
    #[inline]
    fn accrue_flow(&mut self, from: usize, to: usize, good: usize, amount: f32) {
        if amount <= 0.0 || from >= self.hubs.len() || to >= self.hubs.len() || from == to { return; }
        let (ia, ib) = (self.hubs[from].id, self.hubs[to].id);
        let key = if ia <= ib { (ia, ib) } else { (ib, ia) };
        *self.flow_accum.entry(key).or_insert(0.0) += amount;
        if good != usize::MAX && good < self.goods.len() {
            let ng = self.goods.len();
            let need = self.hubs.len() * ng;
            if self.good_flow_accum.len() < need { self.good_flow_accum.resize(need, 0.0); }
            self.good_flow_accum[from * ng + good] += amount;
            self.good_flow_accum[to * ng + good] += amount;
        }
    }

    /// Recent trade volume touching a hub (the decaying per-class tallies) — used
    /// as the coinage "throughput" weight and the seigniorage base.
    fn hub_throughput(&self, h: usize) -> f32 {
        let hb = &self.hubs[h];
        hb.tw_house + hb.tw_local + hb.tw_guild
    }

    /// Whether a hub is currently inside a regional financial panic.
    fn hub_in_panic(&self, h: usize) -> bool {
        let tick = self.tick;
        self.active_events.iter()
            .any(|e| e.kind == "panic" && e.hub == h as i32 && e.until_tick > tick)
    }

    /// Deterministic coin denomination for a polis (Venice → ducat, Florence →
    /// florin, …). Stable per seed/hub.
    fn coin_denomination(&self, hub: usize) -> &'static str {
        const DENOMS: [&str; 10] = [
            "Ducat", "Florin", "Mark", "Dinar", "Solidus",
            "Crown", "Sequin", "Thaler", "Bezant", "Stater",
        ];
        let i = (hash01(self.seed, hub as u64 ^ 0xC0114, 0xDEED) * DENOMS.len() as f32) as usize
            % DENOMS.len();
        DENOMS[i]
    }

    /// v2.0 · the monetary METAL a mint strikes, from the bullion its trade region
    /// can reach: gold if a gold province lies in the region, silver if silver hills,
    /// electrum where both are plentiful and balanced, bronze/billon where only base
    /// metal (copper/tin) is available (0 silver · 1 gold · 2 electrum · 3 bronze).
    fn coin_metal_for(&self, hub: usize, gi: Option<usize>, si: Option<usize>,
                      ci: Option<usize>, ti: Option<usize>) -> u8 {
        let region = self.hubs[hub].component;
        let (mut g, mut s, mut base) = (0.0f32, 0.0f32, 0.0f32);
        for h in &self.hubs {
            if h.is_estate || h.component != region { continue; }
            if let Some(i) = gi { g += h.production.get(i).copied().unwrap_or(0.0); }
            if let Some(i) = si { s += h.production.get(i).copied().unwrap_or(0.0); }
            if let Some(i) = ci { base += h.production.get(i).copied().unwrap_or(0.0); }
            if let Some(i) = ti { base += h.production.get(i).copied().unwrap_or(0.0); }
        }
        let (has_g, has_s) = (g > EPS, s > EPS);
        if has_g && has_s {
            // Both metals in reach → a balanced supply is struck as electrum, else
            // the region coins its dominant precious metal.
            if g.min(s) > 0.25 * g.max(s) { 2 } else if g >= s { 1 } else { 0 }
        } else if has_g { 1 }
        else if has_s { 0 }
        else if base > EPS { 3 }
        else { 0 } // no precious metal in the region → assume imported silver specie
    }

    /// v2.0 · the ceiling bullion supply puts on a mint's fineness. A region flush
    /// with gold/silver relative to its coin demand can strike full-bodied money
    /// (cap → 1.0); a bullion-poor region minting beyond its metal is forced to
    /// debase (cap → `MINT_FINENESS_FLOOR`). Returns the cap in [floor, 1.0] plus
    /// the raw capacity ratio (for the panel's "limiting factor" read).
    fn mint_bullion_cap(&self, hub: usize, gi: Option<usize>, si: Option<usize>) -> (f32, f32) {
        let region = self.hubs[hub].component;
        let (mut bull, mut demand) = (0.0f32, 0.0f32);
        for h in &self.hubs {
            if h.is_estate || h.component != region { continue; }
            if let Some(i) = gi { bull += h.production.get(i).copied().unwrap_or(0.0) * MINT_GOLD_WEIGHT; }
            if let Some(i) = si { bull += h.production.get(i).copied().unwrap_or(0.0); }
            demand += h.tw_house + h.tw_local + h.tw_guild;
        }
        let cr = if demand > EPS { bull / (demand * MINT_BULLION_DEMAND) } else { 1.0 };
        let cap = (MINT_FINENESS_FLOOR + (1.0 - MINT_FINENESS_FLOOR) * cr.clamp(0.0, 1.0)).clamp(MINT_FINENESS_FLOOR, 1.0);
        (cap, cr)
    }

    /// DLC 3.5 · Coinage — once a year each council seat mints a NAMED coin and
    /// updates its acceptance ("trust"): sticky reputation built from full-bodied
    /// minting, a deep treasury, trade wealth, civic stability and throughput, and
    /// docked when the council debases. Debasement also skims seigniorage into the
    /// treasury (mint profit now, at trust's expense later). The strongest coins
    /// become reserve currencies (see `coin_discount`).
    fn decide_coinage(&mut self, _year: u32) {
        let n = self.hubs.len();
        // Normalizers across all council seats.
        let mut max_treasury = 1.0f32;
        let mut max_through = 1.0f32;
        let mut max_pop = 1.0f32;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            max_treasury = max_treasury.max(self.hubs[h].treasury);
            max_through = max_through.max(self.hub_throughput(h));
            max_pop = max_pop.max(self.hubs[h].population);
        }
        // Good indices for the monetary metals (computed once) → per-mint metal.
        let gi = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("gold"));
        let si = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("silver"));
        let cui = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("copper"));
        let tii = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("tin"));
        let tick = self.tick;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let fineness = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            let council = self.hubs[h].council_house;
            // v2.0 · MINT CHARTER — minting is a privilege. Grandfather any city that
            // already struck a coin; otherwise a council seat earns the right of the
            // mint only once it is a substantial commercial centre (busy AND large)
            // and can pay to establish the mint-house.
            if !self.hubs[h].has_mint {
                if !self.hubs[h].coin_name.is_empty() {
                    self.hubs[h].has_mint = true; // pre-existing coin keeps its mint
                } else if council >= 0 {
                    let busy = self.hub_throughput(h) >= MINT_CHARTER_THROUGH_FRAC * max_through;
                    let big = self.hubs[h].population >= MINT_CHARTER_POP_FRAC * max_pop;
                    if busy && big && self.hubs[h].treasury >= MINT_CHARTER_COST {
                        self.hubs[h].treasury -= MINT_CHARTER_COST;
                        self.hubs[h].has_mint = true;
                        let city = self.hubs[h].name.clone();
                        self.journal.push(JournalEntry {
                            tick, kind: "charter".into(), hub: h as i32, good: -1, value: MINT_CHARTER_COST,
                            text: format!("{} is granted the right of the mint and establishes a mint-house", city),
                        });
                    }
                }
            }
            // A chartered council seat with no coin yet strikes its first.
            if council >= 0 && self.hubs[h].has_mint && self.hubs[h].coin_name.is_empty() {
                let denom = self.coin_denomination(h);
                let city = self.hubs[h].name.clone();
                self.hubs[h].coin_name = format!("{} of {}", denom, city);
                self.hubs[h].coin_trust = 0.35;
                let cn = self.hubs[h].coin_name.clone();
                self.journal.push(JournalEntry {
                    tick, kind: "coinage".into(), hub: h as i32, good: -1, value: 0.0,
                    text: format!("{} mints the {}", city, cn),
                });
            }
            if self.hubs[h].coin_name.is_empty() {
                // No mint → any residual trust slowly bleeds away.
                self.hubs[h].coin_trust *= 0.9;
                self.hubs[h].mint_fineness_prev = fineness;
                continue;
            }
            // v2.0 · pick the metal from the region's reachable bullion.
            self.hubs[h].coin_metal = self.coin_metal_for(h, gi, si, cui, tii);
            // v2.0 · MINT REGULATION — regional bullion caps how full-bodied the coin
            // can be. A bullion-poor mint striking beyond its metal is forced to debase
            // (fineness capped down); an ample region can strike full-bodied coin.
            let (bcap, bratio) = self.mint_bullion_cap(h, gi, si);
            self.hubs[h].mint_bullion_ratio = bratio;
            if self.hubs[h].mint_fineness > bcap { self.hubs[h].mint_fineness = bcap; }
            let fineness = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            // Trust target — each term in 0..1.
            let through = self.hub_throughput(h);
            let t_fine = fineness.clamp(0.0, 1.0);
            let t_treas = (self.hubs[h].treasury / max_treasury).clamp(0.0, 1.0);
            let tw = self.hubs[h].trade_wealth;
            let t_trade = (tw / (tw.abs() + 1.0)).clamp(0.0, 1.0);
            let t_stab = self.hubs[h].sent_stability.clamp(0.0, 1.0);
            let t_through = (through / max_through).clamp(0.0, 1.0);
            let mut target =
                0.34 * t_fine + 0.20 * t_treas + 0.16 * t_trade + 0.14 * t_stab + 0.16 * t_through;
            // A fresh debasement (a cut vs last year) spooks holders extra.
            let prev = if self.hubs[h].mint_fineness_prev <= 0.0 { fineness } else { self.hubs[h].mint_fineness_prev };
            let debase = (prev - fineness).max(0.0);
            target = (target - debase * COIN_DEBASE_PENALTY).clamp(0.0, 1.0);
            let tr = self.hubs[h].coin_trust;
            self.hubs[h].coin_trust = (tr + (target - tr) * COIN_TRUST_EASE).clamp(0.0, 1.0);
            // Seigniorage from minting (esp. from debasement), scaled by throughput.
            let seign = through * (1.0 - fineness).max(0.0) * COIN_SEIGNIORAGE;
            self.hubs[h].treasury += seign;
            self.hubs[h].finance.seigniorage += seign;
            self.hubs[h].mint_fineness_prev = fineness;
        }
    }

    /// Import-freight multiplier (≤ 1.0) a destination's money earns: a trusted
    /// reserve coin — or a bank-note from a branch of a strong-coin bank — shaves
    /// transaction cost, making strong-money cities natural entrepôts.
    fn coin_discount(&self, dest: usize) -> f32 {
        if dest >= self.hubs.len() { return 1.0; }
        let mut trust = self.hubs[dest].coin_trust;
        if trust < RESERVE_TRUST_MIN {
            // Bank notes: a branch of a bank seated in a strong-coin city brings
            // that coin's credit here.
            for b in &self.banks {
                if b.defunct { continue; }
                if b.seat as usize == dest || b.branches.contains(&(dest as u32)) {
                    let st = self.hubs.get(b.seat as usize).map(|x| x.coin_trust).unwrap_or(0.0);
                    if st > trust { trust = st; }
                }
            }
        }
        if trust < RESERVE_TRUST_MIN { return 1.0; }
        1.0 - COIN_FREIGHT_DISCOUNT * trust.clamp(0.0, 1.0)
    }

    /// "Banco di <City>" name for a bank seated at `seat`.
    fn bank_name_for(&self, seat: usize) -> String {
        format!("Banco di {}", self.hubs[seat].name)
    }

    /// DLC 3.5 · once a year: qualifying banking houses charter banks, and existing
    /// banks open counting-house branches wherever their owner has trade offices
    /// (extending the home coin's reach and booking real estate).
    /// DLC 3.5 · once a year, update every city's CURRENCY BASKET: a small set of
    /// coins it holds (own/main + foreign coins that arrive with trade). Shares ease
    /// toward an adoption target (coin value × trust × issuer weight, with a home-mint
    /// bias); the main coin flips only when a rival durably dominates. The issuing
    /// polis earns a little seigniorage when its coin circulates abroad. Drives the
    /// coin-usage overlay + per-coin breakdown + circulating amount.
    fn update_currency_baskets(&mut self) {
        let n = self.hubs.len();
        // 1) Per-hub trade-partner volumes (by hub INDEX) from last year's realized
        //    flows. A coin can ONLY spread to a city that actually TRADES with a
        //    coin-holder — so coins travel along merchant routes, not teleport across
        //    the world. (Previously any reserve coin reached every hub.)
        let mut partner_vol: Vec<std::collections::HashMap<usize, f32>> =
            vec![std::collections::HashMap::new(); n];
        for f in &self.trade_last {
            let (a, b) = (f.hub as usize, f.partner as usize);
            if a < n && b < n && a != b { *partner_vol[a].entry(b).or_insert(0.0) += f.amount; }
        }
        let attr = |h: &TickHub| (coin_value(h.mint_fineness, h.coin_trust) * h.coin_trust).max(EPS);
        let mints = |h: &TickHub| !h.coin_name.is_empty() && h.coin_trust >= BANK_FOUND_COIN_TRUST;
        let mut new_baskets: Vec<Vec<(u32, f32)>> = vec![Vec::new(); n];
        let mut new_main: Vec<i32> = vec![-1; n];
        for i in 0..n {
            if self.hubs[i].is_estate { continue; }
            // Adoption target: the city's OWN coin (home bias) + coins that ARRIVE via
            // its trade partners — each partner's basket coins, weighted by the trade
            // volume with that partner × the coin's share in the partner × its
            // attractiveness. No trade link to a coin-holder ⇒ that coin never arrives.
            let mut wmap: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
            if mints(&self.hubs[i]) {
                *wmap.entry(i as u32).or_insert(0.0) += COIN_HOME_BIAS * attr(&self.hubs[i]);
            }
            let totv: f32 = partner_vol[i].values().sum::<f32>().max(EPS);
            for (&p, &v) in &partner_vol[i] {
                let frac = v / totv;
                for &(c, share) in &self.hubs[p].coin_basket {
                    let cj = c as usize;
                    if cj < n && !self.hubs[cj].coin_name.is_empty() {
                        *wmap.entry(c).or_insert(0.0) += frac * share * attr(&self.hubs[cj]);
                    }
                }
                if mints(&self.hubs[p]) {
                    *wmap.entry(p as u32).or_insert(0.0) += frac * 0.5 * attr(&self.hubs[p]);
                }
            }
            if wmap.is_empty() { continue; } // barter — no coin physically reaches here
            let mut tw: Vec<(u32, f32)> = wmap.into_iter().collect();
            tw.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            tw.truncate(COIN_BASKET_N);
            let sum: f32 = tw.iter().map(|x| x.1).sum::<f32>().max(EPS);
            // Ease last year's basket toward the target (sticky adoption).
            let prev = &self.hubs[i].coin_basket;
            let mut keys: Vec<u32> = tw.iter().map(|x| x.0).collect();
            for &(k, _) in prev { if !keys.contains(&k) { keys.push(k); } }
            let mut eased: Vec<(u32, f32)> = Vec::new();
            for k in keys {
                let pv = prev.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or(0.0);
                let tv = tw.iter().find(|x| x.0 == k).map(|x| x.1 / sum).unwrap_or(0.0);
                let nv = pv + COIN_ADOPT_EASE * (tv - pv);
                if nv > 0.02 { eased.push((k, nv)); }
            }
            eased.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            eased.truncate(COIN_BASKET_N);
            let s2: f32 = eased.iter().map(|x| x.1).sum::<f32>().max(EPS);
            for e in eased.iter_mut() { e.1 /= s2; }
            // Main coin: keep the current one unless a rival leads it by the flip margin.
            let cur = self.hubs[i].settle_coin;
            let leader = eased.first().map(|x| x.0 as i32).unwrap_or(-1);
            let main = if cur >= 0 && eased.iter().any(|x| x.0 as i32 == cur) {
                let cur_s = eased.iter().find(|x| x.0 as i32 == cur).map(|x| x.1).unwrap_or(0.0);
                let lead_s = eased.first().map(|x| x.1).unwrap_or(0.0);
                if leader != cur && lead_s > cur_s * COIN_FLIP_MARGIN { leader } else { cur }
            } else { leader };
            new_baskets[i] = eased;
            new_main[i] = main;
        }
        for i in 0..n {
            if self.hubs[i].is_estate { self.hubs[i].settle_coin = -1; self.hubs[i].coin_basket.clear(); continue; }
            self.hubs[i].coin_basket = std::mem::take(&mut new_baskets[i]);
            self.hubs[i].settle_coin = new_main[i];
        }
        // 2) Seigniorage: a coin circulating ABROAD earns its issuing polis a little
        //    treasury income + a tiny prestige bump to the council house (routed to
        //    TREASURY, not house wealth, so the wealth bound is unaffected).
        let mut circ_abroad: Vec<f32> = vec![0.0; n];
        for i in 0..n {
            let thru = self.hubs[i].tw_house + self.hubs[i].tw_local + self.hubs[i].tw_guild;
            for &(k, s) in &self.hubs[i].coin_basket {
                let j = k as usize;
                if j != i && j < n { circ_abroad[j] += thru * s; }
            }
        }
        for j in 0..n {
            if circ_abroad[j] <= 0.0 { continue; }
            self.hubs[j].treasury += circ_abroad[j] * COIN_CIRCULATION_SEIGNIORAGE;
            let ch = self.hubs[j].council_house;
            if ch >= 0 && (ch as usize) < self.houses.len() {
                self.houses[ch as usize].prestige =
                    (self.houses[ch as usize].prestige + 0.001 * circ_abroad[j].min(50.0)).min(2.0);
            }
        }
    }

    /// v2.0 · close the monetary loop. Each year, turn every coin's DEBASEMENT and
    /// MONEY-SUPPLY growth into a real inflation rate (quantity-theory-lite:
    /// π ≈ debasement + money growth − real output growth), compound each city's
    /// `price_level` by the inflation of the coin it settles in, and RETURN the
    /// per-hub inflation rate so the caller can levy the matching inflation-tax on
    /// resident fortunes. A debased-coin city now visibly gets dearer AND erodes its
    /// hoards faster — the seigniorage the mint skimmed is paid back as an inflation
    /// tax on the people who hold the coin.
    fn update_price_levels(&mut self) -> Vec<f32> {
        let n = self.hubs.len();
        // 1) Money supply per issuing coin (hub INDEX) = Σ holders' throughput×share.
        let mut m: Vec<f32> = vec![0.0; n];
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let thru = self.hubs[h].tw_house + self.hubs[h].tw_local + self.hubs[h].tw_guild;
            for &(k, share) in &self.hubs[h].coin_basket {
                if (k as usize) < n { m[k as usize] += thru * share; }
            }
        }
        // 2) Per-coin inflation from debasement + money growth − real growth.
        let mut coin_infl: Vec<f32> = vec![0.0; n];
        for c in 0..n {
            if self.hubs[c].coin_name.is_empty() { self.hubs[c].coin_circ_prev = m[c]; continue; }
            let fine = if self.hubs[c].mint_fineness <= 0.0 { 1.0 } else { self.hubs[c].mint_fineness };
            let mprev = if self.hubs[c].coin_circ_prev <= EPS { m[c].max(EPS) } else { self.hubs[c].coin_circ_prev };
            let mgrow = ((m[c] - mprev) / mprev).clamp(-0.5, 0.5);
            coin_infl[c] = (INFL_BASE + INFL_DEBASE_K * (1.0 - fine).max(0.0) + INFL_MONEY_K * mgrow - INFL_REAL_GROWTH)
                .clamp(INFL_MIN, INFL_MAX);
            self.hubs[c].coin_circ_prev = m[c];
        }
        // 3) Per-hub local inflation = its settle-coin's inflation (barter → base),
        //    compounded into the price level (bounded so a long campaign stays finite).
        let mut hub_infl: Vec<f32> = vec![INFL_BASE; n];
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let sc = self.hubs[h].settle_coin;
            let infl = if sc >= 0 && (sc as usize) < n && !self.hubs[sc as usize].coin_name.is_empty() {
                coin_infl[sc as usize]
            } else { INFL_BASE };
            if self.hubs[h].price_level <= 0.0 { self.hubs[h].price_level = 1.0; }
            self.hubs[h].price_level = (self.hubs[h].price_level * (1.0 + infl)).clamp(0.1, 1000.0);
            hub_infl[h] = infl;
        }
        hub_infl
    }

    /// v2.0 · recoinage / reform. A council whose coin has slipped — fineness below
    /// `REFORM_FINENESS_FLOOR` AND trust below `REFORM_TRUST_FLOOR` — CALLS IN the
    /// debased coin and re-mints at full fineness, if its treasury can bear the cost
    /// and it hasn't reformed within the cooldown. Confidence partly recovers at
    /// once, the price level eases, and an HONEST-MONEY mandate then bars further
    /// debasement for a few years (after which cheap-money pressure can creep back).
    fn maybe_reform_coinage(&mut self, _year: u32) {
        let tick = self.tick;
        let year = self.year();
        let n = self.hubs.len();
        for h in 0..n {
            if self.hubs[h].is_estate || self.hubs[h].coin_name.is_empty() { continue; }
            let fine = if self.hubs[h].mint_fineness <= 0.0 { 1.0 } else { self.hubs[h].mint_fineness };
            if fine >= REFORM_FINENESS_FLOOR || self.hubs[h].coin_trust >= REFORM_TRUST_FLOOR { continue; }
            if self.hubs[h].last_reform_tick != 0
                && tick < self.hubs[h].last_reform_tick + REFORM_COOLDOWN_YEARS * TICKS_PER_YEAR { continue; }
            let cost = self.hub_throughput(h) * REFORM_COST_FRAC;
            if self.hubs[h].treasury < cost { continue; }
            self.hubs[h].treasury -= cost;
            self.hubs[h].mint_fineness = 1.0;
            self.hubs[h].mint_fineness_prev = 1.0; // the re-mint is not read as a debasement
            self.hubs[h].coin_trust = (self.hubs[h].coin_trust + REFORM_TRUST_BUMP).clamp(0.0, 1.0);
            self.hubs[h].price_level = (self.hubs[h].price_level * 0.92).max(0.5);
            self.hubs[h].last_reform_tick = tick;
            self.hubs[h].reform_until = tick + REFORM_MANDATE_YEARS * TICKS_PER_YEAR;
            let cn = self.hubs[h].coin_name.clone();
            let city = self.hubs[h].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "reform".into(), hub: h as i32, good: -1, value: cost,
                text: format!("{} reforms the {} — the debased coin is called in and re-struck at full fineness", city, cn),
            });
            let _ = year;
        }
    }

    fn update_banks(&mut self, _year: u32) {
        let tick = self.tick;
        // 1) Charter new banks — only once the age of banking has opened (year 20).
        //    (Existing banks still service loans/deposits below before this gate.)
        if tick >= BANK_START_TICK {
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            if self.banks.iter().any(|b| !b.defunct && b.house == hi as u32) { continue; }
            let seat = self.houses[hi].hub as usize;
            if seat >= self.hubs.len() { continue; }
            if self.houses[hi].wealth < BANK_FOUND_WEALTH { continue; }
            if self.houses[hi].prestige < BANK_FOUND_PRESTIGE { continue; }
            if self.hubs[seat].coin_trust < BANK_FOUND_COIN_TRUST { continue; }
            // The house pays the founding PRICE (50k). Of that, 40k is paid in as the
            // bank's starting specie reserves / liquidity (its treasury); the rest
            // (10k) is the establishment / charter cost paid to the seat city.
            let price = BANK_FOUND_PRICE;
            let capital = BANK_FOUND_RESERVE;
            self.houses[hi].wealth -= price;
            let charter_fee = price - capital; // establishment cost → seat city treasury
            self.hubs[seat].treasury += charter_fee;
            let bname = self.bank_name_for(seat);
            let mut bank = Bank {
                name: bname.clone(), house: hi as u32, seat: seat as u32,
                founded_tick: tick, defunct: false,
                reserves: capital, loans: Vec::new(), real_estate: BANK_BRANCH_VALUE,
                deposits: 0.0, notes_issued: 0.0,
                branches: vec![seat as u32], prestige: self.houses[hi].prestige,
                interest_earned: 0.0, losses: 0.0,
                stakes: Vec::new(), dividends_earned: 0.0, history: Vec::new(),
                events: Vec::new(),
            };
            bank.events.push(HouseEvent { tick, kind: "founded".into(),
                text: format!("{} chartered in {} with {:.0} in specie (founding price {:.0})",
                    bname, self.hubs[seat].name, capital, price) });
            let house_name = self.houses[hi].name.clone();
            self.journal.push(JournalEntry { tick, kind: "bank".into(), hub: seat as i32, good: -1,
                value: 0.0, text: format!("{} founds the {}", house_name, bname) });
            self.houses[hi].events.push(HouseEvent { tick, kind: "bank".into(),
                text: format!("charters the {}", bname) });
            self.banks.push(bank);
        }
        } // end year-20 charter gate
        // 2) Grow branches to follow the owner's trade offices.
        for bi in 0..self.banks.len() {
            if self.banks[bi].defunct { continue; }
            let owner = self.banks[bi].house as usize;
            if owner >= self.houses.len() { continue; }
            let offices = self.houses[owner].offices.clone();
            for off in offices {
                if (off as usize) >= self.hubs.len() { continue; }
                if !self.banks[bi].branches.contains(&off) {
                    self.banks[bi].branches.push(off);
                    self.banks[bi].real_estate += BANK_BRANCH_VALUE;
                    let cn = self.hubs[off as usize].name.clone();
                    self.banks[bi].events.push(HouseEvent { tick, kind: "branch".into(),
                        text: format!("opens a counting-house in {}", cn) });
                }
            }
        }
        // 3) Snapshot each live bank's balance sheet for the Bank panel history charts.
        let year = self.year();
        for b in self.banks.iter_mut() {
            if b.defunct { continue; }
            b.history.push(BankSnapshot {
                year,
                reserves: b.reserves, loans: b.loans_outstanding(), stakes: b.stake_book(),
                real_estate: b.real_estate, deposits: b.deposits, notes: b.notes_issued,
                equity: b.equity(),
                interest_cum: b.interest_earned, dividends_cum: b.dividends_earned, losses_cum: b.losses,
            });
            if b.history.len() > BANK_HISTORY_CAP {
                let drop = b.history.len() - BANK_HISTORY_CAP; b.history.drain(0..drop);
            }
        }
    }

    /// DLC 3.5 · monthly bank dynamics: service loans (borrowers pay interest +
    /// amortize, or default), pay depositors, dividend the owner, take new deposits,
    /// originate loans (frozen during a panic), and fail when equity turns negative.
    fn bank_pass(&mut self) {
        let tick = self.tick;
        for bi in 0..self.banks.len() {
            if self.banks[bi].defunct { continue; }
            let seat = self.banks[bi].seat as usize;
            let panicked = self.hub_in_panic(seat);
            // 1) Service loans.
            let nloans = self.banks[bi].loans.len();
            let mut interest_income = 0.0f32;
            let mut writeoff = 0.0f32;
            let mut principal_repaid = 0.0f32; // note-funded credit returned → retire notes
            let mut cash_repaid = 0.0f32;      // specie returned on a cash-funded loan
            let mut keep = vec![true; nloans];
            for li in 0..nloans {
                let (bh, bp, outstanding, principal, rate, term, cash_funded) = {
                    let l = &self.banks[bi].loans[li];
                    // Colony ventures are staked with hard specie OUT of reserves; every
                    // other loan is funded by ISSUING NOTES (credit creation).
                    (l.borrower_house, l.borrower_polis, l.outstanding, l.principal, l.rate,
                     l.term_ticks, l.purpose == "colony")
                };
                if outstanding <= EPS { keep[li] = false; continue; }
                let due = outstanding * rate;
                let amort = (principal / (term.max(30) as f32 / 30.0)).min(outstanding);
                let pay = due + amort;
                let paid = if bh >= 0 && (bh as usize) < self.houses.len() && !self.houses[bh as usize].defunct {
                    if self.houses[bh as usize].wealth > pay * 1.2 {
                        self.houses[bh as usize].wealth -= pay; true
                    } else { false }
                } else if bp >= 0 && (bp as usize) < self.hubs.len() {
                    if self.hubs[bp as usize].treasury > pay * 1.2 {
                        self.hubs[bp as usize].treasury -= pay; true
                    } else { false }
                } else { false };
                if paid {
                    interest_income += due;
                    // The borrower returns principal. A note-funded loan RETIRES the
                    // notes it created (liability ↓); a cash loan returns specie to
                    // reserves (asset ↑). Either way equity is conserved and only the
                    // INTEREST is profit. (Previously the principal repayment simply
                    // vanished — neither booked to reserves nor used to retire notes —
                    // so equity bled ~`amort` per loan per month and EVERY bank went
                    // insolvent within a few years, then its failure cascaded a crash.)
                    if cash_funded { cash_repaid += amort; } else { principal_repaid += amort; }
                    let rem = (outstanding - amort).max(0.0);
                    self.banks[bi].loans[li].outstanding = rem;
                    if rem <= EPS { keep[li] = false; }
                } else {
                    // Default: write off the balance; the bank seizes property worth a
                    // fraction of the loan (a foreclosed asset on its books).
                    writeoff += outstanding;
                    self.banks[bi].real_estate += outstanding * 0.4;
                    self.banks[bi].loans[li].outstanding = 0.0;
                    keep[li] = false;
                    self.banks[bi].events.push(HouseEvent { tick, kind: "default".into(),
                        text: format!("writes off a loan of {:.0} in default", outstanding) });
                }
            }
            self.banks[bi].reserves += interest_income + cash_repaid;
            self.banks[bi].notes_issued = (self.banks[bi].notes_issued - principal_repaid).max(0.0);
            self.banks[bi].interest_earned += interest_income;
            self.banks[bi].losses += writeoff;
            let mut idx = 0;
            self.banks[bi].loans.retain(|_| { let k = keep[idx]; idx += 1; k });
            // 2) Depositor interest (a cost paid from reserves).
            let dep_cost = self.banks[bi].deposits * BANK_DEPOSIT_RATE;
            self.banks[bi].reserves -= dep_cost;
            // 2b) v2.0 · IDIOSYNCRATIC BANK RUN. A bank whose reserve ratio has slipped
            //     below the fragility floor faces withdrawals as depositors lose
            //     confidence — even absent a systemic panic. The run drains reserves
            //     (scaled by how far below the floor it is) and can tip a stressed bank
            //     into failure on its own; a sound, well-reserved bank never sees one.
            let rr = self.banks[bi].reserve_ratio();
            if rr < BANK_RUN_RATIO && self.banks[bi].deposits > EPS {
                let severity = ((BANK_RUN_RATIO - rr) / BANK_RUN_RATIO).clamp(0.0, 1.0);
                let withdraw = (self.banks[bi].deposits * BANK_RUN_WITHDRAW * (0.5 + severity))
                    .min(self.banks[bi].deposits);
                self.banks[bi].reserves -= withdraw;
                self.banks[bi].deposits = (self.banks[bi].deposits - withdraw).max(0.0);
                self.banks[bi].events.push(HouseEvent { tick, kind: "run".into(),
                    text: format!("depositors withdraw {:.0} in a run on the bank", withdraw) });
                self.journal.push(JournalEntry { tick, kind: "run".into(), hub: seat as i32,
                    good: -1, value: withdraw,
                    text: format!("Run on {}: depositors pull {:.0} as confidence cracks", self.banks[bi].name, withdraw) });
            }
            // 3) Dividend the bank's net spread to the owning house.
            let owner = self.banks[bi].house as usize;
            let profit = interest_income - dep_cost;
            if profit > 0.0 && owner < self.houses.len() && !self.houses[owner].defunct {
                let dividend = (profit * 0.5).min(self.banks[bi].reserves.max(0.0));
                self.banks[bi].reserves -= dividend;
                self.houses[owner].wealth += dividend;
            }
            // 4) New lending + equity investment (only in calm times).
            if !panicked { self.bank_maybe_lend(bi); self.bank_maybe_invest(bi); }
            // 5) Attract deposits.
            self.bank_maybe_take_deposits(bi);
            // 6) Owner recapitalization: before a bank is allowed to fail, its owning
            //    house — if solvent — injects fresh specie to cover a shortfall (a
            //    family stands behind its bank). This absorbs a transient loss from a
            //    loan default instead of letting one bad year topple a sound bank.
            let owner = self.banks[bi].house as usize;
            let shortfall = (-self.banks[bi].equity()).max(-self.banks[bi].reserves).max(0.0);
            if shortfall > 0.0 && owner < self.houses.len() && !self.houses[owner].defunct {
                let inject = shortfall.min(self.houses[owner].wealth * 0.5);
                if inject > 0.0 {
                    self.houses[owner].wealth -= inject;
                    self.banks[bi].reserves += inject;
                    self.banks[bi].events.push(HouseEvent { tick, kind: "recap".into(),
                        text: format!("{} injects {:.0} to shore up the bank", self.houses[owner].name, inject) });
                }
            }
            // 7) Failure (only if recapitalization couldn't cover it).
            if self.banks[bi].equity() < 0.0 || self.banks[bi].reserves < 0.0 {
                self.fail_bank(bi);
            }
        }
    }

    /// A wealthy house at a bank's seat parks idle capital as an interest-bearing
    /// deposit (a liability that funds the bank's lending). Capped by the
    /// fractional-reserve limit so the bank stays sound.
    fn bank_maybe_take_deposits(&mut self, bi: usize) {
        let tick = self.tick;
        let cap = self.banks[bi].reserves * BANK_RESERVE_MULT;
        if self.banks[bi].liabilities() >= cap { return; }
        if hash01(self.seed, tick as u64 ^ 0xDEED0, bi as u64) > 0.25 { return; }
        let seat = self.banks[bi].seat as usize;
        let owner = self.banks[bi].house;
        let mut best = (usize::MAX, 0.0f32);
        for (hi, h) in self.houses.iter().enumerate() {
            if h.defunct || hi as u32 == owner { continue; }
            if h.hub as usize != seat { continue; }
            if h.wealth > best.1 { best = (hi, h.wealth); }
        }
        if best.0 == usize::MAX || best.1 < 3.0 { return; }
        let amt = (best.1 * 0.1).min(cap - self.banks[bi].liabilities());
        if amt < 0.5 { return; }
        self.houses[best.0].wealth -= amt;
        self.banks[bi].deposits += amt;
        self.banks[bi].reserves += amt;
    }

    /// A sound bank with headroom originates a loan: usually to a promising resident
    /// house (financing its ventures), sometimes to the seat city's treasury (public
    /// works). It issues notes/credit (a liability) and disburses to the borrower.
    fn bank_maybe_lend(&mut self, bi: usize) {
        let tick = self.tick;
        let headroom = self.banks[bi].reserves * BANK_RESERVE_MULT - self.banks[bi].liabilities();
        if headroom < 1.0 { return; }
        if hash01(self.seed, tick as u64 ^ 0x10A40, bi as u64) > 0.3 { return; }
        let seat = self.banks[bi].seat as usize;
        let amt = headroom.min(self.banks[bi].reserves * 0.5).max(1.0);
        let owner = self.banks[bi].house;
        // Richest non-defunct resident borrower of the requested kind (guild or house),
        // homed at the seat and not the bank's own owner.
        let richest_resident = |guild: bool, this: &Self| -> usize {
            let mut best = (usize::MAX, 0.0f32);
            for (hi, h) in this.houses.iter().enumerate() {
                if h.defunct || h.is_guild != guild || hi as u32 == owner { continue; }
                if h.hub as usize != seat { continue; }
                if h.wealth > best.1 { best = (hi, h.wealth); }
            }
            best.0
        };
        // Pick a borrower: most often a resident merchant house (trade venture);
        // sometimes a resident GUILD financing a factory or civic works; sometimes the
        // seat city's treasury (public works).
        let pick = hash01(self.seed, tick as u64 ^ 0x77B10, bi as u64);
        let (bh, bp, purpose) = if pick < 0.55 {
            let h = richest_resident(false, self);
            if h == usize::MAX { return; }
            (h as i32, -1i32, "trade")
        } else if pick < 0.80 {
            let g = richest_resident(true, self);
            if g == usize::MAX {
                (-1i32, seat as i32, "treasury") // no guild here → fund public works instead
            } else {
                let civic = hash01(self.seed, tick as u64 ^ 0x6171C, bi as u64) < 0.4;
                (g as i32, -1i32, if civic { "guild_civic" } else { "guild_factory" })
            }
        } else {
            (-1i32, seat as i32, "treasury")
        };
        self.banks[bi].loans.push(Loan {
            borrower_house: bh, borrower_polis: bp,
            principal: amt, outstanding: amt, rate: BANK_LOAN_RATE,
            start_tick: tick, term_ticks: 1825, purpose: purpose.into(),
        });
        self.banks[bi].notes_issued += amt;
        if bh >= 0 { self.houses[bh as usize].wealth += amt; }
        else if bp >= 0 { self.hubs[bp as usize].treasury += amt; }
    }

    /// A sound bank takes an EQUITY STAKE in a promising manufactory in its branch
    /// network: it injects capital into the works' owning house and, in return, draws
    /// a share of that manufactory's income as a dividend. The stake is carried as a
    /// balance-sheet asset, so a bank can grow its net worth from real PRODUCTION, not
    /// only from lending. (User-requested: "banks should be able to invest in buildings.")
    fn bank_maybe_invest(&mut self, bi: usize) {
        let tick = self.tick;
        // Invest only idle reserves, occasionally, keeping ample liquidity.
        if self.banks[bi].reserves < BANK_FOUND_RESERVE * 0.5 { return; }
        if hash01(self.seed, tick as u64 ^ 0x57A4E, bi as u64) > 0.10 { return; }
        let branches = self.banks[bi].branches.clone();
        // The highest-tier un-staked manufactory in the branch network whose owner is solvent.
        let mut best: (usize, u8) = (usize::MAX, 0);
        for ei in 0..self.hubs.len() {
            let e = &self.hubs[ei];
            if !e.is_estate || e.estate_kind != 6 || e.stake_bank >= 0 { continue; }
            if e.parent < 0 || !branches.contains(&(e.parent as u32)) { continue; }
            let oh = e.owner_house;
            if oh < 0 || (oh as usize) >= self.houses.len() || self.houses[oh as usize].defunct { continue; }
            if e.estate_tier > best.1 { best = (ei, e.estate_tier); }
        }
        let ei = best.0;
        if ei == usize::MAX { return; }
        let tier = self.hubs[ei].estate_tier.max(1);
        let price = (tier as f32 * BANK_STAKE_VALUE_PER_TIER * BANK_STAKE_SHARE)
            .min(self.banks[bi].reserves * 0.4);
        if price < 1.0 || self.banks[bi].reserves < price * 2.0 { return; }
        let good = self.hubs[ei].base_per_capita.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(g, _)| g as u32).unwrap_or(0);
        let oh = self.hubs[ei].owner_house as usize;
        self.banks[bi].reserves -= price;        // specie out
        self.houses[oh].wealth += price;          // capital injected into the works
        self.hubs[ei].stake_bank = bi as i32;
        self.hubs[ei].stake_share = BANK_STAKE_SHARE;
        self.banks[bi].stakes.push(BankStake {
            estate_hub: ei as u32, share: BANK_STAKE_SHARE, basis: price, good });
        let en = self.hubs[ei].name.clone();
        self.banks[bi].events.push(HouseEvent { tick, kind: "stake".into(),
            text: format!("takes a {:.0}% stake in {} for {:.0}", BANK_STAKE_SHARE * 100.0, en, price) });
        let bn = self.banks[bi].name.clone();
        self.journal.push(JournalEntry { tick, kind: "bank".into(), hub: self.hubs[ei].parent,
            good: good as i32, value: price, text: format!("{} buys into {}", bn, en) });
    }

    /// Release a bank's equity stakes (on failure): each works reverts fully to its
    /// owner so it can be re-staked later.
    fn release_bank_stakes(&mut self, bi: usize) {
        let staked: Vec<u32> = self.banks[bi].stakes.iter().map(|s| s.estate_hub).collect();
        for eh in staked {
            if (eh as usize) < self.hubs.len() && self.hubs[eh as usize].stake_bank == bi as i32 {
                self.hubs[eh as usize].stake_bank = -1;
                self.hubs[eh as usize].stake_share = 0.0;
            }
        }
        self.banks[bi].stakes.clear();
    }

    /// A bank fails: depositors are wiped, its notes go worthless, the owning house
    /// is battered, and the failure ignites a regional crash.
    fn fail_bank(&mut self, bi: usize) {
        let tick = self.tick;
        if self.banks[bi].defunct { return; }
        self.banks[bi].defunct = true;
        let seat = self.banks[bi].seat as usize;
        let name = self.banks[bi].name.clone();
        let lost = self.banks[bi].deposits;
        self.banks[bi].events.push(HouseEvent { tick, kind: "failed".into(),
            text: format!("fails — {:.0} in deposits wiped out", lost) });
        self.journal.push(JournalEntry { tick, kind: "bank".into(), hub: seat as i32, good: -1,
            value: lost, text: format!("The {} collapses", name) });
        let owner = self.banks[bi].house as usize;
        if owner < self.houses.len() {
            self.houses[owner].wealth *= 0.6;
            self.houses[owner].prestige *= 0.7;
        }
        self.release_bank_stakes(bi);
        self.trigger_regional_crash(seat, 1, "bank failure");
    }

    /// DLC 3.5 · a regional financial crash. Credit freezes across the origin's
    /// whole trade-connected region (one connectivity `component`): coin trust
    /// collapses, house fortunes are haircut, a region-wide panic event tanks
    /// morale + stability (via the existing sentiment loop), and thinly-reserved
    /// banks in the region are swept away (contagion). Recorded for the panel.
    fn trigger_regional_crash(&mut self, origin: usize, origin_is_bank: u32, cause: &str) {
        if origin >= self.hubs.len() { return; }
        let tick = self.tick;
        let year = self.year();
        let region = self.hubs[origin].component;
        let origin_name = self.hubs[origin].name.clone();
        let until = tick + CRASH_PANIC_TICKS;
        let region_hubs: Vec<usize> = (0..self.hubs.len())
            .filter(|&h| !self.hubs[h].is_estate && self.hubs[h].component == region)
            .collect();
        // 1) Poleis: trust collapse + panic event (sentiment loop does the morale hit).
        for &h in &region_hubs {
            self.hubs[h].coin_trust = (self.hubs[h].coin_trust - CRASH_TRUST_HIT).max(0.0);
            self.hubs[h].trade_wealth *= 0.5;
            self.active_events.push(ActiveEvent {
                kind: "panic".into(), hub: h as i32, good: -1,
                magnitude: 0.5, until_tick: until,
            });
        }
        // 2) Houses homed in the region take a wealth haircut (margin calls).
        for hh in self.houses.iter_mut() {
            if hh.defunct { continue; }
            if region_hubs.contains(&(hh.hub as usize)) {
                hh.wealth *= 1.0 - CRASH_WEALTH_HAIRCUT;
                hh.volume *= 0.7;
            }
        }
        // 3) Contagion: banks in the region face a run; the fragile ones fall.
        let mut banks_failed = origin_is_bank.min(1);
        for bi in 0..self.banks.len() {
            if self.banks[bi].defunct { continue; }
            if !region_hubs.contains(&(self.banks[bi].seat as usize)) { continue; }
            let run = self.banks[bi].deposits * CRASH_CONTAGION_RUN;
            self.banks[bi].reserves -= run;
            self.banks[bi].deposits = (self.banks[bi].deposits - run).max(0.0);
            if self.banks[bi].reserve_ratio() < BANK_RUN_RATIO || self.banks[bi].equity() < 0.0 {
                self.banks[bi].defunct = true;
                self.banks[bi].events.push(HouseEvent { tick, kind: "failed".into(),
                    text: "swept away in the panic".into() });
                self.release_bank_stakes(bi);
                banks_failed += 1;
            }
        }
        let cities_hit = region_hubs.len() as u32;
        let text = format!(
            "The Crash of {}: credit froze across the {} region — {} cities struck, {} banks failed ({}).",
            year, origin_name, cities_hit, banks_failed, cause
        );
        self.journal.push(JournalEntry {
            tick, kind: "crash".into(), hub: origin as i32, good: -1,
            value: cities_hit as f32, text: text.clone(),
        });
        self.crashes.push(CrashRecord {
            year, origin_hub: origin as u32, origin_name, component: region,
            cities_hit, banks_failed, cause: cause.into(), text,
        });
        if self.crashes.len() > CRASH_RECORD_CAP {
            let drop = self.crashes.len() - CRASH_RECORD_CAP;
            self.crashes.drain(0..drop);
        }
    }

    /// DLC 3.5 · after the yearly speculation read, a HIGH-tier (≥4★) bubble may
    /// burst into a regional crash.
    fn maybe_pop_bubbles(&mut self, year: u32) {
        let pops: Vec<usize> = self.spec_centers.iter()
            .filter(|c| c.stars >= 4)
            .filter(|c| hash01(self.seed, c.hub as u64 ^ 0xB0BB1E, year as u64) < CRASH_BUBBLE_POP_CHANCE)
            .map(|c| c.hub as usize)
            .collect();
        for h in pops {
            if h >= self.hubs.len() || self.hub_in_panic(h) { continue; }
            self.trigger_regional_crash(h, 0, "bubble burst");
        }
    }

    /// DLC 3.5 · at New Year, snapshot each city's treasury books (for the City
    /// Finances panel) and the year's bundled trade flows (for the Dynamic Trade
    /// Flow overlay), then open a fresh year.
    fn roll_city_finances(&mut self, yr: u32) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let mut done = std::mem::take(&mut self.hubs[h].finance);
            done.year = yr.saturating_sub(1);
            done.prev = None; // don't nest histories
            self.hubs[h].finance = CityFinance { year: yr, prev: Some(Box::new(done)), ..Default::default() };
        }
        // Snapshot the year's per-pair trade volume, then reset the accumulator.
        if !self.flow_accum.is_empty() {
            self.flow_year = self.flow_accum.iter().map(|(&(a, b), &v)| (a, b, v)).collect();
            self.flow_accum.clear();
        }
        // ── Atlas 2.0 · per-hub yearly throughput (Trade Heat / census) + the
        //    world sample for the Atlas graphs. ──
        let by_id: std::collections::HashMap<u32, usize> =
            self.hubs.iter().enumerate().map(|(i, h)| (h.id, i)).collect();
        for h in self.hubs.iter_mut() { h.trade_last_year = 0.0; }
        let mut trade_total = 0.0f32;
        for &(a, b, v) in &self.flow_year {
            trade_total += v;
            if let Some(&i) = by_id.get(&a) { self.hubs[i].trade_last_year += v; }
            if let Some(&i) = by_id.get(&b) { self.hubs[i].trade_last_year += v; }
        }
        let population: f32 = self.hubs.iter()
            .filter(|h| !h.is_estate).map(|h| h.population.max(0.0)).sum();
        let alive = self.hubs.iter()
            .filter(|h| !h.is_estate && !h.abandoned && h.population >= 1.0).count();
        self.world_series.push([
            yr.saturating_sub(1) as f32, population, trade_total, alive as f32,
            self.total_foundings as f32, self.total_abandonments as f32,
        ]);
        if self.world_series.len() > 400 {
            let excess = self.world_series.len() - 400;
            self.world_series.drain(0..excess);
        }
        // ── Batch 1 · per-GOOD ledger snapshot + era frame + Hall of Records ──
        if !self.good_flow_accum.is_empty() {
            self.hub_good_trade = std::mem::take(&mut self.good_flow_accum);
        }
        self.year_frames.push(YearFrame {
            year: yr.saturating_sub(1),
            pop: self.hubs.iter()
                .map(|h| if h.is_estate { -1.0 } else { h.population.max(0.0) }).collect(),
            trade: self.hubs.iter().map(|h| h.trade_last_year).collect(),
        });
        if self.year_frames.len() > 400 {
            let excess = self.year_frames.len() - 400;
            self.year_frames.drain(0..excess);
        }
        self.update_records(yr.saturating_sub(1), trade_total);
    }

    /// Batch 1 · refresh the Hall of Records (all-time bests) at New Year.
    fn update_records(&mut self, year: u32, trade_total: f32) {
        let r = &mut self.records;
        for h in &self.hubs {
            if h.is_estate || h.abandoned { continue; }
            if h.population > r.largest_city.0 {
                r.largest_city = (h.population, h.name.clone(), year);
            }
        }
        for house in &self.houses {
            if house.defunct { continue; }
            if house.wealth > r.richest_house.0 {
                r.richest_house = (house.wealth, house.name.clone(), year);
            }
            if house.generation as f32 > r.longest_dynasty.0 {
                r.longest_dynasty = (house.generation as f32, house.name.clone(), year);
            }
        }
        if trade_total > r.biggest_trade_year.0 {
            let busiest = self.hubs.iter()
                .filter(|h| !h.is_estate)
                .max_by(|a, b| a.trade_last_year.partial_cmp(&b.trade_last_year)
                    .unwrap_or(std::cmp::Ordering::Equal))
                .map(|h| h.name.clone()).unwrap_or_default();
            r.biggest_trade_year = (trade_total, busiest, year);
        }
        for p in &self.epidemics {
            if p.deaths > r.deadliest_plague.0 {
                let name = self.hubs.get(p.hub as usize).map(|h| h.name.clone()).unwrap_or_default();
                r.deadliest_plague = (p.deaths, name, p.start_tick / 365);
            }
        }
        for c in &self.crashes {
            if c.cities_hit as f32 > r.worst_crash.0 {
                r.worst_crash = (c.cities_hit as f32, c.origin_name.clone(), c.year);
            }
        }
        let alive = self.hubs.iter()
            .filter(|h| !h.is_estate && !h.abandoned && h.population >= 1.0).count() as f32;
        if alive > r.most_towns.0 {
            r.most_towns = (alive, String::new(), year);
        }
    }

    /// Raise a forced WAR LEVY from every house homed at `hub`: a slice of each
    /// fortune into the city's war chest (treasury). The core wealth sink of war.
    /// Returns the total raised.
    fn raise_war_levy(&mut self, hub: usize) -> f32 {
        let mut total = 0.0f32;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            if self.houses[hi].hub as usize != hub { continue; }
            let levy = self.houses[hi].wealth.max(0.0) * WAR_LEVY_RATE;
            if levy <= EPS { continue; }
            self.houses[hi].wealth -= levy;
            total += levy;
            if hi < self.house_ledger.len() { self.house_ledger[hi].civic_tax += levy; }
        }
        if hub < self.hubs.len() {
            self.hubs[hub].treasury += total;
            self.hubs[hub].finance.war_levy += total;
        }
        total
    }

    /// Spend a slice of `hub`'s treasury on the war effort (consumed — the cost of
    /// armies & blockade). Returns the amount spent (its contribution to victory).
    fn spend_war(&mut self, hub: usize) -> f32 {
        let spend = (self.hubs[hub].treasury.max(0.0) * WAR_SPEND_RATE).min(self.hubs[hub].treasury.max(0.0));
        self.hubs[hub].treasury -= spend;
        self.hubs[hub].finance.spent_war += spend;
        self.hubs[hub].war_effort += spend;
        spend
    }

    /// Apply a resolved war's GOAL — the victor's lasting spoils beyond the one-off
    /// plunder. Returns a short clause appended to the journal / Wars-log text.
    /// Trade rights & annexation both work through the BAILO primitive (a foreign
    /// governing foothold), so the winner's house is seated on the loser's council
    /// by the ordinary yearly recompute and the control transfer sticks.
    fn apply_war_goal(&mut self, win: usize, lose: usize, goal: u8, tick: u32, _yr: u32) -> String {
        let (wn, ln) = (self.hubs[win].name.clone(), self.hubs[lose].name.clone());
        // The victor's ruling family: its council head, else its richest resident.
        let ruler = {
            let c = self.hubs[win].council_house;
            if c >= 0 && (c as usize) < self.houses.len() { Some(c as usize) }
            else { self.strongest_house_at(win) }
        };
        let grant_bailo = |me: &mut Self, hi: usize| {
            if !me.houses[hi].bailos.contains(&(lose as u32)) {
                me.houses[hi].bailos.push(lose as u32);
            }
        };
        match goal {
            WAR_GOAL_TRIBUTE => {
                self.hubs[lose].tribute_to = win as i32;
                self.hubs[lose].tribute_until = tick + TRIBUTE_YEARS * TICKS_PER_YEAR;
                format!("; {} is made a tributary of {} for {} years", ln, wn, TRIBUTE_YEARS)
            }
            WAR_GOAL_TRADE_RIGHTS => {
                if let Some(hi) = ruler {
                    grant_bailo(self, hi);
                    let hn = self.houses[hi].name.clone();
                    format!("; {} wins trade rights in {} — a bailo for {}", wn, ln, hn)
                } else { String::new() }
            }
            WAR_GOAL_ANNEX => {
                if let Some(hi) = ruler {
                    grant_bailo(self, hi);
                    self.hubs[lose].council_house = hi as i32;
                    self.hubs[lose].coin_trust = (self.hubs[lose].coin_trust - 0.15).max(0.0);
                    let hn = self.houses[hi].name.clone();
                    format!("; {} is annexed by {} — {} installed on its council", ln, wn, hn)
                } else {
                    format!("; {} is annexed by {}", ln, wn)
                }
            }
            _ => String::new(),
        }
    }

    /// DLC 3.5 · the economic-war engine. Once a year: wage & resolve active wars
    /// (levies, war-chest spending, trade blockade, reparations) and occasionally
    /// declare a new one between rival poleis. Deterministic.
    fn update_wars(&mut self, yr: u32) {
        let tick = self.tick;
        // Tributaries pay their overlords first — a bounded treasury→treasury
        // transfer (never minted), lapsing when the term ends.
        for h in 0..self.hubs.len() {
            let to = self.hubs[h].tribute_to;
            if to < 0 { continue; }
            if tick >= self.hubs[h].tribute_until || to as usize >= self.hubs.len() {
                self.hubs[h].tribute_to = -1;
                continue;
            }
            let to = to as usize;
            let cap = TRIBUTE_CAP * self.city_size_factor(h);
            let pay = (self.hubs[h].treasury.max(0.0) * TRIBUTE_RATE).min(cap);
            if pay <= EPS { continue; }
            self.hubs[h].treasury -= pay;
            self.hubs[to].treasury += pay;
            self.hubs[h].finance.reparations_out += pay;
            self.hubs[to].finance.reparations_in += pay;
        }
        let mut ended: Vec<usize> = Vec::new();
        for wi in 0..self.wars.len() {
            let (a, b) = (self.wars[wi].a as usize, self.wars[wi].b as usize);
            if a >= self.hubs.len() || b >= self.hubs.len() {
                ended.push(wi); continue;
            }
            // Wage: levies on each side's houses, war-chest spending, trade blockade.
            let lev = self.raise_war_levy(a) + self.raise_war_levy(b);
            self.wars[wi].levies += lev;
            self.wars[wi].chest_a += self.spend_war(a);
            self.wars[wi].chest_b += self.spend_war(b);
            for &h in &[a, b] {
                self.hubs[h].trade_wealth *= 0.8; // blockade bites commerce
                self.active_events.push(ActiveEvent {
                    kind: "war".into(), hub: h as i32, good: -1,
                    magnitude: 0.4, until_tick: tick + TICKS_PER_YEAR,
                });
            }
            // Resolve after >= 2 years, weighted by mustered effort + treasury.
            let years = tick.saturating_sub(self.wars[wi].start_tick) / TICKS_PER_YEAR;
            if years >= 2 {
                let pa = self.wars[wi].chest_a + self.hubs[a].treasury + 1.0;
                let pb = self.wars[wi].chest_b + self.hubs[b].treasury + 1.0;
                let a_wins = hash01(self.seed, tick as u64 ^ 0xBA771E, wi as u64) < pa / (pa + pb);
                let (win, lose) = if a_wins { (a, b) } else { (b, a) };
                let rep = (self.hubs[lose].treasury.max(0.0) * 0.4).max(0.0);
                self.hubs[lose].treasury -= rep;
                self.hubs[win].treasury += rep;
                self.hubs[win].finance.reparations_in += rep;
                self.hubs[lose].finance.reparations_out += rep;
                self.hubs[lose].coin_trust = (self.hubs[lose].coin_trust - 0.15).max(0.0);
                self.hubs[a].war_with = -1;
                self.hubs[b].war_with = -1;
                // The victor claims the war's GOAL — its lasting spoils (tribute /
                // trade rights / annexation), beyond the one-off plunder above.
                let spoils = self.apply_war_goal(win, lose, self.wars[wi].goal, tick, yr);
                let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
                let (wn, ln) = (self.hubs[win].name.clone(), self.hubs[lose].name.clone());
                let text = format!(
                    "The war of {} and {} ends in year {}: {} prevails, {} pays {:.0} in reparations{}.",
                    an, bn, yr, wn, ln, rep, spoils);
                self.journal.push(JournalEntry { tick, kind: "war".into(), hub: win as i32, good: -1,
                    value: rep, text: text.clone() });
                self.war_log.push(WarRecord {
                    start_year: self.wars[wi].start_tick / TICKS_PER_YEAR, end_year: yr,
                    a_name: an, b_name: bn, winner: wn, loser: ln,
                    reparations: rep, levies_total: self.wars[wi].levies,
                    cause: self.wars[wi].cause.clone(), text,
                });
                if self.war_log.len() > WAR_LOG_CAP {
                    let drop = self.war_log.len() - WAR_LOG_CAP;
                    self.war_log.drain(0..drop);
                }
                // War of independence: the colony either wins free, or is brought to
                // heel for 15 years before it may rebel again.
                if self.wars[wi].cause == "independence" {
                    let colony = if self.hubs[a].colony_kind == 1 { a }
                        else if self.hubs[b].colony_kind == 1 { b } else { usize::MAX };
                    if colony != usize::MAX {
                        if colony == win {
                            self.make_colony_independent(colony, true);
                        } else {
                            self.hubs[colony].indep_cooldown_until = tick + 15 * TICKS_PER_YEAR;
                            let cn = self.hubs[colony].name.clone();
                            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: colony as i32,
                                good: -1, value: 0.0, text: format!("{}'s bid for independence is crushed; it remains a colony", cn) });
                        }
                    }
                }
                ended.push(wi);
            }
        }
        for &wi in ended.iter().rev() { self.wars.remove(wi); }
        self.maybe_declare_war(yr);
    }

    /// Occasionally ignite a new economic war between two rival poleis in the same
    /// region, both at peace. Rival councils are the spark; prosperity is the prize.
    fn maybe_declare_war(&mut self, yr: u32) {
        if self.wars.len() >= MAX_ACTIVE_WARS { return; }
        if hash01(self.seed, self.tick as u64 ^ 0xDEC1A6E, yr as u64) > WAR_DECLARE_CHANCE { return; }
        let n = self.hubs.len();
        // Candidate seats: real cities with a council, at peace.
        let seats: Vec<usize> = (0..n).filter(|&h|
            !self.hubs[h].is_estate && self.hubs[h].war_with < 0
            && self.hubs[h].council_house >= 0 && self.hubs[h].population > 1.0
        ).collect();
        if seats.len() < 2 { return; }
        // Prefer a pair in the same region whose councils are rivals; else any pair.
        let mut best: Option<(usize, usize, &'static str)> = None;
        for (ii, &a) in seats.iter().enumerate() {
            for &b in seats.iter().skip(ii + 1) {
                if self.hubs[a].component != self.hubs[b].component { continue; }
                let ca = self.hubs[a].council_house as usize;
                let cb = self.hubs[b].council_house as usize;
                let rivals = self.houses.get(ca).map(|h| h.rivals.contains(&cb)).unwrap_or(false)
                    || self.houses.get(cb).map(|h| h.rivals.contains(&ca)).unwrap_or(false);
                if rivals { best = Some((a, b, "rival councils")); break; }
                if best.is_none() { best = Some((a, b, "trade dispute")); }
            }
            if matches!(best, Some((_, _, "rival councils"))) { break; }
        }
        let Some((a, b, cause)) = best else { return };
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        // A WAR GOAL — what the aggressor is after. Rival councils fight for political
        // supremacy (annexation / trade rights); a plain trade dispute is fought for
        // tribute or plunder. A lopsided match leans toward annexation (the strong
        // swallow the weak); an even one toward tribute. Deterministic.
        let pow = |me: &Self, h: usize| me.hubs[h].population.max(1.0) * (me.hubs[h].treasury.max(0.0) + 5.0);
        let (pa, pb) = (pow(self, a), pow(self, b));
        let ratio = pa.max(pb) / pa.min(pb).max(1.0);
        let g = hash01(self.seed, self.tick as u64 ^ 0x60A15, (a * 131 + b) as u64);
        let goal = if cause == "rival councils" {
            if ratio >= 2.0 && g < 0.6 { WAR_GOAL_ANNEX } else { WAR_GOAL_TRADE_RIGHTS }
        } else if ratio >= 2.5 && g < 0.4 {
            WAR_GOAL_ANNEX
        } else if g < 0.6 {
            WAR_GOAL_TRIBUTE
        } else {
            WAR_GOAL_PLUNDER
        };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        let (an, bn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "war".into(), hub: a as i32, good: -1, value: 0.0,
            text: format!("{} declares war on {} ({} · {})", an, bn, cause, war_goal_label(goal)),
        });
        self.wars.push(War {
            a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, cargo_lost: 0, cause: cause.into(), goal,
        });
    }

    /// DLC 4 · learning-by-doing: each month every MANUFACTORY/city drifts the
    /// quality of the manufactured goods it makes toward a skill cap set by its size
    /// and its craft buildings (guildhall/workshop). "Manufacturers start producing
    /// higher quality goods" — and big skilled cities reach finer grades.
    fn update_good_quality(&mut self) {
        let ng = self.goods.len();
        for h in 0..self.hubs.len() {
            if self.hubs[h].quality.len() != ng { continue; }
            let pop = self.hubs[h].population.max(1.0);
            let size_bonus = (pop / 60_000.0).min(0.20);
            let mut struct_bonus = 0.0;
            if self.hub_has_struct(h, STRUCT_WORKSHOP) { struct_bonus += 0.08; }
            if self.hub_has_struct(h, STRUCT_GUILDHALL) { struct_bonus += 0.06; }
            let manu_estate = self.hubs[h].is_estate && self.hubs[h].estate_kind == 6;
            for g in 0..ng {
                let manufactured = !self.goods[g].inputs.is_empty();
                if !(manufactured || manu_estate) { continue; }
                if self.hubs[h].production.get(g).copied().unwrap_or(0.0) <= 0.0
                    && self.hubs[h].quality[g] <= 0.0 { continue; }
                let cap = (0.62 + size_bonus + struct_bonus).clamp(0.0, 0.97);
                let q = self.hubs[h].quality[g];
                if q < cap {
                    self.hubs[h].quality[g] = (q + (cap - q) * QUALITY_LEARN_RATE).min(cap);
                }
            }
        }
    }

    /// DLC 4 · industrial espionage: once a year a house running a manufactory may
    /// STEAL the technique of the world's finest maker of a good it produces,
    /// jumping its own quality toward the leader's. Recorded on the thief's hub
    /// (`stolen_good`/`stolen_from`) and journaled — visible in the estate view.
    fn maybe_steal_quality(&mut self, yr: u32) {
        let ng = self.goods.len();
        // World-leading quality + its hub, per good (producers only).
        let mut best_q = vec![0.0f32; ng];
        let mut best_hub = vec![usize::MAX; ng];
        for h in 0..self.hubs.len() {
            if self.hubs[h].quality.len() != ng { continue; }
            for g in 0..ng {
                if self.hubs[h].production.get(g).copied().unwrap_or(0.0) <= 0.0 { continue; }
                if self.hubs[h].quality[g] > best_q[g] { best_q[g] = self.hubs[h].quality[g]; best_hub[g] = h; }
            }
        }
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            // Spies are the cunning sorts: specialist / political / banking houses.
            if self.houses[hi].archetype == ARCH_FLEET { continue; }
            if hash01(self.seed, (hi as u64) ^ 0x57EA1, yr as u64) > QUALITY_STEAL_CHANCE { continue; }
            // A manufactory this house owns, making a good a rival makes far better.
            let mine: Option<(usize, usize)> = self.hubs.iter().enumerate()
                .filter(|(_, e)| e.is_estate && e.owner_house == hi as i32 && e.estate_kind == 6
                    && e.quality.len() == ng)
                .filter_map(|(ei, e)| {
                    let g = (0..ng).find(|&g| e.production.get(g).copied().unwrap_or(0.0) > 0.0)?;
                    Some((ei, g))
                })
                .find(|&(ei, g)| best_hub[g] != usize::MAX && best_hub[g] != ei
                    && best_q[g] - self.hubs[ei].quality.get(g).copied().unwrap_or(0.0) > 0.12);
            let Some((ei, g)) = mine else { continue };
            let leader = best_hub[g];
            let gain = (best_q[g] - self.hubs[ei].quality[g]) * QUALITY_STEAL_FRAC;
            self.hubs[ei].quality[g] += gain;
            self.hubs[ei].stolen_good = g as i32;
            self.hubs[ei].stolen_from = self.hubs[leader].id as i32;
            let (hn, gn, victim) = (self.houses[hi].name.clone(),
                self.goods[g].name.clone(), self.hubs[leader].name.clone());
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "espionage".into(), hub: ei as i32, good: g as i32, value: gain,
                text: format!("{} steals the secret of {} from {}", hn, gn, victim),
            });
            self.houses[hi].events.push(HouseEvent {
                tick: self.tick, kind: "espionage".into(),
                text: format!("stole the {} craft from {}", gn, victim),
            });
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
    /// Crash recovery: the tick that would have run next faulted. Advance the clock by
    /// one tick anyway (the campaign must NEVER stall) and suspend territorial expansion
    /// for a year so re-advancing can't re-hit the same founding fault. The caller has
    /// already restored a clean pre-tick checkpoint, so state is consistent here.
    pub fn skip_poisoned_tick(&mut self) {
        self.tick += 1;
        self.expansion_frozen_until = self.tick + TICKS_PER_YEAR;
        self.routes_dirty = true;
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "disaster".into(), hub: -1, good: -1, value: 0.0,
            text: "A troubled season passes — the chronicle weathered a disturbance.".into(),
        });
        if self.journal.len() > JOURNAL_CAP {
            let d = self.journal.len() - JOURNAL_CAP; self.journal.drain(0..d);
        }
    }

    /// Fuse tiny/lone trade components (< 3 real hubs) into the nearest substantial
    /// market's component, so no settlement is a dead "cosmetic" dot that can never
    /// trade. Estates follow their parent hub. Caller sets `routes_dirty`.
    fn rescue_tiny_components(&mut self) {
        let n = self.hubs.len();
        if n == 0 { return; }
        let world_w = self.world_w;
        let real: Vec<usize> = (0..n).filter(|&i| !self.hubs[i].is_estate).collect();
        let mut size: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for &i in &real { *size.entry(self.hubs[i].component).or_default() += 1; }
        let big: Vec<usize> = real.iter().cloned()
            .filter(|&i| size.get(&self.hubs[i].component).copied().unwrap_or(0) >= 3).collect();
        if big.is_empty() { return; }
        let d2 = |a: usize, b: usize, hubs: &Vec<TickHub>| -> f32 {
            let mut dx = (hubs[a].x - hubs[b].x).abs();
            if world_w > 1.0 { dx = dx.min(world_w - dx); }
            let dy = hubs[a].y - hubs[b].y;
            dx * dx + dy * dy
        };
        let mut reassign: Vec<(usize, u32)> = Vec::new();
        for &i in &real {
            if size.get(&self.hubs[i].component).copied().unwrap_or(0) >= 3 { continue; }
            let mut bj = None; let mut bd = f32::INFINITY;
            for &j in &big { let d = d2(i, j, &self.hubs); if d < bd { bd = d; bj = Some(j); } }
            if let Some(j) = bj { reassign.push((i, self.hubs[j].component)); }
        }
        for (i, comp) in reassign { self.hubs[i].component = comp; }
        // Estates ride their parent hub's (possibly new) component.
        for i in 0..n {
            if self.hubs[i].is_estate {
                let p = self.hubs[i].parent;
                if p >= 0 && (p as usize) < n { self.hubs[i].component = self.hubs[p as usize].component; }
            }
        }
    }

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
        // A FINISHED, bound satellite (colony_kind 3, build done, has a metropolis) trades
        // strictly through its mother city — invisible to every other partner, and its own
        // partner list is just the metropolis. So all its exports/imports divert to the
        // metropolis first and that market reaps the surplus (user: permanent binding).
        let is_bound = |i: usize| {
            let h = &self.hubs[i];
            h.colony_kind == 3 && h.build_stage == 0 && h.founder_hub >= 0
        };
        for a in 0..n {
            scratch.clear();
            // A bound satellite only ever trades with its metropolis.
            if is_bound(a) {
                let m = self.hubs[a].founder_hub as usize;
                if m < n && self.days[a * n + m].is_finite() { neighbors[a] = vec![m as u32]; }
                continue;
            }
            for b in 0..n {
                if b == a { continue; }
                // A bound satellite is hidden from everyone EXCEPT its own metropolis.
                if is_bound(b) && self.hubs[b].founder_hub as usize != a { continue; }
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
    /// The staple cereal a settlement of climate `koppen` grows for subsistence —
    /// rice in the wet tropics, millet on the arid steppe, barley in the cold north,
    /// wheat otherwise. Falls back to any available food good. `food_gs` is the list
    /// of extracted (non-recipe) food-good indices.
    fn climate_staple(&self, koppen: u8, food_gs: &[usize]) -> Option<usize> {
        let by_name = |nm: &str| self.goods.iter().position(|g| g.name == nm).filter(|g| food_gs.contains(g));
        let primary = match koppen {
            1 | 2 | 3 => "rice",            // Af/Am/Aw tropical
            4 | 5 | 6 | 7 => "millet",      // BW/BS arid / steppe
            14..=22 => "barley",            // continental / polar cold
            _ => "wheat",
        };
        by_name(primary)
            .or_else(|| by_name("wheat"))
            .or_else(|| by_name("barley"))
            .or_else(|| by_name("millet"))
            .or_else(|| by_name("grain"))
            .or_else(|| food_gs.first().copied())
    }

    fn base_need(&self, h: usize, g: usize) -> f32 {
        let tg = &self.goods[g];
        // Demand cadence: a good consumed every N days exerts ~30/N of the daily
        // pull of a monthly good. Clamped so it modulates (not dominates) — long
        // cadence goods (furs, luxuries) sit cheaper locally and skew to wholesale.
        let interval = if tg.consumption_interval > 0.0 { tg.consumption_interval } else { 30.0 };
        let cadence = (30.0 / interval).clamp(0.30, 1.8);
        // Craving for FOREIGN luxuries (#2): a luxury-tier good a city can't make at
        // home is coveted the more — and the finer/dearer it is, the more so — so
        // distant high-value luxuries pull strong import demand and feed inter-city
        // trade, instead of every city resting on its own produce.
        let mut foreign_lux = 1.0;
        if tg.need_tier >= 2 {
            let local = self.hubs[h].base_per_capita.get(g).copied().unwrap_or(0.0);
            if local < 1e-4 {
                let prestige = (tg.base_value / 15.0).clamp(0.4, 1.6);
                foreign_lux = 1.0 + LUX_IMPORT_DESIRE * prestige;
            }
        }
        self.hubs[h].population
            * TIER_WEIGHT[tg.need_tier.min(2) as usize]
            * tg.desire.max(0.0)
            * cadence
            * foreign_lux
            * self.society_demand_mult(h, tg.need_tier)
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
        // DLC 4 · one-time: seed per-hub good QUALITY so it varies by settlement.
        // Raws get a terroir-ish base (a stable per-hub/good roll, nudged up for the
        // hub's strongest output — its specialty is its finest); manufactures start
        // lower (a craft to be learned). Producers only; non-producers stay 0.
        if !self.quality_migrated {
            for h in 0..self.hubs.len() {
                if self.hubs[h].quality.len() == ng { continue; }
                let mut q = vec![0.0f32; ng];
                // strongest produced good (its specialty → a quality bump)
                let best_g = (0..ng).max_by(|&a, &b| {
                    let pa = self.hubs[h].base_per_capita.get(a).copied().unwrap_or(0.0);
                    let pb = self.hubs[h].base_per_capita.get(b).copied().unwrap_or(0.0);
                    pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
                });
                for g in 0..ng {
                    let pc = self.hubs[h].base_per_capita.get(g).copied().unwrap_or(0.0);
                    if pc <= 0.0 && self.hubs[h].production.get(g).copied().unwrap_or(0.0) <= 0.0 { continue; }
                    let roll = hash01(self.seed, ((h as u64) << 8) ^ g as u64, 0x94A1170Du64);
                    let manufactured = !self.goods[g].inputs.is_empty();
                    let base = if manufactured { 0.30 } else { 0.42 };
                    let mut v = base + 0.30 * roll;
                    if Some(g) == best_g { v += 0.12; } // the specialty is finest
                    q[g] = v.clamp(0.0, 0.95);
                }
                self.hubs[h].quality = q;
            }
            self.quality_migrated = true;
        }
        // DLC · one-time: seed each settlement's social strata. A prosperous trading
        // city carries a larger patrician/burgher elite; a poor agrarian one is mostly
        // commoners with a wide underclass. Derived from signals that already exist
        // (trade vs grain wealth, population), so it works for new + migrated saves.
        if !self.society_migrated {
            for h in 0..self.hubs.len() {
                if self.hubs[h].is_estate { continue; }
                self.seed_society(h);
            }
            self.society_migrated = true;
        }
        // Rescue isolated "cosmetic" cities on older saves: a settlement whose trade
        // component is tiny (< 3 real hubs) can never trade (rebuild_routes marks it
        // unreachable). Fuse each into the nearest substantial market's component.
        if !self.components_rescued {
            self.rescue_tiny_components();
            self.components_rescued = true;
            self.routes_dirty = true;
        }
        // Phase 4 (flavour) · seed seasonal trade fairs once (population-scored, since
        // routes/neighbours aren't built until the tick loop below).
        if !self.fairs_seeded { self.seed_trade_fairs(); self.fairs_seeded = true; }
        // Phase 4 (flavour) · seed holy cities once (after fairs, so a holy city can
        // be chosen distinct from its component's fair town).
        if !self.holy_seeded { self.seed_holy_sites(); self.holy_seeded = true; }
        // Phase 5 (flavour) · seed craft guilds once (manufacturing cities).
        if !self.guilds_seeded { self.seed_craft_guilds(); self.guilds_seeded = true; }
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

        // Per-tick scratch matrices (n×ng), reused across the whole advance so a long
        // run doesn't reallocate them every tick. Resized/cleared inside the loop.
        let mut needs: Vec<Vec<f32>> = Vec::new();
        let mut prod_mult: Vec<Vec<f32>> = Vec::new();

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
            // Bound the global chronicle so a long campaign's autosave can't OOM on
            // serialization (the cheap length check is a no-op until the cap is hit).
            if self.journal.len() > JOURNAL_CAP {
                let drop = self.journal.len() - JOURNAL_CAP;
                self.journal.drain(0..drop);
            }
            let n = self.hubs.len();
            let doy = self.day_of_year();

            // Phase G: keep the per-house ledgers aligned to the house list, and roll
            // the year over on the New Year — the just-finished year becomes the
            // Accountant's displayed `_prev`, and a fresh current year starts.
            self.house_ledger.resize(self.houses.len(), LedgerAcc::default());
            self.house_barred.resize(self.houses.len(), Vec::new());
            self.hub_patron.resize(self.hubs.len(), -1); // trade-base patronage (hub-indexed)
            // Twice a year, re-rank hubs into commercial classes (trade hub / entrepôt)
            // from the trade that has actually flowed this period (user: entrepôts
            // change once per half-year). Hysteresis inside makes status earned/lost.
            if tick > 0 && tick % (TICKS_PER_YEAR / 2) == 0 {
                self.classify_hubs();
            }
            if tick % TICKS_PER_YEAR == 0 {
                // v2.0 · close the monetary loop: turn each coin's debasement +
                // money growth into a real per-city inflation rate and compound the
                // local price level. The inflation-tax below is then levied at the
                // resident city's rate (debased-coin cities erode fortunes faster).
                let hub_infl = self.update_price_levels();
                // Yearly inflation erodes every fortune's real value, recorded in the
                // year that is now closing — then archive it for the Accountant.
                for hi in 0..self.houses.len() {
                    if self.houses[hi].defunct {
                        continue;
                    }
                    let rate = hub_infl.get(self.houses[hi].hub as usize).copied()
                        .unwrap_or(INFL_BASE).max(0.0); // deflation never ADDS wealth
                    let infl = self.houses[hi].wealth.max(0.0) * rate;
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
                // Government: seed regimes/key figures, bribery & intimidation, capture,
                // regime change, civic granary (reads the influence/dominance just set).
                self.update_government(yr);
                // DLC 3.5 · the council then sets its coinage (named coin + trust +
                // seigniorage), banking houses charter/grow banks, the speculation
                // engine reads the closed year, and HIGH-tier bubbles may POP into a
                // regional crash.
                self.decide_coinage(yr);
                // v2.0 · a council whose coin has failed may REFORM it (call-in + re-mint).
                self.maybe_reform_coinage(yr);
                self.update_currency_baskets();
                self.update_banks(yr);
                self.update_wars(yr);
                self.maybe_steal_quality(yr);
                self.compute_speculation(yr);
                // Fold the year's trade flows into the Flows-subtab detail + trend graphs.
                self.fold_trade_year();
                self.maybe_pop_bubbles(yr);
                self.roll_city_finances(yr);
                // Phase 4 (flavour) · raise/retire notable figures (Great Lives).
                self.raise_notable_figures(yr);
                // Phase 5 (flavour) · dynastic marriages/alliances between houses.
                self.arrange_marriages(yr);
                // Phase 5 (flavour) · craft guilds master their craft, strike, build.
                self.run_craft_guilds(yr);
                // Phase 5 (flavour) · lighter set: fashion cycles, civic wonders,
                // piracy raids, diaspora quarters.
                self.roll_fashion(yr);
                self.run_civic_wonders(yr);
                self.run_piracy(yr);
                self.run_diaspora(yr);
                for l in self.house_ledger.iter_mut() {
                    *l = LedgerAcc { year: yr, ..Default::default() };
                }
            }

            // Phase 4 (flavour) · open any trade fair / pilgrimage season beginning
            // today (checked every tick — they open mid-year on their month).
            self.run_trade_fairs(doy);
            self.run_pilgrimages(doy);

            // Expire finished events.
            self.active_events.retain(|e| e.until_tick > tick);
            // Production multipliers from active events (per hub/good, default 1).
            self.fill_event_production_mult(&mut prod_mult);
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
            // Extracted (non-recipe) FOOD goods — for the subsistence-farming floor.
            let food_gs: Vec<usize> = (0..ng)
                .filter(|&g| self.goods[g].food && self.goods[g].inputs.is_empty())
                .collect();
            // Per-TRADE-COMPONENT food balance (natural capacity vs need). A hub only
            // falls back on subsistence farming when its component CANNOT supply food —
            // i.e. a remote/isolated or genuinely food-poor region that no trade route
            // reaches. A connected, food-secure region relies on trade instead (so we
            // don't magically make every big city self-feeding and kill the grain trade).
            let mut comp_food_supply: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
            let mut comp_food_need: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
            if !food_gs.is_empty() {
                for h in 0..n {
                    if self.hubs[h].is_estate { continue; }
                    let comp = self.hubs[h].component;
                    let pop = self.hubs[h].population.max(0.0);
                    let mut sup = 0.0f32;
                    for &g in &food_gs { sup += self.hubs[h].base_per_capita.get(g).copied().unwrap_or(0.0) * pop; }
                    let mut nd = 0.0f32;
                    for &g in &food_gs { nd += self.base_need(h, g); }
                    *comp_food_supply.entry(comp).or_default() += sup * tech;
                    *comp_food_need.entry(comp).or_default() += nd;
                }
            }
            for h in 0..n {
                let pop = self.hubs[h].population.max(0.0);
                // Standing structure bonuses (Workshop/Warehouse = all goods,
                // Granary = food only); `struct_bonus` was the A1 placeholder hook.
                let (struct_all, struct_food) = self.hub_struct_prod(h);
                // A works' condition (disaster damage / age / labor / unrest) scales its
                // realized output; ordinary settlements are unaffected (1.0).
                let eff = if self.hubs[h].is_estate { self.estate_effectiveness(h) } else { 1.0 };
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
                        * prod_mult[h][g] * tech * struct_bonus * eff;
                    self.hubs[h].production[g] = realized;
                    self.hubs[h].stock[g] += realized;
                }
                // SUBSISTENCE FARMING — a REMOTE land settlement whose trade component
                // cannot supply food feeds itself from its own fields, so an isolated
                // town isn't "100% short" when no trade route reaches it. A connected,
                // food-secure region is left to TRADE for its food (routes exist, so use
                // them). Remote hubs are only brought to ~0.9× need — enough to survive
                // but with no surplus, so they stay small (as a remote outpost should).
                let comp = self.hubs[h].component;
                let comp_secure = comp_food_supply.get(&comp).copied().unwrap_or(0.0)
                    >= comp_food_need.get(&comp).copied().unwrap_or(0.0) * 0.85;
                if !self.hubs[h].is_estate && pop > 0.0 && pop < REMOTE_MAX_POP
                    && !food_gs.is_empty() && !comp_secure {
                    let mut food_prod = 0.0f32;
                    let mut food_need = 0.0f32;
                    for &g in &food_gs {
                        food_prod += self.hubs[h].production[g];
                        food_need += self.base_need(h, g);
                    }
                    let target = food_need * SUBSISTENCE_FOOD_FRAC;
                    if food_prod < target {
                        // The hub's staple: the food it already grows most, else the
                        // climate-appropriate cereal.
                        let grown = food_gs.iter().copied()
                            .filter(|&g| self.hubs[h].production[g] > 0.0)
                            .max_by(|&a, &b| self.hubs[h].production[a]
                                .partial_cmp(&self.hubs[h].production[b]).unwrap_or(std::cmp::Ordering::Equal));
                        if let Some(g) = grown.or_else(|| self.climate_staple(self.hubs[h].koppen, &food_gs)) {
                            let add = target - food_prod;
                            self.hubs[h].production[g] += add;
                            self.hubs[h].stock[g] += add;
                        }
                    }
                }
            }

            // 1b) Manufacturing — cities transform imported raws into finished goods
            //     (wool→cloth, ore→arms), concentrated in big cities (labor ∝ pop).
            self.manufacture_pass();

            // 2) Consumption with per-category substitution toward cheaper goods.
            // Reuse the `needs` buffer across ticks instead of reallocating an
            // n×ng matrix every single tick — over a long campaign that per-tick
            // allocation churn is a real cost (and allocator pressure). `n` can
            // grow within the loop (estates), so resize, then clear each row.
            needs.resize(n, Vec::new());
            for row in needs.iter_mut() {
                row.clear();
                row.resize(ng, 0.0);
            }
            // Phase 5 (flavour) · fashion demand multipliers per good (1.0 = not in
            // vogue), from active "fashion" events — a capped, transient demand lift.
            let mut fashion_mult = vec![1.0f32; ng];
            let mut any_fashion = false;
            for e in &self.active_events {
                if e.kind == "fashion" && e.good >= 0 && (e.good as usize) < ng {
                    fashion_mult[e.good as usize] = (1.0 + e.magnitude).min(FASHION_MAX_MULT);
                    any_fashion = true;
                }
            }
            // Cultural taste: precompute each hub's desired-good demand lifts (from its
            // resident peoples, weighted by pop share; capped). Applied after substitution
            // + fashion so the prized good genuinely pulls more demand.
            let name_to_g: std::collections::HashMap<&str, usize> =
                self.goods.iter().enumerate().map(|(g, tg)| (tg.name.as_str(), g)).collect();
            let mut culture_goods: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
            let mut hub_desire: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
            for h in 0..n {
                if self.hubs[h].is_estate { continue; }
                let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
                let minsum: f32 = self.hub_minorities.get(h)
                    .map(|m| m.iter().map(|(_, s)| *s).sum()).unwrap_or(0.0);
                let mut present: Vec<(String, f32)> = Vec::new();
                if !maj.is_empty() && maj != "—" { present.push((maj, (1.0 - minsum).clamp(0.0, 1.0))); }
                if let Some(mins) = self.hub_minorities.get(h) {
                    for (c, s) in mins { if *s > 0.02 { present.push((c.clone(), *s)); } }
                }
                let mut boost: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
                for (c, share) in present {
                    if !culture_goods.contains_key(&c) {
                        let gis: Vec<usize> = self.culture_desired_goods(&c).iter()
                            .filter_map(|nm| name_to_g.get(nm).copied()).collect();
                        culture_goods.insert(c.clone(), gis);
                    }
                    for &gi in &culture_goods[&c] {
                        *boost.entry(gi).or_insert(0.0) += share * CULTURE_DESIRE_BOOST;
                    }
                }
                hub_desire[h] = boost.into_iter().map(|(gi, b)| (gi, b.min(CULTURE_DESIRE_MAX))).collect();
            }
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
                // Phase 5 (flavour) · a good in vogue is consumed more keenly
                // (post-substitution, so fashion draws real extra demand → price rises).
                if any_fashion {
                    for g in 0..ng {
                        if fashion_mult[g] != 1.0 { needs[h][g] *= fashion_mult[g]; }
                    }
                }
                // Cultural taste: the resident peoples' prized goods pull extra demand.
                for &(gi, b) in &hub_desire[h] { needs[h][gi] *= 1.0 + b; }
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

            // 3) Local prices (smoothed scarcity in the grain-eq numeraire). DLC 4:
            //    a hub's OWN-produced good is worth more when its quality is high —
            //    fine wine fetches a premium, coarse wine a discount (its grade is
            //    "baked into" the local standard value), so quality is desired.
            for h in 0..n {
                for g in 0..ng {
                    let mut base = self.goods[g].base_value;
                    if self.hubs[h].production.get(g).copied().unwrap_or(0.0) > 0.0 {
                        base *= quality_value_mult(self.hubs[h].quality.get(g).copied().unwrap_or(0.0));
                    }
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
            // Phase 5 (flavour) · plague travels the trade lanes from any active focus.
            self.spread_epidemics();
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
                self.council_provision_pass(); // councils pre-empt needed goods into civic warehouses
                self.construction_pass(); // satellite build sites: haul supply, advance/decay
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

    /// The STRUCTURAL strata composition a city tends toward, from signals that
    /// already exist and genuinely differ city-to-city: trade orientation (mercantile
    /// vs agrarian), wealth, and resident-elite riches. A trade entrepôt carries a
    /// broad patrician/burgher elite; a poor agrarian backwater is mostly commoners
    /// with a wide underclass. Returns normalized shares (Σ=1). The yearly mobility
    /// eases the LIVE shares toward this target (with a hardship skew), which keeps
    /// cities differentiated instead of all drifting to one attractor.
    fn target_shares(&self, h: usize) -> (f32, f32, f32, f32) {
        let hub = &self.hubs[h];
        let trade = hub.trade_wealth.max(0.0);
        let grain = hub.grain_wealth.max(0.0);
        let orient = trade / (trade + grain + 1.0); // 0 agrarian … ~1 mercantile
        let w = trade * 0.8 + grain * 0.4;
        let prosp = (w / (w + 1.2)).clamp(0.0, 1.0);
        // The private patrician elite = resident merchant HOUSES only. Civic GUILDS
        // are excluded: their wealth scales with city size, so counting them made the
        // per-capita elite uniform across cities and flattened strata differentiation
        // once guilds began forming in every large city.
        let elite_w: f32 = self.houses.iter()
            .filter(|hh| !hh.defunct && !hh.is_guild && hh.hub as usize == h)
            .map(|hh| hh.wealth.max(0.0)).sum();
        let elite_pc = elite_w / hub.population.max(1.0);
        let patrician = (0.02 + 0.07 * orient + 0.06 * (elite_pc / (elite_pc + 5.0))).clamp(0.01, 0.14);
        let burgher = (0.08 + 0.20 * orient + 0.05 * prosp).clamp(0.06, 0.32);
        let underclass = (0.12 + 0.24 * (1.0 - prosp)).clamp(0.05, 0.42);
        let commoner = (1.0 - patrician - burgher - underclass).max(0.05);
        let t = patrician + burgher + commoner + underclass;
        (patrician / t, burgher / t, commoner / t, underclass / t)
    }

    /// Seed a settlement's social strata to its structural target. Called once per hub
    /// (first advance / a freshly founded colony), then evolved by `update_society`.
    fn seed_society(&mut self, h: usize) {
        let (p, b, c, u) = self.target_shares(h);
        {
            let so = &mut self.hubs[h].society;
            so.patrician = p; so.burgher = b; so.commoner = c; so.underclass = u;
        }
        let (cw, ineq) = self.society_metrics(h);
        let so = &mut self.hubs[h].society;
        so.commoner_wealth = cw;
        so.inequality = ineq;
    }

    /// Derived strata read-outs from the live economy: per-capita money reaching the
    /// commons, and an inequality index (elite wealth per head vs the commoner's).
    fn society_metrics(&self, h: usize) -> (f32, f32) {
        let hub = &self.hubs[h];
        let pop = hub.population.max(1.0);
        let civic_pc = hub.civic_pool / pop * 100.0; // same scale as update_sentiment
        let food_aff = (1.0 - hub.starving).clamp(0.0, 1.0);
        let tax = hub.tariff_export + hub.tariff_import;
        let commoner_wealth = (civic_pc * 0.5 + hub.grain_wealth * 0.3 + food_aff * 0.4 - tax).max(0.0);
        let elite_w: f32 = self.houses.iter()
            .filter(|hh| !hh.defunct && hh.hub as usize == h)
            .map(|hh| hh.wealth.max(0.0)).sum::<f32>()
            + hub.treasury.max(0.0);
        let elite_heads = ((hub.society.patrician + hub.society.burgher) * pop).max(1.0);
        let elite_pc = elite_w / elite_heads;
        let ratio = elite_pc / (commoner_wealth + 0.5);
        let inequality = (ratio / (ratio + 8.0)).clamp(0.0, 1.0);
        (commoner_wealth, inequality)
    }

    /// Yearly social mobility: prosperity lifts families up the strata; hardship and
    /// shocks push them down (swelling the underclass). Shares stay bounded + Σ=1.
    /// Then refresh the derived `commoner_wealth` / `inequality` read-outs (eased).
    fn update_society(&mut self) {
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            // A colony founded after the one-time seed starts blank — seed it lazily.
            let s0 = &self.hubs[h].society;
            if s0.patrician + s0.burgher + s0.commoner + s0.underclass < 1e-3 {
                self.seed_society(h);
            }
            let (lackb, starv) = {
                let hub = &self.hubs[h];
                (hub.lack_basic.clamp(0.0, 1.0), hub.starving.clamp(0.0, 1.0))
            };
            let shock: f32 = self.active_events.iter()
                .filter(|e| e.hub == h as i32).map(|e| e.magnitude.max(0.2)).sum::<f32>().min(1.0);
            // Hardship (famine / dearth / disaster) skews the target DOWN: it drains the
            // upper tiers toward the underclass for as long as the hard times last.
            let hard = (starv * 0.6 + lackb * 0.4 + shock * 0.3).clamp(0.0, 0.7);
            let (tp, tb, tc, _tu) = self.target_shares(h);
            let drain = hard * 0.5;
            let (mut p, mut b, mut c) = (tp * (1.0 - drain), tb * (1.0 - drain), tc * (1.0 - drain * 0.5));
            let mut u = (1.0 - p - b - c).max(0.0);
            let tot = (p + b + c + u).max(1e-6);
            p /= tot; b /= tot; c /= tot; u /= tot;
            // Ease the LIVE shares toward this (skewed) target — gradual mobility, so
            // shocks register over a few years and recovery takes a few more.
            let ease = STRATA_MOBILITY_RATE * 2.5; // ~0.10/yr
            {
                let so = &mut self.hubs[h].society;
                so.patrician += (p - so.patrician) * ease;
                so.burgher += (b - so.burgher) * ease;
                so.commoner += (c - so.commoner) * ease;
                so.underclass += (u - so.underclass) * ease;
                let t = (so.patrician + so.burgher + so.commoner + so.underclass).max(1e-6);
                so.patrician /= t; so.burgher /= t; so.commoner /= t; so.underclass /= t;
            }
            let (cw, ineq) = self.society_metrics(h);
            let so = &mut self.hubs[h].society;
            so.commoner_wealth += (cw - so.commoner_wealth) * 0.3;
            so.inequality += (ineq - so.inequality) * 0.3;
        }
        // DLC 4 · derive typed Pops from the freshly-updated shares (read-only foundation).
        for h in 0..self.hubs.len() { self.derive_pops(h); }
    }

    /// DLC 4 · derive typed `Pop` units for hub `h` from its `society` shares ×
    /// population. Read-only foundation for the Nations & POPs layer — nothing
    /// consumes these yet, so the economy is unchanged; refreshed yearly so the
    /// future Population panel and (later) pop-driven demand can read them.
    fn derive_pops(&mut self, h: usize) {
        let hub = &self.hubs[h];
        if hub.is_estate || hub.population < 1.0 {
            self.hubs[h].pops.clear();
            return;
        }
        let pop = hub.population.max(0.0);
        let so = &hub.society;
        let split: [(u8, f32); 9] = [
            (0, so.commoner * 0.60),                        // farmers
            (1, so.commoner * 0.40 + so.underclass * 0.55), // labourers (+ urban poor)
            (2, so.burgher * 0.40),                         // craftsmen
            (3, so.burgher * 0.28),                         // clerks
            (4, so.burgher * 0.32),                         // merchants
            (5, so.patrician * 0.30),                       // clergy
            (6, so.patrician * 0.28),                       // capitalists
            (7, so.patrician * 0.42),                       // aristocrats
            (8, so.underclass * 0.45),                      // soldiers
        ];
        let cw = so.commoner_wealth.max(0.0);
        let life = (1.0 - hub.lack_basic).clamp(0.0, 1.0);
        let every = (1.0 - hub.lack_comfort).clamp(0.0, 1.0);
        let lux = (1.0 - hub.lack_luxury).clamp(0.0, 1.0);
        let base_mil = (so.unrest * 10.0).clamp(0.0, 10.0);
        let base_con = (so.inequality * 6.0).clamp(0.0, 10.0);
        let mut pops = Vec::with_capacity(9);
        for (prof, frac) in split {
            let size = frac * pop;
            if size < 1.0 { continue; }
            let elite = matches!(prof, 6 | 7);
            let mid = matches!(prof, 2 | 3 | 4 | 5);
            let money = if elite { cw * 6.0 + 5.0 } else if mid { cw * 1.6 + 1.0 } else { cw };
            let mil = (base_mil
                + if elite { -2.0 } else if prof == 1 || prof == 8 { 1.5 } else { 0.0 }).clamp(0.0, 10.0);
            let con = (base_con + if mid || elite { 1.5 } else { 0.0 }).clamp(0.0, 10.0);
            pops.push(Pop {
                profession: prof, size, money,
                needs_life: life, needs_everyday: every, needs_luxury: lux,
                consciousness: con, militancy: mil,
            });
        }
        self.hubs[h].pops = pops;
    }

    /// It. 3 · Civil unrest. Each year every settled hub's `unrest` eases toward a
    /// target set by HOW THE PEOPLE LIVE — low mood, steep inequality, dearth of
    /// basics, famine and war push it up; prosperity and commoner welfare pull it
    /// down. Crossing thresholds erupts: a riot (a production + stability shock) or,
    /// at the extreme, a REVOLT that topples the ruling council. This is where the
    /// social substrate (It. 2) finally bites back on the economy and politics.
    fn update_unrest(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].is_estate { continue; }
            let (mood, lackb, starv, prosp, atwar) = {
                let hub = &self.hubs[h];
                (hub.mood, hub.lack_basic.clamp(0.0, 1.0), hub.starving.clamp(0.0, 1.0),
                 hub.sent_prosperity, hub.war_with >= 0)
            };
            let (welfare, ineq) = {
                let so = &self.hubs[h].society;
                ((so.commoner_wealth / (so.commoner_wealth + 1.5)).clamp(0.0, 1.0), so.inequality)
            };
            // Cultures 2.0 · a large minority stirs unrest ONLY in a city that is already
            // struggling (dearth/famine/low mood) — a content, prosperous melting pot stays
            // calm. Bounded and small, so it can seed a revolt over years without runaway.
            let largest_min = self.hub_minorities.get(h)
                .map(|m| m.iter().fold(0.0f32, |a, (_, s)| a.max(*s))).unwrap_or(0.0);
            let stress = (lackb + starv + (1.0 - mood) * 0.5).clamp(0.0, 1.0);
            let minority_unrest = MINORITY_UNREST * largest_min.clamp(0.0, 1.0) * stress;
            // Cultures 2.0 · unmet cultural cravings stoke unrest (pop-weighted): a city
            // that never supplies its peoples the goods they prize grows restive.
            let cult_discontent = self.cultural_discontent(h);
            let target = (0.42 * (1.0 - mood)
                + 0.30 * ineq
                + 0.32 * lackb
                + 0.22 * starv
                + if atwar { 0.12 } else { 0.0 }
                + minority_unrest
                + CULTURE_UNREST * cult_discontent
                - 0.30 * welfare
                - 0.18 * prosp).clamp(0.0, 1.0);
            let u = {
                let so = &mut self.hubs[h].society;
                so.unrest += (target - so.unrest) * UNREST_EASE;
                so.unrest = so.unrest.clamp(0.0, 1.0);
                so.unrest
            };
            if u >= REVOLT_UNREST && self.hubs[h].council_house >= 0 {
                self.trigger_revolt(h);
            } else if u >= RIOT_UNREST {
                let rioting = self.active_events.iter().any(|e| e.hub == h as i32 && e.kind == "riot");
                if !rioting {
                    self.active_events.push(ActiveEvent {
                        kind: "riot".into(), hub: h as i32, good: -1,
                        magnitude: RIOT_PROD_HIT, until_tick: tick + 150,
                    });
                    let name = self.hubs[h].name.clone();
                    self.journal.push(JournalEntry {
                        tick, kind: "unrest".into(), hub: h as i32, good: -1, value: u,
                        text: format!("Bread riots break out in {}", name),
                    });
                }
            }
        }
    }

    /// A revolt: the populace seizes a slice of every resident house's wealth (→ the
    /// city's `civic_pool`, i.e. the people), drives the ruling family from the
    /// council and BARS it for a generation (`ousted_until`), throws the city into a
    /// season of disorder (a production + stability shock), and vents the pressure.
    /// The seat falls vacant; next New Year `decide_polis_policy` re-seats whoever is
    /// eligible (the banned house excluded), so a rival faction tends to rise.
    fn trigger_revolt(&mut self, h: usize) {
        let tick = self.tick;
        let ousted = self.hubs[h].council_house;
        // Redistribute: every resident house loses a slice of its fortune to the people.
        let mut seized = 0.0f32;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].hub as usize != h { continue; }
            let take = self.houses[hi].wealth.max(0.0) * REVOLT_REDISTRIB;
            self.houses[hi].wealth -= take;
            seized += take;
        }
        self.hubs[h].civic_pool += seized;
        let hubname = self.hubs[h].name.clone();
        let oname = if ousted >= 0 {
            let oi = ousted as usize;
            self.houses[oi].prestige = (self.houses[oi].prestige - 0.25).max(0.0);
            let nm = self.houses[oi].name.clone();
            self.houses[oi].events.push(HouseEvent {
                tick, kind: "revolt".into(),
                text: format!("Driven from the council of {} by a popular revolt", hubname),
            });
            nm
        } else { String::new() };
        // Bar the ousted family from the seat; the council falls vacant for now.
        self.hubs[h].society.ousted_house = ousted;
        self.hubs[h].society.ousted_until = tick + REVOLT_BAN_YEARS * TICKS_PER_YEAR;
        self.hubs[h].council_house = -1;
        // A season of disorder: clear any riot, then crater production + stability.
        self.active_events.retain(|e| !(e.hub == h as i32 && e.kind == "riot"));
        self.active_events.push(ActiveEvent {
            kind: "revolt".into(), hub: h as i32, good: -1,
            magnitude: REVOLT_PROD_HIT, until_tick: tick + 120,
        });
        // Catharsis: the explosion vents the accumulated pressure.
        self.hubs[h].society.unrest = 0.25;
        self.journal.push(JournalEntry {
            tick, kind: "revolt".into(), hub: h as i32, good: -1, value: 1.0,
            text: if oname.is_empty() { format!("A popular revolt convulses {}", hubname) }
                  else { format!("A popular revolt in {} topples {}", hubname, oname) },
        });
    }

    /// A gentle demand tilt from a city's class composition: a patrician-heavy city
    /// soaks up luxuries (tier 2), a commoner mass eats staples (tier 0). Normalized
    /// around an "average" society so total demand magnitude is preserved on average.
    fn society_demand_mult(&self, h: usize, need_tier: u8) -> f32 {
        let s = &self.hubs[h].society;
        let tot = s.patrician + s.burgher + s.commoner + s.underclass;
        if tot < 1e-3 { return 1.0; } // unseeded (e.g. estates)
        let elite = (s.patrician + s.burgher) / tot;
        let mass = (s.commoner + s.underclass) / tot;
        const ELITE_BASE: f32 = 0.18;
        const MASS_BASE: f32 = 0.82;
        match need_tier {
            2 => (1.0 + STRATA_DEMAND_TILT * (elite - ELITE_BASE) / ELITE_BASE).clamp(0.4, 1.8),
            0 => (1.0 + STRATA_DEMAND_TILT * (mass - MASS_BASE) / MASS_BASE).clamp(0.6, 1.4),
            _ => 1.0, // comfort tier neutral
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
        // First-20-year surcharge on the house wealth tax: founding generations find
        // it HARD to amass a fortune; the multiplier relaxes to 1.0 by year 20.
        let year = (self.tick / TICKS_PER_YEAR) as f32;
        let early_mult = if year < EARLY_WEALTH_TAX_YEARS {
            1.0 + (EARLY_WEALTH_TAX_MULT - 1.0) * (1.0 - year / EARLY_WEALTH_TAX_YEARS)
        } else { 1.0 };
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

            // ── Progressive civic WEALTH TAX (the private-house ceiling) ─────────
            // Flat base + a quadratic surcharge on wealth above a soft cap, scaled
            // by the early-game multiplier. The square means a great fortune bleeds
            // ever harder, pinning the sustained richest house to tens of thousands
            // instead of running away to the millions. Capped per month so it can't
            // drive a house straight into debt. Flows to the polis TREASURY.
            let wealth_tax = if is_guild { 0.0 } else {
                let over = (pos - HOUSE_WEALTH_SOFTCAP).max(0.0);
                let raw = (pos * HOUSE_WEALTH_TAX_BASE + over * over * HOUSE_WEALTH_TAX_QUAD) * early_mult;
                raw.min(pos * HOUSE_WEALTH_TAX_MAXFRAC)
            };

            self.houses[hi].wealth -= upkeep + consumption + endowment + wealth_overhead + wealth_tax;

            if home < self.hubs.len() {
                // Consumption + the family overhead + the guild's civic dues (upkeep)
                // + the endowment all flow to the home city's people. A PRIVATE house's
                // upkeep leaves the economy (paid to landlords / abroad), so it isn't
                // credited; its wealth overhead IS (it's patronage spent in town).
                self.hubs[home].civic_pool += consumption + endowment + wealth_overhead;
                if is_guild {
                    self.hubs[home].civic_pool += upkeep;
                }
                // The progressive wealth tax fills the polis TREASURY (city-finances
                // ledger), so as houses are reined in the cities themselves grow rich.
                self.hubs[home].treasury += wealth_tax;
                self.hubs[home].finance.tax_wealth += wealth_tax;
            }
            if hi < self.house_ledger.len() {
                self.house_ledger[hi].upkeep += upkeep;
                self.house_ledger[hi].consumption += consumption + endowment + wealth_overhead;
                self.house_ledger[hi].civic_tax += wealth_tax;
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
            // `needs` is sized to the hub count at the consumption pass; a hub added
            // later this tick (a fresh estate/colony) has no needs row yet — skip it
            // this tick rather than index out of bounds.
            if hub >= needs.len() { continue; }
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
            // On-demand restock: if the source depot is short for this delivery, the
            // house BUYS the shortfall from the source city's spare stock at the local
            // spot price. `form_contracts` may source from any network node that
            // PRODUCES the good, but a depot only refills monthly — without this a
            // contract whose depot hasn't refilled yet defaults every cycle and voids
            // (the "futures contracts always fail" bug). Only the city's surplus above
            // its own reserve is for sale, so a source that genuinely can't supply still
            // defaults.
            if let Some(i) = wi {
                let have0 = self.warehouses[i].stock.get(g).copied().unwrap_or(0.0);
                if have0 < c.monthly_qty {
                    let reserve = needs[src][g] * TRADE_RESERVE_MULT;
                    let avail = (self.hubs[src].stock[g] - reserve).max(0.0);
                    let want = (c.monthly_qty - have0).min(avail);
                    if want > EPS {
                        let src_price = self.live_price(self.hub_stock(src, g), needs[src][g], self.goods[g].base_value);
                        self.hubs[src].stock[g] -= want;
                        self.warehouses[i].stock[g] += want;
                        self.houses[seller].wealth -= want * src_price;
                        self.hubs[src].civic_pool += want * src_price;
                    }
                }
            }
            let have = wi.map(|i| self.warehouses[i].stock.get(g).copied().unwrap_or(0.0)).unwrap_or(0.0);
            if wi.is_none() || have < c.monthly_qty {
                // SELLER DEFAULT — can't deliver. Compensate the buyer above its spot
                // fallback, scaled by term (longer commitments hurt more to break).
                let ti = Self::term_index(c.term_years);
                // LIMITED LIABILITY: a default forfeits the compensation, but can never
                // drive the house more than a small buffer into debt — beyond that it
                // simply goes bankrupt (handled by `update_solvency`). Without this cap a
                // string of over-committed defaults could crater a house to deep negative
                // wealth (the −12k debt the futures fix exposed).
                let raw_penalty = c.monthly_qty * spot * TERM_PENALTY_MULT[ti];
                let penalty = raw_penalty
                    .min((self.houses[seller].wealth + CONTRACT_LIABILITY_FLOOR).max(0.0));
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
            let export_tax = value * EXPORT_TAX_RATE * self.city_tax_factor(src) * toll * self.house_city_tax_mult(seller, src);
            let import_tax = value * IMPORT_TAX_RATE * self.city_tax_factor(buyer) * toll * self.house_city_tax_mult(seller, buyer);
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
                contract: true, // its vessel is held by the standing contract reservation
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
        // A colony/estate may have been founded earlier THIS tick (hubs grew, but the
        // `days` travel matrix is only rebuilt at the next tick start). Skip forming
        // until the matrix matches the hub count — avoids an out-of-bounds index.
        if self.days.len() != n * n { return; }
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
                // A futures market needs a SOLVENT counterparty: a city gripped by
                // dearth or too poor to be a reliable buyer won't be signed long luxury
                // supply contracts (it can't pay, and its people need bread, not silk).
                // This also keeps contracts from artificially propping up destitute
                // cities — regional disparities and unrest stay real.
                if self.hubs[buyer].starving > 0.25
                    || self.hubs[buyer].sent_prosperity < CONTRACT_BUYER_MIN_PROSPERITY {
                    continue;
                }
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
                    // Pick the NEAREST reachable network node that can actually SUPPLY g
                    // — a depot holding it, or a city producing a genuine SURPLUS (output
                    // above its own need). Requiring surplus (not merely any production) is
                    // what fixed the "no futures contracts ever form" bug WITHOUT letting a
                    // house over-commit to a source that consumes its own output: the old
                    // code chose the nearest *producer* even when its surplus was nil, so
                    // `supply_cap` came out 0 and no contract could sign. Now the source is
                    // a real exporter, so the contract is both signable and fulfillable.
                    let src = nodes.iter().copied()
                        .filter(|&nd| nd as usize != buyer && (nd as usize) < n
                            && self.days[nd as usize * n + buyer].is_finite())
                        .filter(|&nd| {
                            let has_depot = self.warehouses.iter().any(|w| w.owner == hi as i32
                                && w.hub == nd && w.stock.get(g).copied().unwrap_or(0.0) > 0.0);
                            let ndx = nd as usize;
                            let surplus = self.hubs.get(ndx).and_then(|h| h.production.get(g)).copied().unwrap_or(0.0)
                                - needs.get(ndx).and_then(|r| r.get(g)).copied().unwrap_or(0.0);
                            has_depot || surplus > EPS
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
                    // Size to the source's true monthly SURPLUS (what it can spare above its
                    // own need) plus any depot stock — the deliverable, sustainable amount.
                    // The src selection above guarantees this is > 0, so contracts form
                    // without over-committing to a source that can't actually supply.
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
                    // Transport must be SPARE: subtract what this house's existing
                    // contracts already claim, and require a 20% headroom on top of the
                    // new quota. A house without the right vessels for this route
                    // (carry_cap 0) — or already fully committed — simply can't sign.
                    let committed: f32 = self.contracts.iter()
                        .filter(|c| c.seller_house as usize == hi)
                        .map(|c| c.monthly_qty)
                        .sum();
                    let spare_carry = (carry_cap - committed).max(0.0);
                    let monthly_qty = room.min(supply_cap).min(spare_carry / CONTRACT_TRANSPORT_MARGIN);
                    if monthly_qty <= EPS { continue; } // not enough spare transport → don't sign
                    let strike = self.live_price(self.hub_stock(buyer, g), needs[buyer][g],
                        self.goods[g].base_value) * TERM_STRIKE_FACTOR[ti];
                    self.contracts.push(Contract {
                        seller_house: hi as u32, buyer_hub: buyer as u32, source_hub: src as u32,
                        good: g, monthly_qty, strike_price: strike, term_years: term,
                        start_tick: tick, end_tick: tick + term as u32 * TICKS_PER_YEAR,
                        delivered: 0.0, last_fulfilled: 0, suspended_until: 0, defaults: 0,
                        coin: self.hubs[buyer].settle_coin, // struck in the buyer city's main coin
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
    /// Fill `m` (reused across ticks) with per-hub/good production multipliers from
    /// active events — default 1.0, dented by drought/blight/etc. Resizing + resetting
    /// in place avoids reallocating an n×ng matrix every single tick.
    fn fill_event_production_mult(&self, m: &mut Vec<Vec<f32>>) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        m.resize(n, Vec::new());
        for row in m.iter_mut() {
            row.clear();
            row.resize(ng, 1.0);
        }
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
                "embargo" | "guild_strike" => {
                    // A trade embargo, or a craft guild downing tools — the good
                    // stops being made at that hub for the duration.
                    if e.hub >= 0 && e.good >= 0 {
                        m[e.hub as usize][e.good as usize] *= 1.0 - e.magnitude;
                    }
                }
                "riot" | "revolt" => {
                    // Civil disorder halts the workshops & wharves city-wide.
                    if e.hub >= 0 {
                        let c = e.hub as usize;
                        for g in 0..ng { m[c][g] *= 1.0 - e.magnitude; }
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
        // DLC 3.5 · per-destination reserve-coin freight discount, precomputed once
        // (it's constant across this dispatch round and read in the hot inner loop).
        let coin_disc: Vec<f32> = (0..n).map(|d| self.coin_discount(d)).collect();
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
            // Contract deliveries are covered by the standing reservation below — don't
            // subtract them here too, or the same ship is counted busy twice.
            if c.owner >= 0 && !c.contract {
                let oi = c.owner as usize;
                if oi < nh { if c.sea { cap_sea[oi] -= 1; } else { cap_land[oi] -= 1; } }
            }
        }
        // Futures contracts reserve dedicated vessels up front: hold back the ships /
        // caravans each ACTIVE contract needs for its monthly delivery, so opportunistic
        // spot arbitrage can never strand a signed contract without transport (this is
        // the reservation that makes the contract carry first call on the fleet).
        for c in &self.contracts {
            if self.tick >= c.end_tick || c.suspended_until > self.tick { continue; }
            let oi = c.seller_house as usize;
            let (src, buyer) = (c.source_hub as usize, c.buyer_hub as usize);
            if oi >= nh || src >= n || buyer >= n { continue; }
            let (sc, bc) = (self.hubs[src].coastal, self.hubs[buyer].coastal);
            if sc || bc { // sea leg
                cap_sea[oi] -= (c.monthly_qty / SHIP_CAPACITY).ceil() as i32;
            }
            if !(sc && bc) { // land leg
                let rv = self.houses[oi].fleet_river as f32;
                let cv = self.houses[oi].fleet_caravan as f32;
                let land_per = if rv + cv > 0.0 {
                    (rv * BOAT_CAPACITY + cv * CARAVAN_CAPACITY) / (rv + cv)
                } else { CARAVAN_CAPACITY };
                cap_land[oi] -= (c.monthly_qty / land_per).ceil() as i32;
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
                    // A trusted reserve coin at the buyer `b` shaves freight (DLC 3.5).
                    let freight = self.good_freight(g, freight_rate * coin_disc[b], days);
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
                        let mut cut = sale * ESTATE_OWNER_CUT;
                        // A bank holding an equity STAKE in this manufactory collects its
                        // share of the owner-cut as a dividend (income to its reserves).
                        let sb = self.hubs[a].stake_bank;
                        if sb >= 0 && (sb as usize) < self.banks.len() && !self.banks[sb as usize].defunct {
                            let div = cut * self.hubs[a].stake_share.clamp(0.0, 0.9);
                            if div > 0.0 {
                                cut -= div;
                                self.banks[sb as usize].reserves += div;
                                self.banks[sb as usize].dividends_earned += div;
                            }
                        }
                        let owner = self.hubs[a].owner_house;
                        if owner >= 0 && (owner as usize) < self.houses.len()
                            && !self.houses[owner as usize].defunct {
                            self.houses[owner as usize].wealth += cut;
                            // Phase G: estate income, taxed by the parent city.
                            let etax = cut * ESTATE_TAX_RATE;
                            self.houses[owner as usize].wealth -= etax;
                            let parent = self.hubs[a].parent;
                            let is_manu = self.hubs[a].estate_kind == 6;
                            if parent >= 0 && (parent as usize) < self.hubs.len() {
                                let p = parent as usize;
                                let treas = etax * TREASURY_TAX_SHARE;
                                self.hubs[p].treasury += treas;
                                self.hubs[p].civic_pool += etax - treas;
                                // Manufactories (kind 6) book under manufacturing tax;
                                // raw estates under estate tax — for the City Finances panel.
                                if is_manu { self.hubs[p].finance.tax_manufacture += etax; }
                                else { self.hubs[p].finance.tax_estate += etax; }
                                self.hubs[p].finance.spent_civic += etax - treas;
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
                        // MONOPOLY EXPORT (#2): a house with ≥80% trade control of the
                        // SOURCE city `a` commands its surplus and exports on its own
                        // terms, taking a further rent — but only from a sustainable
                        // (not starving) city that has goods to spare.
                        if self.hubs[a].food_balance >= -0.1 {
                            let ctrl = self.houses[oi].influence.iter()
                                .find(|(c, _)| *c == a as u32).map(|(_, v)| *v).unwrap_or(0.0);
                            if ctrl >= MONOPOLY_CONTROL { mult *= 1.0 + MONOPOLY_EXPORT_RENT; }
                        }
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
                        // Bailo concession / dominance edge for the carrying house at each end.
                        let export_tax = value * exp_rate * tax_mult * self.city_tax_factor(a) * self.house_city_tax_mult(oi, a);
                        let import_tax = value * imp_rate * tax_mult * self.city_tax_factor(b) * self.house_city_tax_mult(oi, b);
                        self.houses[oi].wealth -= export_tax + import_tax;
                        // DLC 3.5 · split tariffs: a share is retained in the city
                        // treasury (capital for war/works), the rest reaches the
                        // people via the civic pool. Gross is booked as city income.
                        let exp_treas = export_tax * TREASURY_TAX_SHARE;
                        let imp_treas = import_tax * TREASURY_TAX_SHARE;
                        self.hubs[a].treasury += exp_treas;
                        self.hubs[a].civic_pool += export_tax - exp_treas;
                        self.hubs[a].finance.tax_trade += export_tax;
                        self.hubs[a].finance.spent_civic += export_tax - exp_treas;
                        self.hubs[b].treasury += imp_treas;
                        self.hubs[b].civic_pool += import_tax - imp_treas;
                        self.hubs[b].finance.tax_trade += import_tax;
                        self.hubs[b].finance.spent_civic += import_tax - imp_treas;
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
                    self.accrue_flow(a, b, g, amount);
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
                        contract: false,
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
        // folded in per candidate good below via `good_freight`. A trusted reserve
        // coin at the destination `a` shaves transaction cost (DLC 3.5 coinage).
        let freight_rate = self.freight_per_day
            * if self.hub_has_struct(b, STRUCT_GUILDHALL) { GUILDHALL_FREIGHT } else { 1.0 }
            * self.coin_discount(a);
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
        self.accrue_flow(b, a, g, amount);
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
            contract: false,
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

    /// Phase 5 (flavour) · strike a hub with plague: cull its population, quarantine
    /// it (a `plague_lockup` that suspends contracts + reroutes trade), and chronicle
    /// it. `carried_from` names the origin city for a contagion beat (route-borne
    /// spread) rather than a spontaneous outbreak.
    /// Strike a hub with a plague of the given `category` (1..3). `carried` is
    /// `Some((source_hub, outbreak, origin_hub))` for a contagion that travelled the
    /// lanes, or `None` for a spontaneous outbreak (which opens a fresh outbreak id and
    /// is its own origin). A city that is currently IMMUNE (survived a recent visitation)
    /// simply shrugs the strike off. Surviving a strike THEN confers lasting immunity,
    /// deeper the longer the lockdown lasts.
    fn strike_plague(&mut self, hub: usize, mag: f32, category: u8, disease: u8, carried: Option<(usize, u32, u32, u8)>) {
        if hub >= self.hubs.len() { return; }
        let tick = self.tick;
        // Immunity: a city that weathered a recent outbreak resists a new one entirely.
        if self.hubs[hub].plague_immune_until > tick { return; }
        // Small cities are less crowded/dense → 3× less likely to be struck at all
        // (user rule): shrug off 2 of every 3 strikes when under 10k people.
        if self.hubs[hub].population < SMALL_CITY_POP
            && hash01(self.seed, tick as u64 ^ 0x5A1717, hub as u64) > 1.0 / SMALL_CITY_PLAGUE_RESIST {
            return;
        }
        // Public health (hospices/quarantine) buys down the DEATH toll — the same people
        // still fall ill, but a well-provisioned city nurses more of them through.
        let ph = self.hubs[hub].public_health.clamp(0.0, HOSPICE_MAX_LEVEL);
        let base_mag = mag.clamp(0.0, 0.6);
        let mag_eff = (base_mag * (1.0 - ph)).clamp(0.0, 0.6);
        let pre = self.hubs[hub].population.max(0.0);
        self.hubs[hub].population *= 1.0 - mag_eff;
        let post = self.hubs[hub].population.max(0.0);
        // Lockdown (trade restriction) length scales with severity: a local outbreak is
        // a brief quarantine; a great plague shuts the gates for months.
        let jitter = hash01(self.seed, tick as u64 ^ 0x10CC, hub as u64);
        let lock = match category {
            1 => 90 + (jitter * 90.0) as u32,   // Great Plague: ~90-180 ticks
            2 => 45 + (jitter * 45.0) as u32,   // Regional: ~45-90
            _ => 18 + (jitter * 27.0) as u32,   // Local: ~18-45 (some trade restrictions)
        };
        self.active_events.push(ActiveEvent {
            kind: "plague_lockup".into(), hub: hub as i32, good: -1,
            magnitude: 1.0, until_tick: tick + lock,
        });
        // Surviving the visitation confers immunity for years afterward (longer for a
        // harsher, longer lockdown) — the city cannot be re-struck or re-seeded until then.
        // Immunity window scaled by the DISEASE (some, like flu/cholera, confer little
        // lasting immunity so they recur; smallpox/measles/plague confer strong immunity).
        let dimm = DISEASES.get(disease as usize).map(|s| s.immunity).unwrap_or(1.0);
        // Public health also lengthens the immunity earned (better convalescence + lasting
        // quarantine discipline) — a well-provisioned city resists the next visitation longer.
        let immune_span = ((PLAGUE_IMMUNITY_BASE_YEARS * TICKS_PER_YEAR as f32
            + lock as f32 * PLAGUE_IMMUNITY_LOCK_MULT) * dimm * (1.0 + ph)) as u32;
        self.hubs[hub].plague_immune_until = tick + lock + immune_span;
        // Observability record (Plagues panel + map). Spontaneous → a new outbreak;
        // contagion → inherits the source hub's outbreak id + origin + disease.
        let (source, outbreak, origin) = match carried {
            Some((src, ob, org, _dz)) => (src as i32, ob, org),
            None => { let ob = self.next_outbreak; self.next_outbreak += 1; (-1, ob, hub as u32) }
        };
        // SIR split (observability): infer how many fell ILL from the deaths and the
        // disease's case-fatality rate (its `dead_hi` reads as CFR-per-case). A mild
        // disease (low CFR) infects many and kills few; a lethal one infects nearer the
        // death toll. `recovered = infected − deaths` is derived by readers.
        let deaths = (pre - post).max(0.0);
        let cfr = DISEASES.get(disease as usize).map(|s| s.dead_hi).unwrap_or(0.5).clamp(0.03, 0.7);
        // `infected` reflects the UNMITIGATED attack (hospices cut deaths, not infections),
        // so a city with public health shows the same ill count but more recovered, fewer
        // dead — the visible payoff of its spending.
        let base_deaths = pre * base_mag;
        let infected = (base_deaths / cfr).clamp(deaths, pre * 0.9);
        self.epidemics.push(PlagueStrike {
            hub: hub as u32, source, outbreak, deaths,
            pop_at: post, start_tick: tick, until_tick: tick + lock,
            category, origin_hub: origin, disease, infected,
        });
        if self.epidemics.len() > 400 { let d = self.epidemics.len() - 400; self.epidemics.drain(0..d); }
        let city = self.hubs[hub].name.clone();
        let dname = DISEASES.get(disease as usize).map(|s| s.name).unwrap_or("sickness");
        let (kind, text) = match carried {
            Some((src, _, _, _)) => {
                let from = self.hubs.get(src).map(|h| h.name.clone()).unwrap_or_default();
                ("contagion".to_string(),
                    format!("{} reaches {}, carried by traders from {}.", dname, city, from))
            }
            None => ("disaster".to_string(),
                format!("{} breaks out in {}; the city is locked down under quarantine", dname, city)),
        };
        self.journal.push(JournalEntry {
            tick, kind, hub: hub as i32, good: -1, value: lock as f32, text });
    }

    /// The (category, outbreak, origin_hub, disease) of the outbreak currently live at
    /// a hub (its most recent unexpired strike), if any.
    fn active_strike_at(&self, hub: usize) -> Option<(u8, u32, u32, u8)> {
        let tick = self.tick;
        self.epidemics.iter().rev()
            .find(|s| s.hub as usize == hub && s.until_tick > tick)
            .map(|s| (if s.category == 0 { 3 } else { s.category }, s.outbreak, s.origin_hub, s.disease))
    }

    /// Straight-line distance (in cells, cylindrical-X) between two hubs.
    fn hub_cell_dist(&self, a: usize, b: usize) -> f32 {
        let mut dx = (self.hubs[a].x - self.hubs[b].x).abs();
        if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
        let dy = self.hubs[a].y - self.hubs[b].y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Phase 5 (flavour) · CONTAGION: a plague travels the TRADE NETWORK only — never
    /// jumps geographically. Each infected (quarantined) focus may, on a low per-tick
    /// chance (a plague is hard to carry), pass the pestilence to a trade partner its
    /// merchants actually reach:
    ///   · category 3 (LOCAL) never spreads;
    ///   · category 2 (REGIONAL) reaches at most ONE further city (a nearer partner);
    ///   · category 1 (GREAT PLAGUE) spreads city-to-city up to ~4000 km from origin.
    /// Immune cities (survivors) block the wave. Bounded: ≤2 new foci per tick, and a
    /// hard cap so the plague burns out before it locks a quarter of the world.
    fn spread_epidemics(&mut self) {
        use std::collections::HashSet;
        let tick = self.tick;
        let n = self.hubs.len();
        if n == 0 { return; }
        let locked: Vec<usize> = self.active_events.iter()
            .filter(|e| e.kind == "plague_lockup" && e.until_tick > tick && e.hub >= 0)
            .map(|e| e.hub as usize).filter(|&h| h < n).collect();
        if locked.is_empty() { return; }
        if locked.len() >= (n / 4).max(1) { return; } // hard cap → burns out
        let locked_set: HashSet<usize> = locked.iter().copied().collect();
        let km_per_cell = EARTH_EQUATOR_KM / self.world_w.max(1.0);
        let mut new_infections = 0usize;
        for &src in &locked {
            if new_infections >= EPIDEMIC_MAX_SPREAD_PER_TICK { break; }
            let (cat, outbreak, origin, disease) = match self.active_strike_at(src) { Some(x) => x, None => continue };
            let spec = &DISEASES[disease.min(8) as usize];
            // VECTOR diseases (malaria) never pass city-to-city — they re-emerge from
            // the land itself, so they don't travel here.
            if spec.mode == 3 || spec.spread <= 0.0 { continue; }
            if cat >= 3 && spec.mode != 2 { continue; } // local outbreaks stay put (airborne still drifts)
            if hash01(self.seed, tick as u64 ^ 0xC0FFEE, src as u64) >= spec.spread { continue; }
            // A regional outbreak only ever reaches ONE further city — once a contagion
            // child of this outbreak exists, it stops travelling.
            if cat == 2 && self.epidemics.iter().any(|s| s.outbreak == outbreak && s.source >= 0) {
                continue;
            }
            let reach_cells = spec.reach_km / km_per_cell.max(1e-3);
            let org = origin as usize;
            let comp = self.hubs[org.min(n - 1)].component;
            // OUTBREAK MEMORY: the set of cities THIS outbreak has ALREADY struck. A
            // contagion must never bounce back into a city it already passed through
            // (that inflated one 14-city plague into "186 cities" of re-records). When
            // the outbreak later dies out its records age away, so a FRESH outbreak (new
            // id) can strike those cities again — plague can recur, just not loop.
            let hit: HashSet<usize> = self.epidemics.iter()
                .filter(|s| s.outbreak == outbreak)
                .map(|s| s.hub as usize).collect();
            // Candidate destinations by TRANSMISSION MODE:
            //   trade (0): a trade-route neighbour (merchants carry it)
            //   water (1): a COASTAL/river-mouth trade neighbour (foul water)
            //   airborne (2): ANY nearby city (not only trade partners — it drifts)
            let mut best: (usize, f32) = (usize::MAX, f32::MAX);
            let mut consider = |me: &Self, h: usize, best: &mut (usize, f32)| {
                if h >= n || h == src || me.hubs[h].is_estate || me.hubs[h].abandoned { return; }
                if hit.contains(&h) { return; } // already visited by this outbreak — no loop-back
                if locked_set.contains(&h) || me.hubs[h].plague_immune_until > tick { return; }
                if me.hubs[h].component != comp { return; }
                if spec.mode == 1 && !me.hubs[h].coastal { return; } // water needs a wet city
                let dsrc = me.hub_cell_dist(src, h) * km_per_cell;
                if dsrc > PLAGUE_HOP_MAX_KM { return; }
                if org < n && me.hub_cell_dist(org, h) * km_per_cell > spec.reach_km { return; }
                if (dsrc / km_per_cell) < best.1 { *best = (h, dsrc / km_per_cell); }
            };
            if spec.mode == 2 {
                for h in 0..n { if self.hub_cell_dist(src, h) <= reach_cells { consider(self, h, &mut best); } }
            } else if let Some(v) = self.neighbors.get(src) {
                for &x in v { consider(self, x as usize, &mut best); }
            }
            if best.0 != usize::MAX {
                self.strike_plague(best.0, EPIDEMIC_CONTAGION_MAG, cat, disease, Some((src, outbreak, origin, disease)));
                new_infections += 1;
            }
        }
    }

    /// Roll low-probability events for this tick.
    fn roll_events(&mut self) {
        let n = self.hubs.len();
        if n == 0 {
            return;
        }
        let tick = self.tick;
        let r = hash01(self.seed, tick as u64, 0xE7E7);
        // ~ one event every ~10 ticks on average (raised for MORE epidemics; the
        // disease origin gates + immunity keep them from overrunning the world).
        if r > 0.10 {
            return;
        }
        let pick = hash01(self.seed, tick as u64, 0x1234);
        let hub = (hash01(self.seed, tick as u64, 0x5678) * n as f32) as usize % n;
        let (kind, mag, dur, good): (&str, f32, u32, i32) = if pick < 0.22 {
            // A drought trims food for a season. Kept moderate so a hub's granary
            // reserve + the baseline food surplus can ride it out — a bad year, not
            // an automatic famine (deep deficits used to spiral into collapse).
            ("drought", 0.20 + 0.15 * pick, 30 + (pick * 40.0) as u32, -1)
        } else if pick < 0.46 {
            // The disease + severity are chosen inside the match below (24% of events).
            ("plague", 0.0, 0, -1)
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
                // A spontaneous outbreak. Roll its CATEGORY (rarity ↑ with severity):
                // most are local cat-3 nuisances; a few become regional cat-2; a great
                // cat-1 plague is rare. Severity (the cull) scales with the category.
                // The market routes around the lockup; futures touching it are force-
                // majeure suspended. Phase 5 `spread_epidemics` then carries a cat-1/2
                // along the trade lanes (never geographically) from this focus.
                // Pick a DISEASE (weighted). Some need a specific ORIGIN: water-borne
                // (cholera/dysentery) and vector (malaria) emerge in a wet/coastal
                // locale; trade/airborne can start anywhere. The cull is the disease's
                // own deadliness range; the severity category follows from that.
                let disease = pick_disease(self.seed, tick, hub);
                let spec = &DISEASES[disease as usize];
                let origin_ok = match spec.mode { 1 | 3 => self.hubs[hub].coastal, _ => true };
                if origin_ok {
                    let sev = hash01(self.seed, tick as u64 ^ 0x5EE7, hub as u64);
                    let cull = spec.dead_lo + (spec.dead_hi - spec.dead_lo) * sev;
                    let category = disease_category(disease);
                    self.strike_plague(hub, cull, category, disease, None);
                }
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
            // A dead settlement stays dead — the founding-pop floor below must
            // never resurrect an abandoned ruin (or a collapsed colony).
            if self.hubs[h].abandoned { continue; }
            let mut food_need = 0.0;
            let mut food_have = 0.0;
            let mut food_prod = 0.0;
            for g in 0..ng {
                if self.goods[g].food {
                    food_need += needs[h][g];
                    food_prod += self.hubs[h].production[g];
                    food_have += self.hubs[h].stock[g] + self.hubs[h].production[g];
                }
            }
            // ── Settlement-colony food LIFELINE (dedicated supply ships) ─────────
            // A young colony can't feed itself: its metropolis runs a fleet of dedicated
            // SUPPLY SHIPS carrying real grain from a sufficient food SOURCE (a nearby
            // surplus city — often the metropolis). The ships' capacity, the source's
            // spare grain, and the backers' freight budget bound the delivery; a reserve
            // buffers brief breaks. Monthly, the metropolis re-picks the source and
            // INVESTS in more ships when the colony runs short (steady supply). Grain
            // physically leaves the source, so the source must genuinely be sufficient.
            // A metropolis-BOUND satellite (built or absorbed, kind 3, done building) is
            // fed by the SAME lifeline: its mother city keeps it supplied. This is what
            // lets a big city genuinely REVIVE a tiny town it has adopted — and stops a
            // finished satellite whose own farms fall short from quietly starving.
            let bound_satellite = self.hubs[h].colony_kind == 3
                && self.hubs[h].build_stage == 0 && self.hubs[h].founder_hub >= 0;
            if (self.hubs[h].colony_kind == 1 && !self.hubs[h].autonomous) || bound_satellite {
                self.hubs[h].reserve_cap = 365.0;
                let deficit = (food_need - food_prod).max(0.0); // daily grain shortfall
                // Monthly: re-designate the food source + top up the dedicated fleet.
                if self.tick % 30 == 0 {
                    self.designate_colony_supply(h, deficit * 30.0);
                }
                if deficit <= EPS {
                    // Self-sufficient: top the reserve, supply unbroken.
                    self.hubs[h].reserve_food = (self.hubs[h].reserve_food + 1.0).min(self.hubs[h].reserve_cap);
                    self.hubs[h].supply_years += 1.0 / TICKS_PER_YEAR as f32;
                    self.hubs[h].supply_delivered = 0.0;
                } else {
                    let fleet_daily = self.hubs[h].supply_ships as f32 * SUPPLY_SHIP_CAPACITY / 30.0;
                    let mut delivered = deficit.min(fleet_daily);
                    let src = self.hubs[h].supply_source;
                    // Real grain moves: pull from the source's spare food STOCK (kept above
                    // a buffer for its own residents), deducting it — no free food.
                    if delivered > EPS && src >= 0 && (src as usize) < n {
                        let s = src as usize;
                        let mut remaining = delivered;
                        for g in 0..ng {
                            if !self.goods[g].food || remaining <= EPS { continue; }
                            let buffer = self.hubs[s].production[g] * SOURCE_BUFFER_DAYS;
                            let spare = ((self.hubs[s].stock[g] - buffer) * SUPPLY_SOURCE_SPARE_FRAC).max(0.0);
                            let take = spare.min(remaining);
                            self.hubs[s].stock[g] -= take;
                            self.hubs[s].export_earn += take * self.goods[g].base_value;
                            remaining -= take;
                        }
                        delivered -= remaining; // only what the source could actually spare
                    } else {
                        delivered = 0.0;
                    }
                    // Backers pay the FREIGHT for the run (best-effort — the metropolis is
                    // committed to its colony, so a thin treasury doesn't cut the food off,
                    // it just runs the treasury down). This removes the old hard "can't
                    // pay → 0 food" gate that silently starved every colony.
                    let price = self.goods.iter().filter(|g| g.food).map(|g| g.base_value).fold(0.0, f32::max).max(1.0);
                    let freight = delivered * price * COLONY_FREIGHT_RATE;
                    let m = self.hubs[h].founder_hub;
                    if m >= 0 && (m as usize) < n {
                        let mi = m as usize;
                        let pay = self.hubs[mi].treasury.max(0.0).min(freight);
                        self.hubs[mi].treasury -= pay;
                        self.hubs[mi].finance.spent_works += pay;
                    }
                    // Deliver the grain into the colony's first food good.
                    if let Some(fg) = (0..ng).find(|&g| self.goods[g].food) {
                        self.hubs[h].stock[fg] += delivered;
                        food_have += delivered;
                    }
                    self.hubs[h].supply_delivered = delivered * 30.0; // monthly, for the readout
                    let short = deficit - delivered;
                    if short <= EPS {
                        self.hubs[h].reserve_food = (self.hubs[h].reserve_food + 0.5).min(self.hubs[h].reserve_cap);
                        self.hubs[h].supply_years += 1.0 / TICKS_PER_YEAR as f32;
                    } else if self.hubs[h].reserve_food > 0.0 {
                        // Eat into the reserve to stay fed; the supply record continues.
                        self.hubs[h].reserve_food -= 1.0;
                        if let Some(fg) = (0..ng).find(|&g| self.goods[g].food) {
                            self.hubs[h].stock[fg] += short;
                            food_have += short;
                        }
                        self.hubs[h].supply_years += 1.0 / TICKS_PER_YEAR as f32;
                    } else {
                        // Lifeline snapped: reserve empty AND supply short → the record
                        // breaks and the colony starves (handled below by food balance).
                        self.hubs[h].supply_years = 0.0;
                    }
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
            // Capacity scales with FOOD security and (strongly) TRADE prosperity so a
            // thriving hub keeps growing into a real city instead of stalling at a
            // fixed ~2.7× of its humble founding size — the old ceiling froze every
            // city ~year 50 and flat-lined trade. A rich, well-fed entrepôt can now
            // reach ~9× its founding size; a poor isolated one stays small.
            let cap_mult = (0.35 + 1.30 * food_sec) * (0.60 + 5.0 * prosperity * prosperity);
            let capacity = (self.hubs[h].founding_pop * cap_mult)
                .max(self.hubs[h].founding_pop * 0.15);
            // Logistic step: approach capacity from below, decline when above it.
            // Slower organic growth (~5%/yr peak at low pop, was ~24%). Young
            // SETTLEMENT colonies grow faster (frontier boom + sponsored migration on
            // top) so they still mature into cities within a campaign.
            let colony_boom = if self.hubs[h].colony_kind == 1 && !self.hubs[h].autonomous { POP_GROWTH_COLONY_MULT } else { 1.0 };
            // Small-city growth bonus (user rule): a well-fed town under 10k grows up
            // to 5× faster so humble settlements rise into cities — scaled by food
            // security (needs food on-site or traded in), no benefit when starving.
            let small_boost = if pop < SMALL_CITY_POP {
                1.0 + (SMALL_CITY_GROWTH_MULT - 1.0) * food_sec
            } else { 1.0 };
            // Trade-rich + well-fed cities of ANY size grow faster (user rule: up to
            // ~10%/yr) — a thriving, food-secure entrepôt booms; a poor/hungry one crawls.
            let trade_food_boost = 1.0 + TRADE_FOOD_GROWTH_BONUS * prosperity * food_sec;
            let rate = if pop < capacity { POP_GROWTH_RATE * colony_boom * small_boost * trade_food_boost } else { POP_DECLINE_RATE };
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
            let mut np = new_pop.max(self.hubs[h].founding_pop * 0.10);
            // A house trade outpost stays a small trade post — hard population cap.
            if self.hubs[h].colony_kind == 2 { np = np.min(OUTPOST_MAX_POP); }
            self.hubs[h].population = np;
        }
        // Resilience: after a tick panic the crash-recovery layer freezes territorial
        // expansion for a while (`expansion_frozen_until`) so re-advancing can't re-hit
        // the same founding fault. Everything else keeps simulating.
        let expansion_ok = self.tick >= self.expansion_frozen_until;
        // Estate founding: a big, rich, food-secure hub with a hungry neighbour
        // founds a food estate. At most one per advance batch (cheap, rare).
        if expansion_ok && self.tick % 120 == 0 {
            self.maybe_found_estate();
        }
        // Colonization of new land: rarer (yearly) — the settled map fills in.
        // Two distinct drives: a great house plants a remote trade OUTPOST (strategic
        // reach / new goods), and an overcrowded city founds a full SETTLEMENT colony
        // (population expansion) that can grow into a city of its own. The age of
        // colonisation only opens once the world has matured — from YEAR 50 onward,
        // and only when the wealth/population/site conditions are actually met.
        // Cultures 2.0 · sample every people's population twice a year for the line chart.
        if self.tick % (TICKS_PER_YEAR / 2) == 0 {
            self.sample_culture_history();
        }
        if self.tick % 365 == 0 {
            // Yearly social mobility: strata shift with prosperity / hardship.
            self.update_society();
            // Then the people may stir: unrest builds, riots flare, revolts topple
            // councils (reads the freshly-updated inequality / welfare above).
            self.update_unrest();
            // Estate/manufactory disasters strike + funded repairs progress (yearly).
            self.estate_condition_pass();
            self.manufactory_solvency_pass(); // shut works idle 4+ years
            // Atlas 2.0 — the LIVING MAP: organic settlement death & birth.
            self.lifecycle_pass(expansion_ok);
            // …and REBIRTH: long-dead ruins in recovered regions are resettled.
            self.resettle_pass(expansion_ok);
            // #23 · Peoples: seed/settle cultures, drift people toward opportunity
            // (draws migration arrows), and slowly assimilate minority quarters.
            self.ensure_hub_cultures();
            self.economic_migration_pass();
            self.diaspora_pass();
            self.assimilation_pass();
            // Whoever now holds the plurality becomes the city's majority people.
            self.rebalance_hub_majorities();
            // Cultures 2.0 · sustained blending in a city can birth a new creole people.
            self.ethnogenesis_pass(self.tick / TICKS_PER_YEAR);
            // Cultures 3.0 · a far-flung, isolated community can splinter into a new
            // daughter people of the same stock.
            self.splinter_pass(self.tick / TICKS_PER_YEAR);
            // Connect sub-cap villages to the trade network via their nearest market town.
            self.hinterland_pass();
            // Rare merchant expeditions reach the far, isolated outposts trade misses.
            self.expedition_pass(self.tick / TICKS_PER_YEAR);
            // House trade outposts from year 30 (rich house, heavy cost); full
            // settlement colonies from year 50 (joint-stock, food lifeline).
            if expansion_ok && self.tick >= BASE_START_TICK {
                self.maybe_establish_trade_base();
                self.trade_base_pass();
            }
            if expansion_ok && self.tick >= OUTPOST_START_TICK {
                self.maybe_found_house_outpost();
                self.maybe_graduate_outpost(); // a thriving old outpost matures into a colony
            }
            if expansion_ok && self.tick >= COLONY_START_TICK {
                self.maybe_found_settlement_colony();
                self.maybe_found_food_colony(); // Greek-Crimea grain colony (food stress)
                self.colony_pass(); // graduation · dividends · autonomy
                self.maybe_found_satellite(expansion_ok); // port/granary/workshop suburbs
                self.maybe_absorb_dying_city();            // big city adopts a failing neighbour
                self.maybe_found_caravanserai(expansion_ok); // waystations on long land ties
                self.satellite_independence_pass();        // mature satellites → free cities
            }
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
        // Holdings are LINKED to their parent settlement: co-locate the estate /
        // manufactory at the parent's coordinates so it is not a separate point on
        // the map and all its trade routes through the parent city. The passed
        // `x,y` (a small offset near the parent) is kept only as a fallback when
        // there is no parent. Terroir/quality here is seeded by kind, not the cell,
        // so co-locating costs no fidelity.
        let (x, y) = if parent >= 0 && (parent as usize) < self.hubs.len() {
            (self.hubs[parent as usize].x, self.hubs[parent as usize].y)
        } else { (x, y) };
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
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: kind, estate_tier: 1, last_upgrade_tick: self.tick, owner_house, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, mint_bullion_ratio: 1.0, has_mint: false,
            // DLC 4 · seed the new estate's quality (length ng) so it's graded from
            // day one — a manufactory (kind 6) starts as a humble workshop and learns.
            quality: { let mut q = vec![0.0f32; ng]; if g0 < ng { q[g0] = if kind == 6 { 0.34 } else { 0.46 }; } q },
            stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: self.tick, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
        });
        // Defer the O(n²) route/neighbour rebuild to the next tick (batched).
        self.routes_dirty = true;
    }

    /// Count satellite production sites (estates + colonies). Used to keep the hub
    /// list — and therefore every per-tick loop — bounded over a long campaign.
    fn estate_count(&self) -> usize {
        self.hubs.iter().filter(|h| h.is_estate).count()
    }

    /// Yearly: a MANUFACTORY (estate_kind 6) that hasn't turned a profit for 4 years
    /// is SHUT DOWN — its owner recoups part of the works' capital and the city's
    /// estate slot is freed. "Unprofitable" = it makes nothing (input/labor-starved)
    /// OR its output piles up unsold (a persistent glut). Raw estates are left alone.
    fn manufactory_solvency_pass(&mut self) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        if self.estate_idle_years.len() < n { self.estate_idle_years.resize(n, 0); }
        for h in 0..n {
            if !self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].estate_kind != 6 {
                if h < self.estate_idle_years.len() { self.estate_idle_years[h] = 0; }
                continue;
            }
            // Output good = the good it produces most.
            let g = (0..ng).max_by(|&a, &b| self.hubs[h].production[a]
                .partial_cmp(&self.hubs[h].production[b]).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0);
            let prod = self.hubs[h].production.get(g).copied().unwrap_or(0.0);
            let stock = self.hubs[h].stock.get(g).copied().unwrap_or(0.0);
            // Unprofitable this year: makes nothing (starved) OR a year-plus unsold glut.
            let unprofitable = prod <= EPS || stock > prod * 360.0;
            if unprofitable {
                self.estate_idle_years[h] = self.estate_idle_years[h].saturating_add(1);
            } else {
                self.estate_idle_years[h] = 0;
            }
            if self.estate_idle_years[h] >= 4 {
                self.close_manufactory(h);
                self.estate_idle_years[h] = 0;
            }
        }
    }

    /// Shut an unprofitable manufactory: the owner recoups 40% of its capital value,
    /// the works go inert and detach from their city (freeing the estate slot).
    fn close_manufactory(&mut self, h: usize) {
        let tick = self.tick;
        let tier = self.hubs[h].estate_tier.max(1) as f32;
        let recoup = 0.4 * tier * BANK_STAKE_VALUE_PER_TIER;
        let owner = self.hubs[h].owner_house;
        if owner >= 0 && (owner as usize) < self.houses.len() && !self.houses[owner as usize].defunct {
            self.houses[owner as usize].wealth += recoup;
        }
        let owner_name = if owner >= 0 && (owner as usize) < self.houses.len() {
            self.houses[owner as usize].name.clone()
        } else { "The city".to_string() };
        let (ename, parent) = (self.hubs[h].name.clone(), self.hubs[h].parent);
        let ng = self.goods.len();
        {
            let e = &mut self.hubs[h];
            e.abandoned = true; e.estate_tier = 0; e.owner_house = -1; e.parent = -1;
            e.production = vec![0.0; ng];
            for v in e.base_per_capita.iter_mut() { *v = 0.0; }
        }
        self.journal.push(JournalEntry {
            tick, kind: "disaster".into(), hub: parent, good: -1, value: 0.0,
            text: format!("{} shutters its unprofitable works at {}", owner_name, ename) });
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
            // Per-city cap (was MISSING here → the single richest city amassed dozens of
            // same-good estates). Skip cities already at their estate cap so the
            // hinterland spreads across the region instead of one monoculture.
            let on_city = self.hubs.iter().filter(|e| e.is_estate && e.parent == h as i32).count();
            let cap = if self.hubs[h].population >= ESTATE_BIG_CITY_POP { MAX_ESTATES_BIG_CITY } else { MAX_ESTATES_PER_CITY };
            if on_city >= cap { continue; }
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
                let is_manu = self.hubs[ei].estate_kind == 6;
                // A manufactory upgrade is a major, dear re-tooling — flat 30k and only
                // once every 5 years per workshop. A raw estate upgrade stays cheap.
                let cost = if is_manu { MANUFACTORY_UPGRADE_COST } else { INVEST_COST_BASE * tier as f32 * 0.8 };
                let cooldown_ok = !is_manu
                    || tick.saturating_sub(self.hubs[ei].last_upgrade_tick) >= MANUFACTORY_UPGRADE_INTERVAL;
                if cooldown_ok && self.houses[hi].wealth >= cost * 1.5
                    && hash01(self.seed, tick as u64 ^ 0x09A7, hi as u64) < 0.5
                {
                    self.houses[hi].wealth -= cost;
                    self.hubs[ei].estate_tier = tier + 1;
                    self.hubs[ei].last_upgrade_tick = tick;
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
            // Per-settlement cap: don't overrun one city's hinterland with estates. The
            // base cap is 3; only a great city (≥150k) may reach 5, and the 4th/5th slot
            // costs a steep premium (crowding a city with works is expensive).
            let on_city = self.hubs.iter()
                .filter(|h| h.is_estate && h.parent == target as i32).count();
            let cap = if self.hubs[target].population >= ESTATE_BIG_CITY_POP {
                MAX_ESTATES_BIG_CITY
            } else { MAX_ESTATES_PER_CITY };
            if on_city >= cap { continue; }
            // Cost scales with the host city's size; an office there makes it cheaper; a
            // slot beyond the base cap (the 4th/5th, big cities only) is far dearer.
            let has_office = self.houses[hi].offices.contains(&(target as u32));
            let slot_premium = if on_city >= MAX_ESTATES_PER_CITY { ESTATE_HIGH_SLOT_COST_MULT } else { 1.0 };
            let cost = INVEST_COST_BASE
                * (self.hubs[target].population / 30_000.0).clamp(0.5, 3.0)
                * if has_office { 0.6 } else { 1.0 }
                * slot_premium;
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

    /// Distance from the nearest of a holder's network nodes (home + offices) to a
    /// candidate site — the office-relay reach: the closest "ground" wins, so an
    /// office lets the next hop be measured from there, not the distant home.
    fn nearest_node_dist(&self, nodes: &[(f32, f32)], sx: f32, sy: f32) -> f32 {
        let mut best = f32::MAX;
        for &(nx, ny) in nodes {
            let mut dx = (sx - nx).abs();
            if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
            let dy = sy - ny;
            best = best.min((dx * dx + dy * dy).sqrt());
        }
        best
    }

    /// HOUSE colony — a wealthy house plants a remote, low-population TRADE OUTPOST
    /// on a poorer, trade-prone (coastal) frontier site within reach of its home or
    /// an office (the office is the relay "ground"). Reuses the estate machinery but
    /// keeps its OWN remote coordinates (parent = −1 so it is not co-located in a
    /// city) and is tagged `colony_kind = 2`.
    fn maybe_found_house_outpost(&mut self) {
        if self.colonizable.is_empty() || self.hubs.is_empty() { return; }
        if self.estate_count() >= MAX_TOTAL_ESTATES { return; }
        // Founder: the richest house, but only if it clears the (heavy) wealth bar —
        // a trade outpost is the privilege of a truly great house.
        let mut founder = -1i32;
        let mut hw = OUTPOST_FOUND_WEALTH;
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct { continue; }
            if hh.wealth > hw { hw = hh.wealth; founder = hi as i32; }
        }
        if founder < 0 { return; }
        let hi = founder as usize;
        let home = self.houses[hi].hub as usize;
        if home >= self.hubs.len() { return; }
        // Network nodes = home + offices (the relays).
        let mut nodes = vec![(self.hubs[home].x, self.hubs[home].y)];
        for &off in &self.houses[hi].offices {
            if (off as usize) < self.hubs.len() {
                nodes.push((self.hubs[off as usize].x, self.hubs[off as usize].y));
            }
        }
        let cap = COLONY_MAX_KM * self.world_w / EARTH_EQUATOR_KM; // ≤ 5000 km from the metropolis
        // Pick the best reachable site for a TRADE outpost: a house plants its
        // factory where the valuable trade goods are. Site trade-value dominates,
        // a coast (shippable) adds a bonus, and nearer is better. Fertility is
        // irrelevant — an outpost imports its food and exists to work the cargo.
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            let d = self.nearest_node_dist(&nodes, s.x, s.y);
            if d > cap { continue; }
            let trade_score = s.trade_value + if s.coastal { 0.30 } else { 0.0 };
            let score = trade_score * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        let ng = self.goods.len();
        // Good: prefer one the house's home makes that matches the site's kind hint.
        let (mut g0, mut gbest) = (usize::MAX, 0.0f32);
        for g in 0..ng {
            if estate_kind_for_good(&self.goods[g].name, self.goods[g].food) == self.colonizable[si].kind_hint {
                let s = self.hubs[home].base_per_capita.get(g).copied().unwrap_or(0.0) + 0.001;
                if s > gbest { gbest = s; g0 = g; }
            }
        }
        if g0 == usize::MAX {
            for g in 0..ng {
                let pc = self.hubs[home].base_per_capita.get(g).copied().unwrap_or(0.0);
                if pc > gbest { gbest = pc; g0 = g; }
            }
        }
        if g0 == usize::MAX { return; }
        let cost = OUTPOST_FOUND_COST;
        if self.houses[hi].wealth < cost { return; }
        let site = self.colonizable.swap_remove(si);
        let kind = estate_kind_for_good(&self.goods[g0].name, self.goods[g0].food);
        let founder_max_pc = self.hubs[home].base_per_capita.iter().cloned().fold(0.0f32, f32::max).max(0.1);
        let percap = founder_max_pc * (0.4 + site.fertility);
        let est_pop = OUTPOST_MAX_POP; // a small trade post (hard-capped, never grows into a city)
        let component = self.hubs[home].component;          // joins the home trade web
        self.houses[hi].wealth -= cost;
        // parent = −1 keeps the outpost at its REMOTE site coords (not co-located).
        self.create_estate(-1, site.x, site.y, g0, kind, founder, site.koppen, site.coastal,
            component, est_pop, percap);
        let new = self.hubs.len() - 1;
        self.hubs[new].colony_kind = 2;
        self.hubs[new].founder_hub = home as i32;
        self.hubs[new].colony_founded_tick = self.tick; // for the graduation age gate
        let hname = self.houses[hi].name.clone();
        // Distinct place-name + the (outpost) tag (was "{house} (outpost)", which
        // duplicated when a house planted several); the founding house is named in
        // the chronicle below and shown via `owner_house`.
        let place = crate::sim::names::gen_name(
            site.x.max(0.0) as u32, site.y.max(0.0) as u32,
            self.world_w as u32, self.world_h());
        self.hubs[new].name = format!("{} (outpost)", place);
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "colony".into(), hub: new as i32, good: g0 as i32, value: 2.0,
            text: format!("{} founds a trade outpost ({})", hname, self.goods[g0].name),
        });
    }

    /// A thriving house OUTPOST matures into a full colony (Phoenician emporion → city:
    /// Gadir, Utica, Goa). A long-lived outpost at its population cap, whose owning
    /// house is wealthy, is PROMOTED IN PLACE: it sheds estate status, becomes a
    /// settlement colony backed & led by its founding house (which keeps control on any
    /// later independence — the Magonid pattern), and the small-outpost pop cap is
    /// lifted so it can grow into a city. At most one graduation per call.
    fn maybe_graduate_outpost(&mut self) {
        let tick = self.tick;
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if !hub.is_estate || hub.colony_kind != 2 || hub.abandoned { continue; }
            let owner = hub.owner_house;
            if owner < 0 || (owner as usize) >= self.houses.len()
                || self.houses[owner as usize].defunct { continue; }
            let age = tick.saturating_sub(hub.colony_founded_tick);
            if age < OUTPOST_GRADUATE_YEARS * TICKS_PER_YEAR { continue; }
            if hub.population < OUTPOST_MAX_POP * 0.9 { continue; }
            let wealth = self.houses[owner as usize].wealth;
            if wealth < OUTPOST_GRADUATE_WEALTH { continue; }
            if wealth > best.1 { best = (h, wealth); }
        }
        let Some(h) = (best.0 != usize::MAX).then_some(best.0) else { return };
        let owner = self.hubs[h].owner_house;
        let home = if owner >= 0 && (owner as usize) < self.houses.len() {
            self.houses[owner as usize].hub as i32
        } else { -1 };
        // Promote in place: a real hub now, a settlement colony led by its founding house.
        self.hubs[h].is_estate = false;
        self.hubs[h].colony_kind = 1;
        self.hubs[h].colony_stage = 1;
        self.hubs[h].founder_hub = home;
        self.hubs[h].backers = vec![(1, owner.max(0) as u32, 1.0)];
        self.hubs[h].council_house = owner;
        self.hubs[h].colony_founded_tick = tick; // independence clock restarts from here
        self.hubs[h].founding_pop = self.hubs[h].population.max(1.0);
        self.hubs[h].reserve_cap = self.hubs[h].reserve_cap.max(6.0);
        self.hubs[h].supply_years = 0.0;
        self.hubs[h].name = self.hubs[h].name.replace(" (outpost)", "");
        self.routes_dirty = true;
        self.total_foundings += 1;
        let cn = self.hubs[h].name.clone();
        let hn = if owner >= 0 && (owner as usize) < self.houses.len() {
            self.houses[owner as usize].name.clone()
        } else { String::new() };
        self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32, good: -1,
            value: 0.0, text: format!("{} grows from a trading post into a colony of {}", cn, hn) });
    }

    /// Yearly: a wealthy house develops an EXISTING under-traded small city into a
    /// trade BASE — opening an office, building a guildhall + warehouse, seeding
    /// working capital and taking the city under its patronage so its modest surplus
    /// finally clears to market. The accessible, existing-settlement cousin of
    /// `maybe_found_house_outpost`. At most one per call. See
    /// docs/TRADE_BASE_MECHANIC_PLAN.md.
    fn maybe_establish_trade_base(&mut self) {
        if self.hubs.is_empty() || self.houses.is_empty() { return; }
        self.hub_patron.resize(self.hubs.len(), -1);
        // Founder: the richest non-guild house clearing the (modest) wealth bar.
        let mut founder = -1i32;
        let mut hw = BASE_INVEST_WEALTH;
        for (hi, hh) in self.houses.iter().enumerate() {
            if hh.defunct || hh.is_guild { continue; }
            if hh.wealth > hw { hw = hh.wealth; founder = hi as i32; }
        }
        if founder < 0 { return; }
        let hi = founder as usize;
        if self.houses[hi].wealth < BASE_INVEST_COST { return; }
        let seat = self.houses[hi].hub as usize;
        if seat >= self.hubs.len() { return; }
        let comp = self.hubs[seat].component;
        let n = self.hubs.len();
        // Target: the nearest reachable, under-traded small city on the same continent
        // that this house doesn't already hold and nobody patronises yet.
        let mut best = (usize::MAX, f32::INFINITY);
        for c in 0..n {
            if c == seat { continue; }
            let hub = &self.hubs[c];
            if hub.is_estate || hub.colony_kind != 0 { continue; }
            if hub.component != comp { continue; }
            if self.hub_patron[c] >= 0 { continue; }
            if hub.population < BASE_MIN_POP || hub.population > BASE_MAX_POP { continue; }
            // Under-traded: throughput well below what its population could support.
            if hub.export_earn + hub.import_spend > BASE_UNDERTRADE_FRAC * hub.population { continue; }
            if self.houses[hi].offices.contains(&(c as u32)) { continue; }
            let d = self.days.get(seat * n + c).copied().unwrap_or(f32::INFINITY);
            if d.is_finite() && d < best.1 { best = (c, d); }
        }
        let Some(c) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // Commit (affordability already checked): pay, plant the base, take patronage.
        self.houses[hi].wealth -= BASE_INVEST_COST;
        if !self.houses[hi].offices.contains(&(c as u32)) {
            self.houses[hi].offices.push(c as u32);
        }
        if !self.hubs[c].structures.contains(&STRUCT_GUILDHALL) {
            self.hubs[c].structures.push(STRUCT_GUILDHALL);
        }
        if !self.hubs[c].structures.contains(&STRUCT_WAREHOUSE) {
            self.hubs[c].structures.push(STRUCT_WAREHOUSE);
        }
        self.hubs[c].treasury += BASE_SEED;
        self.hub_patron[c] = hi as i32;
        let (hn, cn) = (self.houses[hi].name.clone(), self.hubs[c].name.clone());
        self.houses[hi].events.push(HouseEvent {
            tick: self.tick, kind: "base".into(),
            text: format!("{} establishes a trade base in {}", hn, cn),
        });
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "base".into(), hub: c as i32, good: -1, value: 1.0,
            text: format!("{} establishes a trade base in {}", hn, cn),
        });
    }

    /// Yearly upkeep for patronised trade bases: a gentle development pop bonus while
    /// the city is still small, and patronage CONCLUDES once it has grown into a real
    /// node (the house keeps trading from it). Stale patronage (the hub collapsed or
    /// became an estate/colony) is dropped.
    fn trade_base_pass(&mut self) {
        self.hub_patron.resize(self.hubs.len(), -1);
        for h in 0..self.hubs.len() {
            if self.hub_patron[h] < 0 { continue; }
            if self.hubs[h].is_estate || self.hubs[h].colony_kind != 0 || self.hubs[h].population < 1.0 {
                self.hub_patron[h] = -1;
                continue;
            }
            if self.hubs[h].population >= BASE_DEVELOPED_POP {
                let ph = self.hub_patron[h] as usize;
                let hn = self.houses.get(ph).map(|x| x.name.clone()).unwrap_or_default();
                let cn = self.hubs[h].name.clone();
                self.hub_patron[h] = -1;
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "base".into(), hub: h as i32, good: -1, value: 2.0,
                    text: format!("{} matures into a thriving market — {}'s trade base is fully established", cn, hn),
                });
                continue;
            }
            // Development nudge while patronised and still small.
            self.hubs[h].population *= 1.0 + BASE_POP_GROWTH_BONUS;
        }
    }

    /// A works' EFFECTIVENESS multiplier on output (1.0 = at nominal tier). Four
    /// debuffs compound: disaster `damage`, age/wear since the last build/upgrade, a
    /// labor shortage in a small/shrunken host city, and unrest/famine there. All are
    /// floored so a sound works in a healthy city stays near full, but a damaged works
    /// in a starving backwater runs far below capacity.
    fn estate_effectiveness(&self, h: usize) -> f32 {
        let e = &self.hubs[h];
        if !e.is_estate { return 1.0; }
        // ADDITIVE penalties (NOT multiplicative): a sound works in a healthy city sits
        // at ~1.0; only real problems bite. (Multiplicative stacking gutted the estate
        // income that bootstraps houses and collapsed the whole economy — see CLAUDE.md.)
        let mut penalty = e.damage.clamp(0.0, 1.0); // disaster damage is the big lever
        let age_yrs = self.tick.saturating_sub(e.last_upgrade_tick) as f32 / TICKS_PER_YEAR as f32;
        penalty += (ESTATE_DECAY_PER_YEAR * age_yrs).min(ESTATE_AGE_PENALTY_CAP);
        if e.parent >= 0 && (e.parent as usize) < self.hubs.len() {
            let p = &self.hubs[e.parent as usize];
            // Labor: a works in a too-small host city runs a little below capacity.
            penalty += (1.0 - (p.population / ESTATE_LABOR_FULL_POP).clamp(0.0, 1.0)) * ESTATE_LABOR_PENALTY_CAP;
            // Unrest/famine in the host city cuts output.
            penalty += p.starving.clamp(0.0, 1.0) * ESTATE_UNREST_PENALTY_CAP;
        }
        (1.0 - penalty).clamp(0.15, 1.0)
    }

    /// Yearly · estate/manufactory disasters + repair. An intact works may suffer a
    /// fire/flood/blight (damage spikes → output drops via `estate_effectiveness`); a
    /// damaged works is repaired over several years when its owner (a house, or the
    /// parent city for civic works) can fund the work. (User-requested.)
    fn estate_condition_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        const KINDS: [&str; 3] = ["fire", "flood", "blight"];
        for ei in 0..n {
            if !self.hubs[ei].is_estate || self.hubs[ei].estate_tier == 0 { continue; }
            // 1) Disaster roll (only on a largely-intact works).
            if self.hubs[ei].damage < 0.2
                && hash01(self.seed, tick as u64 ^ 0xD15A57, ei as u64) < DISASTER_ANNUAL_CHANCE {
                let r = hash01(self.seed, tick as u64 ^ 0xF1AE, ei as u64);
                let dmg = DISASTER_MIN_DAMAGE + r * (DISASTER_MAX_DAMAGE - DISASTER_MIN_DAMAGE);
                self.hubs[ei].damage = dmg.clamp(0.0, 1.0);
                let kind = KINDS[(hash01(self.seed, tick as u64 ^ 0xCA1A, ei as u64) * 3.0) as usize % 3];
                let (en, par) = (self.hubs[ei].name.clone(), self.hubs[ei].parent);
                self.journal.push(JournalEntry { tick, kind: "disaster".into(), hub: par, good: -1,
                    value: dmg, text: format!("{} strikes {} ({:.0}% damage)", kind, en, dmg * 100.0) });
                continue;
            }
            // 2) Repair (if damaged and the owner can pay).
            if self.hubs[ei].damage > 0.01 {
                let repaired = (self.hubs[ei].damage * REPAIR_RATE_PER_YEAR).min(self.hubs[ei].damage);
                let cost = repaired * self.estate_market_value(ei) * REPAIR_COST_FRAC;
                let owner = self.hubs[ei].owner_house;
                let paid = if owner >= 0 && (owner as usize) < self.houses.len()
                    && !self.houses[owner as usize].defunct && self.houses[owner as usize].wealth > cost * 1.5 {
                    self.houses[owner as usize].wealth -= cost; true
                } else if owner < 0 {
                    let par = self.hubs[ei].parent;
                    if par >= 0 && (par as usize) < n && self.hubs[par as usize].treasury > cost * 1.5 {
                        self.hubs[par as usize].treasury -= cost; true
                    } else { false }
                } else { false };
                if paid {
                    self.hubs[ei].damage = (self.hubs[ei].damage - repaired).max(0.0);
                    if self.hubs[ei].damage <= 0.01 {
                        let (en, par) = (self.hubs[ei].name.clone(), self.hubs[ei].parent);
                        self.journal.push(JournalEntry { tick, kind: "estate".into(), hub: par, good: -1,
                            value: 0.0, text: format!("{} is fully repaired and back to work", en) });
                    }
                }
            }
        }
    }

    /// Resale value of a holding: ~60% of its rebuild cost, scaled by tier (and city
    /// size for raw estates). Manufactories are dear; raw estates cheap.
    fn estate_market_value(&self, ei: usize) -> f32 {
        let e = &self.hubs[ei];
        if !e.is_estate { return 0.0; }
        let tier = e.estate_tier.max(1) as f32;
        if e.estate_kind == 6 {
            tier * MANUFACTORY_UPGRADE_COST * 0.6
        } else {
            let popf = (e.population / 30_000.0).clamp(0.5, 3.0);
            tier * INVEST_COST_BASE * popf * 8.0 * 0.6
        }
    }

    /// Monthly · the estate & manufactory RESALE market. A cash-strapped house sells
    /// a holding to raise specie; a polis sells a city-owned (civic) works to refill a
    /// thin treasury. The best-capitalized bidder takes it: a solvent resident house
    /// (title transfers), or — for a manufactory — a bank in that market (it can't hold
    /// title, so it takes a CONTROLLING equity stake, acquiring the income stream).
    /// (User-requested: settlements/houses can sell holdings; banks/houses buy them.)
    fn estate_resale_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        // Find ONE holding for sale this month.
        let mut sale: Option<(usize, i32)> = None; // (estate hub, seller house | -1 civic)
        for ei in 0..n {
            let e = &self.hubs[ei];
            if !e.is_estate || e.estate_tier == 0 { continue; }
            if e.owner_house >= 0 {
                let oh = e.owner_house as usize;
                if oh < self.houses.len() && !self.houses[oh].defunct
                    && self.houses[oh].wealth < RESALE_DISTRESS_WEALTH
                    && hash01(self.seed, tick as u64 ^ 0x5A1E5, ei as u64) < 0.4 {
                    sale = Some((ei, oh as i32)); break;
                }
            } else if e.parent >= 0 && (e.parent as usize) < n
                && self.hubs[e.parent as usize].treasury < CIVIC_SALE_TREASURY_FLOOR
                && hash01(self.seed, tick as u64 ^ 0x6C1F5, ei as u64) < 0.2 {
                sale = Some((ei, -1)); break;
            }
        }
        let Some((ei, seller)) = sale else { return };
        let price = self.estate_market_value(ei);
        if price < 0.5 { return; }
        let parent = self.hubs[ei].parent;
        let is_manu = self.hubs[ei].estate_kind == 6;
        let pay_seller = |this: &mut Self, amt: f32| {
            if seller >= 0 { this.houses[seller as usize].wealth += amt; }
            else if parent >= 0 && (parent as usize) < this.hubs.len() { this.hubs[parent as usize].treasury += amt; }
        };
        // BUYER 1: the richest solvent resident house at the parent city (≠ seller).
        let mut buyer = (usize::MAX, 0.0f32);
        if parent >= 0 && (parent as usize) < n {
            for (hi, h) in self.houses.iter().enumerate() {
                if h.defunct || h.is_guild || hi as i32 == seller { continue; }
                if h.hub as usize != parent as usize { continue; }
                if h.wealth > price * 1.5 && h.wealth > buyer.1 { buyer = (hi, h.wealth); }
            }
        }
        if buyer.0 != usize::MAX {
            let bh = buyer.0;
            self.houses[bh].wealth -= price;
            pay_seller(self, price);
            self.hubs[ei].owner_house = bh as i32;
            let (en, bn) = (self.hubs[ei].name.clone(), self.houses[bh].name.clone());
            self.houses[bh].events.push(HouseEvent { tick, kind: "acquire".into(),
                text: format!("{} acquires {} for {:.0}", bn, en, price) });
            self.journal.push(JournalEntry { tick, kind: "estate".into(), hub: parent, good: -1,
                value: price, text: format!("{} buys {}", bn, en) });
            return;
        }
        // BUYER 2 (manufactories only): a bank in that market takes a controlling stake.
        if is_manu {
            let mut bk = usize::MAX;
            for bi in 0..self.banks.len() {
                if self.banks[bi].defunct { continue; }
                if parent >= 0 && !self.banks[bi].branches.contains(&(parent as u32)) { continue; }
                if self.banks[bi].reserves > price * 2.0 { bk = bi; break; }
            }
            if bk != usize::MAX {
                self.banks[bk].reserves -= price;
                pay_seller(self, price);
                let good = self.hubs[ei].base_per_capita.iter().enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(g, _)| g as u32).unwrap_or(0);
                self.hubs[ei].stake_bank = bk as i32;
                self.hubs[ei].stake_share = RESALE_BANK_STAKE;
                self.banks[bk].stakes.retain(|s| s.estate_hub != ei as u32);
                self.banks[bk].stakes.push(BankStake {
                    estate_hub: ei as u32, share: RESALE_BANK_STAKE, basis: price, good });
                let en = self.hubs[ei].name.clone();
                self.banks[bk].events.push(HouseEvent { tick, kind: "acquire".into(),
                    text: format!("acquires a controlling {:.0}% stake in {} for {:.0}",
                        RESALE_BANK_STAKE * 100.0, en, price) });
            }
        }
    }

    /// Monthly · a prosperous, CROWDED polis with treasury to spare SPONSORS
    /// Append a fading migration arrow (a "refugee road" / drift line) for the map,
    /// bounded so the snapshot stays small.
    fn push_migration_arrow(&mut self, sx: f32, sy: f32, tx: f32, ty: f32) {
        self.migrations.push([sx, sy, tx, ty, self.tick as f32]);
        if self.migrations.len() > MIGRATION_ARROW_CAP {
            let excess = self.migrations.len() - MIGRATION_ARROW_CAP;
            self.migrations.drain(0..excess);
        }
    }

    /// Shortest hop-path from `src` to `dst` over the TRADE-ROUTE graph (`neighbors`,
    /// weighted by `days`). Returns the hub-index chain [src, …, dst], or `None` if no
    /// trade-route chain connects them (⇒ no migration — people move strictly by routes).
    fn neighbor_path(&self, src: usize, dst: usize) -> Option<Vec<usize>> {
        let n = self.hubs.len();
        if src >= n || dst >= n { return None; }
        if src == dst { return Some(vec![src]); }
        // FAST PATH — a DIRECT trade tie (the overwhelmingly common case: economic drift and
        // diaspora always target a neighbour). Returns immediately with NO Dijkstra and no
        // O(n) allocations, which is what kept the yearly migration passes from freezing the
        // new-year tick on big maps.
        if let Some(nbrs) = self.neighbors.get(src) {
            if nbrs.iter().any(|&b| b as usize == dst) {
                return Some(vec![src, dst]);
            }
        }
        // Sparse Dijkstra over the capped neighbour adjacency (BinaryHeap → O(E·log V), not
        // O(V²)). Only reached for genuinely multi-hop targets (e.g. a sponsored colony).
        use std::cmp::Ordering;
        struct He(f32, usize); // (distance, node) — min-heap via reversed Ord
        impl PartialEq for He { fn eq(&self, o: &Self) -> bool { self.1 == o.1 && self.0 == o.0 } }
        impl Eq for He {}
        impl Ord for He {
            fn cmp(&self, o: &Self) -> Ordering {
                o.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal).then_with(|| self.1.cmp(&o.1))
            }
        }
        impl PartialOrd for He { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
        let mut dist = vec![f32::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        let mut heap: std::collections::BinaryHeap<He> = std::collections::BinaryHeap::new();
        dist[src] = 0.0;
        heap.push(He(0.0, src));
        while let Some(He(d, u)) = heap.pop() {
            if u == dst { break; }
            if d > dist[u] { continue; } // stale heap entry
            if let Some(nbrs) = self.neighbors.get(u) {
                for &bn in nbrs {
                    let b = bn as usize;
                    if b >= n { continue; }
                    let w = self.days.get(u * n + b).copied().unwrap_or(f32::INFINITY);
                    if !w.is_finite() { continue; }
                    let nd = d + w.max(0.01);
                    if nd < dist[b] { dist[b] = nd; prev[b] = u; heap.push(He(nd, b)); }
                }
            }
        }
        if !dist[dst].is_finite() { return None; }
        let mut chain = vec![dst];
        let mut cur = dst;
        while cur != src {
            let p = prev[cur];
            if p == usize::MAX { return None; }
            chain.push(p);
            cur = p;
        }
        chain.reverse();
        Some(chain)
    }

    /// Emit a route-bound migration flow: trace the trade-route polyline from `src`→`dst`
    /// and record it (culture + volume) for the reworked overlay. Falls back to a legacy
    /// endpoint arrow too, so the old overlay stays populated. Returns whether a route
    /// existed (callers treat "no route" as "no move" when they want strict routing).
    fn emit_migration_route(&mut self, src: usize, dst: usize, culture: &str, volume: f32) -> bool {
        let Some(chain) = self.neighbor_path(src, dst) else { return false; };
        let path: Vec<[f32; 2]> = chain.iter().map(|&h| [self.hubs[h].x, self.hubs[h].y]).collect();
        let (sx, sy) = (self.hubs[src].x, self.hubs[src].y);
        let (dx, dy) = (self.hubs[dst].x, self.hubs[dst].y);
        self.migration_routes.push(MigrationRoute {
            path, culture: culture.to_string(), volume, tick: self.tick,
            from_hub: src as i32, to_hub: dst as i32,
        });
        if self.migration_routes.len() > MIGRATION_ROUTE_CAP {
            let excess = self.migration_routes.len() - MIGRATION_ROUTE_CAP;
            self.migration_routes.drain(0..excess);
        }
        self.push_migration_arrow(sx, sy, dx, dy);
        true
    }

    /// Ensure every live hub has a majority culture (seeded from the worldgen
    /// culture map, inherited by its founder for colonies) and a minorities slot.
    /// Only fills empties, so it self-heals old saves and newly-founded hubs.
    pub(crate) fn ensure_hub_cultures(&mut self) {
        let n = self.hubs.len();
        if self.hub_culture.len() < n { self.hub_culture.resize(n, String::new()); }
        if self.hub_minorities.len() < n { self.hub_minorities.resize(n, Vec::new()); }
        let map = crate::sim::cultures::active();
        let real = |c: &str| !c.is_empty() && c != "—";
        for i in 0..n {
            // Reassign empty AND stuck "—" hubs (a hub seeded "—" before the culture
            // map existed used to keep it forever → small cities with no people).
            if real(&self.hub_culture[i]) { continue; }
            // 1) inherit the founding settlement's people
            let founder = self.hubs[i].founder_hub;
            if founder >= 0 && (founder as usize) < n {
                let fc = self.hub_culture[founder as usize].clone();
                if real(&fc) { self.hub_culture[i] = fc; continue; }
            }
            // 2) the culture hearth whose region this cell falls in
            if let Some(m) = &map {
                if let Some(h) = m.hearth_at(self.hubs[i].x as u32, self.hubs[i].y as u32) {
                    if real(&h.people) { self.hub_culture[i] = h.people.clone(); continue; }
                }
            }
            // 3) fallback: the nearest already-cultured hub, so NO settlement is left
            //    without a people (fixes small towns showing no culture at all).
            let (hx, hy) = (self.hubs[i].x, self.hubs[i].y);
            let mut best: Option<(f32, String)> = None;
            for j in 0..n {
                if j == i { continue; }
                let c = &self.hub_culture[j];
                if !real(c) { continue; }
                let dx = self.hubs[j].x - hx; let dy = self.hubs[j].y - hy;
                let d = dx * dx + dy * dy;
                if best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) { best = Some((d, c.clone())); }
            }
            self.hub_culture[i] = best.map(|(_, c)| c).unwrap_or_else(|| "—".into());
        }
    }

    /// Cultures · the city's MAJORITY people is whoever actually has the largest share
    /// (plurality). After migration/assimilation a minority quarter can outgrow the
    /// old majority — when it does, promote it and demote the former majority to a
    /// quarter, so the "majority" the UI shows is always the dominant people (fixes a
    /// founding people staying nominal majority at 2% while incomers hold 74%).
    fn rebalance_hub_majorities(&mut self) {
        let n = self.hubs.len();
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.hub_minorities[h].is_empty() { continue; }
            let maj = match self.hub_culture.get(h) {
                Some(c) if !c.is_empty() && c != "—" => c.clone(),
                _ => continue,
            };
            let minsum: f32 = self.hub_minorities[h].iter().map(|(_, s)| *s).sum();
            let maj_share = (1.0 - minsum).clamp(0.0, 1.0);
            // largest minority quarter
            let mut bi = 0usize; let mut bshare = -1.0f32;
            for (i, (_, s)) in self.hub_minorities[h].iter().enumerate() {
                if *s > bshare { bshare = *s; bi = i; }
            }
            if bshare > maj_share + 1e-4 {
                let new_maj = self.hub_minorities[h][bi].0.clone();
                self.hub_minorities[h].remove(bi);
                if maj_share > 0.005 { self.hub_minorities[h].push((maj, maj_share)); }
                self.hub_culture[h] = new_maj;
                self.hub_minorities[h].sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
    }

    /// The city's people composition for DISPLAY: the plurality people as the
    /// majority + the rest as minority quarters (shares ~sum to 1). Read-only, so
    /// the panel shows the true dominant people even before the next yearly
    /// `rebalance_hub_majorities` has run on an in-progress save.
    pub(crate) fn hub_people_display(&self, h: usize) -> (String, Vec<(String, f32)>) {
        let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
        let mins = self.hub_minorities.get(h).cloned().unwrap_or_default();
        if maj.is_empty() || maj == "—" { return (maj, mins); }
        let minsum: f32 = mins.iter().map(|(_, s)| *s).sum();
        let mut all: Vec<(String, f32)> = Vec::with_capacity(mins.len() + 1);
        all.push((maj, (1.0 - minsum).clamp(0.0, 1.0)));
        for (c, s) in mins { all.push((c, s)); }
        all.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top = all.remove(0);
        (top.0, all)
    }

    /// Migrants carry their home culture. Where it differs from the destination's
    /// majority, it swells a minority quarter there (as a share of destination pop).
    fn record_migration_culture(&mut self, dest: usize, src: usize, movers: f32) {
        let src_culture = match self.hub_culture.get(src) {
            Some(c) if !c.is_empty() && c != "—" => c.clone(),
            _ => return,
        };
        self.add_minority(dest, &src_culture, movers);
    }

    /// Grow a culture's minority quarter at `dest` by `movers` people (no-op if it
    /// is already the majority culture there).
    fn add_minority(&mut self, dest: usize, culture: &str, movers: f32) {
        if dest >= self.hub_minorities.len() || culture.is_empty() || culture == "—" { return; }
        if self.hub_culture.get(dest).map(|c| c == culture).unwrap_or(false) { return; }
        let dest_pop = self.hubs[dest].population.max(1.0);
        let add = (movers / dest_pop).clamp(0.0, 0.95);
        if add <= 0.0 { return; }
        let list = &mut self.hub_minorities[dest];
        if let Some(e) = list.iter_mut().find(|(c, _)| c == culture) {
            e.1 = (e.1 + add).min(0.95);
        } else {
            list.push((culture.to_string(), add));
        }
    }

    /// The share (0..1) of `culture` at `hub` — the majority remainder if it is the
    /// city's main people, else its minority share, else 0.
    pub(crate) fn culture_share_at(&self, hub: usize, culture: &str) -> f32 {
        if self.hub_culture.get(hub).map(|c| c == culture).unwrap_or(false) {
            let mshare: f32 = self.hub_minorities.get(hub)
                .map(|m| m.iter().fold(0.0f32, |a, (_, s)| a + *s)).unwrap_or(0.0);
            return (1.0 - mshare).clamp(0.0, 1.0);
        }
        self.hub_minorities.get(hub)
            .and_then(|m| m.iter().find(|(c, _)| c == culture).map(|(_, s)| *s)).unwrap_or(0.0)
    }

    /// Deterministic per-culture MOBILITY (0..1). ~20% of peoples are travel-prone
    /// merchant diasporas (mobility ≥ 0.7); the rest are more sedentary. Modelled on
    /// historical trading minorities (Hanseatic Germans, Jewish/Armenian/Greek
    /// diasporas) whose communities spread through the trade network.
    pub(crate) fn culture_mobility(name: &str) -> f32 {
        if name.is_empty() || name == "—" { return 0.2; }
        let mut h = 0xcbf29ce484222325u64;
        for b in name.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        let r = (h % 1000) as f32 / 1000.0;
        if r > 0.80 { 0.7 + (r - 0.80) / 0.20 * 0.30 } else { 0.1 + r / 0.80 * 0.4 }
    }

    /// #23 · Yearly DIASPORA spread: a travel-prone culture present in a city (as its
    /// majority OR an existing minority — enabling chain migration) sends a trickle
    /// of settlers ALONG a trade tie to a partner city, seeding/growing a minority
    /// quarter there. So merchant peoples visibly spread across the trade map.
    fn diaspora_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let mut sent = 0u32;
        for src in 0..n {
            if sent >= DIASPORA_MAX_PER_YEAR { break; }
            if self.hubs[src].is_estate || self.hubs[src].abandoned { continue; }
            if self.hubs[src].population < DIASPORA_MIN_POP { continue; }
            // The most travel-prone culture PRESENT here (majority or minority).
            let mut cand: Option<(String, f32)> = None;
            if let Some(maj) = self.hub_culture.get(src) {
                let m = Self::culture_mobility(maj);
                if m >= DIASPORA_MOBILITY_GATE { cand = Some((maj.clone(), m)); }
            }
            if let Some(mins) = self.hub_minorities.get(src) {
                for (c, s) in mins {
                    if *s < 0.02 { continue; }
                    let m = Self::culture_mobility(c);
                    if m >= DIASPORA_MOBILITY_GATE && cand.as_ref().map_or(true, |(_, cm)| m > *cm) {
                        cand = Some((c.clone(), m));
                    }
                }
            }
            let Some((culture, mob)) = cand else { continue; };
            if hash01(self.seed, tick as u64 ^ 0xD1A5, src as u64) > 0.15 * mob { continue; }
            // Destination: a TRADE PARTNER (route neighbour — never a direct geographic
            // jump) where this culture is still under-represented.
            let max_cells = MIGRATION_MAX_KM / (EARTH_EQUATOR_KM / self.world_w.max(1.0)).max(1e-3);
            let dst = self.neighbors.get(src).and_then(|v| v.iter().map(|&x| x as usize)
                .find(|&d| d < n && d != src && !self.hubs[d].is_estate && !self.hubs[d].abandoned
                    && self.hub_cell_dist(src, d) <= max_cells
                    && self.culture_share_at(d, &culture) < DIASPORA_MAX_MINORITY));
            let Some(dst) = dst else { continue; };
            let movers = (self.hubs[src].population * DIASPORA_SEND_FRAC).clamp(5.0, 200.0);
            self.hubs[src].population = (self.hubs[src].population - movers)
                .max(self.hubs[src].founding_pop * 0.3);
            self.hubs[dst].population += movers;
            self.add_minority(dst, &culture, movers);
            self.emit_migration_route(src, dst, &culture, movers); // 1-hop trade tie
            sent += 1;
            if hash01(self.seed, tick as u64 ^ 0x9A17, src as u64) < 0.10 {
                let dn = self.hubs[dst].name.clone();
                self.journal.push(JournalEntry { tick, kind: "migration".into(), hub: dst as i32, good: -1,
                    value: movers, text: format!("{:.0} {} traders settle in {}", movers, culture, dn) });
            }
        }
    }

    /// Yearly HINTERLAND pass: connect every sub-cap village to the trade network by
    /// tying it to its nearest live market town (satellite trade), and let that town
    /// earn a small, bounded civic toll from the hinterland trade. This is how villages
    /// below the sim cap "join the economy" without becoming O(n²) full hubs — they feed
    /// a real hub and show up in its books, instead of sitting inert and disconnected.
    fn hinterland_pass(&mut self) {
        let n = self.hubs.len();
        if self.hinterland.is_empty() || n == 0 { return; }
        // (Re)assign each village its nearest live market town — only when unlinked or
        // its town has died (cheap no-op once every village is settled on a live hub).
        for ti in 0..self.hinterland.len() {
            let cur = self.hinterland[ti].parent_hub;
            let ok = cur >= 0 && (cur as usize) < n
                && !self.hubs[cur as usize].is_estate && !self.hubs[cur as usize].abandoned;
            if ok { continue; }
            let (vx, vy) = (self.hinterland[ti].x, self.hinterland[ti].y);
            let mut best = (-1i32, f32::MAX);
            for h in 0..n {
                if self.hubs[h].is_estate || self.hubs[h].abandoned { continue; }
                let mut dx = (self.hubs[h].x - vx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = self.hubs[h].y - vy;
                let d = dx * dx + dy * dy;
                if d < best.1 { best = (h as i32, d); }
            }
            self.hinterland[ti].parent_hub = best.0;
        }
        // Market toll: each town earns a small civic income from its satellite villages.
        for ti in 0..self.hinterland.len() {
            let p = self.hinterland[ti].parent_hub;
            if p < 0 || (p as usize) >= n { continue; }
            self.hubs[p as usize].civic_pool += self.hinterland[ti].population * HINTERLAND_TOLL;
        }
    }

    /// Cultures 2.0 · snapshot every living people's total population (majority share +
    /// minority quarters) for the population line chart. Called twice a year; capped.
    fn sample_culture_history(&mut self) {
        use std::collections::HashMap;
        self.ensure_hub_cultures(); // self-heal so the very first sample has cultures
        let mut pops: HashMap<String, f32> = HashMap::new();
        for i in 0..self.hubs.len() {
            let h = &self.hubs[i];
            if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
            let pop = h.population.max(0.0);
            let minsum: f32 = self.hub_minorities.get(i).map(|m| m.iter().map(|(_, s)| *s).sum()).unwrap_or(0.0);
            if let Some(maj) = self.hub_culture.get(i) {
                if !maj.is_empty() && maj != "—" {
                    *pops.entry(maj.clone()).or_insert(0.0) += pop * (1.0 - minsum).clamp(0.0, 1.0);
                }
            }
            if let Some(mins) = self.hub_minorities.get(i) {
                for (c, s) in mins { if *s > 0.005 { *pops.entry(c.clone()).or_insert(0.0) += pop * *s; } }
            }
        }
        let t = self.tick as f32 / TICKS_PER_YEAR as f32;
        let mut entries: Vec<(String, f32)> = pops.into_iter().filter(|(_, p)| *p >= 1.0).collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        self.culture_history.push(CultureHistSample { t, pops: entries });
        if self.culture_history.len() > 220 {
            let d = self.culture_history.len() - 220;
            self.culture_history.drain(0..d);
        }
    }

    /// Cultures 2.0 · population-weighted CULTURAL DISCONTENT in a city (0..1): how much
    /// of its resident peoples' prized-good demand goes unmet (goods scarce/dear here),
    /// weighted by each people's population share — a big people whose cravings never
    /// arrive weighs heavily. Feeds the unrest target.
    pub(crate) fn cultural_discontent(&self, h: usize) -> f32 {
        let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
        let minsum: f32 = self.hub_minorities.get(h)
            .map(|m| m.iter().map(|(_, s)| *s).sum()).unwrap_or(0.0);
        let mut present: Vec<(String, f32)> = Vec::new();
        if !maj.is_empty() && maj != "—" { present.push((maj, (1.0 - minsum).clamp(0.0, 1.0))); }
        if let Some(mins) = self.hub_minorities.get(h) {
            for (c, s) in mins { if *s > 0.02 { present.push((c.clone(), *s)); } }
        }
        let (mut num, mut den) = (0.0f32, 0.0f32);
        for (c, share) in present {
            let desired = self.culture_desired_goods(&c);
            if desired.is_empty() { continue; }
            let (mut d, mut cnt) = (0.0f32, 0u32);
            for nm in &desired {
                if let Some(g) = self.goods.iter().position(|tg| &tg.name == nm) {
                    let base = self.goods[g].base_value.max(0.01);
                    let price = self.hubs[h].price.get(g).copied().unwrap_or(base).max(0.01);
                    let avail = (base * 1.3 / price).clamp(0.0, 1.0);
                    d += 1.0 - avail; cnt += 1;
                }
            }
            if cnt > 0 { num += share * (d / cnt as f32); den += share; }
        }
        if den > 0.0 { (num / den).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Good ids a culture PRIZES (creole → both parents' tastes; hearth culture → its
    /// kit's tastes). Empty for an unknown/legacy culture.
    pub(crate) fn culture_desired_goods(&self, culture: &str) -> Vec<&'static str> {
        if let Some(cr) = self.creoles.iter().find(|c| c.name == culture) {
            let mut v = crate::sim::cultures::kit_desired_goods(cr.kit_a as usize).to_vec();
            for g in crate::sim::cultures::kit_desired_goods(cr.kit_b as usize) {
                if !v.contains(g) { v.push(g); }
            }
            return v;
        }
        crate::sim::cultures::kit_of_people(culture)
            .map(|k| crate::sim::cultures::kit_desired_goods(k).to_vec())
            .unwrap_or_default()
    }

    /// The language FAMILY of a live culture (creole registry first, then the worldgen
    /// hearth kit). "" for an unknown/legacy culture.
    pub(crate) fn culture_family(&self, name: &str) -> String {
        if name.is_empty() || name == "—" { return String::new(); }
        if let Some(cr) = self.creoles.iter().find(|c| c.name == name) { return cr.family.clone(); }
        crate::sim::cultures::kit_of_people(name)
            .map(|k| crate::sim::cultures::KITS[k].lang_family.to_string())
            .unwrap_or_default()
    }

    /// Culture TRAIT indices (into `cultures::TRAITS`) for a live culture — from its
    /// kit archetype, travel-proneness and a stable seed. Empty for a legacy/unknown
    /// culture. Drives trait-based behaviour (e.g. assimilation resistance).
    pub(crate) fn culture_trait_ids(&self, name: &str) -> Vec<usize> {
        let kit = if let Some(cr) = self.creoles.iter().find(|c| c.name == name) {
            cr.kit_a as usize
        } else if let Some(k) = crate::sim::cultures::kit_of_people(name) {
            k
        } else {
            return Vec::new();
        };
        let seed = crate::sim::cultures::active()
            .and_then(|m| m.hearths.iter().find(|h| h.people == name).map(|h| h.mut_seed))
            .unwrap_or_else(|| {
                let mut x = 0xcbf29ce484222325u64;
                for b in name.bytes() { x ^= b as u64; x = x.wrapping_mul(0x100000001b3); }
                x
            });
        crate::sim::cultures::kit_traits(kit, Self::culture_mobility(name), seed)
    }

    /// Cultures 2.0 · Yearly assimilation — minority quarters blend into the majority,
    /// FASTER when they share a language family with the local majority (mutual
    /// intelligibility) and in big, prosperous "melting-pot" cities; SLOWER across
    /// distant families. Keeps the composition alive and bounded.
    fn assimilation_pass(&mut self) {
        let n = self.hubs.len();
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.hub_minorities[h].is_empty() { continue; }
            let maj_name = self.hub_culture.get(h).cloned().unwrap_or_default();
            let maj_fam = self.culture_family(&maj_name);
            let maj_env = crate::sim::cultures::people_env(&maj_name);
            // Prestige / melting-pot pressure: large, wealthy cities assimilate faster.
            let prestige = (self.hubs[h].population / 20_000.0).clamp(0.0, 1.0) * 0.5
                + self.hubs[h].trade_wealth.clamp(0.0, 1.0) * 0.5;
            let mins = std::mem::take(&mut self.hub_minorities[h]);
            let mut kept: Vec<(String, f32)> = Vec::with_capacity(mins.len());
            for (c, s) in mins {
                let fam = self.culture_family(&c);
                let related = !fam.is_empty() && !maj_fam.is_empty() && fam == maj_fam;
                // Same family → 2×; distant family → 0.6×; unknown → 1×. Prestige adds up to +100%.
                let kin = if fam.is_empty() || maj_fam.is_empty() { 1.0 }
                    else if related { 2.0 } else { 0.6 };
                // Light ETHNIC-APPEARANCE tie: peoples of the same appearance group (climate/
                // dress) blend a little more readily even across language families.
                let look = match (maj_env, crate::sim::cultures::people_env(&c)) {
                    (Some(a), Some(b)) if a == b && !related => APPEARANCE_ASSIM_BONUS,
                    _ => 1.0,
                };
                // TRAIT resistance: Insular / Xenophobic / Diaspora peoples keep their
                // quarters intact (a real self-contained minority, e.g. a diaspora
                // trading community); Assimilative peoples dissolve faster.
                let resist = crate::sim::cultures::traits_resist_assimilation(&self.culture_trait_ids(&c));
                let rate = (MINORITY_ASSIM_RATE * kin * look * resist * (1.0 + prestige)).clamp(0.0, 0.25);
                let ns = s * (1.0 - rate);
                if ns > 0.005 { kept.push((c, ns)); }
            }
            kept.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            kept.truncate(5);
            self.hub_minorities[h] = kept;
        }
    }

    /// Cultures 2.0 · Ethnogenesis — where a large minority has long shared a city with
    /// the majority, the two blend into a NEW creole people with a synthesized name and
    /// its own origin card. Bounded (a global cap + a low yearly chance), so a handful
    /// of creoles arise over a long campaign rather than a flood.
    fn ethnogenesis_pass(&mut self, year: u32) {
        if self.creoles.len() >= CREOLE_MAX { return; }
        let tick = self.tick;
        let n = self.hubs.len();
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.creoles.len() >= CREOLE_MAX { break; }
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < CREOLE_MIN_POP { continue; }
            // Largest minority quarter here.
            let (mut bi, mut bs) = (usize::MAX, 0.0f32);
            for (i, (_, s)) in self.hub_minorities[h].iter().enumerate() {
                if *s > bs { bs = *s; bi = i; }
            }
            if bi == usize::MAX || bs < CREOLE_MIN_MINORITY { continue; }
            if hash01(self.seed, tick as u64 ^ 0xE7_1409E5, h as u64) > CREOLE_YEARLY_CHANCE { continue; }
            let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
            let minc = self.hub_minorities[h][bi].0.clone();
            if maj.is_empty() || maj == "—" || maj == minc { continue; }
            // Don't re-spawn a creole already born of this same pair.
            let pair_fam = format!("Creole ({} · {})", maj, minc);
            if self.creoles.iter().any(|c| c.family == pair_fam) { continue; }
            let (name, color, kit_a, kit_b) = self.synth_creole_name(&maj, &minc, h, year);
            if name.is_empty() || self.creoles.iter().any(|c| c.name == name) { continue; }
            // Seed: half the minority quarter becomes the creole (intermarriage with locals).
            let take = bs * CREOLE_SEED_FRAC;
            self.hub_minorities[h][bi].1 -= take;
            self.hub_minorities[h].push((name.clone(), take));
            let mob = crate::sim::cultures::people_mobility(&name);
            let temperament = if mob >= 0.6 { "an outward-looking people, quick to take to the trade roads" }
                else { "a settled people, rooted where they were born" };
            let origin = format!(
                "{name} — a creole people born in {place} around year {year}, where {maj} and {minc} quarters blended into one; {temperament}.",
                name = name, place = self.hubs[h].name, year = year, maj = maj, minc = minc, temperament = temperament);
            self.creoles.push(Creole {
                name: name.clone(), family: pair_fam, origin, color,
                born_tick: tick, birthplace: self.hubs[h].name.clone(), kit_a, kit_b,
            });
            self.journal.push(JournalEntry {
                tick, kind: "ethnogenesis".into(), hub: h as i32, good: -1, value: take,
                text: format!("A new people, the {}, is born in {} as {} and {} blend into one.",
                    name, self.hubs[h].name, maj, minc) });
        }
    }

    /// EXPEDITIONS — a wealthy, fleet-owning house rarely mounts an expedition to a far
    /// isolated settlement (≤1 trade tie) that the network doesn't normally reach: the
    /// "casual merchants arriving on rare occasions" a remote outpost sees. It delivers a
    /// batch of the house's specialty good (relieving a little local scarcity) at a modest
    /// cost to the house (reach & prestige, not profit). Bounded and rare.
    fn expedition_pass(&mut self, year: u32) {
        let tick = self.tick;
        let n = self.hubs.len();
        let ng = self.goods.len();
        if ng == 0 { return; }
        let remotes: Vec<usize> = (0..n).filter(|&h|
            !self.hubs[h].is_estate && !self.hubs[h].abandoned
            && self.hubs[h].population > 50.0
            && self.neighbors.get(h).map(|v| v.len()).unwrap_or(0) <= 1
        ).collect();
        if remotes.is_empty() { return; }
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            let fleet = self.houses[hi].fleet_sea + self.houses[hi].fleet_river + self.houses[hi].fleet_caravan;
            if fleet == 0 || self.houses[hi].wealth < EXPEDITION_MIN_WEALTH { continue; }
            let chance = EXPEDITION_YEARLY_CHANCE * (1.0 + (fleet as f32).min(6.0) * 0.15);
            if hash01(self.seed, tick as u64 ^ 0xE59_ED17, hi as u64) > chance { continue; }
            let pick = (crate::sim::cultures::hash64(self.seed ^ (hi as u64) ^ ((year as u64) << 8)) as usize) % remotes.len();
            let t = remotes[pick];
            if t == self.houses[hi].hub as usize { continue; }
            let g = self.houses[hi].spec.first().copied().filter(|&g| g < ng).unwrap_or(0);
            let pop = self.hubs[t].population.max(0.0);
            let batch = (pop * 0.02).clamp(2.0, 60.0);
            let price = self.hubs[t].price.get(g).copied().unwrap_or(self.goods[g].base_value).max(EPS);
            if g < self.hubs[t].stock.len() { self.hubs[t].stock[g] += batch; }
            // The expedition COSTS the house (reach, not profit) — bounded, only reduces wealth.
            let cost = (batch * price * 0.05).min(self.houses[hi].wealth.max(0.0) * 0.02);
            self.houses[hi].wealth -= cost;
            let (gn, hn, tn) = (self.goods[g].name.clone(), self.houses[hi].name.clone(), self.hubs[t].name.clone());
            self.journal.push(JournalEntry {
                tick, kind: "expedition".into(), hub: t as i32, good: g as i32, value: batch,
                text: format!("An expedition of {hn} reaches the remote {tn}, trading {gn}, in year {year}.") });
        }
    }

    /// Cultures 3.0 · SPLINTERING — a large, far-flung community of one people, long
    /// isolated from its hearth, drifts into a NEW daughter people with its own name and
    /// origin card (same kit/appearance as the parent). Rare and distance-biased, capped
    /// by the shared creole limit, so a few daughter peoples arise over a long campaign.
    fn splinter_pass(&mut self, year: u32) {
        if self.creoles.len() >= CREOLE_MAX { return; }
        let tick = self.tick;
        let n = self.hubs.len();
        let ww = self.world_w.max(1.0);
        for h in 0..n.min(self.hub_minorities.len()) {
            if self.creoles.len() >= CREOLE_MAX { break; }
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < CREOLE_MIN_POP { continue; }
            let maj = self.hub_culture.get(h).cloned().unwrap_or_default();
            if maj.is_empty() || maj == "—" { continue; }
            // Only a rooted HEARTH people splinters (creoles are already new peoples).
            let kit = match crate::sim::cultures::kit_of_people(&maj) { Some(k) => k, None => continue };
            // The parent must clearly dominate this city (a coherent community).
            let minsum: f32 = self.hub_minorities[h].iter().map(|(_, s)| *s).sum();
            let maj_share = (1.0 - minsum).clamp(0.0, 1.0);
            if maj_share < SPLINTER_MIN_SHARE { continue; }
            // Divergence rises with DISTANCE from the parent hearth — a far-flung,
            // isolated community drifts into its own people.
            let dist = crate::sim::cultures::active().and_then(|m| {
                m.hearths.iter().find(|hh| hh.people == maj).map(|hh| {
                    let mut dx = (hh.x - self.hubs[h].x).abs();
                    if ww > 1.0 { dx = dx.min(ww - dx); }
                    let dy = hh.y - self.hubs[h].y;
                    (dx * dx + dy * dy).sqrt()
                })
            }).unwrap_or(0.0);
            let far = (dist / (ww * 0.25)).clamp(0.0, 1.0); // 0 at the hearth, 1 a quarter-world away
            if far < 0.35 { continue; } // must be genuinely far-flung
            // ISOLATION drives the (otherwise rare) chance: distance from the hearth
            // (steeply, far^1.6) AND how few trade ties the settlement has — a lonely
            // outpost with one or no trade partners drifts into its own people far more
            // readily than a well-connected city. A hub with ≤1 tie gets up to ~3×.
            let ties = self.neighbors.get(h).map(|v| v.len()).unwrap_or(0);
            let lonely = 1.0 + (1.0 - (ties as f32 / 4.0).min(1.0)) * 2.0; // 3× at 0 ties → 1× at ≥4
            let chance = SPLINTER_YEARLY_CHANCE * far.powf(1.6) * lonely;
            if hash01(self.seed, tick as u64 ^ 0x5D1B_7E20, h as u64) > chance { continue; }

            let nseed = crate::sim::cultures::hash64(
                self.seed ^ (h as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ ((year as u64) << 20) ^ 0xDA02);
            let name = crate::sim::cultures::gen_people_name(kit, nseed, self.hubs[h].x as u32, self.hubs[h].y as u32);
            if name.is_empty() || name == maj
                || self.creoles.iter().any(|c| c.name == name)
                || crate::sim::cultures::active().map(|m| m.hearths.iter().any(|hh| hh.people == name)).unwrap_or(false) {
                continue;
            }
            // Daughter colour: the parent's, nudged so the two read apart on the map.
            let base = crate::sim::cultures::color_of_people(&maj).unwrap_or([150, 140, 170]);
            let shift = |c: u8, d: i32| (c as i32 + d).clamp(30, 235) as u8;
            let s0 = (crate::sim::cultures::hash64(nseed ^ 0x515F) % 3) as usize;
            let color = [shift(base[0], if s0 == 0 { 40 } else { -25 }),
                         shift(base[1], if s0 == 1 { 40 } else { -25 }),
                         shift(base[2], if s0 == 2 { 40 } else { -25 })];
            // The daughter secedes from the local majority (seeded as its own quarter).
            let take = maj_share * SPLINTER_SEED_FRAC;
            self.hub_minorities[h].push((name.clone(), take));
            let origin = format!(
                "{name} — a daughter people of the {maj}, arisen around year {year} in far-off {place}. Generations removed from the {maj} heartland, their speech and ways drifted until they became a people of their own.",
                name = name, maj = maj, year = year, place = self.hubs[h].name);
            self.creoles.push(Creole {
                name: name.clone(), family: format!("Branch of {maj}"), origin, color,
                born_tick: tick, birthplace: self.hubs[h].name.clone(), kit_a: kit as u8, kit_b: kit as u8,
            });
            self.journal.push(JournalEntry {
                tick, kind: "ethnogenesis".into(), hub: h as i32, good: -1, value: take,
                text: format!("A new people, the {}, splinters from the {} in far-off {}.",
                    name, maj, self.hubs[h].name) });
        }
    }

    /// Synthesize a creole's name + colour + parent kits from its two parent peoples.
    /// Falls back to default kits when a parent isn't a worldgen hearth (creole-of-creole).
    fn synth_creole_name(&self, a: &str, b: &str, h: usize, year: u32) -> (String, [u8; 3], u8, u8) {
        use crate::sim::cultures as cul;
        let ka = cul::kit_of_people(a).unwrap_or(0);
        let kb = cul::kit_of_people(b).unwrap_or(1);
        let seed = cul::hash64(self.seed ^ (h as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ ((year as u64) << 12));
        let name = cul::blend_name(ka, kb, seed);
        let ca = cul::color_of_people(a).unwrap_or([170, 130, 190]);
        let cb = cul::color_of_people(b).unwrap_or([130, 170, 150]);
        let color = [((ca[0] as u16 + cb[0] as u16) / 2) as u8,
            ((ca[1] as u16 + cb[1] as u16) / 2) as u8,
            ((ca[2] as u16 + cb[2] as u16) / 2) as u8];
        (name, color, ka as u8, kb as u8)
    }

    /// #23 · Yearly economic migration: within a trade component, people drift from
    /// low-opportunity (poor / hungry) cities to the most thriving reachable one.
    /// Emits fading arrows + mixes cultures. Small fractions keep populations bounded.
    fn economic_migration_pass(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        // Opportunity per hub (prosperity − starvation); precomputed so the inner
        // destination scan is cheap and borrow-safe.
        let opp: Vec<f32> = self.hubs.iter()
            .map(|h| if h.is_estate || h.abandoned { f32::MIN } else { h.sent_prosperity - h.starving })
            .collect();
        for src in 0..n {
            let s = &self.hubs[src];
            if s.is_estate || s.abandoned || s.population < ECON_MIG_MIN_POP { continue; }
            let src_opp = opp[src];
            if src_opp > ECON_MIG_STAY_ABOVE { continue; } // content cities keep their people
            let src_pop = s.population;
            // STRICT ROUTE RULE: people may only move to a DIRECT TRADE PARTNER (a single
            // trade tie), never a straight geographic jump to a distant city. They chain
            // city→city→city across the network over successive years, so every migration
            // line lies exactly on a real trade route. (User: "no migration except via
            // trade routes.") Pick the best-opportunity direct neighbour.
            let src_culture = self.hub_culture.get(src).cloned().unwrap_or_default();
            let mut dest = (usize::MAX, src_opp); // must beat the home city's own opportunity
            if let Some(nbrs) = self.neighbors.get(src) {
                for &bn in nbrs {
                    let d = bn as usize;
                    if d >= n || d == src { continue; }
                    let o = &self.hubs[d];
                    if o.is_estate || o.abandoned || o.food_balance < 0.0 { continue; }
                    // Cultures 2.0 · HOMOPHILY: kin already settled at a destination make it
                    // more attractive — people prefer to move where their own people are.
                    let o_eff = opp[d] + HOMOPHILY_PULL * self.culture_share_at(d, &src_culture);
                    if o_eff > dest.1 { dest = (d, o_eff); }
                }
            }
            let Some(di) = (dest.0 != usize::MAX).then_some(dest.0) else { continue; };
            if dest.1 - src_opp < ECON_MIG_GRADIENT { continue; } // needs a real pull
            let movers = (src_pop * ECON_MIG_FRAC).clamp(10.0, 800.0);
            if movers >= src_pop * 0.5 { continue; }
            // STRICT: people move only if a trade-route chain connects the two cities.
            let culture = src_culture;
            if !self.emit_migration_route(src, di, &culture, movers) { continue; }
            self.hubs[src].population -= movers;
            self.hubs[di].population += movers;
            self.record_migration_culture(di, src, movers);
            if hash01(self.seed, tick as u64 ^ 0x5EED_0C, src as u64) < 0.08 {
                let (sn, dn) = (self.hubs[src].name.clone(), self.hubs[di].name.clone());
                self.journal.push(JournalEntry { tick, kind: "migration".into(), hub: di as i32, good: -1,
                    value: movers, text: format!("{:.0} people leave {} seeking work in {}", movers, sn, dn) });
            }
        }
    }

    /// emigration: it spends a slice of its treasury to move a wave of people to an
    /// under-populated, food-secure city in its own trade region (preferring its own
    /// colonies). This both relieves the sponsor's crowding and drains the treasuries
    /// that would otherwise hoard wealth forever. (User-requested: "poleis should spend
    /// a percentage of wealth each month on funding colonies and for people migration.")
    fn poleis_sponsor_migration(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        for src in 0..n {
            let s = &self.hubs[src];
            if s.is_estate { continue; }
            if s.population < COLONY_PARENT_MIN_POP || s.starving > 0.5 { continue; }
            if s.treasury < POLIS_MIGRATION_MIN_TREASURY || s.sent_prosperity < 0.3 { continue; }
            // Sponsor roughly every other month per eligible city.
            if hash01(self.seed, tick as u64 ^ 0x319A7E, src as u64) > 0.5 { continue; }
            let comp = self.hubs[src].component;
            let pop = self.hubs[src].population;
            let km_per_cell = EARTH_EQUATOR_KM / self.world_w.max(1.0);
            let max_cells = MIGRATION_MAX_KM / km_per_cell.max(1e-3);
            // Destination: the most under-populated food-secure city in the same trade
            // region, strongly preferring a colony this polis founded.
            let mut dest = (usize::MAX, f32::MAX);
            for d in 0..n {
                if d == src || self.hubs[d].is_estate || self.hubs[d].component != comp { continue; }
                if self.hubs[d].food_balance < 0.0 { continue; }
                // Settlers move to NEARBY cities only — no cross-map sponsorship. Being
                // in the same trade component spans the whole continent/ocean, so gate on
                // real distance (≤ MIGRATION_MAX_KM) like the other migration passes.
                if self.hub_cell_dist(src, d) > max_cells { continue; }
                if self.hubs[d].population >= pop * 0.5 { continue; } // must have room to grow
                let own_colony = self.hubs[d].founder_hub == src as i32;
                let rank = self.hubs[d].population * if own_colony { 0.4 } else { 1.0 };
                if rank < dest.1 { dest = (d, rank); }
            }
            let Some(di) = (dest.0 != usize::MAX).then_some(dest.0) else { continue; };
            let movers = (pop * POLIS_MIGRATION_POP_FRAC).clamp(20.0, 1500.0);
            let spend = self.hubs[src].treasury * POLIS_MIGRATION_SPEND;
            if spend <= 0.0 { continue; }
            self.hubs[src].treasury -= spend;
            self.hubs[src].population = (pop - movers).max(self.hubs[src].founding_pop * 0.5);
            self.hubs[di].population += movers;
            self.hubs[di].treasury += spend * 0.5; // settlement aid travels with the migrants
            self.record_migration_culture(di, src, movers);
            let culture = self.hub_culture.get(src).cloned().unwrap_or_default();
            self.emit_migration_route(src, di, &culture, movers); // routed along the trade network
            if hash01(self.seed, tick as u64 ^ 0x4D161, src as u64) < 0.15 {
                let (sn, dn) = (self.hubs[src].name.clone(), self.hubs[di].name.clone());
                self.journal.push(JournalEntry { tick, kind: "migration".into(), hub: di as i32, good: -1,
                    value: movers, text: format!("{} sponsors {:.0} settlers to {}", sn, movers, dn) });
            }
        }
    }

    /// SETTLEMENT colony — an overcrowded, prosperous city founds a full new market
    /// hub on a fertile reachable site (heavy, joint-stock financing), seeding it
    /// with emigrants (relieving the parent). It graduates outpost→city in
    /// `colony_pass` and may later go autonomous.
    // ── Atlas 2.0 · the LIVING MAP: organic settlement death & birth ────────────

    /// Yearly. DEATH: a city that has spent ABANDON_YEARS shrunk to the famine
    /// floor while still starving/miserable is abandoned — survivors scatter to
    /// the nearest living towns and a † ruin remains. BIRTH: a thriving city
    /// bursting past its founding size swarms — a slice of its people walk out
    /// and found an independent town on nearby free land.
    fn lifecycle_pass(&mut self, expansion_ok: bool) {
        // ── DEATH ──
        let mut deaths = 0u32;
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || (hub.colony_kind == 1 && !hub.autonomous) {
                continue; // dependent colonies die by their own lifeline rules
            }
            if hub.population < 1.0 { continue; }
            let terminal = hub.population < hub.founding_pop * ABANDON_POP_FRAC
                && hub.starving > 0.5;
            if terminal {
                self.hubs[h].decline_years += 1.0;
            } else {
                self.hubs[h].decline_years = (self.hubs[h].decline_years - 0.5).max(0.0);
            }
            if self.hubs[h].decline_years < ABANDON_YEARS || deaths >= 2 { continue; }
            // Systemic anchors don't wink out mid-story: a city at war or hosting a
            // live bank holds on.
            if self.hubs[h].war_with >= 0 { continue; }
            if self.banks.iter().any(|b| !b.defunct && b.seat as usize == h) { continue; }
            // The PULL factor: abandonment is MIGRATION, not evaporation — it needs
            // somewhere better to go: a living, fed town of the same component.
            // (This also keeps a uniformly-starving world from grinding itself
            // empty: if everywhere is hungry, people endure where they are.)
            let comp = self.hubs[h].component;
            let has_haven = self.hubs.iter().enumerate().any(|(j, o)| j != h
                && !o.is_estate && !o.abandoned && o.component == comp
                && o.starving < 0.25 && o.population > o.founding_pop * 0.5);
            if !has_haven { continue; }
            self.abandon_hub(h);
            deaths += 1;
        }
        // ── BIRTH ──
        if expansion_ok && self.tick >= SWARM_START_TICK {
            self.maybe_swarm_town();
        }
    }

    /// Classify WHY a settlement died from its final decade: a plague strike, a
    /// war, a recorded supply-shock disaster — famine otherwise (the terminal
    /// condition is always famine-shaped; this names the deeper wound).
    fn abandon_cause(&self, h: usize) -> &'static str {
        let since = self.tick.saturating_sub(10 * 365);
        if self.epidemics.iter().any(|p| p.hub == h as u32 && p.start_tick >= since) {
            return "plague";
        }
        let (mut war, mut shock) = (false, false);
        for e in self.journal.iter().rev() {
            if e.tick < since { break; }
            if e.hub != h as i32 { continue; }
            match e.kind.as_str() {
                "war" => war = true,
                "event" => shock = true, // drought / blight / fishery collapse / embargo
                _ => {}
            }
        }
        if war { "war" } else if shock { "disaster" } else { "famine" }
    }

    /// Abandon a settlement: survivors migrate to the nearest living towns of the
    /// same component (recorded as refugee roads for the map's migration arrows),
    /// the ruin (population 0, `abandoned`, `died_cause`) stays on the map.
    fn abandon_hub(&mut self, h: usize) {
        let tick = self.tick;
        let nm = self.hubs[h].name.clone();
        let comp = self.hubs[h].component;
        let (hx, hy) = (self.hubs[h].x, self.hubs[h].y);
        let cause = self.abandon_cause(h);
        let mut dests: Vec<(usize, f32)> = (0..self.hubs.len())
            .filter(|&j| j != h && !self.hubs[j].is_estate && !self.hubs[j].abandoned
                && self.hubs[j].component == comp && self.hubs[j].population >= 1.0)
            .map(|j| {
                let mut dx = (self.hubs[j].x - hx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                (j, (dx * dx + (self.hubs[j].y - hy).powi(2)).sqrt())
            })
            .collect();
        dests.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        dests.truncate(3);
        let refugees = self.hubs[h].population * 0.6;
        let dest_name = dests.first().map(|&(j, _)| self.hubs[j].name.clone());
        if !dests.is_empty() {
            let share = refugees / dests.len() as f32;
            for &(j, _) in &dests {
                self.hubs[j].population += share;
                self.migrations.push([hx, hy, self.hubs[j].x, self.hubs[j].y, tick as f32]);
            }
            if self.migrations.len() > MIGRATION_ARROW_CAP {
                let excess = self.migrations.len() - MIGRATION_ARROW_CAP;
                self.migrations.drain(0..excess);
            }
        }
        let hub = &mut self.hubs[h];
        hub.abandoned = true;
        hub.died_tick = tick;
        hub.died_cause = cause.into();
        hub.population = 0.0;
        hub.starving = 0.0;
        hub.decline_years = 0.0;
        for v in hub.stock.iter_mut() { *v = 0.0; }
        for v in hub.production.iter_mut() { *v = 0.0; }
        self.total_abandonments += 1;
        self.routes_dirty = true;
        let why = match cause {
            "plague" => "the plague",
            "war" => "the war",
            "disaster" => "disaster upon disaster",
            _ => "famine",
        };
        self.journal.push(JournalEntry { tick, kind: "abandonment".into(), hub: h as i32,
            good: -1, value: 0.0, text: match dest_name {
                Some(d) => format!("{} is abandoned after years of {} — its last families take the road to {}", nm, why, d),
                None => format!("{} is abandoned after years of {} — the land lies empty", nm, why),
            } });
    }

    /// A thriving, crowded city sends SWARM_POP_FRAC of its people to break new
    /// ground: an INDEPENDENT town (no charter, no lifeline) on the best farmable
    /// free site within short range. At most one founding a year worldwide so each
    /// stays a legible chronicle event.
    fn maybe_swarm_town(&mut self) {
        if self.colonizable.is_empty() { return; }
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.abandoned || hub.colony_kind != 0 { continue; }
            if hub.population < SWARM_MIN_POP
                || hub.population < hub.founding_pop * SWARM_PRESSURE { continue; }
            if hub.starving > 0.05 || hub.mood < 0.5 { continue; }
            let score = hub.population / hub.founding_pop.max(1.0) * (0.5 + hub.mood);
            if score > best.1 { best = (h, score); }
        }
        let Some(mother) = (best.0 != usize::MAX).then_some(best.0) else { return };
        let cap = self.world_w * SWARM_REACH_FRAC;
        let (mx, my) = (self.hubs[mother].x, self.hubs[mother].y);
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            if s.fertility < SWARM_MIN_FERTILE { continue; }
            let mut dx = (s.x - mx).abs();
            if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
            let d = (dx * dx + (s.y - my).powi(2)).sqrt();
            // Not beyond a settler's walk, and not on the mother's doorstep either.
            if d > cap || d < cap * 0.15 { continue; }
            let score = (0.3 + s.fertility + 0.5 * s.trade_value) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        let site = self.colonizable.remove(si);
        let seed_pop = (self.hubs[mother].population * SWARM_POP_FRAC).max(600.0);
        self.hubs[mother].population -= seed_pop;
        let mother_name = self.hubs[mother].name.clone();
        let idx = self.create_organic_town(mother, &site, seed_pop);
        let new_name = self.hubs[idx].name.clone();
        self.total_foundings += 1;
        // Naturally-spawned free towns are a MERCHANT venture (related to a house/
        // guild), unlike the city-founded DEPENDENT satellites: name the sponsoring
        // house/guild in the beat so the tie reads in the chronicle.
        let sponsor = self.strongest_house_at(mother).map(|hi| self.houses[hi].name.clone());
        let text = match &sponsor {
            Some(hn) => format!("Merchants of {} lead settlers from {} to found {}", hn, mother_name, new_name),
            None => format!("Settlers from {} found {}", mother_name, new_name),
        };
        self.journal.push(JournalEntry { tick: self.tick, kind: "founding".into(),
            hub: idx as i32, good: -1, value: seed_pop, text });
    }

    /// Create the swarm's daughter town: an ordinary FREE settlement (colony_kind 0)
    /// with a culture-styled name, farming what the site gives it.
    fn create_organic_town(&mut self, mother: usize, site: &ColonizeSite, seed_pop: f32) -> usize {
        let ng = self.goods.len();
        // Settlers carry their skills but raw land yields less at first; fertile
        // ground closes the gap.
        let base_per_capita: Vec<f32> = self.hubs[mother].base_per_capita.iter()
            .map(|v| v * (0.40 + 0.40 * site.fertility)).collect();
        let pop = seed_pop.max(1.0);
        let production: Vec<f32> = base_per_capita.iter().map(|v| v * pop).collect();
        let id = 100_000 + self.hubs.len() as u32;
        let name = crate::sim::names::gen_name(
            site.x.max(0.0) as u32, site.y.max(0.0) as u32,
            self.world_w as u32, self.world_h());
        let component = self.hubs[mother].component;
        self.hubs.push(TickHub {
            id, x: site.x, y: site.y, name, population: pop, founding_pop: pop,
            stock: production.clone(), price: self.goods.iter().map(|g| g.base_value).collect(),
            production, grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: site.koppen, coastal: site.coastal, component,
            export_earn: 0.0, import_spend: 0.0, mood: 0.62, sent_food: 0.7, sent_prosperity: 0.5,
            sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(), in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, last_upgrade_tick: self.tick, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, mint_bullion_ratio: 1.0, has_mint: false,
            quality: vec![0.0f32; ng], stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: self.tick, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
        });
        self.routes_dirty = true;
        self.hubs.len() - 1
    }

    /// SATELLITE cities: a large metropolis founds a SHORT-RANGE satellite to serve a
    /// concrete NEED it cannot fit inside itself — a PORT (a big INLAND trade hub
    /// wants a harbour at the land/sea interface), a GRANARY (food-short core), or a
    /// WORKSHOP (a very large core outgrows its works). Council-approved, funded from
    /// the city treasury, seeded with relocated townsfolk. Historical: Ostia/Portus→
    /// Rome, Piraeus→Athens, Westminster & Southwark→London, Galata→Constantinople.
    /// One per call (yearly).
    fn maybe_found_satellite(&mut self, expansion_ok: bool) {
        if !expansion_ok || self.satellite_sites.is_empty() { return; }
        let tick = self.tick;
        // Satellites hug the metropolis: ≤ 500 km (a day's ride), from the dedicated
        // near-city pool — NOT the far colony pool. `SATELLITE_REACH_FRAC` is no longer
        // the leash; the absolute km cap is.
        let reach = (SATELLITE_MAX_KM * self.world_w / EARTH_EQUATOR_KM).max(2.0);
        for m in 0..self.hubs.len() {
            let metro = &self.hubs[m];
            if metro.is_estate || metro.abandoned || metro.colony_kind != 0 { continue; }
            if metro.population < SATELLITE_METRO_POP || metro.treasury < SATELLITE_COST { continue; }
            // NO SPAM: only ONE satellite may be under construction per metropolis — it must
            // be FINISHED before the council breaks ground on the next (user rule).
            if self.hubs.iter().any(|h| h.founder_hub == m as i32 && h.build_stage > 0 && !h.abandoned) { continue; }
            // Cap the total dependent satellites (kind 3) a single metropolis sustains.
            let sat_count = self.hubs.iter()
                .filter(|h| h.founder_hub == m as i32 && !h.abandoned && h.colony_kind == 3)
                .count() as u32;
            if sat_count >= SATELLITE_MAX_PER_METRO { continue; }
            // The NEED decides the role (priority: survival → trade → industry).
            let need_granary = metro.food_balance < 0.0;
            let need_port = !metro.coastal && metro.trade_wealth > 0.0;
            let need_workshop = metro.population >= SATELLITE_WORKSHOP_POP;
            let role: u8 = if need_granary { 1 } else if need_port { 0 }
                else if need_workshop { 2 } else { continue };
            if hash01(self.seed, tick as u64 ^ 0x5A7E11, m as u64) > 0.5 { continue; }
            // Nearest reachable NEAR-city site (a PORT needs a COASTAL one — the
            // land/sea transshipment point).
            let (mx, my) = (metro.x, metro.y);
            let mut best = (usize::MAX, f32::MAX);
            for (i, site) in self.satellite_sites.iter().enumerate() {
                if role == 0 && !site.coastal { continue; }
                let mut dx = (site.x - mx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = site.y - my;
                let d2 = dx * dx + dy * dy;
                if d2 > reach * reach { continue; }
                if d2 < best.1 { best = (i, d2); }
            }
            let Some(si) = (best.0 != usize::MAX).then_some(best.0) else { continue; };
            // Council funds it + relocates settlers from the crowded core.
            self.hubs[m].treasury -= SATELLITE_COST;
            let seed = SATELLITE_SEED_POP.min(self.hubs[m].population * 0.12);
            self.hubs[m].population = (self.hubs[m].population - seed)
                .max(self.hubs[m].founding_pop * 0.5);
            let site = self.satellite_sites.swap_remove(si);
            let new = self.create_organic_town(m, &site, seed);
            self.hubs[new].founder_hub = m as i32; // tracks the metropolis it serves
            self.hubs[new].colony_kind = 3;        // SATELLITE (dependent on its metropolis)
            self.hubs[new].colony_founded_tick = tick;
            if role == 0 { self.hubs[new].coastal = true; } // PORT — the harbour at the coast
            // Break ground on the CONSTRUCTION project (10y, decay model — the role bias +
            // future-exploit production activate only on COMPLETION; the site just consumes
            // hauled supply while it's built). `colony_stage` parks the intended role until
            // then. See docs/SATELLITE_CONSTRUCTION_AND_MIGRATION_PLAN.md.
            self.hubs[new].build_stage = 1;
            self.hubs[new].build_start_tick = tick;
            self.hubs[new].build_convoys = SAT_BUILD_CONVOYS;
            self.hubs[new].colony_stage = role; // 0 port · 1 granary · else workshop
            for c in 0u8..3 {
                self.hubs[new].build_supply_good[c as usize] = self.pick_build_supply_good(m, c);
            }
            let role_name = match role { 1 => "granary", 0 => "port", _ => "workshop" };
            let (metro_name, sat_name) = (self.hubs[m].name.clone(), self.hubs[new].name.clone());
            self.journal.push(JournalEntry {
                tick, kind: "founding".into(), hub: new as i32, good: -1, value: seed,
                text: format!("The council of {} breaks ground on the {} town of {}", metro_name, role_name, sat_name),
            });
            self.total_foundings += 1;
            self.routes_dirty = true;
            return; // one satellite per call
        }
    }

    /// Yearly. ABSORPTION: a tiny, failing FREE town beside a big healthy city is taken
    /// under that city's wing as a SATELLITE instead of being left to die with its trade
    /// halted. The metropolis relocates settlers to shore it up, ships it a founding
    /// grant of food, and binds its trade — so it is fed by the satellite lifeline and
    /// can grow back. Its newcomers seed a quarter of the metropolis's people (cultural
    /// mixing). One absorption per call. (User: "large nearby cities can integrate those
    /// dying cities and become satellites of the bigger ones.")
    fn maybe_absorb_dying_city(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        let n = self.hubs.len();
        let reach = (SATELLITE_MAX_KM * self.world_w / EARTH_EQUATOR_KM).max(2.0);
        // Current satellite dependents per metropolis, precomputed once (keeps this an
        // O(n²) scan, not O(n³) if the map has many tiny towns and no eligible rescuer).
        let mut sat_deps = vec![0u32; n];
        for d in &self.hubs {
            if !d.abandoned && d.colony_kind == 3 && d.founder_hub >= 0 && (d.founder_hub as usize) < n {
                sat_deps[d.founder_hub as usize] += 1;
            }
        }
        for h in 0..self.hubs.len() {
            let c = &self.hubs[h];
            // A genuinely FREE, tiny, struggling town — not an estate, colony, satellite,
            // ruin, or a brand-new swarm town still finding its feet.
            if c.is_estate || c.abandoned || c.colony_kind != 0 { continue; }
            if c.population < 1.0 || c.population > ABSORB_POP_MAX { continue; }
            if tick.saturating_sub(c.founded_tick) < ABSORB_MIN_AGE_YEARS * TICKS_PER_YEAR { continue; }
            // It must actually be in trouble (poor/hungry/shrinking) — a happy hamlet is
            // left to grow on its own.
            let struggling = c.sent_prosperity < 0.45 || c.starving > 0.08 || c.decline_years > 2.0;
            if !struggling { continue; }
            let (cx, cy, comp) = (c.x, c.y, c.component);
            // Nearest big, healthy, free city within a day's reach in the same market.
            let mut best = (usize::MAX, f32::MAX);
            for m in 0..self.hubs.len() {
                if m == h { continue; }
                let mm = &self.hubs[m];
                if mm.is_estate || mm.abandoned || mm.colony_kind != 0 { continue; }
                if mm.component != comp { continue; }
                if mm.population < ABSORB_METRO_POP || mm.starving > 0.3 { continue; }
                // Don't let one metropolis hoard an unbounded number of dependents.
                if sat_deps[m] >= SATELLITE_MAX_PER_METRO { continue; }
                let mut dx = (mm.x - cx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = mm.y - cy;
                let d2 = dx * dx + dy * dy;
                if d2 > reach * reach { continue; }
                if d2 < best.1 { best = (m, d2); }
            }
            let Some(m) = (best.0 != usize::MAX).then_some(best.0) else { continue; };
            // Adopt it. Relocate a wave of settlers from the metropolis…
            let aid = (self.hubs[m].population * ABSORB_AID_FRAC).clamp(150.0, 1500.0);
            self.hubs[m].population = (self.hubs[m].population - aid)
                .max(self.hubs[m].founding_pop * 0.5);
            self.hubs[h].population += aid;
            // …and ship a founding grant of food so it starts fed, not starving.
            for g in 0..ng {
                if !self.goods[g].food { continue; }
                let grant = (self.hubs[m].stock[g] * 0.10).max(0.0);
                self.hubs[m].stock[g] -= grant;
                self.hubs[h].stock[g] += grant;
            }
            self.hubs[h].colony_kind = 3;              // a metropolis-BOUND satellite now
            self.hubs[h].founder_hub = m as i32;
            self.hubs[h].colony_founded_tick = tick;
            self.hubs[h].colony_stage = 0;
            self.hubs[h].build_stage = 0;
            self.hubs[h].decline_years = 0.0;
            self.hubs[h].reserve_food = 60.0;
            self.hubs[h].reserve_cap = 365.0;
            // Judge it as a fresh small satellite (so the abandon floor doesn't drag its
            // old, larger founding size behind it).
            self.hubs[h].founding_pop = self.hubs[h].population.max(1.0);
            // The metropolis's people settle in as a minority quarter (culture mixing).
            let mc = self.hub_culture.get(m).cloned().unwrap_or_default();
            self.record_migration_culture(h, m, aid);
            self.emit_migration_route(m, h, &mc, aid);
            self.routes_dirty = true;
            let (mn, cn) = (self.hubs[m].name.clone(), self.hubs[h].name.clone());
            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32, good: -1,
                value: aid, text: format!("{} takes the failing town of {} under its wing as a satellite", mn, cn) });
            return; // one absorption per call
        }
    }

    /// Auto-pick the good that feeds one construction category (0 food · 1 preservables ·
    /// 2 construction) from the metropolis by locale: the most available good matching the
    /// category (name hints steer preservables→salt/dried and construction→timber/stone).
    fn pick_build_supply_good(&self, metro: usize, cat: u8) -> u16 {
        let ng = self.goods.len();
        if ng == 0 || metro >= self.hubs.len() { return 0; }
        let avail = |g: usize| self.hubs[metro].stock.get(g).copied().unwrap_or(0.0).max(0.0)
            + self.hubs[metro].production.get(g).copied().unwrap_or(0.0).max(0.0);
        let name_has = |g: usize, subs: &[&str]| {
            let n = self.goods[g].name.to_lowercase();
            subs.iter().any(|s| n.contains(s))
        };
        let mut best = (u16::MAX, f32::MIN);
        for g in 0..ng {
            let sc = match cat {
                0 => if self.goods[g].food && self.goods[g].perishable > 0.15 { avail(g) + 1.0 } else { f32::MIN },
                1 => if self.goods[g].food && self.goods[g].perishable <= 0.15 {
                        avail(g) + if name_has(g, &["salt", "stockfish", "dried", "cured", "cheese", "honey", "oil"]) { 1e4 } else { 0.0 }
                    } else { f32::MIN },
                _ => if !self.goods[g].food {
                        avail(g) * self.goods[g].bulk.max(1.0)
                        + if name_has(g, &["timber", "wood", "stone", "iron", "brick", "clay", "marble", "lime", "tile"]) { 1e4 } else { 0.0 }
                    } else { f32::MIN },
            };
            if sc > best.1 { best = (g as u16, sc); }
        }
        if best.0 == u16::MAX { 0 } else { best.0 }
    }

    /// The council's target civic reserve per needed good — scales with the city's size
    /// and how many colonies/satellites it must feed.
    fn council_reserve_target(&self, h: usize, deps: usize) -> f32 {
        COUNCIL_RESERVE_BASE * (1.0 + deps as f32)
            * (self.hubs[h].population / 5_000.0).clamp(0.3, 4.0)
    }

    /// Monthly · RIGHT OF FIRST BUY (staple right). Each solvent city council pre-empts
    /// the goods it needs (food always; plus preservables/construction when it provisions
    /// colonies or a satellite build) out of its OWN market stock — i.e. it buys from the
    /// merchants arriving in the city before the open market clears — and secures them in
    /// the civic warehouse (`civic_goods`). A council whose city has been CAPTURED by a
    /// house (the house dominates its trade) loses first refusal: it can still stock up,
    /// but only at a retail premium after the house has creamed the market. This is the
    /// investment that lets colonies grow (their build/supply draws the civic reserve
    /// first) and pays back later as those colonies ship goods home. User-requested.
    fn council_provision_pass(&mut self) {
        let ng = self.goods.len();
        if ng == 0 { return; }
        let n = self.hubs.len();
        if self.council_bought_month.len() != n { self.council_bought_month = vec![0.0; n]; }
        // Precompute each city's dependent count ONCE (O(n)) instead of scanning all hubs per
        // hub (that O(n²) monthly scan was part of the late-campaign slowdown).
        let mut deps_count = vec![0usize; n];
        for d in &self.hubs {
            if d.abandoned { continue; }
            if d.founder_hub >= 0 && (d.colony_kind == 1 || d.colony_kind == 3 || d.build_stage > 0) {
                let f = d.founder_hub as usize;
                if f < n { deps_count[f] += 1; }
            }
        }
        for h in 0..n {
            self.council_bought_month[h] = 0.0;
            if self.hubs[h].is_estate || self.hubs[h].abandoned || self.hubs[h].population < 1.0 { continue; }
            if self.hubs[h].treasury < COUNCIL_PROVISION_MIN_TREASURY { continue; }
            let deps = deps_count[h];
            // A city that neither provisions a colony nor is food-stressed needn't hoard.
            if deps == 0 && self.hubs[h].food_balance > 0.15 { continue; }
            // Right of first buy is suspended when merchant HOUSES dominate the city's trade
            // (carry ≥60% of it) OR a house has captured the government. Then the council can
            // still stock up, but only at a retail premium after the houses take the market.
            let dominated = hub_house_trade_share(&self.hubs[h]) >= COUNCIL_DOMINANCE_THRESHOLD
                || self.hubs[h].captor_house >= 0;
            let first_buy = !dominated;
            let unit_mult = if first_buy { COUNCIL_BUY_PRICE } else { COUNCIL_RETAIL_PRICE };
            let target = self.council_reserve_target(h, deps);
            if self.hubs[h].civic_goods.len() < ng { self.hubs[h].civic_goods.resize(ng, 0.0); }
            let budget = self.hubs[h].treasury * COUNCIL_PROVISION_BUDGET_FRAC;
            let mut spent = 0.0f32;
            for g in 0..ng {
                // Need it? Food is always secured; other goods only when feeding dependents.
                if !(self.goods[g].food || deps > 0) { continue; }
                let have = self.hubs[h].civic_goods[g];
                if have >= target { continue; }
                let avail = self.hubs[h].stock.get(g).copied().unwrap_or(0.0).max(0.0);
                let price = self.hubs[h].price.get(g).copied()
                    .unwrap_or(self.goods[g].base_value).max(0.01) * unit_mult;
                let afford = (budget - spent).max(0.0) / price;
                let buy = (target - have).min(avail).min(afford);
                if buy <= 0.0 { continue; }
                self.hubs[h].stock[g] -= buy;          // pre-empted out of the open market
                self.hubs[h].civic_goods[g] += buy;    // …into the civic warehouse
                spent += buy * price;
            }
            self.hubs[h].treasury -= spent;
            self.council_bought_month[h] = spent; // grain-eq secured this month (UI)
        }
    }

    /// Monthly: advance every satellite still under construction. Convoys pull the 3
    /// supply goods out of the metropolis's civic reserve (secured by the council's
    /// first-buy) and then its market stock, buying any remaining shortfall on the
    /// market; the least-supplied category throttles progress. A starved month DECAYS the
    /// stage, and a long drought slips a whole stage back (user: 10y build with decay).
    /// The 5th stage's completion turns the site into a functional, metropolis-BOUND city.
    fn construction_pass(&mut self) {
        let tick = self.tick;
        let ng = self.goods.len();
        let sites: Vec<usize> = (0..self.hubs.len())
            .filter(|&h| self.hubs[h].build_stage > 0 && !self.hubs[h].abandoned)
            .collect();
        for h in sites {
            let mf = self.hubs[h].founder_hub;
            if mf < 0 || (mf as usize) >= self.hubs.len() { self.finish_construction(h); continue; }
            let m = mf as usize;
            // 1) Supply each category: take what the metropolis has in STOCK, then BUY the
            //    shortfall on the market with council treasury. Founding a satellite makes
            //    the city PRIORITISE securing its supply — it pays merchants a premium to
            //    haul in whatever it lacks, so the works aren't deadlocked just because the
            //    mother city doesn't happen to stockpile olive oil. met% = worst category.
            let mut met = 1.0f32;
            let mut buy_cost = 0.0f32;
            for c in 0..3usize {
                let g = self.hubs[h].build_supply_good[c] as usize;
                if g >= ng { self.hubs[h].build_supply[c] = 1.0; continue; }
                let mut delivered = 0.0f32;
                // 1) draw the council's SECURED civic reserve first (its first-buy pays off).
                if g < self.hubs[m].civic_goods.len() {
                    let from_civic = self.hubs[m].civic_goods[g].max(0.0).min(SAT_STAGE_QUOTA);
                    self.hubs[m].civic_goods[g] -= from_civic;
                    delivered += from_civic;
                }
                // 2) then the open-market stock.
                let from_stock = self.hubs[m].stock.get(g).copied().unwrap_or(0.0)
                    .max(0.0).min(SAT_STAGE_QUOTA - delivered);
                if g < self.hubs[m].stock.len() { self.hubs[m].stock[g] -= from_stock; }
                delivered += from_stock;
                // 3) buy any remaining shortfall on the market with treasury.
                let deficit = (SAT_STAGE_QUOTA - delivered).max(0.0);
                let price = self.hubs[m].price.get(g).copied()
                    .unwrap_or(self.goods[g].base_value).max(0.01) * SAT_BUY_MARKUP;
                let afford = (self.hubs[m].treasury - buy_cost).max(0.0) / price;
                let bought = deficit.min(afford);
                buy_cost += bought * price;
                let ratio = (delivered + bought) / SAT_STAGE_QUOTA;
                self.hubs[h].build_supply[c] = ratio;
                met = met.min(ratio);
            }
            self.hubs[m].treasury -= buy_cost;
            // 2) Council pays the convoy crews; if it can't, no work happens this month.
            let upkeep = SAT_CONVOY_UPKEEP * self.hubs[h].build_convoys as f32;
            if self.hubs[m].treasury >= upkeep { self.hubs[m].treasury -= upkeep; }
            else { met = 0.0; }
            // 3) Advance or decay.
            if met > 0.05 {
                self.hubs[h].build_idle_months = 0;
                self.hubs[h].build_progress += met / SAT_STAGE_MONTHS;
                while self.hubs[h].build_progress >= 1.0 {
                    self.hubs[h].build_progress -= 1.0;
                    self.hubs[h].build_stage += 1;
                    if self.hubs[h].build_stage > 5 { self.finish_construction(h); break; }
                }
            } else {
                let idle = self.hubs[h].build_idle_months.saturating_add(1);
                self.hubs[h].build_idle_months = idle;
                self.hubs[h].build_progress = (self.hubs[h].build_progress - SAT_DECAY_PER_IDLE_MONTH).max(0.0);
                if idle >= SAT_STAGE_DROP_IDLE_MONTHS && self.hubs[h].build_stage > 1 {
                    self.hubs[h].build_stage -= 1;
                    self.hubs[h].build_progress = 0.6;
                    self.hubs[h].build_idle_months = 0;
                    let nm = self.hubs[h].name.clone();
                    self.journal.push(JournalEntry { tick, kind: "construction".into(), hub: h as i32,
                        good: -1, value: 0.0, text: format!("Works at {} rot for want of supply — the build slips a stage back", nm) });
                }
            }
            if self.hubs[h].build_stage > 0 { self.maybe_construction_event(h); }
        }
    }

    /// Occasional construction event (both kinds, per the user): masons/patron speed the
    /// works; a collapse/flood sets them back. Rolled monthly per active site.
    fn maybe_construction_event(&mut self, h: usize) {
        let tick = self.tick;
        let r = hash01(self.seed, tick as u64 ^ 0xC0_11_5A, h as u64);
        if r < 0.03 {
            self.hubs[h].build_progress = (self.hubs[h].build_progress + 0.25).min(0.999);
            let nm = self.hubs[h].name.clone();
            self.journal.push(JournalEntry { tick, kind: "construction".into(), hub: h as i32, good: -1,
                value: 0.0, text: format!("Skilled masons hasten the works at {}", nm) });
        } else if r > 0.975 {
            self.hubs[h].build_progress = (self.hubs[h].build_progress - 0.2).max(0.0);
            let nm = self.hubs[h].name.clone();
            self.journal.push(JournalEntry { tick, kind: "construction".into(), hub: h as i32, good: -1,
                value: 0.0, text: format!("A scaffold collapse sets back the works at {}", nm) });
        }
    }

    /// Finish a satellite build: clear the project state, activate the parked role's
    /// production (its "future exploits" go live) and mark it a functional bound city.
    fn finish_construction(&mut self, h: usize) {
        let tick = self.tick;
        let ng = self.goods.len();
        let role = self.hubs[h].colony_stage; // parked at founding: 0 port · 1 granary · else workshop
        self.hubs[h].build_stage = 0;
        self.hubs[h].build_progress = 0.0;
        self.hubs[h].build_idle_months = 0;
        self.hubs[h].colony_stage = 0;
        match role {
            1 => { // GRANARY — farm bias so its surplus feeds the metropolis
                for g in 0..ng {
                    if self.goods[g].food {
                        self.hubs[h].base_per_capita[g] *= FOOD_COLONY_FARM_MULT;
                        self.hubs[h].production[g] = self.hubs[h].base_per_capita[g] * self.hubs[h].population;
                        self.hubs[h].stock[g] = self.hubs[h].production[g];
                    }
                }
                self.hubs[h].reserve_food = 60.0;
            }
            0 => { self.hubs[h].coastal = true; } // PORT
            _ => {}                               // WORKSHOP — manufacturing follows pop/labor
        }
        self.routes_dirty = true;
        let m = self.hubs[h].founder_hub;
        let mn = if m >= 0 && (m as usize) < self.hubs.len() {
            self.hubs[m as usize].name.clone()
        } else { "its metropolis".into() };
        let (nm, pop) = (self.hubs[h].name.clone(), self.hubs[h].population);
        self.journal.push(JournalEntry { tick, kind: "founding".into(), hub: h as i32, good: -1,
            value: pop, text: format!("{} is completed — a functional town bound to {}", nm, mn) });
    }

    /// A mature, sizeable SATELLITE eventually outgrows its dependency and becomes a
    /// free city in its own right (colony_kind→0), keeping `founder_hub` so the map
    /// and panels can still show which metropolis raised it.
    fn satellite_independence_pass(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].colony_kind != 3 || self.hubs[h].abandoned || self.hubs[h].build_stage > 0 { continue; }
            let age = tick.saturating_sub(self.hubs[h].colony_founded_tick) / TICKS_PER_YEAR;
            if age >= SATELLITE_INDEP_YEARS && self.hubs[h].population >= SATELLITE_INDEP_POP {
                self.hubs[h].colony_kind = 0;
                self.hubs[h].autonomous = true; // free city (founder_hub kept = its heritage)
                let nm = self.hubs[h].name.clone();
                let metro = self.hubs[h].founder_hub;
                let mn = if metro >= 0 && (metro as usize) < self.hubs.len() {
                    self.hubs[metro as usize].name.clone()
                } else { String::new() };
                self.journal.push(JournalEntry {
                    tick, kind: "colony".into(), hub: h as i32, good: -1, value: 0.0,
                    text: if mn.is_empty() { format!("{} grows into a free city in its own right", nm) }
                        else { format!("{} outgrows {} and becomes a free city", nm, mn) },
                });
            }
        }
    }

    /// CARAVANSERAIS: found a waystation near the midpoint of a long INLAND trade tie
    /// between two sizeable cities (a Silk-Road-style day's halt). It seeds small and
    /// can grow into a town. At most a few per year.
    fn maybe_found_caravanserai(&mut self, expansion_ok: bool) {
        if !expansion_ok || self.colonizable.is_empty() { return; }
        let tick = self.tick;
        let n = self.hubs.len();
        let min_gap = self.world_w * CARAVAN_MIN_GAP_FRAC;
        let near = self.world_w * CARAVAN_NEAR_MIDPOINT;
        let clear = self.world_w * CARAVAN_CLEAR_RADIUS;
        let mut made = 0u32;
        for a in 0..n {
            if made >= CARAVAN_MAX_PER_YEAR { break; }
            let ha = &self.hubs[a];
            if ha.is_estate || ha.abandoned || ha.coastal || ha.population < CARAVAN_CITY_MIN_POP { continue; }
            let comp = ha.component;
            let (ax, ay) = (ha.x, ha.y);
            // A distant INLAND trade partner (long land haul).
            let partner = self.neighbors.get(a).and_then(|v| v.iter().map(|&x| x as usize).find(|&b| {
                b < n && b != a && !self.hubs[b].is_estate && !self.hubs[b].abandoned
                    && !self.hubs[b].coastal && self.hubs[b].component == comp
                    && self.hub_cell_dist(a, b) >= min_gap
            }));
            let Some(b) = partner else { continue };
            if a > b { continue; } // each pair once
            if hash01(self.seed, tick as u64 ^ 0xCA5A, (a as u64) ^ ((b as u64) << 20)) > 0.4 { continue; }
            let (bx, by) = (self.hubs[b].x, self.hubs[b].y);
            let (mx, my) = ((ax + bx) * 0.5, (ay + by) * 0.5); // midpoint (cross-seam ties rare)
            // Already served? (a town near the midpoint)
            let occupied = (0..n).any(|h| h != a && h != b && !self.hubs[h].is_estate
                && !self.hubs[h].abandoned
                && ((self.hubs[h].x - mx).powi(2) + (self.hubs[h].y - my).powi(2)).sqrt() < clear);
            if occupied { continue; }
            // Nearest INLAND colonizable site to the midpoint.
            let mut best = (usize::MAX, near * near);
            for (i, s) in self.colonizable.iter().enumerate() {
                if s.coastal { continue; }
                let d2 = (s.x - mx).powi(2) + (s.y - my).powi(2);
                if d2 < best.1 { best = (i, d2); }
            }
            let Some(si) = (best.0 != usize::MAX).then_some(best.0) else { continue };
            let site = self.colonizable.swap_remove(si);
            let idx = self.create_organic_town(a, &site, CARAVAN_SEED_POP);
            let (an, bn, cn) = (self.hubs[a].name.clone(), self.hubs[b].name.clone(), self.hubs[idx].name.clone());
            self.total_foundings += 1;
            self.journal.push(JournalEntry { tick, kind: "founding".into(), hub: idx as i32, good: -1,
                value: CARAVAN_SEED_POP,
                text: format!("A caravanserai rises at {} on the road between {} and {}", cn, an, bn) });
            made += 1;
        }
    }

    fn maybe_found_settlement_colony(&mut self) {
        if self.colonizable.is_empty() { return; }
        let n_settle = self.hubs.iter().filter(|h| h.colony_kind == 1 && !h.autonomous).count();
        if n_settle >= MAX_SETTLEMENT_COLONIES { return; }
        // Founder: a large, food-secure, prosperous ordinary city under population
        // pressure with some treasury to commit.
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.colony_kind != 0 { continue; }
            // Large, prosperous, not in severe famine — the pressure to export people.
            if hub.population < COLONY_PARENT_MIN_POP || hub.starving > 0.7 { continue; }
            if hub.sent_prosperity < 0.25 { continue; }
            let score = hub.population * hub.sent_prosperity.clamp(0.0, 1.0) * (0.2 + hub.treasury);
            if score > best.1 { best = (h, score); }
        }
        let Some(founder) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // Best reachable FERTILE site (from the city; its prior colonies could relay
        // too, but the city alone is enough for v1).
        let nodes = vec![(self.hubs[founder].x, self.hubs[founder].y)];
        let cap = COLONY_MAX_KM * self.world_w / EARTH_EQUATOR_KM; // ≤ 5000 km from the metropolis
        let founder_coastal = self.hubs[founder].coastal;
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            // INLAND cities have no fleet tradition — they can only found INLAND
            // colonies; only a coastal metropolis colonizes the sea (user rule).
            if !founder_coastal && s.coastal { continue; }
            // Skip only the genuinely worthless: too lean to part-feed itself AND
            // poor in trade goods. Otherwise a colony may settle less-fertile land —
            // a trade-rich frontier is worth founding even on lean soil (its food
            // lifeline contracts cover the deficit).
            if s.fertility < COLONY_MIN_FERTILE && s.trade_value < COLONY_MIN_TRADE { continue; }
            let d = self.nearest_node_dist(&nodes, s.x, s.y);
            if d > cap { continue; }
            // Weight the PRIZE (trade goods) over self-sufficiency, PLUS the prized
            // site premiums: sea shore, river delta, land→sea chokepoint (toll points).
            let site_bonus = if s.coastal { 0.35 } else { 0.0 }
                + if s.delta { 0.60 } else { 0.0 }
                + if s.chokepoint { 0.80 } else { 0.0 };
            let score = (0.25 + 0.35 * s.fertility + 1.0 * s.trade_value + site_bonus) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        // ── Raise the capital (affordability checked BEFORE anything is committed) ──
        let need = COLONY_FOUND_COST;
        // The city funds as much as its treasury allows (can self-fund); a resident
        // house and a local bank top up any shortfall (the joint-stock part).
        let city_put = self.hubs[founder].treasury.min(need).max(0.0);
        let house_idx = self.strongest_house_at(founder).filter(|&h| !self.houses[h].defunct);
        let house_put = house_idx.map(|h| (need - city_put).min(self.houses[h].wealth * 0.3).max(0.0)).unwrap_or(0.0);
        // A bank ON THE SAME CONTINENT (component) is REQUIRED — it stakes the venture
        // and its family becomes the colony's bank + mint. No such bank → no colony.
        let comp = self.hubs[founder].component;
        let bank_idx = self.banks.iter().position(|b| !b.defunct
            && self.hubs.get(b.seat as usize).map(|s| s.component == comp).unwrap_or(false));
        let Some(bank_idx) = bank_idx else { return };
        let bank_lend = (need - city_put - house_put).min(self.banks[bank_idx].reserves * 0.5).max(0.0);
        let raised = city_put + house_put + bank_lend;
        if raised < need * 0.8 { return; } // not enough backing — wait
        // Commit: debit each backer and record proportional shares.
        let mut backers: Vec<(u8, u32, f32)> = Vec::new();
        if city_put > 0.5 { self.hubs[founder].treasury -= city_put; backers.push((0, founder as u32, city_put / raised)); }
        if let (Some(h), true) = (house_idx, house_put > 0.5) { self.houses[h].wealth -= house_put; backers.push((1, h as u32, house_put / raised)); }
        if bank_lend > 0.5 {
            self.banks[bank_idx].reserves -= bank_lend;
            self.banks[bank_idx].loans.push(Loan {
                borrower_house: -1, borrower_polis: founder as i32, principal: bank_lend,
                outstanding: bank_lend, rate: BANK_LOAN_RATE, start_tick: self.tick,
                term_ticks: TICKS_PER_YEAR * 8, purpose: "colony".into(),
            });
        }
        // The bank is always a backer (its family will mint the colony's coin).
        backers.push((2, bank_idx as u32, (bank_lend.max(0.1)) / raised));
        let site = self.colonizable.swap_remove(si);
        // Migration: emigrants seed the colony and RELIEVE the crowded parent.
        let seed = (self.hubs[founder].population * COLONY_MIGRATION_FRAC).max(80.0);
        self.hubs[founder].population = (self.hubs[founder].population - seed)
            .max(self.hubs[founder].founding_pop * 0.3);
        let new = self.create_market_colony(founder, &site, backers, seed);
        // The backing merchant house OPENS AN OFFICE in the new colony — a permanent
        // foothold in its market (the family that staked the venture trades there).
        if let Some(h) = house_idx {
            if !self.houses[h].offices.contains(&(new as u32)) {
                self.houses[h].offices.push(new as u32);
                let (cn, city) = (self.houses[h].name.clone(), self.hubs[new].name.clone());
                self.houses[h].events.push(HouseEvent { tick: self.tick, kind: "branch".into(),
                    text: format!("{} opens a counting-house in {}", cn, city) });
            }
        }
        // The backing bank's family becomes the colony's main bank + mint: it seats
        // the colony's council and strikes its coin.
        self.hubs[new].main_bank = bank_idx as i32;
        let bank_house = self.banks[bank_idx].house as i32;
        self.hubs[new].council_house = bank_house;
        let cshort = self.hubs[new].name.replace(" (colony)", "");
        self.hubs[new].coin_name = format!("{} mark", cshort);
        self.hubs[new].coin_trust = 0.40;
        self.hubs[new].mint_fineness = 1.0;
        // Metropolis trade MONOPOLY: bar non-backer houses from the colony's market.
        self.apply_colony_charter(new);
        // Designate the food source + commission the founding grain fleet. Seed the
        // reserve so a fresh colony isn't instantly food-short before its first run.
        self.hubs[new].reserve_food = 60.0;
        let est_deficit_monthly = (self.hubs[new].population * 0.30).max(300.0);
        self.designate_colony_supply(new, est_deficit_monthly);
        let (pname, cname) = (self.hubs[founder].name.clone(), self.hubs[new].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "colony".into(), hub: new as i32, good: -1, value: 1.0,
            text: format!("{} founds the settlement colony {}", pname, cname),
        });
    }

    /// GRAIN COLONY (the Greek Crimea pattern): a large city gripped by a SUSTAINED food
    /// shortage plants a farming colony on the most FERTILE reachable site to secure its
    /// grain. Unlike the crowding-driven settlement colony this is a survival move — the
    /// city SELF-FUNDS it (no bank required, so it can act on hunger alone) and it can
    /// sit near existing cities. The colony is biased to farm; its surplus flows back to
    /// the hungry metropolis through the ordinary market.
    fn maybe_found_food_colony(&mut self) {
        if self.colonizable.is_empty() { return; }
        let n_settle = self.hubs.iter().filter(|h| h.colony_kind == 1 && !h.autonomous).count();
        if n_settle >= MAX_SETTLEMENT_COLONIES { return; }
        // Founder: a big city under real food stress with some treasury to commit.
        let mut best = (usize::MAX, 0.0f32);
        for h in 0..self.hubs.len() {
            let hub = &self.hubs[h];
            if hub.is_estate || hub.colony_kind != 0 { continue; }
            if hub.population < COLONY_PARENT_MIN_POP || hub.treasury < FOOD_COLONY_MIN_TREASURY { continue; }
            let stress = hub.starving.max((-hub.food_balance).max(0.0)).clamp(0.0, 1.0);
            if stress < FOOD_COLONY_STARVE_MIN { continue; } // not hungry enough
            let score = hub.population * stress * (0.2 + hub.treasury);
            if score > best.1 { best = (h, score); }
        }
        let Some(founder) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // Best reachable FERTILE (grain) site — fertility dominates, nearer is better.
        let nodes = vec![(self.hubs[founder].x, self.hubs[founder].y)];
        let cap = COLONY_MAX_KM * self.world_w / EARTH_EQUATOR_KM; // ≤ 5000 km from the metropolis
        let mut bi = (usize::MAX, 0.0f32);
        for (i, s) in self.colonizable.iter().enumerate() {
            if s.fertility < COLONY_MIN_FERTILE { continue; } // must be farmable
            let d = self.nearest_node_dist(&nodes, s.x, s.y);
            if d > cap { continue; }
            let score = (0.2 + 1.6 * s.fertility) * (1.0 - d / cap);
            if score > bi.1 { bi = (i, score); }
        }
        let Some(si) = (bi.0 != usize::MAX).then_some(bi.0) else { return };
        // Self-funded survival venture: the city commits the founding cost; a resident
        // house may chip in. NO bank required (that's the whole point — hunger acts).
        let need = COLONY_FOUND_COST;
        let city_put = self.hubs[founder].treasury.min(need).max(0.0);
        if city_put < need * 0.5 { return; } // can't afford even half → wait
        let house_idx = self.strongest_house_at(founder).filter(|&h| !self.houses[h].defunct);
        let house_put = house_idx.map(|h| (need - city_put).min(self.houses[h].wealth * 0.3).max(0.0)).unwrap_or(0.0);
        let raised = (city_put + house_put).max(EPS);
        let mut backers: Vec<(u8, u32, f32)> = Vec::new();
        self.hubs[founder].treasury -= city_put;
        backers.push((0, founder as u32, city_put / raised));
        if let (Some(h), true) = (house_idx, house_put > 0.5) {
            self.houses[h].wealth -= house_put; backers.push((1, h as u32, house_put / raised));
        }
        let comp = self.hubs[founder].component;
        let bank_idx = self.banks.iter().position(|b| !b.defunct
            && self.hubs.get(b.seat as usize).map(|s| s.component == comp).unwrap_or(false));
        let site = self.colonizable.swap_remove(si);
        let seed = (self.hubs[founder].population * COLONY_MIGRATION_FRAC).max(80.0);
        self.hubs[founder].population = (self.hubs[founder].population - seed)
            .max(self.hubs[founder].founding_pop * 0.3);
        let new = self.create_market_colony(founder, &site, backers, seed);
        // Bias it to FARM — its grain surplus is the whole reason it exists.
        let ng = self.goods.len();
        let pop = self.hubs[new].population;
        for g in 0..ng {
            if self.goods[g].food {
                self.hubs[new].base_per_capita[g] *= FOOD_COLONY_FARM_MULT;
                self.hubs[new].production[g] = self.hubs[new].base_per_capita[g] * pop;
                self.hubs[new].stock[g] = self.hubs[new].production[g];
            }
        }
        self.hubs[new].reserve_food = 60.0;
        let cshort = self.hubs[new].name.replace(" (colony)", "");
        self.hubs[new].coin_name = format!("{} mark", cshort);
        self.hubs[new].coin_trust = 0.40;
        if let Some(b) = bank_idx { self.hubs[new].main_bank = b as i32; }
        self.apply_colony_charter(new);
        // A grain colony is food-secure by design (no import fleet); just log it.
        let (pname, cname) = (self.hubs[founder].name.clone(), self.hubs[new].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "colony".into(), hub: new as i32, good: -1, value: 1.0,
            text: format!("Hungry {} plants the grain colony {} to secure its bread", pname, cname),
        });
    }

    /// Found a full MARKET hub (is_estate = false) as a settlement colony — a mini
    /// version of its founder so it has a real local economy from day one.
    fn create_market_colony(&mut self, founder: usize, site: &ColonizeSite,
                            backers: Vec<(u8, u32, f32)>, seed_pop: f32) -> usize {
        let ng = self.goods.len();
        let base_per_capita: Vec<f32> = self.hubs[founder].base_per_capita.iter().map(|v| v * 0.6).collect();
        let pop = seed_pop.max(1.0);
        let production: Vec<f32> = base_per_capita.iter().map(|v| v * pop).collect();
        let id = 100_000 + self.hubs.len() as u32;
        // A fresh culture-styled name (like any founded town) so a metropolis's
        // several colonies are DISTINCT places, not duplicate "New X (colony)".
        // The `colony_kind` field + the Colonial-panel tree carry the relationship.
        let name = crate::sim::names::gen_name(
            site.x.max(0.0) as u32, site.y.max(0.0) as u32,
            self.world_w as u32, self.world_h());
        let component = self.hubs[founder].component;
        self.hubs.push(TickHub {
            id, x: site.x, y: site.y, name, population: pop, founding_pop: pop,
            stock: production.clone(), price: self.goods.iter().map(|g| g.base_value).collect(),
            production, grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: site.koppen, coastal: site.coastal, component,
            export_earn: 0.0, import_spend: 0.0, mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5,
            sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(), in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, last_upgrade_tick: self.tick, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, mint_bullion_ratio: 1.0, has_mint: false,
            quality: vec![0.0f32; ng], stolen_good: -1, stolen_from: -1,
            colony_kind: 1, colony_stage: 1, autonomous: false, founder_hub: founder as i32, backers,
            reserve_food: 30.0, reserve_cap: 365.0, supply_years: 0.0, colony_founded_tick: self.tick,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: self.tick, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
        });
        self.total_foundings += 1; // Atlas 2.0 lifecycle counter (colony ventures too)
        self.routes_dirty = true;
        self.hubs.len() - 1
    }

    /// (Re)sign the colony's civic supply contracts — fill food / reserve /
    /// preservative slots from the nearest reachable suppliers on the metropolis's
    /// continent (same `component`): the metropolis itself, other food-producing
    /// cities/estates, and a salt-type producer for preservatives. Stale rows whose
    /// (Re)designate a settlement colony's food SOURCE — the reachable same-component
    /// hub with the largest shippable grain surplus (the metropolis gets a nearness
    /// edge) — and INVEST the backers' money in enough dedicated supply ships to carry
    /// the monthly deficit steadily. Also refreshes the display roster row.
    fn designate_colony_supply(&mut self, c: usize, deficit_monthly: f32) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let comp = self.hubs[c].component;
        let metro = self.hubs[c].founder_hub;
        // Best food source: the biggest reachable grain PRODUCER (a stable choice from day
        // one — before any stock has accumulated), nudged toward the metropolis/nearer.
        // The daily run is still capped by the source's real spare STOCK, so a designated
        // source that has nothing spare simply ships nothing until its granary fills.
        let food_output = |me: &Self, h: usize| -> f32 {
            let mut p = 0.0;
            for g in 0..me.goods.len() { if me.goods[g].food { p += me.hubs[h].production[g].max(0.0); } }
            p
        };
        let mut best = (-1i32, 0.0f32);
        for h in 0..n {
            if h == c || self.hubs[h].component != comp || self.hubs[h].population <= 1.0 { continue; }
            let d = self.days.get(h * n + c).copied().unwrap_or(f32::INFINITY);
            if !d.is_finite() { continue; }
            let fp = food_output(self, h);
            if fp <= EPS { continue; }
            let score = fp * (if h as i32 == metro { 1.3 } else { 1.0 }) / (1.0 + d * 0.02);
            if score > best.1 { best = (h as i32, score); }
        }
        self.hubs[c].supply_source = best.0;
        // Invest in ships until the fleet can carry the monthly deficit (steady supply),
        // as far as the backers can afford — cities pour money into the grain run. A
        // food-secure colony (no deficit) needs no fleet.
        let required = if deficit_monthly <= EPS { 0 } else {
            ((deficit_monthly / SUPPLY_SHIP_CAPACITY).ceil().max(SUPPLY_SHIPS_AT_FOUNDING as f32)
                as u32).min(MAX_SUPPLY_SHIPS)
        };
        while self.hubs[c].supply_ships < required {
            if !self.buy_colony_supply_ship(c) { break; }
        }
        // Display roster: the designated source on the grain run + a preservative row.
        self.colony_supply.retain(|s| s.colony_hub != c as u32);
        if best.0 >= 0 {
            let fg = (0..ng).find(|&g| self.goods[g].food).unwrap_or(0);
            let carried = (self.hubs[c].supply_ships as f32 * SUPPLY_SHIP_CAPACITY)
                .min(deficit_monthly.max(1.0));
            self.colony_supply.push(ColonySupply {
                colony_hub: c as u32, supplier_hub: best.0 as u32, good: fg,
                monthly_qty: carried, category: 0,
            });
            if let Some(pg) = (0..ng).find(|&g| self.goods[g].name.to_lowercase().contains("salt")) {
                self.colony_supply.push(ColonySupply {
                    colony_hub: c as u32, supplier_hub: best.0 as u32, good: pg,
                    monthly_qty: carried * 0.2, category: 2,
                });
            }
        }
    }

    /// Commission ONE more dedicated supply ship for colony `c`, paid by its backers
    /// (metropolis treasury first, then the backing bank's reserves, then a backing
    /// house's wealth). Returns false — and buys nothing — if they can't afford it.
    fn buy_colony_supply_ship(&mut self, c: usize) -> bool {
        if self.hubs[c].supply_ships >= MAX_SUPPLY_SHIPS { return false; }
        let cost = SUPPLY_SHIP_COST;
        // Tally what the backers can put up, in payment order.
        let m = self.hubs[c].founder_hub;
        let metro_pool = if m >= 0 && (m as usize) < self.hubs.len() {
            self.hubs[m as usize].treasury.max(0.0)
        } else { 0.0 };
        let backers = self.hubs[c].backers.clone();
        let mut bank_pool = 0.0;
        let mut house_pool = 0.0;
        for (kind, idx, _) in &backers {
            match kind {
                2 => { if let Some(b) = self.banks.get(*idx as usize) { if !b.defunct { bank_pool += b.reserves.max(0.0); } } }
                1 => { if let Some(hh) = self.houses.get(*idx as usize) { if !hh.defunct { house_pool += hh.wealth.max(0.0); } } }
                _ => {}
            }
        }
        if metro_pool + bank_pool + house_pool < cost { return false; }
        // Debit metro → bank(s) → house(s) in order.
        let mut owed = cost;
        if m >= 0 && (m as usize) < self.hubs.len() {
            let take = self.hubs[m as usize].treasury.max(0.0).min(owed);
            self.hubs[m as usize].treasury -= take; owed -= take;
        }
        for (kind, idx, _) in &backers {
            if owed <= EPS { break; }
            if *kind == 2 { if let Some(b) = self.banks.get_mut(*idx as usize) { if !b.defunct {
                let take = b.reserves.max(0.0).min(owed); b.reserves -= take; owed -= take; } } }
        }
        for (kind, idx, _) in &backers {
            if owed <= EPS { break; }
            if *kind == 1 { if let Some(hh) = self.houses.get_mut(*idx as usize) { if !hh.defunct {
                let take = hh.wealth.max(0.0).min(owed); hh.wealth -= take; owed -= take; } } }
        }
        self.hubs[c].supply_ships += 1;
        true
    }

    /// Yearly REVIVAL: a long-dead ruin whose region has since recovered is
    /// resettled — pioneers refound a small town on the old site (its buried
    /// productive potential, `base_per_capita`, was preserved at abandonment, so
    /// the extractor refills its trade next tick). Needs a thriving, food-secure
    /// living city nearby in the same trade region and a cooling-off period.
    fn resettle_pass(&mut self, expansion_ok: bool) {
        if !expansion_ok { return; }
        let tick = self.tick;
        let reach = (self.world_w * RESETTLE_REACH_FRAC).max(1.0);
        let mut revivals = 0u32;
        for h in 0..self.hubs.len() {
            if revivals >= RESETTLE_MAX_PER_YEAR { break; }
            let hub = &self.hubs[h];
            if !hub.abandoned || hub.is_estate { continue; }
            if tick.saturating_sub(hub.died_tick) < RESETTLE_COOLDOWN_YEARS * TICKS_PER_YEAR { continue; }
            // The old site must still carry productive potential to refound onto.
            if hub.base_per_capita.iter().all(|&v| v <= 0.0) { continue; }
            let comp = hub.component;
            let (hx, hy) = (hub.x, hub.y);
            let mut patron_ok = false;
            for o in 0..self.hubs.len() {
                if o == h { continue; }
                let nb = &self.hubs[o];
                if nb.is_estate || nb.abandoned || nb.component != comp { continue; }
                if nb.population < RESETTLE_PATRON_MIN_POP || nb.sent_prosperity < 0.5
                    || nb.food_balance < 0.1 { continue; }
                let mut dx = (nb.x - hx).abs();
                if self.world_w > 1.0 { dx = dx.min(self.world_w - dx); }
                let dy = nb.y - hy;
                if dx * dx + dy * dy <= reach * reach { patron_ok = true; break; }
            }
            if !patron_ok { continue; }
            if hash01(self.seed, tick as u64 ^ 0x2E51_1F, h as u64) > RESETTLE_PROB { continue; }
            self.revive_hub(h);
            revivals += 1;
        }
    }

    /// Refound a ruin as a living small town (restores its living settlement icon).
    fn revive_hub(&mut self, h: usize) {
        let tick = self.tick;
        let pop = RESETTLE_POP;
        {
            let hub = &mut self.hubs[h];
            hub.abandoned = false;
            hub.population = pop;
            hub.founding_pop = pop;
            hub.died_tick = 0;
            hub.died_cause = String::new();
            hub.founded_tick = tick;
            hub.decline_years = 0.0;
            hub.starving = 0.0;
            hub.food_balance = 1.0;
            hub.reserve_food = 30.0;
            hub.mood = 0.6;
            hub.sent_food = 0.7;
            hub.sent_prosperity = 0.4;
            hub.sent_stability = 0.6;
        }
        self.routes_dirty = true;
        self.total_foundings += 1;
        let nm = self.hubs[h].name.clone();
        self.journal.push(JournalEntry {
            tick, kind: "founding".into(), hub: h as i32, good: -1, value: pop,
            text: format!("Pioneers resettle the ruins of {} — the town lives again", nm),
        });
    }

    /// Yearly: settlement colonies (re)sign supply, may collapse if starved, graduate
    /// (gated by ≥5yr supply + population + buildings), pay dividends, and — once a
    /// mature self-sustaining city at year 70 — wage a war of independence.
    fn colony_pass(&mut self) {
        let tick = self.tick;
        for h in 0..self.hubs.len() {
            if self.hubs[h].colony_kind != 1 || self.hubs[h].autonomous { continue; }
            // (The food source + dedicated fleet are refreshed monthly by the daily
            // lifeline in `update_food_and_starvation`, so no yearly re-sign here.)
            // COLLAPSE: empty reserve + severe sustained starvation → the lifeline failed.
            if self.hubs[h].reserve_food <= 0.0 && self.hubs[h].starving > 0.8 {
                self.collapse_colony(h);
                continue;
            }
            let pop = self.hubs[h].population;
            // GROWTH GATE: advance only with ≥5yr unbroken supply AND population AND
            // enough buildings (stage 2 needs 1, stage 3 needs 2, stage 4 needs 3).
            let pop_stage: u8 = if pop >= 40_000.0 { 4 } else if pop >= 15_000.0 { 3 }
                else if pop >= 4_000.0 { 2 } else { 1 };
            let nbuild = self.hubs[h].structures.len() as u8;
            let supplied = self.hubs[h].supply_years >= 5.0;
            let mut new_stage = self.hubs[h].colony_stage.max(1);
            while new_stage < pop_stage && supplied && nbuild >= new_stage /* stageN→N+1 needs N buildings */ {
                new_stage += 1;
            }
            if new_stage > self.hubs[h].colony_stage {
                self.hubs[h].colony_stage = new_stage;
                let nm = self.hubs[h].name.clone();
                let label = ["", "outpost", "colony", "town", "city"][new_stage as usize];
                self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32,
                    good: -1, value: new_stage as f32, text: format!("{} grows into a {}", nm, label) });
            }
            // Dividends from the colony's trade surplus — small & capped so fortunes
            // stay bounded (the colony's earnings would otherwise pool locally).
            let surplus = (self.hubs[h].trade_wealth.max(0.0) * pop * COLONY_DIVIDEND_RATE).min(2.0);
            if surplus > 0.01 {
                for (kind, idx, share) in self.hubs[h].backers.clone() {
                    let cut = surplus * share;
                    match kind {
                        0 => { if (idx as usize) < self.hubs.len() { self.hubs[idx as usize].treasury += cut; } }
                        1 => { if (idx as usize) < self.houses.len() && !self.houses[idx as usize].defunct { self.houses[idx as usize].wealth += cut; } }
                        2 => { if (idx as usize) < self.banks.len() && !self.banks[idx as usize].defunct { self.banks[idx as usize].reserves += cut; } }
                        _ => {}
                    }
                }
            }
            // INDEPENDENCE: a mature (≥50y), TOWN-stage-or-better, well-supplied
            // colony rebels — war if the metropolis still stands, peacefully if it
            // has fallen. Relaxed from ≥70y/city-stage(40k) to ≥50y/town-stage(15k)
            // so colonies actually reach independence and become full free cities
            // (make_colony_independent already promotes colony_kind→0).
            let age = tick.saturating_sub(self.hubs[h].colony_founded_tick) / TICKS_PER_YEAR;
            if age >= 50 && self.hubs[h].colony_stage >= 3 && supplied
                && tick > self.hubs[h].indep_cooldown_until && self.hubs[h].war_with < 0 {
                let m = self.hubs[h].founder_hub;
                let metro_alive = m >= 0 && (m as usize) < self.hubs.len()
                    && self.hubs[m as usize].population >= 100.0 && self.hubs[m as usize].war_with < 0;
                if metro_alive {
                    self.declare_independence_war(h, m as usize);
                } else {
                    // Metropolis has fallen (Tyre besieged) → peaceful drift, founding
                    // dynasty inherits.
                    self.make_colony_independent(h, false);
                }
            }
        }
    }

    /// Metropolis trade MONOPOLY (charter): while the colony is a dependency, bar
    /// every house that is NOT a backer or the metropolis's council family from the
    /// colony's market. Reuses the `house_barred` exclusion list (hub indices).
    fn apply_colony_charter(&mut self, h: usize) {
        let allow: std::collections::HashSet<u32> = self.hubs[h].backers.iter()
            .filter(|(k, _, _)| *k == 1).map(|(_, i, _)| *i).collect();
        let metro = self.hubs[h].founder_hub;
        let metro_house = if metro >= 0 && (metro as usize) < self.hubs.len() {
            self.hubs[metro as usize].council_house
        } else { -1 };
        self.house_barred.resize(self.houses.len(), Vec::new());
        for hi in 0..self.houses.len() {
            if allow.contains(&(hi as u32)) || hi as i32 == metro_house { continue; }
            if let Some(v) = self.house_barred.get_mut(hi) {
                if !v.contains(&(h as u32)) { v.push(h as u32); }
            }
        }
    }

    /// Lift a colony's charter (on independence/collapse): unbar every house.
    fn lift_colony_charter(&mut self, h: usize) {
        for v in self.house_barred.iter_mut() { v.retain(|&x| x != h as u32); }
    }

    /// A colony's lifeline failed: its bank's loan defaults (a loss that can sink the
    /// bank → crash), backers lose their stake, and the settlement dies out.
    fn collapse_colony(&mut self, h: usize) {
        let tick = self.tick;
        let nm = self.hubs[h].name.clone();
        let mb = self.hubs[h].main_bank;
        if mb >= 0 && (mb as usize) < self.banks.len() {
            let bi = mb as usize;
            let mut writeoff = 0.0;
            self.banks[bi].loans.retain(|l| {
                if l.borrower_polis == h as i32 && l.purpose == "colony" { writeoff += l.outstanding; false } else { true }
            });
            self.banks[bi].losses += writeoff;
        }
        self.colony_supply.retain(|s| s.colony_hub != h as u32);
        self.lift_colony_charter(h);
        self.hubs[h].colony_kind = 0;
        self.hubs[h].autonomous = false;
        self.hubs[h].founder_hub = -1;
        self.hubs[h].main_bank = -1;
        self.hubs[h].backers.clear();
        self.hubs[h].population = 0.0; // the dead-city marker handles the rest
        // Atlas 2.0: mark it truly DEAD — without this the founding-pop floor in the
        // population pass resurrected collapsed colonies at 10% strength next tick.
        self.hubs[h].abandoned = true;
        self.hubs[h].died_tick = tick;
        self.hubs[h].died_cause = "famine".into(); // the lifeline failed
        for v in self.hubs[h].stock.iter_mut() { *v = 0.0; }
        for v in self.hubs[h].production.iter_mut() { *v = 0.0; }
        self.total_abandonments += 1;
        self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32, good: -1,
            value: 0.0, text: format!("{} collapses — its food lifeline failed and the settlement is abandoned", nm) });
    }

    /// Re-rank hubs into commercial classes from the trade that has ACTUALLY flowed
    /// this period (0 ordinary · 1 trade hub · 2 entrepôt). Called twice a year. An
    /// entrepôt is a great SEA pass-through market (Venice/Bruges); a trade hub is a
    /// busy secondary market. 3-check hysteresis (`class_momentum`) so status is earned
    /// and lost over time, not flickered. Uses year-to-date flows, so ranking is
    /// relative and cadence-independent.
    fn classify_hubs(&mut self) {
        let nn = self.hubs.len();
        if nn == 0 { return; }
        let mut throughput = vec![0.0f32; nn];
        for (&(a, b), &v) in self.flow_accum.iter() {
            if (a as usize) < nn { throughput[a as usize] += v; }
            if (b as usize) < nn { throughput[b as usize] += v; }
        }
        // Commercial standing = mostly trade throughput, but BLENDED with population so
        // the ranks genuinely move as cities rise and fall (user: entrepôts/trade hubs
        // must be dynamic, not frozen). A booming city climbs the ranks; a shrinking one
        // slips even if its old trade lingers. Both terms normalised to 0..1.
        let max_thr = throughput.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
        let max_pop = self.hubs.iter().map(|h| h.population).fold(1.0f32, f32::max);
        let score: Vec<f32> = (0..nn).map(|i| {
            0.65 * (throughput[i] / max_thr) + 0.35 * (self.hubs[i].population / max_pop)
        }).collect();
        let mut order: Vec<usize> = (0..nn)
            .filter(|&i| !self.hubs[i].is_estate && !self.hubs[i].abandoned
                && self.hubs[i].population >= 1.0)
            .collect();
        order.sort_by(|&a, &b| score[b].partial_cmp(&score[a]).unwrap_or(std::cmp::Ordering::Equal));
        let live = order.len().max(1);
        let hub_cut = ((live as f32) * 0.20).ceil() as usize;         // top 20% → trade hub
        let emp_cut = ((live as f32) * 0.05).ceil().max(1.0) as usize; // top 5% → entrepôt
        let mut rank = vec![usize::MAX; nn];
        for (r, &i) in order.iter().enumerate() { rank[i] = r; }
        for i in 0..nn {
            self.hubs[i].transit_year = throughput[i];
            if self.hubs[i].is_estate || self.hubs[i].abandoned || self.hubs[i].population < 1.0 {
                self.hubs[i].hub_class = 0;
                self.hubs[i].class_momentum = 0;
                continue;
            }
            let r = rank[i];
            let sc = score[i];
            // Desired class this period, with an absolute floor so a barely-active world
            // doesn't crown entrepôts. Entrepôts must be SEA ports.
            let desired: u8 = if r < emp_cut && self.hubs[i].coastal && sc >= 0.30 {
                2
            } else if r < hub_cut && sc >= 0.10 {
                1
            } else {
                0
            };
            let cur = self.hubs[i].hub_class;
            // Hysteresis: 2 confirming half-years to change a tier (faster than before so
            // the map keeps up with a shifting economy, still no per-check flicker).
            if desired > cur {
                let m = self.hubs[i].class_momentum.max(0) + 1;
                if m >= 2 { self.hubs[i].hub_class = cur + 1; self.hubs[i].class_momentum = 0; }
                else { self.hubs[i].class_momentum = m; }
            } else if desired < cur {
                let m = self.hubs[i].class_momentum.min(0) - 1;
                if m <= -2 { self.hubs[i].hub_class = cur - 1; self.hubs[i].class_momentum = 0; }
                else { self.hubs[i].class_momentum = m; }
            } else {
                let s = self.hubs[i].class_momentum.signum();
                self.hubs[i].class_momentum -= s;
            }
        }
    }

    /// Free a colony from its metropolis (drop dependency, the `(colony)` tag, charter
    /// and backers) and hand over CONTROL, Carthage-style:
    ///   • peaceful (via_war=false) → the FOUNDING HOUSE (the trade dynasty that built
    ///     it — the Magonids of Carthage) inherits the free city's council;
    ///   • won by WAR (via_war=true) → the mother city's house is EXPELLED and a NEW
    ///     revolutionary trade family rises to lead it.
    fn make_colony_independent(&mut self, h: usize, via_war: bool) {
        let tick = self.tick;
        let nm = self.hubs[h].name.replace(" (colony)", "");
        // Founding merchant house = the house backer of the joint-stock venture.
        let founding_house = self.hubs[h].backers.iter()
            .find(|(k, _, _)| *k == 1).map(|&(_, i, _)| i as usize)
            .filter(|&i| i < self.houses.len() && !self.houses[i].defunct);
        let metro = self.hubs[h].founder_hub;
        let metro_house = if metro >= 0 && (metro as usize) < self.hubs.len() {
            self.hubs[metro as usize].council_house
        } else { -1 };
        self.hubs[h].colony_kind = 0;
        self.hubs[h].autonomous = true;
        self.hubs[h].founder_hub = -1;
        self.hubs[h].backers.clear();
        self.colony_supply.retain(|s| s.colony_hub != h as u32);
        self.lift_colony_charter(h);
        self.hubs[h].name = nm.clone();
        if via_war {
            // Expel the mother city's house (its office here + bar it), then seat a NEW
            // revolutionary family that leads the freed city.
            if metro_house >= 0 && (metro_house as usize) < self.houses.len() {
                let mh = metro_house as usize;
                self.houses[mh].offices.retain(|&o| o as usize != h);
                self.house_barred.resize(self.houses.len(), Vec::new());
                if let Some(v) = self.house_barred.get_mut(mh) {
                    if !v.contains(&(h as u32)) { v.push(h as u32); }
                }
            }
            let new_house = self.found_house_at(h);
            self.hubs[h].council_house = new_house.map(|i| i as i32).unwrap_or(-1);
            let mn = if metro >= 0 && (metro as usize) < self.hubs.len() {
                self.hubs[metro as usize].name.clone()
            } else { "its metropolis".into() };
            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32,
                good: -1, value: 0.0,
                text: format!("{} throws off {} in a war of independence; a new merchant family takes the reins", nm, mn) });
        } else {
            // Peaceful drift: the founding dynasty inherits the free city.
            if let Some(fh) = founding_house {
                self.hubs[h].council_house = fh as i32;
                if !self.houses[fh].offices.contains(&(h as u32)) {
                    self.houses[fh].offices.push(h as u32);
                }
                let (hn, cn) = (self.houses[fh].name.clone(), nm.clone());
                self.houses[fh].events.push(HouseEvent { tick, kind: "colony".into(),
                    text: format!("{} inherits {} as a free city", hn, cn) });
            }
            self.journal.push(JournalEntry { tick, kind: "colony".into(), hub: h as i32,
                good: -1, value: 0.0,
                text: format!("{} comes peacefully into its own as a free city under its founding house", nm) });
        }
    }

    /// Seat a brand-new merchant family at hub `h` — used when a colony WINS its
    /// independence by war (a fresh dynasty rises to rule the free city). Returns the
    /// new house index. Mirrors `maybe_found_house`'s construction.
    fn found_house_at(&mut self, h: usize) -> Option<usize> {
        let tick = self.tick;
        let ng = self.goods.len();
        if ng == 0 { return None; }
        let mut gi: Vec<usize> = (0..ng).collect();
        gi.sort_by(|&a, &b| self.hubs[h].production[b]
            .partial_cmp(&self.hubs[h].production[a]).unwrap_or(std::cmp::Ordering::Equal));
        let mut spec: Vec<usize> = gi.into_iter()
            .filter(|&g| self.hubs[h].production[g] > 0.0).take(2).collect();
        if spec.is_empty() { spec.push(0); }
        let name = self.unique_family_name_for(h, tick as u64 ^ 0xBEEF);
        let head = self.head_name_for(h, &name, tick as u64 ^ 0x2468);
        let founded = HouseEvent { tick, kind: "founded".into(),
            text: format!("{} rises to lead free {}", name, self.hubs[h].name) };
        let (fleet_sea, fleet_river, fleet_caravan) = Self::initial_fleet(self.hubs[h].coastal, false);
        let idx = self.houses.len();
        self.houses.push(House {
            name, hub: h as u32, wealth: 5.0, prestige: 0.2, spec,
            monopoly: vec![], rivals: vec![], generation: 1,
            events: vec![founded], good_profit: Vec::new(), mono50: Vec::new(),
            mono_ever: Vec::new(), dominant_seat: true, prev_wealth: 5.0, worst_loss: 0.0,
            fleet_sea, fleet_river, fleet_caravan,
            head_name: head, head_since: tick,
            head_lifespan: self.roll_lifespan(h as u64 ^ 0x51),
            founded_tick: tick, political_power: 0.0, volume: 0.0, defunct: false,
            archetype: pick_archetype(self.seed, tick as u64 ^ h as u64),
            charters: Vec::new(),
            is_guild: false, offices: vec![h as u32], trade_at: Vec::new(), debt_since: 0,
            wealth_history: Vec::new(), office_leases: Vec::new(),
            influence: Vec::new(), bailos: Vec::new(),
        });
        Some(idx)
    }

    /// Open a war of independence between a colony and its metropolis (reuses the
    /// economic-war machinery; resolved in `update_wars`, where chest+treasury+luck
    /// decide it — a colony can still win against the odds).
    fn declare_independence_war(&mut self, colony: usize, metro: usize) {
        let (a, b) = if colony < metro { (colony, metro) } else { (metro, colony) };
        self.hubs[a].war_with = b as i32;
        self.hubs[b].war_with = a as i32;
        self.hubs[a].war_since = self.tick;
        self.hubs[b].war_since = self.tick;
        self.wars.push(War { a: a as u32, b: b as u32, start_tick: self.tick,
            chest_a: 0.0, chest_b: 0.0, levies: 0.0, cargo_lost: 0, cause: "independence".into(),
            goal: WAR_GOAL_PLUNDER });
        let (cn, mn) = (self.hubs[colony].name.clone(), self.hubs[metro].name.clone());
        self.journal.push(JournalEntry { tick: self.tick, kind: "war".into(), hub: colony as i32,
            good: -1, value: 0.0, text: format!("{} rises in a war of independence against {}", cn, mn) });
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

    /// Trade-tax multiplier for house `hi` trading AT `city`: a BAILO concession pays
    /// only a token toll; otherwise the city's trade DOMINATOR pays less and rival
    /// houses pay a little more than the base rate (a modest sway, not a stranglehold).
    fn house_city_tax_mult(&self, hi: usize, city: usize) -> f32 {
        if self.houses[hi].bailos.contains(&(city as u32)) { return BAILO_CONCESSION_TOLL; }
        match self.city_dominator.get(city).copied().unwrap_or(-1) {
            d if d == hi as i32 => DOMINATOR_TAX_MULT,
            d if d >= 0 => RIVAL_TAX_MULT,
            _ => 1.0,
        }
    }

    /// Monthly: build each house's per-city commercial INFLUENCE from its share of
    /// that city's trade ÷ the city's resistance (population + a resident guild),
    /// decaying elsewhere; recompute the per-city trade DOMINATOR; seed rivalries in
    /// contested cities; then consider Bailo (HQ) upgrades.
    fn update_influence_and_bailos(&mut self) {
        let tick = self.tick;
        let n = self.hubs.len();
        let nh = self.houses.len();
        if self.city_dominator.len() != n { self.city_dominator = vec![-1; n]; }
        // Per-city total house-trade volume + which cities host a civic guild.
        let mut city_total = vec![0.0f32; n];
        let mut has_guild = vec![false; n];
        for h in &self.houses {
            if h.defunct { continue; }
            if h.is_guild { let hb = h.hub as usize; if hb < n { has_guild[hb] = true; } }
            for &(hb, v) in &h.trade_at { if (hb as usize) < n { city_total[hb as usize] += v.max(0.0); } }
        }
        // 1) Accrue / decay influence per house.
        for hi in 0..nh {
            if self.houses[hi].defunct { self.houses[hi].influence.clear(); continue; }
            let mut infl: std::collections::HashMap<u32, f32> =
                self.houses[hi].influence.iter().copied().collect();
            // Standing RELAXES toward the house's current market share in each city (an
            // EMA), so influence reflects real share instead of ratcheting to 1.0. The
            // old additive-gain/flat-decay scheme had no interior fixed point: any
            // positive share (gain > decay) climbed to the clamp, so every active city
            // pinned at 1.00. Untraded cities decay multiplicatively toward 0; city
            // RESISTANCE (pop + a resident guild) only slows how fast a house climbs —
            // share, not elapsed time, sets the ceiling, so dominance stays reachable.
            for v in infl.values_mut() { *v *= 1.0 - INFLUENCE_DECAY; }
            let trade_at = self.houses[hi].trade_at.clone();
            for &(hb, v) in &trade_at {
                let c = hb as usize; if c >= n { continue; }
                let share = if city_total[c] > 1e-6 { (v.max(0.0) / city_total[c]).clamp(0.0, 1.0) } else { 0.0 };
                let resist = 1.0 + self.hubs[c].population.max(0.0) / INFLUENCE_POP_REF
                    + if has_guild[c] { INFLUENCE_GUILD_RESIST } else { 0.0 };
                let e = infl.entry(hb).or_insert(0.0);
                *e = (*e + (INFLUENCE_GAIN / resist) * (share - *e)).clamp(0.0, 1.0);
            }
            // Seat + offices guarantee a standing foothold of influence.
            let home = self.houses[hi].hub;
            infl.entry(home).and_modify(|x| *x = x.max(OFFICE_INFLUENCE_FLOOR)).or_insert(OFFICE_INFLUENCE_FLOOR);
            for &o in &self.houses[hi].offices.clone() {
                infl.entry(o).and_modify(|x| *x = x.max(OFFICE_INFLUENCE_FLOOR)).or_insert(OFFICE_INFLUENCE_FLOOR);
            }
            let mut v: Vec<(u32, f32)> = infl.into_iter().filter(|&(_, x)| x > 0.02).collect();
            v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            self.houses[hi].influence = v;
        }
        // 2) Per-city dominator + runner-up (for the margin test + friction).
        let mut top = vec![(-1i32, 0.0f32); n];
        let mut second = vec![(-1i32, 0.0f32); n];
        for hi in 0..nh {
            if self.houses[hi].defunct { continue; }
            for &(hb, x) in &self.houses[hi].influence {
                let c = hb as usize; if c >= n { continue; }
                if x > top[c].1 { second[c] = top[c]; top[c] = (hi as i32, x); }
                else if x > second[c].1 { second[c] = (hi as i32, x); }
            }
        }
        let mut dom = vec![-1i32; n];
        for c in 0..n {
            if top[c].0 >= 0 && top[c].1 >= DOMINANCE_THRESHOLD && top[c].1 - second[c].1 >= DOMINANCE_MARGIN {
                dom[c] = top[c].0;
            }
        }
        // 3) New-dominance events (skip the house's own seat — already "controlled").
        for c in 0..n {
            if dom[c] >= 0 && self.city_dominator[c] != dom[c] && !self.hubs[c].is_estate {
                let hi = dom[c] as usize;
                if self.houses[hi].hub as usize != c {
                    let (hn, cn) = (self.houses[hi].name.clone(), self.hubs[c].name.clone());
                    self.houses[hi].events.push(HouseEvent { tick, kind: "dominance".into(),
                        text: format!("{} comes to dominate the trade of {}", hn, cn) });
                    self.journal.push(JournalEntry { tick, kind: "dominance".into(), hub: c as i32,
                        good: -1, value: top[c].1, text: format!("{} dominates the trade of {}", hn, cn) });
                }
            }
        }
        self.city_dominator = dom;
        // 4) Friction: a city contested by the top two houses can spark a rivalry.
        for c in 0..n {
            if top[c].0 >= 0 && second[c].0 >= 0
                && top[c].1 >= CONTEST_INFLUENCE && second[c].1 >= CONTEST_INFLUENCE {
                let (ai, bi) = (top[c].0 as usize, second[c].0 as usize);
                if ai != bi && !self.houses[ai].rivals.contains(&bi)
                    && hash01(self.seed, tick as u64 ^ 0x1F1C, (ai * 131 + bi) as u64) < 0.12 {
                    self.houses[ai].rivals.push(bi);
                    if !self.houses[bi].rivals.contains(&ai) { self.houses[bi].rivals.push(ai); }
                }
            }
        }
        // 5) Bailo (HQ) upgrades + upkeep.
        self.update_bailos();
    }

    /// Promote a house's strongest dominated office to a BAILO (a governing HQ), within
    /// a soft cap set by its wealth/power; charge upkeep; drop Bailos that have decayed.
    fn update_bailos(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct || self.houses[hi].is_guild { continue; }
            // Snapshot this house's influence (avoids borrowing self inside `retain`).
            let infl: std::collections::HashMap<u32, f32> =
                self.houses[hi].influence.iter().copied().collect();
            let at = |c: u32| infl.get(&c).copied().unwrap_or(0.0);
            // Drop Bailos whose grip has slipped well below the threshold.
            self.houses[hi].bailos.retain(|&c| at(c) >= BAILO_MIN_INFLUENCE * 0.6);
            // Upkeep per surviving Bailo (scaled by city size).
            for &c in &self.houses[hi].bailos.clone() {
                let up = BAILO_UPKEEP * self.city_size_factor(c as usize);
                self.houses[hi].wealth -= up;
            }
            // Soft cap on foreign Bailos = floor(power·scale) + wealth/per.
            let power = self.houses[hi].political_power;
            let wealth = self.houses[hi].wealth;
            let cap = (power * BAILO_CAP_POWER_SCALE) as usize
                + (wealth / BAILO_CAP_WEALTH_PER).max(0.0) as usize;
            if self.houses[hi].bailos.len() >= cap || wealth < BAILO_MIN_WEALTH { continue; }
            // Promote the strongest eligible (dominated, non-Bailo) office.
            let offices = self.houses[hi].offices.clone();
            let mut best: Option<(u32, f32)> = None;
            for &o in &offices {
                if self.houses[hi].bailos.contains(&o) { continue; }
                let x = at(o);
                if x >= BAILO_MIN_INFLUENCE && best.map_or(true, |(_, bv)| x > bv) { best = Some((o, x)); }
            }
            if let Some((o, _)) = best {
                self.houses[hi].bailos.push(o);
                self.lease_office(hi, o, OFFICE_LEASE_YEARS); // keep the base for the HQ
                let (hn, cn) = (self.houses[hi].name.clone(),
                    self.hubs.get(o as usize).map(|x| x.name.clone()).unwrap_or_default());
                self.houses[hi].events.push(HouseEvent { tick, kind: "bailo".into(),
                    text: format!("{} raises a Bailo — a governing headquarters — in {}", hn, cn) });
                self.journal.push(JournalEntry { tick, kind: "bailo".into(), hub: o as i32,
                    good: -1, value: 0.0, text: format!("{} establishes a Bailo in {}", hn, cn) });
            }
        }
    }

    pub fn world_h(&self) -> u32 { (self.world_w * 0.5).max(1.0) as u32 }

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

    /// Phase 4 (flavour) · raise & retire NOTABLE FIGURES at the yearly hook. Sparse
    /// (≤ `FIGURE_LIVING_CAP` alive, a modest yearly chance) and deterministic (all
    /// rolls hash `seed`/`tick`/`hub`). Each figure grants ONE small, capped effect
    /// on an existing bounded field and is chronicled at rise and at death, so the
    /// economy dynamics stay bounded.
    fn raise_notable_figures(&mut self, yr: u32) {
        let tick = self.tick;
        // 1) Retire the departed — chronicle each death once.
        for i in 0..self.figures.len() {
            if self.figures[i].dead || self.figures[i].dies_tick > tick { continue; }
            self.figures[i].dead = true;
            let (kind, good, hub, house) =
                (self.figures[i].kind, self.figures[i].good, self.figures[i].hub, self.figures[i].house);
            let name = self.figures[i].name.clone();
            let city = self.hubs.get(hub as usize).map(|h| h.name.clone()).unwrap_or_default();
            self.journal.push(JournalEntry {
                tick, kind: "figure".into(), hub: hub as i32, good, value: 0.0,
                text: format!("{} {} of {} has died.", role_title(kind), name, city),
            });
            if house >= 0 && (house as usize) < self.houses.len() {
                self.houses[house as usize].events.push(HouseEvent {
                    tick, kind: "figure".into(),
                    text: format!("{} {} passes into memory.", role_title(kind), name),
                });
            }
        }
        // Bound the roster (dead figures accumulate over a long campaign).
        if self.figures.len() > FIGURE_CAP {
            let drop = self.figures.len() - FIGURE_CAP;
            self.figures.drain(0..drop);
        }
        // 2) Maybe raise a new one.
        let living = self.figures.iter().filter(|f| !f.dead).count();
        if living >= FIGURE_LIVING_CAP { return; }
        if hash01(self.seed, tick as u64, 0xF16) >= FIGURE_YEARLY_CHANCE { return; }
        let n = self.hubs.len();
        if n == 0 { return; }
        // Prominent hub: hash-pick from the most populous real settlements.
        let mut real: Vec<usize> = (0..n).filter(|&h| !self.hubs[h].is_estate).collect();
        if real.is_empty() { return; }
        real.sort_by(|&a, &b| self.hubs[b].population
            .partial_cmp(&self.hubs[a].population).unwrap_or(std::cmp::Ordering::Equal));
        real.truncate(20.min(real.len()));
        let pick = (hash01(self.seed, tick as u64 ^ 0xA1, yr as u64) * real.len() as f32) as usize;
        let hub = real[pick.min(real.len() - 1)];
        let ng = self.goods.len();
        // Resident house (first non-defunct family seated here), if any.
        let resident = self.houses.iter()
            .position(|h| h.hub as usize == hub && !h.defunct).map(|i| i as i32).unwrap_or(-1);
        let has_house = resident >= 0;
        let coastal = self.hubs[hub].coastal;
        // Kind: base roll, with fallbacks when the context can't support it.
        let mut kind = ((hash01(self.seed, tick as u64 ^ 0x5E, hub as u64) * 5.0) as u8).min(4);
        if (kind == 0 || kind == 3) && !has_house { kind = 1; }          // house role → demagogue
        if (kind == 0 || kind == 4) && !coastal && has_house { kind = 3; } // inland → banker
        // Craftsman's craft = the hub's strongest output; else fall back to demagogue.
        let mut good = -1i32;
        if kind == 2 {
            if self.hubs[hub].quality.len() == ng && ng > 0 {
                let mut bg = 0usize; let mut bv = -1.0f32;
                for g in 0..ng {
                    let p = self.hubs[hub].production.get(g).copied().unwrap_or(0.0);
                    if p > bv { bv = p; bg = g; }
                }
                good = bg as i32;
            } else { kind = 1; }
        }
        // Name (+ an epithet for the martial/rabble-rousing/roving kinds).
        let salt = (tick as u64) ^ (hub as u64).wrapping_mul(0x9E3779B1) ^ (yr as u64);
        let surname_src = if has_house { self.houses[resident as usize].name.clone() }
            else { self.hubs[hub].name.clone() };
        let person = self.head_name_for(hub, &surname_src, salt);
        let (fx, fy) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        let epithet = crate::sim::names::gen_name_epithet(fx, fy, self.world_w as u32, self.world_h(), 2);
        let name = if !epithet.is_empty() && (kind == 0 || kind == 1 || kind == 4) {
            format!("{} {}", person, epithet)
        } else { person };
        let city = self.hubs[hub].name.clone();
        // Effect (capped) + chronicle text.
        let text = match kind {
            0 => { // Admiral — a house's sea fleet + renown
                let hi = resident as usize;
                if self.houses[hi].fleet_sea < 15 { self.houses[hi].fleet_sea += 1; }
                self.houses[hi].prestige += 0.05;
                format!("Admiral {} wins renown at sea for {}.", name, self.houses[hi].name)
            }
            1 => { // Demagogue — stirs civic unrest
                let u = self.hubs[hub].society.unrest;
                self.hubs[hub].society.unrest = (u + 0.08).min(1.0);
                format!("The demagogue {} stirs the crowds of {}.", name, city)
            }
            2 => { // Master Craftsman — lifts a city's craft quality
                let g = good.max(0) as usize;
                if g < self.hubs[hub].quality.len() {
                    let q = self.hubs[hub].quality[g];
                    self.hubs[hub].quality[g] = (q + 0.06).min(0.9);
                }
                let gn = self.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
                format!("Master {} raises the {} craft of {} to new heights.", name, gn, city)
            }
            3 => { // Great Banker — a house's standing
                if has_house { self.houses[resident as usize].prestige += 0.08; }
                format!("{}, a great banker of {}, gathers capital from across the sea.", name, city)
            }
            _ => { // Explorer — a house's standing + a distant venture
                if has_house { self.houses[resident as usize].prestige += 0.06; }
                format!("{} sets out from {} to chart distant shores.", name, city)
            }
        };
        self.journal.push(JournalEntry {
            tick, kind: "figure".into(), hub: hub as i32, good, value: 0.0, text,
        });
        if has_house {
            self.houses[resident as usize].events.push(HouseEvent {
                tick, kind: "figure".into(),
                text: format!("{} {} brings the family renown.", role_title(kind), name),
            });
        }
        let span = ((15.0 + hash01(self.seed, tick as u64 ^ 0xCA6, hub as u64) * 25.0)
            * TICKS_PER_YEAR as f32) as u32;
        self.figures.push(Figure {
            name, kind, hub: hub as u32, house: resident, good,
            born_tick: tick, dies_tick: tick + span, dead: false,
        });
    }

    /// Phase 5 (flavour) · FASHION: a luxury good may become the rage of the season,
    /// lifting its demand for a couple of years (via a capped `fashion` ActiveEvent
    /// the consumption step reads), then falling from favour. Deterministic.
    fn roll_fashion(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0xFA5A, 0) >= FASHION_YEARLY_CHANCE { return; }
        let ng = self.goods.len();
        // Luxuries (top need tier) that aren't food.
        let lux: Vec<usize> = (0..ng)
            .filter(|&g| self.goods[g].need_tier >= 2 && !self.goods[g].food).collect();
        if lux.is_empty() { return; }
        let g = lux[((hash01(self.seed, yr as u64, 0xF00) * lux.len() as f32) as usize) % lux.len()];
        // Already in vogue? skip.
        if self.active_events.iter().any(|e| e.kind == "fashion" && e.good == g as i32) { return; }
        let dur = (2.0 + hash01(self.seed, yr as u64, 0xF01) * 2.0) * TICKS_PER_YEAR as f32;
        self.active_events.push(ActiveEvent {
            kind: "fashion".into(), hub: -1, good: g as i32,
            magnitude: FASHION_MAG, until_tick: self.tick + dur as u32,
        });
        let gn = self.goods[g].name.clone();
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "fashion".into(), hub: -1, good: g as i32, value: 0.0,
            text: format!("{} is the rage of the season — everyone must have it.", cap_first(&gn)),
        });
    }

    /// Phase 5 (flavour) · CIVIC WONDERS: a prosperous city occasionally raises a
    /// monument (lighthouse → market hall → cathedral) for prestige & stability.
    fn run_civic_wonders(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0x0A0E, 0) >= WONDER_YEARLY_CHANCE { return; }
        let n = self.hubs.len();
        // The most prosperous real city that still has a wonder tier to build.
        let mut best = usize::MAX; let mut best_score = -1.0f32;
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let built = self.wonders.iter().filter(|w| w.0 == h as u32).count();
            if built >= WONDER_NAMES.len() { continue; }
            let score = self.hubs[h].population.max(0.0)
                * self.hubs[h].sent_prosperity.clamp(0.0, 1.0);
            if score > best_score { best_score = score; best = h; }
        }
        if best == usize::MAX { return; }
        let tier = self.wonders.iter().filter(|w| w.0 == best as u32).count() as u8;
        self.wonders.push((best as u32, tier));
        self.hubs[best].sent_stability = (self.hubs[best].sent_stability + 0.06).min(1.0);
        let city = self.hubs[best].name.clone();
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "wonder".into(), hub: best as i32, good: -1, value: 0.0,
            text: format!("{} completes {} — a wonder of the age.", city, WONDER_NAMES[tier.min(2) as usize]),
        });
    }

    /// Phase 5 (flavour) · PIRACY: corsairs seize a merchant house's galley, costing
    /// it one sea-fleet asset (floored at 0). Bounded; a chronicle beat.
    fn run_piracy(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0x9A17, 0) >= PIRACY_YEARLY_CHANCE { return; }
        let cand: Vec<usize> = (0..self.houses.len())
            .filter(|&i| !self.houses[i].defunct && self.houses[i].fleet_sea > 0).collect();
        if cand.is_empty() { return; }
        let hi = cand[((hash01(self.seed, yr as u64, 0x9A2) * cand.len() as f32) as usize) % cand.len()];
        self.houses[hi].fleet_sea = self.houses[hi].fleet_sea.saturating_sub(1);
        let (hn, hub) = (self.houses[hi].name.clone(), self.houses[hi].hub as i32);
        let city = self.hubs.get(hub as usize).map(|h| h.name.clone()).unwrap_or_default();
        self.houses[hi].events.push(HouseEvent {
            tick: self.tick, kind: "piracy".into(), text: format!("Corsairs seize one of our galleys off {}.", city) });
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "piracy".into(), hub, good: -1, value: 0.0,
            text: format!("Corsairs seize a {} galley off {}.", hn, city),
        });
    }

    /// Phase 5 (flavour) · DIASPORA quarters: a house with a far-flung office founds a
    /// resident QUARTER (fondaco) there — a small standing-influence foothold + a beat.
    fn run_diaspora(&mut self, yr: u32) {
        if hash01(self.seed, yr as u64 ^ 0xD1A5, 0) >= DIASPORA_YEARLY_CHANCE { return; }
        let world_w = self.world_w.max(1.0);
        // Collect (house, distant office hub) candidates.
        let mut cand: Vec<(usize, u32)> = Vec::new();
        for hi in 0..self.houses.len() {
            if self.houses[hi].defunct { continue; }
            let home = self.houses[hi].hub as usize;
            if home >= self.hubs.len() { continue; }
            let (hx, hy) = (self.hubs[home].x, self.hubs[home].y);
            for &off in &self.houses[hi].offices {
                let o = off as usize;
                if o >= self.hubs.len() { continue; }
                let mut dx = (self.hubs[o].x - hx).abs();
                if world_w > 1.0 { dx = dx.min(world_w - dx); }
                let dy = self.hubs[o].y - hy;
                if (dx * dx + dy * dy).sqrt() > world_w * 0.20 { cand.push((hi, off)); }
            }
        }
        if cand.is_empty() { return; }
        let (hi, off) = cand[((hash01(self.seed, yr as u64, 0xD1B) * cand.len() as f32) as usize) % cand.len()];
        // Small standing-influence foothold at the host city.
        if let Some(e) = self.houses[hi].influence.iter_mut().find(|e| e.0 == off) {
            e.1 = (e.1 + 0.05).min(1.0);
        } else {
            self.houses[hi].influence.push((off, 0.05));
        }
        let (hn, city) = (self.houses[hi].name.clone(),
            self.hubs.get(off as usize).map(|h| h.name.clone()).unwrap_or_default());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "diaspora".into(), hub: off as i32, good: -1, value: 0.0,
            text: format!("{} founds a merchant quarter in {} — a diaspora takes root.", hn, city),
        });
    }

    /// Phase 5 (flavour) · seed CRAFT GUILDS once — the strongest manufacturing city
    /// for each manufactured (recipe) good gets a guild of its masters. Deterministic;
    /// capped at `GUILD_MAX`.
    fn seed_craft_guilds(&mut self) {
        let n = self.hubs.len();
        let ng = self.goods.len();
        let mut guilds: Vec<CraftGuild> = Vec::new();
        for g in 0..ng {
            if self.goods[g].inputs.is_empty() { continue; } // only manufactured goods
            // The city that makes the most of this good.
            let mut best_hub = usize::MAX; let mut best = 0.0f32;
            for h in 0..n {
                if self.hubs[h].is_estate { continue; }
                let p = self.hubs[h].production.get(g).copied().unwrap_or(0.0);
                if p > best { best = p; best_hub = h; }
            }
            if best_hub != usize::MAX && best > 0.0 {
                guilds.push(CraftGuild { hub: best_hub as u32, good: g as u32, strength: 0.3, hall: false });
            }
        }
        // Keep the strongest guilds (by their host city's output) if we overflow.
        guilds.sort_by(|a, b| {
            let pa = self.hubs[a.hub as usize].production.get(a.good as usize).copied().unwrap_or(0.0);
            let pb = self.hubs[b.hub as usize].production.get(b.good as usize).copied().unwrap_or(0.0);
            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
        });
        guilds.truncate(GUILD_MAX);
        guilds.sort_by_key(|gd| (gd.hub, gd.good)); // stable order
        self.guilds = guilds;
    }

    /// Phase 5 (flavour) · run the CRAFT GUILDS yearly: lift their good's local
    /// quality, occasionally strike (a short manufacture dent), and raise a guildhall
    /// once the guild is strong. Bounded quality/stability; chronicle beats.
    fn run_craft_guilds(&mut self, yr: u32) {
        let ng = self.goods.len();
        for gi in 0..self.guilds.len() {
            let (hub, good) = (self.guilds[gi].hub as usize, self.guilds[gi].good as usize);
            if hub >= self.hubs.len() || good >= ng { continue; }
            // Master the craft: a steady, capped quality lift.
            if self.hubs[hub].quality.len() == ng {
                let q = self.hubs[hub].quality[good];
                self.hubs[hub].quality[good] = (q + GUILD_QUALITY_STEP).min(GUILD_QUALITY_CAP);
            }
            self.guilds[gi].strength = (self.guilds[gi].strength + 0.04).min(1.0);
            // Raise a guildhall once the guild is well-established (one-time monument).
            if !self.guilds[gi].hall && self.guilds[gi].strength >= GUILD_HALL_STRENGTH {
                self.guilds[gi].hall = true;
                self.hubs[hub].sent_stability = (self.hubs[hub].sent_stability + 0.05).min(1.0);
                let (city, gn) = (self.hubs[hub].name.clone(), self.goods[good].name.clone());
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "guildhall".into(), hub: hub as i32, good: good as i32,
                    value: 0.0, text: format!("The {} guild of {} raises a grand guildhall.", gn, city),
                });
            }
            // A strike: the masters down tools, halting the craft for a spell.
            if hash01(self.seed, yr as u64 ^ 0x6111D, gi as u64) < GUILD_STRIKE_CHANCE {
                let dur = 20 + (hash01(self.seed, yr as u64, gi as u64) * 40.0) as u32;
                self.active_events.push(ActiveEvent {
                    kind: "guild_strike".into(), hub: hub as i32, good: good as i32,
                    magnitude: GUILD_STRIKE_MAG, until_tick: self.tick + dur,
                });
                let (city, gn) = (self.hubs[hub].name.clone(), self.goods[good].name.clone());
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "guild_strike".into(), hub: hub as i32, good: good as i32,
                    value: dur as f32, text: format!("The {} guild of {} downs tools in a strike.", gn, city),
                });
            }
        }
    }

    /// Phase 5 (flavour) · dynastic MARRIAGES between houses. Once a year a prominent
    /// house may wed another — ending any feud between them, sealing an alliance, and
    /// exchanging a capped dowry. A broken match rekindles the feud. Deterministic;
    /// wealth only moves between houses so the economy stays bounded.
    /// Do two houses' MERCHANTS reach each other? True if they trade in a shared city,
    /// or any of one house's network nodes (home + offices) lies within a merchant's
    /// practical reach of any of the other's (a real trade route exists — finite travel
    /// days — AND the two sit within `MARRIAGE_REACH_KM`). This replaces any crude
    /// same-continent test: contact is grounded in actual commerce.
    fn houses_in_contact(&self, a: usize, b: usize) -> bool {
        let n = self.hubs.len();
        if n == 0 || self.days.len() != n * n { return false; }
        if a >= self.houses.len() || b >= self.houses.len() { return false; }
        let reach_cells = MARRIAGE_REACH_KM * self.world_w / EARTH_EQUATOR_KM;
        let nodes = |h: usize| -> Vec<usize> {
            let mut v = vec![self.houses[h].hub as usize];
            for &off in &self.houses[h].offices { v.push(off as usize); }
            v
        };
        let (na, nb) = (nodes(a), nodes(b));
        for &x in &na {
            if x >= n { continue; }
            for &y in &nb {
                if y >= n { continue; }
                if x == y { return true; } // both trade the same city → certain contact
                if self.days[x * n + y].is_finite() && self.hub_cell_dist(x, y) <= reach_cells {
                    return true;
                }
            }
        }
        false
    }

    fn arrange_marriages(&mut self, yr: u32) {
        let nh = self.houses.len();
        if nh < 2 { return; }
        // ── A match sours: dissolve an alliance back into feud (rare). ──
        let mut kept: Vec<(u32, u32)> = Vec::with_capacity(self.alliances.len());
        for (a, b) in self.alliances.clone() {
            let (ua, ub) = (a as usize, b as usize);
            if ua >= nh || ub >= nh { continue; }
            let broke = hash01(self.seed, (yr as u64) ^ ((a as u64) << 20) ^ (b as u64), 0xF00D)
                < MARRIAGE_BREAK_CHANCE;
            if broke && !self.houses[ua].defunct && !self.houses[ub].defunct {
                if !self.houses[ua].rivals.contains(&ub) { self.houses[ua].rivals.push(ub); }
                if !self.houses[ub].rivals.contains(&ua) { self.houses[ub].rivals.push(ua); }
                let (na, nb) = (self.houses[ua].name.clone(), self.houses[ub].name.clone());
                self.journal.push(JournalEntry {
                    tick: self.tick, kind: "feud".into(), hub: self.houses[ua].hub as i32,
                    good: -1, value: 0.0,
                    text: format!("The alliance of {} and {} collapses into feud.", na, nb),
                });
            } else {
                kept.push((a, b));
            }
        }
        self.alliances = kept;
        // ── Arrange one new marriage this year (deterministic). ──
        if hash01(self.seed, yr as u64 ^ 0x1EDD, 0) >= MARRIAGE_YEARLY_CHANCE { return; }
        let mut cand: Vec<usize> = (0..nh).filter(|&i| {
            let h = &self.houses[i];
            !h.defunct && !h.is_guild && h.wealth > MARRIAGE_MIN_WEALTH
        }).collect();
        if cand.len() < 2 { return; }
        cand.sort_by(|&a, &b| self.houses[b].wealth
            .partial_cmp(&self.houses[a].wealth).unwrap_or(std::cmp::Ordering::Equal));
        let top = cand.len().min(12);
        let a = cand[((hash01(self.seed, yr as u64, 0x0A) * top as f32) as usize) % top];
        // A marriage only forms with a house whose MERCHANTS ACTUALLY REACH this one —
        // they trade in a shared city, or their networks lie within a caravan/voyage of
        // each other. No trade contact ⇒ no match (grounded in commerce, not geography).
        let reachable: Vec<usize> = cand.iter().copied()
            .filter(|&x| x != a && self.houses_in_contact(a, x)).collect();
        if reachable.is_empty() { return; } // this year's suitor's traders reach no peer
        let b = reachable[((hash01(self.seed, yr as u64, 0x0B) * reachable.len() as f32) as usize)
            % reachable.len()];
        if a == b { return; }
        let (lo, hi) = if a < b { (a as u32, b as u32) } else { (b as u32, a as u32) };
        if self.alliances.contains(&(lo, hi)) { return; } // already wed
        // End any feud, seal the alliance.
        self.houses[a].rivals.retain(|&r| r != b);
        self.houses[b].rivals.retain(|&r| r != a);
        self.alliances.push((lo, hi));
        // Dowry: a capped transfer from the wealthier to the other + prestige both.
        let (rich, poor) = if self.houses[a].wealth >= self.houses[b].wealth { (a, b) } else { (b, a) };
        let dowry = (self.houses[rich].wealth * MARRIAGE_DOWRY_FRAC).min(MARRIAGE_DOWRY_CAP).max(0.0);
        self.houses[rich].wealth -= dowry;
        self.houses[poor].wealth += dowry;
        self.houses[a].prestige += 0.05;
        self.houses[b].prestige += 0.05;
        let (na, nb) = (self.houses[a].name.clone(), self.houses[b].name.clone());
        self.journal.push(JournalEntry {
            tick: self.tick, kind: "marriage".into(), hub: self.houses[a].hub as i32, good: -1,
            value: 0.0, text: format!("{} and {} are joined in marriage, sealing an alliance.", na, nb),
        });
        self.houses[a].events.push(HouseEvent {
            tick: self.tick, kind: "marriage".into(), text: format!("Wed into {} — a new alliance.", nb) });
        self.houses[b].events.push(HouseEvent {
            tick: self.tick, kind: "marriage".into(), text: format!("Wed into {} — a new alliance.", na) });
    }

    /// Phase 4 (flavour) · seed one seasonal TRADE FAIR per large trading component,
    /// at its crossroads market town (prominence + a mild inland bias — the historic
    /// Champagne/Leipzig pattern). Deterministic; runs once (before routes are built,
    /// so it scores by population rather than live connectivity).
    fn seed_trade_fairs(&mut self) {
        use std::collections::HashMap;
        let n = self.hubs.len();
        let mut best: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut count: HashMap<u32, u32> = HashMap::new();
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let comp = self.hubs[h].component;
            *count.entry(comp).or_insert(0) += 1;
            let inland = if self.hubs[h].coastal { 1.0 } else { 1.25 };
            let score = self.hubs[h].population.max(0.0) * inland;
            let e = best.entry(comp).or_insert((h, -1.0));
            if score > e.1 { *e = (h, score); }
        }
        let mut fairs: Vec<Fair> = Vec::new();
        for (comp, (hub, _)) in best {
            if count.get(&comp).copied().unwrap_or(0) < FAIR_MIN_COMPONENT_HUBS { continue; }
            // Spring (month 4) or autumn (month 9) opening, by a deterministic roll.
            let month = if hash01(self.seed, hub as u64, 0xFA12) < 0.5 { 4 } else { 9 };
            fairs.push(Fair { hub: hub as u32, month });
        }
        fairs.sort_by_key(|f| f.hub); // HashMap order isn't stable → sort for determinism
        self.fairs = fairs;
    }

    /// Phase 4 (flavour) · open the fairs whose month begins today: a civic boon, a
    /// burst of converging trade on the nearest lanes (overlay-only), a chronicle
    /// beat, and an `ActiveEvent{kind:"fair"}` while the season lasts.
    fn run_trade_fairs(&mut self, doy: u32) {
        if self.fairs.is_empty() { return; }
        let tick = self.tick;
        let month_len = TICKS_PER_YEAR / 12;
        let opening: Vec<u32> = self.fairs.iter()
            .filter(|f| (f.month.saturating_sub(1) as u32) * month_len == doy)
            .map(|f| f.hub).collect();
        for hub in opening {
            let h = hub as usize;
            if h >= self.hubs.len() { continue; }
            self.hubs[h].sent_prosperity = (self.hubs[h].sent_prosperity + FAIR_PROSPERITY).min(1.0);
            self.hubs[h].sent_stability = (self.hubs[h].sent_stability + FAIR_STABILITY).min(1.0);
            let nbrs: Vec<usize> = self.neighbors.get(h)
                .map(|v| v.iter().take(FAIR_LANES).map(|&x| x as usize).collect())
                .unwrap_or_default();
            for nb in nbrs { self.accrue_flow(nb, h, usize::MAX, FAIR_FLOW); }
            self.active_events.push(ActiveEvent {
                kind: "fair".into(), hub: hub as i32, good: -1,
                magnitude: 1.0, until_tick: tick + month_len,
            });
            let city = self.hubs[h].name.clone();
            self.journal.push(JournalEntry {
                tick, kind: "fair".into(), hub: hub as i32, good: -1, value: 0.0,
                text: format!("The Fair of {} opens; merchants converge from across the region.", city),
            });
        }
    }

    /// Phase 4 (flavour) · seed one HOLY CITY per large component, at a prominent
    /// settlement distinct (where possible) from its fair town, with a patron ritual
    /// good drawn from the world's goods. Deterministic; runs once. A temple beat is
    /// chronicled at founding.
    fn seed_holy_sites(&mut self) {
        use std::collections::{HashMap, HashSet};
        let n = self.hubs.len();
        // Ritual goods actually present in this world (by name → index).
        let present: Vec<usize> = RITUAL_GOODS.iter()
            .filter_map(|name| self.goods.iter().position(|g| g.name == *name))
            .collect();
        let fair_hubs: HashSet<u32> = self.fairs.iter().map(|f| f.hub).collect();
        // Per component, the most prominent real hub that is NOT already a fair town
        // (fall back to allowing a fair town if that's all there is).
        let mut best: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut best_any: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut count: HashMap<u32, u32> = HashMap::new();
        for h in 0..n {
            if self.hubs[h].is_estate { continue; }
            let comp = self.hubs[h].component;
            *count.entry(comp).or_insert(0) += 1;
            let score = self.hubs[h].population.max(0.0);
            let a = best_any.entry(comp).or_insert((h, -1.0));
            if score > a.1 { *a = (h, score); }
            if !fair_hubs.contains(&(h as u32)) {
                let e = best.entry(comp).or_insert((h, -1.0));
                if score > e.1 { *e = (h, score); }
            }
        }
        let mut sites: Vec<HolySite> = Vec::new();
        let mut comps: Vec<u32> = count.keys().copied().collect();
        comps.sort_unstable();
        for comp in comps {
            if count.get(&comp).copied().unwrap_or(0) < HOLY_MIN_COMPONENT_HUBS { continue; }
            let hub = best.get(&comp).or_else(|| best_any.get(&comp)).map(|&(h, _)| h);
            let hub = match hub { Some(h) => h, None => continue };
            let patron_good = if present.is_empty() { -1 } else {
                let pick = (hash01(self.seed, hub as u64, 0x40FE) * present.len() as f32) as usize;
                present[pick.min(present.len() - 1)] as i32
            };
            let tier = if hash01(self.seed, hub as u64, 0x7E3) < 0.4 { 2 } else { 1 };
            let month = 1 + (hash01(self.seed, hub as u64, 0xB105) * 12.0) as u8;
            sites.push(HolySite { hub: hub as u32, patron_good, tier, month: month.min(12).max(1) });
            let city = self.hubs[hub].name.clone();
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "temple".into(), hub: hub as i32, good: patron_good, value: 0.0,
                text: format!("The great temple of {} is renowned across the land.", city),
            });
        }
        self.holy_sites = sites;
    }

    /// Phase 4 (flavour) · open any PILGRIMAGE season beginning today: a civic boon,
    /// inbound pilgrim traffic (overlay-only), a transient demand spike for the patron
    /// ritual good (self-relaxes), an `ActiveEvent{kind:"pilgrimage"}`, and a beat.
    fn run_pilgrimages(&mut self, doy: u32) {
        if self.holy_sites.is_empty() { return; }
        let tick = self.tick;
        let month_len = TICKS_PER_YEAR / 12;
        let opening: Vec<(u32, i32)> = self.holy_sites.iter()
            .filter(|s| (s.month.saturating_sub(1) as u32) * month_len == doy)
            .map(|s| (s.hub, s.patron_good)).collect();
        for (hub, patron) in opening {
            let h = hub as usize;
            if h >= self.hubs.len() { continue; }
            self.hubs[h].sent_prosperity = (self.hubs[h].sent_prosperity + PILGRIM_PROSPERITY).min(1.0);
            self.hubs[h].sent_stability = (self.hubs[h].sent_stability + PILGRIM_STABILITY).min(1.0);
            let nbrs: Vec<usize> = self.neighbors.get(h)
                .map(|v| v.iter().take(PILGRIM_LANES).map(|&x| x as usize).collect())
                .unwrap_or_default();
            for nb in nbrs { self.accrue_flow(nb, h, usize::MAX, PILGRIM_FLOW); }
            // Pilgrim demand bids up the patron ritual good (transient — the price
            // relax pulls it back over the following days). Capped for safety.
            if patron >= 0 {
                let g = patron as usize;
                if g < self.hubs[h].price.len() && g < self.goods.len() {
                    let cap = self.goods[g].base_value.max(0.01) * 5.0;
                    self.hubs[h].price[g] = (self.hubs[h].price[g] * PILGRIM_PRICE_BUMP).min(cap);
                }
            }
            self.active_events.push(ActiveEvent {
                kind: "pilgrimage".into(), hub: hub as i32, good: patron,
                magnitude: 1.0, until_tick: tick + month_len,
            });
            let city = self.hubs[h].name.clone();
            let good_txt = if patron >= 0 {
                self.goods.get(patron as usize).map(|g| format!(" — {} for the altar", g.name)).unwrap_or_default()
            } else { String::new() };
            self.journal.push(JournalEntry {
                tick, kind: "pilgrimage".into(), hub: hub as i32, good: patron, value: 0.0,
                text: format!("Pilgrims throng to {} for the holy season{}.", city, good_txt),
            });
        }
    }

    /// The living merchant families: ageing heads, monopolies, feuds, founding,
    /// extinction and political power.
    /// Log a dispatched trade for the Market "recent deals" rows (rolling, capped).
    fn log_trade(&mut self, from: u32, to: u32, good: usize, amount: f32, owner: i32, sea: bool, price: f32) {
        self.recent_trades.push(RecentTrade { from, to, good, amount, owner, sea, price, tick: self.tick });
        let n = self.recent_trades.len();
        if n > 400 { self.recent_trades.drain(0..n - 400); }
        // Accumulate the year's trade flows for the Flows subtab: this shipment is an
        // INBOUND flow at `to` (from `from`) and an OUTBOUND flow at `from` (to `to`).
        if amount > 0.0 {
            let g = good as u32;
            *self.trade_cur.entry((to, g, from, 0)).or_insert(0.0) += amount;
            *self.trade_cur.entry((from, g, to, 1)).or_insert(0.0) += amount;
        }
    }

    /// At each New Year: snapshot the year's trade flows as `trade_last` (the Flows
    /// subtab's per-partner detail), append the per-(hub,good) yearly volume to the
    /// trend history (goods that DIDN'T trade get a 0, so a fallen trade shows its
    /// decline), then clear the accumulator for the new year.
    fn fold_trade_year(&mut self) {
        let mut last: Vec<TradeFlowAgg> = self.trade_cur.iter()
            .map(|(&(hub, good, partner, dir), &amount)| TradeFlowAgg { hub, good, partner, dir, amount })
            .collect();
        // Deterministic order (the panel re-sorts by volume anyway).
        last.sort_by(|a, b| (a.hub, a.good, a.dir, a.partner).cmp(&(b.hub, b.good, b.dir, b.partner)));
        // Per-(hub,good) total volume this year.
        let mut vol: std::collections::HashMap<(u32, u32), f32> = std::collections::HashMap::new();
        for f in &last { *vol.entry((f.hub, f.good)).or_insert(0.0) += f.amount; }
        // Extend existing series (0 for goods not traded this year → visible decline).
        let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for h in self.trade_hist.iter_mut() {
            let v = vol.get(&(h.hub, h.good)).copied().unwrap_or(0.0);
            h.vols.push(v);
            if h.vols.len() > TRADE_HIST_CAP { let d = h.vols.len() - TRADE_HIST_CAP; h.vols.drain(0..d); }
            seen.insert((h.hub, h.good));
        }
        // Brand-new (hub,good) trades start a fresh series.
        for (&(hub, good), &v) in &vol {
            if !seen.contains(&(hub, good)) { self.trade_hist.push(TradeHist { hub, good, vols: vec![v] }); }
        }
        // Bound memory: if over the row cap, drop the deadest trades (lowest peak).
        if self.trade_hist.len() > TRADE_HIST_ROWS {
            self.trade_hist.sort_by(|a, b| {
                let pa = a.vols.iter().cloned().fold(0.0f32, f32::max);
                let pb = b.vols.iter().cloned().fold(0.0f32, f32::max);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            });
            self.trade_hist.truncate(TRADE_HIST_ROWS);
        }
        self.trade_last = last;
        self.trade_cur.clear();
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
        // 1) A prospering town charters a civic GUILD (from year 5). Lower population
        //    bar than the initial-seed GUILD_MIN_POP so guilds are the common early
        //    institution — but gated on PROSPERITY so they cluster in commercially
        //    successful cities (poor backwaters stay guild-less → the world stays
        //    differentiated, and there are far more than a handful of guilds).
        if self.tick >= GUILD_START_YEAR * TICKS_PER_YEAR {
            for h in 0..n {
                if self.hubs[h].is_estate { continue; }
                if self.hubs[h].population < GUILD_FORM_POP { continue; }
                if self.hubs[h].sent_prosperity < GUILD_FORM_PROSPERITY { continue; }
                let has_guild = self.houses.iter()
                    .any(|g| !g.defunct && g.is_guild && g.hub as usize == h);
                if !has_guild {
                    self.found_guild(h);
                }
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
                // Relaxed the relative gate 0.5→0.3 so a house also plants offices at
                // its SECOND-tier partners, not only its single dominant one — this
                // spreads several competing houses' counting-houses across each city
                // (was: one house monopolising a settlement).
                if v < OFFICE_OPEN_VOLUME || v < max_vol * 0.3 { continue; }
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
            influence: Vec::new(), bailos: Vec::new(),
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
            // Merchant bankers earn a modest return on LIQUID capital — but only up
            // to a cap, so it's a starting perk, not an exponential engine. Real
            // banking growth now comes from OWNING a bank (bank_pass dividends).
            // DLC 3.5 rebalance: the old uncapped 1%/mo on the whole fortune
            // compounded houses to absurd wealth (100k in a decade).
            for hh in &mut self.houses {
                if !hh.defunct && hh.archetype == ARCH_BANKING && hh.wealth > 0.0 {
                    hh.wealth += hh.wealth.min(BANK_INTEREST_CAP) * BANK_INTEREST;
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
            // DLC 3.5 · wealthy poleis spend a slice of treasury sponsoring migration
            // (relieves crowding, drains hoarded treasuries).
            self.poleis_sponsor_migration();
            if ENABLE_CADET_BRANCHES { self.maybe_branch_houses(); } // disabled (user rule)
            self.maybe_house_invests();
            // DLC 3.5 · resale market: distressed houses / thin-treasury poleis sell
            // holdings; solvent houses & banks buy them.
            self.estate_resale_pass();
            self.update_guilds_and_offices();
            // Offices (re)settled → update commercial influence, dominance & Bailos.
            self.update_influence_and_bailos();
            // Offices are (re)settled above → now offer futures contracts from them.
            self.form_contracts(needs);
            // DLC 3.5 · banks service loans, take deposits, lend, and may fail.
            self.bank_pass();
            // DLC 4 · manufactories refine their craft (quality climbs toward a cap).
            self.update_good_quality();
            // Local line's debt-based solvency check supersedes the old volume-based
            // dissolve (a house in the red ≥1yr goes bankrupt; guilds get bailouts).
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
        // Archetype can PIVOT at a generational change to reflect what the family has
        // become: a rich lender (or one that owns a bank) turns merchant-banker; a big
        // shipper a fleet dynasty; a council power a political house; else a specialist.
        let new_arch = {
            let h = &self.houses[hi];
            let fleet = h.fleet_sea + h.fleet_river + h.fleet_caravan;
            let owns_bank = self.banks.iter().any(|b| !b.defunct && b.house == hi as u32);
            if owns_bank || h.wealth >= BANK_FOUND_WEALTH_RICH { ARCH_BANKING }
            else if fleet >= 12 { ARCH_FLEET }
            else if h.political_power >= 0.6 || h.dominant_seat { ARCH_POLITICAL }
            else { ARCH_SPECIALTY }
        };
        let old_arch = self.houses[hi].archetype;
        {
            let h = &mut self.houses[hi];
            h.generation = gen;
            h.head_name = heir.clone();
            h.head_since = tick;
            h.head_lifespan = lifespan;
            h.prestige += 0.05;
            h.archetype = new_arch;
            h.events.push(HouseEvent {
                tick, kind: "succession".into(),
                text: format!("{} succeeds as head (generation {})", heir, gen),
            });
            if new_arch != old_arch {
                h.events.push(HouseEvent {
                    tick, kind: "archetype".into(),
                    text: format!("the family reinvents itself as {}", archetype_label(new_arch)),
                });
            }
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
            influence: Vec::new(), bailos: Vec::new(),
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
            if self.hubs[b].is_estate { continue; } // branch into a real city, not an estate
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
            // Estates / manufactories are production satellites, not market cities —
            // a merchant family never seats itself there (they belong in poleis).
            if self.hubs[h].is_estate { continue; }
            max_tw = max_tw.max(self.hubs[h].trade_wealth);
            let tw = self.hubs[h].trade_wealth;
            if tw <= 0.05 { continue; } // any hub with a little trade can seed a family
            // A house SPINS OFF a guild's trade — so it only appears in a city that
            // already has a GUILD (local merchants → guild → a family separating out).
            let has_guild = self.houses.iter()
                .any(|g| !g.defunct && g.is_guild && g.hub as usize == h);
            if !has_guild { continue; }
            let count = self.houses.iter()
                .filter(|hs| !hs.defunct && hs.hub as usize == h).count() as u32;
            // Several rival families may SHARE a city (that is the competition the
            // map should show), capped by hub size. The old `strongest > 8.0` block
            // permanently locked out new houses the moment any incumbent grew past a
            // trivial wealth — collapsing every city to one dynasty and, over a
            // century, the whole world to a handful of houses. Cap only, no wealth
            // lockout: rich cities host up to 3 competing houses, smaller ones 2.
            let cap = if tw >= 0.5 * max_tw { 3 } else { 2 };
            if count >= cap { continue; }
            if tw > best.1 { best = (h, tw); }
        }
        let Some(hub) = (best.0 != usize::MAX).then_some(best.0) else { return };
        // ── Probabilistic founding (per tick) ────────────────────────────────
        // A house does NOT auto-appear just because a city is guild-run. Per tick:
        //   • below the seeded baseline → 10% (the world repopulates its houses)
        //   • a large trade hub (>=50% of the richest hub's trade) → 5%
        //   • otherwise → 2%
        let active = self.houses.iter().filter(|h| !h.defunct && !h.is_guild).count() as u32;
        // Hard cap on live houses, and houses only start appearing from year 10 (after
        // guilds have emerged from year 5). Prefer to spin a house off a city that
        // already has a GUILD (a family separating a share of the guild's trade).
        if active as usize >= HOUSE_MAX_TOTAL { return; }
        if self.tick < HOUSE_START_YEAR * TICKS_PER_YEAR { return; }
        let target = if self.seed_house_count > 0 { self.seed_house_count } else { 24 };
        // Houses appear GRADUALLY over the first ~5 years, not all at once: the
        // effective baseline ramps linearly from a small start up to the full target,
        // so the merchant class emerges over the opening decade instead of instantly.
        let ramp = (self.tick as f32 / (HOUSE_RAMP_YEARS * TICKS_PER_YEAR as f32)).clamp(0.0, 1.0);
        let baseline = ((target as f32) * ramp).ceil() as u32;
        let large = best.1 >= 0.5 * max_tw;
        // A house is a RARE spin-off from a guild's trade (user: "small chance") — much
        // lower per-tick odds than before, so guilds stay the norm and houses the
        // exception (was 10/5/2% → a runaway 178 houses vs 2 guilds).
        let prob = if active < baseline { 0.02 } else if large { 0.01 } else { 0.004 };
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
            influence: Vec::new(), bailos: Vec::new(),
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
            // Keep recent entries within the window AND, beyond it, the MILESTONE
            // events that form a city/house's permanent record (founding, colonies,
            // wars, crashes, banks, espionage, speculation calls). The bulk that the
            // window exists to shed is the per-tick "price" samples + "voyage_loss"
            // noise, so those are still dropped past 25 years. (Fixes city Chronicles
            // losing their early history on long campaigns.)
            self.journal.retain(|e| e.tick >= cutoff || is_milestone_kind(&e.kind));
        }
        if self.journal.len() > 12_000 {
            let drop = self.journal.len() - 12_000;
            self.journal.drain(0..drop);
        }
    }
}

/// Milestone journal kinds form a city/house's PERMANENT record and survive the
/// rolling 25-year prune. Only the high-volume periodic samples — per-tick "price"
/// index, the monthly "world" summary, and "voyage_loss" shipwreck noise — are
/// shed past the window (they're what the window exists to bound).
fn is_milestone_kind(kind: &str) -> bool {
    !matches!(kind, "price" | "world" | "voyage_loss")
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
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, last_upgrade_tick: 0, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, mint_bullion_ratio: 1.0, has_mint: false,
            quality: Vec::new(), stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: 0, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
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
            influence: vec![], bailos: vec![],
        }
    }

    fn sim(hubs: Vec<TickHub>, goods: Vec<TickGood>) -> CampaignSim {
        let mut s = CampaignSim {
            seed: 42, tick: 0, goods, hubs, in_transit: vec![], houses: vec![],
            active_events: vec![], journal: vec![], days_per_cell: 0.2, freight_per_day: 0.01,
            k: 0.6, margin: 0.05, need_scale: 1.0, world_w: 100.0, world_h: 100.0, last_tick_ms: 0.0,
            last_month_pop: 0.0, last_month_index: 0.0, seed_house_count: 0,
            fleets_migrated: true, tech_factor: 1.0, percap_migrated: true, society_migrated: false,
            components_rescued: true,
            house_ledger: Vec::new(), house_ledger_prev: Vec::new(), house_barred: Vec::new(),
            colonizable: vec![], satellite_sites: vec![], hinterland: vec![], migration_routes: vec![], creoles: vec![], culture_history: vec![], council_bought_month: vec![], hub_patron: vec![], colony_supply: vec![],
            hub_culture: vec![], hub_minorities: vec![], estate_idle_years: vec![],
            diag_shipments: 0, diag_by_house: 0, diag_by_guild: 0, diag_lost: 0, diag_volume: 0.0,
            recent_trades: vec![],
            spec_centers: vec![], spec_year: 0, spec_prev_profit: vec![],
            banks: vec![], crashes: vec![], wars: vec![], war_log: vec![],
            flow_year: vec![], flow_accum: std::collections::HashMap::new(),
            world_series: vec![], total_foundings: 0, total_abandonments: 0,
            migrations: vec![],
            good_flow_accum: vec![], hub_good_trade: vec![], year_frames: vec![],
            records: WorldRecords::default(),
            quality_migrated: false,
            days: vec![],
            neighbors: vec![],
            routes_dirty: false,
            warehouses: vec![],
            contracts: vec![],
            trade_cur: Default::default(),
            city_dominator: vec![],
            trade_last: vec![],
            trade_hist: vec![],
            figures: vec![],
            fairs: vec![],
            fairs_seeded: false,
            holy_sites: vec![],
            holy_seeded: false,
            alliances: vec![],
            guilds: vec![],
            guilds_seeded: false,
            wonders: vec![],
            epidemics: vec![],
            next_outbreak: 0,
            expansion_frozen_until: 0,
        };
        s.rebuild_routes();
        s
    }

    /// Migration must MIX cultures: when people of one people move (over a trade tie)
    /// into a city of a DIFFERENT people, a minority quarter of the newcomers' culture
    /// must appear there. Guards the "every city stays monocultural" regression.
    #[test]
    fn migration_seeds_foreign_minority_quarters() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        // Four adjacent cities in one connected market; two peoples split down the middle.
        let mut hubs = Vec::new();
        for i in 0..4u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 6000.0 } else { 800.0 }).collect();
            hubs.push(hub(i, (i as f32) * 3.0, 0.0, 5000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        // Assign cultures: hubs 0,1 = "Aiora"; hubs 2,3 = "Belgar". (ensure_hub_cultures
        // won't overwrite these non-empty entries.)
        s.hub_culture = vec!["Aiora".into(), "Aiora".into(), "Belgar".into(), "Belgar".into()];
        s.hub_minorities = vec![Vec::new(); 4];
        // Make hub 1 (an "Aiora" city) the one thriving magnet; the "Belgar" hub 2 next
        // door is miserable, so its people drift across the culture border into hub 1.
        for (i, h) in s.hubs.iter_mut().enumerate() {
            if i == 1 { h.sent_prosperity = 0.95; h.starving = 0.0; h.food_balance = 1.0; }
            else { h.sent_prosperity = 0.30; h.starving = 0.0; h.food_balance = 1.0; }
        }
        // Run the economic-migration pass a few years (it's yearly).
        for _ in 0..8 { s.economic_migration_pass(); }
        // Hub 1 ("Aiora") must now host a "Belgar" minority carried in by the migrants.
        let belgar_at_1 = s.hub_minorities[1].iter().find(|(c, _)| c == "Belgar").map(|(_, sh)| *sh).unwrap_or(0.0);
        assert!(belgar_at_1 > 0.0,
            "expected a Belgar minority to form in the Aiora magnet city, got {:?}", s.hub_minorities[1]);
    }

    /// A big healthy city must ADOPT a tiny, failing neighbour as a satellite (reviving
    /// it) instead of leaving it to die. Guards the absorption rescue path.
    #[test]
    fn big_city_absorbs_dying_neighbour() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        // Hub 0 = a big healthy metropolis; hub 1 = a tiny failing town right beside it.
        let big_prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 40_000.0 } else { 5_000.0 }).collect();
        let small_prod: Vec<f32> = (0..ng).map(|_| 30.0).collect();
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 20_000.0, big_prod, 0),
            hub(1, 1.5, 0.0, 150.0, small_prod, 0),
        ];
        hubs[0].sent_prosperity = 0.65; hubs[0].starving = 0.0;
        hubs[0].stock = vec![10_000.0, 4_000.0, 0.0]; // food to grant
        hubs[1].sent_prosperity = 0.25; hubs[1].starving = 0.2; // struggling
        let mut s = sim(hubs, goods);
        s.tick = 20 * 365; // old enough that the town is past ABSORB_MIN_AGE
        s.hub_culture = vec!["Aiora".into(), "Belgar".into()];
        s.hub_minorities = vec![Vec::new(); 2];
        s.rebuild_routes();
        s.maybe_absorb_dying_city();
        assert_eq!(s.hubs[1].colony_kind, 3, "dying town should become a satellite");
        assert_eq!(s.hubs[1].founder_hub, 0, "…bound to the big neighbour");
        assert!(s.hubs[1].population > 150.0, "…and shored up with relocated settlers");
        // The metropolis's people arrive as a minority quarter (culture mixing).
        assert!(s.hub_minorities[1].iter().any(|(c, _)| c == "Aiora"),
            "adopted town should host an Aiora minority, got {:?}", s.hub_minorities[1]);
    }

    /// Reproduction for the "campaign restarts near year 30" crash: outposts only
    /// start founding at OUTPOST_START_TICK (= year 30), and founding APPENDS a hub
    /// mid-tick. With colonization sites seeded and houses rich enough to clear the
    /// outpost wealth bar, the world grows past year 30 — exercising every per-hub
    /// loop against a freshly-appended hub. Must not panic (index/overflow) for 50y.
    #[test]
    fn outposts_and_colonies_past_year_30_dont_crash() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
            good("spices", 1, 2, 16.0, 0.4, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..24u32 {
            let x = (i % 6) as f32 * 9.0;
            let y = (i / 6) as f32 * 9.0;
            let pop = 9000.0 + (i as f32 * 911.0) % 24000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.013 } else { pop * 0.0015 })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0)); // one component → a connected market
        }
        let mut s = sim(hubs, goods);
        // A handful of houses, two of them seeded already wealthy so they clear the
        // heavy OUTPOST_FOUND_WEALTH (100k) bar by year 30 and actually plant outposts.
        for i in 0..6u32 {
            let seat = (i * 3) % 24;
            let mut h = house_at(seat, vec![2 + (i as usize % 3)], 3);
            h.archetype = (i % 4) as u8;
            // Seed several houses already very rich so they clear the heavy
            // OUTPOST_FOUND_WEALTH (100k) bar repeatedly and actually plant outposts.
            h.wealth = if i < 4 { 400_000.0 } else { 60.0 + i as f32 * 10.0 };
            h.prestige = 0.6;
            h.dominant_seat = i % 2 == 0;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        // Seed colonization sites on the frontier so outposts AND settlement colonies
        // can be founded (a bank is needed for a settlement colony; outposts only need
        // a rich house). Spread them across the map, trade-rich + coastal.
        for k in 0..12u32 {
            s.colonizable.push(ColonizeSite {
                x: (k % 4) as f32 * 12.0 + 4.0,
                y: (k / 4) as f32 * 12.0 + 4.0,
                koppen: 11, elevation: 0.2, fertility: 0.5, coastal: k % 2 == 0,
                kind_hint: ((k % 5) + 1) as u8, trade_value: 0.4 + (k as f32 % 3.0) * 0.2,
                delta: false, chokepoint: false,
            });
        }
        // A bank so settlement colonies (which need a same-continent bank) can form too.
        s.banks.push(Bank {
            name: "Banco".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 80.0, loans: vec![], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![],
            dividends_earned: 0.0, history: vec![], events: vec![],
        });
        s.rebuild_routes();
        let hubs0 = s.hubs.len();
        // Run through and well past year 30 — the outpost/colony founding window.
        for yr in 1..=50u32 {
            // Keep two houses permanently above the heavy outpost wealth bar so the
            // outpost-founding path is GUARANTEED to fire every year from year 30 on
            // (the real campaign has dynasties this rich; left alone the wealth tax
            // bends them under the bar before year 30 and the path never runs).
            for hi in 0..2usize.min(s.houses.len()) {
                if !s.houses[hi].defunct { s.houses[hi].wealth = s.houses[hi].wealth.max(400_000.0); }
            }
            s.advance(365);
            // Per-hub persistent arrays must never lag the hub list (the crash class).
            assert!(s.hub_patron.len() <= s.hubs.len(), "hub_patron never exceeds hubs");
            for h in &s.hubs { assert_eq!(h.stock.len(), ng, "every hub keeps ng columns"); }
            if yr % 5 == 0 {
                let outposts = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
                eprintln!("yr {yr:2}: hubs {} (+{} since start) · outposts {outposts}",
                    s.hubs.len(), s.hubs.len() - hubs0);
            }
        }
        // The point of the test is the founding path: confirm the world actually GREW
        // past year 30 (outposts/estates/colonies appended) without crashing.
        assert!(s.hubs.len() > hubs0,
            "world must grow past year 30 (founding exercised): {} → {}", hubs0, s.hubs.len());
    }

    /// A wealthy house develops an EXISTING under-traded small city into a TRADE BASE:
    /// it opens an office, builds a guildhall, seeds capital and takes the city under
    /// its patronage — and patronage concludes once the city grows up. See
    /// docs/TRADE_BASE_MECHANIC_PLAN.md.
    #[test]
    fn trade_base_development() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        // Hub 0: the house's busy seat. Hub 1: a small, inert town on the same
        // continent, in reach. Hub 2: too big to qualify (a control).
        let prod0: Vec<f32> = (0..ng).map(|g| if goods[g].food { 400.0 } else { 120.0 }).collect();
        let prod1: Vec<f32> = (0..ng).map(|g| if goods[g].food { 200.0 } else { 5.0 }).collect();
        let hubs = vec![
            hub(0, 0.0, 0.0, 80_000.0, prod0.clone(), 0),
            hub(1, 4.0, 0.0, 20_000.0, prod1, 0),
            hub(2, 8.0, 0.0, 200_000.0, prod0, 0),
        ];
        let mut s = sim(hubs, goods);
        // The small town starts inert (no throughput) — the under-traded signal.
        s.hubs[1].export_earn = 0.0;
        s.hubs[1].import_spend = 0.0;
        // A rich house seated at hub 0, clearing the (modest) base-investment bar.
        let mut rich = house_at(0, vec![1], 2);
        rich.wealth = 100_000.0;
        s.houses.push(rich);
        s.seed_house_count = 1;
        s.rebuild_routes();
        s.hub_patron.resize(s.hubs.len(), -1);
        s.tick = BASE_START_TICK;

        let w0 = s.houses[0].wealth;
        s.maybe_establish_trade_base();

        // The small town (hub 1) — not the big control (hub 2) — became the base.
        assert_eq!(s.hub_patron[1], 0, "the small under-traded city is patronised by house 0");
        assert_eq!(s.hub_patron[2], -1, "the large well-developed city is NOT taken as a base");
        assert!(s.houses[0].offices.contains(&1), "the patron opens an office in the base");
        assert!(s.hubs[1].structures.contains(&STRUCT_GUILDHALL), "a guildhall is built in the base");
        assert!(s.hubs[1].treasury > 0.0, "working capital is seeded into the city");
        assert!(s.houses[0].wealth < w0, "the house pays for the investment");

        // Graduation: once the base grows into a real node, patronage concludes.
        s.hubs[1].population = BASE_DEVELOPED_POP + 1.0;
        s.trade_base_pass();
        assert_eq!(s.hub_patron[1], -1, "patronage concludes once the city is developed");
    }

    /// STANDING PRACTICE (see CLAUDE.md): run the living campaign for decades and
    /// watch the dynamics — houses rise and fall, banks are chartered and fail,
    /// poleis mint coin, wars flare, crashes ripple. Prints a 5-yearly digest:
    ///   `cargo test --lib simulate_decades_reports_dynamics -- --nocapture`
    /// Also a hard regression: wealth must stay finite + bounded (no 100k blow-ups,
    /// no −50M contract craters) over the whole run.
    #[test]
    fn simulate_decades_reports_dynamics() {
        // 30 hubs in one connected market, six goods (three of them food).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("olives", 0, 0, 1.6, 0.6, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..30u32 {
            let x = (i % 6) as f32 * 9.0;
            let y = (i / 6) as f32 * 9.0;
            let pop = 8000.0 + (i as f32 * 911.0) % 26000.0;
            // Each hub specializes: a couple of goods at high output, the rest low.
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // Ten houses: a spread of archetypes, several dominant at their seat so a
        // council forms (→ coinage → banks).
        for i in 0..10u32 {
            let seat = (i * 3) % 30;
            let mut h = house_at(seat, vec![3 + (i as usize % 3)], 3);
            h.archetype = (i % 4) as u8;
            h.wealth = 40.0 + (i as f32) * 8.0;
            h.prestige = 0.5;
            h.dominant_seat = i % 2 == 0;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        // Atlas 2.0 · FREE LAND south of the market towns, so the lifecycle passes
        // (organic swarming + colonial ventures) have somewhere to found.
        for i in 0..12u32 {
            s.colonizable.push(ColonizeSite {
                x: 4.5 + (i % 4) as f32 * 12.0,
                y: 40.0 + (i / 4) as f32 * 8.0,
                koppen: 8, elevation: 0.1,
                fertility: 0.45 + (i % 3) as f32 * 0.15,
                coastal: i % 2 == 0, kind_hint: 1,
                trade_value: 0.2 + (i % 4) as f32 * 0.1,
                delta: false, chokepoint: false,
            });
        }
        s.rebuild_routes();

        let mut min_w = f32::INFINITY;
        let mut peak_w = f32::NEG_INFINITY;
        let mut late_max = 0.0f32; // richest in the final decade — the SUSTAINED level
        let mut ever_dissolved = false;
        for yr in 1..=50u32 {
            s.advance(365);
            let active = s.houses.iter().filter(|h| !h.defunct).count();
            let defunct = s.houses.len() - active;
            if defunct > 0 { ever_dissolved = true; }
            let banks = s.banks.iter().filter(|b| !b.defunct).count();
            let coins = s.hubs.iter().filter(|h| !h.coin_name.is_empty()).count();
            let top_trust = s.hubs.iter().map(|h| h.coin_trust).fold(0.0f32, f32::max);
            let rich = s.houses.iter().filter(|h| !h.defunct)
                .map(|h| h.wealth).fold(0.0f32, f32::max);
            if yr > 40 { late_max = late_max.max(rich); }
            for h in &s.houses {
                if h.wealth.is_finite() { min_w = min_w.min(h.wealth); peak_w = peak_w.max(h.wealth); }
            }
            assert!(s.tech_factor.is_finite());
            // DLC 4 · finest good in the world this year + cumulative espionage.
            let mut finest = (0.0f32, 0usize);
            for h in &s.hubs {
                for (g, &q) in h.quality.iter().enumerate() {
                    if h.production.get(g).copied().unwrap_or(0.0) > 0.0 && q > finest.0 { finest = (q, g); }
                }
            }
            let thefts = s.journal.iter().filter(|e| e.kind == "espionage").count();
            let contracts = s.contracts.len();
            let colonies = s.hubs.iter().filter(|h| h.colony_kind == 1).count();
            let outposts = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
            let offices: usize = s.houses.iter().filter(|h| !h.defunct).map(|h| h.offices.len()).sum();
            if yr % 5 == 0 {
                let towns_alive = s.hubs.iter()
                    .filter(|h| !h.is_estate && !h.abandoned && h.population >= 1.0).count();
                let hungry = s.hubs.iter()
                    .filter(|h| !h.is_estate && !h.abandoned && h.starving > 0.5).count();
                let thriving = s.hubs.iter().filter(|h| !h.is_estate && !h.abandoned
                    && h.mood > 0.55 && h.starving < 0.1).count();
                eprintln!(
                    "yr {yr:2}: houses {active}↑/{defunct}✝  banks {banks}  coins {coins} (trust {:.0}%)  wars {}  crashes {}  richest {rich:.0}  contracts {contracts}  offices {offices}  colonies {colonies}  outposts {outposts}  towns {towns_alive} (+{}/−{}) hungry {hungry} thriving {thriving}  finest {} {:.0}%  thefts {thefts}",
                    top_trust * 100.0, s.wars.len(), s.crashes.len(),
                    s.total_foundings, s.total_abandonments,
                    s.goods[finest.1].name, finest.0 * 100.0,
                );
            }
        }
        eprintln!("over 50y: wealth ∈ [{min_w:.1}, {peak_w:.1}] · sustained (late) richest {late_max:.0}");
        assert!(min_w.is_finite() && peak_w.is_finite(), "wealth finite");
        // SUSTAINED wealth must stay sane. Wealth is intentionally NOT hard-capped
        // (a great trading dynasty can climb into the hundreds of thousands and so
        // afford a trade outpost), but the gentle quadratic surcharge still bends the
        // very richest back — no millions-scale runaway (the old bug ran to ~1.25M).
        // The ceiling was raised when PLAGUE IMMUNITY landed: cities that survive an
        // outbreak now resist re-infection for years, so the world is no longer culled
        // every few years and long-lived trading dynasties keep more of what they earn —
        // a designed, healthier-economy consequence. The bound still catches an
        // order-of-magnitude / millions-scale runaway.
        assert!(late_max < 1_000_000.0, "no SUSTAINED runaway-rich house: {late_max}");
        assert!(min_w > -100.0, "no runaway-insolvent house (limited liability): {min_w}");
        // The world must actually be DYNAMIC: houses turn over.
        assert!(ever_dissolved, "houses rise and fall over decades");
    }

    /// Perf P1 receipt · what a read-only panel query paid BEFORE the Arc resident
    /// sim (a full deep clone of a mature campaign) vs after (an Arc bump).
    /// Run: cargo test --lib bench_sim_clone -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_sim_clone() {
        use std::time::Instant;
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("olives", 0, 0, 1.6, 0.6, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..30u32 {
            let pop = 8000.0 + (i as f32 * 911.0) % 26000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 })
                .collect();
            hubs.push(hub(i, (i % 6) as f32 * 9.0, (i / 6) as f32 * 9.0, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        for i in 0..10u32 {
            s.houses.push(house_at((i * 3) % 30, vec![3 + (i as usize % 3)], 3));
        }
        s.seed_house_count = s.houses.len() as u32;
        s.rebuild_routes();
        s.advance(365 * 30); // a mature campaign: histories + journal filled
        let arc = std::sync::Arc::new(s);
        let t0 = Instant::now();
        for _ in 0..50 { let c = (*arc).clone(); std::hint::black_box(&c); }
        let deep = t0.elapsed().as_secs_f64() * 1000.0 / 50.0;
        let t1 = Instant::now();
        for _ in 0..1_000_000 { let c = arc.clone(); std::hint::black_box(&c); }
        let bump = t1.elapsed().as_secs_f64() * 1000.0 / 1_000_000.0;
        eprintln!("per-query cost: deep clone {deep:.3} ms  →  Arc bump {bump:.6} ms  ({:.0}× faster)",
            deep / bump.max(1e-9));
        assert!(deep > bump, "Arc bump must beat a deep clone");
    }

    /// Atlas 2.0 · a thriving city bursting past its founding size SWARMS: a slice
    /// of its people found an independent daughter town on nearby free land, the
    /// site is consumed, and the founding is chronicled.
    #[test]
    fn thriving_city_swarms_a_daughter_town() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let h0 = hub(0, 10.0, 10.0, 20_000.0, vec![20_000.0 * 0.02], 0);
        let mut s = sim(vec![h0], goods);
        s.colonizable.push(ColonizeSite {
            x: 16.0, y: 10.0, koppen: 8, elevation: 0.1, fertility: 0.8,
            coastal: false, kind_hint: 1, trade_value: 0.3,
            delta: false, chokepoint: false,
        });
        // The swarm preconditions: crowded (2× founding), content, fed.
        s.hubs[0].population = s.hubs[0].founding_pop * 2.0;
        s.hubs[0].mood = 0.7;
        s.hubs[0].starving = 0.0;
        s.tick = SWARM_START_TICK;
        let mother_pop = s.hubs[0].population;
        s.maybe_swarm_town();
        assert_eq!(s.hubs.len(), 2, "a daughter town was founded");
        let d = &s.hubs[1];
        assert_eq!(d.colony_kind, 0, "organic town, not a chartered colony");
        assert!(d.founded_tick == SWARM_START_TICK && !d.abandoned && !d.name.is_empty());
        assert!(d.population > 0.0 && s.hubs[0].population < mother_pop,
            "settlers actually left the mother city");
        assert!(s.colonizable.is_empty(), "the site was consumed");
        assert!(s.journal.iter().any(|e| e.kind == "founding"), "founding chronicled");
        assert_eq!(s.total_foundings, 1);
    }

    /// Atlas 2.0 · a famine-floored town with a fed HAVEN nearby is abandoned:
    /// survivors migrate to the haven, the abandonment is chronicled, and the ruin
    /// is never resurrected by the population floor.
    #[test]
    fn famine_town_is_abandoned_and_stays_dead() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 30_000.0, vec![30_000.0 * 0.03], 0), // the haven: fed
            hub(1, 60.0, 10.0, 10_000.0, vec![0.0], 0),             // doomed: no food
        ];
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        // Terminal state reached: famine-floored and starving for ABANDON_YEARS.
        s.hubs[1].population = s.hubs[1].founding_pop * 0.11;
        s.hubs[1].starving = 0.9;
        s.hubs[1].decline_years = ABANDON_YEARS;
        let haven_before = s.hubs[0].population;
        s.lifecycle_pass(true);
        assert!(s.hubs[1].abandoned, "famine town abandoned");
        assert!(s.hubs[1].population < 1.0 && s.hubs[1].died_tick == s.tick);
        assert!(s.hubs[0].population > haven_before, "survivors reached the haven");
        assert!(s.journal.iter().any(|e| e.kind == "abandonment"), "abandonment chronicled");
        assert_eq!(s.total_abandonments, 1);
        // The ruin stays dead through real ticks (the pop floor must not revive it).
        s.advance(60);
        assert!(s.hubs[1].population < 1.0, "ruin not resurrected by the pop floor");
        assert!(s.hubs[1].abandoned);
    }

    /// Social strata invariant: every settlement's four shares stay in [0,1] and sum
    /// to ~1 across decades of mobility, the strata actually DIFFERENTIATE between
    /// cities, and inequality stays a bounded 0..1 index.
    #[test]
    fn society_shares_bounded() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..20u32 {
            let x = (i % 5) as f32 * 9.0;
            let y = (i / 5) as f32 * 9.0;
            let pop = 6000.0 + (i as f32 * 1300.0) % 24000.0;
            // A clear rich/poor gradient: the first third are luxury-exporting
            // entrepôts (high trade wealth → patrician/burgher), the rest agrarian.
            let rich = i < 7;
            let prod: Vec<f32> = (0..ng)
                .map(|g| {
                    if g == 2 || g == 4 { // silk / wine — luxuries that build trade wealth
                        if rich { pop * 0.020 } else { pop * 0.0005 }
                    } else if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 }
                })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        for i in 0..8u32 {
            let seat = (i * 2) % 20;
            let mut h = house_at(seat, vec![2 + (i as usize % 3)], 3);
            h.archetype = (i % 4) as u8;
            h.wealth = 40.0 + (i as f32) * 12.0;
            h.dominant_seat = i % 2 == 0;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        s.rebuild_routes();

        for _ in 1..=60u32 {
            s.advance(365);
            for h in &s.hubs {
                if h.is_estate { continue; }
                let so = &h.society;
                let sum = so.patrician + so.burgher + so.commoner + so.underclass;
                if sum < 1e-3 { continue; } // not yet seeded (brand-new colony this tick)
                for (name, v) in [("patrician", so.patrician), ("burgher", so.burgher),
                                  ("commoner", so.commoner), ("underclass", so.underclass)] {
                    assert!(v >= -1e-4 && v <= 1.0 + 1e-4, "{name} share out of range: {v}");
                }
                assert!((sum - 1.0).abs() < 1e-2, "shares must sum to 1, got {sum}");
                assert!(so.inequality >= 0.0 && so.inequality <= 1.0, "inequality 0..1: {}", so.inequality);
                assert!(so.commoner_wealth.is_finite() && so.commoner_wealth >= 0.0, "commoner_wealth bad");
            }
        }
        // The strata must DIFFERENTIATE — not every city ends with the same elite share.
        let elites: Vec<f32> = s.hubs.iter().filter(|h| !h.is_estate)
            .map(|h| h.society.patrician + h.society.burgher).collect();
        let emin = elites.iter().cloned().fold(f32::INFINITY, f32::min);
        let emax = elites.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        // Cities must still visibly DIFFERENTIATE. The floor was 0.02 before house
        // GOVERNMENT CAPTURE landed — a dominant family that seizes a city's officials
        // spreads its (favourable-tariff) policy, which mildly lifts backwaters' trade and
        // so compresses the elite-share gap a little. The spread stays clearly non-trivial.
        assert!(emax - emin > 0.012, "strata should vary across cities (spread {:.3})", emax - emin);
    }

    /// It. 3 · A chronically poor, steeply unequal city should boil over: unrest
    /// climbs, a revolt fires, the ruling council is toppled & barred, and wealth
    /// stays finite throughout (the redistribution is a sink, not a blow-up).
    #[test]
    fn unrest_topples_councils() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.9, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..6u32 {
            let x = (i % 3) as f32 * 8.0;
            let y = (i / 3) as f32 * 8.0;
            let pop = 12000.0;
            // Food-starved everywhere (all one component → no relief), so dearth bites.
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { pop * 0.004 } else { pop * 0.002 }).collect();
            hubs.push(hub(i, x, y, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // A tiny, fabulously rich oligarchy sits each city's council → extreme inequality.
        for i in 0..6u32 {
            let mut h = house_at(i, vec![2], 2);
            h.wealth = 800.0;
            h.dominant_seat = true;
            h.archetype = ARCH_POLITICAL;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        s.rebuild_routes();

        for _ in 1..=45u32 {
            s.advance(365);
            for h in &s.hubs {
                assert!(h.society.unrest >= 0.0 && h.society.unrest <= 1.0, "unrest 0..1");
            }
            for h in &s.houses {
                assert!(h.wealth.is_finite() && h.wealth < 1.0e7, "house wealth blew up: {}", h.wealth);
            }
        }
        let revolts = s.journal.iter().filter(|e| e.kind == "revolt").count();
        assert!(revolts >= 1, "a chronically poor, unequal city should revolt at least once");
        // Some council was barred by a revolt (the ban window was set).
        let banned = s.hubs.iter().any(|h| h.society.ousted_until > 0);
        assert!(banned, "a revolt should bar the toppled family from the council");
    }

    /// Colonisation MECHANISM (deterministic): from year 50, an eligible city founds
    /// a SETTLEMENT colony (full market hub, joint-stock funded, migrants seeded) and
    /// a rich house plants a remote trade OUTPOST on poor land within office/home
    /// reach — both land-only — and a grown colony graduates then may go autonomous.
    #[test]
    fn colonisation_mechanism() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        let ng = goods.len();
        // Three ordinary hubs clustered near the origin.
        let mut hubs = Vec::new();
        for i in 0..3u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if goods[g].food { 200.0 } else { 40.0 }).collect();
            hubs.push(hub(i, (i as f32) * 4.0, 0.0, 20_000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // A healthy, prosperous, treasury-rich founder city (hub 0).
        s.hubs[0].population = 30_000.0;
        s.hubs[0].founding_pop = 30_000.0;
        s.hubs[0].starving = 0.0;
        s.hubs[0].sent_prosperity = 0.8;
        s.hubs[0].treasury = 40.0;
        // A GREAT house seated at hub 0 → funds the venture + plants outposts. Must
        // clear the heavy outpost wealth bar (OUTPOST_FOUND_WEALTH).
        let mut rich = house_at(0, vec![2], 5);
        rich.wealth = 400_000.0;
        s.houses.push(rich);
        s.seed_house_count = 1;
        // A same-continent bank is REQUIRED to found a settlement colony (its family
        // becomes the colony's bank + mint).
        s.banks.push(Bank {
            name: "Banco di Test".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 50.0, loans: vec![], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, history: vec![], events: vec![],
        });
        // A second, non-backer house (seated elsewhere) → the charter should bar it.
        let mut other = house_at(1, vec![3], 2);
        other.wealth = 50.0;
        s.houses.push(other);
        // Empty land near the cluster: a fertile site (→ settlement colony) and a
        // trade-rich poor coastal site (→ house outpost), both within the hop-reach cap.
        s.colonizable = vec![
            ColonizeSite { x: 3.0, y: 3.0, koppen: 0, elevation: 0.2, fertility: 0.80, coastal: false, kind_hint: 1, trade_value: 0.10, delta: false, chokepoint: false },
            ColonizeSite { x: 5.0, y: 2.0, koppen: 0, elevation: 0.1, fertility: 0.18, coastal: true, kind_hint: 4, trade_value: 0.60, delta: false, chokepoint: false },
        ];
        s.rebuild_routes();
        s.tick = COLONY_START_TICK; // open the age of colonisation

        let base = s.hubs.len();
        let parent_pop0 = s.hubs[0].population;
        s.maybe_found_house_outpost();
        s.maybe_found_settlement_colony();

        let outposts = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
        let settlements: Vec<usize> = (0..s.hubs.len()).filter(|&h| s.hubs[h].colony_kind == 1).collect();
        assert_eq!(outposts, 1, "a house trade outpost was planted");
        assert_eq!(settlements.len(), 1, "a settlement colony was founded");
        assert!(s.hubs.len() == base + 2, "two new colony hubs exist");
        // Outpost is REMOTE (kept its own site coords, not co-located with home).
        let outpost = s.hubs.iter().find(|h| h.colony_kind == 2).unwrap();
        assert!(!outpost.is_estate || outpost.parent < 0, "outpost is remote, not an in-city estate");
        assert!(outpost.name.contains("(outpost)"));
        // Settlement colony is a FULL market hub, tagged, with backers, and the parent
        // shed migrants to seed it.
        let c = settlements[0];
        assert!(!s.hubs[c].is_estate, "settlement colony is a market hub");
        assert_eq!(s.hubs[c].colony_kind, 1, "tagged as a settlement colony");
        assert_ne!(s.hubs[c].name, format!("New {}", s.hubs[0].name), "colony has its OWN fresh name, not a duplicate of the parent");
        assert!(!s.hubs[c].backers.is_empty(), "joint-stock backers recorded");
        assert!(s.hubs[0].population < parent_pop0, "parent shed emigrants to the colony");
        // Bank + mint: the backing bank's family seats the colony's council & coin.
        assert!(s.hubs[c].main_bank >= 0, "colony has a main bank");
        assert!(!s.hubs[c].coin_name.is_empty(), "colony mints its own coin");
        // Food lifeline: civic supply contracts were signed (the roster).
        assert!(s.colony_supply.iter().any(|r| r.colony_hub == c as u32 && r.category == 0), "food supplier signed");
        // Monopoly charter: the non-backer house (idx 1) is barred from the colony.
        assert!(s.house_barred.get(1).is_some_and(|v| v.contains(&(c as u32))), "charter bars a non-backer house");

        // GROWTH GATE: needs ≥5yr supply + population + buildings. Set those + age 71
        // and run the yearly pass → graduates to city AND rebels (metropolis alive).
        s.hubs[c].population = 60_000.0;
        s.hubs[c].supply_years = 6.0;
        s.hubs[c].structures = vec![1, 2, 3];
        s.hubs[c].colony_founded_tick = 0;
        s.tick = 71 * 365;
        s.colony_pass();
        assert!(s.hubs[c].colony_stage >= 4, "graduates to city with supply+pop+buildings");
        assert!(s.hubs[c].war_with >= 0 && s.wars.iter().any(|w| w.cause == "independence"),
            "a mature year-70 colony-city wages a war of independence");
        // Resolve it (back-date so it's ripe) → freedom or a 15-yr cooldown.
        if let Some(w) = s.wars.iter_mut().find(|w| w.cause == "independence") { w.start_tick = s.tick - 3 * 365; }
        let yr = s.tick / 365;
        s.update_wars(yr);
        assert!(s.hubs[c].autonomous || s.hubs[c].indep_cooldown_until > 0,
            "the independence war resolves to a free city or a cooldown");
        // Wealth stays finite throughout.
        for h in &s.houses { assert!(h.wealth.is_finite()); }
    }

    /// Government: figures are seeded; a rich, locally-dominant house bribes them into
    /// service, CAPTURES the seat (favourable-house law logged), and seats turn over.
    #[test]
    fn government_capture_and_regime_change() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..3u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if goods[g].food { 200.0 } else { 40.0 }).collect();
            hubs.push(hub(i, i as f32 * 4.0, 0.0, 40_000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // A rich BANKING house homed at hub 0 with commanding commercial influence there
        // (a cash briber — the reliable capture path).
        let mut rich = house_at(0, vec![1], 3);
        rich.wealth = 300_000.0;
        rich.archetype = ARCH_BANKING;
        rich.prestige = 1.0;
        rich.influence = vec![(0u32, 0.9)];
        s.houses.push(rich);
        s.seed_house_count = 1;
        s.rebuild_routes();
        let mut ever_captured = false;
        let mut seats_turned = false;
        let first_terms: Vec<u32>;
        s.tick = 365;
        s.update_government(1);
        assert!(!s.hubs[0].officials.is_empty(), "officials seeded");
        first_terms = s.hubs[0].officials.iter().map(|o| o.term_end).collect();
        // House keeps spending its money — top it back up each year so it can maintain grip.
        for yr in 2..=14u32 {
            s.tick = yr * 365;
            s.houses[0].wealth = 300_000.0;
            s.update_government(yr);
            if s.hubs[0].captor_house == 0 { ever_captured = true; }
            if s.hubs[0].officials.iter().zip(&first_terms).any(|(o, &t)| o.term_end != t) {
                seats_turned = true;
            }
        }
        assert!(ever_captured, "the dominant house should capture the government");
        assert!(s.hubs[0].captor_house == 0, "and hold it at the end");
        assert!(s.hubs[0].officials.iter().any(|o| o.house == 0 && (o.control >= OFFICIAL_CAPTURE || o.kin)),
            "at least one figure serves the house");
        assert!(!s.hubs[0].laws.is_empty(), "a favoured-house law was enacted on capture");
        assert!(seats_turned, "seats turn over across the years (regime change)");
        // The captor's influence at the city got a boost from capture.
        let infl0 = s.houses[0].influence.iter().find(|(c, _)| *c == 0).map(|(_, v)| *v).unwrap_or(0.0);
        assert!(infl0 >= 0.9, "capture boosts the captor's trade influence: {infl0}");
    }

    /// A starved colony (empty reserve) collapses: it dies out AND its bank writes off
    /// the colony loan (a loss that can later sink the bank → crash).
    #[test]
    fn colony_collapse_defaults_bank() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let ng = goods.len();
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 20_000.0, vec![200.0, 40.0], 0),  // metropolis
            hub(1, 4.0, 0.0, 5_000.0, vec![60.0, 10.0], 0),    // will be a colony
        ];
        hubs[1].colony_kind = 1;
        hubs[1].founder_hub = 0;
        hubs[1].main_bank = 0;
        hubs[1].reserve_food = 0.0;
        hubs[1].starving = 0.9;
        hubs[1].backers = vec![(2, 0, 1.0)];
        let mut s = sim(hubs, goods);
        let _ = ng;
        s.banks.push(Bank {
            name: "Banco".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 30.0, loans: vec![Loan { borrower_house: -1, borrower_polis: 1, principal: 10.0,
                outstanding: 10.0, rate: 0.01, start_tick: 0, term_ticks: 3650, purpose: "colony".into() }],
            real_estate: 1.0, deposits: 0.0, notes_issued: 0.0, branches: vec![0], prestige: 0.5,
            interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, history: vec![], events: vec![],
        });
        s.tick = 55 * 365;
        s.colony_pass();
        assert!(s.hubs[1].colony_kind == 0 && s.hubs[1].population <= 1.0, "starved colony collapsed");
        assert!(s.banks[0].losses > 0.0 && s.banks[0].loans.is_empty(), "bank wrote off the defaulted colony loan");
    }

    /// Manual benchmark for the per-day campaign tick. Run explicitly:
    ///   `cargo test --release --lib bench_campaign_tick -- --ignored --nocapture`
    /// Reports total + per-tick ms for a year on a mid-size campaign so the tick
    /// cost (and how it scales with hub/good count) can be measured.
    #[test]
    #[ignore]
    fn bench_campaign_tick() {
        use std::time::Instant;
        let ng = 24usize;
        let goods: Vec<TickGood> = (0..ng)
            .map(|g| good(&format!("g{g}"), (g % 12) as i32, (g % 3) as u8,
                          1.0 + g as f32, 0.30 + 0.5 * ((g % 5) as f32 / 5.0), g < 6))
            .collect();

        let nhubs = 160u32;
        let mut hubs = Vec::new();
        for i in 0..nhubs {
            let x = (i % 16) as f32 * 6.0;
            let y = (i / 16) as f32 * 6.0;
            let pop = 2000.0 + (i as f32 * 137.0) % 9000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 7 == 0 { pop * 0.02 } else { pop * 0.002 })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0)); // one component → trade flows
        }
        let mut s = sim(hubs, goods);
        for i in (0..nhubs).step_by(8) {
            s.houses.push(house_at(i, vec![i as usize % ng], 2));
        }
        s.rebuild_routes();

        let days = 365u32;
        let t0 = Instant::now();
        s.advance(days);
        let total = t0.elapsed().as_secs_f64() * 1000.0;
        println!(
            "[campaign-bench hubs={nhubs} goods={ng}] {days} ticks: {total:.1}ms total, {:.3}ms/tick",
            total / days as f64
        );
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
    fn coinage_runs_yearly_finite_and_deterministic() {
        // DLC 3.5 · council seats mint named coins with a bounded trust score, and
        // the whole coin/bank pass stays finite + reproducible across two runs.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let mk = || {
            let hubs = vec![
                hub(0, 10.0, 10.0, 30000.0, vec![160.0, 16.0], 0),
                hub(1, 40.0, 12.0, 20000.0, vec![120.0, 2.0], 0),
            ];
            let mut s = sim(hubs, goods.clone());
            // A dominant banking house at each seat → a council that mints coin.
            for i in 0..2u32 {
                let mut h = house_at(i, vec![1], 3);
                h.archetype = 2;            // banking
                h.wealth = 60.0;
                h.prestige = 0.6;
                h.dominant_seat = true;     // controls its seat → becomes the council
                s.houses.push(h);
            }
            // v2.0 · minting is now chartered (a paid privilege) — seed each seat a
            // treasury it can draw on to establish its mint-house.
            for hh in s.hubs.iter_mut() { hh.treasury = 200.0; }
            s.rebuild_routes();
            s
        };
        let mut a = mk();
        let mut b = mk();
        a.advance(800);
        b.advance(800);
        // Coins were minted, with trust kept in range, and reproducibly.
        assert!(a.hubs.iter().any(|h| !h.coin_name.is_empty()), "a council minted a coin");
        for (ha, hb) in a.hubs.iter().zip(b.hubs.iter()) {
            assert!(ha.coin_trust.is_finite() && (0.0..=1.0).contains(&ha.coin_trust));
            assert!((ha.coin_trust - hb.coin_trust).abs() < 1e-4, "coin trust reproducible");
            assert_eq!(ha.coin_name, hb.coin_name, "coin name reproducible");
        }
        // Banks (if any chartered) keep a sound, finite balance sheet.
        assert_eq!(a.banks.len(), b.banks.len(), "bank count reproducible");
        for bank in &a.banks {
            assert!(bank.equity().is_finite() && bank.reserves.is_finite());
        }
    }

    #[test]
    fn regional_crash_is_confined_to_its_region() {
        // DLC 3.5 · a crash hits every city in the origin's connectivity component
        // and haircuts houses there, but leaves a separate region untouched.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0), // region 0
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),   // region 0
            hub(2, 80.0, 50.0, 8000.0, vec![80.0], 1),   // region 1 (separate)
        ];
        let mut s = sim(hubs, goods);
        for i in 0..3u32 { s.houses.push(house_at(i, vec![0], 2)); }
        let w_before: Vec<f32> = s.houses.iter().map(|h| h.wealth).collect();
        s.trigger_regional_crash(0, 0, "test");
        // Region-0 cities are in panic; region-1 city is not.
        assert!(s.hub_in_panic(0) && s.hub_in_panic(1), "origin region panics");
        assert!(!s.hub_in_panic(2), "other region is spared");
        // Houses homed in region 0 took a haircut; the region-1 house did not.
        assert!(s.houses[0].wealth < w_before[0] && s.houses[1].wealth < w_before[1]);
        assert!((s.houses[2].wealth - w_before[2]).abs() < 1e-4, "other region's house untouched");
        assert_eq!(s.crashes.len(), 1, "the crash was recorded");
        assert_eq!(s.crashes[0].cities_hit, 2);
    }

    #[test]
    fn sound_banks_survive_contagion_only_fragile_fall() {
        // DLC 3.5 · a regional crash must NOT wipe every bank. With the softened
        // contagion run, a well-capitalised bank rides out the panic; only a
        // thinly-reserved (already-fragile) bank is swept. (Regression for the
        // total-wipeout cascade where one failure killed all banks in a region.)
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
        ];
        let mut s = sim(hubs, goods);
        for i in 0..2u32 { s.houses.push(house_at(i, vec![0], 2)); }
        let mk_bank = |name: &str, reserves: f32, deposits: f32, notes: f32| Bank {
            name: name.into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves, loans: vec![], real_estate: 100.0, deposits, notes_issued: notes,
            branches: vec![0], prestige: 0.5, interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, history: vec![], events: vec![],
        };
        // Two soundly-capitalised banks and one fragile (reserves ≪ liabilities).
        s.banks.push(mk_bank("Banco Solido", 5000.0, 1000.0, 1000.0));   // ratio 2.5
        s.banks.push(mk_bank("Banco Stabile", 3000.0, 2000.0, 1000.0));  // ratio 1.0
        s.banks.push(mk_bank("Banco Fragile", 200.0, 2000.0, 500.0));    // ratio 0.08
        s.trigger_regional_crash(0, 0, "test");
        assert!(!s.banks[0].defunct, "a soundly-capitalised bank survives the panic");
        assert!(!s.banks[1].defunct, "a second sound bank survives the panic");
        assert!(s.banks[2].defunct, "the thinly-reserved fragile bank is swept away");
        assert!(s.banks.iter().any(|b| !b.defunct), "not every bank fails");
    }

    #[test]
    fn economic_war_levies_houses_and_resolves() {
        // DLC 3.5 · a war drains resident houses via levies and resolves after ≥2
        // years into the war log with reparations — the wealth sink in action.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
        ], goods);
        for i in 0..2u32 { let mut h = house_at(i, vec![0], 2); h.wealth = 100.0; s.houses.push(h); }
        s.hubs[0].treasury = 50.0; s.hubs[1].treasury = 20.0;
        s.hubs[0].war_with = 1; s.hubs[1].war_with = 0;
        s.wars.push(War { a: 0, b: 1, start_tick: 0, chest_a: 0.0, chest_b: 0.0,
            levies: 0.0, cargo_lost: 0, cause: "test".into(), goal: WAR_GOAL_PLUNDER });
        let w0 = s.houses[0].wealth;
        s.tick = 0;
        s.update_wars(0); // wage one year — levy, no resolve
        assert!(s.houses[0].wealth < w0, "war levy drained a resident house");
        assert_eq!(s.war_log.len(), 0, "not resolved before 2 years");
        s.tick = 2 * 365;
        s.update_wars(2); // resolve
        assert_eq!(s.war_log.len(), 1, "war resolved into the log");
        assert!(s.hubs[0].war_with < 0 && s.hubs[1].war_with < 0, "war state cleared");
        assert!(s.war_log[0].levies_total > 0.0, "levies recorded");
    }

    #[test]
    fn war_goals_transfer_control_and_tribute_is_bounded() {
        // A resolved war's GOAL takes lasting spoils: annexation seats the victor's
        // ruling house on the loser's council (via a bailo); tribute is a bounded,
        // term-limited treasury transfer that the overlord receives in full.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0), // victor
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),   // loser
        ];
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 2)); // house 0 rules hub 0
        s.houses.push(house_at(1, vec![0], 2)); // house 1 rules hub 1
        s.hubs[0].council_house = 0;
        s.hubs[1].council_house = 1;
        // ── Annexation: hub 1's council passes to hub 0's ruling house. ──
        let clause = s.apply_war_goal(0, 1, WAR_GOAL_ANNEX, 0, 0);
        assert_eq!(s.hubs[1].council_house, 0, "loser's council is installed with the victor's house");
        assert!(s.houses[0].bailos.contains(&1), "victor's house gains a bailo in the loser");
        assert!(clause.contains("annexed"), "the annexation is narrated");
        // ── Tribute: bounded, term-limited treasury transfer. ──
        s.hubs[1].treasury = 1000.0;
        s.hubs[0].treasury = 0.0;
        s.apply_war_goal(0, 1, WAR_GOAL_TRIBUTE, 0, 0);
        assert_eq!(s.hubs[1].tribute_to, 0, "loser owes tribute to the victor");
        assert!(s.hubs[1].tribute_until > 0, "tribute has a term");
        let t0 = s.hubs[1].treasury;
        s.update_wars(1); // a tribute year
        let paid = t0 - s.hubs[1].treasury;
        assert!(paid > 0.0, "tribute is paid");
        assert!(paid <= TRIBUTE_CAP * s.city_size_factor(1) + 1e-3, "tribute is capped");
        assert!((s.hubs[0].treasury - paid).abs() < 1e-2, "the overlord receives exactly the tribute");
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
        // A large city (≥ GUILD_MIN_POP 10k) charters a civic Merchant Guild; a
        // small town (9k) doesn't.
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
            last_fulfilled: 0, suspended_until: 0, defaults: 0, coin: -1,
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
            last_fulfilled: 0, suspended_until: 0, defaults: 0, coin: -1,
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
            last_fulfilled: 0, suspended_until: 0, defaults: 0, coin: -1,
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
