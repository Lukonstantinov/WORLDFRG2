export interface WorldMeta {
  name: string;
  grid_width: number;
  grid_height: number;
  tile_size: number;
  /** Equator position as a fraction of height from the top (0.5 = centered). */
  equator_offset: number;
  /** Latitude expansion factor (1 = default; >1 stretches bands, cropping poles). */
  lat_scale: number;
  /** Line-spacing ratio (gap 30→60 ÷ gap 0→30); shared with the simulation. */
  lat_ratio: number;
  /** True once the world's geography is finalized (campaign steps unlocked). */
  frozen: boolean;
}

/** What open_world returns: the world plus any campaign data the file carried. */
export interface OpenWorldResult {
  meta: WorldMeta;
  /** Pre-split single-file save — offer to split it into world + campaign. */
  legacy: boolean;
  campaign_name: string | null;
  /** JSON-encoded step-completion maps persisted by set_progress. */
  world_progress: string | null;
  campaign_progress: string | null;
}

export interface CampaignInfo {
  name: string;
  /** False when the campaign was saved against a different/refinalized world. */
  world_match: boolean;
  /** JSON step-completion map for the campaign wizard (steps 7-10). */
  campaign_progress: string | null;
}

export interface TileResponse {
  tx: number;
  ty: number;
  layer: string;
  version: number;
  rgba: string; // base64-encoded RGBA pixels
}

export interface CellInfo {
  wx: number;
  wy: number;
  grid_width: number;
  grid_height: number;
  terrain: string;
  elevation: number;
  sea_depth: number;
  temperature: number;
  precipitation: number;
  koppen: number;
  biome: string;
  soil_type: number;
  fertility: number;
  fishery: number;
  plate_index: number;
  is_volcanic: boolean;
  is_shelf: boolean;
  wind_vx: number;
  wind_vy: number;
  current_vx: number;
  current_vy: number;
  current_type: number;
  distance_to_ocean: number;
  salinity: number;    // PSU
  shark_risk: number;  // 0..1
  shipworm_risk: number; // 0..1
  storm_risk: number;  // 0..1
  reef_risk: number;   // 0..1
  disease_risk: number; // 0..1
  goods: { name: string; amount: number }[];
}

// ── Editable trade-good specs (mirror sim/goods_spec.rs) ──
export type GoodDomain = "marine" | "coastal" | "continental" | "island";
export type GoodDistribution = "global" | "local" | "deposits" | "manufactured";

/** One recipe input of a Manufactured good: `qty` units of `good` per 1 output. */
export interface RecipeInput {
  good: string;
  qty: number;
}

export interface GoodEnvelope {
  climate: [number, number][];        // (koppen code, weight)
  temp?: [number, number] | null;     // bell center, width (°C)
  precip?: [number, number, number] | null;     // band lo, hi, edge (mm/yr)
  elevation?: [number, number, number] | null;  // band lo, hi, edge (0..1)
  abs_lat?: [number, number, number] | null;    // band lo, hi, edge (deg)
  fertility: number;
  coast_bonus: number;
}

export interface GoodDepositSpec {
  min_elev: number;
  count_num: number;
  count_den: number;
  /** Per-mineral ore-province noise frequency (each metal lights up its own ranges). */
  province_scale?: number;
}

export interface GoodSpec {
  id: string;
  name: string;
  icon: string;
  color: string;
  enabled: boolean;
  domain: GoodDomain;
  distribution: GoodDistribution;
  rarity: number;
  desire: number;
  network_luxury: boolean;
  builtin: boolean;
  deposit?: GoodDepositSpec | null;
  scoring?: GoodEnvelope | null;
  /** Need category — alternatives within a category substitute for each other. */
  category: string;
  /** Needs ladder tier: 0 basic, 1 comfort, 2 luxury. */
  need_tier: number;
  /** World-standard value per unit in grain-equivalent (wheat = 1). */
  base_value: number;
  /** Freight weight/volume multiplier (1 = silk-light; 3-4 = bulky staple). */
  bulk?: number;
  /** Extra freight per travel-day from spoilage (0 = durable). */
  perishable?: number;
  /** Recipe inputs for a Manufactured good (empty/absent = raw/extracted). */
  inputs?: RecipeInput[];
  /** Output-rate factor for manufacture (∝ population × this). */
  labor?: number;
  /** Demand cadence in days — how often a person consumes a unit (food ~7,
   *  comfort ~45, durables/luxuries ~180). Long = weak local pull → wholesale. */
  consumption_interval?: number;
}

export type PaintValue =
  | { type: "terrain"; value: number }
  | { type: "elevation"; value: number }
  | { type: "shelf"; value: number }
  | { type: "volcanic"; value: number };

export type ActiveTool = "pan" | "select" | "paint" | "elevation" | "shelf" | "volcano";
export type ActiveLayer =
  | "land" | "elevation" | "climate" | "temperature" | "precipitation"
  | "soil" | "fertility" | "plates"
  | "biomes" | "fisheries" | "terrain" | "shelf" | "ridges" | "wind" | "currents"
  | "habitability" | "salinity" | "shark" | "shipworm" | "storm" | "reef" | "disease";

export interface VectorSample {
  x: number;
  y: number;
  vx: number;
  vy: number;
  type?: number; // 0=none, 1=warm, 2=cold (for currents)
}

export interface Streamline {
  points: [number, number][];
  ctype: number; // 0=neutral (equatorial/counter-current/gyre), 1=warm, 2=cold/ACC
}

export interface TradeRoute {
  points: [number, number][];
  kind: number; // 0=overland caravan, 1=maritime, 2=river
  minor: boolean; // lesser town's single connector road (drawn thinner)
}

export interface FisheryBank {
  x: number;
  y: number;
  radius: number;
  score: number;
}

export interface RiverParams {
  density: number;      // 0-1.5: sparse trunk rivers ↔ very many tributaries
  width: number;        // 0.2-2: width multiplier
  lakeFillDepth: number;// 0.0005-0.05 normalized: min depression depth for a lake
  lakeMaxFraction: number; // 0.000002-0.05: max lake size as fraction of grid (low = tiny lakes)
}

export type WorkflowStep = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11;

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
}
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
}
/** Full live per-settlement detail (sentiment + market + history). */
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
  /** Estate descriptors (kind 1 farm/2 mine/3 plantation/4 fishery/5 vineyard). */
  estate_kind?: number;
  estate_owner?: string;
  estate_good?: string;
  /** Buildings erected here: [name, one-line effect]. */
  structures?: [string, string][];
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
  war_with?: string;                 // polis at war with ("" = peace)
  coin_name?: string;
  coin_trust?: number;
  coin_value?: number;
  transit?: TransitRow[];            // carrying trade through this city's merchants
  stolen_good?: string;              // espionage: good whose technique this estate stole
  stolen_from?: string;              // city it was stolen from ("" = none)
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
}
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
  name: string;
  kind: number;       // 1 farm/2 mine/3 plantation/4 fishery/5 vineyard/6 manufactory
  good: string;
  output: number;
  owner: string;
  owner_is_guild: boolean;
  owner_is_civic?: boolean; // city-financed (locally owned)
  tier: number;       // upgrade tier 1..5
}
/** One city in the live richest-cities ranking. */
export interface CityRank {
  id: number;
  name: string;
  population: number;
  wealth: number;
  trade: number;
  pct_world: number;
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
  monopolies: [string, number][]; // good name + share 0..1
  rivals: string[];
  defunct: boolean;
  color?: string;                // stable distinct colour (hex) for this house
  seat?: [number, number];       // home-seat position (world cell coords)
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
  expense_total: number;
  net: number;
  wealth_graph: number[];
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
}
export interface WorldGoodPrice {
  good: number;
  name: string;
  world_price: number;
  producers: number;
  top_hub: string;
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
}

/** One bank's balance sheet + reach. */
export interface BankBrief {
  name: string;
  seat: string;
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
  branches: string[];
  events: string[];
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
  cause: string;
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
}

export interface LakeData {
  cells: [number, number][];
  elevation: number;
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
  site?: string;    // "coast" | "river" | "hills" | "plain"
  dead?: boolean;   // population collapsed → drawn as a black cross, not a dot
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
