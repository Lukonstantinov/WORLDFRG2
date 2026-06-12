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
export type GoodDistribution = "global" | "local" | "deposits";

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
  bought?: number;
  sold?: number;
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
}

export interface HouseTimelineEvent {
  year: number;
  kind: string; // founded | succession | monopoly | control_gained | control_lost | branch | loss | dissolved
  text: string;
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
