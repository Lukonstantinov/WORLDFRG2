export interface WorldMeta {
  name: string;
  grid_width: number;
  grid_height: number;
  tile_size: number;
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
  goods: { name: string; amount: number }[];
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
  | "habitability" | "salinity" | "shark" | "shipworm";

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

export type WorkflowStep = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;

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
  radius: number;     // influence radius in world cells
  population: number;
  monopolies: string[];
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
  points: [number, number][]; // [from, to] world coords of a routed coarse edge
  volume: number;
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
