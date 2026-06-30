import { create } from "zustand";
import type { WorldMeta, RiverData, LakeData, Settlement, EconomySnapshot, Toponym } from "../types";

/** Live latitude framing, kept SEPARATE from `meta` on purpose. Dragging the
 *  Latitude Frame sliders mutates only this slice so the lat-line overlay tracks
 *  live, without churning the `meta` object identity (which ~9 expensive
 *  MapCanvas effects depend on — tile-cache rebuilds + sim IPC). It is reseeded
 *  from `meta` whenever a world loads or the backend persists a change. */
export interface LatConfig {
  equatorOffset: number;
  latScale: number;
  /** Line-spacing ratio (gap 30→60 ÷ gap 0→30). Shared with the SIMULATION
   *  (persisted as lat_ratio) so currents/climate land on the drawn lines. */
  lineRatio: number;
}

interface WorldStore {
  meta: WorldMeta | null;
  isLoaded: boolean;
  /** Live latitude framing for the overlay (see LatConfig). */
  latConfig: LatConfig;
  rivers: RiverData[];
  lakes: LakeData[];
  settlements: Settlement[];
  /** As-generated settlements (carrying-capacity baseline) before any trade
   *  development, so re-running the economy never compounds the growth. */
  settlementsBaseline: Settlement[];
  /** Persisted economy snapshot (Phase 2): hubs, chains, chokepoints. */
  economy: EconomySnapshot | null;
  /** #26 · named geographic features (rivers/mountains/lakes/regions). */
  toponyms: Toponym[];
  setMeta: (meta: WorldMeta) => void;
  /** Update only the live latitude framing while dragging the sliders. Does NOT
   *  touch `meta`, so heavy meta-keyed effects stay quiet during the drag. */
  setLatConfig: (equatorOffset: number, latScale: number, lineRatio: number) => void;
  setRivers: (rivers: RiverData[]) => void;
  setLakes: (lakes: LakeData[]) => void;
  setSettlements: (settlements: Settlement[]) => void;
  /** Apply trade-developed populations without disturbing the baseline. */
  setSettlementsDeveloped: (settlements: Settlement[]) => void;
  setEconomy: (economy: EconomySnapshot | null) => void;
  setToponyms: (toponyms: Toponym[]) => void;
  clear: () => void;
}

const DEFAULT_LAT: LatConfig = { equatorOffset: 0.5, latScale: 1, lineRatio: 1 };

const latFromMeta = (meta: WorldMeta): LatConfig => ({
  equatorOffset: meta.equator_offset ?? 0.5,
  latScale: meta.lat_scale ?? 1,
  lineRatio: meta.lat_ratio ?? 1,
});

export const useWorldStore = create<WorldStore>((set) => ({
  meta: null,
  isLoaded: false,
  latConfig: DEFAULT_LAT,
  rivers: [],
  lakes: [],
  settlements: [],
  settlementsBaseline: [],
  economy: null,
  toponyms: [],
  // Setting meta reseeds the live latitude framing so the two stay in sync on
  // world load and after a persisted slider change.
  setMeta: (meta) => set({ meta, isLoaded: true, latConfig: latFromMeta(meta) }),
  setLatConfig: (equatorOffset, latScale, lineRatio) => set({ latConfig: { equatorOffset, latScale, lineRatio } }),
  setRivers: (rivers) => set({ rivers }),
  setLakes: (lakes) => set({ lakes }),
  setSettlements: (settlements) => set({ settlements, settlementsBaseline: settlements }),
  setSettlementsDeveloped: (settlements) => set({ settlements }),
  setEconomy: (economy) => set({ economy }),
  setToponyms: (toponyms) => set({ toponyms }),
  clear: () =>
    set({ meta: null, isLoaded: false, latConfig: DEFAULT_LAT, rivers: [], lakes: [], settlements: [], settlementsBaseline: [], economy: null, toponyms: [] }),
}));
