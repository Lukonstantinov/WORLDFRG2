import { create } from "zustand";
import { setProgress } from "../bridge/tauri";
import type { MerchantRoute } from "../types";

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

interface UIStore {
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
  /** Goods-browser panel open (toolbar button → browse all goods by origin). */
  showGoodsBrowser: boolean;
  /** Merchant-houses panel open. */
  showHouses: boolean;
  showCityRanking: boolean;
  /** Goods & Chains review window open (always shown before goods generation). */
  chainReviewOpen: boolean;
  /** Action run when the user confirms "Generate" in the chain-review window
   *  (set by StepBiological); null when the window is opened just to inspect. */
  chainReviewConfirm: (() => void) | null;

  setTool: (tool: ActiveTool) => void;
  setLayer: (layer: ActiveLayer) => void;
  setBrushRadius: (r: number) => void;
  setElevationValue: (v: number) => void;
  setStatus: (text: string) => void;
  setInspectedCell: (cell: { wx: number; wy: number } | null) => void;
  setSelectedHub: (id: number | null) => void;
  setSelectedMerchantRoute: (r: MerchantRoute | null) => void;
  setSelectedChain: (id: number | null) => void;
  setWorkflowStep: (step: WorkflowStep) => void;
  markStepCompleted: (step: number) => void;
  setStepsCompleted: (steps: number[]) => void;
  setSimRunning: (running: boolean) => void;
  resetWorkflow: () => void;
  setOverlayVisible: (type: string, visible: boolean) => void;
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
  setShowGoodsBrowser: (v: boolean) => void;
  setShowHouses: (v: boolean) => void;
  setShowCityRanking: (v: boolean) => void;
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
    houseControl: false, merchantRoutes: false,
    hubNames: false, settlementNames: false, tradeRegions: false,
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
  selectedChain: null,
  reachGood: null,
  selectedGood: null,
  openRoads: [],
  selectedExport: null,
  hubDisplay: { size: 1, intensity: 1 },
  settlementRealism: 0.55,
  showGoodsBrowser: false,
  showHouses: false,
  showCityRanking: false,
  chainReviewOpen: false,
  chainReviewConfirm: null,

  setTool: (tool) => set({ activeTool: tool }),
  setLayer: (layer) => set({ activeLayer: layer }),
  setBrushRadius: (r) => set({ brushRadius: r }),
  setElevationValue: (v) => set({ elevationValue: v }),
  setStatus: (text) => set({ statusText: text }),
  setInspectedCell: (cell) => set({ inspectedCell: cell }),
  setSelectedHub: (id) => set({ selectedHub: id, selectedChain: null, selectedExport: null }),
  setSelectedMerchantRoute: (r) => set({ selectedMerchantRoute: r }),
  setSelectedChain: (id) => set({ selectedChain: id }),
  setSimRunning: (running) => set({ simRunning: running }),
  setLayerOpacity: (opacity) => set({ layerOpacity: opacity }),

  setOverlayVisible: (type, visible) =>
    set((state) => ({
      overlayVisibility: { ...state.overlayVisibility, [type]: visible },
    })),

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
  setShowGoodsBrowser: (v) => set({ showGoodsBrowser: v }),
  setShowHouses: (v) => set({ showHouses: v }),
  setShowCityRanking: (v) => set({ showCityRanking: v }),
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
