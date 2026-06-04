import { invoke } from "@tauri-apps/api/core";
import type { WorldMeta, TileResponse, CellInfo, PaintValue, VectorSample, SharkZone, GoodRegion, TradeMatrix, PoliticalCenter, GoodSpec } from "../types";

export async function newWorld(name: string, gridWidth: number, gridHeight: number): Promise<WorldMeta> {
  return invoke("new_world", { name, gridWidth, gridHeight });
}

export async function getWorldMeta(): Promise<WorldMeta | null> {
  return invoke("get_world_meta");
}

/** Persist the latitude framing (equator position + expansion). The next run of
 *  any simulation phase generates against these latitudes. */
export async function setLatitudeConfig(
  equatorOffset: number,
  latScale: number,
): Promise<WorldMeta> {
  return invoke("set_latitude_config", { equatorOffset, latScale });
}

export async function getTiles(
  tiles: [number, number][],
  layers: string[],
  lod: number = 0,
): Promise<TileResponse[]> {
  return invoke("get_tiles", { tiles, layers, lod });
}

export async function getTileRange(
  txMin: number, txMax: number,
  tyMin: number, tyMax: number,
  layers: string[],
  lod: number = 0,
): Promise<TileResponse[]> {
  return invoke("get_tile_range", { txMin, txMax, tyMin, tyMax, layers, lod });
}

export async function paintStroke(
  cells: [number, number][],
  value: PaintValue,
): Promise<[number, number][]> {
  return invoke("paint_stroke", { cells, value });
}

export async function getCellInfo(wx: number, wy: number): Promise<CellInfo> {
  return invoke("get_cell_info", { wx, wy });
}

export async function undoAction(): Promise<[number, number][] | null> {
  return invoke("undo");
}

export async function redoAction(): Promise<[number, number][] | null> {
  return invoke("redo");
}

export async function loadImageTemplate(path: string): Promise<[number, number][]> {
  return invoke("load_image_template", { path });
}

// --- Simulation commands ---

export async function simGeneratePlates(seed: number, plateCount: number): Promise<[number, number][]> {
  return invoke("sim_generate_plates", { seed, plateCount });
}

export async function simInvertTerrain(): Promise<[number, number][]> {
  return invoke("sim_invert_terrain");
}

export async function simGenerateTerrain(seed: number): Promise<[number, number][]> {
  return invoke("sim_generate_terrain", { seed });
}

export async function simOceanAtmosphere(): Promise<[number, number][]> {
  return invoke("sim_ocean_atmosphere");
}

export async function simClassifyClimate(): Promise<[number, number][]> {
  return invoke("sim_classify_climate");
}

export async function simRiversHydrology(
  riverDensity: number,
  riverWidth: number,
  lakeFillDepth: number,
  lakeMaxFraction: number,
): Promise<import("../types").SimRiversResult> {
  return invoke("sim_rivers_hydrology", { riverDensity, riverWidth, lakeFillDepth, lakeMaxFraction });
}

export async function simSoilFertility(riversJson: string): Promise<[number, number][]> {
  return invoke("sim_soil_fertility", { riversJson });
}

export async function simGenerateShelves(
  seed: number, shelfWidth: number, noiseAmount: number,
  depthProfile: number, dropoffWidth: number
): Promise<[number, number][]> {
  return invoke("sim_generate_shelves", { seed, shelfWidth, noiseAmount, depthProfile, dropoffWidth });
}

export async function simScaleElevation(
  scale: number, lockPeaksAbove: number
): Promise<[number, number][]> {
  return invoke("sim_scale_elevation", { scale, lockPeaksAbove });
}

export async function simGenerateSettlements(
  seed: number, riversJson: string
): Promise<import("../types").SimSettlementsResult> {
  return invoke("sim_generate_settlements", { seed, riversJson });
}

export async function simBiological(
  seed: number, riversJson: string, gemDeposits: number, climateStrictness: number,
): Promise<[number, number][]> {
  return invoke("sim_biological", { seed, riversJson, gemDeposits, climateStrictness });
}

export async function simGenerateTerrainFromTemplate(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_from_template", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

export async function simRunAll(seed: number, plateCount: number): Promise<import("../types").SimRunAllResult> {
  return invoke("sim_run_all", { seed, plateCount });
}

export async function simRunAllFromTerrain(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<import("../types").SimRunAllResult> {
  return invoke("sim_run_all_from_terrain", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

// --- Overlay / query commands ---

export interface OverlayVectors {
  wind: VectorSample[];
  currents: VectorSample[];
  current_step: number;
}

export async function getOverlayVectors(): Promise<OverlayVectors> {
  return invoke("get_overlay_vectors");
}

export interface Streamline {
  points: [number, number][];
  ctype: number; // 0=neutral, 1=warm, 2=cold
}

export async function getCurrentStreamlines(): Promise<Streamline[]> {
  return invoke("get_current_streamlines");
}

export interface TradeRoute {
  points: [number, number][];
  kind: number; // 0 = overland caravan, 1 = maritime, 2 = river
  minor: boolean; // lesser town's single connector road (drawn thinner)
}

/** Compute trade routes between the current settlements (pass the store list).
 *  Rivers feed inland routes; `reach`/`maxCrossing` cap open-water crossings
 *  (reach: 0 = global, 1 = coastal+short, 2 = continental only). `desertRoutes`
 *  is the Silk-Road mode: caravans prefer overland steppe corridors & deserts
 *  when seas are dangerous. */
export async function computeTradeRoutes(
  settlements: { x: number; y: number; score: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
): Promise<TradeRoute[]> {
  return invoke("compute_trade_routes", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
  });
}

export interface FisheryBank {
  x: number;
  y: number;
  radius: number;
  score: number;
}

/** Compute circular "grand bank" fishing-ground zones from the fishery field. */
export async function computeFisheryBanks(): Promise<FisheryBank[]> {
  return invoke("compute_fishery_banks");
}

/** Cluster the highest-risk shark-infested water into danger zones. */
export async function computeSharkZones(): Promise<SharkZone[]> {
  return invoke("compute_shark_zones");
}

/** Cluster the highest-risk shipworm (Teredo) hull-hazard water into zones. */
export async function computeShipwormZones(): Promise<SharkZone[]> {
  return invoke("compute_shipworm_zones");
}

/** Cluster the highest-risk storm/cyclone water into danger zones. `month <= 0`
 *  is the combined annual extent; 1..months applies the seasonal phase. */
export async function computeStormZones(month: number, months: number): Promise<SharkZone[]> {
  return invoke("compute_storm_zones", { month, months });
}

/** Cluster the highest-risk reef/shoal wreck water into danger zones. */
export async function computeReefZones(): Promise<SharkZone[]> {
  return invoke("compute_reef_zones");
}

export interface OverlaysResult {
  vectors: OverlayVectors;
  streamlines: Streamline[];
  fishery_banks: FisheryBank[];
  shark_zones: SharkZone[];
  shipworm_zones: SharkZone[];
  reef_zones: SharkZone[];
  good_regions: GoodRegion[];
}

/** One-shot fetch of the *static* map overlays (wind/current vectors +
 *  streamlines, fishery banks, shark/shipworm/reef zones, trade-good regions) in
 *  a single IPC round-trip backed by one shared tile-cache read. Storm zones are
 *  fetched separately (`computeStormZones`) because they depend on the month. */
export async function computeOverlays(): Promise<OverlaysResult> {
  return invoke("compute_overlays");
}

// ── Trade-good library (editable specs; per-world + global) ──
/** The shipped 30-good defaults (for "reset to default"). */
export async function defaultGoods(): Promise<GoodSpec[]> {
  return invoke("default_goods");
}
/** The current world's active good specs (per-world snapshot or defaults). */
export async function getGoodsSpec(): Promise<GoodSpec[]> {
  return invoke("get_goods_spec");
}
/** Snapshot a good-spec list into the current world (used before generation). */
export async function setGoodsSpec(specs: GoodSpec[]): Promise<void> {
  return invoke("set_goods_spec", { specs });
}
/** The global good library (editing template for new worlds). */
export async function getGoodsLibrary(): Promise<GoodSpec[]> {
  return invoke("get_goods_library");
}
/** Persist the global good library. */
export async function saveGoodsLibrary(specs: GoodSpec[]): Promise<void> {
  return invoke("save_goods_library", { specs });
}

/** Cluster every trade-good belt into labelled regions. */
export async function computeGoodRegions(): Promise<GoodRegion[]> {
  return invoke("compute_good_regions");
}

/** Build the region↔region trade matrix (routed + bundled flows). */
export async function computeTradeMatrix(
  settlements: { x: number; y: number; score: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
  luxuryBias: number,
): Promise<TradeMatrix> {
  return invoke("compute_trade_matrix", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    luxuryBias,
  });
}

/** Re-rank settlements by trade power and return political influence centers. */
export async function computePolitical(
  settlements: { x: number; y: number; score: number; population: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
): Promise<PoliticalCenter[]> {
  return invoke("compute_political", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
  });
}

export interface ElevationBand {
  label: string;
  count: number;
  percentage: number;
}

export async function getElevationDistribution(): Promise<ElevationBand[]> {
  return invoke("get_elevation_distribution");
}

// --- File operations ---

export async function saveWorldAs(path: string): Promise<void> {
  return invoke("save_world_as", { path });
}

export async function openWorld(path: string): Promise<WorldMeta> {
  return invoke("open_world", { path });
}

export async function exportHeightmap(path: string): Promise<void> {
  return invoke("export_heightmap", { path });
}

export async function exportLayers(
  dir: string, baseName: string, layers: string[]
): Promise<string[]> {
  return invoke("export_layers", { dir, baseName, layers });
}
