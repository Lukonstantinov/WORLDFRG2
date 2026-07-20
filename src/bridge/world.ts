// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import { invoke } from "@tauri-apps/api/core";
import type { CampaignInfo, CellInfo, EconomySnapshot, LakeData, LakeNode, OpenWorldResult, PaintValue, RidgeLine, RiverData, RiverNode, Settlement, SimRiversResult, SimRunAllResult, SimSettlementsResult, TileResponse, Toponym, WorldMeta } from "@types";
import type { ElevationBand, OverlayVectors, OverlaysState, Streamline } from "./types";

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
  latRatio: number,
): Promise<WorldMeta> {
  return invoke("set_latitude_config", { equatorOffset, latScale, latRatio });
}

/** How many cultures the world starts with (0 = auto by land area). */
export async function setCultureCount(count: number): Promise<void> {
  return invoke("set_culture_count", { count });
}

export async function getCultureCount(): Promise<number> {
  return invoke("get_culture_count");
}

export async function getTiles(
  tiles: [number, number][],
  layers: string[],
  lod: number = 0,
): Promise<TileResponse[]> {
  return invoke("get_tiles", { tiles, layers, lod });
}

/** Raw-bytes tile fetch (no base64/JSON overhead). See TileManager's
 *  parsePackedTiles for the record layout. */
export async function getTilesPacked(
  tiles: [number, number][],
  layers: string[],
  lod: number = 0,
): Promise<ArrayBuffer> {
  return invoke("get_tiles_packed", { tiles, layers, lod });
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
): Promise<import("@types").SimRiversResult> {
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
  seed: number, riversJson: string, realism?: number, maxSettlements?: number
): Promise<import("@types").SimSettlementsResult> {
  return invoke("sim_generate_settlements", { seed, riversJson, realism, maxSettlements });
}

export async function simBiological(
  seed: number, riversJson: string, gemDeposits: number, climateStrictness: number,
): Promise<[number, number][]> {
  return invoke("sim_biological", { seed, riversJson, gemDeposits, climateStrictness });
}

/** One-click refresh of hydrology → biology on an existing world (rivers/lakes +
 *  oxbows + salt + delta abundance + goods) without re-rolling terrain or moving
 *  settlements. Returns the fresh rivers & lakes for the overlays. */
export async function simRefreshHydrologyBiology(
  seed: number, riverDensity: number, riverWidth: number,
  lakeFillDepth: number, lakeMaxFraction: number,
  gemDeposits: number, climateStrictness: number,
): Promise<import("@types").SimRiversResult> {
  return invoke("sim_refresh_hydrology_biology", {
    seed, riverDensity, riverWidth, lakeFillDepth, lakeMaxFraction, gemDeposits, climateStrictness,
  });
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

/** Alternative elevation model: plate-free, world-size-aware ridged cordillera
 *  (mountain count scales with the map) + erosion. Keeps the existing landmass. */
export async function simGenerateTerrainRidged(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_ridged", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

export async function simRunAll(seed: number, plateCount: number): Promise<import("@types").SimRunAllResult> {
  return invoke("sim_run_all", { seed, plateCount });
}

/** Generate mountain ridges from hand-drawn ridge lines. Each line carries its
 *  polyline spine, footprint width, peak height and ruggedness; the backend
 *  widens them into eroded ranges, blended onto the existing elevation (land only). */
export async function simGenerateRidges(
  lines: import("@types").RidgeLine[],
  seed: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_ridges", { linesJson: JSON.stringify(lines), seed });
}

// ── #26 · Geographic toponyms (gated, editable) ──
/** Generate culture-styled names for rivers/mountains/lakes/regions (gated on the
 *  Settlements + Rivers steps; errors otherwise). Returns & persists the list. */
export async function simGenerateToponyms(
  rivers: { points: [number, number][] }[],
  lakes: { cells: [number, number][] }[],
): Promise<import("@types").Toponym[]> {
  return invoke("sim_generate_toponyms", {
    riversJson: JSON.stringify(rivers),
    lakesJson: JSON.stringify(lakes),
  });
}

/** Persist a user-edited toponym list (renames). */
export async function saveToponyms(toponyms: import("@types").Toponym[]): Promise<void> {
  return invoke("save_toponyms", { toponymsJson: JSON.stringify(toponyms) });
}

/** Load the persisted toponym list (empty until generated). */
export async function getToponyms(): Promise<import("@types").Toponym[]> {
  return invoke("get_toponyms");
}

export async function simRunAllFromTerrain(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<import("@types").SimRunAllResult> {
  return invoke("sim_run_all_from_terrain", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

/** Render a downscaled RGBA raster of a world rectangle for one layer — the
 *  terrain backdrop behind the Hydrology river snip. Returns base64 RGBA + dims. */
export async function renderWorldCrop(
  x0: number, y0: number, x1: number, y1: number, layer: string, maxDim: number,
): Promise<{ data: string; w: number; h: number }> {
  return invoke("render_world_crop", { x0, y0, x1, y1, layer, maxDim });
}

/** Hydrology dashboard: build the river-system tree (trunks + nested tributaries)
 *  with per-river stats, elevation profile, cities-on-river and Earth counterpart.
 *  Pass the world's rivers + settlements from the store. */
export async function getRiverSystems(
  rivers: import("@types").RiverData[],
  settlements: { x: number; y: number; name?: string; size?: string }[],
): Promise<import("@types").RiverNode[]> {
  return invoke("get_river_systems", {
    riversJson: JSON.stringify(rivers),
    settlementsJson: JSON.stringify(settlements),
  });
}

/** Classified lakes with their limnological + ecological profiles (Lakes tab). */
export async function getLakeSystems(
  lakes: import("@types").LakeData[],
  rivers: import("@types").RiverData[],
): Promise<import("@types").LakeNode[]> {
  return invoke("get_lake_systems", {
    lakesJson: JSON.stringify(lakes),
    riversJson: JSON.stringify(rivers),
  });
}

export async function getOverlayVectors(): Promise<OverlayVectors> {
  return invoke("get_overlay_vectors");
}

export async function getCurrentStreamlines(): Promise<Streamline[]> {
  return invoke("get_current_streamlines");
}

/** Read the persisted economy snapshot (empty if not yet generated). */
export async function getEconomy(): Promise<EconomySnapshot> {
  return invoke("get_economy");
}

/** Persist the current overlay state (settlements/rivers/lakes) into the DB so a
 *  saved world re-opens with its trade & settlement layers intact. The economy
 *  snapshot is already persisted by computeEconomy. */
export async function persistOverlays(
  settlements: import("@types").Settlement[],
  rivers: import("@types").RiverData[],
  lakes: import("@types").LakeData[],
): Promise<void> {
  return invoke("persist_overlays", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    lakesJson: JSON.stringify(lakes),
  });
}

/** Read all persisted overlay state when re-opening a world. */
export async function getOverlays(): Promise<OverlaysState> {
  return invoke("get_overlays");
}

/** Export the economy snapshot to a file (.json = raw snapshot, else CSV). */
export async function exportTradeData(path: string): Promise<void> {
  return invoke("export_trade_data", { path });
}

export async function getElevationDistribution(): Promise<ElevationBand[]> {
  return invoke("get_elevation_distribution");
}

// --- File operations ---

export async function saveWorldAs(path: string): Promise<void> {
  return invoke("save_world_as", { path });
}

export async function openWorld(path: string): Promise<OpenWorldResult> {
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

// ── World/campaign split ──

/** Freeze the world's geography; campaign steps unlock. */
export async function finalizeWorld(): Promise<void> {
  return invoke("finalize_world");
}

export async function unfreezeWorld(): Promise<void> {
  return invoke("unfreeze_world");
}

export async function newCampaign(name: string): Promise<void> {
  return invoke("new_campaign", { name });
}

export async function saveCampaignAs(path: string): Promise<void> {
  return invoke("save_campaign_as", { path });
}

export async function openCampaign(path: string): Promise<CampaignInfo> {
  return invoke("open_campaign", { path });
}

/** Persist wizard progress ("world" = steps 1-6, "campaign" = 7-10). */
export async function setProgress(scope: "world" | "campaign", progressJson: string): Promise<void> {
  return invoke("set_progress", { scope, progressJson });
}

/** Save the appearance palette override (sparse JSON) with the world file. */
export async function setAppearance(appearanceJson: string): Promise<void> {
  return invoke("set_appearance", { appearanceJson });
}

/** Read the world's saved appearance palette override (null if never set). */
export async function getAppearance(): Promise<string | null> {
  return invoke("get_appearance");
}

/** Copy chosen layer groups from another .worldforge file into the current
 *  world (grid sizes must match). Returns the modified tile coords. */
export async function importWorldLayers(path: string, groups: string[]): Promise<[number, number][]> {
  return invoke("import_world_layers", { path, groups });
}
