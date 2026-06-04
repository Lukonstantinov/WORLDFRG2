import { create } from "zustand";
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

  setTool: (tool: ActiveTool) => void;
  setLayer: (layer: ActiveLayer) => void;
  setBrushRadius: (r: number) => void;
  setElevationValue: (v: number) => void;
  setStatus: (text: string) => void;
  setInspectedCell: (cell: { wx: number; wy: number } | null) => void;
  setWorkflowStep: (step: WorkflowStep) => void;
  markStepCompleted: (step: number) => void;
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
    sharkZones: false, shipwormZones: false, stormZones: false, reefZones: false, tradeFlows: false,
    politicalInfluence: false, chokepoints: false,
    ...Object.fromEntries(GOOD_DEFS.map((g) => [goodOverlayKey(g.name), false])),
  },
  layerOpacity: 1,
  stretchToFit: true,
  landmassSource: "none",
  terrainParams: { density: 0.5, height: 0.5, spread: 0.5, roughness: 0.4, seed: null },
  riverParams: { density: 0.5, width: 1.0, lakeFillDepth: 0.004, lakeMaxFraction: 0.0008 },
  bioParams: { gemDeposits: 6, tradeReach: 1, maxCrossing: 0.18, desertRoutes: false, calendarMonths: 12, stormMonth: 0, economicRegions: 14, luxuryBias: 0.5, climateStrictness: 0.5 },
  showTradeMatrix: false,

  setTool: (tool) => set({ activeTool: tool }),
  setLayer: (layer) => set({ activeLayer: layer }),
  setBrushRadius: (r) => set({ brushRadius: r }),
  setElevationValue: (v) => set({ elevationValue: v }),
  setStatus: (text) => set({ statusText: text }),
  setInspectedCell: (cell) => set({ inspectedCell: cell }),
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

  setWorkflowStep: (step) => {
    const defaults = STEP_DEFAULTS[step] || { layer: "land", tool: "pan" };
    set({
      workflowStep: step,
      activeLayer: defaults.layer,
      activeTool: defaults.tool,
    });
  },

  markStepCompleted: (step) =>
    set((state) => ({
      stepCompleted: { ...state.stepCompleted, [step]: true },
    })),

  resetWorkflow: () =>
    set({
      workflowStep: 1,
      stepCompleted: {},
      activeLayer: "land",
      activeTool: "paint",
    }),
}));
