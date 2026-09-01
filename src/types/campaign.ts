// Split from the former monolithic src/types.ts. Mirrors Rust serde structs.

// ── DLC 1 "Living Trade" tick simulation ──
export interface CampaignClock {
  tick: number;
  year: number;
  day: number;
  season: string;
  last_tick_ms: number;
}
export interface CampaignHubBrief {
  id: number;
  x: number;
  y: number;
  name: string;
  population: number;
  grain_wealth: number;
  trade_wealth: number;
  starving: number;
  is_estate: boolean;
  mood: number;
  /** Month-over-month population growth fraction (+0.05 = +5%). */
  growth: number;
  /** Colony state for map markers: 0 none · 1 settlement colony · 2 house outpost. */
  colony_kind: number;
  colony_stage: number;
  /** Owner house index (house outposts) — map to the house colour. */
  owner_house: number;
  /** Founder/owner-home hub index (lane endpoint); -1 if none. */
  founder_hub: number;
  autonomous: boolean;
  /** Atlas 2.0 · the settlement is a dead ruin († marker, skipped by the sim). */
  abandoned: boolean;
  /** Tick founded mid-campaign (0 = primordial) — drives the "new town" badge. */
  founded_tick: number;
  /** Last full year's trade throughput (grain-eq, in+out) — Trade Heat overlay. */
  trade_volume: number;
  /** Dynamically-earned commercial class (re-ranked twice a year): 0 ordinary ·
   *  1 trade hub · 2 entrepôt. Drives the distinct map marker. */
  hub_class?: number;
  /** Satellite construction stage: 0 = finished/not building · 1..=5 = under construction. */
  build_stage?: number;
  /** Why the settlement died ("famine"/"plague"/"war"/"disaster"; "" = alive). */
  died_cause: string;
  /** Downsampled population history (≤30 points) — the census sparkline. */
  pop_spark: number[];
  /** CITY_PROVINCE_WAR_PLAN.md §3.2 city tier: 1 great · 2 major · 3 lesser ·
   *  4 marginal · 0 = not yet assigned. §3.3 reads tier 1-2 for state eligibility. */
  tier: number;
  /** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.11: 0 content · 1 short · 2 starving. */
  pop_status?: number;
}
/** TRADE_STAGING_AND_POSTS_PLAN.md Slice 7 — one Trading Posts roster row
 *  (campaign_get_trading_posts). */
export interface TradingPost {
  hub: number;
  name: string;
  x: number;
  y: number;
  /** 2 = resource outpost, 4 = route post. */
  motive: number;
  owner_house: string;
  population: number;
  /** hub_class: 0 ordinary · 1 trade hub · 2 entrepôt. */
  rung: number;
  transit_year: number;
  forgone_transit: number;
  writ_holder: string;
  graduated: boolean;
  decline_years: number;
  age_years: number;
  barred_houses: string[];
}
/** Atlas 2.0 · one named trade basin (campaign_get_trade_basins). */
export interface TradeBasin {
  name: string;
  volume: number;
  hub_ids: number[];
  pts: [number, number][];
  cx: number;
  cy: number;
  top_city: string;
  /** Batch 1 · the basin's top traded goods (≤2, by yearly volume). */
  top_goods: string[];
}
/** Batch 1 · Hall of Records — each entry is [value, holder, year]. */
export interface WorldRecords {
  largest_city: [number, string, number];
  richest_house: [number, string, number];
  biggest_trade_year: [number, string, number];
  deadliest_plague: [number, string, number];
  worst_crash: [number, string, number];
  longest_dynasty: [number, string, number];
  most_towns: [number, string, number];
}
/** Batch 1 · era scrubber: the world as it stood at the end of `year`. */
export interface EraHub {
  x: number;
  y: number;
  name: string;
  population: number;
  trade: number;
  dead: boolean;
  is_new: boolean;
}
export interface EraFrame {
  year: number;
  hubs: EraHub[];
}
/** #1/#23 · one people's world census, for the Peoples panel. */
export interface CultureBrief {
  name: string;
  color: [number, number, number];
  population: number;
  towns: number;      // settlements where this culture is the majority
  presence: number;   // settlements where present (≥5%)
  mobility: number;   // 0..1 travel-proneness (≥0.7 = merchant diaspora)
  top_cities: [string, number][];
  houses: string[];
  family?: string;    // language family (Italic, Semitic, …) or "Creole (A · B)"
  origin?: string;    // static origin card (Cultures 2.0)
  kit?: number;       // costume/appearance kit 0..17 (-1 unknown); for figure art
  kit2?: number;      // creole: second parent kit (blend), else -1
  desired_goods?: string[]; // goods this people prizes (cultural taste)
  traits?: CultureTraitBrief[]; // character traits (2–3)
  relations?: CultureRelation[]; // kin / friendly / rival / hostile neighbours
  wealth?: number;    // total wealth (for richest-cultures ranking)
  alive?: boolean;    // false = extinct (filed under Vanished peoples)
  obituary?: string;  // shown for extinct peoples
  lingua_regions?: number; // # trade regions where this people's tongue is the lingua franca
}
export interface CultureTraitBrief { name: string; emoji: string; blurb: string }
/** One people/community's standing in a settlement (campaign_settlement_peoples). */
export interface PeopleGroup {
  culture: string;
  is_majority: boolean;
  pop_share: number;    // 0..1 of the settlement's population
  civic: number;        // 0..1 share of civic influence
  market: number;       // 0..1 share of trade + production
  power: number;        // blended 0..1
  fondaco: boolean;     // a foreign trading community keeps a bailo (fondaco) here
  council_seat: boolean;
  houses: number;       // houses of this culture operating here
  works: number;        // estates/manufactories they own here
  traits: CultureTraitBrief[];
  note: string;
}
export interface SettlementPeoples {
  hub: number;
  name: string;
  population: number;
  majority_culture: string;
  groups: PeopleGroup[];
}
export interface CultureRelation { name: string; kind: string } // kin|friendly|rival|hostile
export interface NotableCity { name: string; x: number; y: number; role: string } // seat|office
export interface NotablePerson {
  name: string; house: string; era: string; known_for: string;
  city: string; wealth: number; alive: boolean; cities: NotableCity[];
}
/** Coarse "where a people lives" raster (Peoples-panel mini-map; mirrors the goods preview). */
export interface CulturePresenceGrid { width: number; height: number; data: number[]; land: number[]; dominant?: number[] }
/** One backer of a colony venture (city / house / bank). */
export interface ColonyBackerRow { kind: number; name: string; color: string; share: number }
/** One civic supply contract feeding a colony. */
export interface ColonySupplyRow { category: number; supplier: string; good: string; qty: number }
/** Colony detail for the HubPanel "Supply" subtab. */
export interface ColonyDetail {
  stage: number;
  autonomous: boolean;
  founder_name: string;
  main_bank_name: string;
  coin_name: string;
  charter_open: boolean;
  supply_years: number;
  reserve_food: number;
  reserve_cap: number;
  age_years: number;
  indep_in_years: number;
  backers: ColonyBackerRow[];
  supply: ColonySupplyRow[];
  supply_ships: number;      // dedicated grain-run ships
  supply_capacity: number;   // monthly carriage of the fleet
  supply_delivered: number;  // food delivered last month
  supply_source: string;     // designated food source city
}
/** Abstract social strata of a settlement (HubPanel "Society" block). The four
 *  shares sum to 1; inequality + welfare are 0..1 derived read-outs. */
export interface SocietyBrief {
  patrician: number;
  burgher: number;
  commoner: number;
  underclass: number;
  commoner_wealth: number;
  inequality: number;
  welfare: number;
  /** 0 = content … 1 = boiling — civil unrest (It. 3). */
  unrest?: number;
}
/** One roster row in the Colonial Office (campaign_get_colonies). Covers both
 *  settlement colonies (colony_kind 1) and house trade outposts (colony_kind 2). */
export interface SatSupplyRow {
  category: string;
  good: string;
  source: string;
  rate: number;
  met: number;
}
export interface SatelliteBrief {
  id: number;
  name: string;
  metropolis: string;
  metropolis_id: number;
  role: string;
  stage: number;
  progress: number;
  overall: number;
  eta_years: number;
  monthly_cost: number;
  fund: number;
  runway_months: number;
  convoys: number;
  idle_months: number;
  founded_year: number;
  supply: SatSupplyRow[];
  exploits: string[];
}
export interface MigrationRouteBrief {
  path: [number, number][];
  culture: string;
  volume: number;
  from_hub: number;
  to_hub: number;
  age_years: number;
}
export interface ProvGoodRow {
  good: string;
  secured: number;
  target: number;
  food: boolean;
}
export interface ProvisioningBrief {
  first_buy: boolean;
  dominant_house: string;
  dominant_share: number;
  dependents: number;
  reserve_target: number;
  bought_month: number;
  goods: ProvGoodRow[];
}

export interface ColonySummary {
  id: number;
  name: string;
  x: number;
  y: number;
  colony_kind: number;   // 1 = settlement colony · 2 = house outpost · 3 = satellite
  colony_stage: number;  // 1 outpost · 2 colony · 3 town · 4 city
  autonomous: boolean;
  population: number;
  founder_hub: number;
  founder_name: string;
  founder_x?: number;
  founder_y?: number;
  main_bank_name: string;
  coin_name: string;
  charter_open: boolean;
  reserve_food: number;
  reserve_cap: number;
  supply_years: number;
  age_years: number;
  indep_in_years: number;
  owner_house_name: string; // house outposts only
  owner_color: string;
  supply_ships: number;      // dedicated grain-run ships
  supply_delivered: number;  // food delivered last month
}
/** Read-only founding-gate status for the Colonial Office "why no colonies yet?"
 *  empty state (campaign_colony_gates). Mirrors maybe_found_settlement_colony. */
export interface ColonyGateStatus {
  year: number;
  start_year: number;
  year_ok: boolean;
  qualifying_founder: string;
  founder_ok: boolean;
  bank_on_continent: boolean;
  colonizable_sites_in_range: number;
  site_ok: boolean;
  settlement_colonies: number;
  max_settlement_colonies: number;
  at_colony_cap: boolean;
  min_pop: number;
  blocking_gate: string; // "cap"|"year"|"founder"|"bank"|"site"|"none"
}
/** One weekly per-hub history sample (settlement-window charts). */
export interface HubSample {
  tick: number;
  population: number;
  wealth: number;
  mood: number;
  price_index: number;
  /** Fraction of demand unmet by need tier (0 = supplied, 1 = none met). */
  lack_basic?: number;
  lack_comfort?: number;
  lack_luxury?: number;
  /** Merchant population by class. */
  pop_house?: number;
  pop_local?: number;
  pop_guild?: number;
}
/** One good's live state at a hub (settlement-window Market tab). */
export interface HubGoodDetail {
  good: number;
  name: string;
  price: number;
  base_value: number;
  stock: number;
  need: number;
  production: number;
  world_min: number;
  world_min_hub: string;
  world_max: number;
  world_max_hub: string;
  world_avg?: number; // mean ×-world price across all settlements right now
  quality?: number;   // this hub's production quality 0..1 for the good
  grade?: string;     // grade label if the hub produces it ("Fine", "Exquisite", …)
  /** PERSISTED yearly price series for this (hub, good), grain-eq, most recent
   *  last (`TradeHist.prices`). Empty for a good this hub has never traded. */
  price_hist?: number[];
  /** Matching yearly traded volume — same years, same length. */
  vol_hist?: number[];
  /** Who SUPPLIED this good here recently, shares summing to 1 (all zero = nothing
   *  arrived): [city, house, guild, local, foreign]. Seller side only — there is no
   *  buyer attribution, see docs/TRADE_AND_MARKET_REVIEW.md. */
  supply_shares?: [number, number, number, number, number];
}
/** One good's world-wide quality + trade picture (the floating Goods window). */
export interface GoodMarketRow {
  good: string;
  best_quality: number;
  best_grade: string;
  best_city: string;
  avg_quality: number;
  produced: number;
  traded: number;
  n_producers: number;
  manufactured: boolean;
  grades: GradeBucket[];   // per quality-tier breakdown (Exquisite→Coarse)
}
/** One quality tier of a good (produced & traded at that grade). */
export interface GradeBucket {
  grade: string;
  produced: number;
  traded: number;
  n_producers: number;
}
/** One shipment touching a settlement (Market tab arrivals/departures). */
export interface ShipmentRow {
  owner: string;
  color: string;
  is_guild: boolean;
  other: string;          // origin (arrivals) or destination (departures) city
  good: string;
  amount: number;
  price: number;          // ×-world price
  value: number;          // amount × local price (ranking key)
  sea: boolean;
  returning_home: boolean;
  /** The price the DEAL was struck at (×-world), as distinct from `price`, which
   *  is the viewing city's own local quote. 0 = unknown (a pre-existing save). */
  deal_price?: number;
  /** Days until an in-flight cargo lands (0 for a completed deal). */
  eta_days?: number;
  /** Days since a completed deal was struck (0 for in-flight). */
  age_days?: number;
}
/** Full live per-settlement detail (sentiment + market + history). */
export interface CoinShare {
  coin_name: string;
  share: number;   // 0..1 of the city's circulation
  main: boolean;   // the city's main settling coin
  reserve: boolean; // a foreign reserve coin circulating here
}

/** A sub-cap hinterland village that markets through a town (satellite trade). */
export interface HinterlandVillage { name: string; population: number; x: number; y: number }
/** Cultures 2.0 · a resident people's contentment in a city (prized-goods supply). */
export interface CultureMood {
  name: string;
  share: number;         // 0..1 of the city's population
  satisfaction: number;  // 0..1 mean availability of its prized goods
  color: [number, number, number];
  met: string[];         // prized goods well-supplied here
  unmet: string[];       // prized goods scarce/dear here
}
/** A waystation along a trade corridor (1 river-port · 2 caravanserai · 3 coastal
 *  factory · 4 mountain-pass hospice). */
export interface CorridorWaystation { x: number; y: number; kind: number }
/** A campaign trade corridor — a long river/terrain-routed haul between a home city
 *  and a distant one, owned by a house and strung with waystations. */
export interface TradeCorridor {
  origin: string;
  dest: string;
  owner: string;
  color: string;
  good: string;
  volume: number;
  km: number;
  days: number;
  land_legs: number;
  river_legs: number;
  sea_legs: number;
  points: [number, number][];
  waystations: CorridorWaystation[];
}
/** A live financed expedition crawling toward a distant city (the way a corridor is
 *  earned). Carries the fleet, cargo, leader, survival and its struggle log. */
export interface ExpeditionView {
  id: number;
  house: number;        // backer house index (Phase 1.3)
  leader: string;
  origin: string;
  dest: string;
  dest_province: number; // -1 if none

  x: number;
  y: number;
  ox: number; oy: number;
  dx: number; dy: number;
  progress: number;     // 0..1 over the whole round trip
  outbound: boolean;
  status: number;       // 0 en-route · 1 arrived · 2 returning
  caravans: number;
  ships: number;
  good: string;
  survived: number;     // fraction of the fleet still alive
  cost: number;
  launched_year: number;
  hazards: [number, number, number][]; // recent (x, y, kind)
}
/** A recent failed venture, for the map ✕ overlay. */
export interface ExpeditionFail { x: number; y: number; kind: number }
export interface ExpeditionsPayload { active: ExpeditionView[]; failed: ExpeditionFail[] }
/** A settlement building resolved with WHO owns/controls it, for the ward grid. */
export interface BuildingInfo {
  label: string;
  effect: string;
  emoji: string;
  owner: string;
  owner_kind: "house" | "civic" | "fondaco" | string;
  color: string;         // hex tint (house heraldry / civic slate / people hearth)
}
export interface HubDetail {
  id: number;
  name: string;
  x: number;
  y: number;
  population: number;
  koppen: number;
  coastal: boolean;
  is_estate: boolean;
  mood: number;
  sent_food: number;
  sent_prosperity: number;
  sent_stability: number;
  grain_wealth: number;
  trade_wealth: number;
  food_balance: number;
  starving: number;
  goods: HubGoodDetail[];
  history: HubSample[];
  events: JournalEntry[];
  houses?: HouseBrief[];
  in_by_sea?: number;   // recent supply arriving by ship (sea)
  in_by_land?: number;  // recent supply arriving by caravan (land)
  /** Current fraction of demand unmet by need tier (0 = supplied, 1 = none met). */
  lack_basic?: number;
  lack_comfort?: number;
  lack_luxury?: number;
  /** Estimated merchant population by class. */
  pop_house?: number;
  pop_local?: number;
  pop_guild?: number;
  /** Abstract social strata of this settlement (null for estates / unseeded hubs). */
  society?: SocietyBrief | null;
  /** Estate descriptors (kind 1 farm/2 mine/3 plantation/4 fishery/5 vineyard). */
  estate_kind?: number;
  estate_owner?: string;
  estate_good?: string;
  /** Buildings erected here: [name, one-line effect]. Legacy flat list. */
  structures?: [string, string][];
  /** Ward-grid buildings, each resolved with its owning faction + tint colour
   *  (house heraldry / civic slate / diaspora fondaco). Recolours live. */
  buildings?: BuildingInfo[];
  /** Trade-base patron: the merchant house developing this city as a base (empty = none). */
  patron?: string;
  /** #23 · majority people of this settlement. */
  culture?: string;
  /** #23 · minority quarters [people, population share 0..1], grown by in-migration. */
  minorities?: [string, number][];
  /** Cultures 2.0 · per-people contentment here (are their prized goods supplied?). */
  culture_moods?: CultureMood[];
  /** Foreign merchant offices hosted in this settlement. */
  offices_here?: OfficeHere[];
  /** Market flow: in-flight shipments arriving / departing (ranked by value). */
  arrivals?: ShipmentRow[];
  departures?: ShipmentRow[];
  /** Recently completed deals (most recent first), by direction. */
  recent_arrivals?: ShipmentRow[];
  recent_departures?: ShipmentRow[];
  bought?: number;
  sold?: number;
  /** Estates & manufactories in this city's hinterland. */
  estates_here?: EstateRow[];
  /** DLC 3 · the polis government of this seat (null for estates). */
  government?: Government | null;
  treasury?: number;                 // retained civic treasury
  finance?: CityFinance | null;      // treasury books (current + prev)
  public_health?: number;            // hospices/quarantine level 0..0.6 (cuts plague deaths)
  satellites?: HinterlandVillage[];  // sub-cap villages that market through this town
  war_with?: string;                 // polis at war with ("" = peace)
  coin_name?: string;
  coin_trust?: number;
  coin_value?: number;
  coin_basket?: CoinShare[];         // which coins circulate here + share (main first)
  transit?: TransitRow[];            // carrying trade through this city's merchants
  stolen_good?: string;              // espionage: good whose technique this estate stole
  stolen_from?: string;              // city it was stolen from ("" = none)
  related_colonies?: ColonySummary[]; // colonies/outposts this city founded
  city_stores?: CityStores;          // civic warehouse + all goods held at the city (+ value)
  dev_tier?: number;                 // development tier 0..5 (Outpost..Emporium)
}
/** City stores: the civic (city-owned) warehouse + all goods held at the city, valued. */
export interface CityStores {
  reserve: CivicGoodRow[];   // civic warehouse contents, top by amount
  reserve_value: number;     // grain-eq value of the civic reserve
  food_reserve: number;      // food held in city stores (granary + reserve), units
  top_goods: CivicGoodRow[]; // ALL goods held at the city, top by value
  goods_value: number;       // grain-eq value of ALL goods at the city ("riches in goods")
  goods_units: number;       // total units of all goods held at the city
}
/** DLC 3 · a polis seat's government: council house + fiscal policy + speculation. */
export interface Government {
  council: string;
  council_color: string;
  council_archetype: string;
  council_is_guild: boolean;
  council_power: number;
  tariff_export: number;
  tariff_import: number;
  tariff_default: boolean;
  mint_fineness: number;
  treasury: number;
  civic_pool: number;
  spec_risk: number;
  spec_tier: string;   // "" | LOW | MED | HIGH
  spec_stars: number;
  spec_pattern: string;
  spec_drivers: string[];
  spec_watch: string[];
  // Government layer
  govt_type: string;
  next_election_years: number;
  captor: string;
  captor_color: string;
  officials: OfficialRow[];
  family_influence: InfluenceRow[];
  laws: LawRow[];
  civic_goods: CivicGoodRow[];
  /** CITY_PROVINCE_WAR_PLAN.md §3.1 · the office as a person — null when no house
   *  holds either office (the ordinary early-campaign case). */
  leader: CityLeader | null;
}
/** §3.1 · the head of whichever house runs this seat — reuses the existing
 *  house-person stack, no new entity. */
export interface CityLeader {
  house: string;
  house_color: string;
  is_guild: boolean;
  /** True when held by CAPTURE (a majority of control-weighted offices) rather
   *  than merely being the dominant council house. */
  is_captor: boolean;
  head_name: string;
  female: boolean;
  /** "" for a middling character. */
  character_phrase: string;
  /** "" when the head has no vice. */
  vice: string;
}
export interface OfficialRow {
  role: string;
  name: string;
  allegiance: string;       // "" = neutral
  allegiance_color: string;
  control: number;          // 0..1
  status: "neutral" | "leaning" | "controlled" | "kin" | string;
}
export interface InfluenceRow { name: string; color: string; pct: number }
export interface LawRow { year: number; text: string }
export interface CivicGoodRow { name: string; amount: number }
/** Trade Flows subtab — one traded good at a settlement (avg + last-year volume,
 *  route count, and a yearly volume series for the trend graph). */
export interface TradeFlowGood {
  good: number;
  name: string;
  avg_volume: number;
  last_volume: number;
  in_volume: number;
  out_volume: number;
  route_count: number;
  history: number[];
  /** TRADE_STAGING_AND_POSTS_PLAN.md Slice 1 — this city's own yearly output of
   *  the good. `transit = max(0, out - own_production)`, `own_export = out -
   *  transit`, `for_us = in - transit`. */
  own_production: number;
}
/** One good's flow along one partner route (per-good route list + map highlight). */
export interface TradeRouteFlow {
  good: number;
  partner: number;
  partner_name: string;
  px: number;
  py: number;
  dir: number;   // 0 inbound, 1 outbound
  amount: number;
  pct: number;
  km: number;
  days: number;
  /** 0 land, 1 sea, 2 river. */
  mode: number;
}
/** A top partner city: share of all this city's trade + goods exchanged. */
export interface TradePartner {
  hub: number;
  name: string;
  px: number;
  py: number;
  volume: number;
  pct: number;
  goods: string[];
}
/** The settlement Trade-Flows payload. */
export interface TradeFlows {
  hub: number;
  hub_x: number;
  hub_y: number;
  goods: TradeFlowGood[];
  routes: TradeRouteFlow[];
  partners: TradePartner[];
}
/** One estate / manufactory in a settlement's hinterland. */
export interface EstateRow {
  /** 4.6 · this estate's own hub id — pass to campaignWorksCard(hub). */
  hub: number;
  name: string;
  kind: number;       // 1 farm/2 mine/3 plantation/4 fishery/5 vineyard/6 manufactory
  good: string;
  output: number;
  owner: string;
  owner_is_guild: boolean;
  owner_is_civic?: boolean; // city-financed (locally owned)
  tier: number;       // upgrade tier 1..5
  damage?: number;    // disaster damage 0 (intact) .. 1 (ruined)
}
/** One city in the live richest-cities ranking. */
export interface CityRank {
  id: number;
  name: string;
  population: number;
  wealth: number;
  trade: number;
  pct_world: number;
  // C1 · prosperity composite + its stock/broad-based components.
  prosperity: number;
  treasury: number;
  commoner_wealth: number;
  inequality: number;
}
/** One active merchant route for the campaign merchant map layer. */
export interface MerchantRoute {
  a: [number, number];
  b: [number, number];
  a_name: string;
  b_name: string;
  holder: string;
  color: string;
  is_guild: boolean;
  sea: boolean;
  volume: number;
  out_goods: [string, number][]; // goods a→b
  ret_goods: [string, number][]; // goods b→a
  path?: [number, number][]; // routed a→b polyline (roads/sea); skipped if no corridor
}
/** One active futures contract as a directional supply lane (source → buyer). */
export interface FuturesLane {
  a: [number, number];   // source (producer/warehouse) city
  b: [number, number];   // buyer (receiver) city
  a_name: string;
  b_name: string;
  holder: string;        // seller house / guild
  color: string;
  is_guild: boolean;
  good: string;
  qty: number;           // monthly delivered quantity
  term: number;          // 1 / 3 / 5 / 7 years
  end_year: number;      // campaign year the contract expires
  suspended: boolean;    // force-majeure (plague lockup) right now
  path?: [number, number][]; // routed source→buyer polyline (roads/sea); straight a→b if absent
  delivered?: number;      // running total delivered to date
  fulfilled_pct?: number;  // delivered vs what was due by now (0-100)
  value?: number;          // grain-eq value moved so far
  sealed_at?: string;      // city where the deal was struck (buyer)
}
/** One house/guild asset for the Warehouses & Estates infographic. */
export interface WarehouseInfo {
  kind: string; // "warehouse" or estate kind (farm/mine/manufactory/…)
  owner: string;
  color: string;
  is_guild: boolean;
  city: string;
  x: number;
  y: number;
  tier: number;
  capacity: number;
  used: number;
  goods: [string, number][]; // (good, stock), largest first
  contracts: number;         // futures contracts this depot supplies
  damage: number;
}
/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.3 (D17) · one good's slot in the
 *  CITY warehouse grid — three grade bands (D3), this month's movement, what
 *  rotted, and a cover-in-months reading. */
export interface CityWarehouseGood {
  good: number;
  name: string;
  amount: number;
  coarse: number;
  common: number;
  fine: number;
  delta_month: number;
  spoiled_month: number;
  need_tier: number; // 0 Life · 1 Daily · 2 Luxury
  cover_months: number;
  /** 4.4 (D20) · recent-delivery shares, 0..1: [city, house, guild, local, foreign]. */
  supply_shares: [number, number, number, number, number];
}
/** The city's own warehouse (D17/F6) — distinct from a house/guild `WarehouseInfo` depot. */
export interface CityWarehouseInfo {
  hub: number;
  city: string;
  capacity: number;
  used: number;
  fill_frac: number;
  spoiled_total_month: number;
  goods: CityWarehouseGood[];
}
/** 4.6 (D15/D16/A10) · one resolved row of a works' ownership bar. */
export interface WorksOwnerShare {
  holder_kind: number; // 0 city · 1 house · 2 guild · 3 bank · 4 realm
  name: string;
  color: string;
  frac: number;
  payout: number;      // 0 offtake · 1 dividend
  instrument: number;  // 0 perpetual SHARE · 1 fixed-term TENANCY
  term_years: number;
}
/** One monthly sample point on a works card's curves. */
export interface WorksMonthPoint { output: number; quality: number; price: number }
/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 · everything one expandable works
 *  card needs — rank/yield (D15), condition, the ownership bar (D1), and the
 *  twelve-month curves (§3). */
export interface WorksCardInfo {
  hub: number;
  name: string;
  kind: number;
  kind_label: string;
  tier: number;
  good: number;
  good_name: string;
  condition: number;
  damage: number;
  yield_index: number;
  yield_label: string;
  rank: number;
  rank_of: number;
  monthly_output: number;
  output_delta: number;
  quality: number;
  owners: WorksOwnerShare[];
  monthly: WorksMonthPoint[];
  /** 4.13 (A3) · "Kalos wine" — set once the works has been chronicled for
   *  reaching GREAT or better; null otherwise. */
  brand: string | null;
}
/** A foreign merchant's office hosted in a settlement (host-side view). */
export interface OfficeHere {
  holder: string;          // house / guild name
  color: string;
  is_guild: boolean;
  origin: string;          // city the holder is based in
  throughput_pct: number;  // % of this settlement's live trade it handles
  goods: string[];
}
/** A merchant family (trading house) — for the Houses panel + settlement window. */
/** DLC 4 · one typed population unit of a hub (Nations & POPs foundation). */
export interface PopBrief {
  profession: string;
  size: number;
  money: number;
  needs_life: number;
  needs_everyday: number;
  needs_luxury: number;
  consciousness: number;
  militancy: number;
}

export interface HouseBrief {
  idx?: number;        // index into sim.houses — key for the ledger query
  barred?: string[];   // cities this house is barred from (active trade wars)
  name: string;        // "House Cassii"
  head_name: string;   // "Marcus Cassii"
  home_hub: number;    // home hub id
  home_name: string;
  wealth: number;
  prestige: number;
  political_power: number;
  volume?: number;     // recent trade volume — the "trade amount" the house moves
  generation: number;
  head_age: number;    // years the current head has led
  specialties: string[];
  top_goods: string[];            // top exported/traded goods (what the house is known for)
  monopolies: [string, number][]; // good name + share 0..1
  rivals: string[];
  defunct: boolean;
  color?: string;                // stable distinct colour (hex) for this house
  seat?: [number, number];       // home-seat position (world cell coords)
  gem_variety?: string;          // #8 · principal stone for this house's "gemstones" (else "")
  dominant?: boolean;            // controls at least one settlement (>=50% of its trade)
  controls?: [number, number][]; // settlements it controls (seat or remote outposts)
  partners?: [number, number][]; // trade-partner settlements (world coords)
  cities?: string[];             // names of cities it trades with / controls (seat first)
  archetype?: number;
  archetype_label?: string;
  archetype_perk?: string;
  charters?: string[];
  fleet_sea?: number;            // ships / river boats / caravans = concurrent cargo slots
  fleet_river?: number;
  fleet_caravan?: number;
  is_guild?: boolean;            // a civic Merchant Guild (acts for its home city)
  offices?: [string, [number, number]][]; // foreign cities where it has an office
  estates?: [string, string][];  // estates/manufactories it owns: [good, city]
  active?: HouseCity[];          // cities ranked most→least influential, with role
  owns_bank?: boolean;           // owns a chartered bank (🏦 badge + Bank subtab)
  founded_year?: number;
  worst_loss?: number;
  mono_ever_count?: number;
  coin_name?: string;            // the coin it mints (via its council seat), "" if none
  coin_value?: number;
  coin_trust?: number;
  tier?: number;                 // 1 great · 2 major · 3 lesser · 4 marginal · 0 = unranked (guild, or too new)
  standing?: number;             // 0..1 score the tier is banded from
  kit?: number;                  // seat culture's language-kit index, -1 unresolvable — drives the dress figure
  head_female?: boolean;
  peak_wealth?: number;          // all-time peak wealth — "the house's finest hour"
  peak_wealth_tick?: number;
  goods_ledger?: GoodTradeRow[]; // goods moved the most (by volume) + profit each
}
/** One good in a house's trade ledger: cumulative amount shipped + profit earned. */
export interface GoodTradeRow {
  good: string;
  volume: number;  // cumulative amount shipped (grain-eq units)
  profit: number;  // cumulative profit earned on that good
}
/** One city a house operates in, for the influence-ranked "Active in" list. */
export interface HouseCity {
  name: string;
  x: number;
  y: number;
  influence: number;             // 0..1
  role: string;                  // "seat" | "bailo" | "dominant" | "office" | "trade"
  contested: boolean;            // a rival also holds significant influence here
}

export interface HouseTimelineEvent {
  year: number;
  kind: string; // founded | succession | monopoly | control_gained | control_lost | branch | loss | dissolved
  text: string;
}

/** One labelled money line in the Accountant view (per-city tax/profit or a
 *  warehouse good); per-city lists arrive sorted largest → lowest. */
export interface LedgerLine {
  label: string;
  amount: number;
}

/** A house/guild's yearly T-account ledger (the last completed year). */
export interface HouseLedger {
  name: string;
  is_guild: boolean;
  year: number;
  // Income
  trade_profit: LedgerLine[];
  office_income: number;
  estate_income: number;
  income_total: number;
  // Expenditure
  import_tax: LedgerLine[];
  export_tax: LedgerLine[];
  estate_tax: number;
  upkeep: number;
  fleet_cost: number;
  lost_cargo: number;
  events: number;
  consumption: number;
  inflation: number;
  /** CITY_PROVINCE_WAR_PLAN.md §3.4e · forced war levy paid this year — its own
   *  line, split out of ordinary civic tax. */
  war_levy: number;
  /** §3.4e · wealth-equivalent loss from war damage to this house's own estates. */
  war_damage: number;
  expense_total: number;
  net: number;
  wealth_graph: number[];
  wealth_years: number[];      // yearly wealth, oldest→newest (~last 10 years)
  wealth_start_year: number;   // campaign year of the first wealth_years sample
  // Warehouse stock at the home city
  warehouse_city: string;
  warehouse: LedgerLine[];
}

export interface HouseHistory {
  name: string;
  color: string;
  founder: string;
  founded_year: number;
  events: HouseTimelineEvent[];
  top_goods: [string, number][]; // most profitable resources (name + cumulative profit)
  defunct: boolean;
  gem_variety?: string;          // #8 · principal stone for this house's "gemstones"
  colonies?: ColonySummary[]; // colonies owned (outposts) or backed by this house
  line?: HeadBrief[];         // the succession line — every head this house has had
}
/** One head of a house, for the chronicle view. */
export interface HeadBrief {
  name: string;
  female: boolean;
  generation: number;
  since_year: number;
  until_year: number;   // 0 = still living
  age_at_accession: number;
  age_at_death: number;
  wealth_start: number;
  wealth_end: number;
  accession: string;    // "founder" | "heir" | "co-heir" | "the hearth-keeper" | "eldest capable" | "sister's son" | "daughter of the house"
  epithet: string;      // "" if none earned
}
/** One member of a house's kin roster (Phase 2.1). */
export interface KinBrief {
  name: string;
  female: boolean;
  age: number;
  role: string;          // "head" | "heir" | "factor" | "idle" | "married out" | "dead"
  posted_name: string;   // the holding they run, if role == "factor" ("" if unposted)
  loyalty: number;       // 0..1
  skill: number;         // 0..1
  character_phrase: string; // "" if unremarkable
  power_share: number;   // 0..100, sums to 100 across the roster
}
/** One ambition, active or historical (Phase 3.1). */
export interface GoalBrief {
  what: string;          // "cornering the silk trade"
  state: number;         // 0 pursuing · 1 achieved · 2 failed · 3 abandoned
  set_year: number;
  deadline_year: number;
  progress_frac: number; // 0..1 where honest, -1 where the kind has no fraction to show
}
export interface GoalsBrief {
  active: GoalBrief[];
  history: GoalBrief[]; // most recent first
}
/** One quarterly round of an active crisis (Phase 3.2-3.6). */
export interface CrisisRoundBrief {
  action: number;      // 0 concede · 1 buy off · 2 venture · 3 stand firm
  result: number;      // -1 backfired · 0 no effect · +1 worked
  head_delta: number;
  text: string;
}
/** The house's OPEN succession crisis, if any. */
export interface ActiveCrisisBrief {
  cause: string;
  round: number;
  round_cap: number;
  head_support: number;
  plot_support: number;
  undecided: number;
  loyalist_name: string;
  loyalist_tint: string;
  plot_name: string;
  plot_tint: string;
  plot_leader_name: string;
  /** Why the plot leader stands where they do (derived, not stored). */
  plot_leader_motive: string;
  /** Why the ruler holds on. */
  head_motive: string;
  heir_choice: number;  // 0 stood with the ruler · 1 turned to the plot · 2 no heir kin
  rounds: CrisisRoundBrief[];
  opened_year: number;
}
/** One closed crisis from the permanent record. */
export interface CrisisRecordBrief {
  opened_year: number;
  closed_year: number;
  cause: string;
  loyalist_name: string;
  loyalist_tint: string;
  plot_name: string;
  plot_tint: string;
  rounds: number;
  peak_plot: number;
  outcome: number;      // 1 prevailed · 2 deposed · 3 dissolved
  successor: string;
}
export interface CrisisBrief {
  active: ActiveCrisisBrief | null;
  history: CrisisRecordBrief[]; // most recent first
  secure_until_year: number;    // 0 if not currently immune
}
/** One house in a lineage chain (Phase 5-adjacent — the dossier's 🌳 Lineage tab). */
export interface LineageNode {
  idx: number;
  name: string;
  alive: boolean;
  tier: number;         // 0 = not yet tiered (guild, or too new)
  origin_kind: number;  // 0 founded/guild-charter · 1 guild-seed · 2 branch · 3 division · 4 departure · 5 independence
  origin_year: number;
  origin_text: string;  // the founding event's own chronicle text
  color: string;
}
export interface HouseLineage {
  ancestors: LineageNode[];  // root-first, NOT including this house
  offshoots: LineageNode[];  // houses whose origin_house is this one
}
export interface JournalEntry {
  tick: number;
  kind: string;
  hub: number;
  good: number;
  value: number;
  text: string;
}
/** "Is trade actually moving?" snapshot for the last advance. */
export interface CampaignDiagnostics {
  tick: number;
  year: number;
  in_transit: number;
  shipments_last: number;
  by_house: number;
  by_guild: number;
  lost_last: number;
  volume_last: number;
  houses_active: number;
  houses_defunct: number;
  fleet_sea: number;
  fleet_river: number;
  fleet_caravan: number;
  controlled_settlements: number;
  total_house_wealth: number;
}
export interface CampaignSnapshot {
  active: boolean;
  clock: CampaignClock;
  hubs: CampaignHubBrief[];
  recent_events: JournalEntry[];
  price_index: number;
  in_transit: number;
  /** Total population across all hubs (shown as a number in the world pulse). */
  total_population: number;
  /** Population change since the last monthly chronicle sample. */
  population_delta: number;
  /** World price-index change since the last monthly chronicle sample. */
  price_index_delta: number;
  /** Atlas 2.0 · recent refugee roads [from_x, from_y, to_x, to_y, tick] —
   *  drawn as fading migration arrows for ~4 years. */
  migrations: [number, number, number, number, number][];
}
export interface WorldGoodPrice {
  good: number;
  name: string;
  world_price: number;
  producers: number;
  top_hub: string;
}
/** #30 · one city's live cost-of-living basket index (campaign_city_price_index). */
/** One live city in the Markets window's picker (campaign_market_cities). */
export interface MarketCity {
  /** Hub INDEX — the same id campaignGetHub takes. */
  id: number;
  name: string;
  population: number;
  x: number;
  y: number;
}
export interface CityPriceIndex {
  name: string;
  index: number; // need-weighted mean of price ÷ base_value, ×100 (100 = world standard)
}
export interface WorldEconomy {
  goods: WorldGoodPrice[];
  index_series: [number, number][];
  /** Current population-weighted fraction of demand unmet by need tier. */
  lack_basic?: number;
  lack_comfort?: number;
  lack_luxury?: number;
  /** Current world merchant-population totals by class. */
  pop_house?: number;
  pop_local?: number;
  pop_guild?: number;
  /** World time series [tick, basic, comfort, luxury] (population-weighted unmet). */
  lack_series?: [number, number, number, number][];
  /** World time series [tick, houses, local, guild] merchant population totals. */
  merchant_series?: [number, number, number, number][];
  /** Atlas 2.0 · yearly world samples [year, population, trade volume, live hubs,
   *  cumulative foundings, cumulative abandonments] for the Atlas graphs. */
  world_series?: [number, number, number, number, number, number][];
  /** Batch 1 · the Hall of Records (all-time world records). */
  records?: WorldRecords;
}

// ── Economy snapshot (Phase 2) ──
export interface EconHubGood {
  good: number;
  good_name: string;
  amount: number;
  quality: number;
  grade: string;
  flavor: string;
  price: number;
}
export interface EconReceive {
  good: number;
  good_name: string;
  amount: number;
  price: number;
  chain: number;
  from_hub: number;
}
export interface ExchangeRate {
  good_name: string;
  /** Units of the counter-good one unit of this good buys here. */
  ratio: number;
}
export interface HubMarketGood {
  good: number;
  good_name: string;
  /** Local price in the grain-equivalent numeraire. */
  price: number;
  /** World-standard value (the good's base_value) for comparison. */
  base_value: number;
  in_flow: number;
  out_flow: number;
  exchanged_for: ExchangeRate[];
}
/** One emergent currency good + the components that made it money. */
export interface HubCurrency {
  good: number;
  name: string;
  liquidity: number;  // distinct trade counterparties
  value: number;      // grain-equivalent base value
  stability: number;  // 0..1, 1 = rock-steady price
  price: number;      // local grain-equivalent price (for exchange ratios)
}
/** Per-hub market panel data from the equilibrium solver. */
export interface HubMarket {
  /** Food-stock value per capita (food security). */
  grain_wealth: number;
  /** Net market earnings per capita (commercial prosperity). */
  trade_wealth: number;
  /** Emergent currency goods at this hub. */
  currency_goods: string[];
  /** Explained currency goods (liquidity/value/stability + grain price). */
  currencies?: HubCurrency[];
  prices: HubMarketGood[];
}
export interface EconHub {
  id: number;
  x: number;
  y: number;
  name: string;
  power: number;
  stars: number;
  wealth: number;
  population: number;
  emporium?: boolean; // greatest pass-through entrepôt (rendered red)
  throughput?: number;
  exports?: number;
  imports?: number;
  partners?: number;
  ref_pct?: number;      // throughput as % of the strongest hub
  nearest_ref?: string;  // closest real 1450 trade hub
  monopolies?: string[];
  koppen?: number;       // climate at the hub cell
  elevation?: number;    // normalized elevation
  coastal?: boolean;     // on/near the coast
  nobility?: number;     // wealthy/patrician class size
  merchants?: number;    // merchant class size
  commoners?: number;    // everyone else
  elite_level?: number;  // 0..1 how large the wealthy class is
  merchant_level?: number; // 0..1 how large the merchant class is
  top_export?: string;   // the good that brings the city the most wealth
  top_export_share?: number; // its fraction of the hub's total export value
  luxuries?: HubLuxury[];
  sea_access?: boolean;  // real sea port (not a closed lake)
  exports_to?: EconExport[]; // where this hub's exports go (with %)
  shortages?: ShortageNote[]; // goods it can't fully obtain + why
  /** Market panel: equilibrium prices, barter ratios, currency goods. */
  market?: HubMarket | null;
  produces: EconHubGood[];
  receives: EconReceive[];
}
export interface EconExport {
  good: number;
  good_name: string;
  to_hub: number;
  amount: number;
  pct: number;   // 0..100 share of this good's exports leaving the hub
  chain: number;
}
export interface ShortageNote {
  good: number;
  good_name: string;
  reason: string; // "no_supplier" | "unreachable" | "deficit" | "no_port"
  severity: number; // 0..1 fraction of demand unmet
}
export interface HubLuxury {
  good: number;
  good_name: string;
  demand: number;
  received: number;
  price: number;
}
export interface EconChainStop {
  hub: number;
  price: number;
  days: number; // cumulative travel days from origin to this stop
  km: number;   // cumulative distance from origin to this stop
  markup?: number;       // merchant resale margin applied at this stop
  toll?: number;         // toll/tax fraction added entering this stop
  demand_spike?: number; // transient premium from local unmet demand
  koppen?: number;       // climate at this stop hub (for the narrative)
  note?: string;         // short text reason for the price change here
}
export interface EconChain {
  id: number;
  good: number;
  good_name: string;
  stops: EconChainStop[];
  points: [number, number][];
  days: number;  // total travel days origin → consumer
  km: number;    // total distance origin → consumer
  value: number; // shipment value = amount × delivered price
  mode: number;  // dominant transport mode (0 land / 1 sea / 2 river)
}
export interface CorridorGood {
  good: number;
  good_name: string;
  value: number;
}
export interface EconCorridor {
  a: number; // hub id (min)
  b: number; // hub id (max)
  points: [number, number][]; // [hub a, hub b] world coords
  fwd_value: number;          // value flowing a→b
  bwd_value: number;          // value flowing b→a
  fwd_goods: CorridorGood[];  // a→b cargo, ranked by value
  bwd_goods: CorridorGood[];  // b→a cargo, ranked by value
  days: number;               // one-way travel days across the corridor
  km: number;
  mode: number;
}
export interface EconChokepoint {
  points: [number, number][];
  volume: number;
  share: number;
  name: string;
}
export interface EconRegion {
  hub: number;
  name: string;
  cells: [number, number][]; // coarse-cell top-left world coords (square territory)
  cell_size: number;
}
export interface GoodStat {
  good: number;
  good_name: string;
  top_importer: number;        // hub id (-1 = none)
  top_exporter: number;        // hub id (-1 = none)
  biggest_desire_hub: number;  // hub with greatest demand
  biggest_desire_class: string; // "nobility" | "merchants" | "commoners"
}
export interface ClassStats {
  label: string;     // "hubs" | "emporiums" | "outposts"
  count: number;
  population: number;
  throughput: number;
  avg_wealth: number;
}
export interface EconomySnapshot {
  hubs: EconHub[];
  chains: EconChain[];
  chokepoints: EconChokepoint[];
  regions: EconRegion[];
  corridors: EconCorridor[];
  good_stats?: GoodStat[];
  class_stats?: ClassStats[];
  goods: string[];
}

export interface SharkZone {
  cells: [number, number][]; // coarse-cell top-left world coords (marked area)
  cell_size: number;
  x: number;                 // label centroid
  y: number;
  score: number;
}

export interface GoodRegion {
  good: string;
  cells: [number, number][]; // coarse-cell top-left world coords (marked area)
  values: number[];          // per-cell abundance 0..255 (parallel to cells)
  subtypes: number[];        // per-cell subtype id (grain/paper); [] if none
  cell_size: number;
  x: number;                 // label centroid
  y: number;
  score: number;
  sublabel: string;          // specific gemstone (Ruby/Sapphire/…); else ""
}

/** CLAUDE.md §8.19 (goods localities, shipped) Slice 5 (D3 · D9 · D10) · ONE good's belt as a
 *  FULL-RESOLUTION mask, which is what lets the overlay stop drawing the coarse
 *  blocks of `GoodRegion` that spill past the coastline (F4).
 *
 *  Full resolution IS the coastline clip: a land good's belt byte is already exactly
 *  zero on every sea cell and a marine good's is zero on land, so drawing the belt at
 *  its own resolution ends it on the coast by construction — never snapped to a
 *  province polygon (D3).
 *
 *  Two layers (D9), split because they answer different questions and compress
 *  differently: COVERAGE is boolean at full resolution and run-length encoded (a belt
 *  is contiguous, so this is small); QUALITY is the belt value on a coarse grid,
 *  because a wash needs no per-cell precision — it is painted only where coverage
 *  says so, so the coarse wash still ends exactly on the coastline. */
export interface GoodBeltMask {
  good: string;
  /** Bounding box of the belt in WORLD cells; the mask covers exactly this box. */
  x0: number;
  y0: number;
  w: number;
  h: number;
  /** Full-resolution 0/1 over the box, row-major, as flat (value, run) pairs — the
   *  same encoding `SimProvincesResult.raster_rle` already uses. */
  /** Coverage AND quality in one run-length-encoded layer: flat `(level, run)`
   *  pairs at the mask's full resolution. `level` is 0 for an uncovered cell and
   *  1..15 for the belt's own absolute value quantized into 16 buckets. Replaced a
   *  0/1 coverage RLE plus a separate coarse quality grid — the two resolutions
   *  were the visible bug (a sharp coastline outline filled with ~8-cell blocks). */
  quality_rle: number[];
  /** The belt's own ABSOLUTE value 0..255 on a `qw × qh` grid of `coarse`-cell
   *  blocks over the same box. Never per-good normalised (D10). */
  quality: number[];
  qw: number;
  qh: number;
  /** World cells per quality block. */
  coarse: number;
  /** Per-quality-block subtype id (grain species / paper source); [] if none. */
  subtypes: number[];
  /** Covered world cells. */
  cells: number;
}

/** One good's belt SAMPLED to a single province, at the SAME resolution
 *  `ProvinceTerrainCrop` (the relief plate) samples at — the province-plate
 *  counterpart of `GoodBeltMask`. Read from the goods tile column (so it works on
 *  any world, no localities/campaign needed), it lets `ProvinceMiniMap` draw belt
 *  AREAS + a QUALITY wash at the same fidelity as the ground under them (F1 /
 *  CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 1 — the old version sampled the
 *  province RASTER, ~24× coarser than the relief plate it was drawn over). */
export interface ProvinceGoodMask {
  good: string;
  /** World-cell origin of the sample grid — same coordinate system as
   *  `ProvinceTerrainCrop.ox/oy`, so both plates position through one transform. */
  ox: number;
  oy: number;
  /** World cells between samples. */
  stride: number;
  cols: number;
  rows: number;
  /** Belt value 0..255 at each SAMPLED world cell, row-major over `cols × rows`
   *  (0 = not covered). Absolute scale (D10), never per-good normalised. */
  q: number[];
  /** Sampled cells covered (`q >= COVERAGE_MIN`). */
  cells: number;
  /** F1 · the good's real extent within this province — a latitude-aware km² sum
   *  over the FULL-RESOLUTION belt, not the sampled grid above. */
  area_km2: number;
  /** Fraction of the province's own land the belt reaches. */
  land_share: number;
}

/** GOODS ATLAS (`campaign_good_atlas`) — everything about one good for the Atlas panel
 *  (the remade Codex). All read from live campaign state. */
export interface AtlasHub { hub: number; name: string; x: number; y: number; amount: number }
export interface AtlasHouse { house: number; name: string; is_guild: boolean; share: number; total_volume: number }
export interface AtlasFlow {
  from: number; to: number;
  from_x: number; from_y: number; to_x: number; to_y: number;
  amount: number;
}
export interface GoodAtlas {
  good: string;
  manufactured: boolean;
  total_produced: number;
  total_traded: number;
  avg_quality: number;
  /** 10 bins over quality 0..1 (count of producing cities per band). */
  quality_hist: number[];
  top_quality: AtlasHub[];
  producers: AtlasHub[];
  consumers: AtlasHub[];
  houses: AtlasHouse[];
  flows: AtlasFlow[];
}

export interface PoliticalCenter {
  x: number;
  y: number;
  power: number;      // 0..1 combined trade power
  rank: number;       // 0 = most powerful
  radius: number;     // (legacy) influence radius in world cells
  stars: number;      // power tier 1..5 — major hubs get 5 (Venice/Genoa)
  population: number;
  monopolies: string[];
  name: string;       // antique hub name (shown when the Hub-names overlay is on)
  emporium?: boolean; // one of the few greatest entrepôts — drawn RED
}

// ── DLC 3 · Finance, the Polis & Speculation ──────────────────────────────────

/** One ranked bubble driver in a polis's speculation reason-chain. */
export interface SpecDriver {
  key: string;     // "thin_float" | "cheap_money" | "leverage" | …
  label: string;   // "Thin float"
  weight: number;  // weighted contribution to the risk score (0..1)
  detail: string;  // generated clause naming the real house/good
}

/** The once-a-year speculation read for one polis (mirrors PoliticalCenter). */
export interface SpecCenter {
  hub: number;
  x: number;
  y: number;
  name: string;
  risk: number;          // 0..1 bubble risk
  stars: number;         // 1..5 tier
  tier: string;          // "LOW" | "MED" | "HIGH"
  pattern_tag: string;   // "tulip-like" | "company-bubble" | …
  drivers: SpecDriver[]; // ranked, largest weight first
  watch_goods: string[];
  year: number;
}

/** A polis as a politico-economic actor (treasury / tariffs / mint / council). */
export interface PolisBrief {
  hub: number;
  name: string;
  x: number;
  y: number;
  population: number;
  treasury: number;
  tariff_export: number;
  tariff_import: number;
  mint_fineness: number;       // 1.0 = full coin, < 1 = debased
  council: string;             // governing house ("—" if none)
  council_archetype: string;
  council_color: string;
  coin_name: string;           // the polis's named coin ("" = none)
  coin_trust: number;          // acceptance / trust 0..1
  coin_value: number;          // value index (≈1.2 strong, <1 debased)
  coin_issuer: string;         // council house whose arms ride the coin ("" → city)
  war_with: string;            // polis at war with ("" = peace)
}

// ── DLC 3.5 · Coin, Credit & Crashes ──────────────────────────────────────────

/** One coin in the world reserve-currency ranking. */
export interface CurrencyBrief {
  hub: number;
  city: string;
  coin_name: string;
  trust: number;        // acceptance 0..1
  fineness: number;     // 1 = full coin, < 1 = debased
  throughput: number;   // trade volume at the issuing city
  is_reserve: boolean;  // accepted abroad
  color: string;
  value: number;        // value index (≈1.2 strong agio, <1 debased)
  issuer: string;       // council house whose arms ride the coin ("" → city)
  circulating: number;  // money supply = Σ holders (throughput × share)
  held_in: number;      // how many settlements hold this coin
}

/** v2.0 · a MINT/polis fused into one card for the unified "Coin & Mints" tab —
 *  the civic polis (treasury, tariffs, council, war) plus its coin. `coin_name`
 *  empty = a council seat that mints no coin yet. */
export interface MintBrief {
  hub: number;
  city: string;
  x: number;
  y: number;
  population: number;
  // civic
  treasury: number;
  tariff_export: number;
  tariff_import: number;
  council: string;
  council_archetype: string;
  council_color: string;
  war_with: string;
  // coin
  coin_name: string;
  issuer: string;
  metal: string;         // "gold" | "silver" | "electrum" | "bronze"
  trust: number;
  fineness: number;
  value: number;
  exchange: number;      // v2.1 metal-aware intrinsic exchange value (silver = 1)
  strength: number;      // headline 0..100 (fineness × acceptance)
  throughput: number;
  is_reserve: boolean;
  circulating: number;
  held_in: number;
  abroad: number;        // holders outside the home market
  // v2.0 monetary loop + reform
  price_level: number;   // local CPI index (1.0 = par)
  bullion: string;       // "ample" | "tight" | "scarce" — coin-supply limiting factor
  has_mint: boolean;     // holds the right of the mint (charter)
  under_mandate: boolean; // honest-money mandate active (no debasement)
  reformed: boolean;      // has reformed its coinage at least once
  // B3 · civic public debt (the Monte)
  debt_principal: number; // principal owed to bondholders (0 = no public debt)
  debt_coupon: number;    // annual coupon rate paid
  debt_ratio: number;     // debt ÷ yearly throughput (≥3 = fiscal strain)
  debt_holders: number;   // number of patrician bondholders
}

/** A3 · one yearly point in a coin's biography (Money panel sparklines). */
export interface CoinSnapshot {
  year: number;
  fineness: number;
  trust: number;
  value: number;
  exchange: number;
  strength: number;
  price_level: number;
  circulating: number;
  metal: number;
  event: string;   // "" | "first" | "charter" | "debasement" | "reform" | "crash"
}

/** v2.0 · one dated entry in the monetary chronicle (Shocks timeline). */
export interface MonetaryEvent {
  year: number;
  tick: number;
  kind: string;   // coinage | reform | run | bank | crash
  city: string;
  value: number;
  text: string;
}

/** v2.0 · one coin in a holder's currency reserves (a donut slice). */
export interface ReserveSlice {
  coin_name: string;
  color: string;
  metal: string;
  share: number;    // 0..1
  primary: boolean; // the holder's main/settlement coin
  mint: boolean;    // the holder's own city mints this coin
}

/** v2.0 · one holder (city / bank / house) and its currency reserve composition. */
export interface ReserveHolder {
  kind: string;     // "city" | "bank" | "house"
  name: string;
  seat: string;     // home/seat city ("" for a city)
  total: number;    // reserves/wealth (grain-eq)
  slices: ReserveSlice[];
}

export interface ReservesPayload {
  cities: ReserveHolder[];
  banks: ReserveHolder[];
  houses: ReserveHolder[];
}

/** One settlement's use of a coin — for the coin-usage overlay + per-coin chart. */
export interface CoinUseCity {
  coin: number;          // issuing-mint hub id (which coin this city settles in)
  coin_name: string;
  city: number;          // the city hub id using it
  name: string;
  x: number;
  y: number;
  volume: number;        // trade settled in this coin at this city
  share: number;         // this coin's share of the city's basket 0..1
  mint: boolean;         // this city is the coin's own mint
  primary: boolean;      // this coin is the city's MAIN settlement currency
  reserve_reach: boolean; // a foreign reserve coin circulating here (held, not primary)
  color: string;         // stable per-coin colour (its council's arms)
}

/** One bank's balance sheet + reach. */
export interface BankBrief {
  name: string;
  seat: string;
  coin_name: string;   // the coin the bank banks in (its seat city's coin)
  coin_value: number;  // that coin's value (×grain) — to denominate the balance sheet
  owner: string;
  owner_idx: number;   // owning house index (match to HouseBrief.idx)
  color: string;
  founded_year: number;
  defunct: boolean;
  reserves: number;
  loans_out: number;
  real_estate: number;
  deposits: number;
  notes_issued: number;
  equity: number;
  reserve_ratio: number;
  n_loans: number;
  interest_earned: number;
  losses: number;
  stake_book: number;
  dividends_earned: number;
  bills_income: number;   // B4 · cumulative bills-of-exchange (FX-spread) income
  seat_x: number;
  seat_y: number;
  branches: string[];
  events: string[];
  history: BankSnapshot[];
  loans: BankLoanRow[];
  stakes: BankStakeRow[];
}

/** A yearly balance-sheet snapshot (drives the Bank panel charts). */
export interface BankSnapshot {
  year: number;
  reserves: number;
  loans: number;
  stakes: number;
  real_estate: number;
  deposits: number;
  notes: number;
  equity: number;
  interest_cum: number;
  dividends_cum: number;
  losses_cum: number;
}

/** One loan/deal on a bank's books, with agreement terms. */
export interface BankLoanRow {
  borrower: string;
  borrower_kind: string; // "house" | "guild" | "polis"
  purpose: string;       // "trade" | "guild_factory" | "guild_civic" | "treasury" | "colony"
  principal: number;
  outstanding: number;
  rate: number;          // monthly
  start_year: number;
  term_years: number;
}

/** One equity stake a bank holds in a manufactory. */
export interface BankStakeRow {
  works: string;
  good: string;
  share: number;
  basis: number;
}

/** A regional financial crash record. */
export interface CrashRecord {
  year: number;
  origin_hub: number;
  origin_name: string;
  component: number;
  cities_hit: number;
  banks_failed: number;
  cause: string;
  text: string;
}

/** A polis's running treasury books (City Finances view); `prev` = last year. */
export interface CityFinance {
  year: number;
  tax_trade: number;
  tax_estate: number;
  tax_manufacture: number;
  tax_wealth: number;
  seigniorage: number;
  war_levy: number;
  reparations_in: number;
  spent_civic: number;
  spent_war: number;
  spent_works: number;
  spent_health?: number; // hospices / quarantine (public health)
  reparations_out: number;
  prev?: CityFinance | null;
}

/** One active economic war. */
export interface WarBrief {
  a: string;
  b: string;
  start_year: number;
  years: number;
  chest_a: number;
  chest_b: number;
  levies: number;
  /** §3.4e · forced levies each side raised from its own resident houses. */
  levies_a?: number;
  levies_b?: number;
  cause: string;
  /** CITY_PROVINCE_WAR_PLAN.md §3.4a · bidirectional war score, −100..100 — positive
   *  favours `a`. ±100 ends the war outright. */
  score: number;
  /** §3.4a · quarterly rounds fought so far (of the round cap's backstop). */
  round: number;
  /** §3.4b · what the aggressor is fighting for, as a phrase. */
  goal_label: string;
  /** §3.4c · for a house-driven war, the house whose feud escalated into it. */
  backer_house_name?: string | null;
  /** §3.4a · the quarterly rounds as a battle history (most recent last). */
  battles?: WarBattle[];
  /** Belligerent seats (world cell coords): a = ATTACKER (red), b = defender (blue). */
  a_x?: number; a_y?: number; b_x?: number; b_y?: number;
}

/** One quarterly round of a war — a "battle" in the panel's history. */
export interface WarBattle {
  round: number;
  year: number;
  /** Side the round favoured: 0 = a (attacker), 1 = b (defender). */
  favored: number;
  delta: number;
  score_after: number;
  decisive: boolean;
}
/** A concluded war (the log). */
export interface WarRecord {
  start_year: number;
  end_year: number;
  a_name: string;
  b_name: string;
  winner: string;
  loser: string;
  reparations: number;
  levies_total: number;
  cause: string;
  text: string;
}
export interface WarsPayload {
  active: WarBrief[];
  log: WarRecord[];
}

/** Phase 6 · one plague-struck city inside an epidemic. */
export interface PlagueCityBrief {
  hub: number;
  x: number;
  y: number;
  name: string;
  deaths: number;
  ill?: number;       // SIR · fell ill in this strike (>= deaths)
  recovered?: number; // SIR · fell ill and survived (ill - deaths)
  pop: number;      // survivors at the strike
  active: boolean;  // still under quarantine
  from_name: string; // carried from (""=spontaneous origin)
  year: number;     // year struck
  order: number;    // spread step (0 = origin)
}

/** Phase 6 · an epidemic = a contagion chain (cities sharing an outbreak). */
export interface EpidemicBrief {
  id: number;
  name: string;
  origin_name: string;
  start_year: number;
  end_year: number;
  active: boolean;
  total_dead: number;
  total_ill?: number;       // SIR · total fell ill across the outbreak
  total_recovered?: number; // SIR · total recovered across the outbreak
  /** 1 = Great Plague (rare, spreads ~4000 km along the lanes), 2 = Regional (reaches
   *  one further city), 3 = Local outbreak (stays put). */
  category: number;
  /** Named disease (Bubonic Plague, Cholera, Malaria, …) + transmission mode. */
  disease?: string;
  transmission?: string;
  cities: PlagueCityBrief[]; // in spread order (origin first)
}

/** Phase 6 · one craft guild (Guilds & Crafts panel + map). */
export interface GuildBrief {
  hub: number;
  x: number;
  y: number;
  city: string;
  good: number;
  good_name: string;
  quality: number;
  output: number;
  strength: number;
  hall: boolean;
  luxury: boolean;
  exceptional: boolean;
  brand: string;    // "Veyra cloth" when exceptional, else ""
  culture: string;
}

/** Phase 6 · a notable figure (Great Lives roster). */
export interface FigureBrief {
  name: string;
  role: string;
  hub: number;
  x: number;
  y: number;
  city: string;
  good_name: string;
  born_year: number;
  died_year: number;
  alive: boolean;
}

/** Phase 6 · a landmark / place of note. */
export interface LandmarkBrief {
  hub: number;
  x: number;
  y: number;
  city: string;
  kind: string; // wonder | temple | fair | guildhall
  label: string;
  detail: string;
}

/** Phase 7 · a link between two houses (marriage alliance or feud). */
export interface HouseLink {
  a_name: string;
  b_name: string;
  a_hub: number;
  b_hub: number;
  ax: number;
  ay: number;
  bx: number;
  by: number;
  a_city: string;
  b_city: string;
}

/** Phase 7 · dynasties: marriage alliances + feuds between houses. */
export interface DynastiesPayload {
  alliances: HouseLink[];
  feuds: HouseLink[];
}

/** One leg of a city's carrying trade ("transit"). */
export interface TransitRow {
  merchant: string;
  is_guild: boolean;
  color: string;
  good: string;
  amount: number;
  value: number;
  from_name: string;
  to_name: string;
  sea: boolean;
  coin: string;    // "" → barter
  barter: string;  // e.g. "~3.2 wheat/unit"
}

export interface SchematicBuilding { label: string; effect: string }
export interface SchematicEstate { label: string; tier: number; owner: string; good: string }
/** One city's blueprint: buildings, estates, bank presence and coin. */
export interface CitySchematic {
  hub: number;
  name: string;
  x: number;
  y: number;
  population: number;
  coin_name: string;
  coin_trust: number;
  coin_metal: string;   // "gold" | "silver" | "electrum" | "bronze"
  council: string;
  buildings: SchematicBuilding[];
  estates: SchematicEstate[];
  banks_seated: string[];
  bank_branches: string[];
}

export interface TradeRegion {
  id: number;
  name: string;
  x: number;
  y: number;
  production: number[]; // per good (matches TradeMatrix.goods order)
  demand: number[];
  net: number[];
}

export interface TradeFlow {
  from: number;
  to: number;
  good: number;
  good_name: string;
  weight: number;
  points: [number, number][];
}

export interface TradeTrunk {
  points: [number, number][]; // [from, to] world coords, ordered source -> consumer
  volume: number;
  good: number;               // dominant good index, or -1
  road: string;               // corridor name for major trunks ("Spice Road"); else ""
}

export interface TradeMatrix {
  regions: TradeRegion[];
  flows: TradeFlow[];
  trunks: TradeTrunk[];
  goods: string[];
}

export interface RiverData {
  points: [number, number][];
  width: number;
  major?: boolean; // long trunk river → darker render shade
  navigable?: boolean;
  mouth_kind?: number; // 0 plain, 1 delta, 2 estuary
  delta?: [number, number][];
  tributary?: boolean; // ends at a confluence with a larger stream (not the sea)
  order?: number; // Strahler-ish stream order (1 = headwater creek)
  meander?: number; // 0..1 render meander scale (0 = steep/straight, 1 = flat floodplain)
  /** True meander geometry — a smoothed, sub-cell render polyline (cell-index
   *  coords, same convention as `points`) computed physically in the backend
   *  (winds on flat lowlands, straight on steep headwaters, clamped to the valley).
   *  Drawn in place of the cosmetic meander when present. Empty/absent on old
   *  saves → the frontend falls back to `meanderPath(points)`. */
  render?: [number, number][];
  /** Braided anabranches (thin secondary channels) on a great river's widest,
   *  flattest reaches — drawn faint beside the trunk. */
  braids?: [number, number][][];
}

/** A city sitting on a river reach (Hydrology dashboard). Mirrors Rust `RiverCityInfo`. */
export interface RiverCityInfo {
  name: string;
  x: number;
  y: number;
  size: string;
  dist_from_mouth_km: number;
}

/** One river in the system tree — a trunk (root) or a nested tributary — with its
 *  hydrological stats, elevation profile, cities and children. Mirrors Rust `RiverNode`. */
export interface RiverNode {
  id: number; // index into the world's rivers array (look up its cell path)
  name: string; // unique, culture-styled river name (dedup'd across the world)
  order: number;
  navigable: boolean;
  tributary: boolean;
  mouth_kind: number; // 0 plain, 1 delta, 2 estuary
  length_km: number;
  drop_m: number;
  source_elev_m: number;
  mouth_elev_m: number;
  avg_slope_m_per_km: number;
  discharge_m3s: number;
  max_width_m: number;
  max_depth_m: number;
  navigable_km: number;
  source_kind: string; // alpine | highland | hills | lowland | bog
  source_x: number;
  source_y: number;
  mouth_x: number;
  mouth_y: number;
  mid_x: number;
  mid_y: number;
  join_km: number; // distance downstream along the parent to this confluence (km)
  trib_total: number;
  city_total: number;
  counterpart: string; // Earth counterpart (roots only)
  regime: string;   // Köppen flow-regime phrase ("perennial temperate", …)
  fish: string;     // fish assemblage prose (real Earth taxa)
  riparian: string; // bankside vegetation phrase
  water: string;    // water character (clarity/sediment/productivity)
  wildlife: string; // charismatic riverine wildlife beyond fish
  story: string;    // trunk: SUMMARY lede of the whole course · tributary: short account
  climate_journey: string; // biomes crossed source→mouth ("temperate forest, then …")
  zones: RiverZone[];      // upper / middle / lower-delta reaches (trunks only)
  species: FishSpecies[]; // signature fish, one per river zone the reach spans
  profile: number[]; // elevation (m), source → mouth
  cities: RiverCityInfo[];
  children: RiverNode[];
}

/** A tributary joining a river at a given distance downstream. Mirrors Rust. */
export interface TribJoin { name: string; km: number; }

/** One reach of a trunk river — upper / middle / lower-delta. Mirrors Rust `RiverZoneOut`. */
export interface RiverZone {
  kind: "upper" | "middle" | "delta";
  label: string;      // "Upper river" | "Middle river" | "Delta / Estuary / Lower course"
  start_km: number;
  end_km: number;
  biome: string;      // dominant biome of this reach
  koppen: string;     // dominant Köppen code (tooltip)
  character: string;  // short width/speed phrase
  story: string;
  tributaries: TribJoin[];
  species: FishSpecies[]; // the full fish assemblage that lives in THIS reach
}

/** One classified lake with its limnological + ecological profile (Hydrology
 *  dashboard, Lakes tab). Mirrors Rust `LakeNode`. */
export interface LakeNode {
  id: number;
  name: string;
  kind: "rift" | "crater" | "salt" | "glacial" | "tropical" | "lowland" | "tarn";
  kind_label: string;
  area_km2: number;
  max_depth_m: number;
  mean_depth_m: number;
  elev_m: number;
  volume_km3: number;
  endorheic: boolean;   // terminal salt lake (no outflow)
  salinity_ppt: number;
  analog: string;       // real-world analog lake
  thermal: string;      // mixing regime
  water: string;        // trophic / clarity
  fish: string;
  wildlife: string;
  endemism: string;
  blurb: string;        // one-line flavour description
  species: FishSpecies[]; // signature fish (variable roster by type × band)
  story: string;        // unique multi-sentence NatGeo-style account
  inflows: string[];    // feeder river names
  outflow: string;      // draining river name ("" = terminal)
  cx: number;
  cy: number;
  area_cells: number;
}

export interface LakeData {
  cells: [number, number][];
  elevation: number;
  /** 0 = normal depression-filled basin · 1 = oxbow/backwater cut off from a
   *  meandering river (drawn as a still green-blue backwater). */
  kind?: number;
  /** True terminal salt lake (arid, no outflow) — tinted pink on the map. */
  endorheic?: boolean;
  /** Approximate salinity (ppt); ~0.2 fresh, 12-120+ for a salt lake. Drives the
   *  brackish→saline→hypersaline pink tint. */
  salinity_ppt?: number;
}

/** A signature fish species on a river reach. `slug` keys an illustration at
 *  `/fish/<slug>.png` (drop your generated plates in `public/fish/`). Mirrors Rust
 *  `FishSpeciesOut`. */
export interface FishSpecies {
  slug: string;
  name: string;
  binomial: string;
  zone: number;      // 0 upper · 1 middle · 2 lower/delta
  real: string;      // real-world model species
  blurb: string;
}

export interface Settlement {
  id: string;
  x: number;
  y: number;
  name: string;
  size: "capital" | "city" | "town" | "village" | "outpost";
  population: number;
  score: number;
  culture?: string; // people/culture governing the site ("Norse", …)
  region?: string;  // region / homeland name ("Vexillia")
  site?: string;    // "coast" | "river" | "hills" | "plain" | "port" (step 7a junction site)
  dead?: boolean;   // abandoned/collapsed → drawn as a † ruin cross, not a dot
  isNew?: boolean;  // founded this campaign, still young → gold founding star
  hubClass?: number; // 0 ordinary · 1 trade hub · 2 entrepôt (campaign, earned live)
}

/** #26 · a named geographic feature. Mirrors the Rust `Toponym` struct.
 *  `desert`/`forest`/`tundra` are large biome-region subnames — a contiguous
 *  patch of the same broad biome, named separately from the culture-hearth
 *  `region` so they can be toggled independently. */
export interface Toponym {
  kind: "river" | "mountain" | "lake" | "region" | "desert" | "forest" | "tundra";
  name: string;
  x: number;
  y: number;
}

/** #29 · one year's merchant-house wealth-inequality reading. */
export interface InequalityPoint {
  year: number;
  gini: number;
  active: number;
  mean_wealth: number;
  top10_share: number;
}
/** #29 · wealth inequality + social mobility snapshot (campaign sim). */
export interface InequalitySnapshot {
  active: boolean;
  year: number;
  gini_now: number;
  top10_share_now: number;
  active_houses: number;
  defunct_houses: number;
  founded_total: number;
  rank_churn: number;
  series: InequalityPoint[]; // oldest → newest
}

/** A point-to-point journey over the shared coarse cost grid (#23 itinerary).
 *  Mirrors the Rust `Itinerary` struct in query_commands.rs. */
export interface Itinerary {
  points: [number, number][]; // routed polyline in world cells
  reachable: boolean;
  km: number;
  land_km: number;
  river_km: number;
  sea_km: number;
  days_foot: number;
  days_horse: number;
  days_cart: number;
  dominant_mode: number; // 0 land · 1 sea · 2 river
}

/** One people's territory for the Peoples overlay (compute_culture_regions). */
export interface CultureRegion {
  cells: [number, number][]; // coarse cell top-left world coords
  cell_size: number;
  x: number;                 // label centroid
  y: number;
  color: [number, number, number];
  label: string;             // people / region name
  culture: string;           // kit name
}

export interface ShelfParams {
  width: number;       // 1-20 cells
  noise: number;       // 0-1
  depthProfile: number; // 0-1
  dropoff: number;     // 1-20 cells
}

export interface SimRiversResult {
  modified: [number, number][];
  rivers: RiverData[];
  lakes: LakeData[];
}

export interface SimRunAllResult {
  modified: [number, number][];
  rivers: RiverData[];
  lakes: LakeData[];
  settlements: Settlement[];
}

export interface SimSettlementsResult {
  modified: [number, number][];
  settlements: Settlement[];
}

/** A good a province's land can yield, with an environmental-suitability QUALITY
 *  (0..1) — never an amount. Mirrors the Rust `ProvinceGood`. */
export interface ProvinceGood {
  good: number;    // good index (→ GOOD_DEFS)
  quality: number; // 0..1 suitability → quality stars
  /** World rank for this good, 1 = the finest land on the map. 0/absent on worlds
   *  generated before ranking existed. */
  rank?: number;
  /** How many provinces yield this good at all (the "of N" in "#3 of N"). */
  of?: number;
}

/** What separates two neighbouring provinces where they touch. Mirrors the Rust
 *  `BORDER_*` constants. */
export const BORDER_OPEN = 0;
export const BORDER_RIDGE = 1;
export const BORDER_RIVER = 2;
export const BORDER_LAKE = 3;

/** One shared frontier: which neighbour, how long, and what natural feature runs
 *  along it. Mirrors the Rust `ProvinceBorder`. */
export interface ProvinceBorder {
  neighbor: number;
  cells: number;  // shared border length in cells
  kind: number;   // BORDER_*
}

/** A province — a cost-flood + feature-snap administrative region. Mirrors the Rust
 *  `Province` struct (serde default snake_case, so keys stay snake_case).
 *
 *  Everything below `settlements` is serde-defaulted on the Rust side and therefore
 *  OPTIONAL here: a world saved before these stats existed loads with them absent,
 *  and the panels must degrade rather than blank. */
export interface Province {
  id: number;
  name: string;            // its OWN generated name (variable length), not the seat's
  seat_x: number;
  seat_y: number;
  cells: number;           // area in cells
  area_km2: number;        // latitude-aware real area
  island: number;
  neighbors: number[];
  koppen: number;          // plurality climate zone
  elevation_class: number; // 0 lowland · 1 hill · 2 upland
  mean_fertility: number;
  coastal: boolean;
  goods: ProvinceGood[];
  /** #9 · real per-good QUALITY 0..1 over EVERY good (best-patch suitability), so the
   *  panel shows a differentiated quality for all goods, not just the top-6 shortlist.
   *  Empty on worlds generated before this field — the UI falls back to the shortlist. */
  good_quality?: number[];
  culture: string;         // plurality over the province's cells (campaign may shift it)
  rural_pop: number;       // baseline countryside population
  analog: string;          // "looks most like…" real-world regions
  settlements: string[];   // settlement ids inside (seat first)

  // ── appended + serde-defaulted (see the note above) ──
  /** Climate mix: share of cells per Köppen code, largest first (top 4). */
  koppen_shares?: [number, number][];
  elev_min_m?: number;
  elev_mean_m?: number;
  elev_max_m?: number;
  relief_m?: number;       // elev_max_m − elev_min_m
  temp_mean?: number;      // °C
  precip_mean?: number;    // mm/yr
  season_amp?: number;     // °C, seasonal half-range
  arid_frac?: number;      // share of cells in a desert/steppe Köppen class
  disease_mean?: number;   // 0..1
  coast_cells?: number;
  river_cells?: number;
  navigable_river?: boolean;
  lake_cells?: number;     // lake cells on the province's own shore
  /** Peoples present, share of cells each, plurality first. */
  culture_shares?: [string, number][];
  food_capacity?: number;  // Σ per-cell food capacity
  rural_cap?: number;      // food_capacity as a population ceiling → saturation
  /** Neighbours with shared length + the feature dividing them, longest first. */
  neighbors_detail?: ProvinceBorder[];
  /** Label anchor — the province's POLE OF INACCESSIBILITY (centre of its largest
   *  inscribed circle), which is always inside the province, unlike a centroid. Falls
   *  back to the seat on worlds generated before this existed. */
  label_x?: number;
  label_y?: number;
  /** Radius of that inscribed circle in cells — how much room the name has, so the
   *  renderer can size the label to the province rather than to the zoom level. */
  label_r?: number;
}

/** Live per-province campaign state (read-only): baseline rural + current urban. */
export interface ProvinceLive {
  id: number;
  rural_pop: number;
  urban_pop: number;
  hub_count: number;
  net_migration: number; // <0 = countryside is a source (people leaving for cities)
}

/** One stat row for a building's hover card. */
export interface PStat { label: string; value: string }

/** A building standing in a province (kind: 0 estate · 1 manufactory · 2 warehouse ·
 *  3 bank · 4 mint), with world-cell position + full hover stats. */
export interface PBuilding { kind: number; name: string; x: number; y: number; stats: PStat[] }

/** A live settlement in a province (for the mini-map + list). */
export interface PSettlement {
  name: string; x: number; y: number; population: number;
  seat: boolean; hub_class: number; dev_tier: number;
}

/** Full detail of one province for the subwindow. */
export interface ProvinceDetail {
  id: number;
  rural_pop: number;
  urban_pop: number;
  net_migration: number;
  settlements: PSettlement[];
  buildings: PBuilding[];
}

/** A cropped elevation/land/biome sample grid over one province's bounding box —
 *  the survey plate's real "relief" base layer (§2.3). `elevation`/`land`/`biome`
 *  are `cols*rows`, row-major; sample (c, r)'s world-cell position is
 *  `(ox + c*stride, oy + r*stride)`. */
export interface ProvinceTerrainCrop {
  ox: number; oy: number; stride: number;
  cols: number; rows: number;
  elevation: number[];
  land: number[];   // 1 = land, 0 = sea/lake
  biome: number[];  // raw sim::biome code, 0 = unclassified/sea
}

/** Result of `sim_generate_provinces`: the province list + a downsampled per-cell
 *  id raster for the map overlay (`4294967295` = sea/no-data; ids are u32). */
export interface SimProvincesResult {
  provinces: Province[];
  raster: number[];
  raster_w: number;
  raster_h: number;
  grid_w: number;
  grid_h: number;
  /** Full-resolution province-id map, run-length encoded as [val, count, …]. */
  raster_rle: number[];
}

// ─────────────────────────────────────────────────────────────────────────────
//  House Dossier — stability gauges + the feud board
//  (mirrors commands/campaign_commands/read_houses.rs)
// ─────────────────────────────────────────────────────────────────────────────

/** One stability gauge. `phrase` is the product — a raw 0..1 tells a player nothing. */
export interface Gauge {
  /** "solvency" | "liquidity" | "exposure" | "succession" | "cohesion" */
  key: string;
  label: string;
  /** 0..1, higher is healthier. */
  score: number;
  /** 1..5 pips, for comparing houses at a glance. */
  pips: number;
  phrase: string;
  /** True when this is the gauge to worry about. */
  warn: boolean;
}

/** One liability line behind the Solvency gauge. */
export interface Liability { label: string; amount: number; note: string }

/** The House Dossier header: five gauges plus the liabilities behind them. */
export interface HouseStability {
  idx: number;
  name: string;
  gauges: Gauge[];
  monthly_burn: number;
  liquid: number;
  liabilities: Liability[];
  liabilities_total: number;
  /** Months already in the red (0 = solvent) and the limit before bankruptcy. */
  debt_months: number;
  debt_limit: number;
  top_good_share: number;
  top_good: string;
  top_city_share: number;
  top_city: string;
  head_years: number;
  head_span_years: number;
  feuds_live: number;
  feuds_hot: number;
}

/** One flare in a feud's history. */
export interface FeudFlareRow {
  year: number;
  stage: string;
  loser: string;
  cost: number;
  text: string;
}

/** A feud: who, why, how hot, and how it ended. */
export interface FeudRow {
  a: number;
  b: number;
  a_name: string;
  b_name: string;
  a_color: string;
  b_color: string;
  /** Already in prose — "the same trade", "a contested council". */
  cause: string;
  /** "cold rivalry" | "open feud" | "trade war" | "vendetta" */
  stage: string;
  stage_idx: number;
  intensity: number;
  good: string;
  city: string;
  started_year: number;
  years: number;
  flares: number;
  damage_a: number;
  damage_b: number;
  /** "running" | "arbitrated" | "sealed by marriage" | "ended in ruin" | "cooled" */
  outcome: string;
  running: boolean;
  ended_year: number;
  log: FeudFlareRow[];
}

// ─────────────────────────────────────────────────────────────────────────────
//  Province LAND state (FIX_PLAN B1)
//  (mirrors commands/campaign_commands/province.rs)
// ─────────────────────────────────────────────────────────────────────────────

/** A house holding estates in a province — drives the tenure plate's colour blocks. */
export interface ProvinceHolder {
  house: number;
  name: string;
  color: string;
  estates: number;
}

/** A land improvement under way. */
export interface ProvinceWorkRow {
  /** 0 clearance · 1 drainage · 2 irrigation · 3 road */
  kind: number;
  label: string;
  progress: number;
  years_left: number;
  yearly_cost: number;
  funder: string;
  stalled: boolean;
}

/** One yearly sample — the series the province plate's time slider scrubs. */
export interface ProvinceLandSample {
  year: number;
  rural: number;
  urban: number;
  forest: number;
  arable: number;
  pasture: number;
  irrigated: number;
  soil: number;
  unrest: number;
  surplus: number;
}

export interface ProvinceEventRow { year: number; kind: string; text: string }

/** A province's mutable land state — everything that CHANGES over a campaign. */
export interface ProvinceLand {
  id: number;
  forest: number;
  arable: number;
  pasture: number;
  irrigated: number;
  waste: number;
  soil: number;
  unrest: number;
  rural: number;
  rural_cap: number;
  urban: number;
  saturation: number;
  surplus: number;
  revenue: number;
  arrears: number;
  tax_rate: number;
  tax_max: number;
  /** [civic/crown, house/noble, temple, common] */
  tenure: [number, number, number, number];
  holders: ProvinceHolder[];
  holder_hub: number;
  /** Phase 5 · a HOUSE whose writ runs here instead of a city's, -1 = the ordinary case. */
  holder_house: number;
  /** The seat city's name, or the holding house's name when one holds this province. */
  holder_name: string;
  works: ProvinceWorkRow[];
  history: ProvinceLandSample[];
  events: ProvinceEventRow[];
}

/** §2.5 · one good's exploitation reading in a province — a pure derived read
 *  except `depletion`, which is the one piece of state that persists. */
export interface ProvinceGoodExploit {
  good: number;
  potential: number;
  actual: number;
  /** actual / potential — below 1 is slack, above 1 is over-worked (the soft cap). */
  exploitation: number;
  /** 0..1 accumulated overexploitation pressure eroding potential. */
  depletion: number;
  /** Share of `actual` that leaves the province via trade rather than being
   *  consumed by the very population that produced it. */
  market_share: number;
}

/** One trader's slice of a province's organized commerce. */
export interface ProvinceTradeHolder {
  /** 0 = private house, 1 = civic guild, 2 = "others" aggregate. */
  kind: number;
  /** House index, or −1 for the aggregate. */
  holder: number;
  name: string;
  volume: number;
  /** Fraction 0..1 of the province's total organized trade. */
  share: number;
  /** The leading house clearing the realm-eligibility threshold. */
  eligible: boolean;
}

/** One city's slice of a province's commerce. */
export interface ProvinceTradeCity {
  hub: number;
  name: string;
  volume: number;
  share: number;
}

/** Per-good tonnage crossing the province boundary in one direction (last full year). */
export interface ProvinceTradeGood {
  good: number;
  amount: number;
}

/** Province trade join — the circular-diagram payload for the province view. */
export interface ProvinceTrade {
  /** Total organized (house + guild) trade volume attributed to this province. */
  total: number;
  by_holder: ProvinceTradeHolder[];
  by_city: ProvinceTradeCity[];
  /** Goods exported out of the province last year, largest first. */
  exports: ProvinceTradeGood[];
  /** Goods imported from outside last year, largest first. */
  imports: ProvinceTradeGood[];
  export_total: number;
  import_total: number;
  /** House that commands ≥ threshold and could proclaim a realm here (−1 = none). */
  controller_house: number;
  controller_name: string;
  controller_share: number;
  /** The eligibility threshold (e.g. 0.20), so the UI needs no local copy. */
  control_threshold: number;
}

/** One free province's realm-eligibility reading (see RealmWatch). */
export interface RealmWatchEntry {
  province_id: number;
  /** Strongest non-guild trader here, or −1 if none. */
  house: number;
  house_name: string;
  /** Its share (0..1) of the province's total organized trade. */
  share: number;
  seat_hub: number;
  seat_name: string;
  /** True when this province WOULD crown at the next year turn. */
  eligible: boolean;
  /** "" when eligible; otherwise the exact reason it cannot crown yet. */
  reason: string;
}

/** The live "why has no realm formed yet?" diagnostic — mirrors the yearly
 *  proclamation pass's own eligibility test over every free province. */
export interface RealmWatch {
  realms_exist: boolean;
  year: number;
  year_floor: number;
  control_threshold: number;
  eligible_count: number;
  entries: RealmWatchEntry[];
  summary: string;
}

/** #9 · One good a province COULD yield (opportunity view), with richness. */
export interface ProvinceGoodPotential {
  good: number;
  name: string;          // good id (map to label/emoji via GOOD_DEFS)
  potential: number;     // live potential yield (belt × land-use × yield scale)
  belt: number;          // frozen belt richness 0..1 (how good the LAND is)
  actual: number;        // current production (0 = untapped)
  is_deposit: boolean;   // an ore/mineral good (richness = deposit grade)
  is_marine: boolean;    // a sea/coast good — belongs on the province coast, not inland
  mean_grade: number;    // deposit goods: mean working grade 0..1
  workings: number;      // deposit goods: number of ore workings in the province
  best_depth: number;    // deposit goods: deepest present (0 surface … 3 flooded)
  /** CLAUDE.md §8.19 (goods localities, shipped) Slice 7 — the non-mineral counterpart of mean_grade/
   *  workings: this good has at least one terroir LOCALITY in the province. */
  has_locality: boolean;
  mean_locality_grade: number; // 0..1, when has_locality
  locality_count: number;
  /** WORLD_AND_TRADE_MASTER_PLAN.md Part III §8.2 — the served `grade_label`
   *  vocabulary (coarse/ordinary/good/fine/exquisite), off whichever richness
   *  this good actually carries here. Empty when absent. */
  grade_word: string;
}

/** #9 · One ore working located in a province (real cell coords) for the minimap. */
export interface ProvinceDepositDot {
  good: string;
  x: number;
  y: number;
  grade: number;
  extent: number;
  depth: number;
}

/** CLAUDE.md §8.19 (goods localities, shipped) Slice 6 · one terroir locality located in a province
 *  (real cell coords), for the survey-plate goods layer — the non-mineral
 *  counterpart of `ProvinceDepositDot`. */
export interface ProvinceLocalityDot {
  good: string;
  x: number;
  y: number;
  grade: number;
  extent: number;
  radius_km: number;
  /** Empty unless the locality cleared the "notable" quality threshold. */
  name: string;
  river_fed: boolean;
  /** D4 · this patch sits in the SEA off the province's coast, not on its land. It
   *  is listed so the survey plate can annotate the adjacent water — the province
   *  gains NO maritime territory, and nothing counts a sea locality toward its land
   *  use, tenure, harvest or revenue. */
  sea: boolean;
}

/** #9 · A province's full goods picture: potential goods + ore workings + localities. */
export interface ProvincePotential {
  goods: ProvinceGoodPotential[];
  deposits: ProvinceDepositDot[];
  localities: ProvinceLocalityDot[];
}

/** REALM_AND_GOVERNMENT_PLAN.md R1 · one proclaimed realm, made into a territory
 *  for the map. `compute_states` now reads a REAL persisted `Realm` (`sim.realms`
 *  + `prov_realm`) rather than deriving a "state" fresh from city tiers each call
 *  — a capital's tier later dropping no longer erases it from the map. */
export interface StateRegion {
  /** Index into the sim's realm list — pass to `campaignGetRealmFamily`. */
  id: number;
  capital_hub: number;
  name: string;
  /** The ruler's style — "King", "Sovereign" — placeholder vocabulary; see the
   *  backend's `tick/realms.rs` module doc for the culture-derived namer this
   *  stands in for. */
  title: string;
  /** [r,g,b] — distinct from any house's heraldic colour (own hue phase), keyed on
   *  the realm's own id so a future capital move never reassigns the colour. */
  color: [number, number, number];
  /** Coarse cell top-left world coords, same shape CultureRegion uses. */
  cells: [number, number][];
  cell_size: number;
  /** Label centroid. */
  x: number;
  y: number;
  province_count: number;
  /** The province ids this realm holds. The overlay tints exactly these cells of
   *  the province raster, so a realm's border IS the province border. */
  province_ids: number[];
  /** 0 city-state · 1 kingdom · 2 great power · 3 hegemon. */
  rank: number;
  founded_tick: number;
  /** The ruling dynasty's name — the house that proclaimed this realm, ELEVATED
   *  rather than dissolved. */
  ruling_house: string;
  legitimacy: number;
  cohesion: number;
  treasury: number;
  debts: number;
}

/** R2 (`REALM_AND_GOVERNMENT_PLAN.md` §3.7) · one member of a realm's dynasty — a
 *  REAL person with a real age, not a merchant house's regenerated `Kin` snapshot.
 *  `idx` is a stable index into the realm's own family list; `father`/`mother`/
 *  `spouse` are indices into that SAME list, or -1. */
export interface PersonBrief {
  idx: number;
  name: string;
  female: boolean;
  age: number;
  alive: boolean;
  father: number;
  mother: number;
  spouse: number;
  is_ruler: boolean;
  is_regent: boolean;
  epithet: string;
  /** 0 if this person has never reigned. */
  reign_years: number;
}

// ── The campaign library (mirrors commands/campaign_library.rs) ──

/** One `.campaign` file as the library lists it. Read from each save's small header,
 *  never from its simulation blob. */
export interface CampaignFileInfo {
  path: string;
  file_name: string;
  name: string;
  world_name: string;
  year: number;
  tick: number;
  hubs: number;
  houses: number;
  /** Unix seconds; falls back to the file's mtime when the save carries no header. */
  saved_at: number;
  size_bytes: number;
  /** True when this save's world matches the world currently open. */
  world_match: boolean;
  /** False for pre-header saves — the year shown was recovered by scanning. */
  has_header: boolean;
}

/** Whether the open world can start a campaign, and what a rebuild has to work with
 *  (mirrors campaign_commands::WorldHumanLayerStatus). */
export interface WorldHumanLayerStatus {
  has_settlements: boolean;
  settlement_count: number;
  has_economy: boolean;
  hub_count: number;
  can_start_campaign: boolean;
  has_provinces: boolean;
  /** Present ⇒ a rebuild reproduces the SAME towns with the SAME ids. */
  settlements_seed: number | null;
  settlements_realism: number | null;
  settlements_max: number | null;
}

/** What `repair_province_settlements` changed. */
export interface ProvinceRepairReport {
  provinces: number;
  provinces_changed: number;
  settlements_attached: number;
  settlements_orphaned: number;
}
