// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import { invoke } from "@tauri-apps/api/core";
import type { CampaignInfo, CellInfo, EconomySnapshot, LakeData, LakeNode, OpenWorldResult, PaintValue, RidgeLine, RiverData, RiverNode, Settlement, SimRiversResult, SimRunAllResult, SimSettlementsResult, TileResponse, Toponym, WorldMeta, RenderPalettes } from "@types";
import type { ElevationBand, OverlayVectors, OverlaysState, Streamline } from "./types";

export async function newWorld(name: string, gridWidth: number, gridHeight: number): Promise<WorldMeta> {
  return invoke("new_world", { name, gridWidth, gridHeight });
}

export async function getWorldMeta(): Promise<WorldMeta | null> {
  return invoke("get_world_meta");
}

/** Persist the latitude framing (equator position + expansion) and axial tilt. The
 *  next run of any simulation phase generates against these latitudes; the seasonal
 *  temperature model reads the obliquity. `obliquity` defaults to Earth's 23.44°. */
export async function setLatitudeConfig(
  equatorOffset: number,
  latScale: number,
  latRatio: number,
  obliquity = 23.44,
): Promise<WorldMeta> {
  return invoke("set_latitude_config", { equatorOffset, latScale, latRatio, obliquity });
}

/** Planetary state driving the emergent climate (energy budget + circulation). */
export interface PlanetConfig {
  /** Rotation rate (× Earth). Sign is direction: negative = retrograde (mirrors
   *  the Coriolis-deflection direction in winds/currents; belt LATITUDE only
   *  depends on magnitude, so it is unaffected by the sign). */
  rotationRate: number;
  /** Stellar irradiance (× Earth solar constant). Global-mean temperature. */
  solarLum: number;
  /** Greenhouse factor (× Earth). Global warming + equator-pole gradient. */
  greenhouse: number;
  /** Orbital eccentricity (0 = circular). Hemispheric season asymmetry. */
  eccentricity: number;
  /** Global aridity multiplier (1.0 = Earth/no-op). >1 = drier, <1 = wetter. */
  dryness: number;
}

/** Rust serde emits snake_case; map it to our camelCase shape. */
interface PlanetConfigRaw {
  rotation_rate: number;
  solar_lum: number;
  greenhouse: number;
  eccentricity: number;
  dryness: number;
}
const fromRawPlanet = (r: PlanetConfigRaw): PlanetConfig => ({
  rotationRate: r.rotation_rate,
  solarLum: r.solar_lum,
  greenhouse: r.greenhouse,
  eccentricity: r.eccentricity,
  dryness: r.dryness,
});

/** Read the world's planetary state (all default to Earth). */
export async function getPlanetConfig(): Promise<PlanetConfig> {
  return fromRawPlanet(await invoke<PlanetConfigRaw>("get_planet_config"));
}

/** Persist the planetary state. The next run of Ocean & Atmosphere → Climate
 *  generates against it (energy-balance temperature + rotation-driven belts). */
export async function setPlanetConfig(cfg: PlanetConfig): Promise<PlanetConfig> {
  return fromRawPlanet(await invoke<PlanetConfigRaw>("set_planet_config", {
    rotationRate: cfg.rotationRate,
    solarLum: cfg.solarLum,
    greenhouse: cfg.greenhouse,
    eccentricity: cfg.eccentricity,
    dryness: cfg.dryness,
  }));
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

/** The plate inspector's click-to-flip: override one plate's oceanic/
 *  continental assignment and re-rasterize landmass from it (same plate
 *  geometry, no re-partition). Uses the world's own recorded generation seed
 *  server-side — never pass one from the UI's Seed field, which may since
 *  have been re-rolled without regenerating. */
export async function simSetPlateOceanic(plateId: number, isOceanic: boolean): Promise<[number, number][]> {
  return invoke("sim_set_plate_oceanic", { plateId, isOceanic });
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

/** Phase 6b: classify ecological biomes (Köppen + climate/soil/relief + rivers
 *  & lakes → the `biome` tile column). Purely descriptive — no later phase
 *  scores off it, so re-running never moves a city or a trade belt. */
export async function simClassifyBiomes(
  riversJson: string, lakesJson: string,
): Promise<[number, number][]> {
  return invoke("sim_classify_biomes", { riversJson, lakesJson });
}

/** Per-biome land-cell counts for the Biomes legend (read-only). */
export async function getBiomeStats(): Promise<import("@types").BiomeStat[]> {
  return invoke("get_biome_stats");
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

/** Elevation model: a CORDILLERA — long continuous chains traced along the
 *  continental margin, with a continental divide, asymmetric flanks (steep
 *  seaward scarp, broad inland piedmont) and parallel sub-ranges. Keeps the
 *  existing landmass. */
export async function simGenerateTerrainCordillera(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_cordillera", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

/** Elevation model: parallel fault blocks — a tilted, asymmetric horst-and-graben
 *  rift system. Strike follows the world's own divergent-boundary trend where
 *  plate data exists, a seeded regional strike otherwise. */
export async function simGenerateTerrainRift(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_rift", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

/** Elevation model: the shape model, then glacial modification — U-valley
 *  broadening, cirque hollows, over-deepened troughs that breach the coast (real
 *  fjords, carved rather than notched). May turn a little land into sea near the
 *  coast. */
export async function simGenerateTerrainGlaciated(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_glaciated", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

/** Elevation model: quantised levels with sharp escarpment rims + outlying
 *  buttes. */
export async function simGenerateTerrainPlateau(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_plateau", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

/** Elevation model: shield cones on volcanic cells, summit calderas on the
 *  densest clusters, hotspot trails from isolated seeds. */
export async function simGenerateTerrainVolcanic(
  seed: number,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<[number, number][]> {
  return invoke("sim_generate_terrain_volcanic", {
    seed, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
}

/** The planetary settings a preview runs against. Passed explicitly (not read
 *  from the world) so the UI can preview values mid-drag, before they commit. */
export interface PreviewSettings {
  obliquity: number;
  rotationRate: number;
  solarLum: number;
  greenhouse: number;
  eccentricity: number;
  dryness: number;
  equatorOffset: number;
  latScale: number;
  latRatio: number;
}

/** TIER 1 — the 1-D zonal profile (EBM temperature curve + seasonal envelope +
 *  circulation belts). Microseconds, so it is safe to call on every drag. */
export async function previewZonalProfile(
  s: PreviewSettings,
): Promise<import("@types").ZonalProfile> {
  return invoke("preview_zonal_profile", { ...s });
}

/** TIER 2 — the real Ocean & Atmosphere → Köppen chain on a downsampled copy of
 *  this world's landmass. A few hundred ms; put it behind a button. */
export async function previewCoarseClimate(
  s: PreviewSettings,
): Promise<import("@types").CoarsePreview> {
  return invoke("preview_coarse_climate", { ...s });
}

export async function simRunAll(
  seed: number,
  plateCount: number,
  elevMode: string,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<import("@types").SimRunAllResult> {
  return invoke("sim_run_all", {
    seed, plateCount, elevMode,
    mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
  });
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

// ── Landmass step lasso area tools (ITCZ_AND_LAND_TOOLS_PLAN.md Commit 1) ──
// Each takes the lasso polygon (world-cell coords, may straddle the
// antimeridian — the backend unwraps it) and returns the modified tile coords.

export async function landOpSmoothRoughen(
  lasso: import("@types").LassoPolygon,
  amount: number,
  seed: number,
): Promise<[number, number][]> {
  return invoke("land_op_smooth_roughen", { lassoJson: JSON.stringify(lasso), amount, seed });
}

export async function landOpFjords(
  lasso: import("@types").LassoPolygon,
  count: number,
  lengthKm: number,
  width: number,
  seed: number,
): Promise<[number, number][]> {
  return invoke("land_op_fjords", { lassoJson: JSON.stringify(lasso), count, lengthKm, width, seed });
}

export type IslandKind = "arc" | "scatter" | "single";

export async function landOpIslands(
  lasso: import("@types").LassoPolygon,
  count: number,
  kind: IslandKind,
  size: number,
  seed: number,
): Promise<[number, number][]> {
  return invoke("land_op_islands", { lassoJson: JSON.stringify(lasso), count, kind, size, seed });
}

export async function landOpFill(
  lasso: import("@types").LassoPolygon,
  land: boolean,
): Promise<[number, number][]> {
  return invoke("land_op_fill", { lassoJson: JSON.stringify(lasso), land });
}

/** A read-only downsampled thumbnail of the current world (land/sea + elevation),
 *  used by the landmass variant compare — never read through the tile/LOD cache. */
export async function renderWorldThumbnail(maxPx: number): Promise<import("@types").WorldThumbnail> {
  return invoke("render_world_thumbnail", { maxPx });
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
  elevMode: string,
  mountainDensity: number,
  mountainHeight: number,
  mountainSpread: number,
  noiseRoughness: number,
): Promise<import("@types").SimRunAllResult> {
  return invoke("sim_run_all_from_terrain", {
    seed, elevMode, mountainDensity, mountainHeight, mountainSpread, noiseRoughness,
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

export async function getPlateMotion(): Promise<import("@types").PlateMotionArrow[]> {
  return invoke("get_plate_motion");
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

/** Partition all land into provinces (watershed / cost-flood). Runs AFTER the
 *  settlement step (settlements seed it). Persists the province list and returns
 *  it plus a downsampled per-cell id raster for the map overlay. */
export async function simGenerateProvinces(
  settlements: import("@types").Settlement[],
  rivers: import("@types").RiverData[],
  granularity?: number,
): Promise<import("@types").SimProvincesResult> {
  return invoke("sim_generate_provinces", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    granularity,
  });
}

/** Post-generation cleanup: fold every province smaller than the threshold into the
 *  neighbour it shares the most border with (never an island). Returns the full
 *  updated layer to reload in place. `minCells` overrides the adaptive default. */
export async function simMergeSmallProvinces(
  minCells?: number,
  selected?: number[],
): Promise<import("@types").SimProvincesResult> {
  return invoke("sim_merge_small_provinces", { minCells, selected });
}

/** Post-generation cleanup: split every NON-POLAR province larger than the threshold
 *  into compact sub-provinces (arctic/antarctic left untouched). The cut is an ORGANIC
 *  cost-flood over the crest/river feature fields — pass `rivers` so it can follow the
 *  channels. `selected` (province ids) limits the split to just those; omit for all.
 *  Returns the full updated layer to reload in place. */
export async function simSplitLargeProvinces(
  maxCells?: number,
  rivers?: import("@types").RiverData[],
  selected?: number[],
): Promise<import("@types").SimProvincesResult> {
  return invoke("sim_split_large_provinces", {
    maxCells,
    riversJson: rivers ? JSON.stringify(rivers) : undefined,
    selected,
  });
}

/** Read back the stored province list (reopening a world / panel refresh). */
export async function getProvinces(): Promise<import("@types").Province[]> {
  return invoke("get_provinces");
}

/** Read back the full province layer (list + downsampled raster) on world open. */
export async function getProvinceLayer(): Promise<import("@types").SimProvincesResult> {
  return invoke("get_province_layer");
}

/** Live per-province campaign state (read-only join: baseline rural + live urban). */
export async function campaignProvinceState(): Promise<import("@types").ProvinceLive[]> {
  return invoke("campaign_province_state");
}

/** Full detail of one province (live settlements + all buildings on it) for the
 *  province subwindow. Returns null when no campaign / province layer exists. */
export async function campaignProvinceDetail(id: number): Promise<import("@types").ProvinceDetail | null> {
  return invoke("campaign_province_detail", { id });
}

/** A cropped elevation/land/biome sample grid over one province's bounding box —
 *  the survey plate's real "relief" base layer (§2.3). `maxDim` bounds the longer
 *  side of the returned grid (~130 matches the plate's own fidelity target).
 *  Returns null when no province layer exists. */
export async function getProvinceTerrainCrop(
  provinceId: number,
  maxDim = 130,
): Promise<import("@types").ProvinceTerrainCrop | null> {
  return invoke("get_province_terrain_crop", { provinceId, maxDim });
}

/** The renderer's own colour tables (elevation · bathymetry · temperature ·
 *  precipitation ramps, plus the Köppen/biome/soil class colours). The legend reads
 *  these instead of keeping its own copy — see CLAUDE.md §8.18. */
export async function getRenderPalettes(): Promise<RenderPalettes> {
  return await invoke("get_render_palettes");
}
