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
  /** Axial tilt (obliquity, degrees) driving seasonality. 23.44 = Earth-like;
   *  higher = more extreme seasons; 0 = no seasons. */
  obliquity: number;
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
  /** Display name for the cell's biome (classified column, else Köppen-derived). */
  biome: string;
  /** Raw biome code (mirrors sim::biome); 0 = unclassified / sea. */
  biome_code: number;
  /** Coarse biome family ("Wetland & riparian", "Boreal", …); "" when unclassified. */
  biome_group: string;
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

export type ActiveTool = "pan" | "select" | "paint" | "elevation" | "shelf" | "volcano" | "ridge" | "lasso";

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
/** A freehand-drawn selection polygon for the Landmass step's area tools
 *  (`ITCZ_AND_LAND_TOOLS_PLAN.md` Commit 1) — world-cell coordinates, may
 *  straddle the antimeridian (the backend's `Lasso::new` unwraps it). */
export type LassoPolygon = [number, number][];

export type ActiveLayer =
  | "land" | "elevation" | "climate" | "temperature" | "precipitation"
  | "sst" | "snow"
  | "soil" | "fertility" | "plates"
  | "biomes" | "fisheries" | "terrain" | "natural" | "shelf" | "ridges" | "wind" | "windspeed" | "currents"
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

export type WorkflowStep = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12;

/** One biome's share of the world — mirrors the Rust `BiomeStat`. Feeds the
 *  Biomes legend in the workflow panel. */
export interface BiomeStat {
  code: number;
  name: string;
  group: string;
  cells: number;
}

/** One latitude's row in the zonal settings preview. Mirrors Rust `ZonalSample`. */
export interface ZonalSample {
  lat: number;
  temp: number;
  tempEarth: number;
  summerMaritime: number;
  winterMaritime: number;
  summerContinental: number;
  winterContinental: number;
  /** Indicative Köppen main class 'A'..'E' — for colouring the strip only. */
  zone: string;
}

/** The 1-D zonal preview: per-latitude traces + belt geometry + guardrails.
 *  Mirrors Rust `ZonalProfile`. */
export interface ZonalProfile {
  samples: ZonalSample[];
  hadleyEdge: number;
  polarFront: number;
  cells: number;
  retrograde: boolean;
  globalMean: number;
  globalMeanEarth: number;
  /** Lowest |latitude| with permanent ice; 90 = none. */
  iceLine: number;
  snowballRisk: boolean;
  beltsCollapsed: boolean;
  visibleTop: number;
  visibleBottom: number;
}

/** The coarse climate preview: a Köppen thumbnail + the class mix it implies.
 *  Mirrors Rust `CoarsePreview`. */
export interface CoarsePreview {
  width: number;
  height: number;
  /** Base64-encoded RGBA pixels, width × height × 4. */
  rgba: string;
  tropicalPct: number;
  aridPct: number;
  temperatePct: number;
  continentalPct: number;
  polarPct: number;
  landCells: number;
  meanTemp: number;
  meanPrecip: number;
  /** The rough major ocean-current streamlines this preview implies, in
   *  thumbnail pixel coords — drawn over the Köppen thumbnail. `ctype`:
   *  0 = neutral, 1 = warm, 2 = cold. Same shape as `Streamline`. */
  streamlines: Streamline[];
}

/** A read-only downsampled land/sea + elevation thumbnail of the current world,
 *  used by the landmass variant compare. Mirrors Rust `WorldThumbnail`. */
export interface WorldThumbnail {
  width: number;
  height: number;
  /** Base64-encoded RGBA pixels, width × height × 4. */
  rgba: string;
}

/** RENDER PALETTES — served by `get_render_palettes` straight out of the Rust
 *  renderer (`palette_commands.rs`), so the legend reads the SAME tables that paint
 *  the pixels rather than keeping a copy that can drift. See CLAUDE.md §8.18. */
export interface RampStop {
  /** Position in the ramp's own units: metres · °C · normalised depth · or the
   *  band's upper bound in mm for the classed precipitation scale. */
  at: number;
  color: string;
}
export interface ClassColor {
  code: number;
  color: string;
}
/** GOODS_LOCALITIES_PLAN.md D10 · one stop on the ONE absolute belt-quality scale
 *  every good's quality layer shades on. The colour is the good's own; this carries
 *  only how strongly it is drawn at a given belt value — so a thin wine fringe and a
 *  thin wheat fringe read alike, and no good is ever renormalised against its own
 *  best. Served from `palette_commands.rs`, never copied here (§8.18). */
export interface QualityStop {
  /** The belt value itself, 0..1 — absolute, never per-good normalised. */
  at: number;
  alpha: number;
  /** How far the good's own hue is mixed in (0 = the pale ground tint, 1 = full). */
  mix: number;
}
/** An alternate reading of the same 0..1 belt-quality scale: a heat ramp (dark
 *  blue → red) that ignores the good's own hue — added because the hue-mix scale
 *  above is subtle for a muted or dark good. Same value, different paint. */
export interface GradeStop {
  at: number;
  /** The `deposits::grade_label` word this breakpoint carries — coarse / ordinary /
   *  good / fine / exquisite, the same vocabulary ore workings use. */
  label: string;
  color: string;
}
export interface StylePalette {
  key: string;
  label: string;
  land: RampStop[];
  sea: RampStop[];
  classed: boolean;
}

export interface RenderPalettes {
  elevation: RampStop[];
  bathymetry: RampStop[];
  temperature: RampStop[];
  precipitation: RampStop[];
  koppen: ClassColor[];
  biome: ClassColor[];
  soil: ClassColor[];
  elev_max_m: number;
  good_quality: QualityStop[];
  good_quality_pale: string;
  good_quality_heatmap: RampStop[];
  good_quality_grades: GradeStop[];
  elevation_styles: StylePalette[];
}
