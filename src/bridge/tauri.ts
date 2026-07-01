import { invoke } from "@tauri-apps/api/core";
import type { WorldMeta, OpenWorldResult, CampaignInfo, TileResponse, CellInfo, PaintValue, VectorSample, SharkZone, GoodRegion, CultureRegion, TradeMatrix, TradeTrunk, PoliticalCenter, GoodSpec, EconomySnapshot } from "../types";

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
  seed: number, riversJson: string, realism?: number
): Promise<import("../types").SimSettlementsResult> {
  return invoke("sim_generate_settlements", { seed, riversJson, realism });
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

export async function simRunAll(seed: number, plateCount: number): Promise<import("../types").SimRunAllResult> {
  return invoke("sim_run_all", { seed, plateCount });
}

// ── #26 · Geographic toponyms (gated, editable) ──
/** Generate culture-styled names for rivers/mountains/lakes/regions (gated on the
 *  Settlements + Rivers steps; errors otherwise). Returns & persists the list. */
export async function simGenerateToponyms(
  rivers: { points: [number, number][] }[],
  lakes: { cells: [number, number][] }[],
): Promise<import("../types").Toponym[]> {
  return invoke("sim_generate_toponyms", {
    riversJson: JSON.stringify(rivers),
    lakesJson: JSON.stringify(lakes),
  });
}
/** Persist a user-edited toponym list (renames). */
export async function saveToponyms(toponyms: import("../types").Toponym[]): Promise<void> {
  return invoke("save_toponyms", { toponymsJson: JSON.stringify(toponyms) });
}
/** Load the persisted toponym list (empty until generated). */
export async function getToponyms(): Promise<import("../types").Toponym[]> {
  return invoke("get_toponyms");
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
  piracy: number,
  season: number,
  months: number,
): Promise<TradeRoute[]> {
  return invoke("compute_trade_routes", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    piracy,
    season,
    months,
  });
}

/** Travel-time / itinerary between two world cells over the shared coarse cost
 *  grid (#23). `reach`: 2 = continental (no open-sea crossings), else sea allowed. */
export async function computeItinerary(
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
  rivers: { points: [number, number][] }[],
  reach: number,
  desertRoutes: boolean,
): Promise<import("../types").Itinerary> {
  return invoke("compute_itinerary", {
    fromX, fromY, toX, toY,
    riversJson: JSON.stringify(rivers),
    reach,
    desertRoutes,
  });
}

/** DLC 3.5 · the live campaign's dynamic trade-flow trunks (last year's actual
 *  shipped volume, routed over the cost grid + bundled; width ∝ volume). */
export async function campaignGetTradeFlow(
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
): Promise<TradeTrunk[]> {
  return invoke("campaign_get_trade_flow", {
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
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

/** Cluster the land under a monsoon-type climate into wet-season flood regions
 *  (Natural Disasters overlay, alongside the hurricane/storm zones). */
export async function computeMonsoonZones(): Promise<SharkZone[]> {
  return invoke("compute_monsoon_zones");
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
/** Live suitability heatmap for a good spec (Goods Editor preview). */
export async function previewGoodScore(spec: GoodSpec): Promise<{ width: number; height: number; data: number[]; land: number[] }> {
  return invoke("preview_good_score", { spec });
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

/** The organic culture territories of the active world (Peoples overlay). */
export async function computeCultureRegions(): Promise<CultureRegion[]> {
  return invoke("compute_culture_regions");
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
  piracy: number,
): Promise<TradeMatrix> {
  return invoke("compute_trade_matrix", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    luxuryBias,
    piracy,
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
  piracy: number,
): Promise<PoliticalCenter[]> {
  return invoke("compute_political", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    piracy,
  });
}

/** Build + persist the economy snapshot (hubs, quality-graded production,
 *  cost-aware flows with per-hop prices, wealth, chokepoints). */
export async function computeEconomy(
  settlements: { x: number; y: number; score: number; population: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
  luxuryBias: number,
  piracy: number,
  season: number,
  months: number,
): Promise<EconomySnapshot> {
  return invoke("compute_economy", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    luxuryBias,
    piracy,
    season,
    months,
  });
}

/** Read the persisted economy snapshot (empty if not yet generated). */
export async function getEconomy(): Promise<EconomySnapshot> {
  return invoke("get_economy");
}

/** Persist the current overlay state (settlements/rivers/lakes) into the DB so a
 *  saved world re-opens with its trade & settlement layers intact. The economy
 *  snapshot is already persisted by computeEconomy. */
export async function persistOverlays(
  settlements: import("../types").Settlement[],
  rivers: import("../types").RiverData[],
  lakes: import("../types").LakeData[],
): Promise<void> {
  return invoke("persist_overlays", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    lakesJson: JSON.stringify(lakes),
  });
}

export interface OverlaysState {
  settlements: import("../types").Settlement[];
  rivers: import("../types").RiverData[];
  lakes: import("../types").LakeData[];
  economy: EconomySnapshot;
}

/** Read all persisted overlay state when re-opening a world. */
export async function getOverlays(): Promise<OverlaysState> {
  return invoke("get_overlays");
}

/** Trade-development feedback: grow each settlement by its hub's trade wealth
 *  (one-way, bounded). Returns the updated settlement list. */
export async function computeSettlementDevelopment(
  settlements: import("../types").Settlement[],
): Promise<import("../types").Settlement[]> {
  return invoke("compute_settlement_development", {
    settlementsJson: JSON.stringify(settlements),
  });
}

/** Export the economy snapshot to a file (.json = raw snapshot, else CSV). */
export async function exportTradeData(path: string): Promise<void> {
  return invoke("export_trade_data", { path });
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

// ── DLC 1 "Living Trade" tick simulation ──
import type { CampaignSnapshot, JournalEntry, WorldEconomy, HubDetail, ColonyDetail, ColonySummary, ColonyGateStatus, HouseBrief, HouseHistory, HouseLedger, CampaignDiagnostics, MerchantRoute, FuturesLane, WarehouseInfo, CityRank, SpecCenter, PolisBrief, TradeFlows, CurrencyBrief, CoinUseCity, BankBrief, CrashRecord, CitySchematic, WarsPayload, GoodMarketRow, PopBrief, EpidemicBrief, GuildBrief, FigureBrief, LandmarkBrief } from "../types";

/** Seed a fresh living-trade sim from the static economy snapshot (step 10). */
export async function campaignStartSim(seed: number): Promise<CampaignSnapshot> {
  return invoke("campaign_start_sim", { seed });
}

/** Advance the sim by N days. The backend keeps the sim resident in memory and
 *  autosaves on a year/wall-clock cadence — call `campaignPersist` to force a flush. */
export async function campaignAdvance(ticks: number): Promise<CampaignSnapshot> {
  return invoke("campaign_advance", { ticks });
}

/** Force-flush the resident sim to disk (call on pause / before close). */
export async function campaignPersist(): Promise<void> {
  return invoke("campaign_persist");
}

/** Current sim snapshot (inactive when no sim has been started). */
export async function campaignGetState(): Promise<CampaignSnapshot> {
  return invoke("campaign_get_state");
}

/** Journal rows, filtered by hub and/or good (-1 = any). */
export async function campaignGetJournal(hub: number, good: number): Promise<JournalEntry[]> {
  return invoke("campaign_get_journal", { hub, good });
}

/** Full live detail for one settlement (sentiment + market + history), or null
 *  when no campaign sim is running / the hub isn't in it. */
export async function campaignGetHub(id: number): Promise<HubDetail | null> {
  return invoke("campaign_get_hub", { id });
}

/** Colony detail (Supply subtab) — null for non-colony hubs. */
export async function campaignGetColony(id: number): Promise<ColonyDetail | null> {
  return invoke("campaign_get_colony", { id });
}

/** Empire-wide colony roster (settlement colonies + house outposts). */
export async function campaignGetColonies(): Promise<ColonySummary[]> {
  return invoke("campaign_get_colonies");
}

/** Read-only colony-founding gate status (the "why none yet?" diagnostics). */
export async function campaignColonyGates(): Promise<ColonyGateStatus | null> {
  return invoke("campaign_colony_gates");
}

/** World-economy panel data (per-good world prices + price-index series). */
export async function campaignGetWorldEconomy(): Promise<WorldEconomy> {
  return invoke("campaign_get_world_economy");
}

/** #30 · live per-city cost-of-living basket index from the running campaign. */
export async function campaignCityPriceIndex(): Promise<import("../types").CityPriceIndex[]> {
  return invoke("campaign_city_price_index");
}

/** All merchant families (active first, richest first). */
export async function campaignGetHouses(): Promise<HouseBrief[]> {
  return invoke("campaign_get_houses");
}

/** #29 · wealth-inequality (Gini) + social-mobility snapshot from the live sim. */
export async function campaignGetInequality(): Promise<import("../types").InequalitySnapshot> {
  return invoke("campaign_get_inequality");
}

/** DLC 4 · the derived typed Pops of one hub (Nations & POPs foundation). */
export async function campaignGetPops(hub: number): Promise<PopBrief[]> {
  return invoke("campaign_get_pops", { hub });
}

/** DLC 3 · the cached yearly speculation read (per-polis bubble risk + why). */
export async function campaignGetSpeculation(): Promise<SpecCenter[]> {
  return invoke("campaign_get_speculation");
}

/** DLC 3 · the poleis as actors (treasury / tariff / mint / council). */
export async function campaignGetPoleis(): Promise<PolisBrief[]> {
  return invoke("campaign_get_poleis");
}

/** Realized trade flows at a settlement (per-good volumes + history, routes,
 *  top partner cities) for the Trade ▸ Flows subtab. Null if no campaign. */
export async function campaignTradeFlows(id: number): Promise<TradeFlows | null> {
  return invoke("campaign_trade_flows", { id });
}

/** DLC 3.5 · the world's coinage ranked by reserve strength (trust × throughput). */
export async function campaignGetCurrencies(): Promise<CurrencyBrief[]> {
  return invoke("campaign_get_currencies");
}

/** Per-city coin usage: which coin each settlement settles its trade in + volume —
 *  for the coin-usage map overlay and the per-coin donut/bar breakdown. */
export async function campaignCoinUsage(): Promise<CoinUseCity[]> {
  return invoke("campaign_coin_usage");
}

/** DLC 3.5 · all chartered banks with their balance sheets. */
export async function campaignGetBanks(): Promise<BankBrief[]> {
  return invoke("campaign_get_banks");
}

/** DLC 3.5 · the log of regional financial crashes (newest first). */
export async function campaignGetCrashes(): Promise<CrashRecord[]> {
  return invoke("campaign_get_crashes");
}

/** DLC 3.5 · active economic wars + the concluded-war log. */
export async function campaignGetWars(): Promise<WarsPayload> {
  return invoke("campaign_get_wars");
}

/** Phase 6 · plagues & epidemics grouped into outbreaks (active-first, deadliest). */
export async function campaignGetEpidemics(): Promise<EpidemicBrief[]> {
  return invoke("campaign_get_epidemics");
}

/** Phase 6 · craft guilds (good, quality, output, strength, guildhall). */
export async function campaignGetGuilds(): Promise<GuildBrief[]> {
  return invoke("campaign_get_guilds");
}

/** Phase 6 · notable figures (Great Lives roster). */
export async function campaignGetFigures(): Promise<FigureBrief[]> {
  return invoke("campaign_get_figures");
}

/** Phase 6 · landmarks & sacred sites (wonders, holy cities, fairs, guildhalls). */
export async function campaignGetLandmarks(): Promise<LandmarkBrief[]> {
  return invoke("campaign_get_landmarks");
}

/** DLC 4 · every good's quality rating + produced/traded totals (Goods window). */
export async function campaignGetGoods(): Promise<GoodMarketRow[]> {
  return invoke("campaign_get_goods");
}

/** DLC 3.5 · per-city schematics (buildings / estates / banks / coin). */
export async function campaignGetSchematics(): Promise<CitySchematic[]> {
  return invoke("campaign_get_schematics");
}

/** A house/guild's yearly T-account ledger (Accountant view). */
export async function campaignHouseLedger(house: number): Promise<HouseLedger | null> {
  return invoke("campaign_house_ledger", { house });
}

/** Active merchant routes (per family/guild, aggregated) for the map layer. */
export async function campaignMerchantRoutes(): Promise<MerchantRoute[]> {
  return invoke("campaign_merchant_routes");
}

/** Active futures contracts as directional supply lanes for the Futures map layer. */
export async function campaignFuturesLanes(): Promise<FuturesLane[]> {
  return invoke("campaign_futures_lanes");
}

/** All house/guild warehouses (largest stock first) for the Warehouses panel. */
export async function campaignWarehouses(): Promise<WarehouseInfo[]> {
  return invoke("campaign_warehouses");
}

/** Live richest-cities ranking with each city's share of world trade. */
export async function campaignCityRanking(): Promise<CityRank[]> {
  return invoke("campaign_city_ranking");
}

/** One house's full chronicle (timeline) by name. */
export async function campaignGetHouseHistory(name: string): Promise<HouseHistory | null> {
  return invoke("campaign_get_house_history", { name });
}

/** "Is trade actually moving?" snapshot — null when no sim is running. */
export async function campaignDiagnostics(): Promise<CampaignDiagnostics | null> {
  return invoke("campaign_diagnostics");
}
