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

use crate::sim::inheritance::{InheritanceRule, LineRule};

pub const TICKS_PER_YEAR: u32 = 365;
pub const SEASONS: [&str; 4] = ["Spring", "Summer", "Autumn", "Winter"];

const EPS: f32 = 1e-4;
const TIER_WEIGHT: [f32; 3] = [1.0, 0.45, 0.22];
/// Cities covet LUXURIES they can't make at home — fine goods from far lands. A
/// luxury good NOT produced locally gets this extra desire, scaled by its prestige
/// (base_value), so high-value foreign luxuries drive vigorous inter-city trade
/// instead of every city being content with what it produces itself (#2).
const LUX_IMPORT_DESIRE: f32 = 0.7;
/// Share of `LUX_IMPORT_DESIRE` a COMFORT-tier good draws when a city cannot make it
/// at home (a luxury draws the full amount). See `production.rs::base_need` for the
/// mechanism.
///
/// **THIS VALUE IS SET BY A GATE THAT HAS NOTHING TO DO WITH TRADE, AND THE TRADE
/// EVIDENCE POINTS THE OTHER WAY.** Recorded rather than acted on, because raising it
/// re-breaks `econ_inheritance_rules_fragment_differently` (that is exactly why it is
/// 0.30: it shipped at 0.60 and left that gate red on main for four commits).
///
/// Measured — `econ_fidelity_scorecard`, one seed per dose, sweeping this constant:
///
/// | dose | basket price gap × distance | goods with a positive gradient | basket CV | real wage |
/// |------|------|------|------|------|
/// | 0.00 | −0.006 | 0 of 6 | 1.573 | 146.3 |
/// | 0.30 | **−0.064** | **0 of 6** | 1.596 | 162.5 |
/// | 0.60 | **+0.041** | **2 of 6** | 1.672 | 169.4 |
/// | 0.90 | +0.053 | 3 of 6 | 1.678 | 152.9 |
///
/// A POSITIVE price/distance gradient is the historically correct sign (Federico,
/// Persson) and its absence is the single largest market-realism failure this project
/// has named — `docs/TRADE_AND_MARKET_REVIEW.md` F2, "distance costs nothing
/// anywhere". Only doses at or above 0.60 produce it. The shipped 0.30 is the WORST
/// of the four tested on that measure.
///
/// Read the caveats before acting on the table: one seed per dose; the low end is not
/// monotone (0.00 → 0.30 makes the gradient *more* negative); real wage peaks at 0.60
/// rather than rising; and every dose leaves basket CV at 1.57–1.68 against a
/// historical 0.20–0.40, so no value here makes the market realistic — the differences
/// are directional inside an already badly-calibrated regime.
///
/// So this is a genuine conflict between two gates, not a value waiting to be nudged.
/// Resolving it means fixing the thing F2 actually blames (freight is ~11% of grain
/// value over the longest route; i.i.d. per-hub harvest shocks leave no regional
/// scarcity for a gradient to form against) rather than paying for integration with a
/// demand constant. Reproduce with the sweep recipe in `docs/SCOREBOARD.md` 2026-08-20d.
const COMFORT_IMPORT_FRAC: f32 = 0.30;
const PRICE_FLOOR_MULT: f32 = 0.15;
const PRICE_CEIL_MULT: f32 = 12.0;
/// N6 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §2) · own-price elasticity of a
/// need CATEGORY's aggregate, by need tier [basic, comfort, luxury]. Shipped a
/// true no-op — the historical shape (grain ≈ −0.1…−0.3, comfort moderate,
/// luxury elastic) is the target once dosed; see the dose-walk note in
/// `docs/SCOREBOARD.md`. Cross-price elasticity WITHIN a category (a dearer
/// member losing share to a cheaper one) is separate, already live, and
/// unaffected by this constant.
const DEMAND_ELASTICITY: [f32; 3] = [0.0, 0.0, 0.0];
/// Elasticity may never move the category aggregate outside this band,
/// whatever the price does. At e = 1.0 an unclamped `rel.powf(-e)` would
/// collapse demand ~12× on a `PRICE_CEIL_MULT`-sized spike, emptying a market
/// on price alone.
const ELASTIC_CLAMP: (f32, f32) = (0.55, 1.45);
/// Tier-0 (basic/food) floor. A starving population's grain demand is nearly
/// perfectly inelastic in reality, and the model has no "the poor go without
/// and live" pathway — only `starving` — so this floor is the mechanism the
/// "starvation must not rise" gate is enforced BY, not merely hoped for.
const SUBSISTENCE_FLOOR: f32 = 0.85;

/// N6 §2.3's own-price elasticity multiplier, parametrized on `e` rather than
/// reading `DEMAND_ELASTICITY` directly — a pure function, testable in
/// isolation from the tick's giant `advance()` loop, and the same shape a
/// future dose walk (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §2.5's table)
/// will exercise without touching this arithmetic.
fn elastic_aggregate_mult_e(e: f32, tier: usize, rel: f32) -> f32 {
    if e <= 0.0 { return 1.0; }
    let mut m = rel.max(PRICE_FLOOR_MULT).powf(-e).clamp(ELASTIC_CLAMP.0, ELASTIC_CLAMP.1);
    if tier == 0 { m = m.max(SUBSISTENCE_FLOOR); }
    m
}

/// The shipped dose — reads `DEMAND_ELASTICITY[tier]` (`[0,0,0]` today, a
/// true no-op: see `elastic_aggregate_mult_e`).
fn elastic_aggregate_mult(tier: usize, rel: f32) -> f32 {
    elastic_aggregate_mult_e(DEMAND_ELASTICITY[tier.min(2)], tier, rel)
}
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
/// TRADE_STAGING_AND_POSTS_PLAN.md §5 slice 3 — `SEA_LOSS`/`CARAVAN_LOSS`/
/// `RIVER_LOSS` above are the loss PROBABILITY for a voyage of this many days;
/// a longer or shorter leg scales via `1 - (1 - p)^(days / LOSS_REFERENCE_DAYS)`
/// (`CampaignSim::distance_scaled_loss`) rather than the old flat per-shipment
/// roll, which made a 9,000 km crossing exactly as safe as a 200 km one (§1.2).
const LOSS_REFERENCE_DAYS: f32 = 20.0;
/// A per-voyage subsistence cost (crew wages, draft-animal fodder, harbour
/// dues) on top of the existing per-unit freight rate — "victualling"
/// (TRADE_STAGING_AND_POSTS_PLAN.md §5 slice 3). Folded into `good_freight`,
/// which every caller (dispatch, `deploy_return_leg`, futures delivery)
/// already reads, so a long haul costs non-linearly more than a short one
/// even before the freight rate itself is applied. Kept small relative to
/// `freight_per_day` (0.018 shipped) since it is a flat add per unit-day,
/// not a rate — see `good_freight`'s own doc comment.
const VICTUAL_PER_DAY: f32 = 0.001;
/// A fixed per-voyage outfitting charge (crew wages up front, harbour dues,
/// loading) independent of how much cargo the voyage carries — "so long hauls
/// need scale" (TRADE_STAGING_AND_POSTS_PLAN.md §5 slice 3): a tiny shipment
/// barely amortises this, a large one absorbs it easily. Charged once per
/// dispatched (house-owned) shipment in `production.rs::dispatch`, deducted
/// from the carrying house directly rather than folded into the per-unit
/// price gap, so it does not distort the arbitrage math that decides WHICH
/// market gets the cargo — only whether running the voyage at all was worth it.
const OUTFIT_COST: f32 = 0.05;
/// TRADE_STAGING_AND_POSTS_PLAN.md §5 slice 6 (§4.1 Brake 2) — the EXTRA voyage
/// loss probability (additive, before `distance_scaled_loss` scales it to the
/// real leg length) a house pays for running a leg through an entrepôt outlet
/// that has barred it. Never a flat block — a post owner who could delete a
/// rival's lane outright would be a stronger weapon than any existing war
/// goal (`WAR_GOAL_PROVINCE` only transfers one province) — but a real,
/// survivable cost for slipping past under embargo.
const BYPASS_LOSS_ADD: f32 = 0.12;
/// Independent trade shorter than this (travel-days) is "local merchants";
/// anything longer is organized "guild" long-haul. Splits the non-house carry.
const LOCAL_HAUL_DAYS: f32 = 8.0;
/// `ACTORS_AND_CARRIAGE_PLAN.md` N1 (the keystone) — make the local haul BIND:
/// an ownerless leg (no house could carry it) longer than this many travel-days
/// does not sail at all, rather than moving for free with no vessel, no capital
/// clamp and no loss risk (§1 of the plan measured 96% of shipments moving this
/// way). Shipped at `INFINITY`, which makes the bind clause dead code and the
/// whole change bit-identical — `n1_local_haul_bind_at_infinity_is_a_noop`
/// proves it. The dose walk down from infinity is its own, separately-gated,
/// multi-commit exercise (§4 of the plan) and is deliberately NOT done here.
const N1_LOCAL_HAUL_BIND_DAYS: f32 = f32::INFINITY;
/// `ACTORS_AND_CARRIAGE_PLAN.md` N1b — let ownerless cargo sink too, at a rate
/// independently dosed from the house loss rates above (today `owner < 0`
/// cargo never sinks at all: "the guard is literal", §1.1 of the plan). Shipped
/// at 0.0, so the roll below never fires and the change is bit-identical.
const N1B_OWNERLESS_LOSS_RATE: f32 = 0.0;
/// A world's equatorial circumference in km — the same conversion every other
/// module states locally per rule 25 (`localities.rs`, `deposits.rs`,
/// `landform.rs`, `landmass_ops.rs`), so a cell-space distance can be read as
/// real km whatever `world_w` the campaign was seeded from.
const KM_EQUATOR: f32 = 40075.0;
/// N1c — a per-mode geographic RANGE cap on the ownerless residual only (a
/// house's own fleet keeps N1_LOCAL_HAUL_BIND_DAYS's day-based bind above;
/// this is the companion the plan's own §1 measured but never dosed: "trade
/// HORIZON is 0.24×world_w, i.e. 9,617 km on a 3600 grid" — a single
/// ownerless leg can cross nearly a quarter of the planet in one hop, with no
/// vessel, no capital and no relay, which is the literal "simple walk"
/// complaint this constant answers). A real pre-modern SINGLE-LEG voyage
/// rarely ran past a few thousand km before making port and re-provisioning
/// (open-sea endurance) or past a few hundred km before a caravan changed
/// hands at a caravanserai — `SHIP_LEG_MAX_KM`/`CARAVAN_LEG_MAX_KM` are that
/// per-mode ceiling, read against the real cylindrical straight-line distance
/// between the two hubs (not the routed/terrain-penalised `days`, which
/// already blends in cost the raw geography doesn't carry). A HOUSE-carried
/// leg (an established merchant network, the closest this sim has to a real
/// relay) is untouched — only `owner < 0` is capped.
/// Trial-measured at the historically-motivated 3500/800 km: it held
/// `simulate_decades_reports_dynamics`'s wealth bound on that test's own
/// (small, abstract-scale) 30-hub world, but broke three PRE-EXISTING unit
/// fixtures (`a_house_records_every_head_it_has_had`, `big_city_is_a_net_
/// importer`, `n7_boycott_is_inert_at_zero`) that place hubs on a `world_w:
/// 100.0` grid never meant to carry real-km meaning — at that scale even two
/// ADJACENT hubs sit ~3,600 km apart by this conversion, so ownerless trade
/// collapsed almost everywhere in those fixtures, not just on genuine
/// megateleports. The economy-fidelity reference world (`world_w: 3600`,
/// real Earth scale — §2.5) was never re-measured against this dose this
/// session, so whether 3500/800 is actually safe there is unconfirmed, not
/// merely untested. Shipped at `INFINITY`, exactly `N1_LOCAL_HAUL_BIND_DAYS`'s
/// own precedent: real, wired, `leg_exceeds_range` gated by
/// `leg_exceeds_range_uses_the_right_cap_per_mode`, dead code at this dose.
/// The dose walk down — on the Earth-scale reference world specifically, not
/// the small unit fixtures — is real future work.
const SHIP_LEG_MAX_KM: f32 = f32::INFINITY;
const CARAVAN_LEG_MAX_KM: f32 = f32::INFINITY;
/// Charter EXCLUSIVITY — the market-square counterpart of N1/N2. A charter
/// (`House.charters`, already granted to a POLITICAL house or a chartered guild
/// that dominates its own seat, on its own specialty goods) today only earns the
/// holder extra RENT (`CHARTER_RENT`) — a rival house or the ownerless residual
/// can still sell the identical good in that same city freely, so "the city
/// doesn't impose it" and the charter is a tax, not a monopoly. This turns it
/// into a real staple right: at a hub that has chartered a good, a leg
/// delivering that good there is barred from completing the sale unless it is
/// carried by the charter holder itself. `CHARTER_EXCLUSIVE_DOSE` is the chance
/// (0..1) such a leg still gets through anyway ("smuggling") — dose 0.0 lets
/// every non-holder trade exactly as before (a true no-op,
/// `charter_exclusive_dose_zero_is_a_noop`), dose 1.0 is an absolute staple.
/// Measured exactly like N2 (`N2_BAN_PRICE_RATIO`'s own doc comment) and hit
/// the SAME wall: both 0.3 and 1.0 held `simulate_decades_reports_dynamics`'s
/// wealth bound (richest 236,749–398,853 over 50y, no blow-up), but both broke
/// the hard-asserted `econ_inheritance_rules_fragment_differently` — at 1.0,
/// partible's mean wealth per house came out ABOVE primogeniture's
/// (284,238 vs 245,817; the assertion requires partible strictly poorer,
/// since a division must leave the average house smaller). A staple-right
/// closure evidently redistributes capital toward whichever houses already
/// hold a charter regardless of which inheritance law is running, swamping
/// the signal the gate measures — the same failure mode §8.15's "read this
/// before fixing this gate again" catalogue already names five times over.
/// Shipped at 0.0: real, wired, dead code today. The dose walk down from 1.0
/// (rather than guessing a value that merely stops panicking) is real future
/// work, not attempted further in this session — see `docs/SCOREBOARD.md`.
const CHARTER_EXCLUSIVE_DOSE: f32 = 0.0;
/// Guilds gain a charter the same way a political house does (dominates its own
/// seat) — Slice 2 of the same change (`now_dom && (ARCH_POLITICAL || is_guild)`
/// at the grant site in `houses.rs`), since a chartered civic Merchant Guild
/// enforcing its own staple is exactly as historical as a merchant house doing
/// the same (the Hanseatic Kontor, a Zunft's guildhall monopoly).
const CHARTER_GUILDS_TOO: bool = true;
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
/// Fraction of accumulated grievance retained through a CALM year (unrest below the
/// riot line). < 1 so a single good harvest doesn't erase years of resentment, but
/// a sustained recovery does eventually cool a city off.
const GRIEVANCE_COOL: f32 = 0.6;
/// Accumulated grievance (in riot-years) at which a chronically-rioting city boils
/// over into a REVOLT on its own — the slow-burn path to revolution, distinct from
/// the acute `REVOLT_UNREST` spike. ~this many years simmering at the riot line.
const GRIEVANCE_REVOLT: f32 = 1.5;
/// DLC 4 · FIX_PLAN B3 — weight of the population-weighted `Pop.militancy` term
/// in the unrest target. Small: the pops' own hardship term already overlaps with
/// `lackb`/`starv` above, so this is meant to add only the profession-MIX signal
/// (an underclass-heavy city reads more militant than a burgher-heavy one at the
/// same inequality/wealth) rather than double-count hardship itself.
const POP_MILITANCY_WEIGHT: f32 = 0.10;
/// DLC 4 · FIX_PLAN B3 — how much population-weighted `Pop.consciousness` (0..1)
/// scales grievance accrual: a more politically aware populace organizes chronic
/// misery into revolt-triggering grievance faster. Bounded to [0.75, 1.25]× so a
/// swing in consciousness nudges the slow-burn revolt path without dominating it.
const CONSCIOUSNESS_GRIEVANCE_MIN: f32 = 0.75;
const CONSCIOUSNESS_GRIEVANCE_MAX: f32 = 1.25;
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
/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.12 (A2) · "whoever grades, profits" —
/// the certifying authority's slice of the owner-cut, taken BEFORE the owner/
/// dividend split (mod.rs's own comment on that block). Deliberately a
/// REDISTRIBUTION of the existing `cut`, never an addition on top of `sale` —
/// this is what keeps it revenue-neutral (nothing is created, rule 18) and, on
/// the evidence of this session's other two 4.x tuning stories (4.7, 4.9), is
/// also what makes it SAFE against `econ_inheritance_rules_fragment_
/// differently`: a uniform skim applied identically to every estate under
/// every inheritance law is a far more symmetric perturbation than a targeted
/// house-to-house transfer, which is the working hypothesis behind trying a
/// real, wired fee here rather than scoping straight down to a query-side-only
/// build the way 4.7/4.9 eventually had to for their own money-moving pieces.
const CERT_FEE_FRAC: f32 = 0.04;
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
/// DEPOSITS_AND_MINING_PLAN.md slice 5 · hard ceiling on mining settlements
/// (the Potosí class) — a genuinely rare founding (only a GREAT/WORLD_CLASS
/// body qualifies), small on purpose: there were only ever a handful of these
/// in a given era, not one per city.
pub(crate) const MAX_MINING_SETTLEMENTS: usize = 8;
/// Same shape as `COLONY_MAX_KM` — a mining venture is still a bold reach, not
/// a trip to the far side of the world.
const MINING_SETTLEMENT_MAX_KM: f32 = 2500.0;
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
/// Settlement-colony site scoring bonus (user rule: "urge to found settlements in
/// the empty provinces") for a site in a province with no live settlement in it yet
/// — comparable in weight to the delta/coastal site premiums above, so a real gap
/// on the map competes with (but doesn't automatically beat) a genuinely better
/// site inside land that's already settled.
const EMPTY_PROVINCE_FOUND_BONUS: f32 = 0.6;
/// Daily logistic population-growth rate below carrying capacity (~5%/yr peak at
/// low population; eases to 0 at capacity). Was 0.0006 (~24%/yr — too fast).
const POP_GROWTH_RATE: f32 = 0.0003;
/// Daily decline rate when a city is above its carrying capacity.
const POP_DECLINE_RATE: f32 = 0.0006;
/// ── EARNED trade development (growth v2) ──────────────────────────────────────
/// A hub's carrying capacity used to be a FIXED multiple of its frozen founding
/// size (max ≈9×), so total world population asymptoted and stalled. It now
/// RATCHETS with realized trade throughput (`trade_last_year`): a hub that grows
/// into a busy entrepôt keeps earning headroom and can climb to metropolis scale
/// over centuries, while an isolated/low-trade hub stays small. Reference &
/// ceiling below are tuned so a top trade nexus reaches ≈30× its founding size.
/// `trade_dev = clamp(trade_last_year / (founding_pop · REF), 0, CAP)`.
/// Max earned headroom (capacity multiplier) a hub gains from TRADE EMINENCE. The
/// busiest hub in the world earns the full amount (RELATIVE normalisation — see the
/// growth block), so a great entrepôt of large founding size reaches ≈150k–300k people
/// while lesser hubs earn proportionally less and stay small. Raised 15→20 so the top
/// trade cities clear the 150k mark even at moderate food/prosperity.
const TRADE_DEV_CAP: f32 = 20.0;
/// ── MEGACITY primacy (the rare >1M capital) ──────────────────────────────────
/// History's million-person cities (Rome, Chang'an, Venice as a mercantile apex) were
/// political CAPITALS that could COMMAND tribute-grain and move it by secured WATER lanes.
/// The regional capital — the top-treasury hub of a trade component — that is also
/// water-connected (coastal) and a real trade hub earns a large extra growth headroom,
/// breaking past the ordinary trade ceiling toward a million. One per region, gated on
/// coast + trade-hub + being fed (the food_sec multiplier still applies), so it stays
/// RARE: a hub only nears 1M when capital + water + trade + prosperity + food all coincide.
const PRIMACY_DEV: f32 = 45.0;   // extra capacity-mult headroom for a qualifying capital
/// ── PROVEN colony headroom ── a settlement colony's `founding_pop` is frozen at
/// `COLONY_MIGRATION_FRAC` (6%) of its founder's population at the moment it's
/// planted — typically only 1-2k people. Without extra headroom, `cap_mult`'s own
/// ceiling (`food_sec` + `prosperity` terms alone, no `trade_dev`/`primacy_dev`,
/// which a young colony realistically never earns) tops out ≈13×, so capacity
/// plateaus around 15-25k — under `colony_pass`'s own 40k "city" stage threshold
/// (`colonies.rs`), so a colony could never structurally reach it. Mirrors
/// `colony_pass`'s own bar (`supply_years >= 5.0`, an unbroken 5-year lifeline)
/// so the extra headroom is EARNED — a colony that keeps starving never gets it.
const COLONY_CAP_DEV: f32 = 22.0;
/// ── EARNED age-of-world headroom (keeps population growing across centuries) ──
/// `trade_dev`/`primacy_dev` are both RELATIVE to the world's own busiest hub each
/// tick — once every hub's relative trade share stabilizes (the whole economy
/// scaling together), no hub earns further headroom and total world population
/// plateaus even though centuries remain in the campaign.
///
/// This was originally keyed to `tech_factor` (documented as "the entire
/// technology + growth model", nominally +1.5%/yr, `PROD_GROWTH_PER_YEAR`) — but
/// measuring it (`econ_diagnose_population_growth`, 300-year run) found
/// `tech_factor` ITSELF is a pre-existing, separate bug: `roll_events`' adverse
/// setbacks (fire `PROD_FIRE_SETBACK` + drought/plague/fishery_collapse/embargo
/// `PROD_EVENT_SETBACK`, at their actual ~36/yr firing rate) compound to roughly
/// −4%/yr, which OUTPACES the +1.5%/yr growth drift despite the comment beside
/// those constants claiming otherwise — so `tech_factor` collapses to its own
/// floor (`TECH_FACTOR_FLOOR` = 0.85) within about 6 years of ANY campaign and
/// stays pinned there permanently (confirmed: flat 0.85 for the entire 300-year
/// diagnostic run). That almost certainly matters far beyond population (`tech`
/// scales the whole world's daily production, mod.rs's day loop) but rebalancing
/// event-setback/growth constants is its own separate, careful `econ_`-gated
/// change — NOT bundled into this fix. Flagged, not silently patched.
///
/// So this headroom instead rides ELAPSED CAMPAIGN TIME (`self.tick`), which is
/// unconditionally monotonic — a saturating exponential, so it's always bounded
/// (approaches but never exceeds `WORLD_AGE_DEV_CAP`) while still rising for as
/// long as the campaign runs. Applied to EVERY hub (not earned/rare like
/// `trade_dev`/`primacy_dev`), so keep it modest — this is the term standing
/// between "population grows for the whole campaign" and "population blows past
/// the dynamics gate's bounded-wealth assert" if raised carelessly.
///
/// TUNING STORY (negative results kept, per CLAUDE.md §2.4): 10.0/150y and
/// 8.0/400y BOTH tripped `simulate_decades_reports_dynamics`' sustained-runaway-
/// wealth guard (`late_max < 1_000_000`) even though their OWN 50-year
/// contribution is tiny (≤0.94 of a ≤5.60 bracket) — a uniform, EVERY-hub capacity
/// nudge shifts population trajectories enough to change which single house ends
/// up richest and by how much, highly non-linearly (a smaller nudge at 8.0/400y
/// produced a WORSE outlier, 1.87M, than the larger one at 10.0/150y's 1.01M —
/// the wealth-concentration feedback is chaotic-sensitive to this parameter, not
/// monotonic). 2.0/400y passes clean (sustained-richest 267,702, well inside the
/// old baseline's own range) — verified against a 300-year run — see
/// `econ_diagnose_population_growth` (economy_validation.rs) and
/// docs/SCOREBOARD.md. Landed conservative on purpose; raising it further needs
/// its own iteration against this same gate, not a one-shot guess.
///
/// RAISED 2.8 → 6.0 (maintainer request: "world population must grow past ~8M").
/// This is SAFE against the dynamics gate by construction: `world_age_cap` below
/// (`disease.rs`) is gated on `has_prov`, and `simulate_decades_reports_dynamics`
/// seeds NO province layer, so it takes the hardcoded `else` = 2.0 branch and never
/// sees this constant at all. Only a real, provinced campaign (every generated
/// world) feels the higher ceiling; food-security still gates the first factor of
/// `cap_mult`, so a hub only reaches the taller ceiling if it is actually fed.
const WORLD_AGE_DEV_CAP: f32 = 6.0;
/// Years of campaign elapsed to earn ~63% of `WORLD_AGE_DEV_CAP`. Larger = slower
/// ramp (population takes longer to feel this headroom, so it keeps room to grow
/// later in a long campaign); smaller = faster ramp (saturates, and stops helping,
/// earlier).
const WORLD_AGE_DEV_REF_YEARS: f32 = 260.0;
/// PUBLIC HEALTH as a capacity lever — "fighting disease lets the city hold more
/// people". A hub's `public_health` (0..1) adds up to this much to its capacity
/// multiplier, so a city that invests in clean water / hospitals grows past the old
/// ~20-25k ceiling instead of being pinned there by the urban graveyard. Modest on
/// purpose: capacity feeds trade wealth, which the dynamics gate bounds.
const HEALTH_CAP_DEV: f32 = 0.8;
/// ── Trade GRAVITY ── how strongly a big / high-class hub PULLS trade from farther
/// afield and is preferred by merchants. A hub's `hub_pull` ≥ 1; its EFFECTIVE distance
/// to every other city = real distance ÷ pull, so a great entrepôt enters the partner
/// lists of cities twice as far away and wins more merchant dispatch.
const HUB_PULL_CLASS: f32 = 0.7;        // per hub_class step (0 town · 1 trade hub · 2 entrepôt)
const HUB_PULL_POP_REF: f32 = 50_000.0; // population giving +1.0 of pull (saturating)
const HUB_PULL_MAX: f32 = 3.5;          // cap so one metropolis can't pull the whole world
/// Net demographic drift (growth v2): a well-fed populace has a small birth
/// surplus so the TOTAL world population can actually grow (not just redistribute
/// via migration); dearth turns it negative. Applied on top of the logistic
/// approach-to-capacity, and only while below capacity, so wealth/pop stay bounded.
const BIRTH_RATE: f32 = 0.00009;   // ~+3.3%/yr at full food security (raised so a fed world grows)
const DEATH_RATE_BASE: f32 = 0.00002; // ~-0.7%/yr baseline mortality
// ── Provinces (Phase 2b · watershed demography) ─ all gated on a seeded province
//    layer, so the dynamics test (which never seeds provinces) is untouched. ──
/// Yearly rural natural increase toward the province's carrying capacity (pre-modern
/// countryside grows slowly, then hits a Malthusian ceiling).
const RURAL_GROWTH: f32 = 0.010;
/// Yearly share of a province's rural pool that migrates to its cities at full
/// pressure (a fuller countryside pushes harder; a stagnant one barely sheds people).
const RURAL_MIGRATION_RATE: f32 = 0.030;
/// Max yearly natural DECLINE of the very largest cities from crowding + endemic
/// disease (the "urban graveyard"): absent in-migration a metropolis shrinks, so it
/// depends on a fed hinterland. Public health mitigates it.
const URBAN_CROWDING_MORTALITY: f32 = 0.012;
/// Above this population the urban-graveyard mortality begins to bite (ramps to full
/// by ~+120k over it).
const URBAN_CROWD_FLOOR: f32 = 25_000.0;
/// Young settlement colonies grow this much faster organically (frontier boom).
const POP_GROWTH_COLONY_MULT: f32 = 2.2;
/// DEPOSITS_AND_MINING_PLAN.md slice 5 (the Potosí case) · a mining settlement
/// booms harder than an ordinary frontier colony while its ore is flowing — a
/// silver strike draws people the way no ordinary farmland does (Potosí
/// reached ~160,000 by 1600, among the largest cities on Earth, from nothing).
/// Kept modest relative to `POP_GROWTH_COLONY_MULT` (both apply together, since
/// a mining settlement IS a settlement colony) so the hard-asserted bounded-
/// wealth dynamics gate stays honest rather than chasing the historical peak.
const MINING_SETTLEMENT_GROWTH_MULT: f32 = 1.6;
/// D3 · a mining settlement whose food lifeline fails DECLINES, it never dies
/// outright the way an ordinary colony's `collapse_colony` does — the ore body
/// persists even when supply falters, so the town shrinks toward a floor.
const MINING_SETTLEMENT_DECLINE_MULT: f32 = 0.97; // per failed-lifeline check
const MINING_SETTLEMENT_FLOOR_FRAC: f32 = 0.20;   // of founding_pop, never below
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
/// Sub-cap villages are no longer FROZEN dots: each grows slowly (yearly) toward a modest
/// local ceiling, lifted by a prosperous, well-fed parent market and pulled down when that
/// market starves or dies — so the long tail of settlements breathes with the regional
/// economy instead of sitting at a fixed census number forever.
const HINTERLAND_BASE_CAP: f32 = 2_500.0; // a village's baseline ceiling (× market pull)
const HINTERLAND_GROWTH: f32 = 0.03;      // ~3%/yr toward the ceiling under a thriving market
const HINTERLAND_DECLINE: f32 = 0.05;     // yearly slip when the parent market fails/starves
/// ── ETHNOGENESIS (Cultures 2.0): a large, long-resident minority blends with the
/// local majority into a NEW creole people. Bounded so only a handful arise per campaign.
const CREOLE_MAX: usize = 24;              // global cap on live creole peoples
const CREOLE_MIN_POP: f32 = 4_000.0;       // only sizeable cities spawn creoles
const CREOLE_MIN_MINORITY: f32 = 0.30;     // the minority must be at least this share
const CREOLE_YEARLY_CHANCE: f32 = 0.08;    // per eligible city per year
const CREOLE_SEED_FRAC: f32 = 0.5;         // share of the minority that becomes the creole
/// VIGOROUS YOUTH — a creole is born with none of a hearth culture's structural
/// supports: no home province to keep feeding it via `province_demography_pass`'s
/// rural→urban migration (that pass carries the OLD `prov_culture` — fixed at
/// campaign start and never reassigned — into the very city a creole was just born
/// in, every single year, forever; see `cities.rs::province_demography_pass`), and it
/// starts as a small single-city minority quarter with no diaspora anywhere else. Left
/// alone this is a structural headwind a hearth culture never faces, so a creole dies
/// out almost as fast as it forms. A bounded, DECAYING bonus (linear to zero by
/// `CREOLE_VIGOR_YEARS`, applied only inside `assimilation_pass`) gives a young
/// creole a fighting chance: it resists being assimilated away while it is still a
/// minority somewhere, and it pulls other minorities (including a reintroduced old
/// majority) into itself faster while it holds a hub's majority. Fully decays to a
/// no-op by maturity — an old creole is governed by exactly the same rules as any
/// hearth culture (rule 18's "any new bonus needs a ceiling", applied here).
const CREOLE_VIGOR_YEARS: f32 = 40.0;      // the bonus decays linearly to nothing over this span
const CREOLE_VIGOR_RESIST: f32 = 0.5;      // at birth, halves its own assimilation-away rate
const CREOLE_VIGOR_PULL: f32 = 1.8;        // at birth, ×1.8 its pull on minorities while majority
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
// #3 · a majority's disposition toward its minorities scales the friction they cause:
// a Xenophobic or Insular people suppresses/chafes against outsiders (more unrest per
// minority), an Assimilative melting-pot people absorbs them (less). 1.0 = neutral.
const XENO_MINORITY_UNREST: f32 = 1.6;
const INSULAR_MINORITY_UNREST: f32 = 1.3;
const ASSIM_MINORITY_UNREST: f32 = 0.6;
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
// ── CRISIS RELIEF (`polis.rs::decide_crisis_relief`) ────────────────────────────
// The council's response to a dearth. Triggers deliberately sit EARLIER than
// `update_government`'s existing famine backstop (`starving > 0.5`, one good), which
// this layer sits above rather than replaces — see that function's own note.
/// Share of basic demand left unmet before the council calls it a dearth.
const RELIEF_LACK_TRIGGER: f32 = 0.25;
/// A food balance below this is a dearth even when the granaries still hold.
const RELIEF_BALANCE_TRIGGER: f32 = -0.10;
/// Above this share of the population starving, a dearth is a FAMINE.
const RELIEF_STARVE_TRIGGER: f32 = 0.25;
/// Share of the civic store released per month in a dearth / in a famine. A council
/// that empties its granary in one month has no second month, which is precisely the
/// mistake the historical *annona* boards were organised to avoid.
const RELIEF_RELEASE_DEARTH: f32 = 0.25;
const RELIEF_RELEASE_FAMINE: f32 = 0.50;
/// Don't bother releasing dust (and don't chronicle it).
const RELIEF_MIN_RELEASE: f32 = 1.0;
/// How long the export bar stands once imposed — two months, re-imposed monthly
/// while the famine lasts, so it lapses on its own when the crisis passes.
const RELIEF_EXPORT_LOCK_TICKS: u32 = 60;
/// N2 (`ACTORS_AND_CARRIAGE_PLAN.md` §3.2) · a non-food good's live price must
/// reach this multiple of its own base value before a council bars its export.
/// Shipped at `INFINITY` — provably dead code, exactly `N1_LOCAL_HAUL_BIND_DAYS`'s
/// pattern. A trial dose (6.0, `N2_BAN_TICKS` 30) was measured, not guessed: it
/// broke `simulate_decades_reports_dynamics`'s hard-asserted wealth bound (a
/// sustained richest house of 1,005,714 — a real "100k blow-up") even after
/// halving twice, which means the RENT a export-locked market hands its
/// resident monopolist is stronger than the plan's own gate ("a staple right is
/// a rent, and rents concentrate") anticipated — a structural finding, not a
/// dose-tuning one. Left at zero dose until that interaction is properly
/// measured; see `docs/ACTORS_AND_CARRIAGE_PLAN.md` §3.2 and `docs/SCOREBOARD.md`.
const N2_BAN_PRICE_RATIO: f32 = f32::INFINITY;
/// How long an N2 export ban stands once imposed, mirroring
/// `RELIEF_EXPORT_LOCK_TICKS` — it re-imposes monthly while the scarcity lasts
/// and lapses on its own once the price recovers.
const N2_BAN_TICKS: u32 = 60;

/// N5 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §1) · seasonal sailing/pass
/// closures as a per-lane travel-time MULTIPLIER, never a wall (§1.5 — a hard
/// closure can starve a city with no mechanism but `starving` to respond).
/// Four slices, not twelve: the mechanism is seasonal (a hemisphere's winter,
/// a monsoon reversal), not monthly, and `storm_season_phase` is a smooth
/// cosine that monthly sampling would not resolve any better (§1.2).
pub(crate) const SEASON_SLICES: u8 = 4;
/// `mult = 1.0 + v as f32 * SEASON_MULT_STEP`, so `v=0` is EXACTLY 1.0 — the
/// zero-dose gate — and the u8 range covers 1.00..4.98 at ~1.6% steps, finer
/// than a whole-day travel time can resolve.
pub(crate) const SEASON_MULT_STEP: f32 = 1.0 / 64.0;
/// A quantised multiplier may never exceed this — the delay-not-a-wall
/// discipline stated as a number: even the stormiest lane in its worst season
/// is a triple travel time, not an impassable one.
pub(crate) const SEASON_MAX_MULT: f32 = 3.0;
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
/// A culture must be the majority in at least this share of a trade region's cities
/// (population-weighted) for its tongue to become the region's LINGUA FRANCA.
const LINGUA_DOMINANCE: f32 = 0.34;
/// Cross-family assimilation multiplier a shared lingua franca grants — a second-
/// language bridge that lifts the 0.6× "distant family" kin toward parity, so
/// minorities integrate faster where a trade tongue is spoken (without full kinship).
const LINGUA_BRIDGE: f32 = 1.2;
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
/// How many houses may each plant their OWN outpost in a single yearly call — several
/// great houses can coexist in the same era, and each should reach for a site in ITS
/// own region rather than the whole world waiting on a single richest house whose
/// network may not even border whatever colonizable sites remain.
const OUTPOST_MAX_PER_CALL: usize = 3;
/// Score bonus that biases a house trade-outpost toward yielding a SCARCE manufacturing
/// input its own workshops lack — turning the outpost into a raw-materials RESOURCE
/// COLONY. Large enough to dominate the base per-capita score (which sits in 0..1).
const OUTPOST_INPUT_BIAS: f32 = 1.0;
/// A long-lived, thriving outpost (at the cap) whose wealthy house has held it this
/// many years may MATURE into a full colony (Phoenician emporion → city: Gadir, Utica).
const OUTPOST_GRADUATE_YEARS: u32 = 30;
/// …and its owning house must be at least this rich to make the investment.
const OUTPOST_GRADUATE_WEALTH: f32 = 60_000.0;
// ── Trade bases (houses develop EXISTING under-traded small cities).
//    The accessible cousin of the outpost: a house
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
// ── #1 · ARTISAN-GUILD WORKSHOPS ──────────────────────────────────────────────
// A manufactory founded on the city's own supply+demand (the diffuse city
// manufacturing already proves the raws arrive), not on a rich house's whim — so
// workshops are MANY and cluster at trade hubs. The demand gate is the guardrail:
// one only opens where the finished good is scarce, so its added supply self-limits.
/// A town needs at least this population to host a founded workshop.
const WORKSHOP_MIN_POP: f32 = 6_000.0;
/// The city must already make at least this much of the good (diffuse manufacturing),
/// which is proof its raw inputs are actually arriving/held.
const WORKSHOP_MIN_PROD: f32 = 0.02;
/// …and the good must be at least this dear vs its base value (under-supplied) to be
/// worth concentrating into a workshop. This is the self-limiting guardrail.
const WORKSHOP_MIN_DEMAND: f32 = 1.08;
/// An Artisan-trait people (majority or a real minority) raises a city's workshop
/// odds — renowned crafters draw the trade (#3's artisan-minority teeth).
const WORKSHOP_ARTISAN_BONUS: f32 = 1.6;
/// Founding cost charged to the owning guild/house (modest — a workshop, not a fleet).
const WORKSHOP_FOUND_COST: f32 = 900.0;
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
/// Per-year BASE chance an intact works suffers a disaster, and the magnitude
/// range every kind draws from — kept UNIFORM across kinds (`disaster_table`
/// only weights WHICH kind is picked and how fast it repairs) so a works'
/// first-ever disaster reproduces the pre-4.7 roll bit-for-bit. A first cut
/// gave each kind its own damage range; even a modest per-kind spread was
/// enough to perturb early wealth trajectories into a sustained-runaway-rich
/// house via this test's own known RNG-consumption-cascade sensitivity (the
/// same shape the 3.4a-c war-tuning story and inheritance-fragmentation fix
/// both hit) — reverted, not chased further, per §2.4's own discipline.
const DISASTER_ANNUAL_CHANCE: f32 = 0.035;
const DISASTER_MIN_DAMAGE: f32 = 0.30;
const DISASTER_MAX_DAMAGE: f32 = 0.70;
/// Fraction of outstanding damage repaired each year when the owner can fund it.
const REPAIR_RATE_PER_YEAR: f32 = 0.30;
/// Repair cost = (damage repaired) × (works value) × this.
const REPAIR_COST_FRAC: f32 = 0.5;

// ── ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.7 (D11/A9) · repair liability ──
// RESERVED, not currently wired in — see `estate_condition_pass`'s own doc
// comment on section 2b for why the reimbursement mechanic these feed was
// reverted (it flips `econ_inheritance_rules_fragment_differently`, this
// codebase's own documented RNG-cascade fragility, unrelated to this slice).
/// A share row that won't fund its slice of a repair loses this much frac,
/// redistributed to the rows that DID pay (D11 — the mechanism that lets a
/// disaster CHANGE ownership, not just dent output).
#[allow(dead_code)]
const DILUTION_STEP: f32 = 0.05;
/// A9 · a TENANCY that refuses its share this many years running is voided
/// outright, not merely diluted — persistent neglect ends the grant.
#[allow(dead_code)]
const TENANCY_NEGLECT_LIMIT: u32 = 3;

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
pub const OFFICE_LEASE_RENT: f32 = 0.05; // monthly, scaled by host city size
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

// ── House FOUNDING capital (Phase 0.1) ───────────────────────────────────────
// A new family separates out of its city's guild and takes a share of that guild's
// capital with it. Sized so it can survive its own first year: its initial fleet costs
// up to 3·SHIP_COST·FLEET_UPKEEP_FRAC ≈ 1.05/month, and `update_solvency` allows
// twelve months in the red, so anything under ~13 is a house born to die.
/// Share of the parent guild's wealth a separating family takes.
// ── Phase 1.1 · house tiers ──────────────────────────────────────────────────
/// `HOUSE_PEOPLE_AND_TIERS.md` §1. Tier 1 additionally requires this absolute standing
/// floor (not just rank) so a young world with no truly great house has an EMPTY Tier
/// 1 — a tier that is always occupied carries no information.
const TIER1_STANDING_ENTER: f32 = 0.55;
/// Hysteresis on the Tier 1 floor: a house already inside only drops out below this,
/// so it doesn't flicker across 0.55 forever.
const TIER1_STANDING_EXIT: f32 = TIER1_STANDING_ENTER - 0.04;
/// Percentile cutoffs among LIVE private houses (0 = the most prominent house, 1 = the
/// least): top 8% Great, next 22% Major, next 40% Lesser, the rest Marginal.
const TIER_PCT_CUTS: [f32; 3] = [0.08, 0.30, 0.70];
/// Dead band applied to whichever cutoff(s) border a house's CURRENT tier, so a score
/// sitting on a boundary doesn't relabel the house every month.
const TIER_PCT_DEAD_BAND: f32 = 0.04;
/// A house's combined captured-council + bailo + charter count is treated as "maxed"
/// on the `seats` term of the standing score once it reaches this many.
const TIER_SEATS_SOFT_CAP: f32 = 5.0;
const TIER_NAMES: [&str; 5] = ["", "great", "major", "lesser", "marginal"];

// ── CITY_PROVINCE_WAR_PLAN.md §3.2 · city tiers ─────────────────────────────────
// Mirrors the house-tier constants directly above, one for one, for the same
// reasons: an absolute Tier-1 floor so a young world has an empty Tier 1, and a
// dead band so a score sitting on a boundary doesn't relabel every month.
const CITY_TIER1_STANDING_ENTER: f32 = 0.55;
const CITY_TIER1_STANDING_EXIT: f32 = CITY_TIER1_STANDING_ENTER - 0.04;
const CITY_TIER_PCT_CUTS: [f32; 3] = [0.08, 0.30, 0.70];
const CITY_TIER_PCT_DEAD_BAND: f32 = 0.04;
const CITY_TIER_NAMES: [&str; 5] = ["", "great", "major", "lesser", "marginal"];
/// A decade (in months) of sustained Tier 1 + rising wealth → "a golden age" (§2.2).
const GOLDEN_AGE_MONTHS: u32 = 120;
/// "A dynasty of merchants" needs this many CONSECUTIVE closed heads in `line`, each
/// leaving the house richer than they found it, before it's chronicled (§2.2).
const DYNASTY_HEADS: usize = 3;

// ── Phase 0.4 · succession & the law of inheritance ─────────────────────────
/// Standing a house gains when a head who GREW the family is succeeded. A funeral is
/// not an achievement, and heads now turn over two to three times a century, so the
/// award is small and ceilinged (rule 18: prestige feeds political power → charters →
/// monopolies → wealth, and every uncapped per-event award has run away here before).
const SUCCESSION_PRESTIGE: f32 = 0.03;
const SUCCESSION_PRESTIGE_CAP: f32 = 1.2;
/// Age at accession by inheritance rule. An heir is not born on the day he inherits:
/// an eldest son takes over in his thirties, the hearth-keeping youngest as a young
/// man, an ELECTED elder ("the eldest capable") near sixty. Tenure is what remains of
/// a life from there, which is why these three numbers — not the death age — are what
/// makes ultimogeniture and seniority behave differently.
const HEIR_AGE_ELDEST: f32 = 27.0;
const HEIR_AGE_YOUNGEST: f32 = 17.0;
const HEIR_AGE_ELECTED: f32 = 44.0;
/// Age at death for someone who already survived to adulthood. Pre-modern life
/// expectancy AT BIRTH was low because so many died as infants; a merchant who lived
/// to hold a house commonly saw his sixties or seventies.
const HEAD_DEATH_AGE_MIN: f32 = 54.0;
const HEAD_DEATH_AGE_SPAN: f32 = 26.0;
/// No head rules for less than this, even when the roll puts death right after
/// accession — a three-week headship is a modelling artefact, not a generation.
const MIN_TENURE_YEARS: f32 = 4.0;
/// At most this many CO-HEIR houses may split off a single partible division, whatever
/// the heir count says. The division is a family splitting, not a fan-out.
const PARTIBLE_MAX_SPLIT: usize = 3;
/// Chance an agnatic house's succession is instead held by a WIDOW REGENT — the one
/// route to a female head a purely agnatic line otherwise has none of (Phase 2.1).
const WIDOW_REGENCY_CHANCE: f32 = 0.08;
/// Phase 2.4 · the cap on how far a head's CHARACTER may move any single decision
/// knob — §3's own number. At the axis extreme (±2) a knob moves by exactly this
/// much; at 0 (no roster, or a neutral roll) it is a no-op, which is what keeps
/// "all-zero character ⇒ bit-identical" true without a special case anywhere.
const CHARACTER_KNOB_CAP: f32 = 0.15;
// ── Phase 2.5 · stewards ─────────────────────────────────────────────────────
// A HIRED factor — a holding with no `posted` kin — is "able, and skims" (§2 of the
// design): a small fixed WAGE plus a small proportional SKIM, both scaled small
// enough to sit well under the family-overhead/warehouse-upkeep sinks already in
// place (OFFICE_LEASE_RENT=0.05, estate upkeep=0.15/month) rather than add a second
// wealth tax on top of them.
/// Monthly wage per hired (unposted) holding.
const STEWARD_WAGE: f32 = 0.08;
/// Monthly skim, per hired holding, as a fraction of the house's positive wealth.
/// Capped at `STEWARD_SKIM_HOLDINGS_CAP` holdings so a house with many hired
/// factors doesn't see the skim compound past a small efficiency loss.
const STEWARD_SKIM_RATE: f32 = 0.0006;
const STEWARD_SKIM_HOLDINGS_CAP: f32 = 3.0;
/// Monthly chance a single hired (unposted) office is POACHED away by a rival —
/// distinct from the ordinary "tie withered" closure.
const STEWARD_POACH_CHANCE: f32 = 0.01;

const HOUSE_SEED_GUILD_SHARE: f32 = 0.18;
/// Below this the guild is too poor to endow a viable family — no house is founded.
/// This is the brake that stops stillborn spawn churn at its source.
const HOUSE_SEED_MIN: f32 = 26.0;
/// Ceiling, so a fabulously rich guild does not mint an instant great house.
const HOUSE_SEED_CAP_MAX: f32 = 320.0;

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
/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.9 (A4) · a fresh captor OCCASIONALLY
/// bars foreign ownership outright to protect its new turf — the conquest/
/// capitulation origin the amendment names. Reuses the already-rare `captor !=
/// prev` trigger (§ "5) Payoff on a fresh capture") rather than inventing a
/// second one, so this stays as bounded as the favoured-house charter beside it.
const FOREIGN_BAR_ON_CAPTURE_CHANCE: f32 = 0.15;

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
//
// SCALED BY THE HOUSE'S OWN HOME-CITY SIZE (`city_size_factor`, 0.3x-4x on
// population/30,000) at the call site — was a flat absolute number, which meant it
// implicitly assumed a fixed economy size forever. `GUILD_WEALTH_SOFTCAP` already
// scaled this way (a few lines below, in `apply_wealth_sinks`); the private-house
// cap had been the one inconsistent case. Investigating population growth fixes
// surfaced this: a genuinely bigger, healthier economy (which those fixes enable)
// pushed a single house's wealth past this test's runaway guard even though the
// house was proportionally no richer relative to its city than before — the
// ceiling was tightening in real terms as the world grew, not staying constant.
const HOUSE_WEALTH_SOFTCAP: f32 = 60_000.0;  // wealth below this escapes the surcharge, at city_size_factor = 1.0
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
pub const UPKEEP_WAREHOUSE_BASE: f32 = 0.30;
/// An estate depot is cheaper to keep than a city warehouse (small rural store).
pub const UPKEEP_ESTATE_FRAC: f32 = 0.5;
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
pub const CAP_UPKEEP: f32 = 0.001;         // monthly upkeep per unit capacity (× city size)
const WEALTH_UPKEEP_RATE: f32 = 0.02;  // monthly overhead on wealth above the allowance
const WEALTH_UPKEEP_FREE: f32 = 30.0;  // wealth free of the family overhead
const WH_START_CAP: f32 = 600.0;       // a fresh depot starts a Tier-1 store
const WH_EXPAND_MULT: f32 = 1.6;       // capacity grows ×this per expansion
const WH_EXPAND_COST: f32 = 6.0;       // base wealth cost to enlarge (× current tier)
const WH_FULL_FRAC: f32 = 0.85;        // enlarge once fill ≥ this fraction
const WH_STOCK_FRAC: f32 = 0.25;       // share of a good's local surplus a house stocks/mo
const WEALTH_HISTORY_CAP: usize = 80;  // years of wealth samples kept per house
const HOUSE_EVENTS_CAP: usize = 60;    // most-recent chronicle entries kept per house
/// Ceiling on a house's MILESTONE entries — founding, successions, divisions, ruin.
/// These are never evicted by chatter (see `is_house_milestone`); only a family that
/// has outlived even this many milestones loses its oldest, and at ~4 per generation
/// that is roughly a thousand years of history.
const HOUSE_MILESTONE_CAP: usize = 120;
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
pub const FLEET_UPKEEP_FRAC: f32 = 0.05;
/// Per-vessel monthly chance of being lost to wear (rot, storms, breakdown) — the
/// slow decay of ships & caravans, so fleets must be continually replaced.
const FLEET_DECAY_CHANCE: f32 = 0.012;

// ── YARDS_VESSELS_AND_DEPOTS_PLAN.md · the yard, the vessel, and shares ──
// D1: a hull is built from a MATERIAL POOL (whatever suitable construction
// material reaches the city — grown locally or landed on the quay), never a
// fixed recipe. D2: a ship is not a good — it is owned, never traded, so it
// lives as a `Vessel`, not a `Distribution::Manufactured` good (rule 33 stays
// clean). D3: fractional ownership (the Venetian *carati*), not one whole
// hull per buyer.
/// The estate kind for a shipyard (S1). Reuses `create_estate`'s existing
/// lifecycle (ownership, damage/repair, the tier ladder) — no new machinery.
pub(crate) const YARD_ESTATE_KIND: u8 = 7;
/// A coastal/river city must clear this population before a yard is worth
/// founding — the same shape `WORKSHOP_MIN_POP` gates a manufactory at.
const YARD_MIN_POP: f32 = 8_000.0;
/// How much of a yard's parent city's LOCAL SURPLUS of a hull material it may
/// draw in one month — the same idiom `WH_STOCK_FRAC` uses for house
/// stocking, so a yard competes for timber the way the plan's D2 says it
/// should, never simply requisitions it.
const YARD_MATERIAL_DRAW_FRAC: f32 = 0.35;
/// Points of accumulated material needed to complete one hull. A yard drawing
/// its full monthly allowance from a well-stocked city clears this in a few
/// months to a couple of years — slower than `decide_fleets`' one-hull-a-month
/// ceiling (F3), which is the point: the yard is a SECOND, independent source
/// of capacity, not a faster version of the existing one.
const HULL_BUILD_POINTS: f32 = 60.0;
/// A `Vessel`'s ownership is split into this many parts (D3 — the Dutch
/// standardised at 1/64 *paerten*; Venice's *carati* were 24ths, but a power
/// of two keeps `parts_always_sum_to_64` exact under repeated integer splits).
pub(crate) const VESSEL_PARTS_TOTAL: u8 = 64;
/// S4, DOSE-WALKED (§2.8/D5): a shipment's cargo-space cost, as a fraction of
/// `SHIP_CAPACITY`/`BOAT_CAPACITY`/`CARAVAN_CAPACITY` per unit shipped, on top
/// of the existing one-slot-per-shipment rule. `0.0` is the shipped setting —
/// a TRUE no-op (F4 stays exactly as measured) — proven by
/// `n_yards_s4_capacity_bind_at_zero_is_a_noop` before any future dose walk.
pub(crate) const CAPACITY_BIND_DOSE: f32 = 0.0;
/// The guild axis (§2, "free"): a guild's charter is regional, a house's is
/// not (F5 — nothing currently distinguishes a Zunft from a Fugger). A guild
/// candidate in `house_for`'s dispatch is skipped past when the leg exceeds
/// this many travel-days, letting the search fall through to the next
/// candidate (another house, else the ownerless residual). `f32::INFINITY` is
/// the shipped, no-op setting — a guild is unbounded exactly as before this
/// change, verified by `n_yards_guild_axis_at_infinity_is_a_noop`.
pub(crate) const GUILD_CHARTER_RANGE_DAYS: f32 = f32::INFINITY;
/// Small-city trade rescue, DOSE-WALKED (§2.4/§2.8). `dispatch`'s target
/// shortlist is ranked by `gap * hub_pull(b)`, and `hub_pull` alone spans
/// 1.0..HUB_PULL_MAX — so a small town (hub_pull ~1) can hold the single
/// BEST raw arbitrage gap on a lane and still never place in gravity's top
/// 3 on any seller's list, ever ("the cities are just dead on the map").
/// Two direct fixes were tried and reverted first: removing the second
/// `hub_pull` weighting entirely, and halving it via `.sqrt()` — both broke
/// `simulate_decades_reports_dynamics`'s hard-asserted wealth bound (see the
/// doc comment at the `targets.push` call site in `production.rs::dispatch`),
/// because that weighting is real wealth-DISPERSION machinery, not a pure
/// bug. This is the "genuinely different mechanism" instead: an ADDED 4th
/// target slot carrying whichever reachable market held the single best
/// UNWEIGHTED gap, fired probabilistically (`hash01(a, g, tick) < DOSE`) so
/// it never replaces or reweights the existing gravity-ranked top 3 — pure
/// addition, not substitution. At `DOSE = 1.0` (always fire) this measurably
/// broke a SECOND, different gate: `econ_inheritance_rules_fragment_
/// differently` inverted (partible's mean wealth per house rose ABOVE
/// primogeniture's), because uniformly raising the trade floor for small
/// towns lifts partible's many small firms' mean more than primogeniture's
/// few large firms' — a structural tension with that gate's own premise, not
/// a tuning miss. Measured dose walk (`econ_inheritance_rules_fragment_
/// differently`'s partible-vs-primogeniture mean wealth, must stay
/// partible < primogeniture):
///
///   dose 1.00  177444 vs 124510  INVERTED (fails)
///   dose 0.30  237464 vs 200738  INVERTED (fails)
///   dose 0.05  275001 vs 290246  correct order (passes)
///
/// `simulate_decades_reports_dynamics`'s sustained-richest figure stayed
/// bit-identical to the pre-rescue baseline (278201) at 0.05 on the
/// dynamics test's own reference world — this dose is low enough that it
/// never actually fires there, and only engages on the larger/longer-run
/// world `econ_inheritance...` exercises.
///
/// **RE-MEASURED AND REVERTED TO 0.0.** Shipped at `0.05` on the strength of
/// the dose walk above (measured on `main` at the time), but once combined
/// with the rest of `main` at merge time (nothing in this file — the
/// interaction is with other commits landed around the same time) the gate
/// it was walked against started failing again: `econ_inheritance_rules_
/// fragment_differently` measured partible RICHER than primogeniture
/// (281944 vs 241796) with the dose still at 0.05. Toggling the constant
/// alone, nothing else, flips the result back to the passing 275267 vs
/// 306897 — confirmed directly, not inferred. This is the exact trap §2.4
/// warns about: a dose walk against one gate, measured at one point in the
/// commit graph, is not proof against every future combination of commits.
/// Back to `0.0` (a true no-op — every dispatch call already degrades to
/// the pre-rescue behaviour, verified by the same gate) until the small-
/// city-exclusion problem this was meant to fix gets a mechanism measured
/// AFTER the codebase it will actually ship alongside, not before.
pub(crate) const SMALL_CITY_RESCUE_DOSE: f32 = 0.0;
/// W2, DOSE-WALKED: the fraction of landed cargo that goes to the carrying
/// house's OWN depot at the destination (room permitting) instead of the
/// undifferentiated city pool (F8). `0.0` ships as a true no-op — every
/// arrival lands in the pool exactly as before — gated by
/// `n_yards_w2_landed_cargo_to_depot_at_zero_is_a_noop`.
pub(crate) const LANDED_CARGO_TO_DEPOT_DOSE: f32 = 0.0;
/// W3, DOSE-WALKED: the fraction of a depot's stock released back to the pool
/// in a month once the local price clears `WH_RELEASE_PRICE_MULT` (F6 — today
/// there is no ordinary sale out of a depot at all). `0.0` ships inert.
pub(crate) const WH_RELEASE_DOSE: f32 = 0.0;
/// W3 · the price threshold (× the good's `base_value`) that counts as "dear
/// enough to sell into" once `WH_RELEASE_DOSE` is raised above zero.
const WH_RELEASE_PRICE_MULT: f32 = 1.6;
/// W4, DOSE-WALKED: whether an office may ship stock from one of its OWN
/// depots to another on its own account (needs S2's `Vessel`s to be more than
/// bookkeeping, per the plan's own "the one dependency worth naming"). `false`
/// ships inert.
pub(crate) const DEPOT_TO_DEPOT_TRANSFER_ENABLED: bool = false;
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
    // Mined/quarried goods — every `Distribution::Deposits` mineral this campaign
    // ships (§8.16), not just the original six. `TickGood` (unlike the world-side
    // `GoodSpec`) carries no `distribution` field, so this stays a name table —
    // but it had gone stale: the gem-split (ruby/sapphire/emerald/diamond/
    // amethyst/topaz/garnet/carnelian), tin, lead, marble, jade and the
    // DEPOSITS_AND_MINING_PLAN slice-3 minerals (mercury, alum, lapis_lazuli,
    // turquoise) all fell through to "any other cultivated trade good" below and
    // got founded as a Plantation instead of a Mine — the wrong depletion curve
    // in `dominant_estate_kind` (cities.rs §2.5: a mine barely recovers, a
    // plantation just wears soil) and the wrong label in the UI/journal. Keep
    // this in sync with `default_list()`/`default_custom_goods()`'s Deposits set.
    //
    // DEPOSITS_AND_MINING_PLAN.md slice 4 (mine vs quarry) · split into TWO real
    // kinds instead of one. A MINE (2) is a deep-shaft body — depth/drainage is
    // its constraint (`mine_depth`/`MINE_UPGRADE_COST_MULT`). A QUARRY (8) is a
    // near-surface working — stone, gems, salt, amber — whose constraint is
    // TRANSPORT (heavy `bulk`, useless far from water), never depth; it never
    // reads `mine_depth` at all (only `estate_kind == 2` does).
    if n.contains("iron") || n.contains("ore") || n.contains("copper") || n.contains("silver")
        || n.contains("gold") || n.contains("coal") || n.contains("tin") || n.contains("lead")
        || n.contains("mercury") { return 2; }
    if n.contains("gem") || n.contains("salt") || n.contains("amber") || n.contains("stone")
        || n.contains("marble") || n.contains("jade") || n.contains("lapis") || n.contains("turquoise")
        || n.contains("alum") || n.contains("ruby") || n.contains("sapphire")
        || n.contains("emerald") || n.contains("diamond") || n.contains("amethyst") || n.contains("topaz")
        || n.contains("garnet") || n.contains("carnelian") { return 8; }
    if food { return 1; }
    // Any other cultivated trade good (silk, spices, cotton, sugar, tea, coffee, …).
    3
}

/// Short label for an estate kind (for journal text + the inspector).
pub fn estate_kind_label(kind: u8) -> &'static str {
    match kind {
        1 => "Farm", 2 => "Mine", 3 => "Plantation", 4 => "Fishery", 5 => "Vineyard",
        6 => "Manufactory", 7 => "Yard", 8 => "Quarry", _ => "Estate",
    }
}

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (D15) · the five-step label a
/// `yield_index` reads as — "the label leads" (shown before the number, the
/// house dossier's own "pips and a phrase, never a raw 0..1" convention).
pub fn yield_label(yield_index: f32) -> &'static str {
    match yield_index {
        y if y >= 3.0 => "world-class",
        y if y >= 1.8 => "great",
        y if y >= 1.1 => "notable",
        y if y >= 0.5 => "ordinary",
        _ => "marginal",
    }
}

/// A3 · yield_label's own GREAT/world-class floor — the one threshold both the
/// chronicle pass and the query layer must agree on, so a card never shows a
/// brand the chronicle didn't (or won't) also announce.
pub const BRAND_YIELD_FLOOR: f32 = 1.8;

/// A3 · the PLACE half of a toponymic brand — the works' own name with its
/// kind suffix stripped ("Vetrani Vineyard" ⇒ "Vetrani"), since `create_estate`
/// already names every works "{owner} {kind}" and a real toponym generator is
/// out of scope for what A3 calls "the cheapest thing in the document".
pub fn brand_place(hub_name: &str, kind_label: &str) -> String {
    let suffix = format!(" {kind_label}");
    hub_name.strip_suffix(suffix.as_str()).unwrap_or(hub_name).trim().to_string()
}

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.7a (A8, recalibrated) · which
/// disasters a work of this KIND can suffer: (name, pick weight among this
/// kind's own options, repair-rate multiplier — <1 slower than
/// `REPAIR_RATE_PER_YEAR`, >1 faster). The weight decides which NAME is drawn
/// once a disaster is already rolling, not how OFTEN one strikes — every kind
/// still draws its damage from the same `DISASTER_MIN_DAMAGE..MAX_DAMAGE`
/// range (see that constant's own doc for why). FLOODING DOMINATES a mine's
/// picks (A8 — water was THE constraint on pre-modern mining, "the common
/// case, not a rare shock") rather than sharing equal odds with collapse.
/// "Blight" is renamed FROST/HAIL for a vineyard (A8 — phylloxera/downy
/// mildew are both centuries too late for this world) and, uniquely, also
/// dents the vineyard's own grade, not just its output (handled by the
/// caller). Farm and plantation share MURRAIN (A8's own well-chosen
/// livestock-panzootic example) since neither carries a distinct "pasture"
/// kind in this engine. Farm's plain seasonal DROUGHT and any kind's WAR
/// sack/raid are deliberately NOT duplicated here — see the doc comment on
/// `estate_condition_pass`.
fn disaster_table(kind: u8) -> &'static [(&'static str, f32, f32)] {
    match kind {
        2 => &[("flooding", 2.2, 0.5), ("collapse", 0.6, 0.8)],
        4 => &[("storm wreck", 1.0, 1.3)],
        5 => &[("frost", 0.8, 0.25), ("hail", 0.5, 0.6)],
        6 => &[("fire", 1.0, 0.8)],
        7 => &[("fire", 1.1, 0.7)], // a timber yard burns readily and rebuilds slowly
        8 => &[("rockfall", 0.9, 0.6)], // a quarry face, not a shaft — no flooding
        1 | 3 => &[("murrain", 0.7, 0.6)],
        _ => &[("fire", 0.5, 1.0)],
    }
}

/// A3 · "Kalos wine", "Upper Vein copper" — the place, then the good in lower
/// case (a proper noun followed by a common one, matching every real example
/// A3 itself cites).
pub fn brand_name(place: &str, good_name: &str) -> String {
    format!("{place} {}", good_name.to_lowercase())
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

// ── ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.2 · spoilage & city warehouse ───
// Monthly spoilage rate per unit of `GoodSpec.perishable` — calibrated so
// wheat's shipped 0.02 perishable reads ≈1%/month, the period anchor §9.1
// cites (Allen/Persson-era granary loss). A very perishable good (fresh
// herring, 0.55) would imply 27.5%/month uncapped; SPOIL_RATE_CAP holds the
// worst case to something a stock column can still show meaningfully.
const SPOIL_PER_PERISHABLE: f32 = 0.5;
const SPOIL_RATE_CAP: f32 = 0.30;
/// A Granary halves FOOD spoilage; a Warehouse halves spoilage generally —
/// both structures already exist for a production bonus (`WORKSHOP_PROD`
/// above); this gives them a second, thematically obvious job instead of a
/// new one.
const SPOIL_GRANARY_FOOD_MULT: f32 = 0.5;
const SPOIL_WAREHOUSE_MULT: f32 = 0.6;
/// Stock held past the city's own `wh_capacity` spoils at this extra
/// multiplier (linearly ramped to it as the overflow fraction reaches 100%
/// over capacity) — an overflowing store rots faster than a well-kept one.
const SPOIL_OVERFLOW_MULT: f32 = 2.0;
/// City warehouse capacity (D17/F6) — continuous in population rather than a
/// house depot's discrete tier ladder (a city's stores grow WITH the city,
/// not in steps bought one at a time), plus a flat bonus per Granary/
/// Warehouse structure actually built.
const CITY_WH_CAP_PER_POP: f32 = 0.08;
const CITY_WH_CAP_BASE: f32 = 400.0;
const CITY_WH_STRUCT_BONUS: f32 = 1_500.0;
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

// ── v2.1 · BIMETALLISM — metal-weight-aware exchange ─────────────────────────
/// The historical gold:silver value ratio: a gold coin is worth ~this many silver
/// coins of equal weight & fineness (the medieval/Renaissance band was ~10–15:1).
pub const GOLD_SILVER_RATIO: f32 = 13.0;
/// v2.1 · the INTRINSIC specie value of one coin, in the silver-coin numeraire
/// (a full-bodied silver coin = 1.0). Driven by the metal struck × its fineness —
/// so a gold Ducat is worth ~`GOLD_SILVER_RATIO`× a silver Florin of equal fineness,
/// electrum sits between, and bronze/billon is token money. This is the metal a coin
/// actually contains; cross-coin exchange rates derive from it (was previously
/// ignored — coins traded purely on fineness×trust, so gold and silver mis-priced 1:1).
pub fn coin_specie(metal: u8, fineness: f32) -> f32 {
    let f = (if fineness <= 0.0 { 1.0 } else { fineness }).clamp(0.0, 1.0);
    let m = match metal {
        1 => GOLD_SILVER_RATIO,                 // gold
        2 => 0.5 * (GOLD_SILVER_RATIO + 1.0),   // electrum (gold+silver alloy)
        3 => 0.08,                              // bronze/billon — token money
        _ => 1.0,                               // silver standard
    };
    m * f
}
/// v2.1 · a coin's metal-aware EXCHANGE value: its intrinsic specie worth times a
/// small acceptance agio (a trusted coin trades a touch above, a distrusted one
/// below, its metal content). This is the number cross-coin rates are read from.
pub fn coin_exchange(metal: u8, fineness: f32, trust: f32) -> f32 {
    coin_specie(metal, fineness) * (0.9 + 0.2 * trust.clamp(0.0, 1.0))
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
/// inflation-tax finite). v2.1 · widened the UPPER tail so genuine monetary crises
/// bite: a badly-debased, over-issued coin now runs real inflation up to 9%/yr. The
/// small positive floor is kept unchanged — it doubles as the inflation-tax floor that
/// helps bound hoarded fortunes, so lowering it into deflation is deliberately avoided.
const INFL_MIN: f32 = 0.002;
const INFL_MAX: f32 = 0.09;

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
const MINT_FINENESS_FLOOR: f32 = 0.72;

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
/// #4 · trade HORIZON, as a fraction of world width. The route matrix marks any pair
/// of seats farther apart than this (cylindrical straight-line) as unreachable, even
/// when the worldgen pathfinder found a sea lane between them. This is a pre-colonial,
/// pre-oceanic-navigation economy: goods move regionally — around a sea, along a
/// coast, overland — not on trans-oceanic lanes between continents. Generous enough
/// to keep Mediterranean-/regional-scale sea trade while cutting the ocean crossings.
const TRADE_MAX_DIST_FRAC: f32 = 0.24;
/// #6 · NO DEAD CITY. The trade horizon + the cross-component gate above can strand a
/// remote inland/lake town with only ONE reachable partner (or none), so it never has
/// a price gradient to trade against — throughput/exports/imports sit at zero and its
/// population freezes for the whole campaign (the "static province" bug). Every real
/// hub is therefore guaranteed at least this many of its NEAREST partners regardless of
/// the horizon, so neighbourhood trade always reaches it. Bounded to the closest hubs,
/// so it restores LOCAL/regional trade for a stranded town without reopening the
/// trans-oceanic lanes the horizon exists to cut.
const MIN_GUARANTEED_PARTNERS: usize = 4;
/// #6b · HUB-AND-SPOKE MARKET LIFELINE. Being CONNECTED is not the same as being able
/// to TRADE: a specialized town ringed by identical producers (a homogeneous coastal or
/// tundra bubble) has no price gradient to export into, and the complementary goods it
/// wants are produced beyond its regional horizon — so it shows partners but zero
/// exports/imports. Real pre-modern trade is hub-and-spoke: such a town ships to, and
/// imports through, a MARKET where diverse goods aggregate. Every market-starved hub is
/// therefore guaranteed a route to the nearest few markets — but strictly WITHIN ITS OWN
/// geographic COMPONENT, so a remote region (a far-arctic coast, an isolated sea) forms
/// its OWN distinct trade network rather than being wired across an ocean to a foreign
/// emporium. Markets are the top `MARKET_TOP_FRAC` of hubs by population IN EACH COMPONENT,
/// rounded UP so every region — however small or poor — has at LEAST one of its own; that
/// per-component guarantee is what actually gives a remote region its own distinct network.
/// The fraction is left at its original value on purpose: the economy-fidelity gate's
/// synthetic reference world is a SINGLE component, where per-component-top-15% equals the
/// old global-top-15% and this whole restructure is bit-identical — raising the fraction
/// there added market lanes that HALVED partible-inheritance fragmentation (82 → 46 houses
/// ever), tripping `econ_inheritance_rules_fragment_differently`. On a real multi-component
/// world the per-component rounding already makes markets more abundant than the old global
/// top-15% (every landmass/sea now carries its own emporium).
const MARKET_TOP_FRAC: f32 = 0.15;
/// How many nearest major markets a market-starved hub is linked to.
const MARKET_LINKS: usize = 2;
/// A secondary SANITY cap on the market lifeline's straight-line length, as a fraction of
/// world width. The same-component gate is the real bound now (a link never crosses open
/// ocean between two separate landmasses); this only stops a pathological line straight
/// across a very large component's own gulf.
const MARKET_REACH_FRAC: f32 = 0.5;
/// #6c · COASTAL CABOTAGE. The cross-component gate (#4/#6/#6b) rightly keeps trade off
/// long trans-oceanic lanes — but it also strands a SMALL ISLAND or near-shore coastal
/// region that worldgen's pathfinder never joined by sea, leaving its towns dead from
/// the start even though the mainland is a short crossing away. Pre-modern economies ran
/// SHORT sea hops constantly (cabotage, coastal and inter-island trade); only the long
/// ocean crossing didn't exist. So a COASTAL hub is linked to the nearest coastal hubs of
/// OTHER components within `CABOTAGE_SEA_FRAC` of world width — a deliberately SHORT
/// crossing (a third of the #4 horizon), so a near-shore island joins the mainland's
/// trade while two continents an ocean apart still do not. Cross-component only, so on a
/// single-component world (the econ-fidelity reference) it is a strict no-op.
const CABOTAGE_SEA_FRAC: f32 = 0.08;
/// TECTONICS_AND_ISOLATION_PLAN.md Part A — the maximum distance `rescue_tiny_
/// components` may fold a tiny (<3-hub) component into a substantial one. Set
/// well above `CABOTAGE_SEA_FRAC`'s short hop (that pass already covers near-shore
/// islands) but nowhere near an ocean crossing: a regional sea, not a crossing
/// between continents. Stated in km (rule 25), converted per world.
const ISOLATION_RESCUE_MAX_KM: f32 = 1800.0;
/// How many nearest cross-component coastal partners a coastal hub gains by cabotage.
const CABOTAGE_LINKS: usize = 2;
/// WORLD_AND_TRADE_MASTER_PLAN.md Part II Slice C1 (the entrepôt) — the real cost
/// (in days) of breaking bulk at an outlet: unloading, warehousing, re-loading.
/// A composed two-leg route must clear this before it can ever beat a direct one,
/// which is what keeps the outlet a genuine shortcut rather than a free relabel.
pub(crate) const ENTREPOT_DWELL_DAYS: f32 = 3.0;
/// Share of a settled trade's PROFIT (never the gross value — rule 18's "never
/// added on top") the outlet port earns for having made the cheaper route
/// possible, credited to its treasury. Kept modest: this is a toll on passing
/// trade, not a second sale.
pub(crate) const ENTREPOT_FEE_FRAC: f32 = 0.08;
/// Terroir estates (wine country, silk hills, spice coast…): the estate founder used
/// to pick a good ONLY from what the frozen worldgen snapshot credited the CITY hub
/// with producing (`base_per_capita > 0`), so a province rich in a good the snapshot
/// missed — the classic case being a fine wine belt a cell or two outside the seat's
/// own catchment — could never seed the vineyard it plainly warranted. A good the city
/// does not itself produce may now be planted where the PROVINCE's own belt score
/// (`prov_good_belt`) is at least this suitable; below it the ground isn't worth it.
const ESTATE_TERROIR_BELT_MIN: f32 = 0.35;
/// A full-strength (belt = 1) terroir good is worth this fraction of the city's MEAN
/// specialty per-capita rate. Self-scaling to the world (so it competes on the city's
/// own production scale, never an unrelated absolute one) and < 1 so a terroir estate
/// can never out-produce the city's real specialties. See `maybe_found_estate`.
const ESTATE_TERROIR_FRAC: f32 = 0.6;
/// DEPOSITS_AND_MINING_PLAN.md slice 4 · how far from a Mine estate's parent city
/// (an estate is co-located with its parent, rule 32) a real working still counts
/// as "this city's own deposit" — a real district's own scale
/// (`DISTRICT_RADIUS_KM`, §8.16) plus margin for the founding search itself.
pub(crate) const MINE_DEPOSIT_SEARCH_KM: f32 = 60.0;
/// DEPOSITS_AND_MINING_PLAN.md slice 5 · the "growing settlement catchment"
/// knob (`CampaignSim::catchment_radius_km`). The honest pre-modern number: a
/// cart hauls grain economically ~30–50 km, so a catchment does not scale
/// freely with population — it can only creep outward slowly (better roads,
/// more carters), never leap.
const CATCHMENT_GROWTH_PER_YEAR_KM: f32 = 0.15;
/// Total lifetime growth caps out here — +10–20 km on the 50–120 km base, per
/// the plan's own number.
const CATCHMENT_MAX_GROWTH_KM: f32 = 20.0;
/// D7 · a mine's `estate_tier` upgrade cost multiplier by `mine_depth` (0..3 =
/// surface/shallow/deep/flooded). Digging deeper needs real drainage capital
/// (Rio Tinto's reverse waterwheels, Agricola's *De Re Metallica*), so a flooded
/// body grows far more slowly than a surface one at the same wealth — see
/// `maybe_house_invests`'s upgrade branch.
const MINE_UPGRADE_COST_MULT: [f32; 4] = [1.0, 1.3, 2.2, 3.5];
/// D7 table · a Quarry founded away from a coast or navigable river (no cheap
/// bulk haul for heavy stone) costs this much more to expand — "useless far
/// from water" made a real number instead of a line in a design doc. Mons
/// Claudianus (a state-funded desert quarry 120 km from any water) is the
/// named exception, not the rule this constant encodes.
const QUARRY_INLAND_UPGRADE_COST_MULT: f32 = 2.0;
/// Mercury consumed per unit of silver output by amalgamation (grain-equivalent
/// value terms, not a physical mass ratio — the sim has no units finer than
/// that). Small: mercury is a catalyst-scale input historically, not a bulk one.
const MERCURY_PER_SILVER: f32 = 0.12;
/// Silver recovery with NO mercury on hand — hand-smelting still works some of
/// the ore, just less completely.
const MERCURY_AMALGAMATION_FLOOR: f32 = 0.75;
/// Silver recovery fully supplied with mercury — real amalgamation recovers ore
/// a hand-smelt leaves behind.
const MERCURY_AMALGAMATION_BONUS: f32 = 1.25;
/// Global ceiling on satellite production sites (estates + colonies). Estates are
/// real hubs in `self.hubs`, so an uncapped count quadratically slows every tick.
const MAX_TOTAL_ESTATES: usize = 220;
/// Ordinary estates (`maybe_found_estate`, which runs far more often than a trade
/// outpost is ever founded) stop at `MAX_TOTAL_ESTATES` minus this — the last slice
/// of the shared budget is reserved so a saturated world can still occasionally
/// plant an outpost. A diagnosed bug (150-year reference run): with no reservation,
/// ordinary estates filled every one of the 220 slots by roughly year 40 and NO
/// outpost ever founded again afterward, even with a wealthy house and unused
/// `colonizable` sites — `maybe_found_house_outpost` shares the exact same global
/// check and was starved forever, not merely rarely.
const OUTPOST_RESERVED_ESTATES: usize = 20;
/// A trade outpost used to work only a good its FOUNDER already produced, so a good
/// no house had ever learned to make — a fine wine/cotton/sugar belt sitting outside
/// every existing city's catchment, the exact "ABSENT" case the goods codex flags —
/// stayed unexploited forever, because the one path that could reach it copied the
/// founder's own output. An outpost may now be founded specifically to OPEN such a
/// trade: if its site's province is at least this suitable (`prov_good_belt`) for a
/// non-food good the WORLD barely produces, the outpost works THAT good instead.
const OUTPOST_EXPLOIT_BELT_MIN: f32 = 0.30;
/// A good counts as "unexploited" (worth opening with an outpost, above) while the
/// total per-capita output summed across every live hub stays under this — i.e. no
/// city has a real industry in it yet. Deliberately small so a good with even one
/// modest producer is left to the ordinary estate path.
const OUTPOST_EXPLOIT_PROD_MAX: f32 = 0.05;
/// Weight of the unexploited-belt pull in site scoring — how strongly an outpost is
/// drawn toward a province that could open a valuable new trade, relative to a site's
/// ordinary trade value. Big enough to redirect a founder toward wine/cotton/sugar
/// country, small enough not to overrule a genuinely rich, already-worked coast.
const OUTPOST_EXPLOIT_SITE_BONUS: f32 = 0.8;

pub const SHIP_COST: f32 = 7.0;
const RIVER_COST: f32 = 4.5;
const CARAVAN_COST: f32 = 4.0;
/// `decide_fleets` only ever executes ONE branch of its buy/sell if-else chain a
/// month, so a house with capital to spare for ten hulls still buys exactly one —
/// a structural ceiling of ~12 hulls/year however rich the house
/// (`ACTORS_AND_CARRIAGE_PLAN.md` §1: measured 2.4 hulls/house on the reference
/// world). `FLEET_BUY_MAX_PER_MONTH` lets a house buy MORE THAN ONE in the same
/// month when its idle capital (after this month's purchases) still clears the
/// buy threshold — each purchase re-checks affordability against the wealth
/// already spent this call, so a house can never buy past what it can actually
/// afford. `simulate_decades_reports_dynamics` and `econ_inheritance_rules_
/// fragment_differently` both held at 3 (a higher buy rate alone does not
/// invert the inheritance gate the way `CHARTER_EXCLUSIVE_DOSE` beside it
/// does), but 3 broke two smaller, exact fixtures elsewhere in the suite
/// (`coinage_runs_yearly_finite_and_deterministic`,
/// `a_house_records_every_head_it_has_had`) whose specific expected outcome
/// this session had no time left to re-derive or re-tune for. Shipped at 1,
/// which reproduces the exact one-hull ceiling above
/// (`fleet_buy_cap_bounds_a_wealthy_house_at_the_shipped_dose`) — real, wired,
/// dead code today; the dose walk up is real future work.
const FLEET_BUY_MAX_PER_MONTH: u32 = 1;

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
// v2.1 · reserve-preference multipliers by struck metal (a modest edge — the full
// bimetallic value ratio lives in `coin_exchange`, not in adoption).
const COIN_METAL_GOLD_PREF: f32 = 1.35;     // gold — the reserve/prestige money
const COIN_METAL_ELECTRUM_PREF: f32 = 1.20; // electrum (gold+silver alloy)
const COIN_METAL_BRONZE_PREF: f32 = 0.70;   // bronze/billon — shunned as a store of value
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
/// BASE monthly interest a bank charges on loans / pays on deposits.
const BANK_LOAN_RATE: f32 = 0.012;
const BANK_DEPOSIT_RATE: f32 = 0.006;
// ── v2.1 · ENDOGENOUS loan pricing (rate set per origination, not flat) ──────
/// How much tight credit (little lending headroom left) lifts the loan rate.
const BANK_RATE_SCARCITY: f32 = 0.9;
/// How much a riskier borrower/purpose lifts the loan rate (risk premium).
const BANK_RATE_RISK: f32 = 0.6;
/// Extra premium charged while the seat is in a financial panic (credit crunch).
const BANK_RATE_PANIC: f32 = 0.7;
/// Ceiling on the priced monthly loan rate (≈ a punishing ~40%/yr in a crunch).
const BANK_LOAN_RATE_MAX: f32 = 0.028;
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
/// A3 · years of yearly coin-biography snapshots kept per mint (bounds save size).
const COIN_HISTORY_CAP: usize = 80;
// ── B4 · bills of exchange (FX-spread bank income) ───────────────────────────
/// The fee a bank captures on cross-coin settlement between two branch markets,
/// as a fraction of (relative exchange gap × the lighter market's throughput). Small
/// — it is a friction on trade, and flows out through dividends + the wealth tax.
const BILL_FEE: f32 = 0.0016;
/// Monthly cap on a single bank's bills income (keeps FX profit bounded).
const BILL_INCOME_CAP: f32 = 8.0;
// ── B3 · civic PUBLIC DEBT (the Monte / Casa di San Giorgio) ─────────────────
/// Public-debt markets open once civic institutions have matured (~year 15).
const DEBT_START_TICK: u32 = 15 * 365;
/// The sticky annual coupon a city pays its bondholders (~5–6%, the Monte's band).
const DEBT_COUPON: f32 = 0.055;
/// The STANDING public debt a mature commercial city funds, as a multiple of its
/// yearly throughput — the Monte was a permanent institution, not just a war measure.
const DEBT_TARGET_RATIO: f32 = 0.6;
/// Most a city grows its debt toward the target in one year (× throughput).
const DEBT_ISSUE_STEP: f32 = 0.15;
/// Serviceability gate: a city only issues if (treasury + proceeds) covers this many
/// years of the resulting coupon — so it never borrows past what it can service.
const DEBT_SERVICE_COVER: f32 = 4.0;
/// Debt is issued up to this multiple of the city's yearly throughput (hard cap).
const DEBT_MAX_RATIO: f32 = 2.0;
/// Above this debt-to-throughput ratio the city can no longer service it → a haircut.
/// Set high so a default is a genuine fiscal COLLAPSE (throughput has cratered under a
/// standing debt), not a routine event — otherwise the trust hit cascades into banking.
const DEBT_DEFAULT_RATIO: f32 = 5.0;
/// Fraction of principal wiped from every holder in a debt default.
const DEBT_HAIRCUT: f32 = 0.3;
/// Trust hit to the city's coin when it defaults on its bonds (credit-standing loss).
/// Small — a debt default dents the coin, it does not by itself topple the banks.
const DEBT_DEFAULT_TRUST_HIT: f32 = 0.04;
/// A city deleverages (retires principal, returning capital to holders) once its debt
/// runs past this multiple of the target ratio — heading OFF a default while it can.
const DEBT_DELEVERAGE_RATIO: f32 = 1.6;
/// Max distinct bondholders tracked per city (bounds save size; new lenders merge).
const DEBT_HOLDER_CAP: usize = 8;
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
/// CITY_PROVINCE_WAR_PLAN.md §3.4b · the victor takes ONE province the loser held
/// (`prov_holder` reassigned) — short of annexing the whole city. Reuses Phase 5's
/// `prov_holder` exactly as a peacetime grant would; a house-held province
/// (`prov_holder_house >= 0`, rule 24) is never up for grabs in a city-vs-city war.
const WAR_GOAL_PROVINCE: u8 = 4;
/// R4 (`REALM_AND_GOVERNMENT_PLAN.md` §1.5) · a purely reputational defeat — no
/// land or coin changes hands beyond ordinary reparations, but the loser's own
/// standing visibly cracks: a realm's `legitimacy`, or (for an ordinary house-led
/// city) its ruling house's `prestige`.
const WAR_GOAL_HUMILIATE: u8 = 5;
/// R4 · the victor's own kin are seated in the loser's government — a real,
/// LOCKED office (`Official.kin`), not merely a bailo. A puppet, not a conquest:
/// the loser keeps its coin, its market, its nominal independence.
const WAR_GOAL_ENTHRONE: u8 = 6;
/// R4 · stronger than tribute: the loser also fights in its overlord's wars and
/// may not declare its own. Only produces the FULL relationship (`Realm.vassals`,
/// `REALM_ROLE_TRIBUTARY`) when the winner itself has a realm to be a vassal OF —
/// downgrades to plain tribute otherwise, the same "richest thing actually
/// available" idiom `WAR_GOAL_PROVINCE` already uses when there's no province.
const WAR_GOAL_VASSALIZE: u8 = 7;
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
        WAR_GOAL_PROVINCE => "for a province",
        WAR_GOAL_ANNEX => "for annexation",
        WAR_GOAL_HUMILIATE => "to humiliate",
        WAR_GOAL_ENTHRONE => "to enthrone a puppet",
        WAR_GOAL_VASSALIZE => "for vassalage",
        _ => "for plunder",
    }
}
/// Yearly chance a new war is declared (when below the cap), before the §3.4c
/// warmonger-ruler bias (`head_character_factor`, axis 0, ±`CHARACTER_KNOB_CAP`).
const WAR_DECLARE_CHANCE: f32 = 0.10;
/// Yearly forced levy on each resident house's wealth → the city war chest. The
/// principal way war drains over-rich houses.
const WAR_LEVY_RATE: f32 = 0.12;
/// Fraction of the treasury a belligerent burns on the war effort each year
/// (consumed — the destructive cost of armies & blockade).
const WAR_SPEND_RATE: f32 = 0.30;

// ── CITY_PROVINCE_WAR_PLAN.md §3.4a · the score & round engine ──────────────────
// Same shape as the succession-crisis engine (`crisis.rs`): a fixed round cap is
// the termination guarantee of LAST RESORT (rule 22's discipline applied to war),
// with faster, more legible paths expected to end most wars well before it.
/// Ticks between quarterly rounds — identical cadence to `CRISIS_ROUND_TICKS`.
const WAR_ROUND_TICKS: u32 = 90;
/// ORDINARY war length: 3 years of quarterly rounds. Past this a war does NOT end
/// automatically — a war of attrition between two rich, determined states can grind on
/// (which is natural). It only settles at this cap once a side's chest runs low (below
/// `WAR_ATTRITION_MIN_CHEST`); otherwise it continues, up to `WAR_ROUND_HARD_CAP`.
const WAR_ROUND_CAP: u16 = 12;
/// The ABSOLUTE ceiling (8 years of rounds): a war can never grind past this, so rule
/// 22's "no war is ever permanent" guarantee still holds no matter how flush both sides
/// are. Most wars end far sooner via a decisive score, exhaustion or war-weariness (a
/// long war saps morale below `WAR_MOOD_WEARY_FLOOR` on its own); this only backstops a
/// genuine deadlock between two sides that both stay funded AND willing.
const WAR_ROUND_HARD_CAP: u16 = 32;
/// Past `WAR_ROUND_CAP`, a belligerent keeps fighting only while its war-affordable
/// treasury stays above this floor (2× the declare floor). Below it the side is running
/// its chest low — not yet fully exhausted, but no longer able to sustain a long war —
/// so the war settles at the ordinary cap rather than dragging to the hard ceiling.
const WAR_ATTRITION_MIN_CHEST: f32 = WAR_MIN_TREASURY * 2.0;
/// |score| reaching this ends the war outright — a decisive victory.
const WAR_SCORE_DECISIVE: f32 = 100.0;
/// Rounds that must elapse before EXHAUSTION (any of the three non-decisive paths)
/// may end a war — one calendar year. The old fixed-2-year mechanism enforced a
/// floor on how fast a war could ever conclude; nothing here replaced it once
/// resolution became per-round, so wars — mechanically capable of ending in their
/// very first round via war weariness — were doing exactly that. Measured:
/// `econ_fidelity_scorecard`'s "wars started / century" stayed near 50-65 through
/// three different attempts at the DECLARATION side (tightening `HOUSE_WAR_CHANCE`,
/// adding `WAR_MIN_TREASURY`, adding `war_cooldown_until`) — the volume was never
/// about how often a war STARTED, it was about how fast one FINISHED and freed a
/// slot for the next. A decisive score (±100, an actual curb-stomp) is exempt —
/// this floor only gates the three exhaustion paths, not a real victory.
const WAR_MIN_ROUNDS_TO_RESOLVE: u16 = 4;
/// A belligerent whose `hub.mood` (already fed by the war's own "war" active-event
/// hostility, §2's blockade note) falls this low sues for peace — WAR WEARINESS,
/// one of §1.4's four independent exhaustion paths. Reuses existing sentiment
/// machinery; no new field.
const WAR_MOOD_WEARY_FLOOR: f32 = 0.30;
/// TREASURY AND CREDIT SPENT: a side is exhausted once both its state treasury AND
/// its resident houses' aggregate positive wealth fall below this.
const WAR_FINANCIAL_EPS: f32 = 5.0;
/// FORCE BROKEN: a side whose levy+spend this year falls under this fraction of the
/// war's own best year for that side (`peak_effort_*`) has nothing left to field.
const WAR_FORCE_BROKEN_FRAC: f32 = 0.10;
/// BACKERS WITHDRAW (§3.4c house-driven wars only): the instigating house
/// (`War.backer_house`) itself going insolvent ends its own war — its backing was
/// the reason the war existed at all.
const WAR_BACKER_INSOLVENT: f32 = 0.0;
/// §3.4f's own precondition list ("reach satisfied, a real grievance, SUFFICIENT
/// TREASURY, and — for a house-driven war — council control") named a war
/// precondition the pre-3.4a candidate filter never actually checked. Without it,
/// a threadbare city could be declared into a war it exhausted out of within the
/// FIRST quarterly round — round-based resolution made that visible where the old
/// fixed 2-year timer hid it. Measured: `econ_fidelity_scorecard`'s "wars started /
/// century" read 65.0 with no floor at all, barely moved to 56.7 after tightening
/// `HOUSE_WAR_CHANCE` 8× (0.20 → 0.025) — proof the volume was never the
/// house-driven path, it was poor cities cycling through instant wars. This is the
/// condition that actually needed tightening, per §3.4f's own rule.
const WAR_MIN_TREASURY: f32 = 80.0;
/// §3.4f/§3.4a · years after a war ends before either belligerent has "a real
/// grievance" to fight again — see `TickHub.war_cooldown_until`'s own doc comment
/// for the measured before/after this fixed.
const WAR_COOLDOWN_YEARS: u32 = 5;
/// Geographic reach of a war, as a fraction of world width. Two cities can only go
/// to war if their seats sit within this cylindrical straight-line distance of each
/// other (on top of the existing same-`component` requirement). This is a
/// pre-modern, pre-colonial world: a city cannot march an army — nor sustain a
/// blockade — across an ocean or a continent, so a feud between two houses on
/// opposite sides of the map must NOT boil over into a state war between their
/// cities. Applies to BOTH the rival-council declaration path and the
/// house-driven (`declare_house_war`) escalation, which previously had no
/// geographic gate at all and was the main source of cross-continent wars.
const WAR_MAX_DIST_FRAC: f32 = 0.16;
/// How strongly a lopsided matchup ACCELERATES a war toward a decisive result.
/// The per-round score swing is multiplied by `1 + this·imbalance`, where
/// `imbalance` is 0 for an even match and 1 for a total mismatch. This is what
/// breaks the "every war lasts the full round cap" symptom: a genuine curb-stomp
/// (one side far richer) now reaches ±`WAR_SCORE_DECISIVE` in a handful of rounds
/// and ends early, while an even match still grinds to the cap. Duration therefore
/// VARIES with the matchup instead of being a uniform ~3-4 years, and the average
/// pace of an even war is left where the halved magnitudes put it (§3.4a).
const WAR_IMBALANCE_ESCALATION: f32 = 1.6;

// ── §3.4e · ledger, damage, blockade, the neutral boom ───────────────────────────
/// Yearly chance a belligerent's own estate/manufactory takes war damage. Lower
/// than the first cut (0.35): that value's extra `hash01` draws every war-year
/// reintroduced the same RNG-divergence sensitivity §3.4a-c's own tuning already
/// found in `econ_inheritance_rules_fragment_differently` (two 60-year
/// sub-simulations sharing a seed but diverging in house/estate count from the
/// first year, so any new per-year randomness in a shared code path shifts which
/// `hash01` draws each one consumes). 0.15 keeps war damage a real, recurring
/// cost without tipping that comparison over again.
const WAR_DAMAGE_CHANCE: f32 = 0.15;
/// A war's own damage roll — smaller than a natural disaster's 0.30–0.70 (a siege
/// nibbles, it doesn't level the works), but it recurs every year the war lasts.
const WAR_DAMAGE_MIN: f32 = 0.08;
const WAR_DAMAGE_MAX: f32 = 0.20;
/// A belligerent's `export_earn` — the term that actually drives `trade_wealth`
/// (see that field's own doc comment) — shrinks to this fraction each year at war.
/// The real, persistent blockade; the older `trade_wealth *= 0.8` line is cosmetic
/// only (`update_houses` overwrites `trade_wealth` from `export_earn`/
/// `import_spend` every day).
const WAR_BLOCKADE_EXPORT_MULT: f32 = 0.55;
/// The neutral WAR BOOM: a hub sharing a belligerent's trade component, itself at
/// peace, gets its own `export_earn` nudged — proportional plus a small flat floor
/// so even a currently-idle neutral hub sees some benefit from supplying the war.
const WAR_BOOM_EXPORT_FRAC: f32 = 0.12;
const WAR_BOOM_EXPORT_FLAT: f32 = 5.0;

// ── §3.4d · houses broken by war (the plan's own highest-risk item, deliberately
// last) — gated to a severe defeat (score ≥ WAR_PRICE_TRIBUTE) so it cannot fire on
// every marginal skirmish. ──
/// Per resident house at a sacked city, the chance it loses holdings there.
const WAR_SACK_CHANCE: f32 = 0.5;
const WAR_SACK_MAX_ESTATES: usize = 2;
/// The purge targets ONE specific house (guaranteed once triggered), so it can
/// afford to be a little more thorough than the scattershot sack.
const WAR_PURGE_MAX_ESTATES: usize = 3;
const WAR_PURGE_CONFISCATE_FRAC: f32 = 0.25;
const WAR_PURGE_POWER_LOSS: f32 = 0.15;

// ── §3.4b · terms priced in war score (§1.4's table) — extended by R4's §1.5 ────
const WAR_PRICE_REPARATIONS: f32 = 10.0;
const WAR_PRICE_HUMILIATE: f32 = 15.0;
const WAR_PRICE_TRADE_RIGHTS: f32 = 25.0;
const WAR_PRICE_ENTHRONE: f32 = 35.0;
const WAR_PRICE_TRIBUTE: f32 = 40.0;
const WAR_PRICE_VASSALIZE: f32 = 50.0;
const WAR_PRICE_PROVINCE: f32 = 55.0;
const WAR_PRICE_ANNEX: f32 = 90.0;
/// The war-score price of a goal — what the winner's final `|score|` must reach to
/// be ENTITLED to demand it. A win that falls short of its own declared goal's price
/// is downgraded to the richest goal the score actually affords (never upgraded —
/// overperforming does not let a trade dispute escalate into an annexation).
fn war_goal_price(goal: u8) -> f32 {
    match goal {
        WAR_GOAL_HUMILIATE => WAR_PRICE_HUMILIATE,
        WAR_GOAL_TRADE_RIGHTS => WAR_PRICE_TRADE_RIGHTS,
        WAR_GOAL_ENTHRONE => WAR_PRICE_ENTHRONE,
        WAR_GOAL_TRIBUTE => WAR_PRICE_TRIBUTE,
        WAR_GOAL_VASSALIZE => WAR_PRICE_VASSALIZE,
        WAR_GOAL_PROVINCE => WAR_PRICE_PROVINCE,
        WAR_GOAL_ANNEX => WAR_PRICE_ANNEX,
        _ => WAR_PRICE_REPARATIONS,
    }
}
// ── R4 · HUMILIATE / ENTHRONE / VASSALIZE tuning ────────────────────────────────
const HUMILIATE_LEGITIMACY_HIT: f32 = 0.10;
const HUMILIATE_LEGITIMACY_GAIN: f32 = 0.05;
const HUMILIATE_PRESTIGE_HIT: f32 = 0.10;
const HUMILIATE_PRESTIGE_GAIN: f32 = 0.05;
/// A puppet's reign — between `TRIBUTE_YEARS` (10) and a vassal's own term, so
/// Enthrone's middling price (35, between trade rights and tribute) is matched by
/// a middling duration. `reseat_official`'s own regular regime-change logic takes
/// over once it elapses — no special-case unwind needed, the SAME mechanism that
/// would naturally replace any official on schedule.
const ENTHRONE_TERM_YEARS: u32 = 15;
/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.10 (D12) · a pre-existing minority
/// share grandfathered into a lease at coronation runs this many years —
/// A1's own 5-9 year mezzadria/métayage citation, at its upper end (the
/// crown's own patience with an inherited arrangement runs long).
const LEASE_TERM_YEARS: u32 = 9;
/// §3.4c · a feud whose winner holds its city's council/captor seat may escalate
/// its worst (vendetta) flare into a full state war instead of the ordinary
/// property damage — "capturing a government is what lets a family spend a city's
/// blood on its own quarrel" (§5's Tiers note, the payoff of the whole leader design).
/// §3.4f measured a PRE-3.4a–e baseline of 6.0 wars/century (rival-council path
/// only). The first cut here used 0.20 and, compounded with `FEUD_FLARE_CHANCE`'s
/// own 0.28/month at the vendetta stage across every feud that ever reaches it,
/// measured 65.0 wars/century in `econ_fidelity_scorecard` — a war declared every
/// ~19 months, an order of magnitude past plausible. Tightened per §3.4f's own
/// rule ("tighten the conditions, do not add a rate limiter") rather than capping
/// war count directly.
const HOUSE_WAR_CHANCE: f32 = 0.025;

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
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.1/D3 · flat `ng × GRADE_BANDS`, NOT
    /// one float per good any more — index `g * GRADE_BANDS + band` (0 coarse ·
    /// 1 common · 2 fine). A pre-4.1 save's `stock` (length `ng`) is migrated into
    /// the common band once by `migrate_stock_bands` on load. Never index this
    /// directly; use `stock_of`/`stock_add`/`stock_take`/`stock_set_total` (mod.rs)
    /// so a reader always sees "today's single value" (F4) unless it's grade-aware.
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
    /// WORLD_AND_TRADE_MASTER_PLAN.md Part III §4 (transport modes, capacity half)
    /// — this hub sits on (or very near) a NAVIGABLE river. Seeded once at
    /// campaign start from the world's real river geometry (`sim::rivers::River`,
    /// same source `compute_route_days_matrix` now reads — CLAUDE.md rule 11:
    /// one source of truth); `#[serde(default)]` so an old save's hubs simply
    /// read false everywhere, the same as never having a river fleet advantage.
    #[serde(default)]
    pub river: bool,
    /// WORLD_AND_TRADE_MASTER_PLAN.md Part III §1 — a CITY is a knower too
    /// (forced by §0: cities found colonies, so they must know things). Same
    /// shape and seeding discipline as `House.known`.
    #[serde(default)] pub known: std::collections::HashMap<u32, Known>,
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
    /// DEPOSITS_AND_MINING_PLAN.md slice 4 (D7) · for a MINE (`estate_kind == 2`)
    /// only: the depth class of the real working nearest this estate's parent
    /// city (`DEPTH_SURFACE`/`_SHALLOW`/`_DEEP`/`_FLOODED`, `sim::deposits`),
    /// looked up once at founding from `CampaignSim::mine_deposits`. Depth is THE
    /// pre-modern mining constraint (Rio Tinto's reverse waterwheels, Agricola's
    /// *De Re Metallica* exist for drainage alone): it does not touch this
    /// estate's baseline output (already baked into `base_per_capita` by the
    /// world-side `workable_intensity() = grade × depth_workability`, §8.16 — an
    /// estate at a deep body already starts smaller), it scales how DEAR it is to
    /// upgrade (`MINE_UPGRADE_COST_MULT`). `DEPTH_SURFACE` (0) on every other
    /// estate kind, an old save, and a world with no positional deposit data —
    /// the safe, ungated default (rule 26's discipline applied to depth).
    #[serde(default)] pub mine_depth: u8,
    /// DEPOSITS_AND_MINING_PLAN.md slice 4/D3 · for a MINE or QUARRY only: the
    /// real body's EXTENT (`EXTENT_WEAK`/`_MODERATE`/`_GREAT`/`_WORLD_CLASS`,
    /// `sim::deposits`), looked up once at founding alongside `mine_depth`.
    /// `u8::MAX` = unknown (an old save, no positional data, or any other kind)
    /// — deliberately NOT `EXTENT_WEAK` (0), which would silently apply D3's
    /// decline to every pre-existing estate. Only a KNOWN weak body declines;
    /// unknown is treated exactly like moderate/great/world-class (persists).
    #[serde(default = "unknown_extent")] pub mine_extent: u8,
    /// DEPOSITS_AND_MINING_PLAN.md slice 5 · the Potosí case — a settlement whose
    /// existence IS the deposit (`maybe_found_mining_colony`). Set once at
    /// founding; read by the population pass (explosive growth) and `colony_pass`
    /// (decline, never death — D3's "persist" rule extended to the SETTLEMENT,
    /// not just the ore). `false` on every ordinary hub/estate/old save.
    #[serde(default)] pub is_mining_settlement: bool,
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
    /// A3 · yearly coin-biography snapshots (fineness/trust/value/price over time)
    /// for the Money panel sparklines. Capped at `COIN_HISTORY_CAP`. serde-defaulted.
    #[serde(default)] pub coin_history: Vec<CoinSnapshot>,
    // ── B3 · civic PUBLIC DEBT (the Monte / Casa di San Giorgio) ──
    /// Principal the city owes its bondholders (funded civic debt). A council raises
    /// it when the treasury runs short (war, public works) instead of failing; it is
    /// serviced by a yearly coupon and can be haircut in a fiscal crisis. 0 = no debt.
    #[serde(default)] pub debt_principal: f32,
    /// The sticky annual coupon rate the city pays its bondholders (the Monte paid ~5%).
    #[serde(default)] pub debt_coupon: f32,
    /// Who holds the debt: `(kind 0 = house · 1 = bank, index, amount lent)`. Coupons
    /// are paid pro-rata; a default haircuts every holder. Capped at `DEBT_HOLDER_CAP`.
    #[serde(default)] pub debt_holders: Vec<(u8, u32, f32)>,
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
    //    serde-defaulted so 0 = "finished / not a construction site"). ──
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
    /// CRISIS RELIEF (`polis.rs::decide_crisis_relief`) · while this exceeds the
    /// current tick the council has forbidden the EXPORT of food — the *tratta*
    /// prohibition every dearth-struck pre-modern city reached for. Doubles as the
    /// "relief is currently running here" flag, so one chronicle beat is written per
    /// EPISODE rather than one per month. 0 = no relief. Serde-default → a save from
    /// before this loads with every city unrestricted.
    #[serde(default)] pub food_export_lock: u32,
    /// N2 (`ACTORS_AND_CARRIAGE_PLAN.md` §3.2) — the general-purpose counterpart of
    /// `food_export_lock`, one slot per good: while `export_ban_until[g]` exceeds
    /// the current tick, this council forbids EXPORT of good `g` (an ordinary
    /// `Local`/`Global` good's stock has spiked far above its base value — the
    /// same "release the granary, bar the export" reflex food already gets,
    /// generalised). Precomputed once per dispatch and consulted in the seller
    /// loop, exactly the shape `food_export_lock` already proved. Resized to
    /// `goods.len()` on first use; empty on an old save (no bans, bit-identical).
    #[serde(default)] pub export_ban_until: Vec<u32>,
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
    /// CITY_PROVINCE_WAR_PLAN.md §3.2 · 1 great · 2 major · 3 lesser · 4 marginal ·
    /// 0 = not yet assigned (a brand-new settlement, or a world with no province
    /// layer — see `assign_city_tiers`'s own early return). Mirrors `House.tier`
    /// exactly, including the "0 = unassigned, never itself a real tier" convention.
    #[serde(default)] pub tier: u8,
    /// The percentile-ranked score behind `tier` — population+trade, treasury and
    /// fiscal reach, territory administered, and the ruling house's own standing.
    #[serde(default)] pub standing: f32,
    /// §3.4f/§3.4a · after a war ends (by ANY path), neither belligerent has a
    /// fresh grievance to fight again on until this tick — "a real grievance" from
    /// §3.4f's own precondition list. Without this, quick round-based resolution
    /// let the same two cities cycle through back-to-back wars unrealistically
    /// often (measured: 56.67 wars/century in `econ_fidelity_scorecard`, barely
    /// moved by tightening `HOUSE_WAR_CHANCE` or adding `WAR_MIN_TREASURY` alone —
    /// the churn was the same two war-eligible seats re-fighting, not new pairs).
    #[serde(default)] pub war_cooldown_until: u32,
    /// R1b · the tick THIS `captor_house` took hold (reset to `tick` every time
    /// `update_government` step 4 changes who holds the seat — including to −1). A
    /// proclamation requires ten years of CONTINUOUS capture, so a house that is
    /// bribed out and back in a single year cannot chain two half-tenures into one.
    #[serde(default)] pub captor_since: u32,
    /// R1b · sovereignty: the realm this city belongs to, or −1 (free — rule 25).
    #[serde(default = "neg_one_i32")] pub realm: i32,
    /// `REALM_ROLE_*` — this city's standing inside `realm`. Meaningless when
    /// `realm < 0`.
    #[serde(default)] pub realm_role: u8,
    /// N7 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §3.1) — the League this hub
    /// belongs to, or −1 (none). Authoritative, exactly as `realm` is: no
    /// second `members` list on `League` itself, per that struct's own doc.
    /// A league is NOT sovereignty (rule 27/§3.1) — this is independent of
    /// `realm`/`realm_role`, and a hub may hold both at once.
    #[serde(default = "neg_one_i32")] pub league: i32,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.2 (D17/F6) · the city's OWN
    /// warehouse capacity — population + Granary/Warehouse structures — kept
    /// separate from the per-house `Warehouse.capacity`. 0 = not yet sized (a
    /// fresh/old-save hub); `warehouse_and_spoilage_pass` (monthly) sizes it
    /// for every non-estate hub, so a 0 reads as "uncapped" for at most one
    /// month rather than forever.
    #[serde(default)] pub wh_capacity: f32,
    /// What rotted THIS MONTH, one entry per good (ng-length; empty ⇒ zeros)
    /// — the warehouse panel's own headline figure (§4.3/§8.1). Reset and
    /// refilled every monthly spoilage pass.
    #[serde(default)] pub wh_spoiled_month: Vec<f32>,
    /// Each good's TOTAL stock as of the last monthly pass — the baseline the
    /// warehouse panel's "▲+340" month-delta reads against (ng-length; empty
    /// ⇒ every delta reads 0, never a spurious first-month spike).
    #[serde(default)] pub wh_last_month: Vec<f32>,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.4 (D20) · who delivered each
    /// good, recently — flat `ng × SUPPLY_CLASSES`, decaying daily like
    /// `in_by_sea`/`in_by_land` already do. Tagged at the highest-volume
    /// delivery sites (own/estate production, incoming trade); a delivery
    /// this doesn't reach (a civic grant, a disaster relief shipment) simply
    /// isn't counted — the single pool and the price formula are unchanged
    /// either way (D20's own scope).
    #[serde(default)] pub supply_accum: Vec<f32>,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.5 (D1) · this works' ownership
    /// table — supersedes `stake_bank`/`stake_share` (F2) as the source of
    /// truth for dividend payout; those two fields are kept only as a cheap
    /// "does a bank have a finance interest here" marker (`development_tier`'s
    /// `finance` check) and are written in lockstep by every acquisition site.
    #[serde(default)] pub shares: Vec<Share>,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (§3) · this works' own
    /// twelve-month output/quality/price ring (its DOMINANT good only).
    /// Empty for a non-estate hub — the works card is an estate-only view.
    #[serde(default)] pub monthly: Vec<MonthSample>,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.13 (A3) · this works has already
    /// been chronicled for reaching GREAT or better — a rise is a milestone
    /// (same discipline as `golden_age_chronicled`/`dynasty_chronicled`), a
    /// later fall is not un-chronicled or re-announced.
    #[serde(default)] pub brand_chronicled: bool,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.7 (A8) · RESERVED, not currently
    /// read anywhere. A8 asks for harvest-failure clustering (a bad year
    /// raising next year's odds); a first cut wired this field into the
    /// disaster roll's chance and, independently, gave each disaster kind its
    /// own damage range and repair pace — each change, even alone, pushed
    /// `simulate_decades_reports_dynamics` into a sustained-runaway-rich
    /// house via this engine's own documented RNG-consumption-cascade
    /// sensitivity (see `estate_condition_pass`'s own doc comment). Reverted
    /// to keep the roll bit-identical to the pre-4.7 code; the field is left
    /// in place, unread, for a future session to re-attempt as its own
    /// isolated, separately-gated change.
    #[serde(default)] pub bad_years: u8,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.7a · RESERVED, not currently
    /// read — see `bad_years`' own doc comment for why a per-kind repair pace
    /// was reverted.
    #[serde(default)] pub disaster_repair_mult: f32,
    /// YARDS_VESSELS_AND_DEPOTS_PLAN.md S1 · a yard estate's (`estate_kind ==
    /// YARD_ESTATE_KIND`) accumulated hull-construction points, drawn monthly
    /// from its parent city's local material surplus. Resets to 0 once a hull
    /// completes (`HULL_BUILD_POINTS`). Meaningless on any other estate kind.
    #[serde(default)] pub yard_progress: f32,
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
    /// 4 grain law (civic granary) · 5 guild monopoly · 6 foreign-ownership bar
    /// (ESTATES_SHARES_AND_WAREHOUSE_PLAN.md A4 — read by `resolve_envoy`;
    /// enacted only at a fresh council capture, the "5) Payoff" step below).
    /// Kinds 1-5 are the pre-existing aspirational set: documented since before
    /// this slice, still enacted nowhere. Left as-is (not this slice's job).
    pub kind: u8,
    /// Beneficiary house (−1 none).
    pub house: i32,
    /// Relevant good (−1 none).
    pub good: i32,
}

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.9 (A4) · `Law.kind` for a foreign-
/// ownership bar. Kept as a top-level const (not just the doc-comment on
/// `Law::kind`) because `resolve_envoy` compares against it directly.
pub(crate) const LAW_FOREIGN_BAR: u8 = 6;

/// Serde default for `owner_house` so old saves / non-estate hubs read −1, not 0
/// (which would point at house index 0).
fn neg_one_i32() -> i32 { -1 }
fn unknown_extent() -> u8 { u8::MAX }

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

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 (§3) · one monthly sample of a
/// works' DOMINANT good — output/quality/price — behind the works card's
/// twelve-month curves. §3's own data model spells this as a fixed `[T; 12]`
/// ring + a cursor; here it's a `Vec` capped at 12 (push-and-trim from the
/// front) instead, for the same ring behaviour without a hand-rolled
/// Deserialize impl for a fixed-size array of a custom struct — the
/// serialized shape a reader cares about (≤12 chronological samples) is
/// identical either way.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MonthSample { pub output: f32, pub quality: f32, pub price: f32 }
pub const WORKS_MONTHLY_CAP: usize = 12;

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
    /// WORLD_AND_TRADE_MASTER_PLAN.md Part III §4 (transport modes, capacity
    /// half) — true when this OVERLAND leg is river-borne (both ends `TickHub.
    /// river`), so it occupies a river-BARGE slot rather than a caravan one.
    /// Meaningless when `sea` is true. `#[serde(default)]` — an old save's
    /// in-flight cargo simply reads false and falls to the caravan pool, same
    /// as before this field existed.
    #[serde(default)] pub river: bool,
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
    /// The price the cargo was actually STRUCK at when it left, grain-equivalent —
    /// `pa` for an outbound spot leg, `pb_buy` for a return leg, the contract price
    /// for a futures delivery. The same figure `RecentTrade.price` records for a
    /// completed deal.
    ///
    /// Purely for the market view. Before this, a settlement's in-flight rows were
    /// stamped with the VIEWING hub's own local price (`read_hubs.rs`'s `mk_row`),
    /// so an inbound cargo displayed as though it had been bought at the price of
    /// the city it was sailing towards — see `docs/TRADE_AND_MARKET_REVIEW.md`
    /// Part 3. Written here, read only by the query layer.
    #[serde(default)] pub price: f32,
    /// `ACTORS_AND_CARRIAGE_PLAN.md` N8 · true when this is an OWNERLESS leg
    /// (`owner < 0`) that also cleared `LOCAL_HAUL_DAYS` at dispatch — i.e. the
    /// carrier this arrival should book as at the destination is `SUPPLY_LOCAL`,
    /// not `SUPPLY_FOREIGN`. Meaningless when `owner >= 0` (books `SUPPLY_HOUSE`
    /// regardless). `#[serde(default)]` — an old save's in-flight cargo reads
    /// false and books `SUPPLY_FOREIGN`, exactly its pre-N8 behaviour.
    #[serde(default)] pub local: bool,
    /// TRADE_STAGING_AND_POSTS_PLAN.md §5 slice 4 (the keystone) — when this
    /// leg's route was composed through an entrepôt outlet (`CampaignSim::
    /// route_outlet`), `to` is that OUTLET, not the ultimate buyer, and `via`
    /// names the real destination. On arrival at the outlet the cargo is a
    /// real stop, not a teleport: `arrive_at_outlet` decides BREAK OF BULK
    /// (sell into the outlet's own market — real revenue, real throughput,
    /// the cargo may be bought onward by anyone, D2) versus continuing —
    /// spawning a fresh onward leg to `via` — by comparing the outlet's own
    /// price against the destination's net of the remaining freight.
    /// `-1` (the default) is a plain, un-transshipped leg, unchanged from
    /// before this field existed — an old save's in-flight cargo reads `-1`.
    #[serde(default = "neg_one_i32")] pub via: i32,
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
    /// True when this leg is a non-sea route between two river-connected hubs
    /// (`TickHub.river` at both ends) — a river-barge voyage, not a caravan
    /// one. Mutually exclusive with `sea`. Display-only: see the `cap_land`
    /// doc comment in `production.rs::dispatch` for why the fleet-capacity
    /// pool itself stays a plain sea/land split.
    #[serde(default)] pub river: bool,
    pub price: f32,
    pub tick: u32,
}

/// The running per-(hub, good, partner, direction) accumulator behind
/// `TradeFlowAgg`, carrying the transport split and the carrier breakdown the
/// fold used to throw away.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradeCur {
    pub amount: f32,
    pub sea_amount: f32,
    /// How much of `amount` moved by RIVER BARGE (a non-sea leg between two
    /// `TickHub.river` hubs) — the rest, `amount - sea_amount - river_amount`,
    /// went by caravan. See `RecentTrade.river`'s own doc comment.
    #[serde(default)] pub river_amount: f32,
    /// house index (u32::MAX = no named owner) → volume carried.
    pub carriers: std::collections::HashMap<u32, f32>,
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
    /// How much of `amount` moved BY SEA (both ends coastal). The rest went
    /// overland — by river barge (`river_amount`) or by caravan (the residual,
    /// `amount - sea_amount - river_amount`).
    ///
    /// `log_trade` has always been handed the shipment's `sea` flag, and the
    /// yearly fold discarded it — the same shape as every other number the plan
    /// notes is "computed inside `dispatch()` and then thrown away". Nothing but
    /// this display reads it, so it cannot move a simulated figure.
    #[serde(default)]
    pub sea_amount: f32,
    /// How much of `amount` moved BY RIVER BARGE — a non-sea leg between two
    /// `TickHub.river` hubs. `InTransit.river`/`RecentTrade.river` were being
    /// computed correctly for a shipment's return leg but hardcoded false for
    /// its outbound leg (a residual of reverting the fleet-CAPACITY split,
    /// which is a different question from whether the mode is worth
    /// reporting — see `production.rs::dispatch`'s doc comment), so a river
    /// city's trade always read as pure caravan overland however it actually
    /// moved. Fixed at the source; this field simply carries the real number
    /// through instead of discarding it a second time.
    #[serde(default)]
    pub river_amount: f32,
    /// Who carried it: house index → volume. `-1` is the residual (local
    /// merchants / no named owner) and is stored as `u32::MAX`.
    #[serde(default)]
    pub carriers: Vec<(u32, f32)>,
    /// Which calendar quarter this entry covers: 0..3 for `trade_last_season`'s
    /// real per-quarter breakdown, `SEASON_WHOLE_YEAR` (4) for `trade_last`'s
    /// existing annual entries — the sentinel keeps every pre-existing
    /// `TradeFlowAgg` construction site (and every old save's deserialized
    /// entry, via `#[serde(default)]`) honestly labelled "not seasonal data"
    /// rather than silently reading as quarter 0.
    #[serde(default = "season_whole_year")]
    pub season: u8,
}
fn season_whole_year() -> u8 { SEASON_WHOLE_YEAR }
/// Sentinel for `TradeFlowAgg.season` meaning "the whole year", not a real quarter.
pub const SEASON_WHOLE_YEAR: u8 = 4;

/// Per-(hub, good) yearly trade-volume series (in + out), so the Flows subtab can
/// graph trade DYNAMICS over the campaign and show which trades have fallen.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeHist {
    pub hub: u32,
    pub good: u32,
    /// Total volume traded each year (most recent last), capped to `TRADE_HIST_CAP`.
    pub vols: Vec<f32>,
    /// This hub's LOCAL PRICE for the good (grain-equivalent, the smoothed
    /// `hubs[h].price[g]`), sampled once at each New Year alongside `vols` — the
    /// only per-(hub, good) price series the project keeps. Before it, the sole
    /// price history anywhere was one world scalar (`sample_journal`) and a
    /// per-hub BASKET index (`HubSample.price_index`), so nothing could answer
    /// "what happened to the price of pepper here" — see
    /// `docs/TRADE_AND_MARKET_REVIEW.md` F9.
    ///
    /// Sparse BY CONSTRUCTION: a row exists only for a (hub, good) pair that
    /// actually traded, and shares `vols`' row cap and pruning, so this costs
    /// nothing on top of the series that was already kept.
    ///
    /// **Tail-aligned, not index-aligned.** A save written before this field
    /// loads with `prices` empty while `vols` already holds up to
    /// `TRADE_HIST_CAP` years, and both then grow and drain in lockstep — so
    /// the LAST entry of each is always the same year and readers must zip from
    /// the END. Nothing is back-filled: a fabricated price history would be
    /// worse than a short one.
    #[serde(default)]
    pub prices: Vec<f32>,
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

/// One head of a house, from accession to death — a single link in the succession
/// LINE. Written when a head takes over and closed when they die, so a house carries
/// its own chronicle of who held it, for how long, at what age, and how the family
/// fared under them.
///
/// This is a RECORD, not a model of a person: nothing in the tick reads it, and the
/// `epithet` is derived at death from what measurably happened during the tenure. The
/// kin roster (siblings, cousins, power shares — an actual family tree rather than a
/// line of heads) is Phase 2 of `docs/proposals/HOUSE_MASTER_PLAN.md`.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct HouseHead {
    pub name: String,
    /// A woman held the house — decided by the culture's LINE RULE (`sim::inheritance`).
    pub female: bool,
    pub generation: u32,
    /// Tick of accession, and of death (0 while this head still lives).
    pub since: u32,
    pub until: u32,
    /// Age in years at accession and at death (0 while living). An heir is NOT born on
    /// the day they inherit — they arrive at whatever age the inheritance rule implies
    /// (a youngest son young, an elected elder old), and their tenure is what remains
    /// of their life from there.
    pub age_at_accession: u32,
    pub age_at_death: u32,
    /// House wealth at accession, and at death.
    pub wealth_start: f32,
    pub wealth_end: f32,
    /// How they came to hold it: "founder" · "heir" · "co-heir" · "the hearth-keeper" ·
    /// "eldest capable" · "sister's son" · "daughter of the house".
    pub accession: String,
    /// A by-name earned at death, derived from the tenure itself ("the Great", "the
    /// Brief"). Empty for most heads — an epithet everyone has says nothing.
    #[serde(default)] pub epithet: String,
}

/// Phase 2.1 · one member of a house's KIN roster — the shared substrate for three
/// things `HOUSE_PEOPLE_AND_TIERS.md` §2 asks for at once: the head's character (§3,
/// `kin[0]` IS the head), holdings authorship (§4 — a `posted` kin vs a hired factor),
/// and, later, a schism's line of descent (§5, Phase 4/5, unbuilt). A house with an
/// EMPTY roster — every save from before this phase, and any house whose generation
/// skips it — behaves exactly as before: nothing in the tick reads `kin`.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Kin {
    pub name: String,
    pub female: bool,
    pub born_tick: u32,
    /// 0 while alive.
    #[serde(default)] pub dies_tick: u32,
    /// 0 head · 1 heir · 2 factor (runs a holding) · 3 idle · 4 married out · 5 dead.
    pub role: u8,
    /// Hub of the holding this person runs, −1 = at the seat / unposted. A SNAPSHOT
    /// taken when the roster was last (re)generated at founding/succession — not
    /// continuously kept in sync with which holdings the house currently has, the same
    /// way `wealth_history` is a periodic sample rather than a live mirror.
    pub posted: i32,
    /// Four culture-derived axes, −2..+2: caution↔boldness · honour↔greed ·
    /// private↔civic · rooted↔expansive (§3). DISPLAY ONLY here — a phrase on the
    /// dossier, nothing more. Wiring an axis to the real decision it names (Phase 2.4)
    /// is future work; the gate for that phase is that an all-zero character leaves
    /// the dynamics run bit-identical, which is trivially true while nothing reads it.
    pub character: [i8; 4],
    pub loyalty: f32,
    pub skill: f32,
    /// Index into the SAME house's `kin`, or −1 — the line of descent a schism would
    /// split along (Phase 4/5, unbuilt).
    #[serde(default = "neg_one_i32")] pub parent: i32,
}

/// A people's law of inheritance, resolved once from its language kit and kept for the
/// life of the campaign. See `sim::inheritance` for what the codes mean and
/// `docs/proposals/HOUSE_INHERITANCE_AND_TERRITORY.md` Part B for the assignment.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CultureRule {
    pub culture: String,
    /// `LineRule` code — who may inherit.
    pub line: u8,
    /// `InheritanceRule` code — how the estate divides.
    pub rule: u8,
}

/// Rank-normalise (percentile, 0..1, ties averaged) a slice of values, so a tier
/// score reads "where this house stands among its LIVE peers" rather than an absolute
/// number that means nothing as the world grows. Highest value → 1.0.
pub fn rank_norm(values: &[f32]) -> Vec<f32> {
    let n = values.len();
    if n <= 1 { return vec![0.5; n]; }
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| values[a].partial_cmp(&values[b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![0.0f32; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && values[idx[j + 1]] == values[idx[i]] { j += 1; }
        let avg_rank = (i + j) as f32 / 2.0;
        let r = avg_rank / (n - 1) as f32;
        for k in i..=j { out[idx[k]] = r; }
        i = j + 1;
    }
    out
}

/// Is this house-event kind a MILESTONE — part of the family's permanent record —
/// rather than chatter? Milestones survive the per-house chronicle cap; everything
/// else is pruned oldest-first. The test of a milestone is simple: would a historian
/// of this family still mention it in a century?
pub fn is_house_milestone(kind: &str) -> bool {
    matches!(kind,
        "founded" | "succession" | "inheritance" | "archetype" | "monopoly"
        | "control_gained" | "control_lost" | "branch" | "charter" | "bailo"
        | "bankruptcy" | "dissolved" | "bank" | "marriage" | "loss" | "tier_up"
        | "golden_age" | "dynasty" | "goal_achieved" | "deposed" | "crisis_survived"
        | "schism" | "plague_extinction" | "province_granted")
}

/// Phase 3.1 · a checkable ambition (`HOUSE_PEOPLE_AND_TIERS.md` §4). A goal must be
/// able to SUCCEED or FAIL and be recorded, or it is decoration — the whole reason
/// this is a struct with a `state` rather than a wish-list string.
///
/// `progress`'s meaning is PER-KIND (documented at each kind below), not a uniform
/// 0..1 — the kinds don't share a single honest notion of "how close", and forcing
/// one would either lie for some kinds or need a second field for others.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Goal {
    pub kind: u8,
    /// −1 when unused by this `kind`.
    pub target_good: i32,
    pub target_hub: i32,
    pub target_house: i32,
    pub target_province: i32,
    pub set_tick: u32,
    pub deadline_tick: u32,
    pub progress: f32,
    /// 0 pursuing · 1 achieved · 2 failed · 3 abandoned (the house holding it died).
    pub state: u8,
}

/// `Goal.kind` — see each kind's success test in `update_house_goal` (houses.rs).
/// Cut from the design's 17 to the 7 that reference systems already in this codebase
/// (`HOUSE_MASTER_PLAN.md` Part 3): every one of these reads state that already
/// exists, so none of them needed new sim machinery to become checkable.
pub const GOAL_CORNER_TRADE: u8 = 0;   // monopoly >= 60% share, held 5 years running
pub const GOAL_SEAT_COUNCIL: u8 = 1;   // captor_house/council_house == self, at the seat
pub const GOAL_RAISE_BAILO: u8 = 2;    // an owned office becomes a bailo
pub const GOAL_CHARTER_BANK: u8 = 3;   // owns a solvent bank, held 10 years running
pub const GOAL_REACH_PROVINCE: u8 = 4; // an OWN expedition completes its round trip there
pub const GOAL_OUTLAST_RIVAL: u8 = 5;  // a named rival goes defunct while this house lives
pub const GOAL_RESTORE_HOUSE: u8 = 6;  // wealth climbs back to the peak it held when set

pub const GOAL_PURSUING: u8 = 0;
pub const GOAL_ACHIEVED: u8 = 1;
pub const GOAL_FAILED: u8 = 2;
pub const GOAL_ABANDONED: u8 = 3;

/// A Tier 1 (great) house pursues two ambitions at once; everyone else, one — §4's
/// own rule, and a cheap extra reason Tier 1 reads as more than a bigger number.
pub const GOAL_SLOTS_TIER1: usize = 2;
pub const GOAL_SLOTS_OTHER: usize = 1;
/// Kept for the record even after leaving `goals` — capped so a centuries-old
/// dynasty's history doesn't grow without bound.
pub const GOAL_HISTORY_CAP: usize = 24;
/// Years of continuous qualification `GOAL_CORNER_TRADE`/`GOAL_CHARTER_BANK` need.
pub const GOAL_HOLD_YEARS_TRADE: f32 = 5.0;
pub const GOAL_HOLD_YEARS_BANK: f32 = 10.0;
/// Default deadline (years from `set_tick`) per kind, indexed by `Goal.kind`. Long
/// enough that a goal spans a meaningful slice of a head's tenure without being
/// unfalsifiable.
pub const GOAL_DEADLINE_YEARS: [f32; 7] = [25.0, 15.0, 15.0, 20.0, 20.0, 25.0, 20.0];

/// Phase 3.2 · a NAMED consequence of character extremes plus low skill
/// (`HOUSE_POWER_AND_POLITICS.md` §4) — derived from `kin[0]` alone, so there is no
/// third random layer beyond the character already rolled at Phase 2.3. `0` = none.
pub const VICE_NONE: u8 = 0;
pub const VICE_LAVISH: u8 = 1;      // civic ≥ +1 and skill ≤ 0.4 — bleeds consumption
pub const VICE_RECKLESS: u8 = 2;    // bold ≥ +2 — overreaches on ventures
pub const VICE_RAPACIOUS: u8 = 3;   // greed ≥ +2 — escalates feuds it cannot win
pub const VICE_MISERLY: u8 = 4;     // bold ≤ −2 and civic ≤ −1 — under-invests
pub const VICE_PAROCHIAL: u8 = 5;   // rooted ≤ −2 — refuses expansion
/// Lavish is the one vice this pass wires a direct wealth cost to (`apply_wealth_sinks`)
/// — one concrete consequence, the same "one touchpoint, not every listed knob"
/// discipline Phase 2.4 already used. The others feed `vice_severity` into crisis
/// discontent/rolls only; see the Phase 3.2 handoff note for what's NOT wired.
pub const VICE_LAVISH_DRAIN: f32 = 0.0015;

/// Display name for a vice code — CITY_PROVINCE_WAR_PLAN.md §3.1 is the first
/// caller that surfaces `head_vice` to the frontend at all (previously it only
/// fed the discontent/crisis roll internally), so this didn't need to exist until now.
pub fn vice_label(v: u8) -> &'static str {
    match v {
        VICE_LAVISH => "lavish",
        VICE_RECKLESS => "reckless",
        VICE_RAPACIOUS => "rapacious",
        VICE_MISERLY => "miserly",
        VICE_PAROCHIAL => "parochial",
        _ => "",
    }
}

/// Phase 3.3–3.6 · a succession crisis in progress. Opens when a house's discontent
/// crosses a threshold; runs a fixed number of quarterly rounds; resolves into
/// PREVAILED / DEPOSED / DISSOLVED. `HOUSE_SUCCESSION_CRISIS.md` +
/// `HOUSE_FACTION_NAMING_AND_RECORD.md`, scoped down — see the Phase 3 handoff note
/// in `docs/proposals/HOUSE_MASTER_PLAN.md` for exactly what was cut and why.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HouseCrisis {
    pub opened_tick: u32,
    /// 0 falling funds · 1 failed ambitions · 2 the head's vice · 3 a hostile kinsman.
    pub cause: u8,
    /// Kin index of the challenger, or −1 = leaderless discontent (easier to survive).
    pub plot_leader: i32,
    pub round: u8,
    pub head_support: f32,
    pub plot_support: f32,
    pub peak_plot: f32,
    pub rounds: Vec<CrisisRound>,
    /// Faction names + tints (`HOUSE_FACTION_NAMING_AND_RECORD.md` §1) — the
    /// loyalists default to the house's own heraldic tincture; the plot's is picked
    /// for contrast, so a player who learns the colour has learned the name.
    pub loyalist_name: String,
    pub loyalist_tint: String,
    pub plot_name: String,
    pub plot_tint: String,
    /// The heir's recorded choice (`HOUSE_POWER_STRUGGLE_VIEW.md` §3): 0 stood with
    /// the ruler · 1 turned to the plot · 2 no heir kin to choose at all.
    pub heir_choice: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrisisRound {
    pub tick: u32,
    /// 0 concede a holding · 1 buy off the plot · 2 launch a venture · 3 stand firm.
    pub action: u8,
    /// −1 backfired · 0 no effect · +1 worked.
    pub result: i8,
    pub head_delta: f32,
    pub text: String,
}

/// Phase 3.6 · the permanent, capped summary a crisis leaves behind once it closes —
/// same discipline as the family chronicle: the struggle's live detail (`HouseCrisis`)
/// is transient, but that it happened, over what, and how it ended is not.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CrisisRecord {
    pub opened_tick: u32,
    pub closed_tick: u32,
    pub cause: u8,
    pub loyalist_name: String,
    pub loyalist_tint: String,
    pub plot_name: String,
    pub plot_tint: String,
    pub rounds: u8,
    pub peak_plot: f32,
    /// 1 the ruler prevailed · 2 deposed · 3 the house dissolved in its own quarrel.
    pub outcome: u8,
    pub successor: String,
}

pub const CRISIS_PREVAILED: u8 = 1;
pub const CRISIS_DEPOSED: u8 = 2;
pub const CRISIS_DISSOLVED: u8 = 3;

/// One quarter (`HOUSE_SUCCESSION_CRISIS.md` §2's cadence) — long enough that a
/// round is a season, short enough that a ~1-year crisis reads as a run of them.
pub const CRISIS_ROUND_TICKS: u32 = 90;
/// Fixed at 4 rounds (~1 year) rather than the design's "3–5" — a fixed cap makes
/// `every_crisis_terminates` trivial to assert and costs the story little.
pub const CRISIS_ROUND_CAP: u8 = 4;
/// Above this, discontent opens a crisis (subject to the grace period below).
pub const CRISIS_DISCONTENT_THRESHOLD: f32 = 0.55;
/// `HOUSE_FACTION_NAMING_AND_RECORD.md` §4 — a head who survives is safe for 5 years.
pub const CRISIS_GRACE_YEARS: u32 = 5;
pub const CRISIS_HISTORY_CAP: usize = 8;
/// Phase 3.5 · civic intervention — chance the seat's council sequesters a slice of
/// wealth into its own treasury after a severe (plot_support ≥ 0.6 at some point)
/// deposition, amid the disorder. Kept small and rare on purpose (§3.5's own gate:
/// dynamics must stay bounded) — this is colour on an already-resolved outcome, not
/// a second wealth sink.
pub const CIVIC_INTERVENTION_CHANCE: f32 = 0.25;
pub const CIVIC_SEQUESTER_FRAC: f32 = 0.03;
/// Cost of the "buy off the plot" action, as a fraction of wealth, hard-capped in
/// absolute terms — `HOUSE_FACTION_NAMING_AND_RECORD.md` §2's own gate is that
/// courting spend must not be large enough to move the econ scorecard.
pub const CRISIS_BUYOFF_FRAC: f32 = 0.03;
pub const CRISIS_BUYOFF_CAP: f32 = 15.0;

/// Phase 4.1 · a house above this simplified `tension` reads (`crisis::house_tension`
/// — a stand-in for `HOUSE_PEOPLE_AND_TIERS.md` §5's own formula, see that function's
/// doc) quarrels or, more rarely, loses a posted kin to Departure.
pub const SCHISM_TENSION_THRESHOLD: f32 = 0.55;
/// Chance a qualifying, POSTED disloyal kin departs rather than merely quarrelling.
pub const DEPARTURE_CHANCE: f32 = 0.35;
pub const SCHISM_COOLDOWN_QUARREL_YEARS: u32 = 2;
pub const SCHISM_COOLDOWN_DEPARTURE_YEARS: u32 = 6;
/// The departing kin takes a smaller share than a wealthy cadet branch does
/// (`HOUSE_BRANCH_WEALTH`'s `found_branch`, 30%) — Departure is a rupture, not an
/// investment, so the parent isn't endowing it the way a deliberate expansion is.
pub const DEPARTURE_WEALTH_FRAC: f32 = 0.25;

/// Phase 4.3 · plague as a LINEAGE event (`HOUSE_MASTER_PLAN.md` 1.6), not just a
/// population headcount. Independent of head mortality, which stays governed
/// entirely by `head_lifespan`/succession — see `disease.rs::plague_house_toll`'s
/// own doc for why extinction is a separate roll rather than "did the head also die".
pub const PLAGUE_KIN_DEATH_CHANCE: f32 = 0.35;
pub const PLAGUE_EXTINCTION_CHANCE: f32 = 0.03;

/// Lineage · `House.origin_kind`, read only when `origin_house >= 0`. Which of the
/// game's house-creation paths produced this house — the Lineage tab's "why" for
/// each node besides the founding text already on `events[0]`.
pub const ORIGIN_NONE: u8 = 0;        // origin_house < 0: an original founding
pub const ORIGIN_GUILD: u8 = 1;       // seeded from a guild's own capital (maybe_found_house)
pub const ORIGIN_BRANCH: u8 = 2;      // a wealthy cadet branch (found_branch)
pub const ORIGIN_DIVISION: u8 = 3;    // a co-heir's share under Partible inheritance (divide_estate)
pub const ORIGIN_DEPARTURE: u8 = 4;   // a disloyal posted kin's schism (departure_schism)
pub const ORIGIN_INDEPENDENCE: u8 = 5; // a fresh dynasty seated when a colony wins independence

/// Phase 4.4 · the foreign hand (`HOUSE_POWER_STRUGGLE_VIEW.md` §2) — built ONLY
/// after `econ_measure_foreign_hand_conjunction` (§2.5's own "measure before
/// building" instruction) found the conjunction firing ~1229 times/century, far
/// above the "a handful a century" bar that would have left it as dead code.
/// Two channels, both concrete: A — a rival holds an office/bailo in our kin's
/// city; B — our kin's house leases in a city that rival CONTROLS. Leverage
/// DEEPENS an existing grievance (a small extra loyalty decay on the exposed kin)
/// — it never creates one outright, so a loyal, contented kin is never meaningfully
/// moved by it (see `apply_foreign_hand`'s own doc for the bound that guarantees this).
pub const FOREIGN_HAND_CHANNEL_A_WEIGHT: f32 = 0.5;
pub const FOREIGN_HAND_CHANNEL_B_WEIGHT: f32 = 0.8;
/// Monthly loyalty decay at leverage = 1.0 (the maximum, both channels + a feud).
/// Small and slow by construction — a single month's exposure should never itself
/// manufacture a plot; it accumulates only under SUSTAINED dependency.
pub const FOREIGN_HAND_DECAY_RATE: f32 = 0.01;
/// Monthly chance, AT leverage = 1.0, that active leverage is disclosed as a
/// chronicle line naming the rival. Scoped down from the design's "always
/// disclosed" (a literal always would need a new persistent per-kin annotation
/// field, another House-adjacent struct patch across every construction site) to
/// "eventually and occasionally visible" — cheap, and the effect itself (the
/// loyalty decay) is unconditional regardless of whether this roll fires.
pub const FOREIGN_HAND_DISCLOSE_CHANCE: f32 = 0.06;

/// WORLD_AND_TRADE_MASTER_PLAN.md Part III §1.1 — what one knower knows about
/// one province. Held sparsely (a `HashMap<province_id, Known>` on the knower,
/// not a dense per-province array) since most knowers know almost nothing
/// about most of the world; an absent entry means level 0 (unknown), which is
/// also what an old save's empty map means — no migration needed for THIS
/// field, only for the map being empty at all (§5, `seed_knowledge`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Known {
    /// 0 unknown · 1 reported (contact, or a partial report from a lost
    /// expedition) · 2 surveyed (an expedition returned — the founding gate) ·
    /// 3 established (we hold or trade there).
    pub level: u8,
    /// When this level was last reached. Map knowledge never uses this (a
    /// charted coast stays charted); it exists for the market-knowledge half
    /// `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` stage 4 owns, which is not
    /// built here — this field is what that stage will read.
    #[serde(default)] pub since_tick: u32,
    /// Who told us. -1 = our own expedition / presence.
    #[serde(default = "neg_one_i32")] pub source: i32,
}

/// WORLD_AND_TRADE_MASTER_PLAN.md Part III §1.2 — level 2, the founding gate.
pub(crate) const KNOWN_SURVEYED: u8 = 2;
/// §1.2 — level 1, contact or a partial report.
pub(crate) const KNOWN_REPORTED: u8 = 1;
/// §1.2 — level 3, established presence (holds or trades there).
pub(crate) const KNOWN_ESTABLISHED: u8 = 3;

/// A merchant family / trading house, with a named head of family who ages, dies
/// and is succeeded by an heir. Houses compete for trade, hold monopolies, feud
/// with rivals, and wield political power in their home city.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct House {
    pub name: String,  // "House Cassii"
    pub hub: u32,      // home hub index
    pub wealth: f32,
    pub prestige: f32,
    /// WORLD_AND_TRADE_MASTER_PLAN.md Part III §1 — this house's MAP knowledge,
    /// per province. Gates founding (`try_found_house_outpost` et al. need
    /// `>= KNOWN_SURVEYED` at the target site's province). Seeded once at
    /// campaign start / on first load of an older save (`seed_knowledge`) from
    /// the house's own holdings and trade partners, so day-one founding is
    /// unconstrained — the fog only ever bites into FUTURE expansion (§3.1).
    #[serde(default)] pub known: std::collections::HashMap<u32, Known>,
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
    /// Cumulative VOLUME shipped per good index — the goods this house moves the most,
    /// paired with `good_profit` for the dossier/compare "trade ledger" (amounts seen
    /// + most profitable). Credited beside `good_profit` at every trade site.
    #[serde(default)] pub good_volume: Vec<f32>,
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
    /// **CROWNED — left the merchant world, did NOT die** (`REALM_AND_GOVERNMENT_
    /// PLAN.md` §5.1). A house that proclaims a realm hands its wealth and trade
    /// assets to the crown and stops competing as a merchant family, but it is very
    /// much alive: it is the dynasty. This is a SEPARATE flag from `defunct` on
    /// purpose, and the distinction is not cosmetic —
    ///   * `dissolve_house` is a LIQUIDATION (writes off outstanding bank loans as
    ///     `Bank.losses`, releases held provinces, strips holdings, chronicles ruin),
    ///     so routing a coronation through it would have a family celebrate its
    ///     crowning by defaulting on its debts and losing its territory; and
    ///   * `GOAL_OUTLAST_RIVAL` closes ACHIEVED when a named rival goes `defunct`,
    ///     so crowning a house via that flag would hand every rival pursuing that
    ///     goal an instant win — a family becomes a king and its enemies celebrate
    ///     having outlived it.
    /// Merchant-world passes filter on `is_merchant()` below, never on `!defunct`
    /// alone; identity readers (arms, `line`, `origin_house` lineage, the chronicle)
    /// are deliberately unaffected, which is what keeps a dynasty's origins legible
    /// long after it stops trading.
    #[serde(default)] pub crowned: bool,
    /// The realm this house rules, or −1. 1:1 and permanent — one house, one realm.
    #[serde(default = "neg_one_i32")] pub realm: i32,
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
    // ── Succession (Phase 0.4 · the inheritance rule) ──
    /// The current head is a woman. Decided by the seat culture's LINE RULE.
    #[serde(default)] pub head_female: bool,
    /// The current head's age in YEARS at accession. `head_since + head_lifespan` is
    /// still the tick they die, but `head_lifespan` is now a TENURE (what is left of a
    /// life that began long before the accession), not a whole lifetime.
    #[serde(default)] pub head_age: u32,
    /// The succession line: every head this house has had, oldest first. Append-only,
    /// and read by nothing in the tick.
    #[serde(default)] pub line: Vec<HouseHead>,
    // ── Tiers (Phase 1.1) ──
    /// Rank band among LIVE private houses: 1 great · 2 major · 3 lesser · 4 marginal ·
    /// 0 not yet computed (no chronicle event fires off the sentinel). Never assigned to
    /// a guild — a guild is a civic office, not a family competing for standing.
    #[serde(default)] pub tier: u8,
    /// The 0..1 score `assign_house_tiers` bands into a tier — printed on the dossier so
    /// "why tier 2, not 1" is answerable, not asserted.
    #[serde(default)] pub standing: f32,
    // ── Positive events (Phase 1.4 · `HOUSE_PEOPLE_AND_TIERS.md` §2.2) ──
    // The mechanism otherwise only produces decline (vices, feuds, ruin) — these give
    // the chronicle something to say besides obituaries. All derived from state that
    // already exists; only a marker each, per the design's own "no new state beyond a
    // marker" rule.
    /// All-time peak wealth, and the tick it was reached — "the house's finest hour",
    /// kept forever. Never chronicled (a peak most months would spam the record); shown
    /// on the dossier as a fact instead.
    #[serde(default)] pub peak_wealth: f32,
    #[serde(default)] pub peak_wealth_tick: u32,
    /// Wealth as of the LAST monthly tier check — `assign_house_tiers`'s own tracker for
    /// "is this house still rising", kept separate from `prev_wealth` (which
    /// `recompute_monopolies_and_power` overwrites to the CURRENT figure before this
    /// runs, so it can't answer the same question).
    #[serde(default)] pub wealth_last_check: f32,
    /// Consecutive MONTHS held at Tier 1 with wealth still rising — "a golden age" fires
    /// once this reaches a decade (2.2). Resets the moment either condition breaks.
    #[serde(default)] pub golden_age_months: u32,
    #[serde(default)] pub golden_age_chronicled: bool,
    /// "A dynasty of merchants" — three consecutive heads in `line` who each grew the
    /// house — has already been chronicled, so it fires once per streak rather than at
    /// every qualifying succession after the third.
    #[serde(default)] pub dynasty_chronicled: bool,
    // ── Kin (Phase 2.1/2.2/2.3) ──
    /// The family roster. Empty for a guild (a civic office, not a family) and for any
    /// house whose roster generation was skipped — both read as "no kin", not an error.
    #[serde(default)] pub kin: Vec<Kin>,
    // ── Goals (Phase 3.1) ──
    /// Active ambitions — 1, or 2 for a Tier 1 house (`GOAL_SLOTS_TIER1`). A goal
    /// biases the WEIGHTS of decisions the house already makes; it never adds a new
    /// action. Never set for a guild — a civic office has no personal ambition.
    #[serde(default)] pub goals: Vec<Goal>,
    /// Every goal that ever left `goals` (achieved, failed, or abandoned), oldest
    /// first, capped at `GOAL_HISTORY_CAP`. The dossier's answer to "what has this
    /// family tried, and how did it go" — a family with three failed ambitions reads
    /// very differently from one with three achieved.
    #[serde(default)] pub goal_history: Vec<Goal>,
    // ── Crisis (Phase 3.2–3.6) ──
    /// The house's open succession crisis, if any — at most one at a time (a house
    /// mid-struggle cannot also open a second). `None` for a guild (a civic office
    /// has no throne to contest) and for the overwhelming majority of ticks, since a
    /// crisis is a rare, discontent-triggered event, not a running gauge.
    #[serde(default)] pub crisis: Option<HouseCrisis>,
    /// A head who survives a crisis earns a grace period: no new crisis may open
    /// before this tick (`HOUSE_FACTION_NAMING_AND_RECORD.md` §4) — without it a weak
    /// head sits in permanent crisis and the mechanic stops meaning anything.
    #[serde(default)] pub crisis_immune_until: u32,
    /// Permanent, capped record of past crises (Phase 3.6) — the same discipline as
    /// `goal_history`: kept forever in spirit, truncated in practice so a centuries-old
    /// dynasty's record doesn't grow without bound.
    #[serde(default)] pub crisis_history: Vec<CrisisRecord>,
    // ── Schism (Phase 4.1) ──
    /// No new Quarrel/Departure may fire before this tick — without it a house that
    /// just quarrelled would qualify again the very next month (the quarrel itself
    /// lowers the disloyal kin's loyalty, which otherwise feeds tension right back
    /// up), the same "must always terminate" lesson the crisis engine already needed.
    #[serde(default)] pub schism_cooldown_until: u32,
    // ── Lineage — where a house came from (−1 = an original founding) ──
    /// The house this one was struck FROM, or −1. Set once at creation, never
    /// changed — a house's origin is a fact about its founding, not something later
    /// events (succession, crisis, its own schisms) can revise.
    #[serde(default = "neg_one_i32")] pub origin_house: i32,
    /// `ORIGIN_*` — which of the game's house-creation paths produced this one.
    /// Meaningless when `origin_house < 0`.
    #[serde(default)] pub origin_kind: u8,
}

impl House {
    /// **The merchant-world predicate.** A house that still competes as a trading
    /// family: alive AND not crowned. Every pass that treats a house as a merchant
    /// — pricing, dispatch, fleets, offices, feuds, tiers, goals, banks, the wealth
    /// tax — must filter on this rather than on `!defunct` alone, or a crowned house
    /// keeps trading with a treasury it no longer owns
    /// (`REALM_AND_GOVERNMENT_PLAN.md` §3.2).
    ///
    /// Readers of a house's IDENTITY (name, arms, `line`, `origin_house` lineage,
    /// the chronicle) must NOT use this: a dynasty's record has to stay legible for
    /// as long as the world remembers it, which is the whole reason `crowned` is not
    /// `defunct`.
    #[inline]
    pub fn is_merchant(&self) -> bool { !self.defunct && !self.crowned }
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

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.5 (D1/§3) · one row of a works'
/// ownership table (`TickHub.shares`) — supersedes the old single-holder
/// `stake_bank`/`stake_share` pair (F2), generalized from "a bank may hold a
/// manufactory stake" to "anyone may hold a fraction of any works". An empty
/// `shares` Vec means 100% to whoever `owner_house` names (today's behaviour,
/// unchanged) — a row is only ever pushed when a SECOND party buys in.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Share {
    /// 0 city · 1 house · 2 guild · 3 bank · 4 realm.
    pub holder_kind: u8,
    /// Index into the matching collection (houses/banks/realms) for kinds
    /// 1-4; meaningless (0) for kind 0 (the parent city holds no separate index).
    pub holder: u32,
    /// Fraction of the works' owner-cut this row collects, 0..1. The table's
    /// own rows need not sum to 1.0 — whatever fraction is UNCLAIMED still
    /// belongs to `owner_house` (or the city), exactly as before any row existed.
    pub frac: f32,
    /// 0 offtake (extraction works, D1/D5 — waits for §4.8) · 1 dividend
    /// (manufactory works, D1 — live from this slice).
    pub payout: u8,
    pub acquired_tick: u32,
    /// What this fraction last traded at — the share-price anchor for a
    /// future valuation line (§6: no live exchange is built on top of this).
    pub paid: f32,
    /// A1 amendment · 0 = perpetual SHARE (mine/quarry/fishery/manufactory —
    /// freely divisible, bought and sold) · 1 = fixed-term TENANCY (farm/
    /// vineyard/plantation/pasture — GRANTED for `term_years` and must be
    /// RENEWED; a term expiring is a real political event a perpetual share
    /// never generates). Renewal itself is not yet built (flagged, not built —
    /// this slice only records the instrument and its term).
    #[serde(default)] pub instrument: u8,
    /// Only meaningful for a TENANCY (`instrument == 1`): the granted term in
    /// years, 5-9 per A1's own mezzadria/métayage citation.
    #[serde(default)] pub term_years: u32,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.7 (A9) · consecutive years this
    /// row has refused to fund its share of a repair. A TENANCY (never a
    /// perpetual SHARE — A9 scopes voiding to the term-limited instrument)
    /// that refuses `TENANCY_NEGLECT_LIMIT` years running is voided outright,
    /// not merely diluted.
    #[serde(default)] pub neglect_years: u32,
}

/// A1 · which works kinds carry a perpetual SHARE vs a fixed-term TENANCY.
/// Mirrors `estate_kind`'s own numbering (1 farm · 2 mine · 3 plantation ·
/// 4 fishery · 5 vineyard · 6 manufactory).
// Not yet called: nothing grants a fresh share/tenancy row until the envoy
// acquisition mechanism (§4.9) exists to call it. Kept typed and correct now
// rather than written when 4.9 needs it, per this slice's own scope (D1/A1).
#[allow(dead_code)]
pub(crate) fn share_instrument_for_kind(kind: u8) -> u8 {
    match kind {
        2 | 4 | 6 => 0, // mine · fishery · manufactory ⇒ SHARE
        _ => 1,          // farm · plantation · vineyard (and anything else) ⇒ TENANCY
    }
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
    /// B4 · cumulative BILLS-OF-EXCHANGE (FX-spread) income — earned settling trade
    /// across the bank's branch cities when they use DIFFERENT coins. serde-defaulted.
    #[serde(default)] pub bills_income: f32,
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

/// A3 · a yearly snapshot of a COIN's monetary state — drives the coin-biography
/// sparklines in the Money panel (fineness / trust / value / price level over time,
/// annotated with the year's monetary event).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoinSnapshot {
    pub year: u32,
    pub fineness: f32,
    pub trust: f32,
    pub value: f32,        // coin_value agio index
    pub exchange: f32,     // v2.1 metal-aware intrinsic exchange value (silver = 1)
    pub strength: f32,     // 0..100 headline
    pub price_level: f32,  // local CPI at the mint
    pub circulating: f32,  // Σ holders' throughput × basket share
    pub metal: u8,
    /// The notable monetary event that year at this mint: "" | "charter" | "first" |
    /// "debasement" | "reform" | "crash". Placed as a marker on the timeline.
    pub event: String,
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
    /// §3.4e · forced levies raised from each side's own resident houses, split out
    /// so the panel can show what each belligerent has spent its families into.
    #[serde(default)] pub levies_a: f32,
    #[serde(default)] pub levies_b: f32,
    /// §3.4a · the quarterly ROUNDS as a battle log — each round that shifted the
    /// war score, so the panel reads as a campaign history rather than one number.
    #[serde(default)] pub battles: Vec<WarBattle>,
    pub cargo_lost: u32,
    pub cause: String,
    /// What the war is FOR — decides what the victor takes at resolution (beyond the
    /// one-off plunder): 0 plunder · 1 tribute · 2 trade rights · 3 annexation ·
    /// 4 a province (§3.4b). `#[serde(default)]` → old saves load as plunder.
    #[serde(default)] pub goal: u8,
    /// §3.4a · bidirectional war score, −100..100. Positive favours `a`. Fed by
    /// quarterly round outcomes; ±100 ends the war outright.
    #[serde(default)] pub score: f32,
    /// §3.4a · quarterly rounds elapsed — `WAR_ROUND_CAP` is the last-resort backstop.
    #[serde(default)] pub round: u16,
    /// §3.4a · the best single YEAR of combined levy+spend either side has managed
    /// so far — the "force broken" exhaustion path reads a side's CURRENT year
    /// against its own past peak, not an absolute threshold, so it scales with
    /// however rich the belligerents are.
    #[serde(default)] pub peak_effort_a: f32,
    #[serde(default)] pub peak_effort_b: f32,
    /// §3.4c · house-driven war: the house whose feud escalated into this war,
    /// automatically committed as a backer. −1 for an ordinary rival-council war.
    /// Its own insolvency is the BACKERS WITHDRAW exhaustion path.
    #[serde(default = "neg_one_i32")] pub backer_house: i32,
}

/// §3.4a · one quarterly round of a war — a "battle" for the panel's history.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WarBattle {
    pub round: u16,
    pub year: u32,
    /// Which side the round favoured: 0 = a (attacker), 1 = b (defender).
    pub favored: u8,
    /// Signed score swing this round (positive favours a).
    pub delta: f32,
    /// War score after this round (−100..100).
    pub score_after: f32,
    /// True when this round was an OCCUPATION-scale blow (the larger swing).
    pub decisive: bool,
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
    /// The world province this site falls in (a raster lookup at generation time),
    /// or -1 if the world has no province layer yet (serde default — old saves and
    /// worlds without a province layer read as "unknown", never "empty", so they
    /// can't accidentally win the empty-province founding bonus below).
    #[serde(default = "neg_one_i32")] pub province: i32,
    /// CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 6 (F6) — the site's OWN
    /// per-good belt strength (0..1, same index space as `self.goods`), read at the
    /// site's coarse cell. `trade_value` above is one VALUE-weighted scalar
    /// ("how rich is this site"); this is the physical PRODUCT MIX ("rich in
    /// what") — the information `create_market_colony` needs to seed a colony as
    /// an extraction economy rather than a 60%-scaled photocopy of its founder.
    /// Empty on a save from before this slice (serde default), which is a true
    /// no-op: every reader falls back to the old founder-basket behaviour.
    #[serde(default)] pub belt: Vec<f32>,
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

/// Cultures 2.0 · a trade region's LINGUA FRANCA — the trade tongue that the
/// region's dominant urban culture spreads. Eases cross-family assimilation and
/// lingers as a legacy tongue after that culture fades.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinguaFranca {
    pub component: u32,     // the trade region this tongue serves
    pub family: String,     // the language family that is the trade tongue
    pub culture: String,    // the culture whose tongue it is (its origin people)
    pub share: f32,         // the dominant culture's share of the region's cities (0..1)
    pub since_year: u32,    // when this tongue became the region's lingua franca
    pub legacy: bool,       // true once its origin culture no longer dominates (a relic tongue)
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
    /// DLC 3.5 · progressive civic wealth tax paid to the home city (war levies are
    /// their own line, `war_levy`, below — CITY_PROVINCE_WAR_PLAN.md §3.4e wants a
    /// war's cost legible on its own, not folded into ordinary taxation).
    #[serde(default)] pub civic_tax: f32,
    /// §3.4e · forced war levy paid this year (`raise_war_levy`) — split out of
    /// `civic_tax` so "what did the war cost me" reads as its own line.
    #[serde(default)] pub war_levy: f32,
    /// §3.4e · wealth-equivalent loss when war damages one of THIS house's own
    /// estates/manufactories (the existing `TickHub.damage` field, no new field
    /// needed — see `war_damage_pass`).
    #[serde(default)] pub war_damage: f32,
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

// ── ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.1 (D3/F4) · grade bands ─────────────
// Three grade bands per good — coarse · common · fine — replacing the single
// float `hubs[h].stock[g]` used to be. `TickHub.stock` is now flat
// `ng × GRADE_BANDS`; every reader/writer goes through these helpers so the
// band layout lives in exactly one place. `Warehouse.stock` (a house/guild
// depot) is DELIBERATELY untouched by this slice — F4's own citation is
// `hubs[h].stock[g]`, and the city-level warehouse this plan adds (§4.2/D17,
// `wh_capacity`/`wh_spoiled_month`) is a new field on `TickHub`, not a rework
// of the existing per-house `Warehouse` struct.
pub const GRADE_BANDS: usize = 3;
pub const GRADE_COARSE: usize = 0;
pub const GRADE_COMMON: usize = 1;
pub const GRADE_FINE: usize = 2;

/// Sum a good's three grade bands — "today's single value" every pre-4.1 reader
/// wants (D3's own "a summing accessor keeps every existing reader working").
/// Defensive against a short/unmigrated vector (reads 0 rather than panicking).
#[inline]
pub(crate) fn stock_of(stock: &[f32], g: usize) -> f32 {
    let base = g * GRADE_BANDS;
    (0..GRADE_BANDS).map(|b| stock.get(base + b).copied().unwrap_or(0.0)).sum()
}

/// Add `amt` to good `g`'s specific band.
#[inline]
pub(crate) fn stock_add(stock: &mut [f32], g: usize, band: usize, amt: f32) {
    let idx = g * GRADE_BANDS + band.min(GRADE_BANDS - 1);
    if let Some(v) = stock.get_mut(idx) { *v += amt; }
}

/// Add `amt` to good `g` with no known grade — a delivery/transfer/civic release
/// that hasn't been graded yet (supplier attribution is §4.4, later). Lands in
/// the COMMON band, F4's own "indistinguishable from 600 mediocre" default.
#[inline]
pub(crate) fn stock_add_ungraded(stock: &mut [f32], g: usize, amt: f32) {
    stock_add(stock, g, GRADE_COMMON, amt);
}

/// Draw up to `amt` of good `g` off the CHEAPEST bands first (coarse → common →
/// fine) — the general market/consumption pool spends its worst stock first, the
/// same "fine drains last" reading the warehouse panel's grade strip gives the
/// *annona* year (§8.1). Returns the amount actually taken (capped by
/// availability, never more than was there).
#[inline]
pub(crate) fn stock_take(stock: &mut [f32], g: usize, amt: f32) -> f32 {
    let base = g * GRADE_BANDS;
    let mut remaining = amt.max(0.0);
    let mut taken = 0.0f32;
    for b in 0..GRADE_BANDS {
        if remaining <= 0.0 { break; }
        if let Some(v) = stock.get_mut(base + b) {
            let take = v.min(remaining);
            *v -= take;
            taken += take;
            remaining -= take;
        }
    }
    taken
}

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.8 (D5) · draw up to `amt` of good `g`
/// off the FINEST bands first (fine → common → coarse) — the mirror image of
/// `stock_take`'s cheapest-first draw, for the one reader that wants the
/// opposite order: an offtake holder taking "the top of output" (D5) draws the
/// best grade available before falling back to a coarser one, exactly as a
/// controlling shareholder historically took first pressing. Returns the
/// amount actually taken.
#[inline]
pub(crate) fn stock_take_finest_first(stock: &mut [f32], g: usize, amt: f32) -> f32 {
    let base = g * GRADE_BANDS;
    let mut remaining = amt.max(0.0);
    let mut taken = 0.0f32;
    for b in (0..GRADE_BANDS).rev() {
        if remaining <= 0.0 { break; }
        if let Some(v) = stock.get_mut(base + b) {
            let take = v.min(remaining);
            *v -= take;
            taken += take;
            remaining -= take;
        }
    }
    taken
}

/// Multiply every band of good `g` by `mult` in place (a proportional loss/gain
/// applied to the whole stock, e.g. rescaling a founding world's food supply).
#[inline]
pub(crate) fn stock_scale(stock: &mut [f32], g: usize, mult: f32) {
    let base = g * GRADE_BANDS;
    for b in 0..GRADE_BANDS {
        if let Some(v) = stock.get_mut(base + b) { *v *= mult; }
    }
}

/// Set good `g`'s TOTAL to `total`, replacing whatever band mix it had — used by
/// the few places that reseed a hub's whole stock from a formula (campaign
/// start, cold start, a founding grant). Lands the new total in the COMMON
/// band; the other two are zeroed, matching `stock_add_ungraded`'s convention.
#[inline]
pub(crate) fn stock_set_total(stock: &mut [f32], g: usize, total: f32) {
    let base = g * GRADE_BANDS;
    if base + GRADE_BANDS > stock.len() { return; }
    stock[base] = 0.0;
    stock[base + 1] = total.max(0.0);
    stock[base + 2] = 0.0;
}

/// Which band a hub's OWN production lands in. An ESTATE (a real works) grades
/// by its own per-good `quality` — D5's "the largest holder skims the finer
/// part" needs a real quality spread to skim from. An ordinary settlement's
/// bulk per-capita production is a VILLAGE in the plan's own sense (D14, "band
/// 0-1 only") and is capped to coarse/common regardless of quality — ownership,
/// grade and contest live only in works.
#[inline]
pub(crate) fn production_band(is_estate: bool, quality: f32) -> usize {
    let band = if quality >= 0.7 { GRADE_FINE } else if quality >= 0.4 { GRADE_COMMON } else { GRADE_COARSE };
    if is_estate { band } else { band.min(GRADE_COMMON) }
}

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.11 (F8) · a settlement's population
/// status — content · short · starving — read straight off the SAME two
/// fields the existing civic-granary release (mod.rs's own "6) Civic
/// granary" step) and the disease pass's starvation accumulator already
/// compute (`food_balance`/`starving`, `disease.rs`). A pure derived read, no
/// new state: `starving` already crosses 0.5 at exactly the granary's own
/// famine-release threshold, so this reuses that line rather than inventing
/// a second one. `POP_STATUS_CONTENT`/`_SHORT`/`_STARVING` name the result.
pub(crate) fn population_status(food_balance: f32, starving: f32) -> u8 {
    if starving > 0.5 { POP_STATUS_STARVING }
    else if food_balance < 0.0 { POP_STATUS_SHORT }
    else { POP_STATUS_CONTENT }
}
pub const POP_STATUS_CONTENT: u8 = 0;
pub const POP_STATUS_SHORT: u8 = 1;
pub const POP_STATUS_STARVING: u8 = 2;
/// Display label — the dossier's own "pips and a phrase, never a raw 0..1"
/// convention (`CLAUDE.md` §5, House Dossier). Reserved: `HubBrief.pop_status`
/// currently ships the raw code and lets the frontend format it; this stays
/// as the Rust-side vocabulary for a future chronicle line ("Genoa goes
/// short") without duplicating the wording twice.
#[allow(dead_code)]
pub(crate) fn population_status_label(status: u8) -> &'static str {
    match status {
        POP_STATUS_STARVING => "starving",
        POP_STATUS_SHORT => "short",
        _ => "content",
    }
}

// ── ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.4 (D20) · supplier attribution ──
// Five seller classes; the panel groups on these, never a named per-house
// ledger (D20 rejects "separate books per seller and a full order book").
pub const SUPPLY_CITY: usize = 0;
pub const SUPPLY_HOUSE: usize = 1;
pub const SUPPLY_GUILD: usize = 2;
pub const SUPPLY_LOCAL: usize = 3;
pub const SUPPLY_FOREIGN: usize = 4;
pub const SUPPLY_CLASSES: usize = 5;

/// Tag `amt` of good `g` as delivered by seller `class`. Accumulates like
/// `good_flow_accum` already does; decayed daily in `advance()`.
#[inline]
pub(crate) fn supply_add(acc: &mut [f32], g: usize, class: usize, amt: f32) {
    if amt <= 0.0 { return; }
    let idx = g * SUPPLY_CLASSES + class.min(SUPPLY_CLASSES - 1);
    if let Some(v) = acc.get_mut(idx) { *v += amt; }
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

/// YARDS_VESSELS_AND_DEPOTS_PLAN.md S2 — the vessel becomes a thing, instead of
/// a bare counter (`House.fleet_sea`/`_river`). Seeded once at campaign start
/// (one `Vessel` per pre-existing hull, wholly owned by its house — the
/// bit-identical migration `seeding_one_whole_hull_per_counter_is_bit_
/// identical` checks) and grown from then on by the yard (S1). Deliberately
/// NOT yet read by `dispatch`/`decide_fleets`/war spoils/the crisis venture —
/// per S1's own "output is a hull-ready event only; nothing consumes it yet"
/// and S4's dose-walk, `fleet_sea`/`_river` stay the single source of truth
/// for capacity until `CAPACITY_BIND_DOSE` is raised above zero. Caravans are
/// deliberately excluded (D4 — overland capacity is HIRED, not built; the
/// existing ownerless residual already models "hired carriage that happens to
/// be free").
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Vessel {
    pub id: u32,
    pub name: String,
    /// 0 = sea, 1 = river.
    pub kind: u8,
    /// Home hub — where the hull was built / is registered.
    pub home_hub: u32,
    /// Current location. Never moved yet (S2 is bookkeeping only); kept
    /// distinct from `home_hub` so W4's future depot-to-depot transfer has
    /// somewhere to write a real position without a second migration.
    pub at_hub: u32,
    pub capacity: f32,
    /// 0..1, drawn from the yard's own material mix quality where it was
    /// built (1.0 for a seeded legacy hull — "as good as it ever needed to
    /// be" is the honest reading of a hull this system never watched built).
    #[serde(default = "one_f32")] pub quality: f32,
    /// 0..1 seaworthiness; damage/decay would drain it (unused until a future
    /// slice wires wear into vessels instead of the bare fleet counters).
    #[serde(default = "one_f32")] pub condition: f32,
    /// D3 — fractional ownership. `(house_index, parts)`, `parts` summing to
    /// `VESSEL_PARTS_TOTAL` across the whole vessel (`vessel_parts_always_
    /// sum_to_64`). A seeded legacy hull is wholly owned by the one house
    /// whose counter it came from.
    pub parts: Vec<VesselShare>,
    pub built_tick: u32,
}

/// One house's stake in a `Vessel` (D3's *carati*/*paerten*).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VesselShare {
    pub house: u32,
    /// Out of `VESSEL_PARTS_TOTAL`.
    pub parts: u8,
}

/// YARDS_VESSELS_AND_DEPOTS_PLAN.md W5 — the *fondaco*: state-owned,
/// foreigner-occupied, compulsory (Venice's Fondaco dei Tedeschi; the
/// Islamic *funduq*/*khan*; the Hanseatic Kontor). What makes an office or a
/// bailo a BUILDING the host city can close, rather than a flag. Structural
/// only — `maybe_found_fondaco` never runs (W5 ships "zero dose first", per
/// the plan's own §3 risk note that `N2` broke the hard wealth bound twice on
/// a market-closure mechanism of this exact shape) — so a world with no
/// fondaco founded is bit-identical to one that never heard of this struct.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Fondaco {
    /// The host city.
    pub hub: u32,
    /// The foreign house/guild compelled to lodge & trade here.
    pub occupant: u32,
    /// The host city's cut of the occupant's trade through this compound, 0..1.
    pub cut: f32,
    pub founded_tick: u32,
    /// The city can close the door — a real, if rare, political act (unused
    /// until W5 is dosed above zero).
    #[serde(default)] pub closed: bool,
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
    /// Accumulated GRIEVANCE (in riot-years): rises while unrest simmers at riot
    /// level, bleeds off in calm years. A city that riots year after year without
    /// relief eventually boils over into a revolt even absent one acute spike —
    /// chronic misery, not just a sudden shock. Reset by the catharsis of a revolt.
    #[serde(default)] pub grievance: f32,
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
pub(crate) const EARTH_EQUATOR_KM: f32 = 40075.0;        // world_w cells span this at the equator

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

// ── FEUDS ────────────────────────────────────────────────────────────────────────
// A feud used to be a flat `rivals` list plus a 15%-per-half-year coin flip where the
// weaker house paid 8% of its wealth. That is a tax, not a quarrel: it had no cause, no
// memory, no escalation and no way to end except one side dying. The elaborated model
// keeps `rivals` in sync (every existing reader is untouched) and adds the four things
// a feud between merchant families actually has — a CAUSE, an INTENSITY that grows and
// cools, STAGES whose weapons differ, and a SETTLEMENT.

/// Why two houses fell out. Index into `FEUD_CAUSES`.
pub const FEUD_TRADE: u8 = 0;      // both live off the same good in the same market
pub const FEUD_SEAT: u8 = 1;       // both court the same city's council
pub const FEUD_MARRIAGE: u8 = 2;   // a match soured
pub const FEUD_MARKET: u8 = 3;     // one barred the other from a market
pub const FEUD_SUCCESSION: u8 = 4; // a disputed inheritance / a poached branch
pub const FEUD_CAUSES: [&str; 5] = [
    "the same trade", "a contested council", "a broken match",
    "a closed market", "a disputed inheritance",
];

/// Feud stages, in order. Each stage licenses a heavier weapon (see `feud_flare`).
pub const FEUD_COLD: u8 = 0;     // mutual dislike; no action beyond lost goodwill
pub const FEUD_OPEN: u8 = 1;     // undercutting — margin bleeds on the shared goods
pub const FEUD_TRADEWAR: u8 = 2; // market closure + influence stripped in shared cities
pub const FEUD_VENDETTA: u8 = 3; // sabotage: ships lost, offices burned
pub const FEUD_STAGES: [&str; 4] = ["cold rivalry", "open feud", "trade war", "vendetta"];

/// How a feud ended. 0 = still running.
pub const FEUD_RUNNING: u8 = 0;
pub const FEUD_ARBITRATED: u8 = 1; // a council both houses trade in imposed a settlement
pub const FEUD_WED: u8 = 2;        // sealed by marriage
pub const FEUD_RUINED: u8 = 3;     // one side went defunct
pub const FEUD_COOLED: u8 = 4;     // contact lapsed; the quarrel was simply forgotten
pub const FEUD_ENDINGS: [&str; 5] = ["running", "arbitrated", "sealed by marriage",
    "ended in ruin", "cooled"];

/// Intensity thresholds separating the four stages.
const FEUD_STAGE_AT: [f32; 4] = [0.0, 0.30, 0.58, 0.82];
/// Intensity gained per month of live contact, scaled by how much the two houses
/// actually overlap (shared goods × shared cities).
const FEUD_HEAT: f32 = 0.055;
/// Monthly cooling when the two no longer touch the same trade or the same city.
const FEUD_COOL: f32 = 0.035;
/// Below this, a feud is forgotten (dropped, `rivals` cleaned up).
const FEUD_FORGET: f32 = 0.04;
/// Wealth the loser of a flare gives up, per stage. The old model charged a flat 8%
/// every time; a cold rivalry now costs almost nothing and only a vendetta bites.
///
/// CALIBRATED, not chosen. The old flat bite was crude but it produced something worth
/// keeping: a house that kept losing was ground DOWN and stayed down, so some cities
/// had a poor ruling family and boiled over. The first cut of these numbers (peaking at
/// 7.5%/flare) let a losing house out-compound its own feud — house wealth in
/// `unrest_topples_councils` went from 676 at baseline to 309 596, every city came out
/// prosperous, and the revolt the test exists to catch stopped happening. A sustained
/// trade war has to beat the economy's own growth rate (~14%/yr in that scenario) or a
/// feud is decoration. At the values below a trade war costs the loser ~14%/yr and a
/// vendetta ~37%/yr, which restores divergence without touching the growth model.
/// Gates: `unrest_topples_councils` AND `simulate_decades_reports_dynamics`.
const FEUD_BITE: [f32; 4] = [0.005, 0.025, 0.060, 0.110];
/// Chance per month that a live feud flares at all, per stage.
const FEUD_FLARE_CHANCE: [f32; 4] = [0.04, 0.12, 0.20, 0.28];
/// A council will impose a settlement on two houses that both trade in it once the
/// feud has run this long and the city is not itself at war.
const FEUD_ARBITRATE_YEARS: u32 = 12;
/// Yearly chance an eligible feud is actually arbitrated.
const FEUD_ARBITRATE_CHANCE: f32 = 0.22;
/// Ceiling on prestige a house may reach THROUGH winning feuds. Prestige is otherwise
/// unbounded and feeds political power, so feud winnings need their own stop.
const FEUD_PRESTIGE_CAP: f32 = 1.2;
/// Prestige a house loses when a council has to settle its quarrel for it.
const FEUD_ARBITRATE_PRESTIGE: f32 = 0.04;
/// Influence a trade-war stage strips from the loser in a contested city, per flare.
const FEUD_INFLUENCE_STRIP: f32 = 0.06;
/// Cap on simultaneous feuds tracked, so a 500-year run cannot grow this without bound.
const FEUDS_CAP: usize = 400;
/// Per-feud flare log cap (the panel shows a recent window).
const FEUD_LOG_CAP: usize = 12;

/// One flare in a feud's history — what happened, when, and what it cost.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeudFlare {
    pub tick: u32,
    /// Stage the feud was at when this happened (`FEUD_STAGES`).
    pub stage: u8,
    /// House that came off worse.
    pub loser: u32,
    /// Wealth the loser gave up (grain-eq).
    pub cost: f32,
    pub text: String,
}

/// A running quarrel between two merchant houses. Unlike the old symmetric `rivals`
/// entry this is a first-class object with a cause, a temperature and an ending, so a
/// feud can be shown, reasoned about, and settled.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Feud {
    /// House indices, always stored with `a < b` so a pair has ONE feud.
    pub a: u32,
    pub b: u32,
    /// `FEUD_*` cause code — why it began.
    pub cause: u8,
    /// The good the quarrel is over (−1 when it is not about a specific trade).
    pub good: i32,
    /// The city they contest (−1 when the quarrel is not local to one market).
    pub hub: i32,
    /// 0..1 temperature. Grows with live overlap, cools without it.
    pub intensity: f32,
    /// `FEUD_STAGES` index, derived from `intensity` (hysteretic — see `feud_stage`).
    pub stage: u8,
    pub started_tick: u32,
    pub last_flare_tick: u32,
    pub flares: u32,
    /// Cumulative wealth each side has lost to the other over the feud's life.
    pub damage_a: f32,
    pub damage_b: f32,
    /// `FEUD_RUNNING` while live; otherwise how it ended.
    pub outcome: u8,
    pub ended_tick: u32,
    /// Recent flares (capped) — the feud's own chronicle.
    pub log: Vec<FeudFlare>,
}

/// Stage for an intensity, with hysteresis: a feud must fall a clear margin below a
/// threshold before it de-escalates, so a value sitting on the line does not oscillate
/// between "trade war" and "open feud" every month.
pub fn feud_stage(intensity: f32, current: u8) -> u8 {
    let up = (0..4).rev().find(|&s| intensity >= FEUD_STAGE_AT[s]).unwrap_or(0) as u8;
    if up >= current { return up; }
    // De-escalate only once 0.06 below the stage the feud is currently holding.
    if intensity < FEUD_STAGE_AT[current as usize] - 0.06 { up } else { current }
}

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

// ── Expeditions & Corridors ──────────────────────────────────────────────────
// A permanent trade corridor is EARNED: a wealthy house finances a risky
// expedition toward a distant, unconnected, valuable city; hazards cull it; only
// after several successful round-trips does the route become an established
// corridor, at which point port / caravanserai villages are founded along it.
/// Expeditions open from ~year 25 — WORLD_AND_TRADE_MASTER_PLAN.md Part III §6
/// decision 7: five years of exploration before COLONY_START_TICK's year-30
/// founding passes open, now that expeditions gate founding (§1.2) rather than
/// running in parallel with it. Was year 15.
const EXP_START_TICK: u32 = 25 * TICKS_PER_YEAR;
/// A house needs this much wealth to bankroll a venture (they are expensive).
const EXP_MIN_HOUSE_WEALTH: f32 = 60.0;
/// Max simultaneously-active ventures (keeps the tick + overlay bounded).
const EXP_MAX_ACTIVE: usize = 10;
/// Reference haul (cells-equivalent) the fleet-size formula scales against.
const EXP_REF_KM: f32 = 2200.0;
/// Outfitting cost per transport unit (caravan/ship), before terrain/sea risk.
const EXP_UNIT_COST: f32 = 2.4;
/// Minimum straight-line separation (fraction of world width) for a venture —
/// corridors are LONG hauls, not next-door ties. Player-reported + diagnosed: at the
/// old 0.14 (≈5,600 km on an Earth-scale world — `EARTH_EQUATOR_KM`) EVERY venture
/// already started beyond that floor, and the scoring formula below used to reward
/// distance with no ceiling at all, so expeditions systematically reached for the
/// far side of the map. Lowered so a venture can target a REGIONAL unconnected
/// settlement, not only a hemisphere away.
const EXP_MIN_GAP_FRAC: f32 = 0.035;
/// Upper bound (fraction of world width) — companion to the floor above. Without
/// one, `expedition_launch_pass`'s own scoring (which used to reward raw distance
/// with no ceiling) had nothing stopping it from picking the single farthest
/// reachable city every time.
const EXP_MAX_GAP_FRAC: f32 = 0.22;
/// Successful round-trips AND cumulative profit needed to establish a corridor
/// (MODERATE: a couple of proven, profitable round-trips prove the route).
const EXP_MIN_SUCCESSES: u16 = 2;
const EXP_EST_PROFIT: f32 = 10.0;
/// Prospect ledger is dropped if abandoned this long (keeps the vec bounded).
const EXP_PROSPECT_TTL: u32 = 30 * TICKS_PER_YEAR;
/// A day's-march spacing (km) for caravanserais on the land legs of a corridor.
const EXP_DAY_MARCH_KM: f32 = 380.0;
/// Recent failed-venture markers kept for the map ✕ overlay.
const EXP_FAILED_CAP: usize = 60;
/// Base per-tick hazard intensity (MODERATE difficulty: ~half of early ventures
/// are expected to fail before a route is proven).
const EXP_HAZARD_BASE: f32 = 0.014;
/// Human labels for `HazardEvent.kind`, indexed 0..=6.
const HAZARD_LABEL: [&str; 7] = [
    "fever", "the climate", "a native raid", "a storm", "shipwreck", "starvation", "bandits",
];

/// One peril that struck an expedition mid-journey (drives the struggle narrative
/// and the failed-attempt map markers). `kind`: 0 illness · 1 climate · 2 native
/// raid · 3 storm · 4 shipwreck · 5 starvation · 6 bandits.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HazardEvent {
    pub tick: u32,
    pub x: f32,
    pub y: f32,
    pub kind: u8,
    /// Fraction of the fleet lost to this event (0..1).
    pub losses: f32,
}

/// A financed trading venture toward a distant, not-yet-connected city. Travels
/// visibly over months; hazards cull it; a total loss is a Failed attempt.
/// `status`: 0 en-route (outbound) · 1 arrived · 2 returning · 3 succeeded · 4 failed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Expedition {
    pub id: u32,
    pub house: u32,          // backer (house index)
    pub leader: String,      // "Doran of House Vell"
    pub origin: u32,         // origin hub id
    pub dest: u32,           // destination hub id
    pub ox: f32, pub oy: f32,
    pub dx: f32, pub dy: f32,
    pub launched_tick: u32,
    pub travel_ticks: u32,   // one-way duration
    pub pos: f32,            // 0..1 progress along the CURRENT leg
    pub outbound: bool,      // heading out (true) vs returning home (false)
    pub caravans: u16,
    pub ships: u16,
    pub good: u16,           // chief cargo good index
    pub cargo_qty: f32,      // units bought at origin
    pub cost: f32,           // capital committed (registered)
    pub revenue: f32,        // banked on arrival/return
    pub arrived_frac: f32,   // surviving fraction of the fleet (1 → 0)
    pub status: u8,
    pub hazards: Vec<HazardEvent>,
    /// Phase 1.3 · the destination's PROVINCE (from `hub_province`), −1 if the
    /// destination hub has none. Lets the house panel highlight where an expedition is
    /// actually reaching for on the province plate, and makes a "reach ⟨province⟩"
    /// goal (Phase 3, unbuilt) checkable once goals exist.
    #[serde(default = "neg_one_i32")] pub dest_province: i32,
}

/// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.9 (D7/D8) · a house's out-of-town
/// acquisition attempt: INTENT → DISPATCH → TRAVEL → STANDING → ROUNDS →
/// OUTCOME (§8.4 of the plan). Cross-city only — a same-city buy stays on
/// `estate_resale_pass`'s cheaper, instant path; the envoy exists because
/// DISTANCE is the mechanic (a rival already resident can close the deal
/// while the envoy is still travelling — `status` 5, pre-empted).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Envoy {
    pub house: u32,
    pub origin_hub: u32,
    pub target_estate: u32,
    pub dispatched_tick: u32,
    pub arrive_tick: u32,
    /// The negotiation is resolved in one settling at arrival (§8.4's "ROUNDS"
    /// collapse to a single check here rather than their own multi-tick state —
    /// the travel itself is what "rounds" would have added little to, and this
    /// keeps the mechanic legible without a second timer). Always 0 while
    /// travelling, 1 once resolved.
    pub rounds_done: u8,
    /// 0 travelling · 1 reserved · 2 agreed (full purchase) · 3 partial (a
    /// minority share) · 4 refused · 5 pre-empted.
    pub status: u8,
}

/// The attempt ledger for a city-pair (a<b by id): a corridor is established only
/// after repeated proven success, so failures accumulate here first.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RouteProspect {
    pub a: u32,
    pub b: u32,
    pub attempts: u16,
    pub successes: u16,
    pub cum_profit: f32,
    pub last_tick: u32,
    pub established: bool,
}

/// An established, permanent trade corridor (event-driven — recorded once, so the
/// overlay no longer recomputes routes every year). Carries the founding story.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Corridor {
    pub a: u32,              // hub id (origin / backer's home side)
    pub b: u32,              // hub id (destination)
    pub owner: i32,          // backing house index (−1 civic)
    pub good: u16,           // chief commodity the corridor carries
    pub founded_tick: u32,
    pub attempts: u16,       // ventures it took to prove the route
    pub successes: u16,
    pub ports: Vec<u32>,          // founded port-village hub ids (coast)
    pub caravanserais: Vec<u32>,  // founded caravanserai-village hub ids (inland)
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
    /// Phase 0.4 · each people's LAW OF INHERITANCE, resolved once from its language
    /// kit (see `sim::inheritance`) and then fixed for the campaign. Held here rather
    /// than looked up per succession so a reloaded save cannot re-roll a culture's law,
    /// and so the tick stays free of the worldgen culture map.
    #[serde(default)]
    pub culture_rules: Vec<CultureRule>,
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
    #[serde(default)]
    pub hub_patron: Vec<i32>,
    /// Settlement DEVELOPMENT tier (0..5) per hub, persisted with hysteresis so it is
    /// earned/lost over a year rather than flickering. Hub-indexed; resized each tick.
    #[serde(default)]
    pub dev_tier: Vec<u8>,
    /// Hysteresis momentum for `dev_tier` (consecutive confirming half-years, ±).
    #[serde(default)]
    pub dev_momentum: Vec<i8>,
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
    /// Cultures 2.0 · the LINGUA FRANCA of each trade region (connectivity component):
    /// the language family of the culture that dominates the region's cities becomes
    /// its trade tongue. It eases cross-family assimilation there, and LINGERS after
    /// the dominant culture fades (a legacy tongue). Recomputed yearly. serde-default.
    #[serde(default)]
    pub lingua: Vec<LinguaFranca>,
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
    /// DIAGNOSTIC · why the ownerless residual took a shipment.
    #[serde(default)] pub diag_why_nohouse: u32,
    #[serde(default)] pub diag_why_slot: u32,
    #[serde(default)] pub diag_why_cash: u32,
    #[serde(default)] pub diag_why_bar: u32,
    /// N1 (`ACTORS_AND_CARRIAGE_PLAN.md`) · a long haul with no house carrier that
    /// the bind refused to let sail at all. Zero while `N1_LOCAL_HAUL_BIND_DAYS`
    /// stays at infinity.
    #[serde(default)] pub diag_why_no_carrier_bind: u32,
    /// Charter exclusivity (`CHARTER_EXCLUSIVE_DOSE`) — a leg barred because the
    /// destination hub has chartered this good to a house that isn't the carrier.
    /// Zero while the dose stays at 0.0.
    #[serde(default)] pub diag_why_charter_bar: u32,
    /// N1c (`SHIP_LEG_MAX_KM`/`CARAVAN_LEG_MAX_KM`) — an ownerless leg refused
    /// because it exceeds its mode's real per-voyage range.
    #[serde(default)] pub diag_why_leg_range_bind: u32,
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
    /// WORLD_AND_TRADE_MASTER_PLAN.md Part II Slice C1 (the entrepôt) — parallel
    /// to `days` (n·n), naming the OUTLET hub a same-component pair's route was
    /// composed through, or -1 when the direct/fallback route already won. Built
    /// alongside `days` in `rebuild_routes`, never serialized for the same reason.
    /// Dispatch reads it to route a small cut of a settled trade's profit to the
    /// outlet — a pure redistribution (rule 18), never added on top.
    #[serde(skip)]
    pub route_outlet: Vec<i32>,
    /// PATHFOUND base route-days for the founding hubs (`base_n` × `base_n`), computed once
    /// at campaign start over the SAME coarse cost grid the trade-route LAYER uses (passes,
    /// rivers, coast-hugging, sea crossings — the trade-route generation rules), so campaign
    /// routes follow real lanes and never a straight line. Serialized (survives save/load);
    /// hubs added later (colonies, index ≥ `base_n`) fall back to Euclidean via `rebuild_routes`.
    #[serde(default)] pub base_days: Vec<f32>,
    #[serde(default)] pub base_n: usize,
    /// N5 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §1.2) · per-lane seasonal
    /// travel-time multiplier, quantised to a `u8` — flat
    /// `season_slices × base_n × base_n`. `base_days` itself keeps holding the
    /// ANNUAL MEAN unchanged; this is read only through `lane_days`. Empty on
    /// an old save (or when `season_slices == 0`) ⇒ every slice reads 1.0,
    /// which is the zero-dose/bit-identical gate (`n5_season_multipliers_at_
    /// unity_are_a_noop`).
    #[serde(default)] pub base_days_season: Vec<u8>,
    /// How many slices `base_days_season` carries this campaign (0 = none).
    /// Stored rather than hard-coded so a future finer split is a data change.
    #[serde(default)] pub season_slices: u8,
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
    /// (hub, good, partner, dir, season — 0..3, the calendar quarter `log_trade` was
    /// called in). In-memory only (`skip`) — a mid-year save/reload loses just the
    /// partial current year; completed years live in `trade_last`/`trade_last_
    /// season`/`trade_hist`. Observability only; never feeds back into the
    /// simulation, so it has no bearing on determinism.
    #[serde(skip)]
    pub trade_cur: std::collections::HashMap<(u32, u32, u32, u8, u8), TradeCur>,
    /// Per-hub trade DOMINATOR (house index, −1 = none), recomputed monthly from
    /// `House.influence`. Derived → not serialized. Drives the dominance trade edge.
    #[serde(skip)]
    pub city_dominator: Vec<i32>,
    /// The LAST completed year's flows (detailed per-partner), the routes/partners
    /// breakdown the Flows subtab reads. Appended LAST → `#[serde(default)]`.
    #[serde(default)]
    pub trade_last: Vec<TradeFlowAgg>,
    /// The LAST completed year's flows, split by calendar QUARTER (`season` 0..3) —
    /// the Seasonal Trade panel's source. Same (hub,good,partner,dir) rows as
    /// `trade_last`, just not summed across the year; `trade_last`'s own totals are
    /// always exactly the sum of the matching four `trade_last_season` entries.
    /// Appended LAST → `#[serde(default)]`, so an old save loads with this empty
    /// until the next New Year folds a real quarterly breakdown into it.
    #[serde(default)]
    pub trade_last_season: Vec<TradeFlowAgg>,
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
    /// Live and settled FEUDS between houses. `houses[].rivals` is kept in sync with
    /// the running ones, so every existing reader (war causes, marriage eligibility,
    /// the Houses panel) is unchanged; this carries the cause/temperature/ending the
    /// flat rival list could not. `#[serde(default)]` so old saves load with none and
    /// rebuild their feuds from the current rival lists on the next pass.
    #[serde(default)]
    pub feuds: Vec<Feud>,
    /// ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.9 (D7/D8) · out-of-town acquisition
    /// attempts in flight or recently resolved (kept ~a month past resolution so
    /// the House Dossier can show the outcome, then GC'd — see `envoy_travel_pass`).
    #[serde(default)]
    pub envoys: Vec<Envoy>,
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
    /// Expeditions & Corridors — financed ventures currently en route / returning.
    /// Appended LAST → `#[serde(default)]` so old `.campaign` saves load with none.
    #[serde(default)]
    pub expeditions: Vec<Expedition>,
    /// Per-pair attempt ledgers (a corridor is earned over several tries).
    #[serde(default)]
    pub route_prospects: Vec<RouteProspect>,
    /// Recent FAILED-venture markers (for the map ✕), bounded to `EXP_FAILED_CAP`.
    #[serde(default)]
    pub failed_expeditions: Vec<HazardEvent>,
    /// Established permanent corridors (event-driven overlay source).
    #[serde(default)]
    pub corridors: Vec<Corridor>,
    /// Next expedition id to hand out.
    #[serde(default)]
    pub next_expedition_id: u32,
    // ── Provinces (Phase 2b · watershed demography) ─────────────────────────────
    // All serde-defaulted → old saves AND the dynamics test (which never seeds
    // provinces) load with these EMPTY, and every province routine early-returns on
    // empty, so the base economy is completely unchanged unless a world's province
    // partition has been seeded into the campaign (real games via campaign_start_sim).
    /// Rural (countryside) population per province id — the migration reservoir.
    #[serde(default)] pub prov_rural: Vec<f32>,
    /// Rural carrying capacity per province id (from the land's food potential).
    #[serde(default)] pub prov_cap: Vec<f32>,
    /// Province majority culture (the identity migrants carry into the cities).
    #[serde(default)] pub prov_culture: Vec<String>,
    /// Province seat (x,y), for assigning campaign-founded hubs by nearest seat.
    #[serde(default)] pub prov_seat: Vec<[f32; 2]>,
    /// Each hub's province id (-1 = unknown / unmapped).
    #[serde(default)] pub hub_province: Vec<i32>,
    /// Rolling net migration per province this year (source<0 / sink>0), for the panel.
    #[serde(default)] pub prov_net_mig: Vec<f32>,
    /// Province adjacency (id → neighbouring province ids), for OVERLAND plague spread
    /// across the countryside from one province to the next.
    #[serde(default)] pub prov_neighbors: Vec<Vec<u32>>,

    // ── Provinces · LAND STATE (FIX_PLAN B1's missing half) ─────────────────────
    // `prov_rural` proved the pattern: a mutable per-province quantity the campaign
    // advances yearly. These are the same pattern applied to the LAND, plus the
    // feedback edge (`prov_surplus` → the seat city's food stock) that closes the
    // world↔campaign loop. Every one is serde-defaulted and `province_land_pass`
    // early-returns on empty, so a campaign without provinces — including the
    // dynamics test — is bit-identical.
    /// Woodland fraction 0..1. Falls as population clears land for the plough.
    #[serde(default)] pub prov_forest: Vec<f32>,
    /// Cropped fraction 0..1 — what is actually under the plough this year.
    #[serde(default)] pub prov_arable: Vec<f32>,
    /// Grazed fraction 0..1.
    #[serde(default)] pub prov_pasture: Vec<f32>,
    /// Irrigated share of the arable 0..1 — a durable improvement (`ProvWork`).
    #[serde(default)] pub prov_irrigated: Vec<f32>,
    /// Soil condition 0..1. Depletes under intensive cropping, recovers on fallow.
    #[serde(default)] pub prov_soil: Vec<f32>,
    /// Province works v2.0 · real area (km², from the world's own `Province.area_km2`)
    /// — a work's cost scales with how much land there actually is to clear/drain/
    /// irrigate/road. Frozen at campaign start like every other geography figure;
    /// zero (and so a no-op multiplier, see `work_cost`) on a pre-v2.0 save.
    #[serde(default)] pub prov_area_km2: Vec<f32>,
    /// Province works v2.0 · real relief (m, `Province.relief_m` = max−min elevation)
    /// — how broken the country is, which is what actually drives the cost of a road
    /// or a drainage channel far more than flat acreage does. Same freeze/fallback
    /// discipline as `prov_area_km2`.
    #[serde(default)] pub prov_relief_m: Vec<f32>,
    /// Tenure shares — [civic/crown, house/noble, temple, common], summing to ~1.
    #[serde(default)] pub prov_tenure: Vec<[f32; 4]>,
    /// Rural tax rate 0..`PROV_TAX_MAX` set by the holder polis (or a player).
    /// Suppress realm formation entirely. Exists for ONE caller: the inheritance
    /// gate (`econ_inheritance_rules_fragment_differently`), which compares four
    /// 60-year sub-simulations that differ only in inheritance law.
    ///
    /// `REALM_YEAR_FLOOR` is 50 and that gate runs 60 years, so a decade of
    /// realm formation lands inside its window — and a coronation moves a whole
    /// house's fortune out of the merchant pool at once (the plan's own §5.2
    /// warning, "crowns drain the merchant pool"). That perturbation is large,
    /// path-dependent, and entirely orthogonal to which law the test is measuring:
    /// it swamped the wealth signal and inverted the result. Excluding it isolates
    /// the variable rather than hiding a regression — the same reason the gate
    /// already fixes the seed and the world.
    ///
    /// Never set outside that test. Realm formation is measured by
    /// `econ_measure_realm_paths` instead, on a world built for it.
    #[serde(default)] pub suppress_realms: bool,
    /// Test-only, and for the SAME ONE CALLER as `suppress_realms` above:
    /// `econ_inheritance_rules_fragment_differently`. Suppresses CRISIS RELIEF
    /// (`polis.rs::decide_crisis_relief`).
    ///
    /// Same reasoning, measured the same way. Relief is a FOOD-MARKET intervention:
    /// it keeps struggling towns alive (the standing dynamics run holds 30 towns to
    /// year 40 with it, against losing one by year 15 without), which changes which
    /// houses survive and therefore how many were ever founded — path-dependent, and
    /// orthogonal to the law of inheritance this gate measures. With it in, the
    /// weakest of the gate's four assertions flipped on a 3% margin: 190 houses ever
    /// under partible against 196 under primogeniture.
    ///
    /// What was NOT hidden by isolating it: the gate's substantive claim held
    /// throughout. Assertion 3 — the one the test's own printed note calls "the
    /// measure that actually moves" — stayed clean and wide (mean wealth 141,368
    /// partible against 157,415 primogeniture). Only the house COUNT moved, and only
    /// within the noise band this gate has flipped inside three times before (see
    /// `ESTATES_SHARES_AND_WAREHOUSE_PLAN.md` 4.7 and 4.9, and `suppress_realms`).
    ///
    /// Never set outside that test. Crisis relief is measured by the standing
    /// dynamics run and the economy scorecard instead.
    #[serde(default)] pub suppress_relief: bool,
    #[serde(default)] pub prov_tax: Vec<f32>,
    /// Unpaid dues accumulated in bad years — collected later or written off.
    #[serde(default)] pub prov_arrears: Vec<f32>,
    /// Rural unrest 0..1 from crowding, taxation and tenure concentration.
    #[serde(default)] pub prov_unrest: Vec<f32>,
    /// Last year's food surplus delivered into the seat city's stock (grain-eq).
    #[serde(default)] pub prov_surplus: Vec<f32>,
    /// Last year's rural dues delivered to the holder's treasury (grain-eq).
    #[serde(default)] pub prov_revenue: Vec<f32>,
    /// The hub whose writ runs here (−1 = no seat / unadministered).
    #[serde(default)] pub prov_holder: Vec<i32>,
    /// Phase 5 (`HOUSE_INHERITANCE_AND_TERRITORY.md` Part D) · a HOUSE whose writ runs
    /// here instead of a city's (−1 = none — the ordinary case, a city administers).
    /// The Stato da Mar case: a merchant house granted a province collects its dues
    /// directly. Every reader of `prov_holder` must keep tolerating a house holding a
    /// province instead — see `province_authority_is_not_assumed_to_be_a_city`.
    #[serde(default)] pub prov_holder_house: Vec<i32>,
    /// Multi-year land improvements under way (clearance, drainage, irrigation, road).
    #[serde(default)] pub prov_works: Vec<ProvWork>,
    /// Yearly samples per province — what the Province panel's time slider scrubs.
    #[serde(default)] pub prov_history: Vec<Vec<ProvSample>>,
    /// Per-province chronicle (revolt, famine, clearance finished, …), capped.
    #[serde(default)] pub prov_events: Vec<Vec<ProvEvent>>,
    /// CITY_PROVINCE_WAR_PLAN.md §2.5 · the FROZEN per-(province, good) belt score
    /// (0..1), flat `prov_count * goods.len()`, snapshotted once at campaign start
    /// from `Province.good_belt` — the world half's own per-good land quality, never
    /// touched again (the one-way snapshot, CLAUDE.md §3.4). `potential` scales this
    /// by LIVE land use every query; the belt itself never changes.
    #[serde(default)] pub prov_good_belt: Vec<f32>,
    /// §2.5 · the ONE piece of exploitation state that actually accumulates in the
    /// tick: a soft-cap pressure multiplier per (province, good), flat like
    /// `prov_good_belt` above. Erodes `potential` when a good is over-worked, heals
    /// when the pressure eases — reuses `prov_soil`'s own wear/heal SHAPE (see
    /// `update_province_goods_pressure`). `potential`/`actual`/`exploitation`
    /// themselves are NOT stored: they're cheap to derive fresh from current land
    /// use + live hub production + this depletion term, so storing them would just
    /// be a second, staler copy.
    #[serde(default)] pub prov_good_depletion: Vec<f32>,
    /// §2.5 · a SINGLE world-wide scalar, self-calibrated once at campaign start
    /// (mirroring `need_scale`'s own calibration in `lifecycle.rs`) so that mean
    /// exploitation reads ≈1.0 on the day the campaign begins, whatever the world's
    /// size or belt intensities happen to be — no hand-tuned constant that would
    /// silently read wrong on a world shaped differently from the one it was picked
    /// against. Serde-defaults to 1.0 (a no-op) for a save from before this existed.
    #[serde(default = "one_f32")] pub prov_good_yield_scale: f32,

    // ═══════════════════════════════════════════════════════════════════════════
    //  REALMS (`docs/REALM_AND_GOVERNMENT_PLAN.md`, R1) — THE THIRD AUTHORITY
    //  LAYER. `prov_holder` says who ADMINISTERS a province and
    //  `prov_holder_house` who is PAID by it (rule 24); `prov_realm` says who is
    //  OBEYED there. All three are independent and all three must be tolerated by
    //  any reader — a house-held writ inside a realm's borders stays legal
    //  (`CITY_PROVINCE_WAR_PLAN.md` §5.9).
    //
    //  **Rule 25 · sovereignty is never assumed to exist.** `prov_realm == -1` is
    //  the pre-state default. It is what every province looks like in year 1 and
    //  what most of them still look like at year 500, so every routine here early-
    //  returns on an empty `realms` list exactly as the land layer early-returns on
    //  an empty `prov_rural`. That is what keeps a campaign without realms — which
    //  includes `simulate_decades_reports_dynamics`, whose sim carries no province
    //  layer at all — bit-identical.
    // ═══════════════════════════════════════════════════════════════════════════
    /// Every realm ever founded, live and fallen. Empty until the first
    /// proclamation, which cannot happen before year 50 (R1b).
    #[serde(default)] pub realms: Vec<Realm>,
    /// N7 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §3) — every League ever
    /// founded, dissolved ones kept (`dissolved_tick != 0`) exactly as a
    /// fallen `Realm` is. Membership lives on `TickHub.league`, never here.
    #[serde(default)] pub leagues: Vec<League>,
    /// Per-province sovereignty: an index into `realms`, or −1 for free land.
    /// Sized alongside the rest of the land layer by `ensure_province_land`.
    #[serde(default)] pub prov_realm: Vec<i32>,

    // ── Province TRADE FLOW (per-good origin→destination accounting) ─────────────
    // The exact per-good tonnage crossing a province's boundary: a shipment from a
    // hub in province A to a hub in province B is an EXPORT of that good from A and
    // an IMPORT of it into B; an intra-province haul is neither. Accumulated in
    // `accrue_flow` (the single choke point every shipment passes) and snapshotted
    // yearly in `roll_city_finances`, exactly like `flow_accum`→`flow_year` and
    // `good_flow_accum`→`hub_good_trade`. All gated on a NON-EMPTY `hub_province`,
    // so a campaign without provinces (incl. the dynamics test) never touches them
    // and stays bit-identical. Flat `prov_count * goods.len()`, province-major.
    /// In-year export accumulator (goods leaving each province), rebuilt each year.
    #[serde(skip)] pub prov_export_accum: Vec<f32>,
    /// In-year import accumulator (goods entering each province from outside).
    #[serde(skip)] pub prov_import_accum: Vec<f32>,
    /// Last full year's exports per (province, good) — the figure the UI and the
    /// realm trade-share eligibility read (a stable yearly total, not mid-year noise).
    #[serde(default)] pub prov_export_year: Vec<f32>,
    /// Last full year's imports per (province, good).
    #[serde(default)] pub prov_import_year: Vec<f32>,

    // ── YARDS_VESSELS_AND_DEPOTS_PLAN.md ──
    /// S2/S3 · every vessel ever built or seeded, live or lost. Appended LAST →
    /// `#[serde(default)]`, so a pre-yards save loads with none (and stays
    /// bit-identical, since nothing yet reads this list for capacity).
    #[serde(default)] pub vessels: Vec<Vessel>,
    /// Running id counter for `vessels` (monotonic, never reused, so a lost
    /// hull's id is never handed to a new one).
    #[serde(default)] pub next_vessel_id: u32,
    /// W5 · every fondaco ever founded. Empty until `maybe_found_fondaco` is
    /// wired to actually run (it currently never is — see `Fondaco`'s own doc).
    #[serde(default)] pub fondacos: Vec<Fondaco>,
    /// DEPOSITS_AND_MINING_PLAN.md slice 4 · every real ore/gem/stone WORKING this
    /// world's geology placed (§8.16), seeded ONCE at campaign start from
    /// `metadata["deposits"]` (`lifecycle.rs`) — a positional/depth index, not a
    /// duplicate of the full `sim::deposits::Deposit` record (grade/extent aren't
    /// needed here). Read by `mine_depth_at` when an estate is founded; never
    /// mutated afterward (the one-way snapshot, CLAUDE.md §3.4). Empty on an old
    /// save, a template/painted world, or a world generated before slice 1 — every
    /// reader treats that as "no depth data", never as "no deposit exists".
    #[serde(default)] pub mine_deposits: Vec<MineSite>,
}

/// DEPOSITS_AND_MINING_PLAN.md slice 4 · one real geological working as seeded
/// into `CampaignSim::mine_deposits` — the good it produces, its cell, and its
/// depth class (`sim::deposits::DEPTH_SURFACE`/`_SHALLOW`/`_DEEP`/`_FLOODED`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MineSite {
    pub good: String,
    pub x: f32,
    pub y: f32,
    pub depth: u8,
    #[serde(default = "unknown_extent")] pub extent: u8,
    #[serde(default)] pub district: u32,
}

fn one_f32() -> f32 { 1.0 }

/// Realm rank — the realm ladder, which REPLACES the merchant tier ladder for a
/// crowned house. Assigned like `assign_house_tiers`/`assign_city_tiers` (percentile
/// among live realms + an absolute floor for the top rank + hysteresis), so a young
/// world's realms are all city-states and "great power" means something. Unused until
/// R1b founds the first realm.
pub const REALM_CITY_STATE: u8 = 0;
pub const REALM_KINGDOM: u8 = 1;
pub const REALM_GREAT_POWER: u8 = 2;
pub const REALM_HEGEMON: u8 = 3;

/// How an annexed city is governed — the one policy that decides what a conquest
/// keeps, how far the writ carries, and how likely a member is to make a separate
/// peace (plan §3.4). Sticky: changing it is a reform with an unrest cost, never a
/// toggle.
pub const AUTONOMY_CENTRALIZED: u8 = 0;
pub const AUTONOMY_CORE_PERIPHERY: u8 = 1;
pub const AUTONOMY_AUTONOMOUS: u8 = 2;

/// A city's standing inside a realm (`TickHub.realm_role`).
pub const REALM_ROLE_SEAT: u8 = 0;
pub const REALM_ROLE_SUBJECT: u8 = 1;
pub const REALM_ROLE_TRIBUTARY: u8 = 2;
pub const REALM_ROLE_OCCUPIED: u8 = 3;

// ── Realm formation paths (`Realm.founding_path`) ────────────────────────────
// The three ways a state comes into being in this world. `REALM_PATH_MERCHANT`
// is 0 so every realm founded before the field existed reads as what it was.
/// A merchant house crowned itself — trade dominance or a captured seat. Venice,
/// Genoa, the Hansa. Rich, and the LOOSEST of the three: its borders follow trade
/// interests rather than land or people.
pub const REALM_PATH_MERCHANT: u8 = 0;
/// A powerful city proclaimed for itself. Rome, Assur, Axum. Compact borders, and
/// it holds together better than a trade network because the thing being held is
/// a city and its own hinterland.
pub const REALM_PATH_CITY: u8 = 1;
/// A contiguous single-culture bloc unified under its largest city. Franks, Poles,
/// Rus'. The TIGHTEST of the three, because the border is where a people ends —
/// and the only path whose frontier a player can read off the culture map.
pub const REALM_PATH_CULTURE: u8 = 2;

/// A crown that passes by blood (`Realm.government`).
pub const REALM_GOV_DYNASTIC: u8 = 0;
/// A crown held by an office, not a family — a republic. Its `family` stays empty
/// and succession is by the council, never by birth.
pub const REALM_GOV_CIVIC: u8 = 1;

/// Cohesion a realm settles toward, by founding path. Cohesion is the share of
/// what a crown assesses that it can actually COLLECT (see
/// `realm_collection_efficiency`), so these are not flavour — a mercantile realm
/// is measurably poorer at governing the same land than a national one.
pub const REALM_COHESION_TARGET: [f32; 3] = [0.62, 0.78, 0.92];
/// How fast cohesion moves toward its target each year. Slow: a realm's grip is a
/// generational property, not something that swings with one bad harvest.
pub const REALM_COHESION_DRIFT: f32 = 0.10;
/// Cohesion lost per province held whose culture differs from the capital's, as a
/// share of the realm's provinces. THE brake on unlimited expansion, and the
/// reason the three paths diverge over time rather than converging: conquest of
/// foreign ground is what turns a tight realm into a loose one.
pub const REALM_COHESION_FOREIGN_PENALTY: f32 = 0.45;
/// How much a realm's own legitimacy pulls its cohesion. Small — legitimacy is
/// about the RULER's right to rule, cohesion about the realm's grip on its land;
/// they are related, not the same thing.
pub const REALM_LEGITIMACY_TO_COHESION: f32 = 0.15;

/// Percentile cuts for the realm rank ladder, mirroring `CITY_TIER_PCT_CUTS`.
pub const REALM_RANK_PCT_CUTS: [f32; 3] = [0.08, 0.30, 0.70];
/// Hysteresis on those cuts, so a realm on a boundary does not relabel yearly.
pub const REALM_RANK_PCT_DEAD_BAND: f32 = 0.04;
/// The ADDITIONAL absolute floor the top rank carries, so a young world — where
/// every realm is small — has no hegemon at all. A rank that is always occupied
/// carries no information, the same reasoning `TIER1_STANDING_ENTER` encodes.
pub const REALM_RANK_TOP_STANDING_ENTER: f32 = 0.60;
pub const REALM_RANK_TOP_STANDING_EXIT: f32 = REALM_RANK_TOP_STANDING_ENTER - 0.04;
/// Names for `Realm.rank`, indexed by it.
pub const REALM_RANK_NAMES: [&str; 4] = ["city-state", "kingdom", "great power", "hegemon"];

// ── PATH B · a powerful city proclaims for itself ────────────────────────────
/// The city tier at or above which a settlement may proclaim on its own account.
/// Tier 1 already carries its own absolute standing floor (`CITY_TIER1_STANDING_
/// ENTER`), which is what lets this replace `REALM_YEAR_FLOOR`'s calendar gate
/// with an EMERGENT condition: a young world simply has no tier-1 city yet.
pub const REALM_CITY_PATH_TIER_MAX: u8 = 3;
/// Yearly chance a qualifying city actually proclaims. Low: most great cities
/// never became states, and the ones that did took their time about it.
pub const REALM_CITY_PATH_CHANCE: f32 = 0.55;
/// A city needs a treasury of at least this multiple of the world's median city
/// treasury to raise a crown of its own.
pub const REALM_CITY_PATH_TREASURY_MULT: f32 = 1.0;
/// Chance a city-path realm comes out CIVIC (a republic) rather than dynastic,
/// when no single house dominates its government.
pub const REALM_CIVIC_CHANCE: f32 = 0.55;

// ── PATH C · cultural domination ──────────────────────────────────────────────
/// The smallest contiguous single-culture bloc that can unify into a realm.
/// Deliberately LOW: small nations are real (the HRE's counties and
/// prince-bishoprics, Andorra, San Marino) and a pre-modern world needs many
/// polities, not a few large ones. The floor exists only to stop a lone province
/// that happens to carry a culture string from calling itself a nation.
pub const REALM_CULTURE_MIN_PROVINCES: usize = 2;
/// Yearly chance a qualifying culture bloc unifies.
pub const REALM_CULTURE_PATH_CHANCE: f32 = 0.50;
/// The share of a people's provinces that must still be FREE for it to unify.
/// Below this the people is already mostly somebody else's subjects, and putting
/// it back together is a conquest rather than a unification — which this path
/// deliberately is not.
pub const REALM_CULTURE_MIN_FREE_FRAC: f32 = 0.60;

// ── CONSOLIDATION (`realms.rs`) ──────────────────────────────────────────────
// Tilly's count of European political units runs ~500 around 1500 down to ~25 by
// 1900. Before this the model had only the first half of that curve: realms
// formed and fragmented, and nothing ever merged, so a world reached 1500-style
// fragmentation and stayed there permanently. These are the mechanisms that let
// the curve bend back down.

// The rates below are TUNED, and the tuning is the deliverable as much as the
// mechanism. Shipped at their first-guess values (expand 0.22 · vassal 0.14 ·
// integrate 0.18 after 40y) consolidation ran away: of 19 realms founded over two
// centuries only FIVE were still standing, with 16 integrations — the model went
// straight past Tilly's four-century curve and collapsed to a handful of empires
// inside 200 years. Slowing all three and letting secession bite (a higher
// cohesion ceiling, so a strained realm actually sheds ground) is what keeps a
// world of many polities that CAN consolidate rather than one that inevitably
// does. See docs/SCOREBOARD.md for the measured before/after.

/// Yearly chance a realm annexes ONE adjacent free province, at full strength.
/// Scaled by cohesion and rank — a crown that cannot govern what it has does not
/// reach for more.
pub const REALM_EXPAND_CHANCE: f32 = 0.06;
/// A realm will not expand while its cohesion is below this: the grip has to
/// exist before the reach does.
pub const REALM_EXPAND_MIN_COHESION: f32 = 0.45;
/// Treasury a realm must hold, as a multiple of its own province count, before it
/// annexes another. Expansion is administration, and administration costs.
pub const REALM_EXPAND_TREASURY_PER_PROV: f32 = 400.0;

/// Yearly chance a strong realm imposes vassalage on a weaker ADJACENT one.
pub const REALM_VASSAL_CHANCE: f32 = 0.035;
/// How much stronger (by province count + rank) the overlord must be.
pub const REALM_VASSAL_STRENGTH_RATIO: f32 = 2.5;
/// Years a vassal is held before it can be INTEGRATED outright. Long, because
/// swallowing a subject realm whole is the rarest and slowest of these moves.
pub const REALM_VASSAL_INTEGRATE_YEARS: u32 = 80;
/// Yearly chance an eligible vassal is integrated once the term has run.
pub const REALM_INTEGRATE_CHANCE: f32 = 0.02;

/// Yearly chance a province SECEDES when it is culturally foreign, distant and
/// the crown's cohesion has collapsed. The counterweight to expansion, and the
/// reason a realm can shrink and die rather than only ever growing.
pub const REALM_SECEDE_CHANCE: f32 = 0.25;
/// Cohesion below which secession becomes possible at all.
pub const REALM_SECEDE_MAX_COHESION: f32 = 0.55;

/// Years over which proclamation ramps in ABOVE `REALM_YEAR_FLOOR`.
///
/// Without it the floor is a cliff: nothing at all happens until the year it
/// names and then several realms appear in that single year, which reads as a
/// scripted event rather than as a world developing. The ramp scales every
/// proclamation chance from 0 at the floor to full `REALM_RAMP_YEARS` later, so
/// the first crown appears a little after the floor and the rest arrive as a
/// stream — which is also how state formation actually looks: a slow start, then
/// an accelerating cascade as neighbours' example and pressure spread.
pub const REALM_RAMP_YEARS: f32 = 15.0;

/// The hard floor on the first proclamation, in years. Not a trigger — after this
/// date any house that meets the conditions may proclaim, and most never will.
pub const REALM_YEAR_FLOOR: u32 = 50;
/// Tier ceiling on the founding HOUSE at proclamation — "at least tier 2" (tier 1 or 2).
/// The gate applies only here (`REALM_AND_GOVERNMENT_PLAN.md` §3.1), never afterward,
/// which is what lets a realm keep a small capital indefinitely (the Karakorum rule,
/// plan §1.3/§4.3). Per the maintainer's rule the CITY's own tier is no longer gated —
/// only that the house has CAPTURED it (see `maybe_proclaim_realms`).
pub const REALM_PROCLAIM_TIER_MAX: u8 = 2;
/// The COST a house must SPEND to found a realm — a court, a standing retinue, the
/// apparatus of a crown — deducted from the wealth that becomes the new crown's treasury
/// (`promote_house_to_realm`), so founding is a real, deliberate outlay, not a free
/// relabelling. It is ADAPTIVE: this fraction of the WEALTHIEST live merchant house's
/// fortune, so it is always "a great sum only a top house can pay" regardless of the
/// world's absolute wealth scale — a flat figure was either trivial or unreachable
/// depending on the world. The richest house can always afford it (its own wealth sets
/// the bar), so a realm CAN always eventually form; poorer houses cannot.
///
/// Lowered 0.6 → 0.35 (maintainer request: realms far more frequent). A house in the
/// world's top stratum now clears the bar with a comfortable margin rather than needing
/// to be near the very peak, so a proclamation waits on the yearly roll, not on one
/// house saving for a decade. Paired with the `council_house`-counts-too widening of the
/// governing gate in `maybe_proclaim_realms`.
pub const REALM_PROCLAIM_COST_FRAC: f32 = 0.35;
/// A floor so the founding cost is never degenerate in a very poor / empty world.
pub const REALM_PROCLAIM_COST_FLOOR: f32 = 1_000.0;
/// Base per-year chance once every other condition is met. Biased by the head's
/// boldness (axis 0) and expansiveness (axis 3) — the same two axes `decide_fleets`
/// and `update_guilds_and_offices` already read for a comparable "dare to commit"
/// decision.
///
/// Raised 0.14 → 0.35 (maintainer request: realms far more frequent). At 0.14 a fully
/// eligible house took ~7 years on average to proclaim; at 0.35 it is ~2-3, so a
/// qualified capital crowns itself within a few years of clearing the gate instead of
/// lingering eligible-but-quiet for most of a reign.
pub const REALM_PROCLAIM_CHANCE: f32 = 0.50;
/// A SECOND path to realm eligibility (maintainer request, §2.4-measured): a house
/// that commands at least this share of a whole PROVINCE's trade may proclaim a crown
/// over that province — seated at the province's OWN largest city — with NO seat
/// office, NO tier requirement, and a cost scaled to its OWN fortune rather than the
/// world's top stratum. The measured funnel (`econ_measure_realm_formation`) collapses
/// precisely at the seat-writ gate: plenty of tier 1-2 merchant dynasties DOMINATE a
/// province's commerce, but only a handful also hold the formal seat of its largest
/// city, so the seat-writ gate throttled realm formation to ~1/decade. Worse, a
/// province administered from OUTSIDE (a "writ of X" case) could never be reached by
/// the seat-office loop at all, and a regionally-dominant but globally-minor house was
/// gated out by the tier-2 and world-scaled-cost bars even when it clearly ran a
/// province's trade. Trade dominance is the historically truer basis for a merchant
/// republic's rise anyway (a Venice, a Genoa). Share is a house's portion of ALL
/// merchant-house trade volume across the province's cities (`province_trade_shares`,
/// summing `House.trade_at` over the province's hubs). Additive: the seat-writ path
/// (`maybe_proclaim_realms`' main loop) is unchanged; this runs as a second pass
/// (`maybe_proclaim_trade_realms`).
pub const PROV_TRADE_CONTROL_FRAC: f32 = 0.20;
/// The flat wealth a house needs — and pays — to proclaim a realm through PURE TRADE
/// DOMINANCE (`maybe_proclaim_trade_realms`). Per the maintainer: the trade path is
/// deterministic, NOT a yearly dice roll — a private house commanding at least
/// `PROV_TRADE_CONTROL_FRAC` of a province's trade and holding at least this much
/// wealth crowns itself the same year it becomes eligible, spending this sum to do it.
pub const REALM_TRADE_MIN_WEALTH: f32 = 50_000.0;
/// Starting legitimacy/cohesion for a freshly proclaimed realm — high but not
/// perfect: the founding generation's own claim is the strongest a dynasty will ever
/// have, and both gauges are designed to be spent down by real events (plan §5), not
/// to start at an artificial ceiling.
pub const REALM_FOUNDING_LEGITIMACY: f32 = 0.70;
pub const REALM_FOUNDING_COHESION: f32 = 1.0;

// ── R2 · genealogy (`REALM_AND_GOVERNMENT_PLAN.md` §3.7) ──────────────────────
/// Age a person is treated as an adult — mirrors `Kin`'s own "childhood → adult at
/// 16" convention (CLAUDE.md §5, Phase 2.1), so a regency ends at the same age a
/// house's own kin roster would call someone grown.
pub const PERSON_ADULT_AGE: u32 = 16;
/// A ruler with no spouse marries once past this age (a per-year roll, not instant).
pub const PERSON_MARRY_AGE: u32 = 18;
pub const PERSON_FERTILE_MIN: u32 = 16;
pub const PERSON_FERTILE_MAX: u32 = 45;
/// Per-year chance an eligible, unmarried ruler marries.
pub const PERSON_MARRY_CHANCE: f32 = 0.35;
/// Per-year chance a married, fertile-age mother bears a child.
pub const PERSON_BIRTH_CHANCE: f32 = 0.30;
/// Per-year mortality hazard under age 5 — chosen so cumulative under-5 mortality
/// lands near 25% (`1 - (1 - x)^5 ≈ 0.25`), the engine of contested succession the
/// plan's §3.8 fragmentation path B depends on.
pub const PERSON_CHILD_MORTALITY: f32 = 0.057;
/// Legitimacy cost of a regency — a minor on the throne is a real weakness, not a
/// cosmetic footnote.
pub const REGENCY_LEGITIMACY_HIT: f32 = 0.15;

// ── R3 · taxation (`REALM_AND_GOVERNMENT_PLAN.md` §3.3) ───────────────────────
/// Index into `Realm.tax_rates`.
pub const TAX_POLL: usize = 0;
pub const TAX_CUSTOMS: usize = 1;
/// How fast collection efficiency falls off with distance from the capital, as a
/// FRACTION of world width (resolution-independent, the same discipline
/// `EFOLD_MID_KM` uses in km rather than cells). At one world-width of distance
/// (an extreme a founding-era realm never reaches), efficiency is already roughly
/// halved before `cohesion` is even applied.
pub const REALM_DISTANCE_DECAY: f32 = 6.0;
/// Ceiling on each realm-set rate — a fraction of the base each levy taxes
/// (population for poll, `trade_wealth` for customs). Kept modest: these are
/// levies ON TOP of the tithe and a member city's own taxes, not a replacement.
pub const REALM_TAX_MAX: [f32; 2] = [0.015, 0.10];
/// How fast a rate drifts toward its treasury-need target each year — slow, so a
/// single bad year doesn't whipsaw rates (mirrors the smoothing every other AI
/// `decide_*` in this file already uses).
pub const REALM_TAX_DRIFT: f32 = 0.15;
/// Treasury level the crown is comfortable at — below it, rates drift up (toward
/// `REALM_TAX_MAX`); at or above it, they ease back toward zero. A realm founds with
/// `wealth − REALM_PROCLAIM_COST` in the treasury, so this comfort level sits well
/// below what a freshly crowned house is left holding.
pub const REALM_TREASURY_COMFORT: f32 = 2000.0;
/// Poll tax costs mood at the taxed city — regressive, and felt.
pub const POLL_TAX_MOOD_COST: f32 = 0.35;
/// A tax farm's term, in years, and how much of the estimated future income the
/// crown accepts up front (a real discount — the whole point of selling is CASH
/// NOW, not the full expected value; `publicani`/*iltizam* both priced this way).
pub const TAX_FARM_YEARS: u32 = 5;
pub const TAX_FARM_DISCOUNT: f32 = 0.65;
/// The crown only farms out the tithe when actually short of cash — a farm is a
/// distress sale, not a standing policy (plan §3.3's own framing).
pub const REALM_FARM_TREASURY_FLOOR: f32 = 400.0;

// ── R5 · the autonomy axis (`REALM_AND_GOVERNMENT_PLAN.md` §3.4) ──────────────
// One policy tying revenue and cohesion-at-distance together — the DATA field
// (`Realm.autonomy`) and its three values (`AUTONOMY_*`, mod.rs) shipped in R1;
// this is where it first gets a real reader. Scoped to the two effects the
// EXISTING R3 collection machinery already models cleanly (revenue, and how
// efficiency falls off with distance); "annexed city keeps its own coin/market"
// and "separate-peace risk" are the table's other two columns and are NOT wired
// here — the first needs realm coin (deferred, R3), the second needs separate
// peace itself (deferred, R4). Named, not silently skipped.
/// Multiplies collected revenue (tithe + poll + customs) — a centralized crown
/// squeezes harder, an autonomous one leaves more with its cities.
pub fn autonomy_revenue_mult(autonomy: u8) -> f32 {
    match autonomy {
        AUTONOMY_CENTRALIZED => 1.25,
        AUTONOMY_AUTONOMOUS => 0.60,
        _ => 1.0, // core & periphery — the baseline R3 was already tuned against
    }
}
/// Multiplies `REALM_DISTANCE_DECAY` in `realm_collection_efficiency` — a
/// centralized realm feels distance HARDER (administration doesn't reach), an
/// autonomous one is close to distance-insensitive (local elites collect for it
/// regardless of how far the capital is), matching the table's own "Cohesion:
/// low at distance / high, distance-insensitive" language.
pub fn autonomy_distance_mult(autonomy: u8) -> f32 {
    match autonomy {
        AUTONOMY_CENTRALIZED => 1.6,
        AUTONOMY_AUTONOMOUS => 0.3,
        _ => 1.0,
    }
}

/// One entry in a realm's own permanent record. Mirrors `HouseEvent`, and obeys the
/// same discipline: milestones (founding, coronation, conquest, partition, fall) are
/// never pruned — for an observation-only game the chronicle is the product (rule 20).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RealmEvent {
    pub tick: u32,
    pub kind: String,
    pub text: String,
}

/// A COUNTRY. Founded when a house that already holds a city's government proclaims
/// sovereignty (R1b); the house is then ELEVATED — its wealth and trade assets become
/// the crown's and it leaves the merchant world, keeping only its identity as the
/// dynasty (`House.crowned`, never `House.defunct` — see the plan's §5.1 for why that
/// distinction is not cosmetic).
///
/// A realm's territory is the set of provinces whose sovereignty it holds, so a
/// realm's border IS a province border, exactly as `StateRegion`'s already is.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Realm {
    pub id: u32,
    pub name: String,
    /// The style of its ruler, drawn from the founding culture's own vocabulary
    /// rather than a global word list — a Clannish steppe people should not produce
    /// a "Republic".
    pub title: String,
    pub capital_hub: u32,
    /// R5 · the realm this one split FROM at a partible succession, or −1 (an
    /// original proclamation). Mirrors `House.origin_house` exactly — a pointer
    /// to the parent, never a duplicated ancestor list, so a cadet realm's
    /// pedigree stays provable without copying its parent's whole family into
    /// every offshoot.
    #[serde(default = "neg_one_i32")] pub origin_realm: i32,
    /// The dynasty. 1:1 with a `House` for the realm's whole life: a house that
    /// gains a second sovereignty merges it into the realm it already has.
    pub ruling_house: u32,
    pub rank: u8,
    pub autonomy: u8,
    pub provinces: Vec<u32>,
    /// Cities under this realm's writ (subject and tributary alike) are NOT
    /// tracked here — the authoritative membership is `TickHub.realm == this
    /// realm's id`, exactly as `prov_realm` is authoritative for provinces. A
    /// separate `cities` list would be a second copy of the same fact with no
    /// mechanism keeping it in sync, and nothing has ever read one (removed
    /// before anything started relying on it, R4).
    pub vassals: Vec<u32>,
    /// The crown's pot — the house's whole wealth at the coronation. There is no
    /// second pot: the dynasty's money IS the realm's money.
    pub treasury: f32,
    /// Bank debt inherited from the house at the coronation, so a crown can default.
    pub debts: f32,
    pub legitimacy: f32,
    pub cohesion: f32,
    pub founded_tick: u32,
    /// Set when the realm ends (partitioned, conquered, or the dynasty dies out);
    /// the record is kept, exactly as a defunct house's is.
    #[serde(default)] pub fallen_tick: u32,
    #[serde(default)] pub events: Vec<RealmEvent>,
    // ─────────────────────────────────────────────────────────────────────────
    // R2 (`REALM_AND_GOVERNMENT_PLAN.md` §3.7) · genealogy. Appended and serde-
    // defaulted so R1's already-founded realms still load — `realm_family_pass`
    // seeds an empty `family` the first time it sees one (mirrors `ensure_
    // province_land`'s own "backfill on demand" for a layer joined mid-campaign).
    // ─────────────────────────────────────────────────────────────────────────
    /// The current ruler — an index into `family`, or −1 before it is seeded.
    #[serde(default = "neg_one_i32")] pub ruler: i32,
    /// A living relative governing on a minor ruler's behalf, or −1. An index into
    /// `family`, same as `ruler`.
    #[serde(default = "neg_one_i32")] pub regent: i32,
    /// The dynasty's own genealogy — REAL people with real ages, real parents, real
    /// children, not a snapshot. This is what supersedes rule 19's tenure
    /// approximation for a realm specifically (a merchant `House` still uses
    /// `head_lifespan`; a realm has a family to draw a lifespan FROM). NEVER
    /// shrinks — a dead person's entry stays (`died_tick` set), exactly as
    /// `House.kin` never removes a dead kinsman, so every `father`/`mother`/
    /// `spouse` index stays valid for the realm's whole life.
    #[serde(default)] pub family: Vec<Person>,
    // ─────────────────────────────────────────────────────────────────────────
    // R3 (`REALM_AND_GOVERNMENT_PLAN.md` §3.3) · taxation. Collection, not rates,
    // is the constraint — see `realms.rs::realm_collection_efficiency`. The
    // harvest tithe itself is NOT a realm-set rate (the province tax slider stays
    // the player's verb, plan §3.3); only poll and customs are the crown's own
    // levies, `tax_rates`-indexed by `TAX_POLL`/`TAX_CUSTOMS`.
    // ─────────────────────────────────────────────────────────────────────────
    #[serde(default)] pub tax_rates: [f32; 2],
    /// This year's tithe income so far (crown share, after collection efficiency)
    /// — reset to 0 at the START of each year's land pass, read by `decide_realm_
    /// taxes` to price a tax farm against a real, current figure rather than a
    /// stale or invented one.
    #[serde(default)] pub tithe_last_year: f32,
    /// A house currently collecting the tithe in the crown's place, having paid
    /// for the right up front. `None` = the crown collects directly (the default,
    /// and where most realms stay most of the time — a farm is a CASH-NOW
    /// decision, not a steady-state policy).
    #[serde(default)] pub tax_farm: Option<TaxFarm>,
    // ─────────────────────────────────────────────────────────────────────────
    // Realm formation paths + the two fields that make `cohesion` and `rank`
    // mean something. All serde-defaulted, so every realm founded before this
    // loads as a dynastic mercantile city-state — which is exactly what it was.
    // ─────────────────────────────────────────────────────────────────────────
    /// HOW this realm came to be — see `REALM_PATH_*`. Not decoration: it sets the
    /// realm's cohesion target, and the three paths were chosen precisely because
    /// they hold together differently. A merchant republic assembled out of trade
    /// interests is a looser thing than a people that unified itself.
    #[serde(default)] pub founding_path: u8,
    /// Dynastic or civic — see `REALM_GOV_*`. A city that proclaims through its
    /// COUNCIL has no family, so it cannot succeed by birth and must never be
    /// styled "King". This is the split that lets Venice and Castile exist in one
    /// model instead of forcing every polity through a bloodline.
    #[serde(default)] pub government: u8,
}

/// R3 · one active tax farm — `publicani`/*iltizam*, sell N years of collection for
/// cash now. Scoped to the harvest tithe only in this pass (the "universal land
/// tax" anchor, and the simplest base to price against); poll/customs farming is
/// real follow-up work, not built here.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaxFarm {
    pub house: u32,
    pub started_tick: u32,
    /// Total term, in years.
    pub years: u32,
}

/// N7 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §4.1) — a LANE-scoped ban:
/// the boycotting hub will not trade with `target` (optionally only in
/// `good`, −1 = all) until `until_tick`. The N2 extension the League needed:
/// `TickHub.export_ban_until` bans a GOOD to everyone, this bans a PARTNER.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Boycott {
    pub target: u32,
    pub good: i32,
    pub until_tick: u32,
}

/// N7 · a VOLUNTARY association of hubs that stay independent. NOT a realm
/// (§3.1): no provinces, no capital, no succession, no writ. The one
/// collective verb is the boycott. Membership lives on `TickHub.league`,
/// never in a `members` list here — see `Realm`'s own doc for why a second
/// copy of the same fact was removed on purpose.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct League {
    pub id: u32,
    pub name: String,
    /// Where the diet MEETS. Deliberately not called a capital: it holds no
    /// writ, no province, and carries no authority over any member.
    pub seat_hub: u32,
    /// Dues, and the only pot. Spent on the collective act (the boycott, once
    /// walked above zero dose); never taxed from a member's own provinces —
    /// that would be sovereignty.
    pub purse: f32,
    pub founded_tick: u32,
    #[serde(default)] pub dissolved_tick: u32,
    /// The tick a shared-threat signal was last present. Drives §3.3 exit 2
    /// (drift): no threat for `LEAGUE_DRIFT_YEARS` and members leave one at a
    /// time — the mechanism that keeps the member count non-monotone in a
    /// quiet world, mirroring `realm_secession_pass`'s own discipline.
    pub last_threat_tick: u32,
    #[serde(default)] pub boycotts: Vec<Boycott>,
    /// Reuses `RealmEvent`'s exact shape — same cap discipline (rule 20).
    #[serde(default)] pub events: Vec<RealmEvent>,
}

/// R2 · one member of a realm's dynasty. Distinct from `Kin` (a merchant house's
/// roster, regenerated wholesale at every succession) on purpose: a realm's
/// genealogy is the point of R2, so it must persist, not snapshot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Person {
    pub name: String,
    pub female: bool,
    pub born_tick: u32,
    /// 0 while alive.
    #[serde(default)] pub died_tick: u32,
    /// Index into the SAME realm's `family`, or −1 — the founding generation, or a
    /// spouse married in from outside the dynasty (an in-law's own ancestry is
    /// never tracked; cross-realm marriage is deferred, plan §6).
    #[serde(default = "neg_one_i32")] pub father: i32,
    #[serde(default = "neg_one_i32")] pub mother: i32,
    #[serde(default = "neg_one_i32")] pub spouse: i32,
    /// Four culture-derived axes, mirroring `Kin.character` exactly.
    pub character: [i8; 4],
    pub skill: f32,
    /// Set at death, mirroring `HouseHead`'s own convention.
    #[serde(default)] pub epithet: String,
    /// 0 = never reigned.
    #[serde(default)] pub reign_start: u32,
    /// 0 = never reigned, or still reigning.
    #[serde(default)] pub reign_end: u32,
}

/// One yearly sample of a province's mutable state — the series behind the province
/// plate's year slider and the Land tab's trend arrows.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProvSample {
    pub year: u32,
    pub rural: f32,
    pub urban: f32,
    pub forest: f32,
    pub arable: f32,
    pub pasture: f32,
    pub irrigated: f32,
    pub soil: f32,
    pub unrest: f32,
    pub surplus: f32,
}

/// One entry in a province's own history. A city chronicle is a biography; a province
/// chronicle is a history, which is why it belongs at this granularity.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProvEvent {
    pub year: u32,
    /// "clearance" | "drainage" | "irrigation" | "road" | "revolt" | "dearth"
    /// | "exhaustion" | "recovery" | "tax" | "holder".
    pub kind: String,
    pub text: String,
}

// ── Province land-state tuning ───────────────────────────────────────────────────
/// The `Province.good_belt` value a good produces when it is ABSENT from the WHOLE
/// province: every cell falls in the belt histogram's bin 0, whose centre is
/// `(bin_w/2)/255` = 8/255 (`GOOD_BINS`=16, see `shared/provinces.rs`). It is an exact,
/// single value, and any real belt in even one cell pushes the province mean strictly
/// above it — so a good is "producible here" iff `good_belt > PROV_GOOD_ABSENT_BELT`.
/// This is the gate that keeps a tropical good (pepper) off an arctic province while
/// still surfacing every good that genuinely covers part of a province, and it works on
/// EXISTING saves (the frozen belt already carries this floor). A tiny epsilon absorbs
/// f32 rounding without admitting a truly-absent good.
pub(crate) const PROV_GOOD_ABSENT_BELT: f32 = 8.0 / 255.0 + 1e-6;
/// Woodland cleared per year at full population pressure (fraction of the province).
const PROV_CLEAR_RATE: f32 = 0.0035;
/// Woodland regrowing per year on land nobody is working.
const PROV_REGROW_RATE: f32 = 0.0022;
/// Soil lost per year at full cropping intensity on unimproved land.
const PROV_DEPLETE: f32 = 0.010;
/// Soil recovered per year on fallow/lightly worked land.
const PROV_RECOVER: f32 = 0.006;
/// Floor soil condition can fall to — exhausted, not dead.
const PROV_SOIL_FLOOR: f32 = 0.25;
/// Grain-eq a unit of rural population yields per year on ORDINARY land (the land
/// multiplier is centred on 1.0, so this is the typical figure, not a ceiling).
const PROV_YIELD_PER_HEAD: f32 = 0.55;
/// The arable share that counts as "ordinary" — the land multiplier's unit point.
const PROV_ARABLE_REFERENCE: f32 = 0.28;
/// Of that, what the countryside eats itself before anything reaches a city.
const PROV_SUBSISTENCE: f32 = 0.42;
/// Irrigation's multiplier on the irrigated share's yield.
const PROV_IRRIGATION_GAIN: f32 = 0.45;
/// Highest rural tax rate a holder may set.
pub const PROV_TAX_MAX: f32 = 0.35;
/// Default rate a newly seeded province is taxed at.
const PROV_TAX_DEFAULT: f32 = 0.12;
/// Unrest added per year per unit of (tax above the tolerated level).
const PROV_UNREST_TAX: f32 = 0.9;
/// Unrest added per year by crowding once rural population passes capacity.
const PROV_UNREST_CROWD: f32 = 0.35;
/// Unrest added per year by a failed harvest (surplus below subsistence).
const PROV_UNREST_DEARTH: f32 = 0.30;
/// Yearly unrest decay when nothing is wrong.
const PROV_UNREST_CALM: f32 = 0.16;
/// A revolt breaks out above this.
const PROV_REVOLT_AT: f32 = 0.72;
/// Share of a revolting province's dues that simply never arrive.
const PROV_REVOLT_LOSS: f32 = 0.6;
/// Tax the countryside tolerates before it resents it.
const PROV_TAX_TOLERATED: f32 = 0.15;
/// Yearly samples kept per province (500 years at 1/yr is fine; cap anyway).
const PROV_HISTORY_CAP: usize = 600;
const PROV_EVENTS_CAP: usize = 40;

/// Phase 5 (`HOUSE_INHERITANCE_AND_TERRITORY.md` Part D) · the Stato da Mar case — a
/// house may be GRANTED an ungoverned-by-house province if it already holds the seat
/// city's BAILO (the strongest reach a house has anywhere), is Tier 1-2, and the
/// province is not currently in open revolt. Kept deliberately narrow: only a
/// house already dominant in the seat can plausibly be granted its hinterland.
const PROV_GRANT_TIER_MAX: u8 = 2;
const PROV_GRANT_UNREST_MAX: f32 = 0.40;
/// Yearly chance, once eligible, that the grant actually happens — so it reads as an
/// event with a date, not an instant the moment eligibility is reached.
const PROV_GRANT_CHANCE: f32 = 0.15;

// ── §2.5 goods exploitation tuning ──────────────────────────────────────────────
/// Depletion added per year at full over-exploitation pressure (exploitation 2.0×
/// potential), before the per-estate-kind rate multiplier. Same SHAPE as
/// `PROV_DEPLETE`, its own constant because the two erode different things.
const PROV_GOOD_DEPLETE: f32 = 0.05;
/// Depletion recovered per year once pressure eases, before the kind multiplier.
const PROV_GOOD_RECOVER: f32 = 0.03;
/// Depletion can erode potential by at most this much — a hard-worked good never
/// drops to literally zero (a soft cap that bites, never a hard stop, per §1.2).
const PROV_GOOD_DEPLETION_CAP: f32 = 0.75;

/// Kinds of multi-year land improvement. Deliberately mirrors the satellite-construction
/// vocabulary (stage → progress → supply) rather than inventing a second project system.
pub const WORK_CLEAR: u8 = 0;      // woodland → arable
pub const WORK_DRAIN: u8 = 1;      // waste/marsh → arable, and a fever-risk cut
pub const WORK_IRRIGATE: u8 = 2;   // raises the irrigated share
pub const WORK_ROAD: u8 = 3;       // a made road to the seat — cheaper dues, less arrears
pub const WORK_KINDS: [&str; 4] = ["clearance", "drainage", "irrigation", "road"];
/// Years of funded work each kind takes.
pub const WORK_YEARS: [f32; 4] = [6.0, 10.0, 8.0, 5.0];
/// Yearly cost (grain-eq) drawn from the funding treasury, per kind.
pub const WORK_COST: [f32; 4] = [40.0, 70.0, 55.0, 45.0];

/// Province works v2.0 · these four kinds used to be startable ONLY by the player
/// (`campaign_start_province_work` — one of just four mutating campaign verbs), so
/// a campaign nobody was micromanaging never improved a single province's land, on
/// any world, ever. `maybe_fund_province_works` gives every province the same
/// opportunity a player had, through the identical `ProvWork`/`advance_province_works`
/// funded-or-stalls machinery — this only decides WHETHER one begins, never how it
/// progresses. Per-province yearly roll, so on average a qualifying province waits
/// `1/PROV_WORK_AUTO_CHANCE` years for its holder to act.
///
/// Two eligibility/funding TIERS (maintainer's design, not the plan's original
/// city-or-house choice): outside a realm, a province may only improve once its own
/// seat city is advanced enough to administer the project (`hub.tier > 0`) and pays
/// from that city's treasury; under a realm's sovereignty it gets FULL capability
/// regardless of the seat's own tier, and the REALM's treasury pays instead — a
/// crown administers its own land whether or not any one of its cities has grown.
const PROV_WORK_AUTO_CHANCE: f32 = 0.12;
/// Arrears above which a road becomes worth building on its own (dues are visibly
/// leaking into arrears).
const PROV_WORK_ROAD_ARREARS: f32 = 6.0;
/// Rural unrest above which a road (which eases unrest 0.08 on completion) becomes
/// worth building even with no arrears yet.
const PROV_WORK_ROAD_UNREST: f32 = 0.30;
/// A funder must hold this many times a work's (size/terrain-scaled) yearly cost
/// before starting one autonomously — headroom so the work doesn't stall the moment
/// it begins, unlike the player verb's bare `>= cost` (a player can watch and
/// refund; the AI cannot).
const PROV_WORK_AUTO_HUB_MULT: f32 = 2.0;
const PROV_WORK_AUTO_REALM_MULT: f32 = 2.0;

/// The province SIZE at which `work_cost`'s area multiplier is exactly 1.0 — a
/// "typical" province, so a much bigger one costs proportionally more to improve
/// and a tiny one proportionally less.
const WORK_AREA_REFERENCE_KM2: f32 = 9000.0;
/// The RELIEF (max−min elevation, m) at which `work_cost`'s terrain multiplier
/// starts to bite meaningfully — broken, mountainous country.
const WORK_RELIEF_REFERENCE_M: f32 = 1200.0;
/// How strongly each work kind's cost responds to terrain roughness — order matches
/// `WORK_CLEAR`/`WORK_DRAIN`/`WORK_IRRIGATE`/`WORK_ROAD`. A road is carved through
/// the relief itself (highest); clearing/draining are harder on a slope but not
/// defined by it (moderate); an irrigation channel follows the easiest contour it
/// can find (lowest — it avoids roughness rather than fighting it).
const WORK_ROUGHNESS_WEIGHT: [f32; 4] = [0.9, 0.8, 0.5, 1.3];

/// A land improvement under way in a province. Funded yearly out of the funder's
/// treasury (a polis), wealth (a house), or — v2.0, when the province lies inside a
/// realm — the CROWN's own treasury (rule 27: sovereignty is a real, independent
/// funding source, not just a tax destination). Starved work stalls rather than
/// failing, whichever funder is set.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProvWork {
    pub province: u32,
    /// `WORK_*`.
    pub kind: u8,
    /// 0..1 completion.
    pub progress: f32,
    /// Hub whose treasury pays (−1 = unfunded, so it stalls).
    pub funder_hub: i32,
    /// House paying instead, or −1.
    pub funder_house: i32,
    /// Appended v2.0 · the realm paying instead (index into `CampaignSim::realms`),
    /// or −1. A province under a realm's sovereignty (`prov_realm >= 0`) is funded
    /// this way in preference to its seat city — the crown administers its own land.
    #[serde(default = "neg_one_i32")] pub funder_realm: i32,
    pub start_tick: u32,
    /// Consecutive years the work went unpaid (decays progress past 2).
    pub idle_years: u32,
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

    /// DEPOSITS_AND_MINING_PLAN.md slice 4 (D7) · the DEPTH + EXTENT of the real
    /// working of `good` nearest (x, y), within `MINE_DEPOSIT_SEARCH_KM` —
    /// `(DEPTH_SURFACE, unknown_extent())` (ungated) if this world carries no
    /// positional deposit data at all, or none of that mineral within reach.
    /// Called once, when a Mine/Quarry estate is founded (`create_estate`);
    /// never re-queried afterward, so a later change to `mine_deposits` (there
    /// is none — it's a one-way worldgen snapshot) could not retroactively
    /// regate an existing estate.
    pub(crate) fn mine_geology_at(&self, good: &str, x: f32, y: f32) -> (u8, u8) {
        if self.mine_deposits.is_empty() {
            return (crate::sim::deposits::DEPTH_SURFACE, unknown_extent());
        }
        let ww = self.world_w.max(1.0);
        let reach = MINE_DEPOSIT_SEARCH_KM * ww / EARTH_EQUATOR_KM;
        let mut best: Option<(f32, u8, u8)> = None;
        for d in &self.mine_deposits {
            if !d.good.eq_ignore_ascii_case(good) { continue; }
            let mut dx = (d.x - x).abs();
            if ww > 1.0 { dx = dx.min(ww - dx); }
            let dy = d.y - y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > reach { continue; }
            if best.map(|(bd, ..)| dist < bd).unwrap_or(true) { best = Some((dist, d.depth, d.extent)); }
        }
        best.map(|(_, depth, extent)| (depth, extent))
            .unwrap_or((crate::sim::deposits::DEPTH_SURFACE, unknown_extent()))
    }

    /// Thin wrapper over `mine_geology_at` for callers that only need the depth.
    #[cfg(test)]
    pub(crate) fn mine_depth_at(&self, good: &str, x: f32, y: f32) -> u8 {
        self.mine_geology_at(good, x, y).0
    }

    /// DEPOSITS_AND_MINING_PLAN.md slice 5 · a settlement's TRADE CATCHMENT
    /// radius, growing slowly across the campaign — "a province view disc that
    /// visibly grows across the year slider". A pure DERIVED read, never
    /// per-hub stored state: `base` mirrors the world-side 50–120 km curve
    /// (`economy.rs`'s own catchment radius, by founding population — the same
    /// curve, so a fresh colony reads identically to how the world-gen catchment
    /// would have scored it), and `grown` is a function of TIME ALONE since
    /// founding, capped — never of live population (which can fall), matching
    /// the plan's "grow slowly" without needing a new mutable field threaded
    /// through every hub-construction site. No conflict with the one-way
    /// snapshot rule (§3.4): this never re-attributes production, only display.
    pub(crate) fn catchment_radius_km(&self, h: usize) -> f32 {
        let hub = &self.hubs[h];
        let pop = hub.founding_pop.max(1.0);
        let t = (pop.ln() - 6.2) / (11.5 - 6.2);
        let base = 50.0 + t.clamp(0.0, 1.0) * (120.0 - 50.0);
        let years = (self.tick.saturating_sub(hub.founded_tick) as f32 / TICKS_PER_YEAR as f32).max(0.0);
        let grown = (CATCHMENT_GROWTH_PER_YEAR_KM * years).min(CATCHMENT_MAX_GROWTH_KM);
        base + grown
    }

    /// DEPOSITS_AND_MINING_PLAN.md slice 4 · mercury amalgamation, wired as a
    /// CONSUMABLE EXTRACTION INPUT (not a manufacturing recipe — silver is dug,
    /// not assembled from parts, so it never touches `manufacture.rs`). Mercury
    /// (slice 3) shipped with correct geology and high value but no recipe
    /// wiring; this is that wiring. Once a day, after ordinary extraction has
    /// booked each mine's ore output: a silver mine that can draw mercury from
    /// its OWN stock (the amalgamation process itself, Potosí's from 1554) works
    /// its ore more completely; one with none on hand still smelts by hand, at a
    /// real but lower recovery. `served` (0 = no mercury, 1 = fully supplied)
    /// interpolates between `MERCURY_AMALGAMATION_FLOOR` and `_BONUS`, and the
    /// stock ledger is adjusted by the DELTA only — ordinary production already
    /// booked the unmodified `realized` amount, so this never double-counts.
    /// A no-op wherever this world has no silver good, no mercury good, or no
    /// silver-mine estate at all (every early-return leaves state untouched).
    pub(crate) fn apply_mercury_amalgamation(&mut self) {
        let Some(silver_g) = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("silver")) else { return };
        let Some(mercury_g) = self.goods.iter().position(|g| g.name.eq_ignore_ascii_case("mercury")) else { return };
        for h in 0..self.hubs.len() {
            if !self.hubs[h].is_estate || self.hubs[h].estate_kind != 2 { continue; }
            let out = self.hubs[h].production.get(silver_g).copied().unwrap_or(0.0);
            if out <= EPS { continue; }
            let needed = out * MERCURY_PER_SILVER;
            let taken = stock_take(&mut self.hubs[h].stock, mercury_g, needed);
            let served = if needed > EPS { (taken / needed).clamp(0.0, 1.0) } else { 1.0 };
            let mult = MERCURY_AMALGAMATION_FLOOR
                + (MERCURY_AMALGAMATION_BONUS - MERCURY_AMALGAMATION_FLOOR) * served;
            let new_out = out * mult;
            let delta = new_out - out;
            self.hubs[h].production[silver_g] = new_out;
            let band = production_band(true, self.hubs[h].quality.get(silver_g).copied().unwrap_or(0.0));
            if delta >= 0.0 {
                stock_add(&mut self.hubs[h].stock, silver_g, band, delta);
            } else {
                stock_take(&mut self.hubs[h].stock, silver_g, -delta);
            }
        }
    }

    /// N5 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §1.2) — which seasonal slice
    /// `base_days_season` is in effect RIGHT NOW.
    #[inline]
    fn season_slice_now(&self) -> usize {
        (self.day_of_year() as usize * self.season_slices as usize) / TICKS_PER_YEAR as usize
    }

    /// N5 §1.2 — the stored multiplier for lane `a→b` in slice `s`, or exactly
    /// 1.0 when nothing was ever quantised there (an old save, or a hub pair
    /// added after `base_n`, both index out of range).
    #[inline]
    fn season_mult(&self, a: usize, b: usize, s: usize) -> f32 {
        if self.base_n == 0 || a >= self.base_n || b >= self.base_n { return 1.0; }
        let idx = s * self.base_n * self.base_n + a * self.base_n + b;
        match self.base_days_season.get(idx) {
            Some(&v) => (1.0 + v as f32 * SEASON_MULT_STEP).min(SEASON_MAX_MULT),
            None => 1.0,
        }
    }

    /// N5 §1.3 — travel days for lane `a→b` RIGHT NOW: the annual mean
    /// (`base_days`/`days` — unchanged, so any caller not yet converted to
    /// this accessor keeps reading exactly what it reads today) times this
    /// lane's seasonal multiplier for the current slice. `season_slices == 0`
    /// (no seasonal data, e.g. an old save) is a true no-op.
    #[inline]
    pub(crate) fn lane_days(&self, a: usize, b: usize) -> f32 {
        let n = self.hubs.len();
        if a >= n || b >= n { return f32::INFINITY; }
        let d = self.days[a * n + b];
        if self.season_slices == 0 || !d.is_finite() { return d; }
        d * self.season_mult(a, b, self.season_slice_now())
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
            // R1b · the clock a proclamation's "held ≥ 10 years continuously" reads.
            // Reset on ANY change — including losing the seat and later retaking it —
            // so a house cannot chain two half-tenures into one long one.
            if captor != prev { self.hubs[h].captor_since = tick; }
            // 5) Payoff on a fresh capture: a favoured-house charter + a trade-influence
            //    boost (its policy tilt is applied in `decide_polis_policy`).
            if captor >= 0 && captor != prev {
                self.push_law(h, 0, captor, -1, year);
                // 4.9 (A4) · the captor occasionally shuts the door behind it.
                if hash01(self.seed, tick as u64 ^ 0x4A57B, h as u64) < FOREIGN_BAR_ON_CAPTURE_CHANCE {
                    self.push_law(h, LAW_FOREIGN_BAR, captor, -1, year);
                }
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
                    stock_add_ungraded(&mut self.hubs[h].stock, fg, rel);
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
    /// TECTONICS_AND_ISOLATION_PLAN.md Part A — an ocean is a real barrier.
    ///
    /// This used to fold ANY component with fewer than 3 real hubs into the
    /// nearest "big" (≥3-hub) component with NO DISTANCE LIMIT — `bd` started at
    /// `f32::INFINITY`, so a two-hub mid-ocean island was relabelled as part of a
    /// continent thousands of km away. `#6`'s own same-component partner
    /// guarantee (`rebuild_routes`) then drew straight-line trans-oceanic trade
    /// lanes between them — exactly the "dishonest arrow between two separate
    /// continents" its own comment says it exists to prevent. The guard was never
    /// wrong; it was being handed a lie about which cells were connected.
    ///
    /// `ISOLATION_RESCUE_MAX_KM` bounds the rescue to a plausible regional sea
    /// crossing (stated in km, converted per world — rule 25). Beyond it, a tiny
    /// component is left on its own: it trades only among its own hubs (if it has
    /// more than one) via the ordinary same-component passes below, and a city
    /// that cannot obtain what it needs starves and is abandoned exactly as any
    /// other shortage does (`abandon_hub` already records `died_cause`) — the
    /// user's own stated design: "if there are not enough goods to sustain the
    /// civilisation, the city becomes dead." No separate "lifeline" exception is
    /// added for a lone-hub island: a single city with no reachable partner at
    /// all is the honest reading of true isolation, and its survival or death is
    /// the economy's answer, not a router special case.
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
        // Cap in CELLS² (avoids a sqrt per candidate pair): a cell is
        // `EARTH_EQUATOR_KM / world_w` km wide, so `cap_km * world_w /
        // EARTH_EQUATOR_KM` is the same cap in cells — the identical conversion
        // every other km-stated reach in this file already uses.
        let cap_cells = ISOLATION_RESCUE_MAX_KM * world_w.max(1.0) / EARTH_EQUATOR_KM;
        let cap_cells2 = cap_cells * cap_cells;
        let mut reassign: Vec<(usize, u32)> = Vec::new();
        for &i in &real {
            if size.get(&self.hubs[i].component).copied().unwrap_or(0) >= 3 { continue; }
            let mut bj = None; let mut bd = f32::INFINITY;
            for &j in &big { let d = d2(i, j, &self.hubs); if d < bd { bd = d; bj = Some(j); } }
            if let (Some(j), true) = (bj, bd <= cap_cells2) { reassign.push((i, self.hubs[j].component)); }
        }
        for (i, comp) in reassign { self.hubs[i].component = comp; }
        // Estates ride their parent hub's (possibly new) component. A REMOTE trade
        // outpost (`parent < 0` by design — CLAUDE.md rule 32 / WORLD_AND_TRADE_
        // MASTER_PLAN.md Part II G3) has no parent to ride, so it falls back to its
        // `founder_hub` instead; before this it was never re-synced by this pass at
        // all, a live trap for any future reassignment.
        for i in 0..n {
            if self.hubs[i].is_estate {
                let p = self.hubs[i].parent;
                if p >= 0 && (p as usize) < n {
                    self.hubs[i].component = self.hubs[p as usize].component;
                } else {
                    let f = self.hubs[i].founder_hub;
                    if f >= 0 && (f as usize) < n { self.hubs[i].component = self.hubs[f as usize].component; }
                }
            }
        }
    }

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
                // Rank candidates by EFFECTIVE distance (real days ÷ the partner's trade
                // gravity), so a big entrepôt far away out-ranks a small town nearby and
                // makes the partner list even of distant cities. Freight elsewhere still
                // uses the REAL `days`, so only WHO trades with whom changes, not the cost.
                if d.is_finite() { scratch.push((b as u32, d / self.hub_pull(b))); }
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
        // Local catchment: guarantee every hub also has its CATCHMENT_K nearest
        // physical neighbours (ignoring hub_pull) so small towns near a large hub
        // aren't crowded out of its dispatch targets by effective-distance ranking.
        // Without this, large entrepôts only ship to other large hubs and small
        // coastal towns like a harbour near a metropolis get zero throughput.
        const CATCHMENT_K: usize = 8;
        let mut catch_scratch: Vec<(u32, f32)> = Vec::with_capacity(n);
        for a in 0..n {
            if is_bound(a) { continue; }
            catch_scratch.clear();
            for b in 0..n {
                if b == a { continue; }
                if is_bound(b) && self.hubs[b].founder_hub as usize != a { continue; }
                let d = self.days[a * n + b];
                if d.is_finite() { catch_scratch.push((b as u32, d)); }
            }
            if catch_scratch.len() > CATCHMENT_K {
                catch_scratch.select_nth_unstable_by(CATCHMENT_K,
                    |x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
                catch_scratch.truncate(CATCHMENT_K);
            }
            for &(b, _) in &catch_scratch {
                if !neighbors[a].contains(&b) {
                    neighbors[a].push(b);
                }
            }
        }
        self.neighbors = neighbors;
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
        // N6 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §2) — the STRUCTURAL
        // twin of `needs`: what people NEED, never discounted by price. Every
        // welfare reader (lack_basic/comfort/luxury, hence
        // `decide_crisis_relief`) reads this; only price-setting and dispatch
        // read the elastic `needs`. At `DEMAND_ELASTICITY == [0,0,0]` the two
        // are built identically and stay element-wise equal (the no-op gate).
        let mut needs_struct: Vec<Vec<f32>> = Vec::new();
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
            self.dev_tier.resize(self.hubs.len(), 0);
            self.dev_momentum.resize(self.hubs.len(), 0);
            // Twice a year, re-rank hubs into commercial classes (trade hub / entrepôt)
            // from the trade that has actually flowed this period (user: entrepôts
            // change once per half-year). Hysteresis inside makes status earned/lost.
            if tick > 0 && tick % (TICKS_PER_YEAR / 2) == 0 {
                self.classify_hubs();
                self.classify_development();
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
                    // Prune the CHATTER — feud flares, lost caravans, warehouse fires —
                    // oldest first. The family's MILESTONES are the record itself and are
                    // never evicted by noise: before this, a house in a hot feud lost its
                    // own founding and every succession inside a couple of years, so a
                    // 500-year dynasty's chronicle read as three weeks of shipping
                    // losses. (It also silently zeroed the Phase 0.4 division metric.)
                    let ev = &mut self.houses[hi].events;
                    if ev.len() > HOUSE_EVENTS_CAP {
                        let mut over = ev.len() - HOUSE_EVENTS_CAP;
                        ev.retain(|e| {
                            if over == 0 || is_house_milestone(&e.kind) { return true; }
                            over -= 1;
                            false
                        });
                        if ev.len() > HOUSE_MILESTONE_CAP {
                            let drop = ev.len() - HOUSE_MILESTONE_CAP;
                            ev.drain(0..drop);
                        }
                    }
                    // Phase 3.1 · check standing ambitions, then take up a new one if
                    // a slot is free. Order matters: a goal that just succeeded/failed
                    // frees its slot the same year a new one can be chosen.
                    self.update_house_goal(hi);
                    self.choose_house_goal(hi);
                }
                self.house_ledger_prev = self.house_ledger.clone();
                let yr = tick / TICKS_PER_YEAR;
                // DLC 3 · the polis council sets the coming year's tariff / mint
                // policy, then the speculation why-engine reads the year that just
                // closed (uses `house_ledger_prev` before the books are reset).
                self.run_polis_policy(yr);
                // Government: seed regimes/key figures, bribery & intimidation, capture,
                // regime change, civic granary (reads the influence/dominance just set).
                self.update_government(yr);
                // DLC 3.5 · the council then sets its coinage (named coin + trust +
                // seigniorage), banking houses charter/grow banks, the speculation
                // engine reads the closed year, and HIGH-tier bubbles may POP into a
                // regional crash.
                self.run_coinage(yr);
                // v2.0 · a council whose coin has failed may REFORM it (call-in + re-mint).
                self.maybe_reform_coinage(yr);
                self.update_currency_baskets();
                self.update_banks(yr);
                self.update_wars(yr);
                // B3 · civic public debt (Monte): service coupons, default if over-levered,
                // and issue fresh bonds where the treasury is short (post-war financing).
                self.update_public_debt(yr);
                self.maybe_steal_quality(yr);
                self.compute_speculation(yr);
                // Fold the year's trade flows into the Flows-subtab detail + trend graphs.
                self.fold_trade_year();
                self.maybe_pop_bubbles(yr);
                // A3 · snapshot each coin's yearly state (after crashes settle) for the
                // Money panel's coin-biography sparklines.
                self.snapshot_coins(yr);
                self.roll_city_finances(yr);
                // Phase 4 (flavour) · raise/retire notable figures (Great Lives).
                self.raise_notable_figures(yr);
                // Feuds · a council both houses trade in may impose a settlement on a
                // long-running quarrel. Runs BEFORE marriages, so a feud the council
                // settled this year is not also "sealed by marriage" in the same year.
                self.arbitrate_feuds(yr);
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
                let supply_class = self.hub_supply_class(h);
                if self.hubs[h].supply_accum.len() != ng * SUPPLY_CLASSES {
                    self.hubs[h].supply_accum.resize(ng * SUPPLY_CLASSES, 0.0);
                }
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
                    let band = production_band(self.hubs[h].is_estate, self.hubs[h].quality.get(g).copied().unwrap_or(0.0));
                    stock_add(&mut self.hubs[h].stock, g, band, realized);
                    supply_add(&mut self.hubs[h].supply_accum, g, supply_class, realized);
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
                            let band = production_band(self.hubs[h].is_estate, self.hubs[h].quality.get(g).copied().unwrap_or(0.0));
                            stock_add(&mut self.hubs[h].stock, g, band, add);
                            supply_add(&mut self.hubs[h].supply_accum, g, SUPPLY_CITY, add);
                        }
                    }
                }
            }

            // DEPOSITS_AND_MINING_PLAN.md slice 4 · mercury amalgamation — a
            // consumable EXTRACTION input, applied once ordinary per-capita
            // extraction has booked every mine's ore output for the day.
            self.apply_mercury_amalgamation();

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
            needs_struct.resize(n, Vec::new());
            for row in needs_struct.iter_mut() {
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
                // DETERMINISM: `hub_desire` is read as a Vec downstream, so building it
                // in HashMap order makes its order vary run to run. Sort by good index.
                let mut bs: Vec<(usize, f32)> = boost.into_iter()
                    .map(|(gi, b)| (gi, b.min(CULTURE_DESIRE_MAX))).collect();
                bs.sort_by_key(|&(gi, _)| gi);
                hub_desire[h] = bs;
            }
            for h in 0..n {
                for g in 0..ng {
                    let b = self.base_need(h, g);
                    needs[h][g] = b;
                    needs_struct[h][g] = b;
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
                    // N6 (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §2.3) · own-price
                    // elasticity of the AGGREGATE, using the LAGGED (EMA) price —
                    // the same `rel` vocabulary substitution already uses one line
                    // above, read a tick stale, which is what turns this into a
                    // damped response instead of a simultaneous equation. Applied
                    // OUTSIDE `base_need`, so `needs_struct` (below) stays exactly
                    // today's structural aggregate — the ration, not the market.
                    let tier = members.first().map(|&g| self.goods[g].need_tier.min(2) as usize).unwrap_or(0);
                    let agg_price: f32 = members.iter().map(|&g| self.hubs[h].price[g]).sum::<f32>()
                        / members.len().max(1) as f32;
                    let agg_base: f32 = members.iter().map(|&g| self.goods[g].base_value).sum::<f32>()
                        / members.len().max(1) as f32;
                    let rel = (agg_price / agg_base.max(EPS)).max(PRICE_FLOOR_MULT);
                    let elastic_total = total * elastic_aggregate_mult(tier, rel);
                    for (mi, &g) in members.iter().enumerate() {
                        needs_struct[h][g] = total * weights[mi] / wsum;
                        needs[h][g] = elastic_total * weights[mi] / wsum;
                    }
                }
                // Phase 5 (flavour) · a good in vogue is consumed more keenly
                // (post-substitution, so fashion draws real extra demand → price rises).
                // Mirrored onto `needs_struct` too — a taste/fashion shift changes
                // what people want, not how they respond to price, so it belongs on
                // both the elastic and the structural aggregate alike.
                if any_fashion {
                    for g in 0..ng {
                        if fashion_mult[g] != 1.0 {
                            needs[h][g] *= fashion_mult[g];
                            needs_struct[h][g] *= fashion_mult[g];
                        }
                    }
                }
                // Cultural taste: the resident peoples' prized goods pull extra demand.
                for &(gi, b) in &hub_desire[h] {
                    needs[h][gi] *= 1.0 + b;
                    needs_struct[h][gi] *= 1.0 + b;
                }
                // Eat down stock; track unmet demand per need-tier for the
                // "% population lacking goods" graph (basic / comfort / luxury).
                // N6 · reads `needs_struct` (the ration), never the elastic
                // `needs` — "elasticity belongs to the market, not the ration"
                // (§2.2). `lack_basic`/`lack_comfort`/`lack_luxury`, and so
                // `decide_crisis_relief`, all trace back to this loop.
                let mut tier_need = [0.0f32; 3];
                let mut tier_unmet = [0.0f32; 3];
                for g in 0..ng {
                    let need = needs_struct[h][g];
                    let eat = need.min(stock_of(&self.hubs[h].stock, g));
                    stock_take(&mut self.hubs[h].stock, g, eat);
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
                    let target = self.live_price(stock_of(&self.hubs[h].stock, g), needs[h][g], base);
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
            //    Supplier attribution (§4.4) decays the same way.
            for hb in &mut self.hubs {
                hb.in_by_sea *= 0.98;
                hb.in_by_land *= 0.98;
                for v in hb.supply_accum.iter_mut() { *v *= 0.98; }
            }
            // (to, good, amount, sea, phase, home, owner, local, via, price) — phase/
            // home/owner let an arriving OUTBOUND house cargo spawn its return leg
            // from the dest hub; `local` (N8) says whether an ownerless arrival was a
            // short haul; `via` (TRADE_STAGING_AND_POSTS_PLAN.md slice 4) names a
            // REAL destination beyond this arrival when the leg that just landed was
            // only the first hop of a composed entrepôt route.
            let mut landed: Vec<(usize, usize, f32, bool, u8, i32, i32, bool, i32, f32)> = Vec::new();
            self.in_transit.retain(|c| {
                if c.eta_tick <= tick {
                    landed.push((c.to as usize, c.good, c.amount, c.sea, c.phase, c.home, c.owner, c.local, c.via, c.price));
                    false
                } else {
                    true
                }
            });
            for (to, g, amt, sea, phase, home, owner, local, via, price) in landed {
                // TRADE_STAGING_AND_POSTS_PLAN.md slice 4 — a leg composed through
                // an entrepôt outlet (`via >= 0`) does NOT take delivery here: the
                // buyer at the real destination already settled this trade at
                // dispatch time (§6d's composed price), so the cargo simply
                // re-embarks for the second, real leg to `via`. This is what makes
                // a long lane an actual RELAY — two real legs with real travel
                // time — instead of one leg whose price alone pretended a stop
                // happened. Full break-of-bulk (the outlet selling the cargo
                // itself instead of forwarding it) is deliberately NOT built here:
                // the economics were already settled for delivery to the ORIGINAL
                // buyer at dispatch time, so diverting the cargo at the stop would
                // pay the seller twice — once via the arbitrage profit already
                // credited, once via the outlet's own local sale. Making the stop a
                // genuine economic choice needs the settlement itself to move to
                // arrival time, which is real future work, not silently skipped.
                if via >= 0 && to < self.hubs.len() && (via as usize) < self.hubs.len() {
                    let b = via as usize;
                    let d2 = self.lane_days(to, b);
                    let eta2 = if d2.is_finite() { tick + (d2.ceil() as u32).max(1) } else { tick + 1 };
                    let sea2 = self.hubs[to].coastal && self.hubs[b].coastal;
                    let river2 = !sea2 && self.hubs[to].river && self.hubs[b].river;
                    self.in_transit.push(InTransit {
                        from: to as u32, to: b as u32, good: g, amount: amt,
                        eta_tick: eta2, owner, sea: sea2, river: river2,
                        // The round-trip bonus leg belongs to a direct voyage only
                        // (§1 above) — a relayed voyage's vessel does not sail home.
                        phase, home: -1, contract: false, price, local, via: -1,
                    });
                    continue;
                }
                if to < self.hubs.len() {
                    // W2, dose-walked (`LANDED_CARGO_TO_DEPOT_DOSE`) · a slice of a
                    // house-owned arrival goes straight into that carrier's own
                    // depot at `to` (room permitting) instead of the pool (F8).
                    // Zero dose today — `landed_cargo_to_depot` returns 0 and the
                    // full amount lands in the pool exactly as before.
                    let diverted = self.landed_cargo_to_depot(to, g, amt, owner);
                    stock_add_ungraded(&mut self.hubs[to].stock, g, amt - diverted);
                    if self.hubs[to].supply_accum.len() != ng * SUPPLY_CLASSES {
                        self.hubs[to].supply_accum.resize(ng * SUPPLY_CLASSES, 0.0);
                    }
                    // N8: attribute the arrival to its actual carrier instead of
                    // always booking SUPPLY_FOREIGN — a house-owned voyage books
                    // SUPPLY_HOUSE, an ownerless short haul SUPPLY_LOCAL, and only an
                    // ownerless long haul SUPPLY_FOREIGN.
                    let sclass = if owner >= 0 { SUPPLY_HOUSE }
                        else if local { SUPPLY_LOCAL }
                        else { SUPPLY_FOREIGN };
                    supply_add(&mut self.hubs[to].supply_accum, g, sclass, amt);
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
            // N6 §2.2 — food balance/starvation reads the STRUCTURAL need, never
            // the price-elastic one: a population must not read as fed merely
            // because dear grain "discouraged" its demand.
            self.update_food_and_starvation(&needs_struct);

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
                // …and, in a dearth, open that same granary again and bar food exports.
                // Runs AFTER provisioning on purpose: a council in famine finds nothing
                // left on its own market to pre-empt, so the two cannot chase each other.
                self.run_crisis_relief();
                self.run_trade_bans(); // N2 — the same reflex, generalised to any good
                self.warehouse_and_spoilage_pass(); // size city warehouses, spoil what rots (§4.2)
                self.works_monthly_pass(); // each estate's 12-month output/quality/price ring (§4.6)
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
            // R1b · a crowned house has left the merchant world — its estates and
            // fleets no longer draw upkeep against its own (now zeroed) wealth. What
            // funds a realm's inherited holdings is a REALM-treasury concern, not
            // built yet (`REALM_AND_GOVERNMENT_PLAN.md` R3); excluding it here is what
            // keeps a coronation from being followed by a manufactured bankruptcy.
            if !self.houses[hi].is_merchant() {
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

            // ── Stewards (Phase 2.5) — a HIRED factor is able, and skims ──────────
            // Every holding without a POSTED kin running it (offices + owned estates,
            // less however many kin are actually posted) costs a small wage, plus a
            // small skim on positive wealth capped to a few holdings' worth. Guilds
            // are civic offices, not families with stewards to hire. Gated on a
            // NON-EMPTY roster: an old save (or a house that hasn't succeeded since
            // Phase 2.1 shipped) has no roster and so no information about who's
            // hired — treating that as "assume everything is hired" would invent a
            // fact the model doesn't have and silently break the "no roster ⇒
            // bit-identical" backward-compatibility invariant every earlier Kin
            // sub-phase relies on. No roster reads as "nothing is known", not
            // "everything is hired".
            let mut steward_cost = 0.0f32;
            if !is_guild && !self.houses[hi].kin.is_empty() {
                let total_holdings = self.houses[hi].offices.len() as f32 + est_count[hi] as f32;
                let posted = self.houses[hi].kin.iter().filter(|k| k.role == 2).count() as f32;
                let hired = (total_holdings - posted).max(0.0);
                if hired > 0.0 {
                    steward_cost = hired * STEWARD_WAGE
                        + self.houses[hi].wealth.max(0.0) * hired.min(STEWARD_SKIM_HOLDINGS_CAP) * STEWARD_SKIM_RATE;
                }
            }

            // ── Conspicuous consumption (spent INTO the home city's people) ──
            // Only a slice of POSITIVE wealth — a house in debt buys no feasts.
            // Phase 2.4 · a CIVIC-minded head spends more of it into the city (which is
            // what fuels `fund_public_works`), a PRIVATE one hoards more — axis 2, ±15%
            // capped. Guilds are civic by construction already, so this only touches
            // private houses.
            let pos = self.houses[hi].wealth.max(0.0);
            let consume_rate = if is_guild { GUILD_CIVIC_RATE }
                else { HOUSE_CONSUMPTION_RATE * self.head_character_factor(hi, 2) };
            // Phase 3.2 · Lavish is the one vice this pass wires a direct wealth cost
            // to — a small extra bleed on top of the character-scaled rate above, so
            // the vice is a real (if small) annual cost rather than a label only.
            let vice_rate = if !is_guild && self.head_vice(hi) == VICE_LAVISH { VICE_LAVISH_DRAIN } else { 0.0 };
            let consumption = pos * (consume_rate + vice_rate);

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
                // Same city_size_factor scaling GUILD_WEALTH_SOFTCAP already uses above —
                // a house's ceiling grows with its own home city instead of assuming a
                // fixed economy size forever (see HOUSE_WEALTH_SOFTCAP's doc comment).
                let softcap = HOUSE_WEALTH_SOFTCAP * self.city_size_factor(home);
                let over = (pos - softcap).max(0.0);
                let raw = (pos * HOUSE_WEALTH_TAX_BASE + over * over * HOUSE_WEALTH_TAX_QUAD) * early_mult;
                raw.min(pos * HOUSE_WEALTH_TAX_MAXFRAC)
            };

            self.houses[hi].wealth -= upkeep + consumption + endowment + wealth_overhead + wealth_tax + steward_cost;

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

    /// Map a contract term (years) to its index into the TERM_* tables.
    fn term_index(years: u8) -> usize { TERM_YEARS.iter().position(|&y| y == years).unwrap_or(0) }

    /// The highest contract-term INDEX (into TERM_*) a house qualifies to offer:
    /// 1yr always · 3yr ≥4 stable yrs · 5yr ≥7 · 7yr >10.
    fn max_term_index(&self, hi: usize) -> usize {
        let y = self.stable_growth_years(hi);
        if y > 10 { 3 } else if y >= 7 { 2 } else if y >= 4 { 1 } else { 0 }
    }

    /// Monthly solvency check. A balance is allowed to go NEGATIVE (debt, shown
    /// in the Accountant); a PRIVATE house that stays in the red for a full year
    /// is declared bankrupt and dissolved. A GUILD never dissolves — its home city
    /// bails it out from the civic pool (and, failing that, simply carries the debt
    /// until its subsidy recovers it), because a city won't let its guild fail.
    fn update_solvency(&mut self) {
        let tick = self.tick;
        for hi in 0..self.houses.len() {
            // R1b · a crowned house is never bankrupted through this path — its
            // `wealth` was zeroed at the coronation (the pot moved whole to the
            // realm treasury) and staying at exactly 0 must never read as insolvency.
            // Routing a coronation into `dissolve_house` here would be the same trap
            // `REALM_AND_GOVERNMENT_PLAN.md` §5.1 already names for `GOAL_OUTLAST_
            // RIVAL` — a different call site, the same mistake.
            if !self.houses[hi].is_merchant() { continue; }
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

    /// Auto-pick the good that feeds one construction category (0 food · 1 preservables ·
    /// 2 construction) from the metropolis by locale: the most available good matching the
    /// category (name hints steer preservables→salt/dried and construction→timber/stone).
    fn pick_build_supply_good(&self, metro: usize, cat: u8) -> u16 {
        let ng = self.goods.len();
        if ng == 0 || metro >= self.hubs.len() { return 0; }
        let avail = |g: usize| stock_of(&self.hubs[metro].stock, g).max(0.0)
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
                let from_stock = stock_of(&self.hubs[m].stock, g)
                    .max(0.0).min(SAT_STAGE_QUOTA - delivered);
                if g < ng { stock_take(&mut self.hubs[m].stock, g, from_stock); }
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
                        let v = self.hubs[h].production[g];
                        stock_set_total(&mut self.hubs[h].stock, g, v);
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

    /// Find (or create) the attempt ledger for a city-pair; returns its index.
    fn prospect_idx(&mut self, a: usize, b: usize) -> usize {
        let (lo, hi) = ((a.min(b)) as u32, (a.max(b)) as u32);
        if let Some(i) = self.route_prospects.iter().position(|p| p.a == lo && p.b == hi) {
            return i;
        }
        self.route_prospects.push(RouteProspect { a: lo, b: hi, ..Default::default() });
        self.route_prospects.len() - 1
    }

    pub fn world_h(&self) -> u32 { (self.world_w * 0.5).max(1.0) as u32 }

    /// "Marcus Cassii"-style head name for `house_name` at `hub`, varied by `salt`.
    fn head_name_for(&self, hub: usize, house_name: &str, salt: u64) -> String {
        let surname = house_name.strip_prefix("House ").unwrap_or(house_name);
        let (x, y) = (self.hubs[hub].x.max(0.0) as u32, self.hubs[hub].y.max(0.0) as u32);
        crate::sim::names::gen_head_name(x, y, self.world_w as u32, self.world_h(), surname, salt)
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

// ── impl CampaignSim split across theme modules (methods live in child files) ──
mod money;
mod war;
mod disease;
mod colonies;
mod polis;
mod cities;
mod houses;
pub use houses::{kin_power_shares, character_phrase};
mod crisis;
mod schism;
mod foreign_hand;
mod production;
mod realms;
mod envoys;
mod offtake;
mod certification;
mod league;
mod yards;
pub(crate) use league::{
    LEAGUE_MIN_MEMBERS, LEAGUE_MAX_FOUNDING_MEMBERS, LEAGUE_YEAR_FLOOR, LEAGUE_FLOW_MIN,
    LEAGUE_DRIFT_YEARS, LEAGUE_DUES_FRAC, LEAGUE_DUES_MIN_TREASURY, LEAGUE_BOYCOTT_MAX,
    LEAGUE_BOYCOTT_TICKS,
};

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
            v += stock_of(&h.stock, g) * goods[g].base_value;
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
mod tests;

/// The economy fidelity gate — the campaign's counterpart to the Earth climate
/// scorecard. See `economy_validation.rs` for the method and the sources.
#[cfg(test)]
mod economy_validation;
