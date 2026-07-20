// Split from the former monolithic src/types.ts. Mirrors Rust serde structs.

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

export type PaintValue =
  | { type: "terrain"; value: number }
  | { type: "elevation"; value: number }
  | { type: "shelf"; value: number }
  | { type: "volcanic"; value: number };

export type ActiveTool = "pan" | "select" | "paint" | "elevation" | "shelf" | "volcano" | "ridge";

/** A hand-drawn mountain-ridge line: a polyline spine (world cells) whose stroke
 *  width encodes the range's footprint width, opacity encodes peak height, and a
 *  character parameter controls ruggedness. `erase` (Shift-draw) flattens instead. */
export interface RidgeLine {
  points: [number, number][];
  width: number;
  height: number;
  character: number;
  erase: boolean;
  noise: number;
}
export type ActiveLayer =
  | "land" | "elevation" | "climate" | "temperature" | "precipitation"
  | "soil" | "fertility" | "plates"
  | "biomes" | "fisheries" | "terrain" | "shelf" | "ridges" | "wind" | "windspeed" | "currents"
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

export type WorkflowStep = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12;
