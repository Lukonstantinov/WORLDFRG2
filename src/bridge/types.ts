// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import type { EconomySnapshot, GoodRegion, LakeData, RiverData, Settlement, SharkZone, VectorSample } from "@types";

// --- Overlay / query commands ---

export interface OverlayVectors {
  wind: VectorSample[];
  currents: VectorSample[];
  current_step: number;
}

export interface Streamline {
  points: [number, number][];
  ctype: number; // 0=neutral, 1=warm, 2=cold
}

export interface TradeRoute {
  points: [number, number][];
  kind: number; // 0 = overland caravan, 1 = maritime, 2 = river
  minor: boolean; // lesser town's single connector road (drawn thinner)
}

export interface FisheryBank {
  x: number;
  y: number;
  radius: number;
  score: number;
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

export interface OverlaysState {
  settlements: import("@types").Settlement[];
  rivers: import("@types").RiverData[];
  lakes: import("@types").LakeData[];
  economy: EconomySnapshot;
}

export interface ElevationBand {
  label: string;
  count: number;
  percentage: number;
}
