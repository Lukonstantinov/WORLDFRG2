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

export type WorkflowStep = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

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
export interface EconHub {
  id: number;
  x: number;
  y: number;
  name: string;
  power: number;
  stars: number;
  wealth: number;
  population: number;
  produces: EconHubGood[];
  receives: EconReceive[];
}
export interface EconChainStop {
  hub: number;
  price: number;
}
export interface EconChain {
  id: number;
  good: number;
  good_name: string;
  stops: EconChainStop[];
  points: [number, number][];
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
export interface EconomySnapshot {
  hubs: EconHub[];
  chains: EconChain[];
  chokepoints: EconChokepoint[];
  regions: EconRegion[];
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
  size: "capital" | "city" | "town" | "village";
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
