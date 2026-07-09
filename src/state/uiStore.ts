import { create } from "zustand";
import { setProgress } from "../bridge/tauri";
import type { MerchantRoute, FuturesLane } from "../types";

/** Persist step completion: steps 1-6 travel with the world file, 7-10 with
 *  the campaign. Fire-and-forget — a failed write only loses the checkmarks. */
function persistProgress(stepCompleted: Record<number, boolean>) {
  const world: Record<number, boolean> = {};
  const campaign: Record<number, boolean> = {};
  for (const [k, v] of Object.entries(stepCompleted)) {
    if (!v) continue;
    const n = Number(k);
    (n <= 6 ? world : campaign)[n] = true;
  }
  setProgress("world", JSON.stringify(world)).catch(() => {});
  setProgress("campaign", JSON.stringify(campaign)).catch(() => {});
}
import type { ActiveTool, ActiveLayer, WorkflowStep } from "../types";
import { GOOD_DEFS, goodOverlayKey } from "../goods";

type LandmassSource = "none" | "plates" | "template" | "painted";

export interface TerrainParams {
  density: number;
  height: number;
  spread: number;
  roughness: number;
  seed: number | null; // null = use world seed
}

export interface RiverParamsState {
  density: number;
  width: number;
  lakeFillDepth: number;
  lakeMaxFraction: number;
}

export interface BioParamsState {
  gemDeposits: number;   // number of highland gemstone deposits
  tradeReach: number;    // 0 = global, 1 = coastal+short, 2 = continental only
  maxCrossing: number;   // max open-water crossing as fraction of map width
  desertRoutes: boolean; // Silk-Road mode: prefer overland steppe/desert caravans when seas are dangerous
  calendarMonths: number; // length of the seasonal calendar ("moons"), default 12
  stormMonth: number;    // storm overlay viewing month: 0 = combined, 1..months
  economicRegions: number;   // number of economic regions / hub granularity (2..40)
  luxuryBias: number;        // 0 = subsistence (staples), 0.5 = neutral, 1 = mercantile (luxuries)
  climateStrictness: number; // 0 = diffuse belts, 0.5 = neutral, 1 = tight climate-locked belts
  piracyLevel: number;       // 0 = safe seas, 1 = pirate-infested (raises maritime route cost)
  tradeSeason: number;       // 0 = all-year routes; 1..calendarMonths applies seasonal closures
}

/** The two subproducts, presented as one app. "forge" = world generation
 *  (paint + sim phases 1–10, geography editable); "chronicle" = the living
 *  campaign played on a finalized world (read-only map, economy simulated live).
 *  Chronicle is only reachable once the world is finalized (frozen). */
export type AppMode = "forge" | "chronicle";

interface UIStore {
  /** Which subproduct is on screen (Forge vs Chronicle). */
  appMode: AppMode;
  activeTool: ActiveTool;
  activeLayer: ActiveLayer;
  brushRadius: number;
  elevationValue: number;
  statusText: string;
  inspectedCell: { wx: number; wy: number } | null;
  workflowStep: WorkflowStep;
  stepCompleted: Record<number, boolean>;
  simRunning: boolean;
  overlayVisibility: Record<string, boolean>;
  layerOpacity: number;
  /** When true, the world is stretched to fill the canvas (no letterbox bars,
   *  but distorts aspect). When false, fit proportionally (undistorted). */
  stretchToFit: boolean;
  landmassSource: LandmassSource;
  terrainParams: TerrainParams;
  riverParams: RiverParamsState;
  bioParams: BioParamsState;
  showTradeMatrix: boolean;
  /** Economy hub inspector (Phase 3): selected hub id, or null. */
  selectedHub: number | null;
  /** Merchant layer: a clicked active route whose round-trip details are shown. */
  selectedMerchantRoute: MerchantRoute | null;
  /** Futures layer: a clicked contract lane whose detail is shown. */
  selectedFuturesLane: FuturesLane | null;
  /** Futures contracts list panel open. */
  showFutures: boolean;
  /** Warehouses infographic panel open. */
  showWarehouses: boolean;
  /** Futures focus filter: when set, lanes matching (city at either end, and/or
   *  holder, and/or good) stay bold on the map and all others fade — used by the
   *  list panel's filter and by city/warehouse selection. */
  futuresFocus: { city?: string; holder?: string; good?: string } | null;
  /** Trade ▸ Flows highlight: glowing arrows from a settlement to its partners on
   *  the map (set by the Flows subtab; [] clears it). dir 0 = inbound, 1 = outbound. */
  flowHighlight: { ax: number; ay: number; bx: number; by: number; dir: number; w: number }[];
  /** Highlighted supply-chain id (Phase 3): traced on the map, or null. */
  selectedChain: number | null;
  /** Per-good reach view: highlight which hubs a chosen good reaches, or null. */
  reachGood: string | null;
  /** Good-flow panel: the good whose routes/price-graph is open, or null. */
  selectedGood: string | null;
  /** Good-flow panel: chain ids opened/highlighted independently on the map. */
  openRoads: number[];
  /** Hub window: an export good whose destination flows are highlighted, or null. */
  selectedExport: string | null;
  /** Adjustable trade-hub marker display (size multiplier + highlight intensity). */
  hubDisplay: { size: number; intensity: number };
  /** Settlement density / realism (0..1): low = sparse & strict, high = dense. */
  settlementRealism: number;
  /** Hard cap on total settlements (20..1000); 0 = auto (realism-driven). */
  settlementCap: number;
  /** Goods-browser panel open (toolbar button → browse all goods by origin). */
  showGoodsBrowser: boolean;
  /** Id of the good whose seeding/climate detail panel is open (null = closed). */
  goodDetailId: string | null;
  /** Merchant-houses panel open. */
  showHouses: boolean;
  showCityRanking: boolean;
  /** v2.0 · the unified Money & Finance panel (mints · banks · bubbles · shocks · schematics). */
  showMoneyFinance: boolean;
  /** #23 · Itinerary / travel-time panel open. */
  showItinerary: boolean;
  /** Atlas 2.0 · the World Atlas (world graphs / city census / timeline) panel. */
  showAtlas: boolean;
  /** #1/#23 · the Peoples (cultures) panel + the culture currently isolated on the map. */
  showPeoples: boolean;
  selectedCulture: string | null;
  /** Route-bound migration overlay mode: ribbon (width∝volume) · dots · focus (inbound
   *  flows of the selected city only). */
  migrationMode: "ribbon" | "dots" | "focus";
  /** Colony/satellite ↔ metropolis link to shine on the map (a=metro, b=colony). */
  colonyHighlight: { ax: number; ay: number; bx: number; by: number } | null;
  /** Plague spread REPLAY: show outbreak `id`'s spread up to step (0=origin). */
  plagueReplay: { id: number; step: number } | null;
  /** Batch 1 · Trade Heat filtered to ONE good (by id/name); null = all goods. */
  heatGood: string | null;
  /** Batch 1 · era scrubber: when set, the map's markers + heat show this past
   *  year instead of the live world (set from the Atlas year slider). */
  eraFrame: import("../types").EraFrame | null;
  /** #30/#29 · Economy Dashboard (price index / inequality) panel open. */
  showEconomyDashboard: boolean;
  /** 🌊 Hydrology dashboard (river systems) panel open. */
  showHydrology: boolean;
  /** #35/#36/#37 · Goods Codex (provenance / history / scarcity) panel open. */
  showGoodsCodex: boolean;
  /** Itinerary routed polyline (world cells) to draw on the map, or null. */
  travelRoute: [number, number][] | null;
  /** 🌊 Hydrology: river indices (into the rivers array) of the selected system's
   *  subtree to glow on the map (others dim), or null. */
  riverHighlight: number[] | null;
  /** 🌊 Hydrology: per-river-index glow colour (branch / order scheme). Missing
   *  entries fall back to the default cyan glow. */
  riverHighlightColors: Record<number, string> | null;
  /** 🌊 Hydrology: index of the lake selected in the Lakes tab to glow on the map
   *  (others dim), or null. */
  lakeHighlight: number | null;
  /** Goods Codex: the good whose provenance/history/scarcity is shown, or null. */
  codexGood: string | null;
  /** Colonial Office — empire-wide colony/outpost roster + founding-gate diagnostics. */
  showColonial: boolean;
  /** Dedicated Bank panel (balance-sheet charts / loans & deals / schematic / info). */
  showBank: boolean;
  /** Index of the bank to focus when the Bank panel opens (null = first/list). */
  selectedBankIdx: number | null;
  /** Bank icons on the map (find banks at a glance). */
  showBankIcons: boolean;
  /** The filterable World News feed (global campaign chronicle). */
  showNews: boolean;
  /** Phase 6 · Plagues & Epidemics panel open. */
  showPlagues: boolean;
  /** Phase 6 · Guilds & Crafts panel open. */
  showGuilds: boolean;
  /** Phase 6 · Notable Figures panel open. */
  showFigures: boolean;
  /** Phase 6 · Landmarks & Sacred Sites panel open. */
  showLandmarks: boolean;
  /** Phase 7 · Dynasties & Alliances panel open. */
  showDynasties: boolean;
  /** DLC 4 · the floating Goods (quality & trade) window. */
  showGoodsWindow: boolean;
  /** Chrome visibility — lets the user hide the left workflow panel and the right
   *  toolbar to get a clean map with just the floating window bar. */
  showWorkflow: boolean;
  showToolbar: boolean;
  /** Coin-usage map overlay: the mint hub id of the coin to highlight on the map
   *  (cities that settle in it), or null for off. Set from the Currencies panel. */
  coinOverlayHub: number | null;
  /** Goods & Chains review window open (always shown before goods generation). */
  chainReviewOpen: boolean;
  /** Action run when the user confirms "Generate" in the chain-review window
   *  (set by StepBiological); null when the window is opened just to inspect. */
  chainReviewConfirm: (() => void) | null;

  setAppMode: (mode: AppMode) => void;
  setTool: (tool: ActiveTool) => void;
  setLayer: (layer: ActiveLayer) => void;
  setBrushRadius: (r: number) => void;
  setElevationValue: (v: number) => void;
  setStatus: (text: string) => void;
  setInspectedCell: (cell: { wx: number; wy: number } | null) => void;
  setSelectedHub: (id: number | null) => void;
  setSelectedMerchantRoute: (r: MerchantRoute | null) => void;
  setSelectedFuturesLane: (r: FuturesLane | null) => void;
  setFlowHighlight: (segs: { ax: number; ay: number; bx: number; by: number; dir: number; w: number }[]) => void;
  setShowFutures: (open: boolean) => void;
  setShowWarehouses: (open: boolean) => void;
  setFuturesFocus: (f: { city?: string; holder?: string; good?: string } | null) => void;
  setSelectedChain: (id: number | null) => void;
  setWorkflowStep: (step: WorkflowStep) => void;
  markStepCompleted: (step: number) => void;
  setStepsCompleted: (steps: number[]) => void;
  setSimRunning: (running: boolean) => void;
  resetWorkflow: () => void;
  setOverlayVisible: (type: string, visible: boolean) => void;
  setOverlaysVisible: (types: string[], visible: boolean) => void;
  toggleOverlay: (type: string) => void;
  setLayerOpacity: (opacity: number) => void;
  setLandmassSource: (source: LandmassSource) => void;
  setStretchToFit: (v: boolean) => void;
  setTerrainParams: (p: Partial<TerrainParams>) => void;
  setRiverParams: (p: Partial<RiverParamsState>) => void;
  setBioParams: (p: Partial<BioParamsState>) => void;
  setShowTradeMatrix: (v: boolean) => void;
  setReachGood: (g: string | null) => void;
  setSelectedGood: (g: string | null) => void;
  toggleOpenRoad: (id: number) => void;
  clearOpenRoads: () => void;
  setSelectedExport: (g: string | null) => void;
  setHubDisplay: (p: Partial<{ size: number; intensity: number }>) => void;
  setSettlementRealism: (v: number) => void;
  setSettlementCap: (v: number) => void;
  setShowGoodsBrowser: (v: boolean) => void;
  setGoodDetail: (id: string | null) => void;
  setShowHouses: (v: boolean) => void;
  setShowCityRanking: (v: boolean) => void;
  setShowItinerary: (v: boolean) => void;
  setShowAtlas: (v: boolean) => void;
  setShowPeoples: (v: boolean) => void;
  setSelectedCulture: (c: string | null) => void;
  setMigrationMode: (m: "ribbon" | "dots" | "focus") => void;
  setColonyHighlight: (l: { ax: number; ay: number; bx: number; by: number } | null) => void;
  setPlagueReplay: (r: { id: number; step: number } | null) => void;
  setHeatGood: (g: string | null) => void;
  setEraFrame: (f: import("../types").EraFrame | null) => void;
  setShowEconomyDashboard: (v: boolean) => void;
  setShowHydrology: (v: boolean) => void;
  setShowGoodsCodex: (v: boolean) => void;
  setTravelRoute: (pts: [number, number][] | null) => void;
  setRiverHighlight: (ids: number[] | null) => void;
  setRiverHighlightColors: (c: Record<number, string> | null) => void;
  setLakeHighlight: (idx: number | null) => void;
  setCodexGood: (g: string | null) => void;
  setShowMoneyFinance: (v: boolean) => void;
  setShowColonial: (v: boolean) => void;
  setShowBank: (v: boolean) => void;
  setSelectedBankIdx: (i: number | null) => void;
  setShowBankIcons: (v: boolean) => void;
  setShowNews: (v: boolean) => void;
  setShowPlagues: (v: boolean) => void;
  setShowGuilds: (v: boolean) => void;
  setShowFigures: (v: boolean) => void;
  setShowLandmarks: (v: boolean) => void;
  setShowDynasties: (v: boolean) => void;
  setShowGoodsWindow: (v: boolean) => void;
  setShowWorkflow: (v: boolean) => void;
  setShowToolbar: (v: boolean) => void;
  setCoinOverlayHub: (v: number | null) => void;
  openChainReview: (onConfirm?: () => void) => void;
  closeChainReview: () => void;
}

// Default layer/tool for each step
const STEP_DEFAULTS: Record<number, { layer: ActiveLayer; tool: ActiveTool }> = {
  1: { layer: "land", tool: "paint" },
  2: { layer: "elevation", tool: "elevation" },
  // Keep the elevation view entering Ocean & Atmosphere so the terrain doesn't
  // appear to "smooth out" (the flat-shaded land layer hid the relief). Currents
  // and wind draw as overlays on top; switch to temperature/precip manually.
  3: { layer: "elevation", tool: "select" },
  4: { layer: "climate", tool: "select" },
  5: { layer: "land", tool: "select" },
  6: { layer: "fertility", tool: "select" },
  7: { layer: "land", tool: "select" },
  8: { layer: "land", tool: "select" },
  9: { layer: "land", tool: "select" },
  10: { layer: "land", tool: "select" },
  11: { layer: "land", tool: "select" },
};

export const useUIStore = create<UIStore>((set) => ({
  appMode: "forge",
  activeTool: "pan",
  activeLayer: "land",
  brushRadius: 3,
  elevationValue: 0.5,
  statusText: "",
  inspectedCell: null,
  workflowStep: 1,
  stepCompleted: {},
  simRunning: false,
  overlayVisibility: {
    rivers: true, lakes: true, settlements: true,
    markers: false, wind: false, currents: false, latLines: false,
    tradeRoutes: false, fisheryBanks: false,
    sharkZones: false, shipwormZones: false, stormZones: false, monsoonZones: false, reefZones: false, tradeFlows: false,
    politicalInfluence: false, chokepoints: false, tradeCorridors: false,
    speculation: false,
    houseControl: false, merchantRoutes: false, futures: false, dynamicFlow: false, tradeHeat: false,
    tradeBasins: false, migrations: true,
    colonies: true,
    hubNames: false, settlementNames: false, tradeRegions: false, cultures: false,
    travelRoute: false, goodScarcity: false, toponyms: false,
    // Per-feature-type toponym label toggles (gated under the master `toponyms`).
    toponymsRiver: true, toponymsLake: true, toponymsMountain: true, toponymsRegion: true,
    plagueZones: false, guildCities: false, figureMarks: false, landmarks: false, dynastyLinks: false,
    ...Object.fromEntries(GOOD_DEFS.map((g) => [goodOverlayKey(g.name), false])),
  },
  layerOpacity: 1,
  stretchToFit: true,
  landmassSource: "none",
  terrainParams: { density: 0.5, height: 0.5, spread: 0.5, roughness: 0.4, seed: null },
  riverParams: { density: 0.5, width: 1.0, lakeFillDepth: 0.006, lakeMaxFraction: 0.0001 },
  bioParams: { gemDeposits: 6, tradeReach: 1, maxCrossing: 0.3, desertRoutes: false, calendarMonths: 12, stormMonth: 0, economicRegions: 14, luxuryBias: 0.5, climateStrictness: 0.5, piracyLevel: 0, tradeSeason: 0 },
  showTradeMatrix: false,
  selectedHub: null,
  selectedMerchantRoute: null,
  selectedFuturesLane: null,
  showFutures: false,
  showWarehouses: false,
  futuresFocus: null,
  flowHighlight: [],
  selectedChain: null,
  reachGood: null,
  selectedGood: null,
  openRoads: [],
  selectedExport: null,
  hubDisplay: { size: 1, intensity: 1 },
  settlementRealism: 0.55,
  settlementCap: 0,
  showGoodsBrowser: false,
  goodDetailId: null,
  showHouses: false,
  showCityRanking: false,
  showMoneyFinance: false,
  showItinerary: false,
  showAtlas: false,
  showPeoples: false,
  selectedCulture: null,
  migrationMode: "ribbon",
  colonyHighlight: null,
  plagueReplay: null,
  heatGood: null,
  eraFrame: null,
  showEconomyDashboard: false,
  showHydrology: false,
  showGoodsCodex: false,
  travelRoute: null,
  riverHighlight: null,
  riverHighlightColors: null,
  lakeHighlight: null,
  codexGood: null,
  showColonial: false,
  showBank: false,
  selectedBankIdx: null,
  showBankIcons: false,
  showNews: false,
  showPlagues: false,
  showGuilds: false,
  showFigures: false,
  showLandmarks: false,
  showDynasties: false,
  showGoodsWindow: false,
  showWorkflow: true,
  showToolbar: true,
  coinOverlayHub: null,
  chainReviewOpen: false,
  chainReviewConfirm: null,

  // Entering Chronicle forces a non-destructive tool + lands on the campaign step
  // (step 11): the map is a read-only stage there, so paint tools must not be
  // active. Leaving to Forge restores the pan tool (the user reselects a tool).
  setAppMode: (mode) =>
    set((state) =>
      mode === "chronicle"
        ? { appMode: mode, activeTool: "pan", workflowStep: 11 as WorkflowStep }
        // Step 11 (the campaign tick) is Chronicle-only and no longer in the Forge
        // wizard; clamp back to Economy (10) so a Forge step is always expanded.
        : { appMode: mode, workflowStep: (state.workflowStep === 11 ? 10 : state.workflowStep) as WorkflowStep }
    ),
  setTool: (tool) => set({ activeTool: tool }),
  setLayer: (layer) => set({ activeLayer: layer }),
  setBrushRadius: (r) => set({ brushRadius: r }),
  setElevationValue: (v) => set({ elevationValue: v }),
  setStatus: (text) => set({ statusText: text }),
  setInspectedCell: (cell) => set({ inspectedCell: cell }),
  setSelectedHub: (id) => set({ selectedHub: id, selectedChain: null, selectedExport: null }),
  setSelectedMerchantRoute: (r) => set({ selectedMerchantRoute: r }),
  setSelectedFuturesLane: (r) => set({ selectedFuturesLane: r }),
  setFlowHighlight: (segs) => set({ flowHighlight: segs }),
  setShowFutures: (open) => set({ showFutures: open }),
  setShowWarehouses: (open) => set({ showWarehouses: open }),
  setFuturesFocus: (f) => set({ futuresFocus: f }),
  setSelectedChain: (id) => set({ selectedChain: id }),
  setSimRunning: (running) => set({ simRunning: running }),
  setLayerOpacity: (opacity) => set({ layerOpacity: opacity }),

  setOverlayVisible: (type, visible) =>
    set((state) => ({
      overlayVisibility: { ...state.overlayVisibility, [type]: visible },
    })),

  // Bulk-set many overlay keys at once (e.g. a whole good category toggled from
  // its master checkbox).
  setOverlaysVisible: (types, visible) =>
    set((state) => {
      const next = { ...state.overlayVisibility };
      for (const t of types) next[t] = visible;
      return { overlayVisibility: next };
    }),

  toggleOverlay: (type) =>
    set((state) => ({
      overlayVisibility: {
        ...state.overlayVisibility,
        [type]: !state.overlayVisibility[type],
      },
    })),

  setLandmassSource: (source) => set({ landmassSource: source }),

  setStretchToFit: (v) => set({ stretchToFit: v }),

  setTerrainParams: (p) =>
    set((state) => ({ terrainParams: { ...state.terrainParams, ...p } })),

  setRiverParams: (p) =>
    set((state) => ({ riverParams: { ...state.riverParams, ...p } })),

  setBioParams: (p) =>
    set((state) => ({ bioParams: { ...state.bioParams, ...p } })),

  setShowTradeMatrix: (v) => set({ showTradeMatrix: v }),

  setReachGood: (g) => set({ reachGood: g }),
  setSelectedGood: (g) => set({ selectedGood: g, openRoads: [] }),
  toggleOpenRoad: (id) => set((state) => ({
    openRoads: state.openRoads.includes(id)
      ? state.openRoads.filter((r) => r !== id)
      : [...state.openRoads, id],
  })),
  clearOpenRoads: () => set({ openRoads: [] }),
  setSelectedExport: (g) => set({ selectedExport: g }),
  setHubDisplay: (p) => set((state) => ({ hubDisplay: { ...state.hubDisplay, ...p } })),
  setSettlementRealism: (v) => set({ settlementRealism: v }),
  setSettlementCap: (v) => set({ settlementCap: v }),
  setShowGoodsBrowser: (v) => set({ showGoodsBrowser: v }),
  setGoodDetail: (id) => set({ goodDetailId: id }),
  setShowHouses: (v) => set({ showHouses: v }),
  setShowCityRanking: (v) => set({ showCityRanking: v }),
  setShowMoneyFinance: (v) => set({ showMoneyFinance: v }),
  setShowItinerary: (v) => set({ showItinerary: v }),
  setShowAtlas: (v) => set({ showAtlas: v }),
  setShowPeoples: (v) => set({ showPeoples: v }),
  setSelectedCulture: (c) => set({ selectedCulture: c }),
  setMigrationMode: (m) => set({ migrationMode: m }),
  setColonyHighlight: (l) => set({ colonyHighlight: l }),
  setPlagueReplay: (r) => set({ plagueReplay: r }),
  setHeatGood: (g) => set({ heatGood: g }),
  setEraFrame: (f) => set({ eraFrame: f }),
  setShowEconomyDashboard: (v) => set({ showEconomyDashboard: v }),
  setShowHydrology: (v) => set({ showHydrology: v }),
  setShowGoodsCodex: (v) => set({ showGoodsCodex: v }),
  setTravelRoute: (pts) => set({ travelRoute: pts }),
  setRiverHighlight: (ids) => set({ riverHighlight: ids }),
  setRiverHighlightColors: (c) => set({ riverHighlightColors: c }),
  setLakeHighlight: (idx) => set({ lakeHighlight: idx }),
  setCodexGood: (g) => set({ codexGood: g }),
  setShowColonial: (v) => set({ showColonial: v }),
  setShowBank: (v) => set({ showBank: v }),
  setSelectedBankIdx: (i) => set({ selectedBankIdx: i }),
  setShowBankIcons: (v) => set({ showBankIcons: v }),
  setShowNews: (v) => set({ showNews: v }),
  setShowPlagues: (v) => set({ showPlagues: v }),
  setShowGuilds: (v) => set({ showGuilds: v }),
  setShowFigures: (v) => set({ showFigures: v }),
  setShowLandmarks: (v) => set({ showLandmarks: v }),
  setShowDynasties: (v) => set({ showDynasties: v }),
  setShowGoodsWindow: (v) => set({ showGoodsWindow: v }),
  setShowWorkflow: (v) => set({ showWorkflow: v }),
  setShowToolbar: (v) => set({ showToolbar: v }),
  setCoinOverlayHub: (v) => set({ coinOverlayHub: v }),
  openChainReview: (onConfirm) => set({ chainReviewOpen: true, chainReviewConfirm: onConfirm ?? null }),
  closeChainReview: () => set({ chainReviewOpen: false, chainReviewConfirm: null }),

  setWorkflowStep: (step) => {
    const defaults = STEP_DEFAULTS[step] || { layer: "land", tool: "pan" };
    set({
      workflowStep: step,
      activeLayer: defaults.layer,
      activeTool: defaults.tool,
    });
  },

  markStepCompleted: (step) =>
    set((state) => {
      const stepCompleted = { ...state.stepCompleted, [step]: true };
      persistProgress(stepCompleted);
      return { stepCompleted };
    }),

  /** Restore completion state (e.g. from a re-opened world/campaign) without
   *  writing it back to the DB. */
  setStepsCompleted: (steps) =>
    set(() => {
      const stepCompleted: Record<number, boolean> = {};
      for (const s of steps) stepCompleted[s] = true;
      return { stepCompleted };
    }),

  resetWorkflow: () =>
    set({
      workflowStep: 1,
      stepCompleted: {},
      activeLayer: "land",
      activeTool: "paint",
    }),
}));
